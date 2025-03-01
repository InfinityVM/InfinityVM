//! Configuration for IVM's EVM execution environment.

use alloy_primitives::Address;
use gas::ivm_gas_handler_register;
use reth_chainspec::ChainSpec;
use reth_evm::{env::EvmEnv, ConfigureEvm, ConfigureEvmEnv, Database, NextBlockEnvAttributes};
use reth_evm_ethereum::{EthEvm, EthEvmConfig};
use reth_node_builder::{
    components::ExecutorBuilder, BuilderContext, FullNodeTypes, NodeTypesWithEngine,
};
use reth_node_ethereum::{BasicBlockExecutorProvider, EthExecutionStrategyFactory};
use reth_primitives::{EthPrimitives, Header, TransactionSigned};
use reth_revm::{inspector_handle_register, EvmBuilder};
use revm_primitives::{CfgEnvWithHandlerCfg, EVMError, HaltReason, HandlerCfg, SpecId, TxEnv};
use std::{convert::Infallible, sync::Arc};

pub mod gas;

/// IVM's EVM configuration
#[derive(Debug, Clone)]
pub struct IvmEvmConfig {
    /// Wrapper around mainnet configuration
    eth: EthEvmConfig,
}

impl IvmEvmConfig {
    /// Create a new instance of [Self].
    pub const fn new(chain_spec: Arc<ChainSpec>) -> Self {
        Self { eth: EthEvmConfig::new(chain_spec) }
    }
}

impl ConfigureEvmEnv for IvmEvmConfig {
    type Header = Header;
    type Transaction = TransactionSigned;
    type Error = Infallible;
    type TxEnv = TxEnv;
    type Spec = SpecId;

    fn tx_env(&self, transaction: &Self::Transaction, signer: Address) -> Self::TxEnv {
        self.eth.tx_env(transaction, signer)
    }

    fn evm_env(&self, header: &Self::Header) -> EvmEnv {
        self.eth.evm_env(header)
    }

    fn next_evm_env(
        &self,
        parent: &Self::Header,
        attributes: NextBlockEnvAttributes,
    ) -> Result<EvmEnv, Self::Error> {
        self.eth.next_evm_env(parent, attributes)
    }
}

impl ConfigureEvm for IvmEvmConfig {
    type Evm<'a, DB: Database + 'a, I: 'a> = EthEvm<'a, I, DB>;
    type EvmError<DBError: core::error::Error + Send + Sync + 'static> = EVMError<DBError>;
    type HaltReason = HaltReason;

    fn evm_with_env<DB: Database>(&self, db: DB, evm_env: EvmEnv) -> Self::Evm<'_, DB, ()> {
        let cfg_env_with_handler_cfg = CfgEnvWithHandlerCfg {
            cfg_env: evm_env.cfg_env,
            handler_cfg: HandlerCfg::new(evm_env.spec),
        };

        EvmBuilder::default()
            .with_db(db)
            .with_cfg_env_with_handler_cfg(cfg_env_with_handler_cfg)
            .with_block_env(evm_env.block_env)
            .append_handler_register(ivm_gas_handler_register)
            .build()
            .into()
    }

    fn evm_with_env_and_inspector<DB, I>(
        &self,
        db: DB,
        evm_env: EvmEnv,
        inspector: I,
    ) -> Self::Evm<'_, DB, I>
    where
        DB: Database,
        I: revm::GetInspector<DB>,
    {
        let cfg_env_with_handler_cfg = CfgEnvWithHandlerCfg {
            cfg_env: evm_env.cfg_env,
            handler_cfg: HandlerCfg::new(evm_env.spec),
        };

        EvmBuilder::default()
            .with_db(db)
            .with_external_context(inspector)
            .with_cfg_env_with_handler_cfg(cfg_env_with_handler_cfg)
            .with_block_env(evm_env.block_env)
            .append_handler_register(inspector_handle_register)
            .append_handler_register(ivm_gas_handler_register)
            .build()
            .into()
    }
}

/// IVM EVM and executor builder.
#[derive(Debug, Default, Clone, Copy)]
pub struct IvmExecutorBuilder;

impl<Types, Node> ExecutorBuilder<Node> for IvmExecutorBuilder
where
    Types: NodeTypesWithEngine<ChainSpec = ChainSpec, Primitives = EthPrimitives>,
    Node: FullNodeTypes<Types = Types>,
{
    type EVM = IvmEvmConfig;
    type Executor = BasicBlockExecutorProvider<EthExecutionStrategyFactory<IvmEvmConfig>>;

    async fn build_evm(
        self,
        ctx: &BuilderContext<Node>,
    ) -> eyre::Result<(Self::EVM, Self::Executor)> {
        let chain_spec = ctx.chain_spec();
        let evm_config = IvmEvmConfig::new(ctx.chain_spec());
        let strategy_factory = EthExecutionStrategyFactory::new(chain_spec, evm_config.clone());
        let executor = BasicBlockExecutorProvider::new(strategy_factory);

        Ok((evm_config, executor))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use alloy_consensus::{TxEip1559, TxEip4844};
    use alloy_network::TxSignerSync;
    use alloy_primitives::{hex, Address, TxKind, B256};
    use alloy_signer_local::LocalSigner;
    use k256::ecdsa::SigningKey;
    use reth::{
        chainspec::{ChainSpecBuilder, MAINNET},
        core::primitives::SignedTransaction,
        primitives::{Account, Block, BlockBody, Transaction},
        revm::{
            db::{CacheDB, EmptyDBTyped},
            DatabaseCommit,
        },
    };
    use reth_evm::{
        execute::{
            BlockExecutionError, BlockExecutorProvider, BlockValidationError, Executor,
            ProviderError,
        },
        Evm,
    };
    use reth_primitives::RecoveredBlock;
    use reth_provider::AccountReader;
    use reth_revm::{database::StateProviderDatabase, test_utils::StateProviderTest};
    use revm::{
        db::states::account_status::AccountStatus as DbAccountStatus,
        interpreter::primitives::AccountStatus,
        primitives::{AccountInfo, HashMap, U256},
        Database,
    };

    // Exact gas used by the transaction returned by `transaction_with_signer`.
    const EXACT_GAS_USED: u64 = 21080;

    fn executor_provider(
        chain_spec: Arc<ChainSpec>,
    ) -> BasicBlockExecutorProvider<EthExecutionStrategyFactory<IvmEvmConfig>> {
        let evm_config = IvmEvmConfig::new(chain_spec.clone());
        let strategy_factory = EthExecutionStrategyFactory::new(chain_spec, evm_config);

        BasicBlockExecutorProvider::new(strategy_factory)
    }

    fn chain_spec() -> Arc<ChainSpec> {
        Arc::new(ChainSpecBuilder::from(&*MAINNET).cancun_activated().build())
    }

    fn setup() -> (
        Header,
        StateProviderTest,
        BasicBlockExecutorProvider<EthExecutionStrategyFactory<IvmEvmConfig>>,
    ) {
        let header = Header {
            timestamp: 1,
            number: 1,
            parent_beacon_block_root: Some(B256::with_last_byte(0x69)),
            excess_blob_gas: Some(0),
            gas_limit: 1_000_000_000,
            ..Header::default()
        };
        let db = StateProviderTest::default();

        let provider = executor_provider(chain_spec());

        (header, db, provider)
    }

    fn transaction() -> (TransactionSigned, Address, u64) {
        let signer = LocalSigner::random();

        transaction_with_signer(signer, 0)
    }

    fn transaction_with_signer(
        signer: LocalSigner<SigningKey>,
        nonce: u64,
    ) -> (TransactionSigned, Address, u64) {
        let gas_limit = EXACT_GAS_USED + 1_000;

        // Create a TX with random data
        let transaction_signed = {
            let mut inner_tx = TxEip1559 {
                input: hex!("0000000011223344").as_ref().into(),
                gas_limit,
                max_fee_per_gas: 1,
                chain_id: 1,
                to: TxKind::Call(Address::default()),
                nonce,
                ..Default::default()
            };

            let signature = signer.sign_transaction_sync(&mut inner_tx).unwrap();
            let tx = Transaction::Eip1559(inner_tx);
            TransactionSigned::new_unhashed(tx, signature)
        };

        (transaction_signed, signer.address(), gas_limit)
    }

    #[test]
    fn evm_transact_with_account_creation() {
        let (transaction_signed, signer_address, _) = transaction();
        let mut db = CacheDB::<EmptyDBTyped<ProviderError>>::default();

        // Show that the account doesn't exist
        assert!(db.basic(signer_address).unwrap().is_none());

        let evm_env = EvmEnv { spec: SpecId::CANCUN, ..Default::default() };
        let evm_config = IvmEvmConfig::new(chain_spec());
        let mut evm = evm_config.evm_with_env(db, evm_env);
        let tx_env =
            evm_config.tx_env(&transaction_signed, transaction_signed.recover_signer().unwrap());

        let result = evm.transact(tx_env).unwrap();

        assert_eq!(result.result.gas_used(), EXACT_GAS_USED);

        let account = result.state.get(&signer_address).unwrap();

        assert_eq!(
            account.info,
            AccountInfo { balance: U256::ZERO, nonce: 1, ..Default::default() }
        );

        let mut status = AccountStatus::LoadedAsNotExisting;
        status.insert(AccountStatus::Touched);
        assert_eq!(account.status, status);
    }

    // Tests 3 transaction from the same signer to test account creation and updating a pre-existing
    // account. And tests 1 transaction from another signer to show that multiple accounts can
    // be created.
    #[test]
    fn evm_transact_with_account_creation_and_update() {
        let signer1 = LocalSigner::random();

        let (transaction_signed, signer_address, _) = transaction_with_signer(signer1.clone(), 0);
        let db = CacheDB::<EmptyDBTyped<ProviderError>>::default();

        // 1st transaction, SAME signer
        let evm_env = EvmEnv { spec: SpecId::CANCUN, ..Default::default() };
        let evm_config = IvmEvmConfig::new(chain_spec());
        let mut evm = evm_config.evm_with_env(db, evm_env);
        let tx_env =
            evm_config.tx_env(&transaction_signed, transaction_signed.recover_signer().unwrap());

        let result = evm.transact(tx_env).unwrap();
        assert_eq!(result.result.gas_used(), EXACT_GAS_USED);
        let account = result.state.get(&signer_address).unwrap();
        assert_eq!(
            account.info,
            AccountInfo { balance: U256::ZERO, nonce: 1, ..Default::default() }
        );
        let mut status = AccountStatus::LoadedAsNotExisting;
        status.insert(AccountStatus::Touched);
        assert_eq!(account.status, status);

        // Commit 1st
        evm.db_mut().commit(result.state);

        // 2nd transaction, SAME signer
        let (transaction_signed2, _, _) = transaction_with_signer(signer1.clone(), 1);
        let tx_env =
            evm_config.tx_env(&transaction_signed2, transaction_signed2.recover_signer().unwrap());

        let result = evm.transact(tx_env).unwrap();
        assert_eq!(result.result.gas_used(), EXACT_GAS_USED);

        let account = result.state.get(&signer_address).unwrap();
        assert_eq!(
            account.info,
            AccountInfo { balance: U256::ZERO, nonce: 2, ..Default::default() }
        );
        assert_eq!(account.status, AccountStatus::Touched);

        // Commit 2nd
        evm.db_mut().commit(result.state);

        // 3rd transaction, SAME signer
        let (transaction_signed3, _, _) = transaction_with_signer(signer1, 2);
        let tx_env =
            evm_config.tx_env(&transaction_signed3, transaction_signed3.recover_signer().unwrap());

        let result = evm.transact(tx_env).unwrap();
        assert_eq!(result.result.gas_used(), EXACT_GAS_USED);
        let account = result.state.get(&signer_address).unwrap();
        assert_eq!(
            account.info,
            AccountInfo { balance: U256::ZERO, nonce: 3, ..Default::default() }
        );
        assert_eq!(account.status, AccountStatus::Touched);

        // Commit 3rd
        evm.db_mut().commit(result.state);

        // 4th transaction, NEW signer
        let (transaction_signed4, signer_address2, _) = transaction();
        let tx_env =
            evm_config.tx_env(&transaction_signed4, transaction_signed4.recover_signer().unwrap());

        assert_eq!(result.result.gas_used(), EXACT_GAS_USED);

        let result = evm.transact(tx_env).unwrap();

        let account = result.state.get(&signer_address2).unwrap();
        assert_eq!(
            account.info,
            AccountInfo { balance: U256::ZERO, nonce: 1, ..Default::default() }
        );
        let mut status = AccountStatus::LoadedAsNotExisting;
        status.insert(AccountStatus::Touched);
        assert_eq!(account.status, status);
    }

    #[test]
    fn execute_empty_block_does_not_error() {
        let (header, db, provider) = setup();

        provider
            .executor(StateProviderDatabase::new(&db))
            .execute_one(&RecoveredBlock::new_unhashed(
                Block {
                    header,
                    body: BlockBody { transactions: vec![], ommers: vec![], withdrawals: None },
                },
                vec![],
            ))
            .unwrap();
    }

    #[test]
    fn execute_block_transaction_from_account_with_no_balance() {
        let (mut header, db, provider) = setup();
        let (transaction_signed, signer_address, _gas_limit) = transaction();

        // We know this is the exact gas used
        header.gas_used = EXACT_GAS_USED;
        // And the expected receipts root
        header.receipts_root =
            B256::from(hex!("5240c13baa9d1e0d29a6c984ba919cb949d4c1a9ceb74060760c90e4d1fcd765"));

        // This account has nothing
        assert!(db.basic_account(&signer_address).unwrap().is_none());

        let mut executor = provider.executor(StateProviderDatabase::new(&db));

        executor
            .execute_one(&RecoveredBlock::new_unhashed(
                Block {
                    header,
                    body: BlockBody {
                        transactions: vec![transaction_signed],
                        ommers: vec![],
                        withdrawals: None,
                    },
                },
                vec![signer_address],
            ))
            .unwrap();

        let expected = AccountInfo { balance: U256::from(0), nonce: 1, ..Default::default() };

        // Check executor state
        let account =
            executor.with_state_mut(|state| state.basic(signer_address).unwrap().unwrap());
        assert_eq!(expected, account);

        let bundle = executor.into_state().take_bundle();

        // And confirm it matches the bundled state
        let account = bundle.state.get(&signer_address).unwrap().clone();
        assert_eq!(account.info.unwrap(), expected);
        assert_eq!(account.status, DbAccountStatus::InMemoryChange);
        assert_eq!(account.original_info, None);
    }

    #[test]
    fn execute_block_from_account_with_high_nonce() {
        let (mut header, mut db, provider) = setup();
        let signer = LocalSigner::random();
        let (transaction_signed, signer_address, _) = transaction_with_signer(signer, 10);

        let exact_gas_used = EXACT_GAS_USED;

        // We know this is the exact gas used
        header.gas_used = exact_gas_used;
        // And the expected receipts root
        header.receipts_root =
            B256::from(hex!("5240c13baa9d1e0d29a6c984ba919cb949d4c1a9ceb74060760c90e4d1fcd765"));

        let user_account = Account { nonce: 10, balance: U256::ZERO, bytecode_hash: None };
        db.insert_account(signer_address, user_account, None, HashMap::default());

        let mut executor = provider.executor(StateProviderDatabase::new(&db));

        executor
            .execute_one(&RecoveredBlock::new_unhashed(
                Block {
                    header,
                    body: BlockBody {
                        transactions: vec![transaction_signed],
                        ommers: vec![],
                        withdrawals: None,
                    },
                },
                vec![signer_address],
            ))
            .unwrap();

        let bundle = executor.into_state().take_bundle();
        let bundle_account = bundle.state.get(&signer_address).unwrap().clone();

        // New account info is expected
        assert_eq!(
            bundle_account.info.unwrap(),
            AccountInfo {
                balance: U256::ZERO,
                // Balance correctly updated
                nonce: 11,
                ..Default::default()
            }
        );
        // Original account info is as expected
        assert_eq!(
            bundle_account.original_info.unwrap(),
            AccountInfo { balance: U256::ZERO, nonce: 10, ..Default::default() }
        );
    }

    #[test]
    fn execute_block_account_with_excess_balance() {
        let (mut header, mut db, provider) = setup();
        let (transaction_signed, signer_address, gas_limit) = transaction();

        let exact_gas_used = EXACT_GAS_USED;

        // We know this is the exact gas used
        header.gas_used = exact_gas_used;
        // And the expected receipts root
        header.receipts_root =
            B256::from(hex!("5240c13baa9d1e0d29a6c984ba919cb949d4c1a9ceb74060760c90e4d1fcd765"));

        let user_balance = U256::from(gas_limit + 1_000);
        let user_account =
            Account { nonce: 0, balance: U256::from(user_balance), bytecode_hash: None };
        db.insert_account(signer_address, user_account, None, HashMap::default());

        let mut executor = provider.executor(StateProviderDatabase::new(&db));

        let block = RecoveredBlock::new_unhashed(
            Block {
                header,
                body: BlockBody {
                    transactions: vec![transaction_signed],
                    ommers: vec![],
                    withdrawals: None,
                },
            },
            vec![signer_address],
        );
        executor.execute_one(&block).unwrap();

        let bundle = executor.into_state().take_bundle();
        let bundle_account = bundle.state.get(&signer_address).unwrap().clone();

        // New account info is expected
        assert_eq!(
            bundle_account.info.unwrap(),
            AccountInfo {
                // Balance does not chance
                balance: U256::from(user_balance),
                nonce: user_account.nonce + 1,
                ..Default::default()
            }
        );
        // Original account info is as expected
        assert_eq!(
            bundle_account.original_info.unwrap(),
            AccountInfo {
                balance: U256::from(user_balance),
                nonce: user_account.nonce,
                ..Default::default()
            }
        );
    }

    #[test]
    fn execute_block_account_with_balance_less_then_gas_limit() {
        let (mut header, mut db, provider) = setup();
        let (transaction_signed, signer_address, gas_limit) = transaction();

        let exact_gas_used = EXACT_GAS_USED;

        // We know this is the exact gas used
        header.gas_used = exact_gas_used;
        // And the expected receipts root
        header.receipts_root =
            B256::from(hex!("5240c13baa9d1e0d29a6c984ba919cb949d4c1a9ceb74060760c90e4d1fcd765"));

        let user_balance = U256::from(gas_limit - 100);
        let user_account = Account { nonce: 0, balance: user_balance, bytecode_hash: None };
        db.insert_account(signer_address, user_account, None, HashMap::default());

        let mut executor = provider.executor(StateProviderDatabase::new(&db));
        executor
            .execute_one(&RecoveredBlock::new_unhashed(
                Block {
                    header,
                    body: BlockBody {
                        transactions: vec![transaction_signed],
                        ommers: vec![],
                        withdrawals: None,
                    },
                },
                vec![signer_address],
            ))
            .unwrap();

        let bundle = executor.into_state().take_bundle();
        let bundle_account = bundle.state.get(&signer_address).unwrap().clone();

        assert_eq!(
            bundle_account.info.unwrap(),
            AccountInfo {
                // Balance does not get changed
                balance: U256::from(user_balance),
                // Nonce is correctly updated
                nonce: 1,
                ..Default::default()
            }
        );
        // Original account info is as expected
        assert_eq!(
            bundle_account.original_info.unwrap(),
            AccountInfo { balance: U256::from(user_balance), nonce: 0, ..Default::default() }
        );
    }

    #[test]
    fn execute_block_with_multiple_transactions_from_new_and_existing_accounts() {
        let (mut header, mut db, provider) = setup();

        let signer1 = LocalSigner::random();
        let (transaction_signed, signer_address1, gas_limit) =
            transaction_with_signer(signer1.clone(), 0);
        let (transaction_signed2, _, _) = transaction_with_signer(signer1.clone(), 1);
        let (transaction_signed3, _, _) = transaction_with_signer(signer1.clone(), 2);

        let signer2 = LocalSigner::random();
        let (transaction_signed4, signer_address2, _) = transaction_with_signer(signer2, 5);
        let (transaction_signed5, _, _) = transaction_with_signer(signer1.clone(), 6);
        let (transaction_signed6, _, _) = transaction_with_signer(signer1, 7);

        // We know this is the exact gas used
        header.gas_used = 6 * EXACT_GAS_USED;
        // And the expected receipts root
        header.receipts_root =
            B256::from(hex!("ad6b6e2b36bd06ab7c61110083c396ab8bba9d62feb2e4b8c9c38513dcdff7ce"));
        // Set exact gas limit for the entire block
        header.gas_limit = 5 * EXACT_GAS_USED + gas_limit;

        // first account has nothing
        assert!(db.basic_account(&signer_address1).unwrap().is_none());

        // second account has some balance
        let user2_balance = U256::from(gas_limit - 100);
        let user2_account = Account { nonce: 5, balance: user2_balance, bytecode_hash: None };
        db.insert_account(signer_address2, user2_account, None, HashMap::default());

        let mut executor = provider.executor(StateProviderDatabase::new(&db));

        executor
            .execute_one(&RecoveredBlock::new_unhashed(
                Block {
                    header,
                    body: BlockBody {
                        transactions: vec![
                            transaction_signed,
                            transaction_signed4,
                            transaction_signed2,
                            transaction_signed5,
                            transaction_signed3,
                            transaction_signed6,
                        ],
                        ommers: vec![],
                        withdrawals: None,
                    },
                },
                vec![
                    signer_address1,
                    signer_address2,
                    signer_address1,
                    signer_address2,
                    signer_address1,
                    signer_address2,
                ],
            ))
            .unwrap();

        let bundle = executor.into_state().take_bundle();

        let bundle_account1 = bundle.state.get(&signer_address1).unwrap().clone();
        assert_eq!(
            bundle_account1.info.unwrap(),
            AccountInfo {
                // Balance does not get changed
                balance: U256::ZERO,
                // Nonce is correctly updated
                nonce: 3,
                ..Default::default()
            }
        );
        // Original account info is as expected
        assert!(bundle_account1.original_info.is_none());
        assert_eq!(bundle_account1.status, DbAccountStatus::InMemoryChange,);

        let bundle_account2 = bundle.state.get(&signer_address2).unwrap().clone();
        assert_eq!(
            bundle_account2.info.unwrap(),
            AccountInfo {
                // Balance does not get changed
                balance: U256::from(user2_balance),
                // Nonce is correctly updated
                nonce: 8,
                ..Default::default()
            }
        );
        // Original account info is as expected
        assert_eq!(
            bundle_account2.original_info.unwrap(),
            AccountInfo { balance: U256::from(user2_balance), nonce: 5, ..Default::default() }
        );
        assert_eq!(bundle_account2.status, DbAccountStatus::Changed);
    }

    #[test]
    fn execute_block_errors_on_nonce_too_high() {
        let (mut header, mut db, provider) = setup();
        let signer = LocalSigner::random();
        let (transaction_signed, signer_address, _) = transaction_with_signer(signer, 10);

        let exact_gas_used = EXACT_GAS_USED;

        // We know this is the exact gas used
        header.gas_used = exact_gas_used;
        // And the expected receipts root
        header.receipts_root =
            B256::from(hex!("5240c13baa9d1e0d29a6c984ba919cb949d4c1a9ceb74060760c90e4d1fcd765"));

        let user_account = Account { nonce: 5, balance: U256::ZERO, bytecode_hash: None };
        db.insert_account(signer_address, user_account, None, HashMap::default());

        let mut executor = provider.executor(StateProviderDatabase::new(&db));

        let err = executor
            .execute_one(&RecoveredBlock::new_unhashed(
                Block {
                    header,
                    body: BlockBody {
                        transactions: vec![transaction_signed],
                        ommers: vec![],
                        withdrawals: None,
                    },
                },
                vec![signer_address],
            ))
            .unwrap_err();

        let BlockExecutionError::Validation(BlockValidationError::EVM { error, .. }) = err else {
            panic!()
        };

        assert_eq!(
            error.to_string(),
            "transaction validation error: nonce 10 too high, expected 5".to_string()
        );
    }

    #[test]
    fn execute_block_errors_on_nonce_too_low() {
        let (mut header, mut db, provider) = setup();
        let signer = LocalSigner::random();
        let (transaction_signed, signer_address, _) = transaction_with_signer(signer, 69);

        let exact_gas_used = EXACT_GAS_USED;

        // We know this is the exact gas used
        header.gas_used = exact_gas_used;
        // And the expected receipts root
        header.receipts_root =
            B256::from(hex!("5240c13baa9d1e0d29a6c984ba919cb949d4c1a9ceb74060760c90e4d1fcd765"));

        let user_account = Account { nonce: 420, balance: U256::ZERO, bytecode_hash: None };
        db.insert_account(signer_address, user_account, None, HashMap::default());

        let mut executor = provider.executor(StateProviderDatabase::new(&db));

        let err = executor
            .execute_one(&RecoveredBlock::new_unhashed(
                Block {
                    header,
                    body: BlockBody {
                        transactions: vec![transaction_signed],
                        ommers: vec![],
                        withdrawals: None,
                    },
                },
                vec![signer_address],
            ))
            .unwrap_err();

        let BlockExecutionError::Validation(BlockValidationError::EVM { error, .. }) = err else {
            panic!()
        };

        assert_eq!(
            error.to_string(),
            "transaction validation error: nonce 69 too low, expected 420".to_string()
        );
    }

    #[test]
    fn execute_block_errors_when_gas_limit_is_reached() {
        let (mut header, db, provider) = setup();
        let (transaction_signed1, signer_address1, gas_limit) = transaction();
        let (transaction_signed2, signer_address2, _) = transaction();

        // Set exact gas limit for the entire block - we expect this to work
        header.gas_limit = EXACT_GAS_USED + gas_limit;
        header.gas_used = 2 * EXACT_GAS_USED;
        header.receipts_root =
            B256::from(hex!("d4263b4f8bc6337d6751b03db4192a544872db8beeb3be926d891e8910842eb1"));

        let transactions = vec![transaction_signed1, transaction_signed2];
        let senders = vec![signer_address1, signer_address2];

        // It works when the header has the exact gas used
        provider
            .executor(StateProviderDatabase::new(&db))
            .execute_one(&RecoveredBlock::new_unhashed(
                Block {
                    header: header.clone(),
                    body: BlockBody {
                        transactions: transactions.clone(),
                        ommers: vec![],
                        withdrawals: None,
                    },
                },
                senders.clone(),
            ))
            .unwrap();

        // Set the header to have one less gwei then what we need - we expect this to error.
        // The first transaction is fully processed and reth is able to account for the exact gas
        // used. The second transaction is rejected when its gas limit is greater then the
        // available gas.
        header.gas_limit = EXACT_GAS_USED + gas_limit - 1;
        let err = provider
            .executor(StateProviderDatabase::new(&db))
            .execute_one(&RecoveredBlock::new_unhashed(
                Block {
                    header,
                    body: BlockBody { transactions, ommers: vec![], withdrawals: None },
                },
                senders,
            ))
            .unwrap_err();

        let BlockExecutionError::Validation(
            BlockValidationError::TransactionGasLimitMoreThanAvailableBlockGas {
                transaction_gas_limit,
                block_available_gas,
            },
        ) = err
        else {
            panic!()
        };
        assert_eq!(transaction_gas_limit, gas_limit);
        assert_eq!(block_available_gas, gas_limit - 1);
    }

    #[test]
    fn execute_block_handles_eip4844() {
        let (mut header, db, provider) = setup();
        let signer = LocalSigner::random();
        let transaction_signed = {
            let blob_versioned_hash = [1u8; 32];

            let mut inner_tx = TxEip4844 {
                chain_id: 1,
                max_fee_per_blob_gas: 1,
                blob_versioned_hashes: vec![blob_versioned_hash.into()],
                gas_limit: 21000,
                ..Default::default()
            };

            let signature = signer.sign_transaction_sync(&mut inner_tx).unwrap();
            let tx = Transaction::Eip4844(inner_tx);
            TransactionSigned::new_unhashed(tx, signature)
        };
        header.gas_used = 21000;
        header.gas_limit = 21000;

        // And the expected receipts root
        header.receipts_root =
            B256::from(hex!("eaa8c40899a61ae59615cf9985f5e2194f8fd2b57d273be63bde6733e89b12ab"));

        let mut executor = provider.executor(StateProviderDatabase::new(&db));
        executor
            .execute_one(&RecoveredBlock::new_unhashed(
                Block {
                    header,
                    body: BlockBody {
                        transactions: vec![transaction_signed],
                        ommers: vec![],
                        withdrawals: None,
                    },
                },
                vec![signer.address()],
            ))
            .unwrap();

        let state = executor.into_state().take_bundle();

        let account = state.state.get(&signer.address()).unwrap().clone();
        assert_eq!(
            account.info.unwrap(),
            AccountInfo { balance: U256::from(0), nonce: 1, ..Default::default() }
        );
        assert_eq!(account.status, DbAccountStatus::InMemoryChange);
        assert_eq!(account.original_info, None);
    }
}

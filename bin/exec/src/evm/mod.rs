//! Configuration for IVM's EVM execution environment.

use crate::evm::builder::IvmEvmBuilder;
use alloy::primitives::{Address, Bytes, U256};
use reth::{
    builder::{
        components::ExecutorBuilder, BuilderContext, ConfigureEvm, FullNodeTypes,
        NodeTypesWithEngine,
    },
    chainspec::{ChainSpec, MAINNET},
    primitives::{EthPrimitives, Header, TransactionSigned},
    revm::{
        primitives::{BlockEnv, CfgEnvWithHandlerCfg, Env, TxEnv},
        Database, Evm, GetInspector,
    },
};
use reth_evm_ethereum::EthEvmConfig;
use reth_node_api::{ConfigureEvmEnv, NextBlockEnvAttributes};
use reth_node_ethereum::{BasicBlockExecutorProvider, EthExecutionStrategyFactory};
use std::{convert::Infallible, sync::Arc};

pub mod builder;

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

    fn fill_tx_env(&self, tx_env: &mut TxEnv, transaction: &TransactionSigned, sender: Address) {
        self.eth.fill_tx_env(tx_env, transaction, sender);
    }

    fn fill_tx_env_system_contract_call(
        &self,
        env: &mut Env,
        caller: Address,
        contract: Address,
        data: Bytes,
    ) {
        self.eth.fill_tx_env_system_contract_call(env, caller, contract, data);
    }

    fn fill_cfg_env(
        &self,
        cfg_env: &mut CfgEnvWithHandlerCfg,
        header: &Self::Header,
        total_difficulty: U256,
    ) {
        self.eth.fill_cfg_env(cfg_env, header, total_difficulty);
    }

    fn next_cfg_and_block_env(
        &self,
        parent: &Self::Header,
        attributes: NextBlockEnvAttributes,
    ) -> Result<(CfgEnvWithHandlerCfg, BlockEnv), Self::Error> {
        self.eth.next_cfg_and_block_env(parent, attributes)
    }
}

impl ConfigureEvm for IvmEvmConfig {
    type DefaultExternalContext<'a> = ();

    fn default_external_context<'a>(&self) -> Self::DefaultExternalContext<'a> {}

    fn evm<DB: Database>(&self, db: DB) -> Evm<'_, Self::DefaultExternalContext<'_>, DB> {
        IvmEvmBuilder::new(db, ()).build()
    }

    fn evm_with_inspector<DB, I>(&self, db: DB, inspector: I) -> Evm<'_, I, DB>
    where
        DB: Database,
        I: GetInspector<DB>,
    {
        IvmEvmBuilder::new(db, ()).build_with_inspector(inspector)
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
    use alloy::primitives::{hex, Address, TxKind, B256};
    use reth::{
        chainspec::{ChainSpecBuilder, MAINNET},
        primitives::{
            Account, Block, BlockBody, BlockWithSenders, EthereumHardfork, ForkCondition,
            Transaction,
        },
    };
    use reth_evm::execute::{BatchExecutor, BlockExecutorProvider};
    use reth_provider::AccountReader;
    use reth_revm::{database::StateProviderDatabase, test_utils::StateProviderTest};
    // use revm::precompile::primitives::{AccountInfo, Bytecode, JumpTable, LegacyAnalyzedBytecode};
    use reth::{
        core::primitives::SignedTransaction,
        revm::db::{CacheDB, EmptyDBTyped},
    };
    use reth_evm::execute::ProviderError;
    use revm::primitives::{AccountInfo, Bytecode, JumpTable, LegacyAnalyzedBytecode};
    use std::collections::HashMap;
    use revm::interpreter::primitives::AccountStatus;
    use reth::revm::DatabaseCommit;
    use k256::ecdsa::SigningKey;


    // Special alloy deps we need for playing happy with reth
    use alloy_consensus::{Transaction as _, TxEip1559};
    use alloy_network::TxSignerSync;
    use alloy_signer_local::LocalSigner;

    fn executor_provider(
        chain_spec: Arc<ChainSpec>,
    ) -> BasicBlockExecutorProvider<EthExecutionStrategyFactory<IvmEvmConfig>> {
        let evm_config = IvmEvmConfig::new(chain_spec.clone());
        let strategy_factory = EthExecutionStrategyFactory::new(chain_spec, evm_config);

        BasicBlockExecutorProvider::new(strategy_factory)
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

        let chain_spec = Arc::new(
            ChainSpecBuilder::from(&*MAINNET)
                .shanghai_activated()
                .with_fork(EthereumHardfork::Cancun, ForkCondition::Timestamp(1))
                .build(),
        );

        let provider = executor_provider(chain_spec);

        (header, db, provider)
    }

    fn transaction() -> (TransactionSigned, Address, u64) {
        let signer = LocalSigner::random();

        transaction_with_signer(signer, 0)
    }

    fn transaction_with_signer(signer: LocalSigner<SigningKey
        >, nonce: u64) -> (TransactionSigned, Address, u64) {
        let exact_gas_used = 21080;
        let gas_limit = exact_gas_used + 1_000;

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
        // let (header, db, provider) = setup();
        let (transaction_signed, signer_address, _) = transaction();
        let db = CacheDB::<EmptyDBTyped<ProviderError>>::default();

        let evm_config = IvmEvmConfig::new(MAINNET.clone());
        let mut evm = evm_config.evm(db);

        evm_config.fill_tx_env(
            evm.tx_mut(),
            &transaction_signed,
            transaction_signed.recover_signer().unwrap(),
        );

        let result = evm.transact().unwrap();

        assert_eq!(result.result.gas_used(), 21080);

        let account = result.state.get(&signer_address).unwrap();

        assert_eq!(
            account.info,
            AccountInfo {
                balance: U256::ZERO,
                nonce: 1,
                code_hash: B256::from(hex!(
                    "c5d2460186f7233c927e7db2dcc703c0e500b653ca82273b7bfad8045d85a470"
                )),
                code: Some(Bytecode::LegacyAnalyzed(LegacyAnalyzedBytecode::default()))
            }
        );

        let mut status = AccountStatus::LoadedAsNotExisting;
        status.insert(AccountStatus::Touched);
        assert_eq!(
            account.status,
            status
        );
    }

    // Tests 3 transaction from the same signer to test account creation and updating a pre-existing account.
    // And tests 1 transaction from another signer to show that multiple accounts can be created.
    #[test]
    fn evm_transact_with_account_creation_and_update() {
        let signer1 = LocalSigner::random();
        
        let (transaction_signed, signer_address, _) = transaction_with_signer(signer1.clone(), 0);
        let db = CacheDB::<EmptyDBTyped<ProviderError>>::default();

        // 1st transaction, SAME signer
        let evm_config = IvmEvmConfig::new(MAINNET.clone());
        let mut evm = evm_config.evm(db);
        evm_config.fill_tx_env(
            evm.tx_mut(),
            &transaction_signed,
            transaction_signed.recover_signer().unwrap(),
        );

        let result = evm.transact().unwrap();
        assert_eq!(result.result.gas_used(), 21080);
        let account = result.state.get(&signer_address).unwrap();
        assert_eq!(
            account.info,
            AccountInfo {
                balance: U256::ZERO,
                nonce: 1,
                code_hash: B256::from(hex!(
                    "c5d2460186f7233c927e7db2dcc703c0e500b653ca82273b7bfad8045d85a470"
                )),
                code: Some(Bytecode::LegacyAnalyzed(LegacyAnalyzedBytecode::default()))
            }
        );
        let mut status = AccountStatus::LoadedAsNotExisting;
        status.insert(AccountStatus::Touched);
        assert_eq!(
            account.status,
            status
        );

        // Commit 1st
        evm.db_mut().commit(result.state);

        // 2nd transaction, SAME signer
        let (transaction_signed2, _, _) = transaction_with_signer(signer1.clone(), 1);
        evm_config.fill_tx_env(
            evm.tx_mut(),
            &transaction_signed2,
            transaction_signed2.recover_signer().unwrap(),
        );
        
        let result = evm.transact().unwrap();
        assert_eq!(result.result.gas_used(), 21080);
        let account = result.state.get(&signer_address).unwrap();
        assert_eq!(
            account.info,
            AccountInfo {
                balance: U256::ZERO,
                nonce: 2,
                code_hash: B256::from(hex!(
                    "c5d2460186f7233c927e7db2dcc703c0e500b653ca82273b7bfad8045d85a470"
                )),
                code: Some(Bytecode::LegacyAnalyzed(LegacyAnalyzedBytecode::default()))
            }
        );
        assert_eq!(
            account.status,
            AccountStatus::Touched
        );

        // Commit 2nd
        evm.db_mut().commit(result.state);

        // 3rd transaction, SAME signer
        let (transaction_signed3, _, _) = transaction_with_signer(signer1.clone(), 2);
        evm_config.fill_tx_env(
            evm.tx_mut(),
            &transaction_signed3,
            transaction_signed3.recover_signer().unwrap(),
        );

        
        let result = evm.transact().unwrap();
        assert_eq!(result.result.gas_used(), 21080);
        let account = result.state.get(&signer_address).unwrap();
        assert_eq!(
            account.info,
            AccountInfo {
                balance: U256::ZERO,
                nonce: 3,
                code_hash: B256::from(hex!(
                    "c5d2460186f7233c927e7db2dcc703c0e500b653ca82273b7bfad8045d85a470"
                )),
                code: Some(Bytecode::LegacyAnalyzed(LegacyAnalyzedBytecode::default()))
            }
        );
        assert_eq!(
            account.status,
            AccountStatus::Touched
        );

        // Commit 3rd
        evm.db_mut().commit(result.state);

        // 4th transaction, NEW signer
        let (transaction_signed4, signer_address2, _) = transaction();
        evm_config.fill_tx_env(
            evm.tx_mut(),
            &transaction_signed4,
            transaction_signed4.recover_signer().unwrap(),
        );

        assert_eq!(result.result.gas_used(), 21080);

        let result = evm.transact().unwrap();

        let account = result.state.get(&signer_address2).unwrap();
        assert_eq!(
            account.info,
            AccountInfo {
                balance: U256::ZERO,
                nonce: 1
                ,
                code_hash: B256::from(hex!(
                    "c5d2460186f7233c927e7db2dcc703c0e500b653ca82273b7bfad8045d85a470"
                )),
                code: Some(Bytecode::LegacyAnalyzed(LegacyAnalyzedBytecode::default()))
            }
        );
        let mut status = AccountStatus::LoadedAsNotExisting;
        status.insert(AccountStatus::Touched);
        assert_eq!(
            account.status,
            status

        );
    }

    #[test]
    fn execute_empty_block() {
        let (header, db, provider) = setup();

        provider
            .batch_executor(StateProviderDatabase::new(&db))
            .execute_and_verify_one(
                (
                    &BlockWithSenders {
                        block: Block {
                            header,
                            body: BlockBody {
                                transactions: vec![],
                                ommers: vec![],
                                withdrawals: None,
                            },
                        },
                        senders: vec![],
                    },
                    U256::ZERO,
                )
                    .into(),
            )
            .unwrap()
    }

    #[test]
    fn accepts_transaction_from_account_with_no_balance() {
        let (mut header, db, provider) = setup();
        let (transaction_signed, signer_address, gas_limit) = transaction();

        // We know this is the exact gas used
        header.gas_used = 21080;
        // And the expected receipts root
        header.receipts_root =
            B256::from(hex!("5240c13baa9d1e0d29a6c984ba919cb949d4c1a9ceb74060760c90e4d1fcd765"));

        // This account has nothing
        assert!(db.basic_account(signer_address).unwrap().is_none());

        let executor = provider.batch_executor(StateProviderDatabase::new(&db));

        provider
            .batch_executor(StateProviderDatabase::new(&db))
            .execute_and_verify_one(
                (
                    &BlockWithSenders {
                        block: Block {
                            header,
                            body: BlockBody {
                                transactions: vec![transaction_signed],
                                ommers: vec![],
                                withdrawals: None,
                            },
                        },
                        senders: vec![signer_address],
                    },
                    U256::ZERO,
                )
                    .into(),
            )
            .unwrap();

        let output = executor.finalize();

        // The user does not exist in state
        // TODO: this is scary, what about the users nonce?
        assert!(output.bundle.state.get(&signer_address).is_none());
    }

    #[test]
    fn accepts_transaction_from_account_with_excess_balance() {
        let (mut header, mut db, provider) = setup();
        let (transaction_signed, signer_address, gas_limit) = transaction();

        let exact_gas_used = 21080;

        // We know this is the exact gas used
        header.gas_used = exact_gas_used;
        // And the expected receipts root
        header.receipts_root =
            B256::from(hex!("5240c13baa9d1e0d29a6c984ba919cb949d4c1a9ceb74060760c90e4d1fcd765"));

        let user_balance = U256::from(gas_limit + 1_000);
        let user_account =
            Account { nonce: 0, balance: U256::from(user_balance), bytecode_hash: None };
        db.insert_account(signer_address, user_account, None, HashMap::default());

        let mut executor = provider.batch_executor(StateProviderDatabase::new(&db));

        executor
            .execute_and_verify_one(
                (
                    &BlockWithSenders {
                        block: Block {
                            header,
                            body: BlockBody {
                                transactions: vec![transaction_signed],
                                ommers: vec![],
                                withdrawals: None,
                            },
                        },
                        senders: vec![signer_address],
                    },
                    U256::ZERO,
                )
                    .into(),
            )
            .unwrap();

        let output = executor.finalize();
        let bundle_account = output.bundle.state.get(&signer_address).unwrap().clone();

        // New account info is expected
        assert_eq!(
            bundle_account.info.unwrap(),
            AccountInfo {
                // Since there balance was _above_ the gas_limit, their balance does not increase
                balance: U256::from(user_balance),
                nonce: user_account.nonce + 1,
                code_hash: B256::from(hex!(
                    "c5d2460186f7233c927e7db2dcc703c0e500b653ca82273b7bfad8045d85a470"
                )),
                code: Some(Bytecode::LegacyAnalyzed(LegacyAnalyzedBytecode::new(
                    Default::default(),
                    0,
                    JumpTable::default()
                ))),
            }
        );
        // Original account info is as expected
        assert_eq!(
            bundle_account.original_info.unwrap(),
            AccountInfo {
                balance: U256::from(user_balance),
                nonce: user_account.nonce,
                code_hash: B256::from(hex!(
                    "c5d2460186f7233c927e7db2dcc703c0e500b653ca82273b7bfad8045d85a470"
                )),
                code: Some(Bytecode::LegacyAnalyzed(LegacyAnalyzedBytecode::new(
                    Default::default(),
                    0,
                    JumpTable::default()
                ))),
            }
        );
    }

    #[test]
    fn accepts_transaction_from_account_with_a_little_balance() {
        let (mut header, mut db, provider) = setup();
        let (transaction_signed, signer_address, gas_limit) = transaction();

        let exact_gas_used = 21080;

        // We know this is the exact gas used
        header.gas_used = exact_gas_used;
        // And the expected receipts root
        header.receipts_root =
            B256::from(hex!("5240c13baa9d1e0d29a6c984ba919cb949d4c1a9ceb74060760c90e4d1fcd765"));

        let user_balance = U256::from(gas_limit - 100);
        let user_account = Account { nonce: 0, balance: user_balance, bytecode_hash: None };
        db.insert_account(signer_address, user_account, None, HashMap::default());

        let mut executor = provider.batch_executor(StateProviderDatabase::new(&db));

        executor
            .execute_and_verify_one(
                (
                    &BlockWithSenders {
                        block: Block {
                            header,
                            body: BlockBody {
                                transactions: vec![transaction_signed],
                                ommers: vec![],
                                withdrawals: None,
                            },
                        },
                        senders: vec![signer_address],
                    },
                    U256::ZERO,
                )
                    .into(),
            )
            .unwrap();

        let output = executor.finalize();
        dbg!(&output);
        let bundle_account = output.bundle.state.get(&signer_address).unwrap().clone();

        // New account info is expected
        assert_eq!(
            bundle_account.info.unwrap(),
            AccountInfo {
                // There balance gets increased to the gas_limit of the transaction
                balance: U256::from(gas_limit),
                nonce: user_account.nonce + 1,
                code_hash: B256::from(hex!(
                    "c5d2460186f7233c927e7db2dcc703c0e500b653ca82273b7bfad8045d85a470"
                )),
                code: Some(Bytecode::LegacyAnalyzed(LegacyAnalyzedBytecode::new(
                    Default::default(),
                    0,
                    JumpTable::default()
                ))),
            }
        );
        // Original account info is as expected
        assert_eq!(
            bundle_account.original_info.unwrap(),
            AccountInfo {
                balance: U256::from(user_balance),
                nonce: user_account.nonce,
                code_hash: B256::from(hex!(
                    "c5d2460186f7233c927e7db2dcc703c0e500b653ca82273b7bfad8045d85a470"
                )),
                code: Some(Bytecode::LegacyAnalyzed(LegacyAnalyzedBytecode::new(
                    Default::default(),
                    0,
                    JumpTable::default()
                ))),
            }
        );
    }

    #[test]
    fn accepts_transaction_from_account_with_no_balance_and_an_account_with_balance() {
        let (mut header, mut db, provider) = setup();
        let (transaction_signed, signer_address, gas_limit) = transaction();
        // this generates a new signer
        let (transaction_signed2, signer_address2, gas_limit2) = transaction();
        // sanity check these are different signers
        assert_ne!(signer_address, signer_address2);

        // We know this is the exact gas used
        header.gas_used = 2 * 21080;
        // And the expected receipts root
        header.receipts_root =
            B256::from(hex!("d4263b4f8bc6337d6751b03db4192a544872db8beeb3be926d891e8910842eb1"));

        // first account has nothing
        assert!(db.basic_account(signer_address).unwrap().is_none());

        // second account has some balance
        let user2_balance = U256::from(gas_limit - 100);
        let user2_account = Account { nonce: 0, balance: user2_balance, bytecode_hash: None };
        db.insert_account(signer_address2, user2_account, None, HashMap::default());

        let mut executor = provider.batch_executor(StateProviderDatabase::new(&db));

        executor
            .execute_and_verify_one(
                (
                    &BlockWithSenders {
                        block: Block {
                            header,
                            body: BlockBody {
                                transactions: vec![transaction_signed, transaction_signed2],
                                ommers: vec![],
                                withdrawals: None,
                            },
                        },
                        senders: vec![signer_address, signer_address2],
                    },
                    U256::ZERO,
                )
                    .into(),
            )
            .unwrap();

        let output = executor.finalize();
        dbg!(&output);

        // The user does not exist in state
        assert!(output.bundle.state.get(&signer_address).is_none());
    }
}

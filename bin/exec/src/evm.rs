//! Configuration for IVM's EVM execution environment.

use alloy::primitives::{Address, Bytes, U256};
use reth::{
    builder::{
        components::ExecutorBuilder, BuilderContext, ConfigureEvm, FullNodeTypes,
        NodeTypesWithEngine,
    },
    chainspec::ChainSpec,
    primitives::{EthPrimitives, Header, TransactionSigned},
    revm::primitives::{BlockEnv, CfgEnvWithHandlerCfg, Env, TxEnv},
};
use reth_evm_ethereum::EthEvmConfig;
use reth_node_api::{ConfigureEvmEnv, NextBlockEnvAttributes};
use reth_node_ethereum::{BasicBlockExecutorProvider, EthExecutionStrategyFactory};
use std::{convert::Infallible, sync::Arc};

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
        disable_gas_fees(cfg_env);
    }

    fn next_cfg_and_block_env(
        &self,
        parent: &Self::Header,
        attributes: NextBlockEnvAttributes,
    ) -> Result<(CfgEnvWithHandlerCfg, BlockEnv), Self::Error> {
        let (mut cfg_env, block_env) = self.eth.next_cfg_and_block_env(parent, attributes)?;
        disable_gas_fees(&mut cfg_env);
        Ok((cfg_env, block_env))
    }
}

/// Modify the `CfgEnvWithHandlerCfg` to disable balance checks. Disabling balance checks will gas
/// the account so the transaction doesn't fail. We don't want users to "print" gas by
/// overestimating gas and then getting a refund, so we disable gas refunds as well.
#[inline]
fn disable_gas_fees(cfg_env: &mut CfgEnvWithHandlerCfg) {
    cfg_env.disable_balance_check = true;
    cfg_env.disable_gas_refund = true;
}

impl ConfigureEvm for IvmEvmConfig {
    type DefaultExternalContext<'a> = ();

    fn default_external_context<'a>(&self) -> Self::DefaultExternalContext<'a> {}
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
    use reth_revm::{
        database::StateProviderDatabase, test_utils::StateProviderTest, TransitionState,
    };
    use std::collections::HashMap;

    // TODO: use alloy_consensus directly bc that is what reth transaction depends on
    use alloy_consensus::{SignableTransaction, Transaction as _, TxEip1559};
    use alloy_network::TxSignerSync;
    use alloy_signer_local::LocalSigner;

    fn executor_provider(
        chain_spec: Arc<ChainSpec>,
    ) -> BasicBlockExecutorProvider<EthExecutionStrategyFactory<IvmEvmConfig>> {
        let evm_config = IvmEvmConfig::new(chain_spec.clone());
        let strategy_factory = EthExecutionStrategyFactory::new(chain_spec.clone(), evm_config);

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

    fn transaction() -> (SignedTransaction, Address) {
        let signer = LocalSigner::random();

        // Create a TX with random data
        let transaction_signed = {
            let mut inner_tx = TxEip1559::default();

            inner_tx.input = hex!("0000000011223344").as_ref().into();
            inner_tx.gas_limit = 21080;
            inner_tx.max_fee_per_gas = 1;
            inner_tx.chain_id = 1;
            inner_tx.to = TxKind::Call(Address::default());

            let signature = signer.sign_transaction_sync(&mut inner_tx).unwrap();
            let tx = Transaction::Eip1559(inner_tx.into());
            TransactionSigned::new_unhashed(tx, signature)
        };

        (transaction_signed,)
    }

    #[test]
    fn execute_empty_block() {
        let (mut header, db, provider) = setup();

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
        let signer = LocalSigner::random();

        // Create a TX with random data
        let transaction_signed = {
            let mut inner_tx = TxEip1559::default();
            inner_tx.input = hex!("0000000011223344").as_ref().into();
            inner_tx.gas_limit = 1_000_000_000;
            inner_tx.max_fee_per_gas = 1_000_000_000;
            inner_tx.chain_id = 1;
            inner_tx.to = TxKind::Call(Address::default());
            let signature = signer.sign_transaction_sync(&mut inner_tx).unwrap();
            let tx = Transaction::Eip1559(inner_tx.into());

            TransactionSigned::new_unhashed(tx, signature)
        };

        // We know this is the exact gas used
        header.gas_used = 21080;
        // And the expected receipts root
        header.receipts_root =
            B256::from(hex!("5240c13baa9d1e0d29a6c984ba919cb949d4c1a9ceb74060760c90e4d1fcd765"));

        /// This account has nothing
        assert!(db.basic_account(signer.address()).unwrap().is_none());

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
                        senders: vec![signer.address()],
                    },
                    U256::ZERO,
                )
                    .into(),
            )
            .unwrap();

        // This account still has nothing
        assert!(db.basic_account(signer.address()).unwrap().is_none());
    }

    #[test]
    fn accepts_transaction_from_account_with_excess_balance() {
        let (mut header, mut db, provider) = setup();
        let signer = LocalSigner::random();

        // Create a TX with random data
        let transaction_signed = {
            let mut inner_tx = TxEip1559::default();

            inner_tx.input = hex!("0000000011223344").as_ref().into();
            inner_tx.gas_limit = 21080;
            inner_tx.max_fee_per_gas = 1;
            inner_tx.chain_id = 1;
            inner_tx.to = TxKind::Call(Address::default());

            let signature = signer.sign_transaction_sync(&mut inner_tx).unwrap();
            let tx = Transaction::Eip1559(inner_tx.into());
            TransactionSigned::new_unhashed(tx, signature)
        };

        let exact_gas_used = 21080;

        // We know this is the exact gas used
        header.gas_used = exact_gas_used;
        // And the expected receipts root
        header.receipts_root =
            B256::from(hex!("5240c13baa9d1e0d29a6c984ba919cb949d4c1a9ceb74060760c90e4d1fcd765"));

        let user_account =
            Account { nonce: 0, balance: U256::from(exact_gas_used + 1), bytecode_hash: None };
        db.insert_account(signer.address(), user_account.clone(), None, HashMap::default());

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
                        senders: vec![signer.address()],
                    },
                    U256::ZERO,
                )
                    .into(),
            )
            .unwrap();

        // This account has exact same balance
        assert_eq!(db.basic_account(signer.address()).unwrap().unwrap(), user_account);
    }

    #[test]
    fn accepts_transaction_from_account_with_a_little_balance() {
        let (mut header, mut db, provider) = setup();
        let signer = LocalSigner::random();

        // Create a TX with random data
        let transaction_signed = {
            let mut inner_tx = TxEip1559::default();

            inner_tx.input = hex!("0000000011223344").as_ref().into();
            inner_tx.gas_limit = 21080;
            inner_tx.max_fee_per_gas = 1;
            inner_tx.chain_id = 1;
            inner_tx.to = TxKind::Call(Address::default());

            let signature = signer.sign_transaction_sync(&mut inner_tx).unwrap();
            let tx = Transaction::Eip1559(inner_tx.into());
            TransactionSigned::new_unhashed(tx, signature)
        };

        let exact_gas_used = 21080;

        // We know this is the exact gas used
        header.gas_used = exact_gas_used;
        // And the expected receipts root
        header.receipts_root =
            B256::from(hex!("5240c13baa9d1e0d29a6c984ba919cb949d4c1a9ceb74060760c90e4d1fcd765"));

        let user_account =
            Account { nonce: 0, balance: U256::from(exact_gas_used - 100), bytecode_hash: None };
        db.insert_account(signer.address(), user_account.clone(), None, HashMap::default());

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
                        senders: vec![signer.address()],
                    },
                    U256::ZERO,
                )
                    .into(),
            )
            .unwrap();

        // This account has exact same balance
        assert_eq!(db.basic_account(signer.address()).unwrap().unwrap(), user_account);
    }
}

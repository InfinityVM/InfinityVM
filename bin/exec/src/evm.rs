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
    use reth_revm::{
        database::StateProviderDatabase, test_utils::StateProviderTest, TransitionState,
    };
    use reth::chainspec::ChainSpecBuilder;
    use reth::chainspec::MAINNET;
    use reth::primitives::EthereumHardfork;
    use reth::primitives::ForkCondition;
    use reth::primitives::BlockWithSenders;
    use reth::primitives::Block;
    use reth::primitives::BlockBody;
    use reth_evm::execute::BlockExecutorProvider;
    use alloy::primitives::B256;
    use reth_evm::execute::BatchExecutor;
    use alloy::consensus::TxEip1559;
    use alloy::primitives::hex;
    use alloy::primitives::TxKind;
    use alloy::primitives::Address;
    use reth::primitives::Transaction;
    use alloy::signers::local::LocalSigner;
    use alloy::network::
    TxSignerSync;

    fn executor_provider(
        chain_spec: Arc<ChainSpec>,
    ) -> BasicBlockExecutorProvider<EthExecutionStrategyFactory<IvmEvmConfig>> {
        let evm_config = IvmEvmConfig::new(chain_spec.clone());
        let strategy_factory = EthExecutionStrategyFactory::new(chain_spec.clone(), evm_config);

        BasicBlockExecutorProvider::new(strategy_factory)
    }

    fn setup() -> (Header, StateProviderTest, BasicBlockExecutorProvider<EthExecutionStrategyFactory<IvmEvmConfig>>) {
        let header = Header {
            timestamp: 1,
            number: 1,
            parent_beacon_block_root: Some(B256::with_last_byte(0x69)),
            excess_blob_gas: Some(0),
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

    #[test]
    fn execute_empty_block() {
        let header = Header {
            timestamp: 1,
            number: 1,
            parent_beacon_block_root: Some(B256::with_last_byte(0x69)),
            excess_blob_gas: Some(0),
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
    fn accepts_transaction_from_account_with_no_gas() {
        let (header, db, provider) = setup();

        let signer = LocalSigner::random();

        
        let mut inner_tx = TxEip1559::default();
        inner_tx.input = hex!("0000000011223344").as_ref().into();
        inner_tx.gas_limit = 1_000_000_000;
        inner_tx.max_fee_per_gas = 1_000_000_000;
        inner_tx.to = TxKind::Call(dest);
        
        let signature = signer.sign_transaction_sync(&mut inner_tx);
        let dest = Address::default();

        // let tx_body = Transaction::Eip1559(inner_tx);


        // let transaction = TransactionSigned::from_transaction_and_signature();

        // provider
        //     .batch_executor(StateProviderDatabase::new(&db))
        //     .execute_and_verify_one(
        //         (
        //             &BlockWithSenders {
        //                 block: Block {
        //                     header,
        //                     body: BlockBody {
        //                         transactions: vec![],
        //                         ommers: vec![],
        //                         withdrawals: None,
        //                     },
        //                 },
        //                 senders: vec![],
        //             },
        //             U256::ZERO,
        //         )
        //             .into(),
        //     )
        //     .unwrap()
    }
}

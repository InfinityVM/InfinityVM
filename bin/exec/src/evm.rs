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
use reth_node_ethereum::{
    node::{EthereumAddOns, EthereumPayloadBuilder},
    BasicBlockExecutorProvider, EthExecutionStrategyFactory, EthereumNode,
};
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

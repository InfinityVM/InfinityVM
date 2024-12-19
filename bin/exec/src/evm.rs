use std::{sync::Arc};
use reth::chainspec::{Chain, ChainSpec};
use alloy::consensus::Header;
use alloy::genesis::Genesis;
use alloy::primitives::{address, Address, Bytes, U256};
use reth::{
    builder::{
        components::{ExecutorBuilder, PayloadServiceBuilder},
        BuilderContext, NodeBuilder,
    },
    payload::{EthBuiltPayload, EthPayloadBuilderAttributes},
    revm::{
        handler::register::EvmHandler,
        inspector_handle_register,
        precompile::{Precompile, PrecompileOutput, PrecompileSpecId},
        primitives::{BlockEnv, CfgEnvWithHandlerCfg, Env, PrecompileResult, TxEnv},
        ContextPrecompiles, Database, Evm, EvmBuilder, GetInspector,
    },
    rpc::types::engine::PayloadAttributes,
    tasks::TaskManager,
    transaction_pool::{PoolTransaction, TransactionPool},
};
use reth_evm_ethereum::EthEvmConfig;
use reth_node_api::{
    ConfigureEvm, ConfigureEvmEnv, FullNodeTypes, NextBlockEnvAttributes, NodeTypes,
    NodeTypesWithEngine, PayloadTypes,
};
use reth_node_core::{args::RpcServerArgs, node_config::NodeConfig};
use reth_node_ethereum::{
    node::{EthereumAddOns, EthereumPayloadBuilder},
    BasicBlockExecutorProvider, EthExecutionStrategyFactory, EthereumNode,
};
use reth::primitives::{EthPrimitives, TransactionSigned};
use reth_tracing::{RethTracer, Tracer};

/// IVM's EVM configuration
#[derive(Debug, Clone)]
pub struct IvmEvmConfig {
    /// Wrapper around mainnet configuration
    eth: EthEvmConfig,
}

impl IvmEvmConfig {
    pub const fn new(chain_spec: Arc<ChainSpec>) -> Self {
        Self { eth: EthEvmConfig::new(chain_spec) }
    }
}
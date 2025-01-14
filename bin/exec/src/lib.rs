//! IVM execution client types for plugging into reth node builder.

use crate::engine::IvmEngineValidatorBuilder;
use evm::IvmExecutorBuilder;
use payload::IvmPayloadBuilder;
use pool::{validator::IvmTransactionAllowConfig, IvmPoolBuilder};
use reth_chainspec::ChainSpec;
use reth_ethereum_engine_primitives::{
    EthBuiltPayload, EthPayloadAttributes, EthPayloadBuilderAttributes,
};
use reth_network::NetworkHandle;
use reth_node_api::{NodeTypes, NodeTypesWithEngine, PayloadTypes};
use reth_node_builder::{
    components::ComponentsBuilder, rpc::RpcAddOns, FullNodeComponents, FullNodeTypes, Node, NodeAdapter, NodeComponentsBuilder,
};
use reth_node_ethereum::{
    node::{EthereumConsensusBuilder, EthereumNetworkBuilder},
    EthereumNode,
};
use reth_primitives::EthPrimitives;
use reth_rpc::eth::EthApi;
use std::path::PathBuf;

pub mod config;
pub mod engine;
pub mod evm;
pub mod payload;
pub mod pool;

/// RPC add on that includes [`IvmEngineValidatorBuilder`].
pub type IvmAddOns<N> = RpcAddOns<
    N,
    EthApi<
        <N as FullNodeTypes>::Provider,
        <N as FullNodeComponents>::Pool,
        NetworkHandle,
        <N as FullNodeComponents>::Evm,
    >,
    IvmEngineValidatorBuilder,
>;

/// Add IVM specific arguments to the default reth cli.
#[derive(Debug, Clone, clap::Args)]
pub struct IvmCliExt {
    /// Path to an IVM config toml file. Defaults to using a config in `<RETH
    /// DATADIR>/ivm_config.toml`. If no file is found one is generated.
    #[arg(long)]
    pub ivm_config: Option<PathBuf>,

    /// Do not enforce the transaction allow config; allow transactions from all senders.
    /// This will override values in the IVM config.
    #[arg(long = "tx-allow.all")]
    pub allow_all: bool,
}

// struct IvmNode;

// impl<N> Node<N> for IvmNode
// where
//     N: FullNodeTypes<Types = Self>,
// {
//     type ComponentsBuilder = ComponentsBuilder<
//         N,
//         IvmPoolBuilder,
//         IvmPayloadBuilder,
//         EthereumNetworkBuilder,
//         IvmExecutorBuilder,
//         EthereumConsensusBuilder,
//     >;

//     type AddOns = IvmAddOns<
//         NodeAdapter<N, <Self::ComponentsBuilder as NodeComponentsBuilder<N>>::Components>,
//     >;

//     /// Denies all transactions by default
//     fn components_builder(&self) -> Self::ComponentsBuilder {
//         ivm_components(IvmTransactionAllowConfig::default())
//     }

//     fn add_ons(&self) -> Self::AddOns {
//         IvmAddOns::default()
//     }
// }

/// Returns a `ComponentsBuilder` configured for an IVM node.
pub fn ivm_components<Node>(
    transaction_allow: IvmTransactionAllowConfig,
) -> ComponentsBuilder<
    Node,
    IvmPoolBuilder,
    IvmPayloadBuilder,
    EthereumNetworkBuilder,
    IvmExecutorBuilder,
    EthereumConsensusBuilder,
>
where
    Node: FullNodeTypes<Types: NodeTypes<ChainSpec = ChainSpec, Primitives = EthPrimitives>>,
    <Node::Types as NodeTypesWithEngine>::Engine: PayloadTypes<
        BuiltPayload = EthBuiltPayload,
        PayloadAttributes = EthPayloadAttributes,
        PayloadBuilderAttributes = EthPayloadBuilderAttributes,
    >,
{
    let pool_builder = IvmPoolBuilder::new(transaction_allow);

    EthereumNode::components()
        .pool(pool_builder)
        .executor(IvmExecutorBuilder)
        .payload(IvmPayloadBuilder::default())
}

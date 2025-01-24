//! IVM execution client types for plugging into reth node builder.

use crate::engine::IvmEngineValidatorBuilder;
use config::IvmConfig;
use evm::IvmExecutorBuilder;
use payload::IvmPayloadBuilder;
use pool::IvmPoolBuilder;
use reth_chainspec::ChainSpec;
use reth_ethereum_engine_primitives::{
    EthBuiltPayload, EthPayloadAttributes, EthPayloadBuilderAttributes,
};
use reth_network::NetworkHandle;
use reth_node_api::{NodeTypes, NodeTypesWithEngine, PayloadTypes};
use reth_node_builder::{
    components::ComponentsBuilder, rpc::RpcAddOns, FullNodeComponents, FullNodeTypes, Node,
    NodeAdapter, NodeComponentsBuilder,
};
use reth_node_ethereum::{
    node::{EthereumConsensusBuilder, EthereumNetworkBuilder},
    EthEngineTypes, EthereumNode,
};
use reth_primitives::EthPrimitives;
use reth_provider::EthStorage;
use reth_rpc::eth::EthApi;
use reth_trie_db::MerklePatriciaTrie;
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

/// Type configuration for an IVM node.
#[derive(Debug, Clone)]
pub struct IvmNode {
    ivm_config: IvmConfig,
}

impl IvmNode {
    /// Create an IVM node with the given allow list.
    pub const fn new(ivm_config: IvmConfig) -> Self {
        Self { ivm_config }
    }

    /// Returns a [`ComponentsBuilder`] configured for an IVM node.
    pub fn components<Node>(
        &self,
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
        let pool_builder = IvmPoolBuilder::new(self.ivm_config.clone());

        EthereumNode::components()
            .pool(pool_builder)
            .executor(IvmExecutorBuilder)
            .payload(IvmPayloadBuilder::default())
    }
}

impl NodeTypes for IvmNode {
    type Primitives = EthPrimitives;
    type ChainSpec = ChainSpec;
    type StateCommitment = MerklePatriciaTrie;
    type Storage = EthStorage;
}

impl NodeTypesWithEngine for IvmNode {
    type Engine = EthEngineTypes;
}

impl<N> Node<N> for IvmNode
where
    N: FullNodeTypes<Types = Self>,
{
    type ComponentsBuilder = ComponentsBuilder<
        N,
        IvmPoolBuilder,
        IvmPayloadBuilder,
        EthereumNetworkBuilder,
        IvmExecutorBuilder,
        EthereumConsensusBuilder,
    >;

    type AddOns = IvmAddOns<
        NodeAdapter<N, <Self::ComponentsBuilder as NodeComponentsBuilder<N>>::Components>,
    >;

    fn components_builder(&self) -> Self::ComponentsBuilder {
        self.components()
    }

    fn add_ons(&self) -> Self::AddOns {
        IvmAddOns::default()
    }
}

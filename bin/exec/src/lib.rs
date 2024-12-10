use reth::{
    builder::{
        components::ComponentsBuilder, rpc::RpcAddOns, EngineObjectValidationError, EngineTypes,
        EngineValidator, FullNodeComponents, FullNodeTypes, Node, NodeAdapter,
        NodeComponentsBuilder, NodeTypes, NodeTypesWithDB, NodeTypesWithEngine, PayloadTypes,
    },
    chainspec::ChainSpec,
    network::NetworkHandle,
    payload::{EthBuiltPayload, EthPayloadBuilderAttributes, ExecutionPayloadValidator},
    primitives::EthPrimitives,
    providers::EthStorage,
    rpc::eth::EthApi,
};
use reth_ethereum_engine_primitives::EthPayloadAttributes;
use reth_node_ethereum::{
    node::{
        EthereumConsensusBuilder, EthereumEngineValidatorBuilder, EthereumExecutorBuilder,
        EthereumNetworkBuilder, EthereumPayloadBuilder, EthereumPoolBuilder,
    },
    EthEngineTypes,
};
use reth_trie_db::MerklePatriciaTrie;
use std::sync::Arc;
// use reth::builder::Block;
use reth::{
    api::InvalidPayloadAttributesError,
    builder::{
        rpc::EngineValidatorBuilder, validate_version_specific_fields, AddOnsContext,
        EngineApiMessageVersion, PayloadOrAttributes,
    },
    primitives::{Block, SealedBlockFor},
    rpc::types::engine::{ExecutionPayload, ExecutionPayloadSidecar, PayloadError},
};

/// Type configuration for a regular Ethereum node.
#[derive(Debug, Default, Clone, Copy)]
#[non_exhaustive]
pub struct EthereumNode;

impl EthereumNode {
    /// Returns a [`ComponentsBuilder`] configured for a regular Ethereum node.
    pub fn components<Node>() -> ComponentsBuilder<
        Node,
        EthereumPoolBuilder,
        EthereumPayloadBuilder,
        EthereumNetworkBuilder,
        EthereumExecutorBuilder,
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
        ComponentsBuilder::default()
            .node_types::<Node>()
            .pool(EthereumPoolBuilder::default())
            .payload(EthereumPayloadBuilder::default())
            .network(EthereumNetworkBuilder::default())
            .executor(EthereumExecutorBuilder::default())
            .consensus(EthereumConsensusBuilder::default())
    }
}

impl NodeTypes for EthereumNode {
    type Primitives = EthPrimitives;
    type ChainSpec = ChainSpec;
    type StateCommitment = MerklePatriciaTrie;
    type Storage = EthStorage;
}

impl NodeTypesWithEngine for EthereumNode {
    type Engine = EthEngineTypes;
}

/// Add-ons w.r.t. l1 ethereum.
pub type EthereumAddOns<N> = RpcAddOns<
    N,
    EthApi<
        <N as FullNodeTypes>::Provider,
        <N as FullNodeComponents>::Pool,
        NetworkHandle,
        <N as FullNodeComponents>::Evm,
    >,
    EthereumEngineValidatorBuilder,
>;

impl<Types, N> Node<N> for EthereumNode
where
    Types: NodeTypesWithDB
        + NodeTypesWithEngine<
            Engine = EthEngineTypes,
            ChainSpec = ChainSpec,
            Primitives = EthPrimitives,
            Storage = EthStorage,
        >,
    N: FullNodeTypes<Types = Types>,
{
    type ComponentsBuilder = ComponentsBuilder<
        N,
        EthereumPoolBuilder,
        EthereumPayloadBuilder,
        EthereumNetworkBuilder,
        EthereumExecutorBuilder,
        EthereumConsensusBuilder,
    >;

    type AddOns = EthereumAddOns<
        NodeAdapter<N, <Self::ComponentsBuilder as NodeComponentsBuilder<N>>::Components>,
    >;

    fn components_builder(&self) -> Self::ComponentsBuilder {
        Self::components()
    }

    fn add_ons(&self) -> Self::AddOns {
        EthereumAddOns::default()
    }
}

/// Ivm engine validator
#[derive(Debug, Clone)]
pub struct IvmEngineValidator {
    inner: ExecutionPayloadValidator<ChainSpec>,
}

impl IvmEngineValidator {
    /// Instantiates a new validator.
    pub const fn new(chain_spec: Arc<ChainSpec>) -> Self {
        Self { inner: ExecutionPayloadValidator::new(chain_spec) }
    }

    /// Returns the chain spec used by the validator.
    #[inline]
    fn chain_spec(&self) -> &ChainSpec {
        self.inner.chain_spec()
    }
}

impl<T> EngineValidator<T> for IvmEngineValidator
where
    T: EngineTypes<PayloadAttributes = EthPayloadAttributes>,
{
    type Block = Block;

    fn validate_version_specific_fields(
        &self,
        version: EngineApiMessageVersion,
        payload_or_attrs: PayloadOrAttributes<'_, T::PayloadAttributes>,
    ) -> Result<(), EngineObjectValidationError> {
        validate_version_specific_fields(self.chain_spec(), version, payload_or_attrs)

        // self.inner.validate_version_specific_fields(version, attributes)?;
    }

    fn ensure_well_formed_attributes(
        &self,
        version: EngineApiMessageVersion,
        attributes: &T::PayloadAttributes,
    ) -> Result<(), EngineObjectValidationError> {
        validate_version_specific_fields(self.chain_spec(), version, attributes.into())?;

        // self.inner.ensure_well_formed_attributes(version, attributes)?;

        Ok(())
    }

    fn ensure_well_formed_payload(
        &self,
        payload: ExecutionPayload,
        sidecar: ExecutionPayloadSidecar,
    ) -> Result<SealedBlockFor<Self::Block>, PayloadError> {
        self.inner.ensure_well_formed_payload(payload, sidecar)
    }

    fn validate_payload_attributes_against_header(
        &self,
        _attr: &<T as PayloadTypes>::PayloadAttributes,
        _header: &<Self::Block as reth::api::Block>::Header,
    ) -> Result<(), InvalidPayloadAttributesError> {
        // skip default timestamp validation
        Ok(())
    }
}

/// Ivm engine validator builder
#[derive(Debug, Default, Clone, Copy)]
#[non_exhaustive]
pub struct IvmEngineValidatorBuilder;

impl<N> EngineValidatorBuilder<N> for IvmEngineValidatorBuilder
where
    N: FullNodeComponents<
        Types: NodeTypesWithEngine<Engine = EthEngineTypes, ChainSpec = ChainSpec>,
    >,
{
    type Validator = IvmEngineValidator;

    async fn build(self, ctx: &AddOnsContext<'_, N>) -> eyre::Result<Self::Validator> {
        Ok(IvmEngineValidator::new(ctx.config.chain.clone()))
    }
}

// impl<N> EngineValidatorAddOn<N> for OpAddOns<N>
// where
//     N: FullNodeComponents<Types: NodeTypes<ChainSpec = OpChainSpec>>,
//     OpEngineValidator: EngineValidator<<N::Types as NodeTypesWithEngine>::Engine>,
// {
//     type Validator = OpEngineValidator;

//     async fn engine_validator(&self, ctx: &AddOnsContext<'_, N>) -> eyre::Result<Self::Validator>
// {         OpEngineValidatorBuilder::default().build(ctx).await
//     }
// }

use reth_node_ethereum::node::{
  EthereumPoolBuilder, 
  EthereumPayloadBuilder, 
  EthereumNetworkBuilder, 
  EthereumExecutorBuilder, 
  EthereumConsensusBuilder
};
use reth::builder::components::ComponentsBuilder;
use reth::builder::NodeAdapter;
use reth::builder::NodeComponentsBuilder;
use reth::primitives::EthPrimitives;
use reth::providers::EthStorage;
use reth::chainspec::ChainSpec;
use reth_node_ethereum::EthEngineTypes;
use reth::builder::NodeTypesWithEngine;
use reth::builder::NodeTypesWithDB;
use reth::builder::Node;
use reth_node_ethereum::node::EthereumEngineValidatorBuilder;
use reth::builder::FullNodeComponents;
use reth::network::NetworkHandle;
use reth::rpc::eth::EthApi;
use reth::builder::rpc::RpcAddOns;
use reth::builder::FullNodeTypes;
use reth::builder::NodeTypes;
use reth::builder::PayloadTypes;
use reth::payload::EthBuiltPayload;
use reth::payload::EthPayloadBuilderAttributes;
use reth_ethereum_engine_primitives::EthPayloadAttributes;
use reth_trie_db::MerklePatriciaTrie;

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

// /// Custom engine validator
// #[derive(Debug, Clone)]
// pub struct CustomEngineValidator {
//     inner: ExecutionPayloadValidator<ChainSpec>,
// }

// impl CustomEngineValidator {
//     /// Instantiates a new validator.
//     pub const fn new(chain_spec: Arc<ChainSpec>) -> Self {
//         Self { inner: ExecutionPayloadValidator::new(chain_spec) }
//     }

//     /// Returns the chain spec used by the validator.
//     #[inline]
//     fn chain_spec(&self) -> &ChainSpec {
//         self.inner.chain_spec()
//     }
// }

// impl<T> EngineValidator<T> for CustomEngineValidator
// where
//     T: EngineTypes<PayloadAttributes = CustomPayloadAttributes>,
// {
//     type Block = Block;

//     fn validate_version_specific_fields(
//         &self,
//         version: EngineApiMessageVersion,
//         payload_or_attrs: PayloadOrAttributes<'_, T::PayloadAttributes>,
//     ) -> Result<(), EngineObjectValidationError> {
//         validate_version_specific_fields(self.chain_spec(), version, payload_or_attrs)
//     }

//     fn ensure_well_formed_attributes(
//         &self,
//         version: EngineApiMessageVersion,
//         attributes: &T::PayloadAttributes,
//     ) -> Result<(), EngineObjectValidationError> {
//         validate_version_specific_fields(self.chain_spec(), version, attributes.into())?;

//         // custom validation logic - ensure that the custom field is not zero
//         if attributes.custom == 0 {
//             return Err(EngineObjectValidationError::invalid_params(
//                 CustomError::CustomFieldIsNotZero,
//             ))
//         }

//         Ok(())
//     }

//     fn ensure_well_formed_payload(
//         &self,
//         payload: ExecutionPayload,
//         sidecar: ExecutionPayloadSidecar,
//     ) -> Result<SealedBlockFor<Self::Block>, PayloadError> {
//         self.inner.ensure_well_formed_payload(payload, sidecar)
//     }

//     fn validate_payload_attributes_against_header(
//         &self,
//         _attr: &<T as PayloadTypes>::PayloadAttributes,
//         _header: &<Self::Block as reth::api::Block>::Header,
//     ) -> Result<(), InvalidPayloadAttributesError> {
//         // skip default timestamp validation
//         Ok(())
//     }
// }

// /// Custom engine validator builder
// #[derive(Debug, Default, Clone, Copy)]
// #[non_exhaustive]
// pub struct CustomEngineValidatorBuilder;

// impl<N> EngineValidatorBuilder<N> for CustomEngineValidatorBuilder
// where
//     N: FullNodeComponents<
//         Types: NodeTypesWithEngine<Engine = CustomEngineTypes, ChainSpec = ChainSpec>,
//     >,
// {
//     type Validator = CustomEngineValidator;

//     async fn build(self, ctx: &AddOnsContext<'_, N>) -> eyre::Result<Self::Validator> {
//         Ok(CustomEngineValidator::new(ctx.config.chain.clone()))
//     }
// }

// impl<N> EngineValidatorAddOn<N> for OpAddOns<N>
// where
//     N: FullNodeComponents<Types: NodeTypes<ChainSpec = OpChainSpec>>,
//     OpEngineValidator: EngineValidator<<N::Types as NodeTypesWithEngine>::Engine>,
// {
//     type Validator = OpEngineValidator;

//     async fn engine_validator(&self, ctx: &AddOnsContext<'_, N>) -> eyre::Result<Self::Validator> {
//         OpEngineValidatorBuilder::default().build(ctx).await
//     }
// }
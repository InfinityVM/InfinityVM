//! IVM Engine API validator.

use reth::{
    api::InvalidPayloadAttributesError,
    builder::{
        rpc::EngineValidatorBuilder, validate_version_specific_fields, AddOnsContext,
        EngineApiMessageVersion, EngineObjectValidationError, EngineTypes, EngineValidator,
        FullNodeComponents, NodeTypesWithEngine, PayloadOrAttributes, PayloadTypes,
        PayloadValidator,
    },
    chainspec::ChainSpec,
    payload::ExecutionPayloadValidator,
    primitives::{Block, EthPrimitives, SealedBlockFor},
    rpc::types::engine::{ExecutionPayload, ExecutionPayloadSidecar, PayloadError},
};
use reth_ethereum_engine_primitives::EthPayloadAttributes;
use reth_node_ethereum::EthEngineTypes;
use std::sync::Arc;

/// Engine API validation logic for IVM.
///
/// The primary divergence from the Engine API spec is that we do not check the block
/// timestamp, which allows us to achieve sub second block times without modifying the header type.
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

impl PayloadValidator for IvmEngineValidator {
    type Block = Block;

    fn ensure_well_formed_payload(
        &self,
        payload: ExecutionPayload,
        sidecar: ExecutionPayloadSidecar,
    ) -> Result<SealedBlockFor<Self::Block>, PayloadError> {
        self.inner.ensure_well_formed_payload(payload, sidecar)
    }
}

impl<Types> EngineValidator<Types> for IvmEngineValidator
where
    Types: EngineTypes<PayloadAttributes = EthPayloadAttributes>,
{
    fn validate_version_specific_fields(
        &self,
        version: EngineApiMessageVersion,
        payload_or_attrs: PayloadOrAttributes<'_, Types::PayloadAttributes>,
    ) -> Result<(), EngineObjectValidationError> {
        validate_version_specific_fields(self.chain_spec(), version, payload_or_attrs)
    }

    fn ensure_well_formed_attributes(
        &self,
        version: EngineApiMessageVersion,
        attributes: &Types::PayloadAttributes,
    ) -> Result<(), EngineObjectValidationError> {
        validate_version_specific_fields(self.chain_spec(), version, attributes.into())
    }

    fn validate_payload_attributes_against_header(
        &self,
        _attr: &<Types as PayloadTypes>::PayloadAttributes,
        _header: &<Self::Block as reth::api::Block>::Header,
    ) -> Result<(), InvalidPayloadAttributesError> {
        // skip timestamp validation
        Ok(())
    }
}

/// Builder for [`IvmEngineValidator`].
#[derive(Debug, Default, Clone)]
pub struct IvmEngineValidatorBuilder;

impl<Node, Types> EngineValidatorBuilder<Node> for IvmEngineValidatorBuilder
where
    Types: NodeTypesWithEngine<
        ChainSpec = ChainSpec,
        Engine = EthEngineTypes,
        Primitives = EthPrimitives,
    >,
    Node: FullNodeComponents<Types = Types>,
{
    type Validator = IvmEngineValidator;

    async fn build(self, ctx: &AddOnsContext<'_, Node>) -> eyre::Result<Self::Validator> {
        Ok(IvmEngineValidator::new(ctx.config.chain.clone()))
    }
}

//! IVM Engine API validator.

use reth::{
    api::InvalidPayloadAttributesError,
    builder::{
        EngineObjectValidationError, EngineTypes, EngineValidator, FullNodeComponents,
        NodeTypesWithEngine, PayloadTypes,
    },
    chainspec::ChainSpec,
    payload::ExecutionPayloadValidator,
    primitives::SealedBlockFor,
};
use reth::{
    builder::{
        rpc::EngineValidatorBuilder, validate_version_specific_fields, AddOnsContext,
        EngineApiMessageVersion, PayloadOrAttributes,
    },
    primitives::Block,
    rpc::types::engine::{ExecutionPayload, ExecutionPayloadSidecar, PayloadError},
};
use reth_node_ethereum::EthEngineTypes;

use reth_ethereum_engine_primitives::EthPayloadAttributes;
use std::sync::Arc;

/// Engine API validation logic for IVM.
///
/// The primary divergence from the Engine API spec is that we do not check the block
/// timestamp, which allows us 
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
    }

    fn ensure_well_formed_attributes(
        &self,
        version: EngineApiMessageVersion,
        attributes: &T::PayloadAttributes,
    ) -> Result<(), EngineObjectValidationError> {
        validate_version_specific_fields(self.chain_spec(), version, attributes.into())
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
        // skip timestamp validation
        Ok(())
    }
}

/// IVM engine validator builder.
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

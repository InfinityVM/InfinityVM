//! IVM Engine API validator.

use reth_node_api::InvalidPayloadAttributesError;
use reth_node_builder::{
        rpc::EngineValidatorBuilder, AddOnsContext, EngineApiMessageVersion,
        EngineObjectValidationError, EngineTypes, EngineValidator, FullNodeComponents,
        NodeTypesWithEngine, PayloadOrAttributes, PayloadTypes, PayloadValidator,
    };
use reth_chainspec::ChainSpec;
use reth_payload_validator::ExecutionPayloadValidator;
use reth_primitives::{Block, EthPrimitives, SealedBlockFor};
use alloy_rpc_types::engine::{ExecutionPayload, ExecutionPayloadSidecar, PayloadError};
use reth_ethereum_engine_primitives::EthPayloadAttributes;
use reth_node_ethereum::{node::EthereumEngineValidator, EthEngineTypes};
use std::sync::Arc;

/// Engine API validation logic for IVM.
///
/// The primary divergence from the Engine API spec is that we do not check the block
/// timestamp, which allows us to achieve sub second block times without modifying the header type.
#[derive(Debug, Clone)]
pub struct IvmEngineValidator {
    payload_validator: ExecutionPayloadValidator<ChainSpec>,
    engine_validator: EthereumEngineValidator,
}

impl IvmEngineValidator {
    /// Instantiates a new validator.
    pub fn new(chain_spec: Arc<ChainSpec>) -> Self {
        Self {
            payload_validator: ExecutionPayloadValidator::new(chain_spec.clone()),
            engine_validator: EthereumEngineValidator::new(chain_spec),
        }
    }
}

impl PayloadValidator for IvmEngineValidator {
    type Block = Block;

    fn ensure_well_formed_payload(
        &self,
        payload: ExecutionPayload,
        sidecar: ExecutionPayloadSidecar,
    ) -> Result<SealedBlockFor<Self::Block>, PayloadError> {
        self.payload_validator.ensure_well_formed_payload(payload, sidecar)
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
        <EthereumEngineValidator as EngineValidator<Types>>::validate_version_specific_fields(
            &self.engine_validator,
            version,
            payload_or_attrs,
        )
    }

    fn ensure_well_formed_attributes(
        &self,
        version: EngineApiMessageVersion,
        attributes: &Types::PayloadAttributes,
    ) -> Result<(), EngineObjectValidationError> {
        <EthereumEngineValidator as EngineValidator<Types>>::ensure_well_formed_attributes(
            &self.engine_validator,
            version,
            attributes,
        )
    }

    fn validate_payload_attributes_against_header(
        &self,
        _attr: &<Types as PayloadTypes>::PayloadAttributes,
        _header: &<Self::Block as reth_node_api::Block>::Header,
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

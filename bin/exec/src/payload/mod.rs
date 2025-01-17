//! IVM payload builder.

use alloy_rpc_types::engine::PayloadAttributes;

use reth::payload::{PayloadBuilderHandle, PayloadBuilderService};
use reth_basic_payload_builder::{BasicPayloadJobGenerator, BasicPayloadJobGeneratorConfig};
use reth_chainspec::ChainSpec;
use reth_ethereum_engine_primitives::{EthBuiltPayload, EthPayloadBuilderAttributes};
use reth_ethereum_payload_builder::EthereumBuilderConfig;
use reth_evm::ConfigureEvm;
use reth_node_api::{FullNodeTypes, HeaderTy, NodeTypesWithEngine, PayloadTypes, TxTy};
use reth_node_builder::{components::PayloadServiceBuilder, BuilderContext, PayloadBuilderConfig};
use reth_primitives::{EthPrimitives, TransactionSigned};
use reth_provider::CanonStateSubscriptions;
use reth_transaction_pool::{PoolTransaction, TransactionPool};
use reth_ethereum_engine_primitives::EthPayloadAttributes;
use crate::evm::IvmEvmConfig;

pub mod builder;

/// Builds a regular ethereum block executor that uses the custom EVM.
#[derive(Debug, Default, Clone)]
#[non_exhaustive]
pub struct IvmPayloadServiceBuilder {
    inner: IvmPayloadBuilderSpawner,
}

impl<Types, Node, Pool> PayloadServiceBuilder<Node, Pool> for IvmPayloadServiceBuilder
where
    Types: NodeTypesWithEngine<ChainSpec = ChainSpec, Primitives = EthPrimitives>,
    Node: FullNodeTypes<Types = Types>,
    Pool: TransactionPool<Transaction: PoolTransaction<Consensus = TransactionSigned>>
        + Unpin
        + 'static,
    Types::Engine: PayloadTypes<
        BuiltPayload = EthBuiltPayload,
        PayloadAttributes = PayloadAttributes,
        PayloadBuilderAttributes = EthPayloadBuilderAttributes,
    >,
{
    async fn spawn_payload_service(
        self,
        ctx: &BuilderContext<Node>,
        pool: Pool,
    ) -> eyre::Result<reth::payload::PayloadBuilderHandle<Types::Engine>> {
        self.inner.spawn(IvmEvmConfig::new(ctx.chain_spec()), ctx, pool)
    }
}

#[derive(Default, Debug, Clone)]
struct IvmPayloadBuilderSpawner;

impl IvmPayloadBuilderSpawner {
    /// A helper method initializing [`IvmPayloadBuilderSpawner`] with the given EVM config.
    pub fn spawn<Types, Node, Evm, Pool>(
        self,
        evm_config: Evm,
        ctx: &BuilderContext<Node>,
        pool: Pool,
    ) -> eyre::Result<PayloadBuilderHandle<Types::Engine>>
    where
        Types: NodeTypesWithEngine<ChainSpec = ChainSpec, Primitives = EthPrimitives>,
        Node: FullNodeTypes<Types = Types>,
        Evm: ConfigureEvm<Header = HeaderTy<Types>, Transaction = TxTy<Node::Types>>,
        Pool: TransactionPool<Transaction: PoolTransaction<Consensus = TxTy<Node::Types>>>
            + Unpin
            + 'static,
        Types::Engine: PayloadTypes<
            BuiltPayload = EthBuiltPayload,
            PayloadAttributes = EthPayloadAttributes,
            PayloadBuilderAttributes = EthPayloadBuilderAttributes,
        >,
    {
        let conf = ctx.payload_builder_config();
        // TODO(zeke): replace with IvmPayloadBuilder
        let payload_builder = reth_ethereum_payload_builder::EthereumPayloadBuilder::new(
            evm_config,
            EthereumBuilderConfig::new(conf.extra_data_bytes()).with_gas_limit(conf.gas_limit()),
        );

        let payload_job_config = BasicPayloadJobGeneratorConfig::default()
            .interval(conf.interval())
            .deadline(conf.deadline())
            .max_payload_tasks(conf.max_payload_tasks());

        let payload_generator = BasicPayloadJobGenerator::with_builder(
            ctx.provider().clone(),
            pool,
            ctx.task_executor().clone(),
            payload_job_config,
            payload_builder,
        );
        let (payload_service, payload_builder) =
            PayloadBuilderService::new(payload_generator, ctx.provider().canonical_state_stream());

        ctx.task_executor().spawn_critical("payload builder service", Box::pin(payload_service));

        Ok(payload_builder)
    }
}

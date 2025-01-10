//! IVM payload building logic. This

use alloy_rpc_types::engine::PayloadAttributes;
use reth_chainspec::ChainSpec;
use reth_ethereum_engine_primitives::{EthBuiltPayload, EthPayloadBuilderAttributes};
use reth_node_api::{FullNodeTypes, NodeTypesWithEngine, PayloadTypes};
use reth_node_builder::{components::PayloadServiceBuilder, BuilderContext};
use reth_node_ethereum::node::EthereumPayloadBuilder;
use reth_primitives::{EthPrimitives, TransactionSigned};
use reth_transaction_pool::{PoolTransaction, TransactionPool};

use crate::evm::IvmEvmConfig;

// TODO: add test that payload building service and be re-executed with same state root
// TODO: update priority logic (best_tx), to not use priority fee amd either be random or FIFO

/// Builds a regular ethereum block executor that uses the custom EVM.
#[derive(Debug, Default, Clone)]
#[non_exhaustive]
pub struct IvmPayloadBuilder {
    inner: EthereumPayloadBuilder,
}

impl<Types, Node, Pool> PayloadServiceBuilder<Node, Pool> for IvmPayloadBuilder
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

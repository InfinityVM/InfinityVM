use crate::pool::validator::{IvmTransactionAllowConfig, IvmTransactionValidator};
use reth::{
    api::NodeTypes,
    builder::{components::PoolBuilder, BuilderContext, FullNodeTypes},
    chainspec::ChainSpec,
    primitives::EthPrimitives,
    providers::CanonStateSubscriptions,
    transaction_pool::{
        blobstore::InMemoryBlobStore, validate::EthTransactionValidatorBuilder,
        CoinbaseTipOrdering, EthPooledTransaction,
        TransactionValidationTaskExecutor,
    },
};
use tracing::{debug, info};

mod validator;

/// IVM transaction pool builder
#[derive(Debug, Clone, Default)]
pub struct IvmPoolBuilder;

pub type IvmTransactionPool<Client, S> = reth::transaction_pool::Pool<
    TransactionValidationTaskExecutor<IvmTransactionValidator<Client, EthPooledTransaction>>,
    CoinbaseTipOrdering<EthPooledTransaction>,
    S,
>;
/// Implement the [`PoolBuilder`] trait for the custom pool builder
///
/// This will be used to build the transaction pool and its maintenance tasks during launch.
impl<Node> PoolBuilder<Node> for IvmPoolBuilder
where
    Node: FullNodeTypes<Types: NodeTypes<ChainSpec = ChainSpec, Primitives = EthPrimitives>>,
{
    // TODO: use DiskFileBlobStore instead
    type Pool = IvmTransactionPool<Node::Provider, InMemoryBlobStore>;

    // TODO: [now] check this against the reth build pool function
    async fn build_pool(self, ctx: &BuilderContext<Node>) -> eyre::Result<Self::Pool> {
        let data_dir = ctx.config().datadir();
        let blob_store = InMemoryBlobStore::default();
        let pool_config = ctx.pool_config();

        let validator = {
            // Configure the standard eth tx validator.
            // TODO: this should be encapsulated in the IvmTransactionValidator build logic
            let eth_transaction_validator = EthTransactionValidatorBuilder::new(ctx.chain_spec())
                .with_head_timestamp(ctx.head().timestamp)
                .kzg_settings(ctx.kzg_settings()?)
                .build(ctx.provider().clone(), blob_store.clone());

            // Configure IVM tx validator (which uses the eth tx validator) and spawn service
            IvmTransactionValidator::build_with_tasks(
                ctx.task_executor().clone(),
                eth_transaction_validator,
                IvmTransactionAllowConfig::default(),
                ctx.config().txpool.additional_validation_tasks,
            )
        };

        let transaction_pool = reth::transaction_pool::Pool::new(
            validator,
            CoinbaseTipOrdering::default(),
            blob_store,
            pool_config,
        );
        // let transaction_pool =
        //     reth::transaction_pool::Pool::eth_pool(validator, blob_store, );
        info!(target: "reth::cli", "Transaction pool initialized");
        let transactions_path = data_dir.txpool_transactions();

        // spawn txpool maintenance task
        {
            let pool = transaction_pool.clone();
            let chain_events = ctx.provider().canonical_state_stream();
            let client = ctx.provider().clone();
            let transactions_backup_config =
                reth::transaction_pool::maintain::LocalTransactionBackupConfig::with_local_txs_backup(transactions_path);

            ctx.task_executor().spawn_critical_with_graceful_shutdown_signal(
                "local transactions backup task",
                |shutdown| {
                    reth::transaction_pool::maintain::backup_local_transactions_task(
                        shutdown,
                        pool.clone(),
                        transactions_backup_config,
                    )
                },
            );

            // spawn the maintenance task
            ctx.task_executor().spawn_critical(
                "txpool maintenance task",
                reth::transaction_pool::maintain::maintain_transaction_pool_future(
                    client,
                    pool,
                    chain_events,
                    ctx.task_executor().clone(),
                    Default::default(),
                ),
            );
            debug!(target: "reth::cli", "Spawned txpool maintenance task");
        }

        Ok(transaction_pool)
    }
}

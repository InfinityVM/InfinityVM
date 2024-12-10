/// IVM transaction pool. This is intended to be identical to the standard reth pool
/// with the one difference being additional transaction validation to allow for
/// free transaction submission without spam.
use alloy::primitives::Address;
// use futures_util::{lock::Mutex, StreamExt};
use tokio::sync::Mutex
;
use reth::{
    api::NodeTypes,
    builder::{components::PoolBuilder, BuilderContext, FullNodeTypes},
    chainspec::ChainSpec,
    primitives::{EthPrimitives, InvalidTransactionError, SealedBlock},
    providers::{CanonStateSubscriptions, StateProviderFactory},
    tasks::TaskSpawner,
    transaction_pool::{
        blobstore::InMemoryBlobStore,
        validate::{EthTransactionValidatorBuilder, ValidationTask},
        BlobStore, EthPoolTransaction, EthTransactionPool, EthTransactionValidator, PoolConfig,
        TransactionOrigin, TransactionValidationOutcome, TransactionValidationTaskExecutor,
        TransactionValidator,
    },
};
use std::{collections::HashSet, sync::Arc};
use tracing::{debug, info};

#[derive(Debug, Clone, Default)]
struct IvmTransactionAllowConfig {
    allow_all: bool,
    to: HashSet<Address>,
    from: HashSet<Address>,
}

impl IvmTransactionAllowConfig {
    fn is_allowed(&self, from: &Address, to: &Address) -> bool {
        if self.allow_all {
            return true;
        }

        if self.to.contains(to) {
            return true;
        }

        self.from.contains(from)
    }
}

/// A custom pool builder
#[derive(Debug, Clone, Default)]
#[non_exhaustive]
pub struct IvmPoolBuilder {
    // TODO: [now] get the pool config from context
    pool_config: PoolConfig,
}

impl IvmPoolBuilder {
    fn new(pool_config: PoolConfig) -> Self {
        Self { pool_config }
    }
}

/// Implement the [`PoolBuilder`] trait for the custom pool builder
///
/// This will be used to build the transaction pool and its maintenance tasks during launch.
impl<Node> PoolBuilder<Node> for IvmPoolBuilder
where
    Node: FullNodeTypes<Types: NodeTypes<ChainSpec = ChainSpec, Primitives = EthPrimitives>>,
{
    type Pool = EthTransactionPool<Node::Provider, InMemoryBlobStore>;

    // TODO: [now] check this against the reth build pool function
    async fn build_pool(self, ctx: &BuilderContext<Node>) -> eyre::Result<Self::Pool> {
        let data_dir = ctx.config().datadir();
        let blob_store = InMemoryBlobStore::default();

        // {
        //     // Configure the standard eth tx validator.
        //     // TODO: this should be encapsulated in the IvmTransactionValidator build logic
        //     let eth_transaction_validator = EthTransactionValidatorBuilder::new(ctx.chain_spec())
        //         .with_head_timestamp(ctx.head().timestamp)
        //         .kzg_settings(ctx.kzg_settings()?)
        //         .build(ctx.provider().clone(), blob_store.clone());

        //     // Configure IVM tx validator (which uses the eth tx validator) and spawn service
        //     let validator0 = IvmTransactionValidator {
        //         inner: eth_transaction_validator,
        //         allow_config: IvmTransactionAllowConfig::default(),
        //     }
        //     .build_with_tasks(
        //         ctx.task_executor().clone(),
        //         ctx.config().txpool.additional_validation_tasks,
        //     );
        // }

        // TODO: replace this with above once everything compiles
        let validator = TransactionValidationTaskExecutor::eth_builder(ctx.chain_spec())
            .with_head_timestamp(ctx.head().timestamp)
            .kzg_settings(ctx.kzg_settings()?)
            .with_additional_tasks(ctx.config().txpool.additional_validation_tasks)
            .build_with_tasks(
                ctx.provider().clone(),
                ctx.task_executor().clone(),
                blob_store.clone(),
            );

        let transaction_pool =
            reth::transaction_pool::Pool::eth_pool(validator, blob_store, self.pool_config);
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

#[derive(Debug)]
struct IvmTransactionValidator<Client, Tx> {
    inner: EthTransactionValidator<Client, Tx>,
    allow_config: IvmTransactionAllowConfig,
}

impl<Client, Tx> IvmTransactionValidator<Client, Tx>
where
    Client: StateProviderFactory,
    Tx: EthPoolTransaction,
{
    /// Validates a single transaction.
    ///
    /// See also [`TransactionValidator::validate_transaction`]
    pub fn validate_one(
        &self,
        origin: TransactionOrigin,
        transaction: Tx,
    ) -> TransactionValidationOutcome<Tx> {
        // First check that the transaction obeys allow lists. We check this first
        // to reduce heavy checks for eip 4844 transactions.
        let sender = transaction.sender_ref();
        /// TODO
        if !self.allow_config.is_allowed(sender, sender) {
            return TransactionValidationOutcome::Invalid(
                transaction,
                InvalidTransactionError::TxTypeNotSupported.into(),
            );
        }

        self.inner.validate_one(origin, transaction)

    }

    /// Validates all given transactions.
    ///
    /// Returns all outcomes for the given transactions in the same order.
    ///
    /// See also [`Self::validate_one`]
    pub fn validate_all(
        &self,
        transactions: Vec<(TransactionOrigin, Tx)>,
    ) -> Vec<TransactionValidationOutcome<Tx>> {
        transactions.into_iter().map(|(origin, tx)| self.validate_one(origin, tx)).collect()
    }

    /// Builds a [`IvmTransactionValidator`] and spawns validation tasks via the
    /// [`TransactionValidationTaskExecutor`]
    ///
    /// The validator will spawn `additional_tasks` additional tasks for validation.
    ///
    /// By default this will spawn 1 additional task.
    pub fn build_with_tasks<T>(
        self,
        tasks: T,
        additional_tasks: usize,
    ) -> TransactionValidationTaskExecutor<IvmTransactionValidator<Client, Tx>>
    where
        T: TaskSpawner,
    {
        let validator = self;

        let (tx, task) = ValidationTask::new();

        // Spawn validation tasks, they are blocking because they perform db lookups
        for _ in 0..additional_tasks {
            let task = task.clone();
            tasks.spawn_blocking(Box::pin(async move {
                task.run().await;
            }));
        }

        // we spawn them on critical tasks because validation, especially for EIP-4844 can be quite
        // heavy
        tasks.spawn_critical_blocking(
            "transaction-validation-service",
            Box::pin(async move {
                task.run().await;
            }),
        );

        let to_validation_task = Arc::new(Mutex::new(tx));

        TransactionValidationTaskExecutor { validator, to_validation_task }
    }
}

impl<Client, Tx> TransactionValidator for IvmTransactionValidator<Client, Tx>
where
    Client: StateProviderFactory,
    Tx: EthPoolTransaction,
{
    type Transaction = Tx;

    async fn validate_transaction(
        &self,
        origin: TransactionOrigin,
        transaction: Self::Transaction,
    ) -> TransactionValidationOutcome<Self::Transaction> {
        self.validate_one(origin, transaction)
    }

    async fn validate_transactions(
        &self,
        transactions: Vec<(TransactionOrigin, Self::Transaction)>,
    ) -> Vec<TransactionValidationOutcome<Self::Transaction>> {
        self.validate_all(transactions)
    }

    fn on_new_head_block(&self, new_tip_block: &SealedBlock) {
        self.inner.on_new_head_block(new_tip_block)
    }
}



use reth::{
    api::NodeTypes,
    builder::{components::PoolBuilder, BuilderContext, FullNodeTypes},
    chainspec::ChainSpec,
    cli::Cli,
    primitives::EthPrimitives,
    providers::CanonStateSubscriptions,
    transaction_pool::{
        blobstore::InMemoryBlobStore, EthTransactionPool, TransactionValidationTaskExecutor,
    },
};
use reth::transaction_pool::PoolConfig;
use tracing::{info, debug};
use reth::providers::StateProviderFactory;
use reth::transaction_pool::EthPoolTransaction;
use reth::transaction_pool::TransactionOrigin;
use reth::transaction_pool::TransactionValidationOutcome;
use reth::transaction_pool::EthTransactionValidator;
use reth::primitives::SealedBlock;
use reth::transaction_pool::TransactionValidator;
use reth::transaction_pool::LocalTransactionConfig;
use alloy::primitives::Address;
use std::collections::HashSet;
use reth::primitives::InvalidTransactionError;

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
pub struct CustomPoolBuilder {
    /// Use custom pool config
    pool_config: PoolConfig,
}

/// Implement the [`PoolBuilder`] trait for the custom pool builder
///
/// This will be used to build the transaction pool and its maintenance tasks during launch.
impl<Node> PoolBuilder<Node> for CustomPoolBuilder
where
    Node: FullNodeTypes<Types: NodeTypes<ChainSpec = ChainSpec, Primitives = EthPrimitives>>,
{
    type Pool = EthTransactionPool<Node::Provider, InMemoryBlobStore>;

    // TODO: [now] check this against the reth build pool function
    async fn build_pool(self, ctx: &BuilderContext<Node>) -> eyre::Result<Self::Pool> {
        let data_dir = ctx.config().datadir();
        let blob_store = InMemoryBlobStore::default();
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
        let outcome = self.inner.validate_one(origin, transaction.clone());
        let is_valid = outcome.is_valid();

        if !is_valid {
          return outcome;
        }

        let sender = transaction.sender_ref();
        if self.allow_config.is_allowed(sender, sender) {
          outcome
        } else {
          TransactionValidationOutcome::Invalid(
                transaction,
                InvalidTransactionError::TxTypeNotSupported.into(),
            )
        }
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

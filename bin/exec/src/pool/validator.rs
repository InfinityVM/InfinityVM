//! IVM has a custom transaction validator that performs allow list checks on top of the standard
//! ethereum transaction checks.

use alloy::primitives::Address;
use reth::{
    primitives::{InvalidTransactionError, SealedBlock},
    providers::StateProviderFactory,
    tasks::TaskSpawner,
    transaction_pool::{
        validate::ValidationTask, BlobStore, EthPoolTransaction, EthTransactionValidator,
        TransactionOrigin, TransactionValidationOutcome, TransactionValidationTaskExecutor,
        TransactionValidator,
    },
};
use std::{collections::HashSet, sync::Arc};
use tokio::sync::Mutex;

/// Configuration for allow list based on sender and recipient.
#[derive(Debug, Clone, Default)]
pub struct IvmTransactionAllowConfig {
    allow_all: bool,
    to: HashSet<Address>,
    from: HashSet<Address>,
}

impl IvmTransactionAllowConfig {
    /// If the transaction passes allow list checks.
    pub fn is_allowed(&self, from: &Address, to: &Address) -> bool {
        if self.allow_all {
            return true;
        }

        if self.to.contains(to) {
            return true;
        }

        self.from.contains(from)
    }
}

#[derive(Debug)]
pub struct IvmTransactionValidator<Client, Tx> {
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
        tasks: T,
        inner: EthTransactionValidator<Client, Tx>,
        allow_config: IvmTransactionAllowConfig,
        additional_tasks: usize,
    ) -> TransactionValidationTaskExecutor<IvmTransactionValidator<Client, Tx>>
    where
        T: TaskSpawner,
    {
        let validator = Self { inner, allow_config };

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

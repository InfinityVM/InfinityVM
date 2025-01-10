//! IVM has a custom transaction validator that performs allow list checks on top of the standard
//! ethereum transaction checks.

use alloy_primitives::Address;
use reth_primitives::{InvalidTransactionError, SealedBlock};
use reth_provider::StateProviderFactory;
use reth_tasks::TaskSpawner;
use reth_transaction_pool::{
    validate::ValidationTask, EthPoolTransaction, EthTransactionValidator, TransactionOrigin,
    TransactionValidationOutcome, TransactionValidationTaskExecutor, TransactionValidator,
};

use std::{collections::HashSet, sync::Arc};
use tokio::sync::Mutex;

/// Configuration for allow list based on sender and recipient.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default)]
pub struct IvmTransactionAllowConfig {
    all: bool,
    to: HashSet<Address>,
    sender: HashSet<Address>,
}

impl IvmTransactionAllowConfig {
    /// If the transaction passes allow list checks.
    pub fn is_allowed(&self, sender: &Address, to: Option<Address>) -> bool {
        if self.all {
            return true;
        }

        if let Some(to) = to {
            if self.to.contains(&to) {
                return true;
            }
        }

        self.sender.contains(sender)
    }
}

/// IVM transaction pool validator.
#[derive(Debug, Clone)]
pub struct IvmTransactionValidator<Client, Tx> {
    eth: EthTransactionValidator<Client, Tx>,
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
        tx: Tx,
    ) -> TransactionValidationOutcome<Tx> {
        // First check that the transaction obeys allow lists. We check this first
        // to reduce heavy checks for eip 4844 transactions
        if !self.allow_config.is_allowed(tx.sender_ref(), tx.to()) {
            return TransactionValidationOutcome::Invalid(
                tx,
                InvalidTransactionError::TxTypeNotSupported.into(),
            );
        }

        // Complete standard eth validation checks
        self.eth.validate_one(origin, tx)
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
        eth: EthTransactionValidator<Client, Tx>,
        allow_config: IvmTransactionAllowConfig,
        additional_tasks: usize,
    ) -> TransactionValidationTaskExecutor<Self>
    where
        T: TaskSpawner,
    {
        let validator = Self::new(eth, allow_config);

        let (tx, task) = ValidationTask::new();

        // Spawn validation tasks, they are blocking because they perform db lookups
        for _ in 0..additional_tasks {
            let task = task.clone();
            tasks.spawn_blocking(Box::pin(async move {
                task.run().await;
            }));
        }

        // We spawn them on critical tasks because validation, especially for EIP-4844 can be quite
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

    pub(crate) const fn new(
        eth: EthTransactionValidator<Client, Tx>,
        allow_config: IvmTransactionAllowConfig,
    ) -> Self {
        Self { eth, allow_config }
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

    fn on_new_head_block<H, B>(&self, new_tip_block: &SealedBlock<H, B>)
    where
        H: reth_primitives_traits::BlockHeader,
        B: reth_primitives_traits::BlockBody,
    {
        self.eth.on_new_head_block(new_tip_block)
    }
}

#[cfg(test)]
mod test {
    use super::IvmTransactionAllowConfig;
    use alloy_primitives::Address;
    use std::collections::HashSet;

    // Helpers to set/get values for tests
    impl IvmTransactionAllowConfig {
        pub(crate) fn deny_all() -> Self {
            Self { to: HashSet::new(), sender: HashSet::new(), all: false }
        }

        pub(crate) fn set_to(&mut self, to: HashSet<Address>) {
            self.to = to;
        }

        pub(crate) fn to(&self) -> HashSet<Address> {
            self.to.clone()
        }

        pub(crate) fn set_sender(&mut self, sender: HashSet<Address>) {
            self.sender = sender;
        }

        pub(crate) fn sender(&self) -> HashSet<Address> {
            self.sender.clone()
        }

        pub(crate) fn set_all(&mut self, allow_all: bool) {
            self.all = allow_all;
        }

        pub(crate) const fn all(&self) -> bool {
            self.all
        }
    }
}

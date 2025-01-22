//! IVM has a custom transaction validator that performs allow list checks on top of the standard
//! ethereum transaction checks.

use alloy_consensus::{
    constants::{
        EIP1559_TX_TYPE_ID, EIP2930_TX_TYPE_ID, EIP4844_TX_TYPE_ID, EIP7702_TX_TYPE_ID,
        LEGACY_TX_TYPE_ID,
    },
    BlockHeader,
};
use alloy_eips::{
    eip1559::ETHEREUM_BLOCK_GAS_LIMIT,
    eip4844::{env_settings::EnvKzgSettings, MAX_BLOBS_PER_BLOCK},
};
use alloy_primitives::U256;
use reth_chainspec::{ChainSpec, EthereumHardforks};
use reth_primitives::{InvalidTransactionError, SealedBlock};
use reth_provider::{StateProvider, StateProviderFactory};
use reth_tasks::TaskSpawner;
use reth_transaction_pool::{
    error::{
        Eip4844PoolTransactionError, Eip7702PoolTransactionError, InvalidPoolTransactionError,
    },
    validate::{
        ensure_intrinsic_gas, ForkTracker, ValidTransaction, ValidationTask,
        DEFAULT_MAX_TX_INPUT_BYTES, MAX_INIT_CODE_BYTE_SIZE,
    },
    BlobStore, EthBlobTransactionSidecar, EthPoolTransaction, LocalTransactionConfig,
    TransactionOrigin, TransactionValidationOutcome, TransactionValidationTaskExecutor,
    TransactionValidator,
};
use std::{
    marker::PhantomData,
    sync::{
        atomic::{AtomicBool, AtomicU64},
        Arc,
    },
};
use tokio::sync::Mutex;

use crate::config::IvmConfig;

/// N.B. This is almost a direct copy of `EthTransactionValidatorBuilder` in reth. Just modified to
/// return our type that does not include balance validation logic. <https://github.com/InfinityVM/reth/blob/28d52312acd46be2bfc46661a7b392feaa2bd4c5/crates/transaction-pool/src/validate/eth.rs#L535>.
///
/// A builder for [`TransactionValidationTaskExecutor`]
#[derive(Debug)]
pub struct IvmTransactionValidatorBuilder {
    chain_spec: Arc<ChainSpec>,
    /// Fork indicator whether we are in the Shanghai stage.
    shanghai: bool,
    /// Fork indicator whether we are in the Cancun hardfork.
    cancun: bool,
    /// Fork indicator whether we are in the Cancun hardfork.
    prague: bool,
    /// Whether using EIP-2718 type transactions is allowed
    eip2718: bool,
    /// Whether using EIP-1559 type transactions is allowed
    eip1559: bool,
    /// Whether using EIP-4844 type transactions is allowed
    eip4844: bool,
    /// Whether using EIP-7702 type transactions is allowed
    eip7702: bool,
    /// The current max gas limit
    block_gas_limit: AtomicU64,
    /// Minimum priority fee to enforce for acceptance into the pool.
    minimum_priority_fee: Option<u128>,
    /// Determines how many additional tasks to spawn
    ///
    /// Default is 1
    additional_tasks: usize,

    /// Stores the setup and parameters needed for validating KZG proofs.
    kzg_settings: EnvKzgSettings,
    /// How to handle [`TransactionOrigin::Local`](TransactionOrigin) transactions.
    local_transactions_config: LocalTransactionConfig,
    /// Max size in bytes of a single transaction allowed
    max_tx_input_bytes: usize,
}

impl IvmTransactionValidatorBuilder {
    /// Creates a new builder for the given [`ChainSpec`]
    ///
    /// By default this assumes the network is on the `Cancun` hardfork and the following
    /// transactions are allowed:
    ///  - Legacy
    ///  - EIP-2718
    ///  - EIP-1559
    ///  - EIP-4844
    pub fn new(chain_spec: Arc<ChainSpec>) -> Self {
        Self {
            block_gas_limit: ETHEREUM_BLOCK_GAS_LIMIT.into(),
            chain_spec,
            minimum_priority_fee: None,
            additional_tasks: 1,
            kzg_settings: EnvKzgSettings::Default,
            local_transactions_config: Default::default(),
            max_tx_input_bytes: DEFAULT_MAX_TX_INPUT_BYTES,

            // by default all transaction types are allowed
            eip2718: true,
            eip1559: true,
            eip4844: true,
            eip7702: true,

            // shanghai is activated by default
            shanghai: true,

            // cancun is activated by default
            cancun: true,

            // prague not yet activated
            prague: false,
        }
    }

    /// Disables the Cancun fork.
    pub const fn no_cancun(self) -> Self {
        self.set_cancun(false)
    }

    /// Whether to allow exemptions for local transaction exemptions.
    pub fn with_local_transactions_config(
        mut self,
        local_transactions_config: LocalTransactionConfig,
    ) -> Self {
        self.local_transactions_config = local_transactions_config;
        self
    }

    /// Set the Cancun fork.
    pub const fn set_cancun(mut self, cancun: bool) -> Self {
        self.cancun = cancun;
        self
    }

    /// Disables the Shanghai fork.
    pub const fn no_shanghai(self) -> Self {
        self.set_shanghai(false)
    }

    /// Set the Shanghai fork.
    pub const fn set_shanghai(mut self, shanghai: bool) -> Self {
        self.shanghai = shanghai;
        self
    }

    /// Disables the Prague fork.
    pub const fn no_prague(self) -> Self {
        self.set_prague(false)
    }

    /// Set the Prague fork.
    pub const fn set_prague(mut self, prague: bool) -> Self {
        self.prague = prague;
        self
    }

    /// Disables the support for EIP-2718 transactions.
    pub const fn no_eip2718(self) -> Self {
        self.set_eip2718(false)
    }

    /// Set the support for EIP-2718 transactions.
    pub const fn set_eip2718(mut self, eip2718: bool) -> Self {
        self.eip2718 = eip2718;
        self
    }

    /// Disables the support for EIP-1559 transactions.
    pub const fn no_eip1559(self) -> Self {
        self.set_eip1559(false)
    }

    /// Set the support for EIP-1559 transactions.
    pub const fn set_eip1559(mut self, eip1559: bool) -> Self {
        self.eip1559 = eip1559;
        self
    }

    /// Disables the support for EIP-4844 transactions.
    pub const fn no_eip4844(self) -> Self {
        self.set_eip4844(false)
    }

    /// Set the support for EIP-4844 transactions.
    pub const fn set_eip4844(mut self, eip4844: bool) -> Self {
        self.eip4844 = eip4844;
        self
    }

    /// Sets the [`EnvKzgSettings`] to use for validating KZG proofs.
    pub fn kzg_settings(mut self, kzg_settings: EnvKzgSettings) -> Self {
        self.kzg_settings = kzg_settings;
        self
    }

    /// Sets a minimum priority fee that's enforced for acceptance into the pool.
    pub const fn with_minimum_priority_fee(mut self, minimum_priority_fee: u128) -> Self {
        self.minimum_priority_fee = Some(minimum_priority_fee);
        self
    }

    /// Sets the number of additional tasks to spawn.
    pub const fn with_additional_tasks(mut self, additional_tasks: usize) -> Self {
        self.additional_tasks = additional_tasks;
        self
    }

    /// Configures validation rules based on the head block's timestamp.
    ///
    /// For example, whether the Shanghai and Cancun hardfork is activated at launch.
    pub fn with_head_timestamp(mut self, timestamp: u64) -> Self {
        self.cancun = self.chain_spec.is_cancun_active_at_timestamp(timestamp);
        self.shanghai = self.chain_spec.is_shanghai_active_at_timestamp(timestamp);
        self.prague = self.chain_spec.is_prague_active_at_timestamp(timestamp);
        self
    }

    /// Sets a max size in bytes of a single transaction allowed into the pool
    pub const fn with_max_tx_input_bytes(mut self, max_tx_input_bytes: usize) -> Self {
        self.max_tx_input_bytes = max_tx_input_bytes;
        self
    }

    /// Sets the block gas limit
    ///
    /// Transactions with a gas limit greater than this will be rejected.
    pub fn set_block_gas_limit(self, block_gas_limit: u64) -> Self {
        self.block_gas_limit.store(block_gas_limit, std::sync::atomic::Ordering::Relaxed);
        self
    }

    /// Builds a the [`IvmTransactionValidator`] without spawning validator tasks.
    pub fn build<Client, Tx, S>(
        self,
        client: Client,
        blob_store: S,
        ivm_config: IvmConfig,
    ) -> IvmTransactionValidator<Client, Tx>
    where
        S: BlobStore,
    {
        let Self {
            chain_spec,
            shanghai,
            cancun,
            prague,
            eip2718,
            eip1559,
            eip4844,
            eip7702,
            block_gas_limit,
            minimum_priority_fee,
            kzg_settings,
            local_transactions_config,
            max_tx_input_bytes,
            ..
        } = self;

        let fork_tracker = ForkTracker {
            shanghai: AtomicBool::new(shanghai),
            cancun: AtomicBool::new(cancun),
            prague: AtomicBool::new(prague),
        };

        let inner = IvmTransactionValidatorInner {
            chain_spec,
            client,
            eip2718,
            eip1559,
            fork_tracker,
            eip4844,
            eip7702,
            block_gas_limit,
            minimum_priority_fee,
            blob_store: Box::new(blob_store),
            kzg_settings,
            local_transactions_config,
            max_tx_input_bytes,
            _marker: Default::default(),
        };

        let ivm_only_inner =
            IvmOnlyTransactionValidatorInner { ivm_config, latest_timestamp: AtomicU64::default() };
        IvmTransactionValidator { eth: Arc::new(inner), ivm: Arc::new(ivm_only_inner) }
    }

    /// Builds a [`IvmTransactionValidator`] and spawns validation tasks via the
    /// [`TransactionValidationTaskExecutor`]
    ///
    /// The validator will spawn `additional_tasks` additional tasks for validation.
    ///
    /// By default this will spawn 1 additional task.
    pub fn build_with_tasks<Client, Tx, T, S>(
        self,
        client: Client,
        tasks: T,
        blob_store: S,
        ivm_config: IvmConfig,
    ) -> TransactionValidationTaskExecutor<IvmTransactionValidator<Client, Tx>>
    where
        T: TaskSpawner,
        S: BlobStore,
    {
        let additional_tasks = self.additional_tasks;
        let validator = self.build(client, blob_store, ivm_config);

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
}

/// N.B. this is largely a copy of `IvmTransactionValidatorInner` <https://github.com/paradigmxyz/reth/blob/d761ac42f5e15c46c00db90ca1b5b6f7f62a4f7c/crates/transaction-pool/src/validate/eth.rs#L133>
/// We try and stay as close as possible to the reth version to make it clear where we differ.
/// The primary differences are that:
///
/// 1) We do not check that the account balance is greater then transaction cost.
/// 2) We return a hardcoded number for the current account balance that should ensure downstream
///    checks pass.
///
///
///
/// A [`TransactionValidator`] implementation that validates ethereum transaction.
///
/// It supports all known ethereum transaction types:
/// - Legacy
/// - EIP-2718
/// - EIP-1559
/// - EIP-4844
/// - EIP-7702
///
/// And enforces additional constraints such as:
/// - Maximum transaction size
/// - Maximum gas limit
///
/// And adheres to the configured [`LocalTransactionConfig`].
#[derive(Debug)]
pub(crate) struct IvmTransactionValidatorInner<Client, T> {
    /// Spec of the chain
    chain_spec: Arc<ChainSpec>,
    /// This type fetches account info from the db
    client: Client,
    /// Blobstore used for fetching re-injected blob transactions.
    blob_store: Box<dyn BlobStore>,
    /// tracks activated forks relevant for transaction validation
    fork_tracker: ForkTracker,
    /// Fork indicator whether we are using EIP-2718 type transactions.
    eip2718: bool,
    /// Fork indicator whether we are using EIP-1559 type transactions.
    eip1559: bool,
    /// Fork indicator whether we are using EIP-4844 blob transactions.
    eip4844: bool,
    /// Fork indicator whether we are using EIP-7702 type transactions.
    eip7702: bool,
    /// The current max gas limit
    block_gas_limit: AtomicU64,
    /// Minimum priority fee to enforce for acceptance into the pool.
    minimum_priority_fee: Option<u128>,
    /// Stores the setup and parameters needed for validating KZG proofs.
    kzg_settings: EnvKzgSettings,
    /// How to handle [`TransactionOrigin::Local`](TransactionOrigin) transactions.
    local_transactions_config: LocalTransactionConfig,
    /// Maximum size in bytes a single transaction can have in order to be accepted into the pool.
    max_tx_input_bytes: usize,
    /// Marker for the transaction type
    _marker: PhantomData<T>,
}

impl<Client, Tx> IvmTransactionValidatorInner<Client, Tx> {
    /// Returns the configured chain id
    pub(crate) fn chain_id(&self) -> u64 {
        self.chain_spec.chain().id()
    }
}

impl<Client, Tx> IvmTransactionValidatorInner<Client, Tx>
where
    Client: StateProviderFactory,
    Tx: EthPoolTransaction,
{
    /// Validates a single transaction using an optional cached state provider.
    /// If no provider is passed, a new one will be created. This allows reusing
    /// the same provider across multiple txs.
    fn validate_one_with_provider(
        &self,
        origin: TransactionOrigin,
        mut transaction: Tx,
        maybe_state: &mut Option<Box<dyn StateProvider>>,
    ) -> TransactionValidationOutcome<Tx> {
        // Checks for tx_type
        match transaction.tx_type() {
            LEGACY_TX_TYPE_ID => {
                // Accept legacy transactions
            }
            EIP2930_TX_TYPE_ID => {
                // Accept only legacy transactions until EIP-2718/2930 activates
                if !self.eip2718 {
                    return TransactionValidationOutcome::Invalid(
                        transaction,
                        InvalidTransactionError::Eip2930Disabled.into(),
                    );
                }
            }
            EIP1559_TX_TYPE_ID => {
                // Reject dynamic fee transactions until EIP-1559 activates.
                if !self.eip1559 {
                    return TransactionValidationOutcome::Invalid(
                        transaction,
                        InvalidTransactionError::Eip1559Disabled.into(),
                    );
                }
            }
            EIP4844_TX_TYPE_ID => {
                // Reject blob transactions.
                if !self.eip4844 {
                    return TransactionValidationOutcome::Invalid(
                        transaction,
                        InvalidTransactionError::Eip4844Disabled.into(),
                    );
                }
            }
            EIP7702_TX_TYPE_ID => {
                // Reject EIP-7702 transactions.
                if !self.eip7702 {
                    return TransactionValidationOutcome::Invalid(
                        transaction,
                        InvalidTransactionError::Eip7702Disabled.into(),
                    );
                }
            }

            _ => {
                return TransactionValidationOutcome::Invalid(
                    transaction,
                    InvalidTransactionError::TxTypeNotSupported.into(),
                )
            }
        };

        // Reject transactions over defined size to prevent DOS attacks
        let tx_input_len = transaction.input().len();
        if tx_input_len > self.max_tx_input_bytes {
            return TransactionValidationOutcome::Invalid(
                transaction,
                InvalidPoolTransactionError::OversizedData(tx_input_len, self.max_tx_input_bytes),
            );
        }

        // Check whether the init code size has been exceeded.
        if self.fork_tracker.is_shanghai_activated() {
            if let Err(err) = transaction.ensure_max_init_code_size(MAX_INIT_CODE_BYTE_SIZE) {
                return TransactionValidationOutcome::Invalid(transaction, err);
            }
        }

        // Checks for gas limit
        let transaction_gas_limit = transaction.gas_limit();
        let block_gas_limit = self.max_gas_limit();
        if transaction_gas_limit > block_gas_limit {
            return TransactionValidationOutcome::Invalid(
                transaction,
                InvalidPoolTransactionError::ExceedsGasLimit(
                    transaction_gas_limit,
                    block_gas_limit,
                ),
            );
        }

        // Ensure max_priority_fee_per_gas (if EIP1559) is less than max_fee_per_gas if any.
        if transaction.max_priority_fee_per_gas() > Some(transaction.max_fee_per_gas()) {
            return TransactionValidationOutcome::Invalid(
                transaction,
                InvalidTransactionError::TipAboveFeeCap.into(),
            );
        }

        // Drop non-local transactions with a fee lower than the configured fee for acceptance into
        // the pool.
        if !self.local_transactions_config.is_local(origin, transaction.sender_ref()) &&
            transaction.is_eip1559() &&
            transaction.max_priority_fee_per_gas() < self.minimum_priority_fee
        {
            return TransactionValidationOutcome::Invalid(
                transaction,
                InvalidPoolTransactionError::Underpriced,
            );
        }

        // Checks for chainid
        if let Some(chain_id) = transaction.chain_id() {
            if chain_id != self.chain_id() {
                return TransactionValidationOutcome::Invalid(
                    transaction,
                    InvalidTransactionError::ChainIdMismatch.into(),
                );
            }
        }

        if transaction.is_eip7702() {
            // Cancun fork is required for 7702 txs
            if !self.fork_tracker.is_prague_activated() {
                return TransactionValidationOutcome::Invalid(
                    transaction,
                    InvalidTransactionError::TxTypeNotSupported.into(),
                );
            }

            if transaction.authorization_count() == 0 {
                return TransactionValidationOutcome::Invalid(
                    transaction,
                    Eip7702PoolTransactionError::MissingEip7702AuthorizationList.into(),
                );
            }
        }

        if let Err(err) = ensure_intrinsic_gas(&transaction, &self.fork_tracker) {
            return TransactionValidationOutcome::Invalid(transaction, err);
        }

        // light blob tx pre-checks
        if transaction.is_eip4844() {
            // Cancun fork is required for blob txs
            if !self.fork_tracker.is_cancun_activated() {
                return TransactionValidationOutcome::Invalid(
                    transaction,
                    InvalidTransactionError::TxTypeNotSupported.into(),
                );
            }

            let blob_count = transaction.blob_count();
            if blob_count == 0 {
                // no blobs
                return TransactionValidationOutcome::Invalid(
                    transaction,
                    InvalidPoolTransactionError::Eip4844(
                        Eip4844PoolTransactionError::NoEip4844Blobs,
                    ),
                );
            }

            if blob_count > MAX_BLOBS_PER_BLOCK {
                // too many blobs
                return TransactionValidationOutcome::Invalid(
                    transaction,
                    InvalidPoolTransactionError::Eip4844(
                        Eip4844PoolTransactionError::TooManyEip4844Blobs {
                            have: blob_count,
                            permitted: MAX_BLOBS_PER_BLOCK,
                        },
                    ),
                );
            }
        }

        // If we don't have a state provider yet, fetch the latest state
        if maybe_state.is_none() {
            match self.client.latest() {
                Ok(new_state) => {
                    *maybe_state = Some(new_state);
                }
                Err(err) => {
                    return TransactionValidationOutcome::Error(*transaction.hash(), Box::new(err))
                }
            }
        }

        let state = maybe_state.as_deref().expect("provider is set");

        // Use provider to get account info
        let account = match state.basic_account(transaction.sender_ref()) {
            Ok(account) => account.unwrap_or_default(),
            Err(err) => {
                return TransactionValidationOutcome::Error(*transaction.hash(), Box::new(err))
            }
        };

        // Unless Prague is active, the signer account shouldn't have bytecode.
        //
        // If Prague is active, only EIP-7702 bytecode is allowed for the sender.
        //
        // Any other case means that the account is not an EOA, and should not be able to send
        // transactions.
        if let Some(code_hash) = &account.bytecode_hash {
            let is_eip7702 = if self.fork_tracker.is_prague_activated() {
                match state.bytecode_by_hash(code_hash) {
                    Ok(bytecode) => bytecode.unwrap_or_default().is_eip7702(),
                    Err(err) => {
                        return TransactionValidationOutcome::Error(
                            *transaction.hash(),
                            Box::new(err),
                        )
                    }
                }
            } else {
                false
            };

            if !is_eip7702 {
                return TransactionValidationOutcome::Invalid(
                    transaction,
                    InvalidTransactionError::SignerAccountHasBytecode.into(),
                );
            }
        }

        let tx_nonce = transaction.nonce();

        // Checks for nonce
        if tx_nonce < account.nonce {
            return TransactionValidationOutcome::Invalid(
                transaction,
                InvalidTransactionError::NonceNotConsistent { tx: tx_nonce, state: account.nonce }
                    .into(),
            );
        }

        // This is where balance is normally checked
        // https://github.com/paradigmxyz/reth/blob/d761ac42f5e15c46c00db90ca1b5b6f7f62a4f7c/crates/transaction-pool/src/validate/eth.rs#L417

        let mut maybe_blob_sidecar = None;

        // heavy blob tx validation
        if transaction.is_eip4844() {
            // extract the blob from the transaction
            match transaction.take_blob() {
                EthBlobTransactionSidecar::None => {
                    // this should not happen
                    return TransactionValidationOutcome::Invalid(
                        transaction,
                        InvalidTransactionError::TxTypeNotSupported.into(),
                    );
                }
                EthBlobTransactionSidecar::Missing => {
                    // This can happen for re-injected blob transactions (on re-org), since the blob
                    // is stripped from the transaction and not included in a block.
                    // check if the blob is in the store, if it's included we previously validated
                    // it and inserted it
                    if matches!(self.blob_store.contains(*transaction.hash()), Ok(true)) {
                        // validated transaction is already in the store
                    } else {
                        return TransactionValidationOutcome::Invalid(
                            transaction,
                            InvalidPoolTransactionError::Eip4844(
                                Eip4844PoolTransactionError::MissingEip4844BlobSidecar,
                            ),
                        );
                    }
                }
                EthBlobTransactionSidecar::Present(blob) => {
                    // validate the blob
                    if let Err(err) = transaction.validate_blob(&blob, self.kzg_settings.get()) {
                        return TransactionValidationOutcome::Invalid(
                            transaction,
                            InvalidPoolTransactionError::Eip4844(
                                Eip4844PoolTransactionError::InvalidEip4844Blob(err),
                            ),
                        );
                    }
                    // store the extracted blob
                    maybe_blob_sidecar = Some(blob);
                }
            }
        }

        // Return the valid transaction
        TransactionValidationOutcome::Valid {
            // WARNING: this is meant to represent the current account balance. Since we don't
            // require an account to have a balance, we hardcode a high account balance that we
            // are confident will ensure this transaction passes downstream checks.
            // Upstream logic: https://github.com/paradigmxyz/reth/blob/d761ac42f5e15c46c00db90ca1b5b6f7f62a4f7c/crates/transaction-pool/src/validate/eth.rs#L475
            balance: U256::from(u64::MAX),
            state_nonce: account.nonce,
            transaction: ValidTransaction::new(transaction, maybe_blob_sidecar),
            // by this point assume all external transactions should be propagated
            propagate: match origin {
                TransactionOrigin::External => true,
                TransactionOrigin::Local => {
                    self.local_transactions_config.propagate_local_transactions
                }
                TransactionOrigin::Private => false,
            },
        }
    }

    /// Validates a single transaction.
    fn validate_one(
        &self,
        origin: TransactionOrigin,
        transaction: Tx,
    ) -> TransactionValidationOutcome<Tx> {
        let mut provider = None;
        self.validate_one_with_provider(origin, transaction, &mut provider)
    }

    fn on_new_head_block<T: BlockHeader>(&self, new_tip_block: &T) {
        // update all forks
        if self.chain_spec.is_cancun_active_at_timestamp(new_tip_block.timestamp()) {
            self.fork_tracker.cancun.store(true, std::sync::atomic::Ordering::Relaxed);
        }

        if self.chain_spec.is_shanghai_active_at_timestamp(new_tip_block.timestamp()) {
            self.fork_tracker.shanghai.store(true, std::sync::atomic::Ordering::Relaxed);
        }

        if self.chain_spec.is_prague_active_at_timestamp(new_tip_block.timestamp()) {
            self.fork_tracker.prague.store(true, std::sync::atomic::Ordering::Relaxed);
        }

        self.block_gas_limit.store(new_tip_block.gas_limit(), std::sync::atomic::Ordering::Relaxed);
    }

    fn max_gas_limit(&self) -> u64 {
        self.block_gas_limit.load(std::sync::atomic::Ordering::Relaxed)
    }
}

#[derive(Debug)]
struct IvmOnlyTransactionValidatorInner {
    ivm_config: IvmConfig,
    latest_timestamp: AtomicU64,
}

/// IVM transaction pool validator.
#[derive(Debug, Clone)]
pub struct IvmTransactionValidator<Client, Tx> {
    eth: Arc<IvmTransactionValidatorInner<Client, Tx>>,
    ivm: Arc<IvmOnlyTransactionValidatorInner>,
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
        let timestamp =
            self.ivm.latest_timestamp.fetch_add(0, std::sync::atomic::Ordering::Relaxed);
        if !self.ivm.ivm_config.is_allowed(tx.sender_ref(), tx.to(), timestamp) {
            // TODO: https://github.com/InfinityVM/InfinityVM/issues/471
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

    #[inline]
    fn on_new_head_block<H, B>(&self, new_tip_block: &SealedBlock<H, B>)
    where
        H: reth_primitives_traits::BlockHeader,
        B: reth_primitives_traits::BlockBody,
    {
        self.eth.on_new_head_block(new_tip_block.header());
        // Store the latest timestamp
        self.ivm
            .latest_timestamp
            .store(new_tip_block.timestamp(), std::sync::atomic::Ordering::Relaxed);
    }

    #[cfg(test)]
    pub(super) fn set_timestamp(&mut self, timestamp: u64) {
        self
            .ivm
            .latest_timestamp
            .store(timestamp, std::sync::atomic::Ordering::Relaxed);
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
        self.on_new_head_block(new_tip_block)
    }
}

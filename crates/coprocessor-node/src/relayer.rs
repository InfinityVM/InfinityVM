//! Logic to broadcast job result onchain.
//!
//! The primary type is the [`RelayCoordinator`]. The [`RelayCoordinator`] is in charge of managing
//! broadcast queues on a per consumer basis, managing tasks queue polling tasks, and all final
//! broadcasting logic. Other subsystems communicate with the [`RelayCoordinator`] by sending the
//! [Relay] message over a mpsc channel. While the [Relay] channel its the only public API of this
//! subsystem its important to note that this systems updates job statuses in the DB.
//!
//! The actual relaying logic is encapsulated by the [`JobRelayer`] abstraction.

use crate::{
    job_executor::FailureReason,
    metrics::Metrics,
    queue::Queues,
    writer::{Write, WriterMsg},
};
use alloy::{
    hex,
    network::{Ethereum, EthereumWallet, TxSigner},
    primitives::Address,
    providers::{fillers::RecommendedFiller, Provider, ProviderBuilder},
    rpc::types::TransactionReceipt,
    signers::Signature,
    transports::http::reqwest,
};
use contracts::i_job_manager::IJobManager;
use ivm_abi::abi_encode_offchain_job_request;
use ivm_db::{
    get_all_failed_jobs, get_job,
    tables::{Job, RequestType},
};
use ivm_proto::{JobStatusType, RelayStrategy};
use reth_db::Database;
use std::{
    sync::{mpsc::SyncSender, Arc},
    time::Duration,
};
use tokio::{
    sync::{mpsc::Receiver, oneshot},
    time::interval,
};
use tracing::{error, info, span, Instrument, Level};

/// Delay between retrying failed jobs, in milliseconds.m
const JOB_RETRY_DELAY_MS: u64 = 250;

type ReqwestTransport = alloy::transports::http::Http<reqwest::Client>;

type RelayerProvider = alloy::providers::fillers::FillProvider<
    alloy::providers::fillers::JoinFill<
        RecommendedFiller,
        alloy::providers::fillers::WalletFiller<EthereumWallet>,
    >,
    alloy::providers::RootProvider<ReqwestTransport>,
    ReqwestTransport,
    Ethereum,
>;

type JobManagerContract = IJobManager::IJobManagerInstance<ReqwestTransport, RelayerProvider>;

const TX_INCLUSION_ERROR: &str = "relay_error_tx_inclusion_error";
const BROADCAST_ERROR: &str = "relay_error_broadcast_failure";

/// Relayer errors
#[derive(thiserror::Error, Debug)]
pub enum Error {
    /// failed to parse the given http rpc url
    #[error("failed to parse http_rpc_url")]
    HttpRpcUrlParse,
    /// broadcast receiver error - channel may be closed
    #[error("broadcast receiver error - channel may be closed")]
    BroadcastReceiver,
    /// rpc transport error
    #[error(transparent)]
    Rpc(#[from] alloy::transports::RpcError<alloy::transports::TransportErrorKind>),
    /// error while waiting for tx inclusion
    #[error("error while broadcasting tx: {0}")]
    TxBroadcast(alloy::contract::Error),
    /// error while waiting for tx inclusion
    #[error("error while waiting for tx inclusion: {0}")]
    TxInclusion(alloy::transports::RpcError<alloy::transports::TransportErrorKind>),
    /// must call [`JobRelayerBuilder::signer`] before building
    #[error("must call JobRelayerBuilder::signer before building")]
    MissingSigner,
    /// invalid job request type
    #[error("invalid job request type")]
    InvalidJobRequestType,
    /// relay channel closed
    #[error("relay receiver dropped")]
    RelayRxDropped,
    /// database error
    #[error("database error: {0}")]
    Database(#[from] ivm_db::Error),
}

/// A message to the job relayer
#[derive(Debug)]
pub enum Relay {
    /// Queue a job, setting the order it should be relayed in.
    Queue {
        /// Consumer address.
        consumer: [u8; 20],
        /// Job ID.
        job_id: [u8; 32],
    },
    /// The given job is ready to be relayed now
    Now(Box<Job>),
}

/// The service in charge of handling all routines related to relay transactions onchain.
#[derive(Debug)]
pub struct RelayCoordinator<D> {
    writer_tx: SyncSender<WriterMsg>,
    relay_rx: Receiver<Relay>,
    job_relayer: Arc<JobRelayer>,
    db: Arc<D>,
    max_retries: u32,
    metrics: Arc<Metrics>,
}

impl<D> RelayCoordinator<D>
where
    D: Database + 'static,
{
    /// Create a new instance of [Self].
    pub fn new(
        writer_tx: SyncSender<WriterMsg>,
        relay_rx: Receiver<Relay>,
        job_relayer: Arc<JobRelayer>,
        db: Arc<D>,
        max_retries: u32,
        metrics: Arc<Metrics>,
    ) -> Self {
        Self { writer_tx, relay_rx, job_relayer, db, max_retries, metrics }
    }

    /// Start the relay coordinator
    pub async fn start(self) -> Result<(), Error> {
        let db = Arc::clone(&self.db);
        let job_relayer = Arc::clone(&self.job_relayer);
        let metrics = Arc::clone(&self.metrics);
        let max_retries = self.max_retries;
        let writer_tx = self.writer_tx.clone();
        tokio::spawn(async move {
            Self::start_job_retry_task(db, job_relayer, metrics, max_retries, writer_tx).await
        });

        let queues = Queues::new();
        let mut relay_rx = self.relay_rx;

        while let Some(relay) = relay_rx.recv().await {
            match relay {
                Relay::Queue { consumer, job_id } => {
                    let empty_queue = queues.peek_back(consumer).is_none();
                    queues.push_front(consumer, job_id);

                    // Queue pollers exit once a queue is empty. So we know that if the queue is
                    // empty then we need to start a new queue poller.
                    if empty_queue {
                        let queues2 = queues.clone();
                        let db2 = self.db.clone();
                        let relayer2 = self.job_relayer.clone();
                        let writer_tx2 = self.writer_tx.clone();
                        tokio::spawn(async move {
                            Self::start_queue_poller(consumer, queues2, relayer2, writer_tx2, db2)
                                .await
                        });
                    };
                }
                Relay::Now(job) => {
                    match &job.relay_strategy {
                        RelayStrategy::Unordered => {
                            // error metrics are handled internally and the job is written to the
                            // failed table.
                            let job_relayer2 = self.job_relayer.clone();
                            let writer_tx2 = self.writer_tx.clone();
                            tokio::spawn(async move {
                                Self::relay_job_result(*job, job_relayer2, writer_tx2.clone()).await
                            });
                        }
                        RelayStrategy::Ordered => {
                            // There should already be a background task
                        }
                    }
                }
            };
        }

        Err(Error::RelayRxDropped)
    }

    async fn start_queue_poller(
        consumer: [u8; 20],
        queues: Queues,
        relayer: Arc<JobRelayer>,
        writer_tx: SyncSender<WriterMsg>,
        db: Arc<D>,
    ) {
        let mut interval = interval(Duration::from_millis(100));
        interval.tick().await;
        while let Some(job_id) = queues.peek_back(consumer) {
            interval.tick().await;
            let job = match get_job(db.clone(), job_id).expect("job get db error") {
                Some(job) => job,
                // Edge case where the job has not been written yet
                None => continue,
            };

            match job.status.status() {
                JobStatusType::Done => {
                    let _job_id = queues.pop_back(consumer).expect("queue is unexpected empty");
                    debug_assert_eq!(job_id, _job_id);
                    // relay the job below
                }
                JobStatusType::Relayed => {
                    tracing::error!("logical error: a job in the queue was already relayed");
                    continue;
                }
                _ => continue,
            }

            let _ = Self::relay_job_result(job, relayer.clone(), writer_tx.clone())
                .instrument(span!(Level::INFO, "ordered_relay"))
                .await;
        }
    }

    /// Retry jobs that failed to relay
    async fn start_job_retry_task(
        db: Arc<D>,
        job_relayer: Arc<JobRelayer>,
        metrics: Arc<Metrics>,
        max_retries: u32,
        writer_tx: SyncSender<WriterMsg>,
    ) -> Result<(), Error> {
        loop {
            // Jobs that we no longer want to retry
            let mut jobs_to_delete = Vec::new();

            let retry_jobs = match get_all_failed_jobs(db.clone()) {
                Ok(jobs) => jobs,
                Err(e) => {
                    error!("error retrieving relay error jobs: {:?}", e);
                    continue;
                }
            };

            // Retry each once
            for mut job in retry_jobs {
                let id = job.id;
                let result = match job.request_type {
                    RequestType::Onchain => {
                        job_relayer.relay_result_for_onchain_job(job.clone()).await
                    }
                    RequestType::Offchain(_) => {
                        let job_params = (&job).try_into()?;
                        let job_request_payload = abi_encode_offchain_job_request(job_params);
                        job_relayer
                            .relay_result_for_offchain_job(job.clone(), job_request_payload)
                            .await
                    }
                };

                match result {
                    Ok(receipt) => {
                        info!("successfully retried job relay for job: {:?}", hex::encode(id));
                        jobs_to_delete.push(id);

                        // Save the relay tx hash and status to DB
                        job.relay_tx_hash = receipt.transaction_hash.to_vec();
                        job.status.status = JobStatusType::Relayed as i32;
                        job.status.failure_reason = None;

                        // We are not in a rush, so we can wait for the write
                        let (tx, rx) = oneshot::channel();
                        writer_tx
                            .send((Write::JobTable(job.clone()), Some(tx)))
                            .expect("db writer broken");
                        let _ = rx.await;
                    }
                    Err(e) => {
                        if job.status.retries == max_retries {
                            metrics.incr_relay_err(&FailureReason::RelayErrExceedRetry.to_string());
                            jobs_to_delete.push(id);
                            info!(
                                id = hex::encode(id),
                                "queueing un-broadcastable job for deletion"
                            );
                        } else {
                            error!(
                                id = hex::encode(id),
                                ?e,
                                "report this error: failed to retry relaying job",
                            );
                            job.status.retries += 1;

                            // We are not in a rush, so we can wait for the write
                            let (tx, rx) = oneshot::channel();
                            writer_tx
                                .send((Write::FailureJobs(job.clone()), Some(tx)))
                                .expect("db writer broken");
                            let _ = rx.await;
                        }
                    }
                }

                // Before retrying another job, wait to reduce general load on the system
                tokio::time::sleep(Duration::from_millis(JOB_RETRY_DELAY_MS)).await;
            }

            for job_id in &jobs_to_delete {
                // We are not in a rush, so we can wait for the write
                let (tx, rx) = oneshot::channel();
                writer_tx
                    .send((Write::FailureJobsDelete(*job_id), Some(tx)))
                    .expect("db writer broken");
                let _ = rx.await;
            }
            tokio::time::sleep(Duration::from_secs(30)).await;
        }
    }

    /// Relay the job result, and if the transaction fails record it in the DLQ.
    async fn relay_job_result(
        mut job: Job,
        job_relayer: Arc<JobRelayer>,
        writer_tx: SyncSender<WriterMsg>,
    ) -> Result<(), FailureReason> {
        let id = job.id;

        let relay_receipt_result = match job.request_type {
            RequestType::Onchain => job_relayer.relay_result_for_onchain_job(job.clone()).await,
            RequestType::Offchain(_) => {
                let job_params = (&job).try_into().map_err(|_| FailureReason::RelayErr)?;
                let job_request_payload = abi_encode_offchain_job_request(job_params);
                job_relayer.relay_result_for_offchain_job(job.clone(), job_request_payload).await
            }
        };

        let relay_tx_hash = match relay_receipt_result {
            Ok(receipt) => receipt.transaction_hash,
            Err(e) => {
                error!("failed to relay job {:?}: {:?}", id, e);
                writer_tx.send((Write::FailureJobs(job), None)).expect("db writer broken");

                return Err(FailureReason::RelayErr);
            }
        };

        // Save the relay tx hash and status to DB
        job.relay_tx_hash = relay_tx_hash.to_vec();
        job.status.status = JobStatusType::Relayed as i32;
        writer_tx.send((Write::JobTable(job), None)).expect("db writer broken");

        Ok(())
    }
}

/// [Builder](https://rust-unofficial.github.io/patterns/patterns/creational/builder.html) for `JobRelayer`.
#[derive(Debug)]
pub struct JobRelayerBuilder<S> {
    signer: Option<S>,
}

impl<S: TxSigner<Signature> + Send + Sync + 'static> Default for JobRelayerBuilder<S> {
    fn default() -> Self {
        Self::new()
    }
}

impl<S: TxSigner<Signature> + Send + Sync + 'static> JobRelayerBuilder<S> {
    /// Create a new [Self].
    pub const fn new() -> Self {
        Self { signer: None }
    }

    /// Specify the signer.
    pub fn signer(mut self, signer: S) -> Self {
        self.signer = Some(signer);
        self
    }

    /// Build a [`JobRelayer`].
    pub fn build(
        self,
        http_rpc_url: String,
        job_manager: Address,
        confirmations: u64,
        metrics: Arc<Metrics>,
    ) -> Result<JobRelayer, Error> {
        let url: reqwest::Url = http_rpc_url.parse().map_err(|_| Error::HttpRpcUrlParse)?;
        info!("🧾 relayer sending transactions to rpc url {url}");

        let signer = self.signer.ok_or(Error::MissingSigner)?;
        let wallet = EthereumWallet::new(signer);

        let provider =
            ProviderBuilder::new().with_recommended_fillers().wallet(wallet).on_http(url);
        let job_manager = JobManagerContract::new(job_manager, provider);

        Ok(JobRelayer { job_manager, confirmations, metrics })
    }
}

/// Submit completed jobs onchain to the `JobManager` contract.
///
/// This is safe to use across threads and should correctly handle nonce incrementing as long as
/// no transactions fail after being broadcasted.
/// TODO: <https://github.com/InfinityVM/InfinityVM/issues/131>
#[derive(Debug)]
pub struct JobRelayer {
    job_manager: JobManagerContract,
    confirmations: u64,
    metrics: Arc<Metrics>,
}

impl JobRelayer {
    /// Submit a completed job to the `JobManager` contract for an onchain job request.
    pub async fn relay_result_for_onchain_job(
        &self,
        job: Job,
    ) -> Result<TransactionReceipt, Error> {
        let call_builder = self
            .job_manager
            .submitResult(job.result_with_metadata.into(), job.zkvm_operator_signature.into());

        let pending_tx = call_builder.send().await.map_err(|error| {
            error!(?error, id = hex::encode(job.id), "tx broadcast failure");
            self.metrics.incr_relay_err(BROADCAST_ERROR);
            Error::TxBroadcast(error)
        })?;

        let receipt = pending_tx
            .with_required_confirmations(self.confirmations)
            .get_receipt()
            .await
            .map_err(|error| {
                error!(?error, id = hex::encode(job.id), "tx inclusion failed");
                self.metrics.incr_relay_err(TX_INCLUSION_ERROR);
                Error::TxInclusion(error)
            })?;

        info!(
            receipt.transaction_index,
            receipt.block_number,
            ?receipt.block_hash,
            ?receipt.transaction_hash,
            id=hex::encode(job.id),
            "tx included"
        );

        Ok(receipt)
    }

    /// Submit a completed job to the `JobManager` contract for an offchain job request.
    pub async fn relay_result_for_offchain_job(
        &self,
        job: Job,
        job_request_payload: Vec<u8>,
    ) -> Result<TransactionReceipt, Error> {
        let request_signature = if let RequestType::Offchain(request_signature) = job.request_type {
            request_signature
        } else {
            error!("internal error please report: cannot relay non-offchain job request");
            return Err(Error::InvalidJobRequestType);
        };

        // Only add the sidecar if there are some blobs. Some offchain jobs might
        // have no offchain input, and thus no sidecar
        let call_builder = match job.blobs_sidecar {
            Some(sidecar) if !sidecar.blobs.is_empty() => {
                let gas_price = self.job_manager.provider().get_gas_price().await?;
                self.job_manager
                    .submitResultForOffchainJob(
                        job.result_with_metadata.into(),
                        job.zkvm_operator_signature.into(),
                        job_request_payload.into(),
                        request_signature.into(),
                    )
                    .sidecar(sidecar)
                    .max_fee_per_blob_gas(gas_price)
            }
            _ => {
                debug_assert!(job.offchain_input.is_empty());
                self.job_manager.submitResultForOffchainJob(
                    job.result_with_metadata.into(),
                    job.zkvm_operator_signature.into(),
                    job_request_payload.into(),
                    request_signature.into(),
                )
            }
        };

        let pending_tx = call_builder.send().await.map_err(|error| {
            error!(
                ?error,
                job.nonce,
                "tx broadcast failure: contract_address = {}",
                hex::encode(&job.consumer_address)
            );
            self.metrics.incr_relay_err(BROADCAST_ERROR);
            Error::TxBroadcast(error)
        })?;

        let receipt =
            pending_tx.with_required_confirmations(0).get_receipt().await.map_err(|error| {
                error!(
                    ?error,
                    job.nonce,
                    "tx inclusion failed: contract_address = {}",
                    hex::encode(&job.consumer_address)
                );
                self.metrics.incr_relay_err(TX_INCLUSION_ERROR);
                Error::TxInclusion(error)
            })?;

        info!(
            receipt.transaction_index,
            receipt.block_number,
            ?receipt.block_hash,
            ?receipt.transaction_hash,
            id=hex::encode(job.id),
            "tx included"
        );

        Ok(receipt)
    }
}

#[cfg(test)]
mod test {
    use std::{collections::HashSet, sync::Arc};

    use crate::{metrics::Metrics, relayer::JobRelayerBuilder};
    use alloy::{
        network::EthereumWallet,
        providers::{Provider, ProviderBuilder},
        rpc::types::Filter,
        signers::local::PrivateKeySigner,
        sol_types::SolEvent,
    };
    use contracts::{i_job_manager::IJobManager, mock_consumer::MockConsumer};
    use ivm_abi::get_job_id;
    use mock_consumer::{
        anvil_with_mock_consumer, mock_consumer_pending_job, mock_contract_input_addr,
        AnvilMockConsumer,
    };
    use prometheus::Registry;

    use ivm_test_utils::{anvil_with_job_manager, get_localhost_port, AnvilJobManager};
    use tokio::task::JoinSet;

    const JOB_COUNT: usize = 30;

    #[tokio::test]
    async fn run_can_successfully_submit_results() {
        ivm_test_utils::test_tracing();

        let anvil_port = get_localhost_port();
        let anvil = anvil_with_job_manager(anvil_port).await;
        let AnvilMockConsumer { mock_consumer, mock_consumer_signer: _ } =
            anvil_with_mock_consumer(&anvil).await;

        let AnvilJobManager { anvil, job_manager, relayer, coprocessor_operator } = anvil;

        let user: PrivateKeySigner = anvil.keys()[5].clone().into();
        let user_wallet = EthereumWallet::from(user);

        let consumer_provider = ProviderBuilder::new()
            .with_recommended_fillers()
            .wallet(user_wallet)
            .on_http(anvil.endpoint().parse().unwrap());
        let consumer_contract = MockConsumer::new(mock_consumer, &consumer_provider);

        let registry = Registry::new();
        let metrics = Arc::new(Metrics::new(&registry));

        let job_relayer = JobRelayerBuilder::new()
            .signer(relayer)
            .build(anvil.endpoint().parse().unwrap(), job_manager, 1, metrics)
            .unwrap();
        let job_relayer = Arc::new(job_relayer);
        let mut join_set = JoinSet::new();

        for i in 0u8..JOB_COUNT as u8 {
            let job: ivm_db::tables::Job =
                mock_consumer_pending_job(i + 1, coprocessor_operator.clone(), mock_consumer).await;

            let mock_addr = mock_contract_input_addr();
            let program_id = job.program_id.clone();

            // Create the job on chain so the contract knows to expect the result. The consumer
            // contract will create a new job
            let create_job_call = consumer_contract.requestBalance(program_id.into(), mock_addr);
            let receipt = create_job_call.send().await.unwrap().get_receipt().await.unwrap();
            let log = receipt.inner.as_receipt().unwrap().logs[0]
                .log_decode::<IJobManager::JobCreated>()
                .unwrap();

            // Ensure test setup is working as we think
            assert_eq!(job.id, log.data().jobID);

            let relayer2 = Arc::clone(&job_relayer);
            join_set.spawn(async move {
                assert!(relayer2.relay_result_for_onchain_job(job).await.is_ok());
            });
        }

        // Wait for all the relay threads to finish so we know the transactions have landed
        while (join_set.join_next().await).is_some() {}

        // Check that each job is in the anvil node logs
        let filter = Filter::new().event(IJobManager::JobCompleted::SIGNATURE).from_block(0);
        let logs = consumer_provider.get_logs(&filter).await.unwrap();

        let seen: HashSet<[u8; 32]> = logs
            .into_iter()
            .map(|log| {
                let decoded = log.log_decode::<IJobManager::JobCompleted>().unwrap().data().clone();
                decoded.jobID.into()
            })
            .collect();
        // nonces from the consumer start at 1
        let expected: HashSet<[u8; 32]> =
            (1..=JOB_COUNT).map(|i| get_job_id(i.try_into().unwrap(), mock_consumer)).collect();

        // We expect to see exactly job ids 0 to 29 in the JobCompleted events
        assert_eq!(seen, expected);
    }
}

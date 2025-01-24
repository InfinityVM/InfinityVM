//! This module contains all components for broadcasting job results onchain.

use crate::{
    pool::FailureReason,
    metrics::Metrics,
    writer::{Write, WriterMsg},
};
use alloy::{
    hex,
    network::{Ethereum, EthereumWallet, TxSigner},
    primitives::{Address, PrimitiveSignature},
    providers::{Provider, ProviderBuilder},
    rpc::types::TransactionReceipt,
    transports::http::{reqwest, Client, Http},
};
use ivm_abi::abi_encode_offchain_job_request;
use ivm_contracts::i_job_manager::IJobManager;
use ivm_db::{
    get_all_failed_jobs,
    tables::{Job, RequestType},
};
use ivm_proto::{JobStatusType, RelayStrategy};
use reth_db::Database;
use std::{sync::Arc, time::Duration};
use tokio::sync::{
    mpsc::{self, error::TryRecvError, Receiver, Sender},
    oneshot,
};
use tracing::{error, info};

/// Delay between retrying failed jobs, in milliseconds.
const JOB_RETRY_DELAY_MS: u64 = 500;
/// Max duration between retries in `relay_job_result`.
const JOB_RETRY_MAX_DELAY_MS: u64 = 30 * 1_000;

type RecommendedFiller = alloy::providers::fillers::JoinFill<
    alloy::providers::Identity,
    alloy::providers::fillers::JoinFill<
        alloy::providers::fillers::GasFiller,
        alloy::providers::fillers::JoinFill<
            alloy::providers::fillers::BlobGasFiller,
            alloy::providers::fillers::JoinFill<
                alloy::providers::fillers::NonceFiller,
                alloy::providers::fillers::ChainIdFiller,
            >,
        >,
    >,
>;

type HttpTransport = Http<Client>;

type RelayerProvider = alloy::providers::fillers::FillProvider<
    alloy::providers::fillers::JoinFill<
        RecommendedFiller,
        alloy::providers::fillers::WalletFiller<EthereumWallet>,
    >,
    alloy::providers::RootProvider<HttpTransport>,
    HttpTransport,
    Ethereum,
>;

type JobManagerContract = IJobManager::IJobManagerInstance<HttpTransport, RelayerProvider>;

const TX_INCLUSION_ERROR: &str = "relay_error_tx_inclusion_error";
const BROADCAST_ERROR: &str = "relay_error_broadcast_failure";

/// Relay config.
#[derive(Debug)]
pub struct RelayConfig {
    /// Number of required confirmations to wait before considering a result tx included on chain.
    pub confirmations: u64,
    /// Maximum number of retries for a job in the dead letter queue.
    pub dlq_max_retries: u32,
    /// Maximum number of retries when initially attempting to relay a job.
    pub initial_relay_max_retries: u32,
}

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
    TxInclusion(#[from] alloy::providers::PendingTransactionError),
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

/// Service to retry relaying jobs on the dead letter queue.
#[derive(Debug)]
pub struct RelayRetry<D> {
    db: Arc<D>,
    job_relayer: Arc<JobRelayer>,
    metrics: Arc<Metrics>,
    dlq_max_retries: u32,
    writer_tx: Sender<WriterMsg>,
}

impl<D> RelayRetry<D>
where
    D: Database + 'static,
{
    /// Create a new instance of [Self].
    pub const fn new(
        db: Arc<D>,
        job_relayer: Arc<JobRelayer>,
        metrics: Arc<Metrics>,
        dlq_max_retries: u32,
        writer_tx: Sender<WriterMsg>,
    ) -> Self {
        Self { db, job_relayer, metrics, dlq_max_retries, writer_tx }
    }

    /// Retry jobs that failed to relay
    pub async fn start(self) -> Result<(), Error> {
        let Self { db, job_relayer, metrics, dlq_max_retries, writer_tx } = self;
        loop {
            // Jobs that we no longer want to retry
            let mut jobs_to_delete = Vec::new();

            let retry_jobs = match get_all_failed_jobs(db.clone()).await {
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
                            .await
                            .expect("db writer broken");
                        let _ = rx.await;
                    }
                    Err(e) => {
                        if job.status.retries == dlq_max_retries {
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
                                .await
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
                    .await
                    .expect("db writer broken");
                let _ = rx.await;
            }
            tokio::time::sleep(Duration::from_secs(30)).await;
        }
    }
}

/// A message to a relay actor.
#[allow(clippy::large_enum_variant)]
#[derive(Debug)]
pub enum RelayMsg {
    /// Tell the actor to relay the given `Job`.
    Relay(Job),

    /// Tell the actor to exit.
    Exit,
}

/// This is a type used to spawn new relay actors.
#[derive(Debug, Clone)]
pub struct RelayActorSpawner {
    writer_tx: Sender<WriterMsg>,
    job_relayer: Arc<JobRelayer>,
    initial_relay_max_retries: u32,
    channel_bound: usize,
}

impl RelayActorSpawner {
    /// Create a new instance of [Self].
    pub const fn new(
        writer_tx: Sender<WriterMsg>,
        job_relayer: Arc<JobRelayer>,
        initial_relay_max_retries: u32,
        channel_bound: usize,
    ) -> Self {
        Self { writer_tx, job_relayer, initial_relay_max_retries, channel_bound }
    }

    /// Spawn a new relay actor.
    ///
    /// It is expected that the caller will spawn exactly one relay actor per execution actor.
    pub fn spawn(&self) -> Sender<RelayMsg> {
        let (relay_tx, relay_rx) = mpsc::channel(self.channel_bound);
        let actor = RelayActor::new(
            self.writer_tx.clone(),
            relay_rx,
            self.job_relayer.clone(),
            self.initial_relay_max_retries,
        );

        tokio::spawn(async move { actor.start().await });

        relay_tx
    }
}

/// The service in charge of handling all routines related to relay transactions onchain.
#[derive(Debug)]
struct RelayActor {
    writer_tx: Sender<WriterMsg>,
    relay_rx: Receiver<RelayMsg>,
    job_relayer: Arc<JobRelayer>,
    initial_relay_max_retries: u32,
}

impl RelayActor {
    /// Create a new instance of [Self].
    const fn new(
        writer_tx: Sender<WriterMsg>,
        relay_rx: Receiver<RelayMsg>,
        job_relayer: Arc<JobRelayer>,
        initial_relay_max_retries: u32,
    ) -> Self {
        Self { writer_tx, relay_rx, job_relayer, initial_relay_max_retries }
    }

    /// Start the relay actor
    async fn start(self) {
        let mut relay_rx = self.relay_rx;

        loop {
            // TODO: for some reason recv_async was not working and never receiving from
            // the channel. This is a hack, but I assume there is some other issue I am
            // missing
            // https://github.com/InfinityVM/InfinityVM/issues/437
            let msg = match relay_rx.try_recv() {
                Err(TryRecvError::Disconnected) => {
                    error!("exiting relay actor");
                    return;
                }
                Err(_error) => {
                    tokio::time::sleep(tokio::time::Duration::from_millis(25)).await;
                    continue;
                }
                Ok(relay_msg) => relay_msg,
            };

            let job = match msg {
                RelayMsg::Relay(job) => job,
                RelayMsg::Exit => return,
            };

            info!(
                id = hex::encode(job.id),
                consumer = hex::encode(&job.consumer_address),
                job.nonce,
                "received job to relay"
            );

            match job.relay_strategy {
                RelayStrategy::Ordered => {
                    // Wait until this job has made its way into a block before relaying the next
                    // job.
                    let _ = Self::relay_job_result(
                        job,
                        self.job_relayer.clone(),
                        self.writer_tx.clone(),
                        self.initial_relay_max_retries,
                    )
                    .await;
                }
                RelayStrategy::Unordered => {
                    // Don't wait for the job to make its way into a block before relaying the next
                    // job.
                    let future = Self::relay_job_result(
                        job,
                        self.job_relayer.clone(),
                        self.writer_tx.clone(),
                        self.initial_relay_max_retries,
                    );
                    tokio::spawn(future);
                }
            }
        }
    }

    /// Relay the job result, and if the transaction fails record it in the DLQ.
    /// We retry the transaction `initial_relay_max_retries` times.
    async fn relay_job_result(
        mut job: Job,
        job_relayer: Arc<JobRelayer>,
        writer_tx: Sender<WriterMsg>,
        initial_relay_max_retries: u32,
    ) -> Result<(), FailureReason> {
        let id = job.id;

        let mut i = 1;
        let relay_receipt = loop {
            let relay_receipt_result = match job.request_type {
                RequestType::Onchain => job_relayer.relay_result_for_onchain_job(job.clone()).await,
                RequestType::Offchain(_) => {
                    let job_params = (&job).try_into().map_err(|_| FailureReason::RelayErr)?;
                    let job_request_payload = abi_encode_offchain_job_request(job_params);
                    job_relayer
                        .relay_result_for_offchain_job(job.clone(), job_request_payload)
                        .await
                }
            };

            if i > initial_relay_max_retries + 1 {
                break relay_receipt_result;
            } else if relay_receipt_result.is_err() {
                let calc_backoff = JOB_RETRY_DELAY_MS * i as u64;
                let backoff = if calc_backoff > JOB_RETRY_MAX_DELAY_MS {
                    JOB_RETRY_MAX_DELAY_MS
                } else {
                    calc_backoff
                };
                tokio::time::sleep(Duration::from_millis(backoff)).await;
            } else {
                break relay_receipt_result;
            }
            i += 1;
        };

        let relay_tx_hash = match relay_receipt {
            Ok(receipt) => receipt.transaction_hash,
            Err(e) => {
                error!("failed to relay job {:?}: {:?}", id, e);
                writer_tx.send((Write::FailureJobs(job), None)).await.expect("db writer broken");

                return Err(FailureReason::RelayErr);
            }
        };

        // Save the relay tx hash and status to DB
        job.relay_tx_hash = relay_tx_hash.to_vec();
        job.status.status = JobStatusType::Relayed as i32;
        writer_tx.send((Write::JobTable(job), None)).await.expect("db writer broken");

        Ok(())
    }
}

/// [Builder](https://rust-unofficial.github.io/patterns/patterns/creational/builder.html) for `JobRelayer`.
#[derive(Debug)]
pub struct JobRelayerBuilder<S> {
    signer: Option<S>,
}

impl<S: TxSigner<PrimitiveSignature> + Send + Sync + 'static> Default for JobRelayerBuilder<S> {
    fn default() -> Self {
        Self::new()
    }
}

impl<S: TxSigner<PrimitiveSignature> + Send + Sync + 'static> JobRelayerBuilder<S> {
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
            consumer=hex::encode(job.consumer_address),
            job.nonce,
            "tx included"
        );
        self.metrics.incr_relayed_total();

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

        let receipt = pending_tx
            .with_required_confirmations(self.confirmations)
            .get_receipt()
            .await
            .map_err(|error| {
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
            consumer=hex::encode(job.consumer_address),
            job.nonce,
            "tx included"
        );
        self.metrics.incr_relayed_total();

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
    use ivm_abi::get_job_id;
    use ivm_contracts::{i_job_manager::IJobManager, mock_consumer::MockConsumer};
    use ivm_mock_consumer::{
        anvil_with_mock_consumer, mock_consumer_pending_job, mock_contract_input_addr,
        AnvilMockConsumer,
    };
    use prometheus::Registry;

    use ivm_test_utils::{anvil_with_job_manager, get_localhost_port, AnvilJobManager};

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

        for i in 0u8..JOB_COUNT as u8 {
            let job =
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

            // Relay the job result sequentially to avoid flakiness due to nonce issues
            // caused by race conditions
            assert!(job_relayer.relay_result_for_onchain_job(job).await.is_ok());
        }

        // Give a little extra time to avoid flakiness
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

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

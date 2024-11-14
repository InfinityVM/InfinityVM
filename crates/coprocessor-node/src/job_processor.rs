//! Job processor implementation.

use crate::{metrics::Metrics, relayer::JobRelayer};
use alloy::{hex, primitives::Signature, signers::Signer};
use async_channel::Receiver;
use ivm_abi::abi_encode_offchain_job_request;
use ivm_db::{
    delete_fail_relay_job, get_all_failed_jobs, put_fail_relay_job, put_job,
    tables::{ElfWithMeta, Job, RequestType},
};
use ivm_proto::{JobStatus, JobStatusType, VmType};
use reth_db::Database;
use std::{marker::Send, sync::Arc, time::Duration};
use tokio::task::JoinSet;
use tracing::{error, info};
use zkvm_executor::service::ZkvmExecutorService;

/// Delay between retrying failed jobs, in milliseconds.m
const JOB_RETRY_DELAY_MS: u64 = 250;

/// Errors from job processor
#[derive(thiserror::Error, Debug)]
pub enum Error {
    /// database error
    #[error("database error: {0}")]
    Database(#[from] ivm_db::Error),
    /// ELF with given verifying key already exists in DB
    #[error("elf with verifying key {0} already exists")]
    ElfAlreadyExists(String),
    /// Job already exists in DB
    #[error("job already exists")]
    JobAlreadyExists,
    /// Failed to send job to exec queue
    #[error("failed to send to exec queue")]
    ExecQueueSendFailed,
    /// Could not read ELF from DB
    #[error("failed reading elf: {0}")]
    ElfReadFailed(String),
    /// Could not write ELF to DB
    #[error("failed writing elf: {0}")]
    ElfWriteFailed(String),
    /// Invalid VM type
    #[error("invalid VM type")]
    InvalidVmType,
    /// exec queue channel unexpected closed
    #[error("exec queue channel unexpected closed")]
    ExecQueueChannelClosed,
}

/// `JobStatus` Failure Reasons
#[derive(thiserror::Error, Debug)]
pub enum FailureReason {
    /// Job submitted with unknown or missing ELF
    #[error("missing_elf")]
    MissingElf,
    /// No ELF found in DB
    #[error("db_error_missing_elf")]
    DbErrMissingElf,
    /// Error retrieving elf because of low-level DB error
    #[error("db_error_get_elf")]
    DbErrGetElf,
    /// Unable to persist failed job to DB
    #[error("db_error_status_failed")]
    DbErrStatusFailed,
    /// Error executing job against zkvm
    #[error("execution_error")]
    ExecErr,
    /// Unable to persist successfully completed job to DB
    #[error("db_error_status_done")]
    DbErrStatusDone,
    /// Unable to persist relayed job to DB
    #[error("db_error_status_relayed")]
    DbErrStatusRelayed,
    /// Failure submitting job to `JobManager` contract
    #[error("relay_error")]
    RelayErr,
    /// Unable to persist failed relay job
    #[error("db_relay_error")]
    DbRelayErr,
    /// Relay retry exceeded for job
    #[error("relay_error_exceed_retry")]
    RelayErrExceedRetry,
}

/// Job processor config.
#[derive(Debug)]
pub struct JobProcessorConfig {
    /// Number of worker threads to run.
    pub num_workers: usize,
    /// Maximum number of retries for a job.
    pub max_retries: u32,
}

/// Job processor service.
///
/// This stores a `JoinSet` with a handle to each job processor worker and the job retry task.
#[derive(Debug)]
pub struct JobProcessorService<S, D> {
    db: Arc<D>,
    exec_queue_receiver: Receiver<Job>,
    job_relayer: Arc<JobRelayer>,
    zk_executor: ZkvmExecutorService<S>,
    task_handles: JoinSet<Result<(), Error>>,
    metrics: Arc<Metrics>,
    config: JobProcessorConfig,
}

// The DB functions in JobProcessorService are async so they yield in the tokio task.
impl<S, D> JobProcessorService<S, D>
where
    S: Signer<Signature> + Send + Sync + Clone + 'static,
    D: Database + 'static,
{
    /// Create a new job processor service.
    pub fn new(
        db: Arc<D>,
        exec_queue_receiver: Receiver<Job>,
        job_relayer: Arc<JobRelayer>,
        zk_executor: ZkvmExecutorService<S>,
        metrics: Arc<Metrics>,
        config: JobProcessorConfig,
    ) -> Self {
        Self {
            db,
            exec_queue_receiver,
            job_relayer,
            zk_executor,
            task_handles: JoinSet::new(),
            metrics,
            config,
        }
    }

    /// Starts both the relay retry cron job and the job processor, and spawns `num_workers` worker
    /// threads
    pub async fn start(&mut self) {
        for _ in 0..self.config.num_workers {
            let exec_queue_receiver = self.exec_queue_receiver.clone();
            let db = Arc::clone(&self.db);
            let zk_executor = self.zk_executor.clone();
            let job_relayer = Arc::clone(&self.job_relayer);
            let metrics = Arc::clone(&self.metrics);

            self.task_handles.spawn({
                async move {
                    Self::start_processor_worker(
                        exec_queue_receiver,
                        db,
                        job_relayer,
                        zk_executor,
                        metrics,
                    )
                    .await
                }
            });
        }

        let db = Arc::clone(&self.db);
        let job_relayer = Arc::clone(&self.job_relayer);
        let metrics = Arc::clone(&self.metrics);
        let max_retries = self.config.max_retries;

        self.task_handles.spawn(async move {
            Self::start_job_retry_task(db, job_relayer, metrics, max_retries).await
        });
    }

    /// Start a single queue puller worker task.
    async fn start_processor_worker(
        exec_queue_receiver: Receiver<Job>,
        db: Arc<D>,
        job_relayer: Arc<JobRelayer>,
        zk_executor: ZkvmExecutorService<S>,
        metrics: Arc<Metrics>,
    ) -> Result<(), Error> {
        loop {
            let mut job =
                exec_queue_receiver.recv().await.map_err(|_| Error::ExecQueueChannelClosed)?;
            let id = job.id;
            info!("executing job {:?}", hex::encode(id));

            let elf_with_meta = match Self::get_elf(&db, &mut job, &metrics).await {
                Ok(elf) => elf,
                Err(_) => continue,
            };

            job = match Self::execute_job(job, &zk_executor, elf_with_meta, &db, &metrics).await {
                Ok(updated_job) => updated_job,
                Err(_) => continue,
            };

            let _ = Self::relay_job_result(job, &job_relayer, &db, &metrics).await;
        }
    }

    async fn get_elf(
        db: &Arc<D>,
        job: &mut Job,
        metrics: &Arc<Metrics>,
    ) -> Result<ElfWithMeta, FailureReason> {
        match ivm_db::get_elf(db.clone(), &job.program_id) {
            Ok(Some(elf)) => Ok(elf),
            Ok(None) => {
                metrics.incr_job_err(&FailureReason::MissingElf.to_string());
                job.status = JobStatus {
                    status: JobStatusType::Failed as i32,
                    failure_reason: Some(FailureReason::MissingElf.to_string()),
                    retries: 0,
                };

                if let Err(e) = put_job(db.clone(), job.clone()) {
                    error!("report this error: failed to save job {:?}: {:?}", job.id, e);
                    metrics.incr_job_err(&FailureReason::DbErrStatusFailed.to_string());
                    return Err(FailureReason::DbErrStatusFailed);
                }
                Err(FailureReason::DbErrMissingElf)
            }
            Err(error) => {
                error!(
                    error = ?error,
                    "DB read error for job {:?} with program ID {:?}",
                    job.id,
                    job.program_id
                );
                metrics.incr_job_err(&FailureReason::DbErrGetElf.to_string());
                job.status = JobStatus {
                    status: JobStatusType::Failed as i32,
                    failure_reason: Some(FailureReason::DbErrGetElf.to_string()),
                    retries: 0,
                };

                if let Err(e) = put_job(db.clone(), job.clone()) {
                    error!("report this error: failed to save job {:?}: {:?}", job.id, e);
                    metrics.incr_job_err(&FailureReason::DbErrStatusFailed.to_string());
                    return Err(FailureReason::DbErrStatusFailed);
                }
                Err(FailureReason::DbErrGetElf)
            }
        }
    }

    async fn execute_job(
        mut job: Job,
        zk_executor: &ZkvmExecutorService<S>,
        elf_with_meta: ElfWithMeta,
        db: &Arc<D>,
        metrics: &Arc<Metrics>,
    ) -> Result<Job, FailureReason> {
        let id = job.id;
        let result = match job.request_type {
            RequestType::Onchain => zk_executor
                .execute_onchain_job(
                    id,
                    job.max_cycles,
                    job.program_id.clone(),
                    job.onchain_input.clone(),
                    elf_with_meta.elf,
                    VmType::Sp1,
                )
                .await
                .map(|(result_with_metadata, signature)| (result_with_metadata, signature, None)),
            RequestType::Offchain(_) => {
                zk_executor
                    .execute_offchain_job(
                        id,
                        job.max_cycles,
                        job.program_id.clone(),
                        job.onchain_input.clone(),
                        job.offchain_input.clone(),
                        elf_with_meta.elf,
                        VmType::Sp1,
                    )
                    .await
            }
        };

        match result {
            Ok((result_with_metadata, zkvm_operator_signature, sidecar)) => {
                tracing::debug!("job {:?} executed successfully", id);
                job.status = JobStatus {
                    status: JobStatusType::Done as i32,
                    failure_reason: None,
                    retries: 0,
                };
                job.result_with_metadata = result_with_metadata;
                job.zkvm_operator_signature = zkvm_operator_signature;
                job.blobs_sidecar = sidecar;
                if let Err(e) = put_job(db.clone(), job.clone()) {
                    error!("report this error: failed to save job {:?}: {:?}", id, e);
                    metrics.incr_job_err(&FailureReason::DbErrStatusDone.to_string());
                    return Err(FailureReason::DbErrStatusDone);
                }
                Ok(job)
            }
            Err(e) => {
                // TODO: We need to relay failed results to make sure we can charge people
                // [ref: https://github.com/InfinityVM/InfinityVM/issues/78]
                error!("failed to execute job {:?}: {:?}", id, e);
                metrics.incr_job_err(&FailureReason::ExecErr.to_string());

                job.status = JobStatus {
                    status: JobStatusType::Failed as i32,
                    failure_reason: Some(FailureReason::ExecErr.to_string()),
                    retries: 0,
                };

                if let Err(e) = put_job(db.clone(), job) {
                    error!("report this error: failed to save job {:?}: {:?}", id, e);
                    metrics.incr_job_err(&FailureReason::DbErrStatusFailed.to_string());
                    return Err(FailureReason::DbErrStatusFailed);
                }
                Err(FailureReason::ExecErr)
            }
        }
    }

    async fn relay_job_result(
        mut job: Job,
        job_relayer: &Arc<JobRelayer>,
        db: &Arc<D>,
        metrics: &Arc<Metrics>,
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
                error!("report this error: failed to relay job {:?}: {:?}", id, e);
                if let Err(e) = put_fail_relay_job(db.clone(), job) {
                    error!("report this error: failed to save relay err {:?}: {:?}", id, e);
                    metrics.incr_job_err(&FailureReason::DbRelayErr.to_string());
                    return Err(FailureReason::DbRelayErr);
                }
                return Err(FailureReason::RelayErr);
            }
        };

        // Save the relay tx hash and status to DB
        job.relay_tx_hash = relay_tx_hash.to_vec();
        job.status =
            JobStatus { status: JobStatusType::Relayed as i32, failure_reason: None, retries: 0 };
        if let Err(e) = put_job(db.clone(), job) {
            error!("report this error: failed to save relayed job {:?}: {:?}", id, e);
            metrics.incr_job_err(&FailureReason::DbErrStatusRelayed.to_string());
            return Err(FailureReason::DbErrStatusRelayed);
        }

        Ok(())
    }

    /// Retry jobs that failed to relay
    async fn start_job_retry_task(
        db: Arc<D>,
        job_relayer: Arc<JobRelayer>,
        metrics: Arc<Metrics>,
        max_retries: u32,
    ) -> Result<(), Error> {
        loop {
            // Jobs that we no longer want to retry
            let mut jobs_to_delete: Vec<[u8; 32]> = Vec::new();

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
                        job.status = JobStatus {
                            status: JobStatusType::Relayed as i32,
                            failure_reason: None,
                            retries: 0,
                        };
                        if let Err(e) = put_job(db.clone(), job) {
                            error!(
                                "report this error: failed to save relayed job {:?}: {:?}",
                                hex::encode(id),
                                e
                            );
                            metrics.incr_job_err(&FailureReason::DbErrStatusRelayed.to_string());
                        }
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
                                "report this error: failed to retry relaying job {:?}: {:?}",
                                hex::encode(id),
                                e
                            );
                            job.status.retries += 1;
                            if let Err(e) = put_fail_relay_job(db.clone(), job) {
                                error!(
                                    "report this error: failed to save retried job {:?}: {:?}",
                                    hex::encode(id),
                                    e
                                );
                                metrics.incr_job_err(&FailureReason::DbErrStatusFailed.to_string());
                            }
                        }
                    }
                }

                // Before retrying another job, wait to reduce general load on the system
                tokio::time::sleep(Duration::from_millis(JOB_RETRY_DELAY_MS)).await;
            }

            for job_id in &jobs_to_delete {
                if let Err(e) = delete_fail_relay_job(db.clone(), *job_id) {
                    error!("report this error: failed to delete retried job {:?}: {:?}", job_id, e);
                    metrics.incr_job_err(&FailureReason::DbErrStatusFailed.to_string());
                }
                // There could potentially be a lot of jobs to delete, so in order to not block for
                // too long we make sure to yield to the tokio runtime.
                tokio::task::yield_now().await;
            }
            tokio::time::sleep(Duration::from_secs(30)).await;
        }
    }
}

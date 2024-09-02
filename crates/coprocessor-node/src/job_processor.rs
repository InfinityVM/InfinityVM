//! Job processor implementation.

use crate::{metrics::Metrics, relayer::JobRelayer};
use abi::abi_encode_offchain_job_request;
use alloy::{hex, primitives::Signature, signers::Signer};
use async_channel::{Receiver, Sender};
use db::{
    delete_fail_relay_job, get_all_failed_jobs, get_elf, get_fail_relay_job, get_job, put_elf,
    put_fail_relay_job, put_job,
    tables::{Job, RequestType},
};
use proto::{JobStatus, JobStatusType, VmType};
use reth_db::Database;
use std::{marker::Send, sync::Arc, time::Duration};
use tokio::task::JoinSet;
use tracing::{error, info};
use zkvm_executor::service::ZkvmExecutorService;

/// Errors from job processor
#[derive(thiserror::Error, Debug)]
pub enum Error {
    /// Could not create ELF in zkvm executor
    #[error("failed to create ELF in zkvm executor: {0}")]
    CreateElfFailed(#[from] zkvm_executor::service::Error),
    /// database error
    #[error("database error: {0}")]
    Database(#[from] db::Error),
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
    /// invalid address length
    #[error("invalid address length")]
    InvalidAddressLength(#[from] abi::Error),
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
    /// Error retrieving elf
    #[error("error_get_elf")]
    ErrGetElf,
    /// Unable to persist failed job to DB
    #[error("db_error_status_failed")]
    DbErrStatusFailed,
    /// Error executing job against zkvm
    #[error("execution_error")]
    ExecErr,
    /// Unable to persist successfully completed job to DB
    #[error("db_error_status_done")]
    DbErrStatusDone,
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

/// Job processor service.
#[derive(Debug)]
pub struct JobProcessorService<S, D> {
    db: Arc<D>,
    exec_queue_sender: Sender<Job>,
    exec_queue_receiver: Receiver<Job>,
    job_relayer: Arc<JobRelayer>,
    zk_executor: ZkvmExecutorService<S>,
    task_handles: JoinSet<Result<(), Error>>,
    metrics: Arc<Metrics>,
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
        exec_queue_sender: Sender<Job>,
        exec_queue_receiver: Receiver<Job>,
        job_relayer: Arc<JobRelayer>,
        zk_executor: ZkvmExecutorService<S>,
        metrics: Arc<Metrics>,
    ) -> Self {
        Self {
            db,
            exec_queue_sender,
            exec_queue_receiver,
            job_relayer,
            zk_executor,
            task_handles: JoinSet::new(),
            metrics,
        }
    }

    /// Submit program ELF, save it in DB, and return verifying key.
    pub async fn submit_elf(&self, elf: Vec<u8>, vm_type: i32) -> Result<Vec<u8>, Error> {
        let program_id = self
            .zk_executor
            .create_elf(&elf, VmType::try_from(vm_type).map_err(|_| Error::InvalidVmType)?)
            .await?;

        if get_elf(self.db.clone(), &program_id)
            .map_err(|e| Error::ElfReadFailed(e.to_string()))?
            .is_some()
        {
            return Err(Error::ElfAlreadyExists(format!(
                "elf with verifying key {:?} already exists",
                hex::encode(program_id.as_slice()),
            )));
        }

        put_elf(
            self.db.clone(),
            VmType::try_from(vm_type).map_err(|_| Error::InvalidVmType)?,
            &program_id,
            elf,
        )
        .map_err(|e| Error::ElfWriteFailed(e.to_string()))?;

        Ok(program_id)
    }

    /// Save job in DB
    pub async fn save_job(db: Arc<D>, job: Job) -> Result<(), Error> {
        put_job(db, job)?;
        Ok(())
    }

    /// Return whether job with `job_id` exists in DB
    pub async fn has_job(&self, job_id: [u8; 32]) -> Result<bool, Error> {
        let job = get_job(self.db.clone(), job_id)?;
        Ok(job.is_some())
    }

    /// Returns job with `job_id` from DB
    pub async fn get_job(&self, job_id: [u8; 32]) -> Result<Option<Job>, Error> {
        let job = get_job(self.db.clone(), job_id)?;
        Ok(job)
    }

    /// Returns all jobs with relay failures
    pub async fn get_all_relay_error_jobs(db: Arc<D>) -> Result<Vec<Job>, Error> {
        let jobs = get_all_failed_jobs(db)?;
        Ok(jobs)
    }
    /// Save failed relayed job in DB
    pub async fn save_relay_error_job(db: Arc<D>, job: Job) -> Result<(), Error> {
        put_fail_relay_job(db, job)?;
        Ok(())
    }

    /// Returns failed relayed job with `job_id` from DB
    pub async fn get_relay_error_job(&self, job_id: [u8; 32]) -> Result<Option<Job>, Error> {
        let job = get_fail_relay_job(self.db.clone(), job_id)?;
        Ok(job)
    }

    /// Deletes relay error job
    pub async fn delete_relay_err_job(db: Arc<D>, job_id: [u8; 32]) -> Result<(), Error> {
        delete_fail_relay_job(db, job_id)?;
        Ok(())
    }

    /// Submits job, saves it in DB, and adds to exec queue
    pub async fn submit_job(&self, mut job: Job) -> Result<(), Error> {
        if self.has_job(job.id).await? {
            return Err(Error::JobAlreadyExists);
        }

        job.status =
            JobStatus { status: JobStatusType::Pending as i32, failure_reason: None, retries: 0 };

        Self::save_job(self.db.clone(), job.clone()).await?;

        self.exec_queue_sender.send(job).await.map_err(|_| Error::ExecQueueSendFailed)?;

        Ok(())
    }

    /// Starts both the relay retry cron job and the job processor, and spawns `num_workers` worker
    /// threads
    pub async fn start(&mut self, num_workers: usize) {
        for _ in 0..num_workers {
            let exec_queue_receiver = self.exec_queue_receiver.clone();
            let db = Arc::clone(&self.db);
            let zk_executor = self.zk_executor.clone();
            let job_relayer = Arc::clone(&self.job_relayer);
            let metrics = Arc::clone(&self.metrics);

            self.task_handles.spawn({
                async move {
                    Self::start_worker(exec_queue_receiver, db, job_relayer, zk_executor, metrics)
                        .await
                }
            });
        }

        let db = Arc::clone(&self.db);
        let job_relayer = Arc::clone(&self.job_relayer);
        let metrics = Arc::clone(&self.metrics);

        self.task_handles.spawn(async move { Self::retry_jobs(db, job_relayer, metrics, 3).await });
    }

    /// Start a single worker thread.
    async fn start_worker(
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
            info!("executing job {:?}", id);

            let elf_with_meta = match db::get_elf(db.clone(), &job.program_id) {
                Ok(Some(elf)) => elf,
                Ok(None) => {
                    error!(
                        ?job.consumer_address,
                        "no ELF found for job {:?} with verifying key {:?}",
                        id,
                        job.program_id,
                    );
                    metrics.incr_job_err(&FailureReason::MissingElf.to_string());

                    job.status = JobStatus {
                        status: JobStatusType::Failed as i32,
                        failure_reason: Some(FailureReason::MissingElf.to_string()),
                        retries: 0,
                    };

                    if let Err(e) = Self::save_job(db.clone(), job).await {
                        error!("report this error: failed to save job {:?}: {:?}", id, e);
                        metrics.incr_job_err(&FailureReason::DbErrMissingElf.to_string());
                    }
                    continue;
                }
                Err(error) => {
                    error!(
                        ?error,
                        "could not find elf for job {:?} with verifying key {:?}",
                        id,
                        job.program_id
                    );

                    metrics.incr_job_err(&FailureReason::ErrGetElf.to_string());

                    job.status = JobStatus {
                        status: JobStatusType::Failed as i32,
                        failure_reason: Some(FailureReason::ErrGetElf.to_string()),
                        retries: 0,
                    };

                    if let Err(e) = Self::save_job(db.clone(), job).await {
                        error!("report this error: failed to save job {:?}: {:?}", id, e);
                        metrics.incr_job_err("db_error_status_failed");
                    }
                    continue;
                }
            };

            let job = match zk_executor
                .execute(
                    id,
                    job.max_cycles,
                    job.program_id.clone(),
                    job.input.clone(),
                    job.program_state.clone(),
                    elf_with_meta.elf,
                    VmType::Risc0,
                )
                .await
            {
                Ok((result_with_metadata, zkvm_operator_signature)) => {
                    tracing::debug!("job {:?} executed successfully", id.clone());

                    job.status = JobStatus {
                        status: JobStatusType::Done as i32,
                        failure_reason: None,
                        retries: 0,
                    };

                    job.result_with_metadata = result_with_metadata;
                    job.zkvm_operator_signature = zkvm_operator_signature;

                    if let Err(e) = Self::save_job(db.clone(), job.clone()).await {
                        error!("report this error: failed to save job {:?}: {:?}", id, e);
                        metrics.incr_job_err(&FailureReason::DbErrStatusFailed.to_string());
                        continue;
                    }
                    job
                }
                Err(e) => {
                    error!("failed to execute job {:?}: {:?}", id, e);
                    metrics.incr_job_err(&FailureReason::ExecErr.to_string());

                    job.status = JobStatus {
                        status: JobStatusType::Failed as i32,
                        failure_reason: Some(FailureReason::ExecErr.to_string()),
                        retries: 0,
                    };

                    if let Err(e) = Self::save_job(db.clone(), job.clone()).await {
                        error!("report this error: failed to save job {:?}: {:?}", id, e);
                        metrics.incr_job_err(&FailureReason::DbErrStatusDone.to_string());
                    }
                    continue;
                }
            };

            let relay_receipt_result = match job.request_type {
                RequestType::Onchain => job_relayer.relay_result_for_onchain_job(job.clone()).await,
                RequestType::Offchain(_) => {
                    let job_request_payload =
                        abi_encode_offchain_job_request(job.clone().try_into()?);
                    job_relayer
                        .relay_result_for_offchain_job(job.clone(), job_request_payload)
                        .await
                }
            };

            let _relay_receipt = match relay_receipt_result {
                Ok(receipt) => receipt,
                Err(e) => {
                    error!("report this error: failed to relay job {:?}: {:?}", id, e);
                    metrics.incr_relay_err(&FailureReason::RelayErr.to_string());
                    if let Err(e) = Self::save_relay_error_job(db.clone(), job).await {
                        error!("report this error: failed to save relay err {:?}: {:?}", id, e);
                        metrics.incr_job_err(&FailureReason::DbRelayErr.to_string());
                    }

                    continue;
                }
            };

            // TODO: Save tx hash of job receipt to DB
            // [ref: https://github.com/Ethos-Works/InfinityVM/issues/46]
        }
    }
    /// Retry jobs that failed to relay
    async fn retry_jobs(
        db: Arc<D>,
        job_relayer: Arc<JobRelayer>,
        metrics: Arc<Metrics>,
        max_retries: u32,
    ) -> Result<(), Error> {
        loop {
            // Jobs to update
            let mut jobs_to_delete: Vec<[u8; 32]> = Vec::new();

            let retry_jobs = match Self::get_all_relay_error_jobs(db.clone()).await {
                Ok(jobs) => jobs,
                Err(e) => {
                    error!("error retrieving relay error jobs: {:?}", e);
                    continue;
                }
            };

            // Retry each once
            for mut job in retry_jobs {
                let result = match job.request_type {
                    RequestType::Onchain => {
                        job_relayer.relay_result_for_onchain_job(job.clone()).await
                    }
                    RequestType::Offchain(_) => {
                        let job_request_payload =
                            abi_encode_offchain_job_request(job.clone().try_into()?);
                        job_relayer
                            .relay_result_for_offchain_job(job.clone(), job_request_payload)
                            .await
                    }
                };

                match result {
                    Ok(_) => {
                        info!("successfully retried job relay for job: {:?}", job.id);
                        jobs_to_delete.push(job.id);
                    }
                    Err(e) => {
                        if job.status.retries == max_retries {
                            metrics.incr_relay_err(&FailureReason::RelayErrExceedRetry.to_string());
                            jobs_to_delete.push(job.id);
                        } else {
                            error!(
                                "report this error: failed to retry relaying job {:?}: {:?}",
                                job.id, e
                            );
                            metrics.incr_relay_err(&FailureReason::RelayErr.to_string());
                            job.status.retries += 1;
                            if let Err(e) =
                                Self::save_relay_error_job(db.clone(), job.clone()).await
                            {
                                error!(
                                    "report this error: failed to save retried job {:?}: {:?}",
                                    job.id, e
                                );
                                metrics.incr_job_err(&FailureReason::DbErrStatusFailed.to_string());
                            }
                        }
                        continue;
                    }
                }

                for job_id in &jobs_to_delete {
                    if let Err(e) = Self::delete_relay_err_job(db.clone(), *job_id).await {
                        error!(
                            "report this error: failed to delete retried job {:?}: {:?}",
                            job_id, e
                        );
                        metrics.incr_job_err(&FailureReason::DbErrStatusFailed.to_string());
                    }
                }
            }
            tokio::time::sleep(Duration::from_secs(30)).await;
        }
    }
}

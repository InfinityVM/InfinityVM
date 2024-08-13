//! Job processor implementation.

use crate::{metrics::Metrics, relayer::JobRelayer};
use alloy::{hex, primitives::Signature, signers::Signer};
use async_channel::{Receiver, Sender};
use db::{
    delete_fail_relay_job, get_all_failed_jobs, get_elf, get_fail_relay_job, get_job, put_elf,
    put_fail_relay_job, put_job,
};
use proto::{CreateElfRequest, ExecuteRequest, Job, JobInputs, JobStatus, JobStatusType, VmType};
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
    /// Job ID conversion error
    #[error("job_id_conversion")]
    JobIdConversion,
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
        let request = CreateElfRequest { program_elf: elf.clone(), vm_type };
        let response = self.zk_executor.create_elf_handler(request).await?;

        if get_elf(self.db.clone(), &response.verifying_key)
            .map_err(|e| Error::ElfReadFailed(e.to_string()))?
            .is_some()
        {
            return Err(Error::ElfAlreadyExists(format!(
                "elf with verifying key {:?} already exists",
                hex::encode(response.verifying_key.as_slice()),
            )));
        }

        put_elf(
            self.db.clone(),
            VmType::try_from(vm_type).map_err(|_| Error::InvalidVmType)?,
            &response.verifying_key,
            elf,
        )
        .map_err(|e| Error::ElfWriteFailed(e.to_string()))?;

        Ok(response.verifying_key)
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
        let job_id: [u8; 32] = job.id.clone().try_into().map_err(|_| Error::JobIdConversion)?;
        if self.has_job(job_id).await? {
            return Err(Error::JobAlreadyExists);
        }

        job.status = Some(JobStatus {
            status: JobStatusType::Pending as i32,
            failure_reason: None,
            retries: 0,
        });

        Self::save_job(self.db.clone(), job.clone()).await?;

        // If the channel is full, this method waits until there is space for a message.
        // In the future we may want to switch to try_send, so it just fails immediately if
        // the queue is full.
        // <https://docs.rs/async-channel/latest/async_channel/struct.Sender.html#method.send>
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
            let id = job.id.clone();
            info!("executing job {:?}", id);

            let elf_with_meta = match db::get_elf(db.clone(), &job.program_verifying_key) {
                Ok(Some(elf)) => elf,
                Ok(None) => {
                    error!(
                        ?job.contract_address,
                        "no ELF found for job {:?} with verifying key {:?}",
                        id,
                        job.program_verifying_key,
                    );
                    metrics.incr_job_err(&FailureReason::MissingElf.to_string());

                    job.status = Some(JobStatus {
                        status: JobStatusType::Failed as i32,
                        failure_reason: Some(FailureReason::MissingElf.to_string()),
                        retries: 0,
                    });

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
                        job.program_verifying_key
                    );

                    metrics.incr_job_err(&FailureReason::ErrGetElf.to_string());

                    job.status = Some(JobStatus {
                        status: JobStatusType::Failed as i32,
                        failure_reason: Some(FailureReason::ErrGetElf.to_string()),
                        retries: 0,
                    });

                    if let Err(e) = Self::save_job(db.clone(), job).await {
                        error!("report this error: failed to save job {:?}: {:?}", id, e);
                        metrics.incr_job_err("db_error_status_failed");
                    }
                    continue;
                }
            };

            let req = ExecuteRequest {
                inputs: Some(JobInputs {
                    job_id: id.clone(),
                    max_cycles: job.max_cycles,
                    program_verifying_key: job.program_verifying_key.clone(),
                    program_input: job.input.clone(),
                    program_elf: elf_with_meta.elf,
                    vm_type: VmType::Risc0 as i32,
                }),
            };

            let job = match zk_executor.execute_handler(req).await {
                Ok(resp) => {
                    tracing::debug!("job {:?} executed successfully", id.clone());

                    job.status = Some(JobStatus {
                        status: JobStatusType::Done as i32,
                        failure_reason: None,
                        retries: 0,
                    });

                    job.result = resp.result_with_metadata;
                    job.zkvm_operator_address = resp.zkvm_operator_address;
                    job.zkvm_operator_signature = resp.zkvm_operator_signature;

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

                    job.status = Some(JobStatus {
                        status: JobStatusType::Failed as i32,
                        failure_reason: Some(FailureReason::ExecErr.to_string()),
                        retries: 0,
                    });

                    if let Err(e) = Self::save_job(db.clone(), job.clone()).await {
                        error!("report this error: failed to save job {:?}: {:?}", id, e);
                        metrics.incr_job_err(&FailureReason::DbErrStatusDone.to_string());
                    }
                    continue;
                }
            };

            let _tx_receipt = match job_relayer.relay(job.clone()).await {
                Ok(tx_receipt) => tx_receipt,
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
                    continue
                }
            };

            // Retry each once
            for mut job in retry_jobs {
                let job_id = job.id.clone().try_into().map_err(|_| Error::JobIdConversion)?;

                let result = job_relayer.relay(job.clone()).await;
                match result {
                    Ok(_) => {
                        info!("successfully retried job relay for job: {:?}", job_id);
                        jobs_to_delete.push(job_id);
                    }
                    Err(e) => {
                        let status = match job.status.as_ref() {
                            Some(status) => status,
                            None => {
                                error!("error retrieving status for job: {:?}", job_id);
                                continue
                            }
                        };
                        if status.retries == max_retries {
                            metrics.incr_relay_err(&FailureReason::RelayErrExceedRetry.to_string());
                            jobs_to_delete.push(job_id);
                        } else {
                            error!(
                                "report this error: failed to retry relaying job {:?}: {:?}",
                                job_id, e
                            );
                            metrics.incr_relay_err(&FailureReason::RelayErr.to_string());
                            if let Some(status) = job.status.as_mut() {
                                status.retries += 1
                            }
                            if let Err(e) =
                                Self::save_relay_error_job(db.clone(), job.clone()).await
                            {
                                error!(
                                    "report this error: failed to save retried job {:?}: {:?}",
                                    job_id, e
                                );
                                metrics.incr_job_err(&FailureReason::DbErrStatusFailed.to_string());
                            }
                        }
                        continue
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

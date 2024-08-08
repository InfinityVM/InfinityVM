//! Job processor implementation.

use alloy::{hex, primitives::Signature, signers::Signer};
use proto::{CreateElfRequest, ExecuteRequest, Job, JobInputs, JobStatus, JobStatusType, VmType};
use std::{marker::Send, sync::Arc};

use crate::{metrics::Metrics, relayer::JobRelayer};
use async_channel::{Receiver, Sender};
use db::{get_elf, get_job, put_elf, put_job};
use reth_db::Database;
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
    pub async fn has_job(&self, job_id: u32) -> Result<bool, Error> {
        let job = get_job(self.db.clone(), job_id)?;
        Ok(job.is_some())
    }

    /// Returns job with `job_id` from DB
    pub async fn get_job(&self, job_id: u32) -> Result<Option<Job>, Error> {
        let job = get_job(self.db.clone(), job_id)?;
        Ok(job)
    }

    /// Submits job, saves it in DB, and adds to exec queue
    pub async fn submit_job(&self, mut job: Job) -> Result<(), Error> {
        let job_id = job.id;
        if self.has_job(job_id).await? {
            return Err(Error::JobAlreadyExists);
        }

        job.status =
            Some(JobStatus { status: JobStatusType::Pending as i32, failure_reason: None });

        Self::save_job(self.db.clone(), job.clone()).await?;

        // If the channel is full, this method waits until there is space for a message.
        // In the future we may want to switch to try_send, so it just fails immediately if
        // the queue is full.
        // <https://docs.rs/async-channel/latest/async_channel/struct.Sender.html#method.send>
        self.exec_queue_sender.send(job).await.map_err(|_| Error::ExecQueueSendFailed)?;

        Ok(())
    }

    /// Start the job processor and spawn `num_workers` worker threads.
    pub async fn start(&mut self, num_workers: usize) {
        for _ in 0..num_workers {
            let exec_queue_receiver = self.exec_queue_receiver.clone();
            let db = Arc::clone(&self.db);
            let zk_executor = self.zk_executor.clone();
            let job_relayer = Arc::clone(&self.job_relayer);
            let metrics = Arc::clone(&self.metrics);

            self.task_handles.spawn(async move {
                Self::start_worker(exec_queue_receiver, db, job_relayer, zk_executor, metrics).await
            });
        }
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
            info!("executing job {}", id);

            let elf_with_meta = match db::get_elf(db.clone(), &job.program_verifying_key) {
                Ok(Some(elf)) => elf,
                Ok(None) => {
                    error!(
                        ?job.contract_address,
                        "no ELF found for job {} with verifying key {:?}",
                        id,
                        job.program_verifying_key,
                    );
                    metrics.incr_job_err(&FailureReason::MissingElf.to_string());

                    job.status = Some(JobStatus {
                        status: JobStatusType::Failed as i32,
                        failure_reason: Some(FailureReason::MissingElf.to_string()),
                    });

                    if let Err(e) = Self::save_job(db.clone(), job).await {
                        error!("report this error: failed to save job {}: {:?}", id, e);
                        metrics.incr_job_err(&FailureReason::DbErrMissingElf.to_string());
                    }
                    continue;
                }
                Err(error) => {
                    error!(
                        ?error,
                        "could not find elf for job {} with verifying key {:?}",
                        id,
                        job.program_verifying_key
                    );

                    metrics.incr_job_err(&FailureReason::ErrGetElf.to_string());

                    job.status = Some(JobStatus {
                        status: JobStatusType::Failed as i32,
                        failure_reason: Some(FailureReason::ErrGetElf.to_string()),
                    });

                    if let Err(e) = Self::save_job(db.clone(), job).await {
                        error!("report this error: failed to save job {}: {:?}", id, e);
                        metrics.incr_job_err("db_error_status_failed");
                    }
                    continue;
                }
            };

            let req = ExecuteRequest {
                inputs: Some(JobInputs {
                    job_id: id,
                    max_cycles: job.max_cycles,
                    program_verifying_key: job.program_verifying_key.clone(),
                    program_input: job.input.clone(),
                    program_elf: elf_with_meta.elf,
                    vm_type: VmType::Risc0 as i32,
                }),
            };

            let job = match zk_executor.execute_handler(req).await {
                Ok(resp) => {
                    tracing::debug!("job {} executed successfully", id);

                    job.status = Some(JobStatus {
                        status: JobStatusType::Done as i32,
                        failure_reason: None,
                    });

                    job.result = resp.result_with_metadata;
                    job.zkvm_operator_address = resp.zkvm_operator_address;
                    job.zkvm_operator_signature = resp.zkvm_operator_signature;

                    if let Err(e) = Self::save_job(db.clone(), job.clone()).await {
                        error!("report this error: failed to save job {}: {:?}", id, e);
                        metrics.incr_job_err(&FailureReason::DbErrStatusFailed.to_string());
                        continue;
                    }
                    job
                }
                Err(e) => {
                    error!("failed to execute job {}: {:?}", id, e);
                    metrics.incr_job_err(&FailureReason::ExecErr.to_string());

                    job.status = Some(JobStatus {
                        status: JobStatusType::Failed as i32,
                        failure_reason: Some(FailureReason::ExecErr.to_string()),
                    });

                    if let Err(e) = Self::save_job(db.clone(), job).await {
                        error!("report this error: failed to save job {}: {:?}", id, e);
                        metrics.incr_job_err(&FailureReason::DbErrStatusDone.to_string());
                    }
                    continue;
                }
            };

            let _tx_receipt = match job_relayer.relay(job).await {
                Ok(tx_receipt) => tx_receipt,
                Err(e) => {
                    error!("report this error: failed to relay job {}: {:?}", id, e);
                    metrics.incr_relay_err(&FailureReason::RelayErr.to_string());
                    continue;
                }
            };
        }
    }
}

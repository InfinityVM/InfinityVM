//! Job processor implementation.

use alloy::{primitives::Signature, signers::Signer};
use proto::{CreateElfRequest, ExecuteRequest, Job, JobInputs, JobStatus, VmType};
use std::{marker::Send, sync::Arc};

use async_channel::{Receiver, Sender};
use base64::prelude::*;
use db::{get_elf, get_job, put_elf, put_job};
use reth_db::Database;
use tokio::task::JoinSet;
use tokio_util::sync::CancellationToken;
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
}

/// Job processor service.
#[derive(Debug)]
pub struct JobProcessorService<S, D> {
    db: Arc<D>,
    exec_queue_sender: Sender<Job>,
    exec_queue_receiver: Receiver<Job>,
    broadcast_queue_sender: Sender<Job>,
    zk_executor: ZkvmExecutorService<S>,
    task_handles: JoinSet<()>,
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
        broadcast_queue_sender: Sender<Job>,
        zk_executor: ZkvmExecutorService<S>,
    ) -> Self {
        Self {
            db,
            exec_queue_sender,
            exec_queue_receiver,
            broadcast_queue_sender,
            zk_executor,
            task_handles: JoinSet::new(),
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
                BASE64_STANDARD.encode(response.verifying_key.as_slice()),
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

        job.status = JobStatus::Pending.into();

        Self::save_job(self.db.clone(), job.clone()).await?;
        self.exec_queue_sender.send(job).await.map_err(|_| Error::ExecQueueSendFailed)?;

        Ok(())
    }

    /// Start the job processor and spawn `num_workers` worker threads.
    pub async fn start(&mut self, num_workers: usize, cancellation_token: CancellationToken) {
        for _ in 0..num_workers {
            let exec_queue_receiver = self.exec_queue_receiver.clone();
            let broadcast_queue_sender = self.broadcast_queue_sender.clone();
            let db = Arc::clone(&self.db);
            let zk_executor = self.zk_executor.clone();
            let token = cancellation_token.clone();
            self.task_handles.spawn(async move {
                Self::start_worker(exec_queue_receiver, broadcast_queue_sender, db, zk_executor, token)
                    .await;
            });
        }
    }

    /// Start a single worker thread.
    async fn start_worker(
        exec_queue_receiver: Receiver<Job>,
        broadcast_queue_sender: Sender<Job>,
        db: Arc<D>,
        zk_executor: ZkvmExecutorService<S>,
        cancellation_token: CancellationToken,
    ) {
        loop {
            if cancellation_token.is_cancelled() {
                info!("Cancel Signal Received.");
                break;
            }

            match exec_queue_receiver.recv().await {
                Ok(mut job) => {
                    let id = job.id;
                    info!("Executing job {}", id);

                    let elf_with_meta = match db::get_elf(db.clone(), &job.program_verifying_key) {
                        Ok(Some(elf)) => elf,
                        Ok(None) => {
                            error!(
                                "No ELF found for job {} with verifying key {:?}",
                                id, &job.program_verifying_key
                            );

                            job.status = JobStatus::Failed.into();
                            if let Err(e) = Self::save_job(db.clone(), job).await {
                                error!("Failed to save job {}: {:?}", id, e);
                            }
                            continue;
                        }
                        Err(e) => {
                            error!(
                                "could not find elf for job {} with verifying key {:?}: {:?}",
                                id, &job.program_verifying_key, e
                            );
                            job.status = JobStatus::Failed.into();
                            if let Err(e) = Self::save_job(db.clone(), job).await {
                                error!("Failed to save job {}: {:?}", id, e);
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

                    match zk_executor.execute_handler(req).await {
                        Ok(resp) => {
                            info!("Job {} executed successfully", id);

                            job.status = JobStatus::Done.into();
                            job.result = resp.result_with_metadata;
                            job.zkvm_operator_address = resp.zkvm_operator_address;
                            job.zkvm_operator_signature = resp.zkvm_operator_signature;

                            if let Err(e) = Self::save_job(db.clone(), job.clone()).await {
                                error!("Failed to save job {}: {:?}", id, e);
                                continue;
                            }

                            info!("Pushing finished job {} to broadcast queue", id);
                            if let Err(e) = broadcast_queue_sender.send(job).await {
                                error!("Failed to push job {} to broadcast queue: {:?}", id, e);
                                continue;
                            }
                        }
                        Err(e) => {
                            error!("Failed to execute job {}: {:?}", id, e);

                            job.status = JobStatus::Failed.into();

                            if let Err(e) = Self::save_job(db.clone(), job).await {
                                error!("Failed to save job {}: {:?}", id, e);
                            }
                        }
                    }
                }
                Err(error) => {
                    error!(?error, "job executor worker stopped due to error");
                    break;
                }
            }
        }

        info!("Exiting...");
    }
}

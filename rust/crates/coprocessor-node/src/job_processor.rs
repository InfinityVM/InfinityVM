//! Job processor implementation.

use alloy::{primitives::Signature, signers::Signer};
use proto::{CreateElfRequest, ExecuteRequest, Job, JobInputs, JobStatus, VmType};
use std::{marker::Send, sync::Arc};

use base64::prelude::*;
use crossbeam::channel::{Receiver, Sender};
use db::{get_job, put_job};
use reth_db::Database;
use tracing::{error, info};
use zkvm_executor::service::ZkvmExecutorService;

/// Errors from job processor
#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("zkvm executor error: {0}")]
    ZkvmExecutorFailed(#[from] zkvm_executor::service::Error),
    /// database error
    #[error("database error: {0}")]
    Database(#[from] db::Error),
    #[error("elf with verifying key {0} already exists")]
    ElfAlreadyExists(String),
    #[error("job already exists")]
    JobAlreadyExists,
    #[error("failed to send to exec queue")]
    ExecQueueSendFailed,
    #[error("failed reading elf: {0}")]
    ElfReadFailed(String),
    #[error("failed writing elf: {0}")]
    ElfWriteFailed(String),
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
}

impl<S, D> JobProcessorService<S, D>
where
    S: Signer<Signature> + Send + Sync + Clone + 'static,
    D: Database + 'static,
{
    /// Create a new job processor service.
    pub const fn new(
        db: Arc<D>,
        exec_queue_sender: Sender<Job>,
        exec_queue_receiver: Receiver<Job>,
        broadcast_queue_sender: Sender<Job>,
        zk_executor: ZkvmExecutorService<S>,
    ) -> Self {
        Self { db, exec_queue_sender, exec_queue_receiver, broadcast_queue_sender, zk_executor }
    }

    pub async fn submit_elf(&self, elf: Vec<u8>, vm_type: i32) -> Result<Vec<u8>, Error> {
        let request = CreateElfRequest { program_elf: elf.clone(), vm_type };
        let response = self.zk_executor.create_elf_handler(request).await?;

        if db::get_elf(self.db.clone(), &response.verifying_key)
            .map_err(|e| Error::ElfReadFailed(e.to_string()))?
            .is_some()
        {
            return Err(Error::ElfAlreadyExists(format!(
                "elf with verifying key {:?} already exists",
                BASE64_STANDARD.encode(response.verifying_key.as_slice()),
            )));
        }

        db::put_elf(
            self.db.clone(),
            VmType::try_from(vm_type).map_err(|_| Error::InvalidVmType)?,
            &response.verifying_key,
            elf,
        )
        .map_err(|e| Error::ElfWriteFailed(e.to_string()))?;

        Ok(response.verifying_key)
    }

    pub async fn save_job(db: Arc<D>, job: Job) -> Result<(), Error> {
        put_job(db.clone(), job)?;
        Ok(())
    }

    pub async fn has_job(&self, job_id: u32) -> Result<bool, Error> {
        let job = get_job(self.db.clone(), job_id)?;
        Ok(job.is_some())
    }

    pub async fn get_job(&self, job_id: u32) -> Result<Option<Job>, Error> {
        let job = get_job(self.db.clone(), job_id)?;
        Ok(job)
    }

    pub async fn submit_job(&self, mut job: Job) -> Result<(), Error> {
        let job_id = job.id;
        if self.has_job(job_id).await? {
            return Err(Error::JobAlreadyExists);
        }

        job.status = JobStatus::Pending.into();

        Self::save_job(self.db.clone(), job.clone()).await?;
        self.exec_queue_sender.send(job).map_err(|_| Error::ExecQueueSendFailed)?;

        Ok(())
    }

    pub async fn start(&self, num_workers: usize) {
        let mut handles = vec![];

        for _ in 0..num_workers {
            let exec_queue_receiver = self.exec_queue_receiver.clone();
            let broadcast_queue_sender = self.broadcast_queue_sender.clone();
            let db = Arc::clone(&self.db);
            let zk_executor = self.zk_executor.clone();

            let handle = tokio::spawn(async move {
                Self::start_worker(exec_queue_receiver, broadcast_queue_sender, db, zk_executor)
                    .await;
            });
            handles.push(handle);
        }

        for handle in handles {
            // TODO (Maanav): How should we handle shutting down worker threads?
            // [ref]: https://github.com/Ethos-Works/InfinityVM/issues/116
            match handle.await {
                Ok(_) => {}
                Err(e) => {
                    error!("Worker task failed: {:?}", e);
                }
            }
        }
    }

    async fn start_worker(
        exec_queue_receiver: Receiver<Job>,
        broadcast_queue_sender: Sender<Job>,
        db: Arc<D>,
        zk_executor: ZkvmExecutorService<S>,
    ) {
        loop {
            match exec_queue_receiver.recv() {
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
                            if let Err(e) = broadcast_queue_sender.send(job) {
                                // TODO (Maanav): Should we set job status to FAILED here?
                                // [ref]: https://github.com/Ethos-Works/InfinityVM/issues/117
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
                    error!(?error, "job executor worker stopped due to error")
                    break;
                }
            }
        }
    }
}

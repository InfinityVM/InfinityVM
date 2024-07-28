//! Job processor implementation.

use alloy::{
    primitives::Signature,
    signers::Signer,
};
use proto::{CreateElfRequest, CreateElfResponse, ExecuteRequest, Job, JobInputs, JobStatus, VmType};
use std::{marker::Send, sync::Arc};

use crossbeam::channel::{self, Receiver, Sender};
use zkvm_executor::service::ZkvmExecutorService;
use reth_db::Database;
use db::{get_job, put_job};
use tracing::info;

/// Errors from job processor
#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("zkvm executor error: {0}")]
    ZkvmExecutorFailed(#[from] zkvm_executor::service::Error),
    /// database error
    #[error("database error: {0}")]
    Database(#[from] db::Error),
    #[error("job already exists")]
    JobAlreadyExists,
    #[error("failed to send to exec queue")]
    ExecQueueSendFailed,
}

/// Job processor service.
#[derive(Debug)]
pub struct JobProcessorService<S, D> {
    db: Arc<D>,
    exec_queue_sender: Arc<Sender<Job>>,
    exec_queue_receiver: Arc<Receiver<Job>>,
    broadcast_queue_sender: Arc<Sender<Job>>,
    zk_executor: ZkvmExecutorService<S, D>,
}

impl<S, D> JobProcessorService<S, D>
where
    S: Signer<Signature> + Send + Sync + 'static,
    D: Database + 'static,
{
    /// Create a new job processor service.
    pub const fn new(db: Arc<D>, exec_queue_sender: Arc<Sender<Job>>, exec_queue_receiver: Arc<Receiver<Job>>, broadcast_queue_sender: Arc<Sender<Job>>, zk_executor: ZkvmExecutorService<S, D>) -> Self {
        Self { db, exec_queue_sender, exec_queue_receiver, broadcast_queue_sender, zk_executor }
    }

    pub async fn submit_elf(&self, elf: Vec<u8>, vm_type: i32) -> Result<Vec<u8>, Error> {
        let request = CreateElfRequest { program_elf: elf, vm_type: vm_type };
        let response = self.zk_executor.create_elf_handler(request).await?;
        Ok(response.verifying_key)
    }

    pub async fn save_job(&self, job: Job) -> Result<(), Error> {
        put_job(self.db.clone(), job)?;
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

        self.save_job(job.clone()).await?;
        self.exec_queue_sender.send(job).map_err(|_| Error::ExecQueueSendFailed)?;

        Ok(())
    }

    pub async fn start(self: Arc<Self>, num_workers: usize) {
        let mut handles = vec![];

        for _ in 0..num_workers {
            let this = Arc::clone(&self);
            let handle = tokio::spawn(async move {
                this.start_worker().await;
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.await.unwrap();
        }
    }

    async fn start_worker(self: Arc<Self>) {
        loop {
            match self.exec_queue_receiver.recv() {
                Ok(mut job) => {
                    info!("Executing job {}", job.id);

                    let req = ExecuteRequest {
                        inputs: Some(JobInputs {
                            job_id: job.id,
                            max_cycles: job.max_cycles,
                            program_verifying_key: job.program_verifying_key.clone(),
                            program_input: job.input.clone(),
                        })
                    };

                    match self.zk_executor.execute_handler(req).await {
                        Ok(resp) => {
                            info!("Job {} executed successfully", job.id);

                            job.status = JobStatus::Done.into();
                            job.result = Some(resp.result_with_metadata);
                            job.zkvm_operator_address = Some(resp.zkvm_operator_address);
                            job.zkvm_operator_signature = Some(resp.zkvm_operator_signature);

                            if let Err(e) = self.save_job(job.clone()).await {
                                tracing::error!("Failed to save job {}: {:?}", job.id, e);
                            }

                            if let Err(e) = self.broadcast_queue_sender.send(job) {
                                tracing::error!("Failed to push job {} to broadcast queue: {:?}", job.id, e);
                            }
                        }
                        Err(e) => {
                            tracing::error!("Failed to execute job {}: {:?}", job.id, e);

                            job.status = JobStatus::Failed.into();

                            if let Err(e) = self.save_job(job).await {
                                tracing::error!("Failed to save job {}: {:?}", job.id, e);
                            }
                        }
                    }
                }
                Err(_) => {
                    tracing::info!("Worker stopping...");
                    break;
                }
            }
        }
    }
}


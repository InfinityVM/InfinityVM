//! Job processor implementation.

use alloy::{
    primitives::Signature,
    signers::Signer,
};
use proto::{Job, CreateElfRequest, CreateElfResponse, VmType, JobStatus};
use std::{marker::Send, sync::Arc};

use crossbeam::queue::ArrayQueue;
use zkvm_executor::service::ZkvmExecutorService;
use reth_db::Database;
use db::{get_job, put_job};

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
    #[error("failed to push to exec queue")]
    ExecQueuePushFailed,
}

/// Job processor service.
#[derive(Debug)]
pub struct JobProcessorService<S, D> {
    db: Arc<D>,
    exec_queue: Arc<ArrayQueue<Job>>,
    broadcast_queue: Arc<ArrayQueue<Job>>,
    zk_executor: ZkvmExecutorService<S, D>,
}

impl<S, D> JobProcessorService<S, D>
where
    S: Signer<Signature> + Send + Sync + 'static,
    D: Database,
{
    /// Create a new job processor service.
    pub const fn new(db: Arc<D>, exec_queue: Arc<ArrayQueue<Job>>, broadcast_queue: Arc<ArrayQueue<Job>>, zk_executor: ZkvmExecutorService<S, D>) -> Self {
        Self { db, exec_queue, broadcast_queue, zk_executor }
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
        self.exec_queue.push(job).map_err(|_| Error::ExecQueuePushFailed)?;

        Ok(())
    }
}

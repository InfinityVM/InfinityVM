//! Job processor implementation.

use alloy::{
    primitives::Signature,
    signers::Signer,
};
use proto::{Job, CreateElfRequest, CreateElfResponse, VmType};
use std::{marker::Send, sync::Arc};

use crossbeam::queue::ArrayQueue;
use zkvm_executor::service::ZkvmExecutorService;
use reth_db::Database;

/// Errors from job processor
#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("zkvm executor error: {0}")]
    ZkvmExecutorFailed(#[from] zkvm_executor::service::Error),
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

    async fn submit_elf(&self, elf: Vec<u8>, vm_type: VmType) -> Result<Vec<u8>, Error> {
        let request = CreateElfRequest { program_elf: elf, vm_type: vm_type.into() };
        let response = self.zk_executor.create_elf_handler(request).await?;
        Ok(response.verifying_key)
    }
}


//! Job processor implementation.

use alloy::{
    primitives::Signature,
    signers::Signer,
};
use proto::Job;
use std::{marker::Send, sync::Arc};

use crossbeam::queue::ArrayQueue;
use zkvm_executor::service::ZkvmExecutorService;
use reth_db::Database;

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
}

//! Handlers for submitting new jobs and programs.
//!
//! For new programs we check that they do not already exist and then persist.
//!
//! For new jobs we check that the job does not exist, persist it and push it onto the exec queue.

use crate::{
    job_processor::relay_job_result,
    queue::{self, Queues2},
    relayer::JobRelayer,
    writer::{Write, WriterMsg},
};
use alloy::{hex, primitives::Signature, signers::Signer};
use ivm_db::{get_elf, get_job, tables::Job};
use ivm_proto::{JobStatus, JobStatusType, RelayStrategy, VmType};
use reth_db::Database;
use std::{
    sync::{mpsc::SyncSender, Arc},
    time::Duration,
};
use tokio::{sync::oneshot, time::interval};
use tracing::{span, Instrument, Level};
use zkvm_executor::service::ZkvmExecutorService;

/// Errors from job processor
#[derive(thiserror::Error, Debug)]
pub enum Error {
    /// Could not create ELF in zkvm executor
    #[error("failed to create ELF in zkvm executor: {0}")]
    CreateElfFailed(#[from] zkvm_executor::service::Error),
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
    /// Invalid VM type
    #[error("invalid VM type")]
    InvalidVmType,
    /// Offchain input over max DA per job
    #[error("offchain input over max DA per job")]
    OffchainInputOverMaxDAPerJob,
    /// Could not push the job id onto the relay queue
    #[error("failed to push to queue: {0}")]
    FailedToPushToQueue(#[from] queue::Error),
}

/// Job and program intake handlers.
///
/// New, valid jobs submitted to this service will be sent over the exec queue to the job processor.
#[derive(Debug)]
pub struct IntakeHandlers<S, D> {
    db: Arc<D>,
    exec_queue_sender: async_channel::Sender<Job>,
    zk_executor: ZkvmExecutorService<S>,
    max_da_per_job: usize,
    queues: Queues2,
    relayer: Arc<JobRelayer>,
    writer_tx: SyncSender<WriterMsg>,
}

impl<S, D> IntakeHandlers<S, D>
where
    S: Signer<Signature> + Send + Sync + Clone + 'static,
    D: Database + 'static,
{
    /// Create an instance of [Self].
    pub const fn new(
        db: Arc<D>,
        exec_queue_sender: async_channel::Sender<Job>,
        zk_executor: ZkvmExecutorService<S>,
        max_da_per_job: usize,
        queues: Queues2,
        relayer: Arc<JobRelayer>,
        writer_tx: SyncSender<WriterMsg>,
    ) -> Self {
        Self { db, exec_queue_sender, zk_executor, max_da_per_job, queues, relayer, writer_tx }
    }

    /// Submits job, saves it in DB, and pushes on the exec queue.
    pub async fn submit_job(&self, mut job: Job) -> Result<(), Error> {
        if job.offchain_input.len() > self.max_da_per_job {
            return Err(Error::OffchainInputOverMaxDAPerJob);
        };

        // TODO: add new table for just job ID so we can avoid writing full job here and reading.
        // We can just pass the job itself along the channel
        // full job https://github.com/InfinityVM/InfinityVM/issues/354
        if get_job(self.db.clone(), job.id)?.is_some() {
            return Err(Error::JobAlreadyExists);
        }

        job.status =
            JobStatus { status: JobStatusType::Pending as i32, failure_reason: None, retries: 0 };
        let (tx, db_write_complete_rx) = oneshot::channel();
        self.writer_tx.send((Write::JobTable(job.clone()), Some(tx))).expect("db writer broken");

        let consumer_address =
            job.consumer_address.clone().try_into().expect("we checked for valid address length");
        if job.relay_strategy == RelayStrategy::Ordered {
            let is_empty = self.queues.peek_back(consumer_address).is_none();
            self.queues.push_front(consumer_address, job.id);

            if is_empty {
                // If the queue was empty we assume there is no background task
                let queues2 = self.queues.clone();
                let db2 = self.db.clone();
                let relayer2 = self.relayer.clone();
                let writer_tx2 = self.writer_tx.clone();
                tokio::spawn(async move {
                    let mut interval = interval(Duration::from_millis(100));
                    while let Some(job_id) = queues2.peek_back(consumer_address) {
                        interval.tick().await;

                        let job = match get_job(db2.clone(), job_id).expect("job get db error") {
                            Some(job) => job,
                            // Edge case where the job has not been written yet
                            None => continue,
                        };

                        match job.status.status() {
                            JobStatusType::Done => {
                                let _job_id = queues2
                                    .pop_back(consumer_address)
                                    .expect("queue is unexpected empty");
                                debug_assert_eq!(job_id, _job_id);
                                tracing::info!(
                                    consumer = hex::encode(consumer_address),
                                    "relaying ordered"
                                );
                                let _ = relay_job_result(job, relayer2.clone(), writer_tx2.clone())
                                    .instrument(span!(Level::INFO, "ordered_relay"))
                                    .await;
                            }
                            JobStatusType::Relayed => {
                                tracing::error!(
                                    "logical error: a job in the queue was already relayed"
                                );
                                continue;
                            }
                            _ => continue,
                        }
                    }
                });
            }
        };

        self.exec_queue_sender.send(job).await.map_err(|_| Error::ExecQueueSendFailed)?;
        let _ = db_write_complete_rx.await;

        Ok(())
    }

    /// Submit program ELF, save it in DB, and return verifying key.
    pub async fn submit_elf(&self, elf: Vec<u8>, vm_type: i32) -> Result<Vec<u8>, Error> {
        let vm_type = VmType::try_from(vm_type).map_err(|_| Error::InvalidVmType)?;
        let program_id = self.zk_executor.create_elf(&elf, vm_type).await?;

        if get_elf(self.db.clone(), &program_id)
            .map_err(|e| Error::ElfReadFailed(e.to_string()))?
            .is_some()
        {
            return Err(Error::ElfAlreadyExists(format!(
                "elf with verifying key {:?} already exists",
                hex::encode(program_id.as_slice()),
            )));
        }

        // Write the elf and make sure it completes before responding to the user.
        let (tx, rx) = oneshot::channel();
        self.writer_tx
            .send((Write::Elf { vm_type, program_id: program_id.clone(), elf }, Some(tx)))
            .expect("writer channel broken");
        let _ = rx.await;

        Ok(program_id)
    }

    /// Returns job with `job_id` from DB
    pub async fn get_job(&self, job_id: [u8; 32]) -> Result<Option<Job>, Error> {
        let job = get_job(self.db.clone(), job_id)?;
        Ok(job)
    }
}

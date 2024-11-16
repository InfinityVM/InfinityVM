//! [`JobExecutor`] handles the execution of jobs.

use crate::{
    metrics::Metrics,
    relayer::Relay,
    writer::{Write, WriterMsg},
};
use alloy::{hex, primitives::Signature, signers::Signer};
use async_channel::Receiver;
use ivm_db::tables::{ElfWithMeta, Job, RequestType};
use ivm_proto::{JobStatus, JobStatusType, VmType};
use reth_db::Database;
use std::{
    marker::Send,
    sync::{mpsc::SyncSender, Arc},
};
use tokio::task::JoinSet;
use tracing::{error, info, instrument};
use zkvm_executor::service::ZkvmExecutorService;

/// Errors for this module.
#[derive(thiserror::Error, Debug)]
pub enum Error {
    /// database error
    #[error("database error: {0}")]
    Database(#[from] ivm_db::Error),
    /// Job already exists in DB
    #[error("job already exists")]
    JobAlreadyExists,
    /// Failed to send job to exec queue
    #[error("failed to send to exec queue")]
    ExecQueueSendFailed,
    /// Invalid VM type
    #[error("invalid VM type")]
    InvalidVmType,
    /// exec queue channel unexpected closed
    #[error("exec queue channel unexpected closed")]
    ExecQueueChannelClosed,
}

/// `JobStatus` failure reasons. These are used in metrics to record failure reasons.
#[derive(thiserror::Error, Debug)]
pub enum FailureReason {
    /// Job submitted with unknown or missing ELF
    #[error("missing_elf")]
    MissingElf,
    /// No ELF found in DB
    #[error("db_error_missing_elf")]
    DbErrMissingElf,
    /// Error retrieving elf because of low-level DB error
    #[error("db_error_get_elf")]
    DbErrGetElf,
    /// Unable to persist failed job to DB
    #[error("db_error_status_failed")]
    DbErrStatusFailed,
    /// Error executing job against zkvm
    #[error("execution_error")]
    ExecErr,
    /// Unable to persist successfully completed job to DB
    #[error("db_error_status_done")]
    DbErrStatusDone,
    /// Unable to persist relayed job to DB
    #[error("db_error_status_relayed")]
    DbErrStatusRelayed,
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

/// Job processor config.
#[derive(Debug)]
pub struct JobExecutorConfig {
    /// Number of worker threads to run.
    pub num_workers: usize,
    /// Maximum number of retries for a job.
    pub max_retries: u32,
}

/// Job processor service.
///
/// This stores a `JoinSet` with a handle to each job processor worker and the job retry task.
#[derive(Debug)]
pub struct JobExecutor<S, D> {
    db: Arc<D>,
    exec_queue_receiver: Receiver<Job>,
    zk_executor: ZkvmExecutorService<S>,
    task_handles: JoinSet<Result<(), Error>>,
    metrics: Arc<Metrics>,
    num_workers: usize,
    writer_tx: SyncSender<WriterMsg>,
    relay_tx: tokio::sync::mpsc::Sender<Relay>,
}

// The DB functions in JobExecutor are async so they yield in the tokio task.
impl<S, D> JobExecutor<S, D>
where
    S: Signer<Signature> + Send + Sync + Clone + 'static,
    D: Database + 'static,
{
    /// Create a new instance of [Self].
    pub fn new(
        db: Arc<D>,
        exec_queue_receiver: Receiver<Job>,
        zk_executor: ZkvmExecutorService<S>,
        metrics: Arc<Metrics>,
        num_workers: usize,
        writer_tx: SyncSender<WriterMsg>,
        relay_tx: tokio::sync::mpsc::Sender<Relay>,
    ) -> Self {
        Self {
            db,
            exec_queue_receiver,
            zk_executor,
            task_handles: JoinSet::new(),
            metrics,
            num_workers,
            writer_tx,
            relay_tx,
        }
    }

    /// Spawns `num_workers` worker tasks.
    pub async fn start(&mut self) {
        for _ in 0..self.num_workers {
            let exec_queue_receiver = self.exec_queue_receiver.clone();
            let db = Arc::clone(&self.db);
            let zk_executor = self.zk_executor.clone();
            let metrics = Arc::clone(&self.metrics);
            let writer_tx = self.writer_tx.clone();
            let relay_tx = self.relay_tx.clone();

            self.task_handles.spawn(async move {
                Self::start_processor_worker(
                    exec_queue_receiver,
                    db,
                    zk_executor,
                    metrics,
                    writer_tx,
                    relay_tx,
                )
                .await
            });
        }
    }

    /// Start a worker.
    async fn start_processor_worker(
        exec_queue_receiver: Receiver<Job>,
        db: Arc<D>,
        zk_executor: ZkvmExecutorService<S>,
        metrics: Arc<Metrics>,
        writer_tx: SyncSender<WriterMsg>,
        relay_tx: tokio::sync::mpsc::Sender<Relay>,
    ) -> Result<(), Error> {
        loop {
            let writer_tx2 = writer_tx.clone();
            let mut job =
                exec_queue_receiver.recv().await.map_err(|_| Error::ExecQueueChannelClosed)?;
            let id = job.id;
            info!("executing job {:?}", hex::encode(id));

            let elf_with_meta =
                match Self::get_elf(&db, &mut job, &metrics, writer_tx2.clone()).await {
                    Ok(elf) => elf,
                    Err(_) => continue,
                };

            job = match Self::execute_job(
                job,
                &zk_executor,
                elf_with_meta,
                &metrics,
                writer_tx2.clone(),
            )
            .await
            {
                Ok(updated_job) => updated_job,
                Err(_) => continue,
            };

            relay_tx.send(Relay::Now(Box::new(job))).await.expect("relay channel is broken");
        }
    }

    async fn get_elf(
        db: &Arc<D>,
        job: &mut Job,
        metrics: &Arc<Metrics>,
        writer_tx: SyncSender<WriterMsg>,
    ) -> Result<ElfWithMeta, FailureReason> {
        match ivm_db::get_elf(db.clone(), &job.program_id).expect("DB reads cannot fail") {
            Some(elf) => Ok(elf),
            None => {
                // TODO: include job ID in metrics
                metrics.incr_job_err(&FailureReason::MissingElf.to_string());
                // Update the job status
                job.status = JobStatus {
                    status: JobStatusType::Failed as i32,
                    failure_reason: Some(FailureReason::MissingElf.to_string()),
                    retries: 0,
                };
                writer_tx.send((Write::JobTable(job.clone()), None)).expect("db writer broken");

                Err(FailureReason::DbErrMissingElf)
            }
        }
    }

    #[instrument(skip_all, level = "debug")]
    async fn execute_job(
        mut job: Job,
        zk_executor: &ZkvmExecutorService<S>,
        elf_with_meta: ElfWithMeta,
        metrics: &Arc<Metrics>,
        writer_tx: SyncSender<WriterMsg>,
    ) -> Result<Job, FailureReason> {
        let id = job.id;
        let result = match job.request_type {
            RequestType::Onchain => zk_executor
                .execute_onchain_job(
                    id,
                    job.max_cycles,
                    job.program_id.clone(),
                    job.onchain_input.clone(),
                    elf_with_meta.elf,
                    VmType::Risc0,
                )
                .await
                .map(|(result_with_metadata, signature)| (result_with_metadata, signature, None)),
            RequestType::Offchain(_) => {
                zk_executor
                    .execute_offchain_job(
                        id,
                        job.max_cycles,
                        job.program_id.clone(),
                        job.onchain_input.clone(),
                        job.offchain_input.clone(),
                        elf_with_meta.elf,
                        VmType::Risc0,
                    )
                    .await
            }
        };

        match result {
            Ok((result_with_metadata, zkvm_operator_signature, sidecar)) => {
                job.status = JobStatus {
                    status: JobStatusType::Done as i32,
                    failure_reason: None,
                    retries: 0,
                };
                job.result_with_metadata = result_with_metadata;
                job.zkvm_operator_signature = zkvm_operator_signature;
                job.blobs_sidecar = sidecar;

                writer_tx.send((Write::JobTable(job.clone()), None)).expect("db writer broken");

                Ok(job)
            }
            Err(e) => {
                // TODO: We need to relay failed results to make sure we can charge people
                // [ref: https://github.com/InfinityVM/InfinityVM/issues/78]
                error!("failed to execute job {:?}: {:?}", id, e);
                // TODO: record job ID.
                metrics.incr_job_err(&FailureReason::ExecErr.to_string());

                job.status = JobStatus {
                    status: JobStatusType::Failed as i32,
                    failure_reason: Some(FailureReason::ExecErr.to_string()),
                    retries: 0,
                };

                writer_tx.send((Write::JobTable(job.clone()), None)).expect("db writer broken");

                Err(FailureReason::ExecErr)
            }
        }
    }
}

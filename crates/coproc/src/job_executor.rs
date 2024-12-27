//! [`JobExecutor`] handles the execution of jobs.

use crate::{
    metrics::Metrics,
    writer::{Write, WriterMsg},
};
use alloy::{
    primitives::PrimitiveSignature,
    signers::{Signer, SignerSync},
};
use flume::{Receiver, Sender};
use ivm_db::tables::{ElfWithMeta, Job, RequestType};
use ivm_proto::{JobStatus, JobStatusType, VmType};
use ivm_zkvm_executor::service::ZkvmExecutorService;
use reth_db::Database;
use std::{marker::Send, sync::Arc};
use tokio::sync::oneshot;
use tracing::{error, instrument, info};

/// A message to the executor
pub type ExecutorMsg = (Job, oneshot::Sender<Job>);

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

/// Job executor service. This is essentially a thread pool for job execution.
#[derive(Debug)]
pub struct JobExecutor<S, D> {
    db: Arc<D>,
    executor_rx: Receiver<ExecutorMsg>,
    zk_executor: ZkvmExecutorService<S>,
    metrics: Arc<Metrics>,
    num_workers: usize,
    writer_tx: Sender<WriterMsg>,
}

impl<S, D> JobExecutor<S, D>
where
    S: Signer<PrimitiveSignature> + SignerSync<PrimitiveSignature> + Send + Sync + Clone + 'static,
    D: Database + 'static,
{
    /// Create a new instance of [Self].
    pub const fn new(
        db: Arc<D>,
        executor_rx: Receiver<ExecutorMsg>,
        zk_executor: ZkvmExecutorService<S>,
        metrics: Arc<Metrics>,
        num_workers: usize,
        writer_tx: Sender<WriterMsg>,
    ) -> Self {
        Self { db, executor_rx, zk_executor, metrics, num_workers, writer_tx }
    }

    /// Spawns `num_workers` worker tasks.
    pub async fn start(&self) {
        let mut threads = vec![];
        for _ in 0..self.num_workers {
            let executor_rx = self.executor_rx.clone();
            let db = Arc::clone(&self.db);
            let zk_executor = self.zk_executor.clone();
            let metrics = Arc::clone(&self.metrics);
            let writer_tx = self.writer_tx.clone();

            threads.push(std::thread::spawn(|| {
                Self::start_executor_worker(executor_rx, db, zk_executor, metrics, writer_tx)
            }));
        }
    }

    /// Start a worker.
    fn start_executor_worker(
        executor_rx: Receiver<ExecutorMsg>,
        db: Arc<D>,
        zk_executor: ZkvmExecutorService<S>,
        metrics: Arc<Metrics>,
        writer_tx: Sender<WriterMsg>,
    ) -> Result<(), Error> {
        let writer_tx2 = writer_tx;

        while let Ok((mut job, reply_tx)) = executor_rx.recv() {
            let elf_with_meta = match Self::get_elf(&db, &mut job, &metrics, writer_tx2.clone()) {
                Ok(elf) => elf,
                Err(_) => continue,
            };

            info!("executed received {}", job.nonce);
            let executed_job = match Self::execute_job(
                job,
                &zk_executor,
                elf_with_meta,
                &metrics,
                writer_tx2.clone(),
            ) {
                Ok(executed_job) => executed_job,
                Err(_) => continue,
            };

            info!("executed executed {}", executed_job.nonce);

            reply_tx.send(executed_job).expect("executor reply one shot channel is broken");
        }

        Err(Error::ExecQueueChannelClosed)
    }

    fn get_elf(
        db: &Arc<D>,
        job: &mut Job,
        metrics: &Arc<Metrics>,
        writer_tx: Sender<WriterMsg>,
    ) -> Result<ElfWithMeta, FailureReason> {
        match ivm_db::get_elf_sync(db.clone(), &job.program_id).expect("DB reads cannot fail") {
            Some(elf) => Ok(elf),
            None => {
                // TODO: include job ID in metrics
                // https://github.com/InfinityVM/InfinityVM/issues/362
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

    /// Returns a job updated with the execution result.
    ///
    /// As a side effect, this will write the updated job to the DB.
    #[instrument(skip_all, level = "debug")]
    fn execute_job(
        mut job: Job,
        zk_executor: &ZkvmExecutorService<S>,
        elf_with_meta: ElfWithMeta,
        metrics: &Arc<Metrics>,
        writer_tx: Sender<WriterMsg>,
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
                    VmType::Sp1,
                )
                .map(|(result_with_metadata, signature)| (result_with_metadata, signature, None)),
            RequestType::Offchain(_) => zk_executor.execute_offchain_job(
                id,
                job.max_cycles,
                job.program_id.clone(),
                job.onchain_input.clone(),
                job.offchain_input.clone(),
                elf_with_meta.elf,
                VmType::Sp1,
            ),
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

                writer_tx.send((Write::JobTable(job), None)).expect("db writer broken");

                Err(FailureReason::ExecErr)
            }
        }
    }
}

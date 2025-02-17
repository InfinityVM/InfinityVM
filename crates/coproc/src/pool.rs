//! [`Pool`] handles the execution of jobs. It does this by maintaining a pool of worker
//! threads that pull [`PoolMsg`]s off of the execution queue.

use crate::{
    metrics::Metrics,
    writer::{Write, WriterMsg},
};
use alloy::{
    hex,
    primitives::PrimitiveSignature,
    signers::{Signer, SignerSync},
};
use flume::Receiver;
use ivm_db::tables::{Job, ProgramWithMeta, RequestType};
use ivm_proto::{JobStatus, JobStatusType, VmType};
use ivm_zkvm_executor::service::ZkvmExecutorService;
use reth_db::Database;
use std::{marker::Send, sync::Arc};
use tokio::sync::oneshot;
use tracing::{error, instrument, warn};

/// A message to the executor
pub type PoolMsg = (Job, oneshot::Sender<Option<Job>>);

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
    #[error("missing_program")]
    MissingProgram,
    /// No ELF found in DB
    #[error("db_error_missing_elf")]
    DbErrMissingProgram,
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
pub struct Pool<S, D> {
    db: Arc<D>,
    pool_rx: Receiver<PoolMsg>,
    zk_executor: ZkvmExecutorService<S>,
    metrics: Arc<Metrics>,
    num_workers: usize,
    writer_tx: tokio::sync::mpsc::Sender<WriterMsg>,
}

impl<S, D> Pool<S, D>
where
    S: Signer<PrimitiveSignature> + SignerSync<PrimitiveSignature> + Send + Sync + Clone + 'static,
    D: Database + 'static,
{
    /// Create a new instance of [Self].
    pub const fn new(
        db: Arc<D>,
        pool_rx: Receiver<PoolMsg>,
        zk_executor: ZkvmExecutorService<S>,
        metrics: Arc<Metrics>,
        num_workers: usize,
        writer_tx: tokio::sync::mpsc::Sender<WriterMsg>,
    ) -> Self {
        Self { db, pool_rx, zk_executor, metrics, num_workers, writer_tx }
    }

    /// Spawns `num_workers` worker tasks.
    pub async fn start(&self) {
        let mut threads = vec![];
        for _ in 0..self.num_workers {
            let pool_rx = self.pool_rx.clone();
            let db = Arc::clone(&self.db);
            let zk_executor = self.zk_executor.clone();
            let metrics = Arc::clone(&self.metrics);
            let writer_tx = self.writer_tx.clone();

            threads.push(std::thread::spawn(|| {
                Self::start_executor_worker(pool_rx, db, zk_executor, metrics, writer_tx)
            }));
        }
    }

    /// Start a worker.
    fn start_executor_worker(
        pool_rx: Receiver<PoolMsg>,
        db: Arc<D>,
        zk_executor: ZkvmExecutorService<S>,
        metrics: Arc<Metrics>,
        writer_tx: tokio::sync::mpsc::Sender<WriterMsg>,
    ) -> Result<(), Error> {
        let writer_tx2 = writer_tx;

        while let Ok((mut job, reply_tx)) = pool_rx.recv() {
            // get_program will update the job status in case of failure
            let program_with_meta =
                match Self::get_program(&db, &mut job, &metrics, writer_tx2.clone()) {
                    Ok(elf) => elf,
                    Err(_) => {
                        reply_tx.send(None).expect("executor reply one shot channel is broken");
                        continue
                    }
                };

            let now = std::time::Instant::now();
            // execute_job will update the job status in case of failure
            let executed_job = match Self::execute_job(
                job,
                &zk_executor,
                program_with_meta,
                &metrics,
                writer_tx2.clone(),
            ) {
                Ok(executed_job) => executed_job,
                Err(_) => {
                    reply_tx.send(None).expect("executor reply one shot channel is broken");
                    continue
                }
            };
            metrics.observe_job_exec_time(now.elapsed());

            reply_tx.send(Some(executed_job)).expect("executor reply one shot channel is broken");
        }

        Err(Error::ExecQueueChannelClosed)
    }

    fn get_program(
        db: &Arc<D>,
        job: &mut Job,
        metrics: &Arc<Metrics>,
        writer_tx: tokio::sync::mpsc::Sender<WriterMsg>,
    ) -> Result<ProgramWithMeta, FailureReason> {
        match ivm_db::get_program_sync(db.clone(), &job.program_id).expect("DB reads cannot fail") {
            Some(elf) => Ok(elf),
            None => {
                warn!(
                    nonce = job.nonce,
                    consumer = hex::encode(&job.consumer_address),
                    program_id = hex::encode(&job.program_id),
                    "missing program",
                );
                // TODO: include job ID in metrics
                // https://github.com/InfinityVM/InfinityVM/issues/362
                metrics.incr_job_err(&FailureReason::MissingProgram.to_string());
                // Update the job status
                job.status = JobStatus {
                    status: JobStatusType::Failed as i32,
                    failure_reason: Some(FailureReason::MissingProgram.to_string()),
                    retries: 0,
                };
                writer_tx
                    .blocking_send((Write::JobTable(job.clone()), None))
                    .expect("db writer broken");

                Err(FailureReason::DbErrMissingProgram)
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
        program_with_meta: ProgramWithMeta,
        metrics: &Arc<Metrics>,
        writer_tx: tokio::sync::mpsc::Sender<WriterMsg>,
    ) -> Result<Job, FailureReason> {
        let id = job.id;
        let result = match job.request_type {
            RequestType::Onchain => zk_executor
                .execute_onchain_job(
                    id,
                    job.max_cycles,
                    job.program_id.clone(),
                    job.onchain_input.clone(),
                    program_with_meta.program_bytes,
                    VmType::Sp1,
                )
                .map(|(result_with_metadata, signature)| (result_with_metadata, signature, None)),
            RequestType::Offchain(_) => zk_executor.execute_offchain_job(
                id,
                job.max_cycles,
                job.program_id.clone(),
                job.onchain_input.clone(),
                job.offchain_input.clone(),
                program_with_meta.program_bytes,
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

                writer_tx
                    .blocking_send((Write::JobTable(job.clone()), None))
                    .expect("db writer broken");

                Ok(job)
            }
            Err(error) => {
                // TODO: We need to relay failed results to make sure we can charge people
                // [ref: https://github.com/InfinityVM/InfinityVM/issues/78]
                warn!(
                    nonce = job.nonce,
                    consumer = hex::encode(&job.consumer_address),
                    program_id = hex::encode(&job.program_id),
                    ?error,
                    "failed job execution",
                );
                // TODO: record job ID.
                metrics.incr_job_err(&FailureReason::ExecErr.to_string());

                job.status = JobStatus {
                    status: JobStatusType::Failed as i32,
                    failure_reason: Some(FailureReason::ExecErr.to_string()),
                    retries: 0,
                };

                writer_tx.blocking_send((Write::JobTable(job), None)).expect("db writer broken");

                Err(FailureReason::ExecErr)
            }
        }
    }
}

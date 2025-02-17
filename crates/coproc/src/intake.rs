//! Handlers for submitting new jobs and programs.
//!
//! For new programs we check that they do not already exist and then persist.
//!
//! For new jobs we check that the job does not exist, persist it and send it to the execution actor
//! associated with the consumer.

use crate::execute::ExecMsg;

use crate::{
    execute::ExecutionActorSpawner,
    writer::{Write, WriterMsg},
};
use alloy::{
    hex,
    primitives::PrimitiveSignature,
    providers::ProviderBuilder,
    signers::{Signer, SignerSync},
    transports::http::reqwest::Url,
};
use dashmap::{DashMap, Entry};
use ivm_db::{get_elf, get_elf_sync, get_job, tables::Job};
use ivm_proto::{JobStatus, JobStatusType, VmType};
use ivm_zkvm_executor::service::ZkvmExecutorService;
use reth_db::Database;
use std::sync::Arc;
use tokio::sync::{mpsc::Sender, oneshot};
use tracing::info;

/// Errors from job processor
#[derive(thiserror::Error, Debug)]
pub enum Error {
    /// Could not create ELF in zkvm executor
    #[error("failed to create ELF in zkvm executor: {0}")]
    CreateElfFailed(#[from] ivm_zkvm_executor::service::Error),
    /// database error
    #[error("database error: {0}")]
    Database(#[from] ivm_db::Error),
    /// ELF with given program ID already exists in DB
    #[error("elf with program ID {0} already exists")]
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
    /// given program ID does not match derived program ID
    #[error("given program ID does not match derived program ID: given: {0}, derived: {1}")]
    MismatchProgramId(String, String),
    /// failed while trying to read the nonce from the consumer contract
    #[error("could not read the nonce from the consumer contract {0}")]
    GetNonceFail(#[from] alloy::contract::Error),
    /// job is marked as failed and it might be in the DLQ
    #[error("job already exists and failed to get relayed - may be in DLQ")]
    JobExistsAndFailedToRelay,
    /// job has already been executed and relayed
    #[error("job has already been executed and relayed")]
    JobRelayed,
    /// nonce is too low for an ordered job
    #[error("nonce is too low for an ordered job")]
    LowNonce,
}

/// Job and program intake handlers.
///
/// New, valid jobs submitted to this service will be sent over the exec queue to the job processor.
#[derive(Debug)]
pub struct IntakeHandlers<S, D> {
    db: Arc<D>,
    zk_executor: ZkvmExecutorService<S>,
    max_da_per_job: usize,
    writer_tx: Sender<WriterMsg>,
    unsafe_skip_program_id_check: bool,
    execution_actor_spawner: ExecutionActorSpawner,
    active_actors: Arc<DashMap<[u8; 20], Sender<ExecMsg>>>,
    http_eth_rpc: Url,
}

impl<S, D> Clone for IntakeHandlers<S, D>
where
    S: Clone,
{
    fn clone(&self) -> Self {
        Self {
            db: self.db.clone(),
            zk_executor: self.zk_executor.clone(),
            max_da_per_job: self.max_da_per_job,
            writer_tx: self.writer_tx.clone(),
            unsafe_skip_program_id_check: self.unsafe_skip_program_id_check,
            execution_actor_spawner: self.execution_actor_spawner.clone(),
            active_actors: self.active_actors.clone(),
            http_eth_rpc: self.http_eth_rpc.clone(),
        }
    }
}

impl<S, D> IntakeHandlers<S, D>
where
    S: Signer<PrimitiveSignature> + SignerSync<PrimitiveSignature> + Send + Sync + Clone + 'static,
    D: Database + 'static,
{
    /// Create an instance of [Self].
    pub fn new(
        db: Arc<D>,
        zk_executor: ZkvmExecutorService<S>,
        max_da_per_job: usize,
        writer_tx: Sender<WriterMsg>,
        unsafe_skip_program_id_check: bool,
        execution_actor_spawner: ExecutionActorSpawner,
        http_eth_rpc: Url,
    ) -> Self {
        Self {
            db,
            zk_executor,
            max_da_per_job,
            writer_tx,
            unsafe_skip_program_id_check,
            execution_actor_spawner,
            active_actors: Default::default(),
            http_eth_rpc,
        }
    }

    /// Submits job, saves it in DB, and pushes on the exec queue.
    ///
    /// Caution: this assumes the consumer address has already been validated to be exactly 20
    /// bytes. For the gRPC service, we do this in the `submit_job` endpoint implementation before
    /// call this method.
    pub async fn submit_job(&self, mut job: Job) -> Result<(), Error> {
        {
            // We compress blob data before submitting it, so we want to make sure we are validating
            // compressed size
            let compressed = lz4_flex::block::compress_prepend_size(&job.offchain_input);
            if compressed.len() > self.max_da_per_job {
                return Err(Error::OffchainInputOverMaxDAPerJob);
            };
        }

        let consumer_address: [u8; 20] =
            job.consumer_address.clone().try_into().expect("caller must validate address length.");

        // TODO: add new table for just job ID so we can avoid writing full job here and reading.
        // We can just pass the job itself along the channel
        // full job https://github.com/InfinityVM/InfinityVM/issues/354
        // NOTE: there is a race condition here where multiple jobs with the same nonce
        // get submitted before the DB write completes. In that case the last submission
        // will take precedence.
        if let Some(old_job) = get_job(self.db.clone(), job.id).await? {
            if old_job.is_failed() {
                return Err(Error::JobExistsAndFailedToRelay);
            }
            if old_job.is_relayed() {
                return Err(Error::JobRelayed);
            }
            if self.get_pending_nonces(consumer_address).await?.contains(&job.nonce) {
                return Ok(())
            }

            // We overwrite the old job. This allows users to submit new jobs in case the original
            // job had an issue
            if old_job != job {
                self.write_pending_job(&mut job).await;
            }
        } else {
            self.write_pending_job(&mut job).await;
        };

        // We do an optimistic check to reduce entry lock contention.
        let execution_tx = if let Some(inner) = self.active_actors.get(&consumer_address) {
            inner.clone()
        } else {
            let provider = ProviderBuilder::new().on_http(self.http_eth_rpc.clone());
            let consumer =
                ivm_contracts::consumer::Consumer::new(consumer_address.into(), provider);
            // This is the next nonce; since we expect contracts to be initialized with
            // nonce 0, the first nonce should be 1.
            let nonce = consumer.getNextNonce().call().await?._0;

            match self.active_actors.entry(consumer_address) {
                Entry::Occupied(e) => e.get().clone(),
                Entry::Vacant(e) => {
                    info!(?nonce, ?consumer_address, "starting execution actor");
                    // ATTENTION: No async allowed while an entry lock is held.
                    let execution_tx = self.execution_actor_spawner.spawn(nonce);
                    e.insert(execution_tx.clone());
                    execution_tx
                }
            }
        };

        if job.is_ordered() {
            let (tx, next_nonce_rx) = oneshot::channel();
            execution_tx.send(ExecMsg::NextNonce(tx)).await.expect("execution tx failed");
            let next_nonce = next_nonce_rx.await.expect("next nonce rx failed");
            if job.nonce < next_nonce {
                job.status = JobStatus {
                    status: JobStatusType::Failed as i32,
                    failure_reason: Some("low nonce".to_string()),
                    retries: 0,
                };

                self.writer_tx
                    .send((Write::JobTable(job.clone()), None))
                    .await
                    .expect("db writer broken");

                return Err(Error::LowNonce)
            }
        }

        // Send the job to actor for processing
        // NOTE: Once actor deletion is implemented, we need to avoid a
        // race between sending the job & deleting the actor.
        execution_tx.send(ExecMsg::Exec(job)).await.expect("execution tx failed");

        Ok(())
    }

    async fn write_pending_job(&self, job: &mut Job) {
        job.status =
            JobStatus { status: JobStatusType::Pending as i32, failure_reason: None, retries: 0 };
        let (tx, db_write_complete_rx) = oneshot::channel();
        self.writer_tx
            .send((Write::JobTable(job.clone()), Some(tx)))
            .await
            .expect("db writer broken");

        // Before responding, make sure the write completes
        let _ = db_write_complete_rx.await;
    }

    /// Submit program ELF, save it in DB, and return program ID.
    pub fn submit_elf(
        &self,
        elf: Vec<u8>,
        vm_type: i32,
        program_id: Vec<u8>,
    ) -> Result<Vec<u8>, Error> {
        let vm_type = VmType::try_from(vm_type).map_err(|_| Error::InvalidVmType)?;

        if get_elf_sync(self.db.clone(), &program_id)
            .map_err(|e| Error::ElfReadFailed(e.to_string()))?
            .is_some()
        {
            return Err(Error::ElfAlreadyExists(hex::encode(program_id.as_slice())));
        }

        // Do expensive check second
        if !self.unsafe_skip_program_id_check {
            let derived_program_id = self.zk_executor.create_elf(&elf, vm_type)?;
            if program_id != derived_program_id {
                return Err(Error::MismatchProgramId(
                    hex::encode(&program_id),
                    hex::encode(&derived_program_id),
                ));
            }
        };

        // Write the elf and make sure it completes before responding to the user.
        self.writer_tx
            .blocking_send((
                Write::Elf { vm_type, program_id: program_id.clone(), elf: elf.clone() },
                None,
            ))
            .expect("writer channel broken");

        let program_bytes = self.zk_executor.create_program(&elf, vm_type)?;
        let (tx, program_rx) = oneshot::channel();
        self.writer_tx
            .blocking_send((
                Write::Program { vm_type, program_id: program_id.clone(), program_bytes },
                Some(tx),
            ))
            .expect("writer channel broken");

        // Program write will complete after the elf, so we wait on that before
        // returning to the user.
        program_rx.blocking_recv().expect("writer responder is broken");

        Ok(program_id)
    }

    /// Returns job with `job_id` from DB
    #[inline(always)]
    pub async fn get_job(&self, job_id: [u8; 32]) -> Result<Option<Job>, Error> {
        let job = get_job(self.db.clone(), job_id).await?;
        Ok(job)
    }

    /// Returns the nonces of jobs that are in the execution and relay pipeline, but are not
    /// yet on chain.
    pub async fn get_pending_nonces(&self, consumer_address: [u8; 20]) -> Result<Vec<u64>, Error> {
        // First see if we even have an execution actor associated with this consumer
        let execution_tx = if let Some(inner) = self.active_actors.get(&consumer_address) {
            inner.clone()
        } else {
            return Ok(Vec::new())
        };

        let (tx, rx) = oneshot::channel();
        execution_tx.send(ExecMsg::Pending(tx)).await.expect("execution tx failed");

        let pending_jobs = rx.await.expect("one shot sender receiver failed");
        Ok(pending_jobs)
    }

    /// Returns true if the elf for the given program ID exists
    pub async fn contains_elf(&self, program_id: Vec<u8>) -> Result<bool, Error> {
        let contains = get_elf(self.db.clone(), program_id)
            .await
            .map_err(|e| Error::ElfReadFailed(e.to_string()))?
            .is_some();
        Ok(contains)
    }
}

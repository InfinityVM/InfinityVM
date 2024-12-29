//! Handlers for submitting new jobs and programs.
//!
//! For new programs we check that they do not already exist and then persist.
//!
//! For new jobs we check that the job does not exist, persist it and send it to the execution actor
//! associated with the consumer.

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
use dashmap::DashMap;
use ivm_db::{get_elf_sync, get_job, tables::Job};
use ivm_proto::{JobStatus, JobStatusType, VmType};
use ivm_zkvm_executor::service::ZkvmExecutorService;
use reth_db::Database;
use std::sync::Arc;
use tokio::sync::{mpsc::Sender, oneshot};

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
}

/// Job and program intake handlers.
///
/// New, valid jobs submitted to this service will be sent over the exec queue to the job processor.
#[derive(Debug)]
pub struct IntakeHandlers<S, D> {
    db: Arc<D>,
    zk_executor: ZkvmExecutorService<S>,
    max_da_per_job: usize,
    writer_tx: flume::Sender<WriterMsg>,
    unsafe_skip_program_id_check: bool,
    execution_actor_spawner: ExecutionActorSpawner,
    active_actors: Arc<DashMap<[u8; 20], Sender<Job>>>,
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
        writer_tx: flume::Sender<WriterMsg>,
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
            active_actors: Arc::new(DashMap::new()),
            http_eth_rpc,
        }
    }

    /// Submits job, saves it in DB, and pushes on the exec queue.
    ///
    /// Caution: this assumes the consumer address has already been validated to be exactly 20
    /// bytes.
    pub async fn submit_job(&self, mut job: Job) -> Result<(), Error> {
        if job.offchain_input.len() > self.max_da_per_job {
            return Err(Error::OffchainInputOverMaxDAPerJob);
        };

        // TODO: add new table for just job ID so we can avoid writing full job here and reading.
        // We can just pass the job itself along the channel
        // full job https://github.com/InfinityVM/InfinityVM/issues/354
        if get_job(self.db.clone(), job.id).await?.is_some() {
            return Err(Error::JobAlreadyExists);
        }

        job.status =
            JobStatus { status: JobStatusType::Pending as i32, failure_reason: None, retries: 0 };
        let (tx, db_write_complete_rx) = oneshot::channel();
        self.writer_tx
            .send_async((Write::JobTable(job.clone()), Some(tx)))
            .await
            .expect("db writer broken");

        let consumer_address: [u8; 20] =
            job.consumer_address.clone().try_into().expect("caller must validate address length.");

        let execution_tx = if !self.active_actors.contains_key(&consumer_address) {
            // Get the latest nonce to initialize the execution actor with
            let provider = ProviderBuilder::new().on_http(self.http_eth_rpc.clone());
            let consumer =
                ivm_contracts::consumer::Consumer::new(consumer_address.into(), provider);
            // This is the next nonce; since we expect contracts to be initialized with nonce 0, the
            // first nonce should be 1.
            let nonce = consumer.getNextNonce().call().await?._0;

            let execution_tx = self.execution_actor_spawner.spawn(nonce);
            self.active_actors.insert(consumer_address, execution_tx.clone());
            execution_tx
        } else {
            self.active_actors
                .get(&consumer_address)
                .expect("we checked above that the entry exists. qed.")
                .clone()
        };

        // Send the job to actor for processing
        execution_tx.send(job).await.expect("execution tx failed");

        // Before responding, make sure the write completes
        let _ = db_write_complete_rx.await;

        Ok(())
    }

    /// Submit program ELF, save it in DB, and return program ID.
    pub fn submit_elf(
        &self,
        elf: Vec<u8>,
        vm_type: i32,
        program_id: Vec<u8>,
    ) -> Result<Vec<u8>, Error> {
        let vm_type = VmType::try_from(vm_type).map_err(|_| Error::InvalidVmType)?;

        if !self.unsafe_skip_program_id_check {
            let derived_program_id = self.zk_executor.create_elf(&elf, vm_type)?;
            if program_id != derived_program_id {
                return Err(Error::MismatchProgramId(
                    hex::encode(&program_id),
                    hex::encode(&derived_program_id),
                ));
            }
        };

        if get_elf_sync(self.db.clone(), &program_id)
            .map_err(|e| Error::ElfReadFailed(e.to_string()))?
            .is_some()
        {
            return Err(Error::ElfAlreadyExists(hex::encode(program_id.as_slice())));
        }

        // Write the elf and make sure it completes before responding to the user.
        let (tx, rx) = oneshot::channel();
        self.writer_tx
            .send((Write::Elf { vm_type, program_id: program_id.clone(), elf }, Some(tx)))
            .expect("writer channel broken");
        let _ = rx.blocking_recv();

        Ok(program_id)
    }

    /// Returns job with `job_id` from DB
    #[inline(always)]
    pub async fn get_job(&self, job_id: [u8; 32]) -> Result<Option<Job>, Error> {
        let job = get_job(self.db.clone(), job_id).await?;
        Ok(job)
    }
}

//! Handlers for submitting new jobs and programs.
//!
//! For new programs we check that they do not already exist and then persist.
//!
//! For new jobs we check that the job does not exist, persist it and push it onto the exec queue.

use crate::{
    relayer::Relay,
    elf_store::RemoteElfClientTrait,
    writer::{Write, WriterMsg},
};
use alloy::{
    hex,
    primitives::PrimitiveSignature,
    signers::{Signer, SignerSync},
};
use flume::Sender;
use ivm_db::{
    get_elf, get_job,
    tables::{ElfWithMeta, Job},
};
use ivm_proto::{JobStatus, JobStatusType, RelayStrategy, VmType};
use ivm_zkvm_executor::service::ZkvmExecutorService;
use reth_db::Database;
use std::sync::Arc;
use tokio::sync::oneshot;

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
    /// Remote DB error
    #[error("remote db error: {0}")]
    RemoteDb(#[from] tonic::transport::Error),
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
}

/// Configuration for `IntakeHandlers`
#[derive(Debug, Clone)]
pub struct IntakeConfig {
    /// Maximum bytes of DA allowed per job
    pub max_da_per_job: usize,
    /// Channel for sending write operations to the database
    pub writer_tx: Sender<WriterMsg>,
    /// Channel for sending relay operations
    pub relay_tx: Sender<Relay>,
    /// Whether to skip program ID verification (WARNING: should be false in production)
    pub unsafe_skip_program_id_check: bool,
    /// General configuration for the coprocessor node
    pub config: crate::config::Config,
}

/// Job and program intake handlers.
///
/// New, valid jobs submitted to this service will be sent over the exec queue to the job processor.
#[derive(Debug)]
pub struct IntakeHandlers<S, D> {
    db: Arc<D>,
    exec_queue_sender: Sender<Job>,
    zk_executor: ZkvmExecutorService<S>,
    config: IntakeConfig,
}

impl<S, D> Clone for IntakeHandlers<S, D>
where
    S: Clone,
{
    fn clone(&self) -> Self {
        Self {
            db: self.db.clone(),
            exec_queue_sender: self.exec_queue_sender.clone(),
            zk_executor: self.zk_executor.clone(),
            config: self.config.clone(),
        }
    }
}

impl<S, D> IntakeHandlers<S, D>
where
    S: Signer<PrimitiveSignature> + SignerSync<PrimitiveSignature> + Send + Sync + Clone + 'static,
    D: Database + 'static,
{
    /// Create an instance of [Self].
    pub const fn new(
        db: Arc<D>,
        exec_queue_sender: Sender<Job>,
        zk_executor: ZkvmExecutorService<S>,
        config: IntakeConfig,
    ) -> Self {
        Self { db, exec_queue_sender, zk_executor, config }
    }

    /// Submits job, saves it in DB, and pushes on the exec queue.
    pub async fn submit_job(&self, mut job: Job) -> Result<(), Error> {
        if job.offchain_input.len() > self.config.max_da_per_job {
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
        self.config
            .writer_tx
            .send_async((Write::JobTable(job.clone()), Some(tx)))
            .await
            .expect("db writer broken");

        if job.relay_strategy == RelayStrategy::Ordered {
            let consumer = job
                .consumer_address
                .clone()
                .try_into()
                .expect("we checked for valid address length");
            self.config
                .relay_tx
                .send_async(Relay::Queue { consumer, job_id: job.id })
                .await
                .expect("relay channel broken");
        };

        self.exec_queue_sender.send_async(job).await.map_err(|_| Error::ExecQueueSendFailed)?;
        let _ = db_write_complete_rx.await;

        Ok(())
    }

    /// Submit program ELF, save it in DB, and return program ID.
    pub async fn submit_elf(
        &self,
        elf: Vec<u8>,
        vm_type: i32,
        program_id: Vec<u8>,
    ) -> Result<Vec<u8>, Error> {
        let vm_type = VmType::try_from(vm_type).map_err(|_| Error::InvalidVmType)?;

        if !self.config.unsafe_skip_program_id_check {
            let derived_program_id = self.zk_executor.create_elf(&elf, vm_type)?;
            if program_id != derived_program_id {
                return Err(Error::MismatchProgramId(
                    hex::encode(&program_id),
                    hex::encode(&derived_program_id),
                ));
            }
        };

        if get_elf(self.db.clone(), &program_id)
            .map_err(|e| Error::ElfReadFailed(e.to_string()))?
            .is_some()
        {
            return Err(Error::ElfAlreadyExists(hex::encode(program_id.as_slice())));
        }

        // Write the elf to embedded DB and make sure it completes before continuing
        let (tx, rx) = oneshot::channel();
        self.config
            .writer_tx
            .send((
                Write::Elf { vm_type, program_id: program_id.clone(), elf: elf.clone() },
                Some(tx),
            ))
            .expect("writer channel broken");
        let _ = rx.await;

        // Store the ELF in remote DB
        match crate::elf_store::RemoteElfClient::connect(&self.config.config.elf_store.endpoint)
            .await
        {
            Ok(mut client) => {
                if let Err(e) = client
                    .store_elf(program_id.clone(), ElfWithMeta { vm_type: vm_type as u8, elf })
                    .await
                {
                    tracing::warn!("Failed to store ELF in remote DB: {}", e);
                }
            }
            Err(e) => {
                tracing::warn!("Failed to connect to remote DB: {}", e);
            }
        }

        Ok(program_id)
    }

    /// Returns job with `job_id` from DB
    pub async fn get_job(&self, job_id: [u8; 32]) -> Result<Option<Job>, Error> {
        let job = get_job(self.db.clone(), job_id)?;
        Ok(job)
    }
}

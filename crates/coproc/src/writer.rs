//! Synchronous database writer. All writes DB writes should go through this.

use ivm_db::tables::{
    B256Key, ElfTable, ElfWithMeta, Job, JobTable, LastBlockHeight, RelayFailureJobs, Sha256Key,
};
use ivm_proto::VmType;
use reth_db::{
    transaction::{DbTx, DbTxMut},
    Database, DatabaseError,
};
use std::sync::Arc;
use tokio::sync::{mpsc::Receiver, oneshot};

// These numbers are arbitrary.
const WRITE_BUFFER_SIZE: usize = 256;
const LAST_WRITE_BUFFER_SIZE: usize = 10_240;

/// A write request to the [`Writer`]. If a sender is included, the writer
/// will respond once the write has been completed.
pub type WriterMsg = (Write, Option<oneshot::Sender<()>>);

/// Job write module errors
#[derive(thiserror::Error, Debug)]
pub enum Error {
    /// channel receiver broken
    #[error("job write receiver errored")]
    JobWriteReceiver,
    /// reth-mdbx lib error
    #[error("mdbx (database): {0}")]
    GenericRethMdbx(#[from] eyre::Report),
    /// reth mdbx database backend error
    #[error("mdbx (database): {0}")]
    RethMdbx(#[from] reth_db::mdbx::Error),
    /// reth database error
    #[error("reth database: {0}")]
    RethDbError(#[from] DatabaseError),
}

/// Write targets.
#[derive(Debug)]
pub enum Write {
    /// Write to relay failure jobs table
    FailureJobs(Job),
    /// Write to jobs table
    JobTable(Job),
    /// Delete a job from the relay failure jobs table.
    FailureJobsDelete([u8; 32]),
    /// Set the last block where events where processed.
    LastBlockHeight(u64),
    /// Write an ELF to the ELF table.
    Elf {
        /// Type of VM the elf targets.
        vm_type: VmType,
        /// Verifying key associated with the ELF.
        program_id: Vec<u8>,
        /// ELF.
        elf: Vec<u8>,
    },
    /// Kill this thread
    Kill,
}

/// All job writes go through this writer.
#[derive(Debug)]
pub struct Writer<D> {
    db: Arc<D>,
    rx: Receiver<WriterMsg>,
}

impl<D> Writer<D>
where
    D: Database + 'static,
{
    /// Create a new instance of [`Self`]
    pub const fn new(db: Arc<D>, rx: Receiver<WriterMsg>) -> Self {
        Self { db, rx }
    }

    /// Start the job writer.
    pub fn start_blocking(self) -> Result<(), Error> {
        let Self { mut rx, db } = self;
        let mut buffer = Vec::with_capacity(LAST_WRITE_BUFFER_SIZE);

        while rx.blocking_recv_many(&mut buffer, WRITE_BUFFER_SIZE) > 0 {
            let kill_seen = Self::write_batch(&db, &mut buffer)?;
            if kill_seen {
                break
            }
        }

        rx.blocking_recv_many(&mut buffer, LAST_WRITE_BUFFER_SIZE);
        Self::write_batch(&db, &mut buffer)?;

        Ok(())
    }

    /// Returns true if the batch contained a kill message.
    /// NOTE: drains `batch`.
    #[inline]
    fn write_batch(db: &Arc<D>, batch: &mut Vec<WriterMsg>) -> Result<bool, Error> {
        let mut kill_seen = false;
        let tx = db.tx_mut()?;

        for (target, resp) in batch.drain(..) {
            match target {
                Write::JobTable(job) => tx.put::<JobTable>(B256Key(job.id), job)?,
                Write::FailureJobs(job) => tx.put::<RelayFailureJobs>(B256Key(job.id), job)?,
                Write::FailureJobsDelete(job_id) => {
                    tx.delete::<RelayFailureJobs>(B256Key(job_id), None).map(|_| ())?
                }
                Write::LastBlockHeight(height) => {
                    tx.put::<LastBlockHeight>(ivm_db::LAST_HEIGHT_KEY, height)?
                }
                Write::Elf { vm_type, program_id, elf } => {
                    let elf_with_meta = ElfWithMeta { vm_type: vm_type as u8, elf };
                    let key = Sha256Key::new(&program_id);
                    tx.put::<ElfTable>(key, elf_with_meta)?;
                }
                Write::Kill => kill_seen = true,
            };

            if let Some(resp) = resp {
                let _ = resp.send(());
            }
        }
        tx.commit()?;

        Ok(kill_seen)
    }
}

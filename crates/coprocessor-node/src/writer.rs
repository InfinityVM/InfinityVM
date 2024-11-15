//! Synchronous database writer. All writes DB writes should go through this.

use ivm_db::tables::{B256Key, Job, JobTable, LastBlockHeight, RelayFailureJobs};
use reth_db::{
    transaction::{DbTx, DbTxMut},
    Database, DatabaseError,
};
use std::sync::{mpsc, Arc};
use tokio::sync::oneshot;

/// A write request to the [`Writer`].
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

/// Table to write job too
#[derive(Debug)]
pub enum Write {
    /// Write to relay failure jobs table
    FailureJobs(Job),
    /// Write to jobs table
    JobTable(Job),
    /// Delete a job from the relay failure jobs table.
    FailureJobsDelete([u8; 32]),
    /// Set the last block where events where processed
    LastBlockHeight(u64),
    /// Kill this thread
    Kill,
}

/// All job writes go through this writer.
#[derive(Debug)]
pub struct Writer<D> {
    db: Arc<D>,
    rx: mpsc::Receiver<WriterMsg>,
}

impl<D> Writer<D>
where
    D: Database + 'static,
{
    /// Create a new instance of [`Self`]
    pub fn new(db: Arc<D>, rx: mpsc::Receiver<WriterMsg>) -> Self {
        Self { db, rx }
    }

    /// Start the job writer.
    pub fn start_blocking(self) -> Result<(), Error> {
        while let Ok((target, resp)) = self.rx.recv() {
            let tx = self.db.tx_mut()?;
            match target {
                Write::JobTable(job) => tx.put::<JobTable>(B256Key(job.id), job)?,
                Write::FailureJobs(job) => tx.put::<RelayFailureJobs>(B256Key(job.id), job)?,
                Write::FailureJobsDelete(job_id) => {
                    tx.delete::<RelayFailureJobs>(B256Key(job_id), None).map(|_| ())?
                }
                Write::LastBlockHeight(height) => {
                    tx.put::<LastBlockHeight>(ivm_db::LAST_HEIGHT_KEY, height)?
                }
                // TODO: flush receiver before exiting
                Write::Kill => return Ok(()),
            };
            tx.commit()?;

            if let Some(resp) = resp {
                let _ = resp.send(());
            }
        }

        Err(Error::JobWriteReceiver)
    }
}

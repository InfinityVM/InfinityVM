//! Synchronous database writer.

use std::sync::{mpsc, Arc};

use ivm_db::{delete_fail_relay_job, put_fail_relay_job, put_job, tables::Job};
use reth_db::Database;
use tokio::sync::oneshot;

/// A write request to the [`Writer`].
pub type WriterMsg = (WriteTarget, Option<oneshot::Sender<()>>);

/// Job write module errors
#[derive(thiserror::Error, Debug)]
pub enum Error {
    /// Channel receiver broken
    #[error("job write receiver errored")]
    JobWriteReceiver,
    /// DB error
    #[error("db: {0}")]
    Db(#[from] ivm_db::Error),
}

/// Table to write job too
#[derive(Debug)]
pub enum WriteTarget {
    /// Write to relay failure jobs table
    FailureJobs(Job),
    /// Write to jobs table
    JobTable(Job),
    /// Delete a job from the relay failure jobs table.
    FailureJobsDelete([u8; 32]),
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
            // TODO: don't clone db
            match target {
                WriteTarget::JobTable(job) => put_job(self.db.clone(), job)?,
                WriteTarget::FailureJobs(job) => put_fail_relay_job(self.db.clone(), job)?,
                WriteTarget::FailureJobsDelete(job_id) => {
                    delete_fail_relay_job(self.db.clone(), job_id).map(|_| ())?
                }
                WriteTarget::Kill => return Ok(()),
            };
            if let Some(resp) = resp {
                let _ = resp.send(());
            }
        }

        Err(Error::JobWriteReceiver)
    }
}

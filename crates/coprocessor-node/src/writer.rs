use std::sync::{mpsc, Arc};

use ivm_db::{put_fail_relay_job, put_job, tables::{Job, JobTable}};
use reth_db::Database;
use tokio::sync::oneshot;

pub type JobWriterMsg = (Job, JobWriteTarget, oneshot::Sender<()>);

/// Job write module errors
#[derive(thiserror::Error, Debug)]
enum Error {
   #[error("job write receiver errored")]
  JobWriteReceiver,
  #[error("db: {0}")]
  Db(#[from] ivm_db::Error)
}

/// Table to write job too
#[derive(Debug)]
pub enum JobWriteTarget {
  RelayFailureJobs,
  JobTable
}

/// All job writes go through this writer.
pub struct JobWriter<D> {
  db: Arc<D>,
  rx: mpsc::Receiver<JobWriterMsg>
}


impl<D> JobWriter<D>
where 
    D: Database + 'static,
{
  pub fn new(db: Arc<D>,  rx: mpsc::Receiver<JobWriterMsg>) -> Self {
    Self { db, rx }
  }

  pub fn start(self) -> Result<(), Error> {
      while let Ok((job, target, resp)) = self.rx.recv() {
          match target {
            JobWriteTarget::JobTable => put_job(self.db.clone(), job)?,
            JobWriteTarget::RelayFailureJobs => put_fail_relay_job(self.db.clone(), job)?,
          }
          let _ = resp.send(());
      }

      Err(Error::JobWriteReceiver)
  }

}
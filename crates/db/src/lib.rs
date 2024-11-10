//! Coprocessor node database.

use ivm_proto::VmType;
use reth_db::{
    create_db,
    mdbx::{DatabaseArguments, DatabaseFlags},
    models::ClientVersion,
    transaction::{DbTx, DbTxMut},
    Database, DatabaseEnv, DatabaseError, TableType,
};
use reth_db_api::cursor::DbCursorRO;
use std::{ops::Deref, path::Path, sync::Arc};
use tables::{
    AddrKey, B256Key, ElfTable, ElfWithMeta, Job, JobTable, LastBlockHeight, QueueMetaTable,
    QueueNodeTable, RelayFailureJobs, Sha256Key,
};

pub mod tables;

const LAST_HEIGHT_KEY: u32 = 0;

/// DB module errors
#[derive(thiserror::Error, Debug)]
pub enum Error {
    /// Error from reth-mdbx lib
    #[error("mdbx (database): {0}")]
    GenericRethMdbx(#[from] eyre::Report),

    /// Reth mdbx database backend error
    #[error("mdbx (database): {0}")]
    RethMdbx(#[from] reth_db::mdbx::Error),

    /// Reth database error
    #[error("reth database: {0}")]
    RethDbError(#[from] DatabaseError),

    /// invalid address length
    #[error("invalid address length")]
    InvalidAddressLength,
}

/// Write an ELF file to the database
pub fn put_elf<D: Database>(
    db: Arc<D>,
    vm_type: VmType,
    program_id: &[u8],
    elf: Vec<u8>,
) -> Result<(), Error> {
    let elf_with_meta = ElfWithMeta { vm_type: vm_type as u8, elf };
    let key = Sha256Key::new(program_id);

    db.update(|tx| tx.put::<ElfTable>(key, elf_with_meta))?.map_err(Into::into)
}

/// Read in an ELF file from the database. [None] if it does not exist
pub fn get_elf<D: Database>(db: Arc<D>, program_id: &[u8]) -> Result<Option<ElfWithMeta>, Error> {
    db.view(|tx| tx.get::<ElfTable>(Sha256Key::new(program_id)))?.map_err(Into::into)
}

/// Write a job to the database
pub fn put_job<D: Database>(db: Arc<D>, job: Job) -> Result<(), Error> {
    db.update(|tx| tx.put::<JobTable>(B256Key(job.id), job))?.map_err(Into::into)
}

/// Read in an Job from the database. [None] if it does not exist
pub fn get_job<D: Database>(db: Arc<D>, job_id: [u8; 32]) -> Result<Option<Job>, Error> {
    db.view(|tx| tx.get::<JobTable>(B256Key(job_id)))?.map_err(Into::into)
}

/// Write last block height to the database
pub fn set_last_block_height<D: Database>(db: Arc<D>, height: u64) -> Result<(), Error> {
    db.update(|tx| tx.put::<LastBlockHeight>(LAST_HEIGHT_KEY, height))?.map_err(Into::into)
}

/// Read last block height from the database.
pub fn get_last_block_height<D: Database>(db: Arc<D>) -> Result<Option<u64>, Error> {
    db.view(|tx| tx.get::<LastBlockHeight>(LAST_HEIGHT_KEY))?.map_err(Into::into)
}

/// Delete in an Job from the database. [None] if it does not exist.
///
/// Returns `true` if the key/value pair was present.
///
/// Docs from libmdbx-rs: `Transaction::<RW>::del`:
/// The data parameter is NOT ignored regardless of whether the database supports
/// sorted duplicate data items or not. If the data parameter is [Some] only the matching
/// data item will be deleted. Otherwise, if data parameter is [None], any/all value(s)
/// for specified key will be deleted.
/// We pass in [None] here since each `job_id` only maps to a single Job.
pub fn delete_job<D: Database>(db: Arc<D>, job_id: [u8; 32]) -> Result<bool, Error> {
    db.update(|tx| tx.delete::<JobTable>(B256Key(job_id), None))?.map_err(Into::into)
}

/// Write a failed relayed job to the database
pub fn put_fail_relay_job<D: Database>(db: Arc<D>, job: Job) -> Result<(), Error> {
    db.update(|tx| tx.put::<RelayFailureJobs>(B256Key(job.id), job))?.map_err(Into::into)
}

/// Read in a failed relayed Job from the database. [None] if it does not exist
pub fn get_fail_relay_job<D: Database>(db: Arc<D>, job_id: [u8; 32]) -> Result<Option<Job>, Error> {
    db.view(|tx| tx.get::<RelayFailureJobs>(B256Key(job_id)))?.map_err(Into::into)
}

/// Read all failed relayed Jobs from the database. [None] if it does not exist
pub fn get_all_failed_jobs<D: Database>(db: Arc<D>) -> Result<Vec<Job>, Error> {
    db.view(|tx| -> Result<Vec<Job>, DatabaseError> {
        let mut failed_jobs = Vec::new();
        let mut cursor = tx.cursor_read::<RelayFailureJobs>()?;

        while let Some((_, job)) = cursor.next()? {
            failed_jobs.push(job);
        }

        Ok(failed_jobs)
    })?
    .map_err(Into::into)
}

/// Delete a failed relayed Job from the database. [None] if it does not exist.
pub fn delete_fail_relay_job<D: Database>(db: Arc<D>, job_id: [u8; 32]) -> Result<bool, Error> {
    db.update(|tx| tx.delete::<RelayFailureJobs>(B256Key(job_id), None))?.map_err(Into::into)
}

/// Open a DB at `path`. Creates the DB if it does not exist.
pub fn init_db<P: AsRef<Path>>(path: P) -> Result<Arc<DatabaseEnv>, Error> {
    let client_version = ClientVersion::default();
    let args = DatabaseArguments::new(client_version.clone());

    let db = create_db(path, args)?;
    db.record_client_version(client_version)?;

    {
        // This logic is largely from reth's `create_tables` fn, but uses our tables
        // instead of their's
        let tx = db.deref().begin_rw_txn().map_err(|e| DatabaseError::InitTx(e.into()))?;

        for table in tables::Tables::ALL {
            let flags = match table.table_type() {
                TableType::Table => DatabaseFlags::default(),
                TableType::DupSort => DatabaseFlags::DUP_SORT,
            };

            tx.create_db(Some(table.name()), flags)
                .map_err(|e| DatabaseError::CreateTable(e.into()))?;
        }

        tx.commit().map_err(|e| DatabaseError::Commit(e.into()))?;
    }

    Ok(Arc::new(db))
}

/// Metadata for queue
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct QueueMeta {
    head: Option<B256Key>,
    tail: Option<B256Key>,
}

/// A node in the queue.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct QueueNode {
    job_id: [u8; 32],
    prev: Option<B256Key>,
    next: Option<B256Key>,
}

/// In memory handle to queue. This is the abstraction used for all queue interactions.
///
/// WARNING: if you add the some job_id to two different queues, you will break the queues.
#[derive(Debug)]
pub struct Queue<D> {
    meta: QueueMeta,
    db: Arc<D>,
    key: AddrKey,
}

impl<D: Database> Queue<D> {
    /// Load a queue handle from the DB
    pub fn load(db: Arc<D>, key: AddrKey) -> Result<Option<Self>, Error> {
        let value =
            db.view(|tx| tx.get::<QueueMetaTable>(key))??.map(|meta| Self { meta, db, key });

        Ok(value)
    }

    /// Push to the front of the queue.
    pub fn push_front(&mut self, job_id: [u8; 32]) -> Result<(), Error> {
        let new_node = if let Some(head) = self.meta.head {
            // Update the current head to have a back pointer to the new node
            let mut head_node = self.db.view(|tx| tx.get::<QueueNodeTable>(head))??.expect("todo");
            head_node.prev = Some(B256Key(job_id));
            self.db.update(|tx| tx.put::<QueueNodeTable>(head, head_node))??;

            QueueNode { job_id, prev: None, next: Some(head) }
        } else {
            self.meta.tail = Some(B256Key(job_id));
            QueueNode { job_id, prev: None, next: None }
        };

        // Always update the head.
        self.db.update(|tx| tx.put::<QueueNodeTable>(B256Key(job_id), new_node))??;

        // Update this metadata in the db.
        self.meta.head = Some(job_id);
        self.commit()?;

        Ok(())
    }

    /// Get the value the value from the back without without removing it.
    pub fn peek_back(&self) -> Option<[u8; 32]> {
        self.meta.tail.clone().map(|k| k.0)
    }

    /// Remove the value from the back.
    pub fn pop_back(&mut self) -> Result<Option<[u8; 32]>, Error> {
        let (tail_node, result) = if let Some(tail) = self.meta.tail.clone() {
            let tail_node = self.db.view(|tx| tx.get::<QueueNodeTable>(tail))??.expect("todo");
            (tail_node, tail)
        } else {
            debug_assert!(self.meta.head.is_none());
            return Ok(None);
        };

        if let Some(new_tail) = tail_node.prev {
            self.db.update(|tx| tx.delete::<QueueNodeTable>(B256Key(tail_node.job_id), None))??;

            let mut new_tail_node =
                self.db.view(|tx| tx.get::<QueueNodeTable>(new_tail.clone()))??.expect("todo");
            new_tail_node.next = None;

            self.meta.tail = Some(new_tail.clone());

            self.db.update(|tx| {
                tx.delete::<QueueNodeTable>(B256Key(tail_node.job_id), None)?;
                tx.put::<QueueNodeTable>(new_tail, new_tail_node)
            })??;
        } else {
            // This was the only node. The queue is now empty
            debug_assert_eq!(self.meta.head, self.meta.tail);

            self.meta.head = None;
            self.meta.tail = None;
        }

        self.commit()?;

        Ok(Some(result.0))
    }

    /// Commit the metadata to the DB.
    fn commit(&mut self) -> Result<(), Error> {
        self.db.update(|tx| tx.put::<QueueMetaTable>(self.key, self.meta.clone()))??;
        Ok(())
    }
}

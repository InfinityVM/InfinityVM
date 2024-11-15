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
    ElfKey, ElfTable, ElfWithMeta, Job, JobID, JobTable, LastBlockHeight, RelayFailureJobs,
};

pub mod tables;
pub mod queue;

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
    let key = ElfKey::new(program_id);

    db.update(|tx| tx.put::<ElfTable>(key, elf_with_meta))?.map_err(Into::into)
}

/// Read in an ELF file from the database. [None] if it does not exist
pub fn get_elf<D: Database>(db: Arc<D>, program_id: &[u8]) -> Result<Option<ElfWithMeta>, Error> {
    db.view(|tx| tx.get::<ElfTable>(ElfKey::new(program_id)))?.map_err(Into::into)
}

/// Write a job to the database
pub fn put_job<D: Database>(db: Arc<D>, job: Job) -> Result<(), Error> {
    db.update(|tx| tx.put::<JobTable>(JobID(job.id), job))?.map_err(Into::into)
}

/// Read in an Job from the database. [None] if it does not exist
pub fn get_job<D: Database>(db: Arc<D>, job_id: [u8; 32]) -> Result<Option<Job>, Error> {
    db.view(|tx| tx.get::<JobTable>(JobID(job_id)))?.map_err(Into::into)
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
    db.update(|tx| tx.delete::<JobTable>(JobID(job_id), None))?.map_err(Into::into)
}

/// Write a failed relayed job to the database
pub fn put_fail_relay_job<D: Database>(db: Arc<D>, job: Job) -> Result<(), Error> {
    db.update(|tx| tx.put::<RelayFailureJobs>(JobID(job.id), job))?.map_err(Into::into)
}

/// Read in a failed relayed Job from the database. [None] if it does not exist
pub fn get_fail_relay_job<D: Database>(db: Arc<D>, job_id: [u8; 32]) -> Result<Option<Job>, Error> {
    db.view(|tx| tx.get::<RelayFailureJobs>(JobID(job_id)))?.map_err(Into::into)
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
    db.update(|tx| tx.delete::<RelayFailureJobs>(JobID(job_id), None))?.map_err(Into::into)
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

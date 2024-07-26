//! Executor database

use proto::{Job, VmType};
use reth_db::{
    create_db,
    mdbx::{DatabaseArguments, DatabaseFlags},
    models::ClientVersion,
    transaction::{DbTx, DbTxMut},
    Database, DatabaseEnv, DatabaseError, TableType,
};
use std::{ops::Deref, path::Path, sync::Arc};
use tables::{ElfKey, ElfWithMeta};

pub mod tables;

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
}

/// Write an ELF file to the database
pub fn put_elf<D: Database>(
    db: Arc<D>,
    vm_type: VmType,
    verifying_key: &[u8],
    elf: Vec<u8>,
) -> Result<(), Error> {
    use tables::ElfTable;
    let elf_with_meta = ElfWithMeta { vm_type: vm_type as u8, elf };
    let key = ElfKey::new(verifying_key);

    let tx = db.tx_mut()?;
    tx.put::<ElfTable>(key, elf_with_meta)?;
    let _commit = tx.commit()?;

    Ok(())
}

/// Read in an ELF file from the database. [None] if it does not exist
pub fn get_elf<D: Database>(
    db: Arc<D>,
    verifying_key: &[u8],
) -> Result<Option<ElfWithMeta>, Error> {
    use tables::ElfTable;
    let key = ElfKey::new(verifying_key);

    let tx = db.tx()?;
    let result = tx.get::<ElfTable>(key);
    // Free mem pages for read only tx
    let _commit = tx.commit()?;

    result.map_err(Into::into)
}

/// Write a job to the database
pub fn put_job<D: Database>(db: Arc<D>, job_id: u32, job: Job) -> Result<(), Error> {
    use tables::JobTable;

    let tx = db.tx_mut()?;
    tx.put::<JobTable>(job_id, job)?;
    let _commit = tx.commit()?;

    Ok(())
}

/// Read in an Job from the database. [None] if it does not exist
pub fn get_job<D: Database>(db: Arc<D>, job_id: u32) -> Result<Option<Job>, Error> {
    use tables::JobTable;

    let tx = db.tx()?;
    let result = tx.get::<JobTable>(job_id);
    // Free mem pages for read only tx
    let _commit = tx.commit()?;

    result.map_err(Into::into)
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
pub fn delete_job<D: Database>(db: Arc<D>, job_id: u32) -> Result<bool, Error> {
    use tables::JobTable;

    let tx = db.tx_mut()?;
    let result = tx.delete::<JobTable>(job_id, None);
    // Free mem pages for read only tx
    let _commit = tx.commit()?;

    result.map_err(Into::into)
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

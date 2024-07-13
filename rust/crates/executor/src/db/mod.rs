//! Executor database

use proto::VmType;
use reth_db::{
    create_db,
    mdbx::DatabaseArguments,
    models::ClientVersion,
    transaction::{DbTx, DbTxMut},
    Database, DatabaseEnv, DatabaseError, TableType,
};
use reth_libmdbx::DatabaseFlags;
use std::{ops::Deref, path::Path, sync::Arc};
use tables::ElfKey;

/// Database tables
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
pub fn write_elf<D: Database>(
    db: Arc<D>,
    vm_type: VmType,
    verifying_key: &[u8],
    elf: Vec<u8>,
) -> Result<(), Error> {
    use crate::db::tables::{Risc0ElfTable, Sp1ElfTable};

    let tx = db.tx_mut()?;
    match vm_type as VmType {
        VmType::Sp1 => {
            let key = ElfKey::new(VmType::Sp1 as u8, verifying_key);
            tx.put::<Sp1ElfTable>(key, elf)?;
        }
        VmType::Risc0 => {
            let key = ElfKey::new(VmType::Risc0 as u8, verifying_key);
            tx.put::<Risc0ElfTable>(key, elf)?;
        }
    }

    let _commit = tx.commit()?;

    Ok(())
}

/// Read in an ELF file from the database. None if it does not exist
pub fn read_elf<D: Database>(
    db: Arc<D>,
    vm_type: &VmType,
    verifying_key: &[u8],
) -> Result<Option<Vec<u8>>, Error> {
    use crate::db::tables::{Risc0ElfTable, Sp1ElfTable};

    let tx = db.tx()?;
    let result = match vm_type {
        VmType::Sp1 => {
            let key = ElfKey::new(VmType::Sp1 as u8, verifying_key);
            tx.get::<Sp1ElfTable>(key)
        }
        VmType::Risc0 => {
            let key = ElfKey::new(VmType::Risc0 as u8, verifying_key);
            tx.get::<Risc0ElfTable>(key)
        }
    };

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

        for table in crate::db::tables::Tables::ALL {
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

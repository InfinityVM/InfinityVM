//! Node database implementation.

use reth_db::{
    create_db,
    mdbx::{DatabaseArguments, DatabaseFlags},
    models::ClientVersion,
    DatabaseEnv, DatabaseError, TableType,
};
use std::{ops::Deref, path::Path};

/// Key for highest global index that has been seen.
pub const SEEN_GLOBAL_INDEX_KEY: u32 = 0;
/// Key for highest global index that has been processed. This is gte seen.
pub const PROCESSED_GLOBAL_INDEX_KEY: u32 = 1;
/// Key for the index of where the next batch should start.
pub const NEXT_BATCH_GLOBAL_INDEX_KEY: u32 = 2;
/// Key for last seen block height.
pub const LAST_SEEN_HEIGHT_KEY: u32 = 0;

pub mod models;
pub mod tables;

/// Open a DB at `path`. Creates the DB if it does not exist.
pub fn init_db<P: AsRef<Path>>(path: P) -> eyre::Result<DatabaseEnv> {
    let client_version = ClientVersion::default();
    let args = DatabaseArguments::new(client_version.clone());
    let db = create_db(path, args)?;
    db.record_client_version(client_version)?;

    {
        // This logic is largely from reth's `create_tables` fn, but uses our tables
        // instead of their's.
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

    Ok(db)
}

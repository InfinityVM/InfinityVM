//! The CLOB node.

use alloy::signers::{k256::ecdsa::SigningKey, local::LocalSigner};
use std::sync::Arc;
use tokio::task::JoinHandle;

pub mod app;
pub mod batcher;
pub mod client;
pub mod db;
pub mod engine;

/// Address to listen for HTTP requests on.
pub const CLOB_LISTEN_ADDR: &str = "CLOB_LISTEN_ADDR";
/// Directory for database.
pub const CLOB_DB_DIR: &str = "CLOB_DB_DIR";
/// Coprocessor Node gRPC address.
pub const CLOB_CN_GRPC_ADDR: &str = "CLOB_CN_GRPC_ADDR";
/// Execution Client HTTP address.
pub const CLOB_ETH_HTTP_ADDR: &str = "CLOB_ETH_HTTP_ADDR";
/// Clob Consumer contract address.
pub const CLOB_CONSUMER_ADDR: &str = "CLOB_CONSUMER_ADDR";
/// Duration between creating batches.
pub const CLOB_BATCHER_DURATION_MS: &str = "CLOB_BATCHER_DURATION_MS";
/// Clob operator's secret key.
pub const CLOB_OPERATOR_KEY: &str = "CLOB_OPERATOR_KEY";

/// Operator signer type.
pub type K256LocalSigner = LocalSigner<SigningKey>;

/// Run the CLOB node.
pub async fn run(
    db_dir: String,
    listen_addr: String,
    batcher_duration_ms: u64,
    operator_signer: K256LocalSigner,
    cn_grpc_url: String,
    clob_consumer_addr: [u8; 20],
) -> eyre::Result<()> {
    let db = crate::db::init_db(db_dir).expect("todo");
    let db = Arc::new(db);

    let (engine_sender, engine_receiver) = tokio::sync::mpsc::channel(32);
    let db2 = Arc::clone(&db);
    let server_handle = tokio::spawn(async move {
        let server_state = app::AppState::new(engine_sender, db2);
        app::http_listen(server_state, &listen_addr).await
    });

    let db2 = Arc::clone(&db);
    let engine_handle = tokio::spawn(async move { engine::run_engine(engine_receiver, db2).await });

    let batcher_handle = tokio::spawn(async move {
        let batcher_duration = tokio::time::Duration::from_millis(batcher_duration_ms);
        batcher::run_batcher(db, batcher_duration, operator_signer, cn_grpc_url, clob_consumer_addr)
            .await
    });

    tokio::try_join!(flatten(server_handle), flatten(engine_handle), flatten(batcher_handle))
        .map(|_| ())
}

async fn flatten<T>(handle: JoinHandle<eyre::Result<T>>) -> eyre::Result<T> {
    match handle.await {
        Ok(Ok(result)) => Ok(result),
        Ok(Err(err)) => Err(err),
        Err(err) => Err(eyre::eyre!(format!("{err}"))),
    }
}

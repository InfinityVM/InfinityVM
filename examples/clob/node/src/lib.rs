//! The CLOB node.

use crate::event::start_deposit_event_listener;
use alloy::{eips::BlockNumberOrTag, signers::local::PrivateKeySigner};
use std::{path::Path, sync::Arc};
use tokio::task::JoinHandle;

pub mod app;
pub mod batcher;
pub mod db;
pub mod engine;
pub mod event;

/// Address to listen for HTTP requests on.
pub const CLOB_LISTEN_ADDR: &str = "CLOB_LISTEN_ADDR";
/// Directory for database.
pub const CLOB_DB_DIR: &str = "CLOB_DB_DIR";
/// Coprocessor Node gRPC address.
pub const CLOB_CN_GRPC_ADDR: &str = "CLOB_CN_GRPC_ADDR";
/// WS Ethereum RPC address.
pub const CLOB_ETH_WS_ADDR: &str = "CLOB_ETH_WS_ADDR";
/// Clob Consumer contract address.
pub const CLOB_CONSUMER_ADDR: &str = "CLOB_CONSUMER_ADDR";
/// Duration between creating batches.
pub const CLOB_BATCHER_DURATION_MS: &str = "CLOB_BATCHER_DURATION_MS";
/// Clob operator's secret key.
pub const CLOB_OPERATOR_KEY: &str = "CLOB_OPERATOR_KEY";
/// Block to start syncing from.
pub const CLOB_JOB_SYNC_START: &str = "CLOB_JOB_SYNC_START";

/// Run the CLOB node.
#[allow(clippy::too_many_arguments)]
pub async fn run<P: AsRef<Path>>(
    db_dir: P,
    listen_addr: String,
    batcher_duration_ms: u64,
    operator_signer: PrivateKeySigner,
    cn_grpc_url: String,
    eth_ws_url: String,
    clob_consumer_addr: [u8; 20],
    job_sync_start: BlockNumberOrTag,
) -> eyre::Result<()> {
    let db = crate::db::init_db(db_dir)?;
    let db = Arc::new(db);

    let (engine_sender, engine_receiver) = tokio::sync::mpsc::channel(32);
    let db2 = Arc::clone(&db);
    let engine_sender_2 = engine_sender.clone();
    let server_handle = tokio::spawn(async move {
        let server_state = app::AppState::new(engine_sender_2, db2);
        app::http_listen(server_state, &listen_addr).await
    });

    let db2 = Arc::clone(&db);
    let engine_handle = tokio::spawn(async move { engine::run_engine(engine_receiver, db2).await });

    let db2 = Arc::clone(&db);
    let deposit_event_listener_handle = tokio::spawn(async move {
        start_deposit_event_listener(
            db2,
            eth_ws_url,
            clob_consumer_addr.into(),
            engine_sender,
            job_sync_start,
        )
        .await
    });

    let batcher_handle = tokio::spawn(async move {
        let batcher_duration = tokio::time::Duration::from_millis(batcher_duration_ms);
        batcher::run_batcher(db, batcher_duration, operator_signer, cn_grpc_url, clob_consumer_addr)
            .await
    });

    tokio::try_join!(
        flatten(server_handle),
        flatten(engine_handle),
        flatten(deposit_event_listener_handle),
        flatten(batcher_handle)
    )
    .map(|_| ())
}

async fn flatten<T>(handle: JoinHandle<eyre::Result<T>>) -> eyre::Result<T> {
    match handle.await {
        Ok(Ok(result)) => Ok(result),
        Ok(Err(err)) => Err(err),
        Err(err) => Err(eyre::eyre!(format!("{err}"))),
    }
}

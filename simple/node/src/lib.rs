//! The CLOB node.

use alloy::{
    eips::BlockNumberOrTag,
    signers::{k256::ecdsa::SigningKey, local::LocalSigner},
};
use simple_core::SimpleState;
use std::{path::Path, sync::Arc};
use tokio::{sync::RwLock, task::JoinHandle};

pub mod app;
pub mod batcher;
pub mod engine;
pub mod event;

/// Address to listen for HTTP requests on.
pub const SIMPLE_LISTEN_ADDR: &str = "SIMPLE_LISTEN_ADDR";
// /// Directory for database.
// pub const CLOB_DB_DIR: &str = "CLOB_DB_DIR";
/// Coprocessor Node gRPC address.
pub const SIMPLE_CN_GRPC_ADDR: &str = "SIMPLE_CN_GRPC_ADDR";
/// WS Ethereum RPC address.
pub const SIMPLE_ETH_WS_ADDR: &str = "SIMPLE_ETH_WS_ADDR";
/// Clob Consumer contract address.
pub const SIMPLE_CONSUMER_ADDR: &str = "SIMPLE_CONSUMER_ADDR";
/// Duration between creating batches.
pub const SIMPLE_BATCHER_DURATION_MS: &str = "SIMPLE_BATCHER_DURATION_MS";
/// Clob operator's secret key.
pub const SIMPLE_OPERATOR_KEY: &str = "SIMPLE_OPERATOR_KEY";
/// Block to start syncing from.
pub const SIMPLE_JOB_SYNC_START: &str = "SIMPLE_JOB_SYNC_START";

/// Operator signer type.
pub type K256LocalSigner = LocalSigner<SigningKey>;

/// Run the CLOB node.
#[allow(clippy::too_many_arguments)]
pub async fn run(
    // db_dir: P,
    simple_state: SimpleState,
    listen_addr: String,
    batcher_duration_ms: u64,
    operator_signer: K256LocalSigner,
    cn_grpc_url: String,
    eth_ws_url: String,
    clob_consumer_addr: [u8; 20],
    job_sync_start: BlockNumberOrTag,
) -> eyre::Result<()> {
    let shared_state = Arc::new(RwLock::new(simple_state));

    let (engine_sender, engine_receiver) = tokio::sync::mpsc::channel(32);
    // let db2 = Arc::clone(&db);
    let engine_sender_2 = engine_sender.clone();
    let server_handle = tokio::spawn({
        let shared_state = Arc::clone(&shared_state);
        async move {
            let server_state = app::AppState::new(shared_state, engine_sender_2);
            app::http_listen(server_state, &listen_addr).await
        }
    });

    // let db2 = Arc::clone(&db);
    let engine_handle = tokio::spawn({
        let shared_state = Arc::clone(&shared_state);
        async move { 
            engine::run_engine(shared_state, engine_receiver).await 
        }
    });

    // let db2 = Arc::clone(&db);
    // let deposit_event_listener_handle = tokio::spawn(async move {
    //     start_deposit_event_listener(
    //         db2,
    //         eth_ws_url,
    //         clob_consumer_addr.into(),
    //         engine_sender,
    //         job_sync_start,
    //     )
    //     .await
    // });

    // let batcher_handle = tokio::spawn(async move {
    //     let batcher_duration = tokio::time::Duration::from_millis(batcher_duration_ms);
    //     batcher::run_batcher(db, batcher_duration, operator_signer, cn_grpc_url, clob_consumer_addr)
    //         .await
    // });

    tokio::try_join!(
        flatten(server_handle),
        flatten(engine_handle),
        // flatten(deposit_event_listener_handle),
        // flatten(batcher_handle)
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

//! The matching game node.

use alloy::signers::{k256::ecdsa::SigningKey, local::LocalSigner};
use std::{path::Path, sync::Arc};
use tokio::task::JoinHandle;

pub mod app;
pub mod batcher;
pub mod client;
pub mod contracts;
pub mod engine;
pub mod state;
pub mod test_utils;

/// Address to listen for HTTP requests on.
pub const LISTEN_ADDR: &str = "LISTEN_ADDR";
/// Directory for database.
pub const DB_DIR: &str = "DB_DIR";
/// Coprocessor Node gRPC address.
pub const CN_GRPC_ADDR: &str = "CN_GRPC_ADDR";
/// Matching game consumer contract address.
pub const CONSUMER_ADDR: &str = "CONSUMER_ADDR";
/// Duration between creating batches.
pub const BATCHER_DURATION_MS: &str = "BATCHER_DURATION_MS";
/// Matching game operator's secret key.
pub const OPERATOR_KEY: &str = "OPERATOR_KEY";

/// Operator signer type.
pub type K256LocalSigner = LocalSigner<SigningKey>;

/// Run the matching game node.
#[allow(clippy::too_many_arguments)]
pub async fn run(
    listen_addr: String,
    batcher_duration_ms: u64,
    operator_signer: K256LocalSigner,
    cn_grpc_url: String,
    consumer_addr: [u8; 20],
) -> eyre::Result<()> {
    let state = Arc::new(state::InMemoryState::new());

    let (engine_sender, engine_receiver) = tokio::sync::mpsc::channel(32);
    let state_server = Arc::clone(&state);
    let engine_sender_2 = engine_sender.clone();
    let server_handle = tokio::spawn(async move {
        let server_state = app::AppState::new(engine_sender_2);
        app::http_listen(server_state, &listen_addr).await
    });

    let state_engine = Arc::clone(&state);
    let engine_handle = tokio::task::spawn_blocking(move || {
        tokio::runtime::Handle::current()
            .block_on(async move { engine::run_engine(engine_receiver, state_engine).await })
    });

    let state_batcher = Arc::clone(&state);
    let batcher_handle = tokio::task::spawn_blocking(move || {
        tokio::runtime::Handle::current().block_on(async move {
            let batcher_duration = tokio::time::Duration::from_millis(batcher_duration_ms);
            batcher::run_batcher(
                state_batcher,
                batcher_duration,
                operator_signer,
                cn_grpc_url,
                consumer_addr,
            )
            .await
        })
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

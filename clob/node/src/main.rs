//! The Infinity CLOB node binary.

use alloy::hex;
use clob_node::{batcher, engine, http_listen, AppState, K256LocalSigner};
use std::{env, sync::Arc};

// Small for now to get to failure cases quicker
const CHANEL_SIZE: usize = 32;
const DB_DIR: &str = "./tmp-data-dir/dev/db";

/// Secret for anvil key #6
pub const DEV_SECRET: &str = "92db14e403b83dfe3df233f83dfa3a0d7096f21ca9b0d6d6b8d88b2b4ec1564e";

use clob_node::{
    CLOB_BATCHER_DURATION_MS, CLOB_CN_GRPC_ADDR, CLOB_CONSUMER_ADDR, CLOB_DB_DIR,
    CLOB_ETH_HTTP_ADDR, CLOB_LISTEN_ADDR, CLOB_OPERATOR_KEY,
};

#[tokio::main]
async fn main() {
    let listen_addr = env::var(CLOB_LISTEN_ADDR).unwrap_or_else(|_| "127.0.0.1:3001".to_string());
    let db_dir = env::var(CLOB_DB_DIR).unwrap_or_else(|_| DB_DIR.to_string());
    let cn_grpc_addr =
        env::var(CLOB_CN_GRPC_ADDR).unwrap_or_else(|_| "127.0.0.1:50051".to_string());
    let batcher_duration_ms: u64 =
        env::var(CLOB_BATCHER_DURATION_MS).unwrap_or_else(|_| "1000".to_string()).parse().unwrap();

    // TODO: contract listening deposit
    let _eth_http_addr =
        env::var(CLOB_ETH_HTTP_ADDR).unwrap_or_else(|_| "http://127.0.0.1:8545".to_string());

    let operator_signer = {
        let operator_key = env::var(CLOB_OPERATOR_KEY).unwrap_or_else(|_| DEV_SECRET.to_string());
        let decoded = hex::decode(operator_key).unwrap();
        K256LocalSigner::from_slice(&decoded).unwrap()
    };

    let db = clob_node::db::init_db(db_dir).expect("todo");
    let db = Arc::new(db);

    let (engine_sender, engine_receiver) = tokio::sync::mpsc::channel(CHANEL_SIZE);

    let db2 = Arc::clone(&db);
    let server_handle = tokio::spawn(async move {
        let server_state = AppState::new(engine_sender, db2);
        http_listen(server_state, &listen_addr).await
    });

    let db2 = Arc::clone(&db);
    let engine_handle = tokio::spawn(async move { engine::run_engine(engine_receiver, db2).await });

    let clob_consumer_addr = {
        let hex = env::var(CLOB_CONSUMER_ADDR)
            .unwrap_or_else(|_| "b794f5ea0ba39494ce839613fffba74279579268".to_string());
        let bytes = hex::decode(hex).unwrap();
        bytes.try_into().unwrap()
    };
    let batcher_handle = tokio::spawn(async move {
        let batcher_duration = tokio::time::Duration::from_millis(batcher_duration_ms);
        batcher::run_batcher(
            db,
            batcher_duration,
            operator_signer,
            cn_grpc_addr,
            clob_consumer_addr,
        )
        .await
    });

    tokio::try_join!(server_handle, engine_handle, batcher_handle).unwrap();
}

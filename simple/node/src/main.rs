//! The Infinity CLOB node binary.

use alloy::{eips::BlockNumberOrTag, hex};
use simple_node::K256LocalSigner;
use std::env;

// // Small for now to get to failure cases quicker
// const DB_DIR: &str = "./tmp-data-dir/dev/db";

/// Secret for anvil key #6
pub const DEV_SECRET: &str = "92db14e403b83dfe3df233f83dfa3a0d7096f21ca9b0d6d6b8d88b2b4ec1564e";

use simple_node::{
    SIMPLE_BATCHER_DURATION_MS, SIMPLE_CN_GRPC_ADDR, SIMPLE_CONSUMER_ADDR, SIMPLE_ETH_WS_ADDR,
    SIMPLE_JOB_SYNC_START, SIMPLE_LISTEN_ADDR, SIMPLE_OPERATOR_KEY,
};

#[tokio::main]
async fn main() -> eyre::Result<()> {
    tracing_subscriber::fmt().event_format(tracing_subscriber::fmt::format()).init();

    let listen_addr = env::var(SIMPLE_LISTEN_ADDR).unwrap_or_else(|_| "127.0.0.1:3001".to_string());
    // let db_dir = env::var(CLOB_DB_DIR).unwrap_or_else(|_| DB_DIR.to_string());
    let cn_grpc_addr =
        env::var(SIMPLE_CN_GRPC_ADDR).unwrap_or_else(|_| "http://127.0.0.1:50051".to_string());
    let batcher_duration_ms: u64 =
        env::var(SIMPLE_BATCHER_DURATION_MS).unwrap_or_else(|_| "1000".to_string()).parse().unwrap();
    let eth_ws_addr =
        env::var(SIMPLE_ETH_WS_ADDR).unwrap_or_else(|_| "ws://127.0.0.1:8545".to_string());

    let job_sync_start: BlockNumberOrTag = env::var(SIMPLE_JOB_SYNC_START)
        .unwrap_or_else(|_| BlockNumberOrTag::Earliest.to_string())
        .parse()
        .unwrap();

    let operator_signer = {
        let operator_key = env::var(SIMPLE_OPERATOR_KEY).unwrap_or_else(|_| DEV_SECRET.to_string());
        let decoded = hex::decode(operator_key).unwrap();
        K256LocalSigner::from_slice(&decoded).unwrap()
    };

    let simple_consumer_addr = {
        let hex = env::var(SIMPLE_CONSUMER_ADDR)
            .unwrap_or_else(|_| "b794f5ea0ba39494ce839613fffba74279579268".to_string());
        let bytes = hex::decode(hex).unwrap();
        bytes.try_into().unwrap()
    };

    simple_node::run(
        // db_dir,
        listen_addr,
        batcher_duration_ms,
        operator_signer,
        cn_grpc_addr,
        eth_ws_addr,
        simple_consumer_addr,
        job_sync_start,
    )
    .await
}

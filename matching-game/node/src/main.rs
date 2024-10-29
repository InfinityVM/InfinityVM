//! The matching game node binary.

use alloy::hex;
use matching_game_node::K256LocalSigner;
use std::env;

// Small for now to get to failure cases quicker
const DEFAULT_DB_DIR: &str = "./tmp-data-dir/dev/db";

/// Secret for anvil key #6
pub const DEV_SECRET: &str = "92db14e403b83dfe3df233f83dfa3a0d7096f21ca9b0d6d6b8d88b2b4ec1564e";

use matching_game_node::{
    BATCHER_DURATION_MS, CN_GRPC_ADDR, CONSUMER_ADDR, DB_DIR, LISTEN_ADDR, OPERATOR_KEY,
};

#[tokio::main]
async fn main() -> eyre::Result<()> {
    tracing_subscriber::fmt().event_format(tracing_subscriber::fmt::format()).init();

    let listen_addr = env::var(LISTEN_ADDR).unwrap_or_else(|_| "127.0.0.1:3001".to_string());
    let cn_grpc_addr =
        env::var(CN_GRPC_ADDR).unwrap_or_else(|_| "http://127.0.0.1:50051".to_string());
    let batcher_duration_ms: u64 =
        env::var(BATCHER_DURATION_MS).unwrap_or_else(|_| "1000".to_string()).parse().unwrap();

    let operator_signer = {
        let operator_key = env::var(OPERATOR_KEY).unwrap_or_else(|_| DEV_SECRET.to_string());
        let decoded = hex::decode(operator_key).unwrap();
        K256LocalSigner::from_slice(&decoded).unwrap()
    };

    let consumer_addr = {
        let hex = env::var(CONSUMER_ADDR)
            .unwrap_or_else(|_| "b794f5ea0ba39494ce839613fffba74279579268".to_string());
        let bytes = hex::decode(hex).unwrap();
        bytes.try_into().unwrap()
    };

    matching_game_node::run(
        listen_addr,
        batcher_duration_ms,
        operator_signer,
        cn_grpc_addr,
        consumer_addr,
    )
    .await
}

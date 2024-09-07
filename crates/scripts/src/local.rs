//! Locally spin up the coprocessor node, anvil, and the clob.

use std::{fs::File, process::Command};

use alloy::primitives::hex;
use clob_node::{
    CLOB_BATCHER_DURATION_MS, CLOB_CN_GRPC_ADDR, CLOB_CONSUMER_ADDR, CLOB_DB_DIR,
    CLOB_ETH_HTTP_ADDR, CLOB_LISTEN_ADDR, CLOB_OPERATOR_KEY,
};
use clob_test_utils::anvil_with_clob_consumer;
use std::process::Stdio;
use test_utils::{anvil_with_job_manager, sleep_until_bound, ProcKill, LOCALHOST};
use tokio::signal::unix::{signal, SignalKind};
use tracing::info;

const ANVIL_PORT: u16 = 60420;
const COPROCESSOR_GRPC_PORT: u16 = 50420;
const COPROCESSOR_PROM_PORT: u16 = 50069;
const CLOB_PORT: u16 = 40420;

#[tokio::main]
async fn main() {
    let subscriber = tracing_subscriber::FmtSubscriber::new();
    tracing::subscriber::set_global_default(subscriber).unwrap();

    info!("Starting anvil on port: {ANVIL_PORT}");
    let anvil = anvil_with_job_manager(ANVIL_PORT).await;
    sleep_until_bound(ANVIL_PORT).await;

    let job_manager = anvil.job_manager.to_string();
    let chain_id = anvil.anvil.chain_id().to_string();
    let http_rpc_url = anvil.anvil.endpoint();
    let ws_rpc_url = anvil.anvil.ws_endpoint();

    let coproc_grpc = format!("{LOCALHOST}:{COPROCESSOR_GRPC_PORT}");
    let coproc_prometheus = format!("{LOCALHOST}:{COPROCESSOR_PROM_PORT}");
    let coproc_relayer_private = hex::encode(anvil.relayer.to_bytes());
    let coproc_operator_private = hex::encode(anvil.coprocessor_operator.to_bytes());
    let coproc_db_dir =
        tempfile::Builder::new().prefix("coprocessor-node-local-db").tempdir().unwrap();
    std::fs::create_dir_all("./logs").unwrap();
    let coproc_log_file = "./logs/coprocessor_node.log".to_string();
    info!(?coproc_log_file, "Writing coprocessor logs to: ");
    let coproc_logs = File::create(coproc_log_file).expect("failed to open log");
    let coproc_logs2 = coproc_logs.try_clone().unwrap();
    info!("Starting coprocessor node");
    info!(?coproc_prometheus, ?coproc_grpc, "Coprocessor listening on");
    info!(?coproc_relayer_private, ?coproc_operator_private, "Coprocessor keys");

    let _proc: ProcKill = Command::new("cargo")
        .arg("run")
        .arg("-p")
        .arg("coprocessor-node")
        .arg("--")
        .env("RELAYER_PRIVATE_KEY", coproc_relayer_private)
        .env("ZKVM_OPERATOR_PRIV_KEY", coproc_operator_private)
        .arg("--grpc-address")
        .arg(&coproc_grpc)
        .arg("--prom-address")
        .arg(&coproc_prometheus)
        .arg("--http-eth-rpc")
        .arg(&http_rpc_url)
        .arg("--ws-eth-rpc")
        .arg(ws_rpc_url)
        .arg("--job-manager-address")
        .arg(job_manager)
        .arg("--chain-id")
        .arg(chain_id)
        .arg("--db-dir")
        .arg(coproc_db_dir.path())
        .stdout(coproc_logs)
        .stderr(coproc_logs2)
        .spawn()
        .unwrap()
        .into();
    sleep_until_bound(COPROCESSOR_GRPC_PORT).await;

    let clob_consumer = anvil_with_clob_consumer(&anvil).await;
    let clob_db_dir = tempfile::Builder::new().prefix("clob-node-local-db").tempdir().unwrap();
    let clob_http = format!("{LOCALHOST}:{CLOB_PORT}");
    let batcher_duration_ms = 1000;
    let clob_log_file = "./logs/clob_node.log".to_string();
    let clob_logs = File::create(clob_log_file.clone()).expect("failed to open log");
    let clob_logs2 = clob_logs.try_clone().unwrap();
    let clob_consumer_addr = clob_consumer.clob_consumer.to_string();
    let clob_operator = hex::encode(clob_consumer.clob_signer.to_bytes());

    tracing::info!("Starting CLOB");
    tracing::info!(?clob_http, "CLOB listening on");
    tracing::info!(?clob_log_file, "CLOB logs");
    let _proc: ProcKill = Command::new("cargo")
        .arg("run")
        .arg("-p")
        .arg("clob-node")
        .env(CLOB_LISTEN_ADDR, clob_http)
        .env(CLOB_DB_DIR, clob_db_dir.path())
        .env(CLOB_CN_GRPC_ADDR, coproc_grpc)
        .env(CLOB_ETH_HTTP_ADDR, http_rpc_url)
        .env(CLOB_CONSUMER_ADDR, clob_consumer_addr)
        .env(CLOB_BATCHER_DURATION_MS, batcher_duration_ms.to_string())
        .env(CLOB_OPERATOR_KEY, clob_operator)
        .stdout(clob_logs)
        .stderr(clob_logs2)
        .spawn()
        .unwrap()
        .into();

    sleep_until_bound(CLOB_PORT).await;

    let mut signal_terminate = signal(SignalKind::terminate()).unwrap();
    let mut signal_interrupt = signal(SignalKind::interrupt()).unwrap();
    tokio::select! {
        _ = signal_terminate.recv() => tracing::info!("Received SIGTERM."),
        _ = signal_interrupt.recv() => tracing::info!("Received SIGINT."),
    };
}

//! E2E tests and helpers.
use alloy::primitives::hex;
use clob::{anvil_with_clob_consumer, AnvilClob};
use clob_node::{
    CLOB_BATCHER_DURATION_MS, CLOB_CN_GRPC_ADDR, CLOB_CONSUMER_ADDR, CLOB_DB_DIR,
    CLOB_ETH_HTTP_ADDR, CLOB_LISTEN_ADDR, CLOB_OPERATOR_KEY,
};
use futures::future::FutureExt;
use proto::coprocessor_node_client::CoprocessorNodeClient;
use std::process::Stdio;
use std::{
    future::Future,
    panic::AssertUnwindSafe,
    process::{self, Command},
};
use test_utils::{
    anvil_with_job_manager, anvil_with_mock_consumer, get_localhost_port, sleep_until_bound,
    AnvilJobManager, AnvilMockConsumer, LOCALHOST,
};
// use tokio::process::{self, Command};
use tonic::transport::Channel;

/// Test utilities for CLOB e2e tests.
pub mod clob;

/// The ethos reth crate is not part of the workspace so the binary is located
/// within the crate
pub const ETHOS_RETH_DEBUG_BIN: &str = "../bin/ethos-reth/target/debug/ethos-reth";
const COPROCESSOR_NODE_DEBUG_BIN: &str = "../target/debug/coprocessor-node";
const CLOB_NODE_DEBUG_BIN: &str = "../target/debug/clob-node";

/// Kill [`std::process::Child`] on `drop`
#[derive(Debug)]
pub struct ProcKill(process::Child);

impl From<process::Child> for ProcKill {
    fn from(child: process::Child) -> Self {
        Self(child)
    }
}

impl Drop for ProcKill {
    fn drop(&mut self) {
        drop(self.0.kill());
    }
}

/// Arguments passed to the test function.
#[derive(Debug)]
pub struct Args {
    /// `MockConsumer` deployment.
    pub mock_consumer: Option<AnvilMockConsumer>,
    /// `ClobConsumer` deployment.
    pub clob_consumer: Option<AnvilClob>,
    /// HTTP endpoint the clob node is listening on.
    pub clob_endpoint: Option<String>,
    /// Anvil setup with `JobManager`.
    pub anvil: AnvilJobManager,
    /// Coprocessor Node gRPC client
    pub coprocessor_node: CoprocessorNodeClient<Channel>,
}

/// E2E test environment builder and runner.
#[derive(Debug, Default)]
pub struct E2E {
    clob: bool,
    mock_consumer: bool,
}

impl E2E {
    /// Create a new [Self]
    pub const fn new() -> Self {
        Self { clob: false, mock_consumer: false }
    }

    /// Setup the clob
    pub const fn clob(mut self) -> Self {
        self.clob = true;
        self
    }

    /// Setup the clob
    pub const fn mock_consumer(mut self) -> Self {
        self.mock_consumer = true;
        self
    }
}

impl E2E {
    /// Run the given `test_fn`.
    pub async fn run<F, R>(self, test_fn: F)
    where
        F: Fn(Args) -> R,
        R: Future<Output = ()>,
    {
        // test_utils::test_tracing();
        let mut procs = vec![];

        let anvil = anvil_with_job_manager().await;

        let job_manager = anvil.job_manager.to_string();
        let chain_id = anvil.anvil.chain_id().to_string();
        let http_rpc_url = anvil.anvil.endpoint();
        let ws_rpc_url = anvil.anvil.ws_endpoint();

        let db_dir = tempfile::Builder::new().prefix("coprocessor-node-test-db").tempdir().unwrap();
        let coprocessor_node_port = get_localhost_port();
        let coprocessor_node_grpc = format!("{LOCALHOST}:{coprocessor_node_port}");
        let prometheus_port = get_localhost_port();
        let prometheus_addr = format!("{LOCALHOST}:{prometheus_port}");
        let relayer_private = hex::encode(anvil.relayer.to_bytes());
        let operator_private = hex::encode(anvil.coprocessor_operator.to_bytes());
        let cn_grpc_addr = format!("http://{coprocessor_node_grpc}");

        // The coprocessor-node expects the relayer private key as an env var
        let proc: ProcKill = Command::new(COPROCESSOR_NODE_DEBUG_BIN)
            .env("RELAYER_PRIVATE_KEY", relayer_private)
            .env("ZKVM_OPERATOR_PRIV_KEY", operator_private)
            .arg("--grpc-address")
            .arg(&cn_grpc_addr)
            .arg("--prom-address")
            .arg(&prometheus_addr)
            .arg("--http-eth-rpc")
            .arg(&http_rpc_url)
            .arg("--ws-eth-rpc")
            .arg(ws_rpc_url)
            .arg("--job-manager-address")
            .arg(job_manager)
            .arg("--chain-id")
            .arg(chain_id)
            .arg("--db-dir")
            .arg(db_dir.path())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .spawn()
            .unwrap()
            .into();
        procs.push(proc);
        sleep_until_bound(coprocessor_node_port).await;
        let coprocessor_node = CoprocessorNodeClient::connect(cn_grpc_addr.clone()).await.unwrap();

        let mut args = Args {
            mock_consumer: None,
            coprocessor_node,
            anvil,
            clob_consumer: None,
            clob_endpoint: None,
        };

        if self.mock_consumer {
            args.mock_consumer = Some(anvil_with_mock_consumer(&args.anvil).await)
        }

        if self.clob {
            let clob_consumer = anvil_with_clob_consumer(&args.anvil).await;
            let db_dir =
                tempfile::Builder::new().prefix("clob-node-test-db").tempdir().unwrap().into_path();
            let listen_port = get_localhost_port();
            let listen_addr = format!("{LOCALHOST}:{listen_port}");
            let clob_signer = hex::encode(clob_consumer.clob_signer.to_bytes());

            let proc: ProcKill = Command::new(CLOB_NODE_DEBUG_BIN)
                .env(CLOB_LISTEN_ADDR, &listen_addr)
                .env(CLOB_DB_DIR, db_dir)
                .env(CLOB_CN_GRPC_ADDR, cn_grpc_addr)
                .env(CLOB_ETH_HTTP_ADDR, &http_rpc_url)
                .env(CLOB_CONSUMER_ADDR, clob_consumer.clob_consumer.to_string())
                .env(CLOB_BATCHER_DURATION_MS, "10")
                .env(CLOB_OPERATOR_KEY, clob_signer)
                .stdout(Stdio::inherit())
                .stderr(Stdio::inherit())
                .spawn()
                .unwrap()
                .into();
            procs.push(proc);
            sleep_until_bound(coprocessor_node_port).await;

            args.clob_consumer = Some(clob_consumer);
            let clob_endpoint = format!("http://{listen_addr}");
            args.clob_endpoint = Some(clob_endpoint);
        }

        let test_result = AssertUnwindSafe(test_fn(args)).catch_unwind().await;
        assert!(test_result.is_ok())
    }
}

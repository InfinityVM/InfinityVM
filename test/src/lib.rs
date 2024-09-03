//! E2E tests and helpers.
use alloy::primitives::hex;
use clob_test_utils::{anvil_with_clob_consumer, AnvilClob};
use futures::future::FutureExt;
use proto::coprocessor_node_client::CoprocessorNodeClient;
use std::{
    future::Future,
    panic::AssertUnwindSafe,
    process::{self, Command, Stdio},
};
use test_utils::{
    anvil_with_job_manager, anvil_with_mock_consumer, get_localhost_port, sleep_until_bound,
    AnvilJobManager, AnvilMockConsumer, LOCALHOST,
};
use tonic::transport::Channel;

/// The ethos reth crate is not part of the workspace so the binary is located
/// within the crate
pub const ETHOS_RETH_DEBUG_BIN: &str = "../bin/ethos-reth/target/debug/ethos-reth";
const COPROCESSOR_NODE_DEBUG_BIN: &str = "../target/debug/coprocessor-node";

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
    /// Create a new [Self].
    pub const fn new() -> Self {
        Self { clob: false, mock_consumer: false }
    }

    /// Setup the clob consumer contracts and service.
    pub const fn clob(mut self) -> Self {
        self.clob = true;
        self
    }

    /// Setup mock consumer contract.
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
        test_utils::test_tracing();

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
        let cn_grpc_client_url = format!("http://{coprocessor_node_grpc}");

        let _proc: ProcKill = Command::new(COPROCESSOR_NODE_DEBUG_BIN)
            .env("RELAYER_PRIVATE_KEY", relayer_private)
            .env("ZKVM_OPERATOR_PRIV_KEY", operator_private)
            .arg("--grpc-address")
            .arg(&coprocessor_node_grpc)
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
        sleep_until_bound(coprocessor_node_port).await;
        let coprocessor_node =
            CoprocessorNodeClient::connect(cn_grpc_client_url.clone()).await.unwrap();

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
            let db_dir = tempfile::Builder::new()
                .prefix("clob-node-test-db")
                .tempdir()
                .unwrap()
                .into_path()
                .to_str()
                .unwrap()
                .to_string();
            let listen_port = get_localhost_port();
            let listen_addr = format!("{LOCALHOST}:{listen_port}");
            let batcher_duration_ms = 1000;

            let clob_consumer_addr = clob_consumer.clob_consumer;
            let listen_addr2 = listen_addr.clone();
            let operator_signer = clob_consumer.clob_signer.clone();
            tokio::spawn(async move {
                clob_node::run(
                    db_dir,
                    listen_addr2,
                    batcher_duration_ms,
                    operator_signer,
                    cn_grpc_client_url.clone(),
                    **clob_consumer_addr,
                )
                .await
            });
            sleep_until_bound(coprocessor_node_port).await;

            let clob_endpoint = format!("http://{listen_addr}");
            args.clob_endpoint = Some(clob_endpoint);
            args.clob_consumer = Some(clob_consumer);
        }

        let test_result = AssertUnwindSafe(test_fn(args)).catch_unwind().await;
        assert!(test_result.is_ok())
    }
}

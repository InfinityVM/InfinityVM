//! Integration tests and helpers.
use alloy::primitives::hex;
use futures::future::FutureExt;
use rand::Rng;
use std::{
    future::Future,
    net::TcpListener,
    panic::AssertUnwindSafe,
    process::{self, Command},
    thread,
    time::Duration,
};
use test_utils::{anvil_with_contracts, TestAnvil};
use tonic::transport::Channel;

use proto::coprocessor_node_client::CoprocessorNodeClient;

const LOCALHOST: &str = "127.0.0.1";
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
    /// Anvil setup stuff
    pub anvil: TestAnvil,
    /// Coprocessor Node gRPC client
    pub coprocessor_node: CoprocessorNodeClient<Channel>,
}

/// Integration test environment builder and runner.
#[derive(Debug)]
pub struct Integration;

impl Integration {
    /// Run the given `test_fn`.
    pub async fn run<F, R>(test_fn: F)
    where
        F: Fn(Args) -> R,
        R: Future<Output = ()>,
    {
        // test_tracing();

        let subscriber = tracing_subscriber::FmtSubscriber::new();
        // use that subscriber to process traces emitted after this point
        tracing::subscriber::set_global_default(subscriber).unwrap();

        // Start an anvil node
        let anvil = anvil_with_contracts().await;
        let job_manager = anvil.job_manager.to_string();
        let chain_id = anvil.anvil.chain_id().to_string();
        let rpc_url = anvil.anvil.endpoint();

        let db_dir = tempfile::Builder::new().prefix("coprocessor-node-test-db").tempdir().unwrap();
        let coprocessor_node_port = get_localhost_port();
        let coprocessor_node_grpc = format!("{LOCALHOST}:{coprocessor_node_port}");
        let relayer_private = hex::encode(anvil.relayer.to_bytes());
        let operator_private = hex::encode(anvil.coprocessor_operator.to_bytes());

        // The coprocessor-node expects the relayer private key as an env var
        std::env::set_var("RELAYER_PRIVATE_KEY", relayer_private);
        std::env::set_var("ZKVM_OPERATOR_PRIV_KEY", operator_private);
        // TODO: update the usage of these args when we setup an e2e test that uses this
        // https://github.com/Ethos-Works/InfinityVM/issues/104
        let _proc: ProcKill = Command::new(COPROCESSOR_NODE_DEBUG_BIN)
            .arg("--grpc-address")
            .arg(&coprocessor_node_grpc)
            .arg("--eth-rpc-address")
            .arg(rpc_url)
            .arg("--job-manager-address")
            .arg(job_manager)
            .arg("--chain-id")
            .arg(chain_id)
            .arg("--db-dir")
            .arg(db_dir.path())
            .spawn()
            .unwrap()
            .into();
        sleep_until_bound(coprocessor_node_port);
        let coprocessor_node =
            CoprocessorNodeClient::connect(format!("http://{coprocessor_node_grpc}"))
                .await
                .unwrap();

        let args = Args { coprocessor_node, anvil };

        let test_result = AssertUnwindSafe(test_fn(args)).catch_unwind().await;
        assert!(test_result.is_ok())
    }
}

fn get_localhost_port() -> u16 {
    let mut rng = rand::thread_rng();

    for _ in 0..64 {
        let port = rng.gen_range(49152..65535);
        if TcpListener::bind((LOCALHOST, port)).is_ok() {
            return port;
        }
    }

    panic!("no port found after 64 attempts");
}

fn sleep_until_bound(port: u16) {
    for _ in 0..16 {
        if TcpListener::bind((LOCALHOST, port)).is_err() {
            return;
        }

        thread::sleep(Duration::from_secs(1));
    }

    panic!("localhost:{port} was not successfully bound");
}

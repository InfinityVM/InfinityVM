//! Integration tests and helpers.
use coprocessor_node::test_utils::anvil_with_contracts;
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
use tonic::transport::Channel;

use proto::coprocessor_node_client::CoprocessorNodeClient;

const LOCALHOST: &str = "127.0.0.1";
const COPROCESSOR_NODE_DEBUG_BIN: &str = "../target/debug/coprocessor-node";

/// Relayer operators private key for development
pub const RELAYER_DEV_SECRET: &str =
    "abcd1d0798f97d79848a013d4936a73bf4cc922c825d33c1cf7073dff6d4abcd";

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

/// Integration test environment builder and runner.
#[derive(Debug)]
pub struct Integration;

impl Integration {
    /// Run the given `test_fn`.
    pub async fn run<F, R>(test_fn: F)
    where
        F: Fn(CoprocessorNodeClient<Channel>) -> R,
        R: Future<Output = ()>,
    {
        // Start an anvil node
        let anvil_config = anvil_with_contracts().await;
        let job_manager = anvil_config.job_manager.to_string();
        let chain_id = anvil_config.anvil.chain_id().to_string();
        let rpc_url = anvil_config.anvil.endpoint();

        let db_dir = tempfile::Builder::new().prefix("coprocessor-node-test-db").tempdir().unwrap();
        let coprocessor_node_port = get_localhost_port();
        let coprocessor_node_grpc = format!("{LOCALHOST}:{coprocessor_node_port}");

        // The coprocessor-node expects the relayer private key as an env var
        std::env::set_var("RELAYER_PRIVATE_KEY", RELAYER_DEV_SECRET);
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
            .arg("dev")
            .spawn()
            .unwrap()
            .into();
        sleep_until_bound(coprocessor_node_port);
        let coprocessor_node =
            CoprocessorNodeClient::connect(format!("http://{coprocessor_node_grpc}"))
                .await
                .unwrap();

        let test_result = AssertUnwindSafe(test_fn(coprocessor_node)).catch_unwind().await;
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

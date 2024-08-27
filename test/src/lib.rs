//! E2E tests and helpers.
use alloy::primitives::hex;
use futures::future::FutureExt;
use std::{
    future::Future,
    panic::AssertUnwindSafe,
    process::{self, Command},
};
use test_utils::{
    anvil_with_contracts, get_localhost_port, sleep_until_bound, TestAnvil, LOCALHOST,
};
use tonic::transport::Channel;

use proto::coprocessor_node_client::CoprocessorNodeClient;

/// Test utilities.
pub mod utils;

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
    /// Anvil setup stuff
    pub anvil: TestAnvil,
    /// Coprocessor Node gRPC client
    pub coprocessor_node: CoprocessorNodeClient<Channel>,
}

/// E2E test environment builder and runner.
#[derive(Debug)]
pub struct E2E;

impl E2E {
    /// Run the given `test_fn`.
    pub async fn run<F, R>(test_fn: F)
    where
        F: Fn(Args) -> R,
        R: Future<Output = ()>,
    {
        test_utils::test_tracing();

        // Start an anvil node
        let anvil = anvil_with_contracts().await;
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

        // The coprocessor-node expects the relayer private key as an env var
        std::env::set_var("RELAYER_PRIVATE_KEY", relayer_private);
        std::env::set_var("ZKVM_OPERATOR_PRIV_KEY", operator_private);
        let _proc: ProcKill = Command::new(COPROCESSOR_NODE_DEBUG_BIN)
            .arg("--grpc-address")
            .arg(&coprocessor_node_grpc)
            .arg("--prom-address")
            .arg(&prometheus_addr)
            .arg("--http-eth-rpc")
            .arg(http_rpc_url)
            .arg("--ws-eth-rpc")
            .arg(ws_rpc_url)
            .arg("--job-manager-address")
            .arg(job_manager)
            .arg("--chain-id")
            .arg(chain_id)
            .arg("--db-dir")
            .arg(db_dir.path())
            .spawn()
            .unwrap()
            .into();
        sleep_until_bound(coprocessor_node_port).await;
        let coprocessor_node =
            CoprocessorNodeClient::connect(format!("http://{coprocessor_node_grpc}"))
                .await
                .unwrap();

        let args = Args { anvil, coprocessor_node };

        let test_result = AssertUnwindSafe(test_fn(args)).catch_unwind().await;
        assert!(test_result.is_ok())
    }
}

#[cfg(test)]
mod test {
    use crate::{ProcKill, ETHOS_RETH_DEBUG_BIN};
    use std::process::Command;

    // #[test]
    // fn ethos_reth_exists() {
    //     let _proc: ProcKill =
    //         Command::new(ETHOS_RETH_DEBUG_BIN).arg("node").arg("--dev").spawn().unwrap().into();
    // }
}

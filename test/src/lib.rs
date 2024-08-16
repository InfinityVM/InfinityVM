//! Integration tests and helpers.
use alloy::primitives::{ChainId, hex};
use futures::future::FutureExt;
use std::{
    future::Future,
    panic::AssertUnwindSafe,
    process::{self, Command},
};

use alloy::{
    network::EthereumWallet,
    node_bindings::{Anvil, AnvilInstance},
    primitives::{utils::keccak256, Address, U256},
    providers::{ext::AnvilApi, ProviderBuilder},
    signers::{local::PrivateKeySigner, Signer},
    sol,
    sol_types::{SolType, SolValue},
};
use test_utils::{anvil_with_contracts, get_localhost_port, sleep_until_bound, TestAnvil, LOCALHOST, deploy_contracts, Params};
use tonic::transport::Channel;

use proto::coprocessor_node_client::CoprocessorNodeClient;

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

pub struct ParamsArgs {
    pub coprocessor_operator_addr: Address,
    pub mock_consumer: Address,
    pub random_signer: PrivateKeySigner,
    pub endpoint: String,
    pub chain_id: ChainId,
    pub coprocessor_node: CoprocessorNodeClient<Channel>,
}

/// Integration test environment builder and runner.
#[derive(Debug)]
pub struct Integration;

impl Integration {
    /// Run the given `test_fn`.
    pub async fn run<F, R>(test_fn: F)
    where
        F: Fn(ParamsArgs) -> R,
        R: Future<Output = ()>,
    {
        test_utils::test_tracing();

        let port = get_localhost_port();
        // Set block time to 0.01 seconds - I WANNA GO FAST MOM
        let anvil = Anvil::new().block_time_f64(0.01).port(port).try_spawn().unwrap();

        let initial_owner: PrivateKeySigner = anvil.keys()[0].clone().into();
        let relayer: PrivateKeySigner = anvil.keys()[1].clone().into();
        let coprocessor_operator: PrivateKeySigner = anvil.keys()[2].clone().into();
        let proxy_admin: PrivateKeySigner = anvil.keys()[3].clone().into();
        let consumer_owner: PrivateKeySigner = anvil.keys()[4].clone().into();
        let offchain_signer: PrivateKeySigner = anvil.keys()[5].clone().into();
        let random_signer: PrivateKeySigner = anvil.keys()[5].clone().into();
        let rpc_url = anvil.endpoint().clone();

        // Start an anvil node
        // let anvil = anvil_with_contracts().await;
        let coprocessor_clone = coprocessor_operator.clone();
        let params = Params{
            initial_owner,
            relayer,
            coprocessor_operator: coprocessor_clone,
            proxy_admin,
            consumer_owner,
            offchain_signer,
            rpc_url: rpc_url.clone(),
        };
        let res = deploy_contracts(params).await;
        let mock_consumer = res.mock_consumer;

        let job_manager = res.job_manager.to_string();
        let chain_id = anvil.chain_id().to_string();
        let http_rpc_url = anvil.endpoint();
        let ws_rpc_url = anvil.ws_endpoint();

        let db_dir = tempfile::Builder::new().prefix("coprocessor-node-test-db").tempdir().unwrap();
        let coprocessor_node_port = get_localhost_port();
        let coprocessor_node_grpc = format!("{LOCALHOST}:{coprocessor_node_port}");
        let prometheus_port = get_localhost_port();
        let prometheus_addr = format!("{LOCALHOST}:{prometheus_port}");
        let relayer_private = hex::encode(res.relayer.to_bytes());
        let operator_private = hex::encode(res.coprocessor_operator.to_bytes());

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

        // let args = Args { anvil, coprocessor_node };
        let args = ParamsArgs {
            coprocessor_operator_addr: coprocessor_operator.address(),
            mock_consumer,
            random_signer,
            endpoint: rpc_url,
            chain_id: anvil.chain_id(),
            coprocessor_node,
        };

        let test_result = AssertUnwindSafe(test_fn(args)).catch_unwind().await;
        assert!(test_result.is_ok())
    }
}

#[cfg(test)]
mod test {
    use crate::{ProcKill, ETHOS_RETH_DEBUG_BIN};
    use reth_e2e_test_utils::wallet::Wallet;
    use std::process::Command;

    #[test]
    fn ethos_reth_exists() {
        let _proc: ProcKill =
            Command::new(ETHOS_RETH_DEBUG_BIN).arg("node").arg("--dev").spawn().unwrap().into();

        // Just check that this works
        let _signers = Wallet::new(6).gen();
    }
}

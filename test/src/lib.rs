//! Integration tests and helpers.
use alloy::primitives::{ChainId, hex};
use futures::future::FutureExt;
use std::{
    future::Future,
    panic::AssertUnwindSafe,
    process::{self, Command},
};
use std::thread::sleep;
use std::time::Duration;
use alloy::{
    network::EthereumWallet,
    node_bindings::{Anvil, AnvilInstance},
    primitives::{utils::keccak256, Address, U256},
    providers::{ext::AnvilApi, ProviderBuilder},
    signers::{local::PrivateKeySigner, Signer},
    sol,
    sol_types::{SolType, SolValue},
};
use alloy::transports::http::reqwest;
use reth_e2e_test_utils::wallet::Wallet;
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
pub struct Args {
    pub coprocessor_operator_addr: Address,
    pub mock_consumer: Address,
    pub random_signer: PrivateKeySigner,
    pub endpoint: String,
    pub chain_id: ChainId,
    pub coprocessor_node: CoprocessorNodeClient<Channel>,
}

/// Integration test environment builder and runner.
#[derive(Debug, Default)]
pub struct Integration{
    pub with_reth: bool
}

impl Integration {
    /// Create new instance of Integration Suite
    pub fn new() -> Self {
        Self::default()
    }

    /// Set `with_reth` flag to true.
    pub fn with_reth(mut self) -> Self {
        self.with_reth = true;
        self
    }

    /// Run the given `test_fn`.
    pub async fn run<F, R>(self, test_fn: F)
    where
        F: Fn(Args) -> R,
        R: Future<Output = ()>,
    {
        test_utils::test_tracing();


        // Ensure the anvil instance will not collide with anything already running on the OS
        let port = get_localhost_port();
        let mut initial_owner: PrivateKeySigner = PrivateKeySigner::random();
        let mut relayer: PrivateKeySigner = PrivateKeySigner::random();
        let mut coprocessor_operator: PrivateKeySigner = PrivateKeySigner::random();
        let mut proxy_admin: PrivateKeySigner = PrivateKeySigner::random();
        let mut consumer_owner: PrivateKeySigner = PrivateKeySigner::random();
        let mut offchain_signer: PrivateKeySigner = PrivateKeySigner::random();
        let mut random_signer: PrivateKeySigner = PrivateKeySigner::random();
        let mut chain: ChainId = 0;

        let mut rpc_url: String = String::new();
        let mut chain_id: String = String::new();
        let mut http_rpc_url: String = String::new();
        let mut ws_rpc_url: String = String::new();

        // TODO: Fix error initializing anvil in the scope of else
        let anvil = Anvil::new().block_time_f64(0.01).port(port).try_spawn().unwrap();

        if self.with_reth {
            // Start reth
            let _proc: ProcKill =
                Command::new(ETHOS_RETH_DEBUG_BIN).arg("node").arg("--dev").spawn().unwrap().into();
            let keys = Wallet::new(6).gen();

            // Set keys
            initial_owner = keys[0].clone().into();
            relayer = keys[1].clone().into();
            coprocessor_operator = keys[2].clone().into();
            proxy_admin = keys[3].clone().into();
            consumer_owner = keys[4].clone().into();
            offchain_signer = keys[5].clone().into();
            random_signer = keys[5].clone().into();

            // Set node config
            rpc_url = format!("http://localhost:{}", port).clone();
            chain = 31_337;
            chain_id = chain.to_string();
            http_rpc_url = format!("http://localhost:{}", port);
            ws_rpc_url = format!("ws://localhost:{}", port);
        } else {
            // Set keys
            initial_owner = anvil.keys()[0].clone().into();
            relayer = anvil.keys()[1].clone().into();
            coprocessor_operator = anvil.keys()[2].clone().into();
            proxy_admin = anvil.keys()[3].clone().into();
            consumer_owner = anvil.keys()[4].clone().into();
            offchain_signer = anvil.keys()[5].clone().into();
            random_signer = anvil.keys()[5].clone().into();

            // Set node config
            rpc_url = anvil.endpoint().clone();
            chain = anvil.chain_id().clone();
            chain_id = anvil.chain_id().to_string();
            http_rpc_url = anvil.endpoint();
            ws_rpc_url = anvil.ws_endpoint();
        }

        let params = Params{
            initial_owner: initial_owner.clone(),
            relayer: relayer.clone(),
            coprocessor_operator: coprocessor_operator.clone(),
            proxy_admin: proxy_admin.clone(),
            consumer_owner: consumer_owner.clone(),
            offchain_signer: offchain_signer.clone(),
            rpc_url: rpc_url.clone(),
        };

        let test_addrs = deploy_contracts(params).await;

        let mock_consumer = test_addrs.mock_consumer;
        let job_manager = test_addrs.job_manager.to_string();

        let db_dir = tempfile::Builder::new().prefix("coprocessor-node-test-db").tempdir().unwrap();
        let coprocessor_node_port = get_localhost_port();
        let coprocessor_node_grpc = format!("{LOCALHOST}:{coprocessor_node_port}");
        let prometheus_port = get_localhost_port();
        let prometheus_addr = format!("{LOCALHOST}:{prometheus_port}");
        let relayer_private = hex::encode(relayer.to_bytes());
        let operator_private = hex::encode(coprocessor_operator.to_bytes());

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


        let args = Args {
            coprocessor_operator_addr: coprocessor_operator.address(),
            mock_consumer,
            random_signer,
            endpoint: rpc_url,
            chain_id: chain,
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

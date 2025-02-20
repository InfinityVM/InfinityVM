//! Utilities for setting up tests.

use std::{net::TcpListener, process::Command};

use crate::wallet::Wallet;
use alloy::{
    network::EthereumWallet,
    node_bindings::{Anvil, AnvilInstance},
    primitives::{keccak256, Address},
    providers::{ext::AnvilApi, ProviderBuilder},
    signers::{local::PrivateKeySigner, Signer},
    sol_types::SolValue,
};
use eyre::WrapErr;
use ivm_abi::{abi_encode_offchain_job_request, JobParams};
use ivm_contracts::{
    job_manager::JobManager, transparent_upgradeable_proxy::TransparentUpgradeableProxy,
};
use rand::Rng;
use tokio::time::{sleep, Duration};
use tracing_subscriber::{filter::LevelFilter, EnvFilter};

pub mod wallet;

/// Localhost IP address
pub const LOCALHOST: &str = "127.0.0.1";

/// Kill [`std::process::Child`] on `drop`
#[derive(Debug)]
pub struct ProcKill(pub std::process::Child);

impl From<std::process::Child> for ProcKill {
    fn from(child: std::process::Child) -> Self {
        Self(child)
    }
}

impl Drop for ProcKill {
    fn drop(&mut self) {
        drop(self.0.kill());
    }
}

/// Initialize a tracing subscriber for tests. Use `RUSTLOG` to set the filter level. Defaults to
/// INFO for log level.
///
/// If the tracing subscriber has already been initialized in a previous test, this
/// function will silently fail due to `try_init()`, which does not reinitialize
/// the subscriber if one is already set.
pub fn test_tracing() {
    let filter =
        EnvFilter::builder().with_default_directive(LevelFilter::INFO.into()).from_env_lossy();
    let _ =
        tracing_subscriber::fmt().with_env_filter(filter).with_writer(std::io::stderr).try_init();
}

/// Find a free port on localhost.
pub fn get_localhost_port() -> u16 {
    let mut rng = rand::thread_rng();

    for _ in 0..64 {
        let port = rng.gen_range(49152..65535);
        if TcpListener::bind((LOCALHOST, port)).is_ok() {
            return port;
        }
    }

    panic!("no port found after 64 attempts");
}

/// Sleep until the given port is bound.
pub async fn sleep_until_bound(port: u16) {
    if let Err(e) = sleep_until_bound_config(port, 16).await {
        panic!("{e}");
    }
}

/// Sleep until the given port is bound after the given number of attempts. Each attempt has 1
/// second of sleep between.
pub async fn sleep_until_bound_config(port: u16, attempts: usize) -> Result<(), String> {
    for _ in 0..attempts {
        if TcpListener::bind((LOCALHOST, port)).is_err() {
            return Ok(());
        }

        sleep(Duration::from_secs(1)).await;
    }

    Err(format!("localhost:{port} was not successfully bound"))
}

/// Output from [`anvil_with_job_manager`]
#[derive(Debug)]
pub struct AnvilJobManager {
    /// Anvil instance
    pub anvil: AnvilInstance,
    /// Address of the job manager contract
    pub job_manager: Address,
    /// Relayer private key
    pub relayer: PrivateKeySigner,
    /// Coprocessor operator private key
    pub coprocessor_operator: PrivateKeySigner,
}

/// Spin up anvil instance with job manager contracts.
pub async fn anvil_with_job_manager(port: u16) -> AnvilJobManager {
    // Ensure the anvil instance will not collide with anything already running on the OS
    // Set block time to 1.0 seconds because anvil doesn't accept 0.01 anymore
    let anvil = Anvil::new()
        .block_time_f64(1.0)
        .port(port)
        // 1000 dev accounts generated and configured
        .args(["-a", "1000", "--hardfork", "cancun"])
        .keep_stdout()
        .spawn();

    let job_manager_deploy = job_manager_deploy(anvil.endpoint()).await;

    job_manager_deploy.into_anvil_job_manager(anvil)
}

/// Handle to an instance of `ivm-exec`. Intended to be used similar to
/// alloys' `AnvilInstance`
pub struct IvmExecInstance {
    child: std::process::Child,
    port: u16,
}

impl IvmExecInstance {
    /// Try to spawn a new ivm exec instance
    pub fn try_spawn(port: u16) -> Result<Self, eyre::Error> {
        let mut cmd = Command::new("ivm-exec");
        // cmd.stdout(std::process::Stdio::piped()).stderr(std::process::Stdio::inherit());

        let datadir =
            tempfile::Builder::new().prefix("ivm-exec-instance-datadir").tempdir().unwrap();

        cmd.arg("node");
        // Dev node that allows txs from anyone
        cmd.arg("--dev").arg("--tx-allow.all");
        // 200ms block times
        cmd.arg("--dev.block-time").arg("200ms");
        cmd.arg("--datadir").arg(datadir.into_path());
        // Enable WS and HTTP rpc endpoints
        cmd.arg("--http").arg("--ws");
        // Explicitly enable most of the HTTP rpc modules
        cmd.arg("--http.api ").arg("admin,debug,eth,net,trace,txpool,web3,rpc,reth");
        // Set the port
        cmd.arg("-p").arg(port.to_string());

        let mut child = cmd.spawn().wrap_err(
            "failed to spawn ivm-exec. do you you have ivm-exec installed an in your path?",
        )?;
        Ok(Self { port, child })
    }

    /// Returns the port of this instance
    pub const fn port(&self) -> u16 {
        self.port
    }

    /// Returns the HTTP endpoint of this instance
    #[doc(alias = "http_endpoint")]
    pub fn endpoint(&self) -> String {
        format!("http://localhost:{}", self.port)
    }

    /// Returns the Websocket endpoint of this instance
    pub fn ws_endpoint(&self) -> String {
        format!("ws://localhost:{}", self.port)
    }
}

impl Drop for IvmExecInstance {
    fn drop(&mut self) {
        self.child.kill().expect("could not kill ivm-exec");
    }
}

pub struct IvmExecJobManager {
    /// ivm-exec http instance
    pub http_endpoint: String,
    /// ivm-exec ws instance
    pub ws_endpoint: String,
    /// Address of the job manager contract
    pub job_manager: Address,
    /// Relayer private key
    pub relayer: PrivateKeySigner,
    /// Coprocessor operator private key
    pub coprocessor_operator: PrivateKeySigner,
}

// pub async fn ivm_exec_with_job_manager(port: u16) ->

/// Get the first `count` of the signers based on the reth dev seed.
pub fn get_signers(count: usize) -> Vec<PrivateKeySigner> {
    Wallet::new(count)
        .gen()
        .into_iter()
        .map(|w| w.to_bytes().0)
        .map(|b| PrivateKeySigner::from_slice(&b).unwrap())
        .collect()
}

/// Get the `num` generated dev account.
pub fn get_account(num: usize) -> PrivateKeySigner {
    let all_wallets = get_signers(num + 1);
    all_wallets[num].clone()
}

/// Job Manager deployment info
#[derive(Debug)]
pub struct JobManagerDeploy {
    /// Anvil instance
    pub rpc_url: String,
    /// Address of the job manager contract
    pub job_manager: Address,
    /// Relayer private key
    pub relayer: PrivateKeySigner,
    /// Coprocessor operator private key
    pub coprocessor_operator: PrivateKeySigner,
}

impl JobManagerDeploy {
    /// Convenience method to convert into `AnvilJobManager`
    pub fn into_anvil_job_manager(self, anvil: AnvilInstance) -> AnvilJobManager {
        AnvilJobManager {
            anvil,
            job_manager: self.job_manager,
            relayer: self.relayer,
            coprocessor_operator: self.coprocessor_operator,
        }
    }
}

/// Deploy `JobManager` contract.
pub async fn job_manager_deploy(rpc_url: String) -> JobManagerDeploy {
    let signers = get_signers(5);

    let initial_owner = signers[0].clone();
    let relayer = signers[1].clone();
    let coprocessor_operator = signers[2].clone();
    let proxy_admin = signers[3].clone();

    let initial_owner_wallet = EthereumWallet::from(initial_owner.clone());

    let provider = ProviderBuilder::new()
        .wallet(initial_owner_wallet.clone())
        .on_http(rpc_url.parse().unwrap());

    let _ = provider.anvil_set_auto_mine(true).await;

    // Deploy the JobManager implementation contract
    let job_manager_implementation = JobManager::deploy(&provider).await.unwrap();

    // initializeJobManager will be called later when we deploy the proxy
    let initializer = job_manager_implementation.initializeJobManager(
        initial_owner.address(),
        relayer.address(),
        coprocessor_operator.address(),
    );
    let initializer_calldata = initializer.calldata();

    // Deploy a proxy contract for JobManager
    let proxy = TransparentUpgradeableProxy::deploy(
        &provider,
        *job_manager_implementation.address(),
        proxy_admin.address(),
        initializer_calldata.clone(),
    )
    .await
    .unwrap();

    let job_manager = *proxy.address();

    JobManagerDeploy { rpc_url, job_manager, relayer, coprocessor_operator }
}

/// Create and sign an ABI-encoded offchain job request.
#[allow(clippy::too_many_arguments)]
pub async fn create_and_sign_offchain_request(
    nonce: u64,
    max_cycles: u64,
    consumer_addr: Address,
    onchain_input: &[u8],
    program_id: &[u8],
    offchain_signer: PrivateKeySigner,
    offchain_input: &[u8],
) -> (Vec<u8>, Vec<u8>) {
    let job_params = JobParams {
        nonce,
        max_cycles,
        // Need to use abi_encode_packed because the contract address
        // should not be zero-padded
        consumer_address: Address::abi_encode_packed(&consumer_addr)
            .try_into()
            .expect("Valid consumer address"),
        onchain_input,
        program_id,
        offchain_input_hash: *keccak256(offchain_input),
    };

    let encoded_job_request = abi_encode_offchain_job_request(job_params);

    let signature =
        offchain_signer.sign_message(&encoded_job_request).await.expect("Signing should work");

    (encoded_job_request, signature.as_bytes().to_vec())
}

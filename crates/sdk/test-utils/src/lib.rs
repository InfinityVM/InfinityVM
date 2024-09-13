//! Utilities for setting up tests.

use std::net::TcpListener;

use alloy::{
    network::EthereumWallet,
    node_bindings::{Anvil, AnvilInstance},
    primitives::Address,
    providers::{ext::AnvilApi, ProviderBuilder},
    signers::local::PrivateKeySigner,
};
use contracts::{
    job_manager::JobManager, transparent_upgradeable_proxy::TransparentUpgradeableProxy,
};
use rand::Rng;
use tokio::time::{sleep, Duration};
use tracing_subscriber::EnvFilter;
use crate::wallet::Wallet;
use alloy::{
    signers::{k256::ecdsa::SigningKey, local::LocalSigner},
};

pub mod wallet;

type K256LocalSigner = LocalSigner<SigningKey>;

/// Max cycles that the `MockContract` calls create job with.
pub const MOCK_CONTRACT_MAX_CYCLES: u64 = 1_000_000;

/// Localhost IP address
pub const LOCALHOST: &str = "127.0.0.1";

/// Kill [`std::process::Child`] on `drop`
#[derive(Debug)]
pub struct ProcKill(std::process::Child);

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

/// Initialize a tracing subscriber for tests. Use `RUSTLOG` to set the filter level.
/// If the tracing subscriber has already been initialized in a previous test, this
/// function will silently fail due to `try_init()`, which does not reinitialize
/// the subscriber if one is already set.
pub fn test_tracing() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .with_writer(std::io::stderr)
        .try_init();
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
    for _ in 0..16 {
        if TcpListener::bind((LOCALHOST, port)).is_err() {
            return;
        }

        sleep(Duration::from_secs(1)).await;
    }

    panic!("localhost:{port} was not successfully bound");
}

/// Output from [`anvil_with_job_manager`]
#[derive(Debug)]
pub struct AnvilJobManager {
    /// Anvil instance
    pub rpc_url: AnvilInstance,
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
    // Set block time to 0.01 seconds - I WANNA GO FAST MOM
    let anvil = Anvil::new().block_time_f64(0.01).port(port).try_spawn().unwrap();

    let job_manager_deploy = job_manager(anvil.endpoint()).await;

    AnvilJobManager { anvil, job_manager: job_manager_deploy.job_manager, relayer: job_manager_deploy.relayer, coprocessor_operator: job_manager_deploy.coprocessor_operator }
}

pub fn get_signers(count: usize) -> Vec<K256LocalSigner> {
    Wallet::new(count)
        .gen()
        .into_iter()
        .map(|w| w.to_bytes().0)
        .map(|b| K256LocalSigner::from_slice(&b).unwrap())
        .collect()
}

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

pub async fn job_manager(rpc_url: String) -> JobManagerDeploy {
    let signers = get_signers(5);

    let initial_owner = signers[0].clone();
    let relayer = signers[1].clone();
    let coprocessor_operator = signers[2].clone();
    let proxy_admin = signers[3].clone();

    let initial_owner_wallet = EthereumWallet::from(initial_owner.clone());

    let provider = ProviderBuilder::new()
        .with_recommended_fillers()
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

    JobManagerDeploy { rpc_url,  job_manager, relayer, coprocessor_operator }
}
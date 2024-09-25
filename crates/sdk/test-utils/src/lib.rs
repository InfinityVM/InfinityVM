//! Utilities for setting up tests.

use std::net::TcpListener;

use crate::wallet::Wallet;
use alloy::{
    network::EthereumWallet,
    node_bindings::{Anvil, AnvilInstance},
    primitives::Address,
    providers::{ext::AnvilApi, ProviderBuilder},
    signers::{
        k256::ecdsa::SigningKey,
        local::{LocalSigner, PrivateKeySigner},
    },
};
use contracts::{
    job_manager::JobManager, transparent_upgradeable_proxy::TransparentUpgradeableProxy,
};
use rand::Rng;
use tokio::time::{sleep, Duration};
use tracing_subscriber::{filter::LevelFilter, EnvFilter};

pub mod wallet;

type K256LocalSigner = LocalSigner<SigningKey>;

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
    // Set block time to 0.01 seconds - I WANNA GO FAST MOM
    let anvil = Anvil::new()
        .block_time_f64(0.01)
        .port(port)
        // 1000 dev accounts generated and configured
        .args(["-a", "1000"])
        .try_spawn()
        .unwrap();

    let job_manager_deploy = job_manager_deploy(anvil.endpoint()).await;

    job_manager_deploy.into_anvil_job_manager(anvil)
}

/// Get the first `count` of the signers based on the reth dev seed.
pub fn get_signers(count: usize) -> Vec<K256LocalSigner> {
    Wallet::new(count)
        .gen()
        .into_iter()
        .map(|w| w.to_bytes().0)
        .map(|b| K256LocalSigner::from_slice(&b).unwrap())
        .collect()
}

/// Get the `num` generated dev account.
pub fn get_account(num: usize) -> K256LocalSigner {
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

    JobManagerDeploy { rpc_url, job_manager, relayer, coprocessor_operator }
}

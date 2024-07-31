//! Utilities for setting up tests.

use crate::contracts::{
    job_manager::JobManager, transparent_upgradeable_proxy::TransparentUpgradeableProxy,
};
use alloy::{
    network::EthereumWallet,
    node_bindings::{Anvil, AnvilInstance},
    primitives::Address,
    providers::ProviderBuilder,
    signers::local::PrivateKeySigner,
};

/// Output from [`anvil_with_contracts`]
#[derive(Debug)]
pub struct TestAnvil {
    /// Anvil instance
    pub anvil: AnvilInstance,
    /// Address of the job manager contract
    pub job_manager: Address,
    /// Relayer private key
    pub relayer: PrivateKeySigner,
    /// Coprocessor operator private key
    pub coprocessor_operator: PrivateKeySigner,
}

/// Setup an anvil instance with job manager contracts
/// let `TestAnvil` { anvil, `job_manager`, relayer, `coprocessor_operator` } =
// anvil_with_contracts();
pub async fn anvil_with_contracts() -> TestAnvil {
    let anvil = Anvil::new().try_spawn().unwrap();

    let initial_owner: PrivateKeySigner = anvil.keys()[0].clone().into();
    let relayer: PrivateKeySigner = anvil.keys()[1].clone().into();
    let coprocessor_operator: PrivateKeySigner = anvil.keys()[2].clone().into();
    let proxy_admin: PrivateKeySigner = anvil.keys()[3].clone().into();

    let initial_owner_wallet = EthereumWallet::from(initial_owner.clone());
    // Create a provider with the wallet.
    let rpc_url = anvil.endpoint();
    let provider = ProviderBuilder::new()
        .with_recommended_fillers()
        .wallet(initial_owner_wallet.clone())
        .on_http(rpc_url.parse().unwrap());

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

    TestAnvil { anvil, job_manager, relayer, coprocessor_operator }
}

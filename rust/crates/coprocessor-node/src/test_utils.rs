//! Utilities for setting up tests.

use crate::contracts::{
    job_manager::JobManager, mock_consumer::MockConsumer,
    transparent_upgradeable_proxy::TransparentUpgradeableProxy,
};
use alloy::{
    network::EthereumWallet,
    node_bindings::{Anvil, AnvilInstance},
    primitives::Address,
    providers::{ext::AnvilApi, ProviderBuilder},
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
    /// Address of the mock consumer contract
    pub mock_consumer: Address,
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
    let consumer_owner: PrivateKeySigner = anvil.keys()[4].clone().into();

    let initial_owner_wallet = EthereumWallet::from(initial_owner.clone());
    let consumer_owner_wallet = EthereumWallet::from(consumer_owner.clone());
    // Create a provider with the wallet.
    let rpc_url = anvil.endpoint();
    let provider = ProviderBuilder::new()
        .with_recommended_fillers()
        .wallet(initial_owner_wallet.clone())
        .on_http(rpc_url.parse().unwrap());

    // configure anvil over rpc
    // note: we can also configure the url in the future
    provider.anvil_set_logging(true).await.unwrap();
    provider.anvil_set_auto_mine(true).await.unwrap();

    // Deploy the JobManager implementation contract
    let job_manager_implementation = JobManager::deploy(&provider).await.unwrap();
    provider.evm_mine(None).await.unwrap();

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
    provider.evm_mine(None).await.unwrap();

    let job_manager = *proxy.address();

    let consumer_provider = ProviderBuilder::new()
        .with_recommended_fillers()
        .wallet(consumer_owner_wallet)
        .on_http(rpc_url.parse().unwrap());

    let mock_consumer = MockConsumer::deploy(consumer_provider, job_manager.clone()).await.unwrap();
    let mock_consumer = *mock_consumer.address();
    provider.evm_mine(None).await.unwrap();

    TestAnvil { anvil, job_manager, relayer, coprocessor_operator, mock_consumer }
}

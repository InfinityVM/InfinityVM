use alloy::{
    network::EthereumWallet,
    node_bindings::{Anvil, AnvilInstance},
    primitives::Address,
    providers::{ext::AnvilApi, ProviderBuilder},
    signers::local::PrivateKeySigner,
};
use clob_contracts::clob_consumer::ClobConsumer;
use contracts::{
    job_manager::JobManager, transparent_upgradeable_proxy::TransparentUpgradeableProxy,
};
use test_utils::get_localhost_port;

/// `MockERC20.sol` bindings
pub mod erc20 {
    #![allow(clippy::all, missing_docs)]
    alloy::sol! {
      /// Mock ERC20
      #[sol(rpc)]
      Erc20,
      "../contracts/out/MockERC20.sol/MockERC20.json"
    }
}

/// Output from [anvil_with_job_manager]
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

/// Output form [anvil_with_clob_consumer]
#[derive(Debug)]
pub struct AnvilClob {
    /// Anvil instance
    pub anvil: AnvilInstance,
    /// Address of the job manager contract
    pub job_manager: Address,
    /// Relayer private key
    pub relayer: PrivateKeySigner,
    /// Coprocessor operator private key
    pub coprocessor_operator: PrivateKeySigner,
    /// Offchain signer for clob.
    pub clob_signer: PrivateKeySigner,
    /// Address of the clob consumer contract
    pub clob_consumer: Address,
    /// Address of quote asset erc20
    pub quote_erc20: Address,
    /// Address of base asset erc20
    pub base_erc20: Address,
}

/// Spin up anvil instance with job manager contracts.
// TODO: DRY this with stuff in test utils crate.
pub async fn anvil_with_job_manager() -> AnvilJobManager {
    // Ensure the anvil instance will not collide with anything already running on the OS
    let port = get_localhost_port();
    // Set block time to 0.01 seconds - I WANNA GO FAST MOM
    let anvil = Anvil::new().block_time_f64(0.01).port(port).try_spawn().unwrap();

    let initial_owner: PrivateKeySigner = anvil.keys()[0].clone().into();
    let relayer: PrivateKeySigner = anvil.keys()[1].clone().into();
    let coprocessor_operator: PrivateKeySigner = anvil.keys()[2].clone().into();
    let proxy_admin: PrivateKeySigner = anvil.keys()[3].clone().into();

    let initial_owner_wallet = EthereumWallet::from(initial_owner.clone());

    let rpc_url = anvil.endpoint();
    let provider = ProviderBuilder::new()
        .with_recommended_fillers()
        .wallet(initial_owner_wallet.clone())
        .on_http(rpc_url.parse().unwrap());

    provider.anvil_set_auto_mine(true).await.unwrap();

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

    AnvilJobManager { anvil, job_manager, relayer, coprocessor_operator }
}

/// Spin up an anvil instance with job manager and clob consumer contracts.
pub async fn anvil_with_clob_consumer() -> AnvilClob {
    let AnvilJobManager { anvil, job_manager, relayer, coprocessor_operator } =
        anvil_with_job_manager().await;

    let consumer_owner: PrivateKeySigner = anvil.keys()[4].clone().into();
    let clob_signer: PrivateKeySigner = anvil.keys()[5].clone().into();

    let consumer_owner_wallet = EthereumWallet::from(consumer_owner.clone());

    let rpc_url = anvil.endpoint();
    let provider = ProviderBuilder::new()
        .with_recommended_fillers()
        .wallet(consumer_owner_wallet)
        .on_http(rpc_url.parse().unwrap());

    // Deploy 2 erc20s
    let base_erc20 = *erc20::Erc20::deploy(&provider).await.unwrap().address();
    let quote_erc20 = *erc20::Erc20::deploy(&provider).await.unwrap().address();

    // Deploy the clob consumer
    let clob_consumer = *ClobConsumer::deploy(
        provider,
        job_manager.clone(),
        clob_signer.address(),
        base_erc20.clone(),
        quote_erc20.clone(),
    )
    .await
    .unwrap()
    .address();

    AnvilClob {
        anvil,
        job_manager,
        relayer,
        coprocessor_operator,
        clob_signer,
        clob_consumer,
        quote_erc20,
        base_erc20,
    }
}

//! High level test utilities specifically for the matching game.

use crate::contracts::matching_game_consumer::MatchingGameConsumer;
use alloy::{
    network::EthereumWallet, primitives::Address, providers::ProviderBuilder,
    signers::local::PrivateKeySigner,
};
use ivm_contracts::{
    proxy_admin::ProxyAdmin, transparent_upgradeable_proxy::TransparentUpgradeableProxy,
};
use ivm_test_utils::{get_signers, AnvilJobManager};

/// Output form [`anvil_with_matching_game_consumer`]
#[derive(Debug)]
pub struct AnvilMatchingGame {
    /// Offchain signer for matching game.
    pub matching_game_signer: PrivateKeySigner,
    /// Address of the matching game consumer contract
    pub matching_game_consumer: Address,
}

/// Deploy `MatchingGameConsumer` to anvil instance.
pub async fn anvil_with_matching_game_consumer(anvil: &AnvilJobManager) -> AnvilMatchingGame {
    let AnvilJobManager { anvil, job_manager, .. } = anvil;
    matching_game_consumer_deploy(anvil.endpoint(), job_manager).await
}

/// Deploy matching game consumer contracts.
pub async fn matching_game_consumer_deploy(
    rpc_url: String,
    job_manager: &Address,
) -> AnvilMatchingGame {
    let signers = get_signers(6);

    let consumer_owner: PrivateKeySigner = signers[4].clone();
    let matching_game_signer: PrivateKeySigner = signers[5].clone();

    let consumer_owner_wallet = EthereumWallet::from(consumer_owner.clone());

    let provider =
        ProviderBuilder::new().wallet(consumer_owner_wallet).on_http(rpc_url.parse().unwrap());

    let init_state_hash: [u8; 32] = Default::default();

    // Deploy matching game consumer implementation
    let matching_game_consumer_impl = MatchingGameConsumer::deploy(provider.clone()).await.unwrap();

    // Deploy proxy admin
    let proxy_admin = ProxyAdmin::deploy(provider.clone()).await.unwrap();

    let initializer = matching_game_consumer_impl.initialize_2(
        consumer_owner.address(),
        *job_manager,
        0,
        init_state_hash.into(),
        matching_game_signer.address(),
    );
    let initializer_calldata = initializer.calldata();

    // Deploy a proxy contract for MatchingGameConsumer
    let matching_game_consumer = TransparentUpgradeableProxy::deploy(
        &provider,
        *matching_game_consumer_impl.address(),
        *proxy_admin.address(),
        initializer_calldata.clone(),
    )
    .await
    .unwrap();

    AnvilMatchingGame {
        matching_game_signer,
        matching_game_consumer: *matching_game_consumer.address(),
    }
}

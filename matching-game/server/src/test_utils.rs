//! High level test utilities specifically for the matching game.

use matching_game_core::{api::Request, tick};

use alloy::{
    network::EthereumWallet,
    primitives::Address,
    providers::ProviderBuilder,
    signers::{
        k256::ecdsa::SigningKey,
        local::{LocalSigner, PrivateKeySigner},
    },
};
use crate::contracts::matching_game_consumer::MatchingGameConsumer;
use test_utils::{get_signers, AnvilJobManager};

/// Local Signer
pub type K256LocalSigner = LocalSigner<SigningKey>;

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

    let provider = ProviderBuilder::new()
        .with_recommended_fillers()
        .wallet(consumer_owner_wallet)
        .on_http(rpc_url.parse().unwrap());

    let init_state_hash: [u8; 32] = Default::default();

    // Deploy the matching game consumer
    let matching_game_consumer = *MatchingGameConsumer::deploy(
        provider,
        *job_manager,
        matching_game_signer.address(),
        0,
        init_state_hash.into(),
    )
    .await
    .unwrap()
    .address();

    AnvilMatchingGame { matching_game_signer, matching_game_consumer }
}

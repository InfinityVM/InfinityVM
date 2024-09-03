//! Utilities for setting up tests.


use alloy::{
    network::EthereumWallet,
    primitives::Address,
    providers::ProviderBuilder,
    signers::local::PrivateKeySigner,
};
use contracts::mock_consumer::MockConsumer;
use test_utils::AnvilJobManager;

/// Output from [`anvil_with_mock_consumer`]
#[derive(Debug)]
pub struct AnvilMockConsumer {
    /// Address of the mock consumer contract
    pub mock_consumer: Address,
}

/// Deploy `MockConsumer` contracts to anvil instance
pub async fn anvil_with_mock_consumer(anvil_job_manager: &AnvilJobManager) -> AnvilMockConsumer {
    let AnvilJobManager { anvil, job_manager, .. } = anvil_job_manager;

    let consumer_owner: PrivateKeySigner = anvil.keys()[4].clone().into();
    let offchain_signer: PrivateKeySigner = anvil.keys()[5].clone().into();

    let consumer_owner_wallet = EthereumWallet::from(consumer_owner.clone());

    let rpc_url = anvil.endpoint();
    let consumer_provider = ProviderBuilder::new()
        .with_recommended_fillers()
        .wallet(consumer_owner_wallet)
        .on_http(rpc_url.parse().unwrap());

    let initial_max_nonce = 0;
    let mock_consumer = MockConsumer::deploy(
        consumer_provider,
        *job_manager,
        offchain_signer.address(),
        initial_max_nonce,
    )
    .await
    .unwrap();
    let mock_consumer = *mock_consumer.address();

    AnvilMockConsumer { mock_consumer }
}

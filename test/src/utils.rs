use alloy::{
    network::EthereumWallet, node_bindings::AnvilInstance, primitives::Address,
    providers::ProviderBuilder, signers::local::PrivateKeySigner,
};
use clob_contracts::clob_consumer::ClobConsumer;

use test_utils::{anvil_with_job_manager, AnvilJobManager};

/// `ERC20.sol` bindings
pub mod erc20 {
    #![allow(clippy::all, missing_docs)]
    alloy::sol! {
      /// ERC20
      #[sol(rpc)]
      Erc20,
      "../contracts/out/ERC20.sol/ERC20.json"
    }
}

/// Output form [`anvil_with_clob_consumer`]
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
    let base_name = "base".to_string();
    let base_symbol = "BASE".to_string();
    let base_erc20 =
        *erc20::Erc20::deploy(&provider, base_name, base_symbol).await.unwrap().address();

    let quote_name = "quote".to_string();
    let quote_symbol = "QUOTE".to_string();
    let quote_erc20 =
        *erc20::Erc20::deploy(&provider, quote_name, quote_symbol).await.unwrap().address();

    // Deploy the clob consumer
    let clob_consumer = *ClobConsumer::deploy(
        provider,
        job_manager,
        clob_signer.address(),
        0,
        base_erc20,
        quote_erc20,
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

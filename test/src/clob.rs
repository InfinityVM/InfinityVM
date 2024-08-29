use alloy::{
    network::EthereumWallet, primitives::Address, providers::ProviderBuilder,
    signers::local::PrivateKeySigner,
};
use clob_contracts::clob_consumer::ClobConsumer;
use clob_core::ClobState;
use clob_node::ClobStateResponse;
use serde::de::DeserializeOwned;
use serde::Serialize;

use test_utils::AnvilJobManager;

/// Make a POST request with JSON.
pub async fn post<Req: Serialize, Resp: DeserializeOwned>(url: &str, req: Req) -> Resp {
    reqwest::Client::new().post(url).json(&req).send().await.unwrap().json().await.unwrap()
}


/// Get the ClobState/
pub async fn get_state(url: &str) -> ClobState {
    let response: ClobStateResponse =
        reqwest::Client::new().get(url).send().await.unwrap().json().await.unwrap();

    let borsh = alloy::hex::decode(&response.borsh_hex_clob_state).unwrap();

    borsh::from_slice(&borsh).unwrap()
}

/// `E2EMockERC20.sol` bindings
pub mod mock_erc20 {
    #![allow(clippy::all, missing_docs)]
    alloy::sol! {
        /// MockERC20
        #[sol(rpc)]
        MockErc20,
        "../contracts/out/E2EMockERC20.sol/E2EMockERC20.json"
    }
}

/// Output form [`anvil_with_clob_consumer`]
#[derive(Debug)]
pub struct AnvilClob {
    /// Offchain signer for clob.
    pub clob_signer: PrivateKeySigner,
    /// Address of the clob consumer contract
    pub clob_consumer: Address,
    /// Address of quote asset erc20
    pub quote_erc20: Address,
    /// Address of base asset erc20
    pub base_erc20: Address,
}

/// Deploy `ClobConsumer` to anvil instance.
pub async fn anvil_with_clob_consumer(anvil: &AnvilJobManager) -> AnvilClob {
    let AnvilJobManager { anvil, job_manager, .. } = anvil;

    let consumer_owner: PrivateKeySigner = anvil.keys()[4].clone().into();
    let clob_signer: PrivateKeySigner = anvil.keys()[5].clone().into();

    let consumer_owner_wallet = EthereumWallet::from(consumer_owner.clone());

    let rpc_url = anvil.endpoint();
    let provider = ProviderBuilder::new()
        .with_recommended_fillers()
        .wallet(consumer_owner_wallet)
        .on_http(rpc_url.parse().unwrap());

    // Deploy base & quote erc20s
    let base_name = "base".to_string();
    let base_symbol = "BASE".to_string();
    let base_erc20 =
        *mock_erc20::MockErc20::deploy(&provider, base_name, base_symbol).await.unwrap().address();

    let quote_name = "quote".to_string();
    let quote_symbol = "QUOTE".to_string();
    let quote_erc20 = *mock_erc20::MockErc20::deploy(&provider, quote_name, quote_symbol)
        .await
        .unwrap()
        .address();

    // Deploy the clob consumer
    let clob_consumer = *ClobConsumer::deploy(
        provider,
        *job_manager,
        clob_signer.address(),
        0,
        base_erc20,
        quote_erc20,
    )
    .await
    .unwrap()
    .address();

    AnvilClob { clob_signer, clob_consumer, quote_erc20, base_erc20 }
}

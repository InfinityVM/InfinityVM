//! HTTP client for the CLOB node.

use crate::app::{ClobStateResponse, CANCEL, CLOB_STATE, ORDERS, WITHDRAW};
use clob_core::{
    api::{
        AddOrderRequest, AddOrderResponse, ApiResponse, CancelOrderRequest, CancelOrderResponse,
        Response, WithdrawRequest, WithdrawResponse,
    },
    ClobState,
};
use serde::{de::DeserializeOwned, Serialize};

/// CLOB Node client.
#[derive(Debug, Clone)]
pub struct Client {
    base_url: String,
}

impl Client {
    /// Create a new [Self].
    pub const fn new(base_url: String) -> Self {
        Self { base_url }
    }

    /// Post a cancel order request.
    pub async fn cancel(&self, req: CancelOrderRequest) -> (CancelOrderResponse, u64) {
        let url = self.path(CANCEL);
        let api_resp: ApiResponse = post(&url, req).await;
        let resp = match api_resp.response {
            Response::CancelOrder(resp) => resp,
            _ => panic!("unexpected response"),
        };

        (resp, api_resp.global_index)
    }

    /// Get the full CLOB state.
    pub async fn clob_state(&self) -> ClobState {
        let url = self.path(CLOB_STATE);
        get_state(&url).await
    }

    /// Post an add order request.
    pub async fn order(&self, req: AddOrderRequest) -> (AddOrderResponse, u64) {
        let url = self.path(ORDERS);
        let api_resp: ApiResponse = post(&url, req).await;
        let resp = match api_resp.response {
            Response::AddOrder(resp) => resp,
            _ => panic!("unexpected response"),
        };

        (resp, api_resp.global_index)
    }

    /// Post withdraw request.
    pub async fn withdraw(&self, req: WithdrawRequest) -> (WithdrawResponse, u64) {
        let url = self.path(WITHDRAW);
        let api_resp: ApiResponse = post(&url, req).await;
        let resp = match api_resp.response {
            Response::Withdraw(resp) => resp,
            _ => panic!("unexpected response"),
        };

        (resp, api_resp.global_index)
    }

    fn path(&self, route: &str) -> String {
        format!("{}{route}", self.base_url)
    }
}

/// Make a POST request with JSON.
async fn post<Req: Serialize, Resp: DeserializeOwned>(url: &str, req: Req) -> Resp {
    reqwest::Client::new().post(url).json(&req).send().await.unwrap().json().await.unwrap()
}

/// Get the `ClobState`.
async fn get_state(url: &str) -> ClobState {
    let response: ClobStateResponse =
        reqwest::Client::new().get(url).send().await.unwrap().json().await.unwrap();

    let borsh = alloy::hex::decode(&response.borsh_hex_clob_state).unwrap();

    borsh::from_slice(&borsh).unwrap()
}

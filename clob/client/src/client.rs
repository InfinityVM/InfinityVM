//! HTTP client for the CLOB node.

use clob_core::{
    api::{
        AddOrderRequest, AddOrderResponse, ApiResponse, CancelOrderRequest, CancelOrderResponse,
        Response, WithdrawRequest, WithdrawResponse,
    },
    ClobState,
};
use clob_node::app::{ClobStateResponse, CANCEL, CLOB_STATE, ORDERS, WITHDRAW};
use eyre::bail;
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
    pub async fn cancel(
        &self,
        req: CancelOrderRequest,
    ) -> eyre::Result<(CancelOrderResponse, u64)> {
        let url = self.path(CANCEL);
        let api_resp: ApiResponse = post(&url, req).await;
        let resp = match api_resp.response {
            Response::CancelOrder(resp) => resp,
            _ => bail!("unexpected api response"),
        };

        Ok((resp, api_resp.global_index))
    }

    /// Get the full CLOB state.
    pub async fn clob_state(&self) -> eyre::Result<ClobState> {
        let url = self.path(CLOB_STATE);
        get_state(&url).await
    }

    /// Post an add order request.
    pub async fn order(&self, req: AddOrderRequest) -> eyre::Result<(AddOrderResponse, u64)> {
        let url = self.path(ORDERS);
        let api_resp: ApiResponse = post(&url, req).await;
        let resp = match api_resp.response {
            Response::AddOrder(resp) => resp,
            _ => bail!("unexpected api response"),
        };

        Ok((resp, api_resp.global_index))
    }

    /// Post withdraw request.
    pub async fn withdraw(&self, req: WithdrawRequest) -> eyre::Result<(WithdrawResponse, u64)> {
        let url = self.path(WITHDRAW);
        let api_resp: ApiResponse = post(&url, req).await;
        let resp = match api_resp.response {
            Response::Withdraw(resp) => resp,
            _ => bail!("unexpected api response"),
        };

        Ok((resp, api_resp.global_index))
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
async fn get_state(url: &str) -> eyre::Result<ClobState> {
    let response: ClobStateResponse = reqwest::Client::new()
        .get(url)
        .header("Content-Type", "application/json")
        .send()
        .await?
        .json()
        .await?;

    let borsh = alloy::hex::decode(&response.borsh_hex_clob_state)?;

    borsh::from_slice(&borsh).map_err(Into::into)
}

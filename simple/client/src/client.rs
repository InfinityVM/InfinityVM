//! HTTP client for the CLOB node.

use simple_core::{
    api::{
        SetValueRequest, SetValueResponse, Response,
    },
    SimpleState,
};
use simple_node::app::{AppResponse, SimpleStateResponse, SET_VALUE, SIMPLE_STATE};
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

    /// Set a Value
    pub async fn set_value(&self, req: SetValueRequest) -> eyre::Result<(SetValueResponse, u64)> {
        let url = self.path(SET_VALUE);
        let app_resp: AppResponse = post(&url, req).await;
        let api_resp = match app_resp {
            AppResponse::Success(r) => r,
            AppResponse::Failure(e) => bail!("unexpected app response: {e:?}"),
        };

        let resp = match api_resp.response {
            Response::SetValue(resp) => resp,
            x => bail!("unexpected api response: {x:?}"),
        };

        Ok((resp, api_resp.global_index))
    }

    pub async fn state(&self) -> eyre::Result<SimpleState> {
        let url = self.path(SIMPLE_STATE);
        println!("Getting state at {:?}", url);
        get_state(&url).await
    }

    fn path(&self, route: &str) -> String {
        format!("{}{route}", self.base_url)
    }
}

/// Make a POST request with JSON.
async fn post<Req: Serialize, Resp: DeserializeOwned>(url: &str, req: Req) -> Resp {
    reqwest::Client::new().post(url).json(&req).send().await.unwrap().json().await.unwrap()
}

/// Get the `State`.
async fn get_state(url: &str) -> eyre::Result<SimpleState> {
    let response: SimpleStateResponse = reqwest::Client::new().get(url).send().await?.json().await?;

    let value = response.value;

    Ok( 
        SimpleState{
            batch_num: 0,
            latest_value: value,
        }
    )
}

//! HTTP client for the matching game node.

use eyre::bail;
use matching_game_core::{
    api::{
        ApiResponse, CancelNumberRequest, CancelNumberResponse, Request, Response,
        SubmitNumberRequest, SubmitNumberResponse,
    },
    MatchingGameState,
};
use matching_game_node::app::{AppResponse, CANCEL, SUBMIT};
use serde::{de::DeserializeOwned, Serialize};

/// Matching game node client.
#[derive(Debug, Clone)]
pub struct Client {
    base_url: String,
}

impl Client {
    /// Create a new [Self].
    pub const fn new(base_url: String) -> Self {
        Self { base_url }
    }

    /// Post a submit number request.
    pub async fn submit_number(
        &self,
        req: SubmitNumberRequest,
    ) -> eyre::Result<(SubmitNumberResponse, u64)> {
        let url = self.path(SUBMIT);
        let app_resp: AppResponse = post(&url, req).await;
        let api_resp = match app_resp {
            AppResponse::Success(r) => r,
            AppResponse::Failure(e) => bail!("unexpected app response: {e:?}"),
        };

        let resp = match api_resp.response {
            Response::SubmitNumber(resp) => resp,
            x => bail!("unexpected api response: {x:?}"),
        };

        Ok((resp, api_resp.global_index))
    }
    /// Post a cancel number request.
    pub async fn cancel_number(
        &self,
        req: CancelNumberRequest,
    ) -> eyre::Result<(CancelNumberResponse, u64)> {
        let url = self.path(CANCEL);
        let app_resp: AppResponse = post(&url, req).await;
        let api_resp = match app_resp {
            AppResponse::Success(r) => r,
            AppResponse::Failure(e) => bail!("unexpected app response: {e:?}"),
        };

        let resp = match api_resp.response {
            Response::CancelNumber(resp) => resp,
            x => bail!("unexpected api response: {x:?}"),
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

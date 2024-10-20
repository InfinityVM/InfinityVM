//! CLOB application https request router.

use crate::{
    engine::GENESIS_GLOBAL_INDEX,
};
use axum::{
    extract::State as ExtractState,
    http::StatusCode,
    response::{IntoResponse, Response},
    Json, Router,
};
use simple_core::api::{
    SetValueRequest, SetValueResponse, ApiResponse, Request
};
use eyre::{eyre, WrapErr};
use reth_db::{transaction::DbTx, Database, DatabaseEnv};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::{mpsc::Sender, oneshot};
use tracing::{info, instrument};

/// SetValue URI.
pub const SET_VALUE: &str = "/set-value";

/// Stateful parts of REST server.
#[derive(Debug, Clone)]
pub struct AppState {
    /// Engine send channel handle.
    engine_sender: Sender<(Request, oneshot::Sender<ApiResponse>)>,
}

impl AppState {
    /// Create a new instance of [Self].
    pub const fn new(
        engine_sender: Sender<(Request, oneshot::Sender<ApiResponse>)>,
    ) -> Self {
        Self { engine_sender }
    }
}

fn app(state: AppState) -> Router {
    axum::Router::new()
        .route(SET_VALUE, axum::routing::post(set_value))
        .with_state(state)
}

/// Run the HTTP server.
pub async fn http_listen(state: AppState, listen_address: &str) -> eyre::Result<()> {
    let app = app(state);

    let listener = tokio::net::TcpListener::bind(listen_address).await?;
    axum::serve(listener, app).await.map_err(Into::into)
}

#[instrument(skip_all)]
async fn set_value(
    ExtractState(state): ExtractState<AppState>,
    Json(req): Json<SetValueRequest>,
) -> Result<AppResponse, AppResponse> {
    let (tx, rx) = oneshot::channel::<ApiResponse>();

    state
        .engine_sender
        .send((Request::SetValue(req), tx))
        .await
        .wrap_err("engine receive unexpectedly dropped")?;
    let resp = rx.await.wrap_err("engine oneshot sender unexpectedly dropped")?;
    info!(?resp);

    // TODO: write to batch?

    Ok(AppResponse::Success(resp))
}

/// Response type from most app endpoints. `eyre` Errors are automatically converted to
/// the `Failure` variant.
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub enum AppResponse {
    /// A successful response.
    Success(ApiResponse),
    /// An error response.
    Failure(String),
}

impl AppResponse {
    /// Get the [`ApiResponse`]. Panics if the response is not [`Self:: Success`]
    pub fn into_good(self) -> ApiResponse {
        match self {
            Self::Success(r) => r,
            Self::Failure(_) => panic!("unexpected error app response"),
        }
    }
}

// Tell axum how to convert `AppError` into a response.
impl IntoResponse for AppResponse {
    fn into_response(self) -> Response {
        match &self {
            Self::Success(_) => (StatusCode::OK, Json(self)).into_response(),
            Self::Failure(_) => (StatusCode::INTERNAL_SERVER_ERROR, Json(self)).into_response(),
        }
    }
}

impl<E> From<E> for AppResponse
where
    E: Into<eyre::Error>,
{
    fn from(err: E) -> Self {
        let e: eyre::Error = err.into();
        Self::Failure(e.to_string())
    }
}

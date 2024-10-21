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
use simple_core::{api::{
    ApiResponse, Request, SetValueRequest, SetValueResponse
}, SimpleState};
use eyre::{eyre, WrapErr};
use reth_db::{transaction::DbTx, Database, DatabaseEnv};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::{mpsc::Sender, oneshot, RwLock};
use tracing::{info, instrument};

/// SetValue URI.
pub const SET_VALUE: &str = "/set-value";

/// Simple state URI
pub const SIMPLE_STATE: &str = "/state";

/// Response to the simple state endpoint. This is just a temp hack until we have better view
/// endpoints.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SimpleStateResponse {
    /// Hex encoded borsh bytes.
    pub value: String,
}

/// Stateful parts of REST server.
#[derive(Debug, Clone)]
pub struct AppState {
    /// Simple State reference
    state: Arc<RwLock<SimpleState>>,
    /// Engine send channel handle.
    engine_sender: Sender<(Request, oneshot::Sender<ApiResponse>)>,
}

impl AppState {
    /// Create a new instance of [Self].
    pub const fn new(
        simple_state: Arc<RwLock<SimpleState>>,
        engine_sender: Sender<(Request, oneshot::Sender<ApiResponse>)>,
    ) -> Self {
        Self { state: simple_state, engine_sender }
    }
}

fn app(state: AppState) -> Router {
    axum::Router::new()
        .route(SET_VALUE, axum::routing::post(set_value))
        .route(SIMPLE_STATE, axum::routing::get(get_state))
        .with_state(state)
}

/// Run the HTTP server.
pub async fn http_listen(state: AppState, listen_address: &str) -> eyre::Result<()> {
    let app = app(state);

    let listener = tokio::net::TcpListener::bind(listen_address).await?;
    println!("Listening at {:?}", listen_address);
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

#[instrument(skip_all)]
async fn get_state(
    ExtractState(state): ExtractState<AppState>,
) -> Result<Json<SimpleStateResponse>, AppResponse> {
    println!("Getting state!");
    let read = state.state.read().await;
    let value = read.latest_value();
    println!("Found value {:?}", value);

    let response = SimpleStateResponse { value };

    Ok(Json(response))
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

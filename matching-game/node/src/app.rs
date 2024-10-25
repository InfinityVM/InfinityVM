//! CLOB application https request router.

use crate::{
    db::{
        tables::{MatchingGameStateTable, GlobalIndexTable},
        PROCESSED_GLOBAL_INDEX_KEY,
    },
    engine::GENESIS_GLOBAL_INDEX,
};
use axum::{
    extract::State as ExtractState,
    http::StatusCode,
    response::{IntoResponse, Response},
    Json, Router,
};
use matching_game_core::api::{
    SubmitNumberRequest, Response, CancelNumberRequest, Request,
};
use eyre::{eyre, WrapErr};
use reth_db::{transaction::DbTx, Database, DatabaseEnv};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::{mpsc::Sender, oneshot};
use tracing::{info, instrument};

/// Submit number URI.
pub const SUBMIT: &str = "/submit";
/// Cancel URI.
pub const CANCEL: &str = "/cancel";

/// Stateful parts of REST server.
#[derive(Debug, Clone)]
pub struct AppState {
    /// Engine send channel handle.
    engine_sender: Sender<(Request, oneshot::Sender<ApiResponse>)>,
    /// The database
    db: Arc<DatabaseEnv>,
}

impl AppState {
    /// Create a new instance of [Self].
    pub const fn new(
        engine_sender: Sender<(Request, oneshot::Sender<ApiResponse>)>,
        db: Arc<DatabaseEnv>,
    ) -> Self {
        Self { engine_sender, db }
    }
}

fn app(state: AppState) -> Router {
    axum::Router::new()
        .route(SUBMIT, axum::routing::post(submit))
        .route(CANCEL, axum::routing::post(cancel))
        .with_state(state)
}

/// Run the HTTP server.
pub async fn http_listen(state: AppState, listen_address: &str) -> eyre::Result<()> {
    let app = app(state);

    let listener = tokio::net::TcpListener::bind(listen_address).await?;
    axum::serve(listener, app).await.map_err(Into::into)
}

#[instrument(skip_all)]
async fn submit(
    ExtractState(state): ExtractState<AppState>,
    Json(req): Json<SubmitNumberRequest>,
) -> Result<AppResponse, AppResponse> {
    let (tx, rx) = oneshot::channel::<ApiResponse>();

    state
        .engine_sender
        .send((Request::SubmitNumber(req), tx))
        .await
        .wrap_err("engine receive unexpectedly dropped")?;
    let resp = rx.await.wrap_err("engine oneshot sender unexpectedly dropped")?;
    info!(?resp);

    Ok(AppResponse::Success(resp))
}

#[instrument(skip_all)]
async fn cancel(
    ExtractState(state): ExtractState<AppState>,
    Json(req): Json<CancelNumberRequest>,
) -> Result<AppResponse, AppResponse> {
    let (tx, rx) = oneshot::channel::<ApiResponse>();

    state
        .engine_sender
        .send((Request::CancelNumber(req), tx))
        .await
        .wrap_err("engine receive unexpectedly dropped")?;
    let resp = rx.await.wrap_err("engine oneshot sender unexpectedly dropped")?;
    info!(?resp);

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

//! The CLOB node.

use crate::{
    db::{
        tables::{ClobStateTable, GlobalIndexTable},
        PROCESSED_GLOBAL_INDEX_KEY,
    },
    engine::START_GLOBAL_INDEX,
};
use alloy::signers::{k256::ecdsa::SigningKey, local::LocalSigner};
use axum::{extract::State as ExtractState, Json, Router};
use clob_core::{
    api::{
        AddOrderRequest, ApiResponse, CancelOrderRequest, DepositRequest, Request, WithdrawRequest,
    },
    ClobState,
};
use reth_db::{transaction::DbTx, Database, DatabaseEnv};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::{mpsc::Sender, oneshot};
use tracing::{info, instrument};

pub mod batcher;
pub mod db;
pub mod engine;

/// Operator signer type.
pub type K256LocalSigner = LocalSigner<SigningKey>;

const DEPOSIT: &str = "/deposit";
const WITHDRAW: &str = "/withdraw";
const ORDERS: &str = "/orders";
const CANCEL: &str = "/cancel";
const CLOB_STATE: &str = "/clob-state";

///  Response to the clob state endpoint
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClobStateResponse {
    // Hex encoded borsh bytes. This is just a temp hack until we have better view endpoints
    borsh_hex_clob_state: String,
}

/// Stateful parts of REST server
#[derive(Debug, Clone)]
pub struct AppState {
    /// Engine send channel handle.
    engine_sender: Sender<(Request, oneshot::Sender<ApiResponse>)>,
    /// The database
    db: Arc<DatabaseEnv>,
}

impl AppState {
    /// Create a new instance of [Self].
    pub fn new(
        engine_sender: Sender<(Request, oneshot::Sender<ApiResponse>)>,
        db: Arc<DatabaseEnv>,
    ) -> Self {
        Self { engine_sender, db }
    }
}

fn app(state: AppState) -> Router {
    axum::Router::new()
        .route(DEPOSIT, axum::routing::post(deposit))
        .route(WITHDRAW, axum::routing::post(withdraw))
        .route(ORDERS, axum::routing::post(add_order))
        .route(CANCEL, axum::routing::post(cancel))
        .route(CLOB_STATE, axum::routing::get(clob_state))
        .with_state(state)
}

/// Run the HTTP server.
pub async fn http_listen(state: AppState, listen_address: &str) {
    let app = app(state);

    let listener = tokio::net::TcpListener::bind(listen_address).await.expect("TODO");
    axum::serve(listener, app).await.expect("TODO");
}

#[instrument(skip_all)]
async fn deposit(
    ExtractState(state): ExtractState<AppState>,
    Json(req): Json<DepositRequest>,
) -> Json<ApiResponse> {
    let (tx, rx) = oneshot::channel::<ApiResponse>();

    state.engine_sender.send((Request::Deposit(req), tx)).await.expect("todo");
    let resp = rx.await.expect("todo");
    info!(?resp);

    Json(resp)
}

#[instrument(skip_all)]
async fn withdraw(
    ExtractState(state): ExtractState<AppState>,
    Json(req): Json<WithdrawRequest>,
) -> Json<ApiResponse> {
    let (tx, rx) = oneshot::channel::<ApiResponse>();

    state.engine_sender.send((Request::Withdraw(req), tx)).await.expect("todo");
    let resp = rx.await.expect("todo");
    info!(?resp);

    Json(resp)
}

#[instrument(skip_all)]
async fn add_order(
    ExtractState(state): ExtractState<AppState>,
    Json(req): Json<AddOrderRequest>,
) -> Json<ApiResponse> {
    let (tx, rx) = oneshot::channel::<ApiResponse>();

    state.engine_sender.send((Request::AddOrder(req), tx)).await.expect("todo");
    let resp = rx.await.expect("todo");
    info!(?resp);

    Json(resp)
}

#[instrument(skip_all)]
async fn cancel(
    ExtractState(state): ExtractState<AppState>,
    Json(req): Json<CancelOrderRequest>,
) -> Json<ApiResponse> {
    let (tx, rx) = oneshot::channel::<ApiResponse>();

    state.engine_sender.send((Request::CancelOrder(req), tx)).await.expect("todo");
    let resp = rx.await.expect("todo");
    info!(?resp);

    Json(resp)
}

#[instrument(skip_all)]
async fn clob_state(ExtractState(state): ExtractState<AppState>) -> Json<ClobStateResponse> {
    let tx = state.db.tx().expect("todo");

    let global_index = tx
        .get::<GlobalIndexTable>(PROCESSED_GLOBAL_INDEX_KEY)
        .expect("todo: db errors")
        .unwrap_or(START_GLOBAL_INDEX);

    let clob_state = if global_index == START_GLOBAL_INDEX {
        ClobState::default()
    } else {
        tx.get::<ClobStateTable>(global_index)
            .expect("todo: db errors")
            .expect("todo: could not find state when some was expected")
            .0
    };
    tx.commit().expect("todo");

    let borsh = borsh::to_vec(&clob_state).unwrap();
    let response = ClobStateResponse { borsh_hex_clob_state: alloy::hex::encode(&borsh) };

    Json(response)
}

#[cfg(test)]
mod tests {
    // ref for testing: https://github.com/tokio-rs/axum/blob/main/examples/testing/src/main.rs
    use super::*;
    use axum::{
        body::Body,
        http::{self, Request as AxumRequest},
    };
    use clob_core::api::AssetBalance;
    use http_body_util::BodyExt;
    use tempfile::tempdir;
    use tower::{Service, ServiceExt};

    const CHANEL_SIZE: usize = 32;

    async fn test_setup() -> AppState {
        let dbdir = tempdir().unwrap();
        let db = Arc::new(crate::db::init_db(dbdir).unwrap());
        let (engine_sender, engine_receiver) = tokio::sync::mpsc::channel(CHANEL_SIZE);

        let server_state = AppState::new(engine_sender, Arc::clone(&db));

        tokio::spawn(async move { crate::engine::run_engine(engine_receiver, db).await });

        server_state
    }

    // POST `uri` with body `Req`, deserializing response into `Resp`.
    async fn post<Req, Resp>(app: &mut Router, uri: &str, req: Req) -> Resp
    where
        Req: serde::Serialize,
        Resp: serde::de::DeserializeOwned,
    {
        let body = Body::from(serde_json::to_vec(&req).unwrap());

        let request = AxumRequest::builder()
            .uri(uri)
            .header(http::header::CONTENT_TYPE, mime::APPLICATION_JSON.as_ref())
            .method(http::Method::POST)
            .body(body)
            .unwrap();

        let response =
            ServiceExt::<AxumRequest<Body>>::ready(app).await.unwrap().call(request).await.unwrap();
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        serde_json::from_slice(&bytes).unwrap()
    }

    // GET `uri`, deserializing response into `Resp`.
    async fn get<Resp>(app: &mut Router, uri: &str) -> Resp
    where
        Resp: serde::de::DeserializeOwned,
    {
        let request = AxumRequest::builder()
            .uri(uri)
            .header(http::header::CONTENT_TYPE, mime::APPLICATION_JSON.as_ref())
            .method(http::Method::GET)
            .body(Body::empty())
            .unwrap();

        let response =
            ServiceExt::<AxumRequest<Body>>::ready(app).await.unwrap().call(request).await.unwrap();
        let bytes = response.into_body().collect().await.unwrap().to_bytes();

        serde_json::from_slice(&bytes).unwrap()
    }

    // Get the clob state. This deals with the overhead of deserializing the hex(borsh(ClobState))
    // encoding.
    async fn get_clob_state(app: &mut Router) -> ClobState {
        let response: ClobStateResponse = get(app, CLOB_STATE).await;

        let borsh = alloy::hex::decode(&response.borsh_hex_clob_state).unwrap();
        borsh::from_slice(&borsh).unwrap()
    }

    // TODO: once we have good error handling, this won't panic
    #[should_panic]
    #[tokio::test]
    async fn cannot_place_bid_with_no_deposit() {
        let server_state = test_setup().await;
        let mut app = app(server_state);

        let _: ApiResponse = post(
            &mut app,
            ORDERS,
            AddOrderRequest { address: [0; 20], is_buy: true, limit_price: 2, size: 3 },
        )
        .await;
    }

    #[should_panic]
    #[tokio::test]
    async fn cannot_place_ask_with_no_deposit() {
        let server_state = test_setup().await;
        let mut app = app(server_state);

        let _: ApiResponse = post(
            &mut app,
            ORDERS,
            AddOrderRequest { address: [0; 20], is_buy: false, limit_price: 2, size: 3 },
        )
        .await;
    }

    #[should_panic]
    #[tokio::test]
    async fn cannot_withdraw_with_no_deposit() {
        let server_state = test_setup().await;
        let mut app = app(server_state);

        let _: ApiResponse = post(
            &mut app,
            ORDERS,
            AddOrderRequest { address: [0; 20], is_buy: false, limit_price: 2, size: 3 },
        )
        .await;
    }

    #[tokio::test]
    async fn place_bids() {
        tracing_subscriber::fmt()
            .event_format(tracing_subscriber::fmt::format().with_file(true).with_line_number(true))
            .init();

        let server_state = test_setup().await;
        let mut app = app(server_state);
        let user1 = [1; 20];
        let user2 = [2; 20];
        let user3 = [3; 20];

        let r: ApiResponse = post(
            &mut app,
            DEPOSIT,
            DepositRequest { address: user1, quote_free: 10, base_free: 0 },
        )
        .await;
        assert_eq!(r.global_index, 1);

        let r: ApiResponse = post(
            &mut app,
            DEPOSIT,
            DepositRequest { address: user2, quote_free: 20, base_free: 0 },
        )
        .await;
        assert_eq!(r.global_index, 2);

        let r: ApiResponse = post(
            &mut app,
            DEPOSIT,
            DepositRequest { address: user3, quote_free: 30, base_free: 0 },
        )
        .await;
        assert_eq!(r.global_index, 3);

        let state = get_clob_state(&mut app).await;
        assert_eq!(state.oid(), 0);
        assert_eq!(
            *state.quote_balances().get(&user1).unwrap(),
            AssetBalance { free: 10, locked: 0 }
        );
        assert_eq!(
            *state.quote_balances().get(&user2).unwrap(),
            AssetBalance { free: 20, locked: 0 }
        );
        assert_eq!(
            *state.quote_balances().get(&user3).unwrap(),
            AssetBalance { free: 30, locked: 0 }
        );
    }
}

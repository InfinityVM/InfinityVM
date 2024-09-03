//! The CLOB node.

use crate::{
    db::{
        tables::{ClobStateTable, GlobalIndexTable},
        PROCESSED_GLOBAL_INDEX_KEY,
    },
    engine::GENESIS_GLOBAL_INDEX,
};
use alloy::signers::{k256::ecdsa::SigningKey, local::LocalSigner};
use axum::{extract::State as ExtractState, Json, Router};
use clob_core::api::{
    AddOrderRequest, ApiResponse, CancelOrderRequest, DepositRequest, Request, WithdrawRequest,
};
use reth_db::{transaction::DbTx, Database, DatabaseEnv};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::{mpsc::Sender, oneshot};
use tracing::{info, instrument};

pub mod batcher;
pub mod client;
pub mod db;
pub mod engine;

/// Address to listen for HTTP requests on.
pub const CLOB_LISTEN_ADDR: &str = "CLOB_LISTEN_ADDR";
/// Directory for database.
pub const CLOB_DB_DIR: &str = "CLOB_DB_DIR";
/// Coprocessor Node gRPC address.
pub const CLOB_CN_GRPC_ADDR: &str = "CLOB_CN_GRPC_ADDR";
/// Execution Client HTTP address.
pub const CLOB_ETH_HTTP_ADDR: &str = "CLOB_ETH_HTTP_ADDR";
/// Clob Consumer contract address.
pub const CLOB_CONSUMER_ADDR: &str = "CLOB_CONSUMER_ADDR";
/// Duration between creating batches.
pub const CLOB_BATCHER_DURATION_MS: &str = "CLOB_BATCHER_DURATION_MS";
/// Clob operator's secret key.
pub const CLOB_OPERATOR_KEY: &str = "CLOB_OPERATOR_KEY";

/// Operator signer type.
pub type K256LocalSigner = LocalSigner<SigningKey>;

const DEPOSIT: &str = "/deposit";
const WITHDRAW: &str = "/withdraw";
const ORDERS: &str = "/orders";
const CANCEL: &str = "/cancel";
const CLOB_STATE: &str = "/clob-state";

/// Run the CLOB node.
pub async fn run(
    db_dir: String,
    listen_addr: String,
    batcher_duration_ms: u64,
    operator_signer: K256LocalSigner,
    cn_grpc_url: String,
    clob_consumer_addr: [u8; 20],
) {
    let db = crate::db::init_db(db_dir).expect("todo");
    let db = Arc::new(db);

    let (engine_sender, engine_receiver) = tokio::sync::mpsc::channel(32);
    let db2 = Arc::clone(&db);
    let server_handle = tokio::spawn(async move {
        let server_state = AppState::new(engine_sender, db2);
        http_listen(server_state, &listen_addr).await
    });

    let db2 = Arc::clone(&db);
    let engine_handle = tokio::spawn(async move { engine::run_engine(engine_receiver, db2).await });

    let batcher_handle = tokio::spawn(async move {
        let batcher_duration = tokio::time::Duration::from_millis(batcher_duration_ms);
        batcher::run_batcher(db, batcher_duration, operator_signer, cn_grpc_url, clob_consumer_addr)
            .await
    });

    tokio::try_join!(server_handle, engine_handle, batcher_handle).unwrap();
}

///  Response to the clob state endpoint. This is just a temp hack until we have better view
/// endpoints.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClobStateResponse {
    /// Hex encoded borsh bytes.
    pub borsh_hex_clob_state: String,
}

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
async fn http_listen(state: AppState, listen_address: &str) {
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
        .unwrap_or(GENESIS_GLOBAL_INDEX);

    let clob_state = tx
        .get::<ClobStateTable>(global_index)
        .expect("todo: db errors")
        .expect("todo: could not find state when some was expected")
        .0;
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
    use clob_core::{api::AssetBalance, ClobState};
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

        let r: ApiResponse = post(
            &mut app,
            ORDERS,
            AddOrderRequest { address: user1, is_buy: true, limit_price: 2, size: 4 },
        )
        .await;
        assert_eq!(r.global_index, 4);

        let r: ApiResponse = post(
            &mut app,
            ORDERS,
            AddOrderRequest { address: user2, is_buy: true, limit_price: 2, size: 8 },
        )
        .await;
        assert_eq!(r.global_index, 5);

        let r: ApiResponse = post(
            &mut app,
            ORDERS,
            AddOrderRequest { address: user3, is_buy: true, limit_price: 2, size: 12 },
        )
        .await;
        assert_eq!(r.global_index, 6);

        let state = get_clob_state(&mut app).await;
        assert_eq!(state.oid(), 3);
        assert_eq!(
            *state.quote_balances().get(&user1).unwrap(),
            AssetBalance { free: 2, locked: 8 }
        );
        assert_eq!(
            *state.quote_balances().get(&user2).unwrap(),
            AssetBalance { free: 4, locked: 16 }
        );
        assert_eq!(
            *state.quote_balances().get(&user3).unwrap(),
            AssetBalance { free: 6, locked: 24 }
        );
    }

    #[tokio::test]
    async fn deposit_order_withdraw_cancel() {
        tracing_subscriber::fmt()
            .event_format(tracing_subscriber::fmt::format().with_file(true).with_line_number(true))
            .init();

        let server_state = test_setup().await;
        let mut app = app(server_state);

        let bob = [69u8; 20];
        let alice = [42u8; 20];

        let alice_deposit = DepositRequest { address: alice, base_free: 200, quote_free: 0 };
        let bob_deposit = DepositRequest { address: bob, base_free: 0, quote_free: 800 };
        for r in [alice_deposit, bob_deposit] {
            let _: ApiResponse = post(&mut app, DEPOSIT, r).await;
        }
        let state = get_clob_state(&mut app).await;
        assert_eq!(
            *state.base_balances().get(&alice).unwrap(),
            AssetBalance { free: 200, locked: 0 }
        );
        assert_eq!(
            *state.quote_balances().get(&bob).unwrap(),
            AssetBalance { free: 800, locked: 0 }
        );

        let alice_limit =
            AddOrderRequest { address: alice, is_buy: false, limit_price: 4, size: 100 };
        let bob_limit1 = AddOrderRequest { address: bob, is_buy: true, limit_price: 1, size: 100 };
        let bob_limit2 = AddOrderRequest { address: bob, is_buy: true, limit_price: 4, size: 100 };
        for r in [alice_limit, bob_limit1, bob_limit2] {
            let _: ApiResponse = post(&mut app, ORDERS, r).await;
        }
        let state = get_clob_state(&mut app).await;
        assert_eq!(
            *state.base_balances().get(&alice).unwrap(),
            AssetBalance { free: 100, locked: 0 }
        );
        assert_eq!(
            *state.quote_balances().get(&alice).unwrap(),
            AssetBalance { free: 400, locked: 0 }
        );
        assert_eq!(
            *state.base_balances().get(&bob).unwrap(),
            AssetBalance { free: 100, locked: 0 }
        );
        assert_eq!(
            *state.quote_balances().get(&bob).unwrap(),
            AssetBalance { free: 300, locked: 100 }
        );

        let alice_withdraw = WithdrawRequest { address: alice, base_free: 100, quote_free: 400 };
        let _: ApiResponse = post(&mut app, WITHDRAW, alice_withdraw).await;
        let state = get_clob_state(&mut app).await;
        assert!(!state.quote_balances().contains_key(&alice));
        assert!(!state.base_balances().contains_key(&alice));

        let bob_cancel = CancelOrderRequest { oid: 1 };
        let _: ApiResponse = post(&mut app, CANCEL, bob_cancel).await;
        let bob_withdraw = WithdrawRequest { address: bob, base_free: 100, quote_free: 400 };
        let _: ApiResponse = post(&mut app, WITHDRAW, bob_withdraw).await;
        let state = get_clob_state(&mut app).await;
        assert!(state.quote_balances().is_empty());
        assert!(state.base_balances().is_empty());
    }
}

//! CLOB execution engine.

use crate::db::{
    models::{ClobStateModel, RequestModel, ResponseModel, VecDiffModel},
    tables::{ClobStateTable, DiffTable, GlobalIndexTable, RequestTable, ResponseTable},
    PROCESSED_GLOBAL_INDEX_KEY, SEEN_GLOBAL_INDEX_KEY,
};
use clob_core::{
    api::{ApiResponse, Request},
    tick, ClobState,
};
use reth_db::{
    transaction::{DbTx, DbTxMut},
    Database,
};
use std::sync::Arc;
use tokio::sync::{mpsc::Receiver, oneshot};
use tracing::instrument;

/// The zero index only contains the default state, but no requests.
pub(crate) const GENESIS_GLOBAL_INDEX: u64 = 0;

pub(crate) fn read_start_up_values<D: Database + 'static>(db: Arc<D>) -> (u64, ClobState) {
    let tx = db.tx().expect("todo");
    let global_index = tx
        .get::<GlobalIndexTable>(PROCESSED_GLOBAL_INDEX_KEY)
        .expect("todo: db errors")
        .unwrap_or(GENESIS_GLOBAL_INDEX);

    let clob_state = if global_index == GENESIS_GLOBAL_INDEX {
        let genesis_state = ClobState::default();
        let model = ClobStateModel(genesis_state.clone());
        db.update(|tx| tx.put::<ClobStateTable>(global_index, model)).unwrap().unwrap();
        genesis_state
    } else {
        tx.get::<ClobStateTable>(global_index)
            .expect("todo: db errors")
            .expect("todo: could not find state when some was expected")
            .0
    };
    tx.commit().expect("todo");

    (global_index, clob_state)
}

/// Run the CLOB execution engine
#[instrument(skip_all)]
pub async fn run_engine<D>(
    mut receiver: Receiver<(Request, oneshot::Sender<ApiResponse>)>,
    db: Arc<D>,
) where
    D: Database + 'static,
{
    let (mut global_index, mut state) = read_start_up_values(Arc::clone(&db));

    loop {
        global_index += 1;

        let (request, response_sender) = receiver.recv().await.expect("todo");

        let request2 = request.clone();
        let tx = db.tx_mut().expect("todo");
        tx.put::<GlobalIndexTable>(SEEN_GLOBAL_INDEX_KEY, global_index).expect("todo");
        tx.put::<RequestTable>(global_index, RequestModel(request2)).expect("todo");
        tx.commit().expect("todo");

        let (response, post_state, diffs) = tick(request, state).expect("TODO");

        // Persist: processed index, response, and new state.
        // TODO: cloning entire state is not ideal, would be better to somehow just apply state
        // diffs.
        let post_state2 = post_state.clone();
        let response2 = response.clone();
        let tx = db.tx_mut().expect("todo");
        tx.put::<GlobalIndexTable>(PROCESSED_GLOBAL_INDEX_KEY, global_index).expect("todo");
        tx.put::<ResponseTable>(global_index, ResponseModel(response2)).expect("todo");
        tx.put::<ClobStateTable>(global_index, ClobStateModel(post_state2)).expect("todo");
        tx.put::<DiffTable>(global_index, VecDiffModel(diffs)).expect("todo");
        tx.commit().expect("todo");

        let api_response = ApiResponse { response, global_index };

        response_sender.send(api_response).expect("todo");

        state = post_state;
    }
}

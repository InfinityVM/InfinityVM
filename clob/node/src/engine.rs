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
use tokio::{
    sync::{mpsc::Receiver, oneshot},
    task::JoinSet,
};
use tracing::{info, instrument};

pub(crate) const START_GLOBAL_INDEX: u64 = 0;

pub(crate) fn read_start_up_values<D: Database + 'static>(db: Arc<D>) -> (u64, ClobState) {
    let tx = db.tx().expect("todo");

    let global_index = tx
        .get::<GlobalIndexTable>(PROCESSED_GLOBAL_INDEX_KEY)
        .expect("todo: db errors")
        .unwrap_or(START_GLOBAL_INDEX);

    let clob_state = if global_index == START_GLOBAL_INDEX {
        // Initialize clob state if we haven't processed anything
        ClobState::default()
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

    // TODO add logic to clear the joinset
    let mut handles = JoinSet::new();

    loop {
        global_index += 1;

        let (request, response_sender) = receiver.recv().await.expect("todo");
        info!(?request);

        // In background thread persist the index and request
        let request2 = request.clone();
        let db2 = Arc::clone(&db);
        handles.spawn(async move {
            let tx = db2.tx_mut().expect("todo");
            tx.put::<GlobalIndexTable>(SEEN_GLOBAL_INDEX_KEY, global_index).expect("todo");
            tx.put::<RequestTable>(global_index, RequestModel(request2)).expect("todo");
            tx.commit().expect("todo");
        });

        // TODO(thursday): Do contract call to update free balance and assert locked balance
        // We will need to do a contract view call which will slow things down

        let (response, post_state, difs) = tick(request, state).expect("TODO");

        // In a background task persist: processed index, response, and new state.
        // TODO: cloning entire state is not ideal, would be better to somehow just apply state
        // diffs.
        let post_state2 = post_state.clone();
        let response2 = response.clone();
        let db2 = Arc::clone(&db);
        handles.spawn(async move {
            let tx = db2.tx_mut().expect("todo");
            tx.put::<GlobalIndexTable>(PROCESSED_GLOBAL_INDEX_KEY, global_index).expect("todo");
            tx.put::<ResponseTable>(global_index, ResponseModel(response2)).expect("todo");
            tx.put::<ClobStateTable>(global_index, ClobStateModel(post_state2)).expect("todo");
            tx.put::<DiffTable>(global_index, VecDiffModel(difs)).expect("todo");
            tx.commit().expect("todo");
        });

        let api_response = ApiResponse { response, global_index };

        response_sender.send(api_response).expect("todo");

        state = post_state;
    }
}

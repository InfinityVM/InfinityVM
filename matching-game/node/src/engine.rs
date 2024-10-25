//! CLOB execution engine.

use crate::db::{
    models::{MatchingGameStateModel, RequestModel, ResponseModel},
    tables::{MatchingGameStateTable, GlobalIndexTable, RequestTable, ResponseTable},
    PROCESSED_GLOBAL_INDEX_KEY, SEEN_GLOBAL_INDEX_KEY,
};
use clob_core::{
    api::{ApiResponse, Request},
    tick, MatchingGameState,
};
use eyre::{eyre, OptionExt, WrapErr};
use reth_db::{
    transaction::{DbTx, DbTxMut},
    Database,
};
use std::sync::Arc;
use tokio::sync::{mpsc::Receiver, oneshot};
use tracing::instrument;

/// The zero index only contains the default state, but no requests.
pub(crate) const GENESIS_GLOBAL_INDEX: u64 = 0;

pub(crate) fn read_start_up_values<D: Database + 'static>(
    db: Arc<D>,
) -> eyre::Result<(u64, MatchingGameState)> {
    let global_index = db
        .view(|tx| tx.get::<GlobalIndexTable>(PROCESSED_GLOBAL_INDEX_KEY))??
        .unwrap_or(GENESIS_GLOBAL_INDEX);

    let matching_game_state = if global_index == GENESIS_GLOBAL_INDEX {
        let genesis_state = MatchingGameState::default();
        let model = MatchingGameStateModel(genesis_state.clone());
        db.update(|tx| tx.put::<MatchingGameStateTable>(global_index, model))??;
        genesis_state
    } else {
        db.view(|tx| tx.get::<MatchingGameStateTable>(global_index))??.ok_or_eyre("missing matching game state")?.0
    };
    Ok((global_index, matching_game_state))
}

/// Run the matching game execution engine
#[instrument(skip_all)]
pub async fn run_engine<D>(
    mut receiver: Receiver<(Request, oneshot::Sender<ApiResponse>)>,
    db: Arc<D>,
) -> eyre::Result<()>
where
    D: Database + 'static,
{
    let (mut global_index, mut state) = read_start_up_values(Arc::clone(&db))?;

    loop {
        global_index += 1;

        let (request, response_sender) =
            receiver.recv().await.ok_or_eyre("engine channel sender unexpected dropped")?;

        let request2 = request.clone();
        db.update(|tx| {
            tx.put::<GlobalIndexTable>(SEEN_GLOBAL_INDEX_KEY, global_index)?;
            tx.put::<RequestTable>(global_index, RequestModel(request2))
        })
        .wrap_err_with(|| format!("failed to write request {global_index}"))??;

        let (response, post_state) = tick(request, state);

        let post_state2 = post_state.clone();
        let response2 = response.clone();
        db.update(|tx| {
            tx.put::<GlobalIndexTable>(PROCESSED_GLOBAL_INDEX_KEY, global_index)
                .wrap_err("processed global index")?;
            tx.put::<ResponseTable>(global_index, ResponseModel(response2)).wrap_err("response")?;
            tx.put::<MatchingGameStateTable>(global_index, MatchingGameStateModel(post_state2))
                .wrap_err("matching game state")?;
        })
        .wrap_err_with(|| format!("failed to write tick results {global_index}"))??;

        let api_response = ApiResponse { response, global_index };

        response_sender
            .send(api_response)
            .map_err(|_| eyre!("engine oneshot unexpectedly dropped {global_index}"))?;

        state = post_state;
    }
}

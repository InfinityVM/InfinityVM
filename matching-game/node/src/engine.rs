//! Matching game execution engine.

use crate::state::InMemoryState;
use eyre::{eyre, OptionExt, WrapErr};
use matching_game_core::{
    api::{ApiResponse, Request},
    tick,
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

/// Run the matching game execution engine
#[instrument(skip_all)]
pub async fn run_engine(
    mut receiver: Receiver<(Request, oneshot::Sender<ApiResponse>)>,
    state: Arc<InMemoryState>,
) -> eyre::Result<()>
{
    let mut global_index = state.get_seen_global_index();

    loop {
        global_index += 1;

        let (request, response_sender) =
            receiver.recv().await.ok_or_eyre("engine channel sender unexpected dropped")?;

        state.set_seen_global_index(global_index);
        state.store_request(global_index, request.clone());

        let response = tick(request);
        state.set_processed_global_index(global_index);

        let api_response = ApiResponse { response, global_index };

        response_sender
            .send(api_response)
            .map_err(|_| eyre!("engine oneshot unexpectedly dropped {global_index}"))?;
    }
}

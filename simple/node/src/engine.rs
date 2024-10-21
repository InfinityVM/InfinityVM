//! CLOB execution engine.

use simple_core::{
    api::{ApiResponse, Request}, tick,
    SimpleState,
};
use eyre::{eyre, OptionExt, WrapErr};
use reth_db::{
    transaction::{DbTx, DbTxMut},
    Database,
};
use std::sync::Arc;
use tokio::sync::{mpsc::Receiver, oneshot, RwLock};
use tracing::instrument;

/// The zero index only contains the default state, but no requests.
pub(crate) const GENESIS_GLOBAL_INDEX: u64 = 0;

/// Run the CLOB execution engine
#[instrument(skip_all)]
pub async fn run_engine(
    mut simple_state: Arc<RwLock<SimpleState>>,
    mut receiver: Receiver<(Request, oneshot::Sender<ApiResponse>)>,
) -> eyre::Result<()>
{

    let mut global_index = 0;

    loop {
        global_index += 1;

        let (request, response_sender) =
            receiver.recv().await.ok_or_eyre("engine channel sender unexpected dropped")?;

        let request2 = request.clone();


        let (response, post_state) = tick(request, simple_state.clone());

        // Persist: processed index, response, and new state.
        // TODO: https://github.com/InfinityVM/InfinityVM/issues/197
        // cloning entire state is not ideal, would be better to somehow just apply state diffs.
        let post_state2 = post_state.clone();
        let response2 = response.clone();

        let api_response = ApiResponse { response, global_index };

        response_sender
            .send(api_response)
            .map_err(|_| eyre!("engine oneshot unexpectedly dropped {global_index}"))?;

    }
}

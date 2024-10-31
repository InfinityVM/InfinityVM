//! Matching game execution engine.

use crate::state::ServerState;
use eyre::{eyre, OptionExt};
use kairos_trie::{stored::memory_db::MemoryDb, TrieRoot};
use matching_game_core::{
    api::{ApiResponse, CancelNumberResponse, MatchPair, Request, Response, SubmitNumberResponse},
    next_state,
};
use std::{rc::Rc, sync::Arc};
use tokio::sync::{mpsc::Receiver, oneshot};
use tracing::instrument;

/// Run the matching game execution engine
#[instrument(skip_all)]
pub async fn run_engine(
    mut receiver: Receiver<(Request, oneshot::Sender<ApiResponse>)>,
    state: Arc<ServerState>,
) -> eyre::Result<()> {
    let mut global_index = state.get_seen_global_index();
    let trie_db = Rc::new(MemoryDb::<Vec<u8>>::empty());
    state.set_merkle_root_engine(TrieRoot::Empty);

    loop {
        global_index += 1;

        let (request, response_sender) =
            receiver.recv().await.ok_or_eyre("engine channel sender unexpected dropped")?;

        state.set_seen_global_index(global_index);
        state.store_request(global_index, request.clone());

        let response = match request {
            Request::SubmitNumber(_) => {
                // Update trie merkle root and get match, if any.
                let (post_txn_merkle_root, _, matches) =
                    next_state(trie_db.clone(), state.get_merkle_root_engine(), &[request]);
                let match_pair = matches
                    .first()
                    .cloned()
                    .map(|m| MatchPair { user1: **m.user1, user2: **m.user2 });
                state.set_merkle_root_engine(post_txn_merkle_root);
                Response::SubmitNumber(SubmitNumberResponse { success: true, match_pair })
            }
            Request::CancelNumber(_) => {
                // Update trie merkle root.
                let (post_txn_merkle_root, _, _) =
                    next_state(trie_db.clone(), state.get_merkle_root_engine(), &[request]);
                state.set_merkle_root_engine(post_txn_merkle_root);
                Response::CancelNumber(CancelNumberResponse { success: true })
            }
        };

        state.set_processed_global_index(global_index);

        let api_response = ApiResponse { response, global_index };

        response_sender
            .send(api_response)
            .map_err(|_| eyre!("engine oneshot unexpectedly dropped {global_index}"))?;
    }
}

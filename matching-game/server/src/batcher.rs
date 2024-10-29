//! Collects requests into batches and submits them to the coprocessor node at some regular
//! cadence.
use crate::state::ServerState;
use abi::{abi_encode_offchain_job_request, JobParams, StatefulAppOnchainInput};
use alloy::{primitives::utils::keccak256, signers::Signer, sol_types::SolValue};
use eyre::OptionExt;
use kairos_trie::{
    stored::{memory_db::MemoryDb, merkle::SnapshotBuilder, Store},
    DigestHasher,
    Entry::{Occupied, Vacant, VacantEmptyTrie},
    KeyHash, NodeHash, PortableHash, PortableHasher, Transaction, TrieRoot,
};
use matching_game_core::{
    api::Request, apply_requests, deserialize_address_list, hash, serialize_address_list, Match,
    Matches,
};
use matching_game_programs::MATCHING_GAME_ID;
use proto::{coprocessor_node_client::CoprocessorNodeClient, SubmitJobRequest};
use risc0_zkvm::sha::Digest;
use sha2::Sha256;
use std::{rc::Rc, sync::Arc};
use tokio::time::{sleep, Duration};
use tonic::transport::Channel;
use tracing::{debug, info, instrument, warn};

use crate::K256LocalSigner;

const MAX_CYCLES: u64 = 32 * 1000 * 1000;
/// First global index that a request will get.
const INIT_INDEX: u64 = 1;

async fn ensure_initialized(state: Arc<ServerState>) -> eyre::Result<()> {
    loop {
        if state.get_processed_global_index() > 0 {
            if state.get_next_batch_global_index() == 0 {
                state.set_next_batch_global_index(INIT_INDEX);
            }
            break;
        }

        debug!("waiting for the first request to be processed before starting batcher");
        sleep(Duration::from_millis(1_000)).await;
    }

    Ok(())
}

/// Run the matching game execution engine
#[instrument(skip_all)]
pub async fn run_batcher(
    state: Arc<ServerState>,
    sleep_duration: Duration,
    signer: K256LocalSigner,
    cn_grpc_url: String,
    consumer_addr: [u8; 20],
) -> eyre::Result<()> {
    // Wait for the system to have at least one processed request from the matching game server
    ensure_initialized(Arc::clone(&state)).await?;
    let program_id = Digest::from(MATCHING_GAME_ID).as_bytes().to_vec();

    let mut coprocessor_node = CoprocessorNodeClient::connect(cn_grpc_url).await.unwrap();
    let mut interval = tokio::time::interval(sleep_duration);

    let trie_db = Rc::new(MemoryDb::<Vec<u8>>::empty());
    state.set_merkle_root(TrieRoot::Empty);

    let mut job_nonce = 0;
    loop {
        interval.tick().await;

        let start_index = state.get_next_batch_global_index();
        let end_index = state.get_processed_global_index();

        if start_index > end_index {
            debug!(start_index, end_index, "skipping batch creation");
            continue;
        }

        info!("creating batch {start_index}..={end_index}");

        tokio::task::yield_now().await;

        let mut requests = Vec::with_capacity((end_index - start_index + 1) as usize);
        for i in start_index..=end_index {
            let r = state.get_request(i).ok_or_eyre(format!("batcher: request {i}"))?;
            requests.push(r);
            tokio::task::yield_now().await;
        }
        let requests_borsh = borsh::to_vec(&requests).expect("borsh works. qed.");

        let pre_txn_merkle_root = state.get_merkle_root();
        let mut txn = Transaction::from_snapshot_builder(SnapshotBuilder::new(
            trie_db.clone(),
            pre_txn_merkle_root,
        ));
        apply_requests(&mut txn, &requests);

        let hasher = &mut DigestHasher::<Sha256>::default();
        let post_txn_merkle_root = txn.commit(hasher).unwrap();
        let snapshot = txn.build_initial_snapshot();
        let snapshot_serialized = serde_json::to_vec(&snapshot).expect("serde works. qed.");

        // Combine requests_borsh and snapshot_serialized
        let mut combined_offchain_input = Vec::new();
        combined_offchain_input.extend_from_slice(&(requests_borsh.len() as u32).to_le_bytes());
        combined_offchain_input.extend_from_slice(&requests_borsh);
        combined_offchain_input.extend_from_slice(&snapshot_serialized);
        let offchain_input_hash = keccak256(&combined_offchain_input);

        let pre_txn_merkle_root_option: Option<[u8; 32]> = pre_txn_merkle_root.into();
        let pre_txn_merkle_root_bytes = if pre_txn_merkle_root_option.is_none() {
            Default::default()
        } else {
            pre_txn_merkle_root_option.unwrap()
        };
        let onchain_input = StatefulAppOnchainInput {
            input_state_root: pre_txn_merkle_root_bytes.into(),
            onchain_input: (&[]).into(),
        };
        let onchain_input_abi_encoded = StatefulAppOnchainInput::abi_encode(&onchain_input);

        let job_params = JobParams {
            nonce: job_nonce,
            max_cycles: MAX_CYCLES,
            onchain_input: onchain_input_abi_encoded.as_slice(),
            offchain_input_hash: offchain_input_hash.into(),
            program_id: &program_id,
            consumer_address: consumer_addr,
        };
        let request = abi_encode_offchain_job_request(job_params);
        let signature = signer.sign_message(&request).await?.as_bytes().to_vec();
        let job_request =
            SubmitJobRequest { request, signature, offchain_input: combined_offchain_input };

        submit_job_with_backoff(&mut coprocessor_node, job_request).await?;

        state.set_next_batch_global_index(end_index + 1);
        state.set_merkle_root(post_txn_merkle_root);

        job_nonce += 1;
    }
}

async fn submit_job_with_backoff(
    client: &mut CoprocessorNodeClient<Channel>,
    request: SubmitJobRequest,
) -> eyre::Result<()> {
    const RETRIES: usize = 3;
    const MULTIPLE: u64 = 8;

    let mut backoff = 1;

    for _ in 0..RETRIES {
        match client.submit_job(request.clone()).await {
            Ok(_) => return Ok(()),
            Err(error) => warn!(backoff, ?error, "failed to submit batch to coprocessor"),
        }

        sleep(Duration::from_secs(backoff)).await;
        backoff *= MULTIPLE;
    }

    eyre::bail!("failed to submit batch to coprocessor after {RETRIES} retries")
}

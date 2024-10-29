//! Collects requests into batches and submits them to the coprocessor node at some regular
//! cadence.
use crate::state::InMemoryState;
use abi::{abi_encode_offchain_job_request, JobParams, StatefulAppOnchainInput};
use alloy::{primitives::utils::keccak256, signers::Signer, sol_types::SolValue};
use eyre::OptionExt;
use matching_game_core::api::Request;
use matching_game_programs::MATCHING_GAME_ID;
use proto::{coprocessor_node_client::CoprocessorNodeClient, SubmitJobRequest};
use reth_db::transaction::{DbTx, DbTxMut};
use reth_db_api::Database;
use risc0_zkvm::sha::Digest;
use std::sync::Arc;
use tokio::time::{sleep, Duration};
use tonic::transport::Channel;
use tracing::{debug, info, instrument, warn};
use kairos_trie::{
    stored::{memory_db::MemoryDb, merkle::SnapshotBuilder, Store},
    DigestHasher, KeyHash, PortableHash, PortableHasher, Transaction, TrieRoot,
};
use std::rc::Rc;
use sha2::Sha256;

use crate::K256LocalSigner;

const MAX_CYCLES: u64 = 32 * 1000 * 1000;
/// First global index that a request will get.
const INIT_INDEX: u64 = 1;

async fn ensure_initialized(state: Arc<InMemoryState>) -> eyre::Result<()> {
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

fn hash(key: u64) -> KeyHash {
    let hasher = &mut DigestHasher::<Sha256>::default();
    key.portable_hash(hasher);
    KeyHash::from_bytes(&hasher.finalize_reset())
}

pub fn serialize_address_list(addresses: &Vec<[u8; 20]>) -> Vec<u8> {
    borsh::to_vec(addresses).expect("borsh works. qed.")
}

pub fn deserialize_address_list(data: &[u8]) -> Vec<[u8; 20]> {
    borsh::from_slice(data).expect("borsh works. qed.")
}

fn apply_requests(txn: &mut Transaction<impl Store<Value = Vec<u8>>>, requests: &[Request]) {
    for r in requests {
        match r {
            Request::SubmitNumber(s) => {
                let mut old_list = deserialize_address_list(txn.entry(&hash(s.number)).unwrap().or_default());
                old_list.push(s.address);
                let _ = txn.insert(&hash(s.number), serialize_address_list(&old_list));
            }
            Request::CancelNumber(c) => {
                let mut old_list = deserialize_address_list(txn.entry(&hash(c.number)).unwrap().or_default());
                old_list.remove(old_list.iter().position(|&x| x == c.address).unwrap());
                let _ = txn.insert(&hash(c.number), serialize_address_list(&old_list));
            }
        }
    }
}

/// Run the matching game execution engine
#[instrument(skip_all)]
pub async fn run_batcher(
    state: Arc<InMemoryState>,
    sleep_duration: Duration,
    signer: K256LocalSigner,
    cn_grpc_url: String,
    consumer_addr: [u8; 20],
) -> eyre::Result<()>
{
    // Wait for the system to have at least one processed request from the matching game server
    ensure_initialized(Arc::clone(&state)).await?;
    let program_id = Digest::from(MATCHING_GAME_ID).as_bytes().to_vec();

    let mut coprocessor_node = CoprocessorNodeClient::connect(cn_grpc_url).await.unwrap();
    let mut interval = tokio::time::interval(sleep_duration);

    let trie_db = Rc::new(MemoryDb::<Vec<u8>>::empty());

    // println!("NARULA TrieRoot::Empty: {:?}", TrieRoot::Empty);
    let mut txn =
        Transaction::from_snapshot_builder(SnapshotBuilder::new(trie_db.clone(), TrieRoot::Empty));
    let hasher = &mut DigestHasher::<Sha256>::default();

    let mut pre_txn_merkle_root = txn.calc_root_hash(hasher).unwrap();

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
            let r = state.get_request(i)
                .ok_or_eyre(format!("batcher: request {i}"))?;
            requests.push(r);
            tokio::task::yield_now().await;
        }
        let requests_borsh = borsh::to_vec(&requests).expect("borsh works. qed.");

        let mut txn = Transaction::from_snapshot_builder(SnapshotBuilder::new(trie_db.clone(), pre_txn_merkle_root));
        apply_requests(&mut txn, &requests);

        let hasher = &mut DigestHasher::<Sha256>::default();
        let output_merkle_root = txn.commit(hasher).unwrap();
        let snapshot = txn.build_initial_snapshot();

        let merkle_root_thirty_two: Option<[u8; 32]> = pre_txn_merkle_root.into();
        let onchain_input_state_root = if merkle_root_thirty_two.is_none() {
            Default::default()
        } else {
            merkle_root_thirty_two.unwrap()
        };

        let snapshot_serialized = serde_json::to_vec(&snapshot).expect("serde works. qed.");
        // Combine requests_borsh and snapshot_serialized
        let mut combined_offchain_input = Vec::new();
        combined_offchain_input.extend_from_slice(&(requests_borsh.len() as u32).to_le_bytes());
        combined_offchain_input.extend_from_slice(&requests_borsh);
        combined_offchain_input.extend_from_slice(&snapshot_serialized);
        let offchain_input_hash = keccak256(&combined_offchain_input);

        let onchain_input = StatefulAppOnchainInput {
            input_state_root: onchain_input_state_root.into(),
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

        pre_txn_merkle_root = output_merkle_root;

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

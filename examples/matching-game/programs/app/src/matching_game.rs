//! ZKVM program for running the matching game.

use alloy::sol_types::SolType;
use ivm_abi::{StatefulAppOnchainInput, StatefulAppResult};
use risc0_zkvm::guest::env;
use matching_game_core::{api::Request, Matches, apply_requests, get_merkle_root_bytes};
use kairos_trie::{
    stored::merkle::{Snapshot, VerifiedSnapshot},
    DigestHasher, NodeHash, Transaction, TrieRoot,
};
use sha2::Sha256;

fn main() {
    let onchain_input_len: u32 = env::read();
    let mut onchain_input_buf = vec![0u8; onchain_input_len as usize];
    env::read_slice(&mut onchain_input_buf);

    let stateful_app_input = StatefulAppOnchainInput::abi_decode(&onchain_input_buf, false).unwrap();
    let input_merkle_root = stateful_app_input.input_state_root;

    let offchain_input_len: u32 = env::read();
    let mut offchain_input_buf = vec![0u8; offchain_input_len as usize];
    env::read_slice(&mut offchain_input_buf);

    // We combined the requests and snapshot in offchain input, so we need to split them here.
    let requests_len = u32::from_le_bytes(offchain_input_buf[..4].try_into().unwrap()) as usize;
    let requests_bytes = offchain_input_buf[4..4 + requests_len].to_vec();
    let snapshot_bytes = offchain_input_buf[4 + requests_len..].to_vec();

    let requests: Vec<Request> = bincode::deserialize(&requests_bytes)
        .expect("TODO: https://github.com/InfinityVM/InfinityVM/issues/296");
    let snapshot: Snapshot<Vec<u8>> = bincode::deserialize(&snapshot_bytes)
        .expect("TODO: https://github.com/InfinityVM/InfinityVM/issues/296");

    let pre_txn_merkle_root = if *input_merkle_root == [0u8; 32] {
        TrieRoot::Empty
    } else {
        TrieRoot::Node(NodeHash::new(*input_merkle_root))
    };

    // Verify the snapshot against the merkle root.
    let hasher = &mut DigestHasher::<Sha256>::default();
    let verified_snapshot = VerifiedSnapshot::verify_snapshot(snapshot, hasher).unwrap();
    let pre_batch_trie_root = verified_snapshot.trie_root_hash();
    assert_eq!(pre_batch_trie_root, pre_txn_merkle_root);

    // Apply the batch of requests to the trie and get the new merkle root.
    let mut trie_txn = Transaction::from(verified_snapshot);
    let matches = apply_requests(&mut trie_txn, &requests);
    let output_merkle_root = trie_txn.calc_root_hash(hasher).unwrap();

    let stateful_app_result = StatefulAppResult {
        output_state_root: get_merkle_root_bytes(output_merkle_root).into(),
        result: Matches::abi_encode(&matches).into(),
    };

    let abi_encoded = StatefulAppResult::abi_encode(&stateful_app_result);

    env::commit_slice(&abi_encoded);
}

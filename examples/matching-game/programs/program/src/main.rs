#![no_main]
sp1_zkvm::entrypoint!(main);

use alloy::sol_types::SolType;
use ivm_abi::{StatefulAppOnchainInput, StatefulAppResult};
use matching_game_core::{api::Request, Matches, apply_requests, get_merkle_root_bytes};
use kairos_trie::{
    stored::merkle::{Snapshot, VerifiedSnapshot},
    DigestHasher, NodeHash, TrieRoot, Transaction,
};
use sha2::Sha256;

fn main() {
    // Read onchain input
    let onchain_input = sp1_zkvm::io::read_vec();
    let stateful_app_input = StatefulAppOnchainInput::abi_decode(&onchain_input, false).unwrap();
    let input_merkle_root = stateful_app_input.input_state_root;

    // Read offchain input
    let offchain_input = sp1_zkvm::io::read_vec();

    // Split combined requests and snapshot
    let requests_len = u32::from_le_bytes(offchain_input[..4].try_into().unwrap()) as usize;
    let requests_bytes = offchain_input[4..4 + requests_len].to_vec();
    let snapshot_bytes = offchain_input[4 + requests_len..].to_vec();

    let requests: Vec<Request> = bincode::deserialize(&requests_bytes)
        .expect("TODO: https://github.com/InfinityVM/InfinityVM/issues/296");
    let snapshot: Snapshot<Vec<u8>> = bincode::deserialize(&snapshot_bytes)
        .expect("TODO: https://github.com/InfinityVM/InfinityVM/issues/296");

    let pre_txn_merkle_root = if *input_merkle_root == [0u8; 32] {
        TrieRoot::Empty
    } else {
        TrieRoot::Node(NodeHash::new(*input_merkle_root))
    };

    // Verify snapshot against merkle root
    let hasher = &mut DigestHasher::<Sha256>::default();
    let verified_snapshot = VerifiedSnapshot::verify_snapshot(snapshot, hasher).unwrap();
    let pre_batch_trie_root = verified_snapshot.trie_root_hash();
    assert_eq!(pre_batch_trie_root, pre_txn_merkle_root);

    // Create transaction and process requests
    let mut txn = Transaction::from_verified_snapshot(verified_snapshot);
    let matches = apply_requests(&mut txn, &requests);
    let matches_output = Matches::abi_encode(&matches);

    // Get new merkle root after transaction
    let new_root = txn.calc_root_hash(hasher).unwrap();

    // Create and encode result
    let result = StatefulAppResult {
        output_state_root: get_merkle_root_bytes(new_root).into(),
        result: matches_output.into(),
    };

    sp1_zkvm::io::commit_slice(&StatefulAppResult::abi_encode(&result));
}
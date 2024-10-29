//! ZKVM program for running the matching game.

use alloy::sol_types::SolType;
use abi::{StatefulAppOnchainInput, StatefulAppResult};
use risc0_zkvm::guest::env;
use matching_game_core::{api::Request, Match, Matches};
use kairos_trie::{
    stored::{memory_db::MemoryDb, merkle::{Snapshot, SnapshotBuilder, VerifiedSnapshot}, Store},
    DigestHasher, KeyHash, NodeHash, PortableHash, PortableHasher, Transaction, TrieRoot,
    Entry::{Occupied, Vacant, VacantEmptyTrie},
};
use std::rc::Rc;
use sha2::Sha256;

fn serialize_address_list(addresses: &Vec<[u8; 20]>) -> Vec<u8> {
    borsh::to_vec(addresses).expect("borsh works. qed.")
}

fn deserialize_address_list(data: &[u8]) -> Vec<[u8; 20]> {
    borsh::from_slice(data).expect("borsh works. qed.")
}

fn hash(key: u64) -> KeyHash {
    let hasher = &mut DigestHasher::<Sha256>::default();
    key.portable_hash(hasher);
    KeyHash::from_bytes(&hasher.finalize_reset())
}

fn apply_requests(txn: &mut Transaction<impl Store<Value = Vec<u8>>>, requests: &[Request]) -> Vec<Match> {
    let mut matches = Vec::<Match>::with_capacity(requests.len());

    for r in requests {
        match r {
            Request::SubmitNumber(s) => {

                let mut old_list = txn.entry(&hash(s.number)).unwrap();
                match old_list {
                    Occupied(mut entry) => {
                        let mut old_list = deserialize_address_list(entry.get());
                        if old_list.is_empty() {
                            old_list.push(s.address);
                        } else {
                            let match_pair = Match { user1: old_list[0].into(), user2: s.address.into() };
                            matches.push(match_pair);

                            // remove the first element from the list
                            old_list.remove(0);
                        }
                        let _ = entry.insert(serialize_address_list(&old_list));
                    }
                    Vacant(_) => {
                        let _ = txn.insert(&hash(s.number), serialize_address_list(&vec![s.address]));
                    }
                    VacantEmptyTrie(_) => {
                        let _ = txn.insert(&hash(s.number), serialize_address_list(&vec![s.address]));
                    }
                }
            }
            Request::CancelNumber(c) => {
                let mut old_list = txn.entry(&hash(c.number)).unwrap();
                match old_list {
                    Occupied(mut entry) => {
                        let mut old_list = deserialize_address_list(entry.get());
                        old_list.remove(old_list.iter().position(|&x| x == c.address).unwrap());
                        let _ = entry.insert(serialize_address_list(&old_list));
                    }
                    Vacant(_) => {
                        // do nothing
                    }
                    VacantEmptyTrie(_) => {
                        // do nothing
                    }
                }
            }
        }
    }

    matches.sort();
    matches
}

fn main() {
    let onchain_input_len: u32 = env::read();
    let mut onchain_input_buf = vec![0u8; onchain_input_len as usize];
    env::read_slice(&mut onchain_input_buf);

    let stateful_app_input = StatefulAppOnchainInput::abi_decode(&onchain_input_buf, false).unwrap();
    let input_merkle_root = stateful_app_input.input_state_root;

    let offchain_input_len: u32 = env::read();
    let mut offchain_input_buf = vec![0u8; offchain_input_len as usize];
    env::read_slice(&mut offchain_input_buf);

    // We combined the requests and trie nodes in offchain input, so we need to split them here.
    let requests_len = u32::from_le_bytes(offchain_input_buf[..4].try_into().unwrap()) as usize;
    let requests_borsh = offchain_input_buf[4..4 + requests_len].to_vec();
    let snapshot_serialized = offchain_input_buf[4 + requests_len..].to_vec();

    let requests: Vec<Request> = borsh::from_slice(&requests_borsh)
        .expect("TODO: https://github.com/InfinityVM/InfinityVM/issues/296");
    let snapshot: Snapshot<Vec<u8>> = serde_json::from_slice(&snapshot_serialized).expect("serde works. qed.");
    let pre_txn_merkle_root = if *input_merkle_root == [0u8; 32] {
        TrieRoot::Empty
    } else {
        TrieRoot::Node(NodeHash::new(*input_merkle_root))
    };

    let hasher = &mut DigestHasher::<Sha256>::default();
    let verified_snapshot = VerifiedSnapshot::verify_snapshot(snapshot, hasher).unwrap();
    let pre_batch_trie_root = verified_snapshot.trie_root_hash();
    assert_eq!(pre_batch_trie_root, pre_txn_merkle_root);

    let mut txn = Transaction::from(verified_snapshot);
    let matches = apply_requests(&mut txn, &requests);
    let output_merkle_root = txn.calc_root_hash(hasher).unwrap();

    let output_merkle_root_option: Option<[u8; 32]> = output_merkle_root.into();
    let output_merkle_root_bytes = output_merkle_root_option.unwrap();

    let stateful_app_result = StatefulAppResult {
        output_state_root: output_merkle_root_bytes.into(),
        result: Matches::abi_encode(&matches).into(),
    };

    let abi_encoded = StatefulAppResult::abi_encode(&stateful_app_result);

    env::commit_slice(&abi_encoded);
}

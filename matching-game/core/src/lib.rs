//! Shared logic and types of the matching game.
use api::Request;
use kairos_trie::{
    stored::{
        memory_db::MemoryDb,
        merkle::{Snapshot, SnapshotBuilder},
        Store,
    },
    DigestHasher,
    Entry::{Occupied, Vacant, VacantEmptyTrie},
    KeyHash, NodeHash, PortableHash, PortableHasher, Transaction, TrieRoot,
};
use sha2::Sha256;
use std::rc::Rc;
/// Matching game server API types.
pub mod api;

alloy::sol! {
    /// A pair of users that matched.
    #[derive(Default, PartialEq, Eq, PartialOrd, Ord, Debug)]
    struct Match {
        /// First user in pair.
        address user1;
        /// Second user in pair.
        address user2;
    }
}

/// Array of matches. Result from matching game program.
pub type Matches = alloy::sol! {
    Match[]
};

/// Serialize a list of addresses.
pub fn serialize_address_list(addresses: &Vec<[u8; 20]>) -> Vec<u8> {
    borsh::to_vec(addresses).expect("borsh works. qed.")
}

/// Deserialize a list of addresses.
pub fn deserialize_address_list(data: &[u8]) -> Vec<[u8; 20]> {
    borsh::from_slice(data).expect("borsh works. qed.")
}

/// Hash a number to a key hash for the merkle trie.
pub fn hash(key: u64) -> KeyHash {
    let hasher = &mut DigestHasher::<Sha256>::default();
    key.portable_hash(hasher);
    KeyHash::from_bytes(&hasher.finalize_reset())
}

/// Get the bytes representation of a merkle root.
pub fn get_merkle_root_bytes(merkle_root: TrieRoot<NodeHash>) -> [u8; 32] {
    <TrieRoot<NodeHash> as Into<Option<[u8; 32]>>>::into(merkle_root).unwrap_or_default()
}

/// Apply a list of requests to a trie and return the new merkle root and snapshot.
pub fn next_state(
    trie_db: Rc<MemoryDb<Vec<u8>>>,
    pre_txn_merkle_root: TrieRoot<NodeHash>,
    requests: &[Request],
) -> (TrieRoot<NodeHash>, Snapshot<Vec<u8>>) {
    let mut txn =
        Transaction::from_snapshot_builder(SnapshotBuilder::new(trie_db, pre_txn_merkle_root));
    apply_requests(&mut txn, requests);

    let hasher = &mut DigestHasher::<Sha256>::default();
    let post_txn_merkle_root = txn.commit(hasher).unwrap();
    let snapshot = txn.build_initial_snapshot();

    (post_txn_merkle_root, snapshot)
}

/// Apply a list of requests to a trie and return the matches.
pub fn apply_requests(
    txn: &mut Transaction<impl Store<Value = Vec<u8>>>,
    requests: &[Request],
) -> Vec<Match> {
    let mut matches = Vec::<Match>::with_capacity(requests.len());

    for r in requests {
        match r {
            Request::SubmitNumber(s) => {
                let addresses = txn.entry(&hash(s.number)).unwrap();
                match addresses {
                    Occupied(mut entry) => {
                        let mut old_list = deserialize_address_list(entry.get());
                        if old_list.is_empty() {
                            // If addresses list is empty, add the new address.
                            old_list.push(s.address);
                        } else {
                            // If addresses list is not empty, we have a match!
                            let match_pair =
                                Match { user1: old_list[0].into(), user2: s.address.into() };
                            matches.push(match_pair);

                            // remove the first element from the list
                            old_list.remove(0);
                        }
                        let _ = entry.insert(serialize_address_list(&old_list));
                    }
                    Vacant(_) | VacantEmptyTrie(_) => {
                        // If addresses list doesn't exist for this number, create it.
                        let _ =
                            txn.insert(&hash(s.number), serialize_address_list(&vec![s.address]));
                    }
                }
            }
            Request::CancelNumber(c) => {
                let addresses = txn.entry(&hash(c.number)).unwrap();
                match addresses {
                    Occupied(mut entry) => {
                        let mut old_list = deserialize_address_list(entry.get());
                        // Remove the user's address from the list.
                        old_list.remove(old_list.iter().position(|&x| x == c.address).unwrap());
                        let _ = entry.insert(serialize_address_list(&old_list));
                    }
                    Vacant(_) | VacantEmptyTrie(_) => {
                        // do nothing
                    }
                }
            }
        }
    }

    matches.sort();
    matches
}

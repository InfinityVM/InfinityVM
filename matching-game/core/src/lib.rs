//! Shared logic and types of the matching game.
use kairos_trie::{
    stored::{memory_db::MemoryDb, merkle::{Snapshot, SnapshotBuilder, VerifiedSnapshot}, Store},
    DigestHasher, KeyHash, NodeHash, PortableHash, PortableHasher, Transaction, TrieRoot,
    Entry::{Occupied, Vacant, VacantEmptyTrie},
};
use sha2::Sha256;
use crate::api::Request;

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

pub fn serialize_address_list(addresses: &Vec<[u8; 20]>) -> Vec<u8> {
    borsh::to_vec(addresses).expect("borsh works. qed.")
}

pub fn deserialize_address_list(data: &[u8]) -> Vec<[u8; 20]> {
    borsh::from_slice(data).expect("borsh works. qed.")
}

pub fn hash(key: u64) -> KeyHash {
    let hasher = &mut DigestHasher::<Sha256>::default();
    key.portable_hash(hasher);
    KeyHash::from_bytes(&hasher.finalize_reset())
}

pub fn apply_requests(txn: &mut Transaction<impl Store<Value = Vec<u8>>>, requests: &[Request]) -> Vec<Match> {
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
                let old_list = txn.entry(&hash(c.number)).unwrap();
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

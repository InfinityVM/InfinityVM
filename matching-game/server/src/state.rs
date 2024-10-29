use std::sync::Mutex;

use kairos_trie::{
    stored::{memory_db::MemoryDb, merkle::SnapshotBuilder, Store},
    DigestHasher,
    Entry::{Occupied, Vacant, VacantEmptyTrie},
    KeyHash, NodeHash, PortableHash, PortableHasher, Transaction, TrieRoot,
};
use matching_game_core::api::Request;

#[derive(Default)]
pub struct ServerState {
    merkle_root: Mutex<TrieRoot<NodeHash>>,
    requests: Mutex<std::collections::HashMap<u64, Request>>,
    seen_global_index: Mutex<u64>,
    processed_global_index: Mutex<u64>,
    next_batch_global_index: Mutex<u64>,
}

impl ServerState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get_merkle_root(&self) -> TrieRoot<NodeHash> {
        *self.merkle_root.lock().unwrap()
    }

    pub fn set_merkle_root(&self, merkle_root: TrieRoot<NodeHash>) {
        *self.merkle_root.lock().unwrap() = merkle_root;
    }

    pub fn store_request(&self, index: u64, request: Request) {
        self.requests.lock().unwrap().insert(index, request);
    }

    pub fn get_request(&self, index: u64) -> Option<Request> {
        self.requests.lock().unwrap().get(&index).map(|r| r.clone())
    }

    pub fn get_seen_global_index(&self) -> u64 {
        *self.seen_global_index.lock().unwrap()
    }

    pub fn set_seen_global_index(&self, index: u64) {
        *self.seen_global_index.lock().unwrap() = index;
    }

    pub fn get_processed_global_index(&self) -> u64 {
        *self.processed_global_index.lock().unwrap()
    }

    pub fn set_processed_global_index(&self, index: u64) {
        *self.processed_global_index.lock().unwrap() = index;
    }

    pub fn get_next_batch_global_index(&self) -> u64 {
        *self.next_batch_global_index.lock().unwrap()
    }

    pub fn set_next_batch_global_index(&self, index: u64) {
        *self.next_batch_global_index.lock().unwrap() = index;
    }
}

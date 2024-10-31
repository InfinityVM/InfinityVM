//! In-memory state for the matching game server.

use std::sync::{
    atomic::{AtomicU64, Ordering},
    Mutex,
};

use kairos_trie::{NodeHash, TrieRoot};
use matching_game_core::api::Request;

/// In-memory state.
#[derive(Default, Debug)]
pub struct ServerState {
    /// The merkle root of the trie after the most recently processed batch.
    merkle_root_batcher: Mutex<TrieRoot<NodeHash>>,
    /// The merkle root of the trie after the most recently processed engine (server) call.
    merkle_root_engine: Mutex<TrieRoot<NodeHash>>,
    requests: Mutex<std::collections::HashMap<u64, Request>>,
    seen_global_index: AtomicU64,
    processed_global_index: AtomicU64,
    next_batch_global_index: AtomicU64,
}

impl ServerState {
    /// Create new in-memory state.
    pub fn new() -> Self {
        Self::default()
    }

    /// Get the merkle root from the most recently processed batch.
    pub fn get_merkle_root_batcher(&self) -> TrieRoot<NodeHash> {
        *self.merkle_root_batcher.lock().unwrap()
    }

    /// Set the merkle root for the most recently processed batch.
    pub fn set_merkle_root_batcher(&self, merkle_root: TrieRoot<NodeHash>) {
        *self.merkle_root_batcher.lock().unwrap() = merkle_root;
    }

    /// Get the merkle root from the most recently processed engine call.
    pub fn get_merkle_root_engine(&self) -> TrieRoot<NodeHash> {
        *self.merkle_root_engine.lock().unwrap()
    }

    /// Set the merkle root for the most recently processed engine call.
    pub fn set_merkle_root_engine(&self, merkle_root: TrieRoot<NodeHash>) {
        *self.merkle_root_engine.lock().unwrap() = merkle_root;
    }

    /// Store a request.
    pub fn store_request(&self, index: u64, request: Request) {
        self.requests.lock().unwrap().insert(index, request);
    }

    /// Get a request by its global index.
    pub fn get_request(&self, index: u64) -> Option<Request> {
        self.requests.lock().unwrap().get(&index).cloned()
    }

    /// Get the global index of the most recently seen request.
    pub fn get_seen_global_index(&self) -> u64 {
        self.seen_global_index.load(Ordering::SeqCst)
    }

    /// Set the global index of the most recently seen request.
    pub fn set_seen_global_index(&self, index: u64) {
        self.seen_global_index.store(index, Ordering::SeqCst);
    }

    /// Get the global index of the most recently processed request.
    pub fn get_processed_global_index(&self) -> u64 {
        self.processed_global_index.load(Ordering::SeqCst)
    }

    /// Set the global index of the most recently processed request.
    pub fn set_processed_global_index(&self, index: u64) {
        self.processed_global_index.store(index, Ordering::SeqCst);
    }

    /// Get the global index of the next batch.
    pub fn get_next_batch_global_index(&self) -> u64 {
        self.next_batch_global_index.load(Ordering::SeqCst)
    }

    /// Set the global index of the next batch.
    pub fn set_next_batch_global_index(&self, index: u64) {
        self.next_batch_global_index.store(index, Ordering::SeqCst);
    }
}

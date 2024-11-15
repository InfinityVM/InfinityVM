//! Abstraction for global handle to all queues.

use dashmap::DashMap;
use ivm_db::queue::Queue;
use parking_lot::Mutex;
use reth_db::Database;
use std::{collections::VecDeque, sync::Arc};
use tracing::instrument;

/// Error type for queue handle.
#[derive(thiserror::Error, Debug)]
pub enum Error {
    /// database error
    #[error("database error: {0}")]
    Database(#[from] ivm_db::Error),
}

/// A global handle to all the existing relay queues.
///
/// We never want two tasks to use a queue at the same time because we risk corrupting the
/// queue. To solve this, we maintain a mutex for each queue, forcing only 1 user at a time.
#[derive(Debug)]
pub struct Queues<D> {
    /// We use `DashMap` as it allows us to safely access the map concurrently.
    inner: Arc<DashMap<[u8; 20], Mutex<Queue<D>>>>,
    db: Arc<D>,
}

impl<D> Clone for Queues<D> {
    fn clone(&self) -> Self {
        Self { inner: Arc::clone(&self.inner), db: Arc::clone(&self.db) }
    }
}

impl<D> Queues<D>
where
    D: Database,
{
    /// Create a new instance of [Self].
    pub fn new(db: Arc<D>) -> Self {
        Self { inner: Arc::new(DashMap::new()), db }
    }

    /// Peek the back of the relay queue for `consumer_address`.
    #[instrument(skip_all)]
    pub fn peek_back(&self, consumer_address: [u8; 20]) -> Result<Option<[u8; 32]>, Error> {
        let mutex = match self.inner.get(&consumer_address) {
            None => return Ok(None),
            Some(mutex) => mutex,
        };
        let queue = mutex.lock();

        (*queue).peek_back().map_err(Into::into)
    }

    /// Pop the element off back of the relay queue for `consumer_address`.
    #[instrument(skip_all)]
    pub fn pop_back(&self, consumer_address: [u8; 20]) -> Result<Option<[u8; 32]>, Error> {
        let mutex = match self.inner.get(&consumer_address) {
            // If we don't have an entry for the queue, we know there is nothing in it.
            None => return Ok(None),
            Some(mutex) => mutex,
        };

        let queue = mutex.lock();

        let back = queue.pop_back()?;
        let is_empty = queue.is_empty()?;
        // Drop the mutex guard
        drop(queue);
        // Drop the entry so we release the lock on the map
        drop(mutex);

        if is_empty {
            // To prevent memory leaks we need to remove the queue from the map
            self.inner.remove(&consumer_address);
        }

        Ok(back)
    }

    /// Push an element onto the front of the queue for the given address
    #[instrument(skip_all)]
    pub fn push_front(&self, consumer_address: [u8; 20], job_id: [u8; 32]) -> Result<(), Error> {
        self.inner
            .entry(consumer_address)
            .or_insert_with(|| Mutex::new(Queue::new(self.db.clone(), consumer_address)))
            .lock()
            .push_front(job_id)
            .map_err(Into::into)
    }
}

type MutexQueue = Mutex<VecDeque<[u8; 32]>>;

/// In memory collection of queues
#[derive(Debug)]
pub struct Queues2 {
    inner: Arc<DashMap<[u8; 20], MutexQueue>>,
}

impl Clone for Queues2 {
    fn clone(&self) -> Self {
        Self { inner: Arc::clone(&self.inner) }
    }
}

impl Default for Queues2 {
    fn default() -> Self {
        Self::new()
    }
}

impl Queues2 {
    /// Create a new instance of [Self].
    pub fn new() -> Self {
        Self { inner: Arc::new(DashMap::new()) }
    }

    /// Peek the back of the relay queue for `consumer_address`.
    #[instrument(skip_all)]
    pub fn peek_back(&self, consumer_address: [u8; 20]) -> Option<[u8; 32]> {
        let mutex = match self.inner.get(&consumer_address) {
            None => return None,
            Some(mutex) => mutex,
        };
        let queue = mutex.lock();

        (*queue).back().copied()
    }

    /// Pop the element off back of the relay queue for `consumer_address`.
    #[instrument(skip_all)]
    pub fn pop_back(&self, consumer_address: [u8; 20]) -> Option<[u8; 32]> {
        let mutex = match self.inner.get(&consumer_address) {
            // If we don't have an entry for the queue, we know there is nothing in it.
            None => return None,
            Some(mutex) => mutex,
        };
        let mut queue = mutex.lock();

        let back = queue.pop_back();
        let is_empty = queue.is_empty();
        // Drop the mutex guard
        drop(queue);
        // Drop the entry so we release the lock on the map
        drop(mutex);

        if is_empty {
            // To prevent memory leaks we need to remove the queue from the map
            self.inner.remove(&consumer_address);
        }

        back
    }

    /// Push an element onto the front of the queue for the given address
    #[instrument(skip_all)]
    pub fn push_front(&self, consumer_address: [u8; 20], job_id: [u8; 32]) {
        self.inner
            .entry(consumer_address)
            .or_insert_with(|| Mutex::new(VecDeque::new()))
            .lock()
            .push_front(job_id)
    }
}

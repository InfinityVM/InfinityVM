//! Abstraction for global handle to all queues.

use dashmap::DashMap;
use parking_lot::Mutex;
use std::{collections::VecDeque, sync::Arc};
use tracing::instrument;

type MutexQueue = Mutex<VecDeque<[u8; 32]>>;

/// A thread safe in memory collection of job id queues, keyed by consumer contract address.
#[derive(Debug)]
pub struct Queues {
    inner: Arc<DashMap<[u8; 20], MutexQueue>>,
}

impl Clone for Queues {
    fn clone(&self) -> Self {
        Self { inner: Arc::clone(&self.inner) }
    }
}

impl Default for Queues {
    fn default() -> Self {
        Self::new()
    }
}

impl Queues {
    /// Create a new instance of [Self].
    pub fn new() -> Self {
        Self { inner: Arc::new(DashMap::new()) }
    }

    /// Peek the back of the relay queue for `consumer_address`.
    #[instrument(skip_all, level = "debug")]
    pub fn peek_back(&self, consumer_address: [u8; 20]) -> Option<[u8; 32]> {
        let mutex = match self.inner.get(&consumer_address) {
            None => return None,
            Some(mutex) => mutex,
        };
        let queue = mutex.lock();

        (*queue).back().copied()
    }

    /// Pop the element off back of the relay queue for `consumer_address`.
    #[instrument(skip_all, level = "debug")]
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
    #[instrument(skip_all, level = "debug")]
    pub fn push_front(&self, consumer_address: [u8; 20], job_id: [u8; 32]) {
        self.inner
            .entry(consumer_address)
            .or_insert_with(|| Mutex::new(VecDeque::new()))
            .lock()
            .push_front(job_id)
    }
}

#[cfg(test)]
mod test {
    use super::Queues;

    #[test]
    fn single_node_queue() {
        let addr0 = [0u8; 20];
        let id0 = [0u8; 32];

        let queue = Queues::new();

        // An uninitialized queue is empty.
        assert!(queue.peek_back(addr0).is_none());
        assert!(queue.pop_back(addr0).is_none());

        // We can add an element to the queue
        queue.push_front(addr0, id0);

        // We can see the single element on the queue
        assert_eq!(queue.peek_back(addr0), Some(id0));

        // We can remove the single element from the queue
        assert_eq!(queue.pop_back(addr0), Some(id0));

        // The queue is now empty
        assert!(queue.peek_back(addr0).is_none());
        assert!(queue.pop_back(addr0).is_none());
    }

    #[test]
    fn push_pop() {
        let addr0 = [0u8; 20];
        let id0 = [0u8; 32];
        let id1 = [1u8; 32];
        let id2 = [2u8; 32];
        let id3 = [3u8; 32];

        let queue = Queues::new();

        // We can perform consecutive pushes and peek back
        queue.push_front(addr0, id0);
        assert_eq!(queue.peek_back(addr0), Some(id0));

        queue.push_front(addr0, id1);
        assert_eq!(queue.peek_back(addr0), Some(id0));

        queue.push_front(addr0, id2);
        assert_eq!(queue.peek_back(addr0), Some(id0));

        queue.push_front(addr0, id3);
        assert_eq!(queue.peek_back(addr0), Some(id0));

        // We can perform consecutive pops and peek back
        assert_eq!(queue.pop_back(addr0), Some(id0));
        assert_eq!(queue.peek_back(addr0), Some(id1));

        assert_eq!(queue.pop_back(addr0), Some(id1));
        assert_eq!(queue.peek_back(addr0), Some(id2));

        assert_eq!(queue.pop_back(addr0), Some(id2));
        assert_eq!(queue.peek_back(addr0), Some(id3));

        assert_eq!(queue.pop_back(addr0), Some(id3));
        assert_eq!(queue.peek_back(addr0), None);
        assert_eq!(queue.pop_back(addr0), None);

        // We can perform interweaving push and pops
        queue.push_front(addr0, id3);
        assert_eq!(queue.peek_back(addr0), Some(id3));

        queue.push_front(addr0, id2);
        assert_eq!(queue.peek_back(addr0), Some(id3));

        assert_eq!(queue.pop_back(addr0), Some(id3));
        assert_eq!(queue.peek_back(addr0), Some(id2));

        queue.push_front(addr0, id1);
        assert_eq!(queue.peek_back(addr0), Some(id2));

        assert_eq!(queue.pop_back(addr0), Some(id2));
        assert_eq!(queue.peek_back(addr0), Some(id1));

        assert_eq!(queue.pop_back(addr0), Some(id1));
        assert_eq!(queue.peek_back(addr0), None);
        assert_eq!(queue.pop_back(addr0), None);
    }
}

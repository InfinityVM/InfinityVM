//! Abstraction for global handle to all queues.

use ivm_db::queue::Queue;
use reth_db::Database;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex, RwLock},
};

/// Error type for queue handle.
#[derive(thiserror::Error, Debug)]
pub enum Error {
    /// database error
    #[error("database error: {0}")]
    Database(#[from] ivm_db::Error),
}

/// A handle to all the existing relay queues.
/// 
/// We never want two tasks to use a queue at the same time because we risk corrupting the
/// queue. To solve this, we maintain a mutex for each queue, forcing only 1 user at a time.
#[derive(Debug)]
pub struct QueueHandle<D> {
    inner: RwLock<HashMap<[u8; 20], Mutex<Queue<D>>>>,
    db: Arc<D>,
}

impl<D> QueueHandle<D>
where
    D: Database,
{
    /// Peek the back of the relay queue for `consumer_address`.
    pub fn peek_back(&self, consumer_address: [u8; 20]) -> Result<Option<[u8; 32]>, Error> {
        let read_map = self.inner.read().expect("todo");
        let mutex = match read_map.get(&consumer_address) {
            None => return Ok(None),
            Some(mutex) => mutex,
        };

        let queue = mutex.lock().expect("we don't handle poisoned locks.");

        (*queue).peek_back().map_err(Into::into)
    }

    /// Pop the element off back of the relay queue for `consumer_address`.
    pub fn pop_back(&self, consumer_address: [u8; 20]) -> Result<Option<[u8; 32]>, Error> {
        let read_map = self.inner.read().expect("todo");
        let mutex = match read_map.get(&consumer_address) {
            // If we don't have an entry for the queue, we know there is nothing in it.
            None => return Ok(None),
            Some(mutex) => mutex,
        };
        
        let queue = mutex.lock().expect("we don't handle poisoned locks.");
        
        let back = queue.pop_back()?;
        // After removing the element, check if the queue is empty
        let is_empty = queue.is_empty()?;
        drop(queue);
        drop(read_map);

        if is_empty {
            // To prevent memory leaks we need to remove the queue from the map
            let mut write_map = self.inner.write().expect("todo");
            write_map.remove(&consumer_address);
        }

        return Ok(back)
    }

    /// Push an element onto the front of the queue for the given address
    pub fn push_front(&self, consumer_address: [u8; 20], job_id: [u8; 32]) -> Result<(), Error> {
        // First check if the queue exists. If it doesn't, insert it into the map
        {
            let read_map = self.inner.read().expect("todo");
            if !read_map.contains_key(&consumer_address) {
                drop(read_map);
                let queue = Mutex::new(Queue::new(self.db.clone(), consumer_address));
                let mut write_map = self.inner.write().expect("todo");
                write_map.insert(consumer_address, queue);
            }
        }

        let read_map = self.inner.read().expect("todo");
        let mutex = match read_map.get(&consumer_address) {
            // If we don't have an entry for the queue, we know there is nothing in it.
            None => unreachable!("we inserted the entry above"),
            Some(mutex) => mutex,
        };
        let queue = mutex.lock().expect("we don't handle poisoned locks.");


        (*queue).push_front(job_id).map_err(Into::into)
    }
}

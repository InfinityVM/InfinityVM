//! Database based Queue meant to track job ids.

use crate::tables::{AddrKey, B256Key, QueueMetaTable, QueueNodeTable};
use reth_db::{
    transaction::{DbTx, DbTxMut},
    Database,
};
use std::sync::Arc;

/// Metadata for queue
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct QueueMeta {
    head: Option<B256Key>,
    tail: Option<B256Key>,
}

/// A node in the queue.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct QueueNode {
    job_id: [u8; 32],
    prev: Option<B256Key>,
    next: Option<B256Key>,
}

/// In memory handle to queue. This is the abstraction used for all queue interactions.
///
/// WARNING: if you add the some job_id to two different queues, you will break the queues.
#[derive(Debug)]
pub struct Queue<D> {
    db: Arc<D>,
    key: AddrKey,
}

impl<D: Database> Queue<D> {
    /// Create a new instance of [Self].
    pub const fn new(db: Arc<D>, key: [u8; 20]) -> Self {
        Self { db, key: AddrKey(key) }
    }

    /// Push to the front of the queue.
    ///
    /// Note, this always loads metadata from DB.
    pub fn push_front(&mut self, job_id: [u8; 32]) -> Result<(), crate::Error> {
        let mut meta = self.load()?;

        let new_node = if let Some(head) = meta.head {
            // Update the current head to have a back pointer to the new node
            let mut head_node = self.db.view(|tx| tx.get::<QueueNodeTable>(head))??.expect("todo");
            head_node.prev = Some(B256Key(job_id));
            self.db.update(|tx| tx.put::<QueueNodeTable>(head, head_node))??;

            QueueNode { job_id, prev: None, next: Some(head) }
        } else {
            // There is no head or tail, so this is the only node in the queue.
            debug_assert!(meta.tail.is_none());

            meta.tail = Some(B256Key(job_id));
            QueueNode { job_id, prev: None, next: None }
        };

        // Always write the new head
        self.db.update(|tx| tx.put::<QueueNodeTable>(B256Key(job_id), new_node))??;
        meta.head = Some(B256Key(job_id));

        // Update this metadata in the db.
        self.commit(meta)?;
        Ok(())
    }

    /// Get the value the value from the back without without removing it.
    pub fn peek_back(&self) -> Result<Option<[u8; 32]>, crate::Error> {
        let meta = self.load()?;
        Ok(meta.tail.clone().map(|k| k.0))
    }

    /// Remove the value from the back.
    pub fn pop_back(&mut self) -> Result<Option<[u8; 32]>, crate::Error> {
        let mut meta = self.load()?;

        let (tail_node, result) = if let Some(tail) = meta.tail.clone() {
            let tail_node = self.db.view(|tx| tx.get::<QueueNodeTable>(tail))??.expect("todo");
            (tail_node, tail)
        } else {
            // The head and tail are not set, so this must be empty. We can return early
            debug_assert!(meta.head.is_none());
            return Ok(None);
        };

        if let Some(new_tail) = tail_node.prev {
            // Delete the old tale from the DB
            self.db.update(|tx| tx.delete::<QueueNodeTable>(B256Key(tail_node.job_id), None))??;

            // Update the new to no longer point at anything
            let mut new_tail_node =
                self.db.view(|tx| tx.get::<QueueNodeTable>(new_tail.clone()))??.expect("todo");
            new_tail_node.next = None;

            // Update the meta to point at the new tail
            meta.tail = Some(new_tail.clone());

            self.db.update(|tx| {
                tx.delete::<QueueNodeTable>(B256Key(tail_node.job_id), None)?;
                tx.put::<QueueNodeTable>(new_tail, new_tail_node)
            })??;
        } else {
            // This was the only node. The queue is now empty
            debug_assert_eq!(meta.head, meta.tail);

            meta.head = None;
            meta.tail = None;
        }

        self.commit(meta)?;
        Ok(Some(result.0))
    }

    /// Load a queue handle from the DB. Will return an empty queue meta if one does not exist.
    fn load(&self) -> Result<QueueMeta, crate::Error> {
        let meta = self.db.view(|tx| tx.get::<QueueMetaTable>(self.key))??.unwrap_or_default();
        Ok(meta)
    }
  
    /// Commit the metadata to the DB.
    fn commit(&mut self, meta: QueueMeta) -> Result<(), crate::Error> {
        self.db.update(|tx| tx.put::<QueueMetaTable>(self.key, meta))??;
        Ok(())
    }
}


#[cfg(test)]
mod test {
    use std::sync::Arc;
    use reth_db::{DatabaseEnv};
    use tempfile::TempDir;
    use crate::init_db;

    use super::Queue;

    fn db() -> (Arc<DatabaseEnv>, TempDir) {
      let tmp_dir = TempDir::new().unwrap();
      let db = init_db(tmp_dir.path()).unwrap();
      (db, tmp_dir)
    }


  #[test]
  fn single_node_queue() {
    let (db, dir) = db();
    let addr0 = [0u8; 20];
    let id0 = [0u8; 32];

    let mut queue = Queue::new(db, addr0);

    // An unitialized queue is empty.
    assert!(queue.peek_back().unwrap().is_none());
    assert!(queue.pop_back().unwrap().is_none());

    // We can add an element to the queue
    queue.push_front(id0).unwrap();

    // We can see the single element on the queue
    assert_eq!(queue.peek_back().unwrap(), Some(id0));

    // We can remove the single element from the queue
    assert_eq!(queue.pop_back().unwrap(), Some(id0));

    // The queue is now empty
    assert!(queue.peek_back().unwrap().is_none());
    assert!(queue.pop_back().unwrap().is_none());

    // Explicitly remove the temp dir so we can catch any errors
    dir.close().unwrap()
  }
}

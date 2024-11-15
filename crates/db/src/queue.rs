//! Database based Queue meant to track job ids.

use crate::tables::{AddrKey, B256Key, QueueMetaTable, QueueNodeTable};
use reth_db::{
    transaction::{DbTx, DbTxMut},
    Database,
};
use std::sync::Arc;

/// In memory handle to queue. This is the abstraction used for all queue interactions.
///
/// Each interaction reads the queue from the DB and writes any updates. If the queue
/// is empty, it will be deleted from the DB. When using an empty queue, the first
/// [`Self::push_front`] will write it to the DB.
///
/// WARNING: if you add the some `job_id` to two different queues, you will break the queues.
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
    /// Note, this always loads metadata from DB and if none exists it will create it.
    pub fn push_front(&self, job_id: [u8; 32]) -> Result<(), crate::Error> {
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
        Ok(meta.tail.map(|k| k.0))
    }

    /// Returns true if the queue is empty.
    pub fn is_empty(&self) -> Result<bool, crate::Error> {
        self.peek_back().map(|tail| tail.is_none())
    }

    /// Remove the value from the back.
    pub fn pop_back(&self) -> Result<Option<[u8; 32]>, crate::Error> {
        let mut meta = self.load()?;

        let (tail_node, result) = if let Some(tail) = meta.tail {
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
                self.db.view(|tx| tx.get::<QueueNodeTable>(new_tail))??.expect("todo");
            new_tail_node.next = None;

            // Update the meta to point at the new tail
            meta.tail = Some(new_tail);

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
    fn commit(&self, meta: QueueMeta) -> Result<(), crate::Error> {
        if meta.tail.is_none() {
            // The queue is empty so we can delete it from the DB.
            debug_assert!(meta.head.is_none());
            self.db.update(|tx| tx.delete::<QueueMetaTable>(self.key, None))??;
        } else {
            // There are some items in the queue, so make sure to update the existing entry.
            self.db.update(|tx| tx.put::<QueueMetaTable>(self.key, meta))??;
        }

        Ok(())
    }
}

/// Metadata for queue.
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

#[cfg(test)]
mod test {
    use super::Queue;
    use crate::{
        init_db,
        tables::{AddrKey, QueueMetaTable},
    };
    use reth_db::{transaction::DbTx, Database, DatabaseEnv};
    use std::sync::Arc;
    use tempfile::TempDir;

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

        let queue = Queue::new(db, addr0);

        // An uninitialized queue is empty.
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

    #[test]
    fn push_pop() {
        let (db, dir) = db();
        let addr0 = [0u8; 20];
        let id0 = [0u8; 32];
        let id1 = [1u8; 32];
        let id2 = [2u8; 32];
        let id3 = [3u8; 32];

        let queue = Queue::new(db.clone(), addr0);

        // We can perform consecutive pushes and peek back
        queue.push_front(id0).unwrap();
        assert_eq!(queue.peek_back().unwrap(), Some(id0));

        queue.push_front(id1).unwrap();
        assert_eq!(queue.peek_back().unwrap(), Some(id0));

        queue.push_front(id2).unwrap();
        assert_eq!(queue.peek_back().unwrap(), Some(id0));

        queue.push_front(id3).unwrap();
        assert_eq!(queue.peek_back().unwrap(), Some(id0));

        // We can perform consecutive pops and peek back
        assert_eq!(queue.pop_back().unwrap(), Some(id0));
        assert_eq!(queue.peek_back().unwrap(), Some(id1));

        assert_eq!(queue.pop_back().unwrap(), Some(id1));
        assert_eq!(queue.peek_back().unwrap(), Some(id2));

        assert_eq!(queue.pop_back().unwrap(), Some(id2));
        assert_eq!(queue.peek_back().unwrap(), Some(id3));

        assert_eq!(queue.pop_back().unwrap(), Some(id3));
        assert_eq!(queue.peek_back().unwrap(), None);
        assert_eq!(queue.pop_back().unwrap(), None);

        // We can perform interweaving push and pops
        queue.push_front(id3).unwrap();
        assert_eq!(queue.peek_back().unwrap(), Some(id3));

        queue.push_front(id2).unwrap();
        assert_eq!(queue.peek_back().unwrap(), Some(id3));

        assert_eq!(queue.pop_back().unwrap(), Some(id3));
        assert_eq!(queue.peek_back().unwrap(), Some(id2));

        queue.push_front(id1).unwrap();
        assert_eq!(queue.peek_back().unwrap(), Some(id2));

        assert_eq!(queue.pop_back().unwrap(), Some(id2));
        assert_eq!(queue.peek_back().unwrap(), Some(id1));

        // Right before the queue is empty, the metadata exists
        let meta =
            db.view(|tx| tx.get::<QueueMetaTable>(AddrKey(addr0))).unwrap().unwrap().unwrap();
        assert_eq!(meta.head, meta.tail);
        assert!(meta.head.is_some());

        assert_eq!(queue.pop_back().unwrap(), Some(id1));
        assert_eq!(queue.peek_back().unwrap(), None);
        assert_eq!(queue.pop_back().unwrap(), None);

        // Once the queue is empty, it no longer exists in the DB
        let option = db.view(|tx| tx.get::<QueueMetaTable>(AddrKey(addr0))).unwrap().unwrap();
        assert!(option.is_none());

        // Explicitly remove the temp dir so we can catch any errors
        dir.close().unwrap()
    }
}

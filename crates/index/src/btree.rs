//! B+Tree — Phase 2 implementation lives here.
//!
//! Exposes `insert`, `search`, `delete`, and `range_scan` over a
//! `storage::BufferPoolManager`.  Generic over key type `K: Ord + Clone`.

use storage::PageId;

/// A disk-backed B+Tree index.
///
/// Nodes are stored as pages managed by the buffer pool; no raw disk I/O
/// happens inside this crate.
pub struct BTree<K> {
    /// Root page id (may change on root split).
    _root: PageId,
    /// Phantom data so the generic parameter K is used.
    _marker: std::marker::PhantomData<K>,
}

impl<K: Ord + Clone> BTree<K> {
    /// Opens or creates a B+Tree rooted at `root_page_id`.
    pub fn new(root_page_id: PageId) -> Self {
        Self {
            _root: root_page_id,
            _marker: std::marker::PhantomData,
        }
    }

    /// Inserts `(key, value)` into the tree. Phase 2 implementation.
    pub fn insert(&mut self, _key: K, _value: u64) {
        // Phase 2 implementation.
    }

    /// Searches for `key`. Returns `Some(value)` if found. Phase 2 implementation.
    pub fn search(&self, _key: &K) -> Option<u64> {
        // Phase 2 implementation.
        None
    }

    /// Deletes `key` from the tree. Phase 2 implementation.
    pub fn delete(&mut self, _key: &K) -> bool {
        // Phase 2 implementation.
        false
    }

    /// Returns an iterator over `(key, value)` pairs in `[start, end]`.
    /// Phase 2 implementation.
    pub fn range_scan(&self, _start: &K, _end: &K) -> Vec<(K, u64)> {
        // Phase 2 implementation.
        vec![]
    }
}

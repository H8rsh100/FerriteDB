//! LockManager — shared/exclusive row locks with deadlock detection.
//!
//! Used as the concurrency control mechanism (alternative to MVCC).
//! Phase 7 will choose one; both are scaffolded so the decision can be
//! made after benchmarking.
//!
//! Deadlock detection: wait-for graph — a directed edge (T1 → T2) means
//! T1 is waiting on a lock held by T2. A cycle = deadlock; abort the
//! youngest transaction.

use crate::TxnId;
use std::collections::HashMap;

/// Lock mode requested by a transaction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LockMode {
    Shared,
    Exclusive,
}

/// A request for a lock on a specific resource.
#[derive(Debug)]
pub struct LockRequest {
    pub txn_id: TxnId,
    pub mode: LockMode,
}

/// Manages lock grants and the wait-for graph.
pub struct LockManager {
    /// Map from resource key to the queue of lock requests.
    _lock_table: HashMap<u64, Vec<LockRequest>>,
}

impl LockManager {
    pub fn new() -> Self {
        Self { _lock_table: HashMap::new() }
    }

    /// Attempts to acquire `mode` lock on `resource_id` for `txn_id`.
    /// Blocks until granted or deadlock is detected.
    /// Phase 7 implementation.
    pub fn acquire(&mut self, _txn_id: TxnId, _resource_id: u64, _mode: LockMode)
        -> Result<(), String>
    {
        Ok(()) // Phase 7 implementation.
    }

    /// Releases all locks held by `txn_id`.
    /// Phase 7 implementation.
    pub fn release_all(&mut self, _txn_id: TxnId) {
        // Phase 7 implementation.
    }
}

impl Default for LockManager {
    fn default() -> Self { Self::new() }
}

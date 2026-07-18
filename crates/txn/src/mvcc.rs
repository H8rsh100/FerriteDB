//! MVCC — Multi-Version Concurrency Control. Phase 7 implementation.
//!
//! Design choice: MVCC over 2PL because:
//! - Readers never block writers (no shared locks on rows).
//! - Writers only conflict with concurrent writers on the *same* row.
//! - Snapshot isolation is straightforward to reason about and test.
//!
//! Each tuple version carries `(min_txn_id, max_txn_id)`:
//!   visible iff min_txn_id <= current_txn_id < max_txn_id.

use crate::TxnId;

/// A versioned tuple wrapper.
#[derive(Debug, Clone)]
pub struct VersionedTuple {
    /// Transaction that created this version.
    pub created_by: TxnId,
    /// Transaction that deleted/superseded this version (u64::MAX = still live).
    pub deleted_by: TxnId,
    /// Payload bytes.
    pub data: Vec<u8>,
}

impl VersionedTuple {
    pub fn is_visible(&self, txn_id: TxnId) -> bool {
        self.created_by <= txn_id && txn_id < self.deleted_by
    }
}

//! Txn crate — WAL, crash recovery, and concurrency control.
//!
//! Phase 7 will implement Write-Ahead Logging and either MVCC or 2PL.
//! For Phase 0 the module tree and public API are declared.

pub mod wal;
pub mod mvcc;
pub mod lock_manager;

pub use wal::Wal;
pub use lock_manager::LockManager;

/// A monotonically increasing transaction identifier.
pub type TxnId = u64;

/// Handle to an active transaction — passed through exec operators.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Transaction {
    pub id: TxnId,
}

//! Write-Ahead Log — Phase 7 implementation.
//!
//! Every page modification is preceded by appending a log record here.
//! On startup, committed records are replayed (redo); uncommitted are undone.

use storage::PageId;
use crate::TxnId;

/// A single WAL record.
#[derive(Debug, Clone)]
pub struct LogRecord {
    pub txn_id: TxnId,
    pub page_id: PageId,
    pub record_type: LogRecordType,
}

/// Discriminates WAL record kinds.
#[derive(Debug, Clone)]
pub enum LogRecordType {
    /// Transaction started.
    Begin,
    /// Page was modified. Stores before and after images for undo/redo.
    Update {
        before: Vec<u8>,
        after: Vec<u8>,
    },
    /// Transaction committed — all updates are durable.
    Commit,
    /// Transaction aborted — undo all updates.
    Abort,
}

/// Manages the WAL file.
pub struct Wal {
    _records: Vec<LogRecord>,
}

impl Wal {
    pub fn new() -> Self {
        Self { _records: Vec::new() }
    }

    /// Appends a log record and flushes to disk before returning.
    /// Phase 7 implementation.
    pub fn append(&mut self, _record: LogRecord) -> Result<(), std::io::Error> {
        Ok(()) // Phase 7 implementation.
    }

    /// Replays committed transactions and undoes uncommitted ones.
    /// Phase 7 implementation.
    pub fn recover(&self) -> Result<(), std::io::Error> {
        Ok(()) // Phase 7 implementation.
    }
}

impl Default for Wal {
    fn default() -> Self { Self::new() }
}

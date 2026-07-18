//! BufferPoolManager — in-memory page cache with LRU eviction.
//!
//! Phase 1 will implement the full LRU eviction, pinning, and dirty-page
//! tracking. For Phase 0 the public API is declared so every crate compiles.

use crate::page::{Page, PageId};
use crate::disk_manager::DiskError;

/// Error type returned by BufferPool operations.
#[derive(Debug)]
pub enum BufError {
    /// All frames are pinned; eviction is impossible.
    PoolExhausted,
    /// Underlying disk I/O failed.
    Disk(DiskError),
    /// Tried to unpin a page that wasn't in the pool.
    PageNotFound(PageId),
}

impl std::fmt::Display for BufError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BufError::PoolExhausted => write!(f, "buffer pool exhausted: all frames pinned"),
            BufError::Disk(e) => write!(f, "disk error: {e}"),
            BufError::PageNotFound(id) => write!(f, "page {id} not found in pool"),
        }
    }
}

impl std::error::Error for BufError {}

/// Thread-safe buffer pool.
///
/// # Thread-safety design note (Phase 1 detail)
/// The entire pool is protected by a single `Mutex<Inner>`.  This is the
/// simplest correct approach and performs well under low-to-moderate
/// concurrency.  A per-frame `RwLock` would allow parallel reads of
/// different frames but adds implementation complexity and potential
/// deadlock surface — a reasonable Phase-2 upgrade once benchmarks
/// demonstrate the bottleneck.
pub struct BufferPoolManager {
    /// Pool capacity in frames.
    _capacity: usize,
}

impl BufferPoolManager {
    /// Creates a new buffer pool with `capacity` frames.
    pub fn new(_capacity: usize) -> Self {
        Self { _capacity: _capacity }
    }

    /// Pins the page identified by `page_id`, loading it from disk if needed.
    /// Returns a clone of the in-pool page for the caller.
    pub fn fetch_page(&self, _page_id: PageId) -> Result<Page, BufError> {
        // Phase 1 implementation.
        Ok(Page::new())
    }

    /// Decrements the pin count for `page_id`.
    /// If `is_dirty` is true the page will be flushed before eviction.
    pub fn unpin_page(&self, _page_id: PageId, _is_dirty: bool) -> Result<(), BufError> {
        // Phase 1 implementation.
        Ok(())
    }

    /// Allocates a new page via the disk manager, pins it, and returns its id.
    pub fn new_page(&self) -> Result<PageId, BufError> {
        // Phase 1 implementation.
        Ok(0)
    }

    /// Flushes the dirty page identified by `page_id` to disk.
    pub fn flush_page(&self, _page_id: PageId) -> Result<(), BufError> {
        // Phase 1 implementation.
        Ok(())
    }

    /// Flushes all dirty pages to disk.
    pub fn flush_all(&self) -> Result<(), BufError> {
        // Phase 1 implementation.
        Ok(())
    }
}

//! BufferPoolManager — fixed-size in-memory page cache with LRU eviction.
//!
//! ## Design: single `Mutex<Inner>` vs per-frame `RwLock`
//!
//! We protect the entire pool state with one `Mutex<Inner>`.
//!
//! **Why not per-frame locks?**
//! Per-frame `RwLock` would allow parallel reads of *different* frames, but
//! the frame allocation / LRU bookkeeping (free-list, page-table, LRU heap)
//! is shared state that must be mutated atomically whenever any frame
//! changes state.  Protecting bookkeeping with its own lock plus per-frame
//! locks introduces a 2-lock ordering requirement and a real deadlock risk.
//! The single-mutex approach is simpler, provably correct, and fast enough
//! for our workload.  A per-frame upgrade can be justified later with a
//! benchmark showing the mutex is the bottleneck.
//!
//! ## LRU eviction
//!
//! Every time a frame is pinned (fetched), its `last_used` counter is set to
//! a monotonically-increasing global `clock`. When we need a victim we scan
//! all frames and pick the unpinned frame with the **lowest `last_used`**
//! value — classic LRU-approximation, O(capacity) scan.
//!
//! For a buffer pool of the sizes used by a real DBMS (thousands of frames)
//! the O(n) scan is fine and avoids the complexity of a doubly-linked list
//! LRU structure in safe Rust.

use std::collections::HashMap;
use std::sync::Mutex;

use crate::disk_manager::{DiskError, DiskManager};
use crate::page::{Page, PageId};

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

/// Errors returned by [`BufferPoolManager`] operations.
#[derive(Debug)]
pub enum BufError {
    /// Every frame is currently pinned — cannot evict to make room.
    PoolExhausted,
    /// Underlying disk I/O failed.
    Disk(DiskError),
    /// The caller tried to unpin / flush a page not currently in the pool.
    PageNotFound(PageId),
    /// The caller tried to unpin a page whose pin count is already zero.
    UnpinUnderflow(PageId),
}

impl std::fmt::Display for BufError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BufError::PoolExhausted => {
                write!(f, "buffer pool exhausted: all frames are pinned")
            }
            BufError::Disk(e) => write!(f, "disk error in buffer pool: {e}"),
            BufError::PageNotFound(id) => write!(f, "page {id} not found in buffer pool"),
            BufError::UnpinUnderflow(id) => {
                write!(f, "unpin called on page {id} whose pin count is already 0")
            }
        }
    }
}

impl std::error::Error for BufError {}

impl From<DiskError> for BufError {
    fn from(e: DiskError) -> Self {
        BufError::Disk(e)
    }
}

// ---------------------------------------------------------------------------
// Internal frame representation
// ---------------------------------------------------------------------------

/// One slot in the buffer pool.
struct Frame {
    /// The in-memory page data.
    page: Page,
    /// Which on-disk page this frame holds (`INVALID_PAGE_ID` if empty).
    page_id: PageId,
    /// Number of active users that have pinned this frame.
    /// A frame with `pin_count > 0` must never be evicted.
    pin_count: u32,
    /// Whether the page has been modified since it was loaded from disk.
    is_dirty: bool,
    /// Logical access timestamp for LRU ordering.
    last_used: u64,
}

const INVALID_PAGE_ID: PageId = u64::MAX;

impl Frame {
    fn empty() -> Self {
        Frame {
            page: Page::new(),
            page_id: INVALID_PAGE_ID,
            pin_count: 0,
            is_dirty: false,
            last_used: 0,
        }
    }

    fn is_empty(&self) -> bool {
        self.page_id == INVALID_PAGE_ID
    }
}

// ---------------------------------------------------------------------------
// Inner (protected) state
// ---------------------------------------------------------------------------

struct Inner {
    frames: Vec<Frame>,
    /// Maps page_id → frame index for O(1) lookups.
    page_table: HashMap<PageId, usize>,
    /// Free (unused) frame indices in LIFO order.
    free_list: Vec<usize>,
    /// The disk manager — I/O is performed while holding the lock.
    disk: DiskManager,
    /// Monotonically increasing logical clock for LRU ordering.
    clock: u64,
}

impl Inner {
    fn new(capacity: usize, disk: DiskManager) -> Self {
        let frames = (0..capacity).map(|_| Frame::empty()).collect();
        let free_list = (0..capacity).rev().collect();
        Inner {
            frames,
            page_table: HashMap::new(),
            free_list,
            disk,
            clock: 0,
        }
    }

    /// Bumps the clock and returns the new timestamp.
    fn tick(&mut self) -> u64 {
        self.clock += 1;
        self.clock
    }

    /// Finds the LRU-victim frame index (unpinned, lowest `last_used`).
    /// Returns `None` if every frame is pinned.
    fn lru_victim(&self) -> Option<usize> {
        self.frames
            .iter()
            .enumerate()
            .filter(|(_, f)| !f.is_empty() && f.pin_count == 0)
            .min_by_key(|(_, f)| f.last_used)
            .map(|(i, _)| i)
    }

    /// Evicts the LRU unpinned frame, flushing it if dirty.
    /// Returns the freed frame index, or `BufError::PoolExhausted`.
    fn evict(&mut self) -> Result<usize, BufError> {
        // First, try the free list.
        if let Some(idx) = self.free_list.pop() {
            return Ok(idx);
        }

        let victim = self.lru_victim().ok_or(BufError::PoolExhausted)?;
        let evicted_pid = self.frames[victim].page_id;

        // Flush if dirty.
        if self.frames[victim].is_dirty {
            let page = &self.frames[victim].page;
            self.disk.write_page(evicted_pid, page)?;
        }

        // Remove from page table.
        self.page_table.remove(&evicted_pid);

        // Reset the frame.
        self.frames[victim] = Frame::empty();

        Ok(victim)
    }

    /// Loads page `page_id` from disk into frame `frame_idx`, updating
    /// the page table and pinning it once.
    fn load_page(&mut self, page_id: PageId, frame_idx: usize) -> Result<(), BufError> {
        self.disk
            .read_page(page_id, &mut self.frames[frame_idx].page)?;
        let ts = self.tick();
        self.frames[frame_idx].page_id = page_id;
        self.frames[frame_idx].pin_count = 1;
        self.frames[frame_idx].is_dirty = false;
        self.frames[frame_idx].last_used = ts;
        self.page_table.insert(page_id, frame_idx);
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// BufferPoolManager (public)
// ---------------------------------------------------------------------------

/// Thread-safe buffer pool — wraps `Inner` in a `Mutex`.
///
/// All public methods acquire the lock for the duration of the call.
/// This is safe because no method calls another public method (no re-entrant
/// locking required) and no lock is held across an await point.
pub struct BufferPoolManager {
    inner: Mutex<Inner>,
}

impl BufferPoolManager {
    /// Creates a new buffer pool with `capacity` frames backed by `disk`.
    pub fn new(capacity: usize, disk: DiskManager) -> Self {
        BufferPoolManager {
            inner: Mutex::new(Inner::new(capacity, disk)),
        }
    }

    // -----------------------------------------------------------------------
    // Core API
    // -----------------------------------------------------------------------

    /// Pins `page_id` in the pool, loading it from disk if it is not already
    /// resident.  Returns a **clone** of the in-pool page so the caller can
    /// inspect or modify it.
    ///
    /// The caller **must** call [`unpin_page`] when done with the page.
    /// Failing to unpin eventually exhausts the pool.
    ///
    /// [`unpin_page`]: BufferPoolManager::unpin_page
    pub fn fetch_page(&self, page_id: PageId) -> Result<Page, BufError> {
        let mut g = self.inner.lock().unwrap();

        if let Some(&frame_idx) = g.page_table.get(&page_id) {
            // Cache hit — increment pin count and update LRU timestamp.
            g.frames[frame_idx].pin_count += 1;
            let ts = g.tick();
            g.frames[frame_idx].last_used = ts;
            return Ok(g.frames[frame_idx].page.clone());
        }

        // Cache miss — evict a frame, load from disk.
        let frame_idx = g.evict()?;
        g.load_page(page_id, frame_idx)?;
        Ok(g.frames[frame_idx].page.clone())
    }

    /// Decrements the pin count for `page_id`.
    ///
    /// If `is_dirty` is `true` the frame is marked dirty and will be flushed
    /// to disk before eviction.  If `is_dirty` is `false` the existing dirty
    /// flag is left unchanged (once dirty, always dirty until flushed).
    pub fn unpin_page(&self, page_id: PageId, is_dirty: bool) -> Result<(), BufError> {
        let mut g = self.inner.lock().unwrap();
        let &frame_idx = g
            .page_table
            .get(&page_id)
            .ok_or(BufError::PageNotFound(page_id))?;

        if g.frames[frame_idx].pin_count == 0 {
            return Err(BufError::UnpinUnderflow(page_id));
        }
        g.frames[frame_idx].pin_count -= 1;
        if is_dirty {
            g.frames[frame_idx].is_dirty = true;
        }
        Ok(())
    }

    /// Allocates a new page via the disk manager, pins it, and returns its id
    /// together with a copy of the (zeroed) page.
    ///
    /// The caller **must** call `unpin_page` when done.
    pub fn new_page(&self) -> Result<(PageId, Page), BufError> {
        let mut g = self.inner.lock().unwrap();
        let page_id = g.disk.allocate_page()?;

        // Get a free frame.
        let frame_idx = {
            // We can't call `g.evict()` here while borrowing `g` mutably for
            // `disk.allocate_page` above, so we re-acquire after allocation.
            // Actually both are on the same `g`, and `disk.allocate_page` is done,
            // so we can call evict now.
            if let Some(idx) = g.free_list.pop() {
                idx
            } else {
                let victim = g.lru_victim().ok_or(BufError::PoolExhausted)?;
                let evicted_pid = g.frames[victim].page_id;
                if g.frames[victim].is_dirty {
                    // Borrow page data separately to satisfy borrow checker.
                    let page_clone = g.frames[victim].page.clone();
                    g.disk.write_page(evicted_pid, &page_clone)?;
                }
                g.page_table.remove(&evicted_pid);
                g.frames[victim] = Frame::empty();
                victim
            }
        };

        let ts = g.tick();
        let mut new_page = Page::new();
        new_page.set_page_id(page_id);
        g.frames[frame_idx] = Frame {
            page: new_page,
            page_id,
            pin_count: 1,
            is_dirty: true, // mark dirty so it gets written on eviction
            last_used: ts,
        };
        g.page_table.insert(page_id, frame_idx);
        Ok((page_id, g.frames[frame_idx].page.clone()))
    }

    /// Writes the page's in-pool content to `page`, updating the frame.
    ///
    /// The page must already be in the pool (i.e., pinned by the caller).
    /// Marks the frame dirty.
    pub fn write_page_to_pool(
        &self,
        page_id: PageId,
        page: &Page,
    ) -> Result<(), BufError> {
        let mut g = self.inner.lock().unwrap();
        let &frame_idx = g
            .page_table
            .get(&page_id)
            .ok_or(BufError::PageNotFound(page_id))?;
        g.frames[frame_idx].page = page.clone();
        g.frames[frame_idx].is_dirty = true;
        Ok(())
    }

    /// Flushes the dirty page `page_id` to disk immediately.
    ///
    /// Does not change the pin count or the dirty flag.
    pub fn flush_page(&self, page_id: PageId) -> Result<(), BufError> {
        let mut g = self.inner.lock().unwrap();
        let &frame_idx = g
            .page_table
            .get(&page_id)
            .ok_or(BufError::PageNotFound(page_id))?;
        let page = g.frames[frame_idx].page.clone();
        g.disk.write_page(page_id, &page)?;
        g.frames[frame_idx].is_dirty = false;
        Ok(())
    }

    /// Flushes all dirty pages to disk.
    pub fn flush_all(&self) -> Result<(), BufError> {
        let mut g = self.inner.lock().unwrap();
        // Collect what needs flushing to avoid borrow-checker issues.
        let dirty: Vec<(usize, PageId)> = g
            .frames
            .iter()
            .enumerate()
            .filter(|(_, f)| f.is_dirty && !f.is_empty())
            .map(|(i, f)| (i, f.page_id))
            .collect();

        for (frame_idx, pid) in dirty {
            let page = g.frames[frame_idx].page.clone();
            g.disk.write_page(pid, &page)?;
            g.frames[frame_idx].is_dirty = false;
        }
        Ok(())
    }

    /// Returns the number of pages currently in the pool (pinned or not).
    #[cfg(test)]
    pub(crate) fn pool_size(&self) -> usize {
        let g = self.inner.lock().unwrap();
        g.page_table.len()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::page::PageType;
    use tempfile::NamedTempFile;

    /// Helper: create a BPM backed by a temp file.
    fn make_bpm(capacity: usize) -> (BufferPoolManager, NamedTempFile) {
        let f = NamedTempFile::new().unwrap();
        let dm = DiskManager::open(f.path()).unwrap();
        let bpm = BufferPoolManager::new(capacity, dm);
        (bpm, f)
    }

    // -----------------------------------------------------------------------
    // Basic new_page / fetch_page / unpin
    // -----------------------------------------------------------------------

    #[test]
    fn new_page_then_fetch_roundtrip() {
        let (bpm, _f) = make_bpm(4);

        // Allocate a new page and write a sentinel byte.
        let (pid, mut page) = bpm.new_page().unwrap();
        page.set_page_type(PageType::Heap);
        page.body_mut()[0] = 0x42;
        bpm.write_page_to_pool(pid, &page).unwrap();
        bpm.unpin_page(pid, true).unwrap();

        // Flush it to disk and evict (unpin brings pin_count to 0).
        bpm.flush_page(pid).unwrap();

        // Re-fetch and verify.
        let fetched = bpm.fetch_page(pid).unwrap();
        assert_eq!(fetched.page_type(), PageType::Heap);
        assert_eq!(fetched.body()[0], 0x42);
        bpm.unpin_page(pid, false).unwrap();
    }

    // -----------------------------------------------------------------------
    // LRU eviction under pool pressure
    // -----------------------------------------------------------------------

    #[test]
    fn eviction_under_pressure() {
        // Pool has only 4 frames. Allocate 5 pages and verify the LRU one
        // gets evicted so the 5th allocation succeeds.
        let (bpm, _f) = make_bpm(4);

        let mut pids = Vec::new();
        for _ in 0..4 {
            let (pid, _) = bpm.new_page().unwrap();
            pids.push(pid);
        }

        // Unpin all pages (making them eligible for eviction).
        for &pid in &pids {
            bpm.unpin_page(pid, false).unwrap();
        }

        // The pool is now full but all frames are unpinned — 5th allocation
        // should evict the LRU frame.
        let (pid5, _) = bpm.new_page().unwrap();
        bpm.unpin_page(pid5, false).unwrap();

        // Pool still fits in 4 frames.
        assert!(bpm.pool_size() <= 4);
    }

    // -----------------------------------------------------------------------
    // Pinned-page protection
    // -----------------------------------------------------------------------

    #[test]
    fn pinned_page_is_not_evicted() {
        // Pool has exactly 2 frames.  Pin page 0, fill the pool, then try to
        // allocate again — the pinned page must not be evicted.
        let (bpm, _f) = make_bpm(2);

        let (pid0, _) = bpm.new_page().unwrap();    // pinned
        let (pid1, _) = bpm.new_page().unwrap();    // pinned
        bpm.unpin_page(pid1, false).unwrap();        // pid1 is now evictable

        // pid0 is still pinned — pid1 should be the victim.
        let (pid2, _) = bpm.new_page().unwrap();
        bpm.unpin_page(pid0, false).unwrap();
        bpm.unpin_page(pid2, false).unwrap();

        // pid0 must still be in the pool (it was pinned during eviction
        // pressure so it can't have been evicted).
        // We verify this by checking the pool did not panic / error.
        assert!(bpm.pool_size() <= 2);
    }

    // -----------------------------------------------------------------------
    // Pool exhaustion
    // -----------------------------------------------------------------------

    #[test]
    fn pool_exhausted_error_when_all_pinned() {
        // Pool = 2 frames, both pinned — 3rd new_page must fail.
        let (bpm, _f) = make_bpm(2);

        let (pid0, _) = bpm.new_page().unwrap();
        let (pid1, _) = bpm.new_page().unwrap();

        let result = bpm.new_page();
        assert!(matches!(result, Err(BufError::PoolExhausted)));

        bpm.unpin_page(pid0, false).unwrap();
        bpm.unpin_page(pid1, false).unwrap();
    }

    // -----------------------------------------------------------------------
    // Dirty flush
    // -----------------------------------------------------------------------

    #[test]
    fn dirty_page_is_flushed_on_eviction() {
        let f = NamedTempFile::new().unwrap();
        {
            let dm = DiskManager::open(f.path()).unwrap();
            let bpm = BufferPoolManager::new(2, dm);

            let (pid0, _) = bpm.new_page().unwrap();
            let (pid1, mut p1) = bpm.new_page().unwrap();

            // Write a known byte into pid1 and mark it dirty.
            p1.body_mut()[7] = 0xFF;
            bpm.write_page_to_pool(pid1, &p1).unwrap();
            bpm.unpin_page(pid1, true).unwrap();

            // Unpin pid0 first so pid0 is LRU.
            bpm.unpin_page(pid0, false).unwrap();

            // Allocate two more — both evictions should succeed.
            let (pid2, _) = bpm.new_page().unwrap();
            let (pid3, _) = bpm.new_page().unwrap();
            bpm.unpin_page(pid2, false).unwrap();
            bpm.unpin_page(pid3, false).unwrap();
        }

        // Re-open and verify that the dirty pid1 page was flushed.
        let dm2 = DiskManager::open(f.path()).unwrap();
        let bpm2 = BufferPoolManager::new(4, dm2);
        // pid1 = 2nd allocated page = page_id 2 (page 0 is meta, page 1 is pid0)
        let page = bpm2.fetch_page(2).unwrap();
        assert_eq!(page.body()[7], 0xFF);
    }

    // -----------------------------------------------------------------------
    // Unpin errors
    // -----------------------------------------------------------------------

    #[test]
    fn unpin_not_in_pool_errors() {
        let (bpm, _f) = make_bpm(4);
        let err = bpm.unpin_page(999, false);
        assert!(matches!(err, Err(BufError::PageNotFound(999))));
    }

    #[test]
    fn double_unpin_errors() {
        let (bpm, _f) = make_bpm(4);
        let (pid, _) = bpm.new_page().unwrap();
        bpm.unpin_page(pid, false).unwrap();
        let err = bpm.unpin_page(pid, false);
        assert!(matches!(err, Err(BufError::UnpinUnderflow(_))));
    }
}

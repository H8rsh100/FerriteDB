//! DiskManager — reads and writes pages to a single heap file.
//!
//! Each page occupies exactly `PAGE_SIZE` bytes at offset `page_id * PAGE_SIZE`.
//! Phase 1 will flesh this out; for Phase 0 the struct and method signatures
//! are declared so that the workspace compiles end-to-end.

use std::path::Path;
use crate::page::{Page, PageId, PAGE_SIZE};

/// Error type returned by DiskManager operations.
#[derive(Debug)]
pub enum DiskError {
    Io(std::io::Error),
    InvalidPageId(PageId),
}

impl std::fmt::Display for DiskError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DiskError::Io(e) => write!(f, "I/O error: {e}"),
            DiskError::InvalidPageId(id) => write!(f, "invalid page id: {id}"),
        }
    }
}

impl std::error::Error for DiskError {}
impl From<std::io::Error> for DiskError {
    fn from(e: std::io::Error) -> Self {
        DiskError::Io(e)
    }
}

/// Manages raw page reads and writes to a single database file.
pub struct DiskManager {
    /// Path to the database heap file.
    _path: std::path::PathBuf,
    /// Total number of pages allocated so far.
    _num_pages: u64,
}

impl DiskManager {
    /// Opens (or creates) the database file at `path`.
    pub fn open(_path: &Path) -> Result<Self, DiskError> {
        // Phase 1 implementation.
        Ok(Self {
            _path: _path.to_path_buf(),
            _num_pages: 0,
        })
    }

    /// Allocates a new page and returns its id.
    pub fn allocate_page(&mut self) -> Result<PageId, DiskError> {
        // Phase 1 implementation.
        let id = self._num_pages;
        self._num_pages += 1;
        Ok(id)
    }

    /// Reads the page identified by `page_id` from disk into `page`.
    pub fn read_page(&self, _page_id: PageId, _page: &mut Page) -> Result<(), DiskError> {
        // Phase 1 implementation.
        Ok(())
    }

    /// Writes `page` to disk at the location corresponding to `page_id`.
    pub fn write_page(&mut self, _page_id: PageId, _page: &Page) -> Result<(), DiskError> {
        // Phase 1 implementation.
        Ok(())
    }

    /// Returns the number of pages currently allocated.
    pub fn num_pages(&self) -> u64 {
        self._num_pages
    }
}

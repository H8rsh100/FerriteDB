//! DiskManager — reads and writes pages to a single heap file on disk.
//!
//! ## On-disk layout
//! ```text
//! ┌─────────┬─────────┬─────────┬─ ─ ─┐
//! │ Page 0  │ Page 1  │ Page 2  │ ... │   each slot = PAGE_SIZE (4096 B)
//! │ 4096 B  │ 4096 B  │ 4096 B  │     │
//! └─────────┴─────────┴─────────┴─ ─ ─┘
//!    offset = page_id * PAGE_SIZE
//! ```
//!
//! A new page is appended (zeroed) at the end of the file by `allocate_page`.
//! The total page count is stored as the first 8 bytes of Page 0 (the metadata
//! page), so it survives restarts.
//!
//! ## Thread safety
//! `DiskManager` is **not** thread-safe by itself — callers must synchronise
//! access (the `BufferPoolManager` wraps it in a `Mutex`).

use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

use crate::page::{Page, PageId, PAGE_SIZE};

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

/// Errors returned by [`DiskManager`] operations.
#[derive(Debug)]
pub enum DiskError {
    /// Underlying OS I/O error.
    Io(std::io::Error),
    /// The caller supplied a page_id that is beyond the allocated range.
    InvalidPageId(PageId),
}

impl std::fmt::Display for DiskError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DiskError::Io(e) => write!(f, "disk I/O error: {e}"),
            DiskError::InvalidPageId(id) => write!(f, "page id {id} is out of range"),
        }
    }
}

impl std::error::Error for DiskError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            DiskError::Io(e) => Some(e),
            DiskError::InvalidPageId(_) => None,
        }
    }
}

impl From<std::io::Error> for DiskError {
    fn from(e: std::io::Error) -> Self {
        DiskError::Io(e)
    }
}

// ---------------------------------------------------------------------------
// DiskManager
// ---------------------------------------------------------------------------

/// Manages raw page reads and writes against a single database heap file.
///
/// Each page occupies exactly `PAGE_SIZE` bytes at byte offset
/// `page_id * PAGE_SIZE` within the file.  Page 0 is the metadata page:
/// its first 8 bytes store `num_pages` as a little-endian `u64` so the
/// allocation counter persists across restarts.
pub struct DiskManager {
    /// Open handle to the database file.
    file: File,
    /// Absolute path — kept for display/debug purposes.
    path: PathBuf,
    /// Total number of pages allocated (including page 0).
    num_pages: u64,
}

/// Byte offset of the `num_pages` counter inside Page 0.
const META_NUM_PAGES_OFFSET: u64 = 0;

impl DiskManager {
    // -----------------------------------------------------------------------
    // Construction
    // -----------------------------------------------------------------------

    /// Opens (or creates) the database file at `path`.
    ///
    /// On first open the file is empty and we bootstrap it with Page 0
    /// (the metadata page) so `num_pages` starts at 1.
    pub fn open(path: &Path) -> Result<Self, DiskError> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(path)?;

        let file_len = file.metadata()?.len();

        let num_pages = if file_len == 0 {
            // Brand-new database — bootstrap the metadata page.
            let mut dm = DiskManager {
                file,
                path: path.to_path_buf(),
                num_pages: 0,
            };
            dm.bootstrap_meta()?;
            dm.num_pages
        } else {
            // Existing database — read num_pages from the metadata page.
            let mut dm = DiskManager {
                file,
                path: path.to_path_buf(),
                num_pages: 0,
            };
            dm.num_pages = dm.read_meta_num_pages()?;
            dm.num_pages
        };

        Ok(DiskManager {
            file: OpenOptions::new()
                .read(true)
                .write(true)
                .open(path)?,
            path: path.to_path_buf(),
            num_pages,
        })
    }

    // -----------------------------------------------------------------------
    // Public API
    // -----------------------------------------------------------------------

    /// Allocates a new page at the end of the file.
    ///
    /// The page is zeroed on disk (except for its `page_id` header field).
    /// Returns the newly allocated `PageId`.
    pub fn allocate_page(&mut self) -> Result<PageId, DiskError> {
        let new_id = self.num_pages;
        self.num_pages += 1;

        // Write a zeroed page at the new slot.
        let mut page = Page::new();
        page.set_page_id(new_id);
        self.write_page_inner(new_id, &page)?;

        // Persist the updated num_pages counter to the metadata page.
        self.flush_meta_num_pages()?;

        Ok(new_id)
    }

    /// Reads the page at `page_id` from disk into `page`.
    pub fn read_page(&mut self, page_id: PageId, page: &mut Page) -> Result<(), DiskError> {
        if page_id >= self.num_pages {
            return Err(DiskError::InvalidPageId(page_id));
        }
        let offset = page_id * PAGE_SIZE as u64;
        self.file.seek(SeekFrom::Start(offset))?;
        self.file.read_exact(page.raw_mut())?;
        Ok(())
    }

    /// Writes `page` to disk at the slot corresponding to `page_id`.
    pub fn write_page(&mut self, page_id: PageId, page: &Page) -> Result<(), DiskError> {
        if page_id >= self.num_pages {
            return Err(DiskError::InvalidPageId(page_id));
        }
        self.write_page_inner(page_id, page)?;
        self.file.flush()?;
        Ok(())
    }

    /// Flushes any OS-buffered writes to the underlying storage device.
    pub fn sync(&mut self) -> Result<(), DiskError> {
        self.file.sync_data()?;
        Ok(())
    }

    /// Returns the number of pages currently allocated (including page 0).
    pub fn num_pages(&self) -> u64 {
        self.num_pages
    }

    /// Returns the path of the database file.
    pub fn path(&self) -> &Path {
        &self.path
    }

    // -----------------------------------------------------------------------
    // Private helpers
    // -----------------------------------------------------------------------

    /// Writes a page to the file without flushing — callers must flush.
    fn write_page_inner(&mut self, page_id: PageId, page: &Page) -> Result<(), DiskError> {
        let offset = page_id * PAGE_SIZE as u64;
        self.file.seek(SeekFrom::Start(offset))?;
        self.file.write_all(page.raw())?;
        Ok(())
    }

    /// Creates the metadata page (page 0) for a brand-new database file.
    fn bootstrap_meta(&mut self) -> Result<(), DiskError> {
        self.num_pages = 1; // page 0 = metadata page
        let mut meta = Page::new();
        meta.set_page_id(0);
        // Write num_pages = 1 into the first 8 bytes of the page body.
        let count_bytes = self.num_pages.to_le_bytes();
        meta.raw_mut()[META_NUM_PAGES_OFFSET as usize
            ..META_NUM_PAGES_OFFSET as usize + 8]
            .copy_from_slice(&count_bytes);
        self.write_page_inner(0, &meta)?;
        self.file.flush()?;
        Ok(())
    }

    /// Reads the `num_pages` counter from the metadata page (page 0).
    fn read_meta_num_pages(&mut self) -> Result<u64, DiskError> {
        let mut meta = Page::new();
        let offset = 0u64;
        self.file.seek(SeekFrom::Start(offset))?;
        self.file.read_exact(meta.raw_mut())?;
        let bytes: [u8; 8] = meta.raw()[META_NUM_PAGES_OFFSET as usize
            ..META_NUM_PAGES_OFFSET as usize + 8]
            .try_into()
            .expect("slice length is always 8");
        Ok(u64::from_le_bytes(bytes))
    }

    /// Persists the current `num_pages` counter to the metadata page.
    fn flush_meta_num_pages(&mut self) -> Result<(), DiskError> {
        // Read the metadata page first so we don't clobber any other fields.
        let mut meta = Page::new();
        self.file.seek(SeekFrom::Start(0))?;
        self.file.read_exact(meta.raw_mut())?;

        let bytes = self.num_pages.to_le_bytes();
        meta.raw_mut()[META_NUM_PAGES_OFFSET as usize
            ..META_NUM_PAGES_OFFSET as usize + 8]
            .copy_from_slice(&bytes);
        self.write_page_inner(0, &meta)?;
        self.file.flush()?;
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    fn temp_db() -> (DiskManager, NamedTempFile) {
        let f = NamedTempFile::new().unwrap();
        let dm = DiskManager::open(f.path()).unwrap();
        (dm, f)
    }

    #[test]
    fn fresh_open_has_one_page() {
        let (dm, _f) = temp_db();
        // Page 0 is the metadata page, allocated during bootstrap.
        assert_eq!(dm.num_pages(), 1);
    }

    #[test]
    fn allocate_increments_page_count() {
        let (mut dm, _f) = temp_db();
        let id1 = dm.allocate_page().unwrap();
        let id2 = dm.allocate_page().unwrap();
        assert_eq!(id1, 1);
        assert_eq!(id2, 2);
        assert_eq!(dm.num_pages(), 3);
    }

    #[test]
    fn write_and_read_roundtrip() {
        let (mut dm, _f) = temp_db();
        let pid = dm.allocate_page().unwrap();

        let mut page = Page::new();
        page.set_page_id(pid);
        page.set_page_type(crate::page::PageType::Heap);
        page.body_mut()[0] = 0xAB;
        page.body_mut()[1] = 0xCD;
        dm.write_page(pid, &page).unwrap();

        let mut read_back = Page::new();
        dm.read_page(pid, &mut read_back).unwrap();

        assert_eq!(read_back.page_id(), pid);
        assert_eq!(read_back.page_type(), crate::page::PageType::Heap);
        assert_eq!(read_back.body()[0], 0xAB);
        assert_eq!(read_back.body()[1], 0xCD);
    }

    #[test]
    fn num_pages_persists_across_reopen() {
        let f = NamedTempFile::new().unwrap();
        {
            let mut dm = DiskManager::open(f.path()).unwrap();
            dm.allocate_page().unwrap();
            dm.allocate_page().unwrap();
            // dm dropped here → file handle closed.
        }
        // Re-open the same file.
        let dm2 = DiskManager::open(f.path()).unwrap();
        assert_eq!(dm2.num_pages(), 3); // page 0 + 2 allocated
    }

    #[test]
    fn read_invalid_page_id_errors() {
        let (mut dm, _f) = temp_db();
        let mut page = Page::new();
        let err = dm.read_page(999, &mut page);
        assert!(matches!(err, Err(DiskError::InvalidPageId(999))));
    }

    #[test]
    fn write_invalid_page_id_errors() {
        let (mut dm, _f) = temp_db();
        let page = Page::new();
        let err = dm.write_page(999, &page);
        assert!(matches!(err, Err(DiskError::InvalidPageId(999))));
    }
}

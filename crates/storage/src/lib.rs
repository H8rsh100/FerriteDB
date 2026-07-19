//! Storage crate — page format, disk manager, and buffer pool.
//!
//! ## Architecture
//! ```text
//!   BufferPoolManager          (in-memory LRU cache of pages)
//!         │
//!         ▼
//!    DiskManager               (raw page reads/writes to a heap file)
//!         │
//!         ▼
//!      Page (4 KiB)            (fixed-size byte buffer with typed header)
//! ```
//!
//! Upper layers (index, catalog, exec) interact only with `BufferPoolManager`
//! — they never call `DiskManager` directly.

pub mod page;
pub mod disk_manager;
pub mod buffer_pool;

pub use page::{Page, PAGE_SIZE, PageId, PageType};
pub use disk_manager::{DiskManager, DiskError};
pub use buffer_pool::{BufferPoolManager, BufError};

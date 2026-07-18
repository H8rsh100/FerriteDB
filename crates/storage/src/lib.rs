//! Storage crate — page format, disk manager, buffer pool.
//!
//! Phase 1 will fill this out. For now the public API surface is declared
//! so that other crates can depend on this crate and compile cleanly.

pub mod page;
pub mod disk_manager;
pub mod buffer_pool;

pub use page::{Page, PAGE_SIZE, PageId};
pub use disk_manager::DiskManager;
pub use buffer_pool::BufferPoolManager;

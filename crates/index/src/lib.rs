//! Index crate — disk-backed B+Tree built on `storage::BufferPoolManager`.
//!
//! ## Architecture
//! ```text
//!   BTree<K>              (Phase 2: insert/search/delete/range_scan)
//!      │
//!      ▼
//!   Node<K>               (in-memory repr, serialised into Pages)
//!      │
//!      ▼
//!   BufferPoolManager     (fetch/pin/unpin/flush pages)
//!      │
//!      ▼
//!   DiskManager / disk
//! ```

pub mod key;
pub mod node;
pub mod btree;

pub use key::BTreeKey;
pub use node::{Node, NodeKind, INVALID_PAGE_ID};
pub use btree::BTree;

//! Index crate — disk-backed B+Tree indexing.
//!
//! Phase 2 will implement the full B+Tree on top of `storage::BufferPoolManager`.
//! For Phase 0 the public API surface is declared so the workspace compiles.

pub mod btree;
pub mod node;

pub use btree::BTree;

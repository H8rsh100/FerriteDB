//! B+Tree node layout — Phase 2 implementation lives here.
//!
//! Internal nodes: `[(key, child_page_id)]` pairs + one extra rightmost child.
//! Leaf nodes: `[(key, value)]` pairs + right-sibling pointer for range scans.
//! Both types are serialized into a `storage::Page` byte buffer.

use storage::PageId;

/// Maximum number of keys in an internal node (order - 1).
pub const INTERNAL_MAX_KEYS: usize = 255;

/// Maximum number of key-value pairs in a leaf node.
pub const LEAF_MAX_PAIRS: usize = 255;

/// Discriminates between the two node types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeKind {
    Internal,
    Leaf,
}

/// An in-memory representation of a B+Tree node deserialized from one page.
///
/// Phase 2 will implement `from_page` / `to_page` serialization.
#[derive(Debug, Clone)]
pub struct Node {
    pub kind: NodeKind,
    pub page_id: PageId,
    /// Keys stored in this node (serialized as bytes in the real impl).
    pub num_keys: usize,
    /// Right-sibling pointer — valid only for leaf nodes.
    pub right_sibling: Option<PageId>,
}

impl Node {
    /// Creates a new empty leaf node.
    pub fn new_leaf(page_id: PageId) -> Self {
        Self {
            kind: NodeKind::Leaf,
            page_id,
            num_keys: 0,
            right_sibling: None,
        }
    }

    /// Creates a new empty internal node.
    pub fn new_internal(page_id: PageId) -> Self {
        Self {
            kind: NodeKind::Internal,
            page_id,
            num_keys: 0,
            right_sibling: None,
        }
    }

    /// Returns true if this node is a leaf.
    pub fn is_leaf(&self) -> bool {
        self.kind == NodeKind::Leaf
    }
}

//! Node — in-memory B+Tree node with page serialisation.
//!
//! ## Page body layout
//! ```text
//! ┌── NODE_HEADER_BYTES (20) ──────────────────────────────────┐
//! │ kind: u8 │ num_keys: u16 │ right_sib: u64 │ parent: u64   │
//! │  [0]     │   [1..3]      │   [3..11]       │  [11..19]     │
//! │ flags: u8 (bit0 = is_root)                                 │
//! │  [19]                                                      │
//! └────────────────────────────────────────────────────────────┘
//! ┌── Entries (variable, up to NODE_ENTRIES_SIZE bytes) ───────┐
//! │  Internal: leftmost_child:u64 │ (key_len:u16 key right_child:u64)* │
//! │  Leaf:     (key_len:u16  key  value:u64)*                          │
//! └────────────────────────────────────────────────────────────────────┘
//! ```

use storage::{Page, PAGE_SIZE, PageId};
use crate::key::BTreeKey;

/// Sentinel for "no page" (Option<PageId> compressed into u64).
pub const INVALID_PAGE_ID: PageId = u64::MAX;

/// Bytes used by the node header within the page body.
pub const NODE_HEADER_BYTES: usize = 20;

/// Bytes available for entry data after page header + node header.
pub const NODE_ENTRIES_SIZE: usize = PAGE_SIZE - 16 /* Page::HEADER_SIZE */ - NODE_HEADER_BYTES;

/// Distinguishes internal nodes from leaf nodes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeKind {
    Internal = 0,
    Leaf = 1,
}

/// In-memory representation of one B+Tree node.
///
/// Invariants (maintained by `BTree`):
/// - Internal: `keys.len() + 1 == children.len()`
/// - Leaf:     `keys.len() == values.len()`
#[derive(Debug, Clone)]
pub struct Node<K> {
    pub page_id: PageId,
    pub kind: NodeKind,
    pub parent: Option<PageId>,
    pub right_sibling: Option<PageId>,
    pub is_root: bool,
    pub keys: Vec<K>,
    /// Leaf record pointers (parallel to `keys`, leaf nodes only).
    pub values: Vec<u64>,
    /// Child page ids (internal nodes only). `children[i]` is the subtree
    /// for keys strictly less than `keys[i]`; `children[keys.len()]` is the
    /// rightmost child.
    pub children: Vec<PageId>,
}

impl<K: BTreeKey> Node<K> {
    // -----------------------------------------------------------------------
    // Constructors
    // -----------------------------------------------------------------------

    /// Creates an empty leaf node (the initial state of a new tree).
    pub fn new_leaf(page_id: PageId, is_root: bool) -> Self {
        Node {
            page_id, kind: NodeKind::Leaf, parent: None,
            right_sibling: None, is_root,
            keys: vec![], values: vec![], children: vec![],
        }
    }

    /// Creates a new internal node.
    pub fn new_internal(page_id: PageId, is_root: bool) -> Self {
        Node {
            page_id, kind: NodeKind::Internal, parent: None,
            right_sibling: None, is_root,
            keys: vec![], values: vec![], children: vec![],
        }
    }

    // -----------------------------------------------------------------------
    // Serialisation: Node → Page
    // -----------------------------------------------------------------------

    pub fn write_to_page(&self, page: &mut Page) {
        page.set_page_id(self.page_id);
        let body = page.body_mut();

        // --- header ---
        body[0] = self.kind as u8;
        let n = self.keys.len() as u16;
        body[1..3].copy_from_slice(&n.to_le_bytes());
        let sib = self.right_sibling.unwrap_or(INVALID_PAGE_ID);
        body[3..11].copy_from_slice(&sib.to_le_bytes());
        let par = self.parent.unwrap_or(INVALID_PAGE_ID);
        body[11..19].copy_from_slice(&par.to_le_bytes());
        body[19] = self.is_root as u8;

        // --- entries ---
        let mut pos = NODE_HEADER_BYTES;

        match self.kind {
            NodeKind::Internal => {
                // leftmost child
                body[pos..pos + 8].copy_from_slice(&self.children[0].to_le_bytes());
                pos += 8;
                // separator keys + right children
                for (i, key) in self.keys.iter().enumerate() {
                    let kb = key.encode();
                    let kl = kb.len() as u16;
                    body[pos..pos + 2].copy_from_slice(&kl.to_le_bytes());
                    pos += 2;
                    body[pos..pos + kb.len()].copy_from_slice(&kb);
                    pos += kb.len();
                    body[pos..pos + 8].copy_from_slice(&self.children[i + 1].to_le_bytes());
                    pos += 8;
                }
            }
            NodeKind::Leaf => {
                for (i, key) in self.keys.iter().enumerate() {
                    let kb = key.encode();
                    let kl = kb.len() as u16;
                    body[pos..pos + 2].copy_from_slice(&kl.to_le_bytes());
                    pos += 2;
                    body[pos..pos + kb.len()].copy_from_slice(&kb);
                    pos += kb.len();
                    body[pos..pos + 8].copy_from_slice(&self.values[i].to_le_bytes());
                    pos += 8;
                }
            }
        }
    }

    // -----------------------------------------------------------------------
    // Deserialisation: Page → Node
    // -----------------------------------------------------------------------

    pub fn from_page(page_id: PageId, page: &Page) -> Option<Self> {
        let body = page.body();
        let kind = match body[0] {
            0 => NodeKind::Internal,
            1 => NodeKind::Leaf,
            _ => return None,
        };
        let num_keys = u16::from_le_bytes(body[1..3].try_into().ok()?) as usize;
        let sib_raw  = u64::from_le_bytes(body[3..11].try_into().ok()?);
        let par_raw  = u64::from_le_bytes(body[11..19].try_into().ok()?);
        let is_root  = body[19] != 0;

        let right_sibling = (sib_raw != INVALID_PAGE_ID).then_some(sib_raw);
        let parent        = (par_raw != INVALID_PAGE_ID).then_some(par_raw);

        let mut pos = NODE_HEADER_BYTES;
        let mut keys: Vec<K>     = Vec::with_capacity(num_keys);
        let mut values: Vec<u64> = Vec::new();
        let mut children: Vec<PageId> = Vec::new();

        match kind {
            NodeKind::Internal => {
                // leftmost child
                let lc = u64::from_le_bytes(body[pos..pos + 8].try_into().ok()?);
                children.push(lc);
                pos += 8;
                for _ in 0..num_keys {
                    let kl = u16::from_le_bytes(body[pos..pos + 2].try_into().ok()?) as usize;
                    pos += 2;
                    let (key, _) = K::decode(&body[pos..pos + kl])?;
                    keys.push(key);
                    pos += kl;
                    let rc = u64::from_le_bytes(body[pos..pos + 8].try_into().ok()?);
                    children.push(rc);
                    pos += 8;
                }
            }
            NodeKind::Leaf => {
                for _ in 0..num_keys {
                    let kl = u16::from_le_bytes(body[pos..pos + 2].try_into().ok()?) as usize;
                    pos += 2;
                    let (key, _) = K::decode(&body[pos..pos + kl])?;
                    keys.push(key);
                    pos += kl;
                    let val = u64::from_le_bytes(body[pos..pos + 8].try_into().ok()?);
                    values.push(val);
                    pos += 8;
                }
            }
        }

        Some(Node { page_id, kind, parent, right_sibling, is_root, keys, values, children })
    }

    // -----------------------------------------------------------------------
    // Helpers
    // -----------------------------------------------------------------------

    /// Number of bytes consumed by the entries section if serialised now.
    pub fn entries_byte_size(&self) -> usize {
        let mut sz = 0usize;
        if self.kind == NodeKind::Internal {
            sz += 8; // leftmost child
            for key in &self.keys {
                sz += 2 + key.encode().len() + 8;
            }
        } else {
            for key in &self.keys {
                sz += 2 + key.encode().len() + 8;
            }
        }
        sz
    }

    /// True when the entries section is at or over capacity.
    pub fn is_full(&self, max_key_bytes: usize) -> bool {
        // Conservative: use max_key_bytes for the split threshold so we
        // never over-fill a page regardless of actual key lengths.
        let entry_size = 2 + max_key_bytes + 8;
        let used = match self.kind {
            NodeKind::Internal => 8 + self.keys.len() * entry_size,
            NodeKind::Leaf     =>     self.keys.len() * entry_size,
        };
        used + entry_size > NODE_ENTRIES_SIZE
    }

    /// Binary-search position for `key` in `self.keys`.
    /// Returns the index i such that `keys[i-1] <= key < keys[i]`.
    pub fn find_child_pos(&self, key: &K) -> usize {
        self.keys.partition_point(|k| k <= key)
    }

    /// Binary-search for an exact key match in a leaf.
    pub fn find_key(&self, key: &K) -> Option<usize> {
        let pos = self.keys.partition_point(|k| k < key);
        if pos < self.keys.len() && &self.keys[pos] == key {
            Some(pos)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_leaf(id: u64, pairs: &[(i64, u64)]) -> Node<i64> {
        let mut n = Node::new_leaf(id, false);
        for &(k, v) in pairs {
            n.keys.push(k);
            n.values.push(v);
        }
        n
    }

    fn make_internal(id: u64, keys: &[i64], children: &[u64]) -> Node<i64> {
        let mut n = Node::new_internal(id, false);
        n.keys = keys.to_vec();
        n.children = children.to_vec();
        n
    }

    #[test]
    fn leaf_serialisation_roundtrip() {
        let original = make_leaf(7, &[(1, 100), (2, 200), (3, 300)]);
        let mut page = Page::new();
        original.write_to_page(&mut page);
        let recovered = Node::<i64>::from_page(7, &page).unwrap();

        assert_eq!(recovered.kind, NodeKind::Leaf);
        assert_eq!(recovered.keys,   vec![1, 2, 3]);
        assert_eq!(recovered.values, vec![100, 200, 300]);
        assert_eq!(recovered.page_id, 7);
    }

    #[test]
    fn internal_serialisation_roundtrip() {
        let original = make_internal(3, &[10, 20], &[5, 6, 7]);
        let mut page = Page::new();
        original.write_to_page(&mut page);
        let recovered = Node::<i64>::from_page(3, &page).unwrap();

        assert_eq!(recovered.kind, NodeKind::Internal);
        assert_eq!(recovered.keys,     vec![10, 20]);
        assert_eq!(recovered.children, vec![5, 6, 7]);
    }

    #[test]
    fn sibling_and_parent_roundtrip() {
        let mut n = make_leaf(1, &[(42, 99)]);
        n.right_sibling = Some(2);
        n.parent        = Some(0);
        n.is_root       = false;

        let mut page = Page::new();
        n.write_to_page(&mut page);
        let r = Node::<i64>::from_page(1, &page).unwrap();

        assert_eq!(r.right_sibling, Some(2));
        assert_eq!(r.parent,        Some(0));
        assert!(!r.is_root);
    }

    #[test]
    fn string_leaf_roundtrip() {
        let mut n = Node::<String>::new_leaf(9, true);
        n.keys   = vec!["alpha".into(), "beta".into(), "gamma".into()];
        n.values = vec![1, 2, 3];

        let mut page = Page::new();
        n.write_to_page(&mut page);
        let r = Node::<String>::from_page(9, &page).unwrap();

        assert_eq!(r.keys,   vec!["alpha", "beta", "gamma"]);
        assert_eq!(r.values, vec![1, 2, 3]);
    }
}

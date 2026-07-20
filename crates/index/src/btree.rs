//! B+Tree implementation over BufferPoolManager.

use std::marker::PhantomData;
use std::sync::Arc;
use storage::{BufError, BufferPoolManager, PageId};
use crate::key::BTreeKey;
use crate::node::{Node, NodeKind};

/// A disk-backed B+Tree index using BufferPoolManager.
pub struct BTree<K: BTreeKey> {
    root_page_id: PageId,
    bpm: Arc<BufferPoolManager>,
    max_key_bytes: usize,
    _marker: PhantomData<K>,
}

impl<K: BTreeKey> BTree<K> {
    /// Creates or opens a B+Tree rooted at `root_page_id`.
    pub fn new(root_page_id: PageId, bpm: Arc<BufferPoolManager>, max_key_bytes: usize) -> Result<Self, BufError> {
        let tree = Self {
            root_page_id,
            bpm,
            max_key_bytes,
            _marker: PhantomData,
        };
        Ok(tree)
    }

    pub fn root_page_id(&self) -> PageId {
        self.root_page_id
    }

    /// Read a node from the buffer pool.
    fn fetch_node(&self, page_id: PageId) -> Result<Node<K>, BufError> {
        let page = self.bpm.fetch_page(page_id)?;
        let node = Node::from_page(page_id, &page).ok_or_else(|| {
            let _ = self.bpm.unpin_page(page_id, false);
            BufError::PageNotFound(page_id)
        })?;
        self.bpm.unpin_page(page_id, false)?;
        Ok(node)
    }

    /// Write a node to the buffer pool.
    fn write_node(&self, node: &Node<K>) -> Result<(), BufError> {
        let page = self.bpm.fetch_page(node.page_id)?;
        let mut updated_page = page;
        node.write_to_page(&mut updated_page);
        self.bpm.write_page_to_pool(node.page_id, &updated_page)?;
        self.bpm.unpin_page(node.page_id, true)?;
        Ok(())
    }

    /// Searches for `key` and returns the associated record value.
    pub fn search(&self, key: &K) -> Result<Option<u64>, BufError> {
        let mut curr_id = self.root_page_id;
        loop {
            let node: Node<K> = self.fetch_node(curr_id)?;
            match node.kind {
                NodeKind::Leaf => {
                    return Ok(node.find_key(key).map(|idx| node.values[idx]));
                }
                NodeKind::Internal => {
                    let idx = node.find_child_pos(key);
                    curr_id = node.children[idx];
                }
            }
        }
    }

    /// Inserts `(key, value)` into the B+Tree.
    pub fn insert(&mut self, key: K, value: u64) -> Result<(), BufError> {
        let mut root: Node<K> = self.fetch_node(self.root_page_id)?;

        // If root is full, split root first.
        if root.is_full(self.max_key_bytes) {
            let (new_root_id, _) = self.bpm.new_page()?;
            let mut new_root = Node::new_internal(new_root_id, true);
            new_root.children.push(self.root_page_id);

            root.is_root = false;
            root.parent = Some(new_root_id);
            self.write_node(&root)?;

            let old_root_id = self.root_page_id;
            self.root_page_id = new_root_id;

            self.split_child(&mut new_root, 0, old_root_id)?;
            self.write_node(&new_root)?;
        }

        self.insert_non_full(self.root_page_id, key, value)
    }

    fn insert_non_full(&self, page_id: PageId, key: K, value: u64) -> Result<(), BufError> {
        let mut node: Node<K> = self.fetch_node(page_id)?;

        if node.kind == NodeKind::Leaf {
            let idx = node.keys.partition_point(|k| k < &key);
            if idx < node.keys.len() && node.keys[idx] == key {
                // Key exists: update value
                node.values[idx] = value;
            } else {
                node.keys.insert(idx, key);
                node.values.insert(idx, value);
            }
            self.write_node(&node)?;
            Ok(())
        } else {
            let mut child_idx = node.find_child_pos(&key);
            let mut child_id = node.children[child_idx];
            let child: Node<K> = self.fetch_node(child_id)?;

            if child.is_full(self.max_key_bytes) {
                self.split_child(&mut node, child_idx, child_id)?;
                self.write_node(&node)?;

                if key > node.keys[child_idx] {
                    child_idx += 1;
                }
                child_id = node.children[child_idx];
            }

            self.insert_non_full(child_id, key, value)
        }
    }

    /// Splits a full child node `child_id` at index `child_idx` of parent node `parent`.
    fn split_child(&self, parent: &mut Node<K>, child_idx: usize, child_id: PageId) -> Result<(), BufError> {
        let mut child: Node<K> = self.fetch_node(child_id)?;
        let (new_child_id, _) = self.bpm.new_page()?;

        let mid = child.keys.len() / 2;

        match child.kind {
            NodeKind::Leaf => {
                let mut new_leaf = Node::new_leaf(new_child_id, false);
                new_leaf.parent = child.parent;
                new_leaf.right_sibling = child.right_sibling;
                child.right_sibling = Some(new_child_id);

                new_leaf.keys = child.keys.split_off(mid);
                new_leaf.values = child.values.split_off(mid);

                let split_key = new_leaf.keys[0].clone();

                parent.keys.insert(child_idx, split_key);
                parent.children.insert(child_idx + 1, new_child_id);

                self.write_node(&child)?;
                self.write_node(&new_leaf)?;
            }
            NodeKind::Internal => {
                let mut new_internal = Node::new_internal(new_child_id, false);
                new_internal.parent = child.parent;

                let split_key = child.keys[mid].clone();

                new_internal.keys = child.keys.split_off(mid + 1);
                child.keys.pop(); // remove mid key

                new_internal.children = child.children.split_off(mid + 1);

                // Update parent pointer for moved children
                for &c_id in &new_internal.children {
                    let mut c_node: Node<K> = self.fetch_node(c_id)?;
                    c_node.parent = Some(new_child_id);
                    self.write_node(&c_node)?;
                }

                parent.keys.insert(child_idx, split_key);
                parent.children.insert(child_idx + 1, new_child_id);

                self.write_node(&child)?;
                self.write_node(&new_internal)?;
            }
        }

        Ok(())
    }

    /// Range scan returning key-value pairs in range `[start_key, end_key]`.
    pub fn range_scan(&self, start_key: &K, end_key: &K) -> Result<Vec<(K, u64)>, BufError> {
        let mut results = Vec::new();
        let mut curr_id = self.root_page_id;

        // Traverse down to leftmost leaf containing start_key
        loop {
            let node: Node<K> = self.fetch_node(curr_id)?;
            match node.kind {
                NodeKind::Leaf => {
                    break;
                }
                NodeKind::Internal => {
                    let idx = node.find_child_pos(start_key);
                    curr_id = node.children[idx];
                }
            }
        }

        // Iterate horizontally across leaves
        let mut leaf_id = Some(curr_id);
        while let Some(lid) = leaf_id {
            let leaf: Node<K> = self.fetch_node(lid)?;
            for (i, k) in leaf.keys.iter().enumerate() {
                if k >= start_key && k <= end_key {
                    results.push((k.clone(), leaf.values[i]));
                }
            }
            if let Some(last_key) = leaf.keys.last() {
                if last_key > end_key {
                    break;
                }
            }
            leaf_id = leaf.right_sibling;
        }

        Ok(results)
    }

    /// Deletes a key from the B+Tree. Returns `true` if key was deleted.
    pub fn delete(&mut self, key: &K) -> Result<bool, BufError> {
        self.delete_from_node(self.root_page_id, key)
    }

    fn delete_from_node(&self, page_id: PageId, key: &K) -> Result<bool, BufError> {
        let mut node: Node<K> = self.fetch_node(page_id)?;

        if node.kind == NodeKind::Leaf {
            if let Some(idx) = node.find_key(key) {
                node.keys.remove(idx);
                node.values.remove(idx);
                self.write_node(&node)?;
                Ok(true)
            } else {
                Ok(false)
            }
        } else {
            let idx = node.find_child_pos(key);
            let child_id = node.children[idx];
            self.delete_from_node(child_id, key)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use storage::DiskManager;
    use tempfile::NamedTempFile;

    fn setup_tree() -> (BTree<i64>, NamedTempFile) {
        let file = NamedTempFile::new().unwrap();
        let disk = DiskManager::open(file.path()).unwrap();
        let bpm = Arc::new(BufferPoolManager::new(10, disk));
        let (root_id, mut page) = bpm.new_page().unwrap();
        let root_node: Node<i64> = Node::new_leaf(root_id, true);
        root_node.write_to_page(&mut page);
        bpm.write_page_to_pool(root_id, &page).unwrap();
        bpm.unpin_page(root_id, true).unwrap();

        let tree = BTree::new(root_id, bpm, 8).unwrap();
        (tree, file)
    }

    #[test]
    fn insert_and_search_single() {
        let (mut tree, _f) = setup_tree();
        tree.insert(42, 1000).unwrap();
        assert_eq!(tree.search(&42).unwrap(), Some(1000));
        assert_eq!(tree.search(&43).unwrap(), None);
    }

    #[test]
    fn insert_multiple_and_split() {
        let (mut tree, _f) = setup_tree();
        for i in 1..=50 {
            tree.insert(i, i as u64 * 10).unwrap();
        }
        for i in 1..=50 {
            assert_eq!(tree.search(&i).unwrap(), Some(i as u64 * 10));
        }
    }

    #[test]
    fn range_scan_test() {
        let (mut tree, _f) = setup_tree();
        for &k in &[10, 20, 30, 40, 50, 60] {
            tree.insert(k, k as u64 * 2).unwrap();
        }
        let res = tree.range_scan(&20, &45).unwrap();
        assert_eq!(res, vec![(20, 40), (30, 60), (40, 80)]);
    }

    #[test]
    fn delete_test() {
        let (mut tree, _f) = setup_tree();
        tree.insert(10, 100).unwrap();
        tree.insert(20, 200).unwrap();
        assert!(tree.delete(&10).unwrap());
        assert_eq!(tree.search(&10).unwrap(), None);
        assert_eq!(tree.search(&20).unwrap(), Some(200));
    }
}

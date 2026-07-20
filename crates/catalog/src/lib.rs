//! Catalog crate — schema definitions and system catalog.
//!
//! Handles table schema registrations and persistence to disk via BufferPoolManager.

pub mod schema;
pub mod table;

pub use schema::{Column, DataType, Schema};
pub use table::Table;

use std::collections::HashMap;
use storage::{BufferPoolManager, PageId, PageType};

/// In-memory registry of all user tables, backed by a catalog page on disk.
pub struct Catalog {
    tables: HashMap<String, Table>,
    /// Page id of the persisted catalog entry (reserved at open time).
    catalog_page_id: PageId,
}

impl Catalog {
    /// Creates a new empty catalog backed by `catalog_page_id`.
    pub fn new(catalog_page_id: PageId) -> Self {
        Self {
            tables: HashMap::new(),
            catalog_page_id,
        }
    }

    pub fn catalog_page_id(&self) -> PageId {
        self.catalog_page_id
    }

    /// Registers a new table. Returns an error if the name already exists.
    pub fn create_table(&mut self, name: String, schema: Schema, root_page_id: PageId)
        -> Result<(), String>
    {
        if self.tables.contains_key(&name) {
            return Err(format!("table '{name}' already exists"));
        }
        self.tables.insert(name.clone(), Table::new(name, schema, root_page_id));
        Ok(())
    }

    /// Looks up a table by name.
    pub fn get_table(&self, name: &str) -> Option<&Table> {
        self.tables.get(name)
    }

    /// Drops a table by name. Returns true if it existed.
    pub fn drop_table(&mut self, name: &str) -> bool {
        self.tables.remove(name).is_some()
    }

    /// Returns the names of all registered tables.
    pub fn list_tables(&self) -> Vec<&str> {
        self.tables.keys().map(|s| s.as_str()).collect()
    }

    /// Encodes all catalog tables into bytes.
    pub fn encode(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.extend_from_slice(&(self.tables.len() as u16).to_le_bytes());
        for table in self.tables.values() {
            buf.extend_from_slice(&table.encode());
        }
        buf
    }

    /// Decodes catalog from bytes.
    pub fn decode(catalog_page_id: PageId, bytes: &[u8]) -> Option<Self> {
        if bytes.len() < 2 {
            return None;
        }
        let num_tables = u16::from_le_bytes(bytes[0..2].try_into().ok()?) as usize;
        let mut pos = 2;
        let mut tables = HashMap::with_capacity(num_tables);

        for _ in 0..num_tables {
            let (table, read) = Table::decode(&bytes[pos..])?;
            tables.insert(table.name.clone(), table);
            pos += read;
        }

        Some(Catalog { tables, catalog_page_id })
    }

    /// Persists the catalog into the reserved `catalog_page_id` page in BufferPoolManager.
    pub fn save(&self, bpm: &BufferPoolManager) -> Result<(), String> {
        let bytes = self.encode();
        let page_res = bpm.fetch_page(self.catalog_page_id);
        let mut page = match page_res {
            Ok(p) => p,
            Err(_) => return Err("Failed to fetch catalog page".to_string()),
        };

        page.set_page_id(self.catalog_page_id);
        page.set_page_type(PageType::Catalog);
        let body = page.body_mut();
        if bytes.len() > body.len() {
            let _ = bpm.unpin_page(self.catalog_page_id, false);
            return Err("Catalog data exceeds page body capacity".to_string());
        }
        body[..bytes.len()].copy_from_slice(&bytes);

        bpm.write_page_to_pool(self.catalog_page_id, &page)
            .map_err(|e| e.to_string())?;
        bpm.unpin_page(self.catalog_page_id, true)
            .map_err(|e| e.to_string())?;
        bpm.flush_page(self.catalog_page_id)
            .map_err(|e| e.to_string())?;

        Ok(())
    }

    /// Loads the catalog from `catalog_page_id` page via BufferPoolManager.
    pub fn load(catalog_page_id: PageId, bpm: &BufferPoolManager) -> Result<Self, String> {
        let page = bpm.fetch_page(catalog_page_id)
            .map_err(|e| e.to_string())?;
        let body = page.body();
        let catalog = Self::decode(catalog_page_id, body)
            .ok_or_else(|| "Failed to decode catalog page".to_string())?;
        bpm.unpin_page(catalog_page_id, false)
            .map_err(|e| e.to_string())?;
        Ok(catalog)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use storage::DiskManager;
    use tempfile::NamedTempFile;

    fn sample_schema() -> Schema {
        Schema::new(vec![
            Column::new("id", DataType::BigInt, false),
            Column::new("name", DataType::Varchar(255), true),
        ])
    }

    #[test]
    fn create_and_get_table() {
        let mut cat = Catalog::new(0);
        cat.create_table("users".into(), sample_schema(), 1).unwrap();
        assert!(cat.get_table("users").is_some());
    }

    #[test]
    fn duplicate_table_errors() {
        let mut cat = Catalog::new(0);
        cat.create_table("users".into(), sample_schema(), 1).unwrap();
        assert!(cat.create_table("users".into(), sample_schema(), 2).is_err());
    }

    #[test]
    fn drop_table() {
        let mut cat = Catalog::new(0);
        cat.create_table("users".into(), sample_schema(), 1).unwrap();
        assert!(cat.drop_table("users"));
        assert!(cat.get_table("users").is_none());
        assert!(!cat.drop_table("users"));
    }

    #[test]
    fn list_tables() {
        let mut cat = Catalog::new(0);
        cat.create_table("a".into(), sample_schema(), 1).unwrap();
        cat.create_table("b".into(), sample_schema(), 2).unwrap();
        let mut tables = cat.list_tables();
        tables.sort();
        assert_eq!(tables, vec!["a", "b"]);
    }

    #[test]
    fn persistence_save_and_load() {
        let file = NamedTempFile::new().unwrap();
        let catalog_pid;
        {
            let disk = DiskManager::open(file.path()).unwrap();
            let bpm = BufferPoolManager::new(5, disk);
            let (cid, _) = bpm.new_page().unwrap();
            catalog_pid = cid;
            bpm.unpin_page(cid, true).unwrap();

            let mut cat = Catalog::new(catalog_pid);
            cat.create_table("users".into(), sample_schema(), 10).unwrap();
            cat.create_table("orders".into(), sample_schema(), 20).unwrap();
            cat.save(&bpm).unwrap();
        }

        {
            let disk = DiskManager::open(file.path()).unwrap();
            let bpm = BufferPoolManager::new(5, disk);
            let loaded_cat = Catalog::load(catalog_pid, &bpm).unwrap();

            assert!(loaded_cat.get_table("users").is_some());
            assert!(loaded_cat.get_table("orders").is_some());
            assert_eq!(loaded_cat.get_table("users").unwrap().root_page_id, 10);
            assert_eq!(loaded_cat.get_table("orders").unwrap().root_page_id, 20);
        }
    }
}

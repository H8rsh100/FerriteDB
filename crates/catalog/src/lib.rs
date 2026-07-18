//! Catalog crate — schema definitions and system catalog.
//!
//! Phase 3 will implement persistence (serialize to a reserved page_id).
//! For Phase 0 the types and method signatures are declared.

pub mod schema;
pub mod table;

pub use schema::{Column, DataType, Schema};
pub use table::Table;

use std::collections::HashMap;
use storage::PageId;

/// In-memory registry of all user tables, backed by a catalog page on disk.
///
/// On startup: deserialize from a reserved page.
/// On shutdown/checkpoint: serialize back to that page.
pub struct Catalog {
    tables: HashMap<String, Table>,
    /// Page id of the persisted catalog entry (reserved at open time).
    _catalog_page_id: PageId,
}

impl Catalog {
    /// Creates a new empty catalog backed by `catalog_page_id`.
    pub fn new(catalog_page_id: PageId) -> Self {
        Self {
            tables: HashMap::new(),
            _catalog_page_id: catalog_page_id,
        }
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
}

#[cfg(test)]
mod tests {
    use super::*;

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
        assert!(!cat.drop_table("users")); // idempotent false
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
}

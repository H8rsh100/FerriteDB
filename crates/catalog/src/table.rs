//! Table — metadata for a single user table.

use storage::PageId;
use crate::schema::Schema;

/// Metadata for one user table stored in the catalog.
#[derive(Debug, Clone)]
pub struct Table {
    /// Table name (must be unique across the catalog).
    pub name: String,
    /// Column definitions.
    pub schema: Schema,
    /// Page id of the first data page (heap) or the B+Tree root page.
    pub root_page_id: PageId,
    /// Approximate row count — maintained by the executor for the cost model.
    pub approx_row_count: u64,
}

impl Table {
    pub fn new(name: String, schema: Schema, root_page_id: PageId) -> Self {
        Self {
            name,
            schema,
            root_page_id,
            approx_row_count: 0,
        }
    }
}

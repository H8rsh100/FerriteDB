//! Table — metadata for a single user table.

use storage::PageId;
use crate::schema::Schema;

/// Metadata for one user table stored in the catalog.
#[derive(Debug, Clone, PartialEq, Eq)]
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

    pub fn encode(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        let name_bytes = self.name.as_bytes();
        buf.extend_from_slice(&(name_bytes.len() as u16).to_le_bytes());
        buf.extend_from_slice(name_bytes);
        buf.extend_from_slice(&self.schema.encode());
        buf.extend_from_slice(&self.root_page_id.to_le_bytes());
        buf.extend_from_slice(&self.approx_row_count.to_le_bytes());
        buf
    }

    pub fn decode(bytes: &[u8]) -> Option<(Self, usize)> {
        if bytes.len() < 2 {
            return None;
        }
        let name_len = u16::from_le_bytes(bytes[0..2].try_into().ok()?) as usize;
        let mut pos = 2;
        if bytes.len() < pos + name_len {
            return None;
        }
        let name = std::str::from_utf8(&bytes[pos..pos + name_len]).ok()?.to_string();
        pos += name_len;

        let (schema, schema_read) = Schema::decode(&bytes[pos..])?;
        pos += schema_read;

        if bytes.len() < pos + 16 {
            return None;
        }
        let root_page_id = u64::from_le_bytes(bytes[pos..pos + 8].try_into().ok()?);
        pos += 8;
        let approx_row_count = u64::from_le_bytes(bytes[pos..pos + 8].try_into().ok()?);
        pos += 8;

        Some((Table { name, schema, root_page_id, approx_row_count }, pos))
    }
}

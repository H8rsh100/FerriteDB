//! Page — the fundamental unit of storage in FerriteDB.
//!
//! A page is a fixed-size (4 KiB) byte buffer. Every on-disk structure
//! (heap files, B+Tree nodes, WAL segments, the system catalog) is
//! decomposed into pages so the buffer pool can manage them uniformly.

/// Unique identifier for a page within a heap file.
pub type PageId = u64;

/// Size of a single page in bytes (4 KiB).
pub const PAGE_SIZE: usize = 4096;

/// A fixed-size in-memory representation of one disk page.
///
/// # Layout
/// ```text
/// ┌──────────────────────── PAGE_SIZE (4096 B) ─────────────────────────┐
/// │  Header (16 B)                     │  Body (4080 B)                 │
/// │  [page_id: 8B][page_type: 1B]      │  raw bytes managed by upper    │
/// │  [flags: 1B][free_ptr: 2B][_: 4B]  │  layers (heap, B+Tree, etc.)   │
/// └──────────────────────────────────────────────────────────────────────┘
/// ```
#[derive(Clone)]
pub struct Page {
    /// The raw byte buffer; exactly PAGE_SIZE bytes.
    pub(crate) data: Box<[u8; PAGE_SIZE]>,
}

/// Distinguishes how a page's body is interpreted.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PageType {
    /// Unallocated / zeroed-out page.
    Free = 0,
    /// Heap data page (tuples).
    Heap = 1,
    /// B+Tree internal node.
    BTreeInternal = 2,
    /// B+Tree leaf node.
    BTreeLeaf = 3,
    /// Write-ahead log segment.
    Wal = 4,
    /// System catalog page.
    Catalog = 5,
}

impl Page {
    /// Byte offset of the page_id field inside the raw buffer.
    pub const OFFSET_PAGE_ID: usize = 0; // 8 bytes
    /// Byte offset of the page_type byte.
    pub const OFFSET_PAGE_TYPE: usize = 8; // 1 byte
    /// Byte offset of the flags byte (reserved for future use).
    pub const OFFSET_FLAGS: usize = 9; // 1 byte
    /// Byte offset of the free-space pointer (u16, little-endian).
    pub const OFFSET_FREE_PTR: usize = 10; // 2 bytes
    /// Total header size in bytes.
    pub const HEADER_SIZE: usize = 16;

    /// Creates a zeroed page.
    pub fn new() -> Self {
        Self {
            data: Box::new([0u8; PAGE_SIZE]),
        }
    }

    /// Returns the page id stored in the header.
    pub fn page_id(&self) -> PageId {
        let bytes = self.data[Self::OFFSET_PAGE_ID..Self::OFFSET_PAGE_ID + 8]
            .try_into()
            .unwrap();
        u64::from_le_bytes(bytes)
    }

    /// Writes the page id into the header.
    pub fn set_page_id(&mut self, id: PageId) {
        self.data[Self::OFFSET_PAGE_ID..Self::OFFSET_PAGE_ID + 8]
            .copy_from_slice(&id.to_le_bytes());
    }

    /// Returns the page type stored in the header.
    pub fn page_type(&self) -> PageType {
        match self.data[Self::OFFSET_PAGE_TYPE] {
            1 => PageType::Heap,
            2 => PageType::BTreeInternal,
            3 => PageType::BTreeLeaf,
            4 => PageType::Wal,
            5 => PageType::Catalog,
            _ => PageType::Free,
        }
    }

    /// Sets the page type in the header.
    pub fn set_page_type(&mut self, pt: PageType) {
        self.data[Self::OFFSET_PAGE_TYPE] = pt as u8;
    }

    /// Returns the free-space pointer (next writable byte offset in the body).
    pub fn free_ptr(&self) -> u16 {
        let bytes = self.data[Self::OFFSET_FREE_PTR..Self::OFFSET_FREE_PTR + 2]
            .try_into()
            .unwrap();
        u16::from_le_bytes(bytes)
    }

    /// Sets the free-space pointer.
    pub fn set_free_ptr(&mut self, ptr: u16) {
        self.data[Self::OFFSET_FREE_PTR..Self::OFFSET_FREE_PTR + 2]
            .copy_from_slice(&ptr.to_le_bytes());
    }

    /// Returns a read-only slice of the page body (everything after the header).
    pub fn body(&self) -> &[u8] {
        &self.data[Self::HEADER_SIZE..]
    }

    /// Returns a mutable slice of the page body.
    pub fn body_mut(&mut self) -> &mut [u8] {
        &mut self.data[Self::HEADER_SIZE..]
    }

    /// Returns the full raw byte slice (header + body).
    pub fn raw(&self) -> &[u8] {
        &self.data[..]
    }

    /// Returns the full raw mutable byte slice.
    pub fn raw_mut(&mut self) -> &mut [u8] {
        &mut self.data[..]
    }
}

impl Default for Page {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for Page {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Page")
            .field("page_id", &self.page_id())
            .field("page_type", &self.page_type())
            .field("free_ptr", &self.free_ptr())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn page_header_roundtrip() {
        let mut p = Page::new();
        p.set_page_id(42);
        p.set_page_type(PageType::Heap);
        p.set_free_ptr(Page::HEADER_SIZE as u16);

        assert_eq!(p.page_id(), 42);
        assert_eq!(p.page_type(), PageType::Heap);
        assert_eq!(p.free_ptr(), Page::HEADER_SIZE as u16);
    }

    #[test]
    fn page_body_write_read() {
        let mut p = Page::new();
        let body = p.body_mut();
        body[0] = 0xDE;
        body[1] = 0xAD;
        assert_eq!(p.body()[0], 0xDE);
        assert_eq!(p.body()[1], 0xAD);
    }

    #[test]
    fn page_size_correct() {
        assert_eq!(std::mem::size_of::<[u8; PAGE_SIZE]>(), 4096);
    }
}

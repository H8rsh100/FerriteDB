# Design Notes

Running log of tradeoffs and design decisions made during each phase.
These notes are meant to be read in interviews — "walk me through a design decision you made."

---

## Phase 0 — Workspace Setup

**Crate-per-phase design**: Each phase lives in its own crate. This enforces
clean dependency layering at the compiler level (you *cannot* accidentally
import `exec` from `storage`) and lets you point an interviewer at a single
crate's code without noise from other layers.

**Resolver 2**: The workspace uses Cargo's `resolver = "2"` which applies
feature unification per-crate rather than globally. Important once crates
start gating optional features (e.g., async I/O for future work).

---

## Phase 1 — Storage Engine

### DiskManager

**Single heap file, offset = page_id × PAGE_SIZE.**
The simplest layout that allows O(1) random access to any page.  An
alternative would be an extent-based layout (groups of pages) as used by
PostgreSQL, which improves sequential scan performance but adds complexity we
don't need at this stage.

**Metadata in Page 0.**
The allocated page count (`num_pages`) lives in the first 8 bytes of Page 0.
On every `allocate_page` call we re-read, update, and re-write Page 0.  This
is safe even after a crash: either the zeroed new page and the updated counter
both hit disk (consistent) or neither does (we just re-use a slot on recovery).
A more robust approach would use a separate metadata sector outside page space,
but Page 0 is good enough for our single-writer model.

**`sync_data` vs `sync_all`.**
We call `file.flush()` (OS page-cache flush) after writes.  `sync_data`
(fdatasync) would additionally flush metadata (file size).  We don't call it
on every write because it is very expensive on spinning disks.  WAL (Phase 7)
will handle durability guarantees properly.

### BufferPoolManager

**Single `Mutex<Inner>` — chosen over per-frame `RwLock`.**
The free-list, page-table, and LRU clock are shared bookkeeping structures
that must be updated atomically whenever *any* frame changes state.  Protecting
them with a separate lock plus per-frame locks creates a 2-lock ordering
constraint and genuine deadlock risk.  With a single coarse lock the pool is
trivially correct and the overhead is negligible for our workload sizes.  A
per-frame lock upgrade should be validated with a benchmark showing the mutex
as the bottleneck before attempting it.

**LRU via O(n) scan rather than a doubly-linked list.**
Safe Rust makes an intrusive doubly-linked list painful (self-referential
structs require `unsafe` or `Pin`).  An O(n) scan over frames is fast enough
for any pool size a single machine would realistically hold in RAM
(even 500,000 frames scans in microseconds on modern hardware).  If we ever
need sub-microsecond eviction at millions of frames, we can switch to a
`LinkedList` with `Unsafe` cursors or an index-based LRU (HashMap +
VecDeque), both of which are well-understood upgrades.

**`write_page_to_pool` pattern.**
`fetch_page` returns a *clone* of the page, not a mutable reference into the
frame.  This avoids holding the lock across arbitrary caller code.  The caller
modifies its clone, then calls `write_page_to_pool` to push the update back.
This is safe and clean, at the cost of one extra allocation per write.  For
Phase 5 (exec operators) this overhead is fine; a zero-copy API can be added
later.

---

## Phase 2 — B+Tree Indexing

### Node Layout & Serialization
- **Disk-backed pages**: Nodes are serialized directly into 4KiB storage pages managed by `BufferPoolManager`.
- **NodeHeader (20B)**: `kind: u8`, `num_keys: u16`, `right_sibling: u64`, `parent: u64`, `flags: u8`.
- **Sibling Pointers**: Leaf nodes maintain a `right_sibling` page ID enabling efficient $O(1)$ horizontal range scanning across leaf pages without re-traversing internal parent nodes.
- **Key Generics (`BTreeKey`)**: Supports generic keys (`i64`, `String`) via explicit binary encoding/decoding traits.

---

## Phase 4 — SQL Parser & AST

### Lexer
- **Zero-allocation scanning**: `Lexer` scans raw characters into typed `Token` variants with precise line and column `Span` tracking.
- **SQL Keywords & Data Types**: Case-insensitive matching for DDL/DML keywords (`SELECT`, `INSERT`, `CREATE TABLE`, `UPDATE`, `DELETE`, `JOIN`, `WHERE`, `ORDER BY`, `LIMIT`, `OFFSET`) and SQL column types (`INT`, `BIGINT`, `VARCHAR(n)`, `BOOLEAN`, `FLOAT`).

### Recursive-Descent Parser
- **Precedence Climbing**: Expressions are parsed bottom-up according to standard operator precedence (`OR` < `AND` < Comparisons < Additive < Multiplicative < Primary/Column references).
- **AST Statements**: Structured into type-safe AST nodes (`CreateTable`, `Insert`, `Select`, `Update`, `Delete`, `BeginTransaction`, `Commit`, `Abort`).
- **Catalog Type Mapping**: `SqlType` cleanly maps to `catalog::DataType` for schema registration.




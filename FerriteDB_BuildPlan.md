# FerriteDB — Build Plan & Agent Prompts

A relational database engine built from scratch in Rust. Storage engine, B+Tree indexing, SQL parsing, query execution/optimization, transactions, and concurrency control.

---

## 1. Repo Structure

```
ferritedb/
├── Cargo.toml                 # workspace root
├── README.md
├── crates/
│   ├── storage/                # Phase 1: pages, disk manager, buffer pool
│   │   ├── src/
│   │   │   ├── page.rs
│   │   │   ├── disk_manager.rs
│   │   │   ├── buffer_pool.rs
│   │   │   └── lib.rs
│   │   └── Cargo.toml
│   ├── index/                  # Phase 2: B+Tree
│   │   ├── src/
│   │   │   ├── btree.rs
│   │   │   ├── node.rs
│   │   │   └── lib.rs
│   │   └── Cargo.toml
│   ├── catalog/                # Phase 3: schema/catalog
│   │   ├── src/
│   │   │   ├── schema.rs
│   │   │   ├── table.rs
│   │   │   └── lib.rs
│   │   └── Cargo.toml
│   ├── sql/                    # Phase 4: lexer, parser, AST
│   │   ├── src/
│   │   │   ├── lexer.rs
│   │   │   ├── parser.rs
│   │   │   ├── ast.rs
│   │   │   └── lib.rs
│   │   └── Cargo.toml
│   ├── exec/                   # Phase 5: query execution (Volcano model)
│   │   ├── src/
│   │   │   ├── operators/
│   │   │   │   ├── seq_scan.rs
│   │   │   │   ├── index_scan.rs
│   │   │   │   ├── filter.rs
│   │   │   │   ├── project.rs
│   │   │   │   └── join.rs
│   │   │   └── lib.rs
│   │   └── Cargo.toml
│   ├── optimizer/               # Phase 6: rule-based + cost-based optimizer
│   │   ├── src/
│   │   │   ├── rules.rs
│   │   │   ├── planner.rs
│   │   │   └── lib.rs
│   │   └── Cargo.toml
│   ├── txn/                     # Phase 7: WAL, MVCC/2PL, lock manager
│   │   ├── src/
│   │   │   ├── wal.rs
│   │   │   ├── mvcc.rs
│   │   │   ├── lock_manager.rs
│   │   │   └── lib.rs
│   │   └── Cargo.toml
│   └── server/                  # Phase 8: CLI + wire protocol/REPL
│       ├── src/main.rs
│       └── Cargo.toml
├── tests/
│   └── integration/             # cross-crate SQL test suite (.sql + expected output)
└── docs/
    └── design_notes.md          # you write these as you go — gold for interviews
```

Cargo workspaces let each phase live in its own crate, so it compiles independently and you can point at a single crate's code in an interview ("here's my B+Tree implementation").

---

## 2. Phase Roadmap

| Phase | Module | Core deliverable | Est. time |
|---|---|---|---|
| 0 | Setup | Workspace scaffold, CI, basic page format | 2-3 days |
| 1 | Storage | Disk manager + buffer pool (LRU eviction) | 2-3 weeks |
| 2 | Indexing | B+Tree with insert/search/delete/range scan | 3-4 weeks |
| 3 | Catalog | Table/schema definitions, system catalog | 1 week |
| 4 | SQL Frontend | Lexer + recursive-descent parser → AST | 2 weeks |
| 5 | Execution | Volcano-model operators (scan/filter/project/join) | 2-3 weeks |
| 6 | Optimizer | Predicate pushdown, index selection, basic cost model | 1-2 weeks |
| 7 | Transactions | WAL, crash recovery, MVCC or 2PL, lock manager | 3-4 weeks |
| 8 | Polish | REPL/CLI, integration test suite, README + demo | 1 week |

Total: ~4-5 months at a steady pace. Ship after Phase 5 with an in-memory demo (SELECT/INSERT working end to end) — that's your interview-ready MVP even before transactions exist.

---

## 3. Agent Prompts (paste one per phase into Claude Code)

Work through these **in order**, one phase per session/branch. Each prompt assumes the previous crates already exist and compile — paste the phase-0 prompt first, get it merged, then move to phase 1, etc.

### Phase 0 — Workspace Setup

```
Set up a new Rust workspace called ferritedb for a relational database engine
built from scratch. Create the following:

- Root Cargo.toml as a workspace pointing to crates: storage, index, catalog,
  sql, exec, optimizer, txn, server
- Each crate as an empty lib crate (except server, which is a bin crate) with
  a Cargo.toml and a minimal lib.rs/main.rs that compiles
- A .gitignore for Rust
- A README.md with a one-paragraph project description and the phase roadmap
  (I'll give you the roadmap table)
- Basic GitHub Actions CI config that runs `cargo build --workspace` and
  `cargo test --workspace` on push

Do not implement any actual database logic yet — this is scaffolding only.
Confirm the workspace builds cleanly with `cargo build --workspace`.
```

### Phase 1 — Storage Engine

```
Implement the storage crate for FerriteDB, a Rust relational database engine.

Requirements:
1. Page format: fixed-size 4KB pages. Define a Page struct with a header
   (page_id, page_type, free space pointer) and a raw byte buffer.
2. DiskManager: reads/writes pages to/from a single heap file on disk by
   page_id (page_id * PAGE_SIZE offset). Support allocating new pages and
   reading/writing existing ones. Use std::fs with proper error handling
   (no unwrap in library code — return Result types).
3. BufferPoolManager: fixed-size in-memory pool of frames. Implement:
   - fetch_page(page_id) -> pins a page in the buffer pool, reading from
     disk if not present
   - unpin_page(page_id, is_dirty)
   - flush_page(page_id) and flush_all()
   - new_page() -> allocates a new page via the disk manager and pins it
   - LRU (or clock) eviction policy for choosing a victim frame when the
     pool is full and a new page needs to be loaded. Never evict a pinned page.
4. Thread-safety: BufferPoolManager should be safe to share across threads
   (use appropriate synchronization — e.g. Mutex/RwLock per frame or on the
   whole pool, your call, but explain the tradeoff in a doc comment).

Write unit tests covering: page read/write round-trips, buffer pool eviction
under pressure (pool smaller than working set), and pinned-page eviction
protection. Explain in comments why you chose LRU vs clock if relevant.
```

### Phase 2 — B+Tree Indexing

```
Implement the index crate for FerriteDB: a disk-backed B+Tree built on top
of the storage crate's BufferPoolManager (do not reimplement paging — use
storage::BufferPoolManager to fetch/pin pages for tree nodes).

Requirements:
1. Node layout: internal nodes store (key, child_page_id) pairs; leaf nodes
   store (key, value) pairs and a right-sibling pointer for range scans.
   Serialize nodes into the Page byte buffer from the storage crate.
2. Operations: insert(key, value), search(key) -> Option<value>,
   delete(key), and range_scan(start_key, end_key) -> iterator over
   (key, value) pairs in order.
3. Node splitting on overflow during insert, and merging/redistribution on
   underflow during delete (implement at least merging; redistribution is a
   bonus).
4. Generic over key type (support at minimum i64 and String keys).

Write unit tests: insert-then-search round trips, splits under heavy insert
load (enough keys to force multiple levels), deletes with merges, and an
ordered range scan test. Include a test that inserts thousands of random
keys and verifies the tree stays balanced (check height stays O(log n)).
```

### Phase 3 — Catalog

```
Implement the catalog crate for FerriteDB.

Requirements:
1. Column and Schema types: column name, data type (support Int, BigInt,
   Varchar(n), Boolean, Float at minimum), nullability.
2. Table struct: table name, Schema, and the page_id of its first data page
   (or its index root page_id for indexed tables).
3. Catalog struct: an in-memory registry of all tables (name -> Table),
   backed by a system table persisted via the storage crate so it survives
   restarts (a simple approach: serialize the catalog to a reserved page_id
   on shutdown/checkpoint and deserialize on startup is fine for now).
4. Methods: create_table(name, schema), get_table(name), drop_table(name),
   list_tables().

Write unit tests for create/get/drop and for persistence (create tables,
"restart" by reloading the catalog from disk, verify tables are still there).
```

### Phase 4 — SQL Frontend

```
Implement the sql crate for FerriteDB: a lexer, recursive-descent parser,
and AST for a SQL subset.

Support these statements:
- CREATE TABLE name (col type [NOT NULL], ...)
- INSERT INTO name VALUES (...)
- SELECT col, col FROM table [WHERE condition] [JOIN table ON condition]
- UPDATE table SET col = value [WHERE condition]
- DELETE FROM table [WHERE condition]

WHERE conditions should support: =, !=, <, >, <=, >=, AND, OR, and literal
comparisons (int, string, bool).

Requirements:
1. Lexer: tokenize SQL text into keywords, identifiers, literals, operators,
   punctuation. Handle whitespace/comments correctly.
2. AST: define enums/structs for each statement type and expression type.
3. Parser: recursive-descent parser producing the AST, with clear syntax
   error messages (include line/column if reasonably easy).

Write unit tests: one test per statement type with a few variations, plus
tests for malformed SQL producing sensible parse errors (not panics).
```

### Phase 5 — Query Execution

```
Implement the exec crate for FerriteDB using the Volcano/iterator execution
model. Each operator implements an Executor trait with a next() -> Option<Tuple>
method, pulling from child operators.

Implement these operators, wiring them to the catalog and storage/index crates:
1. SeqScanExecutor — scans all tuples in a table via the storage crate.
2. IndexScanExecutor — scans via the index crate for equality/range predicates.
3. FilterExecutor — evaluates a WHERE expression (from the sql crate's AST)
   against each tuple from its child, passing through matches.
4. ProjectExecutor — selects specific columns from each tuple.
5. NestedLoopJoinExecutor — joins two child executors on an equality condition.

Also implement a simple planner function: given a parsed sql::ast::Statement
and the catalog, build the corresponding executor tree (e.g. SELECT with a
WHERE clause becomes Project(Filter(SeqScan))).

Write integration tests: create a table via catalog, insert tuples via
INSERT execution, then run SELECT queries with WHERE and JOIN and assert
on the returned rows.
```

### Phase 6 — Optimizer

```
Implement the optimizer crate for FerriteDB.

Requirements:
1. Rule-based rewrites on the logical query plan before execution:
   - Predicate pushdown: push WHERE filters as close to scans as possible,
     including into join conditions where applicable.
   - Index selection: if a WHERE clause filters on an indexed column with
     an equality or range predicate, rewrite SeqScan -> IndexScan.
2. A basic cost model: estimate row counts per table (track approximate
   row counts in the catalog) and prefer IndexScan over SeqScan when the
   estimated selectivity is low; prefer the smaller side as the outer loop
   in NestedLoopJoin.
3. A planner entry point: takes a sql::ast::Statement + catalog, produces
   an optimized executor tree (reusing the exec crate's operators).

Write tests that assert on the *shape* of the produced plan (e.g. "WHERE id
= 5 on an indexed column produces an IndexScan, not a SeqScan") in addition
to correctness tests that check query results are unchanged by optimization.
```

### Phase 7 — Transactions & Concurrency

```
Implement the txn crate for FerriteDB.

Requirements:
1. Write-Ahead Log (WAL): before any page modification is flushed to disk,
   append a log record (transaction id, page id, before/after image or
   redo/undo info) to a log file. Implement log flushing and a basic
   crash-recovery routine (replay committed transactions' redo records on
   startup; undo uncommitted ones).
2. Concurrency control — pick ONE and implement it fully:
   - MVCC: each tuple gets a version with transaction id + timestamp;
     readers see a consistent snapshot without blocking writers, OR
   - 2PL: a LockManager granting shared/exclusive locks per tuple/page,
     blocking conflicting transactions, with deadlock detection (wait-for
     graph cycle detection is sufficient).
3. Transaction API: begin_transaction(), commit(), abort(), wired into the
   exec crate so INSERT/UPDATE/DELETE/SELECT all run inside a transaction
   context.

Write tests: crash-recovery test (simulate a crash mid-transaction, verify
recovery leaves the DB in a consistent state), and a concurrency test with
multiple threads issuing transactions against overlapping data, verifying
isolation guarantees hold (no dirty reads, etc. — pick the isolation level
you're targeting and test for its specific guarantees).
```

### Phase 8 — CLI / Polish

```
Implement the server crate for FerriteDB as a REPL:

1. main.rs: an interactive command-line loop that reads SQL statements from
   stdin (support multi-line statements terminated by ;), parses them via
   the sql crate, plans them via the optimizer crate, executes them via the
   exec/txn crates, and prints results as a formatted table (or row count
   for INSERT/UPDATE/DELETE).
2. Support a few meta-commands: .tables (list tables via catalog),
   .schema <table>, .exit.
3. Nice error messages on parse/execution failures instead of panics.

Then write an integration test suite under tests/integration/: a set of
.sql files with expected output, run through the full pipeline end to end,
covering CREATE TABLE, INSERT, SELECT with WHERE/JOIN, UPDATE, DELETE, and
at least one crash-recovery scenario.

Finally, update the README with: setup instructions, a usage example (REPL
session), the architecture diagram (crates and how they connect), and a
"what I learned" section per module (storage, indexing, parsing,
optimization, transactions, concurrency).
```

---

## 4. Working across phases

- Give the agent **one phase prompt per session**, on its own branch (`git checkout -b phase-1-storage`), and get it merged before moving on — don't let context bleed across phases.
- After each phase, ask the agent to write a short `docs/design_notes.md` entry explaining the tradeoffs it made (LRU vs clock, MVCC vs 2PL, etc). These notes are what you'll actually talk about in interviews.
- If a phase stalls or produces something you don't understand, stop and have it explain the code to you before moving on — the goal is that *you* can defend every design decision, not just that it compiles.

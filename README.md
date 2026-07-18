# FerriteDB

> A relational database engine built from scratch in Rust — storage engine, B+Tree indexing, SQL parsing, query execution, optimizer, and full ACID transactions.

[![CI](https://github.com/H8rsh100/FerriteDB/actions/workflows/ci.yml/badge.svg)](https://github.com/H8rsh100/FerriteDB/actions/workflows/ci.yml)
![Rust](https://img.shields.io/badge/Rust-1.75%2B-orange?logo=rust)
![License](https://img.shields.io/badge/license-MIT-blue)
![Status](https://img.shields.io/badge/status-active%20development-brightgreen)

---

## What is FerriteDB?

FerriteDB is a ground-up implementation of a relational database engine in Rust. No external database libraries — every layer is written by hand to understand exactly how databases work at the systems level: how pages land on disk, how a B+Tree splits and merges, how a recursive-descent parser turns text into an AST, how the Volcano model pulls rows through a pipeline of operators, and how Write-Ahead Logging enables crash recovery.

This is the kind of project that makes you dangerous in a systems/database engineering interview — you can answer *why* at every layer.

---

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│                     server  (REPL / CLI)                │
│                 sql  ←  optimizer  →  exec              │
│                       catalog                           │
│                   index  (B+Tree)                       │
│                   txn  (WAL + MVCC)                     │
│               storage  (pages + buffer pool)            │
│                      OS  /  disk                        │
└─────────────────────────────────────────────────────────┘
```

Each layer is an independent Rust crate in a Cargo workspace. Crates only depend downward — `exec` depends on `catalog` and `storage`, never the reverse.

---

## Crate Map

| Crate | Phase | Responsibility |
|---|---|---|
| `storage` | 1 | 4 KiB page format, disk manager, buffer pool (LRU eviction) |
| `index` | 2 | Disk-backed B+Tree: insert / search / delete / range scan |
| `catalog` | 3 | Table & schema registry, persisted via a reserved catalog page |
| `sql` | 4 | Lexer + recursive-descent parser → typed AST |
| `exec` | 5 | Volcano-model operators: SeqScan, IndexScan, Filter, Project, Join |
| `optimizer` | 6 | Predicate pushdown, index selection, cost-based join ordering |
| `txn` | 7 | Write-Ahead Log, MVCC (snapshot isolation), crash recovery |
| `server` | 8 | Interactive REPL, meta-commands (`.tables`, `.schema`, `.exit`) |

---

## Phase Roadmap

| Phase | Module | Core deliverable | Status |
|---|---|---|---|
| 0 | Setup | Workspace scaffold, CI, page format skeleton | ✅ Done |
| 1 | Storage | Disk manager + buffer pool (LRU eviction) | 🔄 Next |
| 2 | Indexing | B+Tree with insert/search/delete/range scan | ⬜ |
| 3 | Catalog | Table/schema definitions, system catalog | ⬜ |
| 4 | SQL Frontend | Lexer + recursive-descent parser → AST | ⬜ |
| 5 | Execution | Volcano-model operators (scan/filter/project/join) | ⬜ |
| 6 | Optimizer | Predicate pushdown, index selection, basic cost model | ⬜ |
| 7 | Transactions | WAL, crash recovery, MVCC, snapshot isolation | ⬜ |
| 8 | Polish | REPL/CLI, integration test suite, docs | ⬜ |

**Interview-ready MVP**: Phase 5 — `SELECT … FROM … WHERE …` working end-to-end in memory, even before transactions exist.

---

## Getting Started

### Prerequisites

- Rust 1.75+ (`rustup update stable`)

### Build

```bash
git clone https://github.com/H8rsh100/FerriteDB.git
cd FerriteDB
cargo build --workspace
```

### Test

```bash
cargo test --workspace
```

### Run the REPL *(Phase 8)*

```bash
cargo run --bin ferritedb
```

---

## Design Notes

Design tradeoff writeups live in [`docs/design_notes.md`](docs/design_notes.md) — one entry per phase explaining *why* decisions were made (LRU vs clock, MVCC vs 2PL, etc.). These are the notes you talk through in interviews.

---

## License

MIT — see [LICENSE](LICENSE).

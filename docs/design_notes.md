# Design Notes

Running log of tradeoffs and design decisions made during each phase.
These notes are meant to be read in interviews — "walk me through a design decision you made."

---

## Phase 0 — Workspace Setup

**Crate-per-phase design**: Each phase lives in its own crate. This enforces clean dependency layering at the compiler level (you *cannot* accidentally import `exec` from `storage`) and lets you point an interviewer at a single crate's code without noise from other layers.

**Resolver 2**: The workspace uses Cargo's `resolver = "2"` which applies feature unification per-crate rather than globally. Important once crates start gating optional features (e.g., async I/O for future work).

---

*Phase 1 notes will be added after storage implementation.*

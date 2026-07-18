//! Rule-based query rewrites — Phase 6 implementation.
//!
//! Rules operate on a logical plan tree and return a (possibly rewritten)
//! plan. Each rule is a pure function: `fn rewrite(plan: LogicalPlan) -> LogicalPlan`.
//!
//! Planned rules:
//! - `predicate_pushdown` — push filters toward leaves / into joins.
//! - `index_selection`    — replace SeqScan with IndexScan when applicable.

// Phase 6 implementation.

//! Optimizer crate — rule-based rewrites and cost-based plan selection.
//!
//! Phase 6 will implement predicate pushdown, index selection, and the
//! cost model. For Phase 0 the module tree and planner entry point are declared.

pub mod rules;
pub mod planner;

pub use planner::Planner;

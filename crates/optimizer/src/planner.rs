//! Planner — converts a parsed SQL statement into an (optimized) executor tree.
//!
//! Phase 6 implementation. For Phase 0 the entry point is declared.

use sql::ast::Statement;
use catalog::Catalog;

/// Converts a `Statement` into an optimized executor tree.
pub struct Planner;

impl Planner {
    pub fn new() -> Self { Self }

    /// Produces an optimized execution plan for `stmt`.
    /// Phase 6 implementation — currently a no-op skeleton.
    pub fn plan(&self, _stmt: &Statement, _catalog: &Catalog) {
        // Phase 6 implementation.
    }
}

impl Default for Planner {
    fn default() -> Self { Self::new() }
}

//! FilterExecutor — evaluates a WHERE predicate against each child tuple.
//! Phase 5 implementation.

use crate::{ExecError, Executor, Tuple};
use catalog::Schema;
use sql::ast::Expr;

pub struct FilterExecutor {
    schema: Schema,
    _predicate: Expr,
}

impl FilterExecutor {
    pub fn new(schema: Schema, predicate: Expr) -> Self {
        Self { schema, _predicate: predicate }
    }
}

impl Executor for FilterExecutor {
    fn schema(&self) -> &Schema { &self.schema }
    fn next(&mut self) -> Result<Option<Tuple>, ExecError> {
        Ok(None) // Phase 5 implementation.
    }
}

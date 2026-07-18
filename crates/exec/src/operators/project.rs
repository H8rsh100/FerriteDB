//! ProjectExecutor — projects a subset of columns from child tuples.
//! Phase 5 implementation.

use crate::{ExecError, Executor, Tuple};
use catalog::Schema;

pub struct ProjectExecutor {
    output_schema: Schema,
    _column_indices: Vec<usize>,
}

impl ProjectExecutor {
    pub fn new(output_schema: Schema, column_indices: Vec<usize>) -> Self {
        Self { output_schema, _column_indices: column_indices }
    }
}

impl Executor for ProjectExecutor {
    fn schema(&self) -> &Schema { &self.output_schema }
    fn next(&mut self) -> Result<Option<Tuple>, ExecError> {
        Ok(None) // Phase 5 implementation.
    }
}

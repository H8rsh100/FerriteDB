//! NestedLoopJoinExecutor — joins two child executors on an equality condition.
//! Phase 5 implementation.

use crate::{ExecError, Executor, Tuple};
use catalog::Schema;

pub struct NestedLoopJoinExecutor {
    output_schema: Schema,
}

impl NestedLoopJoinExecutor {
    pub fn new(output_schema: Schema) -> Self {
        Self { output_schema }
    }
}

impl Executor for NestedLoopJoinExecutor {
    fn schema(&self) -> &Schema { &self.output_schema }
    fn next(&mut self) -> Result<Option<Tuple>, ExecError> {
        Ok(None) // Phase 5 implementation.
    }
}

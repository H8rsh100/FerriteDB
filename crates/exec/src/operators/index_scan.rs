//! IndexScanExecutor — scans via the B+Tree for equality/range predicates.
//! Phase 5 implementation.

use crate::{ExecError, Executor, Tuple};
use catalog::Schema;

pub struct IndexScanExecutor {
    schema: Schema,
}

impl IndexScanExecutor {
    pub fn new(schema: Schema) -> Self {
        Self { schema }
    }
}

impl Executor for IndexScanExecutor {
    fn schema(&self) -> &Schema { &self.schema }
    fn next(&mut self) -> Result<Option<Tuple>, ExecError> {
        Ok(None) // Phase 5 implementation.
    }
}

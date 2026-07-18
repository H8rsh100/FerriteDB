//! SeqScanExecutor — scans every tuple in a heap table page by page.
//! Phase 5 implementation.

use crate::{ExecError, Executor, Tuple};
use catalog::Schema;

pub struct SeqScanExecutor {
    schema: Schema,
}

impl SeqScanExecutor {
    pub fn new(schema: Schema) -> Self {
        Self { schema }
    }
}

impl Executor for SeqScanExecutor {
    fn schema(&self) -> &Schema { &self.schema }
    fn next(&mut self) -> Result<Option<Tuple>, ExecError> {
        Ok(None) // Phase 5 implementation.
    }
}

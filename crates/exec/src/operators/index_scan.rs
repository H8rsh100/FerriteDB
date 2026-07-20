//! IndexScan operator — scans B+Tree index range to produce tuples.

use catalog::Schema;
use crate::{ExecError, Executor, Tuple};

pub struct IndexScan {
    schema: Schema,
    tuples: Vec<Tuple>,
    cursor: usize,
}

impl IndexScan {
    pub fn new(schema: Schema, tuples: Vec<Tuple>) -> Self {
        Self {
            schema,
            tuples,
            cursor: 0,
        }
    }
}

impl Executor for IndexScan {
    fn schema(&self) -> &Schema {
        &self.schema
    }

    fn next(&mut self) -> Result<Option<Tuple>, ExecError> {
        if self.cursor < self.tuples.len() {
            let t = self.tuples[self.cursor].clone();
            self.cursor += 1;
            Ok(Some(t))
        } else {
            Ok(None)
        }
    }

    fn reset(&mut self) -> Result<(), ExecError> {
        self.cursor = 0;
        Ok(())
    }
}

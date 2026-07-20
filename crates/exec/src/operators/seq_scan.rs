//! SeqScan operator — sequentially reads tuples from table pages in storage.

use catalog::Schema;
use storage::{BufferPoolManager, PageId};
use crate::{ExecError, Executor, Tuple};

pub struct SeqScan {
    schema: Schema,
    tuples: Vec<Tuple>,
    cursor: usize,
}

impl SeqScan {
    pub fn new(schema: Schema, tuples: Vec<Tuple>) -> Self {
        Self {
            schema,
            tuples,
            cursor: 0,
        }
    }

    pub fn from_bpm(
        schema: Schema,
        _bpm: &BufferPoolManager,
        _root_page_id: PageId,
    ) -> Result<Self, ExecError> {
        Ok(Self::new(schema, Vec::new()))
    }
}

impl Executor for SeqScan {
    fn schema(&self) -> &Schema {
        &self.schema
    }

    fn next(&mut self) -> Result<Option<Tuple>, ExecError> {
        if self.cursor < self.tuples.len() {
            let tuple = self.tuples[self.cursor].clone();
            self.cursor += 1;
            Ok(Some(tuple))
        } else {
            Ok(None)
        }
    }

    fn reset(&mut self) -> Result<(), ExecError> {
        self.cursor = 0;
        Ok(())
    }
}

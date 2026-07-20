//! Filter operator — filters input tuples using a predicate expression.

use catalog::Schema;
use sql::ast::Expr;
use crate::{eval_expr, ExecError, Executor, Tuple};

pub struct Filter {
    child: Box<dyn Executor + Send + Sync>,
    predicate: Expr,
}

impl Filter {
    pub fn new(child: Box<dyn Executor + Send + Sync>, predicate: Expr) -> Self {
        Self { child, predicate }
    }
}

impl Executor for Filter {
    fn schema(&self) -> &Schema {
        self.child.schema()
    }

    fn next(&mut self) -> Result<Option<Tuple>, ExecError> {
        while let Some(tuple) = self.child.next()? {
            let val = eval_expr(&self.predicate, &tuple, self.child.schema())?;
            if val.is_truthy() {
                return Ok(Some(tuple));
            }
        }
        Ok(None)
    }

    fn reset(&mut self) -> Result<(), ExecError> {
        self.child.reset()
    }
}

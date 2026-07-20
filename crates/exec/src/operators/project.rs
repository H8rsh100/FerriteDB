//! Project operator — evaluates expressions to form output tuples.

use catalog::Schema;
use sql::ast::Expr;
use crate::{eval_expr, ExecError, Executor, Tuple};

pub struct Project {
    child: Box<dyn Executor + Send + Sync>,
    exprs: Vec<Expr>,
    output_schema: Schema,
}

impl Project {
    pub fn new(
        child: Box<dyn Executor + Send + Sync>,
        exprs: Vec<Expr>,
        output_schema: Schema,
    ) -> Self {
        Self {
            child,
            exprs,
            output_schema,
        }
    }
}

impl Executor for Project {
    fn schema(&self) -> &Schema {
        &self.output_schema
    }

    fn next(&mut self) -> Result<Option<Tuple>, ExecError> {
        if let Some(child_tuple) = self.child.next()? {
            let mut projected_vals = Vec::with_capacity(self.exprs.len());
            for expr in &self.exprs {
                let val = eval_expr(expr, &child_tuple, self.child.schema())?;
                projected_vals.push(val);
            }
            Ok(Some(Tuple::new(projected_vals)))
        } else {
            Ok(None)
        }
    }

    fn reset(&mut self) -> Result<(), ExecError> {
        self.child.reset()
    }
}

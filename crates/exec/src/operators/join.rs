//! NestedLoopJoin operator — joins left and right child iterators.

use catalog::Schema;
use sql::ast::Expr;
use crate::{eval_expr, ExecError, Executor, Tuple};

pub struct NestedLoopJoin {
    left: Box<dyn Executor + Send + Sync>,
    right: Box<dyn Executor + Send + Sync>,
    join_expr: Option<Expr>,
    output_schema: Schema,
    current_left: Option<Tuple>,
}

impl NestedLoopJoin {
    pub fn new(
        left: Box<dyn Executor + Send + Sync>,
        right: Box<dyn Executor + Send + Sync>,
        join_expr: Option<Expr>,
        output_schema: Schema,
    ) -> Self {
        Self {
            left,
            right,
            join_expr,
            output_schema,
            current_left: None,
        }
    }
}

impl Executor for NestedLoopJoin {
    fn schema(&self) -> &Schema {
        &self.output_schema
    }

    fn next(&mut self) -> Result<Option<Tuple>, ExecError> {
        loop {
            if self.current_left.is_none() {
                self.current_left = self.left.next()?;
                if self.current_left.is_none() {
                    return Ok(None);
                }
                self.right.reset()?;
            }

            let left_tuple = self.current_left.as_ref().unwrap();

            while let Some(right_tuple) = self.right.next()? {
                let mut combined_vals = left_tuple.values.clone();
                combined_vals.extend(right_tuple.values);
                let combined_tuple = Tuple::new(combined_vals);

                if let Some(ref expr) = self.join_expr {
                    let cond = eval_expr(expr, &combined_tuple, &self.output_schema)?;
                    if cond.is_truthy() {
                        return Ok(Some(combined_tuple));
                    }
                } else {
                    return Ok(Some(combined_tuple));
                }
            }

            // Right child exhausted for current_left; move to next left tuple
            self.current_left = None;
        }
    }

    fn reset(&mut self) -> Result<(), ExecError> {
        self.left.reset()?;
        self.right.reset()?;
        self.current_left = None;
        Ok(())
    }
}

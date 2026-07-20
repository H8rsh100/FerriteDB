//! Planner — converts a parsed SQL statement into an (optimized) executor tree.

use catalog::Catalog;
use exec::operators::{Filter, NestedLoopJoin, Project, SeqScan};
use exec::Executor;
use sql::ast::{Expr, SelectList, Statement};
use crate::rules::{push_down_predicates, select_indexes, LogicalPlan};

/// Converts a `Statement` into an optimized executor tree.
pub struct Planner;

impl Planner {
    pub fn new() -> Self { Self }

    /// Builds a logical plan from a SQL `Statement` AST.
    pub fn build_logical_plan(&self, stmt: &Statement, catalog: &Catalog) -> Result<LogicalPlan, String> {
        match stmt {
            Statement::Select { columns, from, joins, filter, .. } => {
                let table_meta = catalog.get_table(from)
                    .ok_or_else(|| format!("table '{from}' not found in catalog"))?;

                let mut plan = LogicalPlan::Scan {
                    table_name: from.clone(),
                    schema: table_meta.schema.clone(),
                };

                for join in joins {
                    let join_table = catalog.get_table(&join.table)
                        .ok_or_else(|| format!("table '{}' not found in catalog", join.table))?;
                    
                    let mut combined_cols = plan.schema().columns.clone();
                    combined_cols.extend(join_table.schema.columns.clone());
                    let combined_schema = catalog::Schema::new(combined_cols);

                    let right_plan = LogicalPlan::Scan {
                        table_name: join.table.clone(),
                        schema: join_table.schema.clone(),
                    };

                    plan = LogicalPlan::Join {
                        left: Box::new(plan),
                        right: Box::new(right_plan),
                        condition: Some(join.condition.clone()),
                        schema: combined_schema,
                    };
                }

                if let Some(ref pred) = filter {
                    plan = LogicalPlan::Filter {
                        input: Box::new(plan),
                        predicate: pred.clone(),
                    };
                }

                match columns {
                    SelectList::Star => Ok(plan),
                    SelectList::Exprs(exprs) => {
                        let mut proj_cols = Vec::new();
                        for expr in exprs {
                            match expr {
                                Expr::Column { name, .. } => {
                                    if let Some(col) = plan.schema().column(name) {
                                        proj_cols.push(col.clone());
                                    } else {
                                        proj_cols.push(catalog::Column::new(name, catalog::DataType::BigInt, true));
                                    }
                                }
                                _ => {
                                    proj_cols.push(catalog::Column::new("expr", catalog::DataType::BigInt, true));
                                }
                            }
                        }
                        let proj_schema = catalog::Schema::new(proj_cols);
                        Ok(LogicalPlan::Project {
                            input: Box::new(plan),
                            exprs: exprs.clone(),
                            schema: proj_schema,
                        })
                    }
                }
            }
            _ => Err("only SELECT statements supported by planner currently".into()),
        }
    }

    /// Optimizes a logical plan by applying predicate pushdown and index selection rules.
    pub fn optimize(&self, plan: LogicalPlan, indexed_cols: &[&str]) -> LogicalPlan {
        let pushed = push_down_predicates(plan);
        select_indexes(pushed, indexed_cols)
    }

    /// Lowers a logical plan into a physical Volcano executor tree.
    pub fn build_physical_plan(&self, plan: LogicalPlan) -> Box<dyn Executor + Send + Sync> {
        match plan {
            LogicalPlan::Scan { schema, .. } => {
                Box::new(SeqScan::new(schema, Vec::new()))
            }
            LogicalPlan::IndexScan { schema, .. } => {
                Box::new(SeqScan::new(schema, Vec::new()))
            }
            LogicalPlan::Filter { input, predicate } => {
                let child = self.build_physical_plan(*input);
                Box::new(Filter::new(child, predicate))
            }
            LogicalPlan::Project { input, exprs, schema } => {
                let child = self.build_physical_plan(*input);
                Box::new(Project::new(child, exprs, schema))
            }
            LogicalPlan::Join { left, right, condition, schema } => {
                let left_exec = self.build_physical_plan(*left);
                let right_exec = self.build_physical_plan(*right);
                Box::new(NestedLoopJoin::new(left_exec, right_exec, condition, schema))
            }
        }
    }
}

impl Default for Planner {
    fn default() -> Self { Self::new() }
}

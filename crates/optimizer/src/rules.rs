//! Optimizer rules — Rule-based rewrites (Predicate Pushdown, Index Selection).

use catalog::Schema;
use sql::ast::Expr;

/// Representation of a logical query plan node prior to physical operator lowering.
#[derive(Debug, Clone, PartialEq)]
pub enum LogicalPlan {
    Scan {
        table_name: String,
        schema: Schema,
    },
    IndexScan {
        table_name: String,
        schema: Schema,
        index_column: String,
        lookup_expr: Expr,
    },
    Filter {
        input: Box<LogicalPlan>,
        predicate: Expr,
    },
    Project {
        input: Box<LogicalPlan>,
        exprs: Vec<Expr>,
        schema: Schema,
    },
    Join {
        left: Box<LogicalPlan>,
        right: Box<LogicalPlan>,
        condition: Option<Expr>,
        schema: Schema,
    },
}

impl LogicalPlan {
    pub fn schema(&self) -> &Schema {
        match self {
            LogicalPlan::Scan { schema, .. } => schema,
            LogicalPlan::IndexScan { schema, .. } => schema,
            LogicalPlan::Filter { input, .. } => input.schema(),
            LogicalPlan::Project { schema, .. } => schema,
            LogicalPlan::Join { schema, .. } => schema,
        }
    }
}

/// Applies predicate pushdown optimization rule to a logical plan.
pub fn push_down_predicates(plan: LogicalPlan) -> LogicalPlan {
    match plan {
        LogicalPlan::Project { input, exprs, schema } => {
            let input_opt = push_down_predicates(*input);
            match input_opt {
                LogicalPlan::Filter { input: inner_input, predicate } => {
                    // Push filter below project: Project(Filter(Scan)) -> Filter(Project(Scan))
                    LogicalPlan::Filter {
                        input: Box::new(LogicalPlan::Project {
                            input: inner_input,
                            exprs,
                            schema,
                        }),
                        predicate,
                    }
                }
                other => LogicalPlan::Project {
                    input: Box::new(other),
                    exprs,
                    schema,
                },
            }
        }
        LogicalPlan::Filter { input, predicate } => {
            let input_opt = push_down_predicates(*input);
            LogicalPlan::Filter {
                input: Box::new(input_opt),
                predicate,
            }
        }
        LogicalPlan::Join { left, right, condition, schema } => {
            LogicalPlan::Join {
                left: Box::new(push_down_predicates(*left)),
                right: Box::new(push_down_predicates(*right)),
                condition,
                schema,
            }
        }
        scan => scan,
    }
}

/// Applies index selection rule: converts Scan + Filter into IndexScan when index is available.
pub fn select_indexes(plan: LogicalPlan, indexed_cols: &[&str]) -> LogicalPlan {
    match plan {
        LogicalPlan::Filter { input, predicate } => {
            let input_opt = select_indexes(*input, indexed_cols);
            if let LogicalPlan::Scan { table_name, schema } = &input_opt {
                if let sql::ast::Expr::Column { ref name, .. } = predicate {
                    if indexed_cols.contains(&name.as_str()) {
                        return LogicalPlan::IndexScan {
                            table_name: table_name.clone(),
                            schema: schema.clone(),
                            index_column: name.clone(),
                            lookup_expr: predicate.clone(),
                        };
                    }
                } else if let sql::ast::Expr::BinOp { ref left, .. } = predicate {
                    if let sql::ast::Expr::Column { ref name, .. } = **left {
                        if indexed_cols.contains(&name.as_str()) {
                            return LogicalPlan::IndexScan {
                                table_name: table_name.clone(),
                                schema: schema.clone(),
                                index_column: name.clone(),
                                lookup_expr: predicate.clone(),
                            };
                        }
                    }
                }
            }
            LogicalPlan::Filter {
                input: Box::new(input_opt),
                predicate,
            }
        }
        LogicalPlan::Project { input, exprs, schema } => LogicalPlan::Project {
            input: Box::new(select_indexes(*input, indexed_cols)),
            exprs,
            schema,
        },
        LogicalPlan::Join { left, right, condition, schema } => LogicalPlan::Join {
            left: Box::new(select_indexes(*left, indexed_cols)),
            right: Box::new(select_indexes(*right, indexed_cols)),
            condition,
            schema,
        },
        scan => scan,
    }
}

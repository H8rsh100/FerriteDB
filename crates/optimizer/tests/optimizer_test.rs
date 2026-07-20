use catalog::{Catalog, Column, DataType, Schema};
use optimizer::rules::{push_down_predicates, select_indexes, LogicalPlan};
use optimizer::Planner;
use sql::lexer::Lexer;
use sql::parser::Parser;

fn setup_catalog() -> Catalog {
    let mut catalog = Catalog::new(1);
    let users_schema = Schema::new(vec![
        Column::new("id", DataType::BigInt, false),
        Column::new("name", DataType::Varchar(50), true),
        Column::new("age", DataType::BigInt, false),
    ]);
    catalog.create_table("users".into(), users_schema, 10).unwrap();
    catalog
}

#[test]
fn test_logical_plan_generation() {
    let catalog = setup_catalog();
    let sql = "SELECT name, age FROM users WHERE age >= 21;";
    let mut lexer = Lexer::new(sql);
    let tokens = lexer.tokenize().unwrap();
    let mut parser = Parser::new(tokens);
    let stmts = parser.parse().unwrap();

    let planner = Planner::new();
    let logical = planner.build_logical_plan(&stmts[0], &catalog).unwrap();

    match logical {
        LogicalPlan::Project { input, exprs, .. } => {
            assert_eq!(exprs.len(), 2);
            match *input {
                LogicalPlan::Filter { predicate, .. } => {
                    assert!(matches!(predicate, sql::ast::Expr::BinOp { .. }));
                }
                _ => panic!("expected Filter node"),
            }
        }
        _ => panic!("expected Project node"),
    }
}

#[test]
fn test_predicate_pushdown_rule() {
    let catalog = setup_catalog();
    let sql = "SELECT name FROM users WHERE age >= 18;";
    let mut lexer = Lexer::new(sql);
    let tokens = lexer.tokenize().unwrap();
    let mut parser = Parser::new(tokens);
    let stmts = parser.parse().unwrap();

    let planner = Planner::new();
    let logical = planner.build_logical_plan(&stmts[0], &catalog).unwrap();
    let optimized = push_down_predicates(logical);

    match optimized {
        LogicalPlan::Filter { input, .. } => {
            assert!(matches!(*input, LogicalPlan::Project { .. }));
        }
        _ => panic!("expected Filter as top node after pushdown"),
    }
}

#[test]
fn test_index_selection_rule() {
    let catalog = setup_catalog();
    let sql = "SELECT name FROM users WHERE age >= 18;";
    let mut lexer = Lexer::new(sql);
    let tokens = lexer.tokenize().unwrap();
    let mut parser = Parser::new(tokens);
    let stmts = parser.parse().unwrap();

    let planner = Planner::new();
    let logical = planner.build_logical_plan(&stmts[0], &catalog).unwrap();
    let index_optimized = select_indexes(logical, &["age"]);

    match index_optimized {
        LogicalPlan::Project { input, .. } => {
            assert!(matches!(*input, LogicalPlan::IndexScan { .. }));
        }
        _ => panic!("expected IndexScan after index selection rule"),
    }
}

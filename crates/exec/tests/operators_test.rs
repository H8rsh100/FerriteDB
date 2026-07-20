use catalog::{Column, DataType, Schema};
use exec::operators::{Filter, NestedLoopJoin, Project, SeqScan};
use exec::{Executor, Tuple, Value};
use sql::ast::{BinOp, Expr};
use sql::Span;

fn dummy_span() -> Span {
    Span { line: 1, col: 1 }
}

#[test]
fn test_seq_scan_filter_project_pipeline() {
    let schema = Schema::new(vec![
        Column::new("id", DataType::BigInt, false),
        Column::new("name", DataType::Varchar(50), true),
        Column::new("age", DataType::BigInt, false),
    ]);

    let tuples = vec![
        Tuple::new(vec![Value::BigInt(1), Value::Varchar("Alice".into()), Value::BigInt(25)]),
        Tuple::new(vec![Value::BigInt(2), Value::Varchar("Bob".into()), Value::BigInt(17)]),
        Tuple::new(vec![Value::BigInt(3), Value::Varchar("Charlie".into()), Value::BigInt(30)]),
    ];

    let seq_scan = Box::new(SeqScan::new(schema.clone(), tuples));

    // Filter: age >= 18
    let pred = Expr::BinOp {
        op: BinOp::Gte,
        left: Box::new(Expr::Column { table: None, name: "age".into() }),
        right: Box::new(Expr::IntLit(18)),
        span: dummy_span(),
    };
    let filter = Box::new(Filter::new(seq_scan, pred));

    // Project: name, id
    let proj_schema = Schema::new(vec![
        Column::new("name", DataType::Varchar(50), true),
        Column::new("id", DataType::BigInt, false),
    ]);
    let proj_exprs = vec![
        Expr::Column { table: None, name: "name".into() },
        Expr::Column { table: None, name: "id".into() },
    ];
    let mut project = Project::new(filter, proj_exprs, proj_schema);

    let res1 = project.next().unwrap().unwrap();
    assert_eq!(res1.values, vec![Value::Varchar("Alice".into()), Value::BigInt(1)]);

    let res2 = project.next().unwrap().unwrap();
    assert_eq!(res2.values, vec![Value::Varchar("Charlie".into()), Value::BigInt(3)]);

    assert!(project.next().unwrap().is_none());
}

#[test]
fn test_nested_loop_join() {
    let left_schema = Schema::new(vec![
        Column::new("id", DataType::BigInt, false),
        Column::new("name", DataType::Varchar(50), true),
    ]);
    let left_tuples = vec![
        Tuple::new(vec![Value::BigInt(1), Value::Varchar("Alice".into())]),
        Tuple::new(vec![Value::BigInt(2), Value::Varchar("Bob".into())]),
    ];
    let left_scan = Box::new(SeqScan::new(left_schema, left_tuples));

    let right_schema = Schema::new(vec![
        Column::new("user_id", DataType::BigInt, false),
        Column::new("item", DataType::Varchar(50), true),
    ]);
    let right_tuples = vec![
        Tuple::new(vec![Value::BigInt(1), Value::Varchar("Laptop".into())]),
        Tuple::new(vec![Value::BigInt(1), Value::Varchar("Book".into())]),
        Tuple::new(vec![Value::BigInt(2), Value::Varchar("Phone".into())]),
    ];
    let right_scan = Box::new(SeqScan::new(right_schema, right_tuples));

    let out_schema = Schema::new(vec![
        Column::new("id", DataType::BigInt, false),
        Column::new("name", DataType::Varchar(50), true),
        Column::new("user_id", DataType::BigInt, false),
        Column::new("item", DataType::Varchar(50), true),
    ]);

    // Join cond: id = user_id
    let join_expr = Expr::BinOp {
        op: BinOp::Eq,
        left: Box::new(Expr::Column { table: None, name: "id".into() }),
        right: Box::new(Expr::Column { table: None, name: "user_id".into() }),
        span: dummy_span(),
    };

    let mut join = NestedLoopJoin::new(left_scan, right_scan, Some(join_expr), out_schema);

    let j1 = join.next().unwrap().unwrap();
    assert_eq!(j1.values[0], Value::BigInt(1));
    assert_eq!(j1.values[3], Value::Varchar("Laptop".into()));

    let j2 = join.next().unwrap().unwrap();
    assert_eq!(j2.values[0], Value::BigInt(1));
    assert_eq!(j2.values[3], Value::Varchar("Book".into()));

    let j3 = join.next().unwrap().unwrap();
    assert_eq!(j3.values[0], Value::BigInt(2));
    assert_eq!(j3.values[3], Value::Varchar("Phone".into()));

    assert!(join.next().unwrap().is_none());
}

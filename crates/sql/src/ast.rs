//! AST — typed representation of SQL statements and expressions.
//!
//! Each SQL statement maps to one `Statement` variant.
//! Expressions recurse through `Expr`.
//! Phase 4 will wire these to the parser.

use crate::lexer::Span;

/// A fully-parsed SQL statement.
#[derive(Debug, Clone)]
pub enum Statement {
    /// `CREATE TABLE name (col type [NOT NULL], ...)`
    CreateTable {
        name: String,
        columns: Vec<ColumnDef>,
    },
    /// `INSERT INTO name VALUES (...)`
    Insert {
        table: String,
        values: Vec<Expr>,
    },
    /// `SELECT ... FROM table [JOIN ...] [WHERE ...] [ORDER BY ...] [LIMIT ...]`
    Select {
        columns: SelectList,
        from: String,
        joins: Vec<JoinClause>,
        filter: Option<Expr>,
        order_by: Vec<(Expr, Order)>,
        limit: Option<u64>,
        offset: Option<u64>,
    },
    /// `UPDATE table SET col = expr [WHERE ...]`
    Update {
        table: String,
        assignments: Vec<(String, Expr)>,
        filter: Option<Expr>,
    },
    /// `DELETE FROM table [WHERE ...]`
    Delete {
        table: String,
        filter: Option<Expr>,
    },
    BeginTransaction,
    Commit,
    Abort,
}

/// Column definition inside CREATE TABLE.
#[derive(Debug, Clone)]
pub struct ColumnDef {
    pub name: String,
    pub data_type: SqlType,
    pub nullable: bool,
}

/// SQL type as parsed from the source text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SqlType {
    Int,
    BigInt,
    Varchar(usize),
    Boolean,
    Float,
}

/// The list of projected expressions in a SELECT.
#[derive(Debug, Clone)]
pub enum SelectList {
    /// `SELECT *`
    Star,
    /// `SELECT col1, col2, ...`
    Exprs(Vec<Expr>),
}

/// JOIN clause in a SELECT.
#[derive(Debug, Clone)]
pub struct JoinClause {
    pub table: String,
    pub condition: Expr,
}

/// Sort direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Order {
    Asc,
    Desc,
}

/// A SQL expression — literals, column references, binary ops, etc.
#[derive(Debug, Clone)]
pub enum Expr {
    /// Integer literal.
    IntLit(i64),
    /// Float literal.
    FloatLit(f64),
    /// String literal.
    StrLit(String),
    /// Boolean literal.
    BoolLit(bool),
    /// NULL.
    Null,
    /// Column reference, optionally table-qualified (`table.column`).
    Column { table: Option<String>, name: String },
    /// Binary expression.
    BinOp {
        op: BinOp,
        left: Box<Expr>,
        right: Box<Expr>,
        span: Span,
    },
}

/// Binary operators.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinOp {
    Eq, Neq, Lt, Gt, Lte, Gte,
    And, Or,
    Add, Sub, Mul, Div,
}

//! Exec crate — Volcano/iterator model query execution.

pub mod operators;

use catalog::Schema;
use sql::ast::{BinOp, Expr};

/// A typed SQL value.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Int(i32),
    BigInt(i64),
    Varchar(String),
    Boolean(bool),
    Float(f64),
    Null,
}

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Int(v)     => write!(f, "{v}"),
            Value::BigInt(v)  => write!(f, "{v}"),
            Value::Varchar(s) => write!(f, "{s}"),
            Value::Boolean(b) => write!(f, "{b}"),
            Value::Float(v)   => write!(f, "{v}"),
            Value::Null       => write!(f, "NULL"),
        }
    }
}

impl PartialOrd for Value {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        match (self, other) {
            (Value::Int(a), Value::Int(b)) => a.partial_cmp(b),
            (Value::BigInt(a), Value::BigInt(b)) => a.partial_cmp(b),
            (Value::Float(a), Value::Float(b)) => a.partial_cmp(b),
            (Value::Varchar(a), Value::Varchar(b)) => a.partial_cmp(b),
            (Value::Boolean(a), Value::Boolean(b)) => a.partial_cmp(b),
            (Value::Int(a), Value::BigInt(b)) => (*a as i64).partial_cmp(b),
            (Value::BigInt(a), Value::Int(b)) => a.partial_cmp(&(*b as i64)),
            _ => None,
        }
    }
}

impl Value {
    pub fn encode(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        match self {
            Value::Null => buf.push(0),
            Value::Int(n) => {
                buf.push(1);
                buf.extend_from_slice(&n.to_le_bytes());
            }
            Value::BigInt(n) => {
                buf.push(2);
                buf.extend_from_slice(&n.to_le_bytes());
            }
            Value::Varchar(s) => {
                buf.push(3);
                let bytes = s.as_bytes();
                buf.extend_from_slice(&(bytes.len() as u16).to_le_bytes());
                buf.extend_from_slice(bytes);
            }
            Value::Boolean(b) => {
                buf.push(4);
                buf.push(*b as u8);
            }
            Value::Float(f) => {
                buf.push(5);
                buf.extend_from_slice(&f.to_le_bytes());
            }
        }
        buf
    }

    pub fn decode(bytes: &[u8]) -> Option<(Self, usize)> {
        if bytes.is_empty() {
            return None;
        }
        match bytes[0] {
            0 => Some((Value::Null, 1)),
            1 => {
                if bytes.len() < 5 { return None; }
                let val = i32::from_le_bytes(bytes[1..5].try_into().ok()?);
                Some((Value::Int(val), 5))
            }
            2 => {
                if bytes.len() < 9 { return None; }
                let val = i64::from_le_bytes(bytes[1..9].try_into().ok()?);
                Some((Value::BigInt(val), 9))
            }
            3 => {
                if bytes.len() < 3 { return None; }
                let len = u16::from_le_bytes(bytes[1..3].try_into().ok()?) as usize;
                if bytes.len() < 3 + len { return None; }
                let s = std::str::from_utf8(&bytes[3..3 + len]).ok()?.to_string();
                Some((Value::Varchar(s), 3 + len))
            }
            4 => {
                if bytes.len() < 2 { return None; }
                Some((Value::Boolean(bytes[1] != 0), 2))
            }
            5 => {
                if bytes.len() < 9 { return None; }
                let val = f64::from_le_bytes(bytes[1..9].try_into().ok()?);
                Some((Value::Float(val), 9))
            }
            _ => None,
        }
    }

    pub fn is_truthy(&self) -> bool {
        match self {
            Value::Boolean(b) => *b,
            Value::Int(n) => *n != 0,
            Value::BigInt(n) => *n != 0,
            Value::Null => false,
            _ => false,
        }
    }
}

/// A row returned by an executor.
#[derive(Debug, Clone, PartialEq)]
pub struct Tuple {
    pub values: Vec<Value>,
}

impl Tuple {
    pub fn new(values: Vec<Value>) -> Self {
        Self { values }
    }

    pub fn encode(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.extend_from_slice(&(self.values.len() as u16).to_le_bytes());
        for val in &self.values {
            buf.extend_from_slice(&val.encode());
        }
        buf
    }

    pub fn decode(bytes: &[u8]) -> Option<(Self, usize)> {
        if bytes.len() < 2 {
            return None;
        }
        let num_vals = u16::from_le_bytes(bytes[0..2].try_into().ok()?) as usize;
        let mut pos = 2;
        let mut values = Vec::with_capacity(num_vals);
        for _ in 0..num_vals {
            let (val, read) = Value::decode(&bytes[pos..])?;
            values.push(val);
            pos += read;
        }
        Some((Tuple { values }, pos))
    }
}

/// Error type for execution failures.
#[derive(Debug, Clone, PartialEq)]
pub enum ExecError {
    TypeError(String),
    CatalogError(String),
    StorageError(String),
}

impl std::fmt::Display for ExecError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExecError::TypeError(s)    => write!(f, "type error: {s}"),
            ExecError::CatalogError(s) => write!(f, "catalog error: {s}"),
            ExecError::StorageError(s) => write!(f, "storage error: {s}"),
        }
    }
}

impl std::error::Error for ExecError {}

/// Evaluates a SQL expression against a input tuple and schema context.
pub fn eval_expr(expr: &Expr, tuple: &Tuple, schema: &Schema) -> Result<Value, ExecError> {
    match expr {
        Expr::IntLit(n) => Ok(Value::BigInt(*n)),
        Expr::FloatLit(f) => Ok(Value::Float(*f)),
        Expr::StrLit(s) => Ok(Value::Varchar(s.clone())),
        Expr::BoolLit(b) => Ok(Value::Boolean(*b)),
        Expr::Null => Ok(Value::Null),
        Expr::Column { name, .. } => {
            let idx = schema.column_index(name).ok_or_else(|| {
                ExecError::CatalogError(format!("column '{name}' not found in schema"))
            })?;
            tuple.values.get(idx).cloned().ok_or_else(|| {
                ExecError::TypeError(format!("tuple index out of bounds for column '{name}'"))
            })
        }
        Expr::BinOp { op, left, right, .. } => {
            let l_val = eval_expr(left, tuple, schema)?;
            let r_val = eval_expr(right, tuple, schema)?;
            eval_binop(*op, &l_val, &r_val)
        }
    }
}

fn eval_binop(op: BinOp, left: &Value, right: &Value) -> Result<Value, ExecError> {
    match op {
        BinOp::Eq => Ok(Value::Boolean(left == right)),
        BinOp::Neq => Ok(Value::Boolean(left != right)),
        BinOp::Lt => Ok(Value::Boolean(left < right)),
        BinOp::Gt => Ok(Value::Boolean(left > right)),
        BinOp::Lte => Ok(Value::Boolean(left <= right)),
        BinOp::Gte => Ok(Value::Boolean(left >= right)),
        BinOp::And => Ok(Value::Boolean(left.is_truthy() && right.is_truthy())),
        BinOp::Or => Ok(Value::Boolean(left.is_truthy() || right.is_truthy())),
        BinOp::Add => match (left, right) {
            (Value::BigInt(a), Value::BigInt(b)) => Ok(Value::BigInt(a + b)),
            (Value::Int(a), Value::Int(b)) => Ok(Value::Int(a + b)),
            (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a + b)),
            _ => Err(ExecError::TypeError("invalid addition operands".into())),
        },
        BinOp::Sub => match (left, right) {
            (Value::BigInt(a), Value::BigInt(b)) => Ok(Value::BigInt(a - b)),
            (Value::Int(a), Value::Int(b)) => Ok(Value::Int(a - b)),
            (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a - b)),
            _ => Err(ExecError::TypeError("invalid subtraction operands".into())),
        },
        BinOp::Mul => match (left, right) {
            (Value::BigInt(a), Value::BigInt(b)) => Ok(Value::BigInt(a * b)),
            (Value::Int(a), Value::Int(b)) => Ok(Value::Int(a * b)),
            (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a * b)),
            _ => Err(ExecError::TypeError("invalid multiplication operands".into())),
        },
        BinOp::Div => match (left, right) {
            (Value::BigInt(a), Value::BigInt(b)) => Ok(Value::BigInt(a / b)),
            (Value::Int(a), Value::Int(b)) => Ok(Value::Int(a / b)),
            (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a / b)),
            _ => Err(ExecError::TypeError("invalid division operands".into())),
        },
    }
}

/// The core iterator trait every Volcano operator must implement.
pub trait Executor {
    /// Returns the output schema of this operator.
    fn schema(&self) -> &Schema;

    /// Advances the iterator and returns the next tuple, or `None` if exhausted.
    fn next(&mut self) -> Result<Option<Tuple>, ExecError>;

    /// Resets the executor to the beginning (needed for nested-loop join restarts).
    fn reset(&mut self) -> Result<(), ExecError> {
        Ok(())
    }
}

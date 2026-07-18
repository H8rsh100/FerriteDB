//! Exec crate — Volcano/iterator model query execution.
//!
//! Phase 5 will implement all operators. For Phase 0 the module tree and
//! the core `Executor` trait are declared.

pub mod operators;

use catalog::Schema;

/// A row returned by an executor.
/// Each element corresponds to one column in the output schema.
#[derive(Debug, Clone)]
pub struct Tuple {
    pub values: Vec<Value>,
}

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

/// Error type for execution failures.
#[derive(Debug)]
pub enum ExecError {
    /// Type mismatch during evaluation.
    TypeError(String),
    /// Schema / catalog lookup failed.
    CatalogError(String),
    /// Underlying storage error.
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

/// The core iterator trait every operator must implement.
///
/// ```text
/// while let Some(tuple) = executor.next()? {
///     process(tuple);
/// }
/// ```
pub trait Executor {
    /// Returns the output schema of this operator.
    fn schema(&self) -> &Schema;

    /// Advances the iterator and returns the next tuple, or `None` if exhausted.
    fn next(&mut self) -> Result<Option<Tuple>, ExecError>;

    /// Resets the executor to the beginning (needed for nested-loop join restarts).
    fn reset(&mut self) -> Result<(), ExecError> {
        Ok(()) // default: no-op
    }
}

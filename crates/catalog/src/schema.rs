//! Schema types — column definitions and data types.

/// SQL data types supported by FerriteDB.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DataType {
    /// 32-bit signed integer.
    Int,
    /// 64-bit signed integer.
    BigInt,
    /// Variable-length string up to `n` bytes (UTF-8).
    Varchar(usize),
    /// Boolean (true/false).
    Boolean,
    /// 64-bit IEEE 754 floating-point.
    Float,
}

impl std::fmt::Display for DataType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DataType::Int => write!(f, "INT"),
            DataType::BigInt => write!(f, "BIGINT"),
            DataType::Varchar(n) => write!(f, "VARCHAR({n})"),
            DataType::Boolean => write!(f, "BOOLEAN"),
            DataType::Float => write!(f, "FLOAT"),
        }
    }
}

/// A single column definition.
#[derive(Debug, Clone)]
pub struct Column {
    pub name: String,
    pub data_type: DataType,
    /// If true, NULL values are allowed in this column.
    pub nullable: bool,
}

impl Column {
    pub fn new(name: impl Into<String>, data_type: DataType, nullable: bool) -> Self {
        Self {
            name: name.into(),
            data_type,
            nullable,
        }
    }
}

impl std::fmt::Display for Column {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let null_str = if self.nullable { "NULL" } else { "NOT NULL" };
        write!(f, "{} {} {}", self.name, self.data_type, null_str)
    }
}

/// Ordered list of column definitions for a table.
#[derive(Debug, Clone)]
pub struct Schema {
    pub columns: Vec<Column>,
}

impl Schema {
    pub fn new(columns: Vec<Column>) -> Self {
        Self { columns }
    }

    /// Returns the index of a column by name, or None.
    pub fn column_index(&self, name: &str) -> Option<usize> {
        self.columns.iter().position(|c| c.name == name)
    }

    /// Returns a reference to the column definition, or None.
    pub fn column(&self, name: &str) -> Option<&Column> {
        self.columns.iter().find(|c| c.name == name)
    }
}

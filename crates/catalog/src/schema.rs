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

impl DataType {
    pub fn encode(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        match self {
            DataType::Int => buf.push(0),
            DataType::BigInt => buf.push(1),
            DataType::Varchar(n) => {
                buf.push(2);
                buf.extend_from_slice(&(*n as u32).to_le_bytes());
            }
            DataType::Boolean => buf.push(3),
            DataType::Float => buf.push(4),
        }
        buf
    }

    pub fn decode(bytes: &[u8]) -> Option<(Self, usize)> {
        if bytes.is_empty() {
            return None;
        }
        match bytes[0] {
            0 => Some((DataType::Int, 1)),
            1 => Some((DataType::BigInt, 1)),
            2 => {
                if bytes.len() < 5 {
                    return None;
                }
                let len = u32::from_le_bytes(bytes[1..5].try_into().ok()?) as usize;
                Some((DataType::Varchar(len), 5))
            }
            3 => Some((DataType::Boolean, 1)),
            4 => Some((DataType::Float, 1)),
            _ => None,
        }
    }
}

/// A single column definition.
#[derive(Debug, Clone, PartialEq, Eq)]
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

    pub fn encode(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        let name_bytes = self.name.as_bytes();
        buf.extend_from_slice(&(name_bytes.len() as u16).to_le_bytes());
        buf.extend_from_slice(name_bytes);
        buf.extend_from_slice(&self.data_type.encode());
        buf.push(self.nullable as u8);
        buf
    }

    pub fn decode(bytes: &[u8]) -> Option<(Self, usize)> {
        if bytes.len() < 2 {
            return None;
        }
        let name_len = u16::from_le_bytes(bytes[0..2].try_into().ok()?) as usize;
        let mut pos = 2;
        if bytes.len() < pos + name_len {
            return None;
        }
        let name = std::str::from_utf8(&bytes[pos..pos + name_len]).ok()?.to_string();
        pos += name_len;

        let (data_type, dt_read) = DataType::decode(&bytes[pos..])?;
        pos += dt_read;

        if bytes.len() < pos + 1 {
            return None;
        }
        let nullable = bytes[pos] != 0;
        pos += 1;

        Some((Column { name, data_type, nullable }, pos))
    }
}

impl std::fmt::Display for Column {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let null_str = if self.nullable { "NULL" } else { "NOT NULL" };
        write!(f, "{} {} {}", self.name, self.data_type, null_str)
    }
}

/// Ordered list of column definitions for a table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Schema {
    pub columns: Vec<Column>,
}

impl Schema {
    pub fn new(columns: Vec<Column>) -> Self {
        Self { columns }
    }

    pub fn encode(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.extend_from_slice(&(self.columns.len() as u16).to_le_bytes());
        for col in &self.columns {
            buf.extend_from_slice(&col.encode());
        }
        buf
    }

    pub fn decode(bytes: &[u8]) -> Option<(Self, usize)> {
        if bytes.len() < 2 {
            return None;
        }
        let num_cols = u16::from_le_bytes(bytes[0..2].try_into().ok()?) as usize;
        let mut pos = 2;
        let mut columns = Vec::with_capacity(num_cols);

        for _ in 0..num_cols {
            let (col, read) = Column::decode(&bytes[pos..])?;
            columns.push(col);
            pos += read;
        }

        Some((Schema { columns }, pos))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schema_encode_decode_roundtrip() {
        let schema = Schema::new(vec![
            Column::new("id", DataType::BigInt, false),
            Column::new("name", DataType::Varchar(100), true),
            Column::new("active", DataType::Boolean, false),
        ]);

        let bytes = schema.encode();
        let (decoded, read) = Schema::decode(&bytes).unwrap();
        assert_eq!(read, bytes.len());
        assert_eq!(decoded, schema);
    }
}

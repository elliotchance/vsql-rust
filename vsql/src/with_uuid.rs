// Conversions between uuid's Uuid and BINARY(16) (or its string).

use uuid::Uuid;

use crate::error::Result;
use crate::types::{FromSql, FromSqlError, ToSql, Value};

/// A BINARY VARYING of 16 bytes.
impl ToSql for Uuid {
    fn to_sql(&self) -> Result<Value> {
        Ok(Value::Binary(self.as_bytes().to_vec()))
    }
}

/// A binary string of 16 bytes, or a character string such as
/// "67e55044-10b1-426f-9247-bb680e5fe0c8".
impl FromSql for Uuid {
    fn from_sql(value: &Value) -> Result<Self, FromSqlError> {
        match value {
            Value::Binary(b) => Uuid::from_slice(b),
            Value::Text(s) => Uuid::parse_str(s),
            _ => return Err(FromSqlError::InvalidType),
        }
        .map_err(|e| FromSqlError::Other(Box::new(e)))
    }
}

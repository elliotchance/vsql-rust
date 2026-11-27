// Conversions between serde_json's Value and character strings.

use crate::error::Result;
use crate::types::{FromSql, FromSqlError, ToSql, Value};

/// A CHARACTER VARYING of the JSON text.
impl ToSql for serde_json::Value {
    fn to_sql(&self) -> Result<Value> {
        Ok(Value::Text(self.to_string()))
    }
}

/// A character string of JSON text.
impl FromSql for serde_json::Value {
    fn from_sql(value: &Value) -> Result<Self, FromSqlError> {
        match value {
            Value::Text(s) => serde_json::from_str(s).map_err(|e| FromSqlError::Other(Box::new(e))),
            _ => Err(FromSqlError::InvalidType),
        }
    }
}

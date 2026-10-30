// Conversions between rust_decimal's Decimal and DECIMAL or NUMERIC.

use std::str::FromStr;

use rust_decimal::Decimal;

use crate::error::Result;
use crate::types::{FromSql, FromSqlError, ToSql, Value};

/// A NUMERIC.
impl ToSql for Decimal {
    fn to_sql(&self) -> Result<Value> {
        Ok(Value::Numeric(self.to_string()))
    }
}

/// A DECIMAL or NUMERIC (or an integer).
impl FromSql for Decimal {
    fn from_sql(value: &Value) -> Result<Self, FromSqlError> {
        match value {
            Value::Numeric(s) => Decimal::from_str(s).map_err(|e| FromSqlError::Other(Box::new(e))),
            Value::Integer(i) => Ok(Decimal::from(*i)),
            _ => Err(FromSqlError::InvalidType),
        }
    }
}

// Conversions between bigdecimal's BigDecimal and DECIMAL or NUMERIC.

use std::str::FromStr;

use bigdecimal::BigDecimal;

use crate::error::Result;
use crate::types::{FromSql, FromSqlError, ToSql, Value};

/// A NUMERIC.
impl ToSql for BigDecimal {
    fn to_sql(&self) -> Result<Value> {
        // Without an exponent, which a NUMERIC can't have.
        Ok(Value::Numeric(self.to_plain_string()))
    }
}

/// A DECIMAL, NUMERIC or DECFLOAT (or an integer).
impl FromSql for BigDecimal {
    fn from_sql(value: &Value) -> Result<Self, FromSqlError> {
        match value {
            // A DECFLOAT may have an exponent.
            Value::Numeric(s) | Value::DecFloat(s) => {
                BigDecimal::from_str(s).map_err(|e| FromSqlError::Other(Box::new(e)))
            }
            Value::Integer(i) => Ok(BigDecimal::from(*i)),
            _ => Err(FromSqlError::InvalidType),
        }
    }
}

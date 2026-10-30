//! Converting between Rust and SQL values.

use std::error;
use std::fmt;
use std::str::FromStr;

use vsql_sys::*;

use crate::error::{Error, Result};
use crate::raw::{Param, ParamValue};

/// An SQL type, from a column's description.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Type {
    SmallInt,
    Integer,
    BigInt,
    Real,
    DoublePrecision,
    Decimal,
    Numeric,
    Character,
    CharacterVarying,
    Boolean,
    Date,
    Time,
    TimeWithTimeZone,
    Timestamp,
    TimestampWithTimeZone,
    /// A data type code that isn't known.
    Other(i16),
}

impl Type {
    pub(crate) fn from_code(code: SQLSMALLINT) -> Type {
        match code {
            SQL_SMALLINT => Type::SmallInt,
            SQL_INTEGER => Type::Integer,
            SQL_BIGINT => Type::BigInt,
            SQL_REAL => Type::Real,
            SQL_FLOAT | SQL_DOUBLE => Type::DoublePrecision,
            SQL_DECIMAL => Type::Decimal,
            SQL_NUMERIC => Type::Numeric,
            SQL_CHAR => Type::Character,
            SQL_VARCHAR => Type::CharacterVarying,
            SQL_BOOLEAN => Type::Boolean,
            SQL_TYPE_DATE => Type::Date,
            SQL_TYPE_TIME => Type::Time,
            SQL_TYPE_TIME_WITH_TIMEZONE => Type::TimeWithTimeZone,
            SQL_TYPE_TIMESTAMP => Type::Timestamp,
            SQL_TYPE_TIMESTAMP_WITH_TIMEZONE => Type::TimestampWithTimeZone,
            other => Type::Other(other),
        }
    }

    /// The SQL name of the type, such as `"CHARACTER VARYING"`.
    pub fn name(&self) -> &'static str {
        match self {
            Type::SmallInt => "SMALLINT",
            Type::Integer => "INTEGER",
            Type::BigInt => "BIGINT",
            Type::Real => "REAL",
            Type::DoublePrecision => "DOUBLE PRECISION",
            Type::Decimal => "DECIMAL",
            Type::Numeric => "NUMERIC",
            Type::Character => "CHARACTER",
            Type::CharacterVarying => "CHARACTER VARYING",
            Type::Boolean => "BOOLEAN",
            Type::Date => "DATE",
            Type::Time => "TIME",
            Type::TimeWithTimeZone => "TIME WITH TIME ZONE",
            Type::Timestamp => "TIMESTAMP",
            Type::TimestampWithTimeZone => "TIMESTAMP WITH TIME ZONE",
            Type::Other(_) => "UNKNOWN",
        }
    }
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// A DATE.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Date {
    pub year: i32,
    pub month: u8,
    pub day: u8,
}

/// A TIME, or a TIME WITH TIME ZONE if it has an offset.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Time {
    pub hour: u8,
    pub minute: u8,
    pub second: u8,
    pub microsecond: u32,
    /// The offset from UTC in minutes (east is positive), for a TIME WITH TIME
    /// ZONE.
    pub offset: Option<i16>,
}

/// A TIMESTAMP, or a TIMESTAMP WITH TIME ZONE if its time has an offset.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Timestamp {
    pub date: Date,
    pub time: Time,
}

impl fmt::Display for Date {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:04}-{:02}-{:02}", self.year, self.month, self.day)
    }
}

impl fmt::Display for Time {
    /// The time with microseconds, and the offset as `+HH:MM`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:02}:{:02}:{:02}.{:06}",
            self.hour, self.minute, self.second, self.microsecond
        )?;
        if let Some(offset) = self.offset {
            let sign = if offset < 0 { '-' } else { '+' };
            let offset = offset.unsigned_abs();
            write!(f, "{sign}{:02}:{:02}", offset / 60, offset % 60)?;
        }

        Ok(())
    }
}

impl fmt::Display for Timestamp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.date, self.time)
    }
}

// The datetimes from SQLGetData, such as "2022-01-02T15:04:05.123456+0530".
// There are at most 6 fractional digits.

fn digits<T: FromStr>(s: &str, n: usize) -> Option<T> {
    if s.len() == n && s.bytes().all(|b| b.is_ascii_digit()) {
        s.parse().ok()
    } else {
        None
    }
}

fn parse_date(s: &str) -> Option<Date> {
    let mut parts = s.splitn(3, '-');
    let date = Date {
        year: digits(parts.next()?, 4)?,
        month: digits(parts.next()?, 2)?,
        day: digits(parts.next()?, 2)?,
    };

    Some(date)
}

fn parse_time(s: &str) -> Option<Time> {
    // The offset is "+HHMM" or "-HHMM".
    let (s, offset) = match s.find(['+', '-']) {
        Some(i) => {
            let (s, offset) = s.split_at(i);
            let hhmm = offset.get(1..)?;
            if hhmm.len() != 4 {
                return None;
            }
            let minutes = digits::<i16>(&hhmm[..2], 2)? * 60 + digits::<i16>(&hhmm[2..], 2)?;
            (
                s,
                Some(if offset.starts_with('-') {
                    -minutes
                } else {
                    minutes
                }),
            )
        }
        None => (s, None),
    };
    let (s, fraction) = match s.split_once('.') {
        Some((s, fraction)) if (1..=6).contains(&fraction.len()) => (s, Some(fraction)),
        Some(_) => return None,
        None => (s, None),
    };
    let mut parts = s.splitn(3, ':');
    let time = Time {
        hour: digits(parts.next()?, 2)?,
        minute: digits(parts.next()?, 2)?,
        second: digits(parts.next()?, 2)?,
        microsecond: match fraction {
            Some(f) => digits::<u32>(f, f.len())? * 10u32.pow(6 - f.len() as u32),
            None => 0,
        },
        offset,
    };

    Some(time)
}

fn parse_timestamp(s: &str) -> Option<Timestamp> {
    let (date, time) = s.split_once('T')?;

    Some(Timestamp {
        date: parse_date(date)?,
        time: parse_time(time)?,
    })
}

/// An SQL value. A value that's read is the [`Value`] of the column's type,
/// and a parameter (see [`ToSql`]) is sent as the SQL type of its [`Value`],
/// which is cast to the type it's used as.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    /// NULL.
    Null,
    /// A BOOLEAN.
    Boolean(bool),
    /// A SMALLINT, INTEGER or BIGINT (a parameter is a BIGINT).
    Integer(i64),
    /// A REAL or DOUBLE PRECISION (a parameter is a DOUBLE PRECISION).
    Double(f64),
    /// A DECIMAL or NUMERIC, which is exact, as a decimal string such as
    /// `"-1.25"` (a parameter is a NUMERIC).
    Numeric(String),
    /// A CHARACTER or CHARACTER VARYING (a parameter is a CHARACTER VARYING).
    Text(String),
    /// A DATE.
    Date(Date),
    /// A TIME (WITH TIME ZONE).
    Time(Time),
    /// A TIMESTAMP (WITH TIME ZONE).
    Timestamp(Timestamp),
}

/// How a column is read with SQLGetData.
#[derive(Clone, Copy)]
pub(crate) enum Kind {
    Integer,
    Double,
    Real,
    Text,
}

impl Kind {
    pub(crate) fn of(ty: Type) -> Kind {
        match ty {
            Type::SmallInt | Type::Integer | Type::BigInt => Kind::Integer,
            Type::Real => Kind::Real,
            Type::DoublePrecision => Kind::Double,
            _ => Kind::Text,
        }
    }
}

impl Value {
    /// The value of a column that's read as a string.
    pub(crate) fn parse(s: String, ty: Type) -> Result<Value, FromSqlError> {
        let invalid = |s: &str| FromSqlError::Other(format!("invalid {ty}: {s:?}").into());
        Ok(match ty {
            Type::Boolean => match s.as_str() {
                "TRUE" => Value::Boolean(true),
                "FALSE" => Value::Boolean(false),
                _ => return Err(invalid(&s)),
            },
            Type::Decimal | Type::Numeric => Value::Numeric(s),
            Type::Date => Value::Date(parse_date(&s).ok_or_else(|| invalid(&s))?),
            Type::Time | Type::TimeWithTimeZone => {
                Value::Time(parse_time(&s).ok_or_else(|| invalid(&s))?)
            }
            Type::Timestamp | Type::TimestampWithTimeZone => {
                Value::Timestamp(parse_timestamp(&s).ok_or_else(|| invalid(&s))?)
            }
            _ => Value::Text(s),
        })
    }

    /// The parameter for this value.
    pub(crate) fn to_param(&self, index: usize) -> Result<Param> {
        let text = |parameter_type, column_size, decimal_digits, s: String| Param {
            value_type: SQL_CHAR,
            parameter_type,
            column_size,
            decimal_digits,
            value: ParamValue::Text(s),
        };
        let invalid = |message: String| Error::InvalidParameter { index, message };

        Ok(match self {
            // The type doesn't matter for a NULL, since it's cast to the type
            // it's used as.
            Value::Null => Param {
                value_type: SQL_CHAR,
                parameter_type: SQL_VARCHAR,
                column_size: 1,
                decimal_digits: 0,
                value: ParamValue::Null,
            },
            Value::Boolean(b) => text(
                SQL_BOOLEAN,
                0,
                0,
                if *b { "TRUE" } else { "FALSE" }.to_string(),
            ),
            Value::Integer(i) => Param {
                value_type: SQL_BIGINT,
                parameter_type: SQL_BIGINT,
                column_size: 0,
                decimal_digits: 0,
                value: ParamValue::BigInt(*i),
            },
            Value::Double(d) => Param {
                value_type: SQL_DOUBLE,
                parameter_type: SQL_DOUBLE,
                column_size: 0,
                decimal_digits: 0,
                value: ParamValue::Double(*d),
            },
            Value::Numeric(s) => {
                // A NUMERIC with the precision of the digits. vsql counts at
                // least one digit before the decimal point (so 0.25 is
                // NUMERIC(3, 2)).
                let unsigned = s.strip_prefix('-').unwrap_or(s);
                let (integer, fraction) = unsigned.split_once('.').unwrap_or((unsigned, ""));
                let valid = |p: &str| p.bytes().all(|b| b.is_ascii_digit());
                if integer.is_empty() || !valid(integer) || !valid(fraction) {
                    return Err(invalid(format!("{s:?} isn't a decimal number")));
                }
                let integer_digits = integer.trim_start_matches('0').len().max(1);
                let precision = SQLINTEGER::try_from(integer_digits + fraction.len())
                    .map_err(|_| invalid(format!("{s} has too many digits")))?;
                let scale = SQLSMALLINT::try_from(fraction.len())
                    .map_err(|_| invalid(format!("{s} has too many digits")))?;
                text(SQL_NUMERIC, precision, scale, s.clone())
            }
            Value::Text(s) => {
                // A CHARACTER VARYING as long as the string (which can't be 0).
                let chars = SQLINTEGER::try_from(s.chars().count().max(1))
                    .map_err(|_| invalid("the string is too long".to_string()))?;
                text(SQL_VARCHAR, chars, 0, s.clone())
            }
            Value::Date(d) => {
                let s = d.to_string();
                text(SQL_TYPE_DATE, s.len() as SQLINTEGER, 0, s)
            }
            Value::Time(t) => {
                let s = t.to_string();
                let ty = if t.offset.is_some() {
                    SQL_TYPE_TIME_WITH_TIMEZONE
                } else {
                    SQL_TYPE_TIME
                };
                text(ty, s.len() as SQLINTEGER, 6, s)
            }
            Value::Timestamp(t) => {
                let s = t.to_string();
                let ty = if t.time.offset.is_some() {
                    SQL_TYPE_TIMESTAMP_WITH_TIMEZONE
                } else {
                    SQL_TYPE_TIMESTAMP
                };
                text(ty, s.len() as SQLINTEGER, 6, s)
            }
        })
    }
}

/// A type that can be a parameter.
pub trait ToSql {
    fn to_sql(&self) -> Result<Value>;
}

impl<T: ToSql + ?Sized> ToSql for &T {
    fn to_sql(&self) -> Result<Value> {
        (**self).to_sql()
    }
}

impl<T: ToSql> ToSql for Option<T> {
    fn to_sql(&self) -> Result<Value> {
        match self {
            Some(v) => v.to_sql(),
            None => Ok(Value::Null),
        }
    }
}

impl<T: ToSql + ?Sized> ToSql for Box<T> {
    fn to_sql(&self) -> Result<Value> {
        (**self).to_sql()
    }
}

impl ToSql for Value {
    fn to_sql(&self) -> Result<Value> {
        Ok(self.clone())
    }
}

impl ToSql for bool {
    fn to_sql(&self) -> Result<Value> {
        Ok(Value::Boolean(*self))
    }
}

macro_rules! to_sql_integer {
    ($($t:ty),*) => {$(
        impl ToSql for $t {
            /// A BIGINT, or a NUMERIC if it's out of range.
            #[allow(unreachable_patterns)]
            fn to_sql(&self) -> Result<Value> {
                Ok(match i64::try_from(*self) {
                    Ok(i) => Value::Integer(i),
                    Err(_) => Value::Numeric(self.to_string()),
                })
            }
        }
    )*};
}

to_sql_integer!(i8, i16, i32, i64, i128, isize, u8, u16, u32, u64, u128, usize);

impl ToSql for f32 {
    fn to_sql(&self) -> Result<Value> {
        Ok(Value::Double(f64::from(*self)))
    }
}

impl ToSql for f64 {
    fn to_sql(&self) -> Result<Value> {
        Ok(Value::Double(*self))
    }
}

impl ToSql for str {
    fn to_sql(&self) -> Result<Value> {
        Ok(Value::Text(self.to_string()))
    }
}

impl ToSql for String {
    fn to_sql(&self) -> Result<Value> {
        Ok(Value::Text(self.clone()))
    }
}

impl ToSql for Date {
    fn to_sql(&self) -> Result<Value> {
        Ok(Value::Date(*self))
    }
}

impl ToSql for Time {
    fn to_sql(&self) -> Result<Value> {
        Ok(Value::Time(*self))
    }
}

impl ToSql for Timestamp {
    fn to_sql(&self) -> Result<Value> {
        Ok(Value::Timestamp(*self))
    }
}

/// Why a value couldn't be converted to a Rust type (see [`FromSql`]).
#[derive(Debug)]
#[non_exhaustive]
pub enum FromSqlError {
    /// The SQL type can't be converted to the Rust type.
    InvalidType,
    /// The value is out of the range of the Rust type.
    OutOfRange,
    /// Another problem with the value.
    Other(Box<dyn error::Error + Send + Sync>),
}

impl fmt::Display for FromSqlError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FromSqlError::InvalidType => f.write_str("the type can't be converted"),
            FromSqlError::OutOfRange => f.write_str("the value is out of range"),
            FromSqlError::Other(e) => e.fmt(f),
        }
    }
}

impl error::Error for FromSqlError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            FromSqlError::Other(e) => Some(&**e),
            _ => None,
        }
    }
}

/// A type that a value can be converted to (see [`Row::get`](crate::Row::get)).
pub trait FromSql: Sized {
    fn from_sql(value: &Value) -> Result<Self, FromSqlError>;
}

impl FromSql for Value {
    fn from_sql(value: &Value) -> Result<Self, FromSqlError> {
        Ok(value.clone())
    }
}

/// NULL is None, otherwise it's the value as `T`.
impl<T: FromSql> FromSql for Option<T> {
    fn from_sql(value: &Value) -> Result<Self, FromSqlError> {
        match value {
            Value::Null => Ok(None),
            v => T::from_sql(v).map(Some),
        }
    }
}

impl FromSql for bool {
    fn from_sql(value: &Value) -> Result<Self, FromSqlError> {
        match value {
            Value::Boolean(b) => Ok(*b),
            _ => Err(FromSqlError::InvalidType),
        }
    }
}

macro_rules! from_sql_integer {
    ($($t:ty),*) => {$(
        /// A SMALLINT, INTEGER or BIGINT that's in range.
        impl FromSql for $t {
            fn from_sql(value: &Value) -> Result<Self, FromSqlError> {
                match value {
                    Value::Integer(i) => <$t>::try_from(*i).map_err(|_| FromSqlError::OutOfRange),
                    _ => Err(FromSqlError::InvalidType),
                }
            }
        }
    )*};
}

from_sql_integer!(i8, i16, i32, i64, i128, isize, u8, u16, u32, u64, u128, usize);

/// A REAL or DOUBLE PRECISION, an integer, or a DECIMAL or NUMERIC (which may
/// lose precision).
impl FromSql for f64 {
    fn from_sql(value: &Value) -> Result<Self, FromSqlError> {
        match value {
            Value::Double(d) => Ok(*d),
            Value::Integer(i) => Ok(*i as f64),
            Value::Numeric(s) => s.parse().map_err(|e| FromSqlError::Other(Box::new(e))),
            _ => Err(FromSqlError::InvalidType),
        }
    }
}

/// See f64.
impl FromSql for f32 {
    fn from_sql(value: &Value) -> Result<Self, FromSqlError> {
        f64::from_sql(value).map(|d| d as f32)
    }
}

/// A CHARACTER or CHARACTER VARYING, or a DECIMAL or NUMERIC.
impl FromSql for String {
    fn from_sql(value: &Value) -> Result<Self, FromSqlError> {
        match value {
            Value::Text(s) | Value::Numeric(s) => Ok(s.clone()),
            _ => Err(FromSqlError::InvalidType),
        }
    }
}

impl FromSql for Date {
    fn from_sql(value: &Value) -> Result<Self, FromSqlError> {
        match value {
            Value::Date(d) => Ok(*d),
            _ => Err(FromSqlError::InvalidType),
        }
    }
}

impl FromSql for Time {
    fn from_sql(value: &Value) -> Result<Self, FromSqlError> {
        match value {
            Value::Time(t) => Ok(*t),
            _ => Err(FromSqlError::InvalidType),
        }
    }
}

impl FromSql for Timestamp {
    fn from_sql(value: &Value) -> Result<Self, FromSqlError> {
        match value {
            Value::Timestamp(t) => Ok(*t),
            _ => Err(FromSqlError::InvalidType),
        }
    }
}

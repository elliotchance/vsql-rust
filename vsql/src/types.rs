//! Converting between Rust and SQL values.

use std::error;
use std::fmt;
use std::str::FromStr;
use std::time::Duration;

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
    DecFloat,
    Character,
    CharacterVarying,
    CharacterLargeObject,
    Binary,
    BinaryVarying,
    BinaryLargeObject,
    Boolean,
    Date,
    Time,
    TimeWithTimeZone,
    Timestamp,
    TimestampWithTimeZone,
    /// An INTERVAL, such as INTERVAL YEAR TO MONTH.
    Interval(IntervalFields),
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
            SQL_DECFLOAT => Type::DecFloat,
            SQL_CHAR => Type::Character,
            SQL_VARCHAR => Type::CharacterVarying,
            SQL_CLOB => Type::CharacterLargeObject,
            SQL_BINARY => Type::Binary,
            SQL_VARBINARY => Type::BinaryVarying,
            SQL_BLOB => Type::BinaryLargeObject,
            SQL_BOOLEAN => Type::Boolean,
            SQL_TYPE_DATE => Type::Date,
            SQL_TYPE_TIME => Type::Time,
            SQL_TYPE_TIME_WITH_TIMEZONE => Type::TimeWithTimeZone,
            SQL_TYPE_TIMESTAMP => Type::Timestamp,
            SQL_TYPE_TIMESTAMP_WITH_TIMEZONE => Type::TimestampWithTimeZone,
            other => match IntervalFields::from_code(other) {
                Some(fields) => Type::Interval(fields),
                None => Type::Other(other),
            },
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
            Type::DecFloat => "DECFLOAT",
            Type::Character => "CHARACTER",
            Type::CharacterVarying => "CHARACTER VARYING",
            Type::CharacterLargeObject => "CHARACTER LARGE OBJECT",
            Type::Binary => "BINARY",
            Type::BinaryVarying => "BINARY VARYING",
            Type::BinaryLargeObject => "BINARY LARGE OBJECT",
            Type::Boolean => "BOOLEAN",
            Type::Date => "DATE",
            Type::Time => "TIME",
            Type::TimeWithTimeZone => "TIME WITH TIME ZONE",
            Type::Timestamp => "TIMESTAMP",
            Type::TimestampWithTimeZone => "TIMESTAMP WITH TIME ZONE",
            Type::Interval(fields) => fields.type_name(),
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

/// The fields of an INTERVAL type, such as YEAR TO MONTH.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IntervalFields {
    Year,
    Month,
    Day,
    Hour,
    Minute,
    Second,
    YearToMonth,
    DayToHour,
    DayToMinute,
    DayToSecond,
    HourToMinute,
    HourToSecond,
    MinuteToSecond,
}

impl IntervalFields {
    fn from_code(code: SQLSMALLINT) -> Option<IntervalFields> {
        Some(match code {
            SQL_INTERVAL_YEAR => IntervalFields::Year,
            SQL_INTERVAL_MONTH => IntervalFields::Month,
            SQL_INTERVAL_DAY => IntervalFields::Day,
            SQL_INTERVAL_HOUR => IntervalFields::Hour,
            SQL_INTERVAL_MINUTE => IntervalFields::Minute,
            SQL_INTERVAL_SECOND => IntervalFields::Second,
            SQL_INTERVAL_YEAR_TO_MONTH => IntervalFields::YearToMonth,
            SQL_INTERVAL_DAY_TO_HOUR => IntervalFields::DayToHour,
            SQL_INTERVAL_DAY_TO_MINUTE => IntervalFields::DayToMinute,
            SQL_INTERVAL_DAY_TO_SECOND => IntervalFields::DayToSecond,
            SQL_INTERVAL_HOUR_TO_MINUTE => IntervalFields::HourToMinute,
            SQL_INTERVAL_HOUR_TO_SECOND => IntervalFields::HourToSecond,
            SQL_INTERVAL_MINUTE_TO_SECOND => IntervalFields::MinuteToSecond,
            _ => return None,
        })
    }

    fn type_name(self) -> &'static str {
        match self {
            IntervalFields::Year => "INTERVAL YEAR",
            IntervalFields::Month => "INTERVAL MONTH",
            IntervalFields::Day => "INTERVAL DAY",
            IntervalFields::Hour => "INTERVAL HOUR",
            IntervalFields::Minute => "INTERVAL MINUTE",
            IntervalFields::Second => "INTERVAL SECOND",
            IntervalFields::YearToMonth => "INTERVAL YEAR TO MONTH",
            IntervalFields::DayToHour => "INTERVAL DAY TO HOUR",
            IntervalFields::DayToMinute => "INTERVAL DAY TO MINUTE",
            IntervalFields::DayToSecond => "INTERVAL DAY TO SECOND",
            IntervalFields::HourToMinute => "INTERVAL HOUR TO MINUTE",
            IntervalFields::HourToSecond => "INTERVAL HOUR TO SECOND",
            IntervalFields::MinuteToSecond => "INTERVAL MINUTE TO SECOND",
        }
    }

    /// Whether it's a year-month interval (YEAR, MONTH or YEAR TO MONTH),
    /// otherwise it's a day-time interval.
    pub fn is_year_month(self) -> bool {
        matches!(
            self,
            IntervalFields::Year | IntervalFields::Month | IntervalFields::YearToMonth
        )
    }

    /// The first and last field.
    fn range(self) -> (Field, Field) {
        match self {
            IntervalFields::Year => (Field::Year, Field::Year),
            IntervalFields::Month => (Field::Month, Field::Month),
            IntervalFields::Day => (Field::Day, Field::Day),
            IntervalFields::Hour => (Field::Hour, Field::Hour),
            IntervalFields::Minute => (Field::Minute, Field::Minute),
            IntervalFields::Second => (Field::Second, Field::Second),
            IntervalFields::YearToMonth => (Field::Year, Field::Month),
            IntervalFields::DayToHour => (Field::Day, Field::Hour),
            IntervalFields::DayToMinute => (Field::Day, Field::Minute),
            IntervalFields::DayToSecond => (Field::Day, Field::Second),
            IntervalFields::HourToMinute => (Field::Hour, Field::Minute),
            IntervalFields::HourToSecond => (Field::Hour, Field::Second),
            IntervalFields::MinuteToSecond => (Field::Minute, Field::Second),
        }
    }

    /// The fields from the first to the last.
    fn fields(self) -> impl Iterator<Item = Field> {
        let (first, last) = self.range();
        FIELDS
            .iter()
            .copied()
            .filter(move |field| (first..=last).contains(field))
    }
}

impl fmt::Display for IntervalFields {
    /// Such as "YEAR TO MONTH".
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.type_name()["INTERVAL ".len()..])
    }
}

/// A field of an interval.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Field {
    Year,
    Month,
    Day,
    Hour,
    Minute,
    Second,
}

const FIELDS: [Field; 6] = [
    Field::Year,
    Field::Month,
    Field::Day,
    Field::Hour,
    Field::Minute,
    Field::Second,
];

impl Field {
    fn name(self) -> &'static str {
        match self {
            Field::Year => "YEAR",
            Field::Month => "MONTH",
            Field::Day => "DAY",
            Field::Hour => "HOUR",
            Field::Minute => "MINUTE",
            Field::Second => "SECOND",
        }
    }

    /// The size of the field in months (YEAR and MONTH) or microseconds.
    fn size(self) -> u64 {
        match self {
            Field::Year => 12,
            Field::Month => 1,
            Field::Day => 86_400_000_000,
            Field::Hour => 3_600_000_000,
            Field::Minute => 60_000_000,
            Field::Second => 1_000_000,
        }
    }

    /// The separator before the field, when it isn't the first.
    fn separator(self) -> char {
        match self {
            Field::Month => '-',
            Field::Hour => ' ',
            _ => ':',
        }
    }
}

/// An INTERVAL. A year-month interval (see [`IntervalFields::is_year_month`])
/// is a number of months, and a day-time interval is a number of
/// microseconds. They're negative for a negative interval.
///
/// A day-time interval converts to and from a [`std::time::Duration`] (which
/// can't be negative), and with the `chrono` feature, a `chrono::Duration`
/// (`TimeDelta`). A year-month interval converts to and from a
/// `chrono::Months` (which can't be negative). Smaller units than
/// microseconds are truncated. These types are also parameters, as a
/// `DAY TO SECOND` or `YEAR TO MONTH` interval, which is cast to the interval
/// type it's used as (so a parameter that's cast to `INTERVAL HOUR` loses the
/// minutes and seconds). An interval of the other kind, or one that's out of
/// range, can't be converted.
///
/// ```
/// use std::time::Duration;
/// use vsql::{Interval, IntervalFields};
///
/// # fn main() -> vsql::Result<()> {
/// let conn = vsql::Connection::open_in_memory()?;
/// let d: Duration = conn.query_row("VALUES INTERVAL '1:30' MINUTE TO SECOND", (), |r| r.get(0))?;
/// assert_eq!(d, Duration::from_secs(90));
///
/// let i: Interval = conn.query_row(
///     "VALUES CAST(? AS INTERVAL HOUR TO MINUTE)",
///     [Duration::from_secs(5400)],
///     |r| r.get(0),
/// )?;
/// assert_eq!(i.to_string(), "INTERVAL '1:30' HOUR(2) TO MINUTE");
/// assert_eq!(Duration::try_from(i).unwrap(), Duration::from_secs(5400));
/// # Ok(())
/// # }
/// ```
///
/// ```
/// use vsql::{Interval, IntervalFields};
///
/// # fn main() -> vsql::Result<()> {
/// let conn = vsql::Connection::open_in_memory()?;
/// let i: Interval = conn.query_row("VALUES INTERVAL '-1-02' YEAR TO MONTH", (), |r| r.get(0))?;
/// assert_eq!(
///     i,
///     Interval {
///         fields: IntervalFields::YearToMonth,
///         months: -14,
///         microseconds: 0,
///     }
/// );
/// assert_eq!(i.to_string(), "INTERVAL '-1-02' YEAR(2) TO MONTH");
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Interval {
    pub fields: IntervalFields,
    /// The months of a year-month interval (a multiple of 12 for YEAR), or 0.
    pub months: i64,
    /// The microseconds of a day-time interval, or 0. The fields that aren't
    /// in the interval are left out (so 1.5 days is 1 day as an INTERVAL DAY).
    pub microseconds: i64,
}

impl fmt::Display for Interval {
    /// The interval as a literal, such as `INTERVAL '-1-02' YEAR(2) TO MONTH`.
    /// The first field has the precision it needs (at least 2), and seconds
    /// have 6 fractional digits.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (first, last) = self.fields.range();
        let value = if self.fields.is_year_month() {
            self.months
        } else {
            self.microseconds
        };
        let mut rest = value.unsigned_abs();
        let leading = rest / first.size();
        rest %= first.size();
        let sign = if value < 0 { "-" } else { "" };
        write!(f, "INTERVAL '{sign}{leading}")?;
        for field in self.fields.fields().skip(1) {
            write!(f, "{}{:02}", field.separator(), rest / field.size())?;
            rest %= field.size();
        }
        if last == Field::Second {
            write!(f, ".{rest:06}")?;
        }

        let precision = leading.to_string().len().max(2);
        write!(f, "' {}({precision}", first.name())?;
        if first == Field::Second {
            f.write_str(", 6")?;
        }
        f.write_str(")")?;
        if last != first {
            write!(f, " TO {}", last.name())?;
            if last == Field::Second {
                f.write_str("(6)")?;
            }
        }

        Ok(())
    }
}

// The datetimes from SQLGetData are literals, such as
// "TIMESTAMP '2022-01-02 15:04:05.123456+05:30'". There are at most 6
// fractional digits.

/// The value of a literal, such as "2022-01-02" for "DATE '2022-01-02'".
fn literal<'a>(s: &'a str, keyword: &str) -> Option<&'a str> {
    s.strip_prefix(keyword)?
        .strip_prefix(" '")?
        .strip_suffix('\'')
}

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
    // The offset is "+HH:MM" or "-HH:MM".
    let (s, offset) = match s.find(['+', '-']) {
        Some(i) => {
            let (s, offset) = s.split_at(i);
            let (hh, mm) = offset.get(1..)?.split_once(':')?;
            let minutes = digits::<i16>(hh, 2)? * 60 + digits::<i16>(mm, 2)?;
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

/// A number without a sign.
fn number(s: &str) -> Option<u64> {
    if !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit()) {
        s.parse().ok()
    } else {
        None
    }
}

/// An interval literal, such as "INTERVAL '-1-02' YEAR(2) TO MONTH", which
/// has the fields of its type.
fn parse_interval(s: &str, fields: IntervalFields) -> Option<Interval> {
    let (body, _) = s.strip_prefix("INTERVAL '")?.split_once('\'')?;
    let (negative, mut rest) = match body.strip_prefix('-') {
        Some(body) => (true, body),
        None => (false, body),
    };

    let all: Vec<Field> = fields.fields().collect();
    let mut total: u64 = 0;
    for (i, field) in all.iter().enumerate() {
        let part = match all.get(i + 1) {
            Some(next) => {
                let (part, after) = rest.split_once(next.separator())?;
                rest = after;
                part
            }
            None => rest,
        };
        let (part, fraction) = match part.split_once('.') {
            Some((part, fraction)) if *field == Field::Second && fraction.len() <= 6 => (
                part,
                number(fraction)? * 10u64.pow(6 - fraction.len() as u32),
            ),
            Some(_) => return None,
            None => (part, 0),
        };
        total = total
            .checked_add(number(part)?.checked_mul(field.size())?)?
            .checked_add(fraction)?;
    }

    let total = i64::try_from(total).ok()?;
    let total = if negative { -total } else { total };
    let (months, microseconds) = if fields.is_year_month() {
        (total, 0)
    } else {
        (0, total)
    };

    Some(Interval {
        fields,
        months,
        microseconds,
    })
}

fn parse_timestamp(s: &str) -> Option<Timestamp> {
    let (date, time) = s.split_once(' ')?;

    Some(Timestamp {
        date: parse_date(date)?,
        time: parse_time(time)?,
    })
}

/// An SQL value. A value that's read is the [`Value`] of the column's type,
/// and a parameter (see [`ToSql`]) is sent as the SQL type of its [`Value`],
/// which is cast to the type it's used as.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
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
    /// A DECFLOAT, which is exact, as a decimal string that may have an
    /// exponent, such as `"-1.25"` or `"1.5E+20"`.
    DecFloat(String),
    /// A CHARACTER, CHARACTER VARYING or CHARACTER LARGE OBJECT (a parameter
    /// is a CHARACTER VARYING).
    Text(String),
    /// A BINARY, BINARY VARYING or BINARY LARGE OBJECT (a parameter is a
    /// BINARY VARYING).
    Binary(Vec<u8>),
    /// A DATE.
    Date(Date),
    /// A TIME (WITH TIME ZONE).
    Time(Time),
    /// A TIMESTAMP (WITH TIME ZONE).
    Timestamp(Timestamp),
    /// An INTERVAL.
    Interval(Interval),
}

/// How a column is read with SQLGetData.
#[derive(Clone, Copy)]
pub(crate) enum Kind {
    Integer,
    Double,
    Real,
    Text,
    Binary,
}

impl Kind {
    pub(crate) fn of(ty: Type) -> Kind {
        match ty {
            Type::SmallInt | Type::Integer | Type::BigInt => Kind::Integer,
            Type::Real => Kind::Real,
            Type::DoublePrecision => Kind::Double,
            Type::Binary | Type::BinaryVarying | Type::BinaryLargeObject => Kind::Binary,
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
            Type::DecFloat => Value::DecFloat(s),
            Type::Interval(fields) => {
                Value::Interval(parse_interval(&s, fields).ok_or_else(|| invalid(&s))?)
            }
            Type::Date => Value::Date(
                literal(&s, "DATE")
                    .and_then(parse_date)
                    .ok_or_else(|| invalid(&s))?,
            ),
            Type::Time | Type::TimeWithTimeZone => Value::Time(
                literal(&s, "TIME")
                    .and_then(parse_time)
                    .ok_or_else(|| invalid(&s))?,
            ),
            Type::Timestamp | Type::TimestampWithTimeZone => Value::Timestamp(
                literal(&s, "TIMESTAMP")
                    .and_then(parse_timestamp)
                    .ok_or_else(|| invalid(&s))?,
            ),
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
            // 34 is the precision of a DECFLOAT, and vsql checks the string.
            Value::DecFloat(s) => text(SQL_DECFLOAT, 34, 0, s.clone()),
            Value::Text(s) => {
                // A CHARACTER VARYING as long as the string (which can't be 0).
                let chars = SQLINTEGER::try_from(s.chars().count().max(1))
                    .map_err(|_| invalid("the string is too long".to_string()))?;
                text(SQL_VARCHAR, chars, 0, s.clone())
            }
            Value::Binary(b) => Param {
                value_type: SQL_VARBINARY,
                parameter_type: SQL_VARBINARY,
                // A BINARY VARYING as long as the value (which can't be 0).
                column_size: SQLINTEGER::try_from(b.len().max(1))
                    .map_err(|_| invalid("the binary string is too long".to_string()))?,
                decimal_digits: 0,
                value: ParamValue::Binary(b.clone()),
            },
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
            // The literal is cast to the parameter's interval type.
            Value::Interval(i) => {
                let s = i.to_string();
                text(SQL_VARCHAR, s.len() as SQLINTEGER, 0, s)
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

impl ToSql for [u8] {
    fn to_sql(&self) -> Result<Value> {
        Ok(Value::Binary(self.to_vec()))
    }
}

impl ToSql for Vec<u8> {
    fn to_sql(&self) -> Result<Value> {
        Ok(Value::Binary(self.clone()))
    }
}

impl<const N: usize> ToSql for [u8; N] {
    fn to_sql(&self) -> Result<Value> {
        Ok(Value::Binary(self.to_vec()))
    }
}

impl ToSql for Interval {
    fn to_sql(&self) -> Result<Value> {
        Ok(Value::Interval(*self))
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

/// The integer of a DECIMAL, NUMERIC or DECFLOAT (which may have an exponent,
/// such as `"1.5E+20"`) as a sign and digits, which is an error if it has a
/// fraction.
fn integer_digits(s: &str) -> Result<String, FromSqlError> {
    let not_integer = || FromSqlError::Other(format!("{s} isn't an integer").into());
    let (negative, unsigned) = match s.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, s.strip_prefix('+').unwrap_or(s)),
    };
    let (mantissa, exponent) = match unsigned.split_once(['E', 'e']) {
        Some((mantissa, exponent)) => (
            mantissa,
            exponent.parse::<i64>().map_err(|_| not_integer())?,
        ),
        None => (unsigned, 0),
    };
    let (integer, fraction) = mantissa.split_once('.').unwrap_or((mantissa, ""));
    if integer.is_empty() && fraction.is_empty()
        || !integer
            .bytes()
            .chain(fraction.bytes())
            .all(|b| b.is_ascii_digit())
    {
        return Err(not_integer());
    }

    // The digits without leading zeros, with the decimal point after `point`
    // of them (which may be before or after all of them).
    let all = format!("{integer}{fraction}");
    let digits = all.trim_start_matches('0');
    let leading_zeros = (all.len() - digits.len()) as i64;
    let point = (integer.len() as i64)
        .saturating_add(exponent)
        .saturating_sub(leading_zeros);
    let (whole, rest) = if point <= 0 {
        ("", digits)
    } else if point as usize >= digits.len() {
        (digits, "")
    } else {
        digits.split_at(point as usize)
    };
    if rest.bytes().any(|b| b != b'0') {
        return Err(not_integer());
    }
    if whole.is_empty() {
        return Ok("0".to_string());
    }
    // The zeros after the digits. Anything longer than 40 digits is out of the
    // range of every integer type.
    if point > 40 {
        return Err(FromSqlError::OutOfRange);
    }
    let zeros = point as usize - whole.len();

    Ok(format!(
        "{}{whole}{}",
        if negative { "-" } else { "" },
        "0".repeat(zeros)
    ))
}

macro_rules! from_sql_integer {
    ($($t:ty),*) => {$(
        /// A SMALLINT, INTEGER or BIGINT, or a DECIMAL, NUMERIC or DECFLOAT
        /// that's an integer, which is in range.
        impl FromSql for $t {
            fn from_sql(value: &Value) -> Result<Self, FromSqlError> {
                match value {
                    Value::Integer(i) => <$t>::try_from(*i).map_err(|_| FromSqlError::OutOfRange),
                    Value::Numeric(s) | Value::DecFloat(s) => integer_digits(s)?
                        .parse()
                        .map_err(|_| FromSqlError::OutOfRange),
                    _ => Err(FromSqlError::InvalidType),
                }
            }
        }
    )*};
}

from_sql_integer!(i8, i16, i32, i64, i128, isize, u8, u16, u32, u64, u128, usize);

/// A REAL or DOUBLE PRECISION, an integer, or a DECIMAL, NUMERIC or DECFLOAT
/// (which may lose precision).
impl FromSql for f64 {
    fn from_sql(value: &Value) -> Result<Self, FromSqlError> {
        match value {
            Value::Double(d) => Ok(*d),
            Value::Integer(i) => Ok(*i as f64),
            Value::Numeric(s) | Value::DecFloat(s) => {
                s.parse().map_err(|e| FromSqlError::Other(Box::new(e)))
            }
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

/// A CHARACTER, CHARACTER VARYING or CHARACTER LARGE OBJECT, or a DECIMAL,
/// NUMERIC or DECFLOAT.
impl FromSql for String {
    fn from_sql(value: &Value) -> Result<Self, FromSqlError> {
        match value {
            Value::Text(s) | Value::Numeric(s) | Value::DecFloat(s) => Ok(s.clone()),
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

/// A BINARY, BINARY VARYING or BINARY LARGE OBJECT.
impl FromSql for Vec<u8> {
    fn from_sql(value: &Value) -> Result<Self, FromSqlError> {
        match value {
            Value::Binary(b) => Ok(b.clone()),
            _ => Err(FromSqlError::InvalidType),
        }
    }
}

/// A BINARY, BINARY VARYING or BINARY LARGE OBJECT of N octets.
impl<const N: usize> FromSql for [u8; N] {
    fn from_sql(value: &Value) -> Result<Self, FromSqlError> {
        match value {
            Value::Binary(b) => match <[u8; N]>::try_from(b.as_slice()) {
                Ok(array) => Ok(array),
                Err(_) => Err(FromSqlError::Other(
                    format!("{} octets, not {N}", b.len()).into(),
                )),
            },
            _ => Err(FromSqlError::InvalidType),
        }
    }
}

impl FromSql for Interval {
    fn from_sql(value: &Value) -> Result<Self, FromSqlError> {
        match value {
            Value::Interval(i) => Ok(*i),
            _ => Err(FromSqlError::InvalidType),
        }
    }
}

impl Interval {
    /// A DAY TO SECOND interval.
    pub(crate) fn day_to_second(microseconds: i64) -> Interval {
        Interval {
            fields: IntervalFields::DayToSecond,
            months: 0,
            microseconds,
        }
    }

    /// The microseconds of a day-time interval.
    pub(crate) fn day_time(&self) -> Result<i64, FromSqlError> {
        if self.fields.is_year_month() {
            return Err(FromSqlError::InvalidType);
        }

        Ok(self.microseconds)
    }
}

/// A DAY TO SECOND interval. Nanoseconds are truncated to microseconds.
impl TryFrom<Duration> for Interval {
    type Error = Error;

    fn try_from(d: Duration) -> Result<Self> {
        i64::try_from(d.as_micros())
            .map(Interval::day_to_second)
            .map_err(|_| Error::ToSqlConversion(format!("{d:?} is too long").into()))
    }
}

/// A day-time interval that isn't negative.
impl TryFrom<Interval> for Duration {
    type Error = FromSqlError;

    fn try_from(i: Interval) -> Result<Self, FromSqlError> {
        let microseconds = u64::try_from(i.day_time()?).map_err(|_| FromSqlError::OutOfRange)?;
        Ok(Duration::from_micros(microseconds))
    }
}

/// A DAY TO SECOND interval, which is cast to the interval type it's used as.
impl ToSql for Duration {
    fn to_sql(&self) -> Result<Value> {
        Interval::try_from(*self).map(Value::Interval)
    }
}

/// A day-time interval that isn't negative.
impl FromSql for Duration {
    fn from_sql(value: &Value) -> Result<Self, FromSqlError> {
        match value {
            Value::Interval(i) => Duration::try_from(*i),
            _ => Err(FromSqlError::InvalidType),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn integer_digits_of_decimals() {
        for (s, want) in [
            ("0", "0"),
            ("-0", "0"),
            ("0.000", "0"),
            ("007", "7"),
            ("+7", "7"),
            ("-12.00", "-12"),
            ("100", "100"),
            ("1.5E+20", "150000000000000000000"),
            ("1.5e20", "150000000000000000000"),
            ("0.05E+2", "5"),
            ("1500E-2", "15"),
            ("0E+999999", "0"),
        ] {
            assert_eq!(integer_digits(s).unwrap(), want, "{s}");
        }
        for s in [
            "1.5", "0.05", "-0.5", "15E-1", "", ".", "1.2.3", "a", "1E", "Infinity", "NaN",
        ] {
            assert!(
                matches!(integer_digits(s), Err(FromSqlError::Other(_))),
                "{s}"
            );
        }
        assert!(matches!(
            integer_digits("1E+41"),
            Err(FromSqlError::OutOfRange)
        ));
    }
}

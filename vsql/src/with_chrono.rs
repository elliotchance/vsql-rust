// Conversions between chrono's types and DATE, TIME and TIMESTAMP. Fractional
// seconds are microseconds (smaller ones are truncated), and an offset is in
// minutes.

use chrono::{
    DateTime, Datelike, FixedOffset, NaiveDate, NaiveDateTime, NaiveTime, TimeZone, Timelike, Utc,
};

use crate::error::{Error, Result};
use crate::types::{Date, FromSql, FromSqlError, Time, Timestamp, ToSql, Value};

fn date(d: NaiveDate) -> Date {
    Date {
        year: d.year(),
        month: d.month() as u8,
        day: d.day() as u8,
    }
}

fn time(t: NaiveTime, offset: Option<i16>) -> Time {
    Time {
        hour: t.hour() as u8,
        minute: t.minute() as u8,
        second: t.second() as u8,
        // A leap second is in the nanoseconds.
        microsecond: (t.nanosecond() / 1000).min(999_999),
        offset,
    }
}

fn offset_minutes(offset: &FixedOffset) -> Result<i16> {
    let seconds = offset.local_minus_utc();
    if seconds % 60 != 0 {
        return Err(Error::ToSqlConversion(
            format!("the offset {offset} isn't a whole number of minutes").into(),
        ));
    }

    Ok((seconds / 60) as i16)
}

fn naive_date(d: &Date) -> Result<NaiveDate, FromSqlError> {
    NaiveDate::from_ymd_opt(d.year, d.month.into(), d.day.into()).ok_or(FromSqlError::OutOfRange)
}

fn naive_time(t: &Time) -> Result<NaiveTime, FromSqlError> {
    NaiveTime::from_hms_micro_opt(
        t.hour.into(),
        t.minute.into(),
        t.second.into(),
        t.microsecond,
    )
    .ok_or(FromSqlError::OutOfRange)
}

fn fixed_offset(minutes: i16) -> Result<FixedOffset, FromSqlError> {
    FixedOffset::east_opt(i32::from(minutes) * 60).ok_or(FromSqlError::OutOfRange)
}

impl ToSql for NaiveDate {
    fn to_sql(&self) -> Result<Value> {
        Ok(Value::Date(date(*self)))
    }
}

impl FromSql for NaiveDate {
    fn from_sql(value: &Value) -> Result<Self, FromSqlError> {
        match value {
            Value::Date(d) => naive_date(d),
            _ => Err(FromSqlError::InvalidType),
        }
    }
}

impl ToSql for NaiveTime {
    fn to_sql(&self) -> Result<Value> {
        Ok(Value::Time(time(*self, None)))
    }
}

/// A TIME (without a time zone).
impl FromSql for NaiveTime {
    fn from_sql(value: &Value) -> Result<Self, FromSqlError> {
        match value {
            Value::Time(t) if t.offset.is_none() => naive_time(t),
            _ => Err(FromSqlError::InvalidType),
        }
    }
}

impl ToSql for NaiveDateTime {
    fn to_sql(&self) -> Result<Value> {
        Ok(Value::Timestamp(Timestamp {
            date: date(self.date()),
            time: time(self.time(), None),
        }))
    }
}

/// A TIMESTAMP (without a time zone).
impl FromSql for NaiveDateTime {
    fn from_sql(value: &Value) -> Result<Self, FromSqlError> {
        match value {
            Value::Timestamp(t) if t.time.offset.is_none() => {
                Ok(naive_date(&t.date)?.and_time(naive_time(&t.time)?))
            }
            _ => Err(FromSqlError::InvalidType),
        }
    }
}

impl ToSql for DateTime<FixedOffset> {
    fn to_sql(&self) -> Result<Value> {
        let local = self.naive_local();

        Ok(Value::Timestamp(Timestamp {
            date: date(local.date()),
            time: time(local.time(), Some(offset_minutes(self.offset())?)),
        }))
    }
}

/// A TIMESTAMP WITH TIME ZONE.
impl FromSql for DateTime<FixedOffset> {
    fn from_sql(value: &Value) -> Result<Self, FromSqlError> {
        match value {
            Value::Timestamp(t) => {
                let offset = fixed_offset(t.time.offset.ok_or(FromSqlError::InvalidType)?)?;
                let local = naive_date(&t.date)?.and_time(naive_time(&t.time)?);
                offset
                    .from_local_datetime(&local)
                    .single()
                    .ok_or(FromSqlError::OutOfRange)
            }
            _ => Err(FromSqlError::InvalidType),
        }
    }
}

impl ToSql for DateTime<Utc> {
    fn to_sql(&self) -> Result<Value> {
        self.fixed_offset().to_sql()
    }
}

/// A TIMESTAMP WITH TIME ZONE, in UTC.
impl FromSql for DateTime<Utc> {
    fn from_sql(value: &Value) -> Result<Self, FromSqlError> {
        DateTime::<FixedOffset>::from_sql(value).map(|t| t.with_timezone(&Utc))
    }
}

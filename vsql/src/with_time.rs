// Conversions between the time crate's types and DATE, TIME, TIMESTAMP and
// INTERVAL. Fractional seconds are microseconds (smaller ones are truncated),
// and an offset is in minutes.

use time::{Date as TimeDate, Duration, Month, OffsetDateTime, PrimitiveDateTime, UtcOffset};

use crate::error::{Error, Result};
use crate::types::{
    Date, FromSql, FromSqlError, Interval, IntervalFields, Time, Timestamp, ToSql, Value,
};

fn date(d: TimeDate) -> Date {
    Date {
        year: d.year(),
        month: d.month().into(),
        day: d.day(),
    }
}

fn time(t: time::Time, offset: Option<i16>) -> Time {
    Time {
        hour: t.hour(),
        minute: t.minute(),
        second: t.second(),
        microsecond: t.microsecond(),
        offset,
    }
}

fn offset_minutes(offset: UtcOffset) -> Result<i16> {
    let seconds = offset.whole_seconds();
    if seconds % 60 != 0 {
        return Err(Error::ToSqlConversion(
            format!("the offset {offset} isn't a whole number of minutes").into(),
        ));
    }

    Ok((seconds / 60) as i16)
}

fn time_date(d: &Date) -> Result<TimeDate, FromSqlError> {
    let month = Month::try_from(d.month).map_err(|_| FromSqlError::OutOfRange)?;
    TimeDate::from_calendar_date(d.year, month, d.day).map_err(|_| FromSqlError::OutOfRange)
}

fn time_time(t: &Time) -> Result<time::Time, FromSqlError> {
    time::Time::from_hms_micro(t.hour, t.minute, t.second, t.microsecond)
        .map_err(|_| FromSqlError::OutOfRange)
}

impl ToSql for TimeDate {
    fn to_sql(&self) -> Result<Value> {
        Ok(Value::Date(date(*self)))
    }
}

impl FromSql for TimeDate {
    fn from_sql(value: &Value) -> Result<Self, FromSqlError> {
        match value {
            Value::Date(d) => time_date(d),
            _ => Err(FromSqlError::InvalidType),
        }
    }
}

impl ToSql for time::Time {
    fn to_sql(&self) -> Result<Value> {
        Ok(Value::Time(time(*self, None)))
    }
}

/// A TIME (without a time zone).
impl FromSql for time::Time {
    fn from_sql(value: &Value) -> Result<Self, FromSqlError> {
        match value {
            Value::Time(t) if t.offset.is_none() => time_time(t),
            _ => Err(FromSqlError::InvalidType),
        }
    }
}

impl ToSql for PrimitiveDateTime {
    fn to_sql(&self) -> Result<Value> {
        Ok(Value::Timestamp(Timestamp {
            date: date(self.date()),
            time: time(self.time(), None),
        }))
    }
}

/// A TIMESTAMP (without a time zone).
impl FromSql for PrimitiveDateTime {
    fn from_sql(value: &Value) -> Result<Self, FromSqlError> {
        match value {
            Value::Timestamp(t) if t.time.offset.is_none() => Ok(PrimitiveDateTime::new(
                time_date(&t.date)?,
                time_time(&t.time)?,
            )),
            _ => Err(FromSqlError::InvalidType),
        }
    }
}

impl ToSql for OffsetDateTime {
    fn to_sql(&self) -> Result<Value> {
        Ok(Value::Timestamp(Timestamp {
            date: date(self.date()),
            time: time(self.time(), Some(offset_minutes(self.offset())?)),
        }))
    }
}

/// A TIMESTAMP WITH TIME ZONE.
impl FromSql for OffsetDateTime {
    fn from_sql(value: &Value) -> Result<Self, FromSqlError> {
        match value {
            Value::Timestamp(t) => {
                let minutes = t.time.offset.ok_or(FromSqlError::InvalidType)?;
                let offset = UtcOffset::from_whole_seconds(i32::from(minutes) * 60)
                    .map_err(|_| FromSqlError::OutOfRange)?;
                Ok(
                    PrimitiveDateTime::new(time_date(&t.date)?, time_time(&t.time)?)
                        .assume_offset(offset),
                )
            }
            _ => Err(FromSqlError::InvalidType),
        }
    }
}

/// A DAY TO SECOND interval. Nanoseconds are truncated to microseconds.
impl TryFrom<Duration> for Interval {
    type Error = Error;

    fn try_from(d: Duration) -> Result<Self> {
        let microseconds = i64::try_from(d.whole_microseconds())
            .map_err(|_| Error::ToSqlConversion(format!("{d} is too long").into()))?;

        Ok(Interval {
            fields: IntervalFields::DayToSecond,
            months: 0,
            microseconds,
        })
    }
}

/// A day-time interval.
impl TryFrom<Interval> for Duration {
    type Error = FromSqlError;

    fn try_from(i: Interval) -> Result<Self, FromSqlError> {
        if i.fields.is_year_month() {
            return Err(FromSqlError::InvalidType);
        }

        Ok(Duration::microseconds(i.microseconds))
    }
}

/// A DAY TO SECOND interval, which is cast to the interval type it's used as.
impl ToSql for Duration {
    fn to_sql(&self) -> Result<Value> {
        Interval::try_from(*self).map(Value::Interval)
    }
}

/// A day-time interval.
impl FromSql for Duration {
    fn from_sql(value: &Value) -> Result<Self, FromSqlError> {
        match value {
            Value::Interval(i) => Duration::try_from(*i),
            _ => Err(FromSqlError::InvalidType),
        }
    }
}

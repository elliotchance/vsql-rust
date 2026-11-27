// Converting intervals to and from duration types.

use std::time::Duration;

use vsql::types::FromSqlError;
use vsql::{Connection, Error, FromSql, Interval, IntervalFields, ToSql, Value};

fn memory() -> Connection {
    Connection::open_in_memory().unwrap()
}

fn interval(fields: IntervalFields, months: i64, microseconds: i64) -> Interval {
    Interval {
        fields,
        months,
        microseconds,
    }
}

#[test]
fn std_duration() {
    use IntervalFields::*;

    let conn = memory();
    let got: Duration = conn
        .query_row("VALUES INTERVAL '1 2:3:4.5' DAY TO SECOND", (), |r| {
            r.get(0)
        })
        .unwrap();
    assert_eq!(got, Duration::from_micros(93_784_500_000));

    // As a parameter, it's cast to the interval type.
    let d = Duration::new(93_784, 500_000_999);
    for (qualifier, want) in [
        ("DAY TO SECOND", interval(DayToSecond, 0, 93_784_500_000)),
        ("HOUR TO SECOND", interval(HourToSecond, 0, 93_784_500_000)),
        ("DAY TO HOUR", interval(DayToHour, 0, 93_600_000_000)),
        ("HOUR(5)", interval(Hour, 0, 93_600_000_000)),
    ] {
        let sql = format!("VALUES CAST(? AS INTERVAL {qualifier})");
        let got: Interval = conn.query_row(&sql, [d], |r| r.get(0)).unwrap();
        assert_eq!(got, want, "{qualifier}");
    }
    let got: Duration = conn
        .query_row("VALUES CAST(? AS INTERVAL DAY TO SECOND)", [d], |r| {
            r.get(0)
        })
        .unwrap();
    assert_eq!(got, Duration::from_micros(93_784_500_000));

    // The nanoseconds are truncated.
    assert_eq!(
        Interval::try_from(Duration::from_nanos(1_999)).unwrap(),
        interval(DayToSecond, 0, 1)
    );
    assert_eq!(
        Duration::try_from(interval(Second, 0, 5)).unwrap(),
        Duration::from_micros(5)
    );

    // Too long for an interval.
    assert!(matches!(
        Duration::MAX.to_sql(),
        Err(Error::ToSqlConversion(_))
    ));

    // A negative or year-month interval, or a value that isn't an interval.
    let err = conn
        .query_row("VALUES INTERVAL '-1' SECOND", (), |r| {
            r.get::<_, Duration>(0)
        })
        .unwrap_err();
    assert!(matches!(
        err,
        Error::FromSqlConversion {
            source: FromSqlError::OutOfRange,
            ..
        }
    ));
    assert!(matches!(
        Duration::try_from(interval(YearToMonth, 14, 0)),
        Err(FromSqlError::InvalidType)
    ));
    assert!(matches!(
        Duration::from_sql(&Value::Integer(1)),
        Err(FromSqlError::InvalidType)
    ));
}

#[cfg(feature = "chrono")]
#[test]
fn chrono_duration() {
    use chrono::TimeDelta;
    use IntervalFields::*;

    let conn = memory();
    let got: TimeDelta = conn
        .query_row("VALUES INTERVAL '-1 2:3:4.5' DAY TO SECOND", (), |r| {
            r.get(0)
        })
        .unwrap();
    assert_eq!(got, TimeDelta::microseconds(-93_784_500_000));

    let d = TimeDelta::microseconds(-5_400_000_001);
    let got: Interval = conn
        .query_row("VALUES CAST(? AS INTERVAL HOUR TO MINUTE)", [d], |r| {
            r.get(0)
        })
        .unwrap();
    assert_eq!(got, interval(HourToMinute, 0, -5_400_000_000));
    let got: TimeDelta = conn
        .query_row("VALUES CAST(? AS INTERVAL MINUTE TO SECOND)", [d], |r| {
            r.get(0)
        })
        .unwrap();
    assert_eq!(got, d);

    // The nanoseconds are truncated (towards 0).
    assert_eq!(
        Interval::try_from(TimeDelta::nanoseconds(-1_999)).unwrap(),
        interval(DayToSecond, 0, -1)
    );

    assert!(matches!(
        TimeDelta::MAX.to_sql(),
        Err(Error::ToSqlConversion(_))
    ));
    assert!(matches!(
        TimeDelta::try_from(interval(Year, 12, 0)),
        Err(FromSqlError::InvalidType)
    ));
    assert!(matches!(
        TimeDelta::from_sql(&Value::Integer(1)),
        Err(FromSqlError::InvalidType)
    ));
}

#[cfg(feature = "chrono")]
#[test]
fn chrono_months() {
    use chrono::Months;
    use IntervalFields::*;

    let conn = memory();
    let got: Months = conn
        .query_row("VALUES INTERVAL '1-2' YEAR TO MONTH", (), |r| r.get(0))
        .unwrap();
    assert_eq!(got, Months::new(14));

    let got: Interval = conn
        .query_row("VALUES CAST(? AS INTERVAL MONTH)", [Months::new(14)], |r| {
            r.get(0)
        })
        .unwrap();
    assert_eq!(got, interval(Month, 14, 0));
    let got: Months = conn
        .query_row("VALUES CAST(? AS INTERVAL YEAR)", [Months::new(14)], |r| {
            r.get(0)
        })
        .unwrap();
    assert_eq!(got, Months::new(12));
    assert_eq!(Interval::from(Months::new(3)), interval(YearToMonth, 3, 0));

    // A negative, too big or day-time interval, or a value that isn't an
    // interval.
    let err = conn
        .query_row("VALUES INTERVAL '-1' MONTH", (), |r| r.get::<_, Months>(0))
        .unwrap_err();
    assert!(matches!(
        err,
        Error::FromSqlConversion {
            source: FromSqlError::OutOfRange,
            ..
        }
    ));
    assert!(matches!(
        Months::try_from(interval(Month, 1 << 40, 0)),
        Err(FromSqlError::OutOfRange)
    ));
    assert!(matches!(
        Months::try_from(interval(Day, 0, 1)),
        Err(FromSqlError::InvalidType)
    ));
    assert!(matches!(
        Months::from_sql(&Value::Integer(1)),
        Err(FromSqlError::InvalidType)
    ));
}

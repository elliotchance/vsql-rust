// The time feature.
#![cfg(feature = "time")]

use time::{Date, Duration, Month, OffsetDateTime, PrimitiveDateTime, Time, UtcOffset};
use vsql::types::FromSqlError;
use vsql::{Connection, Error, FromSql, Interval, IntervalFields, ToSql, Value};

fn memory() -> Connection {
    Connection::open_in_memory().unwrap()
}

#[test]
fn datetimes() {
    let conn = memory();
    let date = Date::from_calendar_date(2022, Month::January, 2).unwrap();
    let time = Time::from_hms_micro(15, 4, 5, 123456).unwrap();
    let naive = PrimitiveDateTime::new(date, time);
    let offset = UtcOffset::from_hms(-4, -30, 0).unwrap();
    let aware = naive.assume_offset(offset);

    let got: Date = conn
        .query_row("VALUES CAST(? AS DATE)", [date], |r| r.get(0))
        .unwrap();
    assert_eq!(got, date);
    let got: Time = conn
        .query_row("VALUES CAST(? AS TIME(6))", [time], |r| r.get(0))
        .unwrap();
    assert_eq!(got, time);
    let got: PrimitiveDateTime = conn
        .query_row("VALUES CAST(? AS TIMESTAMP(6))", [naive], |r| r.get(0))
        .unwrap();
    assert_eq!(got, naive);
    let got: OffsetDateTime = conn
        .query_row(
            "VALUES CAST(? AS TIMESTAMP(6) WITH TIME ZONE)",
            [aware],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(got, aware);
    assert_eq!(got.offset(), offset);
    let got: OffsetDateTime = conn
        .query_row("VALUES TIMESTAMP '2022-01-02 15:04:05.5+05:30'", (), |r| {
            r.get(0)
        })
        .unwrap();
    assert_eq!(
        got,
        PrimitiveDateTime::new(date, Time::from_hms_milli(15, 4, 5, 500).unwrap())
            .assume_offset(UtcOffset::from_hms(5, 30, 0).unwrap())
    );

    // Nanoseconds are truncated.
    let got: Time = conn
        .query_row(
            "VALUES CAST(? AS TIME(6))",
            [Time::from_hms_nano(1, 2, 3, 4_000_999).unwrap()],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(got, Time::from_hms_micro(1, 2, 3, 4_000).unwrap());

    // A TIMESTAMP WITH TIME ZONE isn't a PrimitiveDateTime (the offset would
    // be lost), and a TIMESTAMP isn't an OffsetDateTime. A TIME WITH TIME
    // ZONE isn't a Time.
    let row = conn
        .query_row(
            "VALUES ROW(TIMESTAMP '2022-01-02 15:04:05+05:30', TIMESTAMP '2022-01-02 15:04:05', TIME '01:02:03+05:30')",
            (),
            |r| Ok(r.clone()),
        )
        .unwrap();
    assert!(row.get::<_, PrimitiveDateTime>(0).is_err());
    assert!(row.get::<_, OffsetDateTime>(1).is_err());
    assert!(row.get::<_, Time>(2).is_err());
}

#[test]
fn datetime_errors() {
    // An offset that isn't a whole number of minutes.
    let t = OffsetDateTime::UNIX_EPOCH.to_offset(UtcOffset::from_whole_seconds(30).unwrap());
    assert!(matches!(t.to_sql(), Err(Error::ToSqlConversion(_))));

    let invalid = |r: Result<_, FromSqlError>| matches!(r, Err(FromSqlError::InvalidType));
    assert!(invalid(Date::from_sql(&Value::Integer(1)).map(|_| ())));
    assert!(invalid(Time::from_sql(&Value::Integer(1)).map(|_| ())));
    assert!(invalid(
        PrimitiveDateTime::from_sql(&Value::Integer(1)).map(|_| ())
    ));
    assert!(invalid(
        OffsetDateTime::from_sql(&Value::Integer(1)).map(|_| ())
    ));

    // Values that are out of range of the time crate's types.
    let out_of_range = |r: Result<_, FromSqlError>| matches!(r, Err(FromSqlError::OutOfRange));
    let date = vsql::Date {
        year: 2022,
        month: 1,
        day: 2,
    };
    let time = vsql::Time {
        hour: 1,
        minute: 2,
        second: 3,
        microsecond: 0,
        offset: Some(0),
    };
    assert!(out_of_range(
        Date::from_sql(&Value::Date(vsql::Date { month: 13, ..date })).map(|_| ())
    ));
    assert!(out_of_range(
        Date::from_sql(&Value::Date(vsql::Date { day: 32, ..date })).map(|_| ())
    ));
    assert!(out_of_range(
        Time::from_sql(&Value::Time(vsql::Time {
            hour: 24,
            offset: None,
            ..time
        }))
        .map(|_| ())
    ));
    assert!(out_of_range(
        OffsetDateTime::from_sql(&Value::Timestamp(vsql::Timestamp {
            date,
            time: vsql::Time {
                offset: Some(i16::MAX),
                ..time
            },
        }))
        .map(|_| ())
    ));
}

#[test]
fn durations() {
    use IntervalFields::*;

    let conn = memory();
    let got: Duration = conn
        .query_row("VALUES INTERVAL '-1 2:3:4.5' DAY TO SECOND", (), |r| {
            r.get(0)
        })
        .unwrap();
    assert_eq!(got, Duration::microseconds(-93_784_500_000));

    // As a parameter, it's cast to the interval type.
    let d = Duration::microseconds(-5_400_000_001);
    let got: Interval = conn
        .query_row("VALUES CAST(? AS INTERVAL HOUR TO MINUTE)", [d], |r| {
            r.get(0)
        })
        .unwrap();
    assert_eq!(
        got,
        Interval {
            fields: HourToMinute,
            months: 0,
            microseconds: -5_400_000_000,
        }
    );
    let got: Duration = conn
        .query_row("VALUES CAST(? AS INTERVAL MINUTE TO SECOND)", [d], |r| {
            r.get(0)
        })
        .unwrap();
    assert_eq!(got, d);

    // Nanoseconds are truncated (towards 0).
    assert_eq!(
        Interval::try_from(Duration::nanoseconds(-1_999))
            .unwrap()
            .microseconds,
        -1
    );

    assert!(matches!(
        Duration::MAX.to_sql(),
        Err(Error::ToSqlConversion(_))
    ));
    let year_month = Interval {
        fields: YearToMonth,
        months: 14,
        microseconds: 0,
    };
    assert!(matches!(
        Duration::try_from(year_month),
        Err(FromSqlError::InvalidType)
    ));
    assert!(matches!(
        Duration::from_sql(&Value::Integer(1)),
        Err(FromSqlError::InvalidType)
    ));
}

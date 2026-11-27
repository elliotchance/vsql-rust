// Value and Row ergonomics: borrowed access, From, Display, more ToSql and
// FromSql types, and RowIndex.

use std::borrow::Cow;
use std::rc::Rc;
use std::sync::Arc;

use vsql::{
    Connection, Date, Error, FromSql, Interval, IntervalFields, Time, Timestamp, ToSql, Value,
};

fn memory() -> Connection {
    Connection::open_in_memory().unwrap()
}

#[test]
fn borrowed_access() {
    let conn = memory();
    let row = conn
        .query_row(
            "VALUES ROW(CAST(NULL AS INTEGER), TRUE, 12, CAST(1.5 AS DOUBLE PRECISION), 'abc', \
             CAST(1.25 AS NUMERIC(3, 2)), CAST(X'0aff' AS VARBINARY(2)))",
            (),
            |row| Ok(row.clone()),
        )
        .unwrap();
    let values: Vec<&Value> = (0..row.len()).map(|i| row.get_ref(i).unwrap()).collect();

    let nulls: Vec<bool> = values.iter().map(|v| v.is_null()).collect();
    assert_eq!(nulls, [true, false, false, false, false, false, false]);
    let bools: Vec<_> = values.iter().map(|v| v.as_bool()).collect();
    assert_eq!(bools, [None, Some(true), None, None, None, None, None]);
    let ints: Vec<_> = values.iter().map(|v| v.as_i64()).collect();
    assert_eq!(ints, [None, None, Some(12), None, None, None, None]);
    let doubles: Vec<_> = values.iter().map(|v| v.as_f64()).collect();
    assert_eq!(doubles, [None, None, None, Some(1.5), None, None, None]);
    let strs: Vec<_> = values.iter().map(|v| v.as_str()).collect();
    assert_eq!(strs, [None, None, None, None, Some("abc"), None, None]);
    let decimals: Vec<_> = values.iter().map(|v| v.as_decimal_str()).collect();
    assert_eq!(decimals, [None, None, None, None, None, Some("1.25"), None]);
    let bytes: Vec<_> = values.iter().map(|v| v.as_bytes()).collect();
    assert_eq!(
        bytes,
        [None, None, None, None, None, None, Some(&[10u8, 255][..])]
    );

    assert_eq!(
        Value::DecFloat("1.5E+20".to_string()).as_decimal_str(),
        Some("1.5E+20")
    );
}

#[test]
fn value_from() {
    let date = Date {
        year: 2022,
        month: 1,
        day: 2,
    };
    let time = Time {
        hour: 15,
        minute: 4,
        second: 5,
        microsecond: 0,
        offset: None,
    };
    let timestamp = Timestamp { date, time };
    let interval = Interval {
        fields: IntervalFields::Day,
        months: 0,
        microseconds: 86_400_000_000,
    };
    let text = |s: &str| Value::Text(s.to_string());
    let cases: Vec<(Value, Value)> = vec![
        (Value::from(true), Value::Boolean(true)),
        (Value::from(-1i8), Value::Integer(-1)),
        (Value::from(-2i16), Value::Integer(-2)),
        (Value::from(-3i32), Value::Integer(-3)),
        (Value::from(-4i64), Value::Integer(-4)),
        (Value::from(5u8), Value::Integer(5)),
        (Value::from(6u16), Value::Integer(6)),
        (Value::from(7u32), Value::Integer(7)),
        (Value::from(8u64), Value::Integer(8)),
        (
            Value::from(u64::MAX),
            Value::Numeric("18446744073709551615".to_string()),
        ),
        (Value::from(-9i128), Value::Integer(-9)),
        (Value::from(10isize), Value::Integer(10)),
        (Value::from(11u128), Value::Integer(11)),
        (Value::from(12usize), Value::Integer(12)),
        (Value::from(1.5f32), Value::Double(1.5)),
        (Value::from(2.5f64), Value::Double(2.5)),
        (Value::from("a"), text("a")),
        (Value::from("b".to_string()), text("b")),
        (Value::from(Box::<str>::from("c")), text("c")),
        (Value::from(Cow::Borrowed("d")), text("d")),
        (Value::from('e'), text("e")),
        (Value::from(vec![1u8]), Value::Binary(vec![1])),
        (Value::from(&[2u8][..]), Value::Binary(vec![2])),
        (Value::from([3u8, 4]), Value::Binary(vec![3, 4])),
        (
            Value::from(Box::<[u8]>::from([5u8])),
            Value::Binary(vec![5]),
        ),
        (
            Value::from(Cow::Borrowed(&[6u8][..])),
            Value::Binary(vec![6]),
        ),
        (Value::from(date), Value::Date(date)),
        (Value::from(time), Value::Time(time)),
        (Value::from(timestamp), Value::Timestamp(timestamp)),
        (Value::from(interval), Value::Interval(interval)),
        (Value::from(Some(1)), Value::Integer(1)),
        (Value::from(None::<i32>), Value::Null),
    ];
    for (got, want) in cases {
        assert_eq!(got, want);
    }
}

#[test]
fn value_display() {
    let conn = memory();
    let row = conn
        .query_row(
            "VALUES ROW(CAST(NULL AS INTEGER), TRUE, FALSE, -12, CAST(1.5 AS DOUBLE PRECISION), \
             'a b', CAST(1.25 AS NUMERIC(3, 2)), CAST(1.5E+20 AS DECFLOAT), \
             CAST(X'0aff' AS VARBINARY(2)), CAST(X'' AS VARBINARY(1)), DATE '2022-01-02', \
             TIME '15:04:05+05:30', TIMESTAMP '2022-01-02 15:04:05.123', \
             INTERVAL '1 02' DAY TO HOUR)",
            (),
            |row| Ok(row.clone()),
        )
        .unwrap();
    let got: Vec<String> = row.into_values().iter().map(Value::to_string).collect();
    assert_eq!(
        got,
        [
            "NULL",
            "TRUE",
            "FALSE",
            "-12",
            "1.5",
            "a b",
            "1.25",
            "1.5E+20",
            "X'0AFF'",
            "X''",
            "2022-01-02",
            "15:04:05.000000+05:30",
            "2022-01-02 15:04:05.123000",
            "INTERVAL '1 02' DAY(2) TO HOUR",
        ]
    );
}

#[test]
fn to_sql_types() {
    let conn = memory();
    let get = |v: &dyn ToSql| -> Value {
        conn.query_row("VALUES CAST(? AS VARCHAR(10))", [v], |r| r.get(0))
            .unwrap()
    };
    let text = |s: &str| Value::Text(s.to_string());
    assert_eq!(get(&Cow::Borrowed("a")), text("a"));
    assert_eq!(get(&Cow::<str>::Owned("b".to_string())), text("b"));
    assert_eq!(get(&Rc::<str>::from("c")), text("c"));
    assert_eq!(get(&Arc::<str>::from("d")), text("d"));
    assert_eq!(get(&Box::<str>::from("e")), text("e"));
    assert_eq!(get(&'f'), text("f"));
    assert_eq!(get(&'é'), text("é"));

    let binary = |v: &dyn ToSql| -> Vec<u8> {
        conn.query_row("VALUES CAST(? AS VARBINARY(10))", [v], |r| r.get(0))
            .unwrap()
    };
    assert_eq!(binary(&Cow::Borrowed(&[1u8, 2][..])), [1, 2]);
    assert_eq!(binary(&Rc::<[u8]>::from(&[3u8][..])), [3]);
    assert_eq!(binary(&Arc::<[u8]>::from(&[4u8][..])), [4]);
}

#[test]
fn from_sql_types() {
    let conn = memory();
    let row = conn
        .query_row(
            "VALUES ROW('abc', 'é', CAST(X'0102' AS VARBINARY(2)), 1)",
            (),
            |row| Ok(row.clone()),
        )
        .unwrap();
    assert_eq!(&*row.get::<_, Box<str>>(0).unwrap(), "abc");
    assert_eq!(&*row.get::<_, Rc<str>>(0).unwrap(), "abc");
    assert_eq!(&*row.get::<_, Arc<str>>(0).unwrap(), "abc");
    assert_eq!(row.get::<_, Cow<str>>(0).unwrap(), "abc");
    assert_eq!(row.get::<_, char>(1).unwrap(), 'é');
    assert_eq!(&*row.get::<_, Box<[u8]>>(2).unwrap(), [1, 2]);
    assert_eq!(&*row.get::<_, Rc<[u8]>>(2).unwrap(), [1, 2]);
    assert_eq!(&*row.get::<_, Arc<[u8]>>(2).unwrap(), [1, 2]);
    assert_eq!(&*row.get::<_, Cow<[u8]>>(2).unwrap(), [1, 2]);

    let err = row.get::<_, char>(0).unwrap_err();
    assert!(
        err.to_string().ends_with("\"abc\" isn't one character"),
        "{err}"
    );
    assert!(matches!(
        row.get::<_, char>(3),
        Err(Error::FromSqlConversion { index: 3, .. })
    ));
    assert!(matches!(
        char::from_sql(&Value::Text(String::new())),
        Err(vsql::types::FromSqlError::Other(_))
    ));
    assert!(row.get::<_, Rc<str>>(2).is_err());
    assert!(row.get::<_, Rc<[u8]>>(0).is_err());
}

#[test]
fn row_index_types() {
    let conn = memory();
    let row = conn
        .query_row("VALUES ROW(1, 2)", (), |row| Ok(row.clone()))
        .unwrap();
    let name = "col2".to_string();
    assert_eq!(row.get::<_, i64>(&name).unwrap(), 2);
    assert_eq!(row.get::<_, i64>(name.clone()).unwrap(), 2);
    assert_eq!(row.get::<_, i64>(name.as_str()).unwrap(), 2);
    assert_eq!(row.get::<_, i64>(&&*name).unwrap(), 2);
    assert_eq!(row.get::<_, i64>(&0).unwrap(), 1);
    assert_eq!(row.get_ref(&name).unwrap(), &Value::Integer(2));
    assert!(matches!(
        row.get::<_, i64>("nope".to_string()),
        Err(Error::InvalidColumnName(name)) if name == "nope"
    ));
}

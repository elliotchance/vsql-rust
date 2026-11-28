// Errors and other cases that the main tests (vsql.rs) don't need.

use vsql::{Connection, Error, Type, Value};

fn memory() -> Connection {
    Connection::open_in_memory().unwrap()
}

#[test]
fn open_errors() {
    let path = std::env::temp_dir()
        .join("vsql-rust-no-such-directory")
        .join("x.vsql");
    assert!(Connection::open(path).is_err());
}

#[cfg(unix)]
#[test]
fn paths_that_arent_utf8() {
    use std::ffi::OsStr;
    use std::os::unix::ffi::OsStrExt;

    let path = std::path::Path::new(OsStr::from_bytes(b"a\xff.vsql"));
    assert!(matches!(
        Connection::open(path),
        Err(Error::InvalidPath(p)) if p == path
    ));
}

#[test]
fn debug() {
    let mut conn = memory();
    assert_eq!(format!("{conn:?}"), "Connection { autocommit: true, .. }");
    {
        let mut stmt = conn.prepare("VALUES 1").unwrap();
        assert_eq!(format!("{stmt:?}"), "Statement { .. }");
        let rows = stmt.query(()).unwrap();
        assert!(format!("{rows:?}").starts_with("Rows { columns: [Column { name: \"COL1\""));
    }
    let mut tx = conn.transaction().unwrap();
    let sp = tx.savepoint_with_name("a\"b").unwrap();
    assert_eq!(
        format!("{sp:?}"),
        "Savepoint { name: \"\\\"a\\\"\\\"b\\\"\", .. }"
    );
}

#[test]
fn savepoint_errors() {
    let mut conn = memory();
    let long = "x".repeat(1000);
    assert!(conn.savepoint_with_name(long.as_str()).is_err());
    assert!(conn.is_autocommit());
}

#[test]
fn long_names() {
    let conn = memory();

    // The longest name.
    let table = "T".repeat(128);
    let err = conn
        .execute(&format!("SELECT * FROM \"{table}\""), ())
        .unwrap_err();
    assert_eq!(err.sqlstate(), Some("42P01"));
    assert_eq!(
        err.diagnostics().unwrap().table_name.as_deref(),
        Some(table.as_str())
    );

    // The message is longer than 512 characters.
    let table = "T".repeat(600);
    let err = conn
        .execute(&format!("SELECT * FROM \"{table}\""), ())
        .unwrap_err();
    assert_eq!(err.sqlstate(), Some("42601"));
    assert!(err.to_string().contains(&table), "{err}");

    let column = "C".repeat(100);
    conn.execute(&format!("CREATE TABLE t (\"{column}\" INTEGER)"), ())
        .unwrap();
    conn.execute("INSERT INTO t VALUES (1)", ()).unwrap();
    let name = conn
        .query_row("SELECT * FROM t", (), |r| {
            Ok(r.columns()[0].name().to_string())
        })
        .unwrap();
    assert_eq!(name, column);
}

#[test]
fn statements_rows_and_columns() {
    let conn = memory();
    let stmt = conn.prepare("VALUES 1").unwrap();
    assert!(stmt.connection().is_autocommit());

    let row = conn
        .query_row("VALUES (1, 'a')", (), |r| Ok(r.clone()))
        .unwrap();
    assert_eq!(row.len(), 2);
    assert!(!row.is_empty());
    assert_eq!(row.get_ref(1).unwrap(), &Value::Text("a".to_string()));
    assert_eq!(row.columns()[0].ty(), Type::Integer);
    assert_eq!(row.columns()[0].precision_scale(), None);
    assert!(matches!(row.get_ref(2), Err(Error::InvalidColumnIndex(2))));
    assert_eq!(
        row.into_values(),
        vec![Value::Integer(1), Value::Text("a".to_string())]
    );
}

#[cfg(feature = "chrono")]
#[test]
fn chrono_errors() {
    use chrono::{DateTime, FixedOffset, NaiveDate, NaiveTime, TimeZone, Utc};
    use vsql::types::{FromSqlError, ToSql};
    use vsql::FromSql;

    // An offset that isn't a whole number of minutes.
    let offset = FixedOffset::east_opt(30).unwrap();
    let t = offset.with_ymd_and_hms(2022, 1, 2, 3, 4, 5).unwrap();
    assert!(matches!(t.to_sql(), Err(Error::ToSqlConversion(_))));

    let invalid = |r: Result<_, FromSqlError>| matches!(r, Err(FromSqlError::InvalidType));
    assert!(invalid(NaiveDate::from_sql(&Value::Integer(1)).map(|_| ())));
    assert!(invalid(NaiveTime::from_sql(&Value::Integer(1)).map(|_| ())));
    assert!(invalid(
        DateTime::<FixedOffset>::from_sql(&Value::Integer(1)).map(|_| ())
    ));

    // A TIME WITH TIME ZONE isn't a NaiveTime.
    let conn = memory();
    let err = conn
        .query_row("VALUES TIME '01:02:03+05:30'", (), |r| {
            r.get::<_, NaiveTime>(0)
        })
        .unwrap_err();
    assert!(matches!(err, Error::FromSqlConversion { .. }));

    let utc = Utc.with_ymd_and_hms(2022, 1, 2, 3, 4, 5).unwrap();
    let got: DateTime<Utc> = conn
        .query_row("VALUES CAST(? AS TIMESTAMP WITH TIME ZONE)", [utc], |r| {
            r.get(0)
        })
        .unwrap();
    assert_eq!(got, utc);
}

#[cfg(feature = "rust_decimal")]
#[test]
fn rust_decimal_errors() {
    use rust_decimal::Decimal;
    use vsql::types::FromSqlError;
    use vsql::FromSql;

    assert!(matches!(
        Decimal::from_sql(&Value::Boolean(true)),
        Err(FromSqlError::InvalidType)
    ));
    let other = |value| matches!(Decimal::from_sql(&value), Err(FromSqlError::Other(_)));
    assert!(other(Value::Numeric("x".to_string())));
    assert!(other(Value::DecFloat("x".to_string())));
    assert_eq!(
        Decimal::from_sql(&Value::DecFloat("1.5E+3".to_string())).unwrap(),
        Decimal::from(1500)
    );
}

#[test]
fn rows_fetched_one_at_a_time() {
    // A string that's too wide can't be bound, so the rows are fetched one at
    // a time.
    let conn = memory();
    let row = conn
        .query_row(
            "VALUES (CAST('a' AS VARCHAR(100000)), CAST(NULL AS INTEGER), CAST(1.5 AS REAL), \
             CAST(NULL AS VARCHAR(10)), CAST(2.5 AS DOUBLE PRECISION), X'AB')",
            (),
            |r| Ok(r.clone().into_values()),
        )
        .unwrap();
    assert_eq!(
        row,
        vec![
            Value::Text("a".to_string()),
            Value::Null,
            Value::Double(1.5),
            Value::Null,
            Value::Double(2.5),
            Value::Binary(vec![0xab]),
        ]
    );
}

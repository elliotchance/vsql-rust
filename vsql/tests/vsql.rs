// The tests are ported from vsql-python and vsql-go.

use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;

use vsql::types::FromSqlError;
use vsql::{
    params, params_from_iter, Connection, Date, Error, IsolationLevel, Time, Timestamp,
    TransactionOptions, Type, Value,
};

/// A path for a new database file, which is removed when it's dropped.
struct TempPath(PathBuf);

impl TempPath {
    fn new() -> TempPath {
        static N: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "vsql-rust-test-{}-{}.vsql",
            std::process::id(),
            N.fetch_add(1, Ordering::SeqCst)
        ));
        TempPath(path)
    }
}

impl Drop for TempPath {
    fn drop(&mut self) {
        let name = self.0.file_name().unwrap().to_str().unwrap().to_string();
        if let Ok(entries) = std::fs::read_dir(self.0.parent().unwrap()) {
            for entry in entries.flatten() {
                if entry.file_name().to_str().unwrap_or("").starts_with(&name) {
                    let _ = std::fs::remove_file(entry.path());
                }
            }
        }
    }
}

fn memory() -> Connection {
    Connection::open_in_memory().unwrap()
}

fn sqlstate<T: std::fmt::Debug>(result: vsql::Result<T>) -> String {
    result.unwrap_err().sqlstate().unwrap().to_string()
}

#[test]
fn column_values() {
    let conn = memory();
    let tz = |minutes| Some(minutes);
    let cases: Vec<(&str, Value, Type)> = vec![
        ("CAST(12 AS SMALLINT)", Value::Integer(12), Type::SmallInt),
        ("CAST(-12 AS INTEGER)", Value::Integer(-12), Type::Integer),
        (
            "CAST(9223372036854775807 AS BIGINT)",
            Value::Integer(i64::MAX),
            Type::BigInt,
        ),
        ("CAST(1.5 AS REAL)", Value::Double(1.5), Type::Real),
        (
            "CAST(1.25 AS DOUBLE PRECISION)",
            Value::Double(1.25),
            Type::DoublePrecision,
        ),
        (
            "CAST(1.5 AS DECIMAL(10, 3))",
            Value::Numeric("1.500".into()),
            Type::Decimal,
        ),
        (
            "CAST(12 AS NUMERIC(10, 2))",
            Value::Numeric("12.00".into()),
            Type::Numeric,
        ),
        (
            "CAST('ab' AS CHARACTER(4))",
            Value::Text("ab  ".into()),
            Type::Character,
        ),
        (
            "CAST('héllo' AS VARCHAR(10))",
            Value::Text("héllo".into()),
            Type::CharacterVarying,
        ),
        ("TRUE", Value::Boolean(true), Type::Boolean),
        ("FALSE", Value::Boolean(false), Type::Boolean),
        (
            "DATE '2022-01-02'",
            Value::Date(Date {
                year: 2022,
                month: 1,
                day: 2,
            }),
            Type::Date,
        ),
        (
            "TIME '15:04:05.123456'",
            Value::Time(Time {
                hour: 15,
                minute: 4,
                second: 5,
                microsecond: 123456,
                offset: None,
            }),
            Type::Time,
        ),
        (
            "TIME '15:04:05+05:30'",
            Value::Time(Time {
                hour: 15,
                minute: 4,
                second: 5,
                microsecond: 0,
                offset: tz(330),
            }),
            Type::TimeWithTimeZone,
        ),
        (
            "TIMESTAMP '2022-01-02 15:04:05.5'",
            Value::Timestamp(Timestamp {
                date: Date {
                    year: 2022,
                    month: 1,
                    day: 2,
                },
                time: Time {
                    hour: 15,
                    minute: 4,
                    second: 5,
                    microsecond: 500000,
                    offset: None,
                },
            }),
            Type::Timestamp,
        ),
        (
            "TIMESTAMP '2022-01-02 15:04:05.123-04:00'",
            Value::Timestamp(Timestamp {
                date: Date {
                    year: 2022,
                    month: 1,
                    day: 2,
                },
                time: Time {
                    hour: 15,
                    minute: 4,
                    second: 5,
                    microsecond: 123000,
                    offset: tz(-240),
                },
            }),
            Type::TimestampWithTimeZone,
        ),
        ("CAST(NULL AS INTEGER)", Value::Null, Type::Integer),
        ("CAST(NULL AS BOOLEAN)", Value::Null, Type::Boolean),
        ("CAST(NULL AS DATE)", Value::Null, Type::Date),
    ];

    for (expr, want, ty) in cases {
        let mut stmt = conn.prepare(&format!("VALUES {expr}")).unwrap();
        let mut rows = stmt.query(()).unwrap();
        assert_eq!(rows.columns()[0].ty(), ty, "{expr}");
        let row = rows.next().unwrap().unwrap();
        assert_eq!(row.get::<_, Value>(0).unwrap(), want, "{expr}");
        assert!(rows.next().is_none());
    }
}

/// vsql drops the leading zeros of fractional seconds
/// (https://github.com/elliotchance/vsql/issues/697). This test fails once
/// it's fixed, so the `should_panic` can be removed when the amalgamation is
/// updated.
#[test]
#[should_panic(expected = "vsql#697")]
fn fractional_seconds_with_leading_zeros() {
    let conn = memory();
    let t: Timestamp = conn
        .query_row("VALUES TIMESTAMP '2022-01-02 15:04:05.000006'", (), |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(t.time.microsecond, 6, "vsql#697");
}

#[test]
fn rust_types() {
    let conn = memory();
    let row = |sql: &str| {
        conn.query_row(&format!("VALUES {sql}"), (), |row| Ok(row.clone()))
            .unwrap()
    };

    assert_eq!(row("CAST(12 AS SMALLINT)").get::<_, i16>(0).unwrap(), 12);
    assert_eq!(row("CAST(12 AS SMALLINT)").get::<_, u8>(0).unwrap(), 12);
    assert_eq!(row("CAST(1.5 AS REAL)").get::<_, f32>(0).unwrap(), 1.5);
    assert_eq!(
        row("CAST(1.5 AS NUMERIC(5, 2))").get::<_, f64>(0).unwrap(),
        1.5
    );
    assert_eq!(
        row("CAST(1.5 AS NUMERIC(5, 2))")
            .get::<_, String>(0)
            .unwrap(),
        "1.50"
    );
    assert_eq!(
        row("CAST(NULL AS INTEGER)")
            .get::<_, Option<i64>>(0)
            .unwrap(),
        None
    );
    assert_eq!(
        row("CAST(3 AS INTEGER)").get::<_, Option<i64>>(0).unwrap(),
        Some(3)
    );

    // Out of range, the wrong type, and NULL without an Option.
    match row("CAST(100000 AS INTEGER)").get::<_, i16>(0) {
        Err(Error::FromSqlConversion {
            source: FromSqlError::OutOfRange,
            ty: Type::Integer,
            ..
        }) => {}
        other => panic!("{other:?}"),
    }
    match row("'a'").get::<_, i64>(0) {
        Err(Error::FromSqlConversion {
            source: FromSqlError::InvalidType,
            ..
        }) => {}
        other => panic!("{other:?}"),
    }
    match row("CAST(NULL AS INTEGER)").get::<_, i64>(0) {
        Err(Error::FromSqlConversion {
            source: FromSqlError::InvalidType,
            ..
        }) => {}
        other => panic!("{other:?}"),
    }
}

#[test]
fn columns() {
    let conn = memory();
    conn.execute(
        "CREATE TABLE foo (a INTEGER NOT NULL, b VARCHAR(20), c NUMERIC(10, 3))",
        (),
    )
    .unwrap();

    let mut stmt = conn.prepare("SELECT * FROM foo").unwrap();
    let rows = stmt.query(()).unwrap();
    assert_eq!(rows.column_names(), ["A", "B", "C"]);
    let columns = rows.columns();
    assert!(!columns[0].is_nullable());
    assert!(columns[1].is_nullable());
    assert_eq!(columns[1].length(), Some(20));
    assert_eq!(columns[0].length(), None);
    assert_eq!(columns[2].precision_scale(), Some((10, 3)));
    assert_eq!(columns[2].ty().to_string(), "NUMERIC");
}

#[test]
fn column_names_and_indexes() {
    let conn = memory();
    conn.query_row("VALUES ROW(1, 2)", (), |row| {
        assert_eq!(row.get::<_, i64>("COL2")?, 2);
        assert_eq!(row.get::<_, i64>("col2")?, 2);
        assert!(matches!(
            row.get::<_, i64>("nope"),
            Err(Error::InvalidColumnName(_))
        ));
        assert!(matches!(
            row.get::<_, i64>(2),
            Err(Error::InvalidColumnIndex(2))
        ));
        Ok(())
    })
    .unwrap();
}

#[test]
fn parameters() {
    let conn = memory();
    conn.execute(
        "CREATE TABLE foo (
            s SMALLINT, i INTEGER, b BIGINT, r REAL, d DOUBLE PRECISION,
            n NUMERIC(30, 5), c VARCHAR(100), bo BOOLEAN,
            ts TIMESTAMP(6), tz TIMESTAMP(6) WITH TIME ZONE, dt DATE,
            tm TIME(6), tt TIME(6) WITH TIME ZONE)",
        (),
    )
    .unwrap();

    let date = Date {
        year: 2022,
        month: 1,
        day: 2,
    };
    let time = Time {
        hour: 15,
        minute: 4,
        second: 5,
        microsecond: 123456,
        offset: None,
    };
    let time_tz = Time {
        offset: Some(-240),
        ..time
    };
    let check = |column: &str, param: &dyn vsql::ToSql, want: Value| {
        conn.execute("DELETE FROM foo", ()).unwrap();
        conn.execute(
            &format!("INSERT INTO foo ({column}) VALUES (?)"),
            params![param],
        )
        .unwrap_or_else(|e| panic!("{column}: {e}"));
        let got: Value = conn
            .query_row(&format!("SELECT {column} FROM foo"), (), |row| row.get(0))
            .unwrap();
        assert_eq!(got, want, "{column}");
    };

    check("s", &12, Value::Integer(12));
    check("i", &-12i32, Value::Integer(-12));
    check("b", &i64::MAX, Value::Integer(i64::MAX));
    check("r", &1.5f32, Value::Double(1.5));
    check("d", &1.25, Value::Double(1.25));
    check("n", &7, Value::Numeric("7.00000".into()));
    check("n", &1.5, Value::Numeric("1.50000".into()));
    check(
        "n",
        &Value::Numeric("-1.25".into()),
        Value::Numeric("-1.25000".into()),
    );
    // vsql counts a digit before the decimal point (vsql-go#3).
    check(
        "n",
        &Value::Numeric("0.25".into()),
        Value::Numeric("0.25000".into()),
    );
    check(
        "n",
        &Value::Numeric("-0.00005".into()),
        Value::Numeric("-0.00005".into()),
    );
    check(
        "n",
        &Value::Numeric("0".into()),
        Value::Numeric("0.00000".into()),
    );
    check(
        "n",
        &u64::MAX,
        Value::Numeric("18446744073709551615.00000".into()),
    );
    check("c", &"héllo", Value::Text("héllo".into()));
    check("c", &"", Value::Text("".into()));
    check("c", &String::from("owned"), Value::Text("owned".into()));
    check("bo", &true, Value::Boolean(true));
    check("bo", &false, Value::Boolean(false));
    check("dt", &date, Value::Date(date));
    check("tm", &time, Value::Time(time));
    check("tt", &time_tz, Value::Time(time_tz));
    check(
        "ts",
        &Timestamp { date, time },
        Value::Timestamp(Timestamp { date, time }),
    );
    check(
        "tz",
        &Timestamp {
            date,
            time: time_tz,
        },
        Value::Timestamp(Timestamp {
            date,
            time: time_tz,
        }),
    );
    check("i", &None::<i64>, Value::Null);
    check("c", &None::<&str>, Value::Null);
    check("bo", &Value::Null, Value::Null);
    check("dt", &None::<Date>, Value::Null);
}

#[test]
// Each form of Params, including the borrowed ones.
#[allow(clippy::needless_borrows_for_generic_args)]
fn params_forms() {
    let conn = memory();
    let sum = |sql: &str, f: &dyn Fn(&str) -> vsql::Result<i64>| f(sql).unwrap();
    let sql = "VALUES CAST(? AS INTEGER) + CAST(? AS INTEGER)";

    assert_eq!(sum(sql, &|s| conn.query_row(s, (1, 2), |r| r.get(0))), 3);
    assert_eq!(sum(sql, &|s| conn.query_row(s, [1, 2], |r| r.get(0))), 3);
    assert_eq!(sum(sql, &|s| conn.query_row(s, &[1, 2], |r| r.get(0))), 3);
    assert_eq!(
        sum(sql, &|s| conn.query_row(s, &vec![1, 2], |r| r.get(0))),
        3
    );
    assert_eq!(
        sum(sql, &|s| conn.query_row(s, &[1, 2][..], |r| r.get(0))),
        3
    );
    assert_eq!(
        sum(sql, &|s| conn.query_row(s, params![1, 2i8], |r| r.get(0))),
        3
    );
    assert_eq!(
        sum(sql, &|s| conn.query_row(
            s,
            params_from_iter(vec![1, 2]),
            |r| r.get(0)
        )),
        3
    );
}

#[test]
fn parameter_errors() {
    let conn = memory();
    let sql = "VALUES CAST(? AS INTEGER)";

    // Too few parameters.
    assert_eq!(
        sqlstate(conn.query_row(sql, (), |r| r.get::<_, i64>(0))),
        "07001"
    );

    // A NUMERIC that isn't a decimal.
    match conn.query_row(sql, [Value::Numeric("1e5".into())], |r| r.get::<_, i64>(0)) {
        Err(Error::InvalidParameter { index: 1, .. }) => {}
        other => panic!("{other:?}"),
    }
}

#[test]
fn long_strings() {
    let conn = memory();
    conn.execute("CREATE TABLE foo (s VARCHAR(10000))", ())
        .unwrap();
    for n in [255, 256, 257, 1000, 9999] {
        // Multi-byte characters cross the boundary of each piece that
        // SQLGetData reads.
        let want = "é".repeat(n / 2) + &"x".repeat(n % 2);
        conn.execute("DELETE FROM foo", ()).unwrap();
        conn.execute("INSERT INTO foo (s) VALUES (?)", [&want])
            .unwrap();
        let got: String = conn
            .query_row("SELECT s FROM foo", (), |r| r.get(0))
            .unwrap();
        assert_eq!(got, want, "{n}");
    }
}

#[test]
fn errors() {
    let conn = memory();
    for (sql, want) in [
        ("SELECT * FROM nope", "42P01"),
        ("SELEC 1", "42601"),
        // Transactions are started with Connection::transaction.
        ("COMMIT", "42000"),
        ("VALUES 1 / 0", "22012"),
    ] {
        let err = conn.execute(sql, ()).unwrap_err();
        assert_eq!(err.sqlstate(), Some(want), "{sql}");
        assert!(
            err.to_string().ends_with(&format!("(SQLSTATE {want})")),
            "{err}"
        );

        // The connection still works after an error.
        assert_eq!(
            conn.query_row("VALUES 1", (), |r| r.get::<_, i64>(0))
                .unwrap(),
            1
        );
    }

    assert_eq!(sqlstate(conn.prepare("SELEC 1")), "42601");

    conn.execute("CREATE TABLE foo (x INTEGER NOT NULL)", ())
        .unwrap();
    let err = conn
        .execute("INSERT INTO foo (x) VALUES (NULL)", ())
        .unwrap_err();
    assert!(err.sqlstate().unwrap().starts_with("23"), "{err}");
}

#[test]
fn query_returned_no_rows() {
    let conn = memory();
    conn.execute("CREATE TABLE foo (x INTEGER)", ()).unwrap();
    let result = conn.query_row("SELECT x FROM foo", (), |r| r.get::<_, i64>(0));
    assert!(matches!(result, Err(Error::QueryReturnedNoRows)));
}

#[test]
fn rows_changed() {
    let conn = memory();
    conn.execute("CREATE TABLE foo (x INTEGER)", ()).unwrap();
    assert_eq!(
        conn.execute("INSERT INTO foo (x) VALUES (1)", ()).unwrap(),
        1
    );
    assert_eq!(
        conn.execute("INSERT INTO foo (x) VALUES (?)", [2]).unwrap(),
        1
    );
    assert_eq!(conn.execute("UPDATE foo SET x = x + 1", ()).unwrap(), 2);

    // A query's cursor is closed, so the table can be dropped.
    conn.execute("SELECT * FROM foo", ()).unwrap();
    conn.execute("SELECT * FROM foo WHERE x = ?", [2]).unwrap();
    conn.execute("DROP TABLE foo", ()).unwrap();
}

#[test]
fn prepared_statements() {
    let conn = memory();
    conn.execute("CREATE TABLE foo (x INTEGER)", ()).unwrap();
    let mut insert = conn.prepare("INSERT INTO foo (x) VALUES (?)").unwrap();
    for i in 1..=10 {
        assert_eq!(insert.execute([i]).unwrap(), 1);
    }

    // Each statement has its own cursor, so the rows of one are still open
    // while another is executed.
    let mut query1 = conn.prepare("SELECT x FROM foo WHERE x > ?").unwrap();
    let mut query2 = conn.prepare("SELECT x FROM foo WHERE x > ?").unwrap();
    let rows1 = query1.query([5]).unwrap();
    let rows2 = query2.query([8]).unwrap();
    assert_eq!(rows2.count(), 2);
    assert_eq!(rows1.count(), 5);

    // The statement can be executed again after its rows are dropped, and with
    // fewer parameters it's an error rather than reading the old ones
    // (vsql-go#4).
    let mut two = conn
        .prepare("VALUES ROW(CAST(? AS VARCHAR(10)), CAST(? AS VARCHAR(10)))")
        .unwrap();
    let row = two.query_row(("a", "b"), |r| Ok(r.clone())).unwrap();
    assert_eq!(row.get::<_, String>(1).unwrap(), "b");
    assert_eq!(sqlstate(two.query_row(["a"], |r| Ok(r.clone()))), "07001");
}

#[test]
fn query_map() {
    let conn = memory();
    let mut stmt = conn.prepare("VALUES 1, 2, 3").unwrap();
    let xs: Vec<i64> = stmt
        .query_map((), |row| row.get(0))
        .unwrap()
        .collect::<vsql::Result<_>>()
        .unwrap();
    assert_eq!(xs, [1, 2, 3]);

    // Rows that aren't all read close their cursor when they're dropped.
    let first = stmt.query(()).unwrap().next().unwrap().unwrap();
    assert_eq!(first.get::<_, i64>(0).unwrap(), 1);
    assert_eq!(stmt.query(()).unwrap().count(), 3);
}

#[test]
fn transactions() {
    let path = TempPath::new();
    let count = || -> i64 {
        Connection::open(&path.0)
            .unwrap()
            .query_row("SELECT COUNT(*) FROM foo", (), |r| r.get(0))
            .unwrap()
    };

    let mut conn = Connection::open(&path.0).unwrap();
    conn.execute("CREATE TABLE foo (x INTEGER)", ()).unwrap();
    assert!(conn.is_autocommit());

    // Autocommit: another connection sees it.
    conn.execute("INSERT INTO foo (x) VALUES (1)", ()).unwrap();
    assert_eq!(count(), 1);

    // Commit.
    let tx = conn.transaction().unwrap();
    assert!(!tx.is_autocommit());
    tx.execute("INSERT INTO foo (x) VALUES (2)", ()).unwrap();
    assert_eq!(count(), 1);
    tx.commit().unwrap();
    assert!(conn.is_autocommit());
    assert_eq!(count(), 2);

    // Rollback.
    let tx = conn.transaction().unwrap();
    tx.execute("INSERT INTO foo (x) VALUES (3)", ()).unwrap();
    let n: i64 = tx
        .query_row("SELECT COUNT(*) FROM foo", (), |r| r.get(0))
        .unwrap();
    assert_eq!(n, 3);
    tx.rollback().unwrap();
    assert_eq!(count(), 2);

    // Dropping a transaction rolls it back.
    {
        let tx = conn.transaction().unwrap();
        tx.execute("INSERT INTO foo (x) VALUES (4)", ()).unwrap();
    }
    assert!(conn.is_autocommit());
    assert_eq!(count(), 2);

    // Dropping (or closing) a connection rolls back its transaction.
    let tx = conn.transaction().unwrap();
    tx.execute("INSERT INTO foo (x) VALUES (5)", ()).unwrap();
    std::mem::forget(tx);
    conn.close().unwrap();
    assert_eq!(count(), 2);
}

#[test]
fn isolation_levels() {
    let mut conn = memory();
    for level in [
        IsolationLevel::ReadUncommitted,
        IsolationLevel::ReadCommitted,
    ] {
        let options = TransactionOptions::default().isolation_level(level);
        conn.transaction_with(options).unwrap().rollback().unwrap();
    }
    for options in [
        TransactionOptions::default().isolation_level(IsolationLevel::RepeatableRead),
        TransactionOptions::default().isolation_level(IsolationLevel::Serializable),
        TransactionOptions::default().read_only(true),
    ] {
        assert_eq!(
            sqlstate(conn.transaction_with(options).map(|_| ())),
            "HYC00"
        );
        assert!(conn.is_autocommit());
    }
}

#[test]
fn persistence() {
    let path = TempPath::new();
    {
        let conn = Connection::open(&path.0).unwrap();
        conn.execute("CREATE TABLE foo (x VARCHAR(10))", ())
            .unwrap();
        conn.execute("INSERT INTO foo (x) VALUES ('saved')", ())
            .unwrap();
    }

    let conn = Connection::open(&path.0).unwrap();
    let x: String = conn
        .query_row("SELECT x FROM foo", (), |r| r.get(0))
        .unwrap();
    assert_eq!(x, "saved");
}

#[test]
fn memory_databases_are_separate() {
    let a = memory();
    let b = memory();
    a.execute("CREATE TABLE foo (x INTEGER)", ()).unwrap();
    assert_eq!(sqlstate(b.execute("SELECT * FROM foo", ())), "42P01");
}

/// Many connections from many threads, with enough work for the garbage
/// collector to run. A connection is moved to its thread.
#[test]
fn concurrency() {
    let path = TempPath::new();
    Connection::open(&path.0)
        .unwrap()
        .execute("CREATE TABLE foo (g INTEGER, x INTEGER)", ())
        .unwrap();

    let (threads, inserts) = (8, 50);
    let handles: Vec<_> = (0..threads)
        .map(|g| {
            let conn = Connection::open(&path.0).unwrap();
            thread::spawn(move || {
                for i in 0..inserts {
                    conn.execute("INSERT INTO foo (g, x) VALUES (?, ?)", (g, i))
                        .unwrap();
                    let n: i64 = conn
                        .query_row("SELECT COUNT(*) FROM foo WHERE g = ?", [g], |r| r.get(0))
                        .unwrap();
                    assert_eq!(n, i + 1, "thread {g}");
                }
            })
        })
        .collect();
    for handle in handles {
        handle.join().unwrap();
    }

    let n: i64 = Connection::open(&path.0)
        .unwrap()
        .query_row("SELECT COUNT(*) FROM foo", (), |r| r.get(0))
        .unwrap();
    assert_eq!(n, threads * inserts);
}

#[test]
fn connection_is_send() {
    fn assert_send<T: Send>() {}
    assert_send::<Connection>();
}

#[cfg(feature = "chrono")]
#[test]
fn chrono() {
    use chrono::{DateTime, FixedOffset, NaiveDate, NaiveDateTime, NaiveTime, TimeZone, Utc};

    let conn = memory();
    let date = NaiveDate::from_ymd_opt(2022, 1, 2).unwrap();
    let time = NaiveTime::from_hms_micro_opt(15, 4, 5, 123456).unwrap();
    let naive = date.and_time(time);
    let offset = FixedOffset::east_opt(-4 * 3600).unwrap();
    let aware = offset.from_local_datetime(&naive).unwrap();

    let got: NaiveDate = conn
        .query_row("VALUES CAST(? AS DATE)", [date], |r| r.get(0))
        .unwrap();
    assert_eq!(got, date);
    let got: NaiveTime = conn
        .query_row("VALUES CAST(? AS TIME(6))", [time], |r| r.get(0))
        .unwrap();
    assert_eq!(got, time);
    let got: NaiveDateTime = conn
        .query_row("VALUES CAST(? AS TIMESTAMP(6))", [naive], |r| r.get(0))
        .unwrap();
    assert_eq!(got, naive);
    let got: DateTime<FixedOffset> = conn
        .query_row(
            "VALUES CAST(? AS TIMESTAMP(6) WITH TIME ZONE)",
            [aware],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(got, aware);
    assert_eq!(got.offset(), &offset);
    let got: DateTime<Utc> = conn
        .query_row(
            "VALUES CAST(? AS TIMESTAMP(6) WITH TIME ZONE)",
            [aware],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(got, aware.with_timezone(&Utc));

    // A TIMESTAMP WITH TIME ZONE isn't a NaiveDateTime (the offset would be
    // lost), and a TIMESTAMP isn't a DateTime.
    let row = conn
        .query_row(
            "VALUES ROW(TIMESTAMP '2022-01-02 15:04:05+05:30', TIMESTAMP '2022-01-02 15:04:05')",
            (),
            |r| Ok(r.clone()),
        )
        .unwrap();
    assert!(row.get::<_, NaiveDateTime>(0).is_err());
    assert!(row.get::<_, DateTime<FixedOffset>>(1).is_err());
}

#[cfg(feature = "rust_decimal")]
#[test]
fn rust_decimal() {
    use std::str::FromStr;

    use rust_decimal::Decimal;

    let conn = memory();
    for s in ["1.25", "-0.05", "0", "123456789012345.12345", "100"] {
        let d = Decimal::from_str(s).unwrap();
        let got: Decimal = conn
            .query_row("VALUES CAST(? AS NUMERIC(30, 5))", [d], |r| r.get(0))
            .unwrap();
        assert_eq!(got, d, "{s}");
    }
    let got: Decimal = conn.query_row("VALUES 12", (), |r| r.get(0)).unwrap();
    assert_eq!(got, Decimal::from(12));
}

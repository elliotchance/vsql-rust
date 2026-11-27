// The tests are ported from vsql-python and vsql-go.

use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;

use vsql::types::FromSqlError;
use vsql::FromSql;
use vsql::{
    params, params_from_iter, Connection, Date, Error, Interval, IntervalFields, IsolationLevel,
    Time, Timestamp, TransactionOptions, Type, Value,
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

/// https://github.com/elliotchance/vsql/issues/697
#[test]
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
fn integers_from_decimals() {
    let conn = memory();
    conn.execute("CREATE TABLE foo (x NUMERIC(40))", ())
        .unwrap();
    let round_trip = |sql_value: &dyn vsql::ToSql| -> Value {
        conn.execute("DELETE FROM foo", ()).unwrap();
        conn.execute("INSERT INTO foo (x) VALUES (?)", [sql_value])
            .unwrap();
        conn.query_row("SELECT x FROM foo", (), |r| r.get(0))
            .unwrap()
    };

    // Integers that don't fit in a BIGINT are a NUMERIC, and can be read
    // back.
    let value = round_trip(&u64::MAX);
    assert_eq!(u64::from_sql(&value).unwrap(), u64::MAX);
    assert_eq!(i128::from_sql(&value).unwrap(), i128::from(u64::MAX));
    assert!(matches!(
        i64::from_sql(&value),
        Err(FromSqlError::OutOfRange)
    ));
    assert_eq!(i128::from_sql(&round_trip(&i128::MIN)).unwrap(), i128::MIN);
    assert_eq!(u128::from_sql(&round_trip(&u128::MAX)).unwrap(), u128::MAX);
    assert!(matches!(
        u64::from_sql(&round_trip(&-1)),
        Err(FromSqlError::OutOfRange)
    ));

    for (sql, want) in [
        ("VALUES CAST(5 AS NUMERIC(10))", 5),
        ("VALUES CAST(-5.00 AS DECIMAL(10, 2))", -5),
        ("VALUES CAST(1.5E+3 AS DECFLOAT)", 1500),
        ("SELECT AVG(x) FROM (VALUES 2, 4) AS t (x)", 3),
    ] {
        let got: i64 = conn
            .query_row(sql, (), |r| r.get(0))
            .unwrap_or_else(|e| panic!("{sql}: {e}"));
        assert_eq!(got, want, "{sql}");
    }

    let err = conn
        .query_row("VALUES CAST(5.25 AS NUMERIC(10, 2))", (), |r| {
            r.get::<_, i64>(0)
        })
        .unwrap_err();
    assert_eq!(
        err.to_string(),
        "column 0 (COL1, NUMERIC): 5.25 isn't an integer"
    );
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
fn column_names_that_only_differ_in_case() {
    let conn = memory();
    conn.query_row(
        r#"SELECT * FROM (VALUES ROW(1, 2, 3)) AS t ("a", "A", "b")"#,
        (),
        |row| {
            // An exact match is used before one that ignores case.
            assert_eq!(row.get::<_, i64>("a")?, 1);
            assert_eq!(row.get::<_, i64>("A")?, 2);
            assert_eq!(row.get::<_, i64>("B")?, 3);
            Ok(())
        },
    )
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
    assert!(matches!(
        conn.query_row(sql, (), |r| r.get::<_, i64>(0)),
        Err(Error::InvalidParameterCount(0, 1))
    ));

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
fn strings_with_nul_characters() {
    let conn = memory();
    for n in [5, 255, 256, 257, 1000] {
        // A NUL character before, at and after the end of the first piece
        // that SQLGetData reads.
        for at in [0, n / 2, n - 1] {
            let mut want = "a".repeat(n);
            want.replace_range(at..at + 1, "\0");
            let got: String = conn
                .query_row("VALUES CAST(? AS VARCHAR(2000))", [&want], |r| r.get(0))
                .unwrap();
            assert_eq!(got, want, "{n} {at}");
        }
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
fn comments() {
    let conn = memory();
    conn.execute(
        "-- The table.
        CREATE TABLE foo (
            x INTEGER, -- The number.
            /* -- not a comment in a comment */ y VARCHAR(10)
        ) -- The end.",
        (),
    )
    .unwrap();
    conn.execute("INSERT INTO foo (x, y) VALUES (?, '--') -- one", [1])
        .unwrap();
    let mut stmt = conn
        .prepare("SELECT y /* y */ FROM foo -- all\nWHERE x = ?")
        .unwrap();
    let y: String = stmt.query_row([1], |r| r.get(0)).unwrap();
    assert_eq!(y, "--");

    // The line of an error is the same.
    let err = conn.execute("-- x\nSELEC 1", ()).unwrap_err();
    assert!(err.to_string().contains("line 2"), "{err}");
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
fn query_on_a_statement_that_isnt_a_query() {
    let conn = memory();
    conn.execute("CREATE TABLE foo (x INTEGER)", ()).unwrap();

    let mut stmt = conn.prepare("INSERT INTO foo (x) VALUES (?)").unwrap();
    let mut rows = stmt.query([1]).unwrap();
    assert!(rows.columns().is_empty());
    assert!(rows.next().is_none());
    drop(rows);
    let result = stmt.query_row([2], |r| r.get::<_, i64>(0));
    assert!(matches!(result, Err(Error::QueryReturnedNoRows)));

    // Both were inserted (and committed).
    let n: i64 = conn
        .query_row("SELECT COUNT(*) FROM foo", (), |r| r.get(0))
        .unwrap();
    assert_eq!(n, 2);
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
    assert!(matches!(
        two.query_row(["a"], |r| Ok(r.clone())),
        Err(Error::InvalidParameterCount(1, 2))
    ));
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
    conn.execute("CREATE TABLE foo (x INTEGER)", ()).unwrap();
    let insert = "INSERT INTO foo (x) VALUES (1)";
    let level = |level| TransactionOptions::default().isolation_level(level);
    for (options, read_only) in [
        (level(IsolationLevel::ReadUncommitted), true),
        (level(IsolationLevel::ReadCommitted), false),
        (level(IsolationLevel::RepeatableRead), false),
        (level(IsolationLevel::Serializable), false),
        (TransactionOptions::default().read_only(true), true),
    ] {
        let tx = conn.transaction_with(options).unwrap();
        let result = tx.execute(insert, ());
        if read_only {
            assert_eq!(sqlstate(result), "25006", "{options:?}");
        } else {
            assert_eq!(result.unwrap(), 1, "{options:?}");
        }
        tx.rollback().unwrap();
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

fn interval(fields: IntervalFields, months: i64, microseconds: i64) -> Interval {
    Interval {
        fields,
        months,
        microseconds,
    }
}

#[test]
fn binary_decfloat_clob_and_interval_columns() {
    use IntervalFields::*;

    let conn = memory();
    let cases: Vec<(&str, Value, Type)> = vec![
        ("X'00ff'", Value::Binary(vec![0, 255]), Type::Binary),
        (
            "CAST(X'01' AS BINARY VARYING(5))",
            Value::Binary(vec![1]),
            Type::BinaryVarying,
        ),
        (
            "CAST(X'01' AS BLOB)",
            Value::Binary(vec![1]),
            Type::BinaryLargeObject,
        ),
        ("CAST(NULL AS BLOB)", Value::Null, Type::BinaryLargeObject),
        (
            "CAST('ab' AS CLOB)",
            Value::Text("ab".into()),
            Type::CharacterLargeObject,
        ),
        (
            "CAST(1.5E20 AS DECFLOAT)",
            Value::DecFloat("1.5E+20".into()),
            Type::DecFloat,
        ),
        (
            "CAST(-0.25 AS DECFLOAT)",
            Value::DecFloat("-0.25".into()),
            Type::DecFloat,
        ),
        (
            "INTERVAL '-1-2' YEAR TO MONTH",
            Value::Interval(interval(YearToMonth, -14, 0)),
            Type::Interval(YearToMonth),
        ),
        (
            "INTERVAL '1 2:3:4.5' DAY TO SECOND",
            Value::Interval(interval(DayToSecond, 0, 93_784_500_000)),
            Type::Interval(DayToSecond),
        ),
        (
            "CAST(NULL AS INTERVAL DAY)",
            Value::Null,
            Type::Interval(Day),
        ),
    ];

    for (expr, want, ty) in cases {
        let mut stmt = conn.prepare(&format!("VALUES {expr}")).unwrap();
        let mut rows = stmt.query(()).unwrap();
        assert_eq!(rows.columns()[0].ty(), ty, "{expr}");
        let row = rows.next().unwrap().unwrap();
        assert_eq!(row.get::<_, Value>(0).unwrap(), want, "{expr}");
    }
}

#[test]
fn intervals() {
    use IntervalFields::*;

    let conn = memory();
    let cases = [
        (
            "INTERVAL '5' YEAR",
            interval(Year, 60, 0),
            "INTERVAL '5' YEAR(2)",
        ),
        (
            "INTERVAL '100' YEAR(3)",
            interval(Year, 1200, 0),
            "INTERVAL '100' YEAR(3)",
        ),
        (
            "INTERVAL '7' MONTH",
            interval(Month, 7, 0),
            "INTERVAL '7' MONTH(2)",
        ),
        (
            "INTERVAL '3' DAY",
            interval(Day, 0, 259_200_000_000),
            "INTERVAL '3' DAY(2)",
        ),
        (
            "INTERVAL '3' HOUR",
            interval(Hour, 0, 10_800_000_000),
            "INTERVAL '3' HOUR(2)",
        ),
        (
            "INTERVAL '3' MINUTE",
            interval(Minute, 0, 180_000_000),
            "INTERVAL '3' MINUTE(2)",
        ),
        (
            "INTERVAL '3.25' SECOND",
            interval(Second, 0, 3_250_000),
            "INTERVAL '3.250000' SECOND(2, 6)",
        ),
        (
            "INTERVAL '-1-2' YEAR TO MONTH",
            interval(YearToMonth, -14, 0),
            "INTERVAL '-1-02' YEAR(2) TO MONTH",
        ),
        (
            "INTERVAL '1 2' DAY TO HOUR",
            interval(DayToHour, 0, 93_600_000_000),
            "INTERVAL '1 02' DAY(2) TO HOUR",
        ),
        (
            "INTERVAL '1 2:3' DAY TO MINUTE",
            interval(DayToMinute, 0, 93_780_000_000),
            "INTERVAL '1 02:03' DAY(2) TO MINUTE",
        ),
        (
            "INTERVAL '-1 2:3:4.5' DAY TO SECOND",
            interval(DayToSecond, 0, -93_784_500_000),
            "INTERVAL '-1 02:03:04.500000' DAY(2) TO SECOND(6)",
        ),
        (
            "INTERVAL '2:3' HOUR TO MINUTE",
            interval(HourToMinute, 0, 7_380_000_000),
            "INTERVAL '2:03' HOUR(2) TO MINUTE",
        ),
        (
            "INTERVAL '2:3:4' HOUR TO SECOND",
            interval(HourToSecond, 0, 7_384_000_000),
            "INTERVAL '2:03:04.000000' HOUR(2) TO SECOND(6)",
        ),
        (
            "INTERVAL '3:4.000001' MINUTE TO SECOND",
            interval(MinuteToSecond, 0, 184_000_001),
            "INTERVAL '3:04.000001' MINUTE(2) TO SECOND(6)",
        ),
        (
            "INTERVAL '-0:00.5' MINUTE TO SECOND",
            interval(MinuteToSecond, 0, -500_000),
            "INTERVAL '-0:00.500000' MINUTE(2) TO SECOND(6)",
        ),
    ];

    for (sql, want, literal) in cases {
        let row = conn
            .query_row(&format!("VALUES {sql}"), (), |row| Ok(row.clone()))
            .unwrap();
        assert_eq!(row.columns()[0].ty(), Type::Interval(want.fields));
        assert_eq!(row.get::<_, Interval>(0).unwrap(), want, "{sql}");

        // The literal is the same as vsql's.
        assert_eq!(want.to_string(), literal);
        let sql = format!("VALUES CAST({sql} AS VARCHAR(100))");
        let got: String = conn.query_row(&sql, (), |row| row.get(0)).unwrap();
        assert_eq!(got, literal);

        // As a parameter.
        let (_, qualifier) = literal.split_once("' ").unwrap();
        let sql = format!("VALUES CAST(? AS INTERVAL {qualifier})");
        let got: Interval = conn.query_row(&sql, [want], |row| row.get(0)).unwrap();
        assert_eq!(got, want, "{sql}");
    }

    assert_eq!(
        Type::Interval(DayToSecond).to_string(),
        "INTERVAL DAY TO SECOND"
    );
    assert_eq!(DayToSecond.to_string(), "DAY TO SECOND");
    assert!(YearToMonth.is_year_month());
    assert!(!Day.is_year_month());

    // An interval column, and one that's out of the range of its type.
    conn.execute("CREATE TABLE foo (x INTERVAL DAY TO SECOND)", ())
        .unwrap();
    let want = interval(DayToSecond, 0, -93_784_500_000);
    conn.execute("INSERT INTO foo (x) VALUES (?)", [want])
        .unwrap();
    let got: Interval = conn
        .query_row("SELECT x FROM foo", (), |r| r.get(0))
        .unwrap();
    assert_eq!(got, want);
    let too_big = interval(DayToSecond, 0, 100 * 86_400_000_000);
    assert_eq!(
        sqlstate(conn.execute("INSERT INTO foo (x) VALUES (?)", [too_big])),
        "22015"
    );
}

#[test]
fn binary_strings() {
    let conn = memory();
    conn.execute(
        "CREATE TABLE foo (v BINARY VARYING(2000), b BINARY(4), l BLOB)",
        (),
    )
    .unwrap();

    // Values that are read in more than one piece, and an empty one.
    let long: Vec<u8> = (0..1000u32).map(|i| (i % 251) as u8).collect();
    for want in [vec![], vec![0, 1, 2, 255], long] {
        conn.execute("DELETE FROM foo", ()).unwrap();
        conn.execute("INSERT INTO foo (v, l) VALUES (?, ?)", (&want, &want[..]))
            .unwrap();
        let (v, l): (Vec<u8>, Vec<u8>) = conn
            .query_row("SELECT v, l FROM foo", (), |r| Ok((r.get(0)?, r.get(1)?)))
            .unwrap();
        assert_eq!(v, want);
        assert_eq!(l, want);
    }

    // A BINARY is padded with zeros, and can be read as an array of its
    // length.
    conn.execute("DELETE FROM foo", ()).unwrap();
    conn.execute("INSERT INTO foo (b) VALUES (?)", [[1u8, 2]])
        .unwrap();
    let row = conn
        .query_row("SELECT b, v FROM foo", (), |r| Ok(r.clone()))
        .unwrap();
    assert_eq!(row.get::<_, [u8; 4]>(0).unwrap(), [1, 2, 0, 0]);
    assert_eq!(row.get::<_, Option<Vec<u8>>>(1).unwrap(), None);
    match row.get::<_, [u8; 3]>(0) {
        Err(Error::FromSqlConversion {
            source: FromSqlError::Other(_),
            ..
        }) => {}
        other => panic!("{other:?}"),
    }
    match row.get::<_, String>(0) {
        Err(Error::FromSqlConversion {
            source: FromSqlError::InvalidType,
            ..
        }) => {}
        other => panic!("{other:?}"),
    }

    let mut stmt = conn.prepare("SELECT v, b, l FROM foo").unwrap();
    let rows = stmt.query(()).unwrap();
    let lengths: Vec<_> = rows.columns().iter().map(|c| c.length()).collect();
    assert_eq!(lengths, [Some(2000), Some(4), None]);
}

#[test]
fn decfloat_and_clob() {
    let conn = memory();
    conn.execute(
        "CREATE TABLE foo (d DECFLOAT, c CHARACTER LARGE OBJECT(10))",
        (),
    )
    .unwrap();
    let want = Value::DecFloat("1.5E+20".into());
    conn.execute("INSERT INTO foo (d, c) VALUES (?, ?)", (&want, "abc"))
        .unwrap();
    let row = conn
        .query_row("SELECT d, c FROM foo", (), |r| Ok(r.clone()))
        .unwrap();
    assert_eq!(row.get::<_, Value>(0).unwrap(), want);
    assert_eq!(row.get::<_, String>(0).unwrap(), "1.5E+20");
    assert_eq!(row.get::<_, f64>(0).unwrap(), 1.5e20);
    assert_eq!(row.get::<_, String>(1).unwrap(), "abc");
    assert_eq!(row.columns()[1].length(), Some(10));
}

#[test]
fn savepoints() {
    let mut conn = memory();
    conn.execute("CREATE TABLE foo (x INTEGER)", ()).unwrap();
    let insert = |conn: &Connection, value: i64| {
        conn.execute("INSERT INTO foo (x) VALUES (?)", [value])
            .unwrap();
    };
    let xs = |conn: &Connection| -> Vec<i64> {
        let mut stmt = conn.prepare("SELECT x FROM foo ORDER BY x").unwrap();
        let rows = stmt.query_map((), |r| r.get(0)).unwrap();
        rows.collect::<vsql::Result<_>>().unwrap()
    };

    let mut tx = conn.transaction().unwrap();
    insert(&tx, 1);
    {
        let mut sp = tx.savepoint().unwrap();
        insert(&sp, 2);
        // The savepoint is still active after it's rolled back to.
        sp.rollback().unwrap();
        assert_eq!(xs(&sp), [1]);
        insert(&sp, 3);
        sp.rollback().unwrap();
        insert(&sp, 4);
        {
            let sp2 = sp.savepoint().unwrap();
            insert(&sp2, 5);
        }
        assert_eq!(xs(&sp), [1, 4]);
        let sp3 = sp.savepoint_with_name("it's \"quoted\"").unwrap();
        insert(&sp3, 6);
        sp3.commit().unwrap();
        sp.commit().unwrap();
    }
    assert_eq!(xs(&tx), [1, 4, 6]);
    {
        let sp = tx.savepoint().unwrap();
        insert(&sp, 7);
    }
    assert_eq!(xs(&tx), [1, 4, 6]);
    tx.commit().unwrap();

    // A savepoint of the connection starts a transaction.
    let mut sp = conn.savepoint().unwrap();
    assert!(!sp.is_autocommit());
    insert(&sp, 8);
    sp.rollback().unwrap();
    insert(&sp, 9);
    sp.commit().unwrap();
    assert!(conn.is_autocommit());
    assert_eq!(xs(&conn), [1, 4, 6, 9]);

    // Dropping it rolls back the transaction.
    {
        let sp = conn.savepoint_with_name("sp").unwrap();
        insert(&sp, 10);
    }
    assert!(conn.is_autocommit());
    assert_eq!(xs(&conn), [1, 4, 6, 9]);
}

#[test]
fn savepoint_names() {
    let mut conn = memory();
    conn.execute("CREATE TABLE foo (x INTEGER)", ()).unwrap();
    let insert = |conn: &Connection, value: i64| {
        conn.execute("INSERT INTO foo (x) VALUES (?)", [value])
            .unwrap();
    };
    let count = |conn: &Connection| -> i64 {
        conn.query_row("SELECT COUNT(*) FROM foo", (), |r| r.get(0))
            .unwrap()
    };

    let mut tx = conn.transaction().unwrap();
    {
        // A name that's the same as a generated one (VSQL_SP_2 for the
        // savepoint in this one).
        let mut outer = tx.savepoint_with_name("VSQL_SP_2").unwrap();
        insert(&outer, 1);
        {
            let mut inner = outer.savepoint().unwrap();
            insert(&inner, 2);
            // A name that's in use (by a generated name, or a given one).
            assert!(matches!(
                inner.savepoint_with_name("VSQL_SP_2"),
                Err(Error::SavepointNameInUse(name)) if name == "VSQL_SP_2"
            ));
            let mut sp = inner.savepoint_with_name("sp").unwrap();
            assert!(matches!(
                sp.savepoint_with_name("sp"),
                Err(Error::SavepointNameInUse(_))
            ));
        } // Rolled back to, which leaves outer.
        assert_eq!(count(&outer), 1);
        outer.rollback().unwrap();
        assert_eq!(count(&outer), 0);
        insert(&outer, 3);
        outer.commit().unwrap();
    }
    // A name can be used again once its savepoint is released.
    {
        let sp = tx.savepoint_with_name("VSQL_SP_2").unwrap();
        insert(&sp, 4);
        sp.commit().unwrap();
    }
    tx.commit().unwrap();
    assert_eq!(count(&conn), 2);
}

#[test]
fn diagnostics() {
    let conn = memory();
    conn.execute(
        "CREATE TABLE foo (id INTEGER PRIMARY KEY, v INTEGER, CONSTRAINT vpos CHECK (v > 0))",
        (),
    )
    .unwrap();
    conn.execute("INSERT INTO foo (id, v) VALUES (1, 1)", ())
        .unwrap();

    let diagnostics = |sql: &str| conn.execute(sql, ()).unwrap_err().diagnostics().cloned();

    let d = diagnostics("INSERT INTO foo (id, v) VALUES (1, 2)").unwrap();
    assert_eq!(d.constraint_name.as_deref(), Some("FOO_PRIMARY_KEY"));
    assert_eq!(d.constraint_schema.as_deref(), Some("PUBLIC"));
    assert_eq!(d.constraint_catalog.as_deref(), Some(":memory:"));
    assert_eq!(d.table_name.as_deref(), Some("FOO"));
    assert_eq!(d.schema_name.as_deref(), Some("PUBLIC"));
    assert_eq!(d.catalog_name.as_deref(), Some(":memory:"));
    assert_eq!(d.column_name, None);

    let d = diagnostics("INSERT INTO foo (id, v) VALUES (2, -1)").unwrap();
    assert_eq!(d.constraint_name.as_deref(), Some("VPOS"));

    let d = diagnostics("SELECT * FROM nope").unwrap();
    assert_eq!(d.table_name.as_deref(), Some("NOPE"));
    assert_eq!(d.constraint_name, None);

    let d = diagnostics("SELECT nope FROM foo").unwrap();
    assert_eq!(d.column_name.as_deref(), Some("NOPE"));

    let d = diagnostics("VALUES 1 / 0").unwrap();
    assert_eq!(d, Default::default());

    assert!(Error::QueryReturnedNoRows.diagnostics().is_none());
}

#[test]
fn parameter_count() {
    let conn = memory();
    let count = |sql: &str| conn.prepare(sql).unwrap().parameter_count();
    assert_eq!(count("VALUES 1"), 0);
    assert_eq!(count("VALUES CAST(? AS INTEGER) + CAST(? AS INTEGER)"), 2);
}

#[test]
fn version() {
    let version = vsql::version();
    let parts: Vec<i32> = version.split('.').map(|p| p.parse().unwrap()).collect();
    assert_eq!(parts.len(), 3, "{version}");
    assert_eq!(
        vsql::version_number(),
        parts[0] * 1_000_000 + parts[1] * 1_000 + parts[2]
    );
    assert!(vsql::version_number() >= 1_007_058);

    let info = vsql::build_info();
    assert!(info.version.is_empty() || info.version.trim_start_matches('v') == version);
    assert!(!info.v_version.is_empty());
    assert!(!info.date.is_empty());
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

    // A DECFLOAT, which may have an exponent.
    for (sql, want) in [("1.25", "1.25"), ("1.5E20", "150000000000000000000")] {
        let got: Decimal = conn
            .query_row(&format!("VALUES CAST({sql} AS DECFLOAT)"), (), |r| r.get(0))
            .unwrap();
        assert_eq!(got, Decimal::from_str(want).unwrap(), "{sql}");
    }
}

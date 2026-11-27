// Queries are fetched a rowset (many rows) at a time, unless a column can't be
// bound (such as a CLOB without a length).

use vsql::{Connection, Value};

fn date(i: i64) -> vsql::Date {
    vsql::Date {
        year: 2024,
        month: 1,
        day: 1 + (i % 28) as u8,
    }
}

fn memory() -> Connection {
    Connection::open_in_memory().unwrap()
}

#[test]
fn many_rows() {
    let mut conn = memory();
    conn.execute(
        "CREATE TABLE t (id INTEGER, s VARCHAR(10), c CHAR(3), d DOUBLE PRECISION, r REAL, \
         n DECIMAL(10, 2), b VARBINARY(4), dt DATE, x SMALLINT, y BIGINT, z BOOLEAN)",
        (),
    )
    .unwrap();
    let tx = conn.transaction().unwrap();
    {
        let mut insert = tx
            .prepare("INSERT INTO t VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)")
            .unwrap();
        for i in 0..250i64 {
            if i % 7 == 0 {
                let nulls = vec![Value::Null; 10];
                insert
                    .execute(vsql::params_from_iter(
                        std::iter::once(Value::Integer(i)).chain(nulls),
                    ))
                    .unwrap();
                continue;
            }
            insert
                .execute(vsql::params![
                    i,
                    format!("é{i}"),
                    "ab",
                    i as f64 / 2.0,
                    i as f64,
                    format!("{i}.25"),
                    vec![i as u8; (i % 5) as usize],
                    date(i),
                    -i,
                    i * 1_000_000_000,
                    i % 2 == 0
                ])
                .unwrap();
        }
    }
    tx.commit().unwrap();

    let mut stmt = conn.prepare("SELECT * FROM t ORDER BY id").unwrap();
    let mut n = 0;
    for (i, row) in stmt.query(()).unwrap().enumerate() {
        let row = row.unwrap();
        let i = i as i64;
        assert_eq!(row.get::<_, i64>(0).unwrap(), i);
        if i % 7 == 0 {
            for c in 1..11 {
                assert_eq!(row.get_ref(c).unwrap(), &Value::Null);
            }
        } else {
            assert_eq!(row.get::<_, String>(1).unwrap(), format!("é{i}"));
            assert_eq!(row.get::<_, String>(2).unwrap(), "ab ");
            assert_eq!(row.get::<_, f64>(3).unwrap(), i as f64 / 2.0);
            assert_eq!(row.get::<_, f64>(4).unwrap(), i as f64);
            assert_eq!(row.get::<_, String>(5).unwrap(), format!("{i}.25"));
            assert_eq!(
                row.get::<_, Vec<u8>>(6).unwrap(),
                vec![i as u8; (i % 5) as usize]
            );
            assert_eq!(row.get::<_, vsql::Date>(7).unwrap(), date(i));
            assert_eq!(row.get::<_, i64>(8).unwrap(), -i);
            assert_eq!(row.get::<_, i64>(9).unwrap(), i * 1_000_000_000);
            assert_eq!(row.get::<_, bool>(10).unwrap(), i % 2 == 0);
        }
        n += 1;
    }
    assert_eq!(n, 250);
}

#[test]
fn widest_values() {
    // The string of each of these is longer than its column size.
    let conn = memory();
    let sql = "VALUES ROW(TIMESTAMP '9999-12-31 23:59:59.999999+14:00', \
               TIME '23:59:59.999999-12:30', \
               CAST(-99999999999999999999999999999999999999 AS DECIMAL(38, 0)), \
               CAST(-1.25 AS NUMERIC(3, 2)), \
               CAST('-1.234567890123456789012345678901234E-6143' AS DECFLOAT), \
               INTERVAL '-99999999 23:59:59.999999' DAY(8) TO SECOND(6), \
               INTERVAL '-99999999-11' YEAR(8) TO MONTH, \
               INTERVAL '-99999999.999999' SECOND(8, 6), \
               CAST(-32768 AS SMALLINT), -1.7976931348623157E308)";
    // With a CLOB, the row is read with SQLGetData.
    let with_clob = sql.replace("E308)", "E308, CAST('x' AS CLOB))");
    let mut values = conn
        .query_row(&with_clob, (), |row| Ok(row.clone().into_values()))
        .unwrap();
    assert_eq!(values.pop(), Some(Value::Text("x".to_string())));
    let mut stmt = conn.prepare(sql).unwrap();
    let rows: Vec<_> = stmt.query(()).unwrap().map(|r| r.unwrap()).collect();
    assert_eq!(rows[0].clone().into_values(), values);
    assert_eq!(
        rows[0].get::<_, String>(2).unwrap(),
        "-99999999999999999999999999999999999999"
    );
    assert_eq!(rows[0].get::<_, String>(3).unwrap(), "-1.25");
    assert_eq!(
        rows[0].get::<_, vsql::Interval>(5).unwrap().to_string(),
        "INTERVAL '-99999999 23:59:59.999999' DAY(8) TO SECOND(6)"
    );
}

#[test]
fn wide_rows() {
    // Fewer rows fit in each rowset.
    let conn = memory();
    conn.execute(
        "CREATE TABLE t (id INTEGER, a VARCHAR(4000), b VARCHAR(4000), c VARCHAR(4000), \
         d VARCHAR(4000))",
        (),
    )
    .unwrap();
    let mut insert = conn
        .prepare("INSERT INTO t VALUES (?, ?, ?, ?, ?)")
        .unwrap();
    for i in 0..40 {
        let s = "é".repeat(i * 100);
        insert.execute((i as i64, &s, &s, &s, &s)).unwrap();
    }
    let mut stmt = conn.prepare("SELECT * FROM t ORDER BY id").unwrap();
    let rows: Vec<_> = stmt
        .query_map((), |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, String>(4)?))
        })
        .unwrap()
        .map(|r| r.unwrap())
        .collect();
    assert_eq!(rows.len(), 40);
    for (i, (id, d)) in rows.into_iter().enumerate() {
        assert_eq!(id, i as i64);
        assert_eq!(d, "é".repeat(i * 100));
    }
}

#[test]
fn columns_that_arent_bound() {
    // A CLOB or BLOB without a length can't be bound, so the rows are fetched
    // one at a time, and a statement can be used for both.
    let conn = memory();
    conn.execute("CREATE TABLE t (x INTEGER, c CLOB, b BLOB)", ())
        .unwrap();
    for i in 0..150 {
        conn.execute(
            "INSERT INTO t VALUES (?, ?, ?)",
            (i, "x".repeat(i as usize), vec![1u8; i as usize]),
        )
        .unwrap();
    }

    let mut stmt = conn
        .prepare("SELECT * FROM t WHERE x >= ? ORDER BY x")
        .unwrap();
    let mut both = conn.prepare("SELECT x, c FROM t ORDER BY x").unwrap();
    let mut bound = conn.prepare("SELECT x FROM t ORDER BY x").unwrap();
    for _ in 0..2 {
        let rows: Vec<_> = stmt
            .query_map([0], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Vec<u8>>(2)?,
                ))
            })
            .unwrap()
            .map(|r| r.unwrap())
            .collect();
        assert_eq!(rows.len(), 150);
        assert_eq!(rows[149].1.len(), 149);
        assert_eq!(rows[149].2.len(), 149);

        let n = both.query(()).unwrap().count();
        assert_eq!(n, 150);
        let n = bound.query(()).unwrap().count();
        assert_eq!(n, 150);
    }
}

#[test]
fn reusing_a_statement() {
    // The columns are unbound when the rows are dropped (part way through a
    // rowset or not), so the statement can be executed again.
    let conn = memory();
    conn.execute("CREATE TABLE t (x INTEGER, s VARCHAR(5))", ())
        .unwrap();
    for i in 0..230 {
        conn.execute("INSERT INTO t VALUES (?, 'a')", [i]).unwrap();
    }
    let mut stmt = conn
        .prepare("SELECT x, s FROM t WHERE x >= ? ORDER BY x")
        .unwrap();
    for take in [0, 1, 99, 100, 101, 229, 230, 231] {
        let xs: Vec<i64> = stmt
            .query_map([0], |row| row.get(0))
            .unwrap()
            .take(take)
            .map(|r| r.unwrap())
            .collect();
        assert_eq!(xs, (0..230).take(take).collect::<Vec<_>>());
        let x: i64 = stmt.query_row([200], |row| row.get(0)).unwrap();
        assert_eq!(x, 200);
    }

    // A statement that changes rows (and isn't a query) after a query.
    let mut stmt = conn.prepare("DELETE FROM t WHERE x < ?").unwrap();
    assert_eq!(stmt.execute([10]).unwrap(), 10);
    let n = conn
        .prepare("SELECT x FROM t")
        .unwrap()
        .query(())
        .unwrap()
        .count();
    assert_eq!(n, 220);
}

#[test]
fn transactions() {
    let mut conn = memory();
    conn.execute("CREATE TABLE t (x INTEGER)", ()).unwrap();
    let tx = conn.transaction().unwrap();
    for i in 0..150 {
        tx.execute("INSERT INTO t VALUES (?)", [i]).unwrap();
    }
    {
        let mut stmt = tx.prepare("SELECT x FROM t").unwrap();
        assert_eq!(stmt.query(()).unwrap().count(), 150);
    }
    tx.rollback().unwrap();
    let n: i64 = conn
        .query_row("SELECT COUNT(*) FROM t", (), |row| row.get(0))
        .unwrap();
    assert_eq!(n, 0);
}

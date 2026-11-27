// Connection::prepare_cached and the prepared statement cache.

use vsql::Connection;

fn memory() -> Connection {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute("CREATE TABLE foo (x INTEGER)", ()).unwrap();
    conn
}

fn count(conn: &Connection) -> i64 {
    conn.query_row("SELECT COUNT(*) FROM foo", (), |row| row.get(0))
        .unwrap()
}

#[test]
fn statements_are_reused() {
    let conn = memory();
    for x in 1..=3 {
        let mut stmt = conn
            .prepare_cached("INSERT INTO foo (x) VALUES (?)")
            .unwrap();
        assert_eq!(stmt.execute([x]).unwrap(), 1);
    }
    let sum: i64 = conn
        .prepare_cached("SELECT SUM(x) FROM foo")
        .unwrap()
        .query_row((), |row| row.get(0))
        .unwrap();
    assert_eq!(sum, 6);

    // A cached statement isn't prepared again, so dropping the table doesn't
    // stop it being taken from the cache, only executed.
    conn.execute("DROP TABLE foo", ()).unwrap();
    let mut stmt = conn
        .prepare_cached("INSERT INTO foo (x) VALUES (?)")
        .unwrap();
    assert!(stmt.execute([4]).is_err());
    drop(stmt);
    conn.flush_prepared_statement_cache();
    assert!(conn
        .prepare_cached("INSERT INTO foo (x) VALUES (?)")
        .is_err());
}

#[test]
fn statements_in_use_at_the_same_time() {
    let conn = memory();
    let sql = "INSERT INTO foo (x) VALUES (?)";
    let mut a = conn.prepare_cached(sql).unwrap();
    let mut b = conn.prepare_cached(sql).unwrap();
    a.execute([1]).unwrap();
    b.execute([2]).unwrap();
    drop(a);
    drop(b);
    conn.prepare_cached(sql).unwrap().execute([3]).unwrap();
    assert_eq!(count(&conn), 3);
}

#[test]
fn open_cursors_are_closed() {
    let conn = memory();
    conn.execute("INSERT INTO foo (x) VALUES (1), (2)", ())
        .unwrap();
    let sql = "SELECT x FROM foo ORDER BY x";
    {
        let mut stmt = conn.prepare_cached(sql).unwrap();
        let mut rows = stmt.query(()).unwrap();
        rows.next().unwrap().unwrap();
        // Its cursor isn't closed.
        std::mem::forget(rows);
    }
    let xs = conn
        .prepare_cached(sql)
        .unwrap()
        .query_map((), |row| row.get::<_, i64>(0))
        .unwrap()
        .collect::<vsql::Result<Vec<_>>>()
        .unwrap();
    assert_eq!(xs, [1, 2]);
}

#[test]
fn discard_and_capacity() {
    let conn = memory();
    let stmt = conn.prepare_cached("VALUES 1").unwrap();
    assert_eq!(
        format!("{stmt:?}"),
        "CachedStatement { sql: \"VALUES 1\", stmt: Some(Statement { .. }) }"
    );
    assert_eq!(stmt.parameter_count(), 0);
    stmt.discard();

    conn.set_prepared_statement_cache_capacity(0);
    conn.prepare_cached("INSERT INTO foo (x) VALUES (1)")
        .unwrap()
        .execute(())
        .unwrap();
    conn.execute("DROP TABLE foo", ()).unwrap();
    // It wasn't kept, so it's prepared again (and fails).
    assert!(conn
        .prepare_cached("INSERT INTO foo (x) VALUES (1)")
        .is_err());
}

#[test]
fn statements_are_freed_when_the_connection_closes() {
    let conn = memory();
    conn.prepare_cached("VALUES 1").unwrap();
    conn.close().unwrap();
}

use vsql::Connection;

fn count(conn: &Connection) -> i64 {
    conn.query_row("SELECT COUNT(*) FROM foo", (), |row| row.get(0))
        .unwrap()
}

#[test]
fn executes_each_statement() {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute_batch(
        "-- The schema; and some rows.
        CREATE TABLE foo (x INTEGER, y VARCHAR(10));
        INSERT INTO foo (x, y) VALUES (1, 'a;b'); /* ; */
        INSERT INTO foo (x, y) VALUES (2, '--');
        SELECT * FROM foo;
        ;",
    )
    .unwrap();

    let ys = conn
        .prepare("SELECT y FROM foo ORDER BY x")
        .unwrap()
        .query_map((), |row| row.get::<_, String>(0))
        .unwrap()
        .collect::<vsql::Result<Vec<_>>>()
        .unwrap();
    assert_eq!(ys, ["a;b", "--"]);
}

#[test]
fn empty() {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute_batch("").unwrap();
    conn.execute_batch(" ; -- nothing\n").unwrap();
}

#[test]
fn stops_at_an_error() {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute("CREATE TABLE foo (x INTEGER)", ()).unwrap();

    let err = conn
        .execute_batch(
            "INSERT INTO foo (x) VALUES (1);
            INSERT INTO nope (x) VALUES (2);
            INSERT INTO foo (x) VALUES (3)",
        )
        .unwrap_err();
    assert_eq!(err.sqlstate(), Some("42P01"));

    // The first statement was committed, and the last wasn't executed.
    assert_eq!(count(&conn), 1);
}

#[test]
fn in_a_transaction() {
    let mut conn = Connection::open_in_memory().unwrap();
    conn.execute("CREATE TABLE foo (x INTEGER)", ()).unwrap();

    let tx = conn.transaction().unwrap();
    tx.execute_batch("INSERT INTO foo (x) VALUES (1); INSERT INTO foo (x) VALUES (2)")
        .unwrap();
    assert_eq!(count(&tx), 2);
    tx.rollback().unwrap();

    assert_eq!(count(&conn), 0);
}

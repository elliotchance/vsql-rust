use vsql::{Connection, Error};

fn memory() -> Connection {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute("CREATE TABLE foo (x INTEGER)", ()).unwrap();
    conn.execute("CREATE SEQUENCE seq", ()).unwrap();
    conn
}

// A query with a side effect.
const NEXT: &str = "SELECT NEXT VALUE FOR seq FROM (VALUES 1) AS t (a)";

fn next(conn: &Connection) -> i64 {
    conn.query_row(NEXT, (), |r| r.get(0)).unwrap()
}

#[test]
fn connection_execute_on_a_query() {
    let conn = memory();
    let err = conn.execute("SELECT * FROM foo", ()).unwrap_err();
    assert!(matches!(err, Error::ExecuteReturnedResults));
    assert_eq!(
        err.to_string(),
        "execute was given a query, which returns rows"
    );
    assert!(matches!(
        conn.execute("VALUES CAST(? AS INTEGER)", [1]),
        Err(Error::ExecuteReturnedResults)
    ));

    // The query isn't executed.
    assert!(matches!(
        conn.execute(NEXT, ()),
        Err(Error::ExecuteReturnedResults)
    ));
    assert_eq!(next(&conn), 1);

    // An error preparing the statement comes first.
    let err = conn.execute("SELECT * FROM nope", ()).unwrap_err();
    assert_eq!(err.sqlstate(), Some("42P01"));

    // Statements that aren't queries are the same.
    assert_eq!(
        conn.execute("INSERT INTO foo (x) VALUES (1)", ()).unwrap(),
        1
    );
    assert_eq!(
        conn.execute("INSERT INTO foo (x) VALUES (?)", [2]).unwrap(),
        1
    );
    assert_eq!(conn.execute("UPDATE foo SET x = x + 1", ()).unwrap(), 2);
}

#[test]
fn statement_execute_on_a_query() {
    let conn = memory();
    let mut stmt = conn.prepare(NEXT).unwrap();
    assert!(matches!(
        stmt.execute(()),
        Err(Error::ExecuteReturnedResults)
    ));

    // The statement can still be queried, and wasn't executed before.
    let x: i64 = stmt.query_row((), |r| r.get(0)).unwrap();
    assert_eq!(x, 1);
    assert!(matches!(
        stmt.execute(()),
        Err(Error::ExecuteReturnedResults)
    ));
    assert_eq!(next(&conn), 2);
}

#[test]
fn execute_on_a_query_in_a_transaction() {
    let mut conn = memory();
    let tx = conn.transaction().unwrap();
    tx.execute("INSERT INTO foo (x) VALUES (1)", ()).unwrap();
    assert!(matches!(
        tx.execute("SELECT * FROM foo", ()),
        Err(Error::ExecuteReturnedResults)
    ));
    tx.commit().unwrap();

    let n: i64 = conn
        .query_row("SELECT COUNT(*) FROM foo", (), |r| r.get(0))
        .unwrap();
    assert_eq!(n, 1);
}

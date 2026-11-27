use vsql::{Connection, Error};

fn memory() -> Connection {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute("CREATE TABLE foo (x INTEGER, y INTEGER)", ())
        .unwrap();
    conn
}

fn count(conn: &Connection) -> i64 {
    conn.query_row("SELECT COUNT(*) FROM foo", (), |r| r.get(0))
        .unwrap()
}

#[test]
fn connection_execute() {
    let conn = memory();
    let sql = "INSERT INTO foo (x, y) VALUES (?, ?)";

    let err = conn.execute(sql, [1]).unwrap_err();
    assert!(matches!(err, Error::InvalidParameterCount(1, 2)));
    assert_eq!(
        err.to_string(),
        "wrong number of parameters: 1 given, 2 expected"
    );
    assert!(matches!(
        conn.execute(sql, (1, 2, 3)),
        Err(Error::InvalidParameterCount(3, 2))
    ));
    assert!(matches!(
        conn.execute("INSERT INTO foo (x) VALUES (1)", [1]),
        Err(Error::InvalidParameterCount(1, 0))
    ));
    assert_eq!(count(&conn), 0);

    // Other errors are the same.
    let err = conn
        .execute("INSERT INTO nope (x) VALUES (1)", ())
        .unwrap_err();
    assert_eq!(err.sqlstate(), Some("42P01"));
    let err = conn
        .execute("INSERT INTO nope (x) VALUES (?)", [1])
        .unwrap_err();
    assert_eq!(err.sqlstate(), Some("42P01"));

    assert_eq!(conn.execute(sql, (1, 2)).unwrap(), 1);
    assert_eq!(count(&conn), 1);
}

#[test]
fn statement() {
    let conn = memory();
    let mut stmt = conn
        .prepare("INSERT INTO foo (x, y) VALUES (?, ?)")
        .unwrap();
    assert!(matches!(
        stmt.execute([1]),
        Err(Error::InvalidParameterCount(1, 2))
    ));
    assert!(matches!(
        stmt.execute(()),
        Err(Error::InvalidParameterCount(0, 2))
    ));
    assert_eq!(stmt.execute((1, 2)).unwrap(), 1);

    let mut stmt = conn.prepare("VALUES CAST(? AS INTEGER)").unwrap();
    assert!(matches!(
        stmt.query((1, 2)).map(|_| ()),
        Err(Error::InvalidParameterCount(2, 1))
    ));
    assert!(matches!(
        conn.query_row("VALUES 1", [1], |r| r.get::<_, i64>(0)),
        Err(Error::InvalidParameterCount(1, 0))
    ));
    assert_eq!(stmt.query_row([5], |r| r.get::<_, i64>(0)).unwrap(), 5);
    assert_eq!(count(&conn), 1);
}

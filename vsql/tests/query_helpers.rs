use vsql::{Connection, Error, OptionalExtension, Row, Rows};

fn memory() -> Connection {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute("CREATE TABLE foo (x INTEGER)", ()).unwrap();
    for x in 1..=3 {
        conn.execute("INSERT INTO foo (x) VALUES (?)", [x]).unwrap();
    }
    conn
}

#[derive(Debug)]
enum MyError {
    Vsql(Error),
    Odd(i64),
}

impl From<Error> for MyError {
    fn from(e: Error) -> MyError {
        MyError::Vsql(e)
    }
}

fn even(row: &Row) -> Result<i64, MyError> {
    let x: i64 = row.get(0)?;
    if x % 2 == 0 {
        Ok(x)
    } else {
        Err(MyError::Odd(x))
    }
}

#[test]
fn optional() {
    let conn = memory();
    let sql = "SELECT x FROM foo WHERE x = ?";
    let get = |x: i64| conn.query_row(sql, [x], |r| r.get::<_, i64>(0)).optional();
    assert_eq!(get(2).unwrap(), Some(2));
    assert_eq!(get(5).unwrap(), None);

    let err = conn
        .query_row("SELECT * FROM nope", (), |r| r.get::<_, i64>(0))
        .optional()
        .unwrap_err();
    assert_eq!(err.sqlstate(), Some("42P01"));
}

#[test]
fn exists() {
    let conn = memory();
    let mut stmt = conn.prepare("SELECT x FROM foo WHERE x > ?").unwrap();
    assert!(stmt.exists([2]).unwrap());
    assert!(!stmt.exists([3]).unwrap());
    // The cursor is closed, so the statement can be used again.
    assert!(stmt.exists([0]).unwrap());

    // An error reading the first row.
    let mut stmt = conn.prepare("VALUES 1 / ?").unwrap();
    assert_eq!(stmt.exists([0]).unwrap_err().sqlstate(), Some("22012"));
}

#[test]
fn query_and_then() {
    let conn = memory();
    let mut stmt = conn.prepare("SELECT x FROM foo ORDER BY x").unwrap();
    let xs: Vec<_> = stmt.query_and_then((), even).unwrap().collect();
    assert!(matches!(xs[0], Err(MyError::Odd(1))));
    assert!(matches!(xs[1], Ok(2)));
    assert!(matches!(xs[2], Err(MyError::Odd(3))));
    assert_eq!(xs.len(), 3);

    // A vsql error from a row is converted.
    let xs: Vec<_> = stmt
        .query_and_then((), |row| row.get::<_, String>(5).map_err(MyError::from))
        .unwrap()
        .collect();
    assert!(matches!(
        xs[0],
        Err(MyError::Vsql(Error::InvalidColumnIndex(5)))
    ));

    // An error executing the query is a vsql error.
    let mut stmt = conn.prepare("VALUES 1 / ?").unwrap();
    assert_eq!(
        stmt.query_and_then([0], even).err().unwrap().sqlstate(),
        Some("22012")
    );
}

#[test]
fn query_row_and_then() {
    let conn = memory();
    let sql = "SELECT x FROM foo WHERE x = ?";
    assert!(matches!(conn.query_row_and_then(sql, [2], even), Ok(2)));
    assert!(matches!(
        conn.query_row_and_then(sql, [3], even),
        Err(MyError::Odd(3))
    ));
    assert!(matches!(
        conn.query_row_and_then(sql, [5], even),
        Err(MyError::Vsql(Error::QueryReturnedNoRows))
    ));
    match conn.query_row_and_then("SELECT * FROM nope", (), even) {
        Err(MyError::Vsql(e)) => assert_eq!(e.sqlstate(), Some("42P01")),
        other => panic!("{other:?}"),
    }
    match conn.query_row_and_then("VALUES 1 / ?", [0], even) {
        Err(MyError::Vsql(e)) => assert_eq!(e.sqlstate(), Some("22012")),
        other => panic!("{other:?}"),
    }

    let mut stmt = conn.prepare(sql).unwrap();
    assert!(matches!(stmt.query_row_and_then([2], even), Ok(2)));
}

#[test]
fn rows_mapped_and_then() {
    let conn = memory();
    let mut stmt = conn.prepare("SELECT x FROM foo ORDER BY x").unwrap();
    let xs = stmt
        .query(())
        .unwrap()
        .mapped(|row| row.get::<_, i64>(0))
        .collect::<vsql::Result<Vec<_>>>()
        .unwrap();
    assert_eq!(xs, [1, 2, 3]);

    let xs: Vec<_> = stmt.query(()).unwrap().and_then(even).collect();
    assert!(matches!(
        xs[..],
        [Err(MyError::Odd(1)), Ok(2), Err(MyError::Odd(3))]
    ));
}

// The rows own the statement, so they can be returned.
fn at_most(conn: &Connection, n: i64) -> vsql::Result<Rows<'_>> {
    conn.query("SELECT x FROM foo WHERE x <= ? ORDER BY x", [n])
}

#[test]
fn connection_query() {
    let conn = memory();
    let rows = at_most(&conn, 2).unwrap();
    assert_eq!(rows.column_names(), ["X"]);
    let xs = rows
        .map(|row| row.unwrap().get::<_, i64>(0).unwrap())
        .collect::<Vec<_>>();
    assert_eq!(xs, [1, 2]);

    // Dropping rows that weren't read closes the cursor (and commits), so the
    // table can be dropped.
    let mut rows = at_most(&conn, 3).unwrap();
    rows.next().unwrap().unwrap();
    drop(rows);

    let xs = conn
        .query_map("SELECT x FROM foo ORDER BY x", (), |r| r.get::<_, i64>(0))
        .unwrap()
        .collect::<vsql::Result<Vec<_>>>()
        .unwrap();
    assert_eq!(xs, [1, 2, 3]);

    let xs: Vec<_> = conn
        .query_and_then("SELECT x FROM foo ORDER BY x", (), even)
        .unwrap()
        .collect();
    assert!(matches!(xs[1], Ok(2)));

    // Errors preparing and executing.
    assert_eq!(
        conn.query("SELECT * FROM nope", ())
            .err()
            .unwrap()
            .sqlstate(),
        Some("42P01")
    );
    assert_eq!(
        conn.query("VALUES 1 / ?", [0]).err().unwrap().sqlstate(),
        Some("22012")
    );

    conn.execute("DROP TABLE foo", ()).unwrap();
}

#[test]
fn connection_query_in_a_transaction() {
    let mut conn = memory();
    let tx = conn.transaction().unwrap();
    tx.execute("INSERT INTO foo (x) VALUES (4)", ()).unwrap();
    assert_eq!(tx.query("SELECT x FROM foo", ()).unwrap().count(), 4);
    tx.rollback().unwrap();
    assert_eq!(conn.query("SELECT x FROM foo", ()).unwrap().count(), 3);
}

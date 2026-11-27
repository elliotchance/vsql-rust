use vsql::{named_params, Connection, Error};

fn conn() -> Connection {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute("CREATE TABLE foo (x INTEGER, y VARCHAR(10))", ())
        .unwrap();
    conn
}

fn ys(conn: &Connection) -> Vec<String> {
    conn.prepare("SELECT y FROM foo ORDER BY x")
        .unwrap()
        .query_map((), |row| row.get(0))
        .unwrap()
        .collect::<vsql::Result<_>>()
        .unwrap()
}

#[test]
fn execute() {
    let conn = conn();
    let n = conn
        .execute(
            "INSERT INTO foo (x, y) VALUES (:x, :y) -- :z",
            named_params! { ":y": "a", "x": 1 },
        )
        .unwrap();
    assert_eq!(n, 1);
    assert_eq!(ys(&conn), ["a"]);
}

#[test]
fn used_more_than_once() {
    let conn = conn();
    let mut stmt = conn
        .prepare("INSERT INTO foo (x, y) VALUES (:x, CAST(:x AS VARCHAR(10)) || :s || ':x')")
        .unwrap();
    assert_eq!(stmt.parameter_count(), 2);
    stmt.execute(named_params! { ":x": 1, ":s": "-" }).unwrap();
    // Positional values are the names in the order they're first used.
    stmt.execute((2, "+")).unwrap();
    assert_eq!(ys(&conn), ["1-:x", "2+:x"]);
}

#[test]
fn query() {
    let conn = conn();
    conn.execute(
        "INSERT INTO foo (x, y) VALUES (1, 'a'), (2, 'b'), (3, 'c')",
        (),
    )
    .unwrap();
    let mut stmt = conn
        .prepare("SELECT y FROM foo WHERE x > :min AND x < :max ORDER BY x")
        .unwrap();
    let ys = stmt
        .query_map(named_params! { ":max": 3, ":min": 0 }, |row| {
            row.get::<_, String>(0)
        })
        .unwrap()
        .collect::<vsql::Result<Vec<_>>>()
        .unwrap();
    assert_eq!(ys, ["a", "b"]);

    let y: String = conn
        .query_row(
            "SELECT y FROM foo WHERE x = :x",
            named_params! { ":x": 3 },
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(y, "c");
}

#[test]
fn no_names() {
    let conn = conn();
    conn.execute("INSERT INTO foo (x, y) VALUES (1, 'a')", named_params! {})
        .unwrap();
    assert_eq!(ys(&conn), ["a"]);

    let mut stmt = conn.prepare("SELECT y FROM foo").unwrap();
    let y: String = stmt.query_row(named_params! {}, |row| row.get(0)).unwrap();
    assert_eq!(y, "a");
}

#[test]
fn errors() {
    let conn = conn();
    let sql = "INSERT INTO foo (x, y) VALUES (:x, :y)";

    match conn.execute(sql, named_params! { ":x": 1, ":z": "a" }) {
        Err(Error::InvalidParameterName(name)) => assert_eq!(name, "z"),
        other => panic!("{other:?}"),
    }

    let err = conn.execute(sql, named_params! { ":x": 1 }).unwrap_err();
    assert!(matches!(err, Error::InvalidParameter { index: 2, .. }));
    assert_eq!(err.to_string(), "parameter 2: there's no value for :y");

    let err = conn
        .execute(
            "INSERT INTO foo (x, y) VALUES (:x, ?)",
            named_params! { ":x": 1 },
        )
        .unwrap_err();
    assert_eq!(
        err.to_string(),
        "parameter 2: a ? can't be used with named parameters"
    );

    // A statement without names.
    let err = conn
        .execute("INSERT INTO foo (x) VALUES (?)", named_params! { ":x": 1 })
        .unwrap_err();
    assert_eq!(err.to_string(), "there's no parameter :x");

    // A different number of positional values.
    let err = conn.execute(sql, [1]).unwrap_err();
    assert!(matches!(err, Error::InvalidParameterCount(1, 2)), "{err:?}");

    assert_eq!(ys(&conn), Vec::<String>::new());
}

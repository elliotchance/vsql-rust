use vsql::{Connection, Error, Type};

fn memory() -> Connection {
    Connection::open_in_memory().unwrap()
}

#[test]
fn columns_before_execute() {
    let conn = memory();
    conn.execute(
        "CREATE TABLE foo (x INTEGER NOT NULL, \"y z\" VARCHAR(10))",
        (),
    )
    .unwrap();

    let mut stmt = conn.prepare("SELECT x, \"y z\" FROM foo").unwrap();
    assert_eq!(stmt.column_count(), 2);
    assert_eq!(stmt.column_names(), ["X", "y z"]);
    assert_eq!(stmt.column_name(0).unwrap(), "X");
    assert_eq!(stmt.column_name(1).unwrap(), "y z");
    assert!(matches!(
        stmt.column_name(2),
        Err(Error::InvalidColumnIndex(2))
    ));
    assert_eq!(stmt.column_index("x").unwrap(), 0);
    assert_eq!(stmt.column_index("Y Z").unwrap(), 1);
    assert!(matches!(
        stmt.column_index("nope"),
        Err(Error::InvalidColumnName(name)) if name == "nope"
    ));

    let columns = stmt.columns();
    assert_eq!(columns[0].ty(), Type::Integer);
    assert!(!columns[0].is_nullable());
    assert_eq!(columns[1].ty(), Type::CharacterVarying);
    assert_eq!(columns[1].length(), Some(10));
    assert!(columns[1].is_nullable());

    // They're the same columns that the rows have, before and after the
    // statement is executed.
    let before = stmt.columns().to_vec();
    let rows = stmt.query(()).unwrap();
    assert_eq!(rows.columns(), before.as_slice());
    drop(rows);
    assert_eq!(stmt.columns(), before.as_slice());
}

#[test]
fn columns_with_parameters() {
    let conn = memory();
    let stmt = conn
        .prepare("VALUES ROW(CAST(? AS BIGINT), CAST(? AS BOOLEAN))")
        .unwrap();
    assert_eq!(stmt.column_count(), 2);
    assert_eq!(stmt.column_names(), ["COL1", "COL2"]);
    assert_eq!(stmt.columns()[0].ty(), Type::BigInt);
    assert_eq!(stmt.columns()[1].ty(), Type::Boolean);
}

#[test]
fn statement_that_isnt_a_query_has_no_columns() {
    let conn = memory();
    conn.execute("CREATE TABLE foo (x INTEGER)", ()).unwrap();

    let stmt = conn.prepare("INSERT INTO foo (x) VALUES (?)").unwrap();
    assert_eq!(stmt.column_count(), 0);
    assert!(stmt.columns().is_empty());
    assert!(stmt.column_names().is_empty());
    assert!(matches!(
        stmt.column_name(0),
        Err(Error::InvalidColumnIndex(0))
    ));
}

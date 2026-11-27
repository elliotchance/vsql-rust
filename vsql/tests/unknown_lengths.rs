// Strings whose length libvsql doesn't know can't be fetched a rowset at a
// time, so they're fetched a row at a time.

use vsql::Connection;

#[test]
fn unknown_lengths() {
    let conn = Connection::open_in_memory().unwrap();
    let schema: String = conn
        .query_row("VALUES CURRENT_SCHEMA", (), |r| r.get(0))
        .unwrap();
    assert_eq!(schema, "PUBLIC");

    conn.execute("CREATE TABLE foo (x INTEGER)", ()).unwrap();
    let tables: Vec<String> = conn
        .tables(None)
        .unwrap()
        .into_iter()
        .map(|t| t.name)
        .collect();
    assert_eq!(tables, ["FOO"]);
}

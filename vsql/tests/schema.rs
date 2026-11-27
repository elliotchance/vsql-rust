// The schema API: tables, columns and keys.

use vsql::{Connection, ForeignKeyAction, TableType, Type};

fn setup() -> Connection {
    let conn = Connection::open_in_memory().unwrap();
    for sql in [
        "CREATE TABLE foo (id INTEGER NOT NULL, a VARCHAR(10) DEFAULT 'x', b DECIMAL(10, 2), \
         PRIMARY KEY (id), UNIQUE (a))",
        "CREATE TABLE \"Bar\" (x INTEGER, y VARCHAR(10), z INTEGER, \
         CONSTRAINT bar_fk FOREIGN KEY (x) REFERENCES foo (id) ON DELETE CASCADE, \
         FOREIGN KEY (y) REFERENCES foo (a))",
        "CREATE TABLE f_o (q INTEGER)",
        "CREATE TABLE kk (a INTEGER, b INTEGER, c INTEGER, d INTEGER, \
         PRIMARY KEY (b, a), UNIQUE (c, d))",
        "CREATE TABLE kr (e INTEGER, f INTEGER, g INTEGER, h INTEGER, \
         CONSTRAINT kr_2 FOREIGN KEY (f, e) REFERENCES kk (b, a) ON UPDATE SET NULL, \
         CONSTRAINT kr_1 FOREIGN KEY (h, g) REFERENCES kk (c, d) ON DELETE SET DEFAULT)",
        "CREATE SCHEMA s2",
        "CREATE TABLE s2.foo (q BOOLEAN NOT NULL, PRIMARY KEY (q))",
    ] {
        conn.execute(sql, ()).unwrap();
    }
    conn
}

#[test]
fn tables() {
    let conn = setup();
    let names = |schema| {
        conn.tables(schema)
            .unwrap()
            .into_iter()
            .map(|t| {
                assert_eq!(t.table_type, TableType::Table);
                format!("{}.{}", t.schema, t.name)
            })
            .collect::<Vec<_>>()
    };
    assert_eq!(
        names(None),
        [
            "PUBLIC.Bar",
            "PUBLIC.FOO",
            "PUBLIC.F_O",
            "PUBLIC.KK",
            "PUBLIC.KR"
        ]
    );
    assert_eq!(names(Some("S2")), ["S2.FOO"]);
    assert!(names(Some("S%")).is_empty());
    assert!(names(Some("NOPE")).is_empty());
    assert!(names(Some("")).is_empty());

    let system = conn.tables(Some("INFORMATION_SCHEMA")).unwrap();
    assert!(system
        .iter()
        .any(|t| t.name == "TABLES" && t.table_type == TableType::SystemTable));

    // The current schema.
    conn.execute("SET SCHEMA 'S2'", ()).unwrap();
    assert_eq!(names(None), ["S2.FOO"]);
}

#[test]
fn table_exists() {
    let conn = setup();
    assert!(conn.table_exists(None, "FOO").unwrap());
    assert!(conn.table_exists(None, "Bar").unwrap());
    assert!(conn.table_exists(Some("S2"), "FOO").unwrap());
    assert!(conn.table_exists(None, "F_O").unwrap());
    assert!(!conn.table_exists(None, "foo").unwrap());
    assert!(!conn.table_exists(None, "BAR").unwrap());
    assert!(!conn.table_exists(Some("S2"), "Bar").unwrap());
    // Names aren't patterns.
    assert!(!conn.table_exists(None, "F%").unwrap());
    assert!(!conn.table_exists(None, "F_").unwrap());
    assert!(!conn.table_exists(None, "").unwrap());
    assert!(!conn.table_exists(Some(""), "FOO").unwrap());
}

#[test]
fn columns() {
    let conn = setup();
    let columns = conn.columns(None, "FOO").unwrap();
    let names: Vec<_> = columns.iter().map(|c| c.name.as_str()).collect();
    assert_eq!(names, ["ID", "A", "B"]);

    assert_eq!(columns[0].ty, Type::Integer);
    assert!(!columns[0].nullable);
    assert_eq!(columns[0].length, None);
    assert_eq!(columns[0].precision_scale, None);
    assert_eq!(columns[0].default, None);

    assert_eq!(columns[1].ty, Type::CharacterVarying);
    assert!(columns[1].nullable);
    assert_eq!(columns[1].length, Some(10));
    assert_eq!(columns[1].default.as_deref(), Some("'x'"));

    assert_eq!(columns[2].ty, Type::Decimal);
    assert_eq!(columns[2].precision_scale, Some((10, 2)));

    let s2 = conn.columns(Some("S2"), "FOO").unwrap();
    assert_eq!(s2.len(), 1);
    assert_eq!(s2[0].ty, Type::Boolean);

    assert!(!s2[0].nullable);

    assert!(conn.columns(None, "NOPE").unwrap().is_empty());
    assert!(conn.columns(None, "F%").unwrap().is_empty());
}

#[test]
fn column_types() {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute(
        "CREATE TABLE t (a SMALLINT, b BIGINT, c REAL, d DOUBLE PRECISION, e NUMERIC(5), \
         f DECFLOAT, g CHAR(3), h CLOB(100), i BINARY(4), j VARBINARY(5), k BLOB(6), \
         l DATE, m TIME(3), n TIME WITH TIME ZONE, o TIMESTAMP, p TIMESTAMP WITH TIME ZONE, \
         q INTERVAL YEAR TO MONTH, r INTERVAL DAY TO SECOND)",
        (),
    )
    .unwrap();
    let columns = conn.columns(None, "T").unwrap();
    let types: Vec<_> = columns.iter().map(|c| c.ty).collect();
    let mut stmt = conn.prepare("SELECT * FROM t").unwrap();
    let query = stmt.query(()).unwrap();
    let expected: Vec<_> = query.columns().iter().map(|c| c.ty()).collect();
    assert_eq!(types, expected);

    let lengths: Vec<_> = columns.iter().map(|c| c.length).collect();
    assert_eq!(
        &lengths[6..11],
        [Some(3), Some(100), Some(4), Some(5), Some(6)]
    );
    assert_eq!(columns[4].precision_scale, Some((5, 0)));
}

#[test]
fn quoted_names() {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute(
        "CREATE TABLE \"we\"\"ird\" (\"a b\" INTEGER, \"c\"\"\" INTEGER)",
        (),
    )
    .unwrap();
    assert!(conn.table_exists(None, "we\"ird").unwrap());
    let names: Vec<_> = conn
        .columns(None, "we\"ird")
        .unwrap()
        .into_iter()
        .map(|c| c.name)
        .collect();
    assert_eq!(names, ["a b", "c\""]);
}

#[test]
fn primary_key() {
    let conn = setup();
    let pk = conn.primary_key(None, "FOO").unwrap().unwrap();
    assert_eq!(pk.name, "FOO_PRIMARY_KEY");
    assert_eq!(pk.columns, ["ID"]);

    let pk = conn.primary_key(None, "KK").unwrap().unwrap();
    assert_eq!(pk.columns, ["B", "A"]);

    let pk = conn.primary_key(Some("S2"), "FOO").unwrap().unwrap();
    assert_eq!(pk.columns, ["Q"]);

    assert_eq!(conn.primary_key(None, "Bar").unwrap(), None);
    assert_eq!(conn.primary_key(None, "NOPE").unwrap(), None);
    assert_eq!(conn.primary_key(None, "").unwrap(), None);
}

#[test]
fn foreign_keys() {
    let conn = setup();
    let fks = conn.foreign_keys(None, "Bar").unwrap();
    assert_eq!(fks.len(), 2);
    assert_eq!(fks[0].name, "BAR_FK");
    assert_eq!(fks[0].columns, ["X"]);
    assert_eq!(fks[0].referenced_schema, "PUBLIC");
    assert_eq!(fks[0].referenced_table, "FOO");
    assert_eq!(fks[0].referenced_columns, ["ID"]);
    assert_eq!(fks[0].on_delete, ForeignKeyAction::Cascade);
    assert_eq!(fks[0].on_update, ForeignKeyAction::NoAction);
    assert_eq!(fks[1].columns, ["Y"]);
    assert_eq!(fks[1].referenced_columns, ["A"]);

    // Keys with more than one column, in the order of the key.
    let fks = conn.foreign_keys(None, "KR").unwrap();
    assert_eq!(fks.len(), 2);
    assert_eq!(fks[0].name, "KR_1");
    assert_eq!(fks[0].columns, ["H", "G"]);
    assert_eq!(fks[0].referenced_columns, ["C", "D"]);
    assert_eq!(fks[0].on_delete, ForeignKeyAction::SetDefault);
    assert_eq!(fks[1].name, "KR_2");
    assert_eq!(fks[1].columns, ["F", "E"]);
    assert_eq!(fks[1].referenced_columns, ["B", "A"]);
    assert_eq!(fks[1].on_update, ForeignKeyAction::SetNull);

    assert!(conn.foreign_keys(None, "FOO").unwrap().is_empty());
    assert!(conn.foreign_keys(None, "NOPE").unwrap().is_empty());
}

#[test]
fn in_a_transaction() {
    let mut conn = setup();
    let tx = conn.transaction().unwrap();
    tx.execute("CREATE TABLE t (x INTEGER)", ()).unwrap();
    assert!(tx.table_exists(None, "T").unwrap());
    tx.rollback().unwrap();
    assert!(!conn.table_exists(None, "T").unwrap());
}

#[test]
fn errors() {
    let conn = setup();
    // A name that's too long for libvsql.
    let long = "X".repeat(40_000);
    let is_hy090 = |e: vsql::Error| e.sqlstate() == Some("HY090");
    assert!(is_hy090(conn.tables(Some(&long)).unwrap_err()));
    assert!(is_hy090(conn.table_exists(None, &long).unwrap_err()));
    assert!(is_hy090(conn.columns(None, &long).unwrap_err()));
    assert!(is_hy090(conn.primary_key(None, &long).unwrap_err()));
    assert!(is_hy090(conn.foreign_keys(None, &long).unwrap_err()));
}

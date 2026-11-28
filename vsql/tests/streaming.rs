// Large values that are written from an `impl Read` (ReadParam) and read as
// an `impl Read` (Statement::query_streaming), in pieces.

use std::io::{self, Read};

use vsql::{params, params_from_iter, Connection, Error, ReadParam, ToSql, Value};

/// More than one piece (8 MiB) of text, with multi-octet characters that
/// cross the pieces.
fn long_text() -> String {
    "aé€😀".repeat(1 << 20) + "end"
}

fn long_binary() -> Vec<u8> {
    (0..(9 << 20) + 3).map(|i| (i % 251) as u8).collect()
}

fn read_all(r: &mut impl Read) -> Vec<u8> {
    let mut v = Vec::new();
    r.read_to_end(&mut v).unwrap();
    v
}

#[test]
fn write_and_read_large_objects() {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute("CREATE TABLE t (id INTEGER, c CLOB, b BLOB)", ())
        .unwrap();
    let text = long_text();
    let binary = long_binary();
    let n = conn
        .execute(
            "INSERT INTO t (id, c, b) VALUES (?, ?, ?)",
            params![
                1,
                ReadParam::text(text.as_bytes()),
                ReadParam::binary(&binary[..])
            ],
        )
        .unwrap();
    assert_eq!(n, 1);

    let mut stmt = conn.prepare("SELECT id, c, b FROM t").unwrap();
    let mut rows = stmt.query_streaming(()).unwrap();
    assert_eq!(rows.columns().len(), 3);
    assert!(format!("{rows:?}").starts_with("StreamingRows"));
    let mut row = rows.next().unwrap().unwrap();
    assert_eq!(row.columns()[1].name(), "C");
    assert!(format!("{row:?}").starts_with("StreamingRow"));
    let id: i64 = row.get(0).unwrap();
    assert_eq!(id, 1);
    let mut reader = row.reader("c").unwrap().unwrap();
    assert!(format!("{reader:?}").contains("\"C\""));
    assert!(read_all(&mut reader) == text.as_bytes());
    // Reading after the end is nothing.
    assert_eq!(reader.read(&mut [0; 4]).unwrap(), 0);
    drop(reader);
    assert!(read_all(&mut row.reader(2).unwrap().unwrap()) == binary);
    // A column can be read again, and with get.
    let c: String = row.get(1).unwrap();
    assert!(c == text);
    assert!(read_all(&mut row.reader(1).unwrap().unwrap()) == text.as_bytes());
    assert!(rows.next().unwrap().is_none());
    assert!(rows.next().unwrap().is_none());
}

#[test]
fn small_strings_into_any_string_column() {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute(
        "CREATE TABLE t (v VARCHAR(10), vb VARBINARY(10), c CLOB, b BLOB)",
        (),
    )
    .unwrap();
    // A tuple, an array, a slice, a Vec, an Option, a Box and params_from_iter.
    let empty: &[u8] = b"";
    conn.execute(
        "INSERT INTO t (v, vb, c, b) VALUES (?, ?, ?, ?)",
        (
            ReadParam::text(&b"abc"[..]),
            &ReadParam::binary(&b"\x00\x01"[..]),
            Some(ReadParam::text(empty)),
            Box::new(ReadParam::binary(empty)),
        ),
    )
    .unwrap();
    conn.execute(
        "INSERT INTO t (c) VALUES (?)",
        [ReadParam::text(&b"array"[..])],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO t (c) VALUES (?)",
        &[ReadParam::text(&b"ref array"[..])],
    )
    .unwrap();
    let v = vec![ReadParam::text(&b"vec"[..])];
    conn.execute("INSERT INTO t (c) VALUES (?)", &v).unwrap();
    conn.execute("INSERT INTO t (c) VALUES (?)", &v[..])
        .unwrap();
    conn.execute(
        "INSERT INTO t (c) VALUES (?)",
        params_from_iter([ReadParam::text(&b"iter"[..])]),
    )
    .unwrap();
    conn.execute("INSERT INTO t (c) VALUES (?)", [None::<ReadParam<&[u8]>>])
        .unwrap();

    let mut stmt = conn.prepare("SELECT v, vb, c, b FROM t").unwrap();
    let mut rows = stmt.query_streaming(()).unwrap();
    let mut row = rows.next().unwrap().unwrap();
    assert_eq!(read_all(&mut row.reader(0).unwrap().unwrap()), b"abc");
    assert_eq!(read_all(&mut row.reader(1).unwrap().unwrap()), b"\x00\x01");
    assert_eq!(read_all(&mut row.reader(2).unwrap().unwrap()), b"");
    assert_eq!(read_all(&mut row.reader(3).unwrap().unwrap()), b"");
    let mut cs = Vec::new();
    while let Some(mut row) = rows.next().unwrap() {
        assert!(row.reader(0).unwrap().is_none());
        cs.push(row.get::<_, Option<String>>(2).unwrap());
    }
    // The Vec's reader was read to the end by the first statement.
    let cs: Vec<_> = cs.iter().map(|c| c.as_deref()).collect();
    assert_eq!(
        cs,
        [
            Some("array"),
            Some("ref array"),
            Some("vec"),
            Some(""),
            Some("iter"),
            None
        ]
    );
}

#[test]
fn query_with_a_read_param() {
    let conn = Connection::open_in_memory().unwrap();
    let s: String = conn
        .query_row(
            "VALUES CAST(? AS CLOB) || 'd'",
            [ReadParam::text(&b"abc"[..])],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(s, "abcd");
}

/// A reader that fails after its first piece, or is interrupted.
struct Flaky {
    calls: usize,
    kind: io::ErrorKind,
}

impl Read for Flaky {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.calls += 1;
        match self.calls {
            1 => {
                buf[0] = b'x';
                Ok(1)
            }
            2 => Err(io::Error::new(self.kind, "flaky")),
            _ => Ok(0),
        }
    }
}

#[test]
fn reader_errors() {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute("CREATE TABLE t (c CLOB)", ()).unwrap();
    let mut stmt = conn.prepare("INSERT INTO t (c) VALUES (?)").unwrap();
    let flaky = |kind: io::ErrorKind| ReadParam::text(Flaky { calls: 0, kind });

    let err = stmt.execute([flaky(io::ErrorKind::Other)]).unwrap_err();
    assert!(matches!(err, Error::ToSqlConversion(_)), "{err:?}");
    // The statement was cancelled, so it can be executed again, and an
    // interrupted read is retried.
    let p = flaky(io::ErrorKind::Interrupted);
    stmt.execute([&p]).unwrap();
    assert_eq!(p.into_inner().calls, 3);

    // A string that isn't UTF-8 is an error from vsql.
    let err = stmt.execute([ReadParam::text(&b"\xff"[..])]).unwrap_err();
    assert!(err.sqlstate().is_some(), "{err:?}");
    stmt.execute([ReadParam::text(&b"ok"[..])]).unwrap();

    let n: i64 = conn
        .query_row("SELECT COUNT(*) FROM t", (), |r| r.get(0))
        .unwrap();
    assert_eq!(n, 2);
}

#[test]
fn to_sql_reads_the_whole_value() {
    assert_eq!(
        ReadParam::text(&b"abc"[..]).to_sql().unwrap(),
        Value::Text("abc".to_string())
    );
    assert_eq!(
        ReadParam::binary(&b"abc"[..]).to_sql().unwrap(),
        Value::Binary(b"abc".to_vec())
    );
    let err = ReadParam::text(&b"\xff"[..]).to_sql().unwrap_err();
    assert!(matches!(err, Error::ToSqlConversion(_)));
    let flaky = ReadParam::binary(Flaky {
        calls: 1,
        kind: io::ErrorKind::Other,
    });
    assert!(matches!(
        flaky.to_sql().unwrap_err(),
        Error::ToSqlConversion(_)
    ));
    assert!(format!("{flaky:?}").contains("binary: true"));

    // Through a reference, an Option and a Box.
    let p = ReadParam::text(&b"a"[..]);
    assert_eq!(
        <&ReadParam<&[u8]> as ToSql>::to_sql(&&p).unwrap(),
        Value::Text("a".to_string())
    );
    assert_eq!(
        Some(ReadParam::text(&b"b"[..])).to_sql().unwrap(),
        Value::Text("b".to_string())
    );
    assert_eq!(None::<ReadParam<&[u8]>>.to_sql().unwrap(), Value::Null);
    assert_eq!(
        Box::new(ReadParam::text(&b"c"[..])).to_sql().unwrap(),
        Value::Text("c".to_string())
    );
}

#[test]
fn exactly_one_piece() {
    // A value that's exactly one piece (8 MiB) long ends with an empty read.
    let conn = Connection::open_in_memory().unwrap();
    let b = vec![1u8; 8 << 20];
    let n: i64 = conn
        .query_row(
            "VALUES OCTET_LENGTH(CAST(? AS BLOB))",
            [ReadParam::binary(&b[..])],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(n, 8 << 20);
}

#[test]
fn partial_reads() {
    let conn = Connection::open_in_memory().unwrap();
    let text = long_text();
    let mut stmt = conn
        .prepare("VALUES ROW(CAST(? AS CLOB), 1, CAST(NULL AS BLOB), DATE '2024-01-02', CAST(1.5 AS REAL), CAST(2.5 AS DOUBLE PRECISION))")
        .unwrap();
    let mut rows = stmt
        .query_streaming([ReadParam::text(text.as_bytes())])
        .unwrap();
    let mut row = rows.next().unwrap().unwrap();

    // A column that a reader didn't finish is read from the start again.
    let mut buf = [0u8; 3];
    row.reader(0)
        .unwrap()
        .unwrap()
        .read_exact(&mut buf)
        .unwrap();
    assert_eq!(&buf, b"a\xc3\xa9");
    let c: String = row.get(0).unwrap();
    assert!(c == text);

    row.reader(0)
        .unwrap()
        .unwrap()
        .read_exact(&mut buf)
        .unwrap();
    assert!(read_all(&mut row.reader(0).unwrap().unwrap()) == text.as_bytes());

    // So it is after a different column is read.
    row.reader(0)
        .unwrap()
        .unwrap()
        .read_exact(&mut buf)
        .unwrap();
    assert_eq!(row.get::<_, i64>(1).unwrap(), 1);
    assert!(read_all(&mut row.reader(0).unwrap().unwrap()) == text.as_bytes());

    // Any value can be read, as the string that vsql gives for it.
    assert_eq!(read_all(&mut row.reader(1).unwrap().unwrap()), b"1");
    assert_eq!(row.get::<_, f64>(4).unwrap(), 1.5);
    assert_eq!(row.get::<_, f64>(5).unwrap(), 2.5);
    assert!(row.reader(2).unwrap().is_none());
    assert_eq!(
        read_all(&mut row.reader(3).unwrap().unwrap()),
        b"DATE '2024-01-02'"
    );

    assert!(matches!(
        row.reader(6).unwrap_err(),
        Error::InvalidColumnIndex(6)
    ));
    assert!(matches!(
        row.get::<_, i64>("x").unwrap_err(),
        Error::InvalidColumnName(_)
    ));
    assert!(matches!(
        row.get::<_, i64>(0).unwrap_err(),
        Error::FromSqlConversion { index: 0, .. }
    ));
    assert!(matches!(
        row.get::<_, i64>(3).unwrap_err(),
        Error::FromSqlConversion { index: 3, .. }
    ));
}

#[test]
fn statement_without_rows() {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute("CREATE TABLE t (c CLOB)", ()).unwrap();
    let mut stmt = conn.prepare("INSERT INTO t (c) VALUES ('a')").unwrap();
    let mut rows = stmt.query_streaming(()).unwrap();
    assert!(rows.columns().is_empty());
    assert!(rows.next().unwrap().is_none());
    drop(rows);

    let mut stmt = conn.prepare("SELECT c FROM t WHERE c = ?").unwrap();
    let err = stmt.query_streaming(()).unwrap_err();
    assert!(matches!(err, Error::InvalidParameterCount(0, 1)));
    // The cursor of rows that are dropped before the end is closed.
    let mut rows = stmt.query_streaming(["a"]).unwrap();
    assert!(rows.next().unwrap().is_some());
    drop(rows);
    assert_eq!(stmt.query_streaming(["a"]).unwrap().columns().len(), 1);
}

// The uuid, serde_json and bigdecimal features.

#[cfg(feature = "uuid")]
#[test]
fn uuid() {
    use uuid::Uuid;
    use vsql::types::FromSqlError;
    use vsql::{Connection, FromSql, Value};

    let conn = Connection::open_in_memory().unwrap();
    let s = "67e55044-10b1-426f-9247-bb680e5fe0c8";
    let u = Uuid::parse_str(s).unwrap();

    conn.execute("CREATE TABLE foo (id BINARY(16))", ())
        .unwrap();
    conn.execute("INSERT INTO foo (id) VALUES (?)", [u])
        .unwrap();
    let got: Uuid = conn
        .query_row("SELECT id FROM foo", (), |r| r.get(0))
        .unwrap();
    assert_eq!(got, u);

    // vsql's UUID functions, and the string.
    let (binary, text): (Uuid, Uuid) = conn
        .query_row(
            &format!("VALUES ROW(UUID_TO_BINARY('{s}'), '{s}')"),
            (),
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap();
    assert_eq!(binary, u);
    assert_eq!(text, u);
    let got: Uuid = conn
        .query_row("SELECT BINARY_TO_UUID(id) FROM foo", (), |r| r.get(0))
        .unwrap();
    assert_eq!(got, u);

    let other = |v| matches!(Uuid::from_sql(&v), Err(FromSqlError::Other(_)));
    assert!(other(Value::Binary(vec![1, 2, 3])));
    assert!(other(Value::Text("nope".to_string())));
    assert!(matches!(
        Uuid::from_sql(&Value::Integer(1)),
        Err(FromSqlError::InvalidType)
    ));
}

#[cfg(feature = "serde_json")]
#[test]
fn serde_json() {
    use serde_json::json;
    use vsql::types::FromSqlError;
    use vsql::{Connection, FromSql, Value};

    let conn = Connection::open_in_memory().unwrap();
    let want = json!({"a": [1, 2.5, "x", null, true], "b": {"c": "d"}});
    conn.execute("CREATE TABLE foo (j VARCHAR(1000))", ())
        .unwrap();
    conn.execute("INSERT INTO foo (j) VALUES (?)", [&want])
        .unwrap();
    let got: serde_json::Value = conn
        .query_row("SELECT j FROM foo", (), |r| r.get(0))
        .unwrap();
    assert_eq!(got, want);
    let got: serde_json::Value = conn
        .query_row("VALUES '[1, \"two\"]'", (), |r| r.get(0))
        .unwrap();
    assert_eq!(got, json!([1, "two"]));

    assert!(matches!(
        serde_json::Value::from_sql(&Value::Text("{".to_string())),
        Err(FromSqlError::Other(_))
    ));
    assert!(matches!(
        serde_json::Value::from_sql(&Value::Integer(1)),
        Err(FromSqlError::InvalidType)
    ));
}

#[cfg(feature = "bigdecimal")]
#[test]
fn bigdecimal() {
    use std::str::FromStr;

    use bigdecimal::BigDecimal;
    use vsql::types::FromSqlError;
    use vsql::{Connection, FromSql, Value};

    let conn = Connection::open_in_memory().unwrap();
    // More digits than rust_decimal, and ones that BigDecimal writes with an
    // exponent.
    for s in [
        "1.25",
        "-0.05",
        "0",
        "1234567890123456789012345678901234567890.0123456789",
        "1E-10",
        "-1.5E+30",
    ] {
        let d = BigDecimal::from_str(s).unwrap();
        let got: BigDecimal = conn
            .query_row("VALUES CAST(? AS NUMERIC(60, 10))", [&d], |r| r.get(0))
            .unwrap();
        assert_eq!(got, d, "{s}");
    }
    let got: BigDecimal = conn.query_row("VALUES 12", (), |r| r.get(0)).unwrap();
    assert_eq!(got, BigDecimal::from(12));

    // A DECFLOAT, which may have an exponent.
    for (sql, want) in [("1.25", "1.25"), ("1.5E20", "150000000000000000000")] {
        let got: BigDecimal = conn
            .query_row(&format!("VALUES CAST({sql} AS DECFLOAT)"), (), |r| r.get(0))
            .unwrap();
        assert_eq!(got, BigDecimal::from_str(want).unwrap(), "{sql}");
    }

    let other = |v| matches!(BigDecimal::from_sql(&v), Err(FromSqlError::Other(_)));
    assert!(other(Value::Numeric("x".to_string())));
    assert!(other(Value::DecFloat("x".to_string())));
    assert!(matches!(
        BigDecimal::from_sql(&Value::Boolean(true)),
        Err(FromSqlError::InvalidType)
    ));
}

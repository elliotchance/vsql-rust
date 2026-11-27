// Deserializing rows with serde (the serde feature).

#![cfg(feature = "serde")]

use std::collections::{BTreeMap, HashMap};
use std::fmt;

use serde::de::{self, Deserializer, IgnoredAny, MapAccess, Visitor};
use serde::Deserialize;
use vsql::types::FromSqlError;
use vsql::{from_row, Connection, Error, Row};

fn memory() -> Connection {
    Connection::open_in_memory().unwrap()
}

fn row(sql: &str) -> Row {
    memory().query_row(sql, (), |row| Ok(row.clone())).unwrap()
}

/// What deserialize_any visited.
#[derive(Debug, PartialEq)]
struct Any(String);

struct AnyVisitor;

impl<'de> Visitor<'de> for AnyVisitor {
    type Value = Any;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("anything")
    }

    fn visit_unit<E>(self) -> Result<Any, E> {
        Ok(Any("unit".to_string()))
    }

    fn visit_bool<E>(self, v: bool) -> Result<Any, E> {
        Ok(Any(format!("bool {v}")))
    }

    fn visit_i64<E>(self, v: i64) -> Result<Any, E> {
        Ok(Any(format!("i64 {v}")))
    }

    fn visit_f64<E>(self, v: f64) -> Result<Any, E> {
        Ok(Any(format!("f64 {v}")))
    }

    fn visit_borrowed_str<E>(self, v: &'de str) -> Result<Any, E> {
        Ok(Any(format!("borrowed str {v}")))
    }

    fn visit_str<E>(self, v: &str) -> Result<Any, E> {
        Ok(Any(format!("str {v}")))
    }

    fn visit_string<E>(self, v: String) -> Result<Any, E> {
        Ok(Any(format!("string {v}")))
    }

    fn visit_borrowed_bytes<E>(self, v: &'de [u8]) -> Result<Any, E> {
        Ok(Any(format!("borrowed bytes {v:?}")))
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Any, A::Error> {
        let mut keys = Vec::new();
        while let Some((key, _)) = map.next_entry::<String, IgnoredAny>()? {
            keys.push(key);
        }
        Ok(Any(format!("map {keys:?}")))
    }
}

impl<'de> Deserialize<'de> for Any {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Any, D::Error> {
        d.deserialize_any(AnyVisitor)
    }
}

#[derive(Debug, Deserialize, PartialEq)]
struct Person<'a> {
    id: i64,
    name: String,
    #[serde(borrow)]
    nickname: &'a str,
    email: Option<String>,
    score: f64,
    active: bool,
    age: u8,
    big: i128,
    photo: Vec<u8>,
    #[serde(borrow)]
    thumbnail: &'a [u8],
    born: String,
    #[serde(default)]
    missing: i32,
    #[serde(rename = "Quoted")]
    quoted: i16,
}

#[test]
fn struct_fields() {
    let conn = memory();
    conn.execute(
        "CREATE TABLE person (id INTEGER, name VARCHAR(10), nickname VARCHAR(10), \
         email VARCHAR(20), score DOUBLE PRECISION, active BOOLEAN, age SMALLINT, \
         big NUMERIC(30), photo VARBINARY(4), thumbnail VARBINARY(4), born DATE, \
         \"Quoted\" INTEGER, extra INTEGER)",
        (),
    )
    .unwrap();
    conn.execute(
        "INSERT INTO person (id, name, nickname, score, active, age, big, photo, thumbnail, \
         born, \"Quoted\", extra) VALUES (1, 'Robert', 'Bob', 1.5, TRUE, 42, \
         123456789012345678901234567890, X'0102', X'03', DATE '2000-01-02', 7, 8)",
        (),
    )
    .unwrap();

    let row = conn
        .query_row("SELECT * FROM person", (), |row| Ok(row.clone()))
        .unwrap();
    let person: Person = from_row(&row).unwrap();
    assert_eq!(
        person,
        Person {
            id: 1,
            name: "Robert".to_string(),
            nickname: "Bob",
            email: None,
            score: 1.5,
            active: true,
            age: 42,
            big: 123456789012345678901234567890,
            photo: vec![1, 2],
            thumbnail: &[3],
            born: "2000-01-02".to_string(),
            missing: 0,
            quoted: 7,
        }
    );

    #[derive(Debug, Deserialize, PartialEq)]
    struct Name {
        name: String,
    }
    let mut stmt = conn.prepare("SELECT name FROM person").unwrap();
    let names = stmt
        .query_map((), |row| from_row::<Name>(row))
        .unwrap()
        .collect::<vsql::Result<Vec<_>>>()
        .unwrap();
    assert_eq!(
        names,
        [Name {
            name: "Robert".to_string()
        }]
    );
}

#[test]
fn struct_errors() {
    #[derive(Debug, Deserialize)]
    #[allow(dead_code)]
    struct X {
        x: u8,
    }

    let err = from_row::<X>(&row("SELECT * FROM (VALUES ROW(1)) AS t (y)")).unwrap_err();
    assert!(matches!(err, Error::Deserialize(_)), "{err:?}");
    assert_eq!(err.to_string(), "missing field `x`");

    // The value is out of range.
    let err = from_row::<X>(&row("SELECT * FROM (VALUES ROW(300)) AS t (x)")).unwrap_err();
    assert!(
        matches!(
            err,
            Error::FromSqlConversion {
                index: 0,
                source: FromSqlError::OutOfRange,
                ..
            }
        ),
        "{err:?}"
    );

    // An invalid type.
    #[derive(Debug, Deserialize)]
    #[allow(dead_code)]
    struct Y {
        y: String,
    }
    let err = from_row::<Y>(&row("SELECT * FROM (VALUES ROW(1, 2)) AS t (x, y)")).unwrap_err();
    match err {
        Error::FromSqlConversion {
            index,
            name,
            source,
            ..
        } => {
            assert_eq!((index, name.as_str()), (1, "Y"));
            assert_eq!(
                source.to_string(),
                "invalid type: integer `2`, expected a string"
            );
        }
        err => panic!("{err:?}"),
    }

    // A sequence that isn't a binary string.
    #[derive(Debug, Deserialize)]
    #[allow(dead_code)]
    struct Z {
        z: Vec<i64>,
    }
    let err = from_row::<Z>(&row("SELECT * FROM (VALUES ROW('a')) AS t (z)")).unwrap_err();
    assert!(
        matches!(err, Error::FromSqlConversion { index: 0, .. }),
        "{err:?}"
    );
}

#[test]
fn tuples_and_maps() {
    let r = row("VALUES ROW(1, 'a')");
    assert_eq!(from_row::<(i64, String)>(&r).unwrap(), (1, "a".to_string()));
    assert_eq!(from_row::<(i64, &str)>(&r).unwrap(), (1, "a"));

    #[derive(Debug, Deserialize, PartialEq)]
    struct Pair(i64, String);
    assert_eq!(from_row::<Pair>(&r).unwrap(), Pair(1, "a".to_string()));

    let ints = row("VALUES ROW(1, 2, 3)");
    assert_eq!(from_row::<Vec<i64>>(&ints).unwrap(), [1, 2, 3]);
    // Too many columns for the tuple.
    let err = from_row::<(i64, i64)>(&ints).unwrap_err();
    assert_eq!(err.to_string(), "the row has 3 columns, not 2");

    let map: HashMap<String, i64> = from_row(&ints).unwrap();
    assert_eq!(
        map,
        HashMap::from([
            ("COL1".to_string(), 1),
            ("COL2".to_string(), 2),
            ("COL3".to_string(), 3)
        ])
    );
    let map: BTreeMap<&str, i64> = from_row(&ints).unwrap();
    assert_eq!(
        map.keys().copied().collect::<Vec<_>>(),
        ["COL1", "COL2", "COL3"]
    );

    // deserialize_any is a map.
    assert_eq!(
        from_row::<Any>(&r).unwrap(),
        Any("map [\"COL1\", \"COL2\"]".to_string())
    );
}

#[test]
fn single_column() {
    assert_eq!(from_row::<i64>(&row("VALUES 12")).unwrap(), 12);
    assert_eq!(from_row::<i8>(&row("VALUES -1")).unwrap(), -1);
    assert_eq!(from_row::<i16>(&row("VALUES -2")).unwrap(), -2);
    assert_eq!(from_row::<i32>(&row("VALUES -3")).unwrap(), -3);
    assert_eq!(from_row::<u16>(&row("VALUES 4")).unwrap(), 4);
    assert_eq!(from_row::<u32>(&row("VALUES 5")).unwrap(), 5);
    assert_eq!(from_row::<u64>(&row("VALUES 6")).unwrap(), 6);
    assert_eq!(from_row::<u128>(&row("VALUES 7")).unwrap(), 7);
    assert_eq!(
        from_row::<f32>(&row("VALUES CAST(1.5 AS REAL)")).unwrap(),
        1.5
    );
    assert!(!from_row::<bool>(&row("VALUES FALSE")).unwrap());
    assert_eq!(from_row::<char>(&row("VALUES 'x'")).unwrap(), 'x');
    assert_eq!(from_row::<&str>(&row("VALUES 'abc'")).unwrap(), "abc");
    assert_eq!(
        from_row::<String>(&row("VALUES CAST(1.25 AS NUMERIC(3, 2))")).unwrap(),
        "1.25"
    );
    assert_eq!(
        from_row::<&[u8]>(&row("VALUES CAST(X'0a' AS VARBINARY(1))")).unwrap(),
        [10]
    );
    assert_eq!(
        from_row::<Option<i64>>(&row("VALUES CAST(NULL AS INTEGER)")).unwrap(),
        None
    );
    assert_eq!(from_row::<Option<i64>>(&row("VALUES 1")).unwrap(), Some(1));
    from_row::<()>(&row("VALUES CAST(NULL AS INTEGER)")).unwrap();

    #[derive(Debug, Deserialize, PartialEq)]
    struct Unit;
    assert_eq!(
        from_row::<Unit>(&row("VALUES CAST(NULL AS INTEGER)")).unwrap(),
        Unit
    );

    #[derive(Debug, Deserialize, PartialEq)]
    struct Id(i64);
    assert_eq!(from_row::<Id>(&row("VALUES 3")).unwrap(), Id(3));

    #[derive(Debug, Deserialize, PartialEq)]
    enum Color {
        Red,
        Green,
    }
    assert_eq!(
        from_row::<Color>(&row("VALUES 'Green'")).unwrap(),
        Color::Green
    );
    let err = from_row::<Color>(&row("VALUES 'Blue'")).unwrap_err();
    assert_eq!(
        err.to_string(),
        "column 0 (COL1, CHARACTER): unknown variant `Blue`, expected `Red` or `Green`"
    );
    assert!(matches!(
        from_row::<Color>(&row("VALUES 1")),
        Err(Error::FromSqlConversion {
            source: FromSqlError::InvalidType,
            ..
        })
    ));

    IgnoredAny::deserialize(&row("VALUES 1")).unwrap();
    assert_eq!(
        (&row("VALUES 'a'"))
            .deserialize_identifier(AnyVisitor)
            .unwrap(),
        Any("borrowed str a".to_string())
    );

    // A NULL is only an Option or ().
    assert!(matches!(
        from_row::<()>(&row("VALUES 1")),
        Err(Error::FromSqlConversion {
            source: FromSqlError::InvalidType,
            ..
        })
    ));
    assert!(matches!(
        from_row::<i64>(&row("VALUES CAST(NULL AS INTEGER)")),
        Err(Error::FromSqlConversion {
            source: FromSqlError::InvalidType,
            ..
        })
    ));
    // A value out of range, not a message.
    assert!(matches!(
        from_row::<u8>(&row("VALUES 300")),
        Err(Error::FromSqlConversion {
            source: FromSqlError::OutOfRange,
            ..
        })
    ));

    let err = from_row::<i64>(&row("VALUES ROW(1, 2)")).unwrap_err();
    assert_eq!(err.to_string(), "the row has 2 columns, not 1");
}

#[test]
fn newtypes() {
    #[derive(Debug, Deserialize, PartialEq)]
    struct Name(String);

    #[derive(Debug, Deserialize, PartialEq)]
    struct Named {
        name: Name,
    }

    #[derive(Debug, Deserialize, PartialEq)]
    struct Wrapper(Named);

    assert_eq!(
        from_row::<Wrapper>(&row("SELECT * FROM (VALUES ROW('a')) AS t (name)")).unwrap(),
        Wrapper(Named {
            name: Name("a".to_string())
        })
    );
}

#[test]
fn deserialize_any() {
    #[derive(Debug, Deserialize)]
    struct All {
        a: Any,
        b: Any,
        c: Any,
        d: Any,
        e: Any,
        f: Any,
        g: Any,
        h: Any,
        i: Any,
        j: Any,
        k: Any,
    }

    let all: All = from_row(&row(
        "SELECT * FROM (VALUES ROW(CAST(NULL AS INTEGER), TRUE, 1, CAST(1.5 AS DOUBLE PRECISION), \
         CAST(1.25 AS NUMERIC(3, 2)), 'x', CAST(X'01' AS VARBINARY(1)), DATE '2000-01-02', \
         TIME '15:04:05', TIMESTAMP '2000-01-02 15:04:05', INTERVAL '3' DAY)) \
         AS t (a, b, c, d, e, f, g, h, i, j, k)",
    ))
    .unwrap();
    let got: Vec<String> = [
        all.a, all.b, all.c, all.d, all.e, all.f, all.g, all.h, all.i, all.j, all.k,
    ]
    .into_iter()
    .map(|a| a.0)
    .collect();
    assert_eq!(
        got,
        [
            "unit",
            "bool true",
            "i64 1",
            "f64 1.5",
            "borrowed str 1.25",
            "borrowed str x",
            "borrowed bytes [1]",
            "string 2000-01-02",
            "string 15:04:05.000000",
            "string 2000-01-02 15:04:05.000000",
            "string INTERVAL '3' DAY(2)",
        ]
    );
}

#[test]
fn error_is_serde_error() {
    let err = <Error as de::Error>::custom("oops");
    assert_eq!(err.to_string(), "oops");
}

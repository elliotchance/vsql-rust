// Strings and binary strings that are longer than a CHARACTER VARYING or
// BINARY VARYING can be (10,485,760) are CLOB and BLOB parameters.

use vsql::Connection;

const MAX: usize = 10_485_760;

#[test]
fn long_strings() {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute("CREATE TABLE c (x CLOB)", ()).unwrap();
    for n in [MAX, MAX + 1] {
        let s = "é".repeat(n);
        conn.execute("DELETE FROM c", ()).unwrap();
        conn.execute("INSERT INTO c (x) VALUES (?)", [&s]).unwrap();
        let got: String = conn.query_row("SELECT x FROM c", (), |r| r.get(0)).unwrap();
        assert_eq!(got.chars().count(), n);
        assert!(got == s);
    }
}

#[test]
fn long_binary_strings() {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute("CREATE TABLE b (x BLOB)", ()).unwrap();
    for n in [MAX, MAX + 1] {
        let b = vec![7u8; n];
        conn.execute("DELETE FROM b", ()).unwrap();
        conn.execute("INSERT INTO b (x) VALUES (?)", [&b]).unwrap();
        let got: Vec<u8> = conn.query_row("SELECT x FROM b", (), |r| r.get(0)).unwrap();
        assert!(got == b);
    }
}

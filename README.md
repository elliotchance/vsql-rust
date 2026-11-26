# vsql-rust

A Rust crate for [vsql](https://github.com/elliotchance/vsql), a single file or
in-memory SQL database. libvsql (the vsql library) is compiled into the crate,
so there's nothing else to install.

```toml
[dependencies]
vsql = "1"
```

```rust
use vsql::Connection;

fn main() -> vsql::Result<()> {
    let conn = Connection::open("test.vsql")?;
    conn.execute("CREATE TABLE foo (x INTEGER, y VARCHAR(10))", ())?;
    conn.execute("INSERT INTO foo (x, y) VALUES (?, ?)", (1, "a"))?;

    let mut stmt = conn.prepare("SELECT x, y FROM foo")?;
    for row in stmt.query(())? {
        let row = row?;
        let x: i64 = row.get(0)?;
        let y: String = row.get("y")?;
        println!("{x} {y}");
    }

    Ok(())
}
```

`Connection::open` creates the file if it doesn't exist, and
`Connection::open_in_memory` is a database that only exists for the life of
the connection. The API follows [rusqlite](https://docs.rs/rusqlite).

See the [documentation](https://docs.rs/vsql) for parameters, transactions and
the Rust type of each column type, and
[vsql/examples/main.rs](vsql/examples/main.rs) for a complete program:

```sh
cargo run --example main
```

## Transactions

Each statement is committed when it finishes (or when the rows of a query are
dropped), unless it's in a transaction:

```rust
let tx = conn.transaction()?;
tx.execute("INSERT INTO foo (x) VALUES (?)", [2])?;
tx.commit()?; // Or it's rolled back when it's dropped.
```

Transactions are `READ COMMITTED` unless another isolation level (or a read
only transaction) is given with `transaction_with`. vsql's transactions are
`READ COMMITTED` or `SERIALIZABLE`: `READ UNCOMMITTED` is `READ COMMITTED` (and
read only), and `REPEATABLE READ` is `SERIALIZABLE`.

A savepoint can be rolled back to without rolling back the whole transaction:

```rust
let mut tx = conn.transaction()?;
tx.execute("INSERT INTO foo (x) VALUES (?)", [3])?;
{
    let mut sp = tx.savepoint()?;
    sp.execute("INSERT INTO foo (x) VALUES (?)", [4])?;
    sp.rollback()?; // Undoes 4, the savepoint is still active.
    sp.execute("INSERT INTO foo (x) VALUES (?)", [5])?;
    sp.commit()?; // Or it's rolled back to (and released) when it's dropped.
}
tx.commit()?; // 3 and 5.
```

Savepoints can be nested (`sp.savepoint()`), and `conn.savepoint()` starts a
transaction with a savepoint, which is committed when the savepoint is.

## Types

| SQL                              | Rust                                           |
|----------------------------------|------------------------------------------------|
| `SMALLINT`, `INTEGER`, `BIGINT`  | any integer type (if it's in range)            |
| `REAL`, `DOUBLE PRECISION`       | `f64`, `f32`                                   |
| `DECIMAL`, `NUMERIC`, `DECFLOAT` | `String` (exact), `f64`, `rust_decimal::Decimal` |
| `CHARACTER`, `VARCHAR`, `CLOB`   | `String`                                       |
| `BINARY`, `VARBINARY`, `BLOB`    | `Vec<u8>`, `[u8; N]` (`&[u8]` as a parameter)  |
| `BOOLEAN`                        | `bool`                                         |
| `DATE`                           | `vsql::Date`, `chrono::NaiveDate`              |
| `TIME`                           | `vsql::Time`, `chrono::NaiveTime`              |
| `TIMESTAMP`                      | `vsql::Timestamp`, `chrono::NaiveDateTime`     |
| `TIMESTAMP WITH TIME ZONE`       | `vsql::Timestamp`, `chrono::DateTime<FixedOffset>`, `chrono::DateTime<Utc>` |
| `INTERVAL ...`                   | `vsql::Interval`                               |

A `DECFLOAT` is a `vsql::Value::DecFloat` (a string such as `"1.5E+20"`) as a
parameter. A `vsql::Interval` is a number of months (`YEAR`, `MONTH` or
`YEAR TO MONTH`) or microseconds (the other fields):

```rust
let i: vsql::Interval = conn.query_row("VALUES INTERVAL '1 2' DAY TO HOUR", (), |r| r.get(0))?;
assert_eq!(i.microseconds, 26 * 3600 * 1_000_000);
println!("{i}"); // INTERVAL '1 02' DAY(2) TO HOUR
```

A `NULL` is an `Option`, and any value can be read as a `vsql::Value`. The
`chrono` and `rust_decimal` types need those features:

```toml
vsql = { version = "1", features = ["chrono", "rust_decimal"] }
```

## Errors

An error from vsql is `vsql::Error::Sql`, with its SQLSTATE:

```rust
match conn.execute("SELECT * FROM nope", ()) {
    Err(e) if e.sqlstate() == Some("42P01") => println!("no such table"),
    other => other?,
};
```

`e.diagnostics()` has the constraint, table or column that the error is about:

```rust
if let Err(e) = conn.execute("INSERT INTO foo (id) VALUES (1)", ()) {
    if let Some(name) = e.diagnostics().and_then(|d| d.constraint_name.as_deref()) {
        println!("violates {name}");
    }
}
```

## Version

`vsql::version()` is the version of vsql that's compiled into the crate (such
as `"1.7.58"`), and `vsql::build_info()` has how it was built.

## Threads

Each connection has its own libvsql environment, so different connections can
be used from different threads at the same time. A `Connection` can be moved
to another thread (it's `Send`), but not shared (it isn't `Sync`).

## Crates

- [`vsql`](vsql): the API.
- [`vsql-sys`](vsql-sys): the raw libvsql bindings, which compile libvsql from
  the amalgamation.

## Requirements

macOS (ARM64 and x86-64), Linux (ARM64 and x86-64) and Windows (x86-64) are
supported, with Rust 1.75 or later and a C compiler. The first build compiles
libvsql, which takes about 30 seconds, after that it's cached by Cargo.

On Windows (the MSVC target), libvsql is compiled with clang-cl, since the C
that V generates doesn't compile with MSVC. Install
[LLVM](https://llvm.org) (or the "C++ Clang tools for Windows" component of
Visual Studio). clang-cl is found on the `PATH` (or in
`C:\Program Files\LLVM\bin`), or set `VSQL_CLANG_CL` to its path.

## Updating vsql

`vsql-sys/amalgamation` is the libvsql C from a vsql release
(`vsql-VERSION-amalgamation.zip`). To update it to the latest release:

```sh
scripts/update-amalgamation.sh
cargo test --all-features
```

Or give a version (`scripts/update-amalgamation.sh v1.2.3`), or a zip that has
already been downloaded.

## Releasing

`vsql` and `vsql-sys` have the same version (`version` in `Cargo.toml`).
Update it, then publish a GitHub release with the tag `vVERSION` (such as
`v1.0.1`). The release workflow checks that the tag matches the version, tests,
and publishes both crates to crates.io.

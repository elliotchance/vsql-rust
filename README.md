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

vsql's transactions are `READ COMMITTED`.

## Types

| SQL                              | Rust                                           |
|----------------------------------|------------------------------------------------|
| `SMALLINT`, `INTEGER`, `BIGINT`  | any integer type (if it's in range)            |
| `REAL`, `DOUBLE PRECISION`       | `f64`, `f32`                                   |
| `DECIMAL`, `NUMERIC`             | `String` (exact), `f64`, `rust_decimal::Decimal` |
| `CHARACTER`, `VARCHAR`           | `String`                                       |
| `BOOLEAN`                        | `bool`                                         |
| `DATE`                           | `vsql::Date`, `chrono::NaiveDate`              |
| `TIME`                           | `vsql::Time`, `chrono::NaiveTime`              |
| `TIMESTAMP`                      | `vsql::Timestamp`, `chrono::NaiveDateTime`     |
| `TIMESTAMP WITH TIME ZONE`       | `vsql::Timestamp`, `chrono::DateTime<FixedOffset>`, `chrono::DateTime<Utc>` |

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

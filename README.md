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
`execute` and `prepare` take one statement, and `execute_batch` executes SQL
with more than one statement (separated by `;`), such as a schema.

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

`tx.set_drop_behavior(vsql::DropBehavior::Commit)` commits a transaction (or
savepoint) when it's dropped instead (or `Ignore` or `Panic`), and
`tx.finish()` ends it that way and returns the error that dropping it
ignores. `conn.unchecked_transaction()` starts a transaction through a
`&Connection` (such as one in an `Rc`), which is an error if the connection is
already in a transaction.

## Types

| SQL                              | Rust                                           |
|----------------------------------|------------------------------------------------|
| `SMALLINT`, `INTEGER`, `BIGINT`  | any integer type (if it's in range)            |
| `REAL`, `DOUBLE PRECISION`       | `f64`, `f32`                                   |
| `DECIMAL`, `NUMERIC`, `DECFLOAT` | `String` (exact), `f64`, `rust_decimal::Decimal`, any integer type (if it's an integer in range) |
| `CHARACTER`, `VARCHAR`, `CLOB`   | `String`                                       |
| `BINARY`, `VARBINARY`, `BLOB`    | `Vec<u8>`, `[u8; N]` (`&[u8]` as a parameter)  |
| `BOOLEAN`                        | `bool`                                         |
| `DATE`                           | `vsql::Date`, `chrono::NaiveDate`, `time::Date` |
| `TIME`                           | `vsql::Time`, `chrono::NaiveTime`, `time::Time` |
| `TIMESTAMP`                      | `vsql::Timestamp`, `chrono::NaiveDateTime`, `time::PrimitiveDateTime` |
| `TIMESTAMP WITH TIME ZONE`       | `vsql::Timestamp`, `chrono::DateTime<FixedOffset>`, `chrono::DateTime<Utc>`, `time::OffsetDateTime` |
| `INTERVAL ...`                   | `vsql::Interval`, `time::Duration` (day-time)  |

A `DECFLOAT` is a `vsql::Value::DecFloat` (a string such as `"1.5E+20"`) as a
parameter. A `vsql::Interval` is a number of months (`YEAR`, `MONTH` or
`YEAR TO MONTH`) or microseconds (the other fields):

```rust
let i: vsql::Interval = conn.query_row("VALUES INTERVAL '1 2' DAY TO HOUR", (), |r| r.get(0))?;
assert_eq!(i.microseconds, 26 * 3600 * 1_000_000);
println!("{i}"); // INTERVAL '1 02' DAY(2) TO HOUR
```

A day-time interval is also a `std::time::Duration` (if it isn't negative) or
a `chrono::Duration`, and a year-month interval is a `chrono::Months`. As a
parameter, these are a `DAY TO SECOND` or `YEAR TO MONTH` interval, which is
cast to the type it's used as:

```rust
let d = std::time::Duration::from_secs(5400);
let i: vsql::Interval = conn.query_row("VALUES CAST(? AS INTERVAL HOUR TO MINUTE)", [d], |r| r.get(0))?;
println!("{i}"); // INTERVAL '1:30' HOUR(2) TO MINUTE
```

A `NULL` is an `Option`, and any value can be read as a `vsql::Value`. The
`chrono`, `time` and `rust_decimal` types need those features:

```toml
vsql = { version = "1", features = ["chrono", "time", "rust_decimal"] }
```

## Schema

The tables, columns and keys of a schema (`None` is the current schema), with
names as they're stored (upper case unless they were quoted):

```rust
for table in conn.tables(None)? {
    for column in conn.columns(None, &table.name)? {
        println!("{}.{} {:?}", table.name, column.name, column.ty);
    }
}
assert!(conn.table_exists(None, "FOO")?);
let pk = conn.primary_key(None, "FOO")?; // Option<PrimaryKey>
let fks = conn.foreign_keys(None, "FOO")?; // Vec<ForeignKey>
```

## Errors

An error from vsql is `vsql::Error::Sql`, with its SQLSTATE:

```rust
match conn.execute("DELETE FROM nope", ()) {
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

## Connection pools

The `r2d2` and `deadpool` features have `vsql::pool::VsqlConnectionManager`,
which opens connections to a database file for a pool (an in-memory database
can't be pooled, since each connection to memory is a different database):

```rust
let pool = r2d2::Pool::new(vsql::pool::VsqlConnectionManager::file("my.vsql"))?;
let conn = pool.get()?;
conn.execute("INSERT INTO foo (x) VALUES (1)", ())?;
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

macOS, Linux (glibc or musl) and Windows (MSVC, and GNU on x86-64) are
supported, on ARM64 and x86-64, with Rust 1.75 or later and a C compiler. The first build compiles
libvsql, which takes about 30 seconds, after that it's cached by Cargo. Other
OSes (such as FreeBSD) can't be built, since the vsql amalgamation only has the
C that V generates for these.

For the musl targets (such as `x86_64-unknown-linux-musl`), the C compiler is
`musl-gcc` (`musl-tools` on Debian and Ubuntu, or `gcc` and `musl-dev` on
Alpine). For the GNU target on Windows (`x86_64-pc-windows-gnu`), it's MinGW's
`gcc`.

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

`vsql-sys/src/lib.rs` declares everything in `amalgamation/libvsql.h` by hand.
If the new header adds (or changes) a type, constant or routine, the `header`
test of `vsql-sys` fails until `lib.rs` is updated to match.

## Releasing

Publish a GitHub release with the tag `vVERSION` (such as `v1.0.1`). The
release workflow sets the version of `vsql` and `vsql-sys` from the tag (it's
`0.0.0` in `Cargo.toml`), tests, and publishes both crates to crates.io.

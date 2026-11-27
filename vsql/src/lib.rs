//! An embedded SQL database: [vsql](https://github.com/elliotchance/vsql), a
//! single file or in-memory SQL database. libvsql (the vsql library) is
//! compiled into the crate (see [`vsql-sys`](https://docs.rs/vsql-sys)), so
//! there's nothing else to install.
//!
//! ```
//! use vsql::Connection;
//!
//! # fn main() -> vsql::Result<()> {
//! let conn = Connection::open_in_memory()?;
//! conn.execute("CREATE TABLE foo (x INTEGER, y VARCHAR(10))", ())?;
//! conn.execute("INSERT INTO foo (x, y) VALUES (?, ?)", (1, "a"))?;
//!
//! let mut stmt = conn.prepare("SELECT x, y FROM foo")?;
//! for row in stmt.query(())? {
//!     let row = row?;
//!     let x: i64 = row.get(0)?;
//!     let y: String = row.get("y")?;
//!     println!("{x} {y}");
//! }
//! # Ok(())
//! # }
//! ```
//!
//! Each statement is one SQL statement. Its `--` comments are removed before
//! it's given to vsql (which doesn't allow them), and `/* */` comments are
//! kept.
//!
//! # Transactions
//!
//! Each statement is committed when it finishes (or when the rows of a query
//! are dropped), unless it's in a [`Transaction`]. `START TRANSACTION`,
//! `COMMIT` and `ROLLBACK` can't be executed as statements.
//!
//! A [`Savepoint`] (see [`Transaction::savepoint`] and
//! [`Connection::savepoint`]) can be rolled back to without rolling back the
//! whole transaction, and savepoints can be nested.
//!
//! # Parameters
//!
//! Parameters are `?`, given as `()`, a tuple, an array, [`params!`] or
//! [`params_from_iter`]. Each Rust type is sent as an SQL type (see
//! [`Value`]), which is cast to the type it's used as:
//!
//! | Rust                                  | SQL                                        |
//! |---------------------------------------|--------------------------------------------|
//! | `None`                                | `NULL`                                     |
//! | `bool`                                | `BOOLEAN`                                  |
//! | `i8` to `i64`, `u8` to `u32`          | `BIGINT`                                   |
//! | `u64`, `i128`, `u128`                 | `BIGINT` (`NUMERIC` if it's out of range)  |
//! | `f32`, `f64`                          | `DOUBLE PRECISION`                         |
//! | `&str`, `String`                      | `CHARACTER VARYING`                        |
//! | `&[u8]`, `Vec<u8>`, `[u8; N]`         | `BINARY VARYING`                           |
//! | [`Date`]                              | `DATE`                                     |
//! | [`Time`]                              | `TIME(6)` (`WITH TIME ZONE` with an offset) |
//! | [`Timestamp`]                         | `TIMESTAMP(6)` (`WITH TIME ZONE` with an offset) |
//! | [`Interval`]                          | the `INTERVAL` literal                     |
//!
//! A `DECFLOAT` is a [`Value::DecFloat`].
//!
//! # Columns
//!
//! | SQL                              | Rust                                 |
//! |----------------------------------|--------------------------------------|
//! | `SMALLINT`, `INTEGER`, `BIGINT`  | any integer type (if it's in range)  |
//! | `REAL`, `DOUBLE PRECISION`       | `f64`, `f32`                         |
//! | `DECIMAL`, `NUMERIC`, `DECFLOAT` | `String` (exact), `f64`, any integer type (if it's an integer in range) |
//! | `CHARACTER`, `VARCHAR`, `CLOB`   | `String`                             |
//! | `BINARY`, `VARBINARY`, `BLOB`    | `Vec<u8>`, `[u8; N]`                 |
//! | `BOOLEAN`                        | `bool`                               |
//! | `DATE`                           | [`Date`]                             |
//! | `TIME (WITH TIME ZONE)`          | [`Time`]                             |
//! | `TIMESTAMP (WITH TIME ZONE)`     | [`Timestamp`]                        |
//! | `INTERVAL ...`                   | [`Interval`]                         |
//!
//! A `NULL` is only an `Option`. Any column can be read as a [`Value`]. An
//! integer that's out of the range of a `BIGINT` (such as a large `u64`) is
//! sent as a `NUMERIC`, so it can be read back as the same type.
//!
//! # Features
//!
//! - `chrono`: `NaiveDate`, `NaiveTime`, `NaiveDateTime`, `DateTime<FixedOffset>`
//!   and `DateTime<Utc>` as parameters and columns.
//! - `rust_decimal`: `Decimal` as a parameter and a `DECIMAL`, `NUMERIC` or
//!   `DECFLOAT` column.
//!
//! # Errors
//!
//! An error from vsql is [`Error::Sql`], with its SQLSTATE (see
//! [`Error::sqlstate`]), such as `"42P01"` for an undefined table, and its
//! [`Diagnostics`] (see [`Error::diagnostics`]), such as the constraint that
//! was violated.
//!
//! # Version
//!
//! [`version`] is the version of vsql that's compiled into the crate, and
//! [`build_info`] is how it was built.
//!
//! # Threads
//!
//! Each connection has its own libvsql environment, so connections can be
//! used from different threads at the same time. A [`Connection`] is `Send`
//! but not `Sync`.

mod connection;
mod error;
mod params;
mod raw;
mod sql;
mod statement;
pub mod types;
mod version;

#[cfg(feature = "chrono")]
mod with_chrono;
#[cfg(feature = "rust_decimal")]
mod with_rust_decimal;

pub use connection::{Connection, IsolationLevel, Savepoint, Transaction, TransactionOptions};
pub use error::{Diagnostics, Error, Result};
pub use params::{params_from_iter, Params, ParamsFromIter};
pub use statement::{Column, MappedRows, Row, RowIndex, Rows, Statement};
pub use types::{Date, FromSql, Interval, IntervalFields, Time, Timestamp, ToSql, Type, Value};
pub use version::{build_info, version, version_number, BuildInfo};

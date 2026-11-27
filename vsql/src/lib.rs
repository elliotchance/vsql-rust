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
//! kept. [`Connection::execute_batch`] executes SQL with more than one
//! statement (separated by `;`), such as a schema.
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
//! A transaction (or savepoint) that isn't committed is rolled back when it's
//! dropped, unless its [`DropBehavior`] is changed.
//! [`Connection::unchecked_transaction`] starts a transaction through a
//! `&Connection`.
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
//! | `&str`, `String`                      | `CHARACTER VARYING` (`CLOB` if it's longer than 10,485,760) |
//! | `&[u8]`, `Vec<u8>`, `[u8; N]`         | `BINARY VARYING` (`BLOB` if it's longer than 10,485,760) |
//! | [`Date`]                              | `DATE`                                     |
//! | [`Time`]                              | `TIME(6)` (`WITH TIME ZONE` with an offset) |
//! | [`Timestamp`]                         | `TIMESTAMP(6)` (`WITH TIME ZONE` with an offset) |
//! | [`Interval`]                          | the `INTERVAL` literal                     |
//! | [`std::time::Duration`]               | `INTERVAL DAY TO SECOND`                   |
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
//! | `INTERVAL ...`                   | [`Interval`], [`std::time::Duration`] (day-time, not negative) |
//!
//! A `NULL` is only an `Option`. Any column can be read as a [`Value`]. An
//! integer that's out of the range of a `BIGINT` (such as a large `u64`) is
//! sent as a `NUMERIC`, so it can be read back as the same type.
//!
//! # Features
//!
//! - `chrono`: `NaiveDate`, `NaiveTime`, `NaiveDateTime`, `DateTime<FixedOffset>`
//!   and `DateTime<Utc>` as parameters and columns.
//! - `time`: `Date`, `Time`, `PrimitiveDateTime` and `OffsetDateTime` as
//!   parameters and columns, and `Duration` as a day-time `INTERVAL`.
//! - `rust_decimal`: `Decimal` as a parameter and a `DECIMAL`, `NUMERIC` or
//!   `DECFLOAT` column.
//! - `r2d2`, `deadpool`: a connection pool of a database file, see [`pool`].
//! - `uuid`: `Uuid` as a parameter (a `BINARY VARYING` of 16 bytes) and a
//!   binary or character string column (such as from `UUID_TO_BINARY`).
//! - `serde_json`: `serde_json::Value` as a parameter (a `CHARACTER VARYING`)
//!   and a character string column of JSON text.
//! - `bigdecimal`: `BigDecimal` as a parameter and a `DECIMAL`, `NUMERIC` or
//!   `DECFLOAT` column, for more digits than `rust_decimal` has (28).
//!
//! # Schema
//!
//! [`Connection::tables`], [`columns`](Connection::columns),
//! [`primary_key`](Connection::primary_key) and
//! [`foreign_keys`](Connection::foreign_keys) describe the tables of a schema,
//! and [`table_exists`](Connection::table_exists) checks for one.
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
mod optional;
mod params;
#[cfg(any(feature = "r2d2", feature = "deadpool"))]
pub mod pool;
mod raw;
mod schema;
mod sql;
mod statement;
pub mod types;
mod version;

#[cfg(feature = "bigdecimal")]
mod with_bigdecimal;
#[cfg(feature = "chrono")]
mod with_chrono;
#[cfg(feature = "rust_decimal")]
mod with_rust_decimal;
#[cfg(feature = "serde_json")]
mod with_serde_json;
#[cfg(feature = "time")]
mod with_time;
#[cfg(feature = "uuid")]
mod with_uuid;

pub use connection::{
    Connection, DropBehavior, IsolationLevel, Savepoint, Transaction, TransactionOptions,
};
pub use error::{Diagnostics, Error, Result};
pub use optional::OptionalExtension;
pub use params::{params_from_iter, Params, ParamsFromIter};
pub use schema::{ForeignKey, ForeignKeyAction, PrimaryKey, Table, TableColumn, TableType};
pub use statement::{AndThenRows, Column, MappedRows, Row, RowIndex, Rows, Statement};
pub use types::{Date, FromSql, Interval, IntervalFields, Time, Timestamp, ToSql, Type, Value};
pub use version::{build_info, version, version_number, BuildInfo};

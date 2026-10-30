use std::cell::Cell;
use std::marker::PhantomData;
use std::ops::Deref;
use std::path::Path;

use vsql_sys::*;

use crate::error::{Error, Result};
use crate::params::Params;
use crate::raw;
use crate::statement::{Row, Statement};

/// A connection to a database.
///
/// Each connection has its own libvsql environment, so connections can be used
/// from different threads at the same time. A connection can be moved to
/// another thread (it's `Send`), but can't be shared by threads (it isn't
/// `Sync`).
///
/// Each statement is committed when it finishes (or when the rows of a query
/// are dropped), unless it's in a [`Transaction`].
pub struct Connection {
    env: SQLHENV,
    dbc: SQLHDBC,
    in_transaction: Cell<bool>,
    // Not Sync, since a libvsql environment runs one routine at a time and has
    // one transaction.
    _not_sync: PhantomData<Cell<()>>,
}

impl Connection {
    /// Opens a database file, which is created if it doesn't exist.
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Connection> {
        let path = path.as_ref();
        let name = path
            .to_str()
            .ok_or_else(|| Error::InvalidPath(path.to_path_buf()))?;

        Connection::connect(name)
    }

    /// Opens a database that only exists for the life of the connection. Each
    /// connection to memory is a different database.
    pub fn open_in_memory() -> Result<Connection> {
        Connection::connect(":memory:")
    }

    fn connect(name: &str) -> Result<Connection> {
        let env = raw::alloc_env()?;
        let dbc = match raw::alloc_dbc(env) {
            Ok(dbc) => dbc,
            Err(e) => {
                let _ = raw::free_handle(SQL_HANDLE_ENV, env);
                return Err(e);
            }
        };
        if let Err(e) = raw::connect(dbc, name) {
            let _ = raw::free_handle(SQL_HANDLE_DBC, dbc);
            let _ = raw::free_handle(SQL_HANDLE_ENV, env);
            return Err(e);
        }

        // Dropped if it fails, which disconnects and frees the handles.
        let conn = Connection {
            env,
            dbc,
            in_transaction: Cell::new(false),
            _not_sync: PhantomData,
        };
        raw::set_autocommit(dbc, true)?;

        Ok(conn)
    }

    pub(crate) fn dbc(&self) -> SQLHDBC {
        self.dbc
    }

    /// Executes a statement, with a `?` for each of the parameters, and
    /// returns the number of rows that it changed.
    ///
    /// ```
    /// # fn main() -> vsql::Result<()> {
    /// let conn = vsql::Connection::open_in_memory()?;
    /// conn.execute("CREATE TABLE foo (x INTEGER)", ())?;
    /// assert_eq!(conn.execute("INSERT INTO foo (x) VALUES (?)", [123])?, 1);
    /// # Ok(())
    /// # }
    /// ```
    pub fn execute<P: Params>(&self, sql: &str, params: P) -> Result<usize> {
        let values = params.to_values()?;
        if values.is_empty() {
            // Without parameters, the statement doesn't need to be prepared.
            let mut stmt = Statement::new(self)?;
            stmt.exec_direct_sql(sql)?;
            return stmt.finish_execute();
        }

        self.prepare(sql)?.execute_values(values)
    }

    /// Prepares a statement, which can be executed many times.
    pub fn prepare(&self, sql: &str) -> Result<Statement<'_>> {
        let mut stmt = Statement::new(self)?;
        stmt.prepare_sql(sql)?;

        Ok(stmt)
    }

    /// Executes a query and returns `f` of its first row, or
    /// [`Error::QueryReturnedNoRows`].
    ///
    /// ```
    /// # fn main() -> vsql::Result<()> {
    /// let conn = vsql::Connection::open_in_memory()?;
    /// let x: i64 = conn.query_row("VALUES ? + 1", [2], |row| row.get(0))?;
    /// assert_eq!(x, 3);
    /// # Ok(())
    /// # }
    /// ```
    pub fn query_row<T, P, F>(&self, sql: &str, params: P, f: F) -> Result<T>
    where
        P: Params,
        F: FnOnce(&Row) -> Result<T>,
    {
        self.prepare(sql)?.query_row(params, f)
    }

    /// Starts a READ COMMITTED (READ WRITE) transaction, see
    /// [`transaction_with`](Connection::transaction_with).
    pub fn transaction(&mut self) -> Result<Transaction<'_>> {
        self.transaction_with(TransactionOptions::default())
    }

    /// Starts a transaction, which is rolled back when it's dropped unless
    /// it's committed. Statements are executed in the transaction through it
    /// (it dereferences to the [`Connection`]).
    ///
    /// vsql's transactions are READ COMMITTED, so a stricter isolation level
    /// or a read only transaction is an error.
    ///
    /// ```
    /// # fn main() -> vsql::Result<()> {
    /// let mut conn = vsql::Connection::open_in_memory()?;
    /// conn.execute("CREATE TABLE foo (x INTEGER)", ())?;
    ///
    /// let tx = conn.transaction()?;
    /// tx.execute("INSERT INTO foo (x) VALUES (1)", ())?;
    /// tx.commit()?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn transaction_with(&mut self, options: TransactionOptions) -> Result<Transaction<'_>> {
        let access_mode = if options.read_only {
            SQL_TRANSACTION_READ_ONLY
        } else {
            SQL_TRANSACTION_READ_WRITE
        };
        raw::start_tran(self.dbc, access_mode, options.isolation_level.code())?;
        self.in_transaction.set(true);

        Ok(Transaction {
            conn: self,
            finished: false,
        })
    }

    /// Whether each statement is committed when it finishes, which is false in
    /// a [`Transaction`].
    pub fn is_autocommit(&self) -> bool {
        !self.in_transaction.get()
    }

    /// Closes the connection. A transaction that hasn't been committed is
    /// rolled back. Dropping the connection does the same, but ignores errors.
    pub fn close(mut self) -> Result<()> {
        self.close_handles()
    }

    fn close_handles(&mut self) -> Result<()> {
        if self.dbc == SQL_NULL_HANDLE {
            return Ok(());
        }

        let mut result = Ok(());
        if self.in_transaction.replace(false) {
            result = result.and(raw::end_tran(self.dbc, false));
        }
        result = result.and(raw::disconnect(self.dbc));
        result = result.and(raw::free_handle(SQL_HANDLE_DBC, self.dbc));
        result = result.and(raw::free_handle(SQL_HANDLE_ENV, self.env));
        self.dbc = SQL_NULL_HANDLE;

        result
    }
}

impl Drop for Connection {
    fn drop(&mut self) {
        let _ = self.close_handles();
    }
}

impl std::fmt::Debug for Connection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Connection")
            .field("autocommit", &self.is_autocommit())
            .finish_non_exhaustive()
    }
}

/// The isolation level of a transaction. vsql's transactions are READ
/// COMMITTED.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum IsolationLevel {
    ReadUncommitted,
    #[default]
    ReadCommitted,
    RepeatableRead,
    Serializable,
}

impl IsolationLevel {
    fn code(self) -> SQLINTEGER {
        match self {
            IsolationLevel::ReadUncommitted => SQL_TRANSACTION_READ_UNCOMMITTED,
            IsolationLevel::ReadCommitted => SQL_TRANSACTION_READ_COMMITTED,
            IsolationLevel::RepeatableRead => SQL_TRANSACTION_REPEATABLE_READ,
            IsolationLevel::Serializable => SQL_TRANSACTION_SERIALIZABLE,
        }
    }
}

/// The options of a transaction, see
/// [`transaction_with`](Connection::transaction_with).
#[derive(Debug, Clone, Copy, Default)]
#[non_exhaustive]
pub struct TransactionOptions {
    pub isolation_level: IsolationLevel,
    pub read_only: bool,
}

impl TransactionOptions {
    pub fn isolation_level(mut self, isolation_level: IsolationLevel) -> Self {
        self.isolation_level = isolation_level;
        self
    }

    pub fn read_only(mut self, read_only: bool) -> Self {
        self.read_only = read_only;
        self
    }
}

/// A transaction, see [`Connection::transaction`]. It's rolled back when it's
/// dropped, unless it's committed.
pub struct Transaction<'conn> {
    conn: &'conn mut Connection,
    finished: bool,
}

impl Transaction<'_> {
    /// Commits the transaction.
    pub fn commit(mut self) -> Result<()> {
        self.end(true)
    }

    /// Rolls back the transaction.
    pub fn rollback(mut self) -> Result<()> {
        self.end(false)
    }

    fn end(&mut self, commit: bool) -> Result<()> {
        self.finished = true;
        self.conn.in_transaction.set(false);

        raw::end_tran(self.conn.dbc, commit)
    }
}

impl Deref for Transaction<'_> {
    type Target = Connection;

    fn deref(&self) -> &Connection {
        self.conn
    }
}

impl Drop for Transaction<'_> {
    fn drop(&mut self) {
        if !self.finished {
            let _ = self.end(false);
        }
    }
}

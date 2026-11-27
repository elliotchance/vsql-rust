use std::cell::{Cell, RefCell};
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
    // The names (as they are in SQL) of the savepoints that are active.
    savepoints: RefCell<Vec<String>>,
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
            savepoints: RefCell::new(Vec::new()),
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
    /// A READ UNCOMMITTED transaction is always read only (READ WRITE can't be
    /// used with READ UNCOMMITTED), see [`IsolationLevel`].
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
        self.start_transaction(options)?;

        Ok(Transaction {
            conn: self,
            finished: false,
        })
    }

    /// Starts a transaction with a savepoint, see [`Savepoint`]. Committing
    /// the savepoint commits the transaction, and dropping it rolls the
    /// transaction back.
    ///
    /// ```
    /// # fn main() -> vsql::Result<()> {
    /// let mut conn = vsql::Connection::open_in_memory()?;
    /// conn.execute("CREATE TABLE foo (x INTEGER)", ())?;
    ///
    /// let mut sp = conn.savepoint()?;
    /// sp.execute("INSERT INTO foo (x) VALUES (1)", ())?;
    /// sp.rollback()?; // Back to the start, the savepoint is still active.
    /// sp.execute("INSERT INTO foo (x) VALUES (2)", ())?;
    /// sp.commit()?;
    ///
    /// let x: i64 = conn.query_row("SELECT x FROM foo", (), |row| row.get(0))?;
    /// assert_eq!(x, 2);
    /// # Ok(())
    /// # }
    /// ```
    pub fn savepoint(&mut self) -> Result<Savepoint<'_>> {
        self.start_transaction(TransactionOptions::default())?;
        Savepoint::start(self, 1, None, true)
    }

    /// The same as [`savepoint`](Connection::savepoint), with a name for the
    /// savepoint (which is quoted, so it can be any string). It's
    /// [`Error::SavepointNameInUse`] if a savepoint that's still active has the
    /// name.
    pub fn savepoint_with_name<T: Into<String>>(&mut self, name: T) -> Result<Savepoint<'_>> {
        self.start_transaction(TransactionOptions::default())?;
        Savepoint::start(self, 1, Some(name.into()), true)
    }

    fn start_transaction(&self, options: TransactionOptions) -> Result<()> {
        let access_mode =
            if options.read_only || options.isolation_level == IsolationLevel::ReadUncommitted {
                SQL_TRANSACTION_READ_ONLY
            } else {
                SQL_TRANSACTION_READ_WRITE
            };
        raw::start_tran(self.dbc, access_mode, options.isolation_level.code())?;
        self.in_transaction.set(true);

        Ok(())
    }

    fn end_transaction(&self, commit: bool) -> Result<()> {
        self.in_transaction.set(false);
        self.savepoints.borrow_mut().clear();

        raw::end_tran(self.dbc, commit)
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
            self.savepoints.borrow_mut().clear();
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
/// COMMITTED or SERIALIZABLE: READ UNCOMMITTED is READ COMMITTED (and read
/// only), and REPEATABLE READ is SERIALIZABLE.
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

    /// Makes a savepoint in the transaction, see [`Savepoint`].
    pub fn savepoint(&mut self) -> Result<Savepoint<'_>> {
        Savepoint::start(self.conn, 1, None, false)
    }

    /// The same as [`savepoint`](Transaction::savepoint), with a name for the
    /// savepoint (which is quoted, so it can be any string). It's
    /// [`Error::SavepointNameInUse`] if a savepoint that's still active has the
    /// name.
    pub fn savepoint_with_name<T: Into<String>>(&mut self, name: T) -> Result<Savepoint<'_>> {
        Savepoint::start(self.conn, 1, Some(name.into()), false)
    }

    fn end(&mut self, commit: bool) -> Result<()> {
        self.finished = true;

        self.conn.end_transaction(commit)
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

/// A savepoint in a transaction, see [`Connection::savepoint`] and
/// [`Transaction::savepoint`], which can be rolled back to any number of
/// times. Statements are executed through it (it dereferences to the
/// [`Connection`]), and savepoints can be nested.
///
/// Committing a savepoint releases it, so that its changes are part of the
/// transaction (or savepoint) that it's in. Dropping a savepoint that hasn't
/// been committed rolls back to it and releases it. The savepoint of
/// [`Connection::savepoint`] started its transaction, so committing it
/// commits the transaction, and dropping it rolls the transaction back.
///
/// ```
/// # fn main() -> vsql::Result<()> {
/// let mut conn = vsql::Connection::open_in_memory()?;
/// conn.execute("CREATE TABLE foo (x INTEGER)", ())?;
///
/// let mut tx = conn.transaction()?;
/// tx.execute("INSERT INTO foo (x) VALUES (1)", ())?;
/// {
///     let sp = tx.savepoint()?;
///     sp.execute("INSERT INTO foo (x) VALUES (2)", ())?;
/// } // Rolled back.
/// tx.commit()?;
///
/// let n: i64 = conn.query_row("SELECT COUNT(*) FROM foo", (), |row| row.get(0))?;
/// assert_eq!(n, 1);
/// # Ok(())
/// # }
/// ```
pub struct Savepoint<'conn> {
    conn: &'conn Connection,
    // The name as it is in SQL (quoted if it was given).
    name: String,
    depth: u32,
    // Whether the savepoint started the transaction, so it ends it.
    ends_transaction: bool,
    finished: bool,
}

impl Savepoint<'_> {
    fn start(
        conn: &Connection,
        depth: u32,
        name: Option<String>,
        ends_transaction: bool,
    ) -> Result<Savepoint<'_>> {
        // A savepoint with the same name as an active one would replace it,
        // so a name that's given can't be in use, and a generated one is the
        // first that isn't.
        let in_use = |sql_name: &str| conn.savepoints.borrow().iter().any(|n| n == sql_name);
        let name = match name {
            Some(name) => {
                let sql_name = format!("\"{}\"", name.replace('"', "\"\""));
                // A quoted name that's an upper case identifier is the same
                // as the identifier without quotes.
                if in_use(&sql_name) || in_use(&name) {
                    if ends_transaction {
                        let _ = conn.end_transaction(false);
                    }
                    return Err(Error::SavepointNameInUse(name));
                }
                sql_name
            }
            None => (0..)
                .map(|n| match n {
                    0 => format!("VSQL_SP_{depth}"),
                    n => format!("VSQL_SP_{depth}_{n}"),
                })
                .find(|name| !in_use(name) && !in_use(&format!("\"{name}\"")))
                .expect("a savepoint name"),
        };
        if let Err(e) = conn.execute(&format!("SAVEPOINT {name}"), ()) {
            if ends_transaction {
                let _ = conn.end_transaction(false);
            }
            return Err(e);
        }
        conn.savepoints.borrow_mut().push(name.clone());

        Ok(Savepoint {
            conn,
            name,
            depth,
            ends_transaction,
            finished: false,
        })
    }

    /// Makes a savepoint in this savepoint.
    pub fn savepoint(&mut self) -> Result<Savepoint<'_>> {
        Savepoint::start(self.conn, self.depth + 1, None, false)
    }

    /// The same as [`savepoint`](Savepoint::savepoint), with a name for the
    /// savepoint (which is quoted, so it can be any string). It's
    /// [`Error::SavepointNameInUse`] if a savepoint that's still active has the
    /// name.
    pub fn savepoint_with_name<T: Into<String>>(&mut self, name: T) -> Result<Savepoint<'_>> {
        Savepoint::start(self.conn, self.depth + 1, Some(name.into()), false)
    }

    /// Releases the savepoint, or commits the transaction that it started.
    pub fn commit(mut self) -> Result<()> {
        self.finished = true;
        if self.ends_transaction {
            self.conn.end_transaction(true)
        } else {
            self.forget();
            raw::end_savepoint(self.conn.dbc, &self.name, true)
        }
    }

    /// Removes the savepoint from the active ones, before it's released.
    fn forget(&self) {
        let mut savepoints = self.conn.savepoints.borrow_mut();
        if let Some(i) = savepoints.iter().rposition(|n| *n == self.name) {
            savepoints.remove(i);
        }
    }

    /// Rolls back to the savepoint, which is still active, so it can be rolled
    /// back to again (or committed).
    pub fn rollback(&mut self) -> Result<()> {
        raw::end_savepoint(self.conn.dbc, &self.name, false)
    }
}

impl Deref for Savepoint<'_> {
    type Target = Connection;

    fn deref(&self) -> &Connection {
        self.conn
    }
}

impl Drop for Savepoint<'_> {
    fn drop(&mut self) {
        if self.finished {
            return;
        }
        if self.ends_transaction {
            let _ = self.conn.end_transaction(false);
        } else {
            self.forget();
            let _ = self.rollback();
            let _ = raw::end_savepoint(self.conn.dbc, &self.name, true);
        }
    }
}

impl std::fmt::Debug for Savepoint<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Savepoint")
            .field("name", &self.name)
            .finish_non_exhaustive()
    }
}

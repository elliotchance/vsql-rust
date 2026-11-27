use std::cell::{Cell, RefCell};
use std::marker::PhantomData;
use std::ops::Deref;
use std::path::Path;

use vsql_sys::*;

use crate::error::{Error, Result};
use crate::params::Params;
use crate::raw;
use crate::sql;
use crate::statement::{AndThenRows, MappedRows, Row, Rows, Statement};

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
    /// returns the number of rows that it changed. A query (such as a SELECT)
    /// is [`Error::ExecuteReturnedResults`] and isn't executed, see
    /// [`query_row`](Connection::query_row) and [`prepare`](Connection::prepare).
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
        self.prepare(sql)?.execute(params)
    }

    /// Executes SQL that has any number of statements separated by `;` (such
    /// as a schema or a migration), without parameters. A `;` in a string, a
    /// quoted identifier or a comment doesn't end a statement.
    ///
    /// The statements are executed one at a time, in autocommit (so the
    /// statements before one that fails have been committed) or in the
    /// transaction it's executed in. The rows of a query are ignored.
    ///
    /// ```
    /// # fn main() -> vsql::Result<()> {
    /// let conn = vsql::Connection::open_in_memory()?;
    /// conn.execute_batch(
    ///     "CREATE TABLE foo (x INTEGER);
    ///      INSERT INTO foo (x) VALUES (1); -- The first row.
    ///      INSERT INTO foo (x) VALUES (2);",
    /// )?;
    /// let n: i64 = conn.query_row("SELECT COUNT(*) FROM foo", (), |row| row.get(0))?;
    /// assert_eq!(n, 2);
    /// # Ok(())
    /// # }
    /// ```
    pub fn execute_batch(&self, sql: &str) -> Result<()> {
        let mut stmt = Statement::new(self)?;
        for sql in sql::split_statements(sql) {
            stmt.execute_ignoring_rows(sql)?;
        }

        Ok(())
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

    /// The same as [`query_row`](Connection::query_row), but `f` can return
    /// any error that a vsql [`Error`] converts into.
    pub fn query_row_and_then<T, E, P, F>(&self, sql: &str, params: P, f: F) -> Result<T, E>
    where
        P: Params,
        E: From<Error>,
        F: FnOnce(&Row) -> Result<T, E>,
    {
        self.prepare(sql)?.query_row_and_then(params, f)
    }

    /// Executes a query, see [`Statement::query`]. The rows own the
    /// statement, so (unlike `conn.prepare(sql)?.query(params)`) they can be
    /// returned from a function.
    ///
    /// ```
    /// fn at_most(conn: &vsql::Connection, n: i64) -> vsql::Result<vsql::Rows<'_>> {
    ///     conn.query("SELECT x FROM foo WHERE x <= ?", [n])
    /// }
    ///
    /// # fn main() -> vsql::Result<()> {
    /// let conn = vsql::Connection::open_in_memory()?;
    /// conn.execute("CREATE TABLE foo (x INTEGER)", ())?;
    /// for x in 1..=5 {
    ///     conn.execute("INSERT INTO foo (x) VALUES (?)", [x])?;
    /// }
    /// assert_eq!(at_most(&conn, 2)?.count(), 2);
    /// # Ok(())
    /// # }
    /// ```
    pub fn query<P: Params>(&self, sql: &str, params: P) -> Result<Rows<'_>> {
        self.prepare(sql)?.into_rows(params)
    }

    /// Executes a query and maps each row with `f`, see
    /// [`Statement::query_map`]. The rows own the statement, so they can be
    /// returned from a function.
    ///
    /// ```
    /// # fn main() -> vsql::Result<()> {
    /// let conn = vsql::Connection::open_in_memory()?;
    /// let xs = conn
    ///     .query_map("VALUES 1, 2, 3", (), |row| row.get::<_, i64>(0))?
    ///     .collect::<vsql::Result<Vec<_>>>()?;
    /// assert_eq!(xs, [1, 2, 3]);
    /// # Ok(())
    /// # }
    /// ```
    pub fn query_map<T, P, F>(&self, sql: &str, params: P, f: F) -> Result<MappedRows<'_, F>>
    where
        P: Params,
        F: FnMut(&Row) -> Result<T>,
    {
        Ok(self.query(sql, params)?.mapped(f))
    }

    /// Executes a query and maps each row with `f`, which can return any error
    /// that a vsql [`Error`] converts into, see [`Statement::query_and_then`].
    /// The rows own the statement, so they can be returned from a function.
    pub fn query_and_then<T, E, P, F>(
        &self,
        sql: &str,
        params: P,
        f: F,
    ) -> Result<AndThenRows<'_, F>>
    where
        P: Params,
        E: From<Error>,
        F: FnMut(&Row) -> Result<T, E>,
    {
        Ok(self.query(sql, params)?.and_then(f))
    }

    /// Starts a READ COMMITTED (READ WRITE) transaction, see
    /// [`transaction_with`](Connection::transaction_with).
    pub fn transaction(&mut self) -> Result<Transaction<'_>> {
        self.transaction_with(TransactionOptions::default())
    }

    /// Starts a transaction, which is rolled back when it's dropped unless
    /// it's committed (see [`Transaction::set_drop_behavior`]). Statements are executed in the transaction through it
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

        Ok(Transaction::new(self))
    }

    /// Starts a READ COMMITTED (READ WRITE) transaction, like
    /// [`transaction`](Connection::transaction), through a shared reference to
    /// the connection (such as one in an `Rc`). The connection can't be in a
    /// transaction already, which `transaction` makes sure of by borrowing it
    /// mutably, so it's an error if it is.
    ///
    /// ```
    /// # fn main() -> vsql::Result<()> {
    /// let conn = std::rc::Rc::new(vsql::Connection::open_in_memory()?);
    /// conn.execute("CREATE TABLE foo (x INTEGER)", ())?;
    ///
    /// let tx = conn.unchecked_transaction()?;
    /// tx.execute("INSERT INTO foo (x) VALUES (1)", ())?;
    /// assert!(conn.unchecked_transaction().is_err());
    /// tx.commit()?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn unchecked_transaction(&self) -> Result<Transaction<'_>> {
        self.start_transaction(TransactionOptions::default())?;

        Ok(Transaction::new(self))
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
        self.savepoint_with(TransactionOptions::default())
    }

    /// The same as [`savepoint`](Connection::savepoint), with the options of
    /// the transaction that it starts (see
    /// [`transaction_with`](Connection::transaction_with)).
    pub fn savepoint_with(&mut self, options: TransactionOptions) -> Result<Savepoint<'_>> {
        self.start_transaction(options)?;
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

/// What a [`Transaction`] or [`Savepoint`] that hasn't been committed or
/// rolled back does when it's dropped (or finished, see
/// [`Transaction::finish`]).
///
/// ```
/// # fn main() -> vsql::Result<()> {
/// let mut conn = vsql::Connection::open_in_memory()?;
/// conn.execute("CREATE TABLE foo (x INTEGER)", ())?;
///
/// let mut tx = conn.transaction()?;
/// tx.set_drop_behavior(vsql::DropBehavior::Commit);
/// tx.execute("INSERT INTO foo (x) VALUES (1)", ())?;
/// tx.finish()?; // Committed.
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum DropBehavior {
    /// Rolls back the transaction, or rolls back to the savepoint and releases
    /// it.
    #[default]
    Rollback,
    /// Commits the transaction, or releases the savepoint.
    Commit,
    /// Leaves the transaction or savepoint active. vsql's `COMMIT` and
    /// `ROLLBACK` can't be executed as statements, so a transaction then stays
    /// active until the connection is closed (which rolls it back).
    Ignore,
    /// Rolls back like `Rollback`, then panics (unless the thread is already
    /// panicking), for a transaction that must always be committed or rolled
    /// back explicitly.
    Panic,
}

/// A transaction, see [`Connection::transaction`]. It's rolled back when it's
/// dropped, unless it's committed (see [`set_drop_behavior`](Transaction::set_drop_behavior)).
pub struct Transaction<'conn> {
    conn: &'conn Connection,
    drop_behavior: DropBehavior,
    finished: bool,
}

impl<'conn> Transaction<'conn> {
    fn new(conn: &'conn Connection) -> Transaction<'conn> {
        Transaction {
            conn,
            drop_behavior: DropBehavior::Rollback,
            finished: false,
        }
    }
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

    /// What the transaction does when it's dropped (or finished), which is
    /// [`DropBehavior::Rollback`] unless it's changed.
    pub fn drop_behavior(&self) -> DropBehavior {
        self.drop_behavior
    }

    /// Sets what the transaction does when it's dropped (or finished).
    pub fn set_drop_behavior(&mut self, drop_behavior: DropBehavior) {
        self.drop_behavior = drop_behavior;
    }

    /// Ends the transaction according to its [`DropBehavior`], and returns the
    /// error, which dropping it ignores.
    pub fn finish(mut self) -> Result<()> {
        self.finish_()
    }

    fn finish_(&mut self) -> Result<()> {
        match self.drop_behavior {
            DropBehavior::Rollback => self.end(false),
            DropBehavior::Commit => self.end(true),
            DropBehavior::Ignore => {
                self.finished = true;
                Ok(())
            }
            DropBehavior::Panic => {
                let result = self.end(false);
                panic_unless_panicking("transaction");
                result
            }
        }
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
            let _ = self.finish_();
        }
    }
}

/// For [`DropBehavior::Panic`]. Panicking while the thread is panicking
/// would abort.
fn panic_unless_panicking(what: &str) {
    if !std::thread::panicking() {
        panic!("a {what} was dropped without being committed or rolled back");
    }
}

/// A savepoint in a transaction, see [`Connection::savepoint`] and
/// [`Transaction::savepoint`], which can be rolled back to any number of
/// times. Statements are executed through it (it dereferences to the
/// [`Connection`]), and savepoints can be nested.
///
/// Committing a savepoint releases it, so that its changes are part of the
/// transaction (or savepoint) that it's in. Dropping a savepoint that hasn't
/// been committed rolls back to it and releases it (see
/// [`set_drop_behavior`](Savepoint::set_drop_behavior)). The savepoint of
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
    drop_behavior: DropBehavior,
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
            drop_behavior: DropBehavior::Rollback,
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
        self.release()
    }

    fn release(&mut self) -> Result<()> {
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

    /// Rolls back to the savepoint and releases it, or rolls back the
    /// transaction that it started.
    fn rollback_and_release(&mut self) -> Result<()> {
        self.finished = true;
        if self.ends_transaction {
            self.conn.end_transaction(false)
        } else {
            self.forget();
            let rolled_back = self.rollback();
            rolled_back.and(raw::end_savepoint(self.conn.dbc, &self.name, true))
        }
    }

    /// What the savepoint does when it's dropped (or finished), which is
    /// [`DropBehavior::Rollback`] unless it's changed.
    pub fn drop_behavior(&self) -> DropBehavior {
        self.drop_behavior
    }

    /// Sets what the savepoint does when it's dropped (or finished).
    pub fn set_drop_behavior(&mut self, drop_behavior: DropBehavior) {
        self.drop_behavior = drop_behavior;
    }

    /// Ends the savepoint according to its [`DropBehavior`], and returns the
    /// error, which dropping it ignores.
    pub fn finish(mut self) -> Result<()> {
        self.finish_()
    }

    fn finish_(&mut self) -> Result<()> {
        match self.drop_behavior {
            DropBehavior::Rollback => self.rollback_and_release(),
            DropBehavior::Commit => self.release(),
            DropBehavior::Ignore => {
                self.finished = true;
                Ok(())
            }
            DropBehavior::Panic => {
                let result = self.rollback_and_release();
                panic_unless_panicking("savepoint");
                result
            }
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
        if !self.finished {
            let _ = self.finish_();
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

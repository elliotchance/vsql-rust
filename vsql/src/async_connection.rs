use std::any::Any;
use std::fmt;
use std::panic::{self, AssertUnwindSafe};
use std::path::Path;
use std::sync::mpsc;
use std::thread;

use tokio::sync::oneshot;

use crate::{Connection, Error, Result};

type Call = Box<dyn FnOnce(&mut Connection) + Send>;

type Panic = Box<dyn Any + Send>;

enum Message {
    Call(Call),
    Close(oneshot::Sender<Result<()>>),
}

/// A connection for async code (with the `async` feature), which runs a
/// [`Connection`] on its own thread.
///
/// Each [`call`](AsyncConnection::call) runs a closure with the connection on
/// that thread, one at a time, so the async task isn't blocked. It works with
/// any async runtime.
///
/// An `AsyncConnection` is `Send` and `Sync`, and can be cloned. The clones
/// use the same connection (and the same in-memory database). The connection
/// is closed when [`close`](AsyncConnection::close) is called on any of them,
/// or when they're all dropped.
///
/// ```
/// # #[tokio::main(flavor = "current_thread")]
/// # async fn main() -> vsql::Result<()> {
/// use vsql::AsyncConnection;
///
/// let conn = AsyncConnection::open_in_memory().await?;
/// conn.call(|conn| conn.execute("CREATE TABLE foo (x INTEGER)", ()))
///     .await?;
///
/// let x = 123;
/// conn.call(move |conn| conn.execute("INSERT INTO foo (x) VALUES (?)", [x]))
///     .await?;
///
/// let x: i64 = conn
///     .call(|conn| conn.query_row("SELECT x FROM foo", (), |row| row.get(0)))
///     .await?;
/// assert_eq!(x, 123);
///
/// conn.close().await?;
/// # Ok(())
/// # }
/// ```
#[derive(Clone)]
pub struct AsyncConnection {
    sender: mpsc::Sender<Message>,
}

impl AsyncConnection {
    /// Opens a database file, which is created if it doesn't exist (see
    /// [`Connection::open`]).
    pub async fn open<P: AsRef<Path>>(path: P) -> Result<AsyncConnection> {
        let path = path.as_ref().to_path_buf();
        AsyncConnection::start(move || Connection::open(path)).await
    }

    /// Opens a database that only exists for the life of the connection (see
    /// [`Connection::open_in_memory`]).
    pub async fn open_in_memory() -> Result<AsyncConnection> {
        AsyncConnection::start(Connection::open_in_memory).await
    }

    async fn start<F>(open: F) -> Result<AsyncConnection>
    where
        F: FnOnce() -> Result<Connection> + Send + 'static,
    {
        let (sender, receiver) = mpsc::channel();
        let (opened, opened_receiver) = oneshot::channel();
        thread::spawn(move || match open() {
            Ok(conn) => {
                let _ = opened.send(Ok(()));
                run(conn, receiver);
            }
            Err(e) => {
                let _ = opened.send(Err(e));
            }
        });
        // The thread always sends the result.
        opened_receiver
            .await
            .unwrap_or(Err(Error::ConnectionClosed))?;

        Ok(AsyncConnection { sender })
    }

    /// Runs `f` with the connection on its thread, and returns its result.
    ///
    /// It's [`Error::ConnectionClosed`] if the connection has been closed. If
    /// `f` panics, the panic is resumed here, and the connection stays open.
    ///
    /// A [`Transaction`](crate::Transaction) (which borrows the connection)
    /// is started and finished in the same call:
    ///
    /// ```
    /// # #[tokio::main(flavor = "current_thread")]
    /// # async fn main() -> vsql::Result<()> {
    /// let conn = vsql::AsyncConnection::open_in_memory().await?;
    /// conn.call(|conn| {
    ///     conn.execute("CREATE TABLE foo (x INTEGER)", ())?;
    ///     let tx = conn.transaction()?;
    ///     tx.execute("INSERT INTO foo (x) VALUES (1)", ())?;
    ///     tx.execute("INSERT INTO foo (x) VALUES (2)", ())?;
    ///     tx.commit()
    /// })
    /// .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn call<F, R>(&self, f: F) -> Result<R>
    where
        F: FnOnce(&mut Connection) -> Result<R> + Send + 'static,
        R: Send + 'static,
    {
        let (result, receiver) = oneshot::channel::<std::result::Result<Result<R>, Panic>>();
        let call: Call = Box::new(move |conn| {
            let _ = result.send(panic::catch_unwind(AssertUnwindSafe(|| f(conn))));
        });
        if self.sender.send(Message::Call(call)).is_err() {
            return Err(Error::ConnectionClosed);
        }

        // The result isn't sent if the connection is closed before the call
        // runs (by a clone).
        match receiver.await.unwrap_or(Ok(Err(Error::ConnectionClosed))) {
            Ok(result) => result,
            Err(p) => panic::resume_unwind(p),
        }
    }

    /// Closes the connection, for all of the clones (see
    /// [`Connection::close`]). It's [`Error::ConnectionClosed`] if it's
    /// already closed.
    pub async fn close(self) -> Result<()> {
        let (result, receiver) = oneshot::channel();
        if self.sender.send(Message::Close(result)).is_err() {
            return Err(Error::ConnectionClosed);
        }

        receiver.await.unwrap_or(Err(Error::ConnectionClosed))
    }
}

impl From<Connection> for AsyncConnection {
    /// Moves a connection to its own thread.
    fn from(conn: Connection) -> AsyncConnection {
        let (sender, receiver) = mpsc::channel();
        thread::spawn(move || run(conn, receiver));

        AsyncConnection { sender }
    }
}

impl fmt::Debug for AsyncConnection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AsyncConnection").finish_non_exhaustive()
    }
}

/// Runs the calls of a connection until it's closed or every sender is
/// dropped.
fn run(mut conn: Connection, receiver: mpsc::Receiver<Message>) {
    while let Ok(message) = receiver.recv() {
        match message {
            Message::Call(call) => call(&mut conn),
            Message::Close(result) => {
                let _ = result.send(conn.close());
                return;
            }
        }
    }
}

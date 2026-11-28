//! Connection pools, with the `r2d2` or `deadpool` feature.
//!
//! [`VsqlConnectionManager`] opens connections to a database file for an
//! [`r2d2::Pool`] or a [`deadpool::managed::Pool`]. An in-memory database
//! can't be pooled, since each connection to memory is a different database.
//!
//! ```
//! # #[cfg(feature = "r2d2")]
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! # let path = std::env::temp_dir().join(format!("vsql-pool-doc-{}.vsql", std::process::id()));
//! use vsql::pool::VsqlConnectionManager;
//!
//! let manager = VsqlConnectionManager::file(&path);
//! let pool = r2d2::Pool::new(manager)?;
//!
//! let conn = pool.get()?;
//! conn.execute("CREATE TABLE foo (x INTEGER)", ())?;
//! # drop(conn);
//! # drop(pool);
//! # let _ = std::fs::remove_file(&path);
//! # Ok(())
//! # }
//! # #[cfg(not(feature = "r2d2"))]
//! # fn main() {}
//! ```
//!
//! A [`Connection`] from a deadpool pool runs each statement on the thread
//! that uses it, which blocks an async task, so a pool of an async runtime
//! should be used with something like `tokio::task::spawn_blocking`.
//!
//! With Rust older than 1.95, r2d2 needs scheduled-thread-pool 0.2.7
//! (`cargo update -p scheduled-thread-pool --precise 0.2.7`), since 0.2.8
//! needs a newer Rust but doesn't say so.

use std::path::{Path, PathBuf};

use crate::{Connection, ConnectionOptions, Error, Result};

#[cfg(feature = "deadpool")]
pub use deadpool;
#[cfg(feature = "r2d2")]
pub use r2d2;

/// Opens connections to a database file for a pool (see the
/// [module](crate::pool)).
///
/// A connection is checked by executing `VALUES 1` when it's taken from an
/// r2d2 pool (if the pool tests connections on check out, which it does by
/// default), and when it's returned to a deadpool pool.
#[derive(Debug, Clone)]
pub struct VsqlConnectionManager {
    path: PathBuf,
    options: ConnectionOptions,
}

impl VsqlConnectionManager {
    /// A manager of connections to a database file, which is created if it
    /// doesn't exist.
    pub fn file<P: AsRef<Path>>(path: P) -> VsqlConnectionManager {
        VsqlConnectionManager {
            path: path.as_ref().to_path_buf(),
            options: ConnectionOptions::default(),
        }
    }

    /// Opens the connections with [`ConnectionOptions`], such as a
    /// [`busy_timeout`](ConnectionOptions::busy_timeout).
    pub fn with_options(mut self, options: ConnectionOptions) -> VsqlConnectionManager {
        self.options = options;
        self
    }

    /// The path of the database file.
    pub fn path(&self) -> &Path {
        &self.path
    }

    fn connect(&self) -> Result<Connection> {
        Connection::open_with_options(&self.path, self.options)
    }

    fn check(conn: &Connection) -> Result<()> {
        conn.query_row("VALUES 1", (), |_| Ok(()))
    }
}

#[cfg(feature = "r2d2")]
impl r2d2::ManageConnection for VsqlConnectionManager {
    type Connection = Connection;
    type Error = Error;

    fn connect(&self) -> Result<Connection> {
        VsqlConnectionManager::connect(self)
    }

    fn is_valid(&self, conn: &mut Connection) -> Result<()> {
        VsqlConnectionManager::check(conn)
    }

    fn has_broken(&self, _: &mut Connection) -> bool {
        false
    }
}

#[cfg(feature = "deadpool")]
impl deadpool::managed::Manager for VsqlConnectionManager {
    type Type = Connection;
    type Error = Error;

    async fn create(&self) -> Result<Connection> {
        self.connect()
    }

    async fn recycle(
        &self,
        conn: &mut Connection,
        _: &deadpool::managed::Metrics,
    ) -> deadpool::managed::RecycleResult<Error> {
        Ok(VsqlConnectionManager::check(conn)?)
    }
}

// Connection pools (the r2d2 and deadpool features).
#![cfg(any(feature = "r2d2", feature = "deadpool"))]

use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

use vsql::pool::VsqlConnectionManager;

/// A path for a new database file, which is removed when it's dropped.
struct TempPath(PathBuf);

impl TempPath {
    fn new() -> TempPath {
        static N: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "vsql-rust-pool-test-{}-{}.vsql",
            std::process::id(),
            N.fetch_add(1, Ordering::SeqCst)
        ));
        TempPath(path)
    }
}

impl Drop for TempPath {
    fn drop(&mut self) {
        let name = self.0.file_name().unwrap().to_str().unwrap().to_string();
        if let Ok(entries) = std::fs::read_dir(self.0.parent().unwrap()) {
            for entry in entries.flatten() {
                if entry.file_name().to_str().unwrap_or("").starts_with(&name) {
                    let _ = std::fs::remove_file(entry.path());
                }
            }
        }
    }
}

/// A path in a directory that doesn't exist, so the database can't be opened.
fn bad_path() -> PathBuf {
    std::env::temp_dir()
        .join("vsql-rust-no-such-directory")
        .join("x.vsql")
}

#[test]
fn manager_path() {
    let path = TempPath::new();
    let manager = VsqlConnectionManager::file(&path.0);
    assert_eq!(manager.path(), path.0.as_path());
    assert_eq!(manager.clone().path(), path.0.as_path());
    assert!(format!("{manager:?}").contains("VsqlConnectionManager"));
}

#[cfg(feature = "r2d2")]
mod r2d2_pool {
    use super::*;
    use std::thread;
    use vsql::pool::r2d2::{self, ManageConnection};

    #[test]
    fn threads_share_the_database() {
        let path = TempPath::new();
        let pool = r2d2::Pool::builder()
            .max_size(4)
            .build(VsqlConnectionManager::file(&path.0))
            .unwrap();
        pool.get()
            .unwrap()
            .execute("CREATE TABLE foo (x INTEGER)", ())
            .unwrap();

        let handles: Vec<_> = (0..8)
            .map(|i| {
                let pool = pool.clone();
                thread::spawn(move || {
                    let conn = pool.get().unwrap();
                    conn.execute("INSERT INTO foo (x) VALUES (?)", [i]).unwrap();
                })
            })
            .collect();
        for handle in handles {
            handle.join().unwrap();
        }

        let conn = pool.get().unwrap();
        let n: i64 = conn
            .query_row("SELECT COUNT(*) FROM foo", (), |r| r.get(0))
            .unwrap();
        assert_eq!(n, 8);
    }

    #[test]
    fn manager() {
        let path = TempPath::new();
        let manager = VsqlConnectionManager::file(&path.0);
        let mut conn = manager.connect().unwrap();
        manager.is_valid(&mut conn).unwrap();
        assert!(!manager.has_broken(&mut conn));

        assert!(VsqlConnectionManager::file(bad_path()).connect().is_err());
    }
}

#[cfg(feature = "deadpool")]
mod deadpool_pool {
    use super::*;
    use vsql::pool::deadpool::managed::{Pool, PoolError};

    #[tokio::test]
    async fn get_and_recycle() {
        let path = TempPath::new();
        let pool: Pool<VsqlConnectionManager> = Pool::builder(VsqlConnectionManager::file(&path.0))
            .max_size(2)
            .build()
            .unwrap();

        {
            let conn = pool.get().await.unwrap();
            conn.execute("CREATE TABLE foo (x INTEGER)", ()).unwrap();
            conn.execute("INSERT INTO foo (x) VALUES (1)", ()).unwrap();
        }

        // The connection is recycled.
        let conn = pool.get().await.unwrap();
        let x: i64 = conn
            .query_row("SELECT x FROM foo", (), |r| r.get(0))
            .unwrap();
        assert_eq!(x, 1);
        assert_eq!(pool.status().size, 1);
    }

    #[tokio::test]
    async fn create_fails() {
        let pool: Pool<VsqlConnectionManager> =
            Pool::builder(VsqlConnectionManager::file(bad_path()))
                .build()
                .unwrap();
        assert!(matches!(pool.get().await, Err(PoolError::Backend(_))));
    }
}

// AsyncConnection (the async feature).
#![cfg(feature = "async")]

use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

use vsql::{AsyncConnection, Connection, Error};

/// A path for a new database file, which is removed when it's dropped.
struct TempPath(PathBuf);

impl TempPath {
    fn new() -> TempPath {
        static N: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "vsql-rust-async-test-{}-{}.vsql",
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

fn count(conn: &Connection) -> vsql::Result<i64> {
    conn.query_row("SELECT COUNT(*) FROM foo", (), |r| r.get(0))
}

#[tokio::test]
async fn open_file() {
    let path = TempPath::new();
    let conn = AsyncConnection::open(&path.0).await.unwrap();
    conn.call(|conn| conn.execute("CREATE TABLE foo (x INTEGER)", ()))
        .await
        .unwrap();
    conn.call(|conn| conn.execute("INSERT INTO foo (x) VALUES (?)", [1]))
        .await
        .unwrap();
    conn.close().await.unwrap();

    let conn = Connection::open(&path.0).unwrap();
    assert_eq!(count(&conn).unwrap(), 1);
}

#[tokio::test]
async fn open_fails() {
    let path = std::env::temp_dir()
        .join("vsql-rust-no-such-directory")
        .join("x.vsql");
    assert!(AsyncConnection::open(path).await.is_err());
}

#[tokio::test]
async fn errors_are_returned() {
    let conn = AsyncConnection::open_in_memory().await.unwrap();
    let err = conn
        .call(|conn| conn.execute("SELECT * FROM nope", ()))
        .await
        .unwrap_err();
    assert_eq!(err.sqlstate(), Some("42P01"));
}

#[tokio::test]
async fn clones_share_the_connection() {
    let conn = AsyncConnection::open_in_memory().await.unwrap();
    conn.call(|conn| conn.execute("CREATE TABLE foo (x INTEGER)", ()))
        .await
        .unwrap();

    let tasks: Vec<_> = (0..8)
        .map(|i| {
            let conn = conn.clone();
            tokio::spawn(async move {
                conn.call(move |conn| conn.execute("INSERT INTO foo (x) VALUES (?)", [i]))
                    .await
                    .unwrap();
            })
        })
        .collect();
    for task in tasks {
        task.await.unwrap();
    }

    assert_eq!(conn.call(|conn| count(conn)).await.unwrap(), 8);
}

#[tokio::test]
async fn transaction_in_a_call() {
    let conn = AsyncConnection::open_in_memory().await.unwrap();
    conn.call(|conn| {
        conn.execute("CREATE TABLE foo (x INTEGER)", ())?;
        let tx = conn.transaction()?;
        tx.execute("INSERT INTO foo (x) VALUES (1)", ())?;
        // Rolled back when it's dropped.
        Ok(())
    })
    .await
    .unwrap();
    assert_eq!(conn.call(|conn| count(conn)).await.unwrap(), 0);
}

#[tokio::test]
async fn from_connection() {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute("CREATE TABLE foo (x INTEGER)", ()).unwrap();

    let conn = AsyncConnection::from(conn);
    assert_eq!(conn.call(|conn| count(conn)).await.unwrap(), 0);
    assert!(format!("{conn:?}").contains("AsyncConnection"));
}

#[tokio::test]
async fn closed() {
    let conn = AsyncConnection::open_in_memory().await.unwrap();
    let other = conn.clone();
    let third = conn.clone();
    conn.close().await.unwrap();

    let err = other.call(|_| Ok(())).await.unwrap_err();
    assert!(matches!(err, Error::ConnectionClosed));
    assert_eq!(err.to_string(), "the connection has been closed");
    assert!(matches!(other.close().await, Err(Error::ConnectionClosed)));

    // Dropping the last clone also closes it.
    drop(third);
}

#[tokio::test]
async fn panic_is_resumed() {
    let conn = AsyncConnection::open_in_memory().await.unwrap();
    let other = conn.clone();
    let task =
        tokio::spawn(async move { other.call(|_| -> vsql::Result<()> { panic!("oops") }).await });
    let err = task.await.unwrap_err();
    assert!(err.is_panic());

    // The connection is still open.
    let x: i64 = conn
        .call(|conn| conn.query_row("VALUES 1", (), |r| r.get(0)))
        .await
        .unwrap();
    assert_eq!(x, 1);
}

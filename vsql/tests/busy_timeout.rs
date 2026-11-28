// The lock timeout (ConnectionOptions::busy_timeout) and CREATE INDEX timeout.

use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use vsql::{Connection, ConnectionOptions, IsolationLevel, TransactionOptions};

/// A path for a new database file, which is removed when it's dropped.
struct TempPath(PathBuf);

impl TempPath {
    fn new() -> TempPath {
        static N: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "vsql-rust-busy-timeout-test-{}-{}.vsql",
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

fn serializable() -> TransactionOptions {
    TransactionOptions::default().isolation_level(IsolationLevel::Serializable)
}

#[test]
fn defaults() {
    let options = ConnectionOptions::default();
    assert_eq!(options.busy_timeout, Duration::from_secs(5));
    assert_eq!(options.create_index_timeout, Duration::from_secs(60));

    let conn = Connection::open_in_memory().unwrap();
    assert_eq!(conn.busy_timeout().unwrap(), Duration::from_secs(5));
    assert_eq!(
        conn.create_index_timeout().unwrap(),
        Duration::from_secs(60)
    );
}

#[test]
fn options_are_rounded_up_to_milliseconds() {
    for (timeout, expected) in [
        (Duration::ZERO, Duration::ZERO),
        (Duration::from_millis(250), Duration::from_millis(250)),
        (Duration::from_nanos(1), Duration::from_millis(1)),
        (Duration::from_micros(1500), Duration::from_millis(2)),
        (Duration::MAX, Duration::from_millis(i32::MAX as u64)),
    ] {
        let options = ConnectionOptions::default()
            .busy_timeout(timeout)
            .create_index_timeout(timeout);
        let conn = Connection::open_in_memory_with_options(options).unwrap();
        assert_eq!(conn.busy_timeout().unwrap(), expected);
        assert_eq!(conn.create_index_timeout().unwrap(), expected);
    }
}

/// The time it takes another connection to fail to insert while a SERIALIZABLE
/// transaction has changed the table.
fn time_to_fail(timeout: Duration) -> Duration {
    let path = TempPath::new();
    let mut conn = Connection::open(&path.0).unwrap();
    conn.execute("CREATE TABLE t (x INTEGER)", ()).unwrap();
    let tx = conn.transaction_with(serializable()).unwrap();
    tx.execute("INSERT INTO t (x) VALUES (1)", ()).unwrap();

    let options = ConnectionOptions::default().busy_timeout(timeout);
    let other = Connection::open_with_options(&path.0, options).unwrap();
    let start = Instant::now();
    let err = other
        .execute("INSERT INTO t (x) VALUES (2)", ())
        .unwrap_err();
    let elapsed = start.elapsed();
    assert_eq!(err.sqlstate(), Some("40001"), "{err}");

    // It works once the transaction has finished.
    tx.commit().unwrap();
    other.execute("INSERT INTO t (x) VALUES (2)", ()).unwrap();

    elapsed
}

#[test]
fn busy_timeout() {
    let elapsed = time_to_fail(Duration::from_millis(100));
    assert!(elapsed >= Duration::from_millis(100), "{elapsed:?}");
    assert!(elapsed < Duration::from_secs(3), "{elapsed:?}");
}

#[test]
fn zero_busy_timeout_does_not_wait() {
    let elapsed = time_to_fail(Duration::ZERO);
    assert!(elapsed < Duration::from_secs(1), "{elapsed:?}");
}

#[test]
fn create_index_timeout() {
    let path = TempPath::new();
    let mut conn = Connection::open(&path.0).unwrap();
    conn.execute("CREATE TABLE t (x INTEGER)", ()).unwrap();
    let read_committed =
        TransactionOptions::default().isolation_level(IsolationLevel::ReadCommitted);
    let tx = conn.transaction_with(read_committed).unwrap();
    tx.execute("INSERT INTO t (x) VALUES (1)", ()).unwrap();

    let options = ConnectionOptions::default().create_index_timeout(Duration::from_millis(100));
    let other = Connection::open_with_options(&path.0, options).unwrap();
    let start = Instant::now();
    let err = other.execute("CREATE INDEX i ON t (x)", ()).unwrap_err();
    let elapsed = start.elapsed();
    assert_eq!(err.sqlstate(), Some("55P03"), "{err}");
    assert!(elapsed >= Duration::from_millis(100), "{elapsed:?}");
    assert!(elapsed < Duration::from_secs(3), "{elapsed:?}");
}

#[cfg(feature = "r2d2")]
#[test]
fn pool_options() {
    use vsql::pool::r2d2::ManageConnection;
    use vsql::pool::VsqlConnectionManager;

    let path = TempPath::new();
    let options = ConnectionOptions::default().busy_timeout(Duration::from_millis(100));
    let manager = VsqlConnectionManager::file(&path.0).with_options(options);
    let conn = manager.connect().unwrap();
    assert_eq!(conn.busy_timeout().unwrap(), Duration::from_millis(100));
}

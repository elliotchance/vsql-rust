// DropBehavior, finish, unchecked_transaction and savepoint_with.

use std::panic::{catch_unwind, AssertUnwindSafe};

use vsql::{Connection, DropBehavior, IsolationLevel, TransactionOptions};

fn memory() -> Connection {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute("CREATE TABLE foo (x INTEGER)", ()).unwrap();
    conn
}

fn count(conn: &Connection) -> i64 {
    conn.query_row("SELECT COUNT(*) FROM foo", (), |row| row.get(0))
        .unwrap()
}

fn insert(conn: &Connection) {
    conn.execute("INSERT INTO foo (x) VALUES (1)", ()).unwrap();
}

#[test]
fn transaction_drop_behavior() {
    let mut conn = memory();

    let tx = conn.transaction().unwrap();
    assert_eq!(tx.drop_behavior(), DropBehavior::Rollback);
    insert(&tx);
    tx.finish().unwrap();
    assert_eq!(count(&conn), 0);

    let mut tx = conn.transaction().unwrap();
    tx.set_drop_behavior(DropBehavior::Commit);
    assert_eq!(tx.drop_behavior(), DropBehavior::Commit);
    insert(&tx);
    drop(tx);
    assert_eq!(count(&conn), 1);

    let mut tx = conn.transaction().unwrap();
    tx.set_drop_behavior(DropBehavior::Commit);
    insert(&tx);
    tx.finish().unwrap();
    assert_eq!(count(&conn), 2);
    assert!(conn.is_autocommit());

    // The transaction stays active.
    let mut tx = conn.transaction().unwrap();
    tx.set_drop_behavior(DropBehavior::Ignore);
    insert(&tx);
    tx.finish().unwrap();
    assert!(!conn.is_autocommit());
    assert_eq!(count(&conn), 3);
    // Closing the connection rolls it back, see the test of a file below.
    assert!(conn.transaction().is_err());
}

#[test]
fn transaction_panic_drop_behavior() {
    let mut conn = memory();
    let result = catch_unwind(AssertUnwindSafe(|| {
        let mut tx = conn.transaction().unwrap();
        tx.set_drop_behavior(DropBehavior::Panic);
        insert(&tx);
    }));
    let message = *result.unwrap_err().downcast::<String>().unwrap();
    assert_eq!(
        message,
        "a transaction was dropped without being committed or rolled back"
    );
    // It was rolled back.
    assert!(conn.is_autocommit());
    assert_eq!(count(&conn), 0);

    // Committing doesn't panic.
    let mut tx = conn.transaction().unwrap();
    tx.set_drop_behavior(DropBehavior::Panic);
    insert(&tx);
    tx.commit().unwrap();
    assert_eq!(count(&conn), 1);

    // It doesn't panic again while the thread is panicking.
    let result = catch_unwind(AssertUnwindSafe(|| {
        let mut tx = conn.transaction().unwrap();
        tx.set_drop_behavior(DropBehavior::Panic);
        insert(&tx);
        panic!("something else");
    }));
    assert_eq!(
        *result.unwrap_err().downcast::<&str>().unwrap(),
        "something else"
    );
    assert!(conn.is_autocommit());
    assert_eq!(count(&conn), 1);
}

#[test]
fn savepoint_drop_behavior() {
    let mut conn = memory();
    let mut tx = conn.transaction().unwrap();

    let sp = tx.savepoint().unwrap();
    assert_eq!(sp.drop_behavior(), DropBehavior::Rollback);
    insert(&sp);
    sp.finish().unwrap();
    assert_eq!(count(&tx), 0);

    let mut sp = tx.savepoint().unwrap();
    sp.set_drop_behavior(DropBehavior::Commit);
    assert_eq!(sp.drop_behavior(), DropBehavior::Commit);
    insert(&sp);
    drop(sp);
    assert_eq!(count(&tx), 1);

    let mut sp = tx.savepoint().unwrap();
    sp.set_drop_behavior(DropBehavior::Commit);
    insert(&sp);
    sp.finish().unwrap();
    assert_eq!(count(&tx), 2);

    // The savepoint stays active, so it can still be rolled back to.
    let mut sp = tx.savepoint_with_name("kept").unwrap();
    sp.set_drop_behavior(DropBehavior::Ignore);
    insert(&sp);
    sp.finish().unwrap();
    assert_eq!(count(&tx), 3);
    let mut sp = tx.savepoint().unwrap();
    insert(&sp);
    sp.set_drop_behavior(DropBehavior::Panic);
    let result = catch_unwind(AssertUnwindSafe(|| drop(sp)));
    assert_eq!(
        *result.unwrap_err().downcast::<String>().unwrap(),
        "a savepoint was dropped without being committed or rolled back"
    );
    assert_eq!(count(&tx), 3);
    tx.commit().unwrap();
    assert_eq!(count(&conn), 3);
}

#[test]
fn savepoints_that_start_transactions() {
    let mut conn = memory();

    let mut sp = conn.savepoint().unwrap();
    sp.set_drop_behavior(DropBehavior::Commit);
    insert(&sp);
    drop(sp);
    assert!(conn.is_autocommit());
    assert_eq!(count(&conn), 1);

    let mut sp = conn.savepoint().unwrap();
    insert(&sp);
    sp.set_drop_behavior(DropBehavior::Panic);
    assert!(catch_unwind(AssertUnwindSafe(|| drop(sp))).is_err());
    assert!(conn.is_autocommit());
    assert_eq!(count(&conn), 1);

    let sp = conn.savepoint().unwrap();
    insert(&sp);
    sp.finish().unwrap();
    assert_eq!(count(&conn), 1);
}

#[test]
fn savepoint_with() {
    let mut conn = memory();
    let options = TransactionOptions::default()
        .isolation_level(IsolationLevel::Serializable)
        .read_only(true);
    let sp = conn.savepoint_with(options).unwrap();
    assert!(sp.execute("INSERT INTO foo (x) VALUES (1)", ()).is_err());
    drop(sp);
    assert!(conn.is_autocommit());

    let sp = conn
        .savepoint_with(TransactionOptions::default().isolation_level(IsolationLevel::Serializable))
        .unwrap();
    insert(&sp);
    sp.commit().unwrap();
    assert_eq!(count(&conn), 1);
}

#[test]
fn unchecked_transaction() {
    let conn = std::rc::Rc::new(memory());
    let shared = std::rc::Rc::clone(&conn);

    let tx = conn.unchecked_transaction().unwrap();
    insert(&tx);
    // The connection is already in a transaction.
    assert!(shared.unchecked_transaction().is_err());
    assert_eq!(count(&shared), 1);
    tx.rollback().unwrap();
    assert_eq!(count(&shared), 0);

    let mut tx = shared.unchecked_transaction().unwrap();
    insert(&tx);
    {
        let sp = tx.savepoint().unwrap();
        insert(&sp);
    }
    tx.commit().unwrap();
    assert_eq!(count(&conn), 1);
}

#[test]
fn ignored_transactions_are_rolled_back_when_the_connection_closes() {
    let dir = std::env::temp_dir().join(format!("vsql-rust-drop-behavior-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("ignore.vsql");
    let _ = std::fs::remove_file(&path);
    {
        let mut conn = Connection::open(&path).unwrap();
        conn.execute("CREATE TABLE foo (x INTEGER)", ()).unwrap();
        let mut tx = conn.transaction().unwrap();
        tx.set_drop_behavior(DropBehavior::Ignore);
        insert(&tx);
        drop(tx);
        conn.close().unwrap();
    }
    let conn = Connection::open(&path).unwrap();
    assert_eq!(count(&conn), 0);
    drop(conn);
    std::fs::remove_dir_all(&dir).unwrap();
}

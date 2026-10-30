use std::thread;
use vsql_sys::*;

extern "C" {
    fn GC_get_gc_no() -> usize;
}

/// Runs a query (which returns one BIGINT) many times on a new connection.
unsafe fn run(sql: &str, times: usize) {
    let mut env = 0;
    let mut dbc = 0;
    let mut stmt = 0;
    assert_eq!(SQLAllocHandle(SQL_HANDLE_ENV, 0, &mut env), SQL_SUCCESS);
    assert_eq!(SQLAllocHandle(SQL_HANDLE_DBC, env, &mut dbc), SQL_SUCCESS);
    let name = b":memory:";
    assert_eq!(
        SQLConnect(
            dbc,
            name.as_ptr(),
            name.len() as i16,
            std::ptr::null(),
            0,
            std::ptr::null(),
            0
        ),
        SQL_SUCCESS
    );
    assert_eq!(SQLAllocHandle(SQL_HANDLE_STMT, dbc, &mut stmt), SQL_SUCCESS);
    for _ in 0..times {
        assert_eq!(
            SQLExecDirect(stmt, sql.as_ptr(), sql.len() as i32),
            SQL_SUCCESS
        );
        let mut x: i64 = 0;
        let mut ind = 0;
        assert_eq!(SQLFetch(stmt), SQL_SUCCESS);
        assert_eq!(
            SQLGetData(
                stmt,
                1,
                SQL_BIGINT,
                &mut x as *mut i64 as SQLPOINTER,
                8,
                &mut ind
            ),
            SQL_SUCCESS
        );
        assert_eq!(x, 1);
        SQLCloseCursor(stmt);
    }
    SQLFreeHandle(SQL_HANDLE_STMT, stmt);
    SQLDisconnect(dbc);
    SQLFreeHandle(SQL_HANDLE_DBC, dbc);
    SQLFreeHandle(SQL_HANDLE_ENV, env);
}

/// The first call initializes libvsql (and its garbage collector) on a thread
/// that then exits. The collector must still be able to stop the world
/// afterwards, while other threads are in libvsql.
#[test]
fn first_call_from_a_thread_that_exits() {
    thread::spawn(|| unsafe { run("VALUES 1", 1) })
        .join()
        .unwrap();

    // A large literal makes each statement allocate enough for the garbage
    // collector to run many times.
    let sql = format!(
        "VALUES CASE WHEN '{}' = '' THEN 0 ELSE 1 END",
        "x".repeat(5_000)
    );
    let before = unsafe { GC_get_gc_no() };
    let threads: Vec<_> = (0..8)
        .map(|_| {
            let sql = sql.clone();
            thread::spawn(move || unsafe { run(&sql, 20) })
        })
        .collect();
    for t in threads {
        t.join().unwrap();
    }
    let collections = unsafe { GC_get_gc_no() } - before;
    eprintln!("{collections} collections");
    assert!(collections > 0);
}

// libvsql initializes itself in the first routine that's called, so this is in
// its own test binary (process) to be the first use of libvsql.

use std::sync::{Arc, Barrier};
use std::thread;

use vsql::Connection;

/// Many threads open their first connection at the same time.
#[test]
fn concurrent_first_use() {
    let threads = 16;
    let barrier = Arc::new(Barrier::new(threads));
    let handles: Vec<_> = (0..threads)
        .map(|_| {
            let barrier = Arc::clone(&barrier);
            thread::spawn(move || {
                barrier.wait();
                let conn = Connection::open_in_memory().unwrap();
                let x: i64 = conn.query_row("VALUES 1", (), |r| r.get(0)).unwrap();
                assert_eq!(x, 1);
            })
        })
        .collect();
    for handle in handles {
        handle.join().unwrap();
    }
}

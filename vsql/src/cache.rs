//! The prepared statement cache of a connection, see
//! [`Connection::prepare_cached`](crate::Connection::prepare_cached).

use std::collections::VecDeque;

use vsql_sys::*;

use crate::raw;

/// The number of statements that a connection's cache keeps, unless it's
/// changed with `set_prepared_statement_cache_capacity`.
pub(crate) const DEFAULT_CAPACITY: usize = 16;

/// The prepared statements that aren't in use (by their SQL), the least
/// recently used first.
pub(crate) struct StatementCache {
    capacity: usize,
    stmts: VecDeque<(String, SQLHSTMT)>,
}

impl StatementCache {
    pub(crate) fn new() -> StatementCache {
        StatementCache {
            capacity: DEFAULT_CAPACITY,
            stmts: VecDeque::new(),
        }
    }

    /// Removes a statement from the cache, so it can be used.
    pub(crate) fn take(&mut self, sql: &str) -> Option<SQLHSTMT> {
        let i = self.stmts.iter().position(|(s, _)| s == sql)?;

        self.stmts.remove(i).map(|(_, stmt)| stmt)
    }

    /// Adds a statement that's no longer used, as the most recently used.
    pub(crate) fn put(&mut self, sql: String, stmt: SQLHSTMT) {
        // There can be two statements of the same SQL when both were in use.
        if let Some(old) = self.take(&sql) {
            free(old);
        }
        self.stmts.push_back((sql, stmt));
        self.evict();
    }

    pub(crate) fn set_capacity(&mut self, capacity: usize) {
        self.capacity = capacity;
        self.evict();
    }

    /// Frees every statement in the cache.
    pub(crate) fn flush(&mut self) {
        for (_, stmt) in self.stmts.drain(..) {
            free(stmt);
        }
    }

    fn evict(&mut self) {
        while self.stmts.len() > self.capacity {
            if let Some((_, stmt)) = self.stmts.pop_front() {
                free(stmt);
            }
        }
    }

    #[cfg(test)]
    fn sqls(&self) -> Vec<&str> {
        self.stmts.iter().map(|(sql, _)| sql.as_str()).collect()
    }
}

fn free(stmt: SQLHSTMT) {
    let _ = raw::free_handle(SQL_HANDLE_STMT, stmt);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn least_recently_used_statements_are_evicted() {
        // The handles aren't allocated, so freeing them fails (and is ignored).
        let _conn = crate::Connection::open_in_memory().unwrap();
        let mut cache = StatementCache::new();
        cache.set_capacity(2);
        cache.put("a".into(), 999_991);
        cache.put("b".into(), 999_992);
        assert_eq!(cache.take("a"), Some(999_991));
        assert_eq!(cache.take("a"), None);
        cache.put("a".into(), 999_991);
        cache.put("c".into(), 999_993);
        assert_eq!(cache.sqls(), ["a", "c"]);

        // The second statement of the same SQL replaces the first.
        cache.put("c".into(), 999_994);
        assert_eq!(cache.sqls(), ["a", "c"]);
        assert_eq!(cache.take("c"), Some(999_994));

        cache.set_capacity(0);
        assert!(cache.sqls().is_empty());
        cache.set_capacity(2);
        cache.put("d".into(), 999_995);
        cache.flush();
        assert!(cache.sqls().is_empty());
    }
}

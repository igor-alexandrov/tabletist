//! The query history a video's run keeps, in memory: the runner's backend
//! writes no file. It answers a read as the history's file does, a
//! connection's newest entries first, at most as many as asked.
//!
//! No GPU here, so the tests run in the normal suite.

use crate::history::Entry;

/// The newest `keep` of `connection`'s entries in `kept`, newest first.
/// `kept` is in the order the entries were added.
pub fn newest(kept: &[Entry], connection: &str, keep: u32) -> Vec<Entry> {
    kept.iter()
        .rev()
        .filter(|entry| entry.connection == connection)
        .take(usize::try_from(keep).unwrap_or(usize::MAX))
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::history::{Kind, Outcome, Source};

    fn ran(connection: &str, sql: &str) -> Entry {
        Entry {
            at_ms: 1_792_000_000_000,
            source: Source::Sql,
            detail: "Query 1".into(),
            tool: None,
            connection: connection.into(),
            database: "bookshop.sqlite3".into(),
            sql: Some(sql.into()),
            call: None,
            kind: Kind::Read,
            read_only: true,
            rows: Some(6),
            took_ms: Some(3),
            outcome: Outcome::Ok,
            code: None,
            message: None,
            query: None,
        }
    }

    fn statements(entries: &[Entry]) -> Vec<&str> {
        entries
            .iter()
            .map(|entry| entry.sql.as_deref().unwrap_or_default())
            .collect()
    }

    #[test]
    fn a_read_takes_the_connection_s_newest_first_and_no_more_than_asked() {
        let kept = [
            ran("bookshop", "SELECT 1"),
            ran("other", "SELECT 2"),
            ran("bookshop", "SELECT 3"),
            ran("bookshop", "SELECT 4"),
        ];
        assert_eq!(
            statements(&newest(&kept, "bookshop", 10)),
            ["SELECT 4", "SELECT 3", "SELECT 1"]
        );
        assert_eq!(
            statements(&newest(&kept, "bookshop", 2)),
            ["SELECT 4", "SELECT 3"]
        );
        assert_eq!(statements(&newest(&kept, "other", 10)), ["SELECT 2"]);
        assert!(newest(&kept, "nobody", 10).is_empty());
        assert!(newest(&kept, "bookshop", 0).is_empty());
    }
}

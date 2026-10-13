//! The shared fixtures: the one the driver tests and the app's demo mode
//! use, and the Bookshop, which what an agent may read is tested on.

use std::path::Path;

use crate::{Error, Result};

/// The fixture script.
pub const SQLITE_SQL: &str = include_str!("../fixtures/sqlite.sql");

/// Creates a new SQLite file at `path` holding the fixture. Refuses to touch
/// an existing file, so it can never be pointed at a user's database.
pub fn write_sqlite_demo(path: &Path) -> Result<()> {
    write_sqlite(path, SQLITE_SQL)
}

/// The Bookshop as a SQLite script: the made-up shop of the app's
/// screenshots, as tables.
pub const BOOKSHOP_SQLITE_SQL: &str = include_str!("../fixtures/bookshop/sqlite.sql");

/// Creates a new SQLite file at `path` holding the Bookshop. Refuses to
/// touch an existing file, as [`write_sqlite_demo`] does.
pub fn write_sqlite_bookshop(path: &Path) -> Result<()> {
    write_sqlite(path, BOOKSHOP_SQLITE_SQL)
}

/// A new SQLite file at `path` made by `sql`.
fn write_sqlite(path: &Path, sql: &str) -> Result<()> {
    if path.exists() {
        return Err(Error::Io(format!("{} already exists", path.display())));
    }
    let flags =
        rusqlite::OpenFlags::SQLITE_OPEN_READ_WRITE | rusqlite::OpenFlags::SQLITE_OPEN_CREATE;
    let connection = rusqlite::Connection::open_with_flags(path, flags)
        .map_err(|error| Error::Io(error.to_string()))?;
    connection
        .execute_batch(sql)
        .map_err(|error| Error::Io(error.to_string()))
}

/// The PostgreSQL fixture script, loaded by the integration tests through
/// an admin connection.
pub const POSTGRES_SQL: &str = include_str!("../fixtures/postgres.sql");

/// The MySQL fixture script (one statement per `;` at the end of a line).
pub const MYSQL_SQL: &str = include_str!("../fixtures/mysql.sql");

/// The Bookshop for PostgreSQL. The integration tests load it into a
/// database of its own, `bookshop`.
pub const BOOKSHOP_POSTGRES_SQL: &str = include_str!("../fixtures/bookshop/postgres.sql");

/// The Bookshop for MySQL, for a database of its own, `bookshop` (one
/// statement per `;` at the end of a line).
pub const BOOKSHOP_MYSQL_SQL: &str = include_str!("../fixtures/bookshop/mysql.sql");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_bookshop_is_written_to_a_new_file_only() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("bookshop.db");
        write_sqlite_bookshop(&path).unwrap();
        let connection = rusqlite::Connection::open(&path).unwrap();
        let count = |table: &str| -> i64 {
            connection
                .query_row(&format!("SELECT count(*) FROM {table}"), [], |row| {
                    row.get(0)
                })
                .unwrap()
        };
        assert_eq!(count("books"), 12);
        assert_eq!(count("order_items"), 40);
        assert_eq!(count("recent_orders"), 7);
        assert_eq!(count("customer_names"), 8);
        // A file that is there is left as it is.
        let again = write_sqlite_bookshop(&path).unwrap_err();
        assert!(again.to_string().ends_with("already exists"), "{again}");
        assert_eq!(count("books"), 12);
    }
}

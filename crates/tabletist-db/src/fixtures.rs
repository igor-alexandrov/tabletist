//! The shared SQLite fixture, for tests and the app's demo mode.

use std::path::Path;

use crate::{Error, Result};

/// The fixture script.
pub const SQLITE_SQL: &str = include_str!("../fixtures/sqlite.sql");

/// Creates a new SQLite file at `path` holding the fixture. Refuses to touch
/// an existing file, so it can never be pointed at a user's database.
pub fn write_sqlite_demo(path: &Path) -> Result<()> {
    if path.exists() {
        return Err(Error::Io(format!("{} already exists", path.display())));
    }
    let flags =
        rusqlite::OpenFlags::SQLITE_OPEN_READ_WRITE | rusqlite::OpenFlags::SQLITE_OPEN_CREATE;
    let connection = rusqlite::Connection::open_with_flags(path, flags)
        .map_err(|error| Error::Io(error.to_string()))?;
    connection
        .execute_batch(SQLITE_SQL)
        .map_err(|error| Error::Io(error.to_string()))
}

/// The PostgreSQL fixture script, loaded by the integration tests through
/// an admin connection.
pub const POSTGRES_SQL: &str = include_str!("../fixtures/postgres.sql");

/// The MySQL fixture script (one statement per `;` at the end of a line).
pub const MYSQL_SQL: &str = include_str!("../fixtures/mysql.sql");

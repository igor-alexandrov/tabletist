//! The shared fixtures: the one the driver tests and the app's demo mode
//! use, the Bookshop, which what an agent may read is tested on, and the
//! Bookshop's exports, which Files connections are tested on.

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

/// The files [`write_bookshop_files`] writes, by name.
pub const BOOKSHOP_FILES: [&str; 7] = [
    "authors.tsv",
    "books.json",
    "bookshop.duckdb",
    "customers.parquet",
    "events.ndjson",
    "orders.csv",
    "returns.csv",
];

/// Writes the Bookshop's exports into the folder `dir`, as a shop would
/// export them: `orders.csv`, `returns.csv`, `authors.tsv` (tab
/// separated), `customers.parquet`, `books.json` (an array),
/// `events.ndjson` (an order placed, a line each), and `bookshop.duckdb`
/// holding `book_covers` and `order_items`. Refuses a folder that holds
/// any of them already.
pub fn write_bookshop_files(dir: &Path) -> Result<()> {
    let io = |error: &dyn std::fmt::Display| Error::Io(error.to_string());
    if let Some(there) = BOOKSHOP_FILES.iter().find(|name| dir.join(name).exists()) {
        return Err(Error::Io(format!("{there} already exists")));
    }
    let shop = rusqlite::Connection::open_in_memory().map_err(|e| io(&e))?;
    shop.execute_batch(BOOKSHOP_SQLITE_SQL)
        .map_err(|e| io(&e))?;
    let duck = duckdb::Connection::open_in_memory().map_err(|e| io(&e))?;
    // Each table as DuckDB's, from its rows as literals.
    for table in [
        "authors",
        "books",
        "customers",
        "orders",
        "order_items",
        "book_covers",
    ] {
        let mut statement = shop
            .prepare(&format!("SELECT * FROM {table} ORDER BY id"))
            .map_err(|e| io(&e))?;
        let names: Vec<String> = statement
            .column_names()
            .into_iter()
            .map(|name| crate::Dialect::DuckDb.quote_ident(name))
            .collect();
        let width = names.len();
        let rows: Vec<String> = statement
            .query_map([], |row| {
                let cells: Vec<String> = (0..width)
                    .map(|at| {
                        use rusqlite::types::ValueRef;
                        Ok(match row.get_ref(at)? {
                            ValueRef::Null => "NULL".to_owned(),
                            ValueRef::Integer(number) => number.to_string(),
                            ValueRef::Real(number) => format!("{number:?}"),
                            ValueRef::Text(text) => {
                                crate::Dialect::DuckDb.literal(&String::from_utf8_lossy(text))
                            }
                            ValueRef::Blob(bytes) => crate::Dialect::DuckDb.bytes_literal(bytes),
                        })
                    })
                    .collect::<rusqlite::Result<_>>()?;
                Ok(format!("({})", cells.join(", ")))
            })
            .map_err(|e| io(&e))?
            .collect::<rusqlite::Result<_>>()
            .map_err(|e| io(&e))?;
        duck.execute_batch(&format!(
            "CREATE TABLE {table} AS SELECT * FROM (VALUES {}) AS t({})",
            rows.join(", "),
            names.join(", ")
        ))
        .map_err(|e| io(&e))?;
    }
    let path = |name: &str| crate::Dialect::DuckDb.literal(&dir.join(name).to_string_lossy());
    duck.execute_batch(&format!(
        "COPY orders TO {} (HEADER, DELIMITER ',');
         COPY (SELECT id AS order_id, placed_at::DATE + 9 AS returned_at,
                      'damaged' AS reason
               FROM orders WHERE id % 3 = 0)
           TO {} (HEADER, DELIMITER ',');
         COPY authors TO {} (HEADER, DELIMITER '\t');
         COPY customers TO {} (FORMAT parquet, COMPRESSION zstd);
         COPY books TO {} (FORMAT json, ARRAY true);
         COPY (SELECT id AS order_id, 'placed' AS kind, placed_at AS at FROM orders)
           TO {} (FORMAT json);
         ATTACH {} AS shop;
         CREATE TABLE shop.book_covers AS SELECT * FROM book_covers;
         CREATE TABLE shop.order_items AS SELECT * FROM order_items;
         DETACH shop;",
        path("orders.csv"),
        path("returns.csv"),
        path("authors.tsv"),
        path("customers.parquet"),
        path("books.json"),
        path("events.ndjson"),
        path("bookshop.duckdb"),
    ))
    .map_err(|e| io(&e))
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

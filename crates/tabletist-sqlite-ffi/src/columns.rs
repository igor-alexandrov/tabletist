//! A statement's result columns, read as bytes.
//!
//! SQLite keeps a name's bytes as they were written and never checks that
//! they are UTF-8. rusqlite reads a result column's name and declared type as
//! `str` and panics on such bytes, and a release build of the app aborts on a
//! panic. [`result_columns`] reads the same bytes through SQLite's C API and
//! replaces what is not UTF-8.

// Preparing a statement and reading its columns are calls into C.
#![allow(unsafe_code)]

use std::ffi::{CStr, c_char, c_int};
use std::ptr;

use rusqlite::ffi;

/// A result column of a statement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResultColumn {
    /// The column's name, with U+FFFD for bytes that are not UTF-8.
    pub name: String,
    /// The declared type of the table column the result column reads, with
    /// U+FFFD for bytes that are not UTF-8. Empty for an expression.
    pub decl_type: String,
}

/// A prepared statement, finalized when dropped.
struct Prepared(*mut ffi::sqlite3_stmt);

impl Drop for Prepared {
    fn drop(&mut self) {
        // SAFETY: the pointer is null or came from `sqlite3_prepare_v2`, and
        // nothing else finalizes it. Finalizing null does nothing.
        unsafe { ffi::sqlite3_finalize(self.0) };
    }
}

/// The C string at `text` with U+FFFD for bytes that are not UTF-8, or `None`
/// for a null pointer.
///
/// # Safety
///
/// `text` is null, or points to a NUL-terminated string that stays valid
/// until this returns.
unsafe fn lossy(text: *const c_char) -> Option<String> {
    if text.is_null() {
        return None;
    }
    // SAFETY: not null, so a NUL-terminated string by this function's
    // contract. It is copied before this returns.
    let text = unsafe { CStr::from_ptr(text) };
    Some(text.to_string_lossy().into_owned())
}

fn failure(code: c_int, message: Option<String>) -> rusqlite::Error {
    rusqlite::Error::SqliteFailure(ffi::Error::new(code), message)
}

/// The result columns of the first statement in `sql`, in order. None for a
/// statement that returns no rows, and for text that holds no statement.
///
/// The statement is prepared on `connection` to read them and is never run.
/// Text that SQLite does not prepare is SQLite's error.
pub fn result_columns(
    connection: &rusqlite::Connection,
    sql: &str,
) -> rusqlite::Result<Vec<ResultColumn>> {
    if sql.is_empty() {
        return Ok(Vec::new());
    }
    let length = c_int::try_from(sql.len()).map_err(|_| failure(ffi::SQLITE_TOOBIG, None))?;
    // SAFETY: the handle is used only below, while `connection` is borrowed,
    // and only to prepare a statement that is finalized before this returns.
    // The connection is left as it was.
    let db = unsafe { connection.handle() };
    let mut statement = Prepared(ptr::null_mut());
    // SAFETY: `db` is an open connection. `sql` points to `length` bytes and
    // SQLite reads no more than that many. `statement.0` is a place for the
    // statement's pointer, and a null tail pointer asks for no tail.
    let code = unsafe {
        ffi::sqlite3_prepare_v2(
            db,
            sql.as_ptr().cast::<c_char>(),
            length,
            &mut statement.0,
            ptr::null_mut(),
        )
    };
    if code != ffi::SQLITE_OK {
        // SAFETY: `db` is an open connection. Its message is a NUL-terminated
        // string that stays valid until the next call on the connection, and
        // `lossy` copies it first.
        let message = unsafe { lossy(ffi::sqlite3_errmsg(db)) };
        return Err(failure(code, message));
    }
    // SAFETY: `statement.0` is a prepared statement, or null when the text
    // holds no statement, which SQLite counts as no columns.
    let count = unsafe { ffi::sqlite3_column_count(statement.0) };
    (0..count)
        .map(|index| {
            // SAFETY: `statement.0` is a prepared statement and `index` is
            // one of its columns. Each string is NUL-terminated and stays
            // valid until the next call for the same column or the
            // statement's end, and `lossy` copies it first.
            let (name, decl_type) = unsafe {
                (
                    lossy(ffi::sqlite3_column_name(statement.0, index)),
                    lossy(ffi::sqlite3_column_decltype(statement.0, index)),
                )
            };
            Ok(ResultColumn {
                // SQLite has a name for every column; it returns none only
                // when it could not allocate one.
                name: name.ok_or_else(|| failure(ffi::SQLITE_NOMEM, None))?,
                decl_type: decl_type.unwrap_or_default(),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A database in memory after running `sql`, which SQLite takes as
    /// bytes. rusqlite only runs a `str`.
    fn database(sql: &[u8]) -> rusqlite::Connection {
        let connection = rusqlite::Connection::open_in_memory().unwrap();
        let sql = std::ffi::CString::new(sql).unwrap();
        // SAFETY: the connection is open and `sql` is a NUL-terminated
        // string. No callback, no callback argument and no place for an
        // error message are passed.
        let code = unsafe {
            ffi::sqlite3_exec(
                connection.handle(),
                sql.as_ptr(),
                None,
                ptr::null_mut(),
                ptr::null_mut(),
            )
        };
        assert_eq!(code, ffi::SQLITE_OK);
        connection
    }

    fn column(name: &str, decl_type: &str) -> ResultColumn {
        ResultColumn {
            name: name.into(),
            decl_type: decl_type.into(),
        }
    }

    #[test]
    fn names_and_declared_types_that_are_not_utf8_are_read_with_the_byte_replaced() {
        let connection =
            database(b"CREATE TABLE t (id INTEGER PRIMARY KEY, caf\xe9 TEXT, n caf\xe9)");
        assert_eq!(
            result_columns(&connection, "SELECT * FROM t").unwrap(),
            [
                column("id", "INTEGER"),
                column("caf\u{FFFD}", "TEXT"),
                column("n", "caf\u{FFFD}"),
            ]
        );
    }

    #[test]
    fn an_expression_has_a_name_and_no_declared_type() {
        let connection = database(b"CREATE TABLE t (id INTEGER)");
        assert_eq!(
            result_columns(&connection, "SELECT id + 1, count(*) AS n FROM t").unwrap(),
            [column("id + 1", ""), column("n", "")]
        );
    }

    #[test]
    fn a_statement_without_rows_and_text_without_a_statement_have_no_columns() {
        let connection = database(b"CREATE TABLE t (id INTEGER)");
        for sql in ["DELETE FROM t", "-- nothing to run", " ", ""] {
            assert_eq!(result_columns(&connection, sql).unwrap(), [], "{sql:?}");
        }
    }

    #[test]
    fn only_the_first_statement_is_read() {
        let connection = database(b"CREATE TABLE t (id INTEGER)");
        assert_eq!(
            result_columns(&connection, "SELECT 1 AS a; SELECT 2 AS b").unwrap(),
            [column("a", "")]
        );
    }

    #[test]
    fn text_that_does_not_prepare_is_sqlites_error() {
        let connection = database(b"CREATE TABLE t (id INTEGER)");
        match result_columns(&connection, "SELECT * FROM missing") {
            Err(rusqlite::Error::SqliteFailure(error, Some(message))) => {
                assert_eq!(error.code, rusqlite::ErrorCode::Unknown);
                assert_eq!(message, "no such table: missing");
            }
            other => panic!("a failure with SQLite's message, not {other:?}"),
        }
    }

    #[test]
    fn the_statement_is_not_run_and_is_finalized() {
        let connection = database(b"CREATE TABLE t (id INTEGER)");
        assert_eq!(
            result_columns(&connection, "INSERT INTO t VALUES (1) RETURNING id").unwrap(),
            [column("id", "INTEGER")]
        );
        let rows: i64 = connection
            .query_row("SELECT count(*) FROM t", [], |row| row.get(0))
            .unwrap();
        assert_eq!(rows, 0);
        // SQLite refuses to close a connection with a statement left on it.
        assert!(connection.close().is_ok());
    }
}

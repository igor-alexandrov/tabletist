//! What the tests of this crate share.

// Running SQL given as bytes is a call into C.
#![allow(unsafe_code)]

use std::ptr;

use rusqlite::ffi;

/// A database in memory after running `sql`, which SQLite takes as bytes.
/// rusqlite only runs a `str`.
pub(crate) fn database(sql: &[u8]) -> rusqlite::Connection {
    let connection = rusqlite::Connection::open_in_memory().unwrap();
    let sql = std::ffi::CString::new(sql).unwrap();
    // SAFETY: the connection is open and `sql` is a NUL-terminated string.
    // No callback, no callback argument and no place for an error message
    // are passed.
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

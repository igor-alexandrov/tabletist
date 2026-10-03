//! A connection's authorizer, asked with names read as bytes.
//!
//! SQLite asks a connection's authorizer about every action of a statement
//! it prepares and hands it the names the action is on. It keeps a name's
//! bytes as they were written and never checks that they are UTF-8.
//! rusqlite's authorizer reads the names as `str` and panics on such bytes:
//! a debug build denies the action, and a release build of the app aborts on
//! a panic. [`set_authorizer`] installs one that reads the same bytes and
//! replaces what is not UTF-8.

// Installing a callback and reading what SQLite hands it are calls into C.
#![allow(unsafe_code)]

use std::borrow::Cow;
use std::ffi::{CStr, c_char, c_int, c_void};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::ptr;

use rusqlite::ffi;

use crate::failure;

/// What SQLite asks its authorizer about. Every name has U+FFFD for bytes
/// that are not UTF-8.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Action<'a> {
    /// SQLite's action code, such as `SQLITE_READ` or `SQLITE_PRAGMA`.
    pub code: i32,
    /// The first name SQLite gives for the code: a table for a read, a
    /// pragma's name for a pragma.
    pub first: Option<&'a str>,
    /// The second one: a column for a read, a pragma's value.
    pub second: Option<&'a str>,
    /// The database the action is on, such as `main`.
    pub database: Option<&'a str>,
    /// The innermost trigger or view the action comes from. None for what
    /// the statement's own text does.
    pub source: Option<&'a str>,
}

/// An authorizer's answer. SQLite's third one, to ignore the action, is left
/// out: nothing here gives it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Authorization {
    /// SQLite goes on.
    Allow,
    /// SQLite refuses the statement, with its error 23.
    Deny,
}

/// The name SQLite keeps the installed callback under.
const KEPT_AS: &CStr = c"tabletist-sqlite-ffi authorizer";

/// The C string at `text` with U+FFFD for bytes that are not UTF-8, or `None`
/// for a null pointer. Copied only when a byte is replaced.
///
/// # Safety
///
/// `text` is null, or points to a NUL-terminated string that stays valid and
/// unchanged for `'a`.
unsafe fn lossy<'a>(text: *const c_char) -> Option<Cow<'a, str>> {
    if text.is_null() {
        return None;
    }
    // SAFETY: not null, so a NUL-terminated string that outlives `'a` by
    // this function's contract.
    let text = unsafe { CStr::from_ptr(text) };
    Some(text.to_string_lossy())
}

/// What SQLite calls for every action: asks the `F` at `callback`. A panic
/// in it denies the action and does not unwind into SQLite.
///
/// # Safety
///
/// `callback` points to a live `F`, and each of the four names is null or a
/// NUL-terminated string that stays valid until this returns.
unsafe extern "C" fn ask<F>(
    callback: *mut c_void,
    code: c_int,
    first: *const c_char,
    second: *const c_char,
    database: *const c_char,
    source: *const c_char,
) -> c_int
where
    F: Fn(&Action<'_>) -> Authorization,
{
    // Nothing is left half done by a panic here: the names are only read,
    // and the callback is only called through a shared reference.
    let answer = catch_unwind(AssertUnwindSafe(|| {
        // SAFETY: each name is null or a NUL-terminated string that is valid
        // until this function returns, by its contract, and none is kept
        // longer than that.
        let (first, second, database, source) =
            unsafe { (lossy(first), lossy(second), lossy(database), lossy(source)) };
        // SAFETY: a live `F` by this function's contract. Only shared
        // references to it are ever made.
        let callback = unsafe { &*callback.cast::<F>() };
        callback(&Action {
            code,
            first: first.as_deref(),
            second: second.as_deref(),
            database: database.as_deref(),
            source: source.as_deref(),
        })
    }));
    match answer {
        Ok(Authorization::Allow) => ffi::SQLITE_OK,
        Ok(Authorization::Deny) | Err(_) => ffi::SQLITE_DENY,
    }
}

/// What SQLite calls to free the callback it kept.
///
/// # Safety
///
/// `callback` came from `Box::<F>::into_raw`, nothing uses it any more and
/// this is the only call that frees it.
unsafe extern "C" fn free<F>(callback: *mut c_void) {
    // SAFETY: a `Box<F>` turned into a pointer, handed back once, by this
    // function's contract.
    let callback = unsafe { Box::from_raw(callback.cast::<F>()) };
    // A callback that panics as it drops must not unwind into SQLite.
    let _ = catch_unwind(AssertUnwindSafe(|| drop(callback)));
}

/// Has SQLite ask `authorizer` about every action of a statement it prepares
/// on `connection`, in place of the authorizer it had.
///
/// The authorizer is dropped when another one takes its place, when
/// [`remove_authorizer`] takes it off and when the connection closes. A
/// panic in it denies the action. On an error the connection is left without
/// an authorizer.
///
/// rusqlite takes off whatever authorizer is installed when it closes a
/// connection, so one that fails to close has none afterwards.
pub fn set_authorizer<F>(connection: &rusqlite::Connection, authorizer: F) -> rusqlite::Result<()>
where
    F: Fn(&Action<'_>) -> Authorization + Send + 'static,
{
    // SAFETY: the handle is used only below, while `connection` is borrowed.
    // A connection is not `Sync`, so no other thread prepares a statement on
    // it meanwhile.
    let db = unsafe { connection.handle() };
    let callback = Box::into_raw(Box::new(authorizer)).cast::<c_void>();
    // SAFETY: `db` is an open connection. `ask::<F>` gets `callback`, which
    // points to an `F` that lives until `free::<F>` drops it, and that is
    // not before SQLite has stopped calling `ask::<F>` with it: see below.
    // `F` is `Send`, so it may be called on whichever thread the connection
    // moves to.
    let code = unsafe { ffi::sqlite3_set_authorizer(db, Some(ask::<F>), callback) };
    if code != ffi::SQLITE_OK {
        // SAFETY: SQLite did not take the pointer, so the box made above is
        // still this function's to free.
        drop(unsafe { Box::from_raw(callback.cast::<F>()) });
        return Err(failure(code, None));
    }
    // SAFETY: `db` is an open connection and the name is a NUL-terminated
    // string, which SQLite copies. SQLite calls `free::<F>` with `callback`
    // once: when other data is kept under the name or when the connection
    // closes. Either way nothing calls the authorizer by then: this function
    // and `remove_authorizer` point SQLite away from it first, and a closed
    // connection prepares nothing. The authorizer this one replaces is freed
    // here the same way, by the `free` it was kept with, after the call
    // above took it out of use.
    let code =
        unsafe { ffi::sqlite3_set_clientdata(db, KEPT_AS.as_ptr(), callback, Some(free::<F>)) };
    if code != ffi::SQLITE_OK {
        // SQLite frees what it cannot keep, so the callback is gone and
        // nothing may call it.
        // SAFETY: `db` is an open connection, and no callback with no
        // argument takes the authorizer off.
        unsafe { ffi::sqlite3_set_authorizer(db, None, ptr::null_mut()) };
        return Err(failure(code, None));
    }
    Ok(())
}

/// Takes off the connection's authorizer and drops the one
/// [`set_authorizer`] installed. Does nothing on a connection that has none.
pub fn remove_authorizer(connection: &rusqlite::Connection) -> rusqlite::Result<()> {
    // SAFETY: the handle is used only below, while `connection` is borrowed.
    let db = unsafe { connection.handle() };
    // SAFETY: `db` is an open connection, and no callback with no argument
    // takes the authorizer off.
    let code = unsafe { ffi::sqlite3_set_authorizer(db, None, ptr::null_mut()) };
    if code != ffi::SQLITE_OK {
        return Err(failure(code, None));
    }
    // SAFETY: `db` is an open connection and the name is a NUL-terminated
    // string. Null data frees what was kept under the name, through the
    // `free` it was kept with, and SQLite no longer calls it: see above.
    let code = unsafe { ffi::sqlite3_set_clientdata(db, KEPT_AS.as_ptr(), ptr::null_mut(), None) };
    if code != ffi::SQLITE_OK {
        return Err(failure(code, None));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use super::*;
    use crate::testing::database;

    /// `caf\xe9` ("café" in Latin-1) once the byte that is not UTF-8 is
    /// replaced.
    const LOSSY: &str = "caf\u{FFFD}";

    /// An action with its names copied, to keep past the callback.
    type Asked = (
        i32,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
    );

    fn asked(
        code: i32,
        first: Option<&str>,
        second: Option<&str>,
        database: Option<&str>,
        source: Option<&str>,
    ) -> Asked {
        let owned = |name: Option<&str>| name.map(str::to_owned);
        (
            code,
            owned(first),
            owned(second),
            owned(database),
            owned(source),
        )
    }

    /// Installs an authorizer that allows everything and returns the list
    /// it writes each action to.
    fn recorded(connection: &rusqlite::Connection) -> Arc<Mutex<Vec<Asked>>> {
        let actions = Arc::new(Mutex::new(Vec::new()));
        let seen = Arc::clone(&actions);
        set_authorizer(connection, move |action| {
            seen.lock().unwrap().push(asked(
                action.code,
                action.first,
                action.second,
                action.database,
                action.source,
            ));
            Authorization::Allow
        })
        .unwrap();
        actions
    }

    /// What the authorizer was asked while SQLite prepared `sql`.
    fn prepared(
        connection: &rusqlite::Connection,
        actions: &Mutex<Vec<Asked>>,
        sql: &str,
    ) -> Vec<Asked> {
        actions.lock().unwrap().clear();
        connection.prepare(sql).unwrap();
        actions.lock().unwrap().clone()
    }

    fn count(connection: &rusqlite::Connection, sql: &str) -> rusqlite::Result<i64> {
        connection.query_row(sql, [], |row| row.get(0))
    }

    fn is_denied(result: rusqlite::Result<i64>) -> bool {
        matches!(
            result,
            Err(rusqlite::Error::SqliteFailure(error, _))
                if error.code == rusqlite::ErrorCode::AuthorizationForStatementDenied
        )
    }

    #[test]
    fn names_that_are_not_utf8_reach_the_authorizer_with_the_byte_replaced() {
        // A column, a trigger and a database with such a name.
        let connection = database(
            b"CREATE TABLE t (id INTEGER PRIMARY KEY, caf\xe9 TEXT);
              CREATE TABLE log (n INTEGER);
              CREATE TRIGGER caf\xe9 AFTER INSERT ON log
                  BEGIN UPDATE t SET id = id + 1; END;
              ATTACH ':memory:' AS caf\xe9;
              CREATE TABLE caf\xe9.far (n INTEGER);",
        );
        let actions = recorded(&connection);
        assert_eq!(
            prepared(&connection, &actions, "SELECT * FROM t"),
            [
                asked(ffi::SQLITE_SELECT, None, None, None, None),
                asked(ffi::SQLITE_READ, Some("t"), Some("id"), Some("main"), None),
                asked(ffi::SQLITE_READ, Some("t"), Some(LOSSY), Some("main"), None),
            ]
        );
        assert_eq!(
            prepared(&connection, &actions, "SELECT n FROM far"),
            [
                asked(ffi::SQLITE_SELECT, None, None, None, None),
                asked(ffi::SQLITE_READ, Some("far"), Some("n"), Some(LOSSY), None),
            ]
        );
        let inserted = prepared(&connection, &actions, "INSERT INTO log VALUES (1)");
        assert!(
            inserted.contains(&asked(
                ffi::SQLITE_UPDATE,
                Some("t"),
                Some("id"),
                Some("main"),
                Some(LOSSY),
            )),
            "{inserted:?}"
        );
    }

    #[test]
    fn a_name_that_is_not_utf8_is_allowed_and_denied_like_any_other() {
        let connection = database(
            b"CREATE TABLE t (id INTEGER PRIMARY KEY, caf\xe9 TEXT);
              INSERT INTO t VALUES (1, 'x');
              CREATE VIEW v AS SELECT id, caf\xe9 AS cafe FROM t;",
        );
        set_authorizer(&connection, |_| Authorization::Allow).unwrap();
        for sql in [
            "SELECT count(*) FROM t WHERE id = 1",
            "SELECT count(*) FROM (SELECT * FROM t)",
            "SELECT count(*) FROM (SELECT * FROM v)",
            "SELECT count(cafe) FROM v",
        ] {
            assert_eq!(count(&connection, sql).unwrap(), 1, "{sql}");
        }
        set_authorizer(&connection, |action| {
            if action.code == ffi::SQLITE_READ && action.second == Some(LOSSY) {
                Authorization::Deny
            } else {
                Authorization::Allow
            }
        })
        .unwrap();
        for sql in [
            "SELECT count(*) FROM (SELECT * FROM t)",
            "SELECT count(*) FROM (SELECT * FROM v)",
            "SELECT count(cafe) FROM v",
        ] {
            assert!(is_denied(count(&connection, sql)), "{sql}");
        }
        // The other column is still read.
        assert_eq!(
            count(&connection, "SELECT count(*) FROM t WHERE id = 1").unwrap(),
            1
        );
    }

    #[test]
    fn an_authorizer_that_panics_denies_the_action() {
        let connection = database(b"CREATE TABLE t (id INTEGER)");
        set_authorizer(&connection, |action| {
            assert_ne!(action.code, ffi::SQLITE_READ, "no column is read");
            Authorization::Allow
        })
        .unwrap();
        assert!(is_denied(count(&connection, "SELECT count(id) FROM t")));
        assert_eq!(count(&connection, "SELECT 1").unwrap(), 1);
    }

    #[test]
    fn an_authorizer_is_dropped_when_another_takes_its_place() {
        let connection = database(b"CREATE TABLE t (id INTEGER)");
        let first = Arc::new(());
        let kept = Arc::clone(&first);
        set_authorizer(&connection, move |_| {
            let _kept = &kept;
            Authorization::Deny
        })
        .unwrap();
        assert_eq!(Arc::strong_count(&first), 2);
        assert!(is_denied(count(&connection, "SELECT 1")));
        let second = Arc::new(());
        let kept = Arc::clone(&second);
        set_authorizer(&connection, move |_| {
            let _kept = &kept;
            Authorization::Allow
        })
        .unwrap();
        assert_eq!(Arc::strong_count(&first), 1);
        assert_eq!(Arc::strong_count(&second), 2);
        assert_eq!(count(&connection, "SELECT 1").unwrap(), 1);
    }

    #[test]
    fn an_authorizer_is_dropped_when_it_is_removed() {
        let connection = database(b"CREATE TABLE t (id INTEGER)");
        let token = Arc::new(());
        let kept = Arc::clone(&token);
        set_authorizer(&connection, move |_| {
            let _kept = &kept;
            Authorization::Deny
        })
        .unwrap();
        assert!(is_denied(count(&connection, "SELECT 1")));
        remove_authorizer(&connection).unwrap();
        assert_eq!(Arc::strong_count(&token), 1);
        assert_eq!(count(&connection, "SELECT 1").unwrap(), 1);
        // Nothing is left to remove, which is no error.
        remove_authorizer(&connection).unwrap();
    }

    #[test]
    fn an_authorizer_is_dropped_when_the_connection_closes() {
        for close in [
            (|connection| drop(connection)) as fn(rusqlite::Connection),
            |connection| connection.close().unwrap(),
        ] {
            let connection = database(b"CREATE TABLE t (id INTEGER)");
            let token = Arc::new(());
            let kept = Arc::clone(&token);
            set_authorizer(&connection, move |_| {
                let _kept = &kept;
                Authorization::Allow
            })
            .unwrap();
            assert_eq!(Arc::strong_count(&token), 2);
            close(connection);
            assert_eq!(Arc::strong_count(&token), 1);
        }
    }
}

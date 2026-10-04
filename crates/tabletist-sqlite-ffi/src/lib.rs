//! The SQLite calls Tabletist cannot make without `unsafe` code.
//!
//! The rest of the workspace forbids `unsafe`, so the few calls that need it
//! live here behind an API that does not.

use std::ffi::c_int;

use rusqlite::ffi;

mod authorizer;
mod columns;
#[cfg(test)]
mod testing;

pub use authorizer::{Action, Authorization, remove_authorizer, set_authorizer};
pub use columns::{ResultColumn, result_columns};

/// SQLite's result code `code` as rusqlite's error, with `message` when
/// SQLite gave one.
fn failure(code: c_int, message: Option<String>) -> rusqlite::Error {
    rusqlite::Error::SqliteFailure(ffi::Error::new(code), message)
}

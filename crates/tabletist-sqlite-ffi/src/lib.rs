//! The SQLite calls Tabletist cannot make without `unsafe` code.
//!
//! The rest of the workspace forbids `unsafe`, so the few calls that need it
//! live here behind an API that does not.

mod columns;

pub use columns::{ResultColumn, result_columns};

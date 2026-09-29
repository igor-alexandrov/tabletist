# Batch 1: DB Core + SQLite Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** The `tabletist-db` crate: the driver-independent types (values, connection specs, catalog, row queries, errors), a per-dialect SQL builder, and a complete read-only SQLite adapter behind the `Connection` enum, all tested against a shared fixture.

**Architecture:** `crates/tabletist-db` has no UI dependencies. `Connection` is a closed enum (only `Sqlite` in this batch; batches 4 and 5 add `Postgres` and `MySql`). Adapters are async; SQLite runs its blocking calls through `tokio::task::spawn_blocking` on a `rusqlite::Connection` opened read-only. The dialect builder quotes every identifier and binds every filter value as a parameter.

**Tech Stack:** Rust 1.98, tokio 1, rusqlite (bundled SQLite), url 2, percent-encoding 2, serde, thiserror 2, tempfile (tests).

**Spec:** `docs/superpowers/specs/2026-09-27-tabletist-design.md` (sections 4.1 to 4.8)

## Global Constraints

- Edition 2024, `rust-version = "1.98"`, `unsafe_code = "forbid"` (workspace lints).
- `tabletist-db` must not depend on egui, eframe, or any fastframe crate.
- Every session is read-only. SQLite opens with `SQLITE_OPEN_READ_ONLY` and sets `PRAGMA query_only = ON`. No function in this crate writes to a connected database. (The one exception is `fixtures::write_sqlite_demo`, which creates a brand-new demo file and is never given a user's database.)
- Identifiers are always quoted (`"x"` for PostgreSQL/SQLite with `"` doubled, `` `x` `` for MySQL with `` ` `` doubled). Filter values are always bound parameters, never spliced into SQL. `LIMIT`/`OFFSET` are integers formatted by the builder.
- Secrets (`Secrets`) never appear in `Debug` output, errors, or logs.
- Row pages fetch `limit + 1` rows to know whether a next page exists (`RowPage::has_more`), then return at most `limit` rows. Adapters stop reading after `limit + 1` rows whatever the SQL says, so a raw WHERE that comments out the LIMIT cannot pull a whole table into memory.
- Run `cargo fmt --all` before every commit; code in this plan is not pre-wrapped to rustfmt's width.
- With no user sort, rows are ordered by the primary key when there is one (`RowPage::ordered_by_key`). With a user sort, primary-key columns are appended as tiebreakers.
- Every dependency gets a comment in `Cargo.toml` saying why.
- Never use em dashes. One topic per commit on `main`.
- If a listed crate version fails to resolve, use `cargo add <crate>@<major>` for the newest compatible release; keep the APIs used.

## Review Focus

1. **Identifiers containing quotes, spaces, or keywords** (a table named `weird "name"`, a column named `select`): browsing must work and never produce broken SQL. Test: Task 3 `identifiers_with_quotes_are_doubled` and Task 5 `a_table_with_a_quoted_name_can_be_browsed`.
2. **Filter text containing `%`, `_`, or `\`**: "contains 50%" must match the literal `50%`, not everything. Test: Task 3 `like_patterns_escape_wildcards` and Task 5 `contains_matches_literal_percent_signs`.
3. **A raw WHERE clause that tries to write or chain statements** (`1=1) ; DELETE FROM users; SELECT (1`), **or comments out the LIMIT** (`1=1) /*`): must fail or stay within one page, and change nothing. Test: Task 5 `a_raw_where_cannot_modify_data` and `a_raw_where_cannot_drop_the_page_limit`.
4. **Opening a path that is not a SQLite database, or does not exist**: a clear `Connect` error, and no file is created at a missing path. Test: Task 4 `a_missing_file_is_a_connect_error_and_is_not_created` and `a_non_database_file_is_a_connect_error`.
5. **Non-UTF-8 text and binary cells** (BLOBs, text with invalid UTF-8): must load as `Bytes`/lossy `Text`, never fail the whole page. Test: Task 5 `blobs_and_invalid_utf8_load_without_failing`.

---

## File Structure

```
Cargo.toml                                  add workspace member
crates/tabletist-db/Cargo.toml
crates/tabletist-db/src/lib.rs              Connection, CancelHandle, re-exports
crates/tabletist-db/src/error.rs            Error, SshStage, Result
crates/tabletist-db/src/value.rs            Value, ValueKind, ColumnMeta
crates/tabletist-db/src/spec.rs             Driver, ConnectSpec, TlsMode, SshSpec, SshAuth, Secrets, URL parsing
crates/tabletist-db/src/catalog.rs          ObjectRef, ObjectKind, ObjectInfo, Structure and parts
crates/tabletist-db/src/query.rs            RowQuery, Filter, FilterOp, Sort, SortDir, RowPage
crates/tabletist-db/src/dialect.rs          Dialect, Sql, SQL builder, escape_like
crates/tabletist-db/src/sqlite.rs           SQLite adapter
crates/tabletist-db/src/fixtures.rs         the shared SQLite fixture (tests + demo mode)
crates/tabletist-db/fixtures/sqlite.sql     fixture schema and data
crates/tabletist-db/tests/sqlite.rs         integration tests through Connection
```

---

### Task 1: Crate scaffold, errors, and values

**Files:**
- Create: `crates/tabletist-db/Cargo.toml`, `crates/tabletist-db/src/lib.rs`, `crates/tabletist-db/src/error.rs`, `crates/tabletist-db/src/value.rs`
- Modify: `Cargo.toml` (root: `members`, and add the dependency to the app)

**Interfaces:**
- Consumes: nothing.
- Produces:
  - `tabletist_db::Error` (`Debug, Clone, PartialEq, Eq, thiserror::Error`) with variants `Connect(String)`, `Auth(String)`, `Tls(String)`, `Ssh { stage: SshStage, message: String }`, `Query { code: Option<String>, message: String, detail: Option<String>, hint: Option<String> }`, `Cancelled`, `Timeout`, `ConnectionLost(String)`, `InvalidSpec(String)`, `Unsupported(&'static str)`, `Io(String)`; methods `Error::query(message: impl Into<String>) -> Error` and `Error::is_connection_lost(&self) -> bool`.
  - `tabletist_db::SshStage` (`Debug, Clone, PartialEq, Eq, Display`): `Connect`, `HostKeyUnknown { fingerprint: String }`, `HostKeyMismatch { fingerprint: String }`, `Auth`, `Forward`.
  - `tabletist_db::Result<T>`.
  - `tabletist_db::Value` (`Debug, Clone, PartialEq`): `Null`, `Bool(bool)`, `Int(i64)`, `Float(f64)`, `Text(Box<str>)`, `Bytes(Box<[u8]>)`; `Value::is_null(&self) -> bool`.
  - `tabletist_db::ValueKind` (`Debug, Clone, Copy, PartialEq, Eq`): `Numeric, Text, Json, Temporal, Binary, Bool, Other`; `ValueKind::from_sqlite_decl(decl: &str) -> ValueKind`, `ValueKind::of_value(value: &Value) -> ValueKind`.
  - `tabletist_db::ColumnMeta { pub name: String, pub type_name: String, pub kind: ValueKind }` (`Debug, Clone, PartialEq`).

- [ ] **Step 1: Create the crate with failing tests**

Create `crates/tabletist-db/Cargo.toml`:

```toml
[package]
name = "tabletist-db"
version = "0.1.0"
description = "Access to PostgreSQL, MySQL and SQLite for Tabletist"
edition.workspace = true
rust-version.workspace = true
publish = false

[lints]
workspace = true

[dependencies]
# Typed errors the app shows to the user as-is.
thiserror = "2"
# Connection specs are saved in connections.json.
serde = { version = "1", features = ["derive"] }
# Adapters are async; SQLite's blocking calls run on spawn_blocking.
tokio = { version = "1", features = ["rt", "sync"] }
# SQLite, compiled in so every platform gets the same, recent version.
# column_decltype gives each result column's declared type.
rusqlite = { version = "0.37", features = ["bundled", "column_decltype"] }
# Parsing postgres:// and mysql:// connection URLs.
url = "2"
# Decoding user names, passwords and database names in URLs.
percent-encoding = "2"
log = "0.4"

[dev-dependencies]
tokio = { version = "1", features = ["rt-multi-thread", "macros", "time"] }
tempfile = "3"
serde_json = "1"
```

In the root `Cargo.toml`, change `members = ["."]` to:

```toml
members = [".", "crates/tabletist-db"]
```

and add to the app's `[dependencies]`:

```toml
# The database layer (no UI dependencies).
tabletist-db = { path = "crates/tabletist-db" }
```

Create `crates/tabletist-db/src/lib.rs`:

```rust
//! Access to PostgreSQL, MySQL and SQLite for Tabletist.
//!
//! Nothing in this crate writes to a connected database: every session is
//! opened read-only.

mod error;
mod value;

pub use error::{Error, Result, SshStage};
pub use value::{ColumnMeta, Value, ValueKind};
```

Create `crates/tabletist-db/src/error.rs` with only tests:

```rust
//! Errors, worded for the user.

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn query_errors_show_their_message() {
        let error = Error::query("no such table: nope");
        assert_eq!(error.to_string(), "no such table: nope");
        assert!(matches!(error, Error::Query { code: None, .. }));
    }

    #[test]
    fn only_lost_connections_count_as_lost() {
        assert!(Error::ConnectionLost("reset".into()).is_connection_lost());
        assert!(!Error::Cancelled.is_connection_lost());
        assert!(!Error::query("x").is_connection_lost());
    }

    #[test]
    fn ssh_errors_name_their_stage() {
        let error = Error::Ssh {
            stage: SshStage::HostKeyUnknown {
                fingerprint: "SHA256:abc".into(),
            },
            message: "unknown host key".into(),
        };
        assert_eq!(
            error.to_string(),
            "SSH (unknown host key SHA256:abc): unknown host key"
        );
    }
}
```

Create `crates/tabletist-db/src/value.rs` with only tests:

```rust
//! Cell values and column metadata, the same for every driver.

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sqlite_declared_types_map_to_kinds() {
        let cases = [
            ("INTEGER", ValueKind::Numeric),
            ("bigint", ValueKind::Numeric),
            ("NUMERIC(10,2)", ValueKind::Numeric),
            ("DECIMAL", ValueKind::Numeric),
            ("REAL", ValueKind::Numeric),
            ("double precision", ValueKind::Numeric),
            ("TEXT", ValueKind::Text),
            ("VARCHAR(255)", ValueKind::Text),
            ("CLOB", ValueKind::Text),
            ("JSON", ValueKind::Json),
            ("jsonb", ValueKind::Json),
            ("BLOB", ValueKind::Binary),
            ("BOOLEAN", ValueKind::Bool),
            ("DATETIME", ValueKind::Temporal),
            ("date", ValueKind::Temporal),
            ("TIMESTAMP", ValueKind::Temporal),
            ("", ValueKind::Other),
            ("GEOMETRY", ValueKind::Other),
        ];
        for (decl, kind) in cases {
            assert_eq!(ValueKind::from_sqlite_decl(decl), kind, "{decl}");
        }
    }

    #[test]
    fn values_have_kinds_from_their_storage() {
        assert_eq!(ValueKind::of_value(&Value::Int(1)), ValueKind::Numeric);
        assert_eq!(ValueKind::of_value(&Value::Float(1.5)), ValueKind::Numeric);
        assert_eq!(ValueKind::of_value(&Value::Text("a".into())), ValueKind::Text);
        assert_eq!(ValueKind::of_value(&Value::Bytes(vec![1].into())), ValueKind::Binary);
        assert_eq!(ValueKind::of_value(&Value::Bool(true)), ValueKind::Bool);
        assert_eq!(ValueKind::of_value(&Value::Null), ValueKind::Other);
        assert!(Value::Null.is_null());
        assert!(!Value::Int(0).is_null());
    }
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p tabletist-db`
Expected: FAIL to compile: `cannot find type Error`, `cannot find type ValueKind`.

- [ ] **Step 3: Implement errors and values**

Put above the tests in `error.rs`:

```rust
use std::fmt;

pub type Result<T> = std::result::Result<T, Error>;

/// Everything that can go wrong talking to a database. Messages are shown to
/// the user as they are, so they must never contain a password.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    #[error("could not connect: {0}")]
    Connect(String),
    #[error("authentication failed: {0}")]
    Auth(String),
    #[error("TLS: {0}")]
    Tls(String),
    #[error("SSH ({stage}): {message}")]
    Ssh { stage: SshStage, message: String },
    #[error("{message}")]
    Query {
        /// SQLSTATE (PostgreSQL, MySQL) or extended result code (SQLite).
        code: Option<String>,
        message: String,
        detail: Option<String>,
        hint: Option<String>,
    },
    #[error("the query was cancelled")]
    Cancelled,
    #[error("the operation timed out")]
    Timeout,
    #[error("the connection was lost: {0}")]
    ConnectionLost(String),
    #[error("invalid connection settings: {0}")]
    InvalidSpec(String),
    #[error("not supported: {0}")]
    Unsupported(&'static str),
    #[error("{0}")]
    Io(String),
}

impl Error {
    /// A query error with only a message.
    pub fn query(message: impl Into<String>) -> Self {
        Self::Query {
            code: None,
            message: message.into(),
            detail: None,
            hint: None,
        }
    }

    /// Whether the session is unusable and must be reconnected.
    pub fn is_connection_lost(&self) -> bool {
        matches!(self, Self::ConnectionLost(_))
    }
}

/// Where an SSH tunnel failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SshStage {
    Connect,
    /// First connection to this host: the user must trust the key.
    HostKeyUnknown { fingerprint: String },
    /// The host's key changed since it was trusted. Refused.
    HostKeyMismatch { fingerprint: String },
    Auth,
    Forward,
}

impl fmt::Display for SshStage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Connect => f.write_str("connect"),
            Self::HostKeyUnknown { fingerprint } => write!(f, "unknown host key {fingerprint}"),
            Self::HostKeyMismatch { fingerprint } => write!(f, "changed host key {fingerprint}"),
            Self::Auth => f.write_str("authentication"),
            Self::Forward => f.write_str("port forwarding"),
        }
    }
}
```

Put above the tests in `value.rs`:

```rust
/// One cell. Integers wider than `i64` and exact decimals arrive as `Text`
/// with a `Numeric` column kind, so nothing is rounded.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    Text(Box<str>),
    Bytes(Box<[u8]>),
}

impl Value {
    pub fn is_null(&self) -> bool {
        matches!(self, Self::Null)
    }
}

/// How the interface treats a column: alignment, pretty-printing, hex.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValueKind {
    Numeric,
    Text,
    Json,
    Temporal,
    Binary,
    Bool,
    Other,
}

impl ValueKind {
    /// The kind for a SQLite declared column type. Follows SQLite's affinity
    /// rules, with JSON, boolean and date/time names recognised first.
    pub fn from_sqlite_decl(decl: &str) -> Self {
        let decl = decl.to_ascii_uppercase();
        let has = |needle: &str| decl.contains(needle);
        if decl.is_empty() {
            Self::Other
        } else if has("JSON") {
            Self::Json
        } else if has("BOOL") {
            Self::Bool
        } else if has("DATE") || has("TIME") {
            Self::Temporal
        } else if has("INT") {
            Self::Numeric
        } else if has("CHAR") || has("CLOB") || has("TEXT") {
            Self::Text
        } else if has("BLOB") {
            Self::Binary
        } else if has("REAL") || has("FLOA") || has("DOUB") || has("NUMERIC") || has("DEC") {
            Self::Numeric
        } else {
            Self::Other
        }
    }

    /// The kind a value's storage suggests, for columns with no declared type.
    pub fn of_value(value: &Value) -> Self {
        match value {
            Value::Null => Self::Other,
            Value::Bool(_) => Self::Bool,
            Value::Int(_) | Value::Float(_) => Self::Numeric,
            Value::Text(_) => Self::Text,
            Value::Bytes(_) => Self::Binary,
        }
    }
}

/// A result column.
#[derive(Debug, Clone, PartialEq)]
pub struct ColumnMeta {
    pub name: String,
    /// The database's own type name (`int4`, `varchar(255)`, `TEXT`), or
    /// empty when the database does not say.
    pub type_name: String,
    pub kind: ValueKind,
}
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p tabletist-db`
Expected: PASS (5 tests).

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml Cargo.lock crates/tabletist-db
git commit -m "Add the tabletist-db crate with errors and values"
```

---

### Task 2: Connection specs, secrets, and URL parsing

**Files:**
- Create: `crates/tabletist-db/src/spec.rs`
- Modify: `crates/tabletist-db/src/lib.rs`

**Interfaces:**
- Consumes: `Error`, `Result`.
- Produces:
  - `Driver` (`Debug, Clone, Copy, PartialEq, Eq, Hash, Default=Postgres, Serialize, Deserialize` as `"postgres" | "mysql" | "sqlite"`): `Driver::default_port(self) -> u16` (5432, 3306, 0), `Driver::label(self) -> &'static str` ("PostgreSQL", "MySQL", "SQLite").
  - `TlsMode` (`Default=Prefer`, serde kebab-case): `Disable, Prefer, Require, VerifyCa, VerifyFull`.
  - `SshAuth` (serde tagged `{"method": "password"}` / `{"method": "key-file", "path": ..}` / `{"method": "agent"}`): `Password, KeyFile { path: PathBuf }, Agent`.
  - `SshSpec { host: String, port: u16, user: String, auth: SshAuth }`.
  - `ConnectSpec { driver: Driver, host: String, port: u16, user: String, database: String, sqlite_path: Option<PathBuf>, tls: TlsMode, ca_file: Option<PathBuf>, ssh: Option<SshSpec> }` (`Debug, Clone, PartialEq, Serialize, Deserialize, Default`, `#[serde(default)]`); `ConnectSpec::sqlite(path: impl Into<PathBuf>) -> ConnectSpec`, `ConnectSpec::from_url(url: &str) -> Result<(ConnectSpec, Secrets)>`, `ConnectSpec::summary(&self) -> String`.
  - `Secrets { password: Option<String>, ssh_password: Option<String>, ssh_passphrase: Option<String> }` (`Clone, Default`, redacted `Debug`).

- [ ] **Step 1: Write the failing tests**

Create `crates/tabletist-db/src/spec.rs` with only tests:

```rust
//! What to connect to. Secrets are kept apart so a spec can be saved to disk.

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn postgres_urls_parse_with_decoded_parts() {
        let (spec, secrets) = ConnectSpec::from_url(
            "postgres://app%40corp:p%40ss%2Fw0rd@db.example.com:6543/my%20db?sslmode=verify-full&sslrootcert=/etc/ca.pem",
        )
        .unwrap();
        assert_eq!(spec.driver, Driver::Postgres);
        assert_eq!(spec.host, "db.example.com");
        assert_eq!(spec.port, 6543);
        assert_eq!(spec.user, "app@corp");
        assert_eq!(spec.database, "my db");
        assert_eq!(spec.tls, TlsMode::VerifyFull);
        assert_eq!(spec.ca_file, Some(PathBuf::from("/etc/ca.pem")));
        assert_eq!(secrets.password.as_deref(), Some("p@ss/w0rd"));
    }

    #[test]
    fn postgresql_scheme_and_default_port_work() {
        let (spec, secrets) = ConnectSpec::from_url("postgresql://localhost/app").unwrap();
        assert_eq!(spec.driver, Driver::Postgres);
        assert_eq!(spec.port, 5432);
        assert_eq!(spec.user, "");
        assert_eq!(spec.tls, TlsMode::Prefer);
        assert_eq!(secrets.password, None);
    }

    #[test]
    fn mysql_urls_parse_their_ssl_mode() {
        let (spec, _) = ConnectSpec::from_url("mysql://root@127.0.0.1/shop?ssl-mode=REQUIRED").unwrap();
        assert_eq!(spec.driver, Driver::MySql);
        assert_eq!(spec.port, 3306);
        assert_eq!(spec.tls, TlsMode::Require);
        let (spec, _) = ConnectSpec::from_url("mariadb://h/db?ssl-mode=VERIFY_IDENTITY").unwrap();
        assert_eq!(spec.tls, TlsMode::VerifyFull);
    }

    #[test]
    fn ipv6_hosts_lose_their_brackets() {
        let (spec, _) = ConnectSpec::from_url("postgres://[::1]:5433/db").unwrap();
        assert_eq!(spec.host, "::1");
    }

    #[test]
    fn sqlite_urls_name_a_file() {
        let (spec, _) = ConnectSpec::from_url("sqlite:///var/data/app.db").unwrap();
        assert_eq!(spec.driver, Driver::Sqlite);
        assert_eq!(spec.sqlite_path, Some(PathBuf::from("/var/data/app.db")));
        let (spec, _) = ConnectSpec::from_url("sqlite:relative.db").unwrap();
        assert_eq!(spec.sqlite_path, Some(PathBuf::from("relative.db")));
    }

    #[test]
    fn bad_urls_are_invalid_specs() {
        for url in ["", "sqlite:", "oracle://h/db", "not a url", "postgres://h/db?sslmode=sometimes"] {
            assert!(
                matches!(ConnectSpec::from_url(url), Err(Error::InvalidSpec(_))),
                "{url:?} should be rejected"
            );
        }
    }

    #[test]
    fn summaries_never_include_secrets() {
        let (spec, _) = ConnectSpec::from_url("postgres://me:secret@h:5432/db").unwrap();
        assert_eq!(spec.summary(), "me@h:5432/db");
        assert!(!spec.summary().contains("secret"));
        assert_eq!(ConnectSpec::sqlite("/tmp/data/app.db").summary(), "app.db");
        let (spec, _) = ConnectSpec::from_url("mysql://h").unwrap();
        assert_eq!(spec.summary(), "h:3306");
    }

    #[test]
    fn secrets_do_not_print() {
        let secrets = Secrets {
            password: Some("hunter2".into()),
            ssh_password: Some("s3cret".into()),
            ssh_passphrase: None,
        };
        let printed = format!("{secrets:?}");
        assert!(!printed.contains("hunter2"));
        assert!(!printed.contains("s3cret"));
    }

    #[test]
    fn specs_round_trip_through_json_and_old_files_load() {
        let spec = ConnectSpec {
            ssh: Some(SshSpec {
                host: "bastion".into(),
                port: 22,
                user: "ops".into(),
                auth: SshAuth::KeyFile {
                    path: "/home/ops/.ssh/id_ed25519".into(),
                },
            }),
            ..ConnectSpec::from_url("postgres://u@h/db").unwrap().0
        };
        let json = serde_json::to_string(&spec).unwrap();
        assert_eq!(serde_json::from_str::<ConnectSpec>(&json).unwrap(), spec);
        let old: ConnectSpec = serde_json::from_str(r#"{"driver": "sqlite", "sqlite_path": "/a.db"}"#).unwrap();
        assert_eq!(old.driver, Driver::Sqlite);
        assert_eq!(old.tls, TlsMode::Prefer);
    }
}
```

Add to `lib.rs`:

```rust
mod spec;
pub use spec::{ConnectSpec, Driver, Secrets, SshAuth, SshSpec, TlsMode};
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p tabletist-db spec`
Expected: FAIL to compile: `cannot find type ConnectSpec`.

- [ ] **Step 3: Implement specs**

Put above the tests in `spec.rs`:

```rust
use std::fmt;
use std::path::PathBuf;

use percent_encoding::percent_decode_str;
use serde::{Deserialize, Serialize};

use crate::{Error, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Driver {
    #[default]
    Postgres,
    #[serde(rename = "mysql")]
    MySql,
    Sqlite,
}

impl Driver {
    pub fn default_port(self) -> u16 {
        match self {
            Self::Postgres => 5432,
            Self::MySql => 3306,
            Self::Sqlite => 0,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Postgres => "PostgreSQL",
            Self::MySql => "MySQL",
            Self::Sqlite => "SQLite",
        }
    }
}

/// libpq's `sslmode` values; MySQL's `ssl-mode` maps onto them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TlsMode {
    Disable,
    #[default]
    Prefer,
    Require,
    VerifyCa,
    VerifyFull,
}

impl TlsMode {
    fn parse(value: &str) -> Option<Self> {
        match value.to_ascii_lowercase().replace('_', "-").as_str() {
            "disable" | "disabled" => Some(Self::Disable),
            "prefer" | "preferred" | "allow" => Some(Self::Prefer),
            "require" | "required" => Some(Self::Require),
            "verify-ca" => Some(Self::VerifyCa),
            "verify-full" | "verify-identity" => Some(Self::VerifyFull),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "method", rename_all = "kebab-case")]
pub enum SshAuth {
    Password,
    KeyFile { path: PathBuf },
    Agent,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SshSpec {
    pub host: String,
    pub port: u16,
    pub user: String,
    pub auth: SshAuth,
}

/// Everything needed to connect except secrets. Saved in connections.json.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ConnectSpec {
    pub driver: Driver,
    pub host: String,
    pub port: u16,
    pub user: String,
    pub database: String,
    pub sqlite_path: Option<PathBuf>,
    pub tls: TlsMode,
    pub ca_file: Option<PathBuf>,
    pub ssh: Option<SshSpec>,
}

impl Default for ConnectSpec {
    fn default() -> Self {
        Self {
            driver: Driver::Postgres,
            host: "localhost".into(),
            port: Driver::Postgres.default_port(),
            user: String::new(),
            database: String::new(),
            sqlite_path: None,
            tls: TlsMode::Prefer,
            ca_file: None,
            ssh: None,
        }
    }
}

/// Passwords and passphrases. Never saved with the spec, never printed.
#[derive(Clone, Default, PartialEq, Eq)]
pub struct Secrets {
    pub password: Option<String>,
    pub ssh_password: Option<String>,
    pub ssh_passphrase: Option<String>,
}

impl fmt::Debug for Secrets {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Secrets { .. }")
    }
}

fn decode(text: &str) -> String {
    percent_decode_str(text).decode_utf8_lossy().into_owned()
}

impl ConnectSpec {
    pub fn sqlite(path: impl Into<PathBuf>) -> Self {
        Self {
            driver: Driver::Sqlite,
            host: String::new(),
            port: 0,
            sqlite_path: Some(path.into()),
            ..Self::default()
        }
    }

    /// Parses `postgres://`, `postgresql://`, `mysql://`, `mariadb://` and
    /// `sqlite:` URLs. The password, if any, comes back in `Secrets`.
    pub fn from_url(url: &str) -> Result<(Self, Secrets)> {
        let url = url.trim();
        if let Some(rest) = url.strip_prefix("sqlite:") {
            let path = rest.strip_prefix("//").unwrap_or(rest);
            if path.is_empty() {
                return Err(Error::InvalidSpec("the SQLite URL names no file".into()));
            }
            return Ok((Self::sqlite(decode(path)), Secrets::default()));
        }
        let parsed = url::Url::parse(url)
            .map_err(|error| Error::InvalidSpec(format!("not a connection URL: {error}")))?;
        let driver = match parsed.scheme() {
            "postgres" | "postgresql" => Driver::Postgres,
            "mysql" | "mariadb" => Driver::MySql,
            other => {
                return Err(Error::InvalidSpec(format!(
                    "unsupported scheme {other:?}; use postgres://, mysql:// or sqlite:"
                )));
            }
        };
        let host = parsed
            .host_str()
            .unwrap_or("localhost")
            .trim_start_matches('[')
            .trim_end_matches(']')
            .to_owned();
        let mut spec = Self {
            driver,
            host,
            port: parsed.port().unwrap_or(driver.default_port()),
            user: decode(parsed.username()),
            database: decode(parsed.path().trim_start_matches('/')),
            ..Self::default()
        };
        for (key, value) in parsed.query_pairs() {
            match key.as_ref() {
                "sslmode" | "ssl-mode" | "ssl_mode" => {
                    spec.tls = TlsMode::parse(&value)
                        .ok_or_else(|| Error::InvalidSpec(format!("unknown TLS mode {value:?}")))?;
                }
                "sslrootcert" | "ssl-ca" => spec.ca_file = Some(PathBuf::from(value.as_ref())),
                _ => log::debug!("ignoring URL parameter {key}"),
            }
        }
        let secrets = Secrets {
            password: parsed.password().map(decode),
            ..Secrets::default()
        };
        Ok((spec, secrets))
    }

    /// A one-line description without secrets: `user@host:port/db`, or the
    /// file name for SQLite.
    pub fn summary(&self) -> String {
        if self.driver == Driver::Sqlite {
            return self
                .sqlite_path
                .as_ref()
                .and_then(|path| path.file_name())
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_default();
        }
        let mut text = String::new();
        if !self.user.is_empty() {
            text.push_str(&self.user);
            text.push('@');
        }
        text.push_str(&format!("{}:{}", self.host, self.port));
        if !self.database.is_empty() {
            text.push('/');
            text.push_str(&self.database);
        }
        text
    }
}
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p tabletist-db spec`
Expected: PASS (9 tests).

- [ ] **Step 5: Commit**

```bash
git add crates/tabletist-db
git commit -m "Add connection specs, secrets and URL parsing"
```

---

### Task 3: Catalog and query types, and the dialect SQL builder

**Files:**
- Create: `crates/tabletist-db/src/catalog.rs`, `crates/tabletist-db/src/query.rs`, `crates/tabletist-db/src/dialect.rs`
- Modify: `crates/tabletist-db/src/lib.rs`

**Interfaces:**
- Consumes: `Value`, `ColumnMeta`.
- Produces:
  - `ObjectRef { schema: String, name: String }` (`Debug, Clone, PartialEq, Eq, Hash`), `ObjectRef::new(schema: impl Into<String>, name: impl Into<String>) -> ObjectRef`.
  - `ObjectKind` (`Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord`): `Table, View, MaterializedView`.
  - `ObjectInfo { name: String, kind: ObjectKind, estimated_rows: Option<u64> }` (`Debug, Clone, PartialEq`).
  - `ColumnInfo { name, type_name: String, nullable: bool, default: Option<String>, comment: Option<String> }`, `IndexInfo { name: String, columns: Vec<String>, unique: bool, primary: bool, method: Option<String> }`, `ForeignKeyInfo { name: Option<String>, columns: Vec<String>, ref_schema: String, ref_table: String, ref_columns: Vec<String>, on_update: String, on_delete: String }`, `Structure { columns: Vec<ColumnInfo>, primary_key: Vec<String>, indexes: Vec<IndexInfo>, foreign_keys: Vec<ForeignKeyInfo> }` (all `Debug, Clone, PartialEq, Default` where sensible).
  - `FilterOp` (`Debug, Clone, Copy, PartialEq, Eq`): `Eq, Ne, Lt, Gt, Le, Ge, Contains, StartsWith, IsNull, IsNotNull, In`; `FilterOp::ALL: [FilterOp; 11]`, `FilterOp::takes_value(self) -> bool`, `FilterOp::label(self) -> &'static str`.
  - `Filter { column: String, op: FilterOp, value: String }`.
  - `SortDir { Asc, Desc }`, `Sort { column: String, dir: SortDir }`.
  - `RowQuery { object: ObjectRef, filters: Vec<Filter>, raw_where: Option<String>, sort: Vec<Sort>, offset: u64, limit: u32 }`, `RowQuery::new(object: ObjectRef, limit: u32) -> RowQuery`.
  - `RowPage { columns: Vec<ColumnMeta>, rows: Vec<Vec<Value>>, has_more: bool, ordered_by_key: bool, elapsed: std::time::Duration }`.
  - `Dialect` (`Debug, Clone, Copy, PartialEq, Eq`): `Postgres, MySql, Sqlite`; `quote_ident(self, &str) -> String`, `qualified(self, &ObjectRef) -> String`, `select_rows(self, q: &RowQuery, key: &[String]) -> Sql`, `count_rows(self, q: &RowQuery) -> Sql`.
  - `Sql { text: String, params: Vec<Value> }` (`Debug, Clone, PartialEq`).
  - `escape_like(text: &str) -> String`.

- [ ] **Step 1: Write the types and the failing builder tests**

Create `crates/tabletist-db/src/catalog.rs` (types only, no behaviour to test):

```rust
//! Database objects and their structure.

/// A table, view or materialized view, by schema and name.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ObjectRef {
    /// Schema (PostgreSQL), database (MySQL) or attached database (SQLite).
    pub schema: String,
    pub name: String,
}

impl ObjectRef {
    pub fn new(schema: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            schema: schema.into(),
            name: name.into(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ObjectKind {
    Table,
    View,
    /// PostgreSQL only.
    MaterializedView,
}

/// An entry in the sidebar.
#[derive(Debug, Clone, PartialEq)]
pub struct ObjectInfo {
    pub name: String,
    pub kind: ObjectKind,
    /// From the planner's statistics; `None` when the database has none.
    pub estimated_rows: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct ColumnInfo {
    pub name: String,
    pub type_name: String,
    pub nullable: bool,
    pub default: Option<String>,
    pub comment: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct IndexInfo {
    pub name: String,
    /// Column names in index order; expressions appear as `<expression>`.
    pub columns: Vec<String>,
    pub unique: bool,
    pub primary: bool,
    /// `btree`, `hash`, `gin`...; `None` when the database does not say.
    pub method: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct ForeignKeyInfo {
    /// SQLite foreign keys have no names.
    pub name: Option<String>,
    pub columns: Vec<String>,
    pub ref_schema: String,
    pub ref_table: String,
    /// Empty when the key references the other table's primary key implicitly.
    pub ref_columns: Vec<String>,
    pub on_update: String,
    pub on_delete: String,
}

/// What the Structure view shows.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Structure {
    pub columns: Vec<ColumnInfo>,
    pub primary_key: Vec<String>,
    pub indexes: Vec<IndexInfo>,
    pub foreign_keys: Vec<ForeignKeyInfo>,
}
```

Create `crates/tabletist-db/src/query.rs`:

```rust
//! Browsing a table's rows: filters, sorting, paging.

use std::time::Duration;

use crate::{ColumnMeta, ObjectRef, Value};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterOp {
    Eq,
    Ne,
    Lt,
    Gt,
    Le,
    Ge,
    /// Case-insensitive substring match on the value as text.
    Contains,
    StartsWith,
    IsNull,
    IsNotNull,
    /// Comma-separated values.
    In,
}

impl FilterOp {
    pub const ALL: [FilterOp; 11] = [
        Self::Eq,
        Self::Ne,
        Self::Lt,
        Self::Gt,
        Self::Le,
        Self::Ge,
        Self::Contains,
        Self::StartsWith,
        Self::IsNull,
        Self::IsNotNull,
        Self::In,
    ];

    pub fn takes_value(self) -> bool {
        !matches!(self, Self::IsNull | Self::IsNotNull)
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Eq => "=",
            Self::Ne => "≠",
            Self::Lt => "<",
            Self::Gt => ">",
            Self::Le => "≤",
            Self::Ge => "≥",
            Self::Contains => "contains",
            Self::StartsWith => "starts with",
            Self::IsNull => "is NULL",
            Self::IsNotNull => "is not NULL",
            Self::In => "in",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Filter {
    pub column: String,
    pub op: FilterOp,
    /// As typed. The database converts it to the column's type.
    pub value: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortDir {
    Asc,
    Desc,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sort {
    pub column: String,
    pub dir: SortDir,
}

/// One page of one object's rows.
#[derive(Debug, Clone, PartialEq)]
pub struct RowQuery {
    pub object: ObjectRef,
    /// Combined with AND.
    pub filters: Vec<Filter>,
    /// Appended as `AND (<raw>)`. Safe because sessions are read-only.
    pub raw_where: Option<String>,
    pub sort: Vec<Sort>,
    pub offset: u64,
    pub limit: u32,
}

impl RowQuery {
    pub fn new(object: ObjectRef, limit: u32) -> Self {
        Self {
            object,
            filters: Vec::new(),
            raw_where: None,
            sort: Vec::new(),
            offset: 0,
            limit,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RowPage {
    pub columns: Vec<ColumnMeta>,
    /// At most `limit` rows.
    pub rows: Vec<Vec<Value>>,
    /// Whether a next page exists.
    pub has_more: bool,
    /// Whether rows are ordered by a key, so paging is stable.
    pub ordered_by_key: bool,
    pub elapsed: Duration,
}
```

Create `crates/tabletist-db/src/dialect.rs` with only tests:

```rust
//! SQL for each database: identifier quoting, placeholders, filters, paging.

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Filter, FilterOp, ObjectRef, RowQuery, Sort, SortDir, Value};

    fn query() -> RowQuery {
        RowQuery::new(ObjectRef::new("public", "users"), 300)
    }

    fn text(value: &str) -> Value {
        Value::Text(value.into())
    }

    #[test]
    fn identifiers_with_quotes_are_doubled() {
        assert_eq!(Dialect::Postgres.quote_ident(r#"weird "name""#), r#""weird ""name""""#);
        assert_eq!(Dialect::Sqlite.quote_ident("select"), r#""select""#);
        assert_eq!(Dialect::MySql.quote_ident("a`b"), "`a``b`");
        assert_eq!(
            Dialect::MySql.qualified(&ObjectRef::new("shop", "orders")),
            "`shop`.`orders`"
        );
    }

    #[test]
    fn a_plain_page_orders_by_the_key_and_asks_for_one_extra_row() {
        let sql = Dialect::Postgres.select_rows(&query(), &["id".into()]);
        assert_eq!(
            sql.text,
            r#"SELECT * FROM "public"."users" ORDER BY "id" ASC LIMIT 301 OFFSET 0"#
        );
        assert!(sql.params.is_empty());
    }

    #[test]
    fn without_a_key_there_is_no_order() {
        let mut q = query();
        q.offset = 600;
        let sql = Dialect::Sqlite.select_rows(&q, &[]);
        assert_eq!(sql.text, r#"SELECT * FROM "public"."users" LIMIT 301 OFFSET 600"#);
    }

    #[test]
    fn user_sorts_come_first_with_key_tiebreakers() {
        let mut q = query();
        q.sort = vec![Sort {
            column: "email".into(),
            dir: SortDir::Desc,
        }];
        let sql = Dialect::MySql.select_rows(&q, &["id".into()]);
        assert_eq!(
            sql.text,
            "SELECT * FROM `public`.`users` ORDER BY `email` DESC, `id` ASC LIMIT 301 OFFSET 0"
        );
    }

    #[test]
    fn comparison_filters_bind_parameters_with_dialect_placeholders() {
        let mut q = query();
        q.filters = vec![
            Filter { column: "age".into(), op: FilterOp::Ge, value: "18".into() },
            Filter { column: "name".into(), op: FilterOp::Ne, value: "bob".into() },
        ];
        let pg = Dialect::Postgres.select_rows(&q, &[]);
        assert_eq!(
            pg.text,
            r#"SELECT * FROM "public"."users" WHERE "age" >= $1 AND "name" <> $2 LIMIT 301 OFFSET 0"#
        );
        assert_eq!(pg.params, vec![text("18"), text("bob")]);
        let my = Dialect::MySql.select_rows(&q, &[]);
        assert!(my.text.contains("WHERE `age` >= ? AND `name` <> ?"), "{}", my.text);
    }

    #[test]
    fn null_filters_take_no_parameter() {
        let mut q = query();
        q.filters = vec![
            Filter { column: "deleted_at".into(), op: FilterOp::IsNull, value: "ignored".into() },
            Filter { column: "email".into(), op: FilterOp::IsNotNull, value: String::new() },
        ];
        let sql = Dialect::Sqlite.select_rows(&q, &[]);
        assert!(sql.text.contains(r#"WHERE "deleted_at" IS NULL AND "email" IS NOT NULL"#));
        assert!(sql.params.is_empty());
    }

    #[test]
    fn in_filters_split_on_commas_and_an_empty_list_matches_nothing() {
        let mut q = query();
        q.filters = vec![Filter { column: "id".into(), op: FilterOp::In, value: " 1, 2 ,,3 ".into() }];
        let sql = Dialect::Postgres.select_rows(&q, &[]);
        assert!(sql.text.contains(r#""id" IN ($1, $2, $3)"#), "{}", sql.text);
        assert_eq!(sql.params, vec![text("1"), text("2"), text("3")]);
        q.filters[0].value = " , ".into();
        let sql = Dialect::Postgres.select_rows(&q, &[]);
        assert!(sql.text.contains("WHERE 1 = 0"), "{}", sql.text);
    }

    #[test]
    fn like_patterns_escape_wildcards() {
        assert_eq!(escape_like(r"50%_off\now"), r"50\%\_off\\now");
    }

    #[test]
    fn contains_casts_to_text_and_is_case_insensitive_per_dialect() {
        let mut q = query();
        q.filters = vec![Filter { column: "id".into(), op: FilterOp::Contains, value: "5%".into() }];
        let pg = Dialect::Postgres.select_rows(&q, &[]);
        assert!(pg.text.contains(r#"CAST("id" AS TEXT) ILIKE $1"#), "{}", pg.text);
        assert_eq!(pg.params, vec![text(r"%5\%%")]);
        let lite = Dialect::Sqlite.select_rows(&q, &[]);
        assert!(lite.text.contains(r#"CAST("id" AS TEXT) LIKE ? ESCAPE '\'"#), "{}", lite.text);
        let my = Dialect::MySql.select_rows(&q, &[]);
        assert!(my.text.contains("CAST(`id` AS CHAR) LIKE ?"), "{}", my.text);
        q.filters[0].op = FilterOp::StartsWith;
        assert_eq!(Dialect::Postgres.select_rows(&q, &[]).params, vec![text(r"5\%%")]);
    }

    #[test]
    fn raw_where_is_wrapped_in_parentheses_after_filters() {
        let mut q = query();
        q.filters = vec![Filter { column: "a".into(), op: FilterOp::Eq, value: "1".into() }];
        q.raw_where = Some("b = 2 OR c = 3".into());
        let sql = Dialect::Postgres.select_rows(&q, &[]);
        assert!(sql.text.contains("WHERE \"a\" = $1 AND (\nb = 2 OR c = 3\n)"), "{}", sql.text);
        q.raw_where = Some("   ".into());
        assert!(!Dialect::Postgres.select_rows(&q, &[]).text.contains("AND ("));
    }

    #[test]
    fn counts_use_the_same_filters_without_order_or_paging() {
        let mut q = query();
        q.filters = vec![Filter { column: "a".into(), op: FilterOp::Eq, value: "1".into() }];
        q.sort = vec![Sort { column: "a".into(), dir: SortDir::Asc }];
        q.offset = 300;
        let sql = Dialect::Sqlite.count_rows(&q);
        assert_eq!(sql.text, r#"SELECT count(*) FROM "public"."users" WHERE "a" = ?"#);
        assert_eq!(sql.params, vec![text("1")]);
    }
}
```

Add to `lib.rs`:

```rust
mod catalog;
mod dialect;
mod query;

pub use catalog::{ColumnInfo, ForeignKeyInfo, IndexInfo, ObjectInfo, ObjectKind, ObjectRef, Structure};
pub use dialect::{Dialect, Sql, escape_like};
pub use query::{Filter, FilterOp, RowPage, RowQuery, Sort, SortDir};
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p tabletist-db dialect`
Expected: FAIL to compile: `cannot find type Dialect`.

- [ ] **Step 3: Implement the builder**

Put above the tests in `dialect.rs`:

```rust
use crate::{Filter, FilterOp, ObjectRef, RowQuery, SortDir, Value};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dialect {
    Postgres,
    MySql,
    Sqlite,
}

/// SQL text with its bound parameters.
#[derive(Debug, Clone, PartialEq)]
pub struct Sql {
    pub text: String,
    pub params: Vec<Value>,
}

/// Escapes `\`, `%` and `_` so `text` matches literally inside a LIKE
/// pattern whose escape character is `\`.
pub fn escape_like(text: &str) -> String {
    let mut escaped = String::with_capacity(text.len());
    for character in text.chars() {
        if matches!(character, '\\' | '%' | '_') {
            escaped.push('\\');
        }
        escaped.push(character);
    }
    escaped
}

impl Dialect {
    pub fn quote_ident(self, ident: &str) -> String {
        match self {
            Self::MySql => format!("`{}`", ident.replace('`', "``")),
            Self::Postgres | Self::Sqlite => format!("\"{}\"", ident.replace('"', "\"\"")),
        }
    }

    pub fn qualified(self, object: &ObjectRef) -> String {
        format!(
            "{}.{}",
            self.quote_ident(&object.schema),
            self.quote_ident(&object.name)
        )
    }

    /// The placeholder for the `n`th parameter (1-based).
    fn placeholder(self, n: usize) -> String {
        match self {
            Self::Postgres => format!("${n}"),
            Self::MySql | Self::Sqlite => "?".into(),
        }
    }

    fn as_text(self, column: &str) -> String {
        match self {
            Self::MySql => format!("CAST({column} AS CHAR)"),
            Self::Postgres | Self::Sqlite => format!("CAST({column} AS TEXT)"),
        }
    }

    fn like(self) -> &'static str {
        match self {
            Self::Postgres => "ILIKE",
            Self::MySql | Self::Sqlite => "LIKE",
        }
    }

    /// PostgreSQL and MySQL already treat `\` as LIKE's escape character.
    fn like_escape(self) -> &'static str {
        match self {
            Self::Sqlite => " ESCAPE '\\'",
            Self::Postgres | Self::MySql => "",
        }
    }

    fn bind(self, params: &mut Vec<Value>, value: String) -> String {
        params.push(Value::Text(value.into()));
        self.placeholder(params.len())
    }

    fn filter(self, filter: &Filter, params: &mut Vec<Value>) -> String {
        let column = self.quote_ident(&filter.column);
        let compare = |op: &str, params: &mut Vec<Value>| {
            let placeholder = self.bind(params, filter.value.clone());
            format!("{column} {op} {placeholder}")
        };
        match filter.op {
            FilterOp::Eq => compare("=", params),
            FilterOp::Ne => compare("<>", params),
            FilterOp::Lt => compare("<", params),
            FilterOp::Gt => compare(">", params),
            FilterOp::Le => compare("<=", params),
            FilterOp::Ge => compare(">=", params),
            FilterOp::IsNull => format!("{column} IS NULL"),
            FilterOp::IsNotNull => format!("{column} IS NOT NULL"),
            FilterOp::Contains | FilterOp::StartsWith => {
                let escaped = escape_like(&filter.value);
                let pattern = if filter.op == FilterOp::Contains {
                    format!("%{escaped}%")
                } else {
                    format!("{escaped}%")
                };
                let placeholder = self.bind(params, pattern);
                format!(
                    "{} {} {placeholder}{}",
                    self.as_text(&column),
                    self.like(),
                    self.like_escape()
                )
            }
            FilterOp::In => {
                let values: Vec<&str> = filter
                    .value
                    .split(',')
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .collect();
                if values.is_empty() {
                    return "1 = 0".into();
                }
                let placeholders: Vec<String> = values
                    .into_iter()
                    .map(|value| self.bind(params, value.to_owned()))
                    .collect();
                format!("{column} IN ({})", placeholders.join(", "))
            }
        }
    }

    fn where_clause(self, query: &RowQuery, params: &mut Vec<Value>) -> String {
        let mut conditions: Vec<String> = query
            .filters
            .iter()
            .map(|filter| self.filter(filter, params))
            .collect();
        if let Some(raw) = query.raw_where.as_deref().map(str::trim)
            && !raw.is_empty()
        {
            // On its own lines, so a trailing `--` comment in the raw text
            // cannot reach the builder's LIMIT.
            conditions.push(format!("(\n{raw}\n)"));
        }
        if conditions.is_empty() {
            String::new()
        } else {
            format!(" WHERE {}", conditions.join(" AND "))
        }
    }

    /// One page plus one row (to learn whether there is a next page).
    /// `key` is the primary key, used for a stable default order and as a
    /// tiebreaker after the user's sort.
    pub fn select_rows(self, query: &RowQuery, key: &[String]) -> Sql {
        let mut params = Vec::new();
        let mut text = format!("SELECT * FROM {}", self.qualified(&query.object));
        text.push_str(&self.where_clause(query, &mut params));
        let mut order: Vec<String> = query
            .sort
            .iter()
            .map(|sort| {
                let dir = match sort.dir {
                    SortDir::Asc => "ASC",
                    SortDir::Desc => "DESC",
                };
                format!("{} {dir}", self.quote_ident(&sort.column))
            })
            .collect();
        for column in key {
            if !query.sort.iter().any(|sort| &sort.column == column) {
                order.push(format!("{} ASC", self.quote_ident(column)));
            }
        }
        if !order.is_empty() {
            text.push_str(" ORDER BY ");
            text.push_str(&order.join(", "));
        }
        text.push_str(&format!(
            " LIMIT {} OFFSET {}",
            u64::from(query.limit) + 1,
            query.offset
        ));
        Sql { text, params }
    }

    pub fn count_rows(self, query: &RowQuery) -> Sql {
        let mut params = Vec::new();
        let mut text = format!("SELECT count(*) FROM {}", self.qualified(&query.object));
        text.push_str(&self.where_clause(query, &mut params));
        Sql { text, params }
    }
}
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p tabletist-db dialect`
Expected: PASS (11 tests).

- [ ] **Step 5: Commit**

```bash
git add crates/tabletist-db
git commit -m "Add catalog and row query types and the dialect SQL builder"
```

---

### Task 4: SQLite fixture, opening, schemas, objects, and structure

**Files:**
- Create: `crates/tabletist-db/fixtures/sqlite.sql`, `crates/tabletist-db/src/fixtures.rs`, `crates/tabletist-db/src/sqlite.rs`
- Modify: `crates/tabletist-db/src/lib.rs`

**Interfaces:**
- Consumes: `Error`, `Result`, `ObjectInfo`, `ObjectKind`, `ObjectRef`, `Structure`, `ColumnInfo`, `IndexInfo`, `ForeignKeyInfo`.
- Produces:
  - `fixtures::SQLITE_SQL: &str` (the fixture script), `fixtures::write_sqlite_demo(path: &Path) -> Result<()>` (creates a new file; errors if it already exists).
  - `sqlite::Conn` with `Conn::open(path: &Path) -> Result<Conn>` (async), `list_schemas(&self) -> Result<Vec<String>>`, `list_objects(&self, schema: &str) -> Result<Vec<ObjectInfo>>`, `describe(&self, object: &ObjectRef) -> Result<Structure>`, `primary_key(&self, object: &ObjectRef) -> Result<Vec<String>>` (all async).

- [ ] **Step 1: Write the fixture and the failing tests**

Create `crates/tabletist-db/fixtures/sqlite.sql`:

```sql
-- The shared fixture: every awkward case a browser meets. Used by the tests
-- and by the app's demo mode. Keep it deterministic (no random data).

CREATE TABLE users (
    id INTEGER PRIMARY KEY,
    email TEXT NOT NULL UNIQUE,
    name TEXT,
    created_at DATETIME NOT NULL DEFAULT '2026-01-01 00:00:00',
    active BOOLEAN NOT NULL DEFAULT 1,
    meta JSON,
    avatar BLOB,
    score REAL
);
CREATE INDEX users_name_idx ON users (name);

INSERT INTO users (id, email, name, created_at, active, meta, avatar, score) VALUES
    (1, 'ada@example.com', 'Ada Lovelace', '2026-01-02 09:00:00', 1, '{"plan":"pro","tags":["math","engines"]}', X'89504E47', 99.5),
    (2, 'bob@example.com', 'Bob', '2026-01-03 10:30:00', 0, NULL, NULL, NULL),
    (3, 'zoe@example.com', 'Zoë 🚀', '2026-01-04 11:45:00', 1, '{"plan":"free"}', X'00FF10', 12.25),
    (4, 'percent@example.com', '50% off', '2026-01-05 12:00:00', 1, NULL, NULL, 0),
    (5, 'null@example.com', NULL, '2026-01-06 13:15:00', 1, 'not json', CAST(X'C328' AS TEXT), -1);

CREATE TABLE orders (
    id INTEGER PRIMARY KEY,
    user_id INTEGER NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    total NUMERIC(10, 2) NOT NULL,
    note TEXT
);
CREATE INDEX orders_user_total_idx ON orders (user_id, total);

INSERT INTO orders (id, user_id, total, note) VALUES
    (1, 1, '19.99', 'first'),
    (2, 1, '5.00', NULL),
    (3, 3, '120.50', 'bulk');

-- No primary key: paging order is not stable.
CREATE TABLE events (kind TEXT, payload TEXT);
INSERT INTO events (kind, payload) VALUES ('login', 'ada'), ('logout', 'ada'), ('login', 'zoe');

-- Identifiers that need quoting.
CREATE TABLE "weird ""name""" ("col with space" TEXT, "select" INTEGER);
INSERT INTO "weird ""name""" VALUES ('quoted', 1);

-- Enough rows to page, sort and cancel on.
CREATE TABLE big (id INTEGER PRIMARY KEY, label TEXT NOT NULL, bucket INTEGER NOT NULL);
WITH RECURSIVE n(i) AS (SELECT 1 UNION ALL SELECT i + 1 FROM n WHERE i < 100000)
INSERT INTO big (id, label, bucket) SELECT i, 'row ' || i, i % 10 FROM n;

CREATE VIEW active_users AS SELECT id, email FROM users WHERE active = 1;
```

Create `crates/tabletist-db/src/fixtures.rs`:

```rust
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
    let flags = rusqlite::OpenFlags::SQLITE_OPEN_READ_WRITE | rusqlite::OpenFlags::SQLITE_OPEN_CREATE;
    let connection = rusqlite::Connection::open_with_flags(path, flags)
        .map_err(|error| Error::Io(error.to_string()))?;
    connection
        .execute_batch(SQLITE_SQL)
        .map_err(|error| Error::Io(error.to_string()))
}
```

Create `crates/tabletist-db/src/sqlite.rs` with only tests:

```rust
//! SQLite, opened read-only. rusqlite is blocking, so every call runs on
//! tokio's blocking pool.

#[cfg(test)]
mod tests {
    use super::*;

    async fn fixture() -> (Conn, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("fixture.db");
        crate::fixtures::write_sqlite_demo(&path).unwrap();
        (Conn::open(&path).await.unwrap(), dir)
    }

    #[tokio::test]
    async fn a_missing_file_is_a_connect_error_and_is_not_created() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("missing.db");
        assert!(matches!(Conn::open(&path).await, Err(Error::Connect(_))));
        assert!(!path.exists());
    }

    #[tokio::test]
    async fn a_non_database_file_is_a_connect_error() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("notes.txt");
        std::fs::write(&path, "these are not the tables you are looking for, not at all").unwrap();
        assert!(matches!(Conn::open(&path).await, Err(Error::Connect(_))));
    }

    #[tokio::test]
    async fn the_connection_refuses_writes() {
        let (conn, _dir) = fixture().await;
        let result = conn
            .run(|connection| {
                connection
                    .execute("DELETE FROM users", [])
                    .map_err(map_error)
            })
            .await;
        assert!(result.is_err());
        let count = conn
            .run(|connection| {
                connection
                    .query_row("SELECT count(*) FROM users", [], |row| row.get::<_, i64>(0))
                    .map_err(map_error)
            })
            .await
            .unwrap();
        assert_eq!(count, 5);
    }

    #[tokio::test]
    async fn schemas_are_main_and_attached_databases_without_temp() {
        let (conn, _dir) = fixture().await;
        assert_eq!(conn.list_schemas().await.unwrap(), vec!["main".to_owned()]);
    }

    #[tokio::test]
    async fn objects_are_sorted_tables_and_views_without_internal_tables() {
        let (conn, _dir) = fixture().await;
        let objects = conn.list_objects("main").await.unwrap();
        let names: Vec<(&str, ObjectKind)> = objects
            .iter()
            .map(|object| (object.name.as_str(), object.kind))
            .collect();
        assert_eq!(
            names,
            vec![
                ("active_users", ObjectKind::View),
                ("big", ObjectKind::Table),
                ("events", ObjectKind::Table),
                ("orders", ObjectKind::Table),
                ("users", ObjectKind::Table),
                ("weird \"name\"", ObjectKind::Table),
            ]
        );
        assert!(objects.iter().all(|object| object.estimated_rows.is_none()));
    }

    #[tokio::test]
    async fn users_structure_has_columns_key_and_indexes() {
        let (conn, _dir) = fixture().await;
        let structure = conn.describe(&ObjectRef::new("main", "users")).await.unwrap();
        let columns: Vec<&str> = structure.columns.iter().map(|c| c.name.as_str()).collect();
        assert_eq!(
            columns,
            ["id", "email", "name", "created_at", "active", "meta", "avatar", "score"]
        );
        let email = &structure.columns[1];
        assert_eq!(email.type_name, "TEXT");
        assert!(!email.nullable);
        let created = &structure.columns[3];
        assert_eq!(created.default.as_deref(), Some("'2026-01-01 00:00:00'"));
        assert_eq!(structure.primary_key, vec!["id".to_owned()]);
        let name_index = structure
            .indexes
            .iter()
            .find(|index| index.name == "users_name_idx")
            .unwrap();
        assert_eq!(name_index.columns, vec!["name".to_owned()]);
        assert!(!name_index.unique);
        assert!(
            structure
                .indexes
                .iter()
                .any(|index| index.unique && index.columns == vec!["email".to_owned()]),
            "the UNIQUE constraint's automatic index is listed"
        );
    }

    #[tokio::test]
    async fn orders_structure_has_a_foreign_key_and_a_composite_index() {
        let (conn, _dir) = fixture().await;
        let structure = conn.describe(&ObjectRef::new("main", "orders")).await.unwrap();
        assert_eq!(structure.foreign_keys.len(), 1);
        let key = &structure.foreign_keys[0];
        assert_eq!(key.name, None);
        assert_eq!(key.columns, vec!["user_id".to_owned()]);
        assert_eq!(key.ref_table, "users");
        assert_eq!(key.ref_columns, vec!["id".to_owned()]);
        assert_eq!(key.on_delete, "CASCADE");
        let composite = structure
            .indexes
            .iter()
            .find(|index| index.name == "orders_user_total_idx")
            .unwrap();
        assert_eq!(composite.columns, vec!["user_id".to_owned(), "total".to_owned()]);
    }

    #[tokio::test]
    async fn views_have_columns_but_no_key() {
        let (conn, _dir) = fixture().await;
        let structure = conn
            .describe(&ObjectRef::new("main", "active_users"))
            .await
            .unwrap();
        assert_eq!(structure.columns.len(), 2);
        assert!(structure.primary_key.is_empty());
        assert!(conn.primary_key(&ObjectRef::new("main", "events")).await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn describing_a_missing_object_is_a_query_error() {
        let (conn, _dir) = fixture().await;
        let result = conn.describe(&ObjectRef::new("main", "nope")).await;
        assert!(matches!(result, Err(Error::Query { .. })), "{result:?}");
    }
}
```

Add to `lib.rs`:

```rust
pub mod fixtures;
// Reached through `Connection` from Task 5; until then only the tests use it.
#[allow(dead_code)]
mod sqlite;
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p tabletist-db sqlite`
Expected: FAIL to compile: `cannot find type Conn`, `cannot find function map_error`.

- [ ] **Step 3: Implement opening, listing and describing**

Put above the tests in `sqlite.rs`:

```rust
use std::path::Path;
use std::sync::{Arc, Mutex};

use rusqlite::{ErrorCode, OpenFlags};

use crate::{
    ColumnInfo, Error, ForeignKeyInfo, IndexInfo, ObjectInfo, ObjectKind, ObjectRef, Result,
    Structure,
};

/// An open SQLite database.
pub struct Conn {
    inner: Arc<Mutex<rusqlite::Connection>>,
    interrupt: Arc<rusqlite::InterruptHandle>,
}

/// Maps rusqlite's errors onto ours.
pub(crate) fn map_error(error: rusqlite::Error) -> Error {
    match &error {
        rusqlite::Error::SqliteFailure(failure, message) => {
            let message = message.clone().unwrap_or_else(|| error.to_string());
            match failure.code {
                ErrorCode::OperationInterrupted => Error::Cancelled,
                ErrorCode::CannotOpen | ErrorCode::NotADatabase => Error::Connect(message),
                _ => Error::Query {
                    code: Some(failure.extended_code.to_string()),
                    message,
                    detail: None,
                    hint: None,
                },
            }
        }
        _ => Error::query(error.to_string()),
    }
}

impl Conn {
    /// Opens `path` read-only. Never creates a file.
    pub async fn open(path: &Path) -> Result<Self> {
        let path = path.to_path_buf();
        let connection = tokio::task::spawn_blocking(move || -> Result<rusqlite::Connection> {
            if !path.is_file() {
                return Err(Error::Connect(format!("{} does not exist", path.display())));
            }
            let flags = OpenFlags::SQLITE_OPEN_READ_ONLY
                | OpenFlags::SQLITE_OPEN_NO_MUTEX
                | OpenFlags::SQLITE_OPEN_URI;
            let connection = rusqlite::Connection::open_with_flags(&path, flags).map_err(map_error)?;
            connection
                .busy_timeout(std::time::Duration::from_secs(5))
                .map_err(map_error)?;
            connection
                .execute_batch("PRAGMA query_only = ON;")
                .map_err(map_error)?;
            // A file that is not a database only fails on its first read.
            connection
                .query_row("SELECT count(*) FROM sqlite_master", [], |row| row.get::<_, i64>(0))
                .map_err(|error| match map_error(error) {
                    Error::Query { message, .. } => Error::Connect(message),
                    other => other,
                })?;
            Ok(connection)
        })
        .await
        .map_err(|error| Error::Io(error.to_string()))??;
        let interrupt = Arc::new(connection.get_interrupt_handle());
        Ok(Self {
            inner: Arc::new(Mutex::new(connection)),
            interrupt,
        })
    }

    /// Runs `work` on the blocking pool with the connection.
    pub(crate) async fn run<T: Send + 'static>(
        &self,
        work: impl FnOnce(&rusqlite::Connection) -> Result<T> + Send + 'static,
    ) -> Result<T> {
        let inner = Arc::clone(&self.inner);
        tokio::task::spawn_blocking(move || {
            let connection = inner
                .lock()
                .map_err(|_| Error::ConnectionLost("the SQLite connection was poisoned".into()))?;
            work(&connection)
        })
        .await
        .map_err(|error| Error::Io(error.to_string()))?
    }

    pub(crate) fn interrupt_handle(&self) -> Arc<rusqlite::InterruptHandle> {
        Arc::clone(&self.interrupt)
    }

    /// `main` plus attached databases (`temp` is hidden).
    pub async fn list_schemas(&self) -> Result<Vec<String>> {
        self.run(|connection| {
            let mut statement = connection
                .prepare("SELECT name FROM pragma_database_list WHERE name <> 'temp' ORDER BY seq")
                .map_err(map_error)?;
            let names = statement
                .query_map([], |row| row.get::<_, String>(0))
                .map_err(map_error)?
                .collect::<std::result::Result<Vec<_>, _>>()
                .map_err(map_error)?;
            Ok(names)
        })
        .await
    }

    pub async fn list_objects(&self, schema: &str) -> Result<Vec<ObjectInfo>> {
        let sql = format!(
            "SELECT name, type FROM {}.sqlite_master \
             WHERE type IN ('table', 'view') AND name NOT LIKE 'sqlite\\_%' ESCAPE '\\' \
             ORDER BY name",
            crate::Dialect::Sqlite.quote_ident(schema)
        );
        self.run(move |connection| {
            let mut statement = connection.prepare(&sql).map_err(map_error)?;
            let objects = statement
                .query_map([], |row| {
                    let kind: String = row.get(1)?;
                    Ok(ObjectInfo {
                        name: row.get(0)?,
                        kind: if kind == "view" {
                            ObjectKind::View
                        } else {
                            ObjectKind::Table
                        },
                        estimated_rows: None,
                    })
                })
                .map_err(map_error)?
                .collect::<std::result::Result<Vec<_>, _>>()
                .map_err(map_error)?;
            Ok(objects)
        })
        .await
    }

    /// The primary key columns in key order; empty for views and keyless tables.
    pub async fn primary_key(&self, object: &ObjectRef) -> Result<Vec<String>> {
        let object = object.clone();
        self.run(move |connection| primary_key(connection, &object)).await
    }

    pub async fn describe(&self, object: &ObjectRef) -> Result<Structure> {
        let object = object.clone();
        self.run(move |connection| {
            let columns = columns(connection, &object)?;
            if columns.is_empty() {
                return Err(Error::query(format!("no such table or view: {}", object.name)));
            }
            Ok(Structure {
                columns,
                primary_key: primary_key(connection, &object)?,
                indexes: indexes(connection, &object)?,
                foreign_keys: foreign_keys(connection, &object)?,
            })
        })
        .await
    }
}

fn columns(connection: &rusqlite::Connection, object: &ObjectRef) -> Result<Vec<ColumnInfo>> {
    let mut statement = connection
        .prepare(
            "SELECT name, type, \"notnull\", dflt_value FROM pragma_table_xinfo(?1, ?2) \
             WHERE hidden <> 1 ORDER BY cid",
        )
        .map_err(map_error)?;
    statement
        .query_map([&object.name, &object.schema], |row| {
            Ok(ColumnInfo {
                name: row.get(0)?,
                type_name: row.get::<_, Option<String>>(1)?.unwrap_or_default(),
                nullable: row.get::<_, i64>(2)? == 0,
                default: row.get(3)?,
                comment: None,
            })
        })
        .map_err(map_error)?
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(map_error)
}

fn primary_key(connection: &rusqlite::Connection, object: &ObjectRef) -> Result<Vec<String>> {
    let mut statement = connection
        .prepare("SELECT name FROM pragma_table_info(?1, ?2) WHERE pk > 0 ORDER BY pk")
        .map_err(map_error)?;
    statement
        .query_map([&object.name, &object.schema], |row| row.get::<_, String>(0))
        .map_err(map_error)?
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(map_error)
}

fn indexes(connection: &rusqlite::Connection, object: &ObjectRef) -> Result<Vec<IndexInfo>> {
    let mut list = connection
        .prepare("SELECT name, \"unique\", origin FROM pragma_index_list(?1, ?2) ORDER BY name")
        .map_err(map_error)?;
    let entries = list
        .query_map([&object.name, &object.schema], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, i64>(1)? != 0,
                row.get::<_, String>(2)?,
            ))
        })
        .map_err(map_error)?
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(map_error)?;
    let mut info = connection
        .prepare("SELECT name FROM pragma_index_info(?1, ?2) ORDER BY seqno")
        .map_err(map_error)?;
    let mut indexes = Vec::new();
    for (name, unique, origin) in entries {
        let columns = info
            .query_map([&name, &object.schema], |row| row.get::<_, Option<String>>(0))
            .map_err(map_error)?
            .map(|column| column.map(|c| c.unwrap_or_else(|| "<expression>".into())))
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(map_error)?;
        indexes.push(IndexInfo {
            name,
            columns,
            unique,
            primary: origin == "pk",
            method: None,
        });
    }
    Ok(indexes)
}

fn foreign_keys(connection: &rusqlite::Connection, object: &ObjectRef) -> Result<Vec<ForeignKeyInfo>> {
    let mut statement = connection
        .prepare(
            "SELECT id, \"table\", \"from\", \"to\", on_update, on_delete \
             FROM pragma_foreign_key_list(?1, ?2) ORDER BY id, seq",
        )
        .map_err(map_error)?;
    let rows = statement
        .query_map([&object.name, &object.schema], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, Option<String>>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, String>(5)?,
            ))
        })
        .map_err(map_error)?
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(map_error)?;
    let mut keys: Vec<(i64, ForeignKeyInfo)> = Vec::new();
    for (id, table, from, to, on_update, on_delete) in rows {
        if keys.last().is_none_or(|(last, _)| *last != id) {
            keys.push((
                id,
                ForeignKeyInfo {
                    name: None,
                    columns: Vec::new(),
                    ref_schema: object.schema.clone(),
                    ref_table: table,
                    ref_columns: Vec::new(),
                    on_update,
                    on_delete,
                },
            ));
        }
        let (_, key) = keys.last_mut().expect("pushed above");
        key.columns.push(from);
        if let Some(to) = to {
            key.ref_columns.push(to);
        }
    }
    Ok(keys.into_iter().map(|(_, key)| key).collect())
}
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p tabletist-db sqlite`
Expected: PASS (9 tests).

- [ ] **Step 5: Commit**

```bash
git add crates/tabletist-db
git commit -m "Open SQLite and list schemas, objects and structure"
```

---

### Task 5: SQLite rows, counts, cancellation, and the Connection enum

**Files:**
- Modify: `crates/tabletist-db/src/sqlite.rs`, `crates/tabletist-db/src/lib.rs`
- Create: `crates/tabletist-db/tests/sqlite.rs`

**Interfaces:**
- Consumes: everything above.
- Produces:
  - `sqlite::Conn::fetch_rows(&self, query: &RowQuery) -> Result<RowPage>`, `sqlite::Conn::count_rows(&self, query: &RowQuery) -> Result<u64>` (async).
  - `tabletist_db::Connection` enum (variant `Sqlite(sqlite::Conn)` for now) with async `Connection::connect(spec: &ConnectSpec, secrets: &Secrets) -> Result<Connection>`, `list_databases`, `list_schemas`, `list_objects(schema: &str)`, `describe(object: &ObjectRef)`, `fetch_rows(query: &RowQuery)`, `count_rows(query: &RowQuery)`, `close(self)`, and sync `driver(&self) -> Driver`, `dialect(&self) -> Dialect`, `cancel_handle(&self) -> CancelHandle`.
  - `tabletist_db::CancelHandle` (`Clone`, `Debug`): `CancelHandle::cancel(&self) -> Result<()>` (async).

- [ ] **Step 1: Write the failing integration tests**

Create `crates/tabletist-db/tests/sqlite.rs`:

```rust
//! The SQLite adapter through the public `Connection` API, on the shared fixture.

// `allow-unwrap-in-tests` does not cover helpers in an integration-test crate.
#![allow(clippy::unwrap_used)]

use std::time::Duration;

use tabletist_db::{
    ConnectSpec, Connection, Driver, Error, Filter, FilterOp, ObjectRef, RowQuery, Secrets, Sort,
    SortDir, Value, ValueKind,
};

async fn fixture() -> (Connection, tempfile::TempDir) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("fixture.db");
    tabletist_db::fixtures::write_sqlite_demo(&path).unwrap();
    let connection = Connection::connect(&ConnectSpec::sqlite(&path), &Secrets::default())
        .await
        .unwrap();
    (connection, dir)
}

fn users(limit: u32) -> RowQuery {
    RowQuery::new(ObjectRef::new("main", "users"), limit)
}

fn ids(page: &tabletist_db::RowPage) -> Vec<i64> {
    page.rows
        .iter()
        .map(|row| match row[0] {
            Value::Int(id) => id,
            ref other => panic!("id is {other:?}"),
        })
        .collect()
}

#[tokio::test]
async fn connections_report_their_driver_and_have_no_database_list() {
    let (connection, _dir) = fixture().await;
    assert_eq!(connection.driver(), Driver::Sqlite);
    assert!(connection.list_databases().await.unwrap().is_empty());
}

#[tokio::test]
async fn postgres_and_mysql_are_not_supported_yet() {
    let (spec, secrets) = ConnectSpec::from_url("postgres://localhost/db").unwrap();
    assert!(matches!(
        Connection::connect(&spec, &secrets).await,
        Err(Error::Unsupported(_))
    ));
}

#[tokio::test]
async fn a_sqlite_spec_without_a_path_is_invalid() {
    let spec = ConnectSpec {
        driver: Driver::Sqlite,
        ..ConnectSpec::default()
    };
    assert!(matches!(
        Connection::connect(&spec, &Secrets::default()).await,
        Err(Error::InvalidSpec(_))
    ));
}

#[tokio::test]
async fn the_first_page_is_ordered_by_key_with_column_metadata() {
    let (connection, _dir) = fixture().await;
    let page = connection.fetch_rows(&users(3)).await.unwrap();
    assert_eq!(ids(&page), vec![1, 2, 3]);
    assert!(page.has_more);
    assert!(page.ordered_by_key);
    let columns: Vec<(&str, ValueKind)> = page
        .columns
        .iter()
        .map(|column| (column.name.as_str(), column.kind))
        .collect();
    assert_eq!(
        columns,
        vec![
            ("id", ValueKind::Numeric),
            ("email", ValueKind::Text),
            ("name", ValueKind::Text),
            ("created_at", ValueKind::Temporal),
            ("active", ValueKind::Bool),
            ("meta", ValueKind::Json),
            ("avatar", ValueKind::Binary),
            ("score", ValueKind::Numeric),
        ]
    );
}

#[tokio::test]
async fn paging_moves_through_the_table_and_the_last_page_has_no_more() {
    let (connection, _dir) = fixture().await;
    let mut query = users(2);
    query.offset = 4;
    let page = connection.fetch_rows(&query).await.unwrap();
    assert_eq!(ids(&page), vec![5]);
    assert!(!page.has_more);
}

#[tokio::test]
async fn a_user_sort_orders_rows() {
    let (connection, _dir) = fixture().await;
    let mut query = users(5);
    query.sort = vec![Sort {
        column: "email".into(),
        dir: SortDir::Desc,
    }];
    let page = connection.fetch_rows(&query).await.unwrap();
    assert_eq!(ids(&page), vec![3, 4, 5, 2, 1]);
}

#[tokio::test]
async fn filters_narrow_rows_and_counts_agree() {
    let (connection, _dir) = fixture().await;
    let mut query = users(50);
    query.filters = vec![Filter {
        column: "name".into(),
        op: FilterOp::IsNull,
        value: String::new(),
    }];
    assert_eq!(ids(&connection.fetch_rows(&query).await.unwrap()), vec![5]);
    assert_eq!(connection.count_rows(&query).await.unwrap(), 1);

    query.filters = vec![Filter {
        column: "id".into(),
        op: FilterOp::In,
        value: "1, 3".into(),
    }];
    assert_eq!(ids(&connection.fetch_rows(&query).await.unwrap()), vec![1, 3]);

    query.filters = vec![Filter {
        column: "id".into(),
        op: FilterOp::Ge,
        value: "4".into(),
    }];
    assert_eq!(ids(&connection.fetch_rows(&query).await.unwrap()), vec![4, 5]);
}

#[tokio::test]
async fn contains_is_case_insensitive_and_handles_unicode() {
    let (connection, _dir) = fixture().await;
    let mut query = users(50);
    // SQLite's LIKE folds ASCII letters only; non-ASCII text matches exactly.
    query.filters = vec![Filter {
        column: "name".into(),
        op: FilterOp::Contains,
        value: "ZO".into(),
    }];
    assert_eq!(ids(&connection.fetch_rows(&query).await.unwrap()), vec![3]);
    query.filters[0].value = "ë 🚀".into();
    assert_eq!(ids(&connection.fetch_rows(&query).await.unwrap()), vec![3]);
}

#[tokio::test]
async fn contains_matches_literal_percent_signs() {
    let (connection, _dir) = fixture().await;
    let mut query = users(50);
    query.filters = vec![Filter {
        column: "name".into(),
        op: FilterOp::Contains,
        value: "50%".into(),
    }];
    assert_eq!(ids(&connection.fetch_rows(&query).await.unwrap()), vec![4]);
    query.filters[0].value = "%".into();
    assert_eq!(ids(&connection.fetch_rows(&query).await.unwrap()), vec![4]);
}

#[tokio::test]
async fn a_raw_where_cannot_modify_data() {
    let (connection, _dir) = fixture().await;
    let mut query = users(50);
    // A plain syntax error, and one that closes the parenthesis to chain a
    // real second statement (rusqlite refuses to prepare more than one).
    for raw in ["1 = 1; DELETE FROM users", "1=1) ; DELETE FROM users; SELECT (1"] {
        query.raw_where = Some(raw.into());
        assert!(connection.fetch_rows(&query).await.is_err(), "{raw}");
    }
    assert_eq!(connection.count_rows(&users(50)).await.unwrap(), 5);
}

#[tokio::test]
async fn a_raw_where_cannot_drop_the_page_limit() {
    let (connection, _dir) = fixture().await;
    let mut query = RowQuery::new(ObjectRef::new("main", "big"), 10);
    // An unterminated comment would swallow the builder's LIMIT and OFFSET.
    for raw in ["1=1) /*", "1=1) --"] {
        query.raw_where = Some(raw.into());
        if let Ok(page) = connection.fetch_rows(&query).await {
            assert!(page.rows.len() <= 10, "{raw}: {} rows", page.rows.len());
        }
    }
}

#[tokio::test]
async fn a_bad_raw_where_is_a_query_error() {
    let (connection, _dir) = fixture().await;
    let mut query = users(50);
    query.raw_where = Some("no_such_column = 1".into());
    assert!(matches!(
        connection.fetch_rows(&query).await,
        Err(Error::Query { .. })
    ));
}

#[tokio::test]
async fn blobs_and_invalid_utf8_load_without_failing() {
    let (connection, _dir) = fixture().await;
    let page = connection.fetch_rows(&users(50)).await.unwrap();
    assert_eq!(page.rows[0][6], Value::Bytes(vec![0x89, 0x50, 0x4E, 0x47].into()));
    assert_eq!(page.rows[1][6], Value::Null);
    match &page.rows[4][6] {
        Value::Text(text) => assert!(text.contains('\u{FFFD}'), "{text:?}"),
        other => panic!("invalid UTF-8 text should load as lossy text, got {other:?}"),
    }
    assert_eq!(page.rows[2][2], Value::Text("Zoë 🚀".into()));
    assert_eq!(page.rows[0][7], Value::Float(99.5));
}

#[tokio::test]
async fn keyless_tables_page_without_a_stable_order() {
    let (connection, _dir) = fixture().await;
    let page = connection
        .fetch_rows(&RowQuery::new(ObjectRef::new("main", "events"), 50))
        .await
        .unwrap();
    assert_eq!(page.rows.len(), 3);
    assert!(!page.ordered_by_key);
}

#[tokio::test]
async fn a_table_with_a_quoted_name_can_be_browsed() {
    let (connection, _dir) = fixture().await;
    let object = ObjectRef::new("main", "weird \"name\"");
    let mut query = RowQuery::new(object.clone(), 50);
    query.sort = vec![Sort {
        column: "select".into(),
        dir: SortDir::Asc,
    }];
    let page = connection.fetch_rows(&query).await.unwrap();
    assert_eq!(page.columns[0].name, "col with space");
    assert_eq!(page.rows[0][0], Value::Text("quoted".into()));
    assert_eq!(connection.describe(&object).await.unwrap().columns.len(), 2);
}

#[tokio::test]
async fn big_tables_page_deep_quickly() {
    let (connection, _dir) = fixture().await;
    let mut query = RowQuery::new(ObjectRef::new("main", "big"), 300);
    query.offset = 99_900;
    let page = connection.fetch_rows(&query).await.unwrap();
    assert_eq!(page.rows.len(), 100);
    assert!(!page.has_more);
    assert_eq!(connection.count_rows(&RowQuery::new(ObjectRef::new("main", "big"), 300)).await.unwrap(), 100_000);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_running_query_can_be_cancelled() {
    let (connection, _dir) = fixture().await;
    let cancel = connection.cancel_handle();
    let connection = std::sync::Arc::new(connection);
    let running = {
        let connection = std::sync::Arc::clone(&connection);
        tokio::spawn(async move {
            let mut query = RowQuery::new(ObjectRef::new("main", "big"), 10);
            // Ten billion row pairs: far longer than the test waits.
            query.raw_where = Some("(SELECT count(*) FROM big a, big b) > 0".into());
            connection.count_rows(&query).await
        })
    };
    // SQLite ignores an interrupt when nothing runs yet, so keep cancelling
    // until the query stops. A single early cancel would leave the blocking
    // job running and hang the test runtime on shutdown.
    let deadline = std::time::Instant::now() + Duration::from_secs(10);
    while !running.is_finished() {
        assert!(std::time::Instant::now() < deadline, "cancel must stop the query");
        cancel.cancel().await.unwrap();
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    let result = running.await.unwrap();
    assert_eq!(result, Err(Error::Cancelled));
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p tabletist-db --test sqlite`
Expected: FAIL to compile: `cannot find type Connection in tabletist_db`.

- [ ] **Step 3: Implement rows, counts, cancel and Connection**

Add to `sqlite.rs` (inside `impl Conn`, after `describe`):

```rust
    pub async fn fetch_rows(&self, query: &RowQuery) -> Result<RowPage> {
        let key = self.primary_key(&query.object).await?;
        let sql = Dialect::Sqlite.select_rows(query, &key);
        let limit = query.limit as usize;
        let ordered_by_key = !key.is_empty();
        self.run(move |connection| {
            let started = Instant::now();
            let mut statement = connection.prepare(&sql.text).map_err(map_error)?;
            let declared: Vec<(String, String)> = statement
                .columns()
                .iter()
                .map(|column| {
                    (
                        column.name().to_owned(),
                        column.decl_type().unwrap_or_default().to_owned(),
                    )
                })
                .collect();
            let mut rows = statement
                .query(rusqlite::params_from_iter(sql.params.iter().map(to_sqlite)))
                .map_err(map_error)?;
            let mut values: Vec<Vec<Value>> = Vec::new();
            while let Some(row) = rows.next().map_err(map_error)? {
                let mut cells = Vec::with_capacity(declared.len());
                for index in 0..declared.len() {
                    cells.push(from_sqlite(row.get_ref(index).map_err(map_error)?));
                }
                values.push(cells);
                // Never read past one extra row, even if a raw WHERE comments
                // out the LIMIT.
                if values.len() > limit {
                    break;
                }
            }
            let has_more = values.len() > limit;
            values.truncate(limit);
            let columns = declared
                .into_iter()
                .enumerate()
                .map(|(index, (name, type_name))| {
                    let kind = if type_name.is_empty() {
                        values
                            .iter()
                            .map(|row| &row[index])
                            .find(|value| !value.is_null())
                            .map_or(ValueKind::Other, ValueKind::of_value)
                    } else {
                        ValueKind::from_sqlite_decl(&type_name)
                    };
                    ColumnMeta {
                        name,
                        type_name,
                        kind,
                    }
                })
                .collect();
            Ok(RowPage {
                columns,
                rows: values,
                has_more,
                ordered_by_key,
                elapsed: started.elapsed(),
            })
        })
        .await
    }

    pub async fn count_rows(&self, query: &RowQuery) -> Result<u64> {
        let sql = Dialect::Sqlite.count_rows(query);
        self.run(move |connection| {
            let count: i64 = connection
                .query_row(
                    &sql.text,
                    rusqlite::params_from_iter(sql.params.iter().map(to_sqlite)),
                    |row| row.get(0),
                )
                .map_err(map_error)?;
            Ok(u64::try_from(count).unwrap_or(0))
        })
        .await
    }
```

Add these free functions to `sqlite.rs` (after `map_error`):

```rust
fn to_sqlite(value: &Value) -> rusqlite::types::Value {
    use rusqlite::types::Value as Sqlite;
    match value {
        Value::Null => Sqlite::Null,
        Value::Bool(flag) => Sqlite::Integer(i64::from(*flag)),
        Value::Int(number) => Sqlite::Integer(*number),
        Value::Float(number) => Sqlite::Real(*number),
        Value::Text(text) => Sqlite::Text(text.to_string()),
        Value::Bytes(bytes) => Sqlite::Blob(bytes.to_vec()),
    }
}

fn from_sqlite(value: rusqlite::types::ValueRef<'_>) -> Value {
    use rusqlite::types::ValueRef;
    match value {
        ValueRef::Null => Value::Null,
        ValueRef::Integer(number) => Value::Int(number),
        ValueRef::Real(number) => Value::Float(number),
        ValueRef::Text(bytes) => Value::Text(String::from_utf8_lossy(bytes).into()),
        ValueRef::Blob(bytes) => Value::Bytes(bytes.into()),
    }
}
```

Change the `use` lines at the top of `sqlite.rs` to:

```rust
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use rusqlite::{ErrorCode, OpenFlags};

use crate::{
    ColumnInfo, ColumnMeta, Dialect, Error, ForeignKeyInfo, IndexInfo, ObjectInfo, ObjectKind,
    ObjectRef, Result, RowPage, RowQuery, Structure, Value, ValueKind,
};
```

rusqlite refuses to `prepare` text holding more than one statement (`Error::MultipleStatement`), which `map_error` turns into a `Query` error; the read-only flags stop writes regardless.

Replace `crates/tabletist-db/src/lib.rs` with:

```rust
//! Access to PostgreSQL, MySQL and SQLite for Tabletist.
//!
//! Nothing in this crate writes to a connected database: every session is
//! opened read-only.

mod catalog;
mod dialect;
mod error;
pub mod fixtures;
mod query;
mod spec;
mod sqlite;
mod value;

use std::fmt;
use std::sync::Arc;

pub use catalog::{ColumnInfo, ForeignKeyInfo, IndexInfo, ObjectInfo, ObjectKind, ObjectRef, Structure};
pub use dialect::{Dialect, Sql, escape_like};
pub use error::{Error, Result, SshStage};
pub use query::{Filter, FilterOp, RowPage, RowQuery, Sort, SortDir};
pub use spec::{ConnectSpec, Driver, Secrets, SshAuth, SshSpec, TlsMode};
pub use value::{ColumnMeta, Value, ValueKind};

/// An open, read-only database session. Batches 4 and 5 add PostgreSQL and
/// MySQL variants.
pub enum Connection {
    Sqlite(sqlite::Conn),
}

impl Connection {
    pub async fn connect(spec: &ConnectSpec, _secrets: &Secrets) -> Result<Self> {
        match spec.driver {
            Driver::Sqlite => {
                let path = spec
                    .sqlite_path
                    .as_ref()
                    .ok_or_else(|| Error::InvalidSpec("choose a SQLite file".into()))?;
                Ok(Self::Sqlite(sqlite::Conn::open(path).await?))
            }
            Driver::Postgres => Err(Error::Unsupported("PostgreSQL connections")),
            Driver::MySql => Err(Error::Unsupported("MySQL connections")),
        }
    }

    pub fn driver(&self) -> Driver {
        match self {
            Self::Sqlite(_) => Driver::Sqlite,
        }
    }

    pub fn dialect(&self) -> Dialect {
        match self {
            Self::Sqlite(_) => Dialect::Sqlite,
        }
    }

    /// Databases the session can switch to. Empty when switching does not
    /// apply (SQLite, and MySQL where databases are listed as schemas).
    pub async fn list_databases(&self) -> Result<Vec<String>> {
        match self {
            Self::Sqlite(_) => Ok(Vec::new()),
        }
    }

    pub async fn list_schemas(&self) -> Result<Vec<String>> {
        match self {
            Self::Sqlite(conn) => conn.list_schemas().await,
        }
    }

    pub async fn list_objects(&self, schema: &str) -> Result<Vec<ObjectInfo>> {
        match self {
            Self::Sqlite(conn) => conn.list_objects(schema).await,
        }
    }

    pub async fn describe(&self, object: &ObjectRef) -> Result<Structure> {
        match self {
            Self::Sqlite(conn) => conn.describe(object).await,
        }
    }

    pub async fn fetch_rows(&self, query: &RowQuery) -> Result<RowPage> {
        match self {
            Self::Sqlite(conn) => conn.fetch_rows(query).await,
        }
    }

    pub async fn count_rows(&self, query: &RowQuery) -> Result<u64> {
        match self {
            Self::Sqlite(conn) => conn.count_rows(query).await,
        }
    }

    /// A handle that cancels whatever this session is running, from any task.
    pub fn cancel_handle(&self) -> CancelHandle {
        match self {
            Self::Sqlite(conn) => CancelHandle(CancelInner::Sqlite(conn.interrupt_handle())),
        }
    }

    pub async fn close(self) -> Result<()> {
        match self {
            Self::Sqlite(conn) => {
                drop(conn);
                Ok(())
            }
        }
    }
}

/// Cancels the query a session is running. Cancelling when nothing runs
/// does nothing.
#[derive(Clone)]
pub struct CancelHandle(CancelInner);

#[derive(Clone)]
enum CancelInner {
    Sqlite(Arc<rusqlite::InterruptHandle>),
}

impl CancelHandle {
    pub async fn cancel(&self) -> Result<()> {
        match &self.0 {
            CancelInner::Sqlite(handle) => {
                handle.interrupt();
                Ok(())
            }
        }
    }
}

impl fmt::Debug for CancelHandle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("CancelHandle")
    }
}
```

- [ ] **Step 4: Run all crate tests**

Run: `cargo test -p tabletist-db`
Expected: PASS (unit and integration tests).

- [ ] **Step 5: Full checks and commit**

Run: `cargo fmt --all --check && cargo clippy --locked --all-targets -- -D warnings && cargo test --locked --all-targets`
Expected: all pass.

```bash
git add crates/tabletist-db Cargo.lock
git commit -m "Browse SQLite rows with paging, filters, counts and cancel"
```

---

## Done when

- `cargo test -p tabletist-db` passes on the fixture: schemas, objects, structure, paging, sorting, filtering, counting, cancelling, quoting, awkward values, read-only enforcement.
- `Connection` is the only type the app needs from this crate to browse a database.

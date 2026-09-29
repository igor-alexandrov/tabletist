# Batch 5: MySQL + TLS Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Browse MySQL (8.0+) databases the same way as PostgreSQL and SQLite: a read-only adapter (schemas, objects with estimates, structure, rows, counts, cancel through `KILL QUERY`), TLS in every mode, and the connection dialog, URL paste and keyring working for MySQL.

**Architecture:** `tabletist-db` gains `mysql.rs` on `mysql_async` with its rustls/ring backend (the same rustls 0.23 as PostgreSQL; no aws-lc). Unlike PostgreSQL, rows are read with **prepared statements** (the binary protocol): it returns typed values and column types, binds the dialect's `?` parameters as they are, and a prepared statement is always a single statement. `mysql_async` enables multi-statements for plain `query` calls, so user-influenced SQL is only ever run through `exec*`. Every row and count query runs in `START TRANSACTION READ ONLY` on top of `SET SESSION TRANSACTION READ ONLY`. MySQL has no separate schema level: databases are listed as schemas (the spec's choice), and `list_databases` is empty, so no database switcher appears. Cancel opens a short second connection that sends `KILL QUERY <id>`.

**Tech Stack:** mysql_async 0.37 (`default-features = false`, feature `default-rustls-ring`); MySQL 8.4 in Docker for integration tests.

**Spec:** `docs/superpowers/specs/2026-09-27-tabletist-design.md` (sections 4.1 to 4.8)

## Global Constraints

- Everything from earlier batches' Global Constraints still applies. Verify with `cargo test --locked --workspace --all-targets` and `cargo clippy --locked --workspace --all-targets -- -D warnings`; run `cargo fmt --all` before every commit; commits are signed.
- User-influenced SQL (`select_rows`, `count_rows`) runs only through `exec`/`exec_iter`/`exec_first` (prepared statements). Never pass it to `query`/`query_drop`, which allow several statements in one call.
- MySQL sessions: `SET SESSION TRANSACTION READ ONLY` and `sql_mode` without `NO_BACKSLASH_ESCAPES` (so LIKE's default `\` escape works) right after connect; every row and count query inside a read-only transaction that is rolled back.
- rustls stays on `ring` alone across the workspace: `mysql_async` builds its TLS config with rustls's process-default provider, which is only chosen automatically when exactly one provider is compiled in. A test guards this.
- TLS modes: `disable` plain; `prefer` TLS without verification, falling back to plain only when the server has no TLS; `require` TLS without verification; `verify-ca` chain only; `verify-full` chain and host. MySQL's trust roots are the Mozilla bundle (webpki-roots) plus the CA file, not the OS store (a documented difference from PostgreSQL).
- MySQL integration tests read `TABLETIST_TEST_MYSQL_URL`; without it they print "skipped" and pass. `compose.yaml` provides the server at `mysql://tabletist:tabletist@localhost:53306/tabletist`, with a `billing` database the same user can read.
- Never use em dashes. One topic per commit on `main`.

## Review Focus

1. **Values MySQL's binary protocol sends in awkward shapes:** `BIGINT UNSIGNED` above `i64::MAX`, `DECIMAL`, `DATE`/`DATETIME(6)`/negative `TIME`, `JSON`, binary vs text `BLOB`s, `ENUM`: each shows its real value, never a garbled or rounded one. Test: Task 1 `mysql_values_become_display_values` and Task 4 `pages_have_typed_values_and_kinds`.
2. **A raw WHERE that tries to write or chain statements** (`1 = 1; DELETE FROM users`, `1=1) ; COMMIT; DELETE ...`): refused by the prepared statement or the read-only transaction; nothing changes. Test: Task 4 `a_raw_where_cannot_write_or_chain_statements`.
3. **Filter values with `'`, `\`, `%`, `_`** under any server `sql_mode`: match literally. Test: Task 4 `quotes_backslashes_and_wildcards_are_just_text`.
4. **Cancel while a heavy query runs**: returns `Cancelled` and the session stays usable. Test: Task 4 `a_running_query_can_be_cancelled`.
5. **A server with several databases**: the tree expands the database the connection names, not an arbitrary one. Test: Task 5 `the_connections_database_is_expanded_first`.

---

## File Structure

```
compose.yaml                                  + mysql service (8.4) with a health check
compose/mysql-init.sql                        creates `billing` for the test user
.github/workflows/ci.yml                      + mysql integration job
AGENTS.md                                     + how to run the MySQL suite
crates/tabletist-db/Cargo.toml                + mysql_async
crates/tabletist-db/src/tls.rs                + is_tls_error (moved from pg.rs), provider guard test
crates/tabletist-db/src/pg.rs                 uses tls::is_tls_error
crates/tabletist-db/src/mysql.rs              the MySQL adapter
crates/tabletist-db/src/lib.rs                Connection::MySql, CancelHandle for MySQL
crates/tabletist-db/src/fixtures.rs           + MYSQL_SQL
crates/tabletist-db/fixtures/mysql.sql        MySQL fixture
crates/tabletist-db/tests/mysql.rs            env-gated integration tests
crates/tabletist-db/tests/sqlite.rs           drop the "MySQL is not supported" test
src/model.rs                                  ConnectionForm::to_spec accepts MySQL
src/app.rs                                    URL paste fills MySQL; default schema expansion
src/ui/connect_dialog.rs                      MySQL button enabled
```

---

### Task 1: MySQL type names, values, TLS options and errors (no server)

**Files:**
- Create: `crates/tabletist-db/src/mysql.rs`
- Modify: `crates/tabletist-db/Cargo.toml`, `crates/tabletist-db/src/lib.rs`, `crates/tabletist-db/src/tls.rs`, `crates/tabletist-db/src/pg.rs`

**Interfaces:**
- Consumes: `Value`, `ValueKind`, `TlsMode`, `Error`.
- Produces (crate-private in `mysql.rs`): `type_name(column_type: ColumnType, flags: ColumnFlags, charset: u16) -> Cow<'static, str>`; `kind(type_name: &str) -> ValueKind`; `value(value: mysql_async::Value, type_name: &str) -> Value`; `to_mysql(value: &Value) -> mysql_async::Value`; `ssl_opts(mode: TlsMode, ca_file: Option<&Path>) -> Option<SslOpts>`; `connect_error(mysql_async::Error) -> Error`; `query_error(mysql_async::Error) -> Error`. `tls::is_tls_error(&(dyn std::error::Error + 'static)) -> bool` (moved from `pg.rs`).

- [ ] **Step 1: Add the dependency and write the failing tests**

Add to `crates/tabletist-db/Cargo.toml` `[dependencies]`:

```toml
# MySQL, async. rustls on ring (like PostgreSQL); no aws-lc, no OpenSSL.
mysql_async = { version = "0.37", default-features = false, features = ["default-rustls-ring"] }
```

Create `crates/tabletist-db/src/mysql.rs` with only tests:

```rust
//! MySQL 8.0+. Rows and catalog queries use prepared statements (the binary
//! protocol): typed values, column types, and never more than one statement.
//! Rows are read inside a read-only transaction.

#[cfg(test)]
mod tests {
    use super::*;
    use mysql_async::Value as My;

    #[test]
    fn column_types_get_readable_names() {
        let none = ColumnFlags::empty();
        let cases = [
            (ColumnType::MYSQL_TYPE_LONG, none, 63, "int"),
            (ColumnType::MYSQL_TYPE_LONGLONG, ColumnFlags::UNSIGNED_FLAG, 63, "bigint unsigned"),
            (ColumnType::MYSQL_TYPE_NEWDECIMAL, none, 63, "decimal"),
            (ColumnType::MYSQL_TYPE_VAR_STRING, none, 255, "varchar"),
            (ColumnType::MYSQL_TYPE_VAR_STRING, none, 63, "varbinary"),
            (ColumnType::MYSQL_TYPE_BLOB, none, 255, "text"),
            (ColumnType::MYSQL_TYPE_BLOB, none, 63, "blob"),
            (ColumnType::MYSQL_TYPE_STRING, ColumnFlags::ENUM_FLAG, 255, "enum"),
            (ColumnType::MYSQL_TYPE_DATETIME, none, 63, "datetime"),
            (ColumnType::MYSQL_TYPE_JSON, none, 63, "json"),
        ];
        for (column_type, flags, charset, name) in cases {
            assert_eq!(type_name(column_type, flags, charset), name, "{column_type:?}");
        }
    }

    #[test]
    fn names_map_to_kinds() {
        assert_eq!(kind("bigint unsigned"), ValueKind::Numeric);
        assert_eq!(kind("decimal"), ValueKind::Numeric);
        assert_eq!(kind("datetime"), ValueKind::Temporal);
        assert_eq!(kind("json"), ValueKind::Json);
        assert_eq!(kind("varbinary"), ValueKind::Binary);
        assert_eq!(kind("blob"), ValueKind::Binary);
        assert_eq!(kind("text"), ValueKind::Text);
        assert_eq!(kind("enum"), ValueKind::Text);
        assert_eq!(kind("geometry"), ValueKind::Other);
    }

    #[test]
    fn mysql_values_become_display_values() {
        assert_eq!(value(My::NULL, "int"), Value::Null);
        assert_eq!(value(My::Int(-7), "int"), Value::Int(-7));
        assert_eq!(value(My::UInt(u64::MAX), "bigint unsigned"), Value::Text("18446744073709551615".into()));
        assert_eq!(value(My::UInt(5), "bigint unsigned"), Value::Int(5));
        assert_eq!(value(My::Float(1.5), "float"), Value::Float(1.5));
        assert_eq!(value(My::Double(99.5), "double"), Value::Float(99.5));
        assert_eq!(value(My::Bytes(b"123456789012.34".to_vec()), "decimal"), Value::Text("123456789012.34".into()));
        assert_eq!(value(My::Date(1815, 12, 10, 0, 0, 0, 0), "date"), Value::Text("1815-12-10".into()));
        assert_eq!(
            value(My::Date(2026, 1, 2, 9, 0, 0, 0), "datetime"),
            Value::Text("2026-01-02 09:00:00".into())
        );
        assert_eq!(
            value(My::Date(2026, 1, 2, 9, 0, 0, 120), "datetime"),
            Value::Text("2026-01-02 09:00:00.000120".into())
        );
        assert_eq!(value(My::Time(false, 0, 7, 30, 0, 0), "time"), Value::Text("07:30:00".into()));
        assert_eq!(value(My::Time(true, 1, 1, 15, 0, 0), "time"), Value::Text("-25:15:00".into()));
        assert_eq!(value(My::Bytes(vec![0x89, 0x50]), "varbinary"), Value::Bytes(vec![0x89, 0x50].into()));
        assert_eq!(value(My::Bytes("Zoë".as_bytes().to_vec()), "varchar"), Value::Text("Zoë".into()));
        assert_eq!(value(My::Bytes(b"{\"a\": 1}".to_vec()), "json"), Value::Text("{\"a\": 1}".into()));
    }

    #[test]
    fn filter_values_bind_as_mysql_values() {
        assert_eq!(to_mysql(&Value::Text("O'Brien".into())), My::Bytes(b"O'Brien".to_vec()));
        assert_eq!(to_mysql(&Value::Null), My::NULL);
        assert_eq!(to_mysql(&Value::Int(3)), My::Int(3));
    }

    #[test]
    fn tls_modes_become_ssl_options() {
        assert!(ssl_opts(TlsMode::Disable, None).is_none());
        let require = ssl_opts(TlsMode::Require, None).unwrap();
        assert!(require.accept_invalid_certs());
        let prefer = ssl_opts(TlsMode::Prefer, None).unwrap();
        assert!(prefer.accept_invalid_certs());
        let ca = ssl_opts(TlsMode::VerifyCa, Some(std::path::Path::new("/ca.pem"))).unwrap();
        assert!(!ca.accept_invalid_certs() && ca.skip_domain_validation());
        assert_eq!(ca.root_certs().len(), 1);
        let full = ssl_opts(TlsMode::VerifyFull, None).unwrap();
        assert!(!full.accept_invalid_certs() && !full.skip_domain_validation());
    }

    fn server(code: u16, state: &str) -> mysql_async::Error {
        mysql_async::Error::Server(mysql_async::ServerError {
            code,
            message: "message".into(),
            state: state.into(),
        })
    }

    #[test]
    fn errors_map_to_our_kinds() {
        assert!(matches!(connect_error(server(1045, "28000")), Error::Auth(_)));
        assert!(matches!(connect_error(server(1049, "42000")), Error::Connect(_)));
        assert_eq!(query_error(server(1317, "70100")), Error::Cancelled);
        match query_error(server(1054, "42S22")) {
            Error::Query { code, .. } => assert_eq!(code.as_deref(), Some("42S22")),
            other => panic!("{other:?}"),
        }
        assert!(matches!(
            query_error(mysql_async::Error::Driver(mysql_async::DriverError::ConnectionClosed)),
            Error::ConnectionLost(_)
        ));
    }
}
```

Add `mod mysql;` to `crates/tabletist-db/src/lib.rs` (with `#[allow(dead_code)]` and the comment "reached through Connection from Task 2").

Move `is_tls_error` from `pg.rs` to `tls.rs` as `pub(crate) fn is_tls_error(..)` (same body), and in `pg.rs` call `crate::tls::is_tls_error(&error)`. Add to the tests in `tls.rs`:

```rust
    /// mysql_async builds its TLS config with rustls's process-default
    /// provider, which rustls only picks by itself when exactly one provider
    /// (ring) is compiled in. Enabling aws-lc anywhere would make MySQL TLS
    /// connections panic.
    #[test]
    fn exactly_one_crypto_provider_is_compiled_in() {
        let config = std::panic::catch_unwind(|| {
            rustls::ClientConfig::builder()
                .with_root_certificates(rustls::RootCertStore::empty())
                .with_no_client_auth()
        });
        assert!(config.is_ok(), "rustls could not choose a provider by itself");
    }
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p tabletist-db mysql`
Expected: FAIL to compile: `cannot find function type_name`, `value`, `ssl_opts`.

- [ ] **Step 3: Implement**

Put above the tests in `crates/tabletist-db/src/mysql.rs`:

```rust
use std::borrow::Cow;
use std::path::Path;

use mysql_async::consts::{ColumnFlags, ColumnType};
use mysql_async::{DriverError, IoError, SslOpts};

use crate::{Error, TlsMode, Value, ValueKind};

/// MySQL's `binary` character set: bytes, not text.
const BINARY_CHARSET: u16 = 63;
const ACCESS_DENIED: u16 = 1045;
const QUERY_INTERRUPTED: u16 = 1317;

/// A readable type name from a result column's metadata. Result columns do
/// not carry lengths or enum members; the Structure view shows the full
/// `column_type` from information_schema instead.
pub(crate) fn type_name(column_type: ColumnType, flags: ColumnFlags, charset: u16) -> Cow<'static, str> {
    let binary = charset == BINARY_CHARSET;
    let name: &'static str = match column_type {
        ColumnType::MYSQL_TYPE_TINY => "tinyint",
        ColumnType::MYSQL_TYPE_SHORT => "smallint",
        ColumnType::MYSQL_TYPE_INT24 => "mediumint",
        ColumnType::MYSQL_TYPE_LONG => "int",
        ColumnType::MYSQL_TYPE_LONGLONG => "bigint",
        ColumnType::MYSQL_TYPE_FLOAT => "float",
        ColumnType::MYSQL_TYPE_DOUBLE => "double",
        ColumnType::MYSQL_TYPE_DECIMAL | ColumnType::MYSQL_TYPE_NEWDECIMAL => "decimal",
        ColumnType::MYSQL_TYPE_YEAR => "year",
        ColumnType::MYSQL_TYPE_DATE | ColumnType::MYSQL_TYPE_NEWDATE => "date",
        ColumnType::MYSQL_TYPE_DATETIME | ColumnType::MYSQL_TYPE_DATETIME2 => "datetime",
        ColumnType::MYSQL_TYPE_TIMESTAMP | ColumnType::MYSQL_TYPE_TIMESTAMP2 => "timestamp",
        ColumnType::MYSQL_TYPE_TIME | ColumnType::MYSQL_TYPE_TIME2 => "time",
        ColumnType::MYSQL_TYPE_JSON => "json",
        ColumnType::MYSQL_TYPE_BIT => "bit",
        ColumnType::MYSQL_TYPE_GEOMETRY => "geometry",
        ColumnType::MYSQL_TYPE_ENUM => "enum",
        ColumnType::MYSQL_TYPE_SET => "set",
        ColumnType::MYSQL_TYPE_STRING if flags.contains(ColumnFlags::ENUM_FLAG) => "enum",
        ColumnType::MYSQL_TYPE_STRING if flags.contains(ColumnFlags::SET_FLAG) => "set",
        ColumnType::MYSQL_TYPE_STRING => {
            if binary {
                "binary"
            } else {
                "char"
            }
        }
        ColumnType::MYSQL_TYPE_VARCHAR | ColumnType::MYSQL_TYPE_VAR_STRING => {
            if binary {
                "varbinary"
            } else {
                "varchar"
            }
        }
        ColumnType::MYSQL_TYPE_TINY_BLOB
        | ColumnType::MYSQL_TYPE_MEDIUM_BLOB
        | ColumnType::MYSQL_TYPE_LONG_BLOB
        | ColumnType::MYSQL_TYPE_BLOB => {
            if binary {
                "blob"
            } else {
                "text"
            }
        }
        _ => "unknown",
    };
    let numeric = matches!(
        name,
        "tinyint" | "smallint" | "mediumint" | "int" | "bigint" | "float" | "double" | "decimal"
    );
    if numeric && flags.contains(ColumnFlags::UNSIGNED_FLAG) {
        Cow::Owned(format!("{name} unsigned"))
    } else {
        Cow::Borrowed(name)
    }
}

/// The kind for a name from `type_name`.
pub(crate) fn kind(type_name: &str) -> ValueKind {
    match type_name.split(' ').next().unwrap_or_default() {
        "tinyint" | "smallint" | "mediumint" | "int" | "bigint" | "float" | "double" | "decimal"
        | "year" => ValueKind::Numeric,
        "date" | "datetime" | "timestamp" | "time" => ValueKind::Temporal,
        "json" => ValueKind::Json,
        "binary" | "varbinary" | "blob" | "bit" => ValueKind::Binary,
        "char" | "varchar" | "text" | "enum" | "set" => ValueKind::Text,
        _ => ValueKind::Other,
    }
}

fn fraction(micros: u32) -> String {
    if micros == 0 {
        String::new()
    } else {
        format!(".{micros:06}")
    }
}

/// A binary-protocol value as the grid shows it. Unsigned integers too big
/// for `i64` and decimals stay text so nothing is rounded.
pub(crate) fn value(value: mysql_async::Value, type_name: &str) -> Value {
    use mysql_async::Value as My;
    match value {
        My::NULL => Value::Null,
        My::Int(number) => Value::Int(number),
        My::UInt(number) => i64::try_from(number)
            .map(Value::Int)
            .unwrap_or_else(|_| Value::Text(number.to_string().into())),
        My::Float(number) => Value::Float(f64::from(number)),
        My::Double(number) => Value::Float(number),
        My::Date(year, month, day, hour, minute, second, micros) => {
            let date = format!("{year:04}-{month:02}-{day:02}");
            if type_name == "date" {
                Value::Text(date.into())
            } else {
                Value::Text(format!("{date} {hour:02}:{minute:02}:{second:02}{}", fraction(micros)).into())
            }
        }
        My::Time(negative, days, hours, minutes, seconds, micros) => {
            let hours = days * 24 + u32::from(hours);
            let sign = if negative { "-" } else { "" };
            Value::Text(format!("{sign}{hours:02}:{minutes:02}:{seconds:02}{}", fraction(micros)).into())
        }
        My::Bytes(bytes) => match kind(type_name) {
            ValueKind::Binary => Value::Bytes(bytes.into()),
            _ => Value::Text(String::from_utf8_lossy(&bytes).into()),
        },
    }
}

/// A filter value as a statement parameter.
pub(crate) fn to_mysql(value: &Value) -> mysql_async::Value {
    use mysql_async::Value as My;
    match value {
        Value::Null => My::NULL,
        Value::Bool(flag) => My::Int(i64::from(*flag)),
        Value::Int(number) => My::Int(*number),
        Value::Float(number) => My::Double(*number),
        Value::Text(text) => My::Bytes(text.as_bytes().to_vec()),
        Value::Bytes(bytes) => My::Bytes(bytes.to_vec()),
    }
}

/// `mysql_async` TLS options for our mode (`None` means no TLS).
pub(crate) fn ssl_opts(mode: TlsMode, ca_file: Option<&Path>) -> Option<SslOpts> {
    let base = match ca_file {
        Some(path) => SslOpts::default().with_root_certs(vec![path.to_path_buf().into()]),
        None => SslOpts::default(),
    };
    match mode {
        TlsMode::Disable => None,
        TlsMode::Prefer | TlsMode::Require => Some(base.with_danger_accept_invalid_certs(true)),
        TlsMode::VerifyCa => Some(base.with_danger_skip_domain_validation(true)),
        TlsMode::VerifyFull => Some(base),
    }
}

pub(crate) fn connect_error(error: mysql_async::Error) -> Error {
    if crate::tls::is_tls_error(&error) {
        return Error::Tls(error.to_string());
    }
    match error {
        mysql_async::Error::Server(server) if server.code == ACCESS_DENIED => Error::Auth(server.message),
        mysql_async::Error::Server(server) => Error::Connect(server.message),
        mysql_async::Error::Io(IoError::Tls(tls)) => Error::Tls(tls.to_string()),
        mysql_async::Error::Driver(DriverError::NoClientSslFlagFromServer) => {
            Error::Tls("the server does not support TLS".into())
        }
        other => Error::Connect(other.to_string()),
    }
}

pub(crate) fn query_error(error: mysql_async::Error) -> Error {
    match error {
        mysql_async::Error::Server(server) if server.code == QUERY_INTERRUPTED => Error::Cancelled,
        mysql_async::Error::Server(server) => Error::Query {
            code: Some(server.state),
            message: server.message,
            detail: None,
            hint: None,
        },
        mysql_async::Error::Io(_) | mysql_async::Error::Driver(DriverError::ConnectionClosed) => {
            Error::ConnectionLost(error.to_string())
        }
        other => Error::query(other.to_string()),
    }
}
```

(`mysql_async::Error::Io(_) | ...` binds nothing, so `error` is still usable in that arm; if the compiler disagrees, bind with `ref` or format first.)

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p tabletist-db`
Expected: PASS (the new mysql and tls tests plus everything before).

Run: `cargo tree -e features -i rustls | grep aws`
Expected: no output (ring is the only provider).

- [ ] **Step 5: Commit**

```bash
git add crates Cargo.lock
git commit -m "Map MySQL types, values, TLS options and errors"
```

---

### Task 2: MySQL connect, catalog listing, and the integration-test harness

**Files:**
- Create: `compose/mysql-init.sql`, `crates/tabletist-db/fixtures/mysql.sql`, `crates/tabletist-db/tests/mysql.rs`
- Modify: `compose.yaml`, `crates/tabletist-db/src/mysql.rs`, `crates/tabletist-db/src/lib.rs`, `crates/tabletist-db/src/fixtures.rs`, `crates/tabletist-db/tests/sqlite.rs`

**Interfaces:**
- Consumes: Task 1.
- Produces: `mysql::Conn` with async `connect(spec, secrets)`, `list_schemas()`, `list_objects(schema)`, fields `opts: mysql_async::Opts` and `id: u32` (crate-visible, for cancel); `Connection::MySql(Box<mysql::Conn>)` with every method dispatching (`list_databases` returns empty; `describe`/`fetch_rows`/`count_rows` return `Unsupported` until Tasks 3 and 4); `CancelHandle` for MySQL (`KILL QUERY`); `fixtures::MYSQL_SQL`.

- [ ] **Step 1: Add the server, fixture, and failing tests**

Add to `compose.yaml` under `services:`:

```yaml
  mysql:
    image: mysql:8.4
    environment:
      MYSQL_ROOT_PASSWORD: tabletist-root
      MYSQL_DATABASE: tabletist
      MYSQL_USER: tabletist
      MYSQL_PASSWORD: tabletist
    ports:
      - "53306:3306"
    volumes:
      # A second database, so schema listing has more than one.
      - ./compose/mysql-init.sql:/docker-entrypoint-initdb.d/billing.sql:ro
    # Healthy only once the final server listens on TCP (the init server
    # does not), so `docker compose up --wait` returns when tests can run.
    healthcheck:
      test: ["CMD", "mysqladmin", "ping", "-h", "127.0.0.1", "-u", "tabletist", "-ptabletist", "--silent"]
      interval: 2s
      timeout: 5s
      retries: 90
```

and change the comment at the top of `compose.yaml` to:

```yaml
# Databases for integration tests:
#   docker compose up -d --wait postgres mysql
#   TABLETIST_TEST_PG_URL=postgres://tabletist:tabletist@localhost:55432/tabletist \
#   TABLETIST_TEST_MYSQL_URL=mysql://tabletist:tabletist@localhost:53306/tabletist \
#     cargo test --workspace
```

Create `compose/mysql-init.sql`:

```sql
CREATE DATABASE billing;
GRANT ALL PRIVILEGES ON billing.* TO 'tabletist'@'%';
```

Create `crates/tabletist-db/fixtures/mysql.sql` (statements end with `;` at the end of a line; the tests split on that):

```sql
-- The MySQL fixture. Loaded by the integration tests through a normal
-- (writable) connection; the adapter itself is read-only. Safe to rerun.
SET FOREIGN_KEY_CHECKS = 0;
DROP TABLE IF EXISTS billing.invoices;
DROP VIEW IF EXISTS active_users;
DROP TABLE IF EXISTS orders, users, events, `weird "name"`, big, digits;
SET FOREIGN_KEY_CHECKS = 1;

CREATE TABLE users (
    id INT PRIMARY KEY,
    email VARCHAR(255) NOT NULL UNIQUE COMMENT 'Login address',
    name VARCHAR(255),
    created_at DATETIME(6) NOT NULL DEFAULT '2026-01-01 00:00:00',
    active TINYINT(1) NOT NULL DEFAULT 1,
    meta JSON,
    avatar VARBINARY(16),
    score DOUBLE,
    balance DECIMAL(14, 2),
    counter BIGINT UNSIGNED,
    mood ENUM('happy', 'sad'),
    birthday DATE,
    alarm TIME
);
CREATE INDEX users_name_idx ON users (name);
INSERT INTO users VALUES
    (1, 'ada@example.com', 'Ada Lovelace', '2026-01-02 09:00:00', 1, '{"plan": "pro"}', X'89504E47', 99.5, 123456789012.34, 18446744073709551615, 'happy', '1815-12-10', '07:30:00'),
    (2, 'bob@example.com', 'Bob', '2026-01-03 10:30:00', 0, NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL),
    (3, 'zoe@example.com', 'Zoë 🚀', '2026-01-04 11:45:00.000120', 1, '{"plan": "free"}', X'00FF10', 12.25, 0, 5, 'sad', NULL, '-25:15:00'),
    (4, 'percent@example.com', '50% off_now', '2026-01-05 12:00:00', 1, NULL, NULL, 0, NULL, NULL, NULL, NULL, NULL),
    (5, 'quote@example.com', 'O''Brien C:\\temp', '2026-01-06 13:15:00', 1, NULL, NULL, -1, NULL, NULL, NULL, NULL, NULL);

CREATE TABLE orders (
    id INT PRIMARY KEY,
    user_id INT NOT NULL,
    total DECIMAL(10, 2) NOT NULL,
    note TEXT,
    CONSTRAINT orders_user_id_fkey FOREIGN KEY (user_id) REFERENCES users (id) ON DELETE CASCADE,
    INDEX orders_user_total_idx (user_id, total)
);
INSERT INTO orders VALUES (1, 1, 19.99, 'first'), (2, 1, 5.00, NULL), (3, 3, 120.50, 'bulk');

CREATE TABLE events (kind TEXT, payload TEXT);
INSERT INTO events VALUES ('login', 'ada'), ('logout', 'ada'), ('login', 'zoe');

CREATE TABLE `weird "name"` (`col with space` TEXT, `select` INT);
INSERT INTO `weird "name"` VALUES ('quoted', 1);

CREATE TABLE digits (d INT);
INSERT INTO digits VALUES (0), (1), (2), (3), (4), (5), (6), (7), (8), (9);
CREATE TABLE big (id INT PRIMARY KEY, label VARCHAR(32) NOT NULL, bucket INT NOT NULL);
INSERT INTO big SELECT n, CONCAT('row ', n), n % 10 FROM (SELECT a.d + 10 * b.d + 100 * c.d + 1000 * e.d + 10000 * f.d + 1 AS n FROM digits a, digits b, digits c, digits e, digits f) AS numbers;
DROP TABLE digits;
ANALYZE TABLE big;

CREATE VIEW active_users AS SELECT id, email FROM users WHERE active = 1;

CREATE TABLE billing.invoices (
    id INT PRIMARY KEY,
    user_id INT NOT NULL,
    amount DECIMAL(10, 2) NOT NULL,
    CONSTRAINT invoices_user_fkey FOREIGN KEY (user_id) REFERENCES tabletist.users (id) ON UPDATE RESTRICT ON DELETE CASCADE
);
```

Add to `crates/tabletist-db/src/fixtures.rs`:

```rust
/// The MySQL fixture script (one statement per `;` at the end of a line).
pub const MYSQL_SQL: &str = include_str!("../fixtures/mysql.sql");
```

Create `crates/tabletist-db/tests/mysql.rs`:

```rust
//! MySQL through the public `Connection` API. Needs a server:
//! `docker compose up -d --wait mysql` and TABLETIST_TEST_MYSQL_URL (see
//! compose.yaml). Without it every test prints "skipped" and passes.

#![allow(clippy::unwrap_used)]

use std::time::Duration;

use mysql_async::prelude::Queryable;
use tabletist_db::{ConnectSpec, Connection, Driver, Error, ObjectKind, Secrets, TlsMode};
use tokio::sync::OnceCell;

static FIXTURE: OnceCell<()> = OnceCell::const_new();

fn url() -> Option<String> {
    std::env::var("TABLETIST_TEST_MYSQL_URL").ok().filter(|url| !url.trim().is_empty())
}

fn spec() -> Option<(ConnectSpec, Secrets)> {
    let (mut spec, secrets) = ConnectSpec::from_url(&url()?).unwrap();
    spec.tls = TlsMode::Disable;
    Some((spec, secrets))
}

async fn load_fixture() {
    FIXTURE
        .get_or_init(|| async {
            let opts = mysql_async::Opts::from_url(&format!("{}?prefer_socket=false", url().unwrap())).unwrap();
            // The server may still be starting when the first test runs.
            let deadline = std::time::Instant::now() + Duration::from_secs(90);
            let mut conn = loop {
                match mysql_async::Conn::new(opts.clone()).await {
                    Ok(conn) => break conn,
                    Err(error) if std::time::Instant::now() < deadline => {
                        eprintln!("waiting for MySQL: {error}");
                        tokio::time::sleep(Duration::from_secs(1)).await;
                    }
                    Err(error) => panic!("MySQL never answered: {error}"),
                }
            };
            for statement in tabletist_db::fixtures::MYSQL_SQL.split(";\n") {
                let statement: String = statement
                    .lines()
                    .filter(|line| !line.trim_start().starts_with("--"))
                    .collect::<Vec<_>>()
                    .join("\n");
                if !statement.trim().is_empty() {
                    conn.query_drop(statement).await.unwrap();
                }
            }
            conn.disconnect().await.unwrap();
        })
        .await;
}

async fn connect() -> Option<Connection> {
    let Some((spec, secrets)) = spec() else {
        eprintln!("skipped: TABLETIST_TEST_MYSQL_URL is not set");
        return None;
    };
    load_fixture().await;
    Some(Connection::connect(&spec, &secrets).await.unwrap())
}

#[tokio::test]
async fn connections_report_mysql_and_list_databases_as_schemas() {
    let Some(connection) = connect().await else { return };
    assert_eq!(connection.driver(), Driver::MySql);
    assert!(connection.list_databases().await.unwrap().is_empty());
    let schemas = connection.list_schemas().await.unwrap();
    for schema in ["tabletist", "billing", "information_schema"] {
        assert!(schemas.contains(&schema.to_owned()), "{schema} in {schemas:?}");
    }
}

#[tokio::test]
async fn objects_have_kinds_and_estimates() {
    let Some(connection) = connect().await else { return };
    let objects = connection.list_objects("tabletist").await.unwrap();
    let find = |name: &str| objects.iter().find(|object| object.name == name).unwrap();
    assert_eq!(find("users").kind, ObjectKind::Table);
    assert_eq!(find("active_users").kind, ObjectKind::View);
    assert_eq!(find("weird \"name\"").kind, ObjectKind::Table);
    // InnoDB estimates are approximate.
    let estimate = find("big").estimated_rows.unwrap();
    assert!((50_000..=150_000).contains(&estimate), "{estimate}");
    let names: Vec<&str> = objects.iter().map(|object| object.name.as_str()).collect();
    let mut sorted = names.clone();
    sorted.sort_unstable();
    assert_eq!(names, sorted);
}

#[tokio::test]
async fn a_wrong_password_is_an_auth_error() {
    let Some((spec, _)) = spec() else { return };
    let secrets = Secrets { password: Some("definitely wrong".into()), ..Secrets::default() };
    let result = Connection::connect(&spec, &secrets).await;
    assert!(matches!(result, Err(Error::Auth(_))), "{:?}", result.err());
}

#[tokio::test]
async fn a_missing_database_is_a_connect_error() {
    let Some((mut spec, secrets)) = spec() else { return };
    spec.database = "no_such_database".into();
    let result = Connection::connect(&spec, &secrets).await;
    assert!(matches!(result, Err(Error::Connect(_))), "{:?}", result.err());
}

#[tokio::test]
async fn tls_modes_behave_like_libpq() {
    let Some((mut spec, secrets)) = spec() else { return };
    load_fixture().await;
    // MySQL 8 generates a self-signed certificate at first start.
    for mode in [TlsMode::Prefer, TlsMode::Require] {
        spec.tls = mode;
        assert!(Connection::connect(&spec, &secrets).await.is_ok(), "{mode:?}");
    }
    spec.tls = TlsMode::VerifyFull;
    let result = Connection::connect(&spec, &secrets).await;
    assert!(matches!(result, Err(Error::Tls(_))), "{:?}", result.err());
}

#[tokio::test]
async fn a_user_is_required() {
    let spec = ConnectSpec {
        driver: Driver::MySql,
        user: String::new(),
        port: 3306,
        ..ConnectSpec::default()
    };
    assert!(matches!(
        Connection::connect(&spec, &Secrets::default()).await,
        Err(Error::InvalidSpec(_))
    ));
}
```

Add `mysql_async` to nothing else: integration tests can use the crate's regular dependencies.

In `crates/tabletist-db/tests/sqlite.rs`, delete the test `mysql_is_not_supported_yet` (MySQL is supported now).

- [ ] **Step 2: Run the tests to verify they fail**

Ask for the server if it is not running: `docker compose up -d --wait mysql` (this may need `sudo`). Check: `docker compose ps mysql` shows `healthy`.

Run: `TABLETIST_TEST_MYSQL_URL=mysql://tabletist:tabletist@localhost:53306/tabletist cargo test -p tabletist-db --test mysql`
Expected: FAIL: `a_user_is_required` and the connect tests get `Unsupported("MySQL connections")`.

- [ ] **Step 3: Implement connect and listing**

Add to `crates/tabletist-db/src/mysql.rs` (above the helpers; extend the `use` lines to `use std::time::Duration;`, `use mysql_async::prelude::Queryable;`, `use mysql_async::{Opts, OptsBuilder, Params};`, and `use crate::{ConnectSpec, ObjectInfo, ObjectKind, Result, Secrets};`):

```rust
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);

pub struct Conn {
    pub(crate) conn: tokio::sync::Mutex<mysql_async::Conn>,
    /// For KILL QUERY from a second connection.
    pub(crate) opts: Opts,
    /// This session's thread id on the server.
    pub(crate) id: u32,
}

impl Conn {
    pub async fn connect(spec: &ConnectSpec, secrets: &Secrets) -> Result<Self> {
        if spec.user.trim().is_empty() {
            return Err(Error::InvalidSpec("enter a user name".into()));
        }
        if let Some(path) = &spec.ca_file
            && !path.is_file()
        {
            return Err(Error::Tls(format!("could not read {}", path.display())));
        }
        let builder = OptsBuilder::default()
            .ip_or_hostname(spec.host.clone())
            .tcp_port(spec.port)
            .user(Some(spec.user.clone()))
            .pass(secrets.password.clone())
            .db_name((!spec.database.is_empty()).then(|| spec.database.clone()))
            // Never switch a "localhost" connection to the Unix socket.
            .prefer_socket(false);
        let with_tls: Opts = builder.clone().ssl_opts(ssl_opts(spec.tls, spec.ca_file.as_deref())).into();
        let (mut conn, opts) = match tokio::time::timeout(CONNECT_TIMEOUT, mysql_async::Conn::new(with_tls.clone())).await {
            Err(_) => return Err(Error::Timeout),
            Ok(Ok(conn)) => (conn, with_tls),
            // `prefer`: a server without TLS gets a plain connection.
            Ok(Err(mysql_async::Error::Driver(DriverError::NoClientSslFlagFromServer)))
                if spec.tls == TlsMode::Prefer =>
            {
                let plain: Opts = builder.ssl_opts(None::<mysql_async::SslOpts>).into();
                (mysql_async::Conn::new(plain.clone()).await.map_err(connect_error)?, plain)
            }
            Ok(Err(error)) => return Err(connect_error(error)),
        };
        // Fixed statements: safe to send through the text protocol.
        conn.query_drop("SET SESSION TRANSACTION READ ONLY").await.map_err(query_error)?;
        conn.query_drop("SET SESSION sql_mode = REPLACE(@@SESSION.sql_mode, 'NO_BACKSLASH_ESCAPES', '')")
            .await
            .map_err(query_error)?;
        let id = conn.id();
        Ok(Self {
            conn: tokio::sync::Mutex::new(conn),
            opts,
            id,
        })
    }

    /// A catalog query: fixed SQL, prepared, typed results.
    async fn catalog<T>(&self, sql: &str, params: impl Into<Params> + Send) -> Result<Vec<T>>
    where
        T: mysql_async::prelude::FromRow + Send + 'static,
    {
        let mut conn = self.conn.lock().await;
        conn.exec(sql, params).await.map_err(query_error)
    }

    pub async fn list_schemas(&self) -> Result<Vec<String>> {
        self.catalog("SELECT schema_name FROM information_schema.schemata ORDER BY schema_name", Params::Empty)
            .await
    }

    pub async fn list_objects(&self, schema: &str) -> Result<Vec<ObjectInfo>> {
        let rows: Vec<(String, String, Option<u64>)> = self
            .catalog(
                "SELECT table_name, table_type, table_rows FROM information_schema.tables \
                 WHERE table_schema = ? ORDER BY table_name",
                (schema,),
            )
            .await?;
        Ok(rows
            .into_iter()
            .map(|(name, kind, estimate)| {
                let view = kind.contains("VIEW");
                ObjectInfo {
                    name,
                    kind: if view { ObjectKind::View } else { ObjectKind::Table },
                    estimated_rows: if view { None } else { estimate },
                }
            })
            .collect())
    }
}
```

In `crates/tabletist-db/src/lib.rs`:
- change `#[allow(dead_code)] mod mysql;` to `mod mysql;`;
- add the variant `MySql(Box<mysql::Conn>)` (boxed like PostgreSQL) to `Connection`;
- in `connect`, `Driver::MySql => Ok(Self::MySql(Box::new(mysql::Conn::connect(spec, secrets).await?))),`;
- add `Self::MySql(..)` arms: `driver` → `Driver::MySql`; `dialect` → `Dialect::MySql`; `list_databases` → `Ok(Vec::new())`; `list_schemas`/`list_objects` → the new methods; `describe`/`fetch_rows`/`count_rows` → `Err(Error::Unsupported("MySQL browsing arrives in the next task"))`; `close` → `drop(conn); Ok(())`;
- extend the cancel handle:

```rust
    MySql {
        opts: mysql_async::Opts,
        id: u32,
    },
```

with `Self::MySql(conn) => CancelHandle(CancelInner::MySql { opts: conn.opts.clone(), id: conn.id })` and

```rust
            CancelInner::MySql { opts, id } => {
                use mysql_async::prelude::Queryable;
                let mut conn = mysql_async::Conn::new(opts.clone())
                    .await
                    .map_err(|error| Error::Connect(format!("could not cancel: {error}")))?;
                let result = conn.query_drop(format!("KILL QUERY {id}")).await;
                let _ = conn.disconnect().await;
                result.map_err(|error| Error::Connect(format!("could not cancel: {error}")))
            }
```

(`KILL QUERY` takes a number formatted by us; nothing user-typed reaches it.)

- [ ] **Step 4: Run the tests to verify they pass**

Run: `TABLETIST_TEST_MYSQL_URL=mysql://tabletist:tabletist@localhost:53306/tabletist cargo test -p tabletist-db --test mysql`
Expected: PASS (6 tests).

Run: `cargo test --locked --workspace --all-targets`
Expected: PASS (MySQL tests skip without the variable).

- [ ] **Step 5: Commit**

```bash
git add crates compose.yaml compose Cargo.lock
git commit -m "Connect to MySQL and list databases and objects"
```

---

### Task 3: MySQL structure

**Files:**
- Modify: `crates/tabletist-db/src/mysql.rs`, `crates/tabletist-db/src/lib.rs`, `crates/tabletist-db/tests/mysql.rs`

**Interfaces:**
- Consumes: Task 2's `Conn::catalog`.
- Produces: `mysql::Conn::describe(&self, object: &ObjectRef) -> Result<Structure>`, `mysql::Conn::primary_key(&self, object: &ObjectRef) -> Result<Vec<String>>`.

- [ ] **Step 1: Write the failing tests**

Add to `crates/tabletist-db/tests/mysql.rs`:

```rust
use tabletist_db::ObjectRef;

#[tokio::test]
async fn users_structure_has_full_types_comments_key_and_indexes() {
    let Some(connection) = connect().await else { return };
    let structure = connection.describe(&ObjectRef::new("tabletist", "users")).await.unwrap();
    let column = |name: &str| structure.columns.iter().find(|c| c.name == name).unwrap();
    assert_eq!(structure.columns[0].name, "id");
    assert_eq!(column("id").type_name, "int");
    assert!(!column("id").nullable);
    assert!(column("name").nullable);
    assert_eq!(column("email").type_name, "varchar(255)");
    assert_eq!(column("email").comment.as_deref(), Some("Login address"));
    assert_eq!(column("balance").type_name, "decimal(14,2)");
    assert_eq!(column("counter").type_name, "bigint unsigned");
    assert_eq!(column("mood").type_name, "enum('happy','sad')");
    assert!(column("created_at").default.as_deref().unwrap().starts_with("2026-01-01 00:00:00"));
    assert_eq!(structure.primary_key, vec!["id".to_owned()]);
    let primary = structure.indexes.iter().find(|i| i.name == "PRIMARY").unwrap();
    assert!(primary.primary && primary.unique);
    assert_eq!(primary.method.as_deref(), Some("btree"));
    let email = structure.indexes.iter().find(|i| i.name == "email").unwrap();
    assert!(email.unique && !email.primary);
    let name = structure.indexes.iter().find(|i| i.name == "users_name_idx").unwrap();
    assert!(!name.unique);
}

#[tokio::test]
async fn foreign_keys_name_their_target_and_actions() {
    let Some(connection) = connect().await else { return };
    let orders = connection.describe(&ObjectRef::new("tabletist", "orders")).await.unwrap();
    let key = &orders.foreign_keys[0];
    assert_eq!(key.name.as_deref(), Some("orders_user_id_fkey"));
    assert_eq!(key.columns, vec!["user_id".to_owned()]);
    assert_eq!((key.ref_schema.as_str(), key.ref_table.as_str()), ("tabletist", "users"));
    assert_eq!(key.ref_columns, vec!["id".to_owned()]);
    assert_eq!(key.on_delete, "CASCADE");
    let composite = orders.indexes.iter().find(|i| i.name == "orders_user_total_idx").unwrap();
    assert_eq!(composite.columns, vec!["user_id".to_owned(), "total".to_owned()]);
    let invoices = connection.describe(&ObjectRef::new("billing", "invoices")).await.unwrap();
    assert_eq!(invoices.foreign_keys[0].ref_schema, "tabletist");
    assert_eq!(invoices.foreign_keys[0].on_update, "RESTRICT");
}

#[tokio::test]
async fn views_and_quoted_names_describe_and_missing_objects_fail() {
    let Some(connection) = connect().await else { return };
    let view = connection.describe(&ObjectRef::new("tabletist", "active_users")).await.unwrap();
    assert_eq!(view.columns.len(), 2);
    assert!(view.primary_key.is_empty() && view.indexes.is_empty());
    let weird = connection.describe(&ObjectRef::new("tabletist", "weird \"name\"")).await.unwrap();
    assert_eq!(weird.columns[1].name, "select");
    assert!(matches!(
        connection.describe(&ObjectRef::new("tabletist", "nope")).await,
        Err(Error::Query { .. })
    ));
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `TABLETIST_TEST_MYSQL_URL=mysql://tabletist:tabletist@localhost:53306/tabletist cargo test -p tabletist-db --test mysql`
Expected: FAIL: the new tests get `Unsupported`.

- [ ] **Step 3: Implement**

Add to `impl Conn` in `mysql.rs` (extend `use crate::{...}` with `ColumnInfo, ForeignKeyInfo, IndexInfo, ObjectRef, Structure`):

```rust
    pub async fn primary_key(&self, object: &ObjectRef) -> Result<Vec<String>> {
        self.catalog(
            "SELECT column_name FROM information_schema.key_column_usage \
             WHERE table_schema = ? AND table_name = ? AND constraint_name = 'PRIMARY' \
             ORDER BY ordinal_position",
            (&object.schema, &object.name),
        )
        .await
    }

    pub async fn describe(&self, object: &ObjectRef) -> Result<Structure> {
        let at = (&object.schema, &object.name);
        let columns: Vec<(String, String, String, Option<String>, String)> = self
            .catalog(
                "SELECT column_name, column_type, is_nullable, column_default, column_comment \
                 FROM information_schema.columns \
                 WHERE table_schema = ? AND table_name = ? ORDER BY ordinal_position",
                at,
            )
            .await?;
        if columns.is_empty() {
            return Err(Error::query(format!("no such table or view: {}", object.name)));
        }
        let columns = columns
            .into_iter()
            .map(|(name, type_name, nullable, default, comment)| ColumnInfo {
                name,
                type_name,
                nullable: nullable == "YES",
                default,
                comment: (!comment.is_empty()).then_some(comment),
            })
            .collect();
        let index_rows: Vec<(String, i64, String, String)> = self
            .catalog(
                "SELECT index_name, non_unique, COALESCE(column_name, '<expression>'), index_type \
                 FROM information_schema.statistics \
                 WHERE table_schema = ? AND table_name = ? ORDER BY index_name, seq_in_index",
                at,
            )
            .await?;
        let mut indexes: Vec<IndexInfo> = Vec::new();
        for (name, non_unique, column, method) in index_rows {
            if indexes.last().is_none_or(|index| index.name != name) {
                indexes.push(IndexInfo {
                    primary: name == "PRIMARY",
                    unique: non_unique == 0,
                    method: Some(method.to_lowercase()),
                    columns: Vec::new(),
                    name,
                });
            }
            if let Some(index) = indexes.last_mut() {
                index.columns.push(column);
            }
        }
        let key_rows: Vec<(String, String, String, String, String, String, String)> = self
            .catalog(
                "SELECT k.constraint_name, k.column_name, k.referenced_table_schema, \
                        k.referenced_table_name, k.referenced_column_name, r.update_rule, r.delete_rule \
                 FROM information_schema.key_column_usage k \
                 JOIN information_schema.referential_constraints r \
                   ON r.constraint_schema = k.constraint_schema AND r.constraint_name = k.constraint_name \
                  AND r.table_name = k.table_name \
                 WHERE k.table_schema = ? AND k.table_name = ? AND k.referenced_table_name IS NOT NULL \
                 ORDER BY k.constraint_name, k.ordinal_position",
                at,
            )
            .await?;
        let mut foreign_keys: Vec<ForeignKeyInfo> = Vec::new();
        for (name, column, ref_schema, ref_table, ref_column, on_update, on_delete) in key_rows {
            if foreign_keys.last().is_none_or(|key| key.name.as_deref() != Some(name.as_str())) {
                foreign_keys.push(ForeignKeyInfo {
                    name: Some(name),
                    columns: Vec::new(),
                    ref_schema,
                    ref_table,
                    ref_columns: Vec::new(),
                    on_update,
                    on_delete,
                });
            }
            if let Some(key) = foreign_keys.last_mut() {
                key.columns.push(column);
                key.ref_columns.push(ref_column);
            }
        }
        Ok(Structure {
            columns,
            primary_key: self.primary_key(object).await?,
            indexes,
            foreign_keys,
        })
    }
```

In `lib.rs`, route `Connection::describe` for `Self::MySql(conn)` to `conn.describe(object).await`.

- [ ] **Step 4: Run the tests to verify they pass**

Run: `TABLETIST_TEST_MYSQL_URL=mysql://tabletist:tabletist@localhost:53306/tabletist cargo test -p tabletist-db --test mysql`
Expected: PASS (9 tests).

- [ ] **Step 5: Commit**

```bash
git add crates
git commit -m "Describe MySQL tables and views"
```

---

### Task 4: MySQL rows, counts and cancel

**Files:**
- Modify: `crates/tabletist-db/src/mysql.rs`, `crates/tabletist-db/src/lib.rs`, `crates/tabletist-db/tests/mysql.rs`

**Interfaces:**
- Consumes: Tasks 1 to 3.
- Produces: `mysql::Conn::fetch_rows(&self, query: &RowQuery) -> Result<RowPage>`, `mysql::Conn::count_rows(&self, query: &RowQuery) -> Result<u64>`.

- [ ] **Step 1: Write the failing tests**

Add to `crates/tabletist-db/tests/mysql.rs`:

```rust
use tabletist_db::{Filter, FilterOp, RowPage, RowQuery, Sort, SortDir, Value, ValueKind};

fn users(limit: u32) -> RowQuery {
    RowQuery::new(ObjectRef::new("tabletist", "users"), limit)
}

fn ids(page: &RowPage) -> Vec<i64> {
    page.rows
        .iter()
        .map(|row| match row[0] {
            Value::Int(id) => id,
            ref other => panic!("id is {other:?}"),
        })
        .collect()
}

fn filtered(column: &str, op: FilterOp, value: &str) -> RowQuery {
    let mut query = users(50);
    query.filters = vec![Filter { column: column.into(), op, value: value.into() }];
    query
}

#[tokio::test]
async fn pages_have_typed_values_and_kinds() {
    let Some(connection) = connect().await else { return };
    let page = connection.fetch_rows(&users(3)).await.unwrap();
    assert_eq!(ids(&page), vec![1, 2, 3]);
    assert!(page.has_more && page.ordered_by_key);
    let kind = |name: &str| page.columns.iter().find(|c| c.name == name).unwrap().kind;
    assert_eq!(kind("meta"), ValueKind::Json);
    assert_eq!(kind("avatar"), ValueKind::Binary);
    assert_eq!(kind("created_at"), ValueKind::Temporal);
    assert_eq!(kind("balance"), ValueKind::Numeric);
    let ada = &page.rows[0];
    assert_eq!(ada[3], Value::Text("2026-01-02 09:00:00".into()));
    assert_eq!(ada[4], Value::Int(1));
    assert_eq!(ada[6], Value::Bytes(vec![0x89, 0x50, 0x4e, 0x47].into()));
    assert_eq!(ada[7], Value::Float(99.5));
    assert_eq!(ada[8], Value::Text("123456789012.34".into()));
    assert_eq!(ada[9], Value::Text("18446744073709551615".into()));
    assert_eq!(ada[10], Value::Text("happy".into()));
    assert_eq!(ada[11], Value::Text("1815-12-10".into()));
    assert_eq!(ada[12], Value::Text("07:30:00".into()));
    let zoe = &page.rows[2];
    assert_eq!(zoe[2], Value::Text("Zoë 🚀".into()));
    assert_eq!(zoe[3], Value::Text("2026-01-04 11:45:00.000120".into()));
    assert_eq!(zoe[12], Value::Text("-25:15:00".into()));
    assert_eq!(page.rows[1][5], Value::Null);
}

#[tokio::test]
async fn sorting_paging_and_counting_work() {
    let Some(connection) = connect().await else { return };
    let mut query = users(5);
    query.sort = vec![Sort { column: "email".into(), dir: SortDir::Desc }];
    assert_eq!(ids(&connection.fetch_rows(&query).await.unwrap()), vec![3, 5, 4, 2, 1]);
    let mut deep = RowQuery::new(ObjectRef::new("tabletist", "big"), 300);
    deep.offset = 99_900;
    let page = connection.fetch_rows(&deep).await.unwrap();
    assert_eq!(page.rows.len(), 100);
    assert!(!page.has_more);
    let all = RowQuery::new(ObjectRef::new("tabletist", "big"), 1);
    assert_eq!(connection.count_rows(&all).await.unwrap(), 100_000);
    let events = connection
        .fetch_rows(&RowQuery::new(ObjectRef::new("tabletist", "events"), 50))
        .await
        .unwrap();
    assert!(!events.ordered_by_key);
}

#[tokio::test]
async fn filters_work() {
    let Some(connection) = connect().await else { return };
    let rows = |query: RowQuery| {
        let connection = &connection;
        async move { ids(&connection.fetch_rows(&query).await.unwrap()) }
    };
    assert_eq!(rows(filtered("id", FilterOp::Ge, "4")).await, vec![4, 5]);
    assert_eq!(rows(filtered("id", FilterOp::In, "1, 3")).await, vec![1, 3]);
    assert_eq!(rows(filtered("meta", FilterOp::IsNull, "")).await, vec![2, 4, 5]);
    assert_eq!(rows(filtered("name", FilterOp::Contains, "ZO")).await, vec![3]);
    assert_eq!(rows(filtered("mood", FilterOp::Eq, "sad")).await, vec![3]);
    assert_eq!(rows(filtered("counter", FilterOp::Eq, "18446744073709551615")).await, vec![1]);
    assert_eq!(connection.count_rows(&filtered("id", FilterOp::Lt, "3")).await.unwrap(), 2);
}

#[tokio::test]
async fn quotes_backslashes_and_wildcards_are_just_text() {
    let Some(connection) = connect().await else { return };
    let page = connection.fetch_rows(&filtered("name", FilterOp::Eq, r"O'Brien C:\temp")).await.unwrap();
    assert_eq!(ids(&page), vec![5]);
    let page = connection.fetch_rows(&filtered("name", FilterOp::Contains, "50%")).await.unwrap();
    assert_eq!(ids(&page), vec![4]);
    let page = connection.fetch_rows(&filtered("name", FilterOp::Contains, "_now")).await.unwrap();
    assert_eq!(ids(&page), vec![4]);
    let page = connection.fetch_rows(&filtered("name", FilterOp::Contains, r"C:\temp")).await.unwrap();
    assert_eq!(ids(&page), vec![5]);
    let page = connection.fetch_rows(&filtered("name", FilterOp::Eq, "x' OR '1'='1")).await.unwrap();
    assert!(page.rows.is_empty());
}

#[tokio::test]
async fn a_raw_where_cannot_write_or_chain_statements() {
    let Some(connection) = connect().await else { return };
    for raw in ["1 = 1; DELETE FROM users", "1=1) ; COMMIT; DELETE FROM users; SELECT (1"] {
        let mut query = users(50);
        query.raw_where = Some(raw.into());
        assert!(connection.fetch_rows(&query).await.is_err(), "{raw}");
        assert!(connection.count_rows(&query).await.is_err(), "{raw}");
    }
    assert_eq!(connection.count_rows(&users(1)).await.unwrap(), 5);
}

#[tokio::test]
async fn a_bad_raw_where_is_a_query_error_and_the_session_survives() {
    let Some(connection) = connect().await else { return };
    let mut query = users(50);
    query.raw_where = Some("no_such_column = 1".into());
    match connection.fetch_rows(&query).await {
        Err(Error::Query { code, .. }) => assert_eq!(code.as_deref(), Some("42S22")),
        other => panic!("{other:?}"),
    }
    assert!(connection.fetch_rows(&users(1)).await.is_ok());
}

#[tokio::test]
async fn quoted_names_and_views_browse() {
    let Some(connection) = connect().await else { return };
    let mut query = RowQuery::new(ObjectRef::new("tabletist", "weird \"name\""), 5);
    query.sort = vec![Sort { column: "select".into(), dir: SortDir::Asc }];
    let page = connection.fetch_rows(&query).await.unwrap();
    assert_eq!(page.columns[0].name, "col with space");
    let view = connection
        .fetch_rows(&RowQuery::new(ObjectRef::new("tabletist", "active_users"), 5))
        .await
        .unwrap();
    assert_eq!(view.rows.len(), 4);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_running_query_can_be_cancelled() {
    let Some(connection) = connect().await else { return };
    let cancel = connection.cancel_handle();
    let connection = std::sync::Arc::new(connection);
    let running = {
        let connection = std::sync::Arc::clone(&connection);
        tokio::spawn(async move {
            let mut query = RowQuery::new(ObjectRef::new("tabletist", "big"), 5);
            // Ten billion row pairs: far longer than the test waits.
            query.raw_where = Some("(SELECT COUNT(*) FROM big a, big b) > 0".into());
            connection.count_rows(&query).await
        })
    };
    let deadline = std::time::Instant::now() + Duration::from_secs(20);
    while !running.is_finished() {
        assert!(std::time::Instant::now() < deadline, "cancel must stop the query");
        cancel.cancel().await.unwrap();
        tokio::time::sleep(Duration::from_millis(300)).await;
    }
    assert_eq!(running.await.unwrap(), Err(Error::Cancelled));
    assert!(connection.fetch_rows(&users(1)).await.is_ok());
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `TABLETIST_TEST_MYSQL_URL=mysql://tabletist:tabletist@localhost:53306/tabletist cargo test -p tabletist-db --test mysql`
Expected: FAIL: the new tests get `Unsupported`.

- [ ] **Step 3: Implement**

Add to `impl Conn` in `mysql.rs` (extend the imports with `std::time::Instant`, `mysql_async::TxOpts`, and `crate::{ColumnMeta, Dialect, RowPage, RowQuery}`):

```rust
    /// One page, through a prepared statement inside a read-only
    /// transaction. Reading stops after `limit + 1` rows.
    pub async fn fetch_rows(&self, query: &RowQuery) -> Result<RowPage> {
        let key = self.primary_key(&query.object).await?;
        let sql = Dialect::MySql.select_rows(query, &key);
        let limit = query.limit as usize;
        let mut conn = self.conn.lock().await;
        let started = Instant::now();
        let mut transaction = conn.start_transaction(read_only()).await.map_err(query_error)?;
        let mut result = transaction
            .exec_iter(sql.text.as_str(), params(&sql.params))
            .await
            .map_err(query_error)?;
        let columns: Vec<ColumnMeta> = result
            .columns_ref()
            .iter()
            .map(|column| {
                let type_name = type_name(column.column_type(), column.flags(), column.character_set());
                ColumnMeta {
                    name: column.name_str().into_owned(),
                    kind: kind(&type_name),
                    type_name: type_name.into_owned(),
                }
            })
            .collect();
        let mut rows = Vec::new();
        while let Some(row) = result.next().await.map_err(query_error)? {
            rows.push(
                row.unwrap()
                    .into_iter()
                    .zip(&columns)
                    .map(|(cell, column)| value(cell, &column.type_name))
                    .collect(),
            );
            if rows.len() > limit {
                break;
            }
        }
        result.drop_result().await.map_err(query_error)?;
        transaction.rollback().await.map_err(query_error)?;
        let has_more = rows.len() > limit;
        rows.truncate(limit);
        Ok(RowPage {
            columns,
            rows,
            has_more,
            ordered_by_key: !key.is_empty(),
            elapsed: started.elapsed(),
        })
    }

    pub async fn count_rows(&self, query: &RowQuery) -> Result<u64> {
        let sql = Dialect::MySql.count_rows(query);
        let mut conn = self.conn.lock().await;
        let mut transaction = conn.start_transaction(read_only()).await.map_err(query_error)?;
        let count: Option<u64> = transaction
            .exec_first(sql.text.as_str(), params(&sql.params))
            .await
            .map_err(query_error)?;
        transaction.rollback().await.map_err(query_error)?;
        count.ok_or_else(|| Error::query("the count returned no number"))
    }
```

and the free functions:

```rust
fn read_only() -> TxOpts {
    let mut options = TxOpts::default();
    options.with_readonly(Some(true));
    options
}

fn params(values: &[Value]) -> Params {
    if values.is_empty() {
        Params::Empty
    } else {
        Params::Positional(values.iter().map(to_mysql).collect())
    }
}
```

(`row.unwrap()` is `mysql_async::Row::unwrap`, which returns the row's values; it is not `Option::unwrap`.) If a statement fails inside the transaction, dropping the transaction rolls it back, so the session stays usable (`a_bad_raw_where_is_a_query_error_and_the_session_survives` checks this).

In `lib.rs`, route `fetch_rows` and `count_rows` for `Self::MySql(conn)` to the new methods.

- [ ] **Step 4: Run the tests to verify they pass**

Run: `TABLETIST_TEST_MYSQL_URL=mysql://tabletist:tabletist@localhost:53306/tabletist cargo test -p tabletist-db --test mysql`
Expected: PASS (17 tests).

Run: `cargo test --locked --workspace --all-targets && cargo clippy --locked --workspace --all-targets -- -D warnings`
Expected: all pass.

- [ ] **Step 5: Commit**

```bash
git add crates
git commit -m "Browse MySQL rows with counts and cancel"
```

---

### Task 5: MySQL in the app

**Files:**
- Modify: `src/model.rs`, `src/app.rs`, `src/ui/connect_dialog.rs`, `src/ui/mod.rs`

**Interfaces:**
- Consumes: `Connection` for MySQL (through the backend, unchanged).
- Produces: `ConnectionForm::to_spec` accepts MySQL; `apply_url` fills MySQL URLs; the tree expands the connection's database first; the dialog's MySQL button is enabled.

- [ ] **Step 1: Write the failing tests**

In `src/app.rs` tests, replace the MySQL case of `a_pasted_url_fills_the_form_or_explains_why_not` (the lines that set `form(&mut app).url = "mysql://h/db".into();` and assert the message contains "MySQL") with:

```rust
        form(&mut app).url = "mysql://root@db.example.com/shop?ssl-mode=REQUIRED".into();
        app.apply(Action::ApplyUrl);
        assert_eq!(form(&mut app).driver, Driver::MySql);
        assert_eq!(form(&mut app).port, "3306");
        assert_eq!(form(&mut app).database, "shop");
        assert_eq!(form(&mut app).tls, tabletist_db::TlsMode::Require);
        assert!(form(&mut app).message.is_none());
```

Add to the `tests` module in `src/app.rs`:

```rust
    #[test]
    fn a_mysql_form_saves_with_the_default_port() {
        let (mut app, _dir) = app();
        app.apply(Action::NewConnection);
        app.apply(Action::SetDriver(Driver::MySql));
        let form = form(&mut app);
        assert_eq!(form.port, "3306");
        form.name = "Shop".into();
        form.host = "db.example.com".into();
        form.user = "root".into();
        app.apply(Action::SaveConnection { connect: false });
        let saved = &app.connections.connections[0];
        assert_eq!(saved.spec.driver, Driver::MySql);
        assert_eq!(saved.spec.port, 3306);
        assert_eq!(saved.password, PasswordMode::Keyring);
    }

    #[test]
    fn the_connections_database_is_expanded_first() {
        let (mut app, _dir) = app();
        let (spec, _) = ConnectSpec::from_url("mysql://root@db/shop").unwrap();
        let saved = SavedConnection {
            id: ConnectionId::new(),
            name: "Shop".into(),
            color: ColorTag::None,
            password: PasswordMode::None,
            spec,
        };
        let conn = saved.id.clone();
        app.connections.upsert(saved);
        let tab = app.active_tab_id();
        app.apply(Action::Connect { tab, conn });
        let Some(Command::Connect { session, request, .. }) = app.backend.sent.last() else { panic!() };
        let (session, request) = (*session, *request);
        app.apply(Action::Backend(Event::Connected { session, request, driver: Driver::MySql }));
        let Some(Command::ListSchemas { request, .. }) =
            app.backend.sent.iter().rev().find(|c| matches!(c, Command::ListSchemas { .. }))
        else {
            panic!("ListSchemas")
        };
        let request = *request;
        app.apply(Action::Backend(Event::Schemas {
            session,
            request,
            result: Ok(vec!["analytics".into(), "information_schema".into(), "shop".into()]),
        }));
        let nodes = &app.workspace(tab).unwrap().tree.nodes;
        assert!(nodes["shop"].expanded);
        assert!(!nodes.get("analytics").is_some_and(|node| node.expanded));
    }
```

Add to the tests in `src/ui/mod.rs`:

```rust
    #[test]
    fn choosing_mysql_shows_the_server_fields() {
        let mut harness = Harness::new();
        harness.press(Key::N, Modifiers::COMMAND);
        harness.click("MySQL");
        for label in ["Host", "Port", "User", "Password", "Database", "TLS"] {
            assert!(harness.has(label), "{label}");
        }
        assert_eq!(
            match &harness.app.dialog {
                Some(crate::model::Dialog::Connection(form)) => form.port.clone(),
                _ => panic!("no dialog"),
            },
            "3306"
        );
    }
```

In `crates/tabletist-db`, nothing changes in this task.

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test --lib`
Expected: FAIL: MySQL still "arrives in a later version"; `shop` is not expanded; the MySQL button is disabled.

- [ ] **Step 3: Implement**

In `src/model.rs` `ConnectionForm::to_spec`, delete the `Driver::MySql => Err(..)` arm and change `Driver::Postgres => {` to `Driver::Postgres | Driver::MySql => {`, with `driver: self.driver,` instead of `driver: Driver::Postgres,` in the returned spec.

In `src/app.rs` `apply_url`, delete the `Driver::MySql => { form.message = ... }` arm, change `Driver::Postgres => {` to `Driver::Postgres | Driver::MySql => {`, and set `form.driver = spec.driver;` instead of `form.driver = Driver::Postgres;`.

In `src/app.rs`, in the `Event::Schemas` arm, prefer the connection's database when choosing the schema to expand. Replace the `let default = ...` expression with:

```rust
                let database = workspace.spec.database.clone();
                let default = workspace.tree.schemas.value.as_ref().and_then(|schemas| {
                    schemas
                        .iter()
                        .find(|schema| !database.is_empty() && **schema == database)
                        .or_else(|| schemas.iter().find(|schema| *schema == "public" || *schema == "main"))
                        .or(if schemas.len() == 1 { schemas.first() } else { None })
                        .cloned()
                });
```

(For PostgreSQL the database is not a schema name, so this falls through to `public` as before.)

In `src/ui/connect_dialog.rs`, change `(Driver::MySql, false),` to `(Driver::MySql, true),`. The disabled-hover branch stays for future drivers; if clippy reports it unreachable, simplify the loop to `ui.add(button)`.

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test --locked --workspace --all-targets && cargo clippy --locked --workspace --all-targets -- -D warnings`
Expected: all pass.

- [ ] **Step 5: Commit**

```bash
git add src
git commit -m "Connect to MySQL from the dialog and pasted URLs"
```

---

### Task 6: CI, docs, and a real-server check

**Files:**
- Modify: `.github/workflows/ci.yml`, `AGENTS.md`

- [ ] **Step 1: Add the CI job**

Add to `.github/workflows/ci.yml` under `jobs:`:

```yaml
  mysql:
    name: mysql integration
    runs-on: ubuntu-latest
    timeout-minutes: 30
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2
      - name: Start MySQL
        run: docker compose up -d --wait mysql
      - run: cargo test --locked -p tabletist-db --test mysql
        env:
          TABLETIST_TEST_MYSQL_URL: mysql://tabletist:tabletist@localhost:53306/tabletist
```

In `AGENTS.md`, replace the PostgreSQL-only integration paragraph with:

```markdown
Database integration tests need servers: `docker compose up -d --wait postgres mysql`, then
`TABLETIST_TEST_PG_URL=postgres://tabletist:tabletist@localhost:55432/tabletist TABLETIST_TEST_MYSQL_URL=mysql://tabletist:tabletist@localhost:53306/tabletist cargo test --workspace`.
Without the variables those tests print "skipped" and pass; CI runs both suites on Linux.
The native keyring test is `#[ignore]`d; run it by hand on a desktop session.
```

- [ ] **Step 2: Check against real servers**

Run: `docker compose up -d --wait postgres mysql` (may need `sudo`).
Run: `TABLETIST_TEST_PG_URL=postgres://tabletist:tabletist@localhost:55432/tabletist TABLETIST_TEST_MYSQL_URL=mysql://tabletist:tabletist@localhost:53306/tabletist cargo test --locked --workspace --all-targets`
Expected: PASS, including 17 MySQL and 18 PostgreSQL tests. Run the MySQL suite three more times: `cargo test -q -p tabletist-db --test mysql` (with the variable) passes each time.

By hand (`cargo run`): New connection, MySQL, host `localhost`, port `53306`, user `tabletist`, password `tabletist`, database `tabletist`, Save & Connect. Expected: the tree shows `tabletist` expanded and `billing` folded; `users` shows `18446744073709551615` in `counter`, `-25:15:00` in `alarm` for Zoë, and binary avatars in the row panel as hex; the top bar has no database switcher.

- [ ] **Step 3: Full checks and commit**

Run: `cargo fmt --all --check && cargo clippy --locked --workspace --all-targets -- -D warnings && cargo test --locked --workspace --all-targets && RUSTDOCFLAGS='-D warnings' cargo doc --locked --workspace --no-deps`
Expected: all pass.

```bash
git add .github AGENTS.md
git commit -m "Run the MySQL suite in CI"
```

---

## Done when

- A saved MySQL connection opens in a tab and browses like PostgreSQL and SQLite: tree of databases, grid, paging, sort, filters, row panel, Structure view, cancel.
- Every TLS mode works against MySQL 8.4's self-signed certificate as it does against PostgreSQL.
- Nothing typed into a filter or raw WHERE can write or run a second statement.
- CI runs the MySQL suite on Linux.

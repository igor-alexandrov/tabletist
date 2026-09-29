# Batch 4: PostgreSQL + TLS + Keyring Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Browse PostgreSQL databases: a read-only PostgreSQL adapter (catalog, structure, rows, counts, cancel), TLS in every libpq mode on rustls, passwords kept in the OS keyring (or asked for each time), connection-dialog fields for PostgreSQL, and a database switcher in the top bar.

**Architecture:** `tabletist-db` gains `pg.rs` and `tls.rs`; `Connection` gains a `Postgres` variant. Rows are read with the **simple-query protocol**, which sends every type as text (so enums, arrays, ranges and extension types all display), after a `prepare` of the same SQL that yields column types and rejects anything but a single statement. Filter values therefore become quoted literals for PostgreSQL (`standard_conforming_strings = on` makes `'` the only character to escape). Every row and count query runs inside `BEGIN READ ONLY`, on top of `default_transaction_read_only = on`. Passwords live in the OS keyring through `keyring-core`, called on a dedicated thread (as spotifast does); the app asks the backend to load or store them, so the UI thread never touches the keyring.

**Tech Stack:** tokio-postgres 0.7, tokio-postgres-rustls 0.14, rustls 0.23 (ring provider), rustls-native-certs 0.8, rustls-pki-types 1, keyring-core 1 with zbus-secret-service-keyring-store (Linux), apple-native-keyring-store (macOS), windows-native-keyring-store (Windows); PostgreSQL 17 in Docker for integration tests.

**Spec:** `docs/superpowers/specs/2026-09-27-tabletist-design.md` (sections 4.2 to 4.8, 5.3, 5.4, 5.12)

## Global Constraints

- Everything from earlier batches' Global Constraints still applies; verify with `cargo test --locked --workspace --all-targets` and `cargo clippy --locked --workspace --all-targets -- -D warnings`. Run `cargo fmt --all` before every commit.
- PostgreSQL sessions: `default_transaction_read_only = on` and `standard_conforming_strings = on` right after connect; every row and count query inside `build_transaction().read_only(true)`; every user-influenced SQL (`select_rows`, `count_rows`) is `prepare`d before it runs, so a second statement is refused.
- PostgreSQL filter values are quoted literals (`'` doubled). Identifiers stay quoted with `"`. SQLite keeps bound parameters.
- TLS: `rustls` with the `ring` provider only (no aws-lc, no OpenSSL). `prefer` and `require` do not check certificates (libpq semantics); `verify-ca` checks the chain but not the host name; `verify-full` checks both. A `ca_file` replaces the system roots.
- Secrets: never in `connections.json`, logs, `Debug` output or error messages. Keyring account names are `connection/<connection id>/password` under service `dev.tabletist.Tabletist`. Keyring calls run on the `tabletist-keyring` thread, time out after 20 s, and are never made for connections that do not use the keyring.
- PostgreSQL integration tests read `TABLETIST_TEST_PG_URL`; without it they print "skipped" and pass. `compose.yaml` provides the server at `postgres://tabletist:tabletist@localhost:55432/tabletist`.
- Never use em dashes. One topic per commit on `main`.

## Review Focus

1. **A filter value containing `'` or `\`** (`O'Brien`, `C:\temp`) against PostgreSQL: matches literally, never a syntax error, never injection. Test: Task 1 `postgres_values_become_quoted_literals` and Task 5 `quotes_and_backslashes_in_filter_values_are_just_text`.
2. **A raw WHERE that tries to leave read-only mode** (`set_config('default_transaction_read_only','off',false)`, `1=1); COMMIT; DELETE ...`, `nextval(...)`): fails or changes nothing; later queries stay read-only. Test: Task 5 `a_raw_where_cannot_write_or_leave_read_only_mode`.
3. **A keyring that is locked, unavailable, or has no entry**: connecting falls back to the password prompt with a message, never spins or fails silently. Test: Task 7 `a_missing_keyring_password_opens_the_prompt` and `a_keyring_error_opens_the_prompt_with_its_message`.
4. **Switching to a database that cannot be opened** (no permission, dropped meanwhile): the tab shows the error, keeps the database list, and switching back works. Test: Task 7 `a_failed_database_switch_can_switch_back`.
5. **Editing a keyring connection without retyping the password** keeps the saved password; switching it to "Ask every time" or deleting the connection removes it; SQLite saves never touch the keyring. Test: Task 7 `saving_without_retyping_keeps_the_keyring_password`, `changing_to_ask_deletes_the_saved_password`, `deleting_a_connection_deletes_its_password`, `sqlite_saves_never_touch_the_keyring`.

---

## File Structure

```
compose.yaml                                  PostgreSQL 17 with TLS for integration tests
.github/workflows/ci.yml                      + postgres integration job, keyring round trip on macOS/Windows
AGENTS.md                                     + how to run integration tests
crates/tabletist-db/Cargo.toml                + tokio-postgres, tokio-postgres-rustls, rustls, rustls-native-certs, rustls-pki-types
crates/tabletist-db/src/value.rs              + ValueKind::from_pg_type, value_from_pg_text
crates/tabletist-db/src/dialect.rs            PostgreSQL literals instead of $n
crates/tabletist-db/src/tls.rs                rustls configs per TlsMode, SslMode mapping
crates/tabletist-db/src/pg.rs                 the PostgreSQL adapter
crates/tabletist-db/src/lib.rs                Connection::Postgres, CancelHandle for PostgreSQL
crates/tabletist-db/src/fixtures.rs           + POSTGRES_SQL
crates/tabletist-db/fixtures/postgres.sql     PostgreSQL fixture
crates/tabletist-db/tests/postgres.rs         env-gated integration tests
Cargo.toml                                    + keyring-core and the per-OS stores
src/secrets.rs                                keyring thread, SecretStore, MemoryStore, NativeStore, SecretString
src/backend.rs                                + LoadSecret, StoreSecret, ListDatabases; Backend::start_with
src/connections.rs                            + PasswordMode on SavedConnection
src/model.rs                                  ConnectionForm for PostgreSQL, PasswordPrompt, Workspace secrets/databases, new Actions
src/app.rs                                    connect/test/save/delete with secrets, prompt, database switching
src/ui/connect_dialog.rs                      driver switch, PostgreSQL fields, TLS, password mode
src/ui/password_prompt.rs                     the password prompt
src/ui/workspace.rs                           database switcher and TLS label in the top bar
docs/superpowers/specs/2026-09-27-tabletist-design.md   filter values: literals for PostgreSQL
```

---

### Task 1: PostgreSQL values and quoted literals in the dialect

**Files:**
- Modify: `crates/tabletist-db/src/value.rs`, `crates/tabletist-db/src/dialect.rs`, `crates/tabletist-db/src/lib.rs`, `docs/superpowers/specs/2026-09-27-tabletist-design.md`

**Interfaces:**
- Consumes: `Value`, `ValueKind`, `Dialect`.
- Produces: `ValueKind::from_pg_type(name: &str) -> ValueKind`; `tabletist_db::value_from_pg_text(type_name: &str, text: &str) -> Value` (re-exported from `value`); `tabletist_db::quote_literal(text: &str) -> String`; `Dialect::Postgres.select_rows/count_rows` now return `Sql { params: vec![] }` with literals inline.

- [ ] **Step 1: Write the failing tests**

Add to the `tests` module in `crates/tabletist-db/src/value.rs`:

```rust
    #[test]
    fn postgres_type_names_map_to_kinds() {
        let cases = [
            ("bool", ValueKind::Bool),
            ("int4", ValueKind::Numeric),
            ("int8", ValueKind::Numeric),
            ("numeric", ValueKind::Numeric),
            ("float8", ValueKind::Numeric),
            ("jsonb", ValueKind::Json),
            ("json", ValueKind::Json),
            ("timestamptz", ValueKind::Temporal),
            ("interval", ValueKind::Temporal),
            ("bytea", ValueKind::Binary),
            ("text", ValueKind::Text),
            ("varchar", ValueKind::Text),
            ("uuid", ValueKind::Text),
            ("_text", ValueKind::Other),
            ("mood", ValueKind::Other),
            ("geometry", ValueKind::Other),
        ];
        for (name, kind) in cases {
            assert_eq!(ValueKind::from_pg_type(name), kind, "{name}");
        }
    }

    #[test]
    fn postgres_text_values_become_typed_values() {
        assert_eq!(value_from_pg_text("bool", "t"), Value::Bool(true));
        assert_eq!(value_from_pg_text("bool", "f"), Value::Bool(false));
        assert_eq!(value_from_pg_text("int8", "-42"), Value::Int(-42));
        assert_eq!(value_from_pg_text("float8", "99.5"), Value::Float(99.5));
        assert!(matches!(value_from_pg_text("float8", "NaN"), Value::Float(f) if f.is_nan()));
        assert_eq!(value_from_pg_text("float8", "-Infinity"), Value::Float(f64::NEG_INFINITY));
        assert_eq!(
            value_from_pg_text("bytea", "\\x89504e47"),
            Value::Bytes(vec![0x89, 0x50, 0x4e, 0x47].into())
        );
        // Anything that does not parse stays as the server's text.
        assert_eq!(value_from_pg_text("bytea", "\\xzz"), Value::Text("\\xzz".into()));
        assert_eq!(
            value_from_pg_text("numeric", "12345678901234567890.12"),
            Value::Text("12345678901234567890.12".into())
        );
        assert_eq!(value_from_pg_text("_text", "{a,b}"), Value::Text("{a,b}".into()));
    }
```

In `crates/tabletist-db/src/dialect.rs` tests, replace the PostgreSQL parts of four tests and add one:

```rust
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
            r#"SELECT * FROM "public"."users" WHERE "age" >= '18' AND "name" <> 'bob' LIMIT 301 OFFSET 0"#
        );
        assert!(pg.params.is_empty());
        let my = Dialect::MySql.select_rows(&q, &[]);
        assert!(my.text.contains("WHERE `age` >= ? AND `name` <> ?"), "{}", my.text);
        assert_eq!(my.params, vec![text("18"), text("bob")]);
    }

    #[test]
    fn postgres_values_become_quoted_literals() {
        assert_eq!(quote_literal("O'Brien"), "'O''Brien'");
        assert_eq!(quote_literal(r"C:\temp"), r"'C:\temp'");
        assert_eq!(quote_literal(""), "''");
        let mut q = query();
        q.filters = vec![Filter { column: "name".into(), op: FilterOp::Eq, value: "x' OR '1'='1".into() }];
        let pg = Dialect::Postgres.select_rows(&q, &[]);
        assert!(pg.text.contains(r#""name" = 'x'' OR ''1''=''1'"#), "{}", pg.text);
    }

    #[test]
    fn in_filters_split_on_commas_and_an_empty_list_matches_nothing() {
        let mut q = query();
        q.filters = vec![Filter { column: "id".into(), op: FilterOp::In, value: " 1, 2 ,,3 ".into() }];
        let sql = Dialect::Postgres.select_rows(&q, &[]);
        assert!(sql.text.contains(r#""id" IN ('1', '2', '3')"#), "{}", sql.text);
        let lite = Dialect::Sqlite.select_rows(&q, &[]);
        assert!(lite.text.contains(r#""id" IN (?, ?, ?)"#), "{}", lite.text);
        assert_eq!(lite.params, vec![text("1"), text("2"), text("3")]);
        q.filters[0].value = " , ".into();
        let sql = Dialect::Postgres.select_rows(&q, &[]);
        assert!(sql.text.contains("WHERE 1 = 0"), "{}", sql.text);
    }

    #[test]
    fn contains_casts_to_text_and_is_case_insensitive_per_dialect() {
        let mut q = query();
        q.filters = vec![Filter { column: "id".into(), op: FilterOp::Contains, value: "5%".into() }];
        let pg = Dialect::Postgres.select_rows(&q, &[]);
        assert!(pg.text.contains(r#"CAST("id" AS TEXT) ILIKE '%5\%%'"#), "{}", pg.text);
        let lite = Dialect::Sqlite.select_rows(&q, &[]);
        assert!(lite.text.contains(r#"CAST("id" AS TEXT) LIKE ? ESCAPE '\'"#), "{}", lite.text);
        assert_eq!(lite.params, vec![text(r"%5\%%")]);
        let my = Dialect::MySql.select_rows(&q, &[]);
        assert!(my.text.contains("CAST(`id` AS CHAR) LIKE ?"), "{}", my.text);
        q.filters[0].op = FilterOp::StartsWith;
        assert!(Dialect::Postgres.select_rows(&q, &[]).text.contains(r"ILIKE '5\%%'"));
    }

    #[test]
    fn raw_where_is_wrapped_in_parentheses_after_filters() {
        let mut q = query();
        q.filters = vec![Filter { column: "a".into(), op: FilterOp::Eq, value: "1".into() }];
        q.raw_where = Some("b = 2 OR c = 3".into());
        let sql = Dialect::Postgres.select_rows(&q, &[]);
        assert!(sql.text.contains("WHERE \"a\" = '1' AND (\nb = 2 OR c = 3\n)"), "{}", sql.text);
        q.raw_where = Some("   ".into());
        assert!(!Dialect::Postgres.select_rows(&q, &[]).text.contains("AND ("));
    }
```

(Replace the existing tests of the same names; the SQLite count test `counts_use_the_same_filters_without_order_or_paging` stays as it is.)

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p tabletist-db`
Expected: FAIL to compile: `cannot find function value_from_pg_text`, `cannot find function quote_literal`, `from_pg_type` not found.

- [ ] **Step 3: Implement**

Add to `crates/tabletist-db/src/value.rs` (after `impl ValueKind`'s existing methods, inside the impl):

```rust
    /// The kind for a PostgreSQL type name (`pg_type.typname`). Arrays
    /// (`_text`), enums, ranges and extension types are `Other`.
    pub fn from_pg_type(name: &str) -> Self {
        match name {
            "bool" => Self::Bool,
            "int2" | "int4" | "int8" | "oid" | "float4" | "float8" | "numeric" | "money" => {
                Self::Numeric
            }
            "json" | "jsonb" => Self::Json,
            "date" | "time" | "timetz" | "timestamp" | "timestamptz" | "interval" => Self::Temporal,
            "bytea" => Self::Binary,
            "text" | "varchar" | "bpchar" | "char" | "name" | "uuid" | "citext" | "xml" | "inet"
            | "cidr" | "macaddr" => Self::Text,
            _ => Self::Other,
        }
    }
```

and after the impl:

```rust
/// A PostgreSQL value as the simple-query protocol sends it (text), typed by
/// its column's type name. What does not parse stays as the server's text,
/// and exact decimals stay text so nothing is rounded.
pub fn value_from_pg_text(type_name: &str, text: &str) -> Value {
    let as_text = || Value::Text(text.into());
    match type_name {
        "bool" => match text {
            "t" => Value::Bool(true),
            "f" => Value::Bool(false),
            _ => as_text(),
        },
        "int2" | "int4" | "int8" | "oid" => text.parse().map(Value::Int).unwrap_or_else(|_| as_text()),
        "float4" | "float8" => text.parse().map(Value::Float).unwrap_or_else(|_| as_text()),
        "bytea" => decode_bytea(text).map(|bytes| Value::Bytes(bytes.into())).unwrap_or_else(as_text),
        _ => as_text(),
    }
}

/// `\x0a0b...` (PostgreSQL's hex bytea output) to bytes.
fn decode_bytea(text: &str) -> Option<Vec<u8>> {
    let hex = text.strip_prefix("\\x")?;
    if hex.len() % 2 != 0 {
        return None;
    }
    (0..hex.len())
        .step_by(2)
        .map(|index| u8::from_str_radix(hex.get(index..index + 2)?, 16).ok())
        .collect()
}
```

In `crates/tabletist-db/src/lib.rs`, change the value re-export to `pub use value::{ColumnMeta, Value, ValueKind, value_from_pg_text};` and the dialect one to `pub use dialect::{Dialect, Sql, escape_like, quote_literal};`.

In `crates/tabletist-db/src/dialect.rs`, replace `placeholder` and `bind`:

```rust
    /// MySQL and SQLite bind parameters with `?`.
    fn placeholder(self) -> &'static str {
        "?"
    }

    /// A filter value in SQL. PostgreSQL rows are read through the
    /// simple-query protocol, which has no parameters, so its values become
    /// quoted literals; sessions set `standard_conforming_strings = on`, so
    /// `'` is the only character that needs escaping. The others bind.
    fn bind(self, params: &mut Vec<Value>, value: String) -> String {
        match self {
            Self::Postgres => quote_literal(&value),
            Self::MySql | Self::Sqlite => {
                params.push(Value::Text(value.into()));
                self.placeholder().to_owned()
            }
        }
    }
```

and add the free function next to `escape_like`:

```rust
/// A SQL string literal with `'` doubled (standard-conforming strings).
pub fn quote_literal(text: &str) -> String {
    format!("'{}'", text.replace('\'', "''"))
}
```

In the spec, section 4.5, replace "Filter values are always bound parameters." with "Filter values are bound parameters for SQLite and MySQL, and quoted literals (with `'` doubled, under `standard_conforming_strings = on`) for PostgreSQL, whose rows are read through the simple-query protocol." and in 4.2 add after the PostgreSQL bullet: "The SQL is `prepare`d first (to learn column types and refuse a second statement), then run with the simple-query protocol inside a read-only transaction."

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p tabletist-db`
Expected: PASS (all unit and SQLite integration tests).

- [ ] **Step 5: Commit**

```bash
git add crates docs/superpowers/specs
git commit -m "Map PostgreSQL text values and quote PostgreSQL filter literals"
```

---

### Task 2: TLS configurations

**Files:**
- Create: `crates/tabletist-db/src/tls.rs`
- Modify: `crates/tabletist-db/Cargo.toml`, `crates/tabletist-db/src/lib.rs`

**Interfaces:**
- Consumes: `TlsMode`, `Error`.
- Produces: `tls::client_config(mode: TlsMode, ca_file: Option<&Path>) -> Result<rustls::ClientConfig>` (crate-private), `tls::ssl_mode(mode: TlsMode) -> tokio_postgres::config::SslMode` (crate-private), `tls::NoVerify`, `tls::IgnoreName` (crate-private verifiers).

- [ ] **Step 1: Add dependencies and write failing tests**

Add to `crates/tabletist-db/Cargo.toml` `[dependencies]`:

```toml
# PostgreSQL, async. Rows are read through its simple-query protocol.
tokio-postgres = "0.7"
# TLS for tokio-postgres on rustls (no OpenSSL to build or ship).
tokio-postgres-rustls = "0.14"
# ring rather than aws-lc: no cmake/NASM needed to build on Windows.
rustls = { version = "0.23", default-features = false, features = ["ring", "std", "tls12", "logging"] }
# The operating system's trusted roots for verify-ca and verify-full.
rustls-native-certs = "0.8"
# Reading a CA file (PEM) the user chose.
rustls-pki-types = { version = "1", features = ["std"] }
```

Create `crates/tabletist-db/src/tls.rs` with only tests:

```rust
//! TLS for database connections on rustls, with libpq's `sslmode` meanings:
//! `prefer` and `require` encrypt without checking the certificate,
//! `verify-ca` checks the chain, `verify-full` checks the chain and the host.

#[cfg(test)]
mod tests {
    use super::*;
    use rustls::pki_types::{CertificateDer, ServerName, UnixTime};

    #[test]
    fn every_mode_builds_a_config() {
        for mode in [TlsMode::Disable, TlsMode::Prefer, TlsMode::Require] {
            assert!(client_config(mode, None).is_ok(), "{mode:?}");
        }
    }

    #[test]
    fn a_missing_or_empty_ca_file_is_a_tls_error() {
        let dir = tempfile::tempdir().unwrap();
        let missing = dir.path().join("missing.pem");
        assert!(matches!(client_config(TlsMode::VerifyFull, Some(&missing)), Err(Error::Tls(_))));
        let empty = dir.path().join("empty.pem");
        std::fs::write(&empty, "no certificates here").unwrap();
        assert!(matches!(client_config(TlsMode::VerifyCa, Some(&empty)), Err(Error::Tls(_))));
    }

    #[test]
    fn modes_map_to_sslmode() {
        use tokio_postgres::config::SslMode;
        assert!(matches!(ssl_mode(TlsMode::Disable), SslMode::Disable));
        assert!(matches!(ssl_mode(TlsMode::Prefer), SslMode::Prefer));
        for mode in [TlsMode::Require, TlsMode::VerifyCa, TlsMode::VerifyFull] {
            assert!(matches!(ssl_mode(mode), SslMode::Require), "{mode:?}");
        }
    }

    /// A verifier that fails every certificate with a fixed error.
    #[derive(Debug)]
    struct Failing(rustls::Error);

    impl ServerCertVerifier for Failing {
        fn verify_server_cert(
            &self,
            _: &CertificateDer<'_>,
            _: &[CertificateDer<'_>],
            _: &ServerName<'_>,
            _: &[u8],
            _: UnixTime,
        ) -> std::result::Result<ServerCertVerified, rustls::Error> {
            Err(self.0.clone())
        }
        fn verify_tls12_signature(
            &self,
            _: &[u8],
            _: &CertificateDer<'_>,
            _: &DigitallySignedStruct,
        ) -> std::result::Result<HandshakeSignatureValid, rustls::Error> {
            Err(self.0.clone())
        }
        fn verify_tls13_signature(
            &self,
            _: &[u8],
            _: &CertificateDer<'_>,
            _: &DigitallySignedStruct,
        ) -> std::result::Result<HandshakeSignatureValid, rustls::Error> {
            Err(self.0.clone())
        }
        fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
            Vec::new()
        }
    }

    fn verify(verifier: &dyn ServerCertVerifier) -> bool {
        let name = ServerName::try_from("db.example.com").unwrap();
        verifier
            .verify_server_cert(&CertificateDer::from(vec![1, 2, 3]), &[], &name, &[], UnixTime::now())
            .is_ok()
    }

    #[test]
    fn verify_ca_ignores_only_a_name_mismatch() {
        let name = IgnoreName(Arc::new(Failing(rustls::Error::InvalidCertificate(
            CertificateError::NotValidForName,
        ))));
        assert!(verify(&name));
        let issuer = IgnoreName(Arc::new(Failing(rustls::Error::InvalidCertificate(
            CertificateError::UnknownIssuer,
        ))));
        assert!(!verify(&issuer));
    }

    #[test]
    fn no_verify_accepts_any_certificate() {
        assert!(verify(&NoVerify(provider())));
    }
}
```

Add `mod tls;` to `crates/tabletist-db/src/lib.rs` (after `mod spec;`).

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p tabletist-db tls`
Expected: FAIL to compile: `cannot find function client_config`.

- [ ] **Step 3: Implement**

Put above the tests in `crates/tabletist-db/src/tls.rs`:

```rust
use std::path::Path;
use std::sync::Arc;

use rustls::client::WebPkiServerVerifier;
use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::crypto::CryptoProvider;
use rustls::pki_types::pem::PemObject;
use rustls::pki_types::{CertificateDer, ServerName, UnixTime};
use rustls::{CertificateError, DigitallySignedStruct, SignatureScheme};

use crate::{Error, Result, TlsMode};

fn provider() -> Arc<CryptoProvider> {
    Arc::new(rustls::crypto::ring::default_provider())
}

fn tls_error(error: impl std::fmt::Display) -> Error {
    Error::Tls(error.to_string())
}

/// The rustls configuration for `mode`. `Disable` still gets one (the
/// connector needs it) but it is never used.
pub(crate) fn client_config(mode: TlsMode, ca_file: Option<&Path>) -> Result<rustls::ClientConfig> {
    let provider = provider();
    let builder = rustls::ClientConfig::builder_with_provider(Arc::clone(&provider))
        .with_safe_default_protocol_versions()
        .map_err(tls_error)?;
    let config = match mode {
        TlsMode::Disable | TlsMode::Prefer | TlsMode::Require => builder
            .dangerous()
            .with_custom_certificate_verifier(Arc::new(NoVerify(provider)))
            .with_no_client_auth(),
        TlsMode::VerifyCa => {
            let verifier = WebPkiServerVerifier::builder_with_provider(roots(ca_file)?, provider)
                .build()
                .map_err(tls_error)?;
            builder
                .dangerous()
                .with_custom_certificate_verifier(Arc::new(IgnoreName(verifier)))
                .with_no_client_auth()
        }
        TlsMode::VerifyFull => builder
            .with_root_certificates(roots(ca_file)?)
            .with_no_client_auth(),
    };
    Ok(config)
}

/// The CA file's certificates, or the operating system's roots.
fn roots(ca_file: Option<&Path>) -> Result<Arc<rustls::RootCertStore>> {
    let mut roots = rustls::RootCertStore::empty();
    match ca_file {
        Some(path) => {
            let certificates = CertificateDer::pem_file_iter(path)
                .map_err(|error| Error::Tls(format!("could not read {}: {error}", path.display())))?;
            for certificate in certificates {
                let certificate = certificate.map_err(tls_error)?;
                roots.add(certificate).map_err(tls_error)?;
            }
            if roots.is_empty() {
                return Err(Error::Tls(format!("{} holds no certificates", path.display())));
            }
        }
        None => {
            for certificate in rustls_native_certs::load_native_certs().certs {
                let _ = roots.add(certificate);
            }
            if roots.is_empty() {
                return Err(Error::Tls("no trusted certificates found on this system".into()));
            }
        }
    }
    Ok(Arc::new(roots))
}

/// libpq's `sslmode` for our mode. Certificate checks happen in rustls, so
/// every verifying mode is simply "require TLS" to tokio-postgres.
pub(crate) fn ssl_mode(mode: TlsMode) -> tokio_postgres::config::SslMode {
    use tokio_postgres::config::SslMode;
    match mode {
        TlsMode::Disable => SslMode::Disable,
        TlsMode::Prefer => SslMode::Prefer,
        TlsMode::Require | TlsMode::VerifyCa | TlsMode::VerifyFull => SslMode::Require,
    }
}

/// Accepts any certificate but still checks handshake signatures:
/// `prefer` and `require` in libpq do not verify the server.
#[derive(Debug)]
pub(crate) struct NoVerify(Arc<CryptoProvider>);

impl ServerCertVerifier for NoVerify {
    fn verify_server_cert(
        &self,
        _: &CertificateDer<'_>,
        _: &[CertificateDer<'_>],
        _: &ServerName<'_>,
        _: &[u8],
        _: UnixTime,
    ) -> std::result::Result<ServerCertVerified, rustls::Error> {
        Ok(ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        message: &[u8],
        certificate: &CertificateDer<'_>,
        signature: &DigitallySignedStruct,
    ) -> std::result::Result<HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls12_signature(
            message,
            certificate,
            signature,
            &self.0.signature_verification_algorithms,
        )
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        certificate: &CertificateDer<'_>,
        signature: &DigitallySignedStruct,
    ) -> std::result::Result<HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls13_signature(
            message,
            certificate,
            signature,
            &self.0.signature_verification_algorithms,
        )
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        self.0.signature_verification_algorithms.supported_schemes()
    }
}

/// Checks the certificate chain but not the host name (`verify-ca`).
#[derive(Debug)]
pub(crate) struct IgnoreName(pub(crate) Arc<dyn ServerCertVerifier>);

impl ServerCertVerifier for IgnoreName {
    fn verify_server_cert(
        &self,
        end_entity: &CertificateDer<'_>,
        intermediates: &[CertificateDer<'_>],
        server_name: &ServerName<'_>,
        ocsp: &[u8],
        now: UnixTime,
    ) -> std::result::Result<ServerCertVerified, rustls::Error> {
        match self.0.verify_server_cert(end_entity, intermediates, server_name, ocsp, now) {
            Err(rustls::Error::InvalidCertificate(
                CertificateError::NotValidForName | CertificateError::NotValidForNameContext { .. },
            )) => Ok(ServerCertVerified::assertion()),
            other => other,
        }
    }

    fn verify_tls12_signature(
        &self,
        message: &[u8],
        certificate: &CertificateDer<'_>,
        signature: &DigitallySignedStruct,
    ) -> std::result::Result<HandshakeSignatureValid, rustls::Error> {
        self.0.verify_tls12_signature(message, certificate, signature)
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        certificate: &CertificateDer<'_>,
        signature: &DigitallySignedStruct,
    ) -> std::result::Result<HandshakeSignatureValid, rustls::Error> {
        self.0.verify_tls13_signature(message, certificate, signature)
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        self.0.supported_verify_schemes()
    }
}
```

Until Task 3 uses them, mark the module `#[allow(dead_code)] mod tls;` in `lib.rs` with a comment "used by pg.rs from Task 3".

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p tabletist-db tls`
Expected: PASS (5 tests). `every_mode_builds_a_config` does not cover `VerifyCa`/`VerifyFull` without a CA file because a CI machine may have no system roots; `a_missing_or_empty_ca_file_is_a_tls_error` covers their error path.

- [ ] **Step 5: Commit**

```bash
git add crates Cargo.lock
git commit -m "Add rustls configurations for every TLS mode"
```

---

### Task 3: PostgreSQL connect, catalog listing, and the integration-test harness

**Files:**
- Create: `crates/tabletist-db/src/pg.rs`, `crates/tabletist-db/fixtures/postgres.sql`, `crates/tabletist-db/tests/postgres.rs`, `compose.yaml`
- Modify: `crates/tabletist-db/src/lib.rs`, `crates/tabletist-db/src/fixtures.rs`

**Interfaces:**
- Consumes: `tls::{client_config, ssl_mode}`, `ConnectSpec`, `Secrets`, `Error`, `ObjectInfo`, `ObjectKind`.
- Produces: `pg::Conn` with async `connect(spec, secrets)`, `list_databases()`, `list_schemas()`, `list_objects(schema)`; `pg::query_error`, `pg::connect_error` (crate-private); `Connection::Postgres(pg::Conn)` with every `Connection` method dispatching to it (fetch/count/describe return `Err(Error::Unsupported(..))` until Tasks 4 and 5); `CancelHandle` for PostgreSQL; `fixtures::POSTGRES_SQL`.

- [ ] **Step 1: Add the fixture, compose file, and failing integration tests**

Create `compose.yaml`:

```yaml
# Databases for integration tests. `docker compose up -d postgres`, then:
#   TABLETIST_TEST_PG_URL=postgres://tabletist:tabletist@localhost:55432/tabletist \
#     cargo test -p tabletist-db --test postgres
services:
  postgres:
    image: postgres:17
    environment:
      POSTGRES_USER: tabletist
      POSTGRES_PASSWORD: tabletist
      POSTGRES_DB: tabletist
    ports:
      - "55432:5432"
    # TLS with the image's self-signed certificate, so every sslmode can be tested.
    command:
      - -c
      - ssl=on
      - -c
      - ssl_cert_file=/etc/ssl/certs/ssl-cert-snakeoil.pem
      - -c
      - ssl_key_file=/etc/ssl/private/ssl-cert-snakeoil.key
```

Create `crates/tabletist-db/fixtures/postgres.sql`:

```sql
-- The PostgreSQL fixture. Loaded by the integration tests through an admin
-- connection (the adapter is read-only). Safe to run more than once.

DROP SCHEMA IF EXISTS billing CASCADE;
DROP MATERIALIZED VIEW IF EXISTS public.user_counts;
DROP VIEW IF EXISTS public.active_users;
DROP TABLE IF EXISTS public.orders, public.users, public.events, public."weird ""name""", public.big CASCADE;
DROP SEQUENCE IF EXISTS public.tick;
DROP TYPE IF EXISTS public.mood;

CREATE TYPE mood AS ENUM ('happy', 'sad');
CREATE SEQUENCE tick;

CREATE TABLE users (
    id integer PRIMARY KEY,
    email text NOT NULL UNIQUE,
    name text,
    created_at timestamptz NOT NULL DEFAULT '2026-01-01 00:00:00+00',
    active boolean NOT NULL DEFAULT true,
    meta jsonb,
    avatar bytea,
    score double precision,
    balance numeric(14, 2),
    tags text[],
    mood mood,
    uid uuid
);
COMMENT ON COLUMN users.email IS 'Login address';
CREATE INDEX users_name_idx ON users (name);

INSERT INTO users VALUES
    (1, 'ada@example.com', 'Ada Lovelace', '2026-01-02 09:00:00+00', true, '{"plan": "pro"}', '\x89504e47', 99.5, 123456789012.34, '{math,engines}', 'happy', '00000000-0000-0000-0000-000000000001'),
    (2, 'bob@example.com', 'Bob', '2026-01-03 10:30:00+00', false, NULL, NULL, NULL, NULL, NULL, NULL, NULL),
    (3, 'zoe@example.com', 'Zoë 🚀', '2026-01-04 11:45:00+00', true, '{"plan": "free"}', '\x00ff10', 12.25, 0, '{}', 'sad', NULL),
    (4, 'percent@example.com', '50% off', '2026-01-05 12:00:00+00', true, NULL, NULL, 0, NULL, NULL, NULL, NULL),
    (5, 'quote@example.com', 'O''Brien C:\temp', '2026-01-06 13:15:00+00', true, NULL, NULL, -1, NULL, NULL, NULL, NULL);

CREATE TABLE orders (
    id integer PRIMARY KEY,
    user_id integer NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    total numeric(10, 2) NOT NULL,
    note text
);
CREATE INDEX orders_user_total_idx ON orders (user_id, total);
INSERT INTO orders VALUES (1, 1, 19.99, 'first'), (2, 1, 5.00, NULL), (3, 3, 120.50, 'bulk');

CREATE TABLE events (kind text, payload text);
INSERT INTO events VALUES ('login', 'ada'), ('logout', 'ada'), ('login', 'zoe');

CREATE TABLE "weird ""name""" ("col with space" text, "select" integer);
INSERT INTO "weird ""name""" VALUES ('quoted', 1);

CREATE TABLE big (id integer PRIMARY KEY, label text NOT NULL, bucket integer NOT NULL);
INSERT INTO big SELECT i, 'row ' || i, i % 10 FROM generate_series(1, 100000) AS i;
ANALYZE big;

CREATE VIEW active_users AS SELECT id, email FROM users WHERE active;
CREATE MATERIALIZED VIEW user_counts AS SELECT active, count(*) AS n FROM users GROUP BY active;

CREATE SCHEMA billing;
CREATE TABLE billing.invoices (
    id integer PRIMARY KEY,
    user_id integer NOT NULL REFERENCES public.users (id) ON UPDATE RESTRICT ON DELETE SET NULL,
    amount numeric(10, 2) NOT NULL
);
```

(The `ON DELETE SET NULL` on a `NOT NULL` column is legal to declare; it only fails when used, which never happens here.)

Add to `crates/tabletist-db/src/fixtures.rs`:

```rust
/// The PostgreSQL fixture script, loaded by the integration tests through
/// an admin connection.
pub const POSTGRES_SQL: &str = include_str!("../fixtures/postgres.sql");
```

Create `crates/tabletist-db/tests/postgres.rs`:

```rust
//! PostgreSQL through the public `Connection` API. Needs a server: run
//! `docker compose up -d postgres` and set TABLETIST_TEST_PG_URL (see
//! compose.yaml). Without it every test prints "skipped" and passes.

#![allow(clippy::unwrap_used)]

use tabletist_db::{ConnectSpec, Connection, Driver, Error, ObjectKind, Secrets, TlsMode};
use tokio::sync::OnceCell;

static FIXTURE: OnceCell<()> = OnceCell::const_new();

fn url() -> Option<String> {
    std::env::var("TABLETIST_TEST_PG_URL").ok().filter(|url| !url.trim().is_empty())
}

/// The spec and secrets from the URL, with TLS turned off unless a test asks.
fn spec() -> Option<(ConnectSpec, Secrets)> {
    let (mut spec, secrets) = ConnectSpec::from_url(&url()?).unwrap();
    spec.tls = TlsMode::Disable;
    Some((spec, secrets))
}

async fn load_fixture() {
    FIXTURE
        .get_or_init(|| async {
            let mut config: tokio_postgres::Config = url().unwrap().parse().unwrap();
            config.ssl_mode(tokio_postgres::config::SslMode::Disable);
            let (client, connection) = config.connect(tokio_postgres::NoTls).await.unwrap();
            let task = tokio::spawn(connection);
            client.batch_execute(tabletist_db::fixtures::POSTGRES_SQL).await.unwrap();
            drop(client);
            let _ = task.await;
        })
        .await;
}

/// A read-only connection to the fixture, or `None` (test skipped).
async fn connect() -> Option<Connection> {
    let Some((spec, secrets)) = spec() else {
        eprintln!("skipped: TABLETIST_TEST_PG_URL is not set");
        return None;
    };
    load_fixture().await;
    Some(Connection::connect(&spec, &secrets).await.unwrap())
}

#[tokio::test]
async fn connections_report_postgres_and_list_databases() {
    let Some(connection) = connect().await else { return };
    assert_eq!(connection.driver(), Driver::Postgres);
    let databases = connection.list_databases().await.unwrap();
    assert!(databases.contains(&"tabletist".to_owned()), "{databases:?}");
    assert!(!databases.contains(&"template0".to_owned()));
}

#[tokio::test]
async fn schemas_include_public_and_billing() {
    let Some(connection) = connect().await else { return };
    let schemas = connection.list_schemas().await.unwrap();
    for schema in ["public", "billing", "pg_catalog"] {
        assert!(schemas.contains(&schema.to_owned()), "{schema} in {schemas:?}");
    }
}

#[tokio::test]
async fn objects_have_kinds_and_estimates() {
    let Some(connection) = connect().await else { return };
    let objects = connection.list_objects("public").await.unwrap();
    let find = |name: &str| objects.iter().find(|object| object.name == name).unwrap();
    assert_eq!(find("users").kind, ObjectKind::Table);
    assert_eq!(find("active_users").kind, ObjectKind::View);
    assert_eq!(find("user_counts").kind, ObjectKind::MaterializedView);
    assert_eq!(find("weird \"name\"").kind, ObjectKind::Table);
    assert_eq!(find("big").estimated_rows, Some(100_000));
    let names: Vec<&str> = objects.iter().map(|object| object.name.as_str()).collect();
    let mut sorted = names.clone();
    sorted.sort_unstable();
    assert_eq!(names, sorted, "objects are sorted by name");
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
    // The compose server has a self-signed certificate.
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
    let spec = ConnectSpec { driver: Driver::Postgres, user: String::new(), ..ConnectSpec::default() };
    assert!(matches!(
        Connection::connect(&spec, &Secrets::default()).await,
        Err(Error::InvalidSpec(_))
    ));
}
```

Add `tokio-postgres` to the crate's `[dev-dependencies]` is not needed (it is a regular dependency); add `tokio = { version = "1", features = ["rt-multi-thread", "macros", "time", "sync"] }` (add `"sync"`) to `[dev-dependencies]` for `OnceCell`.

In `crates/tabletist-db/tests/sqlite.rs`, delete the test `postgres_and_mysql_are_not_supported_yet` and add:

```rust
#[tokio::test]
async fn mysql_is_not_supported_yet() {
    let (spec, secrets) = ConnectSpec::from_url("mysql://localhost/db").unwrap();
    assert!(matches!(
        Connection::connect(&spec, &secrets).await,
        Err(Error::Unsupported(_))
    ));
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Start the server: `docker compose up -d postgres` and wait until `pg_isready -h localhost -p 55432` reports "accepting connections" (if Docker is not available, use any PostgreSQL 13+ with a `tabletist` superuser and database and set the URL accordingly).

Run: `TABLETIST_TEST_PG_URL=postgres://tabletist:tabletist@localhost:55432/tabletist cargo test -p tabletist-db --test postgres`
Expected: FAIL: `a_user_is_required` and the connect tests fail with `Unsupported("PostgreSQL connections")`.

Also run without the variable: `cargo test -p tabletist-db --test postgres`
Expected: the server tests print "skipped" and pass; `a_user_is_required` fails.

If `docker compose up` fails because the snake-oil certificate files do not exist in the image, replace the three `-c ssl...` lines with a command that generates a certificate first:

```yaml
    entrypoint: ["bash", "-c", "openssl req -new -x509 -days 3650 -nodes -subj /CN=localhost -out /tmp/server.crt -keyout /tmp/server.key && chown postgres /tmp/server.* && chmod 600 /tmp/server.key && exec docker-entrypoint.sh postgres -c ssl=on -c ssl_cert_file=/tmp/server.crt -c ssl_key_file=/tmp/server.key"]
```

and remove `command:`.

- [ ] **Step 3: Implement the adapter's connect and listing**

Create `crates/tabletist-db/src/pg.rs`:

```rust
//! PostgreSQL. Catalog queries use the extended protocol with typed results;
//! row queries use the simple-query protocol (Task 5), which sends every
//! value as text.

use std::time::Duration;

use tokio_postgres::error::SqlState;
use tokio_postgres_rustls::MakeRustlsConnect;

use crate::{ConnectSpec, Error, ObjectInfo, ObjectKind, Result, Secrets};

pub struct Conn {
    pub(crate) client: tokio::sync::Mutex<tokio_postgres::Client>,
    pub(crate) cancel: tokio_postgres::CancelToken,
    pub(crate) tls: MakeRustlsConnect,
}

/// The error and its causes, joined, for messages ("error connecting to
/// server: Connection refused (os error 111)").
fn describe(error: &(dyn std::error::Error + 'static)) -> String {
    let mut text = error.to_string();
    let mut source = error.source();
    while let Some(cause) = source {
        let cause_text = cause.to_string();
        if !text.contains(&cause_text) {
            text.push_str(": ");
            text.push_str(&cause_text);
        }
        source = cause.source();
    }
    text
}

/// Whether a rustls error is anywhere in the chain (wrapped in io::Error by
/// tokio-rustls).
fn is_tls_error(error: &(dyn std::error::Error + 'static)) -> bool {
    let mut current: Option<&(dyn std::error::Error + 'static)> = Some(error);
    while let Some(error) = current {
        if error.downcast_ref::<rustls::Error>().is_some() {
            return true;
        }
        if let Some(io) = error.downcast_ref::<std::io::Error>()
            && io.get_ref().is_some_and(|inner| inner.downcast_ref::<rustls::Error>().is_some())
        {
            return true;
        }
        current = error.source();
    }
    false
}

pub(crate) fn connect_error(error: tokio_postgres::Error) -> Error {
    if let Some(db) = error.as_db_error() {
        let code = db.code();
        if *code == SqlState::INVALID_PASSWORD || *code == SqlState::INVALID_AUTHORIZATION_SPECIFICATION {
            return Error::Auth(db.message().to_owned());
        }
        return Error::Connect(db.message().to_owned());
    }
    if is_tls_error(&error) {
        return Error::Tls(describe(&error));
    }
    let text = describe(&error);
    if text.contains("password missing") {
        return Error::Auth("the server asks for a password".into());
    }
    Error::Connect(text)
}

pub(crate) fn query_error(error: tokio_postgres::Error) -> Error {
    if error.is_closed() {
        return Error::ConnectionLost(describe(&error));
    }
    if let Some(db) = error.as_db_error() {
        if *db.code() == SqlState::QUERY_CANCELED {
            return Error::Cancelled;
        }
        return Error::Query {
            code: Some(db.code().code().to_owned()),
            message: db.message().to_owned(),
            detail: db.detail().map(str::to_owned),
            hint: db.hint().map(str::to_owned),
        };
    }
    Error::ConnectionLost(describe(&error))
}

impl Conn {
    pub async fn connect(spec: &ConnectSpec, secrets: &Secrets) -> Result<Self> {
        if spec.user.trim().is_empty() {
            return Err(Error::InvalidSpec("enter a user name".into()));
        }
        let tls = MakeRustlsConnect::new(crate::tls::client_config(spec.tls, spec.ca_file.as_deref())?);
        let mut config = tokio_postgres::Config::new();
        config
            .host(&spec.host)
            .port(spec.port)
            .user(&spec.user)
            // libpq's default database is the user's name.
            .dbname(if spec.database.is_empty() { &spec.user } else { &spec.database })
            .application_name("Tabletist")
            .connect_timeout(Duration::from_secs(10))
            .ssl_mode(crate::tls::ssl_mode(spec.tls));
        if let Some(password) = &secrets.password {
            config.password(password);
        }
        let (client, connection) = config.connect(tls.clone()).await.map_err(connect_error)?;
        tokio::spawn(async move {
            if let Err(error) = connection.await {
                log::info!("PostgreSQL connection ended: {error}");
            }
        });
        client
            .batch_execute("SET default_transaction_read_only = on; SET standard_conforming_strings = on")
            .await
            .map_err(query_error)?;
        let cancel = client.cancel_token();
        Ok(Self {
            client: tokio::sync::Mutex::new(client),
            cancel,
            tls,
        })
    }

    /// A catalog query: fixed SQL, typed results.
    async fn catalog(
        &self,
        sql: &str,
        params: &[&(dyn tokio_postgres::types::ToSql + Sync)],
    ) -> Result<Vec<tokio_postgres::Row>> {
        let client = self.client.lock().await;
        client.query(sql, params).await.map_err(query_error)
    }

    pub async fn list_databases(&self) -> Result<Vec<String>> {
        let rows = self
            .catalog(
                "SELECT datname::text FROM pg_database \
                 WHERE datallowconn AND NOT datistemplate ORDER BY datname",
                &[],
            )
            .await?;
        Ok(rows.iter().map(|row| row.get(0)).collect())
    }

    pub async fn list_schemas(&self) -> Result<Vec<String>> {
        let rows = self
            .catalog("SELECT nspname::text FROM pg_namespace ORDER BY nspname", &[])
            .await?;
        Ok(rows.iter().map(|row| row.get(0)).collect())
    }

    pub async fn list_objects(&self, schema: &str) -> Result<Vec<ObjectInfo>> {
        let rows = self
            .catalog(
                "SELECT c.relname::text, c.relkind::text, c.reltuples::float8 \
                 FROM pg_class c JOIN pg_namespace n ON n.oid = c.relnamespace \
                 WHERE n.nspname = $1 AND c.relkind IN ('r', 'p', 'f', 'v', 'm') \
                 ORDER BY c.relname",
                &[&schema],
            )
            .await?;
        Ok(rows
            .iter()
            .map(|row| {
                let kind: String = row.get(1);
                let tuples: f64 = row.get(2);
                ObjectInfo {
                    name: row.get(0),
                    kind: match kind.as_str() {
                        "v" => ObjectKind::View,
                        "m" => ObjectKind::MaterializedView,
                        _ => ObjectKind::Table,
                    },
                    // -1 means never analyzed (PostgreSQL 14+).
                    estimated_rows: (tuples >= 0.0).then_some(tuples as u64),
                }
            })
            .collect())
    }
}
```

In `crates/tabletist-db/src/lib.rs`:
- add `mod pg;` and change `#[allow(dead_code)] mod tls;` to `mod tls;`;
- add the variant `Postgres(pg::Conn)` to `Connection`;
- in `connect`, replace the PostgreSQL arm with `Driver::Postgres => Ok(Self::Postgres(pg::Conn::connect(spec, secrets).await?)),` and rename `_secrets` to `secrets`;
- add a `Self::Postgres(..)` arm to every match: `driver` → `Driver::Postgres`, `dialect` → `Dialect::Postgres`, `list_databases`/`list_schemas`/`list_objects` → the `pg::Conn` methods, `describe`/`fetch_rows`/`count_rows` → `Err(Error::Unsupported("PostgreSQL browsing arrives in the next task"))` for now, `close` → `drop(conn); Ok(())`;
- extend the cancel handle:

```rust
#[derive(Clone)]
enum CancelInner {
    Sqlite(Arc<rusqlite::InterruptHandle>),
    Postgres {
        token: tokio_postgres::CancelToken,
        tls: tokio_postgres_rustls::MakeRustlsConnect,
    },
}
```

with `cancel_handle` returning `CancelHandle(CancelInner::Postgres { token: conn.cancel.clone(), tls: conn.tls.clone() })` for PostgreSQL and `cancel` doing:

```rust
            CancelInner::Postgres { token, tls } => token
                .cancel_query(tls.clone())
                .await
                .map_err(|error| Error::Connect(format!("could not cancel: {error}"))),
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `TABLETIST_TEST_PG_URL=postgres://tabletist:tabletist@localhost:55432/tabletist cargo test -p tabletist-db --test postgres`
Expected: PASS (8 tests).

Run: `cargo test --locked --workspace --all-targets`
Expected: PASS (PostgreSQL tests skip without the variable).

- [ ] **Step 5: Commit**

```bash
git add crates compose.yaml Cargo.lock
git commit -m "Connect to PostgreSQL and list databases, schemas and objects"
```

---

### Task 4: PostgreSQL structure

**Files:**
- Modify: `crates/tabletist-db/src/pg.rs`, `crates/tabletist-db/src/lib.rs`, `crates/tabletist-db/tests/postgres.rs`

**Interfaces:**
- Consumes: Task 3's `Conn::catalog`.
- Produces: `pg::Conn::describe(&self, object: &ObjectRef) -> Result<Structure>`, `pg::Conn::primary_key(&self, object: &ObjectRef) -> Result<Vec<String>>`; `Connection::describe` dispatches to it.

- [ ] **Step 1: Write the failing tests**

Add to `crates/tabletist-db/tests/postgres.rs`:

```rust
use tabletist_db::ObjectRef;

#[tokio::test]
async fn users_structure_has_types_defaults_comments_key_and_indexes() {
    let Some(connection) = connect().await else { return };
    let structure = connection.describe(&ObjectRef::new("public", "users")).await.unwrap();
    let column = |name: &str| structure.columns.iter().find(|c| c.name == name).unwrap();
    assert_eq!(structure.columns[0].name, "id");
    assert_eq!(column("id").type_name, "integer");
    assert!(!column("id").nullable);
    assert_eq!(column("created_at").type_name, "timestamp with time zone");
    assert!(column("created_at").default.as_deref().unwrap().contains("2026-01-01"));
    assert_eq!(column("balance").type_name, "numeric(14,2)");
    assert_eq!(column("tags").type_name, "text[]");
    assert_eq!(column("mood").type_name, "mood");
    assert_eq!(column("email").comment.as_deref(), Some("Login address"));
    assert_eq!(structure.primary_key, vec!["id".to_owned()]);
    let pkey = structure.indexes.iter().find(|i| i.name == "users_pkey").unwrap();
    assert!(pkey.primary && pkey.unique);
    assert_eq!(pkey.method.as_deref(), Some("btree"));
    let email = structure.indexes.iter().find(|i| i.name == "users_email_key").unwrap();
    assert!(email.unique && !email.primary);
    assert_eq!(email.columns, vec!["email".to_owned()]);
}

#[tokio::test]
async fn foreign_keys_name_their_target_and_actions() {
    let Some(connection) = connect().await else { return };
    let orders = connection.describe(&ObjectRef::new("public", "orders")).await.unwrap();
    let key = &orders.foreign_keys[0];
    assert_eq!(key.name.as_deref(), Some("orders_user_id_fkey"));
    assert_eq!(key.columns, vec!["user_id".to_owned()]);
    assert_eq!((key.ref_schema.as_str(), key.ref_table.as_str()), ("public", "users"));
    assert_eq!(key.ref_columns, vec!["id".to_owned()]);
    assert_eq!(key.on_delete, "CASCADE");
    assert_eq!(key.on_update, "NO ACTION");
    let composite = orders.indexes.iter().find(|i| i.name == "orders_user_total_idx").unwrap();
    assert_eq!(composite.columns, vec!["user_id".to_owned(), "total".to_owned()]);
    let invoices = connection.describe(&ObjectRef::new("billing", "invoices")).await.unwrap();
    assert_eq!(invoices.foreign_keys[0].ref_schema, "public");
    assert_eq!(invoices.foreign_keys[0].on_update, "RESTRICT");
    assert_eq!(invoices.foreign_keys[0].on_delete, "SET NULL");
}

#[tokio::test]
async fn views_and_quoted_names_describe_and_missing_objects_fail() {
    let Some(connection) = connect().await else { return };
    let view = connection.describe(&ObjectRef::new("public", "active_users")).await.unwrap();
    assert_eq!(view.columns.len(), 2);
    assert!(view.primary_key.is_empty() && view.indexes.is_empty());
    let weird = connection.describe(&ObjectRef::new("public", "weird \"name\"")).await.unwrap();
    assert_eq!(weird.columns[1].name, "select");
    assert!(matches!(
        connection.describe(&ObjectRef::new("public", "nope")).await,
        Err(Error::Query { .. })
    ));
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `TABLETIST_TEST_PG_URL=postgres://tabletist:tabletist@localhost:55432/tabletist cargo test -p tabletist-db --test postgres`
Expected: FAIL: the three new tests get `Unsupported`.

- [ ] **Step 3: Implement**

Add to `impl Conn` in `pg.rs`:

```rust
    async fn relation(&self, object: &ObjectRef) -> Result<u32> {
        let rows = self
            .catalog(
                "SELECT c.oid FROM pg_class c JOIN pg_namespace n ON n.oid = c.relnamespace \
                 WHERE n.nspname = $1 AND c.relname = $2",
                &[&object.schema, &object.name],
            )
            .await?;
        rows.first()
            .map(|row| row.get(0))
            .ok_or_else(|| Error::query(format!("no such table or view: {}", object.name)))
    }

    pub async fn primary_key(&self, object: &ObjectRef) -> Result<Vec<String>> {
        let rows = self
            .catalog(
                "SELECT a.attname::text \
                 FROM pg_index i \
                 JOIN pg_class c ON c.oid = i.indrelid \
                 JOIN pg_namespace n ON n.oid = c.relnamespace \
                 CROSS JOIN LATERAL unnest(i.indkey) WITH ORDINALITY AS k(attnum, ord) \
                 JOIN pg_attribute a ON a.attrelid = i.indrelid AND a.attnum = k.attnum \
                 WHERE n.nspname = $1 AND c.relname = $2 AND i.indisprimary \
                 ORDER BY k.ord",
                &[&object.schema, &object.name],
            )
            .await?;
        Ok(rows.iter().map(|row| row.get(0)).collect())
    }

    pub async fn describe(&self, object: &ObjectRef) -> Result<Structure> {
        let oid = self.relation(object).await?;
        let columns = self
            .catalog(
                "SELECT a.attname::text, format_type(a.atttypid, a.atttypmod), NOT a.attnotnull, \
                        pg_get_expr(d.adbin, d.adrelid), col_description(a.attrelid, a.attnum) \
                 FROM pg_attribute a \
                 LEFT JOIN pg_attrdef d ON d.adrelid = a.attrelid AND d.adnum = a.attnum \
                 WHERE a.attrelid = $1 AND a.attnum > 0 AND NOT a.attisdropped \
                 ORDER BY a.attnum",
                &[&oid],
            )
            .await?
            .iter()
            .map(|row| ColumnInfo {
                name: row.get(0),
                type_name: row.get(1),
                nullable: row.get(2),
                default: row.get(3),
                comment: row.get(4),
            })
            .collect();
        let indexes = self
            .catalog(
                "SELECT ic.relname::text, i.indisunique, i.indisprimary, am.amname::text, \
                        ARRAY(SELECT pg_get_indexdef(i.indexrelid, k, true) \
                              FROM generate_series(1, i.indnkeyatts::int) AS k ORDER BY k) \
                 FROM pg_index i \
                 JOIN pg_class ic ON ic.oid = i.indexrelid \
                 JOIN pg_am am ON am.oid = ic.relam \
                 WHERE i.indrelid = $1 ORDER BY ic.relname",
                &[&oid],
            )
            .await?
            .iter()
            .map(|row| IndexInfo {
                name: row.get(0),
                unique: row.get(1),
                primary: row.get(2),
                method: Some(row.get(3)),
                columns: row.get(4),
            })
            .collect();
        let foreign_keys = self
            .catalog(
                "SELECT con.conname::text, \
                        ARRAY(SELECT a.attname::text FROM unnest(con.conkey) WITH ORDINALITY AS k(attnum, ord) \
                              JOIN pg_attribute a ON a.attrelid = con.conrelid AND a.attnum = k.attnum \
                              ORDER BY k.ord), \
                        rn.nspname::text, rc.relname::text, \
                        ARRAY(SELECT a.attname::text FROM unnest(con.confkey) WITH ORDINALITY AS k(attnum, ord) \
                              JOIN pg_attribute a ON a.attrelid = con.confrelid AND a.attnum = k.attnum \
                              ORDER BY k.ord), \
                        con.confupdtype::text, con.confdeltype::text \
                 FROM pg_constraint con \
                 JOIN pg_class rc ON rc.oid = con.confrelid \
                 JOIN pg_namespace rn ON rn.oid = rc.relnamespace \
                 WHERE con.conrelid = $1 AND con.contype = 'f' ORDER BY con.conname",
                &[&oid],
            )
            .await?
            .iter()
            .map(|row| {
                let on_update: String = row.get(5);
                let on_delete: String = row.get(6);
                ForeignKeyInfo {
                    name: Some(row.get(0)),
                    columns: row.get(1),
                    ref_schema: row.get(2),
                    ref_table: row.get(3),
                    ref_columns: row.get(4),
                    on_update: action(&on_update).into(),
                    on_delete: action(&on_delete).into(),
                }
            })
            .collect();
        Ok(Structure {
            columns,
            primary_key: self.primary_key(object).await?,
            indexes,
            foreign_keys,
        })
    }
```

and the free function:

```rust
/// pg_constraint's one-letter referential actions.
fn action(code: &str) -> &'static str {
    match code {
        "r" => "RESTRICT",
        "c" => "CASCADE",
        "n" => "SET NULL",
        "d" => "SET DEFAULT",
        _ => "NO ACTION",
    }
}
```

Extend the `use crate::{...}` line in `pg.rs` with `ColumnInfo, ForeignKeyInfo, IndexInfo, ObjectRef, Structure`. In `lib.rs`, route `Connection::describe` for `Self::Postgres(conn)` to `conn.describe(object).await`.

- [ ] **Step 4: Run the tests to verify they pass**

Run: `TABLETIST_TEST_PG_URL=postgres://tabletist:tabletist@localhost:55432/tabletist cargo test -p tabletist-db --test postgres`
Expected: PASS (11 tests).

- [ ] **Step 5: Commit**

```bash
git add crates
git commit -m "Describe PostgreSQL tables and views"
```

---

### Task 5: PostgreSQL rows, counts and cancel

**Files:**
- Modify: `crates/tabletist-db/src/pg.rs`, `crates/tabletist-db/src/lib.rs`, `crates/tabletist-db/tests/postgres.rs`

**Interfaces:**
- Consumes: Tasks 1, 3, 4.
- Produces: `pg::Conn::fetch_rows(&self, query: &RowQuery) -> Result<RowPage>`, `pg::Conn::count_rows(&self, query: &RowQuery) -> Result<u64>`; `Connection::fetch_rows`/`count_rows` dispatch to them.

- [ ] **Step 1: Write the failing tests**

Add to `crates/tabletist-db/tests/postgres.rs`:

```rust
use std::time::Duration;

use tabletist_db::{Filter, FilterOp, RowPage, RowQuery, Sort, SortDir, Value, ValueKind};

fn users(limit: u32) -> RowQuery {
    RowQuery::new(ObjectRef::new("public", "users"), limit)
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
    let page = connection.fetch_rows(&users(2)).await.unwrap();
    assert_eq!(ids(&page), vec![1, 2]);
    assert!(page.has_more && page.ordered_by_key);
    let kind = |name: &str| page.columns.iter().find(|c| c.name == name).unwrap().kind;
    assert_eq!(kind("id"), ValueKind::Numeric);
    assert_eq!(kind("active"), ValueKind::Bool);
    assert_eq!(kind("meta"), ValueKind::Json);
    assert_eq!(kind("created_at"), ValueKind::Temporal);
    assert_eq!(kind("avatar"), ValueKind::Binary);
    assert_eq!(kind("tags"), ValueKind::Other);
    let ada = &page.rows[0];
    assert_eq!(ada[4], Value::Bool(true));
    assert_eq!(ada[6], Value::Bytes(vec![0x89, 0x50, 0x4e, 0x47].into()));
    assert_eq!(ada[7], Value::Float(99.5));
    assert_eq!(ada[8], Value::Text("123456789012.34".into()));
    assert_eq!(ada[9], Value::Text("{math,engines}".into()));
    assert_eq!(ada[10], Value::Text("happy".into()));
    assert_eq!(page.rows[1][5], Value::Null);
}

#[tokio::test]
async fn sorting_paging_and_counting_work() {
    let Some(connection) = connect().await else { return };
    let mut query = users(5);
    query.sort = vec![Sort { column: "email".into(), dir: SortDir::Desc }];
    assert_eq!(ids(&connection.fetch_rows(&query).await.unwrap()), vec![3, 5, 4, 2, 1]);
    let mut deep = RowQuery::new(ObjectRef::new("public", "big"), 300);
    deep.offset = 99_900;
    let page = connection.fetch_rows(&deep).await.unwrap();
    assert_eq!(page.rows.len(), 100);
    assert!(!page.has_more);
    assert_eq!(connection.count_rows(&RowQuery::new(ObjectRef::new("public", "big"), 1)).await.unwrap(), 100_000);
    let events = connection.fetch_rows(&RowQuery::new(ObjectRef::new("public", "events"), 50)).await.unwrap();
    assert!(!events.ordered_by_key);
}

#[tokio::test]
async fn filters_work_on_every_type() {
    let Some(connection) = connect().await else { return };
    let rows = |query: RowQuery| {
        let connection = &connection;
        async move { ids(&connection.fetch_rows(&query).await.unwrap()) }
    };
    assert_eq!(rows(filtered("id", FilterOp::Ge, "4")).await, vec![4, 5]);
    assert_eq!(rows(filtered("id", FilterOp::In, "1, 3")).await, vec![1, 3]);
    assert_eq!(rows(filtered("name", FilterOp::IsNull, "")).await, Vec::<i64>::new());
    assert_eq!(rows(filtered("meta", FilterOp::IsNull, "")).await, vec![2, 4, 5]);
    assert_eq!(rows(filtered("active", FilterOp::Eq, "false")).await, vec![2]);
    assert_eq!(rows(filtered("name", FilterOp::Contains, "ZO")).await, vec![3]);
    assert_eq!(rows(filtered("name", FilterOp::Contains, "50%")).await, vec![4]);
    assert_eq!(rows(filtered("tags", FilterOp::Contains, "math")).await, vec![1]);
    assert_eq!(connection.count_rows(&filtered("id", FilterOp::Lt, "3")).await.unwrap(), 2);
}

#[tokio::test]
async fn quotes_and_backslashes_in_filter_values_are_just_text() {
    let Some(connection) = connect().await else { return };
    let rows = connection.fetch_rows(&filtered("name", FilterOp::Eq, r"O'Brien C:\temp")).await.unwrap();
    assert_eq!(ids(&rows), vec![5]);
    let rows = connection
        .fetch_rows(&filtered("name", FilterOp::Eq, "x' OR '1'='1"))
        .await
        .unwrap();
    assert!(rows.rows.is_empty());
}

#[tokio::test]
async fn a_raw_where_cannot_write_or_leave_read_only_mode() {
    let Some(connection) = connect().await else { return };
    for raw in [
        "1 = 1; DELETE FROM users",
        "1=1) ; COMMIT; DELETE FROM users; SELECT (1",
        "nextval('tick') > 0",
    ] {
        let mut query = users(50);
        query.raw_where = Some(raw.into());
        assert!(connection.fetch_rows(&query).await.is_err(), "{raw}");
    }
    // Turning the session default off does not open a writable transaction.
    let mut query = users(50);
    query.raw_where = Some("set_config('default_transaction_read_only', 'off', false) IS NOT NULL".into());
    let _ = connection.fetch_rows(&query).await;
    query.raw_where = Some("nextval('tick') > 0".into());
    assert!(connection.fetch_rows(&query).await.is_err());
    assert_eq!(connection.count_rows(&users(1)).await.unwrap(), 5);
}

#[tokio::test]
async fn a_bad_raw_where_is_a_query_error_with_a_code() {
    let Some(connection) = connect().await else { return };
    let mut query = users(50);
    query.raw_where = Some("no_such_column = 1".into());
    match connection.fetch_rows(&query).await {
        Err(Error::Query { code, .. }) => assert_eq!(code.as_deref(), Some("42703")),
        other => panic!("{other:?}"),
    }
    // The session is still usable after a failed statement.
    assert!(connection.fetch_rows(&users(1)).await.is_ok());
}

#[tokio::test]
async fn quoted_names_and_views_browse() {
    let Some(connection) = connect().await else { return };
    let mut query = RowQuery::new(ObjectRef::new("public", "weird \"name\""), 5);
    query.sort = vec![Sort { column: "select".into(), dir: SortDir::Asc }];
    let page = connection.fetch_rows(&query).await.unwrap();
    assert_eq!(page.columns[0].name, "col with space");
    let view = connection
        .fetch_rows(&RowQuery::new(ObjectRef::new("public", "user_counts"), 5))
        .await
        .unwrap();
    assert_eq!(view.rows.len(), 2);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_running_query_can_be_cancelled() {
    let Some(connection) = connect().await else { return };
    let cancel = connection.cancel_handle();
    let connection = std::sync::Arc::new(connection);
    let running = {
        let connection = std::sync::Arc::clone(&connection);
        tokio::spawn(async move {
            let mut query = users(5);
            query.raw_where = Some("pg_sleep(30) IS NOT NULL".into());
            connection.count_rows(&query).await
        })
    };
    let deadline = std::time::Instant::now() + Duration::from_secs(15);
    while !running.is_finished() {
        assert!(std::time::Instant::now() < deadline, "cancel must stop the query");
        cancel.cancel().await.unwrap();
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
    assert_eq!(running.await.unwrap(), Err(Error::Cancelled));
    assert!(connection.fetch_rows(&users(1)).await.is_ok());
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `TABLETIST_TEST_PG_URL=postgres://tabletist:tabletist@localhost:55432/tabletist cargo test -p tabletist-db --test postgres`
Expected: FAIL: the new tests get `Unsupported`.

- [ ] **Step 3: Implement**

Add to `impl Conn` in `pg.rs`:

```rust
    /// One page. The SQL is prepared first, which refuses a second statement
    /// and yields the column types; it then runs through the simple-query
    /// protocol (every value as text) inside a read-only transaction.
    pub async fn fetch_rows(&self, query: &RowQuery) -> Result<RowPage> {
        let key = self.primary_key(&query.object).await?;
        let sql = Dialect::Postgres.select_rows(query, &key);
        let limit = query.limit as usize;
        let mut client = self.client.lock().await;
        let started = Instant::now();
        let transaction = client
            .build_transaction()
            .read_only(true)
            .start()
            .await
            .map_err(query_error)?;
        let statement = transaction.prepare(&sql.text).await.map_err(query_error)?;
        let columns: Vec<ColumnMeta> = statement
            .columns()
            .iter()
            .map(|column| ColumnMeta {
                name: column.name().to_owned(),
                type_name: column.type_().name().to_owned(),
                kind: ValueKind::from_pg_type(column.type_().name()),
            })
            .collect();
        let messages = transaction.simple_query(&sql.text).await.map_err(query_error)?;
        transaction.rollback().await.map_err(query_error)?;
        let mut rows = Vec::new();
        for message in messages {
            if let SimpleQueryMessage::Row(row) = message {
                if rows.len() > limit {
                    break;
                }
                rows.push(
                    columns
                        .iter()
                        .enumerate()
                        .map(|(index, column)| match row.get(index) {
                            None => Value::Null,
                            Some(text) => value_from_pg_text(&column.type_name, text),
                        })
                        .collect(),
                );
            }
        }
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
        let sql = Dialect::Postgres.count_rows(query);
        let mut client = self.client.lock().await;
        let transaction = client
            .build_transaction()
            .read_only(true)
            .start()
            .await
            .map_err(query_error)?;
        // Prepare first: refuses a second statement before anything runs.
        transaction.prepare(&sql.text).await.map_err(query_error)?;
        let messages = transaction.simple_query(&sql.text).await.map_err(query_error)?;
        transaction.rollback().await.map_err(query_error)?;
        messages
            .iter()
            .find_map(|message| match message {
                SimpleQueryMessage::Row(row) => row.get(0).and_then(|count| count.parse().ok()),
                _ => None,
            })
            .ok_or_else(|| Error::query("the count returned no number"))
    }
```

Extend the imports in `pg.rs`:

```rust
use std::time::{Duration, Instant};

use tokio_postgres::SimpleQueryMessage;

use crate::{
    ColumnInfo, ColumnMeta, ConnectSpec, Dialect, Error, ForeignKeyInfo, IndexInfo, ObjectInfo,
    ObjectKind, ObjectRef, Result, RowPage, RowQuery, Secrets, Structure, Value, ValueKind,
    value_from_pg_text,
};
```

In `lib.rs`, route `fetch_rows` and `count_rows` for `Self::Postgres(conn)` to the new methods.

When a statement fails inside the transaction, dropping `transaction` (by returning early with `?`) makes tokio-postgres roll it back, so the session is usable for the next query (`a_bad_raw_where_is_a_query_error_with_a_code` checks this).

- [ ] **Step 4: Run the tests to verify they pass**

Run: `TABLETIST_TEST_PG_URL=postgres://tabletist:tabletist@localhost:55432/tabletist cargo test -p tabletist-db --test postgres`
Expected: PASS (19 tests).

Run: `cargo test --locked --workspace --all-targets && cargo clippy --locked --workspace --all-targets -- -D warnings`
Expected: all pass.

- [ ] **Step 5: Commit**

```bash
git add crates
git commit -m "Browse PostgreSQL rows with counts and cancel"
```

---

### Task 6: The keyring thread and backend secret commands

**Files:**
- Create: `src/secrets.rs`
- Modify: `Cargo.toml`, `src/lib.rs`, `src/backend.rs`, `src/connections.rs`

**Interfaces:**
- Consumes: `connections::ConnectionId`.
- Produces:
  - `secrets::SERVICE: &str = "dev.tabletist.Tabletist"`; `secrets::password_account(id: &ConnectionId) -> String`.
  - `secrets::SecretString(pub String)` (`Clone, PartialEq, Eq`, redacted `Debug`).
  - `secrets::SecretError` (`Debug, Clone, PartialEq, Eq, Display`): `Locked`, `Unavailable`, `TimedOut`.
  - `secrets::SecretStore` trait (`Send`): `read(&mut self, account) -> Result<Option<String>, SecretError>`, `write(&mut self, account, secret: &str)`, `delete(&mut self, account)`.
  - `secrets::MemoryStore` (`Default`), `secrets::NativeStore` (`Default`).
  - `secrets::Keyring` (`Clone`): `Keyring::start(store: impl SecretStore + 'static) -> Keyring`, `Keyring::native()`, `Keyring::memory()`, `read(&self, account: &str) -> Pending<Option<String>>`, `write(&self, account: &str, secret: SecretString) -> Pending<()>`, `delete(&self, account: &str) -> Pending<()>`; `Pending<T>::wait(self) -> Result<T, SecretError>` (async, 20 s timeout). Each call queues its job immediately, so jobs run in call order.
  - Backend: `Backend::start_with(waker: Waker, keyring: Keyring) -> Backend` (`start` uses `Keyring::native()`); `Command::LoadSecret { request, account: String }`, `Command::StoreSecret { request, account: String, secret: Option<SecretString> }` (None deletes), `Command::ListDatabases { session, request }`; `Event::SecretLoaded { request, result: Result<Option<SecretString>, String> }`, `Event::SecretStored { request, result: Result<(), String> }`, `Event::Databases { session, request, result: Result<Vec<String>, tabletist_db::Error> }`.
  - `connections::PasswordMode` (`Debug, Clone, Copy, PartialEq, Eq, Default=None, Serialize, Deserialize` kebab-case): `None`, `Keyring`, `Ask`; `SavedConnection.password: PasswordMode` (`#[serde(default)]`).

- [ ] **Step 1: Add dependencies and write the failing tests**

Add to the app's `[dependencies]` in `Cargo.toml`:

```toml
# One protected-store API for saved passwords; each platform's native
# provider below. Passwords never go into connections.json.
keyring-core = "1"
```

and target sections:

```toml
[target.'cfg(target_os = "linux")'.dependencies]
# The desktop's Secret Service (GNOME Keyring, KWallet) over D-Bus.
zbus-secret-service-keyring-store = { version = "1.0.1", features = ["rt-tokio-crypto-rust"] }

[target.'cfg(target_os = "macos")'.dependencies]
apple-native-keyring-store = { version = "1.0.2", features = ["keychain"] }

[target.'cfg(windows)'.dependencies]
windows-native-keyring-store = { version = "1.1", default-features = false }
```

Create `src/secrets.rs` with only tests:

```rust
//! Saved passwords in the OS keyring (Secret Service on Linux, Keychain on
//! macOS, Credential Manager on Windows). Every call runs on one dedicated
//! thread, so a locked keyring waiting for the user never blocks the UI or
//! the database runtime.

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn the_memory_keyring_stores_reads_and_deletes() {
        let keyring = Keyring::memory();
        assert_eq!(keyring.read("a").wait().await, Ok(None));
        keyring.write("a", SecretString("hunter2".into())).wait().await.unwrap();
        assert_eq!(keyring.read("a").wait().await, Ok(Some("hunter2".into())));
        keyring.delete("a").wait().await.unwrap();
        keyring.delete("a").wait().await.unwrap();
        assert_eq!(keyring.read("a").wait().await, Ok(None));
    }

    #[tokio::test]
    async fn jobs_run_in_the_order_they_were_asked_for() {
        let keyring = Keyring::memory();
        let write = keyring.write("a", SecretString("first".into()));
        let read = keyring.read("a");
        // Waiting in the other order still sees the write.
        assert_eq!(read.wait().await, Ok(Some("first".into())));
        write.wait().await.unwrap();
    }

    #[test]
    fn secrets_do_not_print() {
        assert!(!format!("{:?}", SecretString("hunter2".into())).contains("hunter2"));
    }

    #[test]
    fn accounts_are_per_connection() {
        let id = crate::connections::ConnectionId("abc".into());
        assert_eq!(password_account(&id), "connection/abc/password");
    }

    /// Talks to the real OS keyring; run by hand or in CI on macOS/Windows:
    /// `cargo test --lib secrets::tests::native_store_round_trip -- --ignored --exact`
    #[tokio::test]
    #[ignore]
    async fn native_store_round_trip() {
        let keyring = Keyring::native();
        let account = format!("test/{}", std::process::id());
        keyring.write(&account, SecretString("dummy".into())).wait().await.unwrap();
        assert_eq!(keyring.read(&account).wait().await, Ok(Some("dummy".into())));
        keyring.delete(&account).wait().await.unwrap();
        assert_eq!(keyring.read(&account).wait().await, Ok(None));
    }
}
```

Add `pub mod secrets;` to `src/lib.rs`.

Add backend tests to the `tests` module in `src/backend.rs`:

```rust
    #[test]
    fn secrets_are_stored_loaded_and_deleted_in_order() {
        use crate::secrets::{Keyring, SecretString};
        let mut backend = Backend::start_with(Waker::default(), Keyring::memory());
        backend.send(Command::StoreSecret {
            request: RequestId(1),
            account: "a".into(),
            secret: Some(SecretString("pw".into())),
        });
        backend.send(Command::LoadSecret { request: RequestId(2), account: "a".into() });
        backend.send(Command::StoreSecret { request: RequestId(3), account: "a".into(), secret: None });
        backend.send(Command::LoadSecret { request: RequestId(4), account: "a".into() });
        let mut loaded = std::collections::HashMap::new();
        for _ in 0..4 {
            match backend.wait(WAIT) {
                Some(Event::SecretLoaded { request, result }) => {
                    loaded.insert(request.0, result);
                }
                Some(Event::SecretStored { result, .. }) => assert_eq!(result, Ok(())),
                other => panic!("{other:?}"),
            }
        }
        assert_eq!(loaded[&2], Ok(Some(SecretString("pw".into()))));
        assert_eq!(loaded[&4], Ok(None));
    }

    #[test]
    fn store_commands_do_not_print_secrets() {
        let command = Command::StoreSecret {
            request: RequestId(1),
            account: "a".into(),
            secret: Some(crate::secrets::SecretString("hunter2".into())),
        };
        assert!(!format!("{command:?}").contains("hunter2"));
    }

    #[test]
    fn sqlite_sessions_list_no_databases() {
        let (_dir, spec) = fixture();
        let mut backend = Backend::start(Waker::default());
        let session = SessionId(1);
        backend.send(Command::Connect { session, request: RequestId(1), spec, secrets: Secrets::default() });
        assert!(matches!(backend.wait(WAIT), Some(Event::Connected { .. })));
        backend.send(Command::ListDatabases { session, request: RequestId(2) });
        assert!(matches!(
            backend.wait(WAIT),
            Some(Event::Databases { request: RequestId(2), result: Ok(databases), .. }) if databases.is_empty()
        ));
    }
```

Add to `src/connections.rs` tests:

```rust
    #[test]
    fn password_modes_default_to_none_and_round_trip() {
        let old: SavedConnection = serde_json::from_str(
            r#"{"id": "x", "name": "Old", "spec": {"driver": "sqlite", "sqlite_path": "/a.db"}}"#,
        )
        .unwrap();
        assert_eq!(old.password, PasswordMode::None);
        let json = serde_json::to_string(&SavedConnection { password: PasswordMode::Ask, ..old }).unwrap();
        assert!(json.contains("\"ask\""), "{json}");
    }
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test --lib`
Expected: FAIL to compile: `cannot find type Keyring`, `no variant StoreSecret`, `no field password`.

- [ ] **Step 3: Implement the keyring**

Put above the tests in `src/secrets.rs`:

```rust
use std::collections::HashMap;
use std::fmt;
use std::sync::{Arc, mpsc};
use std::time::Duration;

use crate::connections::ConnectionId;

/// The keyring service all of Tabletist's entries live under.
pub const SERVICE: &str = "dev.tabletist.Tabletist";
const TIMEOUT: Duration = Duration::from_secs(20);

/// The keyring account for a saved connection's password.
pub fn password_account(id: &ConnectionId) -> String {
    format!("connection/{}/password", id.0)
}

/// A password in transit. Prints as `SecretString(..)`.
#[derive(Clone, PartialEq, Eq)]
pub struct SecretString(pub String);

impl fmt::Debug for SecretString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("SecretString(..)")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SecretError {
    /// The keyring is locked or refused access.
    Locked,
    /// No keyring, or it failed.
    Unavailable,
    /// The keyring did not answer in time (often an unlock prompt left open).
    TimedOut,
}

impl fmt::Display for SecretError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Locked => "the keyring is locked",
            Self::Unavailable => "the keyring is not available",
            Self::TimedOut => "the keyring did not answer",
        })
    }
}

pub trait SecretStore: Send {
    fn read(&mut self, account: &str) -> Result<Option<String>, SecretError>;
    fn write(&mut self, account: &str, secret: &str) -> Result<(), SecretError>;
    fn delete(&mut self, account: &str) -> Result<(), SecretError>;
}

/// For tests and demo mode.
#[derive(Default)]
pub struct MemoryStore(HashMap<String, String>);

impl SecretStore for MemoryStore {
    fn read(&mut self, account: &str) -> Result<Option<String>, SecretError> {
        Ok(self.0.get(account).cloned())
    }

    fn write(&mut self, account: &str, secret: &str) -> Result<(), SecretError> {
        self.0.insert(account.to_owned(), secret.to_owned());
        Ok(())
    }

    fn delete(&mut self, account: &str) -> Result<(), SecretError> {
        self.0.remove(account);
        Ok(())
    }
}

/// The platform keyring, opened on first use.
#[derive(Default)]
pub struct NativeStore {
    store: Option<Arc<keyring_core::api::CredentialStore>>,
}

/// Provider errors can quote platform data; never pass their text on.
fn native_error(error: keyring_core::Error) -> SecretError {
    match error {
        keyring_core::Error::NoStorageAccess(_) => SecretError::Locked,
        _ => SecretError::Unavailable,
    }
}

impl NativeStore {
    fn entry(&mut self, account: &str) -> Result<keyring_core::Entry, SecretError> {
        if self.store.is_none() {
            #[cfg(target_os = "linux")]
            let store = zbus_secret_service_keyring_store::Store::new();
            #[cfg(target_os = "macos")]
            let store = apple_native_keyring_store::keychain::Store::new();
            #[cfg(windows)]
            let store = windows_native_keyring_store::Store::new();
            self.store = Some(store.map_err(native_error)?);
        }
        self.store
            .as_ref()
            .ok_or(SecretError::Unavailable)?
            .build(SERVICE, account, None)
            .map_err(native_error)
    }
}

impl SecretStore for NativeStore {
    fn read(&mut self, account: &str) -> Result<Option<String>, SecretError> {
        match self.entry(account)?.get_password() {
            Ok(secret) => Ok(Some(secret)),
            Err(keyring_core::Error::NoEntry) => Ok(None),
            Err(error) => Err(native_error(error)),
        }
    }

    fn write(&mut self, account: &str, secret: &str) -> Result<(), SecretError> {
        self.entry(account)?.set_password(secret).map_err(native_error)
    }

    fn delete(&mut self, account: &str) -> Result<(), SecretError> {
        match self.entry(account)?.delete_credential() {
            Ok(()) | Err(keyring_core::Error::NoEntry) => Ok(()),
            Err(error) => Err(native_error(error)),
        }
    }
}

type Job = Box<dyn FnOnce(&mut dyn SecretStore) + Send>;

/// A handle to the keyring thread.
#[derive(Clone)]
pub struct Keyring {
    jobs: mpsc::Sender<Job>,
}

/// An answer that will arrive from the keyring thread.
pub struct Pending<T>(tokio::sync::oneshot::Receiver<Result<T, SecretError>>);

impl<T> Pending<T> {
    pub async fn wait(self) -> Result<T, SecretError> {
        match tokio::time::timeout(TIMEOUT, self.0).await {
            Ok(Ok(result)) => result,
            Ok(Err(_)) => Err(SecretError::Unavailable),
            Err(_) => Err(SecretError::TimedOut),
        }
    }
}

impl Keyring {
    pub fn start(store: impl SecretStore + 'static) -> Self {
        let (jobs, queue) = mpsc::channel::<Job>();
        let mut store = store;
        let spawned = std::thread::Builder::new()
            .name("tabletist-keyring".into())
            .spawn(move || {
                while let Ok(job) = queue.recv() {
                    job(&mut store);
                }
            });
        if let Err(error) = spawned {
            log::error!("could not start the keyring thread: {error}");
        }
        Self { jobs }
    }

    pub fn native() -> Self {
        Self::start(NativeStore::default())
    }

    pub fn memory() -> Self {
        Self::start(MemoryStore::default())
    }

    /// Queues `work` now (so calls run in order) and returns its answer.
    fn run<T: Send + 'static>(
        &self,
        work: impl FnOnce(&mut dyn SecretStore) -> Result<T, SecretError> + Send + 'static,
    ) -> Pending<T> {
        let (answer, pending) = tokio::sync::oneshot::channel();
        let job: Job = Box::new(move |store| {
            let _ = answer.send(work(store));
        });
        // A failed send drops the job; `wait` then reports Unavailable.
        let _ = self.jobs.send(job);
        Pending(pending)
    }

    pub fn read(&self, account: &str) -> Pending<Option<String>> {
        let account = account.to_owned();
        self.run(move |store| store.read(&account))
    }

    pub fn write(&self, account: &str, secret: SecretString) -> Pending<()> {
        let account = account.to_owned();
        self.run(move |store| store.write(&account, &secret.0))
    }

    pub fn delete(&self, account: &str) -> Pending<()> {
        let account = account.to_owned();
        self.run(move |store| store.delete(&account))
    }
}
```

Add to `src/connections.rs`:

```rust
/// Where a connection's password comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PasswordMode {
    /// No password (SQLite, trust or peer authentication).
    #[default]
    None,
    /// Saved in the OS keyring.
    Keyring,
    /// Asked for on every connect.
    Ask,
}
```

and the field on `SavedConnection` (after `color`):

```rust
    #[serde(default)]
    pub password: PasswordMode,
```

Then add `password: PasswordMode::None,` (import `crate::connections::PasswordMode` where needed) to every `SavedConnection { .. }` literal the compiler reports: in `src/connections.rs` tests, `src/app.rs` tests, `src/testing.rs`, `src/ui/mod.rs` tests, `src/entrypoint.rs` (`demo_setup`), and `ConnectionForm::to_saved` in `src/model.rs` (Task 7 replaces that one).

- [ ] **Step 4: Implement the backend commands**

In `src/backend.rs`:

Add to `Command`:

```rust
    /// Read a saved password from the keyring.
    LoadSecret { request: RequestId, account: String },
    /// Save (`Some`) or delete (`None`) a password in the keyring.
    StoreSecret { request: RequestId, account: String, secret: Option<SecretString> },
    ListDatabases { session: SessionId, request: RequestId },
```

and to `Event`:

```rust
    SecretLoaded { request: RequestId, result: Result<Option<SecretString>, String> },
    SecretStored { request: RequestId, result: Result<(), String> },
    Databases { session: SessionId, request: RequestId, result: Result<Vec<String>, Error> },
```

with `use crate::secrets::{Keyring, SecretString};` at the top.

Give `Worker` a `keyring: Keyring` field, set in `Worker::new(outbox, keyring)`. Split `Backend::start`:

```rust
    /// Starts the backend with the OS keyring.
    pub fn start(waker: Waker) -> Self {
        Self::start_with(waker, Keyring::native())
    }

    /// Starts the backend with the given keyring (tests use `Keyring::memory()`).
    pub fn start_with(waker: Waker, keyring: Keyring) -> Self {
        // the previous body of `start`, passing `keyring` to `Worker::new`
    }
```

In `Worker::handle`, add before the catch-all arm:

```rust
            Command::LoadSecret { request, account } => {
                // Queued on the keyring thread now, so it runs after any
                // StoreSecret sent before it.
                let pending = self.keyring.read(&account);
                let outbox = self.outbox.clone();
                tokio::spawn(async move {
                    let result = pending
                        .wait()
                        .await
                        .map(|secret| secret.map(SecretString))
                        .map_err(|error| error.to_string());
                    outbox.emit(Event::SecretLoaded { request, result });
                });
            }
            Command::StoreSecret { request, account, secret } => {
                let pending = match secret {
                    Some(secret) => self.keyring.write(&account, secret),
                    None => self.keyring.delete(&account),
                };
                let outbox = self.outbox.clone();
                tokio::spawn(async move {
                    let result = pending.wait().await.map_err(|error| error.to_string());
                    if let Err(error) = &result {
                        log::warn!("could not update the keyring: {error}");
                    }
                    outbox.emit(Event::SecretStored { request, result });
                });
            }
```

In `session_of`, add `Command::ListDatabases { session, .. }` to the session arm and `Command::LoadSecret { .. } | Command::StoreSecret { .. }` to the `SessionId(0)` arm. In `fail`, add:

```rust
        Command::ListDatabases { session, request } => Event::Databases { session, request, result: Err(error) },
```

and add `Command::LoadSecret { .. } | Command::StoreSecret { .. }` to the arms that `return`. In `run_session`, add:

```rust
            Command::ListDatabases { session, request } => {
                let result = connection.list_databases().await;
                let lost = lost_error(&result);
                outbox.emit(Event::Databases { session, request, result });
                lost
            }
```

and add `Command::LoadSecret { .. } | Command::StoreSecret { .. }` to its `=> None` arm.

- [ ] **Step 5: Run the tests to verify they pass**

Run: `cargo test --locked --workspace --all-targets`
Expected: PASS (the native round trip is ignored).

Run once by hand on a desktop with a keyring: `cargo test --lib secrets::tests::native_store_round_trip -- --ignored --exact`
Expected: PASS (on Omarchy, the Secret Service may show an unlock prompt).

- [ ] **Step 6: Commit**

```bash
git add Cargo.toml Cargo.lock src
git commit -m "Keep passwords in the OS keyring on a thread of their own"
```

---

### Task 7: Connecting with passwords, the password prompt, and switching databases

**Files:**
- Modify: `src/model.rs`, `src/app.rs`

**Interfaces:**
- Consumes: Task 6 (`PasswordMode`, `password_account`, `SecretString`, `Command::{LoadSecret, StoreSecret, ListDatabases}`, `Event::{SecretLoaded, SecretStored, Databases}`).
- Produces:
  - `ConnectionForm` gains `driver: Driver, host: String, port: String, user: String, password: String, database: String, tls: TlsMode, ca_file: String, password_mode: PasswordMode, has_saved_password: bool`; manual `Default` (driver SQLite, port empty, TLS prefer, password mode none); `ConnectionForm::to_spec(&self) -> Result<ConnectSpec, String>`; `to_saved` uses it.
  - `model::PasswordPrompt { tab: ConnTabId, name: String, password: String, save: bool, message: Option<String> }`; `Dialog::Password(Box<PasswordPrompt>)`.
  - `Workspace` gains `password_mode: PasswordMode, secrets: Secrets, databases: Fetch<Vec<String>>`.
  - Actions: `SetDriver(Driver)`, `SubmitPassword`, `CancelPassword`, `SwitchDatabase { tab: ConnTabId, database: String }`.
  - `App` gains `pending_secrets: HashMap<RequestId, SecretPurpose>` (`enum SecretPurpose { Connect(ConnTabId), Test }`).

- [ ] **Step 1: Write the failing tests**

Add to the `tests` module in `src/app.rs`:

```rust
    use crate::connections::PasswordMode;
    use crate::model::PasswordPrompt;
    use crate::secrets::{SecretString, password_account};

    fn postgres_saved(app: &mut App, mode: PasswordMode) -> ConnectionId {
        let (spec, _) = ConnectSpec::from_url("postgres://me@db.example.com/app").unwrap();
        let saved = SavedConnection {
            id: ConnectionId::new(),
            name: "Prod".into(),
            color: ColorTag::Red,
            password: mode,
            spec,
        };
        let id = saved.id.clone();
        app.connections.upsert(saved);
        id
    }

    fn sent_secrets(app: &App) -> Vec<&Command> {
        app.backend
            .sent
            .iter()
            .filter(|c| matches!(c, Command::LoadSecret { .. } | Command::StoreSecret { .. }))
            .collect()
    }

    fn prompt(app: &mut App) -> &mut PasswordPrompt {
        match app.dialog.as_mut() {
            Some(Dialog::Password(prompt)) => prompt,
            other => panic!("expected the password prompt, got {other:?}"),
        }
    }

    #[test]
    fn a_keyring_connection_loads_its_password_then_connects() {
        let (mut app, _dir) = app();
        let conn = postgres_saved(&mut app, PasswordMode::Keyring);
        let tab = app.active_tab_id();
        app.apply(Action::Connect { tab, conn: conn.clone() });
        let request = match app.backend.sent.last() {
            Some(Command::LoadSecret { request, account }) => {
                assert_eq!(account, &password_account(&conn));
                *request
            }
            other => panic!("{other:?}"),
        };
        app.apply(Action::Backend(Event::SecretLoaded {
            request,
            result: Ok(Some(SecretString("pw".into()))),
        }));
        match app.backend.sent.last() {
            Some(Command::Connect { secrets, .. }) => assert_eq!(secrets.password.as_deref(), Some("pw")),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn a_missing_keyring_password_opens_the_prompt() {
        let (mut app, _dir) = app();
        let conn = postgres_saved(&mut app, PasswordMode::Keyring);
        let tab = app.active_tab_id();
        app.apply(Action::Connect { tab, conn });
        let Some(Command::LoadSecret { request, .. }) = app.backend.sent.last() else { panic!() };
        let request = *request;
        app.apply(Action::Backend(Event::SecretLoaded { request, result: Ok(None) }));
        assert!(prompt(&mut app).save, "a keyring connection offers to save again");
        assert!(prompt(&mut app).message.is_some());
    }

    #[test]
    fn a_keyring_error_opens_the_prompt_with_its_message() {
        let (mut app, _dir) = app();
        let conn = postgres_saved(&mut app, PasswordMode::Keyring);
        let tab = app.active_tab_id();
        app.apply(Action::Connect { tab, conn });
        let Some(Command::LoadSecret { request, .. }) = app.backend.sent.last() else { panic!() };
        let request = *request;
        app.apply(Action::Backend(Event::SecretLoaded {
            request,
            result: Err("the keyring is locked".into()),
        }));
        assert!(prompt(&mut app).message.as_deref().unwrap().contains("locked"));
    }

    #[test]
    fn an_ask_connection_prompts_and_submitting_connects() {
        let (mut app, _dir) = app();
        let conn = postgres_saved(&mut app, PasswordMode::Ask);
        let tab = app.active_tab_id();
        app.apply(Action::Connect { tab, conn });
        assert!(sent_secrets(&app).is_empty(), "no keyring for an Ask connection");
        prompt(&mut app).password = "typed".into();
        app.apply(Action::SubmitPassword);
        assert!(app.dialog.is_none());
        match app.backend.sent.last() {
            Some(Command::Connect { secrets, .. }) => assert_eq!(secrets.password.as_deref(), Some("typed")),
            other => panic!("{other:?}"),
        }
        assert!(sent_secrets(&app).is_empty(), "not saved unless asked");
    }

    #[test]
    fn saving_from_the_prompt_stores_the_password_and_switches_to_keyring() {
        let (mut app, _dir) = app();
        let conn = postgres_saved(&mut app, PasswordMode::Ask);
        let tab = app.active_tab_id();
        app.apply(Action::Connect { tab, conn: conn.clone() });
        prompt(&mut app).password = "typed".into();
        prompt(&mut app).save = true;
        app.apply(Action::SubmitPassword);
        assert!(app.backend.sent.iter().any(|c| matches!(
            c,
            Command::StoreSecret { secret: Some(SecretString(p)), .. } if p == "typed"
        )));
        assert_eq!(app.connections.get(&conn).unwrap().password, PasswordMode::Keyring);
    }

    #[test]
    fn cancelling_the_prompt_leaves_a_disconnected_tab() {
        let (mut app, _dir) = app();
        let conn = postgres_saved(&mut app, PasswordMode::Ask);
        let tab = app.active_tab_id();
        app.apply(Action::Connect { tab, conn });
        app.apply(Action::CancelPassword);
        assert!(matches!(app.workspace(tab).unwrap().status, SessionStatus::Disconnected(Error::Auth(_))));
    }

    #[test]
    fn reconnecting_reuses_the_password_of_this_tab() {
        let (mut app, _dir) = app();
        let conn = postgres_saved(&mut app, PasswordMode::Ask);
        let tab = app.active_tab_id();
        app.apply(Action::Connect { tab, conn });
        prompt(&mut app).password = "typed".into();
        app.apply(Action::SubmitPassword);
        app.apply(Action::Reconnect(tab));
        assert!(app.dialog.is_none());
        match app.backend.sent.last() {
            Some(Command::Connect { secrets, .. }) => assert_eq!(secrets.password.as_deref(), Some("typed")),
            other => panic!("{other:?}"),
        }
    }

    fn postgres_form(app: &mut App) {
        app.apply(Action::NewConnection);
        app.apply(Action::SetDriver(Driver::Postgres));
        let form = form(app);
        form.name = "Prod".into();
        form.host = "db.example.com".into();
        form.user = "me".into();
        form.database = "app".into();
    }

    #[test]
    fn choosing_postgres_fills_the_default_port_and_keyring_mode() {
        let (mut app, _dir) = app();
        app.apply(Action::NewConnection);
        app.apply(Action::SetDriver(Driver::Postgres));
        assert_eq!(form(&mut app).port, "5432");
        assert_eq!(form(&mut app).password_mode, PasswordMode::Keyring);
    }

    #[test]
    fn a_postgres_form_needs_host_user_and_a_port_number() {
        let (mut app, _dir) = app();
        postgres_form(&mut app);
        form(&mut app).port = "fifty".into();
        app.apply(Action::SaveConnection { connect: false });
        assert!(form(&mut app).message.as_deref().unwrap().contains("port"));
        form(&mut app).port = "5433".into();
        form(&mut app).user.clear();
        app.apply(Action::SaveConnection { connect: false });
        assert!(form(&mut app).message.as_deref().unwrap().contains("user"));
    }

    #[test]
    fn saving_with_a_typed_password_stores_it_and_connect_uses_it() {
        let (mut app, _dir) = app();
        postgres_form(&mut app);
        form(&mut app).password = "pw".into();
        app.apply(Action::SaveConnection { connect: true });
        let saved = &app.connections.connections[0];
        assert_eq!(saved.password, PasswordMode::Keyring);
        assert_eq!(saved.spec.port, 5432);
        assert!(app.backend.sent.iter().any(|c| matches!(
            c,
            Command::StoreSecret { secret: Some(SecretString(p)), .. } if p == "pw"
        )));
        match app.backend.sent.last() {
            Some(Command::Connect { secrets, .. }) => assert_eq!(secrets.password.as_deref(), Some("pw")),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn saving_without_retyping_keeps_the_keyring_password() {
        let (mut app, _dir) = app();
        let conn = postgres_saved(&mut app, PasswordMode::Keyring);
        app.apply(Action::EditConnection(conn));
        assert!(form(&mut app).has_saved_password);
        form(&mut app).name = "Renamed".into();
        app.apply(Action::SaveConnection { connect: false });
        assert!(sent_secrets(&app).is_empty(), "the saved password is left alone");
    }

    #[test]
    fn changing_to_ask_deletes_the_saved_password() {
        let (mut app, _dir) = app();
        let conn = postgres_saved(&mut app, PasswordMode::Keyring);
        app.apply(Action::EditConnection(conn.clone()));
        form(&mut app).password_mode = PasswordMode::Ask;
        app.apply(Action::SaveConnection { connect: false });
        assert!(app.backend.sent.iter().any(|c| matches!(
            c,
            Command::StoreSecret { secret: None, account, .. } if *account == password_account(&conn)
        )));
    }

    #[test]
    fn deleting_a_connection_deletes_its_password() {
        let (mut app, _dir) = app();
        let conn = postgres_saved(&mut app, PasswordMode::Keyring);
        app.apply(Action::DeleteConnection(conn));
        assert!(matches!(sent_secrets(&app).as_slice(), [Command::StoreSecret { secret: None, .. }]));
    }

    #[test]
    fn sqlite_saves_never_touch_the_keyring() {
        let (mut app, _dir) = app();
        let id = with_saved(&mut app);
        app.apply(Action::EditConnection(id.clone()));
        app.apply(Action::SaveConnection { connect: true });
        app.apply(Action::DeleteConnection(id));
        assert!(sent_secrets(&app).is_empty());
    }

    #[test]
    fn a_pasted_postgres_url_fills_every_field() {
        let (mut app, _dir) = app();
        app.apply(Action::NewConnection);
        form(&mut app).url = "postgres://me:pw@db.example.com:6543/app?sslmode=verify-full".into();
        app.apply(Action::ApplyUrl);
        let form = form(&mut app);
        assert_eq!(form.driver, Driver::Postgres);
        assert_eq!((form.host.as_str(), form.port.as_str()), ("db.example.com", "6543"));
        assert_eq!((form.user.as_str(), form.database.as_str()), ("me", "app"));
        assert_eq!(form.password, "pw");
        assert_eq!(form.tls, tabletist_db::TlsMode::VerifyFull);
        assert_eq!(form.name, "me@db.example.com:6543/app");
    }

    #[test]
    fn testing_an_unchanged_keyring_connection_loads_its_password() {
        let (mut app, _dir) = app();
        let conn = postgres_saved(&mut app, PasswordMode::Keyring);
        app.apply(Action::EditConnection(conn));
        app.apply(Action::TestConnection);
        let Some(Command::LoadSecret { request, .. }) = app.backend.sent.last() else { panic!() };
        let request = *request;
        app.apply(Action::Backend(Event::SecretLoaded {
            request,
            result: Ok(Some(SecretString("pw".into()))),
        }));
        match app.backend.sent.last() {
            Some(Command::Test { request: r, secrets, .. }) => {
                assert_eq!(*r, request);
                assert_eq!(secrets.password.as_deref(), Some("pw"));
            }
            other => panic!("{other:?}"),
        }
    }

    fn connected_postgres(app: &mut App) -> ConnTabId {
        let conn = postgres_saved(app, PasswordMode::None);
        let tab = app.active_tab_id();
        app.apply(Action::Connect { tab, conn });
        let Some(Command::Connect { session, request, .. }) = app.backend.sent.last() else { panic!() };
        let (session, request) = (*session, *request);
        app.apply(Action::Backend(Event::Connected { session, request, driver: Driver::Postgres }));
        tab
    }

    #[test]
    fn postgres_tabs_load_their_database_list() {
        let (mut app, _dir) = app();
        let tab = connected_postgres(&mut app);
        let (session, request) = app
            .backend
            .sent
            .iter()
            .find_map(|c| match c {
                Command::ListDatabases { session, request } => Some((*session, *request)),
                _ => None,
            })
            .expect("ListDatabases");
        app.apply(Action::Backend(Event::Databases {
            session,
            request,
            result: Ok(vec!["app".into(), "other".into()]),
        }));
        assert_eq!(app.workspace(tab).unwrap().databases.value.as_ref().unwrap().len(), 2);
    }

    #[test]
    fn switching_database_reconnects_with_a_fresh_tree() {
        let (mut app, _dir) = app();
        let tab = connected_postgres(&mut app);
        app.apply(Action::SwitchDatabase { tab, database: "other".into() });
        let workspace = app.workspace(tab).unwrap();
        assert_eq!(workspace.spec.database, "other");
        assert!(workspace.tree.schemas.value.is_none() && workspace.objects.is_empty());
        match app.backend.sent.last() {
            Some(Command::Connect { spec, .. }) => assert_eq!(spec.database, "other"),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn a_failed_database_switch_can_switch_back() {
        let (mut app, _dir) = app();
        let tab = connected_postgres(&mut app);
        app.apply(Action::SwitchDatabase { tab, database: "forbidden".into() });
        let Some(Command::Connect { session, request, .. }) = app.backend.sent.last() else { panic!() };
        let (session, request) = (*session, *request);
        app.apply(Action::Backend(Event::ConnectFailed {
            session,
            request,
            error: Error::Connect("permission denied for database".into()),
        }));
        assert!(matches!(app.workspace(tab).unwrap().status, SessionStatus::Disconnected(_)));
        app.apply(Action::SwitchDatabase { tab, database: "app".into() });
        match app.backend.sent.last() {
            Some(Command::Connect { spec, .. }) => assert_eq!(spec.database, "app"),
            other => panic!("{other:?}"),
        }
    }
```

Change the existing `a_pasted_url_fills_the_form_or_explains_why_not` test's PostgreSQL case to MySQL:

```rust
        form(&mut app).url = "mysql://h/db".into();
        app.apply(Action::ApplyUrl);
        assert!(form(&mut app).message.as_deref().unwrap().contains("MySQL"));
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test --lib app::tests`
Expected: FAIL to compile: `no variant SetDriver`, `no field host`, `PasswordPrompt` not found.

- [ ] **Step 3: Extend the model**

In `src/model.rs`, replace `ConnectionForm` (its derive, fields, and impl) with:

```rust
/// The connection dialog's fields while it is open.
#[derive(Debug)]
pub struct ConnectionForm {
    /// `Some` when editing a saved connection, `None` for a new one.
    pub editing: Option<ConnectionId>,
    pub name: String,
    pub color: ColorTag,
    pub driver: Driver,
    pub sqlite_path: String,
    pub host: String,
    /// As typed; checked on save.
    pub port: String,
    pub user: String,
    /// Typed in this dialog only. Empty while editing means "keep the saved one".
    pub password: String,
    pub database: String,
    pub tls: TlsMode,
    pub ca_file: String,
    pub password_mode: PasswordMode,
    /// Editing a connection whose password is in the keyring.
    pub has_saved_password: bool,
    /// Text in the "Paste URL" field.
    pub url: String,
    /// A validation or URL error shown under the fields.
    pub message: Option<String>,
    pub test: TestState,
    /// The file dialog request in flight, if any.
    pub pick_request: Option<RequestId>,
}

impl Default for ConnectionForm {
    fn default() -> Self {
        Self {
            editing: None,
            name: String::new(),
            color: ColorTag::None,
            driver: Driver::Sqlite,
            sqlite_path: String::new(),
            host: "localhost".into(),
            port: String::new(),
            user: String::new(),
            password: String::new(),
            database: String::new(),
            tls: TlsMode::Prefer,
            ca_file: String::new(),
            password_mode: PasswordMode::None,
            has_saved_password: false,
            url: String::new(),
            message: None,
            test: TestState::Idle,
            pick_request: None,
        }
    }
}

impl ConnectionForm {
    pub fn from_saved(saved: &crate::connections::SavedConnection) -> Self {
        let spec = &saved.spec;
        Self {
            editing: Some(saved.id.clone()),
            name: saved.name.clone(),
            color: saved.color,
            driver: spec.driver,
            sqlite_path: spec
                .sqlite_path
                .as_ref()
                .map(|path| path.display().to_string())
                .unwrap_or_default(),
            host: spec.host.clone(),
            port: if spec.port == 0 { String::new() } else { spec.port.to_string() },
            user: spec.user.clone(),
            database: spec.database.clone(),
            tls: spec.tls,
            ca_file: spec
                .ca_file
                .as_ref()
                .map(|path| path.display().to_string())
                .unwrap_or_default(),
            password_mode: saved.password,
            has_saved_password: saved.password == PasswordMode::Keyring,
            ..Self::default()
        }
    }

    /// What to connect to, or why the fields do not describe a connection.
    pub fn to_spec(&self) -> Result<ConnectSpec, String> {
        match self.driver {
            Driver::Sqlite => {
                let path = self.sqlite_path.trim();
                if path.is_empty() {
                    return Err("Choose a SQLite file.".into());
                }
                Ok(ConnectSpec::sqlite(path))
            }
            Driver::MySql => Err("MySQL connections arrive in a later version.".into()),
            Driver::Postgres => {
                let host = self.host.trim();
                if host.is_empty() {
                    return Err("Enter a host.".into());
                }
                let port: u16 = self.port.trim().parse().map_err(|_| "Enter a port number.".to_owned())?;
                let user = self.user.trim();
                if user.is_empty() {
                    return Err("Enter a user name.".into());
                }
                let ca_file = self.ca_file.trim();
                Ok(ConnectSpec {
                    driver: Driver::Postgres,
                    host: host.to_owned(),
                    port,
                    user: user.to_owned(),
                    database: self.database.trim().to_owned(),
                    sqlite_path: None,
                    tls: self.tls,
                    ca_file: (!ca_file.is_empty()).then(|| ca_file.into()),
                    ssh: None,
                })
            }
        }
    }

    /// The connection the form describes, or why it cannot be saved.
    pub fn to_saved(&self) -> Result<crate::connections::SavedConnection, String> {
        let name = self.name.trim();
        if name.is_empty() {
            return Err("Give the connection a name.".into());
        }
        let spec = self.to_spec()?;
        let password = if spec.driver == Driver::Sqlite {
            PasswordMode::None
        } else {
            self.password_mode
        };
        Ok(crate::connections::SavedConnection {
            id: self.editing.clone().unwrap_or_else(ConnectionId::new),
            name: name.to_owned(),
            color: self.color,
            password,
            spec,
        })
    }
}

/// Asks for a password before connecting.
#[derive(Debug)]
pub struct PasswordPrompt {
    pub tab: ConnTabId,
    /// The connection's name, for the prompt's title.
    pub name: String,
    pub password: String,
    /// Save the password in the keyring.
    pub save: bool,
    /// Why the prompt appeared (no saved password, keyring locked).
    pub message: Option<String>,
}
```

Change `Dialog` to:

```rust
#[derive(Debug)]
pub enum Dialog {
    Connection(Box<ConnectionForm>),
    Password(Box<PasswordPrompt>),
}
```

Add to `Workspace`:

```rust
    pub password_mode: PasswordMode,
    /// The secrets this tab connected with (kept in memory for Reconnect).
    pub secrets: tabletist_db::Secrets,
    /// Databases on the server (PostgreSQL), for the switcher.
    pub databases: Fetch<Vec<String>>,
```

Add to `Action`:

```rust
    /// The connection dialog's driver switch.
    SetDriver(Driver),
    SubmitPassword,
    CancelPassword,
    /// Reconnect this tab to another database on the same server.
    SwitchDatabase { tab: ConnTabId, database: String },
```

Update imports in `model.rs`: `use tabletist_db::{ConnectSpec, Driver, TlsMode};` and `use crate::connections::{ColorTag, ConnectionId, PasswordMode};`.

- [ ] **Step 4: Implement the reducer**

In `src/app.rs`, add:

```rust
/// What a keyring read was for.
#[derive(Debug, Clone, Copy)]
enum SecretPurpose {
    Connect(ConnTabId),
    Test,
}
```

and a field `pending_secrets: HashMap<RequestId, SecretPurpose>` on `App` (initialised empty in `new`; `use std::collections::HashMap;`).

Replace `connect_tab` and `reconnect`, and add the helpers:

```rust
    /// Opens `saved` in `tab`. `typed` is a password typed in the dialog.
    fn connect_tab(&mut self, tab: ConnTabId, saved: SavedConnection, typed: Option<String>) {
        let session = SessionId(self.next_id());
        let request = RequestId(self.next_id());
        let Some(entry) = self.tabs.iter_mut().find(|t| t.id == tab) else {
            return;
        };
        entry.content = ConnTabContent::Workspace(Box::new(Workspace {
            session,
            conn_id: saved.id,
            name: saved.name,
            color: saved.color,
            driver: saved.spec.driver,
            spec: saved.spec,
            status: SessionStatus::Connecting { request },
            tree: Tree::default(),
            objects: Vec::new(),
            active_object: None,
            row_panel: true,
            pending_open: None,
            password_mode: saved.password,
            secrets: Secrets::default(),
            databases: Fetch::default(),
        }));
        match typed {
            Some(password) => self.send_connect(tab, Secrets { password: Some(password), ..Secrets::default() }),
            None => self.authenticate(tab),
        }
    }

    /// Finds the password this tab needs (none, keyring, or prompt), then connects.
    fn authenticate(&mut self, tab: ConnTabId) {
        let Some(workspace) = self.workspace(tab) else { return };
        match workspace.password_mode {
            PasswordMode::None => self.send_connect(tab, Secrets::default()),
            PasswordMode::Keyring => {
                let account = password_account(&workspace.conn_id);
                let request = RequestId(self.next_id());
                self.pending_secrets.insert(request, SecretPurpose::Connect(tab));
                self.backend.send(Command::LoadSecret { request, account });
            }
            PasswordMode::Ask => self.prompt_password(tab, None),
        }
    }

    /// Sends the Connect for the tab's current session and request.
    fn send_connect(&mut self, tab: ConnTabId, secrets: Secrets) {
        let Some(workspace) = self.workspace_mut(tab) else { return };
        let SessionStatus::Connecting { request } = workspace.status else { return };
        workspace.secrets = secrets.clone();
        let (session, spec) = (workspace.session, workspace.spec.clone());
        self.backend.send(Command::Connect { session, request, spec, secrets });
    }

    fn prompt_password(&mut self, tab: ConnTabId, message: Option<String>) {
        let Some(workspace) = self.workspace(tab) else { return };
        self.dialog = Some(Dialog::Password(Box::new(PasswordPrompt {
            tab,
            name: workspace.name.clone(),
            password: String::new(),
            save: workspace.password_mode == PasswordMode::Keyring,
            message,
        })));
    }

    fn reconnect(&mut self, tab: ConnTabId) {
        let session = SessionId(self.next_id());
        let request = RequestId(self.next_id());
        let Some(workspace) = self.workspace_mut(tab) else { return };
        let old = std::mem::replace(&mut workspace.session, session);
        workspace.status = SessionStatus::Connecting { request };
        let known = workspace.secrets.password.is_some() || workspace.password_mode == PasswordMode::None;
        let secrets = workspace.secrets.clone();
        self.backend.send(Command::Close { session: old });
        if known {
            self.send_connect(tab, secrets);
        } else {
            self.authenticate(tab);
        }
    }
```

Update the `Action::Connect` arm to `self.connect_tab(tab, saved, None);`.

Add reducer arms:

```rust
            Action::SetDriver(driver) => {
                if let Some(Dialog::Connection(form)) = &mut self.dialog {
                    form.driver = driver;
                    if driver != Driver::Sqlite {
                        if form.port.trim().is_empty() {
                            form.port = driver.default_port().to_string();
                        }
                        if form.password_mode == PasswordMode::None {
                            form.password_mode = PasswordMode::Keyring;
                        }
                    }
                }
            }
            Action::SubmitPassword => {
                let Some(Dialog::Password(prompt)) = &mut self.dialog else { return };
                if prompt.password.is_empty() {
                    prompt.message = Some("Enter the password.".into());
                    return;
                }
                let (tab, password, save) = (prompt.tab, prompt.password.clone(), prompt.save);
                self.dialog = None;
                if save
                    && let Some(workspace) = self.workspace_mut(tab)
                {
                    workspace.password_mode = PasswordMode::Keyring;
                    let conn = workspace.conn_id.clone();
                    let request = RequestId(self.next_id());
                    self.backend.send(Command::StoreSecret {
                        request,
                        account: password_account(&conn),
                        secret: Some(SecretString(password.clone())),
                    });
                    if let Some(saved) = self.connections.connections.iter_mut().find(|c| c.id == conn) {
                        saved.password = PasswordMode::Keyring;
                        self.save_connections();
                    }
                }
                self.send_connect(tab, Secrets { password: Some(password), ..Secrets::default() });
            }
            Action::CancelPassword => {
                if let Some(Dialog::Password(prompt)) = self.dialog.take()
                    && let Some(workspace) = self.workspace_mut(prompt.tab)
                {
                    workspace.status =
                        SessionStatus::Disconnected(Error::Auth("a password is needed to connect".into()));
                }
            }
            Action::SwitchDatabase { tab, database } => {
                if let Some(workspace) = self.workspace_mut(tab) {
                    workspace.spec.database = database;
                    workspace.tree = Tree::default();
                    workspace.objects.clear();
                    workspace.active_object = None;
                }
                self.reconnect(tab);
            }
```

(`Error` here is `tabletist_db::Error`; add it to the `tabletist_db` import. `SubmitPassword`'s early `return`s are inside `apply`, which returns `()`.)

Add to `apply_event`:

```rust
            Event::SecretLoaded { request, result } => match self.pending_secrets.remove(&request) {
                Some(SecretPurpose::Connect(tab)) => match result {
                    Ok(Some(secret)) => {
                        self.send_connect(tab, Secrets { password: Some(secret.0), ..Secrets::default() });
                    }
                    Ok(None) => self.prompt_password(
                        tab,
                        Some("No password is saved in the keyring for this connection.".into()),
                    ),
                    Err(message) => self.prompt_password(
                        tab,
                        Some(format!("Could not read the saved password: {message}.")),
                    ),
                },
                Some(SecretPurpose::Test) => {
                    let Some(Dialog::Connection(form)) = &mut self.dialog else { return };
                    if form.test != TestState::Running(request) {
                        return;
                    }
                    match (result, form.to_spec()) {
                        (Ok(Some(secret)), Ok(spec)) => {
                            let secrets = Secrets { password: Some(secret.0), ..Secrets::default() };
                            self.backend.send(Command::Test { request, spec, secrets });
                        }
                        (Ok(None), _) => form.test = TestState::Failed("No password is saved; type it to test.".into()),
                        (Err(message), _) => form.test = TestState::Failed(message),
                        (_, Err(message)) => form.test = TestState::Failed(message),
                    }
                }
                None => {}
            },
            Event::SecretStored { .. } => {}
            Event::Databases { session, request, result } => {
                if let Some(tab) = self.tab_for_session(session)
                    && let Some(workspace) = self.workspace_mut(tab)
                {
                    workspace.databases.finish(request, result);
                }
            }
```

In `after_connect`, before `self.refresh_tree(tab);`, load the database list for PostgreSQL:

```rust
        if let Some(workspace) = self.workspace(tab)
            && workspace.driver == Driver::Postgres
        {
            let session = workspace.session;
            let request = RequestId(self.next_id());
            if let Some(workspace) = self.workspace_mut(tab) {
                workspace.databases.start(request);
            }
            self.backend.send(Command::ListDatabases { session, request });
        }
```

Replace `TestConnection`:

```rust
            Action::TestConnection => {
                let request = RequestId(self.next_id());
                let Some(Dialog::Connection(form)) = &mut self.dialog else { return };
                let spec = match form.to_spec() {
                    Ok(spec) => spec,
                    Err(message) => {
                        form.message = Some(message);
                        return;
                    }
                };
                form.message = None;
                form.test = TestState::Running(request);
                // An unchanged keyring password is read first; the test then
                // runs under the same request id.
                if form.password.is_empty()
                    && form.has_saved_password
                    && let Some(id) = form.editing.clone()
                {
                    self.pending_secrets.insert(request, SecretPurpose::Test);
                    self.backend.send(Command::LoadSecret { request, account: password_account(&id) });
                } else {
                    let password = (!form.password.is_empty()).then(|| form.password.clone());
                    let secrets = Secrets { password, ..Secrets::default() };
                    self.backend.send(Command::Test { request, spec, secrets });
                }
            }
```

Replace `DeleteConnection`:

```rust
            Action::DeleteConnection(id) => {
                if let Some(removed) = self.connections.remove(&id) {
                    self.save_connections();
                    if removed.password == PasswordMode::Keyring {
                        let request = RequestId(self.next_id());
                        self.backend.send(Command::StoreSecret { request, account: password_account(&id), secret: None });
                    }
                }
            }
```

In `save_dialog`, after the `let saved = match form.to_saved() ...` block, capture the typed password and the previous mode, and after `self.save_connections();` update the keyring before connecting:

```rust
        let typed = (!form.password.is_empty()).then(|| form.password.clone());
        let previous = self.connections.get(&saved.id).map(|c| c.password);
        self.dialog = None;
        self.connections.upsert(saved.clone());
        self.save_connections();
        let account = password_account(&saved.id);
        if saved.password == PasswordMode::Keyring {
            if let Some(password) = &typed {
                let request = RequestId(self.next_id());
                self.backend.send(Command::StoreSecret {
                    request,
                    account,
                    secret: Some(SecretString(password.clone())),
                });
            }
        } else if previous == Some(PasswordMode::Keyring) {
            let request = RequestId(self.next_id());
            self.backend.send(Command::StoreSecret { request, account, secret: None });
        }
        if connect {
            if !matches!(self.active_tab().content, ConnTabContent::Picker(_)) {
                self.apply(Action::NewConnTab);
            }
            let tab = self.active_tab_id();
            self.connect_tab(tab, saved, typed);
        }
```

(Remove the old `self.dialog = None; self.connections.upsert(..); self.save_connections(); if connect { .. }` lines this replaces. `typed` must be read before `self.dialog = None` drops the form.)

Replace `apply_url`:

```rust
/// Fills the form from its URL field.
fn apply_url(form: &mut ConnectionForm) {
    let (spec, secrets) = match tabletist_db::ConnectSpec::from_url(&form.url) {
        Ok(parsed) => parsed,
        Err(error) => {
            form.message = Some(error.to_string());
            return;
        }
    };
    match spec.driver {
        Driver::Sqlite => {
            let path = spec.sqlite_path.as_ref().map(|path| path.display().to_string()).unwrap_or_default();
            if form.name.trim().is_empty() {
                form.name = crate::model::file_name(&path);
            }
            form.driver = Driver::Sqlite;
            form.sqlite_path = path;
            form.message = None;
        }
        Driver::MySql => {
            form.message = Some("MySQL connections arrive in a later version.".into());
        }
        Driver::Postgres => {
            if form.name.trim().is_empty() {
                form.name = spec.summary();
            }
            form.driver = Driver::Postgres;
            form.host = spec.host;
            form.port = spec.port.to_string();
            form.user = spec.user;
            form.database = spec.database;
            form.tls = spec.tls;
            form.ca_file = spec.ca_file.map(|path| path.display().to_string()).unwrap_or_default();
            if let Some(password) = secrets.password {
                form.password = password;
            }
            if form.password_mode == PasswordMode::None {
                form.password_mode = PasswordMode::Keyring;
            }
            form.message = None;
        }
    }
}
```

Add the imports `use crate::connections::PasswordMode;`, `use crate::secrets::{SecretString, password_account};`, `Fetch` and `PasswordPrompt` from `crate::model`, and `Driver`, `Error` from `tabletist_db` (merge into the existing `use` lines).

In `src/testing.rs`, `connect_fake` builds `SavedConnection` with `password: PasswordMode::None` (from Task 6), so it connects without a keyring read.

- [ ] **Step 5: Run the tests to verify they pass**

Run: `cargo test --locked --workspace --all-targets`
Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add src
git commit -m "Connect with saved or prompted passwords and switch PostgreSQL databases"
```

---

### Task 8: The dialog's PostgreSQL fields, the password prompt, and the top bar

**Files:**
- Modify: `src/ui/connect_dialog.rs` (replace), `src/ui/workspace.rs`, `src/ui/mod.rs`
- Create: `src/ui/password_prompt.rs`

**Interfaces:**
- Consumes: Task 7's form fields and actions.
- Produces: `ui::password_prompt::show(app: &mut App, ctx: &egui::Context)`; the dialog's driver buttons labelled "SQLite", "PostgreSQL", "MySQL"; the prompt's buttons "Connect" and "Cancel" and checkbox "Save in keyring"; the top bar's database list labelled "Database".

- [ ] **Step 1: Write the failing UI tests**

Add to the tests in `src/ui/mod.rs`:

```rust
    #[test]
    fn choosing_postgres_shows_its_fields() {
        let mut harness = Harness::new();
        harness.press(Key::N, Modifiers::COMMAND);
        assert!(!harness.has("Host"));
        harness.click("PostgreSQL");
        for label in ["Host", "Port", "User", "Password", "Database", "TLS"] {
            assert!(harness.has(label), "{label}");
        }
        assert!(!harness.has("File"));
    }

    #[test]
    fn the_password_prompt_connects_with_the_typed_password() {
        let mut harness = Harness::new();
        let (spec, _) = tabletist_db::ConnectSpec::from_url("postgres://me@db/app").unwrap();
        let saved = crate::connections::SavedConnection {
            id: crate::connections::ConnectionId::new(),
            name: "Prod".into(),
            color: crate::connections::ColorTag::Red,
            password: crate::connections::PasswordMode::Ask,
            spec,
        };
        let conn = saved.id.clone();
        harness.app.connections.upsert(saved);
        let tab = harness.app.active_tab_id();
        harness.app.apply(crate::model::Action::Connect { tab, conn });
        assert!(harness.has("Password for Prod"));
        if let Some(crate::model::Dialog::Password(prompt)) = &mut harness.app.dialog {
            prompt.password = "typed".into();
        }
        harness.click("Connect");
        match crate::testing::last_sent(&harness.app) {
            Command::Connect { secrets, .. } => assert_eq!(secrets.password.as_deref(), Some("typed")),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn escape_cancels_the_password_prompt() {
        let mut harness = Harness::new();
        let (spec, _) = tabletist_db::ConnectSpec::from_url("postgres://me@db/app").unwrap();
        let saved = crate::connections::SavedConnection {
            id: crate::connections::ConnectionId::new(),
            name: "Prod".into(),
            color: crate::connections::ColorTag::None,
            password: crate::connections::PasswordMode::Ask,
            spec,
        };
        let conn = saved.id.clone();
        harness.app.connections.upsert(saved);
        let tab = harness.app.active_tab_id();
        harness.app.apply(crate::model::Action::Connect { tab, conn });
        harness.press(Key::Escape, Modifiers::NONE);
        assert!(harness.app.dialog.is_none());
        assert!(harness.has("Reconnect"));
    }
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test --lib ui::tests`
Expected: FAIL (no driver buttons, no prompt view).

- [ ] **Step 3: Implement the dialog**

Replace `src/ui/connect_dialog.rs`:

```rust
//! The connection dialog: new or edit, SQLite or PostgreSQL.

use egui::{RichText, TextEdit};
use tabletist_db::{Driver, TlsMode};

use crate::app::App;
use crate::connections::{ColorTag, PasswordMode};
use crate::i18n::gettext;
use crate::model::{Action, Dialog, TestState};
use crate::theme;

const TLS_MODES: [(TlsMode, &str); 5] = [
    (TlsMode::Disable, "Off"),
    (TlsMode::Prefer, "Prefer (not verified)"),
    (TlsMode::Require, "Require (not verified)"),
    (TlsMode::VerifyCa, "Verify certificate"),
    (TlsMode::VerifyFull, "Verify certificate and host"),
];

fn tls_label(mode: TlsMode) -> &'static str {
    TLS_MODES.iter().find(|(m, _)| *m == mode).map_or("", |(_, label)| label)
}

pub fn show(app: &mut App, ctx: &egui::Context) {
    let locale = app.locale;
    let palette = app.palette;
    let Some(Dialog::Connection(form)) = &mut app.dialog else {
        return;
    };
    let mut actions = Vec::new();
    let title = if form.editing.is_some() {
        gettext(locale, "Edit connection")
    } else {
        gettext(locale, "New connection")
    };
    // A modal: nothing behind it can be clicked while it is open.
    let modal = egui::Modal::new(egui::Id::new("connection-dialog")).show(ctx, |ui| {
        ui.set_width(480.0);
        ui.label(RichText::new(title.as_ref()).font(theme::semibold(16.0)).color(palette.text));
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            for (driver, enabled) in [(Driver::Sqlite, true), (Driver::Postgres, true), (Driver::MySql, false)] {
                let button = egui::Button::selectable(form.driver == driver, driver.label());
                let response = ui.add_enabled(enabled, button);
                let response = if enabled {
                    response
                } else {
                    response.on_disabled_hover_text(gettext(locale, "Coming in a later version"))
                };
                if response.clicked() && form.driver != driver {
                    actions.push(Action::SetDriver(driver));
                }
            }
        });
        ui.add_space(6.0);
        egui::Grid::new("connection-form")
            .num_columns(2)
            .spacing([12.0, 8.0])
            .show(ui, |ui| {
                ui.label(gettext(locale, "Name"));
                ui.add(TextEdit::singleline(&mut form.name).desired_width(f32::INFINITY));
                ui.end_row();

                ui.label(gettext(locale, "Color"));
                egui::ComboBox::from_id_salt("color-tag")
                    .selected_text(form.color.label())
                    .show_ui(ui, |ui| {
                        for tag in ColorTag::ALL {
                            ui.selectable_value(&mut form.color, tag, tag.label());
                        }
                    });
                ui.end_row();

                if form.driver == Driver::Sqlite {
                    ui.label(gettext(locale, "File"));
                    ui.horizontal(|ui| {
                        ui.add(TextEdit::singleline(&mut form.sqlite_path).desired_width(300.0));
                        if ui.button(gettext(locale, "Choose…")).clicked() {
                            actions.push(Action::PickSqliteFile);
                        }
                    });
                    ui.end_row();
                } else {
                    ui.label(gettext(locale, "Host"));
                    ui.horizontal(|ui| {
                        ui.add(TextEdit::singleline(&mut form.host).desired_width(250.0));
                        ui.label(gettext(locale, "Port"));
                        ui.add(TextEdit::singleline(&mut form.port).desired_width(60.0));
                    });
                    ui.end_row();

                    ui.label(gettext(locale, "User"));
                    ui.add(TextEdit::singleline(&mut form.user).desired_width(f32::INFINITY));
                    ui.end_row();

                    ui.label(gettext(locale, "Password"));
                    let hint = if form.has_saved_password {
                        gettext(locale, "Saved in keyring")
                    } else {
                        gettext(locale, "")
                    };
                    ui.add(
                        TextEdit::singleline(&mut form.password)
                            .password(true)
                            .hint_text(hint)
                            .desired_width(f32::INFINITY),
                    );
                    ui.end_row();

                    ui.label("");
                    egui::ComboBox::from_id_salt("password-mode")
                        .selected_text(match form.password_mode {
                            PasswordMode::Keyring => gettext(locale, "Save in keyring"),
                            PasswordMode::Ask => gettext(locale, "Ask every time"),
                            PasswordMode::None => gettext(locale, "No password"),
                        })
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut form.password_mode, PasswordMode::Keyring, gettext(locale, "Save in keyring"));
                            ui.selectable_value(&mut form.password_mode, PasswordMode::Ask, gettext(locale, "Ask every time"));
                            ui.selectable_value(&mut form.password_mode, PasswordMode::None, gettext(locale, "No password"));
                        });
                    ui.end_row();

                    ui.label(gettext(locale, "Database"));
                    ui.add(
                        TextEdit::singleline(&mut form.database)
                            .hint_text(gettext(locale, "Same as the user"))
                            .desired_width(f32::INFINITY),
                    );
                    ui.end_row();

                    ui.label(gettext(locale, "TLS"));
                    egui::ComboBox::from_id_salt("tls-mode")
                        .selected_text(tls_label(form.tls))
                        .show_ui(ui, |ui| {
                            for (mode, label) in TLS_MODES {
                                ui.selectable_value(&mut form.tls, mode, label);
                            }
                        });
                    ui.end_row();

                    if matches!(form.tls, TlsMode::VerifyCa | TlsMode::VerifyFull) {
                        ui.label(gettext(locale, "CA file"));
                        ui.add(
                            TextEdit::singleline(&mut form.ca_file)
                                .hint_text(gettext(locale, "System certificates"))
                                .desired_width(f32::INFINITY),
                        );
                        ui.end_row();
                    }
                }

                ui.label(gettext(locale, "Paste URL"));
                ui.horizontal(|ui| {
                    let response = ui.add(
                        TextEdit::singleline(&mut form.url)
                            .hint_text("postgres://user@host/db")
                            .desired_width(300.0),
                    );
                    let entered = response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                    if ui.button(gettext(locale, "Fill")).clicked() || entered {
                        actions.push(Action::ApplyUrl);
                    }
                });
                ui.end_row();
            });

        if let Some(message) = &form.message {
            ui.add_space(4.0);
            ui.label(RichText::new(message).color(palette.danger));
        }
        match &form.test {
            TestState::Idle => {}
            TestState::Running(_) => {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label(gettext(locale, "Testing…"));
                });
            }
            TestState::Passed => {
                ui.label(RichText::new(gettext(locale, "Connection works.")).color(palette.accent));
            }
            TestState::Failed(message) => {
                ui.label(RichText::new(message).color(palette.danger));
            }
        }

        ui.add_space(8.0);
        ui.horizontal(|ui| {
            if ui.button(gettext(locale, "Test")).clicked() {
                actions.push(Action::TestConnection);
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let connect = egui::Button::new(
                    RichText::new(gettext(locale, "Save & Connect"))
                        .font(theme::medium(13.5))
                        .color(palette.on_accent),
                )
                .fill(palette.accent);
                if ui.add(connect).clicked() {
                    actions.push(Action::SaveConnection { connect: true });
                }
                if ui.button(gettext(locale, "Save")).clicked() {
                    actions.push(Action::SaveConnection { connect: false });
                }
                if ui.button(gettext(locale, "Cancel")).clicked() {
                    actions.push(Action::CloseDialog);
                }
            });
        });
    });
    // Escape closes the dialog, unless it is closing an open list first.
    if modal.is_top_modal
        && !modal.any_popup_open
        && ctx.input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::Escape))
    {
        actions.push(Action::CloseDialog);
    }
    app.actions.extend(actions);
}
```

(`gettext(locale, "")` returns an empty `Cow`; if `egui::Button::selectable` does not exist in this egui version, use `egui::Button::new(driver.label()).selected(form.driver == driver)`.)

- [ ] **Step 4: Implement the prompt and the top bar**

Create `src/ui/password_prompt.rs`:

```rust
//! Asks for a connection's password.

use egui::{RichText, TextEdit};

use crate::app::App;
use crate::i18n::gettext;
use crate::model::{Action, Dialog};
use crate::theme;

pub fn show(app: &mut App, ctx: &egui::Context) {
    let locale = app.locale;
    let palette = app.palette;
    let Some(Dialog::Password(prompt)) = &mut app.dialog else {
        return;
    };
    let mut actions = Vec::new();
    let modal = egui::Modal::new(egui::Id::new("password-prompt")).show(ctx, |ui| {
        ui.set_width(360.0);
        ui.label(
            RichText::new(format!("{} {}", gettext(locale, "Password for"), prompt.name))
                .font(theme::semibold(15.0))
                .color(palette.text),
        );
        if let Some(message) = &prompt.message {
            ui.label(RichText::new(message).color(palette.secondary));
        }
        ui.add_space(6.0);
        let field = ui.add(TextEdit::singleline(&mut prompt.password).password(true).desired_width(f32::INFINITY));
        if !field.has_focus() && prompt.password.is_empty() {
            field.request_focus();
        }
        let entered = field.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
        ui.checkbox(&mut prompt.save, gettext(locale, "Save in keyring"));
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button(gettext(locale, "Connect")).clicked() || entered {
                    actions.push(Action::SubmitPassword);
                }
                if ui.button(gettext(locale, "Cancel")).clicked() {
                    actions.push(Action::CancelPassword);
                }
            });
        });
    });
    if modal.is_top_modal
        && !modal.any_popup_open
        && ctx.input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::Escape))
    {
        actions.push(Action::CancelPassword);
    }
    app.actions.extend(actions);
}
```

In `src/ui/mod.rs`, add `pub mod password_prompt;` and call `password_prompt::show(app, &ui.ctx().clone());` next to `connect_dialog::show(..)`.

In `src/ui/workspace.rs` `top_bar`, collect the switcher's data before the panel:

```rust
    let databases = workspace.databases.value.clone().unwrap_or_default();
    let current = workspace.spec.database.clone();
    let tls = (workspace.driver != tabletist_db::Driver::Sqlite).then_some(workspace.spec.tls);
```

and inside the `right_to_left` layout, after the Disconnect button:

```rust
                    if let Some(mode) = tls {
                        use tabletist_db::TlsMode;
                        let (text, color) = match mode {
                            TlsMode::Disable => (gettext(locale, "Not encrypted"), palette.warning),
                            TlsMode::Prefer => (gettext(locale, "TLS if offered"), palette.secondary),
                            TlsMode::Require => (gettext(locale, "TLS"), palette.secondary),
                            TlsMode::VerifyCa | TlsMode::VerifyFull => (gettext(locale, "TLS verified"), palette.secondary),
                        };
                        ui.label(RichText::new(text).color(color));
                    }
                    if databases.len() > 1 {
                        let mut chosen = current.clone();
                        egui::ComboBox::from_id_salt(("database", tab.0))
                            .selected_text(&chosen)
                            .show_ui(ui, |ui| {
                                for database in &databases {
                                    ui.selectable_value(&mut chosen, database.clone(), database);
                                }
                            })
                            .response
                            .on_hover_text(gettext(locale, "Database"));
                        if chosen != current {
                            actions.push(Action::SwitchDatabase { tab, database: chosen });
                        }
                    }
```

- [ ] **Step 5: Run the tests to verify they pass**

Run: `cargo test --locked --workspace --all-targets && cargo clippy --locked --workspace --all-targets -- -D warnings`
Expected: all pass.

- [ ] **Step 6: Commit**

```bash
git add src
git commit -m "Add PostgreSQL fields, the password prompt and the database switcher"
```

---

### Task 9: CI, docs, and a real-server check

**Files:**
- Modify: `.github/workflows/ci.yml`, `AGENTS.md`

**Interfaces:**
- Consumes: everything above.
- Produces: a CI job that runs the PostgreSQL suite against `compose.yaml`, and the native keyring round trip on macOS and Windows.

- [ ] **Step 1: Add the CI job and keyring step**

Add to `.github/workflows/ci.yml` under `jobs:`:

```yaml
  postgres:
    name: postgres integration
    runs-on: ubuntu-latest
    timeout-minutes: 30
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2
      - name: Start PostgreSQL
        run: |
          docker compose up -d postgres
          for attempt in $(seq 1 60); do
            pg_isready -h localhost -p 55432 && break
            sleep 1
          done
      - run: cargo test --locked -p tabletist-db --test postgres
        env:
          TABLETIST_TEST_PG_URL: postgres://tabletist:tabletist@localhost:55432/tabletist
```

(`pg_isready` over TCP only answers once the entrypoint's final server is up, after the init phase.)

Add to the `test` job's steps, after `cargo test`:

```yaml
      - name: Native keyring round trip
        if: runner.os == 'Windows' || runner.os == 'macOS'
        timeout-minutes: 2
        run: cargo test --locked --lib secrets::tests::native_store_round_trip -- --ignored --exact
```

Add to `AGENTS.md` under "Checks":

```markdown
Database integration tests need servers. `docker compose up -d postgres`, then
`TABLETIST_TEST_PG_URL=postgres://tabletist:tabletist@localhost:55432/tabletist cargo test -p tabletist-db --test postgres`.
Without the variable those tests print "skipped" and pass; CI runs them on Linux.
The native keyring test is `#[ignore]`d; run it by hand on a desktop session.
```

- [ ] **Step 2: Check against a real server**

Run: `docker compose up -d postgres` and wait for `pg_isready -h localhost -p 55432`.
Run: `TABLETIST_TEST_PG_URL=postgres://tabletist:tabletist@localhost:55432/tabletist cargo test --locked --workspace --all-targets`
Expected: PASS, including 19 PostgreSQL tests.

Run: `cargo run`, then New connection, PostgreSQL, host `localhost`, port `55432`, user `tabletist`, password `tabletist`, database `tabletist`, Save & Connect.
Expected, by hand: the tree shows `public` (expanded) and `billing`; `users` opens with typed values (`true`, bytes, `{math,engines}`); the Structure view lists `users_email_key` and the comment on `email`; the top bar shows "Not encrypted" or "TLS if offered" and a database list with `postgres` and `tabletist`; choosing `postgres` reconnects with an empty tree; quitting and reopening connects without asking for the password (it is in the keyring); editing the connection to "Ask every time" removes it from the keyring (check with `secret-tool search service dev.tabletist.Tabletist` on Linux).

- [ ] **Step 3: Full checks and commit**

Run: `cargo fmt --all --check && cargo clippy --locked --workspace --all-targets -- -D warnings && cargo test --locked --workspace --all-targets && RUSTDOCFLAGS='-D warnings' cargo doc --locked --workspace --no-deps`
Expected: all pass.

```bash
git add .github AGENTS.md
git commit -m "Run the PostgreSQL suite and the keyring round trip in CI"
```

---

## Done when

- A saved PostgreSQL connection (password in the keyring or asked for) opens in a tab; the tree, grid, paging, sort, row panel and Structure view work as on SQLite.
- Every TLS mode behaves like libpq; `verify-full` refuses the compose server's self-signed certificate.
- Nothing typed into a filter or raw WHERE can write or leave read-only mode.
- The database switcher reconnects the tab to another database.
- CI runs the PostgreSQL suite on Linux and the keyring round trip on macOS and Windows.

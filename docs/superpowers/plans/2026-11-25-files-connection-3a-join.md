# Files connection 3a: join with saved connections

> **For agentic workers:** REQUIRED SUB-SKILL: Use rubypowers:subagent-driven-development (recommended) or rubypowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A Files connection attaches saved PostgreSQL, MySQL and SQLite connections, read-only, so one query joins a file to their live tables, as the prompt's "Join with saved connections" asks. The section is in the dialog in both looks, the attachments are kept in `connections.toml`, and Test and opening attach them with passwords the backend reads from the keyring at attach time. Sidebar badges, the list row's joins and agents through attachments are run 3b.

**Architecture:**
- **The engine** (`crates/tabletist-db`):
  - `files::Attach` (the saved connection's id and the name SQL calls it by) is kept on `ConnectSpec::attach`.
  - `files::Attaching` (the name, the saved connection's spec and its secrets) is what a session attaches. It prints without its secrets.
  - `Connection::connect_files` and `Connection::open_files_to_test_with` open each attachment's SSH tunnel, if it has one.
  - `duck::Conn::open` takes the attachments and a `Downloading` callback.
  - Before the fence, `duck/attach.rs` builds each attachment's statements. A PostgreSQL or MySQL login goes into a `CREATE SECRET` of the session's own, dropped right after the `ATTACH '<connection string>' AS name (TYPE …, SECRET …, READ_ONLY)`, which holds no password. A SQLite file is attached by its path, which the fence then lets in.
  - The fence (`allowed_paths`, `enable_external_access = false`, `lock_configuration = true`) keeps any other database from being attached after.
  - The readers install on first use (`INSTALL postgres; LOAD postgres`). `Downloading` is told first, when `duck_extensions()` says the reader isn't installed yet.
  - An attachment is one sidebar schema: `list_schemas` adds the attached names. `catalog_of` maps it to its default schema (PostgreSQL `public`, MySQL's database, SQLite `main`), which `name.table` reads.
- **The core** (`crates/tabletist-core`):
  - `connections::join` lists the saved connections the section offers (`Joinable`, with why one cannot be ticked), names a new attachment, and checks the names.
  - `connections.toml` gains `[[connections.attach]]` (version 10).
  - `Command::Connect` and `Command::Test` carry `attach: Vec<AttachTo>`: the saved connections as they are now, `None` for one deleted. The backend reads each password from the keyring (`backend/attach.rs`) as it opens. No password is read on the UI thread.
  - `Event::FilesDownloading` tells the dialog which reader DuckDB is downloading.
  - The form keeps `files_attach`, `join_open`, `files_downloading` and `joinable`. The last is read again each frame, as Group's choices are.
  - `Action::ToggleAttach` attaches a saved connection under its default name, or lets it go.
- **The view:**
  - `src/ui/attach_list.rs` (new) draws the attach rows, a shared component. macOS shows a box, the environment's stripe and badge, the name and database, then the SQL name field, "not attached", or why the row can't be ticked. The terminal look shows `[x] DEV Bookshop as [bookshop_dev] · read-only`.
  - `connect_dialog/join.rs` (new) is the section, after Reading CSV in both looks.
  - `connect_dialog/mod.rs` moves Up and Down between the rows while a row's box has the keyboard, and says the download in the Test status.

**Tech Stack:** Rust, DuckDB (`duckdb` 1.20000.1, its `postgres`, `mysql` and `sqlite` readers installed on first use), egui (`egui_kittest` headless tests), gettext catalogs (`assets/i18n`).

**Base:** `main` at `a2307ada`, branch `claude/files-connection-3-join`. Run 2 (2a to 2d) is merged.

---

## What was decided

On 2026-11-25 Igor picked these, with the recommended option each time:

| Question | Decision |
| --- | --- |
| How an attached connection shows in the sidebar | **One schema each**, named as SQL calls it, listing its default schema's tables. Its other schemas only in SQL (`bookshop_dev.billing.invoices`). |
| Which saved connections can be attached | **Keyring passwords and SSH tunnels.** A connection whose password is asked for on every connect is listed, but can't be ticked, and says why. |
| What agents may reach through an attachment | **Only what its own setting allows**: Read, with no hidden tables or masked columns (3b). |
| Delivery | **Two PRs**: 3a this plan; 3b the sidebar badges, the list row's joins and agents. |

A probe before planning showed, on the test servers:
- an attachment made before the fence still answers after it;
- the servers refuse writes sent through `postgres_execute` and `mysql_execute`, since a `READ_ONLY` attachment runs in read-only transactions;
- `duckdb_secrets()` shows `password=redacted`, and a wrong password's error doesn't carry it;
- DuckDB quotes a failed statement in its parser errors. So a failed `CREATE SECRET`, which holds the password, is never shown in DuckDB's words.

A debug build of DuckDB aborts the first time it runs one of the readers it downloads: its own allocation checks count theirs ("allocation_count >= size"). The dev profile now builds `libduckdb-sys` without debug info (`DEBUG` off, so `NDEBUG`), as a release build is (`Cargo.toml`).

Canvas (canvas version 218): **"macOS – Files, join"** and **"Omarchy – Files, join"** are new. Each draws:
- a name typed over, and the sample query;
- rows that can't be ticked;
- nothing to attach;
- the closed section with names;
- Test downloading a reader;
- Test failing to attach.

The section open is on "macOS – Connection, Files, details" and "Omarchy – Connection, Files, details and Test". The attach row is on "macOS – Components, additions for Files".

### Where this plan does more, or less, than the prompt or the canvas

| The prompt or canvas | What is built | Why |
| --- | --- | --- |
| The boards: `bookshop_development` attached "as bookshop_dev" | Its database's name: `bookshop_development`, typed over to shorten it. A SQLite file's stem (`development`). The name is made plain as a file's table name is: lowercase, anything but letters, digits and `_` turned into `_` (`Development-2.sqlite3` is `development_2`). `_2` and on where the name is taken. | The prompt: "defaults to the connection's database name … otherwise the database name as is". The prompt wins. |
| "It can be edited in place" | A ticked row's name is a field: on macOS in the right column, in the terminal look after `as`, the next Tab stop after the box. Save (and Test) says what is wrong with one: not lowercase letters, digits and `_`, one of DuckDB's own, a schema the connection has, or given twice. | The boards draw the name as text. Drawn on the new boards. |
| The sample query `JOIN bookshop_dev.public.publishers p` | `JOIN bookshop_dev.publishers p`. | `name.table` reads the default schema on all three engines, and MySQL and SQLite have no `public`. Drawn on the new board. |
| "Production, or any connection set to open read-only, is always attached read-only, even when the files are editable. Say so under the list." | Every attachment is read-only, and the line under the list says so: "Attached connections open read-only here, as the files do." | Files can't be edited until run 4, which decides what a writable Files connection attaches writable, and says production's rule then. A line about production alone would say the others are writable. |
| The attach list | A row that can't be ticked says why: its password (or SSH secret) is asked for on every connect, or a MySQL connection names no database. It still takes the keyboard, so the arrows pass through it and a screen reader says it disabled. With nothing to list: "No saved PostgreSQL, MySQL or SQLite connection to attach." | Decided above, and MySQL's attach needs a database to read `name.table` from. Drawn on the new boards. |
| The attach row: "checkbox, environment stripe and badge, name, database" | The terminal look's row is `[x] DEV Bookshop · bookshop_development as [bookshop_dev]`: the badge carries the environment's colour, with no stripe. Both looks show the hint and the sample query. | The terminal board draws no stripe. Drawn on the new Omarchy board. |
| — | A name is checked against the connection's `.duckdb` schemas as the Source was last read. Saved before it was read, a clash is said when the connection opens, in DuckDB's words. | The dialog reads the Source as soon as it is chosen. |
| — | A connection that can't be attached fails the open and the Test: "Can't attach bookshop_dev: password authentication failed for user …", in the server's words. One whose saved connection was deleted says so, and is let go the next time the Files connection is edited. | A query that names it would fail anyway, with less to go on. Drawn on the new boards. |
| Open question 1: "show 'Downloading the PostgreSQL reader for DuckDB…' in the Test status" | The Test status says it for each reader DuckDB downloads: Excel's too (run 1 installed it without saying so). Opening says "Connecting…" as it does today. | The prompt's default, for the readers it names. |
| — | An attached connection's SSH host key must be trusted already: open that connection once on its own first. An unknown key fails the attach, with the tunnel's words. | The trust prompt belongs to the connection being opened. |
| — | Read-only three ways: DuckDB's `READ_ONLY` attach, its readers' read-only transactions (`postgres_execute` and `mysql_execute` included, `use_transaction := false` too), and PostgreSQL's session `default_transaction_read_only = on`. Each login's secret is dropped right after its `ATTACH`, so none is left for another to use, and a reader's error that should ever repeat the password says `********` in its place. | The probe on the test servers, and the review. No statement is refused by its name. |
| — | MySQL through an SSH tunnel goes to the tunnel's port on this machine by host and port: with `verify-full` (`verify_identity`) its certificate is checked against `127.0.0.1` and fails. PostgreSQL keeps the host for TLS (`hostaddr`). | The MySQL client has no `hostaddr`. Said in the PR. |
| — | DuckDB's PostgreSQL and MySQL readers use their own TLS clients (libpq, libmysqlclient). `sslmode`/`ssl_mode` follows the connection's TLS mode, and its CA file is passed (`sslrootcert`, `ssl_ca`). A verify mode with no CA file uses libpq's own default roots (`~/.postgresql/root.crt`), not the system's as the app's sessions do. | Untested here: the test servers have no TLS. Said in the PR. |
| Agents: "Agents can query the files and attached connections" | An agent's session attaches nothing yet. | 3b. |
| The sidebar's environment badges, the list row's "Joins Bookshop · dev" | Not yet. | 3b. |

The new words have no AI, sparkle or confidence wording, and the UI copy has no em dashes. They're translated into Russian and Brazilian Portuguese; the Portuguese wants a native review.

## Files

| File | Change |
| --- | --- |
| `crates/tabletist-db/src/files.rs`, `spec.rs` | `Attach`, `Attaching`, `attach_name`, `attached_schema`, `reserved_name`, `Reader`, `Downloading`; `ConnectSpec::attach` |
| `crates/tabletist-db/src/duck/attach.rs` (new), `duck.rs` | The statements; installing a reader; attaching before the fence; the attached schemas |
| `crates/tabletist-db/src/lib.rs` | `connect_files`, `open_files_to_test_with`, the attachments' tunnels |
| `crates/tabletist-db/tests/files_attach.rs` (new), `tests/ssh.rs` | The Bookshop attached on SQLite, PostgreSQL and MySQL; through tunnels |
| `Cargo.toml`, `.github/workflows/ci.yml` | DuckDB without debug info in the dev profile; the server jobs run `files_attach` |
| `crates/tabletist-core/src/connections/join.rs` (new), `connections.rs` | `Joinable`, `Blocked`, `joinable`, `default_name`, `name_problem` |
| `crates/tabletist-core/src/connections/file.rs` | `[[connections.attach]]`, version 10 |
| `crates/tabletist-core/src/backend/attach.rs` (new), `backend.rs` | `AttachTo`, the keyring read; Connect and Test attach; `Event::FilesDownloading` |
| `crates/tabletist-core/src/model.rs`, `src/app.rs` | The form's attachments; `Action::ToggleAttach`; what Connect and Test send |
| `src/ui/attach_list.rs` (new), `src/ui/connect_dialog/join.rs` (new), `connect_dialog/{mod,sheet,terminal}.rs`, `src/ui/mod.rs` | The section, its rows, the arrows, the download status |
| `src/ui/join_tests.rs` (new) | Headless tests of the section in both looks |
| `assets/i18n/*.po`, `*.pot` | The section's words |
| `docs/_reference/connections.md`, `docs/_reference/settings-and-files.md`, `docs/_guide/omarchy.md` | Join, `[[connections.attach]]`, the keys |

Every commit passes these, the full check:
- `~/.cargo/bin/cargo fmt --check`
- `~/.cargo/bin/cargo clippy --workspace --all-targets -- -D warnings`
- `~/.cargo/bin/cargo test --workspace --no-fail-fast`, the headless tests (with the server URLs below)
- `RUSTDOCFLAGS='-D warnings' ~/.cargo/bin/cargo doc --workspace --no-deps`
- `sh assets/i18n/update.sh --check`

Run cargo as `~/.cargo/bin/cargo` with `CARGO_TARGET_DIR=$PWD/target`. For the server tests, point `TABLETIST_TEST_PG_URL` and `TABLETIST_TEST_MYSQL_URL` at the test servers when they're up (`ss -ltn` shows 55432 and 53306). Their `bookshop` databases are the ones attached. The SSH test runs only with `TABLETIST_TEST_SSH_URL` (CI's ssh job). The first run downloads DuckDB's PostgreSQL, MySQL and SQLite readers, and Task 1's build compiles DuckDB again without debug info. Commits are signed: one for this plan, then one per task.

---

### Task 1: The engine attaches saved connections

**Files:**
- Create: `crates/tabletist-db/src/duck/attach.rs`, `crates/tabletist-db/tests/files_attach.rs`
- Modify: `crates/tabletist-db/src/files.rs`, `spec.rs`, `duck.rs`, `lib.rs`, `tests/ssh.rs`, `Cargo.toml`, `.github/workflows/ci.yml`, and `ConnectSpec`'s two literals in `crates/tabletist-core/src/connections/file.rs` and `model.rs`

- [ ] **Step 1: Write the failing tests**

The statements' module starts with its tests: the password goes into the secret alone, a tunnel's port, a SQLite file (one not there is refused), and the quoting. The integration tests attach the Bookshop's own database to its exports:
- the SQLite file always;
- PostgreSQL's and MySQL's `bookshop` databases with the server URLs set.

They join a file's table to it, and check it is held read-only. Writes, those sent through to the server included, are refused and none lands. No password, nor any secret, is left in anything the session shows, and a failed attach is said without it.

````diff
diff --git a/crates/tabletist-db/src/duck.rs b/crates/tabletist-db/src/duck.rs
--- a/crates/tabletist-db/src/duck.rs
+++ b/crates/tabletist-db/src/duck.rs
@@ -7,6 +7,8 @@
 //! connection's files and no other, which also keeps it from writing a
 //! file, installing or loading an extension. And a read-only transaction
 //! around every page, count, lookup, script and agent's read.
+
+mod attach;
 
 use std::path::Path;
 use std::sync::atomic::{AtomicBool, Ordering};
diff --git a/crates/tabletist-db/src/duck/attach.rs b/crates/tabletist-db/src/duck/attach.rs
new file mode 100644
--- /dev/null
+++ b/crates/tabletist-db/src/duck/attach.rs
@@ -0,0 +1,151 @@
+//! The saved connections a Files connection's session attaches: DuckDB's
+//! `postgres`, `mysql` and `sqlite` readers, each database attached
+//! read-only under the name SQL calls it by.
+//!
+//! A PostgreSQL or MySQL database's login goes into a secret of the
+//! session's own, in memory, which the `ATTACH` names and which is dropped
+//! right after it: the attached database keeps the login it was given,
+//! and no secret is left for another `ATTACH` or scan to find. The
+//! `ATTACH`'s text holds no password: the connection string it keeps
+//! (`duckdb_databases().path`) says only how TLS is used, where a tunnel
+//! goes, and that PostgreSQL's transactions are read-only. A statement
+//! that holds a password is never put in an error: DuckDB quotes the
+//! statement a parser error is in.
+//!
+//! An attachment is `READ_ONLY`: DuckDB refuses to write to it, and its
+//! readers run every statement sent through (`postgres_execute`,
+//! `mysql_execute`) in a read-only transaction or on a read-only
+//! connection, which the servers enforce. PostgreSQL's session also
+//! starts with `default_transaction_read_only` on.
+
+
+#[cfg(test)]
+mod tests {
+    use super::*;
+    use crate::{ConnectSpec, Secrets};
+
+    fn attaching(spec: ConnectSpec, password: Option<&str>) -> Attaching {
+        Attaching {
+            name: "bookshop_dev".into(),
+            spec,
+            secrets: Secrets {
+                password: password.map(str::to_owned),
+                ..Secrets::default()
+            },
+        }
+    }
+
+    fn postgres() -> ConnectSpec {
+        ConnectSpec {
+            driver: Driver::Postgres,
+            host: "db.example.com".into(),
+            port: 5433,
+            user: "reader".into(),
+            database: "bookshop_development".into(),
+            tls: TlsMode::VerifyFull,
+            ca_file: Some("/etc/bookshop/ca.pem".into()),
+            ..ConnectSpec::default()
+        }
+    }
+
+    /// The login and the password go into the secret; the ATTACH, whose
+    /// text DuckDB keeps and shows, holds neither.
+    #[test]
+    fn a_password_goes_into_the_secret_alone() {
+        let made = statements(0, &attaching(postgres(), Some("it's s3cret")), None).unwrap();
+        let secret = made.secret.unwrap();
+        assert_eq!(
+            secret,
+            "CREATE SECRET tabletist_attach_0 (TYPE postgres, HOST 'db.example.com', \
+             USER 'reader', DATABASE 'bookshop_development', PORT 5433, \
+             PASSWORD 'it''s s3cret')"
+        );
+        assert_eq!(
+            made.attach,
+            "ATTACH 'sslmode=verify-full application_name=Tabletist \
+             options=''-c default_transaction_read_only=on'' \
+             sslrootcert=/etc/bookshop/ca.pem' AS bookshop_dev \
+             (TYPE postgres, SECRET tabletist_attach_0, READ_ONLY)"
+        );
+        assert!(!made.attach.contains("s3cret"));
+        assert_eq!(
+            made.forget.as_deref(),
+            Some("DROP SECRET tabletist_attach_0")
+        );
+        assert_eq!(made.file, None);
+    }
+
+    /// Through a tunnel PostgreSQL goes to its port on this machine and
+    /// still checks the host; MySQL goes there by host and port.
+    #[test]
+    fn a_tunnel_takes_the_connection_to_its_port() {
+        let made = statements(1, &attaching(postgres(), None), Some(40112)).unwrap();
+        let secret = made.secret.unwrap();
+        assert!(secret.contains("HOST 'db.example.com'"), "{secret}");
+        assert!(!secret.contains("PORT"), "{secret}");
+        assert!(!secret.contains("PASSWORD"), "{secret}");
+        assert!(
+            made.attach.contains("hostaddr=127.0.0.1 port=40112"),
+            "{}",
+            made.attach
+        );
+        let mysql = ConnectSpec {
+            driver: Driver::MySql,
+            host: "db.example.com".into(),
+            port: 3306,
+            user: "reader".into(),
+            database: "bookshop".into(),
+            tls: TlsMode::Disable,
+            ..ConnectSpec::default()
+        };
+        let made = statements(2, &attaching(mysql, Some("pw")), Some(40113)).unwrap();
+        assert_eq!(
+            made.secret.unwrap(),
+            "CREATE SECRET tabletist_attach_2 (TYPE mysql, HOST '127.0.0.1', PORT 40113, \
+             USER 'reader', DATABASE 'bookshop', PASSWORD 'pw')"
+        );
+        assert_eq!(
+            made.attach,
+            "ATTACH 'ssl_mode=disabled' AS bookshop_dev \
+             (TYPE mysql, SECRET tabletist_attach_2, READ_ONLY)"
+        );
+    }
+
+    /// A SQLite file is attached by its path, which the fence lets in;
+    /// one that is not there is not.
+    #[test]
+    fn a_sqlite_file_is_attached_by_its_path() {
+        let dir = tempfile::tempdir().unwrap();
+        let file = dir.path().join("o'neil.sqlite3");
+        assert!(
+            statements(0, &attaching(ConnectSpec::sqlite(&file), None), None)
+                .unwrap_err()
+                .to_string()
+                .contains("Can't attach bookshop_dev: no such file")
+        );
+        std::fs::write(&file, "").unwrap();
+        let path = file.to_str().unwrap().to_owned();
+        let made = statements(0, &attaching(ConnectSpec::sqlite(&file), None), None).unwrap();
+        assert_eq!(made.secret, None);
+        assert_eq!(
+            made.attach,
+            format!(
+                "ATTACH '{}' AS bookshop_dev (TYPE sqlite, READ_ONLY)",
+                path.replace('\'', "''")
+            )
+        );
+        assert_eq!(made.file.as_deref(), Some(path.as_str()));
+    }
+
+    /// A value with a space, a quote or a backslash stays one value.
+    #[test]
+    fn a_connection_string_value_is_quoted_where_it_must_be() {
+        assert_eq!(
+            conninfo(&[
+                ("sslmode", "verify-full".into()),
+                ("sslrootcert", r"C:\My certs\o'neil.pem".into()),
+            ]),
+            r"sslmode=verify-full sslrootcert='C:\\My certs\\o\'neil.pem'"
+        );
+    }
+}
diff --git a/crates/tabletist-db/tests/files_attach.rs b/crates/tabletist-db/tests/files_attach.rs
new file mode 100644
--- /dev/null
+++ b/crates/tabletist-db/tests/files_attach.rs
@@ -0,0 +1,326 @@
+//! Saved connections attached to a Files connection, through the public
+//! `Connection` API: the Bookshop's exports joined to the Bookshop's own
+//! database, read-only, under the name SQL calls it by. The SQLite
+//! Bookshop always; PostgreSQL's and MySQL's with TABLETIST_TEST_PG_URL
+//! and TABLETIST_TEST_MYSQL_URL set (their `bookshop` databases).
+//!
+//! DuckDB downloads its PostgreSQL, MySQL and SQLite readers the first time
+//! a test needs them.
+
+// `allow-unwrap-in-tests` does not cover helpers in an integration-test crate.
+#![allow(clippy::unwrap_used)]
+
+use tabletist_db::files::{Attaching, Source};
+use tabletist_db::{
+    ConnectSpec, Connection, Dialect, Error, HostKeys, ObjectRef, RowQuery, ScriptMode, Secrets,
+    StatementOutcome, StopFlag, TlsMode, Value, sql,
+};
+
+/// The Bookshop's exports in a folder of their own.
+fn exports() -> tempfile::TempDir {
+    let dir = tempfile::tempdir().unwrap();
+    tabletist_db::fixtures::write_bookshop_files(dir.path()).unwrap();
+    dir
+}
+
+/// A session over `dir`'s files with `attaching` attached.
+async fn open(dir: &tempfile::TempDir, attaching: &[Attaching]) -> Result<Connection, Error> {
+    let spec = ConnectSpec::files(Source::Folder(dir.path().to_owned()));
+    Connection::connect_files(&spec, attaching, &HostKeys::default(), None).await
+}
+
+/// The SQLite Bookshop in `dir`, attached as `shop`.
+fn sqlite_shop(dir: &tempfile::TempDir) -> Attaching {
+    let path = dir.path().join("shop").with_extension("sqlite3");
+    tabletist_db::fixtures::write_sqlite_bookshop(&path).unwrap();
+    Attaching {
+        name: "shop".into(),
+        spec: ConnectSpec::sqlite(&path),
+        secrets: Secrets::default(),
+    }
+}
+
+/// Runs `text` as the SQL editor does: split, then one read-only script.
+async fn script(connection: &Connection, text: &str) -> Result<Vec<StatementOutcome>, Error> {
+    let statements = sql::statements(Dialect::DuckDb, text);
+    let outcome = connection
+        .run_script(&statements, 100, ScriptMode::ReadOnly, &StopFlag::new())
+        .await?;
+    Ok(outcome.results.into_iter().map(|r| r.outcome).collect())
+}
+
+async fn rows(connection: &Connection, text: &str) -> Vec<Vec<Value>> {
+    match script(connection, text).await.unwrap().remove(0) {
+        StatementOutcome::Rows { rows, .. } => rows,
+        other => panic!("{text}: {other:?}"),
+    }
+}
+
+async fn refused(connection: &Connection, text: &str) -> String {
+    match script(connection, text).await {
+        Err(error) => error.to_string(),
+        Ok(mut outcomes) => match outcomes.remove(0) {
+            StatementOutcome::Error { error, .. } => error.to_string(),
+            other => panic!("{text} ran: {other:?}"),
+        },
+    }
+}
+
+/// Every line of text the session shows of what it attached: the
+/// databases' connection strings, the secrets, and each attached table's
+/// statement.
+async fn shown(connection: &Connection) -> String {
+    let mut said = Vec::new();
+    for query in [
+        "SELECT coalesce(path, '') FROM duckdb_databases()",
+        "SELECT secret_string FROM duckdb_secrets()",
+        "SELECT coalesce(sql, '') FROM duckdb_tables()",
+        "SELECT coalesce(sql, '') FROM duckdb_views()",
+    ] {
+        for row in rows(connection, query).await {
+            said.push(format!("{row:?}"));
+        }
+    }
+    said.join("\n")
+}
+
+#[tokio::test]
+async fn a_sqlite_bookshop_is_a_schema_the_files_join() {
+    let dir = exports();
+    let shop = sqlite_shop(&dir);
+    let connection = open(&dir, &[shop]).await.unwrap();
+    assert_eq!(
+        connection.list_schemas().await.unwrap(),
+        ["main", "bookshop", "shop"]
+    );
+    let objects = connection.list_objects("shop").await.unwrap();
+    assert!(
+        objects.iter().any(|object| object.name == "books"),
+        "{objects:?}"
+    );
+    let books = ObjectRef::new("shop", "books");
+    let structure = connection.describe(&books).await.unwrap();
+    assert_eq!(structure.columns[1].name, "title");
+    let page = connection
+        .fetch_rows(&RowQuery::new(books.clone(), 5))
+        .await
+        .unwrap();
+    assert_eq!(page.rows.len(), 5);
+    assert_eq!(
+        connection
+            .count_rows(&RowQuery::new(books, 100))
+            .await
+            .unwrap(),
+        12
+    );
+    // A file's table and the attached database's, in one query.
+    let joined = rows(
+        &connection,
+        "SELECT count(*) FROM bookshop.order_items i JOIN shop.books b ON b.id = i.book_id",
+    )
+    .await;
+    assert!(
+        matches!(joined.as_slice(), [row] if matches!(row.as_slice(), [Value::Int(count)] if *count > 0)),
+        "{joined:?}"
+    );
+}
+
+#[tokio::test]
+async fn an_attached_connection_is_read_only() {
+    let dir = exports();
+    let shop = sqlite_shop(&dir);
+    let connection = open(&dir, &[shop]).await.unwrap();
+    for statement in [
+        "CREATE TABLE shop.probe (id INTEGER)",
+        "INSERT INTO shop.books (id, title) VALUES (99, 'x')",
+        "DELETE FROM shop.books",
+    ] {
+        let words = refused(&connection, statement).await;
+        assert!(words.contains("read-only"), "{statement}: {words}");
+    }
+    // DuckDB itself holds it read-only, whatever a script's transaction.
+    assert_eq!(
+        rows(
+            &connection,
+            "SELECT readonly FROM duckdb_databases() WHERE database_name = 'shop'"
+        )
+        .await,
+        [[Value::Bool(true)]]
+    );
+    // Nor does a script let it go, or attach another.
+    for statement in ["DETACH shop", "ATTACH ':memory:' AS other"] {
+        assert!(
+            !refused(&connection, statement).await.is_empty(),
+            "{statement}"
+        );
+    }
+    assert_eq!(
+        rows(&connection, "SELECT count(*) FROM shop.books").await,
+        [[Value::Int(12)]]
+    );
+}
+
+#[tokio::test]
+async fn an_attach_that_fails_says_which() {
+    let dir = exports();
+    let gone = Attaching {
+        name: "shop".into(),
+        spec: ConnectSpec::sqlite(dir.path().join("gone.sqlite3")),
+        secrets: Secrets::default(),
+    };
+    let Err(error) = open(&dir, &[gone]).await else {
+        panic!("a gone SQLite file was attached");
+    };
+    assert!(error.to_string().contains("Can't attach shop"), "{error}");
+}
+
+/// The test server's Bookshop: its `bookshop` database, the URL's login.
+fn server(variable: &str) -> Option<(ConnectSpec, Secrets)> {
+    let url = std::env::var(variable)
+        .ok()
+        .filter(|url| !url.trim().is_empty());
+    let Some(url) = url else {
+        eprintln!("skipped: {variable} is not set");
+        return None;
+    };
+    let (mut spec, secrets) = ConnectSpec::from_url(&url).unwrap();
+    spec.database = "bookshop".into();
+    spec.tls = TlsMode::Disable;
+    Some((spec, secrets))
+}
+
+/// What every server is asked: the Bookshop joined, its password nowhere
+/// the session shows, each of `writes` refused (one that starts `then ` is
+/// a step run first, which may run) and none landed, as `landed` counts,
+/// and a wrong password said without it.
+async fn a_server_bookshop_is_attached(
+    spec: ConnectSpec,
+    secrets: Secrets,
+    writes: &[&str],
+    landed: &str,
+) {
+    let dir = exports();
+    let password = secrets.password.clone().unwrap();
+    let attaching = Attaching {
+        name: "shop".into(),
+        spec: spec.clone(),
+        secrets: secrets.clone(),
+    };
+    let connection = open(&dir, &[attaching]).await.unwrap();
+    assert!(
+        connection
+            .list_objects("shop")
+            .await
+            .unwrap()
+            .iter()
+            .any(|object| object.name == "books")
+    );
+    let books = ObjectRef::new("shop", "books");
+    assert_eq!(
+        connection
+            .count_rows(&RowQuery::new(books.clone(), 100))
+            .await
+            .unwrap(),
+        12
+    );
+    assert!(connection.describe(&books).await.unwrap().columns.len() > 1);
+    let joined = rows(
+        &connection,
+        "SELECT count(*) FROM bookshop.order_items i JOIN shop.books b ON b.id = i.book_id",
+    )
+    .await;
+    assert!(
+        matches!(joined.as_slice(), [row] if matches!(row.as_slice(), [Value::Int(count)] if *count > 0)),
+        "{joined:?}"
+    );
+    // The test server's password is its user's name too: what matters is
+    // that no password is said.
+    let said = shown(&connection).await;
+    assert!(!said.contains(&format!("password={password}")), "{said}");
+    assert!(!said.contains(&format!("password='{password}'")), "{said}");
+    // The login was let go once the database was attached.
+    assert_eq!(
+        rows(&connection, "SELECT count(*) FROM duckdb_secrets()").await,
+        [[Value::Int(0)]]
+    );
+    assert_eq!(
+        rows(
+            &connection,
+            "SELECT readonly FROM duckdb_databases() WHERE database_name = 'shop'"
+        )
+        .await,
+        [[Value::Bool(true)]]
+    );
+    // Each write sent through to the server is refused, and none lands.
+    // A step may run (PostgreSQL takes a COMMIT): the write after it
+    // still may not.
+    for statement in writes {
+        match statement.strip_prefix("then ") {
+            Some(step) => drop(script(&connection, step).await),
+            None => assert!(
+                !refused(&connection, statement).await.is_empty(),
+                "{statement}"
+            ),
+        }
+    }
+    assert_eq!(
+        rows(&connection, landed).await,
+        [[Value::Int(0)]],
+        "a write landed"
+    );
+    // A wrong password: said by the server, the password not.
+    let wrong = Attaching {
+        name: "shop".into(),
+        spec,
+        secrets: Secrets {
+            password: Some("not-the-bookshop-password".into()),
+            ..secrets
+        },
+    };
+    let Err(error) = open(&dir, &[wrong]).await else {
+        panic!("a wrong password attached");
+    };
+    let error = error.to_string();
+    assert!(error.contains("Can't attach shop"), "{error}");
+    assert!(!error.contains("not-the-bookshop-password"), "{error}");
+}
+
+#[tokio::test]
+async fn a_postgres_bookshop_is_attached() {
+    let Some((spec, secrets)) = server("TABLETIST_TEST_PG_URL") else {
+        return;
+    };
+    a_server_bookshop_is_attached(
+        spec,
+        secrets,
+        &[
+            "CALL postgres_execute('shop', 'CREATE TABLE attach_probe (id integer)')",
+            "CALL postgres_execute('shop', 'CREATE TABLE attach_probe (id integer)', \
+             use_transaction := false)",
+            "then CALL postgres_execute('shop', 'COMMIT')",
+            "CALL postgres_execute('shop', 'CREATE TABLE attach_probe (id integer)')",
+        ],
+        "SELECT count(*) FROM postgres_query('shop', \
+         'SELECT 1 FROM pg_tables WHERE tablename = ''attach_probe''')",
+    )
+    .await;
+}
+
+#[tokio::test]
+async fn a_mysql_bookshop_is_attached() {
+    let Some((spec, secrets)) = server("TABLETIST_TEST_MYSQL_URL") else {
+        return;
+    };
+    a_server_bookshop_is_attached(
+        spec,
+        secrets,
+        &[
+            "CALL mysql_execute('shop', 'CREATE TABLE attach_probe (id integer)')",
+            "CALL mysql_execute('shop', 'COMMIT')",
+            "SELECT * FROM mysql_query('shop', 'CREATE TABLE attach_probe (id integer)')",
+        ],
+        "SELECT count(*) FROM mysql_query('shop', \
+         'SELECT 1 FROM information_schema.tables WHERE table_name = ''attach_probe''')",
+    )
+    .await;
+}
````

- [ ] **Step 2: Run them to see them fail**

Run: `~/.cargo/bin/cargo test -q -p tabletist-db --test files_attach --lib attach`

Expected: they don't compile. `statements`, `conninfo`, `Attaching` and `Connection::connect_files` aren't there yet.

```text
error[E0425]: cannot find type `Attaching` in this scope
error[E0422]: cannot find struct, variant or union type `Attaching` in this scope
error[E0433]: cannot find type `Driver` in this scope
error[E0433]: cannot find type `TlsMode` in this scope
error[E0425]: cannot find function `statements` in this scope
    |
error[E0432]: unresolved import `tabletist_db::files::Attaching`
error[E0599]: no associated function or constant named `connect_files` found for struct `Connection` in the current scope
error: could not compile `tabletist-db` (test "files_attach") due to 2 previous errors
error[E0425]: cannot find function `conninfo` in this scope
error: could not compile `tabletist-db` (lib test) due to 12 previous errors; 1 warning emitted
```

- [ ] **Step 3: Attach, before the fence**

````diff
diff --git a/.github/workflows/ci.yml b/.github/workflows/ci.yml
--- a/.github/workflows/ci.yml
+++ b/.github/workflows/ci.yml
@@ -129,7 +129,7 @@
             pg_isready -h localhost -p 55432 && break
             sleep 1
           done
-      - run: cargo test --locked -p tabletist-db --lib --test postgres --test insert_audit --test lookup --test delete_rows --test agent --test ddl --test export
+      - run: cargo test --locked -p tabletist-db --lib --test postgres --test insert_audit --test lookup --test delete_rows --test agent --test ddl --test export --test files_attach
         env:
           TABLETIST_TEST_PG_URL: postgres://tabletist:tabletist@localhost:55432/tabletist
       # The app's gateway for the MCP server, which opens its own sessions:
@@ -153,7 +153,7 @@
       - uses: Swatinem/rust-cache@v2
       - name: Start MySQL
         run: docker compose up -d --wait mysql
-      - run: cargo test --locked -p tabletist-db --lib --test mysql --test insert_audit --test lookup --test delete_rows --test agent --test ddl --test export
+      - run: cargo test --locked -p tabletist-db --lib --test mysql --test insert_audit --test lookup --test delete_rows --test agent --test ddl --test export --test files_attach
         env:
           TABLETIST_TEST_MYSQL_URL: mysql://tabletist:tabletist@localhost:53306/tabletist
       # The app's gateway for the MCP server, which opens its own sessions:
diff --git a/Cargo.toml b/Cargo.toml
--- a/Cargo.toml
+++ b/Cargo.toml
@@ -165,6 +165,13 @@
 [profile.dev.package."*"]
 opt-level = 2
 
+# DuckDB's PostgreSQL, MySQL, SQLite and Excel readers are release builds it
+# downloads. A DuckDB built with its own debug checks, which debug info turns
+# on (`DEBUG`), counts their allocations as its own and aborts: it is built
+# as a release is, in every profile.
+[profile.dev.package.libduckdb-sys]
+debug = false
+
 [patch.crates-io]
 # crmne/egui apps-0.36.2, the revision fastframe's getting-started guide
 # names, shared with spotifast: bidi shaping, Wayland frame-callback pacing,
diff --git a/crates/tabletist-core/src/connections/file.rs b/crates/tabletist-core/src/connections/file.rs
--- a/crates/tabletist-core/src/connections/file.rs
+++ b/crates/tabletist-core/src/connections/file.rs
@@ -469,6 +469,7 @@
                 ssh,
                 files,
                 csv,
+                attach: Vec::new(),
             },
             agents,
         })
diff --git a/crates/tabletist-core/src/model.rs b/crates/tabletist-core/src/model.rs
--- a/crates/tabletist-core/src/model.rs
+++ b/crates/tabletist-core/src/model.rs
@@ -3022,6 +3022,7 @@
                     ssh,
                     files: None,
                     csv: tabletist_db::files::Reading::default(),
+                    attach: Vec::new(),
                 })
             }
         }
diff --git a/crates/tabletist-db/src/duck.rs b/crates/tabletist-db/src/duck.rs
--- a/crates/tabletist-db/src/duck.rs
+++ b/crates/tabletist-db/src/duck.rs
@@ -7,6 +7,10 @@
 //! connection's files and no other, which also keeps it from writing a
 //! file, installing or loading an extension. And a read-only transaction
 //! around every page, count, lookup, script and agent's read.
+//!
+//! Saved PostgreSQL, MySQL and SQLite connections it is told to attach are
+//! attached read-only before the fence (`attach`), and stay reachable
+//! after it: the fence keeps any other from being attached.
 
 mod attach;
 
@@ -21,7 +25,10 @@
 
 use crate::adapter::Adapter;
 use crate::agent::{Catalog, ColumnName, Limits, Param, Read, Refusal, Sources};
-use crate::files::{Change, Format, Found, Header, Reading, SAMPLE_ROWS, Source, Types};
+use crate::files::{
+    Attaching, Change, Downloading, Format, Found, Header, Reader, Reading, SAMPLE_ROWS, Source,
+    Types,
+};
 use crate::{
     CancelHandle, CancelInner, ChangeSet, ColumnClass, ColumnInfo, ColumnMeta, Ddl, Dialect, Error,
     Lookup, LookupPage, MAX_LISTED, ObjectInfo, ObjectKind, ObjectRef, Result, RowPage, RowQuery,
@@ -88,7 +95,18 @@
     /// Every file of the connection's, in its order: those it could not
     /// open among them.
     order: Vec<std::path::PathBuf>,
+    /// The saved connections it attached, in order.
+    attached: Arc<Vec<Attached>>,
     version: String,
+}
+
+/// A saved connection the session attached: the name SQL calls it by,
+/// which the sidebar lists it as, and the schema of its own whose tables
+/// are listed there and that `name.table` reads.
+#[derive(Debug, Clone, PartialEq, Eq)]
+struct Attached {
+    name: String,
+    schema: String,
 }
 
 /// A file as the session opened it.
@@ -472,19 +490,111 @@
 }
 
 /// What [`opened`] made: the session, its files, those it could not open
-/// (where it was told to go on past them) and DuckDB's version.
+/// (where it was told to go on past them), the saved connections it
+/// attached and DuckDB's version.
 type Made = (
     duckdb::Connection,
     Vec<Opened>,
     Vec<crate::files::Failure>,
+    Vec<Attached>,
     String,
 );
 
+/// Installs and loads `reader` in the session: from the folder it was
+/// installed into once, else downloaded from DuckDB's servers, which
+/// `downloading` is told of first.
+fn install(
+    connection: &duckdb::Connection,
+    reader: Reader,
+    downloading: Option<&Downloading>,
+) -> Result<()> {
+    let installed: bool = connection
+        .query_row(
+            "SELECT coalesce(bool_or(installed), false) FROM duckdb_extensions() \
+             WHERE extension_name = ?",
+            [reader.listed()],
+            |row| row.get(0),
+        )
+        .map_err(map_error)?;
+    if !installed && let Some(downloading) = downloading {
+        downloading(reader);
+    }
+    let extension = reader.extension();
+    connection
+        .execute_batch(&format!("INSTALL {extension}; LOAD {extension}"))
+        .map_err(map_error)
+}
+
+/// Attaches `attaching`, reached through a tunnel's local port `via`
+/// where it has one, as the session's `at`th: the SQLite file it opens,
+/// which the fence lets in, where it is one.
+fn attached(
+    connection: &duckdb::Connection,
+    at: usize,
+    attaching: &Attaching,
+    via: Option<u16>,
+    downloading: Option<&Downloading>,
+) -> Result<(Attached, Option<String>)> {
+    let name = &attaching.name;
+    let reader = attach::reader(attaching)?;
+    install(connection, reader, downloading).map_err(|error| {
+        Error::Connect(format!(
+            "Can't attach {name}: the {} reader for DuckDB could not be installed ({})",
+            reader.label(),
+            plain(&error)
+        ))
+    })?;
+    let made = attach::statements(at, attaching, via)?;
+    if let Some(secret) = &made.secret {
+        // Never DuckDB's words: a parser error quotes the statement, and
+        // this one holds the password.
+        connection.execute_batch(secret).map_err(|_| {
+            Error::Connect(format!(
+                "Can't attach {name}: DuckDB did not take its login"
+            ))
+        })?;
+    }
+    let attached = connection.execute_batch(&made.attach).map_err(|error| {
+        let error = map_error(error);
+        // The reader's own words: should they ever repeat the password,
+        // it is not said.
+        let mut words = plain(&error);
+        if let Some(password) = attaching.secrets.password.as_deref()
+            && !password.is_empty()
+        {
+            words = words.replace(password, "********");
+        }
+        Error::Connect(format!("Can't attach {name}: {words}"))
+    });
+    // The login is let go whether it was taken or not: the attached
+    // database keeps its own.
+    if let Some(forget) = &made.forget {
+        connection.execute_batch(forget).map_err(map_error)?;
+    }
+    attached?;
+    Ok((
+        Attached {
+            name: name.clone(),
+            schema: crate::files::attached_schema(&attaching.spec),
+        },
+        made.file,
+    ))
+}
+
 /// Opens a session over `files`, read as `reading` says, a file it skips
-/// left out: views and attaches, then the fence. A file that cannot be
-/// opened stops it, unless `past` says to go on without it (a Test, which
-/// says which failed).
-fn opened(files: &[Found], reading: &Reading, past: bool) -> Result<Made> {
+/// left out: views and attaches, then the saved connections in
+/// `attaching`, each with the tunnel port it is reached through, then the
+/// fence. A file that cannot be opened stops it, unless `past` says to go
+/// on without it (a Test, which says which failed); a saved connection
+/// that cannot be attached always does. `downloading` is told of each
+/// reader DuckDB downloads first.
+fn opened(
+    files: &[Found],
+    reading: &Reading,
+    past: bool,
+    attaching: &[(Attaching, Option<u16>)],
+    downloading: Option<&Downloading>,
+) -> Result<Made> {
     let files: Vec<&Found> = files
         .iter()
         .filter(|file| !reading.skips(&file.path))
@@ -505,7 +615,7 @@
     if files.iter().any(|file| file.format == Format::Excel) {
         // Installed once, then found in the folder: offline, a second open
         // still reads Excel files.
-        if let Err(error) = run("INSTALL excel; LOAD excel") {
+        if let Err(error) = install(&connection, Reader::Excel, downloading) {
             let why = format!(
                 "the Excel reader for DuckDB could not be installed ({})",
                 plain(&error)
@@ -593,11 +703,19 @@
             rows,
         });
     }
+    // The saved connections, read-only, as every session is.
+    let mut joined = Vec::with_capacity(attaching.len());
+    let mut attached_files = Vec::new();
+    for (at, (attaching, via)) in attaching.iter().enumerate() {
+        let (each, file) = attached(&connection, at, attaching, *via, downloading)?;
+        joined.push(each);
+        attached_files.extend(file);
+    }
     // The fence: the files that opened and no other, then no setting
     // changed again. One a Test went on without is not read, and its path
     // may be one DuckDB cannot be given. A reader asks by its own spelling
-    // of a file, which names only it.
-    let mut allowed = Vec::with_capacity(opened.len());
+    // of a file, which names only it. An attached SQLite file too.
+    let mut allowed = Vec::with_capacity(opened.len() + attached_files.len());
     for file in &opened {
         let file = &file.found;
         let (path, read) = (path_text(&file.path)?, reader_path(&file.path)?);
@@ -606,6 +724,7 @@
         }
         allowed.push(literal(&path));
     }
+    allowed.extend(attached_files.iter().map(|path| literal(path)));
     run(&format!("SET allowed_paths = [{}]", allowed.join(", ")))?;
     run("SET enable_external_access = false")?;
     run("SET lock_configuration = true")?;
@@ -616,6 +735,7 @@
         connection,
         opened,
         failures,
+        joined,
         format!("DuckDB {}", version.trim_start_matches('v')),
     ))
 }
@@ -1229,17 +1349,25 @@
 }
 
 /// The catalog and schema DuckDB keeps `object` in: a file's view in the
-/// in-memory database's `main`, a `.duckdb` file's table in its own.
-fn catalog_of(object: &ObjectRef) -> (&str, &str) {
+/// in-memory database's `main`, a `.duckdb` file's table in its own, an
+/// attached connection's in its schema the sidebar lists.
+fn catalog_of<'a>(object: &'a ObjectRef, attached: &'a [Attached]) -> (&'a str, &'a str) {
     match object.schema.as_str() {
         MAIN => (MEMORY, MAIN),
-        attached => (attached, MAIN),
+        schema => match attached.iter().find(|each| each.name == schema) {
+            Some(each) => (&each.name, &each.schema),
+            None => (schema, MAIN),
+        },
     }
 }
 
 /// The columns of `object`, as DuckDB's catalog has them.
-fn columns(connection: &duckdb::Connection, object: &ObjectRef) -> Result<Vec<ColumnInfo>> {
-    let (catalog, schema) = catalog_of(object);
+fn columns(
+    connection: &duckdb::Connection,
+    object: &ObjectRef,
+    attached: &[Attached],
+) -> Result<Vec<ColumnInfo>> {
+    let (catalog, schema) = catalog_of(object, attached);
     let mut statement = connection
         .prepare(
             "SELECT column_name, data_type, is_nullable, column_default \
@@ -1267,8 +1395,9 @@
 fn binary_columns(
     connection: &duckdb::Connection,
     object: &ObjectRef,
+    attached: &[Attached],
 ) -> Result<Vec<(String, ColumnClass)>> {
-    Ok(binary_of(&columns(connection, object)?))
+    Ok(binary_of(&columns(connection, object, attached)?))
 }
 
 /// The columns of `columns` that hold bytes, with their class: what the
@@ -1345,10 +1474,19 @@
     /// Opens `source`'s files, read-only: every one is read far enough to
     /// know its columns, and an error names the first that cannot be. A
     /// Test goes on `past` them: those it cannot open or look at, and a
-    /// listed file that is gone, are its failures.
-    pub(crate) async fn open(source: &Source, reading: &Reading, past: bool) -> Result<Self> {
+    /// listed file that is gone, are its failures. The saved connections
+    /// in `attaching` are attached after, each through the tunnel port
+    /// beside it where it has one; `downloading` is told of each reader
+    /// DuckDB downloads first.
+    pub(crate) async fn open(
+        source: &Source,
+        reading: &Reading,
+        past: bool,
+        attaching: Vec<(Attaching, Option<u16>)>,
+        downloading: Option<Downloading>,
+    ) -> Result<Self> {
         let (source, reading) = (source.clone(), reading.clone());
-        let (connection, files, failed, version, order) = tokio::task::spawn_blocking(move || {
+        let made = tokio::task::spawn_blocking(move || {
             // A skipped file is not looked at: one that is gone, or that
             // cannot be looked at, is no matter.
             let scan = crate::files::scan_past(&source, |file| reading.skips(file), past)?;
@@ -1380,7 +1518,14 @@
                     order
                 }
             };
-            opened(&scan.files, &reading, past).map(|(connection, files, mut failed, version)| {
+            opened(
+                &scan.files,
+                &reading,
+                past,
+                &attaching,
+                downloading.as_ref(),
+            )
+            .map(|(connection, files, mut failed, attached, version)| {
                 failed.extend(gone.iter().filter_map(|gone| {
                     Some(crate::files::Failure {
                         file: crate::files::file_name(gone),
@@ -1398,11 +1543,12 @@
                         why: crate::files::Why::Other(why.clone()),
                     })
                 }));
-                (connection, files, failed, version, order)
+                (connection, files, failed, attached, version, order)
             })
         })
         .await
         .map_err(|error| Error::Io(error.to_string()))??;
+        let (connection, files, failed, attached, version, order) = made;
         let interrupt = connection.interrupt_handle();
         Ok(Self {
             inner: Arc::new(Mutex::new(connection)),
@@ -1412,6 +1558,7 @@
             files: Arc::new(files),
             failed,
             order,
+            attached: Arc::new(attached),
             version,
         })
     }
@@ -1647,7 +1794,8 @@
         Ok(Vec::new())
     }
 
-    /// `main`, which holds the files' tables, then each `.duckdb` file.
+    /// `main`, which holds the files' tables, then each `.duckdb` file,
+    /// then each saved connection attached, by the name SQL calls it by.
     async fn list_schemas(&self) -> Result<Vec<String>> {
         Ok(std::iter::once(MAIN.to_owned())
             .chain(
@@ -1656,12 +1804,14 @@
                     .filter(|file| file.found.format.is_database())
                     .map(|file| file.found.name.clone()),
             )
+            .chain(self.attached.iter().map(|each| each.name.clone()))
             .take(MAX_LISTED as usize)
             .collect())
     }
 
     /// In `main`, a table for each file, in the files' order; in a
-    /// `.duckdb` file's schema, its tables and views by name.
+    /// `.duckdb` file's schema, its tables and views by name; in an
+    /// attached connection's, those of its schema `name.table` reads.
     async fn list_objects(&self, schema: &str) -> Result<Vec<ObjectInfo>> {
         if schema == MAIN {
             return Ok(self
@@ -1676,18 +1826,23 @@
                 .take(MAX_LISTED as usize)
                 .collect());
         }
-        let catalog = schema.to_owned();
+        let object = ObjectRef {
+            schema: schema.to_owned(),
+            name: String::new(),
+        };
+        let (catalog, schema) = catalog_of(&object, &self.attached);
+        let (catalog, schema) = (catalog.to_owned(), schema.to_owned());
         self.run(move |connection| {
             read_only(connection, |connection| {
                 let mut statement = connection
                     .prepare(&format!(
                         "SELECT table_name, table_type FROM information_schema.tables \
-                         WHERE table_catalog = ? AND table_schema = 'main' \
+                         WHERE table_catalog = ? AND table_schema = ? \
                          ORDER BY table_name LIMIT {MAX_LISTED}"
                     ))
                     .map_err(map_error)?;
                 let objects = statement
-                    .query_map([catalog.as_str()], |row| {
+                    .query_map([catalog.as_str(), schema.as_str()], |row| {
                         let kind: String = row.get(1)?;
                         Ok(ObjectInfo {
                             name: row.get(0)?,
@@ -1715,8 +1870,11 @@
             options: file.options.clone(),
         });
         let object = object.clone();
+        let attached = Arc::clone(&self.attached);
         self.run(move |connection| {
-            let columns = read_only(connection, |connection| columns(connection, &object))?;
+            let columns = read_only(connection, |connection| {
+                columns(connection, &object, &attached)
+            })?;
             if columns.is_empty() {
                 return Err(Error::query(format!(
                     "no such table or view: {}",
@@ -1752,8 +1910,9 @@
             return Ok(ddl(file.made.clone()));
         }
         let object = object.clone();
+        let attached = Arc::clone(&self.attached);
         self.run(move |connection| {
-            let (catalog, schema) = catalog_of(&object);
+            let (catalog, schema) = catalog_of(&object, &attached);
             let sql: Option<String> = read_only(connection, |connection| {
                 connection
                     .query_row(
@@ -1788,9 +1947,10 @@
         let object = query.object.clone();
         let query = query.clone();
         let limit = query.limit as usize;
+        let attached = Arc::clone(&self.attached);
         self.run(move |connection| {
             read_only(connection, |connection| {
-                let catalog = columns(connection, &query.object)?;
+                let catalog = columns(connection, &query.object, &attached)?;
                 let binary = binary_of(&catalog);
                 // A file has no key: rows come in the file's order.
                 let sql = Dialect::DuckDb.select_rows(&query, &[], &binary)?;
@@ -1814,9 +1974,10 @@
     async fn export_statement(&self, query: &RowQuery, columns: &[String]) -> Result<String> {
         let query = query.clone();
         let chosen = columns.to_vec();
+        let attached = Arc::clone(&self.attached);
         self.run(move |connection| {
             read_only(connection, |connection| {
-                let binary = binary_columns(connection, &query.object)?;
+                let binary = binary_columns(connection, &query.object, &attached)?;
                 Dialect::DuckDb.shown_export(&query, &[], &binary, &chosen)
             })
         })
@@ -1839,9 +2000,10 @@
         let chosen = columns.to_vec();
         let to = to.clone();
         let stop = stop.clone();
+        let attached = Arc::clone(&self.attached);
         self.run(move |connection| {
             read_only(connection, |connection| {
-                let catalog = self::columns(connection, &query.object)?;
+                let catalog = self::columns(connection, &query.object, &attached)?;
                 let binary = binary_of(&catalog);
                 let sql = Dialect::DuckDb.export_rows(&query, &[], &binary, &chosen)?;
                 let mut statement = connection.prepare(&sql.text).map_err(map_error)?;
@@ -1885,9 +2047,10 @@
     async fn count_rows(&self, query: &RowQuery) -> Result<u64> {
         check_raw_where(query)?;
         let query = query.clone();
+        let attached = Arc::clone(&self.attached);
         self.run(move |connection| {
             read_only(connection, |connection| {
-                let binary = binary_columns(connection, &query.object)?;
+                let binary = binary_columns(connection, &query.object, &attached)?;
                 let sql = Dialect::DuckDb.count_rows(&query, &binary)?;
                 let count: i64 = connection
                     .query_row(
diff --git a/crates/tabletist-db/src/duck/attach.rs b/crates/tabletist-db/src/duck/attach.rs
--- a/crates/tabletist-db/src/duck/attach.rs
+++ b/crates/tabletist-db/src/duck/attach.rs
@@ -18,6 +18,199 @@
 //! connection, which the servers enforce. PostgreSQL's session also
 //! starts with `default_transaction_read_only` on.
 
+use crate::files::{Attaching, Reader};
+use crate::{Dialect, Driver, Error, Result, TlsMode};
+
+/// The statements that attach `attaching` as the session's `at`th
+/// attachment, reached through a tunnel's local port `via` where it has
+/// one, in order.
+#[derive(Debug)]
+pub(super) struct Statements {
+    /// Its login, for PostgreSQL and MySQL; none for a SQLite file.
+    pub(super) secret: Option<String>,
+    pub(super) attach: String,
+    /// Drops the secret, once the database is attached.
+    pub(super) forget: Option<String>,
+    /// The SQLite file, which the session's fence lets DuckDB open.
+    pub(super) file: Option<String>,
+}
+
+/// The reader DuckDB attaches `attaching` with.
+pub(super) fn reader(attaching: &Attaching) -> Result<Reader> {
+    Reader::of(attaching.spec.driver).ok_or_else(|| {
+        Error::Connect(format!(
+            "Can't attach {}: only a PostgreSQL, MySQL or SQLite connection can be attached",
+            attaching.name
+        ))
+    })
+}
+
+/// The statements for `attaching`; see [`Statements`].
+pub(super) fn statements(at: usize, attaching: &Attaching, via: Option<u16>) -> Result<Statements> {
+    let spec = &attaching.spec;
+    let name = Dialect::DuckDb.ident(&attaching.name).into_owned();
+    let secret = format!("tabletist_attach_{at}");
+    let literal = |text: &str| Dialect::DuckDb.literal(text);
+    let password = attaching.secrets.password.as_deref();
+    match spec.driver {
+        Driver::Postgres => {
+            // libpq's default database is the user's name.
+            let database = match spec.database.is_empty() {
+                true => &spec.user,
+                false => &spec.database,
+            };
+            let mut login = vec![
+                format!("HOST {}", literal(&spec.host)),
+                format!("USER {}", literal(&spec.user)),
+                format!("DATABASE {}", literal(database)),
+            ];
+            // Through a tunnel the socket goes to its port on this machine
+            // while TLS still checks the host, as the app's own sessions do.
+            let mut path = vec![
+                ("sslmode", pg_ssl_mode(spec.tls).to_owned()),
+                ("application_name", "Tabletist".to_owned()),
+                ("options", "-c default_transaction_read_only=on".to_owned()),
+            ];
+            match via {
+                Some(port) => {
+                    path.push(("hostaddr", "127.0.0.1".to_owned()));
+                    path.push(("port", port.to_string()));
+                }
+                None => login.push(format!("PORT {}", spec.port)),
+            }
+            if let Some(ca) = &spec.ca_file {
+                path.push(("sslrootcert", path_text(ca)?));
+            }
+            if let Some(password) = password {
+                login.push(format!("PASSWORD {}", literal(password)));
+            }
+            Ok(Statements {
+                secret: Some(format!(
+                    "CREATE SECRET {secret} (TYPE postgres, {})",
+                    login.join(", ")
+                )),
+                attach: format!(
+                    "ATTACH {} AS {name} (TYPE postgres, SECRET {secret}, READ_ONLY)",
+                    literal(&conninfo(&path))
+                ),
+                forget: Some(format!("DROP SECRET {secret}")),
+                file: None,
+            })
+        }
+        Driver::MySql => {
+            let (host, port) = match via {
+                Some(port) => ("127.0.0.1", port),
+                None => (spec.host.as_str(), spec.port),
+            };
+            let mut login = vec![
+                format!("HOST {}", literal(host)),
+                format!("PORT {port}"),
+                format!("USER {}", literal(&spec.user)),
+                format!("DATABASE {}", literal(&spec.database)),
+            ];
+            if let Some(password) = password {
+                login.push(format!("PASSWORD {}", literal(password)));
+            }
+            let mut path = vec![("ssl_mode", mysql_ssl_mode(spec.tls).to_owned())];
+            if let Some(ca) = &spec.ca_file {
+                path.push(("ssl_ca", path_text(ca)?));
+            }
+            Ok(Statements {
+                secret: Some(format!(
+                    "CREATE SECRET {secret} (TYPE mysql, {})",
+                    login.join(", ")
+                )),
+                attach: format!(
+                    "ATTACH {} AS {name} (TYPE mysql, SECRET {secret}, READ_ONLY)",
+                    literal(&conninfo(&path))
+                ),
+                forget: Some(format!("DROP SECRET {secret}")),
+                file: None,
+            })
+        }
+        Driver::Sqlite => {
+            let path = spec.sqlite_path.as_deref().ok_or_else(|| {
+                Error::Connect(format!("Can't attach {}: it names no file", attaching.name))
+            })?;
+            // DuckDB attaches a file that is not there as an empty one.
+            if !path.is_file() {
+                return Err(Error::Connect(format!(
+                    "Can't attach {}: no such file {}",
+                    attaching.name,
+                    crate::files::shown(path)
+                )));
+            }
+            let path = path_text(path)?;
+            Ok(Statements {
+                secret: None,
+                attach: format!(
+                    "ATTACH {} AS {name} (TYPE sqlite, READ_ONLY)",
+                    literal(&path)
+                ),
+                forget: None,
+                file: Some(path),
+            })
+        }
+        Driver::BigQuery | Driver::Files => Err(Error::Connect(format!(
+            "Can't attach {}: only a PostgreSQL, MySQL or SQLite connection can be attached",
+            attaching.name
+        ))),
+    }
+}
+
+/// libpq's word for `mode`.
+fn pg_ssl_mode(mode: TlsMode) -> &'static str {
+    match mode {
+        TlsMode::Disable => "disable",
+        TlsMode::Prefer => "prefer",
+        TlsMode::Require => "require",
+        TlsMode::VerifyCa => "verify-ca",
+        TlsMode::VerifyFull => "verify-full",
+    }
+}
+
+/// The MySQL client's word for `mode`.
+fn mysql_ssl_mode(mode: TlsMode) -> &'static str {
+    match mode {
+        TlsMode::Disable => "disabled",
+        TlsMode::Prefer => "preferred",
+        TlsMode::Require => "required",
+        TlsMode::VerifyCa => "verify_ca",
+        TlsMode::VerifyFull => "verify_identity",
+    }
+}
+
+/// `key=value` pairs as a connection string has them, a space between: a
+/// plain value as it is, any other in quotes, a quote or a backslash in it
+/// after a backslash.
+fn conninfo(pairs: &[(&str, String)]) -> String {
+    let plain = |value: &str| {
+        !value.is_empty()
+            && value
+                .chars()
+                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.' | '/' | ':'))
+    };
+    pairs
+        .iter()
+        .map(|(key, value)| match plain(value) {
+            true => format!("{key}={value}"),
+            false => {
+                let value = value.replace('\\', "\\\\").replace('\'', "\\'");
+                format!("{key}='{value}'")
+            }
+        })
+        .collect::<Vec<_>>()
+        .join(" ")
+}
+
+fn path_text(path: &std::path::Path) -> Result<String> {
+    path.to_str().map(str::to_owned).ok_or_else(|| {
+        Error::Connect(format!(
+            "Can't attach {}: the path is not valid UTF-8",
+            path.display()
+        ))
+    })
+}
 
 #[cfg(test)]
 mod tests {
diff --git a/crates/tabletist-db/src/files.rs b/crates/tabletist-db/src/files.rs
--- a/crates/tabletist-db/src/files.rs
+++ b/crates/tabletist-db/src/files.rs
@@ -578,6 +578,132 @@
     name
 }
 
+/// A saved connection a Files connection attaches, as it is kept: the
+/// saved connection's id, and the name SQL calls it by
+/// (`bookshop_dev.orders`).
+#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
+pub struct Attach {
+    pub connection: String,
+    pub name: String,
+}
+
+/// A saved connection to attach as a session opens: the name SQL calls
+/// it by, how it is reached, and its secrets, read from the keyring just
+/// before. Prints without them.
+#[derive(Clone)]
+pub struct Attaching {
+    pub name: String,
+    pub spec: crate::ConnectSpec,
+    pub secrets: crate::Secrets,
+}
+
+impl std::fmt::Debug for Attaching {
+    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
+        f.debug_struct("Attaching")
+            .field("name", &self.name)
+            .field("driver", &self.spec.driver)
+            .finish_non_exhaustive()
+    }
+}
+
+/// The name SQL calls an attached connection by until it is changed: its
+/// database's name as it is (a SQLite file's stem), made a plain name as
+/// a file's table name is (see [`table_name`]). A PostgreSQL connection
+/// that names no database reads its user's.
+pub fn attach_name(spec: &crate::ConnectSpec) -> String {
+    use crate::Driver;
+    let named = match spec.driver {
+        Driver::Sqlite => spec
+            .sqlite_path
+            .as_deref()
+            .and_then(Path::file_stem)
+            .and_then(|stem| stem.to_str())
+            .unwrap_or_default()
+            .to_owned(),
+        Driver::Postgres if spec.database.is_empty() => spec.user.clone(),
+        Driver::Postgres | Driver::MySql | Driver::BigQuery | Driver::Files => {
+            spec.database.clone()
+        }
+    };
+    table_name(&named)
+}
+
+/// The schema of an attached database whose tables the sidebar lists,
+/// and that `name.table` reads: PostgreSQL's `public`, MySQL's database,
+/// SQLite's `main`.
+pub fn attached_schema(spec: &crate::ConnectSpec) -> String {
+    use crate::Driver;
+    match spec.driver {
+        Driver::Postgres => "public".into(),
+        Driver::MySql => spec.database.clone(),
+        Driver::Sqlite | Driver::BigQuery | Driver::Files => "main".into(),
+    }
+}
+
+/// Whether `name` is one an attached connection cannot take: one of
+/// DuckDB's own databases, or a schema every database has.
+pub fn reserved_name(name: &str) -> bool {
+    RESERVED.contains(&name)
+}
+
+/// A reader DuckDB installs the first time a connection needs it, from
+/// DuckDB's servers, and keeps for every open after: the Excel files',
+/// and those of the databases a Files connection attaches.
+#[derive(Debug, Clone, Copy, PartialEq, Eq)]
+pub enum Reader {
+    Excel,
+    Postgres,
+    MySql,
+    Sqlite,
+}
+
+impl Reader {
+    /// The reader of a database `driver` names, where DuckDB has one.
+    pub fn of(driver: crate::Driver) -> Option<Self> {
+        use crate::Driver;
+        match driver {
+            Driver::Postgres => Some(Self::Postgres),
+            Driver::MySql => Some(Self::MySql),
+            Driver::Sqlite => Some(Self::Sqlite),
+            Driver::BigQuery | Driver::Files => None,
+        }
+    }
+
+    /// What `INSTALL` and `LOAD` are told.
+    pub(crate) fn extension(self) -> &'static str {
+        match self {
+            Self::Excel => "excel",
+            Self::Postgres => "postgres",
+            Self::MySql => "mysql",
+            Self::Sqlite => "sqlite",
+        }
+    }
+
+    /// The name `duckdb_extensions()` lists it by, installed or not.
+    pub(crate) fn listed(self) -> &'static str {
+        match self {
+            Self::Excel => "excel",
+            Self::Postgres => "postgres_scanner",
+            Self::MySql => "mysql_scanner",
+            Self::Sqlite => "sqlite_scanner",
+        }
+    }
+
+    /// What it reads, for a message.
+    pub fn label(self) -> &'static str {
+        match self {
+            Self::Excel => "Excel",
+            Self::Postgres => "PostgreSQL",
+            Self::MySql => "MySQL",
+            Self::Sqlite => "SQLite",
+        }
+    }
+}
+
+/// Told, on the session's thread, each time DuckDB is about to download a
+/// reader: what a Test's status says while it waits.
+pub type Downloading = std::sync::Arc<dyn Fn(Reader) + Send + Sync>;
+
 /// The names DuckDB keeps for its own databases, which a `.duckdb` file
 /// cannot be attached under, and the schemas every catalog has: a file
 /// attached as `information_schema` makes `information_schema.columns`
@@ -952,6 +1078,24 @@
         );
     }
 
+    /// What a session attaches prints its name and engine, never its
+    /// secrets.
+    #[test]
+    fn an_attaching_prints_without_its_secrets() {
+        let attaching = Attaching {
+            name: "bookshop_dev".into(),
+            spec: crate::ConnectSpec::default(),
+            secrets: crate::Secrets {
+                password: Some("hunter2".into()),
+                ssh_password: Some("hunter3".into()),
+                ssh_passphrase: Some("hunter4".into()),
+            },
+        };
+        let printed = format!("{attaching:?}");
+        assert!(printed.contains("bookshop_dev"), "{printed}");
+        assert!(!printed.contains("hunter"), "{printed}");
+    }
+
     #[test]
     fn table_names_are_plain_lowercase_words() {
         assert_eq!(table_name("Orders"), "orders");
diff --git a/crates/tabletist-db/src/lib.rs b/crates/tabletist-db/src/lib.rs
--- a/crates/tabletist-db/src/lib.rs
+++ b/crates/tabletist-db/src/lib.rs
@@ -94,6 +94,9 @@
     access: Access,
     /// Declared after `inner`, so the driver closes before its tunnel.
     tunnel: Option<ssh::Tunnel>,
+    /// The tunnels of the saved connections a Files connection attached,
+    /// closed after it too: held, never read.
+    _tunnels: Vec<ssh::Tunnel>,
 }
 
 enum Inner {
@@ -135,7 +138,7 @@
                     .as_ref()
                     .ok_or_else(|| Error::InvalidSpec("choose a folder or files".into()))?;
                 Ok(Self::Files(Box::new(
-                    duck::Conn::open(source, &spec.csv, false).await?,
+                    duck::Conn::open(source, &spec.csv, false, Vec::new(), None).await?,
                 )))
             }
         }
@@ -177,6 +180,7 @@
                 inner: Inner::connect(spec, secrets, None, access).await?,
                 access,
                 tunnel: None,
+                _tunnels: Vec::new(),
             });
         };
         let tunnel = ssh::Tunnel::open(ssh, &spec.host, spec.port, secrets, host_keys).await?;
@@ -185,6 +189,7 @@
                 inner,
                 access,
                 tunnel: Some(tunnel),
+                _tunnels: Vec::new(),
             }),
             Err(error) => Err(tunnel.forward_error().unwrap_or(error)),
         }
@@ -625,16 +630,97 @@
 
     /// A Files connection's session for its Test: every file it can open,
     /// those it cannot left out and said by [`Connection::read_files`],
-    /// where a session to browse does not open with one.
+    /// where a session to browse does not open with one. Nothing is
+    /// attached: see [`Connection::open_files_to_test_with`].
     pub async fn open_files_to_test(spec: &ConnectSpec) -> Result<Self> {
+        Self::open_files(spec, &[], &HostKeys::default(), true, None).await
+    }
+
+    /// [`Connection::open_files_to_test`], with the saved connections in
+    /// `attaching` attached as [`Connection::connect_files`] attaches them.
+    pub async fn open_files_to_test_with(
+        spec: &ConnectSpec,
+        attaching: &[files::Attaching],
+        host_keys: &HostKeys,
+        downloading: Option<files::Downloading>,
+    ) -> Result<Self> {
+        Self::open_files(spec, attaching, host_keys, true, downloading).await
+    }
+
+    /// A Files connection's session, read-only, with the saved
+    /// connections in `attaching` attached read-only under their names,
+    /// each through its own SSH tunnel where it has one (`host_keys` are
+    /// those the user trusts). One that cannot be attached fails it.
+    /// `downloading` is told of each reader DuckDB downloads first.
+    pub async fn connect_files(
+        spec: &ConnectSpec,
+        attaching: &[files::Attaching],
+        host_keys: &HostKeys,
+        downloading: Option<files::Downloading>,
+    ) -> Result<Self> {
+        Self::open_files(spec, attaching, host_keys, false, downloading).await
+    }
+
+    async fn open_files(
+        spec: &ConnectSpec,
+        attaching: &[files::Attaching],
+        host_keys: &HostKeys,
+        past: bool,
+        downloading: Option<files::Downloading>,
+    ) -> Result<Self> {
         let source = spec
             .files
             .as_ref()
             .ok_or_else(|| Error::InvalidSpec("choose a folder or files".into()))?;
+        let mut each = Vec::with_capacity(attaching.len());
+        let mut tunnels = Vec::new();
+        for attaching in attaching {
+            let ssh = match attaching.spec.driver {
+                Driver::Postgres | Driver::MySql => attaching.spec.ssh.as_ref(),
+                Driver::Sqlite | Driver::BigQuery | Driver::Files => None,
+            };
+            let via = match ssh {
+                Some(ssh) => {
+                    let spec = &attaching.spec;
+                    let tunnel = ssh::Tunnel::open(
+                        ssh,
+                        &spec.host,
+                        spec.port,
+                        &attaching.secrets,
+                        host_keys,
+                    )
+                    .await
+                    .map_err(|error| {
+                        Error::Connect(format!("Can't attach {}: {error}", attaching.name))
+                    })?;
+                    let port = tunnel.port;
+                    tunnels.push((attaching.name.clone(), tunnel));
+                    Some(port)
+                }
+                None => None,
+            };
+            each.push((attaching.clone(), via));
+        }
+        let opened = duck::Conn::open(source, &spec.csv, past, each, downloading).await;
+        let conn = match opened {
+            Ok(conn) => conn,
+            // Where an attach failed, a tunnel's own words say more than
+            // the server's refusal.
+            Err(Error::Connect(message)) if message.starts_with("Can't attach ") => {
+                let forwarded = tunnels.iter().find_map(|(name, tunnel)| {
+                    let error = tunnel.forward_error()?;
+                    Some(Error::Connect(format!("Can't attach {name}: {error}")))
+                });
+                return Err(forwarded.unwrap_or(Error::Connect(message)));
+            }
+            Err(error) => return Err(error),
+        };
+        let tunnels = tunnels.into_iter().map(|(_, tunnel)| tunnel).collect();
         Ok(Self {
-            inner: Inner::Files(Box::new(duck::Conn::open(source, &spec.csv, true).await?)),
+            inner: Inner::Files(Box::new(conn)),
             access: Access::ReadOnly,
             tunnel: None,
+            _tunnels: tunnels,
         })
     }
 
diff --git a/crates/tabletist-db/src/spec.rs b/crates/tabletist-db/src/spec.rs
--- a/crates/tabletist-db/src/spec.rs
+++ b/crates/tabletist-db/src/spec.rs
@@ -125,6 +125,10 @@
     /// How a Files connection reads its CSV and TSV files.
     #[serde(skip_serializing_if = "crate::files::Reading::is_default")]
     pub csv: crate::files::Reading,
+    /// The saved connections a Files connection attaches, in order; none
+    /// for every other driver.
+    #[serde(skip_serializing_if = "Vec::is_empty")]
+    pub attach: Vec<crate::files::Attach>,
 }
 
 impl Default for ConnectSpec {
@@ -142,6 +146,7 @@
             ssh: None,
             files: None,
             csv: crate::files::Reading::default(),
+            attach: Vec::new(),
         }
     }
 }
diff --git a/crates/tabletist-db/tests/ssh.rs b/crates/tabletist-db/tests/ssh.rs
--- a/crates/tabletist-db/tests/ssh.rs
+++ b/crates/tabletist-db/tests/ssh.rs
@@ -357,3 +357,40 @@
         );
     }
 }
+
+/// A Files connection attaches PostgreSQL's and MySQL's Bookshops, each
+/// through a tunnel of its own, and reads both.
+#[tokio::test]
+async fn a_files_connection_attaches_through_tunnels() {
+    let Some((ssh, password)) = server() else {
+        return;
+    };
+    let keys = trusted(&ssh).await;
+    let dir = tempfile::tempdir().unwrap();
+    tabletist_db::fixtures::write_bookshop_files(dir.path()).unwrap();
+    let secrets = secrets_with_db(with_password(&password));
+    let mut postgres = pg_via(&ssh);
+    postgres.database = "bookshop".into();
+    let mut mysql = mysql_via(&ssh);
+    mysql.database = "bookshop".into();
+    let attaching = [
+        tabletist_db::files::Attaching {
+            name: "pg_shop".into(),
+            spec: postgres,
+            secrets: secrets.clone(),
+        },
+        tabletist_db::files::Attaching {
+            name: "my_shop".into(),
+            spec: mysql,
+            secrets,
+        },
+    ];
+    let spec = ConnectSpec::files(tabletist_db::files::Source::Folder(dir.path().to_owned()));
+    let connection = Connection::connect_files(&spec, &attaching, &keys, None)
+        .await
+        .unwrap();
+    for schema in ["pg_shop", "my_shop"] {
+        let books = RowQuery::new(ObjectRef::new(schema, "books"), 100);
+        assert_eq!(connection.count_rows(&books).await.unwrap(), 12, "{schema}");
+    }
+}
````

- [ ] **Step 4: Run them to see them pass**

Run: `TABLETIST_TEST_PG_URL=postgres://tabletist:tabletist@localhost:55432/tabletist TABLETIST_TEST_MYSQL_URL=mysql://tabletist:tabletist@localhost:53306/tabletist ~/.cargo/bin/cargo test -q -p tabletist-db --test files_attach --lib attach`

Expected: 6 pass in the lib: the four `duck::attach` tests, `files::tests::an_attaching_prints_without_its_secrets`, and the existing `sqlite::tests::schemas_are_main_and_attached_databases_without_temp`, which the filter takes too. The five `files_attach` tests pass; without the URLs, the server ones say they're skipped. Then `~/.cargo/bin/cargo test -q -p tabletist-db --test files` passes unchanged, and `~/.cargo/bin/cargo test -q -p tabletist-db --test ssh a_files_connection` says it's skipped without `TABLETIST_TEST_SSH_URL`.

- [ ] **Step 5: Run the full check**

Run the five commands of the full check above.

Expected: all pass; 5,700 passed, 0 failed.

- [ ] **Step 6: Commit**

```bash
git add crates/tabletist-db Cargo.toml .github/workflows/ci.yml crates/tabletist-core/src/connections/file.rs crates/tabletist-core/src/model.rs
git commit -m "Attach saved PostgreSQL, MySQL and SQLite connections to a Files session, read-only"
```

### Task 2: Kept, sent and read from the keyring

**Files:**
- Create: `crates/tabletist-core/src/connections/join.rs`, `crates/tabletist-core/src/backend/attach.rs`
- Modify: `crates/tabletist-core/src/connections.rs`, `connections/file.rs`, `backend.rs`, `model.rs`, `src/app.rs`

- [ ] **Step 1: Write the failing tests**

What the section lists, and why a row can't be ticked; a new attachment's name; the names' check; the keyring read at attach time, and why one can't be attached; `[[connections.attach]]` written and read back, and what the reader refuses.

````diff
diff --git a/crates/tabletist-core/src/backend.rs b/crates/tabletist-core/src/backend.rs
--- a/crates/tabletist-core/src/backend.rs
+++ b/crates/tabletist-core/src/backend.rs
@@ -19,6 +19,7 @@
 use crate::secrets::{Keyring, SecretString};
 
 mod agents;
+pub mod attach;
 pub mod export;
 mod history;
 mod updates;
diff --git a/crates/tabletist-core/src/backend/attach.rs b/crates/tabletist-core/src/backend/attach.rs
new file mode 100644
--- /dev/null
+++ b/crates/tabletist-core/src/backend/attach.rs
@@ -0,0 +1,91 @@
+//! The saved connections a Files connection's session attaches: what the
+//! app sends with a Connect or a Test, and their secrets, which only the
+//! backend reads, from the keyring, as the session opens. No password
+//! comes from the app, or goes into the spec, the log or History.
+
+
+#[cfg(test)]
+mod tests {
+    use super::*;
+
+    fn to(name: &str, spec: ConnectSpec, password: PasswordMode) -> AttachTo {
+        AttachTo {
+            name: name.into(),
+            saved: Some(AttachSaved {
+                id: ConnectionId(format!("id-{name}")),
+                spec,
+                password,
+                ssh_secret: PasswordMode::None,
+            }),
+        }
+    }
+
+    fn postgres() -> ConnectSpec {
+        ConnectSpec {
+            user: "reader".into(),
+            database: "bookshop".into(),
+            ..ConnectSpec::default()
+        }
+    }
+
+    #[tokio::test]
+    async fn a_password_is_read_from_the_keyring_as_it_is_attached() {
+        let keyring = Keyring::memory();
+        keyring
+            .write(
+                &password_account(&ConnectionId("id-shop".into())),
+                crate::secrets::SecretString("s3cret".into()),
+            )
+            .wait()
+            .await
+            .unwrap();
+        let each = attaching(
+            vec![
+                to("shop", postgres(), PasswordMode::Keyring),
+                to(
+                    "lite",
+                    ConnectSpec::sqlite("/data/bookshop.db"),
+                    PasswordMode::None,
+                ),
+            ],
+            &keyring,
+        )
+        .await
+        .unwrap();
+        assert_eq!(each[0].name, "shop");
+        assert_eq!(each[0].secrets.password.as_deref(), Some("s3cret"));
+        assert_eq!(each[1].secrets, Secrets::default());
+    }
+
+    #[tokio::test]
+    async fn one_that_cannot_be_attached_says_why() {
+        let keyring = Keyring::memory();
+        let why = |attach: AttachTo| {
+            let keyring = keyring.clone();
+            async move {
+                match attaching(vec![attach], &keyring).await {
+                    Err(error) => error.to_string(),
+                    Ok(_) => panic!("it was attached"),
+                }
+            }
+        };
+        assert!(
+            why(to("shop", postgres(), PasswordMode::Keyring))
+                .await
+                .contains("Can't attach shop: the keyring has no password for it")
+        );
+        assert!(
+            why(to("shop", postgres(), PasswordMode::Ask))
+                .await
+                .contains("Can't attach shop: its password is asked for on every connect")
+        );
+        assert!(
+            why(AttachTo {
+                name: "shop".into(),
+                saved: None,
+            })
+            .await
+            .contains("Can't attach shop: its saved connection was deleted")
+        );
+    }
+}
diff --git a/crates/tabletist-core/src/connections.rs b/crates/tabletist-core/src/connections.rs
--- a/crates/tabletist-core/src/connections.rs
+++ b/crates/tabletist-core/src/connections.rs
@@ -13,6 +13,7 @@
 
 pub(crate) mod file;
 pub mod groups;
+pub mod join;
 
 #[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
 pub struct ConnectionId(pub String);
diff --git a/crates/tabletist-core/src/connections/file.rs b/crates/tabletist-core/src/connections/file.rs
--- a/crates/tabletist-core/src/connections/file.rs
+++ b/crates/tabletist-core/src/connections/file.rs
@@ -1281,8 +1281,8 @@
         let said = |text: &str| read(text).unwrap_err().to_string();
         assert!(said("").contains("missing field `version`"), "{}", said(""));
         assert_eq!(
-            said("version = 10\n"),
-            "it is version 10 of the file, written by a newer Tabletist; this one reads version 9"
+            said("version = 11\n"),
+            "it is version 11 of the file, written by a newer Tabletist; this one reads version 10"
         );
         assert_eq!(
             said("version = 2\n"),
@@ -1889,6 +1889,90 @@
         }
     }
 
+    /// The saved connections a Files connection attaches, by id, after its
+    /// source and CSV tables, make the file version 10.
+    #[test]
+    fn what_a_files_connection_attaches_is_written_in_version_ten() {
+        let mut spec = ConnectSpec::files(Source::Folder("/srv/bookshop-exports".into()));
+        spec.attach = vec![
+            Attach {
+                connection: "dev".into(),
+                name: "bookshop_dev".into(),
+            },
+            Attach {
+                connection: "file".into(),
+                name: "development".into(),
+            },
+        ];
+        let exports = saved("exports", "Bookshop exports", spec);
+        let mut connections = bookshop();
+        connections.push(exports);
+        let text = write(&connections).unwrap();
+        assert!(text.contains("\nversion = 10\n"), "{text}");
+        assert!(
+            text.ends_with(
+                r#"[connections.source]
+folder = "/srv/bookshop-exports"
+
+[[connections.attach]]
+connection = "dev"
+as         = "bookshop_dev"
+
+[[connections.attach]]
+connection = "file"
+as         = "development"
+"#
+            ),
+            "{text}"
+        );
+        assert_eq!(read(&text).unwrap().connections, connections);
+        // Without one, the file is as it was.
+        connections.last_mut().unwrap().spec.attach.clear();
+        assert!(write(&connections).unwrap().contains("\nversion = 7\n"));
+    }
+
+    #[test]
+    fn an_attach_is_a_files_connection_s_with_a_plain_name_once() {
+        let entry = |driver: &str, more: &str| {
+            format!(
+                "version = 10\n[[connections]]\nid = \"a\"\nname = \"A\"\n\
+                 driver = \"{driver}\"\n{more}"
+            )
+        };
+        let source = "[connections.source]\nfolder = \"/a\"\n";
+        let attach = |connection: &str, name: &str| {
+            format!("[[connections.attach]]\nconnection = \"{connection}\"\nas = \"{name}\"\n")
+        };
+        for (text, says) in [
+            (
+                entry("sqlite", &attach("b", "shop")),
+                "only a Files connection attaches others",
+            ),
+            (
+                entry("files", &format!("{source}{}", attach(" ", "shop"))),
+                "an attach names no connection",
+            ),
+            (
+                entry("files", &format!("{source}{}", attach("b", "Shop-Dev"))),
+                "cannot name an attached connection",
+            ),
+            (
+                entry("files", &format!("{source}{}", attach("b", "memory"))),
+                "cannot name an attached connection",
+            ),
+            (
+                entry(
+                    "files",
+                    &format!("{source}{}{}", attach("b", "shop"), attach("c", "shop")),
+                ),
+                "attaches two connections as \"shop\"",
+            ),
+        ] {
+            let problem = read(&text).unwrap_err();
+            assert!(problem.what.contains(says), "{text}\n{problem}");
+        }
+    }
+
     #[test]
     fn a_source_is_a_folder_or_files_and_only_a_files_connections() {
         let entry = |driver: &str, source: &str| {
diff --git a/crates/tabletist-core/src/connections/join.rs b/crates/tabletist-core/src/connections/join.rs
new file mode 100644
--- /dev/null
+++ b/crates/tabletist-core/src/connections/join.rs
@@ -0,0 +1,159 @@
+//! The saved connections a Files connection can attach, as its Join
+//! section lists them, and the name SQL calls each by once it is.
+
+
+#[cfg(test)]
+mod tests {
+    use super::*;
+    use crate::connections::Agents;
+    use tabletist_db::ConnectSpec;
+
+    fn saved(name: &str, environment: Environment, spec: ConnectSpec) -> SavedConnection {
+        SavedConnection {
+            id: ConnectionId(format!("id-{name}")),
+            name: name.into(),
+            group: None,
+            environment,
+            read_only: None,
+            password: PasswordMode::Keyring,
+            ssh_secret: PasswordMode::None,
+            spec,
+            agents: Agents::default(),
+        }
+    }
+
+    fn postgres(database: &str) -> ConnectSpec {
+        ConnectSpec {
+            driver: Driver::Postgres,
+            user: "reader".into(),
+            database: database.into(),
+            ..ConnectSpec::default()
+        }
+    }
+
+    #[test]
+    fn only_databases_duckdb_reads_are_listed_with_why_one_cannot_be_ticked() {
+        let mut asks = saved(
+            "Bookshop",
+            Environment::Staging,
+            postgres("bookshop_staging"),
+        );
+        asks.password = PasswordMode::Ask;
+        let mysql = ConnectSpec {
+            driver: Driver::MySql,
+            user: "reader".into(),
+            ..ConnectSpec::default()
+        };
+        let connections = [
+            saved(
+                "Bookshop",
+                Environment::Dev,
+                postgres("bookshop_development"),
+            ),
+            asks,
+            saved("Bookshop", Environment::Production, postgres("")),
+            saved("Shop", Environment::Local, mysql),
+            saved(
+                "Bookshop",
+                Environment::Local,
+                ConnectSpec::sqlite("/data/development.sqlite3"),
+            ),
+            saved(
+                "Exports",
+                Environment::Local,
+                ConnectSpec::files(tabletist_db::files::Source::Folder("/data".into())),
+            ),
+        ];
+        let listed = joinable(&connections);
+        let said: Vec<(&str, &str, bool, Option<Blocked>)> = listed
+            .iter()
+            .map(|each| {
+                (
+                    each.name.as_str(),
+                    each.database.as_str(),
+                    each.read_only,
+                    each.blocked,
+                )
+            })
+            .collect();
+        assert_eq!(
+            said,
+            [
+                ("Bookshop", "bookshop_development", false, None),
+                (
+                    "Bookshop",
+                    "bookshop_staging",
+                    false,
+                    Some(Blocked::AsksPassword)
+                ),
+                ("Bookshop", "reader", true, None),
+                ("Shop", "", false, Some(Blocked::NoDatabase)),
+                ("Bookshop", "development.sqlite3", false, None),
+            ]
+        );
+    }
+
+    #[test]
+    fn a_name_is_the_database_s_until_it_is_taken() {
+        let dev = saved(
+            "Bookshop",
+            Environment::Dev,
+            postgres("bookshop_development"),
+        );
+        assert_eq!(default_name(&dev, &[], &[]), "bookshop_development");
+        let attached = [Attach {
+            connection: "other".into(),
+            name: "bookshop_development".into(),
+        }];
+        assert_eq!(
+            default_name(&dev, &attached, &["bookshop_development_2".into()]),
+            "bookshop_development_3"
+        );
+        let file = saved(
+            "Bookshop",
+            Environment::Local,
+            ConnectSpec::sqlite("/data/Development-2.sqlite3"),
+        );
+        assert_eq!(default_name(&file, &[], &[]), "development_2");
+        // One of DuckDB's own names is never given.
+        let main = saved("Main", Environment::Local, postgres("main"));
+        assert_eq!(default_name(&main, &[], &[]), "main_2");
+    }
+
+    #[test]
+    fn a_name_must_be_plain_free_and_once() {
+        let named = |names: &[&str]| -> Vec<Attach> {
+            names
+                .iter()
+                .map(|name| Attach {
+                    connection: format!("id-{name}"),
+                    name: (*name).into(),
+                })
+                .collect()
+        };
+        assert_eq!(name_problem(&named(&["bookshop_dev", "shop"]), &[]), None);
+        assert_eq!(
+            name_problem(&named(&[""]), &[]).as_deref(),
+            Some("Name each attached connection.")
+        );
+        assert_eq!(
+            name_problem(&named(&["Bookshop-dev"]), &[]).as_deref(),
+            Some(
+                "Bookshop-dev can't name an attached connection: use lowercase letters, \
+                 digits and _."
+            )
+        );
+        assert_eq!(
+            name_problem(&named(&["bookshop"]), &["bookshop".into()]).as_deref(),
+            Some("bookshop is taken: name the attached connection another way.")
+        );
+        assert_eq!(
+            name_problem(&named(&["memory"]), &[]).as_deref(),
+            Some("memory is taken: name the attached connection another way.")
+        );
+        assert_eq!(
+            name_problem(&named(&["shop", "shop"]), &[]).as_deref(),
+            Some("Two attached connections are named shop.")
+        );
+    }
+}
````

- [ ] **Step 2: Run them to see them fail**

Run: `~/.cargo/bin/cargo test -q -p tabletist-core --lib connections`

Expected: they don't compile. `joinable`, `default_name`, `name_problem`, `AttachTo` and `attaching` aren't there yet.

```text
error[E0425]: cannot find type `ConnectSpec` in this scope
error[E0425]: cannot find type `PasswordMode` in this scope
error[E0425]: cannot find type `AttachTo` in this scope
error[E0422]: cannot find struct, variant or union type `AttachTo` in this scope
error[E0422]: cannot find struct, variant or union type `AttachSaved` in this scope
error[E0425]: cannot find function, tuple struct or tuple variant `ConnectionId` in this scope
error[E0433]: cannot find type `PasswordMode` in this scope
error[E0422]: cannot find struct, variant or union type `ConnectSpec` in this scope
error[E0433]: cannot find type `Keyring` in this scope
error[E0425]: cannot find function `password_account` in this scope
error[E0422]: cannot find struct, variant or union type `Attach` in this scope
error[E0425]: cannot find type `Environment` in this scope
error[E0425]: cannot find type `SavedConnection` in this scope
error[E0422]: cannot find struct, variant or union type `SavedConnection` in this scope
error[E0433]: cannot find type `Environment` in this scope
error[E0425]: cannot find type `Blocked` in this scope
error[E0433]: cannot find type `Blocked` in this scope
    |
error[E0425]: cannot find type `Attach` in this scope
error[E0433]: cannot find type `ConnectSpec` in this scope
error[E0425]: cannot find function `attaching` in this scope
error[E0433]: cannot find type `Secrets` in this scope
error[E0433]: cannot find type `Driver` in this scope
error[E0425]: cannot find function `joinable` in this scope
error[E0425]: cannot find function `default_name` in this scope
error[E0425]: cannot find function `name_problem` in this scope
error: could not compile `tabletist-core` (lib test) due to 61 previous errors; 2 warnings emitted
```

- [ ] **Step 3: The form's attachments, what Connect and Test carry, the keyring read**

`Command::Connect` and `Command::Test` gain `attach`, and the backend's own tests send none. A Connect with attachments opens through `Connection::connect_files`; one with none goes through `connect_with` as before.

````diff
diff --git a/crates/tabletist-core/src/backend.rs b/crates/tabletist-core/src/backend.rs
--- a/crates/tabletist-core/src/backend.rs
+++ b/crates/tabletist-core/src/backend.rs
@@ -71,6 +71,9 @@
         host_keys: HostKeys,
         /// Whether the session may write.
         access: Access,
+        /// The saved connections a Files connection attaches: their
+        /// secrets are read from the keyring as it opens.
+        attach: Vec<attach::AttachTo>,
     },
     /// Connect and close again, for the dialog's Test button.
     Test {
@@ -78,6 +81,8 @@
         spec: ConnectSpec,
         secrets: Secrets,
         host_keys: HostKeys,
+        /// As a Connect's.
+        attach: Vec<attach::AttachTo>,
     },
     /// Stops the check in full that a Files connection's Test `request`
     /// started, where it still runs: the dialog closed, or what it tested
@@ -496,6 +501,12 @@
         request: RequestId,
         result: Result<tabletist_db::files::Scan, String>,
     },
+    /// DuckDB is downloading `reader` for Test `request`: the Test's
+    /// status says so until it answers.
+    FilesDownloading {
+        request: RequestId,
+        reader: tabletist_db::files::Reader,
+    },
     /// A Files connection's Test: what it read of every file.
     FilesTested {
         request: RequestId,
@@ -2332,47 +2343,56 @@
                 secrets,
                 host_keys,
                 access,
+                attach,
             } => {
                 self.connecting.insert(session);
                 let outbox = self.outbox.clone();
                 let ready = self.ready.clone();
+                let keyring = self.keyring.clone();
                 tokio::spawn(async move {
                     let started = tokio::time::Instant::now();
-                    let outcome = Connection::connect_with(&spec, &secrets, &host_keys, access)
-                        .await
-                        .map(|connection| {
-                            let driver = connection.driver();
-                            let encrypted = connection.is_encrypted();
-                            // What the session says it is, not what was
-                            // asked for: the UI shows and trusts this.
-                            let opened = connection.access();
-                            let cancel = connection.cancel_handle();
-                            let (queue, commands) = tokio_mpsc::unbounded_channel();
-                            let (stop, stopped) = tokio::sync::oneshot::channel();
-                            let running = Arc::new(Mutex::new(Running {
-                                grace: grace_after(started.elapsed()),
-                                ..Running::default()
-                            }));
-                            tokio::spawn(run_session(
-                                session,
-                                connection,
-                                commands,
-                                stopped,
-                                Arc::clone(&running),
-                                outbox,
-                            ));
-                            (
-                                driver,
-                                encrypted,
-                                opened,
-                                SessionHandle {
-                                    queue,
-                                    cancel,
-                                    running,
-                                    _stop: stop,
-                                },
-                            )
-                        });
+                    let connected = match attach.is_empty() {
+                        true => Connection::connect_with(&spec, &secrets, &host_keys, access).await,
+                        false => match attach::attaching(attach, &keyring).await {
+                            Ok(attaching) => {
+                                Connection::connect_files(&spec, &attaching, &host_keys, None).await
+                            }
+                            Err(error) => Err(error),
+                        },
+                    };
+                    let outcome = connected.map(|connection| {
+                        let driver = connection.driver();
+                        let encrypted = connection.is_encrypted();
+                        // What the session says it is, not what was
+                        // asked for: the UI shows and trusts this.
+                        let opened = connection.access();
+                        let cancel = connection.cancel_handle();
+                        let (queue, commands) = tokio_mpsc::unbounded_channel();
+                        let (stop, stopped) = tokio::sync::oneshot::channel();
+                        let running = Arc::new(Mutex::new(Running {
+                            grace: grace_after(started.elapsed()),
+                            ..Running::default()
+                        }));
+                        tokio::spawn(run_session(
+                            session,
+                            connection,
+                            commands,
+                            stopped,
+                            Arc::clone(&running),
+                            outbox,
+                        ));
+                        (
+                            driver,
+                            encrypted,
+                            opened,
+                            SessionHandle {
+                                queue,
+                                cancel,
+                                running,
+                                _stop: stop,
+                            },
+                        )
+                    });
                     let _ = ready.send(Ready {
                         session,
                         request,
@@ -2385,6 +2405,7 @@
                 spec,
                 secrets,
                 host_keys,
+                attach,
             } => {
                 let outbox = self.outbox.clone();
                 // A Files connection's Test reads the start of every file
@@ -2399,8 +2420,25 @@
                     | tabletist_db::Driver::BigQuery => false,
                 };
                 if files {
+                    let keyring = self.keyring.clone();
+                    let told = outbox.clone();
+                    let downloading: tabletist_db::files::Downloading = Arc::new(move |reader| {
+                        told.emit(Event::FilesDownloading { request, reader });
+                    });
                     let task = tokio::spawn(async move {
-                        let connection = match Connection::open_files_to_test(&spec).await {
+                        let opened = match attach::attaching(attach, &keyring).await {
+                            Ok(attaching) => {
+                                Connection::open_files_to_test_with(
+                                    &spec,
+                                    &attaching,
+                                    &host_keys,
+                                    Some(downloading),
+                                )
+                                .await
+                            }
+                            Err(error) => Err(error),
+                        };
+                        let connection = match opened {
                             Ok(connection) => connection,
                             Err(error) => {
                                 outbox.emit(Event::FilesTested {
@@ -4055,6 +4093,7 @@
                 secrets: Secrets::default(),
                 host_keys: HostKeys::default(),
                 access,
+                attach: Vec::new(),
             });
             match backend.wait(WAIT) {
                 Some(Event::Connected { access: opened, .. }) => assert_eq!(opened, access),
@@ -4076,6 +4115,7 @@
             secrets: Secrets::default(),
             host_keys: HostKeys::default(),
             access: Access::ReadOnly,
+            attach: Vec::new(),
         });
         assert!(matches!(
             backend.wait(WAIT),
@@ -4237,6 +4277,7 @@
             secrets: Secrets::default(),
             host_keys: HostKeys::default(),
             access: Access::ReadOnly,
+            attach: Vec::new(),
         });
         assert!(matches!(
             backend.wait(WAIT),
@@ -4326,6 +4367,7 @@
             spec,
             secrets: Secrets::default(),
             host_keys: HostKeys::default(),
+            attach: Vec::new(),
         });
         let checks = match backend.wait(WAIT) {
             Some(Event::FilesTested {
@@ -4378,6 +4420,7 @@
                 spec,
                 secrets: Secrets::default(),
                 host_keys: HostKeys::default(),
+                attach: Vec::new(),
             });
             worker.handle(Command::StopCheck {
                 request: RequestId(8),
@@ -4402,6 +4445,7 @@
             spec,
             secrets: Secrets::default(),
             host_keys: HostKeys::default(),
+            attach: Vec::new(),
         });
         assert!(matches!(
             backend.wait(WAIT),
@@ -4424,6 +4468,7 @@
             secrets: Secrets::default(),
             host_keys: HostKeys::default(),
             access: Access::ReadOnly,
+            attach: Vec::new(),
         });
         assert!(matches!(backend.wait(WAIT), Some(Event::Connected { .. })));
         let mut slow = RowQuery::new(ObjectRef::new("main", "big"), 10);
@@ -4481,6 +4526,7 @@
             secrets: Secrets::default(),
             host_keys: HostKeys::default(),
             access: Access::ReadOnly,
+            attach: Vec::new(),
         });
         assert!(matches!(backend.wait(WAIT), Some(Event::Connected { .. })));
         (backend, session)
@@ -4969,6 +5015,7 @@
             secrets: Secrets::default(),
             host_keys: HostKeys::default(),
             access: Access::ReadOnly,
+            attach: Vec::new(),
         });
         for command in queued {
             worker.handle(command);
@@ -7447,6 +7494,7 @@
             secrets: Secrets::default(),
             host_keys: HostKeys::default(),
             access: Access::ReadOnly,
+            attach: Vec::new(),
         });
         assert!(matches!(backend.wait(WAIT), Some(Event::Connected { .. })));
         backend.send(Command::Close { session });
@@ -7474,6 +7522,7 @@
             secrets: Secrets::default(),
             host_keys: HostKeys::default(),
             access: Access::ReadOnly,
+            attach: Vec::new(),
         });
         assert!(matches!(backend.wait(WAIT), Some(Event::Connected { .. })));
         let mut slow = RowQuery::new(ObjectRef::new("main", "big"), 10);
@@ -7585,6 +7634,7 @@
                 ..Secrets::default()
             },
             host_keys: HostKeys::default(),
+            attach: Vec::new(),
         };
         assert!(!format!("{command:?}").contains("hunter2"));
     }
@@ -7666,6 +7716,7 @@
             secrets: Secrets::default(),
             host_keys: HostKeys::default(),
             access: Access::ReadOnly,
+            attach: Vec::new(),
         });
         assert!(matches!(backend.wait(WAIT), Some(Event::Connected { .. })));
         backend.send(Command::ListDatabases {
@@ -7719,6 +7770,7 @@
             secrets: Secrets::default(),
             host_keys: HostKeys::default(),
             access,
+            attach: Vec::new(),
         });
         assert!(matches!(backend.wait(WAIT), Some(Event::Connected { .. })));
         (dir, backend, session)
diff --git a/crates/tabletist-core/src/backend/attach.rs b/crates/tabletist-core/src/backend/attach.rs
--- a/crates/tabletist-core/src/backend/attach.rs
+++ b/crates/tabletist-core/src/backend/attach.rs
@@ -3,6 +3,105 @@
 //! backend reads, from the keyring, as the session opens. No password
 //! comes from the app, or goes into the spec, the log or History.
 
+use tabletist_db::files::{Attach, Attaching};
+use tabletist_db::{ConnectSpec, Error, Secrets};
+
+use crate::connections::{ConnectionId, PasswordMode, SavedConnections};
+use crate::model::SecretKind;
+use crate::secrets::{Keyring, password_account, ssh_account};
+
+/// A saved connection to attach: the name SQL calls it by, and the
+/// saved connection as it is now, `None` when it was deleted.
+#[derive(Debug, Clone, PartialEq)]
+pub struct AttachTo {
+    pub name: String,
+    pub saved: Option<AttachSaved>,
+}
+
+/// What a saved connection to attach is: how it is reached, and where
+/// its secrets are kept.
+#[derive(Debug, Clone, PartialEq)]
+pub struct AttachSaved {
+    pub id: ConnectionId,
+    pub spec: ConnectSpec,
+    pub password: PasswordMode,
+    pub ssh_secret: PasswordMode,
+}
+
+impl AttachTo {
+    /// What `attach` names, among the saved `connections`.
+    pub fn of(attach: &[Attach], connections: &SavedConnections) -> Vec<Self> {
+        attach
+            .iter()
+            .map(|each| Self {
+                name: each.name.clone(),
+                saved: connections
+                    .get(&ConnectionId(each.connection.clone()))
+                    .map(|saved| AttachSaved {
+                        id: saved.id.clone(),
+                        spec: saved.spec.clone(),
+                        password: saved.password,
+                        ssh_secret: saved.ssh_secret,
+                    }),
+            })
+            .collect()
+    }
+}
+
+/// Each of `attach` with its secrets, read from `keyring` now: why one
+/// cannot be attached, where one cannot.
+pub(super) async fn attaching(
+    attach: Vec<AttachTo>,
+    keyring: &Keyring,
+) -> Result<Vec<Attaching>, Error> {
+    let mut each = Vec::with_capacity(attach.len());
+    for to in attach {
+        let name = to.name;
+        let cannot = |why: &str| Error::Connect(format!("Can't attach {name}: {why}"));
+        let Some(saved) = to.saved else {
+            return Err(cannot("its saved connection was deleted"));
+        };
+        let mut secrets = Secrets::default();
+        let ssh = SecretKind::for_ssh(&saved.spec);
+        let needed = [
+            (
+                Some(SecretKind::Database),
+                saved.password,
+                password_account(&saved.id),
+            ),
+            (
+                ssh,
+                match ssh {
+                    Some(_) => saved.ssh_secret,
+                    None => PasswordMode::None,
+                },
+                ssh_account(&saved.id),
+            ),
+        ];
+        for (kind, mode, account) in needed {
+            let Some(kind) = kind else {
+                continue;
+            };
+            match mode {
+                PasswordMode::None => {}
+                PasswordMode::Ask => {
+                    return Err(cannot("its password is asked for on every connect"));
+                }
+                PasswordMode::Keyring => match keyring.read(&account).wait().await {
+                    Ok(Some(secret)) => *kind.slot(&mut secrets) = Some(secret),
+                    Ok(None) => return Err(cannot("the keyring has no password for it")),
+                    Err(error) => return Err(cannot(&error.to_string())),
+                },
+            }
+        }
+        each.push(Attaching {
+            name,
+            spec: saved.spec,
+            secrets,
+        });
+    }
+    Ok(each)
+}
 
 #[cfg(test)]
 mod tests {
diff --git a/crates/tabletist-core/src/connections/file.rs b/crates/tabletist-core/src/connections/file.rs
--- a/crates/tabletist-core/src/connections/file.rs
+++ b/crates/tabletist-core/src/connections/file.rs
@@ -12,7 +12,7 @@
 
 use serde::Deserialize;
 use tabletist_db::agent::{ColumnName, TableName, Tables};
-use tabletist_db::files::{Change, Delimiter, Header, Override, Reading, Source, Types};
+use tabletist_db::files::{Attach, Change, Delimiter, Header, Override, Reading, Source, Types};
 use tabletist_db::{ConnectSpec, Driver, SshAuth, SshSpec, TlsMode};
 
 use super::{
@@ -31,8 +31,12 @@
 /// know a key refuses only a file it would lose it from, and one that
 /// would read agents' tables as all of them, or a masked column's values
 /// as they are, never reads the file. 9 has the group the person put a
-/// connection in (`group`).
-pub(super) const VERSION: u32 = 9;
+/// connection in (`group`), and 10 the saved connections a Files
+/// connection attaches (`[[connections.attach]]`).
+pub(super) const VERSION: u32 = 10;
+
+/// The version that has groups and no connection a Files one attaches.
+const GROUPS_VERSION: u32 = 9;
 
 /// The version that has how Files connections read CSV and no group.
 const CSV_VERSION: u32 = 8;
@@ -132,6 +136,41 @@
     csv: Option<CsvTable>,
     #[serde(default, rename = "override")]
     overrides: Vec<OverrideTable>,
+    #[serde(default)]
+    attach: Vec<AttachTable>,
+}
+
+/// A saved connection a Files connection attaches, as the file has it: by
+/// its id, which a rename leaves alone, and the name SQL calls it by.
+#[derive(Deserialize)]
+#[serde(deny_unknown_fields)]
+struct AttachTable {
+    connection: String,
+    #[serde(rename = "as")]
+    name: String,
+}
+
+impl AttachTable {
+    fn attach(self, connection: &str) -> Result<Attach, String> {
+        if self.connection.trim().is_empty() {
+            return Err(format!(
+                "connection {connection:?}: an attach names no connection"
+            ));
+        }
+        let name = self.name;
+        if tabletist_db::files::table_name(&name) != name
+            || tabletist_db::files::reserved_name(&name)
+        {
+            return Err(format!(
+                "connection {connection:?}: {name:?} cannot name an attached connection; \
+                 a name is lowercase letters, digits and _, and not one of DuckDB's own"
+            ));
+        }
+        Ok(Attach {
+            connection: self.connection,
+            name,
+        })
+    }
 }
 
 /// How a Files connection reads its CSV files, as the file has it: each
@@ -442,6 +481,31 @@
             Driver::Postgres | Driver::MySql | Driver::Sqlite | Driver::BigQuery => {
                 Reading::default()
             }
+        };
+        let attach = match self.driver {
+            Driver::Files => {
+                let mut attach: Vec<Attach> = Vec::with_capacity(self.attach.len());
+                for table in self.attach {
+                    let each = table.attach(&self.name)?;
+                    if attach.iter().any(|other| other.name == each.name) {
+                        return Err(format!(
+                            "connection {:?} attaches two connections as {:?}",
+                            self.name, each.name
+                        ));
+                    }
+                    attach.push(each);
+                }
+                attach
+            }
+            Driver::Postgres | Driver::MySql | Driver::Sqlite | Driver::BigQuery
+                if !self.attach.is_empty() =>
+            {
+                return Err(format!(
+                    "connection {:?}: only a Files connection attaches others",
+                    self.name
+                ));
+            }
+            Driver::Postgres | Driver::MySql | Driver::Sqlite | Driver::BigQuery => Vec::new(),
         };
         // A blank group is none: the connection goes by the rule.
         let group = self
@@ -469,7 +533,7 @@
                 ssh,
                 files,
                 csv,
-                attach: Vec::new(),
+                attach,
             },
             agents,
         })
@@ -720,14 +784,18 @@
     let groups = connections
         .iter()
         .any(|saved| written_group(saved).is_some());
-    let version = match (groups, csv, files, masks, hides, agents) {
-        (true, _, _, _, _, _) => VERSION,
-        (false, true, _, _, _, _) => CSV_VERSION,
-        (false, false, true, _, _, _) => FILES_VERSION,
-        (false, false, false, true, _, _) => MASKED_VERSION,
-        (false, false, false, false, true, _) => TABLES_VERSION,
-        (false, false, false, false, false, true) => AGENTS_VERSION,
-        (false, false, false, false, false, false) => FIRST_VERSION,
+    let attaches = connections
+        .iter()
+        .any(|saved| saved.spec.files.is_some() && !saved.spec.attach.is_empty());
+    let version = match (attaches, groups, csv, files, masks, hides, agents) {
+        (true, _, _, _, _, _, _) => VERSION,
+        (false, true, _, _, _, _, _) => GROUPS_VERSION,
+        (false, false, true, _, _, _, _) => CSV_VERSION,
+        (false, false, false, true, _, _, _) => FILES_VERSION,
+        (false, false, false, false, true, _, _) => MASKED_VERSION,
+        (false, false, false, false, false, true, _) => TABLES_VERSION,
+        (false, false, false, false, false, false, true) => AGENTS_VERSION,
+        (false, false, false, false, false, false, false) => FIRST_VERSION,
     };
     let mut out = format!("{HEADER}version = {version}\n");
     for connection in connections {
@@ -804,6 +872,13 @@
             out.push_str("\n[connections.source]\n");
             lines.write(&mut out);
             csv_tables(&spec.csv, &mut out);
+            for each in &spec.attach {
+                let mut lines = Lines::default();
+                lines.text("connection", &each.connection);
+                lines.text("as", &each.name);
+                out.push_str("\n[[connections.attach]]\n");
+                lines.write(&mut out);
+            }
         }
         if !connection.agents.is_default() {
             let mut lines = Lines::default();
diff --git a/crates/tabletist-core/src/connections/join.rs b/crates/tabletist-core/src/connections/join.rs
--- a/crates/tabletist-core/src/connections/join.rs
+++ b/crates/tabletist-core/src/connections/join.rs
@@ -1,6 +1,134 @@
 //! The saved connections a Files connection can attach, as its Join
 //! section lists them, and the name SQL calls each by once it is.
 
+use tabletist_db::Driver;
+use tabletist_db::files::Attach;
+
+use super::{ConnectionId, PasswordMode, SavedConnection};
+use crate::env::Environment;
+
+/// A saved connection the Join section lists: what its row says, and why
+/// it cannot be attached where it cannot.
+#[derive(Debug, Clone, PartialEq, Eq)]
+pub struct Joinable {
+    pub id: ConnectionId,
+    pub name: String,
+    pub environment: Environment,
+    pub driver: Driver,
+    /// Its database's name, or a SQLite file's: what the row says after
+    /// the engine.
+    pub database: String,
+    /// It opens read-only: production, or its box ticked.
+    pub read_only: bool,
+    pub blocked: Option<Blocked>,
+}
+
+/// Why a saved connection's row cannot be ticked.
+#[derive(Debug, Clone, Copy, PartialEq, Eq)]
+pub enum Blocked {
+    /// Its password is asked for on every connect: no keyring holds it
+    /// to attach it with.
+    AsksPassword,
+    /// A MySQL connection that names no database: DuckDB has no schema
+    /// for `name.table` to read.
+    NoDatabase,
+}
+
+/// The saved PostgreSQL, MySQL and SQLite connections among
+/// `connections`, in the list's order: those a Files connection can
+/// attach.
+pub fn joinable(connections: &[SavedConnection]) -> Vec<Joinable> {
+    connections
+        .iter()
+        .filter_map(|saved| {
+            let spec = &saved.spec;
+            let database = match spec.driver {
+                // libpq's default database is the user's name.
+                Driver::Postgres if spec.database.is_empty() => spec.user.clone(),
+                Driver::Postgres | Driver::MySql => spec.database.clone(),
+                Driver::Sqlite => spec
+                    .sqlite_path
+                    .as_deref()
+                    .and_then(std::path::Path::file_name)
+                    .map(|name| name.to_string_lossy().into_owned())
+                    .unwrap_or_default(),
+                Driver::BigQuery | Driver::Files => return None,
+            };
+            // An SSH secret counts only where there is a tunnel.
+            let ssh_secret = match spec.ssh {
+                Some(_) => saved.ssh_secret,
+                None => PasswordMode::None,
+            };
+            let asks = [saved.password, ssh_secret]
+                .into_iter()
+                .any(|mode| mode == PasswordMode::Ask);
+            let no_database = match spec.driver {
+                Driver::MySql => spec.database.trim().is_empty(),
+                Driver::Postgres | Driver::Sqlite | Driver::BigQuery | Driver::Files => false,
+            };
+            let blocked = match (asks, no_database) {
+                (true, _) => Some(Blocked::AsksPassword),
+                (false, true) => Some(Blocked::NoDatabase),
+                (false, false) => None,
+            };
+            Some(Joinable {
+                id: saved.id.clone(),
+                name: saved.name.clone(),
+                environment: saved.environment,
+                driver: spec.driver,
+                database,
+                read_only: saved.read_only(),
+                blocked,
+            })
+        })
+        .collect()
+}
+
+/// The name SQL calls `saved` by when it is attached beside `attached`:
+/// its database's ([`tabletist_db::files::attach_name`]), with `_2`, `_3`
+/// and on after it where another attached connection, a schema of the
+/// connection's own (`taken`) or one of DuckDB's has it.
+pub fn default_name(saved: &SavedConnection, attached: &[Attach], taken: &[String]) -> String {
+    let wanted = tabletist_db::files::attach_name(&saved.spec);
+    let free = |name: &str| {
+        !tabletist_db::files::reserved_name(name)
+            && !attached.iter().any(|each| each.name == name)
+            && !taken.iter().any(|each| each == name)
+    };
+    if free(&wanted) {
+        return wanted;
+    }
+    (2..)
+        .map(|count| format!("{wanted}_{count}"))
+        .find(|name| free(name))
+        .unwrap_or(wanted)
+}
+
+/// What is wrong with the names `attached` are called by, among the
+/// schemas of the connection's own (`taken`): the first name that is not
+/// a plain one, is DuckDB's own, is taken, or is given twice.
+pub fn name_problem(attached: &[Attach], taken: &[String]) -> Option<String> {
+    for (at, each) in attached.iter().enumerate() {
+        let name = each.name.as_str();
+        if name.is_empty() {
+            return Some("Name each attached connection.".into());
+        }
+        if tabletist_db::files::table_name(name) != name {
+            return Some(format!(
+                "{name} can't name an attached connection: use lowercase letters, digits and _."
+            ));
+        }
+        if tabletist_db::files::reserved_name(name) || taken.iter().any(|other| other == name) {
+            return Some(format!(
+                "{name} is taken: name the attached connection another way."
+            ));
+        }
+        if attached[..at].iter().any(|other| other.name == name) {
+            return Some(format!("Two attached connections are named {name}."));
+        }
+    }
+    None
+}
 
 #[cfg(test)]
 mod tests {
diff --git a/crates/tabletist-core/src/model.rs b/crates/tabletist-core/src/model.rs
--- a/crates/tabletist-core/src/model.rs
+++ b/crates/tabletist-core/src/model.rs
@@ -116,6 +116,9 @@
     SetFilesKind(FilesKind),
     /// A change in a Files connection's Reading CSV section.
     SetCsv(CsvChange),
+    /// Attach the saved connection to the Files connection in the dialog,
+    /// under its database's name, or let it go where it is attached.
+    ToggleAttach(crate::connections::ConnectionId),
     /// One of the partial Test's fixes for the first file it could not
     /// read: saved as an override in the form, and the Test run again.
     FixFile(FileFix),
@@ -2067,6 +2070,9 @@
     pub focus_group: bool,
     /// What Group offers, read again on each frame the dialog draws.
     pub group_choices: crate::connections::groups::Choices,
+    /// Files: the saved connections the Join section lists, read again
+    /// on each frame the dialog draws.
+    pub joinable: Vec<crate::connections::join::Joinable>,
     /// The environment the user chose; `None` until they do, while it
     /// follows where the connection points (see
     /// [`ConnectionForm::environment`]).
@@ -2224,6 +2230,14 @@
     /// Files: whether the Reading CSV section is open. Closed each time
     /// the dialog opens.
     pub csv_open: bool,
+    /// Files: the saved connections it attaches, in the order they were
+    /// ticked, each with the name SQL calls it by.
+    pub files_attach: Vec<tabletist_db::files::Attach>,
+    /// Files: whether the Join with saved connections section is open.
+    /// Closed each time the dialog opens.
+    pub join_open: bool,
+    /// Files: the reader DuckDB is downloading for the running Test.
+    pub files_downloading: Option<tabletist_db::files::Reader>,
     /// Files: whether the Agents section is open, which a Files form
     /// keeps closed until it is opened or `show_agents` asks.
     pub agents_open: bool,
@@ -2414,6 +2428,7 @@
             group_typed: None,
             focus_group: false,
             group_choices: Default::default(),
+            joinable: Vec::new(),
             environment: None,
             read_only: None,
             production_read_only: true,
@@ -2486,6 +2501,9 @@
             files_check: None,
             files_csv: tabletist_db::files::Reading::default(),
             csv_open: false,
+            files_attach: Vec::new(),
+            join_open: false,
+            files_downloading: None,
             agents_open: false,
             retype_asked: false,
             agents_chosen: false,
@@ -2560,6 +2578,7 @@
         self.test_waiting = 0;
         self.files_tested = None;
         self.files_check = None;
+        self.files_downloading = None;
     }
 
     /// Files, terminal look: the path line names are typed into, as the
@@ -2747,6 +2766,7 @@
             group_typed: None,
             focus_group: false,
             group_choices: Default::default(),
+            joinable: Vec::new(),
             environment: Some(saved.environment),
             read_only: saved.read_only,
             production_read_only: true,
@@ -2855,10 +2875,27 @@
                 Some(Source::Folder(_)) | None => Vec::new(),
             },
             files_csv: spec.csv.clone(),
+            files_attach: spec.attach.clone(),
             // A saved connection's agents are what the file says.
             agents_chosen: true,
             ..Self::default()
         }
+    }
+
+    /// The schemas a Files connection has of its own, which no attached
+    /// connection can be named as: `main`, and each `.duckdb` file's, as
+    /// the Source was last read.
+    pub fn own_schemas(&self) -> Vec<String> {
+        let mut schemas = vec!["main".to_owned()];
+        if let FilesFound::Read(scan) = &self.files_found {
+            schemas.extend(
+                scan.files
+                    .iter()
+                    .filter(|file| file.format.is_database())
+                    .map(|file| file.name.clone()),
+            );
+        }
+        schemas
     }
 
     /// What a Files connection's Source names, as the backend reads it:
@@ -2951,10 +2988,19 @@
                 Ok(ConnectSpec::sqlite(path))
             }
             Driver::Files => match self.files_source() {
-                Some(source) => Ok(ConnectSpec {
-                    csv: self.files_csv.clone(),
-                    ..ConnectSpec::files(source)
-                }),
+                Some(source) => {
+                    let taken = self.own_schemas();
+                    if let Some(problem) =
+                        crate::connections::join::name_problem(&self.files_attach, &taken)
+                    {
+                        return Err(problem);
+                    }
+                    Ok(ConnectSpec {
+                        csv: self.files_csv.clone(),
+                        attach: self.files_attach.clone(),
+                        ..ConnectSpec::files(source)
+                    })
+                }
                 None => Err(match self.files_kind {
                     FilesKind::Folder => "Choose a folder.".into(),
                     FilesKind::Files => "Add a file.".into(),
diff --git a/src/app.rs b/src/app.rs
--- a/src/app.rs
+++ b/src/app.rs
@@ -1,5 +1,6 @@
 //! Application state and the reducer.
 
+use crate::backend::attach::AttachTo;
 use crate::backend::{Backend, SessionId};
 use crate::connections::SavedConnections;
 use std::collections::HashMap;
@@ -3349,6 +3350,13 @@
             Action::EditConnection(id) => {
                 if let Some(saved) = self.connections.get(&id) {
                     let mut form = ConnectionForm::from_saved(saved);
+                    // A connection deleted since it was attached, or no
+                    // longer one that can be (its type changed), is let
+                    // go: its row is no longer listed to untick.
+                    let joinable =
+                        crate::connections::join::joinable(&self.connections.connections);
+                    form.files_attach
+                        .retain(|each| joinable.iter().any(|row| row.id.0 == each.connection));
                     // What is in force, where a change made outside the
                     // app is held: what the person was shown.
                     form.agents.access = self.agents_access(saved);
@@ -3523,6 +3531,36 @@
                     self.read_form_source();
                 }
             }
+            Action::ToggleAttach(id) => {
+                let Some(Dialog::Connection(form)) = &mut self.dialog else {
+                    return;
+                };
+                if let Some(at) = form
+                    .files_attach
+                    .iter()
+                    .position(|each| each.connection == id.0)
+                {
+                    form.files_attach.remove(at);
+                } else if let Some(saved) = self.connections.get(&id) {
+                    let blocked = crate::connections::join::joinable(std::slice::from_ref(saved))
+                        .first()
+                        .is_none_or(|each| each.blocked.is_some());
+                    if !blocked {
+                        let taken = form.own_schemas();
+                        let name = crate::connections::join::default_name(
+                            saved,
+                            &form.files_attach,
+                            &taken,
+                        );
+                        form.files_attach.push(tabletist_db::files::Attach {
+                            connection: id.0.clone(),
+                            name,
+                        });
+                    }
+                }
+                // A Test of what it attached before says nothing of this.
+                form.forget_test();
+            }
             Action::SetFilesKind(kind) => {
                 if let Some(Dialog::Connection(form)) = &mut self.dialog {
                     form.files_kind = kind;
@@ -3639,6 +3677,7 @@
                 // The clock starts when the Test is sent, not while a saved
                 // secret is read from the keyring.
                 form.test_started = None;
+                form.files_downloading = None;
                 form.test_took = None;
                 // Typed secrets are used as they are; unchanged saved ones
                 // are read from the keyring first, and the test runs once
@@ -3686,11 +3725,13 @@
                 if loads.is_empty() {
                     form.test_started = Some(std::time::Instant::now());
                     hold_check(&mut self.files_check, &mut self.backend, &spec, request);
+                    let attach = AttachTo::of(&spec.attach, &self.connections);
                     self.backend.send(Command::Test {
                         request,
                         spec,
                         secrets,
                         host_keys: self.host_keys.clone(),
+                        attach,
                     });
                 } else {
                     for (kind, account) in loads {
@@ -4292,6 +4333,7 @@
                 *slot = None;
             }
         }
+        let attach = AttachTo::of(&spec.attach, &self.connections);
         self.backend.send(Command::Connect {
             session,
             request,
@@ -4299,6 +4341,7 @@
             secrets,
             host_keys: self.host_keys.clone(),
             access,
+            attach,
         });
     }
 
@@ -4892,10 +4935,18 @@
                     self.files_check = None;
                 }
             }
+            Event::FilesDownloading { request, reader } => {
+                if let Some(Dialog::Connection(form)) = &mut self.dialog
+                    && form.test == TestState::Running(request)
+                {
+                    form.files_downloading = Some(reader);
+                }
+            }
             Event::FilesTested { request, result } => {
                 if let Some(Dialog::Connection(form)) = &mut self.dialog
                     && form.test == TestState::Running(request)
                 {
+                    form.files_downloading = None;
                     let elapsed = form.test_started.take().map(|started| started.elapsed());
                     form.test_took = if result.is_ok() { elapsed } else { None };
                     form.test = match result {
@@ -5248,11 +5299,13 @@
                                         &spec,
                                         test,
                                     );
+                                    let attach = AttachTo::of(&spec.attach, &self.connections);
                                     self.backend.send(Command::Test {
                                         request: test,
                                         spec,
                                         secrets: form.test_secrets.clone(),
                                         host_keys: self.host_keys.clone(),
+                                        attach,
                                     });
                                 }
                             }
````

- [ ] **Step 4: Run them to see them pass**

Run: `~/.cargo/bin/cargo test -q -p tabletist-core --lib -- attach join`

Expected: the three `connections::join` tests, the two `backend::attach` tests and the two `[[connections.attach]]` tests pass (the filter takes a few existing ones too).

- [ ] **Step 5: Run the full check**

Run the five commands of the full check above.

Expected: all pass; 5,707 passed, 0 failed.

- [ ] **Step 6: Commit**

```bash
git add crates/tabletist-core src/app.rs
git commit -m "Keep the saved connections a Files connection attaches, and read their passwords as it opens"
```

### Task 3: The Join with saved connections section

**Files:**
- Create: `src/ui/attach_list.rs`, `src/ui/connect_dialog/join.rs`, `src/ui/join_tests.rs`
- Modify: `src/ui/connect_dialog/mod.rs`, `sheet.rs`, `terminal.rs`, `src/ui/mod.rs`, `tests/environments.rs` (the attach rows colour by environment), `assets/i18n/*`, `docs/_reference/connections.md`, `docs/_reference/settings-and-files.md`, `docs/_guide/omarchy.md`

- [ ] **Step 1: Write the failing tests**

The helpers save the Bookshop's databases and open a new Files connection's dialog with the section open:
- development, staging (whose password is asked for on every connect) and production on PostgreSQL;
- the SQLite file;
- a Files connection, which isn't listed.

````diff
diff --git a/src/ui/join_tests.rs b/src/ui/join_tests.rs
new file mode 100644
--- /dev/null
+++ b/src/ui/join_tests.rs
@@ -0,0 +1,472 @@
+//! A Files connection's Join with saved connections section, in both
+//! looks (the "Connection, Files, details" boards): the saved
+//! PostgreSQL, MySQL and SQLite connections it lists, a row ticked under
+//! its database's name, a name typed over and checked on Save, a row that
+//! cannot be ticked, the Omarchy keys, and what a Test and an open send:
+//! the saved connections to attach, never a password.
+
+use egui::{Key, Modifiers};
+use tabletist_db::files::{Attach, Reader, Source};
+use tabletist_db::{ConnectSpec, Driver, Error};
+
+use crate::backend::attach::{AttachSaved, AttachTo};
+use crate::backend::{Command, Event};
+use crate::connections::{ConnectionId, PasswordMode, SavedConnection};
+use crate::env::Environment;
+use crate::model::{Action, ConnectionForm, Dialog, TestState};
+use crate::testing::Harness;
+use crate::theme::Look;
+
+fn looks() -> [Look; 2] {
+    [Look::macos(), Look::omarchy()]
+}
+
+fn form(harness: &Harness) -> &ConnectionForm {
+    match &harness.app.dialog {
+        Some(Dialog::Connection(form)) => form,
+        _ => panic!("no connection dialog"),
+    }
+}
+
+fn form_mut(harness: &mut Harness) -> &mut ConnectionForm {
+    match &mut harness.app.dialog {
+        Some(Dialog::Connection(form)) => form,
+        _ => panic!("no connection dialog"),
+    }
+}
+
+/// Whether a node says `text`, alone or as part of what it says, in any
+/// case: the terminal look says its words in lower case.
+fn says(harness: &mut Harness, text: &str) -> bool {
+    let tree = harness.settle();
+    let text = text.to_lowercase();
+    crate::testing::labels(&tree)
+        .iter()
+        .any(|label| label.to_lowercase().contains(&text))
+}
+
+/// The Bookshop's saved databases: development, staging (whose password
+/// is asked for on every connect) and production on PostgreSQL, and the
+/// SQLite file; and a Files connection, which is not listed.
+fn bookshop(harness: &mut Harness) -> [ConnectionId; 4] {
+    let mut save = |name: &str, environment, password, spec: ConnectSpec| {
+        let saved = SavedConnection {
+            id: ConnectionId::new(),
+            name: name.into(),
+            group: None,
+            environment,
+            read_only: None,
+            password,
+            ssh_secret: PasswordMode::None,
+            spec,
+            agents: Default::default(),
+        };
+        let id = saved.id.clone();
+        harness.app.connections.upsert(saved);
+        id
+    };
+    let postgres = |database: &str| ConnectSpec {
+        driver: Driver::Postgres,
+        host: "db.bookshop.test".into(),
+        user: "reader".into(),
+        database: database.into(),
+        ..ConnectSpec::default()
+    };
+    let dev = save(
+        "Bookshop",
+        Environment::Dev,
+        PasswordMode::Keyring,
+        postgres("bookshop_development"),
+    );
+    let staging = save(
+        "Bookshop",
+        Environment::Staging,
+        PasswordMode::Ask,
+        postgres("bookshop_staging"),
+    );
+    let production = save(
+        "Bookshop",
+        Environment::Production,
+        PasswordMode::Keyring,
+        postgres("bookshop_production"),
+    );
+    let file = save(
+        "Bookshop",
+        Environment::Local,
+        PasswordMode::None,
+        ConnectSpec::sqlite("/data/bookshop/development.sqlite3"),
+    );
+    save(
+        "Bookshop exports",
+        Environment::Local,
+        PasswordMode::None,
+        ConnectSpec::files(Source::Folder("/data/bookshop-exports".into())),
+    );
+    [dev, staging, production, file]
+}
+
+/// A new Files connection's dialog in `look`, over the Bookshop's
+/// exports, the Bookshop's databases saved, its Join section open.
+fn joining(look: Look) -> (Harness, [ConnectionId; 4]) {
+    let mut harness = Harness::with_size(egui::vec2(1200.0, 1400.0));
+    harness.set_look(look);
+    let ids = bookshop(&mut harness);
+    harness.app.apply(Action::NewConnection);
+    harness.settle();
+    harness.click(&look.label("Files"));
+    form_mut(&mut harness).files_folder = "/data/bookshop-exports".into();
+    form_mut(&mut harness).join_open = true;
+    harness.settle();
+    (harness, ids)
+}
+
+fn attached(harness: &Harness) -> Vec<(String, String)> {
+    form(harness)
+        .files_attach
+        .iter()
+        .map(|each| (each.connection.clone(), each.name.clone()))
+        .collect()
+}
+
+#[test]
+fn the_section_is_closed_at_first_and_lists_the_saved_databases() {
+    for look in looks() {
+        let mut harness = Harness::with_size(egui::vec2(1200.0, 1400.0));
+        harness.set_look(look);
+        bookshop(&mut harness);
+        harness.app.apply(Action::NewConnection);
+        harness.settle();
+        harness.click(&look.label("Files"));
+        assert!(!form(&harness).join_open, "{}", look.name);
+        assert!(
+            says(&mut harness, "Join with saved connections, None attached"),
+            "{}",
+            look.name
+        );
+        assert!(!harness.has("Attach Bookshop dev"));
+        form_mut(&mut harness).join_open = true;
+        for row in [
+            "Attach Bookshop dev",
+            "Attach Bookshop staging",
+            "Attach Bookshop production",
+            "Attach Bookshop local",
+        ] {
+            assert!(harness.has(row), "{}: {row}", look.name);
+        }
+        // A Files connection is not one to attach.
+        assert!(
+            !harness.has("Attach Bookshop exports local"),
+            "{}",
+            look.name
+        );
+        assert!(
+            says(
+                &mut harness,
+                &look.label("Asks for its password on every connect")
+            ),
+            "{}",
+            look.name
+        );
+        assert!(
+            says(
+                &mut harness,
+                &look.label("Attached connections open read-only here, as the files do")
+            ),
+            "{}",
+            look.name
+        );
+    }
+}
+
+#[test]
+fn a_row_ticked_is_attached_under_its_database_s_name() {
+    for look in looks() {
+        let (mut harness, [dev, _, production, file]) = joining(look);
+        harness.click("Attach Bookshop dev");
+        harness.click("Attach Bookshop local");
+        harness.click("Attach Bookshop production");
+        assert_eq!(
+            attached(&harness),
+            [
+                (dev.0.clone(), "bookshop_development".to_owned()),
+                (file.0.clone(), "development".to_owned()),
+                (production.0.clone(), "bookshop_production".to_owned()),
+            ],
+            "{}",
+            look.name
+        );
+        // Closed, the section says them.
+        form_mut(&mut harness).join_open = false;
+        assert!(
+            says(
+                &mut harness,
+                "Attached as bookshop_development, development, bookshop_production"
+            ),
+            "{}",
+            look.name
+        );
+        form_mut(&mut harness).join_open = true;
+        // The sample query, for the first, and the hint above the rows.
+        assert!(
+            says(
+                &mut harness,
+                "JOIN bookshop_development.publishers p ON p.id = r.publisher_id"
+            ),
+            "{}",
+            look.name
+        );
+        assert!(
+            says(&mut harness, "join a file to its live tables in one query"),
+            "{}",
+            look.name
+        );
+        // Ticked again, it is let go.
+        harness.click("Attach Bookshop local");
+        assert_eq!(attached(&harness).len(), 2, "{}", look.name);
+    }
+}
+
+#[test]
+fn a_row_that_asks_for_its_password_cannot_be_ticked() {
+    for look in looks() {
+        let (mut harness, [_, staging, _, _]) = joining(look);
+        harness.click("Attach Bookshop staging");
+        assert!(attached(&harness).is_empty(), "{}", look.name);
+        // Nor by its action, from elsewhere.
+        harness.app.apply(Action::ToggleAttach(staging));
+        assert!(attached(&harness).is_empty(), "{}", look.name);
+    }
+}
+
+#[test]
+fn a_row_ticked_before_it_could_not_be_still_unticks() {
+    for look in looks() {
+        let (mut harness, [dev, ..]) = joining(look);
+        harness.click("Attach Bookshop dev");
+        // Its password is asked for on every connect from now on.
+        let mut saved = harness.app.connections.get(&dev).unwrap().clone();
+        saved.password = PasswordMode::Ask;
+        harness.app.connections.upsert(saved);
+        harness.settle();
+        assert!(
+            says(&mut harness, "Asks for its password on every connect"),
+            "{}",
+            look.name
+        );
+        harness.click("Attach Bookshop dev");
+        assert!(attached(&harness).is_empty(), "{}", look.name);
+    }
+}
+
+#[test]
+fn a_test_s_download_goes_with_it() {
+    let (mut harness, _) = joining(Look::macos());
+    harness.click("Attach Bookshop dev");
+    harness.app.apply(Action::TestConnection);
+    let TestState::Running(request) = form(&harness).test else {
+        panic!("no test");
+    };
+    harness.app.apply(Action::Backend(Event::FilesDownloading {
+        request,
+        reader: Reader::Postgres,
+    }));
+    // A row ticked meanwhile: that Test, and its download, say nothing
+    // of what is attached now.
+    harness.click("Attach Bookshop production");
+    assert_eq!(form(&harness).files_downloading, None);
+    assert!(!says(&mut harness, "Downloading the PostgreSQL reader"));
+    harness.app.apply(Action::TestConnection);
+    assert!(says(&mut harness, "Testing…"));
+    // A name typed over forgets a Test that passed.
+    form_mut(&mut harness).test = TestState::Passed;
+    let field = crate::ui::attach_list::name_id(0);
+    harness.ctx.memory_mut(|memory| memory.request_focus(field));
+    harness.settle();
+    harness.frame(vec![
+        crate::testing::key(Key::A, Modifiers::COMMAND),
+        egui::Event::Text("dev".into()),
+    ]);
+    harness.settle();
+    assert_eq!(form(&harness).files_attach[0].name, "dev");
+    assert_eq!(form(&harness).test, TestState::Idle);
+}
+
+#[test]
+fn a_name_is_typed_over_and_checked_on_save() {
+    let (mut harness, [dev, ..]) = joining(Look::macos());
+    harness.click("Attach Bookshop dev");
+    form_mut(&mut harness).name = "Bookshop joined".into();
+    form_mut(&mut harness).files_attach[0].name = "Bookshop-dev".into();
+    harness.app.apply(Action::SaveConnection { connect: false });
+    assert_eq!(
+        form(&harness).message.as_deref(),
+        Some(
+            "Bookshop-dev can't name an attached connection: use lowercase letters, digits and _."
+        )
+    );
+    form_mut(&mut harness).files_attach[0].name = "bookshop_dev".into();
+    harness.app.apply(Action::SaveConnection { connect: false });
+    assert!(
+        harness.app.dialog.is_none(),
+        "{:?}",
+        harness.app.dialog.is_some()
+    );
+    let saved = harness
+        .app
+        .connections
+        .connections
+        .iter()
+        .find(|saved| saved.name == "Bookshop joined")
+        .cloned()
+        .unwrap();
+    assert_eq!(
+        saved.spec.attach,
+        [Attach {
+            connection: dev.0.clone(),
+            name: "bookshop_dev".into(),
+        }]
+    );
+}
+
+#[test]
+fn a_test_sends_what_is_attached_and_says_the_download() {
+    for look in looks() {
+        let (mut harness, [dev, ..]) = joining(look);
+        harness.click("Attach Bookshop dev");
+        harness.app.apply(Action::TestConnection);
+        let TestState::Running(request) = form(&harness).test else {
+            panic!("no test: {:?}", form(&harness).test);
+        };
+        let saved = harness.app.connections.get(&dev).unwrap().clone();
+        let Some(Command::Test {
+            attach, secrets, ..
+        }) = harness.app.backend.sent.last()
+        else {
+            panic!("no Test sent");
+        };
+        assert_eq!(
+            attach,
+            &[AttachTo {
+                name: "bookshop_development".into(),
+                saved: Some(AttachSaved {
+                    id: dev.clone(),
+                    spec: saved.spec.clone(),
+                    password: PasswordMode::Keyring,
+                    ssh_secret: PasswordMode::None,
+                }),
+            }]
+        );
+        // No password goes from the app: the backend reads it.
+        assert_eq!(secrets.password, None);
+        harness.app.apply(Action::Backend(Event::FilesDownloading {
+            request,
+            reader: Reader::Postgres,
+        }));
+        assert!(
+            says(
+                &mut harness,
+                &look.label("Downloading the PostgreSQL reader for DuckDB…")
+            ),
+            "{}",
+            look.name
+        );
+        let words = "Can't attach bookshop_development: password authentication failed";
+        harness.app.apply(Action::Backend(Event::FilesTested {
+            request,
+            result: Err(Error::Connect(words.into())),
+        }));
+        assert!(says(&mut harness, &look.label(words)), "{}", look.name);
+        assert_eq!(form(&harness).files_downloading, None);
+    }
+}
+
+#[test]
+fn an_open_sends_what_is_attached() {
+    let mut harness = Harness::new();
+    let [dev, _, production, _] = bookshop(&mut harness);
+    // Production is a BigQuery connection now, which can't be attached.
+    let mut bigquery = harness.app.connections.get(&production).unwrap().clone();
+    bigquery.spec = ConnectSpec::bigquery("bookshop-analytics", None);
+    harness.app.connections.upsert(bigquery);
+    let mut spec = ConnectSpec::files(Source::Folder("/data/bookshop-exports".into()));
+    spec.attach = vec![
+        Attach {
+            connection: dev.0.clone(),
+            name: "bookshop_dev".into(),
+        },
+        Attach {
+            connection: "deleted".into(),
+            name: "gone".into(),
+        },
+        Attach {
+            connection: production.0.clone(),
+            name: "now_bigquery".into(),
+        },
+    ];
+    let saved = SavedConnection {
+        id: ConnectionId::new(),
+        name: "Bookshop exports".into(),
+        group: None,
+        environment: Environment::Local,
+        read_only: Some(true),
+        password: PasswordMode::None,
+        ssh_secret: PasswordMode::None,
+        spec,
+        agents: Default::default(),
+    };
+    let id = saved.id.clone();
+    harness.app.connections.upsert(saved);
+    let tab = harness.app.active_tab_id();
+    harness.app.apply(Action::Connect {
+        tab,
+        conn: id.clone(),
+    });
+    let Command::Connect { attach, .. } = crate::testing::last_sent(&harness.app) else {
+        panic!("expected Connect");
+    };
+    let names: Vec<(&str, bool)> = attach
+        .iter()
+        .map(|each| (each.name.as_str(), each.saved.is_some()))
+        .collect();
+    // One deleted since goes too: the backend says it cannot attach it.
+    assert_eq!(
+        names,
+        [
+            ("bookshop_dev", true),
+            ("gone", false),
+            ("now_bigquery", true)
+        ]
+    );
+    // Edited, the deleted one and the BigQuery one are let go: their rows
+    // are not listed to untick.
+    harness.app.apply(Action::EditConnection(id));
+    assert_eq!(
+        attached(&harness),
+        [(dev.0.clone(), "bookshop_dev".to_owned())]
+    );
+}
+
+#[test]
+fn omarchy_space_ticks_a_row_and_the_arrows_move_in_the_list() {
+    let (mut harness, [dev, ..]) = joining(Look::omarchy());
+    let first = crate::ui::attach_list::toggle_id(0);
+    harness.ctx.memory_mut(|memory| memory.request_focus(first));
+    harness.settle();
+    harness.press(Key::Space, Modifiers::NONE);
+    assert_eq!(
+        attached(&harness),
+        [(dev.0.clone(), "bookshop_development".to_owned())]
+    );
+    let cursor = form(&harness).files_cursor;
+    harness.press(Key::ArrowDown, Modifiers::NONE);
+    harness.settle();
+    assert!(
+        harness
+            .ctx
+            .memory(|memory| memory.has_focus(crate::ui::attach_list::toggle_id(1)))
+    );
+    assert_eq!(form(&harness).files_cursor, cursor);
+    harness.press(Key::ArrowUp, Modifiers::NONE);
+    harness.settle();
+    assert!(harness.ctx.memory(|memory| memory.has_focus(first)));
+}
diff --git a/src/ui/mod.rs b/src/ui/mod.rs
--- a/src/ui/mod.rs
+++ b/src/ui/mod.rs
@@ -76,6 +76,8 @@
 pub mod file_list;
 #[cfg(test)]
 mod files_tests;
+#[cfg(test)]
+mod join_tests;
 pub mod filter_bar;
 pub mod focus;
 #[cfg(test)]
````

- [ ] **Step 2: Run them to see them fail**

Run: `~/.cargo/bin/cargo test -q -p tabletist --lib join_tests`

Expected: they don't compile, since `attach_list` isn't there yet.

```text
error[E0433]: cannot find `attach_list` in `ui`
    |
error: could not compile `tabletist` (lib test) due to 3 previous errors
```

- [ ] **Step 3: The section, its rows and keys, and the docs**

A blocked row's box takes the keyboard (`Sense::focusable_noninteractive`), so the arrows pass through it. While a box has the keyboard, a focus lock filter keeps Up and Down from egui, which would move to the next stop. The form moves them to the next row's box instead.

````diff
diff --git a/docs/_guide/omarchy.md b/docs/_guide/omarchy.md
--- a/docs/_guide/omarchy.md
+++ b/docs/_guide/omarchy.md
@@ -100,8 +100,8 @@
 | Ctrl+S | Save |
 | Ctrl+Enter | Save and connect |
 | Backspace, Delete | In the empty password field: forget the saved password |
-| Up, Down | In add table…'s or add column…'s list: move. Enter adds the one chosen, or in add column…'s puts a table chosen in the field for its columns, and Esc closes the field. In a Files connection's list of files: move |
-| Enter, Space | Choose the choice the cursor is on: a type, an environment, folder or files. On a closed section's heading (a Files connection's reading csv or agents): open it, and on an open one, close it. On group: open its list of groups, where Up and Down move and Enter picks; new group… turns it into a field for a new group's name |
+| Up, Down | In add table…'s or add column…'s list: move. Enter adds the one chosen, or in add column…'s puts a table chosen in the field for its columns, and Esc closes the field. In a Files connection's list of files: move. On a row of its join with saved connections: move to the row above or below |
+| Enter, Space | Choose the choice the cursor is on: a type, an environment, folder or files. On a closed section's heading (a Files connection's reading csv, join with saved connections or agents): open it, and on an open one, close it. On a row of join with saved connections: attach that connection, or let it go. On group: open its list of groups, where Up and Down move and Enter picks; new group… turns it into a field for a new group's name |
 | Ctrl+O | A Files connection: pick its folder, or add files to its list, in the file chooser |
 | Ctrl+N, Ctrl+P | A Files connection's folder, or the add line under its list of files, while the names a typed path starts are listed under it: move (Down and Up too). Enter takes one: a folder lists its own names, a file is added to the list. Esc closes the list. In the add line with nothing listed, Enter adds the path as typed, if it names a file from `/` or `~` |
 | `a` | A Files connection's list: add files |
diff --git a/docs/_reference/connections.md b/docs/_reference/connections.md
--- a/docs/_reference/connections.md
+++ b/docs/_reference/connections.md
@@ -129,6 +129,28 @@
   A connection that is open keeps reading its files as it did when it
   opened, Reconnect included: close it and open it again from the
   connections list to read them with the new settings.
+- **Join with saved connections**, closed until you open it, lists your
+  saved PostgreSQL, MySQL and SQLite connections. Tick one to attach it:
+  its tables are then in the Files connection too, under a name of its
+  own, so one query can join a file to them (`SELECT … FROM returns r
+  JOIN bookshop_development.publishers p ON …`). The name is its
+  database's (a SQLite file's name without its extension) and you can
+  type over it: lowercase letters, digits and `_`, and not a schema the
+  connection has already. In the sidebar each attached connection is a
+  schema, listing the tables of its default schema (PostgreSQL's
+  `public`, MySQL's database, SQLite's own); its other schemas are
+  reached in SQL, as `bookshop_development.billing.invoices`. An attached
+  connection is always read-only. Its password comes from the keyring as
+  the Files connection opens, and its SSH tunnel, if it has one, is
+  opened then too; a connection whose password is asked for on every
+  connect, or a MySQL one that names no database, cannot be attached. If
+  one cannot be attached when the Files connection opens or is tested,
+  that says which and why. In the Omarchy look each row is a Tab stop,
+  Space ticks it and Up and Down move between the rows.
+- The first time a Files connection needs DuckDB's reader for Excel,
+  PostgreSQL, MySQL or SQLite, DuckDB downloads it, and Test says so
+  while it does ("Downloading the PostgreSQL reader for DuckDB…"). Once
+  downloaded, it is used offline.
 - A new Files connection is **local**, and lets agents read. Files open
   read-only: nothing writes to them, and a statement that would read
   another file on the computer, write a file or install anything is
diff --git a/docs/_reference/settings-and-files.md b/docs/_reference/settings-and-files.md
--- a/docs/_reference/settings-and-files.md
+++ b/docs/_reference/settings-and-files.md
@@ -585,6 +585,25 @@
 connection does not open, does nothing. A file that holds either table
 says `version = 8`.
 
+The saved connections a Files connection attaches are an
+`[[connections.attach]]` table each, in the order they were ticked:
+
+```toml
+[[connections.attach]]
+connection = "6f1c2a9e-…"
+as         = "bookshop_dev"
+```
+
+| Key | Values |
+| --- | --- |
+| `connection` | The `id` of a saved PostgreSQL, MySQL or SQLite connection: renaming it changes nothing here |
+| `as` | The name SQL calls it by: lowercase letters, digits and `_`, once in the connection, and none of DuckDB's own (`main`, `memory`, `system`, `temp`, `information_schema`, `pg_catalog`) |
+
+No password is kept here: each comes from the keyring when the Files
+connection opens. A connection the file attaches that was deleted since
+is said when the Files connection opens, and let go the next time it is
+edited. A file that holds one says `version = 10`.
+
 A tunnel is the connection's `[connections.ssh]` table: `host`, and
 optionally `port` and `user`, `auth` (`password`, `key-file` or `agent`),
 and with `key-file` the `key` to use.
diff --git a/src/ui/attach_list.rs b/src/ui/attach_list.rs
new file mode 100644
--- /dev/null
+++ b/src/ui/attach_list.rs
@@ -0,0 +1,394 @@
+//! The list of saved connections a Files connection can attach, a row
+//! each: a box to tick, the connection's environment, its name and
+//! database, and, once ticked, the name SQL calls it by, which can be
+//! typed over. A row that cannot be ticked says why. macOS draws each row
+//! as the boards' attach row (the environment's stripe and badge, the
+//! engine and database under the name); the terminal look as one line,
+//! `[x] DEV Bookshop as bookshop_dev`.
+
+use egui::{CornerRadius, Rect, Sense, Ui, pos2, vec2};
+
+use crate::env::{Environment, Platform, env_colors};
+use crate::i18n::{Locale, gettext};
+use crate::theme::{Look, Palette};
+use crate::typography::{Text, TextRole};
+use crate::ui::workspace::{Badge, env_badge};
+use crate::ui::{focus, widgets};
+
+/// One saved connection, as its row says it.
+pub struct Row<'a> {
+    pub name: &'a str,
+    pub environment: Environment,
+    /// Its engine's name: "PostgreSQL".
+    pub engine: &'a str,
+    pub database: &'a str,
+    /// It opens read-only, so it is attached so.
+    pub read_only: bool,
+    /// Why it cannot be ticked, in the look's words.
+    pub blocked: Option<String>,
+    /// The name SQL calls it by, where it is ticked.
+    pub attached: Option<&'a mut String>,
+}
+
+/// The id of the box of the row at `at`: the keyboard's stop on it.
+pub fn toggle_id(at: usize) -> egui::Id {
+    egui::Id::new(("attach-toggle", at))
+}
+
+/// The id of the name field of the row at `at`.
+pub fn name_id(at: usize) -> egui::Id {
+    egui::Id::new(("attach-name", at))
+}
+
+/// The row whose box has the keyboard, among `rows` rows.
+pub fn focused(ctx: &egui::Context, rows: usize) -> Option<usize> {
+    let focused = ctx.memory(|memory| memory.focused())?;
+    (0..rows).find(|at| toggle_id(*at) == focused)
+}
+
+/// The rows, under one another; the row whose box was ticked or unticked
+/// this frame, if one was. Announced as a list named `name`.
+pub fn show(
+    ui: &mut Ui,
+    name: &str,
+    rows: &mut [Row<'_>],
+    (look, palette, locale): (&Look, &Palette, Locale),
+) -> Option<usize> {
+    let mut toggled = None;
+    ui.scope_builder(egui::UiBuilder::new().id_salt(name), |ui| {
+        ui.ctx().accesskit_node_builder(ui.unique_id(), |node| {
+            node.set_role(egui::accesskit::Role::List);
+            node.set_label(name);
+        });
+        ui.spacing_mut().item_spacing.y = 0.0;
+        let framed = !look.terminal;
+        let frame = egui::Frame::new()
+            .stroke(match framed {
+                true => egui::Stroke::new(1.0, palette.border),
+                false => egui::Stroke::NONE,
+            })
+            .corner_radius(CornerRadius::same(if framed { 8 } else { 0 }));
+        frame.show(ui, |ui| {
+            for (at, row) in rows.iter_mut().enumerate() {
+                if at > 0 && framed {
+                    let y = ui.cursor().top();
+                    widgets::hline(ui, ui.max_rect().x_range(), y, palette.surface_hover);
+                }
+                let pressed = match look.terminal {
+                    true => line(ui, at, row, look, palette, locale),
+                    false => sheet_row(ui, at, row, look, palette, locale),
+                };
+                if pressed {
+                    toggled = Some(at);
+                }
+            }
+        });
+    });
+    toggled
+}
+
+/// The words "Attach Bookshop dev": what the box is named for a screen
+/// reader, the environment after the name, as the boards write it.
+fn named(row: &Row<'_>, locale: Locale) -> String {
+    format!(
+        "{} {} {}",
+        gettext(locale, "Attach"),
+        row.name,
+        row.environment.label(Platform::Native)
+    )
+}
+
+/// The box, as a stop of its own: whether a press of it (the pointer,
+/// Space or Enter) ticked or unticked it this frame.
+fn toggle(
+    ui: &mut Ui,
+    at: usize,
+    rect: Rect,
+    row: &Row<'_>,
+    locale: Locale,
+) -> (egui::Response, bool) {
+    // One that cannot be ticked still takes the keyboard, so the arrows
+    // pass through it and a screen reader says why. One ticked before it
+    // could not be still unticks.
+    let checked = row.attached.is_some();
+    let enabled = row.blocked.is_none() || checked;
+    let response = ui.interact(
+        rect,
+        toggle_id(at),
+        if enabled {
+            Sense::click()
+        } else {
+            Sense::focusable_noninteractive()
+        },
+    );
+    // The arrows are the list's while a box has the keyboard (the form
+    // moves it to the next row's), not egui's to move it to the next stop.
+    if response.has_focus() {
+        let filter = egui::EventFilter {
+            vertical_arrows: true,
+            ..Default::default()
+        };
+        ui.ctx()
+            .memory_mut(|memory| memory.set_focus_lock_filter(response.id, filter));
+    }
+    let label = named(row, locale);
+    response.widget_info(|| {
+        egui::WidgetInfo::selected(egui::WidgetType::Checkbox, enabled, checked, &label)
+    });
+    let pressed = enabled && crate::ui::disclosure::pressed(ui, &response);
+    (response, pressed)
+}
+
+/// macOS: the board's attach row, 40 tall.
+fn sheet_row(
+    ui: &mut Ui,
+    at: usize,
+    row: &mut Row<'_>,
+    look: &Look,
+    palette: &Palette,
+    locale: Locale,
+) -> bool {
+    let width = ui.available_width();
+    let (rect, _) = ui.allocate_exact_size(vec2(width, 40.0), Sense::hover());
+    let center = rect.center().y;
+    let colors = env_colors(row.environment, Platform::Native, palette);
+    let enabled = row.blocked.is_none();
+    let faded = |color: egui::Color32| match enabled {
+        true => color,
+        false => color.gamma_multiply(0.5),
+    };
+    // The box.
+    let box_rect = Rect::from_center_size(pos2(rect.left() + 17.0, center), vec2(14.0, 14.0));
+    let (response, pressed) = toggle(ui, at, box_rect.expand(4.0), row, locale);
+    let checked = row.attached.is_some();
+    let painter = ui.painter().clone();
+    painter.rect(
+        box_rect,
+        CornerRadius::same(look.radius.min(4)),
+        match checked {
+            true => faded(palette.accent),
+            false => palette.panel,
+        },
+        egui::Stroke::new(1.0, faded(palette.border)),
+        egui::StrokeKind::Inside,
+    );
+    if checked {
+        let stroke = egui::Stroke::new(1.6, widgets::ink_on(palette.accent, palette));
+        let (l, c) = (box_rect.left(), box_rect.center());
+        painter.line_segment([pos2(l + 3.0, c.y), pos2(c.x - 1.0, c.y + 3.5)], stroke);
+        painter.line_segment(
+            [
+                pos2(c.x - 1.0, c.y + 3.5),
+                pos2(box_rect.right() - 3.0, c.y - 3.5),
+            ],
+            stroke,
+        );
+    }
+    focus::hint(ui, &response, box_rect, focus::Ring::Outer { radius: 4 });
+    // The environment's stripe.
+    let stripe = Rect::from_center_size(pos2(rect.left() + 37.0, center), vec2(6.0, 24.0));
+    painter.rect_filled(stripe, CornerRadius::same(3), faded(colors.base()));
+    // The name, the badge and read-only; the engine and database under.
+    let left = stripe.right() + 10.0;
+    let top = center - 8.0;
+    let mut x = left;
+    x += widgets::paint_label(
+        ui,
+        x,
+        top,
+        Text::one(
+            look,
+            TextRole::UiBodySemibold,
+            row.name,
+            faded(palette.text),
+        ),
+    );
+    x += 8.0;
+    x += env_badge(ui, x, top, row.environment, &colors, Badge::Mac, look) + 8.0;
+    if row.read_only {
+        widgets::paint_label(
+            ui,
+            x,
+            top,
+            Text::one(
+                look,
+                TextRole::Secondary,
+                &gettext(locale, "read-only"),
+                palette.dim,
+            ),
+        );
+    }
+    let under = format!("{} · {}", row.engine, row.database);
+    widgets::paint_label(
+        ui,
+        left,
+        center + 9.0,
+        Text::one(look, TextRole::Secondary, &under, faded(palette.dim)),
+    );
+    // On the right: the name SQL calls it by, or why it cannot be ticked.
+    let right = Rect::from_min_max(pos2(rect.right() - 210.0, rect.top()), rect.max)
+        .shrink2(vec2(10.0, 6.0));
+    match (&mut row.attached, &row.blocked) {
+        (Some(sql), _) => {
+            name_field(ui, at, right, sql, look, locale, TextRole::Code);
+        }
+        (None, Some(why)) => right_label(
+            ui,
+            right.right(),
+            center,
+            why,
+            TextRole::Secondary,
+            look,
+            palette,
+        ),
+        (None, None) => {
+            let words = gettext(locale, "not attached");
+            right_label(
+                ui,
+                right.right(),
+                center,
+                &words,
+                TextRole::MonoSecondary,
+                look,
+                palette,
+            );
+        }
+    }
+    pressed
+}
+
+/// `text` ending at `right`, centred on `y`, in the faint colour: said to
+/// screen readers too.
+fn right_label(
+    ui: &Ui,
+    right: f32,
+    y: f32,
+    text: &str,
+    role: TextRole,
+    look: &Look,
+    palette: &Palette,
+) {
+    let width = role.width(ui.ctx(), look.faces, text);
+    widgets::paint_label(
+        ui,
+        right - width,
+        y,
+        Text::one(look, role, text, palette.faint),
+    );
+}
+
+/// The terminal look: `[x] DEV Bookshop as bookshop_dev · read-only`.
+fn line(
+    ui: &mut Ui,
+    at: usize,
+    row: &mut Row<'_>,
+    look: &Look,
+    palette: &Palette,
+    locale: Locale,
+) -> bool {
+    let role = TextRole::OBody;
+    let height = 28.0;
+    let width = ui.available_width();
+    let (rect, _) = ui.allocate_exact_size(vec2(width, height), Sense::hover());
+    let center = rect.center().y;
+    let colors = env_colors(row.environment, Platform::Omarchy, palette);
+    let checked = row.attached.is_some();
+    let enabled = row.blocked.is_none();
+    let mark = if checked { "[x]" } else { "[ ]" };
+    let mark_width = role.width(ui.ctx(), look.faces, mark);
+    let mark_rect = Rect::from_min_size(
+        pos2(rect.left(), center - height / 2.0),
+        vec2(mark_width + 4.0, height),
+    );
+    let (response, pressed) = toggle(ui, at, mark_rect, row, locale);
+    let painter = ui.painter().clone();
+    if focus::shown(&response) {
+        painter.rect_filled(mark_rect, CornerRadius::ZERO, palette.selection);
+    }
+    let mut x = rect.left() + 2.0;
+    x += widgets::paint_label(
+        ui,
+        x,
+        center,
+        Text::one(
+            look,
+            role,
+            mark,
+            match (checked, enabled) {
+                (true, _) => palette.success,
+                (false, true) => palette.dim,
+                (false, false) => palette.faint,
+            },
+        ),
+    ) + 10.0;
+    x += env_badge(ui, x, center, row.environment, &colors, Badge::Plain, look) + 10.0;
+    x += widgets::paint_label(
+        ui,
+        x,
+        center,
+        Text::one(
+            look,
+            role,
+            row.name,
+            if enabled { palette.text } else { palette.faint },
+        ),
+    ) + 8.0;
+    // Its database, dim: `· bookshop_development`.
+    if !row.database.is_empty() {
+        let database = format!("· {}", row.database);
+        x += widgets::paint_label(ui, x, center, Text::one(look, role, &database, palette.dim))
+            + 10.0;
+    }
+    focus::hint(ui, &response, mark_rect, focus::Ring::Bar);
+    match (&mut row.attached, &row.blocked) {
+        (Some(sql), _) => {
+            let words = look.label(&gettext(locale, "as"));
+            x += widgets::paint_label(ui, x, center, Text::one(look, role, &words, palette.dim));
+            x += 6.0;
+            let field = Rect::from_min_max(
+                pos2(x, rect.top() + 2.0),
+                pos2((x + 220.0).min(rect.right()), rect.bottom() - 2.0),
+            );
+            name_field(ui, at, field, sql, look, locale, role);
+            if row.read_only {
+                widgets::paint_label(
+                    ui,
+                    field.right() + 8.0,
+                    center,
+                    Text::one(
+                        look,
+                        role,
+                        &look.label(&format!("· {}", gettext(locale, "read-only"))),
+                        palette.dim,
+                    ),
+                );
+            }
+        }
+        (None, Some(why)) => {
+            widgets::paint_label(ui, x, center, Text::one(look, role, why, palette.faint));
+        }
+        (None, None) => {}
+    }
+    pressed
+}
+
+/// The field the name SQL calls a ticked connection by is typed in, in
+/// `rect`.
+fn name_field(
+    ui: &mut Ui,
+    at: usize,
+    rect: Rect,
+    sql: &mut String,
+    look: &Look,
+    locale: Locale,
+    role: TextRole,
+) {
+    let named = gettext(locale, "SQL name");
+    ui.put(
+        rect,
+        widgets::field(ui, sql, look, role, rect.height(), 6)
+            .id(name_id(at))
+            .desired_width(rect.width()),
+    )
+    .widget_info(|| egui::WidgetInfo::text_edit(true, "", sql.as_str(), named.as_ref()));
+}
diff --git a/src/ui/connect_dialog/join.rs b/src/ui/connect_dialog/join.rs
new file mode 100644
--- /dev/null
+++ b/src/ui/connect_dialog/join.rs
@@ -0,0 +1,232 @@
+//! A Files connection's Join with saved connections section, in either
+//! look: the saved PostgreSQL, MySQL and SQLite connections it can
+//! attach, to join a file to their live tables in one query, each by the
+//! name SQL calls it by. Closed, it says which are attached.
+
+use egui::{CornerRadius, Ui};
+use tabletist_db::Driver;
+
+use super::Skin;
+use crate::connections::join::{Blocked, Joinable};
+use crate::i18n::{Locale, Msgid, gettext, msgid};
+use crate::model::{Action, ConnectionForm};
+use crate::typography::TextRole;
+use crate::ui::attach_list::{self, Row};
+use crate::ui::disclosure::{self, Section};
+use crate::ui::widgets;
+
+/// What the open section says under its title.
+const HINT: Msgid =
+    msgid("Attach a saved connection to join a file to its live tables in one query. Optional.");
+
+/// What the open section says under the list: an attached connection
+/// keeps its own safety.
+const SAFETY: Msgid = msgid("Attached connections open read-only here, as the files do.");
+
+/// The closed row's summary: none attached, else their names.
+pub(super) fn summary(form: &ConnectionForm, locale: Locale) -> String {
+    match form.files_attach.as_slice() {
+        [] => gettext(locale, "None attached · optional").into_owned(),
+        attached => {
+            let names: Vec<&str> = attached.iter().map(|each| each.name.as_str()).collect();
+            gettext(locale, "Attached as {names}").replace("{names}", &names.join(", "))
+        }
+    }
+}
+
+/// The engine's name, as a row says it.
+fn engine(driver: Driver) -> &'static str {
+    match driver {
+        Driver::Postgres => "PostgreSQL",
+        Driver::MySql => "MySQL",
+        Driver::Sqlite => "SQLite",
+        Driver::BigQuery => "BigQuery",
+        Driver::Files => "Files",
+    }
+}
+
+/// Why a row cannot be ticked, in `locale`.
+fn blocked(why: Blocked, locale: Locale) -> String {
+    let words = match why {
+        Blocked::AsksPassword => msgid("Asks for its password on every connect"),
+        Blocked::NoDatabase => msgid("Names no database"),
+    };
+    words.say(locale).into_owned()
+}
+
+/// The sample query, for the first attached connection's `name`.
+fn sample(name: &str) -> String {
+    format!(
+        "SELECT r.order_id, p.name\n  FROM returns r\n  JOIN {name}.publishers p ON p.id = r.publisher_id"
+    )
+}
+
+/// The section after Reading CSV, in either look: the saved connections
+/// the form lists ([`ConnectionForm::joinable`]), in the list's order.
+pub(super) fn section(
+    ui: &mut Ui,
+    form: &mut ConnectionForm,
+    skin: &Skin,
+    actions: &mut Vec<Action>,
+) {
+    let joinable = std::mem::take(&mut form.joinable);
+    let Skin { look, palette, .. } = *skin;
+    let title = skin.say(msgid("Join with saved connections"));
+    let summary = look.label(&summary(form, skin.locale));
+    let hint = match look.terminal {
+        true => skin.say(msgid("optional")),
+        false => String::new(),
+    };
+    let opens = skin.opens(ui.ctx());
+    let section = Section {
+        salt: "join",
+        title: &title,
+        summary: &summary,
+        hint: &hint,
+        opens: &opens,
+    };
+    let mut open = form.join_open;
+    disclosure::show(
+        ui,
+        &section,
+        &mut open,
+        look,
+        palette,
+        skin.fill,
+        skin.rule(),
+        |ui| rows(ui, form, &joinable, skin, actions),
+    );
+    form.join_open = open;
+    form.joinable = joinable;
+}
+
+/// The open section: the hint (macOS), the list, the sample query and the
+/// safety line.
+fn rows(
+    ui: &mut Ui,
+    form: &mut ConnectionForm,
+    joinable: &[Joinable],
+    skin: &Skin,
+    actions: &mut Vec<Action>,
+) {
+    let Skin { look, palette, .. } = *skin;
+    let small = TextRole::pick(look, TextRole::Secondary, TextRole::OSecondary);
+    widgets::label(ui, small, &look.label(&skin.say(HINT)), palette.dim, look);
+    ui.add_space(8.0);
+    let first = form.files_attach.first().map(|each| each.name.clone());
+    if joinable.is_empty() {
+        let none = skin.say(msgid(
+            "No saved PostgreSQL, MySQL or SQLite connection to attach.",
+        ));
+        widgets::label(ui, small, &none, palette.dim, look);
+    } else {
+        let typed: Vec<String> = form
+            .files_attach
+            .iter()
+            .map(|each| each.name.clone())
+            .collect();
+        let mut names: std::collections::HashMap<&str, &mut String> = form
+            .files_attach
+            .iter_mut()
+            .map(|each| (each.connection.as_str(), &mut each.name))
+            .collect();
+        let mut listed: Vec<Row<'_>> = joinable
+            .iter()
+            .map(|each| Row {
+                name: &each.name,
+                environment: each.environment,
+                engine: engine(each.driver),
+                database: &each.database,
+                read_only: each.read_only,
+                blocked: each
+                    .blocked
+                    .map(|why| look.label(&blocked(why, skin.locale))),
+                attached: names.remove(each.id.0.as_str()),
+            })
+            .collect();
+        let name = skin.say(msgid("Saved connections to attach"));
+        let mut draw =
+            |ui: &mut Ui| attach_list::show(ui, &name, &mut listed, (look, palette, skin.locale));
+        let toggled = match look.terminal {
+            true => {
+                let mut toggled = None;
+                let label = skin.say(msgid("Attach"));
+                let height = 28.0 * joinable.len() as f32;
+                super::terminal::terminal_row_of(ui, &label, height, skin, |ui, _| {
+                    toggled = draw(ui);
+                });
+                toggled
+            }
+            false => draw(ui),
+        };
+        drop(listed);
+        // A name typed over: a Test of the names before says nothing of it.
+        if !form
+            .files_attach
+            .iter()
+            .map(|each| &each.name)
+            .eq(typed.iter())
+        {
+            form.forget_test();
+        }
+        if let Some(at) = toggled {
+            actions.push(Action::ToggleAttach(joinable[at].id.clone()));
+        }
+    }
+    // SQL keeps its case in either look.
+    if let Some(name) = first {
+        ui.add_space(8.0);
+        let role = TextRole::pick(look, TextRole::MonoSecondary, TextRole::OBody);
+        egui::Frame::new()
+            .fill(match look.terminal {
+                true => palette.window,
+                false => palette.panel,
+            })
+            .corner_radius(CornerRadius::same(if look.terminal { 0 } else { 8 }))
+            .inner_margin(egui::Margin::symmetric(10, 8))
+            .show(ui, |ui| {
+                ui.set_width(ui.available_width());
+                for line in sample(&name).lines() {
+                    widgets::label(ui, role, line, palette.secondary, look);
+                }
+            });
+    }
+    ui.add_space(8.0);
+    let safety = look.label(&skin.say(SAFETY));
+    widgets::label(ui, small, &safety, palette.dim, look);
+}
+
+#[cfg(test)]
+mod tests {
+    use super::*;
+    use tabletist_db::files::Attach;
+
+    #[test]
+    fn the_summary_says_what_is_attached() {
+        let mut form = ConnectionForm::default();
+        assert_eq!(summary(&form, Locale::English), "None attached · optional");
+        form.files_attach = vec![
+            Attach {
+                connection: "a".into(),
+                name: "bookshop_dev".into(),
+            },
+            Attach {
+                connection: "b".into(),
+                name: "bookshop_production".into(),
+            },
+        ];
+        assert_eq!(
+            summary(&form, Locale::English),
+            "Attached as bookshop_dev, bookshop_production"
+        );
+    }
+
+    #[test]
+    fn the_sample_uses_the_first_name() {
+        assert_eq!(
+            sample("bookshop_dev"),
+            "SELECT r.order_id, p.name\n  FROM returns r\n  JOIN bookshop_dev.publishers p ON \
+             p.id = r.publisher_id"
+        );
+    }
+}
diff --git a/src/ui/connect_dialog/mod.rs b/src/ui/connect_dialog/mod.rs
--- a/src/ui/connect_dialog/mod.rs
+++ b/src/ui/connect_dialog/mod.rs
@@ -9,6 +9,7 @@
 #[cfg(test)]
 pub(crate) use files::source_line_id;
 mod group;
+mod join;
 mod reading;
 mod sheet;
 mod terminal;
@@ -484,6 +485,12 @@
         // The override whose row has the keyboard, which `R` resets (the
         // terminal look's key).
         let mut reset = reading::focused_override(ctx, form);
+        // The attach list's row whose box has the keyboard, and the row
+        // the arrows move it to.
+        let attaching = is_files(form)
+            .then(|| crate::ui::attach_list::focused(ctx, form.joinable.len()))
+            .flatten();
+        let mut moved_to = None;
         let toggle_url = ctx.input_mut(|input| {
             // The form's cancel closes the dialog, unless it is closing an
             // open list first.
@@ -562,8 +569,16 @@
                         _ => true,
                     }
                 });
-                // The arrows move through the list of files.
-                if is_files(form) && form.files_kind == FilesKind::Files {
+                // The arrows move through the attach list while one of its
+                // boxes has the keyboard, else through the list of files.
+                if let Some(at) = attaching {
+                    let step = moves(input, keymap, layout);
+                    let rows = form.joinable.len() as isize;
+                    let next = (at as isize + step).clamp(0, rows - 1) as usize;
+                    if next != at {
+                        moved_to = Some(crate::ui::attach_list::toggle_id(next));
+                    }
+                } else if is_files(form) && form.files_kind == FilesKind::Files {
                     let rows = form.files_list.len();
                     let step = moves(input, keymap, layout);
                     if rows > 0 && step != 0 {
@@ -578,6 +593,9 @@
         // memory, which sits behind the same lock.
         if toggle_url {
             show_url(ctx, form, !form.url_mode);
+        }
+        if let Some(id) = moved_to {
+            ctx.memory_mut(|memory| memory.request_focus(id));
         }
     }
     resets_once(&mut actions);
@@ -1451,9 +1469,24 @@
     Partial(String),
 }
 
+/// What the footer says while DuckDB downloads `reader`.
+fn downloading(reader: tabletist_db::files::Reader) -> Msgid {
+    use tabletist_db::files::Reader;
+    match reader {
+        Reader::Excel => msgid("Downloading the Excel reader for DuckDB…"),
+        Reader::Postgres => msgid("Downloading the PostgreSQL reader for DuckDB…"),
+        Reader::MySql => msgid("Downloading the MySQL reader for DuckDB…"),
+        Reader::Sqlite => msgid("Downloading the SQLite reader for DuckDB…"),
+    }
+}
+
 fn status(form: &ConnectionForm, skin: &Skin) -> Option<Status> {
     match form.test {
-        TestState::Running(_) => Some(Status::Running(skin.say(msgid("Testing…")))),
+        // DuckDB downloads a reader the first time a connection needs it.
+        TestState::Running(_) => Some(Status::Running(match form.files_downloading {
+            None => skin.say(msgid("Testing…")),
+            Some(reader) => skin.say(downloading(reader)),
+        })),
         TestState::Passed => {
             let driver = form
                 .test_spec
diff --git a/src/ui/connect_dialog/sheet.rs b/src/ui/connect_dialog/sheet.rs
--- a/src/ui/connect_dialog/sheet.rs
+++ b/src/ui/connect_dialog/sheet.rs
@@ -277,6 +277,8 @@
                         });
                         ui.add_space(12.0);
                         super::reading::section(ui, form, skin, actions);
+                        ui.add_space(12.0);
+                        super::join::section(ui, form, skin, actions);
                     }
                     Driver::Postgres | Driver::MySql => {
                         fieldset(ui, &skin.say(msgid("Server")), skin, |ui| {
diff --git a/src/ui/connect_dialog/terminal.rs b/src/ui/connect_dialog/terminal.rs
--- a/src/ui/connect_dialog/terminal.rs
+++ b/src/ui/connect_dialog/terminal.rs
@@ -362,6 +362,7 @@
                 Driver::Files => {
                     super::files::terminal(ui, form, skin, actions);
                     super::reading::section(ui, form, skin, actions);
+                    super::join::section(ui, form, skin, actions);
                 }
                 Driver::Postgres | Driver::MySql => {
                     terminal_section(ui, &skin.say(msgid("Server")), skin);
diff --git a/src/ui/mod.rs b/src/ui/mod.rs
--- a/src/ui/mod.rs
+++ b/src/ui/mod.rs
@@ -17,6 +17,7 @@
 mod agent_settings_tests;
 #[cfg(test)]
 mod appearance_settings_tests;
+pub mod attach_list;
 pub mod banner;
 pub mod calendar;
 #[cfg(test)]
@@ -76,8 +77,6 @@
 pub mod file_list;
 #[cfg(test)]
 mod files_tests;
-#[cfg(test)]
-mod join_tests;
 pub mod filter_bar;
 pub mod focus;
 #[cfg(test)]
@@ -98,6 +97,8 @@
 mod indent_settings_tests;
 #[cfg(test)]
 mod insert_audit_tests;
+#[cfg(test)]
+mod join_tests;
 pub mod json_text;
 pub mod json_view;
 mod key_picker;
@@ -269,6 +270,7 @@
             form.editing.as_ref(),
             &app.connections.connections,
         );
+        form.joinable = crate::connections::join::joinable(&app.connections.connections);
         let asked =
             connect_dialog::show(&ui.ctx().clone(), paint, form, &app.keymap, loaded, groups);
         app.actions.extend(asked);
diff --git a/tests/environments.rs b/tests/environments.rs
--- a/tests/environments.rs
+++ b/tests/environments.rs
@@ -196,6 +196,9 @@
         "src/ui/quick_open.rs",
         // The Export sheet's subtitle, and the float's head.
         "src/ui/export.rs",
+        // A Files connection's Join section: each saved connection's
+        // stripe and badge.
+        "src/ui/attach_list.rs",
     ];
     let mut found = Vec::new();
     for (path, text) in sources() {
````

- [ ] **Step 4: Bring the catalogs up to date and translate the new words**

Run `sh assets/i18n/update.sh`, then fill in each `msgstr`:

| msgid | ru | pt-BR |
| --- | --- | --- |
| Attach | Присоединить | Anexar |
| SQL name | Имя в SQL | Nome no SQL |
| as | как | como |
| not attached | не присоединено | não anexada |
| Attach a saved connection to join a file to its live tables in one query. Optional. | Присоедините сохранённое подключение, чтобы соединить файл с его живыми таблицами в одном запросе. Необязательно. | Anexe uma conexão salva para cruzar um arquivo com as tabelas dela em uma só consulta. Opcional. |
| Attached as {names} | Присоединены как {names} | Anexadas como {names} |
| Attached connections open read-only here, as the files do. | Присоединённые подключения открываются здесь только для чтения, как и файлы. | As conexões anexadas abrem aqui somente para leitura, como os arquivos. |
| Join with saved connections | Соединение с сохранёнными подключениями | Cruzar com conexões salvas |
| No saved PostgreSQL, MySQL or SQLite connection to attach. | Нет сохранённого подключения PostgreSQL, MySQL или SQLite, которое можно присоединить. | Nenhuma conexão salva de PostgreSQL, MySQL ou SQLite para anexar. |
| None attached · optional | Ничего не присоединено · необязательно | Nenhuma anexada · opcional |
| Saved connections to attach | Сохранённые подключения для присоединения | Conexões salvas para anexar |
| optional | необязательно | opcional |
| Downloading the Excel reader for DuckDB… | Загрузка модуля чтения Excel для DuckDB… | Baixando o leitor de Excel do DuckDB… |
| Downloading the MySQL reader for DuckDB… | Загрузка модуля чтения MySQL для DuckDB… | Baixando o leitor de MySQL do DuckDB… |
| Downloading the PostgreSQL reader for DuckDB… | Загрузка модуля чтения PostgreSQL для DuckDB… | Baixando o leitor de PostgreSQL do DuckDB… |
| Downloading the SQLite reader for DuckDB… | Загрузка модуля чтения SQLite для DuckDB… | Baixando o leitor de SQLite do DuckDB… |
| Asks for its password on every connect | Спрашивает пароль при каждом подключении | Pede a senha a cada conexão |
| Names no database | Не указана база данных | Não informa um banco de dados |

Run `sh assets/i18n/update.sh` again. Then `sh assets/i18n/update.sh --check` and `sh assets/i18n/update_test.sh` pass, with every entry translated.

- [ ] **Step 5: Run them to see them pass**

Run: `~/.cargo/bin/cargo test -q -p tabletist --lib join`

Expected: 13 pass: the 9 in `join_tests`, the 2 in `connect_dialog::join`, and two existing tests whose names hold "join" (`schemas_that_arrive_join_an_open_list`, `the_export_rows_stay_and_a_cancel_row_joins_while_one_runs`).

- [ ] **Step 6: Run the full check**

Run the five commands of the full check above.

Expected: all pass; 5,718 passed, 0 failed.

- [ ] **Step 7: Commit**

```bash
git add src/ui tests/environments.rs assets/i18n docs/_reference/connections.md docs/_reference/settings-and-files.md docs/_guide/omarchy.md
git commit -m "Join a Files connection to saved connections from its dialog, in both looks"
```

### After the tasks

- [ ] **Hand test**, in both looks, with the Bookshop's exports and the test servers' `bookshop` databases saved as connections, their passwords in the keyring:
  1. New connection, Files, the exports folder. Open Join with saved connections, tick PostgreSQL's and the SQLite file. Their names are their databases'; type one over.
  2. Test. The first time, it says DuckDB is downloading each reader. Then it reads the files.
  3. Save and connect. The sidebar has each attached name as a schema. `SELECT count(*) FROM bookshop.order_items i JOIN <name>.books b ON b.id = i.book_id` runs, and `CREATE TABLE <name>.probe (id int)` is refused.
  4. A saved connection whose password is asked for each time can't be ticked, and says why.
  5. Compare with "macOS – Files, join" and "Omarchy – Files, join" by eye.

  Any screenshot stays local, or is blurred.
- [ ] Push the branch and open a pull request against `main`. Its description lists what the prompt asks of it:
  - a summary;
  - how attach works and where credentials go;
  - the `connections.toml` format;
  - the new components;
  - the open questions (1, the readers download on first use; 4, attached is read-only);
  - the departures above;
  - the canvas boards;
  - that the Portuguese wants a native review.

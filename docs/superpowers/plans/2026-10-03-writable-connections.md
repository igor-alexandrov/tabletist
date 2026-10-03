# Writable Connections Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A saved connection whose "Open read-only" box is off opens a read-write session, while browsing, a raw WHERE and the SQL editor still cannot write. Nothing in the app writes yet.

**Architecture:** `tabletist-db` gains `Access { ReadOnly, Writable }`, given to `Connection::connect_with`. A read-only session is exactly today's. A writable one skips the session-level read-only setting, and the script runner fences itself where it leaned on that setting (MySQL: session read-only for the run; SQLite: `query_only` refused and checked). The app takes the access from the saved connection each time a tab connects, and the screens that say "read-only" say it only when it is true.

**Tech Stack:** Rust 1.98, `tokio-postgres`, `mysql_async`, `rusqlite`, egui 0.36. Spec: `docs/superpowers/specs/2026-10-03-value-editing-core-design.md` (this plan is its step 1).

---

## Before you start

- Cargo is `~/.cargo/bin/cargo` (the mise shim fails). Never point `CARGO_TARGET_DIR` at `/tmp`.
- Checks, from `AGENTS.md`:

      ~/.cargo/bin/cargo fmt --all --check
      ~/.cargo/bin/cargo clippy --locked --workspace --all-targets -- -D warnings
      ~/.cargo/bin/cargo test --locked --workspace --all-targets

- Tasks 2 and 3 need the test servers. Without them their tests print "skipped" and prove nothing:

      docker compose up -d --build --wait postgres mysql
      export TABLETIST_TEST_PG_URL=postgres://tabletist:tabletist@localhost:55432/tabletist
      export TABLETIST_TEST_MYSQL_URL=mysql://tabletist:tabletist@localhost:53306/tabletist

  If docker is not available, do tasks 2 and 3 all the same, say in the report that their integration tests were only compiled, and leave them for CI.
- House rules that bite here: no em dashes anywhere; comments explain why, in the surrounding code's voice; a view never names a font or paints a focus ring (`src/ui/focus.rs` does); do not weaken a lint or delete a test to get green. A test this plan changes is named in its task.
- Commit after every task. Subjects are plain sentences ("Open a writable connection's session read-write"), each ending with:

      Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>

## File map

| File | What changes |
|---|---|
| `crates/tabletist-db/src/lib.rs` | `Access`, `Connection::access`, `connect_with(.., access)`, crate docs |
| `crates/tabletist-db/src/pg.rs` | `connect(.., access)`: a writable session skips the read-only default |
| `crates/tabletist-db/src/mysql.rs` | `Conn::access`, `prepare_session(conn, access)` |
| `crates/tabletist-db/src/mysql/script.rs` | the fence at `open`, `Ended::Unopened` |
| `crates/tabletist-db/src/sqlite.rs` | `open(path, access)`, the `query_only` check in the script runner |
| `crates/tabletist-db/src/sql.rs` | SQLite refusal of `PRAGMA query_only` and `writable_schema` |
| `crates/tabletist-db/src/query.rs` | the comment on `raw_where` |
| `crates/tabletist-db/tests/{sqlite,postgres,mysql,ssh}.rs` | both modes |
| `src/connections.rs` | `SavedConnection::access` |
| `src/backend.rs` | `Command::Connect { access }` |
| `src/model.rs` | `Workspace::access` |
| `src/app.rs` | `send_connect` reads the saved box |
| `src/testing.rs` | the fixture connection is read-only |
| `src/ui/connect_dialog/{sheet,terminal}.rs` | the box unlocks |
| `src/ui/workspace.rs`, `src/ui/data_view.rs`, `src/ui/row_panel.rs` | read-only marks follow the connection |
| `src/ui/sql_results.rs` | the refused-write card |
| `docs/superpowers/specs/*.md`, `README.md` | the promise, restated |

---

### Task 1: `Access` on a connection

**Files:**
- Modify: `crates/tabletist-db/src/lib.rs`
- Modify: `src/backend.rs` (two `connect_with` calls), `crates/tabletist-db/tests/ssh.rs` (five)
- Test: `crates/tabletist-db/tests/sqlite.rs`

- [ ] **Step 1: Write the failing test**

In `crates/tabletist-db/tests/sqlite.rs`, add `Access` and `HostKeys` to the `use tabletist_db::{..}` list, and replace `fixture()` with:

```rust
async fn fixture_as(access: Access) -> (Connection, tempfile::TempDir) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("fixture.db");
    tabletist_db::fixtures::write_sqlite_demo(&path).unwrap();
    let connection = Connection::connect_with(
        &ConnectSpec::sqlite(&path),
        &Secrets::default(),
        &HostKeys::default(),
        access,
    )
    .await
    .unwrap();
    (connection, dir)
}

async fn fixture() -> (Connection, tempfile::TempDir) {
    fixture_as(Access::ReadOnly).await
}

#[tokio::test]
async fn a_connection_knows_the_access_it_was_opened_with() {
    let (connection, _dir) = fixture().await;
    assert_eq!(connection.access(), Access::ReadOnly);
    let (writable, _dir) = fixture_as(Access::Writable).await;
    assert_eq!(writable.access(), Access::Writable);
}
```

- [ ] **Step 2: Run it and see it fail**

Run: `~/.cargo/bin/cargo test --locked -p tabletist-db --test sqlite a_connection_knows`
Expected: does not compile, "no `Access` in the root".

- [ ] **Step 3: Add `Access`**

In `crates/tabletist-db/src/lib.rs`, after the `pub use` lines:

```rust
/// Whether a session may write. The app has no writing call yet; a
/// writable session is the one a later `write` will be allowed on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Access {
    /// Nothing can write: the session itself is read-only.
    #[default]
    ReadOnly,
    /// The session is read-write. Browsing still reads in read-only
    /// transactions, and a script still cannot write.
    Writable,
}
```

Give `Connection` the field and its reader, and thread it through `connect_with`:

```rust
pub struct Connection {
    inner: Inner,
    access: Access,
    /// Declared after `inner`, so the driver closes before its tunnel.
    tunnel: Option<ssh::Tunnel>,
}
```

```rust
    /// A read-only session, with no SSH host keys trusted.
    pub async fn connect(spec: &ConnectSpec, secrets: &Secrets) -> Result<Self> {
        Self::connect_with(spec, secrets, &HostKeys::default(), Access::ReadOnly).await
    }

    /// Connects, through an SSH tunnel when the spec has one; `host_keys`
    /// are the SSH host keys the user trusts, and `access` says whether
    /// the session may write.
    pub async fn connect_with(
        spec: &ConnectSpec,
        secrets: &Secrets,
        host_keys: &HostKeys,
        access: Access,
    ) -> Result<Self> {
```

Set `access` in both `Self { .. }` literals of `connect_with`, and add:

```rust
    /// The access the session was opened with.
    pub fn access(&self) -> Access {
        self.access
    }
```

`Connection::close` destructures `Self`: it becomes `let Self { inner, tunnel, .. } = self;`.

`Inner::connect` is not touched yet: the drivers take `access` in tasks 2 to 4.

- [ ] **Step 4: Fix the callers**

Add `Access::ReadOnly` as the last argument of the two `Connection::connect_with` calls in `src/backend.rs` (`Command::Connect` and `Command::Test`; import `tabletist_db::Access` beside the other `tabletist_db` names) and of the five in `crates/tabletist-db/tests/ssh.rs`.

- [ ] **Step 5: Run the tests**

Run: `~/.cargo/bin/cargo test --locked --workspace --all-targets`
Expected: PASS, including `a_connection_knows_the_access_it_was_opened_with`.

- [ ] **Step 6: Commit**

```bash
git add -A && git commit -m "Tell a connection the access it opens with"
```

---

### Task 2: PostgreSQL, a writable session

**Files:**
- Modify: `crates/tabletist-db/src/lib.rs` (`Inner::connect`), `crates/tabletist-db/src/pg.rs:225-253`
- Test: `crates/tabletist-db/tests/postgres.rs`

- [ ] **Step 1: Write the failing tests**

In `crates/tabletist-db/tests/postgres.rs` import `Access` and `HostKeys`, and replace `connect()` with:

```rust
async fn connect_as(access: Access) -> Option<Connection> {
    let Some((spec, secrets)) = spec() else {
        eprintln!("skipped: TABLETIST_TEST_PG_URL is not set");
        return None;
    };
    load_fixture().await;
    Some(
        Connection::connect_with(&spec, &secrets, &HostKeys::default(), access)
            .await
            .unwrap(),
    )
}

async fn connect() -> Option<Connection> {
    connect_as(Access::ReadOnly).await
}

#[tokio::test]
async fn a_writable_session_keeps_the_servers_default_and_a_script_still_runs_read_only() {
    for (access, default) in [(Access::ReadOnly, "on"), (Access::Writable, "off")] {
        let Some(connection) = connect_as(access).await else {
            return;
        };
        let outcome = run(
            &connection,
            "SHOW default_transaction_read_only; SHOW transaction_read_only",
            10,
        )
        .await
        .unwrap();
        let shown = |index: usize| match &outcome.results[index].outcome {
            StatementOutcome::Rows { rows, .. } => rows[0][0].clone(),
            other => panic!("{other:?}"),
        };
        assert_eq!(shown(0), Value::Text(default.into()), "{access:?}");
        // The script's own transaction is read-only whatever the session.
        assert_eq!(shown(1), Value::Text("on".into()), "{access:?}");
    }
}
```

Then make the two existing guard tests run in both modes. In `bypasses_cannot_write` and in `a_raw_where_cannot_write_or_leave_read_only_mode`, replace the opening

```rust
    let Some(connection) = connect().await else {
        return;
    };
```

with a loop that holds the rest of the body:

```rust
    for access in [Access::ReadOnly, Access::Writable] {
        let Some(connection) = connect_as(access).await else {
            return;
        };
        // ... the body as it was, unchanged ...
    }
```

Add `{access:?}` to nothing: the assertions already name their attempt.

- [ ] **Step 2: Run and see the new test fail**

Run: `~/.cargo/bin/cargo test --locked -p tabletist-db --test postgres a_writable_session`
Expected: FAIL, `left: Text("on")`, `right: Text("off")` for `Writable`.

- [ ] **Step 3: Implement**

`Inner::connect` in `lib.rs` takes `access: Access` as its last parameter (both calls in `connect_with` pass it) and hands it to PostgreSQL only for now:

```rust
            Driver::Postgres => Ok(Self::Postgres(Box::new(
                pg::Conn::connect(spec, secrets, via, access).await?,
            ))),
```

In `pg.rs`, `connect` takes `access: Access` (import `crate::Access`), and the `batch_execute` becomes:

```rust
        // A writable session keeps the server's default. Row fetches,
        // counts and the script runner open read-only transactions of
        // their own, and none of the script guard's checks read the
        // session's default.
        let setup = match access {
            Access::ReadOnly => {
                "SET default_transaction_read_only = on; SET standard_conforming_strings = on"
            }
            Access::Writable => "SET standard_conforming_strings = on",
        };
        client.batch_execute(setup).await.map_err(query_error)?;
```

Any other caller of `pg::Conn::connect` (the unit tests in `pg.rs` and `pg/script.rs`) passes `Access::ReadOnly`.

- [ ] **Step 4: Run the PostgreSQL suite**

Run: `~/.cargo/bin/cargo test --locked -p tabletist-db --test postgres`
Expected: PASS. `bypasses_cannot_write` passing in `Writable` is the point: every attempt is refused or fails with 25006, and `probe` stays empty.

- [ ] **Step 5: Commit**

```bash
git add -A && git commit -m "Open a writable PostgreSQL session read-write"
```

---

### Task 3: MySQL, a writable session and the script's fence

**Files:**
- Modify: `crates/tabletist-db/src/lib.rs` (`Inner::connect`), `crates/tabletist-db/src/mysql.rs` (struct `Conn`, `connect`, `prepare_session`, its unit test near line 1152), `crates/tabletist-db/src/mysql/script.rs`
- Test: `crates/tabletist-db/tests/mysql.rs`

Why this task exists: MySQL commits implicitly before DDL, and only the session's read-only setting refuses the DDL that follows. The runner's checks (`standing`, `stands`) read `@@session.transaction_read_only`. On a read-write session a `DROP TABLE` in the SQL editor would go through. So a run makes the session read-only first and the cleanup puts read-write back.

- [ ] **Step 1: Write the failing tests**

In `crates/tabletist-db/tests/mysql.rs` import `Access` and `HostKeys`, and replace `connect()` as in task 2 (`connect_as(access)` calling `connect_with`, `connect()` calling `connect_as(Access::ReadOnly)`, the skip message naming `TABLETIST_TEST_MYSQL_URL`). Add:

```rust
/// Whether the session is read-write between scripts. Asked through a
/// count, which runs outside any script: its WHERE reads the session's
/// own setting.
async fn writes_between_scripts(connection: &Connection) -> bool {
    let mut query = users(1);
    query.raw_where = Some("@@session.transaction_read_only = 0".into());
    connection.count_rows(&query).await.unwrap() == 5
}

#[tokio::test]
async fn a_writable_session_is_read_only_for_a_script_and_read_write_after_it() {
    let Some(writable) = connect_as(Access::Writable).await else {
        return;
    };
    assert!(writes_between_scripts(&writable).await);
    let rows = rows_of(&writable, "SELECT @@session.transaction_read_only").await;
    assert_eq!(rows[0], [Value::Int(1)]);
    assert!(writes_between_scripts(&writable).await);
    // A read-only session never is.
    let Some(read_only) = connect().await else {
        return;
    };
    assert!(!writes_between_scripts(&read_only).await);
    rows_of(&read_only, "SELECT 1").await;
    assert!(!writes_between_scripts(&read_only).await);
}
```

Wrap the bodies of `bypasses_cannot_write`, `ddl_is_refused_as_read_only_and_ends_the_script` and `a_raw_where_cannot_write_or_chain_statements` in `for access in [Access::ReadOnly, Access::Writable] { .. }` exactly as in task 2. At the end of the loop body in `ddl_is_refused_as_read_only_and_ends_the_script` add:

```rust
        assert_eq!(
            writes_between_scripts(&connection).await,
            access == Access::Writable,
            "{access:?}"
        );
```

- [ ] **Step 2: Run and see them fail**

Run: `~/.cargo/bin/cargo test --locked -p tabletist-db --test mysql`
Expected: does not compile until `connect_as` has its `Access`; once it does, `a_writable_session_is_read_only_for_a_script_and_read_write_after_it` FAILS on its first assertion (the session is still read-only), and so does the `writes_between_scripts` line added to the DDL test, for `Writable`.

- [ ] **Step 3: The session**

`Inner::connect` passes `access` to `mysql::Conn::connect(spec, secrets, via, access)`. In `mysql.rs` (import `crate::Access`):

- `Conn` gains `/// What the session was opened as: a script puts it back.` `pub(crate) access: Access,` set in `connect`'s `Ok(Self { .. })`.
- `connect` calls `prepare_session(&mut conn, access).await?`. The module's own tests call `Conn::connect(&spec, &secrets, None)` (near lines 1005 and 1087): they pass `Access::ReadOnly`.
- Beside `READ_ONLY`:

```rust
/// Makes the session read-write again, after a script's fence or a reset
/// on a server whose default is read-only.
const READ_WRITE: &str = "SET SESSION TRANSACTION READ WRITE";
```

- `READ_ONLY` becomes `pub(crate)`, and `prepare_session`:

```rust
/// The session settings every connection runs with, set at connect and
/// again after a SQL editor script's reset, which undoes them. A read-only
/// session says so first, so nothing here runs in a session that could
/// write; a writable one says read-write last. After a reset a cancel
/// meant for a statement can land here, so a statement it interrupts runs
/// once more: the session is not left in the wrong mode because a `SET`
/// was interrupted.
pub(crate) async fn prepare_session(conn: &mut mysql_async::Conn, access: Access) -> Result<()> {
    let sql_mode = LEXING_MODES
        .iter()
        .fold("@@SESSION.sql_mode".to_owned(), |mode, name| {
            format!("REPLACE({mode}, '{name}', '')")
        });
    let sql_mode = format!("SET SESSION sql_mode = {sql_mode}");
    let statements = match access {
        Access::ReadOnly => [READ_ONLY, NAMES, sql_mode.as_str()],
        Access::Writable => [NAMES, sql_mode.as_str(), READ_WRITE],
    };
    // Fixed statements: safe to send through the text protocol.
    for statement in statements {
        retry_cancelled!(execute(conn, statement))?;
    }
    Ok(())
}
```

(Keep the visibility `prepare_session` has today if `mysql/script.rs` already reaches it through `super::`.)

- In the unit test near `mysql.rs:1152`, `prepare_session(&mut conn)` becomes `prepare_session(&mut conn, Access::ReadOnly)`, and after its assertions, inside the same loop, add:

```rust
            prepare_session(&mut conn, Access::Writable).await.unwrap();
            let (read_only, sql_mode): (i64, String) = conn
                .query_first("SELECT @@session.transaction_read_only, @@session.sql_mode")
                .await
                .unwrap()
                .unwrap();
            assert_eq!(read_only, 0, "{mode}");
            assert!(!sql_mode.contains("ANSI"), "{mode}: {sql_mode}");
```

- [ ] **Step 4: The fence**

In `mysql/script.rs` import `READ_ONLY` from `super` and `Access` from `crate`.

`Ended` gains a variant:

```rust
    /// A cancel landed on the opening queries: no statement of the script
    /// ran, so there is nothing to confirm. On a writable session the
    /// cancel may have landed before the fence, where the session is
    /// rightly read-write.
    Unopened,
```

`run_script` passes the access and uses the variant:

```rust
        let ended = match open(&mut conn, limit, self.access).await {
            // A lost session ends the run here: nothing to close.
            Ok(()) => statements(&mut conn, texts, limit as usize, stop, &mut outcome).await?,
            // A cancel landed on the opening queries: no results.
            Err(Error::Cancelled) => {
                outcome.stopped = true;
                Ended::Unopened
            }
```

and ends with `close(&mut conn, ended, self.access).await?;`.

`open`:

```rust
async fn open(conn: &mut mysql_async::Conn, limit: u32, access: Access) -> Result<()> {
    // A writable session is read-write between scripts. The checks read
    // the session's own setting, and only that setting makes the server
    // refuse DDL after the commit DDL implies: so a run makes the session
    // read-only first, and the close puts read-write back.
    if access == Access::Writable {
        retry_cancelled!(execute(conn, READ_ONLY))?;
    }
    begin(conn).await?;
    // One row more than the limit, to know whether more exist.
    let rows = u64::from(limit) + 1;
    execute(conn, &format!("SET SESSION sql_select_limit = {rows}")).await
}
```

`close` takes `access: Access`, treats `Unopened` as confirmed, and prepares the session as it was opened:

```rust
    let ended = match ended {
        Ended::Unconfirmed => match retry_cancelled!(standing(conn)) {
            Ok(Standing::Left) => Ended::Left,
            Ok(Standing::Inside | Standing::Outside) => Ended::Unconfirmed,
            Err(error) => Ended::Broken(cleanup_failed(&error)),
        },
        Ended::Unopened => Ended::Unconfirmed,
        known => known,
    };
```

```rust
    let prepared = prepare_session(conn, access).await;
    match ended {
        Ended::Left => Err(Error::LeftReadOnly),
        Ended::Broken(error) => Err(error),
        Ended::Unconfirmed | Ended::Unopened => rolled_back
            .and(reset)
            .and(prepared)
            .map_err(|error| cleanup_failed(&error)),
    }
```

The unit tests at the end of `mysql/script.rs` call `open(&mut conn, 10)` and `close(&mut conn, Ended::Unconfirmed)` directly (six calls): each gains `Access::ReadOnly`. Beside them, with the same setup they use to get a session, add one for the new path:

```rust
    /// A cancel that landed on a writable session's opening queries: the
    /// session may still be read-write, and that is not a script that left.
    #[tokio::test]
    async fn closing_an_unopened_run_keeps_a_writable_session() {
        let Some(url) = test_url() else {
            return;
        };
        let conn = session(&url).await;
        let mut conn = conn.conn.lock().await;
        execute(&mut conn, "SET SESSION TRANSACTION READ WRITE")
            .await
            .unwrap();
        close(&mut conn, Ended::Unopened, Access::Writable)
            .await
            .unwrap();
        let read_only = read_only_setting(&mut conn, READ_ONLY_SETTINGS)
            .await
            .unwrap();
        assert_eq!(read_only, Some(0));
    }
```

(`test_url` and `session` are the helpers the neighbouring tests use, from `super::super::tests`.)

Update the doc comment of `close` ("applies the connect-time settings again") and of `reset` ("until `prepare_session` runs again") only where they now say something untrue.

- [ ] **Step 5: Run the MySQL suite**

Run: `~/.cargo/bin/cargo test --locked -p tabletist-db --test mysql && ~/.cargo/bin/cargo test --locked -p tabletist-db --lib mysql`
Expected: PASS. In `Writable`, `ddl_is_refused_as_read_only_and_ends_the_script` must still see `made == Some(0)` and code 25006: that is the fence working.

- [ ] **Step 6: Commit**

```bash
git add -A && git commit -m "Fence a script on a writable MySQL session"
```

---

### Task 4: SQLite, a writable handle

**Files:**
- Modify: `crates/tabletist-db/src/lib.rs` (`Inner::connect`), `crates/tabletist-db/src/sqlite.rs:353-375` and its tests

- [ ] **Step 1: Write the failing test**

In the test module of `sqlite.rs`, `fixture()` becomes:

```rust
    async fn fixture_as(access: Access) -> (Conn, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("fixture.db");
        crate::fixtures::write_sqlite_demo(&path).unwrap();
        (Conn::open(&path, access).await.unwrap(), dir)
    }

    async fn fixture() -> (Conn, tempfile::TempDir) {
        fixture_as(Access::ReadOnly).await
    }
```

Every other `Conn::open(&path)` in that module becomes `Conn::open(&path, Access::ReadOnly)`, except in `a_missing_file_is_a_connect_error_and_is_not_created`, whose body runs for both:

```rust
        for access in [Access::ReadOnly, Access::Writable] {
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("missing.db");
            assert!(matches!(
                Conn::open(&path, access).await,
                Err(Error::Connect(_))
            ));
            assert!(!path.exists(), "{access:?}");
        }
```

Add:

```rust
    #[tokio::test]
    async fn only_a_writable_handle_writes_and_only_with_query_only_lifted() {
        for access in [Access::ReadOnly, Access::Writable] {
            let (conn, _dir) = fixture_as(access).await;
            // The standing state refuses a write on both.
            let standing = conn
                .run(|connection| {
                    connection
                        .execute("UPDATE users SET email = email WHERE id = 1", [])
                        .map_err(map_error)
                })
                .await;
            assert!(standing.is_err(), "{access:?}");
            // Lifted, as a save will lift it: only the writable handle
            // writes.
            let lifted = conn
                .run(|connection| {
                    connection
                        .execute_batch("PRAGMA query_only = OFF")
                        .map_err(map_error)?;
                    let updated = connection
                        .execute("UPDATE users SET email = email WHERE id = 1", [])
                        .map_err(map_error);
                    set_session_pragmas(connection).map_err(map_error)?;
                    updated
                })
                .await;
            assert_eq!(lifted.is_ok(), access == Access::Writable, "{access:?}");
        }
    }
```

- [ ] **Step 2: Run and see it fail**

Run: `~/.cargo/bin/cargo test --locked -p tabletist-db --lib sqlite`
Expected: does not compile (`open` takes one argument).

- [ ] **Step 3: Implement**

`Inner::connect` passes `access` to `sqlite::Conn::open(path, access)`. In `sqlite.rs` (import `crate::Access`):

```rust
    /// Opens `path`: read-only, or read-write for a writable connection.
    /// Never creates a file. Either way the session refuses writes
    /// (`PRAGMA query_only`, see `set_session_pragmas`).
    pub async fn open(path: &Path, access: Access) -> Result<Self> {
```

```rust
            let mode = match access {
                Access::ReadOnly => OpenFlags::SQLITE_OPEN_READ_ONLY,
                // Without SQLITE_OPEN_CREATE: a missing file stays missing.
                Access::Writable => OpenFlags::SQLITE_OPEN_READ_WRITE,
            };
            let flags = mode | OpenFlags::SQLITE_OPEN_NO_MUTEX;
```

Fix the module's opening doc line ("SQLite, opened read-only.") to say "SQLite, opened read-only unless the connection is writable."

- [ ] **Step 4: Run the existing guard tests in both modes**

In `crates/tabletist-db/tests/sqlite.rs`, wrap the bodies of `a_raw_where_cannot_modify_data` and `writes_fail_as_read_only_and_refusals_run_nothing` in `for access in [Access::ReadOnly, Access::Writable] { let (connection, _dir) = fixture_as(access).await; .. }`, the rest unchanged. The second pins the error code `"8"`, which `format::refuses_writes` in the app reads. If a writable handle under `query_only` gives another code, stop and report it: the app's refused-write card depends on it.

- [ ] **Step 5: Run**

Run: `~/.cargo/bin/cargo test --locked -p tabletist-db`
Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add -A && git commit -m "Open a writable SQLite file read-write, refusing writes"
```

---

### Task 5: SQLite scripts may not set `query_only` or `writable_schema`

**Files:**
- Modify: `crates/tabletist-db/src/sql.rs` (`refusal`, near line 587)
- Modify: `crates/tabletist-db/tests/sqlite.rs` (`a_script_cannot_change_the_sessions_settings_for_later`)

The refusal is the same for both accesses: `sql::refusal` knows the dialect, not the session. On a read-only handle it is redundant, which is fine.

- [ ] **Step 1: Write the failing tests**

In the test module of `sql.rs`:

```rust
    #[test]
    fn sqlite_refuses_setting_the_pragmas_that_keep_a_handle_from_writing() {
        for refused in [
            "PRAGMA query_only = OFF",
            "pragma Query_Only=0",
            "PRAGMA query_only(0)",
            "PRAGMA main.query_only = 0",
            "PRAGMA \"query_only\" = 0",
            "PRAGMA 'query_only' = 0",
            "PRAGMA `query_only` = 0",
            "PRAGMA [query_only] = 0",
            "PRAGMA main.'query_only'(0)",
            "PRAGMA 'main'.\"query_only\" = 0",
            "PRAGMA /* x */ query_only /* y */ = 0",
            "PRAGMA writable_schema = ON",
            "PRAGMA 'writable_schema'(1)",
            // Not a form SQLite takes, and refused all the same.
            "PRAGMA query_only OFF",
            // SQLite applies a flag pragma when it prepares the statement,
            // so EXPLAIN in front does not make it harmless.
            "EXPLAIN PRAGMA query_only = OFF",
            "explain query plan PRAGMA 'query_only'(0)",
        ] {
            assert!(refusal(Dialect::Sqlite, refused).is_some(), "{refused}");
        }
        assert_eq!(
            refusal(Dialect::Sqlite, "PRAGMA 'query_only' = 0").as_deref(),
            Some("PRAGMA QUERY_ONLY")
        );
        // Reading one is fine, and so is every other pragma.
        for allowed in [
            "PRAGMA query_only",
            "PRAGMA main.query_only",
            "PRAGMA 'query_only'",
            "PRAGMA table_info(users)",
            "PRAGMA foreign_keys = ON",
            "SELECT 'PRAGMA query_only = OFF'",
            "SELECT query_only FROM settings",
            "EXPLAIN SELECT 1",
            "EXPLAIN PRAGMA query_only",
        ] {
            assert_eq!(refusal(Dialect::Sqlite, allowed), None, "{allowed}");
        }
    }
```

In `crates/tabletist-db/tests/sqlite.rs`, `a_script_cannot_change_the_sessions_settings_for_later` set `query_only` off and expected no error. Its first script loses that statement (so `changed.results.len()` is 3), the rest stays, and the test gains at its start:

```rust
    // Setting query_only is refused outright: on a writable file it is
    // what keeps a script from writing.
    for access in [Access::ReadOnly, Access::Writable] {
        let (connection, _dir) = fixture_as(access).await;
        let refused = run(&connection, "SELECT 1;\nPRAGMA 'query_only' = OFF").await;
        assert!(
            matches!(refused, Err(Error::Refused { line: 2, .. })),
            "{access:?}: {refused:?}"
        );
    }
```

- [ ] **Step 2: Run and see them fail**

Run: `~/.cargo/bin/cargo test --locked -p tabletist-db sqlite_refuses_setting && ~/.cargo/bin/cargo test --locked -p tabletist-db --test sqlite a_script_cannot_change`
Expected: FAIL (`PRAGMA query_only = OFF` is not refused).

- [ ] **Step 3: Implement**

In `sql.rs`, in `refusal`, just before `match word(0)`:

```rust
    if dialect == Dialect::Sqlite
        && let Some(name) = guarded_pragma(statement, &tokens)
    {
        return Some(format!("PRAGMA {name}"));
    }
```

and below `is_guarded_setting`:

```rust
/// The SQLite pragmas a script may not set. `query_only` is what keeps a
/// writable handle from writing; `writable_schema` opens the schema table.
const GUARDED_PRAGMAS: [&str; 2] = ["QUERY_ONLY", "WRITABLE_SCHEMA"];

/// The name a token spells in a `PRAGMA`, upper-cased. SQLite reads a
/// pragma's name as a bare word, a quoted name or a string, which is why
/// this does not go through `word_of`: that skips strings.
fn pragma_name(statement: &str, token: &Token) -> Option<String> {
    matches!(
        token.kind,
        TokenKind::Keyword
            | TokenKind::Identifier
            | TokenKind::QuotedIdentifier
            | TokenKind::String
    )
    .then(|| {
        statement[token.range.clone()]
            .trim_matches(['"', '`', '[', ']', '\''])
            .to_ascii_uppercase()
    })
}

/// The guarded pragma a SQLite `PRAGMA` statement names, unless it only
/// reads it (`PRAGMA name`, `PRAGMA schema.name`, nothing after). It errs
/// toward refusing: a guarded name anywhere in a longer statement counts.
fn guarded_pragma(statement: &str, tokens: &[Token]) -> Option<&'static str> {
    let code: Vec<&Token> = tokens
        .iter()
        .filter(|token| !matches!(token.kind, TokenKind::Whitespace | TokenKind::Comment))
        .collect();
    // EXPLAIN [QUERY PLAN] in front changes nothing: SQLite applies a flag
    // pragma when it prepares the statement, explained or not.
    let explained = code
        .iter()
        .take_while(|token| {
            word_of(statement, token)
                .is_some_and(|word| matches!(word.as_str(), "EXPLAIN" | "QUERY" | "PLAN"))
        })
        .count();
    let [pragma, code @ ..] = &code[explained..] else {
        return None;
    };
    if word_of(statement, pragma).as_deref() != Some("PRAGMA") {
        return None;
    }
    let guarded = code
        .iter()
        .filter_map(|token| pragma_name(statement, token))
        .find_map(|name| GUARDED_PRAGMAS.into_iter().find(|guarded| *guarded == name))?;
    let is_name = |token: &Token| pragma_name(statement, token).is_some();
    let reads = match code {
        [name] => is_name(name),
        [schema, dot, name] => {
            is_name(schema) && &statement[dot.range.clone()] == "." && is_name(name)
        }
        _ => false,
    };
    (!reads).then_some(guarded)
}
```

- [ ] **Step 4: Run**

Run: `~/.cargo/bin/cargo test --locked -p tabletist-db`
Expected: PASS. If a spelling in the list is not refused, look at how `tokenize` lexes it before changing the test.

- [ ] **Step 5: Commit**

```bash
git add -A && git commit -m "Refuse a SQLite script that sets query_only"
```

---

### Task 6: The SQLite runner checks `query_only`

**Files:**
- Modify: `crates/tabletist-db/src/sqlite.rs` (`script`, `statements`, tests)
- Test: `crates/tabletist-db/tests/sqlite.rs`

The read-only open was this runner's independent layer; on a writable file the refusal list of task 5 is all there is. This adds the layer back: the runner reads `PRAGMA query_only` before every statement and before its rollback, and anything but 1 ends the run with `Error::LeftReadOnly`, which closes the session like a lost connection. It runs for both accesses.

- [ ] **Step 1: Write the failing tests**

In the test module of `sqlite.rs` (this calls `Conn::run_script` directly, which skips the refusal list: the "spelling the list missed"):

```rust
    #[tokio::test]
    async fn a_script_that_gets_query_only_off_is_stopped_and_writes_nothing() {
        for access in [Access::ReadOnly, Access::Writable] {
            for texts in [
                vec!["PRAGMA query_only = OFF", "UPDATE users SET email = 'x'"],
                // In last position, where no statement follows to be checked.
                vec!["SELECT 1", "PRAGMA query_only = OFF"],
            ] {
                let (conn, _dir) = fixture_as(access).await;
                let script = texts.iter().map(|text| (*text).to_owned()).collect();
                let ran = conn.run_script(script, 10, &StopFlag::new()).await;
                assert!(
                    matches!(ran, Err(Error::LeftReadOnly)),
                    "{access:?} {texts:?}: {ran:?}"
                );
                let (query_only, changed) = conn
                    .run(|connection| {
                        let one = |sql: &str| {
                            connection
                                .query_row(sql, [], |row| row.get::<_, i64>(0))
                                .map_err(map_error)
                        };
                        Ok((
                            one("PRAGMA query_only")?,
                            one("SELECT count(*) FROM users WHERE email = 'x'")?,
                        ))
                    })
                    .await
                    .unwrap();
                assert_eq!((query_only, changed), (1, 0), "{access:?} {texts:?}");
            }
        }
    }
```

In `crates/tabletist-db/tests/sqlite.rs`, what the read-only open used to stop:

```rust
#[tokio::test]
async fn a_script_on_a_writable_file_changes_no_file() {
    let (connection, dir) = fixture_as(Access::Writable).await;
    let path = dir.path().join("fixture.db");
    let before = std::fs::read(&path).unwrap();
    let missing = dir.path().join("missing.db");
    let copy = dir.path().join("copy.db");
    for text in [
        format!("ATTACH '{}' AS other", missing.display()),
        format!("ATTACH 'file:{}?mode=rwc' AS other", missing.display()),
        format!("VACUUM INTO '{}'", copy.display()),
        "PRAGMA journal_mode = WAL".to_owned(),
        "PRAGMA wal_checkpoint(TRUNCATE)".to_owned(),
        "UPDATE users SET email = 'x'".to_owned(),
        "CREATE TABLE made (n)".to_owned(),
    ] {
        // Refused, failed or harmless: each is fine, a changed file is not.
        let _ = run(&connection, &text).await;
        assert_eq!(std::fs::read(&path).unwrap(), before, "{text}");
        assert!(!missing.exists() && !copy.exists(), "{text}");
    }
    let mut names: Vec<_> = std::fs::read_dir(dir.path())
        .unwrap()
        .map(|entry| entry.unwrap().file_name())
        .collect();
    names.sort();
    assert_eq!(names, ["fixture.db"]);
}
```

- [ ] **Step 2: Run and see them fail**

Run: `~/.cargo/bin/cargo test --locked -p tabletist-db a_script_that_gets_query_only_off`
Expected: FAIL: the run returns `Ok` (and on `Writable` the first script's `UPDATE` ran before the rollback undid it).

Run: `~/.cargo/bin/cargo test --locked -p tabletist-db --test sqlite a_script_on_a_writable_file`
Expected: PASS already. **If it fails for some statement, stop and report it: that is a hole in the fence, not a test to adjust.**

- [ ] **Step 3: Implement**

In `sqlite.rs`, above `script`:

```rust
/// Whether the session still refuses writes. A script can turn
/// `query_only` off by a spelling the refusal list does not know, and on a
/// writable handle nothing else would stop its next statement.
fn still_query_only(connection: &rusqlite::Connection) -> Result<bool> {
    connection
        .query_row("PRAGMA query_only", [], |row| row.get::<_, i64>(0))
        .map(|on| on == 1)
        .map_err(map_error)
}
```

In `statements`, after the `stop.is_stopped()` block and before `let started = Instant::now();`:

```rust
        match still_query_only(connection) {
            Ok(true) => {}
            Ok(false) => return Err(Error::LeftReadOnly),
            // A stop that landed on the check: this statement is the
            // cancelled one.
            Err(Error::Cancelled) => {
                outcome.stopped = true;
                outcome.results.push(StatementResult {
                    elapsed: std::time::Duration::ZERO,
                    outcome: StatementOutcome::Cancelled,
                });
                break;
            }
            Err(error) => return Err(error),
        }
```

In `script`, the end becomes:

```rust
    let ran = statements(connection, texts, limit, stop, &mut outcome);
    // Removed before the cleanup, so a stop cannot interrupt it.
    connection.progress_handler(0, None::<fn() -> bool>);
    stop.finish();
    // Asked before the rollback, which puts the setting back: a last
    // statement that turned it off must not pass unseen. A cancel can land
    // on the question, so it is asked once more; no answer counts as left.
    let left = ran.is_ok()
        && !still_query_only(connection)
            .or_else(|_| still_query_only(connection))
            .unwrap_or(false);
    let ended = end_transaction(connection);
    ran?;
    ended.map_err(|error| crate::script::cleanup_failed(&error))?;
    if left {
        return Err(Error::LeftReadOnly);
    }
    Ok(outcome)
```

Update the doc comment of `script` ("Runs `texts` between `BEGIN` and a `ROLLBACK` that always happens.") with one sentence on the check.

- [ ] **Step 4: Run**

Run: `~/.cargo/bin/cargo test --locked -p tabletist-db`
Expected: PASS, the cancel and stop tests of `tests/sqlite.rs` included (`a_stopped_script_is_cancelled_whatever_the_timing`, `a_session_cancel_ends_the_script_as_cancelled`). If one of them now ends with `LeftReadOnly`, an interrupt reached the final check twice; report it instead of loosening the check.

- [ ] **Step 5: Commit**

```bash
git add -A && git commit -m "Check query_only around every statement of a SQLite script"
```

---

### Task 7: The app opens each session with its connection's access

**Files:**
- Modify: `src/connections.rs`, `src/backend.rs` (`Command::Connect`, its handler, the seven test constructions), `src/model.rs` (`Workspace`), `src/app.rs` (`send_connect`), `src/testing.rs` (`fixture_connection`)

- [ ] **Step 1: Write the failing tests**

In the test module of `src/connections.rs`:

```rust
    #[test]
    fn a_connection_opens_with_the_access_its_box_gives() {
        let mut connection = sample();
        connection.environment = Environment::Dev;
        assert_eq!(connection.access(), tabletist_db::Access::Writable);
        connection.environment = Environment::Production;
        assert_eq!(connection.access(), tabletist_db::Access::ReadOnly);
        connection.read_only = Some(false);
        assert_eq!(connection.access(), tabletist_db::Access::Writable);
    }
```

(Use whatever helper the neighbouring tests build a `SavedConnection` with in place of `sample()`.)

In the test module of `src/app.rs`, after `connect`:

```rust
    #[test]
    fn a_session_opens_with_the_access_its_saved_connection_asks_for() {
        use tabletist_db::Access;
        let sent = |app: &App| match app.backend.sent.last() {
            Some(Command::Connect { access, .. }) => *access,
            other => panic!("expected a Connect command, got {other:?}"),
        };
        let (mut app, _dir) = app();
        // Dev, its box never set: writable.
        let (tab, _, _) = connect(&mut app);
        assert_eq!(sent(&app), Access::Writable);
        assert_eq!(app.workspace(tab).unwrap().access, Access::Writable);
        // The box turned on meanwhile changes nothing until the tab
        // connects again.
        let id = app.workspace(tab).unwrap().conn_id.clone();
        let mut saved = app.connections.get(&id).unwrap().clone();
        saved.read_only = Some(true);
        app.connections.upsert(saved);
        assert_eq!(app.workspace(tab).unwrap().access, Access::Writable);
        app.apply(Action::Reconnect(tab));
        assert_eq!(sent(&app), Access::ReadOnly);
        assert_eq!(app.workspace(tab).unwrap().access, Access::ReadOnly);
    }
```

- [ ] **Step 2: Run and see them fail**

Run: `~/.cargo/bin/cargo test --locked -p tabletist --lib a_session_opens_with`
Expected: does not compile (`access` is unknown). After step 3 run it again, and `a_connection_opens_with` the same way.

- [ ] **Step 3: Implement**

`src/connections.rs`, in `impl SavedConnection`:

```rust
    /// The access its sessions open with.
    pub fn access(&self) -> tabletist_db::Access {
        if self.read_only() {
            tabletist_db::Access::ReadOnly
        } else {
            tabletist_db::Access::Writable
        }
    }
```

`src/backend.rs`: `Command::Connect` gains, after `host_keys`:

```rust
        /// Whether the session may write.
        access: Access,
```

Its handler destructures `access` and passes it to `Connection::connect_with(&spec, &secrets, &host_keys, access)`. `Command::Test` keeps `Access::ReadOnly`. Each `backend.send(Command::Connect { .. })` in this file's tests gains `access: Access::ReadOnly,`.

`src/model.rs`: `Workspace` gains, after `environment`:

```rust
    /// What the session was opened as. Taken from the saved connection
    /// each time the tab connects, and fixed until it connects again.
    pub access: tabletist_db::Access,
```

and `Workspace::new` sets `access: saved.access(),` (before the fields that move out of `saved`).

`src/app.rs`, `send_connect`:

```rust
    /// Sends the Connect for the tab's current session and request.
    fn send_connect(&mut self, tab: ConnTabId, secrets: Secrets) {
        // The saved connection's box as it stands now: a session's access
        // is fixed when it connects, so a reconnect picks up a change. A
        // connection deleted since keeps what the tab opened with.
        let saved = self
            .workspace(tab)
            .and_then(|workspace| self.connections.get(&workspace.conn_id))
            .map(SavedConnection::access);
        let Some(workspace) = self.workspace_mut(tab) else {
            return;
        };
        let SessionStatus::Connecting { request } = workspace.status else {
            return;
        };
        if let Some(access) = saved {
            workspace.access = access;
        }
        workspace.secrets = secrets.clone();
        workspace.connect_started = Some(std::time::Instant::now());
        let (session, spec, access) = (workspace.session, workspace.spec.clone(), workspace.access);
```

and the command gains `access,`.

`src/testing.rs`, `fixture_connection`: `read_only: Some(true),` with the comment

```rust
        // Read-only, as every connection was when most tests were written.
        // A test of a writable connection says so itself.
```

- [ ] **Step 4: Run**

Run: `~/.cargo/bin/cargo test --locked --workspace --all-targets`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add -A && git commit -m "Connect each tab with its saved connection's access"
```

---

### Task 8: The dialog's box unlocks

**Files:**
- Modify: `src/ui/connect_dialog/sheet.rs:628-685` and the `safety(ui, skin)` call near line 337
- Modify: `src/ui/connect_dialog/terminal.rs:371-393`
- Test: `src/ui/mod.rs` (`the_dialog_shows_every_connection_read_only`, near line 4991)

- [ ] **Step 1: Rewrite the test**

Replace `the_dialog_shows_every_connection_read_only` with:

```rust
    #[test]
    fn the_read_only_box_follows_the_environment_until_it_is_set() {
        for (look, said) in [
            (
                crate::theme::Look::standard(),
                "Blocks every write from this app. On by default for production; turn off to edit.",
            ),
            (
                crate::theme::Look::macos(),
                "Blocks every write from this app. On by default for production; turn off to edit.",
            ),
            (crate::theme::Look::omarchy(), "· default for production"),
        ] {
            let mut harness = Harness::new();
            harness.set_look(look);
            harness.press(Key::N, Modifiers::COMMAND);
            assert!(harness.has(said), "{}", look.name);
            let read_only = |harness: &mut Harness| {
                let tree = harness.settle();
                let (_, node) = tree
                    .nodes
                    .iter()
                    .find(|(_, node)| {
                        node.role() == egui::accesskit::Role::CheckBox
                            && node.label() == Some("Open read-only")
                    })
                    .expect("the read-only box");
                assert!(!node.is_disabled(), "{}", look.name);
                node.toggled() == Some(egui::accesskit::Toggled::True)
            };
            let click_box = |harness: &mut Harness| {
                let tree = harness.settle();
                let place = crate::testing::bounds(
                    &tree,
                    "Open read-only",
                    egui::accesskit::Role::CheckBox,
                )
                .expect("the read-only box");
                click_at(harness, place.center());
            };
            // A new connection is not production: writable, nothing set.
            assert!(!read_only(&mut harness), "{}", look.name);
            // The terminal look names its choices in lower case.
            harness.click(&look.label("Production"));
            assert!(read_only(&mut harness), "{}", look.name);
            assert_eq!(form(&harness).read_only, None, "the default, not a choice");
            // The box is the user's from the first click. Clicked by its
            // role: the sheet's title beside it has the same name.
            click_box(&mut harness);
            assert!(!read_only(&mut harness), "{}", look.name);
            assert_eq!(form(&harness).read_only, Some(false), "{}", look.name);
            click_box(&mut harness);
            assert_eq!(form(&harness).read_only, Some(true), "{}", look.name);
            // And what a connection was saved with comes back as it was.
            let mut harness = Harness::new();
            harness.set_look(look);
            let id = add_saved(&mut harness, "Shop");
            let mut saved = harness.app.connections.get(&id).unwrap().clone();
            saved.read_only = Some(false);
            harness.app.connections.upsert(saved);
            harness
                .app
                .apply(crate::model::Action::EditConnection(id.clone()));
            assert!(!read_only(&mut harness), "{}", look.name);
            harness.click("Save");
            assert!(harness.app.dialog.is_none(), "{}", look.name);
            assert_eq!(
                harness.app.connections.get(&id).unwrap().read_only,
                Some(false),
                "{}",
                look.name
            );
        }
    }
```

- [ ] **Step 2: Run and see it fail**

Run: `~/.cargo/bin/cargo test --locked -p tabletist --lib the_read_only_box_follows`
Expected: FAIL on the note's text.

- [ ] **Step 3: The sheet (macOS and Windows looks)**

In `sheet.rs`, `locked_check` becomes a box that can be clicked:

```rust
/// A 14 pt check box in the environment's colour.
fn env_check(ui: &mut Ui, on: bool, name: &str, skin: &Skin) -> Response {
    // Three below the line's top, as the design sets the box.
    let (rect, response) = ui.allocate_exact_size(vec2(14.0, 17.0), Sense::click());
    response.widget_info(|| WidgetInfo::selected(WidgetType::Checkbox, true, on, name));
    let mark = Rect::from_min_size(rect.min + vec2(0.0, 3.0), vec2(14.0, 14.0));
    let radius = CornerRadius::same(skin.look.radius.min(4));
    let painter = ui.painter();
    if on {
        let fill = skin.env.base();
        // The tick is cut out of the box in the dialog's own colour, unless
        // its text colour reads better there: the environments' colours
        // are the same on a dark sheet as on a light one.
        let tick = if theme::contrast(skin.fill, fill) >= theme::contrast(skin.palette.text, fill)
        {
            skin.fill
        } else {
            skin.palette.text
        };
        painter.rect_filled(mark, radius, fill);
        paint_check(painter, mark, tick);
    } else {
        painter.rect_stroke(
            mark,
            radius,
            Stroke::new(1.0, skin.palette.secondary),
            StrokeKind::Inside,
        );
    }
    response
}
```

(Import `Response`, `Stroke` and `StrokeKind` from `egui` if the file does not have them. The focus ring is `focus.rs`'s: if its default form sits badly on a 14 pt box, say so with `focus::hint` as the file's other custom controls do, and never paint one here.)

The note and `safety`:

```rust
/// What the read-only promise says under its title.
const READ_ONLY_NOTE: &str =
    "Blocks every write from this app. On by default for production; turn off to edit.";

/// macOS: the read-only promise, on the environment's tint. The box shows
/// the environment's default until it is clicked, and is the user's
/// choice from then on.
fn safety(ui: &mut Ui, form: &mut ConnectionForm, skin: &Skin) {
```

with, in place of `locked_check(ui, &title, skin);`:

```rust
                let on = form.read_only();
                if env_check(ui, on, &title, skin).clicked() {
                    form.read_only = Some(!on);
                }
```

The call becomes `safety(ui, form, skin);`.

- [ ] **Step 4: The terminal look**

In `terminal.rs`, inside the `Read-only` row:

```rust
                let promise = skin.say("Block every write from this app");
                let note = format!("· {}", skin.say("Default for production"));
```

and `draw` toggles the form:

```rust
                let mut draw = |ui: &mut Ui| {
                    // The environment's colour, as text can take it.
                    let mark = skin.env_ink(&skin.env);
                    let mut on = form.read_only();
                    if terminal_check(ui, Some(&mut on), &promise, &name, mark, skin).changed() {
                        form.read_only = Some(on);
                    }
                    widgets::label(ui, role, &note, palette.dim, look);
                };
```

Update the doc comment of `terminal_check` if "`None` locks it" has no caller left; if it has none, drop the `Option` rather than keep a dead branch.

- [ ] **Step 5: Two tests that quote what changed**

- `a_tall_form_that_fits_the_window_opens_whole` (`src/ui/mod.rs`, near line 5989) finds the form's last line by the old notes. They become `"· default for production"` and the new sheet sentence.
- `the_connection_dialog_takes_the_chosen_environments_colours` (`src/ui/env_tests.rs`, near line 176) expects the box filled in each environment's colour. The box now follows the environment, so only production fills it: the test sets `read_only = Some(true)` on the form it draws, so the box is on for every environment it walks.

- [ ] **Step 6: Run**

Run: `~/.cargo/bin/cargo test --locked -p tabletist --lib`
Expected: PASS, with `the_dialog_says_the_connection_opens_read_only` and `the_dialog_draws_each_looks_own_form` untouched.

- [ ] **Step 7: Commit**

```bash
git add -A && git commit -m "Let the dialog's read-only box be turned off"
```

---

### Task 9: Read-only marks say it only when it is true

**Files:**
- Modify: `src/ui/workspace.rs` (`BarInfo`, `bar_info`, the pills near line 1034, the tags near line 1303, the status line near line 1649)
- Modify: `src/ui/data_view.rs:904-912` and `:980-988`
- Modify: `src/ui/row_panel.rs` (`editing_footer`'s terminal note)
- Test: `src/ui/mod.rs`

The picker draws no read-only mark today; none is added here.

- [ ] **Step 1: Write the failing test**

In the test module of `src/ui/mod.rs`, beside the footer tests (it uses the helpers `the_terminal_footer_fits_a_narrow_row_panel` uses):

```rust
    #[test]
    fn only_a_read_only_connection_carries_the_mark() {
        for look in crate::theme::Look::ALL {
            let mut harness = Harness::new();
            harness.set_look(look);
            let tab = with_page(&mut harness);
            focus_grid(&mut harness, tab);
            harness.click("Row 1");
            let marks = |harness: &mut Harness| -> Vec<String> {
                let tree = harness.settle();
                crate::testing::labels(&tree)
                    .into_iter()
                    .filter(|label| label.to_lowercase().contains("read-only"))
                    .collect()
            };
            // The fixture connection is read-only.
            assert!(!marks(&mut harness).is_empty(), "{}", look.name);
            harness.app.workspace_mut(tab).unwrap().access = tabletist_db::Access::Writable;
            assert_eq!(marks(&mut harness), Vec::<String>::new(), "{}", look.name);
        }
    }
```

The Omarchy status line's tag is painted without an accessible label, so the labels alone do not cover step 4. In the same test, for the Omarchy look, also read `harness.painted` the way `the_terminal_footer_fits_a_narrow_row_panel` does and assert that a piece equal to `read-only` is painted for the read-only connection and none for the writable one.

- [ ] **Step 2: Run and see it fail**

Run: `~/.cargo/bin/cargo test --locked -p tabletist --lib only_a_read_only_connection`
Expected: FAIL on the writable half: the marks are still there.

- [ ] **Step 3: The connection bar**

`BarInfo` gains `/// Whether the bar's own connection opens read-only.` `read_only: bool,` set in `bar_info` from `workspace.access == tabletist_db::Access::ReadOnly`.

The native pills (near line 1034):

```rust
    // Pills after the chips: read-only when it is, then TLS and SSH when
    // remote.
    let mut pills = Vec::new();
    if info.read_only {
        pills.push((
            Some(Icon::Lock),
            gettext(locale, "Read-only").into_owned(),
            palette.secondary,
        ));
    }
```

The terminal tags (near line 1303):

```rust
    // Tags after the chips: read-only when it is, then TLS and SSH when
    // remote.
    let mut tags = Vec::new();
    if info.read_only {
        tags.push((gettext(locale, "read-only").into_owned(), palette.text));
    }
```

- [ ] **Step 4: The Omarchy status line**

Near line 1649 the line ends in the struck-through editing keys and a `read-only` pill. The keys stay (editing does not exist yet); the pill is drawn only for a read-only connection. Take the access from the workspace the function already reads, and:

```rust
            let tag = read_only.then(|| gettext(locale, "read-only"));
            let tag_width = tag.as_ref().map_or(0.0, |tag| measure(tag) + 12.0 + 2.0);
```

Wrap the `rect_stroke` and the `paint_text` of the pill in `if let Some(tag) = &tag { .. }`. With no tag the `gap` before it is not reserved: `limit` subtracts `tag_width + gap` only when there is one.

- [ ] **Step 5: The table footer**

In `data_view.rs` both places build the same text. Replace both with one helper beside `note`:

```rust
/// What the footer says of the selection and of a read-only connection;
/// empty when there is nothing to say.
fn state_note(selected: bool, read_only: bool, locale: crate::i18n::Locale) -> String {
    let mut parts = Vec::new();
    if selected {
        parts.push(gettext(locale, "1 row selected"));
    }
    if read_only {
        parts.push(gettext(locale, "read-only"));
    }
    parts.join(" · ")
}
```

`let state = state_note(selected, read_only, locale);` where `read_only` comes from the tab's workspace (`app.workspace(tab)`, as for the bar). Where `state` is drawn or measured, an empty `state` draws nothing and takes no room: guard the `note(ui, &state, ..)` call and whatever width the first site adds for it.

- [ ] **Step 6: The row panel's note**

In `row_panel.rs`, `editing_footer`'s terminal note says the app's version is read-only, which a writable connection now contradicts. `editing_footer` takes `read_only: bool` from its caller (the panel's workspace), and:

```rust
        let note = if read_only {
            gettext(
                locale,
                "read-only connection · editing arrives in a later version",
            )
        } else {
            gettext(locale, "editing arrives in a later version")
        };
```

`the_terminal_footer_fits_a_narrow_row_panel` (`src/ui/mod.rs`, near line 3972) quotes the old text twice. Its connection is the read-only fixture, so its `note` becomes `"read-only connection · editing arrives in a later version"` and its `starts_with("read-only in 0.1.0")` becomes `starts_with("read-only connection")`. The note is about as long as before, so the test still sees it cut.

- [ ] **Step 7: Run**

Run: `~/.cargo/bin/cargo test --locked -p tabletist --lib`
Expected: PASS. A test that fails because it looked for a read-only mark on a connection it made writable itself (not through `fixture_connection`) is about the mark: give its connection `read_only: Some(true)`. Any other failure is a regression to fix in the code.

- [ ] **Step 8: Commit**

```bash
git add -A && git commit -m "Say read-only only of a connection that is"
```

---

### Task 10: The refused-write card names the reason

**Files:**
- Modify: `src/ui/sql_results.rs` (`Place`, `draw`, `blocked`, its two callers near lines 245 and 1065, the tests near line 1771)

The card keeps the wording it shipped with ("this statement was refused"): it never named the server or the statement, and this plan does not start to.

- [ ] **Step 1: Rewrite the tests**

`a_refused_write_reads_as_a_limit_of_this_version` becomes:

```rust
    #[test]
    fn a_refused_write_says_why_this_connection_refused_it() {
        for look in Look::ALL {
            for writable in [false, true] {
                let (mut harness, tab) = editor(look, "UPDATE users SET email = 'x'");
                let workspace = harness.app.workspace_mut(tab).unwrap();
                // The fixture's session is SQLite's: the code is PostgreSQL's.
                workspace.driver = tabletist_db::Driver::Postgres;
                if writable {
                    workspace.access = tabletist_db::Access::Writable;
                }
                run(&mut harness);
                harness.answer_sql(Ok(script_outcome(vec![read_only_refusal()])), None);
                show_pane(&mut harness, tab, ResultPane::Results);
                let (title, other) = if writable {
                    (
                        "The SQL editor only reads data",
                        "This connection opens read-only",
                    )
                } else {
                    (
                        "This connection opens read-only",
                        "The SQL editor only reads data",
                    )
                };
                assert!(harness.has(&look.label(title)), "{title} in {}", look.name);
                assert!(!harness.has(&look.label(other)), "{other} in {}", look.name);
                assert!(
                    harness.has("25006 · cannot execute UPDATE in a read-only transaction"),
                    "{}",
                    look.name
                );
                // Only a read-only connection has a box to turn off.
                assert_eq!(harness.has("Edit connection"), !writable, "{}", look.name);
                if !writable {
                    harness.click("Edit connection");
                    assert!(
                        matches!(
                            harness.app.dialog,
                            Some(crate::model::Dialog::Connection(_))
                        ),
                        "{}",
                        look.name
                    );
                }
            }
        }
    }
```

Add to the same test, after the loop over `writable`, that the editor's own refusal reads as the editor's on a read-only connection: run a script whose answer is `Err(Error::Refused { line: 1, what: "COMMIT".into() })` (through `harness.answer_sql`, as the tests of a failed run in this module do) and assert the title is `"The SQL editor only reads data"` and there is no `"Edit connection"`. A run that fails as a whole opens the Messages pane, so show the Results pane first (`show_pane(&mut harness, tab, ResultPane::Results)`).

`another_error_keeps_its_own_words_in_the_results` (near line 1845) asserts that "This version only reads data" is absent, a text that no longer exists: it asserts the absence of both new titles instead.

In `a_refused_write_in_a_short_pane_scrolls_to_its_last_line`, the fixture connection is read-only, so `title` becomes `look.label("This connection opens read-only")` and `last` becomes `look.label("To write, turn off Open read-only in the connection.")`.

- [ ] **Step 2: Run and see them fail**

Run: `~/.cargo/bin/cargo test --locked -p tabletist --lib a_refused_write`
Expected: FAIL on the titles.

- [ ] **Step 3: Implement**

`Place` gains the workspace it is drawn for (it is built in `draw` from `app.workspace(tab)`):

```rust
    /// The connection the editor runs on: a refused write says whose
    /// box refused it.
    workspace: &'a crate::model::Workspace,
```

`blocked` takes the place and the actions, `fn blocked(ui: &mut Ui, rect: Rect, error: &Error, place: &Place<'_>, env: &Env<'_>, actions: &mut Vec<Action>)`, and both callers pass them. Its texts:

```rust
            let workspace = place.workspace;
            // The connection's box refused it only when the server did. A
            // statement the editor's own guard refuses (`COMMIT`, a guarded
            // `PRAGMA`) is refused on every connection, and turning the box
            // off would not make it run.
            let read_only = workspace.access == tabletist_db::Access::ReadOnly
                && !matches!(error, Error::Refused { .. });
            let title = env.said(|words| {
                words.say(if read_only {
                    "This connection opens read-only"
                } else {
                    "The SQL editor only reads data"
                })
            });
            let text = env.said(|words| {
                if read_only {
                    // Whose box it is: the name, and the environment when
                    // it has one.
                    // An `if`, not a `match`: environments are matched
                    // only in `env.rs`, and a test there holds us to it.
                    let whose = if workspace.environment == crate::env::Environment::None {
                        workspace.name.clone()
                    } else {
                        format!(
                            "{} · {}",
                            workspace.name,
                            workspace.environment.label(crate::env::Platform::Native)
                        )
                    };
                    format!(
                        "{whose} {}",
                        words.say(
                            "blocks writes, so this statement was refused. Nothing changed."
                        )
                    )
                } else {
                    words.say(
                        "Every query runs in a read-only transaction, so this statement was \
                         refused. Nothing changed.",
                    )
                }
            });
```

The card and the database's own words stay as they are. The last line ("Editing arrives in a later version.") is replaced:

```rust
            // What to do about it. A writable connection has nothing to
            // turn off: its editor only reads, and editing in the grid
            // comes with a later step.
            if read_only {
                let how = env.said(|words| {
                    words.say("To write, turn off Open read-only in the connection.")
                });
                Text::one(look, widgets::secondary(look), &how.painted, palette.secondary)
                    .layout(column.ctx())
                    .label(column);
                let edit = env.said(|words| words.say("Edit connection"));
                if widgets::ButtonSpec::new(&edit.painted)
                    .label(&edit.name)
                    .show(column, 28.0, look, palette)
                    .clicked()
                {
                    actions.push(Action::EditConnection(workspace.conn_id.clone()));
                }
            }
```

Rewrite the doc comment of `blocked`: it no longer describes "a limit of this version".

- [ ] **Step 4: Run**

Run: `~/.cargo/bin/cargo test --locked -p tabletist --lib`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add -A && git commit -m "Tell a refused write whose box refused it"
```

---

### Task 11: The promise, restated, and the full checks

**Files:**
- Modify: `crates/tabletist-db/src/lib.rs` (crate and `Connection` docs), `crates/tabletist-db/src/query.rs:88-90`
- Modify: `docs/superpowers/specs/2026-09-27-tabletist-design.md` (criterion 6, section 4.3, the dialog's line in 5.4)
- Modify: `docs/superpowers/specs/2026-09-30-sql-editor-core-design.md` (Intent, guard layers 2 and 3, the cleanup list)
- Modify: `README.md`

- [ ] **Step 1: The crate's own words**

`lib.rs`, the crate doc:

```rust
//! Access to PostgreSQL, MySQL and SQLite for Tabletist.
//!
//! Nothing in this crate writes to a connected database yet. A session is
//! read-only unless it is opened [`Access::Writable`], and then it is
//! fenced: row fetches run in read-only transactions and a script still
//! cannot write.
```

`Connection`'s doc: "An open database session, and the SSH tunnel it runs through, if any."

`query.rs`, the comment on `raw_where`: replace "Sessions are read-only, so it cannot write" with "It runs inside a read-only transaction (SQLite: under `query_only`), so it cannot write".

- [ ] **Step 2: The main spec**

- Success criterion 6 becomes: "On a read-only connection no action in the app can modify data. On a writable one browsing, a raw WHERE and the SQL editor still cannot (see `2026-10-03-value-editing-core-design.md`)."
- Section 4.3 gets a new first sentence, "A session is opened read-only unless its saved connection is writable (`Access`):", keeps its three bullets as what a read-only session does, and ends with one paragraph on a writable session per driver, taken from the "Sessions" section of the value-editing spec.
- In 5.4, "the read-only note" becomes "the read-only box (on by default for production)".

- [ ] **Step 3: The SQL editor spec**

- Intent: after "The app still cannot change table data", add that this holds on writable connections too, and point to the value-editing spec.
- Guard layer 2: "MySQL's session is already read-only" becomes true only of read-only connections; on a writable one the run sets it first. "SQLite is opened read-only by the driver" becomes "opened read-only, or read-write under `query_only`".
- Guard layer 3: SQLite now has a check (`PRAGMA query_only` before every statement and before the rollback).
- Refusal list: add the SQLite entry (`PRAGMA query_only` and `PRAGMA writable_schema` when set, in every spelling of the name).
- Cleanup: MySQL ends with `SET SESSION TRANSACTION READ WRITE` on a writable session.

Describe the code as built, in the spec's own voice; no em dashes.

- [ ] **Step 4: The README**

In the features list, under the connections bullets, add one line: a connection opens read-only when its box says so (the default for production); a writable one opens a read-write session, in which browsing and the SQL editor still only read. Do not promise editing: it does not exist yet.

- [ ] **Step 5: Every check**

Run, with the database servers up and the two URLs exported:

```bash
~/.cargo/bin/cargo fmt --all --check
~/.cargo/bin/cargo clippy --locked --workspace --all-targets -- -D warnings
~/.cargo/bin/cargo test --locked --workspace --all-targets
RUSTDOCFLAGS='-D warnings' ~/.cargo/bin/cargo doc --locked --workspace --no-deps
```

Expected: all four clean. Say in the report which database suites really ran and which printed "skipped".

- [ ] **Step 6: Commit**

```bash
git add -A && git commit -m "Restate the read-only promise for writable connections"
```

---

## Found in review, built after task 6

The review of tasks 4 to 6 found the SQLite fence short in four places. They were fixed in one change after task 6, and the spec now says so:

- A byte-order mark is whitespace to SQLite and was part of a word to `tokenize`: `<BOM>COMMIT` got past the refusal list. For SQLite the tokenizer now reads it as whitespace.
- The runner did not check that its transaction was still open. `query_only` does not stop `PRAGMA journal_mode = WAL` or `VACUUM INTO`; only the transaction does. `statements` now ends the run with `LeftReadOnly` when the session is in autocommit before a statement.
- A raw WHERE holding `; PRAGMA query_only = 0;` turned the setting off, because rusqlite prepares the tail to find a second statement. `check_raw_where` refuses a `;` token, and `Conn::browse` puts the session's settings back when a fetch or count fails.
- `PRAGMA wal_checkpoint` rewrites a file in WAL mode under `query_only`. It is refused in every form.
- A NUL in a raw WHERE ended the statement for SQLite and dropped the page's ORDER BY, LIMIT and OFFSET. `check_raw_where` refuses it.

The second review showed one more: SQLite's variable tokens (`:a(')`) hide a `;` from `check_raw_where`, and the fix for it in `tokenize` opened another (`$` inside a name) and reached into the editor's highlighting and completion. So the tokenizer is left alone and SQLite's own parse becomes the backstop: task 6b.

---

### Task 6b: The SQLite authorizer

**Files:**
- Modify: `crates/tabletist-db/src/sqlite.rs`
- Test: `crates/tabletist-db/src/sqlite.rs` (unit), `crates/tabletist-db/tests/sqlite.rs`

SQLite calls a connection's authorizer whenever it prepares a statement, after its own parse, the tail rusqlite prepares to detect a second statement included, and before a flag pragma is applied. No spelling gets past it. It needs to know whose text is being prepared:

| Fence | When | Allowed |
|---|---|---|
| `Off` | the app's own statements: `BEGIN`, `ROLLBACK`, `set_session_pragmas`, `still_query_only`, catalog queries | everything |
| `Script` | a script's statement, in `statements` around the call of `statement` | everything but: `Transaction`, `Savepoint`; `Pragma` named `query_only` or `writable_schema` with a value; `Pragma` named `wal_checkpoint` in any form |
| `Filter` | the page query and the count query, which hold the raw WHERE | only `Select`, `Read`, `Function`, `Recursive` |

Writes in a script are NOT denied here: `query_only` refuses them with SQLITE_READONLY (code 8), which the app's refused-write card reads. `ATTACH` and other pragmas stay allowed in a script, as the SQL editor spec says.

- [ ] **Step 1: The policy, test first**

A pure function and its test (rusqlite's `hooks` feature is already on; the types are `rusqlite::hooks::{AuthAction, AuthContext, Authorization, TransactionOperation}`):

```rust
/// Whose text SQLite is preparing, which decides what it may do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
enum Fence {
    /// The app's own statements.
    Off = 0,
    /// A statement of a SQL editor script.
    Script = 1,
    /// A table's page or count, which holds the raw WHERE.
    Filter = 2,
}

/// What SQLite may do for text behind `fence`. Asked after SQLite's own
/// parse, so no spelling gets past it; that is why it does not go through
/// `sql::refusal`, whose tokenizer and SQLite's do not always agree.
fn authorize(fence: Fence, action: &AuthAction<'_>) -> Authorization {
    let allowed = match fence {
        Fence::Off => true,
        // A filter only reads.
        Fence::Filter => matches!(
            action,
            AuthAction::Select
                | AuthAction::Read { .. }
                | AuthAction::Function { .. }
                | AuthAction::Recursive
        ),
        // A write is left to `query_only`, whose error the app knows as a
        // refused write. What must not happen is the script leaving its
        // transaction or lifting what refuses the write.
        Fence::Script => match action {
            AuthAction::Transaction { .. } | AuthAction::Savepoint { .. } => false,
            AuthAction::Pragma {
                pragma_name,
                pragma_value,
            } => {
                let name = pragma_name.to_ascii_lowercase();
                name != "wal_checkpoint"
                    && !(pragma_value.is_some()
                        && matches!(name.as_str(), "query_only" | "writable_schema"))
            }
            _ => true,
        },
    };
    if allowed {
        Authorization::Allow
    } else {
        Authorization::Deny
    }
}
```

The test walks a table of (fence, action, allowed): `Off` allows a pragma with a value and a transaction; `Script` denies `Transaction`, `Savepoint`, `query_only`/`QUERY_ONLY`/`writable_schema` with a value, `wal_checkpoint` with and without one, and allows `query_only` without a value, `foreign_keys` with one, `table_info` with one, `Select`, `Update`, `Attach`; `Filter` allows `Select`, `Read`, `Function`, `Recursive` and denies `Pragma` (with and without a value), `Attach`, `Transaction`, `Update`, `Insert`, `Delete`.

- [ ] **Step 2: The state and the install**

```rust
/// Which fence is up, shared with the authorizer SQLite calls.
#[derive(Clone, Default)]
struct Guard(Arc<AtomicU8>);

impl Guard {
    /// Puts `fence` up until the returned value drops.
    fn fence(&self, fence: Fence) -> Fenced<'_> {
        self.0.store(fence as u8, Ordering::SeqCst);
        Fenced(self)
    }

    fn current(&self) -> Fence {
        match self.0.load(Ordering::SeqCst) {
            1 => Fence::Script,
            2 => Fence::Filter,
            _ => Fence::Off,
        }
    }
}

/// Takes the fence down when dropped, on every path.
struct Fenced<'a>(&'a Guard);

impl Drop for Fenced<'_> {
    fn drop(&mut self) {
        self.0.0.store(Fence::Off as u8, Ordering::SeqCst);
    }
}
```

`Conn` gains `guard: Guard`. In `open`, after `set_session_pragmas` and the first read succeeded:

```rust
            let guard = Guard::default();
            let asked = guard.clone();
            connection.authorizer(Some(move |context: AuthContext<'_>| {
                authorize(asked.current(), &context.action)
            }));
```

(the blocking closure returns the connection and the guard together).

- [ ] **Step 3: The failing tests for the fences**

In the test module of `sqlite.rs`, through `Conn::run_script` directly (which skips `sql::refusal`), on both accesses. Each denied text is a statement error, not a closed session: the run is `Ok`, its last result is `StatementOutcome::Error` whose message holds "not authorized", `PRAGMA query_only` reads 1 afterwards and the connection is in autocommit again.

- Denied: `PRAGMA query_only = OFF`, `PRAGMA 'query_only' = 0`, `EXPLAIN PRAGMA query_only = 0`, `PRAGMA main.query_only(0)`, `PRAGMA writable_schema = ON`, `PRAGMA wal_checkpoint`, `PRAGMA wal_checkpoint(TRUNCATE)`, `COMMIT`, `END`, `ROLLBACK`, `SAVEPOINT s`, `"\u{feff}COMMIT"`, and one text holding two statements whose second is hidden from our tokenizer: `SELECT :a('); PRAGMA query_only = 0; --'`.
- Allowed, each `Rows` or `Done`: `PRAGMA query_only`, `PRAGMA table_info(users)`, `SELECT name FROM pragma_table_info('users')`, `PRAGMA foreign_keys = ON`, `SELECT count(*) FROM users`.
- A write is still `query_only`'s to refuse: `UPDATE users SET email = 'x'` fails with code `8`, not "not authorized".

In `crates/tabletist-db/tests/sqlite.rs`, in `a_raw_where_cannot_modify_data`, for both accesses: read `PRAGMA foreign_keys` and `PRAGMA synchronous` through a script, then for each of these raw WHERE texts call `fetch_rows` and `count_rows` (each must fail), and read the two pragmas again: they must equal what they were.

    1=1 OR :a(') IS NULL); PRAGMA foreign_keys = 0; PRAGMA synchronous = 0; SELECT ('
    1=1 OR $a(') IS NULL); PRAGMA foreign_keys = 0; SELECT ('
    1=1 OR @a(") IS NULL); PRAGMA foreign_keys = 0; SELECT ("
    1=1 OR #a(--) IS NULL); PRAGMA foreign_keys = 0; SELECT (1
    1=1 OR :a(/*) IS NULL); PRAGMA foreign_keys = 0; SELECT (1 /* */
    1=1 OR EXISTS (WITH a$b(')') AS (SELECT 1) SELECT 1 FROM a$b)); PRAGMA foreign_keys = 0; SELECT ('

(`foreign_keys` must first be made different from what the texts set, in a way the test can do: if the fixture's session has it off already, the texts set it to 1 instead.) And a filter that only reads still works under the fence: `id IN (SELECT id FROM users WHERE id < 3)` returns two rows.

Run them: the denied script texts FAIL (the pragmas are applied or the run ends `LeftReadOnly`), and the variable-token WHEREs FAIL (the pragma reads changed).

- [ ] **Step 4: Put the fences up**

- `script`, `statements` and `end_transaction` are free functions over `&rusqlite::Connection`: `script` and `statements` take `guard: &Guard`, and `statements` wraps only the user's statement:

```rust
        let result = {
            let _fenced = guard.fence(Fence::Script);
            statement(connection, text, limit)
        };
```

  `run_script` clones `self.guard` into its blocking closure.
- `fetch_rows` and `count_rows`: the fence goes up, as `Fence::Filter`, around the prepare and the stepping of the page query and of the count query only. `check_raw_where` and the lookups of the key and of the binary columns before them run unfenced: they are the app's own and use `pragma_table_xinfo`.
- Nothing else changes mode: `BEGIN`, `ROLLBACK`, `set_session_pragmas`, `still_query_only`, the catalog and `server_version` run with the fence off.

- [ ] **Step 5: The tests of the layers behind it**

Three existing tests reach the runner's own checks by running `PRAGMA query_only = OFF` or `COMMIT` through `Conn::run_script`: `a_script_that_gets_query_only_off_is_stopped_and_writes_nothing`, `a_script_that_ended_its_transaction_runs_nothing_after_it`, `a_statement_after_query_only_went_off_does_not_run`. The authorizer now stops those texts first. The checks stay as the layer behind it, and their tests test them alone: each takes the authorizer off its connection first, through a helper:

```rust
    /// Takes the authorizer off, for a test of the checks behind it.
    async fn without_the_authorizer(conn: &Conn) {
        conn.run(|connection| {
            connection.authorizer(None::<fn(AuthContext<'_>) -> Authorization>);
            Ok(())
        })
        .await
        .unwrap();
    }
```

`failed_browsing_puts_the_session_settings_back` does the same if it needs to. No other existing test may change. If one fails, stop and report it: in particular browsing a view, a filter with a function or a subquery, `PRAGMA table_info` in a script, and the refused-write code `8`.

- [ ] **Step 6: Run**

    ~/.cargo/bin/cargo test --locked -p tabletist-db --test sqlite   (three times)
    ~/.cargo/bin/cargo test --locked -p tabletist-db --lib           (three times)
    ~/.cargo/bin/cargo fmt --all --check
    ~/.cargo/bin/cargo clippy --locked --workspace --all-targets -- -D warnings
    ~/.cargo/bin/cargo test --locked --workspace --all-targets

Expected: PASS, the app's suite included (its demo and fixtures browse SQLite through this code).

- [ ] **Step 7: Commit**

```bash
git add -A && git commit -m "Let SQLite itself refuse what a script and a filter may not do"
```

---

## What this plan leaves for step 2

- A script's other SQLite pragmas (`journal_mode`, `synchronous`, `foreign_keys`) and its `ATTACH`es still last for the session, as the SQL editor spec says. Harmless while nothing writes; `Connection::write` must put its own house in order before it writes on a handle a script has used.
- `mysql_async` opens a read-only transaction as `SET TRANSACTION READ ONLY` then `START TRANSACTION`. A cancel between the two can leave "next transaction read-only" pending on the session, which `Connection::write` must not inherit.
- `Connection::write`, the statement builder, `ColumnInfo.generated`, `IndexInfo.partial` and the row key rule.

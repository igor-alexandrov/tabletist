# Connect until the tables show — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use rubypowers:subagent-driven-development (recommended) or rubypowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** From a connect to the default schema's tables in the sidebar takes 6 round trips on PostgreSQL (was 8) and 4.5 on MySQL and MariaDB (was 8.5): at a 200 ms round trip, 1.62 s → 1.2 s and 1.72 s → 0.9 s, TLS adding what it adds today.

**Architecture:** Two runs, a PR each. Run 1 asks the server version (PostgreSQL) and the session's time zone (MySQL) in the message each connect already sends after sign-in, and keeps them on the connection, so asking them later costs no round trip; MySQL's schema listing goes through the text protocol, one round trip instead of a prepare and an execute. Run 2 adds one backend command that lists the schemas and the tables of the schema the sidebar will unfold, in one flight (PostgreSQL) or one message (MySQL), as `Command::Open` did for a table's structure and page.

**Tech Stack:** Rust, tokio-postgres 0.7.18 (pipelined queries, `simple_query`), mysql_async 0.37 (multi-statement text messages through `answered_rows` / `script::answers`), the test PostgreSQL 17 on 55432, MySQL 8.4 on 53306, MariaDB 11.4 on 53307.

---

## Where the round trips go

Measured on main 46c99dbc through a delay proxy of 100 ms each way that includes the TCP handshake (throwaway `action_bench.rs` and `tcpdelay.py`), TLS off:

| Step, in the app's order | PostgreSQL | MySQL / MariaDB |
|---|---|---|
| Connect (TCP, sign-in, the settings message) | 5 | 3.5 |
| `ServerVersion` | 1 (`SHOW server_version`) | 0 (known from the handshake) |
| `SessionZone` | 0 (none on PostgreSQL) | 2 (a prepared `SELECT`: prepare, execute) |
| `ListSchemas` | 1 | 2 (a prepared `SELECT`) |
| `ListObjects` of the default schema, after the schemas answer | 1 | 1 |
| **Total** | **8** | **8.5** |

`App::after_connect` (`src/app.rs`) sends `ServerVersion`, `SessionZone`, then `refresh_tree` sends `ListSchemas`; the `Event::Schemas` handler picks the default schema with `model::first_schema` (`crates/tabletist-core/src/model.rs:1515`: the schema named like the database, else `public` or `main`, else the only one) and `expand_schema` sends its `ListObjects`. The session runs one command at a time, so each is a round trip of its own. PostgreSQL's database listing comes after and does not hold the session (`backend.rs` `list_databases` answers from a spawned task).

### After

- **Run 1:** PostgreSQL 7 (version in the settings message), MySQL 5.5 (zone in the identity message, schemas one round trip).
- **Run 2:** PostgreSQL 6, MySQL 4.5 (schemas and the default schema's tables together).

### Decisions

- **The server version on PostgreSQL** comes from `SHOW server_version` added to the settings `session_setup` already sends after sign-in, read with `simple_query` in place of `batch_execute`. Not from the startup parameter status: tokio-postgres 0.7 does not expose it. Not as a startup `options` parameter either: PgBouncer refuses unknown startup parameters.
- **The session zone on MySQL** is one more column of the identity `SELECT` that `server_identity` sends with the settings at connect (`SELECT @@hostname, @@port, VERSION(), <zone>`), before the `@@server_uuid` question that MariaDB refuses, so the refusal loses nothing. It is kept on `Conn`. A script cannot change it for good: every run that sets anything is followed by a reset to the connect's settings.
- **MySQL's schema listing** is fixed SQL with no parameters: the text protocol is safe for it, and it is one round trip where the prepared one is two on a fresh session.
- **One command, not two in a row** (as for `Command::Open`): `Command::ListTree { session, schemas: RequestId, objects: RequestId, schema: String }`. The app guesses the schema before the schemas answer, the way `first_schema` would pick it: the database's name on MySQL, `public` on PostgreSQL, `main` on SQLite. A wrong guess (a PostgreSQL database with a schema named like itself, or none called `public`) costs what it costs today: `expand_schema` then lists the right one, one round trip more, and the guessed listing stays in the tree's cache for when that schema is unfolded.
- **Cancels and skips** of a `ListTree` work as for `Open`: a Cancel naming either id stops the command; the other part answers with its own result, or is run again alone if the stop cancelled it; a skip of one id runs the other part as a plain `ListSchemas` or `ListObjects`. `take_part` and `requests_of` already split two-id commands; `ListTree` joins them.
- **SQLite and BigQuery** get a default `Adapter::list_tree`: `list_schemas`, then `list_objects`.
- **Nothing read-only changes:** the same catalog queries run in the same read-only paths; only their order and grouping change.

## Run 1: no round trip for the version and the zone, one for MySQL's schemas

### Task 1: PostgreSQL knows its version from the connect

**Files:** `crates/tabletist-db/src/pg.rs` (code and `pg::tests`)

- [ ] **Step 1: Write the failing test**, next to `opening_a_table_asks_the_server_once`:

```rust
    /// The server's version is read with the settings the connect sends
    /// after sign-in: asking it later asks the server nothing.
    #[tokio::test]
    async fn the_server_version_is_known_from_the_connect() {
        let Some(url) = test_url() else {
            return;
        };
        let (conn, trips) = trip_counted(&url).await;
        let (version, asked) = counted(&trips, conn.server_version()).await;
        let version = version.unwrap();
        assert!(version.starts_with("PostgreSQL "), "{version}");
        assert!(!version.contains('('), "{version}");
        assert_eq!(asked, 0);
    }
```

- [ ] **Step 2: Run it and see it fail** (`asked` is 1):

```bash
CARGO_TARGET_DIR=$PWD/target TABLETIST_TEST_PG_URL=postgres://tabletist:tabletist@localhost:55432/tabletist ~/.cargo/bin/cargo test -p tabletist-db --lib the_server_version_is_known_from_the_connect
```

- [ ] **Step 3: Implement.** `session_setup` stays as it is: `pg/script/write.rs` sends it again after a script that writes, and needs no version. Only `Conn::open` adds the `SHOW` to it, sends it with `simple_query` in place of `batch_execute`, and keeps the version:

```rust
        // The server's version with the settings: one round trip for both.
        let setup = format!("{}; SHOW server_version", session_setup(access));
        let messages = client
            .simple_query(&setup)
            .await
            .map_err(query_error)?;
        let version = version_named(first_text(&messages).unwrap_or_default());
```

with a field `version: String` on `Conn`, set in `open`, and:

```rust
/// "17.2 (Debian 17.2-1.pgdg120+1)" reads as "PostgreSQL 17.2".
fn version_named(shown: &str) -> String {
    let version = shown.split_whitespace().next().unwrap_or_default();
    format!("PostgreSQL {version}")
}

    /// See [`crate::Connection::server_version`]. Read at connect, with the
    /// session's settings.
    async fn server_version(&self) -> Result<String> {
        Ok(self.version.clone())
    }
```

`server_version` in the `Adapter` impl then returns `self.version.clone()`, as above, and the `SHOW server_version` it sent goes. A test of `version_named`: `"17.2 (Debian 17.2-1.pgdg120+1)"` → `"PostgreSQL 17.2"`, `""` → `"PostgreSQL "` as today.

- [ ] **Step 4: Run it and see it pass**, then `pg::` and `--test postgres` with the PostgreSQL URL: all pass.
- [ ] **Step 5: Commit:** `git commit -m "Read the PostgreSQL server version with the connect's settings"`

### Task 2: MySQL knows its session zone from the connect

**Files:** `crates/tabletist-db/src/mysql.rs` (code and `mysql::tests`)

- [ ] **Step 1: Write the failing test** in `mysql::tests`:

```rust
    /// The session's zone is read with the server's identity at connect:
    /// asking it later sends nothing, and it is what the server says.
    #[tokio::test]
    async fn the_session_zone_is_known_from_the_connect() {
        use std::sync::atomic::Ordering;
        let Some(url) = test_url() else {
            return;
        };
        for access in [Access::ReadOnly, Access::Writable] {
            let (conn, asked) = counted_session(&url, access).await;
            let before = asked.load(Ordering::SeqCst);
            let zone = conn.session_zone().await.unwrap();
            assert_eq!(asked.load(Ordering::SeqCst) - before, 0, "{access:?}");
            let opts = Opts::from_url(&format!("{url}?prefer_socket=false")).unwrap();
            let mut admin = mysql_async::Conn::new(opts).await.unwrap();
            let asked_alone: Option<String> = admin
                .query_first(
                    "SELECT IF(@@session.time_zone = 'SYSTEM', @@global.system_time_zone, \
                     @@session.time_zone)",
                )
                .await
                .unwrap();
            assert_eq!(zone, asked_alone, "{access:?}");
        }
    }
```

- [ ] **Step 2: Run it and see it fail** (2 messages), on MySQL (53306).
- [ ] **Step 3: Implement.** Move the zone's expression into a constant, ask it in `server_identity`'s first `SELECT`, and keep it on `Conn`:

```rust
/// The session's zone, or the server's own where the session follows it.
const SESSION_ZONE: &str =
    "IF(@@session.time_zone = 'SYSTEM', @@global.system_time_zone, @@session.time_zone)";
```

`server_identity` returns `(server, version, zone)`: the first `SELECT` becomes `SELECT @@hostname, @@port, VERSION(), {SESSION_ZONE}`, read as `(String, u16, String, Option<String>)`; the `@@server_uuid` question stays last. `Conn` gets `zone: Option<String>`, and:

```rust
    /// See [`crate::Connection::session_zone`]. Read at connect, with the
    /// server's identity: every run that changes a setting is followed by
    /// a reset to the connect's settings, so it holds.
    async fn session_zone(&self) -> Result<Option<String>> {
        Ok(self.zone.clone())
    }
```

`server_identity` has other callers: `open_checked` (the cancel spare's check that it reached the same server, `mysql.rs` ~3057) and two tests (~6548). They keep comparing `(server, version)` and ignore the zone; the tests' destructuring gains a `_`.

- [ ] **Step 4: Run the test on MySQL and on MariaDB (53307), then `--lib mysql::` and `--test mysql` on both.**
- [ ] **Step 5: Commit:** `git commit -m "Read the MySQL session zone with the server's identity at connect"`

### Task 3: MySQL lists its schemas in one round trip

**Files:** `crates/tabletist-db/src/mysql.rs`

- [ ] **Step 1: Write the failing test:**

```rust
    /// The schemas are listed in one message on a fresh session: fixed
    /// SQL, sent as text, with nothing to prepare first.
    #[tokio::test]
    async fn listing_the_schemas_asks_the_server_once() {
        use std::sync::atomic::Ordering;
        let Some(url) = test_url() else {
            return;
        };
        let (conn, asked) = counted_session(&url, Access::ReadOnly).await;
        let before = asked.load(Ordering::SeqCst);
        let schemas = conn.list_schemas().await.unwrap();
        assert_eq!(asked.load(Ordering::SeqCst) - before, 1);
        assert!(schemas.iter().any(|schema| schema == "tabletist"), "{schemas:?}");
    }
```

- [ ] **Step 2: Run it and see it fail** (2 messages).
- [ ] **Step 3: Implement:** `list_schemas` sends its `SELECT` with the text protocol (`conn.query`), through the same session lock and error mapping as `catalog`. Its order (`CAST(schema_name AS BINARY)`) and `LIMIT` stay.
- [ ] **Step 4: Run it on MySQL and MariaDB, then `--lib mysql::` and `--test mysql` on both.**
- [ ] **Step 5: Commit:** `git commit -m "List the MySQL schemas in one round trip"`

### Task 4: check, measure and ship run 1

- [ ] `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --locked --workspace --no-fail-fast` with the PostgreSQL and MySQL URLs, and the MySQL suites again on MariaDB.
- [ ] Measure with the throwaway `action_bench.rs` `connects` through `tcpdelay.py` (never committed): connect until tables expected 1.42 s on PostgreSQL, 1.12 s on MySQL and MariaDB.
- [ ] PR against main.

## Run 2: the schemas and the default schema's tables in one round trip

### Task 5: `Connection::list_tree`

**Files:** `crates/tabletist-db/src/adapter.rs`, `crates/tabletist-db/src/lib.rs`, `crates/tabletist-db/src/pg.rs`, `crates/tabletist-db/src/mysql.rs`

- [ ] **Red tests** in `pg::tests` and `mysql::tests` (MySQL in both access modes): a fresh counted session, `conn.list_tree("public")` (MySQL: the test database's name): 1 round trip / 1 message; the schemas are `list_schemas`'s, the objects are `list_objects`'s for that schema, and each listed table's shape is remembered (a following `open` is 1 trip). A schema that does not exist: the schemas answer, the objects answer as `list_objects` does for it today.
- [ ] `Adapter::list_tree(&self, schema: &str) -> (Result<Vec<String>>, Result<Vec<ObjectInfo>>)` with a default of `list_schemas`, then `list_objects`; `Connection::list_tree` dispatches it.
- [ ] PostgreSQL: the schemas' query goes in the same pipelined flight as `listed_with`'s two (one client lock held across the three; `try_join!(biased; …)` keeps the first error the query's own, as `structure_reads` does). A failed listing part does not fail the schemas, and the reverse.
- [ ] MySQL: the schemas' `SELECT` is the first statement of the listing's one message (`listed_with`); a refusal of a later statement leaves the schemas answered.
- [ ] Commit: `git commit -m "List the schemas and a schema's tables in one round trip"`

### Task 6: `Command::ListTree` in the backend and the app

**Files:** `crates/tabletist-core/src/backend.rs`, `src/app.rs`, `src/app/startup_tests.rs`, `src/testing.rs`

- [ ] `Command::ListTree { session, schemas: RequestId, objects: RequestId, schema: String }`, in every match `ListSchemas` and `ListObjects` are in (`session_of`, `requests_of`, `fail`, `skip`, `take_part`, `running.read`, `src/promo/live.rs`). It answers `Event::Schemas`, then `Event::Objects`, as two commands would.
- [ ] Cancels and skips as `Open`'s (see Decisions); backend tests for both answers, a cancel of each id, a skip of each id, and a cancel while the session connects.
- [ ] `App::refresh_tree` sends `ListTree` with the guessed schema (`model::guessed_schema(driver, database)`, new, next to `first_schema`, with unit tests: MySQL → the database, PostgreSQL → `public`, SQLite → `main`, an empty MySQL database → none, which sends `ListSchemas` alone as today). The guessed node's `objects` starts with the `objects` id; the `Event::Schemas` handler's `expand_schema` then finds it loading and sends nothing. Expanded schemas other than the guess are listed as today.
- [ ] App tests: after a connect, one `ListTree` (no `ListSchemas`, no `ListObjects` for the default schema); a wrong guess (schemas without `public`) sends the right schema's `ListObjects` after the schemas answer; a refresh with two schemas unfolded sends `ListTree` and one `ListObjects`.
- [ ] Commit: `git commit -m "Ask for the schemas and the default schema's tables in one command"`

### Task 7: check, measure and ship run 2

- [ ] The same checks as Task 4.
- [ ] `action_bench.rs` `connects`, its schemas and tables now one `list_tree`: connect until tables expected 1.21 s on PostgreSQL, 0.91 s on MySQL and MariaDB.
- [ ] PR against main.

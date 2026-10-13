# MCP server, Phase 1a, run 2: an agent's read on the database. Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** `Connection::run_agent_read` runs a statement that passed `agent::check_read` on PostgreSQL, MySQL and SQLite: its values bound by the database's own protocol, in a read-only transaction that is rolled back, ended by the database itself when it takes too long.

**Architecture:** The run is the read-only script run the SQL editor has, of one statement, with two additions each driver makes in its own way: bound values, and the database's own statement timeout. The SQL editor's path takes `None` for both and does what it did. A Bookshop fixture for the three databases is what the run is tested on. Nothing in the app calls it yet.

**Tech Stack:** Rust 2024, `tokio-postgres` 0.7, `mysql_async` 0.37, `rusqlite` 0.37. No new crate.

---

## Where this sits

This is the second run of Phase 1a of `docs/superpowers/plans/2026-10-13-mcp-server.md` (section G). Phase 1a is the enforcement core in `tabletist-db`, with no server and nothing visible.

1. **The read checker.** Done: `2026-10-13-mcp-server-1a-1-read-checker.md`, merged as `d376052`.
2. **This run: the drivers.** The Bookshop fixture, values and the database's timeout in the read-only run, `Connection::run_agent_read`, `tests/agent.rs` on real databases, its line in `ci.yml`.
3. **SQLite's `Fence::Agent`.** The authorizer as the engine's own reading of what an agent's statement touches.

Run 1's plan listed one more thing for this run, the size cap on an answer. It is not here. See "What this run leaves out".

## Before you start

- Work on a branch of `main`. One commit for each task, in the order below. Every commit passes `cargo fmt --all --check`, `cargo clippy --locked --workspace --all-targets -- -D warnings` and `cargo test --locked --workspace --all-targets`.
- On Igor's machine `cargo` is `~/.cargo/bin/cargo` (the mise shim fails), and a build in a worktree needs `CARGO_TARGET_DIR` set to that worktree's `target/`.
- Read `AGENTS.md`. Two of its rules shape this code: a `match` on `Dialect` names every variant, and there are no em dashes in comments or messages.
- The diffs below apply with `git apply`. Run `cargo fmt --all` before each commit: the code here is already formatted, so it should change nothing.
- **The servers.** Tasks 1, 5 and 6 have tests that need PostgreSQL and MySQL. Without the two variables those tests print "skipped" and pass, and a step that says "expected: FAIL" shows nothing. Set them:

  ```bash
  export TABLETIST_TEST_PG_URL=postgres://tabletist:tabletist@localhost:55432/tabletist
  export TABLETIST_TEST_MYSQL_URL=mysql://tabletist:tabletist@localhost:53306/tabletist
  ```

  A session cannot start the containers (the Docker socket is refused), but on 2026-10-13 both were up on Igor's machine and reachable on those ports. Check with `ss -ltn | grep -E '55432|53306'` before deciding that a step cannot be run.
- **MySQL needs a `bookshop` database**, which Task 1 adds to `compose/mysql-init.sql`. That file runs on a new volume only. On a server that was started before it, run its last two statements once as root (the password is in `compose.yaml`), or make the container again. On Igor's machine this was done by hand on 2026-10-13, while this plan was drafted: the database is there.

## What was run before this plan was written

Every task's state was built, passed `cargo fmt --check` and clippy with `-D warnings`, and its tests failed and then passed as each step says. Unlike run 1, the PostgreSQL and MySQL tests ran too, on PostgreSQL 17.11 and MySQL 8.4.11. Three things were not run:

- **MariaDB.** There is none here and none in CI. Its spelling of the time limit (`max_statement_time`, in seconds) is written from its documentation, and reached only when the server does not know MySQL's setting. Its unit test checks the text of the statement, not a server's answer.
- **macOS and Windows.** The CI matrix runs the SQLite tests there. Nothing in this run is specific to a platform.
- **A server that someone else is using.** The tests load the Bookshop once for a test binary. Two runs of them at once against one server can trip over each other, as the existing suites can.

## What this run settles

| Question | Decision | Why |
|---|---|---|
| How a value reaches PostgreSQL | Bound, in text format, where the cursor is declared. Never written into the statement | The master plan (B.2) had each `$n` replaced by a quoted literal, because rows are read as text through the simple protocol and tokio-postgres's other protocol asks for binary results. It does not have to be either: a value may choose its own format (`ToSql::encode_format`), and `DECLARE … CURSOR FOR` takes bound parameters. So the values go by the extended protocol on the `DECLARE`, and the rows still come as text through `FETCH`. No text is rewritten, so there is no second reading of the statement to get wrong. The master plan is corrected in this pull request |
| One kind of value for a placeholder of any type | PostgreSQL reads each value as the type it gave the placeholder, as it reads a quoted literal | An agent sends JSON: a number, a string, a boolean, null. Sent as text, a number fills an `integer`, a `numeric` or a `text`, and `{1,2,3}` fills an array. A value its type does not take is the server's error (22P02), and the agent can read it |
| Whose count of placeholders holds | The database's, from its prepare step. When it is not the number of values that came, nothing runs | The check counted them too, by its own reading of the text. This is the count that cannot be wrong |
| PostgreSQL's timeout | `SET LOCAL statement_timeout`, sent before the guard's savepoint. No `lock_timeout` | It is the transaction's, so the `ROLLBACK` that ends every run takes it away. PostgreSQL's manual calls a `lock_timeout` of the same size pointless: the statement's time includes the wait for a lock. A test holds a table under `ACCESS EXCLUSIVE` and sees the read behind it ended |
| MySQL's timeout | `SET SESSION max_execution_time`, and where the server does not know that setting (error 1193) MariaDB's `max_statement_time`. No `lock_wait_timeout` | The reset that ends every run puts it back to 0. MySQL holds only a `SELECT` to it, which is all a read is: a `WITH`, a `UNION`, a derived table, a subquery in the list and in the `WHERE`, and an `EXPLAIN` were each ended by it. So was a read waiting behind another session's `LOCK TABLES … WRITE`, which is why the wait for a lock has no limit of its own here |
| SQLite's timeout | A deadline in the progress handler the run already installs | SQLite has no timeout of its own. The handler is taken off before the run's cleanup, as before. It is not asked while SQLite waits for another program's lock on the file: that wait is the session's busy timeout, five seconds, for an agent as for the SQL editor |
| Telling a timeout from a stop | A statement that was ended while nobody stopped the run is `Error::Timeout`. So are MySQL's error 3024 and MariaDB's 1969 | PostgreSQL ends a statement over its time with the SQLSTATE of a cancel (57014) and words that change with the server's language. The run's `StopFlag` is what the app sets when it cancels, so its absence is what tells them apart |
| MySQL's optimizer hints | Refused by the check: `a /*+ */ optimizer hint is not available to agents.` | Seen on MySQL 8.4: `/*+ MAX_EXECUTION_TIME(n) */` and `/*+ SET_VAR(max_execution_time = n) */` replace the session's limit for the statement, and `/*+ SET_VAR(sql_select_limit = n) */` lifts the row limit. To PostgreSQL and SQLite the same text is a comment |
| An `EXPLAIN` with values on PostgreSQL | Runs by the extended protocol, its rows read as text | It is no cursor, so its values cannot ride on a `DECLARE`. A plan is text, JSON or XML, each of which the server sends as it is written whatever the protocol. Anything else is an error, not a guess |
| What a `Read` carries | The dialect it was checked for, and the values it was checked with | So that what runs is what was checked. `run_agent_read` takes a `Read` and nothing beside it, and refuses one checked for another dialect |
| What the run answers | `agent::Answer`: columns, rows, `truncated`, `elapsed`. Or the crate's `Error`: `Timeout`, `Cancelled`, the statement's own, or one that closes the session | The same error type every other call of `Connection` has. Phase 1b maps it to the refusal codes of B.8 |
| Where the Bookshop lives | In a database of its own named `bookshop`, on each server. SQLite's is a file | Its `orders` is not the driver fixture's `orders`, and the existing suites drop and make theirs. In a database of its own the statements are the same text on all three |

### What this run leaves out

- **The size cap on an answer** (a cell cut at 2,000 characters, binary described, 256 kB in all). The master plan put it in Phase 1a. Its measure is the answer as JSON, and the JSON is written in Phase 1b. A cap in this crate would measure a guess at that JSON. It moves to 1b, beside the code that writes the answer, and the master plan now says so. What that leaves here: a cell comes back whole, as it does for the SQL editor, and the row limit is the only bound.
- **"N of M".** The count of a cut answer is a second statement in the same transaction. Phase 1b, with the tool that words it.
- **A function in the fixture.** The master plan lists one that reads `customers`, for the hidden-table tests. Those are Phase 1c, which also reads the catalog's routines. Making one on MySQL needs `log_bin_trust_function_creators`, which `compose.yaml` sets and which a container started before that line was added does not have. Igor's does not.
- **`Fence::Agent`.** Run 3. This run's SQLite statements run behind `Fence::Script`, as the SQL editor's do.
- **The backend's own timer** behind the database's timeout. Phase 1b: it belongs to the code that calls the run.

### What does not change

The SQL editor's run. `Connection::run_script` takes no values and sets no timeout, and each driver's shared functions take `None` where an agent's run gives them. The existing script suites are the check: `tests/postgres.rs`, `tests/mysql.rs`, `tests/sqlite.rs` and the drivers' own unit tests pass unchanged, except for the `None` an existing test now passes to a function that gained an argument.

## File structure

| File | What changes |
|---|---|
| `crates/tabletist-db/fixtures/bookshop/{sqlite,postgres,mysql}.sql` | New. The Bookshop: seven tables and two views, the same rows in each |
| `crates/tabletist-db/src/fixtures.rs` | The three scripts as constants, and `write_sqlite_bookshop` |
| `compose/mysql-init.sql` | Makes the `bookshop` database and lets the tests' user into it |
| `crates/tabletist-db/src/agent.rs` | `Read` keeps its dialect and its values. MySQL's hints are refused. Declares `agent::run` |
| `crates/tabletist-db/src/agent/run.rs` | New. `Limits`, `Answer`, and what the drivers share: `Sent`, `count_mismatch`, `answer` |
| `crates/tabletist-db/src/adapter.rs` | `Adapter::run_read` |
| `crates/tabletist-db/src/lib.rs` | `Connection::run_agent_read` |
| `crates/tabletist-db/src/sqlite.rs` | Values bound, and the deadline |
| `crates/tabletist-db/src/pg.rs`, `pg/script.rs`, `pg/script/write.rs` | Values bound on the `DECLARE`, the plan's rows, `statement_timeout` |
| `crates/tabletist-db/src/mysql.rs`, `mysql/script.rs`, `mysql/script/write.rs` | Values bound, the time limit |
| `crates/tabletist-db/tests/agent.rs` | New. What an agent's read does on each engine |
| `.github/workflows/ci.yml` | The PostgreSQL and MySQL jobs run `tests/agent.rs` |

`agent/run.rs` is beside `agent.rs` as `pg/script.rs` is beside `pg.rs`. The checker stays one file.

---

### Task 1: The Bookshop

The made-up shop of the app's screenshots, as tables: `publishers`, `authors`, `books`, `book_covers`, `customers`, `orders`, `order_items` (whose key to `orders` cascades), and two views, `recent_orders` over `orders` and `customer_names` over `customers`. The views are for Phase 1c: two ways to a table's rows that do not name it. The three files hold the same rows.

On PostgreSQL the tests make a database named `bookshop` themselves: their user may. On MySQL it may not, so `compose/mysql-init.sql` makes it.

**Files:**
- Create: `crates/tabletist-db/fixtures/bookshop/sqlite.sql`, `postgres.sql`, `mysql.sql`
- Modify: `crates/tabletist-db/src/fixtures.rs`
- Modify: `compose/mysql-init.sql`
- Test: `crates/tabletist-db/tests/agent.rs` (new), and `fixtures.rs`'s own

- [ ] **Step 1: Write the failing tests**

`tests/agent.rs` starts with what every later task needs: a session on the Bookshop for each engine, and `the_bookshop_is_stocked`, which later tests call before and after a read to show that nothing changed. It counts through `run_script`, the SQL editor's call, so it does not lean on what is being tested.

```diff
diff --git a/crates/tabletist-db/src/fixtures.rs b/crates/tabletist-db/src/fixtures.rs
index 1195375..275cd71 100644
--- a/crates/tabletist-db/src/fixtures.rs
+++ b/crates/tabletist-db/src/fixtures.rs
@@ -28,3 +28,31 @@ pub const POSTGRES_SQL: &str = include_str!("../fixtures/postgres.sql");
 
 /// The MySQL fixture script (one statement per `;` at the end of a line).
 pub const MYSQL_SQL: &str = include_str!("../fixtures/mysql.sql");
+
+#[cfg(test)]
+mod tests {
+    use super::*;
+
+    #[test]
+    fn the_bookshop_is_written_to_a_new_file_only() {
+        let dir = tempfile::tempdir().unwrap();
+        let path = dir.path().join("bookshop.db");
+        write_sqlite_bookshop(&path).unwrap();
+        let connection = rusqlite::Connection::open(&path).unwrap();
+        let count = |table: &str| -> i64 {
+            connection
+                .query_row(&format!("SELECT count(*) FROM {table}"), [], |row| {
+                    row.get(0)
+                })
+                .unwrap()
+        };
+        assert_eq!(count("books"), 12);
+        assert_eq!(count("order_items"), 40);
+        assert_eq!(count("recent_orders"), 7);
+        assert_eq!(count("customer_names"), 8);
+        // A file that is there is left as it is.
+        let again = write_sqlite_bookshop(&path).unwrap_err();
+        assert!(again.to_string().ends_with("already exists"), "{again}");
+        assert_eq!(count("books"), 12);
+    }
+}
diff --git a/crates/tabletist-db/tests/agent.rs b/crates/tabletist-db/tests/agent.rs
new file mode 100644
index 0000000..e9b76c4
--- /dev/null
+++ b/crates/tabletist-db/tests/agent.rs
@@ -0,0 +1,239 @@
+//! What an agent's read does on a real database: the Bookshop, made in
+//! each engine (`docs/superpowers/plans/2026-10-13-mcp-server.md`, F.2).
+//!
+//! PostgreSQL and MySQL need their servers (see compose.yaml). Without the
+//! variables those tests print "skipped" and pass.
+
+// `allow-unwrap-in-tests` does not cover helpers in an integration-test crate.
+#![allow(clippy::unwrap_used)]
+
+use tabletist_db::{
+    Access, ConnectSpec, Connection, Dialect, HostKeys, ScriptMode, Secrets, StatementOutcome,
+    StopFlag, Value, sql,
+};
+
+/// How many rows each table and view of the Bookshop has.
+const STOCK: [(&str, i64); 9] = [
+    ("publishers", 5),
+    ("authors", 6),
+    ("books", 12),
+    ("book_covers", 3),
+    ("customers", 8),
+    ("orders", 12),
+    ("order_items", 40),
+    ("recent_orders", 7),
+    ("customer_names", 8),
+];
+
+/// What `text` answers when the SQL editor runs it and only reads: the
+/// rows of its one statement.
+async fn rows_of(connection: &Connection, text: &str, on: &str) -> Vec<Vec<Value>> {
+    let statements = sql::statements(connection.dialect(), text);
+    let outcome = connection
+        .run_script(&statements, 1_000, ScriptMode::ReadOnly, &StopFlag::new())
+        .await
+        .unwrap_or_else(|error| panic!("{on}: {error}"));
+    match outcome
+        .results
+        .into_iter()
+        .next()
+        .map(|result| result.outcome)
+    {
+        Some(StatementOutcome::Rows { rows, .. }) => rows,
+        other => panic!("{on}: {text} answered {other:?}"),
+    }
+}
+
+/// Every table and view of the Bookshop holds what its file gave it: what
+/// a test asks before it reads, and again after, when nothing may have
+/// changed.
+async fn the_bookshop_is_stocked(connection: &Connection, on: &str) {
+    for (table, count) in STOCK {
+        let rows = rows_of(connection, &format!("SELECT count(*) FROM {table}"), on).await;
+        assert_eq!(rows, [[Value::Int(count)]], "{on}: {table}");
+    }
+}
+
+mod sqlite {
+    use super::*;
+
+    /// A Bookshop of the test's own, on a session that cannot write.
+    pub(super) async fn bookshop() -> (Connection, tempfile::TempDir) {
+        let dir = tempfile::tempdir().unwrap();
+        let path = dir.path().join("bookshop.db");
+        tabletist_db::fixtures::write_sqlite_bookshop(&path).unwrap();
+        let connection = Connection::connect_with(
+            &ConnectSpec::sqlite(&path),
+            &Secrets::default(),
+            &HostKeys::default(),
+            Access::ReadOnly,
+        )
+        .await
+        .unwrap();
+        (connection, dir)
+    }
+
+    #[tokio::test]
+    async fn the_bookshop_is_stocked_on_sqlite() {
+        let (connection, _dir) = bookshop().await;
+        assert_eq!(connection.dialect(), Dialect::Sqlite);
+        the_bookshop_is_stocked(&connection, "sqlite").await;
+    }
+}
+
+mod postgres {
+    use super::*;
+    use tabletist_db::TlsMode;
+    use tokio::sync::OnceCell;
+
+    /// Loaded once for the tests of this file: they only read it.
+    static BOOKSHOP: OnceCell<()> = OnceCell::const_new();
+
+    fn url() -> Option<String> {
+        let url = std::env::var("TABLETIST_TEST_PG_URL").ok();
+        url.filter(|url| !url.trim().is_empty())
+    }
+
+    /// A session that may do anything, on the URL's database or on
+    /// `database`.
+    async fn admin(url: &str, database: Option<&str>) -> tokio_postgres::Client {
+        let mut config: tokio_postgres::Config = url.parse().unwrap();
+        config.ssl_mode(tokio_postgres::config::SslMode::Disable);
+        if let Some(database) = database {
+            config.dbname(database);
+        }
+        let (client, session) = config.connect(tokio_postgres::NoTls).await.unwrap();
+        tokio::spawn(session);
+        client
+    }
+
+    /// The Bookshop has a database of its own, `bookshop`: its `orders` is
+    /// not the driver fixture's.
+    async fn load(url: &str) {
+        BOOKSHOP
+            .get_or_init(|| async {
+                let server = admin(url, None).await;
+                let made = server
+                    .query_opt("SELECT 1 FROM pg_database WHERE datname = 'bookshop'", &[])
+                    .await
+                    .unwrap()
+                    .is_some();
+                if !made {
+                    server
+                        .batch_execute("CREATE DATABASE bookshop")
+                        .await
+                        .unwrap();
+                }
+                admin(url, Some("bookshop"))
+                    .await
+                    .batch_execute(tabletist_db::fixtures::BOOKSHOP_POSTGRES_SQL)
+                    .await
+                    .unwrap();
+            })
+            .await;
+    }
+
+    /// A session on the Bookshop that cannot write, or `None` (test
+    /// skipped).
+    pub(super) async fn bookshop() -> Option<Connection> {
+        let Some(url) = url() else {
+            eprintln!("skipped: TABLETIST_TEST_PG_URL is not set");
+            return None;
+        };
+        load(&url).await;
+        let (mut spec, secrets) = ConnectSpec::from_url(&url).unwrap();
+        spec.tls = TlsMode::Disable;
+        spec.database = "bookshop".into();
+        let connection =
+            Connection::connect_with(&spec, &secrets, &HostKeys::default(), Access::ReadOnly)
+                .await
+                .unwrap();
+        Some(connection)
+    }
+
+    #[tokio::test]
+    async fn the_bookshop_is_stocked_on_postgres() {
+        let Some(connection) = bookshop().await else {
+            return;
+        };
+        assert_eq!(connection.dialect(), Dialect::Postgres);
+        the_bookshop_is_stocked(&connection, "postgres").await;
+    }
+}
+
+mod mysql {
+    use super::*;
+    use mysql_async::prelude::Queryable;
+    use tabletist_db::TlsMode;
+    use tokio::sync::OnceCell;
+
+    /// Loaded once for the tests of this file: they only read it.
+    static BOOKSHOP: OnceCell<()> = OnceCell::const_new();
+
+    fn url() -> Option<String> {
+        let url = std::env::var("TABLETIST_TEST_MYSQL_URL").ok();
+        url.filter(|url| !url.trim().is_empty())
+    }
+
+    /// A session of the driver's own on the Bookshop, which has a database
+    /// of its own, `bookshop`. The server's first start makes it
+    /// (compose/mysql-init.sql): the tests' user may not make one.
+    async fn admin(url: &str) -> mysql_async::Conn {
+        let (server, _database) = url.rsplit_once('/').unwrap();
+        let opts =
+            mysql_async::Opts::from_url(&format!("{server}/bookshop?prefer_socket=false")).unwrap();
+        mysql_async::Conn::new(opts).await.unwrap_or_else(|error| {
+            panic!(
+                "the test MySQL has no `bookshop` database ({error}). \
+                 compose/mysql-init.sql makes it on a new volume only: run its last two \
+                 statements as root, or make the volume again"
+            )
+        })
+    }
+
+    async fn load(url: &str) {
+        BOOKSHOP
+            .get_or_init(|| async {
+                let mut conn = admin(url).await;
+                for statement in tabletist_db::fixtures::BOOKSHOP_MYSQL_SQL.split(";\n") {
+                    let statement: String = statement
+                        .lines()
+                        .filter(|line| !line.trim_start().starts_with("--"))
+                        .collect::<Vec<_>>()
+                        .join("\n");
+                    if !statement.trim().is_empty() {
+                        conn.query_drop(statement).await.unwrap();
+                    }
+                }
+                conn.disconnect().await.unwrap();
+            })
+            .await;
+    }
+
+    /// A session on the Bookshop that cannot write, or `None` (test
+    /// skipped).
+    pub(super) async fn bookshop() -> Option<Connection> {
+        let Some(url) = url() else {
+            eprintln!("skipped: TABLETIST_TEST_MYSQL_URL is not set");
+            return None;
+        };
+        load(&url).await;
+        let (mut spec, secrets) = ConnectSpec::from_url(&url).unwrap();
+        spec.tls = TlsMode::Disable;
+        spec.database = "bookshop".into();
+        let connection =
+            Connection::connect_with(&spec, &secrets, &HostKeys::default(), Access::ReadOnly)
+                .await
+                .unwrap();
+        Some(connection)
+    }
+
+    #[tokio::test]
+    async fn the_bookshop_is_stocked_on_mysql() {
+        let Some(connection) = bookshop().await else {
+            return;
+        };
+        assert_eq!(connection.dialect(), Dialect::MySql);
+        the_bookshop_is_stocked(&connection, "mysql").await;
+    }
+}
```

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test --locked -p tabletist-db --lib fixtures --test agent`

Expected: it does not compile. The fixture is not there yet:

```
error[E0425]: cannot find function `write_sqlite_bookshop` in module `tabletist_db::fixtures`
error[E0425]: cannot find value `BOOKSHOP_POSTGRES_SQL` in module `tabletist_db::fixtures`
error[E0425]: cannot find value `BOOKSHOP_MYSQL_SQL` in module `tabletist_db::fixtures`
error[E0425]: cannot find function `write_sqlite_bookshop` in this scope
```

- [ ] **Step 3: Add the fixture**

The three files were written by one script from one list of rows, which is how they agree. The script is not kept: from here on a row is changed in all three files by hand.

```diff
diff --git a/compose/mysql-init.sql b/compose/mysql-init.sql
index d0a5592..f7edbff 100644
--- a/compose/mysql-init.sql
+++ b/compose/mysql-init.sql
@@ -4,3 +4,7 @@ GRANT ALL PRIVILEGES ON billing.* TO 'tabletist'@'%';
 -- triggers only to who has TRIGGER on it. One test of a save needs one.
 CREATE USER 'writer'@'%' IDENTIFIED BY 'writer';
 GRANT SELECT, INSERT, UPDATE, DELETE ON tabletist.* TO 'writer'@'%';
+-- The Bookshop (crates/tabletist-db/fixtures/bookshop/mysql.sql), which
+-- what an agent may read is tested on, in a database of its own.
+CREATE DATABASE bookshop;
+GRANT ALL PRIVILEGES ON bookshop.* TO 'tabletist'@'%';
diff --git a/crates/tabletist-db/fixtures/bookshop/mysql.sql b/crates/tabletist-db/fixtures/bookshop/mysql.sql
new file mode 100644
index 0000000..82bb3c2
--- /dev/null
+++ b/crates/tabletist-db/fixtures/bookshop/mysql.sql
@@ -0,0 +1,166 @@
+-- The Bookshop: the made-up shop of the app's screenshots, as a database.
+-- What an agent may read through the MCP server is tested on it, on each
+-- engine. The three files hold the same rows. Keep them deterministic.
+-- Loaded into a database of its own, `bookshop` (compose/mysql-init.sql).
+-- One statement for each `;` at the end of a line.
+
+DROP VIEW IF EXISTS recent_orders, customer_names;
+DROP TABLE IF EXISTS order_items, orders, customers, book_covers, books, authors, publishers;
+
+CREATE TABLE publishers (
+    id INT PRIMARY KEY,
+    name VARCHAR(64) NOT NULL,
+    city VARCHAR(64)
+) CHARACTER SET utf8mb4;
+CREATE TABLE authors (
+    id INT PRIMARY KEY,
+    name VARCHAR(64) NOT NULL
+) CHARACTER SET utf8mb4;
+CREATE TABLE books (
+    id INT PRIMARY KEY,
+    title VARCHAR(128) NOT NULL,
+    author_id INT NOT NULL,
+    publisher_id INT NOT NULL,
+    price DECIMAL(8, 2) NOT NULL,
+    published DATE,
+    in_print BOOLEAN NOT NULL DEFAULT TRUE,
+    FOREIGN KEY (author_id) REFERENCES authors (id),
+    FOREIGN KEY (publisher_id) REFERENCES publishers (id)
+) CHARACTER SET utf8mb4;
+CREATE TABLE book_covers (
+    id INT PRIMARY KEY,
+    book_id INT NOT NULL,
+    kind VARCHAR(16) NOT NULL,
+    image BLOB,
+    FOREIGN KEY (book_id) REFERENCES books (id)
+) CHARACTER SET utf8mb4;
+CREATE TABLE customers (
+    id INT PRIMARY KEY,
+    name VARCHAR(64) NOT NULL,
+    email VARCHAR(128) NOT NULL,
+    phone VARCHAR(32)
+) CHARACTER SET utf8mb4;
+CREATE TABLE orders (
+    id INT PRIMARY KEY,
+    customer_id INT NOT NULL,
+    email VARCHAR(128) NOT NULL,
+    phone VARCHAR(32),
+    placed_at DATETIME NOT NULL,
+    total DECIMAL(10, 2) NOT NULL,
+    FOREIGN KEY (customer_id) REFERENCES customers (id)
+) CHARACTER SET utf8mb4;
+CREATE TABLE order_items (
+    id INT PRIMARY KEY,
+    order_id INT NOT NULL,
+    book_id INT NOT NULL,
+    quantity INT NOT NULL,
+    price DECIMAL(8, 2) NOT NULL,
+    FOREIGN KEY (order_id) REFERENCES orders (id) ON DELETE CASCADE,
+    FOREIGN KEY (book_id) REFERENCES books (id)
+) CHARACTER SET utf8mb4;
+
+INSERT INTO publishers (id, name, city) VALUES
+    (1, 'Northlight Books', 'Bergen'),
+    (2, 'Harbor Press', 'Halifax'),
+    (3, 'Harbor Lights', 'Lisbon'),
+    (4, 'Studio 91', 'Kyoto'),
+    (5, '100% Books', NULL);
+
+INSERT INTO authors (id, name) VALUES
+    (1, 'Ines Calloway'),
+    (2, 'Tomas Varga'),
+    (3, 'Mei Arakawa'),
+    (4, 'Rufus Oyelaran'),
+    (5, 'Zoë Lindqvist'),
+    (6, 'N. K. O''Dell');
+
+INSERT INTO books (id, title, author_id, publisher_id, price, published, in_print) VALUES
+    (1, 'The Salt Road', 1, 1, 18.50, '2019-03-14', TRUE),
+    (2, 'A Field Guide to Fog', 2, 1, 24.00, '2021-09-02', TRUE),
+    (3, 'Lanterns', 3, 2, 12.99, '2017-11-20', TRUE),
+    (4, 'The Cartographer''s Daughter', 6, 2, 21.75, '2022-05-05', TRUE),
+    (5, 'Small Engines', 4, 3, 15.00, '2015-01-30', FALSE),
+    (6, 'Winter Harbour', 1, 3, 19.99, '2023-10-10', TRUE),
+    (7, 'Notes on Bread', 5, 4, 30.00, '2020-02-29', TRUE),
+    (8, 'Ninety-One Doors', 3, 4, 16.40, '2018-07-07', TRUE),
+    (9, 'The Quiet Ledger', 2, 5, 9.99, '2012-12-12', FALSE),
+    (10, 'Tidewater', 4, 2, 22.00, '2024-04-18', TRUE),
+    (11, 'An Atlas of Small Things', 5, 1, 27.30, '2025-06-01', TRUE),
+    (12, '100% Rain', 6, 5, 11.25, '2016-08-23', TRUE);
+
+INSERT INTO book_covers (id, book_id, kind, image) VALUES
+    (1, 1, 'front', X'89504E470D0A1A0A'),
+    (2, 1, 'back', X'FFD8FFE000104A46'),
+    (3, 7, 'front', X'47494638396101');
+
+INSERT INTO customers (id, name, email, phone) VALUES
+    (1, 'Ada Brandt', 'ada.brandt@example.com', '+1 555 0101'),
+    (2, 'Bo Castellan', 'bo@example.com', '+1 555 0102'),
+    (3, 'Chidi Okafor', 'chidi.okafor@example.com', NULL),
+    (4, 'Dana Whitlock', 'dana@example.org', '+44 20 7946 0104'),
+    (5, 'Emil Sørensen', 'emil@example.com', '+45 55 50 01 05'),
+    (6, 'Farah Qureshi', 'farah.q@example.net', NULL),
+    (7, 'Greta O''Malley', 'greta@example.com', '+353 1 555 0107'),
+    (8, 'Hiro Tanabe', 'hiro@example.org', '+81 3 5550 0108');
+
+INSERT INTO orders (id, customer_id, email, phone, placed_at, total) VALUES
+    (1, 6, 'farah.q@example.net', NULL, '2025-02-08 09:13:00', 176.25),
+    (2, 3, 'chidi.okafor@example.com', NULL, '2025-04-15 10:26:00', 201.70),
+    (3, 8, 'hiro@example.org', '+81 3 5550 0108', '2025-06-22 11:39:00', 94.19),
+    (4, 5, 'emil@example.com', '+45 55 50 01 05', '2025-08-02 12:52:00', 192.50),
+    (5, 2, 'bo@example.com', '+1 555 0102', '2025-10-09 13:05:00', 116.00),
+    (6, 7, 'greta@example.com', '+353 1 555 0107', '2026-12-16 14:18:00', 73.72),
+    (7, 4, 'dana@example.org', '+44 20 7946 0104', '2026-02-23 15:31:00', 144.50),
+    (8, 1, 'ada.brandt@example.com', '+1 555 0101', '2026-04-03 16:44:00', 124.50),
+    (9, 6, 'farah.q@example.net', NULL, '2026-06-10 17:57:00', 71.46),
+    (10, 3, 'chidi.okafor@example.com', NULL, '2026-08-17 08:10:00', 121.25),
+    (11, 8, 'hiro@example.org', '+81 3 5550 0108', '2026-10-24 09:23:00', 135.90),
+    (12, 5, 'emil@example.com', '+45 55 50 01 05', '2026-12-04 10:36:00', 97.20);
+
+INSERT INTO order_items (id, order_id, book_id, quantity, price) VALUES
+    (1, 1, 1, 2, 18.50),
+    (2, 1, 4, 3, 21.75),
+    (3, 1, 7, 1, 30.00),
+    (4, 1, 10, 2, 22.00),
+    (5, 2, 2, 3, 24.00),
+    (6, 2, 5, 1, 15.00),
+    (7, 2, 8, 2, 16.40),
+    (8, 2, 11, 3, 27.30),
+    (9, 3, 3, 1, 12.99),
+    (10, 3, 6, 2, 19.99),
+    (11, 3, 9, 3, 9.99),
+    (12, 3, 12, 1, 11.25),
+    (13, 4, 4, 2, 21.75),
+    (14, 4, 7, 3, 30.00),
+    (15, 4, 10, 1, 22.00),
+    (16, 4, 1, 2, 18.50),
+    (17, 5, 5, 3, 15.00),
+    (18, 5, 8, 1, 16.40),
+    (19, 5, 11, 2, 27.30),
+    (20, 6, 6, 1, 19.99),
+    (21, 6, 9, 2, 9.99),
+    (22, 6, 12, 3, 11.25),
+    (23, 7, 7, 2, 30.00),
+    (24, 7, 10, 3, 22.00),
+    (25, 7, 1, 1, 18.50),
+    (26, 8, 8, 3, 16.40),
+    (27, 8, 11, 1, 27.30),
+    (28, 8, 2, 2, 24.00),
+    (29, 9, 9, 1, 9.99),
+    (30, 9, 12, 2, 11.25),
+    (31, 9, 3, 3, 12.99),
+    (32, 10, 10, 2, 22.00),
+    (33, 10, 1, 3, 18.50),
+    (34, 10, 4, 1, 21.75),
+    (35, 11, 11, 3, 27.30),
+    (36, 11, 2, 1, 24.00),
+    (37, 11, 5, 2, 15.00),
+    (38, 12, 12, 1, 11.25),
+    (39, 12, 3, 2, 12.99),
+    (40, 12, 6, 3, 19.99);
+
+-- Two ways to a table's rows that do not name it: what hiding a table and
+-- masking a column must also cover.
+CREATE VIEW recent_orders AS
+    SELECT id, email, placed_at, total FROM orders WHERE placed_at >= '2026-01-01';
+CREATE VIEW customer_names AS SELECT id, name FROM customers;
diff --git a/crates/tabletist-db/fixtures/bookshop/postgres.sql b/crates/tabletist-db/fixtures/bookshop/postgres.sql
new file mode 100644
index 0000000..38fc705
--- /dev/null
+++ b/crates/tabletist-db/fixtures/bookshop/postgres.sql
@@ -0,0 +1,159 @@
+-- The Bookshop: the made-up shop of the app's screenshots, as a database.
+-- What an agent may read through the MCP server is tested on it, on each
+-- engine. The three files hold the same rows. Keep them deterministic.
+-- Loaded into a database of its own, `bookshop`, which the tests make.
+
+DROP VIEW IF EXISTS recent_orders, customer_names;
+DROP TABLE IF EXISTS order_items, orders, customers, book_covers, books, authors, publishers;
+
+CREATE TABLE publishers (
+    id integer PRIMARY KEY,
+    name text NOT NULL,
+    city text
+);
+CREATE TABLE authors (
+    id integer PRIMARY KEY,
+    name text NOT NULL
+);
+CREATE TABLE books (
+    id integer PRIMARY KEY,
+    title text NOT NULL,
+    author_id integer NOT NULL REFERENCES authors (id),
+    publisher_id integer NOT NULL REFERENCES publishers (id),
+    price numeric(8, 2) NOT NULL,
+    published date,
+    in_print boolean NOT NULL DEFAULT true
+);
+CREATE TABLE book_covers (
+    id integer PRIMARY KEY,
+    book_id integer NOT NULL REFERENCES books (id),
+    kind text NOT NULL,
+    image bytea
+);
+CREATE TABLE customers (
+    id integer PRIMARY KEY,
+    name text NOT NULL,
+    email text NOT NULL,
+    phone text
+);
+CREATE TABLE orders (
+    id integer PRIMARY KEY,
+    customer_id integer NOT NULL REFERENCES customers (id),
+    email text NOT NULL,
+    phone text,
+    placed_at timestamp NOT NULL,
+    total numeric(10, 2) NOT NULL
+);
+CREATE TABLE order_items (
+    id integer PRIMARY KEY,
+    order_id integer NOT NULL REFERENCES orders (id) ON DELETE CASCADE,
+    book_id integer NOT NULL REFERENCES books (id),
+    quantity integer NOT NULL,
+    price numeric(8, 2) NOT NULL
+);
+
+INSERT INTO publishers (id, name, city) VALUES
+    (1, 'Northlight Books', 'Bergen'),
+    (2, 'Harbor Press', 'Halifax'),
+    (3, 'Harbor Lights', 'Lisbon'),
+    (4, 'Studio 91', 'Kyoto'),
+    (5, '100% Books', NULL);
+
+INSERT INTO authors (id, name) VALUES
+    (1, 'Ines Calloway'),
+    (2, 'Tomas Varga'),
+    (3, 'Mei Arakawa'),
+    (4, 'Rufus Oyelaran'),
+    (5, 'Zoë Lindqvist'),
+    (6, 'N. K. O''Dell');
+
+INSERT INTO books (id, title, author_id, publisher_id, price, published, in_print) VALUES
+    (1, 'The Salt Road', 1, 1, 18.50, '2019-03-14', TRUE),
+    (2, 'A Field Guide to Fog', 2, 1, 24.00, '2021-09-02', TRUE),
+    (3, 'Lanterns', 3, 2, 12.99, '2017-11-20', TRUE),
+    (4, 'The Cartographer''s Daughter', 6, 2, 21.75, '2022-05-05', TRUE),
+    (5, 'Small Engines', 4, 3, 15.00, '2015-01-30', FALSE),
+    (6, 'Winter Harbour', 1, 3, 19.99, '2023-10-10', TRUE),
+    (7, 'Notes on Bread', 5, 4, 30.00, '2020-02-29', TRUE),
+    (8, 'Ninety-One Doors', 3, 4, 16.40, '2018-07-07', TRUE),
+    (9, 'The Quiet Ledger', 2, 5, 9.99, '2012-12-12', FALSE),
+    (10, 'Tidewater', 4, 2, 22.00, '2024-04-18', TRUE),
+    (11, 'An Atlas of Small Things', 5, 1, 27.30, '2025-06-01', TRUE),
+    (12, '100% Rain', 6, 5, 11.25, '2016-08-23', TRUE);
+
+INSERT INTO book_covers (id, book_id, kind, image) VALUES
+    (1, 1, 'front', '\x89504e470d0a1a0a'),
+    (2, 1, 'back', '\xffd8ffe000104a46'),
+    (3, 7, 'front', '\x47494638396101');
+
+INSERT INTO customers (id, name, email, phone) VALUES
+    (1, 'Ada Brandt', 'ada.brandt@example.com', '+1 555 0101'),
+    (2, 'Bo Castellan', 'bo@example.com', '+1 555 0102'),
+    (3, 'Chidi Okafor', 'chidi.okafor@example.com', NULL),
+    (4, 'Dana Whitlock', 'dana@example.org', '+44 20 7946 0104'),
+    (5, 'Emil Sørensen', 'emil@example.com', '+45 55 50 01 05'),
+    (6, 'Farah Qureshi', 'farah.q@example.net', NULL),
+    (7, 'Greta O''Malley', 'greta@example.com', '+353 1 555 0107'),
+    (8, 'Hiro Tanabe', 'hiro@example.org', '+81 3 5550 0108');
+
+INSERT INTO orders (id, customer_id, email, phone, placed_at, total) VALUES
+    (1, 6, 'farah.q@example.net', NULL, '2025-02-08 09:13:00', 176.25),
+    (2, 3, 'chidi.okafor@example.com', NULL, '2025-04-15 10:26:00', 201.70),
+    (3, 8, 'hiro@example.org', '+81 3 5550 0108', '2025-06-22 11:39:00', 94.19),
+    (4, 5, 'emil@example.com', '+45 55 50 01 05', '2025-08-02 12:52:00', 192.50),
+    (5, 2, 'bo@example.com', '+1 555 0102', '2025-10-09 13:05:00', 116.00),
+    (6, 7, 'greta@example.com', '+353 1 555 0107', '2026-12-16 14:18:00', 73.72),
+    (7, 4, 'dana@example.org', '+44 20 7946 0104', '2026-02-23 15:31:00', 144.50),
+    (8, 1, 'ada.brandt@example.com', '+1 555 0101', '2026-04-03 16:44:00', 124.50),
+    (9, 6, 'farah.q@example.net', NULL, '2026-06-10 17:57:00', 71.46),
+    (10, 3, 'chidi.okafor@example.com', NULL, '2026-08-17 08:10:00', 121.25),
+    (11, 8, 'hiro@example.org', '+81 3 5550 0108', '2026-10-24 09:23:00', 135.90),
+    (12, 5, 'emil@example.com', '+45 55 50 01 05', '2026-12-04 10:36:00', 97.20);
+
+INSERT INTO order_items (id, order_id, book_id, quantity, price) VALUES
+    (1, 1, 1, 2, 18.50),
+    (2, 1, 4, 3, 21.75),
+    (3, 1, 7, 1, 30.00),
+    (4, 1, 10, 2, 22.00),
+    (5, 2, 2, 3, 24.00),
+    (6, 2, 5, 1, 15.00),
+    (7, 2, 8, 2, 16.40),
+    (8, 2, 11, 3, 27.30),
+    (9, 3, 3, 1, 12.99),
+    (10, 3, 6, 2, 19.99),
+    (11, 3, 9, 3, 9.99),
+    (12, 3, 12, 1, 11.25),
+    (13, 4, 4, 2, 21.75),
+    (14, 4, 7, 3, 30.00),
+    (15, 4, 10, 1, 22.00),
+    (16, 4, 1, 2, 18.50),
+    (17, 5, 5, 3, 15.00),
+    (18, 5, 8, 1, 16.40),
+    (19, 5, 11, 2, 27.30),
+    (20, 6, 6, 1, 19.99),
+    (21, 6, 9, 2, 9.99),
+    (22, 6, 12, 3, 11.25),
+    (23, 7, 7, 2, 30.00),
+    (24, 7, 10, 3, 22.00),
+    (25, 7, 1, 1, 18.50),
+    (26, 8, 8, 3, 16.40),
+    (27, 8, 11, 1, 27.30),
+    (28, 8, 2, 2, 24.00),
+    (29, 9, 9, 1, 9.99),
+    (30, 9, 12, 2, 11.25),
+    (31, 9, 3, 3, 12.99),
+    (32, 10, 10, 2, 22.00),
+    (33, 10, 1, 3, 18.50),
+    (34, 10, 4, 1, 21.75),
+    (35, 11, 11, 3, 27.30),
+    (36, 11, 2, 1, 24.00),
+    (37, 11, 5, 2, 15.00),
+    (38, 12, 12, 1, 11.25),
+    (39, 12, 3, 2, 12.99),
+    (40, 12, 6, 3, 19.99);
+
+-- Two ways to a table's rows that do not name it: what hiding a table and
+-- masking a column must also cover.
+CREATE VIEW recent_orders AS
+    SELECT id, email, placed_at, total FROM orders WHERE placed_at >= '2026-01-01';
+CREATE VIEW customer_names AS SELECT id, name FROM customers;
diff --git a/crates/tabletist-db/fixtures/bookshop/sqlite.sql b/crates/tabletist-db/fixtures/bookshop/sqlite.sql
new file mode 100644
index 0000000..9f2ee01
--- /dev/null
+++ b/crates/tabletist-db/fixtures/bookshop/sqlite.sql
@@ -0,0 +1,155 @@
+-- The Bookshop: the made-up shop of the app's screenshots, as a database.
+-- What an agent may read through the MCP server is tested on it, on each
+-- engine. The three files hold the same rows. Keep them deterministic.
+
+CREATE TABLE publishers (
+    id INTEGER PRIMARY KEY,
+    name TEXT NOT NULL,
+    city TEXT
+);
+CREATE TABLE authors (
+    id INTEGER PRIMARY KEY,
+    name TEXT NOT NULL
+);
+CREATE TABLE books (
+    id INTEGER PRIMARY KEY,
+    title TEXT NOT NULL,
+    author_id INTEGER NOT NULL REFERENCES authors (id),
+    publisher_id INTEGER NOT NULL REFERENCES publishers (id),
+    price NUMERIC(8, 2) NOT NULL,
+    published DATE,
+    in_print BOOLEAN NOT NULL DEFAULT 1
+);
+CREATE TABLE book_covers (
+    id INTEGER PRIMARY KEY,
+    book_id INTEGER NOT NULL REFERENCES books (id),
+    kind TEXT NOT NULL,
+    image BLOB
+);
+CREATE TABLE customers (
+    id INTEGER PRIMARY KEY,
+    name TEXT NOT NULL,
+    email TEXT NOT NULL,
+    phone TEXT
+);
+CREATE TABLE orders (
+    id INTEGER PRIMARY KEY,
+    customer_id INTEGER NOT NULL REFERENCES customers (id),
+    email TEXT NOT NULL,
+    phone TEXT,
+    placed_at DATETIME NOT NULL,
+    total NUMERIC(10, 2) NOT NULL
+);
+CREATE TABLE order_items (
+    id INTEGER PRIMARY KEY,
+    order_id INTEGER NOT NULL REFERENCES orders (id) ON DELETE CASCADE,
+    book_id INTEGER NOT NULL REFERENCES books (id),
+    quantity INTEGER NOT NULL,
+    price NUMERIC(8, 2) NOT NULL
+);
+
+INSERT INTO publishers (id, name, city) VALUES
+    (1, 'Northlight Books', 'Bergen'),
+    (2, 'Harbor Press', 'Halifax'),
+    (3, 'Harbor Lights', 'Lisbon'),
+    (4, 'Studio 91', 'Kyoto'),
+    (5, '100% Books', NULL);
+
+INSERT INTO authors (id, name) VALUES
+    (1, 'Ines Calloway'),
+    (2, 'Tomas Varga'),
+    (3, 'Mei Arakawa'),
+    (4, 'Rufus Oyelaran'),
+    (5, 'Zoë Lindqvist'),
+    (6, 'N. K. O''Dell');
+
+INSERT INTO books (id, title, author_id, publisher_id, price, published, in_print) VALUES
+    (1, 'The Salt Road', 1, 1, 18.50, '2019-03-14', TRUE),
+    (2, 'A Field Guide to Fog', 2, 1, 24.00, '2021-09-02', TRUE),
+    (3, 'Lanterns', 3, 2, 12.99, '2017-11-20', TRUE),
+    (4, 'The Cartographer''s Daughter', 6, 2, 21.75, '2022-05-05', TRUE),
+    (5, 'Small Engines', 4, 3, 15.00, '2015-01-30', FALSE),
+    (6, 'Winter Harbour', 1, 3, 19.99, '2023-10-10', TRUE),
+    (7, 'Notes on Bread', 5, 4, 30.00, '2020-02-29', TRUE),
+    (8, 'Ninety-One Doors', 3, 4, 16.40, '2018-07-07', TRUE),
+    (9, 'The Quiet Ledger', 2, 5, 9.99, '2012-12-12', FALSE),
+    (10, 'Tidewater', 4, 2, 22.00, '2024-04-18', TRUE),
+    (11, 'An Atlas of Small Things', 5, 1, 27.30, '2025-06-01', TRUE),
+    (12, '100% Rain', 6, 5, 11.25, '2016-08-23', TRUE);
+
+INSERT INTO book_covers (id, book_id, kind, image) VALUES
+    (1, 1, 'front', X'89504E470D0A1A0A'),
+    (2, 1, 'back', X'FFD8FFE000104A46'),
+    (3, 7, 'front', X'47494638396101');
+
+INSERT INTO customers (id, name, email, phone) VALUES
+    (1, 'Ada Brandt', 'ada.brandt@example.com', '+1 555 0101'),
+    (2, 'Bo Castellan', 'bo@example.com', '+1 555 0102'),
+    (3, 'Chidi Okafor', 'chidi.okafor@example.com', NULL),
+    (4, 'Dana Whitlock', 'dana@example.org', '+44 20 7946 0104'),
+    (5, 'Emil Sørensen', 'emil@example.com', '+45 55 50 01 05'),
+    (6, 'Farah Qureshi', 'farah.q@example.net', NULL),
+    (7, 'Greta O''Malley', 'greta@example.com', '+353 1 555 0107'),
+    (8, 'Hiro Tanabe', 'hiro@example.org', '+81 3 5550 0108');
+
+INSERT INTO orders (id, customer_id, email, phone, placed_at, total) VALUES
+    (1, 6, 'farah.q@example.net', NULL, '2025-02-08 09:13:00', 176.25),
+    (2, 3, 'chidi.okafor@example.com', NULL, '2025-04-15 10:26:00', 201.70),
+    (3, 8, 'hiro@example.org', '+81 3 5550 0108', '2025-06-22 11:39:00', 94.19),
+    (4, 5, 'emil@example.com', '+45 55 50 01 05', '2025-08-02 12:52:00', 192.50),
+    (5, 2, 'bo@example.com', '+1 555 0102', '2025-10-09 13:05:00', 116.00),
+    (6, 7, 'greta@example.com', '+353 1 555 0107', '2026-12-16 14:18:00', 73.72),
+    (7, 4, 'dana@example.org', '+44 20 7946 0104', '2026-02-23 15:31:00', 144.50),
+    (8, 1, 'ada.brandt@example.com', '+1 555 0101', '2026-04-03 16:44:00', 124.50),
+    (9, 6, 'farah.q@example.net', NULL, '2026-06-10 17:57:00', 71.46),
+    (10, 3, 'chidi.okafor@example.com', NULL, '2026-08-17 08:10:00', 121.25),
+    (11, 8, 'hiro@example.org', '+81 3 5550 0108', '2026-10-24 09:23:00', 135.90),
+    (12, 5, 'emil@example.com', '+45 55 50 01 05', '2026-12-04 10:36:00', 97.20);
+
+INSERT INTO order_items (id, order_id, book_id, quantity, price) VALUES
+    (1, 1, 1, 2, 18.50),
+    (2, 1, 4, 3, 21.75),
+    (3, 1, 7, 1, 30.00),
+    (4, 1, 10, 2, 22.00),
+    (5, 2, 2, 3, 24.00),
+    (6, 2, 5, 1, 15.00),
+    (7, 2, 8, 2, 16.40),
+    (8, 2, 11, 3, 27.30),
+    (9, 3, 3, 1, 12.99),
+    (10, 3, 6, 2, 19.99),
+    (11, 3, 9, 3, 9.99),
+    (12, 3, 12, 1, 11.25),
+    (13, 4, 4, 2, 21.75),
+    (14, 4, 7, 3, 30.00),
+    (15, 4, 10, 1, 22.00),
+    (16, 4, 1, 2, 18.50),
+    (17, 5, 5, 3, 15.00),
+    (18, 5, 8, 1, 16.40),
+    (19, 5, 11, 2, 27.30),
+    (20, 6, 6, 1, 19.99),
+    (21, 6, 9, 2, 9.99),
+    (22, 6, 12, 3, 11.25),
+    (23, 7, 7, 2, 30.00),
+    (24, 7, 10, 3, 22.00),
+    (25, 7, 1, 1, 18.50),
+    (26, 8, 8, 3, 16.40),
+    (27, 8, 11, 1, 27.30),
+    (28, 8, 2, 2, 24.00),
+    (29, 9, 9, 1, 9.99),
+    (30, 9, 12, 2, 11.25),
+    (31, 9, 3, 3, 12.99),
+    (32, 10, 10, 2, 22.00),
+    (33, 10, 1, 3, 18.50),
+    (34, 10, 4, 1, 21.75),
+    (35, 11, 11, 3, 27.30),
+    (36, 11, 2, 1, 24.00),
+    (37, 11, 5, 2, 15.00),
+    (38, 12, 12, 1, 11.25),
+    (39, 12, 3, 2, 12.99),
+    (40, 12, 6, 3, 19.99);
+
+-- Two ways to a table's rows that do not name it: what hiding a table and
+-- masking a column must also cover.
+CREATE VIEW recent_orders AS
+    SELECT id, email, placed_at, total FROM orders WHERE placed_at >= '2026-01-01';
+CREATE VIEW customer_names AS SELECT id, name FROM customers;
diff --git a/crates/tabletist-db/src/fixtures.rs b/crates/tabletist-db/src/fixtures.rs
index 275cd71..5977a87 100644
--- a/crates/tabletist-db/src/fixtures.rs
+++ b/crates/tabletist-db/src/fixtures.rs
@@ -1,4 +1,5 @@
-//! The shared SQLite fixture, for tests and the app's demo mode.
+//! The shared fixtures: the one the driver tests and the app's demo mode
+//! use, and the Bookshop, which what an agent may read is tested on.
 
 use std::path::Path;
 
@@ -10,6 +11,21 @@ pub const SQLITE_SQL: &str = include_str!("../fixtures/sqlite.sql");
 /// Creates a new SQLite file at `path` holding the fixture. Refuses to touch
 /// an existing file, so it can never be pointed at a user's database.
 pub fn write_sqlite_demo(path: &Path) -> Result<()> {
+    write_sqlite(path, SQLITE_SQL)
+}
+
+/// The Bookshop as a SQLite script: the made-up shop of the app's
+/// screenshots, as tables.
+pub const BOOKSHOP_SQLITE_SQL: &str = include_str!("../fixtures/bookshop/sqlite.sql");
+
+/// Creates a new SQLite file at `path` holding the Bookshop. Refuses to
+/// touch an existing file, as [`write_sqlite_demo`] does.
+pub fn write_sqlite_bookshop(path: &Path) -> Result<()> {
+    write_sqlite(path, BOOKSHOP_SQLITE_SQL)
+}
+
+/// A new SQLite file at `path` made by `sql`.
+fn write_sqlite(path: &Path, sql: &str) -> Result<()> {
     if path.exists() {
         return Err(Error::Io(format!("{} already exists", path.display())));
     }
@@ -18,7 +34,7 @@ pub fn write_sqlite_demo(path: &Path) -> Result<()> {
     let connection = rusqlite::Connection::open_with_flags(path, flags)
         .map_err(|error| Error::Io(error.to_string()))?;
     connection
-        .execute_batch(SQLITE_SQL)
+        .execute_batch(sql)
         .map_err(|error| Error::Io(error.to_string()))
 }
 
@@ -29,6 +45,14 @@ pub const POSTGRES_SQL: &str = include_str!("../fixtures/postgres.sql");
 /// The MySQL fixture script (one statement per `;` at the end of a line).
 pub const MYSQL_SQL: &str = include_str!("../fixtures/mysql.sql");
 
+/// The Bookshop for PostgreSQL. The integration tests load it into a
+/// database of its own, `bookshop`.
+pub const BOOKSHOP_POSTGRES_SQL: &str = include_str!("../fixtures/bookshop/postgres.sql");
+
+/// The Bookshop for MySQL, for a database of its own, `bookshop` (one
+/// statement per `;` at the end of a line).
+pub const BOOKSHOP_MYSQL_SQL: &str = include_str!("../fixtures/bookshop/mysql.sql");
+
 #[cfg(test)]
 mod tests {
     use super::*;
```

- [ ] **Step 4: Give a running MySQL its `bookshop` database**

`compose/mysql-init.sql` runs when the server's volume is made. A server that is already running needs the two new statements once, as root:

```sql
CREATE DATABASE IF NOT EXISTS bookshop;
GRANT ALL PRIVILEGES ON bookshop.* TO 'tabletist'@'%';
```

Skip this where the database is there (Igor's machine, since 2026-10-13) or where no MySQL runs. Without it the MySQL test of Step 5 panics with a message that says exactly this.

- [ ] **Step 5: Run the tests**

Run: `cargo test --locked -p tabletist-db --lib fixtures --test agent`

Expected, with both servers: `1 passed` for the library's `fixtures::tests::the_bookshop_is_written_to_a_new_file_only`, and `3 passed` for `tests/agent.rs`, one `the_bookshop_is_stocked_on_…` for each engine. Without a server its test prints "skipped" and passes.

- [ ] **Step 6: Commit**

```bash
cargo fmt --all
cargo clippy --locked -p tabletist-db --all-targets -- -D warnings
git add compose/mysql-init.sql crates/tabletist-db/fixtures/bookshop crates/tabletist-db/src/fixtures.rs crates/tabletist-db/tests/agent.rs
git commit -m "Add the Bookshop as a fixture for the three databases"
```

---

### Task 2: A checked read keeps what it was checked with

`Read` gains the dialect it was checked for and the values it was checked with. The run of Task 4 takes a `Read` and nothing beside it, so the values that are bound are the values that were counted, on a database of the dialect whose rules were applied. `Read` is no longer `Eq`: a value may be a float.

**Files:**
- Modify: `crates/tabletist-db/src/agent.rs`

- [ ] **Step 1: Write the failing tests**

```diff
diff --git a/crates/tabletist-db/src/agent.rs b/crates/tabletist-db/src/agent.rs
index 003755f..fc179e8 100644
--- a/crates/tabletist-db/src/agent.rs
+++ b/crates/tabletist-db/src/agent.rs
@@ -984,7 +984,28 @@ mod tests {
     #[test]
     fn what_passed_prints_without_its_text() {
         let read = read(Dialect::Postgres, "SELECT title FROM books").unwrap();
-        assert_eq!(format!("{read:?}"), "Read { explains: false, .. }");
+        assert_eq!(
+            format!("{read:?}"),
+            "Read { dialect: Postgres, params: 0, explains: false, .. }"
+        );
+    }
+
+    #[test]
+    fn what_passed_keeps_what_it_was_checked_against() {
+        let sent = [Param::Int(4), Param::Text("Lanterns".into())];
+        let read = check_read(
+            Dialect::MySql,
+            "SELECT title FROM books WHERE id = ? OR title = ?",
+            &sent,
+        )
+        .unwrap();
+        assert_eq!(read.dialect(), Dialect::MySql);
+        assert_eq!(read.params(), sent);
+        // Its values are counted when it is printed, and not shown.
+        assert_eq!(
+            format!("{read:?}"),
+            "Read { dialect: MySql, params: 2, explains: false, .. }"
+        );
     }
 
     #[test]
```

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test --locked -p tabletist-db --lib agent::`

Expected: it does not compile.

```
error[E0599]: no method named `dialect` found for struct `agent::Read` in the current scope
error[E0599]: no method named `params` found for struct `agent::Read` in the current scope
```

- [ ] **Step 3: Keep the dialect and the values**

```diff
diff --git a/crates/tabletist-db/src/agent.rs b/crates/tabletist-db/src/agent.rs
index fc179e8..1d34baa 100644
--- a/crates/tabletist-db/src/agent.rs
+++ b/crates/tabletist-db/src/agent.rs
@@ -64,20 +64,34 @@ impl fmt::Debug for Param {
     }
 }
 
-/// A statement that passed [`check_read`]. Its `Debug` output leaves the
-/// text out, as [`sql::Statement`]'s does.
-#[derive(Clone, PartialEq, Eq)]
+/// A statement that passed [`check_read`], with what it was checked
+/// against: [`crate::Connection::run_agent_read`] runs these values on a
+/// database of this dialect, and no others. Its `Debug` output leaves the
+/// text and the values out, as [`sql::Statement`]'s does.
+#[derive(Clone, PartialEq)]
 pub struct Read {
+    dialect: Dialect,
     text: String,
+    params: Vec<Param>,
     explains: bool,
 }
 
 impl Read {
+    /// The dialect the statement was checked for.
+    pub fn dialect(&self) -> Dialect {
+        self.dialect
+    }
+
     /// The statement, without the white space and the `;` around it.
     pub fn text(&self) -> &str {
         &self.text
     }
 
+    /// The values for its placeholders, in the placeholders' order.
+    pub fn params(&self) -> &[Param] {
+        &self.params
+    }
+
     /// Whether it is an `EXPLAIN` of a query, which answers with a plan
     /// and reads no rows.
     pub fn explains(&self) -> bool {
@@ -88,6 +102,8 @@ impl Read {
 impl fmt::Debug for Read {
     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
         f.debug_struct("Read")
+            .field("dialect", &self.dialect)
+            .field("params", &self.params.len())
             .field("explains", &self.explains)
             .finish_non_exhaustive()
     }
@@ -280,7 +296,9 @@ pub fn check_read(dialect: Dialect, sql: &str, params: &[Param]) -> Result<Read,
     let text = statement.text.as_str();
     let explains = on_a_deep_stack(|| read_parsed(dialect, text, params))??;
     Ok(Read {
+        dialect,
         text: text.to_owned(),
+        params: params.to_vec(),
         explains,
     })
 }
```

- [ ] **Step 4: Run the tests**

Run: `cargo test --locked -p tabletist-db --lib agent::`

Expected: `35 passed`. Run 1 left 34: one is new, and `what_passed_prints_without_its_text` changed with what `Read` prints.

- [ ] **Step 5: Commit**

```bash
cargo fmt --all
cargo clippy --locked -p tabletist-db --all-targets -- -D warnings
git add crates/tabletist-db/src/agent.rs
git commit -m "Keep what an agent's read was checked with"
```

---

### Task 3: MySQL's optimizer hints

A comment that begins `/*+` is an optimizer hint to MySQL, and three hints undo what Task 6 sets for a run's session. Seen on MySQL 8.4.11, with a join that would take minutes and a list of 94 rows:

| The session has | The statement | What happened |
|---|---|---|
| `max_execution_time = 300` | no hint | ended after 0.30 s (error 3024) |
| the same | `SELECT /*+ MAX_EXECUTION_TIME(1500) */ …` | ended after 1.50 s |
| the same | `SELECT /*+ SET_VAR(max_execution_time=1500) */ …` | ended after 1.50 s |
| `sql_select_limit = 3` | no hint | 3 rows |
| the same | `SELECT /*+ SET_VAR(sql_select_limit=100000) */ …` | 94 rows |

So a hint is refused, wherever in the statement it stands, in either reading of the statement's comments. The script guard already refuses MySQL's other comment that is code, `/*! … */`.

**Files:**
- Modify: `crates/tabletist-db/src/agent.rs`

- [ ] **Step 1: Write the failing test**

```diff
diff --git a/crates/tabletist-db/src/agent.rs b/crates/tabletist-db/src/agent.rs
index 1d34baa..9cfde81 100644
--- a/crates/tabletist-db/src/agent.rs
+++ b/crates/tabletist-db/src/agent.rs
@@ -1026,6 +1026,41 @@ mod tests {
         );
     }
 
+    #[test]
+    fn a_mysql_optimizer_hint_is_refused() {
+        // Each of the first three undoes what a run sets for its session.
+        for sql in [
+            "SELECT /*+ MAX_EXECUTION_TIME(600000) */ title FROM books",
+            "SELECT /*+ SET_VAR(max_execution_time = 0) */ title FROM books",
+            "SELECT /*+SET_VAR(sql_select_limit=100000)*/ title FROM books",
+            "SELECT title FROM books WHERE id IN (SELECT /*+ NO_MERGE() */ book_id FROM order_items)",
+            "SELECT title FROM books /*+ not where MySQL reads one */",
+            "EXPLAIN SELECT /*+ MAX_EXECUTION_TIME(600000) */ title FROM books",
+        ] {
+            let refusal = refused(Dialect::MySql, sql);
+            assert_eq!(refusal, not_allowed("a /*+ */ optimizer hint"), "{sql}");
+            assert_eq!(
+                refusal.to_string(),
+                "a /*+ */ optimizer hint is not available to agents."
+            );
+        }
+        // No hint: a comment that only looks like one, and the same
+        // characters in a string and in a line comment.
+        for sql in [
+            "SELECT title FROM books /* + a note */",
+            "SELECT title FROM books /* a note /*+ */",
+            "SELECT '/*+ MAX_EXECUTION_TIME(1) */' FROM books",
+            "SELECT title FROM books -- /*+ MAX_EXECUTION_TIME(1) */",
+        ] {
+            assert!(read(Dialect::MySql, sql).is_ok(), "{sql}");
+        }
+        // To the other two it is a comment.
+        for dialect in [Dialect::Postgres, Dialect::Sqlite] {
+            let sql = "SELECT /*+ SeqScan(books) */ title FROM books";
+            assert!(read(dialect, sql).is_ok(), "{dialect:?}");
+        }
+    }
+
     #[test]
     fn more_than_one_statement_or_none_is_refused() {
         for dialect in ALL {
```

- [ ] **Step 2: Run it to see it fail**

Run: `cargo test --locked -p tabletist-db --lib agent::tests::a_mysql_optimizer_hint_is_refused`

Expected: FAIL, at the first statement of the list.

```
MySql let through: SELECT /*+ MAX_EXECUTION_TIME(600000) */ title FROM books
test result: FAILED. 0 passed; 1 failed
```

- [ ] **Step 3: Refuse the hint**

```diff
diff --git a/crates/tabletist-db/src/agent.rs b/crates/tabletist-db/src/agent.rs
index 9cfde81..06e86df 100644
--- a/crates/tabletist-db/src/agent.rs
+++ b/crates/tabletist-db/src/agent.rs
@@ -21,7 +21,7 @@ use std::fmt;
 use sqlparser::ast::{Query, SetExpr, Statement, UtilityOption};
 use sqlparser::dialect::{MySqlDialect, PostgreSqlDialect, SQLiteDialect};
 use sqlparser::parser::Parser;
-use sqlparser::tokenizer::{Token, Tokenizer};
+use sqlparser::tokenizer::{Token, Tokenizer, Whitespace};
 
 use crate::sql::TokenKind;
 use crate::{Dialect, sql};
@@ -331,6 +331,7 @@ fn read_parsed(dialect: Dialect, text: &str, params: &[Param]) -> Result<bool, R
     let tokens = Tokenizer::new(parser, text)
         .tokenize()
         .map_err(|_| unread())?;
+    hints_read(dialect, &tokens, text)?;
     let statements = Parser::new(parser)
         .with_tokens(tokens.clone())
         .parse_statements()
@@ -378,6 +379,30 @@ fn read_parsed(dialect: Dialect, text: &str, params: &[Param]) -> Result<bool, R
     Ok(explains)
 }
 
+/// Refuses a MySQL statement with an optimizer hint, in either reading of
+/// its comments. A comment that begins `/*+` is one to MySQL, and three
+/// hints undo what a run sets for its session: `SET_VAR(sql_select_limit =
+/// …)` lifts the row limit, and `MAX_EXECUTION_TIME(…)` and
+/// `SET_VAR(max_execution_time = …)` the timeout. Seen on MySQL 8.4. The
+/// same text is a comment and nothing more to PostgreSQL and SQLite.
+fn hints_read(dialect: Dialect, tokens: &[Token], text: &str) -> Result<(), Refusal> {
+    match dialect {
+        Dialect::MySql => {}
+        Dialect::Postgres | Dialect::Sqlite => return Ok(()),
+    }
+    let parsed = tokens.iter().any(|token| match token {
+        Token::Whitespace(Whitespace::MultiLineComment(body)) => body.starts_with('+'),
+        _ => false,
+    });
+    let own = sql::tokenize(dialect, text).iter().any(|token| {
+        matches!(token.kind, TokenKind::Comment) && text[token.range.clone()].starts_with("/*+")
+    });
+    if parsed || own {
+        return Err(Refusal::NotAllowed("a /*+ */ optimizer hint".into()));
+    }
+    Ok(())
+}
+
 /// Text the parser could not read. What does not even begin as a query is
 /// named, so the agent learns that the statement is the trouble, not its
 /// spelling: sqlparser does not read every statement a database takes
```

- [ ] **Step 4: Run the tests**

Run: `cargo test --locked -p tabletist-db --lib agent::`

Expected: `36 passed`.

- [ ] **Step 5: Commit**

```bash
cargo fmt --all
cargo clippy --locked -p tabletist-db --all-targets -- -D warnings
git add crates/tabletist-db/src/agent.rs
git commit -m "Refuse MySQL's optimizer hints in an agent's query"
```

---

### Task 4: The run, and SQLite

`Connection::run_agent_read`, what it takes (`Limits`) and gives (`Answer`), what the three drivers share (`agent/run.rs`), and SQLite's side of it. PostgreSQL and MySQL answer `Error::Unsupported` until their own tasks: each commit does what it says and no more.

SQLite's side is small. rusqlite binds the values, each as its own type: text stays text, where a filter's typed text that reads as a number becomes one (`to_sqlite`). The deadline is one more condition in the progress handler that already watches for a stop.

The tests every engine runs are written here, as functions that take a connection: `an_agent_reads_the_bookshop` calls them all. A test statement is written with `?`, and `placed` spells it `$1`, `$2` for PostgreSQL.

**Files:**
- Create: `crates/tabletist-db/src/agent/run.rs`
- Modify: `crates/tabletist-db/src/agent.rs`, `adapter.rs`, `lib.rs`, `sqlite.rs`, `pg.rs`, `mysql.rs`
- Test: `crates/tabletist-db/tests/agent.rs`, and `sqlite.rs`'s own

- [ ] **Step 1: Write the failing tests**

```diff
diff --git a/crates/tabletist-db/src/sqlite.rs b/crates/tabletist-db/src/sqlite.rs
index 59f8fdd..f823459 100644
--- a/crates/tabletist-db/src/sqlite.rs
+++ b/crates/tabletist-db/src/sqlite.rs
@@ -2587,4 +2587,47 @@ mod tests {
         let result = conn.describe(&ObjectRef::new("main", "nope")).await;
         assert!(matches!(result, Err(Error::Query { .. })), "{result:?}");
     }
+
+    #[tokio::test]
+    async fn sqlites_own_count_of_placeholders_is_the_one_that_holds() {
+        let (conn, _dir) = fixture().await;
+        let one = [Param::Int(1)];
+        let ran = conn
+            .run(move |connection| statement(connection, "SELECT ?, ?", 10, Some(&one)))
+            .await;
+        assert_eq!(ran, Err(count_mismatch(2, 1)));
+        let ran = conn
+            .run(|connection| statement(connection, "SELECT 1", 10, Some(&[Param::Null])))
+            .await;
+        assert_eq!(ran, Err(count_mismatch(0, 1)));
+        // The SQL editor sends no values, and its statement is not
+        // counted: SQLite reads a placeholder without one as NULL.
+        let ran = conn
+            .run(|connection| statement(connection, "SELECT ?, ?", 10, None))
+            .await
+            .unwrap();
+        let StatementOutcome::Rows { rows, .. } = ran else {
+            panic!("{ran:?}");
+        };
+        assert_eq!(rows, [[Value::Null, Value::Null]]);
+    }
+
+    #[test]
+    fn an_agents_value_is_bound_as_its_own_type() {
+        use rusqlite::types::Value as Sqlite;
+        assert_eq!(bound(&Param::Null), Sqlite::Null);
+        assert_eq!(bound(&Param::Bool(true)), Sqlite::Integer(1));
+        assert_eq!(bound(&Param::Int(-7)), Sqlite::Integer(-7));
+        assert_eq!(bound(&Param::Float(1.5)), Sqlite::Real(1.5));
+        // Not the number a filter's typed text would become.
+        assert_eq!(bound(&Param::Text("5".into())), Sqlite::Text("5".into()));
+    }
+
+    #[test]
+    fn a_deadline_too_far_for_the_clock_never_comes() {
+        let start = Instant::now();
+        let second = Duration::from_secs(1);
+        assert_eq!(deadline(start, second), Some(start + second));
+        assert_eq!(deadline(start, Duration::MAX), None);
+    }
 }
diff --git a/crates/tabletist-db/tests/agent.rs b/crates/tabletist-db/tests/agent.rs
index e9b76c4..01401e8 100644
--- a/crates/tabletist-db/tests/agent.rs
+++ b/crates/tabletist-db/tests/agent.rs
@@ -7,9 +7,12 @@
 // `allow-unwrap-in-tests` does not cover helpers in an integration-test crate.
 #![allow(clippy::unwrap_used)]
 
+use std::time::{Duration, Instant};
+
+use tabletist_db::agent::{self, Answer, Limits, Param};
 use tabletist_db::{
-    Access, ConnectSpec, Connection, Dialect, HostKeys, ScriptMode, Secrets, StatementOutcome,
-    StopFlag, Value, sql,
+    Access, ConnectSpec, Connection, Dialect, Error, HostKeys, ScriptMode, Secrets,
+    StatementOutcome, StopFlag, Value, sql,
 };
 
 /// How many rows each table and view of the Bookshop has.
@@ -54,6 +57,243 @@ async fn the_bookshop_is_stocked(connection: &Connection, on: &str) {
     }
 }
 
+/// What a test gives a read that should not run out of either.
+const LIMITS: Limits = Limits {
+    rows: 100,
+    timeout: Duration::from_secs(10),
+};
+
+/// A read that takes long without sleeping: every way to pick one of the
+/// 40 order items `copies` times over, each weighed and none kept. Five
+/// copies are a hundred million rows, which no test waits for. Four are
+/// under three million, and are weighed to the end.
+fn slow(copies: usize) -> String {
+    let tables: Vec<String> = (1..=copies)
+        .map(|copy| format!("order_items t{copy}"))
+        .collect();
+    let weights: Vec<String> = (1..=copies)
+        .map(|copy| format!("t{copy}.quantity"))
+        .collect();
+    format!(
+        "SELECT count(*) FROM {} WHERE {} < 0",
+        tables.join(", "),
+        weights.join(" + ")
+    )
+}
+
+fn text(text: &str) -> Value {
+    Value::Text(text.into())
+}
+
+/// `sql` with each `?` as the connection's database spells a placeholder:
+/// `$1`, `$2` and so on for PostgreSQL.
+fn placed(connection: &Connection, sql: &str) -> String {
+    match connection.dialect() {
+        Dialect::MySql | Dialect::Sqlite => sql.to_owned(),
+        Dialect::Postgres => {
+            let mut number = 0;
+            sql.chars()
+                .map(|character| match character {
+                    '?' => {
+                        number += 1;
+                        format!("${number}")
+                    }
+                    other => other.to_string(),
+                })
+                .collect()
+        }
+    }
+}
+
+/// What the connection's database answers an agent that sends `sql`,
+/// written with `?`, and `params`. The statement must pass the check.
+async fn asked(
+    connection: &Connection,
+    sql: &str,
+    params: &[Param],
+    limits: Limits,
+) -> Result<Answer, Error> {
+    let sql = placed(connection, sql);
+    let read = agent::check_read(connection.dialect(), &sql, params)
+        .unwrap_or_else(|refusal| panic!("{sql}: {refusal}"));
+    connection
+        .run_agent_read(&read, limits, &StopFlag::new())
+        .await
+}
+
+/// The rows of an answer that must come.
+async fn read(connection: &Connection, sql: &str, params: &[Param], on: &str) -> Vec<Vec<Value>> {
+    asked(connection, sql, params, LIMITS)
+        .await
+        .unwrap_or_else(|error| panic!("{on}: {sql}: {error}"))
+        .rows
+}
+
+async fn a_read_answers_its_rows(connection: &Connection, on: &str) {
+    the_bookshop_is_stocked(connection, on).await;
+    // The first rows up to the limit, and whether there were more.
+    let books = "SELECT id, title FROM books ORDER BY id";
+    let five = Limits { rows: 5, ..LIMITS };
+    let answer = asked(connection, books, &[], five).await.unwrap();
+    let names: Vec<&str> = answer
+        .columns
+        .iter()
+        .map(|column| column.name.as_str())
+        .collect();
+    assert_eq!(names, ["id", "title"], "{on}");
+    assert_eq!(answer.rows.len(), 5, "{on}");
+    assert_eq!(
+        answer.rows[0],
+        [Value::Int(1), text("The Salt Road")],
+        "{on}"
+    );
+    assert!(answer.truncated, "{on}");
+    let answer = asked(connection, books, &[], LIMITS).await.unwrap();
+    assert_eq!(answer.rows.len(), 12, "{on}");
+    assert!(!answer.truncated, "{on}");
+    // A view is read as a table is.
+    let recent = read(connection, "SELECT count(*) FROM recent_orders", &[], on).await;
+    assert_eq!(recent, [[Value::Int(7)]], "{on}");
+    // A plan is an answer too.
+    let plan = read(connection, "EXPLAIN SELECT title FROM books", &[], on).await;
+    assert!(!plan.is_empty(), "{on}");
+    the_bookshop_is_stocked(connection, on).await;
+}
+
+async fn values_are_bound_to_their_placeholders(connection: &Connection, on: &str) {
+    let found = read(
+        connection,
+        "SELECT title FROM books WHERE id = ?",
+        &[Param::Int(3)],
+        on,
+    )
+    .await;
+    assert_eq!(found, [[text("Lanterns")]], "{on}");
+    // Each value goes to its own placeholder, in their order.
+    let found = read(
+        connection,
+        "SELECT title FROM books WHERE publisher_id = ? AND price < ? ORDER BY id",
+        &[Param::Int(1), Param::Float(20.0)],
+        on,
+    )
+    .await;
+    assert_eq!(found, [[text("The Salt Road")]], "{on}");
+    // Text arrives as it was sent, its quote and its letters.
+    for (name, id) in [("N. K. O'Dell", 6), ("Zoë Lindqvist", 5)] {
+        let found = read(
+            connection,
+            "SELECT id, name FROM authors WHERE name = ?",
+            &[Param::Text(name.into())],
+            on,
+        )
+        .await;
+        assert_eq!(found, [[Value::Int(id), text(name)]], "{on}");
+    }
+    // A value is compared, never read: SQL in one finds no title.
+    for value in [
+        "x' OR '1'='1",
+        "'; DROP TABLE books; --",
+        "\\'; DELETE FROM books; --",
+        "Lanterns' --",
+        "$1",
+        "?",
+    ] {
+        let found = read(
+            connection,
+            "SELECT count(*) FROM books WHERE title = ?",
+            &[Param::Text(value.into())],
+            on,
+        )
+        .await;
+        assert_eq!(found, [[Value::Int(0)]], "{on}: {value}");
+    }
+    // The other kinds of value.
+    for (sql, param, count) in [
+        (
+            "SELECT count(*) FROM books WHERE in_print = ?",
+            Param::Bool(false),
+            2,
+        ),
+        (
+            "SELECT count(*) FROM books WHERE price > ?",
+            Param::Float(21.5),
+            5,
+        ),
+        (
+            "SELECT count(*) FROM books WHERE published > ?",
+            Param::Null,
+            0,
+        ),
+    ] {
+        let found = read(connection, sql, &[param], on).await;
+        assert_eq!(found, [[Value::Int(count)]], "{on}: {sql}");
+    }
+    the_bookshop_is_stocked(connection, on).await;
+}
+
+async fn a_read_that_takes_too_long_is_ended_by_the_database(connection: &Connection, on: &str) {
+    let short = Limits {
+        timeout: Duration::from_millis(200),
+        ..LIMITS
+    };
+    let started = Instant::now();
+    let ended = asked(connection, &slow(5), &[], short).await;
+    assert_eq!(ended.unwrap_err(), Error::Timeout, "{on}");
+    let took = started.elapsed();
+    assert!(took < Duration::from_secs(5), "{on}: {took:?}");
+    // The session is as it was: the next read has its own time, and
+    // answers.
+    let found = read(connection, "SELECT count(*) FROM books", &[], on).await;
+    assert_eq!(found, [[Value::Int(12)]], "{on}");
+    the_bookshop_is_stocked(connection, on).await;
+}
+
+async fn a_read_that_is_stopped_or_fails_says_so(connection: &Connection, on: &str) {
+    // Stopped before it began.
+    let stopped = StopFlag::new();
+    stopped.stop();
+    let books = agent::check_read(connection.dialect(), "SELECT title FROM books", &[]).unwrap();
+    let ended = connection.run_agent_read(&books, LIMITS, &stopped).await;
+    assert_eq!(ended.unwrap_err(), Error::Cancelled, "{on}");
+    // The database's own error, in its words.
+    let error = asked(connection, "SELECT nope FROM books", &[], LIMITS)
+        .await
+        .unwrap_err();
+    assert!(matches!(error, Error::Query { .. }), "{on}: {error:?}");
+    assert!(error.to_string().contains("nope"), "{on}: {error}");
+    // Neither ends the session.
+    let found = read(connection, "SELECT count(*) FROM books", &[], on).await;
+    assert_eq!(found, [[Value::Int(12)]], "{on}");
+    the_bookshop_is_stocked(connection, on).await;
+}
+
+async fn a_read_checked_for_another_database_is_not_run(connection: &Connection, on: &str) {
+    let other = match connection.dialect() {
+        Dialect::Postgres => Dialect::MySql,
+        Dialect::MySql => Dialect::Sqlite,
+        Dialect::Sqlite => Dialect::Postgres,
+    };
+    let read = agent::check_read(other, "SELECT title FROM books", &[]).unwrap();
+    let error = connection
+        .run_agent_read(&read, LIMITS, &StopFlag::new())
+        .await
+        .unwrap_err();
+    assert_eq!(
+        error.to_string(),
+        "the statement was checked for another kind of database",
+        "{on}"
+    );
+}
+
+/// What every engine is asked.
+async fn an_agent_reads_the_bookshop(connection: &Connection, on: &str) {
+    a_read_answers_its_rows(connection, on).await;
+    values_are_bound_to_their_placeholders(connection, on).await;
+    a_read_that_takes_too_long_is_ended_by_the_database(connection, on).await;
+    a_read_that_is_stopped_or_fails_says_so(connection, on).await;
+    a_read_checked_for_another_database_is_not_run(connection, on).await;
+}
+
 mod sqlite {
     use super::*;
 
@@ -79,6 +319,72 @@ mod sqlite {
         assert_eq!(connection.dialect(), Dialect::Sqlite);
         the_bookshop_is_stocked(&connection, "sqlite").await;
     }
+
+    #[tokio::test]
+    async fn an_agent_reads_the_bookshop_on_sqlite() {
+        let (connection, _dir) = bookshop().await;
+        an_agent_reads_the_bookshop(&connection, "sqlite").await;
+    }
+
+    #[tokio::test]
+    async fn sqlite_takes_each_value_as_its_own_type() {
+        let (connection, _dir) = bookshop().await;
+        let types = read(
+            &connection,
+            "SELECT typeof(?), typeof(?), typeof(?), typeof(?), typeof(?)",
+            &[
+                Param::Null,
+                Param::Bool(true),
+                Param::Int(5),
+                Param::Float(1.5),
+                // Text that reads as a number is still text.
+                Param::Text("5".into()),
+            ],
+            "sqlite",
+        )
+        .await;
+        let expected = ["null", "integer", "integer", "real", "text"].map(text);
+        assert_eq!(types, [expected]);
+        // The ends of a 64-bit integer come back as they went.
+        let ends = read(
+            &connection,
+            "SELECT ?, ?",
+            &[Param::Int(i64::MAX), Param::Int(i64::MIN)],
+            "sqlite",
+        )
+        .await;
+        assert_eq!(ends, [[Value::Int(i64::MAX), Value::Int(i64::MIN)]]);
+    }
+
+    #[tokio::test]
+    async fn a_question_mark_in_a_string_is_text_on_sqlite() {
+        let (connection, _dir) = bookshop().await;
+        let found = read(
+            &connection,
+            "SELECT 'where?', title FROM books WHERE id = ?",
+            &[Param::Int(3)],
+            "sqlite",
+        )
+        .await;
+        assert_eq!(found, [[text("where?"), text("Lanterns")]]);
+    }
+
+    /// The run's deadline is taken off with the run: the SQL editor's next
+    /// script on the session has no time limit of the driver's.
+    #[tokio::test]
+    async fn a_deadline_ends_with_its_run_on_sqlite() {
+        let (connection, _dir) = bookshop().await;
+        let short = Limits {
+            timeout: Duration::from_millis(1),
+            ..LIMITS
+        };
+        let ended = asked(&connection, &slow(5), &[], short).await;
+        assert_eq!(ended.unwrap_err(), Error::Timeout);
+        // Four copies take longer than that millisecond, and are weighed
+        // to the end.
+        let counted = rows_of(&connection, &slow(4), "sqlite").await;
+        assert_eq!(counted, [[Value::Int(0)]]);
+    }
 }
 
 mod postgres {
```

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test --locked -p tabletist-db --test agent`

Expected: it does not compile. Nothing runs an agent's read yet:

```
error[E0432]: unresolved imports `tabletist_db::agent::Answer`, `tabletist_db::agent::Limits`
error[E0599]: no method named `run_agent_read` found for reference `&tabletist_db::Connection` in the current scope
```

The library's own tests do not compile either (`cargo test --locked -p tabletist-db --lib sqlite::`): `Param`, `count_mismatch`, `bound` and `deadline` are not in `sqlite.rs`, and `statement` takes three arguments.

- [ ] **Step 3: The run, and SQLite's side of it**

```diff
diff --git a/crates/tabletist-db/src/adapter.rs b/crates/tabletist-db/src/adapter.rs
index 3a60357..7e11d1c 100644
--- a/crates/tabletist-db/src/adapter.rs
+++ b/crates/tabletist-db/src/adapter.rs
@@ -1,5 +1,6 @@
 //! What the adapter of every database provides.
 
+use crate::agent::{Limits, Read};
 use crate::{
     CancelHandle, ChangeSet, Lookup, LookupPage, ObjectInfo, ObjectRef, Result, RowPage, RowQuery,
     ScriptMode, ScriptOutcome, StopFlag, Structure, WriteOutcome,
@@ -41,6 +42,11 @@ pub(crate) trait Adapter {
         stop: &StopFlag,
     ) -> Result<ScriptOutcome>;
 
+    /// `read` has passed [`crate::agent::check_read`] for this database's
+    /// dialect. The outcome is a read-only script's of one statement.
+    async fn run_read(&self, read: &Read, limits: Limits, stop: &StopFlag)
+    -> Result<ScriptOutcome>;
+
     /// `changes` have passed their own check, and the session may write.
     async fn write(&self, changes: &ChangeSet, stop: &StopFlag) -> Result<WriteOutcome>;
 }
diff --git a/crates/tabletist-db/src/agent.rs b/crates/tabletist-db/src/agent.rs
index 06e86df..bc6da88 100644
--- a/crates/tabletist-db/src/agent.rs
+++ b/crates/tabletist-db/src/agent.rs
@@ -11,10 +11,12 @@
 //! [`Refusal`] is why: a code an agent may match on, and a sentence it can
 //! act on.
 //!
-//! The check is the first fence, not the only one. What passes it runs as
-//! any read-only script does ([`crate::ScriptMode::ReadOnly`]): prepared
-//! alone by the database, in a read-only transaction that is rolled back.
-//! A write this check took for a read is refused there.
+//! The check is the first fence, not the only one. What passes it runs
+//! through [`crate::Connection::run_agent_read`], as any read-only script
+//! does ([`crate::ScriptMode::ReadOnly`]): prepared alone by the database,
+//! in a read-only transaction that is rolled back. A write this check took
+//! for a read is refused there. [`Limits`] is how much a read may take,
+//! and [`Answer`] what it gave.
 
 use std::fmt;
 
@@ -26,6 +28,11 @@ use sqlparser::tokenizer::{Token, Tokenizer, Whitespace};
 use crate::sql::TokenKind;
 use crate::{Dialect, sql};
 
+mod run;
+
+pub use run::{Answer, Limits};
+pub(crate) use run::{Sent, answer, count_mismatch};
+
 /// The longest statement that is read at all, in bytes.
 pub const MAX_SQL_BYTES: usize = 100_000;
 
diff --git a/crates/tabletist-db/src/agent/run.rs b/crates/tabletist-db/src/agent/run.rs
new file mode 100644
index 0000000..811a785
--- /dev/null
+++ b/crates/tabletist-db/src/agent/run.rs
@@ -0,0 +1,203 @@
+//! Running a statement that passed the check: how much it may take, and
+//! what it answered.
+
+use std::fmt;
+use std::time::Duration;
+
+use super::Param;
+use crate::{ColumnMeta, Error, Result, ScriptOutcome, StatementOutcome, StopFlag, Value};
+
+/// How much one read of an agent's may take.
+#[derive(Debug, Clone, Copy, PartialEq, Eq)]
+pub struct Limits {
+    /// The most rows of an answer.
+    pub rows: u32,
+    /// How long the database lets the statement run, by its own clock. A
+    /// time under a millisecond is taken for one: PostgreSQL and MySQL
+    /// read 0 as no limit.
+    pub timeout: Duration,
+}
+
+/// What a read answered.
+#[derive(Clone, PartialEq)]
+pub struct Answer {
+    pub columns: Vec<ColumnMeta>,
+    /// At most [`Limits::rows`] of them.
+    pub rows: Vec<Vec<Value>>,
+    /// Whether the statement had more rows than were kept.
+    pub truncated: bool,
+    /// How long the statement took.
+    pub elapsed: Duration,
+}
+
+/// Without the rows: what a database holds is not logged.
+impl fmt::Debug for Answer {
+    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
+        f.debug_struct("Answer")
+            .field("columns", &self.columns.len())
+            .field("rows", &self.rows.len())
+            .field("truncated", &self.truncated)
+            .field("elapsed", &self.elapsed)
+            .finish()
+    }
+}
+
+/// What a driver's read-only run takes beyond a script's when its one
+/// statement is an agent's: the values for the statement's placeholders,
+/// and how long the database may take over it.
+#[derive(Debug, Clone, Copy)]
+pub(crate) struct Sent<'a> {
+    pub params: &'a [Param],
+    pub timeout: Duration,
+}
+
+/// The error of a statement the database counts another number of
+/// placeholders in than values were sent for. The check counted them too
+/// ([`super::check_read`]), by its own reading: where the two disagree the
+/// database's count is the one that holds, and nothing runs.
+pub(crate) fn count_mismatch(counted: usize, sent: usize) -> Error {
+    let plural = |count: usize| if count == 1 { "" } else { "s" };
+    Error::query(format!(
+        "the database counts {counted} parameter{} in the statement, and {sent} value{} came \
+         with it",
+        plural(counted),
+        plural(sent)
+    ))
+}
+
+/// The answer in the outcome of a read-only run of one statement, or why
+/// there is none: the statement's own error, [`Error::Cancelled`] when
+/// `stop` ended it, and [`Error::Timeout`] when the database did.
+pub(crate) fn answer(outcome: ScriptOutcome, stop: &StopFlag) -> Result<Answer> {
+    let Some(result) = outcome.results.into_iter().next() else {
+        // A cancel landed before the statement began.
+        return Err(Error::Cancelled);
+    };
+    match result.outcome {
+        StatementOutcome::Rows {
+            columns,
+            rows,
+            truncated,
+        } => Ok(Answer {
+            columns,
+            rows,
+            truncated,
+            elapsed: result.elapsed,
+        }),
+        // The check lets through a query or an `EXPLAIN` of one, and both
+        // answer rows. In the read-only transaction this wrote nothing.
+        StatementOutcome::Done { .. } => Err(Error::query(
+            "the statement answered no rows, so it is not a read",
+        )),
+        StatementOutcome::Error { error, .. } => Err(error),
+        StatementOutcome::Cancelled if stop.is_stopped() => Err(Error::Cancelled),
+        // Nobody here stopped it, so the database's own timeout did:
+        // PostgreSQL ends a statement over its time as it ends a cancelled
+        // one, and SQLite's is interrupted by the run's deadline. A
+        // statement someone ended at the server reads the same.
+        StatementOutcome::Cancelled => Err(Error::Timeout),
+    }
+}
+
+#[cfg(test)]
+mod tests {
+    use super::*;
+    use crate::{StatementResult, ValueKind};
+
+    fn outcome_of(outcome: StatementOutcome) -> ScriptOutcome {
+        ScriptOutcome {
+            results: vec![StatementResult {
+                elapsed: Duration::from_millis(7),
+                outcome,
+            }],
+            ..ScriptOutcome::default()
+        }
+    }
+
+    fn titles() -> StatementOutcome {
+        StatementOutcome::Rows {
+            columns: vec![ColumnMeta {
+                name: "title".into(),
+                type_name: "text".into(),
+                kind: ValueKind::Text,
+            }],
+            rows: vec![vec![Value::Text("Lanterns".into())]],
+            truncated: true,
+        }
+    }
+
+    #[test]
+    fn rows_are_the_answer() {
+        let answer = answer(outcome_of(titles()), &StopFlag::new()).unwrap();
+        assert_eq!(answer.columns[0].name, "title");
+        assert_eq!(answer.rows, [[Value::Text("Lanterns".into())]]);
+        assert!(answer.truncated);
+        assert_eq!(answer.elapsed, Duration::from_millis(7));
+    }
+
+    #[test]
+    fn an_answer_prints_without_its_rows() {
+        let answer = answer(outcome_of(titles()), &StopFlag::new()).unwrap();
+        assert_eq!(
+            format!("{answer:?}"),
+            "Answer { columns: 1, rows: 1, truncated: true, elapsed: 7ms }"
+        );
+    }
+
+    #[test]
+    fn a_statement_that_failed_is_its_error() {
+        let failed = StatementOutcome::Error {
+            error: Error::query("no such column: nope"),
+            position: None,
+        };
+        assert_eq!(
+            answer(outcome_of(failed), &StopFlag::new()),
+            Err(Error::query("no such column: nope"))
+        );
+    }
+
+    #[test]
+    fn a_statement_that_was_ended_says_who_ended_it() {
+        // Nobody stopped it, so the database's own timeout did.
+        assert_eq!(
+            answer(outcome_of(StatementOutcome::Cancelled), &StopFlag::new()),
+            Err(Error::Timeout)
+        );
+        let stopped = StopFlag::new();
+        stopped.stop();
+        assert_eq!(
+            answer(outcome_of(StatementOutcome::Cancelled), &stopped),
+            Err(Error::Cancelled)
+        );
+        // A run that was ended before its statement began has no result.
+        assert_eq!(
+            answer(ScriptOutcome::default(), &stopped),
+            Err(Error::Cancelled)
+        );
+    }
+
+    #[test]
+    fn a_statement_without_rows_is_no_read() {
+        let done = StatementOutcome::Done {
+            affected: None,
+            warnings: 0,
+        };
+        let error = answer(outcome_of(done), &StopFlag::new()).unwrap_err();
+        assert_eq!(
+            error.to_string(),
+            "the statement answered no rows, so it is not a read"
+        );
+    }
+
+    #[test]
+    fn a_count_the_database_disagrees_with_is_said_in_its_numbers() {
+        assert_eq!(
+            count_mismatch(1, 2).to_string(),
+            "the database counts 1 parameter in the statement, and 2 values came with it"
+        );
+        assert_eq!(
+            count_mismatch(2, 1).to_string(),
+            "the database counts 2 parameters in the statement, and 1 value came with it"
+        );
+    }
+}
diff --git a/crates/tabletist-db/src/lib.rs b/crates/tabletist-db/src/lib.rs
index 015491a..145412a 100644
--- a/crates/tabletist-db/src/lib.rs
+++ b/crates/tabletist-db/src/lib.rs
@@ -6,7 +6,8 @@
 //! session opened [`Access::Writable`]. Everything else stays fenced there
 //! as on a read-only session: row fetches, counts and lookups run in read-only
 //! transactions (on SQLite under `query_only`), and a script run in
-//! [`ScriptMode::ReadOnly`] cannot write.
+//! [`ScriptMode::ReadOnly`] cannot write. Neither can
+//! [`Connection::run_agent_read`], which is such a run of one statement.
 
 mod adapter;
 pub mod agent;
@@ -223,6 +224,42 @@ impl Connection {
         }
     }
 
+    /// Runs a statement that passed [`agent::check_read`], as a script of
+    /// one statement runs in [`ScriptMode::ReadOnly`]: prepared alone, in
+    /// a read-only transaction that is rolled back, keeping at most
+    /// `limits.rows` rows.
+    ///
+    /// Two things a script does not have. The values the statement was
+    /// checked with are bound to its placeholders by the database's own
+    /// protocol, and are never written into its text. And the database
+    /// itself ends the statement after `limits.timeout`, which answers
+    /// [`Error::Timeout`]; a caller keeps a timer of its own behind it, as
+    /// for any script.
+    ///
+    /// A statement that failed is its error, and one `stop` ended is
+    /// [`Error::Cancelled`]. A `read` that was checked for another
+    /// dialect than this session's is refused, with nothing sent.
+    ///
+    /// The future must be awaited to its end, as [`Self::run_script`]'s.
+    pub async fn run_agent_read(
+        &self,
+        read: &agent::Read,
+        limits: agent::Limits,
+        stop: &StopFlag,
+    ) -> Result<agent::Answer> {
+        if read.dialect() != self.dialect() {
+            return Err(Error::query(
+                "the statement was checked for another kind of database",
+            ));
+        }
+        let outcome = match &self.inner {
+            Inner::Sqlite(conn) => conn.run_read(read, limits, stop).await,
+            Inner::Postgres(conn) => conn.run_read(read, limits, stop).await,
+            Inner::MySql(conn) => conn.run_read(read, limits, stop).await,
+        }?;
+        agent::answer(outcome, stop)
+    }
+
     /// The server's name and version for the footer: `PostgreSQL 17.2`,
     /// `MySQL 8.4.3`, `MariaDB 10.11.6`, `SQLite 3.46.0`.
     pub async fn server_version(&self) -> Result<String> {
diff --git a/crates/tabletist-db/src/mysql.rs b/crates/tabletist-db/src/mysql.rs
index 92f3612..71a6058 100644
--- a/crates/tabletist-db/src/mysql.rs
+++ b/crates/tabletist-db/src/mysql.rs
@@ -20,6 +20,7 @@ use mysql_async::{DriverError, IoError, Opts, OptsBuilder, Params, SslOpts, TxOp
 use mysql_common::named_params::ParsedNamedParams;
 
 use crate::adapter::Adapter;
+use crate::agent::{Limits, Read};
 use crate::script::retry_cancelled;
 use crate::{
     Access, CancelHandle, CancelInner, ChangeSet, ColumnClass, ColumnInfo, ColumnMeta, ConnectSpec,
@@ -473,6 +474,15 @@ impl Adapter for Conn {
         self.script(&texts, limit, mode, stop).await
     }
 
+    async fn run_read(
+        &self,
+        _read: &Read,
+        _limits: Limits,
+        _stop: &StopFlag,
+    ) -> Result<ScriptOutcome> {
+        Err(Error::Unsupported("an agent's read on MySQL"))
+    }
+
     async fn write(&self, changes: &ChangeSet, stop: &StopFlag) -> Result<WriteOutcome> {
         self.save(changes, stop).await
     }
diff --git a/crates/tabletist-db/src/pg.rs b/crates/tabletist-db/src/pg.rs
index 92a7185..4b7ed10 100644
--- a/crates/tabletist-db/src/pg.rs
+++ b/crates/tabletist-db/src/pg.rs
@@ -12,6 +12,7 @@ use tokio_postgres::tls::{MakeTlsConnect, TlsConnect};
 use tokio_postgres::{SimpleQueryMessage, Socket};
 
 use crate::adapter::Adapter;
+use crate::agent::{Limits, Read};
 use crate::{
     Access, CancelHandle, CancelInner, ChangeSet, ColumnInfo, ColumnMeta, ConnectSpec, Dialect,
     Error, ForeignKeyInfo, IndexInfo, Lookup, LookupPage, MAX_LISTED, Named, ObjectInfo,
@@ -788,6 +789,15 @@ impl Adapter for Conn {
         self.script(&texts, limit, mode, stop).await
     }
 
+    async fn run_read(
+        &self,
+        _read: &Read,
+        _limits: Limits,
+        _stop: &StopFlag,
+    ) -> Result<ScriptOutcome> {
+        Err(Error::Unsupported("an agent's read on PostgreSQL"))
+    }
+
     async fn write(&self, changes: &ChangeSet, stop: &StopFlag) -> Result<WriteOutcome> {
         self.save(changes, stop).await
     }
diff --git a/crates/tabletist-db/src/sqlite.rs b/crates/tabletist-db/src/sqlite.rs
index f823459..95f507d 100644
--- a/crates/tabletist-db/src/sqlite.rs
+++ b/crates/tabletist-db/src/sqlite.rs
@@ -6,13 +6,14 @@
 use std::borrow::Cow;
 use std::path::Path;
 use std::sync::{Arc, Mutex};
-use std::time::Instant;
+use std::time::{Duration, Instant};
 
 use rusqlite::config::DbConfig;
 use rusqlite::types::{FromSql, FromSqlError, FromSqlResult, ValueRef};
 use rusqlite::{ErrorCode, OpenFlags, OptionalExtension};
 
 use crate::adapter::Adapter;
+use crate::agent::{Limits, Param, Read, Sent, count_mismatch};
 use crate::script::{cancelled_commit, cleanup_failed};
 use crate::{
     Access, CancelHandle, CancelInner, ChangeSet, ColumnClass, ColumnInfo, ColumnMeta, Dialect,
@@ -152,6 +153,10 @@ fn begin(connection: &rusqlite::Connection, mode: ScriptMode) -> Result<()> {
 /// is a `COMMIT` when every statement succeeded (see `end_write`). One
 /// that finds its transaction gone before a statement ends with
 /// `LeftTransaction`.
+///
+/// `sent` is there when the run is of an agent's one statement: its values
+/// are bound, and the run has a deadline, after which the statement that
+/// runs is interrupted as a stop interrupts it.
 fn script(
     connection: &rusqlite::Connection,
     fences: &Fences,
@@ -159,6 +164,7 @@ fn script(
     limit: usize,
     mode: ScriptMode,
     stop: &StopFlag,
+    sent: Option<Sent<'_>>,
 ) -> Result<ScriptOutcome> {
     let mut outcome = ScriptOutcome::default();
     if stop.is_stopped() {
@@ -188,9 +194,24 @@ fn script(
     }
     // SQLite calls this every 1000 virtual machine steps; `true` interrupts
     // the running statement, which fails with SQLITE_INTERRUPT (Cancelled).
+    // The deadline is SQLite's statement timeout: it has no other.
     let watching = stop.clone();
-    connection.progress_handler(1_000, Some(move || watching.is_stopped()));
-    let ran = statements(connection, fences, texts, limit, mode, stop, &mut outcome);
+    let deadline = sent.and_then(|sent| deadline(Instant::now(), sent.timeout));
+    connection.progress_handler(
+        1_000,
+        Some(move || watching.is_stopped() || deadline.is_some_and(|at| Instant::now() >= at)),
+    );
+    let params = sent.map(|sent| sent.params);
+    let ran = statements(
+        connection,
+        fences,
+        texts,
+        limit,
+        mode,
+        stop,
+        params,
+        &mut outcome,
+    );
     // Removed before the cleanup, so a stop cannot interrupt it.
     connection.progress_handler(0, None::<fn() -> bool>);
     if mode == ScriptMode::Write {
@@ -287,6 +308,13 @@ fn end_transaction(connection: &rusqlite::Connection) -> Result<()> {
     attempt().or_else(|_| attempt()).map_err(map_error)
 }
 
+/// When a run that began at `start` is over its `timeout`: `None` for a
+/// time so long that the clock cannot hold its end, which never comes.
+fn deadline(start: Instant, timeout: Duration) -> Option<Instant> {
+    start.checked_add(timeout)
+}
+
+#[allow(clippy::too_many_arguments)] // one call site
 fn statements(
     connection: &rusqlite::Connection,
     fences: &Fences,
@@ -294,6 +322,7 @@ fn statements(
     limit: usize,
     mode: ScriptMode,
     stop: &StopFlag,
+    params: Option<&[Param]>,
     outcome: &mut ScriptOutcome,
 ) -> Result<()> {
     for text in texts {
@@ -339,7 +368,7 @@ fn statements(
         // rollback after the run are the app's.
         let result = {
             let _fenced = fences.fence(Fence::Script);
-            statement(connection, text, limit)
+            statement(connection, text, limit, params)
         };
         let result = match result {
             Ok(result) => result,
@@ -382,10 +411,14 @@ fn counts_changes(text: &str) -> bool {
         .is_some_and(|word| matches!(word.as_str(), "INSERT" | "UPDATE" | "DELETE" | "REPLACE"))
 }
 
+/// Runs one statement. `params` are an agent's values for its
+/// placeholders: the SQL editor has none, and SQLite reads a placeholder
+/// without a value as NULL.
 fn statement(
     connection: &rusqlite::Connection,
     text: &str,
     limit: usize,
+    params: Option<&[Param]>,
 ) -> Result<StatementOutcome> {
     // SQLite names an unaliased column after its text, so a comment after
     // the last token would end up in the name.
@@ -402,6 +435,18 @@ fn statement(
     // for the first, and the run's checks cover what a pragma that got past
     // would have done by then.
     let mut statement = connection.prepare(text).map_err(map_error)?;
+    if let Some(params) = params {
+        // SQLite's own count, which is the one that holds.
+        let counted = statement.parameter_count();
+        if counted != params.len() {
+            return Err(count_mismatch(counted, params.len()));
+        }
+        for (index, param) in params.iter().enumerate() {
+            statement
+                .raw_bind_parameter(index + 1, bound(param))
+                .map_err(map_error)?;
+        }
+    }
     let declared = declared_columns(connection, &statement, text)?;
     if declared.is_empty() {
         let changed = statement.raw_execute().map_err(map_error)?;
@@ -431,6 +476,20 @@ fn statement(
     })
 }
 
+/// An agent's value as SQLite binds it. Text stays text, whatever it
+/// reads as: a filter's value is typed into a box, and an agent's comes
+/// with its type.
+fn bound(param: &Param) -> rusqlite::types::Value {
+    use rusqlite::types::Value as Sqlite;
+    match param {
+        Param::Null => Sqlite::Null,
+        Param::Bool(flag) => Sqlite::Integer(i64::from(*flag)),
+        Param::Int(number) => Sqlite::Integer(*number),
+        Param::Float(number) => Sqlite::Real(*number),
+        Param::Text(text) => Sqlite::Text(text.clone()),
+    }
+}
+
 fn to_sqlite(value: &Value) -> rusqlite::types::Value {
     use rusqlite::types::Value as Sqlite;
     match value {
@@ -1029,7 +1088,42 @@ impl Adapter for Conn {
         let guard = StopOnDrop(Some(stop.clone()));
         let fences = self.fences.clone();
         let outcome = self
-            .run(move |connection| script(connection, &fences, &texts, limit, mode, &stop))
+            .run(move |connection| script(connection, &fences, &texts, limit, mode, &stop, None))
+            .await;
+        guard.disarm();
+        outcome
+    }
+
+    /// See [`crate::Connection::run_agent_read`]. One blocking job, as a
+    /// script is. The progress handler that checks `stop` also checks the
+    /// run's deadline.
+    async fn run_read(
+        &self,
+        read: &Read,
+        limits: Limits,
+        stop: &StopFlag,
+    ) -> Result<ScriptOutcome> {
+        let stop = stop.clone();
+        let guard = StopOnDrop(Some(stop.clone()));
+        let fences = self.fences.clone();
+        let texts = vec![read.text().to_owned()];
+        let params = read.params().to_vec();
+        let outcome = self
+            .run(move |connection| {
+                let sent = Sent {
+                    params: &params,
+                    timeout: limits.timeout,
+                };
+                script(
+                    connection,
+                    &fences,
+                    &texts,
+                    limits.rows as usize,
+                    ScriptMode::ReadOnly,
+                    &stop,
+                    Some(sent),
+                )
+            })
             .await;
         guard.disarm();
         outcome
```

- [ ] **Step 4: Run the tests**

Run: `cargo test --locked -p tabletist-db --lib agent::`, then `cargo test --locked -p tabletist-db --lib sqlite::`, then `cargo test --locked -p tabletist-db --test agent --test sqlite`

Expected: `42 passed` and `47 passed` for the library's `agent::` and `sqlite::` tests, `7 passed` for `tests/agent.rs` (five of SQLite's, and the two other engines' `the_bookshop_is_stocked_on_…`), and `101 passed` for `tests/sqlite.rs`, which is the SQL editor's run on SQLite and has not changed.

- [ ] **Step 5: Commit**

```bash
cargo fmt --all
cargo clippy --locked -p tabletist-db --all-targets -- -D warnings
git add crates/tabletist-db/src crates/tabletist-db/tests/agent.rs
git commit -m "Run an agent's read on SQLite"
```

---

### Task 5: PostgreSQL

Needs `TABLETIST_TEST_PG_URL`.

The script run reads rows as text through a cursor: `DECLARE tabletist_sql NO SCROLL CURSOR FOR <statement>`, then `FETCH`. The values go on the `DECLARE`, by the extended protocol, each in text format (`impl ToSql for Param`): the cursor keeps them, and the `FETCH` after it is the simple query it always was. A statement without values takes the old path, byte for byte.

An `EXPLAIN` is no cursor. Without values it streams as before. With values it runs by the extended protocol, and its rows are read by `Plan`, which takes the three types a plan comes in.

`open` gains the timeout, and the read-only body of `Conn::script` moves into `read_only` so that `agent_read` can share it. `open`, `statements` and `run_statement` each gain an argument, which the run that writes (`pg/script/write.rs`) and the existing tests that call them give as `None`.

**Files:**
- Modify: `crates/tabletist-db/src/pg.rs`, `pg/script.rs`, `pg/script/write.rs`, `agent/run.rs`
- Test: `crates/tabletist-db/tests/agent.rs`, and `pg/script.rs`'s and `agent/run.rs`'s own

- [ ] **Step 1: Write the failing tests**

```diff
diff --git a/crates/tabletist-db/src/agent/run.rs b/crates/tabletist-db/src/agent/run.rs
index 811a785..674b5af 100644
--- a/crates/tabletist-db/src/agent/run.rs
+++ b/crates/tabletist-db/src/agent/run.rs
@@ -189,6 +189,20 @@ mod tests {
         );
     }
 
+    #[test]
+    fn a_timeout_is_whole_milliseconds_and_never_none() {
+        let sent = |timeout: Duration| Sent {
+            params: &[],
+            timeout,
+        };
+        assert_eq!(sent(Duration::from_secs(10)).timeout_ms(), 10_000);
+        assert_eq!(sent(Duration::from_micros(1_999)).timeout_ms(), 1);
+        // 0 would lift the limit.
+        assert_eq!(sent(Duration::ZERO).timeout_ms(), 1);
+        assert_eq!(sent(Duration::from_micros(10)).timeout_ms(), 1);
+        assert_eq!(sent(Duration::MAX).timeout_ms(), 2_147_483_647);
+    }
+
     #[test]
     fn a_count_the_database_disagrees_with_is_said_in_its_numbers() {
         assert_eq!(
diff --git a/crates/tabletist-db/src/pg/script.rs b/crates/tabletist-db/src/pg/script.rs
index 4f7db1e..8f90b8f 100644
--- a/crates/tabletist-db/src/pg/script.rs
+++ b/crates/tabletist-db/src/pg/script.rs
@@ -492,6 +492,9 @@ mod tests {
             send(&conn.run_script(texts, 10, ScriptMode::ReadOnly, stop));
             send(&conn.server_version());
         };
+        let _agent = |conn: &Conn, read: &Read, limits: Limits, stop: &StopFlag| {
+            send(&conn.run_read(read, limits, stop));
+        };
     }
 
     #[test]
@@ -550,6 +553,93 @@ mod tests {
         assert_eq!(unlocked(Err(lost.clone())), Err(lost));
     }
 
+    #[test]
+    fn an_agents_value_is_sent_as_text_for_a_placeholder_of_any_type() {
+        let sent = |param: Param| {
+            let mut out = BytesMut::new();
+            let null = matches!(param.to_sql(&Type::INT4, &mut out).unwrap(), IsNull::Yes);
+            (null, String::from_utf8(out.to_vec()).unwrap())
+        };
+        let text = |text: &str| (false, text.to_owned());
+        assert_eq!(sent(Param::Null), (true, String::new()));
+        assert_eq!(sent(Param::Bool(true)), text("true"));
+        assert_eq!(sent(Param::Bool(false)), text("false"));
+        assert_eq!(sent(Param::Int(i64::MIN)), text("-9223372036854775808"));
+        assert_eq!(sent(Param::Float(1.5)), text("1.5"));
+        // No exponent, which is one spelling fewer for the server to read.
+        assert_eq!(sent(Param::Float(1e21)), text("1000000000000000000000"));
+        // As it is: nothing is quoted, because nothing is written into SQL.
+        assert_eq!(sent(Param::Text("it's".into())), text("it's"));
+        for kind in [Type::INT4, Type::TEXT, Type::TIMESTAMPTZ, Type::INT4_ARRAY] {
+            assert!(<Param as ToSql>::accepts(&kind), "{kind}");
+            let format = Param::Int(1).encode_format(&kind);
+            assert!(matches!(format, Format::Text), "{kind}");
+        }
+    }
+
+    #[test]
+    fn a_plan_is_read_from_the_types_explain_answers_in() {
+        for kind in [Type::TEXT, Type::JSON, Type::XML] {
+            assert!(<Plan as FromSql>::accepts(&kind), "{kind}");
+        }
+        // Not from a type whose binary form is not its text.
+        for kind in [Type::INT4, Type::JSONB, Type::BYTEA] {
+            assert!(!<Plan as FromSql>::accepts(&kind), "{kind}");
+        }
+        let Plan(line) = Plan::from_sql(&Type::TEXT, b"Seq Scan on books").unwrap();
+        assert_eq!(line, "Seq Scan on books");
+        assert!(Plan::from_sql(&Type::TEXT, &[0xC3, 0x28]).is_err());
+    }
+
+    #[tokio::test]
+    async fn the_servers_own_count_of_placeholders_is_the_one_that_holds() {
+        let Some(url) = test_url() else {
+            return;
+        };
+        let conn = session(&url).await;
+        let client = conn.client.lock().await;
+        let stop = StopFlag::new();
+        let mode = ScriptMode::ReadOnly;
+        for (text, sent, counted) in [
+            ("SELECT $1::int, $2::int", vec![Param::Int(1)], 2),
+            ("SELECT 1", vec![Param::Int(1)], 0),
+            // One placeholder written twice is one value.
+            ("SELECT $1::int + $1::int", vec![Param::Int(1); 2], 1),
+        ] {
+            let ran = run_statement(&client, text, 10, mode, &stop, Some(&sent)).await;
+            let error = count_mismatch(counted, sent.len());
+            let position = None;
+            let refused = StatementOutcome::Error { error, position };
+            // Nothing ran, so nothing failed.
+            assert_eq!(ran, Ok((refused, Tx::Open)), "{text}");
+        }
+    }
+
+    #[tokio::test]
+    async fn an_agents_run_has_the_servers_timeout_for_its_transaction_only() {
+        let Some(url) = test_url() else {
+            return;
+        };
+        let conn = session(&url).await;
+        assert_eq!(setting(&conn, "statement_timeout").await, "0");
+        {
+            let client = conn.client.lock().await;
+            let sent = Sent {
+                params: &[],
+                timeout: Duration::from_millis(250),
+            };
+            assert_eq!(open(&client, Some(sent)).await, Ok(Tx::Open));
+        }
+        assert_eq!(setting(&conn, "statement_timeout").await, "250ms");
+        // A lock is waited for as long as the statement may run, no less.
+        assert_eq!(setting(&conn, "lock_timeout").await, "0");
+        {
+            let client = conn.client.lock().await;
+            close(&client, Tx::Open).await.unwrap();
+        }
+        assert_eq!(setting(&conn, "statement_timeout").await, "0");
+    }
+
     /// One test at a time uses the `probe` table: creating it twice at once
     /// can fail, and one test's TRUNCATE would hide another's stray row.
     pub(super) static PROBE: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());
@@ -744,7 +834,7 @@ mod tests {
         };
         let conn = session(&url).await;
         let client = conn.client.lock().await;
-        assert_eq!(open(&client).await, Ok(Tx::Open));
+        assert_eq!(open(&client, None).await, Ok(Tx::Open));
         // What the first two statements of the script would have done.
         client
             .batch_execute("SET search_path = pg_catalog")
@@ -761,6 +851,7 @@ mod tests {
             10,
             ScriptMode::ReadOnly,
             &stop,
+            None,
             &mut outcome,
         )
         .await;
@@ -783,27 +874,27 @@ mod tests {
         let conn = session(&url).await;
         {
             let client = conn.client.lock().await;
-            assert_eq!(open(&client).await, Ok(Tx::Open));
+            assert_eq!(open(&client, None).await, Ok(Tx::Open));
             assert_eq!(in_transaction(&client).await, Ok(Tx::Open));
             assert_eq!(close(&client, Tx::Open).await, Ok(()));
             // Closed: no transaction is left.
             assert_eq!(in_transaction(&client).await, Ok(Tx::Left));
 
-            assert_eq!(open(&client).await, Ok(Tx::Open));
+            assert_eq!(open(&client, None).await, Ok(Tx::Open));
             assert!(client.batch_execute("SELECT 1 / 0").await.is_err());
             assert_eq!(in_transaction(&client).await, Ok(Tx::Aborted));
             assert_eq!(close(&client, Tx::Open).await, Ok(()));
             assert_eq!(in_transaction(&client).await, Ok(Tx::Left));
 
             // Another transaction is not the script's, block or not.
-            assert_eq!(open(&client).await, Ok(Tx::Open));
+            assert_eq!(open(&client, None).await, Ok(Tx::Open));
             client.batch_execute("COMMIT AND CHAIN").await.unwrap();
             assert_eq!(close(&client, Tx::Open).await, Err(Error::LeftReadOnly));
             assert_eq!(in_transaction(&client).await, Ok(Tx::Left));
 
             // Nor is it once it failed (a cancel that landed on a check):
             // a failed block does not say whose it is, its savepoint does.
-            assert_eq!(open(&client).await, Ok(Tx::Open));
+            assert_eq!(open(&client, None).await, Ok(Tx::Open));
             client.batch_execute("COMMIT AND CHAIN").await.unwrap();
             assert!(client.batch_execute("SELECT 1 / 0").await.is_err());
             assert_eq!(in_transaction(&client).await, Ok(Tx::Aborted));
diff --git a/crates/tabletist-db/tests/agent.rs b/crates/tabletist-db/tests/agent.rs
index 01401e8..d1328b5 100644
--- a/crates/tabletist-db/tests/agent.rs
+++ b/crates/tabletist-db/tests/agent.rs
@@ -465,6 +465,187 @@ mod postgres {
         assert_eq!(connection.dialect(), Dialect::Postgres);
         the_bookshop_is_stocked(&connection, "postgres").await;
     }
+
+    #[tokio::test]
+    async fn an_agent_reads_the_bookshop_on_postgres() {
+        let Some(connection) = bookshop().await else {
+            return;
+        };
+        an_agent_reads_the_bookshop(&connection, "postgres").await;
+    }
+
+    #[tokio::test]
+    async fn postgres_reads_a_value_as_the_type_of_its_placeholder() {
+        let Some(connection) = bookshop().await else {
+            return;
+        };
+        let on = "postgres";
+        // The ends of a 64-bit integer come back as they went.
+        let ends = read(
+            &connection,
+            "SELECT ?::bigint, ?::bigint",
+            &[Param::Int(i64::MAX), Param::Int(i64::MIN)],
+            on,
+        )
+        .await;
+        assert_eq!(ends, [[Value::Int(i64::MAX), Value::Int(i64::MIN)]]);
+        // A placeholder with no type to take is text.
+        let sent = read(&connection, "SELECT ? AS sent", &[Param::Int(42)], on).await;
+        assert_eq!(sent, [[text("42")]]);
+        // Text fills a placeholder of any type the server reads from text:
+        // an array here.
+        let found = read(
+            &connection,
+            "SELECT count(*) FROM books WHERE id = ANY(?)",
+            &[Param::Text("{1, 2, 3}".into())],
+            on,
+        )
+        .await;
+        assert_eq!(found, [[Value::Int(3)]]);
+        // One value for a placeholder written twice.
+        let twice = "SELECT title FROM books WHERE id = $1 OR author_id = $1 ORDER BY id";
+        let read_twice = agent::check_read(Dialect::Postgres, twice, &[Param::Int(3)]).unwrap();
+        let found = connection
+            .run_agent_read(&read_twice, LIMITS, &StopFlag::new())
+            .await
+            .unwrap();
+        assert_eq!(found.rows, [[text("Lanterns")], [text("Ninety-One Doors")]]);
+        // A value its placeholder's type does not take is the server's
+        // error, and the session goes on.
+        let error = asked(
+            &connection,
+            "SELECT title FROM books WHERE id = ?",
+            &[Param::Text("three".into())],
+            LIMITS,
+        )
+        .await
+        .unwrap_err();
+        let Error::Query { code, .. } = &error else {
+            panic!("{error:?}");
+        };
+        assert_eq!(code.as_deref(), Some("22P02"), "{error}");
+        the_bookshop_is_stocked(&connection, on).await;
+    }
+
+    #[tokio::test]
+    async fn a_placeholder_in_a_string_is_text_on_postgres() {
+        let Some(connection) = bookshop().await else {
+            return;
+        };
+        let sql = "SELECT '$1', $q$ $1 $q$, title FROM books WHERE id = $1";
+        let read = agent::check_read(Dialect::Postgres, sql, &[Param::Int(3)]).unwrap();
+        let found = connection
+            .run_agent_read(&read, LIMITS, &StopFlag::new())
+            .await
+            .unwrap();
+        assert_eq!(found.rows, [[text("$1"), text(" $1 "), text("Lanterns")]]);
+    }
+
+    #[tokio::test]
+    async fn a_plan_takes_values_on_postgres() {
+        let Some(connection) = bookshop().await else {
+            return;
+        };
+        let on = "postgres";
+        let one = [Param::Int(3)];
+        let plan = read(
+            &connection,
+            "EXPLAIN SELECT title FROM books WHERE id = ?",
+            &one,
+            on,
+        )
+        .await;
+        let Some(Value::Text(first)) = plan.first().and_then(|line| line.first()) else {
+            panic!("{plan:?}");
+        };
+        assert!(first.contains("books"), "{first}");
+        // A plan as JSON is one value of another type.
+        let plan = read(
+            &connection,
+            "EXPLAIN (FORMAT JSON) SELECT title FROM books WHERE id = ?",
+            &one,
+            on,
+        )
+        .await;
+        let [line] = plan.as_slice() else {
+            panic!("{plan:?}");
+        };
+        let [Value::Text(json)] = line.as_slice() else {
+            panic!("{line:?}");
+        };
+        assert!(json.trim_start().starts_with('['), "{json}");
+        // A long plan is cut at the limit like any answer.
+        let cut = asked(
+            &connection,
+            "EXPLAIN (VERBOSE) SELECT * FROM books b JOIN authors a ON a.id = b.author_id \
+             WHERE b.id = ?",
+            &one,
+            Limits { rows: 2, ..LIMITS },
+        )
+        .await
+        .unwrap();
+        assert_eq!(cut.rows.len(), 2);
+        assert!(cut.truncated);
+    }
+
+    /// A statement's time includes its wait for a lock, so the one timeout
+    /// ends a read that waits behind another session's `ALTER TABLE`.
+    #[tokio::test]
+    async fn the_timeout_ends_a_wait_for_a_lock_on_postgres() {
+        let (Some(connection), Some(url)) = (bookshop().await, url()) else {
+            return;
+        };
+        let holder = admin(&url, Some("bookshop")).await;
+        holder
+            .batch_execute("BEGIN; LOCK TABLE book_covers IN ACCESS EXCLUSIVE MODE")
+            .await
+            .unwrap();
+        let short = Limits {
+            timeout: Duration::from_millis(200),
+            ..LIMITS
+        };
+        let started = Instant::now();
+        let ended = asked(&connection, "SELECT count(*) FROM book_covers", &[], short).await;
+        let took = started.elapsed();
+        holder.batch_execute("ROLLBACK").await.unwrap();
+        assert_eq!(ended.unwrap_err(), Error::Timeout);
+        assert!(took < Duration::from_secs(5), "{took:?}");
+        // The lock is gone, and the session reads on.
+        let covers = read(
+            &connection,
+            "SELECT count(*) FROM book_covers",
+            &[],
+            "postgres",
+        )
+        .await;
+        assert_eq!(covers, [[Value::Int(3)]]);
+    }
+
+    /// The timeout is the transaction's: after a run, one that ran out of
+    /// time or not, the session has none, as the SQL editor expects.
+    #[tokio::test]
+    async fn the_timeout_ends_with_its_run_on_postgres() {
+        let Some(connection) = bookshop().await else {
+            return;
+        };
+        let on = "postgres";
+        let none = [[text("0")]];
+        read(&connection, "SELECT count(*) FROM books", &[], on).await;
+        assert_eq!(
+            rows_of(&connection, "SHOW statement_timeout", on).await,
+            none
+        );
+        let short = Limits {
+            timeout: Duration::from_millis(50),
+            ..LIMITS
+        };
+        let ended = asked(&connection, &slow(5), &[], short).await;
+        assert_eq!(ended.unwrap_err(), Error::Timeout);
+        assert_eq!(
+            rows_of(&connection, "SHOW statement_timeout", on).await,
+            none
+        );
+    }
 }
 
 mod mysql {
```

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test --locked -p tabletist-db --test agent postgres`

Expected: FAIL. Six of PostgreSQL's seven tests fail, each where it first asks for a read:

```
postgres: SELECT count(*) FROM books: not supported: an agent's read on PostgreSQL
test result: FAILED. 1 passed; 6 failed
```

The library's own tests do not compile (`cargo test --locked -p tabletist-db --lib pg::script`): `Plan`, `Sent::timeout_ms` and the `ToSql` of a `Param` are not there, and `open`, `statements` and `run_statement` take one argument fewer.

- [ ] **Step 3: Bind the values, and set the timeout**

```diff
diff --git a/crates/tabletist-db/src/agent/run.rs b/crates/tabletist-db/src/agent/run.rs
index 674b5af..20c2930 100644
--- a/crates/tabletist-db/src/agent/run.rs
+++ b/crates/tabletist-db/src/agent/run.rs
@@ -51,6 +51,17 @@ pub(crate) struct Sent<'a> {
     pub timeout: Duration,
 }
 
+impl Sent<'_> {
+    /// The timeout in whole milliseconds: never 0, which PostgreSQL and
+    /// MySQL read as no limit, and never more than PostgreSQL's setting
+    /// holds.
+    pub fn timeout_ms(&self) -> u32 {
+        u32::try_from(self.timeout.as_millis())
+            .unwrap_or(u32::MAX)
+            .clamp(1, i32::MAX as u32)
+    }
+}
+
 /// The error of a statement the database counts another number of
 /// placeholders in than values were sent for. The check counted them too
 /// ([`super::check_read`]), by its own reading: where the two disagree the
diff --git a/crates/tabletist-db/src/pg.rs b/crates/tabletist-db/src/pg.rs
index 4b7ed10..be078ee 100644
--- a/crates/tabletist-db/src/pg.rs
+++ b/crates/tabletist-db/src/pg.rs
@@ -791,11 +791,11 @@ impl Adapter for Conn {
 
     async fn run_read(
         &self,
-        _read: &Read,
-        _limits: Limits,
-        _stop: &StopFlag,
+        read: &Read,
+        limits: Limits,
+        stop: &StopFlag,
     ) -> Result<ScriptOutcome> {
-        Err(Error::Unsupported("an agent's read on PostgreSQL"))
+        self.agent_read(read, limits, stop).await
     }
 
     async fn write(&self, changes: &ChangeSet, stop: &StopFlag) -> Result<WriteOutcome> {
diff --git a/crates/tabletist-db/src/pg/script.rs b/crates/tabletist-db/src/pg/script.rs
index 8f90b8f..5e6ae90 100644
--- a/crates/tabletist-db/src/pg/script.rs
+++ b/crates/tabletist-db/src/pg/script.rs
@@ -8,12 +8,15 @@ use std::time::{Duration, Instant};
 
 use tokio_postgres::SimpleQueryMessage;
 use tokio_postgres::error::SqlState;
+use tokio_postgres::types::private::BytesMut;
+use tokio_postgres::types::{Format, FromSql, IsNull, ToSql, Type, to_sql_checked};
 
-use super::{Conn, column_metas, first_text, query_error, row_values};
+use super::{Conn, column_metas, first_text, query_error, row_values, unexpected};
+use crate::agent::{Limits, Param, Read, Sent, count_mismatch};
 use crate::script::{cannot_start, cleanup_failed, retry_cancelled, statement_failed};
 use crate::{
     ColumnMeta, Dialect, Error, Result, ScriptMode, ScriptOutcome, StatementOutcome,
-    StatementResult, StopFlag,
+    StatementResult, StopFlag, Value,
 };
 
 mod write;
@@ -36,34 +39,126 @@ impl Conn {
         if mode == ScriptMode::Write {
             return write::run(&client, texts, limit as usize, stop).await;
         }
-        let mut outcome = ScriptOutcome::default();
-        let tx = match open(&client).await {
-            // A lost session ends the run here: nothing to close.
-            Ok(Tx::Open) => {
-                statements(&client, texts, limit as usize, mode, stop, &mut outcome).await?
-            }
-            // A cancel landed on the opening queries: no results.
-            Ok(tx) => {
-                outcome.stopped = true;
-                tx
-            }
-            Err(error) if error.is_connection_lost() => return Err(error),
-            // The transaction could not start, say because the session is
-            // stuck in a failed one. The next run would fail the same way,
-            // so the session is closed, after an attempt to end what is
-            // open.
-            Err(error) => {
-                stop.finish();
-                close(&client, Tx::Aborted).await.ok();
-                return Err(cannot_start(ScriptMode::ReadOnly, &error));
-            }
-        };
-        // From here on a cancel would land on the cleanup: tell the
-        // backend to stop repeating its cancel.
-        stop.finish();
-        close(&client, tx).await?;
-        Ok(outcome)
+        read_only(&client, texts, limit as usize, stop, None).await
     }
+
+    /// See [`crate::Connection::run_agent_read`]: the run of a script of
+    /// one statement that only reads, with the statement's values bound
+    /// and the server's own timeout set for the transaction. Awaited to
+    /// its end, as [`Self::script`] is.
+    pub(super) async fn agent_read(
+        &self,
+        read: &Read,
+        limits: Limits,
+        stop: &StopFlag,
+    ) -> Result<ScriptOutcome> {
+        let client = self.client.lock().await;
+        let texts = [read.text().to_owned()];
+        let sent = Sent {
+            params: read.params(),
+            timeout: limits.timeout,
+        };
+        read_only(&client, &texts, limits.rows as usize, stop, Some(sent)).await
+    }
+}
+
+/// The run that only reads: its transaction, its statements, and the
+/// close. `sent` is there when the run is of an agent's one statement.
+async fn read_only(
+    client: &tokio_postgres::Client,
+    texts: &[String],
+    limit: usize,
+    stop: &StopFlag,
+    sent: Option<Sent<'_>>,
+) -> Result<ScriptOutcome> {
+    let mut outcome = ScriptOutcome::default();
+    let tx = match open(client, sent).await {
+        // A lost session ends the run here: nothing to close.
+        Ok(Tx::Open) => {
+            let params = sent.map(|sent| sent.params);
+            let mode = ScriptMode::ReadOnly;
+            statements(client, texts, limit, mode, stop, params, &mut outcome).await?
+        }
+        // A cancel landed on the opening queries: no results.
+        Ok(tx) => {
+            outcome.stopped = true;
+            tx
+        }
+        Err(error) if error.is_connection_lost() => return Err(error),
+        // The transaction could not start, say because the session is
+        // stuck in a failed one. The next run would fail the same way,
+        // so the session is closed, after an attempt to end what is
+        // open.
+        Err(error) => {
+            stop.finish();
+            close(client, Tx::Aborted).await.ok();
+            return Err(cannot_start(ScriptMode::ReadOnly, &error));
+        }
+    };
+    // From here on a cancel would land on the cleanup: tell the
+    // backend to stop repeating its cancel.
+    stop.finish();
+    close(client, tx).await?;
+    Ok(outcome)
+}
+
+/// An agent's value as PostgreSQL is sent it: in text, which the server
+/// reads as the type it gave the placeholder, the way it reads a quoted
+/// literal. So one kind of value fills a placeholder of any type, and a
+/// value its type does not take is the server's error. A value is never
+/// written into the statement.
+impl ToSql for Param {
+    fn to_sql(
+        &self,
+        _: &Type,
+        out: &mut BytesMut,
+    ) -> std::result::Result<IsNull, Box<dyn std::error::Error + Sync + Send>> {
+        let text = match self {
+            Self::Null => return Ok(IsNull::Yes),
+            Self::Bool(true) => "true".to_owned(),
+            Self::Bool(false) => "false".to_owned(),
+            Self::Int(number) => number.to_string(),
+            Self::Float(number) => number.to_string(),
+            Self::Text(text) => text.clone(),
+        };
+        out.extend_from_slice(text.as_bytes());
+        Ok(IsNull::No)
+    }
+
+    fn accepts(_: &Type) -> bool {
+        true
+    }
+
+    fn encode_format(&self, _: &Type) -> Format {
+        Format::Text
+    }
+
+    to_sql_checked!();
+}
+
+/// A value of a plan, which the server sends as it is written whatever
+/// the protocol: `EXPLAIN` answers in text, JSON or XML.
+struct Plan(String);
+
+impl<'a> FromSql<'a> for Plan {
+    fn from_sql(
+        _: &Type,
+        raw: &'a [u8],
+    ) -> std::result::Result<Self, Box<dyn std::error::Error + Sync + Send>> {
+        Ok(Self(std::str::from_utf8(raw)?.to_owned()))
+    }
+
+    fn accepts(kind: &Type) -> bool {
+        *kind == Type::TEXT || *kind == Type::JSON || *kind == Type::XML
+    }
+}
+
+/// `params` as the driver takes a statement's values.
+fn values(params: &[Param]) -> Vec<&(dyn ToSql + Sync)> {
+    params
+        .iter()
+        .map(|param| param as &(dyn ToSql + Sync))
+        .collect()
 }
 
 /// What a SQL editor row statement runs as: `DECLARE` with this prefix,
@@ -91,8 +186,20 @@ enum Tx {
 /// a transaction read-write once it has a snapshot, whatever a statement
 /// tries. `Aborted` when a cancel landed on these queries; an `Err` that is
 /// not a lost session means the transaction could not start.
-async fn open(client: &tokio_postgres::Client) -> Result<Tx> {
-    for step in ["BEGIN READ ONLY", "SELECT 1", "SAVEPOINT tabletist_guard"] {
+///
+/// An agent's run also sets the server's own statement timeout, for the
+/// transaction: the rollback that ends the run takes it away. Before the
+/// savepoint, so that it holds whatever the guard does with that. The
+/// statement cannot lift it: the refusal stops every `SET`, and the check
+/// `set_config`. No lock timeout beside it: a wait for a lock is part of
+/// the statement's time.
+async fn open(client: &tokio_postgres::Client, sent: Option<Sent<'_>>) -> Result<Tx> {
+    let timeout = sent.map(|sent| format!("SET LOCAL statement_timeout = {}", sent.timeout_ms()));
+    let steps = ["BEGIN READ ONLY", "SELECT 1"]
+        .into_iter()
+        .chain(timeout.as_deref())
+        .chain(["SAVEPOINT tabletist_guard"]);
+    for step in steps {
         match client.batch_execute(step).await.map_err(query_error) {
             Ok(()) => {}
             Err(Error::Cancelled) => return Ok(Tx::Aborted),
@@ -104,13 +211,15 @@ async fn open(client: &tokio_postgres::Client) -> Result<Tx> {
 
 /// Runs the statements in order, up to the first that fails or is
 /// stopped, and says where that leaves the transaction. `Err` is a lost
-/// session.
+/// session. `params` are an agent's values for its one statement: a script
+/// has none.
 async fn statements(
     client: &tokio_postgres::Client,
     texts: &[String],
     limit: usize,
     mode: ScriptMode,
     stop: &StopFlag,
+    params: Option<&[Param]>,
     outcome: &mut ScriptOutcome,
 ) -> Result<Tx> {
     for text in texts {
@@ -150,7 +259,7 @@ async fn statements(
             return Ok(tx);
         }
         let started = Instant::now();
-        let (result, tx) = run_statement(client, text, limit, mode, stop).await?;
+        let (result, tx) = run_statement(client, text, limit, mode, stop, params).await?;
         outcome.stopped |= result == StatementOutcome::Cancelled;
         let last = !matches!(
             result,
@@ -379,14 +488,46 @@ fn rows_outcome(
     }
 }
 
+/// A plan's rows from the extended protocol: at most `limit` of them, and
+/// `truncated` when there were more.
+fn plan_outcome(
+    columns: Vec<ColumnMeta>,
+    answered: &[tokio_postgres::Row],
+    limit: usize,
+) -> StatementOutcome {
+    let line = |row: &tokio_postgres::Row| -> Result<Vec<Value>> {
+        (0..row.len())
+            .map(|index| {
+                let value: Option<Plan> = row.try_get(index).map_err(unexpected)?;
+                Ok(value.map_or(Value::Null, |Plan(text)| Value::Text(text.into())))
+            })
+            .collect()
+    };
+    match answered.iter().take(limit).map(line).collect() {
+        Ok(rows) => StatementOutcome::Rows {
+            columns,
+            rows,
+            truncated: answered.len() > limit,
+        },
+        Err(error) => StatementOutcome::Error {
+            error,
+            position: None,
+        },
+    }
+}
+
 /// Runs one statement of a script inside its transaction, and says where
 /// that leaves the transaction. `Err` is a lost session.
+///
+/// `params` are an agent's values. The SQL editor has none, and its
+/// statement with a placeholder is the server's error, as before.
 async fn run_statement(
     client: &tokio_postgres::Client,
     text: &str,
     limit: usize,
     mode: ScriptMode,
     stop: &StopFlag,
+    params: Option<&[Param]>,
 ) -> Result<(StatementOutcome, Tx)> {
     // The driver cannot put a NUL in a message, and fails in a way that
     // reads as a lost session. Nothing is sent: the transaction stays open.
@@ -404,10 +545,23 @@ async fn run_statement(
     // statement hidden in one piece. A prepare error is the outcome; the
     // text never runs another way. Only the columns are kept: the prepared
     // statement itself is closed here.
-    let columns = match client.prepare(text).await {
-        Ok(prepared) => column_metas(&prepared),
+    let prepared = match client.prepare(text).await {
+        Ok(prepared) => prepared,
         Err(error) => return failed(error, Some(0)),
     };
+    let columns = column_metas(&prepared);
+    if let Some(params) = params {
+        // The server's own count, which is the one that holds. Nothing
+        // ran: the transaction stays open.
+        let counted = prepared.params().len();
+        if counted != params.len() {
+            let error = count_mismatch(counted, params.len());
+            let position = None;
+            return Ok((StatementOutcome::Error { error, position }, Tx::Open));
+        }
+    }
+    // Without values a statement runs as it always did.
+    let params = params.unwrap_or_default();
     if columns.is_empty() {
         return match client.simple_query(text).await {
             Ok(messages) => {
@@ -438,7 +592,16 @@ async fn run_statement(
     if mode == ScriptMode::ReadOnly && cursor_statement(text) {
         // Its own call, and a newline, so a trailing -- comment ends there.
         let declare = format!("{CURSOR_PREFIX}{text}\n");
-        if let Err(error) = client.batch_execute(&declare).await {
+        // Values are bound where the cursor is declared, which takes the
+        // other protocol: the cursor keeps them, and its rows are fetched
+        // as text like any other's.
+        let declared = if params.is_empty() {
+            client.batch_execute(&declare).await
+        } else {
+            let bound = client.execute(declare.as_str(), &values(params)).await;
+            bound.map(drop)
+        };
+        if let Err(error) = declared {
             return failed(error, Some(CURSOR_PREFIX.chars().count()));
         }
         // Nothing failed: the transaction is still usable.
@@ -457,6 +620,13 @@ async fn run_statement(
             SimpleQueryMessage::Row(row) => Some(row),
             _ => None,
         }));
+    } else if !params.is_empty() {
+        // An `EXPLAIN` with values, the one statement of an agent's that
+        // is no cursor. Its rows come by the protocol that binds them.
+        return match client.query(text, &values(params)).await {
+            Ok(rows) => Ok((plan_outcome(columns, &rows, limit), Tx::Open)),
+            Err(error) => failed(error, Some(0)),
+        };
     } else {
         // SHOW, EXPLAIN and the like, and every statement with rows of a
         // run that writes: stream, keep limit + 1, drop the rest.
diff --git a/crates/tabletist-db/src/pg/script/write.rs b/crates/tabletist-db/src/pg/script/write.rs
index 11710ef..035d480 100644
--- a/crates/tabletist-db/src/pg/script/write.rs
+++ b/crates/tabletist-db/src/pg/script/write.rs
@@ -21,7 +21,16 @@ pub(super) async fn run(
     let tx = match open(client).await {
         // A lost session ends the run here: nothing to close.
         Ok(Tx::Open) => {
-            statements(client, texts, limit, ScriptMode::Write, stop, &mut outcome).await?
+            statements(
+                client,
+                texts,
+                limit,
+                ScriptMode::Write,
+                stop,
+                None,
+                &mut outcome,
+            )
+            .await?
         }
         // A cancel landed on the opening queries: no results.
         Ok(tx) => {
```

- [ ] **Step 4: Run the tests**

Run: `cargo test --locked -p tabletist-db --lib --test agent --test postgres`

Expected: `438 passed; 0 failed; 1 ignored` for the library, `13 passed` for `tests/agent.rs` (SQLite's five, PostgreSQL's seven, MySQL's one), and `83 passed` for `tests/postgres.rs`, which is the SQL editor's run on PostgreSQL and has not changed.

- [ ] **Step 5: Commit**

```bash
cargo fmt --all
cargo clippy --locked -p tabletist-db --all-targets -- -D warnings
git add crates/tabletist-db/src crates/tabletist-db/tests/agent.rs
git commit -m "Run an agent's read on PostgreSQL"
```

---

### Task 6: MySQL

Needs `TABLETIST_TEST_MYSQL_URL`, and the `bookshop` database of Task 1.

The script run already goes through the prepared protocol, and refused a statement with a `?` because it had no values for one. Now an agent's run has them. `open` sets the time limit after the row limit, and the reset that ends every run takes both away. As on PostgreSQL, the read-only body of `Conn::script` moves into `read_only`, and `open`, `statements` and `run_statement` each gain an argument that the run that writes and the existing tests give as `None`.

One thing is MySQL's alone. The driver rewrites `:name` to `?` before it prepares, by a lexer of its own that loses its place after a `-` or a `/` (`driver_parameter`). A real `:name` does not pass the check, so for an agent's statement what is left is the misreading, and it is told so in words that are true of its statement (`driver_misread`).

`agent/run.rs` learns MySQL's and MariaDB's numbers for a statement over its time.

**Files:**
- Modify: `crates/tabletist-db/src/mysql.rs`, `mysql/script.rs`, `mysql/script/write.rs`, `agent/run.rs`
- Test: `crates/tabletist-db/tests/agent.rs`, and `mysql/script.rs`'s and `agent/run.rs`'s own

- [ ] **Step 1: Write the failing tests**

`what_an_agents_statement_leaves_on_the_session_is_gone_after_the_run` is the test run 1's plan left for this run: `LAST_INSERT_ID(n)`, a `GET_LOCK` lock and a user variable, sent past the check, are gone after the reset. So the check's refusal of each is a second fence on MySQL, as that plan supposed.

```diff
diff --git a/crates/tabletist-db/src/agent/run.rs b/crates/tabletist-db/src/agent/run.rs
index 20c2930..1ca6aa6 100644
--- a/crates/tabletist-db/src/agent/run.rs
+++ b/crates/tabletist-db/src/agent/run.rs
@@ -113,7 +113,7 @@ pub(crate) fn answer(outcome: ScriptOutcome, stop: &StopFlag) -> Result<Answer>
 #[cfg(test)]
 mod tests {
     use super::*;
-    use crate::{StatementResult, ValueKind};
+    use crate::{Named, StatementResult, ValueKind};
 
     fn outcome_of(outcome: StatementOutcome) -> ScriptOutcome {
         ScriptOutcome {
@@ -187,6 +187,33 @@ mod tests {
         );
     }
 
+    #[test]
+    fn mysqls_and_mariadbs_word_for_a_timeout_is_a_timeout() {
+        let numbered = |number: u32| StatementOutcome::Error {
+            error: Error::Query {
+                code: Some("HY000".into()),
+                message: "Query execution was interrupted".into(),
+                detail: None,
+                hint: None,
+                named: Box::new(Named {
+                    number: Some(number),
+                    ..Named::default()
+                }),
+            },
+            position: None,
+        };
+        for number in [3024, 1969] {
+            assert_eq!(
+                answer(outcome_of(numbered(number)), &StopFlag::new()),
+                Err(Error::Timeout),
+                "{number}"
+            );
+        }
+        // Any other number is the statement's own error.
+        let other = answer(outcome_of(numbered(1054)), &StopFlag::new()).unwrap_err();
+        assert!(matches!(other, Error::Query { .. }), "{other:?}");
+    }
+
     #[test]
     fn a_statement_without_rows_is_no_read() {
         let done = StatementOutcome::Done {
diff --git a/crates/tabletist-db/src/mysql/script.rs b/crates/tabletist-db/src/mysql/script.rs
index 35bd5c5..10ac7b7 100644
--- a/crates/tabletist-db/src/mysql/script.rs
+++ b/crates/tabletist-db/src/mysql/script.rs
@@ -507,6 +507,9 @@ mod tests {
             send(&conn.run_script(texts, 10, ScriptMode::ReadOnly, stop));
             send(&conn.server_version());
         };
+        let _agent = |conn: &Conn, read: &Read, limits: Limits, stop: &StopFlag| {
+            send(&conn.run_read(read, limits, stop));
+        };
     }
 
     #[test]
@@ -779,6 +782,136 @@ mod tests {
             .expect("the statement never ran");
     }
 
+    #[test]
+    fn an_agents_values_are_bound_each_as_its_own_type() {
+        use mysql_async::Value as MySql;
+        assert!(matches!(bound(&[]), Params::Empty));
+        let Params::Positional(values) = bound(&[
+            Param::Null,
+            Param::Bool(true),
+            Param::Int(i64::MIN),
+            Param::Float(1.5),
+            Param::Text("it's".into()),
+        ]) else {
+            panic!("values by position");
+        };
+        assert_eq!(
+            values,
+            [
+                MySql::NULL,
+                MySql::Int(1),
+                MySql::Int(i64::MIN),
+                MySql::Double(1.5),
+                // As it is: nothing is quoted, because nothing is written
+                // into SQL.
+                MySql::Bytes(b"it's".to_vec()),
+            ]
+        );
+    }
+
+    #[test]
+    fn the_time_limit_is_spelled_for_mysql_and_for_mariadb() {
+        assert_eq!(
+            time_limits(10_000),
+            [
+                "SET SESSION max_execution_time = 10000",
+                "SET SESSION max_statement_time = 10.000",
+            ]
+        );
+        // MariaDB counts in seconds, and a millisecond is still some time:
+        // 0 would be no limit.
+        assert_eq!(
+            time_limits(1),
+            [
+                "SET SESSION max_execution_time = 1",
+                "SET SESSION max_statement_time = 0.001",
+            ]
+        );
+        assert_eq!(
+            time_limits(1_250)[1],
+            "SET SESSION max_statement_time = 1.250"
+        );
+    }
+
+    #[tokio::test]
+    async fn the_servers_own_count_of_placeholders_is_the_one_that_holds() {
+        let Some(url) = test_url() else {
+            return;
+        };
+        let session = session(&url).await;
+        let mut conn = session.conn.lock().await;
+        for (text, sent, counted) in [
+            ("SELECT ?, ?", vec![Param::Int(1)], 2),
+            ("SELECT 1", vec![Param::Int(1)], 0),
+        ] {
+            let ran = run_statement(&mut conn, text, 10, Some(&sent)).await;
+            let error = count_mismatch(counted, sent.len());
+            let position = None;
+            assert_eq!(
+                ran,
+                Ok(StatementOutcome::Error { error, position }),
+                "{text}"
+            );
+        }
+        // The SQL editor sends no values, and is told what it always was.
+        let ran = run_statement(&mut conn, "SELECT ?", 10, None).await;
+        assert_eq!(ran, Ok(parameter("?", None)));
+        // A `:name` only the driver reads, beside a real placeholder: said
+        // to an agent in words that are true of its statement.
+        let text = "SELECT ? -':id'";
+        let ran = run_statement(&mut conn, text, 10, Some(&[Param::Int(1)])).await;
+        assert_eq!(ran, Ok(driver_misread()));
+        let ran = run_statement(&mut conn, text, 10, None).await;
+        assert_eq!(ran, Ok(parameter("? and :name", None)));
+    }
+
+    /// The check refuses each of these by name. Past it, what they leave on
+    /// the session is gone with the reset that ends the run, and so are
+    /// the run's own row limit and time limit.
+    #[tokio::test]
+    async fn what_an_agents_statement_leaves_on_the_session_is_gone_after_the_run() {
+        let Some(url) = test_url() else {
+            return;
+        };
+        let session = session(&url).await;
+        let mut conn = session.conn.lock().await;
+        let left = "SELECT LAST_INSERT_ID(99), GET_LOCK('tabletist_agent_lock', 0), \
+                    @tabletist_agent := 7";
+        let sent = Sent {
+            params: &[],
+            timeout: Duration::from_secs(10),
+        };
+        let stop = StopFlag::new();
+        let outcome = read_only(
+            &mut conn,
+            &[left.to_owned()],
+            5,
+            Access::ReadOnly,
+            &stop,
+            Some(sent),
+        )
+        .await
+        .unwrap();
+        let [result] = outcome.results.as_slice() else {
+            panic!("{outcome:?}");
+        };
+        let StatementOutcome::Rows { rows, .. } = &result.outcome else {
+            panic!("{result:?}");
+        };
+        // It ran, and took the lock.
+        assert_eq!(rows[0][1], crate::Value::Int(1));
+        let after: (u64, Option<u64>, Option<i64>, u64, u64) = conn
+            .query_first(
+                "SELECT LAST_INSERT_ID(), IS_USED_LOCK('tabletist_agent_lock'), \
+                 @tabletist_agent, @@session.max_execution_time, \
+                 @@session.sql_select_limit = @@global.sql_select_limit",
+            )
+            .await
+            .unwrap()
+            .unwrap();
+        assert_eq!(after, (0, None, None, 0, 1));
+    }
+
     /// Runs statements straight through the driver, as if the refusal had
     /// missed them.
     async fn past_the_refusal(
@@ -912,7 +1045,7 @@ mod tests {
             let mut conn = conn.conn.lock().await;
             assert_eq!(standing(&mut conn).await, Ok(Standing::Outside));
             // The server marks the transaction read-only, or this fails.
-            open(&mut conn, 10, Access::ReadOnly).await.unwrap();
+            open(&mut conn, 10, Access::ReadOnly, None).await.unwrap();
             assert_eq!(standing(&mut conn).await, Ok(Standing::Inside));
             assert_eq!(ready(&mut conn).await, Ok(Standing::Inside));
 
@@ -944,7 +1077,7 @@ mod tests {
             assert_eq!(standing(&mut conn).await, Ok(Standing::Outside));
 
             // A script's own sql_select_limit does not blind the check.
-            open(&mut conn, 10, Access::ReadOnly).await.unwrap();
+            open(&mut conn, 10, Access::ReadOnly, None).await.unwrap();
             conn.query_drop("SET SESSION sql_select_limit = 0")
                 .await
                 .unwrap();
@@ -976,7 +1109,7 @@ mod tests {
         let connected = settings(&conn).await;
         {
             let mut conn = conn.conn.lock().await;
-            open(&mut conn, 10, Access::ReadOnly).await.unwrap();
+            open(&mut conn, 10, Access::ReadOnly, None).await.unwrap();
             // What the first statement of the script would have done.
             conn.query_drop("SET SESSION TRANSACTION READ WRITE")
                 .await
@@ -986,7 +1119,7 @@ mod tests {
             stop.stop();
             let mut outcome = ScriptOutcome::default();
             let texts = ["SELECT 1".to_owned()];
-            let ended = statements(&mut conn, &texts, 10, &stop, &mut outcome).await;
+            let ended = statements(&mut conn, &texts, 10, &stop, None, &mut outcome).await;
             assert_eq!(ended, Ok(Ended::Unconfirmed));
             assert_eq!(outcome.results.len(), 1);
             assert_eq!(outcome.results[0].outcome, StatementOutcome::Cancelled);
diff --git a/crates/tabletist-db/tests/agent.rs b/crates/tabletist-db/tests/agent.rs
index d1328b5..da6193a 100644
--- a/crates/tabletist-db/tests/agent.rs
+++ b/crates/tabletist-db/tests/agent.rs
@@ -723,4 +723,154 @@ mod mysql {
         assert_eq!(connection.dialect(), Dialect::MySql);
         the_bookshop_is_stocked(&connection, "mysql").await;
     }
+
+    #[tokio::test]
+    async fn an_agent_reads_the_bookshop_on_mysql() {
+        let Some(connection) = bookshop().await else {
+            return;
+        };
+        an_agent_reads_the_bookshop(&connection, "mysql").await;
+    }
+
+    #[tokio::test]
+    async fn mysql_takes_each_value_as_its_own_type() {
+        let Some(connection) = bookshop().await else {
+            return;
+        };
+        let on = "mysql";
+        // The ends of a 64-bit integer come back as they went.
+        let ends = read(
+            &connection,
+            "SELECT ?, ?",
+            &[Param::Int(i64::MAX), Param::Int(i64::MIN)],
+            on,
+        )
+        .await;
+        assert_eq!(ends, [[Value::Int(i64::MAX), Value::Int(i64::MIN)]]);
+        // Text that reads as a number is still text: no row has this id.
+        let found = read(
+            &connection,
+            "SELECT count(*) FROM books WHERE title = ?",
+            &[Param::Text("3".into())],
+            on,
+        )
+        .await;
+        assert_eq!(found, [[Value::Int(0)]]);
+        // A backslash is a character of the value, whatever the server's
+        // SQL mode makes of one in a literal.
+        let sent = "a\\b'c\"d";
+        let back = read(&connection, "SELECT ?", &[Param::Text(sent.into())], on).await;
+        assert_eq!(back, [[text(sent)]]);
+    }
+
+    #[tokio::test]
+    async fn a_question_mark_in_a_string_is_text_on_mysql() {
+        let Some(connection) = bookshop().await else {
+            return;
+        };
+        let found = read(
+            &connection,
+            "SELECT 'where?', title FROM books WHERE id = ?",
+            &[Param::Int(3)],
+            "mysql",
+        )
+        .await;
+        assert_eq!(found, [[text("where?"), text("Lanterns")]]);
+    }
+
+    #[tokio::test]
+    async fn a_plan_takes_values_on_mysql() {
+        let Some(connection) = bookshop().await else {
+            return;
+        };
+        let plan = read(
+            &connection,
+            "EXPLAIN SELECT title FROM books WHERE id = ?",
+            &[Param::Int(3)],
+            "mysql",
+        )
+        .await;
+        assert!(!plan.is_empty());
+    }
+
+    /// MySQL holds a `SELECT` to its time limit and nothing else. Every
+    /// shape of read is one to it, wherever the slow part stands.
+    #[tokio::test]
+    async fn the_time_limit_holds_a_read_of_any_shape_on_mysql() {
+        let Some(connection) = bookshop().await else {
+            return;
+        };
+        let slow = slow(5);
+        let short = Limits {
+            timeout: Duration::from_millis(100),
+            ..LIMITS
+        };
+        for sql in [
+            format!("WITH weighed AS ({slow}) SELECT * FROM weighed"),
+            format!("SELECT * FROM ({slow}) weighed"),
+            format!("SELECT 1 UNION ALL ({slow})"),
+            format!("SELECT title, ({slow}) FROM books"),
+            format!("SELECT title FROM books WHERE id > ({slow})"),
+        ] {
+            let ended = asked(&connection, &sql, &[], short).await;
+            assert_eq!(ended.unwrap_err(), Error::Timeout, "{sql}");
+        }
+        the_bookshop_is_stocked(&connection, "mysql").await;
+    }
+
+    /// A read that waits behind another session's lock on a table is ended
+    /// by the same limit: MySQL's timer ends the wait as it ends the run.
+    #[tokio::test]
+    async fn the_time_limit_ends_a_wait_for_a_lock_on_mysql() {
+        let (Some(connection), Some(url)) = (bookshop().await, url()) else {
+            return;
+        };
+        let mut holder = admin(&url).await;
+        holder
+            .query_drop("LOCK TABLES book_covers WRITE")
+            .await
+            .unwrap();
+        let short = Limits {
+            timeout: Duration::from_millis(200),
+            ..LIMITS
+        };
+        let started = Instant::now();
+        let ended = asked(&connection, "SELECT count(*) FROM book_covers", &[], short).await;
+        let took = started.elapsed();
+        holder.query_drop("UNLOCK TABLES").await.unwrap();
+        holder.disconnect().await.unwrap();
+        assert_eq!(ended.unwrap_err(), Error::Timeout);
+        assert!(took < Duration::from_secs(5), "{took:?}");
+        // The lock is gone, and the session reads on.
+        let covers = read(
+            &connection,
+            "SELECT count(*) FROM book_covers",
+            &[],
+            "mysql",
+        )
+        .await;
+        assert_eq!(covers, [[Value::Int(3)]]);
+    }
+
+    /// The time limit is the session's, and the reset that ends a run
+    /// takes it away: after a run, one that ran out of time or not, the
+    /// session has none, as the SQL editor expects.
+    #[tokio::test]
+    async fn the_time_limit_ends_with_its_run_on_mysql() {
+        let Some(connection) = bookshop().await else {
+            return;
+        };
+        let on = "mysql";
+        let limit = "SELECT @@session.max_execution_time";
+        let none = [[Value::Int(0)]];
+        read(&connection, "SELECT count(*) FROM books", &[], on).await;
+        assert_eq!(rows_of(&connection, limit, on).await, none);
+        let short = Limits {
+            timeout: Duration::from_millis(50),
+            ..LIMITS
+        };
+        let ended = asked(&connection, &slow(5), &[], short).await;
+        assert_eq!(ended.unwrap_err(), Error::Timeout);
+        assert_eq!(rows_of(&connection, limit, on).await, none);
+    }
 }
```

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test --locked -p tabletist-db --test agent mysql`

Expected: FAIL. Seven of MySQL's eight tests fail, each where it first asks for a read:

```
mysql: SELECT count(*) FROM books: not supported: an agent's read on MySQL
test result: FAILED. 1 passed; 7 failed
```

The library's own tests do not compile (`cargo test --locked -p tabletist-db --lib mysql::script`): `read_only`, `bound`, `time_limits` and `driver_misread` are not there, and `open`, `statements` and `run_statement` take one argument fewer.

- [ ] **Step 3: Bind the values, and set the time limit**

```diff
diff --git a/crates/tabletist-db/src/agent/run.rs b/crates/tabletist-db/src/agent/run.rs
index 1ca6aa6..5425133 100644
--- a/crates/tabletist-db/src/agent/run.rs
+++ b/crates/tabletist-db/src/agent/run.rs
@@ -100,6 +100,7 @@ pub(crate) fn answer(outcome: ScriptOutcome, stop: &StopFlag) -> Result<Answer>
         StatementOutcome::Done { .. } => Err(Error::query(
             "the statement answered no rows, so it is not a read",
         )),
+        StatementOutcome::Error { error, .. } if timed_out(&error) => Err(Error::Timeout),
         StatementOutcome::Error { error, .. } => Err(error),
         StatementOutcome::Cancelled if stop.is_stopped() => Err(Error::Cancelled),
         // Nobody here stopped it, so the database's own timeout did:
@@ -110,6 +111,21 @@ pub(crate) fn answer(outcome: ScriptOutcome, stop: &StopFlag) -> Result<Answer>
     }
 }
 
+/// The numbers MySQL (3024) and MariaDB (1969) give a statement their own
+/// timeout ended. Only they number an error: see [`crate::Named`].
+const OVER_ITS_TIME: [u32; 2] = [3024, 1969];
+
+/// Whether `error` is a database's word for a statement that ran over the
+/// time the run set.
+fn timed_out(error: &Error) -> bool {
+    match error {
+        Error::Query { named, .. } => named
+            .number
+            .is_some_and(|number| OVER_ITS_TIME.contains(&number)),
+        _ => false,
+    }
+}
+
 #[cfg(test)]
 mod tests {
     use super::*;
diff --git a/crates/tabletist-db/src/mysql.rs b/crates/tabletist-db/src/mysql.rs
index 71a6058..6504286 100644
--- a/crates/tabletist-db/src/mysql.rs
+++ b/crates/tabletist-db/src/mysql.rs
@@ -476,11 +476,11 @@ impl Adapter for Conn {
 
     async fn run_read(
         &self,
-        _read: &Read,
-        _limits: Limits,
-        _stop: &StopFlag,
+        read: &Read,
+        limits: Limits,
+        stop: &StopFlag,
     ) -> Result<ScriptOutcome> {
-        Err(Error::Unsupported("an agent's read on MySQL"))
+        self.agent_read(read, limits, stop).await
     }
 
     async fn write(&self, changes: &ChangeSet, stop: &StopFlag) -> Result<WriteOutcome> {
diff --git a/crates/tabletist-db/src/mysql/script.rs b/crates/tabletist-db/src/mysql/script.rs
index 10ac7b7..30d6bd6 100644
--- a/crates/tabletist-db/src/mysql/script.rs
+++ b/crates/tabletist-db/src/mysql/script.rs
@@ -14,6 +14,7 @@ use super::{
     Conn, READ_ONLY, UNKNOWN_SYSTEM_VARIABLE, column_metas, execute, from_row, prepare_session,
     query_error, row_values, status,
 };
+use crate::agent::{Limits, Param, Read, Sent, count_mismatch};
 use crate::script::{cannot_start, cleanup_failed, retry_cancelled, statement_failed};
 use crate::{
     Access, Dialect, Error, Result, ScriptMode, ScriptOutcome, StatementOutcome, StatementResult,
@@ -51,27 +52,69 @@ impl Conn {
         if mode == ScriptMode::Write {
             return write::run(&mut conn, texts, limit as usize, stop).await;
         }
-        let mut outcome = ScriptOutcome::default();
-        let ended = match open(&mut conn, limit, self.access).await {
-            // A lost session ends the run here: nothing to close.
-            Ok(()) => statements(&mut conn, texts, limit as usize, stop, &mut outcome).await?,
-            // A cancel landed on the opening queries: no results.
-            Err(Error::Cancelled) => {
-                outcome.stopped = true;
-                Ended::Unopened
-            }
-            Err(error) if error.is_connection_lost() => return Err(error),
-            // The transaction could not start. The next run would fail the
-            // same way, so the session is closed, after an attempt to end
-            // what is open.
-            Err(error) => Ended::Broken(cannot_start(ScriptMode::ReadOnly, &error)),
-        };
-        // From here on a cancel would land on the cleanup: tell the
-        // backend to stop repeating its cancel.
-        stop.finish();
-        close(&mut conn, ended, self.access).await?;
-        Ok(outcome)
+        read_only(&mut conn, texts, limit, self.access, stop, None).await
     }
+
+    /// See [`crate::Connection::run_agent_read`]: the run of a script of
+    /// one statement that only reads, with the statement's values bound
+    /// and the server's own limit on its time set for the session, which
+    /// the reset that ends the run takes away. Awaited to its end, as
+    /// [`Self::script`] is.
+    pub(super) async fn agent_read(
+        &self,
+        read: &Read,
+        limits: Limits,
+        stop: &StopFlag,
+    ) -> Result<ScriptOutcome> {
+        let mut conn = self.conn.lock().await;
+        if !can_reset(self.version.contains("MariaDB"), conn.server_version()) {
+            return Err(Error::Unsupported(
+                "a read for an agent needs MySQL 5.7.3 or MariaDB 10.2.4 or later",
+            ));
+        }
+        let texts = [read.text().to_owned()];
+        let sent = Sent {
+            params: read.params(),
+            timeout: limits.timeout,
+        };
+        let access = self.access;
+        read_only(&mut conn, &texts, limits.rows, access, stop, Some(sent)).await
+    }
+}
+
+/// The run that only reads: its transaction, its statements, and the
+/// close. `sent` is there when the run is of an agent's one statement.
+async fn read_only(
+    conn: &mut mysql_async::Conn,
+    texts: &[String],
+    limit: u32,
+    access: Access,
+    stop: &StopFlag,
+    sent: Option<Sent<'_>>,
+) -> Result<ScriptOutcome> {
+    let mut outcome = ScriptOutcome::default();
+    let ended = match open(conn, limit, access, sent).await {
+        // A lost session ends the run here: nothing to close.
+        Ok(()) => {
+            let params = sent.map(|sent| sent.params);
+            statements(conn, texts, limit as usize, stop, params, &mut outcome).await?
+        }
+        // A cancel landed on the opening queries: no results.
+        Err(Error::Cancelled) => {
+            outcome.stopped = true;
+            Ended::Unopened
+        }
+        Err(error) if error.is_connection_lost() => return Err(error),
+        // The transaction could not start. The next run would fail the
+        // same way, so the session is closed, after an attempt to end
+        // what is open.
+        Err(error) => Ended::Broken(cannot_start(ScriptMode::ReadOnly, &error)),
+    };
+    // From here on a cancel would land on the cleanup: tell the
+    // backend to stop repeating its cancel.
+    stop.finish();
+    close(conn, ended, access).await?;
+    Ok(outcome)
 }
 
 /// Where a session stands relative to the read-only transaction a script
@@ -150,7 +193,15 @@ fn started(status: StatusFlags) -> Result<()> {
 /// stops producing rows, and the close's reset puts the default back.
 /// `Err` is a cancel that landed on these queries, a lost session, or a
 /// transaction that could not start.
-async fn open(conn: &mut mysql_async::Conn, limit: u32, access: Access) -> Result<()> {
+///
+/// An agent's run also sets the server's own limit on a statement's time
+/// (`limit_time`), which the same reset takes away.
+async fn open(
+    conn: &mut mysql_async::Conn,
+    limit: u32,
+    access: Access,
+    sent: Option<Sent<'_>>,
+) -> Result<()> {
     // A writable session is read-write between scripts. The checks read
     // the session's own setting, and only that setting makes the server
     // refuse DDL after the commit DDL implies: so a run makes the session
@@ -161,17 +212,53 @@ async fn open(conn: &mut mysql_async::Conn, limit: u32, access: Access) -> Resul
     begin(conn).await?;
     // One row more than the limit, to know whether more exist.
     let rows = u64::from(limit) + 1;
-    execute(conn, &format!("SET SESSION sql_select_limit = {rows}")).await
+    execute(conn, &format!("SET SESSION sql_select_limit = {rows}")).await?;
+    match sent {
+        Some(sent) => limit_time(conn, sent.timeout_ms()).await,
+        None => Ok(()),
+    }
+}
+
+/// The statements that make the server end a statement over its time, as
+/// MySQL and as MariaDB take them. MySQL counts `max_execution_time` in
+/// milliseconds, and holds a `SELECT` to it and nothing else, which is all
+/// a read is. MariaDB has no setting of that name: its
+/// `max_statement_time` is in seconds.
+fn time_limits(ms: u32) -> [String; 2] {
+    [
+        format!("SET SESSION max_execution_time = {ms}"),
+        format!(
+            "SET SESSION max_statement_time = {}.{:03}",
+            ms / 1_000,
+            ms % 1_000
+        ),
+    ]
+}
+
+/// Sets the server's own limit on a statement's time for the session:
+/// MySQL's setting, or MariaDB's where the server does not know that one.
+/// The statement cannot lift it: the refusal stops every `SET`, and the
+/// check the optimizer hints that would (`agent::check_read`).
+async fn limit_time(conn: &mut mysql_async::Conn, ms: u32) -> Result<()> {
+    let [mysql, mariadb] = time_limits(ms);
+    match conn.query_drop(mysql).await {
+        Err(mysql_async::Error::Server(error)) if error.code == UNKNOWN_SYSTEM_VARIABLE => {
+            execute(conn, &mariadb).await
+        }
+        answer => answer.map_err(query_error),
+    }
 }
 
 /// Runs the statements in order, up to the first that fails or is
 /// stopped, and says what that showed of the session. `Err` is a lost
-/// session.
+/// session. `params` are an agent's values for its one statement: a script
+/// has none.
 async fn statements(
     conn: &mut mysql_async::Conn,
     texts: &[String],
     limit: usize,
     stop: &StopFlag,
+    params: Option<&[Param]>,
     outcome: &mut ScriptOutcome,
 ) -> Result<Ended> {
     for text in texts {
@@ -203,7 +290,7 @@ async fn statements(
             }
         }
         let started = Instant::now();
-        let result = run_statement(conn, text, limit).await?;
+        let result = run_statement(conn, text, limit, params).await?;
         outcome.stopped |= matches!(result, StatementOutcome::Cancelled);
         let last = !matches!(
             result,
@@ -424,16 +511,54 @@ fn counts_rows(text: &str) -> bool {
         .is_some_and(|word| matches!(word.as_str(), "INSERT" | "UPDATE" | "DELETE" | "REPLACE"))
 }
 
+/// An agent's values as the driver takes a statement's: each as its own
+/// type, by the protocol that binds them. A value is never written into
+/// the statement.
+fn bound(params: &[Param]) -> Params {
+    use mysql_async::Value as MySql;
+    if params.is_empty() {
+        return Params::Empty;
+    }
+    let value = |param: &Param| match param {
+        Param::Null => MySql::NULL,
+        Param::Bool(flag) => MySql::Int(i64::from(*flag)),
+        Param::Int(number) => MySql::Int(*number),
+        Param::Float(number) => MySql::Double(*number),
+        Param::Text(text) => MySql::Bytes(text.as_bytes().to_vec()),
+    };
+    Params::Positional(params.iter().map(value).collect())
+}
+
+/// The outcome of an agent's statement in which the driver reads a
+/// `:name`. One written as code did not pass the check, so this one stands
+/// where the driver's own lexer lost its place: see `driver_parameter`.
+fn driver_misread() -> StatementOutcome {
+    StatementOutcome::Error {
+        error: Error::query(
+            "the MySQL driver reads a :name in a string, a quoted name or a comment of this \
+             statement as a parameter; put a space after the - or / that comes before it",
+        ),
+        position: None,
+    }
+}
+
 /// Runs one statement of a script. A statement's failure is its outcome;
 /// whether it also ended the transaction, the next check asks. `Err` is a
 /// lost session.
+///
+/// `params` are an agent's values. The SQL editor has none, and its
+/// statement with a placeholder does not run, as before.
 async fn run_statement(
     conn: &mut mysql_async::Conn,
     text: &str,
     limit: usize,
+    params: Option<&[Param]>,
 ) -> Result<StatementOutcome> {
     if let Some(outcome) = driver_parameter(text) {
-        return Ok(outcome);
+        return Ok(match params {
+            None => outcome,
+            Some(_) => driver_misread(),
+        });
     }
     // Prepare first: the server refuses a second statement hidden in one
     // piece, and what the prepared protocol lacks. A prepare error is the
@@ -442,12 +567,21 @@ async fn run_statement(
         Ok(statement) => statement,
         Err(error) => return failed(error),
     };
-    // The driver closes the connection when a statement runs without
-    // values for its parameters, so such a statement does not run.
-    if statement.num_params() > 0 {
-        return Ok(parameter("?", None));
-    }
-    let mut result = match conn.exec_iter(&statement, Params::Empty).await {
+    let counted = usize::from(statement.num_params());
+    let values = match params {
+        // The driver closes the connection when a statement runs without
+        // values for its parameters, so such a statement does not run.
+        None if counted > 0 => return Ok(parameter("?", None)),
+        None => Params::Empty,
+        // The server's own count, which is the one that holds.
+        Some(params) if counted != params.len() => {
+            let error = count_mismatch(counted, params.len());
+            let position = None;
+            return Ok(StatementOutcome::Error { error, position });
+        }
+        Some(params) => bound(params),
+    };
+    let mut result = match conn.exec_iter(&statement, values).await {
         Ok(result) => result,
         Err(error) => return failed(error),
     };
diff --git a/crates/tabletist-db/src/mysql/script/write.rs b/crates/tabletist-db/src/mysql/script/write.rs
index 4ccb93c..84df825 100644
--- a/crates/tabletist-db/src/mysql/script/write.rs
+++ b/crates/tabletist-db/src/mysql/script/write.rs
@@ -155,7 +155,7 @@ async fn statements(
             }
         }
         let started = Instant::now();
-        let result = run_statement(conn, text, limit).await?;
+        let result = run_statement(conn, text, limit, None).await?;
         outcome.stopped |= matches!(result, StatementOutcome::Cancelled);
         let last = !matches!(
             result,
```

- [ ] **Step 4: Run the tests**

Run: `cargo test --locked -p tabletist-db --lib --test agent --test mysql`

Expected: `443 passed; 0 failed; 1 ignored` for the library, `20 passed` for `tests/agent.rs` (SQLite's five, PostgreSQL's seven, MySQL's eight), and `99 passed` for `tests/mysql.rs`, which is the SQL editor's run on MySQL and has not changed.

- [ ] **Step 5: Commit**

```bash
cargo fmt --all
cargo clippy --locked -p tabletist-db --all-targets -- -D warnings
git add crates/tabletist-db/src crates/tabletist-db/tests/agent.rs
git commit -m "Run an agent's read on MySQL"
```

---

### Task 7: CI, the whole workspace, and the pull request

`ci.yml` names the test files its PostgreSQL and MySQL jobs run, so a new file runs against a server only once it is listed. The default job runs it too, on every platform, where only its SQLite tests do anything.

**Files:**
- Modify: `.github/workflows/ci.yml`

- [ ] **Step 1: List the new file in both jobs**

```diff
diff --git a/.github/workflows/ci.yml b/.github/workflows/ci.yml
index 4d66d08..dca1b88 100644
--- a/.github/workflows/ci.yml
+++ b/.github/workflows/ci.yml
@@ -88,7 +88,7 @@ jobs:
             pg_isready -h localhost -p 55432 && break
             sleep 1
           done
-      - run: cargo test --locked -p tabletist-db --lib --test postgres --test insert_audit --test lookup
+      - run: cargo test --locked -p tabletist-db --lib --test postgres --test insert_audit --test lookup --test agent
         env:
           TABLETIST_TEST_PG_URL: postgres://tabletist:tabletist@localhost:55432/tabletist
 
@@ -102,7 +102,7 @@ jobs:
       - uses: Swatinem/rust-cache@v2
       - name: Start MySQL
         run: docker compose up -d --wait mysql
-      - run: cargo test --locked -p tabletist-db --lib --test mysql --test insert_audit --test lookup
+      - run: cargo test --locked -p tabletist-db --lib --test mysql --test insert_audit --test lookup --test agent
         env:
           TABLETIST_TEST_MYSQL_URL: mysql://tabletist:tabletist@localhost:53306/tabletist
 
```

- [ ] **Step 2: Run every check `AGENTS.md` names**

```bash
cargo fmt --all --check
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo test --locked --workspace --all-targets
RUSTDOCFLAGS='-D warnings' cargo doc --locked --workspace --no-deps
```

Expected: all four pass. `tests/engines.rs` reads the new code with the rest of the database layer: it passes because every `match` on `Dialect` names its three variants, and `run_agent_read` compares two dialects with each other, not one with a variant.

Then what each server job runs, with its variable set:

```bash
cargo test --locked -p tabletist-db --lib --test postgres --test insert_audit --test lookup --test agent
cargo test --locked -p tabletist-db --lib --test mysql --test insert_audit --test lookup --test agent
```

Expected, for each: the library's `443 passed; 0 failed; 1 ignored`, `tests/agent.rs` with `20 passed`, and the server's own suite (`83 passed` for PostgreSQL, `99 passed` for MySQL) beside `insert_audit` and `lookup`.

- [ ] **Step 3: Commit**

```bash
git add .github/workflows/ci.yml
git commit -m "Run the agent tests against PostgreSQL and MySQL in CI"
```

- [ ] **Step 4: Open the pull request**

Its description says:

- what `run_agent_read` does on each engine, and that nothing calls it yet, so nothing the user sees changes;
- that values are bound on PostgreSQL, not written into the statement, against the master plan's first design;
- that MySQL's optimizer hints are refused, with what each was seen to do;
- that the SQL editor's run is the same code with `None` in two places, and which suites show that;
- what was not run: MariaDB;
- that an existing MySQL test volume needs the `bookshop` database made once (Task 1, Step 4);
- what moved out of Phase 1a: the size cap, to 1b.

No website page changes: no behaviour, setting, file or key a user sees is touched.

## Self-review

- **Against the master plan, B.1 and B.2.** The statement timeout on each engine: Tasks 4, 5 and 6, each with a test that a read over its time is ended and the session goes on. Parameters on each engine: the same tasks, with the database's own count deciding. Both differ from the master plan's first wording (no `lock_timeout`, no inlining on PostgreSQL), and the master plan is corrected in this pull request.
- **Against the master plan, F.2, without hidden tables and masking.** Parameters: strings with quotes and backslashes, numbers at the ends of 64 bits, null, a placeholder in a string, a count that does not match (the check's, from run 1, and here the database's). Timeout: by the server's own. Row cap: the limit and `truncated`. Session: after a read, a failure, a timeout and a stop the next read answers, the timeout is gone, and every table of the Bookshop counts what it did. Writes through `run_query` and the refusals are run 1's, tested without a database, and are not run again here against one: a statement the check refuses never reaches `run_agent_read`, which takes a `Read`.
- **Not covered, and said above.** The size cap and "N of M" (Phase 1b). A function of the database's users that writes, which the read-only transaction must refuse: it needs a function in the fixture (Phase 1c), and the script suites already show it for the SQL editor's run, which is this run. The backend's timer behind the database's (Phase 1b). MariaDB.
- **Names.** `Limits { rows, timeout }`, `Answer { columns, rows, truncated, elapsed }`, `Sent { params, timeout }`, `count_mismatch`, `answer`, `Adapter::run_read`, `Connection::run_agent_read`, `Conn::agent_read` and `read_only` (PostgreSQL and MySQL) are spelled the same in every task. `Sent::timeout_ms` arrives with its first caller in Task 5, and `timed_out` with MySQL in Task 6.
- **Reviewed before it was written.** A reviewer was given the draft and both servers, and asked to break it: to outlast the timeout, to lift the row limit, to send a value that is read as SQL or ends the session, to leave something on the session, and to find where the SQL editor's run changed. Nothing it tried went through. What it ran beyond these tests: on MySQL, the timeout against a correlated subquery, a `HAVING`, an `ORDER BY` and a window function; on PostgreSQL, values at the ends of what a float and an integer hold, a 200 kB text and letters of other scripts, each answered or refused by the server with the session reading on. It suspected one thing by reading: that MySQL's limit does not cover a wait for a lock. That was then tried, and it does: `the_time_limit_ends_a_wait_for_a_lock_on_mysql` is the test, with PostgreSQL's beside it.
- **Every commit stands.** After Task 4, PostgreSQL and MySQL answer `not supported` to an agent's read, and nothing calls it. The SQL editor's run is unchanged at each commit: the existing suites were run at Tasks 4, 5 and 6.

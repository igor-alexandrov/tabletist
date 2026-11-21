# Export in Two Round Trips Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use rubypowers:subagent-driven-development (recommended) or rubypowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A PostgreSQL export waits on the server twice for a SQL result and three times for a table, however many rows it has, and an SSH tunnel carries those rows at about 80 MB/s on a 200 ms link. At 200 ms: 200,000 rows × 10 columns to CSV 10.6 s → about 1.6 s, and a 64 MB result through SSH 17.7 s → about 3.5 s.

**Architecture:** `Conn::stream_text` (`crates/tabletist-db/src/pg.rs`) stops reading through a cursor with one `FETCH` round trip per 5,000-row batch. It sends `BEGIN READ ONLY` and the prepare in one flight, then the run (behind `READ_GUARD`) and its `ROLLBACK` in a second, and reads the rows off a `simple_query_raw` stream as they arrive. tokio-postgres stops reading the socket while a batch waits on the export's channel, so memory stays at a batch or so. The SSH client advertises a 32 MiB window instead of russh's 2 MiB, which held a tunnel to about 5 MB/s at 200 ms (russh tops a window up only once half of it is used, so about half of it arrives each round trip).

**Tech Stack:** Rust, tokio-postgres 0.7.18 (`simple_query_raw`), russh 0.65.1 (`client::Config::window_size`), the PostgreSQL test server (compose, port 55432).

---

## Why it is slow today, and why two flights and not one

Measured on main `55c6f276` (2026-11-20), release build, a proxy adding 100 ms each way (TCP handshake included), PostgreSQL 17, 7 runs after a warm-up, medians:

| Test | Local | 200 ms link |
|---|---|---|
| Export 200,000 rows × 10 columns to CSV (24 MB) | 0.17 s | 10.6 s (connect alone 1.0 s) |
| 64 MB result (340,000 rows) through an SSH tunnel | 0.80 s | 17.7 s (connect alone 2.2 s) |

TablePro 0.88 beta claims 0.08 s for the first (local) and 0.68 s for the second.

- **Today's trips** (`stream_text`, pg.rs:504): `BEGIN READ ONLY`, prepare, `DECLARE … CURSOR`, then `FETCH 5000` until one comes back empty, then `ROLLBACK`, each waited on alone. For 200,000 rows that is 41 fetches + 4 trips + the table's shape (1) ≈ 46 trips, plus connect: 10.4 s at 200 ms, as measured.
- **SSH:** russh's default `window_size` is 2 MiB (`russh-0.65.1/src/client/mod.rs:2761`). The server may have at most that much unacknowledged data in flight, and russh tops it up only once half of it is used, so a 200 ms link carries about 5 MB/s. Today a FETCH batch (~0.9 MB) is smaller than the window, so the round trips hide it. Once rows stream in one run, the window is what limits a 64 MB transfer (≈ 12 s). russh tops the window up as data *arrives* (`client/encrypted.rs:597-606`, `session.rs:321`), not as it is consumed. Its backpressure is `channel_buffer_size` (100 messages), after which it stops reading the socket. A bigger window costs Tabletist no memory of its own; the server may buffer up to the window for a channel whose reader stalls. (Corrected in review: the first draft took the throughput as window ÷ round trip, twice what russh gives.)
- **Not one flight:** the run could go out with the prepare, as `read_alongside` does for a page. But `client.prepare` looks up any column type the session has not seen (an enum, a domain, an extension's type: `get_type` in `tokio-postgres-0.7.18/src/prepare.rs:89-97`), and that lookup is sent *behind* the run. Its answer would come after every row of the run. The rows cannot be typed (`value_from_pg_text` needs the type's name) until the prepare answers, so the export would have to hold every row in memory or wait forever. Waiting for the prepare first costs one round trip (0.2 s at 200 ms) and keeps memory at a batch.
- **Backpressure holds:** tokio-postgres gives each request a channel of one message chunk (`client.rs:97`). When it is full, the connection keeps the chunk in `pending_responses` and stops reading the socket (`connection.rs:146-166`). So a run whose batch waits on `to` stops the server at TCP's window.
- **MySQL** already streams one statement unbuffered (`mysql.rs:882`, `stream_sql`): not part of this plan.

## Behaviour that changes

- `stream_statement` on PostgreSQL no longer runs through a cursor, so `SHOW search_path` and `EXPLAIN …` stream their rows instead of failing with "syntax error at or near SHOW". The test that pinned the failure (`tests/export.rs:524-529`) now pins the rows. Read-only stays the server's job: the run is inside `BEGIN READ ONLY` behind `READ_GUARD` on a read-only session, as every read is.
- `the_servers_timeout_is_told_apart_on_postgres` set the timeout from inside the first `FETCH`'s statement for the second to run over. With one run there is no second statement, so the test takes the timeout from a database whose sessions have one (`ALTER DATABASE … SET statement_timeout`).

## Files

- Modify `crates/tabletist-db/src/pg.rs`: `stream_text` (two flights, a stream), drop `EXPORT_CURSOR`. Tests: a trip count, a custom-typed column on a fresh session, a text of two statements.
- Modify `crates/tabletist-db/src/lib.rs:534-541, 566-577`: the doc comments that say "through a cursor".
- Modify `crates/tabletist-db/tests/export.rs:524-529, 543-563`: `SHOW` streams; the timeout test.
- Modify `crates/tabletist-db/src/ssh.rs:412-417`: `TUNNEL_WINDOW` and `client_config()`, with a test.
- Modify `docs/_guide/speed-and-security.md`: one paragraph on exports.

The PostgreSQL lib tests and `--test export` already run in CI's PostgreSQL job (`ci.yml:132`), so nothing is added to `ci.yml`.

Every test command below needs the test server (`ss -ltn | grep 55432`; without Docker it may already be up) and runs from the worktree root:

```bash
export TABLETIST_TEST_PG_URL=postgres://tabletist:tabletist@localhost:55432/tabletist
export CARGO_TARGET_DIR=$PWD/target
```

---

### Task 1: A PostgreSQL export streams in two flights

**Files:**
- Modify: `crates/tabletist-db/src/pg.rs:414-416` (`EXPORT_CURSOR`), `:501-554` (`stream_text`), tests module (after `a_page_asks_the_server_once`, ~line 2216)
- Modify: `crates/tabletist-db/src/lib.rs:534-541, 566-577`
- Modify: `crates/tabletist-db/tests/export.rs:518-563`

- [ ] **Step 1: Write the failing tests**

In `crates/tabletist-db/src/pg.rs`, in `mod tests`, after `a_page_asks_the_server_once` (ends ~line 2216), add:

```rust
    /// A table of 12,000 rows with an enum column, for the export tests.
    /// Made once: two tests that create it at once can fail.
    async fn export_trips_table(url: &str) {
        static MADE: tokio::sync::OnceCell<()> = tokio::sync::OnceCell::const_new();
        MADE.get_or_init(|| async {
            let mut config: tokio_postgres::Config = url.parse().unwrap();
            config.ssl_mode(tokio_postgres::config::SslMode::Disable);
            let (client, connection) = config.connect(tokio_postgres::NoTls).await.unwrap();
            tokio::spawn(connection);
            client
                .batch_execute(
                    "DO $$ BEGIN CREATE TYPE export_mood AS ENUM ('calm', 'busy'); \
                     EXCEPTION WHEN duplicate_object THEN NULL; END $$; \
                     CREATE TABLE IF NOT EXISTS export_trips ( \
                         id int PRIMARY KEY, mood export_mood NOT NULL); \
                     INSERT INTO export_trips \
                     SELECT g, CASE WHEN g % 2 = 0 THEN 'calm' ELSE 'busy' END::export_mood \
                     FROM generate_series(1, 12000) g ON CONFLICT DO NOTHING",
                )
                .await
                .unwrap();
        })
        .await;
    }

    /// What an export of `text` sent, read as the app's export reads it
    /// (a channel that holds one batch): its answer, its columns and its
    /// batches.
    async fn exported(
        conn: &Conn,
        text: &str,
        batch: usize,
    ) -> (Result<u64>, Vec<ColumnMeta>, Vec<Vec<Vec<Value>>>) {
        let (to, mut from) = tokio::sync::mpsc::channel(1);
        let stop = StopFlag::new();
        let reading = async move {
            let (mut columns, mut batches) = (Vec::new(), Vec::new());
            while let Some(streamed) = from.recv().await {
                match streamed {
                    crate::Streamed::Columns(metas) => columns = metas,
                    crate::Streamed::Rows(rows) => batches.push(rows),
                }
            }
            (columns, batches)
        };
        let (sent, (columns, batches)) = tokio::join!(
            async move {
                let sent = conn.stream_statement(text, batch, &to, &stop).await;
                drop(to);
                sent
            },
            reading,
        );
        (sent, columns, batches)
    }

    /// An export waits on the server twice, however many rows it has: the
    /// `BEGIN` and the prepare, then the run and its `ROLLBACK`, with the
    /// rows streaming back behind them. A `FETCH` of a batch at a time
    /// waited once for every batch.
    #[tokio::test]
    async fn an_export_asks_the_server_twice_whatever_its_rows() {
        let Some(url) = test_url() else {
            return;
        };
        export_trips_table(&url).await;
        let (conn, trips) = trip_counted(&url).await;
        let text = "SELECT id FROM export_trips ORDER BY id";
        let ((sent, columns, batches), asked) =
            counted(&trips, exported(&conn, text, 5_000)).await;
        assert_eq!(sent.unwrap(), 12_000);
        assert_eq!(columns.len(), 1);
        assert_eq!(columns[0].name, "id");
        let sizes: Vec<usize> = batches.iter().map(Vec::len).collect();
        assert_eq!(sizes, [5_000, 5_000, 2_000]);
        assert_eq!(batches[0][0], vec![Value::Int(1)]);
        assert_eq!(batches[2][1_999], vec![Value::Int(12_000)]);
        assert_eq!(asked, 2);
        assert!(outside_a_transaction(&conn).await);
    }

    /// A column of a type the session has not seen yet has the driver
    /// look the type up while it prepares. That lookup is answered before
    /// the run goes out, so it does not wait behind the run's rows: the
    /// export ends. Once the type is known, two round trips again.
    #[tokio::test]
    async fn an_export_of_a_type_the_session_has_not_seen_ends() {
        let Some(url) = test_url() else {
            return;
        };
        export_trips_table(&url).await;
        let (conn, trips) = trip_counted(&url).await;
        let text = "SELECT id, mood FROM export_trips ORDER BY id";
        let first = tokio::time::timeout(Duration::from_secs(30), exported(&conn, text, 5_000))
            .await
            .expect("the export waits on nothing that waits on it");
        let (sent, columns, batches) = first;
        assert_eq!(sent.unwrap(), 12_000);
        assert_eq!(columns[1].type_name, "export_mood");
        assert_eq!(
            batches[0][0],
            vec![Value::Int(1), Value::Text("busy".into())]
        );
        let ((sent, _, _), asked) = counted(&trips, exported(&conn, text, 5_000)).await;
        assert_eq!(sent.unwrap(), 12_000);
        assert_eq!(asked, 2);
        assert!(outside_a_transaction(&conn).await);
    }

    /// A text of two statements is refused by the prepare, and nothing
    /// of it runs: no columns, no rows, no transaction left open.
    #[tokio::test]
    async fn an_export_of_two_statements_runs_neither() {
        let Some(url) = test_url() else {
            return;
        };
        let conn = session(&url).await;
        let (sent, columns, batches) = exported(&conn, "SELECT 1; SELECT 2", 10).await;
        assert!(sent.is_err(), "{sent:?}");
        assert!(columns.is_empty() && batches.is_empty());
        assert!(outside_a_transaction(&conn).await);
    }
```

`ColumnMeta`, `Value`, `StopFlag`, `Duration` and the `Adapter` trait (for `conn.stream_statement`) are in scope through `use super::*` (pg.rs:5-23 import them). `Value::Text` holds a `Box<str>`, so `"busy".into()` builds it; an `int4` is `Value::Int` (value.rs:116).

- [ ] **Step 2: Run the tests to make sure the trip test fails**

```bash
~/.cargo/bin/cargo test --locked -p tabletist-db --lib pg::tests::an_export_ -- --test-threads=1
```

Expected: `an_export_asks_the_server_twice_whatever_its_rows` FAILS at `assert_eq!(asked, 2)` with `left: 8` (BEGIN, prepare, DECLARE, four FETCHes, ROLLBACK). `an_export_of_a_type_the_session_has_not_seen_ends` FAILS at its second `assert_eq!(asked, 2)` with `left: 8` too. `an_export_of_two_statements_runs_neither` PASSES (today's prepare refuses it as well): it keeps the refusal from regressing.

- [ ] **Step 3: Rewrite `stream_text`**

In `crates/tabletist-db/src/pg.rs`, delete the `EXPORT_CURSOR` constant and its doc comment (lines 414-416):

```rust
/// The cursor an export reads through. One export per connection, so one
/// name.
const EXPORT_CURSOR: &str = "tabletist_export";
```

Replace `stream_text` (from its doc comment `/// \`text\`'s rows through a cursor in a read-only transaction, a` to the closing brace after `Ok(sent)`, lines 502-554) with:

```rust
    /// `text`'s rows, read in a read-only transaction and sent to `to` in
    /// batches of `batch` as they arrive: two round trips, however many
    /// rows. The first is the `BEGIN READ ONLY` and the prepare, which
    /// types the columns and fails on a text of more than one statement.
    /// The second is the run, behind [`READ_GUARD`], and the `ROLLBACK`,
    /// sent together; the rows come back behind them. While a batch waits
    /// on `to`, the driver stops reading the socket, so the server waits
    /// too and no more than a batch or so is held.
    ///
    /// The prepare is waited on before the run goes out, not sent with it
    /// as [`read_alongside`] does: a column of a type the session has not
    /// seen (an enum, a domain) has the driver look the type up, and that
    /// lookup would go out behind the run and be answered after its last
    /// row, which could not be typed until then.
    async fn stream_text(
        &self,
        text: &str,
        batch: usize,
        to: &tokio::sync::mpsc::Sender<crate::Streamed>,
        stop: &StopFlag,
    ) -> Result<u64> {
        use futures_util::StreamExt;

        let client = self.client.lock().await;
        let (begun, prepared) = tokio::join!(
            client.batch_execute("BEGIN READ ONLY"),
            client.prepare(text),
        );
        let statement = match begun.and(prepared) {
            Ok(statement) => statement,
            Err(error) => {
                // A failed prepare leaves a failed transaction; a failed
                // BEGIN, none, and the rollback says so harmlessly.
                let _ = client.batch_execute("ROLLBACK").await;
                return Err(stream_error(error));
            }
        };
        let metas = column_metas(&statement);
        // Its close goes out on its own, waited on by nobody.
        drop(statement);
        crate::stream::send(to, crate::Streamed::Columns(metas.clone())).await?;

        let run = format!("{READ_GUARD};\n{text}");
        let rows = client.simple_query_raw(&run).await.map_err(stream_error)?;
        // The ROLLBACK goes out on its first poll, right behind the run, so
        // it costs no round trip of its own; its answer follows the run's.
        let rollback = client.batch_execute("ROLLBACK");
        let mut rollback = std::pin::pin!(rollback);
        let mut rolled_back = None;
        std::future::poll_fn(|cx| {
            if let std::task::Poll::Ready(done) = rollback.as_mut().poll(cx) {
                rolled_back = Some(done);
            }
            std::task::Poll::Ready(())
        })
        .await;

        let mut rows = std::pin::pin!(rows);
        let mut kept = Vec::with_capacity(batch);
        let mut sent = 0u64;
        while let Some(message) = rows.next().await {
            let SimpleQueryMessage::Row(row) = message.map_err(stream_error)? else {
                continue;
            };
            kept.push(row_values(&row, &metas)?);
            if kept.len() == batch {
                if stop.is_stopped() {
                    return Err(Error::Cancelled);
                }
                sent += kept.len() as u64;
                let full = std::mem::replace(&mut kept, Vec::with_capacity(batch));
                crate::stream::send(to, crate::Streamed::Rows(full)).await?;
            }
        }
        if stop.is_stopped() {
            return Err(Error::Cancelled);
        }
        if !kept.is_empty() {
            sent += kept.len() as u64;
            crate::stream::send(to, crate::Streamed::Rows(kept)).await?;
        }
        stop.finish();
        let rolled_back = match rolled_back {
            Some(done) => done,
            None => rollback.await,
        };
        rolled_back.map_err(stream_error)?;
        Ok(sent)
    }
```

Notes for the implementer:
- `rollback.as_mut().poll(cx)` needs `std::future::Future` in scope; pg.rs:5 imports it.
- `futures-util` is already a dependency of `tabletist-db` (`Cargo.toml:58`, `features = ["std"]`), and `StreamExt::next` needs no other feature. `Pin<&mut SimpleQueryStream>` is `Unpin`, so `rows.next()` works on the pinned stream.
- On an error part way (`?` inside the loop), the function returns without awaiting the ROLLBACK. It is already on its way, and `Connection::stream_rows` says a connection whose stream failed is to be closed (lib.rs:544-548). Today's code returns the same way.
- `stop.is_stopped()` is checked before each batch is sent and before the last one, as today's loop checked before each `FETCH`. A stop that comes while rows are arriving is also a cancel of the statement (`Stopper::halt`), which ends the stream with `QUERY_CANCELED`, mapped to `Error::Cancelled` by `stream_error`.

- [ ] **Step 4: Run the new tests to make sure they pass**

```bash
~/.cargo/bin/cargo test --locked -p tabletist-db --lib pg::tests::an_export_ -- --test-threads=1
```

Expected: 3 passed.

- [ ] **Step 5: Update the export integration tests for what changed**

In `crates/tabletist-db/tests/export.rs`, `a_statement_streams_on_postgres` (lines 518-531): replace

```rust
        // A statement no cursor holds fails with the server's words.
        let shown = streamed_statement(&connection, "SHOW search_path", 10, &StopFlag::new()).await;
        let Err(error) = shown else {
            panic!("{shown:?}");
        };
        assert!(error.to_string().contains("syntax error"), "{error}");
```

with

```rust
        // A statement no cursor could hold streams as well.
        let shown = streamed_statement(&connection, "SHOW search_path", 10, &StopFlag::new())
            .await
            .unwrap();
        assert_eq!(shown.columns, ["search_path"]);
        assert_eq!(shown.sent, 1);
```

Replace `the_servers_timeout_is_told_apart_on_postgres` (lines 543-563, its doc comment included) with:

```rust
    /// The server's `statement_timeout` ends a stream as a timeout, not as
    /// a cancel. An export sets no timeout of its own: this one is the
    /// database's, as a server's configuration or a role's would give it.
    #[tokio::test]
    async fn the_servers_timeout_is_told_apart_on_postgres() {
        let Some(url) = url() else {
            eprintln!("skipped: TABLETIST_TEST_PG_URL is not set");
            return;
        };
        let server = admin(&url, None).await;
        let made = server
            .query_opt(
                "SELECT 1 FROM pg_database WHERE datname = 'export_timeout'",
                &[],
            )
            .await
            .unwrap()
            .is_some();
        if !made {
            server
                .batch_execute("CREATE DATABASE export_timeout")
                .await
                .unwrap();
        }
        server
            .batch_execute("ALTER DATABASE export_timeout SET statement_timeout = 300")
            .await
            .unwrap();
        let (mut spec, secrets) = ConnectSpec::from_url(&url).unwrap();
        spec.tls = TlsMode::Disable;
        spec.database = "export_timeout".into();
        let connection =
            Connection::connect_with(&spec, &secrets, &HostKeys::default(), Access::ReadOnly)
                .await
                .unwrap();
        let ended =
            streamed_statement(&connection, "SELECT pg_sleep(2)", 2, &StopFlag::new()).await;
        assert!(
            matches!(ended, Err(Error::Timeout)),
            "{:?}",
            ended.map(|got| got.sent)
        );
    }
```

`url`, `admin` and `TlsMode` are already in `mod postgres` of this file (lines 405-425). The timeout is 300 ms so the connect's own catalog reads stay well under it on a slow CI runner, and the sleep is 2 s so the statement is well over it. The `ALTER DATABASE` runs every time, so a database left by a run that stopped between the two statements still gets its timeout.

- [ ] **Step 6: Update the doc comments that name the cursor**

In `crates/tabletist-db/src/lib.rs`, in the doc comment of `stream_rows` (lines 534-541), replace

```rust
    /// batches of up to `batch` rows, read-only: through a cursor on
    /// PostgreSQL, unbuffered on MySQL, stepped on SQLite, so no more than
    /// a batch is read ahead of what `to` has taken; on a Files connection
```

with

```rust
    /// batches of up to `batch` rows, read-only: one run streamed in two
    /// round trips on PostgreSQL, unbuffered on MySQL, stepped on SQLite,
    /// so no more than a batch is read ahead of what `to` has taken (on
    /// PostgreSQL, a batch and what the socket holds); on a Files connection
```

In the doc comment of `stream_statement` (lines 574-577), replace

```rust
    /// [`Error::Timeout`]. On PostgreSQL it runs as a cursor, so a
    /// statement no cursor holds (`SHOW`, `EXPLAIN`) fails with the
    /// server's words. A connection whose stream failed once its statement
    /// ran is to be closed, as for [`Self::stream_rows`].
```

with

```rust
    /// [`Error::Timeout`]. On PostgreSQL a `SHOW` or an `EXPLAIN` streams
    /// as a `SELECT` does. A connection whose stream failed once its
    /// statement ran is to be closed, as for [`Self::stream_rows`].
```

Then check nothing else names the cursor:

```bash
grep -rn -i "cursor\|FETCH " crates/tabletist-db/src/pg.rs crates/tabletist-db/src/lib.rs crates/tabletist-db/src/stream.rs crates/tabletist-core/src/export.rs crates/tabletist-core/src/backend/export.rs
```

Expected: no line about the export (pg.rs's grid cursor and editor mentions, if any, are not the export's).

- [ ] **Step 7: Run every PostgreSQL test the CI job runs, and the export tests elsewhere**

```bash
~/.cargo/bin/cargo test --locked -p tabletist-db --lib --test postgres --test insert_audit --test lookup --test delete_rows --test agent --test ddl --test export
```

Expected: all pass, among them `two_hundred_thousand_rows_stream_on_postgres`, `a_stop_ends_the_stream_on_postgres`, `the_export_session_cannot_write_on_postgres` and the two rewritten tests.

```bash
~/.cargo/bin/cargo test --locked -p tabletist-core --lib backend::export
```

Expected: all pass (SQLite; it checks nothing broke above the driver).

- [ ] **Step 8: Format and lint**

```bash
~/.cargo/bin/cargo fmt --all --check && ~/.cargo/bin/cargo clippy --locked --workspace --all-targets -- -D warnings
```

Expected: no output from fmt, clippy finishes with no warnings.

- [ ] **Step 9: Commit**

```bash
git add crates/tabletist-db/src/pg.rs crates/tabletist-db/src/lib.rs crates/tabletist-db/tests/export.rs && git commit -m "Stream a PostgreSQL export in two round trips, not one per batch"
```

If signing fails with "agent refused operation", 1Password is locked: leave it staged and say so.

---

### Task 2: An SSH tunnel keeps a far link busy

**Files:**
- Modify: `crates/tabletist-db/src/ssh.rs:412-417` and its `mod tests` (line 573)

- [ ] **Step 1: Write the failing test**

In `crates/tabletist-db/src/ssh.rs`, at the top of `mod tests` (after `use crate::ssh_config::{HostConfig, Proxy};`, line 575), add:

```rust
    /// At a 200 ms round trip the window lets the server send over 100 MB
    /// a second down a tunnel; russh's default, 2 MiB, held it to about
    /// 10, so a 64 MB export took six seconds on the wire alone.
    #[test]
    fn the_tunnel_window_keeps_a_far_link_busy() {
        let config = client_config();
        let per_second = f64::from(config.window_size) / 0.2;
        assert!(per_second > 100e6, "{per_second}");
        // What the tunnel set before stays as it was.
        assert!(config.nodelay);
        assert_eq!(config.inactivity_timeout, None);
        assert_eq!(config.keepalive_interval, Some(Duration::from_secs(30)));
    }
```

- [ ] **Step 2: Run it to make sure it fails**

```bash
~/.cargo/bin/cargo test --locked -p tabletist-db --lib ssh::tests::the_tunnel_window_keeps_a_far_link_busy
```

Expected: compile error, `cannot find function 'client_config'`.

- [ ] **Step 3: Add the window and `client_config`**

In `crates/tabletist-db/src/ssh.rs`, next to `PROXY_WAITS` (line 36), add:

```rust
/// How much the server may send down a tunnel before it hears back, so at
/// most this much arrives in each round trip. russh's default, 2 MiB, held
/// a 200 ms link to about 10 MB/s; 32 MiB lets it carry 160. It costs no
/// memory of its own: russh tops the window up as data arrives and stops
/// reading the socket once `channel_buffer_size` messages wait unread.
const TUNNEL_WINDOW: u32 = 32 * 1024 * 1024;

/// The SSH client's settings for a tunnel.
fn client_config() -> client::Config {
    client::Config {
        nodelay: true,
        inactivity_timeout: None,
        keepalive_interval: Some(Duration::from_secs(30)),
        window_size: TUNNEL_WINDOW,
        ..Default::default()
    }
}
```

In `Tunnel::open_to`, replace (lines 412-417)

```rust
        let config = Arc::new(client::Config {
            nodelay: true,
            inactivity_timeout: None,
            keepalive_interval: Some(Duration::from_secs(30)),
            ..Default::default()
        });
```

with

```rust
        let config = Arc::new(client_config());
```

- [ ] **Step 4: Run the test, then the SSH tests that need no server**

```bash
~/.cargo/bin/cargo test --locked -p tabletist-db --lib ssh::tests
```

Expected: all pass; the ones that need `TABLETIST_TEST_SSH_URL` print "skipped". (`--test ssh` needs the compose SSH server, which Docker would start. CI's SSH job runs it.)

- [ ] **Step 5: Lint and commit**

```bash
~/.cargo/bin/cargo fmt --all --check && ~/.cargo/bin/cargo clippy --locked --workspace --all-targets -- -D warnings
```

```bash
git add crates/tabletist-db/src/ssh.rs && git commit -m "Let an SSH tunnel's server send 32 MiB before it hears back"
```

---

### Task 3: Measure, and say so on the website

**Files:**
- Modify: `docs/_guide/speed-and-security.md` (after the paragraph that starts "A few steps take more", ~line 46)

The measuring is throwaway and is not committed: a timing test in CI would be flaky, and the numbers belong in the PR and the guide.

- [ ] **Step 1: Set up the far link**

```bash
BENCH=/tmp/claude-1000/-home-igor-Work-tabletist--claude-worktrees-twitter-link-review-7a87a4/d25df536-15a6-47ee-8871-aae8b8fa68fd/scratchpad/bench
```

The harness from 2026-11-20 is in that folder: `tools/tcpdelay.py` (a TCP proxy adding a delay each way, the handshake included), `tools/bench_tablepro.rs` (the timing test), `sshd/sshd_config` (an unprivileged sshd on 127.0.0.1:52222 that takes the test key `crates/tabletist-db/tests/ssh/id_ed25519`), and `key`. If the scratchpad was cleared, rebuild them as below. The `sshd_config` is: `ListenAddress 127.0.0.1`, `Port 52222`, a `HostKey` of its own (`ssh-keygen -t ed25519 -N ''`), `PidFile`, `AuthorizedKeysFile` holding `id_ed25519.pub`, `PasswordAuthentication no`, `KbdInteractiveAuthentication no`, `PubkeyAuthentication yes`, `UsePAM no`, `StrictModes no`, `AllowTcpForwarding yes`, `AllowUsers $USER`.

Start them (each proxy as a background task):

```bash
/usr/bin/sshd -D -e -f "$BENCH/sshd/sshd_config"
```

```bash
python3 -I "$BENCH/tools/tcpdelay.py" 52200 52222 100
```

```bash
python3 -I "$BENCH/tools/tcpdelay.py" 55400 55432 100
```

Make the two tables:

```bash
PGPASSWORD=tabletist psql -h 127.0.0.1 -p 55432 -U tabletist -d tabletist -v ON_ERROR_STOP=1 -q <<'EOF'
DROP SCHEMA IF EXISTS bench_tablepro CASCADE;
CREATE SCHEMA bench_tablepro;
CREATE TABLE bench_tablepro.export_200k AS
SELECT g AS id, 'user_' || g AS name, 'user' || g || '@example.com' AS email,
       (g % 90) + 10 AS age, round((random() * 10000)::numeric, 2) AS balance,
       (g % 2 = 0) AS active, timestamp '2024-01-01' + (g || ' minutes')::interval AS created_at,
       md5(g::text) AS token, (g % 50) AS country_id, 'note ' || (g % 1000) AS note
FROM generate_series(1, 200000) g;
ALTER TABLE bench_tablepro.export_200k ADD PRIMARY KEY (id);
CREATE TABLE bench_tablepro.result_64mb AS
SELECT g AS id, repeat(md5(g::text), 2) AS a, repeat(md5((g+1)::text), 2) AS b, md5((g+2)::text) AS c
FROM generate_series(1, 340000) g;
ALTER TABLE bench_tablepro.result_64mb ADD PRIMARY KEY (id);
VACUUM ANALYZE bench_tablepro.export_200k; VACUUM ANALYZE bench_tablepro.result_64mb;
EOF
```

(`result_64mb` is 61 MiB = 64 MB on the PostgreSQL wire.)

- [ ] **Step 2: Measure**

```bash
cp "$BENCH/tools/bench_tablepro.rs" crates/tabletist-core/tests/ && BENCH_DIR="$BENCH" ~/.cargo/bin/cargo test --locked --release -p tabletist-core --test bench_tablepro -- --ignored --nocapture 2>&1 | grep BENCH; rm crates/tabletist-core/tests/bench_tablepro.rs
```

Expected (before → after, at 200 ms): `export 200k x 10 to CSV, 200 ms link` 10.6 s → about 1.6 s (connect 1.0 + shape, prepare and run 0.6 + the rows). `64 MB result through SSH, 200 ms link` 17.7 s → about 3.5 s (connect 2.2 + 0.6 + the rows at the window's rate). The local rows should not get slower (0.17 s and 0.80 s before). If the SSH case is still above 5 s, the window is not what limits it: check with `RUST_LOG=russh=debug` that russh's line `sender_window_size …, target 33554432` appears (each adjust then adds about 16 MiB), before changing anything else.

- [ ] **Step 3: Clean up**

Stop the two proxies and the sshd (`kill "$(cat "$BENCH/sshd/sshd.pid")"`), then:

```bash
PGPASSWORD=tabletist psql -h 127.0.0.1 -p 55432 -U tabletist -d tabletist -qc "DROP SCHEMA bench_tablepro CASCADE" && git status --short
```

Expected: only the plan's own changes, no `bench_tablepro.rs`.

- [ ] **Step 4: Say it on the speed page**

In `docs/_guide/speed-and-security.md`, after the paragraph that begins "A few steps take more:" (it ends with the SQLite sentence), add a paragraph:

```markdown
An export opens a connection of its own. On PostgreSQL it then waits on
the server three times for a table and twice for a SQL result, however
many rows it writes, and the rows come as fast as the network carries
them, through an SSH tunnel too. Through a network that adds 200 ms to
every round trip, 200,000 rows of ten columns are written to a CSV file
in about N seconds.
```

Replace `N` with the measured median from Step 2, rounded to one decimal (for example "1.6").

- [ ] **Step 5: Commit and open the PR**

```bash
git add docs/_guide/speed-and-security.md && git commit -m "Tell how few round trips an export takes on PostgreSQL"
```

Push the branch and open a PR to `main`. In the body, give the before and after table from Step 2, state the `SHOW`/`EXPLAIN` behaviour change, and note that MySQL was not measured.

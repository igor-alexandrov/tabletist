# MySQL and MariaDB Export Round Trips Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use rubypowers:subagent-driven-development (recommended) or rubypowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** On MySQL and MariaDB, a table export and a SQL result export each wait on the server twice, down from about six and four. Rows still stream one at a time, and no value changes.

**Architecture:** A table export reads its shape (primary key, byte string columns, whether any column is a FLOAT) once, in one message, and remembers it. Both `export_statement` and `stream_rows` use it. The rows then come in one message through the text protocol: the read-only start, the statement prepared from a user variable, its run, the cleanup and the rollback. This is the same message `page_in_one` sends for a page, but the run's rows stream as they arrive instead of being collected. A SQL result is first prepared through the driver, which gives its column types, and then sent as that one message. The text protocol writes a FLOAT with six significant digits, so a table or result with a FLOAT keeps today's binary path.

**Tech Stack:** Rust, mysql_async 0.37 (`QueryResult<TextProtocol>`, `Conn::query_iter`), tokio mpsc, the `counted_session` trip-counting proxy in `crates/tabletist-db/src/mysql.rs` tests.

---

## Measured before this plan (2026-11-21, MySQL 8.4.11, 100 ms each way)

| Case | Export | Connecting alone |
| --- | --- | --- |
| 200k rows × 10 columns to CSV, 200 ms link | 2,273 ms | 704 ms |
| 64 MB result through SSH, 200 ms link | 3,789 ms | 1,907 ms |
| 200k rows to CSV, local | 158 ms | 1 ms |
| 64 MB through SSH, local | 356 ms | 113 ms |

Leaving out connecting, the export takes about 1.57 s at 200 ms, against 0.75 s for PostgreSQL on this branch.

The round trips today, from `crates/tabletist-db/src/mysql.rs`:

- `export_statement`: the primary key (1), and the byte string columns when a filter reads bytes (0 or 1).
- `stream_rows`: the primary key again (1), the byte string columns again (0 or 1), `START TRANSACTION READ ONLY` (1), prepare (1), execute (1), `ROLLBACK` (1).
- `stream_statement`: start (1), prepare (1), execute (1), rollback (1).

After this plan:

| Export | Round trips |
| --- | --- |
| Table, no FLOAT | 2: the shape, then the message |
| SQL result, no FLOAT | 2: the driver's prepare, then the message |
| Table with a FLOAT | 5: the shape, start, prepare, execute, rollback |
| SQL result with a FLOAT | 4: prepare, start, execute, rollback |

A second export of the same text on the same session skips the prepare, because the driver caches prepared statements. Exports open a session of their own, though, so the counts above are what a person sees.

## Behaviour that changes

- A SQL result export is prepared before its read-only transaction starts, not inside it. Preparing reads and writes nothing, so the read-only guarantee is unchanged. A text the prepare refuses now fails with no transaction started at all.
- A table export reads the table's shape once, for both the statement it shows and the rows. That shape is read one round trip before the rows. If a column became a FLOAT in between, the export fails and says so ("export again"), instead of writing six-digit FLOAT values.
- A stop that arrives before the rows' message is sent now keeps the statement from running, on both paths. This is the same fix PostgreSQL got in b7a6efa5.

## Files

- Create `crates/tabletist-db/src/mysql/export.rs` for the export's MySQL reads: the shape question in one message, the streaming one-message run, and its tests. `mysql.rs` is 281 KB, so this code lives in its own file next to `script.rs` and `write.rs`. As a child module, it can use the parent's private items through `super::`.
- Modify `crates/tabletist-db/src/mysql.rs`:
  - add `mod export;` next to `mod script;` (line 581);
  - add `Conn::export_shape` and `Conn::stream_in_one`, next to `stream_sql` (line 882);
  - add the stop check before the run in `stream_sql`;
  - rewrite `export_statement`, `stream_rows` and `stream_statement` (lines 1596–1630);
  - make the test helper `in_transaction` (line 4711) `pub(super)`.
- Modify `crates/tabletist-db/src/lib.rs`: the `stream_rows` doc comment (line 536) names MySQL's round trips.
- Modify `docs/_guide/speed-and-security.md`: the export paragraph names MySQL and MariaDB.

---

### Task 1: Read a table's shape once, in one message

A table export drops from about 6 round trips to 5: the duplicate key read goes, and the byte string columns are asked in the same message as the key.

**Files:**
- Create: `crates/tabletist-db/src/mysql/export.rs`
- Modify: `crates/tabletist-db/src/mysql.rs`: `mod export;` (around line 581), `Conn::export_shape` (beside `known_shape`, around line 1184), `export_statement` and `stream_rows` (lines 1596–1616), `in_transaction` (line 4711) made `pub(super)`.

- [ ] **Step 1: Create the module with the shape question and a failing test**

Create `crates/tabletist-db/src/mysql/export.rs`:

```rust
//! An export's reads on MySQL and MariaDB, in as few round trips as the
//! rows allow: a table's shape in one message, then its rows (or a SQL
//! result's) in one more, streamed as they arrive (see
//! [`Conn::stream_in_one`](super::Conn)).

use super::{BINARY_COLUMNS, PRIMARY_KEY, Shape, inlined, query_error, script, typed};
use crate::{Error, ObjectRef, Result};

/// Whether a table has a FLOAT column, as [`super::LISTED_COLUMNS`] tells
/// one: any column whose type starts with it.
const TABLE_FLOATS: &str = "SELECT EXISTS (SELECT 1 FROM information_schema.columns \
                            WHERE table_schema = ? AND table_name = ? \
                              AND LOWER(column_type) LIKE 'float%')";

/// `object`'s [`Shape`] in one round trip: its primary key, its byte
/// string columns and whether it has a FLOAT, asked as one message.
pub(super) async fn read_shape(conn: &mut mysql_async::Conn, object: &ObjectRef) -> Result<Shape> {
    let at = [object.schema.as_str(), object.name.as_str()];
    let questions = [
        inlined(PRIMARY_KEY, &at),
        inlined(BINARY_COLUMNS, &at),
        inlined(TABLE_FLOATS, &at),
    ];
    let (answers, error) = script::answers(conn, &questions).await;
    if let Some(error) = error {
        return Err(query_error(error));
    }
    let Ok([key, binary, floats]) = <[script::Answer; 3]>::try_from(answers) else {
        return Err(Error::query(
            "unexpected data from the server: an answer is missing",
        ));
    };
    Ok(Shape {
        key: typed(key.rows)?,
        binary: typed(binary.rows)?,
        floats: typed::<bool>(floats.rows)?.first().copied().unwrap_or(false),
    })
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::Ordering;

    use mysql_async::prelude::Queryable;

    use super::super::Conn;
    use super::super::tests::{counted_session, in_transaction, test_url};
    use crate::adapter::Adapter;
    use crate::{Access, ColumnMeta, ObjectRef, Result, RowQuery, StopFlag, Streamed, Value};

    /// One test at a time creates the export tables: creating one twice
    /// at once can fail.
    static TABLES: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

    /// The digits 0 to 9, as rows of `d`, to build numbered rows from on
    /// MySQL and MariaDB alike (MariaDB has no `cte_max_recursion_depth`).
    const DIGITS: &str = "(SELECT 0 d UNION ALL SELECT 1 UNION ALL SELECT 2 UNION ALL SELECT 3 \
                          UNION ALL SELECT 4 UNION ALL SELECT 5 UNION ALL SELECT 6 \
                          UNION ALL SELECT 7 UNION ALL SELECT 8 UNION ALL SELECT 9)";

    /// The export tables, in the test database, and its name:
    /// `export_trips` (12,000 rows of an `int` key and a name),
    /// `export_floats` (one FLOAT row) and `export_types` (a row of each
    /// common type, and a row of NULLs).
    async fn export_tables(url: &str) -> String {
        let _turn = TABLES.lock().await;
        let opts =
            mysql_async::Opts::from_url(&format!("{url}?prefer_socket=false")).unwrap();
        let mut admin = mysql_async::Conn::new(opts).await.unwrap();
        admin
            .query_drop(format!(
                "CREATE TABLE IF NOT EXISTS export_trips (id int PRIMARY KEY, name varchar(20)); \
                 INSERT IGNORE INTO export_trips (id, name) \
                 SELECT n + 1, CONCAT('row ', n + 1) FROM ( \
                   SELECT a.d + 10 * b.d + 100 * c.d + 1000 * e.d + 10000 * f.d AS n \
                   FROM {DIGITS} a, {DIGITS} b, {DIGITS} c, {DIGITS} e, {DIGITS} f) g \
                 WHERE n < 12000; \
                 CREATE TABLE IF NOT EXISTS export_floats (id int PRIMARY KEY, f float); \
                 INSERT IGNORE INTO export_floats VALUES (1, 1.2345678); \
                 CREATE TABLE IF NOT EXISTS export_types (id int PRIMARY KEY, \
                   big bigint unsigned, money decimal(10, 2), ratio double, \
                   stamp datetime(3), day date, span time, yr year, name varchar(20), \
                   bytes varbinary(4), doc json); \
                 INSERT IGNORE INTO export_types VALUES \
                   (1, 18446744073709551615, 12.34, 0.1, '2024-01-02 03:04:05.678', \
                    '2024-01-02', '-01:02:03', 2024, 'ñame', 0x00ff10, '{{\"a\": [1, 2]}}'), \
                   (2, NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL)"
            ))
            .await
            .unwrap();
        admin
            .query_first("SELECT DATABASE()")
            .await
            .unwrap()
            .unwrap()
    }

    /// A table export as the export runs one: the statement it shows,
    /// then its rows streamed, with what `to` was sent.
    async fn exported_table(
        conn: &Conn,
        query: &RowQuery,
        columns: &[String],
        stop: &StopFlag,
    ) -> (Result<String>, Result<u64>, Vec<ColumnMeta>, Vec<Vec<Vec<Value>>>) {
        let shown = conn.export_statement(query, columns).await;
        let (sent, metas, batches) = streamed(|to| async move {
            conn.stream_rows(query, columns, 5_000, &to, stop).await
        })
        .await;
        (shown, sent, metas, batches)
    }

    /// A SQL result's export: what it sent `to`.
    async fn exported_statement(
        conn: &Conn,
        text: &str,
        stop: &StopFlag,
    ) -> (Result<u64>, Vec<ColumnMeta>, Vec<Vec<Vec<Value>>>) {
        streamed(|to| async move {
            conn.stream_statement(text, 5_000, &to, stop).await
        })
        .await
    }

    /// Runs `stream` with a sender whose receiver is read as it goes, as
    /// the export's writer reads it: the outcome, the columns, the batches.
    async fn streamed<F, Fut>(stream: F) -> (Result<u64>, Vec<ColumnMeta>, Vec<Vec<Vec<Value>>>)
    where
        F: FnOnce(tokio::sync::mpsc::Sender<Streamed>) -> Fut,
        Fut: std::future::Future<Output = Result<u64>>,
    {
        let (to, mut from) = tokio::sync::mpsc::channel(1);
        let reading = async move {
            let (mut columns, mut batches) = (Vec::new(), Vec::new());
            while let Some(streamed) = from.recv().await {
                match streamed {
                    Streamed::Columns(metas) => columns = metas,
                    Streamed::Rows(rows) => batches.push(rows),
                }
            }
            (columns, batches)
        };
        let (sent, (columns, batches)) = tokio::join!(stream(to), reading);
        (sent, columns, batches)
    }

    fn trips_query(schema: &str) -> (RowQuery, Vec<String>) {
        (
            RowQuery::new(ObjectRef::new(schema, "export_trips"), 0),
            vec!["id".to_owned(), "name".to_owned()],
        )
    }

    /// A table export reads the table's shape once, for the statement it
    /// shows and for its rows: the shape's one message, the start, the
    /// prepare, the run, the rollback.
    #[tokio::test]
    async fn a_table_export_reads_its_shape_once() {
        let Some(url) = test_url() else {
            return;
        };
        let schema = export_tables(&url).await;
        let (conn, asked) = counted_session(&url, Access::ReadOnly).await;
        let (query, columns) = trips_query(&schema);
        let before = asked.load(Ordering::SeqCst);
        let (shown, sent, _, batches) =
            exported_table(&conn, &query, &columns, &StopFlag::new()).await;
        let trips = asked.load(Ordering::SeqCst) - before;
        assert!(shown.unwrap().contains("ORDER BY"));
        assert_eq!(sent.unwrap(), 12_000);
        let sizes: Vec<usize> = batches.iter().map(Vec::len).collect();
        assert_eq!(sizes, [5_000, 5_000, 2_000]);
        assert_eq!(trips, 5);
        assert!(!in_transaction(&conn).await);
    }
}
```

In `crates/tabletist-db/src/mysql.rs`, add the module next to the others (line 580–583):

```rust
mod export;
mod hidden;
mod script;
mod views;
mod write;
```

In the `tests` module of `mysql.rs`, make `in_transaction` visible to the child module's tests (line 4711):

```rust
    pub(super) async fn in_transaction(conn: &Conn) -> bool {
```

- [ ] **Step 2: Run it to make sure it fails**

```bash
TABLETIST_TEST_MYSQL_URL=mysql://tabletist:tabletist@localhost:53306/tabletist CARGO_TARGET_DIR=$PWD/target ~/.cargo/bin/cargo test -p tabletist-db --lib mysql::export -- --nocapture
```

Expected: FAIL in `a_table_export_reads_its_shape_once` with `left: 6, right: 5`. Also expected: an unused warning for `read_shape`. If MySQL is not up on 53306 (check with `ss -ltn`), the test returns early and passes; say so and stop, since this plan cannot be checked without it.

- [ ] **Step 3: Read the shape once**

In `mysql.rs`, beside `known_shape` (around line 1184), add:

```rust
    /// The shape an export builds its statement from: the one this
    /// session last read of `object`, or read now in one round trip (see
    /// [`export::read_shape`]) and remembered, so that
    /// [`Adapter::export_statement`] and then [`Adapter::stream_rows`]
    /// read it once between them.
    async fn export_shape(&self, object: &ObjectRef) -> Result<Shape> {
        if let Some(shape) = self.known_shape(object) {
            return Ok(shape);
        }
        let shape = {
            let mut session = self.conn.lock().await;
            let conn = session.conn()?;
            export::read_shape(conn, object).await?
        };
        self.remember(object, shape.clone());
        Ok(shape)
    }
```

Replace `export_statement` and `stream_rows` (lines 1596–1616):

```rust
    async fn export_statement(&self, query: &RowQuery, columns: &[String]) -> Result<String> {
        let shape = self.export_shape(&query.object).await?;
        Dialect::MySql.shown_export(query, &shape.key, &shape.classes(query), columns)
    }

    /// A table's export rows, built from the shape read for
    /// `export_statement` (see [`Conn::export_shape`]), through
    /// `stream_sql`.
    async fn stream_rows(
        &self,
        query: &RowQuery,
        columns: &[String],
        batch: usize,
        to: &tokio::sync::mpsc::Sender<crate::Streamed>,
        stop: &StopFlag,
    ) -> Result<u64> {
        let shape = self.export_shape(&query.object).await?;
        let sql = Dialect::MySql.export_rows(query, &shape.key, &shape.classes(query), columns)?;
        self.stream_sql(&sql, batch, to, stop).await
    }
```

`shape.classes(query)` is empty when no filter reads bytes, as `binary_columns` was, so the statements built are unchanged.

- [ ] **Step 4: Run the tests and make sure they pass**

```bash
TABLETIST_TEST_MYSQL_URL=mysql://tabletist:tabletist@localhost:53306/tabletist CARGO_TARGET_DIR=$PWD/target ~/.cargo/bin/cargo test -p tabletist-db --lib mysql -- --nocapture 2>&1 | tail -5
TABLETIST_TEST_MYSQL_URL=mysql://tabletist:tabletist@localhost:53306/tabletist CARGO_TARGET_DIR=$PWD/target ~/.cargo/bin/cargo test -p tabletist-db --test export mysql 2>&1 | tail -5
```

Expected: all pass. `binary_columns` may now have no callers outside `count_rows`. If clippy or the compiler calls it dead code, delete it rather than allow it.

- [ ] **Step 5: Commit**

```bash
git add crates/tabletist-db/src/mysql.rs crates/tabletist-db/src/mysql/export.rs
git commit -m "Read a MySQL table's shape once per export, in one message"
```

---

### Task 2: Stream the rows in one message

Tables and SQL results without a FLOAT now take 2 round trips. Results with a FLOAT keep the binary path.

**Files:**
- Modify: `crates/tabletist-db/src/mysql/export.rs`: add `stream_message` and `text_sent_on`, plus tests.
- Modify: `crates/tabletist-db/src/mysql.rs`:
  - add `Conn::stream_in_one` next to `stream_sql` (line 882);
  - add the stop check before the run in `stream_sql`;
  - rewrite `stream_rows` and `stream_statement` (from Task 1).

- [ ] **Step 1: Write the failing tests**

In `export.rs`'s `tests` module, change the end of `a_table_export_reads_its_shape_once` to `assert_eq!(trips, 2);`. Change its doc comment to: "A table export reads the table's shape once, for the statement it shows and for its rows, and then its rows in one message: two round trips." Then add:

```rust
    /// A SQL result's export waits twice: the driver's prepare, which
    /// gives its column types, and the one message that runs it.
    #[tokio::test]
    async fn a_statement_export_waits_twice() {
        let Some(url) = test_url() else {
            return;
        };
        export_tables(&url).await;
        let (conn, asked) = counted_session(&url, Access::ReadOnly).await;
        let before = asked.load(Ordering::SeqCst);
        let (sent, columns, batches) = exported_statement(
            &conn,
            "SELECT id, name FROM export_trips ORDER BY id",
            &StopFlag::new(),
        )
        .await;
        let trips = asked.load(Ordering::SeqCst) - before;
        assert_eq!(sent.unwrap(), 12_000);
        let names: Vec<&str> = columns.iter().map(|column| column.name.as_str()).collect();
        assert_eq!(names, ["id", "name"]);
        let sizes: Vec<usize> = batches.iter().map(Vec::len).collect();
        assert_eq!(sizes, [5_000, 5_000, 2_000]);
        assert_eq!(
            batches[0][0],
            vec![Value::Int(1), Value::Text("row 1".into())]
        );
        assert_eq!(
            batches[2][1_999],
            vec![Value::Int(12_000), Value::Text("row 12000".into())]
        );
        assert_eq!(trips, 2);
        assert!(!in_transaction(&conn).await);
    }

    /// The one message and the binary protocol give the same values, row
    /// for row, for each common type and for NULL.
    #[tokio::test]
    async fn one_message_gives_the_values_the_binary_protocol_gives() {
        let Some(url) = test_url() else {
            return;
        };
        export_tables(&url).await;
        let (conn, _) = counted_session(&url, Access::ReadOnly).await;
        let sql = crate::dialect::Sql {
            text: "SELECT * FROM export_types ORDER BY id".to_owned(),
            params: Vec::new(),
        };
        let stop = StopFlag::new();
        let (sent, one_columns, one) = streamed(|to| {
            let (conn, sql, stop) = (&conn, &sql, &stop);
            async move { conn.stream_in_one(sql, 10, &to, stop).await }
        })
        .await;
        assert_eq!(sent.unwrap(), 2);
        let (sent, binary_columns, binary) = streamed(|to| {
            let (conn, sql, stop) = (&conn, &sql, &stop);
            async move { conn.stream_sql(sql, 10, &to, stop).await }
        })
        .await;
        assert_eq!(sent.unwrap(), 2);
        assert_eq!(one_columns, binary_columns);
        assert_eq!(one, binary);
    }

    /// A FLOAT keeps every digit it has: a table or a result with one is
    /// read through the binary protocol, as a page with one is. The table's
    /// shape, the start, the prepare, the run, the rollback; for a result,
    /// the prepare, then the start, the run and the rollback.
    #[tokio::test]
    async fn a_float_keeps_its_digits_in_an_export() {
        let Some(url) = test_url() else {
            return;
        };
        let schema = export_tables(&url).await;
        let expected = vec![vec![
            Value::Int(1),
            super::super::value(mysql_async::Value::Float(1.234_567_8), "float"),
        ]];
        let (conn, asked) = counted_session(&url, Access::ReadOnly).await;
        let query = RowQuery::new(ObjectRef::new(&schema, "export_floats"), 0);
        let columns = vec!["id".to_owned(), "f".to_owned()];
        let before = asked.load(Ordering::SeqCst);
        let (_, sent, _, batches) = exported_table(&conn, &query, &columns, &StopFlag::new()).await;
        assert_eq!(sent.unwrap(), 1);
        assert_eq!(batches, vec![expected.clone()]);
        assert_eq!(asked.load(Ordering::SeqCst) - before, 5);
        let before = asked.load(Ordering::SeqCst);
        let (sent, _, batches) =
            exported_statement(&conn, "SELECT id, f FROM export_floats", &StopFlag::new()).await;
        assert_eq!(sent.unwrap(), 1);
        assert_eq!(batches, vec![expected]);
        assert_eq!(asked.load(Ordering::SeqCst) - before, 4);
        assert!(!in_transaction(&conn).await);
    }

    /// A text of two statements is refused by the prepare, and nothing of
    /// it runs: no columns, no rows, no transaction.
    #[tokio::test]
    async fn an_export_of_two_statements_runs_neither() {
        let Some(url) = test_url() else {
            return;
        };
        export_tables(&url).await;
        let (conn, _) = counted_session(&url, Access::Writable).await;
        let (sent, columns, batches) = exported_statement(
            &conn,
            "SELECT 1; DELETE FROM export_trips",
            &StopFlag::new(),
        )
        .await;
        assert!(sent.is_err());
        assert!(columns.is_empty() && batches.is_empty());
        assert!(!in_transaction(&conn).await);
    }

    /// A stop that comes before the run keeps it from running, on the one
    /// message's way and on the binary protocol's.
    #[tokio::test]
    async fn a_stop_before_the_run_keeps_it_from_running() {
        let Some(url) = test_url() else {
            return;
        };
        export_tables(&url).await;
        let (conn, _) = counted_session(&url, Access::ReadOnly).await;
        for text in ["SELECT id FROM export_trips", "SELECT f FROM export_floats"] {
            let stop = StopFlag::new();
            stop.stop();
            let (sent, columns, batches) = exported_statement(&conn, text, &stop).await;
            assert!(matches!(sent, Err(crate::Error::Cancelled)), "{text}: {sent:?}");
            assert!(columns.is_empty() && batches.is_empty(), "{text}");
            assert!(!in_transaction(&conn).await, "{text}");
        }
    }

    /// The server's own time limit ends a statement the one message runs,
    /// and is told apart from other failures.
    #[tokio::test]
    async fn the_servers_timeout_ends_a_one_message_export() {
        let Some(url) = test_url() else {
            return;
        };
        export_tables(&url).await;
        let (conn, _) = counted_session(&url, Access::ReadOnly).await;
        {
            let limit = if conn.version.contains("MariaDB") {
                "SET SESSION max_statement_time = 0.3"
            } else {
                "SET SESSION max_execution_time = 300"
            };
            let mut session = conn.conn.lock().await;
            session.conn().unwrap().query_drop(limit).await.unwrap();
        }
        let (sent, _, _) = exported_statement(
            &conn,
            "SELECT COUNT(*) FROM export_trips a, export_trips b, export_trips c",
            &StopFlag::new(),
        )
        .await;
        assert!(matches!(sent, Err(crate::Error::Timeout)), "{sent:?}");
    }
```

The tests assume:
- `value` (the function at `mysql.rs:2700` that turns a driver value into a `Value`) is private to `mysql`, so `super::super::value` reaches it.
- `Conn::version` and `Conn::conn` are fields `export.rs` can read as a child module.

If any of these isn't true, adjust the paths, not the assertions.

- [ ] **Step 2: Run them to make sure they fail**

```bash
TABLETIST_TEST_MYSQL_URL=mysql://tabletist:tabletist@localhost:53306/tabletist CARGO_TARGET_DIR=$PWD/target ~/.cargo/bin/cargo test -p tabletist-db --lib mysql::export 2>&1 | tail -30
```

Expected: compile error, because there's no `stream_in_one` yet. Once Step 3's empty stub exists, the trip assertions fail: `a_table_export_reads_its_shape_once` reports 5 trips instead of 2, and `a_statement_export_waits_twice` reports 4 instead of 2. To see that state, temporarily add an `async fn stream_in_one` whose body is `self.stream_sql(sql, batch, to, stop).await`.

- [ ] **Step 3: Write the one-message stream**

In `export.rs`, extend the `use` lines and add after `read_shape`:

```rust
use mysql_async::prelude::Queryable;
use mysql_async::{QueryResult, TextProtocol};

use super::{
    BINARY_COLUMNS, PRIMARY_KEY, RUN, Shape, StreamFailed, column_metas, floats, inlined,
    prepared_run, query_error, read_start, script, text_values, typed, write,
};
use crate::{Error, ObjectRef, Result, StopFlag, Streamed};
```

```rust
/// `text` run with `values` (as [`super::literal`]s) in one message
/// through the text protocol, as [`super::page_in_one`] runs a page: the
/// start of the read-only transaction, the statement prepared from a user
/// variable, its run, the cleanup, the rollback. The run's rows are sent on
/// to `to` a batch at a time as they arrive. The server stops a message at
/// its first statement that fails, so the run happens inside the
/// transaction or not at all, and only one statement can be prepared.
///
/// An answer's header is read with the end of the answer before it, so a
/// failure up to the run (the run's own included) is
/// [`StreamFailed::Before`]: nothing more is on its way, and the
/// transaction may be open. Any failure once the run's columns are in is
/// [`StreamFailed::PartWay`].
pub(super) async fn stream_message(
    conn: &mut mysql_async::Conn,
    text: &str,
    values: &[String],
    batch: usize,
    to: &tokio::sync::mpsc::Sender<Streamed>,
    stop: &StopFlag,
) -> std::result::Result<u64, StreamFailed> {
    let mut steps = read_start(conn);
    let ran = steps.len() + RUN;
    steps.extend(prepared_run(text, values));
    steps.push(write::ROLLBACK.to_owned());
    let before = |error| StreamFailed::Before(query_error(error));
    // The first answer is in once this returns; the rest stays on the
    // connection, where a new `QueryResult` reads it (see `script::answers`).
    conn.query_iter(steps.join("; ")).await.map_err(before)?;
    for _ in 0..ran {
        QueryResult::<'_, '_, TextProtocol>::new(&mut *conn)
            .collect::<mysql_async::Row>()
            .await
            .map_err(before)?;
    }
    let mut result = QueryResult::<'_, '_, TextProtocol>::new(&mut *conn);
    let sent = text_sent_on(&mut result, batch, to, stop).await?;
    // The cleanup and the rollback are all that is left.
    stop.finish();
    QueryResult::<'_, '_, TextProtocol>::new(&mut *conn)
        .drop_result()
        .await
        .map_err(|error| StreamFailed::PartWay(query_error(error)))?;
    Ok(sent)
}

/// The run's rows as the text protocol sends them, read as the binary
/// protocol would have sent them (see [`super::binary_form`]) and sent on
/// to `to` a batch at a time. A FLOAT among the columns fails the export:
/// its text has lost digits. The table's shape, read just before, said it
/// had none, so the table changed in between.
async fn text_sent_on(
    result: &mut QueryResult<'_, '_, TextProtocol>,
    batch: usize,
    to: &tokio::sync::mpsc::Sender<Streamed>,
    stop: &StopFlag,
) -> std::result::Result<u64, StreamFailed> {
    let part_way = StreamFailed::PartWay;
    let raw = result.columns().unwrap_or_default();
    let columns = column_metas(&raw);
    if floats(&columns) {
        return Err(part_way(Error::query(
            "a column became a FLOAT as the export started; export again",
        )));
    }
    crate::stream::send(to, Streamed::Columns(columns.clone()))
        .await
        .map_err(part_way)?;
    let mut sent = 0u64;
    let mut rows = Vec::with_capacity(batch);
    while let Some(row) = result
        .next()
        .await
        .map_err(|error| part_way(query_error(error)))?
    {
        let values = text_values(row, &raw, &columns).ok_or_else(|| {
            part_way(Error::query(
                "unexpected data from the server: a value is not what its type writes",
            ))
        })?;
        rows.push(values);
        if rows.len() == batch {
            if stop.is_stopped() {
                return Err(part_way(Error::Cancelled));
            }
            sent += rows.len() as u64;
            let full = std::mem::replace(&mut rows, Vec::with_capacity(batch));
            crate::stream::send(to, Streamed::Rows(full))
                .await
                .map_err(part_way)?;
        }
    }
    if !rows.is_empty() {
        sent += rows.len() as u64;
        crate::stream::send(to, Streamed::Rows(rows))
            .await
            .map_err(part_way)?;
    }
    Ok(sent)
}
```

If `Queryable` turns out unused once this compiles, remove its import. If `write::ROLLBACK` isn't reachable as `super::write::ROLLBACK`, use the path `mysql.rs` uses.

In `mysql.rs`, next to `stream_sql` (line 882), add `stream_in_one`, and give `stream_sql` the same stop check before the run:

```rust
    /// An export's rows in one round trip (see [`export::stream_message`]),
    /// where the statement's values all have a literal; otherwise through
    /// `stream_sql`. The caller has made sure no column is a FLOAT. A stop
    /// that comes before the message keeps it from going.
    async fn stream_in_one(
        &self,
        sql: &crate::dialect::Sql,
        batch: usize,
        to: &tokio::sync::mpsc::Sender<crate::Streamed>,
        stop: &StopFlag,
    ) -> Result<u64> {
        let Some(values) = sql.params.iter().map(literal).collect::<Option<Vec<_>>>() else {
            return self.stream_sql(sql, batch, to, stop).await;
        };
        driver_parameter(&sql.text)?;
        let mut session = self.conn.lock().await;
        let conn = session.conn()?;
        if stop.is_stopped() {
            stop.finish();
            return Err(Error::Cancelled);
        }
        let streamed = export::stream_message(conn, &sql.text, &values, batch, to, stop).await;
        stop.finish();
        match streamed {
            // The message's own rollback ended the transaction.
            Ok(sent) => Ok(sent),
            Err(StreamFailed::Before(error)) => end_read(conn, Err(error))
                .await
                .map_err(crate::agent::over_its_time),
            // As in `stream_sql`: the connection is closed after this.
            Err(StreamFailed::PartWay(error)) => Err(crate::agent::over_its_time(error)),
        }
    }
```

In `stream_sql`, right after `let conn = session.conn()?;`, add:

```rust
        if stop.is_stopped() {
            stop.finish();
            return Err(Error::Cancelled);
        }
```

Replace `stream_rows`'s last line and `stream_statement`:

```rust
        let sql = Dialect::MySql.export_rows(query, &shape.key, &shape.classes(query), columns)?;
        if shape.floats {
            return self.stream_sql(&sql, batch, to, stop).await;
        }
        self.stream_in_one(&sql, batch, to, stop).await
    }

    /// A statement of the person's, as `stream_rows` streams a table's:
    /// prepared through the driver first, which gives its columns' types
    /// (and keeps it prepared for `stream_sql`), then in one message, or
    /// through the binary protocol where a column is a FLOAT or the prepare
    /// gave no columns.
    async fn stream_statement(
        &self,
        text: &str,
        batch: usize,
        to: &tokio::sync::mpsc::Sender<crate::Streamed>,
        stop: &StopFlag,
    ) -> Result<u64> {
        let sql = crate::dialect::Sql {
            text: text.to_owned(),
            params: Vec::new(),
        };
        let metas = {
            let mut session = self.conn.lock().await;
            let conn = session.conn()?;
            let statement = prepare(conn, &sql)
                .await
                .map_err(crate::agent::over_its_time)?;
            column_metas(statement.columns())
        };
        if metas.is_empty() || floats(&metas) {
            return self.stream_sql(&sql, batch, to, stop).await;
        }
        self.stream_in_one(&sql, batch, to, stop).await
    }
```

Update `stream_rows`'s doc comment: "A table's export rows, built from the shape read for `export_statement` (see [`Conn::export_shape`]): in one message (see [`Conn::stream_in_one`]), or through `stream_sql` for a table with a FLOAT."

- [ ] **Step 4: Run the tests and make sure they pass**

```bash
TABLETIST_TEST_MYSQL_URL=mysql://tabletist:tabletist@localhost:53306/tabletist CARGO_TARGET_DIR=$PWD/target ~/.cargo/bin/cargo test -p tabletist-db --lib mysql 2>&1 | tail -5
TABLETIST_TEST_MYSQL_URL=mysql://tabletist:tabletist@localhost:53306/tabletist CARGO_TARGET_DIR=$PWD/target ~/.cargo/bin/cargo test -p tabletist-db --test export --test agent --test mysql 2>&1 | tail -5
```

Expected: all pass, including the existing `a_stop_ends_the_stream_on_mysql`, `two_hundred_thousand_rows_stream_on_mysql` and `the_export_session_cannot_write_on_mysql`.

If `one_message_gives_the_values_the_binary_protocol_gives` fails on one type, `page_in_one` has the same difference for pages. Report the type and both values, and don't loosen the test. That's a finding for the controller, and it may block the plan.

If `the_servers_timeout_ends_a_one_message_export` fails because the server doesn't limit a SQL-level `EXECUTE`, report that too. Don't change the test.

- [ ] **Step 5: Lint**

```bash
CARGO_TARGET_DIR=$PWD/target ~/.cargo/bin/cargo fmt --all -- --check && CARGO_TARGET_DIR=$PWD/target ~/.cargo/bin/cargo clippy -p tabletist-db --all-targets -- -D warnings 2>&1 | tail -5
```

Expected: no output from fmt, and no warnings from clippy.

- [ ] **Step 6: Commit**

```bash
git add crates/tabletist-db/src/mysql.rs crates/tabletist-db/src/mysql/export.rs
git commit -m "Stream a MySQL export in one message once its shape is read"
```

---

### Task 3: Say so in the docs

**Files:**
- Modify: `crates/tabletist-db/src/lib.rs:536-551`
- Modify: `docs/_guide/speed-and-security.md:53-59`

- [ ] **Step 1: Update the `stream_rows` doc comment**

In `lib.rs`, replace "unbuffered on MySQL" with:

```
unbuffered on MySQL and MariaDB, in two round trips once the table's shape
is read (through the binary protocol, in more, for one with a FLOAT),
```

Keep the line wrapping the file uses, about 76 columns.

- [ ] **Step 2: Update the speed page**

In `docs/_guide/speed-and-security.md`, replace the paragraph's first sentence after "An export opens a connection of its own." with:

```
On PostgreSQL it then waits on the server three times for a table and
twice for a SQL result, and on MySQL and MariaDB twice for either (a few
times more for one with a `FLOAT` column), however many rows it writes,
```

Keep the rest of the paragraph as it is. Task 4 checks the 1.8-second figure, which is PostgreSQL's.

- [ ] **Step 3: Commit**

```bash
git add crates/tabletist-db/src/lib.rs docs/_guide/speed-and-security.md
git commit -m "Say how few round trips an export takes on MySQL and MariaDB"
```

---

### Task 4: Measure (controller, not a subagent)

The benchmark files are throwaway and never committed. They live in the session scratchpad at `/tmp/claude-1000/-home-igor-Work-tabletist--claude-worktrees-twitter-link-review-7a87a4/d25df536-15a6-47ee-8871-aae8b8fa68fd/scratchpad/bench/tools/`: `bench_mysql.rs`, `bench_mysql_setup.rs` and `tcpdelay.py`. The tables `bench_export_200k` and `bench_result_64mb` are still in the test MySQL's `tabletist` database.

- [ ] Start the sshd (`sshd/sshd_config`), the delay proxy `52200 → 52222`, and the delay proxy `53300 → 53306`, all 100 ms.
- [ ] Copy `bench_mysql.rs` to `crates/tabletist-core/tests/`, then run:
  ```bash
  BENCH_DIR=<scratchpad>/bench CARGO_TARGET_DIR=$PWD/target ~/.cargo/bin/cargo test -q --release -p tabletist-core --test bench_mysql -- --ignored --nocapture
  ```
  Note the load average before and after. Expected at 200 ms: the CSV export at about 1.4 s, down from 2.27 s, and SSH at about 3.0 s, down from 3.79 s.
- [ ] Remove the copied file, stop the servers, drop the two bench tables, and confirm `git status` is clean.

## Self-review notes

- The two-statement guard is the server's PREPARE, both from the driver and in the message. The read-only guard is `START TRANSACTION READ ONLY`, which runs first in the message, and the server stops a message at its first failure. These are the same guarantees `page_in_one` and `count_in_one` already rely on.
- MariaDB isn't running here. The CI MariaDB job (ci.yml line 187) runs these tests: the fixtures avoid `cte_max_recursion_depth`, and the timeout test picks `max_statement_time`.

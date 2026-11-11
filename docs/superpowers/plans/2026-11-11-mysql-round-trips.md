# MySQL round trips, step 1 of 5: a read's transaction in one statement — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use rubypowers:subagent-driven-development (recommended) or rubypowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ask a MySQL server fewer times for the same work, so that a person far from their server (about 200 ms a round trip) waits less. Five steps, a PR each; this plan is the whole list and step 1 in full.

**Architecture:** Every sequential await on the `mysql_async::Conn` is one round trip. The steps send what the server must answer in order as one message (MySQL's multi-statement text, which mysql_async enables), drop statements that do the same work twice, and let the driver's statement cache hit.

**Tech Stack:** Rust, mysql_async 0.37.1, the test MySQL on 53306.

**Background:** the same work for PostgreSQL's SQL editor run is igor-alexandrov/tabletist#303 (seven trips to four).

---

## How trips are counted

A test session goes through a small TCP proxy in front of the test server (`mysql::tests::counted_session`, added in step 1, used by every step after it). It counts the packets the client starts (sequence id 0): each is a command the client sends and waits on (a query, a prepare, an execute, a reset). Closing a statement, sending long data and quitting get no answer and are not counted. A count is taken on the second run of the same read, once the driver's statement cache holds what it prepares.

## The five steps

Trips are counted after the handshake, on a writable session (a read-only one skips the run's `SET SESSION TRANSACTION READ ONLY`: one less). "Today" is `main` at 3360043c, measured with `counted_session`; the later columns are the plan's arithmetic, each checked by its step's test. A SQL editor run's 13: open 3 (read-only, start, row limit), the statement 3 (check, prepare, execute; the session reset clears the driver's statement cache, so the prepare comes every run), close 7 (check, rollback, reset, and `prepare_session`'s 4).

| Step | What | Where | Today | After |
|---|---|---|---|---|
| 1 | A read's read-only transaction starts in one statement (`START TRANSACTION READ ONLY`), not the driver's two | `fetch_rows`, `count_rows`, `lookup`, `ddl`, `stream_sql` | page 5 (6 on a new page), count 4, lookup 4, DDL 6 | page 4 (5), count 3, lookup 3, DDL 5 |
| 2 | Session settings in one message, and the server's identity with its version in one `SELECT` | `prepare_session`, `Conn::connect`, `server_identity` | connect 7 (one is the driver's own), a SQL editor run 13 | connect 2 (3 on MariaDB), run 10 |
| 3 | A SQL editor run's open (with its first read-only check) and its closing check (with its `ROLLBACK`) each as one message | `mysql/script.rs` `open`, `close` | a one-`SELECT` run 10 (after step 2) | 6 |
| 4 | A page's `LIMIT` and `OFFSET` bound (`LIMIT ? OFFSET ?`), so the next page reuses the prepared statement | `Dialect::select_rows` for MySQL | a new page 5 (after step 1) | 4 |
| 5 | Describe: the CHECK query chosen by the server's version (MariaDB's form always fails to prepare on MySQL), the primary key from the index rows | `Adapter::describe` | 8 (16 the first time) | 6 (12) |

Order: step 1 helps every read, and steps 1, 4 are what browsing a table pays on every page. Step 2 comes before step 3 because the run's close ends with `prepare_session`. Each step is its own PR with its own counting test.

### Decisions for Igor

These change what steps 3 to 5 build. Step 1 needs none of them.

1. **Step 3, the two guards.** The first `standing` check before the first statement, and the `ROLLBACK` before the session reset, look deliberate. Folding them into the messages next to them keeps both at no cost in trips: the open becomes `SET SESSION TRANSACTION READ ONLY; START TRANSACTION READ ONLY; SET SESSION sql_select_limit = N; SELECT @@session.transaction_read_only LIMIT 1`, and the close's check becomes `SELECT @@session.transaction_read_only LIMIT 1; ROLLBACK AND NO CHAIN`. **Decided (Igor, 2026-11-11): keep both guards, folded in.** Dropping them would save nothing more.
2. **Step 5, describe in one message.** Choosing the CHECK form and reading the key from the index rows takes describe from 8 queries to 6. All six could go as one multi-statement text query, with the schema and table name written in as escaped literals (mysql_common's `Value::as_sql`, with backslash escapes on, as `prepare_session` guarantees): 1 trip instead of 6. That departs from the module's rule that catalog queries are prepared. **Proposed: 6 prepared queries in step 5; one message only on a yes.**
3. **A page's key lookup.** Every page still asks `information_schema` for the table's primary key (1 trip). It could be kept per table on the `Conn`, dropped by `describe` and by a page that fails. **Proposed: not in these five steps;** it saves one more trip per page, at the risk of a stale key ordering a page after another client's DDL.

---

## Step 1: a read's transaction in one statement

`mysql_async`'s `start_transaction` with `with_readonly(Some(true))` sends `SET TRANSACTION READ ONLY`, then `START TRANSACTION` (`queryable/transaction.rs:161-177` in 0.37.1): two trips, and a cancel between them leaves "the next transaction is read-only" pending (`mysql/write.rs` says the same of the save). The save and the SQL editor already start theirs as text.

What the driver's `Transaction` did that the text does not:

- **A dropped transaction was rolled back on the connection's next use.** The backend never drops a command's future (`run_session` awaits each command to its end; the MCP server's `within` too), as `mysql/script.rs` already relies on. `begin_read` says so.
- **A failed rollback was tried again on the next use.** `end_read` runs a cancelled rollback once more (`retry_cancelled!`), and a rollback that still fails closes the session (`Error::ConnectionLost`): left inside the transaction, the next catalog query would read its old snapshot.
- **A failed start.** `end_read` runs after a failed `begin_read` too: a cancel can land after the server began the transaction.

The rollback is `ROLLBACK AND NO CHAIN NO RELEASE` (`write::ROLLBACK`, now `pub(super)`): a server whose `completion_type` chains would start the next transaction straight after a plain `ROLLBACK`.

`stream_sql` keeps its rule: after a failure the export's connection is closed, so no rollback reads the rows still on their way.

**Files:**
- Modify: `crates/tabletist-db/src/mysql.rs` (the five reads, `read_page`, `stream_into`, `count`, `prepare`, `show_create`, `nth_text` take `&mut mysql_async::Conn`; `finish` and `read_only()` give way to `begin_read` and `end_read`; tests)
- Modify: `crates/tabletist-db/src/mysql/write.rs:44` (`ROLLBACK` is `pub(super)`)

### Task 1: count the trips, and end every read's transaction

- [ ] **Step 1: Write the failing tests** (the `tests` module of `mysql.rs`, see the diff below): `counted_session`, the `trips` table, `in_transaction`, and

  - `a_read_starts_its_transaction_in_one_statement`: on a read-only and on a writable session, the second page, count, lookup and DDL view ask (4, 3, 3, 5) times, and no transaction is left open.
  - `a_failed_or_cancelled_read_ends_its_transaction`: a page and a count that fail as they run (`id = (SELECT 1 UNION SELECT 2)`), and a page cancelled in `SLEEP(5)`, leave no transaction open.

- [ ] **Step 2: Run them against today's code**

  Run: `TABLETIST_TEST_MYSQL_URL=mysql://tabletist:tabletist@localhost:53306/tabletist cargo test -p tabletist-db --lib mysql::tests::a_`
  Expected: `a_read_starts_its_transaction_in_one_statement` FAILS with `left: (5, 4, 4, 6)`, `right: (4, 3, 3, 5)`. The other passes: the driver's transaction rolled back too.

- [ ] **Step 3: Start and end the reads' transactions as text** (the code part of the diff below).

- [ ] **Step 4: Run the MySQL suites and clippy**

  Run: `TABLETIST_TEST_MYSQL_URL=… cargo test --locked -p tabletist-db --lib --test mysql --test insert_audit --test lookup --test delete_rows --test agent --test ddl --test export` and `cargo clippy --locked --workspace --all-targets -- -D warnings`
  Expected: all pass, clippy clean. Both new tests are in `--lib`, which the MySQL CI job already runs.

- [ ] **Step 5: Commit**

  `git commit -S -m "Start a MySQL read's read-only transaction in one statement, not two"`

### The diff

Applies to `main` at 3360043c with `git apply`.

````diff
diff --git a/crates/tabletist-db/src/mysql.rs b/crates/tabletist-db/src/mysql.rs
index 2653bfc0..d99fc371 100644
--- a/crates/tabletist-db/src/mysql.rs
+++ b/crates/tabletist-db/src/mysql.rs
@@ -16,7 +16,7 @@ use std::time::{Duration, Instant};
 
 use mysql_async::consts::{ColumnFlags, ColumnType, StatusFlags};
 use mysql_async::prelude::Queryable;
-use mysql_async::{DriverError, IoError, Opts, OptsBuilder, Params, SslOpts, TxOpts};
+use mysql_async::{DriverError, IoError, Opts, OptsBuilder, Params, SslOpts};
 use mysql_common::named_params::ParsedNamedParams;
 
 use crate::adapter::Adapter;
@@ -86,14 +86,15 @@ impl Conn {
         stop: &StopFlag,
     ) -> Result<u64> {
         let mut conn = self.conn.lock().await;
-        let mut transaction = conn
-            .start_transaction(read_only())
-            .await
-            .map_err(query_error)?;
-        match stream_into(&mut transaction, sql, batch, to, stop).await {
+        let streamed = async {
+            begin_read(&mut conn).await?;
+            stream_into(&mut conn, sql, batch, to, stop).await
+        }
+        .await;
+        match streamed {
             Ok(sent) => {
                 stop.finish();
-                finish(transaction, Ok(sent)).await
+                end_read(&mut conn, Ok(sent)).await
             }
             // Rows the stream did not read are still on their way: a
             // rollback now would read every one of them first. The
@@ -519,12 +520,12 @@ impl Adapter for Conn {
     /// table's triggers by name, in one read-only transaction.
     async fn ddl(&self, object: &ObjectRef) -> Result<Ddl> {
         let mut conn = self.conn.lock().await;
-        let mut transaction = conn
-            .start_transaction(read_only())
-            .await
-            .map_err(query_error)?;
-        let outcome = show_create(&mut transaction, object).await;
-        let parts = finish(transaction, outcome).await?;
+        let outcome = async {
+            begin_read(&mut conn).await?;
+            show_create(&mut conn, object).await
+        }
+        .await;
+        let parts = end_read(&mut conn, outcome).await?;
         let server = version_name(&self.version);
         Ok(Ddl::new(crate::Driver::MySql, server, None, parts))
     }
@@ -538,12 +539,12 @@ impl Adapter for Conn {
         let limit = query.limit as usize;
         let mut conn = self.conn.lock().await;
         let started = Instant::now();
-        let mut transaction = conn
-            .start_transaction(read_only())
-            .await
-            .map_err(query_error)?;
-        let outcome = read_page(&mut transaction, &sql, limit).await;
-        let (columns, mut rows) = finish(transaction, outcome).await?;
+        let outcome = async {
+            begin_read(&mut conn).await?;
+            read_page(&mut conn, &sql, limit).await
+        }
+        .await;
+        let (columns, mut rows) = end_read(&mut conn, outcome).await?;
         let has_more = rows.len() > limit;
         rows.truncate(limit);
         Ok(RowPage {
@@ -595,12 +596,12 @@ impl Adapter for Conn {
         let binary = self.binary_columns(query).await?;
         let sql = Dialect::MySql.count_rows(query, &binary)?;
         let mut conn = self.conn.lock().await;
-        let mut transaction = conn
-            .start_transaction(read_only())
-            .await
-            .map_err(query_error)?;
-        let outcome = count(&mut transaction, &sql).await;
-        let row = finish(transaction, outcome).await?;
+        let outcome = async {
+            begin_read(&mut conn).await?;
+            count(&mut conn, &sql).await
+        }
+        .await;
+        let row = end_read(&mut conn, outcome).await?;
         row.map(from_row::<u64>)
             .transpose()?
             .ok_or_else(|| Error::query("the count returned no number"))
@@ -609,13 +610,13 @@ impl Adapter for Conn {
     async fn lookup(&self, lookup: &Lookup) -> Result<LookupPage> {
         let sql = Dialect::MySql.lookup(lookup);
         let mut conn = self.conn.lock().await;
-        let mut transaction = conn
-            .start_transaction(read_only())
-            .await
-            .map_err(query_error)?;
         // `read_page` stops one row past the limit.
-        let outcome = read_page(&mut transaction, &sql, lookup.limit as usize).await;
-        let (_, rows) = finish(transaction, outcome).await?;
+        let outcome = async {
+            begin_read(&mut conn).await?;
+            read_page(&mut conn, &sql, lookup.limit as usize).await
+        }
+        .await;
+        let (_, rows) = end_read(&mut conn, outcome).await?;
         crate::lookup::page(rows, lookup)
     }
 
@@ -684,12 +685,12 @@ fn row_values(row: mysql_async::Row, columns: &[ColumnMeta]) -> Vec<Value> {
 
 /// Up to `limit + 1` rows of a page, so the caller can tell there are more.
 async fn read_page(
-    transaction: &mut mysql_async::Transaction<'_>,
+    conn: &mut mysql_async::Conn,
     sql: &crate::dialect::Sql,
     limit: usize,
 ) -> Result<(Vec<ColumnMeta>, Vec<Vec<Value>>)> {
-    let statement = prepare(transaction, sql).await?;
-    let mut result = transaction
+    let statement = prepare(conn, sql).await?;
+    let mut result = conn
         .exec_iter(&statement, params(&sql.params))
         .await
         .map_err(query_error)?;
@@ -707,14 +708,14 @@ async fn read_page(
 
 /// An export's rows, a batch at a time (see [`Conn::stream_rows`]).
 async fn stream_into(
-    transaction: &mut mysql_async::Transaction<'_>,
+    conn: &mut mysql_async::Conn,
     sql: &crate::dialect::Sql,
     batch: usize,
     to: &tokio::sync::mpsc::Sender<crate::Streamed>,
     stop: &StopFlag,
 ) -> Result<u64> {
-    let statement = prepare(transaction, sql).await?;
-    let mut result = transaction
+    let statement = prepare(conn, sql).await?;
+    let mut result = conn
         .exec_iter(&statement, params(&sql.params))
         .await
         .map_err(query_error)?;
@@ -742,12 +743,11 @@ async fn stream_into(
 
 /// The row of a count.
 async fn count(
-    transaction: &mut mysql_async::Transaction<'_>,
+    conn: &mut mysql_async::Conn,
     sql: &crate::dialect::Sql,
 ) -> Result<Option<mysql_async::Row>> {
-    let statement = prepare(transaction, sql).await?;
-    transaction
-        .exec_first(&statement, params(&sql.params))
+    let statement = prepare(conn, sql).await?;
+    conn.exec_first(&statement, params(&sql.params))
         .await
         .map_err(query_error)
 }
@@ -757,14 +757,11 @@ async fn count(
 /// statement runs without a value for each of its parameters. The filters
 /// bind theirs, so such a parameter comes from the raw WHERE.
 async fn prepare(
-    transaction: &mut mysql_async::Transaction<'_>,
+    conn: &mut mysql_async::Conn,
     sql: &crate::dialect::Sql,
 ) -> Result<mysql_async::Statement> {
     driver_parameter(&sql.text)?;
-    let statement = transaction
-        .prep(sql.text.as_str())
-        .await
-        .map_err(query_error)?;
+    let statement = conn.prep(sql.text.as_str()).await.map_err(query_error)?;
     if usize::from(statement.num_params()) != sql.params.len() {
         return Err(parameter("?", None));
     }
@@ -828,13 +825,13 @@ fn parameter(spelled: &str, hint: Option<&str>) -> Error {
 }
 
 /// The parts of `object`'s definition, read by [`Adapter::ddl`] in
-/// `transaction`.
+/// its read-only transaction.
 async fn show_create(
-    transaction: &mut mysql_async::Transaction<'_>,
+    conn: &mut mysql_async::Conn,
     object: &ObjectRef,
 ) -> Result<Vec<crate::ddl::Part>> {
     use crate::ddl::{Part, PartKind};
-    let kind: Option<String> = transaction
+    let kind: Option<String> = conn
         .exec_first(
             "SELECT table_type FROM information_schema.tables \
              WHERE table_schema = ? AND table_name = ?",
@@ -859,8 +856,8 @@ async fn show_create(
         format!("SHOW CREATE TABLE {name}")
     };
     // Its second column, whichever it is: `Create Table` or `Create View`.
-    let own = nth_text(transaction, &shown, 1).await?;
-    let names: Vec<String> = transaction
+    let own = nth_text(conn, &shown, 1).await?;
+    let names: Vec<String> = conn
         .exec(
             "SELECT trigger_name FROM information_schema.triggers \
              WHERE event_object_schema = ? AND event_object_table = ? \
@@ -877,7 +874,7 @@ async fn show_create(
             Dialect::MySql.quote_ident(&trigger)
         );
         // `SQL Original Statement`, after the name and the `sql_mode`.
-        triggers.push(nth_text(transaction, &shown, 2).await?);
+        triggers.push(nth_text(conn, &shown, 2).await?);
     }
     Ok(vec![
         Part {
@@ -892,27 +889,44 @@ async fn show_create(
 }
 
 /// Column `index` of the one row `sql` answers, as text.
-async fn nth_text(
-    transaction: &mut mysql_async::Transaction<'_>,
-    sql: &str,
-    index: usize,
-) -> Result<String> {
-    let row: Option<mysql_async::Row> = transaction.query_first(sql).await.map_err(query_error)?;
+async fn nth_text(conn: &mut mysql_async::Conn, sql: &str, index: usize) -> Result<String> {
+    let row: Option<mysql_async::Row> = conn.query_first(sql).await.map_err(query_error)?;
     row.and_then(|row| row.get_opt::<String, _>(index))
         .transpose()
         .map_err(|_| Error::query("unexpected data from the server: the text is not text"))?
         .ok_or_else(|| Error::query(format!("{sql} returned nothing")))
 }
 
-/// Ends a read transaction whatever happened inside it. mysql_async only
-/// rolls a dropped transaction back on the connection's next use, so a
-/// failed or cancelled read would otherwise keep its locks until then.
-/// The read's own error wins over one from the rollback.
-async fn finish<T>(transaction: mysql_async::Transaction<'_>, outcome: Result<T>) -> Result<T> {
-    let rolled_back = transaction.rollback().await;
-    let value = outcome?;
-    rolled_back.map_err(query_error)?;
-    Ok(value)
+/// Starts a read's transaction: read-only, in one statement. The
+/// driver's own read-only transaction takes two (`SET TRANSACTION READ
+/// ONLY`, then `START TRANSACTION`), each a round trip, and a cancel
+/// between them leaves "the next transaction is read-only" pending (see
+/// `write.rs`). What follows must end with [`end_read`], also when this
+/// fails: a cancel can land after the server began the transaction.
+///
+/// The read's future is awaited to its end, as every command's is: the
+/// backend runs a session's commands one at a time and drops none. One
+/// dropped half-way would leave the session inside this transaction,
+/// which the driver's own would have rolled back on the next query.
+async fn begin_read(conn: &mut mysql_async::Conn) -> Result<()> {
+    execute(conn, "START TRANSACTION READ ONLY").await
+}
+
+/// Ends a read's transaction whatever happened inside it, so that a failed
+/// or cancelled read keeps no locks and the next catalog query does not
+/// read the transaction's old snapshot. A cancel meant for the read can
+/// land on the rollback: it runs once more. The read's own error wins,
+/// unless the rollback failed: a session left inside the transaction is
+/// closed (a lost connection).
+async fn end_read<T>(conn: &mut mysql_async::Conn, outcome: Result<T>) -> Result<T> {
+    let rolled_back = retry_cancelled!(execute(conn, write::ROLLBACK));
+    match (outcome, rolled_back) {
+        (outcome, Ok(())) => outcome,
+        (Err(lost), Err(_)) if lost.is_connection_lost() => Err(lost),
+        (_, Err(error)) => Err(Error::ConnectionLost(format!(
+            "could not end the read's transaction: {error}"
+        ))),
+    }
 }
 
 /// Makes the session read-only: no transaction of its own can write.
@@ -1347,12 +1361,6 @@ fn same_server(session: &str, reached: &str) -> Result<()> {
     }
 }
 
-fn read_only() -> TxOpts {
-    let mut options = TxOpts::default();
-    options.with_readonly(Some(true));
-    options
-}
-
 fn params(values: &[Value]) -> Params {
     if values.is_empty() {
         Params::Empty
@@ -1843,4 +1851,188 @@ mod tests {
             None
         );
     }
+
+    /// A session through a proxy in front of the test server, with a count
+    /// of the commands the client sends and waits on: queries, prepares,
+    /// executes, resets. Far from the server each is a round trip. A
+    /// command is a packet the client starts (sequence 0); closing a
+    /// statement, sending its long data and quitting get no answer, and are
+    /// not counted.
+    pub(super) async fn counted_session(
+        url: &str,
+        access: Access,
+    ) -> (Conn, std::sync::Arc<std::sync::atomic::AtomicUsize>) {
+        use std::sync::Arc;
+        use std::sync::atomic::{AtomicUsize, Ordering};
+        use tokio::io::{AsyncReadExt, AsyncWriteExt};
+
+        const QUIT: u8 = 0x01;
+        const SEND_LONG_DATA: u8 = 0x18;
+        const STATEMENT_CLOSE: u8 = 0x19;
+        let (mut spec, secrets) = ConnectSpec::from_url(url).unwrap();
+        spec.tls = TlsMode::Disable;
+        let server = (spec.host.clone(), spec.port);
+        let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
+            .await
+            .unwrap();
+        spec.host = "127.0.0.1".to_owned();
+        spec.port = listener.local_addr().unwrap().port();
+        let asked = Arc::new(AtomicUsize::new(0));
+        let count = Arc::clone(&asked);
+        tokio::spawn(async move {
+            let (client, _) = listener.accept().await.unwrap();
+            let upstream = tokio::net::TcpStream::connect(server).await.unwrap();
+            let (mut from_client, mut to_client) = client.into_split();
+            let (mut from_server, mut to_server) = upstream.into_split();
+            tokio::spawn(async move {
+                tokio::io::copy(&mut from_server, &mut to_client).await.ok();
+            });
+            loop {
+                let mut header = [0; 4];
+                if from_client.read_exact(&mut header).await.is_err() {
+                    return;
+                }
+                let length = u32::from_le_bytes([header[0], header[1], header[2], 0]);
+                let mut body = vec![0; length as usize];
+                if from_client.read_exact(&mut body).await.is_err() {
+                    return;
+                }
+                // The handshake's answer is the client's packet 1: every
+                // packet 0 the client sends is a command.
+                let answered = !matches!(
+                    body.first(),
+                    Some(&(QUIT | SEND_LONG_DATA | STATEMENT_CLOSE))
+                );
+                if header[3] == 0 && answered {
+                    count.fetch_add(1, Ordering::SeqCst);
+                }
+                if to_server.write_all(&header).await.is_err()
+                    || to_server.write_all(&body).await.is_err()
+                {
+                    return;
+                }
+            }
+        });
+        let conn = Conn::connect(&spec, &secrets, None, access).await.unwrap();
+        (conn, asked)
+    }
+
+    /// One test at a time creates the `trips` table: creating it twice at
+    /// once can fail.
+    static TRIPS: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());
+
+    /// The `trips` table, with a key and three rows.
+    async fn trips(url: &str) -> ObjectRef {
+        let _turn = TRIPS.lock().await;
+        let opts = Opts::from_url(&format!("{url}?prefer_socket=false")).unwrap();
+        let mut admin = mysql_async::Conn::new(opts).await.unwrap();
+        admin
+            .query_drop(
+                "CREATE TABLE IF NOT EXISTS trips (id int PRIMARY KEY, name varchar(20)); \
+                 INSERT IGNORE INTO trips VALUES (1, 'a'), (2, 'b'), (3, 'c')",
+            )
+            .await
+            .unwrap();
+        let schema: String = admin
+            .query_first("SELECT DATABASE()")
+            .await
+            .unwrap()
+            .unwrap();
+        ObjectRef {
+            schema,
+            name: "trips".into(),
+        }
+    }
+
+    /// Whether the session is inside a transaction, by the server's status
+    /// after a query of its own.
+    async fn in_transaction(conn: &Conn) -> bool {
+        let mut conn = conn.conn.lock().await;
+        conn.query_drop("DO 1").await.unwrap();
+        status(&conn).contains(StatusFlags::SERVER_STATUS_IN_TRANS)
+    }
+
+    /// Each read starts its read-only transaction in one statement and ends
+    /// it with one: a page is the key, the start, the statement and the
+    /// end, once the driver has its statements prepared.
+    #[tokio::test]
+    async fn a_read_starts_its_transaction_in_one_statement() {
+        use std::sync::atomic::Ordering;
+        let Some(url) = test_url() else {
+            return;
+        };
+        let object = trips(&url).await;
+        let page = RowQuery::new(object.clone(), 10);
+        let lookup = Lookup {
+            object: object.clone(),
+            key: "id".into(),
+            label: Some("name".into()),
+            part: crate::LookupPart::Matching(String::new()),
+            limit: 10,
+        };
+        for access in [Access::ReadOnly, Access::Writable] {
+            let (conn, asked) = counted_session(&url, access).await;
+            // Once to prepare what the driver keeps prepared, then counted.
+            for counted in [false, true] {
+                let before = asked.load(Ordering::SeqCst);
+                let rows = conn.fetch_rows(&page).await.unwrap();
+                assert_eq!(rows.rows.len(), 3);
+                let fetched = asked.load(Ordering::SeqCst) - before;
+                let before = asked.load(Ordering::SeqCst);
+                assert_eq!(conn.count_rows(&page).await.unwrap(), 3);
+                let counted_rows = asked.load(Ordering::SeqCst) - before;
+                let before = asked.load(Ordering::SeqCst);
+                assert_eq!(conn.lookup(&lookup).await.unwrap().rows.len(), 3);
+                let looked_up = asked.load(Ordering::SeqCst) - before;
+                let before = asked.load(Ordering::SeqCst);
+                conn.ddl(&object).await.unwrap();
+                let shown = asked.load(Ordering::SeqCst) - before;
+                if counted {
+                    // The DDL: its kind, `SHOW CREATE TABLE`, its triggers.
+                    assert_eq!(
+                        (fetched, counted_rows, looked_up, shown),
+                        (4, 3, 3, 5),
+                        "{access:?}"
+                    );
+                }
+            }
+            assert!(!in_transaction(&conn).await, "{access:?}");
+        }
+    }
+
+    /// A read that fails, or is cancelled, still ends its transaction.
+    #[tokio::test]
+    async fn a_failed_or_cancelled_read_ends_its_transaction() {
+        let Some(url) = test_url() else {
+            return;
+        };
+        let object = trips(&url).await;
+        let conn = session(&url).await;
+        // Fails as it runs, not as it is prepared.
+        let mut failing = RowQuery::new(object.clone(), 10);
+        failing.raw_where = Some("id = (SELECT 1 UNION SELECT 2)".into());
+        assert!(matches!(
+            conn.fetch_rows(&failing).await,
+            Err(Error::Query { .. })
+        ));
+        assert!(!in_transaction(&conn).await);
+        assert!(matches!(
+            conn.count_rows(&failing).await,
+            Err(Error::Query { .. })
+        ));
+        assert!(!in_transaction(&conn).await);
+        let mut slow = RowQuery::new(object, 10);
+        slow.raw_where = Some("SLEEP(5) = 0".into());
+        let cancel = conn.cancel_handle();
+        let cancelling = async {
+            tokio::time::sleep(Duration::from_millis(300)).await;
+            cancel.cancel().await
+        };
+        let started = Instant::now();
+        let (fetched, cancelled) = tokio::join!(conn.fetch_rows(&slow), cancelling);
+        cancelled.unwrap();
+        assert_eq!(fetched.unwrap_err(), Error::Cancelled);
+        assert!(started.elapsed() < Duration::from_secs(5));
+        assert!(!in_transaction(&conn).await);
+    }
 }
diff --git a/crates/tabletist-db/src/mysql/write.rs b/crates/tabletist-db/src/mysql/write.rs
index 0075c4b1..92d4892e 100644
--- a/crates/tabletist-db/src/mysql/write.rs
+++ b/crates/tabletist-db/src/mysql/write.rs
@@ -41,7 +41,7 @@ struct Deletion {
 /// `completion_type` says a `COMMIT` or a `ROLLBACK` does next: chained,
 /// the session would be left inside a read-write transaction.
 const COMMIT: &str = "COMMIT AND NO CHAIN NO RELEASE";
-const ROLLBACK: &str = "ROLLBACK AND NO CHAIN NO RELEASE";
+pub(super) const ROLLBACK: &str = "ROLLBACK AND NO CHAIN NO RELEASE";
 
 impl Conn {
     /// See [`crate::Connection::write`]. The transaction is started and
````

---

## Step 2: the session's settings, and who the server is, in one message

Built on step 1's branch (PR stacked on igor-alexandrov/tabletist#305).

- `session_settings(access)` is the four `SET`s as one text of statements, in the same order (read-only first, read-write last). `prepare_session` sends it as one message, run once more after a cancel. The server runs a message's statements in order and stops at the first that fails, so no `SET` runs after one that failed.
- `server_identity(conn, before)` sends `before` and then `SELECT @@server_uuid, VERSION()` as one message. At connect, `before` is the settings. On MariaDB (no `server_uuid`, error 1193) it asks `SELECT @@hostname, @@port, VERSION()` with `before` again, so that a failing `SET` is never taken for the missing UUID. The cancel connection calls it with nothing before.
- `last_row` reads a message's answer: the last row of the last result set that has one. **mysql_async keeps the error of a statement after the first as a pending result, and `is_empty()` already says `true`: only `drop_result` gives the error.** `last_row` ends with it. (`query_drop`, which `prepare_session` uses, already does.) Step 3 reads its messages through `last_row`.

Measured: connect 7 → 2 trips; a one-`SELECT` SQL editor run 13 → 10 on a writable session, 12 → 9 on a read-only one.

Tests (`mysql::tests`, in `--lib`):

- `a_session_is_prepared_and_named_in_one_message`: connect asks twice (the driver's own question, then ours); the server is named by UUID and the version is MySQL's; the settings hold after connect and again after a run, which asks 9 or 10 times. On step 1's code: `left: 7`, `right: 2`.
- `a_message_of_statements_stops_at_the_first_that_fails`: `last_row` gives the last row, and the error of a statement in the middle or at the end (1305). Statements after the failing one did not run.

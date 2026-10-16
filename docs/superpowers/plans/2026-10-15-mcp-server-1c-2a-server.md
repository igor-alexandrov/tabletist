# MCP server, Phase 1c, run 2a: the server's side of hidden tables. Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** The server knows what each connection hides and acts on it. `list_connections` names what is hidden, `describe_schema` leaves hidden tables, the views over them and the keys to them out, and `run_query` reads nothing hidden. A hidden table that was renamed or dropped closes the connection to agents until the person looks. Nothing is visible yet: the app's gateway passes "All" until the next run reads the setting from `connections.toml`.

**Architecture:** `tabletist-db` gains `Hidden::check_settings`: an "All except" entry that names no relation of the catalog stops every call (`Refusal::SettingsNeedALook`, the server's `connection_unavailable`). It also gains `Hidden::hides`, `Display` for `TableName`, and `Connection::agent_hidden`, which reads a driver's catalog outside a statement (a new `Adapter::agents_catalog`). In `tabletist-mcp`, `Listed` carries each connection's `Tables`; `Session::hidden` gives what they come to on the database, and `Session::read` takes them. `describe_schema` filters with the `Hidden`, and `list_connections` writes the names.

**Tech Stack:** Rust 2024, `tabletist-db`'s drivers, `rmcp` through `tabletist-mcp`.

---

## Where this sits

Phase 1c of `docs/superpowers/plans/2026-10-13-mcp-server.md`, hidden tables. 1c-1 (pull request 144) built the checks. Its second run was cut in two on 2026-10-15, the server first: this run, 1c-2a. 1c-2b puts the setting in `connections.toml`, passes it from the app, holds a change made outside the app and documents the keys. 1c-3 is the editor's Tables control and the website's guide.

## Before you start

- Work on a branch of `main`. One commit for each task, in the order below. Every commit passes `cargo fmt --all --check`, `cargo clippy --locked --workspace --all-targets -- -D warnings` and `cargo test --locked --workspace --all-targets`.
- On Igor's machine `cargo` is `~/.cargo/bin/cargo` (the mise shim fails), and a build in a worktree needs `CARGO_TARGET_DIR` set to that worktree's `target/`.
- Read `AGENTS.md`: no `unsafe`, and `tests/engines.rs` refuses a `==` between two `Dialect`s in the library's code: name every engine in a `match`.
- **Task 1's tests need the test servers**, PostgreSQL 17 and MySQL 8.4 from `compose.yaml`, on ports 55432 and 53306: `TABLETIST_TEST_PG_URL=postgres://tabletist:tabletist@localhost:55432/tabletist` and `TABLETIST_TEST_MYSQL_URL=mysql://tabletist:tabletist@localhost:53306/tabletist`. Without them a server's test prints "skipped" and passes, which proves nothing.
- The diffs below apply with `git apply`. Run `cargo fmt --all` before each commit: the code here is already formatted.

## What was run before this plan was written

Every task's state was built here, passed `cargo fmt --check` and clippy with `-D warnings`, and its tests failed and then passed as each step says, against both servers. The last state passed `cargo test --locked --workspace --all-targets` with both servers (3,520 tests, none failed) and `RUSTDOCFLAGS='-D warnings' cargo doc --locked --workspace --no-deps`. Not run: CI, and Windows and macOS (nothing here is per system).

## What this run settles

| Question | Decision | Why |
|---|---|---|
| 1c-2's cut | Two runs, the server first: this one, then the setting in `connections.toml` | Agreed on 2026-10-15. With the setting first, `describe_schema` would describe a hidden table's columns until the second run. This way, when the setting appears, all of it is enforced |
| A hidden table renamed or dropped | On all three, an "All except" entry that names no table closes the connection to agents until the person removes it or points it at a table | Agreed on 2026-10-15, against C.3's OID on PostgreSQL: the checks compare names, an OID changes with every dump and restore, and a drop looks like a rename |
| What the stop answers | `connection_unavailable`, with a sentence that names the entry | C.3 and B.8. The entry is the person's own words, and the names are told (question 8) |
| An "Only" entry that names nothing | Stops nothing | It hides more, not less |
| What `list_connections` says | `hidden` or `only`, each entry as the person wrote it | Question 8: the names are told. Under "Only" the list is what may be read, which is shorter than what may not |
| What `describe_schema` leaves out | Every hidden relation from its list; a hidden one asked for is named under `hidden`; a key to or from a hidden table | A key names the other table's columns. A name asked for is answered, not left to look missing |
| What a view reads, for the listing | Asked of the engine. SQLite: a read of the view is prepared and the authorizer's reports are kept; one that cannot be prepared is hidden while anything is. MySQL: a view of which `VIEW_TABLE_USAGE` lists no table is hidden while anything is | Found by the review: a reading of a SQLite view's text missed a CTE named like the table, a quoted name and `IN table`, and MySQL tells a user without `SHOW VIEW` nothing of any view. Each such view was described with a hidden table's columns |
| A hidden table the user cannot see, on MySQL | A stale entry: the connection is closed to agents, and the sentence says its user can see no table of that name | `information_schema` lists only what the user may use, so a renamed table and one without rights look alike. Closing is the side that leaks nothing |

### What this run leaves out

- The setting itself: `hidden` and `only` in a connection's `[connections.agents]` table, version 5 of the file, the app passing them to its gateway, a change made outside the app held, and the reference page. That is 1c-2b. Until it, `src/backend/agents.rs` lists every connection with `Tables::All`.
- The editor's Tables control, its hint, and an entry marked "not in this database": 1c-3.

## File structure

| File | What changes |
|---|---|
| `crates/tabletist-db/src/agent/hidden.rs` | `Hidden::check_settings` and `hides`; an "All except" entry that names nothing; `Display` for `TableName` |
| `crates/tabletist-db/src/agent.rs` | `Refusal::SettingsNeedALook`, its code and its sentence |
| `crates/tabletist-db/src/adapter.rs`, `lib.rs` | `Adapter::agents_catalog`; `Connection::agent_hidden` |
| `crates/tabletist-db/src/pg.rs`, `pg/hidden.rs`, `mysql.rs`, `mysql/hidden.rs` | Each driver's catalog, for `agents_catalog`; on MySQL a view of which nothing is listed is opaque |
| `crates/tabletist-db/src/sqlite.rs`, `sqlite/fence.rs` | `agents_catalog`; a view depends on what the authorizer reports as a read of it is prepared (`Fences::reads_of`) |
| `crates/tabletist-db/tests/agent.rs` | `agent_hidden` and the stale entry, on the three engines |
| `crates/tabletist-mcp/src/gateway.rs` | `Listed::tables`; `Session::hidden`; `Session::read` takes the tables |
| `crates/tabletist-mcp/src/tools/connections.rs`, `schema.rs`, `query.rs` | `hidden` and `only` in `list_connections`; `describe_schema` without what is hidden; `run_query` with the tables |
| `crates/tabletist-mcp/tests/support/mod.rs`, `tests/tools.rs` | `Shop::hide`; the tools on a connection that hides `customers` |
| `src/backend/agents.rs` | The app's gateway: `Tables::All`, `hidden`, `read` with the tables |
| `docs/superpowers/plans/2026-10-13-mcp-server.md` | 1c-2 in two; C.3's renames; B.8; what this run built |

---

### Task 1: What is hidden, asked of the database, and an entry that names nothing

- `Hidden::check_settings`: an "All except" entry that names no relation of the catalog (compared as every name is, without regard to case, in its schema where it gives one) is `Refusal::SettingsNeedALook`, whose code is `connection_unavailable`. `check_statement` asks it first, so on all three engines every read of such a connection is refused before anything is sent. An entry that names a system catalog is never stale; an "Only" entry that names nothing hides more and stops nothing.
- `Hidden::hides(schema, name)`: what `check_engine_read` would refuse, for the server's listing.
- `TableName` is written as the person writes it: `customers`, `sales.customers`, a part that holds a dot or a double quote in double quotes.
- `Connection::agent_hidden(&Tables)`: what the tables come to on the database now, through a new `Adapter::agents_catalog`. Each driver gives the catalog its check reads (PostgreSQL's and MySQL's `catalog`, now `pub(super)`). Where nothing is hidden, nothing is asked.
- What a view reads is the engine's word for the listing too. On SQLite the catalog prepares `SELECT * FROM` each view while the fences record what the authorizer reports (`Fences::reads_of`), through the views it reads; a view that cannot be prepared is opaque. On MySQL a view of which `VIEW_TABLE_USAGE` lists no table is opaque, as a user without `SHOW VIEW` is told nothing of any view.
- The stale entry's sentence says the user can see no table of that name: on MySQL, `information_schema` lists only what the user may use.

**Files:**
- Modify: `crates/tabletist-db/src/agent.rs`, `crates/tabletist-db/src/agent/hidden.rs`, `crates/tabletist-db/src/adapter.rs`, `crates/tabletist-db/src/lib.rs`, `crates/tabletist-db/src/pg.rs`, `crates/tabletist-db/src/pg/hidden.rs`, `crates/tabletist-db/src/mysql.rs`, `crates/tabletist-db/src/mysql/hidden.rs`, `crates/tabletist-db/src/sqlite.rs`, `crates/tabletist-db/src/sqlite/fence.rs`, `crates/tabletist-db/tests/agent.rs`

- [ ] **Step 1: Write the failing tests**

The refusal is declared with them, as the tests name it.

````diff
diff --git a/crates/tabletist-db/src/agent.rs b/crates/tabletist-db/src/agent.rs
index 9cd1217..390f8da 100644
--- a/crates/tabletist-db/src/agent.rs
+++ b/crates/tabletist-db/src/agent.rs
@@ -146,6 +146,12 @@ pub enum Refusal {
     /// The system catalogs, on a connection that hides a table: they
     /// describe every one.
     SystemCatalog,
+    /// The connection's "All except" list names tables its user cannot
+    /// see, as it writes them: a hidden table renamed, or dropped, which
+    /// look the same, or on MySQL one the user has no right to. Agents may not use the connection until the person looks
+    /// (the MCP plan's C.3). Its code is the server's for a connection it
+    /// cannot open.
+    SettingsNeedALook(Vec<String>),
 }
 
 /// What was too large.
@@ -196,6 +202,7 @@ impl Refusal {
             Self::InvalidParams(_) => "invalid_params",
             Self::TableHidden { .. } => "table_hidden",
             Self::SystemCatalog => "system_catalog",
+            Self::SettingsNeedALook(_) => "connection_unavailable",
         }
     }
 }
@@ -273,6 +280,12 @@ impl fmt::Display for Refusal {
                 f,
                 "{through} reads {name}, which is hidden for agents on this connection."
             ),
+            Self::SettingsNeedALook(missing) => write!(
+                f,
+                "Agents may not use this connection until its agent settings are looked at in \
+                 Tabletist: they hide {}, and its user can see no table of that name.",
+                missing.join(", ")
+            ),
             Self::SystemCatalog => f.write_str(
                 "The system catalogs are not available on this connection. Use describe_schema.",
             ),
diff --git a/crates/tabletist-db/src/agent/hidden.rs b/crates/tabletist-db/src/agent/hidden.rs
index 598bae5..37cdcc1 100644
--- a/crates/tabletist-db/src/agent/hidden.rs
+++ b/crates/tabletist-db/src/agent/hidden.rs
@@ -1151,7 +1151,10 @@ mod tests {
         // SQLite's own tables, in any schema; MySQL's system schemas.
         let sqlite = Hidden::new(
             Dialect::Sqlite,
-            &Catalog::default(),
+            &Catalog {
+                relations: vec![relation("main", "customers", &[])],
+                ..Catalog::default()
+            },
             &Tables::AllExcept(vec![table(None, "customers")]),
         );
         let read = checked(Dialect::Sqlite, "SELECT * FROM temp.sqlite_master");
@@ -1327,6 +1330,53 @@ mod tests {
         );
     }
 
+    #[test]
+    fn an_entry_that_names_no_table_closes_the_connection_until_it_is_looked_at() {
+        // Renamed, or dropped: the "All except" list names what is gone.
+        let renamed = hidden(Tables::AllExcept(vec![
+            table(None, "customers"),
+            table(None, "clients"),
+            table(Some("sales"), "orders"),
+        ]));
+        assert_eq!(
+            renamed.check_settings(),
+            Err(Refusal::SettingsNeedALook(vec![
+                "clients".into(),
+                "sales.orders".into()
+            ]))
+        );
+        assert_eq!(
+            statement(&renamed, "SELECT * FROM books"),
+            Err(Refusal::SettingsNeedALook(vec![
+                "clients".into(),
+                "sales.orders".into()
+            ]))
+        );
+        // A name in another case is the table's; a system catalog's is
+        // hidden with anything.
+        let fine = hidden(Tables::AllExcept(vec![
+            table(None, "CUSTOMERS"),
+            table(Some("pg_catalog"), "pg_class"),
+        ]));
+        assert_eq!(fine.check_settings(), Ok(()));
+        // An "Only" list that names what is gone hides more, not less.
+        let only = hidden(Tables::Only(vec![table(None, "clients")]));
+        assert_eq!(only.check_settings(), Ok(()));
+    }
+
+    #[test]
+    fn an_entry_is_written_as_the_person_writes_it() {
+        assert_eq!(table(None, "customers").to_string(), "customers");
+        assert_eq!(
+            table(Some("sales"), "customers").to_string(),
+            "sales.customers"
+        );
+        assert_eq!(
+            table(Some("odd.schema"), "say \"hi\"").to_string(),
+            "\"odd.schema\".\"say \"\"hi\"\"\""
+        );
+    }
+
     #[test]
     fn nothing_is_hidden_by_an_empty_list_or_by_all() {
         for tables in [Tables::All, Tables::AllExcept(Vec::new())] {
diff --git a/crates/tabletist-db/tests/agent.rs b/crates/tabletist-db/tests/agent.rs
index 5151b71..77262fb 100644
--- a/crates/tabletist-db/tests/agent.rs
+++ b/crates/tabletist-db/tests/agent.rs
@@ -439,6 +439,27 @@ async fn a_hidden_table_is_never_read(connection: &Connection, on: &str) {
         }
         Dialect::Sqlite => {}
     }
+    // What is hidden, asked outside a statement, as the server asks it to
+    // leave hidden tables out of what it says of the schema.
+    let home = match connection.dialect() {
+        Dialect::Sqlite => "main",
+        Dialect::Postgres => "public",
+        Dialect::MySql => "bookshop",
+    };
+    let hidden = connection.agent_hidden(&customers).await.unwrap();
+    assert!(hidden.hides(home, "customers"), "{on}");
+    assert!(hidden.hides(home, "customer_names"), "{on}");
+    assert!(!hidden.hides(home, "books"), "{on}");
+    // A hidden table that is gone, renamed or dropped, closes the
+    // connection to agents until the person looks.
+    let gone = Tables::AllExcept(vec![named("customers"), named("clients")]);
+    let error = hiding(connection, &gone, "SELECT count(*) FROM books")
+        .await
+        .unwrap_err();
+    let look = Refusal::SettingsNeedALook(vec!["clients".into()]);
+    assert_eq!(error, Error::Agent(look.clone()), "{on}");
+    let hidden = connection.agent_hidden(&gone).await.unwrap();
+    assert_eq!(hidden.check_settings(), Err(look), "{on}");
     // Nothing hidden: the table reads.
     let found = count(hiding(connection, &Tables::All, "SELECT count(*) FROM customers").await);
     assert_eq!(found, Ok(vec![vec![Value::Int(8)]]), "{on}");
@@ -495,6 +516,28 @@ mod sqlite {
         an_agent_reads_the_bookshop(&connection, "sqlite").await;
     }
 
+    /// What a view reads, as SQLite says when a read of it is prepared:
+    /// however its text is written, a view over a hidden table is hidden
+    /// in what the server says of the schema.
+    #[tokio::test]
+    async fn a_view_is_hidden_by_what_sqlite_says_it_reads() {
+        let (connection, _dir) = bookshop_with(
+            "CREATE VIEW v_cte AS SELECT * FROM customers
+                 WHERE EXISTS (WITH customers AS (SELECT 1) SELECT 1 FROM customers);
+             CREATE VIEW v_string AS SELECT * FROM 'customers';
+             CREATE VIEW v_in AS SELECT title FROM books WHERE id IN customers;
+             CREATE VIEW v_over AS SELECT * FROM v_string;
+             CREATE VIEW v_books AS SELECT title FROM books;",
+        )
+        .await;
+        let customers = Tables::AllExcept(vec![named("customers")]);
+        let hidden = connection.agent_hidden(&customers).await.unwrap();
+        for view in ["v_cte", "v_string", "v_in", "v_over"] {
+            assert!(hidden.hides("main", view), "{view}");
+        }
+        assert!(!hidden.hides("main", "v_books"));
+    }
+
     #[tokio::test]
     async fn a_hidden_table_is_never_read_on_sqlite() {
         let (connection, _dir) = bookshop().await;
````

- [ ] **Step 2: Run them to see that they do not build**

Run: `cargo test --locked -p tabletist-db --lib agent::hidden`

Expected: it does not build. The tests name what is not there yet:

```text
error[E0599]: no method named `check_settings` found for struct `agent::hidden::Hidden` in the current scope
error[E0599]: `agent::hidden::TableName` doesn't implement `std::fmt::Display`
error: could not compile `tabletist-db` (lib test) due to 6 previous errors
```

- [ ] **Step 3: The stale entry, `hides`, and the catalog outside a statement**

````diff
diff --git a/crates/tabletist-db/src/adapter.rs b/crates/tabletist-db/src/adapter.rs
index ff28332..4a34b3c 100644
--- a/crates/tabletist-db/src/adapter.rs
+++ b/crates/tabletist-db/src/adapter.rs
@@ -1,6 +1,6 @@
 //! What the adapter of every database provides.
 
-use crate::agent::{Limits, Read, Tables};
+use crate::agent::{Catalog, Limits, Read, Tables};
 use crate::{
     CancelHandle, ChangeSet, Lookup, LookupPage, ObjectInfo, ObjectRef, Result, RowPage, RowQuery,
     ScriptMode, ScriptOutcome, StopFlag, Structure, WriteOutcome,
@@ -53,6 +53,10 @@ pub(crate) trait Adapter {
         stop: &StopFlag,
     ) -> Result<ScriptOutcome>;
 
+    /// What the database's catalog says for what an agent's connection
+    /// hides: its relations, what each reads, and the users' functions.
+    async fn agents_catalog(&self) -> Result<Catalog>;
+
     /// `changes` have passed their own check, and the session may write.
     async fn write(&self, changes: &ChangeSet, stop: &StopFlag) -> Result<WriteOutcome>;
 }
diff --git a/crates/tabletist-db/src/agent/hidden.rs b/crates/tabletist-db/src/agent/hidden.rs
index 37cdcc1..e38f03c 100644
--- a/crates/tabletist-db/src/agent/hidden.rs
+++ b/crates/tabletist-db/src/agent/hidden.rs
@@ -47,6 +47,22 @@ pub struct TableName {
     pub name: String,
 }
 
+/// As the person writes it: `customers`, or `sales.customers`. A part that
+/// holds a dot or a double quote is written in double quotes, a quote in it
+/// doubled.
+impl std::fmt::Display for TableName {
+    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
+        let part = |part: &str| match part.contains(['.', '"']) {
+            true => format!("\"{}\"", part.replace('"', "\"\"")),
+            false => part.to_owned(),
+        };
+        match &self.schema {
+            Some(schema) => write!(f, "{}.{}", part(schema), part(&self.name)),
+            None => f.write_str(&part(&self.name)),
+        }
+    }
+}
+
 impl TableName {
     /// Whether this names the relation `name` of `schema`.
     fn names(&self, schema: Option<&str>, name: &str) -> bool {
@@ -155,6 +171,9 @@ pub struct Hidden {
     unnamed_calls: Vec<String>,
     /// [`Catalog::table_functions`], folded.
     table_functions: HashSet<String>,
+    /// The entries of an "All except" list that name no relation of the
+    /// catalog, as the person writes them.
+    missing: Vec<String>,
 }
 
 impl Hidden {
@@ -216,6 +235,7 @@ impl Hidden {
                 .iter()
                 .map(|name| fold(name))
                 .collect(),
+            missing: missing(dialect, catalog, tables),
         }
     }
 
@@ -224,6 +244,22 @@ impl Hidden {
         self.tables.hides_anything()
     }
 
+    /// Whether agents may use the connection at all: not while its "All
+    /// except" list names a table it does not have. A hidden table that was
+    /// renamed is that, and its new name is hidden by nothing.
+    pub fn check_settings(&self) -> Result<(), Refusal> {
+        match self.missing.is_empty() {
+            true => Ok(()),
+            false => Err(Refusal::SettingsNeedALook(self.missing.clone())),
+        }
+    }
+
+    /// Whether the relation `name` of `schema` is hidden from agents: by
+    /// the person's names, by what it reads, or as a system catalog.
+    pub fn hides(&self, schema: &str, name: &str) -> bool {
+        self.check_engine_read(Some(schema), name).is_err()
+    }
+
     /// Whether `schema` is a schema of the database: one the catalog lists
     /// a relation of, or the system's.
     pub fn knows_schema(&self, schema: &str) -> bool {
@@ -294,6 +330,7 @@ impl Hidden {
         if !self.hides_anything() {
             return Ok(());
         }
+        self.check_settings()?;
         // Called by any statement that compares, sorts or casts the wrong
         // value, whatever it names.
         if let Some(function) = self.unnamed_calls.first() {
@@ -356,6 +393,26 @@ impl Hidden {
     }
 }
 
+/// The entries of `tables`' "All except" list that name no relation of
+/// `catalog`, written as the person writes them. A system catalog's name
+/// is none of them: those are hidden with anything.
+fn missing(dialect: Dialect, catalog: &Catalog, tables: &Tables) -> Vec<String> {
+    let Tables::AllExcept(names) = tables else {
+        return Vec::new();
+    };
+    names
+        .iter()
+        .filter(|named| {
+            !system_catalog(dialect, named.schema.as_deref(), &named.name)
+                && !catalog
+                    .relations
+                    .iter()
+                    .any(|relation| named.names(Some(&relation.schema), &relation.name))
+        })
+        .map(ToString::to_string)
+        .collect()
+}
+
 /// Refuses `found` where it is hidden, by the names the catalog writes.
 fn refused(found: &Listed) -> Result<(), Refusal> {
     match &found.why {
diff --git a/crates/tabletist-db/src/lib.rs b/crates/tabletist-db/src/lib.rs
index 42423e2..9d50965 100644
--- a/crates/tabletist-db/src/lib.rs
+++ b/crates/tabletist-db/src/lib.rs
@@ -272,6 +272,21 @@ impl Connection {
         agent::answer(outcome, stop)
     }
 
+    /// What `tables` hides on this connection's database as it is now: for
+    /// what the server tells an agent of the schema, which leaves it out.
+    /// The catalog is read only where something is hidden.
+    pub async fn agent_hidden(&self, tables: &agent::Tables) -> Result<agent::Hidden> {
+        let catalog = match tables.hides_anything() {
+            true => match &self.inner {
+                Inner::Sqlite(conn) => conn.agents_catalog().await?,
+                Inner::Postgres(conn) => conn.agents_catalog().await?,
+                Inner::MySql(conn) => conn.agents_catalog().await?,
+            },
+            false => agent::Catalog::default(),
+        };
+        Ok(agent::Hidden::new(self.dialect(), &catalog, tables))
+    }
+
     /// The server's name and version for the footer: `PostgreSQL 17.2`,
     /// `MySQL 8.4.3`, `MariaDB 10.11.6`, `SQLite 3.46.0`.
     pub async fn server_version(&self) -> Result<String> {
diff --git a/crates/tabletist-db/src/mysql.rs b/crates/tabletist-db/src/mysql.rs
index a613139..b9e5771 100644
--- a/crates/tabletist-db/src/mysql.rs
+++ b/crates/tabletist-db/src/mysql.rs
@@ -20,7 +20,7 @@ use mysql_async::{DriverError, IoError, Opts, OptsBuilder, Params, SslOpts, TxOp
 use mysql_common::named_params::ParsedNamedParams;
 
 use crate::adapter::Adapter;
-use crate::agent::{Limits, Read, Tables};
+use crate::agent::{Catalog, Limits, Read, Tables};
 use crate::script::retry_cancelled;
 use crate::{
     Access, CancelHandle, CancelInner, ChangeSet, ColumnClass, ColumnInfo, ColumnMeta, ConnectSpec,
@@ -523,6 +523,11 @@ impl Adapter for Conn {
         self.agent_read(read, limits, tables, stop).await
     }
 
+    async fn agents_catalog(&self) -> Result<Catalog> {
+        let mut conn = self.conn.lock().await;
+        hidden::catalog(&mut conn).await
+    }
+
     async fn write(&self, changes: &ChangeSet, stop: &StopFlag) -> Result<WriteOutcome> {
         self.save(changes, stop).await
     }
diff --git a/crates/tabletist-db/src/mysql/hidden.rs b/crates/tabletist-db/src/mysql/hidden.rs
index de934c6..c0c711a 100644
--- a/crates/tabletist-db/src/mysql/hidden.rs
+++ b/crates/tabletist-db/src/mysql/hidden.rs
@@ -170,7 +170,7 @@ fn rewritten_reads(note: &str, hidden: &Hidden) -> std::result::Result<(), Refus
 /// schema but the system's, what each view reads where the server says so
 /// (`VIEW_TABLE_USAGE`, MySQL 8.0.13 and later), and the users' routines.
 /// Where it does not say, every view is taken to read what is hidden.
-async fn catalog(conn: &mut mysql_async::Conn) -> Result<Catalog> {
+pub(super) async fn catalog(conn: &mut mysql_async::Conn) -> Result<Catalog> {
     let listed: Vec<(String, String, String)> = conn
         .query(format!(
             "SELECT TABLE_SCHEMA, TABLE_NAME, TABLE_TYPE FROM information_schema.TABLES \
@@ -225,8 +225,11 @@ async fn catalog(conn: &mut mysql_async::Conn) -> Result<Catalog> {
             let calls = calling
                 .as_ref()
                 .is_none_or(|calling| calling.contains(&(schema.clone(), name.clone())));
+            // A view the server lists no table of reads what it does not
+            // say: a user without SHOW VIEW is told nothing of any view.
+            let unsaid = depends_on.is_empty();
             Relation {
-                opaque: view && (usage.is_none() || calls),
+                opaque: view && (usage.is_none() || calls || unsaid),
                 schema,
                 name,
                 depends_on,
diff --git a/crates/tabletist-db/src/pg.rs b/crates/tabletist-db/src/pg.rs
index 0622144..26795ae 100644
--- a/crates/tabletist-db/src/pg.rs
+++ b/crates/tabletist-db/src/pg.rs
@@ -12,7 +12,7 @@ use tokio_postgres::tls::{MakeTlsConnect, TlsConnect};
 use tokio_postgres::{SimpleQueryMessage, Socket};
 
 use crate::adapter::Adapter;
-use crate::agent::{Limits, Read, Tables};
+use crate::agent::{Catalog, Limits, Read, Tables};
 use crate::{
     Access, CancelHandle, CancelInner, ChangeSet, ColumnInfo, ColumnMeta, ConnectSpec, Dialect,
     Error, ForeignKeyInfo, IndexInfo, Lookup, LookupPage, MAX_LISTED, Named, ObjectInfo,
@@ -838,6 +838,11 @@ impl Adapter for Conn {
         self.agent_read(read, limits, tables, stop).await
     }
 
+    async fn agents_catalog(&self) -> Result<Catalog> {
+        let client = self.client.lock().await;
+        hidden::catalog(&client).await
+    }
+
     async fn write(&self, changes: &ChangeSet, stop: &StopFlag) -> Result<WriteOutcome> {
         self.save(changes, stop).await
     }
diff --git a/crates/tabletist-db/src/pg/hidden.rs b/crates/tabletist-db/src/pg/hidden.rs
index 2198ec8..5d8dc80 100644
--- a/crates/tabletist-db/src/pg/hidden.rs
+++ b/crates/tabletist-db/src/pg/hidden.rs
@@ -186,7 +186,7 @@ fn planned_reads(plan: &serde_json::Value, hidden: &Hidden) -> std::result::Resu
 }
 
 /// The catalog as the open transaction sees it.
-async fn catalog(client: &tokio_postgres::Client) -> Result<Catalog> {
+pub(super) async fn catalog(client: &tokio_postgres::Client) -> Result<Catalog> {
     let mut relations: Vec<Relation> = Vec::new();
     for row in rows(client, &relations_sql()).await? {
         let (Some(schema), Some(name)) = (row.get(0), row.get(1)) else {
diff --git a/crates/tabletist-db/src/sqlite.rs b/crates/tabletist-db/src/sqlite.rs
index 65a8454..67ee7de 100644
--- a/crates/tabletist-db/src/sqlite.rs
+++ b/crates/tabletist-db/src/sqlite.rs
@@ -913,6 +913,12 @@ impl Adapter for Conn {
         CancelHandle(CancelInner::Sqlite(Arc::clone(&self.interrupt)))
     }
 
+    async fn agents_catalog(&self) -> Result<Catalog> {
+        let fences = self.fences.clone();
+        self.run(move |connection| agents_catalog(connection, &fences))
+            .await
+    }
+
     /// See [`crate::Connection::write`]. One blocking job. An interrupt
     /// reaches only the statement that is running and is not kept for the
     /// next, so `stop` is what ends the save between two of them.
@@ -1159,7 +1165,7 @@ impl Adapter for Conn {
                 // made a moment ago is in it.
                 let hidden = match tables.hides_anything() {
                     true => {
-                        let catalog = agents_catalog(connection)?;
+                        let catalog = agents_catalog(connection, &fences)?;
                         let hidden = Hidden::new(Dialect::Sqlite, &catalog, &tables);
                         hidden.check_statement(&read).map_err(Error::Agent)?;
                         Some(Arc::new(hidden))
@@ -1190,12 +1196,16 @@ impl Adapter for Conn {
 /// The relations of every schema of the file, its own `sqlite_` tables
 /// left out, for what an agent's connection hides. SQLite's authorizer
 /// reports a view's tables as it reads them, so what a view depends on is
-/// not read here. A shadow table depends on its virtual table, whose name
-/// and an underscore begin its own (FTS5's `notes_config` is `notes`'s).
-fn agents_catalog(connection: &rusqlite::Connection) -> Result<Catalog> {
+/// not read for the fence; for what the server tells an agent of the
+/// schema, a view depends on what SQLite reports it reads as a read of it
+/// is prepared. A shadow table depends
+/// on its virtual table, whose name and an underscore begin its own
+/// (FTS5's `notes_config` is `notes`'s).
+fn agents_catalog(connection: &rusqlite::Connection, fences: &Fences) -> Result<Catalog> {
     let mut statement = connection
         .prepare(
-            "SELECT t.schema, t.name, t.type = 'shadow', v.name FROM pragma_table_list t \
+            "SELECT t.schema, t.name, t.type = 'shadow', v.name, t.type = 'view' \
+             FROM pragma_table_list t \
              LEFT JOIN pragma_table_list v ON t.type = 'shadow' AND v.schema = t.schema \
                  AND v.type = 'virtual' \
                  AND lower(substr(t.name, 1, length(v.name) + 1)) = lower(v.name || '_') \
@@ -1211,13 +1221,18 @@ fn agents_catalog(connection: &rusqlite::Connection) -> Result<Catalog> {
                 row.get::<_, String>(1)?,
                 row.get::<_, bool>(2)?,
                 row.get::<_, Option<String>>(3)?,
+                row.get::<_, bool>(4)?,
             ))
         })
         .map_err(map_error)?
         .collect::<rusqlite::Result<Vec<_>>>()
         .map_err(map_error)?;
     let mut relations: Vec<Relation> = Vec::new();
-    for (schema, name, part, owner) in rows {
+    let mut views: Vec<(String, String)> = Vec::new();
+    for (schema, name, part, owner, view) in rows {
+        if view {
+            views.push((schema.clone(), name.clone()));
+        }
         let same = relations
             .last()
             .is_some_and(|last| last.schema == schema && last.name == name);
@@ -1234,6 +1249,28 @@ fn agents_catalog(connection: &rusqlite::Connection) -> Result<Catalog> {
             last.depends_on.push((schema, owner));
         }
     }
+    // What each view reads, as SQLite says when it prepares a read of the
+    // view. One it cannot prepare (a table it reads is gone) reads what
+    // nobody can say, and is hidden while anything is.
+    for relation in relations
+        .iter_mut()
+        .filter(|relation| views.contains(&(relation.schema.clone(), relation.name.clone())))
+    {
+        let quoted = |part: &str| format!("\"{}\"", part.replace('"', "\"\""));
+        let sql = format!(
+            "SELECT * FROM {}.{}",
+            quoted(&relation.schema),
+            quoted(&relation.name)
+        );
+        match fences.reads_of(|| connection.prepare(&sql).is_ok()) {
+            Some(reads) => relation.depends_on.extend(
+                reads
+                    .into_iter()
+                    .filter(|(schema, name)| *schema != relation.schema || *name != relation.name),
+            ),
+            None => relation.opaque = true,
+        }
+    }
     // The table functions, SQLite's own modules read without a table
     // made of them: the pragmas are refused before this, and `dbstat` and
     // the `sqlite_` ones are system catalogs.
diff --git a/crates/tabletist-db/src/sqlite/fence.rs b/crates/tabletist-db/src/sqlite/fence.rs
index f852240..78974c3 100644
--- a/crates/tabletist-db/src/sqlite/fence.rs
+++ b/crates/tabletist-db/src/sqlite/fence.rs
@@ -158,6 +158,8 @@ struct Shared {
     refused: Mutex<Option<Refusal>>,
     /// What the agent's connection hides, while its statement runs.
     hidden: Mutex<Option<Arc<Hidden>>>,
+    /// The tables read, by schema and name, while they are recorded.
+    reads: Mutex<Option<Vec<(String, String)>>>,
 }
 
 impl Fences {
@@ -198,6 +200,13 @@ impl Fences {
     /// So nothing hidden is read, however the text was written.
     pub(super) fn ask(&self, action: &Action<'_>) -> Authorization {
         let fence = self.current();
+        if action.code == ffi::SQLITE_READ
+            && let Some(table) = action.first
+            && let Some(reads) = self.reads().as_mut()
+        {
+            let schema = action.database.unwrap_or("main").to_owned();
+            reads.push((schema, table.to_owned()));
+        }
         let mut answer = authorize(fence, action);
         let mut hidden_read = None;
         if fence == Fence::Agent
@@ -237,6 +246,20 @@ impl Fences {
         self.0.hidden.lock().unwrap_or_else(PoisonError::into_inner)
     }
 
+    /// The tables SQLite reads as it prepares what `prepare` prepares, by
+    /// schema and name: for a view, every table it reads, through the
+    /// views it reads too. `None` where SQLite could not prepare it.
+    pub(super) fn reads_of(&self, prepare: impl FnOnce() -> bool) -> Option<Vec<(String, String)>> {
+        *self.reads() = Some(Vec::new());
+        let prepared = prepare();
+        let reads = self.reads().take();
+        prepared.then_some(reads).flatten()
+    }
+
+    fn reads(&self) -> std::sync::MutexGuard<'_, Option<Vec<(String, String)>>> {
+        self.0.reads.lock().unwrap_or_else(PoisonError::into_inner)
+    }
+
     /// What an agent's fence denied since the last fence went up.
     pub(super) fn refused(&self) -> Option<Refusal> {
         self.0
````

- [ ] **Step 4: Run the tests**

Run: `cargo test --locked -p tabletist-db --lib agent::hidden`, then `cargo test --locked -p tabletist-db --test agent` with both servers

Expected: PASS.

```text
test result: ok. 14 passed; 0 failed; 0 ignored; 0 measured; 470 filtered out
test result: ok. 28 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

- [ ] **Step 5: Commit**

```bash
cargo fmt --all
cargo clippy --locked --workspace --all-targets -- -D warnings
git add crates/tabletist-db
git commit -m "Ask what an agent's connection hides outside a statement, and close it where an entry names nothing"
```

---

### Task 2: The server says what is hidden, and leaves it out

- `Listed` carries the connection's `tables`. `Session::hidden(tables)` gives what they come to, and `Session::read` takes them; `run_query` passes the listed connection's.
- `list_connections` adds `hidden` for an "All except" list and `only` for an "Only" one, each entry as the person wrote it. Its description says what they mean.
- `describe_schema` asks the session what is hidden before it lists or describes, and refuses while `check_settings` fails. Its list leaves out every hidden relation; a hidden one asked for by name is named under `hidden`; a foreign key to a hidden table, or one of a hidden table's that points at this one, is left out of the table's entry.
- The app's gateway lists every connection with `Tables::All` until 1c-2b, and implements `hidden` and `read` over `Connection::agent_hidden` and `run_agent_read`. The tests' gateway gains `Shop::hide`.

**Files:**
- Modify: `crates/tabletist-mcp/src/gateway.rs`, `crates/tabletist-mcp/src/tools/connections.rs`, `crates/tabletist-mcp/src/tools/query.rs`, `crates/tabletist-mcp/src/tools/schema.rs`, `crates/tabletist-mcp/tests/support/mod.rs`, `crates/tabletist-mcp/tests/tools.rs`, `src/backend/agents.rs`

- [ ] **Step 1: Write the failing tests**

````diff
diff --git a/crates/tabletist-mcp/tests/support/mod.rs b/crates/tabletist-mcp/tests/support/mod.rs
index 58bdac8..30ec23d 100644
--- a/crates/tabletist-mcp/tests/support/mod.rs
+++ b/crates/tabletist-mcp/tests/support/mod.rs
@@ -22,7 +22,7 @@ use rmcp::model::{
 use rmcp::service::RunningService;
 use rmcp::{RoleClient, ServiceExt};
 use serde_json::Value as Json;
-use tabletist_db::agent::{Answer, Limits, Read};
+use tabletist_db::agent::{Answer, Hidden, Limits, Read, Tables};
 use tabletist_db::{
     Access, ConnectSpec, Connection, Dialect, Error, HostKeys, ObjectInfo, ObjectRef, Secrets,
     StopFlag, Structure,
@@ -93,12 +93,24 @@ impl Shop {
                 name: name.to_owned(),
                 dialect: Dialect::Sqlite,
                 limits,
+                tables: Tables::All,
             },
             path: path.to_owned(),
             turn: Arc::default(),
         });
     }
 
+    /// Sets what agents may not read of a listed connection.
+    pub fn hide(&self, name: &str, tables: Tables) {
+        let mut connections = self.inner.connections.lock().unwrap();
+        for kept in connections
+            .iter_mut()
+            .filter(|kept| kept.listed.name == name)
+        {
+            kept.listed.tables = tables.clone();
+        }
+    }
+
     /// Turns a connection's access Off.
     pub fn unlist(&self, name: &str) {
         self.inner
@@ -198,7 +210,11 @@ impl Session for ShopSession {
         Ok(self.connection.describe(object).await?)
     }
 
-    async fn read(&self, read: &Read, limits: Limits) -> Result<Answer, Failure> {
+    async fn hidden(&self, tables: &Tables) -> Result<Hidden, Failure> {
+        Ok(self.connection.agent_hidden(tables).await?)
+    }
+
+    async fn read(&self, read: &Read, limits: Limits, tables: &Tables) -> Result<Answer, Failure> {
         if self.stop.is_cancelled() {
             return Err(Error::Cancelled.into());
         }
@@ -210,9 +226,7 @@ impl Session for ShopSession {
             },
         };
         let flag = StopFlag::new();
-        let run =
-            self.connection
-                .run_agent_read(read, limits, &tabletist_db::agent::Tables::All, &flag);
+        let run = self.connection.run_agent_read(read, limits, tables, &flag);
         tokio::pin!(run);
         let answer = tokio::select! {
             answer = &mut run => answer,
diff --git a/crates/tabletist-mcp/tests/tools.rs b/crates/tabletist-mcp/tests/tools.rs
index 7708d83..bb47cff 100644
--- a/crates/tabletist-mcp/tests/tools.rs
+++ b/crates/tabletist-mcp/tests/tools.rs
@@ -12,7 +12,7 @@ use rmcp::model::ErrorCode;
 use rmcp::service::ServiceError;
 use serde_json::{Value as Json, json};
 use support::{Agent, LIMITS, Shop, request, server};
-use tabletist_db::agent::Limits;
+use tabletist_db::agent::{Limits, TableName, Tables};
 use tabletist_mcp::{MAX_ANSWER_BYTES, MAX_CELL_CHARS};
 
 const DEV: &str = "Bookshop · dev";
@@ -618,3 +618,131 @@ async fn describe_schema_takes_a_list_of_names_and_knows_no_connection_that_is_o
     assert_eq!(called.refused(), "unknown_connection");
     agent.close().await;
 }
+
+/// The Bookshop's `customers`, in every schema.
+fn customers() -> Tables {
+    Tables::AllExcept(vec![TableName {
+        schema: None,
+        name: "customers".into(),
+    }])
+}
+
+#[tokio::test]
+async fn list_connections_names_what_is_hidden() {
+    let shop = Shop::new();
+    let (server, _log) = server(&shop);
+    let agent = Agent::connect(&server).await;
+    shop.hide(DEV, customers());
+    let called = agent.call("list_connections", json!({})).await;
+    assert_eq!(
+        called.answer()["connections"][0]["hidden"],
+        json!(["customers"])
+    );
+    assert!(called.answer()["connections"][0].get("only").is_none());
+    shop.hide(
+        DEV,
+        Tables::Only(vec![TableName {
+            schema: Some("main".into()),
+            name: "books".into(),
+        }]),
+    );
+    let called = agent.call("list_connections", json!({})).await;
+    assert_eq!(
+        called.answer()["connections"][0]["only"],
+        json!(["main.books"])
+    );
+    assert!(called.answer()["connections"][0].get("hidden").is_none());
+    agent.close().await;
+}
+
+#[tokio::test]
+async fn a_hidden_table_is_left_out_of_the_schema_and_its_rows_are_refused() {
+    let shop = Shop::new();
+    let (server, _log) = server(&shop);
+    let agent = Agent::connect(&server).await;
+    shop.hide(DEV, customers());
+    // The list leaves out the table and the view over it.
+    let called = agent
+        .call("describe_schema", json!({ "connection": DEV }))
+        .await;
+    let names: Vec<&str> = called.answer()["tables"]
+        .as_array()
+        .unwrap()
+        .iter()
+        .map(|table| table["name"].as_str().unwrap())
+        .collect();
+    assert!(names.contains(&"orders"), "{names:?}");
+    assert!(!names.contains(&"customers"), "{names:?}");
+    assert!(!names.contains(&"customer_names"), "{names:?}");
+    // Asked for by name, it is named as hidden, and the key to it from
+    // orders is left out, as it would name its columns.
+    let called = agent
+        .call(
+            "describe_schema",
+            json!({ "connection": DEV, "tables": ["customers", "orders", "customer_names"] }),
+        )
+        .await;
+    let answer = called.answer();
+    assert_eq!(answer["hidden"], json!(["customers", "customer_names"]));
+    let orders = &answer["tables"].as_array().unwrap()[0];
+    assert_eq!(orders["name"], json!("orders"));
+    assert!(orders.get("foreign_keys").is_none(), "{orders}");
+    // Its rows are refused, and nothing is read.
+    let called = agent
+        .call(
+            "run_query",
+            json!({ "connection": DEV, "sql": "SELECT * FROM customers" }),
+        )
+        .await;
+    assert_eq!(called.refused(), "table_hidden");
+    assert_eq!(
+        called.text,
+        "customers is hidden for agents on this connection."
+    );
+    agent.close().await;
+}
+
+#[tokio::test]
+async fn a_hidden_table_that_is_gone_closes_the_connection_to_agents() {
+    let shop = Shop::new();
+    let (server, _log) = server(&shop);
+    let agent = Agent::connect(&server).await;
+    shop.hide(DEV, customers());
+    // Renamed: the entry names what is no longer there, and the new name
+    // is hidden by nothing.
+    shop.change("ALTER TABLE customers RENAME TO clients");
+    let look = "Agents may not use this connection until its agent settings are looked at in \
+                Tabletist: they hide customers, and its user can see no table of that name.";
+    for (tool, arguments) in [
+        (
+            "run_query",
+            json!({ "connection": DEV, "sql": "SELECT * FROM clients" }),
+        ),
+        ("describe_schema", json!({ "connection": DEV })),
+        (
+            "describe_schema",
+            json!({ "connection": DEV, "tables": ["clients"] }),
+        ),
+    ] {
+        let called = agent.call(tool, arguments).await;
+        assert_eq!(called.refused(), "connection_unavailable", "{tool}");
+        assert_eq!(called.text, look, "{tool}");
+    }
+    // The person looks: with the entry put right, the connection is open
+    // again.
+    shop.hide(
+        DEV,
+        Tables::AllExcept(vec![TableName {
+            schema: None,
+            name: "clients".into(),
+        }]),
+    );
+    let called = agent
+        .call(
+            "run_query",
+            json!({ "connection": DEV, "sql": "SELECT count(*) FROM books" }),
+        )
+        .await;
+    assert_eq!(called.answer()["rows"], json!([[12]]));
+    agent.close().await;
+}
diff --git a/src/backend/agents.rs b/src/backend/agents.rs
index 198e60a..2659825 100644
--- a/src/backend/agents.rs
+++ b/src/backend/agents.rs
@@ -969,6 +969,7 @@ mod tests {
                     rows: 500,
                     timeout: Duration::from_secs(10),
                 },
+                &Tables::All,
             )
             .await
     }
````

- [ ] **Step 2: Run them to see that they do not build**

Run: `cargo test --locked -p tabletist-mcp --test tools`

Expected: it does not build, as the gateway does not carry the tables yet:

```text
error[E0407]: method `hidden` is not a member of trait `Session`
error[E0050]: method `read` has 4 parameters but the declaration in trait `tabletist_mcp::Session::read` has 3
error[E0560]: struct `Listed` has no field named `tables`
error[E0609]: no field `tables` on type `Listed`
error: could not compile `tabletist-mcp` (test "tools") due to 4 previous errors
```

- [ ] **Step 3: The tables through the gateway, and the tools**

````diff
diff --git a/crates/tabletist-mcp/src/gateway.rs b/crates/tabletist-mcp/src/gateway.rs
index 6b1a39a..b97cfb2 100644
--- a/crates/tabletist-mcp/src/gateway.rs
+++ b/crates/tabletist-mcp/src/gateway.rs
@@ -5,7 +5,7 @@
 
 use std::future::Future;
 
-use tabletist_db::agent::{Answer, Limits, Read};
+use tabletist_db::agent::{Answer, Hidden, Limits, Read, Tables};
 use tabletist_db::{Dialect, ObjectInfo, ObjectRef, Structure};
 use tokio_util::sync::CancellationToken;
 
@@ -21,6 +21,9 @@ pub struct Listed {
     pub dialect: Dialect,
     /// The most rows of an answer, and how long a call may take.
     pub limits: Limits,
+    /// What agents may not read of it. The names are told to an agent
+    /// (the MCP plan's question 8): the rows are what is hidden.
+    pub tables: Tables,
 }
 
 /// The schemas of a connection.
@@ -94,11 +97,16 @@ pub trait Session: Send + Sync {
         object: &ObjectRef,
     ) -> impl Future<Output = Result<Structure, Failure>> + Send;
 
-    /// Runs a statement that passed the check
-    /// ([`tabletist_db::Connection::run_agent_read`]).
+    /// What `tables` hides on the database as it is now
+    /// ([`tabletist_db::Connection::agent_hidden`]).
+    fn hidden(&self, tables: &Tables) -> impl Future<Output = Result<Hidden, Failure>> + Send;
+
+    /// Runs a statement that passed the check, reading nothing `tables`
+    /// hides ([`tabletist_db::Connection::run_agent_read`]).
     fn read(
         &self,
         read: &Read,
         limits: Limits,
+        tables: &Tables,
     ) -> impl Future<Output = Result<Answer, Failure>> + Send;
 }
diff --git a/crates/tabletist-mcp/src/tools/connections.rs b/crates/tabletist-mcp/src/tools/connections.rs
index d49096d..a67c8ff 100644
--- a/crates/tabletist-mcp/src/tools/connections.rs
+++ b/crates/tabletist-mcp/src/tools/connections.rs
@@ -4,6 +4,7 @@ use std::time::Duration;
 
 use serde_json::{Map, Value as Json, json};
 use tabletist_db::Dialect;
+use tabletist_db::agent::{TableName, Tables};
 
 use super::{Done, Invalid, Note, Tool, schema};
 use crate::gateway::{Gateway, Listed};
@@ -16,7 +17,10 @@ pub fn tool() -> Tool {
         title: "List connections",
         description: "The database connections you may use, with what each is and its limits. \
                       Call a connection by its name, exactly as it is written here. A \
-                      connection the person has not opened to agents is not listed.",
+                      connection the person has not opened to agents is not listed. `hidden` \
+                      names the tables you may not read, and `only` the tables you may read \
+                      where no others are: a view over a hidden table is hidden with it, and \
+                      while anything is hidden so are the system catalogs.",
         input: schema(json!({}), &[]),
     }
 }
@@ -47,14 +51,28 @@ fn entry(listed: &Listed) -> Json {
         Dialect::MySql => ("MySQL", "?"),
         Dialect::Sqlite => ("SQLite", "?"),
     };
-    json!({
-        "name": listed.name,
-        "engine": engine,
-        "parameters": parameters,
-        "access": "read",
-        "rows": listed.limits.rows,
-        "timeout_s": seconds(listed.limits.timeout),
-    })
+    let mut entry = Map::new();
+    entry.insert("name".into(), json!(listed.name));
+    entry.insert("engine".into(), json!(engine));
+    entry.insert("parameters".into(), json!(parameters));
+    entry.insert("access".into(), json!("read"));
+    entry.insert("rows".into(), json!(listed.limits.rows));
+    entry.insert("timeout_s".into(), seconds(listed.limits.timeout));
+    // What is hidden, by the names the person gave.
+    let written = |names: &[TableName]| -> Json {
+        Json::Array(names.iter().map(|name| json!(name.to_string())).collect())
+    };
+    match &listed.tables {
+        Tables::All => {}
+        Tables::AllExcept(names) if names.is_empty() => {}
+        Tables::AllExcept(names) => {
+            entry.insert("hidden".into(), written(names));
+        }
+        Tables::Only(names) => {
+            entry.insert("only".into(), written(names));
+        }
+    }
+    Json::Object(entry)
 }
 
 /// Whole seconds as a whole number.
diff --git a/crates/tabletist-mcp/src/tools/query.rs b/crates/tabletist-mcp/src/tools/query.rs
index 4f69ee0..9afeaa4 100644
--- a/crates/tabletist-mcp/src/tools/query.rs
+++ b/crates/tabletist-mcp/src/tools/query.rs
@@ -86,7 +86,7 @@ async fn run<G: Gateway>(
     let session = settle(gateway.open(&listed.id, stop.token()).await, stop, &listed)?;
     note.ran_as = Some(RAN_AS);
     let limits = listed.limits;
-    let answer = within(stop, limits, session.read(&read, limits)).await;
+    let answer = within(stop, limits, session.read(&read, limits, &listed.tables)).await;
     let answer = settle(answer, stop, &listed)?;
     let written = answer::write(&answer, MAX_ANSWER_BYTES).map_err(|_| {
         Refused::new(
diff --git a/crates/tabletist-mcp/src/tools/schema.rs b/crates/tabletist-mcp/src/tools/schema.rs
index b59407e..ac83697 100644
--- a/crates/tabletist-mcp/src/tools/schema.rs
+++ b/crates/tabletist-mcp/src/tools/schema.rs
@@ -4,6 +4,7 @@
 use std::collections::HashMap;
 
 use serde_json::{Map, Value as Json, json};
+use tabletist_db::agent::Hidden;
 use tabletist_db::{Error, ObjectInfo, ObjectKind, ObjectRef, Structure};
 
 use super::{Arguments, Done, Invalid, Note, Tool, find, kept, schema, settle, within};
@@ -29,7 +30,9 @@ pub fn tool() -> Tool {
                       with a rough count of each one's rows, and the names of its other \
                       schemas. With `tables` (at most 20 names): each one's columns with their \
                       types, its primary key, its foreign keys and the keys that point at it, \
-                      and its indexes. Write a table of another schema as schema.table.",
+                      and its indexes. Write a table of another schema as schema.table. A \
+                      table the person hides from you is left out of the list, and named under \
+                      `hidden` when you ask for it.",
         input: schema(
             json!({
                 "connection": {
@@ -98,9 +101,13 @@ async fn run<G: Gateway>(
     }
     let session = settle(gateway.open(&listed.id, stop.token()).await, stop, &listed)?;
     let work = async {
+        let hidden = session.hidden(&listed.tables).await?;
+        hidden
+            .check_settings()
+            .map_err(|refusal| Failure::Database(Error::Agent(refusal)))?;
         match tables {
-            None => list(&session).await,
-            Some(tables) => describe(&session, stop, tables).await,
+            None => list(&session, &hidden).await,
+            Some(tables) => describe(&session, stop, tables, &hidden).await,
         }
     };
     let described = settle(within(stop, listed.limits, work).await, stop, &listed)?;
@@ -117,11 +124,16 @@ struct Described {
 }
 
 /// The tables and views of the connection's own schema, by name, and the
-/// names of its other schemas.
-async fn list<S: Session>(session: &S) -> Result<Described, Failure> {
+/// names of its other schemas. What is hidden is left out.
+async fn list<S: Session>(session: &S, hidden: &Hidden) -> Result<Described, Failure> {
     let Schemas { names, home } = session.schemas().await?;
     let objects = match &home {
-        Some(home) => session.objects(home).await?,
+        Some(home) => session
+            .objects(home)
+            .await?
+            .into_iter()
+            .filter(|object| !hidden.hides(home, &object.name))
+            .collect(),
         None => Vec::new(),
     };
     let mut room = Room::new();
@@ -178,12 +190,14 @@ async fn list<S: Session>(session: &S) -> Result<Described, Failure> {
 }
 
 /// What each of `asked` is made of. A name that is no table of the
-/// connection is listed as not found, and the others are described all the
-/// same. Tables that do not fit the answer are named as left out.
+/// connection is listed as not found, one that is hidden as hidden, and the
+/// others are described all the same. Tables that do not fit the answer
+/// are named as left out.
 async fn describe<S: Session>(
     session: &S,
     stop: &Stop,
     asked: &[String],
+    hidden: &Hidden,
 ) -> Result<Described, Failure> {
     let Schemas { names, home } = session.schemas().await?;
     let mut room = Room::new();
@@ -194,7 +208,7 @@ async fn describe<S: Session>(
     // Each schema's tables, read once.
     let mut listings: HashMap<String, Vec<ObjectInfo>> = HashMap::new();
     let mut tables = Vec::new();
-    let (mut not_found, mut left_out) = (Vec::new(), Vec::new());
+    let (mut not_found, mut left_out, mut hidden_ones) = (Vec::new(), Vec::new(), Vec::new());
     let (mut columns, mut full) = (0, false);
     for name in asked {
         if full {
@@ -220,9 +234,13 @@ async fn describe<S: Session>(
             not_found.push(name);
             continue;
         };
+        if hidden.hides(&object.schema, &object.name) {
+            hidden_ones.push(name);
+            continue;
+        }
         let structure = session.describe(&object).await?;
         let foreign = home.as_deref() != Some(object.schema.as_str());
-        let entry = table(&object, object_kind, foreign, &structure);
+        let entry = table(&object, object_kind, foreign, &structure, hidden);
         if room.take(&entry) {
             tables.push(entry);
             columns += structure.columns.len() as u64;
@@ -237,6 +255,9 @@ async fn describe<S: Session>(
     if !not_found.is_empty() {
         json.insert("not_found".into(), json!(not_found));
     }
+    if !hidden_ones.is_empty() {
+        json.insert("hidden".into(), json!(hidden_ones));
+    }
     if !left_out.is_empty() {
         json.insert("truncated".into(), json!(true));
         json.insert("left_out".into(), json!(left_out));
@@ -300,11 +321,14 @@ fn kind(kind: ObjectKind) -> &'static str {
     }
 }
 
+/// A table's entry. A key to or from a hidden table is left out: it would
+/// name that table's columns.
 fn table(
     object: &ObjectRef,
     object_kind: ObjectKind,
     foreign: bool,
     structure: &Structure,
+    hidden: &Hidden,
 ) -> Json {
     let mut entry = Map::new();
     entry.insert("name".into(), json!(object.name));
@@ -344,9 +368,13 @@ fn table(
         true => table.to_owned(),
         false => format!("{schema}.{table}"),
     };
-    if !structure.foreign_keys.is_empty() {
-        let keys: Vec<Json> = structure
-            .foreign_keys
+    let foreign_keys: Vec<_> = structure
+        .foreign_keys
+        .iter()
+        .filter(|key| !hidden.hides(&key.ref_schema, &key.ref_table))
+        .collect();
+    if !foreign_keys.is_empty() {
+        let keys: Vec<Json> = foreign_keys
             .iter()
             .map(|key| {
                 json!({
@@ -359,9 +387,13 @@ fn table(
             .collect();
         entry.insert("foreign_keys".into(), Json::Array(keys));
     }
-    if !structure.referenced_by.is_empty() {
-        let keys: Vec<Json> = structure
-            .referenced_by
+    let referenced_by: Vec<_> = structure
+        .referenced_by
+        .iter()
+        .filter(|key| !hidden.hides(&key.schema, &key.table))
+        .collect();
+    if !referenced_by.is_empty() {
+        let keys: Vec<Json> = referenced_by
             .iter()
             .map(|key| {
                 json!({
diff --git a/src/backend/agents.rs b/src/backend/agents.rs
index 2659825..8899b1c 100644
--- a/src/backend/agents.rs
+++ b/src/backend/agents.rs
@@ -12,7 +12,7 @@ use std::sync::atomic::AtomicU32;
 use std::sync::{Arc, Mutex};
 use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
 
-use tabletist_db::agent::{Answer, Limits, Read};
+use tabletist_db::agent::{Answer, Hidden, Limits, Read, Tables};
 use tabletist_db::{
     Access, CancelHandle, ConnectSpec, Connection, Error, HostKeys, ObjectInfo, ObjectRef, Secrets,
     StopFlag, Structure,
@@ -654,6 +654,9 @@ impl Gateway for AppGateway {
                 name: agentable.name.clone(),
                 dialect: agentable.spec.driver.dialect(),
                 limits: agentable.limits,
+                // What the connection hides comes with its setting in
+                // connections.toml (1c-2b): until then, nothing.
+                tables: Tables::All,
             })
             .collect()
     }
@@ -900,15 +903,17 @@ impl Session for AppSession {
         self.ended(None, connection.describe(object)).await
     }
 
-    async fn read(&self, read: &Read, limits: Limits) -> Result<Answer, Failure> {
+    async fn hidden(&self, tables: &Tables) -> Result<Hidden, Failure> {
+        let connection = self.connection()?;
+        self.ended(None, connection.agent_hidden(tables)).await
+    }
+
+    async fn read(&self, read: &Read, limits: Limits, tables: &Tables) -> Result<Answer, Failure> {
         let connection = self.connection()?;
         let flag = StopFlag::new();
-        // What the connection hides is the next run's (1c-2): until it
-        // is set there, every table is read as before.
-        let tables = tabletist_db::agent::Tables::All;
         self.ended(
             Some(&flag),
-            connection.run_agent_read(read, limits, &tables, &flag),
+            connection.run_agent_read(read, limits, tables, &flag),
         )
         .await
     }
````

- [ ] **Step 4: Run the tests**

Run: `cargo test --locked -p tabletist-mcp --test tools`, then `cargo test --locked --lib backend::agents` with both servers

Expected: PASS.

```text
test result: ok. 23 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
test result: ok. 29 passed; 0 failed; 0 ignored; 0 measured; 2518 filtered out
```

- [ ] **Step 5: Commit**

```bash
cargo fmt --all
cargo clippy --locked --workspace --all-targets -- -D warnings
git add crates/tabletist-mcp src/backend/agents.rs
git commit -m "Tell agents what a connection hides, and leave it out of what the server says of the schema"
```

---

### Task 3: The master plan

- 1c-2 in two, the server first; C.3's renames stopped on all three, and every change seen at once; B.8's `connection_unavailable` for a stale entry; what this run built.

**Files:**
- Modify: `docs/superpowers/plans/2026-10-13-mcp-server.md`

- [ ] **Step 1: The edits**

````diff
diff --git a/docs/superpowers/plans/2026-10-13-mcp-server.md b/docs/superpowers/plans/2026-10-13-mcp-server.md
index cbc5f17..209336b 100644
--- a/docs/superpowers/plans/2026-10-13-mcp-server.md
+++ b/docs/superpowers/plans/2026-10-13-mcp-server.md
@@ -73,6 +73,12 @@ that hides something, each as this plan recommended. G cuts 1c into three
 runs. Its first (`2026-10-15-mcp-server-1c-1-checks.md`) built the checks, with nothing visible: B.2 and
 B.3 are corrected where its draft showed otherwise.
 
+**Settled on 2026-10-15: 1c's second run in two, the server first, and a
+renamed table stopped on all three.** The server's side (`2026-10-15-mcp-server-1c-2a-server.md`) comes
+before the setting, so that when the setting appears everything of it is
+enforced. A hidden table that was renamed or dropped stops agents on that
+connection on PostgreSQL too: C.3 is corrected.
+
 ## Contents
 
 1. [What the code says, in short](#1-what-the-code-says-in-short)
@@ -1252,7 +1258,7 @@ The codes are stable. The sentences may be reworded.
 | Code | Sentence (an example) | When |
 |---|---|---|
 | `unknown_connection` | No connection named Bookshop · production. | The name matches nothing an agent may see. A connection that is Off answers exactly this |
-| `connection_unavailable` | Tabletist could not open Bookshop · staging. Open it in Tabletist to see why. | The password is asked for each time, the SSH host is not trusted, the keyring is locked, the connect failed. The reason is never sent: it can name a host or a user |
+| `connection_unavailable` | Tabletist could not open Bookshop · staging. Open it in Tabletist to see why. | The password is asked for each time, the SSH host is not trusted, the keyring is locked, the connect failed. The reason is never sent: it can name a host or a user. Also an "All except" list that names a table the database does not have (C.3): "Agents may not use this connection until its agent settings are looked at in Tabletist: they hide customers, and its user can see no table of that name." |
 | `too_many_statements` | Send one statement per call. | B.2 |
 | `not_parsed` | Tabletist could not read this statement, so it was not run. | B.2 |
 | `not_read_only` | run_query only runs reads. Use propose_write for DELETE FROM book_covers WHERE id = 9. | A write, `INTO`, `FOR UPDATE`. Until Phase 2 has `propose_write` the sentence ends at what the statement is ("run_query only runs reads, and this is a DELETE."): the pointer is the server's to add, not the check's |
@@ -1597,6 +1603,43 @@ How soon a change is seen:
 - MySQL: within 30 seconds, the age at which the catalog is read again.
   This is the gap named in B.3.
 
+**As built** (`2026-10-15-mcp-server-1c-1-checks.md` and `2026-10-15-mcp-server-1c-2a-server.md`). Agreed on
+2026-10-15, where it went otherwise:
+
+- **A hidden table renamed or dropped stops agents on all three.** Nothing
+  keeps an OID: the checks compare names (B.3), and an OID changes with
+  every dump and restore. An "All except" entry that names no relation of
+  the database closes the connection to agents
+  (`Hidden::check_settings`, `connection_unavailable`), on every call and
+  on `describe_schema`, until the person removes the entry or points it at
+  a table. The sentence names the entry. A drop stops them too: it looks
+  like a rename. On MySQL so does a table the connection's user has no
+  right to, which its `information_schema` does not list: whether it is
+  there cannot be told. An "Only" entry that names nothing hides more,
+  not less, and stops nothing. An entry that names a system catalog is
+  never stale.
+- **Every change is seen at once, on all three.** The catalog is read for
+  each call (B.3), so MySQL has no 30 seconds.
+- **The server leaves hidden tables out of the schema.** `describe_schema`
+  asks the session what is hidden (`Session::hidden`, through
+  `Connection::agent_hidden`, which reads the catalog only where something
+  is hidden). Its list leaves out every hidden relation, a view over a
+  hidden table with it; a hidden table asked for by name is named under
+  `hidden`; and a foreign key to a hidden table, or from one, is left out
+  of a table's entry, as it would name the hidden table's columns.
+- **What a view reads, asked of the engine for the listing too.** On
+  SQLite the catalog prepares a read of each view and keeps the tables the
+  authorizer reports, through the views it reads; a view that cannot be
+  prepared is hidden while anything is. On MySQL a view of which
+  `VIEW_TABLE_USAGE` lists no table is hidden while anything is: the
+  server tells a user without `SHOW VIEW` nothing of any view. Found by
+  the review of this run, with views that read a hidden table and were
+  described.
+- `list_connections` gives what is hidden as the person named it: `hidden`
+  for an "All except" list, `only` for an "Only" one, each entry written
+  `customers` or `sales.customers`, a part that holds a dot or a quote in
+  double quotes.
+
 ## D. UI work
 
 Views draw and push `Action`s. `App::apply` changes state. Database and disk
@@ -2326,7 +2369,7 @@ is off until the person turns it on.
 | 1a. The enforcement core | 3 | Done: pull requests 122, 123 and 124 |
 | 1b. The server over stdio, with its interface | 9, below | Built (`2026-10-13-mcp-server-1b-1-server.md`, `2026-10-14-mcp-server-1b-2a-gateway.md`, `2026-10-14-mcp-server-1b-2b-stdio.md`, `2026-10-14-mcp-server-1b-3a-agents-tab.md`, `2026-10-14-mcp-server-1b-3b-changes-outside.md`, `2026-10-14-mcp-server-1b-4a-activity.md`, `2026-10-14-mcp-server-1b-4b-log.md`, `2026-10-15-mcp-server-1b-5a-access.md` and `2026-10-15-mcp-server-1b-5b-settings.md`) |
 | 1b on Windows: the way in | 1 | Built (`2026-10-15-mcp-server-1b-6-windows.md`) |
-| 1c. Hidden tables | 3, below | The checks built (`2026-10-15-mcp-server-1c-1-checks.md`). The setting and the server are next |
+| 1c. Hidden tables | 4, below | The checks and the server's side built (`2026-10-15-mcp-server-1c-1-checks.md`, `2026-10-15-mcp-server-1c-2a-server.md`). The setting is next |
 | 1d. HTTP with tokens | 1 | Dropped for now (2026-10-15) |
 
 The move of the connections to `connections.toml` came between 1a and 1b
@@ -2445,6 +2488,10 @@ build; no console target.
   them out, a rename that stops agents, and changes made outside the app
   held (C.3). Then the editor's Tables control in both looks, its hint,
   and the website.
+- The second cut in two, the same day, the server first: 1c-2a, the
+  server's side with nothing to set it (`2026-10-15-mcp-server-1c-2a-server.md`); 1c-2b, the keys in
+  `connections.toml`, the app passing them, changes made outside the app
+  held, and the reference page for the keys.
 - Files: `crates/tabletist-db/src/catalog.rs`, `pg.rs`, `mysql.rs`,
   `sqlite.rs` (what views read, routines, partitions), `agent/` (names,
   dependencies, the plan check, the system catalogs), `tabletist-sqlite-ffi`
````

- [ ] **Step 2: Check the whole of it**

Run: `cargo fmt --all --check`, clippy as above, `RUSTDOCFLAGS='-D warnings' cargo doc --locked --workspace --no-deps`, `grep -c "$(printf '\342\200\224')" docs/superpowers/plans/2026-10-13-mcp-server.md` (a count of em dashes) and `cargo test --locked --workspace --all-targets` with both servers

Expected: fmt and clippy clean, the documentation builds without a warning, no em dash (`0`), and:

```text
3520 passed, 0 failed, 22 ignored
```

- [ ] **Step 3: Commit**

```bash
git add docs/superpowers/plans/2026-10-13-mcp-server.md
git commit -m "Say in the MCP plan what Phase 1c's run 2a built, and stop a renamed hidden table on all three"
```

## Self-review

- **Against the master plan.** G's 1c-2, as cut on 2026-10-15: `list_connections` naming the hidden tables (Task 2), `describe_schema` leaving them out (Task 2), a rename that stops agents (Task 1, `check_settings`, on all three as agreed). C.3's table and timing, and B.8's `connection_unavailable`, corrected in Task 3. Changes made outside the app held, and the setting itself, are 1c-2b's.
- **A review before the plan.** A reviewer ran the draft against both servers and found, each with a run: a MySQL view over a hidden table described with its columns to a user without `SHOW VIEW`; a MySQL entry naming a table the user has no right to, which closed the connection with a sentence that called the table missing; and three SQLite views (a CTE named like the table, `FROM 'customers'`, `IN customers`) that a reading of the view's text missed and that were described. Fixed: SQLite asks the authorizer what a view reads, MySQL hides a view of which nothing is listed, and the sentence says the user can see no such table, with a test of the SQLite views. The MySQL cases need a second user on the server, which the tests' account cannot make: they are not in the suite. It found sound: PostgreSQL's views across schemas, materialized views, partitions, inheritance children and quoted names; foreign keys under "Only"; a hidden name never described; a connection that hides nothing answered byte for byte as before. It left nothing on either server.
- **Left, and said.** A connection whose hidden table its MySQL user cannot see is closed until the entry goes. MariaDB, and MySQL before 8.0.13, hide every view while anything is, as 1c-1 has it.
- **Placeholders.** None: every step has its diff or its command and what it prints.
- **Names.** `check_settings`, `hides`, `agent_hidden`, `agents_catalog`, `Refusal::SettingsNeedALook`, `Listed::tables`, `Session::hidden`, `Shop::hide` and `Fences::reads_of` are the same in every task that uses them.

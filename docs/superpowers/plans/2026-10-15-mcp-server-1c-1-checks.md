# MCP server, Phase 1c, run 1: the hidden-table checks. Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** `Connection::run_agent_read` takes what the connection hides (`agent::Tables`: all, all but a list, only a list) and never reads a hidden table: on SQLite, PostgreSQL and MySQL, by any name, through a view, a partition, a child table, a common table expression or a function of the database's own. While anything is hidden the system catalogs are refused too. Nothing is visible yet: the app passes "All" until the next run reads the setting.

**Architecture:** `crates/tabletist-db/src/agent/hidden.rs` holds the rule. `Hidden::new` works out from a driver's `Catalog` (its relations, what each depends on, the users' functions) which relations are hidden and why. Two checks use it. `check_statement` reads the names the statement's text gives off the app's own tokens: it names what the agent asked for, and it is not trusted alone. The fence is what the engine says the statement reads, handed to `check_engine_read`: SQLite's authorizer under `Fence::Agent` (`sqlite/fence.rs`), and the statement's plan in the run's own read-only transaction on PostgreSQL (`pg/hidden.rs`, `EXPLAIN (VERBOSE, FORMAT JSON)`) and MySQL (`mysql/hidden.rs`, `EXPLAIN FORMAT=JSON` and the rewritten query `SHOW WARNINGS` gives). The catalog is read for each call, in that transaction.

**Tech Stack:** Rust 2024, rusqlite's authorizer through `tabletist-sqlite-ffi`, tokio-postgres, mysql_async, `serde_json` (moved from the crate's dev-dependencies: nothing new in `Cargo.lock`).

---

## Where this sits

Phase 1c of `docs/superpowers/plans/2026-10-13-mcp-server.md`, hidden tables, in three runs (agreed on 2026-10-15). This is the first: the checks in `tabletist-db` (B.3). The second puts the setting in `connections.toml` and has the server refuse, list and leave out what is hidden; the third is the editor's Tables control and the website. Questions 8 and 13 are settled as the master plan recommended (Task 5).

## Before you start

- Work on a branch of `main`. One commit for each task, in the order below. Every commit passes `cargo fmt --all --check`, `cargo clippy --locked --workspace --all-targets -- -D warnings` and `cargo test --locked --workspace --all-targets`.
- On Igor's machine `cargo` is `~/.cargo/bin/cargo` (the mise shim fails), and a build in a worktree needs `CARGO_TARGET_DIR` set to that worktree's `target/`.
- Read `AGENTS.md`: no `unsafe`, and `tests/engines.rs` refuses a `==` between two `Dialect`s in the library's code: name every engine in a `match`.
- **Tasks 3 and 4 need the test servers**, PostgreSQL 17 and MySQL 8.4 from `compose.yaml`, on ports 55432 and 53306. Run their tests with `TABLETIST_TEST_PG_URL=postgres://tabletist:tabletist@localhost:55432/tabletist` and `TABLETIST_TEST_MYSQL_URL=mysql://tabletist:tabletist@localhost:53306/tabletist`. Without them a server's test prints "skipped" and passes, which proves nothing. CI's two integration jobs run `tests/agent.rs` already.
- The diffs below apply with `git apply`. Run `cargo fmt --all` before each commit: the code here is already formatted.

## What was run before this plan was written

Every task's state was built here, passed `cargo fmt --check` and clippy with `-D warnings`, and its tests failed and then passed as each step says, against both servers. The last state passed `cargo test --locked --workspace --all-targets` with both servers (3,507 tests, none failed) and `RUSTDOCFLAGS='-D warnings' cargo doc --locked --workspace --no-deps`. Not run: CI, and Windows and macOS (nothing here is per system).

## What this run settles

| Question | Decision | Why |
|---|---|---|
| Question 8: is an agent told the names of hidden tables? | Yes. The refusal names the table, and the view it asked for where it read one | Agreed on 2026-10-15, as the master plan recommended. Answering as for a table that does not exist leaves the agent guessing; the rows are what is hidden |
| Question 13: the system catalogs | Open on a connection that hides nothing, closed while anything is hidden (`system_catalog`) | Agreed on 2026-10-15, as recommended. They describe every table, hidden ones too |
| Phase 1c's cut | Three runs: the checks (this one), the setting and the server, the editor and the website | Agreed on 2026-10-15 |
| How the statement's names are read | Off the app's tokens, never off the tree; erring toward finding | B.2 keeps the tree from being walked, as its stack is not bounded. The reading is not the fence, the engine's report is, so a name it misses is still refused there |
| When the catalog is read | For each call, in the call's own read-only transaction; not at all where nothing is hidden | The check sees the database as the statement will: a view or a table made a moment ago is in it. Two or three queries more for each call on PostgreSQL and MySQL |
| What SQLite's authorizer refuses | Every `SQLITE_READ` behind an agent's fence of a hidden table, named with the view it was read for | It reports what SQLite reads after its own parse, views expanded: no list of what views depend on is needed |
| PostgreSQL's fence | `EXPLAIN (VERBOSE, FORMAT JSON)` with the agent's values, each `Relation Name` and its `Schema`, and each `Function Name` | The plan names every scan after the server has expanded views, found partitions and child tables, and resolved names by the search path |
| MySQL's fence | `EXPLAIN FORMAT=JSON` and the note 1003 `SHOW WARNINGS` gives after it | The JSON names tables by their aliases; the note names every one `` `db`.`table` ``, views expanded, but leaves out what the optimizer answered while planning. Each covers the other |
| A function of the database's own | Refused while anything is hidden, on PostgreSQL and MySQL | Its body is in no plan (B.3, point 4) |
| The Bookshop's function | On PostgreSQL alone | MySQL lets only a user with `SUPER` make a function while it logs its changes (error 1419 on the test server) |
| Names and case | Compared without regard to case everywhere | The strict reading: a quoted `"Customers"` on PostgreSQL is refused where `customers` is hidden |
| A poisoned lock in SQLite's fences | Still holds what is hidden | Found while cutting the plan: the draft let go of it, and so read what was hidden, after a panic |

### What this run leaves out

- The setting itself (`connections.toml`), the server's answers (`list_connections` naming the hidden tables, `describe_schema` leaving them out), a rename that stops agents and changes made outside the app: the next run. Until then `src/backend/agents.rs` passes `Tables::All`.
- The editor's Tables control, its hint, and the website: the third run.
- Masked columns (Phase 3a), though `system_catalog` is theirs to refuse too.

## File structure

| File | What changes |
|---|---|
| `crates/tabletist-db/src/agent/hidden.rs` | New. `Tables`, `TableName`, `Relation`, `Catalog`, `Hidden` and its checks; the system catalogs; the reading of a statement's names off its tokens; `planned`, the query an `EXPLAIN` explains |
| `crates/tabletist-db/src/agent.rs` | The module; `Refusal::TableHidden` and `Refusal::SystemCatalog`, with their codes and sentences |
| `crates/tabletist-db/src/lib.rs`, `adapter.rs` | `run_agent_read` and `run_read` take `tables` |
| `crates/tabletist-db/src/sqlite.rs`, `sqlite/fence.rs` | The catalog from `pragma_table_list`; the authorizer refuses a hidden table's read |
| `crates/tabletist-db/src/pg.rs`, `pg/hidden.rs`, `pg/script.rs` | The catalog from `pg_class`, `pg_rewrite`, `pg_depend`, `pg_inherits` and `pg_proc`; the plan's relations and functions; the check in the read-only transaction |
| `crates/tabletist-db/src/mysql.rs`, `mysql/hidden.rs`, `mysql/script.rs` | The catalog from `information_schema`; the plan's JSON and the rewritten query; the check in the read-only transaction |
| `crates/tabletist-db/Cargo.toml` | `serde_json` from the dev-dependencies to the dependencies |
| `crates/tabletist-db/fixtures/bookshop/postgres.sql` | `customer_count()`, a function that reads `customers` |
| `crates/tabletist-db/tests/agent.rs` | The hidden-table rows of F.2, on each engine |
| `src/backend/agents.rs`, `crates/tabletist-mcp/tests/support/mod.rs` | Pass `Tables::All` |
| `docs/superpowers/plans/2026-10-13-mcp-server.md` | Questions 8 and 13; 1c in three runs; what this run built |

---

### Task 1: What is hidden, and the names a statement gives

- `agent::Tables` is the connection's setting: `All`, `AllExcept(names)` or `Only(names)`. A `TableName` without a schema is that name in every schema. `hides_anything()` is false for `All` and an empty `AllExcept`, and true for any `Only`, an empty one too: then every table is hidden.
- A driver reads its `Catalog`: each `Relation` with what it depends on (a view's tables, a partition's or a child table's parent), `opaque` where that cannot be known, and the users' routines. `Hidden::new` works out once why each relation is hidden: by its own name, or by the first hidden relation it depends on, which the refusal names (`customer_names reads customers, which is hidden…`). A cycle, which a catalog should not have, is hidden.
- `check_engine_read(schema, name)` is the fence's check: a system catalog is refused, a listed relation by why it is hidden, and one the catalog does not list (made since it was read) by the person's names alone. `check_name` is for a name that may be the statement's alias: only what the catalog lists is refused. `check_function` refuses a function the users wrote while anything is hidden.
- `check_statement` reads the text's names off the app's tokens (B.2: the tree is never walked): every name in a table's place at any depth (after `FROM`, a `JOIN`, a comma of a `FROM`'s list, `LATERAL`, `ONLY`, in parentheses where a table goes), what a `WITH` defines, and each name with a parenthesis after it as a call. A common table expression named like a hidden table is refused, not reasoned about.
- The system catalogs: PostgreSQL's `pg_catalog`, `information_schema`, `pg_toast` and `pg_temp` schemas, and a `pg_` name without a schema, which PostgreSQL finds in `pg_catalog` first; MySQL's `information_schema`, `mysql`, `performance_schema` and `sys`; SQLite's `sqlite_` tables in any schema.
- `Refusal::TableHidden { name, through }` (`table_hidden`) and `Refusal::SystemCatalog` (`system_catalog`), with B.8's sentences.

**Files:**
- Create: `crates/tabletist-db/src/agent/hidden.rs`
- Modify: `crates/tabletist-db/src/agent.rs`

- [ ] **Step 1: Write the failing tests**

The module's tests, and the module declared with what it gives the crate.

````diff
diff --git a/crates/tabletist-db/src/agent.rs b/crates/tabletist-db/src/agent.rs
index f83c0ad..d5939d5 100644
--- a/crates/tabletist-db/src/agent.rs
+++ b/crates/tabletist-db/src/agent.rs
@@ -28,8 +28,10 @@ use sqlparser::tokenizer::{Token, Tokenizer, Whitespace};
 use crate::sql::TokenKind;
 use crate::{Dialect, sql};
 
+mod hidden;
 mod run;
 
+pub use hidden::{Catalog, Hidden, Relation, TableName, Tables};
 pub use run::{Answer, Limits};
 pub(crate) use run::{Sent, answer, count_mismatch};
 
@@ -130,9 +132,19 @@ pub enum Refusal {
     /// is, for the sentence.
     NotAllowed(String),
     /// A function with an effect outside its result, by its name as
-    /// written.
+    /// written. Also one the database's users wrote, on a connection that
+    /// hides a table.
     FunctionNotAllowed(String),
     InvalidParams(ParamFault),
+    /// A table the person hides from agents, by its name, and the relation
+    /// the statement named that depends on it, where it named another.
+    TableHidden {
+        name: String,
+        through: Option<String>,
+    },
+    /// The system catalogs, on a connection that hides a table: they
+    /// describe every one.
+    SystemCatalog,
 }
 
 /// What was too large.
@@ -181,6 +193,8 @@ impl Refusal {
             Self::NotAllowed(_) => "not_allowed",
             Self::FunctionNotAllowed(_) => "function_not_allowed",
             Self::InvalidParams(_) => "invalid_params",
+            Self::TableHidden { .. } => "table_hidden",
+            Self::SystemCatalog => "system_catalog",
         }
     }
 }
@@ -247,6 +261,20 @@ impl fmt::Display for Refusal {
             Self::InvalidParams(ParamFault::NotFinite(position)) => {
                 write!(f, "Parameter {position} is not a finite number.")
             }
+            Self::TableHidden {
+                name,
+                through: None,
+            } => write!(f, "{name} is hidden for agents on this connection."),
+            Self::TableHidden {
+                name,
+                through: Some(through),
+            } => write!(
+                f,
+                "{through} reads {name}, which is hidden for agents on this connection."
+            ),
+            Self::SystemCatalog => f.write_str(
+                "The system catalogs are not available on this connection. Use describe_schema.",
+            ),
         }
     }
 }
diff --git a/crates/tabletist-db/src/agent/hidden.rs b/crates/tabletist-db/src/agent/hidden.rs
new file mode 100644
index 0000000..16780ea
--- /dev/null
+++ b/crates/tabletist-db/src/agent/hidden.rs
@@ -0,0 +1,320 @@
+//! Hidden tables: the tables of a connection an agent may not read, and
+//! the checks that keep it from them (the MCP plan's B.3).
+//!
+//! A connection's [`Tables`] says which tables the person hides: none, all
+//! but a list, or every one but a list. [`Hidden`] is what that comes to
+//! on a database, from its [`Catalog`]: a view over a hidden table is
+//! hidden with it, and so is a partition or a child table of one, and
+//! while anything is hidden the system catalogs are too, since they
+//! describe every table.
+//!
+//! Two checks use it. [`Hidden::check_statement`] reads the names the
+//! statement's own text gives, off its tokens, before anything is sent:
+//! it gives the agent a sentence that names what it asked for, and it is
+//! the one that sees a hidden view whose tables are not. It is not the
+//! fence: a reading of the text is never trusted alone (B.2). The fence is
+//! what the engine says it reads, which each driver asks of it and hands
+//! to [`Hidden::check_engine_read`]: SQLite's authorizer, and the plan of
+//! the statement on PostgreSQL and MySQL.
+//!
+//! Names are compared without regard to case, which is the strict reading
+//! where a database tells case apart: a quoted `"Customers"` on PostgreSQL
+//! is taken for `customers` too, and refused where that is hidden.
+
+#[cfg(test)]
+mod tests {
+    use super::*;
+
+    fn table(schema: Option<&str>, name: &str) -> TableName {
+        TableName {
+            schema: schema.map(str::to_owned),
+            name: name.to_owned(),
+        }
+    }
+
+    fn relation(schema: &str, name: &str, depends_on: &[(&str, &str)]) -> Relation {
+        Relation {
+            schema: schema.to_owned(),
+            name: name.to_owned(),
+            depends_on: depends_on
+                .iter()
+                .map(|(schema, name)| ((*schema).to_owned(), (*name).to_owned()))
+                .collect(),
+            opaque: false,
+        }
+    }
+
+    /// The Bookshop as PostgreSQL lists it, with a few relations more:
+    /// another schema's `customers`, a materialized view over the
+    /// customers, a partitioned table and a child table.
+    fn bookshop() -> Catalog {
+        Catalog {
+            relations: vec![
+                relation("public", "books", &[]),
+                relation("public", "customers", &[]),
+                relation("public", "orders", &[]),
+                relation("sales", "customers", &[]),
+                relation("public", "customer_names", &[("public", "customers")]),
+                relation("public", "recent_orders", &[("public", "orders")]),
+                relation("public", "customer_count", &[("public", "customer_names")]),
+                relation("public", "vip", &[("public", "customers")]),
+                relation("public", "events", &[]),
+                relation("public", "events_2026", &[("public", "events")]),
+            ],
+            routines: vec![("public".into(), "names".into())],
+        }
+    }
+
+    fn checked(dialect: Dialect, sql: &str) -> Read {
+        super::super::check_read(dialect, sql, &[]).unwrap()
+    }
+
+    fn hidden(tables: Tables) -> Hidden {
+        Hidden::new(Dialect::Postgres, &bookshop(), &tables)
+    }
+
+    fn statement(hidden: &Hidden, sql: &str) -> Result<(), Refusal> {
+        hidden.check_statement(&checked(Dialect::Postgres, sql))
+    }
+
+    fn table_hidden(name: &str, through: Option<&str>) -> Result<(), Refusal> {
+        Err(Refusal::TableHidden {
+            name: name.to_owned(),
+            through: through.map(str::to_owned),
+        })
+    }
+
+    #[test]
+    fn a_name_in_any_place_of_a_query_is_read_off_its_tokens() {
+        let read = |sql: &str| names(Dialect::Postgres, sql);
+        let relations = |sql: &str| {
+            read(sql)
+                .relations
+                .into_iter()
+                .map(|name| match name.schema {
+                    Some(schema) => format!("{schema}.{}", name.name),
+                    None => name.name,
+                })
+                .collect::<Vec<_>>()
+        };
+        assert_eq!(relations("SELECT * FROM books"), ["books"]);
+        assert_eq!(
+            relations("SELECT * FROM public.books b, \"Sales\".\"Customers\" AS c"),
+            ["public.books", "Sales.Customers"]
+        );
+        assert_eq!(
+            relations(
+                "SELECT * FROM books b JOIN orders o ON o.id = b.id \
+                 LEFT JOIN LATERAL (SELECT * FROM customers) c ON true, events"
+            ),
+            ["books", "orders", "customers", "events"]
+        );
+        // Nested, and in a set operation; the commas of a select list and
+        // of an IN are not a FROM's.
+        assert_eq!(
+            relations(
+                "SELECT a, (SELECT count(*) FROM orders), b FROM books \
+                 WHERE id IN (1, 2) UNION SELECT 1, 2, 3 FROM customers ORDER BY a, b"
+            ),
+            ["orders", "books", "customers"]
+        );
+        // A comment between a name and its dot, and a database's name
+        // before the schema.
+        assert_eq!(
+            relations("SELECT * FROM shop.public /* the schema */ . books"),
+            ["public.books"]
+        );
+        // A table in parentheses, a join of them, and a list in them.
+        assert_eq!(
+            relations("SELECT * FROM (customers JOIN books ON true), ((orders), events)"),
+            ["customers", "books", "orders", "events"]
+        );
+        assert_eq!(
+            relations("SELECT * FROM books JOIN (customers) ON true"),
+            ["books", "customers"]
+        );
+        // A table function is a call.
+        let found = read("SELECT * FROM names() n, generate_series(1, 3)");
+        assert!(found.relations.is_empty(), "{found:?}");
+        assert!(found.functions.iter().any(|name| name.name == "names"));
+        // What a WITH defines, and what it does not.
+        assert_eq!(
+            read(
+                "WITH RECURSIVE a (x) AS (SELECT 1), b AS MATERIALIZED (SELECT * FROM a) \
+                 SELECT * FROM b"
+            )
+            .defined,
+            ["a", "b"]
+        );
+        assert!(
+            read("SELECT * FROM unnest(ARRAY[1]) WITH ORDINALITY")
+                .defined
+                .is_empty()
+        );
+        // MySQL's backquotes, a doubled one inside.
+        let mysql = names(Dialect::MySql, "SELECT * FROM `shop`.`odd``name`");
+        assert_eq!(
+            mysql.relations,
+            [Name {
+                schema: Some("shop".into()),
+                name: "odd`name".into()
+            }]
+        );
+    }
+
+    #[test]
+    fn a_hidden_table_is_refused_by_any_name_it_goes_by() {
+        let hidden = hidden(Tables::AllExcept(vec![table(Some("public"), "customers")]));
+        assert_eq!(statement(&hidden, "SELECT * FROM books"), Ok(()));
+        for sql in [
+            "SELECT * FROM customers",
+            "SELECT * FROM public.customers",
+            "SELECT * FROM \"Customers\"",
+            "SELECT * FROM CUSTOMERS c",
+            "SELECT * FROM books JOIN customers ON true",
+            "SELECT * FROM books WHERE EXISTS (SELECT 1 FROM customers)",
+            "SELECT * FROM books, LATERAL (SELECT * FROM customers) c",
+            "SELECT 1 FROM books UNION SELECT 1 FROM customers",
+            "WITH c AS (SELECT * FROM customers) SELECT * FROM c",
+        ] {
+            assert_eq!(
+                statement(&hidden, sql),
+                table_hidden("customers", None),
+                "{sql}"
+            );
+        }
+        // Another schema's table of the same name is not this one.
+        assert_eq!(statement(&hidden, "SELECT * FROM sales.customers"), Ok(()));
+        // A common table expression named like the hidden table.
+        assert_eq!(
+            statement(
+                &hidden,
+                "WITH customers AS (SELECT 1) SELECT * FROM customers"
+            ),
+            table_hidden("customers", None)
+        );
+    }
+
+    #[test]
+    fn what_depends_on_a_hidden_table_is_hidden_with_it_and_says_so() {
+        let hidden = hidden(Tables::AllExcept(vec![table(None, "customers")]));
+        // A view, a materialized view over that view, and a child table.
+        assert_eq!(
+            statement(&hidden, "SELECT * FROM customer_names"),
+            table_hidden("customers", Some("customer_names"))
+        );
+        assert_eq!(
+            statement(&hidden, "SELECT * FROM customer_count"),
+            table_hidden("customer_names", Some("customer_count"))
+        );
+        assert_eq!(
+            statement(&hidden, "SELECT * FROM vip"),
+            table_hidden("customers", Some("vip"))
+        );
+        // Without a schema the entry is every schema's.
+        assert_eq!(
+            statement(&hidden, "SELECT * FROM sales.customers"),
+            table_hidden("customers", None)
+        );
+        // A partition of a hidden table.
+        let events = self::hidden(Tables::AllExcept(vec![table(None, "events")]));
+        assert_eq!(
+            events.check_engine_read(Some("public"), "events_2026"),
+            table_hidden("events", Some("events_2026"))
+        );
+    }
+
+    #[test]
+    fn only_a_list_hides_the_rest_and_the_tables_made_later() {
+        let hidden = hidden(Tables::Only(vec![
+            table(None, "books"),
+            table(None, "customer_names"),
+        ]));
+        assert_eq!(statement(&hidden, "SELECT * FROM books"), Ok(()));
+        assert_eq!(
+            statement(&hidden, "SELECT * FROM orders"),
+            table_hidden("orders", None)
+        );
+        // A listed view over a table that is not listed is hidden too.
+        assert_eq!(
+            statement(&hidden, "SELECT * FROM customer_names"),
+            table_hidden("customers", Some("customer_names"))
+        );
+        // A common table expression is no table to list.
+        assert_eq!(
+            statement(&hidden, "WITH x AS (SELECT * FROM books) SELECT * FROM x"),
+            Ok(())
+        );
+        // A table the catalog did not list yet, as the engine reports it.
+        assert_eq!(
+            hidden.check_engine_read(Some("public"), "made_later"),
+            table_hidden("made_later", None)
+        );
+        assert_eq!(hidden.check_engine_read(Some("public"), "books"), Ok(()));
+    }
+
+    #[test]
+    fn the_system_catalogs_and_the_users_functions_are_refused_only_while_something_is_hidden() {
+        let open = hidden(Tables::All);
+        let hiding = hidden(Tables::AllExcept(vec![table(None, "customers")]));
+        for sql in [
+            "SELECT * FROM information_schema.tables",
+            "SELECT * FROM pg_catalog.pg_class",
+            "SELECT * FROM pg_stats",
+        ] {
+            assert_eq!(statement(&open, sql), Ok(()), "{sql}");
+            assert_eq!(
+                statement(&hiding, sql),
+                Err(Refusal::SystemCatalog),
+                "{sql}"
+            );
+        }
+        assert_eq!(
+            hiding.check_engine_read(Some("pg_catalog"), "pg_class"),
+            Err(Refusal::SystemCatalog)
+        );
+        assert_eq!(
+            open.check_engine_read(Some("pg_catalog"), "pg_class"),
+            Ok(())
+        );
+        // A function a user wrote, in an expression and in a FROM.
+        for sql in ["SELECT names()", "SELECT * FROM public.names()"] {
+            assert_eq!(statement(&open, sql), Ok(()), "{sql}");
+            assert_eq!(
+                statement(&hiding, sql),
+                Err(Refusal::FunctionNotAllowed("names".into())),
+                "{sql}"
+            );
+        }
+        assert_eq!(statement(&hiding, "SELECT lower(title) FROM books"), Ok(()));
+        // SQLite's own tables, in any schema; MySQL's system schemas.
+        let sqlite = Hidden::new(
+            Dialect::Sqlite,
+            &Catalog::default(),
+            &Tables::AllExcept(vec![table(None, "customers")]),
+        );
+        let read = checked(Dialect::Sqlite, "SELECT * FROM temp.sqlite_master");
+        assert_eq!(sqlite.check_statement(&read), Err(Refusal::SystemCatalog));
+        let mysql = Hidden::new(
+            Dialect::MySql,
+            &Catalog::default(),
+            &Tables::Only(Vec::new()),
+        );
+        assert_eq!(
+            mysql.check_engine_read(Some("mysql"), "tbl"),
+            Err(Refusal::SystemCatalog)
+        );
+    }
+
+    #[test]
+    fn nothing_is_hidden_by_an_empty_list_or_by_all() {
+        for tables in [Tables::All, Tables::AllExcept(Vec::new())] {
+            let hidden = hidden(tables);
+            assert!(!hidden.hides_anything());
+            assert_eq!(statement(&hidden, "SELECT * FROM customers"), Ok(()));
+            assert_eq!(hidden.check_engine_read(None, "customers"), Ok(()));
+        }
+        // An empty "Only" hides every table.
+        assert!(Tables::Only(Vec::new()).hides_anything());
+    }
+}
````

- [ ] **Step 2: Run them to see that they do not build**

Run: `cargo test --locked -p tabletist-db --lib agent::hidden`

Expected: it does not build. The tests name what is not there yet; among the errors:

```text
error[E0432]: unresolved imports `hidden::Catalog`, `hidden::Hidden`, `hidden::Relation`, `hidden::TableName`, `hidden::Tables`
error[E0425]: cannot find type `TableName` in this scope
error[E0425]: cannot find type `Catalog` in this scope
error[E0425]: cannot find type `Hidden` in this scope
error[E0425]: cannot find function `names` in this scope
error: could not compile `tabletist-db` (lib test) due to 46 previous errors; 1 warning emitted
```

- [ ] **Step 3: The rule and the reading**

````diff
diff --git a/crates/tabletist-db/src/agent/hidden.rs b/crates/tabletist-db/src/agent/hidden.rs
index 16780ea..b460e6d 100644
--- a/crates/tabletist-db/src/agent/hidden.rs
+++ b/crates/tabletist-db/src/agent/hidden.rs
@@ -21,6 +21,680 @@
 //! where a database tells case apart: a quoted `"Customers"` on PostgreSQL
 //! is taken for `customers` too, and refused where that is hidden.
 
+use std::collections::{HashMap, HashSet};
+
+use super::{Read, Refusal};
+use crate::Dialect;
+use crate::sql::{self, TokenKind};
+
+/// What agents may read of a connection's tables.
+#[derive(Debug, Clone, Default, PartialEq, Eq)]
+pub enum Tables {
+    /// Every table.
+    #[default]
+    All,
+    /// Every table but these.
+    AllExcept(Vec<TableName>),
+    /// These tables and no others, the ones made later included.
+    Only(Vec<TableName>),
+}
+
+/// A table as the person names it: in its schema, or by its name alone,
+/// which is that name in every schema.
+#[derive(Debug, Clone, PartialEq, Eq)]
+pub struct TableName {
+    pub schema: Option<String>,
+    pub name: String,
+}
+
+impl TableName {
+    /// Whether this names the relation `name` of `schema`.
+    fn names(&self, schema: Option<&str>, name: &str) -> bool {
+        same(&self.name, name)
+            && match (&self.schema, schema) {
+                (None, _) => true,
+                (Some(own), Some(schema)) => same(own, schema),
+                // A relation without its schema, of a statement that did
+                // not give one, may be in any: it is taken for this one.
+                (Some(_), None) => true,
+            }
+    }
+}
+
+impl Tables {
+    /// Whether any table is hidden. Nothing is checked, and nothing more
+    /// is asked of the database, on a connection that hides nothing.
+    pub fn hides_anything(&self) -> bool {
+        match self {
+            Self::All => false,
+            Self::AllExcept(names) => !names.is_empty(),
+            Self::Only(_) => true,
+        }
+    }
+
+    /// Whether the person hides the relation `name` of `schema` by its own
+    /// name, before what it reads is looked at.
+    fn hides(&self, schema: Option<&str>, name: &str) -> bool {
+        match self {
+            Self::All => false,
+            Self::AllExcept(names) => names.iter().any(|named| named.names(schema, name)),
+            Self::Only(names) => !names.iter().any(|named| named.names(schema, name)),
+        }
+    }
+}
+
+/// A table, a view, a materialized view or a foreign table of the
+/// database, as its catalog lists it, with the relations it depends on: a
+/// view's and a materialized view's tables, and a partition's or a child
+/// table's parent. A system catalog is not listed.
+#[derive(Debug, Clone, PartialEq, Eq)]
+pub struct Relation {
+    pub schema: String,
+    pub name: String,
+    /// By schema and name. One the catalog does not list is passed over.
+    pub depends_on: Vec<(String, String)>,
+    /// Whether what it depends on cannot be known: a view, on a MySQL that
+    /// does not say what its views read. Hidden while anything is.
+    pub opaque: bool,
+}
+
+/// What a driver reads of its database's catalog for the checks: its
+/// relations, and the functions and procedures its users wrote.
+#[derive(Debug, Clone, Default, PartialEq, Eq)]
+pub struct Catalog {
+    pub relations: Vec<Relation>,
+    /// By schema and name. An extension's own are not among them.
+    pub routines: Vec<(String, String)>,
+}
+
+/// A relation of the catalog, as [`Hidden`] keeps it.
+#[derive(Debug, Clone)]
+struct Listed {
+    schema: String,
+    name: String,
+    why: Option<Why>,
+}
+
+/// Why a relation is hidden.
+#[derive(Debug, Clone, PartialEq, Eq)]
+enum Why {
+    /// The person hides it.
+    Named,
+    /// It depends on this hidden relation, by its name.
+    DependsOn(String),
+}
+
+/// What [`Tables`] comes to on one database: each relation that is hidden
+/// and why, and the functions its users wrote, which are refused while
+/// anything is hidden (B.2).
+#[derive(Debug, Clone)]
+pub struct Hidden {
+    dialect: Dialect,
+    tables: Tables,
+    /// Every listed relation, by its name folded to lower case: its schema
+    /// and its name as the catalog writes them, and why it is hidden where
+    /// it is.
+    relations: HashMap<String, Vec<Listed>>,
+    /// The users' functions' names, folded, while anything is hidden.
+    routines: HashSet<String>,
+}
+
+impl Hidden {
+    /// What `tables` hides of `catalog`, on a database that speaks
+    /// `dialect`.
+    pub fn new(dialect: Dialect, catalog: &Catalog, tables: &Tables) -> Self {
+        let index: HashMap<(String, String), usize> = catalog
+            .relations
+            .iter()
+            .enumerate()
+            .map(|(at, relation)| ((fold(&relation.schema), fold(&relation.name)), at))
+            .collect();
+        let depends: Vec<Vec<usize>> = catalog
+            .relations
+            .iter()
+            .map(|relation| {
+                relation
+                    .depends_on
+                    .iter()
+                    .filter_map(|(schema, name)| index.get(&(fold(schema), fold(name))).copied())
+                    .collect()
+            })
+            .collect();
+        let mut why: Vec<Option<Option<Why>>> = vec![None; catalog.relations.len()];
+        for at in 0..catalog.relations.len() {
+            hidden_why(at, catalog, &depends, tables, &mut why);
+        }
+        let mut relations: HashMap<String, Vec<Listed>> = HashMap::new();
+        for (relation, why) in catalog.relations.iter().zip(why) {
+            relations
+                .entry(fold(&relation.name))
+                .or_default()
+                .push(Listed {
+                    schema: relation.schema.clone(),
+                    name: relation.name.clone(),
+                    why: why.flatten(),
+                });
+        }
+        let routines = match tables.hides_anything() {
+            true => catalog
+                .routines
+                .iter()
+                .map(|(_, name)| fold(name))
+                .collect(),
+            false => HashSet::new(),
+        };
+        Self {
+            dialect,
+            tables: tables.clone(),
+            relations,
+            routines,
+        }
+    }
+
+    /// Whether anything is hidden.
+    pub fn hides_anything(&self) -> bool {
+        self.tables.hides_anything()
+    }
+
+    /// Checks a relation the engine says the statement reads: by its
+    /// schema where the engine gives it. One the catalog did not list (a
+    /// table made since it was read) is hidden as the person's own names
+    /// hide it.
+    pub fn check_engine_read(&self, schema: Option<&str>, name: &str) -> Result<(), Refusal> {
+        if !self.hides_anything() {
+            return Ok(());
+        }
+        if system_catalog(self.dialect, schema, name) {
+            return Err(Refusal::SystemCatalog);
+        }
+        match self.listed(schema, name) {
+            Some(found) => refused(found),
+            None if self.tables.hides(schema, name) => Err(Refusal::TableHidden {
+                name: name.to_owned(),
+                through: None,
+            }),
+            None => Ok(()),
+        }
+    }
+
+    /// Checks a relation the engine names where what it gives may be an
+    /// alias of the statement's own (MySQL's plan): refused when it is a
+    /// system catalog, or a listed relation that is hidden; a name the
+    /// catalog does not list is passed over.
+    pub fn check_name(&self, schema: Option<&str>, name: &str) -> Result<(), Refusal> {
+        if !self.hides_anything() {
+            return Ok(());
+        }
+        if system_catalog(self.dialect, schema, name) {
+            return Err(Refusal::SystemCatalog);
+        }
+        match self.listed(schema, name) {
+            Some(found) => refused(found),
+            None => Ok(()),
+        }
+    }
+
+    /// Checks a function the statement calls, by the name it is called by:
+    /// one the database's users wrote is refused while anything is hidden,
+    /// since what it reads cannot be seen (B.2).
+    pub fn check_function(&self, name: &str) -> Result<(), Refusal> {
+        match self.routines.contains(&fold(name)) {
+            true => Err(Refusal::FunctionNotAllowed(name.to_owned())),
+            false => Ok(()),
+        }
+    }
+
+    /// Checks the names the statement's own text gives, read off its
+    /// tokens (see the module's doc): every relation it names in a table's
+    /// place, every common table expression it defines, and every function
+    /// it calls. A name the catalog does not list is the database's to
+    /// answer, as a common table expression's is: the engine's own check
+    /// sees what it reads.
+    pub fn check_statement(&self, read: &Read) -> Result<(), Refusal> {
+        if !self.hides_anything() {
+            return Ok(());
+        }
+        let names = names(read.dialect(), read.text());
+        for defined in &names.defined {
+            // Named like a hidden table: refused, not reasoned about. Which
+            // a name in its scope meant, the table or the expression, is
+            // a question a wrong answer to would leak.
+            if let Some(found) = self.listed(None, defined)
+                && found.why.is_some()
+            {
+                return refused(found);
+            }
+        }
+        for relation in &names.relations {
+            let (schema, name) = (relation.schema.as_deref(), relation.name.as_str());
+            if system_catalog(self.dialect, schema, name) {
+                return Err(Refusal::SystemCatalog);
+            }
+            if schema.is_none() && names.defined.iter().any(|defined| same(defined, name)) {
+                continue;
+            }
+            if let Some(found) = self.listed(schema, name) {
+                refused(found)?;
+            }
+        }
+        for function in &names.functions {
+            self.check_function(&function.name)?;
+        }
+        Ok(())
+    }
+
+    /// The listed relation `name` of `schema` that is hidden, or else any
+    /// listed one: without a schema, every one of that name, and a hidden
+    /// one among them is the one taken.
+    fn listed(&self, schema: Option<&str>, name: &str) -> Option<&Listed> {
+        let found = self.relations.get(&fold(name))?;
+        let mut matching = found
+            .iter()
+            .filter(|listed| schema.is_none_or(|schema| same(&listed.schema, schema)));
+        let first = matching.clone().next()?;
+        Some(
+            matching
+                .find(|listed| listed.why.is_some())
+                .unwrap_or(first),
+        )
+    }
+}
+
+/// Refuses `found` where it is hidden, by the names the catalog writes.
+fn refused(found: &Listed) -> Result<(), Refusal> {
+    match &found.why {
+        None => Ok(()),
+        Some(Why::Named) => Err(Refusal::TableHidden {
+            name: found.name.clone(),
+            through: None,
+        }),
+        Some(Why::DependsOn(hidden)) => Err(Refusal::TableHidden {
+            name: hidden.clone(),
+            through: Some(found.name.clone()),
+        }),
+    }
+}
+
+/// Whether the relation at `at` is hidden, and why, worked out once: by
+/// its own name, else by the first hidden relation it depends on. A
+/// relation met again while its own answer is being worked out (a cycle
+/// the catalog should not have) is taken for hidden.
+fn hidden_why(
+    at: usize,
+    catalog: &Catalog,
+    depends: &[Vec<usize>],
+    tables: &Tables,
+    why: &mut [Option<Option<Why>>],
+) -> Option<Why> {
+    if let Some(known) = &why[at] {
+        return known.clone();
+    }
+    let relation = &catalog.relations[at];
+    // Hidden until worked out, for a cycle.
+    why[at] = Some(Some(Why::Named));
+    let found = if relation.opaque || tables.hides(Some(&relation.schema), &relation.name) {
+        Some(Why::Named)
+    } else {
+        depends[at].iter().find_map(|&on| {
+            hidden_why(on, catalog, depends, tables, why).map(|_| {
+                let name = &catalog.relations[on].name;
+                Why::DependsOn(name.clone())
+            })
+        })
+    };
+    // What a relation depends on through another is named by the one it
+    // reaches first, which is hidden itself: its name is told, not the
+    // hidden table at the end of the chain.
+    why[at] = Some(found.clone());
+    found
+}
+
+/// Whether the relation `name` of `schema` is one of the system catalogs,
+/// which describe every table: its schema's, or by its name where the
+/// database finds such a name in its catalog first.
+fn system_catalog(dialect: Dialect, schema: Option<&str>, name: &str) -> bool {
+    let in_schema = |systems: &[&str]| {
+        schema.is_some_and(|schema| systems.iter().any(|system| same(system, schema)))
+    };
+    match dialect {
+        // PostgreSQL finds a `pg_` name in `pg_catalog` first, whatever
+        // the search path; `pg_temp` schemas are the sessions' own.
+        Dialect::Postgres => {
+            in_schema(&["pg_catalog", "information_schema", "pg_toast"])
+                || schema.is_some_and(|schema| fold(schema).starts_with("pg_temp"))
+                || (schema.is_none() && fold(name).starts_with("pg_"))
+        }
+        Dialect::MySql => in_schema(&["information_schema", "mysql", "performance_schema", "sys"]),
+        // SQLite's own tables are `sqlite_` ones, in any schema.
+        Dialect::Sqlite => fold(name).starts_with("sqlite_"),
+    }
+}
+
+/// `name` as names are compared.
+fn fold(name: &str) -> String {
+    name.to_lowercase()
+}
+
+/// Whether two names are the same name.
+fn same(a: &str, b: &str) -> bool {
+    fold(a) == fold(b)
+}
+
+/// A name a statement gives, as written, without its quotes.
+#[derive(Debug, Clone, PartialEq, Eq)]
+struct Name {
+    schema: Option<String>,
+    name: String,
+}
+
+/// The names a statement gives, as [`names`] reads them.
+#[derive(Debug, Default, PartialEq, Eq)]
+struct Names {
+    /// In a table's place: after `FROM`, after a `JOIN`, after a comma in
+    /// a `FROM`.
+    relations: Vec<Name>,
+    /// Common table expressions, and windows, which are written alike.
+    defined: Vec<String>,
+    /// Called: a name with a parenthesis after it.
+    functions: Vec<Name>,
+}
+
+/// A token as [`names`] reads it.
+#[derive(Debug, Clone, PartialEq, Eq)]
+enum Piece {
+    Word {
+        text: String,
+        quoted: bool,
+        keyword: bool,
+    },
+    Dot,
+    Open,
+    Close,
+    Comma,
+    Other,
+}
+
+impl Piece {
+    /// Whether this is the unquoted word `word`, in any case.
+    fn is(&self, word: &str) -> bool {
+        matches!(self, Self::Word { text, quoted: false, .. } if text.eq_ignore_ascii_case(word))
+    }
+
+    /// The word, where it can be a name: quoted, or not a keyword.
+    fn name(&self) -> Option<&str> {
+        match self {
+            Self::Word {
+                text,
+                quoted,
+                keyword,
+            } if *quoted || !*keyword => Some(text),
+            _ => None,
+        }
+    }
+}
+
+/// The words that end a `FROM`'s list at its own depth.
+const ENDS_FROM: &[&str] = &[
+    "WHERE",
+    "GROUP",
+    "HAVING",
+    "ORDER",
+    "LIMIT",
+    "OFFSET",
+    "FETCH",
+    "UNION",
+    "INTERSECT",
+    "EXCEPT",
+    "WINDOW",
+    "QUALIFY",
+    "FOR",
+    "SELECT",
+    "VALUES",
+];
+
+/// The words a table's place begins after, besides `FROM`.
+const JOINS: &[&str] = &["JOIN", "STRAIGHT_JOIN"];
+
+/// Reads the names a statement gives off the app's own tokens (B.2: the
+/// tree is not walked). It errs toward finding: a name that is a column's
+/// in a table's place is checked as a table's too.
+fn names(dialect: Dialect, text: &str) -> Names {
+    let pieces = pieces(dialect, text);
+    let mut found = Names::default();
+    // Depth of parentheses, and the depths at which a `FROM`'s list is
+    // being read.
+    let mut depth = 0_usize;
+    let mut from_at: Vec<usize> = Vec::new();
+    // Whether the piece before was where a table goes: a parenthesis there
+    // that opens no query holds a table, or a join of tables, `(customers
+    // JOIN books ON …)`, whose names are read as a `FROM`'s list is.
+    let mut table_goes = false;
+    let mut at = 0;
+    while at < pieces.len() {
+        let piece = &pieces[at];
+        let places_table = matches!(piece, Piece::Comma) && from_at.last() == Some(&depth)
+            || piece.is("FROM")
+            || JOINS.iter().any(|join| piece.is(join))
+            || (table_goes && (piece.is("LATERAL") || piece.is("ONLY")));
+        match piece {
+            Piece::Open => {
+                depth += 1;
+                let query = pieces.get(at + 1).is_some_and(|next| {
+                    ["SELECT", "WITH", "VALUES", "TABLE"]
+                        .iter()
+                        .any(|start| next.is(start))
+                });
+                if table_goes && !query {
+                    from_at.push(depth);
+                    table_place(&pieces, at + 1, &mut found);
+                }
+            }
+            Piece::Close => {
+                from_at.retain(|&level| level < depth);
+                depth = depth.saturating_sub(1);
+            }
+            Piece::Comma if from_at.last() == Some(&depth) => {
+                table_place(&pieces, at + 1, &mut found);
+            }
+            Piece::Word { .. } if piece.is("FROM") => {
+                from_at.retain(|&level| level < depth);
+                from_at.push(depth);
+                table_place(&pieces, at + 1, &mut found);
+            }
+            Piece::Word { .. } if JOINS.iter().any(|join| piece.is(join)) => {
+                table_place(&pieces, at + 1, &mut found);
+            }
+            Piece::Word { .. } if ENDS_FROM.iter().any(|end| piece.is(end)) => {
+                from_at.retain(|&level| level < depth);
+            }
+            Piece::Word { .. } if piece.is("WITH") => defined(&pieces, at + 1, &mut found),
+            _ => {}
+        }
+        table_goes = places_table || (table_goes && matches!(piece, Piece::Open));
+        // A name with a parenthesis after it is a call, wherever it is.
+        if let Some((name, after)) = dotted(&pieces, at)
+            && pieces.get(after) == Some(&Piece::Open)
+            && !found.functions.contains(&name)
+        {
+            found.functions.push(name);
+        }
+        at += 1;
+    }
+    found
+}
+
+/// Reads what stands in a table's place from `at`: past `LATERAL` and
+/// `ONLY`, a name, which is a relation's, or a function's where a
+/// parenthesis follows it. A parenthesis there opens a query or a join,
+/// whose own names are read as the rest are.
+fn table_place(pieces: &[Piece], mut at: usize, found: &mut Names) {
+    while pieces
+        .get(at)
+        .is_some_and(|piece| piece.is("LATERAL") || piece.is("ONLY"))
+    {
+        at += 1;
+    }
+    let Some((name, after)) = dotted(pieces, at) else {
+        return;
+    };
+    // A table function: [`names`] takes it for a call.
+    if pieces.get(after) == Some(&Piece::Open) {
+        return;
+    }
+    if !found.relations.contains(&name) {
+        found.relations.push(name);
+    }
+}
+
+/// Reads the names a `WITH` defines from `at`: each `name [(columns)] AS
+/// [NOT] [MATERIALIZED] (`, the ones after a comma too. What is not so
+/// written (`WITH ORDINALITY`, `WITH TIME ZONE`) defines nothing.
+fn defined(pieces: &[Piece], mut at: usize, found: &mut Names) {
+    if pieces.get(at).is_some_and(|piece| piece.is("RECURSIVE")) {
+        at += 1;
+    }
+    loop {
+        let Some(name) = pieces.get(at).and_then(Piece::name) else {
+            return;
+        };
+        let name = name.to_owned();
+        at += 1;
+        if pieces.get(at) == Some(&Piece::Open) {
+            at = closing(pieces, at) + 1;
+        }
+        if !pieces.get(at).is_some_and(|piece| piece.is("AS")) {
+            return;
+        }
+        at += 1;
+        while pieces
+            .get(at)
+            .is_some_and(|piece| piece.is("NOT") || piece.is("MATERIALIZED"))
+        {
+            at += 1;
+        }
+        if pieces.get(at) != Some(&Piece::Open) {
+            return;
+        }
+        found.defined.push(name);
+        at = closing(pieces, at) + 1;
+        if pieces.get(at) != Some(&Piece::Comma) {
+            return;
+        }
+        at += 1;
+    }
+}
+
+/// The place of the parenthesis that closes the one at `open`, or the end.
+fn closing(pieces: &[Piece], open: usize) -> usize {
+    let mut depth = 0_usize;
+    for (at, piece) in pieces.iter().enumerate().skip(open) {
+        match piece {
+            Piece::Open => depth += 1,
+            Piece::Close => {
+                depth = depth.saturating_sub(1);
+                if depth == 0 {
+                    return at;
+                }
+            }
+            _ => {}
+        }
+    }
+    pieces.len()
+}
+
+/// A name at `at`, with the parts after it joined by dots: its last part
+/// is the name and the one before it the schema (a third, a database's or
+/// a catalog's, is passed over). Gives where it ends.
+fn dotted(pieces: &[Piece], at: usize) -> Option<(Name, usize)> {
+    let mut parts = vec![pieces.get(at)?.name()?.to_owned()];
+    let mut next = at + 1;
+    while pieces.get(next) == Some(&Piece::Dot)
+        && let Some(part) = pieces.get(next + 1).and_then(Piece::name)
+    {
+        parts.push(part.to_owned());
+        next += 2;
+    }
+    let name = parts.pop()?;
+    Some((
+        Name {
+            schema: parts.pop(),
+            name,
+        },
+        next,
+    ))
+}
+
+/// The statement as [`names`] reads it: words, dots, parentheses and
+/// commas, with comments and white space left out.
+fn pieces(dialect: Dialect, text: &str) -> Vec<Piece> {
+    sql::tokenize(dialect, text)
+        .into_iter()
+        .filter_map(|token| {
+            let written = &text[token.range.clone()];
+            Some(match token.kind {
+                TokenKind::Whitespace | TokenKind::Comment => return None,
+                TokenKind::Keyword | TokenKind::Identifier => Piece::Word {
+                    text: written.to_owned(),
+                    quoted: false,
+                    keyword: sql::is_keyword(dialect, written) || read_as_keyword(written),
+                },
+                TokenKind::QuotedIdentifier => Piece::Word {
+                    text: unquoted(written),
+                    quoted: true,
+                    keyword: false,
+                },
+                TokenKind::Punctuation => match written {
+                    "." => Piece::Dot,
+                    "(" => Piece::Open,
+                    ")" => Piece::Close,
+                    "," => Piece::Comma,
+                    _ => Piece::Other,
+                },
+                TokenKind::String
+                | TokenKind::Number
+                | TokenKind::ExecutableComment
+                | TokenKind::Operator
+                | TokenKind::Semicolon => Piece::Other,
+            })
+        })
+        .collect()
+}
+
+/// Whether `word` is one [`names`] reads as a keyword whether or not the
+/// tokenizer's list has it.
+fn read_as_keyword(word: &str) -> bool {
+    ENDS_FROM
+        .iter()
+        .chain(JOINS)
+        .chain(&[
+            "FROM",
+            "WITH",
+            "LATERAL",
+            "ONLY",
+            "AS",
+            "RECURSIVE",
+            "MATERIALIZED",
+        ])
+        .any(|known| known.eq_ignore_ascii_case(word))
+}
+
+/// A quoted name without its quotes, and with a doubled quote inside it
+/// read as one.
+fn unquoted(written: &str) -> String {
+    let (open, close) = match written.chars().next() {
+        Some('`') => ('`', '`'),
+        Some('[') => ('[', ']'),
+        _ => ('"', '"'),
+    };
+    let inner = written
+        .strip_prefix(open)
+        .unwrap_or(written)
+        .strip_suffix(close)
+        .unwrap_or(written);
+    let doubled: String = [close, close].iter().collect();
+    inner.replace(&doubled, &close.to_string())
+}
+
 #[cfg(test)]
 mod tests {
     use super::*;
````

- [ ] **Step 4: Run the tests**

Run: `cargo test --locked -p tabletist-db --lib agent::`

Expected: PASS, the checker's tests of Phase 1a and the new ones of `agent::hidden`.

```text
test result: ok. 51 passed; 0 failed; 0 ignored; 0 measured; 422 filtered out
```

- [ ] **Step 5: Commit**

```bash
cargo fmt --all
cargo clippy --locked --workspace --all-targets -- -D warnings
git add crates/tabletist-db/src/agent.rs crates/tabletist-db/src/agent/hidden.rs
git commit -m "Work out which tables an agent may not read, and the names a statement gives"
```

---

### Task 2: SQLite keeps an agent from what is hidden

- `Connection::run_agent_read` and the adapters' `run_read` take `tables: &agent::Tables`. The app passes `Tables::All` until the next run reads the setting, and so do the MCP crate's harness and the tests that are not about hiding. PostgreSQL and MySQL take it and leave it unused (`_tables`) until Tasks 3 and 4.
- On SQLite, where anything is hidden, the run reads the catalog (`pragma_table_list`, every schema, its own `sqlite_` tables left out) on the session's thread, refuses what the text names, and holds the `Hidden` in the fences while the statement is prepared and run. Behind `Fence::Agent` the authorizer asks it of every `SQLITE_READ`: a view's tables come with the view as their source, so the refusal names the view; `count(*)` and `EXISTS` read a table with an empty column, and are refused too. A lock that a panic poisoned still holds what is hidden.
- `tests/agent.rs` gains `a_hidden_table_is_never_read`, the hidden-table rows of F.2, and runs it on SQLite.

**Files:**
- Modify: `crates/tabletist-db/src/lib.rs`, `crates/tabletist-db/src/adapter.rs`, `crates/tabletist-db/src/sqlite.rs`, `crates/tabletist-db/src/sqlite/fence.rs`, `crates/tabletist-db/src/pg.rs`, `crates/tabletist-db/src/pg/script.rs`, `crates/tabletist-db/src/mysql.rs`, `crates/tabletist-db/src/mysql/script.rs`, `crates/tabletist-db/tests/agent.rs`, `crates/tabletist-mcp/tests/support/mod.rs`, `src/backend/agents.rs`

- [ ] **Step 1: Write the failing test**

````diff
diff --git a/crates/tabletist-db/src/mysql/script.rs b/crates/tabletist-db/src/mysql/script.rs
index 30d6bd6..f269ae8 100644
--- a/crates/tabletist-db/src/mysql/script.rs
+++ b/crates/tabletist-db/src/mysql/script.rs
@@ -642,7 +642,7 @@ mod tests {
             send(&conn.server_version());
         };
         let _agent = |conn: &Conn, read: &Read, limits: Limits, stop: &StopFlag| {
-            send(&conn.run_read(read, limits, stop));
+            send(&conn.run_read(read, limits, &crate::agent::Tables::All, stop));
         };
     }
 
diff --git a/crates/tabletist-db/src/pg/script.rs b/crates/tabletist-db/src/pg/script.rs
index fbd7e8b..7d3ae68 100644
--- a/crates/tabletist-db/src/pg/script.rs
+++ b/crates/tabletist-db/src/pg/script.rs
@@ -663,7 +663,7 @@ mod tests {
             send(&conn.server_version());
         };
         let _agent = |conn: &Conn, read: &Read, limits: Limits, stop: &StopFlag| {
-            send(&conn.run_read(read, limits, stop));
+            send(&conn.run_read(read, limits, &crate::agent::Tables::All, stop));
         };
     }
 
diff --git a/crates/tabletist-db/src/sqlite.rs b/crates/tabletist-db/src/sqlite.rs
index 4d43bed..db08f7d 100644
--- a/crates/tabletist-db/src/sqlite.rs
+++ b/crates/tabletist-db/src/sqlite.rs
@@ -2954,7 +2954,7 @@ mod tests {
             rows: 10,
             timeout: Duration::from_millis(250),
         };
-        conn.run_read(&read, limits, &StopFlag::new())
+        conn.run_read(&read, limits, &crate::agent::Tables::All, &StopFlag::new())
             .await
             .unwrap();
         assert_eq!(wait(&conn).await, Ok(5_000));
diff --git a/crates/tabletist-db/tests/agent.rs b/crates/tabletist-db/tests/agent.rs
index 5c8541e..eb16fce 100644
--- a/crates/tabletist-db/tests/agent.rs
+++ b/crates/tabletist-db/tests/agent.rs
@@ -9,7 +9,7 @@
 
 use std::time::{Duration, Instant};
 
-use tabletist_db::agent::{self, Answer, Limits, Param};
+use tabletist_db::agent::{self, Answer, Limits, Param, Refusal, TableName, Tables};
 use tabletist_db::{
     Access, ConnectSpec, Connection, Dialect, Error, HostKeys, ScriptMode, Secrets,
     StatementOutcome, StopFlag, Value, sql,
@@ -117,7 +117,7 @@ async fn asked(
     let read = agent::check_read(connection.dialect(), &sql, params)
         .unwrap_or_else(|refusal| panic!("{sql}: {refusal}"));
     connection
-        .run_agent_read(&read, limits, &StopFlag::new())
+        .run_agent_read(&read, limits, &Tables::All, &StopFlag::new())
         .await
 }
 
@@ -253,7 +253,9 @@ async fn a_read_that_is_stopped_or_fails_says_so(connection: &Connection, on: &s
     let stopped = StopFlag::new();
     stopped.stop();
     let books = agent::check_read(connection.dialect(), "SELECT title FROM books", &[]).unwrap();
-    let ended = connection.run_agent_read(&books, LIMITS, &stopped).await;
+    let ended = connection
+        .run_agent_read(&books, LIMITS, &Tables::All, &stopped)
+        .await;
     assert_eq!(ended.unwrap_err(), Error::Cancelled, "{on}");
     // The database's own error, in its words.
     let error = asked(connection, "SELECT nope FROM books", &[], LIMITS)
@@ -275,7 +277,7 @@ async fn a_read_checked_for_another_database_is_not_run(connection: &Connection,
     };
     let read = agent::check_read(other, "SELECT title FROM books", &[]).unwrap();
     let error = connection
-        .run_agent_read(&read, LIMITS, &StopFlag::new())
+        .run_agent_read(&read, LIMITS, &Tables::All, &StopFlag::new())
         .await
         .unwrap_err();
     assert_eq!(
@@ -285,6 +287,103 @@ async fn a_read_checked_for_another_database_is_not_run(connection: &Connection,
     );
 }
 
+/// A table the person names, in every schema.
+fn named(name: &str) -> TableName {
+    TableName {
+        schema: None,
+        name: name.to_owned(),
+    }
+}
+
+/// What the connection's database answers an agent that sends `sql` on a
+/// connection that hides as `tables` says: its rows, or why not.
+async fn hiding(connection: &Connection, tables: &Tables, sql: &str) -> Result<Answer, Error> {
+    let read = agent::check_read(connection.dialect(), sql, &[])
+        .unwrap_or_else(|refusal| panic!("{sql}: {refusal}"));
+    connection
+        .run_agent_read(&read, LIMITS, tables, &StopFlag::new())
+        .await
+}
+
+/// Refused as hidden: `name`, read for `through` where that is given.
+fn hidden_refusal(name: &str, through: Option<&str>) -> Error {
+    Error::Agent(Refusal::TableHidden {
+        name: name.to_owned(),
+        through: through.map(str::to_owned),
+    })
+}
+
+/// The hidden-table rows of the plan's F.2 (B.3), on the Bookshop.
+async fn a_hidden_table_is_never_read(connection: &Connection, on: &str) {
+    let customers = Tables::AllExcept(vec![named("customers")]);
+    let count = |answer: Result<Answer, Error>| answer.map(|answer| answer.rows);
+    // What is not hidden reads as before, views over it too.
+    for (sql, rows) in [
+        ("SELECT count(*) FROM books", 12),
+        ("SELECT count(*) FROM recent_orders", 7),
+    ] {
+        let found = count(hiding(connection, &customers, sql).await);
+        assert_eq!(found, Ok(vec![vec![Value::Int(rows)]]), "{on}: {sql}");
+    }
+    // The hidden table, by every way to it the text gives: by name, in
+    // another case and in quotes, counted, behind EXISTS, in a common
+    // table expression, and through a view.
+    let quoted = match connection.dialect() {
+        Dialect::MySql => "SELECT * FROM `CUSTOMERS`",
+        Dialect::Postgres | Dialect::Sqlite => "SELECT * FROM \"CUSTOMERS\"",
+    };
+    for sql in [
+        "SELECT * FROM customers",
+        "SELECT * FROM Customers c",
+        quoted,
+        "SELECT count(*) FROM customers",
+        "SELECT title FROM books WHERE EXISTS (SELECT 1 FROM customers)",
+        "WITH c AS (SELECT * FROM customers) SELECT * FROM c",
+        "WITH customers AS (SELECT 1 AS id) SELECT * FROM customers",
+    ] {
+        let error = hiding(connection, &customers, sql).await.unwrap_err();
+        assert_eq!(error, hidden_refusal("customers", None), "{on}: {sql}");
+    }
+    let through = hiding(connection, &customers, "SELECT * FROM customer_names").await;
+    assert_eq!(
+        through.unwrap_err(),
+        hidden_refusal("customers", Some("customer_names")),
+        "{on}"
+    );
+    // In a join in parentheses, where the name is not right after the
+    // FROM: read off the text, and by the database's own check besides.
+    let nested = "SELECT * FROM (customers JOIN books ON true)";
+    let error = hiding(connection, &customers, nested).await.unwrap_err();
+    assert!(
+        matches!(&error, Error::Agent(Refusal::TableHidden { name, .. }) if name == "customers"),
+        "{on}: {error:?}"
+    );
+    // The system catalogs describe every table: closed while one is
+    // hidden, open where none is.
+    let catalog = match connection.dialect() {
+        Dialect::Sqlite => "SELECT count(*) FROM sqlite_master",
+        Dialect::Postgres | Dialect::MySql => "SELECT count(*) FROM information_schema.tables",
+    };
+    let error = hiding(connection, &customers, catalog).await.unwrap_err();
+    assert_eq!(error, Error::Agent(Refusal::SystemCatalog), "{on}");
+    assert!(
+        hiding(connection, &Tables::All, catalog).await.is_ok(),
+        "{on}"
+    );
+    // Only a list: the rest is hidden.
+    let only_books = Tables::Only(vec![named("books")]);
+    let found = count(hiding(connection, &only_books, "SELECT count(*) FROM books").await);
+    assert_eq!(found, Ok(vec![vec![Value::Int(12)]]), "{on}");
+    let error = hiding(connection, &only_books, "SELECT * FROM orders")
+        .await
+        .unwrap_err();
+    assert_eq!(error, hidden_refusal("orders", None), "{on}");
+    // Nothing hidden: the table reads.
+    let found = count(hiding(connection, &Tables::All, "SELECT count(*) FROM customers").await);
+    assert_eq!(found, Ok(vec![vec![Value::Int(8)]]), "{on}");
+    the_bookshop_is_stocked(connection, on).await;
+}
+
 /// What every engine is asked.
 async fn an_agent_reads_the_bookshop(connection: &Connection, on: &str) {
     a_read_answers_its_rows(connection, on).await;
@@ -335,6 +434,12 @@ mod sqlite {
         an_agent_reads_the_bookshop(&connection, "sqlite").await;
     }
 
+    #[tokio::test]
+    async fn a_hidden_table_is_never_read_on_sqlite() {
+        let (connection, _dir) = bookshop().await;
+        a_hidden_table_is_never_read(&connection, "sqlite").await;
+    }
+
     #[tokio::test]
     async fn sqlite_takes_each_value_as_its_own_type() {
         let (connection, _dir) = bookshop().await;
@@ -639,7 +744,7 @@ mod postgres {
         let twice = "SELECT title FROM books WHERE id = $1 OR author_id = $1 ORDER BY id";
         let read_twice = agent::check_read(Dialect::Postgres, twice, &[Param::Int(3)]).unwrap();
         let found = connection
-            .run_agent_read(&read_twice, LIMITS, &StopFlag::new())
+            .run_agent_read(&read_twice, LIMITS, &Tables::All, &StopFlag::new())
             .await
             .unwrap();
         assert_eq!(found.rows, [[text("Lanterns")], [text("Ninety-One Doors")]]);
@@ -668,7 +773,7 @@ mod postgres {
         let sql = "SELECT '$1', $q$ $1 $q$, title FROM books WHERE id = $1";
         let read = agent::check_read(Dialect::Postgres, sql, &[Param::Int(3)]).unwrap();
         let found = connection
-            .run_agent_read(&read, LIMITS, &StopFlag::new())
+            .run_agent_read(&read, LIMITS, &Tables::All, &StopFlag::new())
             .await
             .unwrap();
         assert_eq!(found.rows, [[text("$1"), text(" $1 "), text("Lanterns")]]);
diff --git a/crates/tabletist-mcp/tests/support/mod.rs b/crates/tabletist-mcp/tests/support/mod.rs
index 302330e..58bdac8 100644
--- a/crates/tabletist-mcp/tests/support/mod.rs
+++ b/crates/tabletist-mcp/tests/support/mod.rs
@@ -210,7 +210,9 @@ impl Session for ShopSession {
             },
         };
         let flag = StopFlag::new();
-        let run = self.connection.run_agent_read(read, limits, &flag);
+        let run =
+            self.connection
+                .run_agent_read(read, limits, &tabletist_db::agent::Tables::All, &flag);
         tokio::pin!(run);
         let answer = tokio::select! {
             answer = &mut run => answer,
````

- [ ] **Step 2: Run it to see that it does not build**

Run: `cargo test --locked -p tabletist-db --test agent sqlite::a_hidden_table_is_never_read_on_sqlite`

Expected: it does not build, as `run_agent_read` does not take what is hidden yet:

```text
error[E0061]: this method takes 3 arguments but 4 arguments were supplied
error: could not compile `tabletist-db` (test "agent") due to 6 previous errors
```

- [ ] **Step 3: The argument, and SQLite's check**

````diff
diff --git a/crates/tabletist-db/src/adapter.rs b/crates/tabletist-db/src/adapter.rs
index 7e11d1c..ff28332 100644
--- a/crates/tabletist-db/src/adapter.rs
+++ b/crates/tabletist-db/src/adapter.rs
@@ -1,6 +1,6 @@
 //! What the adapter of every database provides.
 
-use crate::agent::{Limits, Read};
+use crate::agent::{Limits, Read, Tables};
 use crate::{
     CancelHandle, ChangeSet, Lookup, LookupPage, ObjectInfo, ObjectRef, Result, RowPage, RowQuery,
     ScriptMode, ScriptOutcome, StopFlag, Structure, WriteOutcome,
@@ -43,9 +43,15 @@ pub(crate) trait Adapter {
     ) -> Result<ScriptOutcome>;
 
     /// `read` has passed [`crate::agent::check_read`] for this database's
-    /// dialect. The outcome is a read-only script's of one statement.
-    async fn run_read(&self, read: &Read, limits: Limits, stop: &StopFlag)
-    -> Result<ScriptOutcome>;
+    /// dialect. The outcome is a read-only script's of one statement, or
+    /// [`crate::Error::Agent`] where it reads what `tables` hides.
+    async fn run_read(
+        &self,
+        read: &Read,
+        limits: Limits,
+        tables: &Tables,
+        stop: &StopFlag,
+    ) -> Result<ScriptOutcome>;
 
     /// `changes` have passed their own check, and the session may write.
     async fn write(&self, changes: &ChangeSet, stop: &StopFlag) -> Result<WriteOutcome>;
diff --git a/crates/tabletist-db/src/lib.rs b/crates/tabletist-db/src/lib.rs
index c8750dd..42423e2 100644
--- a/crates/tabletist-db/src/lib.rs
+++ b/crates/tabletist-db/src/lib.rs
@@ -244,11 +244,19 @@ impl Connection {
     /// [`Error::Cancelled`]. A `read` that was checked for another
     /// dialect than this session's is refused, with nothing sent.
     ///
+    /// Where `tables` hides anything, the read is refused
+    /// ([`Error::Agent`]) when its text names what is hidden, or when the
+    /// database says it would read it: SQLite's authorizer as it prepares
+    /// the statement, and the statement's plan on PostgreSQL and MySQL,
+    /// asked for in the run's own transaction. Nothing hidden is read
+    /// (the MCP plan's B.3).
+    ///
     /// The future must be awaited to its end, as [`Self::run_script`]'s.
     pub async fn run_agent_read(
         &self,
         read: &agent::Read,
         limits: agent::Limits,
+        tables: &agent::Tables,
         stop: &StopFlag,
     ) -> Result<agent::Answer> {
         if read.dialect() != self.dialect() {
@@ -257,9 +265,9 @@ impl Connection {
             ));
         }
         let outcome = match &self.inner {
-            Inner::Sqlite(conn) => conn.run_read(read, limits, stop).await,
-            Inner::Postgres(conn) => conn.run_read(read, limits, stop).await,
-            Inner::MySql(conn) => conn.run_read(read, limits, stop).await,
+            Inner::Sqlite(conn) => conn.run_read(read, limits, tables, stop).await,
+            Inner::Postgres(conn) => conn.run_read(read, limits, tables, stop).await,
+            Inner::MySql(conn) => conn.run_read(read, limits, tables, stop).await,
         }?;
         agent::answer(outcome, stop)
     }
diff --git a/crates/tabletist-db/src/mysql.rs b/crates/tabletist-db/src/mysql.rs
index 29d1fca..1fb0922 100644
--- a/crates/tabletist-db/src/mysql.rs
+++ b/crates/tabletist-db/src/mysql.rs
@@ -20,7 +20,7 @@ use mysql_async::{DriverError, IoError, Opts, OptsBuilder, Params, SslOpts, TxOp
 use mysql_common::named_params::ParsedNamedParams;
 
 use crate::adapter::Adapter;
-use crate::agent::{Limits, Read};
+use crate::agent::{Limits, Read, Tables};
 use crate::script::retry_cancelled;
 use crate::{
     Access, CancelHandle, CancelInner, ChangeSet, ColumnClass, ColumnInfo, ColumnMeta, ConnectSpec,
@@ -516,6 +516,7 @@ impl Adapter for Conn {
         &self,
         read: &Read,
         limits: Limits,
+        _tables: &Tables,
         stop: &StopFlag,
     ) -> Result<ScriptOutcome> {
         self.agent_read(read, limits, stop).await
diff --git a/crates/tabletist-db/src/pg.rs b/crates/tabletist-db/src/pg.rs
index 20d190e..6fb2719 100644
--- a/crates/tabletist-db/src/pg.rs
+++ b/crates/tabletist-db/src/pg.rs
@@ -12,7 +12,7 @@ use tokio_postgres::tls::{MakeTlsConnect, TlsConnect};
 use tokio_postgres::{SimpleQueryMessage, Socket};
 
 use crate::adapter::Adapter;
-use crate::agent::{Limits, Read};
+use crate::agent::{Limits, Read, Tables};
 use crate::{
     Access, CancelHandle, CancelInner, ChangeSet, ColumnInfo, ColumnMeta, ConnectSpec, Dialect,
     Error, ForeignKeyInfo, IndexInfo, Lookup, LookupPage, MAX_LISTED, Named, ObjectInfo,
@@ -831,6 +831,7 @@ impl Adapter for Conn {
         &self,
         read: &Read,
         limits: Limits,
+        _tables: &Tables,
         stop: &StopFlag,
     ) -> Result<ScriptOutcome> {
         self.agent_read(read, limits, stop).await
diff --git a/crates/tabletist-db/src/sqlite.rs b/crates/tabletist-db/src/sqlite.rs
index db08f7d..8e21937 100644
--- a/crates/tabletist-db/src/sqlite.rs
+++ b/crates/tabletist-db/src/sqlite.rs
@@ -13,7 +13,7 @@ use rusqlite::types::{FromSql, FromSqlError, FromSqlResult, ValueRef};
 use rusqlite::{ErrorCode, OpenFlags, OptionalExtension};
 
 use crate::adapter::Adapter;
-use crate::agent::{Limits, Param, Read, Sent, count_mismatch};
+use crate::agent::{Catalog, Hidden, Limits, Param, Read, Relation, Sent, Tables, count_mismatch};
 use crate::script::{cancelled_commit, cleanup_failed};
 use crate::{
     Access, CancelHandle, CancelInner, ChangeSet, ColumnClass, ColumnInfo, ColumnMeta, Dialect,
@@ -1144,6 +1144,7 @@ impl Adapter for Conn {
         &self,
         read: &Read,
         limits: Limits,
+        tables: &Tables,
         stop: &StopFlag,
     ) -> Result<ScriptOutcome> {
         let stop = stop.clone();
@@ -1151,8 +1152,21 @@ impl Adapter for Conn {
         let fences = self.fences.clone();
         let texts = vec![read.text().to_owned()];
         let params = read.params().to_vec();
+        let (read, tables) = (read.clone(), tables.clone());
         let outcome = self
             .run(move |connection| {
+                // What is hidden, from the catalog as it is now: a table
+                // made a moment ago is in it.
+                let hidden = match tables.hides_anything() {
+                    true => {
+                        let catalog = agents_catalog(connection)?;
+                        let hidden = Hidden::new(Dialect::Sqlite, &catalog, &tables);
+                        hidden.check_statement(&read).map_err(Error::Agent)?;
+                        Some(Arc::new(hidden))
+                    }
+                    false => None,
+                };
+                let _hiding = fences.hide(hidden);
                 let sent = Sent {
                     params: &params,
                     timeout: limits.timeout,
@@ -1173,6 +1187,35 @@ impl Adapter for Conn {
     }
 }
 
+/// The relations of every schema of the file, its own `sqlite_` tables
+/// left out, for what an agent's connection hides. SQLite's authorizer
+/// reports a view's tables as it reads them, so what a view depends on is
+/// not read here.
+fn agents_catalog(connection: &rusqlite::Connection) -> Result<Catalog> {
+    let mut statement = connection
+        .prepare(
+            "SELECT schema, name FROM pragma_table_list \
+             WHERE type IN ('table', 'view', 'virtual') AND name NOT LIKE 'sqlite\\_%' ESCAPE '\\'",
+        )
+        .map_err(map_error)?;
+    let relations = statement
+        .query_map([], |row| {
+            Ok(Relation {
+                schema: row.get(0)?,
+                name: row.get(1)?,
+                depends_on: Vec::new(),
+                opaque: false,
+            })
+        })
+        .map_err(map_error)?
+        .collect::<rusqlite::Result<Vec<_>>>()
+        .map_err(map_error)?;
+    Ok(Catalog {
+        relations,
+        routines: Vec::new(),
+    })
+}
+
 fn columns(connection: &rusqlite::Connection, object: &ObjectRef) -> Result<Vec<ColumnInfo>> {
     // A table WITHOUT ROWID has no rowid for a column to be the alias of.
     let listed = connection
diff --git a/crates/tabletist-db/src/sqlite/fence.rs b/crates/tabletist-db/src/sqlite/fence.rs
index 4c0d510..f852240 100644
--- a/crates/tabletist-db/src/sqlite/fence.rs
+++ b/crates/tabletist-db/src/sqlite/fence.rs
@@ -14,13 +14,13 @@
 //! name that SQLite knows.
 
 use std::sync::atomic::{AtomicU8, Ordering};
-use std::sync::{Arc, Mutex};
+use std::sync::{Arc, Mutex, PoisonError};
 
 use rusqlite::ffi;
 use tabletist_sqlite_ffi::{Action, Authorization};
 
 use crate::Dialect;
-use crate::agent::{Refusal, function_allowed};
+use crate::agent::{Hidden, Refusal, function_allowed};
 
 /// Whose text SQLite is preparing, which decides what it may do.
 #[derive(Debug, Clone, Copy, PartialEq, Eq)]
@@ -156,6 +156,8 @@ pub(super) struct Fences(Arc<Shared>);
 struct Shared {
     up: AtomicU8,
     refused: Mutex<Option<Refusal>>,
+    /// What the agent's connection hides, while its statement runs.
+    hidden: Mutex<Option<Arc<Hidden>>>,
 }
 
 impl Fences {
@@ -187,18 +189,54 @@ impl Fences {
     /// The authorizer's answer for the fence that is up. SQLite says only
     /// "not authorized" of a statement it refuses, so what an agent's
     /// fence denies is kept, the first of it, for the run to say.
+    ///
+    /// Behind an agent's fence a read of a table is also asked of what the
+    /// connection hides. SQLite reports every relation a statement reads,
+    /// after its own parse: a view's own name and then its tables', each
+    /// with the view it is read for, a table that only `count(*)` or
+    /// `EXISTS` reads (with an empty column), and its own `sqlite_schema`.
+    /// So nothing hidden is read, however the text was written.
     pub(super) fn ask(&self, action: &Action<'_>) -> Authorization {
         let fence = self.current();
-        let answer = authorize(fence, action);
+        let mut answer = authorize(fence, action);
+        let mut hidden_read = None;
+        if fence == Fence::Agent
+            && answer == Authorization::Allow
+            && action.code == ffi::SQLITE_READ
+            && let Some(table) = action.first
+            && let Some(hidden) = self.hidden()
+            && let Err(refusal) = hidden.check_engine_read(action.database, table)
+        {
+            answer = Authorization::Deny;
+            hidden_read = Some(through_view(refusal, action.source));
+        }
         if fence == Fence::Agent
             && answer == Authorization::Deny
             && let Ok(mut refused) = self.0.refused.lock()
         {
-            refused.get_or_insert_with(|| refusal(action));
+            refused.get_or_insert_with(|| hidden_read.unwrap_or_else(|| refusal(action)));
         }
         answer
     }
 
+    /// Hides from an agent's statements what `hidden` hides, until the
+    /// returned value drops.
+    pub(super) fn hide(&self, hidden: Option<Arc<Hidden>>) -> Hiding<'_> {
+        *self.held() = hidden;
+        Hiding(self)
+    }
+
+    /// What is hidden now.
+    fn hidden(&self) -> Option<Arc<Hidden>> {
+        self.held().clone()
+    }
+
+    /// What is hidden, held. A lock that a panic left behind still holds
+    /// it: what is hidden is never let go of by an error.
+    fn held(&self) -> std::sync::MutexGuard<'_, Option<Arc<Hidden>>> {
+        self.0.hidden.lock().unwrap_or_else(PoisonError::into_inner)
+    }
+
     /// What an agent's fence denied since the last fence went up.
     pub(super) fn refused(&self) -> Option<Refusal> {
         self.0
@@ -209,6 +247,34 @@ impl Fences {
     }
 }
 
+/// A hidden table read for a view the statement named: the view is named
+/// with it, as the agent asked for the view. A common table expression is
+/// such a source too, and is named the same way.
+fn through_view(refusal: Refusal, source: Option<&str>) -> Refusal {
+    match (refusal, source) {
+        (
+            Refusal::TableHidden {
+                name,
+                through: None,
+            },
+            Some(view),
+        ) if !view.eq_ignore_ascii_case(&name) => Refusal::TableHidden {
+            name,
+            through: Some(view.to_owned()),
+        },
+        (refusal, _) => refusal,
+    }
+}
+
+/// Lets go of what is hidden when dropped, on every path.
+pub(super) struct Hiding<'a>(&'a Fences);
+
+impl Drop for Hiding<'_> {
+    fn drop(&mut self) {
+        *self.0.held() = None;
+    }
+}
+
 /// Takes the fence down when dropped, on every path.
 pub(super) struct Fenced<'a>(&'a Fences);
 
diff --git a/src/backend/agents.rs b/src/backend/agents.rs
index 986f136..198e60a 100644
--- a/src/backend/agents.rs
+++ b/src/backend/agents.rs
@@ -903,8 +903,14 @@ impl Session for AppSession {
     async fn read(&self, read: &Read, limits: Limits) -> Result<Answer, Failure> {
         let connection = self.connection()?;
         let flag = StopFlag::new();
-        self.ended(Some(&flag), connection.run_agent_read(read, limits, &flag))
-            .await
+        // What the connection hides is the next run's (1c-2): until it
+        // is set there, every table is read as before.
+        let tables = tabletist_db::agent::Tables::All;
+        self.ended(
+            Some(&flag),
+            connection.run_agent_read(read, limits, &tables, &flag),
+        )
+        .await
     }
 }
 
````

- [ ] **Step 4: Run the tests**

Run: `cargo test --locked -p tabletist-db --test agent` (with both servers), then `cargo test --locked -p tabletist-db --lib sqlite::`

Expected: PASS.

```text
test result: ok. 25 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
test result: ok. 53 passed; 0 failed; 0 ignored; 0 measured; 420 filtered out
```

- [ ] **Step 5: Commit**

```bash
cargo fmt --all
cargo clippy --locked --workspace --all-targets -- -D warnings
git add crates/tabletist-db crates/tabletist-mcp/tests/support/mod.rs src/backend/agents.rs
git commit -m "Keep an agent from the tables a SQLite connection hides"
```

---

### Task 3: PostgreSQL, by the statement's plan

- `pg/hidden.rs` reads the catalog in the run's read-only transaction: every relation outside the system's schemas (tables, views, materialized views, partitioned and foreign tables) with what its rewrite rules read and its parent (`pg_inherits`), and the users' functions (`pg_proc`, without an extension's). It refuses what the text names, then asks `EXPLAIN (VERBOSE, FORMAT JSON)` of the statement with the agent's values bound, and checks every `Relation Name` with its `Schema`, and every `Function Name` outside the system's schemas. A plan that cannot be read refuses (`not_parsed`); the server's own error on the plan is the answer, as the statement's would have been.
- `pg/script.rs` checks before the statement runs. A refusal leaves the transaction open, and it is rolled back as every agent's read is; an error leaves it failed; a lost session ends the run.
- `agent::planned` gives the query an `EXPLAIN` explains, so an agent's own `EXPLAIN` is checked as its query.
- The Bookshop on PostgreSQL gains `customer_count()`, which reads `customers` (F's function; MySQL's fixture has none, as the comment says).
- `serde_json` moves to the crate's dependencies: the plan is JSON.

**Files:**
- Create: `crates/tabletist-db/src/pg/hidden.rs`
- Modify: `crates/tabletist-db/src/pg.rs`, `crates/tabletist-db/src/pg/script.rs`, `crates/tabletist-db/src/agent.rs`, `crates/tabletist-db/src/agent/hidden.rs`, `crates/tabletist-db/Cargo.toml`, `crates/tabletist-db/fixtures/bookshop/postgres.sql`, `crates/tabletist-db/tests/agent.rs`

- [ ] **Step 1: Write the failing test**

The test on PostgreSQL, the function that reads `customers`, and the unit tests of what comes in this task.

````diff
diff --git a/crates/tabletist-db/Cargo.toml b/crates/tabletist-db/Cargo.toml
index f83a4d9..447aa77 100644
--- a/crates/tabletist-db/Cargo.toml
+++ b/crates/tabletist-db/Cargo.toml
@@ -63,7 +63,11 @@ mysql_async = { version = "0.37", default-features = false, features = ["default
 # run. Moves with mysql_async; mysql.rs fails to compile if the two drift apart.
 mysql_common = { version = "0.37", default-features = false }
 
+# The plans PostgreSQL and MySQL give in JSON, read for the tables an
+# agent's statement would scan (`agent`'s hidden tables). Already in the
+# tree through the app and mysql_async.
+serde_json = "1"
+
 [dev-dependencies]
 tokio = { version = "1", features = ["rt-multi-thread", "macros", "time", "sync", "net"] }
 tempfile = "3"
-serde_json = "1"
diff --git a/crates/tabletist-db/fixtures/bookshop/postgres.sql b/crates/tabletist-db/fixtures/bookshop/postgres.sql
index 38fc705..cce9bc4 100644
--- a/crates/tabletist-db/fixtures/bookshop/postgres.sql
+++ b/crates/tabletist-db/fixtures/bookshop/postgres.sql
@@ -3,6 +3,7 @@
 -- engine. The three files hold the same rows. Keep them deterministic.
 -- Loaded into a database of its own, `bookshop`, which the tests make.
 
+DROP FUNCTION IF EXISTS customer_count();
 DROP VIEW IF EXISTS recent_orders, customer_names;
 DROP TABLE IF EXISTS order_items, orders, customers, book_covers, books, authors, publishers;
 
@@ -157,3 +158,9 @@ INSERT INTO order_items (id, order_id, book_id, quantity, price) VALUES
 CREATE VIEW recent_orders AS
     SELECT id, email, placed_at, total FROM orders WHERE placed_at >= '2026-01-01';
 CREATE VIEW customer_names AS SELECT id, name FROM customers;
+
+-- And a third, a function of the database's own: what it reads, no check
+-- can see. Here alone: SQLite has none to write, and MySQL, which logs its
+-- changes, lets only a user with SUPER write one.
+CREATE FUNCTION customer_count() RETURNS bigint LANGUAGE sql STABLE
+    AS 'SELECT count(*) FROM customers';
diff --git a/crates/tabletist-db/src/agent/hidden.rs b/crates/tabletist-db/src/agent/hidden.rs
index b460e6d..a0ad467 100644
--- a/crates/tabletist-db/src/agent/hidden.rs
+++ b/crates/tabletist-db/src/agent/hidden.rs
@@ -980,6 +980,34 @@ mod tests {
         );
     }
 
+    #[test]
+    fn an_explain_is_planned_as_the_query_it_explains() {
+        let planned_of = |dialect, sql| planned(&checked(dialect, sql)).to_owned();
+        assert_eq!(
+            planned_of(Dialect::Postgres, "SELECT * FROM books"),
+            "SELECT * FROM books"
+        );
+        assert_eq!(
+            planned_of(
+                Dialect::Postgres,
+                "EXPLAIN (VERBOSE, FORMAT JSON) WITH x AS (SELECT 1) SELECT * FROM x"
+            ),
+            "WITH x AS (SELECT 1) SELECT * FROM x"
+        );
+        assert_eq!(
+            planned_of(Dialect::Postgres, "EXPLAIN VERBOSE SELECT 1"),
+            "SELECT 1"
+        );
+        assert_eq!(
+            planned_of(Dialect::MySql, "EXPLAIN FORMAT=JSON SELECT 1"),
+            "SELECT 1"
+        );
+        assert_eq!(
+            planned_of(Dialect::Sqlite, "EXPLAIN QUERY PLAN SELECT 1"),
+            "SELECT 1"
+        );
+    }
+
     #[test]
     fn nothing_is_hidden_by_an_empty_list_or_by_all() {
         for tables in [Tables::All, Tables::AllExcept(Vec::new())] {
diff --git a/crates/tabletist-db/src/pg/hidden.rs b/crates/tabletist-db/src/pg/hidden.rs
new file mode 100644
index 0000000..36f3e68
--- /dev/null
+++ b/crates/tabletist-db/src/pg/hidden.rs
@@ -0,0 +1,100 @@
+//! What an agent may not read on PostgreSQL, asked of the server in the
+//! run's own transaction, before the statement runs (the MCP plan's B.3):
+//! the catalog as it is at that moment, and the statement's plan.
+//!
+//! The plan is the fence. `EXPLAIN (VERBOSE, FORMAT JSON)` names every
+//! relation the statement would scan, with its schema, after the server
+//! has expanded its views, found its partitions and its child tables, and
+//! resolved every name by the session's own search path. What the plan
+//! cannot name (a function's body) is the catalog's: the functions the
+//! database's users wrote are refused while anything is hidden.
+
+#[cfg(test)]
+mod tests {
+    use super::*;
+    use crate::agent::TableName;
+
+    /// A plan shaped as PostgreSQL 17's for `SELECT * FROM customers JOIN
+    /// events ON …`: the parent's scan names its child table too, and a
+    /// partitioned table is named by the partition it scans.
+    fn plan() -> serde_json::Value {
+        serde_json::json!([{ "Plan": {
+            "Node Type": "Hash Join",
+            "Plans": [
+                { "Node Type": "Append", "Plans": [
+                    { "Node Type": "Seq Scan", "Relation Name": "customers", "Schema": "public", "Alias": "customers_1" },
+                    { "Node Type": "Index Only Scan", "Relation Name": "vip", "Schema": "public", "Alias": "customers_2", "Index Name": "vip_pkey" }
+                ] },
+                { "Node Type": "Hash", "Plans": [
+                    { "Node Type": "Bitmap Heap Scan", "Relation Name": "events_2026", "Schema": "public", "Alias": "events",
+                      "Plans": [{ "Node Type": "Bitmap Index Scan", "Index Name": "events_2026_at" }] }
+                ] }
+            ]
+        } }])
+    }
+
+    fn hidden(tables: Tables) -> Hidden {
+        let relation = |name: &str, on: &[&str]| Relation {
+            schema: "public".into(),
+            name: name.into(),
+            depends_on: on
+                .iter()
+                .map(|on| ("public".into(), (*on).into()))
+                .collect(),
+            opaque: false,
+        };
+        let catalog = Catalog {
+            relations: vec![
+                relation("customers", &[]),
+                relation("vip", &["customers"]),
+                relation("events", &[]),
+                relation("events_2026", &["events"]),
+            ],
+            routines: vec![("public".into(), "names".into())],
+        };
+        Hidden::new(Dialect::Postgres, &catalog, &tables)
+    }
+
+    fn except(name: &str) -> Tables {
+        Tables::AllExcept(vec![TableName {
+            schema: None,
+            name: name.into(),
+        }])
+    }
+
+    #[test]
+    fn every_scan_the_plan_names_is_checked() {
+        assert_eq!(planned_reads(&plan(), &hidden(except("books"))), Ok(()));
+        // A child table, by its scan under the parent's.
+        assert_eq!(
+            planned_reads(&plan(), &hidden(except("vip"))),
+            Err(Refusal::TableHidden {
+                name: "vip".into(),
+                through: None
+            })
+        );
+        // A partition of a hidden table.
+        assert_eq!(
+            planned_reads(&plan(), &hidden(except("events"))),
+            Err(Refusal::TableHidden {
+                name: "events".into(),
+                through: Some("events_2026".into())
+            })
+        );
+        // A function in a FROM, the users' own; and the system's catalog.
+        let function = serde_json::json!({
+            "Node Type": "Function Scan", "Function Name": "names", "Schema": "public"
+        });
+        assert_eq!(
+            planned_reads(&function, &hidden(except("vip"))),
+            Err(Refusal::FunctionNotAllowed("names".into()))
+        );
+        let catalog = serde_json::json!({
+            "Node Type": "Seq Scan", "Relation Name": "pg_class", "Schema": "pg_catalog"
+        });
+        assert_eq!(
+            planned_reads(&catalog, &hidden(except("vip"))),
+            Err(Refusal::SystemCatalog)
+        );
+    }
+}
diff --git a/crates/tabletist-db/tests/agent.rs b/crates/tabletist-db/tests/agent.rs
index eb16fce..d6f574d 100644
--- a/crates/tabletist-db/tests/agent.rs
+++ b/crates/tabletist-db/tests/agent.rs
@@ -378,6 +378,21 @@ async fn a_hidden_table_is_never_read(connection: &Connection, on: &str) {
         .await
         .unwrap_err();
     assert_eq!(error, hidden_refusal("orders", None), "{on}");
+    // A function of the database's own, which reads what no check sees:
+    // refused while anything is hidden. The Bookshop has one on PostgreSQL
+    // alone (its fixture says why).
+    if connection.dialect() == Dialect::Postgres {
+        for sql in [
+            "SELECT customer_count()",
+            "SELECT title FROM books WHERE id < customer_count()",
+        ] {
+            let error = hiding(connection, &customers, sql).await.unwrap_err();
+            let refused = Error::Agent(Refusal::FunctionNotAllowed("customer_count".into()));
+            assert_eq!(error, refused, "{on}: {sql}");
+        }
+        let found = count(hiding(connection, &Tables::All, "SELECT customer_count()").await);
+        assert_eq!(found, Ok(vec![vec![Value::Int(8)]]), "{on}");
+    }
     // Nothing hidden: the table reads.
     let found = count(hiding(connection, &Tables::All, "SELECT count(*) FROM customers").await);
     assert_eq!(found, Ok(vec![vec![Value::Int(8)]]), "{on}");
@@ -712,6 +727,14 @@ mod postgres {
         an_agent_reads_the_bookshop(&connection, "postgres").await;
     }
 
+    #[tokio::test]
+    async fn a_hidden_table_is_never_read_on_postgres() {
+        let Some(connection) = bookshop().await else {
+            return;
+        };
+        a_hidden_table_is_never_read(&connection, "postgres").await;
+    }
+
     #[tokio::test]
     async fn postgres_reads_a_value_as_the_type_of_its_placeholder() {
         let Some(connection) = bookshop().await else {
````

- [ ] **Step 2: Run it to see it fail**

Run: `cargo test --locked -p tabletist-db --test agent postgres::a_hidden_table_is_never_read_on_postgres`, with `TABLETIST_TEST_PG_URL` set

Expected: FAIL. PostgreSQL reads the hidden table, so the first refusal the test asks for is an answer:

```text
test postgres::a_hidden_table_is_never_read_on_postgres ... FAILED
thread 'postgres::a_hidden_table_is_never_read_on_postgres' panicked at crates/tabletist-db/tests/agent.rs:344:63:
called `Result::unwrap_err()` on an `Ok` value: Answer { … }
```

- [ ] **Step 3: The catalog, the plan, and the check in the transaction**

````diff
diff --git a/crates/tabletist-db/src/agent.rs b/crates/tabletist-db/src/agent.rs
index d5939d5..649145d 100644
--- a/crates/tabletist-db/src/agent.rs
+++ b/crates/tabletist-db/src/agent.rs
@@ -31,6 +31,7 @@ use crate::{Dialect, sql};
 mod hidden;
 mod run;
 
+pub(crate) use hidden::planned;
 pub use hidden::{Catalog, Hidden, Relation, TableName, Tables};
 pub use run::{Answer, Limits};
 pub(crate) use run::{Sent, answer, count_mismatch};
diff --git a/crates/tabletist-db/src/agent/hidden.rs b/crates/tabletist-db/src/agent/hidden.rs
index a0ad467..39ce047 100644
--- a/crates/tabletist-db/src/agent/hidden.rs
+++ b/crates/tabletist-db/src/agent/hidden.rs
@@ -370,6 +370,48 @@ fn system_catalog(dialect: Dialect, schema: Option<&str>, name: &str) -> bool {
     }
 }
 
+/// The query a statement has the database plan: the statement itself, or
+/// the one an `EXPLAIN` explains, from its first word that begins a query
+/// (past `EXPLAIN`'s options in parentheses, `VERBOSE`, MySQL's
+/// `FORMAT=…`). An `EXPLAIN` is planned so too: what its plan names, the
+/// engine reads to make it, statistics included.
+pub(crate) fn planned(read: &Read) -> &str {
+    let text = read.text();
+    if !read.explains() {
+        return text;
+    }
+    let tokens = sql::tokenize(read.dialect(), text);
+    let mut depth = 0_usize;
+    for token in &tokens {
+        let written = &text[token.range.clone()];
+        match (token.kind, written) {
+            (TokenKind::Punctuation, "(") if depth > 0 => depth += 1,
+            (TokenKind::Punctuation, "(") => {
+                // `EXPLAIN (…)` is options; `EXPLAIN (SELECT …)` a query.
+                let after = &text[token.range.end..];
+                let next = sql::words(read.dialect(), after);
+                match next.first().map(String::as_str) {
+                    Some("SELECT" | "WITH" | "VALUES" | "TABLE") => {
+                        return &text[token.range.start..];
+                    }
+                    _ => depth = 1,
+                }
+            }
+            (TokenKind::Punctuation, ")") => depth = depth.saturating_sub(1),
+            (TokenKind::Keyword | TokenKind::Identifier, word)
+                if depth == 0
+                    && ["SELECT", "WITH", "VALUES", "TABLE"]
+                        .iter()
+                        .any(|start| start.eq_ignore_ascii_case(word)) =>
+            {
+                return &text[token.range.start..];
+            }
+            _ => {}
+        }
+    }
+    text
+}
+
 /// `name` as names are compared.
 fn fold(name: &str) -> String {
     name.to_lowercase()
diff --git a/crates/tabletist-db/src/pg.rs b/crates/tabletist-db/src/pg.rs
index 6fb2719..0622144 100644
--- a/crates/tabletist-db/src/pg.rs
+++ b/crates/tabletist-db/src/pg.rs
@@ -22,6 +22,7 @@ use crate::{
 use tokio_postgres::error::SqlState;
 use tokio_postgres_rustls::MakeRustlsConnect;
 
+mod hidden;
 mod script;
 mod write;
 
@@ -831,10 +832,10 @@ impl Adapter for Conn {
         &self,
         read: &Read,
         limits: Limits,
-        _tables: &Tables,
+        tables: &Tables,
         stop: &StopFlag,
     ) -> Result<ScriptOutcome> {
-        self.agent_read(read, limits, stop).await
+        self.agent_read(read, limits, tables, stop).await
     }
 
     async fn write(&self, changes: &ChangeSet, stop: &StopFlag) -> Result<WriteOutcome> {
diff --git a/crates/tabletist-db/src/pg/hidden.rs b/crates/tabletist-db/src/pg/hidden.rs
index 36f3e68..30da29d 100644
--- a/crates/tabletist-db/src/pg/hidden.rs
+++ b/crates/tabletist-db/src/pg/hidden.rs
@@ -9,6 +9,165 @@
 //! cannot name (a function's body) is the catalog's: the functions the
 //! database's users wrote are refused while anything is hidden.
 
+use tokio_postgres::SimpleQueryMessage;
+
+use super::query_error;
+use crate::agent::{Catalog, Hidden, Read, Refusal, Relation, Tables, planned};
+use crate::{Dialect, Error, Result};
+
+/// Every relation of the database but the system's, each with what it
+/// depends on: the relations a view's or a materialized view's rules
+/// read, and a partition's or a child table's parent. One row for each
+/// relation and each thing it depends on.
+const RELATIONS: &str = "\
+WITH depends AS (
+    SELECT DISTINCT r.ev_class AS relation, d.refobjid AS depends_on
+    FROM pg_rewrite r
+    JOIN pg_depend d ON d.classid = 'pg_rewrite'::regclass AND d.objid = r.oid
+        AND d.refclassid = 'pg_class'::regclass AND d.refobjid <> r.ev_class
+    UNION
+    SELECT inhrelid, inhparent FROM pg_inherits
+)
+SELECT n.nspname, c.relname, dn.nspname, dc.relname
+FROM pg_class c
+JOIN pg_namespace n ON n.oid = c.relnamespace
+LEFT JOIN depends ON depends.relation = c.oid
+LEFT JOIN pg_class dc ON dc.oid = depends.depends_on
+LEFT JOIN pg_namespace dn ON dn.oid = dc.relnamespace
+WHERE c.relkind IN ('r', 'v', 'm', 'p', 'f')
+    AND n.nspname NOT IN ('pg_catalog', 'information_schema')
+    AND n.nspname NOT LIKE 'pg\\_toast%'
+    AND n.nspname NOT LIKE 'pg\\_temp%'
+ORDER BY n.nspname, c.relname";
+
+/// The functions and procedures the database's users wrote: not the
+/// system's, and not an extension's.
+const ROUTINES: &str = "\
+SELECT DISTINCT n.nspname, p.proname
+FROM pg_proc p
+JOIN pg_namespace n ON n.oid = p.pronamespace
+WHERE n.nspname NOT IN ('pg_catalog', 'information_schema')
+    AND NOT EXISTS (
+        SELECT 1 FROM pg_depend d
+        WHERE d.classid = 'pg_proc'::regclass AND d.objid = p.oid AND d.deptype = 'e'
+    )";
+
+/// Checks `read` against what `tables` hides, in the open transaction.
+/// `Ok(None)`: it may run. `Ok(Some(error))`: it may not, and `error` is
+/// why, a refusal or the server's own error on the plan (the statement
+/// would have failed with it). `Err` is a lost session or a cancel.
+pub(super) async fn check(
+    client: &tokio_postgres::Client,
+    read: &Read,
+    tables: &Tables,
+) -> Result<Option<Error>> {
+    let catalog = catalog(client).await?;
+    let hidden = Hidden::new(Dialect::Postgres, &catalog, tables);
+    if let Err(refusal) = hidden.check_statement(read) {
+        return Ok(Some(Error::Agent(refusal)));
+    }
+    let explain = format!("EXPLAIN (VERBOSE, FORMAT JSON) {}", planned(read));
+    let params: Vec<&(dyn tokio_postgres::types::ToSql + Sync)> = read
+        .params()
+        .iter()
+        .map(|param| param as &(dyn tokio_postgres::types::ToSql + Sync))
+        .collect();
+    let rows = match client.query(explain.as_str(), &params).await {
+        Ok(rows) => rows,
+        Err(error) => {
+            let error = query_error(error);
+            if error.is_connection_lost() || error == Error::Cancelled {
+                return Err(error);
+            }
+            return Ok(Some(error));
+        }
+    };
+    let plan: Option<super::script::Plan> = rows.first().and_then(|row| row.try_get(0).ok());
+    let Some(plan) = plan else {
+        return Ok(Some(Error::Agent(Refusal::NotParsed)));
+    };
+    let Ok(plan) = serde_json::from_str::<serde_json::Value>(&plan.0) else {
+        return Ok(Some(Error::Agent(Refusal::NotParsed)));
+    };
+    Ok(planned_reads(&plan, &hidden).err().map(Error::Agent))
+}
+
+/// Checks every relation and every function a plan names.
+fn planned_reads(plan: &serde_json::Value, hidden: &Hidden) -> std::result::Result<(), Refusal> {
+    match plan {
+        serde_json::Value::Object(node) => {
+            let text = |key: &str| node.get(key).and_then(serde_json::Value::as_str);
+            if let Some(name) = text("Relation Name") {
+                hidden.check_engine_read(text("Schema"), name)?;
+            }
+            // A function in a FROM. One in an expression is in no node of
+            // its own: the reading of the text finds it.
+            if let Some(name) = text("Function Name")
+                && !matches!(text("Schema"), Some("pg_catalog" | "information_schema"))
+            {
+                hidden.check_function(name)?;
+            }
+            node.values()
+                .try_for_each(|value| planned_reads(value, hidden))
+        }
+        serde_json::Value::Array(values) => values
+            .iter()
+            .try_for_each(|value| planned_reads(value, hidden)),
+        _ => Ok(()),
+    }
+}
+
+/// The catalog as the open transaction sees it.
+async fn catalog(client: &tokio_postgres::Client) -> Result<Catalog> {
+    let mut relations: Vec<Relation> = Vec::new();
+    for row in rows(client, RELATIONS).await? {
+        let (Some(schema), Some(name)) = (row.get(0), row.get(1)) else {
+            continue;
+        };
+        let same = relations
+            .last()
+            .is_some_and(|last| last.schema == schema && last.name == name);
+        if !same {
+            relations.push(Relation {
+                schema: schema.to_owned(),
+                name: name.to_owned(),
+                depends_on: Vec::new(),
+                opaque: false,
+            });
+        }
+        if let (Some(on_schema), Some(on_name), Some(last)) =
+            (row.get(2), row.get(3), relations.last_mut())
+        {
+            last.depends_on
+                .push((on_schema.to_owned(), on_name.to_owned()));
+        }
+    }
+    let routines = rows(client, ROUTINES)
+        .await?
+        .iter()
+        .filter_map(|row| Some((row.get(0)?.to_owned(), row.get(1)?.to_owned())))
+        .collect();
+    Ok(Catalog {
+        relations,
+        routines,
+    })
+}
+
+/// The rows `sql` answers, as text.
+async fn rows(
+    client: &tokio_postgres::Client,
+    sql: &str,
+) -> Result<Vec<tokio_postgres::SimpleQueryRow>> {
+    let messages = client.simple_query(sql).await.map_err(query_error)?;
+    Ok(messages
+        .into_iter()
+        .filter_map(|message| match message {
+            SimpleQueryMessage::Row(row) => Some(row),
+            _ => None,
+        })
+        .collect())
+}
+
 #[cfg(test)]
 mod tests {
     use super::*;
diff --git a/crates/tabletist-db/src/pg/script.rs b/crates/tabletist-db/src/pg/script.rs
index 7d3ae68..40f004d 100644
--- a/crates/tabletist-db/src/pg/script.rs
+++ b/crates/tabletist-db/src/pg/script.rs
@@ -39,7 +39,7 @@ impl Conn {
         if mode == ScriptMode::Write {
             return write::run(&client, texts, limit as usize, stop).await;
         }
-        read_only(&client, texts, limit as usize, stop, None).await
+        read_only(&client, texts, limit as usize, stop, None, None).await
     }
 
     /// See [`crate::Connection::run_agent_read`]: the run of a script of
@@ -50,6 +50,7 @@ impl Conn {
         &self,
         read: &Read,
         limits: Limits,
+        tables: &crate::agent::Tables,
         stop: &StopFlag,
     ) -> Result<ScriptOutcome> {
         let client = self.client.lock().await;
@@ -58,27 +59,50 @@ impl Conn {
             params: read.params(),
             timeout: limits.timeout,
         };
-        read_only(&client, &texts, limits.rows as usize, stop, Some(sent)).await
+        let hiding = tables.hides_anything().then_some((read, tables));
+        read_only(
+            &client,
+            &texts,
+            limits.rows as usize,
+            stop,
+            Some(sent),
+            hiding,
+        )
+        .await
     }
 }
 
 /// The run that only reads: its transaction, its statements, and the
-/// close. `sent` is there when the run is of an agent's one statement.
+/// close. `sent` is there when the run is of an agent's one statement, and
+/// `hiding` when its connection hides tables: the statement is checked
+/// against them in the transaction, and runs only if it reads none.
 async fn read_only(
     client: &tokio_postgres::Client,
     texts: &[String],
     limit: usize,
     stop: &StopFlag,
     sent: Option<Sent<'_>>,
+    hiding: Option<(&Read, &crate::agent::Tables)>,
 ) -> Result<ScriptOutcome> {
     let mut outcome = ScriptOutcome::default();
     let tx = match open(client, sent).await {
         // A lost session ends the run here: nothing to close.
-        Ok(Tx::Open) => {
-            let params = sent.map(|sent| sent.params);
-            let mode = ScriptMode::ReadOnly;
-            statements(client, texts, limit, mode, stop, params, &mut outcome).await?
-        }
+        Ok(Tx::Open) => match refused_for_hiding(client, hiding).await {
+            Ok(Some((error, tx))) => {
+                outcome.stopped = error == Error::Cancelled;
+                outcome.results.push(StatementResult {
+                    elapsed: Duration::ZERO,
+                    outcome: statement_failed(error, None)?,
+                });
+                tx
+            }
+            Ok(None) => {
+                let params = sent.map(|sent| sent.params);
+                let mode = ScriptMode::ReadOnly;
+                statements(client, texts, limit, mode, stop, params, &mut outcome).await?
+            }
+            Err(error) => return Err(error),
+        },
         // A cancel landed on the opening queries: no results.
         Ok(tx) => {
             outcome.stopped = true;
@@ -102,6 +126,27 @@ async fn read_only(
     Ok(outcome)
 }
 
+/// Why an agent's statement may not run on a connection that hides tables
+/// (`super::hidden`), and where that leaves the transaction: open after a
+/// refusal, failed after the server's own error. `None` where it may run,
+/// or where nothing is hidden. `Err` is a lost session.
+async fn refused_for_hiding(
+    client: &tokio_postgres::Client,
+    hiding: Option<(&Read, &crate::agent::Tables)>,
+) -> Result<Option<(Error, Tx)>> {
+    let Some((read, tables)) = hiding else {
+        return Ok(None);
+    };
+    match super::hidden::check(client, read, tables).await {
+        Ok(None) => Ok(None),
+        Ok(Some(refusal @ Error::Agent(_))) => Ok(Some((refusal, Tx::Open))),
+        Ok(Some(error)) => Ok(Some((error, Tx::Aborted))),
+        Err(Error::Cancelled) => Ok(Some((Error::Cancelled, Tx::Aborted))),
+        Err(error) if error.is_connection_lost() => Err(error),
+        Err(error) => Ok(Some((error, Tx::Aborted))),
+    }
+}
+
 /// An agent's value as PostgreSQL is sent it: in text, which the server
 /// reads as the type it gave the placeholder, the way it reads a quoted
 /// literal. So one kind of value fills a placeholder of any type, and a
@@ -138,7 +183,7 @@ impl ToSql for Param {
 
 /// A value of a plan, which the server sends as it is written whatever
 /// the protocol: `EXPLAIN` answers in text, JSON or XML.
-struct Plan(String);
+pub(super) struct Plan(pub(super) String);
 
 impl<'a> FromSql<'a> for Plan {
     fn from_sql(
````

- [ ] **Step 4: Run the tests**

Run: `cargo test --locked -p tabletist-db --test agent` (with both servers), then `cargo test --locked -p tabletist-db --lib -- pg::hidden agent::hidden`

Expected: PASS.

```text
test result: ok. 26 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 467 filtered out
```

- [ ] **Step 5: Commit**

```bash
cargo fmt --all
cargo clippy --locked --workspace --all-targets -- -D warnings
git add crates/tabletist-db
git commit -m "Keep an agent from the tables a PostgreSQL connection hides, by the statement's plan"
```

---

### Task 4: MySQL, by the plan and the query the server rewrote

- `mysql/hidden.rs` reads the catalog in the run's read-only transaction: `information_schema.TABLES`, what each view reads (`VIEW_TABLE_USAGE`, MySQL 8.0.13 and later; where the server has none, every view is opaque and hidden while anything is), and `ROUTINES`. Each query carries a `LIMIT` of its own, since the run sets `sql_select_limit`. It refuses what the text names, then asks `EXPLAIN FORMAT=JSON` with the agent's values bound and checks each `table_name` the catalog lists (a name there may be an alias), then reads the note `SHOW WARNINGS` gives (code 1003), the query as the server rewrote it, and checks each table it writes `` `db`.`table` ``.
- `mysql/script.rs` checks before the statement runs; a refusal or an error leaves the run unconfirmed, and the transaction is rolled back.

**Files:**
- Create: `crates/tabletist-db/src/mysql/hidden.rs`
- Modify: `crates/tabletist-db/src/mysql.rs`, `crates/tabletist-db/src/mysql/script.rs`, `crates/tabletist-db/tests/agent.rs`

- [ ] **Step 1: Write the failing test**

````diff
diff --git a/crates/tabletist-db/src/mysql/hidden.rs b/crates/tabletist-db/src/mysql/hidden.rs
new file mode 100644
index 0000000..da8c689
--- /dev/null
+++ b/crates/tabletist-db/src/mysql/hidden.rs
@@ -0,0 +1,73 @@
+//! What an agent may not read on MySQL and MariaDB, asked of the server
+//! in the run's own transaction, before the statement runs (the MCP
+//! plan's B.3): the catalog as it is at that moment, and the statement's
+//! plan.
+//!
+//! MySQL's plan is two readings, and neither is enough alone. Its JSON
+//! names each table it reads by the statement's alias where there is one,
+//! and keeps a table that the optimizer read while planning (an `EXISTS`
+//! it answered at once). The note `SHOW WARNINGS` gives after it is the
+//! query as the server rewrote it, every table written `` `db`.`table` ``
+//! with views expanded, and leaves out what the optimizer already
+//! answered. So a table either names is checked, and the reading of the
+//! statement's own text beside them.
+
+#[cfg(test)]
+mod tests {
+    use super::*;
+    use crate::agent::TableName;
+
+    fn bookshop(tables: Tables) -> Hidden {
+        let relation = |name: &str| Relation {
+            schema: "bookshop".into(),
+            name: name.into(),
+            depends_on: Vec::new(),
+            opaque: false,
+        };
+        let catalog = Catalog {
+            relations: vec![relation("books"), relation("customers")],
+            routines: Vec::new(),
+        };
+        Hidden::new(Dialect::MySql, &catalog, &tables)
+    }
+
+    #[test]
+    fn the_rewritten_query_names_each_table_by_its_database_and_its_name() {
+        let hidden = bookshop(Tables::AllExcept(vec![TableName {
+            schema: None,
+            name: "customers".into(),
+        }]));
+        let books = "/* select#1 */ select `bookshop`.`b`.`title` AS `title` \
+                     from `bookshop`.`books` `b`";
+        assert_eq!(rewritten_reads(books, &hidden), Ok(()));
+        // Its alias is `b`, and its own name is still said.
+        let customers = "/* select#1 */ select `bookshop`.`b`.`id` AS `id` \
+                         from `bookshop`.`customers` `b` where (`bookshop`.`b`.`id` = 1)";
+        assert_eq!(
+            rewritten_reads(customers, &hidden),
+            Err(Refusal::TableHidden {
+                name: "customers".into(),
+                through: None
+            })
+        );
+        // The system's own, as the information schema is rewritten.
+        let system = "select `mysql`.`tbl`.`name` AS `TABLE_NAME` from `mysql`.`tables` `tbl`";
+        assert_eq!(
+            rewritten_reads(system, &hidden),
+            Err(Refusal::SystemCatalog)
+        );
+        // An alias that is a hidden table's name is not read as one in the
+        // plan's JSON unless the catalog lists it so.
+        let plan: serde_json::Value = serde_json::json!({
+            "query_block": { "table": { "table_name": "b" } }
+        });
+        assert_eq!(json_reads(&plan, &hidden), Ok(()));
+        let plan: serde_json::Value = serde_json::json!({
+            "query_block": { "nested_loop": [
+                { "table": { "table_name": "<derived2>" } },
+                { "table": { "table_name": "customers" } }
+            ] }
+        });
+        assert!(json_reads(&plan, &hidden).is_err());
+    }
+}
diff --git a/crates/tabletist-db/src/mysql/script.rs b/crates/tabletist-db/src/mysql/script.rs
index f269ae8..f631c8e 100644
--- a/crates/tabletist-db/src/mysql/script.rs
+++ b/crates/tabletist-db/src/mysql/script.rs
@@ -1023,6 +1023,7 @@ mod tests {
             Access::ReadOnly,
             &stop,
             Some(sent),
+            None,
         )
         .await
         .unwrap();
diff --git a/crates/tabletist-db/tests/agent.rs b/crates/tabletist-db/tests/agent.rs
index d6f574d..504981e 100644
--- a/crates/tabletist-db/tests/agent.rs
+++ b/crates/tabletist-db/tests/agent.rs
@@ -993,6 +993,14 @@ mod mysql {
         an_agent_reads_the_bookshop(&connection, "mysql").await;
     }
 
+    #[tokio::test]
+    async fn a_hidden_table_is_never_read_on_mysql() {
+        let Some(connection) = bookshop().await else {
+            return;
+        };
+        a_hidden_table_is_never_read(&connection, "mysql").await;
+    }
+
     #[tokio::test]
     async fn mysql_takes_each_value_as_its_own_type() {
         let Some(connection) = bookshop().await else {
````

- [ ] **Step 2: Run it to see it fail**

Run: `cargo test --locked -p tabletist-db --test agent mysql::a_hidden_table_is_never_read_on_mysql`, with `TABLETIST_TEST_MYSQL_URL` set

Expected: FAIL, as on PostgreSQL before Task 3:

```text
test mysql::a_hidden_table_is_never_read_on_mysql ... FAILED
thread 'mysql::a_hidden_table_is_never_read_on_mysql' panicked at crates/tabletist-db/tests/agent.rs:344:63:
called `Result::unwrap_err()` on an `Ok` value: Answer { … }
```

- [ ] **Step 3: The catalog, the two readings of the plan, and the check**

````diff
diff --git a/crates/tabletist-db/src/mysql.rs b/crates/tabletist-db/src/mysql.rs
index 1fb0922..a613139 100644
--- a/crates/tabletist-db/src/mysql.rs
+++ b/crates/tabletist-db/src/mysql.rs
@@ -50,6 +50,7 @@ const UNKNOWN_SYSTEM_VARIABLE: u16 = 1193;
 
 const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
 
+mod hidden;
 mod script;
 mod write;
 
@@ -516,10 +517,10 @@ impl Adapter for Conn {
         &self,
         read: &Read,
         limits: Limits,
-        _tables: &Tables,
+        tables: &Tables,
         stop: &StopFlag,
     ) -> Result<ScriptOutcome> {
-        self.agent_read(read, limits, stop).await
+        self.agent_read(read, limits, tables, stop).await
     }
 
     async fn write(&self, changes: &ChangeSet, stop: &StopFlag) -> Result<WriteOutcome> {
diff --git a/crates/tabletist-db/src/mysql/hidden.rs b/crates/tabletist-db/src/mysql/hidden.rs
index da8c689..8ffd2fc 100644
--- a/crates/tabletist-db/src/mysql/hidden.rs
+++ b/crates/tabletist-db/src/mysql/hidden.rs
@@ -12,6 +12,189 @@
 //! answered. So a table either names is checked, and the reading of the
 //! statement's own text beside them.
 
+use mysql_async::prelude::Queryable;
+
+use super::query_error;
+use crate::agent::{Catalog, Hidden, Read, Refusal, Relation, Tables, planned};
+use crate::sql::{self, TokenKind};
+use crate::{Dialect, Error, Result};
+
+/// The system's own schemas, which the catalog leaves out.
+const SYSTEM: &str = "('information_schema', 'mysql', 'performance_schema', 'sys')";
+
+/// The run sets `sql_select_limit` for the agent's rows; a `LIMIT` of a
+/// query's own takes its place, so the catalog is read whole.
+const WHOLE: &str = "LIMIT 18446744073709551615";
+
+/// Checks `read` against what `tables` hides, in the open transaction.
+/// `Ok(None)`: it may run. `Ok(Some(error))`: it may not, and `error` is
+/// why, a refusal or the server's own error on the plan (the statement
+/// would have failed with it). `Err` is a lost session or a cancel.
+pub(super) async fn check(
+    conn: &mut mysql_async::Conn,
+    read: &Read,
+    tables: &Tables,
+) -> Result<Option<Error>> {
+    let catalog = catalog(conn).await?;
+    let hidden = Hidden::new(Dialect::MySql, &catalog, tables);
+    if let Err(refusal) = hidden.check_statement(read) {
+        return Ok(Some(Error::Agent(refusal)));
+    }
+    let explain = format!("EXPLAIN FORMAT=JSON {}", planned(read));
+    let plan: Option<String> = match conn
+        .exec_first(explain, super::script::bound(read.params()))
+        .await
+    {
+        Ok(plan) => plan,
+        Err(error) => {
+            let error = query_error(error);
+            if error.is_connection_lost() || error == Error::Cancelled {
+                return Err(error);
+            }
+            return Ok(Some(error));
+        }
+    };
+    let Some(Ok(plan)) = plan.map(|plan| serde_json::from_str::<serde_json::Value>(&plan)) else {
+        return Ok(Some(Error::Agent(Refusal::NotParsed)));
+    };
+    if let Err(refusal) = json_reads(&plan, &hidden) {
+        return Ok(Some(Error::Agent(refusal)));
+    }
+    // The note on the plan: the query as the server rewrote it. MariaDB
+    // may give none.
+    let notes: Vec<(String, u32, String)> =
+        conn.query("SHOW WARNINGS").await.map_err(query_error)?;
+    for (_, code, note) in notes {
+        if code == 1003
+            && let Err(refusal) = rewritten_reads(&note, &hidden)
+        {
+            return Ok(Some(Error::Agent(refusal)));
+        }
+    }
+    Ok(None)
+}
+
+/// Checks every table a JSON plan names. A name may be the statement's
+/// alias, which the catalog does not list: such a name is passed over,
+/// and the note and the text say what it stands for. A derived table's
+/// name (`<derived2>`) is the server's own.
+fn json_reads(plan: &serde_json::Value, hidden: &Hidden) -> std::result::Result<(), Refusal> {
+    match plan {
+        serde_json::Value::Object(node) => {
+            if let Some(name) = node.get("table_name").and_then(serde_json::Value::as_str)
+                && !name.starts_with('<')
+            {
+                hidden.check_name(None, name)?;
+            }
+            node.values()
+                .try_for_each(|value| json_reads(value, hidden))
+        }
+        serde_json::Value::Array(values) => values
+            .iter()
+            .try_for_each(|value| json_reads(value, hidden)),
+        _ => Ok(()),
+    }
+}
+
+/// Checks every table the rewritten query names: each name of two parts,
+/// `` `db`.`table` ``, is one. A column is written with three, and an
+/// alias with one.
+fn rewritten_reads(note: &str, hidden: &Hidden) -> std::result::Result<(), Refusal> {
+    let tokens: Vec<sql::Token> = sql::tokenize(Dialect::MySql, note)
+        .into_iter()
+        .filter(|token| !matches!(token.kind, TokenKind::Whitespace | TokenKind::Comment))
+        .collect();
+    let quoted = |token: &sql::Token| {
+        let written = &note[token.range.clone()];
+        (token.kind == TokenKind::QuotedIdentifier && written.starts_with('`'))
+            .then(|| written[1..written.len() - 1].replace("``", "`"))
+    };
+    let dot =
+        |token: Option<&sql::Token>| token.is_some_and(|token| &note[token.range.clone()] == ".");
+    let mut at = 0;
+    while at < tokens.len() {
+        let Some(first) = quoted(&tokens[at]) else {
+            at += 1;
+            continue;
+        };
+        let mut parts = vec![first];
+        let mut next = at + 1;
+        while dot(tokens.get(next))
+            && let Some(part) = tokens.get(next + 1).and_then(quoted)
+        {
+            parts.push(part);
+            next += 2;
+        }
+        if let [schema, table] = parts.as_slice() {
+            hidden.check_engine_read(Some(schema), table)?;
+        }
+        at = next;
+    }
+    Ok(())
+}
+
+/// The catalog as the open transaction sees it: the relations of every
+/// schema but the system's, what each view reads where the server says so
+/// (`VIEW_TABLE_USAGE`, MySQL 8.0.13 and later), and the users' routines.
+/// Where it does not say, every view is taken to read what is hidden.
+async fn catalog(conn: &mut mysql_async::Conn) -> Result<Catalog> {
+    let listed: Vec<(String, String, String)> = conn
+        .query(format!(
+            "SELECT TABLE_SCHEMA, TABLE_NAME, TABLE_TYPE FROM information_schema.TABLES \
+             WHERE TABLE_SCHEMA NOT IN {SYSTEM} {WHOLE}"
+        ))
+        .await
+        .map_err(query_error)?;
+    let usage: Option<Vec<(String, String, String, String)>> = conn
+        .query(format!(
+            "SELECT VIEW_SCHEMA, VIEW_NAME, TABLE_SCHEMA, TABLE_NAME \
+             FROM information_schema.VIEW_TABLE_USAGE {WHOLE}"
+        ))
+        .await
+        .map(Some)
+        .map_err(query_error)
+        .or_else(|error| match error {
+            Error::ConnectionLost(_) | Error::Cancelled => Err(error),
+            // No such table on this server.
+            _ => Ok(None),
+        })?;
+    // What each view reads, by the view.
+    let mut reads: std::collections::HashMap<(String, String), Vec<(String, String)>> =
+        std::collections::HashMap::new();
+    for (view_schema, view_name, on_schema, on_name) in usage.iter().flatten() {
+        reads
+            .entry((view_schema.clone(), view_name.clone()))
+            .or_default()
+            .push((on_schema.clone(), on_name.clone()));
+    }
+    let relations = listed
+        .into_iter()
+        .map(|(schema, name, kind)| {
+            let view = kind.eq_ignore_ascii_case("VIEW");
+            let depends_on = reads
+                .remove(&(schema.clone(), name.clone()))
+                .unwrap_or_default();
+            Relation {
+                schema,
+                name,
+                depends_on,
+                opaque: view && usage.is_none(),
+            }
+        })
+        .collect();
+    let routines: Vec<(String, String)> = conn
+        .query(format!(
+            "SELECT ROUTINE_SCHEMA, ROUTINE_NAME FROM information_schema.ROUTINES \
+             WHERE ROUTINE_SCHEMA NOT IN {SYSTEM} {WHOLE}"
+        ))
+        .await
+        .map_err(query_error)?;
+    Ok(Catalog {
+        relations,
+        routines,
+    })
+}
+
 #[cfg(test)]
 mod tests {
     use super::*;
diff --git a/crates/tabletist-db/src/mysql/script.rs b/crates/tabletist-db/src/mysql/script.rs
index f631c8e..af9dc21 100644
--- a/crates/tabletist-db/src/mysql/script.rs
+++ b/crates/tabletist-db/src/mysql/script.rs
@@ -52,7 +52,7 @@ impl Conn {
         if mode == ScriptMode::Write {
             return write::run(&mut conn, texts, limit as usize, stop).await;
         }
-        read_only(&mut conn, texts, limit, self.access, stop, None).await
+        read_only(&mut conn, texts, limit, self.access, stop, None, None).await
     }
 
     /// See [`crate::Connection::run_agent_read`]: the run of a script of
@@ -64,6 +64,7 @@ impl Conn {
         &self,
         read: &Read,
         limits: Limits,
+        tables: &crate::agent::Tables,
         stop: &StopFlag,
     ) -> Result<ScriptOutcome> {
         let mut conn = self.conn.lock().await;
@@ -78,12 +79,24 @@ impl Conn {
             timeout: limits.timeout,
         };
         let access = self.access;
-        read_only(&mut conn, &texts, limits.rows, access, stop, Some(sent)).await
+        let hiding = tables.hides_anything().then_some((read, tables));
+        read_only(
+            &mut conn,
+            &texts,
+            limits.rows,
+            access,
+            stop,
+            Some(sent),
+            hiding,
+        )
+        .await
     }
 }
 
 /// The run that only reads: its transaction, its statements, and the
-/// close. `sent` is there when the run is of an agent's one statement.
+/// close. `sent` is there when the run is of an agent's one statement, and
+/// `hiding` when its connection hides tables: the statement is checked
+/// against them in the transaction, and runs only if it reads none.
 async fn read_only(
     conn: &mut mysql_async::Conn,
     texts: &[String],
@@ -91,14 +104,25 @@ async fn read_only(
     access: Access,
     stop: &StopFlag,
     sent: Option<Sent<'_>>,
+    hiding: Option<(&Read, &crate::agent::Tables)>,
 ) -> Result<ScriptOutcome> {
     let mut outcome = ScriptOutcome::default();
     let ended = match open(conn, limit, access, sent).await {
         // A lost session ends the run here: nothing to close.
-        Ok(()) => {
-            let params = sent.map(|sent| sent.params);
-            statements(conn, texts, limit as usize, stop, params, &mut outcome).await?
-        }
+        Ok(()) => match refused_for_hiding(conn, hiding).await? {
+            Some(error) => {
+                outcome.stopped = error == Error::Cancelled;
+                outcome.results.push(StatementResult {
+                    elapsed: Duration::ZERO,
+                    outcome: statement_failed(error, None)?,
+                });
+                Ended::Unconfirmed
+            }
+            None => {
+                let params = sent.map(|sent| sent.params);
+                statements(conn, texts, limit as usize, stop, params, &mut outcome).await?
+            }
+        },
         // A cancel landed on the opening queries: no results.
         Err(Error::Cancelled) => {
             outcome.stopped = true;
@@ -117,6 +141,24 @@ async fn read_only(
     Ok(outcome)
 }
 
+/// Why an agent's statement may not run on a connection that hides tables
+/// (`super::hidden`): a refusal, the server's own error, or a cancel.
+/// `None` where it may run, or where nothing is hidden. `Err` is a lost
+/// session.
+async fn refused_for_hiding(
+    conn: &mut mysql_async::Conn,
+    hiding: Option<(&Read, &crate::agent::Tables)>,
+) -> Result<Option<Error>> {
+    let Some((read, tables)) = hiding else {
+        return Ok(None);
+    };
+    match super::hidden::check(conn, read, tables).await {
+        Ok(refused) => Ok(refused),
+        Err(error) if error.is_connection_lost() => Err(error),
+        Err(error) => Ok(Some(error)),
+    }
+}
+
 /// Where a session stands relative to the read-only transaction a script
 /// runs in.
 #[derive(Debug, Clone, Copy, PartialEq, Eq)]
@@ -514,7 +556,7 @@ fn counts_rows(text: &str) -> bool {
 /// An agent's values as the driver takes a statement's: each as its own
 /// type, by the protocol that binds them. A value is never written into
 /// the statement.
-fn bound(params: &[Param]) -> Params {
+pub(super) fn bound(params: &[Param]) -> Params {
     use mysql_async::Value as MySql;
     if params.is_empty() {
         return Params::Empty;
````

- [ ] **Step 4: Run the tests**

Run: `cargo test --locked -p tabletist-db --test agent` (with both servers), then `cargo test --locked -p tabletist-db --lib mysql::hidden`

Expected: PASS.

```text
test result: ok. 27 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 475 filtered out
```

- [ ] **Step 5: Commit**

```bash
cargo fmt --all
cargo clippy --locked --workspace --all-targets -- -D warnings
git add crates/tabletist-db
git commit -m "Keep an agent from the tables a MySQL connection hides, by its plan and its rewritten query"
```

---

### Task 5: The master plan

- Questions 8 and 13, settled as recommended; G cuts 1c into three runs; B.2, B.3 and F say what this run built, where it went otherwise, and what it leaves.

**Files:**
- Modify: `docs/superpowers/plans/2026-10-13-mcp-server.md`

- [ ] **Step 1: The edits**

````diff
diff --git a/docs/superpowers/plans/2026-10-13-mcp-server.md b/docs/superpowers/plans/2026-10-13-mcp-server.md
index cb4e075..80844ab 100644
--- a/docs/superpowers/plans/2026-10-13-mcp-server.md
+++ b/docs/superpowers/plans/2026-10-13-mcp-server.md
@@ -66,6 +66,13 @@ Nothing of 1d is planned or built: no network listener, no tokens, no
 kept as the design for if it is taken up again. Where another section
 says something comes "with HTTP (1d)", it is not planned now.
 
+**Settled on 2026-10-15: questions 8 and 13, and Phase 1c in three runs.**
+An agent is told the names of the tables hidden from it, and the system
+catalogs are open on a connection that hides nothing and closed on one
+that hides something, each as this plan recommended. G cuts 1c into three
+runs. Its first (`2026-10-15-mcp-server-1c-1-checks.md`) built the checks, with nothing visible: B.2 and
+B.3 are corrected where its draft showed otherwise.
+
 ## Contents
 
 1. [What the code says, in short](#1-what-the-code-says-in-short)
@@ -709,6 +716,9 @@ what overflows. What follows from it:
   tree has (B.3, point 1). It must bound the depth before it walks: a cap on
   the number of tokens is the obvious bound, since a level costs at least
   one. That design is 1c's.
+  As built (`2026-10-15-mcp-server-1c-1-checks.md`), the tree is not walked: 1c reads the names off the
+  tokens too, which can miss one, and what the engine says the statement
+  reads is the fence (B.3).
 
 Why not the alternatives:
 
@@ -881,6 +891,68 @@ board's hint:
 The same sentence, with the list above, goes in the documentation page for
 agents.
 
+**As built in 1c's first run** (`2026-10-15-mcp-server-1c-1-checks.md`): the checks, in
+`crates/tabletist-db/src/agent/hidden.rs` and a `hidden.rs` for each
+server, behind a `tables` argument of `Connection::run_agent_read`. The app
+passes "All" until the next run reads the setting. Agreed on 2026-10-15,
+where it went otherwise:
+
+- **The statement's names are read off its tokens, not its tree** (B.2
+  keeps the tree from being walked). A name after `FROM`, a `JOIN`, a comma
+  of a `FROM`'s list, `LATERAL` or `ONLY`, or in parentheses where a table
+  goes, at any depth, is read, and so is what a `WITH` defines; a name with
+  a parenthesis after it is a call. The reading errs toward finding, and
+  it is not the fence: what it misses, point 3 sees. It is what names the
+  view the agent asked for, and what sees a hidden view whose tables are
+  not.
+- **The catalog is read for each call, in the call's own read-only
+  transaction**, not when the session opens: the check sees the database
+  as the statement will. It costs two or three queries more on PostgreSQL
+  and MySQL, and nothing on a connection that hides nothing. So MySQL has
+  no gap of 30 seconds for a view made later.
+- **SQLite lists no dependencies.** The authorizer reports each table a
+  view reads, with the view (and a table only `count(*)` or `EXISTS`
+  reads, and `sqlite_schema`), so a view is refused by what it reads, and
+  the refusal names it. The catalog is `pragma_table_list`, for the names
+  a refusal gives.
+- **PostgreSQL compares by schema and name**, as the plan writes them,
+  not by OID. The plan names each relation it scans with its schema: a
+  partition (not its parent), a child table under its parent's scan, a
+  materialized view as itself, whose own dependencies the catalog lists.
+  A function in a `FROM` is a `Function Scan` of the plan, and one in an
+  expression is found by the reading of the text. The users' functions
+  (`pg_proc` outside the system's schemas, and not an extension's) are
+  refused while anything is hidden. An `EXPLAIN` of the agent's own is
+  checked as the query it explains.
+- **MySQL's plan is read twice.** Its JSON names a table by the
+  statement's alias where it has one, so a name there is checked only
+  where the catalog lists it. The note `SHOW WARNINGS` gives after it
+  (code 1003) is the query as the server rewrote it, every table written
+  `` `db`.`table` `` and views expanded: each of those is checked. A plan
+  that cannot be read refuses the query (`not_parsed`). The catalog's own
+  queries carry a `LIMIT`, since the run sets `sql_select_limit` for the
+  agent's rows.
+- **On MySQL the plan is not free of the statement.** `EXPLAIN` runs some
+  subqueries while it plans (a constant one, an `EXISTS` it answers at
+  once), and its note leaves out what it answered. A hidden table that the
+  reading of the text and the catalog both miss, and that such a subquery
+  reads, is read once while planning; an error it raises there can carry
+  a value. Every name the text gives in a table's place is refused before
+  the plan is asked for, so this takes a name the reading misses.
+- **The function of the Bookshop is on PostgreSQL alone**
+  (`customer_count()`, F). MySQL lets only a user with `SUPER` make one
+  while it logs its changes, so its fixture has none, and MySQL's function
+  rule is tested off the server only.
+- **Not refused**: the system's functions that say something of a table
+  without reading a row of it, given its name: PostgreSQL's
+  `pg_relation_size`, `pg_table_size`, `has_table_privilege`,
+  `to_regclass` and their like. They say that it exists and how large it
+  is, which the refusal says too (question 8).
+- Names are compared without regard to case everywhere, which on
+  PostgreSQL refuses a quoted name that differs from a hidden one only by
+  case. A statement's name without a schema is taken for a hidden one of
+  any schema.
+
 ### B.4 Masked columns
 
 A masked column keeps its name and its type. Its values reach the agent as
@@ -2033,7 +2105,9 @@ and no snapshot material is committed. UI tests go through `src/testing.rs`
 `email` and `phone`), `order_items` (with a cascading foreign key), a view
 `recent_orders` that reads `orders.email`, a view `customer_names` over
 `customers`, and on PostgreSQL and MySQL a function that reads `customers`.
-The function comes with Phase 1c, which is the first to test one. On each
+The function comes with Phase 1c, which is the first to test one. As built,
+on PostgreSQL alone: MySQL lets only a user with `SUPER` make one while it
+logs its changes (B.3). On each
 server the Bookshop has a database of its own, `bookshop`: its `orders` is
 not the driver fixture's.
 SQLite runs everywhere. PostgreSQL and MySQL run where the suite has a
@@ -2217,7 +2291,7 @@ is off until the person turns it on.
 | 1a. The enforcement core | 3 | Done: pull requests 122, 123 and 124 |
 | 1b. The server over stdio, with its interface | 9, below | Built (`2026-10-13-mcp-server-1b-1-server.md`, `2026-10-14-mcp-server-1b-2a-gateway.md`, `2026-10-14-mcp-server-1b-2b-stdio.md`, `2026-10-14-mcp-server-1b-3a-agents-tab.md`, `2026-10-14-mcp-server-1b-3b-changes-outside.md`, `2026-10-14-mcp-server-1b-4a-activity.md`, `2026-10-14-mcp-server-1b-4b-log.md`, `2026-10-15-mcp-server-1b-5a-access.md` and `2026-10-15-mcp-server-1b-5b-settings.md`) |
 | 1b on Windows: the way in | 1 | Built (`2026-10-15-mcp-server-1b-6-windows.md`) |
-| 1c. Hidden tables | to be cut in its plan | Waits for questions 8 and 13 |
+| 1c. Hidden tables | 3, below | The checks built (`2026-10-15-mcp-server-1c-1-checks.md`). The setting and the server are next |
 | 1d. HTTP with tokens | 1 | Dropped for now (2026-10-15) |
 
 The move of the connections to `connections.toml` came between 1a and 1b
@@ -2329,6 +2403,13 @@ build; no console target.
 
 **1c. Hidden tables.**
 
+- Runs: three, each its own plan and pull request, agreed on 2026-10-15.
+  The checks in `tabletist-db`, nothing visible (`2026-10-15-mcp-server-1c-1-checks.md`). Then the
+  setting in `connections.toml` and the server: its refusals,
+  `list_connections` naming the hidden tables, `describe_schema` leaving
+  them out, a rename that stops agents, and changes made outside the app
+  held (C.3). Then the editor's Tables control in both looks, its hint,
+  and the website.
 - Files: `crates/tabletist-db/src/catalog.rs`, `pg.rs`, `mysql.rs`,
   `sqlite.rs` (what views read, routines, partitions), `agent/` (names,
   dependencies, the plan check, the system catalogs), `tabletist-sqlite-ffi`
@@ -2425,9 +2506,10 @@ The ones Phase 1 waits for are marked.
 7. The count on the approval card: a read-only `SELECT`, with a dry run
    only when the person asks. Accepted, against the board's "Counted by a
    dry run"? (Phase 2.)
-8. Is it right that an agent learns the names of hidden tables, as the
-   boards show? The alternative is to answer as the database would for a
-   table that does not exist. (Phase 1c.)
+8. Settled on 2026-10-15: yes, as the boards show. The refusal names the
+   hidden table, and `list_connections` lists the names (1c). Answering as
+   for a table that does not exist would leave the agent guessing, and the
+   names are not what is hidden: the rows are.
 9. Settled on 2026-10-13: HTTP comes after stdio, as its own pull request
    (1d). Both agents the boards name connect over stdio, and HTTP brings
    the app's first network listener, the tokens and at least nine new
@@ -2442,9 +2524,9 @@ The ones Phase 1 waits for are marked.
     tab the bottom line ends `ga agents` where the board has `a approve  d
     deny  esc later`. Accepted? The connections screen needs no `:` prompt
     any more. (Phase 2.)
-13. System catalogs: open on a connection that hides nothing, closed on one
-    that hides or masks something. Or closed everywhere, for one rule?
-    (Phase 1c.)
+13. Settled on 2026-10-15: open on a connection that hides nothing, closed
+    on one that hides or masks something (`system_catalog`, B.3). Built for
+    hidden tables in 1c's first run (`2026-10-15-mcp-server-1c-1-checks.md`); masks are Phase 3a's.
 14. Settled on 2026-10-13: the Windows way in (the named pipe, the relay
     from a GUI binary) follows the other two by one release, as a run of
     its own. The interface is built and tested on Windows with the others,
````

- [ ] **Step 2: Check the whole of it**

Run: `cargo fmt --all --check`, clippy as above, `RUSTDOCFLAGS='-D warnings' cargo doc --locked --workspace --no-deps`, `grep -c "$(printf '\342\200\224')" docs/superpowers/plans/2026-10-13-mcp-server.md` (a count of em dashes) and `cargo test --locked --workspace --all-targets` with both servers

Expected: fmt and clippy clean, the documentation builds without a warning, no em dash (`0`), and:

```text
3507 passed, 0 failed, 22 ignored
```

- [ ] **Step 3: Commit**

```bash
git add docs/superpowers/plans/2026-10-13-mcp-server.md
git commit -m "Say in the MCP plan what Phase 1c's first run built, and settle questions 8 and 13"
```

## Self-review

- **Against the master plan.** B.3's four places: the statement's names (Task 1's reading, from the tokens and not the tree: the departure is in B.3's as-built list), what depends on a hidden table (Task 1's `Hidden::new` with each driver's catalog: views and materialized views, partitions and child tables on PostgreSQL, `VIEW_TABLE_USAGE` and opaque views on MySQL, the authorizer's source on SQLite), the engine (Tasks 2, 3 and 4), and functions (Task 1's `check_function`, from Tasks 3 and 4's catalogs). The system catalogs, as question 13 settles. B.8's `table_hidden` and `system_catalog`, with its sentences. F.2's hidden-table rows, in `a_hidden_table_is_never_read` on the three engines. F's function, on PostgreSQL alone (Task 5 says why).
- **A review before the plan.** A reviewer was started on the draft and stopped before it ran anything, so nothing it would have found is here. A pass of my own instead found and fixed: a table in parentheses or a join in them, `FROM (customers JOIN books ON true)`, which the reading missed (each engine refused it, and the test holds both); a PostgreSQL plan's shapes (a child table under its parent's scan, a partition, a function scan, a catalog) without a unit test; MySQL's views looked up by a scan for each relation; and a SQLite fence that let go of what was hidden after a panic.
- **Left, and said.** On MySQL a subquery the optimizer runs while it plans reads before the plan is checked: only a hidden table that the reading and the catalog both miss, which B.3 says. The system's functions that tell of a table by its name (`pg_relation_size`, `to_regclass`) are not refused. The catalog is read for each call. MySQL's function rule runs off the server only, and MariaDB is not tested.
- **Placeholders.** None: every step has its diff or its command and what it prints.
- **Names.** `Tables`, `TableName`, `Relation`, `Catalog`, `Hidden`, `check_statement`, `check_engine_read`, `check_name`, `check_function`, `planned`, `Refusal::TableHidden`, `Refusal::SystemCatalog`, `Fences::hide` and `Hiding` are the same in every task that uses them.

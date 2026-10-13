# MCP server, Phase 1a, run 3: SQLite's own reading of an agent's statement. Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** On SQLite an agent's statement runs behind a fence of its own, `Fence::Agent`, which lists what a read does and refuses the rest on what SQLite itself read, and the agent is told what was refused by name.

**Architecture:** SQLite asks a connection's authorizer about every action of a statement it prepares, after its own parse. The driver already answers it for a SQL editor script and for a table's filter. This run adds the third answer, and a place for the first thing it denied, so the run can say `PRAGMA table_info is not available to agents.` where SQLite says only "not authorized". The check gains the one refusal the draft of this run found it was missing.

**Tech Stack:** Rust 2024, `rusqlite` 0.37, `tabletist-sqlite-ffi` (the authorizer). No new crate.

---

## Where this sits

This is the third and last run of Phase 1a of `docs/superpowers/plans/2026-10-13-mcp-server.md` (section G).

1. **The read checker.** Done: `2026-10-13-mcp-server-1a-1-read-checker.md`.
2. **The drivers.** Done: `2026-10-13-mcp-server-1a-2-agent-read.md`, merged as `0421db9`.
3. **This run: SQLite's `Fence::Agent`.**

After it, Phase 1a is done: `tabletist-db` can check an agent's read and run it on the three databases. Nothing in the app calls either yet.

## Before you start

- Work on a branch of `main`. One commit for each task, in the order below. Every commit passes `cargo fmt --all --check`, `cargo clippy --locked --workspace --all-targets -- -D warnings` and `cargo test --locked --workspace --all-targets`.
- On Igor's machine `cargo` is `~/.cargo/bin/cargo` (the mise shim fails), and a build in a worktree needs `CARGO_TARGET_DIR` set to that worktree's `target/`.
- Read `AGENTS.md`. Two of its rules shape this code: a `match` on `Dialect` names every variant, and there are no em dashes in comments or messages.
- The diffs below apply with `git apply`. Run `cargo fmt --all` before each commit: the code here is already formatted, so it should change nothing.
- Every test of this run is SQLite's and needs no server. The whole workspace's tests were run with PostgreSQL and MySQL reachable all the same.

## What was run before this plan was written

Each task's state was built, passed `cargo fmt --check` and clippy with `-D warnings` for the whole workspace, and its tests failed and then passed as each step says. The last state also passed `cargo doc` with warnings denied and the workspace's tests against PostgreSQL 17.11, MySQL 8.4.11 and SQLite. On Linux only: the CI matrix runs the same tests on macOS and Windows.

## What the draft found

**A pragma read as a table needs no parenthesis.** The check refuses SQLite's table-valued pragmas by the start of their name, `pragma_`, where a name is followed by `(`. SQLite does not ask for the parenthesis: such a pragma is a table, and its argument can be a condition on a hidden column.

```sql
SELECT * FROM pragma_table_info WHERE arg = 'customers';
SELECT name FROM pragma_database_list;
SELECT * FROM pragma_optimize;
```

All three passed `check_read` on `main`. The first two answered. The third is `PRAGMA optimize`, which writes: the session's read-only handle refused it ("attempt to write a readonly database"), which is the fence behind the check doing its work. None of this reads a row an agent could not read otherwise. It is the kind of miss the master plan gave SQLite's authorizer for: the check reads text, and SQLite reads the statement.

**What SQLite asks its authorizer**, logged on the Bookshop for this plan. Every statement is asked `SELECT`, and `READ` for each column it reads. Beside those:

| The statement | What else SQLite asks |
|---|---|
| `SELECT … FROM pragma_table_info WHERE arg = 'customers'` | `PRAGMA table_info`, with the value `customers` |
| `SELECT … FROM pragma_table_info('customers')` | the same |
| `SELECT name FROM pragma_database_list` | `PRAGMA database_list`, with no value |
| `SELECT count(*) … WHERE title LIKE 'L%'` | `FUNCTION count`, `FUNCTION like` |
| `SELECT load_extension('x')` | `FUNCTION load_extension` |
| `WITH RECURSIVE …` | `RECURSIVE` |
| A view | The view's own `READ`s, each with the view's name as its source |
| A full-text (FTS5) table | `PRAGMA data_version` with no value, and `READ`s of its shadow tables |
| An R*Tree table, the first time a session reads it | `INSERT` and `DELETE` on its three shadow tables, which it prepares and does not run |
| `EXPLAIN …`, `EXPLAIN QUERY PLAN …` | Nothing more than the query it explains |

A function arrives under the name SQLite knows it by, in lower case, whatever letters the text wrote.

**A view is already SQLite's to refuse.** The first draft of the tests reached a pragma through a view of the database's own, which no check of an agent's text can see. SQLite refused it before the authorizer's answer mattered: every session has `PRAGMA trusted_schema = OFF` (`sqlite.rs:642`), and with the schema not trusted SQLite lets a view neither read a pragma as a table nor call a function that is not marked harmless ("unsafe use of virtual table", "unsafe use of fts3_tokenizer()"). A test pins that, since the plan for hidden tables leans on it.

## What this run settles

| Question | Decision | Why |
|---|---|---|
| What the fence is | A list of what a read does. Everything else is refused | The other two fences list what to refuse, because a script may do nearly anything and a filter is the app's own `SELECT` around one condition. An agent's statement is one read, and the check works the same way: it allows, it does not forbid |
| What a read does | `SELECT`, `READ`, `RECURSIVE`, a function that is not on the check's list for SQLite, and one pragma | From the log above |
| Which pragma | `data_version`, without a value | FTS5 asks for it as it reads. Should the check ever miss `pragma_data_version`, an agent learns a counter |
| `INSERT`, `UPDATE`, `DELETE` | Let through when SQLite prepares them | R*Tree prepares its own when a table connects, and the authorizer cannot tell those from a statement's. Running one is refused by the session, which is read-only twice over (the handle, and `query_only`). `Fence::Filter` does the same, for the same reason |
| The functions | The check's own list (`agent::function_allowed`), asked of the names SQLite found | One list. Here it does not depend on where our tokenizers think a call is |
| What the agent is told | `Error::Agent(Refusal)`: the first thing the fence denied, as the check would have named it | SQLite says "not authorized" and nothing more. `Refusal` has the code (`not_allowed`, `function_not_allowed`) and the sentence already, so Phase 1b words one kind of refusal, wherever it was found |
| The check | Refuses a word that begins `pragma_` on SQLite, wherever it stands and whatever its quotes | The miss above. The first fence should not lean on the second |
| A byte that is no fence | Counts as `Agent`, the strictest | It was `Filter`. `Agent` denies everything `Filter` denies, which a test checks row by row |

### What the check gets wrong on purpose, now

- A column or a table of the user's own whose name begins `pragma_` is refused on SQLite, with or without quotes. SQLite reads `"pragma_table_info"` in quotes as the same table.

### What this run leaves out

- **Hidden tables and masked columns.** The fence lets every `READ` through. Phase 1c gives it the tables to deny, and Phase 3a the columns to answer as NULL (SQLite's third answer, "ignore", which `tabletist-sqlite-ffi` does not give yet). The place this run makes for "what was denied" is where both will say what they refused.
- **Saying which view a denied action came from.** SQLite gives the innermost view or trigger as the action's source. The refusal names the function or the pragma and not the view. No test needed it: with the schema not trusted, SQLite refuses those views first.
- **`sqlite_master`, `dbstat` and the like.** They are reads. Whether an agent may read the catalog is Phase 1c's (open question 13).

### What does not change

The SQL editor's script and a table's filter. `Fence::Script` and `Fence::Filter` answer as before, and a script that is denied gets SQLite's own error as before: only an agent's fence keeps what it denied. `tests/sqlite.rs` and the fence's existing tests pass unchanged, but for one test that names the strictest fence.

## File structure

| File | What changes |
|---|---|
| `crates/tabletist-db/src/agent.rs` | A pragma read as a table is refused by its name alone. `function_allowed` is the crate's, for the fence |
| `crates/tabletist-db/src/error.rs` | `Error::Agent(Refusal)` |
| `crates/tabletist-db/src/sqlite/fence.rs` | `Fence::Agent`, what it allows, and `Fences` keeping what it denied |
| `crates/tabletist-db/src/sqlite.rs` | An agent's statement runs behind `Fence::Agent`, and a denial is answered by name |
| `crates/tabletist-db/tests/agent.rs` | What the fence must not break, and what a view may not do |

---

### Task 1: A pragma read as a table

The check's own miss, closed in the check. `pragma_table` says whether a name is one of SQLite's table-valued pragmas, and `words_read` refuses such a word before it looks for a parenthesis. The prefix leaves the function list, where it only matched a call.

**Files:**
- Modify: `crates/tabletist-db/src/agent.rs`

- [ ] **Step 1: Write the failing test**

```diff
diff --git a/crates/tabletist-db/src/agent.rs b/crates/tabletist-db/src/agent.rs
index bc6da88..74f5141 100644
--- a/crates/tabletist-db/src/agent.rs
+++ b/crates/tabletist-db/src/agent.rs
@@ -1722,6 +1722,54 @@ mod tests {
         }
     }
 
+    /// SQLite reads a table-valued pragma as a table: no parenthesis has to
+    /// follow its name, and its argument can be a condition on the hidden
+    /// column `arg`. So the name is refused wherever it stands.
+    #[test]
+    fn a_pragma_read_as_a_table_needs_no_parenthesis_to_be_refused() {
+        for (sql, name) in [
+            (
+                "SELECT * FROM pragma_table_info WHERE arg = 'customers'",
+                "pragma_table_info",
+            ),
+            (
+                "SELECT name FROM pragma_database_list",
+                "pragma_database_list",
+            ),
+            ("SELECT * FROM PRAGMA_Optimize", "PRAGMA_Optimize"),
+            (
+                "SELECT * FROM main.pragma_integrity_check",
+                "pragma_integrity_check",
+            ),
+            // In quotes it is the same table.
+            (
+                "SELECT * FROM \"pragma_table_info\" WHERE arg = 'customers'",
+                "pragma_table_info",
+            ),
+            (
+                "SELECT * FROM [pragma_function_list]",
+                "pragma_function_list",
+            ),
+            (
+                "SELECT title FROM books WHERE id IN (SELECT cid FROM pragma_table_info)",
+                "pragma_table_info",
+            ),
+        ] {
+            assert_eq!(
+                refused(Dialect::Sqlite, sql),
+                Refusal::FunctionNotAllowed(name.into()),
+                "{sql}"
+            );
+        }
+        // The same letters in a string are text, and the other two
+        // databases have no such tables.
+        assert!(read(Dialect::Sqlite, "SELECT 'pragma_table_info'").is_ok());
+        for dialect in [Dialect::Postgres, Dialect::MySql] {
+            let sql = "SELECT cid FROM pragma_table_info";
+            assert!(read(dialect, sql).is_ok(), "{dialect:?}");
+        }
+    }
+
     /// A name with a parenthesis after it is taken for a call wherever it
     /// stands: the check errs toward refusing.
     #[test]
```

- [ ] **Step 2: Run it to see it fail**

Run: `cargo test --locked -p tabletist-db --lib agent::tests::a_pragma_read`

Expected: FAIL, at the first statement of the list.

```
Sqlite let through: SELECT * FROM pragma_table_info WHERE arg = 'customers'
test result: FAILED. 0 passed; 1 failed
```

- [ ] **Step 3: Refuse the name wherever it stands**

```diff
diff --git a/crates/tabletist-db/src/agent.rs b/crates/tabletist-db/src/agent.rs
index 74f5141..6368bf1 100644
--- a/crates/tabletist-db/src/agent.rs
+++ b/crates/tabletist-db/src/agent.rs
@@ -626,6 +626,11 @@ fn words_read(dialect: Dialect, pieces: &[Piece]) -> Result<(), Refusal> {
             },
             Piece::Open | Piece::Other => continue,
         };
+        // A pragma read as a table is one by its name alone, whatever its
+        // quotes: SQLite asks for no parenthesis after it.
+        if pragma_table(dialect, text) {
+            return Err(Refusal::FunctionNotAllowed(text.clone()));
+        }
         let next = pieces.get(index + 1);
         // A name with a parenthesis after it is a call, in an expression
         // or in a table's place, whatever its quotes.
@@ -670,6 +675,20 @@ fn locks_after(next: Option<&Piece>) -> bool {
     )
 }
 
+/// Whether `name` is one of SQLite's table-valued pragmas,
+/// `pragma_table_info` and the rest. A `PRAGMA` statement is refused, and
+/// this is its other spelling, which SQLite reads as a table: in a `FROM`
+/// with no parenthesis after it, its argument a condition on the hidden
+/// column `arg`. So a column that is named so is refused with it.
+fn pragma_table(dialect: Dialect, name: &str) -> bool {
+    match dialect {
+        Dialect::Sqlite => name
+            .get(..7)
+            .is_some_and(|start| start.eq_ignore_ascii_case("pragma_")),
+        Dialect::Postgres | Dialect::MySql => false,
+    }
+}
+
 /// Refuses a function on the dialect's list, by its name without its
 /// schema: `pg_catalog.pg_sleep(1)` is `pg_sleep`.
 fn function_allowed(dialect: Dialect, name: &str) -> Result<(), Refusal> {
@@ -779,10 +798,9 @@ fn effects(dialect: Dialect) -> (&'static [&'static str], &'static [&'static str
                 "edit",
                 "fts3_tokenizer",
             ],
-            // The table-valued forms of the pragmas, `pragma_table_info`
-            // and the rest: a `PRAGMA` statement is refused, and so is its
-            // other spelling.
-            &["pragma_"],
+            // The table-valued pragmas are `pragma_table`'s: they need no
+            // parenthesis to be read.
+            &[],
         ),
     }
 }
```

- [ ] **Step 4: Run the tests**

Run: `cargo test --locked -p tabletist-db --lib agent::`

Expected: `45 passed`. The tests that were there pass unchanged: `SELECT * FROM pragma_table_info('customers')` is refused as before, by the new rule.

- [ ] **Step 5: Commit**

```bash
cargo fmt --all
cargo clippy --locked -p tabletist-db --all-targets -- -D warnings
git add crates/tabletist-db/src/agent.rs
git commit -m "Refuse a pragma an agent reads as a table, with or without a parenthesis"
```

---

### Task 2: The agent's fence

`Fence::Agent`, what it lets SQLite do, and the run behind it. One task and one commit: the fence's record of what it denied has no reader until the run reads it, and clippy does not let dead code through.

`Fences` was one shared byte. It is now that byte and the first refusal of an agent's fence, behind one `Arc`. The authorizer calls `Fences::ask`, which answers as `authorize` does and keeps what `Fence::Agent` denied. A fence that goes up starts with nothing denied.

In `sqlite.rs` the change is three things: the authorizer asks `ask`, a statement that comes with values (an agent's) goes behind `Fence::Agent` where a script's goes behind `Fence::Script`, and a statement that failed while the fence held a refusal answers that refusal.

**Files:**
- Modify: `crates/tabletist-db/src/sqlite/fence.rs`, `sqlite.rs`, `error.rs`, `agent.rs`
- Test: `crates/tabletist-db/tests/agent.rs`, and `fence.rs`'s and `sqlite.rs`'s own

- [ ] **Step 1: Write the failing tests**

Three kinds. The fence's own, on actions as SQLite hands them over. One in `sqlite.rs` that runs statements past the check, as the PostgreSQL and MySQL drivers' `past_the_refusal` tests do: after Task 1 no statement is known that passes the check and meets the fence, so this is how the fence is shown to hold. And three in `tests/agent.rs` that pass before this task as after it: they pin what the fence must not break (the shapes of an ordinary read, SQLite's full-text and R*Tree tables on a session that reads them for the first time) and what SQLite already refuses in a view.

```diff
diff --git a/crates/tabletist-db/src/sqlite.rs b/crates/tabletist-db/src/sqlite.rs
index a04c2b0..b6c5f36 100644
--- a/crates/tabletist-db/src/sqlite.rs
+++ b/crates/tabletist-db/src/sqlite.rs
@@ -2793,6 +2793,117 @@ mod tests {
         assert_eq!(bound(&Param::Text("5".into())), Sqlite::Text("5".into()));
     }
 
+    use crate::agent::Refusal;
+    use crate::sql;
+
+    /// What an agent's one statement answers when it is run past the
+    /// check, as if the check had missed it: its outcome, behind the
+    /// agent's fence.
+    async fn past_the_check(conn: &Conn, text: &str) -> StatementOutcome {
+        let fences = conn.fences.clone();
+        let texts = vec![text.to_owned()];
+        let outcome = conn
+            .run(move |connection| {
+                let sent = Sent {
+                    params: &[],
+                    timeout: Duration::from_secs(10),
+                };
+                let (mode, stop) = (ScriptMode::ReadOnly, StopFlag::new());
+                script(connection, &fences, &texts, 10, mode, &stop, Some(sent))
+            })
+            .await
+            .unwrap();
+        let [result] = outcome.results.as_slice() else {
+            panic!("{text}: {outcome:?}");
+        };
+        result.outcome.clone()
+    }
+
+    /// The check refuses each of these on the statement's text. Past it,
+    /// SQLite's own reading refuses them, and the agent is told what by
+    /// name where SQLite would only say "not authorized".
+    #[tokio::test]
+    async fn what_the_check_missed_is_refused_where_sqlite_reads_it() {
+        let (conn, _dir) = fixture().await;
+        let not_allowed = |what: &str| Refusal::NotAllowed(what.into());
+        for (text, refusal) in [
+            // A pragma read as a table, with no parenthesis for a name
+            // list to find.
+            (
+                "SELECT name FROM pragma_table_info WHERE arg = 'users'",
+                not_allowed("PRAGMA table_info"),
+            ),
+            (
+                "SELECT * FROM pragma_table_info('users')",
+                not_allowed("PRAGMA table_info"),
+            ),
+            (
+                "SELECT * FROM pragma_database_list",
+                not_allowed("PRAGMA database_list"),
+            ),
+            (
+                "SELECT * FROM pragma_optimize",
+                not_allowed("PRAGMA optimize"),
+            ),
+            ("PRAGMA query_only = 0", not_allowed("PRAGMA query_only")),
+            ("PRAGMA table_info(users)", not_allowed("PRAGMA table_info")),
+            (
+                "SELECT load_extension('nowhere')",
+                Refusal::FunctionNotAllowed("load_extension".into()),
+            ),
+            // By the name SQLite knows the function under, whatever the
+            // letters it was written in.
+            (
+                "SELECT id FROM users WHERE LOAD_EXTENSION('nowhere') IS NULL",
+                Refusal::FunctionNotAllowed("load_extension".into()),
+            ),
+            ("ATTACH DATABASE ':memory:' AS other", not_allowed("ATTACH")),
+            ("SAVEPOINT mine", not_allowed("a transaction statement")),
+            ("COMMIT", not_allowed("a transaction statement")),
+            (
+                "CREATE TABLE notes (body TEXT)",
+                not_allowed("a statement that is not a read"),
+            ),
+            ("ANALYZE", not_allowed("a statement that is not a read")),
+        ] {
+            let outcome = past_the_check(&conn, text).await;
+            let expected = StatementOutcome::Error {
+                error: Error::Agent(refusal),
+                position: None,
+            };
+            assert_eq!(outcome, expected, "{text}");
+        }
+        // A write the fence lets SQLite prepare, as R*Tree's own are, is
+        // the session's to refuse when it runs.
+        let outcome = past_the_check(&conn, "DELETE FROM users").await;
+        let StatementOutcome::Error { error, .. } = &outcome else {
+            panic!("{outcome:?}");
+        };
+        assert!(error.to_string().contains("readonly"), "{error}");
+        // A read is read.
+        let outcome = past_the_check(&conn, "SELECT count(*) FROM users").await;
+        assert!(
+            matches!(outcome, StatementOutcome::Rows { .. }),
+            "{outcome:?}"
+        );
+        // The fence is the agent's: the SQL editor's script reads a pragma
+        // as a table, as before, on the same session.
+        let editor = sql::statements(
+            Dialect::Sqlite,
+            "SELECT name FROM pragma_table_info('users')",
+        );
+        let texts: Vec<String> = editor.into_iter().map(|statement| statement.text).collect();
+        let outcome = conn
+            .run_script(texts, 10, ScriptMode::ReadOnly, &StopFlag::new())
+            .await
+            .unwrap();
+        let rows = matches!(
+            &outcome.results[0].outcome,
+            StatementOutcome::Rows { rows, .. } if !rows.is_empty()
+        );
+        assert!(rows, "{outcome:?}");
+    }
+
     #[test]
     fn a_wait_that_ran_out_is_told_by_its_result_code() {
         let failed = |code: Option<&str>| Error::Query {
diff --git a/crates/tabletist-db/src/sqlite/fence.rs b/crates/tabletist-db/src/sqlite/fence.rs
index ea21904..2527ecf 100644
--- a/crates/tabletist-db/src/sqlite/fence.rs
+++ b/crates/tabletist-db/src/sqlite/fence.rs
@@ -270,6 +270,168 @@ mod tests {
         }
     }
 
+    #[test]
+    fn an_agents_statement_may_only_read() {
+        let pragma = |name, value| action(ffi::SQLITE_PRAGMA, Some(name), value);
+        let function = |name| action(ffi::SQLITE_FUNCTION, None, Some(name));
+        let through_a_view = Action {
+            source: Some("recent_orders"),
+            ..action(ffi::SQLITE_READ, Some("orders"), Some("email"))
+        };
+        let table = [
+            (action(ffi::SQLITE_SELECT, None, None), true),
+            (action(ffi::SQLITE_READ, Some("books"), Some("title")), true),
+            (through_a_view, true),
+            (action(ffi::SQLITE_RECURSIVE, None, None), true),
+            (function("count"), true),
+            (function("json_extract"), true),
+            // The check's list for SQLite, asked of what SQLite read: in
+            // whatever letters, and from inside a view too.
+            (function("load_extension"), false),
+            (function("LOAD_EXTENSION"), false),
+            (function("fts3_tokenizer"), false),
+            (function("readfile"), false),
+            (action(ffi::SQLITE_FUNCTION, None, None), false),
+            // What FTS5 asks as it reads, and no other pragma: not one
+            // without a value, which a filter may read.
+            (pragma("data_version", None), true),
+            (pragma("data_version", Some("1")), false),
+            (pragma("table_info", Some("customers")), false),
+            (pragma("database_list", None), false),
+            (pragma("optimize", None), false),
+            (pragma("integrity_check", None), false),
+            (pragma("query_only", None), false),
+            // What R*Tree prepares when a table connects. Running one is
+            // `query_only`'s to refuse.
+            (action(ffi::SQLITE_INSERT, Some("places_node"), None), true),
+            (action(ffi::SQLITE_DELETE, Some("places_node"), None), true),
+            (
+                action(ffi::SQLITE_UPDATE, Some("places_node"), Some("data")),
+                true,
+            ),
+            // Everything else, by not being on the list.
+            (action(ffi::SQLITE_TRANSACTION, Some("BEGIN"), None), false),
+            (
+                action(ffi::SQLITE_SAVEPOINT, Some("BEGIN"), Some("s")),
+                false,
+            ),
+            (action(ffi::SQLITE_ATTACH, Some("other.db"), None), false),
+            (action(ffi::SQLITE_DETACH, Some("other"), None), false),
+            (action(ffi::SQLITE_CREATE_TABLE, Some("t"), None), false),
+            (action(ffi::SQLITE_CREATE_TEMP_VIEW, Some("v"), None), false),
+            (action(ffi::SQLITE_DROP_TABLE, Some("books"), None), false),
+            (
+                action(ffi::SQLITE_ALTER_TABLE, Some("main"), Some("books")),
+                false,
+            ),
+            (action(ffi::SQLITE_ANALYZE, Some("books"), None), false),
+            (
+                action(ffi::SQLITE_REINDEX, Some("books_title"), None),
+                false,
+            ),
+            (
+                action(ffi::SQLITE_CREATE_VTABLE, Some("t"), Some("fts5")),
+                false,
+            ),
+            // A code SQLite does not have yet.
+            (action(9_999, None, None), false),
+        ];
+        for (action, allowed) in table {
+            let expected = if allowed {
+                Authorization::Allow
+            } else {
+                Authorization::Deny
+            };
+            assert_eq!(authorize(Fence::Agent, &action), expected, "{action:?}");
+            // `Fences::current` counts on this: what a filter may not do,
+            // an agent's statement may not do either.
+            if authorize(Fence::Filter, &action) == Authorization::Deny {
+                assert_eq!(expected, Authorization::Deny, "{action:?}");
+            }
+        }
+    }
+
+    #[test]
+    fn what_an_agents_statement_was_denied_is_kept_for_the_run_to_say() {
+        let fences = Fences::default();
+        let asked = fences.clone();
+        let pragma = action(ffi::SQLITE_PRAGMA, Some("table_info"), Some("customers"));
+        let function = action(ffi::SQLITE_FUNCTION, None, Some("load_extension"));
+        let read = action(ffi::SQLITE_READ, Some("books"), Some("title"));
+        {
+            let _fenced = fences.fence(Fence::Agent);
+            assert_eq!(asked.ask(&read), Authorization::Allow);
+            assert_eq!(fences.refused(), None);
+            assert_eq!(asked.ask(&pragma), Authorization::Deny);
+            // The first denial is the one that is told.
+            assert_eq!(asked.ask(&function), Authorization::Deny);
+            assert_eq!(
+                fences.refused(),
+                Some(Refusal::NotAllowed("PRAGMA table_info".into()))
+            );
+        }
+        // A fence that goes up starts with nothing denied.
+        {
+            let _fenced = fences.fence(Fence::Agent);
+            assert_eq!(fences.refused(), None);
+            assert_eq!(asked.ask(&function), Authorization::Deny);
+            assert_eq!(
+                fences.refused(),
+                Some(Refusal::FunctionNotAllowed("load_extension".into()))
+            );
+        }
+        // Only an agent's fence keeps one: a script's denial is SQLite's
+        // own error, as before.
+        let _fenced = fences.fence(Fence::Script);
+        let begin = action(ffi::SQLITE_TRANSACTION, Some("BEGIN"), None);
+        assert_eq!(asked.ask(&begin), Authorization::Deny);
+        assert_eq!(fences.refused(), None);
+    }
+
+    #[test]
+    fn a_denial_is_named_by_what_sqlite_was_asked() {
+        let named = |code, first, second| refusal(&action(code, first, second));
+        let not_allowed = |what: &str| Refusal::NotAllowed(what.into());
+        assert_eq!(
+            named(ffi::SQLITE_FUNCTION, None, Some("load_extension")),
+            Refusal::FunctionNotAllowed("load_extension".into())
+        );
+        assert_eq!(
+            named(ffi::SQLITE_PRAGMA, Some("table_info"), Some("customers")),
+            not_allowed("PRAGMA table_info")
+        );
+        assert_eq!(
+            named(ffi::SQLITE_ATTACH, Some("other.db"), None),
+            not_allowed("ATTACH")
+        );
+        assert_eq!(
+            named(ffi::SQLITE_DETACH, Some("other"), None),
+            not_allowed("DETACH")
+        );
+        for code in [ffi::SQLITE_TRANSACTION, ffi::SQLITE_SAVEPOINT] {
+            assert_eq!(
+                named(code, Some("BEGIN"), None),
+                not_allowed("a transaction statement")
+            );
+        }
+        // Anything else is a statement that is no read, whatever it is.
+        for code in [ffi::SQLITE_CREATE_TABLE, ffi::SQLITE_ANALYZE, 9_999] {
+            assert_eq!(
+                named(code, Some("t"), None),
+                not_allowed("a statement that is not a read")
+            );
+        }
+        // A name SQLite did not give.
+        assert_eq!(
+            named(ffi::SQLITE_PRAGMA, None, None),
+            not_allowed("a PRAGMA")
+        );
+        assert_eq!(
+            named(ffi::SQLITE_FUNCTION, None, None),
+            not_allowed("a function without a name")
+        );
+    }
+
     #[test]
     fn a_fence_is_up_until_its_guard_drops() {
         let fences = Fences::default();
@@ -285,8 +447,8 @@ mod tests {
     #[test]
     fn a_byte_that_is_no_fence_counts_as_the_strictest_one() {
         let fences = Fences::default();
-        fences.0.store(7, Ordering::SeqCst);
-        assert_eq!(fences.current(), Fence::Filter);
+        fences.0.up.store(7, Ordering::SeqCst);
+        assert_eq!(fences.current(), Fence::Agent);
     }
 
     // Only a debug build checks it.
diff --git a/crates/tabletist-db/tests/agent.rs b/crates/tabletist-db/tests/agent.rs
index 5346968..5c8541e 100644
--- a/crates/tabletist-db/tests/agent.rs
+++ b/crates/tabletist-db/tests/agent.rs
@@ -299,9 +299,18 @@ mod sqlite {
 
     /// A Bookshop of the test's own, on a session that cannot write.
     pub(super) async fn bookshop() -> (Connection, tempfile::TempDir) {
+        bookshop_with("").await
+    }
+
+    /// A Bookshop to which `more` was added before its session opened.
+    async fn bookshop_with(more: &str) -> (Connection, tempfile::TempDir) {
         let dir = tempfile::tempdir().unwrap();
         let path = dir.path().join("bookshop.db");
         tabletist_db::fixtures::write_sqlite_bookshop(&path).unwrap();
+        rusqlite::Connection::open(&path)
+            .unwrap()
+            .execute_batch(more)
+            .unwrap();
         let connection = Connection::connect_with(
             &ConnectSpec::sqlite(&path),
             &Secrets::default(),
@@ -369,6 +378,100 @@ mod sqlite {
         assert_eq!(found, [[text("where?"), text("Lanterns")]]);
     }
 
+    /// The check reads an agent's text, and a view's text is the
+    /// database's. SQLite reads both, and with the session's schema not
+    /// trusted it refuses a view that reads a pragma or calls a function
+    /// that is not harmless, for an agent as for anyone. So a view is no
+    /// way around what an agent's own statement may not do.
+    #[tokio::test]
+    async fn what_a_view_may_not_do_is_sqlites_to_refuse() {
+        let (connection, _dir) = bookshop_with(
+            "CREATE VIEW customer_columns AS SELECT name FROM pragma_table_info('customers');
+             CREATE VIEW tokenizers AS SELECT fts3_tokenizer('simple') AS found;",
+        )
+        .await;
+        for (sql, what) in [
+            ("SELECT name FROM customer_columns", "pragma_table_info"),
+            ("SELECT found FROM tokenizers", "fts3_tokenizer"),
+            (
+                "SELECT title FROM books WHERE id IN (SELECT count(*) FROM customer_columns)",
+                "pragma_table_info",
+            ),
+        ] {
+            // The text names a view and nothing else, so the check passes.
+            let error = asked(&connection, sql, &[], LIMITS).await.unwrap_err();
+            let said = error.to_string();
+            assert!(said.starts_with("unsafe use of "), "{sql}: {said}");
+            assert!(said.contains(what), "{sql}: {said}");
+        }
+        // The session reads on, and nothing changed.
+        the_bookshop_is_stocked(&connection, "sqlite").await;
+    }
+
+    /// The fence lists what a read does. SQLite's own full-text and R*Tree
+    /// tables do more than read columns when they are first opened, and
+    /// are read all the same.
+    #[tokio::test]
+    async fn sqlites_own_virtual_tables_are_read_for_an_agent() {
+        let (connection, _dir) = bookshop_with(
+            "CREATE VIRTUAL TABLE blurbs USING fts5(body);
+             INSERT INTO blurbs VALUES ('A salt road to the sea'), ('Notes on bread');
+             CREATE VIRTUAL TABLE shelves USING rtree(id, left, right);
+             INSERT INTO shelves VALUES (1, 0, 2), (2, 5, 9);",
+        )
+        .await;
+        let found = read(
+            &connection,
+            "SELECT body FROM blurbs WHERE blurbs MATCH ?",
+            &[Param::Text("salt".into())],
+            "sqlite",
+        )
+        .await;
+        assert_eq!(found, [[text("A salt road to the sea")]]);
+        let found = read(
+            &connection,
+            "SELECT id FROM shelves WHERE left >= ? ORDER BY id",
+            &[Param::Int(1)],
+            "sqlite",
+        )
+        .await;
+        assert_eq!(found, [[Value::Int(2)]]);
+    }
+
+    /// What an ordinary read asks of SQLite beside its tables' columns is
+    /// on the fence's list.
+    #[tokio::test]
+    async fn the_shapes_of_a_read_pass_sqlites_own_reading() {
+        let (connection, _dir) = bookshop().await;
+        for (sql, rows) in [
+            // A view, and an aggregate over a join.
+            ("SELECT email FROM recent_orders", 7),
+            (
+                "SELECT a.name, count(*), max(b.price) FROM books b \
+                 JOIN authors a ON a.id = b.author_id GROUP BY a.name",
+                6,
+            ),
+            (
+                "WITH RECURSIVE n(i) AS (SELECT 1 UNION ALL SELECT i + 1 FROM n WHERE i < 3) \
+                 SELECT i FROM n",
+                3,
+            ),
+            ("SELECT value FROM json_each('[1, 2]')", 2),
+            (
+                "SELECT title FROM books WHERE title LIKE 'The %' ORDER BY title COLLATE NOCASE",
+                3,
+            ),
+            (
+                "SELECT lower(title), length(title), date(published) FROM books WHERE id = 3",
+                1,
+            ),
+            ("EXPLAIN QUERY PLAN SELECT title FROM books", 1),
+        ] {
+            let answered = read(&connection, sql, &[], "sqlite").await;
+            assert_eq!(answered.len(), rows, "{sql}");
+        }
+    }
+
     /// SQLite does not ask the progress handler while it waits for another
     /// connection's lock on the file, so the run holds that wait to its own
     /// time too: a read behind a writer is ended when its time is up, and
```

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test --locked -p tabletist-db --lib sqlite::`

Expected: it does not compile. The fence, its record and the error are not there:

```
error[E0599]: no variant, associated function, or constant named `Agent` found for enum `fence::Fence` in the current scope
error[E0599]: no method named `ask` found for struct `fence::Fences` in the current scope
error[E0599]: no method named `refused` found for struct `fence::Fences` in the current scope
error[E0425]: cannot find function `refusal` in this scope
error[E0599]: no variant, associated function, or constant named `Agent` found for enum `error::Error` in the current scope
```

`cargo test --locked -p tabletist-db --test agent sqlite` passes already, with `9 passed`: its three new tests are the ones that hold before and after.

- [ ] **Step 3: The fence, and the run behind it**

```diff
diff --git a/crates/tabletist-db/src/agent.rs b/crates/tabletist-db/src/agent.rs
index 6368bf1..537e700 100644
--- a/crates/tabletist-db/src/agent.rs
+++ b/crates/tabletist-db/src/agent.rs
@@ -691,7 +691,7 @@ fn pragma_table(dialect: Dialect, name: &str) -> bool {
 
 /// Refuses a function on the dialect's list, by its name without its
 /// schema: `pg_catalog.pg_sleep(1)` is `pg_sleep`.
-fn function_allowed(dialect: Dialect, name: &str) -> Result<(), Refusal> {
+pub(crate) fn function_allowed(dialect: Dialect, name: &str) -> Result<(), Refusal> {
     let lower = name.to_ascii_lowercase();
     let (names, prefixes) = effects(dialect);
     if names.contains(&lower.as_str()) || prefixes.iter().any(|prefix| lower.starts_with(prefix)) {
diff --git a/crates/tabletist-db/src/error.rs b/crates/tabletist-db/src/error.rs
index 1ae6cb0..cad651f 100644
--- a/crates/tabletist-db/src/error.rs
+++ b/crates/tabletist-db/src/error.rs
@@ -67,6 +67,11 @@ pub enum Error {
     /// session is closed.
     #[error("the script ended its transaction, so the session was closed")]
     LeftTransaction,
+    /// Why an agent's statement was refused where the database itself read
+    /// it: on SQLite, by the authorizer. The check's own refusals never
+    /// become errors. Nothing ran.
+    #[error("{0}")]
+    Agent(crate::agent::Refusal),
     #[error("the operation timed out")]
     Timeout,
     #[error("the connection was lost: {0}")]
diff --git a/crates/tabletist-db/src/sqlite.rs b/crates/tabletist-db/src/sqlite.rs
index b6c5f36..4d43bed 100644
--- a/crates/tabletist-db/src/sqlite.rs
+++ b/crates/tabletist-db/src/sqlite.rs
@@ -26,7 +26,7 @@ use crate::{
 mod fence;
 mod write;
 
-use fence::{Fence, Fences, authorize};
+use fence::{Fence, Fences};
 
 /// An open SQLite database.
 pub(crate) struct Conn {
@@ -399,14 +399,24 @@ fn statements(
         }
         let started = Instant::now();
         // Only the script's own text is fenced: the checks above and the
-        // rollback after the run are the app's.
+        // rollback after the run are the app's. An agent's statement has
+        // a fence of its own, which lists what a read does.
+        let fence = match params {
+            Some(_) => Fence::Agent,
+            None => Fence::Script,
+        };
         let result = {
-            let _fenced = fences.fence(Fence::Script);
+            let _fenced = fences.fence(fence);
             statement(connection, text, limit, params)
         };
         let result = match result {
             Ok(result) => result,
-            Err(error) => crate::script::statement_failed(error, None)?,
+            Err(error) => {
+                // SQLite says "not authorized" of what a fence denied. An
+                // agent is told what that was.
+                let error = fences.refused().map_or(error, Error::Agent);
+                crate::script::statement_failed(error, None)?
+            }
         };
         let cancelled = result == StatementOutcome::Cancelled;
         outcome.stopped |= cancelled;
@@ -807,10 +817,8 @@ impl Conn {
             // it may do depends on whose text it is (see `Fence`).
             let fences = Fences::default();
             let asked = fences.clone();
-            tabletist_sqlite_ffi::set_authorizer(&connection, move |action| {
-                authorize(asked.current(), action)
-            })
-            .map_err(map_error)?;
+            tabletist_sqlite_ffi::set_authorizer(&connection, move |action| asked.ask(action))
+                .map_err(map_error)?;
             Ok((connection, fences, journal_mode))
         };
         let (connection, fences, journal_mode) = tokio::task::spawn_blocking(opened)
diff --git a/crates/tabletist-db/src/sqlite/fence.rs b/crates/tabletist-db/src/sqlite/fence.rs
index 2527ecf..4c0d510 100644
--- a/crates/tabletist-db/src/sqlite/fence.rs
+++ b/crates/tabletist-db/src/sqlite/fence.rs
@@ -1,10 +1,11 @@
 //! The fences around text that is not the app's own. SQLite asks a
 //! connection's authorizer whenever it prepares a statement, after its own
-//! parse, so what a SQL editor script or a table's raw WHERE may do is
-//! decided here on what SQLite read, not on what our tokenizer made of the
-//! text. `Conn::open` installs the authorizer; a fence goes up around a
-//! script's statement and around a page or count query, and is down for
-//! everything the app runs itself.
+//! parse, so what a SQL editor script, a table's raw WHERE or an agent's
+//! statement may do is decided here on what SQLite read, not on what our
+//! tokenizer made of the text. `Conn::open` installs the authorizer; a
+//! fence goes up around a script's statement, around a page or count query
+//! and around an agent's statement, and is down for everything the app
+//! runs itself.
 //!
 //! The authorizer is `tabletist-sqlite-ffi`'s, not rusqlite's, which panics
 //! on a name that is not UTF-8, whatever the fence. Here such a name arrives
@@ -12,12 +13,15 @@
 //! for a name a fence refuses: those are ASCII, and so is every pragma's
 //! name that SQLite knows.
 
-use std::sync::Arc;
 use std::sync::atomic::{AtomicU8, Ordering};
+use std::sync::{Arc, Mutex};
 
 use rusqlite::ffi;
 use tabletist_sqlite_ffi::{Action, Authorization};
 
+use crate::Dialect;
+use crate::agent::{Refusal, function_allowed};
+
 /// Whose text SQLite is preparing, which decides what it may do.
 #[derive(Debug, Clone, Copy, PartialEq, Eq)]
 #[repr(u8)]
@@ -28,6 +32,8 @@ pub(super) enum Fence {
     Script = 1,
     /// A table's page or count, which holds the raw WHERE.
     Filter = 2,
+    /// A statement an agent sent, which passed `agent::check_read`.
+    Agent = 3,
 }
 
 /// What SQLite may do for text behind `fence`. Asked after SQLite's own
@@ -81,6 +87,29 @@ pub(super) fn authorize(fence: Fence, action: &Action<'_>) -> Authorization {
             }
             _ => true,
         },
+        // An agent's statement is one read, by a check that reads its
+        // text. SQLite reads it again here, and what the two make of it
+        // can differ: a view of the database's own can call a function or
+        // read a pragma that the text never names. So this fence lists
+        // what a read does and refuses the rest, where the other two list
+        // what to refuse.
+        Fence::Agent => match action.code {
+            ffi::SQLITE_SELECT | ffi::SQLITE_READ | ffi::SQLITE_RECURSIVE => true,
+            // The check's list of functions with effects, asked of the
+            // names SQLite found.
+            ffi::SQLITE_FUNCTION => action
+                .second
+                .is_some_and(|name| function_allowed(Dialect::Sqlite, name).is_ok()),
+            // What FTS5 asks for as it reads, and no other pragma: one
+            // read as a table (`pragma_table_info`) arrives here too, with
+            // its argument as its value or with none.
+            ffi::SQLITE_PRAGMA => action.second.is_none() && pragma_is(action, "data_version"),
+            // The write statements R*Tree prepares when a table connects,
+            // which a SELECT does not run. Running one is `query_only`'s
+            // to refuse, as behind a filter.
+            ffi::SQLITE_INSERT | ffi::SQLITE_UPDATE | ffi::SQLITE_DELETE => true,
+            _ => false,
+        },
     };
     if allowed {
         Authorization::Allow
@@ -97,30 +126,87 @@ fn pragma_is(action: &Action<'_>, name: &str) -> bool {
         .is_some_and(|asked| asked.eq_ignore_ascii_case(name))
 }
 
-/// Which fence is up, shared with the authorizer SQLite calls.
+/// What an agent is told of an action its fence denied: a function by its
+/// name, a pragma by its own, and the rest by what kind of statement asks
+/// for it.
+fn refusal(action: &Action<'_>) -> Refusal {
+    let what = match (action.code, action.first, action.second) {
+        (ffi::SQLITE_FUNCTION, _, Some(name)) => {
+            return Refusal::FunctionNotAllowed(name.to_owned());
+        }
+        (ffi::SQLITE_FUNCTION, _, None) => "a function without a name".to_owned(),
+        (ffi::SQLITE_PRAGMA, Some(name), _) => format!("PRAGMA {name}"),
+        (ffi::SQLITE_PRAGMA, None, _) => "a PRAGMA".to_owned(),
+        (ffi::SQLITE_ATTACH, ..) => "ATTACH".to_owned(),
+        (ffi::SQLITE_DETACH, ..) => "DETACH".to_owned(),
+        (ffi::SQLITE_TRANSACTION | ffi::SQLITE_SAVEPOINT, ..) => {
+            "a transaction statement".to_owned()
+        }
+        _ => "a statement that is not a read".to_owned(),
+    };
+    Refusal::NotAllowed(what)
+}
+
+/// Which fence is up, shared with the authorizer SQLite calls, and what
+/// an agent's fence denied since it went up.
 #[derive(Clone, Default)]
-pub(super) struct Fences(Arc<AtomicU8>);
+pub(super) struct Fences(Arc<Shared>);
+
+#[derive(Default)]
+struct Shared {
+    up: AtomicU8,
+    refused: Mutex<Option<Refusal>>,
+}
 
 impl Fences {
     /// Puts `fence` up until the returned value drops. Fences do not nest:
     /// the drop takes down whatever is up.
     pub(super) fn fence(&self, fence: Fence) -> Fenced<'_> {
         debug_assert_eq!(self.current(), Fence::Off, "fences do not nest");
-        self.0.store(fence as u8, Ordering::SeqCst);
+        if let Ok(mut refused) = self.0.refused.lock() {
+            *refused = None;
+        }
+        self.0.up.store(fence as u8, Ordering::SeqCst);
         Fenced(self)
     }
 
     /// The fence that is up, for the authorizer.
     pub(super) fn current(&self) -> Fence {
-        match self.0.load(Ordering::SeqCst) {
+        match self.0.up.load(Ordering::SeqCst) {
             0 => Fence::Off,
             1 => Fence::Script,
-            // Only the three are ever stored. A byte that is none of them
-            // counts as the strictest: `Filter` denies all that `Script`
-            // does, so nothing runs unfenced over it.
-            _ => Fence::Filter,
+            2 => Fence::Filter,
+            // Only the four are ever stored. A byte that is none of them
+            // counts as the strictest: `Agent` denies all that `Filter`
+            // does, and `Filter` all that `Script` does, so nothing runs
+            // unfenced over it.
+            _ => Fence::Agent,
         }
     }
+
+    /// The authorizer's answer for the fence that is up. SQLite says only
+    /// "not authorized" of a statement it refuses, so what an agent's
+    /// fence denies is kept, the first of it, for the run to say.
+    pub(super) fn ask(&self, action: &Action<'_>) -> Authorization {
+        let fence = self.current();
+        let answer = authorize(fence, action);
+        if fence == Fence::Agent
+            && answer == Authorization::Deny
+            && let Ok(mut refused) = self.0.refused.lock()
+        {
+            refused.get_or_insert_with(|| refusal(action));
+        }
+        answer
+    }
+
+    /// What an agent's fence denied since the last fence went up.
+    pub(super) fn refused(&self) -> Option<Refusal> {
+        self.0
+            .refused
+            .lock()
+            .ok()
+            .and_then(|refused| refused.clone())
+    }
 }
 
 /// Takes the fence down when dropped, on every path.
@@ -128,7 +214,7 @@ pub(super) struct Fenced<'a>(&'a Fences);
 
 impl Drop for Fenced<'_> {
     fn drop(&mut self) {
-        self.0.0.store(Fence::Off as u8, Ordering::SeqCst);
+        self.0.0.up.store(Fence::Off as u8, Ordering::SeqCst);
     }
 }
 
```

- [ ] **Step 4: Run the tests**

Run: `cargo test --locked -p tabletist-db --lib sqlite::`, then `cargo test --locked -p tabletist-db --test agent --test sqlite`

Expected: `53 passed` for the library's `sqlite::` tests (the fence's three and the run's one are new), `24 passed` for `tests/agent.rs`, and `102 passed` for `tests/sqlite.rs`, which is the SQL editor's run and a table's filter on SQLite and has not changed.

- [ ] **Step 5: Commit**

```bash
cargo fmt --all
cargo clippy --locked --workspace --all-targets -- -D warnings
git add crates/tabletist-db/src crates/tabletist-db/tests/agent.rs
git commit -m "Fence an agent's statement on SQLite by what SQLite reads"
```

---

### Task 3: The whole workspace, and the pull request

**Files:** none.

- [ ] **Step 1: Run every check `AGENTS.md` names**

```bash
cargo fmt --all --check
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo test --locked --workspace --all-targets
RUSTDOCFLAGS='-D warnings' cargo doc --locked --workspace --no-deps
```

Expected: all four pass. The library has `466 passed; 0 failed; 1 ignored`, and `tests/agent.rs` has `24 passed`. `Error` gained a variant: clippy over the whole workspace is what shows that no `match` on it in the app needed an arm.

- [ ] **Step 2: Open the pull request**

Its description says:

- what the fence lets SQLite do for an agent's statement, and that everything else is refused by name;
- the miss in the check that the draft found, with the three statements, and that it is closed in the check too;
- that a view cannot reach a pragma or an unsafe function, by a setting the sessions already had, and the test that pins it;
- that the SQL editor's script and a table's filter are fenced as before;
- that Phase 1a is done with this, and what Phase 1b needs answered before it starts (open questions 5, 6, 9, 10 and 14 of the master plan).

No website page changes: no behaviour, setting, file or key a user sees is touched.

## Self-review

- **Against the master plan, B.2.** "On SQLite the authorizer sees every function call after SQLite's own parse, so there the list does not depend on our parser": Task 2, with `what_the_check_missed_is_refused_where_sqlite_reads_it` running `load_extension` past the check in two spellings. "SQLite: `load_extension`, and every table-valued `pragma_*`": the check missed the pragmas written without a parenthesis, which Task 1 closes and Task 2 backs.
- **Against the master plan, B.3, as far as Phase 1a goes.** `Fence::Agent` exists beside `Fence::Script` and `Fence::Filter` and is asked for every table and column an agent's statement reads. It denies none of them yet: that needs the connection's hidden tables, which are Phase 1c's.
- **Names.** `Fence::Agent`, `Fences::ask`, `Fences::refused`, `refusal`, `Error::Agent`, `pragma_table` and `function_allowed` are spelled the same in both tasks and in the tests.
- **What a reader might expect and will not find.** A statement that passes the check and is refused by the fence, end to end. Task 1 closed the only one known. The fence is tested on SQLite's own actions and by running statements past the check.
- **Every commit stands.** After Task 1 the check refuses more and nothing else changed. After Task 2 an agent's statement on SQLite has its own fence, and the SQL editor's and a filter's are as they were: `tests/sqlite.rs` was run at each.

## After the run

Run on 2026-10-13, inline, one commit for each task. Every step showed what it says above, and the files the tasks changed are byte for byte the draft's.

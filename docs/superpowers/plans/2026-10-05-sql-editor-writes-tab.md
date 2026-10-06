# SQL Editor Writes, Step 2, First Run: The Tab Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A SQL tab of a writable connection that is not production's can be switched to Read-write and then writes: a run that holds a write is one transaction, committed when every statement succeeded, and the Messages, Results and the footer say how it ended. On production nothing writes from the editor yet.

**Architecture:** `SqlTab` gains a `mode`, and `Workspace::run_mode` says which mode counts (Read-write only while the session can write and, in this run, is not production's). `App::run_sql` decides with `sql::kind` and sends `Command::RunSql` with a `ScriptMode`, which the backend passes to `Connection::run_script` (built in step 1). The views read `SqlRun::mode` and `ScriptOutcome::{end, rollback_warning, broken}`. The refused-write card moves from Results to the head of the Messages and says why its run was read-only. In `tabletist-db` only SQLite changes: three flags a script can set are put back after every run, and `locking_mode` is denied to a script.

**Tech Stack:** Rust 1.98, egui/eframe (the crmne fork), `rusqlite` 0.37. Spec: `docs/superpowers/specs/2026-10-05-sql-editor-writes-design.md` (this plan is the first of two runs of its step 2, "The tab").

---

## Before you start

- Cargo is `~/.cargo/bin/cargo` (the mise shim fails). Never point `CARGO_TARGET_DIR` at `/tmp`.
- The four checks, from `AGENTS.md`. "Run the four checks" below means these, and all four pass after every task:

```bash
~/.cargo/bin/cargo fmt --all --check
~/.cargo/bin/cargo clippy --locked --workspace --all-targets -- -D warnings
~/.cargo/bin/cargo test --locked --workspace --all-targets
RUSTDOCFLAGS='-D warnings' ~/.cargo/bin/cargo doc --locked --workspace --no-deps
```

- **What was run where this plan was written.** The plan's code comes from a draft that was built task by task on Linux, each task passing the four checks (2,354 tests at the end). Each task's tests were then run alone on the tree before it, to record the failure its "run it to see it fail" step expects, and the code's block after them gave exactly the draft's tree. The scenes of task 7 were rendered off screen and looked at (see "By hand" at the end for what that does not cover). A review of the plan found one fault in the draft, fixed with its test before the plan was finished: a commit that failed, followed by a rollback that could not undo everything, still said "Nothing was written."
- **Databases.** This run adds no PostgreSQL or MySQL test and changes none: in `tabletist-db` it touches SQLite only, whose tests ran. The read-write run on PostgreSQL and MySQL is step 1's, already run by CI. No server was reachable where the plan was written, so the app has not written to a real PostgreSQL or MySQL from the editor: that is in "By hand".
- **Platforms.** Nothing here is behind a `cfg`, so one platform's build is every platform's. Only Linux was built; CI builds and tests macOS and Windows.
- **The branch.** Work on `claude/sql-editor-writes-tab`, cut from `main` at `16d0fc1` (the merge of pull request #84). Before task 1, commit this plan on its own: `git add docs/superpowers/plans/2026-10-05-sql-editor-writes-tab.md && git commit -S -m "Plan the SQL editor's tab that writes" -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"`.
- The diffs are against the tree as the task before left it. Save a block to a file and `git apply` it, or make the change by hand. Each task has the tests' block first, which applies on its own, then the code's. A line number in a hunk header is where the draft had it and may have moved. After a block is applied, `~/.cargo/bin/cargo fmt --all --check` must still pass: the blocks are formatted.
- House rules that bite here: no em dashes anywhere; comments explain why, in the surrounding code's voice; a view pushes `Action`s and never changes application state; views draw text only through `TextRole`s; never log SQL text (`SqlTab`, `Statement` and `RunInFlight` print without it); do not weaken a lint or delete a test to get green. Task 6 replaces three tests of the old card and flips one assertion: it says which and why.
- No design or pixel conformance in any test. The design canvas has no artboard for a run that writes; what it gives is the frame (the badge as the tab's switch, the "Write blocked" card, the Components' menu). "What was decided" says where the draft was held against it. Compare by eye, with the scenes of task 7.
- Commits are signed, one per task, after its checks pass. If signing fails ("agent refused operation": 1Password is locked), do not commit unsigned: stage the task, tell the user, and go on once they have unlocked it. Subjects are plain sentences, each ending with:

      Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>

## What was decided

With the user, before the plan:

1. **The cut.** Step 2 is built in two runs. This one is everything but the production confirmation and `editor.sql_new_tab`. The spec's rule holds meanwhile ("No step ships a read-write run on production without its confirmation"): on a writable production connection the tab stays in Read-only. That is one place in the code, `Workspace::sql_writes`, which answers `Err(NoWrites::Unconfirmed)` where `Environment::confirms_writes()` holds. The next run builds the confirmation and takes that arm out, with the sentence "Read-write runs on a production connection are not available yet." (`UNCONFIRMED` in `src/ui/sql_editor.rs`), which the badge's menu, its tooltip and the card say meanwhile.
2. **The SQLite pragmas.** `ignore_check_constraints`, `recursive_triggers` and `legacy_alter_table` are put back after every run (a script may still set one for its own run). `locking_mode` with a value is denied to a script. Checked with SQLite 3.53 before deciding: all four take effect inside the run's transaction and outlive its `COMMIT`; and in a rollback journal an exclusive lock is not let go by `PRAGMA locking_mode = NORMAL`, only by the session's next read of the file.

Made while drafting, where the spec is silent or the code had to choose. Each is small, and each is the user's to overrule in review:

3. **A fourth wording of the card.** "Allow writes in this tab" sets the mode and runs nothing, so the card stays on screen for a tab that is now in Read-write. The spec's text for that state ("Its statements looked like reads") would be false of an `UPDATE`. The card then reads "This run was read-only", "It was sent before this tab could write, so PostgreSQL refused the UPDATE. Nothing changed.", with **Run in a read-write transaction**. Which of the two it is, is told from the run's statements: a run that holds a `sql::kind` `Write` was not taken for a run of reads.
4. **The card follows the state as it is now**, not as it was when the run was sent: its action must do something now. After a reconnect that came back read-only it is the read-only connection's card.
5. **"· rolled back" on a statement that returned rows** only where the statement looks like a write (`INSERT ... RETURNING`): a plain `SELECT` did no work to undo.
6. **Warnings are counted on a line only in a read-write run**, as the spec lists them there ("A read-only run reads as today").
7. **The footer of a commit that failed after MySQL had committed part** reads "partly committed", not "commit failed": that is what it left.
8. **A stop that came with every statement done** has no statement's line to say "Cancelled": the end's lines start with it.
9. **Results, with no rows to show, after a commit that failed** says what the Messages say of it ("The commit failed. Nothing was written.") in the error colour, never "Statement ran". And after a run MySQL committed by itself under a stop that came too late, Results says that the statement ran, not only "Cancelled": the run is written.
10. **A session closed because the script ended its own transaction** (`Error::LeftTransaction`) gets "Some or all of the run may be written." under its error; "The connection was lost during a read-write run." is said only of a lost connection.
11. **The badge keeps its pill** and gains a chevron where it is a menu. In Read-write it takes the warning tint, line and colour, and a pencil for the lock. The production tone is the next run's: no production tab is in Read-write in this one.
12. **The database's words stand twice in the Messages**: under the card, as the spec has them, and in the statement's own line, which is what names the line.
13. **The Messages tab counts a commit that failed** as an error, as it counts a statement's.
14. **The new SQLite tests are unit tests in `crates/tabletist-db/src/sqlite.rs`**, beside the fence's own, where the helpers they use are.
15. **Against the design canvas.** Read before the plan was finished: the "Error states, read-only connection" artboards (macOS and Omarchy), the SQL editor artboards, the Editor settings and the Components. The card's tint, lock, title, first sentence, the database's words under it and the "To write, turn off Open read-only" line under those are the artboard's. The app departs in three places. The artboard ends that line with **Edit connection** as a link (Omarchy: a line of key hints), and the app has a button under it, as its other state cards have. The artboard's "or allow writes for this tab" is not offered on a read-only connection: the spec rejects it. And the canvas draws the badge only as the read-only pill: its Read-write form (decision 11) is this plan's. The Components' menu rule "Disabled items stay visible" is what `MenuChoice::disabled` does.
16. **In the terminal look the new sentences are in lower case throughout**, the database's name and the statements' keywords too, as the Omarchy artboard writes "postgres refused the update"; and the read-only connection's card names the connection by its tag ("PROD blocks writes"). What a database itself said keeps its case.

## What this run leaves

- For the second run of step 2: the production confirmation in both looks, `SqlTab.asking` and `Action::ConfirmSqlRun`, the production tone of the badge, and `editor.sql_new_tab`.
- For step 3, as the spec has it: the question about a missing `WHERE`, stale table tabs, the tree's refresh after DDL, and the leaving guard. Until then closing a tab or a connection cancels a read-write run in flight without asking, and a table tab shows what it loaded until it is refreshed by hand.

## File map

| File | What changes |
|---|---|
| `crates/tabletist-db/src/sqlite.rs` | `set_session_pragmas` puts three more flags back; tests |
| `crates/tabletist-db/src/sqlite/fence.rs` | `Fence::Script` denies `locking_mode` with a value; tests |
| `src/model.rs` | `RunMode`, `NoWrites`, `SqlTab::mode`, `Workspace::{sql_writes, run_mode}`; `Action::{SetSqlMode, ToggleSqlMode, RunSqlAgain}`; the mode on `SqlRun` and `RunInFlight`; `SqlTab::{is_writing, ran_this_text, lost_writing}` |
| `src/app.rs` | `set_sql_mode`, the decision in `run_sql`, `send_run`, `run_sql_again`, `script_mode`, `opens_messages`; reducer tests |
| `src/backend.rs` | `Command::RunSql::mode`; a session a run left broken is closed after it answered |
| `src/ui/sql_editor.rs` | the badge and its menu, Run held back, the footer and the status line; headless tests |
| `src/ui/sql_results.rs` | the end's lines, "· rolled back", warnings, Results for a run that wrote, the three cards; headless tests |
| `src/ui/format.rs` | `refuses_writes` no longer counts the guard's refusal |
| `src/ui/keys.rs` | `Mod+Shift+M`, its row in the help, the card's letters on Omarchy |
| `src/ui/widgets.rs`, `src/ui/sidebar.rs` | `MenuChoice::disabled` |
| `src/testing.rs` | `reconnect_fake_as`, `write_outcome`, `done_outcome`, `refused_write` |
| `src/shots.rs` | scenes for review |
| `docs/superpowers/specs/*.md`, `README.md` | what the editor does now |

---

### Task 1: The SQLite settings a script must not leave behind

**Files:**
- Modify: `crates/tabletist-db/src/sqlite.rs` (`set_session_pragmas`, and its tests module)
- Modify: `crates/tabletist-db/src/sqlite/fence.rs` (`authorize`, and its tests module)

A script may set `ignore_check_constraints`, `recursive_triggers`, `legacy_alter_table` or `locking_mode`, in a read-only run too, and they stay set for the session: a later run that writes, and a grid's Save, would keep rows a CHECK forbids. This task comes first because it is what makes the later ones safe.

The facts it rests on (checked with SQLite 3.53): the four take effect at once, inside a transaction too, and outlive its `COMMIT`. `set_session_pragmas` already runs at connect, after every script on every path (`end_transaction`) and after a filter that failed, so three more statements there settle the three flags. `locking_mode` is different: in a rollback journal `PRAGMA locking_mode = NORMAL` does not let an exclusive lock go, only the session's next read of the file does, so it is denied to a script as `query_only` is. SQLite hands the authorizer a pragma's name without its schema and in the letters it was written in, and the value as the second argument, in every spelling (`= x`, `(x)`).

- [ ] **Step 1: Write the failing tests**

Apply this block. It holds the task's tests, and applies on the tree as the task before left it.

````diff
diff --git a/crates/tabletist-db/src/sqlite.rs b/crates/tabletist-db/src/sqlite.rs
index 3b6ecbe..65b1f0b 100644
--- a/crates/tabletist-db/src/sqlite.rs
+++ b/crates/tabletist-db/src/sqlite.rs
@@ -1418,6 +1426,155 @@ mod tests {
         assert_eq!(events_of(&conn, "probe").await, 1);
     }
 
+    /// The three flags a script may set for its own run, as the session
+    /// has them now: CHECK constraints ignored, triggers firing themselves,
+    /// and the old `ALTER TABLE ... RENAME`.
+    async fn flags(conn: &Conn) -> [i64; 3] {
+        conn.run(|connection| {
+            let one = |sql: &str| {
+                connection
+                    .query_row(sql, [], |row| row.get::<_, i64>(0))
+                    .map_err(map_error)
+            };
+            Ok([
+                one("PRAGMA ignore_check_constraints")?,
+                one("PRAGMA recursive_triggers")?,
+                one("PRAGMA legacy_alter_table")?,
+            ])
+        })
+        .await
+        .unwrap()
+    }
+
+    /// How many rows the probe table `checked` holds.
+    async fn checked_rows(conn: &Conn) -> i64 {
+        conn.run(|connection| {
+            connection
+                .query_row("SELECT count(*) FROM checked", [], |row| row.get(0))
+                .map_err(map_error)
+        })
+        .await
+        .unwrap()
+    }
+
+    #[tokio::test]
+    async fn a_flag_a_script_sets_ends_with_its_run() {
+        const FLAGS: [&str; 3] = [
+            "PRAGMA ignore_check_constraints = ON",
+            "PRAGMA recursive_triggers = ON",
+            "PRAGMA legacy_alter_table = ON",
+        ];
+        let (conn, _dir) = fixture_as(Access::Writable).await;
+        let made = write_unrefused(&conn, &["CREATE TABLE checked (n INTEGER CHECK (n > 0))"])
+            .await
+            .unwrap();
+        assert_eq!(made.end, ScriptEnd::Committed);
+        // Set in a run that only reads, and in one that writes: neither
+        // leaves the session with them.
+        for mode in [ScriptMode::ReadOnly, ScriptMode::Write] {
+            let script = FLAGS.iter().map(|text| (*text).to_owned()).collect();
+            let outcome = conn
+                .run_script(script, 10, mode, &StopFlag::new())
+                .await
+                .unwrap();
+            assert_eq!(outcome.results.len(), 3, "{mode:?}");
+            assert!(outcome.succeeded(), "{mode:?}: {outcome:?}");
+            assert_eq!(flags(&conn).await, [0, 0, 0], "{mode:?}");
+            assert_eq!(standing(&conn).await, (1, true), "{mode:?}");
+        }
+        // So the next run that writes keeps what the table's CHECK allows,
+        // and no more.
+        let refused = write_unrefused(&conn, &["INSERT INTO checked VALUES (-1)"])
+            .await
+            .unwrap();
+        assert!(
+            matches!(refused.results[0].outcome, StatementOutcome::Error { .. }),
+            "{:?}",
+            refused.results[0].outcome
+        );
+        assert_eq!(refused.end, ScriptEnd::RolledBack);
+        assert_eq!(checked_rows(&conn).await, 0);
+        // A script may still set one for its own run, which then keeps
+        // what it wrote under it.
+        let own = write_unrefused(&conn, &[FLAGS[0], "INSERT INTO checked VALUES (-1)"])
+            .await
+            .unwrap();
+        assert_eq!(own.end, ScriptEnd::Committed);
+        assert_eq!(checked_rows(&conn).await, 1);
+        assert_eq!(flags(&conn).await, [0, 0, 0]);
+        // A run that fails or is stopped puts them back as well.
+        let failed = write_unrefused(&conn, &[FLAGS[0], FLAGS[1], "SELECT nope"])
+            .await
+            .unwrap();
+        assert_eq!(failed.end, ScriptEnd::RolledBack);
+        assert_eq!(flags(&conn).await, [0, 0, 0]);
+    }
+
+    /// The session's `locking_mode`.
+    async fn locking_mode(conn: &Conn) -> String {
+        conn.run(|connection| {
+            connection
+                .query_row("PRAGMA locking_mode", [], |row| row.get(0))
+                .map_err(map_error)
+        })
+        .await
+        .unwrap()
+    }
+
+    #[tokio::test]
+    async fn a_script_cannot_take_the_file_for_the_session() {
+        for mode in [ScriptMode::ReadOnly, ScriptMode::Write] {
+            for text in [
+                "PRAGMA locking_mode = EXCLUSIVE",
+                "PRAGMA main.locking_mode(exclusive)",
+                "PRAGMA LOCKING_MODE = EXCLUSIVE",
+            ] {
+                let (conn, dir) = fixture_as(Access::Writable).await;
+                let probe = "INSERT INTO events (kind) VALUES ('probe')";
+                // In a run that writes, with a change before it, which is
+                // what would have taken the lock.
+                let script: Vec<String> = match mode {
+                    ScriptMode::ReadOnly => vec![text.to_owned()],
+                    ScriptMode::Write => vec![probe.to_owned(), text.to_owned()],
+                };
+                let outcome = conn
+                    .run_script(script, 10, mode, &StopFlag::new())
+                    .await
+                    .unwrap();
+                let last = outcome.results.last().unwrap();
+                assert!(
+                    matches!(
+                        &last.outcome,
+                        StatementOutcome::Error {
+                            error: Error::Query { message, .. },
+                            ..
+                        } if message.contains("not authorized")
+                    ),
+                    "{mode:?} {text}: {:?}",
+                    last.outcome
+                );
+                assert_eq!(outcome.end, ScriptEnd::RolledBack, "{mode:?} {text}");
+                assert_eq!(locking_mode(&conn).await, "normal", "{mode:?} {text}");
+                assert_eq!(events_of(&conn, "probe").await, 0, "{mode:?} {text}");
+                // Another program can write to the file right away.
+                let other = rusqlite::Connection::open(dir.path().join("fixture.db")).unwrap();
+                other
+                    .execute_batch("INSERT INTO events (kind) VALUES ('other')")
+                    .unwrap();
+            }
+        }
+        // Asking what the mode is stays a read like any other.
+        let (conn, _dir) = fixture_as(Access::ReadOnly).await;
+        let asked = run_unrefused(&conn, &["PRAGMA locking_mode"])
+            .await
+            .unwrap();
+        assert!(
+            matches!(asked.results[0].outcome, StatementOutcome::Rows { .. }),
+            "{:?}",
+            asked.results[0].outcome
+        );
+    }
+
     /// The session's `query_only` and whether it is out of a transaction.
     async fn standing(conn: &Conn) -> (i64, bool) {
         conn.run(|connection| {
diff --git a/crates/tabletist-db/src/sqlite/fence.rs b/crates/tabletist-db/src/sqlite/fence.rs
index e261a48..ea21904 100644
--- a/crates/tabletist-db/src/sqlite/fence.rs
+++ b/crates/tabletist-db/src/sqlite/fence.rs
@@ -177,6 +182,32 @@ mod tests {
                 false,
             ),
             (Fence::Script, pragma("query_only", None), true),
+            // The file's lock is not a script's to keep, in whatever
+            // letters; asking what the mode is changes nothing.
+            (
+                Fence::Script,
+                pragma("locking_mode", Some("EXCLUSIVE")),
+                false,
+            ),
+            (Fence::Script, pragma("LOCKING_MODE", Some("normal")), false),
+            (Fence::Script, pragma("locking_mode", None), true),
+            // A script may set these for its own run: the session's
+            // settings are put back after it (`set_session_pragmas`).
+            (
+                Fence::Script,
+                pragma("ignore_check_constraints", Some("ON")),
+                true,
+            ),
+            (
+                Fence::Script,
+                pragma("recursive_triggers", Some("ON")),
+                true,
+            ),
+            (
+                Fence::Script,
+                pragma("legacy_alter_table", Some("ON")),
+                true,
+            ),
             (Fence::Script, pragma("foreign_keys", Some("ON")), true),
             (Fence::Script, pragma("table_info", Some("users")), true),
             (Fence::Script, select, true),
````

- [ ] **Step 2: Run the tests to see them fail**

```bash
~/.cargo/bin/cargo test --locked -p tabletist-db --lib -- a_flag_a_script_sets a_script_cannot_take each_fence_allows
```

Expected: FAIL, 3 failed. The fence's table: `Script Action { code: 19, first: Some("locking_mode"), second: Some("EXCLUSIVE"), .. }` is `Allow`, not `Deny`. `a_flag_a_script_sets_ends_with_its_run`: the flags are `[1, 1, 1]`, not `[0, 0, 0]`, after the read-only run. `a_script_cannot_take_the_file_for_the_session`: the statement returned `Rows { .. [[Text("exclusive")]] .. }` where an error was expected.

- [ ] **Step 3: Write the code**

````diff
diff --git a/crates/tabletist-db/src/sqlite.rs b/crates/tabletist-db/src/sqlite.rs
index 3b6ecbe..65b1f0b 100644
--- a/crates/tabletist-db/src/sqlite.rs
+++ b/crates/tabletist-db/src/sqlite.rs
@@ -492,12 +492,20 @@ fn declared_columns(
 /// them off (the pragma does nothing inside a transaction, which is where
 /// a script runs), but a save's refusal of a child with no parent should
 /// not rest on how the library was built.
+///
+/// Three more are put back because they change what a later write keeps,
+/// a save's as much as a script's: a script may turn CHECK constraints
+/// off, let triggers fire themselves, or have `ALTER TABLE ... RENAME`
+/// leave the views and triggers that name the table alone. It may, for its
+/// own run; the flag ends with the run. (`locking_mode`, the fourth
+/// setting of that kind, is denied to a script: see `Fence::Script`.)
 fn set_session_pragmas(connection: &rusqlite::Connection) -> rusqlite::Result<()> {
     connection.busy_timeout(std::time::Duration::from_secs(5))?;
     connection.execute_batch(
         "PRAGMA query_only = ON; PRAGMA trusted_schema = OFF; PRAGMA case_sensitive_like = OFF; \
          PRAGMA full_column_names = OFF; PRAGMA short_column_names = ON; \
-         PRAGMA foreign_keys = ON;",
+         PRAGMA foreign_keys = ON; PRAGMA ignore_check_constraints = OFF; \
+         PRAGMA recursive_triggers = OFF; PRAGMA legacy_alter_table = OFF;",
     )
 }
 
diff --git a/crates/tabletist-db/src/sqlite/fence.rs b/crates/tabletist-db/src/sqlite/fence.rs
index e261a48..ea21904 100644
--- a/crates/tabletist-db/src/sqlite/fence.rs
+++ b/crates/tabletist-db/src/sqlite/fence.rs
@@ -64,13 +64,18 @@ pub(super) fn authorize(fence: Fence, action: &Action<'_>) -> Authorization {
         },
         // A write is left to `query_only`, whose error the app knows as a
         // refused write. What must not happen is the script leaving its
-        // transaction or lifting what refuses the write.
+        // transaction or lifting what refuses the write. Nor may it take
+        // the file for the session: an exclusive `locking_mode` outlives
+        // the run, and in a rollback journal setting it back does not let
+        // the lock go (only the session's next read of the file does), so
+        // every other program would be shut out of the file meanwhile.
+        // Inside the one transaction a script runs in it gains nothing.
         Fence::Script => match action.code {
             ffi::SQLITE_TRANSACTION | ffi::SQLITE_SAVEPOINT => false,
             ffi::SQLITE_PRAGMA => {
                 !pragma_is(action, "wal_checkpoint")
                     && !(action.second.is_some()
-                        && ["query_only", "writable_schema"]
+                        && ["query_only", "writable_schema", "locking_mode"]
                             .iter()
                             .any(|name| pragma_is(action, name)))
             }
````

- [ ] **Step 4: Run the tests to see them pass**

```bash
~/.cargo/bin/cargo test --locked -p tabletist-db --lib -- a_flag_a_script_sets a_script_cannot_take each_fence_allows
```

Expected: PASS, 3 passed.

- [ ] **Step 5: Run the four checks**

All four pass. Fix what does not before going on; do not weaken a lint to get there.

- [ ] **Step 6: Commit**

```bash
git add crates/tabletist-db/src/sqlite.rs crates/tabletist-db/src/sqlite/fence.rs
git commit -S -m "Put back the SQLite flags a script sets, and deny it the file's lock" -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 2: The tab's mode

**Files:**
- Modify: `src/model.rs` (`RunMode`, `NoWrites`, `SqlTab`, `Workspace`, `Action`)
- Modify: `src/app.rs` (`App::apply`, `set_sql_mode`, and its tests module)
- Modify: `src/testing.rs` (`Harness::reconnect_fake_as`)

The model only: nothing draws the mode yet and no run reads it. `SqlTab::mode` is what the user set. `Workspace::run_mode` is the mode that counts, the spec's effective mode: the tab's own while `Workspace::sql_writes` is `Ok`, and `ReadOnly` everywhere else. A tab's own mode is kept through a session that came back read-only, and neither the badge nor the key changes it there.

`sql_writes` is the one place that knows why an editor cannot write. Its second arm, `NoWrites::Unconfirmed`, is this run's stand-in for the production confirmation: it uses `Environment::confirms_writes()`, the same question a grid's Save asks, so the two cannot disagree about which environment is asked about.

- [ ] **Step 1: Write the failing tests**

Apply this block. It holds the task's tests, and applies on the tree as the task before left it.

````diff
diff --git a/src/app.rs b/src/app.rs
index 7ed9d92..23e9509 100644
--- a/src/app.rs
+++ b/src/app.rs
@@ -6559,6 +6579,108 @@ mod tests {
         assert_eq!(cancels_since(&harness, sent), loading);
     }
 
+    /// A SQL editor on a connection that takes writes.
+    fn writable_sql(harness: &mut Harness) -> (ConnTabId, TabId) {
+        let tab = harness.connect_fake_as(false);
+        harness.app.apply(Action::NewSqlTab(tab));
+        let id = harness.app.workspace(tab).unwrap().active_tab.unwrap();
+        (tab, id)
+    }
+
+    /// The mode a run of the editor would have now.
+    fn run_mode(harness: &Harness, tab: ConnTabId, id: TabId) -> RunMode {
+        let workspace = harness.app.workspace(tab).unwrap();
+        workspace.run_mode(workspace.sql_tab(id).unwrap())
+    }
+
+    fn set_mode(harness: &mut Harness, tab: ConnTabId, id: TabId, mode: RunMode) {
+        harness.app.apply(Action::SetSqlMode {
+            tab,
+            sql_tab: id,
+            mode,
+        });
+    }
+
+    fn toggle_mode(harness: &mut Harness, tab: ConnTabId, id: TabId) {
+        harness
+            .app
+            .apply(Action::ToggleSqlMode { tab, sql_tab: id });
+    }
+
+    #[test]
+    fn a_new_editor_reads_only_until_it_is_switched() {
+        let mut harness = Harness::new();
+        let (tab, id) = writable_sql(&mut harness);
+        assert_eq!(sql(&harness, tab, id).mode, RunMode::ReadOnly);
+        assert_eq!(run_mode(&harness, tab, id), RunMode::ReadOnly);
+        set_mode(&mut harness, tab, id, RunMode::ReadWrite);
+        assert_eq!(run_mode(&harness, tab, id), RunMode::ReadWrite);
+        // The key switches to the other mode, and back.
+        toggle_mode(&mut harness, tab, id);
+        assert_eq!(run_mode(&harness, tab, id), RunMode::ReadOnly);
+        toggle_mode(&mut harness, tab, id);
+        assert_eq!(run_mode(&harness, tab, id), RunMode::ReadWrite);
+        // The mode is the tab's own: another editor of the connection
+        // starts read-only all the same.
+        harness.app.apply(Action::NewSqlTab(tab));
+        let other = harness.app.workspace(tab).unwrap().active_tab.unwrap();
+        assert_eq!(run_mode(&harness, tab, other), RunMode::ReadOnly);
+        assert_eq!(run_mode(&harness, tab, id), RunMode::ReadWrite);
+    }
+
+    #[test]
+    fn the_badge_and_the_key_do_nothing_where_an_editor_cannot_write() {
+        use crate::model::NoWrites;
+        // A connection that opens read-only.
+        let mut harness = Harness::new();
+        let (tab, id) = new_sql(&mut harness);
+        let workspace = harness.app.workspace(tab).unwrap();
+        assert_eq!(workspace.sql_writes(), Err(NoWrites::ReadOnlyConnection));
+        set_mode(&mut harness, tab, id, RunMode::ReadWrite);
+        toggle_mode(&mut harness, tab, id);
+        assert_eq!(sql(&harness, tab, id).mode, RunMode::ReadOnly);
+        // A writable connection to production: a write there is asked about
+        // first, and nothing asks about a script yet.
+        let mut harness = Harness::new();
+        let (tab, id) = writable_sql(&mut harness);
+        harness.app.workspace_mut(tab).unwrap().environment = crate::env::Environment::Production;
+        let workspace = harness.app.workspace(tab).unwrap();
+        assert_eq!(workspace.sql_writes(), Err(NoWrites::Unconfirmed));
+        set_mode(&mut harness, tab, id, RunMode::ReadWrite);
+        toggle_mode(&mut harness, tab, id);
+        assert_eq!(sql(&harness, tab, id).mode, RunMode::ReadOnly);
+        // Every other environment writes without a question.
+        for environment in crate::env::Environment::ALL {
+            let mut harness = Harness::new();
+            let (tab, _) = writable_sql(&mut harness);
+            harness.app.workspace_mut(tab).unwrap().environment = environment;
+            let writes = harness.app.workspace(tab).unwrap().sql_writes();
+            assert_eq!(
+                writes.is_ok(),
+                !environment.confirms_writes(),
+                "{environment:?}"
+            );
+        }
+    }
+
+    #[test]
+    fn a_session_that_came_back_read_only_runs_read_only_and_keeps_the_tabs_mode() {
+        let mut harness = Harness::new();
+        let (tab, id) = writable_sql(&mut harness);
+        set_mode(&mut harness, tab, id, RunMode::ReadWrite);
+        // The box was turned on meanwhile.
+        harness.reconnect_fake_as(tab, true);
+        assert_eq!(run_mode(&harness, tab, id), RunMode::ReadOnly);
+        assert_eq!(sql(&harness, tab, id).mode, RunMode::ReadWrite);
+        // Neither the badge nor the key changes what the tab was set to.
+        toggle_mode(&mut harness, tab, id);
+        set_mode(&mut harness, tab, id, RunMode::ReadOnly);
+        assert_eq!(sql(&harness, tab, id).mode, RunMode::ReadWrite);
+        // Writable once more, the tab's own mode counts again.
+        harness.reconnect_fake_as(tab, false);
+        assert_eq!(run_mode(&harness, tab, id), RunMode::ReadWrite);
+    }
+
     #[test]
     fn the_limit_and_timeout_menus_change_the_next_run_and_the_settings() {
         let mut harness = Harness::new();
diff --git a/src/testing.rs b/src/testing.rs
index b746247..1b51d48 100644
--- a/src/testing.rs
+++ b/src/testing.rs
@@ -522,6 +522,45 @@ impl Harness {
     }
 }
 
+impl Harness {
+    /// Sets the "Open read-only" box of `tab`'s saved connection and
+    /// reconnects, as a user who changed the box and pressed Reconnect: the
+    /// session comes back as the box says. The tree's requests are left
+    /// unanswered.
+    pub fn reconnect_fake_as(&mut self, tab: ConnTabId, read_only: bool) {
+        let conn = self
+            .app
+            .workspace(tab)
+            .expect("a workspace")
+            .conn_id
+            .clone();
+        let mut saved = self.app.connections.get(&conn).expect("saved").clone();
+        saved.read_only = Some(read_only);
+        self.app.connections.upsert(saved);
+        self.app.apply(Action::Reconnect(tab));
+        let (session, request) = self
+            .app
+            .backend
+            .sent
+            .iter()
+            .rev()
+            .find_map(|command| match command {
+                Command::Connect {
+                    session, request, ..
+                } => Some((*session, *request)),
+                _ => None,
+            })
+            .expect("a Connect was sent");
+        self.app.apply(Action::Backend(Event::Connected {
+            session,
+            request,
+            driver: Driver::Sqlite,
+            encrypted: false,
+            access: asked_access(&self.app),
+        }));
+    }
+}
+
 use tabletist_db::{ColumnMeta, RowPage, Value, ValueKind};
 
 /// A page shaped like the fixture's users table.
````

- [ ] **Step 2: Run the tests to see them fail**

```bash
~/.cargo/bin/cargo test --locked -p tabletist --lib -- a_new_editor_reads_only the_badge_and_the_key_do_nothing a_session_that_came_back
```

Expected: does not compile. The tests name what the code adds: `cannot find type RunMode`, `unresolved import crate::model::NoWrites`, `no field mode on type &model::SqlTab`, `no method named sql_writes` and `run_mode` on `&Workspace`, `no variant named SetSqlMode` and `ToggleSqlMode` for `model::Action`.

- [ ] **Step 3: Write the code**

````diff
diff --git a/src/app.rs b/src/app.rs
index 7ed9d92..23e9509 100644
--- a/src/app.rs
+++ b/src/app.rs
@@ -16,8 +16,8 @@ use crate::model::{Action, ConnTab, ConnTabContent, ConnTabId, PickerState};
 use crate::model::{
     Advance, CellPos, Completion, ConnectionForm, Dialog, Fetch, FilterBar, FilterRow, Held,
     HostKeyPrompt, LeavePrompt, ObjectTab, ObjectView, Pane, PasswordPrompt, PickTarget, QuickOpen,
-    ResultPane, SecretKind, SessionStatus, SqlTab, Tab, TabId, TestState, TextPrint, Tree, TreeKey,
-    TreeNode, Wanted, Workspace,
+    ResultPane, RunMode, SecretKind, SessionStatus, SqlTab, Tab, TabId, TestState, TextPrint, Tree,
+    TreeKey, TreeNode, Wanted, Workspace,
 };
 use crate::paths::AppDirs;
 use crate::secrets::{SecretString, password_account, ssh_account};
@@ -1100,6 +1100,10 @@ impl App {
                 }
                 self.change_settings(|settings| settings.sql_timeout_secs = secs);
             }
+            Action::SetSqlMode { tab, sql_tab, mode } => {
+                self.set_sql_mode(tab, sql_tab, Some(mode))
+            }
+            Action::ToggleSqlMode { tab, sql_tab } => self.set_sql_mode(tab, sql_tab, None),
             Action::SetResultPane { tab, sql_tab, pane } => {
                 if let Some(sql) = self.sql_tab_mut(tab, sql_tab) {
                     sql.pane = pane;
@@ -3169,6 +3173,22 @@ impl App {
         changed | was_open | sql.completion.is_some()
     }
 
+    /// Sets how an editor's runs end: to `mode`, or to the other one. Only
+    /// where an editor can run read-write: everywhere else the badge and
+    /// the key do nothing, and a mode the tab was given before stays as it
+    /// is for a session that can write again.
+    fn set_sql_mode(&mut self, tab: ConnTabId, id: TabId, mode: Option<RunMode>) {
+        let Some(workspace) = self.workspace_mut(tab) else {
+            return;
+        };
+        if workspace.sql_writes().is_err() {
+            return;
+        }
+        if let Some(sql) = workspace.sql_tab_mut(id) {
+            sql.mode = mode.unwrap_or_else(|| sql.mode.other());
+        }
+    }
+
     /// Runs the statement at the editor's cursor, or every statement. An
     /// editor holding no statement (empty, or only comments) runs nothing.
     /// Nor does one whose session is not connected: the backend would
diff --git a/src/model.rs b/src/model.rs
index 5eb31dc..6a497bd 100644
--- a/src/model.rs
+++ b/src/model.rs
@@ -361,6 +361,18 @@ pub enum Action {
         sql_tab: TabId,
         secs: Option<u32>,
     },
+    /// The badge's menu: how this editor's runs end. Nothing on a
+    /// connection whose editors cannot write (see `Workspace::sql_writes`).
+    SetSqlMode {
+        tab: ConnTabId,
+        sql_tab: TabId,
+        mode: RunMode,
+    },
+    /// `Mod+Shift+M`: the editor's other mode.
+    ToggleSqlMode {
+        tab: ConnTabId,
+        sql_tab: TabId,
+    },
     /// Show a SQL editor's Results or its Messages.
     SetResultPane {
         tab: ConnTabId,
@@ -2031,6 +2043,38 @@ pub type ShownRows<'a> = (
     bool,
 );
 
+/// How a SQL editor's runs are meant to end: what its toolbar's badge
+/// says and switches.
+#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
+pub enum RunMode {
+    /// Every run is a read-only transaction that is rolled back.
+    #[default]
+    ReadOnly,
+    /// A run that changes data is one read-write transaction, committed
+    /// when every statement succeeded. A run of reads is still read-only.
+    ReadWrite,
+}
+
+impl RunMode {
+    pub fn other(self) -> Self {
+        match self {
+            Self::ReadOnly => Self::ReadWrite,
+            Self::ReadWrite => Self::ReadOnly,
+        }
+    }
+}
+
+/// Why no SQL editor of a workspace runs read-write, whatever its mode.
+#[derive(Debug, Clone, Copy, PartialEq, Eq)]
+pub enum NoWrites {
+    /// The session was opened read-only.
+    ReadOnlyConnection,
+    /// The connection's environment asks before a write, and the question
+    /// for a script is not built yet: no read-write run goes to production
+    /// without it.
+    Unconfirmed,
+}
+
 /// A finished SQL editor run.
 #[derive(Debug, Clone, PartialEq)]
 pub struct SqlRun {
@@ -2314,6 +2358,9 @@ pub struct SqlTab {
     pub cursor: usize,
     pub limit: u32,
     pub timeout: Option<Duration>,
+    /// What the user set the badge to. It counts only while the session
+    /// can write: ask `Workspace::run_mode` for the mode a run has.
+    pub mode: RunMode,
     /// The last run, or the one running (a whole-run failure is its error).
     /// Change it through `start_run`, `finish_run` and `abandon_run`.
     pub run: Fetch<SqlRun>,
@@ -2351,6 +2398,7 @@ impl std::fmt::Debug for SqlTab {
             .field("cursor", &self.cursor)
             .field("limit", &self.limit)
             .field("timeout", &self.timeout)
+            .field("mode", &self.mode)
             .field("run", &self.run)
             .field("in_flight", &self.in_flight)
             .field("pane", &self.pane)
@@ -2374,6 +2422,7 @@ impl SqlTab {
             cursor: 0,
             limit,
             timeout,
+            mode: RunMode::default(),
             run: Fetch::default(),
             in_flight: None,
             pane: ResultPane::default(),
@@ -2545,6 +2594,29 @@ impl SqlTab {
 }
 
 impl Workspace {
+    /// Whether a SQL editor of this workspace can run read-write, or why
+    /// none can.
+    pub fn sql_writes(&self) -> Result<(), NoWrites> {
+        if self.access != tabletist_db::Access::Writable {
+            return Err(NoWrites::ReadOnlyConnection);
+        }
+        if self.environment.confirms_writes() {
+            return Err(NoWrites::Unconfirmed);
+        }
+        Ok(())
+    }
+
+    /// The mode a run of `sql` has: its own while the session can write,
+    /// and read-only everywhere else. A tab's own mode is kept through a
+    /// session that came back read-only, and counts again once one is
+    /// writable.
+    pub fn run_mode(&self, sql: &SqlTab) -> RunMode {
+        match self.sql_writes() {
+            Ok(()) => sql.mode,
+            Err(_) => RunMode::ReadOnly,
+        }
+    }
+
     /// Whether the tab has content to show now: its schemas are listed, or
     /// could not be. Until then the tab shows how connecting goes. A switch
     /// of database starts the tree over, so a tab in use can go back to
````

- [ ] **Step 4: Run the tests to see them pass**

```bash
~/.cargo/bin/cargo test --locked -p tabletist --lib -- a_new_editor_reads_only the_badge_and_the_key_do_nothing a_session_that_came_back
```

Expected: PASS, 3 passed.

- [ ] **Step 5: Run the four checks**

All four pass. Fix what does not before going on; do not weaken a lint to get there.

- [ ] **Step 6: Commit**

```bash
git add src/model.rs src/app.rs src/testing.rs
git commit -S -m "Give a SQL tab a mode: read-only or read-write" -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 3: The run's mode, the decision, and Run held back

**Files:**
- Modify: `src/backend.rs` (`Command::RunSql`, `run_session`, `broken_by`, and its tests module)
- Modify: `src/model.rs` (`SqlRun`, `RunInFlight`, `SqlTab::{start_run, finish_run, is_writing}`, and its tests module)
- Modify: `src/app.rs` (`run_sql`, `send_run`, `script_mode`, and its tests module)
- Modify: `src/ui/sql_editor.rs` (`Bar`, `run_button`, and its tests module)
- Modify: `src/testing.rs` (`write_outcome`, `done_outcome`)

From this task on a run can be sent to write, though nothing on screen switches a tab yet (task 5 does).

- `Command::RunSql` gains `mode: ScriptMode`, and the backend passes it on. When a run that writes ends with `ScriptOutcome::broken`, the backend emits `SqlRan` first and closes the session after it (`broken_by`), through the path a lost connection takes: the queued commands are answered and `Event::Disconnected` raises the reconnect banner. A run known to be committed is never shown as one that may be.
- `App::run_sql` decides (`script_mode`): `ScriptMode::Write` only when the tab's `run_mode` is `ReadWrite` and a statement is a `sql::kind` `Write`. `send_run` is the one place that sends a run; task 6 calls it too.
- While a tab's run was sent to write and is in flight (`SqlTab::is_writing`), Run and Run all do nothing there, in the reducer and in the toolbar, whose buttons are disabled and say why. A new run would cancel a transaction that may be on its way to its commit. Cancel still stops it, and a read-only run is replaced by the next as it always was.

The tests' block is long because two signatures change: every `start_run` call in the model's tests gains `READ` (a constant the block adds), and every `Command::RunSql` the backend's tests build gains `mode: ScriptMode::ReadOnly`. No assertion of an existing test changes.

- [ ] **Step 1: Write the failing tests**

Apply this block. It holds the task's tests, and applies on the tree as the task before left it.

````diff
diff --git a/src/app.rs b/src/app.rs
index 23e9509..7ad981c 100644
--- a/src/app.rs
+++ b/src/app.rs
@@ -5764,10 +5814,11 @@ mod tests {
         let request = RequestId(harness.app.next_id());
         let workspace = harness.app.workspace_mut(tab).unwrap();
         let statements = tabletist_db::sql::statements(workspace.driver.dialect(), "SELECT 1");
-        let superseded = workspace
-            .sql_tab_mut(id)
-            .unwrap()
-            .start_run(request, statements);
+        let superseded = workspace.sql_tab_mut(id).unwrap().start_run(
+            request,
+            statements,
+            tabletist_db::ScriptMode::ReadOnly,
+        );
         assert_eq!(superseded, None);
         request
     }
@@ -6681,6 +6732,142 @@ mod tests {
         assert_eq!(run_mode(&harness, tab, id), RunMode::ReadWrite);
     }
 
+    /// The transaction the newest run was sent in.
+    fn sent_mode(harness: &Harness) -> tabletist_db::ScriptMode {
+        match last_sent(&harness.app) {
+            Command::RunSql { mode, .. } => *mode,
+            other => panic!("expected RunSql, got {other:?}"),
+        }
+    }
+
+    /// Ends the run in flight as committed, each statement having changed
+    /// one row.
+    fn commit(harness: &mut Harness, statements: usize) {
+        use crate::testing::{done_outcome, write_outcome};
+        let done = vec![done_outcome(Some(1)); statements];
+        let outcome = write_outcome(done, tabletist_db::ScriptEnd::Committed);
+        harness.answer_sql(Ok(outcome), None);
+    }
+
+    #[test]
+    fn a_run_is_read_write_only_in_a_read_write_editor_and_only_with_a_write() {
+        use tabletist_db::ScriptMode::{ReadOnly, Write};
+        let mut harness = Harness::new();
+        let (tab, id) = writable_sql(&mut harness);
+        let script = "SELECT 1;\nUPDATE users SET email = 'x' WHERE id = 1";
+        // In Read-only nothing is sent to write, whatever the script holds.
+        type_sql(&mut harness, tab, id, script, 0);
+        run(&mut harness, tab, id, true);
+        assert_eq!(sent_mode(&harness), ReadOnly);
+        assert!(!sql(&harness, tab, id).is_writing());
+        harness.answer_sql(Ok(crate::testing::script_outcome(Vec::new())), None);
+        assert_eq!(sql(&harness, tab, id).last_run().unwrap().mode, ReadOnly);
+        // In Read-write the run that holds the write is sent to write.
+        set_mode(&mut harness, tab, id, RunMode::ReadWrite);
+        run(&mut harness, tab, id, true);
+        assert_eq!(sent_mode(&harness), Write);
+        assert!(sql(&harness, tab, id).is_writing());
+        commit(&mut harness, 2);
+        assert!(!sql(&harness, tab, id).is_writing());
+        assert_eq!(sql(&harness, tab, id).last_run().unwrap().mode, Write);
+        // The statement at the cursor is a read: no read-write transaction
+        // is opened for it, though the tab is in Read-write.
+        run(&mut harness, tab, id, false);
+        assert_eq!(sent_mode(&harness), ReadOnly);
+        harness.answer_sql(Ok(crate::testing::script_outcome(Vec::new())), None);
+        // With the cursor in the UPDATE, that statement alone is the run.
+        type_sql(&mut harness, tab, id, script, script.len());
+        run(&mut harness, tab, id, false);
+        assert_eq!(sent_mode(&harness), Write);
+        assert!(matches!(
+            last_sent(&harness.app),
+            Command::RunSql { statements, .. } if statements.len() == 1
+        ));
+    }
+
+    #[test]
+    fn a_statement_is_a_write_unless_it_plainly_reads() {
+        use tabletist_db::ScriptMode::{ReadOnly, Write};
+        let mut harness = Harness::new();
+        let (tab, id) = writable_sql(&mut harness);
+        set_mode(&mut harness, tab, id, RunMode::ReadWrite);
+        for (text, mode) in [
+            ("SELECT * FROM users", ReadOnly),
+            ("WITH old AS (SELECT 1) SELECT * FROM old", ReadOnly),
+            ("EXPLAIN DELETE FROM users", ReadOnly),
+            ("PRAGMA table_info(users)", ReadOnly),
+            ("-- UPDATE in a note\nSELECT 'DELETE'", ReadOnly),
+            ("INSERT INTO users (email) VALUES ('a')", Write),
+            ("DELETE FROM users", Write),
+            ("CREATE TABLE notes (seen int)", Write),
+            (
+                "WITH gone AS (DELETE FROM users RETURNING id) SELECT * FROM gone",
+                Write,
+            ),
+            ("EXPLAIN ANALYZE DELETE FROM users", Write),
+            // One write among reads is enough.
+            ("SELECT 1; DROP TABLE users; SELECT 2", Write),
+        ] {
+            type_sql(&mut harness, tab, id, text, 0);
+            run(&mut harness, tab, id, true);
+            assert_eq!(sent_mode(&harness), mode, "{text}");
+            harness.answer_sql(Ok(crate::testing::script_outcome(Vec::new())), None);
+        }
+    }
+
+    #[test]
+    fn nothing_is_sent_to_write_where_the_session_cannot() {
+        use tabletist_db::ScriptMode::ReadOnly;
+        let write = "DELETE FROM users WHERE id = 1";
+        // The session came back read-only under a tab in Read-write.
+        let mut harness = Harness::new();
+        let (tab, id) = writable_sql(&mut harness);
+        set_mode(&mut harness, tab, id, RunMode::ReadWrite);
+        harness.reconnect_fake_as(tab, true);
+        type_sql(&mut harness, tab, id, write, 0);
+        run(&mut harness, tab, id, false);
+        assert_eq!(sent_mode(&harness), ReadOnly);
+        // Production, with a tab that says Read-write however it came to:
+        // no read-write run goes there without its confirmation.
+        let mut harness = Harness::new();
+        let (tab, id) = writable_sql(&mut harness);
+        let workspace = harness.app.workspace_mut(tab).unwrap();
+        workspace.environment = crate::env::Environment::Production;
+        workspace.sql_tab_mut(id).unwrap().mode = RunMode::ReadWrite;
+        type_sql(&mut harness, tab, id, write, 0);
+        run(&mut harness, tab, id, false);
+        assert_eq!(sent_mode(&harness), ReadOnly);
+    }
+
+    #[test]
+    fn run_does_nothing_while_the_editors_read_write_run_is_in_flight() {
+        let mut harness = Harness::new();
+        let (tab, id) = writable_sql(&mut harness);
+        set_mode(&mut harness, tab, id, RunMode::ReadWrite);
+        type_sql(&mut harness, tab, id, "DELETE FROM users WHERE id = 1", 0);
+        run(&mut harness, tab, id, false);
+        let writing = sql(&harness, tab, id).run.pending.expect("a run in flight");
+        let sent = harness.app.backend.sent.len();
+        // Neither Run nor Run all replaces it, nor cancels it.
+        run(&mut harness, tab, id, false);
+        run(&mut harness, tab, id, true);
+        assert_eq!(harness.app.backend.sent.len(), sent);
+        assert_eq!(sql(&harness, tab, id).run.pending, Some(writing));
+        // Cancel still stops it.
+        harness.app.apply(Action::CancelQuery(tab));
+        assert_eq!(cancels_since(&harness, sent), vec![writing]);
+        commit(&mut harness, 1);
+        assert!(!sql(&harness, tab, id).is_running());
+        // A read-only run is replaced by the next one, as it always was.
+        type_sql(&mut harness, tab, id, "SELECT 1", 0);
+        run(&mut harness, tab, id, false);
+        let reading = sql(&harness, tab, id).run.pending.expect("a run in flight");
+        let sent = harness.app.backend.sent.len();
+        run(&mut harness, tab, id, false);
+        assert_eq!(cancels_since(&harness, sent), vec![reading]);
+        assert_eq!(runs_since(&harness, sent), 1);
+    }
+
     #[test]
     fn the_limit_and_timeout_menus_change_the_next_run_and_the_settings() {
         let mut harness = Harness::new();
@@ -7748,9 +7935,11 @@ mod tests {
         let run = RequestId(app.next_id());
         let workspace = app.workspace_mut(tab).unwrap();
         let statements = tabletist_db::sql::statements(workspace.driver.dialect(), "SELECT 1");
-        let _ = workspace
-            .push_sql_tab(id, 1_000, None)
-            .start_run(run, statements);
+        let _ = workspace.push_sql_tab(id, 1_000, None).start_run(
+            run,
+            statements,
+            tabletist_db::ScriptMode::ReadOnly,
+        );
         workspace.server_version.value = Some("PostgreSQL 17.2".into());
         prompt(&mut app).password = "right".into();
         app.apply(Action::SubmitPassword);
diff --git a/src/backend.rs b/src/backend.rs
index 0d8b6f0..795d41d 100644
--- a/src/backend.rs
+++ b/src/backend.rs
@@ -2744,6 +2757,7 @@ mod tests {
             statements: statements("SELECT 1; SELECT 2"),
             limit: 10,
             timeout: None,
+            mode: ScriptMode::ReadOnly,
         });
         let (request, result, cancel) = sql_ran(&mut backend);
         assert_eq!(request, RequestId(2));
@@ -2760,6 +2774,7 @@ mod tests {
             statements: statements("SELECT 1"),
             limit: 10,
             timeout: Some(Duration::from_secs(3600)),
+            mode: ScriptMode::ReadOnly,
         });
         let (_, result, cancel) = sql_ran(&mut backend);
         assert_eq!(cancel, None);
@@ -2785,6 +2800,7 @@ mod tests {
             statements: statements(&format!("SELECT 1; {ENDLESS}")),
             limit: 10,
             timeout: Some(after),
+            mode: ScriptMode::ReadOnly,
         });
         let (_, result, cancel) = sql_ran(&mut backend);
         assert_eq!(cancel, Some(CancelReason::Timeout(after)));
@@ -2807,6 +2823,7 @@ mod tests {
             statements: statements("SELECT count(*) FROM big"),
             limit: 10,
             timeout: None,
+            mode: ScriptMode::ReadOnly,
         });
         let (request, result, cancel) = sql_ran(&mut backend);
         assert_eq!(request, RequestId(3));
@@ -2823,6 +2840,7 @@ mod tests {
             statements: statements(&format!("SELECT 1; {ENDLESS}")),
             limit: 10,
             timeout: Some(Duration::from_secs(3600)),
+            mode: ScriptMode::ReadOnly,
         });
         wait_until_running(&backend, session, RequestId(2), true);
         backend.send(Command::Cancel {
@@ -2863,6 +2881,7 @@ mod tests {
                 statements: statements(ENDLESS),
                 limit: 10,
                 timeout: None,
+                mode: ScriptMode::ReadOnly,
             });
             wait_until_running(&backend, session, request, true);
             // The cancels sent for the script before it ended with that
@@ -2887,6 +2906,7 @@ mod tests {
             statements: statements(ENDLESS),
             limit: 10,
             timeout: None,
+            mode: ScriptMode::ReadOnly,
         });
         backend.send(Command::RunSql {
             session,
@@ -2894,6 +2914,7 @@ mod tests {
             statements: statements("SELECT 1"),
             limit: 10,
             timeout: None,
+            mode: ScriptMode::ReadOnly,
         });
         // Superseded while the first still runs (or waits its turn).
         backend.send(Command::Cancel {
@@ -2950,6 +2971,7 @@ mod tests {
                 statements: statements("SELECT 1"),
                 limit: 10,
                 timeout: None,
+                mode: ScriptMode::ReadOnly,
             },
         );
         skip(
@@ -2968,6 +2990,7 @@ mod tests {
                 statements: statements("SELECT 1"),
                 limit: 10,
                 timeout: None,
+                mode: ScriptMode::ReadOnly,
             },
             lost(),
         );
@@ -3016,6 +3039,7 @@ mod tests {
             statements: statements(&format!("SELECT 1; {ENDLESS}")),
             limit: 10,
             timeout: None,
+            mode: ScriptMode::ReadOnly,
         });
         wait_until_running(&backend, session, RequestId(2), true);
         backend.send(Command::Close { session });
@@ -3147,6 +3171,7 @@ mod tests {
                     statements: statements(&format!("SELECT 1; {ENDLESS}")),
                     limit: 10,
                     timeout: None,
+                    mode: ScriptMode::ReadOnly,
                 })
                 .unwrap();
             let mut task = tokio::spawn(run_session(
@@ -3823,6 +3848,7 @@ mod tests {
             statements: statements("SELECT 1;\nCOMMIT"),
             limit: 10,
             timeout: Some(Duration::from_secs(3600)),
+            mode: ScriptMode::ReadOnly,
         });
         let (_, result, cancel) = sql_ran(&mut backend);
         assert_eq!(cancel, None);
@@ -3939,6 +3965,96 @@ mod tests {
         assert_eq!(lost_error(&Ok(cancelled_outcome())), None);
     }
 
+    #[test]
+    fn a_broken_outcome_is_why_its_session_is_closed() {
+        let broken = ScriptOutcome {
+            end: tabletist_db::ScriptEnd::Committed,
+            broken: Some(Error::ConnectionLost(
+                "could not end the transaction".into(),
+            )),
+            ..Default::default()
+        };
+        // The outcome is delivered as it is, and is why the session goes.
+        let closed = broken_by(&Ok(broken)).expect("the session is to be closed");
+        assert!(closed.is_connection_lost());
+        assert_eq!(broken_by(&Ok(ScriptOutcome::default())), None);
+        assert_eq!(broken_by(&Ok(cancelled_outcome())), None);
+        assert_eq!(broken_by(&Err(Error::Cancelled)), None);
+    }
+
+    #[test]
+    fn a_script_writes_only_in_a_run_sent_to_write() {
+        use tabletist_db::{ScriptEnd, StatementOutcome};
+        let (_dir, mut backend, session) = connected_as(Access::Writable);
+        let insert = "INSERT INTO events (kind) VALUES ('probe')";
+        let mut run = |request: u64, text: &str, mode: ScriptMode| {
+            backend.send(Command::RunSql {
+                session,
+                request: RequestId(request),
+                statements: statements(text),
+                limit: 10,
+                timeout: None,
+                mode,
+            });
+            let (_, result, _) = sql_ran(&mut backend);
+            result
+        };
+        // The run every editor had before: the database refuses the write.
+        let refused = run(2, insert, ScriptMode::ReadOnly).unwrap();
+        assert!(matches!(
+            refused.results[0].outcome,
+            StatementOutcome::Error { .. }
+        ));
+        assert_eq!(refused.end, ScriptEnd::RolledBack);
+        // Sent to write, it is committed and says how many rows it changed.
+        let written = run(3, insert, ScriptMode::Write).unwrap();
+        assert_eq!(written.end, ScriptEnd::Committed);
+        assert_eq!(
+            written.results[0].outcome,
+            StatementOutcome::Done {
+                affected: Some(1),
+                warnings: 0
+            }
+        );
+        // The session is fenced again: the next read-only run cannot write,
+        // and finds the one row.
+        assert!(matches!(
+            run(4, insert, ScriptMode::ReadOnly).unwrap().results[0].outcome,
+            StatementOutcome::Error { .. }
+        ));
+        let count = "SELECT count(*) FROM events WHERE kind = 'probe'";
+        let counted = run(5, count, ScriptMode::ReadOnly).unwrap();
+        assert!(matches!(
+            &counted.results[0].outcome,
+            StatementOutcome::Rows { rows, .. } if rows == &[vec![Value::Int(1)]]
+        ));
+    }
+
+    #[test]
+    fn a_run_sent_to_write_on_a_read_only_session_sends_nothing_and_keeps_it() {
+        let (_dir, mut backend, session) = connected_as(Access::ReadOnly);
+        backend.send(Command::RunSql {
+            session,
+            request: RequestId(2),
+            statements: statements("INSERT INTO events (kind) VALUES ('probe')"),
+            limit: 10,
+            timeout: None,
+            mode: ScriptMode::Write,
+        });
+        let (_, result, cancel) = sql_ran(&mut backend);
+        assert_eq!(result, Err(Error::ReadOnly));
+        assert_eq!(cancel, None);
+        // The session takes the next request.
+        backend.send(Command::ListSchemas {
+            session,
+            request: RequestId(3),
+        });
+        assert!(matches!(
+            backend.wait(WAIT),
+            Some(Event::Schemas { result: Ok(_), .. })
+        ));
+    }
+
     #[test]
     fn a_lost_connection_answers_queued_scripts() {
         let (sender, mut commands) = tokio_mpsc::unbounded_channel();
@@ -3951,6 +4067,7 @@ mod tests {
                 statements: statements("SELECT 1"),
                 limit: 10,
                 timeout: Some(Duration::from_secs(30)),
+                mode: ScriptMode::ReadOnly,
             })
             .unwrap();
         sender
@@ -3990,6 +4107,7 @@ mod tests {
             statements: statements("SELECT 1"),
             limit: 10,
             timeout: None,
+            mode: ScriptMode::ReadOnly,
         };
         let version = Command::ServerVersion {
             session: SessionId(7),
@@ -4028,6 +4146,7 @@ mod tests {
             statements: statements("SELECT 1"),
             limit: 10,
             timeout: None,
+            mode: ScriptMode::ReadOnly,
         });
         let (_, result, cancel) = sql_ran(&mut backend);
         assert_eq!(cancel, None);
@@ -4427,6 +4546,7 @@ mod tests {
             statements: statements("SELECT 1;\nSELECT 'hunter2' FROM payroll"),
             limit: 10,
             timeout: None,
+            mode: ScriptMode::ReadOnly,
         };
         let printed = format!("{command:?}");
         for typed in ["hunter2", "payroll", "SELECT"] {
diff --git a/src/model.rs b/src/model.rs
index 6a497bd..8bdd40f 100644
--- a/src/model.rs
+++ b/src/model.rs
@@ -3143,6 +3162,9 @@ mod tests {
         tabletist_db::sql::statements(Driver::Sqlite.dialect(), text)
     }
 
+    /// The transaction every run was sent in before an editor could write.
+    const READ: tabletist_db::ScriptMode = tabletist_db::ScriptMode::ReadOnly;
+
     fn editor() -> SqlTab {
         SqlTab::new(TabId(2), 1, 1_000, Some(Duration::from_secs(30)))
     }
@@ -3170,7 +3192,7 @@ mod tests {
         let mut tab = sql_tab(2);
         assert!(tab.pending().is_empty());
         let sql = tab.as_sql_mut().unwrap();
-        assert_eq!(sql.start_run(RequestId(9), script("SELECT 1")), None);
+        assert_eq!(sql.start_run(RequestId(9), script("SELECT 1"), READ), None);
         assert_eq!(tab.pending(), vec![RequestId(9)]);
     }
 
@@ -3178,10 +3200,10 @@ mod tests {
     fn a_new_run_names_the_one_it_replaces() {
         let mut sql = editor();
         assert!(!sql.is_running() && sql.running_for().is_none());
-        assert_eq!(sql.start_run(RequestId(9), script("SELECT 1")), None);
+        assert_eq!(sql.start_run(RequestId(9), script("SELECT 1"), READ), None);
         assert!(sql.is_running() && sql.running_for().is_some());
         assert_eq!(
-            sql.start_run(RequestId(10), script("SELECT 2; SELECT 3")),
+            sql.start_run(RequestId(10), script("SELECT 2; SELECT 3"), READ),
             Some(RequestId(9)),
             "the caller cancels it"
         );
@@ -3192,7 +3214,7 @@ mod tests {
     #[test]
     fn a_finished_run_keeps_the_statements_it_ran() {
         let mut sql = editor();
-        let _ = sql.start_run(RequestId(9), script("SELECT 1; SELECT 2"));
+        let _ = sql.start_run(RequestId(9), script("SELECT 1; SELECT 2"), READ);
         let cancel = Some(crate::backend::CancelReason::User);
         let outcome = tabletist_db::ScriptOutcome {
             stopped: true,
@@ -3211,8 +3233,8 @@ mod tests {
     #[test]
     fn a_stale_answer_does_not_finish_the_run() {
         let mut sql = editor();
-        let _ = sql.start_run(RequestId(9), script("SELECT 1"));
-        let _ = sql.start_run(RequestId(10), script("SELECT 2"));
+        let _ = sql.start_run(RequestId(9), script("SELECT 1"), READ);
+        let _ = sql.start_run(RequestId(10), script("SELECT 2"), READ);
         assert!(!sql.finish_run(RequestId(9), Ok(Default::default()), None));
         assert_eq!(sql.run.pending, Some(RequestId(10)));
         assert!(sql.run.value.is_none());
@@ -3230,9 +3252,9 @@ mod tests {
     #[test]
     fn a_run_that_fails_as_a_whole_is_the_runs_error_and_leaves_no_result() {
         let mut sql = editor();
-        let _ = sql.start_run(RequestId(9), script("SELECT 1"));
+        let _ = sql.start_run(RequestId(9), script("SELECT 1"), READ);
         assert!(sql.finish_run(RequestId(9), Ok(Default::default()), None));
-        let _ = sql.start_run(RequestId(10), script("SELECT 2"));
+        let _ = sql.start_run(RequestId(10), script("SELECT 2"), READ);
         let lost = Error::ConnectionLost("the server went away".into());
         assert!(sql.finish_run(RequestId(10), Err(lost.clone()), None));
         assert!(!sql.is_running() && sql.in_flight.is_none());
@@ -3245,9 +3267,9 @@ mod tests {
     #[test]
     fn abandoning_a_run_keeps_the_last_result() {
         let mut sql = editor();
-        let _ = sql.start_run(RequestId(9), script("SELECT 1"));
+        let _ = sql.start_run(RequestId(9), script("SELECT 1"), READ);
         assert!(sql.finish_run(RequestId(9), Ok(Default::default()), None));
-        let _ = sql.start_run(RequestId(10), script("SELECT 2"));
+        let _ = sql.start_run(RequestId(10), script("SELECT 2"), READ);
         sql.abandon_run();
         assert!(!sql.is_running() && sql.running_for().is_none());
         assert!(sql.in_flight.is_none());
@@ -3379,7 +3401,7 @@ mod tests {
         run_script(&mut sql, "SELECT 1", vec![rows_outcome(5)]);
         assert_eq!(readings(&sql), (Some(1), true, (5, 3), None));
         // The run in flight leaves the last one in place.
-        let _ = sql.start_run(RequestId(20), script("COMMIT"));
+        let _ = sql.start_run(RequestId(20), script("COMMIT"), READ);
         assert_eq!(readings(&sql), (Some(1), true, (5, 3), None));
         let lost = Error::ConnectionLost("the server went away".into());
         assert!(sql.finish_run(RequestId(20), Err(lost), None));
@@ -3387,7 +3409,7 @@ mod tests {
         assert_eq!(readings(&sql), (None, false, (0, 0), None));
         // Starting the next run clears the error and brings nothing back,
         // nor does an answer for a run that was replaced.
-        let _ = sql.start_run(RequestId(21), script("SELECT 2"));
+        let _ = sql.start_run(RequestId(21), script("SELECT 2"), READ);
         assert!(sql.run.error.is_none());
         assert_eq!(readings(&sql), (None, false, (0, 0), None));
         assert!(!sql.finish_run(RequestId(20), Ok(Default::default()), None));
@@ -3476,12 +3498,12 @@ mod tests {
         assert_eq!(readings(&sql), (Some(2), true, (1, 3), Some(2)));
         // The next run fails as a whole: nothing of the run before stays,
         // its rows and its mark alike.
-        let _ = sql.start_run(RequestId(20), script("SELECT 1;\nSELECT x"));
+        let _ = sql.start_run(RequestId(20), script("SELECT 1;\nSELECT x"), READ);
         assert_eq!(readings(&sql), (Some(2), true, (1, 3), None), "in flight");
         assert!(sql.finish_run(RequestId(20), Err(Error::LeftReadOnly), None));
         assert_eq!(readings(&sql), (None, false, (0, 0), None));
         // The run after it is never answered (its session is gone).
-        let _ = sql.start_run(RequestId(21), script("SELECT 1;\nSELECT x"));
+        let _ = sql.start_run(RequestId(21), script("SELECT 1;\nSELECT x"), READ);
         assert_eq!(readings(&sql), (None, false, (0, 0), None));
         sql.abandon_run();
         assert!(sql.run.error.is_none() && !sql.is_running());
@@ -3489,7 +3511,7 @@ mod tests {
         // A refusal is the one whole-run failure with a line to mark, and
         // it too leaves no rows.
         run_script(&mut sql, "SELECT 1;\nSELECT x", vec![rows_outcome(4)]);
-        let _ = sql.start_run(RequestId(30), script("SELECT 1;\nSELECT x"));
+        let _ = sql.start_run(RequestId(30), script("SELECT 1;\nSELECT x"), READ);
         let refused = Error::Refused {
             line: 2,
             what: "COMMIT".into(),
@@ -3509,7 +3531,7 @@ mod tests {
         );
         assert_eq!(sql.error_mark(), Some((1, None)));
         // The run in flight has not failed anywhere yet.
-        let _ = sql.start_run(RequestId(20), script("SELECT 1;\n\n\nCOMMIT"));
+        let _ = sql.start_run(RequestId(20), script("SELECT 1;\n\n\nCOMMIT"), READ);
         assert_eq!(sql.error_mark(), None);
         let refused = Error::Refused {
             line: 4,
@@ -3520,7 +3542,7 @@ mod tests {
         assert_eq!(sql.error_mark(), Some((4, None)));
         // A run that failed as a whole for another reason marks nothing:
         // no statement ran.
-        let _ = sql.start_run(RequestId(21), script("SELECT 1"));
+        let _ = sql.start_run(RequestId(21), script("SELECT 1"), READ);
         let lost = Error::ConnectionLost("the server went away".into());
         assert!(sql.finish_run(RequestId(21), Err(lost), None));
         assert!(sql.run.value.is_none());
@@ -3547,7 +3569,7 @@ mod tests {
         assert_eq!(sql.error_mark(), Some((3, None)));
         // A refusal's line is a line of the text that was refused.
         sql.text = "SELECT 1;\nCOMMIT".into();
-        let _ = sql.start_run(RequestId(30), script("SELECT 1;\nCOMMIT"));
+        let _ = sql.start_run(RequestId(30), script("SELECT 1;\nCOMMIT"), READ);
         let refused = Error::Refused {
             line: 2,
             what: "COMMIT".into(),
@@ -3565,7 +3587,7 @@ mod tests {
         // Typed in while the run was in flight: the mark is stale at once.
         let mut sql = editor();
         sql.text = "SELECT x".into();
-        let _ = sql.start_run(RequestId(9), script("SELECT x"));
+        let _ = sql.start_run(RequestId(9), script("SELECT x"), READ);
         sql.text.insert_str(0, "SELECT 1;\n");
         let outcome = crate::testing::script_outcome(failing());
         assert!(sql.finish_run(RequestId(9), Ok(outcome), None));
@@ -3575,7 +3597,7 @@ mod tests {
         let mut sql = editor();
         run_script(&mut sql, "SELECT x", failing());
         sql.text = "\nSELECT x".into();
-        let _ = sql.start_run(RequestId(40), script("\nSELECT x"));
+        let _ = sql.start_run(RequestId(40), script("\nSELECT x"), READ);
         sql.abandon_run();
         assert_eq!(sql.error_mark(), None);
         sql.text = "SELECT x".into();
@@ -3596,7 +3618,11 @@ mod tests {
         let mut tab = sql_tab(2);
         let sql = tab.as_sql_mut().unwrap();
         sql.text = "SELECT secret_column FROM vault".into();
-        let _ = sql.start_run(RequestId(9), script("SELECT secret_column FROM vault"));
+        let _ = sql.start_run(
+            RequestId(9),
+            script("SELECT secret_column FROM vault"),
+            READ,
+        );
         for printed in [format!("{tab:?}"), format!("{tab:#?}")] {
             assert!(printed.contains("SqlTab"), "{printed}");
             assert!(!printed.contains("secret_column"), "{printed}");
@@ -3644,7 +3670,7 @@ mod tests {
         workspace.tabs.push(object_tab(1, "users", true));
         workspace.push_sql_tab(TabId(2), 1_000, None);
         let running = workspace.push_sql_tab(TabId(3), 1_000, None);
-        let _ = running.start_run(RequestId(9), script("SELECT 1"));
+        let _ = running.start_run(RequestId(9), script("SELECT 1"), READ);
         workspace.server_version.value = Some("SQLite 3.46.0".into());
         workspace
             .object_tab_mut(TabId(1))
diff --git a/src/testing.rs b/src/testing.rs
index 1b51d48..9c27b14 100644
--- a/src/testing.rs
+++ b/src/testing.rs
@@ -755,6 +755,27 @@ pub fn script_outcome(
     }
 }
 
+/// What a script sent to write did: [`script_outcome`], with how its
+/// transaction ended.
+pub fn write_outcome(
+    outcomes: Vec<tabletist_db::StatementOutcome>,
+    end: tabletist_db::ScriptEnd,
+) -> tabletist_db::ScriptOutcome {
+    tabletist_db::ScriptOutcome {
+        end,
+        ..script_outcome(outcomes)
+    }
+}
+
+/// A statement that ran and gave no result set: how many rows it changed,
+/// where it counts them.
+pub fn done_outcome(affected: Option<u64>) -> tabletist_db::StatementOutcome {
+    tabletist_db::StatementOutcome::Done {
+        affected,
+        warnings: 0,
+    }
+}
+
 /// A script stopped before it began: cancelled while it was queued, or
 /// while its transaction opened. No statement has a result.
 pub fn stopped_before_it_began() -> tabletist_db::ScriptOutcome {
@@ -775,7 +796,7 @@ pub fn run_script(
     sql.text = text.into();
     let request = RequestId(sql.run.loaded.map_or(9, |last| last.0 + 1));
     let statements = tabletist_db::sql::statements(Driver::Sqlite.dialect(), text);
-    let _ = sql.start_run(request, statements);
+    let _ = sql.start_run(request, statements, tabletist_db::ScriptMode::ReadOnly);
     assert!(sql.finish_run(request, Ok(script_outcome(outcomes)), None));
 }
 
diff --git a/src/ui/sql_editor.rs b/src/ui/sql_editor.rs
index 34acfea..c414036 100644
--- a/src/ui/sql_editor.rs
+++ b/src/ui/sql_editor.rs
@@ -935,7 +951,84 @@ pub fn status_summary(app: &App, tab: ConnTabId) -> Option<String> {
 
 #[cfg(test)]
 mod tests {
+    use egui::accesskit::Role;
+    use egui::{Key, Modifiers};
+
     use super::*;
+    use crate::model::RunMode;
+    use crate::testing::{Harness, done_outcome, node, write_outcome};
+
+    /// A SQL editor on a connection that takes writes, drawn in `look`,
+    /// with `text` typed into it.
+    fn editor(look: Look, text: &str) -> (Harness, ConnTabId, TabId) {
+        let mut harness = Harness::new();
+        harness.set_look(look);
+        let tab = harness.connect_fake_as(false);
+        harness.press(Key::T, Modifiers::COMMAND);
+        harness.frame(vec![egui::Event::Paste(text.into())]);
+        harness.settle();
+        let id = harness.app.workspace(tab).unwrap().active_tab.unwrap();
+        (harness, tab, id)
+    }
+
+    fn set_mode(harness: &mut Harness, tab: ConnTabId, id: TabId, mode: RunMode) {
+        harness.app.apply(Action::SetSqlMode {
+            tab,
+            sql_tab: id,
+            mode,
+        });
+    }
+
+    /// Whether the button `name` cannot be pressed, and what it says of
+    /// that.
+    fn button(harness: &mut Harness, name: &str) -> (bool, Option<String>) {
+        let tree = harness.settle();
+        let id = node(&tree, name, Role::Button).unwrap_or_else(|| panic!("no {name}"));
+        let (_, node) = tree.nodes.iter().find(|(node, _)| *node == id).unwrap();
+        (node.is_disabled(), node.description().map(str::to_owned))
+    }
+
+    #[test]
+    fn run_and_run_all_wait_for_a_read_write_run_in_flight() {
+        for look in Look::ALL {
+            let (mut harness, tab, id) = editor(look, "DELETE FROM users WHERE id = 1");
+            set_mode(&mut harness, tab, id, RunMode::ReadWrite);
+            for name in ["Run", "Run all"] {
+                assert!(!button(&mut harness, name).0, "{name} in {}", look.name);
+            }
+            harness.press(Key::Enter, Modifiers::COMMAND);
+            let sent = harness.app.backend.sent.len();
+            for name in ["Run", "Run all"] {
+                assert_eq!(
+                    button(&mut harness, name),
+                    (true, Some(WRITING.to_owned())),
+                    "{name} in {}",
+                    look.name
+                );
+                // Nor does a press of it reach the run.
+                harness.click(name);
+            }
+            harness.press(Key::Enter, Modifiers::COMMAND | Modifiers::SHIFT);
+            assert_eq!(harness.app.backend.sent.len(), sent, "{}", look.name);
+            // The way to stop it stays.
+            assert!(harness.has("Cancel query"), "{}", look.name);
+            let done = vec![done_outcome(Some(1))];
+            let committed = write_outcome(done, tabletist_db::ScriptEnd::Committed);
+            harness.answer_sql(Ok(committed), None);
+            for name in ["Run", "Run all"] {
+                assert!(!button(&mut harness, name).0, "{name} in {}", look.name);
+            }
+        }
+    }
+
+    #[test]
+    fn run_stays_while_a_read_only_run_is_in_flight() {
+        let (mut harness, _, _) = editor(Look::standard(), "SELECT 1");
+        harness.press(Key::Enter, Modifiers::COMMAND);
+        for name in ["Run", "Run all"] {
+            assert!(!button(&mut harness, name).0, "{name}");
+        }
+    }
 
     #[test]
     fn the_editor_takes_its_share_of_the_room() {
````

- [ ] **Step 2: Run the tests to see them fail**

```bash
~/.cargo/bin/cargo test --locked -p tabletist --lib -- a_run_is_read_write_only a_statement_is_a_write nothing_is_sent_to_write run_does_nothing_while a_script_writes_only a_run_sent_to_write a_broken_outcome_is_why run_and_run_all_wait run_stays_while
```

Expected: does not compile. `variant backend::Command::RunSql has no field named mode`, `this method takes 2 arguments but 3 arguments were supplied` (`start_run`), `cannot find function broken_by`, `no method named is_writing`, `no field mode on type &SqlRun`, `cannot find value WRITING`.

- [ ] **Step 3: Write the code**

````diff
diff --git a/src/app.rs b/src/app.rs
index 23e9509..7ad981c 100644
--- a/src/app.rs
+++ b/src/app.rs
@@ -3193,19 +3193,26 @@ impl App {
     /// editor holding no statement (empty, or only comments) runs nothing.
     /// Nor does one whose session is not connected: the backend would
     /// answer that the connection is closed, and a run that fails as a
-    /// whole takes the result before it away.
+    /// whole takes the result before it away. Nor one whose read-write run
+    /// is still in flight: a new run would cancel it.
+    ///
+    /// The run is read-write only when the editor's runs are (see
+    /// `Workspace::run_mode`) and one of the statements looks like a
+    /// write. Every other run is the read-only one.
     fn run_sql(&mut self, tab: ConnTabId, id: TabId, all: bool) {
-        let request = RequestId(self.next_id());
-        let Some(workspace) = self.workspace_mut(tab) else {
+        let Some(workspace) = self.workspace(tab) else {
             return;
         };
         if !matches!(workspace.status, SessionStatus::Connected) {
             return;
         }
-        let (session, dialect) = (workspace.session, workspace.driver.dialect());
-        let Some(sql) = workspace.sql_tab_mut(id) else {
+        let dialect = workspace.driver.dialect();
+        let Some(sql) = workspace.sql_tab(id) else {
             return;
         };
+        if sql.is_writing() {
+            return;
+        }
         let mut statements = tabletist_db::sql::statements(dialect, &sql.text);
         if !all {
             statements = tabletist_db::sql::statement_at(&statements, sql.cursor)
@@ -3216,8 +3223,29 @@ impl App {
         if statements.is_empty() {
             return;
         }
+        let mode = script_mode(workspace.run_mode(sql), dialect, &statements);
+        self.send_run(tab, id, statements, mode);
+    }
+
+    /// Sends `statements` to the editor's session as its run, in a
+    /// transaction of `mode`.
+    fn send_run(
+        &mut self,
+        tab: ConnTabId,
+        id: TabId,
+        statements: Vec<tabletist_db::sql::Statement>,
+        mode: tabletist_db::ScriptMode,
+    ) {
+        let request = RequestId(self.next_id());
+        let Some(workspace) = self.workspace_mut(tab) else {
+            return;
+        };
+        let session = workspace.session;
+        let Some(sql) = workspace.sql_tab_mut(id) else {
+            return;
+        };
         let (limit, timeout) = (sql.limit, sql.timeout);
-        let superseded = sql.start_run(request, statements.clone());
+        let superseded = sql.start_run(request, statements.clone(), mode);
         // The session runs one thing at a time: the run still going must
         // stop before this one can start.
         self.cancel(session, superseded);
@@ -3227,6 +3255,7 @@ impl App {
             statements,
             limit,
             timeout,
+            mode,
         });
     }
 
@@ -3779,6 +3808,27 @@ fn apply_url(form: &mut ConnectionForm) -> Result<(), String> {
     Ok(())
 }
 
+/// The transaction a run of `statements` is sent in, in an editor whose
+/// runs have `mode`: a read-write one only in Read-write, and only when a
+/// statement looks like a write. So an editor left in Read-write opens no
+/// read-write transaction for its reads, and nothing writes from one in
+/// Read-only.
+fn script_mode(
+    mode: RunMode,
+    dialect: tabletist_db::Dialect,
+    statements: &[tabletist_db::sql::Statement],
+) -> tabletist_db::ScriptMode {
+    use tabletist_db::sql::{StatementKind, kind};
+    let writes = statements
+        .iter()
+        .any(|statement| kind(dialect, &statement.text) == StatementKind::Write);
+    if mode == RunMode::ReadWrite && writes {
+        tabletist_db::ScriptMode::Write
+    } else {
+        tabletist_db::ScriptMode::ReadOnly
+    }
+}
+
 /// Whether a finished SQL run opens Messages rather than Results. It does
 /// when the run failed, as a whole or in a statement; when its timeout
 /// stopped it (nobody asked for that, so the reason must show); and when
diff --git a/src/backend.rs b/src/backend.rs
index 0d8b6f0..795d41d 100644
--- a/src/backend.rs
+++ b/src/backend.rs
@@ -137,6 +137,9 @@ pub enum Command {
         /// How long the script may run before it is stopped; `None` lets
         /// it run until it ends or the user cancels it.
         timeout: Option<Duration>,
+        /// The transaction the script runs in: read-only and rolled back,
+        /// or read-write and committed when every statement succeeded.
+        mode: ScriptMode,
     },
     /// The server's name and version, for the SQL editor's footer.
     ServerVersion {
@@ -2182,6 +2185,7 @@ async fn run_session(
                 statements,
                 limit,
                 timeout,
+                mode,
             } => {
                 let timer = timeout.map(|after| {
                     Timer::start(
@@ -2193,16 +2197,18 @@ async fn run_session(
                     )
                 });
                 // Awaited to its end whatever stops it: the script rolls
-                // back and leaves the session as it found it.
+                // back, or commits, and leaves the session as it found it.
                 let result = connection
-                    .run_script(statements, *limit, ScriptMode::ReadOnly, &run_stop)
+                    .run_script(statements, *limit, *mode, &run_stop)
                     .await;
                 let timed_out = match timer {
                     Some(timer) => timer.end().await,
                     None => None,
                 };
                 let cancel = cancel_reason(&run_stop, timed_out, &result);
-                let lost = lost_error(&result);
+                // Told first, closed after: a run known to be committed is
+                // never shown as one that may be.
+                let lost = lost_error(&result).or_else(|| broken_by(&result));
                 outbox.emit(Event::SqlRan {
                     session: *session,
                     request: *request,
@@ -2258,6 +2264,13 @@ fn lost_error<T>(result: &Result<T, Error>) -> Option<Error> {
     }
 }
 
+/// Why the session a script ran on must be closed though the script
+/// answered: a run that writes ended, committed or rolled back, and the
+/// session could not be put back as it connected.
+fn broken_by(result: &Result<ScriptOutcome, Error>) -> Option<Error> {
+    result.as_ref().ok()?.broken.clone()
+}
+
 /// How long to wait after the first cancel before sending another. Each
 /// wait after that is twice as long, up to `CANCEL_AGAIN_MAX`.
 const CANCEL_AGAIN: Duration = Duration::from_millis(500);
diff --git a/src/model.rs b/src/model.rs
index 6a497bd..8bdd40f 100644
--- a/src/model.rs
+++ b/src/model.rs
@@ -2080,6 +2080,8 @@ pub enum NoWrites {
 pub struct SqlRun {
     /// The statements as split when the run started.
     pub statements: Vec<tabletist_db::sql::Statement>,
+    /// The transaction the run was sent in.
+    pub mode: tabletist_db::ScriptMode,
     pub outcome: tabletist_db::ScriptOutcome,
     /// Who stopped the run, if someone did.
     pub cancel: Option<crate::backend::CancelReason>,
@@ -2091,7 +2093,7 @@ pub struct SqlRun {
 
 impl SqlRun {
     fn new(
-        statements: Vec<tabletist_db::sql::Statement>,
+        (statements, mode): (Vec<tabletist_db::sql::Statement>, tabletist_db::ScriptMode),
         outcome: tabletist_db::ScriptOutcome,
         cancel: Option<crate::backend::CancelReason>,
     ) -> Self {
@@ -2113,6 +2115,7 @@ impl SqlRun {
             });
         Self {
             statements,
+            mode,
             outcome,
             cancel,
             error_mark,
@@ -2158,6 +2161,8 @@ impl TextPrint {
 pub struct RunInFlight {
     /// The statements as split when the run started.
     pub statements: Vec<tabletist_db::sql::Statement>,
+    /// The transaction the run was sent in.
+    pub mode: tabletist_db::ScriptMode,
     /// When the run started, for the elapsed time.
     pub started: std::time::Instant,
     /// The editor's text when the run started.
@@ -2445,18 +2450,21 @@ impl SqlTab {
         }
     }
 
-    /// Starts a run of `statements` as `request`. Returns the run it
-    /// replaces, if one was still pending, for the caller to cancel.
+    /// Starts a run of `statements` as `request`, sent in a transaction
+    /// of `mode`. Returns the run it replaces, if one was still pending,
+    /// for the caller to cancel.
     #[must_use]
     pub fn start_run(
         &mut self,
         request: RequestId,
         statements: Vec<tabletist_db::sql::Statement>,
+        mode: tabletist_db::ScriptMode,
     ) -> Option<RequestId> {
         let superseded = self.run.pending;
         self.run.start(request);
         self.in_flight = Some(RunInFlight {
             statements,
+            mode,
             started: std::time::Instant::now(),
             text: TextPrint::of(&self.text),
         });
@@ -2482,10 +2490,12 @@ impl SqlTab {
             self.run.value = None;
             self.run.loaded = None;
         }
-        let statements = in_flight.map(|run| run.statements).unwrap_or_default();
+        let ran = in_flight
+            .map(|run| (run.statements, run.mode))
+            .unwrap_or_default();
         self.run.finish(
             request,
-            result.map(|outcome| SqlRun::new(statements, outcome, cancel)),
+            result.map(|outcome| SqlRun::new(ran, outcome, cancel)),
         )
     }
 
@@ -2500,6 +2510,15 @@ impl SqlTab {
         self.run.is_loading()
     }
 
+    /// Whether the run in flight was sent to write. Such a run is never
+    /// replaced by another: that would cancel a transaction that may be
+    /// on its way to its commit. Cancel stops it.
+    pub fn is_writing(&self) -> bool {
+        self.in_flight
+            .as_ref()
+            .is_some_and(|run| run.mode == tabletist_db::ScriptMode::Write)
+    }
+
     /// How long the run in flight has been going.
     pub fn running_for(&self) -> Option<Duration> {
         self.in_flight.as_ref().map(|run| run.started.elapsed())
diff --git a/src/ui/sql_editor.rs b/src/ui/sql_editor.rs
index 34acfea..c414036 100644
--- a/src/ui/sql_editor.rs
+++ b/src/ui/sql_editor.rs
@@ -22,6 +22,10 @@ const SIDE: f32 = 16.0;
 /// The least height of the editor, and of the results under it.
 const MIN_PANE: f32 = 80.0;
 
+/// Why Run and Run all cannot be pressed while a read-write run is in
+/// flight.
+const WRITING: &str = "A read-write run is still going in this tab. Cancel it or wait for it.";
+
 /// The height of what divides the editor from the results: a band to drag
 /// between two rules, or the terminal's one rule.
 fn splitter_height(look: &Look) -> f32 {
@@ -276,6 +280,8 @@ struct Bar<'a> {
     secs: Option<u32>,
     limit_label: &'a MenuLabel,
     timeout_label: &'a MenuLabel,
+    /// The editor's read-write run is in flight: Run and Run all wait.
+    writing: bool,
     locale: Locale,
     look: &'a Look,
     palette: &'a Palette,
@@ -313,6 +319,7 @@ fn toolbar(app: &mut App, ui: &mut Ui, tab: ConnTabId, id: TabId) {
         secs,
         limit_label: &limit_label,
         timeout_label: &timeout_label,
+        writing: sql.is_writing(),
         locale,
         look: &look,
         palette: &palette,
@@ -438,7 +445,9 @@ fn explain_note(ui: &Ui, rect: Rect, locale: Locale) {
     ));
 }
 
-/// Run or Run all in `rect`, with what it does on hover.
+/// Run or Run all in `rect`, with what it does on hover. Neither can be
+/// pressed while the editor's read-write run is in flight, and says so: a
+/// new run would cancel a transaction that may be about to commit.
 fn run_button(
     ui: &mut Ui,
     button: ButtonSpec<'_>,
@@ -455,6 +464,13 @@ fn run_button(
             "Run the statement at the cursor"
         },
     );
+    if bar.writing {
+        let waits = gettext(bar.locale, WRITING);
+        button
+            .disabled(&waits)
+            .show_at(ui, rect, bar.look, bar.palette);
+        return;
+    }
     let response = button.show_at(ui, rect, bar.look, bar.palette);
     if response.on_hover_text(explained).clicked() {
         actions.push(Action::RunSql {
````

- [ ] **Step 4: Run the tests to see them pass**

```bash
~/.cargo/bin/cargo test --locked -p tabletist --lib -- a_run_is_read_write_only a_statement_is_a_write nothing_is_sent_to_write run_does_nothing_while a_script_writes_only a_run_sent_to_write a_broken_outcome_is_why run_and_run_all_wait run_stays_while
```

Expected: PASS, 9 passed. Two of them run a real SQLite session through the backend: a script writes only in a run sent to write, and a run sent to write on a read-only session answers `Error::ReadOnly` and keeps the session.

- [ ] **Step 5: Run the four checks**

All four pass. Fix what does not before going on; do not weaken a lint to get there.

- [ ] **Step 6: Commit**

```bash
git add src/backend.rs src/model.rs src/app.rs src/ui/sql_editor.rs src/testing.rs
git commit -S -m "Send a run to write when a read-write tab runs a write" -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 4: How a read-write run ended: Messages, Results and the footer

**Files:**
- Modify: `src/ui/sql_results.rs` (`Tone`, `Line`, `lines`, `end_lines`, `end_message`, `statement_message`, `results`, and its tests module)
- Modify: `src/ui/sql_editor.rs` (`run_summary`, `end_words`, `end_note`, `status_summary`, and its tests module)
- Modify: `src/model.rs` (`SqlTab::lost_writing`)
- Modify: `src/app.rs` (`opens_messages`, and its tests module)

A read-only run reads exactly as it did; one test pins that. For a run sent to write (`SqlRun::mode`), after the statements' lines:

| End | The line |
|---|---|
| `Committed` | "Committed · 3 statements · 21 ms" (the statements' times, summed) |
| `RolledBack` | "Rolled back. Nothing was written." |
| `Partly { committed }` | "Lines 1 to 4 are written: MySQL commits CREATE, ALTER, DROP and similar statements as they run. The rest was rolled back." ("Line 1 is written:" for one line) |
| `CommitFailed { committed: 0 }` | "The commit failed. Nothing was written.", then the database's error with its code, detail and hint |
| `CommitFailed { committed > 0 }` | "The commit failed.", `Partly`'s sentences, then the error |
| with `rollback_warning` | "MySQL could not roll back every change." in place of the `RolledBack` line, of "The rest was rolled back." and of a failed commit's "Nothing was written.", then the warning's text |
| with `broken` | the end's own line, then "The session could not be put back and was closed." |

A statement whose work the end undid closes its line with "· rolled back" (not where there is a `rollback_warning`, and not the statements MySQL had committed). MySQL's warning count follows the time: "· 2 warnings". Results, with no rows to show, reads "Statement ran · 12 rows affected" where the last statement counted rows. A run MySQL committed by itself under a stop that came too late is written, so Results says that its statement ran, not only "Cancelled". The footer reads `12 rows affected · 14 ms` and `Read-write transaction · committed` (or `rolled back`, `partly committed`, `commit failed`, `not fully rolled back`); the terminal's status line ends `· 12 rows affected · 14 ms · committed`. The Messages open by themselves for `Partly`, `CommitFailed`, a `rollback_warning` and `broken`.

A run sent to write that failed as a whole because its session was lost says under its error that some or all of it may be written. A run that failed as a whole leaves no `SqlRun`, so the tab remembers the mode of its last run (`ran_mode`, beside `ran_text`).

In the terminal look these sentences are in lower case throughout, the database's name and the statements' keywords too (`Words::own`), as its design writes "postgres refused the update". What a database itself said keeps its case, as everywhere.

- [ ] **Step 1: Write the failing tests**

Apply this block. It holds the task's tests, and applies on the tree as the task before left it.

````diff
diff --git a/src/app.rs b/src/app.rs
index 7ad981c..96edea7 100644
--- a/src/app.rs
+++ b/src/app.rs
@@ -6868,6 +6877,39 @@ mod tests {
         assert_eq!(runs_since(&harness, sent), 1);
     }
 
+    #[test]
+    fn messages_open_for_every_end_of_a_read_write_run_that_needs_reading() {
+        use crate::testing::{done_outcome, write_outcome};
+        use tabletist_db::ScriptEnd;
+        let done = || vec![done_outcome(Some(1))];
+        let failed = ScriptEnd::CommitFailed {
+            error: Error::query("a deferred constraint"),
+            committed: 0,
+        };
+        let mut kept = write_outcome(done(), ScriptEnd::RolledBack);
+        kept.rollback_warning = Some("could not be rolled back".into());
+        let mut closed = write_outcome(done(), ScriptEnd::Committed);
+        closed.broken = Some(Error::ConnectionLost(
+            "could not end the transaction".into(),
+        ));
+        for (outcome, messages) in [
+            (write_outcome(done(), ScriptEnd::Committed), false),
+            (
+                write_outcome(done(), ScriptEnd::Partly { committed: 1 }),
+                true,
+            ),
+            (write_outcome(done(), failed), true),
+            (kept, true),
+            (closed, true),
+        ] {
+            assert_eq!(
+                opens_messages(&Ok(outcome.clone()), None),
+                messages,
+                "{outcome:?}"
+            );
+        }
+    }
+
     #[test]
     fn the_limit_and_timeout_menus_change_the_next_run_and_the_settings() {
         let mut harness = Harness::new();
diff --git a/src/ui/sql_editor.rs b/src/ui/sql_editor.rs
index c414036..2b0e8ba 100644
--- a/src/ui/sql_editor.rs
+++ b/src/ui/sql_editor.rs
@@ -1021,6 +1077,84 @@ mod tests {
         }
     }
 
+    #[test]
+    fn the_footer_says_how_a_read_write_run_ended() {
+        use tabletist_db::{Error, ScriptEnd};
+        let failed = |committed| ScriptEnd::CommitFailed {
+            error: Error::query("a deferred constraint"),
+            committed,
+        };
+        let warned = Some("could not be rolled back".to_owned());
+        for (end, warning, words) in [
+            (ScriptEnd::Committed, None, "committed"),
+            (ScriptEnd::RolledBack, None, "rolled back"),
+            (ScriptEnd::Partly { committed: 1 }, None, "partly committed"),
+            (failed(0), None, "commit failed"),
+            // Part of it is written all the same: said as what it left.
+            (failed(1), None, "partly committed"),
+            (
+                ScriptEnd::RolledBack,
+                warned.clone(),
+                "not fully rolled back",
+            ),
+            (
+                ScriptEnd::Partly { committed: 1 },
+                warned,
+                "not fully rolled back",
+            ),
+        ] {
+            for look in Look::ALL {
+                let script = "CREATE TABLE notes (n);\nUPDATE notes SET n = 1";
+                let (mut harness, tab, id) = editor(look, script);
+                set_mode(&mut harness, tab, id, RunMode::ReadWrite);
+                harness.press(Key::Enter, Modifiers::COMMAND | Modifiers::SHIFT);
+                // Nothing is said of a run that is still going.
+                assert!(!harness.has("Read-write transaction · committed"));
+                let mut outcome = write_outcome(
+                    vec![done_outcome(None), done_outcome(Some(12))],
+                    end.clone(),
+                );
+                outcome.rollback_warning = warning.clone();
+                harness.answer_sql(Ok(outcome), None);
+                let context = format!("{words} in {}", look.name);
+                if look.terminal {
+                    // The terminal's status line, which has no footer.
+                    let status = status_summary(&harness.app, tab).expect("an editor shows");
+                    let (_, said) = status.split_once(" · ").expect("after the cursor");
+                    assert_eq!(
+                        said,
+                        format!("12 rows affected · 14 ms · {words}"),
+                        "{context}"
+                    );
+                } else {
+                    assert!(harness.has("12 rows affected · 14 ms"), "{context}");
+                    let note = format!("Read-write transaction · {words}");
+                    assert!(harness.has(&note), "{note} in {}", look.name);
+                    assert!(!harness.has("Read-only transaction · rolled back"));
+                }
+            }
+        }
+    }
+
+    #[test]
+    fn a_run_of_reads_in_a_read_write_tab_is_said_to_be_read_only() {
+        let (mut harness, tab, id) = editor(Look::standard(), "SELECT 1");
+        set_mode(&mut harness, tab, id, RunMode::ReadWrite);
+        harness.press(Key::Enter, Modifiers::COMMAND);
+        let rows = crate::testing::script_outcome(vec![crate::testing::rows_outcome(3)]);
+        harness.answer_sql(Ok(rows), None);
+        assert!(harness.has("3 rows · 14 ms"));
+        assert!(harness.has("Read-only transaction · rolled back"));
+        // A statement that counted rows in a read-only run is no change
+        // that stayed: the footer does not count it.
+        let (mut harness, _, _) = editor(Look::standard(), "UPDATE notes SET n = 1");
+        harness.press(Key::Enter, Modifiers::COMMAND);
+        let counted = crate::testing::script_outcome(vec![done_outcome(Some(12))]);
+        harness.answer_sql(Ok(counted), None);
+        assert!(!harness.has("12 rows affected · 14 ms"));
+        assert!(harness.has("Read-only transaction · rolled back"));
+    }
+
     #[test]
     fn run_stays_while_a_read_only_run_is_in_flight() {
         let (mut harness, _, _) = editor(Look::standard(), "SELECT 1");
diff --git a/src/ui/sql_results.rs b/src/ui/sql_results.rs
index e759479..d79b13f 100644
--- a/src/ui/sql_results.rs
+++ b/src/ui/sql_results.rs
@@ -1180,14 +1469,14 @@ mod tests {
 
     use egui::accesskit::Role;
     use egui::{Key, Modifiers};
-    use tabletist_db::{Error, StatementOutcome};
+    use tabletist_db::{Driver, Error, ScriptEnd, StatementOutcome};
 
     use super::*;
     use crate::backend::{CancelReason, Command};
     use crate::model::{Action, CellPos, ResultPane, SqlTab};
     use crate::testing::{
         Harness, bounds, error_outcome, labels, node, rows_outcome, script_outcome,
-        stopped_before_it_began,
+        stopped_before_it_began, write_outcome,
     };
     use crate::theme::Look;
 
@@ -1926,6 +2215,608 @@ mod tests {
         }
     }
 
+    /// A SQL editor switched to Read-write on a connection that takes
+    /// writes, drawn in `look`, with all of `text` sent as its run. The
+    /// session speaks as `driver` does.
+    fn writing(look: Look, driver: Driver, text: &str) -> (Harness, ConnTabId) {
+        let mut harness = Harness::new();
+        harness.set_look(look);
+        let tab = harness.connect_fake_as(false);
+        harness.app.workspace_mut(tab).unwrap().driver = driver;
+        harness.press(Key::T, Modifiers::COMMAND);
+        harness.frame(vec![egui::Event::Paste(text.into())]);
+        harness.settle();
+        let sql_tab = sql(&harness, tab).id;
+        harness.app.apply(Action::SetSqlMode {
+            tab,
+            sql_tab,
+            mode: crate::model::RunMode::ReadWrite,
+        });
+        run_all(&mut harness);
+        assert!(sql(&harness, tab).is_writing(), "the run was sent to write");
+        (harness, tab)
+    }
+
+    /// The lines the Messages pane shows, from its top: what stands under
+    /// the header's tabs and over the editor's footer (the terminal look's
+    /// status line).
+    fn message_lines(harness: &mut Harness) -> Vec<String> {
+        let tree = harness.settle();
+        let top = bounds(&tree, "Messages", Role::Button)
+            .expect("the Messages tab")
+            .bottom();
+        let labelled = |(_, node): &(_, egui::accesskit::Node)| {
+            let at = node.bounds()?.y0 as f32;
+            Some((at, node.value()?.to_owned()))
+        };
+        let mut found: Vec<(f32, String)> = tree
+            .nodes
+            .iter()
+            .filter(|(_, node)| node.role() == Role::Label)
+            .filter_map(labelled)
+            .filter(|(at, _)| *at > top)
+            .collect();
+        found.sort_by(|(a, _), (b, _)| a.total_cmp(b));
+        // The cursor's place is the first thing under the pane.
+        let under = |text: &str| text.starts_with("Ln ") || text.starts_with("ln ");
+        let bottom = found
+            .iter()
+            .find(|(_, text)| under(text))
+            .map_or(f32::INFINITY, |(at, _)| *at);
+        found.retain(|(at, _)| *at < bottom);
+        found.into_iter().map(|(_, text)| text).collect()
+    }
+
+    /// Three statements that change rows.
+    const CHANGES: &str = "INSERT INTO notes VALUES (1);\nUPDATE notes SET seen = 1;\nDELETE FROM notes WHERE seen = 0";
+
+    #[test]
+    fn a_committed_run_says_so_after_what_each_statement_changed() {
+        for look in Look::ALL {
+            let (mut harness, tab) = writing(look, Driver::Postgres, CHANGES);
+            let outcome = write_outcome(
+                vec![done(Some(1)), done(Some(12)), done(Some(1))],
+                ScriptEnd::Committed,
+            );
+            harness.answer_sql(Ok(outcome), None);
+            // Nothing went wrong: Results shows, with what the last
+            // statement changed in place of rows.
+            assert_eq!(sql(&harness, tab).pane, ResultPane::Results);
+            assert!(
+                harness.has("Statement ran · 1 row affected"),
+                "{}",
+                look.name
+            );
+            assert_eq!(count(&mut harness), None);
+            harness.click("Messages");
+            assert_eq!(
+                message_lines(&mut harness),
+                [
+                    "Line 1: 1 row affected · 14 ms",
+                    "Line 2: 12 rows affected · 14 ms",
+                    "Line 3: 1 row affected · 14 ms",
+                    "Committed · 3 statements · 42 ms",
+                ],
+                "{}",
+                look.name
+            );
+            let reads = if look.terminal {
+                "committed · 3 statements · 42 ms"
+            } else {
+                "Committed · 3 statements · 42 ms"
+            };
+            assert!(painted(&harness, reads), "{reads}: {:?}", harness.painted);
+        }
+    }
+
+    #[test]
+    fn a_committed_statement_that_counts_no_rows_says_it_ran() {
+        let (mut harness, _tab) =
+            writing(Look::standard(), Driver::Sqlite, "CREATE TABLE notes (n)");
+        harness.answer_sql(
+            Ok(write_outcome(vec![done(None)], ScriptEnd::Committed)),
+            None,
+        );
+        assert!(harness.has("Statement ran · no rows returned"));
+        harness.click("Messages");
+        assert_eq!(
+            message_lines(&mut harness),
+            [
+                "Line 1: Statement ran · 14 ms",
+                "Committed · 1 statement · 14 ms"
+            ]
+        );
+    }
+
+    #[test]
+    fn a_run_that_failed_says_it_was_rolled_back_and_that_each_change_is_gone() {
+        for look in Look::ALL {
+            let script = format!("{CHANGES};\nSELECT nope;\nSELECT 2");
+            let (mut harness, tab) = writing(look, Driver::Postgres, &script);
+            let outcome = write_outcome(
+                vec![
+                    done(Some(1)),
+                    done(Some(12)),
+                    done(Some(1)),
+                    error_outcome("column \"nope\" does not exist", None),
+                ],
+                ScriptEnd::RolledBack,
+            );
+            harness.answer_sql(Ok(outcome), None);
+            assert_eq!(sql(&harness, tab).pane, ResultPane::Messages);
+            assert_eq!(
+                message_lines(&mut harness),
+                [
+                    "Line 1: 1 row affected · 14 ms · rolled back",
+                    "Line 2: 12 rows affected · 14 ms · rolled back",
+                    "Line 3: 1 row affected · 14 ms · rolled back",
+                    "Line 4: column \"nope\" does not exist",
+                    "Line 5: Not run",
+                    "Rolled back. Nothing was written.",
+                ],
+                "{}",
+                look.name
+            );
+            // Results says where it failed, never that a statement ran.
+            harness.click("Results");
+            assert!(!says_it_ran(&mut harness), "{}", look.name);
+        }
+    }
+
+    #[test]
+    fn a_read_only_run_reads_as_it_always_did() {
+        // The same answers in the run every editor had before: no line for
+        // an end, and no count said to be rolled back.
+        let (mut harness, _tab) = editor(Look::standard(), CHANGES);
+        run_all(&mut harness);
+        harness.answer_sql(
+            Ok(script_outcome(vec![
+                StatementOutcome::Done {
+                    affected: Some(3),
+                    warnings: 2,
+                },
+                error_outcome("cannot execute UPDATE in a read-only transaction", None),
+            ])),
+            None,
+        );
+        assert_eq!(
+            message_lines(&mut harness),
+            [
+                "Line 1: 3 rows affected · 14 ms",
+                "Line 2: cannot execute UPDATE in a read-only transaction",
+                "Line 3: Not run",
+            ]
+        );
+    }
+
+    #[test]
+    fn a_cancelled_or_timed_out_run_says_it_was_rolled_back() {
+        let timeout = CancelReason::Timeout(Duration::from_secs(30));
+        for (cancel, text) in [
+            (CancelReason::User, "Cancelled"),
+            (timeout, "Cancelled after 30 s (timeout)"),
+        ] {
+            let (mut harness, tab) = writing(Look::standard(), Driver::Postgres, CHANGES);
+            let outcome = write_outcome(
+                vec![done(Some(1)), StatementOutcome::Cancelled],
+                ScriptEnd::RolledBack,
+            );
+            harness.answer_sql(Ok(outcome), Some(cancel));
+            assert_eq!(sql(&harness, tab).pane, ResultPane::Messages);
+            assert_eq!(
+                message_lines(&mut harness),
+                [
+                    "Line 1: 1 row affected · 14 ms · rolled back".to_owned(),
+                    format!("Line 2: {text}"),
+                    "Line 3: Not run".to_owned(),
+                    "Rolled back. Nothing was written.".to_owned(),
+                ]
+            );
+        }
+        // A stop that came with every statement done has no statement's
+        // line to say it: the end says it first.
+        let (mut harness, _tab) = writing(Look::standard(), Driver::Postgres, CHANGES);
+        let mut outcome = write_outcome(
+            vec![done(Some(1)), done(Some(12)), done(Some(1))],
+            ScriptEnd::RolledBack,
+        );
+        outcome.stopped = true;
+        harness.answer_sql(Ok(outcome), Some(CancelReason::User));
+        assert_eq!(
+            message_lines(&mut harness)[3..],
+            ["Cancelled", "Rolled back. Nothing was written."]
+        );
+    }
+
+    #[test]
+    fn a_run_mysql_committed_part_of_says_which_lines_are_written() {
+        for look in Look::ALL {
+            let script = "INSERT INTO notes VALUES (1);\nCREATE TABLE more (n int);\n\
+                          INSERT INTO notes VALUES (2);\nSELECT nope";
+            let (mut harness, tab) = writing(look, Driver::MySql, script);
+            let outcome = write_outcome(
+                vec![
+                    done(Some(1)),
+                    done(None),
+                    done(Some(1)),
+                    error_outcome("Unknown column 'nope'", None),
+                ],
+                ScriptEnd::Partly { committed: 2 },
+            );
+            harness.answer_sql(Ok(outcome), None);
+            assert_eq!(sql(&harness, tab).pane, ResultPane::Messages);
+            assert_eq!(
+                message_lines(&mut harness),
+                [
+                    // What is written keeps its count as it is.
+                    "Line 1: 1 row affected · 14 ms",
+                    "Line 2: Statement ran · 14 ms",
+                    "Line 3: 1 row affected · 14 ms · rolled back",
+                    "Line 4: Unknown column 'nope'",
+                    "Lines 1 to 2 are written: MySQL commits CREATE, ALTER, DROP and similar \
+                     statements as they run. The rest was rolled back.",
+                ],
+                "{}",
+                look.name
+            );
+            // The terminal look writes the whole of it in lower case, the
+            // database's name and its statements too.
+            if look.terminal {
+                let end = harness
+                    .painted
+                    .iter()
+                    .find(|(piece, _)| piece.contains("are written:"))
+                    .map(|(piece, _)| piece.clone())
+                    .expect("the end's line");
+                assert!(end.starts_with(
+                    "lines 1 to 2 are written: mysql commits create, alter, drop and"
+                ));
+                assert_eq!(
+                    harness.painted_color(&end),
+                    Some(harness.app.palette.warning)
+                );
+            }
+        }
+    }
+
+    #[test]
+    fn one_committed_statement_is_one_line_that_is_written() {
+        let script = "CREATE TABLE more (n int);\nSELECT nope";
+        let (mut harness, _tab) = writing(Look::standard(), Driver::MySql, script);
+        let outcome = write_outcome(
+            vec![done(None), error_outcome("Unknown column 'nope'", None)],
+            ScriptEnd::Partly { committed: 1 },
+        );
+        harness.answer_sql(Ok(outcome), None);
+        assert!(harness.has(
+            "Line 1 is written: MySQL commits CREATE, ALTER, DROP and similar statements as \
+             they run. The rest was rolled back."
+        ));
+    }
+
+    /// Why a commit failed, as PostgreSQL says it of a deferred constraint.
+    fn deferred() -> Error {
+        Error::Query {
+            code: Some("23503".into()),
+            message: "insert or update on table \"notes\" violates foreign key constraint".into(),
+            detail: Some("Key (owner)=(9) is not present in table \"users\".".into()),
+            hint: None,
+        }
+    }
+
+    #[test]
+    fn a_commit_that_failed_says_nothing_was_written_and_why() {
+        for look in Look::ALL {
+            let (mut harness, tab) = writing(look, Driver::Postgres, CHANGES);
+            let outcome = write_outcome(
+                vec![done(Some(1)), done(Some(12)), done(Some(1))],
+                ScriptEnd::CommitFailed {
+                    error: deferred(),
+                    committed: 0,
+                },
+            );
+            harness.answer_sql(Ok(outcome), None);
+            // No statement failed, and still the Messages open.
+            assert_eq!(sql(&harness, tab).pane, ResultPane::Messages);
+            assert_eq!(
+                message_lines(&mut harness),
+                [
+                    "Line 1: 1 row affected · 14 ms · rolled back",
+                    "Line 2: 12 rows affected · 14 ms · rolled back",
+                    "Line 3: 1 row affected · 14 ms · rolled back",
+                    "The commit failed. Nothing was written.",
+                    "insert or update on table \"notes\" violates foreign key constraint",
+                    "Code: 23503",
+                    "Detail: Key (owner)=(9) is not present in table \"users\".",
+                ],
+                "{}",
+                look.name
+            );
+            // The commit's error counts beside the tab as a statement's.
+            let tree = harness.settle();
+            let id = node(&tree, "Messages", Role::Button).expect("the Messages tab");
+            let (_, messages) = tree.nodes.iter().find(|(node, _)| *node == id).unwrap();
+            assert_eq!(messages.value(), Some("1"), "{}", look.name);
+            // Results says the same, never that a statement ran.
+            harness.click("Results");
+            assert!(harness.has("The commit failed. Nothing was written."));
+            assert!(!says_it_ran(&mut harness), "{}", look.name);
+        }
+    }
+
+    #[test]
+    fn a_failed_commit_after_mysql_committed_part_says_what_is_written() {
+        let script = "CREATE TABLE more (n int);\nINSERT INTO more VALUES (1)";
+        let (mut harness, _tab) = writing(Look::standard(), Driver::MySql, script);
+        let outcome = write_outcome(
+            vec![done(None), done(Some(1))],
+            ScriptEnd::CommitFailed {
+                error: Error::query("Deadlock found when trying to get lock"),
+                committed: 1,
+            },
+        );
+        harness.answer_sql(Ok(outcome), None);
+        assert_eq!(
+            message_lines(&mut harness),
+            [
+                "Line 1: Statement ran · 14 ms",
+                "Line 2: 1 row affected · 14 ms · rolled back",
+                "The commit failed. Line 1 is written: MySQL commits CREATE, ALTER, DROP and \
+                 similar statements as they run. The rest was rolled back.",
+                "Deadlock found when trying to get lock",
+            ]
+        );
+        assert!(!harness.has("The commit failed. Nothing was written."));
+    }
+
+    #[test]
+    fn what_mysql_could_not_roll_back_is_said_and_nothing_is_said_to_be_gone() {
+        const MYISAM: &str = "Some non-transactional changed tables couldn't be rolled back";
+        let says_gone = |harness: &mut Harness| {
+            message_lines(harness).iter().any(|line| {
+                line.contains("Nothing was written")
+                    || line.contains("The rest was rolled back")
+                    || line.ends_with("· rolled back")
+            })
+        };
+        for look in Look::ALL {
+            // Rolled back, as far as the database could.
+            let script = "INSERT INTO logs VALUES (1);\nSELECT nope";
+            let (mut harness, tab) = writing(look, Driver::MySql, script);
+            let mut outcome = write_outcome(
+                vec![done(Some(1)), error_outcome("Unknown column 'nope'", None)],
+                ScriptEnd::RolledBack,
+            );
+            outcome.rollback_warning = Some(MYISAM.into());
+            harness.answer_sql(Ok(outcome), None);
+            assert_eq!(sql(&harness, tab).pane, ResultPane::Messages);
+            assert_eq!(
+                message_lines(&mut harness),
+                [
+                    "Line 1: 1 row affected · 14 ms",
+                    "Line 2: Unknown column 'nope'",
+                    "MySQL could not roll back every change.",
+                    MYISAM,
+                ],
+                "{}",
+                look.name
+            );
+            assert!(!says_gone(&mut harness), "{}", look.name);
+            // What the database said keeps its case in every look.
+            assert_eq!(
+                harness.painted_color(MYISAM),
+                Some(harness.app.palette.warning),
+                "{}",
+                look.name
+            );
+        }
+        // With part of the run written by the database itself.
+        let script = "CREATE TABLE more (n int);\nINSERT INTO logs VALUES (1);\nSELECT nope";
+        let (mut harness, _tab) = writing(Look::standard(), Driver::MySql, script);
+        let mut outcome = write_outcome(
+            vec![
+                done(None),
+                done(Some(1)),
+                error_outcome("Unknown column 'nope'", None),
+            ],
+            ScriptEnd::Partly { committed: 1 },
+        );
+        outcome.rollback_warning = Some(MYISAM.into());
+        harness.answer_sql(Ok(outcome), None);
+        assert_eq!(
+            message_lines(&mut harness)[3..],
+            [
+                "Line 1 is written: MySQL commits CREATE, ALTER, DROP and similar statements as \
+                 they run. MySQL could not roll back every change.",
+                MYISAM,
+            ]
+        );
+        assert!(!says_gone(&mut harness));
+        // A commit that failed, and a rollback after it that could not
+        // undo everything: nothing says "Nothing was written" then either,
+        // in the Messages or in Results.
+        let script = "INSERT INTO logs VALUES (1)";
+        let (mut harness, _tab) = writing(Look::standard(), Driver::MySql, script);
+        let failed = ScriptEnd::CommitFailed {
+            error: Error::query("Deadlock found when trying to get lock"),
+            committed: 0,
+        };
+        let mut outcome = write_outcome(vec![done(Some(1))], failed);
+        outcome.rollback_warning = Some(MYISAM.into());
+        harness.answer_sql(Ok(outcome), None);
+        assert_eq!(
+            message_lines(&mut harness),
+            [
+                "Line 1: 1 row affected · 14 ms",
+                "The commit failed. MySQL could not roll back every change.",
+                "Deadlock found when trying to get lock",
+                MYISAM,
+            ]
+        );
+        assert!(!says_gone(&mut harness));
+        harness.click("Results");
+        assert!(harness.has("The commit failed. MySQL could not roll back every change."));
+        assert!(!harness.has("The commit failed. Nothing was written."));
+    }
+
+    #[test]
+    fn a_run_mysql_committed_under_a_late_stop_reads_as_written() {
+        // The stop came while the last statement ran, and that statement
+        // made the server commit: nothing was left to roll back.
+        let script = "CREATE TABLE more (n int)";
+        let (mut harness, _tab) = writing(Look::standard(), Driver::MySql, script);
+        let mut outcome = write_outcome(vec![done(None)], ScriptEnd::Committed);
+        outcome.stopped = true;
+        harness.answer_sql(Ok(outcome), Some(CancelReason::User));
+        assert_eq!(
+            message_lines(&mut harness),
+            [
+                "Line 1: Statement ran · 14 ms",
+                "Cancelled",
+                "Committed · 1 statement · 14 ms",
+            ]
+        );
+        // Results does not say of a run that is written only that it was
+        // cancelled.
+        harness.click("Results");
+        assert!(harness.has("Statement ran · no rows returned"));
+        assert!(!harness.has("Cancelled"));
+    }
+
+    #[test]
+    fn a_run_whose_session_could_not_be_put_back_keeps_its_end_and_says_it_was_closed() {
+        for look in Look::ALL {
+            let (mut harness, tab) = writing(look, Driver::Postgres, CHANGES);
+            let mut outcome = write_outcome(
+                vec![done(Some(1)), done(Some(12)), done(Some(1))],
+                ScriptEnd::Committed,
+            );
+            outcome.broken = Some(Error::ConnectionLost(
+                "could not end the transaction".into(),
+            ));
+            harness.answer_sql(Ok(outcome), None);
+            assert_eq!(sql(&harness, tab).pane, ResultPane::Messages);
+            assert_eq!(
+                message_lines(&mut harness)[3..],
+                [
+                    "Committed · 3 statements · 42 ms",
+                    "The session could not be put back and was closed.",
+                ],
+                "{}",
+                look.name
+            );
+            // The backend closes the session once it has told the end: the
+            // reconnect banner comes, and the end stays said under it.
+            let session = harness.app.workspace(tab).unwrap().session;
+            let closed = Error::ConnectionLost("could not end the transaction".into());
+            harness
+                .app
+                .apply(Action::Backend(crate::backend::Event::Disconnected {
+                    session,
+                    error: closed,
+                }));
+            assert!(harness.has("Reconnect"), "{}", look.name);
+            assert_eq!(
+                message_lines(&mut harness)[3..],
+                [
+                    "Committed · 3 statements · 42 ms",
+                    "The session could not be put back and was closed.",
+                ],
+                "{}",
+                look.name
+            );
+        }
+    }
+
+    #[test]
+    fn a_statement_that_returned_rows_is_rolled_back_only_where_it_looks_like_a_write() {
+        let script =
+            "SELECT * FROM notes;\nINSERT INTO notes VALUES (1) RETURNING id;\nSELECT nope";
+        let (mut harness, _tab) = writing(Look::standard(), Driver::Postgres, script);
+        let outcome = write_outcome(
+            vec![
+                rows_outcome(2),
+                rows_outcome(1),
+                error_outcome("column \"nope\" does not exist", None),
+            ],
+            ScriptEnd::RolledBack,
+        );
+        harness.answer_sql(Ok(outcome), None);
+        assert_eq!(
+            message_lines(&mut harness)[..2],
+            [
+                "Line 1: 2 rows · 14 ms",
+                "Line 2: 1 row · 14 ms · rolled back"
+            ]
+        );
+    }
+
+    #[test]
+    fn a_statements_warnings_are_counted_on_its_line() {
+        let script = "INSERT INTO notes VALUES ('too long');\nUPDATE notes SET n = 'x'";
+        let (mut harness, _tab) = writing(Look::standard(), Driver::MySql, script);
+        let warned = |affected, warnings| StatementOutcome::Done {
+            affected: Some(affected),
+            warnings,
+        };
+        let outcome = write_outcome(vec![warned(1, 1), warned(12, 2)], ScriptEnd::Committed);
+        harness.answer_sql(Ok(outcome), None);
+        harness.click("Messages");
+        assert_eq!(
+            message_lines(&mut harness)[..2],
+            [
+                "Line 1: 1 row affected · 14 ms · 1 warning",
+                "Line 2: 12 rows affected · 14 ms · 2 warnings",
+            ]
+        );
+    }
+
+    #[test]
+    fn a_connection_lost_during_a_read_write_run_says_it_may_be_written() {
+        const UNKNOWN: &str =
+            "The connection was lost during a read-write run. Some or all of it may be written.";
+        for look in Look::ALL {
+            let (mut harness, tab) = writing(look, Driver::Postgres, CHANGES);
+            let lost = Error::ConnectionLost("the server went away".into());
+            harness.answer_sql(Err(lost.clone()), None);
+            assert_eq!(sql(&harness, tab).pane, ResultPane::Messages);
+            assert_eq!(
+                message_lines(&mut harness),
+                [lost.to_string(), UNKNOWN.to_owned()],
+                "{}",
+                look.name
+            );
+            harness.click("Results");
+            assert!(harness.has(&format!("{lost}. {UNKNOWN}")), "{}", look.name);
+        }
+        // A script that ended its own transaction: the error says why the
+        // session went, and what is written is as unknown.
+        let (mut harness, _tab) = writing(Look::standard(), Driver::Sqlite, CHANGES);
+        harness.answer_sql(Err(Error::LeftTransaction), None);
+        assert_eq!(
+            message_lines(&mut harness),
+            [
+                Error::LeftTransaction.to_string(),
+                "Some or all of the run may be written.".to_owned()
+            ]
+        );
+        // Lost in a read-only run, nothing can have been written.
+        let (mut harness, _tab) = editor(Look::standard(), "SELECT 1");
+        run(&mut harness);
+        let lost = Error::ConnectionLost("the server went away".into());
+        harness.answer_sql(Err(lost.clone()), None);
+        assert_eq!(message_lines(&mut harness), [lost.to_string()]);
+        // Nor does a run that was refused before anything was sent.
+        let (mut harness, _tab) = writing(Look::standard(), Driver::Postgres, CHANGES);
+        let refused = Error::Refused {
+            line: 1,
+            what: "COMMIT".into(),
+            mode: tabletist_db::ScriptMode::Write,
+        };
+        harness.answer_sql(Err(refused.clone()), None);
+        assert_eq!(message_lines(&mut harness), [refused.to_string()]);
+    }
+
     /// The request of the newest run sent to the backend.
     fn run_request(harness: &Harness) -> RequestId {
         let sent = harness.app.backend.sent.iter().rev();
````

- [ ] **Step 2: Run the tests to see them fail**

```bash
~/.cargo/bin/cargo test --locked -p tabletist --lib -- sql_results::tests sql_editor::tests messages_open_for_every_end
```

Expected: FAIL, 41 passed and 16 failed. These compile, and fail on what is on screen: `messages_open_for_every_end_of_a_read_write_run_that_needs_reading`, `the_footer_says_how_a_read_write_run_ended`, and in `sql_results::tests` the fourteen named `a_committed_run_says_so_after_what_each_statement_changed`, `a_committed_statement_that_counts_no_rows_says_it_ran`, `a_run_that_failed_says_it_was_rolled_back_and_that_each_change_is_gone`, `a_cancelled_or_timed_out_run_says_it_was_rolled_back`, `a_run_mysql_committed_part_of_says_which_lines_are_written`, `one_committed_statement_is_one_line_that_is_written`, `a_commit_that_failed_says_nothing_was_written_and_why`, `a_failed_commit_after_mysql_committed_part_says_what_is_written`, `what_mysql_could_not_roll_back_is_said_and_nothing_is_said_to_be_gone`, `a_run_whose_session_could_not_be_put_back_keeps_its_end_and_says_it_was_closed`, `a_statement_that_returned_rows_is_rolled_back_only_where_it_looks_like_a_write`, `a_statements_warnings_are_counted_on_its_line`, `a_connection_lost_during_a_read_write_run_says_it_may_be_written`, `a_run_mysql_committed_under_a_late_stop_reads_as_written`. `a_read_only_run_reads_as_it_always_did` and `a_run_of_reads_in_a_read_write_tab_is_said_to_be_read_only` pass already, and must still pass after the code.

- [ ] **Step 3: Write the code**

````diff
diff --git a/src/app.rs b/src/app.rs
index 7ad981c..96edea7 100644
--- a/src/app.rs
+++ b/src/app.rs
@@ -3833,8 +3833,11 @@ fn script_mode(
 /// when the run failed, as a whole or in a statement; when its timeout
 /// stopped it (nobody asked for that, so the reason must show); and when
 /// it was cancelled without a statement returning rows, which leaves
-/// Results nothing to show. A run the user stopped after rows came back
-/// stays on Results.
+/// Results nothing to show. And when a run sent to write ended as neither
+/// of the two a user expects, committed or rolled back whole: part of it
+/// is written, its commit failed, the database could not undo all of it,
+/// or its session had to be closed. A run the user stopped after rows came
+/// back stays on Results.
 fn opens_messages(
     result: &Result<tabletist_db::ScriptOutcome, Error>,
     cancel: Option<CancelReason>,
@@ -3846,7 +3849,13 @@ fn opens_messages(
     let any = |wanted: fn(&StatementOutcome) -> bool| {
         outcome.results.iter().any(|result| wanted(&result.outcome))
     };
-    any(|outcome| matches!(outcome, StatementOutcome::Error { .. }))
+    // A run sent to write that left something to read about its end.
+    let end = matches!(
+        outcome.end,
+        tabletist_db::ScriptEnd::Partly { .. } | tabletist_db::ScriptEnd::CommitFailed { .. }
+    ) || outcome.rollback_warning.is_some()
+        || outcome.broken.is_some();
+    end || any(|outcome| matches!(outcome, StatementOutcome::Error { .. }))
         || matches!(cancel, Some(CancelReason::Timeout(_)))
         || (outcome.was_cancelled()
             && !any(|outcome| matches!(outcome, StatementOutcome::Rows { .. })))
diff --git a/src/model.rs b/src/model.rs
index 8bdd40f..a436fe6 100644
--- a/src/model.rs
+++ b/src/model.rs
@@ -2391,6 +2391,9 @@ pub struct SqlTab {
     /// The text the last run that finished started with. Its error mark
     /// names a line of that text, so it holds only while the text is that.
     ran_text: Option<TextPrint>,
+    /// The transaction that run was sent in. A run that failed as a whole
+    /// leaves no `SqlRun` to say it.
+    ran_mode: tabletist_db::ScriptMode,
 }
 
 // Hand-written so the SQL text never reaches logs or panic messages (a
@@ -2439,6 +2442,7 @@ impl SqlTab {
             completion_wanted: None,
             format: false,
             ran_text: None,
+            ran_mode: tabletist_db::ScriptMode::ReadOnly,
         }
     }
 
@@ -2486,6 +2490,7 @@ impl SqlTab {
         }
         let in_flight = self.in_flight.take();
         self.ran_text = in_flight.as_ref().map(|run| run.text);
+        self.ran_mode = in_flight.as_ref().map(|run| run.mode).unwrap_or_default();
         if result.is_err() {
             self.run.value = None;
             self.run.loaded = None;
@@ -2531,6 +2536,18 @@ impl SqlTab {
         self.run.value.as_ref()
     }
 
+    /// Whether the last run was sent to write and lost its session before
+    /// its end was known: some or all of it may be written, and nothing
+    /// can say which.
+    pub fn lost_writing(&self) -> bool {
+        self.ran_mode == tabletist_db::ScriptMode::Write
+            && self
+                .run
+                .error
+                .as_ref()
+                .is_some_and(Error::is_connection_lost)
+    }
+
     /// What the Results pane shows: the last statement of the last run
     /// that returned rows, with its index in the run.
     pub fn shown(&self) -> Option<(usize, &tabletist_db::StatementResult)> {
diff --git a/src/ui/sql_editor.rs b/src/ui/sql_editor.rs
index c414036..2b0e8ba 100644
--- a/src/ui/sql_editor.rs
+++ b/src/ui/sql_editor.rs
@@ -840,21 +840,77 @@ fn ran(sql: &SqlTab) -> bool {
             .is_some_and(|run| !run.outcome.results.is_empty())
 }
 
-/// "5 rows · 14 ms" for the result that shows.
+/// "5 rows · 14 ms" for the result that shows. With none to show, in a run
+/// sent to write, what its last statement changed: "12 rows affected ·
+/// 14 ms".
 fn run_summary(sql: &SqlTab, locale: Locale) -> Option<String> {
+    use tabletist_db::StatementOutcome;
     if !ran(sql) {
         return None;
     }
-    let (_, result) = sql.shown()?;
-    let tabletist_db::StatementOutcome::Rows { rows, .. } = &result.outcome else {
-        return None;
+    let counted = |count: u64, one: &'static str, many: &'static str, took| {
+        format!(
+            "{} {} · {}",
+            format::group_digits(count),
+            gettext(locale, if count == 1 { one } else { many }),
+            format::elapsed(took)
+        )
+    };
+    if let Some((_, result)) = sql.shown() {
+        let StatementOutcome::Rows { rows, .. } = &result.outcome else {
+            return None;
+        };
+        return Some(counted(rows.len() as u64, "row", "rows", result.elapsed));
+    }
+    let run = sql.last_run()?;
+    let last = run.outcome.results.last()?;
+    match &last.outcome {
+        StatementOutcome::Done {
+            affected: Some(count),
+            ..
+        } if run.mode == tabletist_db::ScriptMode::Write => Some(counted(
+            *count,
+            "row affected",
+            "rows affected",
+            last.elapsed,
+        )),
+        _ => None,
+    }
+}
+
+/// What became of the last run's transaction: "rolled back" for every
+/// read-only run, and for a run sent to write how it ended. Where the
+/// database could not undo everything, that is what is said, whatever the
+/// end; and a commit that failed after the database had committed part by
+/// itself reads as what it left, partly committed.
+fn end_words(run: &crate::model::SqlRun) -> &'static str {
+    use tabletist_db::ScriptEnd;
+    if run.mode == tabletist_db::ScriptMode::ReadOnly {
+        return "rolled back";
+    }
+    if run.outcome.rollback_warning.is_some() {
+        return "not fully rolled back";
+    }
+    match &run.outcome.end {
+        ScriptEnd::Committed => "committed",
+        ScriptEnd::RolledBack | ScriptEnd::Partly { committed: 0 } => "rolled back",
+        ScriptEnd::CommitFailed { committed: 0, .. } => "commit failed",
+        ScriptEnd::Partly { .. } | ScriptEnd::CommitFailed { .. } => "partly committed",
+    }
+}
+
+/// "Read-only transaction · rolled back", "Read-write transaction ·
+/// committed": the footer's note of the last run.
+fn end_note(sql: &SqlTab, locale: Locale) -> Option<String> {
+    let run = sql.last_run().filter(|_| ran(sql))?;
+    let transaction = match run.mode {
+        tabletist_db::ScriptMode::ReadOnly => "Read-only transaction",
+        tabletist_db::ScriptMode::Write => "Read-write transaction",
     };
-    let noun = if rows.len() == 1 { "row" } else { "rows" };
     Some(format!(
-        "{} {} · {}",
-        format::group_digits(rows.len() as u64),
-        gettext(locale, noun),
-        format::elapsed(result.elapsed)
+        "{} · {}",
+        gettext(locale, transaction),
+        gettext(locale, end_words(run))
     ))
 }
 
@@ -875,8 +931,7 @@ fn footer(app: &App, ui: &mut Ui, tab: ConnTabId, id: TabId) {
         gettext(locale, "Col")
     );
     let summary = run_summary(sql, locale);
-    let note =
-        ran(sql).then(|| gettext(locale, "Read-only transaction · rolled back").into_owned());
+    let note = end_note(sql, locale);
     let version = workspace.server_version.value.clone();
     egui::Panel::bottom(Id::new(("sql-footer", tab.0, id.0)))
         .exact_size(33.0)
@@ -936,15 +991,16 @@ fn footer(app: &App, ui: &mut Ui, tab: ConnTabId, id: TabId) {
 }
 
 /// The right end of the terminal's status line on an editor: "ln 12:21 ·
-/// 5 rows · 14 ms · rolled back".
+/// 5 rows · 14 ms · rolled back", or after a run sent to write "ln 3:1 ·
+/// 12 rows affected · 14 ms · committed".
 pub fn status_summary(app: &App, tab: ConnTabId) -> Option<String> {
     let locale = app.locale;
     let sql = app.workspace(tab)?.active_sql_tab()?;
     let (line, column) = sql.line_col();
     let mut parts = vec![format!("{} {line}:{column}", gettext(locale, "ln"))];
     parts.extend(run_summary(sql, locale));
-    if ran(sql) {
-        parts.push(gettext(locale, "rolled back").into_owned());
+    if let Some(run) = sql.last_run().filter(|_| ran(sql)) {
+        parts.push(gettext(locale, end_words(run)).into_owned());
     }
     Some(parts.join(" · "))
 }
diff --git a/src/ui/sql_results.rs b/src/ui/sql_results.rs
index e759479..d79b13f 100644
--- a/src/ui/sql_results.rs
+++ b/src/ui/sql_results.rs
@@ -7,7 +7,7 @@
 use std::time::Duration;
 
 use egui::{CornerRadius, Id, Rect, Sense, Ui, WidgetInfo, WidgetType, pos2, vec2};
-use tabletist_db::{Error, StatementOutcome, ValueKind};
+use tabletist_db::{Driver, Error, ScriptEnd, ScriptMode, StatementOutcome, ValueKind};
 
 use crate::app::App;
 use crate::backend::{CancelReason, RequestId};
@@ -58,6 +58,18 @@ impl Words {
             text.into_owned()
         }
     }
+
+    /// A database's name or a statement's keyword inside a sentence of
+    /// ours, as the look writes it: in lower case in the terminal's, whose
+    /// design has "postgres refused the update". Never for what a database
+    /// itself said, nor for a name the user gave.
+    fn own(self, name: &str) -> String {
+        if self.lower {
+            name.to_lowercase()
+        } else {
+            name.to_owned()
+        }
+    }
 }
 
 /// A text as screen readers are told it and as the look paints it. The
@@ -224,12 +236,22 @@ fn draw(app: &App, ui: &mut Ui, tab: ConnTabId, id: TabId, actions: &mut Vec<Act
             if format::refuses_writes(error, place.driver) {
                 blocked(&mut body, rest, error, place.writable, &env);
             } else {
-                note(&body, rest, &whole(error), palette.danger, &env);
+                let said = match may_be_written(sql, error) {
+                    Some(ours) => {
+                        let error = error_text(error);
+                        let error = error.trim_end_matches('.');
+                        env.said(|words| format!("{error}. {}", words.say(ours)))
+                    }
+                    None => whole(error),
+                };
+                note(&body, rest, &said, palette.danger, &env);
             }
         }
         (State::Failed(error), ResultPane::Messages) => {
+            let unknown = may_be_written(sql, error).map(|ours| Line::Ours(ours, Tone::Failed));
             let lines: Vec<Line<'_>> = std::iter::once(Line::Whole(error))
                 .chain(more(error))
+                .chain(unknown)
                 .collect();
             messages(&mut body, &lines, None, &place, &env);
         }
@@ -280,12 +302,15 @@ fn head(sql: &SqlTab, state: &State<'_>, env: &Env<'_>) -> Head {
         running,
         errors: match state {
             State::Failed(_) => 1,
-            State::Ran(run) => run
-                .outcome
-                .results
-                .iter()
-                .filter(|result| matches!(result.outcome, StatementOutcome::Error { .. }))
-                .count(),
+            // A commit that failed is an error of the run as a statement's
+            // is.
+            State::Ran(run) => {
+                let failed = |result: &&tabletist_db::StatementResult| {
+                    matches!(result.outcome, StatementOutcome::Error { .. })
+                };
+                let commit = matches!(run.outcome.end, ScriptEnd::CommitFailed { .. });
+                run.outcome.results.iter().filter(failed).count() + usize::from(commit)
+            }
             State::Idle | State::Waiting => 0,
         },
     }
@@ -689,6 +714,16 @@ fn rows_text(count: u64, words: Words) -> String {
     format!("{} {}", format::group_digits(count), words.say(noun))
 }
 
+/// "12 rows affected", "1 row affected".
+fn affected_text(count: u64, words: Words) -> String {
+    let noun = if count == 1 {
+        "row affected"
+    } else {
+        "rows affected"
+    };
+    format!("{} {}", format::group_digits(count), words.say(noun))
+}
+
 /// The statement that reads as cancelled though it has no result: the
 /// first one, when the run was stopped before it began. A run stopped
 /// later has a `Cancelled` result of its own (every driver gives the
@@ -711,6 +746,9 @@ enum Tone {
     /// A statement that did not run.
     Muted,
     Cancelled,
+    /// What a run left that it was not meant to: part of it written, or
+    /// not all of it rolled back.
+    Warned,
     Failed,
 }
 
@@ -719,7 +757,7 @@ impl Tone {
         match self {
             Self::Plain => palette.text,
             Self::Muted => palette.dim,
-            Self::Cancelled => palette.warning,
+            Self::Cancelled | Self::Warned => palette.warning,
             Self::Failed => palette.danger,
         }
     }
@@ -748,9 +786,64 @@ impl Message {
     }
 }
 
+/// Whether statement `index` did work that the end of `run` undid, so
+/// that its count must not read as a change that stayed. Only in a run
+/// sent to write, and never where the database could not roll everything
+/// back: then no line says that its work is gone.
+fn undone(run: &SqlRun, index: usize, driver: Driver) -> bool {
+    use tabletist_db::sql::{StatementKind, kind};
+    if run.mode != ScriptMode::Write || run.outcome.rollback_warning.is_some() {
+        return false;
+    }
+    // The statements before this one are written.
+    let kept = match &run.outcome.end {
+        ScriptEnd::Committed => return false,
+        ScriptEnd::RolledBack => 0,
+        ScriptEnd::Partly { committed } | ScriptEnd::CommitFailed { committed, .. } => *committed,
+    };
+    if index < kept {
+        return false;
+    }
+    match run.outcome.results.get(index).map(|result| &result.outcome) {
+        Some(StatementOutcome::Done { .. }) => true,
+        // A statement that returned rows may have changed some (RETURNING,
+        // a function that writes): told by what it looks like.
+        Some(StatementOutcome::Rows { .. }) => run.statements.get(index).is_some_and(|statement| {
+            kind(driver.dialect(), &statement.text) == StatementKind::Write
+        }),
+        _ => false,
+    }
+}
+
+/// What the line of a statement that ran ends with in a run sent to write:
+/// how many warnings the database raised (MySQL counts them), and that its
+/// work was rolled back, where it was. Nothing in a read-only run, which
+/// reads as it always did.
+fn after(run: &SqlRun, index: usize, driver: Driver, words: Words) -> String {
+    if run.mode != ScriptMode::Write {
+        return String::new();
+    }
+    let mut text = String::new();
+    if let Some(StatementOutcome::Done { warnings, .. }) =
+        run.outcome.results.get(index).map(|result| &result.outcome)
+        && *warnings > 0
+    {
+        let noun = if *warnings == 1 {
+            "warning"
+        } else {
+            "warnings"
+        };
+        text.push_str(&format!(" · {warnings} {}", words.say(noun)));
+    }
+    if undone(run, index, driver) {
+        text.push_str(&format!(" · {}", words.say("rolled back")));
+    }
+    text
+}
+
 /// What statement `index` of `run` did, after its line (the error's own
 /// line and column when the database gave a position).
-fn statement_message(run: &SqlRun, index: usize, words: Words) -> Message {
+fn statement_message(run: &SqlRun, index: usize, driver: Driver, words: Words) -> Message {
     let Some(statement) = run.statements.get(index) else {
         return Message::new(String::new(), String::new(), Tone::Plain);
     };
@@ -763,6 +856,7 @@ fn statement_message(run: &SqlRun, index: usize, words: Words) -> Message {
         };
     };
     let time = format::elapsed(result.elapsed);
+    let after = after(run, index, driver, words);
     match &result.outcome {
         StatementOutcome::Rows {
             rows, truncated, ..
@@ -773,23 +867,17 @@ fn statement_message(run: &SqlRun, index: usize, words: Words) -> Message {
                 String::new()
             };
             let rows = rows_text(rows.len() as u64, words);
-            Message::new(at, format!("{rows}{cut} · {time}"), Tone::Plain)
+            Message::new(at, format!("{rows}{cut} · {time}{after}"), Tone::Plain)
         }
         StatementOutcome::Done {
             affected: Some(count),
             ..
         } => {
-            let noun = if *count == 1 {
-                "row affected"
-            } else {
-                "rows affected"
-            };
-            let count = format::group_digits(*count);
-            let text = format!("{count} {} · {time}", words.say(noun));
+            let text = format!("{} · {time}{after}", affected_text(*count, words));
             Message::new(at, text, Tone::Plain)
         }
         StatementOutcome::Done { affected: None, .. } => {
-            let text = format!("{} · {time}", words.say("Statement ran"));
+            let text = format!("{} · {time}{after}", words.say("Statement ran"));
             Message::new(at, text, Tone::Plain)
         }
         StatementOutcome::Error { error, position } => {
@@ -816,16 +904,28 @@ enum Line<'a> {
     Statement(usize),
     /// More of the error above it: its code, detail or hint.
     More(&'static str, &'a str),
-    /// A run that failed as a whole, in the error's own words.
+    /// An error that is no statement's, in its own words: a run that
+    /// failed as a whole, or a commit that failed.
     Whole(&'a Error),
+    /// The cancel's own words, where no statement's line says them: the
+    /// stop came with every statement done.
+    Stopped,
+    /// How the transaction of a run sent to write ended.
+    End,
+    /// What the database said it could not roll back.
+    Kept(&'a str),
+    /// A sentence of ours about the run as a whole.
+    Ours(&'static str, Tone),
 }
 
 impl Line<'_> {
-    /// Whether the line holds what a database said, which can be long:
+    /// Whether the line holds what a database said, which can be long, or
+    /// says how a run sent to write ended, which is a sentence or two:
     /// such a line wraps. The others are ours, and short.
     fn wraps(&self, run: Option<&SqlRun>) -> bool {
         match self {
-            Self::More(..) | Self::Whole(_) => true,
+            Self::Stopped => false,
+            Self::More(..) | Self::Whole(_) | Self::End | Self::Kept(_) | Self::Ours(..) => true,
             Self::Statement(index) => run
                 .and_then(|run| run.outcome.results.get(*index))
                 .is_some_and(|result| matches!(result.outcome, StatementOutcome::Error { .. })),
@@ -845,7 +945,8 @@ fn more(error: &Error) -> impl Iterator<Item = Line<'_>> {
     parts.into_iter().flatten()
 }
 
-/// A line for every statement of `run`, in the script's order.
+/// A line for every statement of `run`, in the script's order, and after
+/// them how a run sent to write ended.
 fn lines(run: &SqlRun) -> Vec<Line<'_>> {
     let mut lines = Vec::with_capacity(run.statements.len());
     for index in 0..run.statements.len() {
@@ -856,16 +957,174 @@ fn lines(run: &SqlRun) -> Vec<Line<'_>> {
             lines.extend(more(error));
         }
     }
+    lines.extend(end_lines(run));
     lines
 }
 
+/// What the Messages say once a session was closed because a run sent to
+/// write could not put it back.
+const CLOSED: &str = "The session could not be put back and was closed.";
+
+/// The lines after the statements' in a run sent to write: how its
+/// transaction ended, what the database said of that, and that its session
+/// was closed, where it was. A read-only run has none.
+fn end_lines(run: &SqlRun) -> Vec<Line<'_>> {
+    if run.mode != ScriptMode::Write {
+        return Vec::new();
+    }
+    let outcome = &run.outcome;
+    let mut lines = Vec::new();
+    let said = stopped_at(run).is_some()
+        || outcome
+            .results
+            .iter()
+            .any(|result| result.outcome == StatementOutcome::Cancelled);
+    if outcome.stopped && !said {
+        lines.push(Line::Stopped);
+    }
+    lines.push(Line::End);
+    if let ScriptEnd::CommitFailed { error, .. } = &outcome.end {
+        lines.push(Line::Whole(error));
+        lines.extend(more(error));
+    }
+    if let Some(kept) = &outcome.rollback_warning {
+        lines.push(Line::Kept(kept));
+    }
+    if outcome.broken.is_some() {
+        lines.push(Line::Ours(CLOSED, Tone::Failed));
+    }
+    lines
+}
+
+/// "Lines 1 to 4 are written: MySQL commits CREATE, ALTER, DROP and
+/// similar statements as they run.": the first `committed` statements of
+/// `run`, by the lines they start on.
+fn written(run: &SqlRun, committed: usize, driver: Driver, words: Words) -> String {
+    let first = run.statements.first().map_or(1, |first| first.first_line);
+    let last = committed
+        .checked_sub(1)
+        .and_then(|last| run.statements.get(last))
+        .map_or(first, |last| last.first_line);
+    let lines = if first == last {
+        format!("{} {first} {}", words.say("Line"), words.say("is written:"))
+    } else {
+        format!(
+            "{} {first} {} {last} {}",
+            words.say("Lines"),
+            words.say("to"),
+            words.say("are written:")
+        )
+    };
+    format!(
+        "{lines} {} {} {} {}",
+        words.own(driver.label()),
+        words.say("commits"),
+        words.own("CREATE, ALTER, DROP"),
+        words.say("and similar statements as they run.")
+    )
+}
+
+/// How the transaction of `run`, a run sent to write, ended. Where the
+/// database could not roll everything back, nothing here says that the
+/// rest is gone or that nothing was written.
+fn end_message(run: &SqlRun, driver: Driver, words: Words) -> Message {
+    let outcome = &run.outcome;
+    let kept = outcome.rollback_warning.is_some();
+    let not_undone = || {
+        format!(
+            "{} {}",
+            words.own(driver.label()),
+            words.say("could not roll back every change.")
+        )
+    };
+    let rest = || {
+        if kept {
+            not_undone()
+        } else {
+            words.say("The rest was rolled back.")
+        }
+    };
+    let (text, tone) = match &outcome.end {
+        ScriptEnd::Committed => {
+            let count = outcome.results.len();
+            let noun = if count == 1 {
+                "statement"
+            } else {
+                "statements"
+            };
+            let time: Duration = outcome.results.iter().map(|result| result.elapsed).sum();
+            let text = format!(
+                "{} · {count} {} · {}",
+                words.say("Committed"),
+                words.say(noun),
+                format::elapsed(time)
+            );
+            (text, Tone::Plain)
+        }
+        // No driver says `Partly` of nothing, and none of these arms
+        // would word it.
+        ScriptEnd::RolledBack | ScriptEnd::Partly { committed: 0 } if kept => {
+            (not_undone(), Tone::Warned)
+        }
+        ScriptEnd::RolledBack | ScriptEnd::Partly { committed: 0 } => {
+            (words.say("Rolled back. Nothing was written."), Tone::Plain)
+        }
+        ScriptEnd::Partly { committed } => {
+            let written = written(run, *committed, driver, words);
+            (format!("{written} {}", rest()), Tone::Warned)
+        }
+        // The rollback after the commit could not undo everything either.
+        ScriptEnd::CommitFailed { committed: 0, .. } if kept => (
+            format!("{} {}", words.say("The commit failed."), not_undone()),
+            Tone::Failed,
+        ),
+        ScriptEnd::CommitFailed { committed: 0, .. } => (
+            words.say("The commit failed. Nothing was written."),
+            Tone::Failed,
+        ),
+        ScriptEnd::CommitFailed { committed, .. } => {
+            let written = written(run, *committed, driver, words);
+            let failed = words.say("The commit failed.");
+            (format!("{failed} {written} {}", rest()), Tone::Failed)
+        }
+    };
+    Message::new(String::new(), text, tone)
+}
+
+/// What is said under a run's error when the run was sent to write and
+/// lost its session before its end was known.
+fn may_be_written(sql: &SqlTab, error: &Error) -> Option<&'static str> {
+    if !sql.lost_writing() {
+        return None;
+    }
+    Some(match error {
+        Error::ConnectionLost(_) => {
+            "The connection was lost during a read-write run. Some or all of it may be written."
+        }
+        // The session was closed for what the script did: the error says
+        // so itself.
+        _ => "Some or all of the run may be written.",
+    })
+}
+
 /// What `line` says.
-fn message(line: Line<'_>, run: Option<&SqlRun>, words: Words) -> Message {
+fn message(line: Line<'_>, run: Option<&SqlRun>, driver: Driver, words: Words) -> Message {
+    let ours = |text: String, tone: Tone| Message::new(String::new(), text, tone);
     match line {
         Line::Statement(index) => match run {
-            Some(run) => statement_message(run, index, words),
+            Some(run) => statement_message(run, index, driver, words),
             None => Message::new(String::new(), String::new(), Tone::Plain),
         },
+        Line::Stopped => ours(
+            cancel_text(run.and_then(|run| run.cancel), words),
+            Tone::Cancelled,
+        ),
+        Line::End => match run {
+            Some(run) => end_message(run, driver, words),
+            None => Message::new(String::new(), String::new(), Tone::Plain),
+        },
+        Line::Kept(text) => ours(format::capped(text).into_owned(), Tone::Warned),
+        Line::Ours(text, tone) => ours(words.say(text), tone),
         Line::More(label, text) => Message::new(
             format!("{}:", words.say(label)),
             format::capped(text).into_owned(),
@@ -907,8 +1166,9 @@ fn messages(
             0.0
         }
     };
+    let driver = place.driver;
     let lay = |line: Line<'_>| -> LaidMessage {
-        let message = message(line, run, painted);
+        let message = message(line, run, driver, painted);
         LaidMessage::new(
             &ctx,
             (&message.place, palette.dim),
@@ -945,7 +1205,7 @@ fn messages(
                     locale,
                     lower: false,
                 };
-                let name = message(line, run, named).line();
+                let name = message(line, run, driver, named).line();
                 response.widget_info(|| WidgetInfo::labeled(WidgetType::Label, true, &name));
                 if ui.is_rect_visible(rect) {
                     let at = pos2(rect.left() + pad + indent(line), rect.top() + above);
@@ -1012,6 +1272,27 @@ fn blocked(ui: &mut Ui, rect: Rect, error: &Error, writable: bool, env: &Env<'_>
         });
 }
 
+/// What Results says of a run none of whose statements returned rows:
+/// "Statement ran · 12 rows affected" where the last statement of a run
+/// sent to write counted the rows it changed, and "Statement ran · no rows
+/// returned" everywhere else.
+fn ran_text(run: &SqlRun, words: Words) -> String {
+    let last = run.outcome.results.last().map(|result| &result.outcome);
+    match last {
+        Some(StatementOutcome::Done {
+            affected: Some(count),
+            ..
+        }) if run.mode == ScriptMode::Write => {
+            format!(
+                "{} · {}",
+                words.say("Statement ran"),
+                affected_text(*count, words)
+            )
+        }
+        _ => words.say("Statement ran · no rows returned"),
+    }
+}
+
 /// The Results pane of a run that ran: the rows of its last statement
 /// that returned some, in the grid a table uses. With none, whether the
 /// run was stopped, where it failed, or that it ran.
@@ -1036,7 +1317,10 @@ fn results(ui: &mut Ui, run: &SqlRun, place: &Place<'_>, env: &Env<'_>, actions:
             .results
             .iter()
             .position(|result| matches!(result.outcome, StatementOutcome::Error { .. }));
-        if run.outcome.was_cancelled() {
+        // Unless it is written all the same: MySQL can commit by itself
+        // under a stop that came too late, and then the run did run.
+        let written = run.mode == ScriptMode::Write && run.outcome.end == ScriptEnd::Committed;
+        if run.outcome.was_cancelled() && !written {
             let said = env.said(|words| cancel_text(run.cancel, words));
             note(ui, rect, &said, palette.warning, env);
         } else if let Some(index) = failed {
@@ -1055,12 +1339,17 @@ fn results(ui: &mut Ui, run: &SqlRun, place: &Place<'_>, env: &Env<'_>, actions:
                 locale,
                 lower: look.terminal,
             };
-            let name = statement_message(run, index, named).line();
-            let message = statement_message(run, index, painted);
+            let name = statement_message(run, index, place.driver, named).line();
+            let message = statement_message(run, index, place.driver, painted);
             let parts = (message.place.as_str(), message.text.as_str());
             note_at(ui, rect, parts, &name, palette.danger, env);
+        } else if let ScriptEnd::CommitFailed { .. } = run.outcome.end {
+            // Every statement ran and none of it is kept: said as the
+            // Messages say it, never as a statement that ran.
+            let said = env.said(|words| end_message(run, place.driver, words).text);
+            note(ui, rect, &said, palette.danger, env);
         } else {
-            let said = env.said(|words| words.say("Statement ran · no rows returned"));
+            let said = env.said(|words| ran_text(run, words));
             note(ui, rect, &said, palette.secondary, env);
         }
         return;
````

- [ ] **Step 4: Run the tests to see them pass**

```bash
~/.cargo/bin/cargo test --locked -p tabletist --lib -- sql_results::tests sql_editor::tests messages_open_for_every_end
```

Expected: PASS, 57 passed and 0 failed.

- [ ] **Step 5: Run the four checks**

All four pass. Fix what does not before going on; do not weaken a lint to get there.

- [ ] **Step 6: Commit**

```bash
git add src/ui/sql_results.rs src/ui/sql_editor.rs src/model.rs src/app.rs
git commit -S -m "Say how a read-write run ended, in Messages, Results and the footer" -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 5: The badge's menu and Mod+Shift+M

**Files:**
- Modify: `src/ui/sql_editor.rs` (`Bar`, `toolbar`, `badge`, `badge_width`, `badge_tip`, `mac_toolbar`, `terminal_toolbar`, and its tests module)
- Modify: `src/ui/widgets.rs` (`MenuChoice::disabled`, `popup_menu`)
- Modify: `src/ui/sidebar.rs` (one `MenuChoice` gains the field)
- Modify: `src/ui/keys.rs` (`SHORTCUTS`, `handle`)

This is the task that lets a tab write: before it nothing on screen sets the mode.

- On a connection that takes writes the badge is a menu, built on `widgets::popup_menu` as Limit and Timeout are, in all three looks: a `ComboBox` named "Transaction" whose value is the mode's name. On macOS and Windows it keeps its pill and gains a chevron; on Omarchy the first part of `read-only transaction · limit 1000 · timeout 30s` opens it. In Read-write it reads in the warning colour (with the warning tint and a pencil on the pill).
- On a connection that opens read-only the badge is what it was, a label, and its tooltip adds "This connection opens read-only."
- On a writable production connection (this run only) the menu shows "Read-write transaction" disabled. `MenuChoice` gains `disabled: Option<String>` for that: the reason is the choice's tooltip, and the badge's tooltip says it too.
- `Mod+Shift+M` sends `Action::ToggleSqlMode` for the SQL editor on screen, also while typing, and is in the help as "Read-only or read-write runs in the SQL editor". The Omarchy artboard's `ctrl+w` closes the tab, which is why it is not that.

The toolbar gives way in the order it had: the badge stands where the read-only note stood.

- [ ] **Step 1: Write the failing tests**

Apply this block. It holds the task's tests, and applies on the tree as the task before left it.

````diff
diff --git a/src/ui/sql_editor.rs b/src/ui/sql_editor.rs
index 2b0e8ba..df12a90 100644
--- a/src/ui/sql_editor.rs
+++ b/src/ui/sql_editor.rs
@@ -1012,7 +1181,7 @@ mod tests {
 
     use super::*;
     use crate::model::RunMode;
-    use crate::testing::{Harness, done_outcome, node, write_outcome};
+    use crate::testing::{Harness, bounds, done_outcome, node, write_outcome};
 
     /// A SQL editor on a connection that takes writes, drawn in `look`,
     /// with `text` typed into it.
@@ -1044,6 +1213,208 @@ mod tests {
         (node.is_disabled(), node.description().map(str::to_owned))
     }
 
+    /// A SQL editor on a connection that opens read-only, drawn in `look`.
+    fn read_only_editor(look: Look) -> (Harness, ConnTabId, TabId) {
+        let mut harness = Harness::new();
+        harness.set_look(look);
+        let tab = harness.connect_fake();
+        harness.press(Key::T, Modifiers::COMMAND);
+        let id = harness.app.workspace(tab).unwrap().active_tab.unwrap();
+        (harness, tab, id)
+    }
+
+    fn mode_of(harness: &Harness, tab: ConnTabId, id: TabId) -> RunMode {
+        let workspace = harness.app.workspace(tab).unwrap();
+        workspace.run_mode(workspace.sql_tab(id).unwrap())
+    }
+
+    /// What the badge's menu is set to: none where the badge is no menu.
+    fn badge_value(harness: &mut Harness) -> Option<String> {
+        let tree = harness.settle();
+        let id = node(&tree, TRANSACTION, Role::ComboBox)?;
+        let (_, badge) = tree.nodes.iter().find(|(node, _)| *node == id)?;
+        badge.value().map(str::to_owned)
+    }
+
+    /// The colour the badge's words are painted in, as `look` writes them.
+    fn badge_color(harness: &Harness, look: &Look, mode: RunMode) -> Option<egui::Color32> {
+        harness.painted_color(&look.label(mode_name(mode)))
+    }
+
+    #[test]
+    fn the_badge_is_a_menu_that_switches_the_tabs_mode() {
+        for look in Look::ALL {
+            let (mut harness, tab, id) = editor(look, "SELECT 1");
+            assert_eq!(
+                badge_value(&mut harness).as_deref(),
+                Some("Read-only transaction"),
+                "{}",
+                look.name
+            );
+            let quiet = badge_color(&harness, &look, RunMode::ReadOnly);
+            harness.click(TRANSACTION);
+            harness.click("Read-write transaction");
+            assert_eq!(
+                mode_of(&harness, tab, id),
+                RunMode::ReadWrite,
+                "{}",
+                look.name
+            );
+            assert_eq!(
+                badge_value(&mut harness).as_deref(),
+                Some("Read-write transaction"),
+                "{}",
+                look.name
+            );
+            // The menu closed on the pick, and the badge reads in the
+            // warning colour, which the read-only one did not.
+            assert!(!harness.has("Read-only transaction"), "{}", look.name);
+            let warning = Some(harness.app.palette.warning);
+            assert_eq!(badge_color(&harness, &look, RunMode::ReadWrite), warning);
+            assert_ne!(quiet, warning, "{}", look.name);
+            // Picking the mode in use changes nothing; the other one
+            // switches back.
+            harness.click(TRANSACTION);
+            harness.click("Read-write transaction");
+            assert_eq!(mode_of(&harness, tab, id), RunMode::ReadWrite);
+            harness.click(TRANSACTION);
+            harness.click("Read-only transaction");
+            assert_eq!(
+                mode_of(&harness, tab, id),
+                RunMode::ReadOnly,
+                "{}",
+                look.name
+            );
+        }
+    }
+
+    #[test]
+    fn mod_shift_m_switches_the_mode_of_the_editor_on_screen() {
+        let chord = Modifiers::COMMAND | Modifiers::SHIFT;
+        for look in Look::ALL {
+            let (mut harness, tab, id) = editor(look, "SELECT 1");
+            // With the keyboard in the editor, where it is while typing.
+            harness.press(Key::M, chord);
+            assert_eq!(
+                mode_of(&harness, tab, id),
+                RunMode::ReadWrite,
+                "{}",
+                look.name
+            );
+            // The editor did not take the key for a letter.
+            let text = &harness
+                .app
+                .workspace(tab)
+                .unwrap()
+                .sql_tab(id)
+                .unwrap()
+                .text;
+            assert_eq!(text, "SELECT 1", "{}", look.name);
+            harness.press(Key::M, chord);
+            assert_eq!(
+                mode_of(&harness, tab, id),
+                RunMode::ReadOnly,
+                "{}",
+                look.name
+            );
+        }
+        // The help lists it.
+        let listed = crate::ui::keys::SHORTCUTS.iter().any(|(keys, what, _)| {
+            *keys == "Mod+Shift+M" && *what == "Read-only or read-write runs in the SQL editor"
+        });
+        assert!(listed);
+    }
+
+    #[test]
+    fn on_a_read_only_connection_the_badge_is_a_note_and_the_key_does_nothing() {
+        for look in Look::ALL {
+            let (mut harness, tab, id) = read_only_editor(look);
+            // As it always was: a label, and no menu.
+            assert_eq!(badge_value(&mut harness), None, "{}", look.name);
+            let tree = harness.settle();
+            assert!(
+                node(&tree, "Read-only transaction", Role::Label).is_some(),
+                "{}",
+                look.name
+            );
+            harness.press(Key::M, Modifiers::COMMAND | Modifiers::SHIFT);
+            assert_eq!(
+                mode_of(&harness, tab, id),
+                RunMode::ReadOnly,
+                "{}",
+                look.name
+            );
+            // On hover it says why.
+            let at = bounds(&tree, "Read-only transaction", Role::Label).unwrap();
+            let shown = crate::ui::tests::hover(&mut harness, at.center());
+            let tip = "Every query runs in a read-only transaction that is rolled back. This \
+                       connection opens read-only.";
+            assert!(shown.iter().any(|label| label == tip), "{}", look.name);
+        }
+    }
+
+    #[test]
+    fn the_badge_says_on_hover_what_its_mode_does() {
+        for look in Look::ALL {
+            let (mut harness, tab, id) = editor(look, "SELECT 1");
+            for (mode, tip) in [
+                (
+                    RunMode::ReadOnly,
+                    "Runs are rolled back. Nothing is changed.",
+                ),
+                (
+                    RunMode::ReadWrite,
+                    "A run that changes data is committed when every statement succeeds.",
+                ),
+            ] {
+                set_mode(&mut harness, tab, id, mode);
+                let tree = harness.settle();
+                let at = bounds(&tree, TRANSACTION, Role::ComboBox).unwrap();
+                let shown = crate::ui::tests::hover(&mut harness, at.center());
+                assert!(
+                    shown.iter().any(|label| label == tip),
+                    "{tip} in {}",
+                    look.name
+                );
+            }
+        }
+    }
+
+    #[test]
+    fn on_production_the_read_write_choice_is_shown_and_cannot_be_picked() {
+        for look in Look::ALL {
+            let (mut harness, tab, id) = editor(look, "SELECT 1");
+            harness.app.workspace_mut(tab).unwrap().environment =
+                crate::env::Environment::Production;
+            harness.click(TRANSACTION);
+            let tree = harness.settle();
+            let choice = node(&tree, "Read-write transaction", Role::Button).expect("the choice");
+            let (_, choice) = tree.nodes.iter().find(|(node, _)| *node == choice).unwrap();
+            assert!(choice.is_disabled(), "{}", look.name);
+            harness.click("Read-write transaction");
+            assert_eq!(
+                mode_of(&harness, tab, id),
+                RunMode::ReadOnly,
+                "{}",
+                look.name
+            );
+            harness.press(Key::Escape, Modifiers::NONE);
+            harness.press(Key::M, Modifiers::COMMAND | Modifiers::SHIFT);
+            assert_eq!(
+                mode_of(&harness, tab, id),
+                RunMode::ReadOnly,
+                "{}",
+                look.name
+            );
+            // The badge says why on hover.
+            let tree = harness.settle();
+            let at = bounds(&tree, TRANSACTION, Role::ComboBox).unwrap();
+            let shown = crate::ui::tests::hover(&mut harness, at.center());
+            let tip = format!("Runs are rolled back. Nothing is changed. {UNCONFIRMED}");
+            assert!(shown.contains(&tip), "{}", look.name);
+        }
+    }
+
     #[test]
     fn run_and_run_all_wait_for_a_read_write_run_in_flight() {
         for look in Look::ALL {
````

- [ ] **Step 2: Run the tests to see them fail**

```bash
~/.cargo/bin/cargo test --locked -p tabletist --lib -- sql_editor::tests
```

Expected: does not compile. `cannot find value TRANSACTION`, `cannot find value UNCONFIRMED`, `cannot find function mode_name`.

- [ ] **Step 3: Write the code**

````diff
diff --git a/src/ui/keys.rs b/src/ui/keys.rs
index 760ebb9..8c351ad 100644
--- a/src/ui/keys.rs
+++ b/src/ui/keys.rs
@@ -56,6 +56,11 @@ pub const SHORTCUTS: &[(&str, &str, Holds)] = &[
         ALL,
     ),
     ("Mod+Shift+F", "Format SQL", ALL),
+    (
+        "Mod+Shift+M",
+        "Read-only or read-write runs in the SQL editor",
+        ALL,
+    ),
     ("Ctrl+Space, Mod+I", "Complete in the SQL editor", ALL),
     ("Mod+W", "Close tab", ALL),
     ("Mod+Shift+[ / ]", "Previous / next tab", ALL),
@@ -289,6 +294,13 @@ pub fn handle(app: &mut App, ctx: &egui::Context) {
         if format && let Some((tab, sql_tab)) = sql {
             actions.push(Action::FormatSql { tab, sql_tab });
         }
+        // The other mode of the editor's runs. Not Mod+W, as its design
+        // has it: that closes the tab.
+        if let Some((tab, sql_tab)) = sql
+            && consume_press(input, Modifiers::COMMAND | Modifiers::SHIFT, Key::M)
+        {
+            actions.push(Action::ToggleSqlMode { tab, sql_tab });
+        }
         // A fresh press only: an Esc held to close a dialog over the tab
         // repeats after the dialog is gone.
         if give_up && consume_press(input, Modifiers::NONE, Key::Escape) {
diff --git a/src/ui/sidebar.rs b/src/ui/sidebar.rs
index cc3f9de..6a068f7 100644
--- a/src/ui/sidebar.rs
+++ b/src/ui/sidebar.rs
@@ -583,6 +583,7 @@ fn schema_header(
                     text: display_safe(other).into_owned(),
                     name: None,
                     selected: Some(other.as_str()) == shown,
+                    disabled: None,
                 })
                 .collect()
         });
diff --git a/src/ui/sql_editor.rs b/src/ui/sql_editor.rs
index 2b0e8ba..df12a90 100644
--- a/src/ui/sql_editor.rs
+++ b/src/ui/sql_editor.rs
@@ -9,11 +9,12 @@ use egui::{
 
 use crate::app::App;
 use crate::i18n::{Locale, gettext};
-use crate::model::{Action, ConnTabId, SqlTab, TabId};
+use crate::model::{Action, ConnTabId, NoWrites, RunMode, SqlTab, TabId};
 use crate::settings::Settings;
 use crate::theme::{Icon, Look, Palette};
 use crate::typography::{Text, TextRole};
 use crate::ui::format;
+use crate::ui::states;
 use crate::ui::widgets::{self, ButtonSpec, MenuChoice};
 
 /// Left and right padding of the toolbar.
@@ -282,18 +283,26 @@ struct Bar<'a> {
     timeout_label: &'a MenuLabel,
     /// The editor's read-write run is in flight: Run and Run all wait.
     writing: bool,
+    /// The transaction the editor's runs are in now.
+    mode: RunMode,
+    /// Whether the badge switches that, or why the editor cannot write.
+    writes: Result<(), NoWrites>,
     locale: Locale,
     look: &'a Look,
     palette: &'a Palette,
 }
 
-/// Run and Run all, the transaction every run is wrapped in, and the
-/// limit and timeout it runs with.
+/// Run and Run all, the transaction the editor's runs are in, and the
+/// limit and timeout they run with.
 fn toolbar(app: &mut App, ui: &mut Ui, tab: ConnTabId, id: TabId) {
     let (look, palette, locale) = (app.look, app.palette, app.locale);
-    let Some(sql) = app.workspace(tab).and_then(|w| w.sql_tab(id)) else {
+    let Some(workspace) = app.workspace(tab) else {
         return;
     };
+    let Some(sql) = workspace.sql_tab(id) else {
+        return;
+    };
+    let (mode, writes) = (workspace.run_mode(sql), workspace.sql_writes());
     let title = look.label(&format!("{} {}", gettext(locale, "Query"), sql.number));
     let limit = sql.limit;
     let secs = sql
@@ -320,6 +329,8 @@ fn toolbar(app: &mut App, ui: &mut Ui, tab: ConnTabId, id: TabId) {
         limit_label: &limit_label,
         timeout_label: &timeout_label,
         writing: sql.is_writing(),
+        mode,
+        writes,
         locale,
         look: &look,
         palette: &palette,
@@ -392,6 +403,7 @@ fn menus(
                     text: limit_text(*choice, false, look, locale),
                     name: Some(limit_name(*choice, locale)),
                     selected: *choice == bar.limit,
+                    disabled: None,
                 })
                 .collect()
         },
@@ -418,6 +430,7 @@ fn menus(
                     text: timeout_text(*choice, false, look, locale),
                     name: Some(timeout_name(*choice, locale)),
                     selected: *choice == bar.secs,
+                    disabled: None,
                 })
                 .collect()
         },
@@ -433,16 +446,180 @@ fn menus(
     }
 }
 
-/// Names the transaction note for screen readers and says, on hover, why
-/// it is there.
-fn explain_note(ui: &Ui, rect: Rect, locale: Locale) {
-    let response = ui.interact(rect, ui.id().with("transaction-note"), Sense::hover());
-    let name = gettext(locale, "Read-only transaction");
-    response.widget_info(|| WidgetInfo::labeled(WidgetType::Label, true, name.as_ref()));
-    let _ = response.on_hover_text(gettext(
+/// What the badge's menu is named for screen readers: its value is the
+/// mode.
+const TRANSACTION: &str = "Transaction";
+
+/// Why a tab of a writable connection to production cannot be switched to
+/// Read-write: its runs are to be confirmed first, with their statements
+/// on screen, and that question is not built yet.
+pub(super) const UNCONFIRMED: &str =
+    "Read-write runs on a production connection are not available yet.";
+
+/// A mode as the badge names it.
+fn mode_name(mode: RunMode) -> &'static str {
+    match mode {
+        RunMode::ReadOnly => "Read-only transaction",
+        RunMode::ReadWrite => "Read-write transaction",
+    }
+}
+
+/// What the badge says on hover: what the mode does, and on a connection
+/// whose editors cannot write, why not.
+fn badge_tip(bar: &Bar<'_>) -> String {
+    let say = |text: &'static str| gettext(bar.locale, text);
+    let does = match bar.mode {
+        RunMode::ReadOnly => say("Runs are rolled back. Nothing is changed."),
+        RunMode::ReadWrite => {
+            say("A run that changes data is committed when every statement succeeds.")
+        }
+    };
+    match bar.writes {
+        Ok(()) => does.into_owned(),
+        Err(NoWrites::ReadOnlyConnection) => format!(
+            "{}. {}",
+            say("Every query runs in a read-only transaction that is rolled back"),
+            say("This connection opens read-only.")
+        ),
+        Err(NoWrites::Unconfirmed) => format!("{does} {}", say(UNCONFIRMED)),
+    }
+}
+
+/// Whether the badge opens its menu. On a connection that opens read-only
+/// it is what it always was, a note.
+fn badge_switches(bar: &Bar<'_>) -> bool {
+    bar.writes != Err(NoWrites::ReadOnlyConnection)
+}
+
+/// The badge's width. macOS: 10 at its sides, an 11 pt mark, 6, the words
+/// and, where it is a menu, 6 and a 10 pt chevron. The terminal: the
+/// words.
+fn badge_width(ui: &Ui, bar: &Bar<'_>) -> f32 {
+    let look = bar.look;
+    let words = look.label(&gettext(bar.locale, mode_name(bar.mode)));
+    let words = menu_role(look).width(ui.ctx(), look.faces, &words);
+    if look.terminal {
+        return words;
+    }
+    let chevron = if badge_switches(bar) { 6.0 + 10.0 } else { 0.0 };
+    10.0 + 11.0 + 6.0 + words + chevron + 10.0
+}
+
+/// The badge in `rect`: the transaction the editor's runs are in, and on
+/// a connection that takes writes the menu that switches it. In
+/// Read-write it reads in the warning tone. Returns the mode picked from
+/// the menu this frame, when it is another than the one in use.
+fn badge(ui: &mut Ui, rect: Rect, bar: &Bar<'_>) -> Option<RunMode> {
+    let Bar {
         locale,
-        "Every query runs in a read-only transaction that is rolled back",
-    ));
+        look,
+        palette,
+        mode,
+        ..
+    } = *bar;
+    let name = gettext(locale, mode_name(mode));
+    let words = look.label(&name);
+    let switches = badge_switches(bar);
+    let sense = if switches {
+        Sense::click()
+    } else {
+        Sense::hover()
+    };
+    let response = ui.interact(rect, ui.id().with("transaction-note"), sense);
+    response.widget_info(|| {
+        if switches {
+            let menu = gettext(locale, TRANSACTION);
+            let mut info = WidgetInfo::labeled(WidgetType::ComboBox, true, menu.as_ref());
+            info.current_text_value = Some(name.to_string());
+            info
+        } else {
+            WidgetInfo::labeled(WidgetType::Label, true, name.as_ref())
+        }
+    });
+    let writing = mode == RunMode::ReadWrite;
+    let lit = switches && (response.hovered() || response.has_focus());
+    let center = rect.center().y;
+    let role = menu_role(look);
+    if look.terminal {
+        // Muted words that light up under the pointer, as the menus
+        // beside them.
+        let color = match (writing, lit) {
+            (true, _) => palette.warning,
+            (false, true) => palette.text,
+            (false, false) => palette.dim,
+        };
+        widgets::paint_text(
+            ui,
+            rect.left(),
+            center,
+            Text::one(look, role, &words, color),
+        );
+    } else {
+        let tone = states::Tone::Warning;
+        let (fill, ink) = match (writing, lit) {
+            (true, _) => (tone.fill(look, palette), palette.warning),
+            (false, true) => (palette.surface_hover, palette.secondary),
+            (false, false) => (palette.surface, palette.secondary),
+        };
+        let corner = CornerRadius::same(13);
+        ui.painter().rect_filled(rect, corner, fill);
+        if writing {
+            ui.painter().rect_stroke(
+                rect,
+                corner,
+                Stroke::new(widgets::hairline(ui), tone.line(look, palette)),
+                StrokeKind::Inside,
+            );
+        }
+        let mark = if writing { Icon::Pencil } else { Icon::Lock };
+        mark.image(ink, 11.0).paint_at(
+            ui,
+            Rect::from_center_size(pos2(rect.left() + 15.5, center), vec2(11.0, 11.0)),
+        );
+        widgets::paint_text(
+            ui,
+            rect.left() + 27.0,
+            center,
+            Text::one(look, role, &words, ink),
+        );
+        if switches {
+            Icon::ChevronDown.image(ink, 10.0).paint_at(
+                ui,
+                Rect::from_center_size(pos2(rect.right() - 15.0, center), vec2(10.0, 10.0)),
+            );
+        }
+    }
+    let response = response.on_hover_text(badge_tip(bar));
+    if !switches {
+        return None;
+    }
+    let radius = if look.terminal { 0 } else { 13 };
+    crate::ui::focus::hint(
+        ui,
+        &response,
+        rect,
+        crate::ui::focus::Ring::Outer { radius },
+    );
+    let unconfirmed = bar.writes == Err(NoWrites::Unconfirmed);
+    let modes = [RunMode::ReadOnly, RunMode::ReadWrite];
+    let picked = widgets::popup_menu(&response, rect.width(), look, || {
+        modes
+            .iter()
+            .map(|choice| {
+                let name = gettext(locale, mode_name(*choice));
+                let off = unconfirmed && *choice == RunMode::ReadWrite;
+                MenuChoice {
+                    text: look.label(&name),
+                    name: Some(name.into_owned()),
+                    selected: *choice == mode,
+                    disabled: off.then(|| gettext(locale, UNCONFIRMED).into_owned()),
+                }
+            })
+            .collect()
+    });
+    picked
+        .and_then(|index| modes.get(index).copied())
+        .filter(|picked| *picked != mode)
 }
 
 /// Run or Run all in `rect`, with what it does on hover. Neither can be
@@ -482,7 +659,7 @@ fn run_button(
 }
 
 /// macOS: Run and Run all, a divider and Format; at the right the
-/// transaction note, then the Limit and Timeout menus.
+/// transaction badge, then the Limit and Timeout menus.
 fn mac_toolbar(ui: &mut Ui, rect: Rect, bar: &Bar<'_>, actions: &mut Vec<Action>) {
     let Bar {
         locale,
@@ -520,10 +697,7 @@ fn mac_toolbar(ui: &mut Ui, rect: Rect, bar: &Bar<'_>, actions: &mut Vec<Action>
             [run, all, format]
         }
     };
-    // The note: 10 at its sides, an 11 pt lock, 6, the words.
-    let note = gettext(locale, "Read-only transaction");
-    let note_width =
-        10.0 + 11.0 + 6.0 + TextRole::Secondary.width(ui.ctx(), look.faces, &note) + 10.0;
+    let note_width = badge_width(ui, bar);
     // Everything 8 apart, and 16 between the two ends (8 at the tightest).
     // What gives way as the room runs out: the buttons' keys (the help
     // says them too), then the note, then Format (its key formats too),
@@ -604,26 +778,21 @@ fn mac_toolbar(ui: &mut Ui, rect: Rect, bar: &Bar<'_>, actions: &mut Vec<Action>
             pos2(limit.left() - 8.0 - note_width, center - 13.0),
             vec2(note_width, 26.0),
         );
-        ui.painter()
-            .rect_filled(pill, CornerRadius::same(13), palette.surface);
-        Icon::Lock.image(palette.secondary, 11.0).paint_at(
-            ui,
-            Rect::from_center_size(pos2(pill.left() + 15.5, center), vec2(11.0, 11.0)),
-        );
-        widgets::paint_text(
-            ui,
-            pill.left() + 27.0,
-            center,
-            Text::one(look, TextRole::Secondary, &note, palette.secondary),
-        );
-        explain_note(ui, pill, locale);
+        if let Some(mode) = self::badge(ui, pill, bar) {
+            actions.push(Action::SetSqlMode {
+                tab: bar.tab,
+                sql_tab: bar.id,
+                mode,
+            });
+        }
     }
     menus(ui, [limit, timeout], shape, bar, actions);
 }
 
 /// Omarchy: the tab's title and a muted `read-only transaction · limit
-/// 1000 · timeout 30s`, whose limit and timeout open the menus; at the
-/// right `run` and `run all` with their keys.
+/// 1000 · timeout 30s`, each part of which opens its menu (the first only
+/// on a connection that takes writes); at the right `run` and `run all`
+/// with their keys.
 fn terminal_toolbar(ui: &mut Ui, rect: Rect, bar: &Bar<'_>, actions: &mut Vec<Action>) {
     let Bar {
         locale,
@@ -654,10 +823,9 @@ fn terminal_toolbar(ui: &mut Ui, rect: Rect, bar: &Bar<'_>, actions: &mut Vec<Ac
     };
     let role = TextRole::OBody;
     let width = |role: TextRole, text: &str| role.width(ui.ctx(), look.faces, text);
-    let note = look.label(&gettext(locale, "Read-only transaction"));
     let (title_width, note_width, dot) = (
         width(TextRole::OTableTitle, bar.title),
-        width(role, &note),
+        badge_width(ui, bar),
         width(role, " · "),
     );
     // Everything 14 apart. What gives way as the room runs out: the
@@ -711,14 +879,15 @@ fn terminal_toolbar(ui: &mut Ui, rect: Rect, bar: &Bar<'_>, actions: &mut Vec<Ac
         }
         let line = role.row_height(ui.ctx(), look.faces);
         if noted {
-            let width =
-                widgets::paint_text(ui, x, center, Text::one(look, role, &note, palette.dim));
-            explain_note(
-                ui,
-                Rect::from_min_size(pos2(x, center - line / 2.0), vec2(width, line)),
-                locale,
-            );
-            x += width;
+            let words = Rect::from_min_size(pos2(x, center - line / 2.0), vec2(note_width, line));
+            if let Some(mode) = badge(ui, words, bar) {
+                actions.push(Action::SetSqlMode {
+                    tab: bar.tab,
+                    sql_tab: bar.id,
+                    mode,
+                });
+            }
+            x += note_width;
             x += widgets::paint_text(ui, x, center, dot_text());
         }
         // The menus read as one line with the note: their words, a dot
diff --git a/src/ui/widgets.rs b/src/ui/widgets.rs
index 0d6ed0e..f91a870 100644
--- a/src/ui/widgets.rs
+++ b/src/ui/widgets.rs
@@ -458,6 +458,9 @@ pub struct MenuChoice {
     pub name: Option<String>,
     /// The choice in use.
     pub selected: bool,
+    /// Why the choice cannot be picked, said on hover: it is shown all the
+    /// same, so the menu says what there is to choose.
+    pub disabled: Option<String>,
 }
 
 /// The menu that `button` opens when clicked: `choices` under it, at least
@@ -478,10 +481,16 @@ pub fn popup_menu(
         ui.set_min_width(min_width);
         for (index, choice) in choices().into_iter().enumerate() {
             let text = galley(ui, &choice.text, Color32::PLACEHOLDER, look);
-            let response = ui.add(egui::Button::selectable(choice.selected, text));
+            let enabled = choice.disabled.is_none();
+            let button = egui::Button::selectable(choice.selected, text);
+            let response = ui.add_enabled(enabled, button);
+            let response = match &choice.disabled {
+                Some(reason) => response.on_disabled_hover_text(reason),
+                None => response,
+            };
             if let Some(name) = &choice.name {
                 response.widget_info(|| {
-                    WidgetInfo::selected(WidgetType::Button, true, choice.selected, name)
+                    WidgetInfo::selected(WidgetType::Button, enabled, choice.selected, name)
                 });
             }
             if response.clicked() {
````

- [ ] **Step 4: Run the tests to see them pass**

```bash
~/.cargo/bin/cargo test --locked -p tabletist --lib -- sql_editor::tests
```

Expected: PASS, 12 passed and 0 failed.

- [ ] **Step 5: Run the four checks**

All four pass. Fix what does not before going on; do not weaken a lint to get there.

- [ ] **Step 6: Commit**

```bash
git add src/ui/sql_editor.rs src/ui/widgets.rs src/ui/sidebar.rs src/ui/keys.rs
git commit -S -m "Make the transaction badge a menu, and Mod+Shift+M its key" -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 6: The refused write: three cards at the head of the Messages

**Files:**
- Modify: `src/ui/sql_results.rs` (`Place`, `Why`, `Offer`, `Blocked`, `blocked`, `card_key`, `blocked_card`, `messages`, `results`, and its tests module)
- Modify: `src/ui/format.rs` (`refuses_writes`, and its tests module)
- Modify: `src/model.rs` (`Action::RunSqlAgain`, `SqlTab::ran_this_text`)
- Modify: `src/app.rs` (`run_sql_again`, and its tests module)
- Modify: `src/ui/keys.rs` (`letters`)
- Modify: `src/testing.rs` (`refused_write`)

A write a database refused in a read-only run is still a card and not a statement's error, but the card moves to where the user is looking, the head of the Messages (a failed statement opens them), and says why the run was read-only:

| As things stand now | Title and text | Offers |
|---|---|---|
| The connection opens read-only | "This connection opens read-only". "Bookshop · dev blocks writes, so PostgreSQL refused the UPDATE. Nothing changed." (Omarchy names the connection by its tag: "DEV blocks writes, ..."). Under the database's words: "To write, turn off Open read-only in the connection. It applies from the next connect." | **Edit connection** (Omarchy `e`) |
| The tab is in Read-only | "This tab runs read-only". "Every run here is a read-only transaction, so PostgreSQL refused the UPDATE. Nothing changed." | **Allow writes in this tab** (Omarchy `w`); on production, in this run, no button and the sentence of `UNCONFIRMED` |
| The tab is in Read-write, the run held nothing that looks like a write | "This run was read-only". "Its statements looked like reads, so they ran in a read-only transaction and PostgreSQL refused this one. Nothing changed." | **Run in a read-write transaction** (Omarchy `w`) |
| The tab is in Read-write, the run held a write (sent before the tab was switched, or while its session could not write) | "This run was read-only". "It was sent before this tab could write, so PostgreSQL refused the UPDATE. Nothing changed." | **Run in a read-write transaction** (Omarchy `w`) |

- The card scrolls with the lines under it, so a short pane still reaches them. Under the card stand the database's own words, after its code, then (for a read-only connection) what the way on asks of the user, then the button. That order, the card's warning tint and its lock are the "Write blocked" artboard's.
- `Action::RunSqlAgain` sends the statements of the tab's last run again, to write (`App::send_run`). The reducer looks again when it is applied: the tab's `run_mode` must be `ReadWrite`, the editor must still hold the text that ran (`SqlTab::ran_this_text`), no run may be in flight, and the last run must have been read-only. The button is offered only while the text is the one that ran.
- On Omarchy the button's letter works while the editor does not have the keyboard: `keys::letters` runs only then, and asks `sql_results::card_key` for the letter and the action, so the key does exactly what the button on screen does.
- What Tabletist's own guard refused (`Error::Refused`) is no card any more: `refuses_writes` drops that arm, and the refusal keeps its own sentence. In a run sent to write a read-only error (a standby, a role made read-only) is a statement's error like any other. Results draws no card.

**Tests this task replaces**, because the spec replaces the card they describe ("This replaces 'The SQL editor only reads data'"): `a_refused_write_says_the_editor_only_reads` gives way to one test for each card and one for the guard's refusal; `a_refused_write_in_a_short_pane_scrolls_to_its_last_line` keeps its name and its point, in the Messages; `another_error_keeps_its_own_words_in_the_results` becomes `another_error_is_no_refused_write`. In `src/ui/format.rs` one assertion flips: the guard's refusal is not a refused write.

- [ ] **Step 1: Write the failing tests**

Apply this block. It holds the task's tests, and applies on the tree as the task before left it.

````diff
diff --git a/src/app.rs b/src/app.rs
index 96edea7..4f04f62 100644
--- a/src/app.rs
+++ b/src/app.rs
@@ -6877,6 +6913,68 @@ mod tests {
         assert_eq!(runs_since(&harness, sent), 1);
     }
 
+    #[test]
+    fn run_again_sends_the_last_runs_statements_to_write_only_where_it_may() {
+        use crate::testing::{refused_write, script_outcome};
+        use tabletist_db::ScriptMode::{ReadOnly, Write};
+        // A function that writes behind a statement that reads, refused in
+        // the read-only run it was taken for.
+        let text = "SELECT setval('ids', 9)";
+        let refused = |harness: &mut Harness, mode: RunMode| {
+            let (tab, id) = writable_sql(harness);
+            set_mode(harness, tab, id, mode);
+            type_sql(harness, tab, id, text, 0);
+            run(harness, tab, id, false);
+            assert_eq!(sent_mode(harness), ReadOnly);
+            harness.answer_sql(Ok(script_outcome(vec![refused_write()])), None);
+            (tab, id)
+        };
+        let again = |harness: &mut Harness, tab, id| {
+            let sent = harness.app.backend.sent.len();
+            harness.app.apply(Action::RunSqlAgain { tab, sql_tab: id });
+            runs_since(harness, sent)
+        };
+        // In Read-write, with the text that ran: the same statements, sent
+        // to write.
+        let mut harness = Harness::new();
+        let (tab, id) = refused(&mut harness, RunMode::ReadWrite);
+        assert_eq!(again(&mut harness, tab, id), 1);
+        assert_eq!(sent_mode(&harness), Write);
+        assert!(matches!(
+            last_sent(&harness.app),
+            Command::RunSql { statements, .. }
+                if statements.len() == 1 && statements[0].text == text
+        ));
+        // While that run is in flight the card's button does nothing more.
+        assert_eq!(again(&mut harness, tab, id), 0);
+        // Nor after it: a run that was sent to write is not what the card
+        // was about.
+        commit(&mut harness, 1);
+        assert_eq!(again(&mut harness, tab, id), 0);
+        // Not once the text is another than the one that ran, and again
+        // when it is that text once more.
+        let mut harness = Harness::new();
+        let (tab, id) = refused(&mut harness, RunMode::ReadWrite);
+        type_sql(&mut harness, tab, id, "SELECT setval('ids', 10)", 0);
+        assert_eq!(again(&mut harness, tab, id), 0);
+        type_sql(&mut harness, tab, id, text, 0);
+        assert_eq!(again(&mut harness, tab, id), 1);
+        // Not in a tab that is in Read-only, or was switched back since.
+        let mut harness = Harness::new();
+        let (tab, id) = refused(&mut harness, RunMode::ReadOnly);
+        assert_eq!(again(&mut harness, tab, id), 0);
+        let mut harness = Harness::new();
+        let (tab, id) = refused(&mut harness, RunMode::ReadWrite);
+        set_mode(&mut harness, tab, id, RunMode::ReadOnly);
+        assert_eq!(again(&mut harness, tab, id), 0);
+        // Not once the session came back read-only, whatever the tab says.
+        let mut harness = Harness::new();
+        let (tab, id) = refused(&mut harness, RunMode::ReadWrite);
+        harness.reconnect_fake_as(tab, true);
+        assert_eq!(again(&mut harness, tab, id), 0);
+        assert_eq!(sql(&harness, tab, id).mode, RunMode::ReadWrite);
+    }
+
     #[test]
     fn messages_open_for_every_end_of_a_read_write_run_that_needs_reading() {
         use crate::testing::{done_outcome, write_outcome};
diff --git a/src/testing.rs b/src/testing.rs
index 9c27b14..caaf72f 100644
--- a/src/testing.rs
+++ b/src/testing.rs
@@ -735,6 +735,20 @@ pub fn error_outcome(message: &str, position: Option<usize>) -> tabletist_db::St
     }
 }
 
+/// A write refused in a read-only transaction, as PostgreSQL and MySQL say
+/// it (SQLSTATE 25006).
+pub fn refused_write() -> tabletist_db::StatementOutcome {
+    tabletist_db::StatementOutcome::Error {
+        error: tabletist_db::Error::Query {
+            code: Some("25006".into()),
+            message: "cannot execute UPDATE in a read-only transaction".into(),
+            detail: None,
+            hint: None,
+        },
+        position: None,
+    }
+}
+
 /// What a script did, as a driver reports it: one outcome per statement
 /// that started, each taking 14 ms. A `Cancelled` outcome is a stopped run,
 /// as it is for every driver.
diff --git a/src/ui/format.rs b/src/ui/format.rs
index f9d0b68..7947b94 100644
--- a/src/ui/format.rs
+++ b/src/ui/format.rs
@@ -876,7 +877,8 @@ mod tests {
             mode: tabletist_db::ScriptMode::ReadOnly,
         };
         for driver in [Driver::Postgres, Driver::MySql, Driver::Sqlite] {
-            assert!(refuses_writes(&refused, driver));
+            // The guard's refusal is about the transaction, in either mode.
+            assert!(!refuses_writes(&refused, driver));
             assert!(!refuses_writes(&Error::Timeout, driver));
         }
         // SQLSTATE 25006: read_only_sql_transaction.
diff --git a/src/ui/sql_results.rs b/src/ui/sql_results.rs
index d79b13f..e8b49b2 100644
--- a/src/ui/sql_results.rs
+++ b/src/ui/sql_results.rs
@@ -1475,7 +1710,7 @@ mod tests {
     use crate::backend::{CancelReason, Command};
     use crate::model::{Action, CellPos, ResultPane, SqlTab};
     use crate::testing::{
-        Harness, bounds, error_outcome, labels, node, rows_outcome, script_outcome,
+        Harness, bounds, error_outcome, labels, node, refused_write, rows_outcome, script_outcome,
         stopped_before_it_began, write_outcome,
     };
     use crate::theme::Look;
@@ -2037,62 +2272,309 @@ mod tests {
         }
     }
 
-    /// The server's refusal of a write in a read-only transaction.
-    fn read_only_refusal() -> StatementOutcome {
-        StatementOutcome::Error {
-            error: Error::Query {
-                code: Some("25006".into()),
-                message: "cannot execute UPDATE in a read-only transaction".into(),
-                detail: None,
-                hint: None,
-            },
-            position: None,
+    /// What PostgreSQL says of the refused write, as the card writes it
+    /// under itself.
+    const REFUSAL: &str = "25006 · cannot execute UPDATE in a read-only transaction";
+
+    /// The titles a refused write's card has.
+    const CARDS: [&str; 3] = [
+        "This connection opens read-only",
+        "This tab runs read-only",
+        "This run was read-only",
+    ];
+
+    /// A SQL editor on a connection that takes writes, on PostgreSQL, drawn
+    /// in `look`, with `text` typed into it.
+    fn writable(look: Look, text: &str) -> (Harness, ConnTabId) {
+        let mut harness = Harness::new();
+        harness.set_look(look);
+        let tab = harness.connect_fake_as(false);
+        harness.app.workspace_mut(tab).unwrap().driver = Driver::Postgres;
+        harness.press(Key::T, Modifiers::COMMAND);
+        harness.frame(vec![egui::Event::Paste(text.into())]);
+        harness.settle();
+        (harness, tab)
+    }
+
+    fn set_mode(harness: &mut Harness, tab: ConnTabId, mode: crate::model::RunMode) {
+        let sql_tab = sql(harness, tab).id;
+        harness.app.apply(Action::SetSqlMode { tab, sql_tab, mode });
+    }
+
+    /// The card on screen: its title and its text. A card's words are
+    /// read as they are painted, and the terminal look paints ours in
+    /// lower case: there both come back in lower case, to be compared with
+    /// [`reads`].
+    fn card(harness: &mut Harness) -> Option<(String, String)> {
+        let terminal = harness.app.look.terminal;
+        let reads = |text: &str| {
+            if terminal {
+                text.to_lowercase()
+            } else {
+                text.to_owned()
+            }
+        };
+        let lines = message_lines(harness);
+        let titles = CARDS.map(reads);
+        let at = lines.iter().position(|line| titles.contains(line))?;
+        Some((lines[at].clone(), reads(lines.get(at + 1)?)))
+    }
+
+    /// A card's title and text as [`card`] gives them in `look`.
+    fn reads(look: &Look, title: &str, text: &str) -> Option<(String, String)> {
+        if look.terminal {
+            Some((title.to_lowercase(), text.to_lowercase()))
+        } else {
+            Some((title.to_owned(), text.to_owned()))
+        }
+    }
+
+    /// The transaction and the statements of the newest run sent.
+    fn sent_run(harness: &Harness) -> (tabletist_db::ScriptMode, Vec<String>) {
+        let sent = harness.app.backend.sent.iter().rev();
+        sent.filter_map(|command| match command {
+            Command::RunSql {
+                mode, statements, ..
+            } => {
+                let texts = statements.iter().map(|statement| statement.text.clone());
+                Some((*mode, texts.collect()))
+            }
+            _ => None,
+        })
+        .next()
+        .expect("a RunSql was sent")
+    }
+
+    fn runs_sent(harness: &Harness) -> usize {
+        let sent = harness.app.backend.sent.iter();
+        sent.filter(|command| matches!(command, Command::RunSql { .. }))
+            .count()
+    }
+
+    /// Presses the card's button: by its letter in the terminal look, once
+    /// the editor has let go of the keyboard, and by a click in the others.
+    fn take_offer(harness: &mut Harness, look: &Look, name: &str, letter: Key) {
+        let tree = harness.settle();
+        assert!(
+            node(&tree, name, Role::Button).is_some(),
+            "{name} in {}",
+            look.name
+        );
+        if look.terminal {
+            harness.press(Key::Escape, Modifiers::NONE);
+            harness.press(letter, Modifiers::NONE);
+        } else {
+            harness.click(name);
         }
     }
 
     #[test]
-    fn a_refused_write_says_the_editor_only_reads() {
-        // The card, whole, as a look says it. On a connection that takes
-        // writes it ends with where values are edited.
-        let refused = "Every query runs in a read-only transaction, so this statement was \
-                       refused. Nothing changed.";
-        let says_so = |harness: &mut Harness, look: &Look, writable: bool, case: &str| {
-            let body = if writable {
-                format!("{refused} Edit values in a table's grid.")
-            } else {
-                refused.to_owned()
-            };
-            for ours in ["The SQL editor only reads data", body.as_str()] {
-                let said = look.label(ours);
-                assert!(harness.has(&said), "{said}, {case} in {}", look.name);
-            }
-        };
+    fn a_write_refused_on_a_read_only_connection_offers_the_connection() {
         for look in Look::ALL {
-            // The editor reads on a writable connection as on a read-only
-            // one, and the card says so on both. Only the way on differs:
-            // a read-only connection is not promised a grid that turning
-            // its box off would give.
-            for writable in [false, true] {
-                let (mut harness, tab) = editor(look, "UPDATE users SET email = 'x'");
-                let workspace = harness.app.workspace_mut(tab).unwrap();
-                // The fixture's session is SQLite's: the code is PostgreSQL's.
-                workspace.driver = tabletist_db::Driver::Postgres;
-                if writable {
-                    workspace.access = tabletist_db::Access::Writable;
-                }
-                run(&mut harness);
-                harness.answer_sql(Ok(script_outcome(vec![read_only_refusal()])), None);
-                show_pane(&mut harness, tab, ResultPane::Results);
-                let case = if writable { "writable" } else { "read-only" };
-                says_so(&mut harness, &look, writable, case);
-                assert!(
-                    harness.has("25006 · cannot execute UPDATE in a read-only transaction"),
-                    "{case} in {}",
-                    look.name
-                );
+            // After a statement that returned rows, too: the card leads the
+            // Messages, which the refusal opens.
+            let (mut harness, tab) = editor(look, "SELECT 1;\nUPDATE users SET email = 'x'");
+            harness.app.workspace_mut(tab).unwrap().driver = Driver::Postgres;
+            run_all(&mut harness);
+            harness.answer_sql(
+                Ok(script_outcome(vec![rows_outcome(1), refused_write()])),
+                None,
+            );
+            assert_eq!(sql(&harness, tab).pane, ResultPane::Messages);
+            // The connection by its name and environment, or by the
+            // terminal look's tag.
+            let text = if look.terminal {
+                "DEV blocks writes, so PostgreSQL refused the UPDATE. Nothing changed."
+            } else {
+                "Fixture · dev blocks writes, so PostgreSQL refused the UPDATE. Nothing changed."
+            };
+            assert_eq!(
+                card(&mut harness),
+                reads(&look, CARDS[0], text),
+                "{}",
+                look.name
+            );
+            // Under it the database's own words, then what the way on asks,
+            // and the statements' lines after them.
+            let then = "To write, turn off Open read-only in the connection. It applies from \
+                        the next connect.";
+            let lines = message_lines(&mut harness);
+            assert_eq!(lines[2], REFUSAL, "{}", look.name);
+            assert_eq!(lines[3], look.label(then), "{}", look.name);
+            assert_eq!(
+                lines[lines.len() - 3..],
+                [
+                    "Line 1: 1 row · 14 ms",
+                    "Line 2: cannot execute UPDATE in a read-only transaction",
+                    "Code: 25006",
+                ],
+                "{}",
+                look.name
+            );
+            // The terminal writes all of it in lower case but the tag.
+            if look.terminal {
+                let reads = "DEV blocks writes, so postgresql refused the update. nothing \
+                             changed.";
+                assert!(painted(&harness, reads), "{:?}", harness.painted);
+                assert!(painted(&harness, "this connection opens read-only"));
             }
-            // What the editor's own guard refuses reads the same.
-            let (mut harness, tab) = editor(look, "COMMIT");
+            // Results shows the rows it has, and no card.
+            harness.click("Results");
+            assert!(harness.has("Row 1"), "{}", look.name);
+            assert_eq!(card(&mut harness), None, "{}", look.name);
+            // The way on is the connection's own dialog.
+            harness.click("Messages");
+            take_offer(&mut harness, &look, "Edit connection", Key::E);
+            let conn = harness.app.workspace(tab).unwrap().conn_id.clone();
+            assert!(
+                matches!(
+                    &harness.app.dialog,
+                    Some(crate::model::Dialog::Connection(form)) if form.editing == Some(conn)
+                ),
+                "{}",
+                look.name
+            );
+        }
+    }
+
+    #[test]
+    fn a_write_refused_in_a_read_only_tab_offers_to_allow_writes_and_then_to_run_again() {
+        use crate::model::RunMode;
+        use tabletist_db::ScriptMode;
+        for look in Look::ALL {
+            let update = "UPDATE users SET email = 'x'";
+            let (mut harness, tab) = writable(look, update);
+            run(&mut harness);
+            assert_eq!(sent_run(&harness).0, ScriptMode::ReadOnly);
+            harness.answer_sql(Ok(script_outcome(vec![refused_write()])), None);
+            let text = "Every run here is a read-only transaction, so PostgreSQL refused the \
+                        UPDATE. Nothing changed.";
+            assert_eq!(
+                card(&mut harness),
+                reads(&look, CARDS[1], text),
+                "{}",
+                look.name
+            );
+            assert!(harness.has(REFUSAL), "{}", look.name);
+            // Allowing writes sets the mode and runs nothing.
+            let sent = runs_sent(&harness);
+            take_offer(&mut harness, &look, "Allow writes in this tab", Key::W);
+            assert_eq!(sql(&harness, tab).mode, RunMode::ReadWrite, "{}", look.name);
+            assert_eq!(runs_sent(&harness), sent, "{}", look.name);
+            // The card stays, and says what is true of the run now.
+            let text = "It was sent before this tab could write, so PostgreSQL refused the \
+                        UPDATE. Nothing changed.";
+            assert_eq!(
+                card(&mut harness),
+                reads(&look, CARDS[2], text),
+                "{}",
+                look.name
+            );
+            take_offer(
+                &mut harness,
+                &look,
+                "Run in a read-write transaction",
+                Key::W,
+            );
+            assert_eq!(
+                sent_run(&harness),
+                (ScriptMode::Write, vec![update.to_owned()]),
+                "{}",
+                look.name
+            );
+            assert_eq!(runs_sent(&harness), sent + 1, "{}", look.name);
+        }
+    }
+
+    #[test]
+    fn a_write_taken_for_a_read_offers_to_run_it_in_a_read_write_transaction() {
+        use crate::model::RunMode;
+        use tabletist_db::ScriptMode;
+        for look in Look::ALL {
+            // A function that writes, behind a statement that reads.
+            let script = "SELECT 1;\nSELECT setval('ids', 9)";
+            let (mut harness, tab) = writable(look, script);
+            set_mode(&mut harness, tab, RunMode::ReadWrite);
+            run_all(&mut harness);
+            assert_eq!(sent_run(&harness).0, ScriptMode::ReadOnly, "{}", look.name);
+            harness.answer_sql(
+                Ok(script_outcome(vec![rows_outcome(1), refused_write()])),
+                None,
+            );
+            let text = "Its statements looked like reads, so they ran in a read-only transaction \
+                        and PostgreSQL refused this one. Nothing changed.";
+            assert_eq!(
+                card(&mut harness),
+                reads(&look, CARDS[2], text),
+                "{}",
+                look.name
+            );
+            take_offer(
+                &mut harness,
+                &look,
+                "Run in a read-write transaction",
+                Key::W,
+            );
+            // What ran is what is sent again: both statements, to write.
+            let both = vec!["SELECT 1".to_owned(), "SELECT setval('ids', 9)".to_owned()];
+            assert_eq!(
+                sent_run(&harness),
+                (ScriptMode::Write, both),
+                "{}",
+                look.name
+            );
+            assert!(sql(&harness, tab).is_writing(), "{}", look.name);
+        }
+    }
+
+    #[test]
+    fn a_run_is_not_offered_again_once_the_text_is_another() {
+        use crate::model::RunMode;
+        let (mut harness, tab) = writable(Look::standard(), "SELECT setval('ids', 9)");
+        set_mode(&mut harness, tab, RunMode::ReadWrite);
+        run(&mut harness);
+        harness.answer_sql(Ok(script_outcome(vec![refused_write()])), None);
+        assert!(harness.has("Run in a read-write transaction"));
+        // Typed into since: what would be sent is not what the editor shows.
+        harness.frame(vec![egui::Event::Text(" ".into())]);
+        assert!(card(&mut harness).is_some());
+        assert!(!harness.has("Run in a read-write transaction"));
+    }
+
+    #[test]
+    fn on_production_the_card_says_why_the_tab_cannot_write_and_offers_nothing() {
+        for look in Look::ALL {
+            let (mut harness, tab) = writable(look, "UPDATE users SET email = 'x'");
+            harness.app.workspace_mut(tab).unwrap().environment =
+                crate::env::Environment::Production;
+            run(&mut harness);
+            harness.answer_sql(Ok(script_outcome(vec![refused_write()])), None);
+            let text = "Every run here is a read-only transaction, so PostgreSQL refused the \
+                        UPDATE. Nothing changed. Read-write runs on a production connection are \
+                        not available yet.";
+            assert_eq!(
+                card(&mut harness),
+                reads(&look, CARDS[1], text),
+                "{}",
+                look.name
+            );
+            assert!(!harness.has("Allow writes in this tab"), "{}", look.name);
+            // Nor does the terminal's letter do what no button offers.
+            harness.press(Key::Escape, Modifiers::NONE);
+            harness.press(Key::W, Modifiers::NONE);
+            assert_eq!(
+                sql(&harness, tab).mode,
+                crate::model::RunMode::ReadOnly,
+                "{}",
+                look.name
+            );
+        }
+    }
+
+    #[test]
+    fn what_the_guard_refuses_keeps_its_own_sentence_and_is_no_card() {
+        for look in Look::ALL {
+            let (mut harness, _tab) = editor(look, "COMMIT");
             run(&mut harness);
             let refused = Error::Refused {
                 line: 1,
@@ -2100,12 +2582,31 @@ mod tests {
                 mode: tabletist_db::ScriptMode::ReadOnly,
             };
             harness.answer_sql(Err(refused.clone()), None);
-            show_pane(&mut harness, tab, ResultPane::Results);
-            says_so(&mut harness, &look, false, "the editor's guard");
-            assert!(harness.has(&refused.to_string()), "{}", look.name);
+            assert_eq!(card(&mut harness), None, "{}", look.name);
+            assert_eq!(message_lines(&mut harness), [refused.to_string()]);
         }
     }
 
+    #[test]
+    fn in_a_read_write_run_a_read_only_error_is_a_statements_error() {
+        // A standby, or a role an administrator made read-only: the run
+        // was sent to write, so no card offers what was already done.
+        let (mut harness, _tab) = writing(Look::standard(), Driver::Postgres, CHANGES);
+        let outcome = write_outcome(vec![refused_write()], ScriptEnd::RolledBack);
+        harness.answer_sql(Ok(outcome), None);
+        assert_eq!(card(&mut harness), None);
+        assert_eq!(
+            message_lines(&mut harness),
+            [
+                "Line 1: cannot execute UPDATE in a read-only transaction",
+                "Code: 25006",
+                "Line 2: Not run",
+                "Line 3: Not run",
+                "Rolled back. Nothing was written.",
+            ]
+        );
+    }
+
     #[test]
     fn a_refused_write_in_a_short_pane_scrolls_to_its_last_line() {
         for look in Look::ALL {
@@ -2113,13 +2614,13 @@ mod tests {
             let workspace = harness.app.workspace_mut(tab).unwrap();
             workspace.driver = tabletist_db::Driver::Postgres;
             // The editor takes most of the height: the results have room
-            // for the card's first lines and not for its last.
+            // for the card's first lines and not for what follows it.
             let id = workspace.active_tab.unwrap();
             workspace.sql_tab_mut(id).unwrap().split = 0.8;
             run(&mut harness);
-            // The card ends in the database's own words, and a database
-            // may say a lot: enough here to wrap to several lines, so the
-            // card overflows the pane by lines and not by a few points.
+            // A database may say a lot: enough here to wrap to several
+            // lines, so the messages overflow the pane by lines and not by
+            // a few points.
             let message = "cannot execute UPDATE in a read-only transaction; ".repeat(12);
             let refusal = StatementOutcome::Error {
                 error: Error::Query {
@@ -2131,12 +2632,12 @@ mod tests {
                 position: None,
             };
             harness.answer_sql(Ok(script_outcome(vec![refusal])), None);
-            show_pane(&mut harness, tab, ResultPane::Results);
-            let title = look.label("The SQL editor only reads data");
-            let last = format!("25006 · {message}");
-            // The card's title and its last line, and how far down the
-            // pane shows anything: to the footer under it, or to the
-            // window's end in the terminal look, which has none.
+            assert_eq!(sql(&harness, tab).pane, ResultPane::Messages);
+            let title = look.label(CARDS[0]);
+            let last = "Code: 25006";
+            // The card's title and the messages' last line, and how far
+            // down the pane shows anything: to the footer under it, or to
+            // the window's end in the terminal look, which has none.
             let places = |harness: &mut Harness| {
                 let tree = harness.settle();
                 let place = |label: &str| bounds(&tree, label, Role::Label);
@@ -2147,16 +2648,21 @@ mod tests {
                     Some(footer) => footer.top(),
                     None => harness.size.y,
                 };
-                let [title, last] = [&title, &last].map(|label| place(label).expect("a label"));
-                (title, last, bottom)
+                (place(&title), place(last), bottom)
             };
-            let (title, before, bottom) = places(&mut harness);
-            assert!(before.bottom() > bottom, "{}: {before:?}", look.name);
-            // The wheel over the card brings the rest of it up.
-            harness.frame(vec![egui::Event::PointerMoved(title.center())]);
+            let (title_at, before, bottom) = places(&mut harness);
+            let title_at = title_at.expect("the card leads the messages");
+            // Not built at all, or under the pane's end.
+            assert!(
+                before.is_none_or(|last| last.bottom() > bottom),
+                "{}: {before:?}",
+                look.name
+            );
+            // The wheel over the card brings the rest up.
+            harness.frame(vec![egui::Event::PointerMoved(title_at.center())]);
             harness.frame(vec![egui::Event::MouseWheel {
                 unit: egui::MouseWheelUnit::Point,
-                delta: egui::vec2(0.0, -600.0),
+                delta: egui::vec2(0.0, -2000.0),
                 modifiers: Modifiers::NONE,
                 phase: egui::TouchPhase::Move,
             }]);
@@ -2164,19 +2670,21 @@ mod tests {
                 harness.frame(Vec::new());
             }
             let (_, after, bottom) = places(&mut harness);
+            let after = after.expect("the last line is built once it is in view");
             assert!(after.bottom() <= bottom, "{}: {after:?}", look.name);
         }
     }
 
     #[test]
-    fn another_error_keeps_its_own_words_in_the_results() {
+    fn another_error_is_no_refused_write() {
         let look = Look::macos();
         let (mut harness, tab) = editor(look, "SELECT nope");
         run(&mut harness);
         let failed = error_outcome("no such column: nope", None);
         harness.answer_sql(Ok(script_outcome(vec![failed])), None);
+        assert_eq!(card(&mut harness), None);
         show_pane(&mut harness, tab, ResultPane::Results);
-        assert!(!harness.has("The SQL editor only reads data"));
+        assert!(harness.has("Line 1: no such column: nope"));
     }
 
     #[test]
````

- [ ] **Step 2: Run the tests to see them fail**

```bash
~/.cargo/bin/cargo test --locked -p tabletist --lib -- sql_results::tests format::tests run_again_sends
```

Expected: does not compile. `no variant named RunSqlAgain found for enum model::Action`.

- [ ] **Step 3: Write the code**

````diff
diff --git a/src/app.rs b/src/app.rs
index 96edea7..4f04f62 100644
--- a/src/app.rs
+++ b/src/app.rs
@@ -1038,6 +1038,7 @@ impl App {
                 }
                 self.run_sql(tab, sql_tab, all);
             }
+            Action::RunSqlAgain { tab, sql_tab } => self.run_sql_again(tab, sql_tab),
             Action::SqlTyped { tab, sql_tab } => {
                 if let Some(sql) = self.sql_tab_mut(tab, sql_tab) {
                     // A list asked for by hand in the same frame stays so.
@@ -3227,6 +3228,41 @@ impl App {
         self.send_run(tab, id, statements, mode);
     }
 
+    /// The card's "Run in a read-write transaction": sends the statements
+    /// of the editor's last run again, to write. That run was read-only,
+    /// its statements having looked like reads or the tab having been in
+    /// Read-only then, and the database refused one of them.
+    ///
+    /// The editor's mode is looked at again here: a card left on screen
+    /// writes nothing for a tab that was switched back since, or whose
+    /// session came back read-only. Nor once the text is no longer the one
+    /// that ran: what would be sent is not what the editor shows.
+    fn run_sql_again(&mut self, tab: ConnTabId, id: TabId) {
+        let Some(workspace) = self.workspace(tab) else {
+            return;
+        };
+        if !matches!(workspace.status, SessionStatus::Connected) {
+            return;
+        }
+        let Some(sql) = workspace.sql_tab(id) else {
+            return;
+        };
+        if workspace.run_mode(sql) != RunMode::ReadWrite || !sql.ran_this_text() {
+            return;
+        }
+        let Some(run) = sql
+            .last_run()
+            .filter(|run| run.mode == tabletist_db::ScriptMode::ReadOnly)
+        else {
+            return;
+        };
+        let statements = run.statements.clone();
+        if statements.is_empty() {
+            return;
+        }
+        self.send_run(tab, id, statements, tabletist_db::ScriptMode::Write);
+    }
+
     /// Sends `statements` to the editor's session as its run, in a
     /// transaction of `mode`.
     fn send_run(
diff --git a/src/model.rs b/src/model.rs
index a436fe6..6fedbc1 100644
--- a/src/model.rs
+++ b/src/model.rs
@@ -342,6 +342,12 @@ pub enum Action {
         sql_tab: TabId,
         all: bool,
     },
+    /// The card of a refused write: run the statements of the editor's
+    /// last run again, in a read-write transaction.
+    RunSqlAgain {
+        tab: ConnTabId,
+        sql_tab: TabId,
+    },
     /// Format the editor's script, or the statements its selection
     /// overlaps.
     FormatSql {
@@ -2536,6 +2542,13 @@ impl SqlTab {
         self.run.value.as_ref()
     }
 
+    /// Whether the editor still holds the text its last run started with,
+    /// and no run is in flight: what that run says of a line, or offers to
+    /// do with its statements, holds only then.
+    pub fn ran_this_text(&self) -> bool {
+        !self.is_running() && self.ran_text.is_some_and(|ran| ran.is_of(&self.text))
+    }
+
     /// Whether the last run was sent to write and lost its session before
     /// its end was known: some or all of it may be written, and nothing
     /// can say which.
diff --git a/src/ui/format.rs b/src/ui/format.rs
index f9d0b68..7947b94 100644
--- a/src/ui/format.rs
+++ b/src/ui/format.rs
@@ -767,17 +767,18 @@ pub fn capped(text: &str) -> Cow<'_, str> {
     }
 }
 
-/// Whether `error` is a write that was refused: by Tabletist's own guard,
-/// or by the database, be it a read-only session or, on a writable one,
-/// the script's read-only transaction (SQLite's `query_only`). PostgreSQL
-/// and MySQL say SQLSTATE 25006; SQLite says SQLITE_READONLY (8) and no
-/// more. Its extended codes keep the 8 in their low byte and are not a
-/// refused write: a journal to recover, a lock or a directory it cannot
-/// have, which a SELECT can meet.
+/// Whether `error` is a write the database refused: on a read-only
+/// session or, on a writable one, in the script's read-only transaction
+/// (SQLite's `query_only`). PostgreSQL and MySQL say SQLSTATE 25006; SQLite
+/// says SQLITE_READONLY (8) and no more. Its extended codes keep the 8 in
+/// their low byte and are not a refused write: a journal to recover, a
+/// lock or a directory it cannot have, which a SELECT can meet. What
+/// Tabletist's own guard refuses (`Error::Refused`) is none either: that
+/// list is about the run's transaction, not about writing, and its
+/// sentence says so.
 pub fn refuses_writes(error: &tabletist_db::Error, driver: tabletist_db::Driver) -> bool {
     use tabletist_db::{Driver, Error};
     match error {
-        Error::Refused { .. } => true,
         Error::Query {
             code: Some(code), ..
         } => match driver {
diff --git a/src/ui/keys.rs b/src/ui/keys.rs
index 8c351ad..82419c2 100644
--- a/src/ui/keys.rs
+++ b/src/ui/keys.rs
@@ -1297,6 +1297,13 @@ fn letters(app: &mut App, ctx: &egui::Context, actions: &mut Vec<Action>) {
             actions.push(Action::FoldDocuments { tab, id });
         }
     }
+    // The card of a refused write, while a SQL editor's Messages show it:
+    // the letter its button names.
+    if let Some((key, action)) = crate::ui::sql_results::card_key(app, tab)
+        && pressed(key)
+    {
+        actions.push(action);
+    }
     // The letters below act on an object tab: none of them on a SQL editor.
     let Some(object_tab) = active else {
         ctx.data_mut(|data| data.insert_temp(pending_id(), next_pending));
diff --git a/src/ui/sql_results.rs b/src/ui/sql_results.rs
index d79b13f..e8b49b2 100644
--- a/src/ui/sql_results.rs
+++ b/src/ui/sql_results.rs
@@ -12,7 +12,9 @@ use tabletist_db::{Driver, Error, ScriptEnd, ScriptMode, StatementOutcome, Value
 use crate::app::App;
 use crate::backend::{CancelReason, RequestId};
 use crate::i18n::{Locale, gettext};
-use crate::model::{Action, ConnTabId, ResultPane, SqlRun, SqlTab, TabId};
+use crate::model::{
+    Action, ConnTabId, NoWrites, ResultPane, RunMode, SqlRun, SqlTab, TabId, Workspace,
+};
 use crate::theme::{Icon, Look, Palette};
 use crate::typography::{Laid, Text, TextRole};
 use crate::ui::data_view;
@@ -185,9 +187,9 @@ struct Place<'a> {
     fit: data_view::Fit,
     /// Whose error codes the results read.
     driver: tabletist_db::Driver,
-    /// Whether the connection takes writes: a write the editor refused has
-    /// a table's grid to go to.
-    writable: bool,
+    /// The write a database refused in the last run, when that run was
+    /// read-only: the Messages lead with its card.
+    blocked: Option<Blocked<'a>>,
     /// Whether the arrow keys move in the result's grid.
     keys: bool,
 }
@@ -210,7 +212,7 @@ fn draw(app: &App, ui: &mut Ui, tab: ConnTabId, id: TabId, actions: &mut Vec<Act
         sql,
         fit: data_view::Fit::of(workspace, &app.settings),
         driver: workspace.driver,
-        writable: workspace.access == tabletist_db::Access::Writable,
+        blocked: blocked(workspace, sql),
         keys: workspace.pane == crate::model::Pane::Grid,
     };
     let state = state(sql);
@@ -233,19 +235,15 @@ fn draw(app: &App, ui: &mut Ui, tab: ConnTabId, id: TabId, actions: &mut Vec<Act
             );
         }
         (State::Failed(error), ResultPane::Results) => {
-            if format::refuses_writes(error, place.driver) {
-                blocked(&mut body, rest, error, place.writable, &env);
-            } else {
-                let said = match may_be_written(sql, error) {
-                    Some(ours) => {
-                        let error = error_text(error);
-                        let error = error.trim_end_matches('.');
-                        env.said(|words| format!("{error}. {}", words.say(ours)))
-                    }
-                    None => whole(error),
-                };
-                note(&body, rest, &said, palette.danger, &env);
-            }
+            let said = match may_be_written(sql, error) {
+                Some(ours) => {
+                    let error = error_text(error);
+                    let error = error.trim_end_matches('.');
+                    env.said(|words| format!("{error}. {}", words.say(ours)))
+                }
+                None => whole(error),
+            };
+            note(&body, rest, &said, palette.danger, &env);
         }
         (State::Failed(error), ResultPane::Messages) => {
             let unknown = may_be_written(sql, error).map(|ours| Line::Ours(ours, Tone::Failed));
@@ -253,11 +251,11 @@ fn draw(app: &App, ui: &mut Ui, tab: ConnTabId, id: TabId, actions: &mut Vec<Act
                 .chain(more(error))
                 .chain(unknown)
                 .collect();
-            messages(&mut body, &lines, None, &place, &env);
+            messages(&mut body, &lines, None, &place, &env, actions);
         }
         (State::Ran(run), ResultPane::Results) => results(&mut body, run, &place, &env, actions),
         (State::Ran(run), ResultPane::Messages) => {
-            messages(&mut body, &lines(run), Some(run), &place, &env);
+            messages(&mut body, &lines(run), Some(run), &place, &env, actions);
         }
     }
 }
@@ -1135,13 +1133,16 @@ fn message(line: Line<'_>, run: Option<&SqlRun>, driver: Driver, words: Words) -
 }
 
 /// The messages: one line per statement of `run`, or why the run failed
-/// as a whole. Only the lines in view are written and laid out.
+/// as a whole. Only the lines in view are written and laid out. Over them,
+/// the card of a write a database refused in a read-only run: it scrolls
+/// with the lines, which a short pane then still reaches.
 fn messages(
     ui: &mut Ui,
     lines: &[Line<'_>],
     run: Option<&SqlRun>,
     place: &Place<'_>,
     env: &Env<'_>,
+    actions: &mut Vec<Action>,
 ) {
     let Env {
         look,
@@ -1195,6 +1196,9 @@ fn messages(
         .auto_shrink([false, false])
         .show(ui, |ui| {
             ui.add_space(8.0);
+            if let Some(blocked) = &place.blocked {
+                blocked_card(ui, blocked, place, env, actions);
+            }
             widgets::virtual_rows_varying(ui, &heights, |ui, index| {
                 let line = lines[index];
                 let size = vec2(ui.available_width(), heights[index]);
@@ -1217,58 +1221,293 @@ fn messages(
     keep_messages(ui.ctx(), results_id(place.tab, place.sql.id), area.id);
 }
 
-/// A write that was refused: said as what it is, the editor reading only,
-/// and not as a mistake in the statement. That holds on every connection,
-/// a writable one too, and whoever refused it: the server, SQLite, or the
-/// editor's own guard. A `writable` connection is told where values are
-/// edited. The exact error stays under the card. A pane too short for it
-/// all scrolls.
-fn blocked(ui: &mut Ui, rect: Rect, error: &Error, writable: bool, env: &Env<'_>) {
-    let Env { look, palette, .. } = *env;
-    let inner = rect.shrink2(vec2(16.0, 14.0));
-    let mut pane = ui.new_child(
-        egui::UiBuilder::new()
-            .id_salt("blocked")
-            .max_rect(inner)
-            .layout(egui::Layout::top_down(egui::Align::Min)),
-    );
-    // As short as the pane is: a scroll area keeps 64 pt by itself, which
-    // would run out under the pane.
-    egui::ScrollArea::vertical()
-        .auto_shrink([false, false])
-        .min_scrolled_height(0.0)
-        .show(&mut pane, |column| {
-            column.spacing_mut().item_spacing = vec2(8.0, 10.0);
-            let title = env.said(|words| words.say("The SQL editor only reads data"));
-            let text = env.said(|words| {
-                let refused = words.say(
-                    "Every query runs in a read-only transaction, so this statement was \
-                     refused. Nothing changed.",
+/// Why the run of a refused write was read-only, as things stand now: what
+/// its card says and offers follows from it.
+#[derive(Debug, Clone, Copy, PartialEq, Eq)]
+enum Why {
+    /// The connection opens read-only.
+    Connection,
+    /// The tab's runs are read-only. `unconfirmed`: and cannot be switched
+    /// here (see `NoWrites::Unconfirmed`).
+    Tab { unconfirmed: bool },
+    /// The tab's runs are read-write, and this one held nothing that
+    /// looked like a write.
+    TakenForRead,
+    /// The tab's runs are read-write now and were not when this one, which
+    /// holds a write, was sent: the tab was in Read-only then, or its
+    /// session could not write.
+    SentBefore,
+}
+
+/// What the card of a refused write offers.
+#[derive(Debug, Clone, Copy, PartialEq, Eq)]
+enum Offer {
+    EditConnection,
+    AllowWrites,
+    RunAgain,
+}
+
+impl Offer {
+    /// The button's name, and the letter that presses it in the terminal
+    /// look.
+    fn names(self) -> (&'static str, &'static str, egui::Key) {
+        match self {
+            Self::EditConnection => ("Edit connection", "e", egui::Key::E),
+            Self::AllowWrites => ("Allow writes in this tab", "w", egui::Key::W),
+            Self::RunAgain => ("Run in a read-write transaction", "w", egui::Key::W),
+        }
+    }
+}
+
+/// A write a database refused in a read-only run.
+#[derive(Clone, Copy)]
+struct Blocked<'a> {
+    /// What the database said.
+    error: &'a Error,
+    /// The statement it refused.
+    statement: Option<&'a tabletist_db::sql::Statement>,
+    why: Why,
+    /// Whether the run can be sent again as it is: the editor still holds
+    /// its text.
+    again: bool,
+    workspace: &'a Workspace,
+}
+
+/// The write a database refused in the editor's last run, when that run
+/// was read-only. In a run sent to write a read-only error (a standby, a
+/// role made read-only) is a statement's error like any other.
+fn blocked<'a>(workspace: &'a Workspace, sql: &'a SqlTab) -> Option<Blocked<'a>> {
+    use tabletist_db::sql::{StatementKind, kind};
+    let run = sql.last_run()?;
+    if run.mode != ScriptMode::ReadOnly {
+        return None;
+    }
+    let driver = workspace.driver;
+    let refused =
+        |(index, result): (usize, &'a tabletist_db::StatementResult)| match &result.outcome {
+            StatementOutcome::Error { error, .. } if format::refuses_writes(error, driver) => {
+                Some((index, error))
+            }
+            _ => None,
+        };
+    let (index, error) = run.outcome.results.iter().enumerate().find_map(refused)?;
+    let writes = |statement: &tabletist_db::sql::Statement| {
+        kind(driver.dialect(), &statement.text) == StatementKind::Write
+    };
+    let why = match workspace.sql_writes() {
+        Err(NoWrites::ReadOnlyConnection) => Why::Connection,
+        Err(NoWrites::Unconfirmed) => Why::Tab { unconfirmed: true },
+        Ok(()) if sql.mode == RunMode::ReadOnly => Why::Tab { unconfirmed: false },
+        // In Read-write a run that holds a write is sent to write: this
+        // one was sent before the tab was switched.
+        Ok(()) if run.statements.iter().any(writes) => Why::SentBefore,
+        Ok(()) => Why::TakenForRead,
+    };
+    Some(Blocked {
+        error,
+        statement: run.statements.get(index),
+        why,
+        again: sql.ran_this_text(),
+        workspace,
+    })
+}
+
+impl Blocked<'_> {
+    /// What the card's button does, where it has one: nothing is offered
+    /// that would do nothing.
+    fn offer(&self) -> Option<Offer> {
+        match self.why {
+            Why::Connection => Some(Offer::EditConnection),
+            Why::Tab { unconfirmed: false } => Some(Offer::AllowWrites),
+            Why::Tab { unconfirmed: true } => None,
+            Why::TakenForRead | Why::SentBefore => self.again.then_some(Offer::RunAgain),
+        }
+    }
+
+    fn action(&self, offer: Offer, tab: ConnTabId, sql_tab: TabId) -> Action {
+        match offer {
+            Offer::EditConnection => Action::EditConnection(self.workspace.conn_id.clone()),
+            Offer::AllowWrites => Action::SetSqlMode {
+                tab,
+                sql_tab,
+                mode: RunMode::ReadWrite,
+            },
+            Offer::RunAgain => Action::RunSqlAgain { tab, sql_tab },
+        }
+    }
+
+    /// The card's title and text, and the line that stands under the
+    /// database's words where the way on needs saying. The terminal look
+    /// writes all of it in lower case but the connection, which there is
+    /// the environment's tag, as its design has it ("PROD blocks writes").
+    fn says(&self, words: Words) -> (String, String, Option<String>) {
+        let workspace = self.workspace;
+        let driver = words.own(workspace.driver.label());
+        // "the UPDATE": the refused statement's first word.
+        let verb = self
+            .statement
+            .and_then(|statement| {
+                let words = tabletist_db::sql::words(workspace.driver.dialect(), &statement.text);
+                words.into_iter().next()
+            })
+            .map_or_else(
+                || words.say("the statement"),
+                |verb| format!("{} {}", words.say("the"), words.own(&verb)),
+            );
+        let nothing = words.say("Nothing changed.");
+        let refused = words.say("refused");
+        match self.why {
+            Why::Connection => (
+                words.say("This connection opens read-only"),
+                format!(
+                    "{} {} {driver} {refused} {verb}. {nothing}",
+                    self.who(words),
+                    words.say("blocks writes, so")
+                ),
+                Some(words.say(
+                    "To write, turn off Open read-only in the connection. It applies from the \
+                     next connect.",
+                )),
+            ),
+            Why::Tab { unconfirmed } => {
+                let mut text = format!(
+                    "{} {driver} {refused} {verb}. {nothing}",
+                    words.say("Every run here is a read-only transaction, so")
                 );
-                if writable {
-                    format!("{refused} {}", words.say("Edit values in a table's grid."))
-                } else {
-                    refused
+                if unconfirmed {
+                    text.push(' ');
+                    text.push_str(&words.say(super::sql_editor::UNCONFIRMED));
                 }
-            });
+                (words.say("This tab runs read-only"), text, None)
+            }
+            Why::TakenForRead => (
+                words.say("This run was read-only"),
+                format!(
+                    "{} {driver} {refused} {}. {nothing}",
+                    words.say(
+                        "Its statements looked like reads, so they ran in a read-only \
+                         transaction and"
+                    ),
+                    words.say("this one")
+                ),
+                None,
+            ),
+            Why::SentBefore => (
+                words.say("This run was read-only"),
+                format!(
+                    "{} {driver} {refused} {verb}. {nothing}",
+                    words.say("It was sent before this tab could write, so")
+                ),
+                None,
+            ),
+        }
+    }
+
+    /// The connection, as the card of a read-only one names it: its name
+    /// and environment, or in the terminal look the environment's tag
+    /// alone. A connection of no environment goes by its name.
+    fn who(&self, words: Words) -> String {
+        use crate::env::{Environment, Platform};
+        let workspace = self.workspace;
+        let name = format::display_safe(&workspace.name);
+        match (workspace.environment, words.lower) {
+            (Environment::None, _) => name.into_owned(),
+            (environment, true) => environment.label(Platform::Omarchy).to_owned(),
+            (environment, false) => format!("{name} · {}", environment.label(Platform::Native)),
+        }
+    }
+}
+
+/// The action the card of a refused write offers in the SQL editor on
+/// screen, and the letter that takes it in the terminal look. Only while
+/// the Messages show the card.
+pub(crate) fn card_key(app: &App, tab: ConnTabId) -> Option<(egui::Key, Action)> {
+    let workspace = app.workspace(tab)?;
+    let sql = workspace.active_sql_tab()?;
+    if sql.pane != ResultPane::Messages || sql.run.error.is_some() {
+        return None;
+    }
+    let blocked = blocked(workspace, sql)?;
+    let offer = blocked.offer()?;
+    Some((offer.names().2, blocked.action(offer, tab, sql.id)))
+}
+
+/// The card of a refused write, at the head of the Messages: said as what
+/// it is, a run that was read-only, and not as a mistake in the statement.
+/// Under it the database's own words, then the way on: what it asks of the
+/// user where that needs saying, and the button to the connection, to a
+/// tab that writes, or to the same run sent to write.
+fn blocked_card(
+    ui: &mut Ui,
+    blocked: &Blocked<'_>,
+    place: &Place<'_>,
+    env: &Env<'_>,
+    actions: &mut Vec<Action>,
+) {
+    let Env {
+        look,
+        palette,
+        locale,
+    } = *env;
+    let pad = side(look) as i8;
+    egui::Frame::new()
+        .inner_margin(egui::Margin {
+            left: pad,
+            right: pad,
+            top: 0,
+            bottom: 10,
+        })
+        .show(ui, |column| {
+            column.set_width(column.available_width());
+            column.spacing_mut().item_spacing = vec2(8.0, 10.0);
+            let said = env.said(|words| blocked.says(words).0);
+            let text = env.said(|words| blocked.says(words).1);
+            let then = env.said(|words| blocked.says(words).2.unwrap_or_default());
             let card = states::Card {
                 tone: states::Tone::Warning,
                 icon: Icon::Lock,
-                title: &title.painted,
+                title: &said.painted,
                 text: &text.painted,
             };
             states::card(column, &card, look, palette);
             // The database's own words, after its code when it gave one.
-            let raw = match error {
+            let raw = match blocked.error {
                 Error::Query {
                     code: Some(code), ..
-                } => format!("{code} · {}", error_text(error)),
+                } => format!("{code} · {}", error_text(blocked.error)),
                 other => error_text(other),
             };
             Text::one(look, widgets::code(look), &raw, palette.secondary)
                 .wrap(column.available_width())
                 .layout(column.ctx())
                 .label(column);
+            // What the way on asks of the user, where the button alone
+            // does not say it.
+            if !then.painted.is_empty() {
+                let role = widgets::secondary(look);
+                Text::one(look, role, &then.painted, palette.secondary)
+                    .wrap(column.available_width())
+                    .layout(column.ctx())
+                    .label(column);
+            }
+            let Some(offer) = blocked.offer() else {
+                return;
+            };
+            let (name, key, _) = offer.names();
+            let painted = look.label(&gettext(locale, name));
+            let button = if look.terminal {
+                states::key_button(&painted, key, look)
+            } else {
+                states::button(&painted, look)
+            };
+            let height = states::button_height(look);
+            if button
+                .label(name)
+                .show(column, height, look, palette)
+                .clicked()
+            {
+                actions.push(blocked.action(offer, place.tab, place.sql.id));
+            }
         });
 }
 
@@ -1324,13 +1563,9 @@ fn results(ui: &mut Ui, run: &SqlRun, place: &Place<'_>, env: &Env<'_>, actions:
             let said = env.said(|words| cancel_text(run.cancel, words));
             note(ui, rect, &said, palette.warning, env);
         } else if let Some(index) = failed {
-            if let StatementOutcome::Error { error, .. } = &run.outcome.results[index].outcome
-                && format::refuses_writes(error, place.driver)
-            {
-                blocked(ui, rect, error, place.writable, env);
-                return;
-            }
-            // The statement's line, then what the database said of it.
+            // The statement's line, then what the database said of it. A
+            // write the database refused reads so here too: its card
+            // leads the Messages, which a failed statement opens.
             let named = Words {
                 locale,
                 lower: false,
````

- [ ] **Step 4: Run the tests to see them pass**

```bash
~/.cargo/bin/cargo test --locked -p tabletist --lib -- sql_results::tests format::tests run_again_sends
```

Expected: PASS, 98 passed and 0 failed.

- [ ] **Step 5: Run the four checks**

All four pass. Fix what does not before going on; do not weaken a lint to get there.

- [ ] **Step 6: Commit**

```bash
git add src/ui/sql_results.rs src/ui/format.rs src/model.rs src/app.rs src/ui/keys.rs src/testing.rs
git commit -S -m "Say why a refused write's run was read-only, and offer the way on" -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 7: The scenes, and what the documents say

**Files:**
- Modify: `src/shots.rs` (the `sql-blocked*` and `sql-write-*` scenes)
- Modify: `docs/superpowers/specs/2026-10-05-sql-editor-writes-design.md`, `2026-09-30-sql-editor-core-design.md`, `2026-10-03-value-editing-core-design.md`, `2026-09-27-tabletist-design.md`
- Modify: `README.md`

No behaviour changes here, so there is no test to write first. `src/shots.rs` is built only with `--features shots`, which the four checks do not turn on: it is type-checked on its own below.

The scenes are for review by eye, on the Bookshop demo data: the three cards (`sql-blocked`, `sql-blocked-run`, `sql-blocked-connection`), the badge's menu (`sql-write-menu`), a run that writes on its way with Run held back (`sql-write-running`), and four ends (`sql-write-committed`, `sql-write-rolled-back`, `sql-write-commit-failed`, `sql-write-partly`). The old `sql-blocked` scene forced the Results pane, where the card no longer is.

The documents are the ones the spec lists under "Documents this changes", for what this run builds: the SQL editor spec (its status, intent, "Run all", the guard, the SQLite cleanup, a new run in the same tab, the key, the toolbar, the footer, and the edge cases), the value editing spec (slice 6, the card, the fence's list, the promise), the main spec (success criterion 6, section 4.3, the keyboard table) and the README. The writes spec itself gains its status, what was settled about the pragmas, the fourth wording of the card, how the two runs of step 2 are cut, and a line for each thing this plan decided about the Messages and Results that the spec did not say. The settings spec's key table is the next run's, with `editor.sql_new_tab`. The crate documentation of `tabletist-db` already says what the crate does (step 1 wrote it), and the shortcuts table changed in task 5.

- [ ] **Step 1: Add the scenes**

````diff
diff --git a/src/shots.rs b/src/shots.rs
index c438f84..4a94985 100644
--- a/src/shots.rs
+++ b/src/shots.rs
@@ -701,29 +701,111 @@ fn shots() {
         workspace.object_tab_mut(id).unwrap().rows.started =
             std::time::Instant::now().checked_sub(Duration::from_millis(4200));
     });
-    // A write the editor refused: it only reads, on any connection.
+    // A write a database refused in a read-only run, as each of its
+    // cards: in a tab that is in Read-only, in a tab in Read-write whose
+    // run was taken for one of reads, and on a connection that opens
+    // read-only. The Messages lead with it.
     both("sql-blocked", |harness| {
-        let tab = sql_script(
-            harness,
-            "UPDATE book_images\n   SET kind = 'ebook'\n WHERE id = 2;",
-        );
+        let tab = sql_script(harness, UPDATE);
+        run_sql(harness, tab, true);
+        let refused = crate::testing::refused_write();
+        harness.answer_sql(Ok(crate::testing::script_outcome(vec![refused])), None);
+    });
+    both("sql-blocked-run", |harness| {
+        let tab = sql_script(harness, "SELECT setval('book_images_id_seq', 1);");
+        read_write(harness, tab);
         run_sql(harness, tab, true);
         let refused = tabletist_db::StatementOutcome::Error {
             error: tabletist_db::Error::Query {
                 code: Some("25006".into()),
-                message: "cannot execute UPDATE in a read-only transaction".into(),
+                message: "cannot execute setval() in a read-only transaction".into(),
                 detail: None,
                 hint: None,
             },
             position: None,
         };
         harness.answer_sql(Ok(crate::testing::script_outcome(vec![refused])), None);
-        let sql_tab = harness.app.workspace(tab).unwrap().active_tab.unwrap();
-        harness.app.apply(Action::SetResultPane {
-            tab,
-            sql_tab,
-            pane: crate::model::ResultPane::Results,
-        });
+    });
+    both("sql-blocked-connection", |harness| {
+        let tab = sql_script(harness, UPDATE);
+        let workspace = harness.app.workspace_mut(tab).unwrap();
+        workspace.access = tabletist_db::Access::ReadOnly;
+        run_sql(harness, tab, true);
+        let refused = crate::testing::refused_write();
+        harness.answer_sql(Ok(crate::testing::script_outcome(vec![refused])), None);
+    });
+    // A tab switched to Read-write: the badge's menu, a run that writes on
+    // its way (Run and Run all wait for it), and how such a run ends.
+    both("sql-write-menu", |harness| {
+        let tab = sql_editor(harness);
+        read_write(harness, tab);
+        harness.click("Transaction");
+    });
+    both("sql-write-running", |harness| {
+        writing(harness, WRITES);
+    });
+    both("sql-write-committed", |harness| {
+        let tab = writing(harness, WRITES);
+        let done = vec![
+            crate::testing::done_outcome(Some(12)),
+            crate::testing::done_outcome(Some(3)),
+        ];
+        let end = tabletist_db::ScriptEnd::Committed;
+        harness.answer_sql(Ok(crate::testing::write_outcome(done, end)), None);
+        show_messages(harness, tab);
+    });
+    both("sql-write-rolled-back", |harness| {
+        writing(harness, WRITES);
+        let message = "update or delete on table \"book_images\" violates foreign key \
+                       constraint \"book_covers_image_id_fkey\" on table \"book_covers\"";
+        let outcomes = vec![
+            crate::testing::done_outcome(Some(12)),
+            crate::testing::error_outcome(message, None),
+        ];
+        let end = tabletist_db::ScriptEnd::RolledBack;
+        harness.answer_sql(Ok(crate::testing::write_outcome(outcomes, end)), None);
+    });
+    both("sql-write-commit-failed", |harness| {
+        writing(harness, WRITES);
+        let done = vec![
+            crate::testing::done_outcome(Some(12)),
+            crate::testing::done_outcome(Some(3)),
+        ];
+        let end = tabletist_db::ScriptEnd::CommitFailed {
+            error: tabletist_db::Error::Query {
+                code: Some("23503".into()),
+                message: "update or delete on table \"book_images\" violates foreign key \
+                          constraint \"book_covers_image_id_fkey\" on table \"book_covers\""
+                    .into(),
+                detail: Some("Key (id)=(7) is still referenced from table \"book_covers\".".into()),
+                hint: None,
+            },
+            committed: 0,
+        };
+        harness.answer_sql(Ok(crate::testing::write_outcome(done, end)), None);
+    });
+    // MySQL commits at a CREATE TABLE: what came before it is written
+    // though the script failed after it.
+    both("sql-write-partly", |harness| {
+        let script = "UPDATE book_images SET kind = 'ebook' WHERE kind = 'epub';\n\
+                      CREATE TABLE book_formats (kind varchar(16) PRIMARY KEY);\n\
+                      INSERT INTO book_formats SELECT DISTINCT kind FROM book_images;\n\
+                      INSERT INTO book_formats VALUES ('ebook');";
+        let tab = sql_script(harness, script);
+        harness.app.workspace_mut(tab).unwrap().driver = Driver::MySql;
+        read_write(harness, tab);
+        run_sql(harness, tab, true);
+        let outcomes = vec![
+            crate::testing::done_outcome(Some(12)),
+            crate::testing::done_outcome(None),
+            crate::testing::done_outcome(Some(2)),
+            crate::testing::error_outcome(
+                "Duplicate entry 'ebook' for key 'book_formats.PRIMARY'",
+                None,
+            ),
+        ];
+        let end = tabletist_db::ScriptEnd::Partly { committed: 2 };
+        harness.answer_sql(Ok(crate::testing::write_outcome(outcomes, end)), None);
     });
     both("state-no-schemas", |harness| {
         let tab = workspace(harness);
@@ -975,6 +1057,41 @@ fn run_sql(harness: &mut Harness, tab: ConnTabId, all: bool) {
     harness.app.apply(Action::RunSql { tab, sql_tab, all });
 }
 
+/// One statement that changes rows.
+const UPDATE: &str = "UPDATE book_images\n   SET kind = 'ebook'\n WHERE id = 2;";
+
+/// The script the scenes of a run that writes show.
+const WRITES: &str = "UPDATE book_images\n   SET kind = 'ebook'\n WHERE kind = 'epub';\n\n\
+                      DELETE FROM book_images\n WHERE book_id IS NULL;";
+
+/// Switches the SQL editor on screen to Read-write, as its badge does.
+fn read_write(harness: &mut Harness, tab: ConnTabId) {
+    let sql_tab = harness.app.workspace(tab).unwrap().active_tab.unwrap();
+    harness.app.apply(Action::SetSqlMode {
+        tab,
+        sql_tab,
+        mode: crate::model::RunMode::ReadWrite,
+    });
+}
+
+/// A SQL editor in Read-write holding `script`, all of it sent as its run.
+fn writing(harness: &mut Harness, script: &str) -> ConnTabId {
+    let tab = sql_script(harness, script);
+    read_write(harness, tab);
+    run_sql(harness, tab, true);
+    tab
+}
+
+/// Shows the Messages of the SQL editor on screen.
+fn show_messages(harness: &mut Harness, tab: ConnTabId) {
+    let sql_tab = harness.app.workspace(tab).unwrap().active_tab.unwrap();
+    harness.app.apply(Action::SetResultPane {
+        tab,
+        sql_tab,
+        pane: crate::model::ResultPane::Messages,
+    });
+}
+
 /// The script the SQL scenes show: two statements over `book_images`.
 const SCRIPT: &str = "-- Images of each kind\n\
                       SELECT kind, count(*) AS images\n  \
````

- [ ] **Step 2: Type-check the scenes**

```bash
~/.cargo/bin/cargo clippy --locked --features shots --lib --tests -- -D warnings
```

Expected: `Finished`, with no warning.

- [ ] **Step 3: Update the documents**

````diff
diff --git a/README.md b/README.md
index f9ebf37..699102d 100644
--- a/README.md
+++ b/README.md
@@ -39,7 +39,8 @@ Linux (Omarchy and Hyprland first), macOS and Windows.
   connection tab (reconnecting in that tab reuses them).
 - A connection opens read-only when its "Open read-only" box says so, which
   is the default for production. A writable one opens a read-write session,
-  in which browsing, a raw WHERE and the SQL editor still only read.
+  in which browsing, a raw WHERE and a SQL editor left in Read-only still
+  only read.
 - On a writable connection a table's values are edited in its grid: changes
   stay pending until Save writes them in one transaction, which never
   overwrites a row someone else changed: a save that finds one writes
@@ -62,7 +63,12 @@ Linux (Omarchy and Hyprland first), macOS and Windows.
   (Cmd/Ctrl+Return) or the whole script, with a row limit and a timeout, and
   read a result row in full in the row panel.
   Every run happens in a read-only transaction that is rolled back, and
-  statements that would leave it are refused. Format (Cmd/Ctrl+Shift+F)
+  statements that would leave it are refused. On a writable connection
+  that is not production's a tab can be switched to Read-write (its
+  toolbar's badge, or Cmd/Ctrl+Shift+M): a run that changes data is then
+  one transaction, committed when every statement succeeded and rolled
+  back on the first error, cancel or timeout, and the Messages say which.
+  Format (Cmd/Ctrl+Shift+F)
   lays queries out in river style and uppercases reserved words, in the
   selection's statements or the whole script. Keywords, schemas, tables,
   views and the columns of a statement's tables are completed while typing
diff --git a/docs/superpowers/specs/2026-09-27-tabletist-design.md b/docs/superpowers/specs/2026-09-27-tabletist-design.md
index 2119110..9ea8fb7 100644
--- a/docs/superpowers/specs/2026-09-27-tabletist-design.md
+++ b/docs/superpowers/specs/2026-09-27-tabletist-design.md
@@ -33,8 +33,13 @@ PostgreSQL, MySQL, SQLite. No others in v1.
    existing rows in a table's grid, and only Save writes: every pending
    change of the tab in one transaction, which never overwrites a row
    someone else changed unless the user, asked about that row, chooses to,
-   and on production only after its statements were shown and confirmed. Browsing, a raw WHERE and the SQL editor still
-   cannot write (see `2026-10-03-value-editing-core-design.md`).
+   and on production only after its statements were shown and confirmed.
+   A SQL tab the user switched to Read-write writes too: a run that
+   changes data is one transaction, committed when every statement
+   succeeded, and not yet on a production connection. Browsing, a raw
+   WHERE and every other run still cannot write (see
+   `2026-10-03-value-editing-core-design.md` and
+   `2026-10-05-sql-editor-writes-design.md`).
 7. The UI thread never blocks on the database, network, or disk; any running
    query can be cancelled.
 8. The app follows the Omarchy theme live, and the OS light/dark setting
@@ -278,16 +283,20 @@ A writable session, for a connection whose "Open read-only" box is off, is
 read-write: PostgreSQL keeps the server's default, MySQL gets `SET SESSION
 TRANSACTION READ WRITE`, and SQLite is opened `SQLITE_OPEN_READ_WRITE`
 (never creating the file) with `PRAGMA query_only = ON` as its standing
-state. Browsing, a raw WHERE and the SQL editor still cannot write there:
-row fetches and counts run in read-only transactions on PostgreSQL and
-MySQL, and on SQLite under `query_only` with an authorizer fencing the raw
-WHERE; a script runs behind the SQL editor's guard, which makes a MySQL
-session read-only for the run. The crate writes in exactly one place,
-`Connection::write`, which a read-only session refuses. The app calls it
-from one place as well: the Save of a table tab's pending changes
-(`Command::Write`, sent by the reducer in `src/app/editing.rs`). The detail
-is in `2026-10-03-value-editing-core-design.md`, "Sessions", "Editing in
-the grid" and "Saving".
+state. Browsing, a raw WHERE and a read-only run of the SQL editor still
+cannot write there: row fetches and counts run in read-only transactions
+on PostgreSQL and MySQL, and on SQLite under `query_only` with an
+authorizer fencing the raw WHERE; a read-only script runs behind the SQL
+editor's guard, which makes a MySQL session read-only for the run. The
+crate writes in exactly two places, both of which a read-only session
+refuses: `Connection::write`, and `Connection::run_script` in
+`ScriptMode::Write`. The app calls each from one place: the Save of a
+table tab's pending changes (`Command::Write`, sent by the reducer in
+`src/app/editing.rs`), and the run of a SQL tab in Read-write that holds
+a write (`Command::RunSql` with that mode, sent by `App::send_run`). The
+detail is in `2026-10-03-value-editing-core-design.md`, "Sessions",
+"Editing in the grid" and "Saving", and in
+`2026-10-05-sql-editor-writes-design.md`.
 
 Every session also fixes how values print, since a save sends back what a
 page showed: PostgreSQL sets `extra_float_digits = 3` and `DateStyle =
@@ -607,6 +616,7 @@ read-only table (structure data is small; the data grid is not needed).
 |---|---|
 | Cmd/Ctrl+O | Connections (the picker) |
 | Cmd/Ctrl+T | New SQL editor (added after v1, see `2026-09-30-sql-editor-core-design.md`) |
+| Cmd/Ctrl+Shift+M | Read-only or read-write runs in the SQL editor (see `2026-10-05-sql-editor-writes-design.md`) |
 | Cmd/Ctrl+Shift+W | Close connection |
 | Cmd/Ctrl+1..9, Ctrl+Tab, Ctrl+Shift+Tab | Switch connection (the digits count the open ones) |
 | Cmd/Ctrl+N | New connection |
diff --git a/docs/superpowers/specs/2026-09-30-sql-editor-core-design.md b/docs/superpowers/specs/2026-09-30-sql-editor-core-design.md
index b158348..b6e062c 100644
--- a/docs/superpowers/specs/2026-09-30-sql-editor-core-design.md
+++ b/docs/superpowers/specs/2026-09-30-sql-editor-core-design.md
@@ -3,7 +3,11 @@
 Date: 2026-09-30. Status: implemented. This spec describes the slice as
 built; where the code and the first draft differed, the text follows the
 code. Since 2026-10-01 the row panel shows a selected result row (see Results
-and Shortcuts on a SQL tab).
+and Shortcuts on a SQL tab). Since 2026-10-05 a SQL tab of a writable
+connection can be switched to Read-write
+(`2026-10-05-sql-editor-writes-design.md`): what this spec says of every
+run holds for the read-only run, which every tab starts with and every run
+of reads still is.
 
 ## Intent
 
@@ -14,7 +18,9 @@ table data: every run happens in a read-only transaction that is rolled back,
 and scripts that would leave that transaction are refused before anything
 runs. That holds on a writable connection too, whose session is read-write
 between runs: the editor is fenced there as on a read-only one (see
-`2026-10-03-value-editing-core-design.md`, "Writable connections").
+`2026-10-03-value-editing-core-design.md`, "Writable connections"), until
+the user switches the tab to Read-write. Then a run that holds a write is
+one read-write transaction, committed when every statement succeeded.
 
 Success: on each driver, a user opens a SQL tab, runs a query, sees its rows
 with column types, sees the database's error when a statement fails, and can
@@ -50,7 +56,7 @@ each with its own spec, plan and pull request:
 |---|---|
 | Scope | Core editor only; slices 2 to 5 follow separately. |
 | Query text | In memory only. Closing a SQL tab never asks; no unsaved dot. |
-| Run all | One read-only transaction, statements in order, stop at the first error. Results shows the last statement that returned rows; Messages lists every statement. |
+| Run all | One read-only transaction, statements in order, stop at the first error. Results shows the last statement that returned rows; Messages lists every statement. In a tab switched to Read-write a run that holds a write is one read-write transaction instead. |
 | Read-only guard | Refuse transaction and session-mode statements before running; on PostgreSQL take the snapshot first; check the transaction is still read-only before rolling back; reset the MySQL session after every script. |
 | Editor widget | egui `TextEdit` in code mode with our own layouter and a gutter; the SQL tokenizer lives in `tabletist-db`. |
 | Tab model | `Workspace.objects` becomes `tabs: Vec<Tab>`, `enum Tab { Object(Box<ObjectTab>), Sql(Box<SqlTab>) }`. |
@@ -130,7 +136,10 @@ a string is what the splitter and the guard treat as one.
 
 ### The read-only guard
 
-Four layers, all in `tabletist-db`, so no caller can skip them.
+Four layers, all in `tabletist-db`, so no caller can skip them. They guard
+the read-only run (`ScriptMode::ReadOnly`), one of the two modes
+`run_script` has; the run that writes keeps the refusal list and its own
+checks (`2026-10-05-sql-editor-writes-design.md`).
 
 1. **Refusal before running.** `run_script` checks every statement first and
    runs nothing if any is refused. The whole script fails with
@@ -308,7 +317,10 @@ Cleanup after every script, on every path:
   `PRAGMA trusted_schema = OFF`, `PRAGMA case_sensitive_like = OFF`,
   `PRAGMA full_column_names = OFF` and `PRAGMA short_column_names = ON`, so
   a script cannot leave the session writable, change how the filters
-  compare, or rename the columns of the pages and saves after it. A
+  compare, or rename the columns of the pages and saves after it; and
+  `ignore_check_constraints`, `recursive_triggers` and
+  `legacy_alter_table` off, so a flag a script set for its own run does
+  not change what a later run or save keeps. A
   script's other `PRAGMA`s and its `ATTACH`es last for the session (see
   Intent); those a write would feel are put back by the save itself (the
   value-editing spec, "Saving").
@@ -476,7 +488,9 @@ has SQL tabs; kept on the workspace as a `Fetch<String>`.
   Cancel does, records `CancelReason::Timeout` when it was the first to stop
   it, and waits for `run_script` to return; the interrupted statement then
   reports `Cancelled`, and the transaction is rolled back.
-- A new run in the same tab cancels the one still running (reason `User`).
+- A new run in the same tab cancels the one still running (reason `User`),
+  unless that one was sent to write: then Run and Run all do nothing until
+  it ends, and Cancel stops it.
   Results for a closed tab or a replaced run are dropped by `RequestId`.
   A run is sent only on a connected session: while the workspace is
   connecting or disconnected, Run does nothing.
@@ -596,6 +610,8 @@ tab's result grid as on a table's.
 
 - `Mod+Return` runs the statement at the cursor, `Mod+Shift+Return` runs
   all, also while typing. `Mod+.` cancels.
+- `Mod+Shift+M` switches the tab between Read-only and Read-write, on a
+  connection that takes writes.
 - `Mod+W`, `Mod+Shift+[ / ]`, `Mod+1..9`, `Mod+B`, `Mod+P`, `?` work as on
   any tab.
 - `Mod+R`, `Mod+F` and `Mod+Alt+Left / Right` do nothing on a SQL tab (no
@@ -619,10 +635,14 @@ tab's result grid as on a table's.
 
 - macOS: Run (with `Cmd+Return`), Run all (`Shift+Cmd+Return`); on the right
   a "Read-only transaction" badge whose tooltip explains it, then
-  "Limit 1,000" and "Timeout 30 s" menus.
+  "Limit 1,000" and "Timeout 30 s" menus. On a connection that takes
+  writes the badge is a menu too, with "Read-only transaction" and
+  "Read-write transaction", and in Read-write it reads in the warning
+  tone.
 - Omarchy: the tab title, a muted `read-only transaction · limit 1000 ·
   timeout 30s` whose limit and timeout parts open the same menus, then
-  `run ctrl+enter` and `run all ctrl+shift+enter`.
+  `run ctrl+enter` and `run all ctrl+shift+enter`. On a connection that
+  takes writes the first part opens the badge's menu.
 - Where the toolbar is too narrow, pieces give way in this order: the run
   buttons' keys, the read-only note, on Omarchy the tab title, then the
   menus' words (leaving "1,000" and "30 s"), on macOS the menus' chevrons,
@@ -690,11 +710,16 @@ tab's result grid as on a table's.
   refusal, on a timeout, and on a cancel that left no rows to show; a run
   the user cancelled after rows came back stays on Results. A statement's
   error marks its line in the gutter in the error colour.
+- A write a database refused in a read-only run leads the Messages as a
+  card that says why the run was read-only and offers the way on; what a
+  run sent to write adds to the Messages, Results and the footer is in
+  `2026-10-05-sql-editor-writes-design.md`.
 
 ### Footer
 
 - macOS: `5 rows · 14 ms`, `Read-only transaction · rolled back`, then on
-  the right `Ln 12, Col 21` and the server version.
+  the right `Ln 12, Col 21` and the server version. After a run sent to
+  write: `12 rows affected · 14 ms`, `Read-write transaction · committed`.
 - Omarchy: a mode line of key hints (run, run all, cancel, leave editor,
   tables), then `ln 12:21 · 5 rows · 14 ms · rolled back` on the right. The
   vim mode indicator waits for slice 5.
@@ -706,10 +731,10 @@ tab's result grid as on a table's.
   is "Line 12, col 15:" and the gutter marks that line. The code, detail
   and hint a database gave follow on lines of their own. What a database
   said is cut at 2,000 characters where it is shown.
-- Writes fail through the database's own read-only error (PostgreSQL and
-  MySQL read-only transactions; SQLite's `query_only`, with the read-only
-  open behind it on a read-only connection); escaping the transaction is
-  refused by the guard.
+- In a read-only run writes fail through the database's own read-only
+  error (PostgreSQL and MySQL read-only transactions; SQLite's
+  `query_only`, with the read-only open behind it on a read-only
+  connection); escaping the transaction is refused by the guard.
 - A timeout reads "Cancelled after 30 s (timeout)", a user cancel
   "Cancelled". A `Cancelled` outcome without a reason (a user-set
   `statement_timeout`, say) also reads "Cancelled". The cancelled
@@ -721,7 +746,9 @@ tab's result grid as on a table's.
 - Session settings a script changes (`SET search_path`, `SET time_zone`)
   last only for that run: PostgreSQL rolls them back, MySQL resets the
   session. SQLite puts `query_only`, `trusted_schema`, `case_sensitive_like`,
-  the column-name pragmas and the busy timeout back after every run; its other `PRAGMA`s and its
+  the column-name pragmas, `ignore_check_constraints`, `recursive_triggers`,
+  `legacy_alter_table` and the busy timeout back after every run, and
+  denies a script `locking_mode` with a value; its other `PRAGMA`s and its
   `ATTACH`es last for the session.
 - A run that failed as a whole (a refusal, a lost session) shows its error
   and no older rows: the result of the run before it is dropped.
diff --git a/docs/superpowers/specs/2026-10-03-value-editing-core-design.md b/docs/superpowers/specs/2026-10-03-value-editing-core-design.md
index cf5d489..a02ac43 100644
--- a/docs/superpowers/specs/2026-10-03-value-editing-core-design.md
+++ b/docs/superpowers/specs/2026-10-03-value-editing-core-design.md
@@ -44,7 +44,8 @@ sub-projects, each with its own spec, plan and pull request:
    set, `$EDITOR`.
 5. Rows: add, duplicate, delete (the "Editing a row" artboards).
 6. Writes from the SQL editor, and allowing writes for one tab of a
-   read-only connection.
+   read-only connection. The first is
+   `2026-10-05-sql-editor-writes-design.md`, which rejects the second.
 
 ## Decisions
 
@@ -110,7 +111,10 @@ pending changes never outlive their page.
   connection**, the per-tab switch) belongs to slice 6: until the SQL
   editor can write, turning the box off would not make the statement run,
   and the card must not say it would. From step 3 on, the card adds "Edit
-  values in a table's grid." on a writable connection.
+  values in a table's grid." on a writable connection. Slice 6 replaced
+  this card with three, at the head of the Messages, each saying why its
+  run was read-only (`2026-10-05-sql-editor-writes-design.md`, "The
+  refused write").
 
 ### Sessions (`tabletist-db`)
 
@@ -195,8 +199,9 @@ pending changes never outlive their page.
     session's settings, the catalog, later the save): everything is
     allowed;
   - a script's statement: transaction and savepoint statements are
-    denied, and so are `query_only` and `writable_schema` with a value
-    and `wal_checkpoint` in any form. Writes are left to `query_only`,
+    denied, and so are `query_only`, `writable_schema` and `locking_mode`
+    with a value and `wal_checkpoint` in any form. Writes are left to
+    `query_only`,
     whose error the refused-write card recognises. `ATTACH` and other
     pragmas stay, as the SQL editor spec allows them;
   - a table's page or count, which holds the raw WHERE: a pragma with a
@@ -221,8 +226,9 @@ pending changes never outlive their page.
   authorized", and the checks hold if the authorizer is ever wrong.
 
 The promise, restated: on a read-only connection no action in the app can
-modify data. On a writable connection only Save can; browsing, a raw WHERE
-and the SQL editor still cannot.
+modify data. On a writable connection only Save can, and since slice 6 a
+run in a SQL tab the user switched to Read-write; browsing, a raw WHERE
+and every other run still cannot.
 
 ## What can be edited
 
diff --git a/docs/superpowers/specs/2026-10-05-sql-editor-writes-design.md b/docs/superpowers/specs/2026-10-05-sql-editor-writes-design.md
index 9665f30..1c685cc 100644
--- a/docs/superpowers/specs/2026-10-05-sql-editor-writes-design.md
+++ b/docs/superpowers/specs/2026-10-05-sql-editor-writes-design.md
@@ -1,8 +1,12 @@
 # Writes from the SQL editor
 
 Date: 2026-10-05. Status: step 1 (the run) is built, see
-`docs/superpowers/plans/2026-10-05-sql-editor-writes-run.md`; steps 2 and 3
-are not yet planned. The PostgreSQL and MySQL runs were built with no
+`docs/superpowers/plans/2026-10-05-sql-editor-writes-run.md`. Step 2 (the
+tab) is built in two runs. The first is
+`docs/superpowers/plans/2026-10-05-sql-editor-writes-tab.md`: a tab that
+writes, on every connection but a production one. The second, not yet
+planned, is the production confirmation and `editor.sql_new_tab`. Step 3
+is not yet planned. The PostgreSQL and MySQL runs were built with no
 server at hand: their tests have been compiled, and are first run by CI.
 
 ## Intent
@@ -333,9 +337,10 @@ statement outside a transaction.
    statement runs. A `BEGIN` that fails as busy is the run's error with
    SQLite's message; the session stays open.
 2. Each statement runs behind the script fence (`Fence::Script`), which
-   denies what it denies today: transaction and savepoint statements,
-   `query_only` and `writable_schema` with a value, `wal_checkpoint`. What
-   lets the statement write is `query_only` being off.
+   denies what it denies a read-only run: transaction and savepoint
+   statements, `query_only`, `writable_schema` and `locking_mode` with a
+   value, `wal_checkpoint`. What lets the statement write is `query_only`
+   being off.
 3. Before every statement the driver asks whether its transaction is still
    open. One that is gone ends the run with `Error::LeftTransaction`.
 4. `COMMIT` after the last statement, `ROLLBACK` after an error or a stop.
@@ -473,24 +478,39 @@ A read-only run reads as today. For a read-write run:
     then its text. Nothing then says "Nothing was written".
   - With `broken`: the end's own line, then "The session could not be put
     back and was closed.", and the reconnect banner.
+  - A stop that came with every statement done has no statement's line to
+    say it: "Cancelled" (or the timeout's words) then stands before the
+    end's line.
+  - A commit that failed counts beside the Messages tab as a statement's
+    error does.
 - In a run that did not end `Committed`, a statement whose work was undone
   ends its line with "· rolled back", so no count reads as a change that
-  stayed. With a `rollback_warning` no line gets it.
+  stayed: a statement without a result set, and one that returned rows
+  and looks like a write (`INSERT ... RETURNING`). With a
+  `rollback_warning` no line gets it.
 - Results shows the last statement that returned rows, as today. A
   statement without a result set shows "Statement ran · 12 rows affected"
   when it counted rows, else today's "Statement ran · no rows returned". A
   statement counts rows where it does today (`WITH ... UPDATE` and `CREATE
-  TABLE ... AS` do not).
+  TABLE ... AS` do not). After a commit that failed, with no rows to show,
+  Results says what the Messages say of it ("The commit failed. Nothing
+  was written."), never that a statement ran. After a run that MySQL
+  committed by itself under a stop that came too late, Results says that
+  the statement ran, not only "Cancelled": the run is written.
 - Messages opens by itself as today, and also when a read-write run ends
   `Partly`, `CommitFailed`, with a `rollback_warning` or `broken`.
 - Footer, macOS and Windows: the shown statement's `12 rows affected ·
   14 ms`, as it shows a result's rows today, then `Read-write transaction ·
   committed`, `· rolled back`, `· partly committed`, `· commit failed`,
-  or, with a `rollback_warning`, `· not fully rolled back`. Omarchy:
+  or, with a `rollback_warning`, `· not fully rolled back`. A commit that
+  failed after MySQL had committed part by itself reads `· partly
+  committed`, which is what it left. Omarchy:
   `ln 3:1 · 12 rows affected · 14 ms · committed`.
 - A connection lost during a read-write run shows the reconnect banner and
   "The connection was lost during a read-write run. Some or all of it may
-  be written."
+  be written." A session closed because the script ended its own
+  transaction (`Error::LeftTransaction`) says that itself, and under it
+  "Some or all of the run may be written."
 
 ### The refused write
 
@@ -509,10 +529,22 @@ read-only:
 | Writable connection, tab in Read-only | "This tab runs read-only". "Every run here is a read-only transaction, so PostgreSQL refused the UPDATE. Nothing changed." | **Allow writes in this tab** (Omarchy `w`) |
 | Writable connection, tab in Read-write, run taken for a read | "This run was read-only". "Its statements looked like reads, so they ran in a read-only transaction and PostgreSQL refused this one. Nothing changed." | **Run in a read-write transaction** (Omarchy `w`) |
 
-- "Allow writes in this tab" sets the mode and runs nothing.
+- "Allow writes in this tab" sets the mode and runs nothing. The card
+  stays, for a run that is now one of a tab in Read-write: "This run was
+  read-only", "It was sent before this tab could write, so PostgreSQL
+  refused the UPDATE. Nothing changed.", and **Run in a read-write
+  transaction**. Its statements held a write, so "looked like reads"
+  would not be true of it.
 - Edit connection opens the connection dialog on the workspace's saved
   connection (`Action::EditConnection`).
 - The Omarchy keys work while the editor does not have the keyboard.
+- Under the card stand the database's own words, then, for a read-only
+  connection, the line "To write, turn off Open read-only ...", then the
+  action. The terminal look writes the card in lower case throughout, the
+  database's name and the statement's verb too, and names a read-only
+  connection by its tag, as its artboard does: "PROD blocks writes, so
+  postgresql refused the update. nothing changed." It writes the lines of
+  a run's end the same way.
 - A statement the guard refused (`Error::Refused`) keeps its own sentence
   and is not one of these cards any more: the refusal list is about the
   transaction, not about writing.
@@ -585,8 +617,13 @@ cannot.
   says. Some of them change what a later run that writes keeps
   (`ignore_check_constraints`, `recursive_triggers`, `legacy_alter_table`,
   `locking_mode`), and a script may set them in a read-only run. Step 2
-  settles them, by putting them back after every run or by denying them
-  to a script, before any tab can write.
+  settled them before any tab could write. The first three are put back
+  after every run, with the other connect-time settings: a script may set
+  one for its own run, and no later run or save feels it. `locking_mode`
+  with a value is denied to a script, as `query_only` is, and fails with
+  SQLite's "not authorized": in a rollback journal an exclusive lock is
+  let go only by the session's next read of the file, not by setting the
+  mode back, and inside the one transaction of a run it gains nothing.
 - Run on an empty or comment-only editor does nothing, and neither does
   Run while the session is connecting or disconnected.
 - A tab switched to Read-write whose runs are all reads never opens a
@@ -613,7 +650,14 @@ Each step ends compiling, tested and shippable, and gets its own plan run:
    It also settles the SQLite pragmas that outlive a run (see "Errors and
    edge cases"), since it is the step that lets a tab write.
    It reuses the production sheet and the PROD box of value editing's step
-   3, so it is planned once that has landed. Until step 3 of this spec,
+   3, so it is planned once that has landed. It is built in two runs.
+   The first leaves out the production confirmation and
+   `editor.sql_new_tab`, and so lets no tab of a production connection
+   write: there the badge's menu shows "Read-write transaction" disabled,
+   the key does nothing, and the menu, the badge's tooltip and the card of
+   a tab in Read-only say "Read-write runs on a production connection are
+   not available yet." in place of the way on. The second run builds the
+   confirmation and takes that sentence out. Until step 3 of this spec,
    closing a tab or a connection cancels a read-write run in flight
    without asking, as it cancels any run (rolled back unless its commit
    was already sent), and table tabs show what they loaded until they are
````

- [ ] **Step 4: Check the documents**

```bash
git diff -U0 -- README.md docs/superpowers/specs | grep '^+' | grep -c "$(printf '\xe2\x80\x94')"
```

Expected: `0` (no em dash in a line this task wrote). Read each changed paragraph once against the code: a document that says what the code does not do is a fault of this task.

- [ ] **Step 5: Run the four checks**

All four pass.

- [ ] **Step 6: Render the scenes, where a GPU or a software renderer is at hand**

```bash
~/.cargo/bin/cargo test --locked --features shots --lib shots::shots -- --ignored --exact
```

Expected: `1 passed`, and `target/shots/sql-blocked*.png` and `target/shots/sql-write-*.png` written (each look, light and dark). They are for looking at, here and by the user: no test reads them, they are not committed (`*.png` is ignored), and they are never pushed. If nothing can render, say so in the report and leave it to the user.

- [ ] **Step 7: Commit**

```bash
git add src/shots.rs README.md docs/superpowers/specs
git commit -S -m "Add the scenes of a tab that writes and say what the editor does now" -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```


---

## By hand, for the user

These need a window, a real server or eyes on the design, none of which the run that writes this code has:

1. **The real window, in each look.** The badge's menu opens under the badge and closes on a pick; its tooltip; the pill's warning tint and pencil in Read-write, light and dark; Run and Run all drawn disabled while a run that writes is in flight.
2. **`Cmd+Shift+M` on macOS** reaches the app (no menu item of the system takes it), and `Ctrl+Shift+M` on Windows and Linux.
3. **A real database.** On the Bookshop demo (SQLite, a throwaway file): switch a tab to Read-write, run an `UPDATE`, read "Committed", press Refresh on the table's tab and see the new value (a table tab does not load again by itself until step 3). Run two statements of which the second fails: "Rolled back. Nothing was written.", and the table unchanged. Then the same on PostgreSQL and MySQL where one is at hand; on MySQL a script with a `CREATE TABLE` in the middle and a failing last statement reads "Lines 1 to 2 are written".
4. **The cards**: each of the three on a real refusal, and `e` and `w` on Omarchy after `Esc` has left the editor.
5. **Against the design**, with the scenes of task 7: the badge as the tab's switch, the "Write blocked" card. The canvas has no artboard for a run that writes, so the end's lines and the footer are the spec's wording, not a drawing's.

## What the next run will find here

- `Workspace::sql_writes` with its `NoWrites::Unconfirmed` arm, and `UNCONFIRMED` in `src/ui/sql_editor.rs`: the confirmation replaces both, and the three tests that pin them (`the_badge_and_the_key_do_nothing_where_an_editor_cannot_write`, `on_production_the_read_write_choice_is_shown_and_cannot_be_picked`, `on_production_the_card_says_why_the_tab_cannot_write_and_offers_nothing`) change with it.
- `App::send_run(tab, id, statements, mode)`, the one place a run is sent: a held run is sent through it once it is confirmed, with the statements that were shown.
- `App::run_sql` and `App::run_sql_again`, the two places that decide a run is read-write: both hold the run and ask where the workspace's environment confirms writes.
- `SqlTab::new`, which starts every tab in `RunMode::ReadOnly`: `editor.sql_new_tab` reaches it through `Workspace::push_sql_tab`, as `sql_limit` does.
- The badge draws no production tone yet.

## After the plan ran

The seven tasks were built as written, inline: each task's tests failed alone as its step 2 says, the tree after each equals the plan's code, and the four checks passed after each. `main` moved by twelve commits while the plan was written (pull request #82, the database adapters' contract and `tests/engines.rs`), so the branch was rebased onto it; the rebase had no conflict, and the four checks pass on every commit of the rebased branch. Nothing in this run compares `Driver` or `Dialect` with `==` or `matches!`, which `tests/engines.rs` now looks for.

A review of the finished branch found three faults in the card of a refused write, fixed in one commit after task 7 ("Have the refused write's card answer only a press that read it, and offer only what it can do"), each with a test that failed first:

- **A held key, or a double-click, ran the statements.** "Allow writes in this tab" gives way to "Run in a read-write transaction" in the same place and under the same letter `w`. The repeats of a held `w`, or the second click of a double-click, answered the new offer: the mode was switched and the run committed in one gesture, against "sets the mode and runs nothing". The letter is now taken with `consume_press` (`fresh` in `keys::letters`), and the button ignores a click that is a double-click's second.
- **The card's letter worked with the card off screen.** After a switch of database the editor gives way to the opening screen and the tab keeps its last run; `w` there could allow writes, or send the old run's statements to write on the database that came next. `card_key` now answers only while the workspace is opened, and "Run in a read-write transaction" is offered only on a connected session.
- **A read-only production connection was told to turn its box off.** That would not let a tab write in this run. Its card now says "Read-write runs on a production connection are not available yet." and has no button (`Why::Connection { unconfirmed }`). The next run takes this out with `NoWrites::Unconfirmed`.

In a second commit two tests of the badge read the tab's own mode where they read the effective one, which could not fail there (`on_a_read_only_connection_the_badge_is_a_note_and_the_key_does_nothing`, `on_production_the_read_write_choice_is_shown_and_cannot_be_picked`).

After the pull request was opened (#87), where CI passed on Linux, macOS and Windows and against the PostgreSQL and MySQL servers:

- **A choice that cannot be picked said why only under the pointer** (Copilot's review). The menu drew it as a row that takes no keyboard and gives a screen reader no reason. It is now what the app's disabled buttons are: it takes the keyboard, carries its reason as the node's description, and shows it while the keyboard is on it (`menu::Item::disabled`). The scene `sql-write-menu-production` shows it.
- **Rebased a second time**, onto pull requests #85 and #86. #86 replaced `widgets::popup_menu` and `MenuChoice`, which task 5 of this plan extends, with the shared `src/ui/menu.rs` (`menu::choices`, `menu::Choice`, `menu::Item`). So task 5's diff of `src/ui/widgets.rs` in this plan no longer applies as written: in the branch the badge opens `menu::choices`, `menu::Choice` carries `disabled`, and `menu::Item::disabled` draws the row muted, as the Components sheet's menu rule has it ("Disabled items stay visible"). The four checks pass on every commit of the rebased branch.

Noted and not acted on:

- "Some or all of it may be written" is also said of a run sent to write that was still queued when another request lost the session: it never started. The backend answers a queued script with the same error as a running one. It errs on the safe side.
- In a narrow editor the badge is the second thing to give way, in Read-write too, as the spec has it for the note. The key still switches the mode there, unseen. Whether the badge should stay while the tab is in Read-write is the user's to decide.
- Nothing tests that the backend closes a session after a run left it `broken`, beyond the helper that says why: no SQLite run can be made to leave one.
- After "Allow writes in this tab" the keyboard's place is not carried to the button that takes its place, in the looks with a pointer. That is also what keeps a held Enter from answering it.

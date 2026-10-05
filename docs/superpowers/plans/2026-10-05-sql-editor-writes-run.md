# SQL Editor Writes, Step 1: The Run Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** `tabletist-db` can run a SQL editor script in one read-write transaction that it commits when every statement succeeded, on all three drivers, and can tell a statement that writes from one that reads. Nothing in the app asks for it yet.

**Architecture:** `Connection::run_script` gains a `ScriptMode`. `ReadOnly` is the run that exists, untouched in what it does. `Write` is new: each driver opens a read-write transaction, keeps the guard that holds a script inside its transaction, commits or rolls back, says which in `ScriptOutcome::end`, and puts the session back as it connected. PostgreSQL and MySQL get the new run in a `script/write.rs` beside their `script.rs`; SQLite's is a branch of the functions it has. `sql::kind` and `sql::unbounded` are pure functions over the tokenizer. The backend passes `ScriptMode::ReadOnly`, so the app behaves exactly as before.

**Tech Stack:** Rust 1.98, `tokio-postgres`, `mysql_async` 0.37, `rusqlite` 0.37. Spec: `docs/superpowers/specs/2026-10-05-sql-editor-writes-design.md` (this plan is its step 1, "The run").

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

- Tasks 5 and 6 need the test servers. Without them their tests print "skipped" and prove nothing:

      docker compose up -d --build --wait postgres mysql
      export TABLETIST_TEST_PG_URL=postgres://tabletist:tabletist@localhost:55432/tabletist
      export TABLETIST_TEST_MYSQL_URL=mysql://tabletist:tabletist@localhost:53306/tabletist

- **What was run where this plan was written.** The plan's code comes from a draft that was built task by task, each task passing the four checks on Linux. The SQLite tests ran and pass. No PostgreSQL or MySQL server could be reached, so the tests of tasks 5 and 6 were **only compiled**, and the code they test has never met a server. Run them against the servers before anything else is believed. Where a test fails, first find out whether the test or the code is wrong: each of those tasks lists the facts about the database it rests on. If docker is not available to you either, do the tasks all the same, say in your report that their tests were only compiled, and leave them for CI, which runs all three suites.
- The diffs are against the tree as the task before left it. Save a block to a file and `git apply` it, or make the change by hand; where both a file's tests and its code change, the tests' block comes first and applies on its own. A line number in a hunk header is where the draft had it and may have moved.
- House rules that bite here: no em dashes anywhere; comments explain why, in the surrounding code's voice; `crates/tabletist-db` has no UI dependencies; never log SQL text; do not weaken a lint or delete a test to get green. This plan changes no existing test's assertions: existing tests only gain the new argument or the new field.
- No screen changes, so there is nothing to look at by hand. Nothing here is behind a `cfg`, so one platform's build is every platform's.
- `claude/grid-editing` (value editing, not merged) adds `Error::ReadOnly` too, with the same words and the same place in the enum. Whichever lands second keeps one copy. It also has files named `pg/write.rs`, `mysql/write.rs` and `sqlite/write.rs`: that is the grid's save. The script's run is in `pg/script/write.rs` and `mysql/script/write.rs` here, so the two do not meet.
- Commit after every task. Subjects are plain sentences, each ending with:

      Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>

## File map

| File | What changes |
|---|---|
| `crates/tabletist-db/src/sql.rs` | `StatementKind`, `kind`, `Verb`, `unbounded`; module docs |
| `crates/tabletist-db/src/script.rs` | `ScriptMode`, `ScriptEnd`, three fields on `ScriptOutcome`, `warnings` on `Done`, the sentences that follow the mode, `succeeded`, `put_back` |
| `crates/tabletist-db/src/error.rs` | `Refused` carries its mode; `ReadOnly`; `LeftTransaction` |
| `crates/tabletist-db/src/lib.rs` | `run_script(.., mode, ..)`; crate docs |
| `crates/tabletist-db/src/sqlite.rs` | the run that writes, as a branch of `script`, `statements` and a new `end_write` |
| `crates/tabletist-db/src/pg.rs` | `session_setup` |
| `crates/tabletist-db/src/pg/script.rs` | `mode` through `statements` and `run_statement`; no cursor in a run that writes |
| `crates/tabletist-db/src/pg/script/write.rs` | new: the run that writes on PostgreSQL |
| `crates/tabletist-db/src/mysql/script.rs` | the branch to `write::run`; a statement's warning count |
| `crates/tabletist-db/src/mysql/script/write.rs` | new: the run that writes on MySQL |
| `crates/tabletist-db/tests/{sqlite,postgres,mysql}.rs` | the new argument; the tests of the run that writes |
| `src/backend.rs` | passes `ScriptMode::ReadOnly` |
| `src/ui/sql_results.rs` | two patterns take `Done`'s new field |
| `src/{app,model,testing,shots}.rs`, `src/ui/format.rs` | test values gain the new fields |
| `docs/superpowers/specs/2026-10-05-sql-editor-writes-design.md` | its status |

## What the run that writes must hold, in every driver

Read this once; tasks 4 to 6 are three spellings of it.

1. **One transaction.** Begin, run each statement in order, stop at the first that fails or is cancelled.
2. **Inside its own transaction before every statement.** The refusal list (`sql::refusal`, unchanged) keeps `COMMIT` and the like out. Behind it each driver checks, before every statement, that the session is still in the transaction the run began. PostgreSQL and SQLite never commit by themselves, so a transaction that is gone means the script got past the list: `Error::LeftTransaction`, and the session is closed. MySQL commits by itself at DDL, so there a transaction that is gone is noted (so many statements are written) and a new one begun.
3. **Commit only a run that succeeded and was not stopped.** `ScriptOutcome::succeeded` is the test. The driver calls `stop.finish()` and only then reads the stop flag: a stop set before that line wins over the commit, and from that line on the backend sends no cancel. A cancel that was already on its way can still land on the `COMMIT`; each driver then finds out whether the transaction is still open (rolled back, the run was stopped) or gone (the end is not known, an `Err`).
4. **Say truthfully how it ended**, in `ScriptOutcome::end`. `Committed` only after a `COMMIT` that was answered without an error. `RolledBack` only when nothing of the run remains.
5. **Put the session back on every path**, as it connected: browsing and the next run share it. When that fails after the end is known, the outcome is still returned, with the failure in `broken` (`ScriptOutcome::put_back`), and the backend will close the session. When the end is not known (the connection was lost, a rollback failed), the run is an `Err`.

---

### Task 1: `sql::kind`

A statement is a `Read` when its first word is a query's and no unquoted word in it changes data. It errs toward `Write`. The two mistakes it can make are not alike: a write taken for a read runs in a read-only transaction and is refused by the database; a read taken for a write commits nothing. Step 2 of the spec leans on that.

**Files:**
- Modify: `crates/tabletist-db/src/sql.rs`

- [ ] **Step 1: Write the failing tests**

At the end of `mod tests` in `sql.rs`:

```diff
diff --git a/crates/tabletist-db/src/sql.rs b/crates/tabletist-db/src/sql.rs
index fec8aa6..2fc3a6c 100644
--- a/crates/tabletist-db/src/sql.rs
+++ b/crates/tabletist-db/src/sql.rs
@@ -2021,4 +2100,179 @@ mod tests {
             ]
         );
     }
+
+    const DIALECTS: [Dialect; 3] = [Dialect::Postgres, Dialect::MySql, Dialect::Sqlite];
+
+    #[test]
+    fn a_statement_that_only_reads_is_a_read() {
+        for dialect in DIALECTS {
+            for text in [
+                "SELECT * FROM books",
+                "select 1",
+                "-- a note\nSELECT 1",
+                "VALUES (1), (2)",
+                "TABLE books",
+                "WITH recent AS (SELECT 1) SELECT * FROM recent",
+                "(SELECT 1) UNION (SELECT 2)",
+                "SHOW search_path",
+                "EXPLAIN SELECT * FROM books",
+                "DESCRIBE books",
+                "DESC books",
+                // Nothing to run.
+                "",
+                "-- only a comment",
+            ] {
+                assert_eq!(
+                    kind(dialect, text),
+                    StatementKind::Read,
+                    "{dialect:?} {text}"
+                );
+            }
+        }
+        // A statement in SQLite only.
+        let pragma = "PRAGMA table_info(books)";
+        assert_eq!(kind(Dialect::Sqlite, pragma), StatementKind::Read);
+        assert_eq!(kind(Dialect::Postgres, pragma), StatementKind::Write);
+        assert_eq!(kind(Dialect::MySql, pragma), StatementKind::Write);
+    }
+
+    #[test]
+    fn everything_else_is_a_write() {
+        for dialect in DIALECTS {
+            for text in [
+                "INSERT INTO books VALUES (1)",
+                "update books set title = 'x'",
+                "DELETE FROM books",
+                "MERGE INTO books USING drafts ON books.id = drafts.id WHEN MATCHED THEN DELETE",
+                "REPLACE INTO books VALUES (1)",
+                "CREATE TABLE notes (id int)",
+                "DROP TABLE books",
+                "TRUNCATE books",
+                "CALL refill()",
+                "SET search_path = shop",
+                "VACUUM",
+                "ANALYZE books",
+                "GRANT SELECT ON books TO reader",
+                "/* first */ DELETE FROM books",
+            ] {
+                assert_eq!(
+                    kind(dialect, text),
+                    StatementKind::Write,
+                    "{dialect:?} {text}"
+                );
+            }
+        }
+    }
+
+    #[test]
+    fn a_query_that_holds_a_data_changing_word_is_a_write() {
+        for dialect in DIALECTS {
+            for text in [
+                "WITH gone AS (DELETE FROM books RETURNING *) SELECT * FROM gone",
+                "WITH s AS (SELECT 1) INSERT INTO books SELECT * FROM s",
+                "SELECT * FROM books FOR UPDATE",
+                // Erring toward a write: a bare name, a function.
+                "SELECT * FROM books WHERE update = 1",
+                "SELECT insert('abc', 1, 1, 'x')",
+                "select merge from books",
+            ] {
+                assert_eq!(
+                    kind(dialect, text),
+                    StatementKind::Write,
+                    "{dialect:?} {text}"
+                );
+            }
+        }
+    }
+
+    #[test]
+    fn data_changing_words_in_strings_comments_and_quoted_names_do_not_count() {
+        for dialect in DIALECTS {
+            for text in [
+                "SELECT 'DELETE FROM books'",
+                "SELECT 1 -- then UPDATE it",
+                "SELECT /* INSERT */ 1",
+                "SELECT replace(title, 'a', 'b') FROM books",
+                "SELECT deleted_at, updated_by FROM books",
+            ] {
+                assert_eq!(
+                    kind(dialect, text),
+                    StatementKind::Read,
+                    "{dialect:?} {text}"
+                );
+            }
+        }
+        for (dialect, text) in [
+            (Dialect::Postgres, "SELECT \"update\" FROM books"),
+            (
+                Dialect::Sqlite,
+                "SELECT \"update\", [delete], `insert` FROM books",
+            ),
+            (Dialect::MySql, "SELECT `delete` FROM books"),
+            (Dialect::MySql, "SELECT \"MERGE\""),
+            (Dialect::Postgres, "SELECT $$ MERGE INTO books $$"),
+        ] {
+            assert_eq!(
+                kind(dialect, text),
+                StatementKind::Read,
+                "{dialect:?} {text}"
+            );
+        }
+    }
+
+    #[test]
+    fn an_explain_is_a_write_only_when_it_runs_a_data_change() {
+        for dialect in DIALECTS {
+            for text in [
+                "EXPLAIN DELETE FROM books",
+                "EXPLAIN UPDATE books SET title = 'x'",
+                "explain insert into books values (1)",
+                "EXPLAIN ANALYZE SELECT * FROM books",
+                "EXPLAIN (ANALYZE, BUFFERS) SELECT 1",
+                "EXPLAIN QUERY PLAN DELETE FROM books",
+                "DESCRIBE UPDATE books SET title = 'x'",
+                "DESC DELETE FROM books",
+            ] {
+                assert_eq!(
+                    kind(dialect, text),
+                    StatementKind::Read,
+                    "{dialect:?} {text}"
+                );
+            }
+            for text in [
+                "EXPLAIN ANALYZE DELETE FROM books",
+                "EXPLAIN (ANALYZE) DELETE FROM books",
+                "EXPLAIN ANALYSE DELETE FROM books",
+                "explain analyze verbose update books set title = 'x'",
+                "EXPLAIN (ANALYZE, BUFFERS) INSERT INTO books VALUES (1)",
+                "DESC ANALYZE DELETE FROM books",
+            ] {
+                assert_eq!(
+                    kind(dialect, text),
+                    StatementKind::Write,
+                    "{dialect:?} {text}"
+                );
+            }
+        }
+    }
+
+    #[test]
+    fn a_mysql_executable_comment_makes_a_write() {
+        let text = "SELECT 1 /*! , sleep(1) */";
+        assert_eq!(kind(Dialect::MySql, text), StatementKind::Write);
+        // Elsewhere it is a comment like any other.
+        assert_eq!(kind(Dialect::Postgres, text), StatementKind::Read);
+        assert_eq!(kind(Dialect::Sqlite, text), StatementKind::Read);
+    }
+
+    #[test]
+    fn odd_text_never_panics_the_kind() {
+        for dialect in DIALECTS {
+            for text in [
+                "(", ")", "'", "\"", "/*", "é", ";", "((((", "EXPLAIN", "WITH",
+            ] {
+                let _ = kind(dialect, text);
+            }
+        }
+    }
 }
```

- [ ] **Step 2: Run them to see them fail**

Run: `~/.cargo/bin/cargo test --locked -p tabletist-db --lib sql::tests`
Expected: does not compile. `error[E0425]: cannot find function `kind` in this scope` and `error[E0433]: cannot find type `StatementKind` in this scope`.

- [ ] **Step 3: Write `kind`**

Above `#[cfg(test)]` in `sql.rs`. `bare_word` is new and is used again in task 2: unlike `word_of`, it does not take a quoted name for a word.

```diff
diff --git a/crates/tabletist-db/src/sql.rs b/crates/tabletist-db/src/sql.rs
index fec8aa6..2fc3a6c 100644
--- a/crates/tabletist-db/src/sql.rs
+++ b/crates/tabletist-db/src/sql.rs
@@ -921,6 +921,85 @@ fn unicode_name(statement: &str, tokens: &[Token]) -> bool {
     })
 }
 
+/// What a statement looks like to the SQL editor, which runs a script that
+/// only reads in a read-only transaction and commits no other.
+#[derive(Debug, Clone, Copy, PartialEq, Eq)]
+pub enum StatementKind {
+    /// A query by its first word, with no word in it that changes data.
+    Read,
+    /// Anything else.
+    Write,
+}
+
+/// The words a statement that reads starts with.
+const READS: [&str; 8] = [
+    "SELECT", "VALUES", "TABLE", "WITH", "SHOW", "EXPLAIN", "DESCRIBE", "DESC",
+];
+
+/// The words that explain the statement after them. MySQL takes all three.
+const EXPLAINS: [&str; 3] = ["EXPLAIN", "DESCRIBE", "DESC"];
+
+/// The word that makes an `EXPLAIN` run its statement. PostgreSQL takes
+/// both spellings.
+const ANALYZES: [&str; 2] = ["ANALYZE", "ANALYSE"];
+
+/// The words that change data wherever they stand in a query: in a `WITH`,
+/// after `FOR`.
+const CHANGES: [&str; 4] = ["INSERT", "UPDATE", "DELETE", "MERGE"];
+
+/// The word a token spells when it is not quoted, upper-cased: a keyword
+/// or a bare name.
+fn bare_word(text: &str, token: &Token) -> Option<String> {
+    matches!(token.kind, TokenKind::Keyword | TokenKind::Identifier)
+        .then(|| text[token.range.clone()].to_ascii_uppercase())
+}
+
+/// Whether `statement` only reads, as far as its words say. It errs toward
+/// `Write`: a bare name spelled like a data-changing word counts. Taking a
+/// write for a read is safe, since a read runs in a read-only transaction
+/// where the database refuses the write; taking a read for a write only
+/// commits nothing.
+///
+/// An `EXPLAIN` without `ANALYZE` runs nothing, so it is a read whatever
+/// it explains. With `ANALYZE` it runs its statement, and is what that is.
+pub fn kind(dialect: Dialect, statement: &str) -> StatementKind {
+    let tokens = tokenize(dialect, statement);
+    // MySQL runs what an executable comment holds, and it can hold anything.
+    if tokens
+        .iter()
+        .any(|token| token.kind == TokenKind::ExecutableComment)
+    {
+        return StatementKind::Write;
+    }
+    let first = tokens
+        .iter()
+        .find(|token| !matches!(token.kind, TokenKind::Whitespace | TokenKind::Comment));
+    // Only comments: nothing runs.
+    let Some(first) = first else {
+        return StatementKind::Read;
+    };
+    let words: Vec<String> = tokens
+        .iter()
+        .filter_map(|token| bare_word(statement, token))
+        .collect();
+    let has = |wanted: &str| words.iter().any(|word| word == wanted);
+    let first_word = bare_word(statement, first);
+    let first_word = first_word.as_deref();
+    let explains = first_word.is_some_and(|word| EXPLAINS.contains(&word));
+    if explains && !ANALYZES.iter().any(|word| has(word)) {
+        return StatementKind::Read;
+    }
+    let reads = match first_word {
+        Some(word) => READS.contains(&word) || (dialect == Dialect::Sqlite && word == "PRAGMA"),
+        None => &statement[first.range.clone()] == "(",
+    };
+    if reads && !CHANGES.iter().any(|word| has(word)) {
+        StatementKind::Read
+    } else {
+        StatementKind::Write
+    }
+}
+
 #[cfg(test)]
 mod tests {
     use super::*;
```

- [ ] **Step 4: Run them to see them pass**

Run: `~/.cargo/bin/cargo test --locked -p tabletist-db --lib sql::tests`
Expected: `test result: ok. 76 passed`.
Run the four checks. Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/tabletist-db/src/sql.rs
git commit -m "Tell a statement that writes from one that reads

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 2: `sql::unbounded`

Names `UPDATE` or `DELETE` for a statement of that kind with no `WHERE` outside parentheses. Step 3 of the spec asks the user about such a statement; this task only finds it. The verb is the statement's own: the first word outside parentheses that can be one, after a `WITH` list or an `EXPLAIN ANALYZE`. That is what keeps `SELECT ... FOR UPDATE` and `INSERT ... ON CONFLICT DO UPDATE` from being named.

**Files:**
- Modify: `crates/tabletist-db/src/sql.rs`

- [ ] **Step 1: Write the failing tests**

At the end of `mod tests`:

```diff
diff --git a/crates/tabletist-db/src/sql.rs b/crates/tabletist-db/src/sql.rs
index 2fc3a6c..c657b1e 100644
--- a/crates/tabletist-db/src/sql.rs
+++ b/crates/tabletist-db/src/sql.rs
@@ -2275,4 +2331,100 @@ mod tests {
             }
         }
     }
+
+    #[test]
+    fn an_update_or_delete_without_a_where_is_unbounded() {
+        for dialect in DIALECTS {
+            for (text, verb) in [
+                ("UPDATE books SET title = 'x'", Verb::Update),
+                ("delete from books", Verb::Delete),
+                ("DELETE FROM books -- WHERE id = 1", Verb::Delete),
+                ("UPDATE books SET title = 'WHERE id = 1'", Verb::Update),
+                // A WHERE of a subquery bounds the subquery.
+                (
+                    "UPDATE books SET title = (SELECT name FROM drafts WHERE drafts.id = books.id)",
+                    Verb::Update,
+                ),
+                ("DELETE FROM books ORDER BY id LIMIT 5", Verb::Delete),
+                (
+                    "WITH old AS (SELECT id FROM books WHERE id < 5) DELETE FROM books",
+                    Verb::Delete,
+                ),
+                (
+                    "WITH old AS (SELECT 1) UPDATE books SET title = 'x'",
+                    Verb::Update,
+                ),
+                // It runs the statement behind it.
+                ("EXPLAIN ANALYZE DELETE FROM books", Verb::Delete),
+                ("EXPLAIN ANALYSE DELETE FROM books", Verb::Delete),
+                (
+                    "EXPLAIN (ANALYZE, BUFFERS) UPDATE books SET title = 'x'",
+                    Verb::Update,
+                ),
+            ] {
+                assert_eq!(unbounded(dialect, text), Some(verb), "{dialect:?} {text}");
+            }
+        }
+    }
+
+    #[test]
+    fn a_where_outside_parentheses_bounds_the_statement() {
+        for dialect in DIALECTS {
+            for text in [
+                "UPDATE books SET title = 'x' WHERE id = 1",
+                "delete from books where id = 1",
+                "DELETE FROM books WHERE id IN (SELECT book_id FROM drafts)",
+                "UPDATE books SET title = 'x' WHERE CURRENT OF pick",
+                "WITH old AS (SELECT 1) DELETE FROM books WHERE id = 1",
+                "EXPLAIN ANALYZE DELETE FROM books WHERE id = 1",
+            ] {
+                assert_eq!(unbounded(dialect, text), None, "{dialect:?} {text}");
+            }
+        }
+    }
+
+    #[test]
+    fn only_an_update_or_a_delete_is_ever_unbounded() {
+        for dialect in DIALECTS {
+            for text in [
+                "SELECT * FROM books",
+                "SELECT * FROM books FOR UPDATE",
+                "WITH x AS (SELECT 1) SELECT * FROM x FOR UPDATE",
+                "INSERT INTO books VALUES (1)",
+                "INSERT INTO books VALUES (1) ON CONFLICT (id) DO UPDATE SET title = 'x'",
+                "INSERT INTO books VALUES (1) ON DUPLICATE KEY UPDATE title = 'x'",
+                "WITH s AS (SELECT 1) INSERT INTO books SELECT * FROM s ON CONFLICT (id) DO UPDATE SET title = 'x'",
+                "MERGE INTO books USING drafts ON books.id = drafts.id WHEN MATCHED THEN UPDATE SET title = 'x'",
+                "TRUNCATE books",
+                "DROP TABLE books",
+                // Nothing runs.
+                "EXPLAIN DELETE FROM books",
+                "",
+                "-- DELETE FROM books",
+            ] {
+                assert_eq!(unbounded(dialect, text), None, "{dialect:?} {text}");
+            }
+        }
+    }
+
+    #[test]
+    fn odd_text_never_panics_the_unbounded_check() {
+        for dialect in DIALECTS {
+            for text in [
+                "(",
+                ")",
+                ")))",
+                "'",
+                "/*",
+                "é",
+                "UPDATE",
+                "DELETE (",
+                "WITH",
+                "EXPLAIN ANALYZE",
+            ] {
+                let _ = unbounded(dialect, text);
+            }
+        }
+        assert_eq!(unbounded(Dialect::Postgres, "UPDATE"), Some(Verb::Update));
+    }
 }
```

- [ ] **Step 2: Run them to see them fail**

Run: `~/.cargo/bin/cargo test --locked -p tabletist-db --lib sql::tests`
Expected: does not compile. `cannot find function `unbounded` in this scope` and `cannot find type `Verb` in this scope`.

- [ ] **Step 3: Write `unbounded`**

After `kind`:

```diff
diff --git a/crates/tabletist-db/src/sql.rs b/crates/tabletist-db/src/sql.rs
index 2fc3a6c..c657b1e 100644
--- a/crates/tabletist-db/src/sql.rs
+++ b/crates/tabletist-db/src/sql.rs
@@ -1000,6 +1000,62 @@ pub fn kind(dialect: Dialect, statement: &str) -> StatementKind {
     }
 }
 
+/// A statement that changes or removes rows, for [`unbounded`].
+#[derive(Debug, Clone, Copy, PartialEq, Eq)]
+pub enum Verb {
+    Update,
+    Delete,
+}
+
+/// The words a statement's own verb can be, once a `WITH` list and an
+/// `EXPLAIN` in front of it are passed.
+const VERBS: [&str; 8] = [
+    "SELECT", "INSERT", "UPDATE", "DELETE", "MERGE", "VALUES", "TABLE", "REPLACE",
+];
+
+/// `UPDATE` or `DELETE` when `statement` is one with no `WHERE` of its
+/// own: it reaches every row of its table. A `WHERE` inside parentheses
+/// belongs to a subquery and does not count. A `WITH` in front is passed,
+/// and so is an `EXPLAIN` that has `ANALYZE`, which runs the statement
+/// behind it. An `INSERT` that updates on a conflict and a `MERGE` are
+/// bounded by their rows, and are never named.
+pub fn unbounded(dialect: Dialect, statement: &str) -> Option<Verb> {
+    let tokens = tokenize(dialect, statement);
+    // The words outside parentheses, in order.
+    let mut top = Vec::new();
+    let mut depth = 0_usize;
+    let mut analyzed = false;
+    for token in &tokens {
+        match (token.kind, &statement[token.range.clone()]) {
+            (TokenKind::Punctuation, "(") => depth += 1,
+            (TokenKind::Punctuation, ")") => depth = depth.saturating_sub(1),
+            _ => {
+                if let Some(word) = bare_word(statement, token) {
+                    // Also as an option: EXPLAIN (ANALYZE).
+                    analyzed |= ANALYZES.contains(&word.as_str());
+                    if depth == 0 {
+                        top.push(word);
+                    }
+                }
+            }
+        }
+    }
+    let leads = match top.first()?.as_str() {
+        "UPDATE" | "DELETE" | "WITH" => true,
+        word => EXPLAINS.contains(&word) && analyzed,
+    };
+    if !leads {
+        return None;
+    }
+    let at = top.iter().position(|word| VERBS.contains(&word.as_str()))?;
+    let verb = match top[at].as_str() {
+        "UPDATE" => Verb::Update,
+        "DELETE" => Verb::Delete,
+        _ => return None,
+    };
+    (!top[at + 1..].iter().any(|word| word == "WHERE")).then_some(verb)
+}
+
 #[cfg(test)]
 mod tests {
     use super::*;
```

- [ ] **Step 4: Run them to see them pass**

Run: `~/.cargo/bin/cargo test --locked -p tabletist-db --lib sql::tests`
Expected: `test result: ok. 80 passed`.
Run the four checks. Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/tabletist-db/src/sql.rs
git commit -m "Find an UPDATE or a DELETE that has no WHERE

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 3: The run's mode and its end

The types, and the argument through every layer. After this task `run_script` takes a `ScriptMode`; `Write` on a read-only connection is `Error::ReadOnly` with nothing sent, a refused statement says the sentence of its mode, and `Write` on a writable connection answers `Error::Unsupported` from each driver until its task (4, 5, 6) replaces that line. The app passes `ReadOnly`.

This is the task with the widest reach and the least thought: most of it is the compiler asking for a new argument or a new field.

**Files:**
- Modify: `crates/tabletist-db/src/script.rs`, `error.rs`, `lib.rs`, `sqlite.rs`, `pg/script.rs`, `mysql/script.rs`
- Modify: `crates/tabletist-db/tests/sqlite.rs`, `tests/postgres.rs`, `tests/mysql.rs`
- Modify: `src/backend.rs`, `src/ui/sql_results.rs`, and test values in `src/app.rs`, `src/model.rs`, `src/testing.rs`, `src/shots.rs`, `src/ui/format.rs`

- [ ] **Step 1: Write the failing tests**

In `script.rs` and `error.rs`, in their `mod tests`:

```diff
diff --git a/crates/tabletist-db/src/script.rs b/crates/tabletist-db/src/script.rs
index d119124..a2187cf 100644
--- a/crates/tabletist-db/src/script.rs
+++ b/crates/tabletist-db/src/script.rs
@@ -153,6 +226,35 @@ mod tests {
         assert!(flag.is_finishing());
     }
 
+    #[test]
+    fn an_outcome_starts_rolled_back_and_whole() {
+        let outcome = ScriptOutcome::default();
+        assert_eq!(outcome.end, ScriptEnd::RolledBack);
+        assert_eq!(outcome.rollback_warning, None);
+        assert_eq!(outcome.broken, None);
+        assert_eq!(ScriptMode::default(), ScriptMode::ReadOnly);
+    }
+
+    #[test]
+    fn a_failed_start_or_end_names_the_modes_transaction() {
+        let error = Error::query("no");
+        for (mode, start, end) in [
+            (
+                ScriptMode::ReadOnly,
+                "the connection was lost: could not start the read-only transaction: no",
+                "the connection was lost: could not end the read-only transaction: no",
+            ),
+            (
+                ScriptMode::Write,
+                "the connection was lost: could not start the transaction: no",
+                "the connection was lost: could not end the transaction: no",
+            ),
+        ] {
+            assert_eq!(cannot_start(mode, &error).to_string(), start);
+            assert_eq!(cleanup_failed(mode, &error).to_string(), end);
+        }
+    }
+
     #[test]
     fn only_the_first_stop_says_it_stopped_the_run() {
         let flag = StopFlag::new();
```

```diff
diff --git a/crates/tabletist-db/src/error.rs b/crates/tabletist-db/src/error.rs
index d8a5bd9..c623daa 100644
--- a/crates/tabletist-db/src/error.rs
+++ b/crates/tabletist-db/src/error.rs
@@ -141,6 +155,7 @@ mod tests {
         let refused = Error::Refused {
             line: 4,
             what: "COMMIT".into(),
+            mode: ScriptMode::ReadOnly,
         };
         assert!(!refused.is_connection_lost());
         assert_eq!(
@@ -148,4 +163,28 @@ mod tests {
             "line 4: Tabletist runs every query in a read-only transaction, so COMMIT is not allowed"
         );
     }
+
+    #[test]
+    fn a_refusal_in_a_run_that_writes_names_its_own_transaction() {
+        let refused = Error::Refused {
+            line: 2,
+            what: "COMMIT".into(),
+            mode: ScriptMode::Write,
+        };
+        assert_eq!(
+            refused.to_string(),
+            "line 2: Tabletist runs and commits the script in one transaction of its own, so \
+             COMMIT is not allowed"
+        );
+    }
+
+    #[test]
+    fn leaving_the_transaction_counts_as_a_lost_connection() {
+        assert!(Error::LeftTransaction.is_connection_lost());
+        assert!(!Error::ReadOnly.is_connection_lost());
+        assert_eq!(
+            Error::ReadOnly.to_string(),
+            "this connection opens read-only"
+        );
+    }
 }
```

In `tests/sqlite.rs`: a `write` helper beside `run`, two tests, and `ScriptMode::ReadOnly` in every existing `run_script` call (the same edit is made in the other test files in step 4):

```diff
diff --git a/crates/tabletist-db/tests/sqlite.rs b/crates/tabletist-db/tests/sqlite.rs
index 136397c..3780190 100644
--- a/crates/tabletist-db/tests/sqlite.rs
+++ b/crates/tabletist-db/tests/sqlite.rs
@@ -7,7 +7,7 @@ use std::time::Duration;
 
 use tabletist_db::{
     Access, ConnectSpec, Connection, Dialect, Driver, Error, Filter, FilterOp, HostKeys, ObjectRef,
-    RowQuery, Secrets, Sort, SortDir, StatementOutcome, StopFlag, Value, ValueKind,
+    RowQuery, ScriptMode, Secrets, Sort, SortDir, StatementOutcome, StopFlag, Value, ValueKind,
 };
 
 async fn fixture_as(access: Access) -> (Connection, tempfile::TempDir) {
@@ -824,7 +824,59 @@ async fn run(
     connection: &Connection,
     text: &str,
 ) -> tabletist_db::Result<tabletist_db::ScriptOutcome> {
-    within(connection.run_script(&script(text), 100, &StopFlag::new())).await
+    within(connection.run_script(&script(text), 100, ScriptMode::ReadOnly, &StopFlag::new())).await
+}
+
+/// Runs `text` as a script that writes, with a fresh stop flag and a
+/// generous limit.
+async fn write(
+    connection: &Connection,
+    text: &str,
+) -> tabletist_db::Result<tabletist_db::ScriptOutcome> {
+    within(connection.run_script(&script(text), 100, ScriptMode::Write, &StopFlag::new())).await
+}
+
+#[tokio::test]
+async fn a_run_that_writes_is_refused_on_a_read_only_connection() {
+    let (connection, _dir) = fixture_as(Access::ReadOnly).await;
+    // Whatever the script holds: a write, a read, a statement the guard
+    // refuses, nothing at all.
+    for text in ["DELETE FROM users", "SELECT 1", "COMMIT", ""] {
+        assert_eq!(
+            write(&connection, text).await,
+            Err(Error::ReadOnly),
+            "{text}"
+        );
+    }
+    assert_eq!(connection.count_rows(&users(10)).await.unwrap(), 5);
+}
+
+#[tokio::test]
+async fn a_refusal_names_the_transaction_of_its_mode() {
+    let (connection, _dir) = fixture_as(Access::Writable).await;
+    let refused = write(&connection, "DELETE FROM users;\nCOMMIT")
+        .await
+        .unwrap_err();
+    assert_eq!(
+        refused,
+        Error::Refused {
+            line: 2,
+            what: "COMMIT".into(),
+            mode: ScriptMode::Write,
+        }
+    );
+    assert_eq!(
+        refused.to_string(),
+        "line 2: Tabletist runs and commits the script in one transaction of its own, so COMMIT \
+         is not allowed"
+    );
+    let refused = run(&connection, "SELECT 1;\nCOMMIT").await.unwrap_err();
+    assert_eq!(
+        refused.to_string(),
+        "line 2: Tabletist runs every query in a read-only transaction, so COMMIT is not allowed"
+    );
+    // Nothing of either script ran.
+    assert_eq!(connection.count_rows(&users(10)).await.unwrap(), 5);
 }
 
 #[tokio::test]
@@ -833,6 +885,7 @@ async fn a_script_returns_rows_with_types_and_truncates_at_the_limit() {
     let outcome = within(connection.run_script(
         &script("SELECT id, email FROM users ORDER BY id"),
         3,
+        ScriptMode::ReadOnly,
         &StopFlag::new(),
     ))
     .await
@@ -858,10 +911,14 @@ async fn a_script_returns_rows_with_types_and_truncates_at_the_limit() {
 async fn truncates_at_the_limit_without_reading_the_whole_table() {
     let (connection, _dir) = fixture().await;
     let started = std::time::Instant::now();
-    let outcome =
-        within(connection.run_script(&script("SELECT * FROM big a, big b"), 10, &StopFlag::new()))
-            .await
-            .unwrap();
+    let outcome = within(connection.run_script(
+        &script("SELECT * FROM big a, big b"),
+        10,
+        ScriptMode::ReadOnly,
+        &StopFlag::new(),
+    ))
+    .await
+    .unwrap();
     assert!(matches!(
         outcome.results[0].outcome,
         StatementOutcome::Rows {
@@ -924,7 +981,10 @@ async fn a_statement_without_rows_is_done_without_a_count() {
     let outcome = run(&connection, "PRAGMA foreign_keys = ON").await.unwrap();
     assert_eq!(
         outcome.results[0].outcome,
-        StatementOutcome::Done { affected: None }
+        StatementOutcome::Done {
+            affected: None,
+            warnings: 0,
+        }
     );
 }
 
@@ -935,12 +995,16 @@ async fn a_comment_only_statement_is_done() {
         text: "-- nothing to run\n/* really */".into(),
         ..script("SELECT 1").remove(0)
     };
-    let outcome = within(connection.run_script(&[piece], 100, &StopFlag::new()))
-        .await
-        .unwrap();
+    let outcome =
+        within(connection.run_script(&[piece], 100, ScriptMode::ReadOnly, &StopFlag::new()))
+            .await
+            .unwrap();
     assert_eq!(
         outcome.results[0].outcome,
-        StatementOutcome::Done { affected: None }
+        StatementOutcome::Done {
+            affected: None,
+            warnings: 0,
+        }
     );
 }
 
@@ -962,9 +1026,10 @@ async fn a_hidden_second_statement_is_the_statements_error_not_a_second_run() {
         text: "SELECT 1; SELECT 2".into(),
         ..script("SELECT 1").remove(0)
     };
-    let outcome = within(connection.run_script(&[piece], 100, &StopFlag::new()))
-        .await
-        .unwrap();
+    let outcome =
+        within(connection.run_script(&[piece], 100, ScriptMode::ReadOnly, &StopFlag::new()))
+            .await
+            .unwrap();
     let [result] = outcome.results.as_slice() else {
         panic!("one result");
     };
@@ -1070,7 +1135,7 @@ async fn a_script_on_a_writable_file_changes_no_file() {
 #[tokio::test]
 async fn an_empty_script_is_not_cancelled_but_a_stopped_one_is() {
     let (connection, _dir) = fixture().await;
-    let empty = within(connection.run_script(&[], 10, &StopFlag::new()))
+    let empty = within(connection.run_script(&[], 10, ScriptMode::ReadOnly, &StopFlag::new()))
         .await
         .unwrap();
     assert!(empty.results.is_empty());
@@ -1078,9 +1143,10 @@ async fn an_empty_script_is_not_cancelled_but_a_stopped_one_is() {
 
     let stop = StopFlag::new();
     stop.stop();
-    let stopped = within(connection.run_script(&script("SELECT 1"), 10, &stop))
-        .await
-        .unwrap();
+    let stopped =
+        within(connection.run_script(&script("SELECT 1"), 10, ScriptMode::ReadOnly, &stop))
+            .await
+            .unwrap();
     assert!(stopped.was_cancelled());
     assert!(
         !stopped
@@ -1104,7 +1170,11 @@ async fn a_stopped_script_is_cancelled_whatever_the_timing() {
     let running = {
         let connection = std::sync::Arc::clone(&connection);
         let stop = stop.clone();
-        tokio::spawn(async move { connection.run_script(&script(FOREVER), 10, &stop).await })
+        tokio::spawn(async move {
+            connection
+                .run_script(&script(FOREVER), 10, ScriptMode::ReadOnly, &stop)
+                .await
+        })
     };
     tokio::time::sleep(Duration::from_millis(200)).await;
     stop.stop();
@@ -1129,7 +1199,11 @@ async fn a_stopped_script_never_reports_partial_rows() {
         let connection = std::sync::Arc::clone(&connection);
         let stop = stop.clone();
         let text = format!("SELECT 1; {FOREVER}");
-        tokio::spawn(async move { connection.run_script(&script(&text), 10, &stop).await })
+        tokio::spawn(async move {
+            connection
+                .run_script(&script(&text), 10, ScriptMode::ReadOnly, &stop)
+                .await
+        })
     };
     // The first statement takes microseconds, so after this wait the second
     // is the one running. There is no way to see which statement the job is
@@ -1160,7 +1234,7 @@ async fn a_session_cancel_ends_the_script_as_cancelled() {
         let connection = std::sync::Arc::clone(&connection);
         tokio::spawn(async move {
             connection
-                .run_script(&script(FOREVER), 10, &StopFlag::new())
+                .run_script(&script(FOREVER), 10, ScriptMode::ReadOnly, &StopFlag::new())
                 .await
         })
     };
@@ -1197,7 +1271,7 @@ async fn a_dropped_run_stops_its_remaining_statements() {
     let text = format!("SELECT 1; {FOREVER}; {FOREVER}");
     let dropped = tokio::time::timeout(
         Duration::from_millis(300),
-        connection.run_script(&script(&text), 10, &stop),
+        connection.run_script(&script(&text), 10, ScriptMode::ReadOnly, &stop),
     )
     .await;
     assert!(dropped.is_err(), "the script cannot finish");
```

- [ ] **Step 2: Run them to see them fail**

Run: `~/.cargo/bin/cargo test --locked -p tabletist-db`
Expected: does not compile. Among the errors: `cannot find type `ScriptMode``, `cannot find type `ScriptEnd``, `no variant named `ReadOnly` found for enum `Error``.

- [ ] **Step 3: The types, and the argument through the crate**

`script.rs`, `error.rs` and `lib.rs`. `ScriptEnd::CommitFailed` carries a count beside its error: on MySQL a commit can fail after the server has committed part of the run by itself (task 6), and then "nothing is written" would be false.

```diff
diff --git a/crates/tabletist-db/src/script.rs b/crates/tabletist-db/src/script.rs
index d119124..a2187cf 100644
--- a/crates/tabletist-db/src/script.rs
+++ b/crates/tabletist-db/src/script.rs
@@ -7,6 +7,60 @@ use std::time::Duration;
 
 use crate::{ColumnMeta, Error, Result, Value};
 
+/// How a script's transaction is meant to end.
+#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
+pub enum ScriptMode {
+    /// One read-only transaction, rolled back whatever the script did.
+    #[default]
+    ReadOnly,
+    /// One read-write transaction: committed when every statement
+    /// succeeded, rolled back after the first error, cancel or timeout.
+    Write,
+}
+
+impl ScriptMode {
+    /// The transaction a run of this mode is in, for a message.
+    pub(crate) fn transaction(self) -> &'static str {
+        match self {
+            Self::ReadOnly => "the read-only transaction",
+            Self::Write => "the transaction",
+        }
+    }
+}
+
+/// Why a statement the guard refused cannot run in a script of `mode`.
+pub(crate) fn refusal_sentence(mode: &ScriptMode, what: &str) -> String {
+    match mode {
+        ScriptMode::ReadOnly => format!(
+            "Tabletist runs every query in a read-only transaction, so {what} is not allowed"
+        ),
+        ScriptMode::Write => format!(
+            "Tabletist runs and commits the script in one transaction of its own, so {what} is \
+             not allowed"
+        ),
+    }
+}
+
+/// What remains of a script's work once its run is over.
+#[derive(Debug, Clone, Default, PartialEq)]
+pub enum ScriptEnd {
+    /// Nothing: the transaction was rolled back. Every read-only run ends
+    /// so.
+    #[default]
+    RolledBack,
+    /// Every statement's work is written.
+    Committed,
+    /// The database committed on its own before the run failed or was
+    /// stopped (MySQL, at DDL): the first `committed` statements are
+    /// written, the rest is not.
+    Partly { committed: usize },
+    /// The commit itself failed, and what it would have kept is rolled
+    /// back. `committed` is 0 wherever a transaction holds a whole run: then
+    /// nothing is written. On MySQL it counts the statements the database
+    /// had committed on its own before that, as `Partly` does.
+    CommitFailed { error: Error, committed: usize },
+}
+
 /// What a script did: one result per statement that started, in order.
 /// After an `Error` or `Cancelled` outcome no further statement runs.
 #[derive(Debug, Clone, Default, PartialEq)]
@@ -14,6 +68,15 @@ pub struct ScriptOutcome {
     pub results: Vec<StatementResult>,
     /// A stop or cancel ended the run.
     pub stopped: bool,
+    /// How the run's transaction ended.
+    pub end: ScriptEnd,
+    /// What the database said when it could not undo everything (MySQL's
+    /// non-transactional tables). With it, `RolledBack` and `Partly` no
+    /// longer say that the rest is gone.
+    pub rollback_warning: Option<String>,
+    /// A [`ScriptMode::Write`] run only: the session could not be put back
+    /// after the run and must be closed. `end` still holds.
+    pub broken: Option<Error>,
 }
 
 impl ScriptOutcome {
@@ -46,8 +109,12 @@ pub enum StatementOutcome {
         truncated: bool,
     },
     /// A statement without a result set, with the rows it affected when
-    /// the database says (`None` when it has no meaningful count).
-    Done { affected: Option<u64> },
+    /// the database says (`None` when it has no meaningful count), and the
+    /// warnings it raised (MySQL counts them; the others have none).
+    Done {
+        affected: Option<u64>,
+        warnings: u16,
+    },
     /// The statement failed. `position` is a 1-based character position in
     /// the statement's text (PostgreSQL reports one).
     Error {
@@ -117,8 +184,14 @@ pub(crate) fn statement_failed(error: Error, position: Option<usize>) -> Result<
 
 /// A cleanup failure closes the session: it may still be inside the
 /// script's transaction.
-pub(crate) fn cleanup_failed(error: &Error) -> Error {
-    Error::ConnectionLost(format!("could not end the read-only transaction: {error}"))
+pub(crate) fn cleanup_failed(mode: ScriptMode, error: &Error) -> Error {
+    Error::ConnectionLost(format!("could not end {}: {error}", mode.transaction()))
+}
+
+/// A transaction that could not start closes the session: the next run
+/// would fail the same way.
+pub(crate) fn cannot_start(mode: ScriptMode, error: &Error) -> Error {
+    Error::ConnectionLost(format!("could not start {}: {error}", mode.transaction()))
 }
 
 /// Runs a cleanup step again once when a cancel meant for a statement
```

```diff
diff --git a/crates/tabletist-db/src/error.rs b/crates/tabletist-db/src/error.rs
index d8a5bd9..c623daa 100644
--- a/crates/tabletist-db/src/error.rs
+++ b/crates/tabletist-db/src/error.rs
@@ -2,6 +2,8 @@
 
 use std::fmt;
 
+use crate::script::{ScriptMode, refusal_sentence};
+
 pub type Result<T> = std::result::Result<T, Error>;
 
 /// Everything that can go wrong talking to a database. Messages are shown to
@@ -26,15 +28,24 @@ pub enum Error {
     },
     #[error("the query was cancelled")]
     Cancelled,
-    /// A script holds a statement that could end or change the read-only
-    /// transaction; nothing ran.
-    #[error(
-        "line {line}: Tabletist runs every query in a read-only transaction, so {what} is not allowed"
-    )]
-    Refused { line: usize, what: String },
+    /// A script holds a statement that could end or change its
+    /// transaction; nothing ran. `mode` is the run's, for the sentence.
+    #[error("line {line}: {}", refusal_sentence(.mode, .what))]
+    Refused {
+        line: usize,
+        what: String,
+        mode: ScriptMode,
+    },
+    /// A write was asked of a read-only session. Nothing was sent.
+    #[error("this connection opens read-only")]
+    ReadOnly,
     /// A script left the session read-write. The session is closed.
     #[error("the script left the read-only transaction, so the session was closed")]
     LeftReadOnly,
+    /// A script that writes ended the transaction its run commits. The
+    /// session is closed.
+    #[error("the script ended its transaction, so the session was closed")]
+    LeftTransaction,
     #[error("the operation timed out")]
     Timeout,
     #[error("the connection was lost: {0}")]
@@ -60,7 +71,10 @@ impl Error {
 
     /// Whether the session is unusable and must be reconnected.
     pub fn is_connection_lost(&self) -> bool {
-        matches!(self, Self::ConnectionLost(_) | Self::LeftReadOnly)
+        matches!(
+            self,
+            Self::ConnectionLost(_) | Self::LeftReadOnly | Self::LeftTransaction
+        )
     }
 }
 
```

```diff
diff --git a/crates/tabletist-db/src/lib.rs b/crates/tabletist-db/src/lib.rs
index 9ebca14..37b7e1b 100644
--- a/crates/tabletist-db/src/lib.rs
+++ b/crates/tabletist-db/src/lib.rs
@@ -33,7 +33,9 @@ pub use catalog::{
 pub use dialect::{Dialect, Sql, escape_like, quote_literal};
 pub use error::{Error, Result, SshStage};
 pub use query::{Filter, FilterOp, RowPage, RowQuery, Sort, SortDir};
-pub use script::{ScriptOutcome, StatementOutcome, StatementResult, StopFlag};
+pub use script::{
+    ScriptEnd, ScriptMode, ScriptOutcome, StatementOutcome, StatementResult, StopFlag,
+};
 pub use spec::{ConnectSpec, Driver, ParsedUrl, Secrets, SshAuth, SshSpec, TlsMode};
 pub use ssh::HostKeys;
 pub use value::{ColumnMeta, Value, ValueKind, value_from_pg_text};
@@ -157,8 +159,14 @@ impl Connection {
         self.driver().dialect()
     }
 
-    /// Runs `statements` in order in one read-only transaction that is
-    /// always rolled back, keeping at most `limit` rows per statement.
+    /// Runs `statements` in order in one transaction, keeping at most
+    /// `limit` rows per statement. In [`ScriptMode::ReadOnly`] the
+    /// transaction is read-only and always rolled back. In
+    /// [`ScriptMode::Write`] it is read-write, committed when every
+    /// statement succeeded and rolled back otherwise; the outcome's `end`
+    /// says which. A `Write` run on a session opened
+    /// [`Access::ReadOnly`] is [`Error::ReadOnly`], with nothing sent.
+    ///
     /// Refuses the whole script, running nothing, when a statement could
     /// leave the transaction (see [`sql::refusal`]). `stop` ends the run
     /// between statements (and, on SQLite, inside one); the caller also
@@ -167,14 +175,19 @@ impl Connection {
         &self,
         statements: &[sql::Statement],
         limit: u32,
+        mode: ScriptMode,
         stop: &StopFlag,
     ) -> Result<ScriptOutcome> {
+        if mode == ScriptMode::Write && self.access == Access::ReadOnly {
+            return Err(Error::ReadOnly);
+        }
         let dialect = self.dialect();
         for statement in statements {
             if let Some(what) = sql::refusal(dialect, &statement.text) {
                 return Err(Error::Refused {
                     line: statement.first_line,
                     what,
+                    mode,
                 });
             }
         }
@@ -183,9 +196,9 @@ impl Connection {
         }
         let texts: Vec<String> = statements.iter().map(|s| s.text.clone()).collect();
         match &self.inner {
-            Inner::Sqlite(conn) => conn.run_script(texts, limit, stop).await,
-            Inner::Postgres(conn) => conn.run_script(&texts, limit, stop).await,
-            Inner::MySql(conn) => conn.run_script(&texts, limit, stop).await,
+            Inner::Sqlite(conn) => conn.run_script(texts, limit, mode, stop).await,
+            Inner::Postgres(conn) => conn.run_script(&texts, limit, mode, stop).await,
+            Inner::MySql(conn) => conn.run_script(&texts, limit, mode, stop).await,
         }
     }
 
```

The three drivers take the argument and refuse `Write` for now. Their "could not start" and "could not end" sentences come from `script.rs`, so MySQL's own `cannot_start` goes:

```diff
diff --git a/crates/tabletist-db/src/pg/script.rs b/crates/tabletist-db/src/pg/script.rs
index eeefa64..63d187a 100644
--- a/crates/tabletist-db/src/pg/script.rs
+++ b/crates/tabletist-db/src/pg/script.rs
@@ -8,9 +8,10 @@ use tokio_postgres::SimpleQueryMessage;
 use tokio_postgres::error::SqlState;
 
 use super::{Conn, column_metas, first_text, query_error, row_values};
-use crate::script::{cleanup_failed, retry_cancelled, statement_failed};
+use crate::script::{cannot_start, cleanup_failed, retry_cancelled, statement_failed};
 use crate::{
-    ColumnMeta, Dialect, Error, Result, ScriptOutcome, StatementOutcome, StatementResult, StopFlag,
+    ColumnMeta, Dialect, Error, Result, ScriptMode, ScriptOutcome, StatementOutcome,
+    StatementResult, StopFlag,
 };
 
 impl Conn {
@@ -24,8 +25,12 @@ impl Conn {
         &self,
         texts: &[String],
         limit: u32,
+        mode: ScriptMode,
         stop: &StopFlag,
     ) -> Result<ScriptOutcome> {
+        if mode == ScriptMode::Write {
+            return Err(Error::Unsupported("read-write runs on PostgreSQL"));
+        }
         let client = self.client.lock().await;
         let mut outcome = ScriptOutcome::default();
         let tx = match open(&client).await {
@@ -44,9 +49,7 @@ impl Conn {
             Err(error) => {
                 stop.finish();
                 close(&client, Tx::Aborted).await.ok();
-                return Err(Error::ConnectionLost(format!(
-                    "could not start the read-only transaction: {error}"
-                )));
+                return Err(cannot_start(ScriptMode::ReadOnly, &error));
             }
         };
         // From here on a cancel would land on the cleanup: tell the
@@ -172,7 +175,7 @@ async fn close(client: &tokio_postgres::Client, tx: Tx) -> Result<()> {
     let rolled_back = retry_cancelled!(rollback(client));
     match (tx, rolled_back) {
         (Ok(Tx::Left), _) => Err(Error::LeftReadOnly),
-        (Err(error), _) | (_, Err(error)) => Err(cleanup_failed(&error)),
+        (Err(error), _) | (_, Err(error)) => Err(cleanup_failed(ScriptMode::ReadOnly, &error)),
         (Ok(_), Ok(())) => unlocked(retry_cancelled!(unlock(client))),
     }
 }
@@ -407,7 +410,13 @@ async fn run_statement(
                         _ => None,
                     })
                     .filter(|_| counts_rows(text));
-                Ok((StatementOutcome::Done { affected }, Tx::Open))
+                Ok((
+                    StatementOutcome::Done {
+                        affected,
+                        warnings: 0,
+                    },
+                    Tx::Open,
+                ))
             }
             Err(error) => failed(error, Some(0)),
         };
```

```diff
diff --git a/crates/tabletist-db/src/mysql/script.rs b/crates/tabletist-db/src/mysql/script.rs
index d2548d5..cfdf99f 100644
--- a/crates/tabletist-db/src/mysql/script.rs
+++ b/crates/tabletist-db/src/mysql/script.rs
@@ -12,9 +12,10 @@ use super::{
     Conn, READ_ONLY, UNKNOWN_SYSTEM_VARIABLE, column_metas, execute, from_row, prepare_session,
     query_error, row_values, status,
 };
-use crate::script::{cleanup_failed, retry_cancelled, statement_failed};
+use crate::script::{cannot_start, cleanup_failed, retry_cancelled, statement_failed};
 use crate::{
-    Access, Dialect, Error, Result, ScriptOutcome, StatementOutcome, StatementResult, StopFlag,
+    Access, Dialect, Error, Result, ScriptMode, ScriptOutcome, StatementOutcome, StatementResult,
+    StopFlag,
 };
 
 impl Conn {
@@ -32,8 +33,12 @@ impl Conn {
         &self,
         texts: &[String],
         limit: u32,
+        mode: ScriptMode,
         stop: &StopFlag,
     ) -> Result<ScriptOutcome> {
+        if mode == ScriptMode::Write {
+            return Err(Error::Unsupported("read-write runs on MySQL"));
+        }
         let mut conn = self.conn.lock().await;
         // Said before anything runs, not found out by the cleanup, which
         // would close the session after every run.
@@ -55,7 +60,7 @@ impl Conn {
             // The transaction could not start. The next run would fail the
             // same way, so the session is closed, after an attempt to end
             // what is open.
-            Err(error) => Ended::Broken(cannot_start(&error)),
+            Err(error) => Ended::Broken(cannot_start(ScriptMode::ReadOnly, &error)),
         };
         // From here on a cancel would land on the cleanup: tell the
         // backend to stop repeating its cancel.
@@ -101,14 +106,6 @@ enum Ended {
 /// under its older name.
 const READ_ONLY_SETTINGS: [&str; 2] = ["transaction_read_only", "tx_read_only"];
 
-/// A transaction that could not start closes the session: the next run
-/// would fail the same way.
-fn cannot_start(error: &Error) -> Error {
-    Error::ConnectionLost(format!(
-        "could not start the read-only transaction: {error}"
-    ))
-}
-
 /// Whether the server knows `COM_RESET_CONNECTION`, which the close needs:
 /// MySQL from 5.7.3, MariaDB from 10.2.4. The driver's own rule for
 /// `Conn::reset`, asked before anything runs.
@@ -306,7 +303,7 @@ async fn close(conn: &mut mysql_async::Conn, ended: Ended, access: Access) -> Re
         Ended::Unconfirmed => match retry_cancelled!(standing(conn)) {
             Ok(Standing::Left) => Ended::Left,
             Ok(Standing::Inside | Standing::Outside) => Ended::Unconfirmed,
-            Err(error) => Ended::Broken(cleanup_failed(&error)),
+            Err(error) => Ended::Broken(cleanup_failed(ScriptMode::ReadOnly, &error)),
         },
         known => known,
     };
@@ -328,7 +325,7 @@ async fn close(conn: &mut mysql_async::Conn, ended: Ended, access: Access) -> Re
         Ended::Unconfirmed | Ended::Unopened => rolled_back
             .and(reset)
             .and(prepared)
-            .map_err(|error| cleanup_failed(&error)),
+            .map_err(|error| cleanup_failed(ScriptMode::ReadOnly, &error)),
     }
 }
 
@@ -460,6 +457,7 @@ async fn run_statement(
         }
         return Ok(StatementOutcome::Done {
             affected: counts_rows(text).then_some(affected),
+            warnings: 0,
         });
     }
     // sql_select_limit does not bound every statement (SHOW, a SELECT with
```

```diff
diff --git a/crates/tabletist-db/src/sqlite.rs b/crates/tabletist-db/src/sqlite.rs
index 97816f8..1bd95fb 100644
--- a/crates/tabletist-db/src/sqlite.rs
+++ b/crates/tabletist-db/src/sqlite.rs
@@ -12,10 +12,11 @@ use rusqlite::config::DbConfig;
 use rusqlite::types::{FromSql, FromSqlError, FromSqlResult, ValueRef};
 use rusqlite::{ErrorCode, OpenFlags};
 
+use crate::script::cleanup_failed;
 use crate::{
     Access, ColumnInfo, ColumnMeta, Dialect, Error, ForeignKeyInfo, IndexInfo, MAX_LISTED,
-    ObjectInfo, ObjectKind, ObjectRef, Result, RowPage, RowQuery, ScriptOutcome, StatementOutcome,
-    StatementResult, StopFlag, Structure, Value, ValueKind,
+    ObjectInfo, ObjectKind, ObjectRef, Result, RowPage, RowQuery, ScriptMode, ScriptOutcome,
+    StatementOutcome, StatementResult, StopFlag, Structure, Value, ValueKind,
 };
 
 mod fence;
@@ -105,7 +106,8 @@ fn script(
             outcome.stopped = true;
             // The interrupt may have left a transaction open.
             stop.finish();
-            end_transaction(connection).map_err(|error| crate::script::cleanup_failed(&error))?;
+            end_transaction(connection)
+                .map_err(|error| cleanup_failed(ScriptMode::ReadOnly, &error))?;
             return Ok(outcome);
         }
         Err(error) => return Err(error),
@@ -128,7 +130,7 @@ fn script(
             .unwrap_or(false);
     let ended = end_transaction(connection);
     ran?;
-    ended.map_err(|error| crate::script::cleanup_failed(&error))?;
+    ended.map_err(|error| cleanup_failed(ScriptMode::ReadOnly, &error))?;
     if left {
         return Err(Error::LeftReadOnly);
     }
@@ -247,7 +249,10 @@ fn statement(
     let text = code_only(text);
     // Only comments: SQLite prepares nothing, and there is nothing to run.
     if text.is_empty() {
-        return Ok(StatementOutcome::Done { affected: None });
+        return Ok(StatementOutcome::Done {
+            affected: None,
+            warnings: 0,
+        });
     }
     // A second statement in the text is refused (MultipleStatement), but
     // only after rusqlite prepared it. SQLite asks the authorizer for it as
@@ -259,6 +264,7 @@ fn statement(
         let changed = statement.raw_execute().map_err(map_error)?;
         return Ok(StatementOutcome::Done {
             affected: counts_changes(text).then_some(changed as u64),
+            warnings: 0,
         });
     }
     let mut rows = statement.raw_query();
@@ -627,8 +633,12 @@ impl Conn {
         &self,
         texts: Vec<String>,
         limit: u32,
+        mode: ScriptMode,
         stop: &StopFlag,
     ) -> Result<ScriptOutcome> {
+        if mode == ScriptMode::Write {
+            return Err(Error::Unsupported("read-write runs on SQLite"));
+        }
         let stop = stop.clone();
         let limit = limit as usize;
         // If the caller drops this future, `run` interrupts the statement
```

- [ ] **Step 4: The rest of the crate's tests**

Every `run_script(` call gains `ScriptMode::ReadOnly` before its stop flag, every `StatementOutcome::Done { affected: None }` gains `warnings: 0`, and the test files import `ScriptMode`. Nothing else changes in them:

```diff
diff --git a/crates/tabletist-db/src/pg/script.rs b/crates/tabletist-db/src/pg/script.rs
index eeefa64..63d187a 100644
--- a/crates/tabletist-db/src/pg/script.rs
+++ b/crates/tabletist-db/src/pg/script.rs
@@ -465,7 +474,7 @@ mod tests {
     fn a_script_run_can_be_spawned() {
         fn send<T: Send>(_: &T) {}
         let _check = |conn: &Conn, texts: &[String], stop: &StopFlag| {
-            send(&conn.run_script(texts, 10, stop));
+            send(&conn.run_script(texts, 10, ScriptMode::ReadOnly, stop));
             send(&conn.server_version());
         };
     }
@@ -569,9 +578,12 @@ mod tests {
         stop: &StopFlag,
     ) -> Result<ScriptOutcome> {
         let texts: Vec<String> = script.iter().map(|&text| text.to_owned()).collect();
-        tokio::time::timeout(Duration::from_secs(10), conn.run_script(&texts, 10, stop))
-            .await
-            .expect("the run hung")
+        tokio::time::timeout(
+            Duration::from_secs(10),
+            conn.run_script(&texts, 10, ScriptMode::ReadOnly, stop),
+        )
+        .await
+        .expect("the run hung")
     }
 
     /// Statements the refusal stops long before they get here. Run past
```

```diff
diff --git a/crates/tabletist-db/src/mysql/script.rs b/crates/tabletist-db/src/mysql/script.rs
index d2548d5..cfdf99f 100644
--- a/crates/tabletist-db/src/mysql/script.rs
+++ b/crates/tabletist-db/src/mysql/script.rs
@@ -500,7 +498,7 @@ mod tests {
     fn a_script_run_can_be_spawned() {
         fn send<T: Send>(_: &T) {}
         let _check = |conn: &Conn, texts: &[String], stop: &StopFlag| {
-            send(&conn.run_script(texts, 10, stop));
+            send(&conn.run_script(texts, 10, ScriptMode::ReadOnly, stop));
             send(&conn.server_version());
         };
     }
@@ -655,7 +653,7 @@ mod tests {
                 Err(Error::query("the server did not start a transaction"))
             );
         }
-        let closed = cannot_start(&started(in_transaction).unwrap_err());
+        let closed = cannot_start(ScriptMode::ReadOnly, &started(in_transaction).unwrap_err());
         assert!(closed.is_connection_lost());
         assert_eq!(
             closed.to_string(),
@@ -781,9 +779,12 @@ mod tests {
         stop: &StopFlag,
     ) -> Result<ScriptOutcome> {
         let texts: Vec<String> = script.iter().map(|&text| text.to_owned()).collect();
-        tokio::time::timeout(Duration::from_secs(10), conn.run_script(&texts, 10, stop))
-            .await
-            .expect("the run hung")
+        tokio::time::timeout(
+            Duration::from_secs(10),
+            conn.run_script(&texts, 10, ScriptMode::ReadOnly, stop),
+        )
+        .await
+        .expect("the run hung")
     }
 
     /// The SQLSTATE a statement failed with.
@@ -1028,12 +1029,11 @@ mod tests {
             .await
             .unwrap();
         assert_eq!(outcome.results.len(), SETS.len());
-        assert!(
-            outcome
-                .results
-                .iter()
-                .all(|result| result.outcome == StatementOutcome::Done { affected: None })
-        );
+        assert!(outcome.results.iter().all(|result| result.outcome
+            == StatementOutcome::Done {
+                affected: None,
+                warnings: 0,
+            }));
         assert_eq!(settings(&conn).await, connected);
         // The last statement fails.
         let mut script = SETS.to_vec();
```

```diff
diff --git a/crates/tabletist-db/src/sqlite.rs b/crates/tabletist-db/src/sqlite.rs
index 97816f8..1bd95fb 100644
--- a/crates/tabletist-db/src/sqlite.rs
+++ b/crates/tabletist-db/src/sqlite.rs
@@ -1080,7 +1090,9 @@ mod tests {
                 let (conn, _dir) = fixture_as(access).await;
                 without_the_authorizer(&conn).await;
                 let script = texts.iter().map(|text| (*text).to_owned()).collect();
-                let ran = conn.run_script(script, 10, &StopFlag::new()).await;
+                let ran = conn
+                    .run_script(script, 10, ScriptMode::ReadOnly, &StopFlag::new())
+                    .await;
                 assert!(
                     matches!(ran, Err(Error::LeftReadOnly)),
                     "{access:?} {texts:?}: {ran:?}"
@@ -1108,7 +1120,8 @@ mod tests {
     /// `Connection::run_script` applies.
     async fn run_unrefused(conn: &Conn, texts: &[&str]) -> Result<ScriptOutcome> {
         let script = texts.iter().map(|text| (*text).to_owned()).collect();
-        conn.run_script(script, 10, &StopFlag::new()).await
+        conn.run_script(script, 10, ScriptMode::ReadOnly, &StopFlag::new())
+            .await
     }
 
     /// The session's `query_only` and whether it is out of a transaction.
```

```diff
diff --git a/crates/tabletist-db/tests/postgres.rs b/crates/tabletist-db/tests/postgres.rs
index 4fbd059..369b86e 100644
--- a/crates/tabletist-db/tests/postgres.rs
+++ b/crates/tabletist-db/tests/postgres.rs
@@ -570,7 +570,7 @@ async fn a_running_query_can_be_cancelled() {
     assert!(connection.fetch_rows(&users(1)).await.is_ok());
 }
 
-use tabletist_db::{Dialect, ScriptOutcome, StatementOutcome, StopFlag};
+use tabletist_db::{Dialect, ScriptMode, ScriptOutcome, StatementOutcome, StopFlag};
 
 /// A writable session for the bypass test's probe table.
 async fn admin() -> tokio_postgres::Client {
@@ -761,7 +761,8 @@ async fn run(
     text: &str,
     limit: u32,
 ) -> tabletist_db::Result<ScriptOutcome> {
-    within(connection.run_script(&script(text), limit, &StopFlag::new())).await
+    within(connection.run_script(&script(text), limit, ScriptMode::ReadOnly, &StopFlag::new()))
+        .await
 }
 
 #[tokio::test]
@@ -866,7 +867,10 @@ async fn show_and_statements_without_rows() {
     // SET has no row count; the driver's 0 for its command tag is not one.
     assert_eq!(
         outcome.results[1].outcome,
-        StatementOutcome::Done { affected: None }
+        StatementOutcome::Done {
+            affected: None,
+            warnings: 0,
+        }
     );
 }
 
@@ -1021,7 +1025,11 @@ async fn cancelled_while_running(
     let running = {
         let connection = std::sync::Arc::clone(&connection);
         let stop = stop.clone();
-        tokio::spawn(async move { connection.run_script(&script(text), limit, &stop).await })
+        tokio::spawn(async move {
+            connection
+                .run_script(&script(text), limit, ScriptMode::ReadOnly, &stop)
+                .await
+        })
     };
     runs_on_the_server(&admin, marker).await;
     stop.stop();
@@ -1094,7 +1102,9 @@ async fn a_stop_between_statements_lets_the_running_one_finish() {
         let stop = stop.clone();
         tokio::spawn(async move {
             let text = "DO $$ BEGIN /* tabletist between */ PERFORM pg_sleep(1); END $$; SELECT 2";
-            connection.run_script(&script(text), 10, &stop).await
+            connection
+                .run_script(&script(text), 10, ScriptMode::ReadOnly, &stop)
+                .await
         })
     };
     // A stop without a cancel: the first statement runs to its end, the
@@ -1105,7 +1115,10 @@ async fn a_stop_between_statements_lets_the_running_one_finish() {
     assert_eq!(outcome.results.len(), 2);
     assert_eq!(
         outcome.results[0].outcome,
-        StatementOutcome::Done { affected: None }
+        StatementOutcome::Done {
+            affected: None,
+            warnings: 0,
+        }
     );
     assert_eq!(outcome.results[1].outcome, StatementOutcome::Cancelled);
     assert!(outcome.stopped);
@@ -1218,9 +1231,14 @@ async fn a_script_stopped_before_it_starts_runs_nothing() {
     };
     let stop = StopFlag::new();
     stop.stop();
-    let outcome = within(connection.run_script(&script("SELECT 1; SELECT 2"), 10, &stop))
-        .await
-        .unwrap();
+    let outcome = within(connection.run_script(
+        &script("SELECT 1; SELECT 2"),
+        10,
+        ScriptMode::ReadOnly,
+        &stop,
+    ))
+    .await
+    .unwrap();
     assert_eq!(outcome.results.len(), 1);
     assert_eq!(outcome.results[0].outcome, StatementOutcome::Cancelled);
     assert!(outcome.stopped && outcome.was_cancelled());
@@ -1263,9 +1281,10 @@ async fn one_piece_cannot_hold_two_statements() {
     // it anyway, and the text does not run another way.
     let mut statements = script("SELECT 1");
     statements[0].text = "SELECT 1; SELECT 2".into();
-    let outcome = within(connection.run_script(&statements, 10, &StopFlag::new()))
-        .await
-        .unwrap();
+    let outcome =
+        within(connection.run_script(&statements, 10, ScriptMode::ReadOnly, &StopFlag::new()))
+            .await
+            .unwrap();
     assert_eq!(outcome.results.len(), 1);
     assert!(matches!(
         &outcome.results[0].outcome,
```

```diff
diff --git a/crates/tabletist-db/tests/mysql.rs b/crates/tabletist-db/tests/mysql.rs
index 4ebb6bb..b55e20d 100644
--- a/crates/tabletist-db/tests/mysql.rs
+++ b/crates/tabletist-db/tests/mysql.rs
@@ -854,7 +854,7 @@ async fn a_cancelled_query_leaves_no_transaction_holding_locks() {
     );
 }
 
-use tabletist_db::{Dialect, ScriptOutcome, StatementOutcome, StopFlag};
+use tabletist_db::{Dialect, ScriptMode, ScriptOutcome, StatementOutcome, StopFlag};
 
 fn script(text: &str) -> Vec<tabletist_db::sql::Statement> {
     tabletist_db::sql::statements(Dialect::MySql, text)
@@ -873,7 +873,8 @@ async fn run(
     text: &str,
     limit: u32,
 ) -> tabletist_db::Result<ScriptOutcome> {
-    within(connection.run_script(&script(text), limit, &StopFlag::new())).await
+    within(connection.run_script(&script(text), limit, ScriptMode::ReadOnly, &StopFlag::new()))
+        .await
 }
 
 /// The rows of a script's only statement.
@@ -1070,7 +1071,10 @@ async fn statements_without_rows_and_explain() {
     // SET has no row count; the server's 0 for it is not one.
     assert_eq!(
         outcome.results[0].outcome,
-        StatementOutcome::Done { affected: None }
+        StatementOutcome::Done {
+            affected: None,
+            warnings: 0,
+        }
     );
     assert!(matches!(
         &outcome.results[1].outcome,
@@ -1083,7 +1087,10 @@ async fn statements_without_rows_and_explain() {
     ));
     assert_eq!(
         outcome.results[3].outcome,
-        StatementOutcome::Done { affected: None }
+        StatementOutcome::Done {
+            affected: None,
+            warnings: 0,
+        }
     );
 }
 
@@ -1137,9 +1144,10 @@ async fn what_the_prepared_protocol_refuses_is_the_statements_error() {
     // The splitter would never hand over such a piece.
     let mut statements = script("SELECT 1");
     statements[0].text = "SELECT 1; SELECT 2".into();
-    let outcome = within(connection.run_script(&statements, 10, &StopFlag::new()))
-        .await
-        .unwrap();
+    let outcome =
+        within(connection.run_script(&statements, 10, ScriptMode::ReadOnly, &StopFlag::new()))
+            .await
+            .unwrap();
     assert_eq!(outcome.results.len(), 1);
     assert!(matches!(
         &outcome.results[0].outcome,
@@ -1389,7 +1397,13 @@ async fn bypasses_cannot_write() {
                     ),
                 "{attempt}"
             );
-            let ran = within(connection.run_script(&statements, 10, &StopFlag::new())).await;
+            let ran = within(connection.run_script(
+                &statements,
+                10,
+                ScriptMode::ReadOnly,
+                &StopFlag::new(),
+            ))
+            .await;
             assert!(
                 matches!(ran, Err(Error::Refused { .. })),
                 "{attempt}: {ran:?}"
@@ -1527,7 +1541,9 @@ async fn a_cancelled_script_keeps_earlier_results_and_the_session() {
             // A sleep of its own length, for `runs_on_the_server`. With a
             // table the interrupted SLEEP is an error; alone it answers 1.
             let text = "SELECT 1; SELECT count(*) FROM users WHERE SLEEP(31) = 0; SELECT 3";
-            connection.run_script(&script(text), 10, &stop).await
+            connection
+                .run_script(&script(text), 10, ScriptMode::ReadOnly, &stop)
+                .await
         })
     };
     runs_on_the_server(&mut admin, "SLEEP(31)").await;
@@ -1575,7 +1591,9 @@ async fn a_cancel_reaches_a_session_an_earlier_run_reset() {
             let stop = stop.clone();
             tokio::spawn(async move {
                 let text = format!("SELECT 1; SELECT count(*) FROM users WHERE {sleep} = 0");
-                connection.run_script(&script(&text), 10, &stop).await
+                connection
+                    .run_script(&script(&text), 10, ScriptMode::ReadOnly, &stop)
+                    .await
             })
         };
         runs_on_the_server(&mut admin, sleep).await;
@@ -1611,7 +1629,9 @@ async fn cancels_that_land_on_the_cleanup_leave_the_session_read_only_or_closed(
         let stop = stop.clone();
         tokio::spawn(async move {
             let text = "SELECT 1; SELECT count(*) FROM users WHERE SLEEP(32) = 0";
-            connection.run_script(&script(text), 10, &stop).await
+            connection
+                .run_script(&script(text), 10, ScriptMode::ReadOnly, &stop)
+                .await
         })
     };
     runs_on_the_server(&mut admin, "SLEEP(32)").await;
@@ -1656,7 +1676,9 @@ async fn a_stop_between_statements_lets_the_running_one_finish() {
         let stop = stop.clone();
         tokio::spawn(async move {
             let text = "SELECT SLEEP(1.5); SELECT 2";
-            connection.run_script(&script(text), 10, &stop).await
+            connection
+                .run_script(&script(text), 10, ScriptMode::ReadOnly, &stop)
+                .await
         })
     };
     // A stop without a cancel: the first statement runs to its end, the
@@ -1683,9 +1705,14 @@ async fn a_script_stopped_before_it_starts_runs_nothing() {
     };
     let stop = StopFlag::new();
     stop.stop();
-    let outcome = within(connection.run_script(&script("SELECT 1; SELECT 2"), 10, &stop))
-        .await
-        .unwrap();
+    let outcome = within(connection.run_script(
+        &script("SELECT 1; SELECT 2"),
+        10,
+        ScriptMode::ReadOnly,
+        &stop,
+    ))
+    .await
+    .unwrap();
     assert_eq!(outcome.results.len(), 1);
     assert_eq!(outcome.results[0].outcome, StatementOutcome::Cancelled);
     assert!(outcome.stopped && outcome.was_cancelled());
```

- [ ] **Step 5: The app**

`src/backend.rs` passes `ScriptMode::ReadOnly` (the one line that is not a test), `src/ui/sql_results.rs` lets two patterns skip `Done`'s new field, and test values gain what the types gained: `mode: tabletist_db::ScriptMode::ReadOnly` on an `Error::Refused`, `warnings: 0` on a `Done`, `..Default::default()` on a `ScriptOutcome`.

```diff
diff --git a/src/backend.rs b/src/backend.rs
index 0208846..24ceb49 100644
--- a/src/backend.rs
+++ b/src/backend.rs
@@ -10,7 +10,7 @@ use std::time::Duration;
 
 use tabletist_db::{
     Access, CancelHandle, ConnectSpec, Connection, Driver, Error, HostKeys, ObjectInfo, ObjectRef,
-    RowPage, RowQuery, ScriptOutcome, Secrets, StopFlag, Structure,
+    RowPage, RowQuery, ScriptMode, ScriptOutcome, Secrets, StopFlag, Structure,
 };
 use tokio::sync::mpsc as tokio_mpsc;
 
@@ -1934,8 +1934,8 @@ fn skip(outbox: &Outbox, command: Command) {
             session,
             request,
             result: Ok(ScriptOutcome {
-                results: Vec::new(),
                 stopped: true,
+                ..ScriptOutcome::default()
             }),
             cancel: Some(CancelReason::User),
         }),
@@ -2119,7 +2119,7 @@ async fn run_session(
                 // Awaited to its end whatever stops it: the script rolls
                 // back and leaves the session as it found it.
                 let result = connection
-                    .run_script(statements, *limit, &script_stop)
+                    .run_script(statements, *limit, ScriptMode::ReadOnly, &script_stop)
                     .await;
                 let timed_out = match timer {
                     Some(timer) => timer.end().await,
@@ -2649,6 +2649,7 @@ mod tests {
         ScriptOutcome {
             results: Vec::new(),
             stopped: true,
+            ..Default::default()
         }
     }
 
@@ -3763,6 +3764,7 @@ mod tests {
                 outcome: tabletist_db::StatementOutcome::Cancelled,
             }],
             stopped: true,
+            ..Default::default()
         }
     }
 
@@ -3846,6 +3848,7 @@ mod tests {
             Error::Refused {
                 line: 1,
                 what: "COMMIT".into(),
+                mode: tabletist_db::ScriptMode::ReadOnly,
             },
         ] {
             assert_eq!(lost_error(&Err::<ScriptOutcome, _>(kept)), None);
```

```diff
diff --git a/src/ui/sql_results.rs b/src/ui/sql_results.rs
index a8b43ed..5ba4535 100644
--- a/src/ui/sql_results.rs
+++ b/src/ui/sql_results.rs
@@ -773,6 +773,7 @@ fn statement_message(run: &SqlRun, index: usize, words: Words) -> Message {
         }
         StatementOutcome::Done {
             affected: Some(count),
+            ..
         } => {
             let noun = if *count == 1 {
                 "row affected"
@@ -783,7 +784,7 @@ fn statement_message(run: &SqlRun, index: usize, words: Words) -> Message {
             let text = format!("{count} {} · {time}", words.say(noun));
             Message::new(at, text, Tone::Plain)
         }
-        StatementOutcome::Done { affected: None } => {
+        StatementOutcome::Done { affected: None, .. } => {
             let text = format!("{} · {time}", words.say("Statement ran"));
             Message::new(at, text, Tone::Plain)
         }
@@ -1228,7 +1229,10 @@ mod tests {
 
     /// A statement that ran and gave no result set.
     fn done(affected: Option<u64>) -> StatementOutcome {
-        StatementOutcome::Done { affected }
+        StatementOutcome::Done {
+            affected,
+            warnings: 0,
+        }
     }
 
     #[test]
@@ -1688,6 +1692,7 @@ mod tests {
             Error::Refused {
                 line: 1,
                 what: "COMMIT".into(),
+                mode: tabletist_db::ScriptMode::ReadOnly,
             },
             Error::Unsupported("SQL editor on MySQL before 5.7"),
             Error::LeftReadOnly,
@@ -1785,6 +1790,7 @@ mod tests {
             let refused = Error::Refused {
                 line: 1,
                 what: "COMMIT".into(),
+                mode: tabletist_db::ScriptMode::ReadOnly,
             };
             harness.answer_sql(Err(refused.clone()), None);
             show_pane(&mut harness, tab, ResultPane::Results);
```

```diff
diff --git a/src/app.rs b/src/app.rs
index 9651bd6..2ce2eff 100644
--- a/src/app.rs
+++ b/src/app.rs
@@ -5961,6 +5961,7 @@ mod tests {
         let refused = Error::Refused {
             line: 2,
             what: "COMMIT".into(),
+            mode: tabletist_db::ScriptMode::ReadOnly,
         };
         harness.answer_sql(Err(refused.clone()), None);
         let editor = sql(&harness, tab, id);
@@ -6102,7 +6103,10 @@ mod tests {
             run(&mut harness, tab, id, true);
             harness.answer_sql(
                 Ok(script_outcome(vec![
-                    tabletist_db::StatementOutcome::Done { affected: None },
+                    tabletist_db::StatementOutcome::Done {
+                        affected: None,
+                        warnings: 0,
+                    },
                     tabletist_db::StatementOutcome::Cancelled,
                 ])),
                 cancel,
@@ -6484,6 +6488,7 @@ mod tests {
         harness.answer_sql(
             Ok(script_outcome(vec![tabletist_db::StatementOutcome::Done {
                 affected: None,
+                warnings: 0,
             }])),
             None,
         );
```

```diff
diff --git a/src/model.rs b/src/model.rs
index ddfabc1..0efa687 100644
--- a/src/model.rs
+++ b/src/model.rs
@@ -2907,7 +2907,10 @@ mod tests {
             vec![
                 rows_outcome(5),
                 rows_outcome(2),
-                tabletist_db::StatementOutcome::Done { affected: None },
+                tabletist_db::StatementOutcome::Done {
+                    affected: None,
+                    warnings: 0,
+                },
             ],
         );
         let (index, result) = sql.shown().unwrap();
@@ -2921,7 +2924,10 @@ mod tests {
         run_script(
             &mut sql,
             "SET x = 1",
-            vec![tabletist_db::StatementOutcome::Done { affected: None }],
+            vec![tabletist_db::StatementOutcome::Done {
+                affected: None,
+                warnings: 0,
+            }],
         );
         assert!(sql.shown().is_none());
         assert_eq!(sql.dims(), (0, 0));
@@ -3102,6 +3108,7 @@ mod tests {
         let refused = Error::Refused {
             line: 2,
             what: "COMMIT".into(),
+            mode: tabletist_db::ScriptMode::ReadOnly,
         };
         assert!(sql.finish_run(RequestId(30), Err(refused), None));
         assert_eq!(readings(&sql), (None, false, (0, 0), Some(2)));
@@ -3122,6 +3129,7 @@ mod tests {
         let refused = Error::Refused {
             line: 4,
             what: "COMMIT".into(),
+            mode: tabletist_db::ScriptMode::ReadOnly,
         };
         assert!(sql.finish_run(RequestId(20), Err(refused), None));
         assert_eq!(sql.error_mark(), Some((4, None)));
@@ -3158,6 +3166,7 @@ mod tests {
         let refused = Error::Refused {
             line: 2,
             what: "COMMIT".into(),
+            mode: tabletist_db::ScriptMode::ReadOnly,
         };
         assert!(sql.finish_run(RequestId(30), Err(refused), None));
         assert_eq!(sql.error_mark(), Some((2, None)));
```

```diff
diff --git a/src/testing.rs b/src/testing.rs
index 087a0a1..217ea39 100644
--- a/src/testing.rs
+++ b/src/testing.rs
@@ -618,6 +618,7 @@ pub fn script_outcome(
             })
             .collect(),
         stopped,
+        ..Default::default()
     }
 }
 
@@ -627,6 +628,7 @@ pub fn stopped_before_it_began() -> tabletist_db::ScriptOutcome {
     tabletist_db::ScriptOutcome {
         results: Vec::new(),
         stopped: true,
+        ..Default::default()
     }
 }
 
```

```diff
diff --git a/src/shots.rs b/src/shots.rs
index 7f957a5..dffa45b 100644
--- a/src/shots.rs
+++ b/src/shots.rs
@@ -643,6 +643,7 @@ fn shots() {
         let refused = tabletist_db::Error::Refused {
             line: 6,
             what: "COMMIT".into(),
+            mode: tabletist_db::ScriptMode::ReadOnly,
         };
         harness.answer_sql(Err(refused), None);
         let sql_tab = harness.app.workspace(tab).unwrap().active_tab.unwrap();
@@ -756,6 +757,7 @@ fn sql_editor(harness: &mut Harness) -> ConnTabId {
                 },
             }],
             stopped: false,
+            ..Default::default()
         }),
         None,
     );
```

```diff
diff --git a/src/ui/format.rs b/src/ui/format.rs
index e8cfd19..f9d0b68 100644
--- a/src/ui/format.rs
+++ b/src/ui/format.rs
@@ -873,6 +873,7 @@ mod tests {
         let refused = Error::Refused {
             line: 1,
             what: "COMMIT".into(),
+            mode: tabletist_db::ScriptMode::ReadOnly,
         };
         for driver in [Driver::Postgres, Driver::MySql, Driver::Sqlite] {
             assert!(refuses_writes(&refused, driver));
```

- [ ] **Step 6: Run the tests and the four checks**

Run: `~/.cargo/bin/cargo test --locked -p tabletist-db`
Expected: PASS, with `a_run_that_writes_is_refused_on_a_read_only_connection` and `a_refusal_names_the_transaction_of_its_mode` among the tests of `tests/sqlite.rs`.
Run the four checks. Expected: PASS. The app's 1265 tests pass unchanged in number.

- [ ] **Step 7: Commit**

```bash
git add crates/tabletist-db src
git commit -m "Give a script's run a mode, and its outcome an end

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 4: The run that writes, on SQLite

`query_only` is what keeps a writable SQLite session from writing. A run that writes turns it off for its own length: `PRAGMA query_only = OFF; BEGIN IMMEDIATE` as the app's own statements, outside the fence. `BEGIN IMMEDIATE` takes the write lock at once, so a database another program holds fails the run before any statement ran, with SQLite's message, and the session stays open.

Each statement still runs behind `Fence::Script`, which denies what it always denied (transaction and savepoint statements, `query_only` and `writable_schema` with a value, `wal_checkpoint`). The fence does not change. Before each statement the open transaction is checked; in a run that writes, one that is gone is `Error::LeftTransaction`.

`end_write` decides the end. Before it commits it asks once more whether the transaction is open: a script whose last statement ended it has no statement after it for the check in `statements` to run before. SQLite keeps the transaction open after a `COMMIT` that failed (a deferred foreign key, a reader holding the file), so the rollback in `end_transaction`, which every path reaches, is what makes `CommitFailed` true. `end_transaction` also sets `query_only` again. When it fails and the transaction is still open, nothing was rolled back and the end is not known: the run is an `Err`.

`ScriptOutcome::succeeded` and `put_back` come in with this task because it is the first to use them; tasks 5 and 6 use them too.

**Files:**
- Modify: `crates/tabletist-db/src/script.rs`, `crates/tabletist-db/src/sqlite.rs`
- Test: `crates/tabletist-db/tests/sqlite.rs`, and the `mod tests` of both files

- [ ] **Step 1: Write the failing tests**

Through the public API, in `tests/sqlite.rs`:

```diff
diff --git a/crates/tabletist-db/tests/sqlite.rs b/crates/tabletist-db/tests/sqlite.rs
index 3780190..1b87eb8 100644
--- a/crates/tabletist-db/tests/sqlite.rs
+++ b/crates/tabletist-db/tests/sqlite.rs
@@ -7,7 +7,8 @@ use std::time::Duration;
 
 use tabletist_db::{
     Access, ConnectSpec, Connection, Dialect, Driver, Error, Filter, FilterOp, HostKeys, ObjectRef,
-    RowQuery, ScriptMode, Secrets, Sort, SortDir, StatementOutcome, StopFlag, Value, ValueKind,
+    RowQuery, ScriptEnd, ScriptMode, Secrets, Sort, SortDir, StatementOutcome, StopFlag, Value,
+    ValueKind,
 };
 
 async fn fixture_as(access: Access) -> (Connection, tempfile::TempDir) {
@@ -879,6 +880,204 @@ async fn a_refusal_names_the_transaction_of_its_mode() {
     assert_eq!(connection.count_rows(&users(10)).await.unwrap(), 5);
 }
 
+/// The first cell of a query's only row, read in a read-only run.
+async fn number(connection: &Connection, sql: &str) -> i64 {
+    let outcome = run(connection, sql).await.unwrap();
+    match &outcome.results[0].outcome {
+        StatementOutcome::Rows { rows, .. } => match rows[0][0] {
+            Value::Int(number) => number,
+            ref other => panic!("{sql}: {other:?}"),
+        },
+        other => panic!("{sql}: {other:?}"),
+    }
+}
+
+/// What each statement of a run did, as its row count, or `None` for one
+/// that has none or did not end well.
+fn affected(outcome: &tabletist_db::ScriptOutcome) -> Vec<Option<u64>> {
+    outcome
+        .results
+        .iter()
+        .map(|result| match result.outcome {
+            StatementOutcome::Done { affected, .. } => affected,
+            _ => None,
+        })
+        .collect()
+}
+
+/// Whether the session refuses a write in a read-only run, as it must
+/// after every run that writes.
+async fn is_fenced(connection: &Connection) -> bool {
+    let outcome = run(connection, "DELETE FROM events").await.unwrap();
+    matches!(
+        &outcome.results[0].outcome,
+        StatementOutcome::Error {
+            error: Error::Query { code: Some(code), .. },
+            ..
+        } if code == "8"
+    )
+}
+
+#[tokio::test]
+async fn a_run_that_writes_is_committed_and_counts_its_rows() {
+    let (connection, _dir) = fixture_as(Access::Writable).await;
+    let outcome = write(
+        &connection,
+        "INSERT INTO events (kind, payload) VALUES ('probe', 'a'), ('probe', 'b'), ('probe', 'c');
+         UPDATE events SET payload = 'x' WHERE kind = 'probe' AND payload <> 'c';
+         DELETE FROM events WHERE kind = 'probe' AND payload = 'c';
+         CREATE TABLE notes (id INTEGER PRIMARY KEY)",
+    )
+    .await
+    .unwrap();
+    assert_eq!(outcome.end, ScriptEnd::Committed);
+    assert!(!outcome.was_cancelled());
+    assert_eq!(outcome.broken, None);
+    assert_eq!(affected(&outcome), [Some(3), Some(2), Some(1), None]);
+    let kept = "SELECT count(*) FROM events WHERE kind = 'probe' AND payload = 'x'";
+    assert_eq!(number(&connection, kept).await, 2);
+    assert_eq!(number(&connection, "SELECT count(*) FROM notes").await, 0);
+    assert!(is_fenced(&connection).await);
+}
+
+#[tokio::test]
+async fn a_statement_that_fails_rolls_the_whole_run_back() {
+    let (connection, _dir) = fixture_as(Access::Writable).await;
+    let outcome = write(
+        &connection,
+        "INSERT INTO events (kind) VALUES ('probe');
+         CREATE TABLE notes (id INTEGER PRIMARY KEY);
+         INSERT INTO no_such_table VALUES (1);
+         INSERT INTO events (kind) VALUES ('probe')",
+    )
+    .await
+    .unwrap();
+    assert_eq!(outcome.end, ScriptEnd::RolledBack);
+    // The statement after the error never ran.
+    assert_eq!(outcome.results.len(), 3);
+    assert!(matches!(
+        outcome.results[2].outcome,
+        StatementOutcome::Error { .. }
+    ));
+    let probes = "SELECT count(*) FROM events WHERE kind = 'probe'";
+    assert_eq!(number(&connection, probes).await, 0);
+    let tables = "SELECT count(*) FROM sqlite_master WHERE name = 'notes'";
+    assert_eq!(number(&connection, tables).await, 0);
+    assert!(is_fenced(&connection).await);
+}
+
+#[tokio::test]
+async fn a_run_that_writes_and_is_stopped_is_rolled_back() {
+    let (connection, _dir) = fixture_as(Access::Writable).await;
+    let connection = std::sync::Arc::new(connection);
+    let probes = "SELECT count(*) FROM events WHERE kind = 'probe'";
+    // Stopped before it began: nothing runs.
+    let stop = StopFlag::new();
+    stop.stop();
+    let text = "INSERT INTO events (kind) VALUES ('probe')";
+    let outcome = within(connection.run_script(&script(text), 10, ScriptMode::Write, &stop))
+        .await
+        .unwrap();
+    assert!(outcome.results.is_empty() && outcome.was_cancelled());
+    assert_eq!(outcome.end, ScriptEnd::RolledBack);
+    assert_eq!(number(&connection, probes).await, 0);
+    // Stopped while its second statement runs: the first is undone.
+    let stop = StopFlag::new();
+    let running = {
+        let (connection, stop) = (std::sync::Arc::clone(&connection), stop.clone());
+        let text = format!("{text}; {FOREVER}");
+        tokio::spawn(async move {
+            connection
+                .run_script(&script(&text), 10, ScriptMode::Write, &stop)
+                .await
+        })
+    };
+    tokio::time::sleep(Duration::from_millis(200)).await;
+    stop.stop();
+    let outcome = within(running).await.unwrap().unwrap();
+    assert!(outcome.was_cancelled());
+    assert_eq!(outcome.end, ScriptEnd::RolledBack);
+    assert_eq!(affected(&outcome), [Some(1), None]);
+    assert_eq!(number(&connection, probes).await, 0);
+    assert!(is_fenced(&connection).await);
+}
+
+#[tokio::test]
+async fn rows_a_write_returns_are_cut_at_the_limit_and_every_row_is_written() {
+    let (connection, _dir) = fixture_as(Access::Writable).await;
+    let text = "UPDATE big SET label = 'seen' RETURNING id";
+    let outcome =
+        within(connection.run_script(&script(text), 10, ScriptMode::Write, &StopFlag::new()))
+            .await
+            .unwrap();
+    assert_eq!(outcome.end, ScriptEnd::Committed);
+    assert!(matches!(
+        &outcome.results[0].outcome,
+        StatementOutcome::Rows { rows, truncated: true, .. } if rows.len() == 10
+    ));
+    let seen = "SELECT count(*) FROM big WHERE label = 'seen'";
+    assert_eq!(number(&connection, seen).await, 100_000);
+}
+
+#[tokio::test]
+async fn a_commit_that_fails_writes_nothing_and_the_next_run_can_begin() {
+    let (connection, _dir) = fixture_as(Access::Writable).await;
+    let tables = write(
+        &connection,
+        "CREATE TABLE shelves (id INTEGER PRIMARY KEY);
+         CREATE TABLE slots (
+             id INTEGER PRIMARY KEY,
+             shelf_id INTEGER REFERENCES shelves (id) DEFERRABLE INITIALLY DEFERRED
+         )",
+    )
+    .await
+    .unwrap();
+    assert_eq!(tables.end, ScriptEnd::Committed);
+    // The statement succeeds; the commit finds the shelf missing.
+    let outcome = write(&connection, "INSERT INTO slots VALUES (1, 99)")
+        .await
+        .unwrap();
+    assert_eq!(affected(&outcome), [Some(1)]);
+    assert!(
+        matches!(
+            &outcome.end,
+            ScriptEnd::CommitFailed {
+                error: Error::Query { message, .. },
+                committed: 0,
+            } if message.contains("FOREIGN KEY constraint failed")
+        ),
+        "{:?}",
+        outcome.end
+    );
+    assert_eq!(outcome.broken, None);
+    assert_eq!(number(&connection, "SELECT count(*) FROM slots").await, 0);
+    assert!(is_fenced(&connection).await);
+    // SQLite keeps the transaction open after a failed COMMIT. The run
+    // rolled it back, so the next one starts.
+    let outcome = write(
+        &connection,
+        "INSERT INTO shelves VALUES (99); INSERT INTO slots VALUES (1, 99)",
+    )
+    .await
+    .unwrap();
+    assert_eq!(outcome.end, ScriptEnd::Committed);
+    assert_eq!(number(&connection, "SELECT count(*) FROM slots").await, 1);
+}
+
+#[tokio::test]
+async fn a_run_that_writes_nothing_is_committed_all_the_same() {
+    let (connection, _dir) = fixture_as(Access::Writable).await;
+    let outcome = write(&connection, "SELECT count(*) FROM users")
+        .await
+        .unwrap();
+    assert_eq!(outcome.end, ScriptEnd::Committed);
+    let empty = within(connection.run_script(&[], 10, ScriptMode::Write, &StopFlag::new()))
+        .await
+        .unwrap();
+    assert!(empty.results.is_empty());
+    assert_eq!(empty.end, ScriptEnd::RolledBack);
+}
+
 #[tokio::test]
 async fn a_script_returns_rows_with_types_and_truncates_at_the_limit() {
     let (connection, _dir) = fixture().await;
```

Past the refusal list, in `sqlite.rs`'s `mod tests` (they reach `Conn::run_script`, which the list is in front of), and the two functions' own in `script.rs`:

```diff
diff --git a/crates/tabletist-db/src/sqlite.rs b/crates/tabletist-db/src/sqlite.rs
index 1bd95fb..2210ac1 100644
--- a/crates/tabletist-db/src/sqlite.rs
+++ b/crates/tabletist-db/src/sqlite.rs
@@ -1124,6 +1216,145 @@ mod tests {
             .await
     }
 
+    /// Runs `texts` as a script that writes, past the refusal list.
+    async fn write_unrefused(conn: &Conn, texts: &[&str]) -> Result<ScriptOutcome> {
+        let script = texts.iter().map(|text| (*text).to_owned()).collect();
+        conn.run_script(script, 10, ScriptMode::Write, &StopFlag::new())
+            .await
+    }
+
+    /// How many rows of `events` have this kind.
+    async fn events_of(conn: &Conn, kind: &'static str) -> i64 {
+        conn.run(move |connection| {
+            connection
+                .query_row(
+                    "SELECT count(*) FROM events WHERE kind = ?1",
+                    [kind],
+                    |row| row.get(0),
+                )
+                .map_err(map_error)
+        })
+        .await
+        .unwrap()
+    }
+
+    #[tokio::test]
+    async fn the_fence_denies_a_run_that_writes_what_would_end_its_transaction() {
+        for text in [
+            "COMMIT",
+            "END",
+            "ROLLBACK",
+            "SAVEPOINT mine",
+            "PRAGMA query_only = ON",
+            "PRAGMA writable_schema = ON",
+            "PRAGMA wal_checkpoint",
+        ] {
+            let (conn, _dir) = fixture_as(Access::Writable).await;
+            let probe = "INSERT INTO events (kind) VALUES ('probe')";
+            let outcome = write_unrefused(&conn, &[probe, text, probe]).await.unwrap();
+            // Denied as that statement's error; the run stops and is undone.
+            assert_eq!(outcome.results.len(), 2, "{text}");
+            assert!(
+                matches!(outcome.results[1].outcome, StatementOutcome::Error { .. }),
+                "{text}: {:?}",
+                outcome.results[1].outcome
+            );
+            assert_eq!(outcome.end, ScriptEnd::RolledBack, "{text}");
+            assert_eq!(events_of(&conn, "probe").await, 0, "{text}");
+            assert_eq!(standing(&conn).await, (1, true), "{text}");
+        }
+    }
+
+    #[tokio::test]
+    async fn a_run_that_writes_and_lost_its_transaction_runs_nothing_after_it() {
+        let (conn, _dir) = fixture_as(Access::Writable).await;
+        // What the refusal list and the authorizer both stop; this is the
+        // check behind them.
+        without_the_authorizer(&conn).await;
+        let ran = write_unrefused(
+            &conn,
+            &[
+                "INSERT INTO events (kind) VALUES ('before')",
+                "COMMIT",
+                "INSERT INTO events (kind) VALUES ('after')",
+            ],
+        )
+        .await;
+        assert_eq!(ran, Err(Error::LeftTransaction));
+        // The script's own commit stands; nothing ran outside a transaction.
+        assert_eq!(events_of(&conn, "before").await, 1);
+        assert_eq!(events_of(&conn, "after").await, 0);
+        assert_eq!(standing(&conn).await, (1, true));
+    }
+
+    #[tokio::test]
+    async fn a_run_whose_last_statement_ended_its_transaction_fails_too() {
+        let (conn, _dir) = fixture_as(Access::Writable).await;
+        without_the_authorizer(&conn).await;
+        // Nothing follows the COMMIT, so no check before a statement sees
+        // it: the end of the run has to.
+        let ran = write_unrefused(
+            &conn,
+            &["INSERT INTO events (kind) VALUES ('before')", "COMMIT"],
+        )
+        .await;
+        assert_eq!(ran, Err(Error::LeftTransaction));
+        assert_eq!(standing(&conn).await, (1, true));
+    }
+
+    #[tokio::test]
+    async fn a_stop_before_the_commit_rolls_a_run_that_writes_back() {
+        let (conn, _dir) = fixture_as(Access::Writable).await;
+        let stop = StopFlag::new();
+        let outcome = {
+            let stop = stop.clone();
+            conn.run(move |connection| {
+                begin(connection, ScriptMode::Write)?;
+                connection
+                    .execute_batch("INSERT INTO events (kind) VALUES ('probe')")
+                    .map_err(map_error)?;
+                // Every statement is done, and then the stop comes.
+                stop.stop();
+                end_write(connection, &stop, Ok(()), ScriptOutcome::default())
+            })
+            .await
+            .unwrap()
+        };
+        assert!(outcome.stopped);
+        assert_eq!(outcome.end, ScriptEnd::RolledBack);
+        assert!(stop.is_finishing());
+        assert_eq!(events_of(&conn, "probe").await, 0);
+        assert_eq!(standing(&conn).await, (1, true));
+    }
+
+    #[tokio::test]
+    async fn a_locked_database_fails_a_run_that_writes_before_it_begins() {
+        let (conn, dir) = fixture_as(Access::Writable).await;
+        let other = rusqlite::Connection::open(dir.path().join("fixture.db")).unwrap();
+        other.execute_batch("BEGIN IMMEDIATE").unwrap();
+        // Not the session's five seconds, for the test's sake. The run
+        // puts the session's own timeout back.
+        conn.run(|connection| {
+            connection
+                .busy_timeout(std::time::Duration::from_millis(50))
+                .map_err(map_error)
+        })
+        .await
+        .unwrap();
+        let probe = "INSERT INTO events (kind) VALUES ('probe')";
+        let ran = write_unrefused(&conn, &[probe]).await;
+        assert!(
+            matches!(&ran, Err(Error::Query { message, .. }) if message.contains("locked")),
+            "{ran:?}"
+        );
+        // The session is as it was: fenced, in no transaction, and usable.
+        assert_eq!(standing(&conn).await, (1, true));
+        other.execute_batch("ROLLBACK").unwrap();
+        let outcome = write_unrefused(&conn, &[probe]).await.unwrap();
+        assert_eq!(outcome.end, ScriptEnd::Committed);
+        assert_eq!(events_of(&conn, "probe").await, 1);
+    }
+
     /// The session's `query_only` and whether it is out of a transaction.
     async fn standing(conn: &Conn) -> (i64, bool) {
         conn.run(|connection| {
```

```diff
diff --git a/crates/tabletist-db/src/script.rs b/crates/tabletist-db/src/script.rs
index a2187cf..74333f6 100644
--- a/crates/tabletist-db/src/script.rs
+++ b/crates/tabletist-db/src/script.rs
@@ -235,6 +259,50 @@ mod tests {
         assert_eq!(ScriptMode::default(), ScriptMode::ReadOnly);
     }
 
+    #[test]
+    fn a_run_whose_session_could_not_be_put_back_keeps_its_end() {
+        let committed = ScriptOutcome {
+            end: ScriptEnd::Committed,
+            ..ScriptOutcome::default()
+        };
+        assert_eq!(committed.clone().put_back(Ok(())), committed);
+        let told = committed.put_back(Err(Error::query("no")));
+        assert_eq!(told.end, ScriptEnd::Committed);
+        let broken = told.broken.expect("the session is to be closed");
+        assert!(broken.is_connection_lost());
+        assert_eq!(
+            broken.to_string(),
+            "the connection was lost: could not end the transaction: no"
+        );
+    }
+
+    #[test]
+    fn a_run_succeeded_when_every_statement_ran_to_its_end() {
+        let result = |outcome| StatementResult {
+            elapsed: Duration::ZERO,
+            outcome,
+        };
+        let done = StatementOutcome::Done {
+            affected: Some(1),
+            warnings: 0,
+        };
+        let failed = StatementOutcome::Error {
+            error: Error::query("no"),
+            position: None,
+        };
+        let of = |outcomes: Vec<StatementOutcome>, stopped| ScriptOutcome {
+            results: outcomes.into_iter().map(result).collect(),
+            stopped,
+            ..ScriptOutcome::default()
+        };
+        assert!(of(vec![done.clone(), done.clone()], false).succeeded());
+        assert!(of(Vec::new(), false).succeeded());
+        assert!(!of(vec![done.clone(), failed], false).succeeded());
+        assert!(!of(vec![done.clone(), StatementOutcome::Cancelled], true).succeeded());
+        // Stopped after its last statement.
+        assert!(!of(vec![done], true).succeeded());
+    }
+
     #[test]
     fn a_failed_start_or_end_names_the_modes_transaction() {
         let error = Error::query("no");
```

- [ ] **Step 2: Run them to see them fail**

Run: `~/.cargo/bin/cargo test --locked -p tabletist-db --test sqlite`
Expected: FAIL. Six tests panic with ``called `Result::unwrap()` on an `Err` value: Unsupported("read-write runs on SQLite")``: `a_run_that_writes_is_committed_and_counts_its_rows`, `a_statement_that_fails_rolls_the_whole_run_back`, `a_run_that_writes_and_is_stopped_is_rolled_back`, `rows_a_write_returns_are_cut_at_the_limit_and_every_row_is_written`, `a_commit_that_fails_writes_nothing_and_the_next_run_can_begin`, `a_run_that_writes_nothing_is_committed_all_the_same`.
(The unit tests do not compile yet: `ScriptEnd` is not imported in `sqlite.rs`, and `succeeded` and `put_back` do not exist. `--test sqlite` builds only the integration tests.)

- [ ] **Step 3: Write the run**

`script.rs`:

```diff
diff --git a/crates/tabletist-db/src/script.rs b/crates/tabletist-db/src/script.rs
index a2187cf..74333f6 100644
--- a/crates/tabletist-db/src/script.rs
+++ b/crates/tabletist-db/src/script.rs
@@ -90,6 +90,30 @@ impl ScriptOutcome {
                 .iter()
                 .any(|result| result.outcome == StatementOutcome::Cancelled)
     }
+
+    /// Whether every statement that started ran to its end: what a run
+    /// that writes needs before it commits.
+    pub(crate) fn succeeded(&self) -> bool {
+        !self.stopped
+            && self.results.iter().all(|result| {
+                matches!(
+                    result.outcome,
+                    StatementOutcome::Rows { .. } | StatementOutcome::Done { .. }
+                )
+            })
+    }
+
+    /// The outcome of a run that writes, once its transaction has ended
+    /// and the session was put back, or could not be. The end is known
+    /// either way, so it is told: a run that is committed is never shown
+    /// as one that may be. A session that could not be put back is closed
+    /// for it (`broken`).
+    pub(crate) fn put_back(mut self, session: Result<()>) -> Self {
+        if let Err(error) = session {
+            self.broken = Some(cleanup_failed(ScriptMode::Write, &error));
+        }
+        self
+    }
 }
 
 /// One statement's outcome and how long it took.
```

`sqlite.rs`. `script` and `statements` take the mode; `begin` and `end_write` are new; the placeholder in `Conn::run_script` goes:

```diff
diff --git a/crates/tabletist-db/src/sqlite.rs b/crates/tabletist-db/src/sqlite.rs
index 1bd95fb..2210ac1 100644
--- a/crates/tabletist-db/src/sqlite.rs
+++ b/crates/tabletist-db/src/sqlite.rs
@@ -15,8 +15,8 @@ use rusqlite::{ErrorCode, OpenFlags};
 use crate::script::cleanup_failed;
 use crate::{
     Access, ColumnInfo, ColumnMeta, Dialect, Error, ForeignKeyInfo, IndexInfo, MAX_LISTED,
-    ObjectInfo, ObjectKind, ObjectRef, Result, RowPage, RowQuery, ScriptMode, ScriptOutcome,
-    StatementOutcome, StatementResult, StopFlag, Structure, Value, ValueKind,
+    ObjectInfo, ObjectKind, ObjectRef, Result, RowPage, RowQuery, ScriptEnd, ScriptMode,
+    ScriptOutcome, StatementOutcome, StatementResult, StopFlag, Structure, Value, ValueKind,
 };
 
 mod fence;
@@ -79,16 +79,36 @@ fn still_query_only(connection: &rusqlite::Connection) -> Result<bool> {
         .map_err(map_error)
 }
 
-/// Runs `texts` between `BEGIN` and a `ROLLBACK` that always happens, each
-/// behind `Fence::Script`, which denies it leaving the transaction or
-/// turning `query_only` off. Behind the fence, a run that finds `query_only`
-/// off, before a statement or at its end, or its transaction gone before a
-/// statement, ends with `LeftReadOnly` after that rollback.
+/// Starts a script's transaction. A run that writes lifts `query_only`
+/// for its length and takes the write lock at once, so a database another
+/// program holds is found before any statement ran. Both are the app's own
+/// statements, outside the fence.
+fn begin(connection: &rusqlite::Connection, mode: ScriptMode) -> Result<()> {
+    let sql = match mode {
+        ScriptMode::ReadOnly => "BEGIN DEFERRED",
+        ScriptMode::Write => "PRAGMA query_only = OFF; BEGIN IMMEDIATE",
+    };
+    connection.execute_batch(sql).map_err(map_error)
+}
+
+/// Runs `texts` in one transaction, each behind `Fence::Script`, which
+/// denies it leaving the transaction or setting `query_only`.
+///
+/// A read-only run sits between `BEGIN` and a `ROLLBACK` that always
+/// happens. Behind the fence, a run that finds `query_only` off, before a
+/// statement or at its end, or its transaction gone before a statement,
+/// ends with `LeftReadOnly` after that rollback.
+///
+/// A run that writes has `query_only` off from `BEGIN` to its end, which
+/// is a `COMMIT` when every statement succeeded (see `end_write`). One
+/// that finds its transaction gone before a statement ends with
+/// `LeftTransaction`.
 fn script(
     connection: &rusqlite::Connection,
     fences: &Fences,
     texts: &[String],
     limit: usize,
+    mode: ScriptMode,
     stop: &StopFlag,
 ) -> Result<ScriptOutcome> {
     let mut outcome = ScriptOutcome::default();
@@ -98,18 +118,22 @@ fn script(
     }
     // A cancel (the session's interrupt) can land while BEGIN runs; that
     // ends the run with no results.
-    match connection
-        .execute_batch("BEGIN DEFERRED")
-        .map_err(map_error)
-    {
+    match begin(connection, mode) {
         Err(Error::Cancelled) => {
             outcome.stopped = true;
             // The interrupt may have left a transaction open.
             stop.finish();
-            end_transaction(connection)
-                .map_err(|error| cleanup_failed(ScriptMode::ReadOnly, &error))?;
+            end_transaction(connection).map_err(|error| cleanup_failed(mode, &error))?;
             return Ok(outcome);
         }
+        // A database that is locked: the run's error, in SQLite's words.
+        // `query_only` was lifted before the lock was asked for, so the
+        // session's settings are put back; then it is as it was.
+        Err(error) if mode == ScriptMode::Write => {
+            stop.finish();
+            end_transaction(connection).map_err(|error| cleanup_failed(mode, &error))?;
+            return Err(error);
+        }
         Err(error) => return Err(error),
         Ok(()) => {}
     }
@@ -117,9 +141,12 @@ fn script(
     // the running statement, which fails with SQLITE_INTERRUPT (Cancelled).
     let watching = stop.clone();
     connection.progress_handler(1_000, Some(move || watching.is_stopped()));
-    let ran = statements(connection, fences, texts, limit, stop, &mut outcome);
+    let ran = statements(connection, fences, texts, limit, mode, stop, &mut outcome);
     // Removed before the cleanup, so a stop cannot interrupt it.
     connection.progress_handler(0, None::<fn() -> bool>);
+    if mode == ScriptMode::Write {
+        return end_write(connection, stop, ran, outcome);
+    }
     stop.finish();
     // Asked before the rollback, which puts the setting back: a last
     // statement that turned it off must not pass unseen. A cancel can land
@@ -137,6 +164,66 @@ fn script(
     Ok(outcome)
 }
 
+/// Ends a run that writes. Its transaction is committed when every
+/// statement succeeded and no stop came before this point, and rolled back
+/// otherwise; the session's settings are put back either way, `query_only`
+/// first. From `finish` on the backend sends no cancel, so the run ends as
+/// its commit ends.
+fn end_write(
+    connection: &rusqlite::Connection,
+    stop: &StopFlag,
+    ran: Result<()>,
+    mut outcome: ScriptOutcome,
+) -> Result<ScriptOutcome> {
+    let succeeded = ran.is_ok() && outcome.succeeded();
+    // The backend is told first, so a stop it sets from here on sends no
+    // cancel. One set before this line still wins over the commit.
+    stop.finish();
+    let stopped = stop.is_stopped();
+    // What ends the run with an error once the session is put back.
+    let mut failed = None;
+    if succeeded && stopped {
+        outcome.stopped = true;
+    } else if succeeded && connection.is_autocommit() {
+        // The last statement ended the transaction. No statement followed
+        // it, so the check before a statement never saw it.
+        failed = Some(Error::LeftTransaction);
+    } else if succeeded {
+        match connection.execute_batch("COMMIT").map_err(map_error) {
+            Ok(()) => outcome.end = ScriptEnd::Committed,
+            // An interrupt that was on its way landed on the commit, which
+            // SQLite then leaves open for the rollback below.
+            Err(Error::Cancelled) if !connection.is_autocommit() => outcome.stopped = true,
+            // Interrupted, and the transaction is gone: which way it went
+            // is not known.
+            Err(Error::Cancelled) => {
+                failed = Some(cleanup_failed(ScriptMode::Write, &Error::Cancelled));
+            }
+            // SQLite keeps the transaction open when a COMMIT fails (a
+            // deferred foreign key, a reader holding the file): the
+            // rollback below is what makes "nothing is written" true.
+            Err(error) => {
+                outcome.end = ScriptEnd::CommitFailed {
+                    error,
+                    committed: 0,
+                };
+            }
+        }
+    }
+    let ended = end_transaction(connection);
+    ran?;
+    if let Some(error) = failed {
+        return Err(error);
+    }
+    // A rollback that did not happen leaves the end unknown.
+    if let Err(error) = &ended
+        && !connection.is_autocommit()
+    {
+        return Err(cleanup_failed(ScriptMode::Write, error));
+    }
+    Ok(outcome.put_back(ended))
+}
+
 /// Rolls back what is still open and puts the connect-time settings back
 /// (a script may have changed them). A cancel can interrupt the cleanup
 /// itself, so it gets one more try.
@@ -157,6 +244,7 @@ fn statements(
     fences: &Fences,
     texts: &[String],
     limit: usize,
+    mode: ScriptMode,
     stop: &StopFlag,
     outcome: &mut ScriptOutcome,
 ) -> Result<()> {
@@ -171,25 +259,32 @@ fn statements(
         }
         // The open transaction is what stops what `query_only` lets
         // through (a change of journal mode, the empty file a VACUUM INTO
-        // leaves). The statement before this one succeeded, so SQLite did
-        // not end it over an error: the script did.
+        // leaves), and in a run that writes it is what the run commits.
+        // The statement before this one succeeded, so SQLite did not end
+        // it over an error: the script did.
         if connection.is_autocommit() {
-            return Err(Error::LeftReadOnly);
+            return Err(match mode {
+                ScriptMode::ReadOnly => Error::LeftReadOnly,
+                ScriptMode::Write => Error::LeftTransaction,
+            });
         }
-        match still_query_only(connection) {
-            Ok(true) => {}
-            // A stop that landed on the check: this statement is the
-            // cancelled one.
-            Err(Error::Cancelled) => {
-                outcome.stopped = true;
-                outcome.results.push(StatementResult {
-                    elapsed: std::time::Duration::ZERO,
-                    outcome: StatementOutcome::Cancelled,
-                });
-                break;
+        // A run that writes has `query_only` off itself.
+        if mode == ScriptMode::ReadOnly {
+            match still_query_only(connection) {
+                Ok(true) => {}
+                // A stop that landed on the check: this statement is the
+                // cancelled one.
+                Err(Error::Cancelled) => {
+                    outcome.stopped = true;
+                    outcome.results.push(StatementResult {
+                        elapsed: std::time::Duration::ZERO,
+                        outcome: StatementOutcome::Cancelled,
+                    });
+                    break;
+                }
+                // No answer counts as left, as at the end of the run.
+                Ok(false) | Err(_) => return Err(Error::LeftReadOnly),
             }
-            // No answer counts as left, as at the end of the run.
-            Ok(false) | Err(_) => return Err(Error::LeftReadOnly),
         }
         let started = Instant::now();
         // Only the script's own text is fenced: the checks above and the
@@ -636,9 +731,6 @@ impl Conn {
         mode: ScriptMode,
         stop: &StopFlag,
     ) -> Result<ScriptOutcome> {
-        if mode == ScriptMode::Write {
-            return Err(Error::Unsupported("read-write runs on SQLite"));
-        }
         let stop = stop.clone();
         let limit = limit as usize;
         // If the caller drops this future, `run` interrupts the statement
@@ -646,7 +738,7 @@ impl Conn {
         let guard = StopOnDrop(Some(stop.clone()));
         let fences = self.fences.clone();
         let outcome = self
-            .run(move |connection| script(connection, &fences, &texts, limit, &stop))
+            .run(move |connection| script(connection, &fences, &texts, limit, mode, &stop))
             .await;
         guard.disarm();
         outcome
```

- [ ] **Step 4: Run them to see them pass**

Run: `~/.cargo/bin/cargo test --locked -p tabletist-db`
Expected: PASS: 294 unit tests, 56 in `tests/sqlite.rs`.
Run the four checks. Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/tabletist-db
git commit -m "Run a script that writes on SQLite

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 5: The run that writes, on PostgreSQL

`pg/script/write.rs` holds the run; it reuses the read-only run's `statements`, `in_transaction` (the guard's savepoint swap), `own_failed`, `rollback`, `unlock` and `unlocked` from `pg/script.rs`, its parent module, which is why none of them changes visibility.

What differs from the read-only run:

- **`BEGIN`, plain.** No `READ ONLY`, no snapshot step. The transaction takes the server's and the role's defaults.
- **No cursor.** `run_statement` takes the mode and runs a row query through a cursor only in a read-only run. A cursor runs its query as far as it is fetched: in a run that commits, `SELECT refill(id) FROM shelves` would do its work for `limit + 1` rows and keep that. A run that writes streams every statement with columns to its end and drops the rows past `limit + 1`, as the read-only run already does for `SHOW`.
- **The close.** `confirm` swaps the savepoint once more; that is also what makes an answered `COMMIT` mean committed (in a failed block PostgreSQL answers `COMMIT` with a rollback and no error, and the driver does not show the tag). Then `stop.finish()`, then `COMMIT`. A `COMMIT` that errors: a cancel means stopped and rolled back, a lost connection is the run's `Err`, anything else is `CommitFailed`.
- **`put_back`.** A committed transaction keeps what a rolled back one undid: session settings, the role, `WITH HOLD` cursors, `LISTEN`. `CLOSE ALL; UNLISTEN *; RESET SESSION AUTHORIZATION; RESET ROLE; RESET ALL`, then the connect-time settings again (`session_setup`, moved out of `connect` so both send the same text), then the advisory locks. Never `DISCARD ALL`: it drops the prepared statements tokio-postgres keeps on the server.

**Facts this task rests on, not run where the plan was written.** Check each when a test disagrees:
- `RELEASE tabletist_guard; SAVEPOINT tabletist_guard` fails outside a transaction block and in one chained to by `COMMIT AND CHAIN` (the read-only run's tests already prove this).
- A `COMMIT` that fails over a deferred constraint leaves the session outside a transaction.
- `RESET ALL` leaves `role` and `session_authorization` alone.
- A multi-statement `batch_execute` of `CLOSE ALL; UNLISTEN *; ...` is accepted outside a transaction block.
- `SET ROLE "<current user>"` is allowed, and `SHOW role` then reads that name and `none` after `RESET ROLE`.

**Files:**
- Create: `crates/tabletist-db/src/pg/script/write.rs`
- Modify: `crates/tabletist-db/src/pg.rs`, `crates/tabletist-db/src/pg/script.rs`
- Test: `crates/tabletist-db/tests/postgres.rs`, and `write.rs`'s own `mod tests`

- [ ] **Step 1: Write the failing tests**

At the end of `tests/postgres.rs`, and `ScriptEnd` in its imports. Each test has a table of its own, made by a second connection (`admin`), since the tests of a file run at once:

```diff
diff --git a/crates/tabletist-db/tests/postgres.rs b/crates/tabletist-db/tests/postgres.rs
index 369b86e..83b0fe7 100644
--- a/crates/tabletist-db/tests/postgres.rs
+++ b/crates/tabletist-db/tests/postgres.rs
@@ -570,7 +570,7 @@ async fn a_running_query_can_be_cancelled() {
     assert!(connection.fetch_rows(&users(1)).await.is_ok());
 }
 
-use tabletist_db::{Dialect, ScriptMode, ScriptOutcome, StatementOutcome, StopFlag};
+use tabletist_db::{Dialect, ScriptEnd, ScriptMode, ScriptOutcome, StatementOutcome, StopFlag};
 
 /// A writable session for the bypass test's probe table.
 async fn admin() -> tokio_postgres::Client {
@@ -1301,3 +1301,408 @@ async fn the_server_version_has_no_distribution_suffix() {
     assert!(version.starts_with("PostgreSQL "), "{version}");
     assert!(!version.contains('('), "{version}");
 }
+
+/// Runs `text` as a script that writes, with a fresh stop flag.
+async fn write(
+    connection: &Connection,
+    text: &str,
+    limit: u32,
+) -> tabletist_db::Result<ScriptOutcome> {
+    within(connection.run_script(&script(text), limit, ScriptMode::Write, &StopFlag::new())).await
+}
+
+/// An empty table of this name with one column `n`, made from outside the
+/// session under test. Each test has a table of its own: they run at once.
+async fn scratch(admin: &tokio_postgres::Client, table: &str) {
+    admin
+        .batch_execute(&format!(
+            "DROP TABLE IF EXISTS {table} CASCADE; CREATE TABLE {table} (n int)"
+        ))
+        .await
+        .unwrap();
+}
+
+/// A count, asked from outside the session under test.
+async fn counted(admin: &tokio_postgres::Client, sql: &str) -> i64 {
+    admin.query_one(sql, &[]).await.unwrap().get(0)
+}
+
+/// What each statement of a run did, as its row count, or `None` for one
+/// that has none or did not end well.
+fn affected(outcome: &ScriptOutcome) -> Vec<Option<u64>> {
+    outcome
+        .results
+        .iter()
+        .map(|result| match result.outcome {
+            StatementOutcome::Done { affected, .. } => affected,
+            _ => None,
+        })
+        .collect()
+}
+
+/// The first cell of a query's only row, read in a read-only run.
+async fn shown(connection: &Connection, sql: &str) -> Value {
+    let outcome = run(connection, sql, 1).await.unwrap();
+    match &outcome.results[0].outcome {
+        StatementOutcome::Rows { rows, .. } => rows[0][0].clone(),
+        other => panic!("{sql}: {other:?}"),
+    }
+}
+
+/// Whether the session refuses a write in a read-only run, as it must
+/// after every run that writes.
+async fn is_fenced(connection: &Connection, table: &str) -> bool {
+    let outcome = run(connection, &format!("DELETE FROM {table}"), 1)
+        .await
+        .unwrap();
+    matches!(
+        &outcome.results[0].outcome,
+        StatementOutcome::Error {
+            error: Error::Query { code: Some(code), .. },
+            ..
+        } if code == "25006"
+    )
+}
+
+#[tokio::test]
+async fn a_run_that_writes_is_refused_on_a_read_only_connection() {
+    let Some(connection) = connect_as(Access::ReadOnly).await else {
+        return;
+    };
+    for text in ["DELETE FROM users", "SELECT 1", "COMMIT"] {
+        assert_eq!(
+            write(&connection, text, 10).await,
+            Err(Error::ReadOnly),
+            "{text}"
+        );
+    }
+}
+
+#[tokio::test]
+async fn a_run_that_writes_is_committed_and_counts_its_rows() {
+    let Some(connection) = connect_as(Access::Writable).await else {
+        return;
+    };
+    let admin = admin().await;
+    scratch(&admin, "write_counts").await;
+    admin
+        .batch_execute("DROP TABLE IF EXISTS write_counts_made")
+        .await
+        .unwrap();
+    let outcome = write(
+        &connection,
+        "INSERT INTO write_counts SELECT generate_series(1, 5);
+         UPDATE write_counts SET n = n + 10 WHERE n > 2;
+         DELETE FROM write_counts WHERE n = 1;
+         CREATE TABLE write_counts_made (id int)",
+        10,
+    )
+    .await
+    .unwrap();
+    assert_eq!(outcome.end, ScriptEnd::Committed);
+    assert!(!outcome.was_cancelled());
+    assert_eq!(outcome.broken, None);
+    assert_eq!(affected(&outcome), [Some(5), Some(3), Some(1), None]);
+    assert_eq!(
+        counted(&admin, "SELECT count(*) FROM write_counts").await,
+        4
+    );
+    assert_eq!(
+        counted(&admin, "SELECT count(*) FROM write_counts_made").await,
+        0
+    );
+    assert!(is_fenced(&connection, "write_counts").await);
+    admin
+        .batch_execute("DROP TABLE write_counts, write_counts_made")
+        .await
+        .unwrap();
+}
+
+#[tokio::test]
+async fn a_statement_that_fails_rolls_the_whole_run_back() {
+    let Some(connection) = connect_as(Access::Writable).await else {
+        return;
+    };
+    let admin = admin().await;
+    scratch(&admin, "write_fails").await;
+    let outcome = write(
+        &connection,
+        "INSERT INTO write_fails VALUES (1);
+         CREATE TABLE write_fails_made (id int);
+         INSERT INTO write_fails_missing VALUES (1);
+         INSERT INTO write_fails VALUES (2)",
+        10,
+    )
+    .await
+    .unwrap();
+    assert_eq!(outcome.end, ScriptEnd::RolledBack);
+    // The statement after the error never ran.
+    assert_eq!(outcome.results.len(), 3);
+    assert!(matches!(
+        outcome.results[2].outcome,
+        StatementOutcome::Error { .. }
+    ));
+    assert_eq!(counted(&admin, "SELECT count(*) FROM write_fails").await, 0);
+    let made = "SELECT count(*) FROM pg_class WHERE relname = 'write_fails_made'";
+    assert_eq!(counted(&admin, made).await, 0);
+    assert!(is_fenced(&connection, "write_fails").await);
+    admin.batch_execute("DROP TABLE write_fails").await.unwrap();
+}
+
+#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
+async fn a_run_that_writes_and_is_cancelled_is_rolled_back() {
+    let Some(connection) = connect_as(Access::Writable).await else {
+        return;
+    };
+    let connection = std::sync::Arc::new(connection);
+    let admin = admin().await;
+    scratch(&admin, "write_cancelled").await;
+    let cancel = connection.cancel_handle();
+    let stop = StopFlag::new();
+    let running = {
+        let connection = std::sync::Arc::clone(&connection);
+        let stop = stop.clone();
+        tokio::spawn(async move {
+            let text = "INSERT INTO write_cancelled VALUES (1); \
+                        SELECT pg_sleep(30) /* tabletist write cancelled */";
+            connection
+                .run_script(&script(text), 10, ScriptMode::Write, &stop)
+                .await
+        })
+    };
+    runs_on_the_server(&admin, "/* tabletist write cancelled */").await;
+    stop.stop();
+    let deadline = std::time::Instant::now() + Duration::from_secs(15);
+    // As the backend does: repeat the cancel until the cleanup begins.
+    while !stop.is_finishing() && !running.is_finished() {
+        assert!(
+            std::time::Instant::now() < deadline,
+            "cancel must stop the script"
+        );
+        cancel.cancel().await.unwrap();
+        tokio::time::sleep(Duration::from_millis(100)).await;
+    }
+    let outcome = within(running).await.unwrap().unwrap();
+    assert!(outcome.was_cancelled());
+    assert_eq!(outcome.end, ScriptEnd::RolledBack);
+    assert_eq!(affected(&outcome), [Some(1), None]);
+    assert_eq!(
+        counted(&admin, "SELECT count(*) FROM write_cancelled").await,
+        0
+    );
+    // The session survives, and is fenced.
+    assert!(is_fenced(&connection, "write_cancelled").await);
+    admin
+        .batch_execute("DROP TABLE write_cancelled")
+        .await
+        .unwrap();
+}
+
+#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
+async fn a_stop_between_the_statements_of_a_run_that_writes_rolls_it_back() {
+    let Some(connection) = connect_as(Access::Writable).await else {
+        return;
+    };
+    let connection = std::sync::Arc::new(connection);
+    let admin = admin().await;
+    scratch(&admin, "write_stopped").await;
+    let stop = StopFlag::new();
+    let running = {
+        let connection = std::sync::Arc::clone(&connection);
+        let stop = stop.clone();
+        tokio::spawn(async move {
+            let text = "INSERT INTO write_stopped VALUES (1); \
+                        DO $$ BEGIN /* tabletist write stopped */ PERFORM pg_sleep(1); END $$; \
+                        INSERT INTO write_stopped VALUES (2)";
+            connection
+                .run_script(&script(text), 10, ScriptMode::Write, &stop)
+                .await
+        })
+    };
+    // A stop without a cancel: the statement that runs ends by itself, the
+    // one after it never starts, and nothing is committed.
+    runs_on_the_server(&admin, "/* tabletist write stopped */").await;
+    stop.stop();
+    let outcome = within(running).await.unwrap().unwrap();
+    assert_eq!(outcome.results.len(), 3);
+    assert_eq!(outcome.results[2].outcome, StatementOutcome::Cancelled);
+    assert_eq!(outcome.end, ScriptEnd::RolledBack);
+    assert_eq!(
+        counted(&admin, "SELECT count(*) FROM write_stopped").await,
+        0
+    );
+    admin
+        .batch_execute("DROP TABLE write_stopped")
+        .await
+        .unwrap();
+}
+
+#[tokio::test]
+async fn every_row_is_written_whatever_the_limit_keeps() {
+    let Some(connection) = connect_as(Access::Writable).await else {
+        return;
+    };
+    let admin = admin().await;
+    scratch(&admin, "write_limit").await;
+    admin
+        .batch_execute("DROP SEQUENCE IF EXISTS write_limit_seq; CREATE SEQUENCE write_limit_seq")
+        .await
+        .unwrap();
+    let cut = |outcome: &ScriptOutcome, index: usize| {
+        matches!(
+            &outcome.results[index].outcome,
+            StatementOutcome::Rows { rows, truncated: true, .. } if rows.len() == 10
+        )
+    };
+    let outcome = write(
+        &connection,
+        "INSERT INTO write_limit SELECT generate_series(1, 50) RETURNING n;
+         SELECT nextval('write_limit_seq') FROM generate_series(1, 50)",
+        10,
+    )
+    .await
+    .unwrap();
+    assert_eq!(outcome.end, ScriptEnd::Committed);
+    assert!(cut(&outcome, 0), "{:?}", outcome.results[0].outcome);
+    assert_eq!(
+        counted(&admin, "SELECT count(*) FROM write_limit").await,
+        50
+    );
+    // A row query runs to its end too: no cursor stops it at the limit.
+    assert!(cut(&outcome, 1), "{:?}", outcome.results[1].outcome);
+    assert_eq!(
+        counted(&admin, "SELECT last_value FROM write_limit_seq").await,
+        50
+    );
+    // A data-modifying WITH returns rows, and DECLARE would refuse it.
+    let outcome = write(
+        &connection,
+        "WITH gone AS (DELETE FROM write_limit WHERE n <= 20 RETURNING n) \
+         SELECT count(*) FROM gone",
+        10,
+    )
+    .await
+    .unwrap();
+    assert_eq!(outcome.end, ScriptEnd::Committed);
+    assert!(matches!(
+        &outcome.results[0].outcome,
+        StatementOutcome::Rows { rows, .. } if rows == &[vec![Value::Int(20)]]
+    ));
+    assert_eq!(
+        counted(&admin, "SELECT count(*) FROM write_limit").await,
+        30
+    );
+    admin
+        .batch_execute("DROP TABLE write_limit; DROP SEQUENCE write_limit_seq")
+        .await
+        .unwrap();
+}
+
+#[tokio::test]
+async fn a_commit_that_fails_writes_nothing_and_the_next_run_can_begin() {
+    let Some(connection) = connect_as(Access::Writable).await else {
+        return;
+    };
+    let admin = admin().await;
+    admin
+        .batch_execute(
+            "DROP TABLE IF EXISTS write_slots, write_shelves;
+             CREATE TABLE write_shelves (id int PRIMARY KEY);
+             CREATE TABLE write_slots (
+                 id int PRIMARY KEY,
+                 shelf_id int REFERENCES write_shelves (id) DEFERRABLE INITIALLY DEFERRED
+             )",
+        )
+        .await
+        .unwrap();
+    // The statement succeeds; the commit finds the shelf missing.
+    let outcome = write(&connection, "INSERT INTO write_slots VALUES (1, 99)", 10)
+        .await
+        .unwrap();
+    assert_eq!(affected(&outcome), [Some(1)]);
+    assert!(
+        matches!(
+            &outcome.end,
+            ScriptEnd::CommitFailed {
+                error: Error::Query { code: Some(code), .. },
+                committed: 0,
+            } if code == "23503"
+        ),
+        "{:?}",
+        outcome.end
+    );
+    assert_eq!(outcome.broken, None);
+    assert_eq!(counted(&admin, "SELECT count(*) FROM write_slots").await, 0);
+    let outcome = write(
+        &connection,
+        "INSERT INTO write_shelves VALUES (99); INSERT INTO write_slots VALUES (1, 99)",
+        10,
+    )
+    .await
+    .unwrap();
+    assert_eq!(outcome.end, ScriptEnd::Committed);
+    assert_eq!(counted(&admin, "SELECT count(*) FROM write_slots").await, 1);
+    admin
+        .batch_execute("DROP TABLE write_slots, write_shelves")
+        .await
+        .unwrap();
+}
+
+/// What a script can leave in a session, read back in read-only runs: the
+/// search path, the role, the open cursors, the advisory locks and the
+/// channels listened on.
+async fn session_state(connection: &Connection) -> [Value; 5] {
+    [
+        shown(connection, "SHOW search_path").await,
+        shown(connection, "SHOW role").await,
+        shown(connection, "SELECT count(*) FROM pg_cursors").await,
+        shown(
+            connection,
+            "SELECT count(*) FROM pg_locks WHERE locktype = 'advisory' AND pid = pg_backend_pid()",
+        )
+        .await,
+        shown(connection, "SELECT count(*) FROM pg_listening_channels()").await,
+    ]
+}
+
+#[tokio::test]
+async fn a_run_that_writes_leaves_the_session_as_it_connected() {
+    let Some(connection) = connect_as(Access::Writable).await else {
+        return;
+    };
+    let admin = admin().await;
+    scratch(&admin, "write_session").await;
+    let connected = session_state(&connection).await;
+    let Value::Text(user) = shown(&connection, "SELECT current_user").await else {
+        panic!("the user's name is text");
+    };
+    // What a committed transaction keeps, and a rolled back one does not.
+    let leaves = format!(
+        "SET search_path = pg_catalog;
+         SET ROLE \"{user}\";
+         DECLARE held CURSOR WITH HOLD FOR SELECT 1;
+         SELECT pg_advisory_lock(4242);
+         LISTEN tabletist_write_session;
+         INSERT INTO public.write_session VALUES (1)"
+    );
+    let outcome = write(&connection, &leaves, 10).await.unwrap();
+    assert_eq!(outcome.end, ScriptEnd::Committed);
+    assert_eq!(outcome.broken, None);
+    assert_eq!(session_state(&connection).await, connected, "committed");
+    assert!(is_fenced(&connection, "write_session").await);
+    // After a run that failed, and so was rolled back.
+    let outcome = write(&connection, &format!("{leaves}; SELECT 1 / 0"), 10)
+        .await
+        .unwrap();
+    assert_eq!(outcome.end, ScriptEnd::RolledBack);
+    assert_eq!(session_state(&connection).await, connected, "rolled back");
+    assert!(is_fenced(&connection, "write_session").await);
+    assert_eq!(
+        counted(&admin, "SELECT count(*) FROM write_session").await,
+        1
+    );
+    admin
+        .batch_execute("DROP TABLE write_session")
+        .await
+        .unwrap();
+}
```

- [ ] **Step 2: Run them to see them fail**

Run: `~/.cargo/bin/cargo test --locked -p tabletist-db --test postgres`
Expected, with `TABLETIST_TEST_PG_URL` set: FAIL, the new tests panicking with ``Unsupported("read-write runs on PostgreSQL")`` (`a_run_that_writes_is_refused_on_a_read_only_connection` passes: task 3 made it so). Without the variable they print "skipped" and pass, which proves nothing.

- [ ] **Step 3: Write the run**

`pg.rs`: the connect-time settings in one function.

```diff
diff --git a/crates/tabletist-db/src/pg.rs b/crates/tabletist-db/src/pg.rs
index 603b1f4..aefc4f0 100644
--- a/crates/tabletist-db/src/pg.rs
+++ b/crates/tabletist-db/src/pg.rs
@@ -154,6 +154,20 @@ pub(crate) fn connect_error(error: tokio_postgres::Error) -> Error {
     Error::Connect(text)
 }
 
+/// The settings a session has from the start. A writable session keeps
+/// the server's default: row fetches, counts and a script that only reads
+/// open read-only transactions of their own, and none of the script
+/// guard's checks read the session's default. A script that writes sends
+/// these again after its run, which resets every setting.
+fn session_setup(access: Access) -> &'static str {
+    match access {
+        Access::ReadOnly => {
+            "SET default_transaction_read_only = on; SET standard_conforming_strings = on"
+        }
+        Access::Writable => "SET standard_conforming_strings = on",
+    }
+}
+
 pub(crate) fn query_error(error: tokio_postgres::Error) -> Error {
     if error.is_closed() {
         return Error::ConnectionLost(describe(&error));
@@ -258,17 +272,10 @@ impl Conn {
                 log::info!("PostgreSQL connection ended: {error}");
             }
         });
-        // A writable session keeps the server's default. Row fetches,
-        // counts and the script runner open read-only transactions of
-        // their own, and none of the script guard's checks read the
-        // session's default.
-        let setup = match access {
-            Access::ReadOnly => {
-                "SET default_transaction_read_only = on; SET standard_conforming_strings = on"
-            }
-            Access::Writable => "SET standard_conforming_strings = on",
-        };
-        client.batch_execute(setup).await.map_err(query_error)?;
+        client
+            .batch_execute(session_setup(access))
+            .await
+            .map_err(query_error)?;
         let cancel = client.cancel_token();
         Ok(Self {
             client: tokio::sync::Mutex::new(client),
```

`pg/script.rs`: the module, the branch, and the mode through `statements` and `run_statement`.

```diff
diff --git a/crates/tabletist-db/src/pg/script.rs b/crates/tabletist-db/src/pg/script.rs
index 63d187a..bcb37a6 100644
--- a/crates/tabletist-db/src/pg/script.rs
+++ b/crates/tabletist-db/src/pg/script.rs
@@ -1,6 +1,8 @@
-//! Running a SQL editor script on PostgreSQL: one read-only transaction
-//! managed by hand, the guard's savepoint around every statement, and the
-//! close that asks the server where the session stands.
+//! Running a SQL editor script on PostgreSQL: one transaction managed by
+//! hand, the guard's savepoint around every statement, and the close that
+//! asks the server where the session stands. This file is the run that
+//! only reads, in a read-only transaction that is rolled back; `write` is
+//! the run that commits.
 
 use std::time::{Duration, Instant};
 
@@ -14,6 +16,8 @@ use crate::{
     StatementResult, StopFlag,
 };
 
+mod write;
+
 impl Conn {
     /// See [`crate::Connection::run_script`]. The transaction is managed by
     /// hand: `tokio_postgres::Transaction` has no streaming simple query.
@@ -28,14 +32,16 @@ impl Conn {
         mode: ScriptMode,
         stop: &StopFlag,
     ) -> Result<ScriptOutcome> {
-        if mode == ScriptMode::Write {
-            return Err(Error::Unsupported("read-write runs on PostgreSQL"));
-        }
         let client = self.client.lock().await;
+        if mode == ScriptMode::Write {
+            return write::run(&client, texts, limit as usize, stop).await;
+        }
         let mut outcome = ScriptOutcome::default();
         let tx = match open(&client).await {
             // A lost session ends the run here: nothing to close.
-            Ok(Tx::Open) => statements(&client, texts, limit as usize, stop, &mut outcome).await?,
+            Ok(Tx::Open) => {
+                statements(&client, texts, limit as usize, mode, stop, &mut outcome).await?
+            }
             // A cancel landed on the opening queries: no results.
             Ok(tx) => {
                 outcome.stopped = true;
@@ -103,6 +109,7 @@ async fn statements(
     client: &tokio_postgres::Client,
     texts: &[String],
     limit: usize,
+    mode: ScriptMode,
     stop: &StopFlag,
     outcome: &mut ScriptOutcome,
 ) -> Result<Tx> {
@@ -143,7 +150,7 @@ async fn statements(
             return Ok(tx);
         }
         let started = Instant::now();
-        let (result, tx) = run_statement(client, text, limit, stop).await?;
+        let (result, tx) = run_statement(client, text, limit, mode, stop).await?;
         outcome.stopped |= result == StatementOutcome::Cancelled;
         let last = !matches!(
             result,
@@ -378,6 +385,7 @@ async fn run_statement(
     client: &tokio_postgres::Client,
     text: &str,
     limit: usize,
+    mode: ScriptMode,
     stop: &StopFlag,
 ) -> Result<(StatementOutcome, Tx)> {
     // The driver cannot put a NUL in a message, and fails in a way that
@@ -422,7 +430,12 @@ async fn run_statement(
         };
     }
     let mut kept = Vec::new();
-    if cursor_statement(text) {
+    // Only in a run that reads: a cursor runs its query as far as it is
+    // fetched and no further, so in a run that commits, `SELECT
+    // refill(id) FROM shelves` would do its work for `limit + 1` rows and
+    // keep that. There every statement runs to its end, and `DECLARE`
+    // would refuse a data-modifying `WITH` anyway.
+    if mode == ScriptMode::ReadOnly && cursor_statement(text) {
         // Its own call, and a newline, so a trailing -- comment ends there.
         let declare = format!("{CURSOR_PREFIX}{text}\n");
         if let Err(error) = client.batch_execute(&declare).await {
@@ -445,7 +458,8 @@ async fn run_statement(
             _ => None,
         }));
     } else {
-        // SHOW, EXPLAIN and the like: stream, keep limit + 1, drop the rest.
+        // SHOW, EXPLAIN and the like, and every statement with rows of a
+        // run that writes: stream, keep limit + 1, drop the rest.
         use futures_util::StreamExt;
         let stream = match client.simple_query_raw(text).await {
             Ok(stream) => stream,
```

Create `crates/tabletist-db/src/pg/script/write.rs`:

```rust
//! A SQL editor script that writes, on PostgreSQL: one read-write
//! transaction, the guard's savepoint before every statement, a commit when
//! every statement succeeded, and the session put back afterwards.

use super::super::session_setup;
use super::{Tx, in_transaction, own_failed, rollback, statements, unlock, unlocked};
use crate::pg::query_error;
use crate::script::{cannot_start, cleanup_failed, retry_cancelled};
use crate::{Access, Error, Result, ScriptEnd, ScriptMode, ScriptOutcome, StopFlag};

/// See [`crate::Connection::run_script`], for [`ScriptMode::Write`].
///
/// The future must be awaited to its end, as the read-only run's.
pub(super) async fn run(
    client: &tokio_postgres::Client,
    texts: &[String],
    limit: usize,
    stop: &StopFlag,
) -> Result<ScriptOutcome> {
    let mut outcome = ScriptOutcome::default();
    let tx = match open(client).await {
        // A lost session ends the run here: nothing to close.
        Ok(Tx::Open) => {
            statements(client, texts, limit, ScriptMode::Write, stop, &mut outcome).await?
        }
        // A cancel landed on the opening queries: no results.
        Ok(tx) => {
            outcome.stopped = true;
            tx
        }
        Err(error) if error.is_connection_lost() => return Err(error),
        // The transaction could not start. The next run would fail the
        // same way, so the session is closed, after an attempt to end what
        // is open.
        Err(error) => {
            stop.finish();
            rollback(client).await.ok();
            return Err(cannot_start(ScriptMode::Write, &error));
        }
    };
    close(client, tx, stop, outcome).await
}

/// Starts the script's transaction and sets the guard's savepoint (see
/// `in_transaction`). A plain `BEGIN`: the transaction takes the server's
/// and the role's defaults, so a role an administrator made read-only
/// stays so, and its error is the statement's. `Aborted` when a cancel
/// landed on these queries; an `Err` that is not a lost session means the
/// transaction could not start.
async fn open(client: &tokio_postgres::Client) -> Result<Tx> {
    for step in ["BEGIN", "SAVEPOINT tabletist_guard"] {
        match client.batch_execute(step).await.map_err(query_error) {
            Ok(()) => {}
            Err(Error::Cancelled) => return Ok(Tx::Aborted),
            Err(error) => return Err(error),
        }
    }
    Ok(Tx::Open)
}

/// Where a transaction the run believes open really stands, asked after
/// the last statement: the guard's swap once more. PostgreSQL never commits
/// by itself, so a transaction that is gone means a statement the refusal
/// missed ended it.
///
/// This is also what makes an answered `COMMIT` mean committed. In a
/// failed transaction PostgreSQL answers `COMMIT` with a rollback and no
/// error, and the driver does not show which; after a swap that worked the
/// transaction is not a failed one.
async fn confirm(client: &tokio_postgres::Client) -> Result<Tx> {
    match in_transaction(client).await {
        // A failed block, or a cancel on the check, which fails the block
        // if there is one: the check cannot say whose it is.
        Ok(Tx::Aborted) | Err(Error::Cancelled) => retry_cancelled!(own_failed(client)),
        other => other,
    }
}

/// Ends the run: commits when every statement succeeded, the transaction
/// is still the script's own and no stop came, and rolls back otherwise;
/// then puts the session back.
///
/// `Err` is a run whose end is not known, or a script that left its
/// transaction. Either closes the session (they count as a lost
/// connection).
async fn close(
    client: &tokio_postgres::Client,
    tx: Tx,
    stop: &StopFlag,
    mut outcome: ScriptOutcome,
) -> Result<ScriptOutcome> {
    // A run that will not commit runs nothing more of the script: the
    // backend is told to stop repeating its cancel before the check, which
    // one that is on its way can still land on.
    let committing = tx == Tx::Open && outcome.succeeded() && !stop.is_stopped();
    if !committing {
        stop.finish();
    }
    let tx = match tx {
        Tx::Open => confirm(client).await,
        known => Ok(known),
    };
    // The backend is told before the flag is read, so a stop it sets from
    // here on sends no cancel, which would land on the commit or the
    // cleanup. One set before this line still wins over the commit.
    stop.finish();
    let stopped = stop.is_stopped();
    let commit = committing && matches!(tx, Ok(Tx::Open)) && !stopped;
    if commit {
        match client.batch_execute("COMMIT").await.map_err(query_error) {
            Ok(()) => outcome.end = ScriptEnd::Committed,
            // A cancel that was on its way landed on the commit's own
            // work. The server has rolled the transaction back.
            Err(Error::Cancelled) => outcome.stopped = true,
            // Whether it went through cannot be known.
            Err(error) if error.is_connection_lost() => return Err(error),
            // A deferred constraint, a serialization failure. The server
            // has rolled the transaction back.
            Err(error) => {
                outcome.end = ScriptEnd::CommitFailed {
                    error,
                    committed: 0,
                };
            }
        }
    } else if outcome.succeeded() {
        match tx {
            // Stopped after the last statement.
            _ if stopped => outcome.stopped = true,
            // Every statement ran, and still the transaction failed: said
            // rather than a run that ends rolled back without a word.
            Ok(Tx::Aborted) => {
                outcome.end = ScriptEnd::CommitFailed {
                    error: Error::query("the transaction failed before it could be committed"),
                    committed: 0,
                };
            }
            _ => {}
        }
    }
    // Whatever is open is rolled back as far as that works, also when the
    // session is closed anyway. After a COMMIT that failed the server has
    // nothing left to roll back, and only warns.
    let rolled_back = match outcome.end {
        ScriptEnd::Committed => Ok(()),
        _ => retry_cancelled!(rollback(client)),
    };
    match (tx, rolled_back) {
        (Ok(Tx::Left), _) => Err(Error::LeftTransaction),
        (Err(error), _) | (_, Err(error)) => Err(cleanup_failed(ScriptMode::Write, &error)),
        (Ok(_), Ok(())) => Ok(outcome.put_back(put_back(client).await)),
    }
}

/// What a script can leave in a session once its transaction is committed,
/// undone: a rolled back transaction takes these with it, a committed one
/// keeps them, and none of it may reach table browsing or the next run.
/// `RESET ALL` leaves the role and the session's authorization alone, so
/// they are reset by name. Temporary tables last for the session.
const PUT_BACK: &str = "CLOSE ALL; UNLISTEN *; RESET SESSION AUTHORIZATION; RESET ROLE; RESET ALL";

/// Puts the session back as it connected: the script's cursors, channels,
/// role and settings gone, the connect-time settings set again, and the
/// session's advisory locks released. A step a cancel interrupted runs
/// once more.
async fn put_back(client: &tokio_postgres::Client) -> Result<()> {
    for step in [PUT_BACK, session_setup(Access::Writable)] {
        retry_cancelled!(execute(client, step))?;
    }
    unlocked(retry_cancelled!(unlock(client)))
}

async fn execute(client: &tokio_postgres::Client, sql: &str) -> Result<()> {
    client.batch_execute(sql).await.map_err(query_error)
}
```

- [ ] **Step 4: The driver's own tests**

They reach `open` and `close` and run past the refusal list. Three helpers of `pg/script.rs`'s tests become `pub(super)` for them, and one call there gains the mode:

```diff
diff --git a/crates/tabletist-db/src/pg/script.rs b/crates/tabletist-db/src/pg/script.rs
index 63d187a..bcb37a6 100644
--- a/crates/tabletist-db/src/pg/script.rs
+++ b/crates/tabletist-db/src/pg/script.rs
@@ -537,11 +551,13 @@ mod tests {
 
     /// One test at a time uses the `probe` table: creating it twice at once
     /// can fail, and one test's TRUNCATE would hide another's stray row.
-    static PROBE: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());
+    pub(super) static PROBE: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());
 
     /// A writable session with an empty `probe` table, and the table's
     /// lock, held until the test ends.
-    async fn probe(url: &str) -> (tokio_postgres::Client, tokio::sync::MutexGuard<'static, ()>) {
+    pub(super) async fn probe(
+        url: &str,
+    ) -> (tokio_postgres::Client, tokio::sync::MutexGuard<'static, ()>) {
         let turn = PROBE.lock().await;
         let mut config: tokio_postgres::Config = url.parse().unwrap();
         config.ssl_mode(tokio_postgres::config::SslMode::Disable);
@@ -555,7 +571,7 @@ mod tests {
     }
 
     /// The rows in the `probe` table.
-    async fn probe_rows(admin: &tokio_postgres::Client) -> i64 {
+    pub(super) async fn probe_rows(admin: &tokio_postgres::Client) -> i64 {
         admin
             .query_one("SELECT count(*) FROM probe", &[])
             .await
@@ -738,7 +754,15 @@ mod tests {
         let stop = StopFlag::new();
         stop.stop();
         let mut outcome = ScriptOutcome::default();
-        let tx = statements(&client, &["SELECT 1".to_owned()], 10, &stop, &mut outcome).await;
+        let tx = statements(
+            &client,
+            &["SELECT 1".to_owned()],
+            10,
+            ScriptMode::ReadOnly,
+            &stop,
+            &mut outcome,
+        )
+        .await;
         assert_eq!(tx, Ok(Tx::Open));
         assert_eq!(outcome.results.len(), 1);
         assert_eq!(outcome.results[0].outcome, StatementOutcome::Cancelled);
```

At the end of `write.rs`:

```rust
#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::super::tests::{probe, probe_rows};
    use super::*;
    use crate::ConnectSpec;
    use crate::pg::Conn;
    use crate::pg::tests::test_url;

    /// A fresh writable session, as the app opens it.
    async fn writable(url: &str) -> Conn {
        let (mut spec, secrets) = ConnectSpec::from_url(url).unwrap();
        spec.tls = crate::TlsMode::Disable;
        Conn::connect(&spec, &secrets, None, Access::Writable)
            .await
            .unwrap()
    }

    /// Runs statements that write straight through the driver, as if the
    /// refusal had missed them.
    async fn past_the_refusal(conn: &Conn, script: &[&str]) -> Result<ScriptOutcome> {
        let texts: Vec<String> = script.iter().map(|&text| text.to_owned()).collect();
        tokio::time::timeout(
            Duration::from_secs(10),
            conn.run_script(&texts, 10, ScriptMode::Write, &StopFlag::new()),
        )
        .await
        .expect("the run hung")
    }

    /// Statements the refusal stops long before they get here. Run past
    /// it, they end the transaction the run means to commit. The statement
    /// after them must not run, because it would run outside any
    /// transaction, and the run must fail even when nothing follows them,
    /// so that the session is closed.
    #[tokio::test]
    async fn a_script_that_ended_its_transaction_runs_nothing_more_and_fails() {
        let Some(url) = test_url() else {
            return;
        };
        let (admin, _turn) = probe(&url).await;
        for (script, written) in [
            // The script's own commit stands. Nothing ran after it.
            (
                vec![
                    "INSERT INTO probe VALUES (1)",
                    "COMMIT",
                    "INSERT INTO probe VALUES (2)",
                ],
                1,
            ),
            (vec!["INSERT INTO probe VALUES (1)", "COMMIT"], 1),
            (
                vec![
                    "INSERT INTO probe VALUES (1)",
                    "ROLLBACK",
                    "INSERT INTO probe VALUES (2)",
                ],
                0,
            ),
            // A new transaction, which is not the run's.
            (
                vec![
                    "INSERT INTO probe VALUES (1)",
                    "COMMIT AND CHAIN",
                    "INSERT INTO probe VALUES (2)",
                ],
                1,
            ),
        ] {
            admin.batch_execute("TRUNCATE probe").await.unwrap();
            let conn = writable(&url).await;
            let ran = past_the_refusal(&conn, &script).await;
            assert_eq!(ran, Err(Error::LeftTransaction), "{script:?}");
            assert_eq!(probe_rows(&admin).await, written, "{script:?}");
        }
    }

    /// A transaction that failed with every statement done cannot be
    /// committed: PostgreSQL would answer the COMMIT with a rollback and no
    /// error. The check before the commit sees it.
    #[tokio::test]
    async fn a_transaction_that_failed_behind_the_run_is_not_called_committed() {
        let Some(url) = test_url() else {
            return;
        };
        let (admin, _turn) = probe(&url).await;
        let conn = writable(&url).await;
        let client = conn.client.lock().await;
        assert_eq!(open(&client).await, Ok(Tx::Open));
        client
            .batch_execute("INSERT INTO probe VALUES (1)")
            .await
            .unwrap();
        // Fails the block the way no statement of a run can without the
        // run seeing it.
        assert!(client.batch_execute("SELECT 1 / 0").await.is_err());
        let outcome = close(
            &client,
            Tx::Open,
            &StopFlag::new(),
            ScriptOutcome::default(),
        )
        .await
        .unwrap();
        assert!(
            matches!(outcome.end, ScriptEnd::CommitFailed { committed: 0, .. }),
            "{:?}",
            outcome.end
        );
        assert_eq!(probe_rows(&admin).await, 0);
    }

    /// A stop that comes after the last statement still wins over the
    /// commit.
    #[tokio::test]
    async fn a_stop_before_the_commit_rolls_back() {
        let Some(url) = test_url() else {
            return;
        };
        let (admin, _turn) = probe(&url).await;
        let conn = writable(&url).await;
        let client = conn.client.lock().await;
        assert_eq!(open(&client).await, Ok(Tx::Open));
        client
            .batch_execute("INSERT INTO probe VALUES (1)")
            .await
            .unwrap();
        let stop = StopFlag::new();
        stop.stop();
        let outcome = close(&client, Tx::Open, &stop, ScriptOutcome::default())
            .await
            .unwrap();
        assert!(outcome.stopped);
        assert_eq!(outcome.end, ScriptEnd::RolledBack);
        assert!(stop.is_finishing());
        assert_eq!(probe_rows(&admin).await, 0);
    }
}
```

- [ ] **Step 5: Run them to see them pass**

Run: `~/.cargo/bin/cargo test --locked -p tabletist-db`
Expected, with the server: PASS. In your report say whether the PostgreSQL tests ran or were only compiled.
Run the four checks. Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add crates/tabletist-db
git commit -m "Run a script that writes on PostgreSQL

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 6: The run that writes, on MySQL

`mysql/script/write.rs` holds the run; it reuses `run_statement` and `reset` from `mysql/script.rs` and `execute`, `status` and `prepare_session` from `mysql.rs`.

MySQL is the driver where a transaction does not hold a whole run: the server commits by itself before DDL (`CREATE`, `ALTER`, `DROP`, `RENAME`, `TRUNCATE` and more), also when the statement then fails. The run cannot stop that; it has to see it and say it.

- **`inside`** asks whether the session is in a transaction with a query of its own (`DO 0`) and reads the status in the answer. The status the driver holds will not do after a statement that failed: an error packet carries none, and mysql_async empties what it held (`Conn::handle_err`).
- **Before every statement** (`ready`): outside a transaction means the statement before it made the server commit. `Run::committed` becomes the count of statements so far, and a new transaction begins. No statement ever runs outside one.
- **After a statement that failed, and after a stop between statements** (`settle`): the same question. Outside can mean two things here. After a statement that can make the server commit, it committed what came before. After one that cannot (`can_commit`: its first word is in `NEVER_COMMITS`, the statements that read or change rows and those that set, show or explain), the server rolled the transaction back itself, as it does for a deadlock, and nothing since the last commit is written. The statement decides, not the error's code: a DDL statement that loses a deadlock has still committed what came before it.
- **The end.** `COMMIT` when the run succeeded. A `COMMIT` answered with a cancel is followed by `inside`: still in the transaction means nothing was committed; outside means the server does not say which way it went, and the run is an `Err`. Otherwise `ROLLBACK`, and the end is `Partly { committed }` when something was noted, else `RolledBack`. A `ROLLBACK` that raises a warning (1196, a table without transactions) has its text read with `SHOW WARNINGS` at once, into `rollback_warning`. It is asked for once: after a `SHOW WARNINGS` that failed, a second would show that failure.
- **No `sql_select_limit`.** A `SELECT` that calls a function that writes must run for every row. Rows past `limit + 1` are read and dropped, which `run_statement` already does.
- **The reset** is the read-only run's: `COM_RESET_CONNECTION`, then the connect-time statements, ending read-write only after a run that ended cleanly.

`run_statement` now reports a statement's warning count in both modes. Existing tests pin `warnings: 0` on statements that should raise none: `SETS` in `mysql/script.rs`'s tests (which includes `SET sql_mode = 'ANSI,NO_BACKSLASH_ESCAPES'`) and two `Done` values in `tests/mysql.rs` (`statements_without_rows_and_explain`). If one of them warns on your server, the count is the truth: change the pinned number for that statement and say so in your report.

**Facts this task rests on, not run where the plan was written.** Check each when a test disagrees:
- After `START TRANSACTION` the status has `SERVER_STATUS_IN_TRANS`; after the implicit commit of a DDL statement it does not.
- A `CREATE TABLE` that fails (the table exists) has committed what came before it.
- A deadlock (error 1213, SQLSTATE 40001) rolls the whole transaction back and leaves the session outside one. If the session is found inside one instead, the run's `ROLLBACK` undoes it and the end is `RolledBack` all the same.
- InnoDB rolls back the lighter of two deadlocked transactions: the test makes the other side change twenty rows first so the script loses. If the other side loses, its `query_drop(..).unwrap()` panics and says so.
- `information_schema.processlist.info` shows a statement that waits for a lock (`runs_on_the_server` waits on it).
- `ROLLBACK` after a change to a MyISAM table answers with one warning whose text holds "non-transactional".
- `INSERT IGNORE` of a value too long for its column raises one warning and stores the row.
- `DO 0` is accepted by every supported MySQL and MariaDB.
- None of the statements in `NEVER_COMMITS` causes an implicit commit, as the refusal list leaves them (`SET autocommit` is refused).
- Not tested, because it cannot be staged: a `COMMIT` answered with 1317 (query interrupted). The code does not rest on what it means; it asks.

**Files:**
- Create: `crates/tabletist-db/src/mysql/script/write.rs`
- Modify: `crates/tabletist-db/src/mysql/script.rs`
- Test: `crates/tabletist-db/tests/mysql.rs`, and `write.rs`'s own `mod tests`

- [ ] **Step 1: Write the failing tests**

At the end of `tests/mysql.rs`, and `ScriptEnd` in its imports:

```diff
diff --git a/crates/tabletist-db/tests/mysql.rs b/crates/tabletist-db/tests/mysql.rs
index b55e20d..3266428 100644
--- a/crates/tabletist-db/tests/mysql.rs
+++ b/crates/tabletist-db/tests/mysql.rs
@@ -854,7 +854,7 @@ async fn a_cancelled_query_leaves_no_transaction_holding_locks() {
     );
 }
 
-use tabletist_db::{Dialect, ScriptMode, ScriptOutcome, StatementOutcome, StopFlag};
+use tabletist_db::{Dialect, ScriptEnd, ScriptMode, ScriptOutcome, StatementOutcome, StopFlag};
 
 fn script(text: &str) -> Vec<tabletist_db::sql::Statement> {
     tabletist_db::sql::statements(Dialect::MySql, text)
@@ -1784,3 +1784,586 @@ async fn the_words_format_uppercases_cannot_be_table_aliases() {
     assert_eq!(value, Some(1));
     conn.disconnect().await.unwrap();
 }
+
+/// Runs `text` as a script that writes, with a fresh stop flag.
+async fn write(
+    connection: &Connection,
+    text: &str,
+    limit: u32,
+) -> tabletist_db::Result<ScriptOutcome> {
+    within(connection.run_script(&script(text), limit, ScriptMode::Write, &StopFlag::new())).await
+}
+
+/// An empty table of this name with a key `id` and a column `n`, made
+/// from outside the session under test. Each test has a table of its own:
+/// they run at once.
+async fn scratch(admin: &mut mysql_async::Conn, table: &str) {
+    admin
+        .query_drop(format!("DROP TABLE IF EXISTS {table}"))
+        .await
+        .unwrap();
+    admin
+        .query_drop(format!(
+            "CREATE TABLE {table} (id int PRIMARY KEY, n int NOT NULL DEFAULT 0)"
+        ))
+        .await
+        .unwrap();
+}
+
+/// A count, asked from outside the session under test.
+async fn counted(admin: &mut mysql_async::Conn, sql: &str) -> i64 {
+    admin.query_first(sql).await.unwrap().unwrap()
+}
+
+/// What each statement of a run did, as its row count, or `None` for one
+/// that has none or did not end well.
+fn affected(outcome: &ScriptOutcome) -> Vec<Option<u64>> {
+    outcome
+        .results
+        .iter()
+        .map(|result| match result.outcome {
+            StatementOutcome::Done { affected, .. } => affected,
+            _ => None,
+        })
+        .collect()
+}
+
+/// Whether the session refuses a write in a read-only run, as it must
+/// after every run that writes.
+async fn is_fenced(connection: &Connection, table: &str) -> bool {
+    let outcome = run(connection, &format!("DELETE FROM {table}"), 1)
+        .await
+        .unwrap();
+    matches!(
+        &outcome.results[0].outcome,
+        StatementOutcome::Error {
+            error: Error::Query { code: Some(code), .. },
+            ..
+        } if code == "25006"
+    )
+}
+
+#[tokio::test]
+async fn a_run_that_writes_is_refused_on_a_read_only_connection() {
+    let Some(connection) = connect_as(Access::ReadOnly).await else {
+        return;
+    };
+    for text in ["DELETE FROM users", "SELECT 1", "COMMIT"] {
+        assert_eq!(
+            write(&connection, text, 10).await,
+            Err(Error::ReadOnly),
+            "{text}"
+        );
+    }
+}
+
+/// A procedure can commit the run's transaction and open one of its own,
+/// which the run could not tell from its own.
+#[tokio::test]
+async fn a_procedure_is_refused_in_a_run_that_writes_too() {
+    let Some(connection) = connect_as(Access::Writable).await else {
+        return;
+    };
+    assert_eq!(
+        write(&connection, "SELECT 1;\nCALL refill()", 10).await,
+        Err(Error::Refused {
+            line: 2,
+            what: "CALL".into(),
+            mode: ScriptMode::Write,
+        })
+    );
+}
+
+#[tokio::test]
+async fn a_run_that_writes_is_committed_and_counts_its_rows() {
+    let Some(connection) = connect_as(Access::Writable).await else {
+        return;
+    };
+    let mut admin = admin().await;
+    scratch(&mut admin, "write_counts").await;
+    let outcome = write(
+        &connection,
+        "INSERT INTO write_counts (id) VALUES (1), (2), (3), (4), (5);
+         UPDATE write_counts SET n = 10 WHERE id > 2;
+         DELETE FROM write_counts WHERE id = 1",
+        10,
+    )
+    .await
+    .unwrap();
+    assert_eq!(outcome.end, ScriptEnd::Committed);
+    assert!(!outcome.was_cancelled());
+    assert_eq!(outcome.broken, None);
+    assert_eq!(outcome.rollback_warning, None);
+    assert_eq!(affected(&outcome), [Some(5), Some(3), Some(1)]);
+    assert_eq!(
+        counted(&mut admin, "SELECT count(*) FROM write_counts").await,
+        4
+    );
+    // Read-write between scripts, as it connected, and fenced in a run
+    // that only reads.
+    assert!(writes_between_scripts(&connection).await);
+    assert!(is_fenced(&connection, "write_counts").await);
+    assert!(writes_between_scripts(&connection).await);
+    admin.query_drop("DROP TABLE write_counts").await.unwrap();
+}
+
+#[tokio::test]
+async fn a_statement_that_fails_rolls_the_run_back() {
+    let Some(connection) = connect_as(Access::Writable).await else {
+        return;
+    };
+    let mut admin = admin().await;
+    scratch(&mut admin, "write_fails").await;
+    let outcome = write(
+        &connection,
+        "INSERT INTO write_fails (id) VALUES (1);
+         INSERT INTO write_fails_missing VALUES (1);
+         INSERT INTO write_fails (id) VALUES (2)",
+        10,
+    )
+    .await
+    .unwrap();
+    // Nothing made the server commit, so nothing is written: not `Partly`.
+    assert_eq!(outcome.end, ScriptEnd::RolledBack);
+    assert_eq!(outcome.rollback_warning, None);
+    assert_eq!(outcome.results.len(), 2);
+    assert_eq!(
+        counted(&mut admin, "SELECT count(*) FROM write_fails").await,
+        0
+    );
+    assert!(writes_between_scripts(&connection).await);
+    admin.query_drop("DROP TABLE write_fails").await.unwrap();
+}
+
+#[tokio::test]
+async fn what_the_server_committed_by_itself_is_said_to_be_written() {
+    let Some(connection) = connect_as(Access::Writable).await else {
+        return;
+    };
+    let mut admin = admin().await;
+    scratch(&mut admin, "write_partly").await;
+    admin
+        .query_drop("DROP TABLE IF EXISTS write_partly_made")
+        .await
+        .unwrap();
+    // CREATE TABLE commits the first insert. The second is rolled back
+    // with the statement that failed.
+    let outcome = write(
+        &connection,
+        "INSERT INTO write_partly (id) VALUES (1);
+         CREATE TABLE write_partly_made (id int);
+         INSERT INTO write_partly (id) VALUES (2);
+         INSERT INTO write_partly_missing VALUES (1)",
+        10,
+    )
+    .await
+    .unwrap();
+    assert_eq!(outcome.end, ScriptEnd::Partly { committed: 2 });
+    assert_eq!(
+        counted(&mut admin, "SELECT count(*) FROM write_partly").await,
+        1
+    );
+    assert_eq!(
+        counted(&mut admin, "SELECT count(*) FROM write_partly_made").await,
+        0
+    );
+    // A CREATE TABLE that fails has still committed what came before it.
+    let outcome = write(
+        &connection,
+        "INSERT INTO write_partly (id) VALUES (3);
+         CREATE TABLE write_partly_made (id int)",
+        10,
+    )
+    .await
+    .unwrap();
+    assert_eq!(outcome.end, ScriptEnd::Partly { committed: 1 });
+    assert_eq!(
+        counted(&mut admin, "SELECT count(*) FROM write_partly").await,
+        2
+    );
+    // With every statement done, all of it is committed, whoever did it.
+    let outcome = write(
+        &connection,
+        "INSERT INTO write_partly (id) VALUES (4);
+         DROP TABLE write_partly_made;
+         INSERT INTO write_partly (id) VALUES (5)",
+        10,
+    )
+    .await
+    .unwrap();
+    assert_eq!(outcome.end, ScriptEnd::Committed);
+    assert_eq!(
+        counted(&mut admin, "SELECT count(*) FROM write_partly").await,
+        4
+    );
+    assert!(writes_between_scripts(&connection).await);
+    admin.query_drop("DROP TABLE write_partly").await.unwrap();
+}
+
+#[tokio::test]
+async fn the_limit_cuts_what_is_shown_and_nothing_that_is_written() {
+    let Some(connection) = connect_as(Access::Writable).await else {
+        return;
+    };
+    let mut admin = admin().await;
+    scratch(&mut admin, "write_limit").await;
+    admin
+        .query_drop("DROP TABLE IF EXISTS write_limit_copy")
+        .await
+        .unwrap();
+    let outcome = write(
+        &connection,
+        "INSERT INTO write_limit (id) VALUES (1), (2), (3), (4), (5), (6), (7), (8);
+         INSERT INTO write_limit (id) SELECT id + 100 FROM write_limit;
+         CREATE TABLE write_limit_copy AS SELECT * FROM write_limit;
+         SELECT id FROM write_limit",
+        3,
+    )
+    .await
+    .unwrap();
+    assert_eq!(outcome.end, ScriptEnd::Committed);
+    assert_eq!(affected(&outcome)[..2], [Some(8), Some(8)]);
+    assert!(matches!(
+        &outcome.results[3].outcome,
+        StatementOutcome::Rows { rows, truncated: true, .. } if rows.len() == 3
+    ));
+    assert_eq!(
+        counted(&mut admin, "SELECT count(*) FROM write_limit").await,
+        16
+    );
+    assert_eq!(
+        counted(&mut admin, "SELECT count(*) FROM write_limit_copy").await,
+        16
+    );
+    admin
+        .query_drop("DROP TABLE write_limit, write_limit_copy")
+        .await
+        .unwrap();
+}
+
+#[tokio::test]
+async fn a_change_the_server_cannot_roll_back_is_said() {
+    let Some(connection) = connect_as(Access::Writable).await else {
+        return;
+    };
+    let mut admin = admin().await;
+    admin
+        .query_drop("DROP TABLE IF EXISTS write_myisam")
+        .await
+        .unwrap();
+    admin
+        .query_drop("CREATE TABLE write_myisam (id int PRIMARY KEY) ENGINE = MyISAM")
+        .await
+        .unwrap();
+    let outcome = write(
+        &connection,
+        "INSERT INTO write_myisam VALUES (1); INSERT INTO write_myisam_missing VALUES (1)",
+        10,
+    )
+    .await
+    .unwrap();
+    assert_eq!(outcome.end, ScriptEnd::RolledBack);
+    let warning = outcome.rollback_warning.expect("the server warns");
+    assert!(warning.contains("non-transactional"), "{warning}");
+    // And it is true: the row stayed.
+    assert_eq!(
+        counted(&mut admin, "SELECT count(*) FROM write_myisam").await,
+        1
+    );
+    assert!(writes_between_scripts(&connection).await);
+    admin.query_drop("DROP TABLE write_myisam").await.unwrap();
+}
+
+#[tokio::test]
+async fn a_statement_says_how_many_warnings_it_raised() {
+    let Some(connection) = connect_as(Access::Writable).await else {
+        return;
+    };
+    let mut admin = admin().await;
+    admin
+        .query_drop("DROP TABLE IF EXISTS write_warns")
+        .await
+        .unwrap();
+    admin
+        .query_drop("CREATE TABLE write_warns (s varchar(3))")
+        .await
+        .unwrap();
+    // IGNORE makes a warning of what strict mode would refuse.
+    let outcome = write(
+        &connection,
+        "INSERT INTO write_warns VALUES ('abc'); INSERT IGNORE INTO write_warns VALUES ('abcdef')",
+        10,
+    )
+    .await
+    .unwrap();
+    assert_eq!(outcome.end, ScriptEnd::Committed);
+    assert_eq!(
+        outcome.results[0].outcome,
+        StatementOutcome::Done {
+            affected: Some(1),
+            warnings: 0,
+        }
+    );
+    assert_eq!(
+        outcome.results[1].outcome,
+        StatementOutcome::Done {
+            affected: Some(1),
+            warnings: 1,
+        }
+    );
+    admin.query_drop("DROP TABLE write_warns").await.unwrap();
+}
+
+/// Runs `text` as a script that writes and makes it lose a deadlock on
+/// the statement holding `marker`: that statement must wait for the row
+/// with id 1 of `table`, after the script has changed the row with id 2.
+/// The other side has changed far more rows, so the server picks the
+/// script as the one to roll back.
+async fn deadlocked(table: &str, text: String, marker: &str) -> Option<ScriptOutcome> {
+    let connection = std::sync::Arc::new(connect_as(Access::Writable).await?);
+    let mut other = admin().await;
+    other.query_drop("START TRANSACTION").await.unwrap();
+    other
+        .query_drop(format!(
+            "INSERT INTO {table} (id) VALUES (10), (11), (12), (13), (14), (15), (16), (17), \
+             (18), (19), (20), (21), (22), (23), (24), (25), (26), (27), (28), (29)"
+        ))
+        .await
+        .unwrap();
+    other
+        .query_drop(format!("UPDATE {table} SET n = 1 WHERE id = 1"))
+        .await
+        .unwrap();
+    let running = {
+        let connection = std::sync::Arc::clone(&connection);
+        tokio::spawn(async move {
+            connection
+                .run_script(&script(&text), 10, ScriptMode::Write, &StopFlag::new())
+                .await
+        })
+    };
+    let mut watcher = admin().await;
+    runs_on_the_server(&mut watcher, marker).await;
+    // Closes the circle: this waits for the script's row, the script waits
+    // for this one's. The script loses, and this goes through.
+    other
+        .query_drop(format!("UPDATE {table} SET n = 1 WHERE id = 2"))
+        .await
+        .unwrap();
+    let outcome = within(running).await.unwrap().unwrap();
+    other.query_drop("ROLLBACK").await.unwrap();
+    assert!(
+        matches!(
+            &outcome.results.last().unwrap().outcome,
+            StatementOutcome::Error {
+                error: Error::Query { code: Some(code), .. },
+                ..
+            } if code == "40001"
+        ),
+        "{:?}",
+        outcome.results.last()
+    );
+    // The session survives, read-write between scripts.
+    assert!(writes_between_scripts(&connection).await);
+    Some(outcome)
+}
+
+#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
+async fn a_deadlock_rolls_the_run_back_and_is_not_taken_for_a_commit() {
+    if url().is_none() {
+        eprintln!("skipped: TABLETIST_TEST_MYSQL_URL is not set");
+        return;
+    }
+    let mut admin = admin().await;
+    scratch(&mut admin, "write_deadlock").await;
+    admin
+        .query_drop("INSERT INTO write_deadlock (id) VALUES (1), (2), (3)")
+        .await
+        .unwrap();
+    // The server rolls the whole transaction back and leaves the session
+    // outside one, as a commit would. Nothing of the run is written.
+    let text = "UPDATE write_deadlock SET n = 7 WHERE id = 3; \
+                UPDATE write_deadlock SET n = 7 WHERE id = 2; \
+                UPDATE write_deadlock SET n = 7 /* tabletist deadlock one */ WHERE id = 1";
+    let Some(outcome) =
+        deadlocked("write_deadlock", text.to_owned(), "tabletist deadlock one").await
+    else {
+        return;
+    };
+    assert_eq!(outcome.end, ScriptEnd::RolledBack);
+    assert_eq!(
+        counted(
+            &mut admin,
+            "SELECT count(*) FROM write_deadlock WHERE n = 7"
+        )
+        .await,
+        0
+    );
+    // After a commit the server made earlier in the run, only what came
+    // before that commit is written.
+    admin
+        .query_drop("DROP TABLE IF EXISTS write_deadlock_made")
+        .await
+        .unwrap();
+    let text = "UPDATE write_deadlock SET n = 7 WHERE id = 3; \
+                CREATE TABLE write_deadlock_made (id int); \
+                UPDATE write_deadlock SET n = 8 WHERE id = 2; \
+                UPDATE write_deadlock SET n = 8 /* tabletist deadlock two */ WHERE id = 1";
+    let Some(outcome) =
+        deadlocked("write_deadlock", text.to_owned(), "tabletist deadlock two").await
+    else {
+        return;
+    };
+    assert_eq!(outcome.end, ScriptEnd::Partly { committed: 2 });
+    assert_eq!(
+        counted(
+            &mut admin,
+            "SELECT count(*) FROM write_deadlock WHERE n = 7"
+        )
+        .await,
+        1
+    );
+    assert_eq!(
+        counted(
+            &mut admin,
+            "SELECT count(*) FROM write_deadlock WHERE n = 8"
+        )
+        .await,
+        0
+    );
+    admin
+        .query_drop("DROP TABLE write_deadlock, write_deadlock_made")
+        .await
+        .unwrap();
+}
+
+#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
+async fn a_run_that_writes_and_is_cancelled_is_rolled_back() {
+    let Some(connection) = connect_as(Access::Writable).await else {
+        return;
+    };
+    let connection = std::sync::Arc::new(connection);
+    let mut admin = admin().await;
+    scratch(&mut admin, "write_cancelled").await;
+    let cancel = connection.cancel_handle();
+    let stop = StopFlag::new();
+    let running = {
+        let connection = std::sync::Arc::clone(&connection);
+        let stop = stop.clone();
+        tokio::spawn(async move {
+            let text = "INSERT INTO write_cancelled (id) VALUES (1); \
+                        SELECT count(*) FROM users WHERE SLEEP(35) = 0";
+            connection
+                .run_script(&script(text), 10, ScriptMode::Write, &stop)
+                .await
+        })
+    };
+    // A sleep of its own length: `runs_on_the_server` sees every session.
+    runs_on_the_server(&mut admin, "SLEEP(35)").await;
+    stop.stop();
+    let deadline = std::time::Instant::now() + Duration::from_secs(15);
+    // As the backend does: repeat the cancel until the cleanup begins.
+    while !stop.is_finishing() && !running.is_finished() {
+        assert!(
+            std::time::Instant::now() < deadline,
+            "cancel must stop the script"
+        );
+        cancel.cancel().await.unwrap();
+        tokio::time::sleep(Duration::from_millis(100)).await;
+    }
+    let outcome = within(running).await.unwrap().unwrap();
+    assert!(outcome.was_cancelled());
+    assert_eq!(outcome.end, ScriptEnd::RolledBack);
+    // The insert ran, and is undone.
+    assert_eq!(affected(&outcome), [Some(1), None]);
+    assert_eq!(
+        counted(&mut admin, "SELECT count(*) FROM write_cancelled").await,
+        0
+    );
+    // The session survives, read-write between scripts as it connected.
+    assert!(writes_between_scripts(&connection).await);
+    assert_connect_time_settings(&connection).await;
+    admin
+        .query_drop("DROP TABLE write_cancelled")
+        .await
+        .unwrap();
+}
+
+#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
+async fn a_stop_after_a_statement_that_committed_still_says_what_is_written() {
+    let Some(connection) = connect_as(Access::Writable).await else {
+        return;
+    };
+    let connection = std::sync::Arc::new(connection);
+    let mut admin = admin().await;
+    scratch(&mut admin, "write_stopped").await;
+    admin
+        .query_drop("DROP TABLE IF EXISTS write_stopped_made")
+        .await
+        .unwrap();
+    let stop = StopFlag::new();
+    let running = {
+        let connection = std::sync::Arc::clone(&connection);
+        let stop = stop.clone();
+        tokio::spawn(async move {
+            let text = "INSERT INTO write_stopped (id) VALUES (1); \
+                        CREATE TABLE write_stopped_made AS SELECT SLEEP(1) AS slept; \
+                        INSERT INTO write_stopped (id) VALUES (2)";
+            connection
+                .run_script(&script(text), 10, ScriptMode::Write, &stop)
+                .await
+        })
+    };
+    // A stop without a cancel: the CREATE TABLE ends by itself, having
+    // committed the insert before it, and the statement after it never
+    // starts. The check that would have seen the commit never ran either.
+    runs_on_the_server(&mut admin, "write_stopped_made").await;
+    stop.stop();
+    let outcome = within(running).await.unwrap().unwrap();
+    assert_eq!(outcome.results.len(), 3);
+    assert_eq!(outcome.results[2].outcome, StatementOutcome::Cancelled);
+    assert_eq!(outcome.end, ScriptEnd::Partly { committed: 2 });
+    assert_eq!(
+        counted(&mut admin, "SELECT count(*) FROM write_stopped").await,
+        1
+    );
+    assert!(writes_between_scripts(&connection).await);
+    admin
+        .query_drop("DROP TABLE write_stopped, write_stopped_made")
+        .await
+        .unwrap();
+}
+
+#[tokio::test]
+async fn a_run_that_writes_leaves_the_session_as_it_connected() {
+    let Some(connection) = connect_as(Access::Writable).await else {
+        return;
+    };
+    let mut admin = admin().await;
+    scratch(&mut admin, "write_session").await;
+    let leaves = |id: u32| {
+        format!(
+            "SET time_zone = '+05:00'; SET @tabletist_left_over = 1; \
+             SET sql_select_limit = 1; INSERT INTO write_session (id) VALUES ({id})"
+        )
+    };
+    let outcome = write(&connection, &leaves(1), 10).await.unwrap();
+    assert_eq!(outcome.end, ScriptEnd::Committed);
+    assert_eq!(outcome.broken, None);
+    assert_connect_time_settings(&connection).await;
+    assert!(writes_between_scripts(&connection).await);
+    // After a run that failed, and so was rolled back.
+    let failing = format!(
+        "{}; INSERT INTO write_session_missing VALUES (1)",
+        leaves(2)
+    );
+    let outcome = write(&connection, &failing, 10).await.unwrap();
+    assert!(matches!(
+        outcome.results.last().unwrap().outcome,
+        StatementOutcome::Error { .. }
+    ));
+    assert_connect_time_settings(&connection).await;
+    assert!(writes_between_scripts(&connection).await);
+    assert!(is_fenced(&connection, "write_session").await);
+    admin.query_drop("DROP TABLE write_session").await.unwrap();
+}
```

- [ ] **Step 2: Run them to see them fail**

Run: `~/.cargo/bin/cargo test --locked -p tabletist-db --test mysql`
Expected, with `TABLETIST_TEST_MYSQL_URL` set: FAIL, the new tests panicking with ``Unsupported("read-write runs on MySQL")``. Without the variable they print "skipped" and pass.

- [ ] **Step 3: Write the run**

`mysql/script.rs`: the module, the branch after the check that the server can reset its session, and the warning count.

```diff
diff --git a/crates/tabletist-db/src/mysql/script.rs b/crates/tabletist-db/src/mysql/script.rs
index cfdf99f..513f6cf 100644
--- a/crates/tabletist-db/src/mysql/script.rs
+++ b/crates/tabletist-db/src/mysql/script.rs
@@ -1,5 +1,7 @@
-//! Running a SQL editor script on MySQL: one read-only transaction, the
-//! checks around every statement, and the session reset that ends it.
+//! Running a SQL editor script on MySQL: one transaction, the checks
+//! around every statement, and the session reset that ends it. This file
+//! is the run that only reads, in a read-only transaction that is rolled
+//! back; `write` is the run that commits.
 
 use std::time::{Duration, Instant};
 
@@ -18,6 +20,8 @@ use crate::{
     StopFlag,
 };
 
+mod write;
+
 impl Conn {
     /// See [`crate::Connection::run_script`]. Statements run through the
     /// prepared protocol, which cannot hold two, and `sql_select_limit`
@@ -36,9 +40,6 @@ impl Conn {
         mode: ScriptMode,
         stop: &StopFlag,
     ) -> Result<ScriptOutcome> {
-        if mode == ScriptMode::Write {
-            return Err(Error::Unsupported("read-write runs on MySQL"));
-        }
         let mut conn = self.conn.lock().await;
         // Said before anything runs, not found out by the cleanup, which
         // would close the session after every run.
@@ -47,6 +48,9 @@ impl Conn {
                 "the SQL editor needs MySQL 5.7.3 or MariaDB 10.2.4 or later",
             ));
         }
+        if mode == ScriptMode::Write {
+            return write::run(&mut conn, texts, limit as usize, stop).await;
+        }
         let mut outcome = ScriptOutcome::default();
         let ended = match open(&mut conn, limit, self.access).await {
             // A lost session ends the run here: nothing to close.
@@ -452,12 +456,13 @@ async fn run_statement(
     let columns = column_metas(result.columns_ref());
     if columns.is_empty() {
         let affected = result.affected_rows();
+        let warnings = result.warnings();
         if let Err(error) = result.drop_result().await {
             return failed(error);
         }
         return Ok(StatementOutcome::Done {
             affected: counts_rows(text).then_some(affected),
-            warnings: 0,
+            warnings,
         });
     }
     // sql_select_limit does not bound every statement (SHOW, a SELECT with
```

Create `crates/tabletist-db/src/mysql/script/write.rs`:

```rust
//! A SQL editor script that writes, on MySQL: one read-write transaction,
//! a note of every commit the server makes by itself, a commit when every
//! statement succeeded, and the session reset afterwards.

use std::time::{Duration, Instant};

use mysql_async::consts::StatusFlags;
use mysql_async::prelude::Queryable;

use super::super::{execute, prepare_session, query_error, status};
use super::{reset, run_statement};
use crate::script::{cannot_start, cleanup_failed, retry_cancelled};
use crate::{
    Access, Dialect, Error, Result, ScriptEnd, ScriptMode, ScriptOutcome, StatementOutcome,
    StatementResult, StopFlag,
};

/// See [`crate::Connection::run_script`], for [`ScriptMode::Write`]. The
/// caller has made sure the server can reset its session.
///
/// No `sql_select_limit` here: a `SELECT` that calls a function that
/// writes must run for every row, and no server version is trusted to keep
/// that limit off an `INSERT ... SELECT`. Rows past `limit + 1` are read
/// and dropped.
///
/// The future must be awaited to its end, as the read-only run's.
pub(super) async fn run(
    conn: &mut mysql_async::Conn,
    texts: &[String],
    limit: usize,
    stop: &StopFlag,
) -> Result<ScriptOutcome> {
    let mut outcome = ScriptOutcome::default();
    let mut run = Run::default();
    match begin(conn).await {
        // A lost session ends the run here: nothing to close.
        Ok(()) => statements(conn, texts, limit, stop, &mut outcome, &mut run).await?,
        // A cancel landed on the opening query: no results.
        Err(Error::Cancelled) => outcome.stopped = true,
        Err(error) if error.is_connection_lost() => return Err(error),
        // The transaction could not start. The next run would fail the
        // same way, so the session is closed, after an attempt to end what
        // is open.
        Err(error) => run.broken = Some(cannot_start(ScriptMode::Write, &error)),
    }
    close(conn, run, stop, outcome).await
}

/// What a run knows of its session beside its statements' results.
#[derive(Debug, Default)]
struct Run {
    /// How many of the script's first statements the server has committed
    /// by itself: it commits before DDL and some other statements, also
    /// when the statement then fails.
    committed: usize,
    /// A check could not be made, or a transaction could not start: the
    /// session is closed with this error, and the run's end is not told.
    broken: Option<Error>,
}

/// Starts a transaction, and holds the server to saying so: `inside` reads
/// the same mark later.
async fn begin(conn: &mut mysql_async::Conn) -> Result<()> {
    execute(conn, "START TRANSACTION").await?;
    if status(conn).contains(StatusFlags::SERVER_STATUS_IN_TRANS) {
        Ok(())
    } else {
        Err(Error::query("the server did not start a transaction"))
    }
}

/// Whether the session is inside a transaction, asked with a query of its
/// own whose answer carries the server's status. The status the driver
/// holds is no answer after a statement that failed: an error packet
/// carries none, and the driver empties what it held.
///
/// The status tells a transaction from none, not one transaction from
/// another. Without stored procedures no single statement can end the
/// run's transaction and leave another open, which is why `CALL` stays
/// refused.
async fn inside(conn: &mut mysql_async::Conn) -> Result<bool> {
    execute(conn, "DO 0").await?;
    Ok(status(conn).contains(StatusFlags::SERVER_STATUS_IN_TRANS))
}

/// The first words of the statements that never make the server commit:
/// those that read or change rows, and those that set, show or explain.
/// (`SET autocommit` would, and the refusal list keeps it out.)
const NEVER_COMMITS: [&str; 14] = [
    "SELECT", "INSERT", "UPDATE", "DELETE", "REPLACE", "WITH", "VALUES", "TABLE", "SET", "DO",
    "SHOW", "EXPLAIN", "DESCRIBE", "DESC",
];

/// Whether a statement of this text can make the server commit. When the
/// session is outside a transaction after a statement that cannot has
/// failed, the server has rolled the transaction back (a deadlock, a lock
/// wait it answers so): nothing of it is written. After any other
/// statement that failed, the session is outside because the server
/// committed before it ran the statement. It errs toward "can": a commit
/// that is said and did not happen sends the user to look, one that
/// happened and is not said does not.
fn can_commit(text: &str) -> bool {
    !crate::sql::words(Dialect::MySql, text)
        .first()
        .is_some_and(|word| NEVER_COMMITS.contains(&word.as_str()))
}

/// Runs the statements in order, up to the first that fails or is
/// stopped, noting in `run` what the server committed by itself. `Err` is
/// a lost session.
async fn statements(
    conn: &mut mysql_async::Conn,
    texts: &[String],
    limit: usize,
    stop: &StopFlag,
    outcome: &mut ScriptOutcome,
    run: &mut Run,
) -> Result<()> {
    for (index, text) in texts.iter().enumerate() {
        let checked = if stop.is_stopped() {
            Err(Error::Cancelled)
        } else {
            ready(conn, index, run).await
        };
        match checked {
            Ok(()) => {}
            // A stop between statements, or a cancel that landed on the
            // check: this statement is the cancelled one. The statement
            // before it may have made the server commit, which this check
            // did not get to see.
            Err(Error::Cancelled) => {
                outcome.stopped = true;
                outcome.results.push(StatementResult {
                    elapsed: Duration::ZERO,
                    outcome: StatementOutcome::Cancelled,
                });
                return settle(conn, stop, run, Some(index)).await;
            }
            Err(error) if error.is_connection_lost() => return Err(error),
            // Where the session stands is not known: nothing more runs.
            Err(error) => {
                run.broken = Some(Error::ConnectionLost(format!(
                    "could not ask where the session stands: {error}"
                )));
                return Ok(());
            }
        }
        let started = Instant::now();
        let result = run_statement(conn, text, limit).await?;
        outcome.stopped |= matches!(result, StatementOutcome::Cancelled);
        let last = !matches!(
            result,
            StatementOutcome::Rows { .. } | StatementOutcome::Done { .. }
        );
        outcome.results.push(StatementResult {
            elapsed: started.elapsed(),
            outcome: result,
        });
        if last {
            // Outside a transaction after a statement that can make the
            // server commit: it committed what came before. After one
            // that cannot, the server rolled the transaction back, and
            // what it committed earlier stands.
            return settle(conn, stop, run, can_commit(text).then_some(index)).await;
        }
    }
    Ok(())
}

/// Asks where the session stands once no more of the script will run: the
/// rollback that follows undoes only what is still in a transaction.
/// `outside` is how many statements are written when the session is found
/// outside one, or `None` when that means the server rolled back.
///
/// Nothing more of the script runs, so the backend is told to stop
/// repeating its cancel first; one that is on its way gets the question
/// asked once more. `Err` is a lost session.
async fn settle(
    conn: &mut mysql_async::Conn,
    stop: &StopFlag,
    run: &mut Run,
    outside: Option<usize>,
) -> Result<()> {
    stop.finish();
    match retry_cancelled!(inside(conn)) {
        Ok(true) => {}
        Ok(false) => {
            if let Some(committed) = outside {
                run.committed = committed;
            }
        }
        Err(error) if error.is_connection_lost() => return Err(error),
        Err(error) => {
            run.broken = Some(Error::ConnectionLost(format!(
                "could not ask where the session stands: {error}"
            )));
        }
    }
    Ok(())
}

/// The check before the statement at `index`: a session found outside a
/// transaction is there because the statement before this one made the
/// server commit. Everything before `index` is then written, and the next
/// statement gets a new transaction: none ever runs outside one.
///
/// `Err` is a cancel that landed on the check, a lost session, or a check
/// that could not be made.
async fn ready(conn: &mut mysql_async::Conn, index: usize, run: &mut Run) -> Result<()> {
    if !inside(conn).await? {
        run.committed = index;
        begin(conn).await?;
    }
    Ok(())
}

/// Ends the run: commits when every statement succeeded and no stop came,
/// rolls back otherwise, says what of the run is written, and resets the
/// session as the read-only run's close does. A step a cancel interrupted
/// runs once more.
///
/// `Err` is a run whose end is not known. It closes the session (it counts
/// as a lost connection).
async fn close(
    conn: &mut mysql_async::Conn,
    mut run: Run,
    stop: &StopFlag,
    mut outcome: ScriptOutcome,
) -> Result<ScriptOutcome> {
    // The backend is told before the flag is read, so a stop it sets from
    // here on sends no cancel, which would land on the commit or the
    // cleanup. One set before this line still wins over the commit.
    stop.finish();
    let stopped = stop.is_stopped();
    let succeeded = run.broken.is_none() && outcome.succeeded();
    let mut commit_error = None;
    if succeeded && stopped {
        outcome.stopped = true;
    } else if succeeded {
        match execute(conn, "COMMIT").await {
            Ok(()) => outcome.end = ScriptEnd::Committed,
            // A cancel that was on its way landed on the commit. Still
            // inside the transaction, nothing was committed, and the
            // rollback below undoes it. Outside one, the server does not
            // say which way the commit went.
            Err(Error::Cancelled) => match retry_cancelled!(inside(conn)) {
                Ok(true) => outcome.stopped = true,
                Ok(false) => {
                    run.broken = Some(Error::ConnectionLost(
                        "a cancel landed on the commit, and the server does not say whether it \
                         went through"
                            .into(),
                    ));
                }
                Err(error) if error.is_connection_lost() => return Err(error),
                Err(error) => run.broken = Some(cleanup_failed(ScriptMode::Write, &error)),
            },
            // Whether it went through cannot be known.
            Err(error) if error.is_connection_lost() => return Err(error),
            Err(error) => commit_error = Some(error),
        }
    }
    // Whatever is still open is rolled back and the session reset as far
    // as that works, also when the session is closed anyway.
    let rolled_back = match outcome.end {
        ScriptEnd::Committed => Ok(None),
        _ => roll_back(conn).await,
    };
    if let Ok(warning) = &rolled_back {
        outcome.rollback_warning.clone_from(warning);
    }
    if outcome.end != ScriptEnd::Committed {
        outcome.end = match commit_error {
            Some(error) => ScriptEnd::CommitFailed {
                error,
                committed: run.committed,
            },
            None if run.committed > 0 => ScriptEnd::Partly {
                committed: run.committed,
            },
            None => ScriptEnd::RolledBack,
        };
    }
    let reset = retry_cancelled!(reset(conn));
    // A run that did not end cleanly leaves the session read-only: it is
    // about to be closed, and until then nothing may write on it.
    let clean = run.broken.is_none() && rolled_back.is_ok() && reset.is_ok();
    let access = if clean {
        Access::Writable
    } else {
        Access::ReadOnly
    };
    let prepared = prepare_session(conn, access).await;
    match (run.broken, rolled_back) {
        (Some(error), _) => Err(error),
        (None, Err(error)) => Err(cleanup_failed(ScriptMode::Write, &error)),
        (None, Ok(_)) => Ok(outcome.put_back(reset.and(prepared))),
    }
}

/// What stands for a warning whose text could not be read.
const UNREAD_WARNING: &str =
    "the server raised a warning when it rolled back, and its text could not be read";

/// Rolls the run's transaction back, and gives what the server warned of
/// when it could not undo everything: a change to a table without
/// transactions stays (warning 1196). A rollback a cancel interrupted runs
/// once more. The warning is read before anything else is sent, since the
/// next statement replaces it, and it is asked for once: after a `SHOW
/// WARNINGS` that failed, a second would show that failure.
async fn roll_back(conn: &mut mysql_async::Conn) -> Result<Option<String>> {
    retry_cancelled!(execute(conn, "ROLLBACK"))?;
    if conn.get_warnings() == 0 {
        return Ok(None);
    }
    let warnings: mysql_async::Result<Vec<(String, u16, String)>> =
        conn.query("SHOW WARNINGS").await;
    match warnings.map_err(query_error) {
        Ok(warnings) => {
            let text = warnings
                .into_iter()
                .map(|(_, _, message)| message)
                .collect::<Vec<_>>()
                .join(" ");
            Ok(Some(if text.is_empty() {
                UNREAD_WARNING.to_owned()
            } else {
                text
            }))
        }
        Err(error) if error.is_connection_lost() => Err(error),
        Err(_) => Ok(Some(UNREAD_WARNING.to_owned())),
    }
}
```

- [ ] **Step 4: The driver's own tests**

Four helpers of `mysql/script.rs`'s tests become `pub(super)`:

```diff
diff --git a/crates/tabletist-db/src/mysql/script.rs b/crates/tabletist-db/src/mysql/script.rs
index cfdf99f..513f6cf 100644
--- a/crates/tabletist-db/src/mysql/script.rs
+++ b/crates/tabletist-db/src/mysql/script.rs
@@ -664,17 +669,19 @@ mod tests {
 
     /// One test at a time uses the `probe` table: creating it twice at once
     /// can fail, and one test's TRUNCATE would hide another's stray row.
-    static PROBE: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());
+    pub(super) static PROBE: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());
 
     /// A writable connection, outside the adapter.
-    async fn admin(url: &str) -> mysql_async::Conn {
+    pub(super) async fn admin(url: &str) -> mysql_async::Conn {
         let opts = Opts::from_url(&format!("{url}?prefer_socket=false")).unwrap();
         mysql_async::Conn::new(opts).await.unwrap()
     }
 
     /// A writable connection with an empty `probe` table, and the table's
     /// lock, held until the test ends.
-    async fn probe(url: &str) -> (mysql_async::Conn, tokio::sync::MutexGuard<'static, ()>) {
+    pub(super) async fn probe(
+        url: &str,
+    ) -> (mysql_async::Conn, tokio::sync::MutexGuard<'static, ()>) {
         let turn = PROBE.lock().await;
         let mut admin = admin(url).await;
         admin
@@ -686,7 +693,7 @@ mod tests {
     }
 
     /// The rows in the `probe` table.
-    async fn probe_rows(admin: &mut mysql_async::Conn) -> i64 {
+    pub(super) async fn probe_rows(admin: &mut mysql_async::Conn) -> i64 {
         admin
             .query_first("SELECT count(*) FROM probe")
             .await
```

At the end of `write.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::super::tests::{probe, probe_rows};
    use super::*;
    use crate::mysql::Conn;
    use crate::mysql::tests::test_url;
    use crate::{ConnectSpec, TlsMode};

    #[test]
    fn only_a_statement_that_reads_or_changes_rows_cannot_make_the_server_commit() {
        for text in [
            "SELECT 1",
            "insert into t values (1)",
            "UPDATE t SET a = 1",
            "DELETE FROM t",
            "REPLACE INTO t VALUES (1)",
            "WITH x AS (SELECT 1) SELECT * FROM x",
            "(SELECT 1) UNION (SELECT 2)",
            "/* first */ UPDATE t SET a = 1",
            // They can lose a deadlock too, and commit nothing.
            "SET @held = (SELECT a FROM t WHERE a = 1 FOR UPDATE)",
            "DO refill()",
            "SHOW TABLES",
            "EXPLAIN ANALYZE UPDATE t SET a = 1",
            "DESCRIBE t",
        ] {
            assert!(!can_commit(text), "{text}");
        }
        for text in [
            "CREATE TABLE t (a int)",
            "ALTER TABLE t ADD b int",
            "DROP TABLE t",
            "TRUNCATE t",
            "RENAME TABLE t TO u",
            "ANALYZE TABLE t",
            "LOAD DATA INFILE 'x' INTO TABLE t",
            "",
        ] {
            assert!(can_commit(text), "{text}");
        }
    }

    /// A fresh writable session, as the app opens it.
    async fn writable(url: &str) -> Conn {
        let (mut spec, secrets) = ConnectSpec::from_url(url).unwrap();
        spec.tls = TlsMode::Disable;
        Conn::connect(&spec, &secrets, None, Access::Writable)
            .await
            .unwrap()
    }

    /// A statement the refusal stops long before it gets here. Run past
    /// it, it ends the run's transaction, which the next check takes for a
    /// commit the server made: what came before is said to be written, and
    /// the statement after it runs in a transaction of its own, which the
    /// failure at the end rolls back.
    #[tokio::test]
    async fn a_script_that_ended_its_transaction_never_runs_outside_one() {
        let Some(url) = test_url() else {
            return;
        };
        let (mut admin, _turn) = probe(&url).await;
        let conn = writable(&url).await;
        let texts: Vec<String> = [
            "INSERT INTO probe VALUES (1)",
            "COMMIT",
            "INSERT INTO probe VALUES (2)",
            "INSERT INTO probe_missing VALUES (3)",
        ]
        .iter()
        .map(|&text| text.to_owned())
        .collect();
        let outcome = tokio::time::timeout(
            Duration::from_secs(10),
            conn.run_script(&texts, 10, ScriptMode::Write, &StopFlag::new()),
        )
        .await
        .expect("the run hung")
        .unwrap();
        assert_eq!(outcome.end, ScriptEnd::Partly { committed: 2 });
        assert_eq!(probe_rows(&mut admin).await, 1);
    }

    /// A stop that comes after the last statement still wins over the
    /// commit.
    #[tokio::test]
    async fn a_stop_before_the_commit_rolls_back() {
        let Some(url) = test_url() else {
            return;
        };
        let (mut admin, _turn) = probe(&url).await;
        let conn = writable(&url).await;
        let mut conn = conn.conn.lock().await;
        begin(&mut conn).await.unwrap();
        execute(&mut conn, "INSERT INTO probe VALUES (1)")
            .await
            .unwrap();
        let stop = StopFlag::new();
        stop.stop();
        let outcome = close(&mut conn, Run::default(), &stop, ScriptOutcome::default())
            .await
            .unwrap();
        assert!(outcome.stopped);
        assert_eq!(outcome.end, ScriptEnd::RolledBack);
        assert!(stop.is_finishing());
        assert_eq!(probe_rows(&mut admin).await, 0);
    }
}
```

- [ ] **Step 5: Run them to see them pass**

Run: `~/.cargo/bin/cargo test --locked -p tabletist-db`
Expected, with the server: PASS. `only_a_statement_that_reads_or_changes_rows_cannot_make_the_server_commit` runs without one. In your report say whether the MySQL tests ran or were only compiled, and against MySQL, MariaDB or both.
Run the four checks. Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add crates/tabletist-db
git commit -m "Run a script that writes on MySQL

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 7: Say what the crate can do now

**Files:**
- Modify: `crates/tabletist-db/src/lib.rs`, `crates/tabletist-db/src/sql.rs`
- Modify: `docs/superpowers/specs/2026-10-05-sql-editor-writes-design.md`

- [ ] **Step 1: The crate's documentation**

"Nothing in this crate writes" is no longer true of the crate, though it is still true of the app, which passes `ScriptMode::ReadOnly`:

```diff
diff --git a/crates/tabletist-db/src/lib.rs b/crates/tabletist-db/src/lib.rs
index 37b7e1b..7976626 100644
--- a/crates/tabletist-db/src/lib.rs
+++ b/crates/tabletist-db/src/lib.rs
@@ -1,9 +1,11 @@
 //! Access to PostgreSQL, MySQL and SQLite for Tabletist.
 //!
-//! Nothing in this crate writes to a connected database yet. A session is
-//! read-only unless it is opened [`Access::Writable`], and then it is
-//! fenced: row fetches and counts run in read-only transactions (on SQLite
-//! under `query_only`), and a SQL editor script still cannot write.
+//! One call writes to a connected database: [`Connection::run_script`] in
+//! [`ScriptMode::Write`], and only on a session opened
+//! [`Access::Writable`]. A session is read-only unless it is opened so, and
+//! then it is fenced: row fetches and counts run in read-only transactions
+//! (on SQLite under `query_only`), and a script run in
+//! [`ScriptMode::ReadOnly`] cannot write.
 
 mod catalog;
 mod check;
@@ -40,15 +42,15 @@ pub use spec::{ConnectSpec, Driver, ParsedUrl, Secrets, SshAuth, SshSpec, TlsMod
 pub use ssh::HostKeys;
 pub use value::{ColumnMeta, Value, ValueKind, value_from_pg_text};
 
-/// Whether a session may write. The app has no writing call yet; a
-/// writable session is the one a later `write` will be allowed on.
+/// Whether a session may write. A script run in [`ScriptMode::Write`] is
+/// what writes, and it is refused on a session that is not writable.
 #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
 pub enum Access {
     /// Nothing can write: the session itself is read-only.
     #[default]
     ReadOnly,
     /// The session is read-write. Browsing still reads in read-only
-    /// transactions, and a script still cannot write.
+    /// transactions, and a script writes only when it is run to.
     Writable,
 }
 
```

```diff
diff --git a/crates/tabletist-db/src/sql.rs b/crates/tabletist-db/src/sql.rs
index c657b1e..79be23c 100644
--- a/crates/tabletist-db/src/sql.rs
+++ b/crates/tabletist-db/src/sql.rs
@@ -1,7 +1,8 @@
 //! SQL text as tokens, for highlighting, for splitting a script into
-//! statements, and for the read-only guard. Tokenizing never fails; it only
-//! has to agree with the database on where strings, comments and statements
-//! end.
+//! statements, for the guard that keeps a script inside its transaction,
+//! and for telling a statement that writes from one that reads. Tokenizing
+//! never fails; it only has to agree with the database on where strings,
+//! comments and statements end.
 
 use std::ops::Range;
 
@@ -586,9 +587,10 @@ fn word_of(text: &str, token: &Token) -> Option<String> {
     }
 }
 
-/// Why `statement` must not run in Tabletist's read-only transaction: the
-/// statement kind it is, for the message. `None` when it may run. Matched
-/// on tokens, so `SELECT 'COMMIT'` and a column named `end_date` pass.
+/// Why `statement` must not run in a script's transaction, which
+/// Tabletist begins and ends itself, read-only or not: the statement kind
+/// it is, for the message. `None` when it may run. Matched on tokens, so
+/// `SELECT 'COMMIT'` and a column named `end_date` pass.
 pub fn refusal(dialect: Dialect, statement: &str) -> Option<String> {
     let tokens = tokenize(dialect, statement);
     if dialect == Dialect::MySql
```

- [ ] **Step 2: The spec's status**

In `docs/superpowers/specs/2026-10-05-sql-editor-writes-design.md`, the status line becomes:

```markdown
Date: 2026-10-05. Status: step 1 (the run) is built, see
`docs/superpowers/plans/2026-10-05-sql-editor-writes-run.md`; steps 2 and 3
are not yet planned.
```

If a task turned out other than this plan says (a fact about PostgreSQL or MySQL that did not hold, a test that had to change), write it into the spec's section for that driver and into this plan's task, as the repository does after every plan run.

The other documents the spec lists under "Documents this changes" (the SQL editor spec's intent, the value editing spec's promise, the README, the main spec's section 4.3) describe what the app does. The app does not write until step 2, so they change with step 2, not here.

- [ ] **Step 3: Run the four checks**

Expected: PASS, the documentation build among them (it checks the links to `ScriptMode` and `Connection::run_script` in the new text).

- [ ] **Step 4: Commit**

```bash
git add crates/tabletist-db docs/superpowers
git commit -m "Say that a script can be run to write

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

## What steps 2 and 3 will find here

- `Connection::run_script(statements, limit, ScriptMode::Write, stop)` on a writable connection, and `ScriptOutcome::{end, rollback_warning, broken}` to read.
- `sql::kind` to decide whether a run is read-write, and `sql::unbounded` for the question about a missing `WHERE`.
- `Error::Refused { mode, .. }`, `Error::ReadOnly` and `Error::LeftTransaction`, none of which the app words yet beyond their `Display`.
- `StatementOutcome::Done { warnings, .. }`, which the Messages pane does not show yet.
- `src/backend.rs` passing `ScriptMode::ReadOnly` in the one place a run is sent: `Command::RunSql` gains the mode there in step 2.

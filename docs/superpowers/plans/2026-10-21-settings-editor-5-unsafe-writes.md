# Settings › Editor, step 5: Writes without WHERE Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Before a run of the SQL editor that writes to every row of a table (an `UPDATE` or a `DELETE` without `WHERE`, or a `TRUNCATE`), ask first: "Update every row in book_covers?" with the rows the database's statistics give, Cancel / Run anyway on macOS and Windows, `[y] yes  [N] no` in the Omarchy look's status line. Show the row that turns it off, Confirm UPDATE or DELETE without WHERE, in both looks.

**Architecture:** `tabletist-db` reads the statement: `sql::every_row` names the verb and the tables after it, from the tokens `sql::unbounded` reads, which now takes a `LIMIT` or a join's `ON` for a bound and finds a `WITH`'s own `UPDATE` or `DELETE`. `App::run_sql`, the card's run again and Run without timeout send through `send_or_ask`, which holds a run sent to write that has such a statement in a new dialog, `Dialog::ConfirmEveryRow`, with the run's statements, its caps, the database it goes to, and a `WideStatement` for each statement asked about (its line, verb, tables and the sidebar's count of their rows, found as completion finds a name and as each engine folds it). Nothing else runs while it is open. `src/ui/every_row_prompt.rs` draws it: the Components board's dialog on macOS and Windows, and in the terminal look a modal laid over the status line, whose place `workspace::status_line` leaves for it to read. Run anyway sends the run with the mode Run gives it then; after either answer the keyboard goes back into the editor's text if it was there when the question came. Two keymap commands answer it, under a new `When::EveryRow`.

**Tech Stack:** Rust, egui (crmne fork) with AccessKit, headless tests through `src/testing.rs`.

**Spec:** `docs/superpowers/specs/2026-10-19-settings-editor-design.md`, sections "Steps" item 5 and "Step 5: Writes without WHERE".

---

## Before Task 1

Commit the spec's step 5 section and this plan:

```bash
git add docs/superpowers/specs/2026-10-19-settings-editor-design.md docs/superpowers/plans/2026-10-21-settings-editor-5-unsafe-writes.md
git commit -S -m "Plan Settings › Editor step 5: writes without WHERE"
```

## Where this sits

Steps 1 to 4 are merged (#181, #183, #204, #210); the query history (#197) built step 6. This plan is a branch of `main` at `c923a9f6`. A production tab cannot write from the SQL editor yet (`NoWrites::Unconfirmed`), so this question never stands beside the production confirmation.

## Departures, for a yes or a no before code

| Asked, or drawn | Built | Why |
|---|---|---|
| "`TRUNCATE` always asks" | Asks with the row off too, on PostgreSQL and MySQL | The row names UPDATE and DELETE; "always" read as whatever it says. The other reading: TRUNCATE follows the row |
| "with the row estimate from the database's statistics" | The sidebar's count, no query before the question; no number where it has none (SQLite, a schema not loaded) | The catalog's count is that estimate; asking the server first would put a round trip, and a failure, before the question |
| "without WHERE" | A `LIMIT` or a join's `ON` of its own counts as a `WHERE`; a `WITH`'s own `UPDATE` or `DELETE` is asked about | `DELETE FROM logs LIMIT 1000` and a join reach some rows, not every row; a `WITH`'s `DELETE` reaches them all |
| The writes spec (2026-10-05): "`TRUNCATE` is not asked about"; "This UPDATE has no WHERE" | It asks; the question names the table, as this spec's "Update every row in book_covers?" | This spec is newer |
| (the words the boards do not draw) | "Line 3 has no WHERE." / "takes out every row." (TRUNCATE); "Update every row of its table?" where no table can be read; "Run 2 statements that change every row?" with a line each, Omarchy's `lines 1, 3`; a TRUNCATE's tables' rows summed, only where each has a count | Only the one question is drawn |
| Omarchy: the question in the status line | A dialog drawn over the status line, transparent around it | The keyboard leaves the editor, as for every other question, so `y` and `n` type nothing |
| Omarchy `[N]` (the default) | `n`, `N`, Enter and Esc say no; `y` and `Y` yes | The design's capital marks the answer Enter gives; elsewhere a capital is Shift, so both cases answer |
| macOS: Cancel / Run anyway | Enter cancels unless Run anyway has the keyboard | Nothing that writes to every row is the default |
| (several statements) | One question, a line each | One answer for one run |
| (the card's Run again, Run without timeout) | Ask again | Each is a new run that writes |

A reviewer given the draft broke it with tests of its own, and a second read this plan against it. The draft now fixes what they found:

- A `DELETE` or an `UPDATE` inside a `WITH` (`WITH d AS (DELETE FROM books RETURNING *) SELECT ...`) was never asked about: the statement's verb was read as the `SELECT`. A `WITH`'s own verb is read now, bounded by what stands in its own parentheses.
- MySQL's `DELETE FROM logs LIMIT 1000` and `UPDATE ... LIMIT 1`, and a join with `ON`, were asked about as "every row". A `LIMIT` or an `ON` of the statement's own bounds it.
- MySQL's `DELETE b FROM books b ...` named the alias `b`; it names `books` now. `ONLY` after a comma (`TRUNCATE a, ONLY b`) was read as a table; a name of three parts gave its first two; a table named twice was counted twice.
- On PostgreSQL `DELETE FROM "USERS"` found `users`' rows: a name is looked up as each engine reads it now.
- Run anyway could send to another database the tab had switched to, and a plain run of another tab went out under the question. Neither does now.
- In the terminal look a letter typed in the frame the question opens in, while the editor still had the keyboard, both went into the text and answered; Shift+Y and Shift+N (and Caps Lock) answered nothing, though the hint reads `[N]`.
- Whether the editor had the keyboard was kept by tab, and could outlive its question when another dialog took its place; it is kept under the question's own number now. A question whose editor closed is closed.
- `app.rs`'s test of which runs write would have passed without testing its two `DELETE`s.
- The terminal line's count and lines were painted with no node for a screen reader.

The pull request's review (#212) found that any `ON` was taken for a bound, in every dialect and every join: MySQL's `LEFT JOIN` deletes and PostgreSQL's and SQLite's `UPDATE ... FROM a JOIN b ON ...` (whose `ON` does not touch the target) were never asked about. A follow-up commit takes an `ON` for a bound on MySQL only, and only of an inner join (`joins_target`), with a test of each case; the diffs above show the draft before it.

Left as they are, for the record:

- `EXPLAIN (ANALYZE false) DELETE ...` is asked about, though nothing runs: `unbounded` reads `ANALYZE` as an option either way.
- A question taken away by another dialog (none can open over it from the keys) sends nothing, and says nothing.
- Run anyway on a session that came back read-only sends the run read-only, where the database refuses it, as Run would.

## Files

| File | What it does after this plan |
|---|---|
| `crates/tabletist-db/src/sql.rs` | `Verb::Truncate`; `every_row`, `EveryRow`, `NamedTable`: the verb and the tables a statement reaches every row of; `unbounded` bounded by `WHERE`, `LIMIT` and `ON`, and reading a `WITH`'s own verb (`words_of`, `unbounded_at`); `truncate_at`, `tables_after`, `table_at`, `aliases`; `unquoted`, shared with completion. |
| `crates/tabletist-db/src/complete.rs` | A quoted name's text from `sql::unquoted`. |
| `src/model.rs` | `Dialog::ConfirmEveryRow(EveryRowPrompt)`, `WideStatement`, `Action::RunEveryRow` and `CancelEveryRow`; the Omarchy row. |
| `src/app.rs` | `send_or_ask`, `wide_statements`, `answer_every_row`; Run, the card's run again and Run without timeout ask. Its test of which runs write gives its two `DELETE`s a `WHERE`. |
| `src/completion.rs` | `resolve` public to the crate, for the question's tables. |
| `src/keymap.rs` | `RunEveryRow` (the dialog's button; `y` or `Y`) and `CancelEveryRow` (Esc; `n`, `N` or Esc, shown `N`), under `When::EveryRow`. |
| `src/ui/every_row_prompt.rs` | New: the question, as each look asks it. |
| `src/ui/workspace.rs` | `STATUS_LINE`, the line's height; the status line leaves its place for the question. |
| `src/ui/mod.rs` | Shows the question; declares the tests. |
| `src/ui/every_row_tests.rs` | New: this step's headless tests. |
| `src/ui/sql_results.rs`, `src/ui/sql_editor.rs` | Their tests of a run that writes give its `UPDATE` a `WHERE`: they test the run, and the question is the new tests'. |
| `src/settings.rs` | The row in `OptionId::RUNNING`, out of `EDITOR_LATER`. |
| `src/ui/editor_settings_tests.rs` | Step 1's tests meet the row. |
| `docs/_reference/settings-and-files.md`, `docs/_guide/sql-editor.md`, `docs/_guide/omarchy.md`, `docs/_guide/macos.md` | The row, its key, the question and its keys. |

Every task ends with the repository's checks passing:

    cargo fmt --all --check
    cargo clippy --locked --workspace --all-targets -- -D warnings
    cargo test --locked --workspace --all-targets
    RUSTDOCFLAGS='-D warnings' cargo doc --locked --workspace --no-deps

(`cargo` is `~/.cargo/bin/cargo` in these sessions, with `CARGO_TARGET_DIR` at the worktree's `target/`, never `/tmp`.) Commit after each task, signed (`git commit -S`). If signing says "agent refused operation", 1Password is locked: leave the change staged and say so.

---

### Task 1: What reaches every row

**Files:**
- Modify: `crates/tabletist-db/src/sql.rs`, `crates/tabletist-db/src/complete.rs`

- [ ] **Step 1: Write the failing tests**

```diff
diff --git a/crates/tabletist-db/src/sql.rs b/crates/tabletist-db/src/sql.rs
--- a/crates/tabletist-db/src/sql.rs
+++ b/crates/tabletist-db/src/sql.rs
@@ -2456,7 +2456,12 @@ mod tests {
                     "UPDATE books SET title = (SELECT name FROM drafts WHERE drafts.id = books.id)",
                     Verb::Update,
                 ),
-                ("DELETE FROM books ORDER BY id LIMIT 5", Verb::Delete),
+                ("DELETE FROM books ORDER BY id", Verb::Delete),
+                // A LIMIT of a subquery bounds the subquery.
+                (
+                    "UPDATE books SET title = (SELECT name FROM drafts LIMIT 1)",
+                    Verb::Update,
+                ),
                 (
                     "WITH old AS (SELECT id FROM books WHERE id < 5) DELETE FROM books",
                     Verb::Delete,
@@ -2486,6 +2491,9 @@ mod tests {
                 "delete from books where id = 1",
                 "DELETE FROM books WHERE id IN (SELECT book_id FROM drafts)",
                 "UPDATE books SET title = 'x' WHERE CURRENT OF pick",
+                // A LIMIT of its own bounds it too.
+                "DELETE FROM books ORDER BY id LIMIT 5",
+                "UPDATE books SET title = 'x' LIMIT 1",
                 "WITH old AS (SELECT 1) DELETE FROM books WHERE id = 1",
                 "EXPLAIN ANALYZE DELETE FROM books WHERE id = 1",
             ] {
@@ -2538,4 +2546,180 @@ mod tests {
         }
         assert_eq!(unbounded(Dialect::Postgres, "UPDATE"), Some(Verb::Update));
     }
+
+    fn table(schema: Option<&str>, name: &str) -> NamedTable {
+        NamedTable {
+            schema: schema.map(str::to_owned),
+            name: name.to_owned(),
+            quoted: false,
+        }
+    }
+
+    fn quoted(schema: Option<&str>, name: &str) -> NamedTable {
+        NamedTable {
+            quoted: true,
+            ..table(schema, name)
+        }
+    }
+
+    #[test]
+    fn every_row_names_the_verb_and_the_tables_after_it() {
+        let reach = |dialect, text| every_row(dialect, text).expect(text);
+        for dialect in DIALECTS {
+            for (text, verb, tables) in [
+                (
+                    "UPDATE book_covers SET kind = 'ebook'",
+                    Verb::Update,
+                    vec![table(None, "book_covers")],
+                ),
+                (
+                    "update public.book_covers as c set kind = 'x'",
+                    Verb::Update,
+                    vec![table(Some("public"), "book_covers")],
+                ),
+                (
+                    "DELETE FROM book_covers b ORDER BY id",
+                    Verb::Delete,
+                    vec![table(None, "book_covers")],
+                ),
+                (
+                    "WITH old AS (SELECT id FROM covers WHERE id < 5) DELETE FROM books",
+                    Verb::Delete,
+                    vec![table(None, "books")],
+                ),
+                (
+                    "EXPLAIN ANALYZE DELETE FROM /* all */ books",
+                    Verb::Delete,
+                    vec![table(None, "books")],
+                ),
+                // Unreadable: the question names no table.
+                ("UPDATE", Verb::Update, vec![]),
+                ("DELETE FROM (", Verb::Delete, vec![]),
+            ] {
+                let found = reach(dialect, text);
+                assert_eq!(
+                    (found.verb, found.tables),
+                    (verb, tables),
+                    "{dialect:?} {text}"
+                );
+            }
+        }
+        // Each dialect's quotes, and its own words before the table.
+        let pg = reach(
+            Dialect::Postgres,
+            r#"UPDATE ONLY "Book ""Covers""" * SET a = 1"#,
+        );
+        assert_eq!(pg.tables, vec![quoted(None, r#"Book "Covers""#)]);
+        let mysql = reach(
+            Dialect::MySql,
+            "DELETE LOW_PRIORITY QUICK IGNORE FROM `shop`.`books`",
+        );
+        assert_eq!(mysql.tables, vec![quoted(Some("shop"), "books")]);
+        let sqlite = reach(Dialect::Sqlite, "UPDATE OR REPLACE [books] SET a = 1");
+        assert_eq!(sqlite.tables, vec![quoted(None, "books")]);
+        // A name of three parts: the last two are the schema and the table.
+        let three = reach(Dialect::Postgres, "DELETE FROM shop.public.books");
+        assert_eq!(three.tables, vec![table(Some("public"), "books")]);
+        // MySQL's statements of several tables: the ones the verb names,
+        // by the tables their aliases stand for.
+        let several = reach(Dialect::MySql, "UPDATE books b, covers c SET b.a = c.a");
+        assert_eq!(
+            several.tables,
+            vec![table(None, "books"), table(None, "covers")]
+        );
+        let targets = reach(Dialect::MySql, "DELETE b, c FROM shop.books AS b, covers c");
+        assert_eq!(
+            targets.tables,
+            vec![table(Some("shop"), "books"), table(None, "covers")]
+        );
+        let using = reach(Dialect::MySql, "DELETE FROM b USING books b");
+        assert_eq!(using.tables, vec![table(None, "books")]);
+        // A WITH's own DELETE or UPDATE runs whatever reads it.
+        let cte = reach(
+            Dialect::Postgres,
+            "WITH d AS (DELETE FROM books RETURNING *) SELECT count(*) FROM d",
+        );
+        assert_eq!(
+            (cte.verb, cte.tables),
+            (Verb::Delete, vec![table(None, "books")])
+        );
+        let logged = reach(
+            Dialect::Postgres,
+            "WITH u AS (UPDATE books SET a = 1 RETURNING id) INSERT INTO log SELECT id FROM u",
+        );
+        assert_eq!(logged.verb, Verb::Update);
+    }
+
+    #[test]
+    fn a_truncate_always_reaches_every_row_where_the_dialect_has_one() {
+        let pg = every_row(
+            Dialect::Postgres,
+            "TRUNCATE TABLE ONLY books, ONLY public.covers *, books RESTART IDENTITY",
+        );
+        // Each table once.
+        assert_eq!(
+            pg,
+            Some(EveryRow {
+                verb: Verb::Truncate,
+                tables: vec![table(None, "books"), table(Some("public"), "covers")],
+            })
+        );
+        let mysql = every_row(Dialect::MySql, "-- empty it\ntruncate `books`");
+        assert_eq!(
+            mysql.map(|found| found.tables),
+            Some(vec![quoted(None, "books")])
+        );
+        // SQLite has no TRUNCATE: the database refuses the word.
+        assert_eq!(every_row(Dialect::Sqlite, "TRUNCATE books"), None);
+        // `unbounded` still names UPDATE and DELETE alone.
+        assert_eq!(unbounded(Dialect::Postgres, "TRUNCATE books"), None);
+    }
+
+    #[test]
+    fn a_bounded_statement_and_a_read_reach_no_row_of_every_row() {
+        for dialect in DIALECTS {
+            for text in [
+                "UPDATE books SET title = 'x' WHERE id = 1",
+                "DELETE FROM books WHERE id IN (SELECT book_id FROM drafts)",
+                "SELECT * FROM books FOR UPDATE",
+                "INSERT INTO books VALUES (1) ON CONFLICT (id) DO UPDATE SET title = 'x'",
+                "SELECT 'TRUNCATE books'",
+                "-- TRUNCATE books",
+                "EXPLAIN DELETE FROM books",
+                // A LIMIT, or a join's ON, bounds it as a WHERE does.
+                "DELETE FROM logs LIMIT 1000",
+                "UPDATE books JOIN covers ON books.id = covers.book_id SET a = 1",
+                "DELETE b FROM books b JOIN covers c ON b.id = c.book_id",
+                // A WITH's DELETE with a WHERE of its own.
+                "WITH d AS (DELETE FROM books WHERE id = 1 RETURNING *) SELECT * FROM d",
+                "",
+            ] {
+                assert_eq!(every_row(dialect, text), None, "{dialect:?} {text}");
+            }
+        }
+    }
+
+    #[test]
+    fn odd_text_never_panics_the_every_row_check() {
+        for dialect in DIALECTS {
+            for text in [
+                "UPDATE .",
+                "UPDATE a.",
+                "UPDATE a AS",
+                "TRUNCATE ,",
+                "TRUNCATE a,",
+                "DELETE OR",
+                "UPDATE \"",
+                "TRUNCATE `",
+                "DELETE FROM a..b",
+                "TRUNCATE ONLY",
+                "WITH x AS (UPDATE",
+                "WITH x AS (DELETE FROM a) (",
+                "UPDATE [",
+                "DELETE FROM a.b.c",
+            ] {
+                let _ = every_row(dialect, text);
+            }
+        }
+    }
 }
```

- [ ] **Step 2: Run the tests to see them fail**

Run: `cargo test --locked -p tabletist-db --lib sql::`
Expected: FAIL to compile: `sql` has no `every_row`, `EveryRow` or `TableName`, and `Verb` no `Truncate`.

- [ ] **Step 3: Read the verb and its tables**

`unbounded` keeps what it says, and finds its verb's place for `every_row`. A `TRUNCATE` is read where the dialect has one. The tables are the names after the verb and its words (`ONLY`, MySQL's `LOW_PRIORITY QUICK IGNORE FROM`, `TABLE`, SQLite's `OR REPLACE`), each with its schema, its alias passed, separated by commas. Completion's quoted names come out of the same `unquoted`.

```diff
diff --git a/crates/tabletist-db/src/sql.rs b/crates/tabletist-db/src/sql.rs
--- a/crates/tabletist-db/src/sql.rs
+++ b/crates/tabletist-db/src/sql.rs
@@ -1111,11 +1111,13 @@ pub fn kind(dialect: Dialect, statement: &str) -> StatementKind {
     }
 }
 
-/// A statement that changes or removes rows, for [`unbounded`].
+/// A statement that changes or removes every row of a table: what
+/// [`unbounded`] and [`every_row`] name.
 #[derive(Debug, Clone, Copy, PartialEq, Eq)]
 pub enum Verb {
     Update,
     Delete,
+    Truncate,
 }
 
 /// The words a statement's own verb can be, once a `WITH` list and an
@@ -1124,47 +1126,359 @@ const VERBS: [&str; 8] = [
     "SELECT", "INSERT", "UPDATE", "DELETE", "MERGE", "VALUES", "TABLE", "REPLACE",
 ];
 
-/// `UPDATE` or `DELETE` when `statement` is one with no `WHERE` of its
-/// own: it reaches every row of its table. A `WHERE` inside parentheses
-/// belongs to a subquery and does not count. A `WITH` in front is passed,
-/// and so is an `EXPLAIN` that has `ANALYZE`, which runs the statement
-/// behind it. An `INSERT` that updates on a conflict and a `MERGE` are
-/// bounded by their rows, and are never named.
+/// The words that bound an `UPDATE` or a `DELETE` among its own words:
+/// a `WHERE`, a `LIMIT` (MySQL's, and SQLite's where it is built with one)
+/// and the `ON` of a join (MySQL's statements of several tables).
+const BOUNDS: [&str; 3] = ["WHERE", "LIMIT", "ON"];
+
+/// `UPDATE` or `DELETE` when `statement` is one with nothing of its own to
+/// bound it (a `WHERE`, a `LIMIT`, a join's `ON`): it reaches every row of
+/// its table. What stands
+/// inside parentheses belongs to a subquery and does not count. A `WITH`
+/// in front is passed, and so is an `EXPLAIN` that has `ANALYZE`, which
+/// runs the statement behind it. A `WITH`'s own `UPDATE` or `DELETE`
+/// (`WITH d AS (DELETE FROM books RETURNING *) SELECT ...`) is named too:
+/// it runs whatever the statement after it reads. An `INSERT` that updates
+/// on a conflict and a `MERGE` are bounded by their rows, and are never
+/// named; nor is a `TRUNCATE`, which has no `WHERE` to leave out
+/// ([`every_row`] names it).
 pub fn unbounded(dialect: Dialect, statement: &str) -> Option<Verb> {
-    let tokens = tokenize(dialect, statement);
-    // The words outside parentheses, in order.
-    let mut top = Vec::new();
-    let mut depth = 0_usize;
+    unbounded_at(statement, &tokenize(dialect, statement)).map(|(verb, _)| verb)
+}
+
+/// A word of a statement, for [`unbounded_at`].
+struct Word {
+    /// Upper-cased.
+    text: String,
+    /// Its token's place.
+    at: usize,
+    /// The parentheses it stands in: 0 outside all of them, and a number
+    /// of its own for each pair.
+    group: usize,
+    /// It is the first thing inside its parentheses.
+    opens: bool,
+}
+
+/// The words of `tokens`, each with the parentheses it stands in, and
+/// whether an `ANALYZE` stands anywhere (also as an option:
+/// `EXPLAIN (ANALYZE)`).
+fn words_of(statement: &str, tokens: &[Token]) -> (Vec<Word>, bool) {
+    let mut words = Vec::new();
     let mut analyzed = false;
-    for token in &tokens {
+    let mut groups = vec![0];
+    let mut next = 1;
+    let mut opened = false;
+    for (at, token) in tokens.iter().enumerate() {
         match (token.kind, &statement[token.range.clone()]) {
-            (TokenKind::Punctuation, "(") => depth += 1,
-            (TokenKind::Punctuation, ")") => depth = depth.saturating_sub(1),
-            _ => {
-                if let Some(word) = bare_word(statement, token) {
-                    // Also as an option: EXPLAIN (ANALYZE).
-                    analyzed |= ANALYZES.contains(&word.as_str());
-                    if depth == 0 {
-                        top.push(word);
-                    }
+            (TokenKind::Punctuation, "(") => {
+                groups.push(next);
+                next += 1;
+                opened = true;
+            }
+            (TokenKind::Punctuation, ")") => {
+                // A `)` that closes nothing leaves the outermost as it is.
+                if groups.len() > 1 {
+                    groups.pop();
                 }
+                opened = false;
+            }
+            (TokenKind::Whitespace | TokenKind::Comment, _) => {}
+            _ => {
+                if let Some(text) = bare_word(statement, token) {
+                    analyzed |= ANALYZES.contains(&text.as_str());
+                    let group = groups.last().copied().unwrap_or_default();
+                    words.push(Word {
+                        text,
+                        at,
+                        group,
+                        opens: opened,
+                    });
+                }
+                opened = false;
             }
         }
     }
-    let leads = match top.first()?.as_str() {
+    (words, analyzed)
+}
+
+/// [`unbounded`] of `statement`, whose tokens are `tokens`, with the place
+/// in `tokens` of its verb.
+fn unbounded_at(statement: &str, tokens: &[Token]) -> Option<(Verb, usize)> {
+    let (words, analyzed) = words_of(statement, tokens);
+    let mut top = words.iter().filter(|word| word.group == 0);
+    let leads = match top.next()?.text.as_str() {
         "UPDATE" | "DELETE" | "WITH" => true,
         word => EXPLAINS.contains(&word) && analyzed,
     };
     if !leads {
         return None;
     }
-    let at = top.iter().position(|word| VERBS.contains(&word.as_str()))?;
-    let verb = match top[at].as_str() {
-        "UPDATE" => Verb::Update,
-        "DELETE" => Verb::Delete,
-        _ => return None,
+    // The statement's own verb: the first word outside parentheses that
+    // can be one.
+    let own = words
+        .iter()
+        .find(|word| word.group == 0 && VERBS.contains(&word.text.as_str()));
+    // A WITH's statements: the verbs that open their parentheses.
+    let ctes = words.iter().filter(|word| word.group > 0 && word.opens);
+    let bounded = |verb: &Word| {
+        let after = words.iter().filter(|word| word.at > verb.at);
+        after
+            .filter(|word| word.group == verb.group)
+            .any(|word| BOUNDS.contains(&word.text.as_str()))
     };
-    (!top[at + 1..].iter().any(|word| word == "WHERE")).then_some(verb)
+    own.into_iter().chain(ctes).find_map(|word| {
+        let verb = match word.text.as_str() {
+            "UPDATE" => Verb::Update,
+            "DELETE" => Verb::Delete,
+            _ => return None,
+        };
+        (!bounded(word)).then_some((verb, word.at))
+    })
+}
+
+/// A statement that reaches every row of the tables it names.
+#[derive(Debug, Clone, PartialEq, Eq)]
+pub struct EveryRow {
+    pub verb: Verb,
+    /// The tables it names, each once, in order. Empty where they cannot
+    /// be read off the statement.
+    pub tables: Vec<NamedTable>,
+}
+
+/// A table as a statement names it.
+#[derive(Debug, Clone, PartialEq, Eq)]
+pub struct NamedTable {
+    pub schema: Option<String>,
+    /// Without its quotes.
+    pub name: String,
+    /// The name was quoted: it is spelled as the database has it.
+    pub quoted: bool,
+}
+
+/// What reaches every row of a table: an `UPDATE` or a `DELETE` that
+/// [`unbounded`] names, or a `TRUNCATE` where the dialect has one (SQLite
+/// does not). Its tables are the ones after the verb, in order: `books`
+/// of `UPDATE ONLY books b SET`, both of MySQL's `DELETE a, b FROM ...`
+/// (by the tables their aliases stand for) and of PostgreSQL's
+/// `TRUNCATE a, b`.
+pub fn every_row(dialect: Dialect, statement: &str) -> Option<EveryRow> {
+    let tokens = tokenize(dialect, statement);
+    let (verb, at) = match unbounded_at(statement, &tokens) {
+        Some(found) => found,
+        None => (Verb::Truncate, truncate_at(dialect, statement, &tokens)?),
+    };
+    let after = code_of(&tokens[at + 1..]);
+    let (named, end) = tables_after(statement, &after);
+    // MySQL's DELETE of several tables names them by the aliases its FROM
+    // or USING, after them, gives them: `DELETE b FROM books b`.
+    let aliases = aliases(statement, &after[end..]);
+    let mut tables: Vec<NamedTable> = Vec::new();
+    for table in named {
+        let alias = aliases
+            .iter()
+            .find(|(alias, _)| table.schema.is_none() && alias.eq_ignore_ascii_case(&table.name));
+        let table = alias.map_or(table, |(_, aliased)| aliased.clone());
+        if !tables.contains(&table) {
+            tables.push(table);
+        }
+    }
+    Some(EveryRow { verb, tables })
+}
+
+/// `tokens` without their whitespace and comments.
+fn code_of(tokens: &[Token]) -> Vec<&Token> {
+    let code = |token: &&Token| !matches!(token.kind, TokenKind::Whitespace | TokenKind::Comment);
+    tokens.iter().filter(code).collect()
+}
+
+/// The place in `tokens` of the `TRUNCATE` that begins `statement`, where
+/// the dialect has the statement.
+fn truncate_at(dialect: Dialect, statement: &str, tokens: &[Token]) -> Option<usize> {
+    let truncates = match dialect {
+        Dialect::Postgres | Dialect::MySql => true,
+        Dialect::Sqlite => false,
+    };
+    let (at, first) = tokens
+        .iter()
+        .enumerate()
+        .find(|(_, token)| !matches!(token.kind, TokenKind::Whitespace | TokenKind::Comment))?;
+    let word = bare_word(statement, first);
+    (truncates && word.as_deref() == Some("TRUNCATE")).then_some(at)
+}
+
+/// The words that may stand between a verb and its first table:
+/// `UPDATE ONLY`, MySQL's `DELETE LOW_PRIORITY QUICK IGNORE FROM`,
+/// `TRUNCATE TABLE ONLY`. SQLite's `OR` takes the word after it with it
+/// (`UPDATE OR REPLACE`). `ONLY` may stand before each table.
+const BEFORE_TABLES: [&str; 7] = [
+    "ONLY",
+    "LOW_PRIORITY",
+    "QUICK",
+    "IGNORE",
+    "FROM",
+    "TABLE",
+    "OR",
+];
+
+/// The words that end the list of a verb's tables, though they are not
+/// quoted: where an unquoted alias would stand.
+const AFTER_TABLES: [&str; 17] = [
+    "SET",
+    "FROM",
+    "USING",
+    "WHERE",
+    "JOIN",
+    "INNER",
+    "LEFT",
+    "RIGHT",
+    "CROSS",
+    "NATURAL",
+    "STRAIGHT_JOIN",
+    "RESTART",
+    "CONTINUE",
+    "CASCADE",
+    "RETURNING",
+    "ORDER",
+    "LIMIT",
+];
+
+/// The name `token` spells, without quotes, and whether it was quoted.
+/// With `keywords`, a keyword too, where it is none of [`AFTER_TABLES`].
+fn name_of(statement: &str, token: &Token, keywords: bool) -> Option<(String, bool)> {
+    let text = &statement[token.range.clone()];
+    match token.kind {
+        TokenKind::QuotedIdentifier => Some((unquoted(text), true)),
+        TokenKind::Identifier => Some((text.to_owned(), false)),
+        TokenKind::Keyword if keywords => {
+            let word = text.to_ascii_uppercase();
+            (!AFTER_TABLES.contains(&word.as_str())).then(|| (text.to_owned(), false))
+        }
+        _ => None,
+    }
+}
+
+/// The table named at `code[at]`, of as many parts as it has (the last two
+/// count: `db.schema.table`), and the place after it.
+fn table_at(
+    statement: &str,
+    code: &[&Token],
+    at: usize,
+    keywords: bool,
+) -> Option<(NamedTable, usize)> {
+    let part = |at: usize| name_of(statement, code.get(at)?, keywords);
+    let dot = |at: usize| {
+        code.get(at)
+            .is_some_and(|token| &statement[token.range.clone()] == ".")
+    };
+    let mut parts = vec![part(at)?];
+    let mut next = at + 1;
+    while dot(next)
+        && let Some(more) = part(next + 1)
+    {
+        parts.push(more);
+        next += 2;
+    }
+    let (name, quoted) = parts.pop()?;
+    let schema = parts.pop().map(|(schema, _)| schema);
+    Some((
+        NamedTable {
+            schema,
+            name,
+            quoted,
+        },
+        next,
+    ))
+}
+
+/// The tables named at the start of `code`, the ones after a verb: a list
+/// of names, each with its schema where it has one and its alias passed,
+/// separated by commas. With the place in `code` where the list ends.
+fn tables_after(statement: &str, code: &[&Token]) -> (Vec<NamedTable>, usize) {
+    let text = |at: usize| code.get(at).map(|token| &statement[token.range.clone()]);
+    let word = |at: usize| code.get(at).and_then(|token| bare_word(statement, token));
+    let mut at = 0;
+    while let Some(word) = word(at).filter(|word| BEFORE_TABLES.contains(&word.as_str())) {
+        at += if word == "OR" { 2 } else { 1 };
+    }
+    let mut tables = Vec::new();
+    loop {
+        if word(at).as_deref() == Some("ONLY") {
+            at += 1;
+        }
+        let Some((table, next)) = table_at(statement, code, at, true) else {
+            break;
+        };
+        tables.push(table);
+        at = next;
+        // PostgreSQL's `*` after a table takes its children in.
+        if text(at) == Some("*") {
+            at += 1;
+        }
+        if word(at).as_deref() == Some("AS") {
+            at += 2;
+        } else if code
+            .get(at)
+            .and_then(|token| name_of(statement, token, true))
+            .is_some()
+            && text(at + 1) != Some(".")
+        {
+            at += 1;
+        }
+        if text(at) != Some(",") {
+            break;
+        }
+        at += 1;
+    }
+    (tables, at.min(code.len()))
+}
+
+/// The aliases `code` gives tables: `books b`, `shop.books AS b`, each
+/// alias with its table. Names only, not keywords: an approximate scan.
+fn aliases(statement: &str, code: &[&Token]) -> Vec<(String, NamedTable)> {
+    let text = |at: usize| code.get(at).map(|token| &statement[token.range.clone()]);
+    let mut found = Vec::new();
+    let mut at = 0;
+    while at < code.len() {
+        let Some((table, after)) = table_at(statement, code, at, false) else {
+            at += 1;
+            continue;
+        };
+        let as_word = code
+            .get(after)
+            .and_then(|token| bare_word(statement, token))
+            .is_some_and(|word| word == "AS");
+        let alias_at = after + usize::from(as_word);
+        let alias = code
+            .get(alias_at)
+            .and_then(|token| name_of(statement, token, false))
+            .filter(|_| text(alias_at + 1) != Some("."));
+        match alias {
+            Some((alias, _)) => {
+                found.push((alias, table));
+                at = alias_at + 1;
+            }
+            None => at = after,
+        }
+    }
+    found
+}
+
+/// The name a quoted identifier spells: its quotes taken off and a
+/// doubled quote inside it made one. `[name]` has no doubling to undo; an
+/// unterminated one no closing quote to take off.
+pub(crate) fn unquoted(quoted: &str) -> String {
+    let mut chars = quoted.chars();
+    let Some(open) = chars.next() else {
+        return String::new();
+    };
+    let close = if open == '[' { ']' } else { open };
+    let inner = chars.as_str();
+    let inner = inner.strip_suffix(close).unwrap_or(inner);
+    if open == '[' {
+        inner.to_owned()
+    } else {
+        inner.replace(&format!("{open}{open}"), &open.to_string())
+    }
 }
 
 #[cfg(test)]
```

```diff
diff --git a/crates/tabletist-db/src/complete.rs b/crates/tabletist-db/src/complete.rs
--- a/crates/tabletist-db/src/complete.rs
+++ b/crates/tabletist-db/src/complete.rs
@@ -4,7 +4,7 @@
 
 use std::ops::Range;
 
-use crate::sql::{Token, TokenKind, is_code};
+use crate::sql::{Token, TokenKind, is_code, unquoted};
 
 /// Where a completion would go, and what belongs there.
 #[derive(Debug, Clone, PartialEq, Eq)]
@@ -121,21 +121,9 @@ impl Piece<'_> {
 
     /// The name it spells, without its quotes.
     fn name(&self) -> String {
-        if self.kind != TokenKind::QuotedIdentifier {
-            return self.text.to_owned();
-        }
-        let mut chars = self.text.chars();
-        let Some(open) = chars.next() else {
-            return String::new();
-        };
-        let close = if open == '[' { ']' } else { open };
-        // An unterminated name has no closing quote to strip.
-        let inner = chars.as_str();
-        let inner = inner.strip_suffix(close).unwrap_or(inner);
-        if open == '[' {
-            inner.to_owned()
-        } else {
-            inner.replace(&format!("{open}{open}"), &open.to_string())
+        match self.kind {
+            TokenKind::QuotedIdentifier => unquoted(self.text),
+            _ => self.text.to_owned(),
         }
     }
 }
```

- [ ] **Step 4: Run the tests to see them pass**

Run: `cargo test --locked -p tabletist-db --lib -- sql:: complete::`
Expected: PASS. Then the four checks above.

- [ ] **Step 5: Commit**

```bash
git add crates/tabletist-db/src/sql.rs crates/tabletist-db/src/complete.rs
git commit -S -m "Read the statements that reach every row of a table, and their tables"
```

---

### Task 2: The question

**Files:**
- Modify: `src/model.rs`, `src/app.rs`, `src/completion.rs`, `src/keymap.rs`, `src/ui/workspace.rs`, `src/ui/mod.rs`
- Create: `src/ui/every_row_prompt.rs`, `src/ui/every_row_tests.rs`
- Modify (tests): `src/ui/sql_results.rs`, `src/ui/sql_editor.rs`, `src/app.rs`

- [ ] **Step 1: Write the failing tests**

The tests of a run that writes in `sql_results.rs`, `sql_editor.rs` and `app.rs` run an `UPDATE` or a `DELETE` without `WHERE`; they test the run, so it gets a `WHERE`, and the question is tested on its own. (Left as they were, `app.rs`'s would pass without testing: its assert reads the run sent before.)

```diff
diff --git a/src/ui/every_row_tests.rs b/src/ui/every_row_tests.rs
new file mode 100644
--- /dev/null
+++ b/src/ui/every_row_tests.rs
@@ -0,0 +1,339 @@
+//! The question before a run that changes every row of a table, in both
+//! looks: which runs ask, what the question says, and that nothing is
+//! sent before Run anyway.
+
+use egui::{Key, Modifiers};
+
+use crate::backend::Command;
+use crate::model::{Action, ConnTabId, Dialog, RunMode, TabId};
+use crate::settings::{OptionValue, RunScope};
+use crate::testing::Harness;
+use crate::theme::Look;
+use tabletist_db::ScriptMode;
+
+use super::appearance_settings_tests::shows;
+
+fn looks() -> [Look; 2] {
+    [Look::macos(), Look::omarchy()]
+}
+
+/// A writable connection in `look`, with a new SQL editor in `mode` that
+/// has the keyboard and holds `text`, its cursor at the end.
+fn editor(look: Look, mode: RunMode, text: &str) -> (Harness, ConnTabId, TabId) {
+    let mut harness = Harness::new();
+    harness.set_look(look);
+    let tab = harness.connect_fake_as(false);
+    harness.press(Key::T, Modifiers::COMMAND);
+    let id = harness.app.workspace(tab).unwrap().active_tab.unwrap();
+    harness.app.apply(Action::SetSqlMode {
+        tab,
+        sql_tab: id,
+        mode,
+    });
+    harness.frame(vec![egui::Event::Paste(text.into())]);
+    harness.settle();
+    (harness, tab, id)
+}
+
+/// The runs sent so far: each one's statements and mode.
+fn runs(harness: &Harness) -> Vec<(Vec<String>, ScriptMode)> {
+    let sent = harness.app.backend.sent.iter();
+    sent.filter_map(|command| match command {
+        Command::RunSql {
+            statements, mode, ..
+        } => Some((statements.iter().map(|s| s.text.clone()).collect(), *mode)),
+        _ => None,
+    })
+    .collect()
+}
+
+fn asks(harness: &Harness) -> bool {
+    matches!(harness.app.dialog, Some(Dialog::ConfirmEveryRow(_)))
+}
+
+fn run_key(harness: &mut Harness) {
+    harness.press(Key::Enter, Modifiers::COMMAND);
+    harness.settle();
+}
+
+fn type_key(harness: &mut Harness, key: Key, text: &str) {
+    super::tests::type_key(harness, key, text);
+}
+
+#[test]
+fn an_update_without_where_asks_and_sends_nothing_before_run_anyway() {
+    // macOS, and Windows, which asks the same.
+    for look in [Look::macos(), Look::standard()] {
+        let (mut harness, _, _) = editor(look, RunMode::ReadWrite, "UPDATE users SET a = 1");
+        run_key(&mut harness);
+        assert!(asks(&harness), "{}", look.name);
+        assert!(runs(&harness).is_empty());
+        assert!(harness.has("Update every row in users?"));
+        // The fixture's statistics say 1,200,000 rows of users.
+        let said = "Line 1 has no WHERE. About 1,200,000 rows, by the database's statistics.";
+        assert!(harness.has(said), "{}", look.name);
+        harness.click("Run anyway");
+        harness.settle();
+        assert!(!asks(&harness));
+        assert_eq!(
+            runs(&harness),
+            [(vec!["UPDATE users SET a = 1".to_owned()], ScriptMode::Write)]
+        );
+        // The editor has the keyboard again.
+        assert!(harness.ctx.text_edit_focused(), "{}", look.name);
+    }
+}
+
+#[test]
+fn cancel_esc_and_enter_send_nothing_and_give_the_editor_back() {
+    let (mut harness, _, _) = editor(Look::macos(), RunMode::ReadWrite, "DELETE FROM orders");
+    run_key(&mut harness);
+    assert!(harness.has("Delete every row in orders?"));
+    assert!(harness.has("Line 1 has no WHERE. About 3 rows, by the database's statistics."));
+    harness.click("Cancel");
+    harness.settle();
+    assert!(!asks(&harness));
+    assert!(harness.ctx.text_edit_focused());
+    run_key(&mut harness);
+    harness.press(Key::Escape, Modifiers::NONE);
+    harness.settle();
+    assert!(!asks(&harness));
+    // Enter with the keyboard on no button cancels: nothing that writes to
+    // every row is the default.
+    run_key(&mut harness);
+    harness.press(Key::Enter, Modifiers::NONE);
+    harness.settle();
+    assert!(!asks(&harness));
+    assert!(runs(&harness).is_empty());
+    // A held run key's repeat, with its modifier, answers nothing.
+    run_key(&mut harness);
+    harness.press(Key::Enter, Modifiers::COMMAND);
+    harness.settle();
+    assert!(asks(&harness));
+    assert!(runs(&harness).is_empty());
+}
+
+#[test]
+fn a_bounded_write_a_read_and_a_read_only_tab_never_ask() {
+    for look in looks() {
+        let bounded = "UPDATE users SET a = 1 WHERE id = 1";
+        let (mut harness, _, _) = editor(look, RunMode::ReadWrite, bounded);
+        run_key(&mut harness);
+        assert!(!asks(&harness), "{}", look.name);
+        assert_eq!(
+            runs(&harness),
+            [(vec![bounded.to_owned()], ScriptMode::Write)]
+        );
+        // A read in Read-write is a read-only run.
+        let (mut harness, _, _) = editor(look, RunMode::ReadWrite, "SELECT * FROM users");
+        run_key(&mut harness);
+        assert!(!asks(&harness), "{}", look.name);
+        assert_eq!(runs(&harness)[0].1, ScriptMode::ReadOnly);
+        // On a tab in Read-only the run is read-only: the database refuses
+        // the write, and the card offers to run it again.
+        let (mut harness, _, _) = editor(look, RunMode::ReadOnly, "DELETE FROM users");
+        run_key(&mut harness);
+        assert!(!asks(&harness), "{}", look.name);
+        assert_eq!(runs(&harness)[0].1, ScriptMode::ReadOnly);
+    }
+}
+
+#[test]
+fn a_table_the_sidebar_does_not_know_is_asked_about_without_rows() {
+    let (mut harness, _, _) = editor(Look::macos(), RunMode::ReadWrite, "UPDATE drafts SET a = 1");
+    run_key(&mut harness);
+    assert!(harness.has("Update every row in drafts?"));
+    assert!(harness.has("Line 1 has no WHERE."));
+}
+
+#[test]
+fn several_statements_are_asked_about_in_one_question() {
+    let text = "UPDATE users SET a = 1;\nSELECT 1;\nDELETE FROM orders";
+    let (mut harness, _, _) = editor(Look::macos(), RunMode::ReadWrite, text);
+    harness
+        .app
+        .apply(Action::SetOption(OptionValue::RunScope(RunScope::All)));
+    run_key(&mut harness);
+    assert!(harness.has("Run 2 statements that change every row?"));
+    assert!(harness.has("Line 1: Update every row in users, about 1,200,000 rows"));
+    assert!(harness.has("Line 3: Delete every row in orders, about 3 rows"));
+    harness.click("Run anyway");
+    harness.settle();
+    assert_eq!(runs(&harness)[0].0.len(), 3);
+}
+
+#[test]
+fn off_the_setting_lets_update_and_delete_run_and_truncate_still_asks() {
+    for text in ["DELETE FROM users", "UPDATE users SET a = 1"] {
+        let (mut harness, _, _) = editor(Look::macos(), RunMode::ReadWrite, text);
+        harness
+            .app
+            .apply(Action::SetOption(OptionValue::ConfirmUnsafeWrites(false)));
+        run_key(&mut harness);
+        assert!(!asks(&harness), "{text}");
+        assert_eq!(runs(&harness).len(), 1, "{text}");
+    }
+    // SQLite has no TRUNCATE; PostgreSQL does.
+    let text = "TRUNCATE users, orders";
+    let (mut harness, tab, _) = editor(Look::macos(), RunMode::ReadWrite, text);
+    harness
+        .app
+        .apply(Action::SetOption(OptionValue::ConfirmUnsafeWrites(false)));
+    harness.app.workspace_mut(tab).unwrap().driver = tabletist_db::Driver::Postgres;
+    run_key(&mut harness);
+    assert!(harness.has("Truncate users and orders?"));
+    assert!(
+        harness
+            .has("Line 1 takes out every row. About 1,200,003 rows, by the database's statistics.")
+    );
+}
+
+#[test]
+fn the_terminal_asks_in_the_status_line_and_takes_its_letters() {
+    let text = "update users set a = 1";
+    let (mut harness, tab, id) = editor(Look::omarchy(), RunMode::ReadWrite, text);
+    run_key(&mut harness);
+    assert!(asks(&harness));
+    // In the status line's place, at the window's foot.
+    let asked = harness
+        .painted_rect("update every row in users?")
+        .expect("the question");
+    let line = harness
+        .ctx
+        .data(|data| data.get_temp::<egui::Rect>(super::every_row_prompt::status_line_id(tab)))
+        .expect("the status line");
+    assert!(line.contains(asked.center()), "{asked:?} in {line:?}");
+    assert!(harness.has("update every row in users? about 1,200,000 rows · line 1"));
+    assert!(shows(&mut harness, "[y] yes"));
+    assert!(shows(&mut harness, "[N] no"));
+    // `n` answers no, and types nothing into the editor.
+    type_key(&mut harness, Key::N, "n");
+    harness.settle();
+    assert!(!asks(&harness));
+    let sql = harness.app.workspace(tab).unwrap().sql_tab(id).unwrap();
+    assert_eq!(sql.text, text);
+    // Enter and Esc answer no too.
+    for key in [Key::Enter, Key::Escape] {
+        run_key(&mut harness);
+        assert!(asks(&harness));
+        harness.press(key, Modifiers::NONE);
+        harness.settle();
+        assert!(!asks(&harness), "{key:?}");
+    }
+    assert!(runs(&harness).is_empty());
+    // `N`, as the hint writes it, too.
+    run_key(&mut harness);
+    type_key(&mut harness, Key::N, "N");
+    harness.settle();
+    assert!(!asks(&harness));
+    assert!(runs(&harness).is_empty());
+    // `y` runs it.
+    run_key(&mut harness);
+    type_key(&mut harness, Key::Y, "y");
+    harness.settle();
+    assert!(!asks(&harness));
+    assert_eq!(runs(&harness), [(vec![text.to_owned()], ScriptMode::Write)]);
+    let sql = harness.app.workspace(tab).unwrap().sql_tab(id).unwrap();
+    assert_eq!(sql.text, text);
+}
+
+#[test]
+fn the_keyboard_goes_back_only_where_it_was() {
+    // Out of the text in the terminal look, the run key still runs; the
+    // answer leaves the keyboard out of the text.
+    let (mut harness, tab, id) = editor(Look::omarchy(), RunMode::ReadWrite, "delete from users");
+    harness.press(Key::Escape, Modifiers::NONE);
+    harness.settle();
+    assert!(!harness.ctx.text_edit_focused());
+    run_key(&mut harness);
+    assert!(asks(&harness));
+    type_key(&mut harness, Key::N, "n");
+    harness.settle();
+    assert!(!asks(&harness));
+    assert!(!harness.ctx.text_edit_focused());
+    // In the text, it goes back into the text.
+    let workspace = harness.app.workspace_mut(tab).unwrap();
+    workspace.sql_tab_mut(id).unwrap().focus_editor = true;
+    harness.settle();
+    assert!(harness.ctx.text_edit_focused());
+    run_key(&mut harness);
+    type_key(&mut harness, Key::N, "n");
+    harness.settle();
+    assert!(harness.ctx.text_edit_focused());
+}
+
+#[test]
+fn a_run_of_the_refused_write_card_asks_too() {
+    // Run read-only, then again read-write from the card: the run that
+    // writes is the one asked about.
+    let (mut harness, tab, id) = editor(Look::macos(), RunMode::ReadOnly, "DELETE FROM users");
+    run_key(&mut harness);
+    assert_eq!(runs(&harness).len(), 1);
+    let refused = crate::testing::script_outcome(vec![crate::testing::refused_write()]);
+    harness.answer_sql(Ok(refused), None);
+    harness.app.apply(Action::SetSqlMode {
+        tab,
+        sql_tab: id,
+        mode: RunMode::ReadWrite,
+    });
+    harness.app.apply(Action::RunSqlAgain { tab, sql_tab: id });
+    harness.settle();
+    assert!(asks(&harness));
+    assert_eq!(runs(&harness).len(), 1);
+}
+
+#[test]
+fn run_without_timeout_asks_again() {
+    let (mut harness, tab, id) = editor(Look::macos(), RunMode::ReadWrite, "DELETE FROM users");
+    run_key(&mut harness);
+    harness.click("Run anyway");
+    harness.settle();
+    // The timeout stopped it, and nothing was written.
+    let stopped = crate::testing::script_outcome(vec![tabletist_db::StatementOutcome::Cancelled]);
+    let timeout = crate::backend::CancelReason::Timeout(std::time::Duration::from_secs(30));
+    harness.answer_sql(Ok(stopped), Some(timeout));
+    harness.settle();
+    harness
+        .app
+        .apply(Action::RunWithoutTimeout { tab, sql_tab: id });
+    harness.settle();
+    assert!(asks(&harness));
+    assert_eq!(runs(&harness).len(), 1);
+}
+
+#[test]
+fn nothing_else_runs_under_the_question_and_a_switched_database_sends_nothing() {
+    let (mut harness, tab, _) = editor(Look::macos(), RunMode::ReadWrite, "DELETE FROM users");
+    let other = harness.add_sql_tab(tab);
+    run_key(&mut harness);
+    assert!(asks(&harness));
+    // Another editor's run, asked for under the question: not sent.
+    harness.app.apply(Action::RunSql {
+        tab,
+        sql_tab: other,
+        all: true,
+    });
+    assert!(runs(&harness).is_empty());
+    // The tab went to another database meanwhile: Run anyway sends nothing
+    // there.
+    let workspace = harness.app.workspace_mut(tab).unwrap();
+    workspace.spec = tabletist_db::ConnectSpec::sqlite("/tmp/another.db");
+    harness.app.apply(Action::RunEveryRow { refocus: false });
+    assert!(!asks(&harness));
+    assert!(runs(&harness).is_empty());
+}
+
+#[test]
+fn a_postgres_name_is_looked_up_as_postgres_folds_it() {
+    let counted = "Line 1 has no WHERE. About 1,200,000 rows, by the database's statistics.";
+    for (text, said) in [
+        ("DELETE FROM USERS", counted),
+        // Quoted, it is that name exactly, which the sidebar does not have.
+        (r#"DELETE FROM "USERS""#, "Line 1 has no WHERE."),
+    ] {
+        let (mut harness, tab, _) = editor(Look::macos(), RunMode::ReadWrite, text);
+        harness.app.workspace_mut(tab).unwrap().driver = tabletist_db::Driver::Postgres;
+        run_key(&mut harness);
+        assert!(harness.has(said), "{text}");
+    }
+}
```

```diff
diff --git a/src/ui/mod.rs b/src/ui/mod.rs
--- a/src/ui/mod.rs
+++ b/src/ui/mod.rs
@@ -35,6 +35,8 @@ mod editor_results_tests;
 mod editor_settings_tests;
 #[cfg(test)]
 mod env_tests;
+#[cfg(test)]
+mod every_row_tests;
 pub mod filter_bar;
 pub mod focus;
 pub mod format;
```

```diff
diff --git a/src/ui/sql_results.rs b/src/ui/sql_results.rs
--- a/src/ui/sql_results.rs
+++ b/src/ui/sql_results.rs
@@ -2962,7 +2962,7 @@ mod tests {
         use crate::model::RunMode;
         use tabletist_db::ScriptMode;
         for look in Look::ALL {
-            let update = "UPDATE users SET email = 'x'";
+            let update = "UPDATE users SET email = 'x' WHERE id = 1";
             let (mut harness, tab) = writable(look, update);
             set_mode(&mut harness, tab, RunMode::ReadOnly);
             run(&mut harness);
@@ -3065,7 +3065,7 @@ mod tests {
     #[test]
     fn on_production_the_card_says_why_the_tab_cannot_write_and_offers_nothing() {
         for look in Look::ALL {
-            let (mut harness, tab) = writable(look, "UPDATE users SET email = 'x'");
+            let (mut harness, tab) = writable(look, "UPDATE users SET email = 'x' WHERE id = 1");
             // The tab an editor opened on production is: in Read-only.
             set_mode(&mut harness, tab, crate::model::RunMode::ReadOnly);
             harness.app.workspace_mut(tab).unwrap().environment =
@@ -3097,7 +3097,7 @@ mod tests {
     /// A refused `UPDATE` in a tab switched to Read-only on a writable
     /// connection, drawn in `look`: the card offers to allow writes.
     fn refused_in_a_read_only_tab(look: Look) -> (Harness, ConnTabId) {
-        let (mut harness, tab) = writable(look, "UPDATE users SET email = 'x'");
+        let (mut harness, tab) = writable(look, "UPDATE users SET email = 'x' WHERE id = 1");
         set_mode(&mut harness, tab, crate::model::RunMode::ReadOnly);
         run(&mut harness);
         harness.answer_sql(Ok(script_outcome(vec![refused_write()])), None);
@@ -3188,7 +3188,7 @@ mod tests {
             assert_eq!(runs_sent(&harness), sent, "{}", look.name);
             // Nor does `e` open the dialog of a connection that opens
             // read-only.
-            let (mut harness, tab) = editor(look, "UPDATE users SET email = 'x'");
+            let (mut harness, tab) = editor(look, "UPDATE users SET email = 'x' WHERE id = 1");
             harness.app.workspace_mut(tab).unwrap().driver = Driver::Postgres;
             run(&mut harness);
             harness.answer_sql(Ok(script_outcome(vec![refused_write()])), None);
@@ -3251,7 +3251,7 @@ mod tests {
     fn a_read_only_production_connection_is_not_told_to_turn_the_box_off() {
         for look in Look::ALL {
             // Production opens read-only unless its box says otherwise.
-            let (mut harness, tab) = editor(look, "UPDATE users SET email = 'x'");
+            let (mut harness, tab) = editor(look, "UPDATE users SET email = 'x' WHERE id = 1");
             let workspace = harness.app.workspace_mut(tab).unwrap();
             workspace.driver = Driver::Postgres;
             workspace.environment = crate::env::Environment::Production;
@@ -3323,7 +3323,7 @@ mod tests {
     #[test]
     fn a_refused_write_in_a_short_pane_scrolls_to_its_last_line() {
         for look in Look::ALL {
-            let (mut harness, tab) = editor(look, "UPDATE users SET email = 'x'");
+            let (mut harness, tab) = editor(look, "UPDATE users SET email = 'x' WHERE id = 1");
             let workspace = harness.app.workspace_mut(tab).unwrap();
             workspace.driver = tabletist_db::Driver::Postgres;
             // The editor takes most of the height: the results have room
@@ -3490,7 +3490,7 @@ mod tests {
     }
 
     /// Three statements that change rows.
-    const CHANGES: &str = "INSERT INTO notes VALUES (1);\nUPDATE notes SET seen = 1;\nDELETE FROM notes WHERE seen = 0";
+    const CHANGES: &str = "INSERT INTO notes VALUES (1);\nUPDATE notes SET seen = 1 WHERE seen = 0;\nDELETE FROM notes WHERE seen = 0";
 
     #[test]
     fn a_committed_run_says_so_after_what_each_statement_changed() {
@@ -3976,7 +3976,8 @@ mod tests {
 
     #[test]
     fn a_statements_warnings_are_counted_on_its_line() {
-        let script = "INSERT INTO notes VALUES ('too long');\nUPDATE notes SET n = 'x'";
+        let script =
+            "INSERT INTO notes VALUES ('too long');\nUPDATE notes SET n = 'x' WHERE n = ''";
         let (mut harness, _tab) = writing(Look::standard(), Driver::MySql, script);
         let warned = |affected, warnings| StatementOutcome::Done {
             affected: Some(affected),
```

```diff
diff --git a/src/ui/sql_editor.rs b/src/ui/sql_editor.rs
--- a/src/ui/sql_editor.rs
+++ b/src/ui/sql_editor.rs
@@ -1688,7 +1688,7 @@ mod tests {
             ),
         ] {
             for look in Look::ALL {
-                let script = "CREATE TABLE notes (n);\nUPDATE notes SET n = 1";
+                let script = "CREATE TABLE notes (n);\nUPDATE notes SET n = 1 WHERE n = 0";
                 let (mut harness, tab, id) = editor(look, script);
                 set_mode(&mut harness, tab, id, RunMode::ReadWrite);
                 harness.press(Key::Enter, Modifiers::COMMAND | Modifiers::SHIFT);
```

```diff
diff --git a/src/app.rs b/src/app.rs
--- a/src/app.rs
+++ b/src/app.rs
@@ -13227,13 +13227,13 @@ masked = ["orders.email"]
             ("PRAGMA table_info(users)", ReadOnly),
             ("-- UPDATE in a note\nSELECT 'DELETE'", ReadOnly),
             ("INSERT INTO users (email) VALUES ('a')", Write),
-            ("DELETE FROM users", Write),
+            ("DELETE FROM users WHERE id = 1", Write),
             ("CREATE TABLE notes (seen int)", Write),
             (
                 "WITH gone AS (DELETE FROM users RETURNING id) SELECT * FROM gone",
                 Write,
             ),
-            ("EXPLAIN ANALYZE DELETE FROM users", Write),
+            ("EXPLAIN ANALYZE DELETE FROM users WHERE id = 1", Write),
             // One write among reads is enough.
             ("SELECT 1; DROP TABLE users; SELECT 2", Write),
         ] {
```

- [ ] **Step 2: Run the tests to see them fail**

Run: `cargo test --locked --lib -- every_row_tests sql_results sql_editor`
Expected: FAIL to compile: `Dialog` has no `ConfirmEveryRow`, `Action` no `RunEveryRow`, and `ui` no `every_row_prompt`.

- [ ] **Step 3: Ask before the run**

```diff
diff --git a/src/model.rs b/src/model.rs
--- a/src/model.rs
+++ b/src/model.rs
@@ -630,6 +630,16 @@ pub enum Action {
     ConfirmWrite,
     /// Close the production confirmation and send nothing.
     CancelWrite,
+    /// The question before a run that reaches every row of a table: Run
+    /// anyway sends the run it holds. `refocus`: the editor had the
+    /// keyboard when the question came, and has it back.
+    RunEveryRow {
+        refocus: bool,
+    },
+    /// Its Cancel: the run is not sent. `refocus` as for Run anyway.
+    CancelEveryRow {
+        refocus: bool,
+    },
     /// Answer the question about a row a save found changed. `at` is the
     /// row's place among the save's conflicts as the question showed it: an
     /// answer for another row than the one asked about is dropped.
@@ -2577,6 +2587,9 @@ pub enum Dialog {
     Leave(Box<LeavePrompt>),
     /// A save to production, with its statements, before anything is sent.
     ConfirmWrite(Box<WritePrompt>),
+    /// A run of the SQL editor that reaches every row of a table, before
+    /// anything is sent.
+    ConfirmEveryRow(Box<EveryRowPrompt>),
     /// A save found rows changed on the server: what to do with each.
     Conflict(Box<ConflictPrompt>),
     /// What agents may do on a connection, in the terminal look's box.
@@ -2619,6 +2632,40 @@ pub struct LeavePrompt {
     pub saving: bool,
 }
 
+/// Asks before a run of the SQL editor that writes to every row of a
+/// table: an `UPDATE` or a `DELETE` without `WHERE`, or a `TRUNCATE`.
+/// Nothing of the run is sent before the answer.
+#[derive(Debug)]
+pub struct EveryRowPrompt {
+    pub tab: ConnTabId,
+    pub id: TabId,
+    /// This question's own number, from `App::next_id`: what the view
+    /// keeps of it is kept under it, and never meets another question's.
+    pub asked: u64,
+    /// Where the run goes: Run anyway sends nothing once the tab is on
+    /// another database.
+    pub on: RanOn,
+    /// What Run anyway sends: the run as it was asked for.
+    pub statements: Vec<tabletist_db::sql::Statement>,
+    pub caps: RunCaps,
+    /// The run's statements that reach every row, in its order.
+    pub wide: Vec<WideStatement>,
+}
+
+/// A statement of a run that reaches every row of the tables it names.
+#[derive(Debug, Clone, PartialEq, Eq)]
+pub struct WideStatement {
+    /// The editor's line it starts on.
+    pub line: usize,
+    pub verb: tabletist_db::sql::Verb,
+    /// Its tables, as the statement names them: with their schema where it
+    /// gives one. Empty where they cannot be read off it.
+    pub tables: Vec<String>,
+    /// The rows they hold together, by the database's statistics: where
+    /// the sidebar has a count above 0 for each of them.
+    pub rows: Option<u64>,
+}
+
 /// Asks before a save to production.
 pub struct WritePrompt {
     pub tab: ConnTabId,
```

```diff
diff --git a/src/app.rs b/src/app.rs
--- a/src/app.rs
+++ b/src/app.rs
@@ -1818,6 +1818,8 @@ impl App {
                     self.dialog = None;
                 }
             }
+            Action::RunEveryRow { refocus } => self.answer_every_row(true, refocus),
+            Action::CancelEveryRow { refocus } => self.answer_every_row(false, refocus),
             Action::MoveSelection {
                 tab,
                 id,
@@ -6436,7 +6438,7 @@ impl App {
         }
         let mode = script_mode(workspace.run_mode(sql), dialect, &statements);
         let caps = sql.caps();
-        self.send_run(tab, id, statements, mode, caps);
+        self.send_or_ask(tab, id, statements, mode, caps);
     }
 
     /// The card's "Run in a read-write transaction": sends the statements
@@ -6472,7 +6474,8 @@ impl App {
             return;
         }
         let caps = sql.caps();
-        self.send_run(tab, id, statements, tabletist_db::ScriptMode::Write, caps);
+        let mode = tabletist_db::ScriptMode::Write;
+        self.send_or_ask(tab, id, statements, mode, caps);
     }
 
     /// The results' Show all: the statement whose rows the limit cut, sent
@@ -6530,6 +6533,157 @@ impl App {
             timeout: None,
             ..run.caps
         };
+        self.send_or_ask(tab, id, statements, mode, caps);
+    }
+
+    /// [`Self::send_run`], or first the question about a run that writes
+    /// to every row of a table: one sent to write that holds a `TRUNCATE`,
+    /// or an `UPDATE` or a `DELETE` without `WHERE` while Settings ›
+    /// Editor asks about them. A run that only reads never asks, nor one
+    /// of a tab in Read-only. While another dialog is open nothing is
+    /// asked, and nothing sent.
+    fn send_or_ask(
+        &mut self,
+        tab: ConnTabId,
+        id: TabId,
+        statements: Vec<tabletist_db::sql::Statement>,
+        mode: tabletist_db::ScriptMode,
+        caps: crate::model::RunCaps,
+    ) {
+        // Nothing else runs while the question is open.
+        if matches!(self.dialog, Some(Dialog::ConfirmEveryRow(_))) {
+            return;
+        }
+        let wide = match mode {
+            tabletist_db::ScriptMode::Write => self.wide_statements(tab, &statements),
+            tabletist_db::ScriptMode::ReadOnly => Vec::new(),
+        };
+        if wide.is_empty() {
+            self.send_run(tab, id, statements, mode, caps);
+            return;
+        }
+        let asked = self.next_id();
+        let Some(workspace) = self.workspace(tab).filter(|_| self.dialog.is_none()) else {
+            return;
+        };
+        let prompt = crate::model::EveryRowPrompt {
+            tab,
+            id,
+            asked,
+            on: crate::model::RanOn(workspace.spec.clone()),
+            statements,
+            caps,
+            wide,
+        };
+        self.dialog = Some(Dialog::ConfirmEveryRow(Box::new(prompt)));
+    }
+
+    /// The statements of `statements` the question asks about, each with
+    /// its tables and what the sidebar's statistics say of their rows.
+    fn wide_statements(
+        &self,
+        tab: ConnTabId,
+        statements: &[tabletist_db::sql::Statement],
+    ) -> Vec<crate::model::WideStatement> {
+        use tabletist_db::sql::Verb;
+        let Some(workspace) = self.workspace(tab) else {
+            return Vec::new();
+        };
+        let dialect = workspace.driver.dialect();
+        let show_system = self.settings.show_system_schemas;
+        let schemas = workspace
+            .tree
+            .visible_schemas(workspace.driver, show_system);
+        let catalog = catalog_of(workspace, &schemas);
+        let asks = |verb: Verb| match verb {
+            Verb::Update | Verb::Delete => self.settings.confirm_unsafe_writes,
+            Verb::Truncate => true,
+        };
+        statements
+            .iter()
+            .filter_map(|statement| {
+                let found = tabletist_db::sql::every_row(dialect, &statement.text)?;
+                if !asks(found.verb) {
+                    return None;
+                }
+                // A count of 0 is none: older PostgreSQL says so of a
+                // table never analysed.
+                let rows_of = |named: &tabletist_db::sql::NamedTable| {
+                    let schema = named.schema.as_deref();
+                    // PostgreSQL folds a name that is not quoted to lower
+                    // case, and finds a table by its exact name; MySQL and
+                    // SQLite find it whatever its case, as completion does.
+                    let (name, exact) = match dialect {
+                        tabletist_db::Dialect::Postgres if named.quoted => {
+                            (named.name.clone(), true)
+                        }
+                        tabletist_db::Dialect::Postgres => (named.name.to_lowercase(), true),
+                        tabletist_db::Dialect::MySql | tabletist_db::Dialect::Sqlite => {
+                            (named.name.clone(), false)
+                        }
+                    };
+                    let object = crate::completion::resolve(schema, &name, &catalog)
+                        .filter(|object| !exact || object.name == name)?;
+                    let info = workspace.tree.object_info(&object)?;
+                    info.estimated_rows.filter(|rows| *rows > 0)
+                };
+                let rows = (!found.tables.is_empty())
+                    .then(|| found.tables.iter().map(rows_of).sum::<Option<u64>>())
+                    .flatten();
+                let tables = found
+                    .tables
+                    .iter()
+                    .map(|named| match &named.schema {
+                        Some(schema) => format!("{schema}.{}", named.name),
+                        None => named.name.clone(),
+                    })
+                    .collect();
+                Some(crate::model::WideStatement {
+                    line: statement.first_line,
+                    verb: found.verb,
+                    tables,
+                    rows,
+                })
+            })
+            .collect()
+    }
+
+    /// The answer to the question about every row: with `run`, the run it
+    /// holds, sent as Run would send it now. A tab closed meanwhile sends
+    /// nothing, nor one now on another database; one whose session came
+    /// back read-only sends it read-only, where the database refuses the
+    /// write. With `refocus` the editor
+    /// gets the keyboard back, which it had when the question came.
+    fn answer_every_row(&mut self, run: bool, refocus: bool) {
+        let Some(Dialog::ConfirmEveryRow(prompt)) = self.dialog.take() else {
+            return;
+        };
+        let crate::model::EveryRowPrompt {
+            tab,
+            id,
+            on,
+            statements,
+            caps,
+            ..
+        } = *prompt;
+        let Some(sql) = self.sql_tab_mut(tab, id) else {
+            return;
+        };
+        sql.focus_editor |= refocus;
+        if !run {
+            return;
+        }
+        let Some(workspace) = self
+            .workspace(tab)
+            .filter(|workspace| workspace.spec == on.0)
+        else {
+            return;
+        };
+        let Some(sql) = workspace.sql_tab(id) else {
+            return;
+        };
+        let dialect = workspace.driver.dialect();
+        let mode = script_mode(workspace.run_mode(sql), dialect, &statements);
         self.send_run(tab, id, statements, mode, caps);
     }
 
```

```diff
diff --git a/src/completion.rs b/src/completion.rs
--- a/src/completion.rs
+++ b/src/completion.rs
@@ -221,7 +221,11 @@ fn target<'a>(site: &Site, manual: bool, catalog: &Catalog<'a>) -> Target<'a> {
 /// schema, a bare name in the bare schema and then in the other loaded
 /// ones, in their order. The name as spelled first, else whatever its
 /// case.
-fn resolve(schema: Option<&str>, name: &str, catalog: &Catalog<'_>) -> Option<ObjectRef> {
+pub(crate) fn resolve(
+    schema: Option<&str>,
+    name: &str,
+    catalog: &Catalog<'_>,
+) -> Option<ObjectRef> {
     let within = |schema: &str| {
         let objects = objects_of(catalog, schema);
         let spelled = objects.iter().find(|info| info.name == name);
```

```diff
diff --git a/src/keymap.rs b/src/keymap.rs
--- a/src/keymap.rs
+++ b/src/keymap.rs
@@ -237,6 +237,8 @@ pub enum When {
     AgentAccess,
     /// The question about leaving with pending changes.
     Leaving,
+    /// The question before a run that changes every row of a table.
+    EveryRow,
     /// The cursor's row is a new one, not saved yet: the row panel is its
     /// form.
     NewRow,
@@ -624,6 +626,8 @@ commands! {
     LeaveWrite = "leave_write",
     LeaveDiscard = "leave_discard",
     LeaveStay = "leave_stay",
+    RunEveryRow = "run_every_row",
+    CancelEveryRow = "cancel_every_row",
     ConflictOverwrite = "conflict_overwrite",
     ConflictUseServer = "conflict_use_server",
     ConflictKeepMine = "conflict_keep_mine",
@@ -992,6 +996,8 @@ impl Command {
             C::LeaveWrite => info("Write and leave", "", Prompts),
             C::LeaveDiscard => info("Discard and leave", "", Prompts),
             C::LeaveStay => info("Stay", "", Prompts),
+            C::RunEveryRow => info("Run anyway, when a run changes every row", "", Prompts),
+            C::CancelEveryRow => info("Cancel a run that changes every row", "", Prompts),
             C::ConflictOverwrite => info("Overwrite", "", Prompts),
             C::ConflictUseServer => info("Use the server's values", "", Prompts),
             C::ConflictKeepMine => info("Keep mine and reload", "", Prompts),
@@ -2068,6 +2074,22 @@ pub const BINDINGS: &[Binding] = {
         bind(C::LeaveWrite, PROMPT, NONE, keys(&[k("w")])).when(When::Leaving),
         bind(C::LeaveDiscard, PROMPT, NONE, keys(&[k("d")])).when(When::Leaving),
         bind(C::LeaveStay, PROMPT, keys(&[k("esc")]), keys(&[k("esc")])).when(When::Leaving),
+        // Either case, with Caps Lock on too.
+        bind(
+            C::RunEveryRow,
+            PROMPT,
+            control("dialog button"),
+            keys(&[k("y"), k("Y")]).shown("y"),
+        )
+        .when(When::EveryRow),
+        // `N` as the design writes it: the answer Enter gives.
+        bind(
+            C::CancelEveryRow,
+            PROMPT,
+            keys(&[k("esc")]),
+            keys(&[k("n"), k("N"), k("esc")]).shown("N"),
+        )
+        .when(When::EveryRow),
         bind(C::ConflictOverwrite, PROMPT, NONE, keys(&[k("o")])).when(When::Conflict),
         bind(C::ConflictUseServer, PROMPT, NONE, keys(&[k("s")])).when(When::Conflict),
         bind(
```

```diff
diff --git a/src/ui/workspace.rs b/src/ui/workspace.rs
--- a/src/ui/workspace.rs
+++ b/src/ui/workspace.rs
@@ -1939,6 +1939,10 @@ fn editing_status(app: &App, tab: ConnTabId) -> Editing {
     }
 }
 
+/// The height of the terminal look's status line, and of what takes its
+/// place: the `:` prompt, the question about a run that changes every row.
+pub(crate) const STATUS_LINE: f32 = 31.0;
+
 /// The field of the terminal's `:` prompt in the tab `tab`.
 fn command_id(tab: ConnTabId) -> egui::Id {
     egui::Id::new(("command-line", tab.0))
@@ -1960,7 +1964,7 @@ fn command_line(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId) {
     };
     let mut actions = Vec::new();
     egui::Panel::bottom(egui::Id::new(("status-line", tab.0)))
-        .exact_size(31.0)
+        .exact_size(STATUS_LINE)
         .resizable(false)
         .show_separator_line(false)
         .frame(Frame::new().fill(palette.panel))
@@ -2251,12 +2255,15 @@ fn status_line(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId) {
         });
     }
     egui::Panel::bottom(egui::Id::new(("status-line", tab.0)))
-        .exact_size(31.0)
+        .exact_size(STATUS_LINE)
         .resizable(false)
         .show_separator_line(false)
         .frame(Frame::new().fill(palette.panel))
         .show(ui, |ui| {
             let rect = ui.max_rect();
+            // Where a question that takes the line's place stands.
+            let line = super::every_row_prompt::status_line_id(tab);
+            ui.ctx().data_mut(|data| data.insert_temp(line, rect));
             widgets::hline(ui, rect.x_range(), rect.top() + 0.5, palette.outline);
             let y = rect.top() + 1.0 + 15.0;
             // The row is shown in the inspector; the cell is edited, where
```

```diff
diff --git a/src/ui/every_row_prompt.rs b/src/ui/every_row_prompt.rs
new file mode 100644
--- /dev/null
+++ b/src/ui/every_row_prompt.rs
@@ -0,0 +1,343 @@
+//! The question before a run of the SQL editor that changes every row of a
+//! table: an `UPDATE` or a `DELETE` without `WHERE`, or a `TRUNCATE`. On
+//! macOS and Windows a dialog with Cancel and Run anyway; in the terminal
+//! look a line laid over the status line, answered with its letters. Each
+//! says how many rows the tables hold, where the sidebar has the
+//! database's count of them.
+
+use egui::{Align2, Frame, Id, Key, Margin, Modifiers, Rect, Stroke, pos2};
+
+use crate::app::App;
+use crate::i18n::{gettext, ngettext};
+use crate::keymap::{Command, Layout, When};
+use crate::model::{Action, Dialog, EveryRowPrompt, WideStatement};
+use crate::typography::{Text, TextRole};
+use crate::ui::format;
+use crate::ui::keys::{prompt_letter, take_enter};
+use crate::ui::widgets::{self, ButtonSpec};
+use crate::ui::workspace::STATUS_LINE;
+use crate::ui::write_prompts::{Skin, button_row, fitted, keyboard_on};
+use tabletist_db::sql::Verb;
+
+/// Where the terminal look's status line of `tab` was last drawn: the
+/// question stands there.
+pub(crate) fn status_line_id(tab: crate::model::ConnTabId) -> Id {
+    Id::new(("status-line-rect", tab.0))
+}
+
+pub fn show(app: &mut App, ctx: &egui::Context) {
+    let (look, palette, locale) = (app.look, app.palette, app.locale);
+    let skin = Skin {
+        look: &look,
+        palette: &palette,
+        locale,
+    };
+    let Some(Dialog::ConfirmEveryRow(prompt)) = &app.dialog else {
+        return;
+    };
+    // Whether the editor had the keyboard when the question came: it
+    // still has it in the frame the question opens in, being drawn before
+    // it. The keyboard goes back there after the answer, and nowhere else.
+    // Kept under the question's own number, which no other question has.
+    let kept = Id::new(("every-row-refocus", prompt.asked));
+    let refocus = ctx
+        .data(|data| data.get_temp::<bool>(kept))
+        .unwrap_or_else(|| {
+            let editor = crate::ui::sql_text::editor_id(prompt.tab, prompt.id);
+            ctx.memory(|memory| memory.has_focus(editor))
+        });
+    ctx.data_mut(|data| data.insert_temp(kept, refocus));
+    let mut answer = None;
+    let editor = app
+        .workspace(prompt.tab)
+        .and_then(|workspace| workspace.sql_tab(prompt.id));
+    if editor.is_none() {
+        // Its editor closed meanwhile: nothing to send, and nothing would
+        // answer a question left on screen.
+        answer = Some(false);
+    } else {
+        // Taken before anything is drawn: a button that has the keyboard
+        // would read Enter as a press of itself. An Enter with a modifier
+        // (the run key) answers nothing.
+        let enter = ctx.input_mut(take_enter);
+        let top = if look.terminal {
+            line(ctx, prompt, skin, enter, &mut answer)
+        } else {
+            sheet(ctx, prompt, skin, enter, &mut answer)
+        };
+        if top && ctx.input_mut(|input| input.consume_key(Modifiers::NONE, Key::Escape)) {
+            answer = answer.or(Some(false));
+        }
+    }
+    if let Some(run) = answer {
+        ctx.data_mut(|data| data.remove::<bool>(kept));
+        app.actions.push(match run {
+            true => Action::RunEveryRow { refocus },
+            false => Action::CancelEveryRow { refocus },
+        });
+    }
+}
+
+/// "book_covers", "books and covers", "authors, books and covers".
+fn listed(tables: &[String], skin: Skin<'_>) -> String {
+    let names: Vec<String> = tables
+        .iter()
+        .map(|name| format::display_safe(name).into_owned())
+        .collect();
+    match names.as_slice() {
+        [] => String::new(),
+        [one] => one.clone(),
+        [rest @ .., last] => format!("{} {} {last}", rest.join(", "), skin.say("and")),
+    }
+}
+
+/// What one statement does, as a question: "Update every row in
+/// book_covers?", "Truncate books and covers?".
+fn asked(wide: &WideStatement, skin: Skin<'_>) -> String {
+    let words = match (wide.verb, wide.tables.is_empty()) {
+        (Verb::Update, false) => "Update every row in {tables}?",
+        (Verb::Update, true) => "Update every row of its table?",
+        (Verb::Delete, false) => "Delete every row in {tables}?",
+        (Verb::Delete, true) => "Delete every row of its table?",
+        (Verb::Truncate, false) => "Truncate {tables}?",
+        (Verb::Truncate, true) => "Truncate its tables?",
+    };
+    skin.say(words)
+        .replace("{tables}", &listed(&wide.tables, skin))
+}
+
+/// The question: the one statement's, or how many there are.
+fn question(prompt: &EveryRowPrompt, skin: Skin<'_>) -> String {
+    match prompt.wide.as_slice() {
+        [one] => asked(one, skin),
+        many => skin
+            .say("Run {n} statements that change every row?")
+            .replace("{n}", &many.len().to_string()),
+    }
+}
+
+/// "about 1,200 rows", where the statistics say.
+fn rows(wide: &WideStatement, skin: Skin<'_>) -> Option<String> {
+    let rows = wide.rows?;
+    let plural = u32::try_from(rows).unwrap_or(u32::MAX);
+    let words = ngettext(skin.locale, "about {n} row", "about {n} rows", plural);
+    Some(
+        skin.look
+            .label(&words)
+            .replace("{n}", &format::group_digits(rows)),
+    )
+}
+
+/// "Line 3"
+fn line_of(wide: &WideStatement, skin: Skin<'_>) -> String {
+    skin.say("Line {n}").replace("{n}", &wide.line.to_string())
+}
+
+/// What the dialog says under its question: for one statement, its line
+/// and why it reaches every row, then the rows; for several, a line each.
+fn details(prompt: &EveryRowPrompt, skin: Skin<'_>) -> Vec<String> {
+    match prompt.wide.as_slice() {
+        [one] => {
+            let why = match one.verb {
+                Verb::Update | Verb::Delete => skin.say("has no WHERE."),
+                Verb::Truncate => skin.say("takes out every row."),
+            };
+            let mut text = format!("{} {why}", line_of(one, skin));
+            if let Some(rows) = rows(one, skin) {
+                let by = skin.say("by the database's statistics.");
+                text.push_str(&format!(" {}, {by}", capitalized(&rows, skin)));
+            }
+            vec![text]
+        }
+        many => many
+            .iter()
+            .map(|wide| {
+                let what = asked(wide, skin);
+                let mut text = format!("{}: {}", line_of(wide, skin), what.trim_end_matches('?'));
+                if let Some(rows) = rows(wide, skin) {
+                    text.push_str(&format!(", {rows}"));
+                }
+                text
+            })
+            .collect(),
+    }
+}
+
+/// `text` with a capital first letter, where the look writes capitals.
+fn capitalized(text: &str, skin: Skin<'_>) -> String {
+    let mut chars = text.chars();
+    match chars.next() {
+        Some(first) if !skin.look.terminal => first.to_uppercase().chain(chars).collect(),
+        _ => text.to_owned(),
+    }
+}
+
+/// macOS and Windows: the question, what it is about, and Cancel and Run
+/// anyway. Enter answers as the button that has the keyboard, and cancels
+/// from anywhere else: nothing that writes to every row is the default.
+/// Puts the answer in `answer`: whether to run. Returns whether the dialog
+/// is the one on top.
+fn sheet(
+    ctx: &egui::Context,
+    prompt: &EveryRowPrompt,
+    skin: Skin<'_>,
+    enter: bool,
+    answer: &mut Option<bool>,
+) -> bool {
+    let Skin {
+        look,
+        palette,
+        locale,
+    } = skin;
+    let title = question(prompt, skin);
+    let lines = details(prompt, skin);
+    let modal = widgets::modal(Id::new("every-row-prompt"), look, palette).show(ctx, |ui| {
+        ui.set_width(fitted(ui.ctx(), 440.0));
+        widgets::label(ui, widgets::dialog_title(look), &title, palette.text, look);
+        ui.add_space(4.0);
+        let width = ui.available_width();
+        for line in &lines {
+            Text::one(look, widgets::body(look), line, palette.secondary)
+                .wrap(width)
+                .layout(ui.ctx())
+                .label(ui);
+        }
+        ui.add_space(14.0);
+        let (cancel, run) = (gettext(locale, "Cancel"), gettext(locale, "Run anyway"));
+        let buttons = vec![ButtonSpec::new(&cancel), ButtonSpec::new(&run).danger()];
+        if enter {
+            *answer = Some(keyboard_on(ui, &buttons) == Some(1));
+        }
+        if let Some(pressed) = button_row(ui, buttons, skin).1 {
+            *answer = Some(pressed == 1);
+        }
+    });
+    modal.is_top_modal
+}
+
+/// The terminal look: the question where the status line was, as the
+/// design's confirm prompt has it, `[y] yes  [N] no` after it. `y` runs;
+/// `n`, Enter and Esc cancel. Each hint is its button too. Puts the answer
+/// in `answer`: whether to run. Returns whether the line is the dialog on
+/// top.
+fn line(
+    ctx: &egui::Context,
+    prompt: &EveryRowPrompt,
+    skin: Skin<'_>,
+    enter: bool,
+    answer: &mut Option<bool>,
+) -> bool {
+    let Skin {
+        look,
+        palette,
+        locale,
+    } = skin;
+    // The letters, as typed text. Not while a key is held: the run key may
+    // still be down, and a repeat is no answer. Nor while a field has the
+    // keyboard: the editor keeps it for the frame the question opens in,
+    // and what is typed then went into its text.
+    let typing = ctx.text_edit_focused();
+    let (keymap, layout) = (crate::ui::keys::published(ctx), Layout::of(look));
+    let letter = |text: &str| {
+        prompt_letter(&keymap, layout, When::EveryRow, text)
+            .filter(|command| matches!(command, Command::RunEveryRow | Command::CancelEveryRow))
+    };
+    let typed = ctx.input_mut(|input| {
+        let held = input.events.iter().any(|event| {
+            matches!(
+                event,
+                egui::Event::Key {
+                    pressed: true,
+                    repeat: true,
+                    ..
+                }
+            )
+        });
+        let mut typed = None;
+        input.events.retain(|event| match event {
+            egui::Event::Text(text) if letter(text).is_some() => {
+                typed = typed.or(letter(text));
+                false
+            }
+            _ => true,
+        });
+        typed.filter(|_| !held && !typing)
+    });
+    match typed {
+        Some(command) => *answer = Some(command == Command::RunEveryRow),
+        None if enter => *answer = Some(false),
+        None => {}
+    }
+    let screen = ctx.content_rect();
+    let at = ctx
+        .data(|data| data.get_temp::<Rect>(status_line_id(prompt.tab)))
+        .unwrap_or_else(|| {
+            Rect::from_min_max(
+                pos2(screen.left(), screen.bottom() - STATUS_LINE),
+                screen.max,
+            )
+        });
+    let role = TextRole::OSecondary;
+    // "about 13 rows · line 1", "lines 1, 3, 4".
+    let mut tail = match prompt.wide.as_slice() {
+        [one] => rows(one, skin).into_iter().collect::<Vec<_>>(),
+        _ => Vec::new(),
+    };
+    let numbers: Vec<String> = prompt
+        .wide
+        .iter()
+        .map(|wide| wide.line.to_string())
+        .collect();
+    tail.push(match numbers.as_slice() {
+        [one] => skin.say("line {n}").replace("{n}", one),
+        many => skin.say("lines {n}").replace("{n}", &many.join(", ")),
+    });
+    let tail = tail.join(" · ");
+    let said = question(prompt, skin);
+    let (yes_key, no_key) = (
+        crate::ui::keys::written(ctx, look, Command::RunEveryRow).bracketed(),
+        crate::ui::keys::written(ctx, look, Command::CancelEveryRow).bracketed(),
+    );
+    let (yes, no) = (skin.say("yes"), skin.say("no"));
+    let (run, cancel) = (gettext(locale, "Run anyway"), gettext(locale, "Cancel"));
+    let modal = egui::Modal::new(Id::new("every-row-prompt"))
+        .backdrop_color(egui::Color32::TRANSPARENT)
+        .frame(Frame::new().fill(palette.panel).inner_margin(Margin::ZERO))
+        .area(
+            egui::Area::new(Id::new("every-row-prompt-line"))
+                .order(egui::Order::Foreground)
+                .anchor(Align2::LEFT_TOP, at.min.to_vec2()),
+        )
+        .show(ctx, |ui| {
+            let (rect, _) = ui.allocate_exact_size(at.size(), egui::Sense::hover());
+            // The design's confirm prompt: a red rule along its top.
+            ui.painter().hline(
+                rect.x_range(),
+                rect.top() + 0.5,
+                Stroke::new(1.0, palette.danger),
+            );
+            let y = rect.top() + 1.0 + 15.0;
+            let mut x = rect.left() + 12.0;
+            x += widgets::paint_text(ui, x, y, Text::one(look, role, &said, palette.text));
+            let dim = Text::one(look, role, &format!(" {tail}"), palette.dim);
+            x += widgets::paint_text(ui, x, y, dim);
+            widgets::announce(ui, rect, &format!("{said} {tail}"));
+            x += 18.0;
+            for (key, word, name, runs) in
+                [(&yes_key, &yes, &run, true), (&no_key, &no, &cancel, false)]
+            {
+                let hint = Text::new(look)
+                    .add(role, key, palette.accent)
+                    .space(role, " ")
+                    .add(role, word, palette.secondary);
+                let width = widgets::paint_text(ui, x, y, hint);
+                let place =
+                    Rect::from_min_max(pos2(x, rect.top() + 1.0), pos2(x + width, rect.bottom()));
+                if ButtonSpec::new(name).hidden_at(ui, place).clicked() {
+                    *answer = Some(runs);
+                }
+                x += width + 16.0;
+            }
+        });
+    modal.is_top_modal
+}
```

```diff
diff --git a/src/ui/mod.rs b/src/ui/mod.rs
--- a/src/ui/mod.rs
+++ b/src/ui/mod.rs
@@ -35,6 +35,7 @@ mod editor_results_tests;
 mod editor_settings_tests;
 #[cfg(test)]
 mod env_tests;
+pub mod every_row_prompt;
 #[cfg(test)]
 mod every_row_tests;
 pub mod filter_bar;
@@ -127,6 +128,7 @@ pub fn show(app: &mut App, ui: &mut egui::Ui) {
     about::show(app, &ui.ctx().clone());
     settings::show(app, &ui.ctx().clone());
     write_prompts::show(app, &ui.ctx().clone());
+    every_row_prompt::show(app, &ui.ctx().clone());
     conflict_prompt::show(app, &ui.ctx().clone());
     agent_access::show(app, &ui.ctx().clone());
     // Last: it says what was done, whatever is on screen by now.
```

- [ ] **Step 4: Run the tests to see them pass**

Run: `cargo test --locked --lib -- every_row_tests sql_results sql_editor history_tests editor_settings_tests`
Expected: PASS. Then the four checks above (the keymap's own tests, generated from its data, take the two new commands).

- [ ] **Step 5: Look at it by eye**

Not a test, and nothing of it is committed. Append a throwaway `#[test] #[ignore]` scene to `src/shots.rs` that connects the Bookshop fixture writable, opens a SQL editor in Read-write holding `UPDATE book_covers SET kind = 'ebook'`, runs it, and shoots `Harness::for_shots(egui::vec2(1440.0, 900.0), 1.0, true, Look::macos())` and `(1920.0, 1080.0)` for `Look::omarchy()`. Run it with `cargo test --locked --features shots --lib shots::<name> -- --ignored`, and set each beside the Components board's Dialog and confirm prompt. Remove the scene before committing.

- [ ] **Step 6: Commit**

```bash
git add src/model.rs src/app.rs src/completion.rs src/keymap.rs src/ui/workspace.rs src/ui/mod.rs src/ui/every_row_prompt.rs src/ui/every_row_tests.rs src/ui/sql_results.rs src/ui/sql_editor.rs
git commit -S -m "Ask before a SQL run that changes every row of a table"
```

---

### Task 3: The Confirm UPDATE or DELETE without WHERE row

**Files:**
- Modify: `src/settings.rs`, `src/model.rs`
- Modify (tests): `src/ui/every_row_tests.rs`, `src/ui/editor_settings_tests.rs`

- [ ] **Step 1: Write the failing tests**

```diff
diff --git a/src/ui/every_row_tests.rs b/src/ui/every_row_tests.rs
--- a/src/ui/every_row_tests.rs
+++ b/src/ui/every_row_tests.rs
@@ -11,7 +11,7 @@ use crate::testing::Harness;
 use crate::theme::Look;
 use tabletist_db::ScriptMode;
 
-use super::appearance_settings_tests::shows;
+use super::appearance_settings_tests::{settings_saves, shows};
 
 fn looks() -> [Look; 2] {
     [Look::macos(), Look::omarchy()]
@@ -337,3 +337,56 @@ fn a_postgres_name_is_looked_up_as_postgres_folds_it() {
         assert!(harness.has(said), "{text}");
     }
 }
+
+/// Settings on its Editor pane.
+fn on_the_pane(harness: &mut Harness) {
+    harness.app.apply(Action::ShowSettings);
+    harness
+        .app
+        .apply(Action::ShowSettingsPane(crate::model::SettingsPane::Editor));
+    harness.settle();
+}
+
+#[test]
+fn the_sheet_has_the_confirm_row_last_in_running_queries_and_sets_it() {
+    let mut harness = Harness::with_size(egui::vec2(1440.0, 1400.0));
+    harness.set_look(Look::macos());
+    on_the_pane(&mut harness);
+    let top = |harness: &Harness, words: &str| harness.painted_rect(words).expect(words).top();
+    let row = "Confirm UPDATE or DELETE without WHERE";
+    assert!(top(&harness, "Statement at cursor") < top(&harness, row));
+    assert!(top(&harness, row) < top(&harness, "Indent"));
+    assert!(harness.has("On every connection that allows writes"));
+    let saves = settings_saves(&harness);
+    harness.click(row);
+    harness.settle();
+    assert!(!harness.app.settings.confirm_unsafe_writes);
+    assert_eq!(settings_saves(&harness), saves + 1);
+    assert!(
+        harness
+            .app
+            .settings_file
+            .text
+            .contains("confirm_unsafe_writes = false")
+    );
+}
+
+#[test]
+fn the_screen_has_the_confirm_row_after_the_run_key_and_toggles_it() {
+    let mut harness = Harness::with_size(egui::vec2(1920.0, 1080.0));
+    harness.set_look(Look::omarchy());
+    on_the_pane(&mut harness);
+    let top = |harness: &Harness, words: &str| harness.painted_rect(words).expect(words).top();
+    let row = "confirm write without where";
+    // After the run key's row, by its hint.
+    assert!(top(&harness, "a selection always wins") < top(&harness, row));
+    assert!(top(&harness, row) < top(&harness, "line numbers"));
+    assert!(shows(&mut harness, "connections that allow writes"));
+    // Down to it from the first row: new tabs, limit, timeout, run key.
+    for _ in 0..4 {
+        type_key(&mut harness, Key::J, "j");
+    }
+    harness.press(Key::Space, Modifiers::NONE);
+    harness.settle();
+    assert!(!harness.app.settings.confirm_unsafe_writes);
+}
```

```diff
diff --git a/src/ui/editor_settings_tests.rs b/src/ui/editor_settings_tests.rs
--- a/src/ui/editor_settings_tests.rs
+++ b/src/ui/editor_settings_tests.rs
@@ -640,6 +640,7 @@ fn the_screen_sets_each_editor_option_with_its_keys() {
         "per tab",
         "caps results you browse · show all and exports get every row",
         "a selection always wins",
+        "connections that allow writes",
         "schema objects",
     ] {
         assert!(shows(&mut harness, words), "{words}");
@@ -656,6 +657,9 @@ fn the_screen_sets_each_editor_option_with_its_keys() {
     // The whole tab, with space.
     type_key(&mut harness, Key::J, "j");
     harness.press(Key::Space, Modifiers::NONE);
+    // Writes without WHERE asked about no more, with space.
+    type_key(&mut harness, Key::J, "j");
+    harness.press(Key::Space, Modifiers::NONE);
     // Line numbers from relative to absolute; four spaces; completion on
     // the key; no closing brackets and wrapped lines, with space; keywords
     // from the look's lower case to upper; leading commas, with space.
@@ -686,7 +690,8 @@ fn the_screen_sets_each_editor_option_with_its_keys() {
     assert_eq!(settings.commas, crate::settings::Commas::Leading);
     assert!(!settings.close_pairs);
     assert!(settings.wrap);
-    assert_eq!(settings_saves(&harness), saves + 11);
+    assert!(!settings.confirm_unsafe_writes);
+    assert_eq!(settings_saves(&harness), saves + 12);
     // The file has each of them, and R puts the cursor's back.
     let text = harness.app.settings_file.text.clone();
     assert!(
@@ -709,7 +714,8 @@ fn a_look_s_own_value_picked_while_none_is_set_is_not_written() {
     let mut harness = Harness::with_size(egui::vec2(1920.0, 1080.0));
     harness.set_look(Look::omarchy());
     on_the_pane(&mut harness);
-    for _ in 0..4 {
+    // Past the running queries' five rows.
+    for _ in 0..5 {
         type_key(&mut harness, Key::J, "j");
     }
     type_key(&mut harness, Key::H, "h");
```

- [ ] **Step 2: Run the tests to see them fail**

Run: `cargo test --locked --lib -- every_row_tests editor_settings_tests settings`
Expected: FAIL: the two row tests find no row, `the_screen_sets_each_editor_option_with_its_keys` steps the rows as they were and finds no "connections that allow writes", and `a_look_s_own_value_picked_while_none_is_set_is_not_written` lands on the row after line numbers.

- [ ] **Step 3: Show the row**

```diff
diff --git a/src/settings.rs b/src/settings.rs
--- a/src/settings.rs
+++ b/src/settings.rs
@@ -1076,11 +1076,15 @@ impl OptionId {
     ];
     /// The Editor pane's options whose features are not built yet: no row
     /// draws them, and their keys are read, kept and reset.
-    pub const EDITOR_LATER: [OptionId; 2] = [Self::ConfirmUnsafeWrites, Self::EditorKeys];
-    /// The Editor pane's Running queries section, as far as its options
-    /// are built.
-    pub const RUNNING: [OptionId; 4] =
-        [Self::NewTab, Self::RowLimit, Self::Timeout, Self::RunScope];
+    pub const EDITOR_LATER: [OptionId; 1] = [Self::EditorKeys];
+    /// The Editor pane's Running queries section, in both looks.
+    pub const RUNNING: [OptionId; 5] = [
+        Self::NewTab,
+        Self::RowLimit,
+        Self::Timeout,
+        Self::RunScope,
+        Self::ConfirmUnsafeWrites,
+    ];
     /// Its Typing section on macOS and Windows.
     pub const TYPING: [OptionId; 6] = [
         Self::Indent,
```

```diff
diff --git a/src/model.rs b/src/model.rs
--- a/src/model.rs
+++ b/src/model.rs
@@ -3046,6 +3046,7 @@ impl SettingsPane {
                 SettingsRow::Option(OptionId::RowLimit),
                 SettingsRow::Option(OptionId::Timeout),
                 SettingsRow::Option(OptionId::RunScope),
+                SettingsRow::Option(OptionId::ConfirmUnsafeWrites),
                 SettingsRow::Option(OptionId::LineNumbers),
                 SettingsRow::Option(OptionId::Indent),
                 SettingsRow::Option(OptionId::Autocomplete),
```

- [ ] **Step 4: Run the tests to see them pass**

Run: `cargo test --locked --lib -- every_row_tests editor_settings_tests settings ui::settings_access_tests`
Expected: PASS. Then the four checks above.

- [ ] **Step 5: Compare with the boards by eye**

Not a test, and nothing of it is committed. Shoot Settings › Editor in both looks (`Harness::for_shots`, 1440 × 1100 on macOS and 1920 × 1080 on Omarchy) and set it beside the two "Settings, Editor" boards' Running queries. Remove the scene before committing.

- [ ] **Step 6: Commit**

```bash
git add src/settings.rs src/model.rs src/ui/every_row_tests.rs src/ui/editor_settings_tests.rs
git commit -S -m "Show Settings › Editor's Confirm UPDATE or DELETE without WHERE row in both looks"
```

---

### Task 4: The website

**Files:**
- Modify: `docs/_reference/settings-and-files.md`, `docs/_guide/sql-editor.md`, `docs/_guide/omarchy.md`, `docs/_guide/macos.md`

- [ ] **Step 1: Say what the question and the row do**

```diff
diff --git a/docs/_reference/settings-and-files.md b/docs/_reference/settings-and-files.md
--- a/docs/_reference/settings-and-files.md
+++ b/docs/_reference/settings-and-files.md
@@ -52,6 +52,7 @@ open SQL editor follows each change at once.
 | **Row limit** | 100, 500, 1,000, 5,000 or 10,000 rows for each statement, or No limit. The rows past the limit are not fetched; nothing is added to your SQL. **Show all** under a cut result fetches the rest | 1,000 |
 | **Timeout** | 5, 15 or 30 seconds, 1 or 5 minutes, or Off. A run still going then is cancelled and rolled back; Cancel works either way. **Run without timeout** runs a stopped run again with none | 30 s |
 | **⌘↩ runs** (Omarchy: ctrl+enter runs) | Statement at cursor, or Whole tab: every statement in order, stopping at the first error. A selection always runs what it selects, whichever is set | Statement at cursor |
+| **Confirm UPDATE or DELETE without WHERE** (Omarchy: confirm write without where) | On or off. On, a run in Read-write asks before it sends an `UPDATE` or a `DELETE` with no `WHERE`, `LIMIT` or join condition of its own (a `WHERE` inside a subquery does not count; one inside a `WITH` is asked about too): "Update every row in book_covers?", with about how many rows the table holds by the database's statistics, where the sidebar has loaded that count (SQLite keeps none). Cancel (Omarchy: `n`, Enter or Esc) sends nothing; Run anyway (Omarchy: `y`) runs it. A `TRUNCATE` asks either way. A tab in Read-only never asks: its runs cannot write | On |
 | **Indent** | 2 spaces, 4 spaces or Tab: what Tab puts in at the cursor, or at the start of every line a selection touches, and Shift+Tab takes off them; and how far Format sets a `CASE`'s `WHEN` and `ELSE` in (four for Tab: the layout is set in spaces) | 2 spaces |
 | **Autocomplete** | As you type, On Ctrl+Space (the list opens only when you ask), or Off. Keywords are offered in the case **Keywords** sets, upper case with Keep | As you type |
 | **Close brackets and quotes** (Omarchy: close brackets) | On or off. On, `(`, `[`, `{` and a quote (and a backtick, which quotes names on MySQL and SQLite) get their closing one after the cursor where nothing follows that it would split off. A quote is not paired inside a word, right after the same quote, or inside a string, a quoted name or a comment. A closing one typed before its twin steps over it, and Backspace takes both out while the pair is empty. Typed over a selection, the two go round it, which stays selected and undoes in one step. A paste, an input method's text, or two characters arriving at once go in as they come | On |
@@ -175,7 +176,7 @@ bar = "tint"
 | `editor.run.limit` | 1 to 10000 rows for each statement, or `0` for no limit | `1000` |
 | `editor.run.timeout` | `"1s"` … `"3600s"` or `"1m"` … `"60m"`, or `"off"`. A longer one is a line that cannot be read | `"30s"` |
 | `editor.run.run` | `"statement"` or `"all"`: what Cmd/Ctrl+Return runs | `"statement"` |
-| `editor.run.confirm_unsafe_writes` | `true` or `false`. Read and kept; not acted on yet | `true` |
+| `editor.run.confirm_unsafe_writes` | `true` or `false` | `true` |
 | `editor.typing.keys` | `"standard"` or `"vim"`. Read and kept; not acted on yet | Not set: standard on macOS and Windows, vim in the Omarchy look |
 | `editor.typing.indent` | `2`, `4` or `"tab"` | `2` |
 | `editor.typing.autocomplete` | `"typing"`, `"manual"` or `"off"` | `"typing"` |
```

```diff
diff --git a/docs/_guide/sql-editor.md b/docs/_guide/sql-editor.md
--- a/docs/_guide/sql-editor.md
+++ b/docs/_guide/sql-editor.md
@@ -43,6 +43,16 @@ look) sets what a run may do.
 
 {% include shot.html file="macos-sql-committed" alt="A read-write run of two statements, with Messages saying how many rows each changed and that the run was committed" %}
 
+A run in Read-write that would change every row of a table asks first:
+an `UPDATE` or a `DELETE` with no `WHERE`, `LIMIT` or join condition of
+its own (a `WHERE` inside a subquery does not count), and a `TRUNCATE`. The
+question names the table and, where the sidebar has the database's count
+of its rows, about how many it holds: "Update every row in
+book_covers?". **Cancel** sends nothing; **Run anyway** runs it. In the
+Omarchy look the question takes the status line's place: `y` runs, `n`,
+Enter or Esc does not. **Settings › Editor** can stop the question for
+`UPDATE` and `DELETE`; a `TRUNCATE` always asks.
+
 A new tab opens in Read-only. With **New tabs run in** set to Follow
 connection in **Settings › Editor**, it opens in Read-write on a writable
 connection that is not production. On a connection that opens read-only,
```

```diff
diff --git a/docs/_guide/omarchy.md b/docs/_guide/omarchy.md
--- a/docs/_guide/omarchy.md
+++ b/docs/_guide/omarchy.md
@@ -272,6 +272,7 @@ A save can ask you something first. Each question has its own keys:
 | Tab, Shift+Tab | Indent, outdent, in the text: at the cursor, or every line of the selection |
 | `:ro`, `:rw` | Read-only and read-write runs, with the keyboard out of the text |
 | Ctrl+Space | Complete |
+| `y`; `n`, Enter or Esc | When the status line asks before a run that changes every row of a table (an `UPDATE` or a `DELETE` without `WHERE`, a `TRUNCATE`): run it; do not |
 
 {% include shot.html file="omarchy-sql" alt="A SQL editor in the Omarchy look, with its result under it and the run keys in the status line" %}
 
```

```diff
diff --git a/docs/_guide/macos.md b/docs/_guide/macos.md
--- a/docs/_guide/macos.md
+++ b/docs/_guide/macos.md
@@ -226,6 +226,7 @@ column's line, or come to them with Tab or F6.
 | Tab, Shift+Tab | Indent, outdent: at the cursor, or every line of the selection |
 | Cmd+Shift+M | Switch between read-only and read-write runs |
 | Cmd+I, Ctrl+Space | Complete |
+| Esc, Return | Cancel, when a run that changes every row of a table asks first. Return runs only from Run anyway, with the keyboard on it |
 
 While the completion list is open, Up and Down move in it, Tab accepts
 and Esc closes it. Return accepts too, except when it stays a line break:
```

- [ ] **Step 2: Build and check the site**

Run: `cd docs && bin/check` where Ruby and the bundle are installed; otherwise leave it to `docs.yml` on the pull request.
Expected: the build and its checks pass.

- [ ] **Step 3: Commit**

```bash
git add docs/_reference/settings-and-files.md docs/_guide/sql-editor.md docs/_guide/omarchy.md docs/_guide/macos.md
git commit -S -m "Describe the question before a run that changes every row on the website"
```

---

### Task 5: The pull request

- [ ] Check the branch has no pull request merged under it, push it, and open one pull request: a summary, the row built (Keybindings is what is left hidden), the departures above, and what each look does by default (on in both; `TRUNCATE` asks either way). The keys added: `y` and `n` (shown `N`, either case) in the Omarchy look, Esc in both. No screenshot.

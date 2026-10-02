# SQL Editor, Slice 2: Autocomplete Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A completion list at the SQL editor's cursor that offers keywords, then schemas, tables and views, then the columns of the tables a statement names, accepted with Tab or Enter and undone with one `Mod+Z`.

**Architecture:** `tabletist-db` gains a `complete` module that reads a script's tokens and says where a completion would go and what belongs there (no parser, no catalog), plus `Dialect::ident` for bare-or-quoted names. The app turns that site and what the workspace knows into candidates (`src/completion.rs`), keeps the open list on `SqlTab`, and works it out in `App::refresh_completion`, called from `frame_ui`. `src/ui/keys.rs` takes the list's keys before the editor and the global shortcuts; `src/ui/sql_text.rs` carries out an accepted insertion and holds Esc; `src/ui/sql_complete.rs` draws the list. Columns come through the existing `Command::Describe`, cached on the workspace.

**Tech Stack:** Rust 2024, egui 0.36 (crmne fork, rev `0b43114`), AccessKit 0.24 through egui. No new dependency.

**Spec:** `docs/superpowers/specs/2026-10-01-sql-autocomplete-design.md`

## Deviations from the spec

Decided while planning; the spec's intent holds.

- The view's action is `SqlTyped { tab, sql_tab }`, sent only when the field's text changed by typing. The spec's `SqlEdited { typed: false }` had nothing to do: an open list is recomputed whenever the script or the cursor changes.
- `Completion` also keeps `typed` (the part of the word before the cursor), which the highlight rule compares.
- Keywords and phrases match from their start only. Matching inside them (`in` finding `JOIN` and `UNION`) is noise; names still match anywhere.
- The matched part of a name is drawn at weight 500 on macOS, not 600: Plex Mono is bundled at 400 and 500 only.
- A qualifier counts only when its dots touch the names (`b.ti`, not `b . ti`).
- A subquery in `FROM` is not skipped as a whole: the scan goes on inside it, so its tables are sources. A table that follows a subquery in a comma list (`FROM (SELECT ...) s, books b`) is not found; one after `JOIN` is.
- The bare schema's objects are asked for at sites that use them (tables, columns, qualifiers), not at the first word of a statement.
- The commits are ten, not seven: keys with insertion, and the view, are separate commits, so are the keyword candidates and the list's model, and the docs are a commit of their own.

## Changes during execution

Found in review while the tasks were built. The task texts below are as planned, except where a line here says the text was brought in line.

- Task 1: the test draws a button above the editor, so the arrow assertions test the filter; the hold block sits after the focus request.
- Task 2: `FROM` and `UPDATE` are table words only where they start a table list. A `FROM` counts once the statement has had `SELECT`, `DELETE`, `UPDATE` or `SHOW`, not after `IS [NOT] DISTINCT`, and not as the first `FROM` inside `EXTRACT(`, `TRIM(`, `SUBSTRING(` or `OVERLAY(`. An `UPDATE` does not count after `FOR`, `KEY` or `DO`. A function in a `FROM` list is skipped with its arguments and alias, and the list goes on. `TABLE_WORDS` became `SOURCE_WORDS` and `TABLE`. The module has 28 tests, not 9.
- Task 3: `Candidate::is_typed` says whether a row only repeats what is typed: a keyword whatever its case, any other name as spelled. `rank` uses it, and so do Task 4's Enter rule and opening rule (their text below was brought in line).

## Global Constraints

- Checks before every commit (AGENTS.md): `cargo fmt --all --check`, `cargo clippy --locked --workspace --all-targets -- -D warnings`, `cargo test --locked --workspace --all-targets`, `RUSTDOCFLAGS='-D warnings' cargo doc --locked --workspace --no-deps`. Use `~/.cargo/bin/cargo` (the mise shim fails). When a task touches `src/shots.rs` (Task 6), also run `~/.cargo/bin/cargo check --locked --features shots --tests`: nothing else compiles it.
- The code in this plan is not formatted: run `~/.cargo/bin/cargo fmt --all` before each commit's checks.
- The code in this plan was written against the sources without compiling it. Where a signature differs from what the plan shows, follow the code that exists and keep the behaviour the step's tests name.
- Test modules may lack imports the new tests use: add the `use` lines. Where this plan shows a `use` line in the middle of a file that is appended to, put it with the file's other imports.
- No driver changes, so no database servers are needed. The integration tests print "skipped" without their variables and pass; say so when reporting.
- Commits are signed. If the SSH agent refuses, commit with `git -c commit.gpgsign=false commit` and note it; they are re-signed before push. Never set `SSH_AUTH_SOCK` in git commands.
- End every commit message with `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`. Subject lines follow the repo: an imperative sentence, no prefix.
- Views push `Action`s; `App::apply` reduces them. A view may edit the text a field is editing and the cursor it reports, and may take a one-shot request off the tab (as `edit` takes `focus_editor`), nothing else.
- Views draw text only through `TextRole`s; never name a font, family or size.
- User-facing strings go through `gettext`. Never use em dashes anywhere (code, strings, docs, commits).
- No design or pixel conformance checks in any test. Compare with the "SQL editor" artboards by hand; screenshots stay local and use the Bookshop demo data. Tests use the fixture's names (`users`, `orders`, `active_users`).
- The SQL text is never logged. A `Site` and a `Completion` hold names read from the script: never log or `Debug`-print them from the app.
- Add a focused regression test for every behaviour. Do not weaken lints or add `allow`s without saying why.

## Review Focus

1. **Enter stays a line break** where a user means one: after a one-letter word, after a table name and after `AS` (Task 4 `no_list_after_a_table_name_or_as`), on an exact match also when another row was highlighted a letter earlier (Task 9 `an_exact_match_takes_the_highlight_back`), and in a list that is still empty (Task 8 `enter_is_a_line_break_while_a_list_waits`).
2. **Esc**: the first closes the list and the editor keeps the keyboard, the second leaves the editor, and arrows and Tab stay the editor's while Esc is held (Task 1, Task 5).
3. **Undo**: one `Mod+Z` after an insertion restores exactly the typed word (Task 5 `undo_restores_the_typed_word`).
4. **Nothing per frame**: a frame that changes nothing lists nothing and tokenizes nothing (Task 4 `a_quiet_frame_computes_nothing`).
5. **Fetches**: one `ListObjects` per schema and one `Describe` per table, never repeated, never retried after a failure, at most eight per statement, and only on a connected session (Tasks 8 and 9).
6. **Shortcuts keep their meaning**: `Ctrl+N` and `Ctrl+P` move the highlight only in the terminal look with the list open (Task 5).

---

## File Structure

```
crates/tabletist-db/src/complete.rs  site(): word, qualifier, expects, sources, CTEs;
                                     PHRASES (new)
crates/tabletist-db/src/reserved.rs  reserved words per dialect (new)
crates/tabletist-db/src/sql.rs       keywords(dialect) made public
crates/tabletist-db/src/dialect.rs   Dialect::ident
crates/tabletist-db/src/lib.rs       mod complete, mod reserved
src/completion.rs                    Candidate, Kind, Catalog, list(), needs() (new)
src/lib.rs                           mod completion
src/model.rs                         Completion, Wanted, SqlTab.completion(_wanted),
                                     the completion actions, Workspace.columns,
                                     catalog_generation, bare_schema
src/app.rs                           the completion reducers, refresh_completion,
                                     send_needs, load_columns, Structure routing,
                                     invalidation, frame_ui
src/ui/sql_text.rs                   hold_escape, Parsed.tokens, SqlTyped,
                                     insert_completion, the list's place and focus
src/ui/sql_complete.rs               the list's view (new)
src/ui/complete_tests.rs             headless tests of the list (new, tests only)
src/ui/keys.rs                       the list's keys, Ctrl+Space, the help entry
src/ui/workspace.rs                  the mode line's `tab complete`
src/ui/mod.rs                        mod sql_complete, mod complete_tests
src/typography.rs                    CompletionName, CompletionMatch
src/typography/fonts.rs              the Tab glyph in the glyph test
src/shots.rs                         a `sql-complete` scene
README.md                            the feature line
```

---

### Task 1: The editor keeps the keyboard on an Esc it is told to hold

egui drops a widget's focus on Escape at the start of the pass (`Focus::begin_pass`), before `keys::handle` runs, unless the focused widget's lock filter holds Escape. A multiline `TextEdit` sets its filter while it draws (arrows, and Tab through `lock_focus(true)`), and `Memory::set_focus_lock_filter` replaces the whole filter. This task proves that setting the filter again after the field is drawn keeps the keyboard, before anything is built on it. If the test cannot be made to pass, stop and report: the fallback (asking for the focus back on the frame after Esc) changes Task 5.

**Files:**
- Modify: `src/ui/sql_text.rs` (`Field`, `edit`, `show`, tests)

**Interfaces:**
- Produces: `Field.hold_escape: bool`. `show` passes `false` until Task 5.

- [ ] **Step 1: Write the failing test** (append to the tests of `src/ui/sql_text.rs`)

```rust
    /// One frame of an editor on its own, with `events`. Returns whether
    /// it has the keyboard afterwards.
    fn editor_frame(
        harness: &mut crate::testing::Harness,
        sql: &mut SqlTab,
        hold_escape: bool,
        events: Vec<egui::Event>,
    ) -> bool {
        let (look, palette) = (Look::standard(), Palette::light());
        let input = egui::RawInput {
            screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, harness.size)),
            events,
            ..Default::default()
        };
        let mut focused = false;
        let mut output = harness.ctx.run_ui(input, |ui| {
            egui::CentralPanel::default().show(ui, |ui| {
                let field = Field {
                    id: Id::new("held editor"),
                    name: "SQL",
                    left: 14,
                    dialect: Dialect::Sqlite,
                    look: &look,
                    palette: &palette,
                    hold_escape,
                };
                focused = edit(ui, sql, &field).focused;
            });
        });
        // As `Harness::frame_with` does: a delta dropped unapplied panics.
        output.textures_delta.clear();
        focused
    }

    #[test]
    fn an_editor_told_to_hold_escape_keeps_the_keyboard() {
        use crate::testing::{key, release};
        use egui::{Key, Modifiers};
        let mut harness = crate::testing::Harness::new();
        let mut sql = SqlTab::new(TabId(1), 1, 1_000, None);
        sql.text = "SELECT 1\nFROM users".into();
        // A new editor asks for the keyboard; it has it a frame later.
        editor_frame(&mut harness, &mut sql, true, Vec::new());
        editor_frame(&mut harness, &mut sql, true, Vec::new());
        assert!(editor_frame(&mut harness, &mut sql, true, Vec::new()));
        // Esc is held: the editor still has the keyboard.
        let esc = || vec![key(Key::Escape, Modifiers::NONE)];
        assert!(editor_frame(&mut harness, &mut sql, true, esc()));
        let up = vec![release(Key::Escape, Modifiers::NONE)];
        assert!(editor_frame(&mut harness, &mut sql, true, up));
        // The arrows are still the editor's: the cursor moves up a line
        // and the keyboard stays.
        let before = sql.cursor;
        let arrow = vec![key(Key::ArrowUp, Modifiers::NONE)];
        assert!(editor_frame(&mut harness, &mut sql, true, arrow));
        assert!(editor_frame(&mut harness, &mut sql, true, Vec::new()));
        assert!(sql.cursor < before, "{} then {}", before, sql.cursor);
        // So is Tab: it indents.
        let tab = vec![key(Key::Tab, Modifiers::NONE)];
        assert!(editor_frame(&mut harness, &mut sql, true, tab));
        assert!(sql.text.contains('\t'));
        // Not told to hold it: Esc leaves the editor, as before.
        assert!(editor_frame(&mut harness, &mut sql, false, Vec::new()));
        assert!(!editor_frame(&mut harness, &mut sql, false, esc()));
    }
```

- [ ] **Step 2: Run to see it fail**

Run: `~/.cargo/bin/cargo test --locked --lib ui::sql_text::tests::an_editor_told_to_hold_escape`
Expected: FAIL to compile (`Field` has no field `hold_escape`).

- [ ] **Step 3: Implement**

In `src/ui/sql_text.rs`, add the field to `Field`:

```rust
    /// Keep the keyboard on Esc (a completion list is open and takes it).
    hold_escape: bool,
```

In `show`, where `Field` is built, add `hold_escape: false,`.

In `edit`, after the `accesskit_node_builder` call that names the field:

```rust
    if field.hold_escape && output.response.has_focus() {
        // The field set its own filter while it drew (the arrows and Tab).
        // egui replaces the whole filter, so those are named again.
        ui.memory_mut(|memory| {
            memory.set_focus_lock_filter(
                field.id,
                egui::EventFilter {
                    tab: true,
                    horizontal_arrows: true,
                    vertical_arrows: true,
                    escape: true,
                },
            );
        });
    }
```

- [ ] **Step 4: Run to see it pass**

Run: `~/.cargo/bin/cargo test --locked --lib ui::sql_text`
Expected: PASS, all of the module's tests.

- [ ] **Step 5: Run the checks and commit**

```bash
~/.cargo/bin/cargo fmt --all --check && ~/.cargo/bin/cargo clippy --locked --workspace --all-targets -- -D warnings && ~/.cargo/bin/cargo test --locked --workspace --all-targets
git add src/ui/sql_text.rs
git commit -m "Let the SQL editor hold Esc when it is told to

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 2: Where a completion goes: `complete::site`

**Files:**
- Create: `crates/tabletist-db/src/complete.rs`
- Modify: `crates/tabletist-db/src/lib.rs` (add `pub mod complete;` beside `pub mod sql;`)

**Interfaces:**
- Produces: `complete::site(tokens: &[Token], text: &str, cursor: usize) -> Option<Site>`, `Site { word, qualifier, expects, sources, ctes }`, `Expects { Start, Tables, Name, Columns }`, `Source { schema, name, alias }`.

- [ ] **Step 1: Write the failing tests** (the whole test module of the new file; create the file with only this module and the `use` lines first)

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::Dialect;
    use crate::sql::tokenize;

    /// The site at the `|` of `marked`.
    fn at(dialect: Dialect, marked: &str) -> Option<Site> {
        let cursor = marked.find('|').expect("a cursor mark");
        let text = marked.replacen('|', "", 1);
        site(&tokenize(dialect, &text), &text, cursor)
    }

    fn pg(marked: &str) -> Site {
        at(Dialect::Postgres, marked).expect("a site")
    }

    fn source(schema: Option<&str>, name: &str, alias: Option<&str>) -> Source {
        Source {
            schema: schema.map(str::to_owned),
            name: name.to_owned(),
            alias: alias.map(str::to_owned),
        }
    }

    #[test]
    fn the_word_is_the_one_the_cursor_is_in_or_ends() {
        assert_eq!(pg("SELECT na|").word, 7..9);
        // Inside a word: the whole word is replaced.
        assert_eq!(pg("SELECT na|me FROM users").word, 7..11);
        // After a space, and at the start of a word: empty.
        assert_eq!(pg("SELECT |").word, 7..7);
        assert_eq!(pg("SELECT * FROM |users").word, 14..14);
        // A keyword being typed is a word too.
        assert_eq!(pg("SELECT * FRO|").word, 9..12);
    }

    #[test]
    fn nothing_inside_strings_comments_numbers_and_quoted_names() {
        for dialect in [Dialect::Postgres, Dialect::MySql, Dialect::Sqlite] {
            for marked in [
                "SELECT 'us|ers'",
                "SELECT 'unterminated us|",
                "SELECT 1 -- from us|",
                "SELECT /* us| */ 1",
                "SELECT 12|",
            ] {
                assert_eq!(at(dialect, marked), None, "{dialect:?}: {marked}");
            }
        }
        // Each dialect's own quotes (PostgreSQL has no backtick names).
        assert_eq!(at(Dialect::MySql, "SELECT `us|ers`"), None);
        assert_eq!(at(Dialect::Sqlite, "SELECT `us|ers`"), None);
        assert_eq!(at(Dialect::Postgres, "SELECT \"Us|ers\""), None);
        assert_eq!(at(Dialect::Sqlite, "SELECT [Us|ers]"), None);
        assert_eq!(at(Dialect::Postgres, "SELECT $$ us| $$"), None);
        assert_eq!(at(Dialect::MySql, "SELECT 1 # us|"), None);
    }

    #[test]
    fn the_statement_is_the_one_between_the_semicolons_around_the_cursor() {
        // Trailing spaces still belong to the statement.
        let site = pg("SELECT 1; SELECT * FROM users u WHERE   |");
        assert_eq!(site.expects, Expects::Columns);
        assert_eq!(site.sources, [source(None, "users", Some("u"))]);
        // After a `;`: a new statement.
        let fresh = pg("SELECT * FROM users; |");
        assert_eq!(fresh.expects, Expects::Start);
        assert!(fresh.sources.is_empty());
        // The statement before the cursor's does not leak in, nor the one
        // after it.
        let middle = pg("SELECT * FROM a; SELECT x| FROM b; SELECT * FROM c");
        assert_eq!(middle.sources, [source(None, "b", None)]);
        // An empty script.
        let empty = pg("|");
        assert_eq!((empty.expects, empty.word), (Expects::Start, 0..0));
        // A `;` in a string is not a statement's end.
        assert_eq!(pg("SELECT ';' FROM users WHERE |").sources.len(), 1);
    }

    #[test]
    fn what_is_expected() {
        let expects = |marked| pg(marked).expects;
        assert_eq!(expects("SEL|"), Expects::Start);
        assert_eq!(expects("-- a comment\n  sel|"), Expects::Start);
        for marked in [
            "SELECT * FROM us|",
            "SELECT * FROM users JOIN or|",
            "UPDATE us|",
            "INSERT INTO us|",
            "TABLE us|",
            "SELECT * FROM users, or|",
            "SELECT * FROM users u, orders AS o, ac|",
        ] {
            assert_eq!(expects(marked), Expects::Tables, "{marked}");
        }
        for marked in [
            // Most likely an alias, or a new name.
            "SELECT * FROM users u|",
            "SELECT * FROM main.users wh|",
            "SELECT * FROM users JOIN orders o|",
            "SELECT count(*) AS to|",
            "SELECT * FROM users AS u|",
        ] {
            assert_eq!(expects(marked), Expects::Name, "{marked}");
        }
        for marked in [
            "SELECT na|",
            "SELECT id, na|",
            "SELECT * FROM users WHERE na|",
            "SELECT * FROM users u WHERE na|",
            "SELECT * FROM users JOIN orders ON us|",
            "SELECT * FROM users u JOIN orders o ON o.id = 1, na|",
            "SELECT lower(a, na|",
            "SELECT * FROM users u ORDER BY na|",
            // The word after an alias is past the name.
            "SELECT * FROM users u wh|",
        ] {
            assert_eq!(expects(marked), Expects::Columns, "{marked}");
        }
        assert_eq!(
            at(Dialect::MySql, "SELECT * FROM a STRAIGHT_JOIN us|").map(|site| site.expects),
            Some(Expects::Tables)
        );
    }

    #[test]
    fn qualifiers_are_the_names_before_the_dots() {
        assert_eq!(pg("SELECT u.na|").qualifier, ["u"]);
        assert_eq!(pg("SELECT u.|").qualifier, ["u"]);
        assert_eq!(pg("SELECT u.|").word, 9..9);
        assert_eq!(pg("SELECT * FROM public.us|").qualifier, ["public"]);
        assert_eq!(pg("SELECT public.users.na|").qualifier, ["public", "users"]);
        assert_eq!(pg("SELECT \"My Schema\".\"Us\"\"ers\".na|").qualifier, ["My Schema", "Us\"ers"]);
        let mysql = at(Dialect::MySql, "SELECT `shop`.`users`.na|").unwrap();
        assert_eq!(mysql.qualifier, ["shop", "users"]);
        let sqlite = at(Dialect::Sqlite, "SELECT [main].na|").unwrap();
        assert_eq!(sqlite.qualifier, ["main"]);
        // The dots must touch the names.
        assert!(pg("SELECT u. na|").qualifier.is_empty());
        // A dot with no name before it offers nothing.
        assert_eq!(at(Dialect::Postgres, "SELECT (1).|"), None);
    }

    #[test]
    fn sources_are_the_tables_a_statement_names() {
        assert_eq!(
            pg("SELECT | FROM users u JOIN main.orders AS o ON o.user_id = u.id").sources,
            [
                source(None, "users", Some("u")),
                source(Some("main"), "orders", Some("o")),
            ]
        );
        assert_eq!(
            pg("SELECT | FROM users, orders o, active_users").sources,
            [
                source(None, "users", None),
                source(None, "orders", Some("o")),
                source(None, "active_users", None),
            ]
        );
        assert_eq!(pg("UPDATE users SET na|").sources, [source(None, "users", None)]);
        assert_eq!(pg("INSERT INTO users (na|").sources, [source(None, "users", None)]);
        assert_eq!(pg("DELETE FROM users WHERE na|").sources, [source(None, "users", None)]);
        assert_eq!(
            pg("SELECT | FROM \"Order Items\" oi").sources,
            [source(None, "Order Items", Some("oi"))]
        );
        // A keyword after a table is not its alias.
        assert_eq!(
            pg("SELECT | FROM users WHERE id = 1").sources,
            [source(None, "users", None)]
        );
        // PostgreSQL's ONLY and LATERAL come before the name.
        assert_eq!(pg("SELECT | FROM ONLY users u").sources, [source(None, "users", Some("u"))]);
    }

    #[test]
    fn a_subquerys_tables_are_sources_too() {
        // Approximate by design: every depth counts.
        assert_eq!(
            pg("SELECT | FROM (SELECT * FROM orders o) s JOIN users u ON true").sources,
            [source(None, "orders", Some("o")), source(None, "users", Some("u"))]
        );
        assert_eq!(
            pg("SELECT * FROM users WHERE id IN (SELECT user_id FROM orders WHERE |)").sources,
            [source(None, "users", None), source(None, "orders", None)]
        );
    }

    #[test]
    fn cte_names_are_listed() {
        let site = pg("WITH recent AS (SELECT * FROM orders), big (id) AS NOT MATERIALIZED (SELECT 1) SELECT * FROM re|");
        assert_eq!(site.ctes, ["recent", "big"]);
        assert_eq!(site.expects, Expects::Tables);
        assert_eq!(pg("WITH RECURSIVE tree AS (SELECT 1) SELECT |").ctes, ["tree"]);
        // Unfinished: the name before `AS (` already counts.
        assert_eq!(pg("WITH recent AS (SELECT * FROM us|").ctes, ["recent"]);
        assert!(pg("SELECT * FROM us|").ctes.is_empty());
    }

    #[test]
    fn a_long_run_of_words_does_not_recurse() {
        let words = "a ".repeat(200_000);
        let text = format!("SELECT * FROM {words}");
        let tokens = tokenize(Dialect::Postgres, &text);
        assert!(site(&tokens, &text, text.len()).is_some());
    }
}
```

- [ ] **Step 2: Run to see them fail**

Add `pub mod complete;` to `crates/tabletist-db/src/lib.rs`.
Run: `~/.cargo/bin/cargo test --locked -p tabletist-db --lib complete::`
Expected: FAIL to compile (`site`, `Site`, `Expects`, `Source` missing).

- [ ] **Step 3: Implement**

Put above the tests in `crates/tabletist-db/src/complete.rs`:

```rust
//! Where a completion would go in a SQL script and what belongs there,
//! read off the script's tokens. There is no parser: the scan is
//! approximate, and it never fails.

use std::ops::Range;

use crate::sql::{Token, TokenKind};

/// Where a completion would go, and what belongs there.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Site {
    /// The word at the cursor: the bytes a completion replaces. Empty at
    /// a fresh position (after a dot or a space, at the start of a word).
    pub word: Range<usize>,
    /// The names before the word, joined to it by dots, quotes removed:
    /// `["public"]` in `public.bo`, `["b"]` in `b.ti`.
    pub qualifier: Vec<String>,
    /// What belongs at the word when it has no qualifier.
    pub expects: Expects,
    /// The tables the statement names, in order.
    pub sources: Vec<Source>,
    /// The names of the statement's common table expressions.
    pub ctes: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Expects {
    /// The first word of a statement: keywords only.
    Start,
    /// After FROM, JOIN, UPDATE, INTO or TABLE, or a comma in a FROM
    /// list: schemas, tables, views and CTE names.
    Tables,
    /// After a table name in FROM or JOIN, or after AS: most likely an
    /// alias or a new name.
    Name,
    /// Anywhere else: columns of the sources, then keywords.
    Columns,
}

/// A table a statement names.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Source {
    pub schema: Option<String>,
    pub name: String,
    pub alias: Option<String>,
}

/// The keywords a table's name follows.
const TABLE_WORDS: [&str; 6] = ["FROM", "JOIN", "STRAIGHT_JOIN", "UPDATE", "INTO", "TABLE"];

/// A token the server reads: not white space, not a comment.
struct Piece<'a> {
    kind: TokenKind,
    text: &'a str,
    range: Range<usize>,
}

impl Piece<'_> {
    fn is(&self, text: &str) -> bool {
        self.text == text
    }

    fn is_keyword(&self, words: &[&str]) -> bool {
        self.kind == TokenKind::Keyword
            && words.iter().any(|word| word.eq_ignore_ascii_case(self.text))
    }

    /// A name a user gave: not a keyword.
    fn is_name(&self) -> bool {
        matches!(
            self.kind,
            TokenKind::Identifier | TokenKind::QuotedIdentifier
        )
    }

    /// The name it spells, without its quotes.
    fn name(&self) -> String {
        if self.kind != TokenKind::QuotedIdentifier {
            return self.text.to_owned();
        }
        let mut chars = self.text.chars();
        let Some(open) = chars.next() else {
            return String::new();
        };
        let close = if open == '[' { ']' } else { open };
        // An unterminated name has no closing quote to strip.
        let inner = chars.as_str();
        let inner = inner.strip_suffix(close).unwrap_or(inner);
        if open == '[' {
            inner.to_owned()
        } else {
            inner.replace(&format!("{open}{open}"), &open.to_string())
        }
    }
}

/// The site at byte `cursor` of `text`, whose tokens are `tokens`. `None`
/// where nothing is completed: in a string, a comment, a number or a quoted
/// name, and after a dot that follows no name.
pub fn site(tokens: &[Token], text: &str, cursor: usize) -> Option<Site> {
    // The statement: from the last `;` before the cursor to the first one
    // at or after it.
    let first = tokens
        .iter()
        .rposition(|token| token.kind == TokenKind::Semicolon && token.range.end <= cursor)
        .map_or(0, |index| index + 1);
    let last = tokens[first..]
        .iter()
        .position(|token| token.kind == TokenKind::Semicolon)
        .map_or(tokens.len(), |index| first + index);
    let statement = &tokens[first..last];
    // The token the cursor is inside of or at the end of.
    let held = statement
        .iter()
        .find(|token| token.range.start < cursor && cursor <= token.range.end);
    let word = match held {
        Some(token) => match token.kind {
            TokenKind::Identifier | TokenKind::Keyword => token.range.clone(),
            TokenKind::String
            | TokenKind::Comment
            | TokenKind::ExecutableComment
            | TokenKind::Number
            | TokenKind::QuotedIdentifier => return None,
            TokenKind::Operator
            | TokenKind::Punctuation
            | TokenKind::Semicolon
            | TokenKind::Whitespace => cursor..cursor,
        },
        None => cursor..cursor,
    };
    let pieces: Vec<Piece<'_>> = statement
        .iter()
        .filter(|token| !matches!(token.kind, TokenKind::Whitespace | TokenKind::Comment))
        .map(|token| Piece {
            kind: token.kind,
            text: &text[token.range.clone()],
            range: token.range.clone(),
        })
        .collect();
    let before = &pieces[..pieces.partition_point(|piece| piece.range.start < word.start)];
    let qualifier = qualifier(before, word.start)?;
    let expects = if qualifier.is_empty() {
        expects(before)
    } else {
        Expects::Columns
    };
    Some(Site {
        word,
        qualifier,
        expects,
        sources: sources(&pieces),
        ctes: ctes(&pieces),
    })
}

/// The names joined by dots to a word starting at `start`, in order.
/// `None` when a dot touching the word follows no name.
fn qualifier(before: &[Piece<'_>], start: usize) -> Option<Vec<String>> {
    let mut names = Vec::new();
    let (mut at, mut edge) = (before.len(), start);
    while at >= 1 && before[at - 1].is(".") && before[at - 1].range.end == edge {
        let dot = &before[at - 1];
        let name = at
            .checked_sub(2)
            .map(|index| &before[index])
            .filter(|piece| {
                (piece.is_name() || piece.kind == TokenKind::Keyword)
                    && piece.range.end == dot.range.start
            })?;
        names.push(name.name());
        edge = name.range.start;
        at -= 2;
    }
    names.reverse();
    Some(names)
}

/// What belongs after the pieces `before` a word.
fn expects(before: &[Piece<'_>]) -> Expects {
    let Some(previous) = before.last() else {
        return Expects::Start;
    };
    if table_position(before) {
        return Expects::Tables;
    }
    if previous.is_keyword(&["AS"]) {
        return Expects::Name;
    }
    if previous.is_name() {
        // The name may be the end of `schema.table`: look before the chain.
        let mut start = before.len() - 1;
        while start >= 2 && before[start - 1].is(".") && before[start - 2].is_name() {
            start -= 2;
        }
        if table_position(&before[..start]) {
            return Expects::Name;
        }
    }
    Expects::Columns
}

/// Whether a table's name belongs after the pieces `before`: they end in
/// one of the table words, or in a comma of a FROM list.
fn table_position(before: &[Piece<'_>]) -> bool {
    let Some((previous, rest)) = before.split_last() else {
        return false;
    };
    previous.is_keyword(&TABLE_WORDS) || (previous.is(",") && in_from_list(rest))
}

/// Whether a comma after the pieces `before` separates the tables of a
/// FROM list: the nearest keyword at its depth, `AS` aside, is FROM.
fn in_from_list(before: &[Piece<'_>]) -> bool {
    let mut depth = 0_usize;
    for piece in before.iter().rev() {
        if piece.is(")") {
            depth += 1;
        } else if piece.is("(") {
            // The comma is inside these parentheses: arguments, a list.
            let Some(inside) = depth.checked_sub(1) else {
                return false;
            };
            depth = inside;
        } else if depth == 0 && piece.kind == TokenKind::Keyword && !piece.is_keyword(&["AS"]) {
            return piece.is_keyword(&["FROM"]);
        }
    }
    false
}

/// The tables a statement names, at every depth.
fn sources(pieces: &[Piece<'_>]) -> Vec<Source> {
    let mut found = Vec::new();
    let mut at = 0;
    while at < pieces.len() {
        let piece = &pieces[at];
        at += 1;
        if !piece.is_keyword(&TABLE_WORDS[..5]) {
            continue;
        }
        // FROM takes a list; the others one table.
        let list = piece.is_keyword(&["FROM"]);
        loop {
            let (source, next) = table(pieces, at);
            found.extend(source);
            at = next;
            if !(list && pieces.get(at).is_some_and(|piece| piece.is(","))) {
                break;
            }
            at += 1;
        }
    }
    found
}

/// The table reference at `at`: the source it names, if it names one, and
/// the index after it. A parenthesis (a subquery) is left where it is, so
/// the scan goes on inside it.
fn table(pieces: &[Piece<'_>], mut at: usize) -> (Option<Source>, usize) {
    while pieces
        .get(at)
        .is_some_and(|piece| piece.is_keyword(&["ONLY", "LATERAL"]))
    {
        at += 1;
    }
    let Some(first) = pieces.get(at).filter(|piece| piece.is_name()) else {
        return (None, at);
    };
    at += 1;
    let (mut schema, mut name) = (None, first.name());
    if pieces.get(at).is_some_and(|piece| piece.is("."))
        && let Some(second) = pieces.get(at + 1).filter(|piece| piece.is_name())
    {
        schema = Some(name);
        name = second.name();
        at += 2;
    }
    if pieces.get(at).is_some_and(|piece| piece.is_keyword(&["AS"])) {
        at += 1;
    }
    let alias = pieces.get(at).filter(|piece| piece.is_name()).map(Piece::name);
    if alias.is_some() {
        at += 1;
    }
    (
        Some(Source {
            schema,
            name,
            alias,
        }),
        at,
    )
}

/// The index after the parenthesis that closes the one at `at`, or the end.
fn after_parens(pieces: &[Piece<'_>], at: usize) -> usize {
    let mut depth = 0_usize;
    for (index, piece) in pieces.iter().enumerate().skip(at) {
        if piece.is("(") {
            depth += 1;
        } else if piece.is(")") {
            depth = depth.saturating_sub(1);
            if depth == 0 {
                return index + 1;
            }
        }
    }
    pieces.len()
}

/// The names of the common table expressions a statement starts with: each
/// name before `AS (` in the WITH list.
fn ctes(pieces: &[Piece<'_>]) -> Vec<String> {
    let mut names = Vec::new();
    if !pieces.first().is_some_and(|piece| piece.is_keyword(&["WITH"])) {
        return names;
    }
    let mut at = 1;
    if pieces.get(at).is_some_and(|piece| piece.is_keyword(&["RECURSIVE"])) {
        at += 1;
    }
    while let Some(name) = pieces.get(at).filter(|piece| piece.is_name()) {
        at += 1;
        // An optional list of column names.
        if pieces.get(at).is_some_and(|piece| piece.is("(")) {
            at = after_parens(pieces, at);
        }
        if !pieces.get(at).is_some_and(|piece| piece.is_keyword(&["AS"])) {
            break;
        }
        at += 1;
        names.push(name.name());
        // `NOT MATERIALIZED` and the like, then the query in parentheses.
        let mut skipped = 0;
        while skipped < 2 && pieces.get(at).is_some_and(|piece| !piece.is("(")) {
            at += 1;
            skipped += 1;
        }
        if !pieces.get(at).is_some_and(|piece| piece.is("(")) {
            break;
        }
        at = after_parens(pieces, at);
        if !pieces.get(at).is_some_and(|piece| piece.is(",")) {
            break;
        }
        at += 1;
    }
    names
}
```

- [ ] **Step 4: Run to see them pass**

Run: `~/.cargo/bin/cargo test --locked -p tabletist-db --lib complete::`
Expected: PASS.

- [ ] **Step 5: Run the checks and commit**

```bash
~/.cargo/bin/cargo fmt --all --check && ~/.cargo/bin/cargo clippy --locked --workspace --all-targets -- -D warnings && ~/.cargo/bin/cargo test --locked --workspace --all-targets && RUSTDOCFLAGS='-D warnings' ~/.cargo/bin/cargo doc --locked --workspace --no-deps
git add crates/tabletist-db/src/complete.rs crates/tabletist-db/src/lib.rs
git commit -m "Find where a completion goes in a SQL script

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 3: Keywords as candidates

**Files:**
- Modify: `crates/tabletist-db/src/sql.rs` (`keywords`, `is_keyword`)
- Modify: `crates/tabletist-db/src/complete.rs` (`PHRASES`)
- Create: `src/completion.rs`
- Modify: `src/lib.rs` (add `pub mod completion;` in the list of modules, in alphabetical order)

**Interfaces:**
- Consumes: `complete::site`, `Site`, `Expects` (Task 2).
- Produces: `sql::keywords(dialect) -> impl Iterator<Item = &'static str>`, `complete::PHRASES`, `completion::{Kind, Candidate, Catalog, Listed, KEPT, list}`, and in tests `completion::LISTED`.

- [ ] **Step 1: Write the failing tests**

Append to the tests of `crates/tabletist-db/src/sql.rs`:

```rust
    #[test]
    fn the_keywords_of_a_dialect_are_the_ones_it_highlights() {
        let mysql: Vec<&str> = keywords(Dialect::MySql).collect();
        assert!(mysql.contains(&"SELECT") && mysql.contains(&"REGEXP"));
        assert!(!mysql.contains(&"ILIKE"));
        for dialect in [Dialect::Postgres, Dialect::MySql, Dialect::Sqlite] {
            for word in keywords(dialect) {
                assert!(is_keyword(dialect, word), "{dialect:?}: {word}");
                assert_eq!(word, word.to_ascii_uppercase());
            }
        }
    }
```

Create `src/completion.rs` holding only this test module:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tabletist_db::sql::tokenize;

    /// The site at the `|` of `marked` (read as SQLite), and what is typed
    /// of its word.
    pub fn site_at(marked: &str) -> (Site, String) {
        let cursor = marked.find('|').expect("a cursor mark");
        let text = marked.replacen('|', "", 1);
        let tokens = tokenize(Dialect::Sqlite, &text);
        let site = tabletist_db::complete::site(&tokens, &text, cursor).expect("a site");
        let typed = text[site.word.start..cursor].to_owned();
        (site, typed)
    }

    fn keywords_at(marked: &str, manual: bool) -> Vec<String> {
        let (site, typed) = site_at(marked);
        let catalog = Catalog {
            dialect: Dialect::Sqlite,
        };
        let listed = list(&site, &typed, manual, &catalog);
        listed.candidates.into_iter().map(|c| c.label).collect()
    }

    #[test]
    fn keywords_follow_the_case_that_is_typed() {
        assert_eq!(keywords_at("sel|", false), ["select"]);
        assert_eq!(keywords_at("SEL|", false), ["SELECT"]);
        assert_eq!(keywords_at("Sel|", false), ["SELECT"]);
        // Nothing typed: upper case, in order, phrases among them.
        let all = keywords_at("|", true);
        assert_eq!(all[0], "ALL");
        assert!(all.contains(&"GROUP BY".to_owned()));
    }

    #[test]
    fn keywords_match_from_their_start_only() {
        // Not JOIN or UNION, which only hold `in`. The exact match leads.
        assert_eq!(
            keywords_at("SELECT * FROM users WHERE in|", false),
            ["in", "inner", "inner join", "insert", "intersect", "interval", "into"]
        );
        assert_eq!(keywords_at("gr|", false), ["group", "group by"]);
    }

    #[test]
    fn a_keyword_is_inserted_as_it_reads_and_marks_what_matched() {
        let (site, typed) = site_at("sel|");
        let catalog = Catalog {
            dialect: Dialect::Sqlite,
        };
        let listed = list(&site, &typed, false, &catalog);
        assert_eq!(
            listed.candidates,
            [Candidate {
                kind: Kind::Keyword,
                label: "select".into(),
                insert: "select".into(),
                matched: 0..3,
                detail: String::new(),
            }]
        );
        assert_eq!(listed.more, 0);
    }

    #[test]
    fn no_keywords_where_a_table_or_a_new_name_belongs() {
        assert!(keywords_at("SELECT * FROM se|", false).is_empty());
        // A new name: keywords only when the list was asked for by hand.
        assert!(keywords_at("SELECT * FROM users wh|", false).is_empty());
        assert_eq!(keywords_at("SELECT * FROM users wh|", true), ["when", "where"]);
        // After a dot: a name of that table or schema, never a keyword.
        assert!(keywords_at("SELECT u.se|", false).is_empty());
    }

    #[test]
    fn a_list_keeps_the_best_and_counts_the_rest() {
        let found = (0..150)
            .map(|index| {
                let label = format!("name_{index:03}");
                let candidate = Candidate {
                    kind: Kind::Table,
                    insert: label.clone(),
                    label,
                    matched: 0..2,
                    detail: String::new(),
                };
                (0, candidate)
            })
            .rev()
            .collect();
        let listed = rank(found, "na");
        assert_eq!(listed.candidates.len(), KEPT);
        assert_eq!(listed.more, 50);
        assert_eq!(listed.candidates[0].label, "name_000");
        assert_eq!(listed.candidates[99].label, "name_099");
    }

    #[test]
    fn candidates_are_ordered_exact_then_group_then_start_then_name() {
        let named = |label: &str, group: Group, matched: Range<usize>| {
            let candidate = Candidate {
                kind: Kind::Column,
                label: label.into(),
                insert: label.into(),
                matched,
                detail: String::new(),
            };
            (group, candidate)
        };
        let found = vec![
            named("order", 1, 0..2),
            named("author", 0, 4..6),
            named("origin", 0, 0..2),
            named("Oracle", 0, 0..2),
            named("or", 1, 0..2),
        ];
        let labels: Vec<String> = rank(found, "or").candidates.into_iter().map(|c| c.label).collect();
        assert_eq!(labels, ["or", "Oracle", "origin", "author", "order"]);
    }

    #[test]
    fn text_is_found_whatever_its_case() {
        assert_eq!(find_ignoring_case("first_name", "NAME"), Some(6..10));
        assert_eq!(find_ignoring_case("Żółw", "żó"), Some(0..4));
        assert_eq!(find_ignoring_case("abc", "x"), None);
        assert_eq!(find_ignoring_case("abc", "abcd"), None);
        assert_eq!(find_ignoring_case("abc", ""), Some(0..0));
    }
}
```

- [ ] **Step 2: Run to see them fail**

Add `pub mod completion;` to `src/lib.rs`.
Run: `~/.cargo/bin/cargo test --locked --workspace --lib -- completion:: the_keywords_of`
Expected: FAIL to compile (`keywords`, `Catalog`, `list`, `rank`, `find_ignoring_case` missing).

- [ ] **Step 3: Implement**

In `crates/tabletist-db/src/sql.rs`, replace `is_keyword` with:

```rust
/// The keywords `dialect` highlights, in upper case: the words the editor
/// completes too. A keyword worth completing is worth colouring, so there
/// is one list.
pub fn keywords(dialect: Dialect) -> impl Iterator<Item = &'static str> {
    let own = match dialect {
        Dialect::Postgres => POSTGRES_KEYWORDS,
        Dialect::MySql => MYSQL_KEYWORDS,
        Dialect::Sqlite => SQLITE_KEYWORDS,
    };
    KEYWORDS.iter().chain(own).copied()
}

/// Whether `word` (any case) is highlighted as a keyword in `dialect`.
pub fn is_keyword(dialect: Dialect, word: &str) -> bool {
    keywords(dialect).any(|keyword| keyword.eq_ignore_ascii_case(word))
}
```

In `crates/tabletist-db/src/complete.rs`, under `TABLE_WORDS`:

```rust
/// Keyword phrases the editor offers as one item.
pub const PHRASES: [&str; 14] = [
    "GROUP BY",
    "ORDER BY",
    "PARTITION BY",
    "LEFT JOIN",
    "RIGHT JOIN",
    "INNER JOIN",
    "CROSS JOIN",
    "FULL JOIN",
    "IS NULL",
    "IS NOT NULL",
    "NOT IN",
    "NOT LIKE",
    "NOT EXISTS",
    "UNION ALL",
];
```

Put above the tests in `src/completion.rs`:

```rust
//! The rows of a SQL editor's completion list: what a site offers, from
//! what the workspace knows. No UI.

use std::ops::Range;

use tabletist_db::Dialect;
use tabletist_db::complete::{Expects, PHRASES, Site};

/// How many candidates a list keeps; the rest are only counted.
pub const KEPT: usize = 100;

#[cfg(test)]
thread_local! {
    /// How many times this thread worked out a list.
    pub static LISTED: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

/// What a candidate is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Keyword,
    Schema,
    Table,
    View,
    MaterializedView,
    Column,
}

/// One row of a completion list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Candidate {
    pub kind: Kind,
    /// What the row shows.
    pub label: String,
    /// What accepting inserts in place of the word.
    pub insert: String,
    /// The part of `label` that matched what was typed.
    pub matched: Range<usize>,
    /// A column's type. Empty for every other kind.
    pub detail: String,
}

/// What the workspace knows, as a list reads it.
pub struct Catalog {
    pub dialect: Dialect,
}

/// A list's rows, best first, and how many more matched.
pub struct Listed {
    pub candidates: Vec<Candidate>,
    pub more: usize,
}

/// Where a kind of candidate comes in a site's list: lower first.
type Group = u8;

/// Keywords come after every name.
const KEYWORDS: Group = 9;

/// What `site` offers for the `typed` part of its word, from `catalog`.
/// `manual` says the list was asked for by hand.
pub fn list(site: &Site, typed: &str, manual: bool, catalog: &Catalog) -> Listed {
    #[cfg(test)]
    LISTED.with(|count| count.set(count.get() + 1));
    let mut found: Vec<(Group, Candidate)> = Vec::new();
    if offers_keywords(site, manual) {
        let words = tabletist_db::sql::keywords(catalog.dialect).chain(PHRASES);
        found.extend(
            words
                .filter_map(|word| keyword(word, typed))
                .map(|candidate| (KEYWORDS, candidate)),
        );
    }
    rank(found, typed)
}

/// Whether keywords belong at `site`: not after a dot, not where a table
/// goes, and where a new name goes only when asked for by hand.
fn offers_keywords(site: &Site, manual: bool) -> bool {
    if !site.qualifier.is_empty() {
        return false;
    }
    match site.expects {
        Expects::Start | Expects::Columns => true,
        Expects::Name => manual,
        Expects::Tables => false,
    }
}

/// `word` as a candidate when it starts with `typed`, in the case typed:
/// lower case for lower-case letters, upper case otherwise.
fn keyword(word: &str, typed: &str) -> Option<Candidate> {
    let matched = find_ignoring_case(word, typed).filter(|found| found.start == 0)?;
    let lower = typed.chars().any(char::is_lowercase) && !typed.chars().any(char::is_uppercase);
    let label = if lower {
        word.to_lowercase()
    } else {
        word.to_owned()
    };
    Some(Candidate {
        kind: Kind::Keyword,
        insert: label.clone(),
        label,
        matched,
        detail: String::new(),
    })
}

/// Where `typed` first occurs in `label`, whatever the case of either.
fn find_ignoring_case(label: &str, typed: &str) -> Option<Range<usize>> {
    let needle: Vec<char> = typed.chars().flat_map(char::to_lowercase).collect();
    if needle.is_empty() {
        return Some(0..0);
    }
    label.char_indices().find_map(|(start, _)| {
        let (mut matched, mut end) = (0, start);
        for (offset, character) in label[start..].char_indices() {
            if matched == needle.len() {
                break;
            }
            for lower in character.to_lowercase() {
                if needle.get(matched) != Some(&lower) {
                    return None;
                }
                matched += 1;
            }
            end = start + offset + character.len_utf8();
        }
        (matched == needle.len()).then_some(start..end)
    })
}

/// `found` in a list's order, cut at [`KEPT`]: a candidate that inserts
/// exactly what is typed first, then by group, then names that start with
/// what is typed before names that only hold it, then by name.
fn rank(mut found: Vec<(Group, Candidate)>, typed: &str) -> Listed {
    found.sort_by_cached_key(|(group, candidate)| {
        (
            candidate.insert != typed,
            *group,
            candidate.matched.start != 0,
            candidate.label.to_lowercase(),
            candidate.label.clone(),
        )
    });
    let more = found.len().saturating_sub(KEPT);
    found.truncate(KEPT);
    Listed {
        candidates: found.into_iter().map(|(_, candidate)| candidate).collect(),
        more,
    }
}
```

- [ ] **Step 4: Run to see them pass**

Run: `~/.cargo/bin/cargo test --locked --workspace --lib completion::` and `~/.cargo/bin/cargo test --locked -p tabletist-db --lib sql::`
Expected: PASS.

- [ ] **Step 5: Run the checks and commit**

```bash
~/.cargo/bin/cargo fmt --all --check && ~/.cargo/bin/cargo clippy --locked --workspace --all-targets -- -D warnings && ~/.cargo/bin/cargo test --locked --workspace --all-targets && RUSTDOCFLAGS='-D warnings' ~/.cargo/bin/cargo doc --locked --workspace --no-deps
git add crates/tabletist-db/src/sql.rs crates/tabletist-db/src/complete.rs src/completion.rs src/lib.rs
git commit -m "Offer SQL keywords as completions

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 4: The list's model, its actions, and when it opens

**Files:**
- Modify: `src/model.rs` (`Completion`, `Wanted`, `ListedOf`, `SqlTab`, `Action`, `Workspace.catalog_generation`, tests)
- Modify: `src/app.rs` (reducers, `refresh_completion`, `frame_ui`)
- Modify: `src/ui/sql_text.rs` (`Parsed.tokens`, `parsed` and `Parsed` become `pub(crate)`, `Edited.typed`, `SqlTyped`)
- Create: `src/ui/complete_tests.rs`
- Modify: `src/ui/mod.rs` (add `#[cfg(test)] mod complete_tests;` beside `mod env_tests;`)

**Interfaces:**
- Consumes: `completion::{list, Catalog, Listed, Candidate}`, `complete::{site, Site, Expects}`.
- Produces: `SqlTab.completion: Option<Completion>`, `SqlTab.completion_wanted: Option<Wanted>`, `Completion::{new, is_of, is_of_text, is_on, highlighted, relist, move_by, enter_is_a_line_break}`, `Workspace.catalog_generation: u64`, `App::refresh_completion(&mut self, &egui::Context) -> bool`, and the actions `SqlTyped`, `OpenCompletion`, `CloseCompletion` (each `{ tab, sql_tab }`), `MoveCompletion { tab, sql_tab, step: isize }`, `AcceptCompletion { tab, sql_tab, row: Option<usize> }`.

- [ ] **Step 1: Write the failing tests**

Append to the tests of `src/model.rs`:

```rust
    fn completion_of(labels: &[&str], typed: &str) -> Completion {
        Completion::new(
            false,
            (TextPrint::of(typed), typed.len(), 0),
            completion_site(typed),
            typed.to_owned(),
            completion_rows(labels),
            false,
        )
    }

    fn completion_site(typed: &str) -> tabletist_db::complete::Site {
        tabletist_db::complete::Site {
            word: 0..typed.len(),
            qualifier: Vec::new(),
            expects: tabletist_db::complete::Expects::Columns,
            sources: Vec::new(),
            ctes: Vec::new(),
        }
    }

    fn completion_rows(labels: &[&str]) -> crate::completion::Listed {
        crate::completion::Listed {
            candidates: labels
                .iter()
                .map(|label| crate::completion::Candidate {
                    kind: crate::completion::Kind::Column,
                    label: (*label).to_owned(),
                    insert: (*label).to_owned(),
                    matched: 0..0,
                    detail: String::new(),
                })
                .collect(),
            more: 0,
        }
    }

    #[test]
    fn a_completions_highlight_moves_and_stops_at_the_ends() {
        let mut list = completion_of(&["a", "b", "c"], "");
        list.move_by(-1);
        assert_eq!((list.selected, list.moved), (0, false));
        list.move_by(1);
        list.move_by(1);
        list.move_by(1);
        assert_eq!((list.selected, list.moved), (2, true));
        // No rows: nothing to move to.
        let mut empty = completion_of(&[], "");
        empty.move_by(1);
        assert_eq!(empty.selected, 0);
        assert!(empty.highlighted().is_none());
    }

    #[test]
    fn a_moved_highlight_stays_on_its_row_until_the_typed_word_changes() {
        let relist = |list: &mut Completion, labels: &[&str], typed: &str| {
            let of = (TextPrint::of(typed), typed.len(), 0);
            let rows = completion_rows(labels);
            list.relist(of, completion_site(typed), typed.to_owned(), rows, false);
        };
        let mut list = completion_of(&["description", "desc"], "des");
        // Not moved: always the first row.
        relist(&mut list, &["dest", "description", "desc"], "des");
        assert_eq!(list.selected, 0);
        // Moved: it follows its row when rows arrive around it.
        list.move_by(2);
        relist(&mut list, &["desc", "description"], "des");
        assert_eq!(list.highlighted().map(|c| c.label.as_str()), Some("desc"));
        assert!(list.moved);
        // Its row went: the first row, and not moved.
        relist(&mut list, &["description"], "des");
        assert_eq!((list.selected, list.moved), (0, false));
        // The typed word changed: the first row again, moved or not.
        relist(&mut list, &["description", "desc"], "des");
        list.move_by(1);
        assert_eq!((list.selected, list.moved), (1, true));
        relist(&mut list, &["desc", "description"], "desc");
        assert_eq!((list.selected, list.moved), (0, false));
    }

    #[test]
    fn enter_is_a_line_break_on_what_is_already_typed() {
        let list = completion_of(&["as", "asc"], "as");
        assert!(list.enter_is_a_line_break("as"));
        let mut moved = completion_of(&["as", "asc"], "as");
        moved.move_by(1);
        assert!(!moved.enter_is_a_line_break("as"));
        // No row to insert: Enter is the editor's.
        assert!(completion_of(&[], "zz").enter_is_a_line_break("zz"));
    }

    #[test]
    fn a_list_is_on_the_word_it_was_opened_on() {
        let list = completion_of(&["select"], "se");
        let mut site = completion_site("sel");
        assert!(list.is_on(&site));
        // The word is gone: the cursor is at its start, or it was deleted.
        site.word = 0..0;
        assert!(!list.is_on(&site));
        // Another word.
        site.word = 4..6;
        assert!(!list.is_on(&site));
        // A list opened on an empty word goes on as the word is typed.
        let empty = completion_of(&["select"], "");
        assert!(empty.is_on(&completion_site("s")));
        assert!(empty.is_on(&completion_site("")));
    }
```

Create `src/ui/complete_tests.rs`:

```rust
//! Headless tests of the SQL editor's completion list: what opens it, its
//! keys, what it inserts and what it asks the backend for.

use egui::{Key, Modifiers};

use crate::backend::Command;
use crate::completion::LISTED;
use crate::model::{Action, Completion, ConnTabId, SqlTab, TabId};
use crate::testing::Harness;
use crate::ui::sql_text::TOKENIZED;

/// A connected fixture workspace showing an empty SQL editor that has the
/// keyboard.
fn editor() -> (Harness, ConnTabId) {
    let mut harness = Harness::new();
    let tab = harness.connect_fake();
    harness.press(Key::T, Modifiers::COMMAND);
    (harness, tab)
}

/// Types `text` at the cursor, as the keyboard does.
fn type_text(harness: &mut Harness, text: &str) {
    harness.frame(vec![egui::Event::Text(text.into())]);
    harness.settle();
}

/// Pastes `text` at the cursor: an edit that is not typing.
fn paste(harness: &mut Harness, text: &str) {
    harness.frame(vec![egui::Event::Paste(text.into())]);
    harness.settle();
}

fn sql(harness: &Harness, tab: ConnTabId) -> &SqlTab {
    harness.app.workspace(tab).unwrap().active_sql_tab().unwrap()
}

fn ids(harness: &Harness, tab: ConnTabId) -> (ConnTabId, TabId) {
    (tab, sql(harness, tab).id)
}

fn list(harness: &Harness, tab: ConnTabId) -> Option<&Completion> {
    sql(harness, tab).completion.as_ref()
}

/// What the open list offers, in order; nothing when it is closed.
fn labels(harness: &Harness, tab: ConnTabId) -> Vec<String> {
    list(harness, tab)
        .map(|list| list.candidates.iter().map(|c| c.label.clone()).collect())
        .unwrap_or_default()
}

#[test]
fn typing_two_letters_opens_the_list_and_one_does_not() {
    let (mut harness, tab) = editor();
    type_text(&mut harness, "s");
    assert!(list(&harness, tab).is_none());
    type_text(&mut harness, "e");
    assert_eq!(labels(&harness, tab), ["select", "set"]);
    let open = list(&harness, tab).unwrap();
    assert_eq!((open.selected, open.manual, open.typed.as_str()), (0, false, "se"));
}

#[test]
fn the_list_narrows_while_typing_and_closes_with_nothing_left() {
    let (mut harness, tab) = editor();
    type_text(&mut harness, "se");
    type_text(&mut harness, "l");
    assert_eq!(labels(&harness, tab), ["select"]);
    type_text(&mut harness, "x");
    assert!(list(&harness, tab).is_none());
}

#[test]
fn a_word_that_is_already_whole_opens_nothing() {
    let (mut harness, tab) = editor();
    // `set` is the only keyword that starts with `set`.
    type_text(&mut harness, "set");
    assert!(list(&harness, tab).is_none());
}

#[test]
fn pasting_and_deleting_open_nothing_and_deleting_keeps_an_open_list() {
    let (mut harness, tab) = editor();
    paste(&mut harness, "sele");
    assert!(list(&harness, tab).is_none());
    harness.press(Key::Backspace, Modifiers::NONE);
    assert_eq!(sql(&harness, tab).text, "sel");
    assert!(list(&harness, tab).is_none());
    type_text(&mut harness, "e");
    assert_eq!(labels(&harness, tab), ["select"]);
    // Open: deleting keeps it while its word remains.
    harness.press(Key::Backspace, Modifiers::NONE);
    harness.press(Key::Backspace, Modifiers::NONE);
    assert_eq!(labels(&harness, tab), ["select", "set"]);
    harness.press(Key::Backspace, Modifiers::NONE);
    harness.press(Key::Backspace, Modifiers::NONE);
    assert_eq!(sql(&harness, tab).text, "");
    assert!(list(&harness, tab).is_none());
}

#[test]
fn the_list_closes_when_the_cursor_leaves_its_word() {
    let (mut harness, tab) = editor();
    type_text(&mut harness, "se");
    assert!(list(&harness, tab).is_some());
    harness.press(Key::Home, Modifiers::NONE);
    assert_eq!(sql(&harness, tab).cursor, 0);
    assert!(list(&harness, tab).is_none());
}

#[test]
fn no_list_after_a_table_name_or_as() {
    let (mut harness, tab) = editor();
    paste(&mut harness, "select * from users ");
    // Most likely an alias: Enter must stay a line break.
    type_text(&mut harness, "wh");
    assert!(list(&harness, tab).is_none());
    // Asked for by hand, the keywords are there.
    let (tab_id, sql_tab) = ids(&harness, tab);
    harness.app.apply(Action::OpenCompletion {
        tab: tab_id,
        sql_tab,
    });
    harness.settle();
    assert_eq!(labels(&harness, tab), ["when", "where"]);
    assert!(list(&harness, tab).unwrap().manual);

    let (mut harness, tab) = editor();
    paste(&mut harness, "select 1 as ");
    type_text(&mut harness, "to");
    assert!(list(&harness, tab).is_none());
}

#[test]
fn a_list_asked_for_by_hand_opens_on_an_empty_word() {
    let (mut harness, tab) = editor();
    let (tab_id, sql_tab) = ids(&harness, tab);
    harness.app.apply(Action::OpenCompletion {
        tab: tab_id,
        sql_tab,
    });
    harness.settle();
    let open = list(&harness, tab).expect("a list");
    assert!(open.manual);
    assert_eq!(open.candidates[0].label, "ALL");
    // It goes on as its word is typed, still by hand.
    type_text(&mut harness, "s");
    assert_eq!(labels(&harness, tab), ["select", "set", "show"]);
    assert!(list(&harness, tab).unwrap().manual);
}

#[test]
fn a_list_belongs_to_the_editor_on_screen() {
    let (mut harness, tab) = editor();
    type_text(&mut harness, "se");
    harness.press(Key::T, Modifiers::COMMAND);
    let workspace = harness.app.workspace(tab).unwrap();
    assert_eq!(workspace.sql_tabs().count(), 2);
    assert!(workspace.sql_tabs().all(|sql| sql.completion.is_none()));
}

#[test]
fn running_closes_the_list() {
    let (mut harness, tab) = editor();
    type_text(&mut harness, "se");
    harness.press(Key::Enter, Modifiers::COMMAND);
    assert!(matches!(
        crate::testing::last_sent(&harness.app),
        Command::RunSql { .. }
    ));
    assert!(list(&harness, tab).is_none());
}

#[test]
fn a_quiet_frame_computes_nothing() {
    let (mut harness, tab) = editor();
    type_text(&mut harness, "se");
    assert!(list(&harness, tab).is_some());
    let counts = || (LISTED.with(|c| c.get()), TOKENIZED.with(|c| c.get()));
    let before = counts();
    harness.settle();
    harness.settle();
    assert_eq!(counts(), before);
    // One more letter: the script is tokenized once, for the editor and
    // the list together.
    type_text(&mut harness, "l");
    assert_eq!(counts().1, before.1 + 1);
    assert_eq!(labels(&harness, tab), ["select"]);
}
```

- [ ] **Step 2: Run to see them fail**

Add `#[cfg(test)] mod complete_tests;` to `src/ui/mod.rs`.
Run: `~/.cargo/bin/cargo test --locked --lib complete_tests`
Expected: FAIL to compile (`Completion`, the actions, `SqlTab.completion` missing).

- [ ] **Step 3: The model**

In `src/model.rs`, add to the imports `use std::sync::Arc;` (if it is not there) and, above `SqlTab`:

```rust
/// How a completion list was asked for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Wanted {
    /// By typing a word.
    Typed,
    /// By hand (`Ctrl+Space`).
    Manual,
}

/// What a completion list was worked out from: the script, the cursor and
/// the catalog's generation.
pub type ListedOf = (TextPrint, usize, u64);

/// A SQL editor's completion list, while it is open. Its site holds names
/// read from the script, so it is never logged (it has no `Debug`).
#[derive(Clone)]
pub struct Completion {
    /// Opened with `Ctrl+Space`, not by typing.
    pub manual: bool,
    of: ListedOf,
    pub site: tabletist_db::complete::Site,
    /// The part of the word before the cursor.
    pub typed: String,
    pub candidates: Arc<Vec<crate::completion::Candidate>>,
    /// Matches beyond the ones kept.
    pub more: usize,
    /// The highlighted row.
    pub selected: usize,
    /// The user moved the highlight since the typed word last changed.
    pub moved: bool,
    /// Objects or columns this site needs are being fetched.
    pub loading: bool,
    /// Insert the highlighted row on the next draw.
    pub accept: bool,
}

impl Completion {
    pub fn new(
        manual: bool,
        of: ListedOf,
        site: tabletist_db::complete::Site,
        typed: String,
        listed: crate::completion::Listed,
        loading: bool,
    ) -> Self {
        Self {
            manual,
            of,
            site,
            typed,
            candidates: Arc::new(listed.candidates),
            more: listed.more,
            selected: 0,
            moved: false,
            loading,
            accept: false,
        }
    }

    /// Whether the list was worked out from `of`: nothing to recompute.
    pub fn is_of(&self, of: ListedOf) -> bool {
        self.of == of
    }

    /// Whether the list was worked out from the script `text`.
    pub fn is_of_text(&self, text: &str) -> bool {
        self.of.0 == TextPrint::of(text)
    }

    /// Whether a cursor at `site` is still on the word the list is on: the
    /// same start and qualifier, and the word has not gone (the cursor at
    /// its start, or all of it deleted).
    pub fn is_on(&self, site: &tabletist_db::complete::Site) -> bool {
        site.word.start == self.site.word.start
            && site.qualifier == self.site.qualifier
            && (self.site.word.is_empty() || !site.word.is_empty())
    }

    pub fn highlighted(&self) -> Option<&crate::completion::Candidate> {
        self.candidates.get(self.selected)
    }

    /// Replaces the rows. The highlight is the first row, unless the user
    /// moved it and the typed word is the same: then it stays on its row
    /// while that row is still listed.
    pub fn relist(
        &mut self,
        of: ListedOf,
        site: tabletist_db::complete::Site,
        typed: String,
        listed: crate::completion::Listed,
        loading: bool,
    ) {
        let kept = (self.moved && typed == self.typed)
            .then(|| self.highlighted())
            .flatten()
            .and_then(|old| {
                listed
                    .candidates
                    .iter()
                    .position(|new| new.kind == old.kind && new.insert == old.insert)
            });
        self.selected = kept.unwrap_or(0);
        self.moved = kept.is_some();
        self.of = of;
        self.site = site;
        self.typed = typed;
        self.candidates = Arc::new(listed.candidates);
        self.more = listed.more;
        self.loading = loading;
    }

    /// Moves the highlight by `step` rows, stopping at the ends.
    pub fn move_by(&mut self, step: isize) {
        let Some(last) = self.candidates.len().checked_sub(1) else {
            return;
        };
        let next = self.selected.saturating_add_signed(step).min(last);
        if next != self.selected {
            self.selected = next;
            self.moved = true;
        }
    }

    /// Whether Enter is the editor's line break rather than an insertion:
    /// there is no row, or the highlighted one inserts what the script
    /// `text` already reads there.
    pub fn enter_is_a_line_break(&self, text: &str) -> bool {
        self.highlighted().is_none_or(|candidate| {
            let word = text.get(self.site.word.clone());
            candidate.is_typed(&self.typed) || word.is_some_and(|word| candidate.is_typed(word))
        })
    }
}
```

Add to `SqlTab` (after `focus_editor`), and set both to `None` in `SqlTab::new`:

```rust
    /// The completion list, while it is open.
    pub completion: Option<Completion>,
    /// A list was asked for and is not worked out yet:
    /// `App::refresh_completion` takes this and decides.
    pub completion_wanted: Option<Wanted>,
```

Leave `SqlTab`'s hand-written `Debug` as it is: it must not print the list.

Add to `Workspace` (after `server_version`), set to `0` where a `Workspace` is built:

```rust
    /// Bumped whenever the names a completion list reads change (the
    /// tree's objects, the columns), so an open list is worked out again.
    pub catalog_generation: u64,
```

Add to `Action`, after `SqlEditorFocused`:

```rust
    /// A SQL editor's text changed by typing (the view tells): a
    /// completion list may open.
    SqlTyped {
        tab: ConnTabId,
        sql_tab: TabId,
    },
    /// Open the completion list at the editor's cursor (`Ctrl+Space`).
    OpenCompletion {
        tab: ConnTabId,
        sql_tab: TabId,
    },
    /// Move the completion list's highlight by this many rows.
    MoveCompletion {
        tab: ConnTabId,
        sql_tab: TabId,
        step: isize,
    },
    /// Insert a row of the completion list: the one given (a click), or
    /// the highlighted one.
    AcceptCompletion {
        tab: ConnTabId,
        sql_tab: TabId,
        row: Option<usize>,
    },
    /// Close the completion list.
    CloseCompletion {
        tab: ConnTabId,
        sql_tab: TabId,
    },
```

- [ ] **Step 4: The reducers and `refresh_completion`**

In `src/app.rs`, import `Completion` and `Wanted` from `crate::model`, and `TextPrint` if it is not imported. `app.rs` already imports `crate::theme::Catalog`: name the completion's one by its path, `crate::completion::Catalog`. Replace the `Action::RunSql` arm and add the new arms after it:

```rust
            Action::RunSql { tab, sql_tab, all } => {
                if let Some(sql) = self.sql_tab_mut(tab, sql_tab) {
                    sql.completion = None;
                    sql.completion_wanted = None;
                }
                self.run_sql(tab, sql_tab, all);
            }
            Action::SqlTyped { tab, sql_tab } => {
                if let Some(sql) = self.sql_tab_mut(tab, sql_tab) {
                    // A list asked for by hand in the same frame stays so.
                    if sql.completion_wanted.is_none() {
                        sql.completion_wanted = Some(Wanted::Typed);
                    }
                }
            }
            Action::OpenCompletion { tab, sql_tab } => {
                if let Some(sql) = self.sql_tab_mut(tab, sql_tab) {
                    sql.completion_wanted = Some(Wanted::Manual);
                }
            }
            Action::MoveCompletion { tab, sql_tab, step } => {
                if let Some(list) = self
                    .sql_tab_mut(tab, sql_tab)
                    .and_then(|sql| sql.completion.as_mut())
                {
                    list.move_by(step);
                }
            }
            Action::AcceptCompletion { tab, sql_tab, row } => {
                if let Some(list) = self
                    .sql_tab_mut(tab, sql_tab)
                    .and_then(|sql| sql.completion.as_mut())
                {
                    if let Some(row) = row.filter(|row| *row < list.candidates.len()) {
                        list.selected = row;
                    }
                    // The editor's view inserts it on its next draw.
                    list.accept = list.highlighted().is_some();
                }
            }
            Action::CloseCompletion { tab, sql_tab } => {
                if let Some(sql) = self.sql_tab_mut(tab, sql_tab) {
                    sql.completion = None;
                    sql.completion_wanted = None;
                }
            }
```

Add beside `run_sql`:

```rust
    /// Works out the completion list of the SQL editor on screen: opens
    /// the one that was asked for, recomputes an open one whose script,
    /// cursor or catalog changed, and closes one with nothing left to
    /// offer. Lists of editors that are not on screen are dropped.
    /// Returns whether a list changed, so the frame is drawn again.
    ///
    /// Called from `frame_ui`, which has the egui context: the script's
    /// tokens come from the cache the editor's layouter fills, so a script
    /// is tokenized once per change.
    pub fn refresh_completion(&mut self, ctx: &egui::Context) -> bool {
        let active = self.active_sql();
        let mut changed = false;
        for conn in &mut self.tabs {
            let ConnTabContent::Workspace(workspace) = &mut conn.content else {
                continue;
            };
            for sql in workspace.sql_tabs_mut() {
                if active != Some((conn.id, sql.id)) {
                    changed |= sql.completion.take().is_some();
                    sql.completion_wanted = None;
                }
            }
        }
        let Some((tab, id)) = active else {
            return changed;
        };
        let palette = self.palette;
        let Some(workspace) = self.workspace_mut(tab) else {
            return changed;
        };
        let dialect = workspace.driver.dialect();
        let generation = workspace.catalog_generation;
        let Some(sql) = workspace.sql_tab_mut(id) else {
            return changed;
        };
        let wanted = sql.completion_wanted.take();
        let was_open = sql.completion.is_some();
        if wanted.is_none() && !was_open {
            return changed;
        }
        let of = (TextPrint::of(&sql.text), sql.cursor, generation);
        if wanted.is_none() && sql.completion.as_ref().is_some_and(|list| list.is_of(of)) {
            return changed;
        }
        let editor = crate::ui::sql_text::editor_id(tab, id);
        let parsed = crate::ui::sql_text::parsed(ctx, editor, dialect, &palette, &sql.text);
        let Some(site) = tabletist_db::complete::site(&parsed.tokens, &sql.text, sql.cursor)
        else {
            sql.completion = None;
            return changed | was_open;
        };
        // A cursor that is stale or inside a character has no word.
        let Some(typed) = sql.text.get(site.word.start..sql.cursor).map(str::to_owned) else {
            sql.completion = None;
            return changed | was_open;
        };
        // An open list goes on only for the word it was opened on.
        let going = sql.completion.as_ref().is_some_and(|list| list.is_on(&site));
        let manual = match wanted {
            Some(Wanted::Manual) => true,
            _ if going => sql.completion.as_ref().is_some_and(|list| list.manual),
            Some(Wanted::Typed) => false,
            None => {
                // The cursor left the word.
                sql.completion = None;
                return true;
            }
        };
        let catalog = crate::completion::Catalog { dialect };
        let listed = crate::completion::list(&site, &typed, manual, &catalog);
        let loading = false;
        let empty = listed.candidates.is_empty() && !loading;
        if going {
            match sql.completion.as_mut() {
                Some(list) if !empty => {
                    list.manual = manual;
                    list.relist(of, site, typed, listed, loading);
                }
                _ => sql.completion = None,
            }
            return true;
        }
        // Opening. By typing: a word of two characters or more, or right
        // after a dot; not where a new name goes; not when the only row
        // is what is already typed.
        let only_exact = listed.more == 0
            && matches!(listed.candidates.as_slice(), [only] if only.is_typed(&typed));
        let after_dot = typed.is_empty() && !site.qualifier.is_empty();
        let new_name = site.expects == tabletist_db::complete::Expects::Name
            && site.qualifier.is_empty();
        let opens = manual
            || ((typed.chars().count() >= 2 || after_dot) && !new_name && !only_exact);
        sql.completion = (opens && !empty)
            .then(|| Completion::new(manual, of, site, typed, listed, loading));
        changed | was_open | sql.completion.is_some()
    }
```

In `frame_ui`, call it after each `apply_actions`:

```rust
        self.poll_backend();
        self.apply_actions();
        self.refresh_completion(ui.ctx());
        // A dialog takes the keyboard: no shortcut acts behind it.
        if self.dialog.is_none() {
            crate::ui::keys::handle(self, ui.ctx());
        }
        crate::ui::show(self, ui);
        self.apply_actions();
        // A list that opened or changed shows on the next frame.
        if self.refresh_completion(ui.ctx()) {
            ui.ctx().request_repaint();
        }
```

- [ ] **Step 5: The editor keeps its tokens and says when it was typed in**

In `src/ui/sql_text.rs`:

Make `Parsed` and `parsed` `pub(crate)`, and keep the tokens:

```rust
pub(crate) struct Parsed {
    /// What it was worked out from: the script, the dialect that reads it
    /// and the colours.
    of: (TextPrint, Dialect, Palette),
    runs: Vec<(Range<usize>, Color32)>,
    statements: Vec<Statement>,
    /// The script's tokens, for the completion list.
    pub(crate) tokens: Vec<Token>,
}
```

and in `parsed`, build it with the tokens moved in last:

```rust
    let tokens = sql::tokenize(dialect, text);
    let parsed = Arc::new(Parsed {
        of,
        runs: runs(&tokens, text, palette),
        statements: sql::statements_from(text, &tokens),
        tokens,
    });
```

Add to `Edited`:

```rust
    /// The field's text changed this frame by typing: not by a paste, an
    /// undo, a deletion or an input method's text while it is composed.
    typed: bool,
```

In `edit`, before building `Edited`:

```rust
    let typed = output.response.changed()
        && ui.input(|input| {
            input.events.iter().any(|event| {
                matches!(
                    event,
                    egui::Event::Text(_) | egui::Event::Ime(egui::ImeEvent::Commit(_))
                )
            })
        });
```

and add `typed,` to the `Edited` it returns.

In `show`, beside the `SqlEditorFocused` push:

```rust
    if edited.typed {
        app.actions.push(Action::SqlTyped { tab, sql_tab: id });
    }
```

- [ ] **Step 6: Run to see them pass**

Run: `~/.cargo/bin/cargo test --locked --lib complete_tests` and `~/.cargo/bin/cargo test --locked --lib model::tests`
Expected: PASS.

If `a_quiet_frame_computes_nothing` counts two tokenizations for one letter, `refresh_completion` and the layouter disagree on the cache key: check that both pass the same `Dialect` and `app.palette`.

- [ ] **Step 7: Run the checks and commit**

```bash
~/.cargo/bin/cargo fmt --all --check && ~/.cargo/bin/cargo clippy --locked --workspace --all-targets -- -D warnings && ~/.cargo/bin/cargo test --locked --workspace --all-targets && RUSTDOCFLAGS='-D warnings' ~/.cargo/bin/cargo doc --locked --workspace --no-deps
git add src/model.rs src/app.rs src/ui/sql_text.rs src/ui/complete_tests.rs src/ui/mod.rs
git commit -m "Open a completion list while SQL is typed

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 5: The list's keys, and insertion with undo

**Files:**
- Modify: `src/ui/keys.rs` (`handle`, `take_press`, `is_press`)
- Modify: `src/ui/sql_text.rs` (`insert_completion`, `char_index`, `edit`, `show`)
- Modify: `src/ui/complete_tests.rs`

**Interfaces:**
- Consumes: `SqlTab.completion`, `Completion::{enter_is_a_line_break, is_of_text, highlighted}`, the completion actions, `Field.hold_escape`.
- Produces: the keys of the spec's table; an accepted row inserted on the editor's next draw.

- [ ] **Step 1: Write the failing tests** (append to `src/ui/complete_tests.rs`)

```rust
/// What a real Ctrl press reports: outside macOS Ctrl is the command key
/// too.
fn ctrl() -> Modifiers {
    if cfg!(target_os = "macos") {
        Modifiers::CTRL
    } else {
        Modifiers::CTRL | Modifiers::COMMAND
    }
}

fn editor_has_keyboard(harness: &Harness, tab: ConnTabId) -> bool {
    let (tab, id) = ids(harness, tab);
    let editor = crate::ui::sql_text::editor_id(tab, id);
    harness.ctx.memory(|memory| memory.has_focus(editor))
}

fn selected(harness: &Harness, tab: ConnTabId) -> Option<usize> {
    list(harness, tab).map(|list| list.selected)
}

#[test]
fn down_and_up_move_the_highlight_and_stop_at_the_ends() {
    let (mut harness, tab) = editor();
    type_text(&mut harness, "se");
    for (key, row) in [
        (Key::ArrowDown, 1),
        (Key::ArrowDown, 1),
        (Key::ArrowUp, 0),
        (Key::ArrowUp, 0),
    ] {
        harness.press(key, Modifiers::NONE);
        assert_eq!(selected(&harness, tab), Some(row), "{key:?}");
    }
    // The editor's cursor stayed where it was.
    assert_eq!(sql(&harness, tab).cursor, 2);
}

#[test]
fn tab_inserts_the_highlighted_row() {
    let (mut harness, tab) = editor();
    type_text(&mut harness, "se");
    harness.press(Key::ArrowDown, Modifiers::NONE);
    harness.press(Key::Tab, Modifiers::NONE);
    let sql = sql(&harness, tab);
    assert_eq!((sql.text.as_str(), sql.cursor), ("set", 3));
    assert!(sql.completion.is_none());
    assert!(editor_has_keyboard(&harness, tab));
}

#[test]
fn enter_inserts_and_what_is_typed_next_follows_it() {
    let (mut harness, tab) = editor();
    type_text(&mut harness, "sel");
    harness.press(Key::Enter, Modifiers::NONE);
    assert_eq!(sql(&harness, tab).text, "select");
    type_text(&mut harness, " 1");
    assert_eq!(sql(&harness, tab).text, "select 1");
}

#[test]
fn enter_on_what_is_already_typed_is_a_line_break() {
    let (mut harness, tab) = editor();
    type_text(&mut harness, "as");
    assert_eq!(labels(&harness, tab), ["as", "asc"]);
    harness.press(Key::Enter, Modifiers::NONE);
    assert_eq!(sql(&harness, tab).text, "as\n");
    assert!(list(&harness, tab).is_none());
}

#[test]
fn shift_enter_and_shift_tab_are_the_editors() {
    let (mut harness, tab) = editor();
    type_text(&mut harness, "se");
    harness.press(Key::Enter, Modifiers::SHIFT);
    assert_eq!(sql(&harness, tab).text, "se\n");

    let (mut harness, tab) = editor();
    type_text(&mut harness, "se");
    harness.press(Key::Tab, Modifiers::SHIFT);
    // Nothing to outdent, and nothing inserted.
    assert_eq!(sql(&harness, tab).text, "se");
    assert!(list(&harness, tab).is_some());
}

#[test]
fn escape_closes_the_list_then_leaves_the_editor() {
    let (mut harness, tab) = editor();
    type_text(&mut harness, "se");
    harness.press(Key::Escape, Modifiers::NONE);
    assert!(list(&harness, tab).is_none());
    assert!(editor_has_keyboard(&harness, tab), "the first Esc is the list's");
    harness.press(Key::Escape, Modifiers::NONE);
    assert!(!editor_has_keyboard(&harness, tab), "the second leaves");
}

#[test]
fn left_and_right_stay_the_editors_while_the_list_is_open() {
    let (mut harness, tab) = editor();
    type_text(&mut harness, "se");
    harness.press(Key::ArrowLeft, Modifiers::NONE);
    assert_eq!(sql(&harness, tab).cursor, 1);
    assert!(editor_has_keyboard(&harness, tab));
    // Still on its word, with less of it typed.
    assert_eq!(labels(&harness, tab), ["select", "set", "show"]);
}

#[test]
fn undo_restores_the_typed_word() {
    let (mut harness, tab) = editor();
    type_text(&mut harness, "sel");
    harness.press(Key::Tab, Modifiers::NONE);
    assert_eq!(sql(&harness, tab).text, "select");
    harness.press(Key::Z, Modifiers::COMMAND);
    assert_eq!(sql(&harness, tab).text, "sel");
}

#[test]
fn ctrl_space_opens_the_list_by_hand() {
    let (mut harness, tab) = editor();
    harness.press(Key::Space, ctrl());
    let open = list(&harness, tab).expect("a list");
    assert!(open.manual);
    assert_eq!(open.candidates[0].label, "ALL");
    assert_eq!(sql(&harness, tab).text, "");
}

#[test]
fn ctrl_n_and_ctrl_p_move_the_highlight_only_in_the_terminal_look() {
    let (mut harness, tab) = editor();
    harness.set_look(crate::theme::Look::omarchy());
    type_text(&mut harness, "se");
    harness.press(Key::N, ctrl());
    assert_eq!(selected(&harness, tab), Some(1));
    // The list took the key: New connection did not open.
    assert!(harness.app.dialog.is_none());
    harness.press(Key::P, ctrl());
    assert_eq!(selected(&harness, tab), Some(0));

    // The other looks: not the list's keys.
    let (mut harness, tab) = editor();
    type_text(&mut harness, "se");
    harness.press(Key::N, ctrl());
    assert!(selected(&harness, tab).is_none_or(|row| row == 0));
}
```

- [ ] **Step 2: Run to see them fail**

Run: `~/.cargo/bin/cargo test --locked --lib complete_tests`
Expected: the new tests FAIL (the keys reach the editor: Tab indents, Enter breaks the line, Esc leaves).

- [ ] **Step 3: The keys**

In `src/ui/keys.rs`, in `handle`, before `ctx.input_mut`:

```rust
    // The open completion list of the editor on screen: whether it has a
    // row, and whether Enter stays the editor's line break.
    let completing = sql.and_then(|(tab, id)| {
        let sql = app.workspace(tab)?.sql_tab(id)?;
        let list = sql.completion.as_ref()?;
        Some((
            !list.candidates.is_empty(),
            list.enter_is_a_line_break(&sql.text),
        ))
    });
    let terminal = app.look.terminal;
```

At the top of the `ctx.input_mut` closure, before the `RunSql` block:

```rust
        // The completion list's keys come first: the editor never sees
        // them, nor do the shortcuts below (Ctrl+N, Ctrl+P).
        if let Some((tab, sql_tab)) = sql {
            if editing && take_press(input, Modifiers::CTRL, Key::Space) {
                actions.push(Action::OpenCompletion { tab, sql_tab });
            }
            if let Some((has_row, enter_breaks)) = completing {
                let accept = Action::AcceptCompletion {
                    tab,
                    sql_tab,
                    row: None,
                };
                let close = Action::CloseCompletion { tab, sql_tab };
                if has_row {
                    let mut moves = vec![
                        (Modifiers::NONE, Key::ArrowDown, 1),
                        (Modifiers::NONE, Key::ArrowUp, -1),
                    ];
                    if terminal {
                        moves.push((Modifiers::CTRL, Key::N, 1));
                        moves.push((Modifiers::CTRL, Key::P, -1));
                    }
                    for (modifiers, key, step) in moves {
                        if take_press(input, modifiers, key) {
                            actions.push(Action::MoveCompletion { tab, sql_tab, step });
                        }
                    }
                    if take_press(input, Modifiers::NONE, Key::Tab) {
                        actions.push(accept);
                    } else if enter_breaks {
                        // The editor gets its line break; the list is done.
                        let pressed = |event: &egui::Event| {
                            is_press(event, Modifiers::NONE, Key::Enter)
                        };
                        if input.events.iter().any(pressed) {
                            actions.push(close);
                        }
                    } else if take_press(input, Modifiers::NONE, Key::Enter) {
                        actions.push(accept);
                    }
                }
                if take_press(input, Modifiers::NONE, Key::Escape) {
                    actions.push(Action::CloseCompletion { tab, sql_tab });
                }
            }
        }
```

(`accept` and `close` are each moved at most once: the branches exclude each other. If the borrow checker objects, build the action in each branch.)

Beside `consume_press`:

```rust
/// Whether `event` is `key` going down with exactly `modifiers`: no
/// extra Shift, which `consume_key` and `matches_logically` let through.
fn is_press(event: &egui::Event, modifiers: Modifiers, key: Key) -> bool {
    matches!(
        event,
        egui::Event::Key {
            key: pressed,
            modifiers: held,
            pressed: true,
            ..
        } if *pressed == key && held.matches_exact(modifiers)
    )
}

/// Whether `key` went down this frame with exactly `modifiers` (a held
/// key's repeats count). Takes the presses, so neither the editor nor a
/// shortcut after this sees them.
fn take_press(input: &mut egui::InputState, modifiers: Modifiers, key: Key) -> bool {
    let before = input.events.len();
    input.events.retain(|event| !is_press(event, modifiers, key));
    input.events.len() != before
}
```

- [ ] **Step 4: Insertion, and holding Esc while the list is open**

In `src/ui/sql_text.rs`, import `egui::text::CCursor` beside `CCursorRange`, and add beside `byte_offset`:

```rust
/// The character index, as egui counts a cursor, of the byte offset `byte`
/// of `text`.
fn char_index(text: &str, byte: usize) -> usize {
    text.get(..byte)
        .map_or_else(|| text.chars().count(), |before| before.chars().count())
}

/// Carries out an accepted completion before the field handles the
/// frame's events: replaces the word with the highlighted row's text and
/// puts the cursor after it. The list is taken off the tab either way.
///
/// egui groups undo by time and would merge the insertion with the typing
/// around it, so an undo point holding the script and the cursor as they
/// were goes in first: one undo takes back exactly the insertion.
fn insert_completion(ctx: &egui::Context, id: Id, sql_tab: &mut SqlTab) {
    let Some(list) = sql_tab.completion.take_if(|list| list.accept) else {
        return;
    };
    let Some(candidate) = list.highlighted() else {
        return;
    };
    let word = list.site.word.clone();
    // The list must be of this very script, and the row must change it.
    let changes = sql_tab
        .text
        .get(word.clone())
        .is_some_and(|old| old != candidate.insert);
    if !list.is_of_text(&sql_tab.text) || !changes {
        return;
    }
    let mut state = egui::TextEdit::load_state(ctx, id).unwrap_or_default();
    let before = state.cursor.char_range().unwrap_or_else(|| {
        CCursorRange::one(CCursor::new(char_index(&sql_tab.text, sql_tab.cursor)))
    });
    let mut undoer = state.undoer();
    undoer.add_undo(&(before, sql_tab.text.clone()));
    state.set_undoer(undoer);
    sql_tab.text.replace_range(word.clone(), &candidate.insert);
    sql_tab.cursor = word.start + candidate.insert.len();
    let after = CCursor::new(char_index(&sql_tab.text, sql_tab.cursor));
    state.cursor.set_char_range(Some(CCursorRange::one(after)));
    egui::TextEdit::store_state(ctx, id, state);
}
```

Call it first thing in `edit`:

```rust
    insert_completion(ui.ctx(), field.id, sql_tab);
```

In `show`, before the scroll area's closure, read whether a list is open, and pass it as the field's `hold_escape` (replacing `false`):

```rust
    // An open completion list takes Esc: the editor keeps the keyboard.
    let holds_escape = sql_tab.completion.is_some();
```

```rust
                hold_escape: holds_escape,
```

Add a unit test beside `a_character_index_becomes_a_byte_offset`:

```rust
    #[test]
    fn a_byte_offset_becomes_a_character_index() {
        assert_eq!(char_index("SELECT 1", 8), 8);
        assert_eq!(char_index("żółw 1", 7), 4);
        // Past the end, or inside a character: the end.
        assert_eq!(char_index("żółw", 99), 4);
        assert_eq!(char_index("żółw", 1), 4);
    }
```

- [ ] **Step 5: Run to see them pass**

Run: `~/.cargo/bin/cargo test --locked --lib complete_tests` and `~/.cargo/bin/cargo test --locked --lib ui::`
Expected: PASS, including every earlier key test (`Mod+Return`, `Mod+N`, `Mod+P`, the tree and grid arrows).

If `undo_restores_the_typed_word` restores less or more than `sel`, read `Undoer::add_undo` and `Undoer::undo` in the egui fork (`crates/egui/src/util/undoer.rs`): the point added must be the last entry of `undos` when the insertion frame feeds its state.

- [ ] **Step 6: Run the checks and commit**

```bash
~/.cargo/bin/cargo fmt --all --check && ~/.cargo/bin/cargo clippy --locked --workspace --all-targets -- -D warnings && ~/.cargo/bin/cargo test --locked --workspace --all-targets
git add src/ui/keys.rs src/ui/sql_text.rs src/ui/complete_tests.rs
git commit -m "Move in the completion list and insert from it by key

The list is drawn by the next commit: the two land together.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 6: The list's view

**Files:**
- Create: `src/ui/sql_complete.rs`
- Modify: `src/ui/mod.rs` (add `pub mod sql_complete;` before `pub mod sql_editor;`)
- Modify: `src/ui/sql_text.rs` (`show`, `forget`)
- Modify: `src/typography.rs` (two roles), `src/typography/fonts.rs` (the glyph test)
- Modify: `src/ui/workspace.rs` (`status_line`)
- Modify: `src/ui/keys.rs` (`SHORTCUTS`, its test)
- Modify: `src/shots.rs` (a scene)
- Modify: `src/ui/complete_tests.rs`

**Interfaces:**
- Consumes: `Completion`, `Candidate`, `Kind`, `AcceptCompletion`, `CloseCompletion`, `Edited` (galley, origin, focused).
- Produces: `sql_complete::{show, Shown, place, window, row_name, VISIBLE}`, `TextRole::{CompletionName, CompletionMatch}`.

The measures below are the artboards' (macOS and Omarchy "SQL editor"). Compare the result with them by hand; no test checks a measure.

- [ ] **Step 1: Write the failing tests**

Create `src/ui/sql_complete.rs` holding only this test module:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_list_goes_under_its_word_or_above_it_when_there_is_no_room() {
        let screen = Rect::from_min_size(Pos2::ZERO, vec2(800.0, 600.0));
        let size = vec2(330.0, 160.0);
        let word = |x: f32, y: f32| Rect::from_min_max(pos2(x, y), pos2(x, y + 22.0));
        // Room below: just under the line.
        assert_eq!(place(word(100.0, 200.0), size, screen), pos2(100.0, 224.0));
        // No room below: just above the line.
        assert_eq!(place(word(100.0, 500.0), size, screen), pos2(100.0, 338.0));
        // Too far right: pushed back inside.
        assert_eq!(place(word(700.0, 200.0), size, screen).x, 470.0);
        // No room either way: below, as far as it goes.
        let small = Rect::from_min_size(Pos2::ZERO, vec2(800.0, 200.0));
        assert_eq!(place(word(100.0, 90.0), size, small).y, 114.0);
    }

    #[test]
    fn the_rows_in_view_follow_the_highlight() {
        // Fewer rows than fit: all of them.
        assert_eq!(window(0, 2, 3), 0);
        // Moving down past the last row in view scrolls by one.
        assert_eq!(window(0, 4, 20), 0);
        assert_eq!(window(0, 5, 20), 1);
        // Moving back up inside the view scrolls nothing.
        assert_eq!(window(1, 3, 20), 1);
        // Above the view: the highlight is the first row.
        assert_eq!(window(6, 2, 20), 2);
        // The list shrank under the view.
        assert_eq!(window(15, 0, 4), 0);
        assert_eq!(window(15, 7, 8), 3);
    }

    #[test]
    fn a_row_is_named_by_its_label_kind_and_type() {
        let locale = Locale::English;
        let row = |kind, detail: &str| Candidate {
            kind,
            label: "title".into(),
            insert: "title".into(),
            matched: 0..0,
            detail: detail.into(),
        };
        assert_eq!(row_name(&row(Kind::Table, ""), locale), "title, table");
        assert_eq!(row_name(&row(Kind::Column, "text"), locale), "title, column, text");
        assert_eq!(
            row_name(&row(Kind::MaterializedView, ""), locale),
            "title, materialized view"
        );
    }
}
```

Append to `src/ui/complete_tests.rs`:

```rust
use egui::accesskit::Role;

/// The node labelled `label`, whatever its role.
fn named<'a>(
    tree: &'a egui::accesskit::TreeUpdate,
    label: &str,
) -> Option<&'a (egui::accesskit::NodeId, egui::accesskit::Node)> {
    tree.nodes.iter().find(|(_, node)| node.label() == Some(label))
}

/// Where the node labelled `label` sits, in points.
fn bounds_of(tree: &egui::accesskit::TreeUpdate, label: &str) -> egui::Rect {
    let (_, node) = named(tree, label).unwrap_or_else(|| panic!("nothing labelled {label:?}"));
    let rect = node.bounds().expect("bounds");
    egui::Rect::from_min_max(
        egui::pos2(rect.x0 as f32, rect.y0 as f32),
        egui::pos2(rect.x1 as f32, rect.y1 as f32),
    )
}

fn painted(harness: &Harness, part: &str) -> usize {
    harness.painted.iter().filter(|(text, _)| text.contains(part)).count()
}

#[test]
fn the_list_and_its_rows_are_named_for_screen_readers() {
    let (mut harness, _tab) = editor();
    type_text(&mut harness, "se");
    let tree = harness.settle();
    assert!(named(&tree, "Completions").is_some());
    assert!(crate::testing::node(&tree, "select, keyword", Role::Button).is_some());
    assert!(crate::testing::node(&tree, "set, keyword", Role::Button).is_some());
}

#[test]
fn the_editor_names_the_highlighted_row() {
    let (mut harness, _tab) = editor();
    type_text(&mut harness, "se");
    let active = |tree: &egui::accesskit::TreeUpdate| {
        let (_, editor) = named(tree, "SQL").expect("the editor");
        editor.active_descendant()
    };
    let tree = harness.settle();
    let first = crate::testing::node(&tree, "select, keyword", Role::Button);
    assert!(first.is_some());
    assert_eq!(active(&tree), first);
    harness.press(Key::ArrowDown, Modifiers::NONE);
    let tree = harness.settle();
    assert_eq!(active(&tree), crate::testing::node(&tree, "set, keyword", Role::Button));
    // Closed: the editor names no row.
    harness.press(Key::Escape, Modifiers::NONE);
    let tree = harness.settle();
    assert_eq!(active(&tree), None);
}

#[test]
fn a_click_on_a_row_inserts_it_and_the_editor_keeps_the_keyboard() {
    let (mut harness, tab) = editor();
    type_text(&mut harness, "se");
    let tree = harness.settle();
    let at = bounds_of(&tree, "set, keyword").center();
    let button = |pressed| egui::Event::PointerButton {
        pos: at,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: Modifiers::NONE,
    };
    harness.frame(vec![egui::Event::PointerMoved(at)]);
    harness.frame(vec![button(true)]);
    harness.frame(vec![button(false)]);
    harness.settle();
    assert_eq!(sql(&harness, tab).text, "set");
    assert!(list(&harness, tab).is_none());
    assert!(editor_has_keyboard(&harness, tab));
}

#[test]
fn the_footer_names_the_keys_and_counts_the_rows_out_of_sight() {
    let (mut harness, tab) = editor();
    harness.press(Key::Space, ctrl());
    let open = list(&harness, tab).expect("a list");
    let hidden = open.candidates.len() + open.more - crate::ui::sql_complete::VISIBLE;
    assert!(hidden > 0);
    harness.settle();
    assert_eq!(painted(&harness, &format!("{hidden} more")), 1);
    assert_eq!(painted(&harness, "insert"), 1);
    // Five rows show, and no sixth.
    let tree = harness.settle();
    let rows = tree
        .nodes
        .iter()
        .filter(|(_, node)| node.label().is_some_and(|label| label.ends_with(", keyword")))
        .count();
    assert_eq!(rows, crate::ui::sql_complete::VISIBLE);
}

#[test]
fn the_terminal_look_names_its_keys_in_the_list_and_the_mode_line() {
    let (mut harness, _tab) = editor();
    harness.set_look(crate::theme::Look::omarchy());
    harness.settle();
    assert_eq!(painted(&harness, "complete"), 0);
    type_text(&mut harness, "se");
    harness.settle();
    assert_eq!(painted(&harness, "ctrl+n/p"), 1);
    // The list's footer and the mode line's `tab complete`.
    assert_eq!(painted(&harness, "complete"), 2);
}

#[test]
fn losing_the_keyboard_closes_the_list() {
    let (mut harness, tab) = editor();
    type_text(&mut harness, "se");
    // Quick open takes the keyboard.
    harness.press(Key::P, Modifiers::COMMAND);
    assert!(harness.app.dialog.is_some());
    assert!(list(&harness, tab).is_none());
}

#[test]
fn scrolling_the_word_out_of_the_pane_closes_the_list() {
    let (mut harness, tab) = editor();
    paste(&mut harness, &"select 1;\n".repeat(200));
    type_text(&mut harness, "se");
    assert!(list(&harness, tab).is_some());
    // The wheel over the editor, far enough for the last line to leave.
    let tree = harness.settle();
    let over = bounds_of(&tree, "SQL").center();
    harness.frame(vec![egui::Event::PointerMoved(over)]);
    for _ in 0..10 {
        harness.frame(vec![egui::Event::MouseWheel {
            unit: egui::MouseWheelUnit::Line,
            delta: egui::vec2(0.0, 20.0),
            phase: egui::TouchPhase::Move,
            modifiers: Modifiers::NONE,
        }]);
    }
    harness.settle();
    assert!(list(&harness, tab).is_none());
    assert!(editor_has_keyboard(&harness, tab));
}
```

Check `egui::Event::MouseWheel`'s fields in the fork (`crates/egui/src/data/input.rs` or `data/input/`) and write the event as it is declared there.

In `src/ui/keys.rs`, add `"Complete in the SQL editor",` to the list `the_shortcut_table_covers_the_spec_map` expects.

In `src/typography/fonts.rs`, add `⇥` to the string the glyph test checks (`"⌘⇧⌫↩"` becomes `"⌘⇧⌫↩⇥"`).

- [ ] **Step 2: Run to see them fail**

Add `pub mod sql_complete;` to `src/ui/mod.rs`.
Run: `~/.cargo/bin/cargo test --locked --lib -- sql_complete complete_tests`
Expected: FAIL to compile (`place`, `window`, `row_name`, `VISIBLE` missing).

- [ ] **Step 3: The roles**

In `src/typography.rs`, add to `TextRole` after `Code`:

```rust
    /// A name in the SQL editor's completion list.
    CompletionName,
    /// The part of that name that matches what was typed.
    CompletionMatch,
```

Add both to `TextRole::ALL` after `Self::Code` (its length becomes 40), to `id` (`"completion-name"`, `"completion-match"`) and to `spec`:

```rust
            Self::CompletionName => style(Mono, 400, 12.5),
            // Plex Mono is bundled at 400 and 500: the heaviest there is.
            Self::CompletionMatch => style(Mono, 500, 12.5),
```

Run `~/.cargo/bin/cargo test --locked --lib typography`. A test that lists every role, or checks them against the typography table, needs the two new entries: add them there as the test asks.

If the glyph test fails for `⇥`, take it out of that test again and write the macOS footer as `↩ Tab` in Step 4 (`format!("↩ Tab {insert}")`); say so in the commit body.

- [ ] **Step 4: The view**

Put above the tests in `src/ui/sql_complete.rs`:

```rust
//! The SQL editor's completion list: a small panel under the word being
//! typed. It never takes the keyboard: the editor keeps it, and `ui::keys`
//! routes the list's keys.

use std::borrow::Cow;

use egui::{
    Color32, CornerRadius, Id, Pos2, Rect, Sense, Stroke, StrokeKind, Ui, WidgetInfo,
    WidgetType, pos2, vec2,
};

use crate::completion::{Candidate, Kind};
use crate::i18n::{Locale, gettext};
use crate::model::Completion;
use crate::theme::{Look, Palette};
use crate::typography::{Text, TextRole};
use crate::ui::widgets;

/// How many rows show at once; the list follows the highlight.
pub const VISIBLE: usize = 5;

/// The list's measures in a look.
struct Shape {
    width: f32,
    /// The padding round the rows: across, then down.
    pad: egui::Vec2,
    row: f32,
    /// A row's own padding at each end.
    inset: f32,
    /// The kind letter's column and the gap after it; none in the terminal.
    letter: Option<(f32, f32)>,
    corner: u8,
    row_corner: u8,
    /// From the last row to the footer's hairline.
    footer_gap: f32,
    /// The footer's height under the hairline.
    footer: f32,
}

impl Shape {
    /// macOS: 330 pt wide, 5 pt of padding, 26 pt rows with a 20 pt letter
    /// column, corners of 8 and 5. The terminal: 360 pt wide, 4 pt above
    /// and below, 24 pt rows across the whole list, corners of 3.
    fn of(look: &Look) -> Self {
        if look.terminal {
            Self {
                width: 360.0,
                pad: vec2(0.0, 4.0),
                row: 24.0,
                inset: 10.0,
                letter: None,
                corner: 3,
                row_corner: 0,
                footer_gap: 3.0,
                footer: 22.0,
            }
        } else {
            Self {
                width: 330.0,
                pad: vec2(5.0, 5.0),
                row: 26.0,
                inset: 10.0,
                letter: Some((20.0, 8.0)),
                corner: 8,
                row_corner: 5,
                footer_gap: 4.0,
                footer: 24.0,
            }
        }
    }

    fn size(&self, rows: usize) -> egui::Vec2 {
        let height = 2.0 * self.pad.y + rows as f32 * self.row + self.footer_gap + self.footer;
        vec2(self.width, height)
    }
}

/// Where a list of `size` goes: under `anchor` (the start of its word, as
/// tall as the line), above it when only there is room, and inside
/// `screen` across.
pub fn place(anchor: Rect, size: egui::Vec2, screen: Rect) -> Pos2 {
    let gap = 2.0;
    let below = anchor.bottom() + gap;
    let above = anchor.top() - gap - size.y;
    let top = if below + size.y > screen.bottom() && above >= screen.top() {
        above
    } else {
        below
    };
    let left = anchor.left().min(screen.right() - size.x).max(screen.left());
    pos2(left, top)
}

/// The first row in view: the view that started at `first`, moved as
/// little as it takes to show `selected` among `rows` rows.
pub fn window(first: usize, selected: usize, rows: usize) -> usize {
    first
        .min(selected)
        .max((selected + 1).saturating_sub(VISIBLE))
        .min(rows.saturating_sub(VISIBLE))
}

/// A kind in words: what a row shows at its right, and how it is named.
fn kind_name(kind: Kind, locale: Locale) -> Cow<'static, str> {
    match kind {
        Kind::Keyword => gettext(locale, "keyword"),
        Kind::Schema => gettext(locale, "schema"),
        Kind::Table => gettext(locale, "table"),
        Kind::View => gettext(locale, "view"),
        Kind::MaterializedView => gettext(locale, "materialized view"),
        Kind::Column => gettext(locale, "column"),
    }
}

/// The letter macOS shows before a row.
fn kind_letter(kind: Kind) -> &'static str {
    match kind {
        Kind::Keyword => "K",
        Kind::Schema => "S",
        Kind::Table => "T",
        Kind::View | Kind::MaterializedView => "V",
        Kind::Column => "C",
    }
}

/// What a row is called for a screen reader: "books, table", and a column
/// with its type, "title, column, text".
pub fn row_name(candidate: &Candidate, locale: Locale) -> String {
    let kind = kind_name(candidate.kind, locale);
    if candidate.detail.is_empty() {
        format!("{}, {kind}", candidate.label)
    } else {
        format!("{}, {kind}, {}", candidate.label, candidate.detail)
    }
}

/// What the pointer did to the list this frame.
pub struct Shown {
    /// A row is pressed or was clicked: the press landed outside the
    /// editor's field, which must keep the keyboard.
    pub pressed: bool,
    /// The row clicked.
    pub picked: Option<usize>,
}

/// Draws `list` for the editor `editor`, at `anchor`: the start of its
/// word on screen, as tall as the line.
pub fn show(
    ui: &Ui,
    list: &Completion,
    anchor: Rect,
    editor: Id,
    look: &Look,
    palette: &Palette,
    locale: Locale,
) -> Shown {
    let shape = Shape::of(look);
    let total = list.candidates.len();
    let rows = total.min(VISIBLE);
    let size = shape.size(rows);
    let id = list_id(editor);
    let first = window(ui.data(|data| data.get_temp(id)).unwrap_or(0), list.selected, total);
    ui.data_mut(|data| data.insert_temp(id, first));
    let mut shown = Shown {
        pressed: false,
        picked: None,
    };
    egui::Area::new(id)
        .order(egui::Order::Foreground)
        // Whole at once: a list that fades in lags the typing.
        .fade_in(false)
        .fixed_pos(place(anchor, size, ui.ctx().content_rect()))
        .show(ui.ctx(), |ui| {
            let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
            let name = gettext(locale, "Completions");
            ui.interact(rect, id.with("list"), Sense::hover())
                .widget_info(|| WidgetInfo::labeled(WidgetType::Other, true, name.as_ref()));
            panel(ui, rect, &shape, look, palette);
            let inner = rect.shrink2(shape.pad);
            for (slot, index) in (first..first + rows).enumerate() {
                let candidate = &list.candidates[index];
                let top = inner.top() + slot as f32 * shape.row;
                let row = Rect::from_min_size(pos2(inner.left(), top), vec2(inner.width(), shape.row));
                let row_id = id.with(("row", index));
                let response = ui.interact(row, row_id, Sense::click());
                let selected = index == list.selected;
                let name = row_name(candidate, locale);
                response.widget_info(|| {
                    WidgetInfo::selected(WidgetType::SelectableLabel, true, selected, &name)
                });
                let corner = CornerRadius::same(shape.row_corner);
                if selected {
                    ui.painter().rect_filled(row, corner, palette.selection);
                    // The keyboard stays in the editor: its node says
                    // which row is the current one.
                    ui.ctx().accesskit_node_builder(editor, |node| {
                        node.set_active_descendant(row_id.accesskit_id());
                    });
                } else if response.hovered() {
                    ui.painter().rect_filled(row, corner, palette.surface_hover);
                }
                paint_row(ui, row, candidate, &shape, look, palette, locale);
                shown.pressed |= response.is_pointer_button_down_on() || response.clicked();
                if response.clicked() {
                    shown.picked = Some(index);
                }
            }
            footer(ui, rect, rows, list, &shape, look, palette, locale);
        });
    shown
}

/// The id the list of the editor `editor` is kept under: its area, and
/// the first row in view.
pub fn list_id(editor: Id) -> Id {
    editor.with("completions")
}

/// The panel behind the rows: raised and rounded on macOS, the terminal's
/// darker surface with an accent border.
fn panel(ui: &Ui, rect: Rect, shape: &Shape, look: &Look, palette: &Palette) {
    let corner = CornerRadius::same(shape.corner);
    let (shadow, fill, border) = if look.terminal {
        let shadow = egui::epaint::Shadow {
            offset: [0, 12],
            blur: 32,
            spread: 0,
            color: Color32::from_black_alpha(115),
        };
        (shadow, palette.panel, palette.accent)
    } else {
        let shadow = egui::epaint::Shadow {
            offset: [0, 8],
            blur: 24,
            spread: 0,
            color: palette.shadow,
        };
        (shadow, widgets::raised_fill(palette), palette.border)
    };
    let painter = ui.painter();
    painter.add(shadow.as_shape(rect, corner));
    painter.rect_filled(rect, corner, fill);
    painter.rect_stroke(rect, corner, Stroke::new(1.0, border), StrokeKind::Inside);
}

/// The muted colour of a kind, a type and the footer.
fn muted(look: &Look, palette: &Palette) -> Color32 {
    if look.terminal {
        palette.dim
    } else {
        palette.secondary
    }
}

/// One row: on macOS the kind's letter, then the name with what matched
/// emphasised (heavier on macOS, in the accent in the terminal), and at
/// the right the kind in words or a column's type.
fn paint_row(
    ui: &Ui,
    row: Rect,
    candidate: &Candidate,
    shape: &Shape,
    look: &Look,
    palette: &Palette,
    locale: Locale,
) {
    let y = row.center().y;
    let muted = muted(look, palette);
    let mut x = row.left() + shape.inset;
    if let Some((column, gap)) = shape.letter {
        Text::one(look, TextRole::TagSmall, kind_letter(candidate.kind), muted)
            .layout(ui.ctx())
            .paint_left(ui.painter(), x, y);
        x += column + gap;
    }
    let detail = if candidate.detail.is_empty() {
        kind_name(candidate.kind, locale)
    } else {
        Cow::Borrowed(candidate.detail.as_str())
    };
    let detail_role = TextRole::pick(look, TextRole::ColumnType, TextRole::OCaption);
    let detail = Text::one(look, detail_role, &detail, muted).layout(ui.ctx());
    let right = row.right() - shape.inset;
    detail.paint_right(ui.painter(), right, y);
    let (plain, hit, hit_color) = if look.terminal {
        (TextRole::OCode, TextRole::OCode, palette.accent)
    } else {
        (TextRole::CompletionName, TextRole::CompletionMatch, palette.text)
    };
    let label = candidate.label.as_str();
    let matched = candidate.matched.clone();
    // A range that does not fit the label is drawn as no match.
    let (before, found, after) = match (
        label.get(..matched.start),
        label.get(matched.clone()),
        label.get(matched.end..),
    ) {
        (Some(before), Some(found), Some(after)) => (before, found, after),
        _ => (label, "", ""),
    };
    let name = Text::new(look)
        .add(plain, before, palette.text)
        .add(hit, found, hit_color)
        .add(plain, after, palette.text)
        .layout(ui.ctx());
    // A long name stops before the kind, not under it.
    let room = Rect::from_min_max(
        pos2(x, row.top()),
        pos2(right - detail.width() - 8.0, row.bottom()),
    );
    name.paint_left(&ui.painter().with_clip_rect(room), x, y);
}

/// Under a hairline: the keys, and how many matches are out of sight, or
/// that names are on their way.
#[expect(clippy::too_many_arguments, reason = "one call, drawing with all of it")]
fn footer(
    ui: &Ui,
    rect: Rect,
    rows: usize,
    list: &Completion,
    shape: &Shape,
    look: &Look,
    palette: &Palette,
    locale: Locale,
) {
    let top = rect.top() + shape.pad.y + rows as f32 * shape.row + shape.footer_gap;
    let across = egui::Rangef::new(rect.left() + shape.pad.x, rect.right() - shape.pad.x);
    let line = if look.terminal {
        palette.outline
    } else {
        palette.border
    };
    widgets::hline(ui, across, top, line);
    let y = top + shape.footer / 2.0;
    let muted = muted(look, palette);
    let hidden = (list.candidates.len() + list.more).saturating_sub(rows);
    let count = if list.loading {
        Some(gettext(locale, "Loading").into_owned())
    } else if hidden > 0 {
        Some(gettext(locale, "{count} more").replace("{count}", &hidden.to_string()))
    } else {
        None
    };
    let left = rect.left() + shape.pad.x + shape.inset;
    let right = rect.right() - shape.pad.x - shape.inset;
    if look.terminal {
        let role = TextRole::OCaption;
        let complete = format!(" {} · ", gettext(locale, "complete"));
        let moves = format!(" {}", gettext(locale, "move"));
        let mut text = Text::new(look)
            .add(role, "tab", palette.text)
            .add(role, &complete, muted)
            .add(role, "ctrl+n/p", palette.text)
            .add(role, &moves, muted);
        if let Some(count) = &count {
            text = text.add(role, &format!(" · {}", count.to_lowercase()), muted);
        }
        text.layout(ui.ctx()).paint_left(ui.painter(), left, y);
    } else {
        let role = TextRole::Shortcut;
        let keys = format!("↩ ⇥ {}", gettext(locale, "insert"));
        Text::one(look, role, &keys, muted)
            .layout(ui.ctx())
            .paint_left(ui.painter(), left, y);
        if let Some(count) = &count {
            widgets::paint_text_right(ui, right, y, Text::one(look, role, count, muted));
        }
    }
}
```

If a helper already formats counted strings in this crate (see how `src/ui/sql_results.rs` builds "First 1,000 rows"), use it for "{count} more" rather than `replace`. If `clippy::too_many_arguments` does not fire for `footer`, drop the `expect`.

- [ ] **Step 5: Place the list from the editor**

In `src/ui/sql_text.rs`, in `show`, after the gutter's press is handled (the `edited.focused = true` block) and while `sql_tab` is still in use, read the open list and where its word starts:

```rust
    // The open completion list, and the character its word starts at.
    // (After the field drew: an accepted list was taken off the tab there.)
    let completing = sql_tab.completion.clone().map(|list| {
        let start = char_index(&sql_tab.text, list.site.word.start);
        (list, start)
    });
```

After the `SqlTyped` push (where `app.actions` is free again):

```rust
    if let Some((list, start)) = completing {
        // The start of the word on screen, as tall as its line.
        let word = edited
            .galley
            .pos_from_cursor(CCursor::new(start))
            .translate(edited.origin.to_vec2());
        // Scrolled out of the pane, the list has nothing to hang under.
        let in_sight = text_pane.contains(word.center());
        if in_sight {
            let shown = super::sql_complete::show(ui, &list, word, editor, &look, &palette, locale);
            if shown.pressed {
                // A press on a row lands outside the field, which gives
                // the keys up: they stay the editor's.
                ui.memory_mut(|memory| memory.request_focus(editor));
                edited.focused = true;
            }
            if let Some(row) = shown.picked {
                app.actions.push(Action::AcceptCompletion {
                    tab,
                    sql_tab: id,
                    row: Some(row),
                });
            }
        }
        // Checked after the list drew, so a press on a row does not count
        // as the editor losing the keyboard.
        if !in_sight || !edited.focused {
            app.actions.push(Action::CloseCompletion { tab, sql_tab: id });
        }
    }
```

In `forget`, also drop the first row in view the list kept:

```rust
        data.remove::<usize>(super::sql_complete::list_id(editor));
```

and add `("list", ...)` to `remembered` only if a test of `forget` fails without it.

- [ ] **Step 6: The mode line, the help entry and a screenshot scene**

In `src/ui/workspace.rs`, in `status_line`, where `workspace` is in scope:

```rust
    // The completion list's own key, while it is open.
    let completing = workspace
        .active_sql_tab()
        .is_some_and(|sql| sql.completion.is_some());
```

and build the editor's hints so `tab complete` leads while it is:

```rust
            let words = ["run", "run all", "cancel", "leave editor", "tables", "complete"]
                .map(|word| gettext(locale, word));
            let mut editor_hints: Vec<widgets::Hint<'_>> = vec![
                ("ctrl+enter", &*words[0], true),
                ("ctrl+shift+enter", &*words[1], true),
                ("ctrl+.", &*words[2], true),
                ("esc", &*words[3], true),
                ("ctrl+b", &*words[4], true),
            ];
            if completing {
                editor_hints.insert(0, ("tab", &*words[5], true));
            }
            let hints: &[widgets::Hint<'_>] = if on_editor {
                &editor_hints
            } else {
                &table_hints
            };
```

In `src/ui/keys.rs`, add to `SHORTCUTS` after the `Mod+Return` entry:

```rust
    ("Ctrl+Space", "Complete in the SQL editor"),
```

In `src/shots.rs`, after the `sql-idle` scene:

```rust
    both("sql-complete", |harness| {
        sql_editor(harness);
        // The editor has the keyboard a frame after it opens.
        harness.settle();
        // The script ends without a `;`: close its last statement, so the
        // word typed next starts a new one.
        harness.frame(vec![egui::Event::Paste(";\n".into())]);
        harness.frame(vec![egui::Event::Text("se".into())]);
    });
```

- [ ] **Step 7: Run to see them pass**

Run: `~/.cargo/bin/cargo test --locked --lib -- sql_complete complete_tests`, then `~/.cargo/bin/cargo test --locked --lib ui::` and `~/.cargo/bin/cargo check --locked --features shots --tests`
Expected: PASS.

Notes for failures:
- `the_terminal_look_names_its_keys_in_the_list_and_the_mode_line`: if `widgets::key_hints` paints a key and its word as two pieces of text, the count of "complete" is still two (the footer and the hint's word); if it paints all hints as one piece, it is two as well. A count of one means the mode line does not show the hint.
- `scrolling_the_word_out_of_the_pane_closes_the_list`: if the wheel does not scroll the editor in the harness, scroll it through `egui::scroll_area::State` (the editor keeps its area's id as `ScrollId`, see `forget`), and keep the assertion.
- `a_click_on_a_row_inserts_it_and_the_editor_keeps_the_keyboard`: if the editor lost the keyboard, the `shown.pressed` branch does not run on the frame egui counts the press as elsewhere: compare with the gutter's press in the same function.

- [ ] **Step 8: Look at it**

Render the scene in both looks and compare with the artboards by hand (panel, rows, emphasis of the typed part, footer):

```bash
~/.cargo/bin/cargo test --locked --features shots --lib shots -- --ignored sql
```

The PNGs stay local: do not commit or attach them.

- [ ] **Step 9: Run the checks and commit**

```bash
~/.cargo/bin/cargo fmt --all --check && ~/.cargo/bin/cargo clippy --locked --workspace --all-targets -- -D warnings && ~/.cargo/bin/cargo test --locked --workspace --all-targets && RUSTDOCFLAGS='-D warnings' ~/.cargo/bin/cargo doc --locked --workspace --no-deps && ~/.cargo/bin/cargo check --locked --features shots --tests
git add src/ui/sql_complete.rs src/ui/mod.rs src/ui/sql_text.rs src/typography.rs src/typography/fonts.rs src/ui/workspace.rs src/ui/keys.rs src/shots.rs src/ui/complete_tests.rs
git commit -m "Draw the completion list under the word being typed

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

Step 1 of the spec (the list and keywords) is complete here.

---

### Task 7: Names bare when safe, quoted otherwise

**Files:**
- Create: `crates/tabletist-db/src/reserved.rs`
- Modify: `crates/tabletist-db/src/lib.rs` (add `mod reserved;`)
- Modify: `crates/tabletist-db/src/dialect.rs` (`Dialect::ident`, tests)

**Interfaces:**
- Produces: `Dialect::ident(self, name: &str) -> Cow<'_, str>`.

- [ ] **Step 1: Write the failing tests** (append to the tests of `crates/tabletist-db/src/dialect.rs`)

```rust
    #[test]
    fn a_plain_name_is_inserted_bare_and_any_other_quoted() {
        let pg = Dialect::Postgres;
        assert_eq!(pg.ident("books"), "books");
        assert_eq!(pg.ident("book_reviews2"), "book_reviews2");
        assert_eq!(pg.ident("_private"), "_private");
        // PostgreSQL folds a bare name to lower case.
        assert_eq!(pg.ident("Books"), "\"Books\"");
        assert_eq!(pg.ident("BOOKS"), "\"BOOKS\"");
        for dialect in [Dialect::MySql, Dialect::Sqlite] {
            assert_eq!(dialect.ident("Books"), "Books", "{dialect:?}");
        }
        // Not a plain word.
        assert_eq!(pg.ident("Order Items"), "\"Order Items\"");
        assert_eq!(pg.ident("2fa"), "\"2fa\"");
        assert_eq!(pg.ident("a\"b"), "\"a\"\"b\"");
        assert_eq!(pg.ident("żółw"), "\"żółw\"");
        assert_eq!(pg.ident(""), "\"\"");
        assert_eq!(Dialect::MySql.ident("order-items"), "`order-items`");
        assert_eq!(Dialect::MySql.ident("a`b"), "`a``b`");
        assert_eq!(Dialect::Sqlite.ident("order items"), "\"order items\"");
    }

    #[test]
    fn a_reserved_word_is_quoted() {
        for word in ["user", "order", "group", "table", "select", "end", "primary"] {
            assert_eq!(Dialect::Postgres.ident(word), format!("\"{word}\""), "{word}");
        }
        for word in ["order", "group", "key", "keys", "index", "rank", "SELECT"] {
            assert_eq!(Dialect::MySql.ident(word), format!("`{word}`"), "{word}");
        }
        for word in ["order", "group", "key", "index", "transaction", "Values"] {
            assert_eq!(Dialect::Sqlite.ident(word), format!("\"{word}\""), "{word}");
        }
        // Reserved elsewhere, not here.
        assert_eq!(Dialect::Postgres.ident("key"), "key");
        assert_eq!(Dialect::MySql.ident("user"), "user");
    }
```

- [ ] **Step 2: Run to see them fail**

Run: `~/.cargo/bin/cargo test --locked -p tabletist-db --lib dialect::`
Expected: FAIL to compile (no method `ident`).

- [ ] **Step 3: The reserved words**

Create `crates/tabletist-db/src/reserved.rs` and add `mod reserved;` to `lib.rs`. The lists below were written from memory of the manuals: check each against its page and correct it before committing (a word too many costs a pair of quotes; a word missing makes an insertion that does not parse).

- PostgreSQL: <https://www.postgresql.org/docs/current/sql-keywords-appendix.html>, the words marked "reserved" or "reserved (can be function or type)" in the PostgreSQL column.
- MySQL: <https://dev.mysql.com/doc/refman/8.4/en/keywords.html>, the words marked (R); MariaDB's additions: <https://mariadb.com/kb/en/reserved-words/>.
- SQLite: <https://www.sqlite.org/lang_keywords.html>, the whole list.

If the pages cannot be reached, keep the lists as written and say in the report that they were not checked.

```rust
//! The words each dialect reserves: a name spelled like one is quoted
//! when the editor inserts it. Longer than the highlighting keywords in
//! `sql`, which stay short.

use crate::Dialect;

/// PostgreSQL's reserved key words, and the ones reserved except as a
/// function or type name.
const POSTGRES: &[&str] = &[
    "ALL", "ANALYSE", "ANALYZE", "AND", "ANY", "ARRAY", "AS", "ASC", "ASYMMETRIC",
    "AUTHORIZATION", "BINARY", "BOTH", "CASE", "CAST", "CHECK", "COLLATE", "COLLATION",
    "COLUMN", "CONCURRENTLY", "CONSTRAINT", "CREATE", "CROSS", "CURRENT_CATALOG",
    "CURRENT_DATE", "CURRENT_ROLE", "CURRENT_SCHEMA", "CURRENT_TIME", "CURRENT_TIMESTAMP",
    "CURRENT_USER", "DEFAULT", "DEFERRABLE", "DESC", "DISTINCT", "DO", "ELSE", "END",
    "EXCEPT", "FALSE", "FETCH", "FOR", "FOREIGN", "FREEZE", "FROM", "FULL", "GRANT",
    "GROUP", "HAVING", "ILIKE", "IN", "INITIALLY", "INNER", "INTERSECT", "INTO", "IS",
    "ISNULL", "JOIN", "LATERAL", "LEADING", "LEFT", "LIKE", "LIMIT", "LOCALTIME",
    "LOCALTIMESTAMP", "NATURAL", "NOT", "NOTNULL", "NULL", "OFFSET", "ON", "ONLY", "OR",
    "ORDER", "OUTER", "OVERLAPS", "PLACING", "PRIMARY", "REFERENCES", "RETURNING",
    "RIGHT", "SELECT", "SESSION_USER", "SIMILAR", "SOME", "SYMMETRIC", "SYSTEM_USER",
    "TABLE", "TABLESAMPLE", "THEN", "TO", "TRAILING", "TRUE", "UNION", "UNIQUE", "USER",
    "USING", "VARIADIC", "VERBOSE", "WHEN", "WHERE", "WINDOW", "WITH",
];

/// MySQL 8.4's reserved words, with the ones MariaDB adds.
const MYSQL: &[&str] = &[
    "ACCESSIBLE", "ADD", "ALL", "ALTER", "ANALYZE", "AND", "AS", "ASC", "ASENSITIVE",
    "BEFORE", "BETWEEN", "BIGINT", "BINARY", "BLOB", "BOTH", "BY", "CALL", "CASCADE",
    "CASE", "CHANGE", "CHAR", "CHARACTER", "CHECK", "COLLATE", "COLUMN", "CONDITION",
    "CONSTRAINT", "CONTINUE", "CONVERT", "CREATE", "CROSS", "CUBE", "CUME_DIST",
    "CURRENT_DATE", "CURRENT_ROLE", "CURRENT_TIME", "CURRENT_TIMESTAMP", "CURRENT_USER",
    "CURSOR", "DATABASE", "DATABASES", "DAY_HOUR", "DAY_MICROSECOND", "DAY_MINUTE",
    "DAY_SECOND", "DEC", "DECIMAL", "DECLARE", "DEFAULT", "DELAYED", "DELETE",
    "DENSE_RANK", "DESC", "DESCRIBE", "DETERMINISTIC", "DISTINCT", "DISTINCTROW", "DIV",
    "DOUBLE", "DROP", "DUAL", "EACH", "ELSE", "ELSEIF", "EMPTY", "ENCLOSED", "ESCAPED",
    "EXCEPT", "EXISTS", "EXIT", "EXPLAIN", "FALSE", "FETCH", "FIRST_VALUE", "FLOAT",
    "FLOAT4", "FLOAT8", "FOR", "FORCE", "FOREIGN", "FROM", "FULLTEXT", "FUNCTION",
    "GENERATED", "GET", "GRANT", "GROUP", "GROUPING", "GROUPS", "HAVING",
    "HIGH_PRIORITY", "HOUR_MICROSECOND", "HOUR_MINUTE", "HOUR_SECOND", "IF", "IGNORE",
    "IN", "INDEX", "INFILE", "INNER", "INOUT", "INSENSITIVE", "INSERT", "INT", "INT1",
    "INT2", "INT3", "INT4", "INT8", "INTEGER", "INTERSECT", "INTERVAL", "INTO",
    "IO_AFTER_GTIDS", "IO_BEFORE_GTIDS", "IS", "ITERATE", "JOIN", "JSON_TABLE", "KEY",
    "KEYS", "KILL", "LAG", "LAST_VALUE", "LATERAL", "LEAD", "LEADING", "LEAVE", "LEFT",
    "LIKE", "LIMIT", "LINEAR", "LINES", "LOAD", "LOCALTIME", "LOCALTIMESTAMP", "LOCK",
    "LONG", "LONGBLOB", "LONGTEXT", "LOOP", "LOW_PRIORITY", "MANUAL", "MATCH",
    "MAXVALUE", "MEDIUMBLOB", "MEDIUMINT", "MEDIUMTEXT", "MIDDLEINT",
    "MINUTE_MICROSECOND", "MINUTE_SECOND", "MOD", "MODIFIES", "NATURAL", "NOT",
    "NO_WRITE_TO_BINLOG", "NTH_VALUE", "NTILE", "NULL", "NUMERIC", "OF", "OFFSET", "ON",
    "OPTIMIZE", "OPTIMIZER_COSTS", "OPTION", "OPTIONALLY", "OR", "ORDER", "OUT", "OUTER",
    "OUTFILE", "OVER", "PAGE_CHECKSUM", "PARALLEL", "PARSE_VCOL_EXPR", "PARTITION",
    "PERCENT_RANK", "PRECISION", "PRIMARY", "PROCEDURE", "PURGE", "QUALIFY", "RANGE",
    "RANK", "READ", "READS", "READ_WRITE", "REAL", "RECURSIVE", "REF_SYSTEM_ID",
    "REFERENCES", "REGEXP", "RELEASE", "RENAME", "REPEAT", "REPLACE", "REQUIRE",
    "RESIGNAL", "RESTRICT", "RETURN", "RETURNING", "REVOKE", "RIGHT", "RLIKE", "ROW",
    "ROWS", "ROW_NUMBER", "SCHEMA", "SCHEMAS", "SECOND_MICROSECOND", "SELECT",
    "SENSITIVE", "SEPARATOR", "SET", "SHOW", "SIGNAL", "SLOW", "SMALLINT", "SPATIAL",
    "SPECIFIC", "SQL", "SQLEXCEPTION", "SQLSTATE", "SQLWARNING", "SQL_BIG_RESULT",
    "SQL_CALC_FOUND_ROWS", "SQL_SMALL_RESULT", "SSL", "STARTING", "STATS_AUTO_RECALC",
    "STATS_PERSISTENT", "STATS_SAMPLE_PAGES", "STORED", "STRAIGHT_JOIN", "SYSTEM",
    "TABLE", "TABLESAMPLE", "TERMINATED", "THEN", "TINYBLOB", "TINYINT", "TINYTEXT",
    "TO", "TRAILING", "TRIGGER", "TRUE", "UNDO", "UNION", "UNIQUE", "UNLOCK", "UNSIGNED",
    "UPDATE", "USAGE", "USE", "USING", "UTC_DATE", "UTC_TIME", "UTC_TIMESTAMP", "VALUES",
    "VARBINARY", "VARCHAR", "VARCHARACTER", "VARYING", "VIRTUAL", "WHEN", "WHERE",
    "WHILE", "WINDOW", "WITH", "WRITE", "XOR", "YEAR_MONTH", "ZEROFILL",
];

/// Every SQLite keyword. Many may be used as names without quotes, but
/// which ones depends on the build, and quotes always work.
const SQLITE: &[&str] = &[
    "ABORT", "ACTION", "ADD", "AFTER", "ALL", "ALTER", "ALWAYS", "ANALYZE", "AND", "AS",
    "ASC", "ATTACH", "AUTOINCREMENT", "BEFORE", "BEGIN", "BETWEEN", "BY", "CASCADE",
    "CASE", "CAST", "CHECK", "COLLATE", "COLUMN", "COMMIT", "CONFLICT", "CONSTRAINT",
    "CREATE", "CROSS", "CURRENT", "CURRENT_DATE", "CURRENT_TIME", "CURRENT_TIMESTAMP",
    "DATABASE", "DEFAULT", "DEFERRABLE", "DEFERRED", "DELETE", "DESC", "DETACH",
    "DISTINCT", "DO", "DROP", "EACH", "ELSE", "END", "ESCAPE", "EXCEPT", "EXCLUDE",
    "EXCLUSIVE", "EXISTS", "EXPLAIN", "FAIL", "FILTER", "FIRST", "FOLLOWING", "FOR",
    "FOREIGN", "FROM", "FULL", "GENERATED", "GLOB", "GROUP", "GROUPS", "HAVING", "IF",
    "IGNORE", "IMMEDIATE", "IN", "INDEX", "INDEXED", "INITIALLY", "INNER", "INSERT",
    "INSTEAD", "INTERSECT", "INTO", "IS", "ISNULL", "JOIN", "KEY", "LAST", "LEFT",
    "LIKE", "LIMIT", "MATCH", "MATERIALIZED", "NATURAL", "NO", "NOT", "NOTHING",
    "NOTNULL", "NULL", "NULLS", "OF", "OFFSET", "ON", "OR", "ORDER", "OTHERS", "OUTER",
    "OVER", "PARTITION", "PLAN", "PRAGMA", "PRECEDING", "PRIMARY", "QUERY", "RAISE",
    "RANGE", "RECURSIVE", "REFERENCES", "REGEXP", "REINDEX", "RELEASE", "RENAME",
    "REPLACE", "RESTRICT", "RETURNING", "RIGHT", "ROLLBACK", "ROW", "ROWS", "SAVEPOINT",
    "SELECT", "SET", "TABLE", "TEMP", "TEMPORARY", "THEN", "TIES", "TO", "TRANSACTION",
    "TRIGGER", "UNBOUNDED", "UNION", "UNIQUE", "UPDATE", "USING", "VACUUM", "VALUES",
    "VIEW", "VIRTUAL", "WHEN", "WHERE", "WINDOW", "WITH", "WITHOUT",
];

/// Whether `name` (any case) is a word `dialect` reserves, or one the
/// editor highlights as a keyword there.
pub fn is_reserved(dialect: Dialect, name: &str) -> bool {
    let words = match dialect {
        Dialect::Postgres => POSTGRES,
        Dialect::MySql => MYSQL,
        Dialect::Sqlite => SQLITE,
    };
    crate::sql::is_keyword(dialect, name)
        || words.iter().any(|word| word.eq_ignore_ascii_case(name))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_lists_are_upper_case_words_without_repeats() {
        for words in [POSTGRES, MYSQL, SQLITE] {
            let mut seen = std::collections::HashSet::new();
            for word in words {
                assert!(
                    word.bytes().all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || b == b'_'),
                    "{word}"
                );
                assert!(seen.insert(word), "{word} twice");
            }
        }
    }
}
```

The second test's last two lines (`key` on PostgreSQL, `user` on MySQL) hold with these lists; if the manual says otherwise, follow the manual and change the test.

- [ ] **Step 4: `Dialect::ident`**

In `crates/tabletist-db/src/dialect.rs`, after `quote_ident`:

```rust
    /// `name` as it is written into a statement the user reads: bare when
    /// that is safe, quoted otherwise. Safe is a plain word the dialect
    /// does not reserve. PostgreSQL folds a bare name to lower case, so
    /// there a plain word is in lower case. The rule errs toward quoting:
    /// a quoted name always works.
    pub fn ident(self, name: &str) -> std::borrow::Cow<'_, str> {
        let word = |byte: u8| match self {
            Self::Postgres => byte.is_ascii_lowercase() || byte == b'_',
            Self::MySql | Self::Sqlite => byte.is_ascii_alphabetic() || byte == b'_',
        };
        let mut bytes = name.bytes();
        let plain = bytes.next().is_some_and(word)
            && bytes.all(|byte| word(byte) || byte.is_ascii_digit());
        if plain && !crate::reserved::is_reserved(self, name) {
            std::borrow::Cow::Borrowed(name)
        } else {
            std::borrow::Cow::Owned(self.quote_ident(name))
        }
    }
```

- [ ] **Step 5: Run to see them pass**

Run: `~/.cargo/bin/cargo test --locked -p tabletist-db --lib -- dialect:: reserved::`
Expected: PASS.

- [ ] **Step 6: Run the checks and commit**

```bash
~/.cargo/bin/cargo fmt --all --check && ~/.cargo/bin/cargo clippy --locked --workspace --all-targets -- -D warnings && ~/.cargo/bin/cargo test --locked --workspace --all-targets && RUSTDOCFLAGS='-D warnings' ~/.cargo/bin/cargo doc --locked --workspace --no-deps
git add crates/tabletist-db/src/reserved.rs crates/tabletist-db/src/dialect.rs crates/tabletist-db/src/lib.rs
git commit -m "Write a name bare when it is safe and quoted otherwise

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 8: Schemas, tables and views

**Files:**
- Modify: `src/completion.rs` (`Catalog`, `Need`, `needs`, `Target`, `list`, tests)
- Modify: `src/model.rs` (`Workspace::{bare_schema, catalog_changed, is_loading}`)
- Modify: `src/app.rs` (`refresh_completion`, `send_needs`, `catalog_of`, `load_objects`, the `Schemas` and `Objects` events, `refresh_tree`)
- Modify: `src/shots.rs` (the `sql-complete` scene)
- Modify: `src/ui/complete_tests.rs`

**Interfaces:**
- Consumes: `Dialect::ident` (Task 7), `Tree`, `SchemaNode`, `Fetch`.
- Produces: `Catalog<'a> { dialect, tree, bare, schemas }`, `Need::Objects(String)`, `completion::needs(site, catalog) -> Vec<Need>`, `Workspace::bare_schema() -> Option<&str>`, `Workspace::catalog_changed()`, `Workspace::is_loading(&Need) -> bool`, `App::send_needs`.

- [ ] **Step 1: Write the failing unit tests** (in the tests of `src/completion.rs`)

Every place in these tests that builds a `Catalog` now goes through `catalog` below; change the two in Task 3's tests (`keywords_at` and `a_keyword_is_inserted_as_it_reads_and_marks_what_matched`) to `let tree = Tree::default(); let catalog = catalog(Dialect::Sqlite, &tree, None, &[]);`.

```rust
    use crate::model::{Fetch, SchemaNode, Tree};
    use tabletist_db::{ObjectInfo, ObjectKind};

    fn catalog<'a>(
        dialect: Dialect,
        tree: &'a Tree,
        bare: Option<&'a str>,
        schemas: &'a [String],
    ) -> Catalog<'a> {
        Catalog {
            dialect,
            tree,
            bare,
            schemas,
        }
    }

    /// A tree with these schemas loaded, each with its objects.
    fn tree_of(schemas: &[(&str, &[(&str, ObjectKind)])]) -> Tree {
        let mut tree = Tree::default();
        for (schema, objects) in schemas {
            let objects = objects
                .iter()
                .map(|(name, kind)| ObjectInfo {
                    name: (*name).to_owned(),
                    kind: *kind,
                    estimated_rows: None,
                })
                .collect();
            let node = SchemaNode {
                objects: Fetch {
                    value: Some(objects),
                    ..Default::default()
                },
                ..Default::default()
            };
            tree.nodes.insert((*schema).to_owned(), node);
        }
        tree
    }

    fn shop() -> (Tree, Vec<String>) {
        use ObjectKind::{MaterializedView, Table, View};
        let tree = tree_of(&[
            (
                "public",
                &[
                    ("users", Table),
                    ("active_users", View),
                    ("Order Items", Table),
                    ("user_totals", MaterializedView),
                ],
            ),
            ("billing", &[("invoices", Table), ("users", Table)]),
        ]);
        (tree, vec!["billing".to_owned(), "public".to_owned()])
    }

    /// `(label, insert, kind)` of what a PostgreSQL list offers at `marked`.
    fn offered(marked: &str, bare: Option<&str>) -> Vec<(String, String, Kind)> {
        let (tree, schemas) = shop();
        let (site, typed) = site_at(marked);
        let catalog = catalog(Dialect::Postgres, &tree, bare, &schemas);
        list(&site, &typed, false, &catalog)
            .candidates
            .into_iter()
            .map(|c| (c.label, c.insert, c.kind))
            .collect()
    }

    fn row(label: &str, insert: &str, kind: Kind) -> (String, String, Kind) {
        (label.to_owned(), insert.to_owned(), kind)
    }

    #[test]
    fn a_table_site_offers_tables_schemas_and_cte_names() {
        assert_eq!(
            offered("SELECT * FROM |", Some("public")),
            [
                // The bare schema's, bare.
                row("active_users", "active_users", Kind::View),
                row("Order Items", "\"Order Items\"", Kind::Table),
                row("user_totals", "user_totals", Kind::MaterializedView),
                row("users", "users", Kind::Table),
                // Every other loaded schema's, qualified.
                row("billing.invoices", "billing.invoices", Kind::Table),
                row("billing.users", "billing.users", Kind::Table),
                // The schemas.
                row("billing", "billing", Kind::Schema),
                row("public", "public", Kind::Schema),
            ]
        );
        // Names that start with what is typed lead the ones that hold it.
        assert_eq!(
            offered("SELECT * FROM us|", Some("public")),
            [
                row("user_totals", "user_totals", Kind::MaterializedView),
                row("users", "users", Kind::Table),
                row("active_users", "active_users", Kind::View),
                row("billing.users", "billing.users", Kind::Table),
            ]
        );
        assert_eq!(
            offered("WITH recent AS (SELECT 1) SELECT * FROM rec|", Some("public")),
            [row("recent", "recent", Kind::Table)]
        );
    }

    #[test]
    fn nothing_is_bare_without_a_bare_schema() {
        assert_eq!(
            offered("SELECT * FROM inv|", None),
            [row("billing.invoices", "billing.invoices", Kind::Table)]
        );
        assert!(
            offered("SELECT * FROM users|", None)
                .iter()
                .all(|(label, ..)| label.contains('.'))
        );
    }

    #[test]
    fn a_schema_and_a_dot_offer_that_schemas_tables() {
        assert_eq!(
            offered("SELECT * FROM billing.|", Some("public")),
            [
                row("invoices", "invoices", Kind::Table),
                row("users", "users", Kind::Table),
            ]
        );
        // Whatever the case the schema is typed in.
        assert_eq!(offered("SELECT * FROM BILLING.inv|", Some("public")).len(), 1);
        // Not a schema: nothing.
        assert!(offered("SELECT * FROM nope.|", Some("public")).is_empty());
        // After a dot there are no keywords, wherever it is.
        assert!(offered("SELECT billing.sel|", Some("public")).is_empty());
    }

    #[test]
    fn a_site_needs_the_objects_of_the_schema_it_reads() {
        let (tree, schemas) = shop();
        let needed = |marked: &str, bare| {
            let (site, _) = site_at(marked);
            needs(&site, &catalog(Dialect::Postgres, &tree, bare, &schemas))
        };
        assert_eq!(needed("SELECT * FROM |", Some("public")), [Need::Objects("public".into())]);
        assert_eq!(needed("SELECT * FROM billing.|", Some("public")), [Need::Objects("billing".into())]);
        assert!(needed("SELECT * FROM |", None).is_empty());
        assert!(needed("SEL|", Some("public")).is_empty());
    }
```

- [ ] **Step 2: Write the failing headless tests** (append to `src/ui/complete_tests.rs`)

```rust
use crate::backend::Event;
use tabletist_db::{ObjectInfo, ObjectKind};

fn table(name: &str) -> ObjectInfo {
    ObjectInfo {
        name: name.to_owned(),
        kind: ObjectKind::Table,
        estimated_rows: None,
    }
}

/// How many commands the backend was sent that `counts` holds for.
fn sent(harness: &Harness, counts: impl Fn(&Command) -> bool) -> usize {
    harness.app.backend.sent.iter().filter(|command| counts(command)).count()
}

fn asked_for_objects(harness: &Harness, of: &str) -> usize {
    sent(harness, |command| matches!(command, Command::ListObjects { schema, .. } if schema == of))
}

/// Answers the newest `ListObjects`.
fn answer_objects(harness: &mut Harness, result: Result<Vec<ObjectInfo>, tabletist_db::Error>) {
    let (session, request, schema) = harness
        .app
        .backend
        .sent
        .iter()
        .rev()
        .find_map(|command| match command {
            Command::ListObjects {
                session,
                request,
                schema,
            } => Some((*session, *request, schema.clone())),
            _ => None,
        })
        .expect("a ListObjects was sent");
    harness.app.apply(Action::Backend(Event::Objects {
        session,
        request,
        schema,
        result,
    }));
    harness.settle();
}

/// An editor on a workspace that also has a schema `reports`, whose
/// objects were never loaded, with `select * from reports` typed.
fn editor_before_reports() -> (Harness, ConnTabId) {
    let (mut harness, tab) = editor();
    let workspace = harness.app.workspace_mut(tab).unwrap();
    workspace.tree.schemas.value = Some(vec!["main".into(), "reports".into()]);
    paste(&mut harness, "select * from reports");
    (harness, tab)
}

#[test]
fn tables_and_views_are_offered_where_a_table_goes() {
    let (mut harness, tab) = editor();
    paste(&mut harness, "select * from ");
    type_text(&mut harness, "us");
    // What starts with the typed text, then what holds it.
    assert_eq!(labels(&harness, tab), ["users", "active_users"]);
    harness.press(Key::Tab, Modifiers::NONE);
    assert_eq!(sql(&harness, tab).text, "select * from users");
}

#[test]
fn a_schema_and_then_its_tables() {
    let (mut harness, tab) = editor();
    paste(&mut harness, "select * from ");
    type_text(&mut harness, "ma");
    assert_eq!(labels(&harness, tab), ["main"]);
    harness.press(Key::Tab, Modifiers::NONE);
    // The dot opens the list on the schema's tables.
    type_text(&mut harness, ".");
    assert_eq!(labels(&harness, tab), ["active_users", "orders", "users"]);
    type_text(&mut harness, "or");
    harness.press(Key::Enter, Modifiers::NONE);
    assert_eq!(sql(&harness, tab).text, "select * from main.orders");
    assert_eq!(asked_for_objects(&harness, "main"), 1, "loaded at connect, not again");
}

#[test]
fn a_schema_is_loaded_on_demand() {
    let (mut harness, tab) = editor_before_reports();
    assert_eq!(asked_for_objects(&harness, "reports"), 0);
    type_text(&mut harness, ".");
    assert_eq!(asked_for_objects(&harness, "reports"), 1);
    // The list waits, open, with no rows.
    let open = list(&harness, tab).expect("a waiting list");
    assert!(open.candidates.is_empty() && open.loading);
    answer_objects(&mut harness, Ok(vec![table("monthly"), table("yearly")]));
    assert_eq!(labels(&harness, tab), ["monthly", "yearly"]);
    assert!(!list(&harness, tab).unwrap().loading);
    // Not asked again, and not unfolded in the sidebar.
    type_text(&mut harness, "m");
    assert_eq!(labels(&harness, tab), ["monthly"]);
    assert_eq!(asked_for_objects(&harness, "reports"), 1);
    let workspace = harness.app.workspace(tab).unwrap();
    assert!(!workspace.tree.nodes["reports"].expanded);
}

#[test]
fn enter_is_a_line_break_while_a_list_waits() {
    let (mut harness, tab) = editor_before_reports();
    type_text(&mut harness, ".");
    assert!(list(&harness, tab).is_some_and(|list| list.candidates.is_empty()));
    harness.press(Key::Enter, Modifiers::NONE);
    assert_eq!(sql(&harness, tab).text, "select * from reports.\n");
    assert!(list(&harness, tab).is_none());
}

#[test]
fn an_answer_that_leaves_the_list_empty_closes_it() {
    let (mut harness, tab) = editor_before_reports();
    type_text(&mut harness, ".");
    answer_objects(&mut harness, Ok(Vec::new()));
    assert!(list(&harness, tab).is_none());
}

#[test]
fn a_load_that_failed_is_not_asked_again() {
    let (mut harness, tab) = editor_before_reports();
    type_text(&mut harness, ".");
    answer_objects(&mut harness, Err(tabletist_db::Error::query("permission denied")));
    assert!(list(&harness, tab).is_none());
    type_text(&mut harness, "mo");
    assert_eq!(asked_for_objects(&harness, "reports"), 1);
}

#[test]
fn nothing_is_asked_of_a_session_that_is_not_connected() {
    let (mut harness, tab) = editor_before_reports();
    harness.app.workspace_mut(tab).unwrap().status =
        crate::model::SessionStatus::Disconnected(tabletist_db::Error::query("gone"));
    type_text(&mut harness, ".");
    assert_eq!(asked_for_objects(&harness, "reports"), 0);
    assert!(list(&harness, tab).is_none());
}

#[test]
fn refreshing_the_tree_forgets_what_only_the_list_loaded() {
    let (mut harness, tab) = editor_before_reports();
    type_text(&mut harness, ".");
    answer_objects(&mut harness, Ok(vec![table("monthly")]));
    harness.app.apply(Action::RefreshTree(tab));
    let workspace = harness.app.workspace(tab).unwrap();
    assert!(!workspace.tree.nodes.contains_key("reports"));
    assert!(workspace.tree.nodes.contains_key("main"), "the sidebar's own stays");
}

#[test]
fn a_large_schema_is_listed_once_per_change() {
    let (mut harness, tab) = editor();
    let objects: Vec<ObjectInfo> = (0..10_000).map(|index| table(&format!("name_{index:05}"))).collect();
    let workspace = harness.app.workspace_mut(tab).unwrap();
    workspace.tree.nodes.get_mut("main").unwrap().objects.value = Some(objects);
    paste(&mut harness, "select * from ");
    type_text(&mut harness, "na");
    let open = list(&harness, tab).expect("a list");
    assert_eq!((open.candidates.len(), open.more), (100, 9_900));
    let listed = LISTED.with(|count| count.get());
    harness.settle();
    harness.press(Key::ArrowDown, Modifiers::NONE);
    assert_eq!(LISTED.with(|count| count.get()), listed, "moving lists nothing");
}
```

- [ ] **Step 3: Run to see them fail**

Run: `~/.cargo/bin/cargo test --locked --lib -- completion:: complete_tests`
Expected: FAIL to compile (`Catalog` has no `tree`; `needs`, `Need` missing).

- [ ] **Step 4: The candidates**

In `src/completion.rs`, add the imports and replace `Catalog`, `list` and `offers_keywords`:

```rust
use tabletist_db::{ObjectInfo, ObjectKind};

use crate::model::Tree;
```

```rust
/// What the workspace knows, as a list reads it.
pub struct Catalog<'a> {
    pub dialect: Dialect,
    pub tree: &'a Tree,
    /// The schema a bare name is looked up in (`public`, the connection's
    /// database, `main`), when the tree has it.
    pub bare: Option<&'a str>,
    /// The schemas the sidebar shows.
    pub schemas: &'a [String],
}

/// Names a list reads that the workspace may not have loaded yet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Need {
    /// The tables and views of this schema.
    Objects(String),
}

/// Where a kind of candidate comes in a site's list: lower first.
type Group = u8;

/// The bare schema's tables; after a dot, the named schema's.
const BARE: Group = 0;
/// The other schemas' tables, qualified.
const QUALIFIED: Group = 1;
const SCHEMAS: Group = 2;
const CTES: Group = 3;
/// Keywords come after every name.
const KEYWORDS: Group = 9;

/// What a site asks for, once its qualifier is read.
enum Target<'a> {
    Nothing,
    Keywords,
    /// Tables and views of every loaded schema, schemas and CTE names.
    Tables,
    /// The tables and views of this schema: the site is after `schema.`.
    SchemaTables(&'a str),
}

/// What `site` asks for. `manual` says the list was asked for by hand.
fn target<'a>(site: &Site, manual: bool, catalog: &Catalog<'a>) -> Target<'a> {
    match site.qualifier.as_slice() {
        [] => match site.expects {
            Expects::Start | Expects::Columns => Target::Keywords,
            // A new name: keywords only when asked for by hand.
            Expects::Name if manual => Target::Keywords,
            Expects::Name => Target::Nothing,
            Expects::Tables => Target::Tables,
        },
        [schema] => schema_named(catalog, schema).map_or(Target::Nothing, Target::SchemaTables),
        _ => Target::Nothing,
    }
}

/// The shown schema called `name`: as spelled, else whatever its case.
fn schema_named<'a>(catalog: &Catalog<'a>, name: &str) -> Option<&'a str> {
    let schemas = catalog.schemas;
    schemas
        .iter()
        .find(|schema| *schema == name)
        .or_else(|| schemas.iter().find(|schema| schema.eq_ignore_ascii_case(name)))
        .map(String::as_str)
}

/// The loaded tables and views of `schema`; none while they are not.
fn objects_of<'a>(catalog: &Catalog<'a>, schema: &str) -> &'a [ObjectInfo] {
    let node = catalog.tree.nodes.get(schema);
    node.and_then(|node| node.objects.value.as_deref())
        .unwrap_or_default()
}

/// The names `site` reads that may still have to be loaded: the caller
/// asks for the ones the workspace never loaded.
pub fn needs(site: &Site, catalog: &Catalog<'_>) -> Vec<Need> {
    let schema = match target(site, true, catalog) {
        Target::Tables => catalog.bare,
        Target::SchemaTables(schema) => Some(schema),
        Target::Nothing | Target::Keywords => None,
    };
    schema
        .map(|schema| Need::Objects(schema.to_owned()))
        .into_iter()
        .collect()
}

/// What `site` offers for the `typed` part of its word, from `catalog`.
/// `manual` says the list was asked for by hand.
pub fn list(site: &Site, typed: &str, manual: bool, catalog: &Catalog<'_>) -> Listed {
    #[cfg(test)]
    LISTED.with(|count| count.set(count.get() + 1));
    let dialect = catalog.dialect;
    let mut found: Vec<(Group, Candidate)> = Vec::new();
    match target(site, manual, catalog) {
        Target::Nothing => {}
        Target::Keywords => {
            let words = tabletist_db::sql::keywords(dialect).chain(PHRASES);
            let words = words.filter_map(|word| keyword(word, typed));
            found.extend(words.map(|candidate| (KEYWORDS, candidate)));
        }
        Target::Tables => {
            let mut loaded: Vec<&String> = catalog.tree.nodes.keys().collect();
            loaded.sort();
            for schema in loaded {
                let bare = catalog.bare == Some(schema.as_str());
                let (group, qualifier) = if bare {
                    (BARE, None)
                } else {
                    (QUALIFIED, Some(schema.as_str()))
                };
                let objects = objects_of(catalog, schema).iter();
                let objects = objects.filter_map(|info| object(info, qualifier, typed, dialect));
                found.extend(objects.map(|candidate| (group, candidate)));
            }
            let schemas = catalog.schemas.iter();
            let schemas = schemas.filter_map(|schema| name(schema, Kind::Schema, typed, dialect));
            found.extend(schemas.map(|candidate| (SCHEMAS, candidate)));
            let ctes = site.ctes.iter();
            let ctes = ctes.filter_map(|cte| name(cte, Kind::Table, typed, dialect));
            found.extend(ctes.map(|candidate| (CTES, candidate)));
        }
        Target::SchemaTables(schema) => {
            let objects = objects_of(catalog, schema).iter();
            let objects = objects.filter_map(|info| object(info, None, typed, dialect));
            found.extend(objects.map(|candidate| (BARE, candidate)));
        }
    }
    rank(found, typed)
}

/// A table or view as a candidate when it holds `typed`: bare, or as
/// `schema.name` when it lives outside the bare schema.
fn object(
    info: &ObjectInfo,
    schema: Option<&str>,
    typed: &str,
    dialect: Dialect,
) -> Option<Candidate> {
    let label = match schema {
        Some(schema) => format!("{schema}.{}", info.name),
        None => info.name.clone(),
    };
    let matched = find_ignoring_case(&label, typed)?;
    let insert = match schema {
        Some(schema) => format!("{}.{}", dialect.ident(schema), dialect.ident(&info.name)),
        None => dialect.ident(&info.name).into_owned(),
    };
    let kind = match info.kind {
        ObjectKind::Table => Kind::Table,
        ObjectKind::View => Kind::View,
        ObjectKind::MaterializedView => Kind::MaterializedView,
    };
    Some(Candidate {
        kind,
        label,
        insert,
        matched,
        detail: String::new(),
    })
}

/// A schema or a CTE as a candidate when its name holds `typed`.
fn name(name: &str, kind: Kind, typed: &str, dialect: Dialect) -> Option<Candidate> {
    Some(Candidate {
        kind,
        label: name.to_owned(),
        insert: dialect.ident(name).into_owned(),
        matched: find_ignoring_case(name, typed)?,
        detail: String::new(),
    })
}
```

`offers_keywords` is gone: `target` decides. The block above defines `type Group` and `KEYWORDS` again with the new groups: drop Task 3's. Keep `keyword`, `find_ignoring_case` and `rank` as they are.

- [ ] **Step 5: The workspace's side**

In `src/model.rs`, in `impl Workspace`:

```rust
    /// The schema a statement's bare names are looked up in: `public` on
    /// PostgreSQL, the connection's database on MySQL, `main` on SQLite.
    /// `None` when the server has no such schema: then a completion
    /// inserts every name with its schema.
    pub fn bare_schema(&self) -> Option<&str> {
        let name = match self.driver {
            Driver::Postgres => "public",
            Driver::MySql => self.spec.database.as_str(),
            Driver::Sqlite => "main",
        };
        let schemas = self.tree.schemas.value.as_ref()?;
        schemas.iter().find(|schema| *schema == name).map(String::as_str)
    }

    /// The names a completion list reads changed: an open list is worked
    /// out again.
    pub fn catalog_changed(&mut self) {
        self.catalog_generation = self.catalog_generation.wrapping_add(1);
    }

    /// Whether what `need` names is being fetched.
    pub fn is_loading(&self, need: &crate::completion::Need) -> bool {
        match need {
            crate::completion::Need::Objects(schema) => self
                .tree
                .nodes
                .get(schema)
                .is_some_and(|node| node.objects.is_loading()),
        }
    }
```

Add a test beside the other workspace tests in `src/model.rs`:

```rust
    #[test]
    fn the_bare_schema_is_the_one_a_dialect_searches() {
        let mut workspace = crate::testing::workspace();
        // The fixture is SQLite.
        assert_eq!(workspace.bare_schema(), None, "no schemas listed yet");
        workspace.tree.schemas.value = Some(vec!["main".into(), "audit".into()]);
        assert_eq!(workspace.bare_schema(), Some("main"));
        workspace.driver = Driver::Postgres;
        assert_eq!(workspace.bare_schema(), None, "no public here");
        workspace.tree.schemas.value = Some(vec!["public".into(), "audit".into()]);
        assert_eq!(workspace.bare_schema(), Some("public"));
        workspace.driver = Driver::MySql;
        workspace.spec.database = "audit".into();
        assert_eq!(workspace.bare_schema(), Some("audit"));
        workspace.spec.database = String::new();
        assert_eq!(workspace.bare_schema(), None, "no database chosen");
    }
```

- [ ] **Step 6: The app's side**

In `src/app.rs`:

Call `workspace.catalog_changed()` wherever the tree's names change: in `load_objects` (after `.start(request)`), in the `Event::Schemas` arm (after `schemas.finish` accepted the answer), in the `Event::Objects` arm (after `node.objects.finish`), and in `Action::SwitchDatabase` (after `workspace.tree = Tree::default()`).

In `refresh_tree`, after `self.load_schemas(tab);`:

```rust
        // A schema only a completion list loaded is not the sidebar's to
        // refresh: it goes, and loads again when a list needs it.
        if let Some(workspace) = self.workspace_mut(tab) {
            workspace
                .tree
                .nodes
                .retain(|_, node| node.expanded || node.objects.is_loading());
            workspace.catalog_changed();
        }
```

Add beside `refresh_completion`:

```rust
    /// Asks the backend for the names a completion list needs and the
    /// workspace never loaded. Only on a connected session; a load that
    /// failed is not asked for again until the tree is refreshed.
    fn send_needs(&mut self, tab: ConnTabId, needs: &[Need]) {
        for need in needs {
            let Some(workspace) = self.workspace(tab) else {
                return;
            };
            if !matches!(workspace.status, SessionStatus::Connected) {
                return;
            }
            match need {
                Need::Objects(schema) => {
                    let node = workspace.tree.nodes.get(schema);
                    if node.is_none_or(|node| node.objects.needs_load()) {
                        self.load_objects(tab, schema);
                    }
                }
            }
        }
    }
```

and as a free function near `lost`:

```rust
/// What `workspace` knows, as a completion list reads it. `schemas` are
/// the ones its sidebar shows.
fn catalog_of<'a>(
    workspace: &'a Workspace,
    schemas: &'a [String],
) -> crate::completion::Catalog<'a> {
    crate::completion::Catalog {
        dialect: workspace.driver.dialect(),
        tree: &workspace.tree,
        bare: workspace.bare_schema(),
        schemas,
    }
}
```

In `refresh_completion`, replace everything from `let palette = self.palette;` to the end with:

```rust
        let palette = self.palette;
        let show_system = self.settings.show_system_schemas;
        let Some(workspace) = self.workspace_mut(tab) else {
            return changed;
        };
        let dialect = workspace.driver.dialect();
        let generation = workspace.catalog_generation;
        let Some(sql) = workspace.sql_tab_mut(id) else {
            return changed;
        };
        let wanted = sql.completion_wanted.take();
        let was_open = sql.completion.is_some();
        if wanted.is_none() && !was_open {
            return changed;
        }
        let (print, cursor) = (TextPrint::of(&sql.text), sql.cursor);
        let unchanged = |list: &Completion| list.is_of((print, cursor, generation));
        if wanted.is_none() && sql.completion.as_ref().is_some_and(unchanged) {
            return changed;
        }
        let editor = crate::ui::sql_text::editor_id(tab, id);
        let parsed = crate::ui::sql_text::parsed(ctx, editor, dialect, &palette, &sql.text);
        let Some(site) = tabletist_db::complete::site(&parsed.tokens, &sql.text, cursor) else {
            sql.completion = None;
            return changed | was_open;
        };
        // A cursor that is stale or inside a character has no word.
        let Some(typed) = sql.text.get(site.word.start..cursor).map(str::to_owned) else {
            sql.completion = None;
            return changed | was_open;
        };
        // An open list goes on only for the word it was opened on.
        let going = sql.completion.as_ref().is_some_and(|list| list.is_on(&site));
        let manual = match wanted {
            Some(Wanted::Manual) => true,
            _ if going => sql.completion.as_ref().is_some_and(|list| list.manual),
            Some(Wanted::Typed) => false,
            None => {
                // The cursor left the word.
                sql.completion = None;
                return true;
            }
        };
        // Asking is part of opening: the names the site reads are asked
        // for before the list is worked out, whatever it holds so far.
        let Some(workspace) = self.workspace(tab) else {
            return changed;
        };
        let schemas = workspace.tree.visible_schemas(workspace.driver, show_system);
        let needs = crate::completion::needs(&site, &catalog_of(workspace, &schemas));
        self.send_needs(tab, &needs);
        let Some(workspace) = self.workspace(tab) else {
            return changed;
        };
        let catalog = catalog_of(workspace, &schemas);
        let listed = crate::completion::list(&site, &typed, manual, &catalog);
        let loading = needs.iter().any(|need| workspace.is_loading(need));
        // The generation after asking: asking changed it.
        let of = (print, cursor, workspace.catalog_generation);
        let Some(sql) = self.sql_tab_mut(tab, id) else {
            return changed;
        };
        // A list with no rows stays open only while names are on their way.
        let empty = listed.candidates.is_empty() && !loading;
        if going {
            match sql.completion.as_mut() {
                Some(list) if !empty => {
                    list.manual = manual;
                    list.relist(of, site, typed, listed, loading);
                }
                _ => sql.completion = None,
            }
            return true;
        }
        // Opening. By typing: a word of two characters or more, or right
        // after a dot; not where a new name goes; not when the only row
        // is what is already typed (its names were still asked for).
        let only_exact = listed.more == 0
            && matches!(listed.candidates.as_slice(), [only] if only.is_typed(&typed));
        let after_dot = typed.is_empty() && !site.qualifier.is_empty();
        let new_name = site.expects == tabletist_db::complete::Expects::Name
            && site.qualifier.is_empty();
        let opens = manual
            || ((typed.chars().count() >= 2 || after_dot) && !new_name && !only_exact);
        sql.completion = (opens && !empty)
            .then(|| Completion::new(manual, of, site, typed, listed, loading));
        changed | was_open | sql.completion.is_some()
```

Import `crate::completion::Need`.

In `src/shots.rs`, make the `sql-complete` scene complete a table of the demo data:

```rust
    both("sql-complete", |harness| {
        sql_editor(harness);
        harness.settle();
        harness.frame(vec![egui::Event::Paste("\nSELECT * FROM ".into())]);
        harness.frame(vec![egui::Event::Text("bo".into())]);
    });
```

- [ ] **Step 7: Run to see them pass**

Run: `~/.cargo/bin/cargo test --locked --lib -- completion:: complete_tests model::tests` and `~/.cargo/bin/cargo check --locked --features shots --tests`
Expected: PASS. The earlier app tests that count commands at connect and on refresh must pass unchanged: `refresh_tree` sends what it sent before.

If `nothing_is_asked_of_a_session_that_is_not_connected` cannot type because the editor is not drawn while the session is down, keep the status change and ask through the model instead (`Action::OpenCompletion` after putting the text and cursor on the tab), with the same two assertions.

- [ ] **Step 8: Run the checks and commit**

```bash
~/.cargo/bin/cargo fmt --all --check && ~/.cargo/bin/cargo clippy --locked --workspace --all-targets -- -D warnings && ~/.cargo/bin/cargo test --locked --workspace --all-targets && RUSTDOCFLAGS='-D warnings' ~/.cargo/bin/cargo doc --locked --workspace --no-deps && ~/.cargo/bin/cargo check --locked --features shots --tests
git add src/completion.rs src/model.rs src/app.rs src/shots.rs src/ui/complete_tests.rs
git commit -m "Complete schemas, tables and views in the SQL editor

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

Step 2 of the spec is complete here.

---

### Task 9: Columns

**Files:**
- Modify: `src/completion.rs` (`Catalog.columns`, `Need::Columns`, `Target::Columns`, `resolve`, `list`, `needs`, tests)
- Modify: `src/model.rs` (`Workspace.columns`, `is_loading`, `forget_session_requests`)
- Modify: `src/app.rs` (`catalog_of`, `send_needs`, `load_columns`, the `Structure` event, `refresh_tree`)
- Modify: `src/ui/complete_tests.rs`

**Interfaces:**
- Consumes: `Command::Describe`, `Event::Structure`, `Site.sources`.
- Produces: `Workspace.columns: HashMap<ObjectRef, Fetch<Vec<ColumnInfo>>>`, `Need::Columns(ObjectRef)`, `completion::TABLES`.

- [ ] **Step 1: Write the failing unit tests** (in the tests of `src/completion.rs`)

Change the `catalog` helper to give every catalog an empty column cache, and add the helpers below:

```rust
    use std::collections::HashMap;
    use tabletist_db::{ColumnInfo, ObjectRef};

    type Columns = HashMap<ObjectRef, Fetch<Vec<ColumnInfo>>>;

    fn catalog<'a>(
        dialect: Dialect,
        tree: &'a Tree,
        bare: Option<&'a str>,
        schemas: &'a [String],
    ) -> Catalog<'a> {
        Catalog {
            dialect,
            tree,
            bare,
            schemas,
            // No columns known. (Leaked: a test's few bytes.)
            columns: Box::leak(Box::new(Columns::new())),
        }
    }

    /// A schema, a table, and its columns: each a name and a type.
    type TableColumns<'a> = (&'a str, &'a str, &'a [(&'a str, &'a str)]);

    /// A column cache holding these tables' columns.
    fn columns_of(tables: &[TableColumns<'_>]) -> Columns {
        let mut kept = Columns::new();
        for (schema, table, columns) in tables {
            let columns = columns
                .iter()
                .map(|(name, type_name)| ColumnInfo {
                    name: (*name).to_owned(),
                    type_name: (*type_name).to_owned(),
                    ..Default::default()
                })
                .collect();
            let fetched = Fetch {
                value: Some(columns),
                ..Default::default()
            };
            kept.insert(ObjectRef::new(*schema, *table), fetched);
        }
        kept
    }

    fn shop_columns() -> Columns {
        columns_of(&[
            (
                "public",
                "users",
                &[("id", "integer"), ("name", "text"), ("Created At", "timestamptz")],
            ),
            ("billing", "invoices", &[("id", "integer"), ("total", "numeric")]),
        ])
    }

    /// What a PostgreSQL list offers at `marked` with the shop's columns
    /// known.
    fn offered_columns(marked: &str) -> Vec<Candidate> {
        let (tree, schemas) = shop();
        let columns = shop_columns();
        let (site, typed) = site_at(marked);
        let catalog = Catalog {
            columns: &columns,
            ..catalog(Dialect::Postgres, &tree, Some("public"), &schemas)
        };
        list(&site, &typed, false, &catalog).candidates
    }

    fn labels_of(candidates: &[Candidate]) -> Vec<&str> {
        candidates.iter().map(|c| c.label.as_str()).collect()
    }

    #[test]
    fn columns_of_the_statements_tables_come_before_keywords() {
        let found = offered_columns("SELECT na| FROM users");
        assert_eq!(labels_of(&found), ["name", "natural"]);
        assert_eq!((found[0].kind, found[0].detail.as_str()), (Kind::Column, "text"));
        assert_eq!(found[1].kind, Kind::Keyword);
        // A name that needs quotes is shown bare and inserted quoted.
        let quoted = offered_columns("SELECT cr| FROM users");
        assert_eq!(quoted[0].label, "Created At");
        assert_eq!(quoted[0].insert, "\"Created At\"");
        // A column two tables share is offered once.
        let shared = offered_columns("SELECT id| FROM users u JOIN billing.invoices i ON true");
        assert_eq!(labels_of(&shared).iter().filter(|label| **label == "id").count(), 1);
        // A table the workspace does not know has no columns.
        assert_eq!(labels_of(&offered_columns("SELECT na| FROM nope")), ["natural"]);
    }

    #[test]
    fn a_qualifier_is_an_alias_then_a_table_then_a_schema() {
        let join = "FROM users u JOIN billing.invoices i ON true";
        assert_eq!(
            labels_of(&offered_columns(&format!("SELECT u.| {join}"))),
            ["Created At", "id", "name"]
        );
        assert_eq!(labels_of(&offered_columns(&format!("SELECT i.to| {join}"))), ["total"]);
        // A table's own name.
        assert_eq!(labels_of(&offered_columns("SELECT users.na| FROM users")), ["name"]);
        // An alias spelled like a schema is the alias.
        assert_eq!(
            labels_of(&offered_columns("SELECT billing.| FROM users billing")),
            ["Created At", "id", "name"]
        );
        // `schema.table.`: that table's columns, named by the statement or not.
        assert_eq!(
            labels_of(&offered_columns("SELECT billing.invoices.| FROM users")),
            ["id", "total"]
        );
        // An alias of a table the workspace does not know: nothing.
        assert!(offered_columns("SELECT x.| FROM nope x").is_empty());
    }

    #[test]
    fn a_column_site_needs_its_tables_columns_up_to_eight() {
        let (tree, schemas) = shop();
        let needed = |marked: &str| {
            let (site, _) = site_at(marked);
            needs(&site, &catalog(Dialect::Postgres, &tree, Some("public"), &schemas))
        };
        let object = |schema: &str, name: &str| Need::Columns(ObjectRef::new(schema, name));
        assert_eq!(
            needed("SELECT | FROM users u JOIN billing.invoices i ON true"),
            [
                Need::Objects("public".into()),
                object("public", "users"),
                object("billing", "invoices"),
            ]
        );
        assert_eq!(
            needed("SELECT billing.invoices.| FROM users"),
            [
                Need::Objects("public".into()),
                Need::Objects("billing".into()),
                object("billing", "invoices"),
            ]
        );
        // The same table twice is asked for once.
        assert_eq!(needed("SELECT | FROM users a JOIN users b ON true").len(), 2);

        let names: Vec<String> = (0..10).map(|index| format!("t{index}")).collect();
        let tables: Vec<(&str, ObjectKind)> =
            names.iter().map(|name| (name.as_str(), ObjectKind::Table)).collect();
        let wide = tree_of(&[("public", tables.as_slice())]);
        let (site, _) = site_at(&format!("SELECT | FROM {}", names.join(", ")));
        let schemas = vec!["public".to_owned()];
        let needed = needs(&site, &catalog(Dialect::Postgres, &wide, Some("public"), &schemas));
        let columns = needed.iter().filter(|need| matches!(need, Need::Columns(_)));
        assert_eq!(columns.count(), TABLES);
    }
```

- [ ] **Step 2: Write the failing headless tests** (append to `src/ui/complete_tests.rs`)

```rust
use tabletist_db::{ColumnInfo, ObjectRef, Structure};

fn structure(columns: &[(&str, &str)]) -> Structure {
    Structure {
        columns: columns
            .iter()
            .map(|(name, type_name)| ColumnInfo {
                name: (*name).to_owned(),
                type_name: (*type_name).to_owned(),
                ..Default::default()
            })
            .collect(),
        ..Default::default()
    }
}

fn users() -> Structure {
    structure(&[("id", "INTEGER"), ("email", "TEXT"), ("meta", "JSON")])
}

fn asked_to_describe(harness: &Harness, table: &str) -> usize {
    sent(harness, |command| matches!(command, Command::Describe { object, .. } if object.name == table))
}

/// Answers the newest `Describe` of `table`.
fn answer_describe(harness: &mut Harness, table: &str, result: Result<Structure, tabletist_db::Error>) {
    let (session, request) = harness
        .app
        .backend
        .sent
        .iter()
        .rev()
        .find_map(|command| match command {
            Command::Describe {
                session,
                request,
                object,
            } if object.name == table => Some((*session, *request)),
            _ => None,
        })
        .unwrap_or_else(|| panic!("no Describe of {table} was sent"));
    harness.app.apply(Action::Backend(Event::Structure {
        session,
        request,
        result,
    }));
    harness.settle();
}

#[test]
fn columns_are_fetched_once_and_fill_the_waiting_list() {
    let (mut harness, tab) = editor();
    paste(&mut harness, "select * from users where ");
    type_text(&mut harness, "em");
    assert_eq!(asked_to_describe(&harness, "users"), 1);
    // No keyword starts with `em`: the list waits for the columns.
    let open = list(&harness, tab).expect("a waiting list");
    assert!(open.candidates.is_empty() && open.loading);
    answer_describe(&mut harness, "users", Ok(users()));
    assert_eq!(labels(&harness, tab), ["email"]);
    assert_eq!(list(&harness, tab).unwrap().candidates[0].detail, "TEXT");
    type_text(&mut harness, "a");
    assert_eq!(asked_to_describe(&harness, "users"), 1);
    harness.press(Key::Tab, Modifiers::NONE);
    assert_eq!(sql(&harness, tab).text, "select * from users where email");
}

#[test]
fn an_alias_and_a_dot_offer_that_tables_columns() {
    let (mut harness, tab) = editor();
    paste(&mut harness, "select * from users u join orders o on ");
    type_text(&mut harness, "o");
    type_text(&mut harness, ".");
    assert!(list(&harness, tab).is_some_and(|list| list.loading));
    let orders = structure(&[("id", "INTEGER"), ("user_id", "INTEGER"), ("total", "REAL")]);
    answer_describe(&mut harness, "orders", Ok(orders));
    assert_eq!(labels(&harness, tab), ["id", "total", "user_id"]);
    assert_eq!(asked_to_describe(&harness, "orders"), 1);
}

#[test]
fn a_schema_and_a_table_and_a_dot_offer_its_columns() {
    let (mut harness, tab) = editor();
    type_text(&mut harness, "select main.users.");
    answer_describe(&mut harness, "users", Ok(users()));
    assert_eq!(labels(&harness, tab), ["email", "id", "meta"]);
}

#[test]
fn a_table_tabs_structure_fills_the_columns() {
    let mut harness = Harness::new();
    let tab = harness.connect_fake();
    harness.click("users");
    harness.answer_rows(crate::testing::page(5, false));
    let table = harness.app.workspace(tab).unwrap().active_tab.unwrap();
    harness.app.describe(tab, table);
    harness.answer_structure(users());
    let workspace = harness.app.workspace(tab).unwrap();
    assert!(workspace.columns.contains_key(&ObjectRef::new("main", "users")));
    // The tab got its structure as before.
    assert!(workspace.object_tab(table).unwrap().structure.value.is_some());

    let asked = asked_to_describe(&harness, "users");
    harness.press(Key::T, Modifiers::COMMAND);
    paste(&mut harness, "select * from users where ");
    type_text(&mut harness, "em");
    assert_eq!(labels(&harness, tab), ["email"]);
    assert_eq!(asked_to_describe(&harness, "users"), asked, "known already");
}

#[test]
fn a_fetch_that_failed_is_not_asked_again() {
    let (mut harness, tab) = editor();
    paste(&mut harness, "select * from users where ");
    type_text(&mut harness, "em");
    answer_describe(&mut harness, "users", Err(tabletist_db::Error::query("permission denied")));
    assert!(list(&harness, tab).is_none());
    type_text(&mut harness, "a");
    assert_eq!(asked_to_describe(&harness, "users"), 1);
}

#[test]
fn an_exact_match_takes_the_highlight_back() {
    let (mut harness, tab) = editor();
    paste(&mut harness, "select * from users order by ");
    type_text(&mut harness, "des");
    let described = structure(&[("id", "INTEGER"), ("description", "TEXT")]);
    answer_describe(&mut harness, "users", Ok(described));
    // The column leads the keyword.
    assert_eq!(labels(&harness, tab), ["description", "desc"]);
    // Moved away and back: the highlight is the user's now.
    harness.press(Key::ArrowDown, Modifiers::NONE);
    harness.press(Key::ArrowUp, Modifiers::NONE);
    assert!(list(&harness, tab).unwrap().moved);
    // One more letter spells the keyword: it leads, highlighted, and
    // Enter is the line break that was meant.
    type_text(&mut harness, "c");
    assert_eq!(labels(&harness, tab), ["desc", "description"]);
    assert_eq!(selected(&harness, tab), Some(0));
    harness.press(Key::Enter, Modifiers::NONE);
    assert_eq!(sql(&harness, tab).text, "select * from users order by desc\n");
}

#[test]
fn refreshing_and_reconnecting_forget_the_columns() {
    let (mut harness, tab) = editor();
    paste(&mut harness, "select * from users where ");
    type_text(&mut harness, "em");
    answer_describe(&mut harness, "users", Ok(users()));
    assert_eq!(harness.app.workspace(tab).unwrap().columns.len(), 1);
    harness.app.apply(Action::RefreshTree(tab));
    assert!(harness.app.workspace(tab).unwrap().columns.is_empty());

    let workspace = harness.app.workspace_mut(tab).unwrap();
    workspace.columns.insert(ObjectRef::new("main", "users"), crate::model::Fetch::default());
    harness.app.apply(Action::Reconnect(tab));
    assert!(harness.app.workspace(tab).unwrap().columns.is_empty());
}
```

- [ ] **Step 3: Run to see them fail**

Run: `~/.cargo/bin/cargo test --locked --lib -- completion:: complete_tests`
Expected: FAIL to compile (`Catalog` has no `columns`; `Need::Columns`, `TABLES`, `Workspace.columns` missing).

- [ ] **Step 4: The candidates**

In `src/completion.rs`:

```rust
use std::collections::{HashMap, HashSet};

use tabletist_db::complete::{Expects, PHRASES, Site, Source};
use tabletist_db::{ColumnInfo, Dialect, ObjectInfo, ObjectKind, ObjectRef};

use crate::model::{Fetch, Tree};

/// How many of a statement's tables have their columns fetched.
pub const TABLES: usize = 8;
```

Add to `Catalog`:

```rust
    /// The columns of the tables whose columns were fetched.
    pub columns: &'a HashMap<ObjectRef, Fetch<Vec<ColumnInfo>>>,
```

Add to `Need`:

```rust
    /// The columns of this table or view.
    Columns(ObjectRef),
```

Add `const COLUMNS: Group = 0;` beside the other groups, and to `Target`:

```rust
    /// The columns of these tables, and then keywords when the site is
    /// not after a dot.
    Columns {
        objects: Vec<ObjectRef>,
        keywords: bool,
    },
```

Replace `target` and `needs`, and add `resolve` and `sources`:

```rust
/// What `site` asks for. `manual` says the list was asked for by hand.
fn target<'a>(site: &Site, manual: bool, catalog: &Catalog<'a>) -> Target<'a> {
    let of = |object: Option<ObjectRef>| Target::Columns {
        objects: object.into_iter().collect(),
        keywords: false,
    };
    match site.qualifier.as_slice() {
        [] => match site.expects {
            Expects::Start => Target::Keywords,
            Expects::Columns => Target::Columns {
                objects: sources(site, catalog),
                keywords: true,
            },
            // A new name: keywords only when asked for by hand.
            Expects::Name if manual => Target::Keywords,
            Expects::Name => Target::Nothing,
            Expects::Tables => Target::Tables,
        },
        // An alias first, then a table's own name, then a schema.
        [one] => {
            let alias = |source: &&Source| {
                let alias = source.alias.as_deref();
                alias.is_some_and(|alias| alias.eq_ignore_ascii_case(one))
            };
            let table = |source: &&Source| source.name.eq_ignore_ascii_case(one);
            let sources = &site.sources;
            let found = sources.iter().find(alias).or_else(|| sources.iter().find(table));
            let object = found
                .and_then(|source| resolve(source.schema.as_deref(), &source.name, catalog));
            match (object, schema_named(catalog, one)) {
                (Some(object), _) => of(Some(object)),
                // `FROM billing.` reads as a table called `billing` until
                // its name is typed: a schema is what it is.
                (None, Some(schema)) => Target::SchemaTables(schema),
                // An alias of a table the workspace does not know (yet).
                (None, None) if found.is_some() => of(None),
                (None, None) => Target::Nothing,
            }
        }
        [schema, table] => of(resolve(Some(schema), table, catalog)),
        _ => Target::Nothing,
    }
}

/// The loaded table or view a statement names: `schema.name` in that
/// schema, a bare name in the bare schema and then in the other loaded
/// ones. The name as spelled first, else whatever its case.
fn resolve(schema: Option<&str>, name: &str, catalog: &Catalog<'_>) -> Option<ObjectRef> {
    let within = |schema: &str| {
        let objects = objects_of(catalog, schema);
        let found = objects
            .iter()
            .find(|info| info.name == name)
            .or_else(|| objects.iter().find(|info| info.name.eq_ignore_ascii_case(name)))?;
        Some(ObjectRef::new(schema, found.name.as_str()))
    };
    match schema {
        Some(schema) => within(schema_named(catalog, schema)?),
        None => catalog.bare.and_then(within).or_else(|| {
            let mut loaded: Vec<&String> = catalog.tree.nodes.keys().collect();
            loaded.sort();
            loaded.into_iter().find_map(|schema| within(schema))
        }),
    }
}

/// The statement's tables the workspace knows, each once, the first
/// [`TABLES`] of them.
fn sources(site: &Site, catalog: &Catalog<'_>) -> Vec<ObjectRef> {
    let mut objects: Vec<ObjectRef> = Vec::new();
    for source in &site.sources {
        if objects.len() == TABLES {
            break;
        }
        if let Some(object) = resolve(source.schema.as_deref(), &source.name, catalog)
            && !objects.contains(&object)
        {
            objects.push(object);
        }
    }
    objects
}

/// The names `site` reads that may still have to be loaded: the caller
/// asks for the ones the workspace never loaded.
pub fn needs(site: &Site, catalog: &Catalog<'_>) -> Vec<Need> {
    let schema_objects = |schema: &str| Need::Objects(schema.to_owned());
    match target(site, true, catalog) {
        Target::Nothing | Target::Keywords => Vec::new(),
        Target::Tables => catalog.bare.map(schema_objects).into_iter().collect(),
        Target::SchemaTables(schema) => vec![schema_objects(schema)],
        Target::Columns { objects, .. } => {
            // The statement's tables are matched against loaded objects:
            // the bare schema's, and those of a `schema.table.` qualifier.
            let mut needs: Vec<Need> = catalog.bare.map(schema_objects).into_iter().collect();
            if let [schema, _] = site.qualifier.as_slice()
                && let Some(schema) = schema_named(catalog, schema)
                && catalog.bare != Some(schema)
            {
                needs.push(schema_objects(schema));
            }
            needs.extend(objects.into_iter().map(Need::Columns));
            needs
        }
    }
}
```

In `list`, move the keyword lines into a helper and add the `Columns` arm:

```rust
        Target::Keywords => keywords(&mut found, typed, dialect),
        Target::Columns { objects, keywords: with_keywords } => {
            // A column several tables share is offered once.
            let mut seen = HashSet::new();
            for object in &objects {
                let fetched = catalog.columns.get(object);
                let columns = fetched.and_then(|fetched| fetched.value.as_deref());
                for info in columns.unwrap_or_default() {
                    if seen.insert(info.name.as_str())
                        && let Some(candidate) = column(info, typed, dialect)
                    {
                        found.push((COLUMNS, candidate));
                    }
                }
            }
            if with_keywords {
                keywords(&mut found, typed, dialect);
            }
        }
```

```rust
/// Adds the keywords and phrases that start with `typed`.
fn keywords(found: &mut Vec<(Group, Candidate)>, typed: &str, dialect: Dialect) {
    let words = tabletist_db::sql::keywords(dialect).chain(PHRASES);
    let words = words.filter_map(|word| keyword(word, typed));
    found.extend(words.map(|candidate| (KEYWORDS, candidate)));
}

/// A column as a candidate when its name holds `typed`, with its type.
fn column(info: &ColumnInfo, typed: &str, dialect: Dialect) -> Option<Candidate> {
    Some(Candidate {
        kind: Kind::Column,
        label: info.name.clone(),
        insert: dialect.ident(&info.name).into_owned(),
        matched: find_ignoring_case(&info.name, typed)?,
        detail: info.type_name.clone(),
    })
}
```

- [ ] **Step 5: The cache and its fetch**

In `src/model.rs`, add to `Workspace` (set to an empty map where a `Workspace` is built):

```rust
    /// The columns of tables and views, for the SQL editor's completion:
    /// fetched for the tables a statement names, and filled by a table
    /// tab's structure. Cleared with the tree, and with the session.
    pub columns: HashMap<ObjectRef, Fetch<Vec<tabletist_db::ColumnInfo>>>,
```

Add the arm to `Workspace::is_loading`:

```rust
            crate::completion::Need::Columns(object) => {
                self.columns.get(object).is_some_and(Fetch::is_loading)
            }
```

In `forget_session_requests`, add:

```rust
        // Fetches the old session will never answer, and names that may
        // not be the new one's.
        self.columns.clear();
        self.catalog_changed();
```

In `src/app.rs`:

Add `columns: &workspace.columns,` to `catalog_of`.

Add the arm to `send_needs`:

```rust
                Need::Columns(object) => {
                    let kept = workspace.columns.get(object);
                    if kept.is_none_or(Fetch::needs_load) {
                        self.load_columns(tab, object.clone());
                    }
                }
```

Add beside `describe`:

```rust
    /// Asks for `object`'s structure, for its columns in the completion
    /// list. Nothing waits on it but the cache.
    fn load_columns(&mut self, tab: ConnTabId, object: ObjectRef) {
        let request = RequestId(self.next_id());
        let Some(workspace) = self.workspace_mut(tab) else {
            return;
        };
        workspace
            .columns
            .entry(object.clone())
            .or_default()
            .start(request);
        workspace.catalog_changed();
        let session = workspace.session;
        self.backend.send(Command::Describe {
            session,
            request,
            object,
        });
    }
```

Replace the `Event::Structure` arm:

```rust
            Event::Structure {
                session,
                request,
                result,
            } => {
                let Some(tab) = self.tab_for_session(session) else {
                    return;
                };
                let Some(workspace) = self.workspace_mut(tab) else {
                    return;
                };
                let waiting = workspace
                    .object_tabs_mut()
                    .find(|object| object.structure.pending == Some(request));
                if let Some(object) = waiting {
                    // A table tab's structure tells the completion list its
                    // columns too.
                    let columns = result.as_ref().ok().map(|structure| structure.columns.clone());
                    let named = object.object.clone();
                    object.structure.finish(request, result);
                    if let Some(columns) = columns {
                        let kept = workspace.columns.entry(named).or_default();
                        if !kept.is_loading() {
                            kept.value = Some(columns);
                            kept.error = None;
                        }
                        workspace.catalog_changed();
                    }
                } else if let Some(kept) = workspace
                    .columns
                    .values_mut()
                    .find(|kept| kept.pending == Some(request))
                {
                    kept.finish(request, result.map(|structure| structure.columns));
                    workspace.catalog_changed();
                }
            }
```

In `refresh_tree`, in the block that drops the schemas a list loaded, add `workspace.columns.clear();` before `workspace.catalog_changed();`.

- [ ] **Step 6: Run to see them pass**

Run: `~/.cargo/bin/cargo test --locked --lib -- completion:: complete_tests` and `~/.cargo/bin/cargo test --locked --lib app::`
Expected: PASS, with the existing structure tests of table tabs unchanged.

- [ ] **Step 7: Run the checks and commit**

```bash
~/.cargo/bin/cargo fmt --all --check && ~/.cargo/bin/cargo clippy --locked --workspace --all-targets -- -D warnings && ~/.cargo/bin/cargo test --locked --workspace --all-targets && RUSTDOCFLAGS='-D warnings' ~/.cargo/bin/cargo doc --locked --workspace --no-deps
git add src/completion.rs src/model.rs src/app.rs src/ui/complete_tests.rs
git commit -m "Complete the columns of the tables a statement names

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

Step 3 of the spec is complete here.

---

### Task 10: Docs and the full check

**Files:**
- Modify: `README.md`
- Modify: `docs/superpowers/specs/2026-10-01-sql-autocomplete-design.md`
- Modify: `docs/superpowers/specs/2026-09-30-sql-editor-core-design.md` (nothing but a pointer, if it helps a reader)

- [ ] **Step 1: The README**

After the line about the SQL editor ("A SQL editor per connection (Cmd/Ctrl+T) ..."), add:

```markdown
- Completion while typing SQL: keywords, schemas, tables and views, and the
  columns of the tables a statement names (Ctrl+Space asks for it anywhere).
```

- [ ] **Step 2: The spec, as built**

In `docs/superpowers/specs/2026-10-01-sql-autocomplete-design.md`, set the status line to `Status: implemented. This spec describes the slice as built; where the code and the first draft differed, the text follows the code.` and bring the text in line with this plan's "Deviations from the spec" and with anything else that changed while building: the `SqlTyped` action, `Completion.typed`, keywords matching from their start, weight 500, qualifiers whose dots touch, subqueries scanned and not skipped, where the bare schema's objects are asked for, and the commit list.

- [ ] **Step 3: The whole check**

```bash
~/.cargo/bin/cargo fmt --all --check
~/.cargo/bin/cargo clippy --locked --workspace --all-targets -- -D warnings
~/.cargo/bin/cargo test --locked --workspace --all-targets
RUSTDOCFLAGS='-D warnings' ~/.cargo/bin/cargo doc --locked --workspace --no-deps
~/.cargo/bin/cargo check --locked --features shots --tests
```

Expected: all pass; the database integration tests print "skipped" (no servers are needed for this slice).

- [ ] **Step 4: By hand, in the app**

Run the demo (`~/.cargo/bin/cargo run -- --demo`) in the macOS or standard look and in the Omarchy look, open a SQL editor (`Mod+T`), and check against the artboards and the spec:

- `sel` offers `select`; Tab and Enter insert; `Mod+Z` gives `sel` back.
- `select * from ` then two letters of a table offers tables and views; a schema and a dot offers its tables.
- `select * from <table> t where t.` offers that table's columns with their types, after a moment the first time.
- `from <table> b` then Enter is a line break; so is `order by <column> desc` then Enter.
- Esc closes the list and the cursor stays; Esc again leaves the editor.
- Omarchy: `ctrl+n` and `ctrl+p` move the highlight; with the list closed `ctrl+p` opens quick open. The mode line shows `tab complete` while the list is open.
- With a screen reader if one is at hand (Orca on Linux, VoiceOver on macOS): the highlighted row is announced while typing goes on. Say plainly in the report if this was not tried.

Report which platforms this was run on, and which were only compiled.

- [ ] **Step 5: Commit**

```bash
git add README.md docs/superpowers/specs/2026-10-01-sql-autocomplete-design.md
git commit -m "Describe SQL completion as it was built

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```


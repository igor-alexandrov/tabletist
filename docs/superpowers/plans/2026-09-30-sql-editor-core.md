# SQL Editor, Slice 1: Core Editor Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A SQL editor tab per connection: type SQL, run the statement at the cursor or the whole script in a rolled-back read-only transaction with a row limit and a timeout, and read the result in the existing grid, with no way for a script to change table data.

**Architecture:** `tabletist-db` gains a `sql` module (tokenizer, statement splitter, read-only guard) and `Connection::run_script` / `server_version`, implemented per driver behind the guard. The backend runs scripts as `Command::RunSql`, fires the session's cancel for user cancels and timeouts, and tells the script to stop through a shared `StopFlag`. In the app, `Workspace.objects` becomes `tabs: Vec<Tab>` (`Tab::Object` or `Tab::Sql`), reducers handle SQL tabs, and a new `ui::sql_editor` view draws the toolbar, the highlighted editor with its gutter, the results and the footer.

**Tech Stack:** Rust 2024 (rust-version 1.98, async closures available), egui 0.36 (crmne fork), tokio-postgres 0.7.18, mysql_async 0.37.1, rusqlite 0.37. One new direct dependency: `futures-util` (already in the lock file through tokio-postgres) for `StreamExt::next` on PostgreSQL's row stream.

**Spec:** `docs/superpowers/specs/2026-09-30-sql-editor-core-design.md`

## Deviations from the spec

Decided while planning; the spec's intent holds.

- `run_script` takes a `StopFlag` as well: the backend sets it with the cancel, so a stop between statements (and SQLite's progress handler during one) never races a cancel that arrives before its statement.
- `StatementOutcome::Error` carries the whole `Error` (code, detail, hint) rather than a message string.
- (Withdrawn after review.) The editor/results splitter was first planned as egui's own resizable panel; that loses the split on a passing window resize and keeps it in egui's persisted memory under ids that repeat across launches. The split is a share kept on the tab (`SqlTab.split`, `SetSqlSplit`), as the spec says.
- The server version is asked for when a workspace's first SQL tab opens (`Command::ServerVersion`, kept as `Workspace.server_version: Fetch<String>`), not at connect: adding an event at connect would reorder every existing backend and app test.
- The refusal message comes from the error's own text and starts "line N:" (lower case, like the crate's other messages); Messages shows it as is.
- The PostgreSQL position of an error from `FETCH` or `CLOSE` is dropped (it points into Tabletist's text, not the user's).

## Global Constraints

- Checks before every commit (AGENTS.md): `cargo fmt --all --check`, `cargo clippy --locked --workspace --all-targets -- -D warnings`, `cargo test --locked --workspace --all-targets`, `RUSTDOCFLAGS='-D warnings' cargo doc --locked --workspace --no-deps`. Use `~/.cargo/bin/cargo` (the mise shim fails). When a task touches `src/shots.rs` (Tasks 1 and 9), also run `cargo check --locked --features shots --tests`: nothing else compiles it.
- Test modules may lack imports the new tests use (`std::time::Duration`, `accesskit::Role`, `tabletist_db::Dialect`, ...): add the `use` lines.
- Database integration tests need servers: `docker compose up -d --build --wait postgres mysql`, then `TABLETIST_TEST_PG_URL=postgres://tabletist:tabletist@localhost:55432/tabletist TABLETIST_TEST_MYSQL_URL=mysql://tabletist:tabletist@localhost:53306/tabletist cargo test --workspace`. Without the variables those tests print "skipped" and pass; say so when reporting.
- Commits are signed. If the SSH agent refuses, commit with `git -c commit.gpgsign=false commit` and note it; they are re-signed before push. Never set `SSH_AUTH_SOCK` in git commands.
- End every commit message with `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`. Subject lines follow the repo: an imperative sentence, no prefix ("Split SQL scripts into statements").
- Views push `Action`s; `App::apply` reduces them. A view may edit the text a field is editing (the SQL text, and the cursor offset it reports), nothing else.
- Views draw text only through `TextRole`s; never name a font, family or size.
- User-facing strings go through `gettext`. Never use em dashes anywhere (code, strings, docs, commits).
- No design or pixel conformance checks in any test. Compare with the "SQL editor" artboards by hand; screenshots stay local and use the Bookshop demo data (never names from the design's data).
- The SQL text is never logged.
- Add a focused regression test for every behaviour. Do not weaken lints or add `allow`s without saying why.

## Review Focus

1. **No script writes.** Every refused form in the spec is refused on tokens (`Task 4` unit tests), and the bypass scripts leave a probe table unchanged on PostgreSQL and MySQL (`Task 6` `bypasses_cannot_write`, `Task 7` `bypasses_cannot_write`).
2. **Cleanup always runs.** A statement error, a cancel and a timeout all end in `ROLLBACK` (and on MySQL a session reset); a PostgreSQL typo leaves the session connected (`Task 6` `a_statement_error_keeps_the_session`); MySQL session settings never leak into browsing (`Task 7` `a_run_leaves_the_session_as_it_found_it`).
3. **The limit never reads a whole big table.** PostgreSQL uses a cursor, MySQL `sql_select_limit`, SQLite stops stepping (`Task 5`/`6`/`7` `truncates_at_the_limit`).
4. **Cancel and timeout stop a slow statement** and keep earlier results (`Task 8` `a_timeout_cancels_the_script_and_keeps_earlier_results`).
5. **Stale results** for a closed tab or a replaced run are dropped (`Task 11` `a_result_for_a_closed_sql_tab_is_ignored`).
6. **Keys**: `Mod+Return` runs while typing; `Mod+R` and `Mod+F` do nothing on a SQL tab; `Mod+O` opens a connection tab (`Task 12`).

---

## File Structure

```
crates/tabletist-db/Cargo.toml       futures-util; rusqlite "hooks" (progress handler)
crates/tabletist-db/src/sql.rs       tokenize, statements, statement_at, refusal (new)
crates/tabletist-db/src/script.rs    ScriptOutcome, StatementResult, StatementOutcome,
                                     StopFlag, retry_cancelled (new)
crates/tabletist-db/src/error.rs     Error::Refused, Error::LeftReadOnly
crates/tabletist-db/src/spec.rs      Driver::dialect
crates/tabletist-db/src/lib.rs       run_script (guard, dispatch), server_version
crates/tabletist-db/src/sqlite.rs    run_script, server_version
crates/tabletist-db/src/pg.rs        run_script, server_version
crates/tabletist-db/src/mysql.rs     run_script, server_version, prepare_session
crates/tabletist-db/tests/*.rs       run_script integration and bypass tests
src/backend.rs                       Command::{RunSql, ServerVersion}, Event::{SqlRan,
                                     ServerVersion}, CancelReason, StateFile::Settings,
                                     Running.stop, timeout timer
src/model.rs                         TabId, Tab, SqlTab, SqlRun, ResultPane, new Actions
src/settings.rs                      sql_limit, sql_timeout_secs
src/app.rs                           SQL tab reducers, save_settings, generic tab actions
src/ui/keys.rs                       Mod+T, Mod+O, Mod+Return, SQL tab routing
src/ui/sql_editor.rs                 toolbar, splitter, footer, status line summary (new)
src/ui/sql_text.rs                   highlighting layouter, gutter, statement band (new)
src/ui/sql_results.rs                results header, grid, messages, states (new)
src/ui/workspace.rs                  dispatch to the SQL view; terminal status line
src/ui/object_tabs.rs                SQL tabs in the strip; `+ sql`
src/ui/sidebar.rs                    "SQL Editor" button (macOS and standard looks)
src/ui/data_view.rs                  cell formatting shared with SQL results
src/theme.rs, assets/icons/          Icon::Play, Icon::SquareCode
README.md                            feature and shortcut lines
```

---

### Task 1: Rename `ObjectTabId` to `TabId` and `active_object` to `active_tab`

A purely mechanical commit so the feature commits stay readable.

**Files:**
- Modify: every file under `src/` that names `ObjectTabId` or the `Workspace.active_object` field (about 60 sites: `app.rs`, `model.rs`, `shots.rs`, `entrypoint.rs`, `ui/*.rs`).

- [ ] **Step 1: Rename the type**

```bash
grep -rl '\bObjectTabId\b' src | xargs sed -i 's/\bObjectTabId\b/TabId/g'
```

- [ ] **Step 2: Rename the field, not the methods**

The field is read as `.active_object` without a following `(` or `_`; the methods `App::active_object()` and `Workspace::active_object_tab()` keep their names.

```bash
grep -rl 'active_object' src | xargs sed -i -E \
  -e 's/\.active_object([^_(A-Za-z0-9]|$)/.active_tab\1/g' \
  -e 's/(pub )?active_object: /\1active_tab: /g'
```

In `src/model.rs`, update the field's doc comment if it mentions objects:

```rust
    /// The tab the workspace shows: an object tab or a SQL editor.
    pub active_tab: Option<TabId>,
```

and the id type's doc:

```rust
/// One open tab in a workspace (an object or a SQL editor). Ids come from
/// `App::next_id`, so they never repeat.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TabId(pub u64);
```

- [ ] **Step 3: Build and test**

Run: `~/.cargo/bin/cargo build --locked && ~/.cargo/bin/cargo test --locked --workspace --all-targets`
Expected: builds; every test passes unchanged. Fix any site sed missed by hand (the compiler names them).

- [ ] **Step 4: Format, lint, commit**

```bash
~/.cargo/bin/cargo fmt --all && ~/.cargo/bin/cargo clippy --locked --workspace --all-targets -- -D warnings
git add -A src
git commit -m "Rename object tab ids to tab ids

SQL editor tabs will share the id space and the workspace's active tab.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 2: Tokenize SQL

**Files:**
- Create: `crates/tabletist-db/src/sql.rs`
- Modify: `crates/tabletist-db/src/lib.rs` (add `pub mod sql;`)

**Interfaces:**
- Produces: `sql::TokenKind`, `sql::Token { kind, range: Range<usize> }`, `sql::tokenize(dialect: Dialect, text: &str) -> Vec<Token>`, `sql::is_keyword(dialect, word) -> bool`.

- [ ] **Step 1: Write the failing tests**

Create `crates/tabletist-db/src/sql.rs` with only the test module first:

```rust
//! SQL text as tokens, for highlighting, for splitting a script into
//! statements, and for the read-only guard. Tokenizing never fails; it only
//! has to agree with the database on where strings, comments and statements
//! end.

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;

    fn kinds(dialect: Dialect, text: &str) -> Vec<(TokenKind, &str)> {
        tokenize(dialect, text)
            .into_iter()
            .filter(|token| token.kind != TokenKind::Whitespace)
            .map(|token| (token.kind, &text[token.range]))
            .collect()
    }

    #[test]
    fn words_numbers_strings_and_punctuation() {
        use TokenKind::*;
        assert_eq!(
            kinds(Dialect::Postgres, "SELECT a, 1.5e3 FROM t WHERE b = 'x''y';"),
            vec![
                (Keyword, "SELECT"),
                (Identifier, "a"),
                (Punctuation, ","),
                (Number, "1.5e3"),
                (Keyword, "FROM"),
                (Identifier, "t"),
                (Keyword, "WHERE"),
                (Identifier, "b"),
                (Operator, "="),
                (String, "'x''y'"),
                (Semicolon, ";"),
            ]
        );
    }

    #[test]
    fn keywords_are_case_insensitive() {
        assert_eq!(
            kinds(Dialect::Sqlite, "select")[0].0,
            TokenKind::Keyword
        );
    }

    #[test]
    fn postgres_dollar_bodies_and_escape_strings() {
        use TokenKind::*;
        let text = "SELECT $$a;b$$, $fn$ x $$ y $fn$, E'it\\'s;', $1";
        assert_eq!(
            kinds(Dialect::Postgres, text),
            vec![
                (Keyword, "SELECT"),
                (String, "$$a;b$$"),
                (Punctuation, ","),
                (String, "$fn$ x $$ y $fn$"),
                (Punctuation, ","),
                (String, "E'it\\'s;'"),
                (Punctuation, ","),
                (Punctuation, "$1"),
            ]
        );
    }

    #[test]
    fn postgres_block_comments_nest() {
        use TokenKind::*;
        assert_eq!(
            kinds(Dialect::Postgres, "/* a /* b */ c */ SELECT"),
            vec![(Comment, "/* a /* b */ c */"), (Keyword, "SELECT")]
        );
        // SQLite does not nest: the first */ ends it.
        assert_eq!(
            kinds(Dialect::Sqlite, "/* a /* b */ c")[0],
            (Comment, "/* a /* b */")
        );
    }

    #[test]
    fn quoted_identifiers_per_dialect() {
        use TokenKind::*;
        assert_eq!(
            kinds(Dialect::Postgres, r#""we""ird""#),
            vec![(QuotedIdentifier, r#""we""ird""#)]
        );
        assert_eq!(
            kinds(Dialect::MySql, "`a;b`"),
            vec![(QuotedIdentifier, "`a;b`")]
        );
        assert_eq!(
            kinds(Dialect::Sqlite, "[a;b]"),
            vec![(QuotedIdentifier, "[a;b]")]
        );
        // MySQL's default sql_mode reads "..." as a string.
        assert_eq!(kinds(Dialect::MySql, r#""a;b""#), vec![(String, r#""a;b""#)]);
    }

    #[test]
    fn mysql_comments() {
        use TokenKind::*;
        assert_eq!(
            kinds(Dialect::MySql, "SELECT 1--no comment"),
            vec![
                (Keyword, "SELECT"),
                (Number, "1"),
                (Operator, "-"),
                (Operator, "-"),
                (Keyword, "no"),
                (Identifier, "comment"),
            ]
        );
        assert_eq!(
            kinds(Dialect::MySql, "SELECT 1 -- a comment\n# another\n"),
            vec![
                (Keyword, "SELECT"),
                (Number, "1"),
                (Comment, "-- a comment"),
                (Comment, "# another"),
            ]
        );
        assert_eq!(
            kinds(Dialect::MySql, "/*!50000 COMMIT */"),
            vec![(ExecutableComment, "/*!50000 COMMIT */")]
        );
        assert_eq!(
            kinds(Dialect::MySql, r"'it\'s'"),
            vec![(String, r"'it\'s'")]
        );
    }

    #[test]
    fn unterminated_strings_and_comments_run_to_the_end() {
        use TokenKind::*;
        assert_eq!(kinds(Dialect::Postgres, "'abc"), vec![(String, "'abc")]);
        assert_eq!(kinds(Dialect::Postgres, "/* abc"), vec![(Comment, "/* abc")]);
        assert_eq!(kinds(Dialect::Postgres, "$$abc"), vec![(String, "$$abc")]);
    }

    #[test]
    fn non_ascii_text_keeps_char_boundaries() {
        let text = "SELECT 'Zoë 🚀', naïve FROM t";
        for token in tokenize(Dialect::Postgres, text) {
            assert!(text.is_char_boundary(token.range.start));
            assert!(text.is_char_boundary(token.range.end));
        }
        assert!(
            kinds(Dialect::Postgres, text).contains(&(TokenKind::Identifier, "naïve"))
        );
    }

    #[test]
    fn tokens_cover_the_whole_text() {
        let text = "SELECT a -- x\n/* y */ FROM `t` WHERE c = 'd' ; §";
        for dialect in [Dialect::Postgres, Dialect::MySql, Dialect::Sqlite] {
            let tokens = tokenize(dialect, text);
            let mut at = 0;
            for token in &tokens {
                assert_eq!(token.range.start, at);
                at = token.range.end;
            }
            assert_eq!(at, text.len());
        }
    }
}
```

In `crates/tabletist-db/src/lib.rs`, after `mod query;` add `pub mod sql;`.

- [ ] **Step 2: Run the tests to see them fail**

Run: `~/.cargo/bin/cargo test --locked -p tabletist-db --lib sql::`
Expected: FAIL to compile (`tokenize`, `TokenKind` not found).

- [ ] **Step 3: Implement the tokenizer**

Add above the test module in `sql.rs`:

```rust
use std::ops::Range;

use crate::Dialect;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenKind {
    Keyword,
    Identifier,
    /// `"name"`, `` `name` `` or `[name]`, quotes included.
    QuotedIdentifier,
    /// Quotes included; PostgreSQL `$tag$` bodies too.
    String,
    Number,
    Comment,
    /// MySQL's `/*! ... */`, whose contents MySQL runs.
    ExecutableComment,
    Operator,
    Punctuation,
    Semicolon,
    Whitespace,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    pub kind: TokenKind,
    /// Byte range in the text, always on char boundaries.
    pub range: Range<usize>,
}

/// Reserved words and common clauses, shared by every dialect.
const KEYWORDS: &[&str] = &[
    "ALL", "ALTER", "AND", "ANY", "AS", "ASC", "BEGIN", "BETWEEN", "BY", "CASE", "CAST",
    "COMMIT", "CREATE", "CROSS", "CURRENT", "DEFAULT", "DELETE", "DESC", "DISTINCT", "DROP",
    "ELSE", "END", "EXCEPT", "EXISTS", "EXPLAIN", "FALSE", "FETCH", "FILTER", "FIRST", "FOR",
    "FROM", "FULL", "GROUP", "HAVING", "IN", "INNER", "INSERT", "INTERSECT", "INTERVAL",
    "INTO", "IS", "JOIN", "LEFT", "LIKE", "LIMIT", "NATURAL", "NO", "NOT", "NULL", "OFFSET",
    "ON", "ONLY", "OR", "ORDER", "OUTER", "OVER", "PARTITION", "RECURSIVE", "RIGHT",
    "ROLLBACK", "ROWS", "SELECT", "SET", "SHOW", "TABLE", "THEN", "TO", "TRUE", "UNION",
    "UPDATE", "USING", "VALUES", "VIEW", "WHEN", "WHERE", "WINDOW", "WITH",
];

const POSTGRES_KEYWORDS: &[&str] = &["ILIKE", "LATERAL", "RETURNING", "SIMILAR"];
const MYSQL_KEYWORDS: &[&str] = &["DESCRIBE", "REGEXP", "STRAIGHT_JOIN"];
const SQLITE_KEYWORDS: &[&str] = &["GLOB", "PRAGMA", "RETURNING"];

/// Whether `word` (any case) is highlighted as a keyword in `dialect`.
pub fn is_keyword(dialect: Dialect, word: &str) -> bool {
    let upper = word.to_ascii_uppercase();
    let own = match dialect {
        Dialect::Postgres => POSTGRES_KEYWORDS,
        Dialect::MySql => MYSQL_KEYWORDS,
        Dialect::Sqlite => SQLITE_KEYWORDS,
    };
    KEYWORDS.contains(&upper.as_str()) || own.contains(&upper.as_str())
}

/// Splits `text` into tokens that cover it end to end.
pub fn tokenize(dialect: Dialect, text: &str) -> Vec<Token> {
    let bytes = text.as_bytes();
    let mut tokens = Vec::new();
    let mut at = 0;
    while at < bytes.len() {
        let start = at;
        let (kind, end) = next(dialect, text, start);
        debug_assert!(end > start && text.is_char_boundary(end));
        tokens.push(Token {
            kind,
            range: start..end,
        });
        at = end;
    }
    tokens
}

fn next(dialect: Dialect, text: &str, start: usize) -> (TokenKind, usize) {
    let bytes = text.as_bytes();
    let byte = bytes[start];
    let peek = |offset: usize| bytes.get(start + offset).copied();
    match byte {
        b if b.is_ascii_whitespace() => (
            TokenKind::Whitespace,
            scan(bytes, start, |b| b.is_ascii_whitespace()),
        ),
        b'-' if peek(1) == Some(b'-') && dash_comment(dialect, peek(2)) => {
            (TokenKind::Comment, line_end(bytes, start))
        }
        b'#' if dialect == Dialect::MySql => (TokenKind::Comment, line_end(bytes, start)),
        b'/' if peek(1) == Some(b'*') => {
            let kind = if dialect == Dialect::MySql && peek(2) == Some(b'!') {
                TokenKind::ExecutableComment
            } else {
                TokenKind::Comment
            };
            (kind, block_comment_end(bytes, start, dialect == Dialect::Postgres))
        }
        b'\'' => (
            TokenKind::String,
            quoted_end(bytes, start, b'\'', dialect == Dialect::MySql),
        ),
        b'E' | b'e'
            if dialect == Dialect::Postgres && peek(1) == Some(b'\'') =>
        {
            (TokenKind::String, quoted_end(bytes, start + 1, b'\'', true))
        }
        b'"' if dialect == Dialect::MySql => {
            (TokenKind::String, quoted_end(bytes, start, b'"', true))
        }
        b'"' => (
            TokenKind::QuotedIdentifier,
            quoted_end(bytes, start, b'"', false),
        ),
        b'`' if dialect != Dialect::Postgres => (
            TokenKind::QuotedIdentifier,
            quoted_end(bytes, start, b'`', false),
        ),
        b'[' if dialect == Dialect::Sqlite => {
            let end = bytes[start..]
                .iter()
                .position(|&b| b == b']')
                .map_or(bytes.len(), |offset| start + offset + 1);
            (TokenKind::QuotedIdentifier, end)
        }
        b'$' if dialect == Dialect::Postgres => match dollar_tag(bytes, start) {
            Some(tag_end) => {
                let tag = &bytes[start..tag_end];
                let body = find(bytes, tag_end, tag).map_or(bytes.len(), |at| at + tag.len());
                (TokenKind::String, body)
            }
            // `$1`: a parameter, one piece of punctuation.
            None => (
                TokenKind::Punctuation,
                scan(bytes, start + 1, |b| b.is_ascii_digit()),
            ),
        },
        b'0'..=b'9' => (TokenKind::Number, number_end(bytes, start)),
        b'.' if peek(1).is_some_and(|b| b.is_ascii_digit()) => {
            (TokenKind::Number, number_end(bytes, start))
        }
        b';' => (TokenKind::Semicolon, start + 1),
        b if is_word_start(b) => {
            let end = scan(bytes, start, |b| is_word_part(dialect, b));
            let kind = if is_keyword(dialect, &text[start..end]) {
                TokenKind::Keyword
            } else {
                TokenKind::Identifier
            };
            (kind, end)
        }
        b'+' | b'-' | b'*' | b'/' | b'<' | b'>' | b'=' | b'~' | b'!' | b'@' | b'#' | b'%'
        | b'^' | b'&' | b'|' | b'?' | b':' => (TokenKind::Operator, start + 1),
        _ => {
            // One whole character, however many bytes.
            let width = text[start..].chars().next().map_or(1, char::len_utf8);
            (TokenKind::Punctuation, start + width)
        }
    }
}

/// `--` starts a comment everywhere but MySQL, where whitespace or a
/// control character (or the end) must follow: `1--1` is arithmetic.
fn dash_comment(dialect: Dialect, after: Option<u8>) -> bool {
    dialect != Dialect::MySql || after.is_none_or(|b| b.is_ascii_whitespace() || b.is_ascii_control())
}

fn is_word_start(byte: u8) -> bool {
    byte.is_ascii_alphabetic() || byte == b'_' || byte >= 0x80
}

fn is_word_part(dialect: Dialect, byte: u8) -> bool {
    byte.is_ascii_alphanumeric()
        || byte == b'_'
        || byte >= 0x80
        || (byte == b'$' && dialect != Dialect::Sqlite)
}

fn scan(bytes: &[u8], from: usize, keep: impl Fn(u8) -> bool) -> usize {
    bytes[from..]
        .iter()
        .position(|&b| !keep(b))
        .map_or(bytes.len(), |offset| from + offset)
}

/// The end of a line comment: the newline stays outside it.
fn line_end(bytes: &[u8], from: usize) -> usize {
    scan(bytes, from, |b| b != b'\n')
}

fn find(bytes: &[u8], from: usize, needle: &[u8]) -> Option<usize> {
    bytes[from..]
        .windows(needle.len())
        .position(|window| window == needle)
        .map(|offset| from + offset)
}

fn block_comment_end(bytes: &[u8], start: usize, nests: bool) -> usize {
    let mut depth = 0usize;
    let mut at = start;
    while at + 1 < bytes.len() {
        match (bytes[at], bytes[at + 1]) {
            (b'/', b'*') if depth == 0 || nests => {
                depth += 1;
                at += 2;
            }
            (b'*', b'/') => {
                depth -= 1;
                at += 2;
                if depth == 0 {
                    return at;
                }
            }
            _ => at += 1,
        }
    }
    bytes.len()
}

/// A quoted run from `start` (the opening quote): a doubled quote stays
/// inside, and with `backslash` a backslash escapes the next byte.
fn quoted_end(bytes: &[u8], start: usize, quote: u8, backslash: bool) -> usize {
    let mut at = start + 1;
    while at < bytes.len() {
        match bytes[at] {
            b'\\' if backslash => at += 2,
            b if b == quote => {
                if bytes.get(at + 1) == Some(&quote) {
                    at += 2;
                } else {
                    return at + 1;
                }
            }
            _ => at += 1,
        }
    }
    bytes.len()
}

/// `$tag$` or `$$` at `start`: the end of the opening tag.
fn dollar_tag(bytes: &[u8], start: usize) -> Option<usize> {
    let tag_end = scan(bytes, start + 1, |b| b.is_ascii_alphanumeric() || b == b'_');
    let tag = &bytes[start + 1..tag_end];
    let starts_well = tag.first().is_none_or(|b| !b.is_ascii_digit());
    (starts_well && bytes.get(tag_end) == Some(&b'$')).then_some(tag_end + 1)
}

fn number_end(bytes: &[u8], start: usize) -> usize {
    let mut at = scan(bytes, start, |b| b.is_ascii_digit());
    if bytes.get(at) == Some(&b'.') {
        at = scan(bytes, at + 1, |b| b.is_ascii_digit());
    }
    if matches!(bytes.get(at), Some(b'e' | b'E')) {
        let sign = usize::from(matches!(bytes.get(at + 1), Some(b'+' | b'-')));
        if bytes.get(at + 1 + sign).is_some_and(u8::is_ascii_digit) {
            at = scan(bytes, at + 1 + sign, |b| b.is_ascii_digit());
        }
    }
    at
}
```

Note on `quoted_end` with `backslash`: a trailing backslash can step past the end; `at += 2` then `while at < len` exits and returns `bytes.len()`. It never slices, so no panic. Byte indexes stay on char boundaries because every rule stops at an ASCII byte or the end, and the fallback takes a whole char.

- [ ] **Step 4: Run the tests**

Run: `~/.cargo/bin/cargo test --locked -p tabletist-db --lib sql::`
Expected: PASS.

- [ ] **Step 5: Format, lint, commit**

```bash
~/.cargo/bin/cargo fmt --all && ~/.cargo/bin/cargo clippy --locked --workspace --all-targets -- -D warnings
git add crates/tabletist-db/src/sql.rs crates/tabletist-db/src/lib.rs
git commit -m "Tokenize SQL for each dialect

Strings, comments, dollar bodies and quoted names end where the database
says they do, so highlighting, splitting and the read-only guard agree.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 3: Split a script into statements

**Files:**
- Modify: `crates/tabletist-db/src/sql.rs`

**Interfaces:**
- Produces: `sql::Statement { range, end, start_line, first_line, text }`, `sql::statements(dialect, text) -> Vec<Statement>`, `sql::statement_at(&[Statement], cursor: usize) -> Option<&Statement>`, `Statement::line_col(&self, position: usize) -> (usize, usize)`, `sql::words(dialect, text) -> Vec<String>` (upper-cased word tokens, comments skipped).

- [ ] **Step 1: Write the failing tests** (append to `sql.rs` tests)

```rust
    fn texts(dialect: Dialect, text: &str) -> Vec<String> {
        statements(dialect, text).into_iter().map(|s| s.text).collect()
    }

    #[test]
    fn splits_on_semicolons_outside_strings_and_bodies() {
        let script = "SELECT ';'; SELECT $$a;b$$;\n-- only a comment;\nSELECT 3";
        assert_eq!(
            texts(Dialect::Postgres, script),
            // A leading comment belongs to the statement after it (its `;`
            // is inside the comment).
            vec!["SELECT ';'", "SELECT $$a;b$$", "-- only a comment;\nSELECT 3"]
        );
        assert_eq!(
            texts(Dialect::MySql, "SELECT `a;b`; SELECT \"c;d\""),
            vec!["SELECT `a;b`", "SELECT \"c;d\""]
        );
    }

    #[test]
    fn empty_and_comment_only_pieces_are_dropped() {
        assert!(statements(Dialect::Sqlite, " ;; -- nothing\n/* x */ ;").is_empty());
    }

    #[test]
    fn first_line_skips_leading_comments() {
        let script = "SELECT 1;\n\n-- counts\nSELECT\n  2;";
        let found = statements(Dialect::Postgres, script);
        assert_eq!(found[0].first_line, 1);
        assert_eq!(found[1].start_line, 3);
        assert_eq!(found[1].first_line, 4);
        assert_eq!(found[1].text, "-- counts\nSELECT\n  2");
    }

    #[test]
    fn the_statement_at_the_cursor() {
        let script = "SELECT 1;  SELECT 2;\n\nSELECT 3";
        let found = statements(Dialect::Postgres, script);
        let at = |cursor| statement_at(&found, cursor).map(|s| s.text.as_str());
        assert_eq!(at(0), Some("SELECT 1"));
        assert_eq!(at(8), Some("SELECT 1")); // just before the ;
        assert_eq!(at(9), Some("SELECT 1")); // just after it
        assert_eq!(at(10), Some("SELECT 1")); // between: the one before
        assert_eq!(at(14), Some("SELECT 2"));
        assert_eq!(at(21), Some("SELECT 2")); // blank line after it
        assert_eq!(at(script.len()), Some("SELECT 3"));
        let leading = statements(Dialect::Postgres, "\n\n  SELECT 1");
        assert_eq!(statement_at(&leading, 0).map(|s| s.first_line), Some(3));
        assert!(statement_at(&[], 0).is_none());
    }

    #[test]
    fn a_position_maps_to_a_line_and_column() {
        let script = "SELECT 0;\nSELECT a,\n  bogus FROM t";
        let second = &statements(Dialect::Postgres, script)[1];
        // PostgreSQL positions are 1-based characters in the statement.
        let position = second.text.find("bogus").unwrap() + 1;
        assert_eq!(second.line_col(position), (3, 3));
        assert_eq!(second.line_col(1), (2, 1));
    }

    #[test]
    fn an_unterminated_quoted_name_does_not_panic() {
        assert_eq!(words(Dialect::Postgres, "SELECT \"Zoë"), vec!["SELECT", "ZOë"]);
        assert_eq!(words(Dialect::Sqlite, "[Zoë"), vec!["ZOë"]);
    }

    #[test]
    fn words_skip_comments_and_upper_case() {
        assert_eq!(
            words(Dialect::MySql, "/* x */ set @@session.`tx_read_only` = 0"),
            vec!["SET", "SESSION", "TX_READ_ONLY"]
        );
    }
```

- [ ] **Step 2: Run to see them fail**

Run: `~/.cargo/bin/cargo test --locked -p tabletist-db --lib sql::`
Expected: FAIL to compile (`statements`, `statement_at`, `words` missing).

- [ ] **Step 3: Implement**

Add to `sql.rs` (above the tests):

```rust
/// One statement of a script, as the editor shows it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Statement {
    /// Byte range in the script, without surrounding whitespace or the `;`.
    pub range: Range<usize>,
    /// Byte offset just past its `;`, or `range.end` when it has none.
    pub end: usize,
    /// 1-based line of `range.start`.
    pub start_line: usize,
    /// Characters before `range.start` on its line.
    pub start_column: usize,
    /// 1-based line of its first token that is not a comment: the SQL,
    /// not a comment above it.
    pub first_line: usize,
    /// The script's text in `range`.
    pub text: String,
}

impl Statement {
    /// The 1-based line and column in the script of a 1-based character
    /// `position` in `text` (as PostgreSQL reports error positions).
    pub fn line_col(&self, position: usize) -> (usize, usize) {
        let before: String = self.text.chars().take(position.saturating_sub(1)).collect();
        let lines = before.matches('\n').count();
        let column = before.rsplit('\n').next().map_or(0, |line| line.chars().count()) + 1;
        let first_column = if lines == 0 { self.start_column } else { 0 };
        (self.start_line + lines, column + first_column)
    }
}
```

The second statement in the test starts at column 0 of line 2, so `line_col(1)` is `(2, 1)`.

```rust
fn is_code(kind: TokenKind) -> bool {
    !matches!(
        kind,
        TokenKind::Whitespace | TokenKind::Comment | TokenKind::ExecutableComment
    )
}

fn line_of(text: &str, byte: usize) -> usize {
    text[..byte].matches('\n').count() + 1
}

/// The statements of `script`, split on `;` tokens. Pieces holding only
/// whitespace and comments are dropped.
pub fn statements(dialect: Dialect, script: &str) -> Vec<Statement> {
    let tokens = tokenize(dialect, script);
    let mut found = Vec::new();
    for piece in tokens.split_inclusive(|token| token.kind == TokenKind::Semicolon) {
        let body: &[Token] = match piece.last() {
            Some(last) if last.kind == TokenKind::Semicolon => &piece[..piece.len() - 1],
            _ => piece,
        };
        let Some(first_code) = body.iter().find(|token| is_code(token.kind)) else {
            continue;
        };
        let visible = body.iter().filter(|token| token.kind != TokenKind::Whitespace);
        let (Some(first), Some(last)) = (visible.clone().next(), visible.last()) else {
            continue;
        };
        let range = first.range.start..last.range.end;
        let end = piece.last().map_or(range.end, |token| token.range.end);
        let line_start = script[..range.start].rfind('\n').map_or(0, |at| at + 1);
        found.push(Statement {
            start_line: line_of(script, range.start),
            start_column: script[line_start..range.start].chars().count(),
            first_line: line_of(script, first_code.range.start),
            text: script[range.clone()].to_owned(),
            range,
            end,
        });
    }
    found
}

/// The statement Run executes for a cursor at byte `cursor`: the one whose
/// range, extended to its `;`, holds it; else the nearest one ending
/// before it; else the first after it.
pub fn statement_at(statements: &[Statement], cursor: usize) -> Option<&Statement> {
    statements
        .iter()
        .find(|statement| statement.range.start <= cursor && cursor <= statement.end)
        .or_else(|| statements.iter().rev().find(|statement| statement.end <= cursor))
        .or_else(|| statements.first())
}

/// The statement's words (keywords and names, quotes removed), upper-cased,
/// in order. Comments and strings are skipped.
pub fn words(dialect: Dialect, text: &str) -> Vec<String> {
    tokenize(dialect, text)
        .into_iter()
        .filter_map(|token| {
            let word = &text[token.range];
            match token.kind {
                TokenKind::Keyword | TokenKind::Identifier => Some(word.to_ascii_uppercase()),
                // Strip the quotes, not a byte count: an unterminated name can
                // end inside a multi-byte character.
                TokenKind::QuotedIdentifier => Some(
                    word.trim_start_matches(['"', '`', '['])
                        .trim_end_matches(['"', '`', ']'])
                        .to_ascii_uppercase(),
                ),
                _ => None,
            }
        })
        .collect()
}
```

The `split_inclusive` on a slice keeps each `;` with the piece before it.

- [ ] **Step 4: Run the tests**

Run: `~/.cargo/bin/cargo test --locked -p tabletist-db --lib sql::`
Expected: PASS.

- [ ] **Step 5: Format, lint, commit**

```bash
~/.cargo/bin/cargo fmt --all && ~/.cargo/bin/cargo clippy --locked --workspace --all-targets -- -D warnings
git add crates/tabletist-db/src/sql.rs
git commit -m "Split SQL scripts into statements

Each statement knows its range, the line of its first real token and
where its semicolon ends, so Run can pick the one at the cursor.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 4: The read-only guard's refusal list, and its errors

**Files:**
- Modify: `crates/tabletist-db/src/sql.rs`, `crates/tabletist-db/src/error.rs`

**Interfaces:**
- Produces: `sql::refusal(dialect, statement_text) -> Option<String>` (what is refused, for the message), `Error::Refused { line: usize, what: String }`, `Error::LeftReadOnly`; `Error::is_connection_lost` is true for `LeftReadOnly`.

- [ ] **Step 1: Write the failing tests**

Append to the `sql.rs` tests:

```rust
    #[test]
    fn transaction_and_session_statements_are_refused() {
        let refused = |dialect, text: &str| refusal(dialect, text);
        for text in [
            "BEGIN",
            "start transaction",
            "COMMIT",
            "end",
            "ROLLBACK",
            "abort",
            "SAVEPOINT a",
            "RELEASE a",
            "PREPARE TRANSACTION 'x'",
            "COMMIT PREPARED 'x'",
            "SET TRANSACTION READ WRITE",
            "SET SESSION CHARACTERISTICS AS TRANSACTION READ WRITE",
            "set default_transaction_read_only = off",
            r#"SET "default_transaction_read_only" = off"#,
            "RESET ALL",
            "reset default_transaction_read_only",
            "SET standard_conforming_strings = off",
            r#"SET U&"default_transaction_read\005fonly" = off"#,
            "SELECT set_config('standard_conforming_strings', 'off', false)",
            "SET client_encoding = 'SJIS'",
            "DISCARD ALL",
            "COPY users TO STDOUT",
            "/* hi */ commit",
        ] {
            assert!(refused(Dialect::Postgres, text).is_some(), "{text}");
        }
        for text in [
            "SET SESSION TRANSACTION READ WRITE",
            "SET GLOBAL TRANSACTION READ WRITE",
            "SET @@session.transaction_read_only = 0",
            "set `tx_read_only` = 0",
            "SET autocommit = 1",
            "XA START 'x'",
            "LOCK TABLES t READ",
            "UNLOCK TABLES",
            "CALL p()",
            "CREATE USER x",
            "ALTER USER x IDENTIFIED BY 'y'",
            "DROP USER x",
            "RENAME USER x TO y",
            "CREATE ROLE r",
            "DROP ROLE r",
            "GRANT ALL ON *.* TO x",
            "REVOKE ALL ON *.* FROM x",
            "SET PASSWORD = 'x'",
            "SET DEFAULT ROLE ALL TO x",
            "FLUSH PRIVILEGES",
            "INSTALL PLUGIN p SONAME 'p.so'",
            "UNINSTALL PLUGIN p",
            "SELECT * FROM t INTO OUTFILE '/tmp/x'",
            "SELECT 1 INTO DUMPFILE '/tmp/x'",
            "SELECT 1 /*! , 2 */",
            "SELECT 1 /*M! , 2 */",
            "SET sql_mode = 'ANSI_QUOTES'",
            "SET @@session.sql_mode = ''",
            "SET NAMES gbk",
            "SET CHARACTER SET gbk",
            "SET character_set_client = gbk",
            "RESET sql_mode",
        ] {
            assert!(refused(Dialect::MySql, text).is_some(), "{text}");
        }
        assert!(refused(Dialect::Sqlite, "BEGIN IMMEDIATE").is_some());
    }

    #[test]
    fn unterminated_names_are_not_refused_or_a_panic() {
        assert_eq!(refusal(Dialect::MySql, "SELECT `ë"), None);
    }

    #[test]
    fn look_alikes_are_allowed() {
        for (dialect, text) in [
            (Dialect::Postgres, "SELECT 'COMMIT'"),
            (Dialect::Postgres, "SELECT end_date FROM t"),
            (Dialect::Postgres, "SET search_path = public"),
            (Dialect::Postgres, "SELECT 1 -- COMMIT"),
            (Dialect::Postgres, "WITH x AS (SELECT 1) SELECT * FROM x"),
            (Dialect::MySql, r#"SELECT "into outfile""#),
            (Dialect::MySql, "SET time_zone = '+00:00'"),
            (Dialect::MySql, "SELECT * FROM users"),
            (Dialect::Sqlite, "PRAGMA table_info(users)"),
            (Dialect::Sqlite, "SELECT * FROM `call`"),
        ] {
            assert_eq!(refusal(dialect, text), None, "{text}");
        }
    }
```

Append to the tests in `crates/tabletist-db/src/error.rs` (create a `#[cfg(test)] mod tests` if there is none next to the existing ones):

```rust
    #[test]
    fn leaving_read_only_counts_as_a_lost_connection() {
        assert!(Error::LeftReadOnly.is_connection_lost());
        let refused = Error::Refused {
            line: 4,
            what: "COMMIT".into(),
        };
        assert!(!refused.is_connection_lost());
        assert_eq!(
            refused.to_string(),
            "line 4: Tabletist runs every query in a read-only transaction, so COMMIT is not allowed"
        );
    }
```

- [ ] **Step 2: Run to see them fail**

Run: `~/.cargo/bin/cargo test --locked -p tabletist-db --lib`
Expected: FAIL to compile (`refusal`, `Error::Refused`, `Error::LeftReadOnly` missing).

- [ ] **Step 3: Implement**

In `error.rs`, add variants to `Error` (after `Cancelled`):

```rust
    /// A script holds a statement that could end or change the read-only
    /// transaction; nothing ran.
    #[error(
        "line {line}: Tabletist runs every query in a read-only transaction, so {what} is not allowed"
    )]
    Refused { line: usize, what: String },
    /// A script left the session read-write. The session is closed.
    #[error("the script left the read-only transaction, so the session was closed")]
    LeftReadOnly,
```

and make `is_connection_lost` cover it:

```rust
    /// Whether the session is unusable and must be reconnected.
    pub fn is_connection_lost(&self) -> bool {
        matches!(self, Self::ConnectionLost(_) | Self::LeftReadOnly)
    }
```

In `sql.rs`, add:

```rust
/// Why `statement` must not run in Tabletist's read-only transaction: the
/// statement kind it is, for the message. `None` when it may run. Matched
/// on tokens, so `SELECT 'COMMIT'` and a column named `end_date` pass.
pub fn refusal(dialect: Dialect, statement: &str) -> Option<String> {
    if dialect == Dialect::MySql
        && tokenize(dialect, statement)
            .iter()
            .any(|token| token.kind == TokenKind::ExecutableComment)
    {
        return Some("a /*! */ comment".into());
    }
    let words = words(dialect, statement);
    let word = |index: usize| words.get(index).map(String::as_str).unwrap_or_default();
    let named = |pairs: &[(&str, &str)]| {
        pairs
            .iter()
            .find(|(first, second)| word(0) == *first && word(1) == *second)
            .map(|(first, second)| format!("{first} {second}"))
    };
    // Settings that leave read-only, or that change how later statements
    // are lexed (the tokenizer assumes the connect-time values).
    let session_name = |name: &str| {
        name.ends_with("READ_ONLY")
            || name == "AUTOCOMMIT"
            || name == "SQL_MODE"
            || name == "STANDARD_CONFORMING_STRINGS"
            || name == "CLIENT_ENCODING"
            || name.starts_with("CHARACTER_SET")
    };
    match word(0) {
        first @ ("BEGIN" | "START" | "COMMIT" | "END" | "ROLLBACK" | "ABORT" | "SAVEPOINT"
        | "RELEASE") => return Some(first.to_owned()),
        "PREPARE" if word(1) == "TRANSACTION" => return Some("PREPARE TRANSACTION".into()),
        "SET" if words.iter().any(|w| w == "TRANSACTION" || w == "CHARACTERISTICS") => {
            return Some("SET TRANSACTION".into());
        }
        "SET" if matches!(word(1), "NAMES" | "CHARSET")
            || (word(1) == "CHARACTER" && word(2) == "SET") =>
        {
            return Some("SET NAMES".into());
        }
        "SET" if words.iter().skip(1).any(|w| session_name(w)) => {
            return Some(format!("SET {}", words.iter().skip(1).find(|w| session_name(w))?));
        }
        "RESET" if word(1) == "ALL" || words.iter().skip(1).any(|w| session_name(w)) => {
            return Some("RESET".into());
        }
        "DISCARD" if word(1) == "ALL" => return Some("DISCARD ALL".into()),
        "COPY" if dialect == Dialect::Postgres => return Some("COPY".into()),
        // A U&"..." name can spell a guarded setting in escapes.
        "SET" | "RESET" if dialect == Dialect::Postgres && unicode_name(statement) => {
            return Some(format!("{} with a U& name", word(0)));
        }
        _ => {}
    }
    // set_config() changes the same settings as SET, from any statement.
    if dialect == Dialect::Postgres && words.iter().any(|word| word == "SET_CONFIG") {
        return Some("set_config".into());
    }
    if dialect != Dialect::MySql {
        return None;
    }
    match word(0) {
        first @ ("XA" | "LOCK" | "UNLOCK" | "CALL" | "GRANT" | "REVOKE" | "FLUSH" | "INSTALL"
        | "UNINSTALL") => return Some(first.to_owned()),
        _ => {}
    }
    if let Some(found) = named(&[
        ("CREATE", "USER"),
        ("ALTER", "USER"),
        ("DROP", "USER"),
        ("RENAME", "USER"),
        ("CREATE", "ROLE"),
        ("DROP", "ROLE"),
        ("SET", "PASSWORD"),
        ("SET", "DEFAULT"),
    ]) {
        return Some(found);
    }
    words
        .windows(2)
        .find(|pair| pair[0] == "INTO" && (pair[1] == "OUTFILE" || pair[1] == "DUMPFILE"))
        .map(|pair| format!("INTO {}", pair[1]))
}
```

and the helper (a `U&` name tokenizes as the identifier `U`, the operator `&`, then a quoted identifier):

```rust
/// Whether a PostgreSQL statement names something with `U&"..."`.
fn unicode_name(statement: &str) -> bool {
    let tokens: Vec<Token> = tokenize(Dialect::Postgres, statement)
        .into_iter()
        .filter(|token| token.kind != TokenKind::Whitespace)
        .collect();
    tokens.windows(3).any(|three| {
        statement[three[0].range.clone()].eq_ignore_ascii_case("U")
            && &statement[three[1].range.clone()] == "&"
            && three[2].kind == TokenKind::QuotedIdentifier
    })
}
```

`words` skips strings, so `SELECT "into outfile"` (a MySQL string) passes. The `SET DEFAULT` pair catches `SET DEFAULT ROLE`.

- [ ] **Step 4: Run the tests**

Run: `~/.cargo/bin/cargo test --locked -p tabletist-db --lib`
Expected: PASS. If an existing test lists every `Error` variant (grep `Error::Timeout` in tests), add the two new ones there.

- [ ] **Step 5: Format, lint, commit**

```bash
~/.cargo/bin/cargo fmt --all && ~/.cargo/bin/cargo clippy --locked --workspace --all-targets -- -D warnings
git add crates/tabletist-db/src/sql.rs crates/tabletist-db/src/error.rs
git commit -m "Refuse statements that would leave the read-only transaction

Transaction control, session read-only switches, COPY, and on MySQL
procedures, account statements, file exports and executable comments.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 5: Script outcomes, `run_script` and `server_version` on SQLite

**Files:**
- Create: `crates/tabletist-db/src/script.rs`
- Modify: `crates/tabletist-db/src/lib.rs`, `crates/tabletist-db/src/spec.rs`, `crates/tabletist-db/src/sqlite.rs`, `crates/tabletist-db/Cargo.toml` (rusqlite `hooks` feature), `crates/tabletist-db/tests/sqlite.rs`

**Interfaces:**
- Produces: `ScriptOutcome { results: Vec<StatementResult> }` with `ScriptOutcome::was_cancelled()`; `StatementResult { elapsed, outcome }`; `StatementOutcome::{Rows { columns, rows, truncated }, Done { affected: Option<u64> }, Error { error: Error, position: Option<usize> }, Cancelled}`; `StopFlag` (`new`, `stop`, `is_stopped`, `Clone`, `Default`); `pub(crate) fn statement_failed(error, position) -> Result<StatementOutcome>`; `Connection::run_script(&self, statements: &[Statement], limit: u32, stop: &StopFlag) -> Result<ScriptOutcome>`; `Connection::server_version(&self) -> Result<String>`; `Driver::dialect(self) -> Dialect`. Re-export `ScriptOutcome, StatementOutcome, StatementResult, StopFlag` from `lib.rs`.

Spec note: the spec's `StatementOutcome::Error { message, position }` becomes `Error { error: Error, position }` so the Messages tab can show the code, detail and hint like `error_box` does; and `run_script` takes a `StopFlag` so a stop between statements (and SQLite's progress handler) never races a cancel.

- [ ] **Step 1: Write the failing tests**

Append to `crates/tabletist-db/tests/sqlite.rs` (the file already has `fixture()`, `#![allow(clippy::unwrap_used)]`, and imports from `tabletist_db`; add `sql`, `StatementOutcome`, `StopFlag`, `Dialect` to the `use`):

```rust
fn script(text: &str) -> Vec<tabletist_db::sql::Statement> {
    tabletist_db::sql::statements(Dialect::Sqlite, text)
}

#[tokio::test]
async fn a_script_returns_rows_with_types_and_truncates_at_the_limit() {
    let (connection, _dir) = fixture().await;
    let outcome = connection
        .run_script(&script("SELECT id, email FROM users ORDER BY id"), 3, &StopFlag::new())
        .await
        .unwrap();
    let [result] = outcome.results.as_slice() else {
        panic!("one result");
    };
    let StatementOutcome::Rows { columns, rows, truncated } = &result.outcome else {
        panic!("rows");
    };
    assert_eq!(columns[0].name, "id");
    assert_eq!(columns[0].kind, ValueKind::Numeric);
    assert_eq!(rows.len(), 3);
    assert!(*truncated);
}

#[tokio::test]
async fn truncates_at_the_limit_without_reading_the_whole_table() {
    let (connection, _dir) = fixture().await;
    let started = std::time::Instant::now();
    let outcome = connection
        .run_script(&script("SELECT * FROM big a, big b"), 10, &StopFlag::new())
        .await
        .unwrap();
    assert!(matches!(
        outcome.results[0].outcome,
        StatementOutcome::Rows { truncated: true, .. }
    ));
    assert!(started.elapsed() < std::time::Duration::from_secs(5));
}

#[tokio::test]
async fn a_script_stops_at_the_first_error_and_keeps_earlier_results() {
    let (connection, _dir) = fixture().await;
    let outcome = connection
        .run_script(
            &script("SELECT 1; SELECT nope FROM users; SELECT 3"),
            100,
            &StopFlag::new(),
        )
        .await
        .unwrap();
    assert_eq!(outcome.results.len(), 2);
    assert!(matches!(outcome.results[0].outcome, StatementOutcome::Rows { .. }));
    assert!(matches!(outcome.results[1].outcome, StatementOutcome::Error { .. }));
}

#[tokio::test]
async fn writes_fail_as_read_only_and_refusals_run_nothing() {
    let (connection, _dir) = fixture().await;
    let outcome = connection
        .run_script(&script("DELETE FROM users"), 100, &StopFlag::new())
        .await
        .unwrap();
    assert!(matches!(outcome.results[0].outcome, StatementOutcome::Error { .. }));
    let refused = connection
        .run_script(&script("SELECT 1;\nCOMMIT"), 100, &StopFlag::new())
        .await;
    assert!(matches!(refused, Err(Error::Refused { line: 2, .. })));
    let count = connection.count_rows(&users(10)).await.unwrap();
    assert_eq!(count, 5);
}

#[tokio::test]
async fn a_statement_without_rows_is_done() {
    let (connection, _dir) = fixture().await;
    let outcome = connection
        .run_script(&script("PRAGMA foreign_keys = ON"), 100, &StopFlag::new())
        .await
        .unwrap();
    assert!(matches!(outcome.results[0].outcome, StatementOutcome::Done { .. }));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_stopped_script_is_cancelled_and_keeps_earlier_results() {
    let (connection, _dir) = fixture().await;
    let connection = std::sync::Arc::new(connection);
    let stop = StopFlag::new();
    let running = {
        let connection = std::sync::Arc::clone(&connection);
        let stop = stop.clone();
        tokio::spawn(async move {
            let statements = script(
                "SELECT 1; WITH RECURSIVE n(i) AS (SELECT 1 UNION ALL SELECT i + 1 FROM n) SELECT count(*) FROM n",
            );
            connection.run_script(&statements, 10, &stop).await
        })
    };
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    stop.stop();
    let outcome = running.await.unwrap().unwrap();
    assert!(matches!(outcome.results[0].outcome, StatementOutcome::Rows { .. }));
    assert!(matches!(outcome.results[1].outcome, StatementOutcome::Cancelled));
    assert!(outcome.was_cancelled());
    // The session still works.
    connection.run_script(&script("SELECT 1"), 1, &StopFlag::new()).await.unwrap();
}

#[tokio::test]
async fn the_server_version_names_sqlite() {
    let (connection, _dir) = fixture().await;
    let version = connection.server_version().await.unwrap();
    assert!(version.starts_with("SQLite 3."), "{version}");
}
```

In `crates/tabletist-db/src/spec.rs` tests (or a new test in `lib.rs`), add:

```rust
    #[test]
    fn each_driver_has_its_dialect() {
        assert_eq!(Driver::Postgres.dialect(), crate::Dialect::Postgres);
        assert_eq!(Driver::MySql.dialect(), crate::Dialect::MySql);
        assert_eq!(Driver::Sqlite.dialect(), crate::Dialect::Sqlite);
    }
```

- [ ] **Step 2: Run to see them fail**

Run: `~/.cargo/bin/cargo test --locked -p tabletist-db`
Expected: FAIL to compile (`run_script`, `StopFlag`, `StatementOutcome`, `Driver::dialect` missing).

- [ ] **Step 3: Implement the shared types**

Create `crates/tabletist-db/src/script.rs`:

```rust
//! Running a script typed into the SQL editor: what each statement did,
//! and how a run is told to stop.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use crate::{ColumnMeta, Error, Result, Value};

/// What a script did: one result per statement that started, in order.
/// After an `Error` or `Cancelled` outcome no further statement runs.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ScriptOutcome {
    pub results: Vec<StatementResult>,
}

impl ScriptOutcome {
    /// Whether a stop ended the run: a statement was cancelled, or it was
    /// stopped before any statement began.
    pub fn was_cancelled(&self) -> bool {
        self.results.is_empty()
            || self
                .results
                .iter()
                .any(|result| result.outcome == StatementOutcome::Cancelled)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct StatementResult {
    pub elapsed: Duration,
    pub outcome: StatementOutcome,
}

#[derive(Debug, Clone, PartialEq)]
pub enum StatementOutcome {
    /// At most `limit` rows; `truncated` says more existed.
    Rows {
        columns: Vec<ColumnMeta>,
        rows: Vec<Vec<Value>>,
        truncated: bool,
    },
    /// A statement without a result set, with the rows it affected when
    /// the database says.
    Done { affected: Option<u64> },
    /// The statement failed. `position` is a 1-based character position in
    /// the statement's text (PostgreSQL reports one).
    Error {
        error: Error,
        position: Option<usize>,
    },
    /// Stopped while this statement ran. Its rows are dropped.
    Cancelled,
}

/// Tells a running script to stop: checked between statements, and by
/// SQLite while a statement runs. The backend sets it together with the
/// session's cancel.
#[derive(Debug, Clone, Default)]
pub struct StopFlag(Arc<AtomicBool>);

impl StopFlag {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn stop(&self) {
        self.0.store(true, Ordering::SeqCst);
    }

    pub fn is_stopped(&self) -> bool {
        self.0.load(Ordering::SeqCst)
    }
}

/// A statement's failure as its outcome: a cancel is `Cancelled`, a lost
/// session ends the whole run, anything else is the statement's error.
pub(crate) fn statement_failed(error: Error, position: Option<usize>) -> Result<StatementOutcome> {
    match error {
        Error::Cancelled => Ok(StatementOutcome::Cancelled),
        error if error.is_connection_lost() => Err(error),
        error => Ok(StatementOutcome::Error { error, position }),
    }
}

/// A cleanup failure closes the session: it may still be inside the
/// script's transaction.
pub(crate) fn cleanup_failed(error: &Error) -> Error {
    Error::ConnectionLost(format!("could not end the read-only transaction: {error}"))
}
```

In `lib.rs`: add `mod script;`, and

```rust
pub use script::{ScriptOutcome, StatementOutcome, StatementResult, StopFlag};
```

and on `Connection`:

```rust
    /// Runs `statements` in order in one read-only transaction that is
    /// always rolled back, keeping at most `limit` rows per statement.
    /// Refuses the whole script, running nothing, when a statement could
    /// leave the transaction (see [`sql::refusal`]). `stop` ends the run
    /// between statements (and, on SQLite, inside one); the caller also
    /// fires [`CancelHandle::cancel`] for a statement already running.
    pub async fn run_script(
        &self,
        statements: &[sql::Statement],
        limit: u32,
        stop: &StopFlag,
    ) -> Result<ScriptOutcome> {
        let dialect = self.dialect();
        for statement in statements {
            if let Some(what) = sql::refusal(dialect, &statement.text) {
                return Err(Error::Refused {
                    line: statement.first_line,
                    what,
                });
            }
        }
        if statements.is_empty() {
            return Ok(ScriptOutcome::default());
        }
        let texts: Vec<String> = statements.iter().map(|s| s.text.clone()).collect();
        match &self.inner {
            Inner::Sqlite(conn) => conn.run_script(texts, limit, stop).await,
            Inner::Postgres(conn) => conn.run_script(&texts, limit, stop).await,
            Inner::MySql(conn) => conn.run_script(&texts, limit, stop).await,
        }
    }

    /// The server's name and version for the footer: `PostgreSQL 17.2`,
    /// `MySQL 8.4.3`, `MariaDB 10.11.6`, `SQLite 3.46.0`.
    pub async fn server_version(&self) -> Result<String> {
        match &self.inner {
            Inner::Sqlite(conn) => conn.server_version().await,
            Inner::Postgres(conn) => conn.server_version().await,
            Inner::MySql(conn) => conn.server_version().await,
        }
    }
```

Until Tasks 6 and 7 land, give `pg::Conn` and `mysql::Conn` placeholder methods returning `Err(Error::Unsupported("the SQL editor on this database"))` for both, so the crate compiles; those tasks replace them.

In `spec.rs`, inside `impl Driver`:

```rust
    /// The SQL dialect the driver speaks.
    pub fn dialect(self) -> crate::Dialect {
        match self {
            Self::Postgres => crate::Dialect::Postgres,
            Self::MySql => crate::Dialect::MySql,
            Self::Sqlite => crate::Dialect::Sqlite,
        }
    }
```

- [ ] **Step 4: Implement SQLite**

In `crates/tabletist-db/Cargo.toml`, change the rusqlite line and its comment:

```toml
# SQLite, compiled in so every platform gets the same, recent version.
# column_decltype gives each result column's declared type; hooks gives the
# progress handler that stops a SQL editor script.
rusqlite = { version = "0.37", features = ["bundled", "column_decltype", "hooks"] }
```

In `sqlite.rs`, first extract the column building from `fetch_rows` into a helper (and use it there):

```rust
/// Result columns from their declared types; a column without one takes
/// its kind from the first value that is not NULL.
fn column_metas(declared: Vec<(String, String)>, rows: &[Vec<Value>]) -> Vec<ColumnMeta> {
    declared
        .into_iter()
        .enumerate()
        .map(|(index, (name, type_name))| {
            let kind = if type_name.is_empty() {
                rows.iter()
                    .map(|row| &row[index])
                    .find(|value| !value.is_null())
                    .map_or(ValueKind::Other, ValueKind::of_value)
            } else {
                ValueKind::from_sqlite_decl(&type_name)
            };
            ColumnMeta {
                name,
                type_name,
                kind,
            }
        })
        .collect()
}
```

Then add to `impl Conn`:

```rust
    /// See [`crate::Connection::run_script`]. The whole script is one
    /// blocking job; a progress handler checks `stop` while a statement
    /// runs.
    pub async fn run_script(
        &self,
        texts: Vec<String>,
        limit: u32,
        stop: &StopFlag,
    ) -> Result<ScriptOutcome> {
        let stop = stop.clone();
        let limit = limit as usize;
        self.run(move |connection| {
            let watching = stop.clone();
            connection.progress_handler(1_000, Some(move || watching.is_stopped()));
            let outcome = script(connection, &texts, limit, &stop);
            connection.progress_handler(0, None::<fn() -> bool>);
            outcome
        })
        .await
    }

    pub async fn server_version(&self) -> Result<String> {
        self.run(|connection| {
            let version: String = connection
                .query_row("SELECT sqlite_version()", [], |row| row.get(0))
                .map_err(map_error)?;
            Ok(format!("SQLite {version}"))
        })
        .await
    }
```

and the free functions:

```rust
fn script(
    connection: &rusqlite::Connection,
    texts: &[String],
    limit: usize,
    stop: &StopFlag,
) -> Result<ScriptOutcome> {
    let mut outcome = ScriptOutcome::default();
    match connection.execute_batch("BEGIN DEFERRED").map_err(map_error) {
        Err(Error::Cancelled) => return Ok(outcome),
        Err(error) => return Err(error),
        Ok(()) => {}
    }
    let ran = statements(connection, texts, limit, stop, &mut outcome);
    // An error can end SQLite's transaction by itself; roll back only one
    // that is still open.
    let rolled_back = if connection.is_autocommit() {
        Ok(())
    } else {
        connection
            .execute_batch("ROLLBACK")
            .or_else(|_| connection.execute_batch("ROLLBACK"))
            .map_err(map_error)
    };
    ran?;
    rolled_back.map_err(|error| crate::script::cleanup_failed(&error))?;
    Ok(outcome)
}

fn statements(
    connection: &rusqlite::Connection,
    texts: &[String],
    limit: usize,
    stop: &StopFlag,
    outcome: &mut ScriptOutcome,
) -> Result<()> {
    for text in texts {
        if stop.is_stopped() {
            outcome.results.push(StatementResult {
                elapsed: std::time::Duration::ZERO,
                outcome: StatementOutcome::Cancelled,
            });
            break;
        }
        let started = Instant::now();
        let result = match statement(connection, text, limit) {
            Ok(result) => result,
            Err(error) => crate::script::statement_failed(error, None)?,
        };
        let last = matches!(
            result,
            StatementOutcome::Error { .. } | StatementOutcome::Cancelled
        );
        outcome.results.push(StatementResult {
            elapsed: started.elapsed(),
            outcome: result,
        });
        if last {
            break;
        }
    }
    Ok(())
}

fn statement(connection: &rusqlite::Connection, text: &str, limit: usize) -> Result<StatementOutcome> {
    // prepare refuses a second statement in the text (MultipleStatement).
    let mut statement = connection.prepare(text).map_err(map_error)?;
    let declared: Vec<(String, String)> = statement
        .columns()
        .iter()
        .map(|column| {
            (
                column.name().to_owned(),
                column.decl_type().unwrap_or_default().to_owned(),
            )
        })
        .collect();
    if declared.is_empty() {
        let changed = statement.raw_execute().map_err(map_error)?;
        return Ok(StatementOutcome::Done {
            affected: Some(changed as u64),
        });
    }
    let mut rows = statement.raw_query();
    let mut values: Vec<Vec<Value>> = Vec::new();
    while let Some(row) = rows.next().map_err(map_error)? {
        let mut cells = Vec::with_capacity(declared.len());
        for index in 0..declared.len() {
            cells.push(from_sqlite(row.get_ref(index).map_err(map_error)?));
        }
        values.push(cells);
        if values.len() > limit {
            break;
        }
    }
    let truncated = values.len() > limit;
    values.truncate(limit);
    Ok(StatementOutcome::Rows {
        columns: column_metas(declared, &values),
        rows: values,
        truncated,
    })
}
```

Imports to add in `sqlite.rs`: `use crate::{ScriptOutcome, StatementOutcome, StatementResult, StopFlag};`. Check the `progress_handler` signature in rusqlite 0.37 (`~/.cargo/registry/src/*/rusqlite-0.37*/src/hooks/mod.rs`): it takes `num_ops: c_int` and `Option<F>` with `F: FnMut() -> bool + Send + 'static`; returning `true` interrupts with `SQLITE_INTERRUPT`, which `map_error` turns into `Error::Cancelled`. `rusqlite::Error::MultipleStatement` maps through `map_error` to a query error, so a hidden second statement becomes the statement's error.

- [ ] **Step 5: Run the tests**

Run: `~/.cargo/bin/cargo test --locked -p tabletist-db`
Expected: PASS (PostgreSQL and MySQL suites print "skipped" without their URLs).

- [ ] **Step 6: Format, lint, commit**

```bash
~/.cargo/bin/cargo fmt --all && ~/.cargo/bin/cargo clippy --locked --workspace --all-targets -- -D warnings
git add crates/tabletist-db
git commit -m "Run SQL scripts on SQLite in a rolled-back transaction

Each statement reports rows (up to the limit), a count or its error;
a stop flag ends the run between and inside statements.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 6: `run_script` and `server_version` on PostgreSQL

**Files:**
- Modify: `crates/tabletist-db/src/pg.rs`, `crates/tabletist-db/Cargo.toml` (`futures-util`), `crates/tabletist-db/tests/postgres.rs`

**Interfaces:**
- Consumes: Task 5's types, `pg::query_error`.
- Produces: `pg::Conn::run_script(&self, texts: &[String], limit: u32, stop: &StopFlag)`, `pg::Conn::server_version(&self)`.

- [ ] **Step 1: Write the failing integration tests**

Append to `crates/tabletist-db/tests/postgres.rs` (it has `connect()`, `load_fixture()`, `url()`; add the imports as in Task 5). The fixture has `public.users` (5 rows). Add a writable helper next to `load_fixture`:

```rust
async fn admin() -> tokio_postgres::Client {
    let mut config: tokio_postgres::Config = url().unwrap().parse().unwrap();
    config.ssl_mode(tokio_postgres::config::SslMode::Disable);
    let (client, connection) = config.connect(tokio_postgres::NoTls).await.unwrap();
    tokio::spawn(connection);
    client
}

fn script(text: &str) -> Vec<tabletist_db::sql::Statement> {
    tabletist_db::sql::statements(Dialect::Postgres, text)
}

async fn run(connection: &Connection, text: &str, limit: u32) -> tabletist_db::Result<ScriptOutcome> {
    connection.run_script(&script(text), limit, &StopFlag::new()).await
}

#[tokio::test]
async fn a_script_has_typed_columns_and_truncates_at_the_limit() {
    let Some(connection) = connect().await else {
        return;
    };
    let outcome = run(&connection, "SELECT id, active FROM users ORDER BY id", 2)
        .await
        .unwrap();
    let StatementOutcome::Rows { columns, rows, truncated } = &outcome.results[0].outcome else {
        panic!("rows");
    };
    assert_eq!(columns[1].kind, ValueKind::Bool);
    assert_eq!(rows.len(), 2);
    assert!(*truncated);
}

#[tokio::test]
async fn truncates_at_the_limit() {
    let Some(connection) = connect().await else {
        return;
    };
    let started = std::time::Instant::now();
    // In the select list, generate_series streams (in FROM it would
    // materialise every row first).
    let outcome = run(&connection, "SELECT generate_series(1, 50000000) AS g", 10)
        .await
        .unwrap();
    assert!(matches!(
        outcome.results[0].outcome,
        StatementOutcome::Rows { truncated: true, .. }
    ));
    assert!(started.elapsed() < std::time::Duration::from_secs(5));
}

#[tokio::test]
async fn unions_in_parentheses_and_trailing_comments_use_the_cursor() {
    let Some(connection) = connect().await else {
        return;
    };
    let outcome = run(
        &connection,
        "(SELECT 1 AS n) UNION ALL (SELECT 2) -- note",
        1,
    )
    .await
    .unwrap();
    assert!(matches!(
        &outcome.results[0].outcome,
        StatementOutcome::Rows { rows, truncated: true, .. } if rows.len() == 1
    ));
}

#[tokio::test]
async fn show_and_statements_without_rows() {
    let Some(connection) = connect().await else {
        return;
    };
    let outcome = run(&connection, "SHOW search_path; SET LOCAL work_mem = '8MB'", 10)
        .await
        .unwrap();
    assert!(matches!(outcome.results[0].outcome, StatementOutcome::Rows { .. }));
    assert!(matches!(outcome.results[1].outcome, StatementOutcome::Done { .. }));
}

#[tokio::test]
async fn a_statement_error_keeps_the_session() {
    let Some(connection) = connect().await else {
        return;
    };
    let outcome = run(&connection, "SELECT 1;\nSELECT a,\n  bogus FROM users; SELECT 3", 10)
        .await
        .unwrap();
    assert_eq!(outcome.results.len(), 2);
    let StatementOutcome::Error { error, position } = &outcome.results[1].outcome else {
        panic!("error");
    };
    assert!(matches!(error, Error::Query { code: Some(code), .. } if code == "42703"));
    // Points at "a" (1-based, into the statement's own text).
    assert_eq!(*position, Some(8));
    // Not closed: the next run works.
    run(&connection, "SELECT 1", 1).await.unwrap();
}

#[tokio::test]
async fn a_declare_error_points_into_the_users_text() {
    let Some(connection) = connect().await else {
        return;
    };
    // Prepares (it is a valid WITH) but DECLARE refuses a data-modifying WITH.
    let outcome = run(
        &connection,
        "WITH gone AS (DELETE FROM users RETURNING id) SELECT * FROM gone",
        10,
    )
    .await
    .unwrap();
    assert!(matches!(
        &outcome.results[0].outcome,
        StatementOutcome::Error { position: None | Some(1..), .. }
    ));
}

#[tokio::test]
async fn bypasses_cannot_write() {
    let Some(connection) = connect().await else {
        return;
    };
    let admin = admin().await;
    admin
        .batch_execute("CREATE TABLE IF NOT EXISTS probe (n int); TRUNCATE probe;")
        .await
        .unwrap();
    for attempt in [
        "SET TRANSACTION READ WRITE; INSERT INTO probe VALUES (1)",
        "ROLLBACK; SET default_transaction_read_only = off; INSERT INTO probe VALUES (1)",
        "SET \"default_transaction_read_only\" = off; INSERT INTO probe VALUES (1)",
        "SELECT set_config('default_transaction_read_only', 'off', false); INSERT INTO probe VALUES (1)",
        "INSERT INTO probe VALUES (1)",
        "COPY probe FROM STDIN",
        "PREPARE s AS INSERT INTO probe VALUES (1); EXECUTE s",
        "DO $$ BEGIN INSERT INTO probe VALUES (1); END $$",
    ] {
        let _ = run(&connection, attempt, 10).await;
        let count: i64 = admin
            .query_one("SELECT count(*) FROM probe", &[])
            .await
            .unwrap()
            .get(0);
        assert_eq!(count, 0, "{attempt}");
    }
    // And browsing still reads, read-only.
    assert!(connection.fetch_rows(&users(1)).await.is_ok());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_cancelled_script_keeps_earlier_results_and_the_session() {
    let Some(connection) = connect().await else {
        return;
    };
    let connection = std::sync::Arc::new(connection);
    let cancel = connection.cancel_handle();
    let stop = StopFlag::new();
    let running = {
        let connection = std::sync::Arc::clone(&connection);
        let stop = stop.clone();
        tokio::spawn(async move {
            connection
                .run_script(&script("SELECT 1; SELECT pg_sleep(30)"), 10, &stop)
                .await
        })
    };
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    stop.stop();
    while !running.is_finished() {
        cancel.cancel().await.unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
    let outcome = running.await.unwrap().unwrap();
    assert!(matches!(outcome.results[0].outcome, StatementOutcome::Rows { .. }));
    assert!(matches!(outcome.results[1].outcome, StatementOutcome::Cancelled));
    run(&connection, "SELECT 1", 1).await.unwrap();
}

#[tokio::test]
async fn the_server_version_has_no_distribution_suffix() {
    let Some(connection) = connect().await else {
        return;
    };
    let version = connection.server_version().await.unwrap();
    assert!(version.starts_with("PostgreSQL "), "{version}");
    assert!(!version.contains('('), "{version}");
}
```

- [ ] **Step 2: Run to see them fail**

First add the `futures-util` line from Step 3 to `crates/tabletist-db/Cargo.toml` and run `~/.cargo/bin/cargo build -p tabletist-db` once without `--locked`, so Cargo.lock records it (commit Cargo.lock with this task). Then run (servers up): `TABLETIST_TEST_PG_URL=postgres://tabletist:tabletist@localhost:55432/tabletist ~/.cargo/bin/cargo test --locked -p tabletist-db --test postgres`
Expected: the new tests FAIL with `Unsupported` from the placeholder.

- [ ] **Step 3: Implement**

`Cargo.toml` (tabletist-db), after the tokio-postgres lines:

```toml
# Reading PostgreSQL's simple-query row stream one row at a time (already
# in the tree through tokio-postgres).
futures-util = { version = "0.3", default-features = false, features = ["std"] }
```

In `crates/tabletist-db/src/script.rs`, add the retry macro (first used here, so it lands with its first caller and `-D warnings` stays green):

```rust
/// Runs a cleanup step again once when a cancel meant for a statement
/// landed on it instead: `retry_cancelled!(rollback(&client))` evaluates
/// the expression (a future of `Result<T>`) a second time. A macro, not a
/// function taking an async closure: a closure borrowing the connection
/// makes the run's future not `Send`, and the backend spawns it.
macro_rules! retry_cancelled {
    ($step:expr) => {
        match $step.await {
            Err($crate::Error::Cancelled) => $step.await,
            other => other,
        }
    };
}
pub(crate) use retry_cancelled;
```

In `pg.rs`, extract two helpers from `fetch_rows` (and use them there):

```rust
fn column_metas(statement: &tokio_postgres::Statement) -> Vec<ColumnMeta> {
    statement
        .columns()
        .iter()
        .map(|column| ColumnMeta {
            name: column.name().to_owned(),
            type_name: column.type_().name().to_owned(),
            kind: ValueKind::from_pg_type(column.type_().name()),
        })
        .collect()
}

fn row_values(row: &tokio_postgres::SimpleQueryRow, columns: &[ColumnMeta]) -> Result<Vec<Value>> {
    columns
        .iter()
        .enumerate()
        .map(|(index, column)| {
            Ok(match row.try_get(index).map_err(unexpected)? {
                None => Value::Null,
                Some(text) => value_from_pg_text(&column.type_name, text),
            })
        })
        .collect()
}
```

Then add to `impl Conn` (replacing the placeholders):

```rust
    /// See [`crate::Connection::run_script`]. The transaction is managed by
    /// hand: `tokio_postgres::Transaction` has no streaming simple query.
    pub async fn run_script(
        &self,
        texts: &[String],
        limit: u32,
        stop: &StopFlag,
    ) -> Result<ScriptOutcome> {
        let client = self.client.lock().await;
        let mut outcome = ScriptOutcome::default();
        match client.batch_execute("BEGIN READ ONLY").await.map_err(query_error) {
            Err(Error::Cancelled) => {
                outcome.stopped = true;
                return Ok(outcome);
            }
            Err(error) => return Err(error),
            Ok(()) => {}
        }
        // Take the snapshot first: PostgreSQL then refuses to make the
        // transaction read-write, whatever a statement tries.
        match client.batch_execute("SELECT 1").await.map_err(query_error) {
            Ok(()) => {
                for text in texts {
                    if stop.is_stopped() {
                        outcome.stopped = true;
                        outcome.results.push(StatementResult {
                            elapsed: Duration::ZERO,
                            outcome: StatementOutcome::Cancelled,
                        });
                        break;
                    }
                    let started = Instant::now();
                    let result = match run_statement(&client, text, limit as usize, stop).await {
                        Ok(result) => result,
                        Err(error) => {
                            // The session is gone; nothing to roll back.
                            return Err(error);
                        }
                    };
                    let last = matches!(
                        result,
                        StatementOutcome::Error { .. } | StatementOutcome::Cancelled
                    );
                    outcome.results.push(StatementResult {
                        elapsed: started.elapsed(),
                        outcome: result,
                    });
                    if last {
                        break;
                    }
                }
            }
            Err(Error::Cancelled) => outcome.stopped = true,
            Err(error) => return Err(error),
        }
        if matches!(
            outcome.results.last().map(|result| &result.outcome),
            Some(StatementOutcome::Cancelled)
        ) {
            outcome.stopped = true;
        }
        // From here on a cancel would land on the cleanup: tell the
        // backend to stop repeating its cancel.
        stop.finish();
        // After an error or a cancel the transaction is aborted: it cannot
        // write, SHOW would fail, and ROLLBACK ends it.
        let aborted = outcome.results.is_empty()
            || matches!(
                outcome.results.last().map(|result| &result.outcome),
                Some(StatementOutcome::Error { .. } | StatementOutcome::Cancelled)
            );
        let read_only = if aborted {
            Ok(true)
        } else {
            retry_cancelled!(still_read_only(&client))
        };
        let rolled_back = retry_cancelled!(rollback(&client));
        match (read_only, rolled_back) {
            (Ok(false), _) => Err(Error::LeftReadOnly),
            (Err(error), _) | (_, Err(error)) => Err(cleanup_failed(&error)),
            (Ok(true), Ok(())) => Ok(outcome),
        }
    }

    pub async fn server_version(&self) -> Result<String> {
        let client = self.client.lock().await;
        let messages = client
            .simple_query("SHOW server_version")
            .await
            .map_err(query_error)?;
        let version = first_text(&messages).unwrap_or_default();
        // "17.2 (Debian 17.2-1.pgdg120+1)" reads as "17.2".
        let version = version.split_whitespace().next().unwrap_or_default();
        Ok(format!("PostgreSQL {version}"))
    }
```

Free functions in `pg.rs`:

```rust
const CURSOR_PREFIX: &str = "DECLARE tabletist_sql NO SCROLL CURSOR FOR ";

fn first_text(messages: &[SimpleQueryMessage]) -> Option<String> {
    messages.iter().find_map(|message| match message {
        SimpleQueryMessage::Row(row) => row.get(0).map(str::to_owned),
        _ => None,
    })
}

async fn rollback(client: &tokio_postgres::Client) -> Result<()> {
    client.batch_execute("ROLLBACK").await.map_err(query_error)
}

async fn still_read_only(client: &tokio_postgres::Client) -> Result<bool> {
    let messages = client
        .simple_query("SHOW transaction_read_only")
        .await
        .map_err(query_error)?;
    Ok(first_text(&messages).as_deref() != Some("off"))
}

/// A statement's failure as its outcome. `offset` is how many characters
/// of wrapping precede the user's text in what the server saw; `None`
/// drops the position (it points into other text).
fn failed(error: tokio_postgres::Error, offset: Option<usize>) -> Result<StatementOutcome> {
    use tokio_postgres::error::ErrorPosition;
    let position = error
        .as_db_error()
        .and_then(|db| match db.position()? {
            ErrorPosition::Original(position) => Some(*position as usize),
            ErrorPosition::Internal { .. } => None,
        })
        .zip(offset)
        .and_then(|(position, offset)| position.checked_sub(offset))
        .filter(|position| *position > 0);
    statement_failed(query_error(error), position)
}

/// Whether a statement that returns rows can run as a cursor.
fn cursor_statement(text: &str) -> bool {
    let tokens = crate::sql::tokenize(crate::Dialect::Postgres, text);
    let first = tokens.iter().find(|token| {
        !matches!(
            token.kind,
            crate::sql::TokenKind::Whitespace | crate::sql::TokenKind::Comment
        )
    });
    first.is_some_and(|token| {
        let word = text[token.range.clone()].to_ascii_uppercase();
        matches!(word.as_str(), "SELECT" | "VALUES" | "TABLE" | "WITH" | "(")
    })
}

async fn run_statement(
    client: &tokio_postgres::Client,
    text: &str,
    limit: usize,
    stop: &StopFlag,
) -> Result<StatementOutcome> {
    // Prepare first: it yields the column types and refuses a second
    // statement hidden in one piece. A prepare error is the outcome; the
    // text never runs another way.
    let prepared = match client.prepare(text).await {
        Ok(prepared) => prepared,
        Err(error) => return failed(error, Some(0)),
    };
    let columns = column_metas(&prepared);
    if columns.is_empty() {
        return match client.simple_query(text).await {
            Ok(messages) => Ok(StatementOutcome::Done {
                affected: messages.iter().find_map(|message| match message {
                    SimpleQueryMessage::CommandComplete(count) => Some(*count),
                    _ => None,
                }),
            }),
            Err(error) => failed(error, Some(0)),
        };
    }
    if cursor_statement(text) {
        // Its own call, and a newline, so a trailing -- comment ends there.
        let declare = format!("{CURSOR_PREFIX}{text}\n");
        if let Err(error) = client.batch_execute(&declare).await {
            return failed(error, Some(CURSOR_PREFIX.chars().count()));
        }
        if stop.is_stopped() {
            return Ok(StatementOutcome::Cancelled);
        }
        let fetched = client
            .simple_query(&format!("FETCH {} FROM tabletist_sql", limit + 1))
            .await;
        let messages = match fetched {
            Ok(messages) => messages,
            Err(error) => return failed(error, None),
        };
        if let Err(error) = client.batch_execute("CLOSE tabletist_sql").await {
            return failed(error, None);
        }
        let mut rows = Vec::new();
        for message in &messages {
            if let SimpleQueryMessage::Row(row) = message {
                rows.push(row_values(row, &columns)?);
            }
        }
        let truncated = rows.len() > limit;
        rows.truncate(limit);
        return Ok(StatementOutcome::Rows {
            columns,
            rows,
            truncated,
        });
    }
    // SHOW, EXPLAIN and the like: stream, keep limit + 1, drop the rest.
    use futures_util::StreamExt;
    let stream = match client.simple_query_raw(text).await {
        Ok(stream) => stream,
        Err(error) => return failed(error, Some(0)),
    };
    let mut stream = std::pin::pin!(stream);
    let mut rows = Vec::new();
    while let Some(message) = stream.next().await {
        match message {
            Ok(SimpleQueryMessage::Row(row)) if rows.len() <= limit => {
                rows.push(row_values(&row, &columns)?);
            }
            Ok(_) => {}
            Err(error) => return failed(error, Some(0)),
        }
    }
    let truncated = rows.len() > limit;
    rows.truncate(limit);
    Ok(StatementOutcome::Rows {
        columns,
        rows,
        truncated,
    })
}
```

Imports to add: `use crate::script::{cleanup_failed, retry_cancelled, statement_failed};` (the last is the macro) and `use crate::{ScriptOutcome, StatementOutcome, StatementResult, StopFlag};`. The `(` in `cursor_statement` is a `Punctuation` token whose text is `(`, so the match covers it.

The position test expects 8 for `a` in `SELECT a,\n  bogus FROM users`: PostgreSQL reports the first undefined column (`a` is undefined in `users` only if it does not exist; the fixture's `users` has no column `a`, so position 8 is `a`). If the fixture ever gains an `a` column, the error moves to `bogus`; adjust the expected position then.

- [ ] **Step 4: Run the tests**

Run: `TABLETIST_TEST_PG_URL=postgres://tabletist:tabletist@localhost:55432/tabletist ~/.cargo/bin/cargo test --locked -p tabletist-db --test postgres`
Expected: PASS. Then `~/.cargo/bin/cargo test --locked -p tabletist-db` without the variable: PASS with "skipped".

- [ ] **Step 5: Format, lint, commit**

```bash
~/.cargo/bin/cargo fmt --all && ~/.cargo/bin/cargo clippy --locked --workspace --all-targets -- -D warnings
git add crates/tabletist-db Cargo.lock
git commit -m "Run SQL scripts on PostgreSQL

Each statement is prepared first, row queries read limit + 1 rows through
a cursor, and the transaction is checked and rolled back on every path.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 7: `run_script` and `server_version` on MySQL

**Files:**
- Modify: `crates/tabletist-db/src/mysql.rs`, `crates/tabletist-db/tests/mysql.rs`

**Interfaces:**
- Produces: `mysql::Conn::run_script(&self, texts: &[String], limit: u32, stop: &StopFlag)`, `mysql::Conn::server_version(&self)`, `async fn prepare_session(conn: &mut mysql_async::Conn) -> mysql_async::Result<()>` (the connect-time statements, shared with the cleanup).

- [ ] **Step 1: Write the failing integration tests**

Append to `crates/tabletist-db/tests/mysql.rs` (`connect()` and `admin()` exist; add imports as in Task 5):

```rust
fn script(text: &str) -> Vec<tabletist_db::sql::Statement> {
    tabletist_db::sql::statements(Dialect::MySql, text)
}

async fn run(connection: &Connection, text: &str, limit: u32) -> tabletist_db::Result<ScriptOutcome> {
    connection.run_script(&script(text), limit, &StopFlag::new()).await
}

#[tokio::test]
async fn truncates_at_the_limit() {
    let Some(connection) = connect().await else {
        return;
    };
    for limit in [2, 3] {
        // Same text twice: mysql_async caches the prepared statement.
        let outcome = run(&connection, "SELECT id FROM users", limit).await.unwrap();
        assert!(matches!(
            &outcome.results[0].outcome,
            StatementOutcome::Rows { rows, truncated: true, .. } if rows.len() == limit as usize
        ));
    }
}

#[tokio::test]
async fn a_run_leaves_the_session_as_it_found_it() {
    let Some(connection) = connect().await else {
        return;
    };
    // A small limit, a changed time zone, and a failure at the end.
    let _ = run(&connection, "SET time_zone = '+05:00'; SELECT nope", 2).await;
    let outcome = run(
        &connection,
        "SELECT @@session.time_zone = @@global.time_zone, @@session.transaction_read_only",
        10,
    )
    .await
    .unwrap();
    let StatementOutcome::Rows { rows, .. } = &outcome.results[0].outcome else {
        panic!("rows");
    };
    assert_eq!(rows[0][0], Value::Int(1));
    assert_eq!(rows[0][1], Value::Int(1));
    // sql_select_limit is back to its default: describe and paging see
    // every row (users has more columns and rows than the limit of 2).
    let structure = connection.describe(&users(1).object).await.unwrap();
    assert!(structure.columns.len() > 3);
    let page = connection.fetch_rows(&users(5)).await.unwrap();
    assert_eq!(page.rows.len(), 5);
}

#[tokio::test]
async fn bypasses_cannot_write() {
    let Some(connection) = connect().await else {
        return;
    };
    let mut admin = admin().await;
    admin
        .query_drop("CREATE TABLE IF NOT EXISTS probe (n int)")
        .await
        .unwrap();
    admin.query_drop("TRUNCATE probe").await.unwrap();
    for attempt in [
        "COMMIT; SET SESSION TRANSACTION READ WRITE; INSERT INTO probe VALUES (1)",
        "SET @@session.transaction_read_only = 0; INSERT INTO probe VALUES (1)",
        "INSERT INTO probe VALUES (1)",
        "CALL nothing()",
        "/*!50000 COMMIT */ SELECT 1",
        "CREATE USER sneaky",
        "GRANT ALL ON *.* TO tabletist",
        "SELECT 1 INTO OUTFILE '/tmp/tabletist-probe'",
        "PREPARE s FROM 'INSERT INTO probe VALUES (1)'; EXECUTE s",
        "EXECUTE IMMEDIATE 'INSERT INTO probe VALUES (1)'",
        "SET @a = 1, NAMES gbk",
        "SET GLOBAL read_only = 0",
    ] {
        let _ = run(&connection, attempt, 10).await;
        let count: Option<i64> = admin.query_first("SELECT count(*) FROM probe").await.unwrap();
        assert_eq!(count, Some(0), "{attempt}");
    }
    let users: Option<i64> = admin
        .query_first("SELECT count(*) FROM mysql.user WHERE user = 'sneaky'")
        .await
        .unwrap_or(Some(0));
    assert_eq!(users, Some(0));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_cancelled_script_keeps_earlier_results_and_stays_read_only() {
    let Some(connection) = connect().await else {
        return;
    };
    let connection = std::sync::Arc::new(connection);
    let cancel = connection.cancel_handle();
    let stop = StopFlag::new();
    let running = {
        let connection = std::sync::Arc::clone(&connection);
        let stop = stop.clone();
        tokio::spawn(async move {
            connection
                .run_script(&script("SELECT 1; SELECT SLEEP(30) = 0"), 10, &stop)
                .await
        })
    };
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    stop.stop();
    // Keep cancelling until it ends: some cancels land on the cleanup,
    // which runs each step again once, so the session ends read-only or
    // closed, never read-write.
    while !running.is_finished() {
        cancel.cancel().await.unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
    match running.await.unwrap() {
        Ok(outcome) => {
            assert!(matches!(outcome.results[0].outcome, StatementOutcome::Rows { .. }));
            let check = run(&connection, "SELECT @@session.transaction_read_only", 1)
                .await
                .unwrap();
            assert!(matches!(
                &check.results[0].outcome,
                StatementOutcome::Rows { rows, .. } if rows[0][0] == Value::Int(1)
            ));
        }
        Err(error) => assert!(error.is_connection_lost(), "{error}"),
    }
}

#[tokio::test]
async fn the_server_version_names_the_server() {
    let Some(connection) = connect().await else {
        return;
    };
    let version = connection.server_version().await.unwrap();
    assert!(
        version.starts_with("MySQL ") || version.starts_with("MariaDB "),
        "{version}"
    );
    assert!(!version.contains('-'), "{version}");
}
```


- [ ] **Step 2: Run to see them fail**

Run: `TABLETIST_TEST_MYSQL_URL=mysql://tabletist:tabletist@localhost:53306/tabletist ~/.cargo/bin/cargo test --locked -p tabletist-db --test mysql`
Expected: the new tests FAIL (`Unsupported`).

- [ ] **Step 3: Implement**

In `mysql.rs`, move the connect-time statements into a function and call it from `connect` in their place:

```rust
/// The session settings every connection runs with. Run again after a
/// SQL editor script resets the session.
async fn prepare_session(conn: &mut mysql_async::Conn) -> mysql_async::Result<()> {
    // Fixed statements: safe to send through the text protocol.
    conn.query_drop("SET SESSION TRANSACTION READ ONLY").await?;
    conn.query_drop(
        "SET SESSION sql_mode = REPLACE(REPLACE(@@SESSION.sql_mode, 'NO_BACKSLASH_ESCAPES', ''), 'ANSI_QUOTES', '')",
    )
    .await
}
```

(in `connect`: `prepare_session(&mut conn).await.map_err(query_error)?;`)

Extract the column building from `read_page`:

```rust
fn column_metas(columns: &[mysql_async::Column]) -> Vec<ColumnMeta> {
    columns
        .iter()
        .map(|column| {
            let type_name = type_name(column.column_type(), column.flags(), column.character_set());
            ColumnMeta {
                name: column.name_str().into_owned(),
                kind: kind(&type_name),
                type_name: type_name.into_owned(),
            }
        })
        .collect()
}
```

Add to `impl Conn` (replacing the placeholders):

```rust
    /// See [`crate::Connection::run_script`]. Statements run through the
    /// prepared protocol, which cannot hold two; `sql_select_limit` makes
    /// the server stop at `limit + 1` rows. MySQL session state is not
    /// transactional, so the session is reset afterwards.
    pub async fn run_script(
        &self,
        texts: &[String],
        limit: u32,
        stop: &StopFlag,
    ) -> Result<ScriptOutcome> {
        let mut conn = self.conn.lock().await;
        let mut outcome = ScriptOutcome::default();
        let opened = match conn
            .query_drop("START TRANSACTION READ ONLY")
            .await
            .map_err(query_error)
        {
            Ok(()) => conn
                .query_drop(format!("SET SESSION sql_select_limit = {}", u64::from(limit) + 1))
                .await
                .map_err(query_error),
            Err(error) => Err(error),
        };
        match opened {
            Ok(()) => {
                for text in texts {
                    if stop.is_stopped() {
                        outcome.stopped = true;
                        outcome.results.push(StatementResult {
                            elapsed: Duration::ZERO,
                            outcome: StatementOutcome::Cancelled,
                        });
                        break;
                    }
                    let started = Instant::now();
                    let result = run_statement(&mut conn, text, limit as usize).await?;
                    let last = matches!(
                        result,
                        StatementOutcome::Error { .. } | StatementOutcome::Cancelled
                    );
                    outcome.results.push(StatementResult {
                        elapsed: started.elapsed(),
                        outcome: result,
                    });
                    if last {
                        break;
                    }
                }
            }
            Err(Error::Cancelled) => outcome.stopped = true,
            Err(error) if error.is_connection_lost() => return Err(error),
            Err(error) => {
                stop.finish();
                end_script(&mut conn).await?;
                return Err(error);
            }
        }
        if matches!(
            outcome.results.last().map(|result| &result.outcome),
            Some(StatementOutcome::Cancelled)
        ) {
            outcome.stopped = true;
        }
        stop.finish();
        end_script(&mut conn).await?;
        Ok(outcome)
    }

    pub async fn server_version(&self) -> Result<String> {
        let mut conn = self.conn.lock().await;
        let full: Option<String> = conn.query_first("SELECT VERSION()").await.map_err(query_error)?;
        let full = full.unwrap_or_default();
        let name = if full.contains("MariaDB") { "MariaDB" } else { "MySQL" };
        let number = full.split('-').next().unwrap_or_default();
        Ok(format!("{name} {number}"))
    }
```

Free functions:

```rust
fn failed(error: mysql_async::Error) -> Result<StatementOutcome> {
    statement_failed(query_error(error), None)
}

async fn run_statement(
    conn: &mut mysql_async::Conn,
    text: &str,
    limit: usize,
) -> Result<StatementOutcome> {
    let mut result = match conn.exec_iter(text, ()).await {
        Ok(result) => result,
        Err(error) => return failed(error),
    };
    let columns = column_metas(&result.columns_ref());
    if columns.is_empty() {
        let affected = result.affected_rows();
        if let Err(error) = result.drop_result().await {
            return failed(error);
        }
        return Ok(StatementOutcome::Done {
            affected: Some(affected),
        });
    }
    let mut rows = Vec::new();
    loop {
        match result.next().await {
            Ok(Some(row)) => {
                if rows.len() <= limit {
                    rows.push(
                        row.unwrap()
                            .into_iter()
                            .zip(&columns)
                            .map(|(cell, column)| value(cell, &column.type_name))
                            .collect(),
                    );
                }
            }
            Ok(None) => break,
            Err(error) => return failed(error),
        }
    }
    let truncated = rows.len() > limit;
    rows.truncate(limit);
    Ok(StatementOutcome::Rows {
        columns,
        rows,
        truncated,
    })
}

/// Whether the session is still read-only. MariaDB before 11.1 names the
/// variable `tx_read_only`.
async fn still_read_only(conn: &mut mysql_async::Conn) -> Result<bool> {
    let value = match conn
        .query_first::<i64, _>("SELECT @@session.transaction_read_only")
        .await
    {
        Err(mysql_async::Error::Server(error)) if error.code == UNKNOWN_SYSTEM_VARIABLE => conn
            .query_first::<i64, _>("SELECT @@session.tx_read_only")
            .await
            .map_err(query_error)?,
        other => other.map_err(query_error)?,
    };
    Ok(value != Some(0))
}

/// Checks the session, rolls back, resets it and applies the connect-time
/// settings again. A step a cancel interrupted runs once more; any other
/// failure closes the session.
async fn end_script(conn: &mut mysql_async::Conn) -> Result<()> {
    let read_only = retry_cancelled!(still_read_only(conn));
    let rolled_back = retry_cancelled!(async { conn.query_drop("ROLLBACK").await.map_err(query_error) });
    let reset = retry_cancelled!(async { conn.reset().await.map_err(query_error) });
    let prepared = retry_cancelled!(async { prepare_session(conn).await.map_err(query_error) });
    match read_only {
        Ok(false) => return Err(Error::LeftReadOnly),
        Err(error) => return Err(cleanup_failed(&error)),
        Ok(true) => {}
    }
    rolled_back.map_err(|error| cleanup_failed(&error))?;
    match reset {
        Ok(true) => {}
        // The server predates COM_RESET_CONNECTION.
        Ok(false) => {
            return Err(cleanup_failed(&Error::query(
                "the server cannot reset the session",
            )));
        }
        Err(error) => return Err(cleanup_failed(&error)),
    }
    prepared.map_err(|error| cleanup_failed(&error))
}
```

Imports: `use crate::script::{cleanup_failed, retry_cancelled, statement_failed};`, `use crate::{ScriptOutcome, StatementOutcome, StatementResult, StopFlag};`. Check `mysql_async::Conn::reset` in 0.37.1 (`conn/mod.rs`): `pub async fn reset(&mut self) -> Result<bool>`; `false` means the server lacks `COM_RESET_CONNECTION`. It keeps the connection id, so `KILL QUERY` still targets it. Each `retry_cancelled!` awaits its first future before building the second, so the mutable borrows of `conn` never overlap.

- [ ] **Step 4: Run the tests**

Run: `TABLETIST_TEST_MYSQL_URL=mysql://tabletist:tabletist@localhost:53306/tabletist ~/.cargo/bin/cargo test --locked -p tabletist-db --test mysql`
Expected: PASS, including the existing lock tests (`a_failed_query_leaves_no_transaction_holding_locks`). Then the whole crate without variables: PASS.

- [ ] **Step 5: Format, lint, commit**

```bash
~/.cargo/bin/cargo fmt --all && ~/.cargo/bin/cargo clippy --locked --workspace --all-targets -- -D warnings
git add crates/tabletist-db
git commit -m "Run SQL scripts on MySQL and reset the session after

Prepared statements only, sql_select_limit for the row limit, and a
session reset so nothing a script set reaches table browsing.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 8: Backend: `RunSql`, `ServerVersion`, cancel reasons and the timeout

**Files:**
- Modify: `src/backend.rs`

**Interfaces:**
- Produces: `Command::RunSql { session, request, statements: Vec<sql::Statement>, limit: u32, timeout: Option<Duration> }`, `Command::ServerVersion { session, request }`, `Event::SqlRan { session, request, result: Result<ScriptOutcome, Error>, cancel: Option<CancelReason> }`, `Event::ServerVersion { session, request, result: Result<String, Error> }`, `pub enum CancelReason { User, Timeout(Duration) }`, `StateFile::Settings(Settings)`; `Running.stop: Option<StopFlag>`.

- [ ] **Step 1: Write the failing tests** (in `backend.rs` tests, next to `cancel_stops_a_running_query_and_the_session_keeps_working`)

```rust
    fn statements(text: &str) -> Vec<tabletist_db::sql::Statement> {
        tabletist_db::sql::statements(tabletist_db::Dialect::Sqlite, text)
    }

    fn connected_sqlite() -> (tempfile::TempDir, Backend, SessionId) {
        let (dir, spec) = fixture();
        let mut backend = Backend::start_with(Waker::default(), Keyring::memory());
        let session = SessionId(1);
        backend.send(Command::Connect {
            session,
            request: RequestId(1),
            spec,
            secrets: Secrets::default(),
            host_keys: HostKeys::default(),
        });
        assert!(matches!(backend.wait(WAIT), Some(Event::Connected { .. })));
        (dir, backend, session)
    }

    const ENDLESS: &str =
        "WITH RECURSIVE n(i) AS (SELECT 1 UNION ALL SELECT i + 1 FROM n) SELECT count(*) FROM n";

    #[test]
    fn a_script_runs_and_answers_with_its_outcome() {
        let (_dir, mut backend, session) = connected_sqlite();
        backend.send(Command::RunSql {
            session,
            request: RequestId(2),
            statements: statements("SELECT 1; SELECT 2"),
            limit: 10,
            timeout: None,
        });
        let Some(Event::SqlRan {
            request: RequestId(2),
            result: Ok(outcome),
            cancel: None,
            ..
        }) = backend.wait(WAIT)
        else {
            panic!("expected SqlRan");
        };
        assert_eq!(outcome.results.len(), 2);
    }

    #[test]
    fn a_timeout_cancels_the_script_and_keeps_earlier_results() {
        let (_dir, mut backend, session) = connected_sqlite();
        backend.send(Command::RunSql {
            session,
            request: RequestId(2),
            statements: statements(&format!("SELECT 1; {ENDLESS}")),
            limit: 10,
            timeout: Some(Duration::from_millis(300)),
        });
        let Some(Event::SqlRan {
            result: Ok(outcome),
            cancel: Some(CancelReason::Timeout(limit)),
            ..
        }) = backend.wait(WAIT)
        else {
            panic!("expected a timed-out SqlRan");
        };
        assert_eq!(limit, Duration::from_millis(300));
        assert!(matches!(
            outcome.results[0].outcome,
            tabletist_db::StatementOutcome::Rows { .. }
        ));
        assert!(matches!(
            outcome.results[1].outcome,
            tabletist_db::StatementOutcome::Cancelled
        ));
        // The session keeps working.
        backend.send(Command::ListSchemas {
            session,
            request: RequestId(3),
        });
        assert!(matches!(
            backend.wait(WAIT),
            Some(Event::Schemas { result: Ok(_), .. })
        ));
    }

    #[test]
    fn a_user_cancel_stops_a_script() {
        let (_dir, mut backend, session) = connected_sqlite();
        backend.send(Command::RunSql {
            session,
            request: RequestId(2),
            statements: statements(ENDLESS),
            limit: 10,
            timeout: None,
        });
        std::thread::sleep(Duration::from_millis(200));
        backend.send(Command::Cancel {
            session,
            request: RequestId(2),
        });
        assert!(matches!(
            backend.wait(WAIT),
            Some(Event::SqlRan {
                cancel: Some(CancelReason::User),
                ..
            })
        ));
    }

    #[test]
    fn the_server_version_is_asked_for() {
        let (_dir, mut backend, session) = connected_sqlite();
        backend.send(Command::ServerVersion {
            session,
            request: RequestId(2),
        });
        assert!(matches!(
            backend.wait(WAIT),
            Some(Event::ServerVersion { result: Ok(version), .. }) if version.starts_with("SQLite")
        ));
    }

    #[test]
    fn a_closed_session_fails_a_script() {
        let (_dir, mut backend, session) = connected_sqlite();
        backend.send(Command::Close { session });
        backend.send(Command::RunSql {
            session,
            request: RequestId(2),
            statements: statements("SELECT 1"),
            limit: 10,
            timeout: None,
        });
        assert!(matches!(
            backend.wait(WAIT),
            Some(Event::SqlRan { result: Err(_), cancel: None, .. })
        ));
    }
```

- [ ] **Step 2: Run to see them fail**

Run: `~/.cargo/bin/cargo test --locked --lib backend::`
Expected: FAIL to compile.

- [ ] **Step 3: Implement**

Add to `Command`:

```rust
    /// Run a SQL editor script (see `Connection::run_script`). A timeout
    /// fires the session's cancel, like a user's Cancel.
    RunSql {
        session: SessionId,
        request: RequestId,
        statements: Vec<tabletist_db::sql::Statement>,
        limit: u32,
        timeout: Option<Duration>,
    },
    /// The server's name and version, for the SQL editor's footer.
    ServerVersion {
        session: SessionId,
        request: RequestId,
    },
```

Add to `Event`:

```rust
    SqlRan {
        session: SessionId,
        request: RequestId,
        result: Result<ScriptOutcome, Error>,
        /// Why the run was stopped, when a stop ended it.
        cancel: Option<CancelReason>,
    },
    ServerVersion {
        session: SessionId,
        request: RequestId,
        result: Result<String, Error>,
    },
```

and the type:

```rust
/// Who stopped a SQL editor run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CancelReason {
    User,
    Timeout(Duration),
}
```

`StateFile` gains `Settings(crate::settings::Settings)` with `Self::Settings(settings) => settings.save(path),` in `save`.

`Running` gains:

```rust
    /// The running SQL editor script's stop flag: a Cancel sets it too, so
    /// the script also stops between statements.
    stop: Option<StopFlag>,
```

In `Worker::handle`'s `Command::Cancel` arm, inside `if running.request == Some(request)`, after pushing the cancel task:

```rust
                        if let Some(stop) = &running.stop {
                            stop.stop();
                        }
```

Add `RunSql` and `ServerVersion` to `session_of` (`=> *session`), `request_of` (`=> Some(*request)`), and `fail`:

```rust
        Command::RunSql {
            session, request, ..
        } => Event::SqlRan {
            session,
            request,
            result: Err(error),
            cancel: None,
        },
        Command::ServerVersion { session, request } => Event::ServerVersion {
            session,
            request,
            result: Err(error),
        },
```

In `run_session`, create the stop flag in the same critical section that sets `running.request`, so a Cancel that arrives right after the run starts always finds it:

```rust
              if !skipped {
                  running.request = request;
                  running.stop = matches!(command, Command::RunSql { .. }).then(StopFlag::new);
              }
```

and clear it where `running.request` is cleared (`lock(&running).request = None;` becomes a block that also sets `stop = None`). Then add arms before the catch-all:

```rust
            Command::RunSql {
                session,
                request,
                statements,
                limit,
                timeout,
            } => {
                let stop = lock(&running).stop.clone().unwrap_or_default();
                let timed_out = Arc::new(AtomicBool::new(false));
                let timer = timeout.map(|after| {
                    let (stop, running, timed_out) =
                        (stop.clone(), Arc::clone(&running), Arc::clone(&timed_out));
                    let cancel = connection.cancel_handle();
                    tokio::spawn(async move {
                        tokio::time::sleep(after).await;
                        timed_out.store(true, Ordering::SeqCst);
                        stop.stop();
                        // A cancel can reach the server before the statement
                        // it is meant for; send again until the run reaches
                        // its cleanup (`finish`), where a cancel would only
                        // interrupt the rollback.
                        while !stop.is_finishing() {
                            let cancel = cancel.clone();
                            let task = tokio::spawn(async move {
                                let _ = cancel.cancel().await;
                            });
                            {
                                let mut running = lock(&running);
                                running.cancels.retain(|task| !task.is_finished());
                                running.cancels.push(task);
                            }
                            tokio::time::sleep(Duration::from_millis(500)).await;
                        }
                    })
                });
                let result = connection.run_script(&statements, limit, &stop).await;
                if let Some(timer) = timer {
                    timer.abort();
                }
                let stopped = stop.is_stopped()
                    && match &result {
                        Ok(outcome) => outcome.was_cancelled(),
                        Err(error) => *error == Error::Cancelled,
                    };
                let cancel = stopped.then(|| match timeout {
                    Some(after) if timed_out.load(Ordering::SeqCst) => CancelReason::Timeout(after),
                    _ => CancelReason::User,
                });
                let lost = lost_error(&result);
                outbox.emit(Event::SqlRan {
                    session,
                    request,
                    result,
                    cancel,
                });
                lost
            }
            Command::ServerVersion { session, request } => {
                let result = connection.server_version().await;
                let lost = lost_error(&result);
                outbox.emit(Event::ServerVersion {
                    session,
                    request,
                    result,
                });
                lost
            }
```

Imports: `use std::sync::atomic::{AtomicBool, Ordering};`, `use std::time::Duration;` (if absent), `use tabletist_db::{ScriptOutcome, StopFlag};`. `Error` derives `PartialEq`, so `*error == Error::Cancelled` compiles.

When `run_script` returns `Err(LeftReadOnly)` or a cleanup `ConnectionLost`, `lost_error` makes `run_session` fail the queue, emit `Disconnected` and close the connection: the reconnect banner follows with no new code.

Update `app.rs`'s `apply_event` with arms that do nothing yet (`Event::SqlRan { .. } | Event::ServerVersion { .. } => {}`) so it compiles; Task 11 fills them.

- [ ] **Step 4: Run the tests**

Run: `~/.cargo/bin/cargo test --locked --lib backend::`
Expected: PASS.

- [ ] **Step 5: Format, lint, commit**

```bash
~/.cargo/bin/cargo fmt --all && ~/.cargo/bin/cargo clippy --locked --workspace --all-targets -- -D warnings
git add src/backend.rs src/app.rs
git commit -m "Run SQL editor scripts on the backend with a timeout

A timeout or a Cancel sets the script's stop flag and fires the session's
cancel; the result says which stopped it.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 9: Workspace tabs as `Tab::Object` or `Tab::Sql`

**Files:**
- Modify: `src/model.rs`, `src/app.rs`, `src/shots.rs`, `src/ui/keys.rs`, `src/ui/object_tabs.rs`, `src/ui/mod.rs` (tests)

**Interfaces:**
- Produces: `Tab`, `SqlTab`, `SqlRun`, `ResultPane`; `Workspace { tabs: Vec<Tab>, next_query: u32, server_version: Fetch<String>, .. }` (replacing `objects`); `Workspace::{tab, tab_mut, object_tab, object_tab_mut, sql_tab, sql_tab_mut, active_object_tab, active_sql_tab, objects, objects_mut}`; `Tab::{id, as_object, as_object_mut, as_sql, as_sql_mut, pending, name}`; actions renamed `ActivateObjectTab`→`ActivateTab`, `CloseObjectTab`→`CloseTab`, `CycleObjectTab`→`CycleTab` (fields unchanged: `tab`, `object_tab`/`step`).

- [ ] **Step 1: Write the failing tests** (in `model.rs` tests)

```rust
    fn sql_tab(id: u64) -> Tab {
        Tab::Sql(SqlTab::new(TabId(id), 1, 1_000, Some(Duration::from_secs(30))))
    }

    #[test]
    fn a_workspace_finds_its_tabs_by_kind() {
        let mut workspace = crate::testing::workspace();
        workspace.tabs.push(Tab::Object(Box::new(ObjectTab::new(
            TabId(1),
            ObjectRef::new("main", "users"),
            ObjectKind::Table,
            true,
            100,
            None,
        ))));
        workspace.tabs.push(sql_tab(2));
        workspace.active_tab = Some(TabId(2));
        assert!(workspace.object_tab(TabId(1)).is_some());
        assert!(workspace.object_tab(TabId(2)).is_none());
        assert!(workspace.sql_tab(TabId(2)).is_some());
        assert!(workspace.active_object_tab().is_none());
        assert_eq!(workspace.active_sql_tab().map(|sql| sql.number), Some(1));
        assert_eq!(workspace.objects().count(), 1);
    }

    #[test]
    fn a_sql_tab_waits_for_its_run() {
        let mut tab = sql_tab(2);
        assert!(tab.pending().is_empty());
        tab.as_sql_mut().unwrap().run.start(RequestId(9));
        assert_eq!(tab.pending(), vec![RequestId(9)]);
    }
```

`crate::testing::workspace()` does not exist yet: add a `pub fn workspace() -> Workspace` to `src/testing.rs` that builds one the way `App::open_workspace` does (copy the field list from `open_workspace`, with `ConnectSpec::sqlite("/tmp/fixture.db")`, `Driver::Sqlite`, `SessionId(1)`), so model tests can build one without an app. If `open_workspace` builds it inline, move that literal into `Workspace::new(session, conn_id, saved, secrets)` in `model.rs` and call it from both places instead of copying.

- [ ] **Step 2: Run to see them fail**

Run: `~/.cargo/bin/cargo test --locked --lib model::`
Expected: FAIL to compile.

- [ ] **Step 3: Add the types to `model.rs`**

```rust
/// One open tab in a workspace. The object tab is boxed: it is much
/// larger than a SQL tab (clippy's large_enum_variant).
#[derive(Debug)]
pub enum Tab {
    Object(Box<ObjectTab>),
    Sql(SqlTab),
}

impl Tab {
    pub fn id(&self) -> TabId {
        match self {
            Self::Object(object) => object.id,
            Self::Sql(sql) => sql.id,
        }
    }

    pub fn as_object(&self) -> Option<&ObjectTab> {
        match self {
            Self::Object(object) => Some(object.as_ref()),
            Self::Sql(_) => None,
        }
    }

    pub fn as_object_mut(&mut self) -> Option<&mut ObjectTab> {
        match self {
            Self::Object(object) => Some(object.as_mut()),
            Self::Sql(_) => None,
        }
    }

    pub fn as_sql(&self) -> Option<&SqlTab> {
        match self {
            Self::Sql(sql) => Some(sql),
            Self::Object(_) => None,
        }
    }

    pub fn as_sql_mut(&mut self) -> Option<&mut SqlTab> {
        match self {
            Self::Sql(sql) => Some(sql),
            Self::Object(_) => None,
        }
    }

    /// What the tab is loading; closing it cancels these.
    pub fn pending(&self) -> Vec<RequestId> {
        match self {
            Self::Object(object) => object.pending().collect(),
            Self::Sql(sql) => sql.run.pending.into_iter().collect(),
        }
    }

    /// A preview tab is replaced by the next single click. SQL tabs never are.
    pub fn is_preview(&self) -> bool {
        matches!(self, Self::Object(object) if !object.pinned)
    }
}

/// Which pane of a SQL tab's results shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ResultPane {
    #[default]
    Results,
    Messages,
}

/// A finished SQL editor run.
#[derive(Debug, Clone, PartialEq)]
pub struct SqlRun {
    /// The statements as split when the run started.
    pub statements: Vec<tabletist_db::sql::Statement>,
    pub outcome: tabletist_db::ScriptOutcome,
    pub cancel: Option<crate::backend::CancelReason>,
}

/// One SQL editor. Its text lives only in memory.
#[derive(Debug)]
pub struct SqlTab {
    pub id: TabId,
    /// "Query 3".
    pub number: u32,
    pub text: String,
    /// Byte offset of the editor's cursor, reported by the view.
    pub cursor: usize,
    pub limit: u32,
    pub timeout: Option<Duration>,
    /// The last run, or the one running (a whole-run failure is its error).
    pub run: Fetch<SqlRun>,
    /// The statements of the run in flight.
    pub running: Vec<tabletist_db::sql::Statement>,
    /// When the run in flight started, for the elapsed time.
    pub started: Option<std::time::Instant>,
    pub pane: ResultPane,
    pub selection: Option<CellPos>,
    /// Focus the editor on the next frame.
    pub focus_editor: bool,
}

impl SqlTab {
    pub fn new(id: TabId, number: u32, limit: u32, timeout: Option<Duration>) -> Self {
        Self {
            id,
            number,
            text: String::new(),
            cursor: 0,
            limit,
            timeout,
            run: Fetch::default(),
            running: Vec::new(),
            started: None,
            pane: ResultPane::default(),
            selection: None,
            focus_editor: true,
        }
    }
}
```

In `Workspace`, replace `pub objects: Vec<ObjectTab>,` with:

```rust
    /// Open tabs, in strip order.
    pub tabs: Vec<Tab>,
    /// The number the next SQL editor gets ("Query 1", "Query 2", ...).
    pub next_query: u32,
    /// "PostgreSQL 17.2", asked for when the first SQL editor opens.
    pub server_version: Fetch<String>,
```

(initialise `tabs: Vec::new(), next_query: 1, server_version: Fetch::default()` where the workspace is built). Replace the `impl Workspace` block:

```rust
impl Workspace {
    pub fn tab(&self, id: TabId) -> Option<&Tab> {
        self.tabs.iter().find(|tab| tab.id() == id)
    }

    pub fn tab_mut(&mut self, id: TabId) -> Option<&mut Tab> {
        self.tabs.iter_mut().find(|tab| tab.id() == id)
    }

    pub fn object_tab(&self, id: TabId) -> Option<&ObjectTab> {
        self.tab(id)?.as_object()
    }

    pub fn object_tab_mut(&mut self, id: TabId) -> Option<&mut ObjectTab> {
        self.tab_mut(id)?.as_object_mut()
    }

    pub fn sql_tab(&self, id: TabId) -> Option<&SqlTab> {
        self.tab(id)?.as_sql()
    }

    pub fn sql_tab_mut(&mut self, id: TabId) -> Option<&mut SqlTab> {
        self.tab_mut(id)?.as_sql_mut()
    }

    pub fn active_object_tab(&self) -> Option<&ObjectTab> {
        self.object_tab(self.active_tab?)
    }

    pub fn active_sql_tab(&self) -> Option<&SqlTab> {
        self.sql_tab(self.active_tab?)
    }

    pub fn objects(&self) -> impl Iterator<Item = &ObjectTab> {
        self.tabs.iter().filter_map(Tab::as_object)
    }

    pub fn objects_mut(&mut self) -> impl Iterator<Item = &mut ObjectTab> {
        self.tabs.iter_mut().filter_map(Tab::as_object_mut)
    }
}
```

Add `use std::time::Duration;` to `model.rs` if absent.

- [ ] **Step 4: Move every call site to `tabs`**

Rename the three generic actions in `model.rs` (`ActivateObjectTab`→`ActivateTab`, `CloseObjectTab`→`CloseTab`, `CycleObjectTab`→`CycleTab`) and fix their uses (`grep -rn 'ActivateObjectTab\|CloseObjectTab\|CycleObjectTab' src`). Then work through the compiler errors with these patterns:

| Old | New |
|---|---|
| `workspace.objects.iter()` over object tabs | `workspace.objects()` |
| `workspace.objects.iter_mut()` | `workspace.objects_mut()` |
| `workspace.objects.iter().position(\|o\| o.id == id)` | `workspace.tabs.iter().position(\|t\| t.id() == id)` |
| `workspace.objects.push(opened)` | `workspace.tabs.push(Tab::Object(Box::new(opened)))` |
| `workspace.objects.clear()` (SwitchDatabase) | keep SQL tabs, see below |
| `closed.pending()` (a removed `ObjectTab`) | `closed.pending()` on the removed `Tab` (now a `Vec`) |
| `workspace.objects.len()` / `[next].id` in cycling | `workspace.tabs.len()` / `workspace.tabs[next].id()` |
| `workspace.objects.iter_mut().find(\|o\| o.rows.pending == Some(request))` | `workspace.objects_mut().find(...)` |

The replacements that change meaning:

`open_object` (preview replacement and reuse):

```rust
        if let Some(existing) = workspace.objects_mut().find(|o| o.object == object) {
            existing.pinned |= pin;
            let id = existing.id;
            workspace.active_tab = Some(id);
            return;
        }
        // ... unchanged up to `preview`:
        let preview = if pin {
            None
        } else {
            workspace.tabs.iter().position(Tab::is_preview)
        };
        let session = workspace.session;
        let replaced = match preview {
            Some(index) => Some(std::mem::replace(
                &mut workspace.tabs[index],
                Tab::Object(Box::new(opened)),
            )),
            None => {
                workspace.tabs.push(Tab::Object(Box::new(opened)));
                None
            }
        };
        workspace.active_tab = Some(new_id);
        if let Some(replaced) = replaced {
            self.cancel(session, replaced.pending());
        }
```

`CloseTab`:

```rust
            Action::CloseTab { tab, object_tab } => {
                if let Some(workspace) = self.workspace_mut(tab)
                    && let Some(index) = workspace.tabs.iter().position(|t| t.id() == object_tab)
                {
                    let closed = workspace.tabs.remove(index);
                    let session = workspace.session;
                    if workspace.active_tab == Some(object_tab) {
                        workspace.active_tab = workspace
                            .tabs
                            .get(index)
                            .or_else(|| workspace.tabs.last())
                            .map(Tab::id);
                    }
                    // Nothing will show what it was loading (a count or a
                    // script can hold the connection for minutes).
                    self.cancel(session, closed.pending());
                }
            }
```

`ActivateTab` checks `workspace.tab(object_tab).is_some()`. `CycleTab` cycles over `workspace.tabs`.

`SwitchDatabase` keeps the user's SQL:

```rust
                    workspace.tree = Tree::default();
                    workspace.tabs.retain(|tab| matches!(tab, Tab::Sql(_)));
                    workspace.active_tab = workspace.tabs.first().map(Tab::id);
                    workspace.server_version = Fetch::default();
```

(any SQL run still pending is cancelled by the reconnect; clear it too: `for sql in workspace.tabs.iter_mut().filter_map(Tab::as_sql_mut) { sql.run.pending = None; sql.started = None; }`).

`ui/object_tabs.rs` builds its list from all tabs; for now name SQL tabs `"Query {n}"` (Task 13 adds the icon and the `+ sql` button):

```rust
    let tabs: Vec<(TabId, String, bool)> = workspace
        .tabs
        .iter()
        .map(|tab| match tab {
            Tab::Object(object) => (object.id, object.object.name.clone(), object.pinned),
            Tab::Sql(sql) => (
                sql.id,
                format!("{} {}", gettext(locale, "Query"), sql.number),
                true,
            ),
        })
        .collect();
```

`ui/keys.rs` number keys (terminal look) activate by index over `workspace.tabs.iter().map(Tab::id)`.

`shots.rs:723` (`workspace.objects.reverse()`) becomes `workspace.tabs.reverse()`. Tests that read `workspace.objects` switch to `workspace.objects()` or `workspace.tabs`.

- [ ] **Step 5: Run everything**

Run: `~/.cargo/bin/cargo test --locked --workspace --all-targets`
Expected: every existing test passes, plus the two new ones.

- [ ] **Step 6: Format, lint, commit**

```bash
~/.cargo/bin/cargo fmt --all && ~/.cargo/bin/cargo clippy --locked --workspace --all-targets -- -D warnings
git add -A src
git commit -m "Hold a workspace's tabs as object tabs or SQL editors

Opening, closing, cycling and the strip work on every tab; switching
database keeps SQL editors.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 10: Settings for the row limit and timeout, saved on change

**Files:**
- Modify: `src/settings.rs`, `src/app.rs`

**Interfaces:**
- Produces: `Settings { sql_limit: u32, sql_timeout_secs: Option<u32>, .. }`, `Settings::SQL_LIMITS: [u32; 3]`, `Settings::SQL_TIMEOUTS: [Option<u32>; 5]`, `Settings::sql_timeout(&self) -> Option<Duration>`, `App::save_settings(&mut self)`.

- [ ] **Step 1: Write the failing tests** (in `settings.rs` tests; extend the existing ones)

In `defaults_match_the_spec` add:

```rust
        assert_eq!(settings.sql_limit, 1_000);
        assert_eq!(settings.sql_timeout_secs, Some(30));
        assert_eq!(settings.sql_timeout(), Some(std::time::Duration::from_secs(30)));
```

In `settings_round_trip`, add `sql_limit: 100, sql_timeout_secs: None,` to the literal. New tests:

```rust
    #[test]
    fn older_files_get_the_sql_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        std::fs::write(&path, br#"{"page_size": 100}"#).unwrap();
        let settings = Settings::load(&path);
        assert_eq!(settings.sql_limit, 1_000);
        assert_eq!(settings.sql_timeout_secs, Some(30));
    }

    #[test]
    fn the_sql_limit_is_clamped() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        std::fs::write(&path, br#"{"sql_limit": 0, "sql_timeout_secs": null}"#).unwrap();
        let settings = Settings::load(&path);
        assert_eq!(settings.sql_limit, 1);
        assert_eq!(settings.sql_timeout_secs, None);
    }
```

In `app.rs` tests:

```rust
    #[test]
    fn saving_settings_sends_them_to_the_backend() {
        let mut harness = Harness::new();
        harness.app.settings.sql_limit = 100;
        harness.app.save_settings();
        assert!(matches!(
            last_sent(&harness.app),
            Command::Save { file: StateFile::Settings(settings), .. } if settings.sql_limit == 100
        ));
    }
```

- [ ] **Step 2: Run to see them fail**

Run: `~/.cargo/bin/cargo test --locked --lib settings:: app::`
Expected: FAIL to compile.

- [ ] **Step 3: Implement**

`settings.rs`, in the struct:

```rust
    /// Rows a SQL editor statement keeps (the Limit menu).
    pub sql_limit: u32,
    /// Seconds before a SQL editor run is cancelled; `None` waits forever.
    pub sql_timeout_secs: Option<u32>,
```

constants and helper:

```rust
    pub const SQL_LIMITS: [u32; 3] = [100, 1_000, 10_000];
    pub const SQL_TIMEOUTS: [Option<u32>; 5] = [Some(10), Some(30), Some(60), Some(300), None];

    pub fn sql_timeout(&self) -> Option<std::time::Duration> {
        self.sql_timeout_secs
            .map(|secs| std::time::Duration::from_secs(u64::from(secs)))
    }
```

`Default` gains `sql_limit: 1_000, sql_timeout_secs: Some(30),`; `load` clamps `settings.sql_limit = settings.sql_limit.clamp(1, Self::MAX_PAGE_SIZE);`.

`app.rs`:

```rust
    /// Writes the settings (the SQL editor's menus change them).
    pub fn save_settings(&mut self) {
        self.backend.send(Command::Save {
            path: self.dirs.settings_file(),
            file: StateFile::Settings(self.settings.clone()),
        });
    }
```

- [ ] **Step 4: Run the tests**

Run: `~/.cargo/bin/cargo test --locked --lib settings:: app::`
Expected: PASS.

- [ ] **Step 5: Format, lint, commit**

```bash
~/.cargo/bin/cargo fmt --all && ~/.cargo/bin/cargo clippy --locked --workspace --all-targets -- -D warnings
git add src/settings.rs src/app.rs
git commit -m "Remember the SQL editor's row limit and timeout

Settings gain the two values and are written through the backend when
they change.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 11: App reducers for SQL tabs

**Files:**
- Modify: `src/model.rs` (actions, `SqlTab` helpers), `src/app.rs`

**Interfaces:**
- Produces: `Action::NewSqlTab(ConnTabId)`, `Action::RunSql { tab, sql_tab: TabId, all: bool }`, `Action::SetSqlLimit { tab, sql_tab, limit: u32 }`, `Action::SetSqlTimeout { tab, sql_tab, secs: Option<u32> }`, `Action::SetResultPane { tab, sql_tab, pane: ResultPane }`; `App::active_sql(&self) -> Option<(ConnTabId, TabId)>`, `App::active_workspace_tab(&self) -> Option<(ConnTabId, TabId)>`; `SqlTab::{shown, shown_rows, line_col, error_mark, dims}`.
- `SelectCell` / `MoveSelection` also act on a SQL tab's result grid. `CancelQuery` cancels a SQL tab's run. `Refresh` does nothing on a SQL tab.

- [ ] **Step 1: Write the failing tests** (in `app.rs` tests; use `Harness`, `connect_tab`/`connect_fake`, `last_sent` as the existing tests do)

```rust
    fn new_sql(harness: &mut Harness) -> (ConnTabId, TabId) {
        let tab = harness.connect_fake();
        harness.app.apply(Action::NewSqlTab(tab));
        let id = harness.app.workspace(tab).unwrap().active_tab.unwrap();
        (tab, id)
    }

    fn sql(harness: &Harness, tab: ConnTabId, id: TabId) -> &SqlTab {
        harness.app.workspace(tab).unwrap().sql_tab(id).unwrap()
    }

    fn type_sql(harness: &mut Harness, tab: ConnTabId, id: TabId, text: &str, cursor: usize) {
        let sql = harness.app.workspace_mut(tab).unwrap().sql_tab_mut(id).unwrap();
        sql.text = text.into();
        sql.cursor = cursor;
    }

    #[test]
    fn sql_tabs_are_numbered_per_connection_and_ask_for_the_version() {
        let mut harness = Harness::new();
        let (tab, first) = new_sql(&mut harness);
        assert!(matches!(last_sent(&harness.app), Command::ServerVersion { .. }));
        harness.app.apply(Action::NewSqlTab(tab));
        let workspace = harness.app.workspace(tab).unwrap();
        let second = workspace.active_tab.unwrap();
        assert_eq!(workspace.sql_tab(first).unwrap().number, 1);
        assert_eq!(workspace.sql_tab(second).unwrap().number, 2);
        assert_eq!(workspace.sql_tab(second).unwrap().limit, 1_000);
        assert!(workspace.sql_tab(second).unwrap().focus_editor);
    }

    #[test]
    fn run_sends_the_statement_at_the_cursor_and_run_all_sends_every_one() {
        let mut harness = Harness::new();
        let (tab, id) = new_sql(&mut harness);
        type_sql(&mut harness, tab, id, "SELECT 1;\nSELECT 2;", 12);
        harness.app.apply(Action::RunSql { tab, sql_tab: id, all: false });
        assert!(matches!(
            last_sent(&harness.app),
            Command::RunSql { statements, limit: 1_000, timeout: Some(_), .. }
                if statements.len() == 1 && statements[0].text == "SELECT 2"
        ));
        harness.app.apply(Action::RunSql { tab, sql_tab: id, all: true });
        assert!(matches!(
            last_sent(&harness.app),
            Command::RunSql { statements, .. } if statements.len() == 2
        ));
    }

    #[test]
    fn running_an_empty_editor_does_nothing() {
        let mut harness = Harness::new();
        let (tab, id) = new_sql(&mut harness);
        type_sql(&mut harness, tab, id, "  -- nothing\n", 0);
        let sent = harness.app.backend.sent.len();
        harness.app.apply(Action::RunSql { tab, sql_tab: id, all: true });
        assert_eq!(harness.app.backend.sent.len(), sent);
        assert!(sql(&harness, tab, id).run.error.is_none());
    }

    #[test]
    fn a_new_run_cancels_the_one_still_running() {
        let mut harness = Harness::new();
        let (tab, id) = new_sql(&mut harness);
        type_sql(&mut harness, tab, id, "SELECT 1", 0);
        harness.app.apply(Action::RunSql { tab, sql_tab: id, all: false });
        let first = sql(&harness, tab, id).run.pending.unwrap();
        harness.app.apply(Action::RunSql { tab, sql_tab: id, all: false });
        assert!(harness.app.backend.sent.iter().any(
            |command| matches!(command, Command::Cancel { request, .. } if *request == first)
        ));
    }

    fn answer_sql(
        harness: &mut Harness,
        result: Result<tabletist_db::ScriptOutcome, tabletist_db::Error>,
        cancel: Option<crate::backend::CancelReason>,
    ) {
        let Command::RunSql { session, request, .. } = *harness
            .app
            .backend
            .sent
            .iter()
            .rev()
            .find(|command| matches!(command, Command::RunSql { .. }))
            .unwrap()
        else {
            unreachable!()
        };
        harness.app.apply(Action::Backend(Event::SqlRan {
            session,
            request,
            result,
            cancel,
        }));
    }

    fn rows_outcome(count: usize) -> tabletist_db::StatementOutcome {
        let page = crate::testing::page(count, false);
        tabletist_db::StatementOutcome::Rows {
            columns: page.columns,
            rows: page.rows,
            truncated: false,
        }
    }

    #[test]
    fn a_result_shows_its_rows_and_an_error_opens_messages() {
        use tabletist_db::{ScriptOutcome, StatementOutcome, StatementResult};
        let mut harness = Harness::new();
        let (tab, id) = new_sql(&mut harness);
        type_sql(&mut harness, tab, id, "SELECT 1; SELECT x", 0);
        harness.app.apply(Action::RunSql { tab, sql_tab: id, all: true });
        answer_sql(
            &mut harness,
            Ok(ScriptOutcome {
                results: vec![
                    StatementResult { elapsed: Duration::from_millis(14), outcome: rows_outcome(5) },
                    StatementResult {
                        elapsed: Duration::ZERO,
                        outcome: StatementOutcome::Error {
                            error: tabletist_db::Error::query("no such column: x"),
                            position: None,
                        },
                    },
                ],
            }),
            None,
        );
        let sql = sql(&harness, tab, id);
        assert_eq!(sql.pane, ResultPane::Messages);
        assert_eq!(sql.shown().map(|(index, _)| index), Some(0));
        assert_eq!(sql.dims(), (5, 3));
        assert_eq!(sql.error_mark(), Some((1, None)));
    }

    #[test]
    fn a_refused_script_is_the_runs_error() {
        let mut harness = Harness::new();
        let (tab, id) = new_sql(&mut harness);
        type_sql(&mut harness, tab, id, "COMMIT", 0);
        harness.app.apply(Action::RunSql { tab, sql_tab: id, all: false });
        answer_sql(
            &mut harness,
            Err(tabletist_db::Error::Refused { line: 1, what: "COMMIT".into() }),
            None,
        );
        let sql = sql(&harness, tab, id);
        assert!(sql.run.error.is_some());
        assert_eq!(sql.pane, ResultPane::Messages);
    }

    #[test]
    fn a_result_for_a_closed_sql_tab_is_ignored() {
        let mut harness = Harness::new();
        let (tab, id) = new_sql(&mut harness);
        type_sql(&mut harness, tab, id, "SELECT 1", 0);
        harness.app.apply(Action::RunSql { tab, sql_tab: id, all: false });
        let request = sql(&harness, tab, id).run.pending.unwrap();
        harness.app.apply(Action::CloseTab { tab, object_tab: id });
        assert!(harness.app.backend.sent.iter().any(
            |command| matches!(command, Command::Cancel { request: r, .. } if *r == request)
        ));
        answer_sql(&mut harness, Ok(Default::default()), None);
        assert!(harness.app.workspace(tab).unwrap().tabs.is_empty());
    }

    #[test]
    fn cancel_stops_the_active_sql_run() {
        let mut harness = Harness::new();
        let (tab, id) = new_sql(&mut harness);
        type_sql(&mut harness, tab, id, "SELECT 1", 0);
        harness.app.apply(Action::RunSql { tab, sql_tab: id, all: false });
        let request = sql(&harness, tab, id).run.pending.unwrap();
        harness.app.apply(Action::CancelQuery(tab));
        assert!(matches!(
            last_sent(&harness.app),
            Command::Cancel { request: r, .. } if *r == request
        ));
    }

    #[test]
    fn the_limit_and_timeout_menus_change_the_next_run_and_the_settings() {
        let mut harness = Harness::new();
        let (tab, id) = new_sql(&mut harness);
        harness.app.apply(Action::SetSqlLimit { tab, sql_tab: id, limit: 100 });
        assert!(matches!(last_sent(&harness.app), Command::Save { .. }));
        harness.app.apply(Action::SetSqlTimeout { tab, sql_tab: id, secs: None });
        type_sql(&mut harness, tab, id, "SELECT 1", 0);
        harness.app.apply(Action::RunSql { tab, sql_tab: id, all: false });
        assert!(matches!(
            last_sent(&harness.app),
            Command::RunSql { limit: 100, timeout: None, .. }
        ));
        assert_eq!(harness.app.settings.sql_limit, 100);
        assert_eq!(harness.app.settings.sql_timeout_secs, None);
    }

    #[test]
    fn arrows_move_in_a_sql_result() {
        use tabletist_db::{ScriptOutcome, StatementResult};
        let mut harness = Harness::new();
        let (tab, id) = new_sql(&mut harness);
        type_sql(&mut harness, tab, id, "SELECT 1", 0);
        harness.app.apply(Action::RunSql { tab, sql_tab: id, all: false });
        answer_sql(
            &mut harness,
            Ok(ScriptOutcome {
                results: vec![StatementResult { elapsed: Duration::ZERO, outcome: rows_outcome(3) }],
            }),
            None,
        );
        harness.app.apply(Action::MoveSelection { tab, object_tab: id, rows: 1, cols: 0 });
        harness.app.apply(Action::MoveSelection { tab, object_tab: id, rows: 1, cols: 1 });
        assert_eq!(sql(&harness, tab, id).selection, Some(CellPos { row: 1, col: 1 }));
    }

    #[test]
    fn refresh_does_nothing_on_a_sql_tab() {
        let mut harness = Harness::new();
        let (tab, _) = new_sql(&mut harness);
        let sent = harness.app.backend.sent.len();
        harness.app.apply(Action::Refresh(tab));
        assert_eq!(harness.app.backend.sent.len(), sent);
    }
```

`model.rs` tests for the helpers:

```rust
    #[test]
    fn a_sql_tab_reports_its_cursor_line_and_column() {
        let mut sql = SqlTab::new(TabId(1), 1, 10, None);
        sql.text = "SELECT 1;\nSELECT ë, 2".into();
        sql.cursor = sql.text.len();
        assert_eq!(sql.line_col(), (2, 12));
    }
```

- [ ] **Step 2: Run to see them fail**

Run: `~/.cargo/bin/cargo test --locked --lib app:: model::`
Expected: FAIL to compile.

- [ ] **Step 3: Implement the model helpers** (`model.rs`, `impl SqlTab`)

```rust
    /// The result the Results pane shows: the last statement that returned
    /// rows, with its index in the run.
    pub fn shown(&self) -> Option<(usize, &tabletist_db::StatementResult)> {
        let run = self.run.value.as_ref()?;
        run.outcome
            .results
            .iter()
            .enumerate()
            .rev()
            .find(|(_, result)| {
                matches!(result.outcome, tabletist_db::StatementOutcome::Rows { .. })
            })
    }

    /// The shown result's columns and rows.
    pub fn shown_rows(
        &self,
    ) -> Option<(&[tabletist_db::ColumnMeta], &[Vec<tabletist_db::Value>], bool)> {
        match &self.shown()?.1.outcome {
            tabletist_db::StatementOutcome::Rows {
                columns,
                rows,
                truncated,
            } => Some((columns, rows, *truncated)),
            _ => None,
        }
    }

    /// Rows and columns of the shown result.
    pub fn dims(&self) -> (usize, usize) {
        self.shown_rows()
            .map_or((0, 0), |(columns, rows, _)| (rows.len(), columns.len()))
    }

    /// The cursor's 1-based line and column (in characters).
    pub fn line_col(&self) -> (usize, usize) {
        let cursor = self.cursor.min(self.text.len());
        let before = self.text.get(..cursor).unwrap_or(&self.text);
        let line = before.matches('\n').count() + 1;
        let column = before.rsplit('\n').next().map_or(0, |line| line.chars().count()) + 1;
        (line, column)
    }

    /// Where the last run failed: the editor line, and the column when the
    /// database gave a position.
    pub fn error_mark(&self) -> Option<(usize, Option<usize>)> {
        if let Some(tabletist_db::Error::Refused { line, .. }) = &self.run.error {
            return Some((*line, None));
        }
        let run = self.run.value.as_ref()?;
        run.outcome.results.iter().enumerate().find_map(|(index, result)| {
            let tabletist_db::StatementOutcome::Error { position, .. } = &result.outcome else {
                return None;
            };
            let statement = run.statements.get(index)?;
            Some(match position {
                Some(position) => {
                    let (line, column) = statement.line_col(*position);
                    (line, Some(column))
                }
                None => (statement.first_line, None),
            })
        })
    }
```

Add to `Action`:

```rust
    /// Open a SQL editor in the connection tab's workspace.
    NewSqlTab(ConnTabId),
    /// Run the statement at the cursor, or every statement.
    RunSql { tab: ConnTabId, sql_tab: TabId, all: bool },
    SetSqlLimit { tab: ConnTabId, sql_tab: TabId, limit: u32 },
    SetSqlTimeout { tab: ConnTabId, sql_tab: TabId, secs: Option<u32> },
    SetResultPane { tab: ConnTabId, sql_tab: TabId, pane: ResultPane },
```

- [ ] **Step 4: Implement the reducers** (`app.rs`)

```rust
            Action::NewSqlTab(tab) => self.new_sql_tab(tab),
            Action::RunSql { tab, sql_tab, all } => self.run_sql(tab, sql_tab, all),
            Action::SetSqlLimit { tab, sql_tab, limit } => {
                if let Some(sql) = self.sql_tab_mut(tab, sql_tab) {
                    sql.limit = limit;
                }
                self.settings.sql_limit = limit;
                self.save_settings();
            }
            Action::SetSqlTimeout { tab, sql_tab, secs } => {
                if let Some(sql) = self.sql_tab_mut(tab, sql_tab) {
                    sql.timeout = secs.map(|secs| Duration::from_secs(u64::from(secs)));
                }
                self.settings.sql_timeout_secs = secs;
                self.save_settings();
            }
            Action::SetResultPane { tab, sql_tab, pane } => {
                if let Some(sql) = self.sql_tab_mut(tab, sql_tab) {
                    sql.pane = pane;
                }
            }
```

Methods:

```rust
    fn sql_tab_mut(&mut self, tab: ConnTabId, id: TabId) -> Option<&mut SqlTab> {
        self.workspace_mut(tab)?.sql_tab_mut(id)
    }

    /// The connection tab and the active tab of its workspace, of any kind.
    pub fn active_workspace_tab(&self) -> Option<(ConnTabId, TabId)> {
        let tab = self.active_tab_id();
        Some((tab, self.workspace(tab)?.active_tab?))
    }

    /// The active tab when it is a SQL editor.
    pub fn active_sql(&self) -> Option<(ConnTabId, TabId)> {
        let (tab, id) = self.active_workspace_tab()?;
        self.workspace(tab)?.sql_tab(id)?;
        Some((tab, id))
    }

    fn new_sql_tab(&mut self, tab: ConnTabId) {
        let id = TabId(self.next_id());
        let request = RequestId(self.next_id());
        let (limit, timeout) = (self.settings.sql_limit, self.settings.sql_timeout());
        let Some(workspace) = self.workspace_mut(tab) else {
            return;
        };
        let number = workspace.next_query;
        workspace.next_query += 1;
        workspace.tabs.push(Tab::Sql(SqlTab::new(id, number, limit, timeout)));
        workspace.active_tab = Some(id);
        workspace.pane = Pane::Grid;
        let session = workspace.session;
        if workspace.server_version.needs_load() {
            workspace.server_version.start(request);
            self.backend.send(Command::ServerVersion { session, request });
        }
    }

    fn run_sql(&mut self, tab: ConnTabId, id: TabId, all: bool) {
        let request = RequestId(self.next_id());
        let Some(workspace) = self.workspace_mut(tab) else {
            return;
        };
        let (session, dialect) = (workspace.session, workspace.driver.dialect());
        let Some(sql) = workspace.sql_tab_mut(id) else {
            return;
        };
        let found = tabletist_db::sql::statements(dialect, &sql.text);
        let statements: Vec<_> = if all {
            found
        } else {
            tabletist_db::sql::statement_at(&found, sql.cursor)
                .cloned()
                .into_iter()
                .collect()
        };
        if statements.is_empty() {
            return;
        }
        let superseded = sql.run.pending;
        sql.run.start(request);
        sql.running = statements.clone();
        sql.started = Some(std::time::Instant::now());
        let (limit, timeout) = (sql.limit, sql.timeout);
        self.cancel(session, superseded);
        self.backend.send(Command::RunSql {
            session,
            request,
            statements,
            limit,
            timeout,
        });
    }
```

In `apply_event`, replace the Task 8 placeholder:

```rust
            Event::SqlRan {
                session,
                request,
                result,
                cancel,
            } => {
                let Some(tab) = self.tab_for_session(session) else {
                    return;
                };
                let Some(sql) = self.workspace_mut(tab).and_then(|workspace| {
                    workspace
                        .tabs
                        .iter_mut()
                        .filter_map(Tab::as_sql_mut)
                        .find(|sql| sql.run.pending == Some(request))
                }) else {
                    return;
                };
                let statements = std::mem::take(&mut sql.running);
                let failed = match &result {
                    Err(_) => true,
                    Ok(outcome) => outcome.results.iter().any(|result| {
                        matches!(result.outcome, tabletist_db::StatementOutcome::Error { .. })
                    }),
                };
                sql.run.finish(
                    request,
                    result.map(|outcome| SqlRun {
                        statements,
                        outcome,
                        cancel,
                    }),
                );
                sql.started = None;
                sql.selection = None;
                sql.pane = if failed {
                    ResultPane::Messages
                } else {
                    ResultPane::Results
                };
            }
            Event::ServerVersion {
                session,
                request,
                result,
            } => {
                if let Some(tab) = self.tab_for_session(session)
                    && let Some(workspace) = self.workspace_mut(tab)
                {
                    workspace.server_version.finish(request, result);
                }
            }
```

`SelectCell` and `MoveSelection` handle a SQL tab too. In `MoveSelection`, before the object branch:

```rust
                if let Some(sql) = self.sql_tab_mut(tab, object_tab) {
                    let (height, width) = sql.dims();
                    sql.selection = (height > 0 && width > 0).then(|| match sql.selection {
                        None => CellPos { row: 0, col: 0 },
                        Some(cell) => CellPos {
                            row: step(cell.row, rows, height),
                            col: step(cell.col, cols, width),
                        },
                    });
                    return;
                }
```

and in `SelectCell`: `if let Some(sql) = self.sql_tab_mut(tab, object_tab) { sql.selection = Some(cell); }` next to the object branch (the pane update stays).

`CancelQuery`: when the active tab is SQL, cancel its run:

```rust
            Action::CancelQuery(tab) => {
                if let Some(workspace) = self.workspace(tab) {
                    let session = workspace.session;
                    let pending: Vec<RequestId> = if let Some(sql) = workspace.active_sql_tab() {
                        sql.run.pending.into_iter().collect()
                    } else {
                        match workspace.active_object_tab() {
                            // ... unchanged object / tree branches ...
                        }
                    };
                    self.cancel(session, pending);
                }
            }
```

`Refresh(tab)`: return early when `self.workspace(tab).and_then(Workspace::active_sql_tab).is_some()`.

`App::active_object()` must return `Some` only when the active tab is an object tab (it feeds object-only keys: paging, filter, row panel, copy):

```rust
    /// The connection tab and object tab the keyboard acts on; `None` when
    /// the active tab is a SQL editor.
    pub fn active_object(&self) -> Option<(ConnTabId, TabId)> {
        let tab = self.active_tab_id();
        let workspace = self.workspace(tab)?;
        Some((tab, workspace.active_object_tab()?.id))
    }
```

Add `assert!(harness.app.active_object().is_none());` to `sql_tabs_are_numbered_per_connection_and_ask_for_the_version`.

- [ ] **Step 5: Run the tests**

Run: `~/.cargo/bin/cargo test --locked --lib`
Expected: PASS.

- [ ] **Step 6: Format, lint, commit**

```bash
~/.cargo/bin/cargo fmt --all && ~/.cargo/bin/cargo clippy --locked --workspace --all-targets -- -D warnings
git add src/model.rs src/app.rs
git commit -m "Open SQL editors and run their statements

Run sends the statement at the cursor, Run all every statement; results
replace only the run they answer, and an error opens Messages.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 12: Keys: `Mod+T`, `Mod+O`, `Mod+Return` and SQL tab routing

**Files:**
- Modify: `src/ui/keys.rs`, `src/ui/conn_tabs.rs` (tooltip naming the shortcut, if any), `src/ui/mod.rs` (tests)

**Interfaces:**
- Consumes: `App::active_sql`, `App::active_workspace_tab`.
- Keys: `Mod+T` → `NewSqlTab(active)` in a workspace (nothing on the picker); `Mod+O` → `NewConnTab`; with a SQL tab active, `Mod+Shift+Return` → `RunSql { all: true }` and `Mod+Return` → `RunSql { all: false }`, both even while typing; `Mod+W`, `Mod+Shift+[ ]` act on any tab; `Mod+R`, `Mod+F`, `Mod+Alt+arrows`, Space and the object-only letters do nothing on a SQL tab; arrows, Page Up/Down, Home/End and `j/k/h/l` move in a SQL result when the editor does not have focus.

- [ ] **Step 1: Write the failing tests** (`src/ui/mod.rs` tests)

```rust
    #[test]
    fn command_t_opens_a_sql_editor_and_command_o_a_connection_tab() {
        let mut harness = Harness::new();
        let tab = harness.connect_fake();
        harness.press(Key::T, Modifiers::COMMAND);
        let workspace = harness.app.workspace(tab).unwrap();
        assert!(workspace.active_sql_tab().is_some());
        assert!(harness.has("Query 1 tab"));
        let tabs = harness.app.tabs.len();
        harness.press(Key::O, Modifiers::COMMAND);
        assert_eq!(harness.app.tabs.len(), tabs + 1);
        // On the picker, Mod+T does nothing.
        harness.press(Key::T, Modifiers::COMMAND);
        assert_eq!(harness.app.tabs.len(), tabs + 1);
    }

    #[test]
    fn command_return_runs_while_typing() {
        let mut harness = Harness::new();
        let tab = harness.connect_fake();
        harness.press(Key::T, Modifiers::COMMAND);
        harness.frame(vec![egui::Event::Text("SELECT 1;\nSELECT 2".into())]);
        harness.press(Key::Enter, Modifiers::COMMAND);
        assert!(matches!(
            harness.app.backend.sent.last(),
            Some(crate::backend::Command::RunSql { statements, .. })
                if statements.len() == 1 && statements[0].text == "SELECT 2"
        ));
        harness.press(Key::Enter, Modifiers::COMMAND | Modifiers::SHIFT);
        assert!(matches!(
            harness.app.backend.sent.last(),
            Some(crate::backend::Command::RunSql { statements, .. }) if statements.len() == 2
        ));
        // Mod+Return did not type a newline.
        let workspace = harness.app.workspace(tab).unwrap();
        assert_eq!(workspace.active_sql_tab().unwrap().text, "SELECT 1;\nSELECT 2");
    }

    #[test]
    fn refresh_and_filter_do_nothing_on_a_sql_tab() {
        let mut harness = Harness::new();
        let tab = harness.connect_fake();
        harness.press(Key::T, Modifiers::COMMAND);
        let sent = harness.app.backend.sent.len();
        harness.press(Key::R, Modifiers::COMMAND);
        harness.press(Key::F, Modifiers::COMMAND);
        assert_eq!(harness.app.backend.sent.len(), sent);
        assert!(harness.app.workspace(tab).unwrap().active_sql_tab().is_some());
    }

    #[test]
    fn command_w_closes_a_sql_tab() {
        let mut harness = Harness::new();
        let tab = harness.connect_fake();
        harness.press(Key::T, Modifiers::COMMAND);
        harness.press(Key::W, Modifiers::COMMAND);
        assert!(harness.app.workspace(tab).unwrap().tabs.is_empty());
    }
```

```rust
    #[test]
    fn after_escape_arrows_move_in_a_sql_result() {
        let mut harness = Harness::new();
        let tab = harness.connect_fake();
        harness.app.apply(crate::model::Action::NewSqlTab(tab));
        let id = harness.app.workspace(tab).unwrap().active_tab.unwrap();
        harness.app.workspace_mut(tab).unwrap().sql_tab_mut(id).unwrap().text = "SELECT 1".into();
        harness.app.apply(crate::model::Action::RunSql { tab, sql_tab: id, all: false });
        harness.answer_sql(
            Ok(tabletist_db::ScriptOutcome {
                results: vec![tabletist_db::StatementResult {
                    elapsed: std::time::Duration::ZERO,
                    outcome: crate::testing::rows_outcome(3),
                }],
            }),
            None,
        );
        harness.press(Key::Escape, Modifiers::NONE);
        harness.press(Key::ArrowDown, Modifiers::NONE);
        harness.press(Key::ArrowDown, Modifiers::NONE);
        let workspace = harness.app.workspace(tab).unwrap();
        assert_eq!(
            workspace.sql_tab(id).unwrap().selection,
            Some(crate::model::CellPos { row: 1, col: 0 })
        );
    }
```

This test uses `Harness::answer_sql` and `testing::rows_outcome`: move Task 11's `answer_sql` and `rows_outcome` test helpers into `src/testing.rs` now (`impl Harness { pub fn answer_sql(&mut self, result, cancel) }` and `pub fn rows_outcome(count: usize) -> StatementOutcome`) and make Task 11's tests call those.

(These need the SQL view to exist so the editor can take focus and text: they pass after Task 14. Write them now, see them fail, and expect the `command_return_runs_while_typing` case to keep failing until Task 14; mark it `#[ignore = "needs the editor view (Task 14)"]` in this commit and remove the ignore in Task 14.)

- [ ] **Step 2: Run to see them fail**

Run: `~/.cargo/bin/cargo test --locked --lib ui::tests::command_`
Expected: FAIL (Mod+T opens a connection tab).

- [ ] **Step 3: Implement** (`keys.rs`)

In `handle`, compute next to `object`:

```rust
    let sql = app.active_sql();
    let any_tab = app.active_workspace_tab();
    let in_workspace = app.workspace(active).is_some();
```

Replace the `Mod+T` binding and add the new ones (Shift variants first, as the file already does):

```rust
        if let Some((tab, sql_tab)) = sql {
            key(
                Modifiers::COMMAND | Modifiers::SHIFT,
                Key::Enter,
                Action::RunSql { tab, sql_tab, all: true },
            );
            key(
                Modifiers::COMMAND,
                Key::Enter,
                Action::RunSql { tab, sql_tab, all: false },
            );
        }
        if in_workspace {
            key(Modifiers::COMMAND, Key::T, Action::NewSqlTab(active));
        }
        key(Modifiers::COMMAND, Key::O, Action::NewConnTab);
```

Gate `Mod+R`, `Mod+F` and `Mod+Shift+R` (row panel) with `if sql.is_none() { ... }`. Move `Mod+Shift+[ ]` and `Mod+W` out of the `if let Some((tab, object_tab)) = object` block into one on `any_tab`, using `Action::CycleTab` and `Action::CloseTab`; `Mod+Alt+arrows` stay object-only.

Grid routing: a SQL result counts as a grid.

```rust
    let sql_grid = sql.is_some_and(|(tab, id)| {
        app.workspace(tab)
            .and_then(|workspace| workspace.sql_tab(id))
            .is_some_and(|sql| sql.dims().0 > 0)
    });
```

Route arrows by `grid || sql_grid` everywhere `grid` decides it today. In particular `tree_arrows` becomes

```rust
    let tree_arrows = !editing
        && app.workspace(active).is_some_and(|workspace| {
            workspace.pane == crate::model::Pane::Tree || !(grid || sql_grid)
        });
```

and the block that turns arrows, Page Up/Down and Home/End into `MoveSelection` moves out of `if let Some((tab, object_tab)) = object` onto `if let Some((tab, object_tab)) = any_tab`, guarded by `!editing && (grid || sql_grid) && !tree_arrows`. Space (row panel) and Copy stay on `grid` only. `NewSqlTab` sets `workspace.pane = Pane::Grid` (Task 11), so arrows go to the result once the editor lets go of focus.

In `letters`, after the tree `j/k` handling, when the active tab is SQL, push only the moves and return:

```rust
    if let Some(sql_tab) = active.filter(|id| workspace.sql_tab(*id).is_some()) {
        let step = |rows: isize, cols: isize| Action::MoveSelection { tab, object_tab: sql_tab, rows, cols };
        if !tree {
            for (key, rows, cols) in [(Key::J, 1, 0), (Key::K, -1, 0), (Key::H, 0, -1), (Key::L, 0, 1)] {
                if pressed(key) {
                    actions.push(step(rows, cols));
                }
            }
        }
        ctx.data_mut(|data| data.insert_temp(pending_id(), next_pending));
        return;
    }
```

(`letters` only runs when no text field has focus, so typing `j` in the editor never moves.)

`SHORTCUTS` rows:

```rust
    ("Mod+T", "New SQL editor"),
    ("Mod+O", "New connection tab"),
    ("Mod+Return, Mod+Shift+Return", "Run statement / run all"),
    ...
    ("Mod+W", "Close tab"),
    ("Mod+Shift+[ / ]", "Previous / next tab"),
```

Update the test at the bottom of `keys.rs` that lists the table's descriptions. Two existing tests press `Mod+T` on the picker to open a connection tab: `one_connection_needs_no_tab_bar` and `ctrl_t_opens_a_tab` in `src/ui/mod.rs` (around lines 193 and 411). Change them to `Key::O` (and rename the second to `ctrl_o_opens_a_tab`). `grep -rn '"Mod+T"\|⌘T\|Ctrl+T' src` and change any tooltip that names `Mod+T` for connections (the `+` in `conn_tabs.rs`) to `Mod+O`.

- [ ] **Step 4: Run the tests**

Run: `~/.cargo/bin/cargo test --locked --lib`
Expected: PASS (with the one test ignored until Task 14).

- [ ] **Step 5: Format, lint, commit**

```bash
~/.cargo/bin/cargo fmt --all && ~/.cargo/bin/cargo clippy --locked --workspace --all-targets -- -D warnings
git add src/ui src/testing.rs src/app.rs
git commit -m "Open SQL editors with Mod+T and connections with Mod+O

Mod+Return runs the statement at the cursor and Mod+Shift+Return runs
all, even while typing; table-only keys leave SQL tabs alone.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 13: The SQL view's shell: strip, sidebar button, toolbar, footer, status line

**Files:**
- Create: `src/ui/sql_editor.rs`, `assets/icons/square-code.svg`
- Modify: `src/ui/mod.rs` (`mod sql_editor;`), `src/ui/workspace.rs`, `src/ui/object_tabs.rs`, `src/ui/sidebar.rs`, `src/theme.rs` (icons)

**Interfaces:**
- Produces: `sql_editor::show(app, ui, tab, id)` (toolbar, footer, a top panel for the editor, a central panel for results; both bodies filled by Tasks 14 and 15), `sql_editor::status_summary(app, tab) -> Option<String>` for the terminal status line, `Icon::Play`, `Icon::SquareCode`.

- [ ] **Step 1: Write the failing tests** (`src/ui/mod.rs` tests)

```rust
    #[test]
    fn a_sql_tab_shows_its_toolbar_and_footer_in_every_look() {
        for look in Look::ALL {
            let mut harness = Harness::new();
            harness.set_look(look);
            let tab = harness.connect_fake();
            harness.app.apply(crate::model::Action::NewSqlTab(tab));
            harness.settle();
            let label = |text: &str| if look.terminal { text.to_lowercase() } else { text.into() };
            assert!(harness.has(&label("Run")), "{}", look.name);
            assert!(harness.has(&label("Run all")), "{}", look.name);
            assert!(harness.has("Limit"), "{}", look.name);
            assert!(harness.has("Timeout"), "{}", look.name);
            assert!(harness.has(&format!("{} tab", label("Query 1"))), "{}", look.name);
        }
    }

    #[test]
    fn the_sidebar_button_and_the_terminal_plus_open_a_sql_editor() {
        let mut harness = Harness::new();
        let tab = harness.connect_fake();
        harness.click("SQL Editor");
        assert!(harness.app.workspace(tab).unwrap().active_sql_tab().is_some());
        let mut harness = Harness::new();
        harness.set_look(Look::omarchy());
        let tab = harness.connect_fake();
        harness.click("New SQL editor");
        assert!(harness.app.workspace(tab).unwrap().active_sql_tab().is_some());
    }

    #[test]
    fn clicking_run_sends_the_script() {
        let mut harness = Harness::new();
        let tab = harness.connect_fake();
        harness.app.apply(crate::model::Action::NewSqlTab(tab));
        let id = harness.app.workspace(tab).unwrap().active_tab.unwrap();
        harness.app.workspace_mut(tab).unwrap().sql_tab_mut(id).unwrap().text = "SELECT 1".into();
        harness.click("Run");
        assert!(matches!(
            harness.app.backend.sent.last(),
            Some(crate::backend::Command::RunSql { .. })
        ));
    }
```

- [ ] **Step 2: Run to see them fail**

Run: `~/.cargo/bin/cargo test --locked --lib ui::tests::a_sql_tab`
Expected: FAIL (nothing drawn for SQL tabs).

- [ ] **Step 3: Icons**

`assets/icons/square-code.svg` (Lucide "square-code", ISC, same format as `table-2.svg`):

```svg
<svg
  xmlns="http://www.w3.org/2000/svg"
  width="24"
  height="24"
  viewBox="0 0 24 24"
  fill="none"
  stroke="#ffffff"
  stroke-width="2"
  stroke-linecap="round"
  stroke-linejoin="round"
>
  <path d="m10 9-3 3 3 3" />
  <path d="m14 15 3-3-3-3" />
  <rect x="3" y="3" width="18" height="18" rx="2" />
</svg>
```

In `theme.rs`'s `icons!`: `Play => lucide "play",` (with the other lucide lines, alphabetical) and `SquareCode => "square-code",` (with the app-own lines).

- [ ] **Step 4: The view shell** (`src/ui/sql_editor.rs`)

```rust
//! A SQL editor tab: the toolbar, the editor over its results, and the
//! footer. The editor (`sql_text`) and the results (`sql_results`) draw
//! their own panes.

use egui::{Frame, Id, Margin, Ui, WidgetInfo, WidgetType};

use crate::app::App;
use crate::i18n::gettext;
use crate::model::{Action, ConnTabId, TabId};
use crate::settings::Settings;
use crate::theme::Icon;
use crate::typography::TextRole;
use crate::ui::format::group_digits;
use crate::ui::keys::keys_label;
use crate::ui::widgets::{self, ButtonSpec};

pub fn show(app: &mut App, ui: &mut Ui, tab: ConnTabId, id: TabId) {
    toolbar(app, ui, tab, id);
    if !app.look.terminal {
        footer(app, ui, tab, id);
    }
    let available = ui.available_height();
    egui::Panel::top(Id::new(("sql-editor", tab.0, id.0)))
        .resizable(true)
        .default_size(available * 0.45)
        .size_range(80.0..=(available - 80.0).max(80.0))
        .frame(Frame::new().fill(app.palette.window))
        .show(ui, |ui| super::sql_text::show(app, ui, tab, id));
    egui::CentralPanel::default()
        .frame(Frame::new().fill(app.palette.window))
        .show(ui, |ui| super::sql_results::show(app, ui, tab, id));
}

/// "Limit 1,000" (macOS) or "limit 1000" (terminal).
fn limit_text(app: &App, limit: u32) -> String {
    let label = gettext(app.locale, "Limit");
    if app.look.terminal {
        format!("{} {limit}", app.look.label(&label))
    } else {
        format!("{label} {}", group_digits(u64::from(limit)))
    }
}

fn timeout_text(app: &App, secs: Option<u32>) -> String {
    match secs {
        None => app.look.label(&gettext(app.locale, "No timeout")),
        Some(secs) if app.look.terminal => format!("timeout {secs}s"),
        Some(secs) => format!("{} {secs} s", gettext(app.locale, "Timeout")),
    }
}

fn toolbar(app: &mut App, ui: &mut Ui, tab: ConnTabId, id: TabId) {
    let (look, palette, locale) = (app.look, app.palette, app.locale);
    let Some(sql) = app.workspace(tab).and_then(|w| w.sql_tab(id)) else {
        return;
    };
    let (limit, secs) = (sql.limit, sql.timeout.map(|t| t.as_secs() as u32));
    let title = look.label(&format!("{} {}", gettext(locale, "Query"), sql.number));
    let mut actions = Vec::new();
    let side = if look.terminal { 16 } else { 20 };
    egui::Panel::top(Id::new(("sql-toolbar", tab.0, id.0)))
        .exact_size(if look.terminal { 41.0 } else { 45.0 })
        .resizable(false)
        .show_separator_line(false)
        .frame(Frame::new().fill(palette.window).inner_margin(Margin::symmetric(side, 0)))
        .show(ui, |ui| {
            ui.horizontal_centered(|ui| {
                ui.spacing_mut().item_spacing.x = 8.0;
                if look.terminal {
                    widgets::label(ui, TextRole::OTableTitle, &title, palette.text, &look);
                    widgets::label(
                        ui,
                        TextRole::OSecondary,
                        &gettext(locale, "read-only transaction"),
                        palette.dim,
                        &look,
                    );
                }
                let run = look.label(&gettext(locale, "Run"));
                let run_all = look.label(&gettext(locale, "Run all"));
                let (run_keys, all_keys) = if look.terminal {
                    ("ctrl+enter".to_owned(), "ctrl+shift+enter".to_owned())
                } else {
                    (keys_label("Mod+Return"), keys_label("Mod+Shift+Return"))
                };
                if ButtonSpec::new(&run)
                    .icon(Icon::Play)
                    .shortcut(&run_keys)
                    .primary()
                    .show(ui, look.control_height, &look, &palette)
                    .clicked()
                {
                    actions.push(Action::RunSql { tab, sql_tab: id, all: false });
                }
                if ButtonSpec::new(&run_all)
                    .shortcut(&all_keys)
                    .show(ui, look.control_height, &look, &palette)
                    .clicked()
                {
                    actions.push(Action::RunSql { tab, sql_tab: id, all: true });
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if let Some(secs) = menu(
                        ui,
                        Id::new(("sql-timeout", tab.0, id.0)),
                        "Timeout",
                        &timeout_text(app, secs),
                        Settings::SQL_TIMEOUTS.map(|choice| (choice, timeout_text(app, choice))),
                        secs,
                        app,
                    ) {
                        actions.push(Action::SetSqlTimeout { tab, sql_tab: id, secs });
                    }
                    if let Some(limit) = menu(
                        ui,
                        Id::new(("sql-limit", tab.0, id.0)),
                        "Limit",
                        &limit_text(app, limit),
                        Settings::SQL_LIMITS.map(|choice| (choice, limit_text(app, choice))),
                        limit,
                        app,
                    ) {
                        actions.push(Action::SetSqlLimit { tab, sql_tab: id, limit });
                    }
                    if !look.terminal {
                        let badge = widgets::label(
                            ui,
                            widgets::secondary(&look),
                            &gettext(locale, "Read-only transaction"),
                            palette.secondary,
                            &look,
                        );
                        badge.on_hover_text(gettext(
                            locale,
                            "Every query runs in a read-only transaction that is rolled back",
                        ));
                    }
                });
            });
        });
    app.actions.extend(actions);
}

/// A menu of choices; returns the one picked this frame.
fn menu<T: Copy + PartialEq, const N: usize>(
    ui: &mut Ui,
    id: Id,
    name: &str,
    current: &str,
    choices: [(T, String); N],
    selected: T,
    app: &App,
) -> Option<T> {
    let (look, palette) = (app.look, app.palette);
    let mut picked = None;
    let combo = widgets::popup_button(
        ui,
        egui::ComboBox::from_id_salt(id).selected_text(widgets::galley(
            ui,
            current,
            egui::Color32::PLACEHOLDER,
            &look,
        )),
        &look,
        &palette,
        |ui| {
            for (choice, text) in &choices {
                let text = widgets::galley(ui, text, egui::Color32::PLACEHOLDER, &look);
                if ui.selectable_label(*choice == selected, text).clicked() {
                    picked = Some(*choice);
                }
            }
        },
    );
    let name = gettext(app.locale, name).into_owned();
    let value = current.to_owned();
    combo.response.widget_info(|| {
        let mut info = WidgetInfo::labeled(WidgetType::ComboBox, true, &name);
        info.current_text_value = Some(value.clone());
        info
    });
    picked
}

/// The macOS and standard footer: rows and time, the transaction, the
/// cursor and the server.
fn footer(app: &mut App, ui: &mut Ui, tab: ConnTabId, id: TabId) {
    let (look, palette, locale) = (app.look, app.palette, app.locale);
    let Some(workspace) = app.workspace(tab) else {
        return;
    };
    let Some(sql) = workspace.sql_tab(id) else {
        return;
    };
    let version = workspace.server_version.value.clone().unwrap_or_default();
    let (line, column) = sql.line_col();
    let summary = run_summary(app, sql);
    let ran = sql.run.value.is_some();
    egui::Panel::bottom(Id::new(("sql-footer", tab.0, id.0)))
        .exact_size(33.0)
        .resizable(false)
        .show_separator_line(false)
        .frame(Frame::new().fill(palette.panel).inner_margin(Margin::symmetric(20, 0)))
        .show(ui, |ui| {
            let full = ui.max_rect().expand2(egui::vec2(20.0, 0.0));
            widgets::hline(ui, full.x_range(), full.top() + 0.5, palette.outline);
            let status = palette.secondary.lerp_to_gamma(palette.dim, 0.5);
            let role = widgets::secondary(&look);
            ui.horizontal_centered(|ui| {
                ui.spacing_mut().item_spacing.x = 16.0;
                if let Some(summary) = &summary {
                    widgets::label(ui, role, summary, status, &look);
                }
                if ran {
                    widgets::label(
                        ui,
                        role,
                        &gettext(locale, "Read-only transaction · rolled back"),
                        status,
                        &look,
                    );
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    widgets::label(ui, role, &version, status, &look);
                    widgets::label(
                        ui,
                        role,
                        &format!("{} {line}, {} {column}", gettext(locale, "Ln"), gettext(locale, "Col")),
                        status,
                        &look,
                    );
                });
            });
        });
}

/// "5 rows · 14 ms" for the shown result.
fn run_summary(app: &App, sql: &crate::model::SqlTab) -> Option<String> {
    let (_, result) = sql.shown()?;
    let (rows, ..) = sql.shown_rows()?;
    Some(format!(
        "{} {} · {}",
        group_digits(rows.len() as u64),
        gettext(app.locale, if rows.len() == 1 { "row" } else { "rows" }),
        crate::ui::format::elapsed(result.elapsed)
    ))
}

/// The terminal status line's right side: "ln 12:21 · 5 rows · 14 ms ·
/// rolled back".
pub fn status_summary(app: &App, tab: ConnTabId) -> Option<String> {
    let sql = app.workspace(tab)?.active_sql_tab()?;
    let (line, column) = sql.line_col();
    let mut parts = vec![format!("ln {line}:{column}")];
    if let Some(summary) = run_summary(app, sql) {
        parts.push(summary);
    }
    if sql.run.value.is_some() {
        parts.push(gettext(app.locale, "rolled back").into_owned());
    }
    Some(parts.join(" · "))
}
```

Check against the code: `ButtonSpec::show(ui, height, look, palette)`, `widgets::label(ui, role, text, color, look) -> Response`, `look.label(&str) -> String` (lowercases in the terminal look; if it returns `Cow`, add `.into_owned()`), `look.control_height`, `format::elapsed`, `format::group_digits`. Create empty `src/ui/sql_text.rs` and `src/ui/sql_results.rs` with `pub fn show(_app: &mut App, _ui: &mut Ui, _tab: ConnTabId, _id: TabId) {}` for now.

- [ ] **Step 5: Dispatch, strip, sidebar, status line**

`workspace.rs`, in the central panel's `match active`, before the object branch:

```rust
                  Some(active_id)
                      if app
                          .workspace(tab)
                          .and_then(|workspace| workspace.sql_tab(active_id))
                          .is_some() =>
                  {
                      super::sql_editor::show(app, ui, tab, active_id);
                  }
```

(the row panel condition already needs `ObjectView::Data` from `active_object_tab()`, which is `None` for SQL tabs).

`status_line`: when `app.workspace(tab).and_then(|w| w.active_sql_tab()).is_some()`, use these hints and `sql_editor::status_summary(app, tab)` for the summary, and skip the struck-through editing keys:

```rust
            let hints: &[widgets::Hint] = if sql {
                &[
                    ("ctrl+enter", "run", true),
                    ("ctrl+shift+enter", "run all", true),
                    ("ctrl+.", "cancel", true),
                    ("esc", "leave editor", true),
                    ("ctrl+b", "tables", true),
                ]
            } else {
                &[/* the existing list */]
            };
```

`object_tabs.rs`: keep the `(TabId, String, bool)` list from Task 9 and add a flag `sql: bool`; `mac_tab` draws `Icon::SquareCode` instead of `Icon::Table` for SQL tabs. In the terminal look, after the tabs, draw the `+ sql` button with its hint:

```rust
                if look.terminal {
                    let label = gettext(locale, "New SQL editor");
                    let response = ButtonSpec::new("+ sql")
                        .shortcut("ctrl+t")
                        .label(&label)
                        .show(ui, height(&look) - 9.0, &look, &palette);
                    if response.clicked() {
                        actions.push(Action::NewSqlTab(tab));
                    }
                }
```

Terminal tab names are lower case (`query 1`): pass `look.label(&name)` for SQL tabs. The accessible name stays `"{name} tab"`.

`sidebar.rs`: reserve a footer on every look: `let footer = if look.terminal { FOOTER } else { SQL_BUTTON };` with `const SQL_BUTTON: f32 = 8.0 + 28.0 + 8.0;`, and after the tree on non-terminal looks:

```rust
            if !look.terminal {
                let rect = egui::Rect::from_min_size(
                    egui::pos2(full.left() + 12.0, full.bottom() - SQL_BUTTON + 8.0),
                    egui::vec2(full.width() - 24.0, 28.0),
                );
                let response = ButtonSpec::new(&gettext(locale, "SQL Editor"))
                    .icon(Icon::SquareCode)
                    .shortcut(&keys_label("Mod+T"))
                    .show_at(ui, rect, &look, &palette);
                if response.clicked() {
                    actions.push(Action::NewSqlTab(tab));
                }
            }
```

- [ ] **Step 6: Run the tests**

Run: `~/.cargo/bin/cargo test --locked --lib`
Expected: PASS, including `every_look_lays_out_at_small_and_large_sizes`.

- [ ] **Step 7: Look at it by hand**

Run the app against the demo (`~/.cargo/bin/cargo run -- --demo`), open a SQL tab with `Mod+T` in each look you can reach, and compare the toolbar, strip, sidebar button and footer with the "SQL editor" artboards. Fix spacing by eye. Do not commit screenshots.

- [ ] **Step 8: Format, lint, commit**

```bash
~/.cargo/bin/cargo fmt --all && ~/.cargo/bin/cargo clippy --locked --workspace --all-targets -- -D warnings
git add assets/icons/square-code.svg src/theme.rs src/ui
git commit -m "Draw SQL editor tabs: toolbar, footer and ways to open one

Run and Run all, the limit and timeout menus, the read-only badge, a
SQL Editor button in the sidebar and + sql in the terminal strip.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 14: The editor: highlighting, gutter, statement band

**Files:**
- Modify: `src/ui/sql_text.rs`, `src/ui/mod.rs` (remove the Task 12 `#[ignore]`)

**Interfaces:**
- Produces: `sql_text::show(app, ui, tab, id)`; `sql_text::color_of(kind: TokenKind, palette: &Palette) -> Color32`; `sql_text::layouter(look, role, palette, dialect)`.

- [ ] **Step 1: Write the failing tests**

In `sql_text.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tabletist_db::sql::TokenKind;

    #[test]
    fn each_token_kind_takes_its_palette_colour() {
        let palette = crate::theme::Palette::default();
        assert_eq!(color_of(TokenKind::Keyword, &palette), palette.accent);
        assert_eq!(color_of(TokenKind::String, &palette), palette.success);
        assert_eq!(color_of(TokenKind::Number, &palette), palette.orange);
        assert_eq!(color_of(TokenKind::Comment, &palette), palette.dim);
        assert_eq!(color_of(TokenKind::Identifier, &palette), palette.text);
    }
}
```

(If `Palette` has no `Default`, build one the way `testing.rs` or `theme.rs` tests do.)

In `src/ui/mod.rs`:

```rust
    #[test]
    fn the_editor_is_a_named_text_field_with_the_query() {
        let mut harness = Harness::new();
        let tab = harness.connect_fake();
        harness.press(Key::T, Modifiers::COMMAND);
        harness.frame(vec![egui::Event::Text("SELECT 1".into())]);
        let tree = harness.settle();
        assert!(crate::testing::node(&tree, "SQL", accesskit::Role::MultilineTextInput).is_some()
            || harness.has("SQL"));
        let workspace = harness.app.workspace(tab).unwrap();
        let sql = workspace.active_sql_tab().unwrap();
        assert_eq!(sql.text, "SELECT 1");
        assert_eq!(sql.cursor, 8);
    }

    #[test]
    fn escape_leaves_the_editor() {
        let mut harness = Harness::new();
        harness.connect_fake();
        harness.press(Key::T, Modifiers::COMMAND);
        assert!(harness.ctx.text_edit_focused() || harness.ctx.memory(|m| m.focused().is_some()));
        harness.press(Key::Escape, Modifiers::NONE);
        assert!(!harness.ctx.text_edit_focused());
    }
```

Remove the `#[ignore]` from `command_return_runs_while_typing`.

- [ ] **Step 2: Run to see them fail**

Run: `~/.cargo/bin/cargo test --locked --lib sql_text:: ui::tests::the_editor ui::tests::command_return`
Expected: FAIL.

- [ ] **Step 3: Implement** (`src/ui/sql_text.rs`)

```rust
//! The SQL editor's text: highlighted through the tokenizer, with a line
//! number gutter and a faint band behind the statement Run executes.

use std::sync::Arc;

use egui::{Color32, Galley, Id, Margin, Shape, Ui, WidgetInfo, WidgetType};
use tabletist_db::Dialect;
use tabletist_db::sql::{self, TokenKind};

use crate::app::App;
use crate::model::{ConnTabId, TabId};
use crate::theme::{Look, Palette};
use crate::typography::{Text, TextRole};

pub fn color_of(kind: TokenKind, palette: &Palette) -> Color32 {
    match kind {
        TokenKind::Keyword => palette.accent,
        TokenKind::String => palette.success,
        TokenKind::Number => palette.orange,
        TokenKind::Comment | TokenKind::ExecutableComment => palette.dim,
        TokenKind::Operator | TokenKind::Punctuation | TokenKind::Semicolon => palette.secondary,
        TokenKind::Identifier | TokenKind::QuotedIdentifier | TokenKind::Whitespace => palette.text,
    }
}

/// A text edit layouter that colours `dialect`'s tokens and never wraps:
/// one galley row per line, so the gutter can number rows.
pub fn layouter(
    look: &Look,
    role: TextRole,
    palette: &Palette,
    dialect: Dialect,
) -> impl FnMut(&Ui, &dyn egui::TextBuffer, f32) -> Arc<Galley> + use<> {
    let faces = look.faces;
    let palette = *palette;
    move |ui, buffer, _wrap| {
        let text = buffer.as_str();
        let mut out = Text::in_faces(faces);
        for token in sql::tokenize(dialect, text) {
            out = out.add(role, &text[token.range], color_of(token.kind, &palette));
        }
        if text.is_empty() {
            out = out.add(role, "", palette.text);
        }
        out.wrap(f32::INFINITY).layout(ui.ctx()).galley
    }
}

/// The editor's text role: monospace in every look.
fn role(look: &Look) -> TextRole {
    TextRole::pick(look, TextRole::InspectorValue, TextRole::OBody)
}

pub fn show(app: &mut App, ui: &mut Ui, tab: ConnTabId, id: TabId) {
    let (look, palette) = (app.look, app.palette);
    let Some(workspace) = app.workspace_mut(tab) else {
        return;
    };
    let dialect = workspace.driver.dialect();
    let Some(sql_tab) = workspace.sql_tab_mut(id) else {
        return;
    };
    let role = role(&look);
    let found = sql::statements(dialect, &sql_tab.text);
    let current = sql::statement_at(&found, sql_tab.cursor).cloned();
    let error_line = sql_tab.error_mark().map(|(line, _)| line);
    let focus = std::mem::take(&mut sql_tab.focus_editor);
    let lines = sql_tab.text.matches('\n').count() + 1;
    let digit = role.width(ui.ctx(), look.faces, "0");
    let gutter = (digit * lines.to_string().len().max(2) as f32 + 24.0).ceil();
    egui::ScrollArea::both()
        .id_salt(("sql-scroll", tab.0, id.0))
        .auto_shrink([false, false])
        .show(ui, |ui| {
            let band = ui.painter().add(Shape::Noop);
            let mut layouter = layouter(&look, role, &palette, dialect);
            let output = egui::TextEdit::multiline(&mut sql_tab.text)
                .id(Id::new(("sql-text", tab.0, id.0)))
                .code_editor()
                .lock_focus(true)
                .frame(egui::Frame::NONE)
                .margin(Margin {
                    left: gutter as i8,
                    right: 8,
                    top: 8,
                    bottom: 8,
                })
                .desired_width(f32::INFINITY)
                .desired_rows(12)
                .font(role.font_id(look.faces))
                .layouter(&mut layouter)
                .show(ui);
            output
                .response
                .widget_info(|| WidgetInfo::labeled(WidgetType::TextEdit, true, "SQL"));
            if focus {
                output.response.request_focus();
            }
            if let Some(range) = output.cursor_range {
                sql_tab.cursor = byte_offset(&sql_tab.text, range.primary.index);
            }
            let (cursor_line, _) = sql_tab.line_col();
            let galley = &output.galley;
            let origin = output.galley_pos;
            let row_rect = |line: usize| {
                galley
                    .rows
                    .get(line - 1)
                    .map(|row| row.rect().translate(origin.to_vec2()))
            };
            // The band behind the statement Run executes.
            if let Some(statement) = &current {
                let last = statement.start_line + statement.text.matches('\n').count();
                if let (Some(top), Some(bottom)) = (row_rect(statement.start_line), row_rect(last)) {
                    let rect = egui::Rect::from_min_max(
                        egui::pos2(output.response.rect.left(), top.top()),
                        egui::pos2(output.response.rect.right(), bottom.bottom()),
                    );
                    ui.painter().set(
                        band,
                        Shape::rect_filled(rect, 0.0, palette.accent.gamma_multiply(0.06)),
                    );
                }
            }
            // Line numbers: relative on the terminal look, the cursor's line
            // absolute.
            for line in 1..=galley.rows.len() {
                let Some(rect) = row_rect(line) else {
                    continue;
                };
                let number = if look.terminal && line != cursor_line {
                    line.abs_diff(cursor_line)
                } else {
                    line
                };
                let color = if Some(line) == error_line {
                    palette.danger
                } else if line == cursor_line {
                    palette.secondary
                } else {
                    palette.faint
                };
                crate::ui::widgets::paint_text_right(
                    ui,
                    output.response.rect.left() + gutter - 12.0,
                    rect.center().y,
                    Text::one(&look, role, &number.to_string(), color),
                );
            }
        });
}

/// The byte offset of a character index.
fn byte_offset(text: &str, chars: usize) -> usize {
    text.char_indices().nth(chars).map_or(text.len(), |(byte, _)| byte)
}
```

Check against egui 0.36 (the crmne fork): `TextEditOutput { response, galley, galley_pos, cursor_range, .. }`; `cursor_range.primary.index` is a char index; `galley.rows` items expose `rect()` (a `PlacedRow` in 0.36; if the method is named differently, use `row.pos` and `row.size`). `TextEdit::frame` takes a `Frame` (as `data_view.rs` uses); `Margin` fields are `i8`. `Text::add` chains by value as in `typography::layouter`. `code_editor()` already locks focus, so Tab inserts a tab; Escape surrenders focus by egui's default.

- [ ] **Step 4: Run the tests**

Run: `~/.cargo/bin/cargo test --locked --lib`
Expected: PASS.

- [ ] **Step 5: Look at it by hand**

Type a two-statement script in each look; check colours, the gutter (relative numbers on Omarchy), the band following the cursor, and the error line turning red after a failing run. Compare with the artboards by eye.

- [ ] **Step 6: Format, lint, commit**

```bash
~/.cargo/bin/cargo fmt --all && ~/.cargo/bin/cargo clippy --locked --workspace --all-targets -- -D warnings
git add src/ui
git commit -m "Highlight SQL in the editor with a line number gutter

Tokens take palette colours, the statement Run executes carries a faint
band, and the line that failed is marked.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 15: Results: grid, messages and states

**Files:**
- Modify: `src/ui/sql_results.rs`, `src/ui/data_view.rs` (extract the cell formatter), `src/ui/mod.rs` (tests)

**Interfaces:**
- Produces: `data_view::plain_cell(ctx, value, kind, look, full_precision) -> Cell` (shared with the table grid); `sql_results::show(app, ui, tab, id)`.

- [ ] **Step 1: Write the failing tests** (`src/ui/mod.rs` tests; `Harness::answer_sql` and `testing::rows_outcome` exist since Task 12)

```rust
    #[test]
    fn a_result_fills_the_grid() {
        let mut harness = Harness::new();
        let tab = harness.connect_fake();
        harness.press(Key::T, Modifiers::COMMAND);
        harness.frame(vec![egui::Event::Text("SELECT 1".into())]);
        harness.press(Key::Enter, Modifiers::COMMAND);
        harness.answer_sql(
            Ok(tabletist_db::ScriptOutcome {
                results: vec![tabletist_db::StatementResult {
                    elapsed: std::time::Duration::from_millis(14),
                    outcome: crate::testing::rows_outcome(5),
                }],
            }),
            None,
        );
        harness.settle();
        assert!(harness.has("Results 5"));
        assert!(harness.has("email"));
        assert!(harness.has("Statement at line 1 · 14 ms"));
        let _ = tab;
    }

    #[test]
    fn an_error_shows_in_messages_with_its_line() {
        let mut harness = Harness::new();
        harness.connect_fake();
        harness.press(Key::T, Modifiers::COMMAND);
        harness.frame(vec![egui::Event::Text("SELECT 1;\nSELECT x".into())]);
        harness.press(Key::Enter, Modifiers::COMMAND | Modifiers::SHIFT);
        harness.answer_sql(
            Ok(tabletist_db::ScriptOutcome {
                results: vec![
                    tabletist_db::StatementResult {
                        elapsed: std::time::Duration::ZERO,
                        outcome: crate::testing::rows_outcome(1),
                    },
                    tabletist_db::StatementResult {
                        elapsed: std::time::Duration::ZERO,
                        outcome: tabletist_db::StatementOutcome::Error {
                            error: tabletist_db::Error::query("no such column: x"),
                            position: None,
                        },
                    },
                ],
            }),
            None,
        );
        harness.settle();
        assert!(harness.has("Line 2: no such column: x"));
    }

    #[test]
    fn a_timeout_reads_as_cancelled_after_its_limit() {
        let mut harness = Harness::new();
        harness.connect_fake();
        harness.press(Key::T, Modifiers::COMMAND);
        harness.frame(vec![egui::Event::Text("SELECT 1".into())]);
        harness.press(Key::Enter, Modifiers::COMMAND);
        harness.answer_sql(
            Ok(tabletist_db::ScriptOutcome {
                results: vec![tabletist_db::StatementResult {
                    elapsed: std::time::Duration::from_secs(30),
                    outcome: tabletist_db::StatementOutcome::Cancelled,
                }],
            }),
            Some(crate::backend::CancelReason::Timeout(std::time::Duration::from_secs(30))),
        );
        harness.click("Messages");
        assert!(harness.has("Line 1: Cancelled after 30 s (timeout)"));
    }

    #[test]
    fn a_run_stopped_before_it_began_reads_as_cancelled() {
        let mut harness = Harness::new();
        harness.connect_fake();
        harness.press(Key::T, Modifiers::COMMAND);
        harness.frame(vec![egui::Event::Text("SELECT 1".into())]);
        harness.press(Key::Enter, Modifiers::COMMAND);
        harness.answer_sql(
            Ok(tabletist_db::ScriptOutcome::default()),
            Some(crate::backend::CancelReason::User),
        );
        harness.settle();
        assert!(harness.has("Cancelled"));
        assert!(!harness.has("Statement ran · no rows returned"));
    }

    #[test]
    fn before_a_run_the_results_say_how_to_run() {
        let mut harness = Harness::new();
        harness.connect_fake();
        harness.press(Key::T, Modifiers::COMMAND);
        assert!(harness.has(&format!(
            "Run a statement with {}",
            crate::ui::keys::keys_label("Mod+Return")
        )));
    }

    #[test]
    fn a_truncated_result_says_the_limit_was_reached() {
        let mut harness = Harness::new();
        harness.connect_fake();
        harness.press(Key::T, Modifiers::COMMAND);
        harness.frame(vec![egui::Event::Text("SELECT 1".into())]);
        harness.press(Key::Enter, Modifiers::COMMAND);
        let page = crate::testing::page(3, false);
        harness.answer_sql(
            Ok(tabletist_db::ScriptOutcome {
                results: vec![tabletist_db::StatementResult {
                    elapsed: std::time::Duration::ZERO,
                    outcome: tabletist_db::StatementOutcome::Rows {
                        columns: page.columns,
                        rows: page.rows,
                        truncated: true,
                    },
                }],
            }),
            None,
        );
        harness.settle();
        assert!(harness.has("First 1,000 rows (limit reached)"));
    }
```

- [ ] **Step 2: Run to see them fail**

Run: `~/.cargo/bin/cargo test --locked --lib ui::tests::a_result ui::tests::an_error ui::tests::a_timeout ui::tests::before_a_run ui::tests::a_truncated`
Expected: FAIL.

- [ ] **Step 3: Extract the cell formatter** (`data_view.rs`)

Move the non-tag branches of the grid's cell closure into:

```rust
/// A cell's text and style for any result grid: NULL, colours, JSON at a
/// glance, timestamps to the second unless `full_precision`.
pub fn plain_cell<'a>(
    ctx: &egui::Context,
    value: &'a tabletist_db::Value,
    kind: ValueKind,
    look: &Look,
    full_precision: bool,
) -> Cell<'a> {
    if value.is_null() {
        return Cell { text: "NULL".into(), null: true, style: Style::Plain };
    }
    if let Some(color) = format::color(value) {
        return Cell { text: format::cell_text(value), null: false, style: Style::Color(color) };
    }
    if let (ValueKind::Json, tabletist_db::Value::Text(text)) = (kind, value)
        && text.len() <= 16 * 1024
        && let Some(doc) = crate::ui::json_view::parsed_in(ctx, text)
    {
        let (count, strings) = crate::ui::json_view::summary(&doc);
        let shown = if look.terminal {
            strings.join(" · ")
        } else {
            strings.into_iter().next().unwrap_or_default()
        };
        return Cell { text: shown.into(), null: false, style: Style::Json(count) };
    }
    let text = format::cell_text(value);
    let text = if kind == ValueKind::Temporal && !full_precision {
        match format::to_the_second(&text) {
            std::borrow::Cow::Borrowed(_) => text,
            std::borrow::Cow::Owned(short) => short.into(),
        }
    } else {
        text
    };
    Cell { text, null: false, style: Style::Plain }
}
```

The table grid's closure becomes: NULL and colour checks happen in `plain_cell`, but the tag check sits between the colour and JSON checks today. Keep that order: in the table closure, call `plain_cell` only after checking tags:

```rust
                |row, col| {
                    let value = &page.rows[row][col];
                    if !value.is_null()
                        && format::color(value).is_none()
                        && let Some(style) = tags[col].style(value)
                    {
                        return Cell { text: format::cell_text(value), null: false, style };
                    }
                    plain_cell(&ctx, value, page.columns[col].kind, &look, full_precision)
                },
```

Run the existing grid tests to confirm nothing changed.

- [ ] **Step 4: Implement** (`src/ui/sql_results.rs`)

```rust
//! A SQL editor's results: the last statement's rows in the grid, or
//! every statement's outcome in Messages.

use egui::{Frame, Id, Margin, Ui};
use tabletist_db::{StatementOutcome, ValueKind};

use crate::app::App;
use crate::backend::CancelReason;
use crate::i18n::gettext;
use crate::model::{Action, ConnTabId, ResultPane, SqlTab, TabId};
use crate::theme::Icon;
use crate::ui::format::{elapsed, group_digits};
use crate::ui::grid::{self, Column};
use crate::ui::widgets;

pub fn show(app: &mut App, ui: &mut Ui, tab: ConnTabId, id: TabId) {
    header(app, ui, tab, id);
    let pane = app
        .workspace(tab)
        .and_then(|w| w.sql_tab(id))
        .map(|sql| sql.pane);
    match pane {
        Some(ResultPane::Messages) => messages(app, ui, tab, id),
        Some(ResultPane::Results) => results(app, ui, tab, id),
        None => {}
    }
}

fn header(app: &mut App, ui: &mut Ui, tab: ConnTabId, id: TabId) {
    let (look, palette, locale) = (app.look, app.palette, app.locale);
    let Some(sql) = app.workspace(tab).and_then(|w| w.sql_tab(id)) else {
        return;
    };
    let count = sql.dims().0;
    let pane = sql.pane;
    let info = sql.shown().and_then(|(index, result)| {
        let statement = sql.run.value.as_ref()?.statements.get(index)?;
        Some(format!(
            "{} {} · {}",
            gettext(locale, "Statement at line"),
            statement.first_line,
            elapsed(result.elapsed)
        ))
    });
    let running = sql.started.map(|started| started.elapsed());
    let mut actions = Vec::new();
    egui::Panel::top(Id::new(("sql-results-header", tab.0, id.0)))
        .exact_size(37.0)
        .resizable(false)
        .show_separator_line(false)
        .frame(Frame::new().fill(palette.window).inner_margin(Margin::symmetric(12, 0)))
        .show(ui, |ui| {
            ui.horizontal_centered(|ui| {
                let results = look.label(&format!("{} {}", gettext(locale, "Results"), count));
                if widgets::toggle(ui, pane == ResultPane::Results, &results, &look, &palette).clicked() {
                    actions.push(Action::SetResultPane { tab, sql_tab: id, pane: ResultPane::Results });
                }
                let messages = look.label(&gettext(locale, "Messages"));
                if widgets::toggle(ui, pane == ResultPane::Messages, &messages, &look, &palette).clicked() {
                    actions.push(Action::SetResultPane { tab, sql_tab: id, pane: ResultPane::Messages });
                }
                if let Some(info) = &info {
                    widgets::label(ui, widgets::secondary(&look), info, palette.secondary, &look);
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if let Some(running) = running {
                        let cancel = gettext(locale, "Cancel query");
                        if widgets::icon_button(ui, Icon::CircleX, &cancel, &look, &palette).clicked() {
                            actions.push(Action::CancelQuery(tab));
                        }
                        widgets::label(ui, widgets::secondary(&look), &elapsed(running), palette.secondary, &look);
                        ui.spinner();
                        ui.ctx().request_repaint_after(std::time::Duration::from_millis(250));
                    }
                });
            });
        });
    app.actions.extend(actions);
}

fn centered_note(ui: &mut Ui, app: &App, text: &str) {
    ui.centered_and_justified(|ui| {
        widgets::label(ui, widgets::body(&app.look), text, app.palette.secondary, &app.look);
    });
}

fn results(app: &mut App, ui: &mut Ui, tab: ConnTabId, id: TabId) {
    let (look, palette, locale) = (app.look, app.palette, app.locale);
    let full_precision = app.workspace(tab).is_some_and(|w| w.full_precision);
    let Some(sql) = app.workspace(tab).and_then(|w| w.sql_tab(id)) else {
        return;
    };
    if sql.run.value.is_none() {
        if sql.run.is_loading() {
            ui.centered_and_justified(|ui| {
                ui.spinner();
            });
        } else if let Some(error) = &sql.run.error {
            let error = error.clone();
            let mut retry = false;
            crate::ui::data_view::error_box(ui, &error, &look, &palette, locale, || retry = true);
            if retry {
                app.actions.push(Action::RunSql { tab, sql_tab: id, all: false });
            }
        } else {
            let hint = format!(
                "{} {}",
                gettext(locale, "Run a statement with"),
                crate::ui::keys::keys_label("Mod+Return")
            );
            centered_note(ui, app, &hint);
        }
        return;
    }
    let Some((columns, rows, truncated)) = sql.shown_rows() else {
        // No statement returned rows: say whether it was stopped.
        let cancelled = sql
            .run
            .value
            .as_ref()
            .filter(|run| run.outcome.was_cancelled())
            .map(|run| cancel_text(app, run.cancel));
        let note = cancelled
            .unwrap_or_else(|| gettext(locale, "Statement ran · no rows returned").into_owned());
        centered_note(ui, app, &note);
        return;
    };
    let grid_id = Id::new(("sql-grid", tab.0, id.0, sql.run.loaded.map(|r| r.0), full_precision));
    let selection = sql.selection;
    let limit = sql.limit;
    let grid_columns: Vec<Column> = columns
        .iter()
        .map(|column| Column {
            name: &column.name,
            type_line: column.type_name.clone(),
            numeric: column.kind == ValueKind::Numeric,
            sort: None,
            key: false,
            flexible: column.kind == ValueKind::Json,
        })
        .collect();
    if truncated {
        egui::Panel::bottom(Id::new(("sql-truncated", tab.0, id.0)))
            .exact_size(28.0)
            .resizable(false)
            .show_separator_line(false)
            .frame(Frame::new().fill(palette.panel).inner_margin(Margin::symmetric(12, 0)))
            .show(ui, |ui| {
                let note = format!(
                    "{} {} {}",
                    gettext(locale, "First"),
                    group_digits(u64::from(limit)),
                    gettext(locale, "rows (limit reached)")
                );
                ui.horizontal_centered(|ui| {
                    widgets::label(ui, widgets::secondary(&look), &note, palette.secondary, &look);
                });
            });
    }
    let ctx = ui.ctx().clone();
    let output = grid::show(
        ui,
        grid_id,
        &grid_columns,
        rows.len(),
        0,
        selection,
        &palette,
        &look,
        |row, col| {
            crate::ui::data_view::plain_cell(&ctx, &rows[row][col], columns[col].kind, &look, full_precision)
        },
    );
    if let Some(cell) = output.clicked {
        app.actions.push(Action::SelectCell { tab, object_tab: id, cell });
    }
}

fn cancel_text(app: &App, cancel: Option<CancelReason>) -> String {
    match cancel {
        Some(CancelReason::Timeout(after)) => format!(
            "{} {} s ({})",
            gettext(app.locale, "Cancelled after"),
            after.as_secs(),
            gettext(app.locale, "timeout")
        ),
        _ => gettext(app.locale, "Cancelled").into_owned(),
    }
}

/// One line per statement: its line, then what it did.
fn message_lines(app: &App, sql: &SqlTab) -> Vec<(String, bool)> {
    let locale = app.locale;
    if let Some(error) = &sql.run.error {
        return vec![(error.to_string(), true)];
    }
    let Some(run) = &sql.run.value else {
        return Vec::new();
    };
    // Stopped before its first statement began.
    if run.outcome.results.is_empty() {
        return vec![(cancel_text(app, run.cancel), true)];
    }
    run.statements
        .iter()
        .enumerate()
        .map(|(index, statement)| {
            let result = run.outcome.results.get(index);
            let (text, bad) = match result.map(|result| (&result.outcome, result.elapsed)) {
                None => (gettext(locale, "Not run").into_owned(), false),
                Some((StatementOutcome::Rows { rows, truncated, .. }, time)) => (
                    format!(
                        "{} {}{} · {}",
                        group_digits(rows.len() as u64),
                        gettext(locale, if rows.len() == 1 { "row" } else { "rows" }),
                        if *truncated { format!(" ({})", gettext(locale, "limit reached")) } else { String::new() },
                        elapsed(time)
                    ),
                    false,
                ),
                Some((StatementOutcome::Done { affected: Some(count) }, time)) if *count > 0 => (
                    format!("{} {} · {}", group_digits(*count), gettext(locale, "rows affected"), elapsed(time)),
                    false,
                ),
                Some((StatementOutcome::Done { .. }, time)) => {
                    (format!("{} · {}", gettext(locale, "Statement ran"), elapsed(time)), false)
                }
                Some((StatementOutcome::Error { error, position }, _)) => {
                    let place = position
                        .map(|position| {
                            let (_, column) = statement.line_col(position);
                            format!(" ({} {column})", gettext(locale, "col"))
                        })
                        .unwrap_or_default();
                    (format!("{error}{place}"), true)
                }
                Some((StatementOutcome::Cancelled, _)) => (cancel_text(app, run.cancel), true),
            };
            (
                format!("{} {}: {text}", gettext(locale, "Line"), statement.first_line),
                bad,
            )
        })
        .collect()
}

fn messages(app: &mut App, ui: &mut Ui, tab: ConnTabId, id: TabId) {
    let (look, palette) = (app.look, app.palette);
    let Some(sql) = app.workspace(tab).and_then(|w| w.sql_tab(id)) else {
        return;
    };
    let lines = message_lines(app, sql);
    egui::ScrollArea::vertical()
        .id_salt(("sql-messages", tab.0, id.0))
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.add_space(8.0);
            for (text, bad) in &lines {
                ui.horizontal(|ui| {
                    ui.add_space(12.0);
                    let color = if *bad { palette.danger } else { palette.text };
                    widgets::label(ui, widgets::code(&look), text, color, &look);
                });
            }
        });
}
```

A refused run shows the error's own text, which already starts with `line N:`. Keep the `Line N:` prefix capitalised as the test expects.

- [ ] **Step 5: Run the tests**

Run: `~/.cargo/bin/cargo test --locked --lib`
Expected: PASS.

- [ ] **Step 6: Look at it by hand**

Run a query returning rows, one that errors, one that is refused (`COMMIT`), a slow one you cancel, and one that times out (set the timeout to 10 s and run `SELECT pg_sleep(20)` on a PostgreSQL connection, or a recursive CTE on the demo). Compare the header, grid, messages and footer with the artboards.

- [ ] **Step 7: Format, lint, commit**

```bash
~/.cargo/bin/cargo fmt --all && ~/.cargo/bin/cargo clippy --locked --workspace --all-targets -- -D warnings
git add src/ui
git commit -m "Show SQL results in the grid and every statement in Messages

The last statement's rows fill the grid; Messages gives each statement's
rows, count, error or cancel, and statements that did not run.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 16: Docs and the full check

**Files:**
- Modify: `README.md`

- [ ] **Step 1: README**

Under "What it does", after the filter bar line:

```markdown
- A SQL editor per connection (Cmd/Ctrl+T): run the statement at the cursor
  (Cmd/Ctrl+Return) or the whole script, with a row limit and a timeout.
  Every run happens in a read-only transaction that is rolled back, and
  statements that would leave it are refused.
```

and change the quick open line's shortcut list if it names `Cmd/Ctrl+T` for connections (it moves to `Cmd/Ctrl+O`).

- [ ] **Step 2: Full checks**

```bash
~/.cargo/bin/cargo fmt --all --check
~/.cargo/bin/cargo clippy --locked --workspace --all-targets -- -D warnings
~/.cargo/bin/cargo test --locked --workspace --all-targets
RUSTDOCFLAGS='-D warnings' ~/.cargo/bin/cargo doc --locked --workspace --no-deps
docker compose up -d --build --wait postgres mysql
TABLETIST_TEST_PG_URL=postgres://tabletist:tabletist@localhost:55432/tabletist TABLETIST_TEST_MYSQL_URL=mysql://tabletist:tabletist@localhost:53306/tabletist ~/.cargo/bin/cargo test --locked --workspace
```

Expected: all pass. Report which suites ran against servers and that macOS and Windows were only compiled if so.

- [ ] **Step 3: Commit**

```bash
git add README.md
git commit -m "Describe the SQL editor in the README

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

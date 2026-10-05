# Exhaustive Engine Matches Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Every place that depends on the database engine names every engine, so adding a `Driver` or a `Dialect` variant fails to compile wherever nobody has decided for it.

**Architecture:** The code compares a `Driver` or a `Dialect` with one variant in 42 places (`==`, `!=`), and one `match` has a `_` arm for the driver. Each becomes a `match` that names all three variants. Where a match guard in the SQL tokenizer needs a yes or no, it asks a private method on `Dialect` whose body is such a match. A test that reads the sources, `tests/engines.rs`, keeps comparisons from coming back, the way `src/env.rs` already does for `Environment`.

**Tech Stack:** Rust 2024. No new dependency.

**Spec:** none. The request was "turn those `==` checks into exhaustive matches", after a survey of what adding a database driver takes found that the compiler only points at the exhaustive matches.

**What was checked before this plan was written:** the whole change was built as a draft on this branch. With it applied, the four checks of `AGENTS.md` passed on Linux (1265 tests of the app, 272 of `tabletist-db`'s library, the SQLite suite and `tests/engines.rs`). The test of Task 1 was then run alone on the untouched sources, where it passes with its list and, without the list, names all 42 comparisons. The diffs below are that draft, file by file. The PostgreSQL and MySQL suites did not run: this machine's session cannot reach Docker, so they printed "skipped". CI runs them. Nothing was compiled for macOS or Windows; no changed line is behind a `cfg`.

---

## Rules of the repository that bite here

- A refactoring: nothing behaves differently, no string a user reads changes, and no test other than the new one is added, changed or removed.
- No em dashes anywhere, in code, comments, tests or docs.
- One topic per commit. Every commit passes all four checks:

```bash
~/.cargo/bin/cargo fmt --all --check
~/.cargo/bin/cargo clippy --locked --workspace --all-targets -- -D warnings
~/.cargo/bin/cargo test --locked --workspace --all-targets
RUSTDOCFLAGS='-D warnings' ~/.cargo/bin/cargo doc --locked --workspace --no-deps
```

Expected: no output from `fmt`, `Finished` from `clippy` and `doc`, and every `test result:` line `ok` with `0 failed`.

- `cargo` is `~/.cargo/bin/cargo` (the `mise` shim fails here). Builds use the worktree's own `target/`.
- `clippy.toml` allows `unwrap` only inside tests. The helpers of an integration test are not inside one, so `tests/engines.rs` uses `expect`.
- Each diff below applies with `git apply` from the repository root, or by hand. After applying, `cargo fmt --all --check` must print nothing: the diffs are already formatted.

## Decisions to review

1. **A match at each place, not one "file or server" answer.** Twelve places in the app ask whether the driver is SQLite, meaning "a file: no host, no user, no password, no TLS, no tunnel". They become twelve matches with `Driver::Sqlite` on one side and `Driver::Postgres | Driver::MySql` on the other, as `src/model.rs`, `src/env.rs` and `src/app.rs` already write theirs. A fourth driver is then asked the question twelve times. One answer on `Driver` (a `kind()` returning `File` or `Server`) would ask it once. That is a design of its own, and it is easier to judge with a fourth driver in hand, so it is left out.
2. **The tokenizer asks named questions.** `next` in `sql.rs` is one `match` on a byte, and an arm such as ``b'#' if dialect == Dialect::MySql`` has to fall through to the arms below it when the answer is no. A guard cannot hold a `match`, so each guard asks a private method on `Dialect` (`hash_comments`, `nests_comments`, ...), ten in all, in an `impl Dialect` block in `sql.rs` beside the code that reads text. Where the arm's body can hold the match, it does, and no method is added.
3. **Two places are rewritten rather than translated.**
   - `ConnectionForm::password_can_be_intercepted` (`src/model.rs`) becomes one match on the driver, then on the TLS mode. Same truth table: for PostgreSQL `disable` and `prefer` are unchecked, `require` is unchecked without a CA file, the two `verify` modes are checked; for MySQL `require` is unchecked whatever the CA file; never for SQLite.
   - `sql::refusal` checks what each dialect's own tokens can hide (MySQL's `/*! */`, PostgreSQL's `U&` names, SQLite's refused PRAGMAs) in one match at the top. SQLite's check moves up past the collecting of the words. That collecting has no effect but its result, and the checks of the other two dialects never ran for SQLite, so the same statements are refused with the same words.
4. **The test sees comparisons, not everything.** `tests/engines.rs` finds `==`, `!=` and `matches!` against a variant, also behind `&` or in `Some(`. It reads text, not Rust, so it does not find a `_` arm in a match on the driver, a comparison written `self == Self::MySql` inside `impl Driver` or `impl Dialect`, or `if let Driver::Sqlite = driver`. None of the last two exists. There was one `_` arm, in `ConnectSpec::effective_tls`; Task 3 names the drivers there. `AGENTS.md` states the rule for all of them and says what the test does not find.
5. **Tests may compare.** `assert_eq!(form.driver, Driver::MySql)` and the like stay. The test skips each file's `tests` module, as the tests in `src/env.rs` do. A file that is tests throughout (`src/testing.rs`, `src/shots.rs`, `src/ui/complete_tests.rs`, `src/ui/env_tests.rs`) is read as code: an `==` against a variant there would fail the test. None has one, and `assert_eq!` is not a comparison the scan finds.

## File structure

| File | What changes |
|---|---|
| `tests/engines.rs` | new: the scan, a test of the scan, and the rule |
| `crates/tabletist-db/src/sql.rs` | ten questions on `Dialect` for the tokenizer; `refusal` and `set_refusal` match on the dialect |
| `crates/tabletist-db/src/sql/format.rs` | `structure` and `dashes` match on the dialect |
| `crates/tabletist-db/src/spec.rs` | `summary` matches on the driver; `effective_tls` loses its `_` |
| `crates/tabletist-db/src/lib.rs` | `connect_with` matches on the driver for the tunnel |
| `src/app.rs` | `Action::SetDriver` and `after_connect` |
| `src/model.rs` | `password_can_be_intercepted` and `to_saved` |
| `src/ui/connect_dialog/sheet.rs`, `terminal.rs` | the dialog's sections |
| `src/ui/picker.rs` | `is_local` and `target` |
| `src/ui/workspace.rs` | `connecting`, `failure_title`, `bar_info`, `card_rows`, and a new `is_remote` the last two share |
| `AGENTS.md` | the rule, under Architecture |

---

### Task 1: A test that finds the comparisons

The test comes first so that each later task starts from a failure that lists its places. Until those tasks are done, the files they change are on a list the test skips.

**Files:**
- Create: `tests/engines.rs`

- [ ] **Step 1: Write the test**

Create `tests/engines.rs`:

```rust
//! Code that depends on the database engine answers for every engine.

use std::path::{Path, PathBuf};

/// Every Rust file of the app and of the database layer, by its path from
/// the repository root, with its text.
fn sources() -> Vec<(String, String)> {
    fn walk(dir: &Path, into: &mut Vec<PathBuf>) {
        for entry in std::fs::read_dir(dir).expect("read a source directory") {
            let path = entry.expect("read a directory entry").path();
            if path.is_dir() {
                walk(&path, into);
            } else if path.extension().is_some_and(|ext| ext == "rs") {
                into.push(path);
            }
        }
    }
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut paths = Vec::new();
    walk(&root.join("src"), &mut paths);
    walk(&root.join("crates/tabletist-db/src"), &mut paths);
    assert!(paths.len() > 40, "found the sources");
    paths.sort();
    paths
        .into_iter()
        .map(|path| {
            let name = path.strip_prefix(root).expect("a path under the root");
            // Windows checks the sources out with CRLF line ends.
            let text = std::fs::read_to_string(&path).expect("read a source file");
            (
                name.display().to_string().replace('\\', "/"),
                text.replace("\r\n", "\n"),
            )
        })
        .collect()
}

/// Files whose comparisons are still to become matches. Each task of the
/// plan takes its own out; the last one removes the list.
const PENDING: &[&str] = &[
    "crates/tabletist-db/src/lib.rs",
    "crates/tabletist-db/src/spec.rs",
    "crates/tabletist-db/src/sql.rs",
    "crates/tabletist-db/src/sql/format.rs",
    "src/app.rs",
    "src/model.rs",
    "src/ui/connect_dialog/sheet.rs",
    "src/ui/connect_dialog/terminal.rs",
    "src/ui/picker.rs",
    "src/ui/workspace.rs",
];

/// The code of a file on one line: no comment lines, and nothing from its
/// `tests` module on. A comparison split over lines reads as one, and a
/// test may compare what it likes.
fn code(text: &str) -> String {
    let text = text
        .find("#[cfg(test)]\nmod tests {")
        .map_or(text, |at| &text[..at]);
    text.lines()
        .map(str::trim)
        .filter(|line| !line.starts_with("//"))
        .collect::<Vec<_>>()
        .join(" ")
}

/// The comparisons of a `Driver` or a `Dialect` with one of its variants in
/// `code`: `==`, `!=` and `matches!`, each with a little of what is around.
/// A variant behind `&` or in `Some(` is compared all the same. One written
/// `Self::` is not found: only the two enums' own methods can write that.
fn comparisons(code: &str) -> Vec<String> {
    let mut found = Vec::new();
    for name in ["Driver::", "Dialect::"] {
        for (at, _) in code.match_indices(name) {
            let before = code[..at]
                .trim_end_matches("tabletist_db::")
                .trim_end_matches("crate::")
                .trim_end_matches('&')
                .trim_end_matches("Some(")
                .trim_end();
            let variant = code[at + name.len()..]
                .chars()
                .take_while(char::is_ascii_alphanumeric)
                .count();
            let end = at + name.len() + variant;
            let after = code[end..].trim_start();
            // Inside a `matches!(` that has not closed yet.
            let in_matches = before.rfind("matches!(").is_some_and(|call| {
                let mut depth = 1;
                for c in before[call + "matches!(".len()..].chars() {
                    match c {
                        '(' => depth += 1,
                        ')' => depth -= 1,
                        _ => {}
                    }
                    if depth == 0 {
                        return false;
                    }
                }
                true
            });
            if before.ends_with("==")
                || before.ends_with("!=")
                || after.starts_with("==")
                || after.starts_with("!=")
                || in_matches
            {
                let start = code[..at]
                    .char_indices()
                    .rev()
                    .nth(29)
                    .map_or(0, |(index, _)| index);
                found.push(code[start..end].to_owned());
            }
        }
    }
    found
}

#[test]
fn the_scan_finds_each_way_to_compare() {
    for text in [
        "if spec.driver == Driver::Sqlite {",
        "let remote = tabletist_db::Driver::Sqlite != workspace.driver;",
        "b'#' if dialect\n    == Dialect::MySql => comment(),",
        "if matches!(self.dialect, Dialect::Postgres | Dialect::Sqlite) {",
        "if matches!(driver.dialect(), Dialect::MySql) {",
        "let file = saved.map(|saved| saved.driver) == Some(Driver::Sqlite);",
        "if drivers.iter().any(|driver| driver == &Driver::Sqlite) {",
    ] {
        assert!(!comparisons(&code(text)).is_empty(), "{text}");
    }
    for text in [
        "match driver {\n    Driver::Sqlite => true,\n    Driver::Postgres | Driver::MySql => false,\n}",
        "form.driver = Driver::Sqlite;",
        "// if driver == Driver::Sqlite",
        "let tokens = tokenize(Dialect::Postgres, text);",
        "#[cfg(test)]\nmod tests {\n    assert!(driver == Driver::Sqlite);\n}",
        "Dialect::Postgres if first == \"PREPARE\" => refuse(),",
        "if matches!(kind, TokenKind::Word) && is_keyword(Dialect::Postgres, word) {",
    ] {
        assert_eq!(comparisons(&code(text)), [""; 0], "{text}");
    }
}

#[test]
fn no_code_compares_a_driver_or_a_dialect() {
    // A comparison gives an engine added later the other side's answer
    // without anyone choosing it. A `match` that names every engine does
    // not compile until someone has.
    let found: Vec<String> = sources()
        .iter()
        .filter(|(path, _)| !PENDING.contains(&path.as_str()))
        .flat_map(|(path, text)| {
            comparisons(&code(text))
                .into_iter()
                .map(move |found| format!("{path}: {found}"))
        })
        .collect();
    assert!(
        found.is_empty(),
        "match on every engine instead:\n{}",
        found.join("\n")
    );
}
```

- [ ] **Step 2: Run it**

Run: `~/.cargo/bin/cargo test --locked --test engines`
Expected: `test result: ok. 2 passed`.

- [ ] **Step 3: See that it bites**

Delete the line `"src/ui/picker.rs",` from `PENDING` and run the same command.
Expected: `no_code_compares_a_driver_or_a_dialect` FAILS with "match on every engine instead:" and two lines that start with `src/ui/picker.rs:`, each ending in `== tabletist_db::Driver::Sqlite`. Put the line back and see the test pass again.

- [ ] **Step 4: Run the four checks**

```bash
~/.cargo/bin/cargo fmt --all --check
~/.cargo/bin/cargo clippy --locked --workspace --all-targets -- -D warnings
~/.cargo/bin/cargo test --locked --workspace --all-targets
RUSTDOCFLAGS='-D warnings' ~/.cargo/bin/cargo doc --locked --workspace --no-deps
```

Expected: no output from `fmt`, `Finished` from `clippy` and `doc`, and every `test result:` line `ok` with `0 failed`.

- [ ] **Step 5: Commit**

```bash
git add tests/engines.rs
git commit -m "Find the places that compare a driver or a dialect"
```

---

### Task 2: SQL text

The tokenizer, the read-only guard of the SQL editor and the formatter: 27 comparisons of a `Dialect`.

**Files:**
- Modify: `crates/tabletist-db/src/sql.rs`, `crates/tabletist-db/src/sql/format.rs`, `tests/engines.rs`

- [ ] **Step 1: Take the two files off the list**

In `tests/engines.rs`, delete these two lines from `PENDING`:

```rust
    "crates/tabletist-db/src/sql.rs",
    "crates/tabletist-db/src/sql/format.rs",
```

- [ ] **Step 2: See the test fail**

Run: `~/.cargo/bin/cargo test --locked --test engines`
Expected: FAIL with 27 lines, 25 that start with `crates/tabletist-db/src/sql.rs:` and 2 with `crates/tabletist-db/src/sql/format.rs:`.

- [ ] **Step 3: Change `sql.rs`**

The first half of the diff is the tokenizer: the ten questions, and the guards of `next`, `dash_comment`, `is_word_part` and `line_end` that ask them. The second half is `refusal` and `set_refusal` (decision 3). `postgres_or_mysql` goes: the arm that used it now names the dialects.

```diff
diff --git i/crates/tabletist-db/src/sql.rs w/crates/tabletist-db/src/sql.rs
index fec8aa6..52f8639 100644
--- i/crates/tabletist-db/src/sql.rs
+++ w/crates/tabletist-db/src/sql.rs
@@ -146,6 +146,90 @@ pub fn is_keyword(dialect: Dialect, word: &str) -> bool {
     keywords(dialect).any(|keyword| keyword.eq_ignore_ascii_case(word))
 }
 
+/// How a dialect reads text. A guard in the tokenizer asks one of these
+/// rather than naming a dialect, so every rule answers for every dialect.
+impl Dialect {
+    /// A byte-order mark where a token starts reads as a space.
+    fn bom_is_space(self) -> bool {
+        match self {
+            Self::Sqlite => true,
+            Self::Postgres | Self::MySql => false,
+        }
+    }
+
+    /// `#` starts a line comment.
+    fn hash_comments(self) -> bool {
+        match self {
+            Self::MySql => true,
+            Self::Postgres | Self::Sqlite => false,
+        }
+    }
+
+    /// `/*! ... */` and MariaDB's `/*M! ... */` are run, not skipped.
+    fn runs_comments(self) -> bool {
+        match self {
+            Self::MySql => true,
+            Self::Postgres | Self::Sqlite => false,
+        }
+    }
+
+    /// A block comment inside a block comment has an end of its own.
+    fn nests_comments(self) -> bool {
+        match self {
+            Self::Postgres => true,
+            Self::MySql | Self::Sqlite => false,
+        }
+    }
+
+    /// A backslash in a `'...'` string escapes the next character.
+    fn backslash_escapes(self) -> bool {
+        match self {
+            Self::MySql => true,
+            Self::Postgres | Self::Sqlite => false,
+        }
+    }
+
+    /// `E'...'` is a string with backslash escapes.
+    fn escape_strings(self) -> bool {
+        match self {
+            Self::Postgres => true,
+            Self::MySql | Self::Sqlite => false,
+        }
+    }
+
+    /// `` `name` `` is a quoted name.
+    fn backtick_names(self) -> bool {
+        match self {
+            Self::MySql | Self::Sqlite => true,
+            Self::Postgres => false,
+        }
+    }
+
+    /// `[name]` is a quoted name.
+    fn bracket_names(self) -> bool {
+        match self {
+            Self::Sqlite => true,
+            Self::Postgres | Self::MySql => false,
+        }
+    }
+
+    /// `$tag$ ... $tag$` is a string and `$1` a parameter.
+    fn dollar_quotes(self) -> bool {
+        match self {
+            Self::Postgres => true,
+            Self::MySql | Self::Sqlite => false,
+        }
+    }
+
+    /// `$` can stand inside a word.
+    fn dollar_in_words(self) -> bool {
+        match self {
+            Self::Postgres | Self::MySql => true,
+            Self::Sqlite => false,
+        }
+    }
+}
+
 /// Splits `text` into tokens that cover it end to end.
 pub fn tokenize(dialect: Dialect, text: &str) -> Vec<Token> {
     let bytes = text.as_bytes();
@@ -173,53 +257,53 @@ fn next(dialect: Dialect, text: &str, start: usize) -> (TokenKind, usize) {
         b if is_space(b) => (TokenKind::Whitespace, scan(bytes, start, is_space)),
         // SQLite reads a byte-order mark where a token starts as a space,
         // so the word after it is a word of its own.
-        0xEF if dialect == Dialect::Sqlite && peek(1) == Some(0xBB) && peek(2) == Some(0xBF) => {
+        0xEF if dialect.bom_is_space() && peek(1) == Some(0xBB) && peek(2) == Some(0xBF) => {
             (TokenKind::Whitespace, start + 3)
         }
         b'-' if peek(1) == Some(b'-') && dash_comment(dialect, peek(2)) => {
             (TokenKind::Comment, line_end(dialect, bytes, start))
         }
-        b'#' if dialect == Dialect::MySql => (TokenKind::Comment, line_end(dialect, bytes, start)),
+        b'#' if dialect.hash_comments() => (TokenKind::Comment, line_end(dialect, bytes, start)),
         b'/' if peek(1) == Some(b'*') => {
             // `/*!` and MariaDB's `/*M!`: MySQL runs what is inside.
             let executable =
                 peek(2) == Some(b'!') || (peek(2) == Some(b'M') && peek(3) == Some(b'!'));
-            let kind = if dialect == Dialect::MySql && executable {
+            let kind = if dialect.runs_comments() && executable {
                 TokenKind::ExecutableComment
             } else {
                 TokenKind::Comment
             };
             (
                 kind,
-                block_comment_end(bytes, start, dialect == Dialect::Postgres),
+                block_comment_end(bytes, start, dialect.nests_comments()),
             )
         }
         b'\'' => (
             TokenKind::String,
-            quoted_end(bytes, start, b'\'', dialect == Dialect::MySql),
+            quoted_end(bytes, start, b'\'', dialect.backslash_escapes()),
         ),
-        b'E' | b'e' if dialect == Dialect::Postgres && peek(1) == Some(b'\'') => {
+        b'E' | b'e' if dialect.escape_strings() && peek(1) == Some(b'\'') => {
             (TokenKind::String, escape_string_end(bytes, start))
         }
-        b'"' if dialect == Dialect::MySql => {
-            (TokenKind::String, quoted_end(bytes, start, b'"', true))
-        }
-        b'"' => (
-            TokenKind::QuotedIdentifier,
-            quoted_end(bytes, start, b'"', false),
-        ),
-        b'`' if dialect != Dialect::Postgres => (
+        b'"' => match dialect {
+            Dialect::MySql => (TokenKind::String, quoted_end(bytes, start, b'"', true)),
+            Dialect::Postgres | Dialect::Sqlite => (
+                TokenKind::QuotedIdentifier,
+                quoted_end(bytes, start, b'"', false),
+            ),
+        },
+        b'`' if dialect.backtick_names() => (
             TokenKind::QuotedIdentifier,
             quoted_end(bytes, start, b'`', false),
         ),
-        b'[' if dialect == Dialect::Sqlite => {
+        b'[' if dialect.bracket_names() => {
             let end = bytes[start..]
                 .iter()
                 .position(|&b| b == b']')
                 .map_or(bytes.len(), |offset| start + offset + 1);
             (TokenKind::QuotedIdentifier, end)
         }
-        b'$' if dialect == Dialect::Postgres => match dollar_tag(bytes, start) {
+        b'$' if dialect.dollar_quotes() => match dollar_tag(bytes, start) {
             Some(tag_end) => {
                 let tag = &bytes[start..tag_end];
                 let body = find(bytes, tag_end, tag).map_or(bytes.len(), |at| at + tag.len());
@@ -258,7 +342,10 @@ fn next(dialect: Dialect, text: &str, start: usize) -> (TokenKind, usize) {
 /// `--` starts a comment everywhere but MySQL, where whitespace or a
 /// control character (or the end) must follow: `1--1` is arithmetic.
 fn dash_comment(dialect: Dialect, after: Option<u8>) -> bool {
-    dialect != Dialect::MySql || after.is_none_or(|b| is_space(b) || b.is_ascii_control())
+    match dialect {
+        Dialect::MySql => after.is_none_or(|b| is_space(b) || b.is_ascii_control()),
+        Dialect::Postgres | Dialect::Sqlite => true,
+    }
 }
 
 fn is_word_start(byte: u8) -> bool {
@@ -269,7 +356,7 @@ fn is_word_part(dialect: Dialect, byte: u8) -> bool {
     byte.is_ascii_alphanumeric()
         || byte == b'_'
         || byte >= 0x80
-        || (byte == b'$' && dialect != Dialect::Sqlite)
+        || (byte == b'$' && dialect.dollar_in_words())
 }
 
 fn scan(bytes: &[u8], from: usize, keep: impl Fn(u8) -> bool) -> usize {
@@ -289,7 +376,10 @@ fn is_space(byte: u8) -> bool {
 /// The end of a line comment: the newline stays outside it. PostgreSQL ends
 /// it at a carriage return too; MySQL and SQLite only at a line feed.
 fn line_end(dialect: Dialect, bytes: &[u8], from: usize) -> usize {
-    let cr_ends = dialect == Dialect::Postgres;
+    let cr_ends = match dialect {
+        Dialect::Postgres => true,
+        Dialect::MySql | Dialect::Sqlite => false,
+    };
     scan(bytes, from, |b| b != b'\n' && !(cr_ends && b == b'\r'))
 }
 
@@ -591,76 +681,86 @@ fn word_of(text: &str, token: &Token) -> Option<String> {
 /// on tokens, so `SELECT 'COMMIT'` and a column named `end_date` pass.
 pub fn refusal(dialect: Dialect, statement: &str) -> Option<String> {
     let tokens = tokenize(dialect, statement);
-    if dialect == Dialect::MySql
-        && tokens
+    // What the dialect's own tokens can hide from the words below.
+    let hidden = match dialect {
+        Dialect::MySql => tokens
             .iter()
             .any(|token| token.kind == TokenKind::ExecutableComment)
-    {
-        return Some("a /*! */ comment".into());
-    }
-    // A U&"..." name can spell any name in escapes, set_config included.
-    if dialect == Dialect::Postgres && unicode_name(statement, &tokens) {
-        return Some("a U& name".into());
+            .then(|| "a /*! */ comment".to_owned()),
+        // A U&"..." name can spell any name in escapes, set_config included.
+        Dialect::Postgres => unicode_name(statement, &tokens).then(|| "a U& name".to_owned()),
+        Dialect::Sqlite => refused_pragma(statement, &tokens).map(|name| format!("PRAGMA {name}")),
+    };
+    if hidden.is_some() {
+        return hidden;
     }
     let mut words: Vec<String> = tokens
         .iter()
         .filter_map(|token| word_of(statement, token))
         .collect();
-    // MariaDB's CREATE OR REPLACE USER / ROLE: match them as CREATE USER.
-    if dialect == Dialect::MySql
-        && words.len() > 2
-        && words[0] == "CREATE"
-        && words[1] == "OR"
-        && words[2] == "REPLACE"
-    {
-        words.drain(1..3);
+    match dialect {
+        // MariaDB's CREATE OR REPLACE USER / ROLE: match them as CREATE USER.
+        Dialect::MySql => {
+            if words.len() > 2 && words[0] == "CREATE" && words[1] == "OR" && words[2] == "REPLACE"
+            {
+                words.drain(1..3);
+            }
+        }
+        Dialect::Postgres | Dialect::Sqlite => {}
     }
     let word = |index: usize| words.get(index).map(String::as_str).unwrap_or_default();
     let guarded_name = || words.iter().skip(1).find(|name| is_guarded_setting(name));
-    let postgres_or_mysql = dialect != Dialect::Sqlite;
-    if dialect == Dialect::Sqlite
-        && let Some(name) = refused_pragma(statement, &tokens)
-    {
-        return Some(format!("PRAGMA {name}"));
-    }
     match word(0) {
         first @ ("BEGIN" | "START" | "COMMIT" | "END" | "ROLLBACK" | "ABORT" | "SAVEPOINT"
         | "RELEASE") => return Some(first.to_owned()),
         // The SQL they run is a string the guard cannot read, and prepared
         // statements outlive the rollback.
-        "PREPARE" if dialect == Dialect::Postgres && word(1) == "TRANSACTION" => {
-            return Some("PREPARE TRANSACTION".into());
-        }
-        first @ ("PREPARE" | "EXECUTE" | "DEALLOCATE") if postgres_or_mysql => {
-            return Some(first.to_owned());
-        }
+        first @ ("PREPARE" | "EXECUTE" | "DEALLOCATE") => match dialect {
+            Dialect::Postgres if first == "PREPARE" && word(1) == "TRANSACTION" => {
+                return Some("PREPARE TRANSACTION".into());
+            }
+            Dialect::Postgres | Dialect::MySql => return Some(first.to_owned()),
+            Dialect::Sqlite => {}
+        },
         "SET" => {
             if let Some(refused) = set_refusal(dialect, statement, &tokens, &words) {
                 return Some(refused);
             }
         }
-        "RESET" if dialect == Dialect::MySql => {
-            return Some(match word(1) {
-                "" => "RESET".to_owned(),
-                second => format!("RESET {second}"),
-            });
-        }
-        "RESET" if word(1) == "ALL" => return Some("RESET ALL".into()),
-        "RESET" => {
-            if let Some(name) = guarded_name() {
-                return Some(format!("RESET {name}"));
+        "RESET" => match dialect {
+            Dialect::MySql => {
+                return Some(match word(1) {
+                    "" => "RESET".to_owned(),
+                    second => format!("RESET {second}"),
+                });
             }
-        }
+            Dialect::Postgres | Dialect::Sqlite => {
+                if word(1) == "ALL" {
+                    return Some("RESET ALL".into());
+                }
+                if let Some(name) = guarded_name() {
+                    return Some(format!("RESET {name}"));
+                }
+            }
+        },
         "DISCARD" if word(1) == "ALL" => return Some("DISCARD ALL".into()),
-        "COPY" if dialect == Dialect::Postgres => return Some("COPY".into()),
+        "COPY" => match dialect {
+            Dialect::Postgres => return Some("COPY".into()),
+            Dialect::MySql | Dialect::Sqlite => {}
+        },
         _ => {}
     }
-    // set_config() changes the same settings as SET, from any statement.
-    if dialect == Dialect::Postgres && words.iter().any(|word| word == "SET_CONFIG") {
-        return Some("set_config".into());
-    }
-    if dialect != Dialect::MySql {
-        return None;
+    match dialect {
+        // set_config() changes the same settings as SET, from any statement.
+        Dialect::Postgres => {
+            return words
+                .iter()
+                .any(|word| word == "SET_CONFIG")
+                .then(|| "set_config".to_owned());
+        }
+        Dialect::Sqlite => return None,
+        // The rest is MySQL's.
+        Dialect::MySql => {}
     }
     // The session's default database is not reset by the cleanup; a server
     // that prepares USE would carry it into browsing and the next run.
@@ -809,12 +909,16 @@ fn set_refusal(
     let mut code = tokens
         .iter()
         .filter(|token| !matches!(token.kind, TokenKind::Whitespace | TokenKind::Comment));
-    if dialect == Dialect::MySql
-        && let Some(second) = code.nth(1)
-        && matches!(second.kind, TokenKind::Keyword | TokenKind::Identifier)
-        && statement[second.range.clone()].eq_ignore_ascii_case("STATEMENT")
-    {
-        return Some("SET STATEMENT".into());
+    match dialect {
+        Dialect::MySql => {
+            if let Some(second) = code.nth(1)
+                && matches!(second.kind, TokenKind::Keyword | TokenKind::Identifier)
+                && statement[second.range.clone()].eq_ignore_ascii_case("STATEMENT")
+            {
+                return Some("SET STATEMENT".into());
+            }
+        }
+        Dialect::Postgres | Dialect::Sqlite => {}
     }
     let has = |wanted: &str| words.iter().skip(1).any(|word| word == wanted);
     if has("CHARACTERISTICS") {
@@ -845,16 +949,15 @@ fn set_refusal(
     if let Some(name) = words.iter().skip(1).find(|name| is_guarded_setting(name)) {
         return Some(format!("SET {name}"));
     }
-    // Server state that is neither rolled back nor reset by the cleanup.
-    if dialect == Dialect::MySql
-        && let Some(scope) = words
+    match dialect {
+        // Server state that is neither rolled back nor reset by the cleanup.
+        Dialect::MySql => words
             .iter()
             .skip(1)
             .find(|word| matches!(word.as_str(), "GLOBAL" | "PERSIST" | "PERSIST_ONLY"))
-    {
-        return Some(format!("SET {scope}"));
+            .map(|scope| format!("SET {scope}")),
+        Dialect::Postgres | Dialect::Sqlite => None,
     }
-    None
 }
 
 /// The words of each top-level assignment of a `SET` (split on commas
```

- [ ] **Step 4: Change `sql/format.rs`**

```diff
diff --git i/crates/tabletist-db/src/sql/format.rs w/crates/tabletist-db/src/sql/format.rs
index ede363e..1f0be25 100644
--- i/crates/tabletist-db/src/sql/format.rs
+++ w/crates/tabletist-db/src/sql/format.rs
@@ -658,7 +658,10 @@ impl<'a> Layout<'a> {
     /// though it can be a name: not on MySQL, where it may be an alias and
     /// an alias's case can matter.
     fn structure(&self, index: usize) -> bool {
-        self.dialect != Dialect::MySql && self.word(index, STRUCTURE)
+        match self.dialect {
+            Dialect::MySql => false,
+            Dialect::Postgres | Dialect::Sqlite => self.word(index, STRUCTURE),
+        }
     }
 
     /// A comment inside a query.
@@ -735,12 +738,16 @@ impl<'a> Layout<'a> {
             let item = self.items[index];
             item.kind == TokenKind::Operator && item.text == "-"
         };
-        self.dialect == Dialect::MySql
-            && index >= 2
-            && self.items[index].space.is_empty()
-            && minus(index - 1)
-            && self.items[index - 1].space.is_empty()
-            && minus(index - 2)
+        match self.dialect {
+            Dialect::MySql => {
+                index >= 2
+                    && self.items[index].space.is_empty()
+                    && minus(index - 1)
+                    && self.items[index - 1].space.is_empty()
+                    && minus(index - 2)
+            }
+            Dialect::Postgres | Dialect::Sqlite => false,
+        }
     }
 
     /// Whether the item at `index` is a name, whatever it spells: it
```

- [ ] **Step 5: See the tests pass**

Run: `~/.cargo/bin/cargo test --locked --test engines`
Expected: `test result: ok. 2 passed`.

Run: `~/.cargo/bin/cargo test --locked -p tabletist-db --lib sql::`
Expected: `test result: ok. 111 passed`. The filter is a substring, so these are the tests of `sql` (the tokenizer, the guard, the formatter) and of `mysql` with them. None of them changed: they are what says the rewrite reads, refuses and formats as before.

- [ ] **Step 6: Run the four checks**

```bash
~/.cargo/bin/cargo fmt --all --check
~/.cargo/bin/cargo clippy --locked --workspace --all-targets -- -D warnings
~/.cargo/bin/cargo test --locked --workspace --all-targets
RUSTDOCFLAGS='-D warnings' ~/.cargo/bin/cargo doc --locked --workspace --no-deps
```

Expected: no output from `fmt`, `Finished` from `clippy` and `doc`, and every `test result:` line `ok` with `0 failed`.

- [ ] **Step 7: Commit**

```bash
git add crates/tabletist-db/src/sql.rs crates/tabletist-db/src/sql/format.rs tests/engines.rs
git commit -m "Have the code that reads SQL answer for every dialect"
```

---

### Task 3: The connection spec

**Files:**
- Modify: `crates/tabletist-db/src/spec.rs`, `crates/tabletist-db/src/lib.rs`, `tests/engines.rs`

- [ ] **Step 1: Take the two files off the list**

In `tests/engines.rs`, delete these two lines from `PENDING`:

```rust
    "crates/tabletist-db/src/lib.rs",
    "crates/tabletist-db/src/spec.rs",
```

- [ ] **Step 2: See the test fail**

Run: `~/.cargo/bin/cargo test --locked --test engines`
Expected: FAIL with 2 lines: `crates/tabletist-db/src/lib.rs: ... spec.driver != Driver::Sqlite` and `crates/tabletist-db/src/spec.rs: ... self.driver == Driver::Sqlite`.

- [ ] **Step 3: Change `spec.rs`**

The first hunk is the `_` arm the test cannot see (decision 4).

```diff
diff --git i/crates/tabletist-db/src/spec.rs w/crates/tabletist-db/src/spec.rs
index 9996db9..18493c8 100644
--- i/crates/tabletist-db/src/spec.rs
+++ w/crates/tabletist-db/src/spec.rs
@@ -257,20 +257,23 @@ impl ConnectSpec {
     pub fn effective_tls(&self) -> TlsMode {
         match (self.driver, self.tls, &self.ca_file) {
             (Driver::Postgres, TlsMode::Require, Some(_)) => TlsMode::VerifyCa,
-            (_, tls, _) => tls,
+            (Driver::Postgres | Driver::MySql | Driver::Sqlite, tls, _) => tls,
         }
     }
 
     /// A one-line description without secrets: `user@host:port/db`, or the
     /// file name for SQLite.
     pub fn summary(&self) -> String {
-        if self.driver == Driver::Sqlite {
-            return self
-                .sqlite_path
-                .as_ref()
-                .and_then(|path| path.file_name())
-                .map(|name| name.to_string_lossy().into_owned())
-                .unwrap_or_default();
+        match self.driver {
+            Driver::Sqlite => {
+                return self
+                    .sqlite_path
+                    .as_ref()
+                    .and_then(|path| path.file_name())
+                    .map(|name| name.to_string_lossy().into_owned())
+                    .unwrap_or_default();
+            }
+            Driver::Postgres | Driver::MySql => {}
         }
         let mut text = String::new();
         if !self.user.is_empty() {
```

- [ ] **Step 4: Change `lib.rs`**

```diff
diff --git i/crates/tabletist-db/src/lib.rs w/crates/tabletist-db/src/lib.rs
index 9ebca14..8b26e11 100644
--- i/crates/tabletist-db/src/lib.rs
+++ w/crates/tabletist-db/src/lib.rs
@@ -105,7 +105,11 @@ impl Connection {
         host_keys: &HostKeys,
         access: Access,
     ) -> Result<Self> {
-        let ssh = spec.ssh.as_ref().filter(|_| spec.driver != Driver::Sqlite);
+        // A file is opened here, whatever tunnel its spec still names.
+        let ssh = match spec.driver {
+            Driver::Sqlite => None,
+            Driver::Postgres | Driver::MySql => spec.ssh.as_ref(),
+        };
         let Some(ssh) = ssh else {
             return Ok(Self {
                 inner: Inner::connect(spec, secrets, None, access).await?,
```

- [ ] **Step 5: See the tests pass**

Run: `~/.cargo/bin/cargo test --locked --test engines`
Expected: `test result: ok. 2 passed`.

Run: `~/.cargo/bin/cargo test --locked -p tabletist-db --lib spec::`
Expected: `test result: ok. 17 passed`.

- [ ] **Step 6: Run the four checks**

```bash
~/.cargo/bin/cargo fmt --all --check
~/.cargo/bin/cargo clippy --locked --workspace --all-targets -- -D warnings
~/.cargo/bin/cargo test --locked --workspace --all-targets
RUSTDOCFLAGS='-D warnings' ~/.cargo/bin/cargo doc --locked --workspace --no-deps
```

Expected: no output from `fmt`, `Finished` from `clippy` and `doc`, and every `test result:` line `ok` with `0 failed`.

- [ ] **Step 7: Commit**

```bash
git add crates/tabletist-db/src/spec.rs crates/tabletist-db/src/lib.rs tests/engines.rs
git commit -m "Have a connection's spec answer for every driver"
```

---

### Task 4: The app's state

**Files:**
- Modify: `src/app.rs`, `src/model.rs`, `tests/engines.rs`

- [ ] **Step 1: Take the two files off the list**

In `tests/engines.rs`, delete these two lines from `PENDING`:

```rust
    "src/app.rs",
    "src/model.rs",
```

- [ ] **Step 2: See the test fail**

Run: `~/.cargo/bin/cargo test --locked --test engines`
Expected: FAIL with 5 lines, 2 that start with `src/app.rs:` and 3 with `src/model.rs:`.

- [ ] **Step 3: Change `app.rs`**

```diff
diff --git i/src/app.rs w/src/app.rs
index 9651bd6..efe77d6 100644
--- i/src/app.rs
+++ w/src/app.rs
@@ -1144,12 +1144,15 @@ impl App {
                     // A port left at the old driver's default follows the driver.
                     let old_default = form.driver.default_port().to_string();
                     form.driver = driver;
-                    if driver != Driver::Sqlite {
-                        if form.port.trim().is_empty() || form.port.trim() == old_default {
-                            form.port = driver.default_port().to_string();
-                        }
-                        if form.password_mode == PasswordMode::None {
-                            form.password_mode = PasswordMode::Keyring;
+                    match driver {
+                        Driver::Sqlite => {}
+                        Driver::Postgres | Driver::MySql => {
+                            if form.port.trim().is_empty() || form.port.trim() == old_default {
+                                form.port = driver.default_port().to_string();
+                            }
+                            if form.password_mode == PasswordMode::None {
+                                form.password_mode = PasswordMode::Keyring;
+                            }
                         }
                     }
                 }
@@ -2538,16 +2541,20 @@ impl App {
     /// After a (re)connect: load the tree, restart everything that waited on
     /// the old session, and open anything queued for this connection.
     pub fn after_connect(&mut self, tab: ConnTabId) {
-        if let Some(workspace) = self.workspace(tab)
-            && workspace.driver == Driver::Postgres
-        {
-            let session = workspace.session;
-            let request = RequestId(self.next_id());
-            if let Some(workspace) = self.workspace_mut(tab) {
-                workspace.databases.start(request);
+        if let Some(workspace) = self.workspace(tab) {
+            match workspace.driver {
+                // The one driver with other databases to switch to.
+                Driver::Postgres => {
+                    let session = workspace.session;
+                    let request = RequestId(self.next_id());
+                    if let Some(workspace) = self.workspace_mut(tab) {
+                        workspace.databases.start(request);
+                    }
+                    self.backend
+                        .send(Command::ListDatabases { session, request });
+                }
+                Driver::MySql | Driver::Sqlite => {}
             }
-            self.backend
-                .send(Command::ListDatabases { session, request });
         }
         // The footer of a SQL editor names the server; ask again when the
         // session is new (see `Workspace::forget_session_requests`).
```

- [ ] **Step 4: Change `model.rs`**

The first hunk is `password_can_be_intercepted` (decision 3).

```diff
diff --git i/src/model.rs w/src/model.rs
index ddfabc1..bcc0f71 100644
--- i/src/model.rs
+++ w/src/model.rs
@@ -726,17 +726,19 @@ impl ConnectionForm {
     /// off or does not check the server's certificate (PostgreSQL's
     /// `require` with a CA file does, like libpq).
     pub fn password_can_be_intercepted(&self) -> bool {
-        let checked_by_ca = self.driver == Driver::Postgres
-            && self.tls == TlsMode::Require
-            && !self.ca_file.trim().is_empty();
-        self.driver != Driver::Sqlite
-            && !self.ssh
-            && matches!(
-                self.tls,
-                TlsMode::Disable | TlsMode::Prefer | TlsMode::Require
-            )
-            && !checked_by_ca
-            && !is_local_host(&self.host)
+        let unchecked = match self.driver {
+            Driver::Sqlite => false,
+            Driver::Postgres => match self.tls {
+                TlsMode::Disable | TlsMode::Prefer => true,
+                TlsMode::Require => self.ca_file.trim().is_empty(),
+                TlsMode::VerifyCa | TlsMode::VerifyFull => false,
+            },
+            Driver::MySql => match self.tls {
+                TlsMode::Disable | TlsMode::Prefer | TlsMode::Require => true,
+                TlsMode::VerifyCa | TlsMode::VerifyFull => false,
+            },
+        };
+        unchecked && !self.ssh && !is_local_host(&self.host)
     }
 
     /// The environment the connection is saved with: the chosen one, else
@@ -974,17 +976,18 @@ impl ConnectionForm {
             return Err("Give the connection a name.".into());
         }
         let spec = self.to_spec()?;
-        let password = if spec.driver == Driver::Sqlite {
-            PasswordMode::None
-        } else if self.password_mode == PasswordMode::Keyring
-            && self.password.is_empty()
-            && !self.has_saved_password
-        {
+        let password = match spec.driver {
+            Driver::Sqlite => PasswordMode::None,
             // Nothing typed and nothing saved: a server that needs no
             // password (trust or peer authentication).
-            PasswordMode::None
-        } else {
-            self.password_mode
+            Driver::Postgres | Driver::MySql
+                if self.password_mode == PasswordMode::Keyring
+                    && self.password.is_empty()
+                    && !self.has_saved_password =>
+            {
+                PasswordMode::None
+            }
+            Driver::Postgres | Driver::MySql => self.password_mode,
         };
         let ssh_secret = match (&spec.ssh, self.ssh_auth) {
             (None, _) | (Some(_), SshAuthKind::Agent) => PasswordMode::None,
```

- [ ] **Step 5: See the tests pass**

Run: `~/.cargo/bin/cargo test --locked --test engines`
Expected: `test result: ok. 2 passed`.

Run: `~/.cargo/bin/cargo test --locked -p tabletist --lib model::`
Expected: `test result: ok. 63 passed`.

- [ ] **Step 6: Run the four checks**

```bash
~/.cargo/bin/cargo fmt --all --check
~/.cargo/bin/cargo clippy --locked --workspace --all-targets -- -D warnings
~/.cargo/bin/cargo test --locked --workspace --all-targets
RUSTDOCFLAGS='-D warnings' ~/.cargo/bin/cargo doc --locked --workspace --no-deps
```

Expected: no output from `fmt`, `Finished` from `clippy` and `doc`, and every `test result:` line `ok` with `0 failed`.

- [ ] **Step 7: Commit**

```bash
git add src/app.rs src/model.rs tests/engines.rs
git commit -m "Have the app's state answer for every driver"
```

---

### Task 5: The views

**Files:**
- Modify: `src/ui/connect_dialog/sheet.rs`, `src/ui/connect_dialog/terminal.rs`, `src/ui/picker.rs`, `src/ui/workspace.rs`, `tests/engines.rs`

- [ ] **Step 1: Take the four files off the list**

In `tests/engines.rs`, delete these four lines from `PENDING`, which leaves it empty (`const PENDING: &[&str] = &[];`, as `cargo fmt` writes it):

```rust
    "src/ui/connect_dialog/sheet.rs",
    "src/ui/connect_dialog/terminal.rs",
    "src/ui/picker.rs",
    "src/ui/workspace.rs",
```

- [ ] **Step 2: See the test fail**

Run: `~/.cargo/bin/cargo test --locked --test engines`
Expected: FAIL with 8 lines: 1 for `src/ui/connect_dialog/sheet.rs`, 1 for `src/ui/connect_dialog/terminal.rs`, 2 for `src/ui/picker.rs` and 4 for `src/ui/workspace.rs`.

- [ ] **Step 3: Change the dialog's two looks**

```diff
diff --git i/src/ui/connect_dialog/sheet.rs w/src/ui/connect_dialog/sheet.rs
index 94df3e0..e253891 100644
--- i/src/ui/connect_dialog/sheet.rs
+++ w/src/ui/connect_dialog/sheet.rs
@@ -320,18 +320,21 @@ pub(super) fn body(
             } else {
                 identity(ui, form, skin, focus_name, actions);
                 ui.add_space(18.0);
-                if form.driver == Driver::Sqlite {
-                    fieldset(ui, &skin.say("Database"), skin, |ui| {
-                        file_field(ui, form, skin, actions);
-                    });
-                } else {
-                    fieldset(ui, &skin.say("Server"), skin, |ui| {
-                        server(ui, form, skin);
-                    });
-                    ui.add_space(18.0);
-                    fieldset(ui, &skin.say("Security"), skin, |ui| {
-                        security(ui, form, skin, actions);
-                    });
+                match form.driver {
+                    Driver::Sqlite => {
+                        fieldset(ui, &skin.say("Database"), skin, |ui| {
+                            file_field(ui, form, skin, actions);
+                        });
+                    }
+                    Driver::Postgres | Driver::MySql => {
+                        fieldset(ui, &skin.say("Server"), skin, |ui| {
+                            server(ui, form, skin);
+                        });
+                        ui.add_space(18.0);
+                        fieldset(ui, &skin.say("Security"), skin, |ui| {
+                            security(ui, form, skin, actions);
+                        });
+                    }
                 }
                 ui.add_space(18.0);
                 safety(ui, form, skin);
```

```diff
diff --git i/src/ui/connect_dialog/terminal.rs w/src/ui/connect_dialog/terminal.rs
index c4a4c18..2aa15b7 100644
--- i/src/ui/connect_dialog/terminal.rs
+++ w/src/ui/connect_dialog/terminal.rs
@@ -329,14 +329,17 @@ pub(super) fn terminal_body(
             terminal_row_of(ui, &heading, Group::Buttons.height(), skin, |ui, _| {
                 environment_choice(ui, form, &heading, skin);
             });
-            if form.driver == Driver::Sqlite {
-                terminal_section(ui, &skin.say("Database"), skin);
-                file_field(ui, form, skin, actions);
-            } else {
-                terminal_section(ui, &skin.say("Server"), skin);
-                terminal_server(ui, form, skin);
-                terminal_section(ui, &skin.say("Security"), skin);
-                terminal_security(ui, form, skin, actions);
+            match form.driver {
+                Driver::Sqlite => {
+                    terminal_section(ui, &skin.say("Database"), skin);
+                    file_field(ui, form, skin, actions);
+                }
+                Driver::Postgres | Driver::MySql => {
+                    terminal_section(ui, &skin.say("Server"), skin);
+                    terminal_server(ui, form, skin);
+                    terminal_section(ui, &skin.say("Security"), skin);
+                    terminal_security(ui, form, skin, actions);
+                }
             }
             terminal_section(ui, &skin.say("Safety"), skin);
             // A line of text, as tall as the text.
```

- [ ] **Step 4: Change `picker.rs`**

```diff
diff --git i/src/ui/picker.rs w/src/ui/picker.rs
index e4b7c83..d134cae 100644
--- i/src/ui/picker.rs
+++ w/src/ui/picker.rs
@@ -100,8 +100,14 @@ pub fn groups<'a>(connections: &[&'a SavedConnection]) -> Vec<Group<'a>> {
 /// Whether a connection stays on this machine: a file, or a local host
 /// without a tunnel.
 fn is_local(connection: &SavedConnection) -> bool {
-    connection.spec.driver == tabletist_db::Driver::Sqlite
-        || (connection.spec.ssh.is_none() && crate::model::is_local_host(&connection.spec.host))
+    use tabletist_db::Driver;
+    let spec = &connection.spec;
+    match spec.driver {
+        Driver::Sqlite => true,
+        Driver::Postgres | Driver::MySql => {
+            spec.ssh.is_none() && crate::model::is_local_host(&spec.host)
+        }
+    }
 }
 
 /// The connections the picker lists, in its order: grouped and filtered by
@@ -612,14 +618,15 @@ impl Opening {
 /// Where a connection points, as the rows say it: host and port, then the
 /// database (a file's name for SQLite).
 fn target(connection: &SavedConnection) -> (String, String) {
+    use tabletist_db::Driver;
     let spec = &connection.spec;
-    if spec.driver == tabletist_db::Driver::Sqlite {
-        return (spec.summary(), String::new());
+    match spec.driver {
+        Driver::Sqlite => (spec.summary(), String::new()),
+        Driver::Postgres | Driver::MySql => (
+            format!("{}:{}", spec.host, spec.port),
+            format!("/{}", spec.database),
+        ),
     }
-    (
-        format!("{}:{}", spec.host, spec.port),
-        format!("/{}", spec.database),
-    )
 }
 
 /// The row's own response, answering a click by selecting and a double
```

- [ ] **Step 5: Change `workspace.rs`**

`bar_info` and `card_rows` both asked whether the connection is remote, each with its own `!sqlite && ...`. They now share `is_remote`. In `connecting`, the words still go through `say` as string literals (`say(reach)` where `reach` is `"Open"` or `"Connect to"`).

```diff
diff --git i/src/ui/workspace.rs w/src/ui/workspace.rs
index 9e9d143..3bfb924 100644
--- i/src/ui/workspace.rs
+++ w/src/ui/workspace.rs
@@ -178,27 +178,26 @@ fn opening(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId) {
 /// connect with nothing to lose, the button that gives up: in the middle
 /// of the tab, or in the terminal look from its top left.
 fn connecting(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId) {
+    use tabletist_db::Driver;
     let (locale, palette, look) = (app.locale, app.palette, app.look);
     let Some(workspace) = app.workspace(tab) else {
         return;
     };
     let say = |text: &'static str| look.label(&gettext(locale, text));
     let spec = &workspace.spec;
-    let sqlite = spec.driver == tabletist_db::Driver::Sqlite;
-    let target = if sqlite {
-        spec.summary()
-    } else {
-        format!("{}:{}", spec.host, spec.port)
-    };
-    let target = match spec.ssh.as_ref().filter(|_| !sqlite) {
-        Some(ssh) => format!("{target} {} {}", say("via"), ssh.host),
-        None => target,
+    let (reach, target) = match spec.driver {
+        Driver::Sqlite => ("Open", spec.summary()),
+        Driver::Postgres | Driver::MySql => {
+            let target = format!("{}:{}", spec.host, spec.port);
+            let target = match &spec.ssh {
+                Some(ssh) => format!("{target} {} {}", say("via"), ssh.host),
+                None => target,
+            };
+            ("Connect to", target)
+        }
     };
     let connected = matches!(workspace.status, SessionStatus::Connected);
-    let (reach, load) = (
-        say(if sqlite { "Open" } else { "Connect to" }),
-        say("Load schema"),
-    );
+    let (reach, load) = (say(reach), say("Load schema"));
     let list = [
         states::Step {
             state: if connected {
@@ -284,14 +283,16 @@ fn failure_title(
             Icon::Lock,
             format!("{} {}", say("Password rejected for"), spec.user),
         ),
-        Error::Connect(_) | Error::Timeout if spec.driver == Driver::Sqlite => (
-            Icon::CircleAlert,
-            format!("{} {}", say("Can't open"), spec.summary()),
-        ),
-        Error::Connect(_) | Error::Timeout => (
-            Icon::WifiOff,
-            format!("{} {}:{}", say("Can't reach"), spec.host, spec.port),
-        ),
+        Error::Connect(_) | Error::Timeout => match spec.driver {
+            Driver::Sqlite => (
+                Icon::CircleAlert,
+                format!("{} {}", say("Can't open"), spec.summary()),
+            ),
+            Driver::Postgres | Driver::MySql => (
+                Icon::WifiOff,
+                format!("{} {}:{}", say("Can't reach"), spec.host, spec.port),
+            ),
+        },
         Error::Tls(_) => (Icon::ShieldAlert, say("TLS or certificate problem")),
         Error::Ssh {
             stage: SshStage::HostKeyUnknown { .. } | SshStage::HostKeyMismatch { .. },
@@ -477,14 +478,22 @@ struct Chip {
     card: Vec<(String, String)>,
 }
 
+/// Whether a connection's traffic leaves this machine: a file's never does.
+fn is_remote(workspace: &crate::model::Workspace) -> bool {
+    use tabletist_db::Driver;
+    match workspace.driver {
+        Driver::Sqlite => false,
+        Driver::Postgres | Driver::MySql => !crate::model::is_local_host(&workspace.spec.host),
+    }
+}
+
 fn bar_info(app: &App, tab: ConnTabId) -> Option<BarInfo> {
+    use tabletist_db::Driver;
     let workspace = app.workspace(tab)?;
     let spec = &workspace.spec;
-    let sqlite = workspace.driver == tabletist_db::Driver::Sqlite;
     // Local traffic never crosses a network, so its TLS says nothing; a
     // remote connection always says how far it can be trusted.
-    let remote = !sqlite && !crate::model::is_local_host(&spec.host);
-    let tls = remote.then(|| {
+    let tls = is_remote(workspace).then(|| {
         let encrypted =
             matches!(workspace.status, SessionStatus::Connected).then_some(workspace.encrypted);
         let (text, warn) = tls_status(spec.effective_tls(), encrypted);
@@ -493,10 +502,9 @@ fn bar_info(app: &App, tab: ConnTabId) -> Option<BarInfo> {
     Some(BarInfo {
         env: workspace.environment,
         read_only: workspace.access == tabletist_db::Access::ReadOnly,
-        database: if sqlite {
-            String::new()
-        } else {
-            spec.database.clone()
+        database: match workspace.driver {
+            Driver::Sqlite => String::new(),
+            Driver::Postgres | Driver::MySql => spec.database.clone(),
         },
         databases: workspace.databases.value.clone().unwrap_or_default(),
         tls,
@@ -544,24 +552,27 @@ fn card_rows(
     locale: crate::i18n::Locale,
     now: u64,
 ) -> Vec<(String, String)> {
+    use tabletist_db::Driver;
     let say = |text: &'static str| look.label(&gettext(locale, text));
     let spec = &workspace.spec;
-    let sqlite = workspace.driver == tabletist_db::Driver::Sqlite;
     let mut rows = Vec::new();
-    if sqlite {
-        let path = spec
-            .sqlite_path
-            .as_ref()
-            .map(|path| path.display().to_string())
-            .unwrap_or_default();
-        rows.push((say("File"), path));
-    } else {
-        rows.push((say("Host"), format!("{}:{}", spec.host, spec.port)));
-        if !spec.database.is_empty() {
-            rows.push((say("Database"), display_safe(&spec.database).into_owned()));
+    match workspace.driver {
+        Driver::Sqlite => {
+            let path = spec
+                .sqlite_path
+                .as_ref()
+                .map(|path| path.display().to_string())
+                .unwrap_or_default();
+            rows.push((say("File"), path));
         }
-        if !spec.user.is_empty() {
-            rows.push((say("User"), spec.user.clone()));
+        Driver::Postgres | Driver::MySql => {
+            rows.push((say("Host"), format!("{}:{}", spec.host, spec.port)));
+            if !spec.database.is_empty() {
+                rows.push((say("Database"), display_safe(&spec.database).into_owned()));
+            }
+            if !spec.user.is_empty() {
+                rows.push((say("User"), spec.user.clone()));
+            }
         }
     }
     let server = workspace
@@ -571,8 +582,7 @@ fn card_rows(
         .unwrap_or_else(|| workspace.driver.label().to_owned());
     rows.push((say("Server"), server));
     let connected = matches!(workspace.status, SessionStatus::Connected);
-    let remote = !sqlite && !crate::model::is_local_host(&spec.host);
-    let tls = if remote {
+    let tls = if is_remote(workspace) {
         let encrypted = connected.then_some(workspace.encrypted);
         say(tls_status(spec.effective_tls(), encrypted).0)
     } else {
```

- [ ] **Step 6: See the tests pass**

Run: `~/.cargo/bin/cargo fmt --all` (it puts the empty `PENDING` on one line), then `~/.cargo/bin/cargo test --locked --test engines`
Expected: `test result: ok. 2 passed`.

- [ ] **Step 7: Run the four checks**

```bash
~/.cargo/bin/cargo fmt --all --check
~/.cargo/bin/cargo clippy --locked --workspace --all-targets -- -D warnings
~/.cargo/bin/cargo test --locked --workspace --all-targets
RUSTDOCFLAGS='-D warnings' ~/.cargo/bin/cargo doc --locked --workspace --no-deps
```

Expected: no output from `fmt`, `Finished` from `clippy` and `doc`, and every `test result:` line `ok` with `0 failed`.

The headless UI tests in `src/ui/` draw the connect dialog in both looks, the picker's rows, the connecting steps, a failed connect's title and the connection bar: they are what says the views draw as before.

- [ ] **Step 8: Commit**

```bash
git add src/ui/connect_dialog/sheet.rs src/ui/connect_dialog/terminal.rs src/ui/picker.rs src/ui/workspace.rs tests/engines.rs
git commit -m "Have the views answer for every driver"
```

---

### Task 6: The rule, written down

**Files:**
- Modify: `tests/engines.rs`, `AGENTS.md`

- [ ] **Step 1: Remove the empty list**

In `tests/engines.rs`, delete the constant with its comment:

```rust
/// Files whose comparisons are still to become matches. Each task of the
/// plan takes its own out; the last one removes the list.
const PENDING: &[&str] = &[];
```

and, in `no_code_compares_a_driver_or_a_dialect`, the line that used it:

```rust
        .filter(|(path, _)| !PENDING.contains(&path.as_str()))
```

- [ ] **Step 2: Say it in `AGENTS.md`**

```diff
diff --git i/AGENTS.md w/AGENTS.md
index 8f8f6c7..bed49c4 100644
--- i/AGENTS.md
+++ w/AGENTS.md
@@ -14,6 +14,11 @@ MySQL, SQLite) on egui/eframe and fastframe. The design lives in
 - Database, network, and disk work runs on the backend runtime
   (`src/backend.rs`), never on the UI thread.
 - `crates/tabletist-db` has no UI dependencies.
+- Code that depends on the database engine matches on `Driver` or `Dialect`
+  and names every variant: no `==`, `!=`, `matches!` or `_` arm. A new engine
+  then fails to compile wherever nobody has decided for it.
+  `tests/engines.rs` finds `==`, `!=` and `matches!`. Nothing finds a `_`
+  arm, or a comparison written with `Self::` in the enums' own methods.
 - The workspace forbids `unsafe`. AppKit calls that cannot be made without it
   go in `crates/tabletist-appkit`, and SQLite calls in
   `crates/tabletist-sqlite-ffi`, behind a safe API, each with a SAFETY note;
```

- [ ] **Step 3: Run the four checks**

```bash
~/.cargo/bin/cargo fmt --all --check
~/.cargo/bin/cargo clippy --locked --workspace --all-targets -- -D warnings
~/.cargo/bin/cargo test --locked --workspace --all-targets
RUSTDOCFLAGS='-D warnings' ~/.cargo/bin/cargo doc --locked --workspace --no-deps
```

Expected: no output from `fmt`, `Finished` from `clippy` and `doc`, and every `test result:` line `ok` with `0 failed`.

- [ ] **Step 4: Commit**

```bash
git add tests/engines.rs AGENTS.md
git commit -m "Say that engine code matches on every engine"
```

---

## By hand

Nothing to look at: no pixel and no word changes. What only CI can say is whether the PostgreSQL and MySQL suites still pass (`ci.yml`, the `postgres integration` and `mysql integration` jobs), since they did not run where the draft was checked.

## Self-review

- **Coverage:** the 42 comparisons the scan finds on `main` are 25 in `sql.rs`, 2 in `sql/format.rs`, 1 each in `lib.rs` and `spec.rs`, 2 in `app.rs`, 3 in `model.rs`, 1 each in the dialog's two looks, 2 in `picker.rs` and 4 in `workspace.rs`. Tasks 2 to 5 take 27, 2, 5 and 8 of them. The `_` arm is in Task 3.
- **Each commit passes alone:** the tasks change disjoint files, and nothing added in one (the ten questions in `sql.rs`, `is_remote` in `workspace.rs`) is used from another file. The list in the test shrinks with each.
- **No placeholders:** every step has its diff or its exact lines.
- **Left for later:** one "file or server" answer on `Driver` (decision 1).

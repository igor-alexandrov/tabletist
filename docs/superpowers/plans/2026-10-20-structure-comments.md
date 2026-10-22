# Column and table comments Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Show a table's comment and its columns' comments in the Structure view, a column's comment as its grid header's tooltip, and the table's comment in the MCP schema tool (igor-alexandrov/tabletist#193).

**Architecture:** `Structure` gains `comment`, read by each engine's `describe` (PostgreSQL `obj_description`, MySQL `table_comment`, SQLite none). The Structure view draws the table's comment over everything and a Comment column whose presence a `Driver` match decides; a cell shows the comment's first line, cut, with the whole on hover and as its accessible label. `grid::Column` gains `comment`, filled by the data view, shown on hover and as the header's AccessKit description. The MCP schema tool says the table's comment unless the table has a masked column or reads one.

**Tech Stack:** Rust 2024, egui/eframe (crmne fork), tokio-postgres, mysql_async, rusqlite, rmcp. Headless UI tests through `src/testing.rs` (AccessKit tree + events).

**Spec:** `docs/superpowers/specs/2026-10-20-structure-comments-design.md`.

**House rules that bite here (from `AGENTS.md`):**
- Engine-dependent code matches on `Driver`/`Dialect` naming every variant: no `==`, `!=`, `matches!`, `_` arm.
- Views draw text only through `TextRole`s. `on_hover_text` with a string is the established tooltip form.
- Never use em dashes, in code comments, docs or commit messages.
- One topic per commit, each passing the checks. Commits are signed through 1Password: if `git commit` fails with `1Password: agent returned an error`, stop, leave the work staged, and ask the user to unlock it. Never skip signing.
- Comments in this codebase are full sentences in plain words, saying why. Match the density around them.

**The checks** (run at the end of every task, all must pass). Run `cargo fmt --all` first: the code below is not always in rustfmt's form (long lines, and the order of `mod` lines in `src/ui/mod.rs`).

```bash
cargo fmt --all --check
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo test --locked --workspace --all-targets
RUSTDOCFLAGS='-D warnings' cargo doc --locked --workspace --no-deps
```

Database integration tests need the servers (`docker compose up -d --build --wait postgres mysql`; they may already be up on ports 55432 and 53306) and:

```bash
TABLETIST_TEST_PG_URL=postgres://tabletist:tabletist@localhost:55432/tabletist TABLETIST_TEST_MYSQL_URL=mysql://tabletist:tabletist@localhost:53306/tabletist cargo test -p tabletist-db --test postgres --test mysql
```

Without the variables those tests print "skipped" and pass: that is no proof. Run them with the variables.

---

## File map

| File | Change |
| --- | --- |
| `crates/tabletist-db/src/catalog.rs` | `Structure.comment` |
| `crates/tabletist-db/src/pg.rs` | read `obj_description` in `describe` |
| `crates/tabletist-db/src/mysql.rs` | read `table_comment` in `describe` |
| `crates/tabletist-db/src/sqlite.rs` | `comment: None` |
| `crates/tabletist-db/fixtures/postgres.sql`, `mysql.sql` | table and view comments |
| `crates/tabletist-db/tests/postgres.rs`, `mysql.rs` | integration tests |
| every `Structure { .. }` literal without `..Default::default()` | `comment: None` (the compiler lists them) |
| `crates/tabletist-mcp/src/tools/schema.rs` | `"comment"` in a table's entry, tool description |
| `crates/tabletist-mcp/tests/support/mod.rs` | `Shop::comment` |
| `crates/tabletist-mcp/tests/tools.rs` | MCP test |
| `src/ui/structure.rs` | table comment, Comment column, `comment_line`, unit tests |
| `src/ui/comment_tests.rs` (new) | headless tests of every comment the UI shows |
| `src/ui/mod.rs` | `#[cfg(test)] mod comment_tests;` |
| `src/ui/grid.rs` | `Column.comment`, header description and tooltip, unit test |
| `src/ui/data_view.rs`, `src/ui/sql_results.rs`, `src/ui/settings/preview.rs` | fill `Column.comment` |
| `src/shots.rs` | Bookshop scene comments |
| `docs/_guide/browsing-data.md` | Structure section |
| `assets/screenshots/macos-structure.webp` | rendered again |

---

### Task 1: Read a table's own comment

**Files:**
- Modify: `crates/tabletist-db/src/catalog.rs:148-157`
- Modify: `crates/tabletist-db/src/pg.rs` (`describe`, from line 572; `Ok(Structure {` at 756)
- Modify: `crates/tabletist-db/src/mysql.rs` (`describe`, from line 249; `Ok(Structure {` at 443)
- Modify: `crates/tabletist-db/src/sqlite.rs:1017-1023`
- Modify: `crates/tabletist-db/fixtures/postgres.sql`, `crates/tabletist-db/fixtures/mysql.sql`
- Test: `crates/tabletist-db/tests/postgres.rs`, `crates/tabletist-db/tests/mysql.rs`

- [ ] **Step 1: Give the fixtures comments**

In `crates/tabletist-db/fixtures/postgres.sql`, after the line `COMMENT ON COLUMN users.email IS 'Login address';` add:

```sql
COMMENT ON TABLE users IS 'People who sign in';
```

and after `CREATE VIEW active_users AS SELECT id, email FROM users WHERE active;` add:

```sql
COMMENT ON VIEW active_users IS 'Users who can sign in';
```

In `crates/tabletist-db/fixtures/mysql.sql`, end the `CREATE TABLE users (...)` statement with a table comment: change the closing

```sql
    alarm TIME
);
```

to

```sql
    alarm TIME
) COMMENT = 'People who sign in';
```

(`orders` keeps none in both, and MySQL keeps no comment on a view.)

- [ ] **Step 2: Write the failing integration tests**

In `crates/tabletist-db/tests/postgres.rs`, after `users_structure_has_types_defaults_comments_key_and_indexes`:

```rust
#[tokio::test]
async fn a_table_and_a_view_say_their_own_comments() {
    let Some(connection) = connect().await else {
        return;
    };
    let comment = |name: &'static str| {
        let connection = &connection;
        async move {
            connection
                .describe(&ObjectRef::new("public", name))
                .await
                .unwrap()
                .comment
        }
    };
    assert_eq!(comment("users").await.as_deref(), Some("People who sign in"));
    assert_eq!(
        comment("active_users").await.as_deref(),
        Some("Users who can sign in")
    );
    assert_eq!(comment("orders").await, None);
}
```

In `crates/tabletist-db/tests/mysql.rs`, after `users_structure_has_full_types_comments_key_and_indexes`:

```rust
#[tokio::test]
async fn a_table_says_its_own_comment_and_a_view_has_none() {
    let Some(connection) = connect().await else {
        return;
    };
    let comment = |name: &'static str| {
        let connection = &connection;
        async move {
            connection
                .describe(&ObjectRef::new("tabletist", name))
                .await
                .unwrap()
                .comment
        }
    };
    assert_eq!(comment("users").await.as_deref(), Some("People who sign in"));
    assert_eq!(comment("orders").await, None);
    // MySQL writes VIEW there for every view: nobody wrote it.
    assert_eq!(comment("active_users").await, None);
}
```

- [ ] **Step 3: Run them to see them fail**

Run: `TABLETIST_TEST_PG_URL=postgres://tabletist:tabletist@localhost:55432/tabletist TABLETIST_TEST_MYSQL_URL=mysql://tabletist:tabletist@localhost:53306/tabletist cargo test -p tabletist-db --test postgres --test mysql comment`
Expected: compile error, `no field 'comment' on type 'Structure'`.

- [ ] **Step 4: Add the field**

In `crates/tabletist-db/src/catalog.rs`, in `pub struct Structure`, after `referenced_by`:

```rust
    /// The comment on the table or view itself, where its author wrote
    /// one. SQLite keeps none.
    pub comment: Option<String>,
```

- [ ] **Step 5: Read it on PostgreSQL**

In `crates/tabletist-db/src/pg.rs` `describe`, just before `Ok(Structure {`:

```rust
        // The comment on the table or view itself. An empty one is none:
        // `COMMENT ON ... IS ''` drops it.
        let comment: Option<String> = match self
            .catalog("SELECT obj_description($1, 'pg_class')", &[&oid])
            .await?
            .first()
        {
            Some(row) => column(row, 0)?,
            None => None,
        };
```

and in the literal, after `referenced_by,`:

```rust
            comment: comment.filter(|comment| !comment.is_empty()),
```

- [ ] **Step 6: Read it on MySQL**

In `crates/tabletist-db/src/mysql.rs` `describe`, just before `Ok(Structure {`:

```rust
        // The comment on the table itself. MySQL writes `VIEW` there for
        // every view, which nobody wrote.
        let comment = self
            .catalog::<String>(
                "SELECT CASE WHEN table_type = 'VIEW' THEN '' \
                        ELSE COALESCE(table_comment, '') END \
                 FROM information_schema.tables \
                 WHERE table_schema = ? AND table_name = ?",
                at,
            )
            .await?
            .into_iter()
            .next()
            .filter(|comment| !comment.is_empty());
```

and in the literal, after `referenced_by,`:

```rust
            comment,
```

- [ ] **Step 7: SQLite keeps none**

In `crates/tabletist-db/src/sqlite.rs` `describe`, in the `Ok(Structure {` literal, after `referenced_by: referencing_keys(connection, &object)?,`:

```rust
                comment: None,
```

- [ ] **Step 8: Name the field in every other literal**

Run: `cargo build --locked --workspace --all-targets 2>&1 | grep -B2 -A6 "missing field \`comment\`"`

For each literal it names (expected: two in `src/ui/mod.rs`, and three in `src/shots.rs`, which only builds with `--features shots`; the others already end in `..Default::default()`), add `comment: None,` after the literal's `referenced_by` line. Do not touch literals that already end in `..Default::default()` or `..Structure::default()`. Repeat until it builds:

Run: `cargo build --locked --workspace --all-targets && cargo build --locked --all-targets --features shots`
Expected: both build.

- [ ] **Step 9: Run the integration tests to see them pass**

Run: `TABLETIST_TEST_PG_URL=postgres://tabletist:tabletist@localhost:55432/tabletist TABLETIST_TEST_MYSQL_URL=mysql://tabletist:tabletist@localhost:53306/tabletist cargo test -p tabletist-db --test postgres --test mysql`
Expected: every test passes, none prints "skipped".

- [ ] **Step 10: Run the checks, then commit**

Run the four checks. Then:

```bash
git add -A crates/tabletist-db src
git commit -m "Read a table's own comment on PostgreSQL and MySQL

Structure gets the comment on the table or view itself:
obj_description on PostgreSQL, table_comment on MySQL, where a view's
VIEW is none. SQLite keeps none.

Part of #193."
```

---

### Task 2: The MCP schema tool says a table's comment

**Files:**
- Modify: `crates/tabletist-mcp/tests/support/mod.rs` (`Inner`, `Shop::new`, a new `Shop::comment`, `ShopSession::describe` at line 238)
- Modify: `crates/tabletist-mcp/src/tools/schema.rs` (description at lines 30-39, `table()` at 350-366)
- Test: `crates/tabletist-mcp/tests/tools.rs`

- [ ] **Step 1: Let the test gateway give a table a comment**

The tests' gateway is SQLite, which keeps no comments. In `crates/tabletist-mcp/tests/support/mod.rs`, add to `struct Inner`:

```rust
    /// The comments the tests gave tables, by name: what PostgreSQL and
    /// MySQL would describe and SQLite cannot.
    comments: Mutex<Vec<(String, String)>>,
```

In `Shop::new`, in `Inner { .. }`, add `comments: Mutex::default(),`. After `Shop::mask`, add:

```rust
    /// Gives the table `table` a comment, as PostgreSQL and MySQL keep
    /// one: its description carries it.
    pub fn comment(&self, table: &str, words: &str) {
        let mut comments = self.inner.comments.lock().unwrap();
        comments.push((table.to_owned(), words.to_owned()));
    }
```

Change `ShopSession::describe` to:

```rust
    async fn describe(&self, object: &ObjectRef) -> Result<Structure, Failure> {
        let mut structure = self.connection.describe(object).await?;
        let comments = self.shop.inner.comments.lock().unwrap();
        structure.comment = comments
            .iter()
            .find(|(table, _)| *table == object.name)
            .map(|(_, words)| words.clone());
        Ok(structure)
    }
```

- [ ] **Step 2: Write the failing test**

In `crates/tabletist-mcp/tests/tools.rs`, after `describe_schema_marks_a_masked_column_and_a_view_that_reads_one`:

```rust
#[tokio::test]
async fn describe_schema_says_a_tables_comment_unless_it_could_give_a_masked_value() {
    let shop = Shop::new();
    shop.comment("orders", "One order of a customer");
    shop.comment("recent_orders", "The orders of the last thirty days");
    shop.comment("customer_names", "Who ordered");
    let (server, _log) = server(&shop);
    let agent = Agent::connect(&server).await;
    let comments = |answer: &Json| -> Vec<Option<String>> {
        let tables = answer["tables"].as_array().unwrap();
        tables
            .iter()
            .map(|table| table.get("comment").and_then(Json::as_str).map(str::to_owned))
            .collect()
    };
    let asked = json!({
        "connection": DEV,
        "tables": ["orders", "recent_orders", "customer_names", "order_items"],
    });
    let called = agent.call("describe_schema", asked.clone()).await;
    assert_eq!(
        comments(called.answer()),
        [
            Some("One order of a customer".to_owned()),
            Some("The orders of the last thirty days".to_owned()),
            Some("Who ordered".to_owned()),
            None,
        ]
    );
    // A table's comment is its author's words and may give a value of its
    // masked column as an example, as a column's comment may: the table
    // with the column and the view that reads it say none.
    shop.mask(DEV, &["orders.email"]);
    let called = agent.call("describe_schema", asked).await;
    assert_eq!(
        comments(called.answer()),
        [None, None, Some("Who ordered".to_owned()), None]
    );
    agent.close().await;
}
```

If `Json` is not imported in `tools.rs`, use `serde_json::Value` (check the file's `use` lines first and follow them).

- [ ] **Step 3: Run it to see it fail**

Run: `cargo test -p tabletist-mcp --test tools describe_schema_says_a_tables_comment`
Expected: FAIL, the first `assert_eq!` gets `[None, None, None, None]`.

- [ ] **Step 4: Say the comment in a table's entry**

In `crates/tabletist-mcp/src/tools/schema.rs` `table()`, replace

```rust
    if let Some(column) = hidden.reads_masked(&object.schema, &object.name) {
        entry.insert("reads_masked".into(), json!(column));
    }
```

with

```rust
    let reads_masked = hidden.reads_masked(&object.schema, &object.name);
    if let Some(column) = &reads_masked {
        entry.insert("reads_masked".into(), json!(column));
    }
    // The table's comment is its author's words, as a column's is, and
    // may give a value of a masked column of it as an example: a table
    // with one, or a view that reads one, says none.
    let masks = hidden
        .masks_table(Some(&object.schema), &object.name)
        .is_some();
    if !masks
        && reads_masked.is_none()
        && let Some(comment) = &structure.comment
    {
        entry.insert("comment".into(), answer::text(comment));
    }
```

In `tool()`'s description, change

```
                      and its indexes. Write a table of another schema as schema.table. A \
```

to

```
                      its indexes and its comment. Write a table of another schema as \
                      schema.table. A \
```

and change

```
                      comment and the values it may hold are left out. A view marked \
                      `reads_masked` may not be read: it reads such a column, or on MySQL \
                      reads its table.",
```

to

```
                      comment and the values it may hold are left out, and so is the \
                      comment of its table. A view marked `reads_masked` may not be read: \
                      it reads such a column, or on MySQL reads its table, and its comment \
                      is left out.",
```

Then check the whole string still reads as sentences: `grep -n -A 14 'title: "Describe the schema"' crates/tabletist-mcp/src/tools/schema.rs`.

- [ ] **Step 5: Run the test to see it pass**

Run: `cargo test -p tabletist-mcp`
Expected: PASS, every test of the crate. If a test pins the tool's description text, update it to the new words.

- [ ] **Step 6: Run the checks, then commit**

```bash
git add crates/tabletist-mcp
git commit -m "Say a table's comment in describe_schema

Unless the table has a masked column, or is a view that reads one: a
table's comment is its author's words, as a column's is, and may give a
masked value as an example.

Part of #193."
```

---

### Task 3: Comments in the Structure view

**Files:**
- Modify: `src/ui/structure.rs` (imports at 6-17; new consts and fns; the Columns grid at 389-432; the lines loop at 472-550; the top of the inner `Frame` at 375-381; a new `#[cfg(test)] mod tests` at the end)
- Create: `src/ui/comment_tests.rs`
- Modify: `src/ui/mod.rs` (declare the test module after `pub mod structure;`)

Background for the engineer:
- egui marks a hover-only widget hovered only when it was registered after (on top of) the click widget under the pointer. Each column's line gets a click widget after the grid is drawn, so a tooltip on the cell itself would never show. The tooltip goes on a hover widget registered after the line's click widget.
- `crate::ui::grid::ellipsize(text, max, keep_end, width)` cuts with "…" by bisection; `escape_hidden` writes out hidden characters; `for_display` keeps line breaks and bounds the text for layout.
- `harness.painted` is every text the last frame painted; `crate::testing::labels(&tree)` is every accessible name.

- [ ] **Step 1: Write the failing unit tests of `comment_line`**

At the end of `src/ui/structure.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    /// Eight points per character, "…" included.
    fn width(text: &str) -> f32 {
        text.chars().count() as f32 * 8.0
    }

    #[test]
    fn a_short_comment_is_shown_whole() {
        assert_eq!(
            comment_line("Login address", 320.0, width),
            ("Login address".to_owned(), false)
        );
    }

    #[test]
    fn a_long_line_is_cut_to_the_width() {
        let (shown, cut) = comment_line(&"x".repeat(60), 320.0, width);
        assert!(cut);
        assert!(shown.ends_with('…'), "{shown}");
        assert!(width(&shown) <= 320.0, "{shown}");
    }

    #[test]
    fn a_line_just_too_wide_is_cut_and_says_so() {
        // 41 characters at 8 points: one too many for 320.
        let (shown, cut) = comment_line(&"x".repeat(41), 320.0, width);
        assert!(cut);
        assert_eq!(shown, format!("{}…", "x".repeat(39)));
    }

    #[test]
    fn a_first_line_just_too_wide_of_more_lines_ends_in_one_ellipsis() {
        // Room for one "…" is kept first: 312 points, 38 and the "…".
        let comment = format!("{}\nmore", "x".repeat(41));
        let (shown, cut) = comment_line(&comment, 320.0, width);
        assert!(cut);
        assert_eq!(shown, format!("{}…", "x".repeat(38)));
    }

    #[test]
    fn a_comment_of_more_lines_shows_its_first_and_says_there_is_more() {
        assert_eq!(
            comment_line("Login address\nUsed to sign in", 320.0, width),
            ("Login address…".to_owned(), true)
        );
    }

    #[test]
    fn a_line_longer_than_the_limit_is_cut_there_before_it_is_measured() {
        let (shown, cut) = comment_line(&"x".repeat(100_000), f32::INFINITY, width);
        assert!(cut);
        assert_eq!(shown.chars().count(), COMMENT_LINE_CHARS + 1);
        assert!(shown.ends_with('…'));
    }

    #[test]
    fn hidden_characters_are_written_out() {
        let comment = "a\u{200B}b";
        let (shown, _) = comment_line(comment, f32::INFINITY, width);
        assert_eq!(shown, escape_hidden(comment));
        assert_ne!(shown, comment);
    }

    #[test]
    fn postgres_and_mysql_keep_comments_and_sqlite_does_not() {
        assert!(keeps_comments(Driver::Postgres));
        assert!(keeps_comments(Driver::MySql));
        assert!(!keeps_comments(Driver::Sqlite));
    }
}
```

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test --locked --lib ui::structure::tests`
Expected: compile error, `cannot find function 'comment_line'`.

- [ ] **Step 3: Write `comment_line` and `keeps_comments`**

In `src/ui/structure.rs`, change the imports:

```rust
use tabletist_db::Driver;
```

(after the `egui` imports) and

```rust
use crate::ui::format::{NAME_MAX_CHARS, display_safe, escape_hidden, for_display};
```

Then, after `const TAG: usize = 1;`:

```rust
/// The most of a comment's first line the Comment column lays out, as
/// many as of a name: a PostgreSQL comment has no limit, and the line is
/// measured every frame.
const COMMENT_LINE_CHARS: usize = NAME_MAX_CHARS;

/// The widest a comment runs in the Comment column.
const COMMENT_WIDTH: f32 = 320.0;

/// Whether the engine keeps a comment on a column, for the Columns table
/// to have a place for it.
fn keeps_comments(driver: Driver) -> bool {
    match driver {
        Driver::Postgres | Driver::MySql => true,
        Driver::Sqlite => false,
    }
}

/// What the Comment column shows of `comment`: its first line, cut to
/// `COMMENT_LINE_CHARS`, hidden characters written out, then cut with
/// "…" to `max` points as `width` measures; and whether that leaves any
/// of the comment out. A line that goes on, or a comment that does, ends
/// in "…" too.
fn comment_line(comment: &str, max: f32, width: impl Fn(&str) -> f32) -> (String, bool) {
    let mut lines = comment.lines();
    let first = lines.next().unwrap_or_default();
    let more = lines.next().is_some();
    let (kept, long) = match first.char_indices().nth(COMMENT_LINE_CHARS) {
        Some((at, _)) => (&first[..at], true),
        None => (first, false),
    };
    let escaped = escape_hidden(kept);
    let goes_on = long || more;
    // Room for the "…" that says it goes on.
    let max = if goes_on { max - width("…") } else { max };
    let shown = crate::ui::grid::ellipsize(&escaped, max, false, &width);
    // `ellipsize` lends the text back only where it fits. Not by length:
    // its "…" is three bytes, so a line cut by one character is longer.
    let narrowed = matches!(shown, std::borrow::Cow::Owned(_));
    let mut shown = shown.into_owned();
    if goes_on && !narrowed {
        shown.push('…');
    }
    (shown, goes_on || narrowed)
}
```

If `ellipsize` refuses `&width` (it takes `impl Fn(&str) -> f32`), pass `|text: &str| width(text)` instead.

- [ ] **Step 4: Run them to see them pass**

Run: `cargo test --locked --lib ui::structure::tests`
Expected: 8 passed. (`NAME_MAX_CHARS` is `pub` in `src/ui/format.rs`; `escape_hidden` is `pub` there too.)

- [ ] **Step 5: Write the failing headless tests**

Create `src/ui/comment_tests.rs`:

```rust
//! The comments the app shows, checked against their spec
//! (`docs/superpowers/specs/2026-10-20-structure-comments-design.md`):
//! the table's over its Structure view, each column's in the view's
//! Comment column, and each column's on its header in the grid.

use egui::accesskit::Role;
use tabletist_db::{ColumnInfo, Driver, Structure};

use crate::model::{Action, ObjectView};
use crate::testing::{Harness, bounds, labels, page};

fn column(name: &str, comment: Option<&str>) -> ColumnInfo {
    ColumnInfo {
        name: name.into(),
        type_name: "text".into(),
        comment: comment.map(Into::into),
        ..ColumnInfo::default()
    }
}

fn described(columns: Vec<ColumnInfo>, comment: Option<&str>) -> Structure {
    Structure {
        columns,
        primary_key: vec!["id".into()],
        comment: comment.map(Into::into),
        ..Structure::default()
    }
}

/// The fake connection's `users`, as `driver` would describe it, in its
/// Structure view described as `structure`.
fn structure_view(driver: Driver, structure: Structure) -> Harness {
    let mut harness = Harness::new();
    let tab = harness.connect_fake();
    harness.app.workspace_mut(tab).unwrap().driver = driver;
    harness.click("users");
    harness.answer_rows(page(5, false));
    let object_tab = harness.app.workspace(tab).unwrap().active_tab.unwrap();
    harness.app.apply(Action::SetView {
        tab,
        object_tab,
        view: ObjectView::Structure,
    });
    harness.answer_structure(structure);
    harness.settle();
    harness
}

/// Moves the pointer to `at` and waits past the tooltip's delay. Returns
/// every name on screen then.
fn hover(harness: &mut Harness, at: egui::Pos2) -> Vec<String> {
    harness.frame(vec![egui::Event::PointerMoved(at)]);
    // Each frame is 1/60 s.
    for _ in 0..60 {
        harness.frame(Vec::new());
    }
    labels(&harness.frame(Vec::new()))
}

/// The text the last frame painted that begins with `start`.
fn painted_from(harness: &Harness, start: &str) -> String {
    harness
        .painted
        .iter()
        .find(|(text, _)| text.starts_with(start))
        .map(|(text, _)| text.clone())
        .unwrap_or_else(|| panic!("{start}… is painted"))
}

// Test 1 of the spec.
#[test]
fn postgres_and_mysql_show_a_columns_comment_in_the_comment_column() {
    for driver in [Driver::Postgres, Driver::MySql] {
        let columns = vec![column("id", None), column("email", Some("Login address"))];
        let mut harness = structure_view(driver, described(columns, None));
        assert!(harness.has("Comment"), "{driver:?}");
        assert!(harness.has("Login address"), "{driver:?}");
    }
}

#[test]
fn a_long_comment_is_cut_and_said_whole() {
    // Under `DISPLAY_LINE_CHARS` (500): `for_display` keeps it one line.
    let long = ["Where the order ships to."; 12].join(" ");
    let columns = vec![column("id", None), column("address", Some(&long))];
    let mut harness = structure_view(Driver::Postgres, described(columns, None));
    // A screen reader hears the whole of it.
    assert!(harness.has(&long));
    let drawn = painted_from(&harness, "Where the order");
    assert!(drawn.ends_with('…'), "{drawn}");
    assert!(drawn.chars().count() < long.chars().count(), "{drawn}");
    // The pointer over it shows it whole: said once more, by the tooltip.
    let at = harness.painted_rect(&drawn).expect("where it is drawn").center();
    let before = labels(&harness.settle()).iter().filter(|name| **name == long).count();
    let after = hover(&mut harness, at).iter().filter(|name| **name == long).count();
    assert_eq!(after, before + 1);
}

#[test]
fn a_comment_of_more_lines_shows_its_first_and_says_it_whole() {
    let comment = "Login address\nUsed to sign in";
    let columns = vec![column("id", None), column("email", Some(comment))];
    let mut harness = structure_view(Driver::Postgres, described(columns, None));
    assert!(harness.has(comment));
    assert_eq!(painted_from(&harness, "Login address"), "Login address…");
}

#[test]
fn a_comment_of_a_hundred_thousand_characters_on_one_line_is_drawn_cut() {
    let huge = "x".repeat(100_000);
    let columns = vec![column("id", None), column("notes", Some(&huge))];
    let harness = structure_view(Driver::Postgres, described(columns, None));
    let drawn = painted_from(&harness, "xxx");
    assert!(drawn.ends_with('…'), "{} characters", drawn.chars().count());
    assert!(drawn.chars().count() <= 257, "{} characters", drawn.chars().count());
}

// Test 2 of the spec.
#[test]
fn sqlite_has_no_comment_column() {
    let columns = vec![column("id", None), column("email", None)];
    let mut harness = structure_view(Driver::Sqlite, described(columns, None));
    assert!(harness.has("Nullable"));
    assert!(!harness.has("Comment"));
}

// Test 3 of the spec.
#[test]
fn the_tables_comment_stands_over_the_view_and_takes_no_room_without_one() {
    let columns = || vec![column("id", None)];
    let top_of_columns = |harness: &mut Harness| {
        let tree = harness.settle();
        bounds(&tree, "Columns", Role::Label).expect("the Columns heading").top()
    };
    let mut with = structure_view(
        Driver::Postgres,
        described(columns(), Some("People who sign in")),
    );
    assert!(with.has("People who sign in"));
    let tree = with.settle();
    let comment = bounds(&tree, "People who sign in", Role::Label).expect("the comment");
    let label = bounds(&tree, "Label for foreign keys", Role::Label)
        .expect("the label card's title");
    assert!(comment.bottom() <= label.top(), "{comment:?} over {label:?}");
    let mut without = structure_view(Driver::Postgres, described(columns(), None));
    assert!(top_of_columns(&mut without) < top_of_columns(&mut with));
}
```

In `src/ui/mod.rs`, after `pub mod structure;`:

```rust
#[cfg(test)]
mod comment_tests;
```

Notes on the names the tests read (check them against the code, adjust the test, not the view, if one differs):
- The label card's title node is "Label for foreign keys": `Harness::new()` uses `Look::standard()`, which writes the title as the macOS look does (`label_part` lays it out with `widgets::label`).
- `Action::SetView { tab, object_tab, view }` is how `src/shots.rs` switches to the Structure view; clicking "Structure" works too if that is simpler.
- `bounds` matches `node.label().or_else(|| node.value())`, so plain labels are found by their text.

- [ ] **Step 6: Run them to see them fail**

Run: `cargo test --locked --lib ui::comment_tests`
Expected: FAIL in each but `sqlite_has_no_comment_column` (no "Comment" title, no comment painted, no table comment).

- [ ] **Step 7: Draw the comment cell**

In `src/ui/structure.rs`, after `fn cell(..)`:

```rust
/// A column's comment in the Comment column: its first line, cut as
/// [`comment_line`] cuts it, and the whole comment for a screen reader.
/// Where the cell is, when what it shows leaves some of the comment out:
/// the pointer is shown the whole there.
fn comment_cell(
    ui: &mut egui::Ui,
    comment: Option<&str>,
    look: &Look,
    palette: &Palette,
) -> Option<Rect> {
    let Some(comment) = comment else {
        cell(ui, "", false, look, palette);
        return None;
    };
    let role = widgets::body(look);
    let ctx = ui.ctx().clone();
    let width = |text: &str| role.width(&ctx, look.faces, text);
    let (shown, cut) = comment_line(comment, COMMENT_WIDTH, width);
    let laid = Text::one(look, role, &shown, palette.text).layout(ui.ctx());
    let (rect, response) = ui.allocate_exact_size(laid.size(), Sense::hover());
    // Made only while a screen reader, or a test, reads the tree.
    response.widget_info(|| WidgetInfo::labeled(WidgetType::Label, true, for_display(comment)));
    if ui.is_rect_visible(rect) {
        laid.paint(ui.painter(), rect.min);
    }
    cut.then_some(rect)
}
```

(`WidgetInfo::labeled` takes `impl ToString`; pass `&for_display(comment)` if it asks for a reference.)

- [ ] **Step 8: Put the Comment column in the Columns table**

In `show()`, after `let dialect = workspace.driver.dialect();`:

```rust
    // Where the engine keeps no comment, the view has no place for one.
    let comments = keeps_comments(workspace.driver);
```

In the Columns grid, replace

```rust
                                for title in ["Name", "Type", "Nullable", "Default", "Key"] {
                                    quiet(ui, &gettext(locale, title), &look, &palette);
                                }
```

with

```rust
                                let titles: &[&str] = if comments {
                                    &["Name", "Type", "Nullable", "Default", "Key", "Comment"]
                                } else {
                                    &["Name", "Type", "Nullable", "Default", "Key"]
                                };
                                for title in titles {
                                    quiet(ui, &gettext(locale, title), &look, &palette);
                                }
```

Before the `egui::Grid::new(("structure-columns", ..))` line, next to `let mut lines = ..`, add:

```rust
                        // Where each cut comment is drawn, by line.
                        let mut cut = Vec::with_capacity(structure.columns.len());
```

and in the row, after `cell(ui, key, false, &look, &palette);` and before `ui.end_row();`:

```rust
                                    if comments {
                                        let comment = column.comment.as_deref();
                                        cut.push(comment_cell(ui, comment, &look, &palette));
                                    }
```

In the lines loop (`for (line, (column, rows)) in structure.columns.iter().zip(&lines).enumerate()`), right after `let response = ui.interact(rect, ui.id().with(row), Sense::CLICK);` and its `if response.clicked() ..` block:

```rust
                            // The whole of a cut comment under the pointer.
                            // Over the line, which takes the clicks: a
                            // widget under it would not be hovered.
                            if let Some(Some(at)) = cut.get(line) {
                                let id = ("structure-comment", object_tab.0, line);
                                let tip = ui.interact(*at, ui.id().with(id), Sense::hover());
                                if tip.hovered()
                                    && let Some(comment) = &column.comment
                                {
                                    let _ = tip.on_hover_text(for_display(comment));
                                }
                            }
```

- [ ] **Step 9: Draw the table's comment over the view**

After `fn quiet(..)`:

```rust
/// The table's own comment, over everything else of the view: its
/// author's words, wrapped as the label card's help is.
fn table_comment(ui: &mut egui::Ui, comment: &str, look: &Look, palette: &Palette) {
    let wrap = HELP.min(ui.available_width());
    Text::one(look, widgets::body(look), &for_display(comment), palette.secondary)
        .wrap(wrap)
        .layout(ui.ctx())
        .label(ui);
    ui.add_space(16.0);
}
```

In `show()`, inside the inner `Frame::new() ... .show(ui, |ui| {`, before `if let Some(label) = &label {`:

```rust
                        if let Some(comment) = &structure.comment {
                            table_comment(ui, comment, &look, &palette);
                        }
```

- [ ] **Step 10: Run the tests to see them pass**

Run: `cargo test --locked --lib ui::comment_tests && cargo test --locked --lib ui::structure`
Expected: PASS.

Then the Structure view's other tests, which must not change:

Run: `cargo test --locked --lib structure`
Expected: PASS.

- [ ] **Step 11: Look at it**

Run the shots (they need a GPU, which a Mac has):

```bash
cargo test --locked --features shots --lib shots::shots -- --ignored
```

Open `target/shots/structure-macos-light.png` and `target/shots/structure-omarchy-dark.png` (Read them as images). The Bookshop scene has no comments until Task 5, so check only that nothing moved: the Comment title after Key, the lines as tall as before. If the shots cannot run here, say so in the report and move on.

- [ ] **Step 12: Run the checks, then commit**

Run the four checks, then:

```bash
git add src/ui/structure.rs src/ui/comment_tests.rs src/ui/mod.rs
git commit -m "Show the table's and the columns' comments in the Structure view

The table's comment stands over the view. On PostgreSQL and MySQL the
Columns table has a Comment column: a comment's first line, cut to 320
pt with the whole under the pointer, and whole for a screen reader.
SQLite keeps no comments and has no such column.

Part of #193."
```

---

### Task 4: A column's comment on its header in the grid

**Files:**
- Modify: `src/ui/grid.rs` (`pub struct Column` at 55-71; the header loop around 1252-1272; test columns at 2015-2034, 2989-2998, 3109-3118; a new test)
- Modify: `src/ui/data_view.rs:1491-1508`, `src/ui/sql_results.rs:1915-1923`, `src/ui/settings/preview.rs:211-220`
- Test: `src/ui/grid.rs` tests, `src/ui/comment_tests.rs`

- [ ] **Step 1: Write the failing headless test**

In `src/ui/comment_tests.rs`, add `node` to the `crate::testing` import (`use crate::testing::{Harness, bounds, labels, node, page};`), and at the end of the file:

```rust
// Test 4 of the spec.
#[test]
fn a_columns_comment_is_its_headers_description_and_tooltip_in_the_grid() {
    let mut harness = Harness::new();
    harness.connect_fake();
    harness.click("users");
    harness.answer_rows(page(5, false));
    let columns = vec![
        column("id", None),
        column("email", Some("Login address")),
        column("meta", None),
    ];
    harness.answer_structure(described(columns, None));
    let tree = harness.settle();
    let description = |label: &str| {
        let id = node(&tree, label, Role::Button).unwrap_or_else(|| panic!("{label}"));
        let (_, found) = tree.nodes.iter().find(|(node, _)| *node == id).unwrap();
        found.description().map(str::to_owned)
    };
    assert_eq!(description("email").as_deref(), Some("Login address"));
    assert_eq!(description("id"), None);
    let at = bounds(&tree, "email", Role::Button).expect("the email header").center();
    assert!(hover(&mut harness, at).iter().any(|name| name == "Login address"));
}
```

- [ ] **Step 2: Write the failing grid unit test**

In `src/ui/grid.rs` tests, after `a_header_that_sorts_nothing_is_a_label_that_keeps_clicks_from_the_rows`:

```rust
    #[test]
    fn a_header_says_its_columns_comment_after_its_name() {
        let ctx = egui::Context::default();
        crate::theme::install(&ctx, false, &Look::standard());
        ctx.enable_accesskit();
        for sortable in [true, false] {
            let mut columns = columns_that(sortable);
            columns[1].comment = Some("Login address");
            frame_of(&ctx, &columns, 3, vec![]);
            let (_, tree) = frame_of(&ctx, &columns, 3, vec![]);
            let role = if sortable { Role::Button } else { Role::Label };
            let description = |label: &str| {
                let id = crate::testing::node(&tree, label, role).expect(label);
                let (_, found) = tree.nodes.iter().find(|(node, _)| *node == id).unwrap();
                found.description().map(str::to_owned)
            };
            assert_eq!(description("email").as_deref(), Some("Login address"));
            assert_eq!(description("id"), None);
        }
    }
```

- [ ] **Step 3: Run both to see them fail**

Run: `cargo test --locked --lib comment`
Expected: compile error, `no field 'comment' on type 'Column'`.

- [ ] **Step 4: Add `Column.comment`**

In `src/ui/grid.rs` `pub struct Column<'a>`, after `required`:

```rust
    /// What the table's author wrote of the column: its header's tooltip,
    /// and said after its name to a screen reader.
    pub comment: Option<&'a str>,
```

Add `comment: None,` after `required: false,` in the test columns (`columns_that`, both columns; the two `.map(|..| Column { .. })` in the tests at about 2989 and 3109), in `src/ui/sql_results.rs` (a result has no structure) and in `src/ui/settings/preview.rs`.

- [ ] **Step 5: Say it on the header**

In `src/ui/grid.rs`, in the header loop, right after `draw_header(ui, &painter, rect, column, pad, look, palette);`:

```rust
            // The column's comment: said after its name, and shown under
            // the pointer. Laid out only when it is asked for.
            if let Some(comment) = column.comment {
                ui.ctx().accesskit_node_builder(response.id, |node| {
                    node.set_description(crate::ui::format::for_display(comment));
                });
                if response.hovered() {
                    let tip = crate::ui::format::for_display(comment);
                    let _ = response.clone().on_hover_text(tip);
                }
            }
```

- [ ] **Step 6: Fill it in the data view**

In `src/ui/data_view.rs`, in the `.map(|column| { .. })` that builds `Column`s, before `Column {`:

```rust
                let comment = structure
                    .and_then(|structure| {
                        structure.columns.iter().find(|info| info.name == column.name)
                    })
                    .and_then(|info| info.comment.as_deref());
```

and in the literal, after `required: false,`:

```rust
                    comment,
```

- [ ] **Step 7: Run the tests to see them pass**

Run: `cargo test --locked --lib comment && cargo test --locked --lib ui::grid`
Expected: PASS.

- [ ] **Step 8: Run the checks, then commit**

```bash
git add src/ui
git commit -m "Show a column's comment on its header in the grid

Under the pointer, and after the column's name for a screen reader.
A SQL result's columns have none.

Part of #193."
```

---

### Task 5: The website and its picture

**Files:**
- Modify: `src/shots.rs` (`structure()` at 163-240, and the other `Structure` literals if Task 1 did not already name `comment`)
- Modify: `docs/_guide/browsing-data.md:84-92`
- Modify: `assets/screenshots/macos-structure.webp`

- [ ] **Step 1: Give the Bookshop scene comments**

In `src/shots.rs` `structure()`, change the `column` closure's `comment: None,` to

```rust
        // Two of them say what they hold, as a Bookshop's author would.
        comment: match name {
            "kind" => Some("Cover or preview page".into()),
            "image_data" => Some("Its width, height and the address of each size".into()),
            _ => None,
        },
```

and in the `Structure { .. }` literal it returns, set

```rust
        comment: Some("The pictures of a book: its cover and the pages of its preview".into()),
```

(in place of the `comment: None,` Task 1 put there).

Run: `cargo build --locked --all-targets --features shots`
Expected: builds.

- [ ] **Step 2: Render the website's picture again**

The website's pictures are 1280 by 800 points at 2x, in the macOS look, light. The scene that sizes them for the site is temporary and is not committed (as before: see commit 8e9edfd). Add to the end of `src/shots.rs`, for this step only:

```rust
#[test]
#[ignore = "temporary: the website's Structure picture"]
fn site_structure() {
    let look = crate::theme::Look::macos();
    let mut harness = Harness::for_shots(egui::vec2(1280.0, 800.0), 2.0, true, look);
    harness.app.titlebar = crate::app::TitleBar {
        height: 38.0,
        inset: 80.0,
    };
    let tab = workspace(&mut harness);
    let object_tab = harness.app.workspace(tab).unwrap().active_tab.unwrap();
    harness.app.apply(Action::SetView {
        tab,
        object_tab,
        view: crate::model::ObjectView::Structure,
    });
    harness.answer_structure(structure());
    harness.shot(&out("site-macos-structure.png"));
}
```

Run:

```bash
cargo test --locked --features shots --lib shots::site_structure -- --ignored
cwebp -lossless -exact target/shots/site-macos-structure.png -o assets/screenshots/macos-structure.webp
webpinfo -summary assets/screenshots/macos-structure.webp
```

Expected: the test passes, and `webpinfo` says 2560 by 1600, as the old picture was. Read the PNG as an image and check:
- the table's comment over the label card;
- the Comment column after Key, with the two comments;
- nothing cut that should not be, and no text running off the window's right edge (a review render found the first `kind` comment clipped there, hence its short words; if one still is, shorten that comment in the scene).

Then delete `site_structure` from `src/shots.rs`. If the shots cannot run here (no GPU), keep the old picture, say so in the report, and go on.

- [ ] **Step 3: Write it on the website**

In `docs/_guide/browsing-data.md`, change the Structure section's first lines:

```markdown
Switch a table from **Data** to **Structure** to read its columns, indexes
and foreign keys.

{% include shot.html file="macos-structure" alt="The Structure view of a table: the label for foreign keys, then its columns, indexes and foreign keys" %}
```

to

```markdown
Switch a table from **Data** to **Structure** to read its columns, indexes
and foreign keys.

{% include shot.html file="macos-structure" alt="The Structure view of a table: its comment, the label for foreign keys, then its columns with their comments, its indexes and foreign keys" %}

On PostgreSQL and MySQL the view shows the comments written on the table
and its columns. The table's comment stands above everything else. Each
column's is in the **Comment** column, on one line: hold the pointer over
a comment that is cut to read all of it. SQLite keeps no comments, so its
tables have no Comment column. In the grid, hold the pointer over a
column's header to read its comment.
```

Check the words have no em dash: `perl -CSD -ne 'print "$.: $_" if /\x{2014}/' docs/_guide/browsing-data.md` prints nothing.

If Ruby and Bundler are set up, build the site as CI does:

```bash
cd docs && bundle install && bin/check
```

Expected: passes. If they are not set up, say so in the report: `docs.yml` checks it on the pull request.

- [ ] **Step 4: Run the checks, then commit**

```bash
git add src/shots.rs docs/_guide/browsing-data.md assets/screenshots/macos-structure.webp
git commit -m "Describe comments on the website, and show them in its picture

The Structure section says where the table's and the columns' comments
show, and that SQLite keeps none. The Bookshop scene has a table
comment and two column comments, and the website's Structure picture is
rendered again from it.

Closes #193."
```

---

### Task 6: Check the whole

- [ ] **Step 1: Every check, and the database suites**

```bash
cargo fmt --all --check
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo test --locked --workspace --all-targets
RUSTDOCFLAGS='-D warnings' cargo doc --locked --workspace --no-deps
TABLETIST_TEST_PG_URL=postgres://tabletist:tabletist@localhost:55432/tabletist TABLETIST_TEST_MYSQL_URL=mysql://tabletist:tabletist@localhost:53306/tabletist cargo test -p tabletist-db --test postgres --test mysql
```

Expected: all pass, and no database test prints "skipped".

- [ ] **Step 2: The engine rule**

Run: `cargo test --locked --test engines`
Expected: PASS (`keeps_comments` matches `Driver` with every variant named).

- [ ] **Step 3: Report coverage honestly**

What ran: macOS only. Linux and Windows were compiled by CI only. The Omarchy look's Structure view was seen in a shot (if the shots ran), not in a running window.

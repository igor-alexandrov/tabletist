# Foreign Key Label, Run 1: The Choice Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A foreign key's picker labels and searches the referenced table by the column kept for that table on that saved connection, falls back to the app's pick, and says which column it is and who chose it, in both looks.

**Architecture:** `src/labels.rs` holds the choice (`LabelChoice`), its store (`Labels`, the file `labels.json` in the state directory) and the one rule that turns a choice and a table's columns into what labels its rows (`resolve`). It has no UI and talks to nothing. `src/app/labels.rs` is the reducer's part: `App::label_of`, the only reader of the store, and what happens when a chosen column is gone. The picker (`src/key_picker.rs`, `src/app/key_picker.rs`) keeps what `resolve` answered in place of a bare column name. The views read it from the picker: the macOS look's foot and the Omarchy look's status line.

**Tech Stack:** Rust 1.98, egui (the crmne fork), serde_json for the store, AccessKit for the headless tests (`src/testing.rs`). Spec: `docs/superpowers/specs/2026-10-12-foreign-key-label-design.md` (section 1, the first half of section 2, the data half of the "renamed or dropped" state of section 5; tests 1, 3, 14 and what test 9 keeps).

---

## Read this first

### The runs

The brief asks for one pull request. It is built in four runs, each a set of commits that passes the checks and works by itself. This plan is run 1. The later runs are planned when their turn comes, against the tree as it then stands.

| Run | What it builds | Spec tests |
|---|---|---|
| **1. The choice** (this plan) | The choice, its file, the resolver, the picker labelled by it, the words that say so | 1, 3, 14, and what 9 keeps |
| 2. The chooser from the picker | The keys in the keymap, the foot's button (its tooltip, its disabled form, its name for a screen reader), the chooser in both looks with the key and the bytes listed and disabled, choosing and searching again, Esc order, loading, key-only, "just changed", the "is gone" notice, the names and announcements of section 6 | 2, 4, 6 (in the chooser), 7, 8, 9, 10, 11, 13, 15 |
| 3. The Structure door | The "Label for foreign keys" line, the `label` tag, a cursor on the column lines and a column's menu (the Structure view has neither today), `gl`, `:label` and its three forms, on read-only connections | 5, 6 (through `:label`), 12, 15 |
| 4. Finish | The website's key and editing pages, the optional grid door (or the note that it was skipped), the pull request | |

After run 1 nothing in the app sets a choice yet: the picker shows `Label name · Automatic`, and a choice written into `labels.json` by hand is honoured. Run 2 is where a user first changes it.

Three things this run leaves for run 2's plan:

- **A picker that is open and labelled is not labelled again when its column goes.** `picker_columns` gives a label only to a picker that waits for one. If a refresh drops the chosen column while a picker is open on it, that picker searches by the column once more and shows the database's error. Run 2's "choose, then search again" needs the same step: label an open picker again and send its search.
- **`src/ui/label_tests.rs` spells no key yet.** The first test there that writes a key in a string (`ctrl+l`) must add the file to `TESTS` in `tests/keys.rs`, as the other test files are.
- **The Structure view is three plain tables in both looks.** No line of it can be focused, has a cursor or has a menu, and its letters are read as the grid's. Run 3 builds that before it can hang `gl` and "Use as label" on a column.

### What was run where this plan was written

Everything in it. The whole run was built first, then replayed task by task, and each task's tree was checked alone: `cargo fmt --check`, `cargo clippy --workspace --all-targets -D warnings`, clippy with the `shots` feature (tasks 2 and 3) and the app's tests; the last tree passed the four checks whole (2914 tests across the workspace). The code blocks and diffs below are those trees, not a retelling, and the diffs were applied in order to a copy of `89a7e88` by the plan's reviewer. The line numbers in the diffs are of that tree.

Two things were not run:

- **PostgreSQL and MySQL.** This session cannot reach Docker, so task 4's two server tests printed "skipped". The SQLite one ran and passes. CI runs all three.
- **A real window.** The picker was rendered headless in the three looks (the scene `edit_key_picker` of `src/shots.rs`) and looked at: the macOS foot reads `Esc cancels · Label title · Automatic`, and the Omarchy status line `-- INSERT -- book_id · int8  label title · automatic` with the keys at its right. Nothing was pressed by hand.

### The keys were checked against the keymap

The brief says to stop if the keymap's collision test fails on a proposed key. It does not. Every proposed key was added to `src/keymap.rs` as a trial, and its 18 tests passed, `no_two_commands_share_a_key_where_both_could_be_read` among them. The trial was then taken out: the keys are run 2's and run 3's to add.

| Brief's scope | In the keymap | Keys tried |
|---|---|---|
| fk-picker | `cell-editor`, when the picker's list is open (`When::KeyPicker`, which exists) | `cmd+l` / `ctrl+l` |
| fk-chooser | `prompt`, when the chooser is open (a new `When::LabelChooser`), as Quick open and the question about leaving are | `up` `down` / `j` `k` `ctrl+n` `ctrl+p`, `enter`, `esc`, and `shift+enter` claimed for the variant |
| structure | Tried in `grid` with no condition, the placement where the most could clash | `cmd+l` / `:label`, `alt+cmd+l` / `gl`, `:label auto`, `:label none` |

Why the three overlaps the brief names pass: `ctrl+l` as Pane right is normal mode's and the picker's is insert mode's; `cmd+l` as Go to line is the SQL editor's scope, which is never read beside a cell's editor; `ctrl+shift+enter` is another chord than `shift+enter`.

Two things run 3 has to decide, neither a collision:

- **The Structure view has no scope of its own.** Its letters are read as the grid's today. A scope `structure` beside the nine is what the brief means; the trial shows the keys also fit in `grid`.
- **`:label {column}` takes an argument.** The prompt reads a line by comparing all of it with a binding (`Keymap::ex`); only `:N` is read otherwise. `:label auto` and `:label none` fit as they are. The form with a column needs the reader to take a line that begins with `label `.

### Where the boards and the app differ

| The design asks | What is built | Why |
|---|---|---|
| Omarchy: the picker is a float with its own `> harb` field, a `▌` bar, a `✓` on the current row and a count | The picker stays the list under the cell's field, with `· current`, as pull request 114 built it. This feature adds the status line's words (run 1) and the chooser's float beside the list (run 2) | The float is the picker's own shape, not the label's. Changing it belongs to a change of the picker |
| Omarchy status line: `-- INSERT --  label name · automatic` | `-- INSERT --  book_id · int8  label name · automatic`. The label follows the column being edited, and gives way before the key hints on a narrow line | The app's line names the column in insert mode, where the board names it over the float |
| macOS foot: a button `Label name · Automatic ▾` with a tooltip | Run 1: the same words, as words. Run 2 makes them the button that opens the chooser | There is nothing to open until run 2 |
| Strings for a label column, and `Label keys only` for a table with nothing to choose | Two more states get words the notes board has none for: `Label keys only · Automatic` (no text column, but one could be chosen: `shelves (id, floor)`), and `Label keys only · Set by you` (No label was chosen) | Both can be changed in the chooser, so neither is the disabled `Label keys only` |
| Numbers and dates "drawn as the grid draws them" | Drawn as the list draws a key: plain, as the database writes the value as text. A grid with digit grouping on shows `1,234` where a label shows `1234` | It is the text the search matches (open question 2: searched only as drawn). A label drawn grouped could be typed and not found |
| "The key" cannot be a label | In a picker: the column its foreign key references. In Structure (run 3): the primary key. A foreign key that references another column lists keys alone where that column is the chosen label | The lookup names the key and the label in one select list, and the app's pick already left out only the referenced column |
| A "told" flag for the "is gone" notice | The store keeps the lost column's name until it is said, then drops it | The same once-only, and the file holds nothing that is over |
| `resolve(conn, table)` | `App::label_of(tab, table, key)` | The app's pick depends on which column the key references |

### What was decided

- **The file** is `labels.json` in the state directory (`AppDirs::state`): `$XDG_STATE_HOME/tabletist` on Linux, `~/Library/Application Support/dev.tabletist.Tabletist` on macOS, `%LOCALAPPDATA%\tabletist\Tabletist\data` on Windows. It is read once at start on the UI thread, as `connections.json` is, and written by the backend (`Command::Save`). A file that cannot be read is kept aside as `labels.json.bad` and the store starts empty (`util::load_json`).
- **Deleting a saved connection deletes its choices.** A duplicate is a new saved connection and starts with none.
- **A chosen column is called gone where the table's columns arrive** (`Event::Structure`), and only by columns that are fresh: a picker that opens while they are being read again waits for the answer, and a read that failed says nothing (it leaves the columns from before it). The choice goes back to Automatic there and the file is written. Saying it is run 2's.
- **A chosen column that turned into bytes**, or that is this picker's own key, labels nothing and stays the user's choice. It is not "gone": the notice would say what is not true.
- **Open questions:** every default of the brief is kept. Separate choices per saved connection, labels searched only as drawn, no sample values, nothing new in the referencing table's Structure, the state file and not `settings.toml`, lower case in the Omarchy look.

### The Bookshop fixture

The brief says to use the columns the Bookshop fixture has. What it has (`src/testing.rs`):

- `publishers (id, name)`, `book_covers`, `genres (id, parent_id, name)`. No `legal_name`, `email`, `city`, `founded_on` or `logo`.
- No `isbns` table. `isbns` is only a name in a list of a screenshot scene (`src/shots.rs`), with no columns.
- No date and no bytes in any table a key points at.

So run 1's tests use:

- `publishers.name`, and `genres.parent_id` for a label that is a number;
- `legal_name`, a name `publishers` lacks, for the column that is gone (a column that is gone is one the table does not have);
- in the database tests, the Bookshop's `editions` with the two columns the screenshot scenes give it that matter here (`price numeric(10,2)`, `published_on date`) and one they do not: `pages`, a whole number, which the scenes keep inside the edition's `metadata`. Its three rows are made up for the test, as the lookup tests' publishers are;
- in the unit tests of the rule, column lists written in the test: `isbns (isbn)`, `shelves (id, floor)` as the lookup tests have it, and a key beside a column of bytes.

Run 1 adds nothing to the fixture of `src/testing.rs`. Run 2 needs the smallest key-only table there (`isbns (isbn)`) and one column of bytes on a table a key points at; its plan says which. All of this goes into the pull request's list of replaced placeholders.

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

- `src/shots.rs` is outside those four and builds pickers. Tasks 2 and 3 also run `~/.cargo/bin/cargo clippy --locked --features shots --lib --tests -- -D warnings`.
- Run `~/.cargo/bin/cargo fmt --all` before each task's checks.
- **No pixels.** The tests check behaviour, names and words through the AccessKit tree and the commands sent. No snapshot is added and no picture is committed. To look at the result, render the scene `edit_key_picker` with a throwaway `#[test] #[ignore]` in `src/shots.rs`, and take the test out again before the checks.
- House rules that bite here: views push `Action`s and change nothing; disk work is the backend's; text is drawn through `TextRole`s; strings go through `gettext`; code that depends on the engine names every variant; no em dashes; a key is spelled only in `src/keymap.rs` (this run spells none).
- Commits are signed, one per task, after its checks pass. If signing fails ("agent refused operation": 1Password is locked), do not commit unsigned: stage the task and tell the user. Each message ends with:

      Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>

- A diff below applies with `git apply`. Where the tree has moved, find the place by the names around it.

## File structure

| File | What it is for |
|---|---|
| `src/labels.rs` (new) | `LabelChoice`, `Source`, `Resolved`, `Excluded`; `excluded`, `automatic`, `resolve`; the store `Labels` and its file. Pure |
| `src/app/labels.rs` (new) | `App::label_of`, the one reader of the store; `App::check_label`, which finds a chosen column gone |
| `src/paths.rs` | `AppDirs::labels_file()` |
| `src/backend.rs` | `StateFile::Labels`, so the backend writes the file |
| `src/app.rs` | `App::labels`, read at start; `save_labels`; a deleted connection forgets its choices; columns that arrive are checked |
| `src/key_picker.rs` | `Columns::Known` holds a `Resolved`. `label_column` moves to `labels::automatic` |
| `src/app/key_picker.rs` | The picker asks the resolver where it asked `label_column` |
| `src/ui/key_picker.rs` | `label_line`, the words; the foot draws them |
| `src/ui/workspace.rs` | The Omarchy status line draws them in insert mode |
| `src/ui/label_tests.rs` (new) | The headless tests of this spec, on the Bookshop's fixture |
| `crates/tabletist-db/tests/lookup.rs` | Test 14 on the three engines |
| `docs/_reference/settings-and-files.md` | The website's list of files names `labels.json` |

---

### Task 1: The choice, the rule and the store

**Files:**
- Create: `src/labels.rs`
- Modify: `src/lib.rs`

Nothing uses the module yet. It is public, so nothing in it is dead code to clippy.

- [ ] **Step 1: Declare the module**

In `src/lib.rs`, after `pub mod known_hosts;`:

```rust
pub mod labels;
```

- [ ] **Step 2: Write the failing tests**

Create `src/labels.rs` with the module's tests alone:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn column(name: &str, type_name: &str) -> ColumnInfo {
        ColumnInfo {
            name: name.into(),
            type_name: type_name.into(),
            ..ColumnInfo::default()
        }
    }

    /// The Bookshop's `genres`: a key, a number and a name.
    fn genres() -> Vec<ColumnInfo> {
        vec![
            column("id", "INTEGER"),
            column("parent_id", "INTEGER"),
            column("name", "TEXT"),
        ]
    }

    fn chosen(name: &str) -> LabelChoice {
        LabelChoice::Column(name.into())
    }

    fn resolved(choice: &LabelChoice, columns: &[ColumnInfo]) -> Resolved {
        resolve(choice, Dialect::Sqlite, columns, "id")
    }

    #[test]
    fn the_apps_pick_is_the_first_text_column_that_is_not_the_key() {
        // As the structure names PostgreSQL's types, which is how
        // `column_class` reads them.
        let columns = [
            column("code", "text"),
            column("founded", "integer"),
            column("name", "character varying(64)"),
            column("email", "text"),
        ];
        assert_eq!(
            automatic(Dialect::Postgres, &columns, "code"),
            Some("name".into())
        );
        assert_eq!(
            automatic(Dialect::Postgres, &columns, "founded"),
            Some("code".into())
        );
        assert_eq!(automatic(Dialect::Postgres, &columns[1..2], "id"), None);
    }

    #[test]
    fn automatic_resolves_to_the_apps_pick() {
        assert_eq!(
            resolved(&LabelChoice::Automatic, &genres()),
            Resolved {
                column: Some("name".into()),
                source: Source::Automatic,
                automatic: Some("name".into()),
                gone: None,
            }
        );
    }

    #[test]
    fn a_chosen_column_and_no_label_are_the_users() {
        let by_number = resolved(&chosen("parent_id"), &genres());
        assert_eq!(by_number.column.as_deref(), Some("parent_id"));
        assert_eq!(by_number.source, Source::User);
        // The chooser still names what Automatic would use.
        assert_eq!(by_number.automatic.as_deref(), Some("name"));
        let none = resolved(&LabelChoice::NoLabel, &genres());
        assert_eq!((none.column, none.source), (None, Source::User));
    }

    #[test]
    fn a_table_of_its_key_alone_has_nothing_to_choose() {
        let isbns = [column("isbn", "TEXT")];
        for choice in [LabelChoice::Automatic, LabelChoice::NoLabel, chosen("isbn")] {
            let resolved = resolve(&choice, Dialect::Sqlite, &isbns, "isbn");
            assert_eq!(
                (resolved.column, resolved.source),
                (None, Source::KeysOnly),
                "{choice:?}"
            );
        }
        // So with a key and bytes.
        let covers = [column("id", "INTEGER"), column("image", "BLOB")];
        let resolved = resolved(&LabelChoice::Automatic, &covers);
        assert_eq!(resolved.source, Source::KeysOnly);
    }

    #[test]
    fn a_table_with_no_text_column_is_listed_by_keys_until_one_is_chosen() {
        let shelves = [column("id", "INTEGER"), column("floor", "INTEGER")];
        let by_app = resolved(&LabelChoice::Automatic, &shelves);
        assert_eq!((by_app.column, by_app.source), (None, Source::Automatic));
        let by_floor = resolved(&chosen("floor"), &shelves);
        assert_eq!(by_floor.column.as_deref(), Some("floor"));
    }

    #[test]
    fn a_chosen_column_that_is_gone_falls_back_to_the_apps_pick() {
        let resolved = resolved(&chosen("legal_name"), &genres());
        assert_eq!(
            resolved,
            Resolved {
                column: Some("name".into()),
                source: Source::Automatic,
                automatic: Some("name".into()),
                gone: Some("legal_name".into()),
            }
        );
    }

    #[test]
    fn the_key_and_bytes_are_never_a_label() {
        for (dialect, binary) in [
            (Dialect::Postgres, "bytea"),
            (Dialect::MySql, "blob"),
            (Dialect::MySql, "binary(16)"),
            (Dialect::MySql, "varbinary(255)"),
            (Dialect::Sqlite, "BLOB"),
        ] {
            let cover = column("cover", binary);
            assert_eq!(
                excluded(dialect, &cover, &["id"]),
                Some(Excluded::Binary),
                "{binary}"
            );
        }
        let id = column("id", "INTEGER");
        assert_eq!(excluded(Dialect::Sqlite, &id, &["id"]), Some(Excluded::Key));
        assert_eq!(excluded(Dialect::Sqlite, &id, &["isbn"]), None);
        // A date and a number can be.
        for kind in ["date", "numeric(10,2)", "boolean", "jsonb"] {
            let other = column("other", kind);
            assert_eq!(excluded(Dialect::Postgres, &other, &["id"]), None, "{kind}");
        }
        // Chosen where another key references it: this picker lists keys,
        // and the choice stays the user's.
        let resolved = resolve(&chosen("name"), Dialect::Sqlite, &genres(), "name");
        assert_eq!((resolved.column, resolved.source), (None, Source::User));
    }

    fn publishers() -> ObjectRef {
        ObjectRef::new("public", "publishers")
    }

    #[test]
    fn a_choice_is_kept_by_connection_and_table() {
        let (dev, production) = (ConnectionId("dev".into()), ConnectionId("prod".into()));
        let mut labels = Labels::default();
        assert_eq!(labels.choice(&dev, &publishers()), LabelChoice::Automatic);
        assert!(labels.choose(&dev, &publishers(), chosen("name")));
        assert!(!labels.choose(&dev, &publishers(), chosen("name")));
        assert_eq!(labels.choice(&dev, &publishers()), chosen("name"));
        // Another saved connection, another table, another schema: not theirs.
        assert_eq!(
            labels.choice(&production, &publishers()),
            LabelChoice::Automatic
        );
        let authors = ObjectRef::new("public", "authors");
        assert_eq!(labels.choice(&dev, &authors), LabelChoice::Automatic);
        let archived = ObjectRef::new("archive", "publishers");
        assert_eq!(labels.choice(&dev, &archived), LabelChoice::Automatic);
    }

    #[test]
    fn automatic_is_kept_as_nothing() {
        let dev = ConnectionId("dev".into());
        let mut labels = Labels::default();
        labels.choose(&dev, &publishers(), LabelChoice::NoLabel);
        assert!(labels.choose(&dev, &publishers(), LabelChoice::Automatic));
        assert_eq!(labels, Labels::default());
    }

    #[test]
    fn a_lost_column_is_said_once() {
        let dev = ConnectionId("dev".into());
        let mut labels = Labels::default();
        labels.choose(&dev, &publishers(), chosen("legal_name"));
        labels.lost(&dev, &publishers(), "legal_name");
        assert_eq!(labels.choice(&dev, &publishers()), LabelChoice::Automatic);
        assert_eq!(labels.gone(&dev, &publishers()), Some("legal_name"));
        assert!(labels.told(&dev, &publishers()));
        assert!(!labels.told(&dev, &publishers()));
        assert_eq!(labels, Labels::default());
        // A new choice before it was said: there is nothing left to say.
        labels.lost(&dev, &publishers(), "legal_name");
        labels.choose(&dev, &publishers(), chosen("name"));
        assert_eq!(labels.gone(&dev, &publishers()), None);
    }

    #[test]
    fn a_deleted_connections_choices_are_forgotten() {
        let (dev, production) = (ConnectionId("dev".into()), ConnectionId("prod".into()));
        let mut labels = Labels::default();
        labels.choose(&dev, &publishers(), chosen("name"));
        labels.choose(&production, &publishers(), LabelChoice::NoLabel);
        assert!(labels.forget(&dev));
        assert!(!labels.forget(&dev));
        assert_eq!(labels.choice(&dev, &publishers()), LabelChoice::Automatic);
        assert_eq!(
            labels.choice(&production, &publishers()),
            LabelChoice::NoLabel
        );
    }

    #[test]
    fn the_file_reads_as_it_was_written_and_a_person_can_read_it() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("labels.json");
        assert_eq!(Labels::load(&path), Labels::default());
        let dev = ConnectionId("dev".into());
        let mut labels = Labels::default();
        labels.choose(&dev, &publishers(), chosen("name"));
        labels.choose(
            &dev,
            &ObjectRef::new("public", "authors"),
            LabelChoice::NoLabel,
        );
        labels.lost(&dev, &ObjectRef::new("public", "genres"), "title");
        labels.save(&path).unwrap();
        assert_eq!(Labels::load(&path), labels);
        let written: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        assert_eq!(
            written,
            serde_json::json!({
                "version": 1,
                "connections": { "dev": { "public": {
                    "authors": { "choice": "no-label" },
                    "genres": { "gone": "title" },
                    "publishers": { "choice": { "column": "name" } },
                } } },
            })
        );
        // A file with less in it reads: every part has its default.
        std::fs::write(
            &path,
            br#"{"connections":{"dev":{"public":{"genres":{}}}}}"#,
        )
        .unwrap();
        let read = Labels::load(&path);
        assert_eq!(
            read.choice(&dev, &ObjectRef::new("public", "genres")),
            LabelChoice::Automatic
        );
    }
}
```

- [ ] **Step 3: Run them to see them fail**

Run: `~/.cargo/bin/cargo test --locked -p tabletist --lib labels::`
Expected: it does not compile. `LabelChoice`, `Resolved`, `Source`, `Excluded`, `Labels`, `resolve`, `automatic` and `excluded` are not found in the module.

- [ ] **Step 4: Write the module**

Put this above the tests in `src/labels.rs`:

```rust
//! The column that labels the rows of a table foreign keys point at: what
//! the user chose, kept on this computer, and what the choice comes to
//! against the table's columns. No UI, and nothing here talks to the
//! backend or to a database.

use std::collections::BTreeMap;
use std::path::Path;

use serde::{Deserialize, Serialize};
use tabletist_db::{ColumnClass, ColumnInfo, Dialect, ObjectRef, column_class};

use crate::connections::ConnectionId;

/// What labels a table's rows, as the user left it.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum LabelChoice {
    /// The app picks: see [`automatic`].
    #[default]
    Automatic,
    /// No label: the rows are listed by their keys alone.
    NoLabel,
    /// This column.
    Column(String),
}

impl LabelChoice {
    fn is_automatic(&self) -> bool {
        *self == Self::Automatic
    }
}

/// Who picked what a table's rows are labelled by.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    /// The app.
    Automatic,
    /// The user: a column, or no label.
    User,
    /// Nobody: the table has no column that could be a label.
    KeysOnly,
}

/// What a choice comes to against a table's columns.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Resolved {
    /// The column the rows are labelled and searched by. None: by their
    /// keys alone.
    pub column: Option<String>,
    pub source: Source,
    /// What the app picks for the table, whoever chose.
    pub automatic: Option<String>,
    /// The column that was chosen and that the table no longer has.
    pub gone: Option<String>,
}

/// Why a column cannot label its table's rows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Excluded {
    /// It is the key the rows are found by.
    Key,
    /// It holds bytes.
    Binary,
}

/// Why `column` cannot be a label, where it cannot. `keys` are the columns
/// the rows are found by: the one a foreign key references, or the
/// table's primary key.
pub fn excluded(dialect: Dialect, column: &ColumnInfo, keys: &[&str]) -> Option<Excluded> {
    if keys.contains(&column.name.as_str()) {
        return Some(Excluded::Key);
    }
    match column_class(dialect, &column.type_name) {
        ColumnClass::Binary => Some(Excluded::Binary),
        ColumnClass::Integer { .. }
        | ColumnClass::Decimal { .. }
        | ColumnClass::Float
        | ColumnClass::Text { .. }
        | ColumnClass::Boolean
        | ColumnClass::Json
        | ColumnClass::Other => None,
    }
}

/// The column the app picks to read a row of a table by: its first text
/// column that is not the key.
pub fn automatic(dialect: Dialect, columns: &[ColumnInfo], key: &str) -> Option<String> {
    columns
        .iter()
        .filter(|column| column.name != key)
        .find(|column| {
            matches!(
                column_class(dialect, &column.type_name),
                ColumnClass::Text { .. }
            )
        })
        .map(|column| column.name.clone())
}

/// What `choice` comes to for a table of `columns`, in a picker whose
/// foreign key references `key`. A column that was chosen and is not among
/// them is `gone`, and the app's pick stands in for it. A chosen column
/// that cannot be a label any more (it is this picker's key, or holds
/// bytes now) labels nothing: the choice is still the user's.
pub fn resolve(
    choice: &LabelChoice,
    dialect: Dialect,
    columns: &[ColumnInfo],
    key: &str,
) -> Resolved {
    let automatic = automatic(dialect, columns, key);
    let allowed = |column: &ColumnInfo| excluded(dialect, column, &[key]).is_none();
    let chosen = match choice {
        LabelChoice::Column(name) => Some(name),
        LabelChoice::Automatic | LabelChoice::NoLabel => None,
    };
    let gone = chosen
        .filter(|name| !columns.iter().any(|column| column.name == **name))
        .cloned();
    if !columns.iter().any(allowed) {
        return Resolved {
            column: None,
            source: Source::KeysOnly,
            automatic,
            gone,
        };
    }
    let (column, source) = match choice {
        LabelChoice::Automatic => (automatic.clone(), Source::Automatic),
        LabelChoice::NoLabel => (None, Source::User),
        LabelChoice::Column(_) if gone.is_some() => (automatic.clone(), Source::Automatic),
        LabelChoice::Column(name) => {
            let found = columns.iter().find(|column| column.name == *name);
            (
                found
                    .filter(|column| allowed(column))
                    .map(|column| column.name.clone()),
                Source::User,
            )
        }
    };
    Resolved {
        column,
        source,
        automatic,
        gone,
    }
}

/// The file's format.
pub const VERSION: u32 = 1;

/// What is kept of one table.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default)]
struct Kept {
    #[serde(skip_serializing_if = "LabelChoice::is_automatic")]
    choice: LabelChoice,
    /// A chosen column that was found gone, until the user is told. The
    /// choice went back to Automatic when it was noted.
    #[serde(skip_serializing_if = "Option::is_none")]
    gone: Option<String>,
}

/// Every choice, stored as `labels.json` in the state directory: by the
/// saved connection's id, then by schema, then by table. Nothing of it is
/// ever written to a database.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Labels {
    version: u32,
    connections: BTreeMap<String, BTreeMap<String, BTreeMap<String, Kept>>>,
}

impl Default for Labels {
    fn default() -> Self {
        Self {
            version: VERSION,
            connections: BTreeMap::new(),
        }
    }
}

impl Labels {
    /// The store at `path`. A missing file is an empty store; one that
    /// cannot be read is kept aside (`util::load_json`), and the store
    /// starts empty.
    pub fn load(path: &Path) -> Self {
        crate::util::load_json(path)
    }

    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        crate::util::save_json(path, self)
    }

    fn kept(&self, conn: &ConnectionId, table: &ObjectRef) -> Option<&Kept> {
        self.connections
            .get(&conn.0)?
            .get(&table.schema)?
            .get(&table.name)
    }

    fn kept_mut(&mut self, conn: &ConnectionId, table: &ObjectRef) -> &mut Kept {
        self.connections
            .entry(conn.0.clone())
            .or_default()
            .entry(table.schema.clone())
            .or_default()
            .entry(table.name.clone())
            .or_default()
    }

    /// Drops what is kept of `table` where it says nothing any more, and
    /// the schema and the connection it leaves empty.
    fn tidy(&mut self, conn: &ConnectionId, table: &ObjectRef) {
        let Some(schemas) = self.connections.get_mut(&conn.0) else {
            return;
        };
        if let Some(tables) = schemas.get_mut(&table.schema) {
            if tables.get(&table.name) == Some(&Kept::default()) {
                tables.remove(&table.name);
            }
            if tables.is_empty() {
                schemas.remove(&table.schema);
            }
        }
        if schemas.is_empty() {
            self.connections.remove(&conn.0);
        }
    }

    /// What was chosen for `table` on the saved connection `conn`.
    pub fn choice(&self, conn: &ConnectionId, table: &ObjectRef) -> LabelChoice {
        self.kept(conn, table)
            .map(|kept| kept.choice.clone())
            .unwrap_or_default()
    }

    /// Makes `choice` the choice for `table`. A column that was noted gone
    /// and not said yet is forgotten: the user chose again. Returns
    /// whether anything changed.
    pub fn choose(&mut self, conn: &ConnectionId, table: &ObjectRef, choice: LabelChoice) -> bool {
        let before = self.kept(conn, table).cloned().unwrap_or_default();
        let after = Kept { choice, gone: None };
        if before == after {
            return false;
        }
        *self.kept_mut(conn, table) = after;
        self.tidy(conn, table);
        true
    }

    /// The column chosen for `table` is not there any more: the choice
    /// goes back to Automatic, and `column` is kept to be said once.
    pub fn lost(&mut self, conn: &ConnectionId, table: &ObjectRef, column: &str) {
        *self.kept_mut(conn, table) = Kept {
            choice: LabelChoice::Automatic,
            gone: Some(column.to_owned()),
        };
    }

    /// The chosen column of `table` that was found gone, and that the user
    /// has not been told of.
    pub fn gone(&self, conn: &ConnectionId, table: &ObjectRef) -> Option<&str> {
        self.kept(conn, table)?.gone.as_deref()
    }

    /// The user was told. Returns whether there was anything to tell.
    pub fn told(&mut self, conn: &ConnectionId, table: &ObjectRef) -> bool {
        if self.gone(conn, table).is_none() {
            return false;
        }
        self.kept_mut(conn, table).gone = None;
        self.tidy(conn, table);
        true
    }

    /// Forgets every choice of a saved connection that was deleted.
    /// Returns whether there was any.
    pub fn forget(&mut self, conn: &ConnectionId) -> bool {
        self.connections.remove(&conn.0).is_some()
    }
}
```

- [ ] **Step 5: Run the tests**

Run: `~/.cargo/bin/cargo test --locked -p tabletist --lib labels::`
Expected: `test result: ok. 12 passed`.

- [ ] **Step 6: Run the four checks**

Expected: all pass.

- [ ] **Step 7: Commit**

```bash
git add src/labels.rs src/lib.rs
git commit -S -m "Keep which column labels a table's rows: the choice, the rule and the file

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 2: The picker is labelled by the kept choice

**Files:**
- Create: `src/app/labels.rs`, `src/ui/label_tests.rs`
- Modify: `src/paths.rs`, `src/backend.rs`, `src/app.rs`, `src/key_picker.rs`, `src/app/key_picker.rs`, `src/ui/key_picker.rs`, `src/ui/key_picker_tests.rs`, `src/ui/mod.rs`, `docs/_reference/settings-and-files.md`

In this run nothing in the app sets a choice. The tests keep one with `Labels::choose`, as the file holds it at a start.

- [ ] **Step 1: Write the failing tests**

Create `src/ui/label_tests.rs`:

```rust
//! The label of a foreign key's picker, checked against its spec
//! (`docs/superpowers/specs/2026-10-12-foreign-key-label-design.md`:
//! "test N" below is a test of its list), on the Bookshop's `book_covers`,
//! `publishers` and `genres`.
//!
//! Only behaviour, names and words are checked. No colour, size or
//! picture of the design is.

use egui::{Key, Modifiers};
use tabletist_db::ObjectRef;

use crate::backend::{Command, Event, RequestId, StateFile};
use crate::connections::ConnectionId;
use crate::key_picker::{Columns, Picker};
use crate::labels::{LabelChoice, Resolved, Source};
use crate::model::{Action, CellPos, ConnTabId, EditStart, Pane, TabId};
use crate::testing::{GENRES, Harness, PUBLISHERS, publishers_structure};
use crate::theme::Look;

/// `publisher_id` in `book_covers`, and `parent_id` in `genres`, by their
/// place in the page.
const KEY: usize = 1;

fn publishers() -> ObjectRef {
    ObjectRef::new("main", "publishers")
}

fn genres() -> ObjectRef {
    ObjectRef::new("main", "genres")
}

fn column(name: &str) -> LabelChoice {
    LabelChoice::Column(name.into())
}

/// What a picker is labelled by: the column, and who picked it.
fn labelled(column: Option<&str>, source: Source, automatic: Option<&str>) -> Resolved {
    Resolved {
        column: column.map(str::to_owned),
        source,
        automatic: automatic.map(str::to_owned),
        gone: None,
    }
}

/// The Bookshop's covers with their key to publishers, writable, the grid
/// holding the keyboard.
fn covers_in(look: Look) -> (Harness, ConnTabId, TabId) {
    let mut harness = Harness::new();
    harness.set_look(look);
    let (tab, id) = harness.keyed_covers();
    harness.app.workspace_mut(tab).unwrap().pane = Pane::Grid;
    harness.settle();
    (harness, tab, id)
}

fn connection(harness: &Harness, tab: ConnTabId) -> ConnectionId {
    harness.app.workspace(tab).unwrap().conn_id.clone()
}

/// Keeps `choice` for `table` on the saved connection of `tab`, as the
/// file holds it at a start.
fn keep(harness: &mut Harness, tab: ConnTabId, table: &ObjectRef, choice: LabelChoice) {
    let conn = connection(harness, tab);
    harness.app.labels.choose(&conn, table, choice);
}

/// Opens the grid's editor on the foreign key cell of the page's row
/// `row`, typed into, and answers nothing.
fn open(harness: &mut Harness, tab: ConnTabId, id: TabId, row: usize) {
    let cell = CellPos { row, col: KEY };
    let start = EditStart::Typed("h".into());
    harness.app.apply(Action::EditCell {
        tab,
        id,
        cell,
        start,
    });
}

/// Opens it as [`open`] does, and answers what a picker of `publishers`
/// asks: the table's columns, then its lookups.
fn edit(harness: &mut Harness, tab: ConnTabId, id: TabId, row: usize) {
    open(harness, tab, id, row);
    harness.answer_structure(publishers_structure());
    harness.answer_lookups(&PUBLISHERS);
}

fn picker(harness: &Harness, tab: ConnTabId, id: TabId) -> &Picker {
    let object = harness.app.workspace(tab).unwrap().object_tab(id).unwrap();
    let editor = object.edits.editor.as_ref().expect("an editor is open");
    editor.picker.as_ref().expect("the editor has a picker")
}

/// The label column each lookup sent so far names.
fn searched(harness: &Harness) -> Vec<Option<String>> {
    let sent = harness.lookups();
    sent.into_iter()
        .map(|(_, _, lookup)| lookup.label)
        .collect()
}

/// The label stores the app sent to be saved.
fn saved(harness: &Harness) -> Vec<crate::labels::Labels> {
    let path = harness.app.dirs.labels_file();
    harness
        .app
        .backend
        .sent
        .iter()
        .filter_map(|command| match command {
            Command::Save {
                path: to,
                file: StateFile::Labels(labels),
            } if *to == path => Some(labels.clone()),
            _ => None,
        })
        .collect()
}

/// Test 1: with nothing chosen the label is the app's pick, as it was
/// before there was a choice, and the picker says whose pick it is.
#[test]
fn automatic_is_the_apps_pick_and_the_picker_says_whose_it_is() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    edit(&mut harness, tab, id, 1);
    let by_app = labelled(Some("name"), Source::Automatic, Some("name"));
    assert!(picker(&harness, tab, id).label == Columns::Known(by_app.clone()));
    assert_eq!(harness.app.label_of(tab, &publishers(), "id"), Some(by_app));
    let name = Some("name".to_owned());
    assert!(!searched(&harness).is_empty());
    assert!(searched(&harness).iter().all(|label| *label == name));
    // Nothing was chosen: nothing is written.
    assert!(saved(&harness).is_empty());
}

/// Test 3, the grid's editor: a kept choice labels the picker, and the
/// picker says the user made it.
#[test]
fn a_kept_choice_labels_the_picker_as_the_users() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    keep(&mut harness, tab, &publishers(), column("name"));
    edit(&mut harness, tab, id, 1);
    let by_user = labelled(Some("name"), Source::User, Some("name"));
    assert!(picker(&harness, tab, id).label == Columns::Known(by_user));
}

/// Test 3: the choice is the table's, so the row panel's field and a new
/// row's cell are labelled by it as the grid's editor is.
#[test]
fn a_choice_labels_every_key_that_points_at_the_table() {
    let by_user = Columns::Known(labelled(None, Source::User, Some("name")));
    // The row panel's field.
    let (mut harness, tab, id) = covers_in(Look::macos());
    keep(&mut harness, tab, &publishers(), LabelChoice::NoLabel);
    let cell = CellPos { row: 0, col: KEY };
    harness.app.apply(Action::SelectCell { tab, id, cell });
    harness.app.apply(Action::FocusFields { tab, id });
    let start = EditStart::Replace(String::new());
    harness.app.apply(Action::EditField {
        tab,
        id,
        cell,
        start,
    });
    harness.answer_structure(publishers_structure());
    harness.answer_lookups(&PUBLISHERS);
    assert!(picker(&harness, tab, id).label == by_user);
    assert!(searched(&harness).iter().all(Option::is_none));
    // A new row's cell: adding the row opens the editor on it.
    let (mut harness, tab, id) = covers_in(Look::macos());
    keep(&mut harness, tab, &publishers(), LabelChoice::NoLabel);
    harness.press(Key::N, Modifiers::COMMAND);
    harness.settle();
    harness.answer_structure(publishers_structure());
    harness.answer_lookups(&PUBLISHERS);
    assert!(picker(&harness, tab, id).label == by_user);
    assert!(!searched(&harness).is_empty());
    assert!(searched(&harness).iter().all(Option::is_none));
}

/// Test 3: a choice is one saved connection's. Another connection's, and
/// another table's, change nothing here.
#[test]
fn a_choice_is_its_saved_connections_alone() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    let other = ConnectionId("another-connection".into());
    assert_ne!(other, connection(&harness, tab));
    let none = LabelChoice::NoLabel;
    harness
        .app
        .labels
        .choose(&other, &publishers(), none.clone());
    keep(&mut harness, tab, &genres(), none);
    edit(&mut harness, tab, id, 1);
    let by_app = labelled(Some("name"), Source::Automatic, Some("name"));
    assert!(picker(&harness, tab, id).label == Columns::Known(by_app));
}

/// A column that is no text can be the label: `genres` listed by the
/// number of each genre's parent. The search names that column.
#[test]
fn a_number_column_can_label_a_table() {
    let mut harness = Harness::new();
    harness.set_look(Look::macos());
    let (tab, id) = harness.genres();
    keep(&mut harness, tab, &genres(), column("parent_id"));
    // The table points at itself: its columns came with its own tab.
    open(&mut harness, tab, id, 1);
    harness.answer_lookups(&GENRES);
    let by_user = labelled(Some("parent_id"), Source::User, Some("name"));
    assert!(picker(&harness, tab, id).label == Columns::Known(by_user));
    let parent = Some("parent_id".to_owned());
    assert!(!searched(&harness).is_empty());
    assert!(searched(&harness).iter().all(|label| *label == parent));
}

/// Test 9, what is kept: a chosen column the table no longer has is found
/// when its columns are read. The choice goes back to Automatic, the
/// picker is labelled by the app's pick, and the column's name is kept to
/// be said once. (Saying it is run 2's.)
#[test]
fn a_chosen_column_that_is_gone_goes_back_to_automatic() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    // `publishers` has no such column: it was renamed since it was chosen.
    keep(&mut harness, tab, &publishers(), column("legal_name"));
    edit(&mut harness, tab, id, 1);
    let by_app = labelled(Some("name"), Source::Automatic, Some("name"));
    assert!(picker(&harness, tab, id).label == Columns::Known(by_app));
    let name = Some("name".to_owned());
    assert!(searched(&harness).iter().all(|label| *label == name));
    let conn = connection(&harness, tab);
    let labels = &harness.app.labels;
    assert_eq!(labels.choice(&conn, &publishers()), LabelChoice::Automatic);
    assert_eq!(labels.gone(&conn, &publishers()), Some("legal_name"));
    // Written once, with the choice dropped.
    assert_eq!(saved(&harness), [harness.app.labels.clone()]);
}

/// A column is called gone only by columns that are fresh: while the
/// table's columns are being read again the picker waits, and the choice
/// stays until their answer says the column is not there.
#[test]
fn a_column_is_not_called_gone_while_the_columns_are_being_read_again() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    edit(&mut harness, tab, id, 1);
    harness.app.apply(Action::CancelEdit { tab, id });
    keep(&mut harness, tab, &publishers(), column("legal_name"));
    // A refresh of the columns is on its way.
    let request = RequestId(u64::MAX);
    let workspace = harness.app.workspace_mut(tab).unwrap();
    let session = workspace.session;
    workspace
        .columns
        .get_mut(&publishers())
        .unwrap()
        .start(request);
    let sent = harness.lookups().len();
    open(&mut harness, tab, id, 1);
    assert!(picker(&harness, tab, id).label == Columns::Waiting);
    assert_eq!(harness.lookups().len(), sent);
    let conn = connection(&harness, tab);
    assert_eq!(
        harness.app.labels.choice(&conn, &publishers()),
        column("legal_name")
    );
    assert!(saved(&harness).is_empty());
    // The answer: the column is not among them.
    harness.app.apply(Action::Backend(Event::Structure {
        session,
        request,
        result: Ok(publishers_structure()),
    }));
    let by_app = labelled(Some("name"), Source::Automatic, Some("name"));
    assert!(picker(&harness, tab, id).label == Columns::Known(by_app));
    assert!(harness.lookups().len() > sent);
    assert_eq!(
        harness.app.labels.gone(&conn, &publishers()),
        Some("legal_name")
    );
}

/// Nor by a read that failed: the columns it leaves are the ones from
/// before it.
#[test]
fn a_read_that_failed_calls_no_column_gone() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    edit(&mut harness, tab, id, 1);
    harness.app.apply(Action::CancelEdit { tab, id });
    keep(&mut harness, tab, &publishers(), column("legal_name"));
    let request = RequestId(u64::MAX);
    let workspace = harness.app.workspace_mut(tab).unwrap();
    let session = workspace.session;
    workspace
        .columns
        .get_mut(&publishers())
        .unwrap()
        .start(request);
    harness.app.apply(Action::Backend(Event::Structure {
        session,
        request,
        result: Err(tabletist_db::Error::query("the read was refused")),
    }));
    let conn = connection(&harness, tab);
    assert_eq!(
        harness.app.labels.choice(&conn, &publishers()),
        column("legal_name")
    );
    assert!(saved(&harness).is_empty());
}

/// A saved connection that is deleted takes its choices with it.
#[test]
fn deleting_a_connection_forgets_its_choices() {
    let (mut harness, tab, _) = covers_in(Look::macos());
    keep(&mut harness, tab, &publishers(), column("name"));
    let conn = connection(&harness, tab);
    harness.app.apply(Action::DeleteConnection(conn.clone()));
    assert_eq!(
        harness.app.labels.choice(&conn, &publishers()),
        LabelChoice::Automatic
    );
    assert_eq!(saved(&harness), [crate::labels::Labels::default()]);
}
```

Register it in `src/ui/mod.rs`, after `pub mod keys;`:

```rust
#[cfg(test)]
mod label_tests;
```

- [ ] **Step 2: Run them to see them fail**

Run: `~/.cargo/bin/cargo test --locked -p tabletist --lib label_tests`
Expected: it does not compile. `StateFile::Labels`, `App::labels`, `App::label_of`, `AppDirs::labels_file` and `Columns::Known` holding a `Resolved` are not there.

- [ ] **Step 3: The file's place, and the backend that writes it**

```diff
diff --git a/src/backend.rs b/src/backend.rs
--- a/src/backend.rs
+++ b/src/backend.rs
@@ -192,6 +192,7 @@ pub enum StateFile {
     Connections(SavedConnections),
     KnownHosts(HostKeys),
     Settings(crate::settings::Settings),
+    Labels(crate::labels::Labels),
 }
 
 impl StateFile {
@@ -200,6 +201,7 @@ impl StateFile {
             Self::Connections(connections) => connections.save(path),
             Self::KnownHosts(keys) => crate::known_hosts::save(path, keys),
             Self::Settings(settings) => settings.save(path),
+            Self::Labels(labels) => labels.save(path),
         }
     }
 }
diff --git a/src/paths.rs b/src/paths.rs
--- a/src/paths.rs
+++ b/src/paths.rs
@@ -9,7 +9,7 @@ use directories::{BaseDirs, ProjectDirs};
 pub struct AppDirs {
     /// Settings, saved connections, trusted SSH hosts, themes.
     pub config: PathBuf,
-    /// Logs.
+    /// Logs, and the label columns chosen for foreign keys.
     pub state: PathBuf,
     /// The user's home directory, where there is one: a path under it is
     /// shown with `~`. Found with the others, at the start: without `HOME`
@@ -81,6 +81,12 @@ impl AppDirs {
         self.config.join("themes")
     }
 
+    /// The columns chosen to label the rows foreign keys point at. Of
+    /// this computer, and no setting: it is kept with the state.
+    pub fn labels_file(&self) -> PathBuf {
+        self.state.join("labels.json")
+    }
+
     pub fn log_file(&self) -> PathBuf {
         self.state.join("tabletist.log")
     }
@@ -114,6 +120,7 @@ mod tests {
             PathBuf::from("/root/config/known_hosts.json")
         );
         assert_eq!(dirs.themes_dir(), PathBuf::from("/root/config/themes"));
+        assert_eq!(dirs.labels_file(), PathBuf::from("/root/state/labels.json"));
         assert_eq!(dirs.log_file(), PathBuf::from("/root/state/tabletist.log"));
         assert_eq!(dirs.panic_log(), PathBuf::from("/root/state/panic.log"));
     }
```

- [ ] **Step 4: The reducer's part**

Create `src/app/labels.rs`:

```rust
//! The reducer's part in the label of a table that foreign keys point at:
//! what the stored choice comes to for a workspace's table, and what
//! happens to a choice whose column is gone. The choice and the rule are
//! in `crate::labels`. Nothing else reads the store.

use tabletist_db::{ColumnInfo, Dialect, ObjectRef};

use super::App;
use crate::connections::ConnectionId;
use crate::labels::{self, LabelChoice, Labels, Resolved};
use crate::model::{ConnTabId, Fetch};

/// What labels the rows of `table`, whose columns the workspace of the
/// saved connection `conn` keeps as `kept`, in a picker whose foreign key
/// references `key`. None while the columns are not known. None too while
/// a chosen column is missing from columns that are being read again: it
/// is called gone only by columns that are fresh.
pub(super) fn resolved(
    labels: &Labels,
    (conn, dialect): (&ConnectionId, Dialect),
    kept: &Fetch<Vec<ColumnInfo>>,
    table: &ObjectRef,
    key: &str,
) -> Option<Resolved> {
    let columns = kept.value.as_deref()?;
    let choice = labels.choice(conn, table);
    let resolved = labels::resolve(&choice, dialect, columns, key);
    if resolved.gone.is_some() && kept.is_loading() {
        return None;
    }
    Some(resolved)
}

impl App {
    /// What labels the rows of `table` for the foreign keys of `tab` that
    /// reference its column `key`: the one resolver every view reads.
    pub fn label_of(&self, tab: ConnTabId, table: &ObjectRef, key: &str) -> Option<Resolved> {
        let workspace = self.workspace(tab)?;
        let of = (&workspace.conn_id, workspace.driver.dialect());
        resolved(&self.labels, of, workspace.columns.get(table)?, table, key)
    }

    /// The columns of `table` were just read. A column chosen as its label
    /// that is not among them is gone: the choice goes back to Automatic,
    /// and the column's name is kept to be said once.
    pub(super) fn check_label(&mut self, tab: ConnTabId, table: &ObjectRef) {
        let Some(workspace) = self.workspace(tab) else {
            return;
        };
        let fresh = workspace
            .columns
            .get(table)
            .filter(|kept| !kept.is_loading())
            .and_then(|kept| kept.value.as_deref());
        let Some(columns) = fresh else {
            return;
        };
        let conn = workspace.conn_id.clone();
        let LabelChoice::Column(name) = self.labels.choice(&conn, table) else {
            return;
        };
        if columns.iter().any(|column| column.name == name) {
            return;
        }
        self.labels.lost(&conn, table, &name);
        self.save_labels();
    }
}
```

- [ ] **Step 5: The app holds the store, saves it, and checks the columns that arrive**

`read` is the table whose columns an `Event::Structure` brought, from either of its two branches, and none where the read failed. `check_label` runs before `picker_columns`, so a picker that waited is labelled by what is left.

```diff
diff --git a/src/app.rs b/src/app.rs
--- a/src/app.rs
+++ b/src/app.rs
@@ -27,6 +27,7 @@ use crate::theme::{self, Catalog, Palette};
 
 mod editing;
 mod key_picker;
+mod labels;
 
 /// What a keyring read is for; each names the exact request it serves, so
 /// a late answer for an earlier attempt is dropped.
@@ -82,6 +83,9 @@ pub struct App {
     pub actions: Vec<Action>,
     pub backend: Backend,
     pub connections: SavedConnections,
+    /// The label columns chosen for the tables foreign keys point at, from
+    /// labels.json. Read through `App::label_of` only.
+    pub labels: crate::labels::Labels,
     pub dialog: Option<Dialog>,
     /// Keyring reads in flight, and what each is for.
     pending_secrets: HashMap<RequestId, SecretPurpose>,
@@ -135,6 +139,7 @@ impl App {
                 (HostKeys::default(), Some(problem))
             }
         };
+        let labels = crate::labels::Labels::load(&dirs.labels_file());
         let mut app = Self {
             dirs,
             settings,
@@ -149,6 +154,7 @@ impl App {
             actions: Vec::new(),
             backend,
             connections,
+            labels,
             dialog: None,
             pending_secrets: HashMap::new(),
             pending_stores: HashMap::new(),
@@ -1671,6 +1677,11 @@ impl App {
             Action::DeleteConnection(id) => {
                 if let Some(removed) = self.connections.remove(&id) {
                     self.save_connections();
+                    // Its label choices go with it. A tab that is still
+                    // open on it falls back to Automatic.
+                    if self.labels.forget(&id) {
+                        self.save_labels();
+                    }
                     if removed.password == PasswordMode::Keyring {
                         self.store_secret(&id, false, None, false);
                     }
@@ -2503,6 +2514,8 @@ impl App {
                 let Some(workspace) = self.workspace_mut(tab) else {
                     return;
                 };
+                // The table whose columns this answer brought, if any.
+                let mut read = None;
                 let waiting = workspace
                     .object_tabs_mut()
                     .find(|o| o.structure.pending == Some(request));
@@ -2517,23 +2530,33 @@ impl App {
                     // one, its key and its types: it is made again.
                     object.edits.review = None;
                     if let Some(columns) = columns {
-                        let kept = workspace.columns.entry(named).or_default();
+                        let kept = workspace.columns.entry(named.clone()).or_default();
                         // One being fetched gets its own answer.
                         if !kept.is_loading() {
                             kept.value = Some(columns);
                             kept.error = None;
+                            read = Some(named);
                         }
                         workspace.catalog_changed();
                     }
-                } else if let Some(kept) = workspace
+                } else if let Some((named, kept)) = workspace
                     .columns
-                    .values_mut()
-                    .find(|kept| kept.pending == Some(request))
+                    .iter_mut()
+                    .find(|(_, kept)| kept.pending == Some(request))
                 {
-                    // Asked for by a completion list.
+                    // Asked for by a completion list. A read that failed
+                    // leaves the columns it had: they are not fresh.
+                    let fresh = result.is_ok();
                     kept.finish(request, result.map(|structure| structure.columns));
+                    read = fresh.then(|| named.clone());
                     workspace.catalog_changed();
                 }
+                // A column chosen as the table's label may be gone from
+                // the columns just read. Before the pickers: they label
+                // by what is left.
+                if let Some(table) = read {
+                    self.check_label(tab, &table);
+                }
                 // A picker that waited for these columns learns them.
                 self.picker_columns(tab);
             }
@@ -2822,6 +2845,14 @@ impl App {
         self.save_connections();
     }
 
+    /// Saves the label choices on the backend, as the connections are.
+    pub(crate) fn save_labels(&mut self) {
+        self.backend.send(Command::Save {
+            path: self.dirs.labels_file(),
+            file: StateFile::Labels(self.labels.clone()),
+        });
+    }
+
     /// Saves the connections on the backend (writing syncs the disk, which
     /// can stall a frame).
     fn save_connections(&mut self) {
```

- [ ] **Step 6: The picker keeps what the resolver answered**

`label_column` and its test go: they are `labels::automatic` and its test now, moved word for word in task 1.

```diff
diff --git a/src/key_picker.rs b/src/key_picker.rs
--- a/src/key_picker.rs
+++ b/src/key_picker.rs
@@ -6,12 +6,13 @@ use std::ops::Range;
 use std::time::{Duration, Instant};
 
 use tabletist_db::{
-    ColumnClass, ColumnInfo, Dialect, Error, Lookup, LookupPage, LookupPart, LookupRow, Matched,
-    NewValue, ObjectRef, Structure, Value, column_class,
+    ColumnClass, Error, Lookup, LookupPage, LookupPart, LookupRow, Matched, NewValue, ObjectRef,
+    Structure, Value,
 };
 
 use crate::backend::RequestId;
 use crate::edit::{DoubtState, Edits, State, Table, new_id};
+use crate::labels::Resolved;
 use crate::model::Advance;
 
 /// How many matches a list holds; more are said, not paged.
@@ -113,21 +114,6 @@ pub fn unmatched(table: &Table<'_>, own: &ObjectRef, edits: &Edits) -> Vec<Unmat
         .collect()
 }
 
-/// The column a person reads a row of the referenced table by: its first
-/// text column that is not the key.
-pub fn label_column(dialect: Dialect, columns: &[ColumnInfo], key: &str) -> Option<String> {
-    columns
-        .iter()
-        .filter(|column| column.name != key)
-        .find(|column| {
-            matches!(
-                column_class(dialect, &column.type_name),
-                ColumnClass::Text { .. }
-            )
-        })
-        .map(|column| column.name.clone())
-}
-
 /// A UUID as it is written: 36 characters with their hyphens, or 32 hex
 /// digits.
 fn is_uuid(text: &str) -> bool {
@@ -246,8 +232,20 @@ pub fn found_in(label: &str, text: &str) -> Option<Range<usize>> {
 pub enum Columns {
     /// Its columns are being fetched.
     Waiting,
-    /// The label column, or none: the table is searched by its key alone.
-    Known(Option<String>),
+    /// What labels its rows, and who picked it (`labels::resolve`). With
+    /// no column the table is searched by its key alone.
+    Known(Resolved),
+}
+
+impl Columns {
+    /// The column the options are labelled by, once it is known and where
+    /// there is one.
+    pub fn column(&self) -> Option<&str> {
+        match self {
+            Self::Waiting => None,
+            Self::Known(label) => label.column.as_deref(),
+        }
+    }
 }
 
 /// The search that was last sent.
@@ -617,6 +615,8 @@ mod tests {
 
     use std::collections::BTreeSet;
 
+    use tabletist_db::{ColumnInfo, Dialect};
+
     use crate::edit::{Doubt, DoubtState, Edits, NEW_ROWS, Pending, State, Table};
     use tabletist_db::{Access, ColumnMeta, ForeignKeyInfo, ObjectKind, RowPage, ValueKind};
 
@@ -638,7 +638,12 @@ mod tests {
     fn picker(searching: bool, text: &str) -> Picker {
         Picker::new(
             (ObjectRef::new("main", "publishers"), "id".into()),
-            Columns::Known(Some("name".into())),
+            Columns::Known(crate::labels::Resolved {
+                column: Some("name".into()),
+                source: crate::labels::Source::Automatic,
+                automatic: Some("name".into()),
+                gone: None,
+            }),
             None,
             searching,
             text,
@@ -691,32 +696,6 @@ mod tests {
         assert_eq!(target(&structure, "kind"), None);
     }
 
-    #[test]
-    fn the_label_is_the_first_text_column_that_is_not_the_key() {
-        let column = |name: &str, type_name: &str| ColumnInfo {
-            name: name.into(),
-            type_name: type_name.into(),
-            ..ColumnInfo::default()
-        };
-        // As the structure names PostgreSQL's types, which is how
-        // `column_class` reads them.
-        let columns = [
-            column("code", "text"),
-            column("founded", "integer"),
-            column("name", "character varying(64)"),
-            column("email", "text"),
-        ];
-        assert_eq!(
-            label_column(Dialect::Postgres, &columns, "code"),
-            Some("name".to_owned())
-        );
-        assert_eq!(
-            label_column(Dialect::Postgres, &columns, "founded"),
-            Some("code".to_owned())
-        );
-        assert_eq!(label_column(Dialect::Postgres, &columns[1..2], "id"), None);
-    }
-
     #[test]
     fn a_text_is_pinned_only_where_it_reads_as_the_keys_type() {
         let whole = ColumnClass::Integer {
```

```diff
diff --git a/src/app/key_picker.rs b/src/app/key_picker.rs
--- a/src/app/key_picker.rs
+++ b/src/app/key_picker.rs
@@ -81,12 +81,14 @@ impl App {
                 }),
             };
             let kept = workspace.columns.get(&target.0);
-            let label = match kept.and_then(|kept| kept.value.as_deref()) {
-                Some(columns) => {
-                    Columns::Known(key_picker::label_column(table.dialect, columns, &target.1))
-                }
-                None => Columns::Waiting,
-            };
+            // What labels the table's rows: the user's choice, or the
+            // app's pick.
+            let of = (&workspace.conn_id, table.dialect);
+            let label = kept
+                .and_then(|kept| {
+                    super::labels::resolved(&self.labels, of, kept, &target.0, &target.1)
+                })
+                .map_or(Columns::Waiting, Columns::Known);
             // A fetch that failed is asked for again: the completion list
             // asks once per keystroke and so lets a failure stand, while
             // this asks once per editor opened. A cached error is never
@@ -143,7 +145,7 @@ impl App {
                 let passes = crate::edit::problem(table.dialect, column, key).is_none();
                 key_picker::pinned(key, class, &column.type_name, passes)
             };
-            let reads = picker.lookups(label.clone(), &text, pin);
+            let reads = picker.lookups(label.column.clone(), &text, pin);
             Some((workspace.session, reads, text, picker.serial))
         });
         let Some((session, (pinned, matching), text, serial)) = planned else {
@@ -191,10 +193,14 @@ impl App {
     /// search, or says why it has no list.
     pub(super) fn picker_columns(&mut self, tab: ConnTabId) {
         let shown = self.active_workspace_tab();
+        // Read beside the workspace, which is borrowed to be changed: a
+        // line for each table a column was chosen for.
+        let labels = self.labels.clone();
         let Some(workspace) = self.workspace_mut(tab) else {
             return;
         };
         let dialect = workspace.driver.dialect();
+        let conn = workspace.conn_id.clone();
         let mut ready = Vec::new();
         let mut commits = Vec::new();
         // Split, so the tabs and the cache are borrowed side by side.
@@ -213,10 +219,15 @@ impl App {
             let Some(kept) = columns.get(&picker.target) else {
                 continue;
             };
-            if let Some(known) = kept.value.as_deref() {
-                picker.label =
-                    Columns::Known(key_picker::label_column(dialect, known, &picker.key));
-                ready.push(id);
+            if kept.value.is_some() {
+                let of = (&conn, dialect);
+                let label = super::labels::resolved(&labels, of, kept, &picker.target, &picker.key);
+                // None: its chosen column is missing from columns that
+                // are being read again. Their answer comes here too.
+                if let Some(label) = label {
+                    picker.label = Columns::Known(label);
+                    ready.push(id);
+                }
             } else if let Some(error) = &kept.error {
                 picker.failed = Some(error.clone());
                 // Nothing will answer: a kept Enter takes the text. Under
```

`picker_columns` reads a copy of the store: the workspace is borrowed to be changed there, and the store is a line for each table a column was chosen for.

- [ ] **Step 7: The view and the older test read the column through `Columns::column`**

```diff
diff --git a/src/ui/key_picker.rs b/src/ui/key_picker.rs
--- a/src/ui/key_picker.rs
+++ b/src/ui/key_picker.rs
@@ -10,7 +10,7 @@ use egui::{CornerRadius, Id, Rect, Sense, Ui, UiBuilder, WidgetInfo, WidgetType,
 use tabletist_db::{LookupRow, Value};
 
 use crate::i18n::{Locale, gettext, ngettext};
-use crate::key_picker::{Columns, LIMIT, Picker, VISIBLE, found_in};
+use crate::key_picker::{LIMIT, Picker, VISIBLE, found_in};
 use crate::theme::{Icon, Look, Palette};
 use crate::typography::{Text, TextRole};
 use crate::ui::format::{display_safe, plain_text};
@@ -418,7 +418,7 @@ pub fn show(
     names: &Names<'_>,
     (look, palette, locale): (&Look, &Palette, Locale),
 ) -> Shown {
-    let labelled = matches!(&picker.label, Columns::Known(Some(_)));
+    let labelled = picker.label.column().is_some();
     let shape = Shape::of(look, labelled);
     let total = picker.options.len();
     // Whether there is a state line is decided before the list is placed,
diff --git a/src/ui/key_picker_tests.rs b/src/ui/key_picker_tests.rs
--- a/src/ui/key_picker_tests.rs
+++ b/src/ui/key_picker_tests.rs
@@ -14,7 +14,7 @@ use tabletist_db::{LookupPart, NewValue, ObjectRef, Value};
 
 use crate::backend::{Command, Event, RequestId};
 use crate::edit::{Doubt, DoubtState};
-use crate::key_picker::{Columns, Picker, Take};
+use crate::key_picker::{Picker, Take};
 use crate::model::{Action, Advance, CellPos, ConnTabId, EditStart, Pane, TabId};
 use crate::testing::{Harness, PUBLISHERS, publishers_structure};
 use crate::theme::Look;
@@ -596,7 +596,7 @@ fn a_failed_refresh_of_known_columns_does_not_take_the_list_away() {
     assert_eq!(publishers_described(&harness), described);
     let opened = picker(&harness, tab, id).unwrap();
     assert!(opened.failed.is_none());
-    assert!(matches!(opened.label, Columns::Known(Some(_))));
+    assert_eq!(opened.label.column(), Some("name"));
     assert_eq!(harness.lookups().len(), sent + 1);
     harness.answer_lookups(&PUBLISHERS);
     assert!(picker(&harness, tab, id).unwrap().failed.is_none());
```

- [ ] **Step 8: The website's list of files**

`AGENTS.md`: `_reference/settings-and-files.md` follows `src/paths.rs`.

```diff
diff --git a/docs/_reference/settings-and-files.md b/docs/_reference/settings-and-files.md
--- a/docs/_reference/settings-and-files.md
+++ b/docs/_reference/settings-and-files.md
@@ -79,6 +79,7 @@ On Linux, `XDG_CONFIG_HOME` and `XDG_STATE_HOME` move the two folders.
 | `connections.json` | Settings | Your saved connections. It never holds a password |
 | `known_hosts.json` | Settings | The SSH host keys you trusted |
 | `themes/` | Settings | Palette files |
+| `labels.json` | State | The column you chose to label each table's rows in a foreign key's list, by saved connection |
 | `tabletist.log`, `panic.log` | State | Logs |
 
 Passwords are kept in the system keyring, never in these files.
```

- [ ] **Step 9: Run the tests**

Run: `~/.cargo/bin/cargo fmt --all && ~/.cargo/bin/cargo test --locked -p tabletist --lib`
Expected: `test result: ok. 2187 passed; 0 failed; 20 ignored` (2167 before this plan, 12 more with task 1, 9 more here, and one less for the test that moved; the count moves with `main`).

- [ ] **Step 10: Run the four checks, and clippy with `--features shots`**

Expected: all pass. If the website's tools are installed, also `cd docs && bin/check`.

- [ ] **Step 11: Commit**

```bash
git add src/app/labels.rs src/ui/label_tests.rs src/paths.rs src/backend.rs src/app.rs \
  src/key_picker.rs src/app/key_picker.rs src/ui/key_picker.rs src/ui/key_picker_tests.rs \
  src/ui/mod.rs docs/_reference/settings-and-files.md
git commit -S -m "Label a foreign key's list by the column kept for its table

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 3: The picker says which column labels it, and who chose it

**Files:**
- Modify: `src/ui/key_picker.rs`, `src/ui/workspace.rs`
- Test: `src/ui/label_tests.rs`

The words are the notes board's. The macOS look says them in the list's foot, after `Esc cancels` and as words of their own: two older tests click the label `Esc cancels`, and run 2 makes the label's words a button. The Omarchy look's list has no foot, so its status line says them.

- [ ] **Step 1: Write the failing tests**

Append to `src/ui/label_tests.rs`:

```rust
/// The words that say what labels a picker, by what it resolved to: the
/// notes board's, and in the Omarchy look its lower case round a name
/// that is left as it is.
#[test]
fn the_label_is_worded_by_its_column_and_who_chose_it() {
    use super::key_picker::label_line;
    let locale = crate::i18n::Locale::default();
    let said = |look: Look, label: Resolved| {
        let (said, column) = label_line(&Columns::Known(label), &look, locale).unwrap();
        // Where the name stands is where it reads.
        (said.clone(), column.map(|at| said[at].to_owned()))
    };
    let named = |words: &str| (words.to_owned(), Some("Name".to_owned()));
    let plain = |words: &str| (words.to_owned(), None);
    let by_app = labelled(Some("Name"), Source::Automatic, Some("Name"));
    let by_user = labelled(Some("Name"), Source::User, None);
    let nothing = labelled(None, Source::KeysOnly, None);
    assert_eq!(
        said(Look::macos(), by_app.clone()),
        named("Label Name · Automatic")
    );
    assert_eq!(
        said(Look::macos(), by_user.clone()),
        named("Label Name · Set by you")
    );
    assert_eq!(
        said(Look::macos(), nothing.clone()),
        plain("Label keys only")
    );
    assert_eq!(
        said(Look::omarchy(), by_app),
        named("label Name · automatic")
    );
    assert_eq!(
        said(Look::omarchy(), by_user),
        named("label Name · set by you")
    );
    assert_eq!(said(Look::omarchy(), nothing), plain("label keys only"));
    // Two the board does not draw: no column, and something to choose.
    let unpicked = labelled(None, Source::Automatic, None);
    let unlabelled = labelled(None, Source::User, Some("Name"));
    assert_eq!(
        said(Look::macos(), unpicked),
        plain("Label keys only · Automatic")
    );
    assert_eq!(
        said(Look::macos(), unlabelled),
        plain("Label keys only · Set by you")
    );
    // Nothing is said while the columns are not known.
    assert!(label_line(&Columns::Waiting, &Look::macos(), locale).is_none());
}

/// The macOS look says it in the list's foot, after the foot's own words,
/// which stay as they were.
#[test]
fn the_foot_names_the_label_and_who_chose_it() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    edit(&mut harness, tab, id, 1);
    assert!(harness.has("Esc cancels"));
    assert!(harness.has("Label name · Automatic"));
    for (choice, said) in [
        (column("name"), "Label name · Set by you"),
        (LabelChoice::NoLabel, "Label keys only · Set by you"),
    ] {
        let (mut harness, tab, id) = covers_in(Look::macos());
        keep(&mut harness, tab, &publishers(), choice);
        edit(&mut harness, tab, id, 1);
        assert!(harness.has("Esc cancels"), "{said}");
        assert!(harness.has(said), "{said}");
        assert!(!harness.has("Label name · Automatic"), "{said}");
    }
}

/// The Omarchy look's list has no foot: the status line says it, in the
/// look's lower case.
#[test]
fn the_terminal_looks_status_line_names_the_label() {
    let (mut harness, tab, id) = covers_in(Look::omarchy());
    edit(&mut harness, tab, id, 1);
    assert!(harness.has("label name · automatic"));
    assert!(!harness.has("Esc cancels"));
    let (mut harness, tab, id) = covers_in(Look::omarchy());
    keep(&mut harness, tab, &publishers(), column("name"));
    edit(&mut harness, tab, id, 1);
    assert!(harness.has("label name · set by you"));
}
```

- [ ] **Step 2: Run them to see them fail**

Run: `~/.cargo/bin/cargo test --locked -p tabletist --lib label_tests`
Expected: it does not compile: `label_line` is not found in `ui::key_picker`.

- [ ] **Step 3: The words, and the foot**

```diff
diff --git a/src/ui/key_picker.rs b/src/ui/key_picker.rs
--- a/src/ui/key_picker.rs
+++ b/src/ui/key_picker.rs
@@ -10,7 +10,8 @@ use egui::{CornerRadius, Id, Rect, Sense, Ui, UiBuilder, WidgetInfo, WidgetType,
 use tabletist_db::{LookupRow, Value};
 
 use crate::i18n::{Locale, gettext, ngettext};
-use crate::key_picker::{LIMIT, Picker, VISIBLE, found_in};
+use crate::key_picker::{Columns, LIMIT, Picker, VISIBLE, found_in};
+use crate::labels::Source;
 use crate::theme::{Icon, Look, Palette};
 use crate::typography::{Text, TextRole};
 use crate::ui::format::{display_safe, plain_text};
@@ -183,6 +184,37 @@ fn head_line(
     }
 }
 
+/// What the picker says of its label column: "Label name · Automatic",
+/// and where in it the column's name stands. In the terminal look the
+/// app's own words are the look's lower case, and the column's name is as
+/// it is. None while the referenced table's columns are not known.
+pub fn label_line(
+    label: &Columns,
+    look: &Look,
+    locale: Locale,
+) -> Option<(String, Option<std::ops::Range<usize>>)> {
+    let Columns::Known(label) = label else {
+        return None;
+    };
+    let words = match (&label.column, label.source) {
+        (_, Source::KeysOnly) => "Label keys only",
+        (Some(_), Source::Automatic) => "Label {column} · Automatic",
+        (Some(_), Source::User) => "Label {column} · Set by you",
+        // The app found no column to read a row by, and one can be
+        // chosen; or the user chose none.
+        (None, Source::Automatic) => "Label keys only · Automatic",
+        (None, Source::User) => "Label keys only · Set by you",
+    };
+    let words = look.label(&gettext(locale, words));
+    let Some((column, at)) = label.column.as_deref().zip(words.find("{column}")) else {
+        return Some((words, None));
+    };
+    // A placeholder: a translation puts the name where it reads.
+    let column = display_safe(column);
+    let said = words.replacen("{column}", &column, 1);
+    Some((said, Some(at..at + column.len())))
+}
+
 /// A label as the text it is written in: a text is borrowed, any other
 /// value is written out.
 fn label_text(label: &Value) -> Cow<'_, str> {
@@ -343,6 +375,9 @@ pub fn doubts(
         .collect()
 }
 
+/// The least room the foot gives the words that name the label column.
+const LABEL_ROOM: f32 = 48.0;
+
 /// The size of the check that marks the row the cell points at.
 const CHECK: f32 = 12.0;
 
@@ -566,6 +601,7 @@ pub fn show(
             }
             if foot {
                 let y = top + shape.line / 2.0;
+                // The label column stands after the key that cancels.
                 let cancels = gettext(locale, "Esc cancels");
                 // Set NULL stands at the foot's right end, and the words
                 // end before it.
@@ -580,6 +616,23 @@ pub fn show(
                 let taken = button.as_ref().map_or(0.0, |(_, wide)| wide + 8.0);
                 let args = (&*cancels, line_role, muted);
                 say(ui, (left, y), room - taken, args, inner, look);
+                // What labels the options stands after it, as words of
+                // its own, cut to the room there is. With too little for
+                // more than a mark it is not drawn.
+                if let Some((label, _)) = label_line(&picker.label, look, locale) {
+                    let width = |text: &str| line_role.width(ui.ctx(), look.faces, text);
+                    let dot = " · ";
+                    let from = left + width(&cancels);
+                    let label_left = from + width(dot);
+                    let label_room = left + room - taken - label_left;
+                    if label_room >= LABEL_ROOM {
+                        Text::one(look, line_role, dot, muted)
+                            .layout(ui.ctx())
+                            .paint_left(&ui.painter().with_clip_rect(inner), from, y);
+                        let args = (&*label, line_role, muted);
+                        say(ui, (label_left, y), label_room, args, inner, look);
+                    }
+                }
                 if let Some((button, wide)) = button {
                     let place = Rect::from_center_size(
                         pos2(inner.right() - shape.inset - wide / 2.0, y),
```

- [ ] **Step 4: The Omarchy status line**

```diff
diff --git a/src/ui/workspace.rs b/src/ui/workspace.rs
--- a/src/ui/workspace.rs
+++ b/src/ui/workspace.rs
@@ -1582,6 +1582,9 @@ struct Editing {
     /// Insert mode is in a foreign key cell's picker: the line names the
     /// list's keys in place of the editor's.
     picking: bool,
+    /// What the picker's options are labelled by, and where in the words
+    /// the column's name stands: "label name · automatic".
+    label: Option<(String, Option<std::ops::Range<usize>>)>,
     /// How much is pending, while anything is: "3 pending · 2 rows".
     pending: Option<String>,
     /// The rows to add, while there are any: "+1 new".
@@ -1793,11 +1796,19 @@ fn editing_status(app: &App, tab: ConnTabId) -> Editing {
                 .as_ref()
                 .is_some_and(|picker| picker.failed.is_none())
     });
+    // The list has no foot in this look: the line says what labels it.
+    let label = edits
+        .editor
+        .as_ref()
+        .filter(|_| picking)
+        .and_then(|editor| editor.picker.as_ref())
+        .and_then(|picker| super::key_picker::label_line(&picker.label, look, locale));
     Editing {
         can_edit,
         insert,
         walks,
         picking,
+        label,
         pending,
         added,
         can_add,
@@ -2160,6 +2171,20 @@ fn status_line(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId) {
                         x += widgets::paint_label(ui, x, y, said()) + gap;
                     }
                 }
+                if let Some((said, column)) = &editing.label {
+                    // The column's name stands out of the words round it.
+                    // It gives way before the keys do.
+                    let at = column.clone().unwrap_or(said.len()..said.len());
+                    let label = || {
+                        Text::new(&look)
+                            .add(role, &said[..at.start], palette.dim)
+                            .add(role, &said[at.clone()], palette.text)
+                            .add(role, &said[at.end..], palette.dim)
+                    };
+                    if named && x + widgets::measure(ui, label()) + gap + keys_width <= right {
+                        x += widgets::paint_label(ui, x, y, label()) + gap;
+                    }
+                }
                 if named && x + keys_width <= right {
                     widgets::paint_text_right(ui, right, y, keys());
                 } else if named && editing.picking {
```

- [ ] **Step 5: Run the tests**

Run: `~/.cargo/bin/cargo fmt --all && ~/.cargo/bin/cargo test --locked -p tabletist --lib`
Expected: `test result: ok. 2190 passed; 0 failed; 20 ignored`.

- [ ] **Step 6: Look at it**

Add a throwaway test at the end of `src/shots.rs`, render, and read two of the pictures:

```rust
#[test]
#[ignore]
fn throwaway_label() {
    both("label-picker", edit_key_picker);
}
```

Run: `~/.cargo/bin/cargo test --locked --features shots --lib shots::throwaway_label -- --ignored`
Expected in `target/shots/label-picker-macos-light.png`: the foot reads `Esc cancels · Label title · Automatic`. In `label-picker-omarchy-dark.png`: the status line reads `-- INSERT --  book_id · int8  label title · automatic`, with `title` brighter than the words round it and the key hints at the right. Take the test out again and delete the pictures. Say in the task's report what you saw.

- [ ] **Step 7: Run the four checks, and clippy with `--features shots`**

Expected: all pass. `tests/keys.rs` passes: no key is spelled here.

- [ ] **Step 8: Commit**

```bash
git add src/ui/key_picker.rs src/ui/workspace.rs src/ui/label_tests.rs
git commit -S -m "Say in a foreign key's list which column labels it, and who chose it

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 4: A date or a number as a label, on the three engines

**Files:**
- Modify: `crates/tabletist-db/tests/lookup.rs`, `crates/tabletist-db/src/lookup.rs`

Spec test 14. The lookup already casts its label to text to match it (`Dialect::matches`), whatever the column's type, and answers the value as the driver reads it. So these tests pass as soon as they are written: they are the proof, and what keeps it so. There is no failing step.

The table is the Bookshop's `editions` as the screenshot scenes have it, with a page count, a price and a date. A decimal keeps its scale on PostgreSQL and MySQL (`12.50`, answered as text) and is a number on SQLite (`12.5`), so each engine is searched for the text it draws.

- [ ] **Step 1: Write the tests, and say in the type what a label may be**

```diff
diff --git a/crates/tabletist-db/src/lookup.rs b/crates/tabletist-db/src/lookup.rs
--- a/crates/tabletist-db/src/lookup.rs
+++ b/crates/tabletist-db/src/lookup.rs
@@ -12,9 +12,10 @@ pub struct Lookup {
     pub object: ObjectRef,
     /// The column the foreign key references.
     pub key: String,
-    /// The column a person reads a row by. `None`: the table has no text
-    /// column but its key, and rows are found by the key alone. Never the
-    /// key column: the statement names both in one select list.
+    /// The column a person reads a row by, of any type: it is searched as
+    /// the text the database writes it as. `None`: rows are found by the
+    /// key alone. Never the key column: the statement names both in one
+    /// select list.
     pub label: Option<String>,
     pub part: LookupPart,
     /// The most rows wanted. One more is read, to learn whether there are
diff --git a/crates/tabletist-db/tests/lookup.rs b/crates/tabletist-db/tests/lookup.rs
--- a/crates/tabletist-db/tests/lookup.rs
+++ b/crates/tabletist-db/tests/lookup.rs
@@ -141,6 +141,76 @@ async fn the_bookshop_is_searched(connection: &Connection, schema: &str, table:
     assert_eq!(page.rows.len(), 5, "{on}");
 }
 
+/// A search of `schema.table`, the Bookshop's editions, by its `id` and by
+/// the column `label`.
+fn editions(schema: &str, table: &str, label: &str, text: &str) -> Lookup {
+    Lookup {
+        object: ObjectRef::new(schema, table),
+        key: "id".into(),
+        label: Some(label.into()),
+        part: matching(text),
+        limit: 50,
+    }
+}
+
+/// A label that is no text is searched by the text the database writes it
+/// as, which is what the list draws
+/// (`docs/superpowers/specs/2026-10-12-foreign-key-label-design.md`, test
+/// 14). The editions are the three of each module's `EDITIONS`. `price` is
+/// the first edition's price as this engine answers it, and `drawn` the
+/// text it is drawn as: a decimal keeps its scale on PostgreSQL and MySQL
+/// and is a number on SQLite.
+async fn labels_that_are_no_text_are_searched_as_drawn(
+    connection: &Connection,
+    (schema, table): (&str, &str),
+    (price, drawn): (Value, &str),
+    on: &str,
+) {
+    let read = |label: &'static str, text: &str| {
+        let lookup = editions(schema, table, label, text);
+        async move {
+            let page = connection
+                .lookup(&lookup)
+                .await
+                .unwrap_or_else(|error| panic!("{on}: {error}"));
+            let rows = page.rows.into_iter();
+            rows.map(|row| (row.key, row.label, row.matched))
+                .collect::<Vec<_>>()
+        }
+    };
+    let date = |text: &str| Some(Value::Text(text.into()));
+    // A date, by a part of it and by the whole.
+    assert_eq!(
+        read("published_on", "1998").await,
+        [
+            (Value::Int(1), date("1998-04-01"), Matched::Label),
+            (Value::Int(3), date("1998-12-24"), Matched::Label),
+        ],
+        "{on}"
+    );
+    assert_eq!(
+        read("published_on", "1998-04-01").await,
+        [(Value::Int(1), date("1998-04-01"), Matched::Label)],
+        "{on}"
+    );
+    // A whole number.
+    assert_eq!(
+        read("pages", "212").await,
+        [(Value::Int(1), Some(Value::Int(212)), Matched::Label)],
+        "{on}"
+    );
+    // A decimal, by the text it is drawn as.
+    assert_eq!(
+        read("price", drawn).await,
+        [(Value::Int(1), Some(price), Matched::Label)],
+        "{on}"
+    );
+    // What no label holds finds nothing, and is no error on a column of
+    // dates or of numbers.
+    assert!(read("published_on", "harb").await.is_empty(), "{on}");
+    assert!(read("pages", "harb").await.is_empty(), "{on}");
+}
+
 mod sqlite {
     use super::*;
 
@@ -158,6 +228,19 @@ mod sqlite {
         CREATE TABLE shelves (id INTEGER PRIMARY KEY, floor INTEGER);
         INSERT INTO shelves VALUES (12, 1), (120, 1), (3, 2);";
 
+    const EDITIONS: &str = "
+        CREATE TABLE editions (
+            id INTEGER PRIMARY KEY,
+            title TEXT NOT NULL,
+            pages INTEGER,
+            price NUMERIC(10,2),
+            published_on DATE
+        );
+        INSERT INTO editions VALUES
+            (1, 'The Lighthouse', 212, 12.50, '1998-04-01'),
+            (2, 'Harbor Nights', 96, 30.00, '2004-11-30'),
+            (3, 'Low Tide', 304, 9.99, '1998-12-24');";
+
     /// A database of its own, made with `sql`.
     async fn made_with(sql: &str, access: Access) -> (Connection, tempfile::TempDir) {
         let dir = tempfile::tempdir().unwrap();
@@ -186,6 +269,18 @@ mod sqlite {
         the_bookshop_is_searched(&connection, "main", "publishers", "sqlite, Writable").await;
     }
 
+    #[tokio::test]
+    async fn labels_that_are_no_text_are_searched_as_drawn_on_sqlite() {
+        let (connection, _dir) = made_with(EDITIONS, Access::ReadOnly).await;
+        labels_that_are_no_text_are_searched_as_drawn(
+            &connection,
+            ("main", "editions"),
+            (Value::Float(12.5), "12.5"),
+            "sqlite",
+        )
+        .await;
+    }
+
     #[tokio::test]
     async fn a_table_with_no_label_is_searched_by_its_key() {
         let (connection, _dir) = made_with(CREATE, Access::ReadOnly).await;
@@ -383,11 +478,55 @@ mod postgres {
             (9100000000000000004, 'Harbor Press'),
             (9100000000000000007, 'Harbor Lights')";
 
+    const DROP_EDITIONS: &str = "DROP TABLE IF EXISTS lookup_editions";
+
+    const EDITIONS: &str = "
+        CREATE TABLE lookup_editions (
+            id bigint PRIMARY KEY,
+            title text NOT NULL,
+            pages integer,
+            price numeric(10,2),
+            published_on date
+        );
+        INSERT INTO lookup_editions VALUES
+            (1, 'The Lighthouse', 212, 12.50, '1998-04-01'),
+            (2, 'Harbor Nights', 96, 30.00, '2004-11-30'),
+            (3, 'Low Tide', 304, 9.99, '1998-12-24')";
+
     fn url() -> Option<String> {
         let url = std::env::var("TABLETIST_TEST_PG_URL").ok();
         url.filter(|url| !url.trim().is_empty())
     }
 
+    #[tokio::test]
+    async fn labels_that_are_no_text_are_searched_as_drawn_on_postgres() {
+        let Some(url) = url() else {
+            eprintln!("skipped: TABLETIST_TEST_PG_URL is not set");
+            return;
+        };
+        let mut config: tokio_postgres::Config = url.parse().unwrap();
+        config.ssl_mode(tokio_postgres::config::SslMode::Disable);
+        let (admin, session) = config.connect(tokio_postgres::NoTls).await.unwrap();
+        tokio::spawn(session);
+        admin.batch_execute(DROP_EDITIONS).await.unwrap();
+        admin.batch_execute(EDITIONS).await.unwrap();
+
+        let (mut spec, secrets) = ConnectSpec::from_url(&url).unwrap();
+        spec.tls = TlsMode::Disable;
+        let connection =
+            Connection::connect_with(&spec, &secrets, &HostKeys::default(), Access::ReadOnly)
+                .await
+                .unwrap();
+        labels_that_are_no_text_are_searched_as_drawn(
+            &connection,
+            ("public", "lookup_editions"),
+            (Value::Text("12.50".into()), "12.50"),
+            "postgres",
+        )
+        .await;
+        admin.batch_execute(DROP_EDITIONS).await.unwrap();
+    }
+
     #[tokio::test]
     async fn the_bookshop_is_searched_on_postgres() {
         let Some(url) = url() else {
@@ -453,11 +592,56 @@ mod mysql {
              (9100000000000000007, 'Harbor Lights')",
     ];
 
+    const DROP_EDITIONS: &str = "DROP TABLE IF EXISTS lookup_editions";
+
+    const EDITIONS: [&str; 2] = [
+        "CREATE TABLE lookup_editions (
+             id BIGINT PRIMARY KEY,
+             title VARCHAR(64) NOT NULL,
+             pages INT,
+             price DECIMAL(10,2),
+             published_on DATE
+         )",
+        "INSERT INTO lookup_editions VALUES
+             (1, 'The Lighthouse', 212, 12.50, '1998-04-01'),
+             (2, 'Harbor Nights', 96, 30.00, '2004-11-30'),
+             (3, 'Low Tide', 304, 9.99, '1998-12-24')",
+    ];
+
     fn url() -> Option<String> {
         let url = std::env::var("TABLETIST_TEST_MYSQL_URL").ok();
         url.filter(|url| !url.trim().is_empty())
     }
 
+    #[tokio::test]
+    async fn labels_that_are_no_text_are_searched_as_drawn_on_mysql() {
+        let Some(url) = url() else {
+            eprintln!("skipped: TABLETIST_TEST_MYSQL_URL is not set");
+            return;
+        };
+        let opts = mysql_async::Opts::from_url(&format!("{url}?prefer_socket=false")).unwrap();
+        let mut admin = mysql_async::Conn::new(opts).await.unwrap();
+        admin.query_drop(DROP_EDITIONS).await.unwrap();
+        for statement in EDITIONS {
+            admin.query_drop(statement).await.unwrap();
+        }
+
+        let (mut spec, secrets) = ConnectSpec::from_url(&url).unwrap();
+        spec.tls = TlsMode::Disable;
+        let connection =
+            Connection::connect_with(&spec, &secrets, &HostKeys::default(), Access::ReadOnly)
+                .await
+                .unwrap();
+        labels_that_are_no_text_are_searched_as_drawn(
+            &connection,
+            ("tabletist", "lookup_editions"),
+            (Value::Text("12.50".into()), "12.50"),
+            "mysql",
+        )
+        .await;
+        admin.query_drop(DROP_EDITIONS).await.unwrap();
+    }
+
     #[tokio::test]
     async fn the_bookshop_is_searched_on_mysql() {
         let Some(url) = url() else {
```

- [ ] **Step 2: Run them**

Run: `~/.cargo/bin/cargo fmt --all && ~/.cargo/bin/cargo test --locked -p tabletist-db --test lookup`
Expected: `test result: ok. 12 passed`. Without the servers the PostgreSQL and MySQL tests print "skipped" and pass.

- [ ] **Step 3: Run them on the servers, where you can**

```bash
docker compose up -d --build --wait postgres mysql
TABLETIST_TEST_PG_URL=postgres://tabletist:tabletist@localhost:55432/tabletist \
TABLETIST_TEST_MYSQL_URL=mysql://tabletist:tabletist@localhost:53306/tabletist \
  ~/.cargo/bin/cargo test --locked -p tabletist-db --test lookup -- --nocapture
```

Expected: 12 passed and no "skipped" line. If a server writes a value otherwise than the test expects (a date style that is not ISO, say), do not change the lookup to make it pass: report what the server answered. Where Docker cannot be reached, say so in the report: CI runs the three.

- [ ] **Step 4: Run the four checks**

Expected: all pass.

- [ ] **Step 5: Commit**

```bash
git add crates/tabletist-db/tests/lookup.rs crates/tabletist-db/src/lookup.rs
git commit -S -m "Check that a label that is a date or a number is searched as it is drawn

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

## When the run is done

- The four checks pass on the last commit, and each of the four commits passed them.
- Report: what was looked at in task 3, whether task 4 ran on the servers, and anything a diff needed that this plan did not say.
- Do not open the pull request: it is one for the whole brief, and run 4 opens it. Run 2's plan is written next, against this tree.

# Foreign Key Editing, Run 2: The Picker in the Grid Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** The editor of a grid cell in a single-column foreign key column searches the referenced table as the user types, lists the rows it finds under the cell, and takes the key of the row the user picks, in both looks, on existing and new rows.

**Architecture:** The picker is state of the open editor (`edit::Editor::picker`). `src/key_picker.rs` holds that state and every decision that needs no view: the label column, whether a text can be pinned, which option is highlighted, what Enter takes. `src/app/key_picker.rs` is its reducer: it sends `Command::Lookup`, reads `Event::Lookup`, and is asked by `Action::CommitEdit` what to take. `src/ui/key_picker.rs` draws the list the way `src/ui/sql_complete.rs` draws the completion list: in a foreground area, never taking the keyboard from the field. `src/ui/keys.rs` takes the list's keys ahead of the app's shortcuts.

**Tech Stack:** Rust 1.98, egui (the crmne fork), AccessKit for the headless tests (`src/testing.rs`). Spec: `docs/superpowers/specs/2026-10-10-foreign-key-editing-design.md` (sections 1 to 6 and 9, tests 10 to 20, 23 and 24). Run 1 (`2026-10-10-foreign-key-editing-1-lookup.md`) must be merged first: this plan uses `tabletist_db::{Lookup, LookupPart, LookupPage, LookupRow, Matched}`, `Command::Lookup` and `Event::Lookup`.

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

- `src/shots.rs` is outside those four. Tasks 2 and 7 touch what it builds, so they also run `~/.cargo/bin/cargo clippy --locked --features shots --lib --tests -- -D warnings`.
- **What was run where this plan was written.** Nothing. The code blocks were written from reading the tree at `6e04567` (before run 1) and were not compiled. Where a block and the compiler disagree, the compiler is right: keep the block's behaviour and its tests, and fix its spelling. The line numbers are of that tree; find a place by the names around it.
- **No pixels.** The tests check behaviour, names and words through the AccessKit tree and the actions, as `src/ui/insert_audit_tests.rs` does. No snapshot is added. The list's measures below (widths, row heights) are from the boards and are not tested: look at the result in the running app (`cargo run -- --demo` if the demo has a foreign key, or any database of yours) and say in the task's report what you looked at, or that you could not.
- Run `~/.cargo/bin/cargo fmt --all` before each task's checks.
- House rules that bite here: views push `Action`s and change nothing but the field's text; text is drawn through `TextRole`s only; the focus ring is drawn by `src/ui/focus.rs`; strings go through `gettext`; code that depends on the engine matches on `Dialect` and names every variant; no em dashes; what a user typed never reaches a log; `app.rs` and `ui/mod.rs` get no new code beyond the arms and calls named below.
- Commits are signed, one per task, after its checks pass. If signing fails, do not commit unsigned: stage the task and tell the user. Each message ends with:

      Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>

## What the spec asks, and what gets built

| | The spec | What gets built | Why |
|---|---|---|---|
| a | Whether a text can be pinned goes "by the check a typed cell of that type gets" | The foreign key column's own check (`edit::problem`), on the column of the tab. Its type is the referenced key's type, or one the database compares with it | The tab already holds that column. The referenced table's is only in the column cache |
| b | Omarchy: "a float over the pane. Its first line is the field" | The field stays on the cell, with the `>` prompt and the count in it, and the list is the float under it | One field for both looks: the cell's editor. The float's first line and the cell's field would be two fields for one text |
| c | "At most eight options show, and the rest scroll" | The list follows the highlight: the keys scroll it. The wheel does not | The completion list does the same. A list of 50 that the wheel scrolls wants a scroll area that takes no keyboard: not this run |
| d | "A NULL label and an empty one are drawn as the grid draws a NULL and an empty text" | A NULL label is the word `NULL`, muted. An empty one is an empty first line | The grid's chips are cells' own. Said in the pull request |
| e | A label too long for the list is "cut with an ellipsis", with its match bold | A label that fits has its match bold. One that is cut is drawn plain with its ellipsis | Cutting a text of two weights at a character's edge is what `sql_complete::name_pieces` does for a prefix match only |
| f | "Sending one cancels what the one before it still has running" and "Closing the editor cancels what runs" | Both, through one list of the lookups in flight (`App::lookups`): each frame, one that no open picker waits for is cancelled | An editor is dropped in many places (a save, a reload, a closed tab). One sweep finds them all |
| g | "The delay is a value the test harness sets to nothing" | `App::lookup_delay`, 150 ms, zero in `Harness` | As the spec says |
| h | Test 12: a click on an option "does not first keep the text" | A press on the list keeps the field's keyboard, as a press on the Now button does, so no `LeaveEdit` is sent before the click lands | `cell_editor::now_button` is the pattern |
| k | "A screen reader hears the pick: publisher_id set to ..., Harbor Press" (section 5) | Not built. The list's options are named, the highlighted one is the field's active descendant, and the cell's pending value is what a reader finds after the pick | The reducer takes the option, and only a view can announce (`widgets::announce`, on a frame it draws). **Ask the user** whether it is wanted now: it needs a line the next frame says once |
| l | The answers are announced as "2 publishers match", "50 or more publishers match" (section 9) | "2 matches in publishers", "50 or more matches in publishers", "No matches in publishers" | A table's name cannot be made to agree with a count: "1 publishers matches" |
| m | The field is red, and says why, while its text fails the column's check (today, for every cell) | Not for a foreign key's field while its text is a search: `harb` in a column of numbers is no mistake. It shows once a commit took the text and the column refused it, above the cell, since the list is under it | Section 5: "the text is a search and not a value" |
| i | The row panel's field (section 8) | Not in this run: an editor opened in the panel gets no picker yet (`open_picker` asks for `EditorPlace::Grid`) | Run 3 |
| j | A key that names no row (section 7) | Not in this run | Run 4 |

## File map

| File | What it is for |
|---|---|
| `src/key_picker.rs` | New. `Picker` and its decisions, no UI (task 1) |
| `src/lib.rs` | `pub mod key_picker;` (task 1) |
| `src/edit.rs` | `Editor::picker`, `Editor::select` (task 2) |
| `src/model.rs` | `Action::PickerMove`, `Action::PickOption`, `Action::PickerNull` (task 3) |
| `src/app/key_picker.rs` | New. The reducer: open, search, answers, what a commit takes (tasks 2, 3) |
| `src/app.rs` | `mod key_picker;`, `App::lookup_delay`, `App::lookups`, the arms and calls named in tasks 2, 3 and 6 |
| `src/app/editing.rs` | `edit_cell` opens the picker; the new actions are dropped under a prompt (tasks 2, 3) |
| `src/testing.rs` | The keyed fixture and `Harness::answer_lookups` (task 2) |
| `src/ui/key_picker.rs` | New. The list (tasks 4, 5) |
| `src/ui/key_picker_tests.rs` | New. The app's tests of the spec, 10 to 20 (tasks 2 to 5) |
| `src/ui/mod.rs` | The two `mod` lines (tasks 2, 4) |
| `src/ui/cell_editor.rs` | The field's name, its icon or prompt, select all, Esc in a picker (tasks 4, 5) |
| `src/ui/data_view.rs` | Draws the list at the cell, and turns what it says into actions (task 4) |
| `src/ui/keys.rs` | The list's keys (tasks 4, 5) |
| `src/ui/workspace.rs` | The terminal's line names the picker's keys (task 5) |
| `src/ui/insert_audit_tests.rs` | `INS-09b` moves to the keyed fixture (task 6) |
| `docs/_guide/editing-data.md`, `docs/_guide/omarchy.md`, the spec | Task 7 |

---

### Task 1: What a picker holds, and what it decides

**Files:**
- Create: `src/key_picker.rs`
- Modify: `src/lib.rs`

- [ ] **Step 1: Declare the module**

In `src/lib.rs`, after `pub mod i18n;`:

```rust
pub mod key_picker;
```

- [ ] **Step 2: Write the module with its tests**

Create `src/key_picker.rs`. The tests are at its end, and are written with it: the file is pure, so they are its specification.

```rust
//! The picker of a foreign key cell's editor: what it searches for, what
//! it lists and what Enter takes. No UI, and nothing here talks to the
//! backend: `app::key_picker` sends what this decides.

use std::ops::Range;
use std::time::{Duration, Instant};

use tabletist_db::{
    ColumnClass, ColumnInfo, Dialect, Error, Lookup, LookupPage, LookupPart, LookupRow, Matched,
    ObjectRef, Structure, Value, column_class,
};

use crate::backend::RequestId;
use crate::model::Advance;

/// How many matches a list holds; more are said, not paged.
pub const LIMIT: u32 = 50;
/// How long after the last keystroke a search is sent.
pub const DELAY: Duration = Duration::from_millis(150);
/// How many options show at once; the list follows the highlight.
pub const VISIBLE: usize = 8;

/// The table and the column that the one-column foreign key of `column`
/// points at. `None` for a column no such key names, and for a key whose
/// referenced column the catalog does not say.
pub fn target(structure: &Structure, column: &str) -> Option<(ObjectRef, String)> {
    let foreign = structure
        .foreign_keys
        .iter()
        .find(|foreign| foreign.columns.len() == 1 && foreign.columns[0] == column)?;
    let [key] = foreign.ref_columns.as_slice() else {
        return None;
    };
    let table = ObjectRef::new(foreign.ref_schema.clone(), foreign.ref_table.clone());
    Some((table, key.clone()))
}

/// The column a person reads a row of the referenced table by: its first
/// text column that is not the key.
pub fn label_column(dialect: Dialect, columns: &[ColumnInfo], key: &str) -> Option<String> {
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

/// A UUID as it is written: 36 characters with their hyphens, or 32 hex
/// digits.
fn is_uuid(text: &str) -> bool {
    let hex = |part: &str| part.bytes().all(|byte| byte.is_ascii_hexdigit());
    match text.len() {
        32 => hex(text),
        36 => {
            let mut parts = text.split('-');
            [8, 4, 4, 4, 12]
                .into_iter()
                .all(|len| parts.next().is_some_and(|part| part.len() == len && hex(part)))
                && parts.next().is_none()
        }
        _ => false,
    }
}

/// `text` as the key to read one row by with `=`, where it reads as a
/// value of the key's type. `passes` says the foreign key column's own
/// check takes it. A whole number goes as one: MySQL compares a number
/// with a string as floats. A key of a type the app does not know is
/// never read by `=`, but a `uuid` that is written as one: any other text
/// is an error on PostgreSQL.
pub fn pinned(text: &str, class: ColumnClass, type_name: &str, passes: bool) -> Option<Value> {
    if text.is_empty() || !passes {
        return None;
    }
    let as_text = || Value::Text(text.into());
    match class {
        // The column's check lets a keyword through (`NULL`, `DEFAULT`),
        // which is no key. A whole number too wide for an `i64` goes as
        // its digits.
        ColumnClass::Integer { .. } => match text.parse() {
            Ok(number) => Some(Value::Int(number)),
            Err(_) => text.bytes().all(|byte| byte.is_ascii_digit()).then(as_text),
        },
        ColumnClass::Decimal { .. } | ColumnClass::Float => {
            text.parse::<f64>().is_ok().then(as_text)
        }
        ColumnClass::Text { .. } => Some(as_text()),
        ColumnClass::Other if type_name.eq_ignore_ascii_case("uuid") => {
            is_uuid(text).then(as_text)
        }
        ColumnClass::Boolean | ColumnClass::Json | ColumnClass::Binary | ColumnClass::Other => None,
    }
}

/// Where `text` is first found in `label`, whatever its case: the bytes
/// of `label` to draw as the match. The engine's own comparison decided
/// that the row matched; this is only for the bold.
pub fn found_in(label: &str, text: &str) -> Option<Range<usize>> {
    if text.is_empty() {
        return None;
    }
    let needle: Vec<char> = text.chars().flat_map(char::to_lowercase).collect();
    let starts: Vec<usize> = label.char_indices().map(|(at, _)| at).collect();
    starts.iter().find_map(|&start| {
        let mut end = start;
        let mut matched = 0;
        for character in label[start..].chars() {
            for lower in character.to_lowercase() {
                if needle.get(matched) != Some(&lower) {
                    return None;
                }
                matched += 1;
            }
            end += character.len_utf8();
            if matched == needle.len() {
                return Some(start..end);
            }
        }
        None
    })
}

/// What is known of the referenced table's label column.
#[derive(Clone, PartialEq)]
pub enum Columns {
    /// Its columns are being fetched.
    Waiting,
    /// The label column, or none: the table is searched by its key alone.
    Known(Option<String>),
}

/// The search that was last sent.
pub struct Asked {
    /// What it is for: the editor's text without the spaces at its ends,
    /// or nothing.
    pub text: String,
    /// The read of the pinned row, while it runs.
    pub pinned: Option<RequestId>,
    /// The read of the matches, while it runs.
    pub matching: Option<RequestId>,
    /// The pinned row's answer once it came: the row, or none.
    pub first: Option<Option<LookupRow>>,
}

/// What Enter or Tab takes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Take {
    /// The text as typed: today's commit.
    Text,
    /// The option at this place of the list.
    Option(usize),
    /// Nothing yet: the list is not the answer for the text that shows.
    Wait,
}

/// The picker of an open editor.
pub struct Picker {
    /// The table the key points at.
    pub target: ObjectRef,
    /// Its column the key references.
    pub key: String,
    pub label: Columns,
    /// The row the cell pointed at when the editor opened on its value:
    /// listed first and marked while nothing is searched for.
    pub current: Option<Value>,
    /// The text is searched for. False while an editor that opened on its
    /// value has not been typed into: the search is for nothing.
    pub searching: bool,
    /// The editor's text when it was last looked at: `Action::EditorTyped`
    /// is also sent for a text that did not change.
    seen: String,
    pub asked: Option<Asked>,
    pub options: Vec<LookupRow>,
    /// There are more matches than the list holds.
    pub more: bool,
    /// The text `options` answer. `None` before the first answer.
    pub listed: Option<String>,
    pub highlight: Option<usize>,
    /// The user moved the highlight since the list last changed.
    pub moved: bool,
    /// When the next search is to be sent.
    pub due: Option<Instant>,
    /// Enter or Tab, pressed before the list answered the text.
    pub kept: Option<Advance>,
    /// Why there is no list: the database's words.
    pub failed: Option<Error>,
    /// A commit took the text and its column refused it. Until then what
    /// the column would say of the text is not shown: the text is a
    /// search, and `harb` is no mistake in a column of numbers.
    pub refused: bool,
}

impl Picker {
    /// The picker of an editor that opens with `text`. `searching` is
    /// false for one opened on its cell's value.
    pub fn new(
        (target, key): (ObjectRef, String),
        label: Columns,
        current: Option<Value>,
        searching: bool,
        text: &str,
    ) -> Self {
        Self {
            target,
            key,
            label,
            current,
            searching,
            seen: text.to_owned(),
            asked: None,
            options: Vec::new(),
            more: false,
            listed: None,
            highlight: None,
            moved: false,
            due: None,
            kept: None,
            failed: None,
            refused: false,
        }
    }

    /// What is searched for while the editor holds `typed`.
    pub fn text<'a>(&self, typed: &'a str) -> &'a str {
        if self.searching { typed.trim() } else { "" }
    }

    /// The editor's text is `typed` now. A text that changed is searched
    /// for `delay` from `now`, and what waited for the old one is dropped.
    pub fn typed(&mut self, typed: &str, now: Instant, delay: Duration) {
        if typed == self.seen {
            return;
        }
        self.seen = typed.to_owned();
        self.searching = true;
        self.due = Some(now + delay);
        self.kept = None;
        self.moved = false;
        self.refused = false;
    }

    /// The two reads of a search for `text`: the pinned row's, where `pin`
    /// names one, and the matches'.
    pub fn lookups(&self, label: Option<String>, text: &str, pin: Option<Value>) -> (Option<Lookup>, Lookup) {
        let read = |part: LookupPart, limit: u32| Lookup {
            object: self.target.clone(),
            key: self.key.clone(),
            label: label.clone(),
            part,
            limit,
        };
        (
            pin.map(|value| read(LookupPart::Pinned(value), 1)),
            read(LookupPart::Matching(text.to_owned()), LIMIT),
        )
    }

    /// The requests whose answers this picker still waits for.
    pub fn in_flight(&self) -> Vec<RequestId> {
        self.asked
            .iter()
            .flat_map(|asked| [asked.pinned, asked.matching])
            .flatten()
            .collect()
    }

    /// Whether `key` is the row the cell pointed at when the editor
    /// opened.
    pub fn is_current(&self, key: &Value) -> bool {
        self.current.as_ref() == Some(key)
    }

    /// Takes the answer of `request`, if it is one this picker waits for.
    /// Returns whether it was.
    pub fn answered(&mut self, request: RequestId, result: Result<LookupPage, Error>) -> bool {
        let Some(asked) = self.asked.as_mut() else {
            return false;
        };
        if asked.pinned == Some(request) {
            asked.pinned = None;
            // A failure is said by the matches' answer, which fails too.
            let first = result.ok().and_then(|page| page.rows.into_iter().next());
            if let Some(row) = &first {
                // Shown at once, first, over whatever the list held.
                self.options.retain(|option| option.key != row.key);
                self.options.insert(0, row.clone());
                self.highlight = Some(0);
                self.moved = false;
            }
            asked.first = Some(first);
            return true;
        }
        if asked.matching != Some(request) {
            return false;
        }
        asked.matching = None;
        self.listed = Some(asked.text.clone());
        self.moved = false;
        match result {
            Ok(page) => {
                let mut options: Vec<LookupRow> = asked.first.clone().flatten().into_iter().collect();
                for row in page.rows {
                    if !options.iter().any(|option| option.key == row.key) {
                        options.push(row);
                    }
                }
                self.options = options;
                self.more = page.more;
                self.failed = None;
            }
            Err(error) => {
                self.options.clear();
                self.more = false;
                self.failed = Some(error);
            }
        }
        self.highlight = self.first_highlight();
        true
    }

    /// Which option an answer highlights. With a text: the first, when
    /// its key is the text or its label matched; none when the options
    /// only begin with the text in their keys, so a typed `12` is not
    /// turned into row 120. With a field the user emptied: none. With the
    /// text not yet changed: the row the cell points at.
    fn first_highlight(&self) -> Option<usize> {
        if !self.searching {
            let current = self.current.as_ref()?;
            return self.options.iter().position(|option| &option.key == current);
        }
        if self.listed.as_deref() == Some("") {
            return None;
        }
        match self.options.first()?.matched {
            Matched::Key | Matched::Label => Some(0),
            Matched::KeyStart => None,
        }
    }

    /// Moves the highlight `by` options, stopping at the list's ends. From
    /// none, a step down goes to the first.
    pub fn step(&mut self, by: isize) {
        self.kept = None;
        if self.options.is_empty() {
            return;
        }
        let last = self.options.len() - 1;
        let to = match self.highlight {
            Some(at) => at.saturating_add_signed(by).min(last),
            None if by > 0 => 0,
            None => return,
        };
        self.highlight = Some(to);
        self.moved = true;
    }

    /// The user chose the option at `index`, by a click.
    pub fn choose(&mut self, index: usize) {
        if index < self.options.len() {
            self.highlight = Some(index);
            self.moved = true;
        }
    }

    /// What Enter or Tab takes while the editor holds `typed`.
    pub fn take(&self, typed: &str) -> Take {
        // An option the user moved to is the row they chose, whatever
        // list shows.
        if self.moved {
            return self.highlight.map_or(Take::Text, Take::Option);
        }
        // Opened on its value and not typed into: nothing was searched
        // for, and nothing is taken.
        if !self.searching || self.failed.is_some() {
            return Take::Text;
        }
        if self.due.is_some() {
            return Take::Wait;
        }
        let text = typed.trim();
        let Some(asked) = self.asked.as_ref().filter(|asked| asked.text == text) else {
            return Take::Wait;
        };
        // The pinned row came back found: it is the first option, and a
        // full id and Enter does not wait for the scan.
        if let Some(Some(_)) = &asked.first {
            return Take::Option(0);
        }
        if asked.matching.is_some() || self.listed.as_deref() != Some(text) {
            return Take::Wait;
        }
        self.highlight.map_or(Take::Text, Take::Option)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const HARBOR_PRESS: i64 = 9_100_000_000_000_000_004;
    const HARBOR_LIGHTS: i64 = 9_100_000_000_000_000_007;

    fn row(key: i64, label: &str, matched: Matched) -> LookupRow {
        LookupRow {
            key: Value::Int(key),
            label: Some(Value::Text(label.into())),
            matched,
        }
    }

    fn page(rows: Vec<LookupRow>) -> Result<LookupPage, Error> {
        Ok(LookupPage { rows, more: false })
    }

    fn picker(searching: bool, text: &str) -> Picker {
        Picker::new(
            (ObjectRef::new("main", "publishers"), "id".into()),
            Columns::Known(Some("name".into())),
            None,
            searching,
            text,
        )
    }

    /// A picker that asked for `text`, with a pinned read or without one.
    fn asking(text: &str, pinned: bool) -> Picker {
        let mut picker = picker(true, text);
        picker.asked = Some(Asked {
            text: text.to_owned(),
            pinned: pinned.then_some(RequestId(1)),
            matching: Some(RequestId(2)),
            first: None,
        });
        picker
    }

    fn keys(picker: &Picker) -> Vec<Value> {
        picker.options.iter().map(|option| option.key.clone()).collect()
    }

    #[test]
    fn a_one_column_key_has_a_target_and_a_composite_one_has_none() {
        let key = |columns: &[&str], referenced: &[&str]| tabletist_db::ForeignKeyInfo {
            columns: columns.iter().map(|name| (*name).to_owned()).collect(),
            ref_schema: "main".into(),
            ref_table: "publishers".into(),
            ref_columns: referenced.iter().map(|name| (*name).to_owned()).collect(),
            ..tabletist_db::ForeignKeyInfo::default()
        };
        let structure = Structure {
            foreign_keys: vec![
                key(&["a", "b"], &["a", "b"]),
                key(&["publisher_id"], &["id"]),
                key(&["unnamed_id"], &[]),
            ],
            ..Structure::default()
        };
        assert_eq!(
            target(&structure, "publisher_id"),
            Some((ObjectRef::new("main", "publishers"), "id".to_owned()))
        );
        assert_eq!(target(&structure, "a"), None);
        assert_eq!(target(&structure, "unnamed_id"), None);
        assert_eq!(target(&structure, "kind"), None);
    }

    #[test]
    fn the_label_is_the_first_text_column_that_is_not_the_key() {
        let column = |name: &str, type_name: &str| ColumnInfo {
            name: name.into(),
            type_name: type_name.into(),
            ..ColumnInfo::default()
        };
        // As the structure names PostgreSQL's types, which is how
        // `column_class` reads them.
        let columns = [
            column("code", "text"),
            column("founded", "integer"),
            column("name", "character varying(64)"),
            column("email", "text"),
        ];
        assert_eq!(
            label_column(Dialect::Postgres, &columns, "code"),
            Some("name".to_owned())
        );
        assert_eq!(
            label_column(Dialect::Postgres, &columns, "founded"),
            Some("code".to_owned())
        );
        assert_eq!(label_column(Dialect::Postgres, &columns[1..2], "id"), None);
    }

    #[test]
    fn a_text_is_pinned_only_where_it_reads_as_the_keys_type() {
        let whole = ColumnClass::Integer {
            min: i128::from(i64::MIN),
            max: i128::from(i64::MAX),
        };
        assert_eq!(
            pinned("9100000000000000004", whole, "int8", true),
            Some(Value::Int(HARBOR_PRESS))
        );
        // What the column's own check refuses is never sent with `=`.
        assert_eq!(pinned("harb", whole, "int8", false), None);
        assert_eq!(pinned("", whole, "int8", true), None);
        // A keyword passes the column's check and is no key.
        assert_eq!(pinned("NULL", whole, "int8", true), None);
        assert_eq!(
            pinned("18446744073709551615", whole, "bigint unsigned", true),
            Some(Value::Text("18446744073709551615".into()))
        );
        let text = ColumnClass::Text { max_chars: None };
        assert_eq!(pinned("DE", text, "text", true), Some(Value::Text("DE".into())));
        let uuid = "123e4567-e89b-12d3-a456-426614174000";
        assert_eq!(
            pinned(uuid, ColumnClass::Other, "uuid", true),
            Some(Value::Text(uuid.into()))
        );
        assert_eq!(pinned("harb", ColumnClass::Other, "uuid", true), None);
        assert_eq!(pinned("1", ColumnClass::Other, "ltree", true), None);
    }

    #[test]
    fn the_match_is_found_whatever_its_case_and_on_whole_characters() {
        assert_eq!(found_in("Harbor Press", "harb"), Some(0..4));
        assert_eq!(found_in("New Harbor", "HARBOR"), Some(4..10));
        assert_eq!(found_in("Zoë's", "ë"), Some(2..4));
        assert_eq!(found_in("Harbor", "x"), None);
        assert_eq!(found_in("Harbor", ""), None);
    }

    #[test]
    fn an_answer_highlights_a_label_or_a_key_and_not_the_start_of_a_key() {
        let mut picker = asking("harb", false);
        assert!(picker.answered(
            RequestId(2),
            page(vec![
                row(HARBOR_PRESS, "Harbor Press", Matched::Label),
                row(HARBOR_LIGHTS, "Harbor Lights", Matched::Label),
            ])
        ));
        assert_eq!(picker.highlight, Some(0));
        assert_eq!(picker.take("harb"), Take::Option(0));

        let mut picker = asking("12", false);
        picker.answered(RequestId(2), page(vec![row(120, "Aardvark", Matched::KeyStart)]));
        assert_eq!(picker.highlight, None);
        // A typed 12 is not turned into row 120.
        assert_eq!(picker.take("12"), Take::Text);
    }

    #[test]
    fn the_pinned_row_is_first_and_listed_once() {
        let mut picker = asking("5", true);
        picker.answered(RequestId(1), page(vec![row(5, "Studio 91", Matched::Key)]));
        // Found: Enter does not wait for the matches.
        assert_eq!(picker.take("5"), Take::Option(0));
        picker.answered(
            RequestId(2),
            page(vec![
                row(5, "Studio 91", Matched::Key),
                row(50, "Fifty", Matched::KeyStart),
            ]),
        );
        assert_eq!(keys(&picker), [Value::Int(5), Value::Int(50)]);
        assert_eq!(picker.highlight, Some(0));
    }

    #[test]
    fn enter_waits_for_the_answer_of_the_text_that_shows() {
        let mut picker = asking("har", false);
        picker.answered(RequestId(2), page(vec![row(1, "Harrow", Matched::Label)]));
        assert_eq!(picker.take("har"), Take::Option(0));
        // One more letter: the list is for another text now.
        let now = Instant::now();
        picker.typed("harb", now, DELAY);
        assert_eq!(picker.due, Some(now + DELAY));
        assert_eq!(picker.take("harb"), Take::Wait);
        // Sent, not yet answered.
        picker.due = None;
        picker.asked = Some(Asked {
            text: "harb".into(),
            pinned: None,
            matching: Some(RequestId(4)),
            first: None,
        });
        assert_eq!(picker.take("harb"), Take::Wait);
        // An answer for the old text is not this picker's any more.
        assert!(!picker.answered(RequestId(2), page(Vec::new())));
        picker.answered(RequestId(4), page(vec![row(HARBOR_PRESS, "Harbor Press", Matched::Label)]));
        assert_eq!(picker.take("harb"), Take::Option(0));
    }

    #[test]
    fn a_text_that_did_not_change_starts_no_search() {
        let mut picker = picker(false, "9100000000000000001");
        picker.typed("9100000000000000001", Instant::now(), DELAY);
        assert!(!picker.searching);
        assert_eq!(picker.due, None);
        assert_eq!(picker.text("9100000000000000001"), "");
        // Not typed into: Enter takes nothing, whatever is listed.
        assert_eq!(picker.take("9100000000000000001"), Take::Text);
    }

    #[test]
    fn with_no_text_the_current_row_is_highlighted_and_an_emptied_field_has_none() {
        let mut opened = picker(false, "9100000000000000004");
        opened.current = Some(Value::Int(HARBOR_PRESS));
        opened.asked = Some(Asked {
            text: String::new(),
            pinned: Some(RequestId(1)),
            matching: Some(RequestId(2)),
            first: None,
        });
        opened.answered(RequestId(1), page(vec![row(HARBOR_PRESS, "Harbor Press", Matched::Key)]));
        opened.answered(
            RequestId(2),
            page(vec![
                row(5, "Studio 91", Matched::KeyStart),
                row(HARBOR_PRESS, "Harbor Press", Matched::KeyStart),
            ]),
        );
        assert_eq!(keys(&opened), [Value::Int(HARBOR_PRESS), Value::Int(5)]);
        assert_eq!(opened.highlight, Some(0));
        assert!(opened.is_current(&Value::Int(HARBOR_PRESS)));

        let mut emptied = asking("", false);
        emptied.answered(RequestId(2), page(vec![row(5, "Studio 91", Matched::KeyStart)]));
        assert_eq!(emptied.highlight, None);
        assert_eq!(emptied.take(""), Take::Text);
    }

    #[test]
    fn the_arrows_move_the_highlight_and_stop_at_the_ends() {
        let mut picker = asking("9", false);
        picker.answered(
            RequestId(2),
            page(vec![
                row(90, "a", Matched::KeyStart),
                row(91, "b", Matched::KeyStart),
                row(92, "c", Matched::KeyStart),
            ]),
        );
        assert_eq!(picker.highlight, None);
        picker.step(-1);
        assert_eq!(picker.highlight, None);
        picker.step(1);
        assert_eq!(picker.highlight, Some(0));
        picker.step(8);
        assert_eq!(picker.highlight, Some(2));
        picker.step(-1);
        assert_eq!(picker.highlight, Some(1));
        // The user's choice is taken at once, from whatever list shows.
        picker.due = Some(Instant::now());
        assert_eq!(picker.take("9x"), Take::Option(1));
    }

    #[test]
    fn a_search_that_fails_takes_the_text() {
        let mut picker = asking("harb", false);
        picker.kept = Some(Advance::Down);
        picker.answered(RequestId(2), Err(Error::query("permission denied for table publishers")));
        assert!(picker.failed.is_some());
        assert!(picker.options.is_empty());
        assert_eq!(picker.take("harb"), Take::Text);
    }

    #[test]
    fn a_search_is_two_reads_when_a_row_is_pinned() {
        let picker = picker(true, "5");
        let (pinned, matching) = picker.lookups(Some("name".into()), "5", Some(Value::Int(5)));
        let pinned = pinned.unwrap();
        assert!(pinned.part == LookupPart::Pinned(Value::Int(5)));
        assert_eq!(pinned.limit, 1);
        assert!(matching.part == LookupPart::Matching("5".into()));
        assert_eq!(matching.limit, LIMIT);
        let (pinned, _) = picker.lookups(None, "harb", None);
        assert!(pinned.is_none());
    }
}
```

`tabletist_db::LookupRow` has no `Debug`, so the tests compare keys, not rows.

- [ ] **Step 3: Run the tests**

Run: `~/.cargo/bin/cargo test --locked --lib key_picker::tests`
Expected: PASS, twelve tests. A failure here is a mistake in the module, not in a test: each test is a sentence of the spec's sections 3 to 5.

- [ ] **Step 4: Run the four checks, then commit**

`Picker` is not used outside its tests yet. If clippy says it is dead code, the cause is that `src/lib.rs` does not make the module `pub`: step 1 does.

```bash
git add src/key_picker.rs src/lib.rs && git commit -m "Decide what a foreign key picker lists and takes

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 2: An editor on a foreign key cell opens its picker and searches

**Files:**
- Modify: `src/edit.rs` (`Editor`, near line 1034, and the two test editors near lines 2409 and 3406)
- Create: `src/app/key_picker.rs`
- Modify: `src/app.rs` (`mod editing;`, `struct App` near line 64, `App::new` near line 151, `apply_event`, `frame_ui` near line 3865, the test editor near line 13486)
- Modify: `src/app/editing.rs` (`edit_cell`, near line 725 and 745)
- Modify: `src/testing.rs`
- Create: `src/ui/key_picker_tests.rs`
- Modify: `src/ui/mod.rs` (the `mod` list, beside `mod insert_audit_tests;`)

- [ ] **Step 1: The editor holds a picker**

In `src/edit.rs`, add two fields to `pub struct Editor`, after `problem`:

```rust
    /// The search of the table a foreign key cell points at, while the
    /// editor is the small field of such a cell.
    pub picker: Option<crate::key_picker::Picker>,
    /// Taken by the view with `focus`: the whole text is selected, not
    /// the cursor put at its end. An editor opened on a foreign key's
    /// value, where the first character typed starts a search.
    pub select: bool,
```

Every place that builds an `Editor` gets `picker: None, select: false,`: `edit_cell` in `src/app/editing.rs` (near line 725), the two tests of `src/edit.rs` and the one of `src/app.rs` that the compiler names.

- [ ] **Step 2: The fixture and the fake table**

In `src/testing.rs`, after `book_covers_page`, add:

```rust
/// The Bookshop's publishers, as (id, name).
pub const PUBLISHERS: [(i64, &str); 3] = [
    (9_100_000_000_000_000_001, "Northlight Books"),
    (9_100_000_000_000_000_004, "Harbor Press"),
    (9_100_000_000_000_000_007, "Harbor Lights"),
];

/// The structure of the Bookshop's `publishers`.
pub fn publishers_structure() -> tabletist_db::Structure {
    use tabletist_db::ColumnInfo;
    let column = |name: &str, type_name: &str| ColumnInfo {
        name: name.into(),
        type_name: type_name.into(),
        ..ColumnInfo::default()
    };
    tabletist_db::Structure {
        columns: vec![column("id", "INTEGER"), column("name", "TEXT")],
        primary_key: vec!["id".into()],
        ..tabletist_db::Structure::default()
    }
}

/// `book_covers` with its foreign key: `publisher_id` points at
/// `publishers.id`. The tests that type into `publisher_id` as a number
/// keep [`book_covers_structure`], which has no key.
pub fn keyed_covers_structure() -> tabletist_db::Structure {
    tabletist_db::Structure {
        foreign_keys: vec![tabletist_db::ForeignKeyInfo {
            columns: vec!["publisher_id".into()],
            ref_schema: "main".into(),
            ref_table: "publishers".into(),
            ref_columns: vec!["id".into()],
            ..tabletist_db::ForeignKeyInfo::default()
        }],
        ..book_covers_structure()
    }
}

/// A page of `book_covers` whose three rows point at the three
/// publishers in turn: Northlight Books, Harbor Press, Harbor Lights.
/// [`book_covers_page`] numbers its publishers on from the first, and
/// two of those are no publisher.
pub fn keyed_covers_page() -> RowPage {
    let mut page = book_covers_page(3);
    for (row, (key, _)) in page.rows.iter_mut().zip(PUBLISHERS) {
        row[1] = Value::Int(key);
    }
    page
}

/// What a database holding `rows` as a table of (key, label) answers
/// `lookup`: by the rules of `Connection::lookup`, with ASCII's sense of
/// case.
pub fn fake_lookup(lookup: &tabletist_db::Lookup, rows: &[(i64, &str)]) -> tabletist_db::LookupPage {
    use tabletist_db::{LookupPart, LookupRow, Matched};
    let rank = |matched: Matched| match matched {
        Matched::Key => 0,
        Matched::Label => 1,
        Matched::KeyStart => 2,
    };
    let mut found: Vec<(Matched, i64, &str)> = rows
        .iter()
        .filter_map(|&(key, label)| {
            let written = key.to_string();
            let matched = match &lookup.part {
                LookupPart::Pinned(Value::Int(pinned)) => (*pinned == key).then_some(Matched::Key),
                LookupPart::Pinned(Value::Text(pinned)) => {
                    (**pinned == *written).then_some(Matched::Key)
                }
                LookupPart::Pinned(_) => None,
                LookupPart::Matching(text) if text.is_empty() => Some(Matched::KeyStart),
                LookupPart::Matching(text) if written == *text => Some(Matched::Key),
                LookupPart::Matching(text)
                    if lookup.label.is_some()
                        && label.to_lowercase().contains(&text.to_lowercase()) =>
                {
                    Some(Matched::Label)
                }
                LookupPart::Matching(text) => written.starts_with(text.as_str()).then_some(Matched::KeyStart),
            };
            Some((matched?, key, label))
        })
        .collect();
    found.sort_by_key(|&(matched, key, _)| (rank(matched), key));
    let limit = lookup.limit as usize;
    let more = found.len() > limit;
    found.truncate(limit);
    tabletist_db::LookupPage {
        rows: found
            .into_iter()
            .map(|(matched, key, label)| LookupRow {
                key: Value::Int(key),
                label: lookup.label.as_ref().map(|_| Value::Text(label.into())),
                matched,
            })
            .collect(),
        more,
    }
}
```

In `pub struct Harness` add a field, and set it to `0` in `with_backend`:

```rust
    /// How many of the `Lookup` commands sent so far were answered by
    /// [`Harness::answer_lookups`].
    pub lookups_answered: usize,
```

In `with_backend`, after `app.look = crate::theme::Look::standard();`:

```rust
        // A search is sent on the frame after the keystroke, not 150 ms
        // later: no test waits.
        app.lookup_delay = std::time::Duration::ZERO;
```

And a new `impl Harness` block after the one that holds `book_covers`:

```rust
impl Harness {
    /// As [`Self::book_covers`], with the key of `publisher_id` to
    /// `publishers`: a table whose `publisher_id` cells open the picker.
    pub fn keyed_covers(&mut self) -> (ConnTabId, TabId) {
        let tab = self.connect_fake_as(false);
        self.app.apply(Action::OpenObject {
            tab,
            object: tabletist_db::ObjectRef::new("main", "book_covers"),
            kind: ObjectKind::Table,
            pin: true,
        });
        self.answer_structure(keyed_covers_structure());
        self.answer_rows(keyed_covers_page());
        let id = self
            .app
            .workspace(tab)
            .and_then(|workspace| workspace.active_tab)
            .expect("the table's tab is open");
        (tab, id)
    }

    /// The `Lookup` commands sent so far, with their requests.
    pub fn lookups(&self) -> Vec<(SessionId, RequestId, tabletist_db::Lookup)> {
        self.app
            .backend
            .sent
            .iter()
            .filter_map(|command| match command {
                Command::Lookup {
                    session,
                    request,
                    lookup,
                } => Some((*session, *request, lookup.clone())),
                _ => None,
            })
            .collect()
    }

    /// Answers every lookup not answered yet, in the order they were
    /// sent, as a table holding `rows` would. One that was superseded is
    /// answered too: the app drops what it no longer waits for. Returns
    /// how many it answered.
    pub fn answer_lookups(&mut self, rows: &[(i64, &str)]) -> usize {
        let waiting: Vec<_> = self.lookups().split_off(self.lookups_answered);
        for (session, request, lookup) in &waiting {
            self.app.apply(Action::Backend(Event::Lookup {
                session: *session,
                request: *request,
                result: Ok(fake_lookup(lookup, rows)),
            }));
        }
        self.lookups_answered += waiting.len();
        waiting.len()
    }

    /// Answers the newest lookup with `error`, and counts every lookup
    /// before it as answered.
    pub fn fail_lookup(&mut self, error: tabletist_db::Error) {
        let all = self.lookups();
        let (session, request, _) = all.last().expect("a Lookup was sent").clone();
        self.lookups_answered = all.len();
        self.app.apply(Action::Backend(Event::Lookup {
            session,
            request,
            result: Err(error),
        }));
    }
}
```

`SessionId` and `RequestId` may need adding to the file's `use crate::backend::{...}` line.

- [ ] **Step 3: Write the failing tests**

Create `src/ui/key_picker_tests.rs`:

```rust
//! The foreign key picker, checked against its spec
//! (`docs/superpowers/specs/2026-10-10-foreign-key-editing-design.md`:
//! "test N" below is a test of its list, "spec N" a section of it), on
//! the Bookshop's `book_covers` and `publishers`.
//!
//! Only behaviour, names and words are checked. No colour, size or
//! picture of the design is.

use egui::{Key, Modifiers};
use tabletist_db::{LookupPart, Value};

use crate::backend::Command;
use crate::key_picker::{Picker, Take};
use crate::model::{Action, Advance, CellPos, ConnTabId, EditStart, Pane, TabId};
use crate::testing::{Harness, PUBLISHERS, publishers_structure};
use crate::theme::Look;

const NORTHLIGHT: &str = "9100000000000000001";
const HARBOR_PRESS: &str = "9100000000000000004";
const HARBOR_LIGHTS: &str = "9100000000000000007";
/// `publisher_id`, by its place in the page.
const PUBLISHER: usize = 1;

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

/// Opens the editor on `publisher_id` of the page's row `row` from
/// `start`, and answers what the picker asks: the publishers' columns,
/// then its lookups.
fn edit(harness: &mut Harness, tab: ConnTabId, id: TabId, row: usize, start: EditStart) {
    harness.app.apply(Action::EditCell {
        tab,
        id,
        cell: CellPos {
            row,
            col: PUBLISHER,
        },
        start,
    });
    // The newest Describe is the one for the publishers' columns, where
    // one was sent; where none was, this answers one already answered and
    // changes nothing.
    harness.answer_structure(publishers_structure());
    harness.answer_lookups(&PUBLISHERS);
}

/// Makes the editor's text `text`, as typing does, and no more.
fn set_text(harness: &mut Harness, tab: ConnTabId, id: TabId, text: &str) {
    let object = harness.app.workspace_mut(tab).unwrap().object_tab_mut(id).unwrap();
    object.edits.editor.as_mut().unwrap().text = text.to_owned();
    harness.app.apply(Action::EditorTyped { tab, id });
}

fn picker<'h>(harness: &'h Harness, tab: ConnTabId, id: TabId) -> Option<&'h Picker> {
    let object = harness.app.workspace(tab)?.object_tab(id)?;
    object.edits.editor.as_ref()?.picker.as_ref()
}

/// The keys the picker lists, as text.
fn listed(harness: &Harness, tab: ConnTabId, id: TabId) -> Vec<String> {
    picker(harness, tab, id)
        .unwrap()
        .options
        .iter()
        .map(|option| crate::ui::format::plain_text(&option.key))
        .collect()
}

fn editing(harness: &Harness, tab: ConnTabId, id: TabId) -> bool {
    let object = harness.app.workspace(tab).unwrap().object_tab(id).unwrap();
    object.edits.editor.is_some()
}

/// Test 10, the part no view is needed for: an editor opened on the
/// cell's value searches for nothing, lists the current row first and
/// highlights it.
#[test]
fn an_editor_on_a_foreign_key_lists_the_table_with_the_current_row_first() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    // Row 1 of the page points at Harbor Press (`keyed_covers_page`).
    edit(&mut harness, tab, id, 1, EditStart::Value);
    let picker = picker(&harness, tab, id).expect("the editor has a picker");
    assert_eq!(picker.target.name, "publishers");
    assert_eq!(picker.key, "id");
    assert!(!picker.searching);
    assert_eq!(
        listed(&harness, tab, id),
        [HARBOR_PRESS, NORTHLIGHT, HARBOR_LIGHTS]
    );
    assert_eq!(picker.highlight, Some(0));
    assert!(picker.is_current(&picker.options[0].key));
    // Two reads: the row the cell points at, by its key as a number, and
    // the table's first rows.
    let sent = harness.lookups();
    assert_eq!(sent.len(), 2);
    assert!(sent[0].2.part == LookupPart::Pinned(Value::Int(9_100_000_000_000_000_004)));
    assert!(sent[1].2.part == LookupPart::Matching(String::new()));
    assert_eq!(sent[1].2.label.as_deref(), Some("name"));
    // On macOS the text is selected, so typing starts a search.
    let object = harness.app.workspace(tab).unwrap().object_tab(id).unwrap();
    assert!(object.edits.editor.as_ref().unwrap().select);
}

/// Spec 2: opened by a typed character, the search is for it from the
/// start.
#[test]
fn a_typed_character_is_searched_for_at_once() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    edit(&mut harness, tab, id, 0, EditStart::Typed("h".into()));
    let picker = picker(&harness, tab, id).unwrap();
    assert!(picker.searching);
    assert_eq!(picker.listed.as_deref(), Some("h"));
    // Northlight, Harbor Press and Harbor Lights all hold an h.
    assert_eq!(listed(&harness, tab, id).len(), 3);
    assert_eq!(picker.highlight, Some(0));
}

/// Test 16: a later answer replaces the list, an answer for an earlier
/// text is dropped, and sending a search cancels the one before it.
#[test]
fn an_answer_for_an_earlier_text_is_dropped() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    edit(&mut harness, tab, id, 0, EditStart::Typed("h".into()));
    // Two searches sent, neither answered.
    set_text(&mut harness, tab, id, "ha");
    harness.app.send_due_lookups(std::time::Instant::now());
    set_text(&mut harness, tab, id, "harb");
    harness.app.send_due_lookups(std::time::Instant::now());
    let sent = harness.lookups();
    let of_ha = sent.iter().rev().nth(1).unwrap().1;
    let cancelled = harness.app.backend.sent.iter().any(|command| {
        matches!(command, Command::Cancel { request, .. } if *request == of_ha)
    });
    assert!(cancelled, "the search for `ha` was not cancelled");
    harness.answer_lookups(&PUBLISHERS);
    let picker = picker(&harness, tab, id).unwrap();
    assert_eq!(picker.listed.as_deref(), Some("harb"));
    assert_eq!(listed(&harness, tab, id), [HARBOR_PRESS, HARBOR_LIGHTS]);
}

/// Test 17: a column no one-column key names opens the text editor and
/// no list, and so does the large editor.
#[test]
fn a_column_without_a_key_and_the_large_editor_have_no_picker() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    // `kind` is no foreign key.
    harness.app.apply(Action::EditCell {
        tab,
        id,
        cell: CellPos { row: 0, col: 2 },
        start: EditStart::Value,
    });
    assert!(editing(&harness, tab, id));
    assert!(picker(&harness, tab, id).is_none());
    harness.app.apply(Action::CancelEdit { tab, id });
    // Alt+Enter moves the text to the large editor, which has no list.
    edit(&mut harness, tab, id, 0, EditStart::Typed("h".into()));
    assert!(picker(&harness, tab, id).is_some());
    harness.app.apply(Action::EditorBreak { tab, id });
    assert!(picker(&harness, tab, id).is_none());
}

/// Spec 3 and test 19: the columns of the referenced table that cannot be
/// read, and a search the database refuses, are the list's failure.
#[test]
fn a_search_that_fails_says_so_and_leaves_the_text_editor() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    edit(&mut harness, tab, id, 0, EditStart::Typed("9".into()));
    set_text(&mut harness, tab, id, "91");
    harness.app.send_due_lookups(std::time::Instant::now());
    harness.fail_lookup(tabletist_db::Error::query("permission denied for table publishers"));
    let picker = picker(&harness, tab, id).unwrap();
    assert!(picker.failed.is_some());
    assert_eq!(picker.take("91"), Take::Text);
}

/// Spec 8, the last point: an editor that goes takes its searches with
/// it.
#[test]
fn closing_the_editor_cancels_what_runs() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    harness.app.apply(Action::EditCell {
        tab,
        id,
        cell: CellPos {
            row: 0,
            col: PUBLISHER,
        },
        start: EditStart::Typed("h".into()),
    });
    harness.answer_structure(publishers_structure());
    let running = harness.lookups().last().unwrap().1;
    harness.app.apply(Action::CancelEdit { tab, id });
    harness.app.send_due_lookups(std::time::Instant::now());
    let cancelled = harness.app.backend.sent.iter().any(|command| {
        matches!(command, Command::Cancel { request, .. } if *request == running)
    });
    assert!(cancelled);
    // Its late answer finds no picker and changes nothing.
    harness.answer_lookups(&PUBLISHERS);
    assert!(!editing(&harness, tab, id));
}
```

`Advance`, `Key` and `Modifiers` are for the tests the next tasks add, and so are the constants `HARBOR_LIGHTS` and `NORTHLIGHT` if no test of this task names them. Clippy refuses what a test module does not use: take them out of this task's file and add each back in the task that first needs it.

In `src/ui/mod.rs`, beside the line that declares `insert_audit_tests`, add:

```rust
#[cfg(test)]
mod key_picker_tests;
```

- [ ] **Step 4: Run them to see them fail**

Run: `~/.cargo/bin/cargo test --locked --lib key_picker_tests`
Expected: compile errors: no field `lookup_delay` on `App`, no method `send_due_lookups`, no field `picker` on `Editor` where step 1 was not done.

- [ ] **Step 5: The app's fields**

In `src/app.rs`, in `pub struct App`, after `closed_editors`:

```rust
    /// How long after the last keystroke a foreign key picker's search is
    /// sent. `key_picker::DELAY`; nothing in the tests.
    pub lookup_delay: std::time::Duration,
    /// The lookups in flight. One that no open picker waits for any more
    /// is cancelled (`send_due_lookups`): an editor goes in many ways.
    pub lookups: Vec<(SessionId, RequestId)>,
```

In `App::new`, beside `closed_editors: Vec::new(),`:

```rust
            lookup_delay: crate::key_picker::DELAY,
            lookups: Vec::new(),
```

Beside `mod editing;` at the top of `src/app.rs`:

```rust
mod key_picker;
```

- [ ] **Step 6: The reducer**

Create `src/app/key_picker.rs`:

```rust
//! The reducer's part in a foreign key cell's picker: it opens with the
//! editor, sends its searches and reads their answers. What it decides is
//! in `crate::key_picker`.

use std::time::{Duration, Instant};

use tabletist_db::{Error, LookupPage};

use super::App;
use crate::backend::{Command, RequestId, SessionId};
use crate::edit::{EditorPlace, Table};
use crate::key_picker::{self, Asked, Columns, Picker};
use crate::model::{ConnTabId, Fetch, TabId};

impl App {
    /// Gives the tab's editor, which just opened, its picker, where its
    /// cell is a foreign key's and the editor is the small field on the
    /// cell. `from_value` says it opened on the cell's value, not on
    /// something typed. The referenced table's columns are asked for if
    /// the workspace does not know them, and the first search is sent as
    /// soon as they are known.
    pub(super) fn open_picker(&mut self, tab: ConnTabId, id: TabId, from_value: bool) {
        let terminal = self.look.terminal;
        let made = self.workspace(tab).and_then(|workspace| {
            let object = workspace.object_tab(id)?;
            let table = Table::of(workspace, object)?;
            let editor = object.edits.editor.as_ref()?;
            // The row panel's field gets its picker in run 3.
            if editor.large || editor.place != EditorPlace::Grid {
                return None;
            }
            let column = table.column(editor.cell.col)?;
            let target = key_picker::target(table.structure?, &column.name)?;
            let class = table.class(editor.cell.col)?;
            // The row the cell points at now: what it loaded, or the
            // pending text the editor opened on.
            let pending = object.edits.cells.contains_key(&(editor.cell.row, editor.cell.col));
            let current = match (from_value, pending) {
                (false, _) => None,
                (true, true) => {
                    let text = editor.text.trim();
                    key_picker::pinned(text, class, &column.type_name, editor.problem.is_none())
                }
                (true, false) => table
                    .loaded(editor.cell)
                    .filter(|value| !value.is_null())
                    .cloned(),
            };
            let kept = workspace.columns.get(&target.0);
            let label = match kept.and_then(|kept| kept.value.as_deref()) {
                Some(columns) => {
                    Columns::Known(key_picker::label_column(table.dialect, columns, &target.1))
                }
                None => Columns::Waiting,
            };
            let failed = kept.and_then(|kept| kept.error.clone());
            let load = kept.is_none_or(Fetch::needs_load);
            let mut picker = Picker::new(target, label, current, !from_value, &editor.text);
            picker.failed = failed;
            Some((picker, load))
        });
        let Some((picker, load)) = made else {
            return;
        };
        let table = picker.target.clone();
        if let Some(editor) = self.editor_mut(tab, id) {
            editor.picker = Some(picker);
            // The terminal look keeps its own rule: `i` goes on from the
            // text's end.
            editor.select = from_value && !terminal;
        }
        if load {
            self.load_columns(tab, table);
        }
        self.search(tab, id);
    }

    /// Sends the search for what the tab's editor holds: the pinned row's
    /// read, where the text names one, then the matches'. What the search
    /// before it still has running is cancelled. Nothing is sent while the
    /// referenced table's columns are not known.
    pub(super) fn search(&mut self, tab: ConnTabId, id: TabId) {
        let planned = self.workspace(tab).and_then(|workspace| {
            let object = workspace.object_tab(id)?;
            let table = Table::of(workspace, object)?;
            let editor = object.edits.editor.as_ref()?;
            let picker = editor.picker.as_ref()?;
            let Columns::Known(label) = &picker.label else {
                return None;
            };
            let column = table.column(editor.cell.col)?;
            let class = table.class(editor.cell.col)?;
            let text = picker.text(&editor.text).to_owned();
            let pin = if !picker.searching {
                picker.current.clone()
            } else {
                let passes = crate::edit::problem(table.dialect, column, &text).is_none();
                key_picker::pinned(&text, class, &column.type_name, passes)
            };
            let reads = picker.lookups(label.clone(), &text, pin);
            Some((workspace.session, reads, text, picker.in_flight()))
        });
        let Some((session, (pinned, matching), text, stale)) = planned else {
            return;
        };
        self.lookups.retain(|(_, request)| !stale.contains(request));
        self.cancel(session, stale);
        let pinned = pinned.map(|lookup| self.send_lookup(session, lookup));
        let matching = Some(self.send_lookup(session, matching));
        if let Some(picker) = self
            .editor_mut(tab, id)
            .and_then(|editor| editor.picker.as_mut())
        {
            picker.asked = Some(Asked {
                text,
                pinned,
                matching,
                first: None,
            });
            picker.due = None;
        }
    }

    fn send_lookup(&mut self, session: SessionId, lookup: tabletist_db::Lookup) -> RequestId {
        let request = RequestId(self.next_id());
        self.lookups.push((session, request));
        self.backend.send(Command::Lookup {
            session,
            request,
            lookup,
        });
        request
    }

    /// The columns of a table came, or failed to: a picker of `tab` that
    /// waited for them learns its label column and sends its first
    /// search, or says why it has no list.
    pub(super) fn picker_columns(&mut self, tab: ConnTabId) {
        let Some(workspace) = self.workspace_mut(tab) else {
            return;
        };
        let dialect = workspace.driver.dialect();
        let mut ready = Vec::new();
        // Split, so the tabs and the cache are borrowed side by side.
        let columns = std::mem::take(&mut workspace.columns);
        for object in workspace.object_tabs_mut() {
            let id = object.id;
            let Some(picker) = object
                .edits
                .editor
                .as_mut()
                .and_then(|editor| editor.picker.as_mut())
                .filter(|picker| picker.label == Columns::Waiting)
            else {
                continue;
            };
            let Some(kept) = columns.get(&picker.target) else {
                continue;
            };
            if let Some(known) = kept.value.as_deref() {
                picker.label =
                    Columns::Known(key_picker::label_column(dialect, known, &picker.key));
                ready.push(id);
            } else if let Some(error) = &kept.error {
                picker.failed = Some(error.clone());
            }
        }
        workspace.columns = columns;
        for id in ready {
            self.search(tab, id);
        }
    }

    /// A lookup answered. The picker that waits for it takes it; one that
    /// nothing waits for any more is dropped.
    pub(super) fn lookup_answered(
        &mut self,
        session: SessionId,
        request: RequestId,
        result: Result<LookupPage, Error>,
    ) {
        self.lookups.retain(|(_, sent)| *sent != request);
        let Some(tab) = self.tab_for_session(session) else {
            return;
        };
        let Some(workspace) = self.workspace_mut(tab) else {
            return;
        };
        for object in workspace.object_tabs_mut() {
            let Some(picker) = object
                .edits
                .editor
                .as_mut()
                .and_then(|editor| editor.picker.as_mut())
            else {
                continue;
            };
            if picker.in_flight().contains(&request) {
                picker.answered(request, result);
                return;
            }
        }
    }

    /// Sends the searches whose time has come, and stops the lookups no
    /// open picker waits for: their editor closed, or was dropped with its
    /// page. Returns how long until the next search is due, for the frame
    /// to come back then.
    pub fn send_due_lookups(&mut self, now: Instant) -> Option<Duration> {
        let mut due = Vec::new();
        let mut next: Option<Duration> = None;
        let mut waited = Vec::new();
        for (tab, workspace) in self.open_connections() {
            for object in workspace.object_tabs() {
                let Some(picker) = object
                    .edits
                    .editor
                    .as_ref()
                    .and_then(|editor| editor.picker.as_ref())
                else {
                    continue;
                };
                waited.extend(picker.in_flight());
                match picker.due {
                    Some(at) if at <= now => due.push((tab, object.id)),
                    Some(at) => next = Some(next.map_or(at - now, |soonest| soonest.min(at - now))),
                    None => {}
                }
            }
        }
        let orphans: Vec<_> = self
            .lookups
            .iter()
            .filter(|(_, request)| !waited.contains(request))
            .copied()
            .collect();
        self.lookups.retain(|(_, request)| waited.contains(request));
        for (session, request) in orphans {
            self.cancel(session, [request]);
        }
        for (tab, id) in due {
            self.search(tab, id);
        }
        next
    }
}
```

Names to check against the tree before building: `Workspace::driver` and its `dialect()` (as `Table::of` reads them in `src/edit.rs`), `ObjectTab::id`, `App::open_connections` (it yields `(ConnTabId, &Workspace)`, see `tab_showing`), `App::cancel(session, requests)`, `App::load_columns(tab, object)` and `crate::model::Fetch`. `cancel` and `load_columns` are private to `impl App` in `src/app.rs`: a child module may call them.

- [ ] **Step 7: Call it**

In `src/app/editing.rs`, `edit_cell` ends with a `match opened { ... }` that puts the editor on the tab, and then sets the workspace's pane. The picker is opened after both, where nothing of the tab is borrowed. Before the `match`, note whether an editor opens:

```rust
        let is_open = opened.is_ok();
```

leave the `match` as it is, and make the function's last statement:

```rust
        if is_open {
            self.open_picker(tab, id, from_value);
        }
```

`from_value` is the function's own (near line 683). The two early `return`s of the `match` (no such cell, typing on a locked cell) return before it, as they should.

In `src/app.rs`:

1. `Action::EditorBreak`: inside its `if let Some(editor)`, after `editor.touched = true;`, add `editor.picker = None;` with the comment `// The large editor has no list.`
2. `Action::EditorTyped`: replace the arm with:

```rust
            Action::EditorTyped { tab, id } => {
                let problem = self.editor_problem(tab, id);
                let delay = self.lookup_delay;
                if let Some(editor) = self.editor_mut(tab, id) {
                    editor.problem = problem;
                    editor.touched = true;
                    // The row panel's field wraps, and Enter commits it: a
                    // line break can only be pasted into it. Its text is
                    // one of several lines from then on, edited as one.
                    if editor.place == EditorPlace::Panel && editor.text.contains('\n') {
                        editor.large = true;
                    }
                    if editor.large {
                        // The large editor has no list.
                        editor.picker = None;
                    } else if let Some(picker) = editor.picker.as_mut() {
                        picker.typed(&editor.text, std::time::Instant::now(), delay);
                    }
                }
            }
```

3. `apply_event`, the `Event::Structure` arm: the picker learns its columns after either branch filled the cache. The arm's body is one `if let Some(object) = waiting { ... } else if let Some(kept) = ... { ... }` chain (and whatever follows it). After the whole chain, as the arm's last statement, add:

```rust
                self.picker_columns(tab);
```

   `tab` is the arm's own (`let Some(tab) = self.tab_for_session(session)`). If the borrow of `workspace` is still alive there, end it by putting the chain in a block.

4. `apply_event`: replace run 1's empty `Event::Lookup { .. } => {}` arm (and its comment) with:

```rust
            Event::Lookup {
                session,
                request,
                result,
            } => self.lookup_answered(session, request, result),
```

5. `frame_ui`: the searches are sent before the keys and the view, and again after the frame's own actions. After the first `self.apply_actions();` and after the second, add the same line:

```rust
        self.tick_lookups(ui.ctx());
```

   and beside `frame_ui`, in the same `impl App`:

```rust
    /// The foreign key picker's searches wait for a pause in the typing:
    /// the ones that are due are sent, and the frame comes back when the
    /// next one is.
    fn tick_lookups(&mut self, ctx: &egui::Context) {
        if let Some(wait) = self.send_due_lookups(std::time::Instant::now()) {
            ctx.request_repaint_after(wait);
        }
    }
```

- [ ] **Step 8: Run the tests**

Run: `~/.cargo/bin/cargo test --locked --lib key_picker`
Expected: PASS, the six tests of `key_picker_tests` and the twelve of `key_picker::tests`.

Then the whole suite: `~/.cargo/bin/cargo test --locked --workspace --all-targets`. Every test that was there passes unchanged: `Harness::book_covers` has no foreign key, so no editor of theirs gets a picker. If one fails, it built a structure with a one-column foreign key of its own (`src/ui/mod.rs` near line 11188 does, for `gd`): read what it does. A test that opens an editor on such a column now gets a `Describe` and two `Lookup` commands after its `EditCell`, which a test that counts the commands sent will see. Fix the count, and say so in the report.

- [ ] **Step 9: Run the four checks and the shots build, then commit**

The screenshots' Bookshop (`src/shots.rs`) has a one-column foreign key, `book_images.book_id` to `books`, and two of its scenes open an editor on it (`edit_field`, `row_form_field`). From this task the first of them has a picker that waits for the books' columns, so its picture shows the list as searching. Run 3 gives the scenes their answers. Nothing to do here but know it.

```bash
git add src/edit.rs src/app.rs src/app/editing.rs src/app/key_picker.rs src/testing.rs src/ui/mod.rs src/ui/key_picker_tests.rs && git commit -m "Open a picker with the editor of a foreign key cell

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 3: Enter takes the option, and the list is moved in

**Files:**
- Modify: `src/model.rs` (after `Action::CancelEdit`, near line 302)
- Modify: `src/app/key_picker.rs`
- Modify: `src/app.rs` (`Action::CommitEdit`, and three new arms beside it)
- Modify: `src/app/editing.rs` (`dropped_under_a_prompt`)
- Modify: `src/ui/key_picker_tests.rs`

- [ ] **Step 1: Write the failing tests**

Append to `src/ui/key_picker_tests.rs` (add `Advance` back to its `use` line, and `NewValue` to the `tabletist_db` one):

```rust
/// Makes the editor's text `text`, as typing does, and has the search
/// for it sent and answered.
fn type_text(harness: &mut Harness, tab: ConnTabId, id: TabId, text: &str) {
    set_text(harness, tab, id, text);
    harness.app.send_due_lookups(std::time::Instant::now());
    harness.answer_lookups(&PUBLISHERS);
}

/// The pending text of `publisher_id` in the page's row `row`.
fn pending(harness: &Harness, tab: ConnTabId, id: TabId, row: usize) -> Option<String> {
    let object = harness.app.workspace(tab)?.object_tab(id)?;
    match &object.edits.cells.get(&(row, PUBLISHER))?.new {
        NewValue::Text(text) => Some(text.clone()),
        _ => None,
    }
}

fn commit(harness: &mut Harness, tab: ConnTabId, id: TabId, then: Advance) {
    harness.app.apply(Action::CommitEdit { tab, id, then });
}

fn selection(harness: &Harness, tab: ConnTabId, id: TabId) -> (usize, usize) {
    let object = harness.app.workspace(tab).unwrap().object_tab(id).unwrap();
    let cell = object.selection.unwrap();
    (cell.row, cell.col)
}

/// Test 11: the first option of `harb` is highlighted, and Enter makes
/// the cell pending with its key. Tab does the same and moves right.
#[test]
fn enter_takes_the_highlighted_option() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    edit(&mut harness, tab, id, 0, EditStart::Typed("h".into()));
    type_text(&mut harness, tab, id, "harb");
    assert_eq!(listed(&harness, tab, id), [HARBOR_PRESS, HARBOR_LIGHTS]);
    commit(&mut harness, tab, id, Advance::Down);
    assert!(!editing(&harness, tab, id));
    assert_eq!(pending(&harness, tab, id, 0).as_deref(), Some(HARBOR_PRESS));
    assert_eq!(selection(&harness, tab, id), (1, PUBLISHER));

    edit(&mut harness, tab, id, 1, EditStart::Typed("l".into()));
    type_text(&mut harness, tab, id, "lights");
    commit(&mut harness, tab, id, Advance::Right);
    assert_eq!(pending(&harness, tab, id, 1).as_deref(), Some(HARBOR_LIGHTS));
    assert_eq!(selection(&harness, tab, id), (1, PUBLISHER + 1));
}

/// Test 12: the arrows choose, and a click takes its option.
#[test]
fn the_arrows_and_a_click_choose_an_option() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    edit(&mut harness, tab, id, 0, EditStart::Typed("h".into()));
    type_text(&mut harness, tab, id, "harb");
    harness.app.apply(Action::PickerMove { tab, id, by: 1 });
    commit(&mut harness, tab, id, Advance::Down);
    assert_eq!(pending(&harness, tab, id, 0).as_deref(), Some(HARBOR_LIGHTS));

    // Row 1 points at Harbor Press: Harbor Lights is a change.
    edit(&mut harness, tab, id, 1, EditStart::Typed("h".into()));
    type_text(&mut harness, tab, id, "harb");
    harness.app.apply(Action::PickOption { tab, id, index: 1 });
    assert!(!editing(&harness, tab, id));
    assert_eq!(pending(&harness, tab, id, 1).as_deref(), Some(HARBOR_LIGHTS));
    // A click moves no selection.
    assert_eq!(selection(&harness, tab, id), (1, PUBLISHER));
}

/// Test 10's other half: an editor opened on its value and left with
/// Enter changes nothing, and an option moved to from there is taken.
#[test]
fn an_editor_that_was_only_opened_takes_nothing() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    edit(&mut harness, tab, id, 1, EditStart::Value);
    commit(&mut harness, tab, id, Advance::Stay);
    assert!(!editing(&harness, tab, id));
    assert_eq!(pending(&harness, tab, id, 1), None);

    edit(&mut harness, tab, id, 1, EditStart::Value);
    // From Harbor Press, the current row, one down: Northlight Books.
    harness.app.apply(Action::PickerMove { tab, id, by: 1 });
    commit(&mut harness, tab, id, Advance::Stay);
    assert_eq!(pending(&harness, tab, id, 1).as_deref(), Some(NORTHLIGHT));
}

/// Test 14: a text that only begins keys highlights nothing, and Enter
/// keeps the text.
#[test]
fn a_text_that_only_begins_keys_is_kept_as_typed() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    edit(&mut harness, tab, id, 0, EditStart::Typed("9".into()));
    type_text(&mut harness, tab, id, "91");
    assert_eq!(listed(&harness, tab, id).len(), 3);
    assert_eq!(picker(&harness, tab, id).unwrap().highlight, None);
    commit(&mut harness, tab, id, Advance::Stay);
    assert_eq!(pending(&harness, tab, id, 0).as_deref(), Some("91"));
}

/// Test 15: an Enter sent before the answer takes the first option of
/// that answer. Typing after the Enter drops it. A full key and Enter is
/// applied by the pinned row's answer alone.
#[test]
fn an_enter_before_the_answer_waits_for_it() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    edit(&mut harness, tab, id, 0, EditStart::Typed("l".into()));
    // The list is for `l` (Northlight, Harbor Lights): `harb` is typed and
    // Enter pressed before anything of it is sent.
    assert_eq!(listed(&harness, tab, id), [NORTHLIGHT, HARBOR_LIGHTS]);
    set_text(&mut harness, tab, id, "harb");
    let before = harness.lookups().len();
    commit(&mut harness, tab, id, Advance::Down);
    assert!(editing(&harness, tab, id), "the commit did not wait");
    // The search went out with the Enter, not 150 ms later.
    assert!(harness.lookups().len() > before);
    harness.answer_lookups(&PUBLISHERS);
    assert!(!editing(&harness, tab, id));
    assert_eq!(pending(&harness, tab, id, 0).as_deref(), Some(HARBOR_PRESS));

    // Typing after the Enter drops it.
    edit(&mut harness, tab, id, 1, EditStart::Typed("l".into()));
    set_text(&mut harness, tab, id, "harb");
    commit(&mut harness, tab, id, Advance::Down);
    set_text(&mut harness, tab, id, "harbo");
    harness.app.send_due_lookups(std::time::Instant::now());
    harness.answer_lookups(&PUBLISHERS);
    assert!(editing(&harness, tab, id));
    assert_eq!(picker(&harness, tab, id).unwrap().kept, None);
    harness.app.apply(Action::CancelEdit { tab, id });

    // A full key: the pinned row's answer is enough. Row 2 points at
    // Harbor Lights, so Northlight Books is a change.
    edit(&mut harness, tab, id, 2, EditStart::Typed("9".into()));
    set_text(&mut harness, tab, id, NORTHLIGHT);
    commit(&mut harness, tab, id, Advance::Stay);
    let sent = harness.lookups();
    let (session, request, lookup) = sent[sent.len() - 2].clone();
    assert!(matches!(lookup.part, LookupPart::Pinned(_)));
    harness.app.apply(Action::Backend(crate::backend::Event::Lookup {
        session,
        request,
        result: Ok(crate::testing::fake_lookup(&lookup, &PUBLISHERS)),
    }));
    assert!(!editing(&harness, tab, id));
    assert_eq!(pending(&harness, tab, id, 2).as_deref(), Some(NORTHLIGHT));
}

/// Spec 4 and 5: what the column would say of a search's text is not
/// shown while it is a search. A commit that takes the text and is
/// refused shows it, until the text changes.
#[test]
fn a_search_is_no_mistake_until_a_commit_is_refused() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    edit(&mut harness, tab, id, 0, EditStart::Typed("z".into()));
    type_text(&mut harness, tab, id, "zzz");
    // An integer column refuses `zzz`, and nothing matches it.
    let object = harness.app.workspace(tab).unwrap().object_tab(id).unwrap();
    assert!(object.edits.editor.as_ref().unwrap().problem.is_some());
    assert!(!picker(&harness, tab, id).unwrap().refused);
    commit(&mut harness, tab, id, Advance::Down);
    assert!(editing(&harness, tab, id), "a text the column refuses was kept");
    assert!(picker(&harness, tab, id).unwrap().refused);
    set_text(&mut harness, tab, id, "zz");
    assert!(!picker(&harness, tab, id).unwrap().refused);
}

/// Test 18: Set NULL closes the editor without its text and makes the
/// cell NULL, where the column takes one.
#[test]
fn set_null_drops_the_text_and_sets_null() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    edit(&mut harness, tab, id, 0, EditStart::Typed("harb".into()));
    harness.app.apply(Action::PickerNull { tab, id });
    assert!(!editing(&harness, tab, id));
    // `publisher_id` of the fixture is NOT NULL (its `ColumnInfo` has
    // `nullable: false`): nothing is set, and the cell says why.
    let object = harness.app.workspace(tab).unwrap().object_tab(id).unwrap();
    assert!(object.edits.cells.is_empty());
    assert!(object.edits.why.is_some());
}

/// Test 20: what leaves the editor without a pick keeps the text as
/// typed, never the highlighted option.
#[test]
fn leaving_the_editor_keeps_the_text_and_not_the_option() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    edit(&mut harness, tab, id, 0, EditStart::Typed("9".into()));
    type_text(&mut harness, tab, id, HARBOR_LIGHTS);
    // Harbor Lights is highlighted: its key is the text. Another key is
    // typed over it and the keyboard goes elsewhere before any answer.
    set_text(&mut harness, tab, id, "42");
    let leave = crate::testing::leave_edit(&harness.app, tab, id);
    harness.app.apply(leave);
    assert!(!editing(&harness, tab, id));
    assert_eq!(pending(&harness, tab, id, 0).as_deref(), Some("42"));
}
```

The fixture's `publisher_id` cannot be NULL, so the nullable half of test 18 (the button shows, and sets NULL) is checked in task 4 on the view, with a structure whose column is nullable.

- [ ] **Step 2: Run them to see them fail**

Run: `~/.cargo/bin/cargo test --locked --lib key_picker_tests`
Expected: compile errors: no variants `PickerMove`, `PickOption`, `PickerNull` on `Action`.

- [ ] **Step 3: The actions**

In `src/model.rs`, in `enum Action`, after `CancelEdit`:

```rust
    /// Move the highlight of the open editor's foreign key picker `by`
    /// options: the arrows, Page Up and Page Down, the terminal look's
    /// Ctrl+N and Ctrl+P.
    PickerMove {
        tab: ConnTabId,
        id: TabId,
        by: isize,
    },
    /// Take the option at `index` of that picker's list as the cell's new
    /// value: a click on it.
    PickOption {
        tab: ConnTabId,
        id: TabId,
        index: usize,
    },
    /// The picker's Set NULL: drop the editor's text and make the cell
    /// NULL, where its column takes one.
    PickerNull {
        tab: ConnTabId,
        id: TabId,
    },
```

In `src/app/editing.rs`, `dropped_under_a_prompt`: add the three to the list, after `| Action::CancelEdit { .. }`:

```rust
            | Action::PickerMove { .. }
            | Action::PickOption { .. }
            | Action::PickerNull { .. }
```

Build once: the compiler names every other exhaustive `match` over `Action` (`src/ui/write_prompts.rs` words a held action near line 105; a function there may name every action). Give the three the treatment `CancelEdit` or `CommitEdit` gets in each place.

- [ ] **Step 4: What a commit takes**

Add to `impl App` in `src/app/key_picker.rs` (and `ColumnClass` to its `tabletist_db` imports, `Advance` and `Action` to its `crate::model` ones, `Take` to its `crate::key_picker` ones, `start_text` to its `crate::edit` ones):

```rust
    /// What a commit of the tab's editor takes, where the editor has a
    /// picker: the highlighted option's key becomes the editor's text, or
    /// the text stays as typed. Returns false when the commit has to wait
    /// for the list to answer the text: it is kept, the search is sent at
    /// once, and the answer applies it.
    pub(super) fn picker_takes(&mut self, tab: ConnTabId, id: TabId, then: Advance) -> bool {
        let class = self
            .table(tab, id, |table, object| {
                let editor = object.edits.editor.as_ref()?;
                table.class(editor.cell.col)
            })
            .flatten()
            .unwrap_or(ColumnClass::Other);
        let Some(editor) = self.editor_mut(tab, id) else {
            return true;
        };
        let Some(picker) = editor.picker.as_mut() else {
            return true;
        };
        match picker.take(&editor.text) {
            Take::Text => true,
            Take::Option(index) => {
                if let Some(option) = picker.options.get(index) {
                    // As if its key had been typed: the same check, the
                    // same pending cell.
                    editor.text = start_text(&option.key, class);
                    editor.touched = true;
                }
                true
            }
            Take::Wait => {
                picker.kept = Some(then);
                if picker.due.take().is_some() {
                    self.search(tab, id);
                }
                false
            }
        }
    }

    /// The picker's Set NULL: the editor closes without its text, and the
    /// cell is made NULL where its column takes one. Set NULL does nothing
    /// while an editor is open, so the close comes first.
    pub(super) fn picker_null(&mut self, tab: ConnTabId, id: TabId) {
        let Some(place) = self.editor_mut(tab, id).map(|editor| editor.place) else {
            return;
        };
        self.apply(Action::CancelEdit { tab, id });
        self.set_null(tab, id, place);
    }
```

`App::table` is private to `src/app/editing.rs` (`fn table<T>`). Make it `pub(super) fn table<T>`: this module is its sibling.

In `lookup_answered`, a kept Enter is applied once the answer decides it. The commit is an action, and `self` is borrowed by the loop over the tabs: the loop only notes it. Replace the loop with:

```rust
        let mut commit = None;
        for object in workspace.object_tabs_mut() {
            let id = object.id;
            let Some(editor) = object.edits.editor.as_mut() else {
                continue;
            };
            let crate::edit::Editor { text, picker, .. } = editor;
            let Some(picker) = picker.as_mut() else {
                continue;
            };
            if !picker.in_flight().contains(&request) {
                continue;
            }
            picker.answered(request, result);
            // A kept Enter or Tab, once the answer says what it takes.
            let kept = picker
                .kept
                .filter(|_| !matches!(picker.take(text), Take::Wait));
            if let Some(then) = kept {
                picker.kept = None;
                commit = Some((id, then));
            }
            break;
        }
        if let Some((id, then)) = commit {
            self.apply(Action::CommitEdit { tab, id, then });
        }
```

`result` is moved into `answered` inside a loop: the `break` after it is what lets the compiler accept that. If it does not, find the object's id in a first pass over `workspace.object_tabs()` and take the answer in a second step with `workspace.object_tab_mut(id)`.

A kept key must not wait for ever when the referenced table's columns cannot be read. In `picker_columns`, where a picker is given `picker.failed = Some(error.clone());`, note its kept key too, and apply it after the loop as above:

```rust
            } else if let Some(error) = &kept.error {
                picker.failed = Some(error.clone());
                // Nothing will answer: a kept Enter takes the text.
                if let Some(then) = picker.kept.take() {
                    commits.push((id, then));
                }
            }
```

with `let mut commits = Vec::new();` beside `ready`, and after the `for id in ready { ... }`:

```rust
        for (id, then) in commits {
            self.apply(Action::CommitEdit { tab, id, then });
        }
```

- [ ] **Step 5: The arms**

In `src/app.rs`, the `Action::CommitEdit` arm gets one line at its start:

```rust
            Action::CommitEdit { tab, id, then } => {
                // A foreign key picker says what is taken: its highlighted
                // option, the text, or nothing until its list answers.
                if !self.picker_takes(tab, id, then) {
                    return;
                }
                let field = self.panel_field(tab, id);
                // ... the rest of the arm, unchanged ...
```

The arm goes on with `if self.close_editor(tab, id, false) { ... }`. Give that `if` an `else`: a text the column refused stays in the editor, and from now on the field says why.

```rust
                } else if let Some(picker) = self
                    .editor_mut(tab, id)
                    .and_then(|editor| editor.picker.as_mut())
                {
                    // The text was taken as a value and refused: what the
                    // column says of it shows, until the text changes.
                    picker.refused = true;
                }
```

and three arms after `Action::CancelEdit`:

```rust
            Action::PickerMove { tab, id, by } => {
                if let Some(picker) = self
                    .editor_mut(tab, id)
                    .and_then(|editor| editor.picker.as_mut())
                {
                    picker.step(by);
                }
            }
            Action::PickOption { tab, id, index } => {
                let chosen = self
                    .editor_mut(tab, id)
                    .and_then(|editor| editor.picker.as_mut())
                    .map(|picker| picker.choose(index))
                    .is_some();
                if chosen {
                    let then = Advance::Stay;
                    self.apply(Action::CommitEdit { tab, id, then });
                }
            }
            Action::PickerNull { tab, id } => self.picker_null(tab, id),
```

Check how the arm above returns: if `apply`'s `match` is the function's last expression, `return;` is right; if the arm must fall out of a block, wrap the rest of the arm in `if self.picker_takes(tab, id, then) { ... }` instead.

- [ ] **Step 6: Run the tests**

Run: `~/.cargo/bin/cargo test --locked --lib key_picker`
Expected: PASS, fourteen tests of `key_picker_tests` now.

`leaving_the_editor_keeps_the_text_and_not_the_option` passes without a change to `close_editor`: a `LeaveEdit` never asks the picker. If it fails with Harbor Lights' key pending, something made `LeaveEdit` go through `picker_takes`: it must not.

- [ ] **Step 7: Run the four checks, then commit**

```bash
git add src/model.rs src/app.rs src/app/editing.rs src/app/key_picker.rs src/ui/key_picker_tests.rs && git commit -m "Take the option a foreign key picker highlights

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

(Add `src/ui/write_prompts.rs` if step 3 touched it.)

---

### Task 4: The list under the cell

The list is drawn the way the completion list is (`src/ui/sql_complete.rs`): read that file's `show` first. Its three rules are this list's: one foreground `egui::Area` for every table's list, rows that are clicked and never focused, and a press anywhere on the list reported back so the field keeps the keyboard.

**Files:**
- Create: `src/ui/key_picker.rs`
- Modify: `src/ui/mod.rs` (`mod key_picker;`, beside `mod sql_complete;`)
- Modify: `src/ui/cell_editor.rs` (`Target`, `take_keyboard`, `field`)
- Modify: `src/ui/data_view.rs` (`editor_target`, and after `grid::show` near line 1464)
- Modify: `src/ui/keys.rs` (`handle`, near line 322)
- Modify: `src/ui/key_picker_tests.rs`

- [ ] **Step 1: Write the failing tests**

Append to `src/ui/key_picker_tests.rs` (add `Key` and `Modifiers` back to its `use` lines, and `use egui::accesskit::Role;`):

```rust
/// Types `text` on the selected cell, a frame at a time, and answers what
/// the picker asks.
fn type_on_cell(harness: &mut Harness, text: &str) {
    harness.frame(vec![egui::Event::Text(text.into())]);
    harness.settle();
    harness.answer_structure(publishers_structure());
    harness.settle();
    harness.answer_lookups(&PUBLISHERS);
    harness.settle();
}

fn select(harness: &mut Harness, tab: ConnTabId, id: TabId, row: usize) {
    let cell = CellPos {
        row,
        col: PUBLISHER,
    };
    harness.app.apply(Action::SelectCell { tab, id, cell });
    harness.settle();
}

/// Tests 9 and 10: the field and the list are named as the board names
/// them, and the head line says what is listed.
#[test]
fn the_field_and_the_list_are_named_for_the_table_they_search() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    select(&mut harness, tab, id, 0);
    type_on_cell(&mut harness, "harb");
    let tree = harness.settle();
    assert!(
        crate::testing::node(&tree, "Search publishers for publisher_id", Role::TextInput).is_some(),
        "{:?}",
        crate::testing::labels(&tree)
    );
    assert!(crate::testing::node(&tree, "Matching publishers", Role::ListBox).is_some());
    assert!(harness.has("publishers matching \"harb\" · ↑↓ choose · ↩ set"));
    assert!(
        crate::testing::node(&tree, "Harbor Press, 9100000000000000004", Role::ListBoxOption)
            .is_some()
    );
    assert!(harness.has("2 matches in publishers"));
    assert!(harness.has("Esc cancels"));
    // NOT NULL: no Set NULL.
    assert!(crate::testing::node(&tree, "Set NULL", Role::Button).is_none());
}

/// Test 11 through the keys: the arrows move the list, not the grid's
/// selection, and Enter takes.
#[test]
fn the_keys_reach_the_list_while_the_field_has_the_keyboard() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    select(&mut harness, tab, id, 0);
    type_on_cell(&mut harness, "harb");
    harness.press(Key::ArrowDown, Modifiers::NONE);
    assert_eq!(picker(&harness, tab, id).unwrap().highlight, Some(1));
    assert_eq!(selection(&harness, tab, id), (0, PUBLISHER));
    harness.press(Key::Enter, Modifiers::NONE);
    assert!(!editing(&harness, tab, id));
    assert_eq!(pending(&harness, tab, id, 0).as_deref(), Some(HARBOR_LIGHTS));
    assert_eq!(selection(&harness, tab, id), (1, PUBLISHER));
}

/// Test 12: a click on an option takes it, and the text typed is not
/// first kept as a value.
#[test]
fn a_click_on_an_option_takes_it() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    select(&mut harness, tab, id, 0);
    type_on_cell(&mut harness, "harb");
    harness.click("Harbor Lights, 9100000000000000007");
    assert!(!editing(&harness, tab, id));
    assert_eq!(pending(&harness, tab, id, 0).as_deref(), Some(HARBOR_LIGHTS));
}

/// Test 13, on macOS: Esc closes the list and leaves the cell as it was.
#[test]
fn esc_closes_the_list_without_a_change() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    select(&mut harness, tab, id, 0);
    type_on_cell(&mut harness, "harb");
    harness.press(Key::Escape, Modifiers::NONE);
    assert!(!editing(&harness, tab, id));
    assert_eq!(pending(&harness, tab, id, 0), None);
    let tree = harness.settle();
    assert!(crate::testing::node(&tree, "Matching publishers", Role::ListBox).is_none());
}

/// Test 19: nothing found, a failure, and more than the list holds each
/// have their line.
#[test]
fn the_list_says_when_it_has_nothing_failed_or_holds_more() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    select(&mut harness, tab, id, 0);
    type_on_cell(&mut harness, "zzz");
    assert!(harness.has("No publishers match \"zzz\""));
    harness.press(Key::Escape, Modifiers::NONE);

    select(&mut harness, tab, id, 0);
    harness.frame(vec![egui::Event::Text("h".into())]);
    harness.settle();
    harness.fail_lookup(tabletist_db::Error::query("permission denied for table publishers"));
    assert!(harness.has("permission denied for table publishers"));
    harness.press(Key::Escape, Modifiers::NONE);

    // Sixty publishers whose names hold an x: fifty are listed.
    let names: Vec<String> = (0..60).map(|index| format!("Annex {index}")).collect();
    let many: Vec<(i64, &str)> = names
        .iter()
        .enumerate()
        .map(|(index, name)| (index as i64 + 1, name.as_str()))
        .collect();
    select(&mut harness, tab, id, 0);
    harness.frame(vec![egui::Event::Text("x".into())]);
    harness.settle();
    harness.answer_lookups(&many);
    assert!(harness.has("First 50 shown. Keep typing to narrow."));
    assert!(harness.has("50 or more matches in publishers"));
}

/// Test 18, the nullable half: the foot offers Set NULL, and it sets
/// NULL.
#[test]
fn a_nullable_columns_list_sets_null() {
    let mut harness = Harness::new();
    harness.set_look(Look::macos());
    let tab = harness.connect_fake_as(false);
    harness.app.apply(Action::OpenObject {
        tab,
        object: tabletist_db::ObjectRef::new("main", "book_covers"),
        kind: tabletist_db::ObjectKind::Table,
        pin: true,
    });
    let mut structure = crate::testing::keyed_covers_structure();
    structure.columns[PUBLISHER].nullable = true;
    harness.answer_structure(structure);
    harness.answer_rows(crate::testing::book_covers_page(3));
    let id = harness.app.workspace(tab).unwrap().active_tab.unwrap();
    harness.app.workspace_mut(tab).unwrap().pane = Pane::Grid;
    select(&mut harness, tab, id, 0);
    type_on_cell(&mut harness, "harb");
    harness.click("Set NULL");
    assert!(!editing(&harness, tab, id));
    let object = harness.app.workspace(tab).unwrap().object_tab(id).unwrap();
    assert!(matches!(
        object.edits.cells.get(&(0, PUBLISHER)).map(|pending| &pending.new),
        Some(NewValue::Null)
    ));
}
```

`Role::TextInput` is the role egui gives a one-line text field. If the first test fails on the role alone, print the node with that label and use the role it has.

- [ ] **Step 2: Run them to see them fail**

Run: `~/.cargo/bin/cargo test --locked --lib key_picker_tests`
Expected: the six new tests FAIL: nothing is labelled "Search publishers for publisher_id" (the field is still "Edit publisher_id"), and no list is drawn.

- [ ] **Step 3: The field says it searches**

In `src/ui/cell_editor.rs`:

1. `pub struct Target` gets a field:

```rust
    /// The cell is a foreign key's, and its editor searches the table
    /// the key points at.
    pub search: Option<Search>,
```

   and beside `Target`:

```rust
/// What a foreign key cell's field says of its search.
pub struct Search {
    /// The table that is searched.
    pub table: String,
    /// How many options the list holds, as the terminal look counts them
    /// in the field: `2`, or `50+`. `None` before the first answer.
    pub count: Option<String>,
    /// The cell's column takes NULL: the list's foot offers Set NULL.
    pub nullable: bool,
    /// A commit took the text and the column refused it: the field shows
    /// why. Until then the text is a search, and the field says nothing
    /// of what the column would make of it.
    pub refused: bool,
}
```

2. Selecting the text. Beside `take_keyboard`:

```rust
/// As [`take_keyboard`], with the whole of `text` selected: what is typed
/// next takes its place.
fn take_keyboard_selected(ctx: &egui::Context, id: Id, text: &str) {
    take_keyboard(ctx, id, text);
    let mut state = egui::text_edit::TextEditState::load(ctx, id).unwrap_or_default();
    let range = CCursorRange::two(CCursor::new(0), CCursor::new(text.chars().count()));
    state.cursor.set_char_range(Some(range));
    state.store(ctx, id);
}
```

   In `field`, replace `if std::mem::take(&mut editor.focus) { take_keyboard(ui.ctx(), id, &editor.text); }` with:

```rust
    if std::mem::take(&mut editor.focus) {
        if std::mem::take(&mut editor.select) {
            take_keyboard_selected(ui.ctx(), id, &editor.text);
        } else {
            take_keyboard(ui.ctx(), id, &editor.text);
        }
    }
```

3. The name. In `field`, replace the line that builds `name` with:

```rust
    let name = match &target.search {
        // A placeholder each: a translation puts the names where they read.
        Some(search) => gettext(locale, "Search {table} for {column}")
            .replace("{table}", &display_safe(&search.table))
            .replace("{column}", &display_safe(&target.name)),
        None => format!("{} {}", gettext(locale, "Edit"), display_safe(&target.name)),
    };
```

4. The search icon, in the looks that are not the terminal's. The text starts after it. In `field`, where `inner` is computed from `rect.left() + pad`, make the left edge a variable first:

```rust
    // A foreign key's field has the magnifier of a search field at its
    // start, and its text begins after it.
    let lead = match &target.search {
        Some(_) if !look.terminal => SEARCH_LEAD,
        _ => 0.0,
    };
    let text_left = rect.left() + pad + lead;
```

   use `text_left` in place of `rect.left() + pad` in the two places `inner` uses it, and after the `TextEdit` is shown paint the icon:

```rust
    if lead > 0.0 {
        let place = Rect::from_center_size(
            pos2(rect.left() + pad + SEARCH_ICON / 2.0, center),
            vec2(SEARCH_ICON, SEARCH_ICON),
        );
        crate::theme::Icon::Search
            .image(palette.secondary, SEARCH_ICON)
            .paint_at(ui, place);
    }
```

5. What the column says of the text. `field` draws a red ring while `editor.problem` is set (`let failing = editor.problem.is_some();`) and says the problem under the cell (`say_under`). A search's text is no value yet: `harb` in a column of numbers would be red from its first letter, and the note would stand where the list is. For a foreign key's field both wait until a commit was refused:

```rust
    // A foreign key's text is a search until a commit takes it: what its
    // column would say of it shows only once it was refused.
    let said = target.search.as_ref().is_none_or(|search| search.refused);
    let failing = editor.problem.is_some() && said;
```

   and the note's condition becomes `if let (Some(problem), true, true) = (&editor.problem, target.hold, said)`. The list is under the cell, so that note goes over it: give `say_under` one more argument, `above: bool`, which makes it take the place above the cell (the `rect.top() - gap - size.y` it already has for a cell at the window's bottom) whatever room is below, and pass `target.search.is_some()`. Its other callers pass `false`.

   with, at the top of the file:

```rust
/// The magnifier in a foreign key cell's field, and the room it takes
/// with the gap after it.
const SEARCH_ICON: f32 = 12.0;
const SEARCH_LEAD: f32 = SEARCH_ICON + 6.0;
```

   `Icon::Search`, `.image(tint, size)` and `.paint_at(ui, rect)` are what `widgets::add_search` uses (`src/ui/widgets.rs` near line 528); `Icon` is `crate::theme::Icon`.

In `src/ui/data_view.rs`, `editor_target` builds the `Target`. Add the field:

```rust
        search: object
            .edits
            .editor
            .as_ref()
            .filter(|editor| editor.cell == cell)
            .and_then(|editor| editor.picker.as_ref())
            .map(|picker| cell_editor::Search {
                table: picker.target.name.clone(),
                count: picker.listed.as_ref().map(|_| {
                    if picker.more {
                        format!("{}+", crate::key_picker::LIMIT)
                    } else {
                        picker.options.len().to_string()
                    }
                }),
                nullable: table.column(cell.col).is_some_and(|column| column.nullable),
                refused: picker.refused,
            }),
```

Every other place that builds a `cell_editor::Target` (the compiler names them: `src/ui/row_form.rs` or `src/ui/row_panel.rs` build one for the panel through the same `editor_target`, and tests may build one by hand) gets `search: None` where it is built by hand.

- [ ] **Step 4: The list**

Create `src/ui/key_picker.rs`:

```rust
//! The list of a foreign key cell's picker: the rows of the table the key
//! points at, under the cell's field. It never takes the keyboard: the
//! field keeps it, and `ui::keys` routes the list's keys. Drawn as
//! `ui::sql_complete` draws the completion list.

use egui::accesskit::Role;
use egui::{CornerRadius, Id, Rect, Sense, Ui, UiBuilder, WidgetInfo, WidgetType, pos2, vec2};
use tabletist_db::{LookupRow, Value};

use crate::i18n::{Locale, gettext, ngettext};
use crate::key_picker::{Columns, LIMIT, Picker, VISIBLE, found_in};
use crate::theme::{Look, Palette};
use crate::typography::{Text, TextRole};
use crate::ui::format::{display_safe, plain_text};
use crate::ui::{sql_complete, widgets};

/// What the pointer did to the list this frame.
#[derive(Default)]
pub struct Shown {
    /// The list is pressed or was clicked, on an option or beside one:
    /// the press landed outside the field, which must keep the keyboard.
    pub pressed: bool,
    /// The option picked: by a primary click, or by a screen reader's.
    pub picked: Option<usize>,
    /// Set NULL was clicked.
    pub null: bool,
}

/// What the list is of.
pub struct Names<'a> {
    /// The table that is searched.
    pub table: &'a str,
    /// The cell's column takes NULL: the foot offers Set NULL.
    pub nullable: bool,
}

/// The list's measures in a look.
struct Shape {
    /// The padding round its lines.
    pad: egui::Vec2,
    /// An option's height: two lines of text with a label, one without.
    option: f32,
    /// A line of words: the head, the foot, a state.
    line: f32,
    inset: f32,
    corner: u8,
    row_corner: u8,
}

impl Shape {
    fn of(look: &Look, labelled: bool) -> Self {
        if look.terminal {
            Self {
                pad: vec2(0.0, 4.0),
                option: 24.0,
                line: 22.0,
                inset: 10.0,
                corner: 3,
                row_corner: 0,
            }
        } else {
            Self {
                pad: vec2(4.0, 4.0),
                option: if labelled { 38.0 } else { 26.0 },
                line: 24.0,
                inset: 8.0,
                corner: 8,
                row_corner: 5,
            }
        }
    }
}

/// The colour of what is said beside the options.
fn muted(look: &Look, palette: &Palette) -> egui::Color32 {
    if look.terminal {
        palette.dim
    } else {
        palette.secondary
    }
}

/// The one line the list says in place of options, or under them.
fn state_line(picker: &Picker, typed: &str, table: &str, locale: Locale) -> Option<(String, bool)> {
    let table = display_safe(table);
    if let Some(error) = &picker.failed {
        return Some((display_safe(&error.to_string()).into_owned(), true));
    }
    if picker.listed.is_none() {
        let text = gettext(locale, "Searching {table}…").replace("{table}", &table);
        return Some((text, false));
    }
    if picker.options.is_empty() {
        let text = picker.text(typed);
        let said = if text.is_empty() {
            gettext(locale, "No {table}").replace("{table}", &table)
        } else {
            gettext(locale, "No {table} match \"{text}\"")
                .replace("{table}", &table)
                .replace("{text}", &display_safe(text))
        };
        return Some((said, false));
    }
    picker.more.then(|| {
        let text = gettext(locale, "First {count} shown. Keep typing to narrow.")
            .replace("{count}", &LIMIT.to_string());
        (text, false)
    })
}

/// What an answer is announced as: "2 matches in publishers". The
/// table's name is not made to agree with the count: it is a name.
fn count_line(picker: &Picker, table: &str, locale: Locale) -> Option<String> {
    picker.listed.as_ref()?;
    let table = display_safe(table);
    let count = picker.options.len();
    Some(if count == 0 {
        gettext(locale, "No matches in {table}").replace("{table}", &table)
    } else if picker.more {
        gettext(locale, "{count} or more matches in {table}")
            .replace("{count}", &LIMIT.to_string())
            .replace("{table}", &table)
    } else {
        let plural = u32::try_from(count).unwrap_or(u32::MAX);
        ngettext(locale, "{count} match in {table}", "{count} matches in {table}", plural)
            .replace("{count}", &count.to_string())
            .replace("{table}", &table)
    })
}

/// The head line: what is listed, and the keys.
fn head_line(picker: &Picker, typed: &str, table: &str, locale: Locale) -> String {
    let table = display_safe(table);
    let keys = gettext(locale, "↑↓ choose · ↩ set");
    let text = picker.text(typed);
    if text.is_empty() {
        format!("{table} · {keys}")
    } else {
        let listed = gettext(locale, "{table} matching \"{text}\"")
            .replace("{table}", &table)
            .replace("{text}", &display_safe(text));
        format!("{listed} · {keys}")
    }
}

/// What an option is called for a screen reader: its label and its key.
fn option_name(option: &LookupRow) -> String {
    let key = plain_text(&option.key);
    match &option.label {
        Some(Value::Null) | None => display_safe(&key).into_owned(),
        Some(label) => format!("{}, {}", display_safe(&plain_text(label)), display_safe(&key)),
    }
}

/// Draws the list of `picker` at `anchor`, the cell its field `field` is
/// on, while the field holds `typed`.
pub fn show(
    ui: &Ui,
    picker: &Picker,
    typed: &str,
    anchor: Rect,
    field: Id,
    names: &Names<'_>,
    (look, palette, locale): (&Look, &Palette, Locale),
) -> Shown {
    let labelled = matches!(&picker.label, Columns::Known(Some(_)));
    let shape = Shape::of(look, labelled);
    let total = picker.options.len();
    let rows = total.min(VISIBLE);
    // The list follows the highlight: the keys scroll it.
    let first = picker
        .highlight
        .map_or(0, |at| (at + 1).saturating_sub(VISIBLE))
        .min(total.saturating_sub(rows));
    let state = state_line(picker, typed, names.table, locale);
    let head = !look.terminal;
    let foot = !look.terminal;
    let lines = usize::from(head) + usize::from(foot) + usize::from(state.is_some());
    // As wide as the cell, and wide enough for a label and a key.
    let width = anchor.width().max(280.0);
    let height = 2.0 * shape.pad.y + rows as f32 * shape.option + lines as f32 * shape.line;
    let size = vec2(width, height);
    let mut shown = Shown::default();
    // One area for the lists of every table: only the table on screen
    // has one, and egui keeps each area's place for good.
    let id = Id::new("key-picker");
    egui::Area::new(id)
        .order(egui::Order::Foreground)
        .fade_in(false)
        .constrain(false)
        .fixed_pos(sql_complete::place(anchor, size, ui.ctx().content_rect()))
        .show(ui.ctx(), |ui| {
            let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
            // A press between the options picks nothing, but it is the
            // list's: the field keeps the keyboard.
            let whole = ui.interact(rect, id.with("list"), Sense::CLICK);
            let list_name =
                gettext(locale, "Matching {table}").replace("{table}", &display_safe(names.table));
            whole.widget_info(|| WidgetInfo::labeled(WidgetType::Other, true, &list_name));
            ui.ctx().accesskit_node_builder(whole.id, |node| {
                node.set_role(Role::ListBox);
                node.set_size_of_set(total);
            });
            let clicked_over = ui.input(|input| input.pointer.any_click());
            let clicked_over = clicked_over && ui.rect_contains_pointer(rect);
            shown.pressed = whole.is_pointer_button_down_on() || whole.clicked() || clicked_over;
            let border = sql_complete::panel(ui, rect, shape.corner, look, palette);
            let inner = rect.shrink2(shape.pad);
            let muted = muted(look, palette);
            let line_role = TextRole::pick(look, TextRole::Shortcut, TextRole::OCaption);
            let mut top = inner.top();
            let left = inner.left() + shape.inset;
            if head {
                let text = head_line(picker, typed, names.table, locale);
                let y = top + shape.line / 2.0;
                widgets::paint_label(ui, left, y, Text::one(look, line_role, &text, muted));
                top += shape.line;
            }
            let options = UiBuilder::new()
                .max_rect(inner)
                .accessibility_parent(whole.id);
            let options = ui.new_child(options);
            for (slot, index) in (first..first + rows).enumerate() {
                let option = &picker.options[index];
                let row = Rect::from_min_size(
                    pos2(inner.left(), top + slot as f32 * shape.option),
                    vec2(inner.width(), shape.option),
                );
                let row_id = id.with(("option", index));
                // Clicked, never focused: an option that could take the
                // keyboard would take it from the field.
                let response = options.interact(row, row_id, Sense::CLICK);
                let selected = picker.highlight == Some(index);
                let name = option_name(option);
                response.widget_info(|| WidgetInfo::labeled(WidgetType::Other, true, &name));
                ui.ctx().accesskit_node_builder(row_id, |node| {
                    node.set_role(Role::ListBoxOption);
                    node.set_selected(selected);
                    node.set_position_in_set(index);
                    node.set_size_of_set(total);
                });
                let corner = CornerRadius::same(shape.row_corner);
                if selected {
                    ui.painter().rect_filled(row, corner, palette.selection);
                    // The keyboard stays in the field: its node says which
                    // option is the current one.
                    ui.ctx().accesskit_node_builder(field, |node| {
                        node.set_active_descendant(row_id.accesskit_id());
                    });
                } else if response.hovered() {
                    ui.painter().rect_filled(row, corner, palette.surface_hover);
                }
                let text = picker.text(typed);
                let current = picker.is_current(&option.key);
                paint_option(ui, row, option, text, current, &shape, (look, palette, locale));
                shown.pressed |= response.is_pointer_button_down_on() || response.clicked();
                if response.clicked() {
                    shown.picked = Some(index);
                }
            }
            top += rows as f32 * shape.option;
            if let Some((said, failed)) = &state {
                let color = if *failed { palette.danger } else { muted };
                let y = top + shape.line / 2.0;
                widgets::paint_label(ui, left, y, Text::one(look, line_role, said, color));
                top += shape.line;
            }
            if foot {
                let y = top + shape.line / 2.0;
                let cancels = gettext(locale, "Esc cancels");
                widgets::paint_label(ui, left, y, Text::one(look, line_role, &cancels, muted));
                if names.nullable {
                    let label = gettext(locale, "Set NULL");
                    let button = widgets::ButtonSpec::new(&label)
                        .padding(8.0)
                        .role(widgets::secondary(look));
                    let wide = button.width(ui, look);
                    let place = Rect::from_center_size(
                        pos2(inner.right() - shape.inset - wide / 2.0, y),
                        vec2(wide, shape.line - 4.0),
                    );
                    let response = button.show_at(ui, place, look, palette);
                    shown.pressed |= response.is_pointer_button_down_on() || response.clicked();
                    shown.null = response.clicked();
                }
            }
            if let Some(count) = count_line(picker, names.table, locale) {
                widgets::announce(ui, rect, &count);
            }
            ui.painter().add(border);
        });
    shown
}

/// One option. On macOS two lines: the label, its match heavier, and
/// under it the key, small and muted. In the terminal look one line: the
/// label, its match in the accent, then the key, muted. An option of a
/// table with no label column is its key alone. The row the cell points
/// at is marked at its end.
fn paint_option(
    ui: &Ui,
    row: Rect,
    option: &LookupRow,
    text: &str,
    current: bool,
    shape: &Shape,
    (look, palette, locale): (&Look, &Palette, Locale),
) {
    let muted = muted(look, palette);
    let left = row.left() + shape.inset;
    let mut right = row.right() - shape.inset;
    if current {
        let mark = if look.terminal {
            format!("· {}", gettext(locale, "current"))
        } else {
            "✓".to_owned()
        };
        let role = TextRole::pick(look, TextRole::Shortcut, TextRole::OCaption);
        let laid = Text::one(look, role, &mark, muted).layout(ui.ctx());
        right -= laid.paint_right(ui.painter(), right, row.center().y) + 8.0;
    }
    let room = (right - left).max(0.0);
    let clip = Rect::from_min_max(pos2(left, row.top()), pos2(right, row.bottom()));
    let painter = ui.painter().with_clip_rect(clip);
    // A label is words: the body face, its match heavier. The terminal
    // look writes everything in its one face, the match in the accent.
    let (plain, hit, hit_color) = if look.terminal {
        (TextRole::OCode, TextRole::OCode, palette.accent)
    } else {
        (TextRole::UiBody, TextRole::UiBodySemibold, palette.text)
    };
    let key_role = TextRole::pick(look, TextRole::MonoSecondary, TextRole::OCode);
    let key = plain_text(&option.key);
    let key = display_safe(&key);
    // A key that begins with the text has that start emphasised too.
    let key_hit = (!text.is_empty() && key.starts_with(text)).then_some(text.len());
    let key_text = || match key_hit {
        Some(end) => Text::new(look)
            .add(key_role, &key[..end], palette.text)
            .add(key_role, &key[end..], muted),
        None => Text::one(look, key_role, &key, muted),
    };
    let label = match &option.label {
        None => None,
        Some(Value::Null) => Some(Text::one(look, plain, "NULL", muted)),
        Some(value) => {
            let label = plain_text(value);
            let label = display_safe(&label);
            // One line: what a line break would start is not shown.
            let label = label.lines().next().unwrap_or_default();
            let whole = |shown: &str| match found_in(shown, text) {
                Some(found) => Text::new(look)
                    .add(plain, &shown[..found.start], palette.text)
                    .add(hit, &shown[found.clone()], hit_color)
                    .add(plain, &shown[found.end..], palette.text),
                None => Text::one(look, plain, shown, palette.text),
            };
            let width = |shown: &str| plain.width(ui.ctx(), look.faces, shown);
            Some(if width(label) <= room {
                whole(label)
            } else {
                // Cut, and drawn plain: see the plan's row e.
                let cut = crate::ui::grid::ellipsize(label, room, false, width);
                Text::one(look, plain, &cut, palette.text)
            })
        }
    };
    match (label, look.terminal) {
        (None, _) => {
            key_text().layout(ui.ctx()).paint_left(&painter, left, row.center().y);
        }
        (Some(label), true) => {
            let wide = label.layout(ui.ctx()).paint_left(&painter, left, row.center().y);
            key_text()
                .layout(ui.ctx())
                .paint_left(&painter, left + wide + 8.0, row.center().y);
        }
        (Some(label), false) => {
            let quarter = row.height() / 4.0;
            label
                .layout(ui.ctx())
                .paint_left(&painter, left, row.top() + quarter + 2.0);
            key_text()
                .layout(ui.ctx())
                .paint_left(&painter, left, row.bottom() - quarter - 1.0);
        }
    }
}
```

Names to check against the tree: `palette.selection`, `palette.surface_hover`, `palette.secondary`, `palette.dim`, `palette.danger`, `TextRole::pick`, `widgets::paint_label`, `widgets::announce`, `widgets::ButtonSpec` (`now_button` in `src/ui/cell_editor.rs` builds one the same way), `crate::ui::grid::ellipsize`, `ngettext`'s arguments (a `u32` count last, as `src/ui/cell_editor.rs` near line 1013 calls it). `sql_complete::place` is `pub`; `sql_complete::panel` is `pub(crate)`.

In `src/ui/mod.rs`, beside `mod sql_complete;`:

```rust
mod key_picker;
```

- [ ] **Step 5: Draw it at the cell**

In `src/ui/data_view.rs`, after `grid::show(...)` returns `output` and before the block that draws the large editor (`if large && let (Some(editor), Some(target)) = (editor, &target)`), add:

```rust
        // A foreign key cell's list, under the cell its field is on. A
        // press on it is the editor's own: the field keeps the keyboard,
        // and the frame does not read as an edit that was left.
        if !large
            && let (Some(editor), Some(target)) = (editor.as_deref(), &target)
            && let Some(picker) = &editor.picker
            && let Some(search) = &target.search
            && let Some(anchor) = output.editing_rect
        {
            let names = key_picker::Names {
                table: &picker.target.name,
                nullable: search.nullable,
            };
            let skin = (&look, &palette, locale);
            let shown =
                key_picker::show(ui, picker, &editor.text, anchor, target.id, &names, skin);
            if shown.pressed {
                ui.memory_mut(|memory| memory.request_focus(target.id));
                outcome.left = false;
                outcome.kept = false;
            }
            if let Some(index) = shown.picked {
                let id = object_tab;
                actions.push(Action::PickOption { tab, id, index });
            } else if shown.null {
                let id = object_tab;
                actions.push(Action::PickerNull { tab, id });
            }
        }
```

`use crate::ui::key_picker;` at the top of the file. `output.editing_rect` is what the large editor is anchored to. `editor` is the `Option<&mut Editor>` the function already holds: `editor.as_deref()` borrows it for the block and gives it back before the large editor's block moves it.

The `outcome` is read a few lines below (`if outcome.changed { ... }`), so the changes to `outcome.left` above are made in time.

- [ ] **Step 6: The keys**

In `src/ui/keys.rs`, `handle`: before `let mut actions = Vec::new();` (near line 320) add:

```rust
    // The table on screen has a foreign key cell's picker open, and its
    // field has the keyboard: the list's keys are read here, before the
    // shortcuts below and before the field.
    let picking = object.filter(|&(tab, id)| {
        let field = crate::ui::cell_editor::field_id(tab, id);
        let has = ctx.memory(|memory| memory.has_focus(field));
        has && app
            .workspace(tab)
            .and_then(|workspace| workspace.object_tab(id))
            .and_then(|object| object.edits.editor.as_ref())
            .is_some_and(|editor| editor.picker.is_some() && !editor.large)
    });
```

and inside `ctx.input_mut(|input| { ... })`, right after the completion list's block:

```rust
        // The foreign key picker's keys, by the same rule.
        if let Some((tab, id)) = picking {
            picker_keys(input, (tab, id), terminal, &mut actions);
        }
```

and beside `completion_keys`:

```rust
/// The keys that move in the list of a foreign key cell's picker, whose
/// field has the keyboard. Enter, Tab and Esc are the field's
/// (`cell_editor::ending_keys`): the reducer asks the picker what Enter
/// takes. The terminal's Ctrl+N and Ctrl+P are the list's as long as it
/// is open: they never reach New connection and Quick open.
fn picker_keys(
    input: &mut egui::InputState,
    (tab, id): (ConnTabId, TabId),
    terminal: bool,
    actions: &mut Vec<Action>,
) {
    let page = crate::key_picker::VISIBLE as isize;
    for (modifiers, key, by, taken) in [
        (Modifiers::NONE, Key::ArrowDown, 1, true),
        (Modifiers::NONE, Key::ArrowUp, -1, true),
        (Modifiers::NONE, Key::PageDown, page, true),
        (Modifiers::NONE, Key::PageUp, -page, true),
        (Modifiers::CTRL, Key::N, 1, terminal),
        (Modifiers::CTRL, Key::P, -1, terminal),
    ] {
        if taken {
            // One step for each press: a frame may hold several.
            let presses = take_press(input, modifiers, key);
            actions.extend((0..presses).map(|_| Action::PickerMove { tab, id, by }));
        }
    }
}
```

`object` in `handle` is the table on screen as `(ConnTabId, TabId)` (it is what `open` is worked out from, near line 176).

- [ ] **Step 7: Run the tests**

Run: `~/.cargo/bin/cargo test --locked --lib key_picker`
Expected: PASS, twenty tests of `key_picker_tests`.

Where a test of this task fails, find which of three things it is before changing code:
1. A name: print `crate::testing::labels(&tree)` and compare it with the string the test asks for, character for character (`·`, `↑↓`, `↩`, the straight quotes).
2. The click: `a_click_on_an_option_takes_it` clicks through AccessKit, which sends a click to the option's node. If the cell ends pending with `harb` and failing its check, the `LeaveEdit` won: the press was not reported (`shown.pressed`) or `outcome.left` was read before the list was drawn.
3. The keys: `the_keys_reach_the_list_while_the_field_has_the_keyboard` fails with the grid's selection moved if `picking` is false. Print whether the field has egui's focus on the frame of the press.

- [ ] **Step 8: Look at it**

Run the app on a database with a foreign key and open the picker. Check by eye, against the boards `macOS – Editing values, editors by type` (card 6) and `macOS – Inserting a row`: the list is under the cell and inside the window, an option is two lines, the match is heavier, the current row has its check, the foot has `Esc cancels`. Put what you saw, or that you could not run it, in the task's report.

- [ ] **Step 9: Run the four checks, then commit**

```bash
git add src/ui/key_picker.rs src/ui/mod.rs src/ui/cell_editor.rs src/ui/data_view.rs src/ui/keys.rs src/ui/key_picker_tests.rs && git commit -m "List the rows a foreign key cell can point at

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 5: The terminal look

In the Omarchy look the field keeps the cell, with a `>` prompt and the count of options; the list has no head and no foot; Esc closes the picker without a change, where on a text cell it keeps the edit; and the line at the bottom names the picker's keys.

**Files:**
- Modify: `src/ui/cell_editor.rs` (`ending_keys`, `leaving_keys`, `field`)
- Modify: `src/ui/workspace.rs` (`struct Editing` near line 1559, where `walks` is worked out near line 1767, the insert mode's keys near line 2064)
- Modify: `src/ui/key_picker_tests.rs`

- [ ] **Step 1: Write the failing tests**

Append to `src/ui/key_picker_tests.rs`:

```rust
/// Test 13, in the terminal look: Esc closes the picker without a change
/// (on a text cell it keeps the edit), and so does Ctrl+C. Ctrl+N and
/// Ctrl+P move the highlight and do nothing else.
#[test]
fn the_terminal_look_moves_with_ctrl_n_and_closes_with_esc() {
    let (mut harness, tab, id) = covers_in(Look::omarchy());
    select(&mut harness, tab, id, 0);
    harness.app.apply(Action::EditCell {
        tab,
        id,
        cell: CellPos {
            row: 0,
            col: PUBLISHER,
        },
        start: EditStart::Replace(String::new()),
    });
    harness.settle();
    type_on_cell(&mut harness, "harb");
    assert_eq!(picker(&harness, tab, id).unwrap().highlight, Some(0));
    let tabs = harness.app.tabs.len();
    // What a real Ctrl press reports: outside macOS Ctrl is the command
    // key too (as `ctrl()` of `src/ui/complete_tests.rs`).
    let ctrl = if cfg!(target_os = "macos") {
        Modifiers::CTRL
    } else {
        Modifiers::CTRL | Modifiers::COMMAND
    };
    harness.press(Key::N, ctrl);
    assert_eq!(picker(&harness, tab, id).unwrap().highlight, Some(1));
    harness.press(Key::P, ctrl);
    assert_eq!(picker(&harness, tab, id).unwrap().highlight, Some(0));
    // No new connection, no quick open.
    assert_eq!(harness.app.tabs.len(), tabs);
    assert!(harness.app.dialog.is_none());
    // The line at the bottom names the picker's keys. It is painted,
    // not announced: the whole of it is one piece of text.
    harness.settle();
    let keys = "ctrl+n/p move · enter set · esc close";
    assert!(
        harness.painted.iter().any(|(piece, _)| piece == keys),
        "{:?}",
        harness.painted
    );
    // No head line and no foot in this look.
    assert!(!harness.has("Esc cancels"));

    harness.press(Key::Escape, Modifiers::NONE);
    assert!(!editing(&harness, tab, id));
    assert_eq!(pending(&harness, tab, id, 0), None);
}

/// Spec 2: in the terminal look `i` goes on from the text's end, and
/// nothing is selected.
#[test]
fn the_terminal_look_selects_nothing_on_opening() {
    let (mut harness, tab, id) = covers_in(Look::omarchy());
    edit(&mut harness, tab, id, 1, EditStart::Value);
    let object = harness.app.workspace(tab).unwrap().object_tab(id).unwrap();
    assert!(!object.edits.editor.as_ref().unwrap().select);
}
```

`Look::omarchy()` is the terminal look (`src/theme.rs` near line 443). How a quick open or a new connection shows in `App` may differ from `dialog`: assert on whatever `Action::NewConnection` and `Action::OpenQuickOpen` change. The line's keys are painted with `paint_text_right` (`src/ui/workspace.rs` near line 2100), and a painted piece is the whole of a `Text`: the words above are what the closure of step 5 builds, spaces and dots included.

- [ ] **Step 2: Run them to see them fail**

Run: `~/.cargo/bin/cargo test --locked --lib key_picker_tests::the_terminal`
Expected: the first FAILS at the Esc: the editor closed and `harb` is pending and to fix (the terminal's Esc keeps an edit). The line's words are missing too. The second passes already (task 2).

- [ ] **Step 3: Esc closes a picker in every look**

In `src/ui/cell_editor.rs`, `ending_keys` and `leaving_keys` get one more argument, `picking: bool`, passed through:

```rust
fn ending_keys(
    ui: &Ui,
    has: bool,
    had: bool,
    terminal: bool,
    picking: bool,
    outcome: &mut Outcome,
) {
```

with its last line `leaving_keys(input, has || had, terminal, picking, outcome);`. In `leaving_keys`, the arm that takes the terminal's Esc ends with `outcome.left = true; outcome.kept = true;`. Replace those two lines with:

```rust
                if picking {
                    // The text of a foreign key's picker is a search, not
                    // a value: Esc closes it without a change, as the
                    // Components sheet gives a picker.
                    outcome.cancel = true;
                } else {
                    outcome.left = true;
                    outcome.kept = true;
                }
```

Callers: `field` passes `target.search.is_some()`; `in_panel` passes `false` until run 3; every other caller the compiler names passes `false`.

- [ ] **Step 4: The prompt and the count**

In `field`, the terminal look's lead is the prompt's width. Replace the `lead` of task 4 with:

```rust
    let prompt = "> ";
    let lead = match &target.search {
        Some(_) if look.terminal => role.width(ui.ctx(), look.faces, prompt),
        Some(_) => SEARCH_LEAD,
        None => 0.0,
    };
```

(`role` is the cell's text role, already a local of `field`; move the `lead` below the line that defines it if needed.) Paint the prompt where the icon is painted:

```rust
    if look.terminal && target.search.is_some() {
        Text::one(look, role, prompt, palette.dim)
            .layout(ui.ctx())
            .paint_left(ui.painter(), rect.left() + pad, center);
    }
```

The count goes where a text column's `27 / 200` goes: `field` paints that from `target.max_chars` at the field's right. After that block add:

```rust
    // The terminal look counts the list's options in the field.
    if let (true, Some(count)) = (
        look.terminal,
        target.search.as_ref().and_then(|search| search.count.as_ref()),
    ) {
        let laid = Text::one(look, widgets::secondary(look), count, palette.dim).layout(ui.ctx());
        right -= laid.paint_right(ui.painter(), right, center) + pad;
    }
```

- [ ] **Step 5: The line at the bottom**

In `src/ui/workspace.rs`:

1. `struct Editing` gets:

```rust
    /// Insert mode is in a foreign key cell's picker: the line names the
    /// list's keys in place of the editor's.
    picking: bool,
```

2. Where `walks` is worked out from `edits.editor` (near line 1767), work `picking` out beside it and put it in the `Editing` that is built there:

```rust
    let picking = edits
        .editor
        .as_ref()
        .is_some_and(|editor| editor.picker.is_some() && !editor.large);
```

3. In `status_line`, the insert mode's `keys` closure builds `esc normal · tab next`. Give it the picker's words first:

```rust
                let keys = || {
                    if editing.picking {
                        let moves = gettext(locale, "move");
                        let sets = gettext(locale, "set");
                        let closes = gettext(locale, "close");
                        return Text::new(&look)
                            .add(role, "ctrl+n/p", palette.text)
                            .space(role, " ")
                            .add(role, &moves, palette.dim)
                            .add(role, " · ", palette.dim)
                            .add(role, "enter", palette.text)
                            .space(role, " ")
                            .add(role, &sets, palette.dim)
                            .add(role, " · ", palette.dim)
                            .add(role, "esc", palette.text)
                            .space(role, " ")
                            .add(role, &closes, palette.dim);
                    }
                    // ... the closure's body as it is ...
                };
```

   The three words are made outside the closure if the borrow checker wants them to outlive it, as `normal` and `next` are.

- [ ] **Step 6: Run the tests**

Run: `~/.cargo/bin/cargo test --locked --lib key_picker`
Expected: PASS, twenty-two tests of `key_picker_tests`.

Then `~/.cargo/bin/cargo test --locked --lib` whole: the terminal look's own editing tests pass unchanged, since `picking` is false for every editor without a picker.

- [ ] **Step 7: Look at it**

Run the app in the Omarchy look (the settings' look, or however `docs/_guide/omarchy.md` says to choose it) on a table with a foreign key, press `cc` on such a cell and type. Compare with the board `Omarchy – Editing values, editors by type` (card 6): the `>` prompt, the count, one line to an option, the match in the accent, the accent border. Say what you saw in the report.

- [ ] **Step 8: Run the four checks, then commit**

```bash
git add src/ui/cell_editor.rs src/ui/workspace.rs src/ui/key_picker_tests.rs && git commit -m "Give the terminal look its foreign key picker

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 6: A new row's cell, and `gd` on a pending value

**Files:**
- Modify: `src/ui/insert_audit_tests.rs` (`INS-09b`, near line 436)
- Modify: `src/ui/key_picker_tests.rs`
- Modify: `src/app.rs` (`Action::FollowSelectedKey`, near line 536)

- [ ] **Step 1: Write the failing tests**

In `src/ui/insert_audit_tests.rs`, `typing_into_a_foreign_key_opens_its_picker` is ignored ("INS-09b: a foreign key column opens the text editor, not a picker") and is built on `covers_in`, whose `book_covers` has no foreign key. Move it to the keyed fixture and take its `ignore` away:

```rust
/// INS-09b. Spec 3: "A foreign key column opens the search picker on the
/// first keystroke."
#[test]
fn typing_into_a_foreign_key_opens_its_picker() {
    let mut harness = Harness::new();
    harness.set_look(Look::macos());
    let (tab, id) = harness.keyed_covers();
    harness.app.workspace_mut(tab).unwrap().pane = Pane::Grid;
    harness.settle();
    harness.press(Key::N, Modifiers::COMMAND);
    harness.settle();
    assert_eq!(selected(&harness, tab, id), Some((new_row(0), 1)));
    harness.frame(vec![egui::Event::Text("harb".into())]);
    harness.settle();
    assert!(harness.has("publishers matching \"harb\" · ↑↓ choose · ↩ set"));
}
```

Append to `src/ui/key_picker_tests.rs`:

```rust
/// Test 23: a new row's foreign key cell opens the list, and a pick fills
/// what was required.
#[test]
fn a_new_rows_cell_is_picked_into() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    harness.press(Key::N, Modifiers::COMMAND);
    harness.settle();
    let new = crate::edit::new_row(0);
    // Adding the row opened the editor on its first required cell,
    // `publisher_id`, on nothing: the table's first rows are listed.
    harness.answer_structure(publishers_structure());
    harness.answer_lookups(&PUBLISHERS);
    assert_eq!(
        listed(&harness, tab, id),
        [NORTHLIGHT, HARBOR_PRESS, HARBOR_LIGHTS]
    );
    assert_eq!(picker(&harness, tab, id).unwrap().highlight, None);
    assert!(!harness.app.lacking(tab, id).is_empty());
    type_text(&mut harness, tab, id, "harb");
    commit(&mut harness, tab, id, Advance::Stay);
    assert_eq!(pending(&harness, tab, id, new).as_deref(), Some(HARBOR_PRESS));
    assert!(harness.app.lacking(tab, id).is_empty());
}

/// Test 24: `gd` follows the value the cell shows, the pending one where
/// there is one.
#[test]
fn gd_follows_the_pending_value() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    edit(&mut harness, tab, id, 0, EditStart::Typed("h".into()));
    type_text(&mut harness, tab, id, "harb");
    commit(&mut harness, tab, id, Advance::Stay);
    harness.app.apply(Action::FollowSelectedKey {
        tab,
        object_tab: id,
    });
    let filter = harness
        .app
        .backend
        .sent
        .iter()
        .rev()
        .find_map(|command| match command {
            Command::FetchRows { query, .. } if query.object.name == "publishers" => {
                query.filters.first().cloned()
            }
            _ => None,
        })
        .expect("the publishers were asked for");
    assert_eq!((filter.column.as_str(), filter.value.as_str()), ("id", HARBOR_PRESS));
}
```

`App::lacking(tab, id)` is the columns a tab's new rows still need (`src/app/editing.rs` near line 374). If following a key from a tab that holds a pending change raises the leave prompt, the fetch is not sent: read `follow_foreign_key` (it guards the tab it opens, not the one it leaves) and adjust the assertion to what it does, keeping the point: the value that is followed is Harbor Press's.

- [ ] **Step 2: Run them**

Run: `~/.cargo/bin/cargo test --locked --lib picker`
Expected: `typing_into_a_foreign_key_opens_its_picker` and `a_new_rows_cell_is_picked_into` PASS already: `edit_cell` opens the picker for a new row's cell as for any (if the second fails, see step 3). `gd_follows_the_pending_value` FAILS: the filter's value is the loaded `9100000000000000001`.

- [ ] **Step 3: If a new row's cell has no picker**

`open_picker` reads the cell's column through `Table::column`, which a new row's cell has too, and its `current` is `table.loaded(cell)`, which is `None` for a new row: nothing to fix. If the test still fails, print `picker(&harness, tab, id).is_some()` right after the row is added: `add_row` in `src/app/editing.rs` opens the editor through `edit_cell`, so the picker opens there. If it builds its `Editor` by hand instead, call `self.open_picker(tab, id, true)` after it.

- [ ] **Step 4: `gd` reads what the cell shows**

In `src/app.rs`, `Action::FollowSelectedKey` reads the value from the page (`let value = page.rows.get(cell.row)?.get(cell.col)?;`) and follows it when it is not NULL. Make it read the pending value first:

```rust
                        // What the cell shows: its pending value, where it
                        // has one that waits for a save. A cell to fix, a
                        // pending NULL and what the database will make
                        // have nothing to follow.
                        let pending = object.edits.cells.get(&(cell.row, cell.col));
                        let value = match pending {
                            Some(pending) => match (&pending.new, &pending.state) {
                                (tabletist_db::NewValue::Text(text), crate::edit::State::Ready) => {
                                    Some(text.clone())
                                }
                                _ => None,
                            },
                            None => page
                                .rows
                                .get(cell.row)
                                .and_then(|row| row.get(cell.col))
                                .filter(|value| !value.is_null())
                                .map(crate::ui::format::plain_text),
                        }?;
```

and build the tuple from `value` (a `String` now) in place of `crate::ui::format::plain_text(value)`, without the `(!value.is_null()).then(...)` around it. `foreign.ref_columns.first().cloned().unwrap_or_default()` stays. A new row's cell is not in `page.rows`, and is followed through its pending value by the same code.

- [ ] **Step 5: Run the tests**

Run: `~/.cargo/bin/cargo test --locked --lib picker && ~/.cargo/bin/cargo test --locked --lib following`
Expected: PASS, and the tests of following a key that were there (`following_a_foreign_key_opens_the_target_filtered_to_the_row` and the UUID one in `src/ui/mod.rs`) pass unchanged.

- [ ] **Step 6: Run the four checks, then commit**

```bash
git add src/app.rs src/ui/insert_audit_tests.rs src/ui/key_picker_tests.rs && git commit -m "Pick into a new row, and follow a pending key

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 7: The website and the spec

**Files:**
- Modify: `docs/_guide/editing-data.md`
- Modify: `docs/_guide/omarchy.md`
- Modify: `docs/superpowers/specs/2026-10-10-foreign-key-editing-design.md`

- [ ] **Step 1: The guide**

In `docs/_guide/editing-data.md`, after the "Words that are not text" part of "Change a value" and before "Save", add:

```markdown
### Foreign keys

A cell whose column points at another table searches that table while you
type. Start typing a name, a title or an email, or the start of an id, and
the rows that match are listed under the cell with their ids.

| To | Press |
| --- | --- |
| Move in the list | Up, Down, Page Up, Page Down |
| Set the cell to the highlighted row | Enter, or Tab to move on |
| Take a row with the pointer | Click it |
| Leave the cell as it was | Esc |

The list is searched by the table's first text column and by its key. An id
you type or paste in full is taken as it is.

The first row is highlighted when what you typed is in its name or is its
id. When the rows only begin with what you typed in their ids, none is, and
Enter keeps your text: `12` stays 12 and does not become row 120.

Press Enter on a cell without typing to see the table's first rows, with
the row the cell points at now on top.

The picker is on the cells of the grid, in a new row too. A key of several
columns is edited as text.
```

In the same file, under "What cannot be edited" or wherever the page says what the row panel does, change nothing: the row panel gets the picker in the next run.

- [ ] **Step 2: The Omarchy page**

In `docs/_guide/omarchy.md`, in the table of "Editing", after the row for `Ctrl+C`:

```markdown
| Ctrl+N, Ctrl+P, Down, Up | Move in a foreign key cell's list |
| Enter, Tab | In that list: set the cell to the highlighted row |
| Esc | In that list: close it and leave the cell as it was |
```

- [ ] **Step 3: Check the site**

```bash
cd docs && bundle install && bin/check
```

Expected: the build and the checks pass, as `docs.yml` runs them. If Ruby or the bundle is not there, say so in the report: `docs.yml` checks the pull request.

- [ ] **Step 4: The spec's status**

In `docs/superpowers/specs/2026-10-10-foreign-key-editing-design.md`, add to the status paragraph at the top, after run 1's sentence:

```markdown
Run 2 (`docs/superpowers/plans/2026-10-10-foreign-key-editing-2-grid-picker.md`)
is built: a foreign key cell of the grid, on an existing or a new row,
searches and picks in both looks. Its plan's "What the spec asks, and what
gets built" says what of sections 2 to 5 the app does otherwise: the
Omarchy field stays on the cell, the wheel does not scroll the list, and a
label that is cut is drawn without its match.
```

- [ ] **Step 5: Commit**

```bash
git add docs/_guide/editing-data.md docs/_guide/omarchy.md docs/superpowers/specs/2026-10-10-foreign-key-editing-design.md && git commit -m "Say how a foreign key cell is edited

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

## What this leaves for the runs

- **Run 3** (`2026-10-10-foreign-key-editing-3-row-panel.md`): the row panel's field. `open_picker` refuses `EditorPlace::Panel` today, `in_panel` passes `picking: false`, and nothing draws a list in the panel.
- **Run 4** (`2026-10-10-foreign-key-editing-4-missing-row.md`): the warning for a key that names no row. `picker_takes` knows when a value was taken from the list, which run 4 needs: it does not ask about such a value.
- **A screenshot of the picker** for each look (`src/shots.rs`): with run 3, when the panel can show it too. The Bookshop of `src/shots.rs` has no `publisher_id` on its `book_covers` today.
- **Platform coverage.** The tests run headless and are the same on every platform. Say in the pull request on which platform the list was looked at, and that the others were only compiled.

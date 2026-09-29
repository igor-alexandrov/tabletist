# Batch 3: Browsing UI (MVP) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** The first usable Tabletist: in a connected tab, a sidebar tree of schemas and objects, object tabs (preview and pinned), a virtualized data grid with paging and server-side sort, a row panel showing every field of the selected row, and a Structure view, all on SQLite, plus a demo mode that opens the fixture.

**Architecture:** State lives in `Workspace` (tree, object tabs, row panel flag). Every fetch is a `Fetch<T>`: last good value, the `RequestId` in flight, last error; `Fetch::finish` accepts only the pending request, which drops stale results. Views stay pure: the grid, tree and panels read state and push `Action`s. Formatting of values is pure functions in `ui/format.rs`. The grid is a custom widget built on `virtual_rows`, with a header painted over the rows at the top of the scroll view (sticky) and resizable columns kept in egui temp memory.

**Tech Stack:** Batches 0 to 2; serde_json (JSON pretty-printing, already a dependency).

**Spec:** `docs/superpowers/specs/2026-09-27-tabletist-design.md` (sections 5.1, 5.2, 5.5 to 5.8, 5.10)

## Global Constraints

- Everything from earlier batches' Global Constraints still applies.
- Views never mutate `App` state except text a field is editing (tree filter) and view-local egui memory (column widths, row panel filter, expanded fields). Copying to the clipboard (`ctx.copy_text`) is a side effect views and `keys.rs` may do directly.
- Results are applied only through `Fetch::finish` with the pending `RequestId`; a superseded or unknown request is dropped. Paging and sorting clear the shown page at once, so old rows never appear under the new offset or sort.
- A preview tab (opened by a single click) is replaced by the next single click; double-click, sorting, paging, or selecting a cell pins it.
- Grid cells show one line truncated at 256 characters; NULL is drawn dimmed; numeric columns are right-aligned.
- The row panel shows the full value: JSON pretty-printed in monospace (up to 256 KiB), binary as size plus a hex dump of the first 4 KiB, text longer than 20 lines or 4,000 characters collapsed behind "Show all".
- Page size comes from `settings.page_size` (default 300).
- Accessible labels: sidebar object rows are labelled with the object name; object tabs with "`<name>` tab"; their close buttons "Close `<name>`"; grid headers with the column name; grid rows "Row `<n>`" (1-based, absolute); row panel copy buttons "Copy `<column>`".
- Run `cargo fmt --all` before every commit; code in this plan is not pre-wrapped to rustfmt's width.
- Never use em dashes. One topic per commit on `main`.

## Review Focus

1. **A page result arriving after the user already paged again, sorted, or closed the tab**: must be dropped; the grid must never show rows for a query it no longer displays. Test: Task 3 `a_superseded_page_result_is_dropped` and `a_result_for_a_closed_object_tab_is_ignored`.
2. **The selected cell falling outside a new, shorter page** (last page has 5 rows, selection was row 200): selection is clamped or cleared, never indexes out of bounds. Test: Task 3 `selection_is_clamped_to_a_shorter_page`.
3. **A value that is huge or multi-line** (a 10 MB text, a JSON document with newlines): the cell shows one short line; the row panel does not hang. Test: Task 1 `long_and_multiline_text_is_one_short_line`, `hex_dumps_stop_at_the_limit`, `huge_json_is_shown_as_it_is`, and Task 6 `a_huge_single_line_value_is_collapsed_in_the_row_panel`.
4. **Keyboard movement past the edges and on an empty page** (Up on row 0, End with no rows): no panic, selection stays in range or stays empty. Test: Task 3 `moving_the_selection_stays_in_range`.
5. **Reconnecting a tab that had objects open**: the tree reloads and the active object tab refetches on the new session instead of waiting forever on the dead one. Test: Task 3 `reconnecting_reloads_the_tree_and_the_active_tab` and `reconnecting_restarts_inactive_tabs_that_were_loading`.

---

## File Structure

```
src/model.rs            + Fetch, Tree, SchemaNode, TreeNode, TreeRow, ObjectTabId, ObjectTab,
                          ObjectView, CellPos, is_system_schema; Workspace fields; new Actions
src/app.rs              + tree loading, object tabs, paging, sort, selection, views, refresh,
                          cancel, copy_text; Connected triggers after_connect
src/ui/format.rs        value formatting (pure)
src/ui/grid.rs          the data grid widget
src/ui/sidebar.rs       the tree panel
src/ui/object_tabs.rs   the object tab bar
src/ui/data_view.rs     footer + grid + error states for the Data view
src/ui/structure.rs     the Structure view
src/ui/row_panel.rs     the right-hand row panel
src/ui/workspace.rs     composes the above
src/ui/keys.rs          object-tab, paging, refresh, cancel, selection, copy shortcuts
src/theme.rs            + Icon::ChevronUp
src/testing.rs          fake-connected workspace helpers, copy capture, run_until
src/entrypoint.rs       demo_setup opens the fixture
```

---

### Task 1: Value formatting

**Files:**
- Create: `src/ui/format.rs`
- Modify: `src/ui/mod.rs` (`pub mod format;`)

**Interfaces:**
- Consumes: `tabletist_db::{Value, ValueKind}`.
- Produces: `format::CELL_MAX_CHARS: usize = 256`, `format::HEX_LIMIT: usize = 4096`, `format::COLLAPSE_LINES: usize = 20`, `format::COLLAPSE_CHARS: usize = 4_000`; `cell_text(value: &Value) -> Cow<'_, str>`; `plain_text(value: &Value) -> String` (full value, for copying); `full_text(value: &Value, kind: ValueKind) -> String` (row panel); `pretty_json(text: &str) -> Option<String>`; `hex_dump(bytes: &[u8]) -> String`; `human_size(bytes: usize) -> String`; `compact_count(n: u64) -> String`; `range_label(offset: u64, shown: usize, has_more: bool, estimate: Option<u64>) -> Option<String>`; `tsv_row(row: &[Value]) -> String`; `elapsed(duration: Duration) -> String`.

- [ ] **Step 1: Write the failing tests**

Create `src/ui/format.rs` with only tests:

```rust
//! Turning values into text for the grid, the row panel and the clipboard.

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use tabletist_db::{Value, ValueKind};

    fn text(value: &str) -> Value {
        Value::Text(value.into())
    }

    #[test]
    fn scalars_render_plainly_and_null_says_so() {
        assert_eq!(cell_text(&Value::Null), "NULL");
        assert_eq!(cell_text(&Value::Bool(true)), "true");
        assert_eq!(cell_text(&Value::Int(-42)), "-42");
        assert_eq!(cell_text(&Value::Float(99.5)), "99.5");
        assert_eq!(cell_text(&text("Zoë 🚀")), "Zoë 🚀");
    }

    #[test]
    fn long_and_multiline_text_is_one_short_line() {
        let long = "x".repeat(10_000_000);
        let cell = cell_text(&text(&long));
        assert_eq!(cell.chars().count(), CELL_MAX_CHARS + 1);
        assert!(cell.ends_with('…'));
        assert_eq!(cell_text(&text("a\nb\tc\r\nd")), "a b c  d");
    }

    #[test]
    fn short_text_is_not_copied() {
        let value = text("borrowed");
        assert!(matches!(cell_text(&value), std::borrow::Cow::Borrowed(_)));
    }

    #[test]
    fn blobs_show_their_size_in_the_grid() {
        assert_eq!(cell_text(&Value::Bytes(vec![0; 3].into())), "BLOB · 3 B");
        assert_eq!(cell_text(&Value::Bytes(vec![0; 1536].into())), "BLOB · 1.5 KB");
    }

    #[test]
    fn plain_text_is_the_whole_value() {
        let long = "y".repeat(1000);
        assert_eq!(plain_text(&text(&long)), long);
        assert_eq!(plain_text(&Value::Bytes(vec![0x00, 0xff, 0x10].into())), "0x00ff10");
        assert_eq!(plain_text(&Value::Null), "NULL");
    }

    #[test]
    fn json_is_pretty_printed_and_invalid_json_is_left_alone() {
        let pretty = full_text(&text(r#"{"plan":"pro","tags":["a"]}"#), ValueKind::Json);
        assert_eq!(pretty, "{\n  \"plan\": \"pro\",\n  \"tags\": [\n    \"a\"\n  ]\n}");
        assert_eq!(full_text(&text("not json"), ValueKind::Json), "not json");
        assert_eq!(full_text(&text("{\"a\":1}"), ValueKind::Text), "{\"a\":1}");
    }

    #[test]
    fn huge_json_is_shown_as_it_is() {
        let huge = format!("[{}1]", "1,".repeat(200_000));
        assert_eq!(full_text(&text(&huge), ValueKind::Json), huge);
    }

    #[test]
    fn hex_dumps_have_offsets_and_sixteen_bytes_a_line() {
        let bytes: Vec<u8> = (0u8..20).collect();
        assert_eq!(
            hex_dump(&bytes),
            "00000000  00 01 02 03 04 05 06 07 08 09 0a 0b 0c 0d 0e 0f\n00000010  10 11 12 13"
        );
    }

    #[test]
    fn hex_dumps_stop_at_the_limit() {
        let full = full_text(&Value::Bytes(vec![0xab; 10 * 1024 * 1024].into()), ValueKind::Binary);
        assert!(full.starts_with("10.0 MB\n"));
        assert!(full.ends_with('…'));
        assert!(full.lines().count() <= HEX_LIMIT / 16 + 3);
    }

    #[test]
    fn sizes_and_counts_are_compact() {
        assert_eq!(human_size(512), "512 B");
        assert_eq!(human_size(2048), "2.0 KB");
        assert_eq!(human_size(5 * 1024 * 1024), "5.0 MB");
        assert_eq!(compact_count(999), "999");
        assert_eq!(compact_count(1_234), "1.2K");
        assert_eq!(compact_count(1_000), "1K");
        assert_eq!(compact_count(1_250_000), "1.3M");
        assert_eq!(compact_count(7_000_000_000), "7B");
    }

    #[test]
    fn range_labels_describe_the_page() {
        assert_eq!(range_label(0, 300, true, Some(1_200_000)).as_deref(), Some("1–300 of ~1.2M"));
        assert_eq!(range_label(0, 300, true, None).as_deref(), Some("1–300"));
        assert_eq!(range_label(300, 50, false, Some(10)).as_deref(), Some("301–350 of 350"));
        assert_eq!(range_label(0, 5, false, None).as_deref(), Some("1–5 of 5"));
        assert_eq!(range_label(0, 0, false, None), None);
    }

    #[test]
    fn tsv_rows_keep_one_line_per_row() {
        let row = vec![Value::Int(1), text("a\tb\nc"), Value::Null];
        assert_eq!(tsv_row(&row), "1\ta b c\tNULL");
    }

    #[test]
    fn elapsed_times_read_naturally() {
        assert_eq!(elapsed(Duration::from_millis(84)), "84 ms");
        assert_eq!(elapsed(Duration::from_millis(1234)), "1.2 s");
    }
}
```

Add `pub mod format;` to `src/ui/mod.rs`.

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test --lib format`
Expected: FAIL to compile: `cannot find function cell_text`.

- [ ] **Step 3: Implement**

Put above the tests:

```rust
use std::borrow::Cow;
use std::fmt::Write as _;
use std::time::Duration;

use tabletist_db::{Value, ValueKind};

/// Characters a grid cell shows before cutting the value off.
pub const CELL_MAX_CHARS: usize = 256;
/// Bytes of a binary value the row panel dumps as hex.
pub const HEX_LIMIT: usize = 4096;
/// Lines of a long value the row panel shows before "Show all".
pub const COLLAPSE_LINES: usize = 20;
/// Characters of a long value the row panel shows before "Show all", so a
/// huge single-line value is never laid out whole.
pub const COLLAPSE_CHARS: usize = 4_000;
/// JSON larger than this is shown as it is, not re-parsed every frame.
const PRETTY_JSON_MAX: usize = 256 * 1024;

/// One short line for a grid cell.
pub fn cell_text(value: &Value) -> Cow<'_, str> {
    match value {
        Value::Null => Cow::Borrowed("NULL"),
        Value::Bool(flag) => Cow::Borrowed(if *flag { "true" } else { "false" }),
        Value::Int(number) => Cow::Owned(number.to_string()),
        Value::Float(number) => Cow::Owned(number.to_string()),
        Value::Text(text) => one_line(text),
        Value::Bytes(bytes) => Cow::Owned(format!("BLOB · {}", human_size(bytes.len()))),
    }
}

fn one_line(text: &str) -> Cow<'_, str> {
    let too_long = text.chars().nth(CELL_MAX_CHARS).is_some();
    let breaks = text.contains(['\n', '\r', '\t']);
    if !too_long && !breaks {
        return Cow::Borrowed(text);
    }
    let mut line: String = text
        .chars()
        .take(CELL_MAX_CHARS)
        .map(|character| if matches!(character, '\n' | '\r' | '\t') { ' ' } else { character })
        .collect();
    if too_long {
        line.push('…');
    }
    Cow::Owned(line)
}

/// The whole value as text, for the clipboard. Binary becomes `0x` hex.
pub fn plain_text(value: &Value) -> String {
    match value {
        Value::Bytes(bytes) => {
            let mut hex = String::with_capacity(2 + bytes.len() * 2);
            hex.push_str("0x");
            for byte in bytes.iter() {
                let _ = write!(hex, "{byte:02x}");
            }
            hex
        }
        Value::Text(text) => text.to_string(),
        other => cell_text(other).into_owned(),
    }
}

/// The whole value as the row panel shows it.
pub fn full_text(value: &Value, kind: ValueKind) -> String {
    match value {
        Value::Text(text) if kind == ValueKind::Json && text.len() <= PRETTY_JSON_MAX => {
            pretty_json(text).unwrap_or_else(|| text.to_string())
        }
        Value::Bytes(bytes) => {
            let shown = &bytes[..bytes.len().min(HEX_LIMIT)];
            let mut text = format!("{}\n{}", human_size(bytes.len()), hex_dump(shown));
            if bytes.len() > HEX_LIMIT {
                text.push_str("\n…");
            }
            text
        }
        other => plain_text(other),
    }
}

pub fn pretty_json(text: &str) -> Option<String> {
    let parsed: serde_json::Value = serde_json::from_str(text).ok()?;
    serde_json::to_string_pretty(&parsed).ok()
}

/// `00000000  00 01 02 ...`, sixteen bytes a line.
pub fn hex_dump(bytes: &[u8]) -> String {
    let mut out = String::new();
    for (line, chunk) in bytes.chunks(16).enumerate() {
        if line > 0 {
            out.push('\n');
        }
        let _ = write!(out, "{:08x} ", line * 16);
        for byte in chunk {
            let _ = write!(out, " {byte:02x}");
        }
    }
    out
}

pub fn human_size(bytes: usize) -> String {
    const KB: f64 = 1024.0;
    let size = bytes as f64;
    if size < KB {
        format!("{bytes} B")
    } else if size < KB * KB {
        format!("{:.1} KB", size / KB)
    } else {
        format!("{:.1} MB", size / (KB * KB))
    }
}

/// `1.2K`, `3.4M`, `7B`, dropping a trailing `.0`.
pub fn compact_count(n: u64) -> String {
    let (value, suffix) = match n {
        0..=999 => return n.to_string(),
        1_000..=999_999 => (n as f64 / 1e3, "K"),
        1_000_000..=999_999_999 => (n as f64 / 1e6, "M"),
        _ => (n as f64 / 1e9, "B"),
    };
    // Round half away from zero first: `{:.1}` alone rounds 1.25 to 1.2.
    let text = format!("{:.1}", (value * 10.0).round() / 10.0);
    format!("{}{suffix}", text.trim_end_matches(".0"))
}

/// `1–300 of ~1.2M`, or `None` when the page is empty.
pub fn range_label(offset: u64, shown: usize, has_more: bool, estimate: Option<u64>) -> Option<String> {
    if shown == 0 {
        return None;
    }
    let first = offset + 1;
    let last = offset + shown as u64;
    let total = if !has_more {
        format!(" of {last}")
    } else if let Some(estimate) = estimate.filter(|estimate| *estimate >= last) {
        format!(" of ~{}", compact_count(estimate))
    } else {
        String::new()
    };
    Some(format!("{first}–{last}{total}"))
}

/// A row as tab-separated values on one line.
pub fn tsv_row(row: &[Value]) -> String {
    row.iter()
        .map(|value| plain_text(value).replace(['\t', '\n', '\r'], " "))
        .collect::<Vec<_>>()
        .join("\t")
}

pub fn elapsed(duration: Duration) -> String {
    if duration < Duration::from_secs(1) {
        format!("{} ms", duration.as_millis())
    } else {
        format!("{:.1} s", duration.as_secs_f64())
    }
}
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test --lib format`
Expected: PASS (13 tests).

- [ ] **Step 5: Commit**

```bash
git add src/ui/format.rs src/ui/mod.rs
git commit -m "Add value formatting for cells, the row panel and copying"
```

---

### Task 2: Tree model and loading schemas and objects

**Files:**
- Modify: `src/model.rs`, `src/app.rs`, `src/testing.rs`

**Interfaces:**
- Consumes: `backend::{Command, Event, RequestId}`, `tabletist_db::{ObjectInfo, ObjectKind, ObjectRef, Driver, Error}`.
- Produces:
  - `model::Fetch<T> { value: Option<T>, pending: Option<RequestId>, error: Option<Error> }` (`Debug`, `Default`), `start(&mut self, request)`, `finish(&mut self, request, result: Result<T, Error>) -> bool`, `is_loading(&self) -> bool`, `needs_load(&self) -> bool`.
  - `model::Tree { schemas: Fetch<Vec<String>>, nodes: HashMap<String, SchemaNode>, filter: String }` (`Default`), `Tree::visible_rows(&self, driver: Driver, show_system: bool) -> Vec<TreeRow>`, `Tree::object_info(&self, object: &ObjectRef) -> Option<&ObjectInfo>`.
  - `model::SchemaNode { expanded: bool, objects: Fetch<Vec<ObjectInfo>>, collapsed: HashSet<ObjectKind> }` (`Default`).
  - `model::TreeNode` (`Debug, Clone, PartialEq, Eq`): `Schema(String)`, `Group(String, ObjectKind)`, `Object(ObjectRef, ObjectKind)`.
  - `model::TreeRow { node: TreeNode, depth: u8, label: String, count: Option<usize>, expanded: Option<bool>, loading: bool, error: Option<String> }` (`Debug, Clone, PartialEq`).
  - `model::is_system_schema(driver: Driver, name: &str) -> bool`.
  - `Workspace` gains `tree: Tree`, `objects: Vec<ObjectTab>`, `active_object: Option<ObjectTabId>`, `row_panel: bool`, `pending_open: Option<(ObjectRef, ObjectKind)>` (ObjectTab types defined here too; behaviour in Task 3).
  - Actions: `ToggleSchema { tab: ConnTabId, schema: String }`, `ToggleGroup { tab: ConnTabId, schema: String, kind: ObjectKind }`, `RefreshTree(ConnTabId)`.
  - `App::after_connect(&mut self, tab: ConnTabId)`, `App::load_schemas(tab)`, `App::load_objects(tab, schema: &str)`, `App::expand_schema(tab, schema: &str)`.
  - `testing::Harness::connect_fake(&mut self) -> ConnTabId` (connects through the recording backend and answers schemas `["main"]` and objects `users` (table), `orders` (table), `active_users` (view)); `testing::last_sent(app) -> &Command`.

- [ ] **Step 1: Add the model types (with failing tree tests)**

Add to `src/model.rs`:

```rust
use std::collections::{HashMap, HashSet};

use tabletist_db::{Error, ObjectInfo, ObjectKind, ObjectRef, RowPage, RowQuery, SortDir, Structure};

/// Something being fetched: the last good value, the request in flight, and
/// the last error. Only the pending request's result is accepted, which is
/// how stale results are dropped.
#[derive(Debug)]
pub struct Fetch<T> {
    pub value: Option<T>,
    pub pending: Option<RequestId>,
    pub error: Option<Error>,
}

impl<T> Default for Fetch<T> {
    fn default() -> Self {
        Self {
            value: None,
            pending: None,
            error: None,
        }
    }
}

impl<T> Fetch<T> {
    pub fn start(&mut self, request: RequestId) {
        self.pending = Some(request);
        self.error = None;
    }

    /// Applies a result if it answers the pending request. Returns whether it did.
    pub fn finish(&mut self, request: RequestId, result: Result<T, Error>) -> bool {
        if self.pending != Some(request) {
            return false;
        }
        self.pending = None;
        match result {
            Ok(value) => {
                self.value = Some(value);
                self.error = None;
            }
            Err(error) => self.error = Some(error),
        }
        true
    }

    pub fn is_loading(&self) -> bool {
        self.pending.is_some()
    }

    /// Never loaded and not loading.
    pub fn needs_load(&self) -> bool {
        self.value.is_none() && self.pending.is_none() && self.error.is_none()
    }
}

/// The sidebar's state.
#[derive(Debug, Default)]
pub struct Tree {
    pub schemas: Fetch<Vec<String>>,
    pub nodes: HashMap<String, SchemaNode>,
    /// Typed into the sidebar's filter field.
    pub filter: String,
}

#[derive(Debug, Default)]
pub struct SchemaNode {
    pub expanded: bool,
    pub objects: Fetch<Vec<ObjectInfo>>,
    /// Groups the user folded (Tables, Views...).
    pub collapsed: HashSet<ObjectKind>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TreeNode {
    Schema(String),
    Group(String, ObjectKind),
    Object(ObjectRef, ObjectKind),
}

/// One visible line of the tree.
#[derive(Debug, Clone, PartialEq)]
pub struct TreeRow {
    pub node: TreeNode,
    pub depth: u8,
    /// Schema or object name; empty for groups (the view names the kind).
    pub label: String,
    /// Objects in a group.
    pub count: Option<usize>,
    /// `Some` for rows that fold.
    pub expanded: Option<bool>,
    pub loading: bool,
    pub error: Option<String>,
}

const KINDS: [ObjectKind; 3] = [ObjectKind::Table, ObjectKind::View, ObjectKind::MaterializedView];

/// Schemas the database keeps for itself, hidden unless the user asks.
pub fn is_system_schema(driver: Driver, name: &str) -> bool {
    match driver {
        Driver::Postgres => {
            matches!(name, "pg_catalog" | "information_schema")
                || name.starts_with("pg_toast")
                || name.starts_with("pg_temp_")
        }
        Driver::MySql => matches!(name, "mysql" | "sys" | "performance_schema" | "information_schema"),
        Driver::Sqlite => false,
    }
}

impl Tree {
    /// The lines to draw. A filter shows every loaded match, unfolded.
    pub fn visible_rows(&self, driver: Driver, show_system: bool) -> Vec<TreeRow> {
        let needle = self.filter.trim().to_lowercase();
        let filtering = !needle.is_empty();
        let mut rows = Vec::new();
        let Some(schemas) = &self.schemas.value else {
            return rows;
        };
        for schema in schemas {
            if !show_system && is_system_schema(driver, schema) {
                continue;
            }
            let node = self.nodes.get(schema);
            let expanded = node.is_some_and(|node| node.expanded);
            let matching: Vec<&ObjectInfo> = node
                .and_then(|node| node.objects.value.as_ref())
                .map(|objects| {
                    objects
                        .iter()
                        .filter(|object| !filtering || object.name.to_lowercase().contains(&needle))
                        .collect()
                })
                .unwrap_or_default();
            if filtering && matching.is_empty() {
                continue;
            }
            let open = expanded || filtering;
            rows.push(TreeRow {
                node: TreeNode::Schema(schema.clone()),
                depth: 0,
                label: schema.clone(),
                count: None,
                expanded: Some(open),
                loading: node.is_some_and(|node| node.objects.is_loading()),
                error: node.and_then(|node| node.objects.error.as_ref().map(ToString::to_string)),
            });
            if !open {
                continue;
            }
            for kind in KINDS {
                let items: Vec<&&ObjectInfo> = matching.iter().filter(|object| object.kind == kind).collect();
                if items.is_empty() {
                    continue;
                }
                let folded = !filtering && node.is_some_and(|node| node.collapsed.contains(&kind));
                rows.push(TreeRow {
                    node: TreeNode::Group(schema.clone(), kind),
                    depth: 1,
                    label: String::new(),
                    count: Some(items.len()),
                    expanded: Some(!folded),
                    loading: false,
                    error: None,
                });
                if folded {
                    continue;
                }
                for object in items {
                    rows.push(TreeRow {
                        node: TreeNode::Object(ObjectRef::new(schema.clone(), object.name.clone()), kind),
                        depth: 2,
                        label: object.name.clone(),
                        count: None,
                        expanded: None,
                        loading: false,
                        error: None,
                    });
                }
            }
        }
        rows
    }

    pub fn object_info(&self, object: &ObjectRef) -> Option<&ObjectInfo> {
        self.nodes
            .get(&object.schema)?
            .objects
            .value
            .as_ref()?
            .iter()
            .find(|info| info.name == object.name)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ObjectTabId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ObjectView {
    #[default]
    Data,
    Structure,
}

/// A cell in the current page: row index within the page, column index.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CellPos {
    pub row: usize,
    pub col: usize,
}

/// One open table or view.
#[derive(Debug)]
pub struct ObjectTab {
    pub id: ObjectTabId,
    pub object: ObjectRef,
    pub kind: ObjectKind,
    /// A preview tab (not pinned) is replaced by the next single click.
    pub pinned: bool,
    pub view: ObjectView,
    pub query: RowQuery,
    pub rows: Fetch<RowPage>,
    pub structure: Fetch<Structure>,
    pub selection: Option<CellPos>,
    pub estimated_rows: Option<u64>,
}

impl ObjectTab {
    pub fn new(id: ObjectTabId, object: ObjectRef, kind: ObjectKind, pinned: bool, page_size: u32, estimated_rows: Option<u64>) -> Self {
        Self {
            id,
            query: RowQuery::new(object.clone(), page_size),
            object,
            kind,
            pinned,
            view: ObjectView::Data,
            rows: Fetch::default(),
            structure: Fetch::default(),
            selection: None,
            estimated_rows,
        }
    }

    pub fn page(&self) -> Option<&RowPage> {
        self.rows.value.as_ref()
    }

    pub fn sort_of(&self, column: &str) -> Option<SortDir> {
        self.query.sort.iter().find(|sort| sort.column == column).map(|sort| sort.dir)
    }
}
```

Add to `Workspace`:

```rust
    pub tree: Tree,
    pub objects: Vec<ObjectTab>,
    pub active_object: Option<ObjectTabId>,
    /// Whether the row panel is open.
    pub row_panel: bool,
    /// An object to open as soon as the session connects (demo mode).
    pub pending_open: Option<(ObjectRef, ObjectKind)>,
```

and a helper:

```rust
impl Workspace {
    pub fn object_tab(&self, id: ObjectTabId) -> Option<&ObjectTab> {
        self.objects.iter().find(|tab| tab.id == id)
    }

    pub fn object_tab_mut(&mut self, id: ObjectTabId) -> Option<&mut ObjectTab> {
        self.objects.iter_mut().find(|tab| tab.id == id)
    }

    pub fn active_object_tab(&self) -> Option<&ObjectTab> {
        self.object_tab(self.active_object?)
    }
}
```

Add the tree actions to `Action`:

```rust
    ToggleSchema { tab: ConnTabId, schema: String },
    ToggleGroup { tab: ConnTabId, schema: String, kind: ObjectKind },
    RefreshTree(ConnTabId),
```

In `App::connect_tab` (Batch 2), initialise the new fields:

```rust
            tree: Tree::default(),
            objects: Vec::new(),
            active_object: None,
            row_panel: true,
            pending_open: None,
```

Add tree tests at the bottom of `src/model.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn info(name: &str, kind: ObjectKind) -> ObjectInfo {
        ObjectInfo { name: name.into(), kind, estimated_rows: None }
    }

    fn tree() -> Tree {
        let mut tree = Tree::default();
        tree.schemas.value = Some(vec!["public".into(), "billing".into(), "pg_catalog".into()]);
        let mut public = SchemaNode { expanded: true, ..SchemaNode::default() };
        public.objects.value = Some(vec![
            info("orders", ObjectKind::Table),
            info("users", ObjectKind::Table),
            info("active_users", ObjectKind::View),
        ]);
        tree.nodes.insert("public".into(), public);
        let mut billing = SchemaNode::default();
        billing.objects.value = Some(vec![info("invoices", ObjectKind::Table)]);
        tree.nodes.insert("billing".into(), billing);
        tree
    }

    fn labels(rows: &[TreeRow]) -> Vec<String> {
        rows.iter()
            .map(|row| match &row.node {
                TreeNode::Group(_, kind) => format!("{kind:?}({})", row.count.unwrap()),
                _ => row.label.clone(),
            })
            .collect()
    }

    #[test]
    fn expanded_schemas_show_groups_and_objects_and_system_schemas_hide() {
        let rows = tree().visible_rows(Driver::Postgres, false);
        assert_eq!(
            labels(&rows),
            ["public", "Table(2)", "orders", "users", "View(1)", "active_users", "billing"]
        );
        let with_system = tree().visible_rows(Driver::Postgres, true);
        assert!(labels(&with_system).contains(&"pg_catalog".to_owned()));
    }

    #[test]
    fn folded_groups_hide_their_objects() {
        let mut tree = tree();
        tree.nodes.get_mut("public").unwrap().collapsed.insert(ObjectKind::Table);
        assert_eq!(
            labels(&tree.visible_rows(Driver::Postgres, false)),
            ["public", "Table(2)", "View(1)", "active_users", "billing"]
        );
    }

    #[test]
    fn a_filter_shows_loaded_matches_in_every_schema_unfolded() {
        let mut tree = tree();
        tree.filter = "IN".into();
        assert_eq!(labels(&tree.visible_rows(Driver::Postgres, false)), ["billing", "Table(1)", "invoices"]);
        tree.filter = "nothing matches".into();
        assert!(tree.visible_rows(Driver::Postgres, false).is_empty());
    }

    #[test]
    fn system_schemas_are_per_driver() {
        assert!(is_system_schema(Driver::Postgres, "pg_toast_temp_1"));
        assert!(is_system_schema(Driver::MySql, "performance_schema"));
        assert!(!is_system_schema(Driver::Sqlite, "main"));
        assert!(!is_system_schema(Driver::Postgres, "public"));
    }

    #[test]
    fn fetch_accepts_only_the_pending_request() {
        let mut fetch: Fetch<u32> = Fetch::default();
        assert!(fetch.needs_load());
        fetch.start(RequestId(1));
        fetch.start(RequestId(2));
        assert!(!fetch.finish(RequestId(1), Ok(1)));
        assert!(fetch.is_loading());
        assert!(fetch.finish(RequestId(2), Ok(2)));
        assert_eq!(fetch.value, Some(2));
        fetch.start(RequestId(3));
        assert!(fetch.finish(RequestId(3), Err(Error::Cancelled)));
        assert_eq!(fetch.value, Some(2), "an error keeps the last good value");
        assert_eq!(fetch.error, Some(Error::Cancelled));
    }
}
```

- [ ] **Step 2: Write the failing reducer tests**

Add to `src/testing.rs`:

```rust
use crate::backend::{Command, Event};
use crate::connections::{ColorTag, ConnectionId, SavedConnection};
use crate::model::{Action, ConnTabId};
use tabletist_db::{ConnectSpec, Driver, ObjectInfo, ObjectKind};

pub fn last_sent(app: &App) -> &Command {
    app.backend.sent.last().expect("a command was sent")
}

impl Harness {
    /// Connects the active tab through the recording backend and answers the
    /// tree's first requests: schema `main` with `users`, `orders` and the
    /// view `active_users`.
    pub fn connect_fake(&mut self) -> ConnTabId {
        let saved = SavedConnection {
            id: ConnectionId::new(),
            name: "Fixture".into(),
            color: ColorTag::Blue,
            spec: ConnectSpec::sqlite("/tmp/fixture.db"),
        };
        let conn = saved.id.clone();
        self.app.connections.upsert(saved);
        let tab = self.app.active_tab_id();
        self.app.apply(Action::Connect { tab, conn });
        let Command::Connect { session, request, .. } = *last_sent(&self.app) else {
            panic!("expected Connect");
        };
        self.app.apply(Action::Backend(Event::Connected { session, request, driver: Driver::Sqlite }));
        let Command::ListSchemas { request, .. } = *last_sent(&self.app) else {
            panic!("expected ListSchemas");
        };
        self.app.apply(Action::Backend(Event::Schemas { session, request, result: Ok(vec!["main".into()]) }));
        let Command::ListObjects { request, .. } = *last_sent(&self.app) else {
            panic!("expected ListObjects for the default schema");
        };
        let objects = vec![
            ObjectInfo { name: "active_users".into(), kind: ObjectKind::View, estimated_rows: None },
            ObjectInfo { name: "orders".into(), kind: ObjectKind::Table, estimated_rows: Some(3) },
            ObjectInfo { name: "users".into(), kind: ObjectKind::Table, estimated_rows: Some(1_200_000) },
        ];
        self.app.apply(Action::Backend(Event::Objects {
            session,
            request,
            schema: "main".into(),
            result: Ok(objects),
        }));
        tab
    }
}
```

The `let Command::X { .. } = *last_sent(..)` pattern needs `Command` fields that are `Copy` for the ones bound; bind only `session` and `request` (both `Copy`) with `..`. If the compiler refuses to move out of the borrow, use `match last_sent(&self.app) { Command::Connect { session, request, .. } => (*session, *request), other => panic!("{other:?}") }` instead.

Add to the tests in `src/app.rs`:

```rust
    use crate::model::{TreeNode, TreeRow};
    use crate::testing::Harness;

    #[test]
    fn connecting_loads_schemas_then_expands_the_default_schema() {
        let mut harness = Harness::new();
        let tab = harness.connect_fake();
        let workspace = harness.app.workspace(tab).unwrap();
        assert_eq!(workspace.tree.schemas.value.as_deref(), Some(&["main".to_owned()][..]));
        assert!(workspace.tree.nodes["main"].expanded);
        let rows: Vec<TreeRow> = workspace.tree.visible_rows(Driver::Sqlite, false);
        assert!(rows.iter().any(|row| matches!(&row.node, TreeNode::Object(object, _) if object.name == "users")));
    }

    #[test]
    fn toggling_a_schema_folds_it_and_unfolding_does_not_reload() {
        let mut harness = Harness::new();
        let tab = harness.connect_fake();
        let sent = harness.app.backend.sent.len();
        harness.app.apply(Action::ToggleSchema { tab, schema: "main".into() });
        assert!(!harness.app.workspace(tab).unwrap().tree.nodes["main"].expanded);
        harness.app.apply(Action::ToggleSchema { tab, schema: "main".into() });
        assert!(harness.app.workspace(tab).unwrap().tree.nodes["main"].expanded);
        assert_eq!(harness.app.backend.sent.len(), sent, "loaded objects are reused");
    }

    #[test]
    fn toggling_a_group_folds_it() {
        let mut harness = Harness::new();
        let tab = harness.connect_fake();
        harness.app.apply(Action::ToggleGroup { tab, schema: "main".into(), kind: ObjectKind::Table });
        assert!(harness.app.workspace(tab).unwrap().tree.nodes["main"].collapsed.contains(&ObjectKind::Table));
    }

    #[test]
    fn refreshing_the_tree_reloads_schemas_and_expanded_objects() {
        let mut harness = Harness::new();
        let tab = harness.connect_fake();
        let before = harness.app.backend.sent.len();
        harness.app.apply(Action::RefreshTree(tab));
        let new: Vec<_> = harness.app.backend.sent[before..].iter().collect();
        assert!(new.iter().any(|c| matches!(c, Command::ListSchemas { .. })));
        assert!(new.iter().any(|c| matches!(c, Command::ListObjects { schema, .. } if schema == "main")));
    }

    #[test]
    fn a_stale_objects_result_is_ignored() {
        let mut harness = Harness::new();
        let tab = harness.connect_fake();
        harness.app.apply(Action::RefreshTree(tab));
        let session = harness.app.workspace(tab).unwrap().session;
        harness.app.apply(Action::Backend(Event::Objects {
            session,
            request: RequestId(1),
            schema: "main".into(),
            result: Ok(Vec::new()),
        }));
        let node = &harness.app.workspace(tab).unwrap().tree.nodes["main"];
        assert_eq!(node.objects.value.as_ref().unwrap().len(), 3, "old value kept");
    }

```

(The reconnect test needs `Action::OpenObject`, so it lives in Task 3.)

- [ ] **Step 3: Run the tests to verify they fail**

Run: `cargo test --lib`
Expected: FAIL (tree actions not handled; `connect_fake` panics at "expected ListSchemas").

- [ ] **Step 4: Implement tree loading**

In `src/app.rs`, make `Event::Connected` call `after_connect` for an adopted tab:

```rust
                match adopted {
                    Some(tab) => self.after_connect(tab),
                    // Nobody is waiting for this session any more.
                    None => self.backend.send(Command::Close { session }),
                }
```

Add the reducer arms:

```rust
            Action::ToggleSchema { tab, schema } => {
                let expanded = self
                    .workspace(tab)
                    .and_then(|workspace| workspace.tree.nodes.get(&schema))
                    .is_some_and(|node| node.expanded);
                if expanded {
                    if let Some(workspace) = self.workspace_mut(tab)
                        && let Some(node) = workspace.tree.nodes.get_mut(&schema)
                    {
                        node.expanded = false;
                    }
                } else {
                    self.expand_schema(tab, &schema);
                }
            }
            Action::ToggleGroup { tab, schema, kind } => {
                if let Some(workspace) = self.workspace_mut(tab) {
                    let node = workspace.tree.nodes.entry(schema).or_default();
                    if !node.collapsed.remove(&kind) {
                        node.collapsed.insert(kind);
                    }
                }
            }
            Action::RefreshTree(tab) => self.refresh_tree(tab),
```

and to `apply_event`:

```rust
            Event::Schemas { session, request, result } => {
                let Some(tab) = self.tab_for_session(session) else { return };
                let Some(workspace) = self.workspace_mut(tab) else { return };
                if !workspace.tree.schemas.finish(request, result) {
                    return;
                }
                let default = workspace.tree.schemas.value.as_ref().and_then(|schemas| {
                    schemas
                        .iter()
                        .find(|schema| *schema == "public" || *schema == "main")
                        .or(if schemas.len() == 1 { schemas.first() } else { None })
                        .cloned()
                });
                if let Some(schema) = default {
                    self.expand_schema(tab, &schema);
                }
            }
            Event::Objects { session, request, schema, result } => {
                if let Some(tab) = self.tab_for_session(session)
                    && let Some(workspace) = self.workspace_mut(tab)
                    && let Some(node) = workspace.tree.nodes.get_mut(&schema)
                {
                    node.objects.finish(request, result);
                }
            }
```

Add the helpers:

```rust
    /// After a (re)connect: load the tree, reload what was open, and open
    /// anything queued for this connection.
    pub fn after_connect(&mut self, tab: ConnTabId) {
        self.refresh_tree(tab);
        let Some(workspace) = self.workspace_mut(tab) else { return };
        let active = workspace.active_object;
        let pending = workspace.pending_open.take();
        if let Some(id) = active {
            self.fetch_rows(tab, id);
        }
        if let Some((object, kind)) = pending {
            self.open_object(tab, object, kind, true);
        }
    }

    pub fn load_schemas(&mut self, tab: ConnTabId) {
        let request = RequestId(self.next_id());
        let Some(workspace) = self.workspace_mut(tab) else { return };
        workspace.tree.schemas.start(request);
        let session = workspace.session;
        self.backend.send(Command::ListSchemas { session, request });
    }

    pub fn load_objects(&mut self, tab: ConnTabId, schema: &str) {
        let request = RequestId(self.next_id());
        let Some(workspace) = self.workspace_mut(tab) else { return };
        workspace.tree.nodes.entry(schema.to_owned()).or_default().objects.start(request);
        let session = workspace.session;
        self.backend.send(Command::ListObjects { session, request, schema: schema.to_owned() });
    }

    pub fn expand_schema(&mut self, tab: ConnTabId, schema: &str) {
        let Some(workspace) = self.workspace_mut(tab) else { return };
        let node = workspace.tree.nodes.entry(schema.to_owned()).or_default();
        node.expanded = true;
        if node.objects.needs_load() || node.objects.error.is_some() {
            self.load_objects(tab, schema);
        }
    }

    fn refresh_tree(&mut self, tab: ConnTabId) {
        self.load_schemas(tab);
        let expanded: Vec<String> = self
            .workspace(tab)
            .map(|workspace| {
                workspace
                    .tree
                    .nodes
                    .iter()
                    .filter(|(_, node)| node.expanded || node.objects.is_loading())
                    .map(|(name, _)| name.clone())
                    .collect()
            })
            .unwrap_or_default();
        for schema in expanded {
            self.load_objects(tab, &schema);
        }
    }
```

Add the imports these need to `src/app.rs`: `use crate::model::ObjectTabId;` and `use tabletist_db::{ObjectKind, ObjectRef};` (merged into the existing `use` lines).

`fetch_rows` and `open_object` come in Task 3; to compile this task, add them as stubs that do nothing (`fn fetch_rows(&mut self, _tab: ConnTabId, _id: ObjectTabId) {}` and `fn open_object(&mut self, _tab: ConnTabId, _object: ObjectRef, _kind: ObjectKind, _pin: bool) {}`), which Task 3 replaces.

- [ ] **Step 5: Run the tests to verify they pass**

Run: `cargo test --lib`
Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add src/model.rs src/app.rs src/testing.rs
git commit -m "Load the sidebar tree of schemas and objects"
```

---

### Task 3: Object tabs, paging, sorting, selection, views, refresh and cancel in the reducer

**Files:**
- Modify: `src/model.rs` (actions), `src/app.rs`, `src/testing.rs`

**Interfaces:**
- Consumes: Task 2's model.
- Produces:
  - Actions: `OpenObject { tab, object: ObjectRef, kind: ObjectKind, pin: bool }`, `ActivateObjectTab { tab, object_tab: ObjectTabId }`, `CloseObjectTab { tab, object_tab }`, `PinObjectTab { tab, object_tab }`, `CycleObjectTab { tab, step: isize }`, `SetView { tab, object_tab, view: ObjectView }`, `NextPage { tab, object_tab }`, `PrevPage { tab, object_tab }`, `SortBy { tab, object_tab, column: String }`, `SelectCell { tab, object_tab, cell: CellPos }`, `MoveSelection { tab, object_tab, rows: isize, cols: isize }`, `ToggleRowPanel(ConnTabId)`, `Refresh(ConnTabId)`, `CancelQuery(ConnTabId)`, `RetryRows { tab, object_tab }`, `RetryStructure { tab, object_tab }`.
  - `App::open_object(tab, object, kind, pin)`, `App::fetch_rows(tab, id)`, `App::describe(tab, id)`, `App::active_object(&self) -> Option<(ConnTabId, ObjectTabId)>`, `App::copy_text(&self, whole_row: bool) -> Option<String>`.
  - `testing::page(rows: usize, has_more: bool) -> RowPage` (columns `id` int4 Numeric, `email` text Text, `meta` json Json); `Harness::answer_rows(&mut self, page: RowPage)` (answers the newest `FetchRows`).

- [ ] **Step 1: Write the failing tests**

Add to `src/testing.rs`:

```rust
use tabletist_db::{ColumnMeta, RowPage, Value, ValueKind};

/// A page shaped like the fixture's users table.
pub fn page(rows: usize, has_more: bool) -> RowPage {
    RowPage {
        columns: vec![
            ColumnMeta { name: "id".into(), type_name: "INTEGER".into(), kind: ValueKind::Numeric },
            ColumnMeta { name: "email".into(), type_name: "TEXT".into(), kind: ValueKind::Text },
            ColumnMeta { name: "meta".into(), type_name: "JSON".into(), kind: ValueKind::Json },
        ],
        rows: (0..rows)
            .map(|index| {
                vec![
                    Value::Int(index as i64 + 1),
                    Value::Text(format!("user{}@example.com", index + 1).into()),
                    if index == 0 { Value::Text(r#"{"plan":"pro"}"#.into()) } else { Value::Null },
                ]
            })
            .collect(),
        has_more,
        ordered_by_key: true,
        elapsed: std::time::Duration::from_millis(12),
    }
}

impl Harness {
    /// Answers the newest FetchRows command with `page`.
    pub fn answer_rows(&mut self, page: RowPage) {
        let (session, request) = self
            .app
            .backend
            .sent
            .iter()
            .rev()
            .find_map(|command| match command {
                Command::FetchRows { session, request, .. } => Some((*session, *request)),
                _ => None,
            })
            .expect("a FetchRows was sent");
        self.app.apply(Action::Backend(Event::Rows { session, request, result: Ok(page) }));
    }
}
```

Add to the tests in `src/app.rs`:

```rust
    use crate::model::{CellPos, ObjectTabId, ObjectView};
    use crate::testing::{last_sent, page};
    use tabletist_db::{ObjectRef, SortDir};

    #[test]
    fn reconnecting_reloads_the_tree_and_the_active_tab() {
        let mut harness = Harness::new();
        let tab = harness.connect_fake();
        harness.app.apply(Action::OpenObject {
            tab,
            object: tabletist_db::ObjectRef::new("main", "users"),
            kind: ObjectKind::Table,
            pin: true,
        });
        harness.app.apply(Action::Reconnect(tab));
        let Command::Connect { session, request, .. } = *last_sent(&harness.app) else {
            panic!("expected Connect");
        };
        let before = harness.app.backend.sent.len();
        harness.app.apply(Action::Backend(Event::Connected { session, request, driver: Driver::Sqlite }));
        let new: Vec<_> = harness.app.backend.sent[before..].iter().collect();
        assert!(new.iter().any(|c| matches!(c, Command::ListSchemas { session: s, .. } if *s == session)));
        assert!(new.iter().any(|c| matches!(c, Command::ListObjects { session: s, .. } if *s == session)));
        assert!(new.iter().any(|c| matches!(c, Command::FetchRows { session: s, .. } if *s == session)));
    }

    #[test]
    fn reconnecting_restarts_inactive_tabs_that_were_loading() {
        let mut harness = Harness::new();
        let tab = harness.connect_fake();
        let background = open(&mut harness, tab, "orders", true);
        open(&mut harness, tab, "users", true);
        // `orders` is still waiting on the old session when it dies.
        harness.app.apply(Action::Reconnect(tab));
        let Command::Connect { session, request, .. } = *last_sent(&harness.app) else {
            panic!("expected Connect");
        };
        let before = harness.app.backend.sent.len();
        harness.app.apply(Action::Backend(Event::Connected { session, request, driver: Driver::Sqlite }));
        let refetched = harness.app.backend.sent[before..]
            .iter()
            .filter(|c| matches!(c, Command::FetchRows { .. }))
            .count();
        assert_eq!(refetched, 2, "both the active tab and the loading background tab refetch");
        assert!(object(&harness, tab, background).rows.is_loading());
    }

    fn users() -> ObjectRef {
        ObjectRef::new("main", "users")
    }

    fn open(harness: &mut Harness, tab: ConnTabId, name: &str, pin: bool) -> ObjectTabId {
        harness.app.apply(Action::OpenObject { tab, object: ObjectRef::new("main", name), kind: ObjectKind::Table, pin });
        harness.app.workspace(tab).unwrap().active_object.unwrap()
    }

    fn object(harness: &Harness, tab: ConnTabId, id: ObjectTabId) -> &crate::model::ObjectTab {
        harness.app.workspace(tab).unwrap().object_tab(id).unwrap()
    }

    fn last_query(harness: &Harness) -> tabletist_db::RowQuery {
        match last_sent(&harness.app) {
            Command::FetchRows { query, .. } => query.clone(),
            other => panic!("expected FetchRows, got {other:?}"),
        }
    }

    #[test]
    fn opening_an_object_fetches_its_first_page_with_the_estimate() {
        let mut harness = Harness::new();
        let tab = connect_tab(&mut harness);
        let id = open(&mut harness, tab, "users", false);
        let query = last_query(&harness);
        assert_eq!(query.object, users());
        assert_eq!(query.offset, 0);
        assert_eq!(query.limit, harness.app.settings.page_size);
        assert_eq!(object(&harness, tab, id).estimated_rows, Some(1_200_000));
        harness.answer_rows(page(5, false));
        assert_eq!(object(&harness, tab, id).page().unwrap().rows.len(), 5);
    }

    fn connect_tab(harness: &mut Harness) -> ConnTabId {
        harness.connect_fake()
    }

    #[test]
    fn a_single_click_replaces_the_preview_and_a_double_click_pins() {
        let mut harness = Harness::new();
        let tab = connect_tab(&mut harness);
        open(&mut harness, tab, "users", false);
        open(&mut harness, tab, "orders", false);
        let workspace = harness.app.workspace(tab).unwrap();
        assert_eq!(workspace.objects.len(), 1, "the preview was replaced");
        assert_eq!(workspace.objects[0].object.name, "orders");
        let orders = open(&mut harness, tab, "orders", true);
        assert!(object(&harness, tab, orders).pinned);
        open(&mut harness, tab, "users", false);
        assert_eq!(harness.app.workspace(tab).unwrap().objects.len(), 2, "a pinned tab is never replaced");
    }

    #[test]
    fn opening_an_open_object_activates_it_without_refetching() {
        let mut harness = Harness::new();
        let tab = connect_tab(&mut harness);
        let first = open(&mut harness, tab, "users", true);
        open(&mut harness, tab, "orders", true);
        let sent = harness.app.backend.sent.len();
        assert_eq!(open(&mut harness, tab, "users", false), first);
        assert_eq!(harness.app.backend.sent.len(), sent);
    }

    #[test]
    fn closing_the_active_object_tab_activates_its_neighbour() {
        let mut harness = Harness::new();
        let tab = connect_tab(&mut harness);
        let a = open(&mut harness, tab, "users", true);
        let b = open(&mut harness, tab, "orders", true);
        harness.app.apply(Action::CloseObjectTab { tab, object_tab: b });
        assert_eq!(harness.app.workspace(tab).unwrap().active_object, Some(a));
        harness.app.apply(Action::CloseObjectTab { tab, object_tab: a });
        assert_eq!(harness.app.workspace(tab).unwrap().active_object, None);
    }

    #[test]
    fn object_tabs_cycle_with_wrapping() {
        let mut harness = Harness::new();
        let tab = connect_tab(&mut harness);
        let a = open(&mut harness, tab, "users", true);
        let b = open(&mut harness, tab, "orders", true);
        harness.app.apply(Action::CycleObjectTab { tab, step: 1 });
        assert_eq!(harness.app.workspace(tab).unwrap().active_object, Some(a));
        harness.app.apply(Action::CycleObjectTab { tab, step: -1 });
        assert_eq!(harness.app.workspace(tab).unwrap().active_object, Some(b));
    }

    #[test]
    fn next_and_previous_pages_move_the_offset_and_pin() {
        let mut harness = Harness::new();
        let tab = connect_tab(&mut harness);
        let id = open(&mut harness, tab, "users", false);
        harness.answer_rows(page(300, true));
        harness.app.apply(Action::NextPage { tab, object_tab: id });
        assert_eq!(last_query(&harness).offset, 300);
        assert!(object(&harness, tab, id).pinned);
        harness.answer_rows(page(10, false));
        let sent = harness.app.backend.sent.len();
        harness.app.apply(Action::NextPage { tab, object_tab: id });
        assert_eq!(harness.app.backend.sent.len(), sent, "no next page after the last one");
        harness.app.apply(Action::PrevPage { tab, object_tab: id });
        assert_eq!(last_query(&harness).offset, 0);
        let sent = harness.app.backend.sent.len();
        harness.answer_rows(page(300, true));
        harness.app.apply(Action::PrevPage { tab, object_tab: id });
        assert_eq!(harness.app.backend.sent.len(), sent, "no previous page before the first");
    }

    #[test]
    fn sorting_cycles_ascending_descending_off_and_restarts_paging() {
        let mut harness = Harness::new();
        let tab = connect_tab(&mut harness);
        let id = open(&mut harness, tab, "users", true);
        harness.answer_rows(page(300, true));
        harness.app.apply(Action::NextPage { tab, object_tab: id });
        let sort = |harness: &mut Harness| {
            harness.app.apply(Action::SortBy { tab, object_tab: id, column: "email".into() });
            last_query(harness)
        };
        let query = sort(&mut harness);
        assert_eq!(query.offset, 0);
        assert_eq!(query.sort[0].dir, SortDir::Asc);
        assert_eq!(sort(&mut harness).sort[0].dir, SortDir::Desc);
        assert!(sort(&mut harness).sort.is_empty());
    }

    #[test]
    fn a_superseded_page_result_is_dropped() {
        let mut harness = Harness::new();
        let tab = connect_tab(&mut harness);
        let id = open(&mut harness, tab, "users", true);
        let (session, first) = match last_sent(&harness.app) {
            Command::FetchRows { session, request, .. } => (*session, *request),
            other => panic!("{other:?}"),
        };
        harness.app.apply(Action::SortBy { tab, object_tab: id, column: "email".into() });
        harness.app.apply(Action::Backend(Event::Rows { session, request: first, result: Ok(page(300, true)) }));
        assert!(object(&harness, tab, id).page().is_none());
        // And a page already shown is cleared the moment the query changes.
        harness.answer_rows(page(300, true));
        harness.app.apply(Action::NextPage { tab, object_tab: id });
        assert!(object(&harness, tab, id).page().is_none());
        assert!(object(&harness, tab, id).rows.is_loading());
    }

    #[test]
    fn a_result_for_a_closed_object_tab_is_ignored() {
        let mut harness = Harness::new();
        let tab = connect_tab(&mut harness);
        let id = open(&mut harness, tab, "users", true);
        harness.app.apply(Action::CloseObjectTab { tab, object_tab: id });
        harness.answer_rows(page(5, false));
        assert!(harness.app.workspace(tab).unwrap().objects.is_empty());
    }

    #[test]
    fn selecting_a_cell_pins_and_selection_is_clamped_to_a_shorter_page() {
        let mut harness = Harness::new();
        let tab = connect_tab(&mut harness);
        let id = open(&mut harness, tab, "users", false);
        harness.answer_rows(page(300, true));
        harness.app.apply(Action::SelectCell { tab, object_tab: id, cell: CellPos { row: 200, col: 2 } });
        assert!(object(&harness, tab, id).pinned);
        harness.app.apply(Action::Refresh(tab));
        harness.answer_rows(page(5, false));
        assert_eq!(object(&harness, tab, id).selection, Some(CellPos { row: 4, col: 2 }));
        harness.app.apply(Action::Refresh(tab));
        harness.answer_rows(page(0, false));
        assert_eq!(object(&harness, tab, id).selection, None);
    }

    #[test]
    fn selection_is_cleared_when_paging() {
        let mut harness = Harness::new();
        let tab = connect_tab(&mut harness);
        let id = open(&mut harness, tab, "users", true);
        harness.answer_rows(page(300, true));
        harness.app.apply(Action::SelectCell { tab, object_tab: id, cell: CellPos { row: 3, col: 0 } });
        harness.app.apply(Action::NextPage { tab, object_tab: id });
        assert_eq!(object(&harness, tab, id).selection, None);
    }

    #[test]
    fn moving_the_selection_stays_in_range() {
        let mut harness = Harness::new();
        let tab = connect_tab(&mut harness);
        let id = open(&mut harness, tab, "users", true);
        let mv = |harness: &mut Harness, rows: isize, cols: isize| {
            harness.app.apply(Action::MoveSelection { tab, object_tab: id, rows, cols });
            object(harness, tab, id).selection
        };
        assert_eq!(mv(&mut harness, 1, 0), None, "no page, nothing to select");
        harness.answer_rows(page(0, false));
        assert_eq!(mv(&mut harness, isize::MAX, 0), None, "an empty page stays empty");
        harness.app.apply(Action::Refresh(tab));
        harness.answer_rows(page(5, false));
        assert_eq!(mv(&mut harness, 1, 0), Some(CellPos { row: 0, col: 0 }), "the first move selects the first cell");
        assert_eq!(mv(&mut harness, -1, -1), Some(CellPos { row: 0, col: 0 }));
        assert_eq!(mv(&mut harness, isize::MAX, isize::MAX), Some(CellPos { row: 4, col: 2 }));
        assert_eq!(mv(&mut harness, isize::MIN, 0), Some(CellPos { row: 0, col: 2 }));
    }

    #[test]
    fn switching_to_structure_describes_once() {
        let mut harness = Harness::new();
        let tab = connect_tab(&mut harness);
        let id = open(&mut harness, tab, "users", true);
        harness.app.apply(Action::SetView { tab, object_tab: id, view: ObjectView::Structure });
        assert!(matches!(last_sent(&harness.app), Command::Describe { object, .. } if *object == users()));
        let sent = harness.app.backend.sent.len();
        harness.app.apply(Action::SetView { tab, object_tab: id, view: ObjectView::Data });
        harness.app.apply(Action::SetView { tab, object_tab: id, view: ObjectView::Structure });
        assert_eq!(harness.app.backend.sent.len(), sent, "a pending describe is not repeated");
    }

    #[test]
    fn cancel_sends_cancel_for_the_session_and_refresh_refetches() {
        let mut harness = Harness::new();
        let tab = connect_tab(&mut harness);
        open(&mut harness, tab, "users", true);
        harness.app.apply(Action::CancelQuery(tab));
        let session = harness.app.workspace(tab).unwrap().session;
        assert!(matches!(last_sent(&harness.app), Command::Cancel { session: s } if *s == session));
        harness.app.apply(Action::Refresh(tab));
        assert!(matches!(last_sent(&harness.app), Command::FetchRows { .. }));
    }

    #[test]
    fn the_row_panel_toggles() {
        let mut harness = Harness::new();
        let tab = connect_tab(&mut harness);
        assert!(harness.app.workspace(tab).unwrap().row_panel);
        harness.app.apply(Action::ToggleRowPanel(tab));
        assert!(!harness.app.workspace(tab).unwrap().row_panel);
    }

    #[test]
    fn copy_text_gives_the_selected_cell_or_row() {
        let mut harness = Harness::new();
        let tab = connect_tab(&mut harness);
        let id = open(&mut harness, tab, "users", true);
        assert_eq!(harness.app.copy_text(false), None);
        harness.answer_rows(page(2, false));
        harness.app.apply(Action::SelectCell { tab, object_tab: id, cell: CellPos { row: 0, col: 1 } });
        assert_eq!(harness.app.copy_text(false).as_deref(), Some("user1@example.com"));
        assert_eq!(
            harness.app.copy_text(true).as_deref(),
            Some("1\tuser1@example.com\t{\"plan\":\"pro\"}")
        );
    }
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test --lib app::tests`
Expected: FAIL to compile: unknown `Action::OpenObject` and friends.

- [ ] **Step 3: Implement**

Add the actions to `Action` in `src/model.rs`:

```rust
    OpenObject { tab: ConnTabId, object: ObjectRef, kind: ObjectKind, pin: bool },
    ActivateObjectTab { tab: ConnTabId, object_tab: ObjectTabId },
    CloseObjectTab { tab: ConnTabId, object_tab: ObjectTabId },
    PinObjectTab { tab: ConnTabId, object_tab: ObjectTabId },
    CycleObjectTab { tab: ConnTabId, step: isize },
    SetView { tab: ConnTabId, object_tab: ObjectTabId, view: ObjectView },
    NextPage { tab: ConnTabId, object_tab: ObjectTabId },
    PrevPage { tab: ConnTabId, object_tab: ObjectTabId },
    SortBy { tab: ConnTabId, object_tab: ObjectTabId, column: String },
    SelectCell { tab: ConnTabId, object_tab: ObjectTabId, cell: CellPos },
    /// Arrow keys (±1), Page Up/Down (±page), Home/End (isize::MIN/MAX).
    MoveSelection { tab: ConnTabId, object_tab: ObjectTabId, rows: isize, cols: isize },
    ToggleRowPanel(ConnTabId),
    /// Refetch the active object tab (and its structure if loaded).
    Refresh(ConnTabId),
    CancelQuery(ConnTabId),
    RetryRows { tab: ConnTabId, object_tab: ObjectTabId },
    RetryStructure { tab: ConnTabId, object_tab: ObjectTabId },
```

Replace the Task 2 stubs and add the arms in `src/app.rs`:

```rust
            Action::OpenObject { tab, object, kind, pin } => self.open_object(tab, object, kind, pin),
            Action::ActivateObjectTab { tab, object_tab } => {
                if let Some(workspace) = self.workspace_mut(tab)
                    && workspace.object_tab(object_tab).is_some()
                {
                    workspace.active_object = Some(object_tab);
                }
            }
            Action::CloseObjectTab { tab, object_tab } => {
                if let Some(workspace) = self.workspace_mut(tab)
                    && let Some(index) = workspace.objects.iter().position(|o| o.id == object_tab)
                {
                    workspace.objects.remove(index);
                    if workspace.active_object == Some(object_tab) {
                        workspace.active_object = workspace
                            .objects
                            .get(index)
                            .or_else(|| workspace.objects.last())
                            .map(|o| o.id);
                    }
                }
            }
            Action::PinObjectTab { tab, object_tab } => {
                if let Some(object) = self.object_tab_mut(tab, object_tab) {
                    object.pinned = true;
                }
            }
            Action::CycleObjectTab { tab, step } => {
                if let Some(workspace) = self.workspace_mut(tab)
                    && !workspace.objects.is_empty()
                {
                    let len = workspace.objects.len() as isize;
                    let current = workspace
                        .active_object
                        .and_then(|id| workspace.objects.iter().position(|o| o.id == id))
                        .unwrap_or(0) as isize;
                    let next = (current + step).rem_euclid(len) as usize;
                    workspace.active_object = Some(workspace.objects[next].id);
                }
            }
            Action::SetView { tab, object_tab, view } => {
                let describe = self.object_tab_mut(tab, object_tab).is_some_and(|object| {
                    object.view = view;
                    view == ObjectView::Structure && object.structure.needs_load()
                });
                if describe {
                    self.describe(tab, object_tab);
                }
            }
            Action::NextPage { tab, object_tab } => {
                let moved = self.object_tab_mut(tab, object_tab).is_some_and(|object| {
                    if !object.page().is_some_and(|page| page.has_more) {
                        return false;
                    }
                    object.query.offset += u64::from(object.query.limit);
                    object.pinned = true;
                    object.selection = None;
                    // Never show rows for a query we no longer display.
                    object.rows.value = None;
                    true
                });
                if moved {
                    self.fetch_rows(tab, object_tab);
                }
            }
            Action::PrevPage { tab, object_tab } => {
                let moved = self.object_tab_mut(tab, object_tab).is_some_and(|object| {
                    if object.query.offset == 0 {
                        return false;
                    }
                    object.query.offset = object.query.offset.saturating_sub(u64::from(object.query.limit));
                    object.pinned = true;
                    object.selection = None;
                    object.rows.value = None;
                    true
                });
                if moved {
                    self.fetch_rows(tab, object_tab);
                }
            }
            Action::SortBy { tab, object_tab, column } => {
                if let Some(object) = self.object_tab_mut(tab, object_tab) {
                    let next = match object.sort_of(&column) {
                        None => Some(SortDir::Asc),
                        Some(SortDir::Asc) => Some(SortDir::Desc),
                        Some(SortDir::Desc) => None,
                    };
                    object.query.sort = next.map(|dir| vec![Sort { column, dir }]).unwrap_or_default();
                    object.query.offset = 0;
                    object.pinned = true;
                    object.selection = None;
                    object.rows.value = None;
                    self.fetch_rows(tab, object_tab);
                }
            }
            Action::SelectCell { tab, object_tab, cell } => {
                if let Some(object) = self.object_tab_mut(tab, object_tab) {
                    object.selection = Some(cell);
                    object.pinned = true;
                }
            }
            Action::MoveSelection { tab, object_tab, rows, cols } => {
                if let Some(object) = self.object_tab_mut(tab, object_tab) {
                    let (height, width) = object
                        .page()
                        .map(|page| (page.rows.len(), page.columns.len()))
                        .unwrap_or((0, 0));
                    if height == 0 || width == 0 {
                        object.selection = None;
                    } else {
                        object.selection = Some(match object.selection {
                            None => CellPos { row: 0, col: 0 },
                            Some(cell) => CellPos {
                                row: step(cell.row, rows, height),
                                col: step(cell.col, cols, width),
                            },
                        });
                        object.pinned = true;
                    }
                }
            }
            Action::ToggleRowPanel(tab) => {
                if let Some(workspace) = self.workspace_mut(tab) {
                    workspace.row_panel = !workspace.row_panel;
                }
            }
            Action::Refresh(tab) => {
                let active = self.workspace(tab).and_then(|w| w.active_object_tab()).map(|o| {
                    (o.id, o.structure.value.is_some() || o.structure.error.is_some())
                });
                match active {
                    Some((id, described)) => {
                        self.fetch_rows(tab, id);
                        if described {
                            self.describe(tab, id);
                        }
                    }
                    None => self.refresh_tree(tab),
                }
            }
            Action::CancelQuery(tab) => {
                if let Some(workspace) = self.workspace(tab) {
                    let session = workspace.session;
                    self.backend.send(Command::Cancel { session });
                }
            }
            Action::RetryRows { tab, object_tab } => self.fetch_rows(tab, object_tab),
            Action::RetryStructure { tab, object_tab } => self.describe(tab, object_tab),
```

Add to `apply_event`:

```rust
            Event::Rows { session, request, result } => {
                let Some(tab) = self.tab_for_session(session) else { return };
                let Some(workspace) = self.workspace_mut(tab) else { return };
                let Some(object) = workspace.objects.iter_mut().find(|o| o.rows.pending == Some(request)) else {
                    return;
                };
                object.rows.finish(request, result);
                let (height, width) = object
                    .page()
                    .map(|page| (page.rows.len(), page.columns.len()))
                    .unwrap_or((0, 0));
                object.selection = match object.selection {
                    Some(_) if height == 0 || width == 0 => None,
                    Some(cell) => Some(CellPos { row: cell.row.min(height - 1), col: cell.col.min(width - 1) }),
                    None => None,
                };
            }
            Event::Structure { session, request, result } => {
                if let Some(tab) = self.tab_for_session(session)
                    && let Some(workspace) = self.workspace_mut(tab)
                    && let Some(object) = workspace.objects.iter_mut().find(|o| o.structure.pending == Some(request))
                {
                    object.structure.finish(request, result);
                }
            }
```

Add helpers and a free function:

```rust
    fn object_tab_mut(&mut self, tab: ConnTabId, id: ObjectTabId) -> Option<&mut ObjectTab> {
        self.workspace_mut(tab)?.object_tab_mut(id)
    }

    /// The connection tab and object tab the keyboard acts on.
    pub fn active_object(&self) -> Option<(ConnTabId, ObjectTabId)> {
        let tab = self.active_tab_id();
        let workspace = self.workspace(tab)?;
        Some((tab, workspace.active_object?))
    }

    pub fn open_object(&mut self, tab: ConnTabId, object: ObjectRef, kind: ObjectKind, pin: bool) {
        let page_size = self.settings.page_size;
        let new_id = ObjectTabId(self.next_id());
        let Some(workspace) = self.workspace_mut(tab) else { return };
        if let Some(existing) = workspace.objects.iter_mut().find(|o| o.object == object) {
            existing.pinned |= pin;
            workspace.active_object = Some(existing.id);
            return;
        }
        let estimate = workspace.tree.object_info(&object).and_then(|info| info.estimated_rows);
        let opened = ObjectTab::new(new_id, object, kind, pin, page_size, estimate);
        let preview = if pin { None } else { workspace.objects.iter().position(|o| !o.pinned) };
        match preview {
            Some(index) => workspace.objects[index] = opened,
            None => workspace.objects.push(opened),
        }
        workspace.active_object = Some(new_id);
        self.fetch_rows(tab, new_id);
    }

    pub fn fetch_rows(&mut self, tab: ConnTabId, id: ObjectTabId) {
        let request = RequestId(self.next_id());
        let Some(workspace) = self.workspace_mut(tab) else { return };
        let session = workspace.session;
        let Some(object) = workspace.object_tab_mut(id) else { return };
        object.rows.start(request);
        let query = object.query.clone();
        self.backend.send(Command::FetchRows { session, request, query });
    }

    pub fn describe(&mut self, tab: ConnTabId, id: ObjectTabId) {
        let request = RequestId(self.next_id());
        let Some(workspace) = self.workspace_mut(tab) else { return };
        let session = workspace.session;
        let Some(object) = workspace.object_tab_mut(id) else { return };
        object.structure.start(request);
        let target = object.object.clone();
        self.backend.send(Command::Describe { session, request, object: target });
    }

    /// Text for the clipboard: the selected cell, or its whole row as TSV.
    pub fn copy_text(&self, whole_row: bool) -> Option<String> {
        let (tab, id) = self.active_object()?;
        let object = self.workspace(tab)?.object_tab(id)?;
        let cell = object.selection?;
        let row = object.page()?.rows.get(cell.row)?;
        Some(if whole_row {
            crate::ui::format::tsv_row(row)
        } else {
            crate::ui::format::plain_text(row.get(cell.col)?)
        })
    }
```

Replace Task 2's `after_connect` so a reconnect restarts everything that was waiting on the dead session: the active tab's rows and loaded structure, and any background tab whose rows or structure were in flight:

```rust
    pub fn after_connect(&mut self, tab: ConnTabId) {
        self.refresh_tree(tab);
        let Some(workspace) = self.workspace_mut(tab) else { return };
        let active = workspace.active_object;
        let pending = workspace.pending_open.take();
        let stale: Vec<(ObjectTabId, bool, bool)> = workspace
            .objects
            .iter()
            .map(|object| {
                let is_active = Some(object.id) == active;
                (
                    object.id,
                    is_active || object.rows.is_loading(),
                    object.structure.is_loading() || (is_active && object.structure.value.is_some()),
                )
            })
            .collect();
        for (id, rows, structure) in stale {
            if rows {
                self.fetch_rows(tab, id);
            }
            if structure {
                self.describe(tab, id);
            }
        }
        if let Some((object, kind)) = pending {
            self.open_object(tab, object, kind, true);
        }
    }
```

```rust
/// Moves `index` by `delta` within `0..len` (len > 0), saturating.
fn step(index: usize, delta: isize, len: usize) -> usize {
    let moved = (index as i128 + delta as i128).clamp(0, len as i128 - 1);
    moved as usize
}
```

Extend the existing imports in `app.rs` with `CellPos, ObjectTab, ObjectView` (from `crate::model`) and `Sort, SortDir` (from `tabletist_db`); do not add second `use` lines for names Task 2 already imported.

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test --lib`
Expected: PASS, including `reconnecting_reloads_the_tree_and_the_active_tab`.

- [ ] **Step 5: Commit**

```bash
git add src/model.rs src/app.rs src/testing.rs
git commit -m "Open object tabs and page, sort, select and describe in the reducer"
```

---

### Task 4: The data grid widget

**Files:**
- Create: `src/ui/grid.rs`
- Modify: `src/ui/mod.rs` (`pub mod grid;`), `src/theme.rs` (add `ChevronUp => lucide "chevron-up"` to `Icon`)

**Interfaces:**
- Consumes: `CellPos`, `virtual_rows`, `Palette`, `Icon`.
- Produces: `grid::ROW_HEIGHT: f32 = 24.0`, `grid::HEADER_HEIGHT: f32 = 38.0`; `grid::Column<'a> { name: &'a str, type_name: &'a str, numeric: bool, sort: Option<SortDir> }`; `grid::Cell<'a> { text: Cow<'a, str>, null: bool }`; `grid::GridOutput { clicked: Option<CellPos>, sort_clicked: Option<usize> }` (`Default, Debug, PartialEq`); `grid::show<'a>(ui: &mut Ui, id: Id, columns: &[Column<'_>], row_count: usize, first_row_number: u64, selection: Option<CellPos>, palette: &Palette, cell: impl FnMut(usize, usize) -> Cell<'a>) -> GridOutput`; `grid::column_at(widths: &[f32], x: f32) -> usize`; `grid::initial_widths(...) -> Vec<f32>`.

- [ ] **Step 1: Write the failing tests**

Create `src/ui/grid.rs` with only tests:

```rust
//! The data grid: a sticky header with resizable columns over virtualized
//! rows, one selected cell. Reads data through a closure and reports clicks;
//! it never changes application state.

#[cfg(test)]
mod tests {
    use super::*;
    use egui::accesskit::Role;

    fn columns() -> Vec<Column<'static>> {
        vec![
            Column { name: "id", type_name: "INTEGER", numeric: true, sort: None },
            Column { name: "email", type_name: "TEXT", numeric: false, sort: Some(SortDir::Asc) },
        ]
    }

    /// Runs one frame of a grid with `rows` rows and returns its output and
    /// the AccessKit tree.
    fn frame(
        ctx: &egui::Context,
        rows: usize,
        events: Vec<egui::Event>,
    ) -> (GridOutput, egui::accesskit::TreeUpdate) {
        let mut result = GridOutput::default();
        let palette = crate::theme::Palette::dark();
        let mut output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(800.0, 400.0))),
                events,
                ..Default::default()
            },
            |ui| {
                let columns = columns();
                result = show(ui, egui::Id::new("grid"), &columns, rows, 0, None, &palette, |row, col| Cell {
                    text: format!("r{row}c{col}").into(),
                    null: false,
                });
            },
        );
        // egui panics if a frame's texture updates are dropped unhandled.
        output.textures_delta.clear();
        (result, output.platform_output.accesskit_update.expect("accesskit"))
    }

    fn click(target: egui::accesskit::NodeId) -> egui::Event {
        egui::Event::AccessKitActionRequest(egui::accesskit::ActionRequest {
            target_tree: egui::accesskit::TreeId::ROOT,
            target_node: target,
            action: egui::accesskit::Action::Click,
            data: None,
        })
    }

    #[test]
    fn columns_are_found_by_x() {
        let widths = [100.0, 50.0, 80.0];
        assert_eq!(column_at(&widths, -10.0), 0);
        assert_eq!(column_at(&widths, 99.0), 0);
        assert_eq!(column_at(&widths, 120.0), 1);
        assert_eq!(column_at(&widths, 1000.0), 2);
    }

    #[test]
    fn initial_widths_fit_content_within_limits() {
        let columns = columns();
        let widths = initial_widths(&columns, 3, &mut |_, col| Cell {
            text: if col == 0 { "1".into() } else { "x".repeat(500).into() },
            null: false,
        });
        assert!(widths[0] >= 48.0 && widths[0] < 120.0, "{widths:?}");
        assert_eq!(widths[1], 360.0);
    }

    #[test]
    fn only_visible_rows_are_built() {
        let ctx = egui::Context::default();
        // The header draws with Inter's named weights, which need the fonts.
        crate::theme::install(&ctx, false);
        ctx.enable_accesskit();
        frame(&ctx, 100_000, vec![]);
        let (_, tree) = frame(&ctx, 100_000, vec![]);
        let rows = tree
            .nodes
            .iter()
            .filter(|(_, node)| node.label().is_some_and(|label| label.starts_with("Row ")))
            .count();
        assert!(rows > 5 && rows < 40, "{rows} rows built");
    }

    #[test]
    fn clicking_a_header_reports_the_column_and_clicking_a_row_selects_it() {
        let ctx = egui::Context::default();
        crate::theme::install(&ctx, false);
        ctx.enable_accesskit();
        frame(&ctx, 10, vec![]);
        let (_, tree) = frame(&ctx, 10, vec![]);
        let email = crate::testing::node(&tree, "email", Role::Button).expect("email header");
        let (output, _) = frame(&ctx, 10, vec![click(email)]);
        assert_eq!(output.sort_clicked, Some(1));
        let (_, tree) = frame(&ctx, 10, vec![]);
        let row = crate::testing::node(&tree, "Row 3", Role::Button).expect("row 3");
        let (output, _) = frame(&ctx, 10, vec![click(row)]);
        assert_eq!(output.clicked, Some(CellPos { row: 2, col: 0 }));
    }
}
```

Add `pub mod grid;` to `src/ui/mod.rs` and `ChevronUp => lucide "chevron-up",` to the `Icon` enum in `src/theme.rs`.

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test --lib grid`
Expected: FAIL to compile: `cannot find function show`.

- [ ] **Step 3: Implement the grid**

Put above the tests:

```rust
use std::borrow::Cow;

use egui::{Align2, CornerRadius, Id, Rect, Sense, Stroke, StrokeKind, Ui, WidgetInfo, WidgetType, pos2, vec2};
use tabletist_db::SortDir;

use crate::model::CellPos;
use crate::theme::{self, Icon, Palette};
use crate::ui::widgets::virtual_rows;

pub const ROW_HEIGHT: f32 = 24.0;
pub const HEADER_HEIGHT: f32 = 38.0;
const NUMBER_WIDTH: f32 = 52.0;
const MIN_WIDTH: f32 = 48.0;
const MAX_INITIAL_WIDTH: f32 = 360.0;
const HANDLE_WIDTH: f32 = 6.0;
const SAMPLE_ROWS: usize = 50;

pub struct Column<'a> {
    pub name: &'a str,
    pub type_name: &'a str,
    pub numeric: bool,
    pub sort: Option<SortDir>,
}

pub struct Cell<'a> {
    pub text: Cow<'a, str>,
    pub null: bool,
}

#[derive(Debug, Default, PartialEq)]
pub struct GridOutput {
    pub clicked: Option<CellPos>,
    pub sort_clicked: Option<usize>,
}

/// The column under `x`, measured from the first data column's left edge.
pub fn column_at(widths: &[f32], x: f32) -> usize {
    let mut edge = 0.0;
    for (index, width) in widths.iter().enumerate() {
        edge += width;
        if x < edge {
            return index;
        }
    }
    widths.len().saturating_sub(1)
}

/// Widths that fit the header and the first rows, within limits.
pub fn initial_widths<'a>(
    columns: &[Column<'_>],
    rows: usize,
    cell: &mut impl FnMut(usize, usize) -> Cell<'a>,
) -> Vec<f32> {
    columns
        .iter()
        .enumerate()
        .map(|(col, column)| {
            let header = column.name.chars().count().max(column.type_name.chars().count()) as f32 * 7.5 + 28.0;
            let widest = (0..rows.min(SAMPLE_ROWS))
                .map(|row| cell(row, col).text.chars().count())
                .max()
                .unwrap_or(0) as f32
                * 7.0
                + 16.0;
            header.max(widest).clamp(MIN_WIDTH, MAX_INITIAL_WIDTH)
        })
        .collect()
}

#[allow(clippy::too_many_arguments)] // one call site per view; a struct adds nothing
pub fn show<'a>(
    ui: &mut Ui,
    id: Id,
    columns: &[Column<'_>],
    row_count: usize,
    first_row_number: u64,
    selection: Option<CellPos>,
    palette: &Palette,
    mut cell: impl FnMut(usize, usize) -> Cell<'a>,
) -> GridOutput {
    let mut output = GridOutput::default();
    let widths_id = id.with(("widths", columns.len()));
    let last_id = id.with("last-selection");
    let mut widths: Vec<f32> = ui
        .data(|data| data.get_temp::<Vec<f32>>(widths_id))
        .filter(|widths| widths.len() == columns.len())
        .unwrap_or_else(|| initial_widths(columns, row_count, &mut cell));
    let last: Option<CellPos> = ui.data(|data| data.get_temp::<Option<CellPos>>(last_id)).flatten();
    let reveal = selection.filter(|cell| Some(*cell) != last);
    let total = NUMBER_WIDTH + widths.iter().sum::<f32>();

    egui::ScrollArea::both()
        .id_salt(id)
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
            let origin = ui.cursor().min;
            ui.allocate_space(vec2(total, HEADER_HEIGHT));

            if let Some(target) = reveal {
                let col = target.col.min(widths.len().saturating_sub(1));
                let x = origin.x + NUMBER_WIDTH + widths[..col].iter().sum::<f32>();
                let y = origin.y + HEADER_HEIGHT + target.row as f32 * ROW_HEIGHT;
                // Include the header's height above the row so the sticky
                // header never covers it.
                let rect = Rect::from_min_size(
                    pos2(x - NUMBER_WIDTH, y - HEADER_HEIGHT),
                    vec2(widths.get(col).copied().unwrap_or(0.0) + NUMBER_WIDTH, ROW_HEIGHT + HEADER_HEIGHT),
                );
                ui.scroll_to_rect(rect, None);
            }

            virtual_rows(ui, row_count, ROW_HEIGHT, |ui, row| {
                let (rect, response) = ui.allocate_exact_size(vec2(total, ROW_HEIGHT), Sense::click());
                let number = first_row_number + row as u64 + 1;
                let label = format!("Row {number}");
                response.widget_info(|| WidgetInfo::labeled(WidgetType::Button, true, &label));
                if response.clicked() {
                    let col = response
                        .interact_pointer_pos()
                        .map(|pointer| column_at(&widths, pointer.x - rect.left() - NUMBER_WIDTH))
                        .unwrap_or(0);
                    output.clicked = Some(CellPos { row, col });
                }
                let painter = ui.painter().clone();
                let selected_row = selection.is_some_and(|cell| cell.row == row);
                if selected_row {
                    painter.rect_filled(rect, CornerRadius::ZERO, palette.accent.gamma_multiply(0.14));
                } else if row % 2 == 1 {
                    painter.rect_filled(rect, CornerRadius::ZERO, palette.surface.gamma_multiply(0.45));
                }
                painter.text(
                    pos2(rect.left() + NUMBER_WIDTH - 8.0, rect.center().y),
                    Align2::RIGHT_CENTER,
                    number.to_string(),
                    theme::regular(11.5),
                    palette.dim,
                );
                let divider = Stroke::new(1.0, palette.outline.gamma_multiply(0.6));
                let mut x = rect.left() + NUMBER_WIDTH;
                for (col, width) in widths.iter().enumerate() {
                    let cell_rect = Rect::from_min_size(pos2(x, rect.top()), vec2(*width, ROW_HEIGHT));
                    x += width;
                    if !ui.is_rect_visible(cell_rect) {
                        continue;
                    }
                    let content = cell(row, col);
                    let color = if content.null { palette.dim } else { palette.text };
                    let clip = painter.with_clip_rect(cell_rect.shrink2(vec2(6.0, 0.0)).intersect(ui.clip_rect()));
                    let (anchor, at) = if columns[col].numeric && !content.null {
                        (Align2::RIGHT_CENTER, pos2(cell_rect.right() - 6.0, cell_rect.center().y))
                    } else {
                        (Align2::LEFT_CENTER, pos2(cell_rect.left() + 6.0, cell_rect.center().y))
                    };
                    clip.text(at, anchor, content.text, theme::regular(13.0), color);
                    painter.vline(cell_rect.right(), rect.y_range(), divider);
                    if selection == Some(CellPos { row, col }) {
                        painter.rect_stroke(
                            cell_rect.shrink(1.0),
                            CornerRadius::same(2),
                            Stroke::new(1.5, palette.accent),
                            StrokeKind::Inside,
                        );
                    }
                }
            });

            // The header, painted over the rows at the top of the visible
            // area so it stays put while rows scroll under it. Its widgets
            // come after the rows', so they sit on top.
            let top = ui.clip_rect().top().max(origin.y);
            let header = Rect::from_min_size(pos2(origin.x, top), vec2(total, HEADER_HEIGHT));
            let painter = ui.painter().clone();
            painter.rect_filled(header, CornerRadius::ZERO, palette.panel);
            painter.hline(header.x_range(), header.bottom(), Stroke::new(1.0, palette.outline));
            let mut x = origin.x + NUMBER_WIDTH;
            for (col, column) in columns.iter().enumerate() {
                let rect = Rect::from_min_size(pos2(x, top), vec2(widths[col], HEADER_HEIGHT));
                let response = ui.interact(rect, id.with(("header", col)), Sense::click());
                response.widget_info(|| WidgetInfo::labeled(WidgetType::Button, true, column.name));
                if response.clicked() {
                    output.sort_clicked = Some(col);
                }
                if response.hovered() {
                    painter.rect_filled(rect, CornerRadius::ZERO, palette.surface_hover);
                }
                let text = painter.with_clip_rect(rect.shrink2(vec2(8.0, 0.0)));
                text.text(
                    pos2(rect.left() + 8.0, rect.top() + 12.0),
                    Align2::LEFT_CENTER,
                    column.name,
                    theme::semibold(12.5),
                    palette.text,
                );
                text.text(
                    pos2(rect.left() + 8.0, rect.top() + 27.0),
                    Align2::LEFT_CENTER,
                    column.type_name,
                    theme::regular(11.0),
                    palette.dim,
                );
                if let Some(dir) = column.sort {
                    let icon = if dir == SortDir::Asc { Icon::ChevronUp } else { Icon::ChevronDown };
                    icon.image(palette.accent, 12.0).paint_at(
                        ui,
                        Rect::from_center_size(pos2(rect.right() - 12.0, rect.top() + 12.0), vec2(12.0, 12.0)),
                    );
                }
                let handle = Rect::from_min_max(
                    pos2(rect.right() - HANDLE_WIDTH / 2.0, top),
                    pos2(rect.right() + HANDLE_WIDTH / 2.0, top + HEADER_HEIGHT),
                );
                let drag = ui.interact(handle, id.with(("resize", col)), Sense::drag());
                if drag.hovered() || drag.dragged() {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeColumn);
                }
                if drag.dragged() {
                    widths[col] = (widths[col] + drag.drag_delta().x).max(MIN_WIDTH);
                }
                painter.vline(rect.right(), header.y_range(), Stroke::new(1.0, palette.outline));
                x += widths[col];
            }
        });

    ui.data_mut(|data| {
        data.insert_temp(widths_id, widths);
        data.insert_temp(last_id, selection);
    });
    output
}
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test --lib grid`
Expected: PASS (4 tests). If `only_visible_rows_are_built` counts too many rows, the scroll area is not clipping: check `auto_shrink([false, false])` and that the frame's screen rect is 400 px tall.

- [ ] **Step 5: Commit**

```bash
git add src/ui/grid.rs src/ui/mod.rs src/theme.rs
git commit -m "Add the data grid with a sticky header and resizable columns"
```

---

### Task 5: Sidebar, object tabs, and the Data view

**Files:**
- Create: `src/ui/sidebar.rs`, `src/ui/object_tabs.rs`, `src/ui/data_view.rs`
- Modify: `src/ui/workspace.rs`, `src/ui/mod.rs`

**Interfaces:**
- Consumes: `Tree::visible_rows`, `TreeRow`, `TreeNode`, `ObjectTab`, `grid::show`, `format::{cell_text, range_label, elapsed}`.
- Produces: `sidebar::show(app, ui, tab)`, `object_tabs::show(app, ui, tab)`, `data_view::show(app, ui, tab, object_tab)`; `workspace::show` composes sidebar, row panel (Task 6), object tabs, and the active view.

- [ ] **Step 1: Write the failing UI tests**

Add to the tests in `src/ui/mod.rs`:

```rust
    use crate::backend::Command;

    fn fetches(harness: &Harness) -> usize {
        harness.app.backend.sent.iter().filter(|c| matches!(c, Command::FetchRows { .. })).count()
    }

    #[test]
    fn the_sidebar_lists_objects_and_a_click_opens_a_preview_tab() {
        let mut harness = Harness::new();
        harness.connect_fake();
        assert!(harness.has("main"));
        assert!(harness.has("active_users"));
        harness.click("users");
        assert_eq!(fetches(&harness), 1);
        assert!(harness.has("users tab"));
        assert!(harness.has("Close users"));
    }

    #[test]
    fn a_loaded_page_shows_headers_rows_and_the_range() {
        let mut harness = Harness::new();
        harness.connect_fake();
        harness.click("users");
        harness.answer_rows(crate::testing::page(5, false));
        assert!(harness.has("email"));
        assert!(harness.has("Row 5"));
        assert!(harness.has("1–5 of 5"));
    }

    #[test]
    fn header_clicks_sort_and_the_next_page_button_pages() {
        let mut harness = Harness::new();
        harness.connect_fake();
        harness.click("users");
        harness.answer_rows(crate::testing::page(300, true));
        harness.click("email");
        match crate::testing::last_sent(&harness.app) {
            Command::FetchRows { query, .. } => assert_eq!(query.sort[0].column, "email"),
            other => panic!("{other:?}"),
        }
        harness.answer_rows(crate::testing::page(300, true));
        harness.click("Next page");
        match crate::testing::last_sent(&harness.app) {
            Command::FetchRows { query, .. } => assert_eq!(query.offset, 300),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn a_failed_page_shows_the_error_and_retry_refetches() {
        let mut harness = Harness::new();
        harness.connect_fake();
        harness.click("users");
        let (session, request) = match crate::testing::last_sent(&harness.app) {
            Command::FetchRows { session, request, .. } => (*session, *request),
            other => panic!("{other:?}"),
        };
        harness.app.apply(crate::model::Action::Backend(crate::backend::Event::Rows {
            session,
            request,
            result: Err(tabletist_db::Error::query("no such column: nope")),
        }));
        assert!(harness.has("no such column: nope"));
        let before = fetches(&harness);
        harness.click("Retry");
        assert_eq!(fetches(&harness), before + 1);
    }

    #[test]
    fn a_running_query_shows_a_cancel_button() {
        let mut harness = Harness::new();
        harness.connect_fake();
        harness.click("users");
        harness.click("Cancel query");
        assert!(matches!(crate::testing::last_sent(&harness.app), Command::Cancel { .. }));
    }

    #[test]
    fn the_object_tab_close_button_closes_it() {
        let mut harness = Harness::new();
        let tab = harness.connect_fake();
        harness.click("users");
        harness.click("Close users");
        assert!(harness.app.workspace(tab).unwrap().objects.is_empty());
        assert!(harness.has("Select a table or view in the sidebar"));
    }
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test --lib ui::tests`
Expected: FAIL (no sidebar or views yet).

- [ ] **Step 3: Implement the sidebar**

Create `src/ui/sidebar.rs`:

```rust
//! The sidebar: filter, refresh, and the tree of schemas and objects.

use egui::{Align2, CornerRadius, Frame, Id, Margin, RichText, Sense, TextEdit, WidgetInfo, WidgetType, pos2, vec2};
use tabletist_db::ObjectKind;

use crate::app::App;
use crate::i18n::{Locale, gettext};
use crate::model::{Action, ConnTabId, TreeNode, TreeRow};
use crate::theme::{self, Icon};
use crate::ui::widgets::{icon_button, virtual_rows};

const ROW_HEIGHT: f32 = 24.0;
const INDENT: f32 = 14.0;

fn group_label(locale: Locale, kind: ObjectKind, count: usize) -> String {
    let name = match kind {
        ObjectKind::Table => gettext(locale, "Tables"),
        ObjectKind::View => gettext(locale, "Views"),
        ObjectKind::MaterializedView => gettext(locale, "Materialized views"),
    };
    format!("{name} ({count})")
}

pub fn show(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId) {
    let locale = app.locale;
    let palette = app.palette;
    let show_system = app.settings.show_system_schemas;
    let Some(workspace) = app.workspace(tab) else { return };
    let rows = workspace.tree.visible_rows(workspace.driver, show_system);
    let schemas_loading = workspace.tree.schemas.is_loading() && workspace.tree.schemas.value.is_none();
    let schemas_error = workspace.tree.schemas.error.as_ref().map(ToString::to_string);
    let active = workspace.active_object_tab().map(|object| object.object.clone());
    let mut actions = Vec::new();

    egui::Panel::left(Id::new(("sidebar", tab.0)))
        .resizable(true)
        .default_size(240.0)
        .size_range(180.0..=480.0)
        .frame(Frame::new().fill(palette.panel).inner_margin(Margin::same(8)))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                if let Some(workspace) = app.workspace_mut(tab) {
                    ui.add(
                        TextEdit::singleline(&mut workspace.tree.filter)
                            .hint_text(gettext(locale, "Filter"))
                            .desired_width(ui.available_width() - 30.0),
                    );
                }
                if icon_button(ui, Icon::RefreshCw, &gettext(locale, "Refresh objects"), &palette).clicked() {
                    actions.push(Action::RefreshTree(tab));
                }
            });
            ui.add_space(6.0);
            if schemas_loading {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label(gettext(locale, "Loading…"));
                });
            }
            if let Some(error) = schemas_error {
                ui.label(RichText::new(error).color(palette.danger));
                if ui.button(gettext(locale, "Retry")).clicked() {
                    actions.push(Action::RefreshTree(tab));
                }
            }
            egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                virtual_rows(ui, rows.len(), ROW_HEIGHT, |ui, index| {
                    tree_row(ui, &rows[index], tab, active.as_ref(), locale, &palette, &mut actions);
                });
            });
        });
    app.actions.extend(actions);
}

fn tree_row(
    ui: &mut egui::Ui,
    row: &TreeRow,
    tab: ConnTabId,
    active: Option<&tabletist_db::ObjectRef>,
    locale: Locale,
    palette: &crate::theme::Palette,
    actions: &mut Vec<Action>,
) {
    let (rect, response) = ui.allocate_exact_size(vec2(ui.available_width(), ROW_HEIGHT), Sense::click());
    let label = match &row.node {
        TreeNode::Group(_, kind) => group_label(locale, *kind, row.count.unwrap_or(0)),
        _ => row.label.clone(),
    };
    response.widget_info(|| WidgetInfo::labeled(WidgetType::Button, true, &label));
    let selected = matches!(&row.node, TreeNode::Object(object, _) if Some(object) == active);
    if selected {
        ui.painter().rect_filled(rect, CornerRadius::same(theme::RADIUS_SMALL), palette.accent.gamma_multiply(0.2));
    } else if response.hovered() {
        ui.painter().rect_filled(rect, CornerRadius::same(theme::RADIUS_SMALL), palette.surface_hover);
    }
    let left = rect.left() + 4.0 + f32::from(row.depth) * INDENT;
    if let Some(expanded) = row.expanded {
        let icon = if expanded { Icon::ChevronDown } else { Icon::ChevronRight };
        icon.image(palette.secondary, 14.0)
            .paint_at(ui, egui::Rect::from_center_size(pos2(left + 7.0, rect.center().y), vec2(14.0, 14.0)));
    }
    let (font, color) = match row.node {
        TreeNode::Schema(_) => (theme::medium(13.0), palette.text),
        TreeNode::Group(..) => (theme::regular(12.5), palette.secondary),
        TreeNode::Object(..) => (theme::regular(13.0), palette.text),
    };
    ui.painter().with_clip_rect(rect).text(
        pos2(left + 18.0, rect.center().y),
        Align2::LEFT_CENTER,
        &label,
        font,
        color,
    );
    if row.loading {
        let spinner = egui::Rect::from_center_size(pos2(rect.right() - 10.0, rect.center().y), vec2(12.0, 12.0));
        egui::Spinner::new().size(12.0).paint_at(ui, spinner);
    }
    if let Some(error) = &row.error {
        response.clone().on_hover_text(error);
        ui.painter().circle_filled(pos2(rect.right() - 10.0, rect.center().y), 4.0, palette.danger);
    }
    match &row.node {
        TreeNode::Schema(schema) if response.clicked() => {
            actions.push(Action::ToggleSchema { tab, schema: schema.clone() });
        }
        TreeNode::Group(schema, kind) if response.clicked() => {
            actions.push(Action::ToggleGroup { tab, schema: schema.clone(), kind: *kind });
        }
        TreeNode::Object(object, kind) => {
            if response.double_clicked() {
                actions.push(Action::OpenObject { tab, object: object.clone(), kind: *kind, pin: true });
            } else if response.clicked() {
                actions.push(Action::OpenObject { tab, object: object.clone(), kind: *kind, pin: false });
            }
        }
        _ => {}
    }
}
```

- [ ] **Step 4: Implement object tabs**

Create `src/ui/object_tabs.rs`:

```rust
//! The object tab bar inside a connection tab. Preview tabs are italic.

use egui::{
    Align, CornerRadius, Frame, Id, Layout, Margin, Rect, RichText, Sense, Stroke, StrokeKind, UiBuilder,
    WidgetInfo, WidgetType, pos2, vec2,
};

use crate::app::App;
use crate::i18n::gettext;
use crate::model::{Action, ConnTabId};
use crate::theme::{self, Icon};
use crate::ui::widgets::icon_button;

const HEIGHT: f32 = 32.0;
const TAB_WIDTH: f32 = 170.0;

pub fn show(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId) {
    let locale = app.locale;
    let palette = app.palette;
    let Some(workspace) = app.workspace(tab) else { return };
    let tabs: Vec<_> = workspace
        .objects
        .iter()
        .map(|object| (object.id, object.object.name.clone(), object.pinned))
        .collect();
    let active = workspace.active_object;
    if tabs.is_empty() {
        return;
    }
    let mut actions = Vec::new();
    egui::Panel::top(Id::new(("object-tabs", tab.0)))
        .exact_size(HEIGHT)
        .resizable(false)
        .frame(Frame::new().fill(palette.panel).inner_margin(Margin::symmetric(6, 3)))
        .show(ui, |ui| {
            egui::ScrollArea::horizontal()
                .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        for (id, name, pinned) in tabs {
                            let is_active = Some(id) == active;
                            let (rect, response) = ui.allocate_exact_size(vec2(TAB_WIDTH, HEIGHT - 6.0), Sense::click());
                            let label = format!("{name} {}", gettext(locale, "tab"));
                            response.widget_info(|| WidgetInfo::selected(WidgetType::Button, true, is_active, &label));
                            if response.double_clicked() {
                                actions.push(Action::PinObjectTab { tab, object_tab: id });
                            } else if response.clicked() {
                                actions.push(Action::ActivateObjectTab { tab, object_tab: id });
                            }
                            if response.middle_clicked() {
                                actions.push(Action::CloseObjectTab { tab, object_tab: id });
                            }
                            let corner = CornerRadius::same(theme::RADIUS_SMALL);
                            let fill = if is_active { palette.window } else if response.hovered() { palette.surface_hover } else { palette.panel };
                            ui.painter().rect_filled(rect, corner, fill);
                            if is_active {
                                ui.painter().rect_stroke(rect, corner, Stroke::new(1.0, palette.outline), StrokeKind::Inside);
                            }
                            let inner = rect.shrink2(vec2(8.0, 0.0));
                            let close_rect = Rect::from_center_size(pos2(inner.right() - 12.0, inner.center().y), vec2(24.0, 24.0));
                            let mut close_ui = ui.new_child(UiBuilder::new().max_rect(close_rect));
                            let close = format!("{} {name}", gettext(locale, "Close"));
                            if icon_button(&mut close_ui, Icon::X, &close, &palette).clicked() {
                                actions.push(Action::CloseObjectTab { tab, object_tab: id });
                            }
                            let mut label_ui = ui.new_child(
                                UiBuilder::new()
                                    .max_rect(Rect::from_min_max(inner.min, pos2(close_rect.left() - 4.0, inner.max.y)))
                                    .layout(Layout::left_to_right(Align::Center)),
                            );
                            let mut text = RichText::new(&name).font(theme::regular(13.0)).color(if is_active { palette.text } else { palette.secondary });
                            if !pinned {
                                text = text.italics();
                            }
                            label_ui.add(egui::Label::new(text).truncate().selectable(false));
                        }
                    });
                });
        });
    app.actions.extend(actions);
}
```

- [ ] **Step 5: Implement the Data view**

Create `src/ui/data_view.rs`:

```rust
//! The Data view: footer (view switch, range, paging, timing, cancel) and the
//! grid, or the error or empty state.

use egui::{Frame, Id, Margin, RichText};
use tabletist_db::ValueKind;

use crate::app::App;
use crate::i18n::gettext;
use crate::model::{Action, ConnTabId, ObjectTabId, ObjectView};
use crate::theme::{self, Icon};
use crate::ui::format;
use crate::ui::grid::{self, Cell, Column};
use crate::ui::widgets::icon_button;

pub fn footer(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId, object_tab: ObjectTabId) {
    let locale = app.locale;
    let palette = app.palette;
    let Some(object) = app.workspace(tab).and_then(|w| w.object_tab(object_tab)) else { return };
    let view = object.view;
    let loading = object.rows.is_loading() || object.structure.is_loading();
    let page = object.page();
    let range = page.and_then(|page| {
        format::range_label(object.query.offset, page.rows.len(), page.has_more, object.estimated_rows)
    });
    let can_next = page.is_some_and(|page| page.has_more) && !object.rows.is_loading();
    let can_prev = object.query.offset > 0 && !object.rows.is_loading();
    let timing = page.map(|page| format::elapsed(page.elapsed));
    let unordered = page.is_some_and(|page| !page.ordered_by_key) && object.query.sort.is_empty();
    let mut actions = Vec::new();
    egui::Panel::bottom(Id::new(("object-footer", tab.0, object_tab.0)))
        .exact_size(32.0)
        .resizable(false)
        .frame(Frame::new().fill(palette.panel).inner_margin(Margin::symmetric(8, 4)))
        .show(ui, |ui| {
            ui.horizontal_centered(|ui| {
                for (target, label) in [(ObjectView::Data, gettext(locale, "Data")), (ObjectView::Structure, gettext(locale, "Structure"))] {
                    if ui.selectable_label(view == target, label.as_ref()).clicked() && view != target {
                        actions.push(Action::SetView { tab, object_tab, view: target });
                    }
                }
                ui.separator();
                if view == ObjectView::Data {
                    ui.add_enabled_ui(can_prev, |ui| {
                        if icon_button(ui, Icon::ChevronLeft, &gettext(locale, "Previous page"), &palette).clicked() {
                            actions.push(Action::PrevPage { tab, object_tab });
                        }
                    });
                    ui.add_enabled_ui(can_next, |ui| {
                        if icon_button(ui, Icon::ChevronRight, &gettext(locale, "Next page"), &palette).clicked() {
                            actions.push(Action::NextPage { tab, object_tab });
                        }
                    });
                    let range = range.clone().unwrap_or_else(|| {
                        if loading { gettext(locale, "Loading…") } else { gettext(locale, "No rows") }.into_owned()
                    });
                    ui.label(RichText::new(range).color(palette.secondary));
                    if unordered {
                        ui.label(RichText::new(gettext(locale, "Unordered")).color(palette.dim))
                            .on_hover_text(gettext(locale, "This object has no primary key, so rows may move between pages."));
                    }
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if loading {
                        if icon_button(ui, Icon::CircleX, &gettext(locale, "Cancel query"), &palette).clicked() {
                            actions.push(Action::CancelQuery(tab));
                        }
                        ui.spinner();
                    } else if let Some(timing) = &timing {
                        ui.label(RichText::new(timing).color(palette.dim));
                    }
                });
            });
        });
    app.actions.extend(actions);
}

pub fn show(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId, object_tab: ObjectTabId) {
    let locale = app.locale;
    let palette = app.palette;
    let Some(object) = app.workspace(tab).and_then(|w| w.object_tab(object_tab)) else { return };
    let mut actions = Vec::new();
    if let Some(error) = &object.rows.error {
        error_box(ui, error, &palette, locale, || actions.push(Action::RetryRows { tab, object_tab }));
    } else if let Some(page) = object.page() {
        if page.rows.is_empty() {
            ui.centered_and_justified(|ui| {
                ui.label(RichText::new(gettext(locale, "No rows")).color(palette.secondary));
            });
        } else {
            let columns: Vec<Column<'_>> = page
                .columns
                .iter()
                .map(|column| Column {
                    name: &column.name,
                    type_name: &column.type_name,
                    numeric: column.kind == ValueKind::Numeric,
                    sort: object.sort_of(&column.name),
                })
                .collect();
            let output = grid::show(
                ui,
                Id::new(("grid", tab.0, object_tab.0)),
                &columns,
                page.rows.len(),
                object.query.offset,
                object.selection,
                &palette,
                |row, col| {
                    let value = &page.rows[row][col];
                    Cell { text: format::cell_text(value), null: value.is_null() }
                },
            );
            if let Some(cell) = output.clicked {
                actions.push(Action::SelectCell { tab, object_tab, cell });
            }
            if let Some(col) = output.sort_clicked {
                actions.push(Action::SortBy { tab, object_tab, column: page.columns[col].name.clone() });
            }
        }
    } else {
        ui.centered_and_justified(|ui| {
            ui.spinner();
        });
    }
    app.actions.extend(actions);
}

/// An error with its code, detail and hint, and a Retry button.
pub fn error_box(
    ui: &mut egui::Ui,
    error: &tabletist_db::Error,
    palette: &crate::theme::Palette,
    locale: crate::i18n::Locale,
    mut retry: impl FnMut(),
) {
    Frame::new()
        .fill(palette.danger.gamma_multiply(0.12))
        .inner_margin(Margin::same(12))
        .corner_radius(egui::CornerRadius::same(theme::RADIUS))
        .show(ui, |ui| {
            ui.label(RichText::new(error.to_string()).color(palette.text));
            if let tabletist_db::Error::Query { code, detail, hint, .. } = error {
                for (label, text) in [("Code", code), ("Detail", detail), ("Hint", hint)] {
                    if let Some(text) = text {
                        ui.label(RichText::new(format!("{}: {text}", gettext(locale, label))).color(palette.secondary));
                    }
                }
            }
            if ui.button(gettext(locale, "Retry")).clicked() {
                retry();
            }
        });
}
```

`gettext` takes `&'static str`; the labels in the loop above are literals, so they satisfy it.

- [ ] **Step 6: Compose the workspace**

Replace `show` in `src/ui/workspace.rs` (keep `top_bar` and `banner` from Batch 2, but change `banner` so the `Connecting` spinner only fills the body when the tree has never loaded):

```rust
pub fn show(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId) {
    top_bar(app, ui, tab);
    banner(app, ui, tab);
    let Some(workspace) = app.workspace(tab) else { return };
    if workspace.tree.schemas.value.is_none() && workspace.tree.schemas.error.is_none() {
        return; // still connecting; the banner shows progress
    }
    let active = workspace.active_object;
    let view = workspace.active_object_tab().map(|object| object.view);
    let row_panel = workspace.row_panel;
    super::sidebar::show(app, ui, tab);
    if let (Some(object_tab), Some(ObjectView::Data), true) = (active, view, row_panel) {
        super::row_panel::show(app, ui, tab, object_tab);
    }
    egui::CentralPanel::default()
        .frame(Frame::new().fill(app.palette.window))
        .show(ui, |ui| {
            super::object_tabs::show(app, ui, tab);
            match active {
                Some(object_tab) => {
                    super::data_view::footer(app, ui, tab, object_tab);
                    egui::CentralPanel::default()
                        .frame(Frame::new().fill(app.palette.window))
                        .show(ui, |ui| match view {
                            Some(ObjectView::Structure) => super::structure::show(app, ui, tab, object_tab),
                            _ => super::data_view::show(app, ui, tab, object_tab),
                        });
                }
                None => {
                    ui.centered_and_justified(|ui| {
                        ui.label(
                            RichText::new(gettext(app.locale, "Select a table or view in the sidebar"))
                                .color(app.palette.secondary),
                        );
                    });
                }
            }
        });
}
```

In `banner`, change the `Connecting` arm so it only draws the spinner inline (a small `ui.horizontal` with spinner and "Connecting…") instead of `centered_and_justified`, since the body now fills the rest.

Add `use crate::model::ObjectView;` to `workspace.rs`, and to `src/ui/mod.rs` add `pub mod data_view; pub mod object_tabs; pub mod row_panel; pub mod sidebar; pub mod structure;`. Create `src/ui/row_panel.rs` and `src/ui/structure.rs` as stubs for now (Task 6 fills them):

```rust
pub fn show(_app: &mut crate::app::App, _ui: &mut egui::Ui, _tab: crate::model::ConnTabId, _object_tab: crate::model::ObjectTabId) {}
```

- [ ] **Step 7: Run the tests to verify they pass**

Run: `cargo test --lib`
Expected: PASS. If `click("users")` finds the object tab instead of the sidebar row, confirm the object tab's label ends with " tab".

- [ ] **Step 8: Commit**

```bash
git add src/ui
git commit -m "Add the sidebar tree, object tabs and the Data view"
```

---

### Task 6: Row panel, Structure view, and keyboard shortcuts

**Files:**
- Modify: `src/ui/row_panel.rs`, `src/ui/structure.rs`, `src/ui/keys.rs`, `src/testing.rs`

**Interfaces:**
- Consumes: `format::{full_text, plain_text, COLLAPSE_LINES}`, `Structure`, `App::copy_text`, `App::active_object`.
- Produces: `row_panel::show(app, ui, tab, object_tab)`; `structure::show(app, ui, tab, object_tab)`; shortcuts from spec 5.10 for object tabs, paging, refresh, cancel, row panel, selection movement, and copy; `testing::Harness::copied: Option<String>` (set from `OutputCommand::CopyText`), `Harness::copy(&mut self, shift: bool)`.

- [ ] **Step 1: Extend the harness to capture copies**

In `src/testing.rs`, add a field `pub copied: Option<String>` to `Harness` (initialised to `None`), and in `frame` capture it before returning:

```rust
        for command in &output.platform_output.commands {
            if let egui::OutputCommand::CopyText(text) = command {
                self.copied = Some(text.clone());
            }
        }
```

Add a helper that sends egui's copy event with the given Shift state:

```rust
    pub fn copy(&mut self, shift: bool) {
        self.settle();
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, self.size)),
            events: vec![egui::Event::Copy],
            modifiers: egui::Modifiers { shift, ..egui::Modifiers::COMMAND },
            ..Default::default()
        };
        let app = &mut self.app;
        let output = self.ctx.run_ui(input, |ui| app.frame_ui(ui));
        for command in &output.platform_output.commands {
            if let egui::OutputCommand::CopyText(text) = command {
                self.copied = Some(text.clone());
            }
        }
    }
```

- [ ] **Step 2: Write the failing tests**

Add to the tests in `src/ui/mod.rs`:

```rust
    fn with_page(harness: &mut Harness) -> crate::model::ConnTabId {
        let tab = harness.connect_fake();
        harness.click("users");
        harness.answer_rows(crate::testing::page(5, false));
        tab
    }

    fn selection(harness: &Harness, tab: crate::model::ConnTabId) -> Option<crate::model::CellPos> {
        harness.app.workspace(tab).unwrap().active_object_tab().unwrap().selection
    }

    #[test]
    fn clicking_a_row_fills_the_row_panel_and_its_copy_buttons_copy_full_values() {
        let mut harness = Harness::new();
        let tab = with_page(&mut harness);
        assert!(harness.has("Select a row to see its fields"));
        harness.click("Row 1");
        assert_eq!(selection(&harness, tab), Some(crate::model::CellPos { row: 0, col: 0 }));
        assert!(harness.has("Copy email"));
        harness.click("Copy meta");
        assert_eq!(harness.copied.as_deref(), Some(r#"{"plan":"pro"}"#));
    }

    #[test]
    fn arrow_keys_move_the_selection_and_copy_takes_the_cell_or_row() {
        let mut harness = Harness::new();
        let tab = with_page(&mut harness);
        harness.press(Key::ArrowDown, Modifiers::NONE);
        assert_eq!(selection(&harness, tab), Some(crate::model::CellPos { row: 0, col: 0 }));
        harness.press(Key::ArrowDown, Modifiers::NONE);
        harness.press(Key::ArrowRight, Modifiers::NONE);
        assert_eq!(selection(&harness, tab), Some(crate::model::CellPos { row: 1, col: 1 }));
        harness.press(Key::End, Modifiers::NONE);
        assert_eq!(selection(&harness, tab), Some(crate::model::CellPos { row: 4, col: 1 }));
        harness.copy(false);
        assert_eq!(harness.copied.as_deref(), Some("user5@example.com"));
        harness.copy(true);
        assert_eq!(harness.copied.as_deref(), Some("5\tuser5@example.com\tNULL"));
    }

    #[test]
    fn space_and_ctrl_shift_r_toggle_the_row_panel() {
        let mut harness = Harness::new();
        let tab = with_page(&mut harness);
        harness.press(Key::Space, Modifiers::NONE);
        assert!(!harness.app.workspace(tab).unwrap().row_panel);
        harness.press(Key::R, Modifiers::COMMAND | Modifiers::SHIFT);
        assert!(harness.app.workspace(tab).unwrap().row_panel);
    }

    #[test]
    fn object_tab_and_page_shortcuts_work() {
        let mut harness = Harness::new();
        let tab = harness.connect_fake();
        harness.app.apply(crate::model::Action::OpenObject {
            tab,
            object: tabletist_db::ObjectRef::new("main", "users"),
            kind: tabletist_db::ObjectKind::Table,
            pin: true,
        });
        harness.answer_rows(crate::testing::page(300, true));
        harness.press(Key::ArrowRight, Modifiers::COMMAND | Modifiers::ALT);
        match crate::testing::last_sent(&harness.app) {
            Command::FetchRows { query, .. } => assert_eq!(query.offset, 300),
            other => panic!("{other:?}"),
        }
        harness.press(Key::Period, Modifiers::COMMAND);
        assert!(matches!(crate::testing::last_sent(&harness.app), Command::Cancel { .. }));
        harness.press(Key::R, Modifiers::COMMAND);
        assert!(matches!(crate::testing::last_sent(&harness.app), Command::FetchRows { .. }));
        harness.press(Key::W, Modifiers::COMMAND);
        assert!(harness.app.workspace(tab).unwrap().objects.is_empty());
        assert_eq!(harness.app.tabs.len(), 1, "Cmd+W closes the object tab, not the connection");
    }

    #[test]
    fn a_huge_single_line_value_is_collapsed_in_the_row_panel() {
        let mut harness = Harness::new();
        harness.connect_fake();
        harness.click("users");
        let mut page = crate::testing::page(1, false);
        page.rows[0][1] = tabletist_db::Value::Text("x".repeat(1_000_000).into());
        harness.answer_rows(page);
        harness.click("Row 1");
        let tree = harness.settle();
        assert!(crate::testing::labels(&tree).iter().any(|label| label.starts_with("Show all")));
    }

    #[test]
    fn the_structure_view_lists_columns_indexes_and_keys() {
        let mut harness = Harness::new();
        let tab = with_page(&mut harness);
        harness.click("Structure");
        let (session, request) = match crate::testing::last_sent(&harness.app) {
            Command::Describe { session, request, .. } => (*session, *request),
            other => panic!("{other:?}"),
        };
        let structure = tabletist_db::Structure {
            columns: vec![tabletist_db::ColumnInfo {
                name: "email".into(),
                type_name: "TEXT".into(),
                nullable: false,
                default: None,
                comment: None,
            }],
            primary_key: vec!["id".into()],
            indexes: vec![tabletist_db::IndexInfo {
                name: "users_email_idx".into(),
                columns: vec!["email".into()],
                unique: true,
                primary: false,
                method: None,
            }],
            foreign_keys: Vec::new(),
        };
        harness.app.apply(crate::model::Action::Backend(crate::backend::Event::Structure {
            session,
            request,
            result: Ok(structure),
        }));
        assert!(harness.has("Columns"));
        assert!(harness.has("users_email_idx"));
        assert!(harness.has("No foreign keys"));
        assert!(harness.app.workspace(tab).is_some());
    }
```

- [ ] **Step 3: Run the tests to verify they fail**

Run: `cargo test --lib ui::tests`
Expected: FAIL (row panel and structure are stubs; shortcuts missing).

- [ ] **Step 4: Implement the row panel**

Replace `src/ui/row_panel.rs`:

```rust
//! The row panel: every field of the selected row, in full.

use egui::{Frame, Id, Margin, RichText, TextEdit};
use tabletist_db::ValueKind;

use crate::app::App;
use crate::i18n::gettext;
use crate::model::{ConnTabId, ObjectTabId};
use crate::theme::{self, Icon};
use crate::ui::format;
use crate::ui::widgets::icon_button;

pub fn show(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId, object_tab: ObjectTabId) {
    let locale = app.locale;
    let palette = app.palette;
    let Some(object) = app.workspace(tab).and_then(|w| w.object_tab(object_tab)) else { return };
    let filter_id = Id::new(("row-panel-filter", tab.0, object_tab.0));
    egui::Panel::right(Id::new(("row-panel", tab.0)))
        .resizable(true)
        .default_size(340.0)
        .size_range(240.0..=720.0)
        .frame(Frame::new().fill(palette.panel).inner_margin(Margin::same(10)))
        .show(ui, |ui| {
            let selected = object
                .selection
                .and_then(|cell| object.page().map(|page| (cell, page)))
                .and_then(|(cell, page)| page.rows.get(cell.row).map(|row| (cell, page, row)));
            let Some((cell, page, row)) = selected else {
                ui.centered_and_justified(|ui| {
                    ui.label(RichText::new(gettext(locale, "Select a row to see its fields")).color(palette.secondary));
                });
                return;
            };
            ui.label(
                RichText::new(format!(
                    "{} · {} {}",
                    object.object.name,
                    gettext(locale, "Row"),
                    object.query.offset + cell.row as u64 + 1
                ))
                .font(theme::semibold(14.0))
                .color(palette.text),
            );
            let mut filter: String = ui.data(|data| data.get_temp(filter_id)).unwrap_or_default();
            ui.add(TextEdit::singleline(&mut filter).hint_text(gettext(locale, "Filter fields")).desired_width(f32::INFINITY));
            ui.data_mut(|data| data.insert_temp(filter_id, filter.clone()));
            let needle = filter.trim().to_lowercase();
            ui.separator();
            egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                for (col, (column, value)) in page.columns.iter().zip(row.iter()).enumerate() {
                    if !needle.is_empty() && !column.name.to_lowercase().contains(&needle) {
                        continue;
                    }
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(&column.name).font(theme::semibold(12.5)).color(palette.text));
                        ui.label(RichText::new(&column.type_name).font(theme::regular(11.5)).color(palette.dim));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let label = format!("{} {}", gettext(locale, "Copy"), column.name);
                            if icon_button(ui, Icon::Copy, &label, &palette).clicked() {
                                ui.ctx().copy_text(format::plain_text(value));
                            }
                        });
                    });
                    if value.is_null() {
                        ui.label(RichText::new("NULL").font(theme::mono(12.0)).color(palette.dim));
                    } else {
                        let text = format::full_text(value, column.kind);
                        let expanded_id = Id::new(("row-panel-expanded", tab.0, object_tab.0, cell.row, col));
                        let expanded: bool = ui.data(|data| data.get_temp(expanded_id)).unwrap_or(false);
                        let lines = text.lines().count();
                        let size = format::human_size(text.len());
                        let long = lines > format::COLLAPSE_LINES || text.len() > format::COLLAPSE_CHARS;
                        let shown: String = if long && !expanded {
                            text.lines()
                                .take(format::COLLAPSE_LINES)
                                .collect::<Vec<_>>()
                                .join("\n")
                                .chars()
                                .take(format::COLLAPSE_CHARS)
                                .collect()
                        } else {
                            text
                        };
                        let mono = matches!(column.kind, ValueKind::Json | ValueKind::Binary);
                        let font = if mono { theme::mono(12.0) } else { theme::regular(13.0) };
                        ui.add(
                            TextEdit::multiline(&mut shown.as_str())
                                .font(font)
                                .desired_width(f32::INFINITY)
                                .desired_rows(1),
                        );
                        if long {
                            let label = if expanded {
                                gettext(locale, "Show less").into_owned()
                            } else {
                                format!("{} ({size})", gettext(locale, "Show all"))
                            };
                            if ui.link(label).clicked() {
                                ui.data_mut(|data| data.insert_temp(expanded_id, !expanded));
                            }
                        }
                    }
                    ui.add_space(6.0);
                }
            });
        });
}
```

- [ ] **Step 5: Implement the Structure view**

Replace `src/ui/structure.rs`:

```rust
//! The Structure view: columns, indexes and foreign keys as plain tables.

use egui::RichText;

use crate::app::App;
use crate::i18n::gettext;
use crate::model::{Action, ConnTabId, ObjectTabId};
use crate::theme;

pub fn show(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId, object_tab: ObjectTabId) {
    let locale = app.locale;
    let palette = app.palette;
    let Some(object) = app.workspace(tab).and_then(|w| w.object_tab(object_tab)) else { return };
    let mut actions = Vec::new();
    if let Some(error) = &object.structure.error {
        super::data_view::error_box(ui, error, &palette, locale, || actions.push(Action::RetryStructure { tab, object_tab }));
    } else if let Some(structure) = &object.structure.value {
        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            ui.add_space(8.0);
            heading(ui, &gettext(locale, "Columns"), &palette);
            egui::Grid::new(("structure-columns", object_tab.0)).striped(true).spacing([16.0, 6.0]).show(ui, |ui| {
                for title in ["Name", "Type", "Nullable", "Default", "Key"] {
                    ui.label(RichText::new(gettext(locale, title)).color(palette.secondary));
                }
                ui.end_row();
                for column in &structure.columns {
                    ui.label(&column.name);
                    ui.label(RichText::new(&column.type_name).font(theme::mono(12.0)));
                    ui.label(if column.nullable { gettext(locale, "yes") } else { gettext(locale, "no") });
                    ui.label(RichText::new(column.default.as_deref().unwrap_or("")).font(theme::mono(12.0)));
                    ui.label(if structure.primary_key.contains(&column.name) { "PK" } else { "" });
                    ui.end_row();
                }
            });
            ui.add_space(16.0);
            heading(ui, &gettext(locale, "Indexes"), &palette);
            if structure.indexes.is_empty() {
                ui.label(RichText::new(gettext(locale, "No indexes")).color(palette.secondary));
            } else {
                egui::Grid::new(("structure-indexes", object_tab.0)).striped(true).spacing([16.0, 6.0]).show(ui, |ui| {
                    for title in ["Name", "Columns", "Unique", "Method"] {
                        ui.label(RichText::new(gettext(locale, title)).color(palette.secondary));
                    }
                    ui.end_row();
                    for index in &structure.indexes {
                        ui.label(&index.name);
                        ui.label(index.columns.join(", "));
                        ui.label(if index.primary { "PK" } else if index.unique { "yes" } else { "" });
                        ui.label(index.method.as_deref().unwrap_or(""));
                        ui.end_row();
                    }
                });
            }
            ui.add_space(16.0);
            heading(ui, &gettext(locale, "Foreign keys"), &palette);
            if structure.foreign_keys.is_empty() {
                ui.label(RichText::new(gettext(locale, "No foreign keys")).color(palette.secondary));
            } else {
                egui::Grid::new(("structure-fks", object_tab.0)).striped(true).spacing([16.0, 6.0]).show(ui, |ui| {
                    for title in ["Name", "Columns", "References", "On update", "On delete"] {
                        ui.label(RichText::new(gettext(locale, title)).color(palette.secondary));
                    }
                    ui.end_row();
                    for key in &structure.foreign_keys {
                        ui.label(key.name.as_deref().unwrap_or("·"));
                        ui.label(key.columns.join(", "));
                        let target = if key.ref_columns.is_empty() {
                            format!("{}.{}", key.ref_schema, key.ref_table)
                        } else {
                            format!("{}.{} ({})", key.ref_schema, key.ref_table, key.ref_columns.join(", "))
                        };
                        ui.label(target);
                        ui.label(&key.on_update);
                        ui.label(&key.on_delete);
                        ui.end_row();
                    }
                });
            }
        });
    } else {
        ui.centered_and_justified(|ui| {
            ui.spinner();
        });
    }
    app.actions.extend(actions);
}

fn heading(ui: &mut egui::Ui, text: &str, palette: &crate::theme::Palette) {
    ui.label(RichText::new(text).font(theme::semibold(15.0)).color(palette.text));
    ui.add_space(4.0);
}
```

- [ ] **Step 6: Implement the shortcuts**

Replace `handle` in `src/ui/keys.rs`:

```rust
pub fn handle(app: &mut App, ctx: &egui::Context) {
    let active = app.active_tab_id();
    let object = app.active_object();
    let editing = ctx.text_edit_focused();
    let mut actions = Vec::new();
    ctx.input_mut(|input| {
        let mut key = |modifiers: Modifiers, key: Key, action: Action| {
            if input.consume_key(modifiers, key) {
                actions.push(action);
            }
        };
        // Shift variants first: egui ignores an extra Shift when matching.
        key(Modifiers::COMMAND | Modifiers::SHIFT, Key::W, Action::CloseConnTab(active));
        key(Modifiers::COMMAND | Modifiers::SHIFT, Key::R, Action::ToggleRowPanel(active));
        key(Modifiers::CTRL | Modifiers::SHIFT, Key::Tab, Action::CycleConnTab(-1));
        key(Modifiers::CTRL, Key::Tab, Action::CycleConnTab(1));
        key(Modifiers::COMMAND, Key::T, Action::NewConnTab);
        key(Modifiers::COMMAND, Key::N, Action::NewConnection);
        for (index, number) in NUMBERS.into_iter().enumerate() {
            key(Modifiers::COMMAND, number, Action::ActivateConnTabIndex(index));
        }
        key(Modifiers::COMMAND, Key::R, Action::Refresh(active));
        key(Modifiers::COMMAND, Key::Period, Action::CancelQuery(active));
        if let Some((tab, object_tab)) = object {
            // With Shift held, US layouts report `{` and `}`, so match both.
            for (pressed, step) in [
                (Key::OpenBracket, -1),
                (Key::OpenCurlyBracket, -1),
                (Key::CloseBracket, 1),
                (Key::CloseCurlyBracket, 1),
            ] {
                key(Modifiers::COMMAND | Modifiers::SHIFT, pressed, Action::CycleObjectTab { tab, step });
            }
            key(Modifiers::COMMAND | Modifiers::ALT, Key::ArrowLeft, Action::PrevPage { tab, object_tab });
            key(Modifiers::COMMAND | Modifiers::ALT, Key::ArrowRight, Action::NextPage { tab, object_tab });
            key(Modifiers::COMMAND, Key::W, Action::CloseObjectTab { tab, object_tab });
            if !editing {
                let page = 20;
                for (pressed, rows, cols) in [
                    (Key::ArrowUp, -1, 0),
                    (Key::ArrowDown, 1, 0),
                    (Key::ArrowLeft, 0, -1),
                    (Key::ArrowRight, 0, 1),
                    (Key::PageUp, -page, 0),
                    (Key::PageDown, page, 0),
                    (Key::Home, isize::MIN, 0),
                    (Key::End, isize::MAX, 0),
                ] {
                    key(Modifiers::NONE, pressed, Action::MoveSelection { tab, object_tab, rows, cols });
                }
                key(Modifiers::NONE, Key::Space, Action::ToggleRowPanel(tab));
            }
        }
    });
    if !editing {
        let (copy, shift) = ctx.input(|input| {
            (input.events.iter().any(|event| matches!(event, egui::Event::Copy)), input.modifiers.shift)
        });
        if copy && let Some(text) = app.copy_text(shift) {
            ctx.copy_text(text);
        }
    }
    app.actions.extend(actions);
}
```

`Home`/`End` move within the column (`cols: 0`), matching spreadsheet behaviour for Cmd-less Home/End in a grid of one page.

- [ ] **Step 7: Run the tests to verify they pass**

Run: `cargo test --lib`
Expected: PASS.

- [ ] **Step 8: Commit**

```bash
git add src/ui src/testing.rs
git commit -m "Add the row panel, Structure view and browsing shortcuts"
```

---

### Task 7: Demo mode on the fixture, end-to-end test, and screenshots

**Files:**
- Modify: `src/entrypoint.rs`, `src/testing.rs`

**Interfaces:**
- Consumes: `tabletist_db::fixtures::write_sqlite_demo`, `Backend::start`, everything above.
- Produces: `entrypoint::demo_setup(app: &mut App)` writes `demo.sqlite` into the demo profile's state directory, saves a "Demo" connection (blue), connects the active tab, and queues `main.users` to open; `testing::Harness::with_backend(size, backend) -> Harness`; `Harness::run_until(&mut self, timeout: Duration, done: impl Fn(&App) -> bool) -> bool`.

- [ ] **Step 1: Write the failing end-to-end test**

Add to `src/testing.rs`:

```rust
impl Harness {
    pub fn with_backend(size: egui::Vec2, backend: crate::backend::Backend) -> Self {
        let dir = tempfile::tempdir().expect("temp dir");
        let ctx = egui::Context::default();
        ctx.enable_accesskit();
        let mut app = App::new(AppDirs::at(dir.path()), Settings::default(), backend);
        app.attach(&ctx, false);
        Self { app, ctx, size, copied: None, _dir: dir }
    }

    /// Runs frames until `done` holds or `timeout` passes. Returns whether it held.
    pub fn run_until(&mut self, timeout: std::time::Duration, done: impl Fn(&App) -> bool) -> bool {
        let deadline = std::time::Instant::now() + timeout;
        while std::time::Instant::now() < deadline {
            self.frame(Vec::new());
            if done(&self.app) {
                self.settle();
                return true;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        false
    }
}
```

(Update `Harness::with_size` to call `with_backend(size, Backend::recording())`.)

Add to the tests in `src/entrypoint.rs`:

```rust
    #[test]
    fn demo_mode_browses_the_fixture_end_to_end_at_every_size() {
        use crate::testing::Harness;
        for size in [egui::vec2(720.0, 480.0), egui::vec2(1280.0, 800.0), egui::vec2(2560.0, 1440.0)] {
            let backend = crate::backend::Backend::start(crate::backend::Waker::default());
            let mut harness = Harness::with_backend(size, backend);
            demo_setup(&mut harness.app);
            let loaded = harness.run_until(std::time::Duration::from_secs(20), |app| {
                app.workspace(app.active_tab_id())
                    .and_then(|workspace| workspace.active_object_tab())
                    .and_then(|object| object.page())
                    .is_some_and(|page| page.rows.len() == 5)
            });
            assert!(loaded, "the demo users table loads at {size:?}");
            assert!(harness.has("users tab"));
            assert!(harness.has("email"));
            assert!(harness.has("1–5 of 5"));
            harness.click("Row 3");
            assert!(harness.has("Copy email"));
            harness.click("Structure");
            assert!(harness.run_until(std::time::Duration::from_secs(10), |app| {
                app.workspace(app.active_tab_id())
                    .and_then(|workspace| workspace.active_object_tab())
                    .is_some_and(|object| object.structure.value.is_some())
            }));
            assert!(harness.has("users_name_idx"));
        }
    }
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test --lib demo_mode_browses`
Expected: FAIL: `the demo users table loads` (demo_setup is still empty).

- [ ] **Step 3: Implement demo_setup**

Replace `demo_setup` in `src/entrypoint.rs`:

```rust
/// Fills a demo profile: the shared SQLite fixture as a saved "Demo"
/// connection, connected in the first tab with the users table open.
pub fn demo_setup(app: &mut App) {
    let path = app.dirs.state.join("demo.sqlite");
    if let Err(error) = std::fs::create_dir_all(&app.dirs.state) {
        log::error!("could not create the demo profile: {error}");
        return;
    }
    if !path.exists()
        && let Err(error) = tabletist_db::fixtures::write_sqlite_demo(&path)
    {
        log::error!("could not write the demo database: {error}");
        return;
    }
    let saved = crate::connections::SavedConnection {
        id: crate::connections::ConnectionId::new(),
        name: "Demo".into(),
        color: crate::connections::ColorTag::Blue,
        spec: tabletist_db::ConnectSpec::sqlite(&path),
    };
    let conn = saved.id.clone();
    app.connections.upsert(saved);
    let tab = app.active_tab_id();
    app.apply(crate::model::Action::Connect { tab, conn });
    if let Some(workspace) = app.workspace_mut(tab) {
        workspace.pending_open = Some((
            tabletist_db::ObjectRef::new("main", "users"),
            tabletist_db::ObjectKind::Table,
        ));
    }
}
```

- [ ] **Step 4: Run all tests**

Run: `cargo test --locked --all-targets`
Expected: PASS.

- [ ] **Step 5: Look at it**

Run: `cargo run -- --demo --demo-shot /tmp/tabletist-batch3.png --demo-size 1280x800`
Expected: `/tmp/tabletist-batch3.png` shows the Demo tab, the sidebar with `main`, Tables and Views, the `users` tab with five rows, the footer "1–5 of 5", and the row panel's empty state. Check it in both a light and a dark desktop theme (on Omarchy, switch themes and rerun).

Run: `cargo run -- --demo`
Expected, by hand: click `big` (preview tab, italic), page through with Cmd/Ctrl+Alt+Right, sort by `label`, select a cell and move with arrows while the row panel follows, copy with Cmd/Ctrl+C and Shift+Cmd/Ctrl+C, open `weird "name"`, switch to Structure on `orders` and see its foreign key, press Cmd/Ctrl+. during a long page and see "the query was cancelled" with Retry.

- [ ] **Step 6: Full checks and commit**

Run: `cargo fmt --all --check && cargo clippy --locked --all-targets -- -D warnings && cargo test --locked --all-targets && RUSTDOCFLAGS='-D warnings' cargo doc --locked --no-deps`
Expected: all pass.

```bash
git add src
git commit -m "Open the SQLite fixture in demo mode and test browsing end to end"
```

---

## Done when

- `cargo run -- --demo` shows a working browser over the fixture: tree, preview/pinned tabs, paging, sorting, selection with keyboard, row panel with full values, Structure view, cancel.
- Real SQLite files saved in Batch 2 browse the same way.
- All checks pass on Linux, macOS, and Windows CI.

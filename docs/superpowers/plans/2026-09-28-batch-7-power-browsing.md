# Batch 7: Power Browsing Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make Tabletist fast to drive: an exact row count on demand, a filter bar (column/operator/value rows plus raw WHERE), cancel for every running query, quick open (Cmd/Ctrl+P), arrow-key navigation in the sidebar tree, and a shortcuts dialog (`?`) listing the full keyboard map.

**Architecture:** Everything is app-side; `tabletist-db` already has `RowQuery::filters`, `raw_where`, `count_rows` and cancel, and the backend already runs `Command::CountRows` and emits `Event::Count`. Each object tab gains a `count: Fetch<u64>` and a `FilterBar`; applying filters rewrites `query.filters`/`raw_where`, resets paging and the count, and refetches. Quick open and help are new `Dialog` variants. The workspace remembers which pane has keyboard focus (`Pane::Tree` or `Pane::Grid`) and the tree keeps a cursor, so arrow keys go to the pane the user last worked in. One `SHORTCUTS` table in `ui/keys.rs` feeds the help dialog.

**Tech Stack:** Rust, egui 0.36 (crmne fork), existing crates only (no new dependencies; the fuzzy matcher is ~30 lines).

**Spec:** `docs/superpowers/specs/2026-09-27-tabletist-design.md` (sections 5.5, 5.6, 5.9, 5.10; batch 7 in section 9)

## Global Constraints

- Everything from earlier batches' Global Constraints still applies. Verify with `cargo test --locked --workspace --all-targets` and `cargo clippy --locked --workspace --all-targets -- -D warnings`; run `cargo fmt --all` before every commit; commits are signed (unsigned while the agent is locked, re-signed before push).
- No new dependencies.
- Views push `Action`s; `App::apply` reduces them. UI code may edit plain form state in place (as the connection dialog does), but anything that queries goes through an action.
- Every request result is stale-guarded by its `RequestId` through `Fetch<T>`; a result for an older query (before a filter change, refresh or page move) is dropped.
- Keyboard: all shortcuts live in `ui/keys.rs` and are suppressed while a text field has focus, except the Command-chords that make sense there (Cmd/Ctrl+F, Cmd/Ctrl+P, Cmd/Ctrl+., Enter/Escape inside the filter bar and quick open, which those widgets handle themselves). "Command" is Cmd on macOS and Ctrl elsewhere.
- User-facing strings go through `gettext`; never use em dashes.
- One topic per commit on `main`.

## Review Focus

1. **A filter value the database cannot use** (text in a number column, a bad date, a raw WHERE with a syntax error): the error shows inline with Retry, the filter bar keeps its rows, and fixing the value and applying again recovers. Test: Task 2 `a_failed_filter_keeps_the_bar_and_applying_again_recovers`.
2. **A count or page that arrives after the filters changed:** dropped, never shown as the total for the new filters. Test: Task 2 `a_count_for_old_filters_is_dropped`.
3. **Keys typed into the filter bar or quick open field** (`?`, arrows, Space, Cmd/Ctrl+C): edit the text; they never move the grid or tree, open help, or copy a cell. Test: Task 5 `typing_a_question_mark_in_a_field_does_not_open_help` and Task 4 `arrows_in_the_filter_bar_do_not_move_the_tree`.
4. **Quick open with nothing loaded, or with thousands of objects:** a hint says only loaded schemas are searched; results are capped at 50 and a prefix match ranks above a scattered one. Test: Task 3 `quick_open_ranks_and_caps_results` and `quick_open_explains_what_it_searches`.
5. **A tree cursor whose row disappears** (filter typed, schema collapsed, refresh): the next key starts from a visible row instead of doing nothing or panicking. Test: Task 4 `a_cursor_on_a_hidden_row_restarts_at_the_top`.

---

## File Structure

```
src/model.rs            ObjectTab.count, FilterBar/FilterRow, Pane, Tree.cursor,
                        TreeKey, QuickOpen, Dialog::{QuickOpen, Help}, new Actions
src/app.rs              count, filter, quick open, tree key and help reducers
src/util.rs             fuzzy_score()
src/ui/format.rs        range_label() takes an exact total; group_digits()
src/ui/data_view.rs     footer: Count button, exact total, "Filtered", stop while counting
src/ui/filter_bar.rs    the filter bar (new)
src/ui/quick_open.rs    the quick open dialog (new)
src/ui/help.rs          the shortcuts dialog (new)
src/ui/sidebar.rs       tree cursor highlight; clicks set the cursor
src/ui/workspace.rs     draws the filter bar above the grid
src/ui/keys.rs          SHORTCUTS table; Cmd/Ctrl+F, Cmd/Ctrl+P, ?, tree keys, pane routing
src/ui/mod.rs           dispatch new dialogs; headless UI tests
```

---

### Task 1: Exact count and cancel while counting

**Files:**
- Modify: `src/model.rs`, `src/app.rs`, `src/ui/format.rs`, `src/ui/data_view.rs`
- Test: `src/app.rs` tests, `src/ui/format.rs` tests, `src/ui/mod.rs` tests

**Interfaces:**
- Produces: `ObjectTab.count: Fetch<u64>`; `Action::CountRows { tab: ConnTabId, object_tab: ObjectTabId }`; `App::count_rows(tab, id)`; `ObjectTab::reset_count(&mut self)` (clears value, pending and error); `format::range_label(offset, shown, has_more, estimate, exact: Option<u64>)`; `format::group_digits(u64) -> String`.

- [ ] **Step 1: Write the failing tests**

`src/ui/format.rs` tests:

```rust
    #[test]
    fn an_exact_total_replaces_the_estimate() {
        assert_eq!(
            range_label(0, 300, true, Some(90_000), Some(123_456)).as_deref(),
            Some("1–300 of 123,456")
        );
        assert_eq!(
            range_label(0, 300, true, Some(90_000), None).as_deref(),
            Some("1–300 of ~90K")
        );
        assert_eq!(group_digits(0), "0");
        assert_eq!(group_digits(1_234_567), "1,234,567");
    }
```

(Existing `range_label` calls in tests and `data_view.rs` gain a trailing `None`.)

`src/app.rs` tests (next to the paging tests; `connected_postgres` and the Rows helpers already exist there, reuse whichever opens an object tab and answers its first page, e.g. `testing::page` with `Harness`):

```rust
    fn open_users(harness: &mut Harness) -> (ConnTabId, ObjectTabId) {
        let tab = harness.connect_fake();
        harness.app.apply(Action::OpenObject {
            tab,
            object: ObjectRef::new("main", "users"),
            kind: ObjectKind::Table,
            pin: true,
        });
        harness.answer_rows(page(3, false));
        let id = harness.app.workspace(tab).unwrap().active_object.unwrap();
        (tab, id)
    }

    #[test]
    fn count_sends_the_query_and_shows_the_exact_total() {
        let mut harness = Harness::new();
        let (tab, id) = open_users(&mut harness);
        harness.app.apply(Action::CountRows { tab, object_tab: id });
        let Some(Command::CountRows { session, request, query }) = harness.app.backend.sent.last() else {
            panic!("expected CountRows");
        };
        assert_eq!(query.object.name, "users");
        let (session, request) = (*session, *request);
        harness.app.apply(Action::Backend(Event::Count { session, request, result: Ok(42) }));
        let object = harness.app.workspace(tab).unwrap().object_tab(id).unwrap();
        assert_eq!(object.count.value, Some(42));
    }

    #[test]
    fn a_cancelled_count_keeps_the_rows() {
        let mut harness = Harness::new();
        let (tab, id) = open_users(&mut harness);
        harness.app.apply(Action::CountRows { tab, object_tab: id });
        harness.app.apply(Action::CancelQuery(tab));
        assert!(matches!(harness.app.backend.sent.last(), Some(Command::Cancel { .. })));
        let request = harness.app.workspace(tab).unwrap().object_tab(id).unwrap().count.pending.unwrap();
        let session = harness.app.workspace(tab).unwrap().session;
        harness.app.apply(Action::Backend(Event::Count {
            session,
            request,
            result: Err(tabletist_db::Error::Cancelled),
        }));
        let object = harness.app.workspace(tab).unwrap().object_tab(id).unwrap();
        assert_eq!(object.count.value, None);
        assert!(object.count.error.is_some());
        assert_eq!(object.page().unwrap().rows.len(), 3, "rows untouched");
    }

    #[test]
    fn refresh_forgets_the_count() {
        let mut harness = Harness::new();
        let (tab, id) = open_users(&mut harness);
        harness.app.apply(Action::CountRows { tab, object_tab: id });
        let (session, request) = match harness.app.backend.sent.last() {
            Some(Command::CountRows { session, request, .. }) => (*session, *request),
            other => panic!("{other:?}"),
        };
        harness.app.apply(Action::Backend(Event::Count { session, request, result: Ok(42) }));
        harness.app.apply(Action::Refresh(tab));
        assert_eq!(harness.app.workspace(tab).unwrap().object_tab(id).unwrap().count.value, None);
    }
```

`src/ui/mod.rs` test:

```rust
    #[test]
    fn the_footer_offers_count_and_then_shows_the_total() {
        let mut harness = Harness::new();
        let tab = harness.connect_fake();
        harness.app.apply(Action::OpenObject {
            tab,
            object: tabletist_db::ObjectRef::new("main", "users"),
            kind: tabletist_db::ObjectKind::Table,
            pin: true,
        });
        harness.answer_rows(crate::testing::page(3, true));
        harness.click("Count");
        let (session, request) = match harness.app.backend.sent.last() {
            Some(Command::CountRows { session, request, .. }) => (*session, *request),
            other => panic!("{other:?}"),
        };
        harness.app.apply(Action::Backend(Event::Count { session, request, result: Ok(1234) }));
        assert!(harness.has("1–3 of 1,234"));
        assert!(!harness.has("Count"));
    }
```

- [ ] **Step 2: Run them to watch them fail**

Run: `cargo test --locked --lib -- count range_label group_digits`
Expected: compile errors (`CountRows` action, `count` field, new `range_label` argument).

- [ ] **Step 3: Implement**

`model.rs`: `ObjectTab` gains `pub count: Fetch<u64>` (initialized `Fetch::default()`), and

```rust
    /// Forgets the exact count: the rows it counted changed (filters,
    /// refresh), and a count still running for them is ignored.
    pub fn reset_count(&mut self) {
        self.count = Fetch::default();
    }
```

`Action::CountRows { tab: ConnTabId, object_tab: ObjectTabId }`.

`app.rs`:

```rust
            Action::CountRows { tab, object_tab } => self.count_rows(tab, object_tab),
```

```rust
    pub fn count_rows(&mut self, tab: ConnTabId, id: ObjectTabId) {
        let request = RequestId(self.next_id());
        let Some(workspace) = self.workspace_mut(tab) else {
            return;
        };
        let session = workspace.session;
        let Some(object) = workspace.object_tab_mut(id) else {
            return;
        };
        object.count.start(request);
        let query = object.query.clone();
        self.backend.send(Command::CountRows { session, request, query });
    }
```

`Event::Count { session, request, result }` (today falls to the `other =>` debug arm): find the workspace by session and the object tab whose `count.pending == Some(request)`, then `object.count.finish(request, result)`. `Action::Refresh` resets every refreshed object tab's count (`reset_count`) before refetching.

`format.rs`:

```rust
/// `1234567` as `1,234,567`.
pub fn group_digits(number: u64) -> String {
    let digits = number.to_string();
    let mut out = String::new();
    for (index, digit) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index) % 3 == 0 {
            out.push(',');
        }
        out.push(digit);
    }
    out
}
```

`range_label` takes `exact: Option<u64>` last; when `Some(total)`, the suffix is `format!(" of {}", group_digits(total))` whatever `has_more` says.

`data_view.rs` footer:
- `loading` also includes `object.count.is_loading()`, so the stop button and Cmd/Ctrl+. cover a running count.
- Pass `object.count.value` to `range_label`.
- In the Data view, after the range label: when the count is neither loaded nor loading, a small `Count` button (`Action::CountRows`); if `object.count.error` is set, its hover text is the error. While counting, a label `Counting…` in `palette.dim`.

- [ ] **Step 4: Run and commit**

Run: `cargo test --locked --workspace --all-targets`
Expected: PASS.

```bash
cargo fmt --all
git add src
git commit -m "Count rows exactly on demand, with cancel while counting"
```

---

### Task 2: The filter bar

**Files:**
- Create: `src/ui/filter_bar.rs`
- Modify: `src/model.rs`, `src/app.rs`, `src/ui/workspace.rs`, `src/ui/data_view.rs`, `src/ui/keys.rs`, `src/ui/mod.rs`

**Interfaces:**
- Consumes: `ObjectTab.count`, `reset_count` (Task 1).
- Produces: `FilterBar { open: bool, rows: Vec<FilterRow>, raw: bool, raw_text: String, focus: bool }`, `FilterRow { column: String, op: FilterOp, value: String }`, `ObjectTab.filter: FilterBar`; actions `ToggleFilterBar(ConnTabId)`, `AddFilterRow { tab, object_tab }`, `RemoveFilterRow { tab, object_tab, index: usize }`, `ApplyFilters { tab, object_tab }`, `ClearFilters { tab, object_tab }`; `op_label(FilterOp) -> &'static str` and `FILTER_OPS: [FilterOp; 11]` in `filter_bar.rs`; `FilterBar::to_query(&self) -> (Vec<Filter>, Option<String>)`.

- [ ] **Step 1: Write the failing tests**

`model.rs` tests:

```rust
    #[test]
    fn a_filter_bar_becomes_filters_and_a_raw_where() {
        let bar = FilterBar {
            open: true,
            rows: vec![
                FilterRow { column: "age".into(), op: FilterOp::Gt, value: "30".into() },
                // Empty value: skipped, except for the NULL tests.
                FilterRow { column: "name".into(), op: FilterOp::Contains, value: " ".into() },
                FilterRow { column: "email".into(), op: FilterOp::IsNull, value: String::new() },
                // No column: skipped.
                FilterRow { column: String::new(), op: FilterOp::Eq, value: "x".into() },
            ],
            raw: true,
            raw_text: "  id % 2 = 0 ".into(),
            focus: false,
        };
        let (filters, raw) = bar.to_query();
        assert_eq!(
            filters,
            vec![
                Filter { column: "age".into(), op: FilterOp::Gt, value: "30".into() },
                Filter { column: "email".into(), op: FilterOp::IsNull, value: String::new() },
            ]
        );
        assert_eq!(raw.as_deref(), Some("id % 2 = 0"));
        let off = FilterBar { raw: false, ..bar.clone() };
        assert_eq!(off.to_query().1, None);
    }
```

`app.rs` tests (reuse Task 1's `open_users`):

```rust
    #[test]
    fn applying_filters_refetches_from_the_first_page_and_forgets_the_count() {
        let mut harness = Harness::new();
        let (tab, id) = open_users(&mut harness);
        {
            let object = harness.app.workspace_mut(tab).unwrap().object_tab_mut(id).unwrap();
            object.query.offset = 300;
            object.count.value = Some(9);
            object.filter.rows = vec![FilterRow { column: "age".into(), op: FilterOp::Gt, value: "30".into() }];
        }
        harness.app.apply(Action::ApplyFilters { tab, object_tab: id });
        let Some(Command::FetchRows { query, .. }) = harness.app.backend.sent.last() else { panic!() };
        assert_eq!(query.offset, 0);
        assert_eq!(query.filters.len(), 1);
        let object = harness.app.workspace(tab).unwrap().object_tab(id).unwrap();
        assert_eq!(object.count.value, None);
        assert!(object.pinned);
    }

    #[test]
    fn a_count_for_old_filters_is_dropped() {
        let mut harness = Harness::new();
        let (tab, id) = open_users(&mut harness);
        harness.app.apply(Action::CountRows { tab, object_tab: id });
        let (session, request) = match harness.app.backend.sent.last() {
            Some(Command::CountRows { session, request, .. }) => (*session, *request),
            other => panic!("{other:?}"),
        };
        harness.app.workspace_mut(tab).unwrap().object_tab_mut(id).unwrap().filter.rows =
            vec![FilterRow { column: "age".into(), op: FilterOp::Gt, value: "30".into() }];
        harness.app.apply(Action::ApplyFilters { tab, object_tab: id });
        harness.app.apply(Action::Backend(Event::Count { session, request, result: Ok(1000) }));
        assert_eq!(harness.app.workspace(tab).unwrap().object_tab(id).unwrap().count.value, None);
    }

    #[test]
    fn a_failed_filter_keeps_the_bar_and_applying_again_recovers() {
        let mut harness = Harness::new();
        let (tab, id) = open_users(&mut harness);
        harness.app.apply(Action::ToggleFilterBar(tab));
        harness.app.workspace_mut(tab).unwrap().object_tab_mut(id).unwrap().filter.rows =
            vec![FilterRow { column: "age".into(), op: FilterOp::Gt, value: "old".into() }];
        harness.app.apply(Action::ApplyFilters { tab, object_tab: id });
        let (session, request) = match harness.app.backend.sent.last() {
            Some(Command::FetchRows { session, request, .. }) => (*session, *request),
            other => panic!("{other:?}"),
        };
        harness.app.apply(Action::Backend(Event::Rows {
            session,
            request,
            result: Err(tabletist_db::Error::query("invalid input syntax for type integer")),
        }));
        let object = harness.app.workspace(tab).unwrap().object_tab(id).unwrap();
        assert!(object.filter.open);
        assert_eq!(object.filter.rows[0].value, "old");
        assert!(object.rows.error.is_some());
        harness.app.workspace_mut(tab).unwrap().object_tab_mut(id).unwrap().filter.rows[0].value = "30".into();
        harness.app.apply(Action::ApplyFilters { tab, object_tab: id });
        harness.answer_rows(page(2, false));
        let object = harness.app.workspace(tab).unwrap().object_tab(id).unwrap();
        assert!(object.rows.error.is_none());
        assert_eq!(object.page().unwrap().rows.len(), 2);
    }

    #[test]
    fn clearing_filters_closes_the_bar_and_refetches_everything() {
        let mut harness = Harness::new();
        let (tab, id) = open_users(&mut harness);
        harness.app.apply(Action::ToggleFilterBar(tab));
        harness.app.workspace_mut(tab).unwrap().object_tab_mut(id).unwrap().filter.rows[0].value = "1".into();
        harness.app.apply(Action::ApplyFilters { tab, object_tab: id });
        harness.app.apply(Action::ClearFilters { tab, object_tab: id });
        let object = harness.app.workspace(tab).unwrap().object_tab(id).unwrap();
        assert!(!object.filter.open);
        assert!(object.query.filters.is_empty() && object.query.raw_where.is_none());
        assert!(matches!(harness.app.backend.sent.last(), Some(Command::FetchRows { query, .. }) if query.filters.is_empty()));
    }

    #[test]
    fn opening_the_filter_bar_starts_with_one_row_on_the_first_column() {
        let mut harness = Harness::new();
        let (tab, id) = open_users(&mut harness);
        harness.app.apply(Action::ToggleFilterBar(tab));
        let object = harness.app.workspace(tab).unwrap().object_tab(id).unwrap();
        assert!(object.filter.open && object.filter.focus);
        assert_eq!(object.filter.rows.len(), 1);
        assert_eq!(object.filter.rows[0].column, object.page().unwrap().columns[0].name);
        harness.app.apply(Action::ToggleFilterBar(tab));
        assert!(!harness.app.workspace(tab).unwrap().object_tab(id).unwrap().filter.open);
    }
```

`ui/mod.rs` test:

```rust
    #[test]
    fn command_f_opens_the_filter_bar_and_enter_applies_it() {
        let mut harness = Harness::new();
        let tab = harness.connect_fake();
        harness.app.apply(Action::OpenObject {
            tab,
            object: tabletist_db::ObjectRef::new("main", "users"),
            kind: tabletist_db::ObjectKind::Table,
            pin: true,
        });
        harness.answer_rows(crate::testing::page(3, false));
        harness.press(Key::F, Modifiers::COMMAND);
        assert!(harness.has("Apply"));
        assert!(harness.has("Raw WHERE"));
        // the value field has focus: type and press Enter
        harness.frame(vec![egui::Event::Text("30".into())]);
        harness.press(Key::Enter, Modifiers::NONE);
        assert!(matches!(harness.app.backend.sent.last(), Some(Command::FetchRows { query, .. }) if query.filters.len() == 1));
    }
```

- [ ] **Step 2: Run them to watch them fail**

Run: `cargo test --locked --lib -- filter`
Expected: compile errors for the new types and actions.

- [ ] **Step 3: Model and reducer**

`model.rs`:

```rust
/// One condition in the filter bar.
#[derive(Debug, Clone, PartialEq)]
pub struct FilterRow {
    pub column: String,
    pub op: FilterOp,
    pub value: String,
}

/// An object tab's filter bar: rows combined with AND, plus raw SQL.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct FilterBar {
    pub open: bool,
    pub rows: Vec<FilterRow>,
    /// Use `raw_text` as a WHERE clause as well.
    pub raw: bool,
    pub raw_text: String,
    /// Move keyboard focus into the bar on the next frame.
    pub focus: bool,
}

impl FilterBar {
    /// The filters and raw WHERE the bar describes. Rows without a column,
    /// or without a value for an operator that needs one, are skipped.
    pub fn to_query(&self) -> (Vec<Filter>, Option<String>) {
        let filters = self
            .rows
            .iter()
            .filter(|row| !row.column.is_empty())
            .filter(|row| {
                matches!(row.op, FilterOp::IsNull | FilterOp::IsNotNull)
                    || !row.value.trim().is_empty()
            })
            .map(|row| Filter {
                column: row.column.clone(),
                op: row.op,
                value: row.value.trim().to_owned(),
            })
            .collect();
        let raw = self.raw_text.trim();
        (filters, (self.raw && !raw.is_empty()).then(|| raw.to_owned()))
    }
}
```

`ObjectTab.filter: FilterBar` (default). Actions as listed in Interfaces.

`app.rs`:

```rust
            Action::ToggleFilterBar(tab) => {
                let Some((tab, id)) = self.active_object().filter(|(t, _)| *t == tab) else {
                    return;
                };
                if let Some(object) = self.object_tab_mut(tab, id) {
                    let first = object
                        .page()
                        .and_then(|page| page.columns.first())
                        .map(|column| column.name.clone())
                        .unwrap_or_default();
                    let bar = &mut object.filter;
                    bar.open = !bar.open;
                    bar.focus = bar.open;
                    if bar.open && bar.rows.is_empty() {
                        bar.rows.push(FilterRow { column: first, op: FilterOp::Eq, value: String::new() });
                    }
                }
            }
            Action::AddFilterRow { tab, object_tab } => {
                if let Some(object) = self.object_tab_mut(tab, object_tab) {
                    let column = object.filter.rows.last().map(|row| row.column.clone()).unwrap_or_default();
                    object.filter.rows.push(FilterRow { column, op: FilterOp::Eq, value: String::new() });
                    object.filter.focus = true;
                }
            }
            Action::RemoveFilterRow { tab, object_tab, index } => {
                if let Some(object) = self.object_tab_mut(tab, object_tab)
                    && index < object.filter.rows.len()
                {
                    object.filter.rows.remove(index);
                }
            }
            Action::ApplyFilters { tab, object_tab } => self.apply_filters(tab, object_tab),
            Action::ClearFilters { tab, object_tab } => {
                if let Some(object) = self.object_tab_mut(tab, object_tab) {
                    object.filter = FilterBar::default();
                }
                self.apply_filters(tab, object_tab);
            }
```

```rust
    /// Runs the object tab's query with its filter bar's conditions, from
    /// the first page.
    fn apply_filters(&mut self, tab: ConnTabId, id: ObjectTabId) {
        let Some(object) = self.object_tab_mut(tab, id) else {
            return;
        };
        let (filters, raw_where) = object.filter.to_query();
        object.query.filters = filters;
        object.query.raw_where = raw_where;
        object.query.offset = 0;
        object.pinned = true;
        object.selection = None;
        object.reset_count();
        self.fetch_rows(tab, id);
    }
```

(`self.object_tab_mut(tab, id)` already exists; `SortBy` uses it.)

- [ ] **Step 4: The bar**

`src/ui/filter_bar.rs`:

```rust
//! The filter bar above the grid: column, operator and value rows combined
//! with AND, and an optional raw WHERE clause.

use egui::{RichText, TextEdit};
use tabletist_db::FilterOp;

use crate::app::App;
use crate::i18n::gettext;
use crate::model::{Action, ConnTabId, ObjectTabId};
use crate::theme::Icon;
use crate::ui::widgets::icon_button;

pub const FILTER_OPS: [FilterOp; 11] = [
    FilterOp::Eq,
    FilterOp::Ne,
    FilterOp::Lt,
    FilterOp::Gt,
    FilterOp::Le,
    FilterOp::Ge,
    FilterOp::Contains,
    FilterOp::StartsWith,
    FilterOp::In,
    FilterOp::IsNull,
    FilterOp::IsNotNull,
];

pub fn op_label(op: FilterOp) -> &'static str {
    match op {
        FilterOp::Eq => "=",
        FilterOp::Ne => "≠",
        FilterOp::Lt => "<",
        FilterOp::Gt => ">",
        FilterOp::Le => "≤",
        FilterOp::Ge => "≥",
        FilterOp::Contains => "contains",
        FilterOp::StartsWith => "starts with",
        FilterOp::In => "in (a, b, …)",
        FilterOp::IsNull => "is NULL",
        FilterOp::IsNotNull => "is not NULL",
    }
}

pub fn show(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId, object_tab: ObjectTabId) {
    let locale = app.locale;
    let palette = app.palette;
    let Some(object) = app.workspace_mut(tab).and_then(|w| w.object_tab_mut(object_tab)) else {
        return;
    };
    if !object.filter.open {
        return;
    }
    // Columns from the page, else from the structure.
    let columns: Vec<String> = object
        .page()
        .map(|page| page.columns.iter().map(|c| c.name.clone()).collect())
        .or_else(|| {
            object.structure.value.as_ref().map(|s| s.columns.iter().map(|c| c.name.clone()).collect())
        })
        .unwrap_or_default();
    let mut actions = Vec::new();
    let mut apply = false;
    let focus = std::mem::take(&mut object.filter.focus);
    let bar = &mut object.filter;
    egui::Frame::new()
        .fill(palette.panel)
        .inner_margin(egui::Margin::symmetric(8, 6))
        .show(ui, |ui| {
            let count = bar.rows.len();
            for index in 0..count {
                let row = &mut bar.rows[index];
                ui.horizontal(|ui| {
                    egui::ComboBox::from_id_salt(("filter-column", tab.0, object_tab.0, index))
                        .selected_text(&row.column)
                        .width(160.0)
                        .show_ui(ui, |ui| {
                            for column in &columns {
                                ui.selectable_value(&mut row.column, column.clone(), column);
                            }
                        });
                    egui::ComboBox::from_id_salt(("filter-op", tab.0, object_tab.0, index))
                        .selected_text(gettext(locale, op_label(row.op)))
                        .width(110.0)
                        .show_ui(ui, |ui| {
                            for op in FILTER_OPS {
                                ui.selectable_value(&mut row.op, op, gettext(locale, op_label(op)));
                            }
                        });
                    if !matches!(row.op, FilterOp::IsNull | FilterOp::IsNotNull) {
                        let field = ui.add(
                            TextEdit::singleline(&mut row.value)
                                .hint_text(gettext(locale, "Value"))
                                .desired_width(220.0),
                        );
                        if focus && index + 1 == count {
                            field.request_focus();
                        }
                        if field.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                            apply = true;
                        }
                    }
                    if icon_button(ui, Icon::X, &gettext(locale, "Remove condition"), &palette).clicked() {
                        actions.push(Action::RemoveFilterRow { tab, object_tab, index });
                    }
                });
            }
            ui.horizontal(|ui| {
                if ui.button(gettext(locale, "+ Condition")).clicked() {
                    actions.push(Action::AddFilterRow { tab, object_tab });
                }
                ui.checkbox(&mut bar.raw, gettext(locale, "Raw WHERE"));
                if bar.raw {
                    let field = ui.add(
                        TextEdit::singleline(&mut bar.raw_text)
                            .font(crate::theme::mono(12.5))
                            .hint_text("id > 10 AND name LIKE 'A%'")
                            .desired_width(320.0),
                    );
                    if field.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        apply = true;
                    }
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button(gettext(locale, "Clear")).clicked() {
                        actions.push(Action::ClearFilters { tab, object_tab });
                    }
                    if ui.button(RichText::new(gettext(locale, "Apply")).color(palette.accent)).clicked() {
                        apply = true;
                    }
                });
            });
        });
    if apply {
        actions.push(Action::ApplyFilters { tab, object_tab });
    }
    app.actions.extend(actions);
}
```

(If `Icon::X` is not in the icon set, use the close icon the tab bar uses; ledger the name.)

`workspace.rs`: inside the central panel for the Data view, draw `super::filter_bar::show(app, ui, tab, object_tab)` in a `egui::Panel::top(Id::new(("filter-bar", tab.0, object_tab.0)))` above `data_view::show` (the bar sizes to its rows; use `.resizable(false)` and let it shrink when closed by not showing the panel when `!open`).

`data_view.rs` footer: when `!object.query.filters.is_empty() || object.query.raw_where.is_some()`, a `Filtered` label in `palette.accent` after the range (hover text: "Cmd/Ctrl+F edits the filter").

`keys.rs`: `key(Modifiers::COMMAND, Key::F, Action::ToggleFilterBar(active));` placed with the other Command chords (works while typing, so Cmd/Ctrl+F also closes the bar from inside it).

- [ ] **Step 5: Run and commit**

Run: `cargo test --locked --workspace --all-targets` (with `TABLETIST_TEST_PG_URL` and `TABLETIST_TEST_MYSQL_URL` set: the dialect filter tests there are the SQL side of this feature).
Expected: PASS.

```bash
cargo fmt --all
git add src
git commit -m "Add the filter bar with raw WHERE"
```

---

### Task 3: Quick open

**Files:**
- Create: `src/ui/quick_open.rs`
- Modify: `src/util.rs`, `src/model.rs`, `src/app.rs`, `src/ui/keys.rs`, `src/ui/mod.rs`

**Interfaces:**
- Produces: `util::fuzzy_score(query: &str, candidate: &str) -> Option<i64>`; `Dialog::QuickOpen(Box<QuickOpen>)` with `QuickOpen { tab: ConnTabId, query: String, selected: usize }`; `App::quick_open_matches(&self, tab, query) -> Vec<(ObjectRef, ObjectKind)>` (best first, at most 50); actions `OpenQuickOpen`, `QuickOpenMove(isize)`, `QuickOpenPick`.

- [ ] **Step 1: Write the failing tests**

`util.rs` tests:

```rust
    #[test]
    fn fuzzy_scores_prefer_prefixes_and_runs() {
        assert!(fuzzy_score("usr", "users").is_some());
        assert!(fuzzy_score("xyz", "users").is_none());
        assert_eq!(fuzzy_score("", "users"), Some(0));
        let prefix = fuzzy_score("ord", "orders").unwrap();
        let scattered = fuzzy_score("ord", "product_reviews_old").unwrap();
        assert!(prefix > scattered, "{prefix} vs {scattered}");
        let word = fuzzy_score("items", "order_items").unwrap();
        let inside = fuzzy_score("items", "subitemset").unwrap();
        assert!(word > inside, "{word} vs {inside}");
        assert!(fuzzy_score("USERS", "users").is_some(), "case-insensitive");
    }
```

`app.rs` tests:

```rust
    fn with_objects(harness: &mut Harness, tab: ConnTabId, schema: &str, names: &[&str]) {
        let workspace = harness.app.workspace_mut(tab).unwrap();
        let node = workspace.tree.nodes.entry(schema.to_owned()).or_default();
        node.objects.value = Some(
            names
                .iter()
                .map(|name| ObjectInfo {
                    object: ObjectRef::new(schema, *name),
                    kind: ObjectKind::Table,
                    estimated_rows: None,
                })
                .collect(),
        );
    }

    #[test]
    fn quick_open_ranks_and_caps_results() {
        let mut harness = Harness::new();
        let tab = harness.connect_fake();
        let many: Vec<String> = (0..200).map(|i| format!("log_{i:03}")).collect();
        let many: Vec<&str> = many.iter().map(String::as_str).collect();
        with_objects(&mut harness, tab, "main", &many);
        with_objects(&mut harness, tab, "shop", &["orders", "product_reviews_old"]);
        let matches = harness.app.quick_open_matches(tab, "ord");
        assert_eq!(matches[0].0.name, "orders");
        assert!(harness.app.quick_open_matches(tab, "log").len() <= 50);
        let qualified = harness.app.quick_open_matches(tab, "shop.ord");
        assert_eq!(qualified[0].0.name, "orders");
    }

    #[test]
    fn picking_a_quick_open_result_opens_and_pins_it() {
        let mut harness = Harness::new();
        let tab = harness.connect_fake();
        with_objects(&mut harness, tab, "shop", &["orders", "users"]);
        harness.app.apply(Action::OpenQuickOpen);
        let Some(Dialog::QuickOpen(open)) = &mut harness.app.dialog else { panic!() };
        open.query = "us".into();
        harness.app.apply(Action::QuickOpenPick);
        assert!(harness.app.dialog.is_none());
        let workspace = harness.app.workspace(tab).unwrap();
        let object = workspace.object_tab(workspace.active_object.unwrap()).unwrap();
        assert_eq!(object.object.name, "users");
        assert!(object.pinned);
    }
```

(Construct `ObjectInfo` with whatever fields it really has; `estimated_rows` is from the catalog. `SchemaNode` needs `Default`; add the derive if missing.)

`ui/mod.rs` tests:

```rust
    /// A connected tab whose `main` schema shows `orders` and `users`.
    fn tree_harness() -> (Harness, crate::model::ConnTabId) {
        let mut harness = Harness::new();
        let tab = harness.connect_fake();
        let workspace = harness.app.workspace_mut(tab).unwrap();
        workspace.tree.schemas.value = Some(vec!["main".into()]);
        let node = workspace.tree.nodes.entry("main".into()).or_default();
        node.expanded = true;
        node.objects.value = Some(
            ["orders", "users"]
                .into_iter()
                .map(|name| tabletist_db::ObjectInfo {
                    object: tabletist_db::ObjectRef::new("main", name),
                    kind: tabletist_db::ObjectKind::Table,
                    estimated_rows: None,
                })
                .collect(),
        );
        (harness, tab)
    }

    #[test]
    fn command_p_opens_quick_open_and_enter_opens_the_match() {
        let (mut harness, tab) = tree_harness();
        harness.press(Key::P, Modifiers::COMMAND);
        assert!(harness.has("Open table or view"));
        harness.frame(vec![egui::Event::Text("users".into())]);
        harness.press(Key::Enter, Modifiers::NONE);
        assert!(harness.app.dialog.is_none());
        let workspace = harness.app.workspace(tab).unwrap();
        assert!(workspace.active_object.is_some());
    }

    #[test]
    fn quick_open_explains_what_it_searches() {
        let mut harness = Harness::new();
        harness.connect_fake();
        harness.press(Key::P, Modifiers::COMMAND);
        assert!(harness.has("Searches schemas loaded in the sidebar"));
    }
```

- [ ] **Step 2: Run them to watch them fail**

Run: `cargo test --locked --lib -- fuzzy quick_open`
Expected: compile errors for the new names.

- [ ] **Step 3: The matcher**

`util.rs`:

```rust
/// How well `candidate` matches `query` typed in quick open: every query
/// character must appear in order (ignoring case); runs of adjacent
/// characters and matches at word starts score higher, and shorter
/// candidates win ties. `None` when it does not match.
pub fn fuzzy_score(query: &str, candidate: &str) -> Option<i64> {
    let query: Vec<char> = query
        .to_lowercase()
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect();
    if query.is_empty() {
        return Some(0);
    }
    let text: Vec<char> = candidate.to_lowercase().chars().collect();
    let mut score = 0i64;
    let mut next = 0;
    let mut previous: Option<usize> = None;
    for (index, &c) in text.iter().enumerate() {
        if next == query.len() {
            break;
        }
        if c != query[next] {
            continue;
        }
        score += 10;
        if previous.is_some_and(|p| p + 1 == index) {
            score += 15;
        }
        if index == 0 || matches!(text[index - 1], '.' | '_' | '-' | ' ') {
            score += 20;
        }
        previous = Some(index);
        next += 1;
    }
    (next == query.len()).then(|| score - text.len() as i64)
}
```

- [ ] **Step 4: Dialog, reducer, keys**

`model.rs`: `QuickOpen { tab, query, selected }`, `Dialog::QuickOpen(Box<QuickOpen>)`, `Action::{OpenQuickOpen, QuickOpenMove(isize), QuickOpenPick}`.

`app.rs`:

```rust
    /// Loaded objects of `tab`'s connection that match `query`, best first.
    pub fn quick_open_matches(&self, tab: ConnTabId, query: &str) -> Vec<(ObjectRef, ObjectKind)> {
        let Some(workspace) = self.workspace(tab) else {
            return Vec::new();
        };
        let mut scored: Vec<(i64, ObjectRef, ObjectKind)> = workspace
            .tree
            .nodes
            .values()
            .filter_map(|node| node.objects.value.as_ref())
            .flatten()
            .filter_map(|info| {
                let qualified = format!("{}.{}", info.object.schema, info.object.name);
                let score = crate::util::fuzzy_score(query, &info.object.name)
                    .max(crate::util::fuzzy_score(query, &qualified))?;
                Some((score, info.object.clone(), info.kind))
            })
            .collect();
        scored.sort_by(|a, b| {
            b.0.cmp(&a.0)
                .then_with(|| a.1.schema.cmp(&b.1.schema))
                .then_with(|| a.1.name.cmp(&b.1.name))
        });
        scored.truncate(50);
        scored.into_iter().map(|(_, object, kind)| (object, kind)).collect()
    }
```

(`Option::max` treats `None` as smallest, which is what is wanted; `?` then drops candidates matching neither.)

```rust
            Action::OpenQuickOpen => {
                let tab = self.active_tab_id();
                if self.dialog.is_none() && self.workspace(tab).is_some() {
                    self.dialog = Some(Dialog::QuickOpen(Box::new(QuickOpen {
                        tab,
                        query: String::new(),
                        selected: 0,
                    })));
                }
            }
            Action::QuickOpenMove(step) => {
                let (tab, query) = match &self.dialog {
                    Some(Dialog::QuickOpen(open)) => (open.tab, open.query.clone()),
                    _ => return,
                };
                let count = self.quick_open_matches(tab, &query).len();
                if let Some(Dialog::QuickOpen(open)) = &mut self.dialog {
                    open.selected = (open.selected as isize + step)
                        .clamp(0, count.saturating_sub(1) as isize) as usize;
                }
            }
            Action::QuickOpenPick => {
                let Some(Dialog::QuickOpen(open)) = &self.dialog else {
                    return;
                };
                let (tab, query, selected) = (open.tab, open.query.clone(), open.selected);
                let matches = self.quick_open_matches(tab, &query);
                if let Some((object, kind)) = matches.get(selected).or(matches.first()).cloned() {
                    self.dialog = None;
                    self.apply(Action::OpenObject { tab, object, kind, pin: true });
                }
            }
```

`keys.rs`: `key(Modifiers::COMMAND, Key::P, Action::OpenQuickOpen);` with the Command chords.

`src/ui/quick_open.rs`: a `Modal` (like `password_prompt.rs`) 520 wide: title "Open table or view"; a single-line field bound to `open.query` with focus requested every frame; a dim line "Searches schemas loaded in the sidebar"; up to 50 result rows as `selectable_label(index == selected, "schema.name")` (clicking one sets `selected` and pushes `QuickOpenPick`); ArrowUp/ArrowDown consumed while the modal is on top push `QuickOpenMove(∓1)`; Enter pushes `QuickOpenPick`; Escape closes. When the query changes, reset `selected` to 0. Dispatch it in `ui/mod.rs` next to the other dialogs. "No matches" when the list is empty.

- [ ] **Step 5: Run and commit**

Run: `cargo test --locked --workspace --all-targets`
Expected: PASS.

```bash
cargo fmt --all
git add src
git commit -m "Add quick open"
```

---

### Task 4: Arrow keys in the sidebar tree

**Files:**
- Modify: `src/model.rs`, `src/app.rs`, `src/ui/sidebar.rs`, `src/ui/keys.rs`, `src/ui/mod.rs`

**Interfaces:**
- Produces: `pub enum Pane { Tree, Grid }` and `Workspace.pane: Pane` (starts `Tree`); `Tree.cursor: Option<TreeNode>`; `pub enum TreeKey { Up, Down, Left, Right, Enter, Home, End }`; actions `TreeKey { tab: ConnTabId, key: TreeKey }` and `SetTreeCursor { tab: ConnTabId, node: TreeNode }`. `OpenObject` and `SelectCell` set `pane = Grid`; `SetTreeCursor` sets `pane = Tree`.

- [ ] **Step 1: Write the failing tests** (`app.rs`)

```rust
    fn tree_tab(harness: &mut Harness) -> ConnTabId {
        let tab = harness.connect_fake();
        // two schemas, the first expanded with a Tables group of two objects
        let workspace = harness.app.workspace_mut(tab).unwrap();
        workspace.tree.schemas.value = Some(vec!["main".into(), "temp".into()]);
        let node = workspace.tree.nodes.entry("main".into()).or_default();
        node.expanded = true;
        node.objects.value = Some(vec![
            ObjectInfo { object: ObjectRef::new("main", "orders"), kind: ObjectKind::Table, estimated_rows: None },
            ObjectInfo { object: ObjectRef::new("main", "users"), kind: ObjectKind::Table, estimated_rows: None },
        ]);
        tab
    }

    fn cursor(harness: &Harness, tab: ConnTabId) -> Option<TreeNode> {
        harness.app.workspace(tab).unwrap().tree.cursor.clone()
    }

    #[test]
    fn arrows_walk_the_visible_tree_and_enter_opens() {
        let mut harness = Harness::new();
        let tab = tree_tab(&mut harness);
        let key = |harness: &mut Harness, key| harness.app.apply(Action::TreeKey { tab, key });
        key(&mut harness, TreeKey::Down);
        assert_eq!(cursor(&harness, tab), Some(TreeNode::Schema("main".into())));
        key(&mut harness, TreeKey::Down); // Tables group
        key(&mut harness, TreeKey::Down); // orders
        assert_eq!(cursor(&harness, tab), Some(TreeNode::Object(ObjectRef::new("main", "orders"), ObjectKind::Table)));
        key(&mut harness, TreeKey::End);
        assert_eq!(cursor(&harness, tab), Some(TreeNode::Schema("temp".into())));
        key(&mut harness, TreeKey::Home);
        key(&mut harness, TreeKey::Down);
        key(&mut harness, TreeKey::Down);
        key(&mut harness, TreeKey::Enter);
        let workspace = harness.app.workspace(tab).unwrap();
        let object = workspace.object_tab(workspace.active_object.unwrap()).unwrap();
        assert_eq!(object.object.name, "orders");
        assert!(object.pinned);
        assert_eq!(workspace.pane, Pane::Grid);
    }

    #[test]
    fn left_and_right_fold_and_climb() {
        let mut harness = Harness::new();
        let tab = tree_tab(&mut harness);
        harness.app.apply(Action::SetTreeCursor {
            tab,
            node: TreeNode::Object(ObjectRef::new("main", "users"), ObjectKind::Table),
        });
        harness.app.apply(Action::TreeKey { tab, key: TreeKey::Left }); // to the group
        harness.app.apply(Action::TreeKey { tab, key: TreeKey::Left }); // fold the group
        harness.app.apply(Action::TreeKey { tab, key: TreeKey::Left }); // to the schema
        assert_eq!(cursor(&harness, tab), Some(TreeNode::Schema("main".into())));
        harness.app.apply(Action::TreeKey { tab, key: TreeKey::Left }); // fold the schema
        assert!(!harness.app.workspace(tab).unwrap().tree.nodes["main"].expanded);
        harness.app.apply(Action::TreeKey { tab, key: TreeKey::Right }); // unfold
        assert!(harness.app.workspace(tab).unwrap().tree.nodes["main"].expanded);
    }

    #[test]
    fn a_cursor_on_a_hidden_row_restarts_at_the_top() {
        let mut harness = Harness::new();
        let tab = tree_tab(&mut harness);
        harness.app.apply(Action::SetTreeCursor {
            tab,
            node: TreeNode::Object(ObjectRef::new("main", "orders"), ObjectKind::Table),
        });
        harness.app.workspace_mut(tab).unwrap().tree.filter = "users".into();
        harness.app.apply(Action::TreeKey { tab, key: TreeKey::Down });
        let cursor = cursor(&harness, tab).unwrap();
        let rows = harness.app.workspace(tab).unwrap().tree.visible_rows(Driver::Sqlite, false);
        assert!(rows.iter().any(|row| row.node == cursor), "{cursor:?} is visible");
    }
```

`ui/mod.rs` tests:

```rust
    // tree_harness() comes from Task 3's tests.

    #[test]
    fn clicking_the_tree_sends_arrows_there_and_the_grid_takes_them_back() {
        let (mut harness, tab) = tree_harness();
        harness.click("orders");
        harness.answer_rows(crate::testing::page(3, false));
        assert_eq!(harness.app.workspace(tab).unwrap().pane, crate::model::Pane::Tree);
        harness.press(Key::ArrowDown, Modifiers::NONE);
        let workspace = harness.app.workspace(tab).unwrap();
        assert_eq!(
            workspace.tree.cursor,
            Some(crate::model::TreeNode::Object(
                tabletist_db::ObjectRef::new("main", "users"),
                tabletist_db::ObjectKind::Table
            ))
        );
        let object_tab = workspace.active_object.unwrap();
        assert_eq!(workspace.object_tab(object_tab).unwrap().selection, None, "the grid did not move");
        harness.app.apply(Action::SelectCell {
            tab,
            object_tab,
            cell: crate::model::CellPos { row: 0, col: 0 },
        });
        harness.press(Key::ArrowDown, Modifiers::NONE);
        let workspace = harness.app.workspace(tab).unwrap();
        assert_eq!(workspace.pane, crate::model::Pane::Grid);
        assert_eq!(
            workspace.object_tab(object_tab).unwrap().selection,
            Some(crate::model::CellPos { row: 1, col: 0 })
        );
    }

    #[test]
    fn arrows_in_the_filter_bar_do_not_move_the_tree() {
        let (mut harness, tab) = tree_harness();
        harness.click("orders");
        harness.answer_rows(crate::testing::page(3, false));
        let before = harness.app.workspace(tab).unwrap().tree.cursor.clone();
        // Cmd/Ctrl+F focuses the filter bar's value field.
        harness.press(Key::F, Modifiers::COMMAND);
        harness.press(Key::ArrowDown, Modifiers::NONE);
        assert_eq!(harness.app.workspace(tab).unwrap().tree.cursor, before);
    }
```

(`CellPos` field names and `ObjectInfo` fields are whatever `model.rs` and `catalog.rs` define; adjust the literals, not the assertions.)

- [ ] **Step 2: Run them to watch them fail**

Run: `cargo test --locked --lib -- tree_ arrows left_and_right cursor`
Expected: compile errors for the new names.

- [ ] **Step 3: Implement**

`model.rs`: `Pane`, `TreeKey`, the fields, the actions; `SchemaNode` derives `Default` if it does not.

`app.rs`:

```rust
            Action::SetTreeCursor { tab, node } => {
                if let Some(workspace) = self.workspace_mut(tab) {
                    workspace.tree.cursor = Some(node);
                    workspace.pane = Pane::Tree;
                }
            }
            Action::TreeKey { tab, key } => self.tree_key(tab, key),
```

and in `OpenObject` and `SelectCell`: set the workspace's `pane = Pane::Grid`.

```rust
    fn tree_key(&mut self, tab: ConnTabId, key: TreeKey) {
        let show_system = self.settings.show_system_schemas;
        let Some(workspace) = self.workspace(tab) else {
            return;
        };
        let rows = workspace.tree.visible_rows(workspace.driver, show_system);
        if rows.is_empty() {
            return;
        }
        let at = workspace
            .tree
            .cursor
            .as_ref()
            .and_then(|cursor| rows.iter().position(|row| &row.node == cursor));
        let last = rows.len() - 1;
        let target = match (key, at) {
            // No cursor, or it is on a row no longer shown: start at the top.
            (_, None) => Some(0),
            (TreeKey::Up, Some(at)) => Some(at.saturating_sub(1)),
            (TreeKey::Down, Some(at)) => Some((at + 1).min(last)),
            (TreeKey::Home, _) => Some(0),
            (TreeKey::End, _) => Some(last),
            (TreeKey::Right, Some(at)) => match rows[at].expanded {
                Some(false) => {
                    self.toggle_tree_row(tab, &rows[at].node);
                    Some(at)
                }
                Some(true) => Some((at + 1).min(last)),
                None => Some(at),
            },
            (TreeKey::Left, Some(at)) => match rows[at].expanded {
                Some(true) => {
                    self.toggle_tree_row(tab, &rows[at].node);
                    Some(at)
                }
                // Up to the parent: the nearest row above that is shallower.
                _ => Some(
                    (0..at)
                        .rev()
                        .find(|&i| rows[i].depth < rows[at].depth)
                        .unwrap_or(at),
                ),
            },
            (TreeKey::Enter, Some(at)) => {
                match &rows[at].node {
                    TreeNode::Object(object, kind) => self.apply(Action::OpenObject {
                        tab,
                        object: object.clone(),
                        kind: *kind,
                        pin: true,
                    }),
                    node => self.toggle_tree_row(tab, node),
                }
                None
            }
        };
        if let Some(index) = target
            && let Some(workspace) = self.workspace_mut(tab)
        {
            workspace.tree.cursor = Some(rows[index].node.clone());
            workspace.pane = Pane::Tree;
        }
    }

    /// Folds or unfolds a schema or group row.
    fn toggle_tree_row(&mut self, tab: ConnTabId, node: &TreeNode) {
        match node {
            TreeNode::Schema(schema) => self.apply(Action::ToggleSchema { tab, schema: schema.clone() }),
            TreeNode::Group(schema, kind) => self.apply(Action::ToggleGroup {
                tab,
                schema: schema.clone(),
                kind: *kind,
            }),
            TreeNode::Object(..) => {}
        }
    }
```

(In `(_, None)` with `TreeKey::Enter`, the first row just gets the cursor; that is fine.)

`sidebar.rs`: pass the workspace's `tree.cursor` into `tree_row`; draw the cursor row with a 1 px `palette.accent` outline (`painter().rect_stroke`) when `pane == Tree`; every row click also pushes `Action::SetTreeCursor { tab, node: row.node.clone() }` after its existing action (so a single-click preview keeps arrows in the tree).

`keys.rs`: when `!editing` and the active tab's workspace has `pane == Pane::Tree` (or no grid is showing), map ArrowUp/Down/Left/Right, Home, End and Enter to `Action::TreeKey`; otherwise keep today's grid mapping. Space keeps toggling the row panel only in the grid pane.

- [ ] **Step 4: Run and commit**

Run: `cargo test --locked --workspace --all-targets`
Expected: PASS, including every existing grid-key test (they open objects with `OpenObject`, which sets `Pane::Grid`).

```bash
cargo fmt --all
git add src
git commit -m "Navigate the sidebar tree with the arrow keys"
```

---

### Task 5: The shortcut map and the help dialog

**Files:**
- Create: `src/ui/help.rs`
- Modify: `src/model.rs`, `src/app.rs`, `src/ui/keys.rs`, `src/ui/mod.rs`

**Interfaces:**
- Produces: `pub const SHORTCUTS: &[(&str, &str)]` in `keys.rs` (keys with `Mod` for Cmd/Ctrl, and the action's description); `pub fn keys_label(keys: &str) -> String` (`Mod` becomes `Cmd` on macOS and `Ctrl` elsewhere); `Dialog::Help`; `Action::ShowHelp`.

- [ ] **Step 1: Write the failing tests**

`keys.rs` tests (new `#[cfg(test)] mod tests`):

```rust
    #[test]
    fn the_shortcut_table_covers_the_spec_map() {
        let descriptions: Vec<&str> = SHORTCUTS.iter().map(|(_, what)| *what).collect();
        for expected in [
            "New connection tab",
            "Close connection tab",
            "Switch connection tab",
            "New connection",
            "Close object tab",
            "Previous / next object tab",
            "Refresh",
            "Filter bar",
            "Quick open",
            "Previous / next page",
            "Cancel running query",
            "Toggle row panel",
            "Copy cell / copy row",
            "Move in the tree or grid",
            "Shortcuts",
        ] {
            assert!(descriptions.contains(&expected), "{expected}");
        }
    }

    #[test]
    fn mod_is_named_for_the_platform() {
        let label = keys_label("Mod+Shift+W");
        if cfg!(target_os = "macos") {
            assert_eq!(label, "Cmd+Shift+W");
        } else {
            assert_eq!(label, "Ctrl+Shift+W");
        }
    }
```

`ui/mod.rs` tests:

```rust
    #[test]
    fn question_mark_opens_the_shortcuts_and_escape_closes_them() {
        let mut harness = Harness::new();
        harness.frame(vec![egui::Event::Text("?".into())]);
        assert!(harness.has("Keyboard shortcuts"));
        assert!(harness.has("Quick open"));
        harness.press(Key::Escape, Modifiers::NONE);
        assert!(harness.app.dialog.is_none());
    }

    #[test]
    fn typing_a_question_mark_in_a_field_does_not_open_help() {
        let mut harness = Harness::new();
        let tab = harness.connect_fake();
        harness.app.apply(Action::OpenObject {
            tab,
            object: tabletist_db::ObjectRef::new("main", "users"),
            kind: tabletist_db::ObjectKind::Table,
            pin: true,
        });
        harness.answer_rows(crate::testing::page(3, false));
        // Cmd/Ctrl+F focuses the filter bar's value field.
        harness.press(Key::F, Modifiers::COMMAND);
        harness.frame(vec![egui::Event::Text("?".into())]);
        assert!(harness.app.dialog.is_none());
        let workspace = harness.app.workspace(tab).unwrap();
        let object = workspace.object_tab(workspace.active_object.unwrap()).unwrap();
        assert_eq!(object.filter.rows[0].value, "?");
    }
```

- [ ] **Step 2: Run them to watch them fail**

Run: `cargo test --locked --lib -- shortcut mod_is_named question_mark`
Expected: compile errors for the new names.

- [ ] **Step 3: Implement**

`keys.rs`:

```rust
/// Every shortcut, for the help dialog. `Mod` is Cmd on macOS, Ctrl elsewhere.
pub const SHORTCUTS: &[(&str, &str)] = &[
    ("Mod+T", "New connection tab"),
    ("Mod+Shift+W", "Close connection tab"),
    ("Mod+1…9, Ctrl+Tab, Ctrl+Shift+Tab", "Switch connection tab"),
    ("Mod+N", "New connection"),
    ("Mod+W", "Close object tab"),
    ("Mod+Shift+[ / ]", "Previous / next object tab"),
    ("Mod+R", "Refresh"),
    ("Mod+F", "Filter bar"),
    ("Mod+P", "Quick open"),
    ("Mod+Alt+Left / Right", "Previous / next page"),
    ("Mod+.", "Cancel running query"),
    ("Space, Mod+Shift+R", "Toggle row panel"),
    ("Mod+C, Mod+Shift+C", "Copy cell / copy row"),
    ("Arrows, Page Up/Down, Home/End, Enter", "Move in the tree or grid"),
    ("?", "Shortcuts"),
];

/// `keys` with `Mod` named for this platform.
pub fn keys_label(keys: &str) -> String {
    let command = if cfg!(target_os = "macos") { "Cmd" } else { "Ctrl" };
    keys.replace("Mod", command)
}
```

In `handle`, after the existing chords and only when `!editing` and no dialog is open: if the frame's events contain `egui::Event::Text(text)` with `text == "?"`, push `Action::ShowHelp`. (Matching the typed text rather than a key works on every keyboard layout.)

`model.rs`: `Dialog::Help`, `Action::ShowHelp`. `app.rs`: `Action::ShowHelp => if self.dialog.is_none() { self.dialog = Some(Dialog::Help) }`; `CloseDialog` already clears any dialog.

`src/ui/help.rs`: a `Modal` 460 wide titled "Keyboard shortcuts"; a two-column `egui::Grid` of `keys_label(keys)` (in `theme::mono(12.5)`, `palette.secondary`) and `gettext(locale, what)`; a Close button; Escape pushes `Action::CloseDialog`. Dispatch it in `ui/mod.rs` with the other dialogs.

- [ ] **Step 4: Run and commit**

Run: `cargo test --locked --workspace --all-targets` and `cargo clippy --locked --workspace --all-targets -- -D warnings`
Expected: PASS, no warnings.

```bash
cargo fmt --all
git add src
git commit -m "Add the shortcuts dialog and the full shortcut map"
```

- [ ] **Step 5: By hand (when a display is available)**

`cargo run -- --demo`: Cmd/Ctrl+P opens `users`; Cmd/Ctrl+F filters `age > 30`, Enter applies, the footer says Filtered; Count shows the exact total; arrows walk the tree after clicking it; `?` lists the shortcuts. Ledger it as not done if there is no display.

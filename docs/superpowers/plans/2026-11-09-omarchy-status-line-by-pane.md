# Omarchy status line by pane: implementation plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use rubypowers:subagent-driven-development (recommended) or rubypowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** The Omarchy status line shows the keys of the pane that has the
keyboard (tree, its filter field, grid, row panel, SQL script, SQL
results), with `ctrl+h/l panes` wherever those keys move.

**Spec:** `docs/superpowers/specs/2026-11-09-omarchy-status-line-by-pane-design.md`

**Architecture:** A new module `src/ui/pane_hints.rs` says which pane has
the keyboard (`Held::of`) and builds the tree's, filter's and panel's
sets as owned `(Written, String)` pairs. `workspace::status_line` asks it
first and draws its set in place of the view's; the grid, Structure, DDL,
script and results sets stay in `status_line` and gain the pane hint.
`focus::can_step` says whether ctrl+h/l would move from a pane.

**Tech stack:** Rust, egui, `crate::testing::Harness`. Cargo is
`~/.cargo/bin/cargo`.

---

## File map

- Create `src/ui/pane_hints.rs`: `Held`, `Held::of`, `tree`, `filter`,
  `panel`, `panes_key`.
- Create `src/ui/pane_hints_tests.rs`: the harness tests.
- Modify `src/ui/mod.rs`: `mod pane_hints;` and the `#[cfg(test)] mod
  pane_hints_tests;`.
- Modify `src/ui/focus.rs`: `pub fn can_step`.
- Modify `src/ui/sidebar.rs`: remember the filter field's id;
  `pub fn filter_focused(ctx, tab) -> bool`.
- Modify `crates/tabletist-core/src/keymap.rs`: `pub fn own_key_in`, the
  first own (not `via`) key of a command in a scope, for `esc grid`.
- Modify `src/ui/workspace.rs` (`status_line`): use the module.
- Modify `src/ui/start.rs` (`status_line`): the tree's set while the tree
  has the keys.

## Task 1: `focus::can_step`

**Files:** `src/ui/focus.rs`, test in the `#[cfg(test)]` block of
`src/ui/pane_hints_tests.rs` (Task 6 runs it through the harness).

- [ ] Add, after `neighbour`:

```rust
/// Whether ctrl+h or ctrl+l would move the keyboard from `from`: another
/// region of `among` was drawn the frame before, the regions `step`
/// steps among.
pub fn can_step(ctx: &egui::Context, among: &[Region], from: Region) -> bool {
    ctx.data_mut(|data| {
        let regions: &mut Regions = data.get_temp_mut_or_default(regions_id());
        regions
            .before
            .iter()
            .any(|(region, _)| *region != from && among.contains(region))
    })
}
```

- [ ] `~/.cargo/bin/cargo build` compiles. Commit: "Say whether the pane keys
  would move the keyboard from a pane".

## Task 2: the filter field's focus, and the own key

- [ ] In `src/ui/sidebar.rs`, beside the terminal filter field (line ~314),
  remember its id:

```rust
ui.data_mut(|data| data.insert_temp(filter_field_key(tab), field.id));
```

and add:

```rust
fn filter_field_key(tab: ConnTabId) -> egui::Id {
    egui::Id::new(("tree-filter-field", tab.0))
}

/// Whether the terminal look's filter field above the tree has the
/// keyboard. Its id is the one the last frame drew it with.
pub fn filter_focused(ctx: &egui::Context, tab: ConnTabId) -> bool {
    ctx.data(|data| data.get_temp::<egui::Id>(filter_field_key(tab)))
        .is_some_and(|id| ctx.memory(|memory| memory.has_focus(id)))
}
```

- [ ] In `keymap.rs`, beside `key_in`:

```rust
/// The first key of `command`'s own in `scope`: not one that only says
/// another command's key (`via`). Empty where it has none.
pub fn own_key_in(&self, layout: Layout, command: Command, scope: Scope) -> Written {
    self.chords(layout, command, scope)
        .next()
        .map_or_else(Written::default, |keys| Written(written(layout, keys)))
}
```

with a keymap unit test that `own_key_in(Omarchy, BackToGrid,
Scope::Inspector) == "esc"` while `key_in` says `ctrl+h`.

- [ ] Build, run the keymap test, commit: "Remember the tree filter's field,
  and write a command's own key".

## Task 3: `pane_hints` module

**Files:** create `src/ui/pane_hints.rs`; `mod pane_hints;` in
`src/ui/mod.rs`.

```rust
//! The terminal look's status line names the keys of the pane that has
//! the keyboard: the tree's, its filter field's, the row panel's. The
//! grid's, a SQL editor's and the views' sets are drawn by the line
//! itself, with the pane keys added.

use tabletist_core::keymap::{Command, Keymap, Layout, Scope, Written};

use crate::model::{ConnTabId, Pane, Workspace};
use crate::ui::focus::{self, Region};

/// The pane that has the keyboard, as the status line reads it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Held {
    /// The tree's filter field.
    Filter,
    /// A field of the row panel of the table on screen.
    Panel,
    /// The tree.
    Tree,
    /// Anything else: the view in front says.
    View,
}

impl Held {
    pub fn of(ctx: &egui::Context, tab: ConnTabId, workspace: &Workspace) -> Self {
        if crate::ui::sidebar::filter_focused(ctx, tab) {
            return Self::Filter;
        }
        let panel = workspace.active_object_tab().and_then(|object| {
            let id = workspace.active_tab?;
            let columns = object.rows.value.as_ref()?.columns.len();
            crate::ui::row_panel::focused_field(ctx, tab, id, columns)
        });
        if panel.is_some() {
            return Self::Panel;
        }
        if workspace.pane == Pane::Tree && !ctx.text_edit_focused() {
            return Self::Tree;
        }
        Self::View
    }
}

/// `ctrl+h/l panes`, where those keys would move the keyboard from `from`.
pub fn panes_key(
    ctx: &egui::Context, keymap: &Keymap, layout: Layout, from: Region,
) -> Option<(Written, String)> { /* together(PaneLeft, PaneRight),
    "panes", if focus::can_step(ctx, &Region::PANES, from) */ }

/// j/k move · enter open · / filter · t tree · za fold · ctrl+h/l panes ·
/// E export table. Export gives way first, then fold.
pub fn tree(...) -> Vec<(Written, String)>;
/// enter/esc tree.
pub fn filter(...) -> Vec<(Written, String)>;
/// j/k field · [ ] row · i edit (where the row can be edited) · gy copy
/// as · esc grid · ctrl+h/l panes.
pub fn panel(...) -> Vec<(Written, String)>;
```

The words go through `gettext(locale, …)`. Keys:
- tree: `key_in(MoveInTree, Sidebar)`, `key(OpenObject)`,
  `key(FilterObjects)`, `key(ToggleTree)`, `key(FoldGroup)`,
  `panes_key(Tree)`, `key_in(Export, Sidebar)`.
- filter: the field's Enter and Esc are egui's, no keymap command's. Add
  `pub fn Written::plain(text: &str) -> Written` in `keymap.rs` (keys a
  widget takes itself) and use `(Written::plain("enter/esc"), "tree")`.
- panel: `key_in(MoveField, Inspector)`, `together([PreviousRow,
  NextRow])`, `key_in(EditCell, Inspector)`, `key(CopyRowAs)`,
  `own_key_in(BackToGrid, Inspector)`, `panes_key(Panel)`.
  "Can be edited": `crate::edit::Table::of(workspace, object).is_some()`
  and the connection is not read-only, as `editing_status` reads it.

- [ ] Build. Commit: "Build the tree's, the filter's and the row panel's key
  hints".

## Task 4: `status_line` uses them

**File:** `src/ui/workspace.rs`, `status_line` (~2759–3500).

- [ ] Before the `egui::Panel::bottom`, compute

```rust
let held = super::pane_hints::Held::of(ui.ctx(), tab, workspace);
let own: Option<Vec<(Written, String)>> = match held {
    Held::Filter => Some(pane_hints::filter(keymap, layout, locale)),
    Held::Panel => Some(pane_hints::panel(ui.ctx(), keymap, layout, locale, editable)),
    Held::Tree => Some(pane_hints::tree(ui.ctx(), keymap, layout, locale)),
    Held::View => None,
};
let own_hints: Option<Vec<widgets::Hint<'_>>> =
    own.as_ref().map(|own| own.iter().map(|(key, word)| (key, word.as_str(), true)).collect());
```

- [ ] The History branch and the Agents branch draw `own_hints` in place of
  theirs where it is `Some`. Their right ends are unchanged.
- [ ] The final pick becomes
  `own_hints.as_deref().unwrap_or(if on_editor { … } else if …)`.
  With `own_hints` `Some` on a SQL tab (the tree), the editor branch's vim
  mode is not drawn: take the generic path (`on_editor && own.is_none()`).
- [ ] Grid set: after `(&sidebar_key, "tables", true)` push the pane hint
  (`panes_key(Grid)`). Structure: after its `tables`. DDL: after
  `structure`, before `normal`.
- [ ] Editor sets: split. Script (the editor has focus, or no results
  grid): today's, with `(FocusResults key, "results")` after `leave editor`
  where the results show a grid. Results (`result_keys` true):
  `MoveRow · MoveColumn · OpenInspector · CopyCells · CopyAs ·
  (PaneUp, "editor") · panes_key(Grid) · (Export, "export result")`.
- [ ] Build, run `~/.cargo/bin/cargo test --lib`, fix what an old test says
  of the line (tests that read the editor's or the grid's set). Commit:
  "Name the keys of the pane that has the keyboard on the status line".

## Task 5: start screen

- [ ] `start::status_line` takes `own: Option<&[widgets::Hint<'_>]>` and
  draws it in place of its own set; `workspace::status_line` passes the
  tree's set when `Held::of` is `Tree` (or `Filter`'s for the filter).
- [ ] Build, test, commit: "Name the tree's keys on the start screen's line
  too".

## Task 6: tests

**File:** `src/ui/pane_hints_tests.rs`, `#[cfg(test)] mod
pane_hints_tests;` in `src/ui/mod.rs`.

Helper: the texts painted on the status line, left to right:

```rust
fn line(harness: &Harness) -> Vec<String> {
    let top = harness.size.y - crate::ui::workspace::STATUS_LINE;
    let mut on: Vec<_> = harness.text_rects.iter()
        .filter(|(_, rect)| rect.top() >= top)
        .collect();
    on.sort_by(|a, b| a.1.left().total_cmp(&b.1.left()));
    on.into_iter().map(|(text, _)| text.clone()).collect()
}
```

Tests (fixtures: `insert_audit_tests::covers_in(Look::omarchy())`, the
`ran` helper of `pane_keys_tests.rs` moved to `pub(super)`):

- [ ] `slash_then_esc_from_the_tree_names_the_trees_keys`: covers, ctrl+h
  to the tree, type `/`: line == `["enter/esc tree", …summary]`; Esc:
  line starts `j/k move, enter open, / filter, t tree, za fold, ctrl+h/l
  panes, E export table`.
- [ ] `ctrl_l_from_the_tree_names_the_grids_keys`: then ctrl+l: line has
  `j/k row` and `ctrl+h/l panes` right after `ctrl+b tables`, no `j/k
  move`.
- [ ] `ctrl_l_from_a_row_names_the_panels_keys`: select a row, ctrl+l:
  `j/k field, [ ] row, i edit, gy copy as, esc grid, ctrl+h/l panes`;
  ctrl+h: the grid's again.
- [ ] `a_read_only_rows_panel_offers_no_edit`: `covers_as(…, true)`.
- [ ] `a_sql_editor_names_the_scripts_keys_then_the_results`: `ran`, the
  script's set with `ctrl+j results`; ctrl+j: `j/k row … ctrl+k editor,
  ctrl+h/l panes, E export result`.
- [ ] `with_agents_in_front_the_tree_offers_no_pane_keys`.
- [ ] `a_narrow_window_drops_export_first`: `Harness::with_size(640×…)`, the
  tree's set loses `E export table` before `za fold`.
- [ ] `the_other_looks_keep_their_line`: macOS, the tree focused: no `j/k
  move` painted on the line.
- [ ] Run each new test, see it pass. Commit: "Test the status line's keys
  by pane".

## Task 7: finish

- [ ] `~/.cargo/bin/cargo fmt`, `~/.cargo/bin/cargo clippy --all-targets --
  -D warnings`, `~/.cargo/bin/cargo test --lib`.
- [ ] Update `docs/_guide/omarchy.md` where it describes the status line.
- [ ] Push the branch, open one PR against main, watch CI.

# Omarchy status line by pane

The Omarchy look's bottom status line shows the keys of the pane that has
the keyboard, not those of the view in front. Today it picks its hints by
view alone, and never names ctrl+h/l: a user who pressed `/` in the tree,
then Esc, was left in the sidebar with the table's keys on the line and no
hint of the way back.

The user's words: "I think that footer should be a dinamical dependeing on
the pane selected."

## Scope

- The Omarchy status line in `src/ui/workspace.rs` (`status_line`), and the
  start screen's line in `src/ui/start.rs` for the tree's set.
- Not the classic look, not the sidebar's own footer
  (`src/ui/sidebar.rs`, `HINTS`), not the row panel's header hints.

## Which pane has the keyboard

A function `status_pane` in `src/ui/workspace.rs` answers, in this order,
the way `src/ui/keys.rs` reads the same state:

1. **Filter**: the tree's filter field has egui's focus.
2. **Panel**: a field of the row panel of the table on screen has focus
   (`row_panel::focused_field`).
3. **Script**: a SQL editor's script has focus (`sql_text::editor_id`).
4. **Tree**: `workspace.pane == Pane::Tree`, and no text field has focus.
5. **View**: anything else: the view in front picks the set, as today.

In a SQL tab, View with the results shown is **Results**.

## The sets

Keys are written from the keymap, as today; the words below are the
Omarchy defaults.

| Pane | Set |
|---|---|
| Tree | `j/k move · enter open · / filter · t tree · za fold · ctrl+h/l panes · E export table` |
| Filter | `enter/esc tree` |
| Grid | today's set, with `ctrl+h/l panes` right after `ctrl+b tables` |
| Panel | `j/k field · [ ] row · i edit · gy copy as · esc grid · ctrl+h/l panes` (`i edit` only where the row can be edited) |
| Script | `run · run all · cancel · leave editor · ctrl+j results · tables` (`complete` leads while the list is open; the vim mode starts the line) |
| Results | `j/k row · h/l col · enter inspect · y copy · gy copy as · ctrl+k editor · ctrl+h/l panes · E export result` |
| Structure | today's set, with `ctrl+h/l panes` after `tables` |
| DDL | today's set, with `ctrl+h/l panes` after `s structure` (it has no `tables`) |
| Agents, History | unchanged |

- The tree's set shows whatever view is in front, the start screen too.
- Where room runs out, hints give way from the end: export first, then fold
  (tree), as today's rule.
- The line's right end (the summary, what was just said) is unchanged.

## When `ctrl+h/l panes` shows

Only where ctrl+h or ctrl+l would move: `focus::can_step(ctx, among, from)`
says whether a region of `among` other than `from` was drawn the frame
before, from the same list `focus::step` reads. The Agents and History tabs
draw no pane, so with one in front the tree's set has no pane hint.

## Testing

Harness tests (`crate::testing::Harness`, `Look::omarchy()`) in
`src/ui/pane_hints_tests.rs`, reading `harness.painted`:

- `/` then Esc from the tree: the tree's set, with `ctrl+h/l panes`.
- Typing in the filter field: `enter/esc tree`.
- ctrl+l from the tree: the grid's set.
- ctrl+l from a selected row: the panel's set; ctrl+h back: the grid's.
- A SQL editor: the script's set, then ctrl+j: the results' set.
- Agents in front, the tree focused: no `ctrl+h/l panes`.
- A narrow window: export gives way first.

No design-conformance or pixel tests.

## Departures from the design canvas

Approved by the user, each explicitly:

1. `ctrl+h/l panes` in hint lines. The canvas has ctrl+h/l only in a note
   on OmarchyStates' "6 · keyboard focus" board and the inspector's
   `ctrl+l focus`.
2. The status line changes with the pane that has the keyboard.
3. The Tree, Filter, Panel and Results sets, which the canvas never draws.

## Decisions

- **Pane keys: one shared hint, `ctrl+h/l panes`, in every pane's set.**
  The user picked it over a hint named for each direction.
- **Sidebar footer: kept as it is.** The canvas draws it, and it says what
  the sidebar does while the grid has the keys.
- **Classic look: unchanged.** Its status line has no key hints of this kind.
- **Tree's set: `j/k move · enter open · / filter · t tree · za fold ·
  ctrl+h/l panes · E export table`.** Export gives way first, then fold.
- **Grid: `ctrl+h/l panes` after `ctrl+b tables`.** Export still the first
  to give way; the pane hint goes late.
- **Row panel: its field keys.** `j/k field · [ ] row · i edit · gy copy as
  · esc grid · ctrl+h/l panes`; its header stays as the canvas draws it.
- **SQL editor: split by pane.** Script and results each get their own set,
  with ctrl+j / ctrl+k between them.
- **Filter field: `enter/esc tree`.** Both leave the field for the tree;
  the tree's letters do nothing while typing.
- **The pane hint only where it moves.** Agents and History draw no pane.
- **Design and all three canvas departures: "Yes to all".**

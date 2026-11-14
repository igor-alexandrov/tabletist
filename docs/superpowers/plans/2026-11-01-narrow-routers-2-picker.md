# Narrowing what the routers see of `App`, run 2: the picker's chrome

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A picker tab's header (title, count, New connection) and body (search, footer, list) take `ui::Paint` and what they show instead of `&mut App`. `picker::show` stays as the picker tab's router: it draws the backdrop and the header's frame, the notices, then the header and the body, builds the body's answers and extends `app.actions` after each part. This follows the routers' plan (`2026-11-01-narrow-routers.md`, run 2). Behaviour does not change.

**Files:**
- Modify: `src/ui/picker.rs` (`show` split into `frame`, `header` and `body`; `Asked` and its builder `asked`)
- Modify: `docs/superpowers/plans/2026-11-01-narrow-routers.md` (run 2, and a point of the pattern)

## Survey (2026-11-01, `main` at `55e2ad41`)

`picker::show` was one function of 420 lines. In the order it drew:

| Part | What it read of `App` | Now |
|---|---|---|
| backdrop; the header's fill, rule and drag area | Paint | `frame(ui, &paint, full) -> Rect`, the header's rect |
| the notices | `notice`, `agents_held`, `connections` (run 1) | unchanged, called by the router |
| title, count, New connection | Paint, `titlebar.inset`, `connections` (its length), `keymap` with `layout()` | `header(ui, paint, header, inset, total, &Keymap) -> (Line, Vec<Action>)` |
| search, footer, list | Paint, `tabs[active]` (the search edited, `focus_search` taken), `connections`, `keymap` with `layout()`, `active_tab_id`, and for each row drawn `tab_showing` and `agents_access` | `body(ui, paint, Room, &mut PickerState, &SavedConnections, &Keymap, &Asked) -> Vec<Action>` |

`Line` is where the header's contents sit (their line, where the count ends, where New connection begins), which the macOS search shares. `Room` is the picker's rect, how far down the header and the notices reach, and that `Line`. `Asked` holds the picker's tab and each saved connection's `Opening`. All three, `Opening`, and the builder stay private to `picker.rs`, since the router is there.

Where the survey differs from the plan:

- **Only the header's frame is drawn before the notices.** The plan had the whole header before them. New connection is focusable, and egui moves Tab through focusable widgets in the order they are made: drawing it before the notices would put it before their buttons, where it came after. So the router draws the backdrop and the header's frame (its fill, its rule and the drag area, which senses `CLICK | DRAG` without `FOCUSABLE` and so has no focus and no AccessKit node), then the notices, then the header's contents. The plan says so now, and its pattern gains a point on it.
- **The `Workspace(_)` arm is gone.** The body defaulted its search, selection, folds and question when the active tab was a workspace. `ui::show` calls `picker::show` only for a picker tab, so the router now returns before the body where the tab is not one. No frame draws differently.

## What this run teaches

- **A router split into parts keeps every focusable widget in its place among the others.** What only paints, and a widget egui gives no focus or AccessKit node, can move; a button cannot. So a part may be drawn in two steps around shared chrome, as the picker's header is around the notices.
- **The answers a part needs about every row are built for every saved connection** before the tab is lent `&mut`, in a map keyed by the connection. `tab_showing` and `agents_access` only read `App`, so asking them of rows the list does not draw changes nothing. `Asked::opening` falls back to the picker's tab, which no saved connection reaches.
- **Each part returns its actions and the router extends `app.actions` after it,** in the order they reached it before: the notices', the header's (New connection), then the body's (the footer's question, the group headings, the rows).
- **A view's keys come from `&Keymap` and `Paint::layout()`,** as quick open's do. The header and the body each write the keys they show; New connection's is written in both, as before it was written once and shown in both.

## Tasks

### Task 1: the router and its parts

- [x] `frame` draws the backdrop and the header's fill, rule and drag area, and returns the header's rect.
- [x] `header` draws the title, count and New connection from `Paint`, the inset, the count and `&Keymap`, and returns its `Line` and actions.
- [x] `body` draws the search, footer and list from `Paint`, its `Room`, `&mut PickerState`, `&SavedConnections`, `&Keymap` and `&Asked`, and returns its actions. It borrows the picker's search, selection, folds and question after the search field edits them, where it cloned them.
- [x] `asked` builds each saved connection's `Opening` before the router lends the tab.
- [x] `show` draws `frame`, the notices, `header` and `body` in that order, extending `app.actions` after each.

### Task 2: CI's checks and the PR

- [x] `cargo fmt --all --check`
- [x] `cargo clippy --locked --workspace --all-targets -- -D warnings`
- [x] `cargo test --locked --workspace --all-targets`
- [ ] Push and open the PR.

The net is the existing tests, which drive the whole `App`: the picker's in `src/ui/mod.rs`, `src/ui/env_tests.rs`, `src/ui/agents_tests.rs` and `src/app.rs` (search, selection, folding, the delete question, New connection, a row's buttons, menus and Agents part), the picker's shots in `src/shots.rs`, and every test that starts on the picker. No test changed: no signature a test calls moved.

## For run 3

`workspace::show` draws the connection bar before the notices, and a workspace's connecting screens (`opening`, `connecting`, `failure`, `banner`) after them. Run 3 narrows those screens where they are drawn, so their place in the focus order holds; if it moves any of them, it checks that place against the notices' buttons first.

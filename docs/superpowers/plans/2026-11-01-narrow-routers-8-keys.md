# Narrowing what the routers see of `App`, run 8: the keys' helpers

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** The helpers of the keys' router that read one workspace take `&Workspace`, or the table tab's `&ObjectTab`, instead of `&App` and a tab's id. The ones that read the keymap take `&Keymap` and a `Layout`, and the locale where they say something. `handle`, `letters`, `editing_keys` and `text_size` keep `&App`/`&mut App`, and find what they lend. This is the last run of the routers' plan (`2026-11-01-narrow-routers.md`, run 8), and closes it. Behaviour does not change.

**Files:**
- Modify: `src/ui/keys.rs` (the helpers below, and their calls in `handle`, `editing_keys` and `letters`)
- Modify: `docs/superpowers/plans/2026-11-01-narrow-routers.md` (run 8, and the plan's closing)

## Survey (2026-11-02, `main` at `19e1e7c5`)

| Function | What it read of `App` | Now takes |
|---|---|---|
| `switchable` | `workspace(tab)`'s databases | `&Workspace` |
| `selection_row` | `workspace(tab)`'s object tab `id` | `&ObjectTab` |
| `on_last_row` | `workspace(tab)`'s object tab `id` | `&ObjectTab` |
| `history_shown` | `workspace(tab)`, while its History tab is in front | `&Workspace` |
| `agents_log` | `workspace(tab)`'s Agents tab, through `agents_view` | `&Workspace` |
| `agents_view` | `workspace(tab)`, while its Agents tab is in front | `&Workspace` |
| `delete_row` | `workspace(tab)`'s object tab `id`; `look` and `locale` for what a refusal says | `&Workspace, &Look, Locale, tab, id, actions` |
| `delete_selected`, `delete_count` | the same, and `delete_row` | `&Workspace, &Look, Locale, tab, id, [count,] actions` |
| `history_chords` | `keymap`, `layout()`; `history_command` | `&Keymap, Layout, Locale, ctx, tab, history, actions` |
| `history_command` | `locale` for the toast Copy SQL says | `ctx, Locale, tab, command, chosen, actions` |
| `window_letter` | `keymap`, `layout()` | `&Keymap, Layout, scopes, char` |
| `agents_shows`, `agents_lists`, `agents_here` | `workspace(tab)` through `agents_view`; `App::agents_here(tab)`; `agents.listed` | unchanged: `&App, tab` |
| `handle`, `letters`, `editing_keys`, `text_size` | the dialog, the keymap, the active tab, its view, what is busy and selected | unchanged, by the plan's design |

Where the survey differs from the plan:

- **`selection_row` and `on_last_row` take the table tab, `&ObjectTab`,** the tab's slice the plan allows: neither reads anything of the workspace but the tab. `handle` finds the tab once in a closure, `table`, and asks both of it.
- **The deletes take the look and the locale beside the workspace.** What a refusal says (`delete_said`, `sweep_said`) is written in the look's case and the locale's words. `Paint` would hand them a palette they do not read. `delete_count` takes seven arguments, clippy's limit.
- **`history_chords` takes the locale too:** Copy SQL, which it reaches through `history_command`, says so in a toast.
- **`agents_shows`, `agents_lists` and `agents_here` keep `&App`.** Each asks `App::agents_here(tab)`, which joins the workspace with the connection's agents access and with the server being on, and `agents_lists` reads the sessions `agents` lists for the connection. Handed that answer, each caller would build it first: `App` one call down, which the plan's rule for a helper reading more than one workspace's state keeps. They find the workspace for `agents_view` with `app.workspace(tab)`.
- **No helper `handle` calls once reads more than one workspace.** The three above read the workspace and state beside it (`agents`, `connections`), and are called from several places in `handle` and `letters`.
- **`letters` and `editing_keys` lend the workspace they found.** `letters` found it already, for the terminal look's keys; every narrowed call below that point takes it. `editing_keys` finds the workspace and then its table tab, where it found the tab in one chain: it returns early in the same frames as before.

## What did not change

- **The order of actions.** Each helper pushes into the `actions` it is lent where it pushed, and is called where it was. No call moved.
- **What is asked, and when.** Each narrowed call finds the workspace the helper found itself, with the same `app.workspace(tab)`, at the same place. Where a helper returned `None` or `false` for a closed workspace, its caller's `and_then` or `is_some_and` does. The deletes are called only where a workspace was found, so the case in which `delete_row` pushed `DeleteRow` with no workspace is not reached by any caller, as it was not before: `letters` returns before them without one, and `editing_keys` without a table tab.
- **The focus order.** Nothing is drawn here.

## What this run teaches

- **A helper that reads one slice of a router's state takes the slice, and the router keeps finding it.** Where the router has found the slice already (`letters`' workspace), the calls get shorter; where it has not (`handle`), `app.workspace(tab).and_then(helper)` reads as plainly as the helper's old call did.
- **A helper that asks an answer of `App` that joins state (`agents_here`) stays on `App`** while every caller would have to build that answer first.

## Tasks

### Task 1: the helpers

- [x] `switchable`, `history_shown`, `agents_log` and `agents_view` take `&Workspace`; `selection_row` and `on_last_row` take `&ObjectTab`.
- [x] `delete_row`, `delete_selected` and `delete_count` take `&Workspace`, the look and the locale.
- [x] `history_chords` takes `&Keymap`, `Layout` and the locale; `history_command` the locale; `window_letter` `&Keymap` and `Layout`.
- [x] `handle`, `editing_keys` and `letters` find the workspace or the tab and lend it.

### Task 2: CI's checks and the PR

- [x] `cargo fmt --all --check`
- [x] `cargo clippy --locked --workspace --all-targets -- -D warnings`
- [x] `cargo test --locked --workspace --all-targets`
- [x] `RUSTDOCFLAGS='-D warnings' cargo doc --locked --workspace --no-deps`
- [ ] Push and open the PR.

The net is the existing tests, which drive the whole `App` through its keys: `keyboard_tests.rs` and the keys' tests in `src/ui/mod.rs` for the window's letters and the start screen's typing; `history_tests.rs` for the History tab's chords, letters, Copy SQL and Clear history's answers; `agents_tests.rs` for the Agents tab's letters, its Log view and Enter on a selected call; `delete_tests.rs`, `delete_many_tests.rs` and `grid_keys_tests.rs` for the deletes, a refusal's words, a visual selection's delete and a count's, and `insert_audit_tests.rs` for Down on the last row adding one; `start_tests.rs` for Switch database from the start screen. No test changed: every helper is private to `keys.rs`, and no test calls one.

## For later

This run ends the routers' plan. Its closing section, in the plan, lists what still takes `App` under `src/ui/` and why, and what a separate core crate would need.

# Narrowing what the routers see of `App`, run 3: a workspace's connecting screens

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** What a workspace shows before its database does (how connecting goes, or why it failed) and the strip over a tab whose connection was lost take `ui::Paint` and the workspace instead of `&mut App`, and return their actions. The connection bar's height and the line its contents centre on take the look and the title bar's height. `workspace::show` lends each screen the workspace it found and extends `app.actions` after it. This follows the routers' plan (`2026-11-01-narrow-routers.md`, run 3). Behaviour does not change.

**Files:**
- Modify: `src/ui/workspace.rs` (`show`, `opening`, `connecting`, `failure`, `banner`, `top_bar_height`, `bar_line`, and `top_bar`'s call of `top_bar_height`)
- Modify: `src/ui/mod.rs` (`window_buttons_line`'s call of `bar_line`)
- Modify: `docs/superpowers/plans/2026-11-01-narrow-routers.md` (run 3)

## Survey (2026-11-01, `main` at `a846335e`)

| Function | What it read of `App` | Now takes |
|---|---|---|
| `workspace::opening` | `palette.window`; the workspace (`workspace(tab)`), for whether the connect failed | `ui, paint, tab, &Workspace`, returns the actions of the screen it draws |
| `workspace::connecting` | Paint, the workspace; pushed Cancel connecting's `Disconnect` | `ui, paint, tab, &Workspace`, returns its actions |
| `workspace::failure` | Paint, the workspace; pushed Retry's `Reconnect`, then Edit connection's `EditConnection` | `ui, paint, tab, &Workspace`, returns its actions in that order |
| `workspace::banner` | Paint, the workspace; pushed Reconnect's `Reconnect`, then Disconnect's `Disconnect` | `ui, paint, tab, &Workspace`, returns its actions in that order |
| `workspace::top_bar_height` | `look`, `titlebar.height` | `look, titlebar, zoom` |
| `workspace::bar_line` | `look`, through `top_bar_height` `titlebar.height` | `look, titlebar, zoom` |
| `ui::window_buttons_line` | `active_tab`, then `look` and `titlebar` through `bar_line` | unchanged: keeps `&App` and passes `&app.look` and `app.titlebar.height` |

`top_bar` keeps `&mut App` until run 4. It calls `top_bar_height` with `&look` and `app.titlebar.height`.

Where the survey differs from the plan:

- **No screen answers a closed workspace.** The plan allowed `Option<&Workspace>` for one that does. `workspace::show` finds the workspace once and returns where it is closed, before any screen is drawn, so `opening` and its two screens are lent the one it found. `banner` comes after the terminal look's object tabs and status line, which take `&mut App`, so the router finds the workspace again there, as `banner` found it itself: where none is found, nothing is drawn and nothing is pushed, as before. Each screen takes `&Workspace`.
- **`top_bar_height` and `bar_line` keep `zoom`.** The title bar's height is in window points, and the bar's in egui's, so both still take egui's `zoom` beside the look and that height, and divide where they divided.
- **`show` asks whether the workspace has opened before the terminal look's tabs and status line,** where it asked after. Those two were drawn only for an opened workspace and the screens only for one that has not opened, so no frame draws a different thing or in a different order. Asking first lets the router lend `opening` the workspace it found without a borrow across the two calls that take `&mut App`.

## What this run teaches

- **A screen the router draws inside a panel returns the panel's `inner`.** `opening` draws its `CentralPanel` and returns what `failure` or `connecting` returned in it, so the router extends `app.actions` once, after the panel. Nothing else pushed between, so the frame's order holds.
- **A screen that draws nothing for a status returns an empty list** where it returned (`banner` for a connected, idle or woken workspace).
- **The router lends the workspace it found** where nothing between the find and the call takes `&mut App`, and finds it again where something does.
- **Each screen is drawn where it was,** so the notices' buttons still come before Cancel connecting, Retry, Edit connection and Copy details, and the terminal look's tabs and status line before the strip's Disconnect and Reconnect, in the order Tab moves.

## Tasks

### Task 1: the screens and the bar's measures

- [x] `opening`, `connecting`, `failure` and `banner` take `ui, paint, tab, &Workspace` and return their actions, pushed in the order they reached `app.actions` before.
- [x] `top_bar_height` and `bar_line` take `look, titlebar, zoom`. `top_bar` and `window_buttons_line` pass the look and `app.titlebar.height`.
- [x] `workspace::show` lends `opening` the workspace it found, finds it again for `banner`, and extends `app.actions` after each.

### Task 2: CI's checks and the PR

- [x] `cargo fmt --all --check`
- [x] `cargo clippy --locked --workspace --all-targets -- -D warnings`
- [x] `cargo test --locked --workspace --all-targets`
- [ ] Push and open the PR.

The net is the existing tests, which drive the whole `App`: the connecting and failure screens' in `src/ui/mod.rs` (the steps and Cancel connecting, the failure card and its buttons, Copy details) and `src/app.rs`, the lost connection's strip and its buttons in `src/ui/mod.rs` and `src/app.rs`, `the_mac_window_buttons_centre_on_the_line_of_the_bar_that_leads` for `bar_line`, and the shots of these screens in `src/shots.rs` (`state-connecting`, `state-connect-auth`, `state-connect-unreachable`, `state-connect-tls`, `state-disconnected`). No test changed: no signature a test calls moved.

## For run 4

`top_bar` still takes `&mut App`. It already reads the look and `app.titlebar` beside the bar's height, so run 4 passes the title bar's inset and height with the rest of what the bar reads.

# Narrowing what the routers see of `App`, run 1: the notices bar

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** The bar under the window's leading bar takes `ui::Paint` and what it shows instead of `&mut App`. That bar is the held agents' changes, then the app's notice, and `ui::workspace::show` and `ui::picker::show` both draw it. The settings terminal's notice band takes its skin and the notice. This follows the routers' plan (`2026-11-01-narrow-routers.md`, "The pattern for a router's chrome"). Behaviour does not change.

**Files:**
- Modify: `src/ui/mod.rs` (`notices`, `notice`, `agents_held`, `agents_held_said`, `notice_line`)
- Modify: `src/ui/workspace.rs` (`show`), `src/ui/picker.rs` (`show`): each passes what the bar reads and extends `app.actions` where the bar did
- Modify: `src/ui/settings/terminal.rs` (`notice`, and its call in `show`)
- Modify: `src/app.rs` (five tests), `src/ui/agents_tests.rs` (one test): `agents_held_said`'s signature moved

## Survey (2026-11-01, `main` at `282438b6`)

| Function | What it read of `App` | Now takes |
|---|---|---|
| `ui::notices` | through the two below | `ui, paint, notice: Option<&str>, held: &[Raised], saved: &[SavedConnection]`, returns `(f32, Vec<Action>)` |
| `ui::notice` (private) | `notice` (cloned), `palette`; extended `actions` | `ui, paint, notice, &mut Vec<Action>`, returns its height |
| `ui::agents_held` (private) | Paint, `agents_held` (cloned for the buttons); extended `actions` | `ui, paint, held, saved, &mut Vec<Action>`, returns its height |
| `ui::agents_held_said` | `locale`, `agents_held`, `connections.connections` | `locale, held, saved` |
| `ui::notice_line` | Paint | `ui, paint, message, &mut Vec<Action>` |
| `settings::terminal::notice` | `notice`, `palette` | `ui, &Skin, notice, &mut Vec<Action>` |

The routers' plan had these right. Nothing was more tangled than it said.

## What this run teaches

What later runs reuse:

- **Chrome that two routers draw is a `ui` function that returns its measure and its actions,** `(f32, Vec<Action>)`. Each router calls it with the three fields it reads (`app.notice.as_deref()`, `&app.agents_held`, `&app.connections.connections`) and `Paint::of(app)`, and extends `app.actions` right there. The picker gathers its own actions in a local and extends them at its end. It has pushed none before the bar, so the frame's order is unchanged: the bar's actions, then the picker's.
- **The parts inside such chrome push into one local `actions`** (private helpers keep `&mut Vec<Action>`, as the views' private helpers do). The held bar's are pushed before the notice's, the order in which they reached `app.actions` before.
- **A helper that `ui` and a settings screen share takes `Paint`.** The terminal's skin holds `&Look` and `&Palette`, so `settings::terminal::notice` builds a `Paint` from the skin's three fields where it calls `notice_line`. Its own band reads `skin.palette`.
- **A function the tests ask of the app (`agents_held_said`) takes the fields it read,** and the tests pass them. There is no `&App` wrapper kept for the tests.

## Tasks

### Task 1: `notices` and what it draws

- [x] `notices` takes `ui, paint, notice, held, saved` and returns `(height, actions)`, the held bar's before the notice's.
- [x] `notice` and `agents_held` push into the bar's `actions` and return their heights. `notice` no longer clones the notice, and `agents_held` clones `held` for its buttons, as it cloned `app.agents_held`.
- [x] `agents_held_said` takes `locale, held, saved`. `notice_line` takes `ui, paint, message, actions`.

### Task 2: the callers

- [x] `ui::workspace::show` and `ui::picker::show` call `notices` with the three fields and `Paint::of(app)`, and extend `app.actions` there.
- [x] `settings::terminal::notice` takes `&Skin` and `app.notice.as_deref()`.
- [x] The six tests that call `agents_held_said` pass `locale`, `&agents_held` and `&connections.connections`.

### Task 3: CI's checks and the PR

- [x] `cargo fmt --all --check`
- [x] `cargo clippy --locked --workspace --all-targets -- -D warnings`
- [x] `cargo test --locked --workspace --all-targets`
- [ ] Push and open the PR.

The net is the existing tests:

- the held bar's: `src/ui/agents_tests.rs`'s `the_held_bar_says_what_changed_and_its_buttons_answer_it`, the held tests in `src/app.rs`, `src/ui/agent_settings_tests.rs`;
- the notice's in `src/ui/mod.rs`: `a_notice_stands_under_the_bar_that_leads_the_window`, `a_notice_is_solid_so_nothing_under_it_shows_through`, `a_notice_shows_until_dismissed`, `the_settings_screen_shows_a_notice_until_it_is_dismissed`;
- every picker and workspace test, which draws the bar each frame.

Only the six calls of `agents_held_said` moved.

## For run 2

The picker's header is drawn before the notices and its body after them. With `notices` narrow, `picker::show` can split into those two parts and keep only the routing between them.

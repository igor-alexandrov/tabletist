# Moving the lower layer into a crate of its own, run 1: the lower layer's tests

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** The lower layer's tests stand on the lower layer: none names `crate::ui`, `crate::app` or the harness in `crate::testing`, so they can move into the core crate with their modules. The data fixtures they use move from `src/testing.rs` to a lower module, `src/fixtures.rs`, and `tests/layers.rs` reads the lower layer's test modules too. This is run 1 of `2026-11-02-core-crate.md`. Behaviour does not change: only tests and their fixtures move.

**Files:**
- Create: `src/fixtures.rs`
- Modify: `src/lib.rs`, `src/testing.rs`
- Modify: the tests of `src/edit.rs`, `src/model.rs`, `src/review.rs`, `src/key_picker.rs`, `src/settings.rs`, `src/theme.rs`, `src/keymap.rs`, `src/typography/fonts.rs`
- Modify: the tests of `src/app/commands.rs`, `src/ui/grid.rs`, `src/ui/value_tags.rs`, `src/ui/keyboard_tests.rs`
- Modify: `tests/layers.rs`, `AGENTS.md`
- Modify: `src/env.rs` (`no_view_writes_a_hex_colour` allows `src/fixtures.rs`)
- Modify: `docs/superpowers/plans/2026-11-02-core-crate.md` (run 1's checkboxes)

## Survey (2026-11-02, `main` at `e9b7547d`)

What the lower layer's tests named above them, before this run:

| Where | What | Now |
|---|---|---|
| `edit` (37), `model` (21), `review` (4), `key_picker` (2), `settings` (1) | `crate::testing`'s data: `book_covers_*`, `publishers_*`, `genres_*`, `fixture_structure`, `page`, `workspace`, `zones`, `UTC`, `rows_outcome`, `cut_outcome`, `done_outcome`, `error_outcome`, `script_outcome`, `run_script` | `crate::fixtures`, same names |
| `theme`'s `data_is_monospace_on_macos_and_omarchy` | `crate::ui::grid::data_role` | in `src/ui/grid.rs`'s tests, beside `data_role` |
| `theme`'s `value_tags_take_a_themes_rose_and_olive_only_when_they_stand_apart` | `crate::ui::value_tags::terminal_slots` | in `src/ui/value_tags.rs`'s tests, beside `terminal_slots` |
| `keymap`'s `a_menu_key_holds_cmd_or_ctrl_and_is_one_press` | `crate::app::RUNNABLE` | in `src/app/commands.rs`'s tests, beside `RUNNABLE` |
| `typography/fonts.rs`'s `macos_draws_the_keys_its_shortcuts_name` and `the_mac_has_a_glyph_for_every_key_the_keymap_writes` | `crate::testing::Harness` | in `src/ui/keyboard_tests.rs`, which `tests/keys.rs` lets spell keys (the first writes `⌘⇧⌫↩`) |

What moved to `src/fixtures.rs`: every free item of `src/testing.rs` that takes no `App`, `Harness` or egui type, 35 of them with their doc comments (the plan's list, and `fixture_objects` and `bookshop_objects`, the schema the fake session lists, which are data too), and `tokyo_night` from `theme`'s tests, which `theme`'s tests and the moved value-tags test both use. `zones_shared` became `pub(crate)`: the harness answers `Event::Zones` with it. What stays in `testing`: the `Harness` and its impls, `node`, `bounds`, `labels`, `key`, `release`, `last_sent`, `asked_access`, `leave_edit`, and its own test.

Where the run differs from the plan:

- **`tests/layers.rs` reads every test module, not only `mod tests`.** `typography/desktop_font.rs` has three (`parse_tests`, `name_tests`, `load_tests`), which the old search read as code. `tests_module` became `test_modules`: each `#[cfg(test)]` module at the margin, whatever its name. Code is what is outside all of them; each is read with the same path search, one module deeper (in `src/a.rs`, `super::super` is the root inside `mod tests`), for `ui`, `app` and `testing`.
- **`env`'s `no_view_writes_a_hex_colour` allows `src/fixtures.rs`.** It reads every source file outside test modules for a hex colour, and Tokyo Night's seed (`#1a1b26`, …) had been inside `theme`'s tests. `src/fixtures.rs` is test data as a whole, as `src/shots.rs` is, so it joins the files the test allows, with the reason in its comment.
- **The new check was shown to fail** by putting one `crate::testing::UTC` back into `edit.rs`'s tests: it named the file and the path. A unit test of the search, `the_search_reads_every_test_module_one_module_down`, holds the same.

## What did not change

- **Every test still runs.** 3,710 `#[test]` functions under `src/` and `tests/` before, 3,712 after: the five moved ones under new paths, and `tests/layers.rs`'s two new ones.
- **What the interface's tests name.** `testing` re-exports the fixtures with `pub use crate::fixtures::*;`, so `crate::testing::book_covers_page` and the rest still resolve, and no test above changed its imports.
- **The non-test code.** Nothing outside `#[cfg(test)]` moved, so no release or dev build changes.

## What this run teaches

- **Test data is lower-layer code.** The Bookshop's structures and pages are values of `tabletist_db` and `model` types, and nothing about them needs the window. Kept beside the harness they tied the lower layer's tests to the interface; beside the types they serve, they can cross into the core crate with them, under its `testing` feature (run 4).
- **A test that calls the interface belongs to the interface.** Each of the five that moved tests a function above (`data_role`, `terminal_slots`, `RUNNABLE`, the harness's faces) more than the module it sat in.

## Tasks

### Task 1: the fixtures go down

- [x] `src/fixtures.rs` holds the data fixtures and `tokyo_night`; `src/lib.rs` declares it under `#[cfg(test)]`.
- [x] `src/testing.rs` re-exports them; its imports lost what only the fixtures used.
- [x] The lower layer's tests name `crate::fixtures`.

### Task 2: the tests that reach the interface go up

- [x] `keymap`'s menu-key test to `src/app/commands.rs`; `theme`'s two to `src/ui/grid.rs` and `src/ui/value_tags.rs`; `typography/fonts.rs`'s two to `src/ui/keyboard_tests.rs`.

### Task 3: `tests/layers.rs` reads the lower layer's tests

- [x] `fixtures` in `LOWER`; `test_modules`, `tests`, `paths_to` and the two tests above; `AGENTS.md`.

### Task 4: CI's checks and the PR

- [x] `cargo fmt --all --check`
- [x] `cargo clippy --locked --workspace --all-targets -- -D warnings`
- [x] `cargo test --locked --workspace --all-targets`
- [x] `RUSTDOCFLAGS='-D warnings' cargo doc --locked --workspace --no-deps`
- [ ] Push and open the PR.

## For later

Run 2 (text and style go up) and run 3 (what the boundary breaks that `tests/layers.rs` cannot see) are independent of each other; either can go next. Each re-surveys from `main`.

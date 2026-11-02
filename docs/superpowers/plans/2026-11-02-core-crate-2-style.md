# Moving the lower layer into a crate of its own, run 2: text and style go up

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** What sets up an `egui::Context` and what draws text leave the lower layer, and what reads font files from the system comes down into it, so the core crate (run 4) needs neither `fastframe-text`, `fastframe-fonts`, `fastframe-icons` nor `egui_extras`, and names no `egui::Context`. This is run 2 of `2026-11-02-core-crate.md`. Behaviour does not change: code moves, and its callers name its new place.

**Files:**
- Create: `src/style.rs` (from `src/theme.rs`)
- Create: `src/font_files.rs`; `git mv` `src/typography/desktop_font.rs` and `src/typography/installed_font.rs` to `src/font_files/`
- Modify: `src/theme.rs`, `src/typography.rs`, `src/typography/fonts.rs`, `src/lib.rs`
- Modify: `src/backend.rs`, `src/app.rs`, `src/testing.rs` (callers)
- Modify: 30 files under `src/ui/` (`Icon`'s imports, their tests' `install`/`apply`, and `text_settings_tests.rs`'s `Read`)
- Modify: `tests/layers.rs`, `AGENTS.md`
- Modify: `docs/superpowers/plans/2026-11-02-core-crate.md` (run 2 done, the `Icon` finding)

## Survey (2026-11-02, `main` at `6cf97c62`)

What of `typography` the lower layer named before this run:

| Where | What | Now |
|---|---|---|
| `backend` | `read_font`, `desktop_font::Read` (in `Event::Font`), `installed_font::families` | `crate::font_files::…`, same names |
| `theme`'s `install_with`, `set_read_faces`, `apply_to_style` | `TerminalFaces`, `Kind`, `configure`, `family_name`, `data_family_name`, `text_styles` | in `src/style.rs`, above; `TerminalFaces` from `font_files` |
| `theme`'s `the_settings_size_the_interface_and_the_data_apart` | `TextRole` | in `typography`'s tests (a new `mod tests`), with its own `text` helper |
| `theme`'s doc comments | links to `crate::typography` and `crate::typography::installed_font` | a code span for `typography`; a link to `crate::font_files::installed_font` |

What `font_files` itself names: `theme::DataFace` (in `installed_font`), which stays down, and its own two files. Nothing else of `typography` had to come down with it.

What else of `theme` had to leave the core, which the plan had not listed:

- **`Icon`.** `fastframe_icons::icons!` makes it, and the core's `Cargo.toml` is to list no `fastframe-icons`. Only the interface names it (28 files under `src/ui/`) and `install_with` installs it, so it goes up into `style` with `install_with`. Its `directory: "../assets/icons/"` is relative to the invoking file, and `src/style.rs` stands where `src/theme.rs` does.

After the run the lower layer names no `fastframe_text`, `fastframe_fonts`, `fastframe_icons`, `egui_extras`, `egui::Context`, `egui::Ui`, `egui::Painter` or `egui::Response` outside its tests, and its tests name no `egui::Context` either. `theme` is left with `egui::Color32` and `egui::Theme`, `fastframe_theme` and `serde`.

What moved:

- **To `src/style.rs`:** `text_rendering`, `install`, `install_with`, `set_read_faces`, `apply`, `apply_to_style` and `Icon`, unchanged, and the seven tests of `theme` that build an `egui::Context` (`applying_a_palette_sets_egui_visuals`, `a_palette_survives_an_os_theme_switch`, `the_standard_look_keeps_todays_metrics`, `the_omarchy_look_is_square_with_bordered_controls`, `omarchy_borders_follow_the_text_colour_in_light_themes`, `the_macos_look_rounds_borderless_controls`, `focused_fields_keep_the_accent_stroke_in_every_look`).
- **To `src/font_files.rs` and `src/font_files/`:** `desktop_font.rs` and `installed_font.rs` by `git mv`, and `TerminalFaces` (from `typography/fonts.rs`) and `read_font` (from `typography.rs`). `desktop_font` names `super::TerminalFaces`.
- **To `typography`'s tests:** `the_settings_size_the_interface_and_the_data_apart`, which tests `TextRole::size` under the settings.

What the callers name now: `App` calls `style::apply`, `style::install_with` and `style::set_read_faces`, and holds a `font_files::desktop_font::Read`; the harness calls `style::install_with`; the interface's tests call `style::install` and `style::apply`; `text_settings_tests.rs` names `font_files::desktop_font::Read`; the views take `Icon` from `crate::style`. The entrypoint names none of them: it calls `App::attach` once the window exists, and `attach` sets the style up.

Where the run differs from the plan:

- **`Icon` goes up too** (above).
- **`installed_font`'s test reads Plex Mono's bytes itself.** `the_font_the_settings_name_is_read_from_its_files` compared a read face with `super::super::fonts::PLEX_MONO`, the bundled bytes in `typography/fonts.rs`, which stay above. It now takes `include_bytes!` of the same file, as it already did for Plex Mono Medium; the path is the same depth from `src/font_files/`.
- **`typography/fonts.rs` gates `Arc` and `FontData` by platform.** `TerminalFaces` was the one item there that named them on every platform; without it they are used only on macOS and Linux (and in tests), as `VariationCoords` already was, and Windows would warn of two unused imports, which CI's clippy denies. They take the same `cfg` as `VariationCoords`. Type-checked in a scratch crate (the real `fonts.rs` through `#[path]`, with stubs for what it names) with `cargo clippy --target … -- -D warnings`: clean for `x86_64-pc-windows-msvc`, `aarch64-apple-darwin` and Linux; the file without the `cfg` fails for Windows on exactly those two imports. Type-checked in an extract, not run: CI runs it on each.
- **`tests/layers.rs` forbids `typography` and `style` to the lower layer.** Moving `typography` into `ABOVE` alone would not stop a lower module naming it, since the search looked only for `ui` and `app` (and `testing`, in tests). Both lists now hold `typography` and `style`. The check was shown to fail with one `type Probe = crate::typography::Kind;` put at the foot of `src/theme.rs`: it named the file and the path, and passed once it was taken out.
- **`tests/layers.rs` reads a test module one platform compiles as tests** (from the review). Run 1's record says the search reads `desktop_font.rs`'s three test modules; it read only `name_tests`, the one under a bare `#[cfg(test)]`. `parse_tests` and `load_tests`, under `#[cfg(all(test, target_os = "linux"))]`, were read as code at the file's depth, so a `super::super::super::style` in one would have passed. `marks_tests` now takes `#[cfg(all(…, test, …))]` too, and a test of its own, `a_test_module_one_platform_compiles_is_read_as_tests`, holds it. The search's tests also gained cases of `typography` and `style` in code and in a test module (from the review).

## What did not change

- **Every test still runs.** 3,712 `#[test]` functions under `src/` and `tests/` before, 3,713 after: the eight moved ones under new paths, and `tests/layers.rs`'s new one.
- **The code moved as it was.** No function body changed, apart from the one test's bytes above; the font files keep their `cfg(target_os)` paths as they were, under their new directory. The only platform-specific change is the two imports above.
- **`tests/keys.rs`** lists `src/typography/fonts.rs`, which did not move.

## What this run teaches

- **Data is what crosses, setting it up is not.** `theme` had both: the palette and the looks, which the settings, the keymap and the backend read, and the code that pours them into egui's style, which only the window runs. Split, the data needs only `Color32`; the setting up keeps every egui dependency the core does not want.
- **A macro's dependency counts as much as a call's.** `Icon` looked like an enum of data, but the crate behind its macro is one the core must not have: a re-survey that lists the crates a module names, not only the paths, finds it.

## Tasks

### Task 1: style goes up

- [x] `src/style.rs` with `text_rendering`, `install`, `install_with`, `set_read_faces`, `apply`, `apply_to_style`, `Icon` and the seven context tests; `src/lib.rs` declares it.
- [x] `theme`'s imports and doc comments lose what went.
- [x] `App`, the harness, the interface's tests and the views name `style`.

### Task 2: font reading comes down

- [x] `git mv` of `desktop_font.rs` and `installed_font.rs` to `src/font_files/`; `src/font_files.rs` with `TerminalFaces` and `read_font`; `src/lib.rs` declares it.
- [x] `typography` keeps `fonts` and `configure`; `fonts.rs` takes `TerminalFaces` from `font_files`, and gates `Arc` and `FontData` as `VariationCoords` is.
- [x] The backend, `App` and the text settings' tests name `font_files`.
- [x] `theme`'s text-size test moves to `typography`'s tests.

### Task 3: `tests/layers.rs`

- [x] `typography` and `style` in `ABOVE`, `font_files` in `LOWER`; `typography` and `style` in what the lower layer may not name; `AGENTS.md`.
- [x] A test module under `#[cfg(all(test, …))]` is read as tests (review).

### Task 4: CI's checks and the PR

- [x] `cargo fmt --all --check`
- [x] `cargo clippy --locked --workspace --all-targets -- -D warnings`
- [x] `cargo test --locked --workspace --all-targets`
- [x] `RUSTDOCFLAGS='-D warnings' cargo doc --locked --workspace --no-deps`
- [ ] Push and open the PR.

## For later

Run 3 (the version, `impl TabName`, `model`'s doc links to `App`) is independent of this one; run 4, the move, comes after both. Each re-surveys from `main`.

# Moving the lower layer into a crate of its own Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** The lower layer (the modules `LOWER` in `tests/layers.rs` lists: the app's state and its edits, the backend, the settings, the keymap, the theme's data and the text they write) moves out of the `tabletist` crate into a workspace crate of its own, `tabletist-core`, so an edit to the interface no longer compiles it again. The first runs untangle what the crate boundary would break, inside the one crate; the move is a later run. Behaviour does not change.

**Architecture:** `tabletist-core` sits under `crates/`, beside `tabletist-db`. It depends on `egui` for three data types (`Key`, `Color32`, `FontData`), never on `eframe`, and never names an `egui::Context` or `egui::Ui`. The root crate keeps the interface, the reducer, the entrypoint and the tests' harness, and re-exports each core module at its old path (`pub use tabletist_core::model;`), so `crate::model::…` keeps meaning what it does everywhere above. The crate boundary then does the work `tests/layers.rs` does today: a core module cannot name `crate::ui`, because the core crate does not depend on the root one.

**Tech Stack:** Rust 2024 workspace, Cargo (resolver 3), egui and eframe 0.36 (crmne fork), fastframe v0.13.0.

**Scope:** The baseline, the plan and run 1 are this PR. Each later run is one PR from `main`, in its own session, leaving `main` green. No run changes behaviour, and none touches the release profile (`lto = "thin"`, `codegen-units = 1`): a faster profile was declined for now.

---

## Baseline (2026-11-02, `main` at `e9b7547d`)

This Linux box: 32 threads, 125 GB, other sessions building beside it (load average 5 to 8 while these ran, so a few seconds either way are noise). `~/.cargo/bin/cargo`, `CARGO_TARGET_DIR` in the worktree, `--locked`. "Lower" is a real value change in a lower-layer file, `completion::TABLES` 8 → 9 and back; "UI" is one in an interface file, `ui::grid::GUTTER` 22.0 → 23.0 and back. Each after-an-edit figure is the mean of the change and its revert.

| Build | Wall time | Where it goes |
|---|---|---|
| `cargo build --release`, clean | 191 s | dependencies until 51 s, then the `tabletist` lib 84 s, then the bin 56 s |
| `cargo build --release`, after a lower edit | 141 s (141.5, 141.1) | the lib 85 s, the bin 56 s |
| `cargo build --release`, after a UI edit | 141 s (141.5, 140.9) | the same |
| `cargo build`, clean | 109 s | |
| `cargo build`, after a lower edit | 5.0 s (5.0, 5.1) | |
| `cargo build`, after a UI edit | 5.1 s (5.1, 5.1) | |
| `cargo check`, dependencies built | 38 s | |
| `cargo check`, after a lower edit | 1.9 s (1.9, 1.9) | |
| `cargo check`, after a UI edit | 1.9 s (1.9, 1.9) | |

Where a release build's time goes, from one `-Z time-passes` run of each (`RUSTC_BOOTSTRAP=1` on the pinned 1.98):

- **The lib, 87 s:** LLVM's passes over its one codegen unit 71 s, single-threaded; everything before it (type checking, borrow checking, monomorphization, metadata) about 16 s.
- **The bin, 57 s:** thin LTO 56 s. Every crate is one module, so the step runs in parallel across crates, and `tabletist`'s module, by far the biggest, is the long pole.

So about 127 of the 141 s after an edit are LLVM working on one module of 120,000 lines, on one thread at a time, while 31 others wait.

One experiment shows how much that matters: `--config 'profile.release.package.tabletist.codegen-units=N'`, the `tabletist` package only, every other setting as it is:

| `codegen-units` for `tabletist` | the lib | the bin (thin LTO) | after an edit | stripped binary |
|---|---|---|---|---|
| 1 (today) | 84 s | 56 s | 141 s | 40.6 MB |
| 2 | 48 s | 36 s | 84 s | 48.1 MB |
| 4 | 29 s | 23 s | 52 s | 49.0 MB |

The binary grows by 18% with a second unit; how fast it runs was not measured.

## What the split buys, and what it does not

A crate split gives LLVM two modules instead of one, so it buys mostly what a second codegen unit buys, less the imbalance: the core is about a third of the non-test code (39,400 of 120,600 lines, `typography` gone up), the interface two thirds. An estimate, from those shares and the passes above, not a measurement:

- **Release, after an interface edit:** the core is not compiled. The root's lib is about two thirds of today's (11 s of front end, 48 s of LLVM), and thin LTO's long pole is the root's module, about two thirds of 56 s, a little more for what it imports from the core. About **100 s instead of 141 s**.
- **Release, after a core edit:** the core compiles first, but the root starts once the core's metadata is out (about 5 s in), and the core's LLVM runs beside the root's. About **105 s**. It is no slower than an interface edit, because the gain is parallelism, not skipped work.
- **Release, clean:** the same 40 s or so off the 191 s.
- **`cargo build` and `cargo check` after an edit:** no gain worth naming. They take 5 s and 1.9 s today, because the dev profile is incremental and the dependencies are optimized once. A split may move them by a fraction of a second either way.
- **Not helped:** thin LTO still runs over every crate at every link, and the root's module is still the long pole of it. The split does not make the interface compile faster than two thirds of today's time, and `ui` alone is 133,000 lines, more than half of the crate.

So the honest numbers: the split takes about 40 s off a 141 s release build after an edit, or 30%. **`codegen-units = 2` on the `tabletist` package alone takes 57 s off, and 4 take 89 s, with no code moved**, at the cost of a bigger binary (and an unmeasured effect on its speed). That is a one-line change to the release profile, not the `[profile.fast]` that was declined, and it is the owner's call; this plan does not make it. The two add up: after the split, each crate's codegen units still divide its LLVM work.

What the split is still for, with or without that change: the boundary becomes the compiler's, not a test's; the core's tests build and run without the interface (`cargo test -p tabletist-core`); an interface edit stops compiling the core again in every profile; and it is the step that makes a later split of `ui` possible at all.

## Survey (2026-11-02, `main` at `e9b7547d`)

### Sizes

Lines of Rust, tests included:

| Part | Lines |
|---|---|
| The lower layer (34 modules) | about 73,000 |
| `ui` | 133,283 |
| `app` | 34,497 |
| `testing`, `shots`, `promo` (tests only) | 6,819 |
| `entrypoint`, `menu`, `macos`, `main`, `lib` | 3,035 |
| The crate | about 250,000 |

The biggest lower modules: `backend` 11,027, `model` 8,384, `settings` 6,633, `keymap` 6,488, `edit` 6,267, `update` 3,513, `dates` 2,946, `connections` 2,480, `typography` 2,197.

### What each lower module takes from egui and fastframe

Only five lower modules name egui at all. The rest use the standard library, `tabletist_db`, `tabletist_mcp`, tokio, serde, jiff, toml, tempfile, rusqlite and the keyring stores.

| Module | egui and fastframe | Kind of use |
|---|---|---|
| `keymap` | `egui::Key` (`use egui::Key`, 123 `Key::` in its table of names and `Stroke::Key`); `keymap/overrides.rs` names `egui::Key::Escape` and `::B` | an enum, data |
| `env` | `egui::Color32` (`EnvColors`), `fastframe_theme::parse_palette` and the Omarchy seed | data |
| `desktop_theme` | `egui::Color32`, `fastframe_theme::parse_palette` and the Omarchy seed | data |
| `theme` | `Color32` (its `Palette`), `egui::Theme`; `install`, `install_with`, `set_read_faces`, `apply` take an `egui::Context` and build `FontDefinitions`, `Style`, `Visuals`, `Shadow`, `Margin`; `fastframe_theme` (`CustomTheme`, `Catalog`, `Palette`), `fastframe_text`, `fastframe_fonts`, `fastframe_icons`, `egui_extras::install_image_loaders` | data, and the interface's style |
| `typography` | egui's text throughout (`LayoutJob`, `Galley`, `FontId`, `TextStyle`, `WidgetInfo`, label selection, a `Context` to lay out on); `typography/desktop_font.rs` and `installed_font.rs` read font files into `egui::FontData` with `skrifa` and, on macOS, `tabletist_appkit`; `typography/fonts.rs` embeds the bundled fonts and fills `FontDefinitions` | the interface's text, and reading font files |

The finding that decides the egui question: **`fastframe-theme` and `fastframe-fonts` depend on `egui` themselves** (`Cargo.lock`), and `env`, `desktop_theme` and `theme` need `fastframe_theme` to read palettes and Omarchy's theme. A core crate free of egui would need fastframe split first, which is not ours to do. So the core crate depends on `egui`, as fastframe-theme does, and the untangling is about the interface, not about egui:

- **`keymap` keeps `egui::Key`.** A key type of its own with a conversion in the interface would buy nothing while egui is in the core's tree anyway, and would cost a conversion at every place the interface reads a press.
- **`theme`'s palette stays on `egui::Color32`** (which is `ecolor`'s, re-exported). Its data (`Palette`, `Look`, `Faces`, `Typeface`, `DataFace`, `PinFrom`, `mix`, `contrast`, the catalog and `resolve`) stays in the core. Its `install`, `install_with`, `set_read_faces`, `apply` and `apply_to_style` set up an `egui::Context`: they go up, to a module of the interface (run 2). So does its `Icon`, which `fastframe_icons::icons!` makes and only the interface names (found by run 2's re-survey).
- **`env` stays as it is.** `EnvColors` is in `Color32`s, which the core has.
- **`typography` goes up,** except reading font files. Its text roles, laying out and `configure` are the interface's ("views draw text only through `TextRole`s"), and an edit to them should not compile the core again. The backend reads fonts on its runtime (`Command::ReadFont`, `Command::ListFonts`), so `desktop_font.rs`, `installed_font.rs`, `read_font` and `TerminalFaces` (`[Arc<FontData>; 3]`) move down into a lower module of their own, `font_files` (run 2). The bundled fonts' bytes (`PLEX_SANS`, …, in `typography/fonts.rs`) go up with `configure`, which is all that reads them.

What the guard checks after the move: the core's `Cargo.toml` lists no `eframe`, `egui_extras`, `egui_kittest`, `fastframe-text` or `fastframe-icons`, and a test of its own (`crates/tabletist-core/tests/interface.rs`, as `tabletist-mcp`'s `tests/secrets.rs` reads its sources) finds no `egui::Context`, `egui::Ui`, `egui::Painter` or `egui::Response` in them.

### What lower modules take from each other

All inside the layer: `model` stands on `backend`, `connections`, `edit`, `env`, `keymap`, `settings`, `history`, `labels`, `recents`, `review`, `search`, `vim`, `completion`, `format`, `endpoint`, `agents_shown`, `util`; `backend` on nearly everything, including `desktop_theme` and `typography`'s font reading. One cycle crosses the line run 2 draws: `theme` names `typography`'s `Kind`, `TerminalFaces`, `configure`, `family_name`, `data_family_name` and `text_styles`, all of them in `install` and `apply`, which go up with it, and `typography` names `theme`'s data, which stays down. `keymap` names `theme::{Look, Typeface}`, which stay down.

### What the lower layer reaches above

`tests/layers.rs` keeps non-test code from naming `crate::ui` and `crate::app`, and it passes. What it does not read, and the crate boundary would break:

- **Tests.** The lower layer's `#[cfg(test)]` modules name `crate::testing` 62 times, in `edit` (37), `model` (21), `review` (4), `key_picker` (2), `settings` (1) and `typography/fonts.rs` (2, the `Harness`). All but the harness are data: the Bookshop's structures and pages, statement outcomes, `run_script`, a `workspace()` without an app, `zones()` and `UTC`. And three tests name the interface: `keymap`'s `a_menu_key_holds_cmd_or_ctrl_and_is_one_press` reads `crate::app::RUNNABLE`; `theme`'s `data_is_monospace_on_macos_and_omarchy` calls `crate::ui::grid::data_role`, and `value_tags_take_a_themes_rose_and_olive_only_when_they_stand_apart` calls `crate::ui::value_tags::terminal_slots` (run 1).
- **Doc links.** `model.rs` links `[`crate::app::App::start_matches`]` and `[`crate::app::App::quick_open_list`]`: across crates rustdoc cannot resolve them, and `cargo doc` runs with `-D warnings` (run 3). The other doc comments that name `ui::` or `app::` do so in code spans, which rustdoc does not resolve.
- **An inherent impl above.** `app/commands.rs` has `impl TabName { fn of(tab: &Tab) }`; `TabName` is `model`'s. Rust allows an inherent impl only in the type's own crate (run 3).
- **The version.** `update::CURRENT` (what a check compares a release with, and fastframe-update's `current_version`), `relay.rs` and `backend/agents.rs` read `env!("CARGO_PKG_VERSION")`. In the core crate that is the core's version, which would make the update check compare the wrong number with no error at all (run 3).

Trait impls above on lower types: one, `impl Link for crate::typography::Laid` in `ui/focus.rs`, which is the interface's own trait and stays legal wherever `typography` is.

### Visibility

33 items in the lower layer are `pub(crate)`. Across the boundary, those the root crate names must become `pub`; the rest stay `pub(crate)`, meaning the core. By name, the candidates the root crate may name: `history::Entry::read`, `completion::resolve`, `connections::file` (a `pub(crate) mod`) with `Problem`, its `line` and `what`, and `line_of`; `session`'s `of`, `object`, `sorts`, `bar`; `managed::read`; `keymap::presses`, `clash`, `Entry`, `entries`; `model`'s fields `changeset`, `then`, `overwrite`, `kept`; `agents_shown::same_columns`; `update::check::tests` with `RELEASE` and `ok`, which are test helpers (below). A name match overcounts (`read`, `of` and `bar` are common words): the move run lets the compiler say which, and makes only those `pub`. `pub(super)` and `pub(in …)` stay as they are: they are inside one module.

### Tests

- The lower layer's own unit tests are inline (`mod tests` at the foot of a file, a few `#[path]` test files). They move with their modules and run as the core's tests.
- The interface's tests use test hooks of the lower layer that exist only under `#[cfg(test)]`, which is not set for a dependency: `util::pin_now` (and `now_secs`, which reads what it pinned), `completion::LISTED`, `format::FULL_TEXTS`, `update::check::tests::RELEASE` and `ok`. In the core these become `#[cfg(any(test, feature = "testing"))]`, with the data fixtures in a `testing`-gated module (`tabletist_core::fixtures`, run 1 makes it). The root crate's `[dev-dependencies]` turn the feature on. Resolver 3 enables a dev-dependency's features only for targets that need dev-dependencies, so a release build never has them.
- `tests/keys.rs` lists `src/keymap.rs`, `src/keymap/overrides.rs` and `src/typography/fonts.rs` by path, and walks `src/`; `tests/engines.rs` reads `src`, `crates/tabletist-db/src` and `crates/tabletist-mcp/src`. Both need the core's paths.
- `tests/layers.rs` is the subject of its own section below.

### Build script, features and platform code

- `build.rs` compiles the gettext catalogs (`fastframe_i18n::build::compile_catalogs("assets/i18n")`) that `i18n.rs` includes from `OUT_DIR`, and on a Windows host embeds the icon and version details into the exe. The catalogs go with `i18n` into the core's own `build.rs` (`../../assets/i18n`); the icon stays in the root's.
- Features: `shots` is the interface's (it renders test frames). The core gets one, `testing`, above.
- `include_str!`/`include_bytes!` paths are relative to the file: `contrib/omarchy/*.json.tpl` (`desktop_theme`, `env`, `theme`, and `fixtures`, which run 1 made), `assets/update-public-key.hex` (`update`), `tests/fixtures/…` (`connections`, `update/check.rs`), `docs/_reference/settings-and-files.md` (`connections/file.rs`), `assets/fonts/…` (`installed_font.rs`'s tests). Each gains `../../` once it is under `crates/tabletist-core/src/`. Two tests join `CARGO_MANIFEST_DIR` (`env.rs`, `installed_font.rs`), which becomes the core's directory. In `env.rs` that is more than a path (found by run 3): its `sources()` walks `CARGO_MANIFEST_DIR/src`, and four tests read the whole crate through it and name files on both sides (`environment_colours_are_written_only_in_this_module`, `no_view_writes_a_hex_colour`, `environments_are_matched_only_in_this_module`, `only_the_environment_surfaces_ask_for_its_colours`, with `src/ui/value_tags.rs`, `src/ui/workspace.rs` and the rest beside `src/theme.rs` and `src/model.rs`). In the core they would read only the core's `src/` and still pass, the interface unread. The move takes them to the root crate (an integration test under `tests/`, reading `src/` and `crates/tabletist-core/src/` by their paths from the workspace root, as `tests/engines.rs` reads three crates), or gives `sources()` both directories; either way each path they name is the file's new one.
- Platform code moves as it is: `cfg(target_os)`/`cfg(windows)`/`cfg(unix)` in `secrets` (the three keyring stores, `security-framework`, `secret-service`), `paths`, `relay`, `backend` and `backend/agents/listen.rs` (sockets and named pipes), `util`, `history`, `managed`, `settings`, `update`, `connections/file.rs`, `desktop_theme`, `typography/desktop_font.rs` and `installed_font.rs` (fontconfig, CoreText through `tabletist-appkit`). The platform dependencies they use move to the core's `Cargo.toml` under the same `[target.…]` tables. CI tests `--workspace` on Linux, macOS and Windows, so the core is built and tested on each without a change to `ci.yml`.

## The crate

- **Name and place:** `tabletist-core`, `crates/tabletist-core`, `publish = false`, `edition`, `rust-version` and lints from the workspace.
- **Its modules:** every module `LOWER` lists when the move runs, with `typography` gone up and `font_files` and `fixtures` down by then. Its `lib.rs` declares them as the root's does today.
- **Its dependencies:** the subset of the root's that the lower layer names (`tabletist-db`, `tabletist-mcp`, `tokio`, `tokio-util`, `serde`, `serde_json`, `log`, `directories`, `egui`, `fastframe-theme`, `fastframe-i18n` (and as a build dependency), `fastframe-update`, `skrifa`, `notify`, `keyring-core`, the platform stores, `ureq`, `rfd`, `jiff`, `toml`, `tempfile`, `uuid`, `hmac`, `sha2`, `rusqlite`, `zbus`, `secret-service`, `security-framework`, `tabletist-appkit`), each with the root's comment moved beside it and the root's line removed where nothing above names the crate any more. Every version and feature is one the tree builds today, so `Cargo.lock` gains the one `tabletist-core` package and the root's dependency list changes; both by hand, checked with `--locked`.
- **The version:** the root's and the core's both come from `[workspace.package] version` (run 3), so `env!("CARGO_PKG_VERSION")` is the same number in both. `release.yml` reads the `tabletist` package's version through `cargo metadata`, which still gives it. The release steps in `AGENTS.md` bump `[workspace.package]` and run `cargo update --workspace` (run 3), which covers the core's entry once it exists; the `Cargo.lock` diff of a release becomes two lines.
- **The root crate's `lib.rs`:** `pub use tabletist_core::{agents_shown, backend, …};`, one name per lower module, in place of their `pub mod` lines. Nothing under `src/ui/` or `src/app/` changes its imports.

## How `tests/layers.rs` evolves

- **Run 1** extends it to the lower layer's tests: a test module of a lower file names neither `crate::ui`, `crate::app` nor `crate::testing`. The data fixtures it may use are in the lower module `fixtures`.
- **Run 2** moves `typography` from `LOWER` to `ABOVE`, adds the new interface module (`style`) to `ABOVE` and `font_files` to `LOWER`, and adds `typography` and `style` to what the lower layer's code and tests may not name.
- **Run 4, the move,** deletes it. The core crate does not depend on the root crate, so a core module naming `crate::ui` does not compile, and the root's `src/` holds only the modules above. Its rule that a new module goes in one list or the other becomes the choice of crate. `crates/tabletist-core/tests/interface.rs` (above) keeps the core off egui's interface, and `AGENTS.md`'s architecture paragraph says the crate boundary holds the layers.

## Runs

Each run is one PR from `main`, and passes these:

- `cargo fmt --all --check`
- `cargo clippy --locked --workspace --all-targets -- -D warnings`
- `cargo test --locked --workspace --all-targets`
- `RUSTDOCFLAGS='-D warnings' cargo doc --locked --workspace --no-deps`

Each writes its record beside this plan as `2026-…-core-crate-N-<name>.md`, re-surveying first, since `main` moves. No run changes behaviour; a test changes only where it moves or calls what moved.

1. **The lower layer's tests stand on the lower layer** (this PR, below). The data fixtures of `src/testing.rs` (the Bookshop's structures and pages, the statement outcomes, `run_script`, `workspace()`, `zones()`, `UTC`, `fake_lookup` and the rest that take no `App`) move to a new lower module, `src/fixtures.rs`, under `#[cfg(test)]`; `testing` re-exports them, so the interface's tests do not change. The lower layer's tests name `crate::fixtures`. The three tests that call the interface or read `app::RUNNABLE` move up beside what they call, and `typography/fonts.rs`'s two harness tests move to the interface's tests. `tests/layers.rs` reads the lower layer's test modules too.
2. **Text and style go up** (done: `2026-11-02-core-crate-2-style.md`). `theme`'s `install`, `install_with`, `set_read_faces`, `apply`, `apply_to_style`, `text_rendering` and `Icon`, and its tests that need an `egui::Context`, move to a new module above, `src/style.rs`. `typography`'s font reading (`desktop_font.rs`, `installed_font.rs`, `read_font`, `TerminalFaces`) moves down into a new lower module, `src/font_files.rs` (with `src/font_files/`); the backend names it there. `typography` moves to `ABOVE`. The callers of `theme::install`/`apply` (the entrypoint, the harness, `App`) name `style`. Re-survey first: whatever else of `typography` a lower module names must come down with `font_files` or stay out of the core.
3. **What the boundary breaks that `tests/layers.rs` cannot see** (done: `2026-11-02-core-crate-3-boundary.md`). `[workspace.package] version` and `version.workspace = true` in the root (the core takes it at the move), with `AGENTS.md`'s Releasing section and the line of `claude-review.yml`'s prompt that expects a bump's one `Cargo.lock` line (two after the move); `impl TabName` moves into `model`; `model`'s two doc links to `App` become code spans. Small, and independent of run 2.
4. **The move.** `crates/tabletist-core` with its `Cargo.toml`, `build.rs` and `lib.rs`; `git mv` of every lower module, so history follows; the root's `lib.rs` re-exports them; `pub(crate)` becomes `pub` where the compiler asks; the test hooks and `fixtures` take `cfg(any(test, feature = "testing"))`; the root's dev-dependency turns `testing` on; the `include_str!` paths and the two `CARGO_MANIFEST_DIR` tests; `tests/keys.rs` and `tests/engines.rs` read the core's sources; `env.rs`'s four tests that read the whole crate's sources go up or read both crates (above); `tests/layers.rs` goes and `tests/interface.rs` comes; `AGENTS.md`, whose paths to `src/keymap.rs`, `src/settings.rs`, `src/paths.rs` and the rest of the core (the website's rules among them) and to `tests/layers.rs` change, and `claude-review.yml`'s prompt, which names `src/settings.rs` and `src/paths.rs`. It measures again, as the baseline above did, and writes the numbers into its record. If the diff is too big to review as one, it may move the files first with the root's `#[path]` attributes pointing into the new directory (one commit) and make the crate in a second, in the same PR: `main` sees only the finished move.

After run 4, the levers left are the ones the baseline points at: `codegen-units` above 1 for one or both crates (the owner's call, above), and splitting `ui` itself (133,000 lines, more than half of the crate) so that thin LTO's long pole shrinks again.

---

## Run 1: the lower layer's tests stand on the lower layer

**Files:**
- Create: `src/fixtures.rs` (the data fixtures, from `src/testing.rs`, and `tokyo_night` from `theme`'s tests)
- Modify: `src/lib.rs` (`#[cfg(test)] pub mod fixtures;`)
- Modify: `src/testing.rs` (the fixtures go; `pub use crate::fixtures::*;` keeps every `crate::testing::…` above working)
- Modify: `src/edit.rs`, `src/model.rs`, `src/review.rs`, `src/key_picker.rs`, `src/settings.rs` (their tests name `crate::fixtures`)
- Modify: `src/keymap.rs` → `src/app/commands.rs` (`a_menu_key_holds_cmd_or_ctrl_and_is_one_press`)
- Modify: `src/theme.rs` → `src/ui/grid.rs` (`data_is_monospace_on_macos_and_omarchy`) and `src/ui/value_tags.rs` (`value_tags_take_a_themes_rose_and_olive_only_when_they_stand_apart`)
- Modify: `src/typography/fonts.rs` → `src/ui/keyboard_tests.rs` (`macos_draws_the_keys_its_shortcuts_name`, `the_mac_has_a_glyph_for_every_key_the_keymap_writes`)
- Modify: `tests/layers.rs` (`fixtures` in `LOWER`; a test that reads the lower layer's test modules)
- Modify: `AGENTS.md` (the architecture paragraph on the lower layer)
- Modify: `src/env.rs` (`no_view_writes_a_hex_colour` reads every source file outside test modules; `src/fixtures.rs` is test data as a whole, and holds Tokyo Night's seed colours)

### Task 1: the fixtures go down

- [x] Create `src/fixtures.rs`: the free functions, constants and statics of `src/testing.rs` that take no `App`, `Harness` or egui type (`fixture_connection`, `workspace`, `page`, `fixture_structure`, `book_covers_structure`, `book_covers_page`, `zones_shared`, `zones`, `UTC`, `COVERS_MADE`, `stamped_covers_*`, `PUBLISHERS`, `publishers_*`, `keyed_covers_*`, `genres_*`, `GENRES`, `isbns_structure`, `editions_*`, `fake_lookup`, the outcomes from `rows_outcome` to `stopped_before_it_began`, `run_script`), with their doc comments and the imports they need. What stays in `testing`: the `Harness` and its impls, `node`, `bounds`, `labels`, `key`, `release`, `last_sent`, `asked_access`, `leave_edit`, and its own tests.
- [x] Add `tokyo_night` from `theme`'s tests as `pub fn tokyo_night() -> crate::theme::Palette`; `theme`'s tests call `crate::fixtures::tokyo_night()`.
- [x] `src/lib.rs`: `#[cfg(test)] pub mod fixtures;` in its place among the modules, with a doc line.
- [x] `src/testing.rs`: `pub use crate::fixtures::*;` where the fixtures were, so `crate::testing::book_covers_page` and the rest keep working above, and the harness's impls find them.
- [x] The lower layer's tests name `crate::fixtures` where they named `crate::testing` (`edit`, `model`, `review`, `key_picker`, `settings`).
- [x] `cargo test --locked --workspace --all-targets` passes, with the same number of tests as before.

### Task 2: the tests that reach the interface go up

- [x] `keymap`'s `a_menu_key_holds_cmd_or_ctrl_and_is_one_press` moves to `src/app/commands.rs`'s tests, beside `RUNNABLE`, naming `crate::keymap::{…}` for what it took from `super::*`.
- [x] `theme`'s `data_is_monospace_on_macos_and_omarchy` moves to `src/ui/grid.rs`'s tests, beside `data_role`.
- [x] `theme`'s `value_tags_take_a_themes_rose_and_olive_only_when_they_stand_apart` moves to `src/ui/value_tags.rs`'s tests, beside `terminal_slots`, with `crate::fixtures::tokyo_night()`.
- [x] `typography/fonts.rs`'s two harness tests move to `src/ui/keyboard_tests.rs`, which `tests/keys.rs` already lets spell keys (the first test writes `⌘⇧⌫↩`).
- [x] The same number of tests passes as before.

### Task 3: `tests/layers.rs` reads the lower layer's tests

- [x] `fixtures` goes in `LOWER`.
- [x] A new test, `the_lower_layers_tests_name_neither_the_interface_nor_the_harness`: each lower file's test modules (every `#[cfg(test)]` module at the margin, whatever its name: `typography/desktop_font.rs` has three; no lower module has a test file of its own) name no `crate::ui`, `crate::app` or `crate::testing`, read with the same path search as the code, one module deeper. A unit test of the search finds `crate::testing::Harness` in a tests module and passes over `crate::fixtures::page`.
- [x] The new test fails on a lower test that names `crate::testing` (one put back into `edit.rs`'s tests for the check, then taken out), and passes after.
- [x] `AGENTS.md`: the lower layer's tests stand on the lower layer too; their data comes from `src/fixtures.rs`.

### Task 4: the record, the checks and the PR

- [x] Write `2026-11-02-core-crate-1-tests.md`: the survey of what moved, what did not change, what this run teaches.
- [x] `cargo fmt --all --check`
- [x] `cargo clippy --locked --workspace --all-targets -- -D warnings`
- [x] `cargo test --locked --workspace --all-targets`
- [x] `RUSTDOCFLAGS='-D warnings' cargo doc --locked --workspace --no-deps`
- [ ] Push and open the PR.

The net is the moved tests themselves (each still runs, under a new path) and the counts: the workspace runs as many tests before as after.

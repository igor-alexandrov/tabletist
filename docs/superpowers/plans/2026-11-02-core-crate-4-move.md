# Moving the lower layer into a crate of its own, run 4: the move

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** The lower layer leaves the `tabletist` crate for a workspace crate of its own, `tabletist-core`, so an edit to the interface no longer compiles it again, and the crate boundary holds the layers in place of `tests/layers.rs`. This is run 4 of `2026-11-02-core-crate.md`, the last. Behaviour does not change.

**Files:**
- Create: `crates/tabletist-core/Cargo.toml`, `build.rs`, `src/lib.rs`, `tests/interface.rs`; the root's `tests/environments.rs`
- Move (`git mv`): the 35 modules `LOWER` listed, from `src/` to `crates/tabletist-core/src/`, their directories with them
- Delete: `tests/layers.rs`
- Modify: `Cargo.toml`, `Cargo.lock`, `build.rs`, `src/lib.rs`
- Modify: the moved files' `include_str!`/`include_bytes!` paths, `installed_font.rs`'s `CARGO_MANIFEST_DIR`, `env.rs`'s four scans moved to `tests/environments.rs`, the test hooks' `cfg`, eleven items made `pub`, `fixtures.rs` (`pub`, `RELEASE`, two `expect`s)
- Modify: `tests/keys.rs`, `tests/engines.rs`; `src/app/updates.rs`, `src/ui/update_settings_tests.rs` (`RELEASE`'s new path); comments naming old paths in `src/entrypoint.rs`, `src/ui/connect_dialog/choice.rs`, `crates/tabletist-db/tests/postgres.rs`
- Modify: `.github/workflows/ci.yml`, `packaging.yml`, `claude-review.yml`; `packaging/arch/tabletist/PKGBUILD.in`; `AGENTS.md`
- Modify: `docs/superpowers/plans/2026-11-02-core-crate.md` (run 4 done, the plan closed)

## Survey (2026-11-02, `main` at `4395aa5a`)

`LOWER` held 35 modules (53 files), as run 2 left it: `typography` up, `font_files` and `fixtures` down. Each moved whole. The root's `src/` keeps `app`, `entrypoint`, `macos`, `main`, `menu`, `promo`, `shots`, `style`, `testing`, `typography`, `ui` and `lib`, the list `ABOVE` had.

### What the compiler asked

- **`pub`.** Eleven items the root names: `agents_shown::same_columns`, `completion::resolve`, `session::KeptTab`'s `of`, `object`, `sorts` and `bar`, `keymap::clash`, and `model`'s fields `WritePrompt::{changeset, then}` and `ConflictPrompt::{overwrite, kept}`. The plan's other candidates (`history::Entry::read`, `connections::file` and its `Problem` and `line_of`, `managed::read`, `keymap::presses`, `Entry`, `entries`) stay `pub(crate)`: only the core names them.
- **The test hooks.** What the interface's tests use takes `cfg(any(test, feature = "testing"))`: the backend's `sent`, `saves` and `exports` (and the lines that fill them), `Backend::inject`, `util::pin_now` (and the `PINNED_NOW` that `now_secs` reads), `completion::LISTED`, `format::FULL_TEXTS` (and the lines that count), and the module `fixtures`. What only the core's own tests use stays `cfg(test)` (`Backend::wait`, `running`, `saving`, `cancelling`, `watched`, `history`'s `count`, `endpoint::relay_for`, `util::symlink_file`, `edit::NONE_DELETED`), so a build with `testing` has no item it does not use.
- **`update::check::tests::RELEASE`** was a constant inside a test module, which a feature cannot open without compiling the tests. It moves into `fixtures` as `RELEASE`; the check's tests import it, and the interface's two test files name `crate::fixtures::RELEASE`.
- **`fixtures`' two `unwrap`s** (in `tokyo_night`) become `expect`s: under `cfg(any(test, …))` clippy no longer sees a test, and the workspace warns on `unwrap_used`.
- **egui's `serde` feature.** The theme and the settings serialize `Color32`; in the one crate eframe's `persistence` turned it on. The core asks for it itself. Nothing new is built.

### The manifests

- **`tabletist-core`** depends on what the moved code names, each with the root's comment moved beside it: `anyhow` (fastframe-update's errors), `serde`, `serde_json` (with `raw_value`, which `model` uses), `log`, `directories`, `egui` (with `serde`), `skrifa`, `fastframe-theme`, `fastframe-i18n` (and as a build dependency), `fastframe-update`, `tabletist-db`, `tabletist-mcp`, `ureq`, `tokio-util`, `tokio`, `rfd`, `uuid`, `hmac`, `sha2`, `keyring-core`, `tempfile`, `toml`, `notify`, `rusqlite`, `jiff`, and the platform tables as the root had them (`zbus`, the Secret Service store and `secret-service` on Linux; the keychain store, `security-framework` and `tabletist-appkit` on macOS; the Windows store). Its dev-dependency is tokio's `test-util`, for the backend's paused clock. `version.workspace = true`, so `CARGO_PKG_VERSION` is the app's.
- **The root** keeps what the interface and the entrypoint name, and adds `tabletist-core` twice: as a dependency, and as a dev-dependency with `testing`. It loses every dependency only the core names, and the Linux and Windows dependency tables with them. Three move to its dev-dependencies, because only its tests name them: `tokio` (the promo's runtime), `toml` (a promo scenario) and `skrifa` (the bundled fonts' metrics). Its build dependency on `fastframe-i18n` goes with the catalogs.
- **`build.rs`.** The core's compiles the catalogs from `../../assets/i18n`. The root's keeps the Windows icon and version details, and prints `rerun-if-changed=build.rs`: without a line of its own a build script runs again after every edit in the package, and on Linux and macOS it no longer printed any.
- **`Cargo.lock`** gains the `tabletist-core` entry, written by hand after `tabletist-appkit` with the core's dependencies, and the `tabletist` entry's list changes to the root's. Every gate ran with `--locked`; nothing was re-resolved.

### What reads sources by path

- **`env.rs`'s four scans** read both crates, so they move to the root as `tests/environments.rs` (the review on #270: the core's tests stand on the core alone, and a scan of `src/` would fail `cargo test -p tabletist-core` over an interface file). Its `sources()` walks `src/` and `crates/tabletist-core/src/` from the root and names each file by that path, so `src/ui/value_tags.rs` stays what it was and `src/theme.rs` is `crates/tabletist-core/src/theme.rs`; each directory must hold more than 20 files, so neither can drop out unseen. The design's fixed colours come from `env_colors(env, Platform::Native, &Palette::light())`, which in the light are `native_table`'s as they stand, so nothing new is `pub`.
- **`installed_font.rs`'s test** joins `../../assets/fonts` to the core's directory.
- **`tests/keys.rs` and `tests/engines.rs`** walk `crates/tabletist-core/src` beside the rest; `keys.rs`'s `MAY` names the keymap's new paths.
- **`include_str!`/`include_bytes!`** gain `../../`: the Omarchy templates (`desktop_theme`, `env`, `theme`, `fixtures`), `assets/update-public-key.hex`, `tests/fixtures/…` (`connections`, `fixtures`'s `RELEASE`), `docs/_reference/settings-and-files.md`, and the fonts in `installed_font.rs`.

### CI, packaging and the review

What names `src/…` paths, the package or its tests, outside the crates:

| Where | What it did | After the move |
|---|---|---|
| `ci.yml`, `postgres` and `mysql` jobs | `cargo test --locked --lib backend::agents`, the root's lib | would match no test and pass; now `-p tabletist-core --lib backend::agents` (32 tests, run here against the local PostgreSQL and MySQL). The GUI libraries stay: the core's `rfd` builds a Wayland client |
| `ci.yml`, the keyring round trip | `--workspace --lib secrets::tests::native_store_round_trip --exact` | still found, in the core's lib |
| `ci.yml`, the relay on Windows | `--test cli` | `tests/cli.rs` stays in the root |
| `packaging/arch/tabletist/PKGBUILD.in`'s `check()` | `cargo test --release --lib`, the root's lib | would leave the core's tests out; now `--lib -p tabletist -p tabletist-core` |
| `packaging.yml`'s paths | `Cargo.toml`, `build.rs`, `crates/tabletist-appkit/**`, … | adds `crates/tabletist-core/Cargo.toml`, which now holds the keyring stores and `tabletist-appkit` (both lists, which it writes twice) |
| `release.yml`, `packaging.yml` | `cargo metadata`'s `tabletist` package: version and description | unchanged: still the root's |
| `cargo build --release --target …` in both | the default member, the root | unchanged; the binary is the root's |
| `claude-review.yml`'s prompt | the website's sources: `src/ui/keys.rs`, `src/settings.rs`, `src/paths.rs` | the core's `keymap.rs`, `settings.rs`, `paths.rs`. `src/ui/keys.rs` was already wrong: `AGENTS.md` ties the shortcuts page to `BINDINGS` in the keymap |
| `.gitattributes`, `.dockerignore`, `omastore.toml`, `packaging/macos`, `packaging/windows`, `packaging/linux` | | name no source path, nor the package's metadata |

`AGENTS.md`: the architecture paragraph says the crate boundary holds the layers, and names `tests/interface.rs`, the `testing` feature and where a new module goes; the backend, the keymap, `vim`, the website's sources and the updater's paths are the core's; a release's `Cargo.lock` diff is `tabletist`'s and `tabletist-core`'s `version` lines.

### The guard

`crates/tabletist-core/tests/interface.rs`: the core's `Cargo.toml`, in every dependency table, names no `eframe`, `egui-winit`, `egui_glow`, `egui_extras`, `egui_kittest`, `fastframe-text`, `fastframe-fonts` or `fastframe-icons`, read as TOML so a `[dependencies.<crate>]` table, a dotted key and a rename's `package` are found too (the review on #270 found the first reading went line by line and missed them); and no file under its `src/` or `tests/` names `egui::Context`, `egui::Ui…`, `egui::Painter` or `egui::Response`, as a path or an item of a `use egui::{…}` group. A test of the search finds each form and passes over `Color32`, `FontData` and a longer crate name. `cargo tree -p tabletist-core` holds none of those crates, nor `tabletist`.

## Measurements (2026-11-03)

As the baseline: this box, `~/.cargo/bin/cargo`, `--locked`, a target directory of its own in the worktree (`target/measure`, empty before the clean build). "Lower" is `completion::TABLES` 8 → 9 and back, which is now a core edit; "UI" is `ui::grid::GUTTER` 22.0 → 23.0 and back. Each after-an-edit figure is the mean of the change and its revert. Other sessions were building beside it: load average 9 to 13 during the release builds, 20 to 29 during dev and check, against 5 to 8 for the baseline, so the dev and check figures lean slow if anything. Per crate from `--timings`.

| Build | Baseline | Estimate | After the move | Where it goes |
|---|---|---|---|---|
| `cargo build --release`, clean | 191 s | about 150 s | **138 s** | dependencies until 49 s, then the core 52 s; the root's lib from 57 s (once the core's metadata is out) beside it, 33 s; the bin (thin LTO) 34 s |
| `cargo build --release`, after a lower (core) edit | 141 s | about 105 s | **86 s** (86.1, 85.3) | the core 52 s; the root's lib from 6 s, 32 s, beside it; the bin 34 s after the core |
| `cargo build --release`, after a UI edit | 141 s | about 100 s | **66 s** (65.9, 65.8) | the core not compiled; the root's lib 32 s, the bin 34 s |
| `cargo build`, clean | 109 s | | **103 s** | |
| `cargo build`, after a lower (core) edit | 5.0 s | no gain | **5.0 s** (5.0, 5.0) | the core 1.8 s, the root's lib 1.6 s, the bin 1.7 s |
| `cargo build`, after a UI edit | 5.1 s | no gain | **3.4 s** (3.4, 3.3) | the root's lib 1.6 s, the bin 1.6 s |
| `cargo check`, dependencies built | 38 s | | **36 s** | |
| `cargo check`, after a lower (core) edit | 1.9 s | no gain | **2.1 s** (2.1, 2.1) | |
| `cargo check`, after a UI edit | 1.9 s | no gain | **1.1 s** (1.1, 1.1) | |

The stripped binary is 40.9 MB (40,857,264 bytes), against 40.6 MB: a second crate costs 0.6%.

What the numbers say against the plan's estimate:

- **A release build after a UI edit takes 66 s, not 141 s: 53% off,** where the estimate was 30%. The estimate split LLVM's work by lines (a third in the core, two thirds above); it splits about the other way. The core's lib takes 52 s in release, the root's 32 s, though the root has twice the lines. Not measured, but the likely reason: the core is where the backend's async code and its generic dependencies (tokio, the drivers through `tabletist-db`, serde) are monomorphized, while the interface is mostly egui calls in plain functions. Either way an interface edit, the common one, skips the heavier half, and thin LTO's long pole shrinks with the root's module (56 s → 34 s).
- **After a core edit, 86 s: 39% off.** The core's 52 s and the bin's 34 s run one after the other; the root's lib runs beside the core and is never the wait.
- **A clean release build, 138 s instead of 191 s,** more than the 40 s the estimate expected, for the same reason.
- **The dev profile and `check` gain a little after a UI edit (5.1 → 3.4 s, 1.9 → 1.1 s)** and nothing after a core edit, as the estimate said: incremental compilation had most of it already.

## What did not change

- **Every test still runs.** 3,743 `#[test]` and `#[tokio::test]` lines under `src/` and `tests/` before; 3,739 under `src/`, `tests/` and `crates/tabletist-core/` after: `tests/layers.rs`'s seven gone, `tests/interface.rs`'s three new. `cargo test --workspace --all-targets` passes (4,787 tests, 19 ignored, 29 suites); `cargo test -p tabletist-core` builds without the interface and passes 984 unit tests and `interface.rs`'s 3.
- **No function body changed,** apart from `fixtures`' two `expect`s and the source-reading tests' paths. The moved files' history follows them (`git log --follow`).
- **Platform code moved as it was.** The `cfg(target_os)` items and the platform dependency tables moved unchanged, and no `cfg`-gated line changed apart from a test path in `installed_font.rs`, which compiles on every platform, so nothing needed a scratch crate for Windows or macOS. CI builds and tests the core on all three.
- **The version is 0.4.1** for both packages; `cargo metadata` gives the root's to `release.yml`.
- **The release profile** is as it was.

## Where the run differs from the plan

- **One commit, not two.** The diff, with its renames, is about 600 lines each way, most of it `tests/layers.rs` going and the manifests; the `#[path]` route was not needed.
- **`RELEASE` moved into `fixtures`** (above): the plan had `update::check::tests` take the feature, which would compile that module's tests into a `testing` build.
- **CI's integration jobs and the AUR check named the root's lib** for tests that moved, and would have passed with fewer tests or none; the plan's survey had not read the workflows' test filters.
- **`tokio`, `toml` and `skrifa` stay in the root as dev-dependencies,** for the promo and a font test.

## What this run teaches

- **A test filter is a path too.** `cargo test --lib backend::agents` and `cargo test --lib` read like commands, but they name a package by default, and after a move they match nothing and pass. Grep the workflows for `--lib`, `--test` and filters, not only for `src/`.
- **Measure the split; lines do not predict it.** The core is a third of the lines and three fifths of the two libraries' release compile time (52 of 84 s). Which half of a crate is heavy decides what a split buys, and here the heavy half was the one an interface edit no longer touches.

## Tasks

### Task 1: the crate

- [x] `crates/tabletist-core` with its `Cargo.toml`, `build.rs` and `lib.rs`; `git mv` of the 35 modules; the root's `lib.rs` re-exports them.
- [x] The root's `Cargo.toml` and `build.rs`; `Cargo.lock` by hand, `--locked` everywhere.
- [x] `pub` where the compiler asked; the test hooks and `fixtures` under `testing`; the root's dev-dependency.

### Task 2: what reads paths

- [x] The include paths; `env.rs`'s scans to `tests/environments.rs`; `installed_font.rs`'s font path.
- [x] `tests/keys.rs`, `tests/engines.rs`; `tests/layers.rs` goes, `tests/interface.rs` comes.
- [x] CI's integration jobs, the AUR check, `packaging.yml`'s paths, the review prompt, `AGENTS.md`.

### Task 3: the measurements and the checks

- [x] The baseline's builds again, written above.
- [x] `cargo fmt --all --check`
- [x] `cargo clippy --locked --workspace --all-targets -- -D warnings`, and `-p tabletist-core --all-targets` alone
- [x] `cargo test --locked --workspace --all-targets`
- [x] `cargo test -p tabletist-core`
- [x] `RUSTDOCFLAGS='-D warnings' cargo doc --locked --workspace --no-deps`
- [x] Push and open the PR (#270).

## After the plan

The plan is closed. The levers left are the baseline's: `codegen-units` above 1 for one or both crates (the owner's call; the core's 52 s of release LLVM is now the longest single step after a core edit), and splitting `ui`, the bulk of the root's 32 s and of thin LTO's 34 s.

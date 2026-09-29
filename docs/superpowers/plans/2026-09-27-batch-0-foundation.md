# Batch 0: Foundation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A compiling, tested Tabletist skeleton: a native eframe window that follows the Omarchy/OS theme, with a connection tab bar (picker tabs only), settings on disk, keyboard shortcuts for tabs, headless UI tests, and a demo mode that can take screenshots.

**Architecture:** One Cargo workspace whose root package is the app (`tabletist`, lib + bin). The app follows spotifast: views push `Action`s into `app.actions`; `App::apply` reduces them after drawing. fastframe crates supply fonts, text rendering, icons, palettes (with Omarchy following), i18n, and logging. No database code yet.

**Tech Stack:** Rust 1.98 (edition 2024), eframe/egui 0.36 (glow) with the crmne egui/winit forks, fastframe v0.1.7 (text, fonts, icons, theme, i18n, log), clap 4, serde/serde_json, directories 6, image 0.25 (PNG screenshots), tempfile (tests).

**Spec:** `docs/superpowers/specs/2026-09-27-tabletist-design.md`

## Global Constraints

- Edition 2024, `rust-version = "1.98"`, toolchain pinned to `1.98.0` in `rust-toolchain.toml`.
- `unsafe_code = "forbid"` for every crate.
- CI (Linux, macOS, Windows): `cargo fmt --all --check`, `cargo clippy --locked --all-targets -- -D warnings`, `cargo test --locked --all-targets`, `RUSTDOCFLAGS=-D warnings cargo doc --locked --no-deps`.
- UI: `eframe`/`egui` 0.36, `glow` renderer, features `accesskit`, `default_fonts`, `glow`, `persistence`, `wayland`, `x11`. egui and winit come from `crmne/egui` rev `0b431145c22ee27b20b0aff7243d74f4b32e9a79` and `crmne/winit` rev `fc4f39ff0993dd235952b749dd16b856c0194e1f` through `[patch.crates-io]`.
- fastframe crates all at `tag = "v0.1.7"` and moved together: `fastframe-text`, `-fonts`, `-icons`, `-theme`, `-i18n`, `-log`. Not `-tray`, `-shell`, `-update`.
- App-id `dev.tabletist.Tabletist`; binary `tabletist`; Omarchy slug `tabletist`.
- Views emit `Action`s; the reducer (`App::apply`) applies them after drawing. Views never mutate `App` fields other than pushing to `app.actions` and view-local UI state (text being typed).
- The UI thread never waits on the database, network, or disk (theme scanning runs on fastframe's background thread).
- Every dependency gets a comment in `Cargo.toml` saying why it is there.
- Settings JSON is written atomically, loaded with `#[serde(default)]`, and an unreadable file is kept aside as `*.bad`.
- Run `cargo fmt --all` before every commit; code in this plan is not pre-wrapped to rustfmt's width.
- Never use em dashes in code comments, docs, or commit messages.
- Work on `main`, one topic per commit, each commit compiling and passing checks.
- If a listed crate version fails to resolve, use `cargo add <crate>@<major>` to pick the newest compatible release; do not change the APIs used.

## Review Focus

1. **A damaged `settings.json`** (truncated, wrong types): the app must start with defaults, keep the damaged file as `settings.json.bad`, and not crash. Test: Task 2, `damaged_settings_are_kept_aside_and_defaults_used`.
2. **Closing the last connection tab**: the window must never be left with zero tabs (there is nothing to draw). Test: Task 4, `closing_the_last_tab_leaves_a_fresh_picker`.
3. **Closing a tab left of the active one**: the active tab must stay the same tab, not shift to its neighbour. Test: Task 4, `closing_a_tab_before_the_active_one_keeps_the_active_tab`.
4. **Cmd/Ctrl+Shift+W vs Cmd/Ctrl+W**: egui ignores an extra Shift when matching, so the Shift variant must be consumed first or Cmd+Shift+W would never fire distinctly (Cmd+W is reserved for object tabs in batch 3). Test: Task 5, `ctrl_shift_w_closes_the_connection_tab`.
5. **A user palette file that names an unknown colour** or is missing: the theme must fall back to the OS light/dark palette, never panic. Test: Task 3, `an_unknown_custom_theme_falls_back_to_the_system_theme`.

---

## File Structure

```
tabletist/
  Cargo.toml                    workspace + app package, deps, patches, profiles, lints
  clippy.toml                   allow unwrap/expect in tests
  rust-toolchain.toml           1.98.0
  mise.toml                     rust + mbx (optional for contributors)
  build.rs                      compiles i18n catalogs
  AGENTS.md                     rules for coding agents
  .github/workflows/ci.yml      quality + test matrix
  assets/i18n/.gitkeep          PO catalogs go here (English only for now)
  contrib/omarchy/tabletist.json.tpl   Omarchy palette template
  packaging/linux/dev.tabletist.Tabletist.desktop
  src/main.rs                   calls entrypoint::main
  src/lib.rs                    module list
  src/entrypoint.rs             CLI, logging, native options, eframe window, screenshots
  src/paths.rs                  AppDirs: config/state dirs and file names
  src/util.rs                   write_atomic, save_json, load_json
  src/settings.rs               Settings
  src/i18n.rs                   Locale + gettext re-exports
  src/theme.rs                  Palette, style mapping, fonts/icons install, Icon, resolve
  src/model.rs                  ConnTabId, ConnTab, ConnTabContent, PickerState, Action
  src/app.rs                    App state, reducer, frame entry points
  src/ui/mod.rs                 top-level layout
  src/ui/conn_tabs.rs           connection tab bar
  src/ui/picker.rs              picker tab body (empty state for now)
  src/ui/keys.rs                keyboard shortcuts
  src/ui/widgets.rs             icon_button, virtual_rows
  src/testing.rs                #[cfg(test)] headless harness
  tests/cli.rs                  runs the binary with --version / --help
```

---

### Task 1: Workspace scaffold, toolchain, CI, and a binary that prints its version

**Files:**
- Create: `Cargo.toml`, `clippy.toml`, `rust-toolchain.toml`, `mise.toml`, `build.rs`, `assets/i18n/.gitkeep`, `AGENTS.md`, `.github/workflows/ci.yml`, `src/main.rs`, `src/lib.rs`, `src/entrypoint.rs`, `src/i18n.rs`, `tests/cli.rs`
- Modify: `.gitignore`

**Interfaces:**
- Consumes: nothing.
- Produces: `tabletist::entrypoint::main() -> anyhow::Result<()>`; `tabletist::entrypoint::Cli` (clap parser with `verbose`, `demo`, `demo_shot`, `demo_size`); `tabletist::i18n::{Locale, gettext, ngettext, pgettext}` with `Locale::English` only.

- [ ] **Step 1: Write the failing CLI test**

Create `tests/cli.rs`:

```rust
//! Runs the built binary, the way a launcher or a packager would.

use std::process::Command;

fn tabletist() -> Command {
    Command::new(env!("CARGO_BIN_EXE_tabletist"))
}

#[test]
fn version_prints_the_package_version() {
    let output = tabletist().arg("--version").output().expect("run tabletist");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(stdout.trim(), format!("tabletist {}", env!("CARGO_PKG_VERSION")));
}

#[test]
fn demo_flags_require_demo() {
    let output = tabletist()
        .args(["--demo-shot", "out.png"])
        .output()
        .expect("run tabletist");
    assert!(!output.status.success(), "--demo-shot without --demo must be rejected");
}
```

- [ ] **Step 2: Create the workspace manifest**

Create `Cargo.toml`:

```toml
[workspace]
members = ["."]
resolver = "3"

[workspace.package]
edition = "2024"
rust-version = "1.98"

[workspace.lints.rust]
unsafe_code = "forbid"

[workspace.lints.clippy]
unwrap_used = "warn"

[package]
name = "tabletist"
version = "0.1.0"
description = "A native database client"
edition.workspace = true
rust-version.workspace = true
publish = false
default-run = "tabletist"

[lints]
workspace = true

[lib]
name = "tabletist"
path = "src/lib.rs"

[[bin]]
name = "tabletist"
path = "src/main.rs"

[dependencies]
# Errors at the binary edge (startup, eframe) where callers only report them.
anyhow = "1"
# Command-line flags: --verbose and the demo/screenshot flags.
clap = { version = "4.5", features = ["derive"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
log = "0.4"
# Platform config/state directories (XDG on Linux).
directories = "6"
# The whole UI. glow keeps the binary and dependency tree small; a 2D table
# browser does not need wgpu. AccessKit exposes controls to screen readers and
# to the headless tests.
eframe = { version = "0.36", default-features = false, features = ["accesskit", "default_fonts", "glow", "persistence", "wayland", "x11"] }
egui = "0.36"
# SVG loading for the Lucide icons.
egui_extras = { version = "0.36", features = ["svg"] }
# Writes --demo-shot screenshots as PNG.
image = { version = "0.25", default-features = false, features = ["png"] }
# Shared foundation with spotifast (crmne/fastframe). Every crate comes from
# the same tag; move them together.
fastframe-text = { git = "https://github.com/crmne/fastframe", tag = "v0.1.7" }
fastframe-fonts = { git = "https://github.com/crmne/fastframe", tag = "v0.1.7" }
fastframe-icons = { git = "https://github.com/crmne/fastframe", tag = "v0.1.7" }
# Palettes, presets, and following the Omarchy theme live.
fastframe-theme = { git = "https://github.com/crmne/fastframe", tag = "v0.1.7" }
# gettext catalogs compiled at build time (see build.rs).
fastframe-i18n = { git = "https://github.com/crmne/fastframe", tag = "v0.1.7" }
# Log file for bug reports and a panic log without payloads.
fastframe-log = { git = "https://github.com/crmne/fastframe", tag = "v0.1.7" }

[build-dependencies]
fastframe-i18n = { git = "https://github.com/crmne/fastframe", tag = "v0.1.7", features = ["build"] }

[dev-dependencies]
# Throwaway config/state directories for tests.
tempfile = "3"

[profile.release]
lto = "thin"
codegen-units = 1
strip = true
panic = "abort"

# Immediate-mode layout is too slow at opt-level 0 to try the app in
# development. Dependencies are optimized; the app crate stays incremental.
[profile.dev.package."*"]
opt-level = 2

[patch.crates-io]
# crmne/egui apps-0.36, shared with spotifast: bidi shaping, the busy-loop
# fix, and Wayland frame-callback pacing. Every egui crate from one revision.
ecolor = { git = "https://github.com/crmne/egui", rev = "0b431145c22ee27b20b0aff7243d74f4b32e9a79" }
eframe = { git = "https://github.com/crmne/egui", rev = "0b431145c22ee27b20b0aff7243d74f4b32e9a79" }
egui = { git = "https://github.com/crmne/egui", rev = "0b431145c22ee27b20b0aff7243d74f4b32e9a79" }
egui-winit = { git = "https://github.com/crmne/egui", rev = "0b431145c22ee27b20b0aff7243d74f4b32e9a79" }
egui_extras = { git = "https://github.com/crmne/egui", rev = "0b431145c22ee27b20b0aff7243d74f4b32e9a79" }
egui_glow = { git = "https://github.com/crmne/egui", rev = "0b431145c22ee27b20b0aff7243d74f4b32e9a79" }
emath = { git = "https://github.com/crmne/egui", rev = "0b431145c22ee27b20b0aff7243d74f4b32e9a79" }
epaint = { git = "https://github.com/crmne/egui", rev = "0b431145c22ee27b20b0aff7243d74f4b32e9a79" }
epaint_default_fonts = { git = "https://github.com/crmne/egui", rev = "0b431145c22ee27b20b0aff7243d74f4b32e9a79" }
# crmne/winit apps-0.30, used by the eframe fork above.
winit = { git = "https://github.com/crmne/winit", rev = "fc4f39ff0993dd235952b749dd16b856c0194e1f" }
```

Create `clippy.toml`:

```toml
allow-unwrap-in-tests = true
allow-expect-in-tests = true
```

Create `rust-toolchain.toml`:

```toml
# Local builds and CI use the same compiler, clippy and rustfmt. Upgrades are
# deliberate: bump this, then fix new lints in the same commit.
[toolchain]
channel = "1.98.0"
components = ["rustfmt", "clippy"]
```

Create `mise.toml`:

```toml
# Optional: mise users get the pinned toolchain with builds through mbx
# (https://mr-boxington.jdx.dev). Plain rustup and cargo work without it.
[tools]
rust = { version = "1.98.0", components = ["rustfmt", "clippy"], mr_boxington = true }
mr-boxington = "latest"
```

Create `build.rs`:

```rust
//! Compiles the gettext catalogs in assets/i18n into Rust modules.

fn main() {
    fastframe_i18n::build::compile_catalogs("assets/i18n");
}
```

Create an empty `assets/i18n/.gitkeep`.

Replace `.gitignore` with:

```
/target
*.png
!assets/**/*.png
!packaging/**/*.png
```

- [ ] **Step 3: Create the library, i18n module, and entrypoint**

Create `src/lib.rs`:

```rust
//! Tabletist: a native database client.

pub mod entrypoint;
pub mod i18n;
```

Create `src/i18n.rs`:

```rust
//! Bundled gettext catalogs. English is the source language and, for now, the
//! only one. Every user-facing string goes through `gettext` so translations
//! can be added later without touching the views.

pub use fastframe_i18n::{gettext, ngettext, pgettext};

include!(concat!(env!("OUT_DIR"), "/catalogs.rs"));

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Locale {
    #[default]
    English,
}

impl fastframe_i18n::Locale for Locale {
    fn catalog(self) -> Option<&'static dyn fastframe_i18n::Translator> {
        match self {
            Self::English => None,
        }
    }
}
```

Create `src/entrypoint.rs`:

```rust
//! Command line, logging, and the native window.

use std::path::PathBuf;

use clap::Parser;

/// Command-line flags.
#[derive(Debug, Parser)]
#[command(name = "tabletist", version, about = "A native database client")]
pub struct Cli {
    /// Log at debug level.
    #[arg(long)]
    pub verbose: bool,
    /// Run with sample data in a throwaway profile.
    #[arg(long)]
    pub demo: bool,
    /// With --demo: save a screenshot of the settled window to this PNG, then quit.
    #[arg(long, requires = "demo")]
    pub demo_shot: Option<PathBuf>,
    /// With --demo: the window's inner size, as WIDTHxHEIGHT.
    #[arg(long, requires = "demo", value_parser = parse_size)]
    pub demo_size: Option<[f32; 2]>,
}

fn parse_size(text: &str) -> Result<[f32; 2], String> {
    let (width, height) = text
        .split_once('x')
        .ok_or_else(|| format!("expected WIDTHxHEIGHT, got {text:?}"))?;
    let parse = |part: &str| {
        part.trim()
            .parse::<f32>()
            .map_err(|_| format!("not a number: {part:?}"))
    };
    Ok([parse(width)?, parse(height)?])
}

pub fn main() -> anyhow::Result<()> {
    let _cli = Cli::parse();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sizes_parse_as_width_by_height() {
        assert_eq!(parse_size("1280x800"), Ok([1280.0, 800.0]));
        assert!(parse_size("1280").is_err());
        assert!(parse_size("wide x 800").is_err());
    }
}
```

Create `src/main.rs`:

```rust
fn main() -> anyhow::Result<()> {
    tabletist::entrypoint::main()
}
```

- [ ] **Step 4: Run the tests**

Run: `cargo test --locked --all-targets` (first run: drop `--locked`, it creates `Cargo.lock`)
Expected: PASS: `version_prints_the_package_version`, `demo_flags_require_demo`, `sizes_parse_as_width_by_height`.

- [ ] **Step 5: Add AGENTS.md and CI**

Create `AGENTS.md`:

```markdown
# Tabletist agent guide

Tabletist is a small, fast, native database client (PostgreSQL,
MySQL, SQLite) on egui/eframe and fastframe. The design lives in
`docs/superpowers/specs/2026-09-27-tabletist-design.md`; batch plans live in
`docs/superpowers/plans/`.

## Architecture

- `src/ui/` draws views and pushes `Action`s onto `app.actions`. `App::apply`
  in `src/app.rs` applies them after drawing. Do not mutate application state
  from inside a view, apart from text a field is editing.
- Database, network, and disk work runs on the backend runtime
  (`src/backend.rs`), never on the UI thread.
- `crates/tabletist-db` has no UI dependencies.
- Platform code sits behind `cfg`. A fix for one platform keeps Linux, macOS,
  and Windows compiling.
- Settings and state files stay readable, backward compatible, and atomically
  written. Never log passwords, passphrases, or connection URLs with secrets.
- Prefer existing dependencies. Explain every crate in `Cargo.toml`.
- egui and winit come from the crmne forks spotifast uses; fastframe crates
  share one tag. Move each group together.

## Checks

    cargo fmt --all --check
    cargo clippy --locked --all-targets -- -D warnings
    cargo test --locked --all-targets
    RUSTDOCFLAGS='-D warnings' cargo doc --locked --no-deps

Add a focused regression test for every behaviour change. UI behaviour is
tested headlessly through `src/testing.rs` (AccessKit tree + events). Do not
weaken a lint, delete a test, or add an `allow` to make CI green without
saying why the rule does not apply.

## Style

- Never use em dashes. Use a full stop, comma, colon, or parentheses.
- Work on `main`, linear history, one topic per commit, each passing checks.
- Report platform coverage honestly: say when something was only compiled.
```

Create `.github/workflows/ci.yml`:

```yaml
name: CI

on:
  push:
    branches: [main]
  pull_request:
  workflow_dispatch:

permissions:
  contents: read

concurrency:
  group: ci-${{ github.workflow }}-${{ github.event.pull_request.number || github.ref }}
  cancel-in-progress: true

env:
  CARGO_TERM_COLOR: always

jobs:
  quality:
    runs-on: ubuntu-latest
    timeout-minutes: 30
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          components: rustfmt, clippy
      - name: GUI build dependencies
        run: |
          sudo apt-get update
          sudo apt-get install -y libxkbcommon-dev libwayland-dev libgl1-mesa-dev
      - uses: Swatinem/rust-cache@v2
      - run: cargo fmt --all --check
      - run: cargo clippy --locked --all-targets -- -D warnings
      - run: cargo doc --locked --no-deps
        env:
          RUSTDOCFLAGS: -D warnings

  test:
    name: test (${{ matrix.os }})
    runs-on: ${{ matrix.os }}
    timeout-minutes: 45
    strategy:
      fail-fast: false
      matrix:
        os: [ubuntu-latest, macos-latest, windows-latest]
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - name: GUI build dependencies
        if: runner.os == 'Linux'
        run: |
          sudo apt-get update
          sudo apt-get install -y libxkbcommon-dev libwayland-dev libgl1-mesa-dev
      - uses: Swatinem/rust-cache@v2
      - run: cargo test --locked --all-targets
```

(`dtolnay/rust-toolchain@stable` defers to `rust-toolchain.toml`, so CI uses 1.98.0.)

- [ ] **Step 6: Run all checks**

Run: `cargo fmt --all --check && cargo clippy --locked --all-targets -- -D warnings && cargo test --locked --all-targets`
Expected: all pass, no warnings.

- [ ] **Step 7: Commit**

```bash
git add Cargo.toml Cargo.lock clippy.toml rust-toolchain.toml mise.toml build.rs assets AGENTS.md .github .gitignore src tests
git commit -m "Scaffold the Tabletist workspace, toolchain and CI"
```

---

### Task 2: Paths, atomic JSON files, and settings

**Files:**
- Create: `src/paths.rs`, `src/util.rs`, `src/settings.rs`
- Modify: `src/lib.rs`

**Interfaces:**
- Consumes: nothing.
- Produces:
  - `paths::AppDirs { pub config: PathBuf, pub state: PathBuf }` with `AppDirs::discover() -> AppDirs`, `AppDirs::at(root: &Path) -> AppDirs`, and `settings_file()`, `connections_file()`, `known_hosts_file()`, `themes_dir()`, `log_file()`, `panic_log()` (all `-> PathBuf`).
  - `util::write_atomic(path: &Path, bytes: &[u8]) -> std::io::Result<()>`, `util::save_json<T: Serialize>(path: &Path, value: &T) -> std::io::Result<()>`, `util::load_json<T: DeserializeOwned + Default>(path: &Path) -> T`.
  - `settings::Settings { version: u32, page_size: u32, show_system_schemas: bool, custom_theme: Option<String> }` with `Settings::load(path: &Path) -> Settings`, `Settings::save(&self, path: &Path) -> std::io::Result<()>`, `Settings::CURRENT_VERSION: u32 = 1`, `Settings::DEFAULT_PAGE_SIZE: u32 = 300`.

- [ ] **Step 1: Write the failing tests**

Create `src/util.rs` with only the tests first:

```rust
//! Small file helpers shared by the settings and connection stores.

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
    #[serde(default)]
    struct Sample {
        name: String,
        count: u32,
    }

    #[test]
    fn json_round_trips_through_an_atomic_write() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nested").join("sample.json");
        let value = Sample { name: "a".into(), count: 3 };
        save_json(&path, &value).unwrap();
        assert_eq!(load_json::<Sample>(&path), value);
        assert!(!dir.path().join("nested").join("sample.json.tmp").exists());
    }

    #[test]
    fn a_missing_file_loads_the_default() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(load_json::<Sample>(&dir.path().join("none.json")), Sample::default());
    }

    #[test]
    fn a_damaged_file_is_kept_aside_and_the_default_used() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sample.json");
        std::fs::write(&path, b"{\"name\": ").unwrap();
        assert_eq!(load_json::<Sample>(&path), Sample::default());
        assert!(!path.exists());
        assert_eq!(
            std::fs::read(dir.path().join("sample.json.bad")).unwrap(),
            b"{\"name\": "
        );
    }
}
```

Create `src/settings.rs` with only the tests first:

```rust
//! User settings, stored as `settings.json` in the config directory.

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_match_the_spec() {
        let settings = Settings::default();
        assert_eq!(settings.version, Settings::CURRENT_VERSION);
        assert_eq!(settings.page_size, 300);
        assert!(!settings.show_system_schemas);
        assert_eq!(settings.custom_theme, None);
    }

    #[test]
    fn settings_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        let settings = Settings {
            page_size: 500,
            show_system_schemas: true,
            custom_theme: Some("Nord.json".into()),
            ..Settings::default()
        };
        settings.save(&path).unwrap();
        assert_eq!(Settings::load(&path), settings);
    }

    #[test]
    fn older_files_with_missing_and_unknown_fields_still_load() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        std::fs::write(&path, br#"{"page_size": 100, "from_the_future": true}"#).unwrap();
        let settings = Settings::load(&path);
        assert_eq!(settings.page_size, 100);
        assert_eq!(settings.custom_theme, None);
    }

    #[test]
    fn damaged_settings_are_kept_aside_and_defaults_used() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        std::fs::write(&path, br#"{"page_size": "lots"}"#).unwrap();
        assert_eq!(Settings::load(&path), Settings::default());
        assert!(dir.path().join("settings.json.bad").exists());
    }

    #[test]
    fn page_size_is_clamped_to_a_sane_range() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        std::fs::write(&path, br#"{"page_size": 0}"#).unwrap();
        assert_eq!(Settings::load(&path).page_size, Settings::MIN_PAGE_SIZE);
        std::fs::write(&path, br#"{"page_size": 999999999}"#).unwrap();
        assert_eq!(Settings::load(&path).page_size, Settings::MAX_PAGE_SIZE);
    }
}
```

Create `src/paths.rs` with only the tests first:

```rust
//! Where Tabletist keeps its files.

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn files_live_in_their_directories() {
        let dirs = AppDirs::at(std::path::Path::new("/root"));
        assert_eq!(dirs.settings_file(), PathBuf::from("/root/config/settings.json"));
        assert_eq!(dirs.connections_file(), PathBuf::from("/root/config/connections.json"));
        assert_eq!(dirs.known_hosts_file(), PathBuf::from("/root/config/known_hosts.json"));
        assert_eq!(dirs.themes_dir(), PathBuf::from("/root/config/themes"));
        assert_eq!(dirs.log_file(), PathBuf::from("/root/state/tabletist.log"));
        assert_eq!(dirs.panic_log(), PathBuf::from("/root/state/panic.log"));
    }

    #[test]
    fn discovered_directories_are_named_for_the_app() {
        let dirs = AppDirs::discover();
        let config = dirs.config.to_string_lossy().to_lowercase();
        assert!(config.contains("tabletist"), "{config}");
    }
}
```

Add to `src/lib.rs`:

```rust
pub mod paths;
pub mod settings;
pub mod util;
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test --lib`
Expected: FAIL to compile: `cannot find function save_json`, `cannot find type Settings`, `cannot find type AppDirs`.

- [ ] **Step 3: Implement util, paths, settings**

Put above the tests in `src/util.rs`:

```rust
use std::io::Write;
use std::path::{Path, PathBuf};

use serde::Serialize;
use serde::de::DeserializeOwned;

/// `path` with `suffix` appended to its file name (`a.json` -> `a.json.tmp`).
fn with_suffix(path: &Path, suffix: &str) -> PathBuf {
    let mut name = path.as_os_str().to_owned();
    name.push(suffix);
    PathBuf::from(name)
}

/// Writes `bytes` to a temporary file beside `path`, flushes it to disk, then
/// renames it over `path`, so a crash never leaves a half-written file.
/// `std::fs::rename` replaces an existing file on every platform.
pub fn write_atomic(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let temporary = with_suffix(path, ".tmp");
    {
        let mut file = std::fs::File::create(&temporary)?;
        file.write_all(bytes)?;
        file.sync_all()?;
    }
    std::fs::rename(&temporary, path)
}

/// Saves `value` as pretty JSON, atomically.
pub fn save_json<T: Serialize>(path: &Path, value: &T) -> std::io::Result<()> {
    let bytes = serde_json::to_vec_pretty(value).map_err(std::io::Error::other)?;
    write_atomic(path, &bytes)
}

/// Loads JSON from `path`. A missing file gives the default. A file that does
/// not parse is renamed to `<name>.bad` (so the user's data is not lost and the
/// next save does not overwrite it) and the default is used.
pub fn load_json<T: DeserializeOwned + Default>(path: &Path) -> T {
    let bytes = match std::fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return T::default(),
        Err(error) => {
            log::warn!("could not read {}: {error}", path.display());
            return T::default();
        }
    };
    match serde_json::from_slice(&bytes) {
        Ok(value) => value,
        Err(error) => {
            let aside = with_suffix(path, ".bad");
            log::warn!(
                "{} is damaged ({error}); keeping it as {} and using defaults",
                path.display(),
                aside.display()
            );
            if let Err(error) = std::fs::rename(path, &aside) {
                log::warn!("could not keep {} aside: {error}", path.display());
            }
            T::default()
        }
    }
}
```

Put above the tests in `src/paths.rs`:

```rust
use std::path::{Path, PathBuf};

use directories::ProjectDirs;

/// The directories Tabletist writes to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AppDirs {
    /// Settings, saved connections, trusted SSH hosts, themes.
    pub config: PathBuf,
    /// Logs.
    pub state: PathBuf,
}

impl AppDirs {
    /// The platform's conventional directories: `~/.config/tabletist` and
    /// `~/.local/state/tabletist` on Linux.
    pub fn discover() -> Self {
        match ProjectDirs::from("dev", "tabletist", "Tabletist") {
            Some(project) => Self {
                config: project.config_dir().to_path_buf(),
                state: project
                    .state_dir()
                    .map(Path::to_path_buf)
                    .unwrap_or_else(|| project.data_local_dir().to_path_buf()),
            },
            None => Self::at(&std::env::current_dir().unwrap_or_default().join("tabletist")),
        }
    }

    /// Directories under one root, for tests and demo mode.
    pub fn at(root: &Path) -> Self {
        Self {
            config: root.join("config"),
            state: root.join("state"),
        }
    }

    pub fn settings_file(&self) -> PathBuf {
        self.config.join("settings.json")
    }

    pub fn connections_file(&self) -> PathBuf {
        self.config.join("connections.json")
    }

    pub fn known_hosts_file(&self) -> PathBuf {
        self.config.join("known_hosts.json")
    }

    pub fn themes_dir(&self) -> PathBuf {
        self.config.join("themes")
    }

    pub fn log_file(&self) -> PathBuf {
        self.state.join("tabletist.log")
    }

    pub fn panic_log(&self) -> PathBuf {
        self.state.join("panic.log")
    }
}
```

Put above the tests in `src/settings.rs`:

```rust
use std::path::Path;

use serde::{Deserialize, Serialize};

/// Everything the user can set. New fields need a default so older files
/// keep loading; unknown fields (from newer versions) are ignored.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// File format version, for future migrations.
    pub version: u32,
    /// Rows fetched per page in the data grid.
    pub page_size: u32,
    /// Show `pg_catalog`, `information_schema`, `mysql`, `sys` and friends.
    pub show_system_schemas: bool,
    /// A palette file name in the themes directory. `None` follows the
    /// desktop: Omarchy when present, else the OS light/dark setting.
    pub custom_theme: Option<String>,
}

impl Settings {
    pub const CURRENT_VERSION: u32 = 1;
    pub const DEFAULT_PAGE_SIZE: u32 = 300;
    pub const MIN_PAGE_SIZE: u32 = 10;
    pub const MAX_PAGE_SIZE: u32 = 10_000;

    pub fn load(path: &Path) -> Self {
        let mut settings: Settings = crate::util::load_json(path);
        settings.page_size = settings
            .page_size
            .clamp(Self::MIN_PAGE_SIZE, Self::MAX_PAGE_SIZE);
        settings.version = Self::CURRENT_VERSION;
        settings
    }

    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        crate::util::save_json(path, self)
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            version: Self::CURRENT_VERSION,
            page_size: Self::DEFAULT_PAGE_SIZE,
            show_system_schemas: false,
            custom_theme: None,
        }
    }
}
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test --lib`
Expected: PASS (9 new tests).

- [ ] **Step 5: Commit**

```bash
git add src/lib.rs src/util.rs src/paths.rs src/settings.rs
git commit -m "Add app directories, atomic JSON files and settings"
```

---

### Task 3: Theme: palette, Omarchy following, fonts and icons

**Files:**
- Create: `src/theme.rs`, `contrib/omarchy/tabletist.json.tpl`
- Modify: `src/lib.rs`

**Interfaces:**
- Consumes: nothing from earlier tasks.
- Produces:
  - `theme::Palette` (`Copy + PartialEq + Debug + Serialize + Deserialize`) with fields `dark, window, panel, surface, surface_hover, surface_active, outline, text, secondary, dim, accent, accent_hover, on_accent, danger, warning, overlay, shadow` (all `egui::Color32` except `dark: bool`), `Palette::dark()`, `Palette::light()`.
  - `theme::Catalog = fastframe_theme::Catalog<Palette>`, `theme::CustomTheme = fastframe_theme::CustomTheme<Palette>`.
  - `theme::resolve(catalog: &Catalog, custom: Option<&str>, system: Option<egui::Theme>) -> Palette`.
  - `theme::apply(ctx: &egui::Context, palette: &Palette)`.
  - `theme::install(ctx: &egui::Context, system_fallbacks: bool)`: fonts (Inter + desktop hinting), image loaders, icons.
  - `theme::enable_desktop_themes(catalog: &mut Catalog)`.
  - `theme::Icon` enum with variants `Plus, X, Search, RefreshCw, ChevronRight, ChevronDown, ChevronLeft, CircleAlert, CircleCheck, CircleX, Copy, Pencil, Trash2, Eye, Lock, LogOut, Ellipsis, Info, PanelLeft, Settings, Clock, Pin` and `Icon::image(self, tint: Color32, size: f32) -> egui::Image<'static>` (generated by `fastframe_icons::icons!`).
  - Font helpers `theme::regular(size) -> FontId`, `theme::medium(size)`, `theme::semibold(size)`, `theme::mono(size)`.
  - Layout constants `theme::RADIUS: u8 = 6`, `theme::RADIUS_SMALL: u8 = 4`.

- [ ] **Step 1: Write the failing tests**

Create `src/theme.rs` containing only:

```rust
//! Palette, typography, icons, and the mapping onto egui's style.

#[cfg(test)]
mod tests {
    use super::*;
    use fastframe_theme::Palette as _;

    fn nord() -> CustomTheme {
        let mut palette = Palette::dark();
        palette.accent = egui::Color32::from_rgb(0x88, 0xc0, 0xd0);
        CustomTheme {
            filename: "Nord.json".into(),
            palette,
        }
    }

    fn omarchy() -> CustomTheme {
        let mut palette = Palette::light();
        palette.accent = egui::Color32::from_rgb(0x90, 0x7a, 0xa9);
        CustomTheme {
            filename: fastframe_theme::omarchy::FILENAME.into(),
            palette,
        }
    }

    #[test]
    fn every_base_colour_can_be_set_by_name() {
        let mut palette = Palette::dark();
        for name in fastframe_theme::BASE_COLORS {
            assert!(palette.set(name, egui::Color32::RED), "{name} is not settable");
        }
        assert!(!palette.set("no_such_colour", egui::Color32::RED));
    }

    #[test]
    fn without_a_choice_the_os_theme_decides() {
        let catalog = Catalog::preview(vec![nord()], false);
        assert_eq!(resolve(&catalog, None, Some(egui::Theme::Light)), Palette::light());
        assert_eq!(resolve(&catalog, None, Some(egui::Theme::Dark)), Palette::dark());
        assert_eq!(resolve(&catalog, None, None), Palette::dark());
    }

    #[test]
    fn omarchy_wins_over_the_os_theme_when_followed() {
        let catalog = Catalog::preview(vec![omarchy()], true);
        assert_eq!(
            resolve(&catalog, None, Some(egui::Theme::Dark)),
            omarchy().palette
        );
    }

    #[test]
    fn a_chosen_theme_wins_over_everything() {
        let catalog = Catalog::preview(vec![nord(), omarchy()], true);
        assert_eq!(
            resolve(&catalog, Some("Nord.json"), Some(egui::Theme::Light)),
            nord().palette
        );
    }

    #[test]
    fn an_unknown_custom_theme_falls_back_to_the_system_theme() {
        let catalog = Catalog::preview(vec![nord()], false);
        assert_eq!(
            resolve(&catalog, Some("Missing.json"), Some(egui::Theme::Light)),
            Palette::light()
        );
    }

    #[test]
    fn applying_a_palette_sets_egui_visuals() {
        let ctx = egui::Context::default();
        let palette = Palette::light();
        apply(&ctx, &palette);
        let style = ctx.global_style();
        assert_eq!(style.visuals.panel_fill, palette.panel);
        assert!(!style.visuals.dark_mode);
        assert_eq!(style.visuals.override_text_color, Some(palette.text));
    }

    #[test]
    fn the_omarchy_template_names_only_known_colours() {
        let template = include_str!("../contrib/omarchy/tabletist.json.tpl");
        for line in template.lines().filter(|line| line.contains("\": \"{{")) {
            let name = line.trim().trim_start_matches('"');
            let name = &name[..name.find('"').unwrap()];
            if name == "base" {
                continue;
            }
            assert!(
                fastframe_theme::BASE_COLORS.contains(&name),
                "{name} is not a base colour"
            );
        }
    }
}
```

Create `contrib/omarchy/tabletist.json.tpl` (Omarchy renders it with the current theme's colours):

```
{
  "base": "{{ mode }}",
  "colors": {
    "window": "{{ background }}",
    "panel": "{{ mix background foreground 3% }}",
    "surface": "{{ mix background foreground 8% }}",
    "surface_hover": "{{ mix background foreground 12% }}",
    "surface_active": "{{ mix background foreground 18% }}",
    "outline": "{{ mix background foreground 20% }}",
    "text": "{{ foreground }}",
    "secondary": "{{ mix background foreground 70% }}",
    "dim": "{{ mix background foreground 50% }}",
    "accent": "{{ accent }}",
    "accent_hover": "{{ mix accent foreground 15% }}",
    "on_accent": "{{ background }}",
    "danger": "{{ red }}",
    "warning": "{{ yellow }}"
  }
}
```

Add `pub mod theme;` to `src/lib.rs`.

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test --lib theme`
Expected: FAIL to compile: `cannot find type Palette`, `cannot find function resolve`.

- [ ] **Step 3: Implement the theme module**

Put above the tests in `src/theme.rs`:

```rust
use egui::{Color32, CornerRadius, FontId, Stroke, Vec2};

/// A palette file from the themes directory.
pub type CustomTheme = fastframe_theme::CustomTheme<Palette>;
/// The palette files, the shared presets, and Omarchy's live palette.
pub type Catalog = fastframe_theme::Catalog<Palette>;

pub const RADIUS: u8 = 6;
pub const RADIUS_SMALL: u8 = 4;

/// Every colour the interface draws with. The names match fastframe's
/// sixteen base colours so palette files and Omarchy can set them.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Palette {
    pub dark: bool,
    pub window: Color32,
    pub panel: Color32,
    pub surface: Color32,
    pub surface_hover: Color32,
    pub surface_active: Color32,
    pub outline: Color32,
    pub text: Color32,
    pub secondary: Color32,
    pub dim: Color32,
    pub accent: Color32,
    pub accent_hover: Color32,
    pub on_accent: Color32,
    pub danger: Color32,
    pub warning: Color32,
    pub overlay: Color32,
    pub shadow: Color32,
}

impl Palette {
    pub fn dark() -> Self {
        Self {
            dark: true,
            window: Color32::from_rgb(0x12, 0x14, 0x17),
            panel: Color32::from_rgb(0x17, 0x1a, 0x1e),
            surface: Color32::from_rgb(0x1e, 0x22, 0x28),
            surface_hover: Color32::from_rgb(0x27, 0x2c, 0x33),
            surface_active: Color32::from_rgb(0x30, 0x36, 0x3f),
            outline: Color32::from_rgb(0x2b, 0x31, 0x39),
            text: Color32::from_rgb(0xe8, 0xeb, 0xef),
            secondary: Color32::from_rgb(0xa3, 0xab, 0xb6),
            dim: Color32::from_rgb(0x6b, 0x74, 0x80),
            accent: Color32::from_rgb(0x4c, 0x9a, 0xff),
            accent_hover: Color32::from_rgb(0x6d, 0xae, 0xff),
            on_accent: Color32::from_rgb(0x08, 0x10, 0x1c),
            danger: Color32::from_rgb(0xf5, 0x71, 0x7f),
            warning: Color32::from_rgb(0xf2, 0xb8, 0x5c),
            overlay: Color32::from_rgb(0x22, 0x27, 0x2e),
            shadow: Color32::from_black_alpha(140),
        }
    }

    pub fn light() -> Self {
        Self {
            dark: false,
            window: Color32::from_rgb(0xf8, 0xf9, 0xfb),
            panel: Color32::from_rgb(0xff, 0xff, 0xff),
            surface: Color32::from_rgb(0xee, 0xf0, 0xf3),
            surface_hover: Color32::from_rgb(0xe3, 0xe6, 0xeb),
            surface_active: Color32::from_rgb(0xd7, 0xdb, 0xe1),
            outline: Color32::from_rgb(0xdd, 0xe1, 0xe6),
            text: Color32::from_rgb(0x14, 0x17, 0x1a),
            secondary: Color32::from_rgb(0x53, 0x5b, 0x66),
            dim: Color32::from_rgb(0x8b, 0x93, 0x9e),
            accent: Color32::from_rgb(0x1f, 0x6f, 0xe0),
            accent_hover: Color32::from_rgb(0x18, 0x5c, 0xbd),
            on_accent: Color32::WHITE,
            danger: Color32::from_rgb(0xd6, 0x3b, 0x4c),
            warning: Color32::from_rgb(0xb8, 0x7a, 0x14),
            overlay: Color32::from_rgb(0xff, 0xff, 0xff),
            shadow: Color32::from_black_alpha(50),
        }
    }
}

impl fastframe_theme::Palette for Palette {
    fn base(base: fastframe_theme::Base) -> Self {
        match base {
            fastframe_theme::Base::Dark => Self::dark(),
            fastframe_theme::Base::Light => Self::light(),
        }
    }

    fn set(&mut self, name: &str, color: Color32) -> bool {
        match name {
            "window" => self.window = color,
            "panel" => self.panel = color,
            "surface" => self.surface = color,
            "surface_hover" => self.surface_hover = color,
            "surface_active" => self.surface_active = color,
            "outline" => self.outline = color,
            "text" => self.text = color,
            "secondary" => self.secondary = color,
            "dim" => self.dim = color,
            "accent" => self.accent = color,
            "accent_hover" => self.accent_hover = color,
            "on_accent" => self.on_accent = color,
            "danger" => self.danger = color,
            "warning" => self.warning = color,
            "overlay" => self.overlay = color,
            "shadow" => self.shadow = color,
            _ => return false,
        }
        true
    }
}

/// Picks the palette to draw with: the user's chosen file, else Omarchy's
/// live palette when the desktop is Omarchy, else the OS light/dark setting.
pub fn resolve(catalog: &Catalog, custom: Option<&str>, system: Option<egui::Theme>) -> Palette {
    if let Some(theme) = custom.and_then(|name| catalog.find(name)) {
        return theme.palette;
    }
    if catalog.follows_omarchy()
        && let Some(theme) = catalog.system_theme()
    {
        return theme.palette;
    }
    match system {
        Some(egui::Theme::Light) => Palette::light(),
        _ => Palette::dark(),
    }
}

/// Adds the shared presets and, on Omarchy, the desktop's live palette.
pub fn enable_desktop_themes(catalog: &mut Catalog) {
    catalog.enable_desktop_themes(fastframe_theme::DesktopThemes {
        slug: "tabletist",
        omarchy_template: include_str!("../contrib/omarchy/tabletist.json.tpl"),
        omarchy_previous_templates: &[],
        presets: true,
    });
}

pub fn regular(size: f32) -> FontId {
    fastframe_fonts::Weight::Regular.font_id(size)
}

pub fn medium(size: f32) -> FontId {
    fastframe_fonts::Weight::Medium.font_id(size)
}

pub fn semibold(size: f32) -> FontId {
    fastframe_fonts::Weight::SemiBold.font_id(size)
}

pub fn mono(size: f32) -> FontId {
    FontId::monospace(size)
}

/// How the desktop renders text, read once per process. Tests use the
/// platform default so they never wait on D-Bus.
fn text_rendering() -> fastframe_text::TextRendering {
    static RENDERING: std::sync::OnceLock<fastframe_text::TextRendering> =
        std::sync::OnceLock::new();
    *RENDERING.get_or_init(|| {
        if cfg!(test) {
            fastframe_text::TextRendering::platform_default()
        } else {
            fastframe_text::detect()
        }
    })
}

/// Installs fonts, image loaders and icons once per egui context.
/// `system_fallbacks` is off in tests and screenshots for reproducible output.
pub fn install(ctx: &egui::Context, system_fallbacks: bool) {
    let mut fonts = fastframe_fonts::FontSetup::default()
        .system_fallbacks(system_fallbacks)
        .definitions();
    text_rendering().apply_to(&mut fonts);
    ctx.set_fonts(fonts);
    egui_extras::install_image_loaders(ctx);
    fastframe_icons::install::<Icon>(ctx);
}

/// Applies the palette to egui's own widgets and text styles.
pub fn apply(ctx: &egui::Context, palette: &Palette) {
    let mut style = (*ctx.global_style()).clone();
    apply_to_style(&mut style, palette);
    ctx.set_global_style(style);
}

fn apply_to_style(style: &mut egui::Style, palette: &Palette) {
    let visuals = &mut style.visuals;
    *visuals = if palette.dark {
        egui::Visuals::dark()
    } else {
        egui::Visuals::light()
    };
    visuals.dark_mode = palette.dark;
    text_rendering().apply_to_visuals(visuals);
    visuals.panel_fill = palette.panel;
    visuals.window_fill = palette.overlay;
    visuals.extreme_bg_color = palette.surface;
    visuals.faint_bg_color = palette.surface;
    visuals.code_bg_color = palette.surface;
    visuals.override_text_color = Some(palette.text);
    visuals.weak_text_color = Some(palette.secondary);
    visuals.hyperlink_color = palette.accent;
    visuals.selection.bg_fill = palette.accent.gamma_multiply(0.35);
    visuals.selection.stroke = Stroke::new(1.0, palette.accent);
    visuals.window_stroke = Stroke::new(1.0, palette.outline);
    visuals.window_corner_radius = CornerRadius::same(RADIUS + 2);
    visuals.menu_corner_radius = CornerRadius::same(RADIUS);
    visuals.window_shadow = egui::epaint::Shadow {
        offset: [0, 6],
        blur: 24,
        spread: 0,
        color: palette.shadow,
    };
    visuals.popup_shadow = egui::epaint::Shadow {
        offset: [0, 4],
        blur: 16,
        spread: 0,
        color: palette.shadow,
    };
    let corner = CornerRadius::same(RADIUS_SMALL);
    for widget in [
        &mut visuals.widgets.inactive,
        &mut visuals.widgets.hovered,
        &mut visuals.widgets.active,
        &mut visuals.widgets.open,
    ] {
        widget.corner_radius = corner;
        widget.bg_stroke = Stroke::NONE;
        widget.fg_stroke = Stroke::new(1.0, palette.text);
        widget.expansion = 0.0;
    }
    visuals.widgets.noninteractive.corner_radius = corner;
    visuals.widgets.noninteractive.bg_fill = palette.panel;
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0, palette.outline);
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, palette.text);
    visuals.widgets.inactive.bg_fill = palette.surface;
    visuals.widgets.inactive.weak_bg_fill = palette.surface;
    visuals.widgets.hovered.bg_fill = palette.surface_hover;
    visuals.widgets.hovered.weak_bg_fill = palette.surface_hover;
    visuals.widgets.active.bg_fill = palette.surface_active;
    visuals.widgets.active.weak_bg_fill = palette.surface_active;
    visuals.widgets.open.bg_fill = palette.surface_hover;
    visuals.widgets.open.weak_bg_fill = palette.surface_hover;
    visuals.text_cursor.stroke = Stroke::new(2.0, palette.accent);
    visuals.striped = false;

    use egui::FontFamily::{Monospace, Proportional};
    use egui::TextStyle;
    style.text_styles = [
        (TextStyle::Small, FontId::new(11.5, Proportional)),
        (TextStyle::Body, FontId::new(13.5, Proportional)),
        (TextStyle::Button, FontId::new(13.5, Proportional)),
        (TextStyle::Heading, FontId::new(20.0, Proportional)),
        (TextStyle::Monospace, FontId::new(12.5, Monospace)),
    ]
    .into();
    style.spacing.item_spacing = Vec2::new(8.0, 6.0);
    style.spacing.button_padding = Vec2::new(10.0, 5.0);
    style.spacing.interact_size = Vec2::new(40.0, 26.0);
    style.spacing.menu_margin = egui::Margin::same(6);
    style.spacing.window_margin = egui::Margin::same(16);
    style.interaction.selectable_labels = false;
    style.interaction.tooltip_delay = 0.4;
    style.animation_time = 0.12;
}

fastframe_icons::icons! {
    /// Every icon the interface draws. All are shared Lucide icons from
    /// fastframe-icons, so no SVG files live in this repository yet.
    pub enum Icon {
        prefix: "tabletist-icon-",
        directory: "../assets/icons/",
        ChevronDown => lucide "chevron-down",
        ChevronLeft => lucide "chevron-left",
        ChevronRight => lucide "chevron-right",
        CircleAlert => lucide "circle-alert",
        CircleCheck => lucide "circle-check",
        CircleX => lucide "circle-x",
        Clock => lucide "clock",
        Copy => lucide "copy",
        Ellipsis => lucide "ellipsis",
        Eye => lucide "eye",
        Info => lucide "info",
        Lock => lucide "lock",
        LogOut => lucide "log-out",
        PanelLeft => lucide "panel-left",
        Pencil => lucide "pencil",
        Pin => lucide "pin",
        Plus => lucide "plus",
        RefreshCw => lucide "refresh-cw",
        Search => lucide "search",
        Settings => lucide "settings",
        Trash2 => lucide "trash-2",
        X => lucide "x",
    }
}
```

The `icons!` macro's `directory` must exist even when every icon is shared: create `assets/icons/.gitkeep`.

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test --lib theme`
Expected: PASS (7 tests). If `Catalog::preview` or `system_theme()` returns `None` for the preview Omarchy palette, check `fastframe-theme/src/catalog.rs` (`preview` at line ~191): the Omarchy palette is found by its filename `fastframe_theme::omarchy::FILENAME`; keep the test's filename in step with it.

- [ ] **Step 5: Commit**

```bash
git add src/theme.rs src/lib.rs contrib assets/icons
git commit -m "Add the palette, Omarchy template, fonts and icons"
```

---

### Task 4: Model and reducer for connection tabs

**Files:**
- Create: `src/model.rs`, `src/app.rs`
- Modify: `src/lib.rs`

**Interfaces:**
- Consumes: `paths::AppDirs`, `settings::Settings`, `theme::{Palette, Catalog, resolve, apply, install, enable_desktop_themes}`, `i18n::Locale`.
- Produces:
  - `model::ConnTabId(pub u64)` (`Copy, Eq, Hash, Debug`).
  - `model::ConnTab { pub id: ConnTabId, pub content: ConnTabContent }`.
  - `model::ConnTabContent::Picker(PickerState)` (batch 2 adds `Workspace(Box<Workspace>)`).
  - `model::PickerState { pub search: String }` (`Default`).
  - `model::Action` with variants `NewConnTab`, `CloseConnTab(ConnTabId)`, `ActivateConnTab(ConnTabId)`, `ActivateConnTabIndex(usize)`, `CycleConnTab(isize)` (derives `Debug, Clone`).
  - `app::App` with pub fields `dirs: AppDirs, settings: Settings, locale: Locale, palette: Palette, themes: Catalog, tabs: Vec<ConnTab>, active: usize, actions: Vec<Action>`; methods `App::new(dirs: AppDirs, settings: Settings) -> App`, `next_id(&mut self) -> u64`, `active_tab(&self) -> &ConnTab`, `active_tab_id(&self) -> ConnTabId`, `apply(&mut self, action: Action)`, `apply_actions(&mut self)`, `attach(&mut self, ctx: &egui::Context, follow_desktop: bool)`, `logic(&mut self, ctx: &egui::Context)`, `frame_ui(&mut self, ui: &mut egui::Ui)`.

- [ ] **Step 1: Write the failing reducer tests**

Create `src/model.rs`:

```rust
//! Application state types and the actions that change them.

/// Identifies a connection tab for its whole life, whatever its position.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ConnTabId(pub u64);

/// One tab in the connection tab bar: one connection, or the picker that
/// chooses one.
#[derive(Debug)]
pub struct ConnTab {
    pub id: ConnTabId,
    pub content: ConnTabContent,
}

#[derive(Debug)]
pub enum ConnTabContent {
    /// Choose a saved connection (or create one).
    Picker(PickerState),
}

#[derive(Debug, Default)]
pub struct PickerState {
    /// Text typed into the picker's search field.
    pub search: String,
}

/// Everything that changes application state. Views push these; `App::apply`
/// applies them after the frame is drawn.
#[derive(Debug, Clone)]
pub enum Action {
    /// Open a new picker tab and make it active.
    NewConnTab,
    CloseConnTab(ConnTabId),
    ActivateConnTab(ConnTabId),
    /// Cmd/Ctrl+1..9: activate the tab at this position, if there is one.
    ActivateConnTabIndex(usize),
    /// Ctrl+Tab (+1) and Ctrl+Shift+Tab (-1), wrapping around.
    CycleConnTab(isize),
}
```

Create `src/app.rs` with only the tests:

```rust
//! Application state and the reducer.

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Action, ConnTabContent};

    fn app() -> (App, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let app = App::new(AppDirs::at(dir.path()), Settings::default());
        (app, dir)
    }

    fn ids(app: &App) -> Vec<u64> {
        app.tabs.iter().map(|tab| tab.id.0).collect()
    }

    #[test]
    fn a_new_app_has_one_picker_tab() {
        let (app, _dir) = app();
        assert_eq!(app.tabs.len(), 1);
        assert_eq!(app.active, 0);
        assert!(matches!(app.tabs[0].content, ConnTabContent::Picker(_)));
    }

    #[test]
    fn new_tabs_open_at_the_end_and_become_active() {
        let (mut app, _dir) = app();
        app.apply(Action::NewConnTab);
        app.apply(Action::NewConnTab);
        assert_eq!(app.tabs.len(), 3);
        assert_eq!(app.active, 2);
        let unique: std::collections::HashSet<_> = ids(&app).into_iter().collect();
        assert_eq!(unique.len(), 3, "tab ids must be unique");
    }

    #[test]
    fn closing_the_last_tab_leaves_a_fresh_picker() {
        let (mut app, _dir) = app();
        let only = app.tabs[0].id;
        app.apply(Action::CloseConnTab(only));
        assert_eq!(app.tabs.len(), 1);
        assert_ne!(app.tabs[0].id, only);
        assert_eq!(app.active, 0);
    }

    #[test]
    fn closing_a_tab_before_the_active_one_keeps_the_active_tab() {
        let (mut app, _dir) = app();
        app.apply(Action::NewConnTab);
        app.apply(Action::NewConnTab);
        let active = app.active_tab_id();
        let first = app.tabs[0].id;
        app.apply(Action::CloseConnTab(first));
        assert_eq!(app.active_tab_id(), active);
    }

    #[test]
    fn closing_the_active_tab_activates_its_right_neighbour_or_the_new_last() {
        let (mut app, _dir) = app();
        app.apply(Action::NewConnTab);
        app.apply(Action::NewConnTab);
        let [a, b, c] = [app.tabs[0].id, app.tabs[1].id, app.tabs[2].id];
        app.apply(Action::ActivateConnTab(b));
        app.apply(Action::CloseConnTab(b));
        assert_eq!(app.active_tab_id(), c);
        app.apply(Action::CloseConnTab(c));
        assert_eq!(app.active_tab_id(), a);
    }

    #[test]
    fn closing_an_unknown_tab_changes_nothing() {
        let (mut app, _dir) = app();
        let before = ids(&app);
        app.apply(Action::CloseConnTab(crate::model::ConnTabId(9999)));
        assert_eq!(ids(&app), before);
    }

    #[test]
    fn tabs_activate_by_index_and_cycle_with_wrapping() {
        let (mut app, _dir) = app();
        app.apply(Action::NewConnTab);
        app.apply(Action::NewConnTab);
        app.apply(Action::ActivateConnTabIndex(0));
        assert_eq!(app.active, 0);
        app.apply(Action::ActivateConnTabIndex(7));
        assert_eq!(app.active, 0, "an index past the end is ignored");
        app.apply(Action::CycleConnTab(-1));
        assert_eq!(app.active, 2);
        app.apply(Action::CycleConnTab(1));
        assert_eq!(app.active, 0);
    }

    #[test]
    fn queued_actions_are_drained_in_order() {
        let (mut app, _dir) = app();
        app.actions.push(Action::NewConnTab);
        app.actions.push(Action::ActivateConnTabIndex(0));
        app.apply_actions();
        assert!(app.actions.is_empty());
        assert_eq!(app.tabs.len(), 2);
        assert_eq!(app.active, 0);
    }
}
```

Add to `src/lib.rs`:

```rust
pub mod app;
pub mod model;
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test --lib app`
Expected: FAIL to compile: `cannot find type App`.

- [ ] **Step 3: Implement App**

Put above the tests in `src/app.rs`. `frame_ui` calls `crate::ui::keys::handle` and `crate::ui::show`, which Task 5 creates; for this task add a stub `src/ui/mod.rs` so it compiles:

```rust
use crate::i18n::Locale;
use crate::model::{Action, ConnTab, ConnTabContent, ConnTabId, PickerState};
use crate::paths::AppDirs;
use crate::settings::Settings;
use crate::theme::{self, Catalog, Palette};

pub struct App {
    pub dirs: AppDirs,
    pub settings: Settings,
    pub locale: Locale,
    pub palette: Palette,
    pub themes: Catalog,
    pub tabs: Vec<ConnTab>,
    /// Index into `tabs`. Always valid: `tabs` is never empty.
    pub active: usize,
    /// Pushed by views and shortcuts, applied after the frame is drawn.
    pub actions: Vec<Action>,
    /// The OS theme seen last frame, to notice light/dark switches.
    system_theme: Option<egui::Theme>,
    next_id: u64,
}

impl App {
    pub fn new(dirs: AppDirs, settings: Settings) -> Self {
        let mut app = Self {
            dirs,
            settings,
            locale: Locale::default(),
            palette: Palette::dark(),
            themes: Catalog::default(),
            tabs: Vec::new(),
            active: 0,
            actions: Vec::new(),
            system_theme: None,
            next_id: 1,
        };
        let tab = app.picker_tab();
        app.tabs.push(tab);
        app
    }

    /// A fresh id for tabs, sessions and requests. Never reused.
    pub fn next_id(&mut self) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    fn picker_tab(&mut self) -> ConnTab {
        ConnTab {
            id: ConnTabId(self.next_id()),
            content: ConnTabContent::Picker(PickerState::default()),
        }
    }

    pub fn active_tab(&self) -> &ConnTab {
        &self.tabs[self.active]
    }

    pub fn active_tab_id(&self) -> ConnTabId {
        self.active_tab().id
    }

    fn tab_index(&self, id: ConnTabId) -> Option<usize> {
        self.tabs.iter().position(|tab| tab.id == id)
    }

    /// Applies queued actions until none are left (an action may queue more).
    pub fn apply_actions(&mut self) {
        while !self.actions.is_empty() {
            for action in std::mem::take(&mut self.actions) {
                self.apply(action);
            }
        }
    }

    pub fn apply(&mut self, action: Action) {
        match action {
            Action::NewConnTab => {
                let tab = self.picker_tab();
                self.tabs.push(tab);
                self.active = self.tabs.len() - 1;
            }
            Action::CloseConnTab(id) => self.close_tab(id),
            Action::ActivateConnTab(id) => {
                if let Some(index) = self.tab_index(id) {
                    self.active = index;
                }
            }
            Action::ActivateConnTabIndex(index) => {
                if index < self.tabs.len() {
                    self.active = index;
                }
            }
            Action::CycleConnTab(step) => {
                let len = self.tabs.len() as isize;
                self.active = (self.active as isize + step).rem_euclid(len) as usize;
            }
        }
    }

    fn close_tab(&mut self, id: ConnTabId) {
        let Some(index) = self.tab_index(id) else {
            return;
        };
        self.tabs.remove(index);
        if self.tabs.is_empty() {
            let tab = self.picker_tab();
            self.tabs.push(tab);
            self.active = 0;
        } else if index < self.active {
            self.active -= 1;
        } else {
            self.active = self.active.min(self.tabs.len() - 1);
        }
    }

    /// Called once the window exists. `follow_desktop` is false in demo mode
    /// and tests, which must not scan the user's themes or Omarchy.
    pub fn attach(&mut self, ctx: &egui::Context, follow_desktop: bool) {
        theme::install(ctx, follow_desktop);
        if follow_desktop {
            theme::enable_desktop_themes(&mut self.themes);
            let repaint = ctx.clone();
            self.themes.start(
                self.dirs.themes_dir(),
                self.settings.custom_theme.clone(),
                &fastframe_theme::Waker::new(move || repaint.request_repaint()),
            );
        }
        self.system_theme = ctx.system_theme();
        self.palette = self.resolve_palette();
        theme::apply(ctx, &self.palette);
    }

    fn resolve_palette(&self) -> Palette {
        theme::resolve(
            &self.themes,
            self.settings.custom_theme.as_deref(),
            self.system_theme,
        )
    }

    /// Work that does not draw: theme changes on disk or in the OS.
    pub fn logic(&mut self, ctx: &egui::Context) {
        if self.themes.needs_reload() {
            let repaint = ctx.clone();
            self.themes.start(
                self.dirs.themes_dir(),
                self.settings.custom_theme.clone(),
                &fastframe_theme::Waker::new(move || repaint.request_repaint()),
            );
        }
        let scanned = self.themes.poll();
        let system = ctx.system_theme();
        if scanned || system != self.system_theme {
            self.system_theme = system;
            let palette = self.resolve_palette();
            if palette != self.palette {
                self.palette = palette;
                theme::apply(ctx, &palette);
            }
        }
    }

    /// Draws one frame, then applies what the frame asked for.
    pub fn frame_ui(&mut self, ui: &mut egui::Ui) {
        crate::ui::keys::handle(self, ui.ctx());
        crate::ui::show(self, ui);
        self.apply_actions();
    }
}
```

Create the stub `src/ui/mod.rs` (Task 5 replaces it):

```rust
//! The interface.

pub mod keys {
    pub fn handle(_app: &mut crate::app::App, _ctx: &egui::Context) {}
}

pub fn show(_app: &mut crate::app::App, _ui: &mut egui::Ui) {}
```

Add `pub mod ui;` to `src/lib.rs`.

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test --lib app`
Expected: PASS (8 tests).

- [ ] **Step 5: Commit**

```bash
git add src/model.rs src/app.rs src/ui/mod.rs src/lib.rs
git commit -m "Add the app state and connection tab reducer"
```

---

### Task 5: Connection tab bar, picker placeholder, shortcuts, and the headless harness

**Files:**
- Create: `src/testing.rs`, `src/ui/conn_tabs.rs`, `src/ui/picker.rs`, `src/ui/keys.rs`, `src/ui/widgets.rs`
- Modify: `src/ui/mod.rs` (replace stub), `src/lib.rs`

**Interfaces:**
- Consumes: `App`, `Action`, `ConnTabContent`, `theme::{Icon, Palette, semibold, regular}`, `i18n::gettext`.
- Produces:
  - `ui::show(app: &mut App, ui: &mut egui::Ui)`.
  - `ui::keys::handle(app: &mut App, ctx: &egui::Context)`.
  - `ui::conn_tabs::tab_title(app: &App, tab: &ConnTab) -> String`.
  - `ui::widgets::icon_button(ui: &mut egui::Ui, icon: Icon, label: &str, palette: &Palette) -> egui::Response` (accessible label = `label`, hover text = `label`).
  - `ui::widgets::virtual_rows(ui: &mut egui::Ui, count: usize, row_height: f32, row: impl FnMut(&mut egui::Ui, usize))`.
  - `testing::Harness` (`#[cfg(test)]`): `Harness::new() -> Harness`, `Harness::with_size(size: egui::Vec2) -> Harness`, fields `pub app: App`, `pub ctx: egui::Context`; methods `frame(&mut self, events: Vec<egui::Event>) -> egui::accesskit::TreeUpdate`, `settle(&mut self) -> egui::accesskit::TreeUpdate` (two empty frames), `click(&mut self, label: &str)`, `press(&mut self, key: egui::Key, modifiers: egui::Modifiers)`, `has(&mut self, label: &str) -> bool`, and free fns `testing::node(tree, label, role) -> Option<egui::accesskit::NodeId>`, `testing::key(key, modifiers) -> egui::Event`.

- [ ] **Step 1: Write the harness and failing UI tests**

Create `src/testing.rs`:

```rust
//! Headless UI test harness: runs frames without a window and inspects the
//! AccessKit tree, the same tree screen readers see.

use egui::accesskit::{self, NodeId, Role, TreeUpdate};

use crate::app::App;
use crate::paths::AppDirs;
use crate::settings::Settings;

pub struct Harness {
    pub app: App,
    pub ctx: egui::Context,
    pub size: egui::Vec2,
    _dir: tempfile::TempDir,
}

impl Harness {
    pub fn new() -> Self {
        Self::with_size(egui::vec2(1280.0, 800.0))
    }

    pub fn with_size(size: egui::Vec2) -> Self {
        let dir = tempfile::tempdir().expect("temp dir");
        let ctx = egui::Context::default();
        ctx.enable_accesskit();
        let mut app = App::new(AppDirs::at(dir.path()), Settings::default());
        app.attach(&ctx, false);
        Self {
            app,
            ctx,
            size,
            _dir: dir,
        }
    }

    pub fn frame(&mut self, events: Vec<egui::Event>) -> TreeUpdate {
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, self.size)),
            events,
            ..Default::default()
        };
        let app = &mut self.app;
        let mut output = self.ctx.run_ui(input, |ui| app.frame_ui(ui));
        output.textures_delta.clear();
        output
            .platform_output
            .accesskit_update
            .expect("AccessKit is enabled")
    }

    /// Two frames: egui lays out on the first and settles sizes on the second.
    pub fn settle(&mut self) -> TreeUpdate {
        self.frame(Vec::new());
        self.frame(Vec::new())
    }

    /// Clicks the widget labelled `label` through AccessKit: a button if
    /// there is one, else any node with that label (selectable labels and
    /// toggles get other roles).
    pub fn click(&mut self, label: &str) {
        let tree = self.settle();
        let target = node(&tree, label, Role::Button)
            .or_else(|| {
                tree.nodes
                    .iter()
                    .find(|(_, node)| node.label() == Some(label))
                    .map(|(id, _)| *id)
            })
            .unwrap_or_else(|| panic!("nothing labelled {label:?}: {:?}", labels(&tree)));
        self.frame(vec![egui::Event::AccessKitActionRequest(
            accesskit::ActionRequest {
                target_tree: accesskit::TreeId::ROOT,
                target_node: target,
                action: accesskit::Action::Click,
                data: None,
            },
        )]);
        self.settle();
    }

    pub fn press(&mut self, key_: egui::Key, modifiers: egui::Modifiers) {
        self.settle();
        self.frame(vec![key(key_, modifiers)]);
        self.settle();
    }

    pub fn has(&mut self, label: &str) -> bool {
        let tree = self.settle();
        labels(&tree).iter().any(|found| found == label)
    }
}

pub fn node(tree: &TreeUpdate, label: &str, role: Role) -> Option<NodeId> {
    tree.nodes
        .iter()
        .find(|(_, node)| node.label() == Some(label) && node.role() == role)
        .map(|(id, _)| *id)
}

pub fn labels(tree: &TreeUpdate) -> Vec<String> {
    tree.nodes
        .iter()
        .filter_map(|(_, node)| node.label().map(str::to_owned))
        .collect()
}

pub fn key(key: egui::Key, modifiers: egui::Modifiers) -> egui::Event {
    egui::Event::Key {
        key,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers,
    }
}
```

Add to `src/lib.rs`:

```rust
#[cfg(test)]
pub mod testing;
```

Replace `src/ui/mod.rs` with the module list and tests (implementation in Step 3):

```rust
//! The interface. Views read `App` and push `Action`s; they never change
//! state directly.

pub mod conn_tabs;
pub mod keys;
pub mod picker;
pub mod widgets;

#[cfg(test)]
mod tests {
    use crate::testing::Harness;
    use egui::{Key, Modifiers};

    #[test]
    fn the_tab_bar_shows_a_new_tab_and_a_plus_button() {
        let mut harness = Harness::new();
        assert!(harness.has("New tab"));
        assert!(harness.has("New connection tab"));
        assert!(harness.has("Close New tab"));
    }

    #[test]
    fn the_plus_button_opens_a_tab() {
        let mut harness = Harness::new();
        harness.click("New connection tab");
        assert_eq!(harness.app.tabs.len(), 2);
        assert_eq!(harness.app.active, 1);
    }

    #[test]
    fn the_close_button_closes_its_tab() {
        // One tab, so the label "Close New tab" is unambiguous.
        let mut harness = Harness::new();
        let closing = harness.app.tabs[0].id;
        harness.click("Close New tab");
        assert_eq!(harness.app.tabs.len(), 1);
        assert_ne!(harness.app.tabs[0].id, closing);
    }

    #[test]
    fn ctrl_t_opens_a_tab() {
        let mut harness = Harness::new();
        harness.press(Key::T, Modifiers::COMMAND);
        assert_eq!(harness.app.tabs.len(), 2);
    }

    #[test]
    fn ctrl_shift_w_closes_the_connection_tab() {
        let mut harness = Harness::new();
        harness.app.apply(crate::model::Action::NewConnTab);
        let active = harness.app.active_tab_id();
        harness.press(Key::W, Modifiers::COMMAND | Modifiers::SHIFT);
        assert_eq!(harness.app.tabs.len(), 1);
        assert!(harness.app.tabs.iter().all(|tab| tab.id != active));
    }

    #[test]
    fn plain_ctrl_w_does_not_close_the_connection_tab() {
        let mut harness = Harness::new();
        harness.app.apply(crate::model::Action::NewConnTab);
        harness.press(Key::W, Modifiers::COMMAND);
        assert_eq!(harness.app.tabs.len(), 2);
    }

    #[test]
    fn number_shortcuts_and_ctrl_tab_switch_tabs() {
        let mut harness = Harness::new();
        harness.app.apply(crate::model::Action::NewConnTab);
        harness.app.apply(crate::model::Action::NewConnTab);
        harness.press(Key::Num1, Modifiers::COMMAND);
        assert_eq!(harness.app.active, 0);
        harness.press(Key::Tab, Modifiers::CTRL);
        assert_eq!(harness.app.active, 1);
        harness.press(Key::Tab, Modifiers::CTRL | Modifiers::SHIFT);
        assert_eq!(harness.app.active, 0);
    }

    #[test]
    fn the_picker_shows_its_empty_state() {
        let mut harness = Harness::new();
        assert!(harness.has("No saved connections yet"));
    }

    #[test]
    fn the_window_lays_out_at_small_and_large_sizes() {
        for size in [egui::vec2(720.0, 480.0), egui::vec2(2560.0, 1440.0)] {
            let mut harness = Harness::with_size(size);
            let tree = harness.settle();
            assert!(crate::testing::node(&tree, "New connection tab", egui::accesskit::Role::Button).is_some());
        }
    }
}
```

Create `src/ui/widgets.rs` with only its tests first:

```rust
//! Small widgets shared by the views.

#[cfg(test)]
mod tests {
    use super::*;
    use egui::{Rect, pos2, vec2};

    fn run(clip: Rect, f: impl FnMut(&mut egui::Ui)) {
        let ctx = egui::Context::default();
        let mut f = f;
        let _ = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), vec2(400.0, 4000.0))),
                ..Default::default()
            },
            |ui| {
                ui.set_clip_rect(clip);
                f(ui);
            },
        );
    }

    #[test]
    fn virtual_rows_only_build_visible_rows() {
        let mut built = Vec::new();
        run(Rect::from_min_size(pos2(0.0, 0.0), vec2(400.0, 240.0)), |ui| {
            virtual_rows(ui, 10_000, 24.0, |ui, index| {
                built.push(index);
                ui.allocate_exact_size(vec2(ui.available_width(), 24.0), egui::Sense::hover());
            });
        });
        assert!(!built.is_empty());
        assert!(built.len() < 20, "built {} rows", built.len());
        assert_eq!(built[0], 0);
    }

    #[test]
    fn virtual_rows_keep_the_full_height() {
        let mut span = 0.0;
        run(Rect::from_min_max(pos2(0.0, 400.0), pos2(400.0, 600.0)), |ui| {
            let start = ui.cursor().top();
            virtual_rows(ui, 1_000, 24.0, |ui, _| {
                ui.allocate_exact_size(vec2(ui.available_width(), 24.0), egui::Sense::hover());
            });
            span = ui.cursor().top() - start;
        });
        assert!((span - 24_000.0).abs() < 1.0, "span {span}");
    }
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test --lib ui`
Expected: FAIL to compile: missing modules `conn_tabs`, `keys`, `picker`, and `virtual_rows`.

- [ ] **Step 3: Implement the views, widgets and shortcuts**

Add to `src/ui/mod.rs`, between the module list and the tests:

```rust
use egui::Frame;

use crate::app::App;
use crate::model::ConnTabContent;

pub fn show(app: &mut App, ui: &mut egui::Ui) {
    conn_tabs::show(app, ui);
    let fill = app.palette.window;
    egui::CentralPanel::default()
        .frame(Frame::new().fill(fill))
        .show(ui, |ui| {
            if matches!(app.active_tab().content, ConnTabContent::Picker(_)) {
                picker::show(app, ui);
            }
        });
}
```

Put above the tests in `src/ui/widgets.rs`:

```rust
use egui::{Response, Sense, Ui, WidgetInfo, WidgetType, vec2};

use crate::theme::{Icon, Palette};

/// A square icon button. `label` is its accessible name and tooltip, so it
/// is reachable by screen readers and by the headless tests.
pub fn icon_button(ui: &mut Ui, icon: Icon, label: &str, palette: &Palette) -> Response {
    let size = vec2(24.0, 24.0);
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    response.widget_info(|| WidgetInfo::labeled(WidgetType::Button, ui.is_enabled(), label));
    if ui.is_rect_visible(rect) {
        if response.hovered() {
            ui.painter().rect_filled(
                rect,
                egui::CornerRadius::same(crate::theme::RADIUS_SMALL),
                palette.surface_hover,
            );
        }
        let tint = if response.hovered() {
            palette.text
        } else {
            palette.secondary
        };
        icon.image(tint, 16.0)
            .paint_at(ui, egui::Rect::from_center_size(rect.center(), vec2(16.0, 16.0)));
    }
    response.on_hover_text(label)
}

/// Lays out only the rows that intersect the visible area of the enclosing
/// scroll view. Every row must be exactly `row_height` tall. One extra row on
/// each side stays built so Tab can move focus into it.
pub fn virtual_rows(ui: &mut Ui, count: usize, row_height: f32, mut row: impl FnMut(&mut Ui, usize)) {
    if count == 0 {
        return;
    }
    let previous_spacing = ui.spacing().item_spacing;
    ui.spacing_mut().item_spacing.y = 0.0;
    let clip = ui.clip_rect();
    let start_y = ui.cursor().top();
    let width = ui.available_width();
    let first = (((clip.top() - start_y) / row_height).floor().max(0.0) as usize)
        .min(count)
        .saturating_sub(1);
    let last = (((clip.bottom() - start_y) / row_height).ceil().max(0.0) as usize + 1).min(count);
    if first > 0 {
        ui.allocate_space(vec2(width, first as f32 * row_height));
    }
    for index in first..last {
        row(ui, index);
    }
    if last < count {
        ui.allocate_space(vec2(width, (count - last) as f32 * row_height));
    }
    ui.spacing_mut().item_spacing = previous_spacing;
}
```

Create `src/ui/conn_tabs.rs`:

```rust
//! The connection tab bar across the top of the window.

use egui::{
    Align, CornerRadius, Frame, Layout, Margin, Rect, RichText, Sense, Stroke, StrokeKind,
    UiBuilder, WidgetInfo, WidgetType, pos2, vec2,
};

use crate::app::App;
use crate::i18n::gettext;
use crate::model::{Action, ConnTab, ConnTabContent};
use crate::theme::{self, Icon};
use crate::ui::widgets::icon_button;

pub const HEIGHT: f32 = 36.0;
const TAB_WIDTH: f32 = 180.0;

/// The text a tab shows and is announced as.
pub fn tab_title(app: &App, tab: &ConnTab) -> String {
    match &tab.content {
        ConnTabContent::Picker(_) => gettext(app.locale, "New tab").into_owned(),
    }
}

pub fn show(app: &mut App, ui: &mut egui::Ui) {
    let palette = app.palette;
    egui::Panel::top("conn-tabs")
        .exact_size(HEIGHT)
        .resizable(false)
        .show_separator_line(false)
        .frame(Frame::new().fill(palette.panel).inner_margin(Margin::symmetric(6, 4)))
        .show(ui, |ui| {
            egui::ScrollArea::horizontal()
                .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        for index in 0..app.tabs.len() {
                            tab(app, ui, index);
                        }
                        let new_label = gettext(app.locale, "New connection tab");
                        if icon_button(ui, Icon::Plus, &new_label, &palette).clicked() {
                            app.actions.push(Action::NewConnTab);
                        }
                    });
                });
        });
}

/// One tab. The tab's own click area is allocated first and its close button
/// after it, so the button sits on top and receives its own clicks.
fn tab(app: &mut App, ui: &mut egui::Ui, index: usize) {
    let palette = app.palette;
    let id = app.tabs[index].id;
    let title = tab_title(app, &app.tabs[index]);
    let active = index == app.active;
    let (rect, response) = ui.allocate_exact_size(vec2(TAB_WIDTH, HEIGHT - 8.0), Sense::click());
    response.widget_info(|| WidgetInfo::selected(WidgetType::Button, true, active, &title));
    if response.clicked() {
        app.actions.push(Action::ActivateConnTab(id));
    }
    if response.middle_clicked() {
        app.actions.push(Action::CloseConnTab(id));
    }
    let fill = if active {
        palette.window
    } else if response.hovered() {
        palette.surface_hover
    } else {
        palette.panel
    };
    let corner = CornerRadius::same(theme::RADIUS);
    ui.painter().rect_filled(rect, corner, fill);
    if active {
        ui.painter()
            .rect_stroke(rect, corner, Stroke::new(1.0, palette.outline), StrokeKind::Inside);
    }

    let inner = rect.shrink2(vec2(8.0, 0.0));
    let close_rect = Rect::from_center_size(pos2(inner.right() - 12.0, inner.center().y), vec2(24.0, 24.0));
    let mut close_ui = ui.new_child(UiBuilder::new().max_rect(close_rect));
    let close = format!("{} {title}", gettext(app.locale, "Close"));
    if icon_button(&mut close_ui, Icon::X, &close, &palette).clicked() {
        app.actions.push(Action::CloseConnTab(id));
    }

    let label_rect = Rect::from_min_max(inner.min, pos2(close_rect.left() - 4.0, inner.max.y));
    let mut label_ui = ui.new_child(
        UiBuilder::new()
            .max_rect(label_rect)
            .layout(Layout::left_to_right(Align::Center)),
    );
    let color = if active { palette.text } else { palette.secondary };
    label_ui.add(
        egui::Label::new(RichText::new(&title).font(theme::regular(13.0)).color(color))
            .truncate()
            .selectable(false),
    );
}
```

Create `src/ui/picker.rs`:

```rust
//! The body of a picker tab. Batch 2 adds the saved-connection list.

use egui::RichText;

use crate::app::App;
use crate::i18n::gettext;
use crate::theme;

pub fn show(app: &mut App, ui: &mut egui::Ui) {
    let palette = app.palette;
    ui.vertical_centered(|ui| {
        ui.add_space(ui.available_height() * 0.3);
        ui.label(
            RichText::new(gettext(app.locale, "Connections"))
                .font(theme::semibold(20.0))
                .color(palette.text),
        );
        ui.add_space(8.0);
        ui.label(
            RichText::new(gettext(app.locale, "No saved connections yet"))
                .color(palette.secondary),
        );
    });
}
```

Create `src/ui/keys.rs`:

```rust
//! Keyboard shortcuts. Command means Cmd on macOS and Ctrl elsewhere.

use egui::{Key, Modifiers};

use crate::app::App;
use crate::model::Action;

const NUMBERS: [Key; 9] = [
    Key::Num1,
    Key::Num2,
    Key::Num3,
    Key::Num4,
    Key::Num5,
    Key::Num6,
    Key::Num7,
    Key::Num8,
    Key::Num9,
];

pub fn handle(app: &mut App, ctx: &egui::Context) {
    let active = app.active_tab_id();
    let mut actions = Vec::new();
    ctx.input_mut(|input| {
        let mut key = |modifiers: Modifiers, key: Key, action: Action| {
            if input.consume_key(modifiers, key) {
                actions.push(action);
            }
        };
        // egui ignores an extra Shift when matching, so each Shift shortcut
        // is consumed before the plain one it extends.
        key(Modifiers::COMMAND | Modifiers::SHIFT, Key::W, Action::CloseConnTab(active));
        key(Modifiers::CTRL | Modifiers::SHIFT, Key::Tab, Action::CycleConnTab(-1));
        key(Modifiers::CTRL, Key::Tab, Action::CycleConnTab(1));
        key(Modifiers::COMMAND, Key::T, Action::NewConnTab);
        for (index, number) in NUMBERS.into_iter().enumerate() {
            key(Modifiers::COMMAND, number, Action::ActivateConnTabIndex(index));
        }
    });
    app.actions.extend(actions);
}
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test --lib`
Expected: PASS, including the 9 UI tests and 2 widget tests. If `Harness::click` cannot find "Close New tab", check that `icon_button` sets `widget_info` before painting (egui records it per frame).

- [ ] **Step 5: Run clippy**

Run: `cargo clippy --locked --all-targets -- -D warnings`
Expected: no warnings.

- [ ] **Step 6: Commit**

```bash
git add src/testing.rs src/ui src/lib.rs
git commit -m "Add the connection tab bar, picker placeholder and tab shortcuts"
```

---

### Task 6: Native window, logging, demo mode and screenshots

**Files:**
- Modify: `src/entrypoint.rs`
- Create: `packaging/linux/dev.tabletist.Tabletist.desktop`

**Interfaces:**
- Consumes: `App::{new, attach, logic, frame_ui}`, `AppDirs::{discover, at, log_file, panic_log, settings_file}`, `Settings::load`.
- Produces: `entrypoint::APP_ID: &str = "dev.tabletist.Tabletist"`; `entrypoint::native_options(size: Option<[f32; 2]>, persist: bool) -> eframe::NativeOptions`; a running window. Batch 3 extends demo mode through `entrypoint::demo_setup(app: &mut App)` (added here as an empty hook).

- [ ] **Step 1: Write the failing test for native options**

Add to the `tests` module in `src/entrypoint.rs`:

```rust
    #[test]
    fn native_options_carry_the_app_id_and_size() {
        let options = native_options(Some([900.0, 600.0]), false);
        assert_eq!(options.viewport.app_id.as_deref(), Some(APP_ID));
        assert_eq!(options.viewport.inner_size, Some(egui::vec2(900.0, 600.0)));
        assert!(!options.persist_window);
    }

    #[test]
    fn the_desktop_file_names_the_app_id() {
        let desktop = include_str!("../packaging/linux/dev.tabletist.Tabletist.desktop");
        assert!(desktop.contains("StartupWMClass=dev.tabletist.Tabletist"));
        assert!(desktop.contains("Exec=tabletist"));
    }
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test --lib entrypoint`
Expected: FAIL to compile: `cannot find function native_options`, `cannot find value APP_ID`, and the missing desktop file.

- [ ] **Step 3: Implement the window**

Create `packaging/linux/dev.tabletist.Tabletist.desktop`:

```ini
[Desktop Entry]
Type=Application
Name=Tabletist
Comment=Browse PostgreSQL, MySQL and SQLite databases
Exec=tabletist
Icon=dev.tabletist.Tabletist
Terminal=false
Categories=Development;Database;
StartupWMClass=dev.tabletist.Tabletist
```

Replace everything above `#[cfg(test)]` in `src/entrypoint.rs` after `parse_size` with:

```rust
use crate::app::App;
use crate::paths::AppDirs;
use crate::settings::Settings;

/// The Wayland app-id and X11 WM class, matching the `.desktop` file so
/// Hyprland window rules and launchers find the window.
pub const APP_ID: &str = "dev.tabletist.Tabletist";

pub fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    // Demo runs use a throwaway profile: nothing touches the user's files.
    let demo_root = if cli.demo {
        Some(tempfile_root()?)
    } else {
        None
    };
    let dirs = match &demo_root {
        Some(root) => AppDirs::at(root),
        None => AppDirs::discover(),
    };
    let mut logging = fastframe_log::Logging::new("tabletist", env!("CARGO_PKG_VERSION")).filter(
        if cli.verbose {
            "info,tabletist=debug,tabletist_db=debug"
        } else {
            "warn,tabletist=info"
        },
    );
    if !cli.demo {
        logging = logging.file(dirs.log_file()).panic_log(dirs.panic_log());
    }
    logging.init()?;

    let settings = Settings::load(&dirs.settings_file());
    let demo = cli.demo;
    let shot = cli.demo_shot.clone().map(Shot::new);
    eframe::run_native(
        "Tabletist",
        native_options(cli.demo_size, !demo),
        Box::new(move |cc| {
            let mut app = App::new(dirs, settings);
            app.attach(&cc.egui_ctx, !demo);
            if demo {
                demo_setup(&mut app);
            }
            Ok(Box::new(Window { app, shot, demo }))
        }),
    )
    .map_err(|error| anyhow::anyhow!("could not open the window: {error}"))
}

/// A fresh directory under the system temp dir for a demo profile.
fn tempfile_root() -> anyhow::Result<PathBuf> {
    let root = std::env::temp_dir().join(format!("tabletist-demo-{}", std::process::id()));
    std::fs::create_dir_all(&root)?;
    Ok(root)
}

/// Fills a demo profile with sample data. Batch 3 adds the SQLite demo
/// connection here.
pub fn demo_setup(_app: &mut App) {}

pub fn native_options(size: Option<[f32; 2]>, persist: bool) -> eframe::NativeOptions {
    let viewport = egui::ViewportBuilder::default()
        .with_title("Tabletist")
        .with_app_id(APP_ID)
        .with_inner_size(size.unwrap_or([1280.0, 800.0]))
        .with_min_inner_size([720.0, 480.0]);
    eframe::NativeOptions {
        viewport,
        persist_window: persist,
        ..Default::default()
    }
}

/// A pending --demo-shot: when to take it and where to write it.
struct Shot {
    path: PathBuf,
    due: std::time::Instant,
    asked: bool,
}

impl Shot {
    fn new(path: PathBuf) -> Self {
        Self {
            path,
            // Enough frames for fonts, icons and (later) demo queries to land.
            due: std::time::Instant::now() + std::time::Duration::from_millis(1500),
            asked: false,
        }
    }
}

struct Window {
    app: App,
    shot: Option<Shot>,
    demo: bool,
}

impl Window {
    fn drive_shot(&mut self, ctx: &egui::Context) {
        let Some(shot) = self.shot.as_mut() else {
            return;
        };
        ctx.request_repaint();
        if !shot.asked && std::time::Instant::now() >= shot.due {
            ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::default()));
            shot.asked = true;
        }
        let image = ctx.input(|input| {
            input.events.iter().find_map(|event| match event {
                egui::Event::Screenshot { image, .. } => Some(image.clone()),
                _ => None,
            })
        });
        let Some(image) = image else {
            return;
        };
        let [width, height] = [image.size[0] as u32, image.size[1] as u32];
        let pixels: Vec<u8> = image
            .pixels
            .iter()
            .flat_map(|pixel| pixel.to_srgba_unmultiplied())
            .collect();
        match ::image::RgbaImage::from_raw(width, height, pixels) {
            Some(buffer) => match buffer.save(&shot.path) {
                Ok(()) => log::info!("wrote {width}x{height} to {}", shot.path.display()),
                Err(error) => log::error!("could not write {}: {error}", shot.path.display()),
            },
            None => log::error!("the frame buffer did not match {width}x{height}"),
        }
        self.shot = None;
        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
    }
}

impl eframe::App for Window {
    fn persist_egui_memory(&self) -> bool {
        !self.demo
    }

    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.app.logic(ctx);
        self.drive_shot(ctx);
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.app.frame_ui(ui);
    }
}
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test --locked --all-targets`
Expected: PASS (all tests so far).

- [ ] **Step 5: Try the app and take a screenshot**

Run: `cargo run -- --demo --demo-shot /tmp/tabletist-batch0.png --demo-size 1280x800`
Expected: a window opens, shows the tab bar with "New tab" and a plus button, then closes by itself; `/tmp/tabletist-batch0.png` exists and shows it. Open it and check the tab bar and the picker text are readable in the current theme.

Run: `cargo run`
Expected: the window stays open; Ctrl+T adds tabs, Ctrl+Shift+W closes them, closing the last one leaves a fresh "New tab". On Omarchy, switching the Omarchy theme (`omarchy-theme-next`) recolours the window without a restart.

- [ ] **Step 6: Full checks and commit**

Run: `cargo fmt --all --check && cargo clippy --locked --all-targets -- -D warnings && cargo test --locked --all-targets && RUSTDOCFLAGS='-D warnings' cargo doc --locked --no-deps`
Expected: all pass.

```bash
git add src/entrypoint.rs packaging
git commit -m "Open the native window with logging, demo mode and screenshots"
```

---

## Done when

- `cargo run` opens a themed window with a working connection tab bar.
- `cargo run -- --demo --demo-shot out.png` writes a screenshot and exits.
- All checks in Global Constraints pass locally; CI is green on Linux, macOS, Windows (push `main` to verify; report any platform that was only compiled locally).

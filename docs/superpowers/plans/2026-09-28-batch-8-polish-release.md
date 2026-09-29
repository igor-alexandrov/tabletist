# Batch 8: Polish and Release Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make Tabletist ready to hand to other people: clear error and empty states, an accessibility pass, the small deferred fixes from batches 5 to 7, an app identity (MIT license, README, icon, desktop entry), and tag-driven releases that build Linux, Windows and macOS packages and publish to the AUR.

**Architecture:** UI polish stays in the existing reducer and views and is checked two ways: headless AccessKit tests for behaviour, and the `--features shots` renderer (Batch 7 follow-up) for how it looks. Packaging follows spotifast's layout (`packaging/{linux,macos,windows,arch}`) without its Ruby tooling: plain `hdiutil`/`codesign`/`notarytool` for macOS, Inno Setup for Windows, and an Arch container that renders PKGBUILDs and pushes them to the AUR. A `release.yml` workflow runs on `v*` tags; a `packaging.yml` workflow dry-runs the packaging on pull requests that touch it.

**Tech Stack:** GitHub Actions (ubuntu-24.04 and ubuntu-24.04-arm, windows-latest, macos-latest, `archlinux:base-devel` container), Inno Setup 6 (preinstalled on windows-latest), `hdiutil`, `lipo`, `codesign`, `xcrun notarytool`, `rsvg-convert` and ImageMagick (local, for the icon assets).

**Spec:** `docs/superpowers/specs/2026-09-27-tabletist-design.md` (sections 5.11, 7, 8; batch 8 in section 9)

## Decisions (from the user, 2026-09-28)

- License: **MIT**.
- macOS: **ad-hoc signed by default; signed and notarized automatically when Apple secrets exist** (`APPLE_CERTIFICATE_P12`, `APPLE_CERTIFICATE_PASSWORD`, `APPLE_SIGNING_IDENTITY`, `APPLE_ID`, `APPLE_TEAM_ID`, `APPLE_APP_PASSWORD`).
- AUR: **auto-publish on tag** (`tabletist-bin` from release binaries, `tabletist` from source) with `AUR_SSH_KEY`; skipped without the secret.
- Targets: **x86_64 and arm64 everywhere**: Linux x86_64/aarch64 (`.tar.gz`), Windows x64/arm64 (`.zip` and `-setup.exe`), macOS universal (`.dmg`).

## Global Constraints

- Everything from earlier batches' Global Constraints still applies (tests, clippy, fmt, signed commits, no em dashes, one topic per commit on `main`).
- App identity everywhere: app id and window class `dev.tabletist.Tabletist`, bundle id `dev.tabletist.Tabletist`, executable `tabletist` (`tabletist.exe`), display name `Tabletist`.
- Release artifacts are named `tabletist-v<version>-<target-triple>.<ext>` (DMG: `tabletist-v<version>-macos-universal.dmg`) and listed in `checksums.txt` (SHA-256).
- Release builds never restore compiler caches (published binaries build from scratch), as in spotifast.
- Secrets are optional: every signing or publishing step checks its secret and skips with a notice when it is missing; a missing secret never fails a release.
- Windows binaries must not import the MSVC runtime DLLs (static CRT, verified with `dumpbin` as spotifast does).
- UI changes are checked in renders: `cargo test --features shots --lib shots -- --ignored` and read the PNGs in `target/shots/`.

## Review Focus

1. **A connection that fails or drops** (wrong host, refused, lost mid-query): the tab says what happened in words a user understands, and offers Reconnect and Edit connection; nothing is left spinning. Test: Task 1 `a_disconnected_tab_offers_reconnect_and_edit` and the `states` shots.
2. **A database with nothing in it** (no schemas visible, an empty schema, a filter matching no rows): each says so and offers the next step instead of a blank area. Test: Task 1 `an_empty_schema_says_so` and `no_rows_under_a_filter_offers_to_clear_it`.
3. **Screen readers and keyboard users:** every interactive widget in every main screen has an accessible name; text meets contrast minimums in both themes. Test: Task 2 `every_interactive_node_is_named` and `palette_text_meets_contrast`.
4. **A release tag on a repository without any secrets** (a fork, or before the maintainer adds them): the release still publishes Linux, Windows and ad-hoc macOS artifacts; signing, notarization and AUR steps skip with a notice. Test: Task 6 dry run (`workflow_dispatch` with no secrets) and the `packaging.yml` PR run.
5. **Installed apps integrate with the desktop:** the Linux `.desktop` entry passes `desktop-file-validate` and its `StartupWMClass` matches the window's app id (Hyprland rules and docks match it); macOS shows the icon and name; Windows adds a Start menu entry and uninstalls cleanly. Test: Task 4 `the_desktop_entry_matches_the_app_id`, `desktop-file-validate` in CI, and the Task 5 packaging dry run.

---

## File Structure

```
src/ui/workspace.rs            disconnected banner: plain words, Reconnect + Edit connection
src/ui/sidebar.rs              empty schema / no schemas states
src/ui/data_view.rs            "No rows match" + Clear filter
src/app.rs                     deferred fixes (Task 3)
src/ui/filter_bar.rs           focus fixes (Task 3)
src/ui/keys.rs                 help text accuracy, Cmd/Ctrl+R by pane (Task 3)
src/theme.rs                   contrast test
src/shots.rs                   + states scenes
src/ui/mod.rs                  accessibility test over all scenes
LICENSE                        MIT
README.md                      what it is, install, build, shortcuts pointer
assets/icon/tabletist.svg      the icon (source)
packaging/linux/dev.tabletist.Tabletist.desktop     (exists) validated
packaging/linux/dev.tabletist.Tabletist.svg         icon for hicolor/scalable
packaging/macos/icon-1024.png, bundle.sh, Info.plist, dmg.sh
packaging/windows/tabletist.ico, tabletist.iss
packaging/arch/tabletist-bin/PKGBUILD.in, packaging/arch/tabletist/PKGBUILD.in, render.sh
.github/workflows/release.yml  tag -> build all, package, checksum, GitHub release, AUR
.github/workflows/packaging.yml  PR dry run of bundle, DMG, installer, PKGBUILD
AGENTS.md                      how to cut a release
```

---

### Task 1: Error and empty states

**Files:** `src/ui/workspace.rs`, `src/ui/sidebar.rs`, `src/ui/data_view.rs`, `src/shots.rs`, `src/ui/mod.rs` (tests)

**Interfaces:**
- Produces: `pub fn describe_error(error: &tabletist_db::Error) -> String` in `src/ui/format.rs` (a plain-language sentence per error kind; the raw error text stays available as hover text).

- [ ] **Step 1: Failing tests** (in `ui/mod.rs` tests):

```rust
    #[test]
    fn a_disconnected_tab_offers_reconnect_and_edit() {
        let mut harness = Harness::new();
        let tab = harness.connect_fake();
        let session = harness.app.workspace(tab).unwrap().session;
        harness.app.apply(crate::model::Action::Backend(crate::backend::Event::Disconnected {
            session,
            error: tabletist_db::Error::ConnectionLost("server closed the connection".into()),
        }));
        assert!(harness.has("The connection was lost."));
        assert!(harness.has("Reconnect"));
        harness.click("Edit connection");
        assert!(matches!(harness.app.dialog, Some(crate::model::Dialog::Connection(_))));
    }

    #[test]
    fn an_empty_schema_says_so() {
        let (mut harness, tab) = tree_harness();
        harness.app.workspace_mut(tab).unwrap().tree.nodes.get_mut("main").unwrap().objects.value =
            Some(Vec::new());
        assert!(harness.has("No tables or views"));
    }

    #[test]
    fn no_rows_under_a_filter_offers_to_clear_it() {
        let mut harness = Harness::new();
        let tab = harness.connect_fake();
        harness.app.apply(crate::model::Action::OpenObject {
            tab,
            object: tabletist_db::ObjectRef::new("main", "users"),
            kind: tabletist_db::ObjectKind::Table,
            pin: true,
        });
        harness.answer_rows(crate::testing::page(3, false));
        let id = harness.app.workspace(tab).unwrap().active_object.unwrap();
        harness.app.workspace_mut(tab).unwrap().object_tab_mut(id).unwrap().filter.rows =
            vec![crate::model::FilterRow {
                column: "id".into(),
                op: tabletist_db::FilterOp::Eq,
                value: "999".into(),
            }];
        harness.app.apply(crate::model::Action::ApplyFilters { tab, object_tab: id });
        harness.answer_rows(crate::testing::page(0, false));
        assert!(harness.has("No rows match the filter"));
        harness.click("Clear filter");
        assert!(matches!(
            harness.app.backend.sent.last(),
            Some(crate::backend::Command::FetchRows { query, .. }) if query.filters.is_empty()
        ));
    }

    #[test]
    fn errors_are_described_in_plain_words() {
        use tabletist_db::{Error, SshStage};
        for (error, words) in [
            (Error::Connect("Connection refused (os error 111)".into()), "Could not reach the server"),
            (Error::Auth("password authentication failed".into()), "The server refused the login"),
            (Error::Timeout, "The server did not answer in time"),
            (Error::ConnectionLost("eof".into()), "The connection was lost"),
            (Error::Tls("invalid peer certificate".into()), "The secure connection failed"),
            (Error::Ssh { stage: SshStage::Auth, message: "x".into() }, "The SSH login failed"),
        ] {
            assert!(crate::ui::format::describe_error(&error).starts_with(words), "{error:?}");
        }
    }
```


- [ ] **Step 2: Watch them fail** (`cargo test --locked --lib -- disconnected empty_schema no_rows_under described`).

- [ ] **Step 3: Implement.**
  - `format::describe_error`: `Connect` → "Could not reach the server. Check the host and port, and that the server is running."; `Auth` → "The server refused the login. Check the user and password."; `Timeout` → "The server did not answer in time."; `ConnectionLost` → "The connection was lost."; `Tls` → "The secure connection failed. Try another TLS mode, or check the certificate."; `Ssh { stage }` → "The SSH login failed." / "The SSH server could not be reached." / "The SSH server could not reach the database." by stage (host key stages keep their own message); `Query` → its message; others → `error.to_string()`.
  - Workspace banner (`workspace.rs`): `describe_error` in `palette.text`, the raw error as hover text, buttons **Reconnect** and **Edit connection** (`Action::EditConnection(workspace.conn_id)`), the banner in `palette.danger.gamma_multiply(0.12)` with 12/10 padding and `theme::RADIUS` corners.
  - Sidebar: a loaded schema whose object list is empty draws one dim row "No tables or views" under it (in `visible_rows` as a new `TreeNode::Empty(schema)` that is not clickable, or drawn by the sidebar when a schema is expanded with zero objects; pick the one that keeps `visible_rows` tests intact and ledger it). With no schemas at all: "This database has no schemas you can see." centred in the sidebar.
  - Data view: zero rows and filters active: "No rows match the filter" and a **Clear filter** button (`Action::ClearFilters`); zero rows, no filters: "This table is empty".
- [ ] **Step 4: Shots.** Add a `states` scene set to `src/shots.rs`: disconnected tab, empty schema, no rows under a filter, a query error. Render and read them; adjust spacing until they look deliberate (centred, 12 pt gaps, buttons at control height).
- [ ] **Step 5: Run and commit** (`cargo test --locked --workspace --all-targets`, clippy; commit "Explain errors in plain words and give empty states a next step").

---

### Task 2: Accessibility pass

**Files:** `src/ui/mod.rs` (tests), `src/theme.rs` (tests), any view whose widget lacks a name, `src/ui/grid.rs` (cell values for screen readers)

- [ ] **Step 1: Failing tests.**

```rust
    /// Interactive roles a screen reader announces; each must have a name.
    const NAMED: [egui::accesskit::Role; 5] = [
        egui::accesskit::Role::Button,
        egui::accesskit::Role::TextInput,
        egui::accesskit::Role::CheckBox,
        egui::accesskit::Role::ComboBox,
        egui::accesskit::Role::Tab,
    ];

    fn unnamed(tree: &egui::accesskit::TreeUpdate) -> Vec<String> {
        tree.nodes
            .iter()
            .filter(|(_, node)| NAMED.contains(&node.role()))
            .filter(|(_, node)| node.label().is_none_or(str::is_empty)
                && node.value().is_none_or(str::is_empty)
                && node.placeholder().is_none_or(str::is_empty))
            .map(|(id, node)| format!("{:?} {id:?}", node.role()))
            .collect()
    }

    #[test]
    fn every_interactive_node_is_named() {
        // Each main screen: picker with a saved connection, workspace with a
        // table open and the row panel, filter bar open, connection dialog
        // (PostgreSQL with the SSH section open), quick open, help, host key
        // prompt, password prompt.
        for (name, tree) in scenes() {
            let missing = unnamed(&tree);
            assert!(missing.is_empty(), "{name}: {missing:?}");
        }
    }
```

(`scenes()` builds each screen with the harness helpers used by earlier tests and returns `(name, harness.settle())`; put it next to the test.)

In `theme.rs` tests:

```rust
    fn luminance(color: egui::Color32) -> f64 {
        let channel = |c: u8| {
            let c = f64::from(c) / 255.0;
            if c <= 0.039_28 { c / 12.92 } else { ((c + 0.055) / 1.055).powf(2.4) }
        };
        0.2126 * channel(color.r()) + 0.7152 * channel(color.g()) + 0.0722 * channel(color.b())
    }

    fn contrast(a: egui::Color32, b: egui::Color32) -> f64 {
        let (hi, lo) = {
            let (x, y) = (luminance(a), luminance(b));
            if x > y { (x, y) } else { (y, x) }
        };
        (hi + 0.05) / (lo + 0.05)
    }

    #[test]
    fn palette_text_meets_contrast() {
        for palette in [Palette::light(), Palette::dark()] {
            for background in [palette.window, palette.panel, palette.surface] {
                assert!(contrast(palette.text, background) >= 4.5);
                assert!(contrast(palette.secondary, background) >= 4.5);
                // Dim text is decoration (NULL, row numbers): large-text minimum.
                assert!(contrast(palette.dim, background) >= 3.0);
            }
            assert!(contrast(palette.on_accent, palette.accent) >= 4.5);
        }
    }
```

- [ ] **Step 2: Watch them fail**, then fix what they report: give every unnamed widget a name (`widget_info` for painted widgets, `.hint_text`/labels for fields, `on_hover_text` is not a name), and adjust `Palette::light()`/`dark()` colors that miss the ratios (darken `secondary`/`dim` in light, lighten in dark) while keeping the look; re-render shots to confirm.
- [ ] **Step 3: Keyboard reach.** Add a test that Tab from the start of a connected workspace reaches the sidebar filter, the tree, the object tab close button and the footer's Count and paging buttons (walk `Key::Tab` up to 60 times collecting focused node labels); fix any widget that cannot take focus.
- [ ] **Step 4: Run and commit** ("Name every control for screen readers and meet contrast minimums").

---

### Task 3: Deferred fixes from batches 5 to 7

**Files:** `src/app.rs`, `src/ui/filter_bar.rs`, `src/ui/keys.rs`, `src/ui/quick_open.rs`

Each fix gets its test first (RED), then the fix:

1. **Retry applies the bar:** after a failed filtered fetch, `Action::RetryRows` re-applies the filter bar's current conditions (`apply_filters`) when the bar is open, else refetches as today. Test `retry_after_a_failed_filter_uses_the_bar`.
2. **Cmd/Ctrl+R refreshes the tree when the tree has the arrows** (spec 5.10): `keys.rs` sends `Action::RefreshTree(tab)` when `pane == Pane::Tree`, else `Action::Refresh(tab)`. Test `command_r_refreshes_the_tree_from_the_tree`.
3. **End with no tree cursor goes to the last row** (`tree_key`: `(TreeKey::End, None) => Some(last)`, `(TreeKey::Up, None) => Some(last)`). Test `end_without_a_cursor_goes_to_the_bottom`.
4. **Filter bar focus:** reopening focuses the last row with a value field, else the raw WHERE field when shown, else the "+ Condition" button; Cmd/Ctrl+F on an open bar that does not have focus focuses it instead of closing it; Cmd/Ctrl+F in the Structure view switches to the Data view and opens the bar. Tests `reopening_the_bar_focuses_a_field`, `command_f_focuses_an_open_bar`, `command_f_from_structure_opens_the_data_view`.
5. **"+ Condition" after removing every row uses the first column.** Test `adding_a_condition_to_an_empty_bar_uses_the_first_column`.
6. **SSH wording:** Cancel on an SSH prompt sets "The SSH login needs a password or passphrase"; the Test's missing saved SSH secret says "No SSH secret is saved; type it to test". Test `ssh_prompt_messages_name_ssh`.
7. **Quick open does not snap back on scroll:** scroll to the selection only when it changed (remember the last selected index in the dialog state). Test: with 50 results, a wheel scroll event keeps the scroll offset on the next frame (read it from `egui::scroll_area::State::load`).
8. **Help text accuracy:** the tree row lists "Arrows, Home/End, Enter" and the grid row "Arrows, Page Up/Down, Home/End" (two entries instead of one). Update `the_shortcut_table_covers_the_spec_map`.

- [ ] Run the whole suite and commit ("Fix the deferred minors from batches 5 to 7").

---

### Task 4: Identity: license, README, icon, desktop entry

**Files:** `LICENSE`, `README.md`, `assets/icon/tabletist.svg`, `packaging/linux/dev.tabletist.Tabletist.svg`, `packaging/macos/icon-1024.png`, `packaging/windows/tabletist.ico`, `Cargo.toml` (`license = "MIT"`, `repository`, `readme`), `src/entrypoint.rs` (window icon), `src/lib.rs` or `tests/` (desktop entry test)

- [ ] **Step 1: Failing test** (`src/entrypoint.rs` tests):

```rust
    #[test]
    fn the_desktop_entry_matches_the_app_id() {
        let entry = include_str!("../packaging/linux/dev.tabletist.Tabletist.desktop");
        assert!(entry.contains(&format!("StartupWMClass={APP_ID}")));
        assert!(entry.contains(&format!("Icon={APP_ID}")));
        assert!(entry.contains("Exec=tabletist"));
    }

    #[test]
    fn the_window_has_the_app_icon() {
        let options = native_options(None, false);
        assert!(options.viewport.icon.is_some());
    }
```

- [ ] **Step 2: The icon.** `assets/icon/tabletist.svg`, 1024×1024 viewBox: a rounded square (corner radius 22%) in a deep blue gradient (#2563eb to #1e40af), with a white table glyph: a header bar and three body rows, two column rules, stroke width 56, rounded caps, the header filled. Keep it legible at 16 px (check `rsvg-convert -w 16`). Generate:

```bash
rsvg-convert -w 1024 -h 1024 assets/icon/tabletist.svg -o packaging/macos/icon-1024.png
cp assets/icon/tabletist.svg packaging/linux/dev.tabletist.Tabletist.svg
for s in 16 24 32 48 64 128 256; do rsvg-convert -w $s -h $s assets/icon/tabletist.svg -o /tmp/tabletist-$s.png; done
magick /tmp/tabletist-{16,24,32,48,64,128,256}.png packaging/windows/tabletist.ico
rsvg-convert -w 256 -h 256 assets/icon/tabletist.svg -o assets/icon/tabletist-256.png
```

Read the PNGs (16, 32, 256, 1024) to check them. The window icon (`ViewportBuilder::with_icon`) loads `assets/icon/tabletist-256.png` with `eframe::icon_data::from_png_bytes(include_bytes!(...))`.

- [ ] **Step 3: LICENSE (MIT, "Copyright (c) 2026 Igor Alexandrov"), README.md** (one paragraph on what Tabletist is; features list; screenshots from `target/shots` copied to `docs/screenshots/` at 1x; install per platform: AUR `tabletist-bin`, DMG (first launch: right-click, Open, when unsigned), Windows installer; build from source; keyboard: press `?`), Cargo metadata (`license`, `repository = "https://github.com/igor-alexandrov/tabletist"`, `readme`).
- [ ] **Step 4: Desktop entry.** Keep `packaging/linux/dev.tabletist.Tabletist.desktop`; add `Keywords=database;sql;postgres;mysql;sqlite;` and `StartupNotify=true`. CI (`quality` job): `sudo apt-get install -y desktop-file-utils && desktop-file-validate packaging/linux/dev.tabletist.Tabletist.desktop`.
- [ ] **Step 5: Omarchy and Hyprland check (by hand, this machine).** Run the release build with `WAYLAND_DISPLAY=wayland-1` on the current workspace; `hyprctl clients -j` shows `class: dev.tabletist.Tabletist`; switching the Omarchy theme (`omarchy-theme-next`) recolors the running app within a second; ledger what was seen. Document in README how the Omarchy template in `contrib/omarchy/` is used.
- [ ] **Step 6: Run and commit** ("Add the MIT license, README, app icon and desktop entry checks").

---

### Task 5: macOS bundle, DMG and Windows installer

**Files:** `packaging/macos/bundle.sh`, `packaging/macos/Info.plist`, `packaging/macos/dmg.sh`, `packaging/windows/tabletist.iss`, `.github/workflows/packaging.yml`

- [ ] **Step 1: `packaging/macos/Info.plist`** (spotifast's, trimmed): `CFBundleName`/`CFBundleDisplayName` Tabletist, `CFBundleIdentifier` `dev.tabletist.Tabletist`, `CFBundleExecutable` `tabletist`, `CFBundleIconFile` `tabletist`, `CFBundleShortVersionString` `__VERSION__`, `CFBundleVersion` `__BUILD__`, `LSMinimumSystemVersion` 11.0, `LSApplicationCategoryType` `public.app-category.developer-tools`, `NSHighResolutionCapable` true, `NSHumanReadableCopyright` "MIT License".
- [ ] **Step 2: `packaging/macos/bundle.sh <binary> <output.app> <version>`**: spotifast's script with Tabletist names (copy binary, fill the plist, build the `.icns` from `icon-1024.png` with `sips` and `iconutil`, sign with `CODESIGN_IDENTITY` and `--options runtime --timestamp` when set, else ad-hoc `codesign --force --sign -`, then `codesign --verify --strict`).
- [ ] **Step 3: `packaging/macos/dmg.sh <app> <output.dmg>`**: stage the app plus an `Applications` symlink in a temp dir; `hdiutil create -volname Tabletist -srcfolder <dir> -ov -format UDZO <dmg>`; when `CODESIGN_IDENTITY` is set, `codesign --sign` the DMG; when `APPLE_ID`, `APPLE_TEAM_ID` and `APPLE_APP_PASSWORD` are set, `xcrun notarytool submit <dmg> --apple-id … --team-id … --password … --wait` then `xcrun stapler staple <dmg>`; otherwise print "Not notarized: Apple secrets are not set."
- [ ] **Step 4: `packaging/windows/tabletist.iss`**: spotifast's script without the Spotify registry parts: a new fixed `AppId` GUID (generate once, never change), `AppName` Tabletist, `DefaultDirName={localappdata}\Programs\Tabletist`, `PrivilegesRequired=lowest`, `OutputBaseFilename=tabletist-v{#Version}-{#Arch}-pc-windows-msvc-setup`, `SetupIconFile=tabletist.ico`, Start menu entry, optional desktop icon, `LicenseFile=..\..\LICENSE`, `CloseApplications=yes`, launch after install.
- [ ] **Step 5: Static CRT for Windows.** `.cargo/config.toml`: `[target.x86_64-pc-windows-msvc] rustflags = ["-C", "target-feature=+crt-static"]` and the same for `aarch64-pc-windows-msvc`. Confirm the Windows CI build still passes.
- [ ] **Step 6: `packaging.yml`** (on pull requests and pushes that touch `packaging/**` or the workflows, and `workflow_dispatch`):
  - macOS job: build `aarch64-apple-darwin` and `x86_64-apple-darwin`, `lipo` a universal binary, `bundle.sh` with version `0.0.0-dev`, `dmg.sh`, then `hdiutil attach` the DMG and check `Tabletist.app/Contents/MacOS/tabletist` runs `--version` (add a `--version` flag to the CLI if it lacks one; clap's `version` attribute).
  - Windows job: release build, `dumpbin /dependents` shows no `VCRUNTIME`/`MSVCP` import, ISCC builds the installer, then a silent install (`/VERYSILENT /CURRENTUSER`) and `tabletist.exe --version` from the install folder, then the silent uninstaller.
  - Arch job (container `archlinux:base-devel`): render both PKGBUILDs (Task 6 `render.sh`) for a fixture version, `makepkg --printsrcinfo` as a non-root user, and for `tabletist` (source) a full `makepkg -s --noconfirm` build.
- [ ] **Step 7: Run and commit** ("Package the macOS app and DMG and the Windows installer"). The first `packaging.yml` run on the pushed commit is this task's verification; ledger it.

---

### Task 6: Release workflow and AUR publishing

**Files:** `.github/workflows/release.yml`, `packaging/arch/tabletist-bin/PKGBUILD.in`, `packaging/arch/tabletist/PKGBUILD.in`, `packaging/arch/render.sh`, `AGENTS.md`

- [ ] **Step 1: PKGBUILD templates.**
  - `tabletist-bin`: `arch=('x86_64' 'aarch64')`, sources `tabletist-v${pkgver}-${CARCH}-unknown-linux-gnu.tar.gz` from the GitHub release, `depends=('libglvnd' 'libxkbcommon' 'wayland' 'libx11' 'dbus')`, `optdepends=('libxkbcommon-x11: X11 sessions')`, installs the binary, LICENSE, the `.desktop` file and the SVG icon; `provides=('tabletist')`, `conflicts=('tabletist')`.
  - `tabletist` (source): `makedepends=('rust' 'cargo')`, source is the release's source archive, `build()` runs `cargo build --release --locked`, `check()` runs `cargo test --release --locked --lib`, same `package()` installs.
  - `render.sh <template> <version> <sha256…>` fills `@VERSION@`, `@PKGREL@` (1), `@AMD64_SHA256@`, `@ARM64_SHA256@`, `@SOURCE_SHA256@`.
- [ ] **Step 2: `release.yml`** on `push: tags: ["v*"]` and `workflow_dispatch` (input `tag`, for a dry run that uploads artifacts but does not publish):
  - `build` matrix: `ubuntu-24.04` x86_64-unknown-linux-gnu, `ubuntu-24.04-arm` aarch64-unknown-linux-gnu, `windows-latest` x86_64-pc-windows-msvc and aarch64-pc-windows-msvc (cross, `rustup target add`). Linux installs `libxkbcommon-dev libwayland-dev libgl1-mesa-dev`. `cargo build --release --locked --target …`. Windows verifies no MSVC runtime import. Package: `dist/tabletist-<tag>-<target>/` with the binary, README.md, LICENSE, and on Linux `packaging/linux/*`; `.tar.gz` on Linux, `.zip` on Windows; Windows also runs ISCC for the `-setup.exe`.
  - `macos` job: both targets, `lipo`, import the `.p12` into a temporary keychain when `APPLE_CERTIFICATE_P12` is set (`security create-keychain`, `security import`, `security set-key-partition-list`) and export `CODESIGN_IDENTITY`; `bundle.sh`; `dmg.sh` (notarizes when the Apple ID secrets exist); upload `tabletist-<tag>-macos-universal.dmg`.
  - `publish` job (needs build and macos; only on tags): download artifacts, add the source archive (`curl` the tag's tarball) as `tabletist-<tag>-source.tar.gz`, write `checksums.txt` (`sha256sum *`), `softprops/action-gh-release@v2` with all files, `prerelease` when the tag has a `-`.
  - `aur` job (needs publish; tags without `-` only; container `archlinux:base-devel`): skip with a notice when `AUR_SSH_KEY` is empty; else render both PKGBUILDs from `checksums.txt`, `makepkg --printsrcinfo > .SRCINFO` as a non-root user, clone `ssh://aur@aur.archlinux.org/tabletist-bin.git` and `tabletist.git` with the key (`AUR_KNOWN_HOSTS` or `ssh-keyscan aur.archlinux.org`), commit "Update to <version>" and push.
- [ ] **Step 3: AGENTS.md "Releasing"**: bump `version` in `Cargo.toml`, `cargo update -p tabletist`, commit, `git tag -s v0.1.0 -m v0.1.0`, `git push origin v0.1.0`; which secrets enable signing, notarization and AUR; how to dry-run with `workflow_dispatch`.
- [ ] **Step 4: Verify.** Push, then run `release.yml` by hand (`gh workflow run release.yml -f tag=v0.1.0-dry`) with no secrets set: every build, the macOS DMG and the installer succeed; publish and AUR skip because it is not a tag push. Ledger the run. Do **not** push a real tag: tagging the first release is the user's call.
- [ ] **Step 5: Commit** ("Release on tags: Linux, Windows and macOS packages, checksums, AUR").

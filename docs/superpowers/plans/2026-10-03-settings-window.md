# Settings: the window on macOS and Windows Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** In the standard and macOS looks, `Mod+,` opens the Settings window as a sheet: a nav, the General tab's four options as a menu, two segmented controls and a toggle, and a footer that says where the file is and offers Reveal, Export and Reset. The key is listed in the shortcuts. "Settings…" in the macOS app menu was planned here as Task 7 and is built by a plan of its own, `2026-10-04-settings-macos-menu.md`; its work was merged into this plan's pull request, so the two are delivered together.

**Architecture:** Step 3 left everything the window needs but its second layout: `Dialog::Settings`, `OptionId` and `OptionValue`, `Action::SetOption` through `App::change_settings`, and `src/ui/settings/mod.rs` that picks a layout. This step adds `src/ui/settings/sheet.rs` for the looks that are not a terminal's, a `widgets::toggle`, and three footer actions (Reveal on the backend beside the editor's launcher, Export through `Backend::save_bytes`, Reset with a question in place). The gates that kept `Mod+,` and the shortcuts dialog's button to the terminal look go.

**Tech Stack:** Rust 2024, egui (crmne fork, 0.36), tokio. Headless UI tests through `src/testing.rs`; `harness.set_look(Look::standard())` and `Look::macos()` are the two looks this window is drawn in.

**Spec:** `docs/superpowers/specs/2026-10-03-settings-general-design.md`, sections "The window" (macOS and Windows, Opening), "The file's actions" (Reveal, Export) and "Testing". This plan is step 4 of its four and the last.

**Design:** the user's Design canvas, artboard "macOS – Settings, General". It is not in the repository and is not to be added. The values this plan needs are written out under "Design values" below.

---

## Rules of the repository that bite here

- Views push `Action`s; only `App::apply` changes state. Disk and process work runs on the backend.
- Text only through `TextRole`s (`src/typography.rs`); colours only from `Palette`; focus rings only through `src/ui/focus.rs`.
- No em dashes anywhere, in code, comments, tests or docs.
- One topic per commit. Every commit passes all four:
  `~/.cargo/bin/cargo fmt --all --check`,
  `~/.cargo/bin/cargo clippy --locked --workspace --all-targets -- -D warnings`,
  `~/.cargo/bin/cargo test --locked --workspace --all-targets`,
  `RUSTDOCFLAGS='-D warnings' ~/.cargo/bin/cargo doc --locked --workspace --no-deps`.
- `clippy::unwrap_used` warns outside tests.
- Subagents do not commit: they `git add` what the task changed and report. The controller commits, signed.
- Design conformance is never a test. Nothing compares a frame with the artboard, and no design file enters the repository.
- Shots and fixtures use the Bookshop demo data only. Renders stay local.
- Cargo builds use the worktree's own `target/`. Never a target directory under `/tmp` or one shared with another checkout.
- The app's window cannot open in an agent's session. What needs a real window is listed under "By hand" at the end and left to the user.

## Design values

From the artboard, in CSS px, which are egui points. Colours are named by the palette token that holds the artboard's value in the light palette.

| Part | Value |
|---|---|
| Sheet | 1040 wide (or the window less 80), `palette.window` fill, 1 px `palette.border`, radius `look.dialog_radius`, the shadow of `connect_dialog::frame` |
| Nav | 210 wide, `palette.panel`, 1 px `palette.outline` at its right, padding 14 top and bottom, 10 at the sides |
| Nav title | "Settings", `TextRole::StateTitle` (15, semibold), inset 10 more, 10 under it |
| Nav item | 28 tall, radius 5, text inset 8; selected: `palette.selection` fill, `palette.accent_hover` text, `TextRole::UiBodyStrong` |
| Content | padding 22 top, 28 at the sides |
| Title | "General", `TextRole::DialogTitle`; 2 under it the subtitle in `TextRole::UiBody`, `palette.dim`; 6 under that |
| Section label | "Data", `widgets::section_label`, 14 over it, 2 under |
| Row | label column 300, gap 16, 9 over and under its content, 1 px `palette.surface` rule under it |
| Row label | `TextRole::UiBody`, `palette.text`; small print 2 under it in `TextRole::Secondary`, `palette.dim` |
| Control and hint | 12 between them; hint in `TextRole::Secondary`, `palette.dim` (the sample timestamp in `TextRole::MonoSecondary`) |
| Menu | 120 wide, `look.control_height` tall |
| Toggle | 30 by 18, radius 9; knob 14 by 14, 2 from the edge; off track `palette.border.lerp_to_gamma(palette.faint, 0.23)`, on track `palette.accent`, knob `palette.window` |
| Sample tags | "print" and "ebook", `TextRole::ValueTag`, padding 2 by 6, radius 4, 6 between; colours `value_tags::slot_colors(0, ..)` and `slot_colors(1, ..)` |
| Footer | `palette.panel`, 1 px `palette.outline` over it, padding 12 by 28, 12 between its parts; `TextRole::Secondary`, `palette.secondary`; the path in `TextRole::MonoSecondary` |
| Links | `palette.accent` (hover `palette.accent_hover`); Reset to defaults in `palette.danger` |

Texts, exactly:

- Subtitle: "How Tabletist shows data. Changes apply right away."
- Rows per page, small print: "Table view; the SQL editor has its own limit"
- Timestamps: segments "To the second", "Full precision"; hint `2026-01-12 09:14:03` or `2026-01-12 09:14:03.482915`
- Numbers: segments `1,240.50`, `1240.50`; hint "Grouping is display only; copy gives the raw value"
- Value tags, small print: "Colors for enum, CHECK (…IN…) and boolean columns"
- Footer: "Stored in", then the path; links "Reveal in Finder" (macOS), "Show in Explorer" (Windows), "Show in folder" (elsewhere), "Export…", "Reset to defaults"
- Reset question: "Reset every option on this tab?" with "Reset" and "Cancel"

## File structure

| File | What changes |
|---|---|
| `src/backend.rs` | `Command::RevealSettingsFile`, `reveal_command`, the launcher shared with the editor, `Opened` in `Event::SettingsFileOpened`; `save_bytes` takes its dialog's title |
| `src/model.rs` | `SettingsDialog::resetting`; actions `RevealSettingsFile`, `ExportSettings`, `ResetSettings`, `ConfirmResetSettings(bool)` |
| `src/app.rs` | the four actions; the notice for a failed reveal |
| `src/ui/widgets.rs` | `toggle` |
| `src/ui/settings/mod.rs` | picks the layout by the look; the rows' small print and hints shared by both; the path as it is shown |
| `src/ui/settings/sheet.rs` | new: the window in the standard and macOS looks |
| `src/ui/keys.rs` | `Mod+,` in every look; its line in `SHORTCUTS` |
| `src/ui/help.rs` | the Settings button in every look |
| `src/ui/mod.rs` | the window's tests |
| `src/shots.rs` | scene `MacSettings` |
| `docs/superpowers/specs/2026-10-03-settings-general-design.md` | what this step learned |

---

### Task 1: Reveal the settings file from the backend

The editor's launcher becomes a launcher of any program, and Reveal is the second program it starts. Both share "write the file first when it is not there" (`open_in_editor`), which already records that write as the backend's own.

**Files:**
- Modify: `src/backend.rs` (`Command`, `Event`, `open_in_editor`'s callers, `start_editor`, the worker's match, the four exhaustive `Command` matches near lines 1662, 1686, 1756 and 1992)
- Modify: `src/app.rs` (the `Event::SettingsFileOpened` arm)
- Test: `src/backend.rs` (`mod tests`), `src/app.rs` (`mod tests`)

- [ ] **Step 1: Write the failing tests**

In `src/backend.rs`, beside `the_editor_is_the_systems_way_to_open_a_text_file`:

```rust
#[test]
fn reveal_shows_the_file_where_it_is_kept() {
    let path = std::path::Path::new("/config/settings.toml");
    let (program, args) = reveal_command(path);
    #[cfg(target_os = "macos")]
    assert_eq!(
        (program.as_str(), args),
        ("open", vec!["-R".into(), path.as_os_str().to_owned()])
    );
    // The switch, and the path in quotes of its own, as explorer's
    // command line must read: a space or a comma in the path would
    // otherwise end it there.
    #[cfg(windows)]
    {
        assert_eq!(
            (program.as_str(), args),
            (
                "explorer",
                vec![std::ffi::OsString::from(
                    "/select,\"/config/settings.toml\""
                )]
            )
        );
        let awkward = std::path::Path::new(r"C:\Users\Doe, John\My Settings\settings.toml");
        let (_, args) = reveal_command(awkward);
        assert_eq!(
            args,
            vec![std::ffi::OsString::from(
                r#"/select,"C:\Users\Doe, John\My Settings\settings.toml""#
            )]
        );
    }
    // No file manager is asked to select a file the same way: the
    // directory is opened.
    #[cfg(all(unix, not(target_os = "macos")))]
    assert_eq!(
        (program.as_str(), args),
        ("xdg-open", vec![std::ffi::OsString::from("/config")])
    );
}

#[test]
fn what_starts_the_program_is_told_with_its_answer() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("settings.toml");
    let (outbox, events) = quiet_outbox();
    // A file that is not there is written first, whichever program it is
    // for, and a program that does not start is told with which it was.
    open_settings_file(
        &path,
        PAGE_500,
        &Written::default(),
        Opened::Folder,
        |shown| {
            assert_eq!(std::fs::read_to_string(shown).unwrap(), PAGE_500);
            Err(std::io::Error::other("no file manager"))
        },
        &outbox,
    );
    match events.try_recv() {
        Ok(Event::SettingsFileOpened {
            with: Opened::Folder,
            result: Err(error),
        }) => assert!(error.contains("no file manager"), "{error}"),
        other => panic!("{other:?}"),
    }
}
```

`quiet_outbox` is the helper the reader's tests use for an outbox with its events' receiver. `open_settings_file` is the helper Step 3 asks for.

In `src/app.rs`, beside the test of a failed editor start (search for `Could not open`):

```rust
#[test]
fn a_reveal_that_fails_is_told_in_the_notice() {
    let (mut app, _dir) = app();
    app.apply(Action::Backend(Event::SettingsFileOpened {
        with: crate::backend::Opened::Folder,
        result: Err("no file manager".into()),
    }));
    let notice = app.notice.expect("a notice");
    assert!(notice.contains("Could not show"), "{notice}");
    assert!(notice.contains("no file manager"), "{notice}");
}
```

`app()` is the helper of `src/app.rs`'s tests: it gives the `App` and the temporary directory that must outlive it.

- [ ] **Step 2: Run them and see them fail**

Run: `~/.cargo/bin/cargo test --locked --lib reveal`
Expected: does not compile: `reveal_command` and `Opened` are not defined.

- [ ] **Step 3: Implement**

`Command`, after `EditSettingsFile`:

```rust
/// Shows the settings file `path` in the file manager, written from
/// `text` first when it is not there. Answered with
/// [`Event::SettingsFileOpened`].
RevealSettingsFile {
    path: PathBuf,
    text: String,
},
```

`Event`:

```rust
/// A program was started on the settings file, or why it was not.
SettingsFileOpened {
    with: Opened,
    result: Result<(), String>,
},
```

```rust
/// What the settings file was handed to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Opened {
    /// The editor.
    Editor,
    /// The file manager, to show where the file is.
    Folder,
}
```

The program, beside `editor_command`:

```rust
/// The program that shows where `path` is kept on this system, and what
/// it is given. On Windows the one argument is explorer's command line as
/// it must read, and is given to it as it is ([`start_reveal`]).
fn reveal_command(path: &std::path::Path) -> (String, Vec<std::ffi::OsString>) {
    #[cfg(target_os = "macos")]
    {
        ("open".into(), vec!["-R".into(), path.as_os_str().to_owned()])
    }
    #[cfg(windows)]
    {
        // The switch, then the path in quotes of its own. Explorer reads
        // its command line itself: it takes `/select,` for its switch only
        // outside quotes, and splits what follows at a comma outside them,
        // so a path with a space or a comma is whole only in quotes. A
        // path on Windows holds no quote to escape.
        let mut select = std::ffi::OsString::from("/select,\"");
        select.push(path);
        select.push("\"");
        ("explorer".into(), vec![select])
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        // No two file managers select a file the same way; every desktop
        // opens a directory.
        let directory = crate::util::directory_of(path).as_os_str().to_owned();
        ("xdg-open".into(), vec![directory])
    }
}
```

Split `start_editor` so both programs are started and reaped the same way:

```rust
/// Starts `command`, which runs `program`, and lets it go: it is the
/// user's window from here. A thread of its own waits for it, so it leaves
/// no zombie, and says in the log when it ended with a failure: a launcher
/// with nothing to open the file with has no other way to be heard.
fn start(program: String, mut command: std::process::Command) -> std::io::Result<()> {
    use std::process::Stdio;
    let mut child = command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;
    std::thread::spawn(move || match child.wait() {
        Ok(status) if status.success() => {}
        // Windows' explorer ends with a failure whatever it did: its
        // status says nothing, and a warning at every press would.
        Ok(_) if cfg!(windows) => {}
        Ok(status) => log::warn!("{program} ended with {status}"),
        Err(error) => log::warn!("could not wait for {program}: {error}"),
    });
    Ok(())
}

/// Starts the editor on `path`.
fn start_editor(path: &std::path::Path) -> std::io::Result<()> {
    let paths = std::env::var_os("PATH");
    let found = |name: &str| {
        paths
            .as_deref()
            .is_some_and(|paths| on_path(name, std::env::split_paths(paths)))
    };
    let (program, args) = editor_command(path, found);
    let mut command = std::process::Command::new(&program);
    command.args(args);
    start(program, command)
}

/// Starts the file manager where `path` is.
fn start_reveal(path: &std::path::Path) -> std::io::Result<()> {
    let (program, args) = reveal_command(path);
    let mut command = std::process::Command::new(&program);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt as _;
        // As it is written: quoted by the usual rules, the quotes around
        // the path would be escaped and the switch taken into them.
        for arg in &args {
            command.raw_arg(arg);
        }
    }
    #[cfg(not(windows))]
    command.args(&args);
    start(program, command)
}
```

The worker's two arms share one helper, so they differ only in what they pass:

```rust
/// Hands the settings file to a program: `open_in_editor` with `start`,
/// then the answer, logged when it is a failure and sent to the app.
fn open_settings_file(
    path: &std::path::Path,
    text: &str,
    written: &Written,
    with: Opened,
    start: impl FnOnce(&std::path::Path) -> std::io::Result<()>,
    outbox: &Outbox,
) {
    let result = open_in_editor(path, text, written, start);
    if let Err(error) = &result {
        match with {
            Opened::Editor => {
                log::warn!("could not open {} in the editor: {error}", path.display());
            }
            Opened::Folder => {
                log::warn!("could not show {} in the file manager: {error}", path.display());
            }
        }
    }
    outbox.emit(Event::SettingsFileOpened { with, result });
}
```

Each arm clones the outbox and the `Written`, and calls it on the blocking pool with `Opened::Editor` and `start_editor`, or `Opened::Folder` and `start_reveal`. `open_in_editor`'s doc comment says it serves both. Add `Command::RevealSettingsFile { .. }` to each of the four exhaustive matches that list `EditSettingsFile`.

`src/app.rs`:

```rust
Event::SettingsFileOpened { with, result } => {
    if let Err(error) = result {
        let path = self.dirs.settings_file();
        self.notice = Some(match with {
            Opened::Editor => format!(
                "Could not open {} in the editor: {error}.",
                path.display()
            ),
            Opened::Folder => format!(
                "Could not show {} in the file manager: {error}.",
                path.display()
            ),
        });
    }
}
```

Every existing `Event::SettingsFileOpened { result }` in tests gains `with: Opened::Editor`.

- [ ] **Step 4: Run the tests**

Run: `~/.cargo/bin/cargo test --locked --lib backend:: && ~/.cargo/bin/cargo test --locked --lib app::`
Expected: PASS.

- [ ] **Step 5: Stage for the controller**

`git add src/backend.rs src/app.rs`. Commit message: "Show the settings file in the file manager from the backend".

---

### Task 2: The Reveal and Export actions

**Files:**
- Modify: `src/model.rs` (`Action`)
- Modify: `src/app.rs` (`App::apply`)
- Modify: `src/backend.rs` (`Backend::save_bytes`)
- Test: `src/app.rs` (`mod tests`)

- [ ] **Step 1: Write the failing tests**

```rust
#[test]
fn reveal_sends_the_path_and_the_text_the_app_holds() {
    let (mut app, _dir) = app();
    app.apply(Action::RevealSettingsFile);
    match crate::testing::last_sent(&app) {
        Command::RevealSettingsFile { path, text } => {
            assert_eq!(path, &app.dirs.settings_file());
            assert_eq!(text, &app.settings_file.text);
        }
        other => panic!("{other:?}"),
    }
    // Written by the backend if the file is gone: the app's own write.
    assert!(app.settings_file.offered.contains(&app.settings_file.text));
}

#[test]
fn export_offers_the_canonical_text_under_the_settings_name() {
    let (mut app, _dir) = app();
    app.apply(Action::SetOption(crate::settings::OptionValue::PageSize(500)));
    app.apply(Action::ExportSettings);
    let text = app.settings.to_toml();
    assert_eq!(
        app.backend.saves.last(),
        Some(&("tabletist-settings.toml".to_owned(), text.len()))
    );
}
```

`Backend::saves` is the test field that records each save asked for: `Vec<(String, usize)>`, a name and a size.

- [ ] **Step 2: Run them and see them fail**

Run: `~/.cargo/bin/cargo test --locked --lib -- reveal_sends export_offers`
Expected: does not compile: the two actions are not defined.

- [ ] **Step 3: Implement**

`src/model.rs`, after `EditSettingsFile`:

```rust
/// Show the settings file in the file manager.
RevealSettingsFile,
/// Save a copy of the settings where the user says.
ExportSettings,
```

`Backend::save_bytes` gains the dialog's title as its first parameter, `title: &str`, passed to `.set_title(title)`; its doc comment says "Asks where to save `bytes`, in a dialog titled `title`, suggesting `name`". `Action::SaveValue` passes `"Save value"`.

`src/app.rs`:

```rust
// As for the editor: the backend writes the text when no file is there,
// and that write is the app's own, to be known when it comes back.
Action::RevealSettingsFile => {
    let text = self.offer_settings_text();
    self.backend.send(Command::RevealSettingsFile {
        path: self.dirs.settings_file(),
        text,
    });
}
// The app's own text, whatever the file holds: a line the app ignores
// is not a setting to hand on.
Action::ExportSettings => self.backend.save_bytes(
    "Export settings",
    "tabletist-settings.toml".to_owned(),
    self.settings.to_toml().into_bytes(),
),
```

- [ ] **Step 4: Run the tests**

Run: `~/.cargo/bin/cargo test --locked --lib app::`
Expected: PASS.

- [ ] **Step 5: Stage for the controller**

`git add src/model.rs src/app.rs src/backend.rs`. Commit message: "Reveal and export the settings from the app".

---

### Task 3: Reset to defaults, asked first

**Files:**
- Modify: `src/model.rs` (`SettingsDialog`, `Action`)
- Modify: `src/app.rs` (`App::apply`)
- Test: `src/app.rs` (`mod tests`)

- [ ] **Step 1: Write the failing tests**

```rust
#[test]
fn reset_asks_first_and_then_resets_only_the_four_options() {
    let (mut app, _dir) = app();
    app.apply(Action::ShowSettings);
    // An option of the window, and a key it does not show.
    app.change_settings(|settings| {
        settings.page_size = 500;
        settings.group_digits = true;
        settings.show_system_schemas = true;
        settings.sql_limit = 50;
    });
    let resetting = |app: &App| match &app.dialog {
        Some(Dialog::Settings(dialog)) => dialog.resetting,
        _ => panic!("the window is not open"),
    };
    app.apply(Action::ResetSettings);
    assert!(resetting(&app));
    assert_eq!(app.settings.page_size, 500, "nothing is reset yet");
    // Cancel leaves everything.
    app.apply(Action::ConfirmResetSettings(false));
    assert!(!resetting(&app));
    assert_eq!(app.settings.page_size, 500);
    // Reset puts the four back and leaves the others.
    app.apply(Action::ResetSettings);
    app.apply(Action::ConfirmResetSettings(true));
    assert!(!resetting(&app));
    let defaults = Settings::default();
    assert_eq!(app.settings.page_size, defaults.page_size);
    assert_eq!(app.settings.group_digits, defaults.group_digits);
    assert!(app.settings.show_system_schemas);
    assert_eq!(app.settings.sql_limit, 50);
    assert_eq!(app.settings_file.text, app.settings.to_toml());
}

#[test]
fn a_reset_nobody_asked_for_resets_nothing() {
    let (mut app, _dir) = app();
    app.apply(Action::ShowSettings);
    app.change_settings(|settings| settings.page_size = 500);
    app.apply(Action::ConfirmResetSettings(true));
    assert_eq!(app.settings.page_size, 500);
}
```

Use the real field names of `Settings` (`sql_limit` may be an `Option`; set it to a value that is not the default).

- [ ] **Step 2: Run them and see them fail**

Run: `~/.cargo/bin/cargo test --locked --lib reset`
Expected: does not compile.

- [ ] **Step 3: Implement**

`SettingsDialog`:

```rust
/// Reset to defaults was chosen and waits for its answer: the footer
/// asks in place of its links.
pub resetting: bool,
```

`Action`:

```rust
/// Ask whether to put the Settings window's options back.
ResetSettings,
/// The answer: put them back, or leave them.
ConfirmResetSettings(bool),
```

`App::apply`:

```rust
Action::ResetSettings => {
    if let Some(Dialog::Settings(dialog)) = &mut self.dialog {
        dialog.resetting = true;
    }
}
Action::ConfirmResetSettings(reset) => {
    let Some(Dialog::Settings(dialog)) = &mut self.dialog else {
        return;
    };
    // An answer to a question that was asked.
    let asked = std::mem::take(&mut dialog.resetting);
    if asked && reset {
        // The options the window shows. A key it has no control for is
        // not the window's to change.
        self.change_settings(|settings| {
            for option in crate::settings::OptionId::ALL {
                option.default_value().set(settings);
            }
        });
    }
}
```

If `apply` cannot `return` from that arm (it runs code after the match), write the arm with `if let` instead.

- [ ] **Step 4: Run the tests**

Run: `~/.cargo/bin/cargo test --locked --lib app::`
Expected: PASS.

- [ ] **Step 5: Stage for the controller**

`git add src/model.rs src/app.rs`. Commit message: "Reset the window's options once the question is answered".

---

### Task 4: `widgets::toggle`

**Files:**
- Modify: `src/ui/widgets.rs`
- Test: `src/ui/widgets.rs` (`mod tests`)

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn a_toggle_is_a_checkbox_that_says_what_it_is_set_to_and_takes_a_click() {
    use crate::testing::Harness;
    use egui::accesskit::{Action, ActionRequest, Role, Toggled, TreeId};
    let mut harness = Harness::new();
    let palette = harness.app.palette;
    for on in [false, true] {
        let tree = harness.frame_with(|ui| {
            toggle(ui, on, "Value tags", &palette);
        });
        let id = crate::testing::node(&tree, "Value tags", Role::CheckBox).expect("the toggle");
        let node = tree
            .nodes
            .iter()
            .find(|(node, _)| *node == id)
            .map(|(_, node)| node)
            .expect("its node");
        let expected = if on { Toggled::True } else { Toggled::False };
        assert_eq!(node.toggled(), Some(expected));
        // A click, as a screen reader makes one.
        let click = egui::Event::AccessKitActionRequest(ActionRequest {
            target_tree: TreeId::ROOT,
            target_node: id,
            action: Action::Click,
            data: None,
        });
        let mut clicked = false;
        harness.frame_with_events(vec![click], |ui| {
            clicked = toggle(ui, on, "Value tags", &palette).clicked();
        });
        assert!(clicked);
    }
}
```

It goes in `src/ui/widgets.rs`'s `mod tests`. `frame_with` draws only what its closure draws (`harness.click` draws the whole app and would not find the toggle), and `frame_with_events` hands the frame an event: `wait_over_a_page` in `src/ui/states.rs` is the working example of both. If `WidgetInfo::selected` with `WidgetType::Checkbox` does not set `toggled` on the node in this fork, assert what it does set (look at how `widgets::checkbox`'s node reads in an existing test) and keep the two facts: a checkbox named "Value tags" is in the tree, and a click on it is reported by `clicked()`.

- [ ] **Step 2: Run it and see it fail**

Run: `~/.cargo/bin/cargo test --locked --lib a_toggle_is`
Expected: does not compile: `toggle` is not defined.

- [ ] **Step 3: Implement**

```rust
/// A switch's track and its knob.
const TOGGLE: egui::Vec2 = vec2(30.0, 18.0);
const TOGGLE_KNOB: f32 = 14.0;

/// A switch: off or `on`, flipped by a click, by Space or by Enter. To a
/// screen reader it is a checkbox named `name`. The caller flips what it
/// shows when the response says it was clicked.
pub fn toggle(ui: &mut Ui, on: bool, name: &str, palette: &Palette) -> Response {
    let (rect, response) = ui.allocate_exact_size(TOGGLE, Sense::click());
    response.widget_info(|| WidgetInfo::selected(WidgetType::Checkbox, true, on, name));
    if ui.is_rect_visible(rect) {
        let track = if on {
            palette.accent
        } else {
            // Darker than a border: the knob and the track are told apart
            // on the window and on a panel alike.
            palette.border.lerp_to_gamma(palette.faint, 0.23)
        };
        let radius = (TOGGLE.y / 2.0) as u8;
        ui.painter()
            .rect_filled(rect, CornerRadius::same(radius), track);
        let inset = (TOGGLE.y - TOGGLE_KNOB) / 2.0;
        let left = if on {
            rect.right() - inset - TOGGLE_KNOB
        } else {
            rect.left() + inset
        };
        let knob = Rect::from_min_size(
            egui::pos2(left, rect.top() + inset),
            vec2(TOGGLE_KNOB, TOGGLE_KNOB),
        );
        ui.painter().rect_filled(
            knob,
            CornerRadius::same((TOGGLE_KNOB / 2.0) as u8),
            palette.window,
        );
    }
    focus::hint(ui, &response, rect, Ring::Outer { radius: 9 });
    response
}
```

It takes no look: the switch is the same in both looks that draw it, and `focus::hint` takes the ring. Check how `Ring::Outer` is used by another control for the radius it expects.

- [ ] **Step 4: Run the test**

Run: `~/.cargo/bin/cargo test --locked --lib a_toggle_is`
Expected: PASS.

- [ ] **Step 5: Stage for the controller**

`git add src/ui/widgets.rs`. Commit message: "Add a toggle switch to the widgets".

---

### Task 5: The sheet, and the ways in for every look

The window in the standard and macOS looks: frame, nav, title, the four rows with their controls. The footer comes in Task 6.

**Files:**
- Create: `src/ui/settings/sheet.rs`
- Modify: `src/ui/settings/mod.rs` (`show`, the rows' words)
- Modify: `src/ui/keys.rs` (the gate on `Mod+,`, `SHORTCUTS`)
- Modify: `src/ui/help.rs` (the gate on the Settings button)
- Modify: `src/ui/settings/terminal.rs` (takes its small print from `mod.rs` if it has the same words)
- Test: `src/ui/mod.rs`, `src/ui/keys.rs`

- [ ] **Step 1: Write the failing tests**

In `src/ui/mod.rs`, replace `the_other_looks_have_no_settings_window_yet` with:

```rust
/// The two looks that draw the Settings window as a sheet.
const SHEET_LOOKS: [fn() -> crate::theme::Look; 2] =
    [crate::theme::Look::standard, crate::theme::Look::macos];

/// A harness in `look` with the Settings window open.
fn settings_sheet(look: crate::theme::Look) -> Harness {
    let mut harness = Harness::new();
    harness.set_look(look);
    harness.press(egui::Key::Comma, egui::Modifiers::COMMAND);
    assert!(settings_open(&harness), "{}", look.name);
    harness
}

#[test]
fn mod_comma_opens_the_settings_window_in_every_look_and_escape_closes_it() {
    for look in crate::theme::Look::ALL {
        let mut harness = Harness::new();
        harness.set_look(look);
        harness.press(egui::Key::Comma, egui::Modifiers::COMMAND);
        assert!(settings_open(&harness), "{}", look.name);
        // A second press leaves it as it is.
        harness.press(egui::Key::Comma, egui::Modifiers::COMMAND);
        assert!(settings_open(&harness), "{}", look.name);
        harness.press(egui::Key::Escape, egui::Modifiers::NONE);
        assert!(harness.app.dialog.is_none(), "{}", look.name);
    }
}

#[test]
fn the_shortcuts_dialog_opens_the_settings_in_every_look() {
    for look in crate::theme::Look::ALL {
        let mut harness = Harness::new();
        harness.set_look(look);
        harness.frame(vec![egui::Event::Text("?".into())]);
        harness.click("Settings");
        assert!(settings_open(&harness), "{}", look.name);
    }
}

#[test]
fn the_settings_sheet_shows_the_general_tab() {
    for look in SHEET_LOOKS {
        let mut harness = settings_sheet(look());
        for text in [
            "Settings",
            "General",
            "How Tabletist shows data. Changes apply right away.",
            "Rows per page",
            "Table view; the SQL editor has its own limit",
            "Timestamps",
            "Numbers",
            "Grouping is display only; copy gives the raw value",
            "Value tags",
        ] {
            assert!(harness.has(text), "{text}");
        }
        // The sample is of the precision that is set.
        assert!(harness.painted_color("2026-01-12 09:14:03").is_some());
    }
}

#[test]
fn each_control_of_the_settings_sheet_changes_its_setting_and_saves() {
    for look in SHEET_LOOKS {
        let mut harness = settings_sheet(look());
        let before = settings_saves(&harness);
        harness.click("Full precision");
        assert_eq!(
            harness.app.settings.timestamps,
            crate::settings::Timestamps::Full
        );
        harness.settle();
        assert!(harness.painted_color("2026-01-12 09:14:03.482915").is_some());
        harness.click("1,240.50");
        assert!(harness.app.settings.group_digits);
        harness.click("1240.50");
        assert!(!harness.app.settings.group_digits);
        let tags = harness.app.settings.value_tags;
        harness.click("Value tags");
        assert_eq!(harness.app.settings.value_tags, !tags);
        // The menu: opened, and a size picked from it.
        harness.click("Rows per page");
        harness.click("Rows per page 500");
        assert_eq!(harness.app.settings.page_size, 500);
        assert_eq!(settings_saves(&harness), before + 5);
        assert!(settings_open(&harness), "the window stays open");
    }
}

#[test]
fn a_page_size_from_the_file_that_is_not_in_the_list_is_in_the_menu() {
    let mut harness = settings_sheet(crate::theme::Look::standard());
    harness.app.apply(crate::model::Action::SetOption(
        crate::settings::OptionValue::PageSize(250),
    ));
    harness.click("Rows per page");
    for size in ["100", "250", "300", "500", "1,000", "5,000"] {
        let entry = format!("Rows per page {size}");
        assert!(harness.has(&entry), "{entry}");
    }
}

#[test]
fn the_letters_of_the_terminal_screen_do_nothing_in_the_sheet() {
    let mut harness = settings_sheet(crate::theme::Look::macos());
    let before = harness.app.settings.clone();
    for key in [egui::Key::J, egui::Key::L, egui::Key::H, egui::Key::Space] {
        harness.press(key, egui::Modifiers::NONE);
    }
    harness.press(egui::Key::R, egui::Modifiers::SHIFT);
    harness.press(egui::Key::E, egui::Modifiers::CTRL);
    assert_eq!(harness.app.settings, before);
    // Nothing may have been sent at all: `last_sent` would panic.
    let sent = &harness.app.backend.sent;
    assert!(
        !sent
            .iter()
            .any(|command| matches!(command, Command::EditSettingsFile { .. }))
    );
}
```

Rename `mod_comma_opens_the_settings_in_the_terminal_look_and_escape_closes_them` and `the_shortcuts_dialog_opens_the_settings_in_the_terminal_look` only if they now repeat the tests above; keep what they check of the terminal screen (the cursor's mark, the cursor kept by a second press).

How a menu is opened and an entry clicked in a test is shown by the SQL editor's Limit menu tests (`harness.click("Limit")`, then `harness.click("Limit 100")` in `src/ui/mod.rs`).

`harness.has` and `harness.click` find what a screen reader is told, not what is painted: every word these tests look for must be a label or be announced (`widgets::label`, `widgets::paint_label`, `widgets::announce`). `harness.painted_color` finds painted text, which is why the sample timestamp is checked with it.

In `src/ui/keys.rs`:

```rust
#[test]
fn the_shortcut_table_lists_the_settings_key() {
    assert!(SHORTCUTS.contains(&("Mod+,", "Settings")));
}
```

- [ ] **Step 2: Run them and see them fail**

Run: `~/.cargo/bin/cargo test --locked --lib settings`
Expected: FAIL: the standard and macOS looks open nothing.

- [ ] **Step 3: Open the window in every look**

`src/ui/keys.rs`: drop `app.look.terminal &&` and its comment's second sentence from the `Mod+,` handler, and add the line to `SHORTCUTS` before `("?", "Shortcuts")`:

```rust
("Mod+,", "Settings"),
```

`src/ui/help.rs`: drop `look.terminal &&` and the comment over it.

`src/ui/settings/mod.rs`:

```rust
mod file_pane;
mod sheet;
mod terminal;
```

```rust
pub fn show(app: &mut App, ctx: &egui::Context) {
    let Some(Dialog::Settings(dialog)) = &app.dialog else {
        return;
    };
    let (row, resetting) = (dialog.row, dialog.resetting);
    let mut actions = Vec::new();
    if app.look.terminal {
        // The screen's keys act on its cursor. The sheet has none: its
        // controls are reached with Tab.
        keys(app, ctx, row, &mut actions);
        terminal::show(app, ctx, row, &mut actions);
    } else {
        sheet::show(app, ctx, resetting, &mut actions);
    }
    app.actions.extend(actions);
}
```

The module's doc comment loses "the other looks get their window in a later step" and says what `sheet.rs` draws.

The words both layouts use go in `mod.rs`, so they cannot drift:

```rust
/// What a row says under its label, where it says something. English.
pub(super) fn small_print(option: OptionId) -> Option<&'static str> {
    match option {
        OptionId::PageSize => Some("Table view; the SQL editor has its own limit"),
        OptionId::ValueTags => Some("Colors for enum, CHECK (…IN…) and boolean columns"),
        OptionId::Timestamps | OptionId::GroupDigits => None,
    }
}

/// A timestamp as the grid shows one at each precision.
pub(super) fn sample_timestamp(timestamps: Timestamps) -> &'static str {
    match timestamps {
        Timestamps::Second => "2026-01-12 09:14:03",
        Timestamps::Full => "2026-01-12 09:14:03.482915",
    }
}
```

`terminal.rs`'s `hint` takes its two timestamps from `sample_timestamp`.

- [ ] **Step 4: Draw the sheet**

`src/ui/settings/sheet.rs`. The code below is the shape and the measures; where it calls a helper of this repository (`Text`, `widgets::paint_text`, `widgets::section_label`, `focus::hint`, `value_tags::slot_colors`), check the helper's signature and follow it. The tests of Step 1 are the contract.

```rust
//! The Settings window in the looks that are not a terminal's: a sheet
//! over the dimmed window, with the nav at its left, the tab's options in
//! rows, and a footer that says where the file is and what can be done
//! with it.

use egui::{CornerRadius, Rect, Sense, Stroke, Ui, WidgetInfo, WidgetType, pos2, vec2};

use crate::app::App;
use crate::i18n::{Locale, gettext};
use crate::model::Action;
use crate::settings::{OptionId, OptionValue, Settings, Timestamps};
use crate::theme::{Look, Palette};
use crate::typography::{Text, TextRole};
use crate::ui::focus::{self, Ring};
use crate::ui::widgets::{self, Segment};

use super::{label, sample_timestamp, small_print};

/// The sheet at its widest, and what it leaves of the window at each side.
const WIDTH: f32 = 1040.0;
const MARGIN: f32 = 40.0;
/// The nav, its padding, and one of its items.
const NAV: f32 = 210.0;
const NAV_PAD: egui::Vec2 = vec2(10.0, 14.0);
const NAV_ITEM: f32 = 28.0;
/// The content's padding: over the title, and at its sides.
const TOP: f32 = 22.0;
const SIDE: f32 = 28.0;
/// A row: its label's column, the gap after it, and the space over and
/// under what it holds.
const LABEL: f32 = 300.0;
const GAP: f32 = 16.0;
const ROW_PAD: f32 = 9.0;
/// Between a control and its hint.
const HINT_GAP: f32 = 12.0;
/// The page size menu's button.
const MENU: f32 = 120.0;

/// How the sheet draws: the look, the palette and the language.
#[derive(Clone, Copy)]
struct Skin<'a> {
    look: &'a Look,
    palette: &'a Palette,
    locale: Locale,
}

impl Skin<'_> {
    fn say(&self, text: &'static str) -> String {
        gettext(self.locale, text).into_owned()
    }
}

pub(super) fn show(app: &App, ctx: &egui::Context, resetting: bool, actions: &mut Vec<Action>) {
    let skin = Skin {
        look: &app.look,
        palette: &app.palette,
        locale: app.locale,
    };
    let screen = ctx.content_rect();
    let width = WIDTH.min(screen.width() - 2.0 * MARGIN).max(NAV + 320.0);
    // Its parts fill it edge to edge: no margin of its own.
    let frame = egui::Frame::new()
        .fill(skin.palette.window)
        .corner_radius(CornerRadius::same(skin.look.dialog_radius))
        .stroke(Stroke::new(1.0, skin.palette.border))
        .shadow(egui::epaint::Shadow {
            offset: [0, 20],
            blur: 50,
            spread: 0,
            color: skin.palette.shadow,
        });
    let modal = widgets::modal(egui::Id::new("settings"), skin.look, skin.palette)
        .frame(frame)
        .show(ctx, |ui| {
            ui.set_width(width);
            ui.spacing_mut().item_spacing = egui::Vec2::ZERO;
            ui.horizontal_top(|ui| {
                nav(ui, &skin);
                ui.vertical(|ui| {
                    ui.set_width(width - NAV);
                    content(ui, &app.settings, &skin, actions);
                    // Task 6 draws the footer here.
                    let _ = resetting;
                });
            });
        });
    if modal.is_top_modal
        && !egui::Popup::is_any_open(ctx)
        && ctx.input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::Escape))
    {
        actions.push(Action::CloseDialog);
    }
}
```

`nav`: allocate `NAV` by the content's height (lay the content out first if the nav must be as tall as it: paint the nav's fill and its rule on a shape reserved with `ui.painter().add(egui::Shape::Noop)` before the content, and set it once the height is known, as `widgets::popup_button` does with its shadow). Paint `palette.panel` with the sheet's left corners, the rule in `palette.outline`, "Settings" in `TextRole::StateTitle`, and one item "General": `palette.selection` fill with radius 5, the word in `TextRole::UiBodyStrong` and `palette.accent_hover`. The item is a button to a screen reader, selected: `WidgetInfo::selected(WidgetType::Button, true, true, "General")`, with a focus ring from `focus::hint`.

`content`:

```rust
fn content(ui: &mut Ui, settings: &Settings, skin: &Skin, actions: &mut Vec<Action>) {
    let Skin { look, palette, .. } = *skin;
    egui::Frame::new()
        .inner_margin(egui::Margin {
            left: SIDE as i8,
            right: SIDE as i8,
            top: TOP as i8,
            bottom: 0,
        })
        .show(ui, |ui| {
            widgets::label(
                ui,
                TextRole::DialogTitle,
                &skin.say("General"),
                palette.text,
                look,
            );
            ui.add_space(2.0);
            widgets::label(
                ui,
                TextRole::UiBody,
                &skin.say("How Tabletist shows data. Changes apply right away."),
                palette.dim,
                look,
            );
            ui.add_space(6.0 + 14.0);
            let section = widgets::section_label(&skin.say("Data"), look, palette);
            ui.add(egui::Label::new(section.layout(ui.ctx()).galley));
            ui.add_space(2.0);
            for option in OptionId::ALL {
                // Each row's controls have ids of their own: two segmented
                // controls in one `Ui` would share their arrow keys.
                ui.push_id(label(option), |ui| row(ui, option, settings, skin, actions));
            }
        });
}
```

`row`: a horizontal strip. At the left a column `LABEL` wide with the label (`TextRole::UiBody`, `palette.text`) and, 2 under it, `small_print(option)` in `TextRole::Secondary` and `palette.dim`. After `GAP`, the control, then `HINT_GAP` and the hint, centred on the row's middle. `ROW_PAD` over and under, and a 1 px rule in `palette.surface` under the row (`widgets::hline`).

The controls:

```rust
fn control(
    ui: &mut Ui,
    option: OptionId,
    settings: &Settings,
    skin: &Skin,
    actions: &mut Vec<Action>,
) {
    let Skin { look, palette, .. } = *skin;
    let name = skin.say(label(option));
    match option {
        OptionId::PageSize => {
            // Text goes through the look's roles: `widgets::galley`, as
            // the filter bar's menus are written.
            let written = |ui: &Ui, size: u32| {
                let text = crate::ui::format::group_digits(u64::from(size));
                widgets::galley(ui, &text, egui::Color32::PLACEHOLDER, look)
            };
            let combo = egui::ComboBox::from_id_salt("page-size")
                .width(MENU)
                .selected_text(written(ui, settings.page_size));
            let menu = widgets::popup_button(ui, combo, look, palette, |ui| {
                for size in page_sizes(settings.page_size) {
                    let text = written(ui, size);
                    if ui
                        .selectable_label(size == settings.page_size, text)
                        .clicked()
                    {
                        actions.push(Action::SetOption(OptionValue::PageSize(size)));
                    }
                }
            });
            menu.response
                .widget_info(|| WidgetInfo::labeled(WidgetType::ComboBox, true, &name));
        }
        OptionId::Timestamps => {
            let words = [skin.say("To the second"), skin.say("Full precision")];
            let segments = [Segment::Text(&words[0]), Segment::Text(&words[1])];
            let selected = usize::from(settings.timestamps == Timestamps::Full);
            let size = segment_size(ui, &words, look);
            if let Some(picked) = widgets::segmented(ui, &segments, selected, size, look, palette)
            {
                let value = [Timestamps::Second, Timestamps::Full][picked];
                actions.push(Action::SetOption(OptionValue::Timestamps(value)));
            }
        }
        OptionId::GroupDigits => {
            // A number as each value writes it: no language has others.
            let words = ["1,240.50".to_owned(), "1240.50".to_owned()];
            let segments = [Segment::Text(&words[0]), Segment::Text(&words[1])];
            let selected = usize::from(!settings.group_digits);
            let size = segment_size(ui, &words, look);
            if let Some(picked) = widgets::segmented(ui, &segments, selected, size, look, palette)
            {
                actions.push(Action::SetOption(OptionValue::GroupDigits(picked == 0)));
            }
        }
        OptionId::ValueTags => {
            if widgets::toggle(ui, settings.value_tags, &name, palette).clicked() {
                actions.push(Action::SetOption(OptionValue::ValueTags(
                    !settings.value_tags,
                )));
            }
        }
    }
}

/// The sizes the menu offers: the list, with the size in use in its place
/// when the file set one that is not in it.
fn page_sizes(current: u32) -> Vec<u32> {
    let mut sizes = Settings::PAGE_SIZES.to_vec();
    if !sizes.contains(&current) {
        sizes.push(current);
        sizes.sort_unstable();
    }
    sizes
}

/// One segment, wide enough for the widest of `words` with 10 at each
/// side: the segments of a control are of one width.
fn segment_size(ui: &Ui, words: &[String], look: &Look) -> egui::Vec2 {
    let role = TextRole::FieldLabel;
    let widest = words
        .iter()
        .map(|word| widgets::measure(ui, Text::one(look, role, word, egui::Color32::PLACEHOLDER)))
        .fold(0.0, f32::max);
    vec2((widest + 20.0).ceil(), 22.0)
}
```

`format::group_digits` is what the SQL editor's "Limit 1,000" is written with.

The menu's entries are named for a screen reader as the Limit menu's are ("Limit 100"): the option and the size, "Rows per page 500". Give each entry that name with `widget_info` (`WidgetInfo::selected(WidgetType::Button, true, selected, name)`, as `widgets::popup_menu` does), and the button the name "Rows per page" with the role of a combo box, as the Limit button has. If naming the entries of an `egui::ComboBox` proves awkward, build the menu as the SQL editor builds Limit (`src/ui/sql_editor.rs`, a button and `widgets::popup_menu` with `MenuChoice`s): the tests are the same either way.

The hints, after each control:

- Rows per page: none.
- Timestamps: `sample_timestamp(settings.timestamps)` in `TextRole::MonoSecondary`, `palette.dim`.
- Numbers: "Grouping is display only; copy gives the raw value" in `TextRole::Secondary`, `palette.dim`.
- Value tags: two tags, "print" and "ebook", in `TextRole::ValueTag`. With tags on, each on its fill from `value_tags::slot_colors(0, look, palette)` and `slot_colors(1, ..)` (padding 2 by 6, radius 4); with tags off, the two words plain in `palette.dim`, 6 apart either way. They are values of a column, not words: they are not translated.

Add a unit test of `page_sizes` in `sheet.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_size_that_is_not_in_the_list_takes_its_place_among_the_others() {
        assert_eq!(page_sizes(300), Settings::PAGE_SIZES);
        assert_eq!(page_sizes(250), [100, 250, 300, 500, 1_000, 5_000]);
        assert_eq!(page_sizes(9_000), [100, 300, 500, 1_000, 5_000, 9_000]);
    }
}
```

- [ ] **Step 5: Run the tests**

Run: `~/.cargo/bin/cargo test --locked --lib settings && ~/.cargo/bin/cargo test --locked --lib shortcut`
Expected: PASS. `the_shortcut_table_covers_the_spec_map` and the help dialog's width tests still pass with the new line.

- [ ] **Step 6: Stage for the controller**

`git add src/ui/settings src/ui/keys.rs src/ui/help.rs src/ui/mod.rs`. One commit, since the gate and the shortcut's line are in one file and belong to one fact (the key now opens a window in every look): "Draw the Settings window as a sheet in the standard and macOS looks".

---

### Task 6: The footer

**Files:**
- Modify: `src/ui/settings/sheet.rs`
- Modify: `src/ui/settings/mod.rs` (the path as it is shown), `src/ui/settings/file_pane.rs` (gives up `shown_path` to `mod.rs`, which both layouts use)
- Test: `src/ui/mod.rs`

- [ ] **Step 1: Write the failing tests**

```rust
#[test]
fn the_settings_sheet_says_where_the_file_is_and_what_can_be_done_with_it() {
    for look in SHEET_LOOKS {
        let mut harness = settings_sheet(look());
        assert!(harness.has("Stored in"));
        let reveal = if cfg!(target_os = "macos") {
            "Reveal in Finder"
        } else if cfg!(windows) {
            "Show in Explorer"
        } else {
            "Show in folder"
        };
        harness.click(reveal);
        match crate::testing::last_sent(&harness.app) {
            Command::RevealSettingsFile { path, text } => {
                assert_eq!(path, &harness.app.dirs.settings_file());
                assert_eq!(text, &harness.app.settings_file.text);
            }
            other => panic!("{other:?}"),
        }
        harness.click("Export…");
        assert_eq!(
            harness.app.backend.saves.last().map(|(name, _)| name.as_str()),
            Some("tabletist-settings.toml")
        );
        assert!(settings_open(&harness), "the window stays open");
    }
}

#[test]
fn reset_to_defaults_asks_in_place_of_the_footers_links() {
    for look in SHEET_LOOKS {
        let mut harness = settings_sheet(look());
        harness.click("Full precision");
        harness.click("Reset to defaults");
        assert!(harness.has("Reset every option on this tab?"));
        assert!(!harness.has("Export…"), "the links give way");
        harness.click("Cancel");
        assert!(harness.has("Export…"));
        assert_eq!(
            harness.app.settings.timestamps,
            crate::settings::Timestamps::Full
        );
        harness.click("Reset to defaults");
        harness.click("Reset");
        assert_eq!(
            harness.app.settings.timestamps,
            crate::settings::Timestamps::Second
        );
        assert!(harness.has("Export…"));
    }
}

#[test]
fn the_settings_sheet_counts_the_lines_the_app_ignores() {
    let mut harness = settings_sheet(crate::theme::Look::standard());
    let ignored = "1 line in the file could not be read and was ignored";
    assert!(!harness.has(ignored));
    harness.app.apply(crate::model::Action::Backend(
        crate::backend::Event::SettingsFile {
            text: "[data]\ngroup_digits = \"yes\"\npage_size = 500\n".into(),
            own: false,
        },
    ));
    assert!(harness.has(ignored));
    assert_eq!(harness.app.settings.page_size, 500);
}

#[test]
fn escape_answers_the_reset_question_before_it_closes_the_window() {
    let mut harness = settings_sheet(crate::theme::Look::macos());
    harness.click("Reset to defaults");
    harness.press(egui::Key::Escape, egui::Modifiers::NONE);
    assert!(settings_open(&harness));
    assert!(harness.has("Export…"));
    harness.press(egui::Key::Escape, egui::Modifiers::NONE);
    assert!(harness.app.dialog.is_none());
}
```

The sentence is a label, or announced, so a screen reader hears it and `harness.has` finds it.

- [ ] **Step 2: Run them and see them fail**

Run: `~/.cargo/bin/cargo test --locked --lib -- settings_sheet reset_to_defaults escape_answers`
Expected: FAIL: "Stored in" is not found.

- [ ] **Step 3: Implement**

In `mod.rs`, for both layouts (move `shown_path` here from `file_pane.rs` and have the pane call this). The home directory is `AppDirs::home`, found at the start: no frame looks it up.

```rust
/// The settings file's path as the window writes it: the home directory
/// as `~`.
pub(super) fn path_shown(app: &App) -> String {
    shown_path(&app.dirs.settings_file(), app.dirs.home.as_deref())
}
```

The Reveal link's words follow the system the app was built for, as the program it starts does:

```rust
/// What the link that shows the file in the file manager says. English.
fn reveal_words() -> &'static str {
    if cfg!(target_os = "macos") {
        "Reveal in Finder"
    } else if cfg!(windows) {
        "Show in Explorer"
    } else {
        "Show in folder"
    }
}
```

A link, as the data view draws its own (see "Full precision" in `src/ui/data_view.rs`):

```rust
/// A link of the footer: `text` in `color`, a click away from what it
/// does. Tab reaches it, and Space and Enter press it.
fn link(ui: &mut Ui, text: &str, color: egui::Color32, skin: &Skin) -> egui::Response {
    let Skin { look, palette, .. } = *skin;
    let role = TextRole::Secondary;
    let width = widgets::measure(ui, Text::one(look, role, text, color));
    let (rect, response) = ui.allocate_exact_size(vec2(width.ceil(), 18.0), Sense::click());
    response.widget_info(|| WidgetInfo::labeled(WidgetType::Link, true, text));
    let color = if response.hovered() && color == palette.accent {
        palette.accent_hover
    } else {
        color
    };
    widgets::paint_text(
        ui,
        rect.left(),
        rect.center().y,
        Text::one(look, role, text, color),
    );
    focus::hint(ui, &response, rect, Ring::Outer { radius: 3 });
    response
}
```

`footer(ui, app, resetting, skin, actions)`, under the content with the sheet's bottom corners: `palette.panel` fill, the rule over it in `palette.outline`, padding 12 by 28, 12 between its parts.

- Left: "Stored in" in `TextRole::Secondary` and `palette.secondary`, a space, the path from `path_shown(app)` in `TextRole::MonoSecondary`.
- A line of its own over those, while `app.settings_file.invalid` is not empty, in `palette.warning`: the spec's sentence, "2 lines in the file could not be read and were ignored", and for one line "1 line in the file could not be read and was ignored". The number, a space, and `ngettext(locale, "line in the file could not be read and was ignored", "lines in the file could not be read and were ignored", count)`, as `src/ui/json_view.rs` writes its counts (`ngettext` takes a `u32`).
- Right, while `!resetting`: `link(reveal_words())` pushing `Action::RevealSettingsFile`, `link("Export…")` pushing `Action::ExportSettings`, and `link("Reset to defaults")` in `palette.danger` pushing `Action::ResetSettings`.
- Right, while `resetting`: "Reset every option on this tab?" in `palette.text`, then `link("Reset")` in `palette.danger` pushing `ConfirmResetSettings(true)` and `link("Cancel")` pushing `ConfirmResetSettings(false)`.

A path too long for what the links leave is cut at its start with `…` (`grid::ellipsize` cuts at the end; cut by hand from the left, by characters, until it fits). The links are never cut.

Escape, in `show`: while `resetting`, it answers the question with `ConfirmResetSettings(false)` and leaves the window open.

The content scrolls when the window is too short for it, and the footer stays: wrap the rows in `egui::ScrollArea::vertical().max_height(screen.height() - 2.0 * MARGIN - <the title's and the footer's heights>)` with `auto_shrink([false, true])`, as `help.rs` does for its list.

- [ ] **Step 4: Run the tests**

Run: `~/.cargo/bin/cargo test --locked --lib settings`
Expected: PASS, the terminal screen's tests with them: its pane still shows the path.

- [ ] **Step 5: Stage for the controller**

`git add src/ui/settings src/ui/mod.rs`. Commit message: "Give the Settings window its footer: where the file is, Reveal, Export and Reset".

---

### Task 7: "Settings…" in the macOS app menu (moved to its own plan)

Not built by this plan. The item needs `tabletist-appkit`, and none of it can be run from Linux, so it has a plan of its own, `docs/superpowers/plans/2026-10-04-settings-macos-menu.md`. That plan's work was merged into this plan's pull request, which therefore delivers the item too: on macOS `⌘,` reaches the window through the menu, and through the key handler where the item could not be added.

---

### Task 8: The shot, the spec, and the by-hand list

**Files:**
- Modify: `src/shots.rs`
- Modify: `docs/superpowers/specs/2026-10-03-settings-general-design.md`

- [ ] **Step 1: Add the scene**

`Screen::MacSettings`, as `Screen::OmarchySettings` was added: in `ALL` (the array's length grows by one), `name` "macos-settings-general", `look` `Look::macos()`, `size` 1440 by 900 and `design_scale` with the other macOS screens, the light palette, and the same setup as `OmarchySettings` (the Bookshop workspace under it, then `Action::ShowSettings`).

- [ ] **Step 2: Render it and look at it**

Run the shots as `OmarchySettings` was rendered (see how `target/shots/mock-*.png` are made in `src/shots.rs` and its tests). Read `target/shots/mock-macos-settings-general.png` and compare by eye with the Design values above: the nav, the four rows, the controls, the footer. Fix what is off. The render stays in `target/`; it is not committed and not pushed.

- [ ] **Step 3: Say in the spec what this step learned**

In `docs/superpowers/specs/2026-10-03-settings-general-design.md`:

- "The window": `SettingsDialog` has `resetting`; the sheet takes none of the terminal screen's keys (its controls are reached with Tab; Escape answers the reset question before it closes the window).
- "The file's actions": `Event::SettingsFileOpened { with, result }` and `Opened`; Export's dialog title is "Export settings".
- "Delivery", step 3: drop the sentence that says the ways in are offered only in the terminal look until step 4.
- "Errors and edge cases": a notice raised while the sheet is open (a failed save, a file manager that did not start) shows in the app's notice bar under the dimmed window, and can be dismissed once the sheet closes. The terminal screen covers the bar and has a band of its own.

Check the last one against what the sheet does before writing it: if the notice bar cannot be seen behind the sheet in a window of ordinary size, the sheet needs the band too, and that is a task to add here, not a limit to write down.

- [ ] **Step 4: Run all four checks**

Expected: PASS.

- [ ] **Step 5: Stage for the controller**

Two commits: `src/shots.rs` ("Render the Settings window among the shots"), the spec ("Say in the spec what the Settings window learned").

---

## By hand, with a window

No agent's session can open the app's window. These are for the user, and go in the pull request's test plan unchecked:

- Standard and macOS looks: `Mod+,` opens the sheet; each control changes an open grid at once; Tab reaches every control and each shows its focus ring; Escape closes.
- The menu lists a page size set by hand in the file (say 250) in its place.
- Reveal opens the file manager where the file is (Finder selects it on macOS, Explorer on Windows, also with a space or a comma in the path); with the file deleted first, it is written and then shown.
- Export… asks where to save and writes the canonical text there; Cancel writes nothing.
- Reset to defaults asks, Cancel leaves everything, Reset puts the four options back and leaves `show_system_schemas` and the SQL limit as they were.
- A line the app cannot read, added in an editor while the sheet is open: the footer counts it.
- macOS: "Settings…" is in the app menu under About and shows `⌘,`; the item and the key each open the window once (the other plan's by-hand list has the rest).
- A window shorter than the sheet: the rows scroll and the footer stays.

## Self-review

- Spec coverage: the sheet (Tasks 5, 6), the toggle (4), Reveal, Export, Reset and the count of ignored lines (1, 2, 3, 6), the ways in for every look and the `SHORTCUTS` line (5). The macOS menu item (7) is built by its own plan and delivered in the same pull request; nothing of step 4 in "Delivery" is left out.
- Names used across tasks: `Opened::{Editor, Folder}`, `Command::RevealSettingsFile { path, text }`, `Action::{RevealSettingsFile, ExportSettings, ResetSettings, ConfirmResetSettings(bool)}`, `SettingsDialog::resetting`, `widgets::toggle(ui, on, name, palette)`, `sheet::show(app, ctx, resetting, actions)`.
- Known unknowns, each flagged where it stands: the sheet's layout code is a shape to follow, not text to paste (the helpers' signatures decide), the toggle's test needs the order of frames a neighbouring widget test uses, and nothing under `cfg(target_os = "macos")` or `cfg(windows)` can be run by an agent.

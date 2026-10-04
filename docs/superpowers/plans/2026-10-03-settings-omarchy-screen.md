# Settings: the Omarchy screen Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** In the terminal look, `ctrl+,` opens a Settings screen over the whole window: the General tab's four options under a cursor moved and changed with keys, the settings file shown beside them with the cursor's line highlighted and the ignored lines in red, and a key that opens the file in the editor.

**Architecture:** The options become values (`OptionId`, `OptionValue` in `src/settings.rs`) so that keys, clicks and the file all change a setting through one action, `SetOption`, and `App::change_settings`. `Dialog::Settings` holds only the cursor. The screen is `src/ui/settings/`: `mod.rs` owns what both layouts share (the options' words, the ways in, the keys), `terminal.rs` draws the Omarchy screen, `file_pane.rs` colours the file's text. The backend opens the file in the editor. Before a held key can change a setting, the backend learns to tell the app which texts of the file are the app's own writes, so that one of them arriving late is never taken for a change from outside.

**Tech Stack:** Rust 2024, egui (crmne fork, 0.36), tokio. Headless UI tests through `src/testing.rs` with `harness.set_look(Look::omarchy())`.

**Spec:** `docs/superpowers/specs/2026-10-03-settings-general-design.md`, sections "The window" (Omarchy, Opening), "The file's actions" (the editor) and "Errors and edge cases". This plan is step 3 of its four. Step 4 draws the macOS and Windows window and adds the footer's Reveal, Export and Reset, the macOS menu item and the `SHORTCUTS` line.

**Design:** the user's Design canvas, artboard "Omarchy – Settings, General". It is not in the repository and is not to be added. The values this plan needs are written out below.

---

## Ground rules (read first)

- Work in `/home/igor/Work/tabletist/.claude/worktrees/settings-general-tab-957dcf`, on a branch of its own for step 3, cut from the tip of step 2's (`claude/settings-live-reload`), or from `main` once that is merged. Start from a clean working tree.
- `cargo` through mise is broken here: always run `~/.cargo/bin/cargo`, always with `--locked`. Never point `CARGO_TARGET_DIR` at `/tmp`, and never at another checkout's `target/`. `Cargo.toml` and `Cargo.lock` do not change in this step.
- One topic per commit, signed: plain `git commit -S`. Never set `SSH_AUTH_SOCK` in a git command. If signing fails with "agent refused operation", 1Password has locked: do **not** commit unsigned. Stage the task's files, run `git write-tree`, note the tree id and the commit message in your report, and go on. End every message with `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.
- Never push.
- Every commit passes all four checks of `AGENTS.md`: `cargo fmt --all --check`, `cargo clippy --locked --workspace --all-targets -- -D warnings`, `cargo test --locked --workspace --all-targets`, `RUSTDOCFLAGS='-D warnings' cargo doc --locked --workspace --no-deps`. A doc comment links only to what exists at that commit.
- No em dashes anywhere (code, comments, docs, commit messages).
- Tests are behavioural: a key moves the cursor, a key changes a setting and saves it, a line is painted in the colour of an error. Never a design or pixel conformance test: do not assert the design's sizes, offsets or colours as numbers. Design material is never committed. Fixtures use neutral names.
- Views draw text only through `TextRole`s (`src/typography.rs`) and the `widgets` helpers; a view never names a font, a family or a size.
- Keyboard focus is drawn in `src/ui/focus.rs`. A view never paints a control's focus ring. A pane marks where its arrows are by itself: the screen's cursor is that mark.
- Views do not mutate application state: they push `Action`s, and `App::apply` changes the state. Disk and process work runs on the backend.
- Every string a user reads goes through `gettext`, and through `look.label(..)` where the terminal look lower-cases it (see `Skin::say` in `src/ui/connect_dialog/mod.rs`).
- Comments say why, in the voice of the code around them. `clippy::unwrap_used` warns outside tests.
- This session has no display: nothing here can be checked in a real window. Say so in the report; do not claim a by-hand check.

## What exists (from steps 1 and 2)

`src/settings.rs`: `Settings { page_size, timestamps, group_digits, value_tags, show_system_schemas, custom_theme, sql_limit, sql_timeout_secs }`, `Timestamps { Second, Full }` with `name()`, `Key` with `path()`, `Settings::to_toml`, `Settings::from_toml -> Loaded`, `Loaded::of`, `Loaded::into_parts`, `SettingsFile { text, invalid, lines, live }`.

`src/app.rs`: `App::settings`, `App::settings_file`, `App::apply_settings(new)` (effects, no write), `App::change_settings(|settings| ..)` (applied and written as the canonical text; a change that changes nothing writes nothing), private `save_settings`, `Event::SettingsFile { text }` applied unless it equals `settings_file.text`, `Event::SettingsWatch { live }`.

`src/backend.rs`: `Command::WatchSettings`, `watch_settings`, `concerns`, `settled`, `read_file`, and the reader `read_settings(path, changes, outbox, read)`, which takes the function that reads the file (the tests have one that fails) and keeps the text it sent last (`sent`); `Saves::write`, which writes a `StateFile`; `Command::Save` with `StateFile::Settings`, `Event::Saved { path, result }`.

`src/model.rs`: `Dialog { Connection, Password, HostKey, QuickOpen, Help, About }`, `Action::{ShowHelp, ShowAbout, CloseDialog}`.

The terminal look's patterns to follow: `src/ui/connect_dialog/terminal.rs` (a header band, rows of a label and a value, `[x]` checks, a footer whose key hints are buttons too: `ButtonSpec::new(name).hidden_at(ui, rect)`), `src/ui/widgets.rs` (`modal`, `key_hints`, `paint_text`, `paint_label`, `measure`, `hline`, `vline`, `selection`, `section_label`, `Text`), `src/ui/help.rs` (a dialog that closes on Escape), `src/ui/keys.rs` (`handle`, the one place global shortcuts are read).

## File structure

| File | Change |
|---|---|
| `src/settings.rs` | `SettingsFile::saved` (the text the app last asked to be written). `OptionId`, `OptionValue`, `PAGE_SIZES`, and the three ways a key changes an option. |
| `src/model.rs` | `Dialog::Settings`, `SettingsDialog`, five actions. |
| `src/app.rs` | The actions; what to do with a write of its own that comes back; the editor action and its event. |
| `src/backend.rs` | The reader tells an own write from another's (`Event::SettingsFile { own }`). `Command::EditSettingsFile`, `Event::SettingsFileOpened`, the editor's program. |
| `src/ui/settings/mod.rs` | New. What both layouts share: the options' words, `show`, the keys. |
| `src/ui/settings/terminal.rs` | New. The Omarchy screen. |
| `src/ui/settings/file_pane.rs` | New. The file's text, coloured. |
| `src/ui/keys.rs` | `Mod+,`. |
| `src/ui/help.rs` | The Settings button. |
| `src/ui/mod.rs` | `pub mod settings;`, the call in `show`, UI tests. |
| `src/shots.rs` | A scene of the screen, for review by eye (feature `shots`). |

---

### Task 1: A write of the app's own is known for what it is

**Files:**
- Modify: `src/backend.rs` (`Saves`, `Saves::write`, `read_file`, `read_settings`, `watch_settings`, `Worker::handle`, `Event::SettingsFile`, the reader's tests), `src/settings.rs` (`SettingsFile`), `src/app.rs` (`save_settings`, the `Event::SettingsFile` arm), and every place that builds an `Event::SettingsFile`

Today the app drops a text from the disk only if it equals the text it holds. Two quick changes in the app can be read between their two writes: the first text then comes back, differs from the second, and is applied for a moment. A third change made in that moment is built on the older settings and loses the second. A menu cannot be clicked that fast; a key held down on an option can.

The backend knows exactly what it wrote. It says so: `Event::SettingsFile { text, own }`, where `own` means "this is, byte for byte, what the backend itself wrote to the file last". The app then has a rule with no guess in it:

| The text | The app |
|---|---|
| is the one it holds | drops it (an echo, or a save that changed nothing) |
| is another's (`own` false) | applies it |
| is its own, and the one it last asked to be written | applies it: its save landed over a change from outside that it had applied in between, and the disk is what counts |
| is its own, and older than that | drops it: read between two of its writes; the newest is still to come |

Whether a text on disk is the backend's own is decided under one lock, held by the writer while it writes and records the text, and by the reader while it reads and compares. Without it a write could land between the reader's read and its comparison, and the app's own older text would look like someone else's.

- [ ] **Step 1: Write the failing tests**

`src/backend.rs` tests. First adapt the paused-clock helper `reader(answer)` (it builds a reader whose reads are answered by `answer(n)`): `answer` now returns `std::io::Result<Found>`. The existing tests that use it wrap their bytes (`Found { bytes, own: false }`) and keep every assertion. Then add:

```rust
    #[test]
    fn a_write_of_the_backends_own_is_sent_as_its_own() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.toml");
        let (mut backend, live) = watching(&path);
        assert!(live);
        // Another's save: the app is told it is not its own.
        crate::util::write_atomic(&path, b"[data]\npage_size = 500\n").unwrap();
        assert_eq!(
            files_until(&mut backend, "[data]\npage_size = 500\n"),
            vec![("[data]\npage_size = 500\n".to_owned(), false)]
        );
        // The app's save, through the backend.
        let settings = crate::settings::Settings {
            page_size: 100,
            ..Default::default()
        };
        backend.send(Command::Save {
            path: path.clone(),
            file: StateFile::Settings(settings.clone()),
        });
        assert_eq!(
            files_until(&mut backend, &settings.to_toml()),
            vec![(settings.to_toml(), true)]
        );
        // The text before it, put back by someone else, is theirs.
        crate::util::write_atomic(&path, b"[data]\npage_size = 500\n").unwrap();
        assert_eq!(
            files_until(&mut backend, "[data]\npage_size = 500\n"),
            vec![("[data]\npage_size = 500\n".to_owned(), false)]
        );
    }
```

with a helper beside `texts_until` that keeps the flag and skips the `Saved` event of the save:

```rust
    /// The settings file's texts the backend sends until `expected` comes,
    /// that one included, each with whether it is the backend's own write;
    /// empty when it never does.
    fn files_until(backend: &mut Backend, expected: &str) -> Vec<(String, bool)> {
        let deadline = std::time::Instant::now() + WAIT;
        let mut files = Vec::new();
        while std::time::Instant::now() < deadline {
            if let Some(Event::SettingsFile { text, own }) =
                backend.wait(Duration::from_millis(200))
            {
                let done = text == expected;
                files.push((text, own));
                if done {
                    return files;
                }
            }
        }
        Vec::new()
    }
```

`texts_until` stays for the tests that have it; it and the paused-clock helper `texts(events)` now match `Event::SettingsFile { text, .. }`.

`src/app.rs` tests, after `the_apps_own_text_coming_back_is_not_read_again`:

```rust
    /// The settings file's text coming from the disk.
    fn from_disk(text: &str, own: bool) -> Action {
        Action::Backend(Event::SettingsFile {
            text: text.into(),
            own,
        })
    }

    #[test]
    fn an_older_write_of_the_apps_own_coming_back_late_undoes_nothing() {
        let (mut app, _dir) = app();
        app.change_settings(|settings| settings.sql_limit = 100);
        let first = app.settings_file.text.clone();
        app.change_settings(|settings| settings.page_size = 500);
        let second = app.settings_file.text.clone();
        let saved = settings_saves(&app).len();
        // The disk was read between the two writes.
        app.apply(from_disk(&first, true));
        assert_eq!(app.settings.page_size, 500, "the newer change stays");
        assert_eq!(app.settings_file.text, second);
        // Then the newest comes back.
        app.apply(from_disk(&second, true));
        assert_eq!(app.settings.page_size, 500);
        assert_eq!(settings_saves(&app).len(), saved);
    }

    #[test]
    fn the_same_older_text_from_someone_else_is_a_change() {
        let (mut app, _dir) = app();
        app.change_settings(|settings| settings.sql_limit = 100);
        let first = app.settings_file.text.clone();
        app.change_settings(|settings| settings.page_size = 500);
        // Not the backend's write: someone put that text there.
        app.apply(from_disk(&first, false));
        assert_eq!(app.settings.page_size, Settings::DEFAULT_PAGE_SIZE);
        assert_eq!(app.settings_file.text, first);
    }

    #[test]
    fn the_apps_save_landing_over_a_change_from_outside_is_what_the_disk_has() {
        let (mut app, _dir) = app();
        app.change_settings(|settings| settings.sql_limit = 100);
        let ours = app.settings_file.text.clone();
        // An edit from outside is read before the app's save lands...
        app.apply(from_disk("[data]\npage_size = 500\n", false));
        assert_eq!(app.settings.page_size, 500);
        assert_eq!(app.settings.sql_limit, 1_000);
        let saved = settings_saves(&app).len();
        // ...and then it lands, over the edit.
        app.apply(from_disk(&ours, true));
        assert_eq!(app.settings.sql_limit, 100);
        assert_eq!(app.settings.page_size, Settings::DEFAULT_PAGE_SIZE);
        assert_eq!(app.settings_file.text, ours);
        assert_eq!(settings_saves(&app).len(), saved, "nothing is written back");
    }
```

Every other place that builds an `Event::SettingsFile { text }` (the step 2 tests in `src/app.rs` and `src/ui/mod.rs`) gains `own: false`: those are edits from outside.

- [ ] **Step 2: Run them to see them fail**

Run: `~/.cargo/bin/cargo test --locked -p tabletist --lib an_older_write_of_the_apps_own`
Expected: does not compile, "no field `own`".

- [ ] **Step 3: The backend**

`src/backend.rs`.

`Event::SettingsFile` becomes:

```rust
    /// The settings file changed on disk: its text. `own` when it is, byte
    /// for byte, what the backend itself wrote there last: the app's own
    /// save coming back, which it must not take for someone else's change.
    SettingsFile { text: String, own: bool },
```

A cell shared by the writer and the reader, beside `Saves`:

```rust
/// The text the backend wrote to the settings file last, if it wrote one.
/// Locked while it is written and while the reader reads the file and
/// compares: a write cannot land between the reader's read and its
/// comparison and make the app's own older text look like someone else's.
type Written = Arc<Mutex<Option<String>>>;
```

`Saves` gains `settings_written: Written` (it derives `Default`), and `Saves::write` writes the settings under the lock:

```rust
            let result = match &file {
                // Known to the settings file's reader for the app's own.
                StateFile::Settings(settings) => {
                    let mut written = lock(&self.settings_written);
                    let result = file.save(path);
                    // After a write that failed nothing on the disk is
                    // known to be the app's: the text there may be an
                    // older write of its own, but the app has moved on
                    // from it, and someone putting it back must be heard.
                    *written = result.is_ok().then(|| settings.to_toml());
                    result
                }
                _ => file.save(path),
            }
            .map_err(|error| error.to_string());
```

(`Settings::save` writes `to_toml()`: the two are the same text.)

What a read finds:

```rust
/// The settings file as the reader found it.
struct Found {
    bytes: Vec<u8>,
    /// It is what the backend wrote there last.
    own: bool,
}
```

`read_file` takes the cell and holds it across the read and the comparison:

```rust
async fn read_file(path: PathBuf, written: Written) -> std::io::Result<Found> {
    let read = move || {
        let written = lock(&written);
        let bytes = std::fs::read(path)?;
        let own = written
            .as_deref()
            .is_some_and(|text| text.as_bytes() == bytes.as_slice());
        Ok(Found { bytes, own })
    };
    match tokio::task::spawn_blocking(read).await {
        Ok(found) => found,
        Err(error) => Err(std::io::Error::other(error)),
    }
}
```

`read_settings`: its `F` is `Future<Output = std::io::Result<Found>>`; the loop carries the `Found` out instead of the bytes, and its end becomes:

```rust
        match String::from_utf8(found.bytes) {
            Ok(text) if sent.as_deref() == Some(text.as_str()) => {}
            Ok(text) => {
                sent = Some(text.clone());
                outbox.emit(Event::SettingsFile {
                    text,
                    own: found.own,
                });
            }
            Err(_) => log::warn!("{} is not text; it is not read", path.display()),
        }
```

`watch_settings` takes the cell (`written: Written`) and passes the reader `move |path| read_file(path, Arc::clone(&written))`; `Worker::handle` gives it `Arc::clone(&self.saves.settings_written)`.

- [ ] **Step 4: The app**

`src/settings.rs`, `SettingsFile` gains:

```rust
    /// The text the app last asked to be written, if it asked: what tells
    /// its newest write, coming back from the disk, from an older one.
    pub saved: Option<String>,
```

`Loaded::into_parts` fills it with `None`. Any `SettingsFile { .. }` literal in a test needs it; the compiler names them.

`src/app.rs`, `save_settings` remembers what it sends (it is the one place that sends it):

```rust
    fn save_settings(&mut self) {
        self.settings_file.saved = Some(self.settings.to_toml());
        self.backend.send(Command::Save {
            path: self.dirs.settings_file(),
            file: StateFile::Settings(self.settings.clone()),
        });
    }
```

`change_settings` builds a new `SettingsFile` before it calls `save_settings`; keep `saved` across that as it keeps `live` (`let saved = self.settings_file.saved.take();` and `SettingsFile { live, saved, ..file }`).

The `Event::SettingsFile` arm:

```rust
            Event::SettingsFile { text, own } => {
                // What the app holds already: its own write coming back,
                // or a save that changed nothing.
                if text == self.settings_file.text {
                    return;
                }
                // A write of its own that is not what it holds. An older
                // one was read between two of its writes: the newest is
                // still to come, and applying this would undo the change
                // made since. The newest itself landed over a change from
                // outside that was applied in between: the disk has it.
                if own && self.settings_file.saved.as_deref() != Some(text.as_str()) {
                    return;
                }
                let loaded = Settings::from_toml(&text);
                loaded.warn_invalid(&self.dirs.settings_file());
                let live = self.settings_file.live;
                let saved = self.settings_file.saved.take();
                let (settings, file) = loaded.into_parts();
                // The file as its writer left it: not written back, so a
                // line that was ignored stays where they can see it.
                self.settings_file = SettingsFile {
                    live,
                    saved,
                    ..file
                };
                self.apply_settings(settings);
            }
```

- [ ] **Step 5: Run the tests**

```bash
~/.cargo/bin/cargo test --locked -p tabletist --lib backend::tests::
~/.cargo/bin/cargo test --locked -p tabletist --lib app::tests::
~/.cargo/bin/cargo test --locked -p tabletist --lib settings::
~/.cargo/bin/cargo test --locked --workspace --all-targets
```

Expected: PASS, with the step 2 tests of the reader and of the app unchanged but for `own: false` and the helper's `Found`. Run the backend tests five times: they wait on the real file system.

- [ ] **Step 6: Commit**

```bash
git add src/backend.rs src/settings.rs src/app.rs src/ui/mod.rs
git commit -S -m "Tell the app which texts of its settings file are its own writes

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 2: The options as values

**Files:**
- Modify: `src/settings.rs`

The screen's keys, a click and the file all say the same thing: this option, that value. This task gives that a type, and the three ways a key picks the value.

- [ ] **Step 1: Write the failing tests**

In `src/settings.rs`'s tests:

```rust
    #[test]
    fn an_option_is_read_and_set_as_a_value() {
        let mut settings = Settings::default();
        for (option, value) in [
            (OptionId::PageSize, OptionValue::PageSize(500)),
            (OptionId::Timestamps, OptionValue::Timestamps(Timestamps::Full)),
            (OptionId::GroupDigits, OptionValue::GroupDigits(true)),
            (OptionId::ValueTags, OptionValue::ValueTags(false)),
        ] {
            assert_eq!(value.option(), option);
            assert_ne!(settings.value(option), value);
            value.set(&mut settings);
            assert_eq!(settings.value(option), value);
        }
        assert_eq!(settings.page_size, 500);
        assert_eq!(settings.timestamps, Timestamps::Full);
        assert!(settings.group_digits);
        assert!(!settings.value_tags);
    }

    #[test]
    fn every_option_is_stored_under_its_own_key_and_starts_at_the_default() {
        let keys: Vec<Key> = OptionId::ALL.iter().map(|option| option.key()).collect();
        assert_eq!(
            keys,
            vec![
                Key::PageSize,
                Key::Timestamps,
                Key::GroupDigits,
                Key::ValueTags
            ]
        );
        let defaults = Settings::default();
        for option in OptionId::ALL {
            assert_eq!(option.default_value(), defaults.value(option));
        }
    }

    #[test]
    fn the_page_size_steps_through_its_choices_and_stops_at_the_ends() {
        let step = |size: u32, forward: bool| {
            let settings = Settings {
                page_size: size,
                ..Settings::default()
            };
            settings.stepped(OptionId::PageSize, forward)
        };
        assert_eq!(step(300, true), OptionValue::PageSize(500));
        assert_eq!(step(300, false), OptionValue::PageSize(100));
        assert_eq!(step(100, false), OptionValue::PageSize(100));
        assert_eq!(step(5_000, true), OptionValue::PageSize(5_000));
        // A size set by hand is between two choices: it steps to the next.
        assert_eq!(step(250, true), OptionValue::PageSize(300));
        assert_eq!(step(250, false), OptionValue::PageSize(100));
        assert_eq!(step(9_000, true), OptionValue::PageSize(9_000));
        assert_eq!(step(9_000, false), OptionValue::PageSize(5_000));
    }

    #[test]
    fn an_option_of_two_values_steps_to_the_side_it_is_drawn_on() {
        let settings = Settings::default();
        // Timestamps: to the second on the left, full on the right.
        assert_eq!(
            settings.stepped(OptionId::Timestamps, true),
            OptionValue::Timestamps(Timestamps::Full)
        );
        assert_eq!(
            settings.stepped(OptionId::Timestamps, false),
            OptionValue::Timestamps(Timestamps::Second)
        );
        // Numbers: grouped on the left, plain on the right.
        assert_eq!(
            settings.stepped(OptionId::GroupDigits, false),
            OptionValue::GroupDigits(true)
        );
        assert_eq!(
            settings.stepped(OptionId::GroupDigits, true),
            OptionValue::GroupDigits(false)
        );
        // A check: off to the left, on to the right.
        assert_eq!(
            settings.stepped(OptionId::ValueTags, false),
            OptionValue::ValueTags(false)
        );
        assert_eq!(
            settings.stepped(OptionId::ValueTags, true),
            OptionValue::ValueTags(true)
        );
    }

    #[test]
    fn space_flips_an_option_of_two_values_and_leaves_the_page_size() {
        let settings = Settings::default();
        assert_eq!(
            settings.flipped(OptionId::ValueTags),
            Some(OptionValue::ValueTags(false))
        );
        assert_eq!(
            settings.flipped(OptionId::GroupDigits),
            Some(OptionValue::GroupDigits(true))
        );
        assert_eq!(
            settings.flipped(OptionId::Timestamps),
            Some(OptionValue::Timestamps(Timestamps::Full))
        );
        assert_eq!(settings.flipped(OptionId::PageSize), None);
    }
```

- [ ] **Step 2: Run them to see them fail**

Run: `~/.cargo/bin/cargo test --locked -p tabletist --lib settings::`
Expected: does not compile, "cannot find type `OptionId`".

- [ ] **Step 3: Implement**

`src/settings.rs`, after `Key` and its impl:

```rust
/// An option the Settings window shows: one row of it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OptionId {
    PageSize,
    Timestamps,
    GroupDigits,
    ValueTags,
}

impl OptionId {
    /// In the order the window has them.
    pub const ALL: [OptionId; 4] = [
        Self::PageSize,
        Self::Timestamps,
        Self::GroupDigits,
        Self::ValueTags,
    ];

    /// The key of the file the option is stored under.
    pub fn key(self) -> Key {
        match self {
            Self::PageSize => Key::PageSize,
            Self::Timestamps => Key::Timestamps,
            Self::GroupDigits => Key::GroupDigits,
            Self::ValueTags => Key::ValueTags,
        }
    }

    /// What the option is before anyone sets it.
    pub fn default_value(self) -> OptionValue {
        Settings::default().value(self)
    }
}

/// An option with a value: what a key, a click or a reset sets.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OptionValue {
    PageSize(u32),
    Timestamps(Timestamps),
    GroupDigits(bool),
    ValueTags(bool),
}

impl OptionValue {
    pub fn option(self) -> OptionId {
        match self {
            Self::PageSize(_) => OptionId::PageSize,
            Self::Timestamps(_) => OptionId::Timestamps,
            Self::GroupDigits(_) => OptionId::GroupDigits,
            Self::ValueTags(_) => OptionId::ValueTags,
        }
    }

    /// Puts the value in `settings`.
    pub fn set(self, settings: &mut Settings) {
        match self {
            Self::PageSize(size) => settings.page_size = size,
            Self::Timestamps(choice) => settings.timestamps = choice,
            Self::GroupDigits(on) => settings.group_digits = on,
            Self::ValueTags(on) => settings.value_tags = on,
        }
    }
}
```

In `impl Settings`:

```rust
    /// The page sizes the Settings window offers.
    pub const PAGE_SIZES: [u32; 5] = [100, 300, 500, 1_000, 5_000];

    /// What `option` is set to.
    pub fn value(&self, option: OptionId) -> OptionValue {
        match option {
            OptionId::PageSize => OptionValue::PageSize(self.page_size),
            OptionId::Timestamps => OptionValue::Timestamps(self.timestamps),
            OptionId::GroupDigits => OptionValue::GroupDigits(self.group_digits),
            OptionId::ValueTags => OptionValue::ValueTags(self.value_tags),
        }
    }

    /// The value one step from `option`'s, towards the right (`forward`)
    /// or the left, as the window draws its choices: the next page size
    /// (none past the ends), the segment on that side, a check on to the
    /// right and off to the left.
    pub fn stepped(&self, option: OptionId, forward: bool) -> OptionValue {
        match option {
            OptionId::PageSize => {
                let size = self.page_size;
                let next = if forward {
                    Self::PAGE_SIZES.into_iter().find(|choice| *choice > size)
                } else {
                    Self::PAGE_SIZES
                        .into_iter()
                        .rev()
                        .find(|choice| *choice < size)
                };
                OptionValue::PageSize(next.unwrap_or(size))
            }
            OptionId::Timestamps => OptionValue::Timestamps(if forward {
                Timestamps::Full
            } else {
                Timestamps::Second
            }),
            // Grouped is the left of the two.
            OptionId::GroupDigits => OptionValue::GroupDigits(!forward),
            OptionId::ValueTags => OptionValue::ValueTags(forward),
        }
    }

    /// The other value of an option that has two; `None` for one with more.
    pub fn flipped(&self, option: OptionId) -> Option<OptionValue> {
        match option {
            OptionId::PageSize => None,
            OptionId::Timestamps => Some(OptionValue::Timestamps(match self.timestamps {
                Timestamps::Second => Timestamps::Full,
                Timestamps::Full => Timestamps::Second,
            })),
            OptionId::GroupDigits => Some(OptionValue::GroupDigits(!self.group_digits)),
            OptionId::ValueTags => Some(OptionValue::ValueTags(!self.value_tags)),
        }
    }
```

- [ ] **Step 4: Run the tests**

Run: `~/.cargo/bin/cargo test --locked -p tabletist --lib settings::`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/settings.rs
git commit -S -m "Give the settings their options as values a key can step

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 3: The dialog and its actions

**Files:**
- Modify: `src/model.rs` (`Dialog`, `Action`), `src/app.rs` (`App::apply`)

- [ ] **Step 1: Write the failing tests**

In `src/app.rs`'s tests:

```rust
    fn settings_row(app: &App) -> Option<usize> {
        match &app.dialog {
            Some(Dialog::Settings(dialog)) => Some(dialog.row),
            _ => None,
        }
    }

    #[test]
    fn settings_open_over_nothing_or_the_shortcuts_and_close() {
        let (mut app, _dir) = app();
        app.apply(Action::ShowSettings);
        assert_eq!(settings_row(&app), Some(0));
        // A second ask changes nothing: the cursor stays where it is.
        app.apply(Action::MoveSettingsRow(2));
        app.apply(Action::ShowSettings);
        assert_eq!(settings_row(&app), Some(2));
        app.apply(Action::CloseDialog);
        assert!(app.dialog.is_none());
        // The shortcuts dialog offers it, and gives way to it.
        app.apply(Action::ShowHelp);
        app.apply(Action::ShowSettings);
        assert_eq!(settings_row(&app), Some(0));
        // Another dialog does not.
        app.apply(Action::CloseDialog);
        app.apply(Action::ShowAbout);
        app.apply(Action::ShowSettings);
        assert!(matches!(app.dialog, Some(Dialog::About)));
    }

    #[test]
    fn the_settings_cursor_stays_among_the_options() {
        use crate::settings::OptionId;
        let (mut app, _dir) = app();
        app.apply(Action::ShowSettings);
        app.apply(Action::MoveSettingsRow(-1));
        assert_eq!(settings_row(&app), Some(0));
        app.apply(Action::MoveSettingsRow(1));
        assert_eq!(settings_row(&app), Some(1));
        app.apply(Action::MoveSettingsRow(99));
        assert_eq!(settings_row(&app), Some(OptionId::ALL.len() - 1));
        app.apply(Action::SelectSettingsRow(2));
        assert_eq!(settings_row(&app), Some(2));
        app.apply(Action::SelectSettingsRow(99));
        assert_eq!(settings_row(&app), Some(2), "no such row: left where it was");
        // Without the screen the cursor's actions do nothing.
        app.apply(Action::CloseDialog);
        app.apply(Action::MoveSettingsRow(1));
        assert!(app.dialog.is_none());
    }

    #[test]
    fn setting_an_option_changes_it_and_saves() {
        use crate::settings::{OptionValue, Timestamps};
        let (mut app, _dir) = app();
        app.apply(Action::SetOption(OptionValue::Timestamps(Timestamps::Full)));
        assert_eq!(app.settings.timestamps, Timestamps::Full);
        assert_eq!(settings_saves(&app).len(), 1);
        // The value it already has is not written again.
        app.apply(Action::SetOption(OptionValue::Timestamps(Timestamps::Full)));
        assert_eq!(settings_saves(&app).len(), 1);
        // A page size out of range comes into it, as from the file.
        app.apply(Action::SetOption(OptionValue::PageSize(5)));
        assert_eq!(app.settings.page_size, Settings::MIN_PAGE_SIZE);
    }
```

- [ ] **Step 2: Run them to see them fail**

Run: `~/.cargo/bin/cargo test --locked -p tabletist --lib settings_open_over_nothing`
Expected: does not compile, "no variant named `ShowSettings`".

- [ ] **Step 3: Implement**

`src/model.rs`, `Dialog` gains:

```rust
    /// The Settings window.
    Settings(Box<SettingsDialog>),
```

and, after `QuickOpen`:

```rust
/// The Settings window while it is open. The settings themselves are the
/// app's: every change applies at once, so the window holds nothing of
/// them.
#[derive(Debug, Default)]
pub struct SettingsDialog {
    /// The option the keys act on, as an index into
    /// [`crate::settings::OptionId::ALL`].
    pub row: usize,
}
```

`Action` gains, after `ShowAbout`:

```rust
    /// Show the Settings window.
    ShowSettings,
    /// Move the Settings window's cursor by this many options.
    MoveSettingsRow(isize),
    /// Put the Settings window's cursor on this option.
    SelectSettingsRow(usize),
    /// Set an option. Applied and saved at once.
    SetOption(crate::settings::OptionValue),
```

`src/app.rs`, `App::apply`, beside `Action::ShowAbout`:

```rust
            Action::ShowSettings => {
                // The shortcuts dialog offers it, and gives way to it. Asked
                // for while it is open, it stays as it is.
                if matches!(self.dialog, None | Some(Dialog::Help)) {
                    self.dialog = Some(Dialog::Settings(Box::default()));
                }
            }
            Action::MoveSettingsRow(by) => {
                if let Some(Dialog::Settings(dialog)) = &mut self.dialog {
                    let last = crate::settings::OptionId::ALL.len() - 1;
                    dialog.row = dialog.row.saturating_add_signed(by).min(last);
                }
            }
            Action::SelectSettingsRow(row) => {
                if let Some(Dialog::Settings(dialog)) = &mut self.dialog
                    && row < crate::settings::OptionId::ALL.len()
                {
                    dialog.row = row;
                }
            }
            Action::SetOption(value) => self.change_settings(|settings| value.set(settings)),
```

Every `match` on `Dialog` that lists its variants gets the new one; the compiler names them. `SetOption` works with or without the window open: it is what a menu would send too.

- [ ] **Step 4: Run the tests**

Run: `~/.cargo/bin/cargo test --locked -p tabletist --lib app::tests::`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/model.rs src/app.rs
git commit -S -m "Give the Settings window its dialog and its actions

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 4: The screen, its ways in and its keys

**Files:**
- Create: `src/ui/settings/mod.rs`, `src/ui/settings/terminal.rs`
- Modify: `src/ui/mod.rs` (`pub mod settings;`, the call in `show`), `src/ui/keys.rs` (`Mod+,`), `src/ui/help.rs` (the button)
- Test: `src/ui/mod.rs`

The screen as the artboard has it, top to bottom. Sizes are in points and are what the artboard gives; they belong in the view, never in a test.

- **The whole window.** The screen covers the app's window: the window colour (`palette.window`), nothing of the workspace shows or takes a click.
- **Header**, 40 tall, on `palette.panel`, a 1 pt rule (`palette.outline`) under it. At the left, 14 in: `settings` in `TextRole::OScreenTitle`, then 12 on, `general` in the dim colour. At the right, 14 in: `changes apply right away · ctrl+, opens this`, dim, with `ctrl+,` in the accent.
- **Nav**, 220 wide, a 1 pt rule at its right, 10 of space at its top: one item, `general`, 14 in, selected: the selection fill (`palette.selection`) with a 2 pt accent bar at its left edge.
- **Rows**, in what is left (or left of the file pane, Task 5), 8 of space at the top:
  - A section label `data`: 16 in, 12 over it and 4 under, dim (`widgets::section_label`).
  - One row per option, 30 tall, 16 in at the sides, in three columns: the label (280), the value (320), the hint (the rest), all `TextRole::OBody`.
  - The cursor's row has the selection fill and the 2 pt accent bar at its left, and its label is in the accent and starts with `▌`.
- **Footer**, 30 tall, on `palette.panel`, a 1 pt rule over it, 12 in: the key hints, 16 apart: `j/k` move, `h/l` change, `space` toggle, `R` reset option, `esc` close. (Task 6 adds `ctrl+e`.) Each hint that stands for an action is its button too, as in the connection dialog's footer.

The four rows, in the words `gettext` gets (the terminal look lower-cases them through `look.label`):

| Option | Label | Value | Hint |
|---|---|---|---|
| `PageSize` | `Rows per page` | The number as it is (`300`). On the cursor's row: `‹ 300 ›`, the chevrons in the accent, each dim when there is no choice on its side. | `table view` |
| `Timestamps` | `Timestamps` | Two segments, `second` and `full`. | A sample: `2026-01-12 09:14:03`, or `2026-01-12 09:14:03.482915` for full. |
| `GroupDigits` | `Numbers` | Two segments, `1,240.50` and `1240.50`. | `grouping on` or `grouping off`. |
| `ValueTags` | `Value tags` | `[x]` in the success colour, or `[ ]` dim. | Two sample tags, `print` and `ebook`: in the colours of the first two tag slots (`value_tags::terminal_slots`) when on, dim when off. |

A segment that is chosen is filled with the accent (its text in `palette.window`) on the cursor's row and outlined with a 1 pt rule (`palette.outline`) on the others; one that is not chosen is dim.

- [ ] **Step 1: Write the failing tests**

In `src/ui/mod.rs`'s tests (a `settings` block of its own, after the settings tests of step 2):

```rust
    /// A harness in the terminal look with the Settings screen open.
    fn settings_screen() -> Harness {
        let mut harness = Harness::new();
        harness.set_look(crate::theme::Look::omarchy());
        harness.press(egui::Key::Comma, egui::Modifiers::COMMAND);
        assert!(settings_open(&harness));
        harness
    }

    fn settings_open(harness: &Harness) -> bool {
        matches!(harness.app.dialog, Some(crate::model::Dialog::Settings(_)))
    }

    fn settings_cursor(harness: &Harness) -> usize {
        match &harness.app.dialog {
            Some(crate::model::Dialog::Settings(dialog)) => dialog.row,
            _ => panic!("the Settings screen is not open"),
        }
    }

    fn settings_saves(harness: &Harness) -> usize {
        harness
            .app
            .backend
            .sent
            .iter()
            .filter(|command| {
                matches!(
                    command,
                    Command::Save {
                        file: crate::backend::StateFile::Settings(_),
                        ..
                    }
                )
            })
            .count()
    }

    #[test]
    fn mod_comma_opens_the_settings_in_the_terminal_look_and_escape_closes_them() {
        let mut harness = settings_screen();
        assert!(harness.has("settings"));
        // The cursor starts on the first option.
        assert!(harness.painted_color("▌rows per page").is_some());
        // A second press leaves the screen as it is.
        harness.press(egui::Key::J, egui::Modifiers::NONE);
        harness.press(egui::Key::Comma, egui::Modifiers::COMMAND);
        assert_eq!(settings_cursor(&harness), 1);
        harness.press(egui::Key::Escape, egui::Modifiers::NONE);
        assert!(harness.app.dialog.is_none());
    }

    #[test]
    fn the_other_looks_have_no_settings_window_yet() {
        for look in [crate::theme::Look::standard(), crate::theme::Look::macos()] {
            let mut harness = Harness::new();
            harness.set_look(look);
            harness.press(egui::Key::Comma, egui::Modifiers::COMMAND);
            assert!(harness.app.dialog.is_none(), "{}", look.name);
            harness.frame(vec![egui::Event::Text("?".into())]);
            assert!(!harness.has("Settings"), "{}", look.name);
        }
    }

    #[test]
    fn the_shortcuts_dialog_opens_the_settings_in_the_terminal_look() {
        let mut harness = Harness::new();
        harness.set_look(crate::theme::Look::omarchy());
        harness.frame(vec![egui::Event::Text("?".into())]);
        harness.click("Settings");
        assert!(settings_open(&harness));
    }

    #[test]
    fn the_settings_cursor_moves_with_j_and_k_and_the_arrows() {
        let mut harness = settings_screen();
        assert_eq!(settings_cursor(&harness), 0);
        harness.press(egui::Key::J, egui::Modifiers::NONE);
        harness.press(egui::Key::ArrowDown, egui::Modifiers::NONE);
        assert_eq!(settings_cursor(&harness), 2);
        harness.press(egui::Key::K, egui::Modifiers::NONE);
        harness.press(egui::Key::ArrowUp, egui::Modifiers::NONE);
        harness.press(egui::Key::K, egui::Modifiers::NONE);
        assert_eq!(settings_cursor(&harness), 0);
        // The cursor's row says so.
        assert!(harness.painted_color("▌rows per page").is_some());
        assert!(harness.painted_color("timestamps").is_some());
    }

    #[test]
    fn h_and_l_change_the_cursors_option_and_save_it() {
        use crate::settings::Timestamps;
        let mut harness = settings_screen();
        // Rows per page.
        harness.press(egui::Key::L, egui::Modifiers::NONE);
        assert_eq!(harness.app.settings.page_size, 500);
        assert_eq!(settings_saves(&harness), 1);
        harness.press(egui::Key::ArrowLeft, egui::Modifiers::NONE);
        harness.press(egui::Key::H, egui::Modifiers::NONE);
        assert_eq!(harness.app.settings.page_size, 100);
        // At the end of the choices a step changes and writes nothing.
        let saved = settings_saves(&harness);
        harness.press(egui::Key::H, egui::Modifiers::NONE);
        assert_eq!(harness.app.settings.page_size, 100);
        assert_eq!(settings_saves(&harness), saved);
        // Timestamps.
        harness.press(egui::Key::J, egui::Modifiers::NONE);
        harness.press(egui::Key::ArrowRight, egui::Modifiers::NONE);
        assert_eq!(harness.app.settings.timestamps, Timestamps::Full);
        assert!(
            harness
                .painted_color("2026-01-12 09:14:03.482915")
                .is_some()
        );
        // Numbers: grouped is the left of the two.
        harness.press(egui::Key::J, egui::Modifiers::NONE);
        harness.press(egui::Key::H, egui::Modifiers::NONE);
        assert!(harness.app.settings.group_digits);
        assert!(harness.painted_color("grouping on").is_some());
    }

    #[test]
    fn space_flips_an_option_of_two_values_and_shift_r_resets_the_cursors() {
        let mut harness = settings_screen();
        // Space on the page size does nothing: it has more than two values.
        harness.press(egui::Key::Space, egui::Modifiers::NONE);
        assert_eq!(settings_saves(&harness), 0);
        for _ in 0..3 {
            harness.press(egui::Key::J, egui::Modifiers::NONE);
        }
        harness.press(egui::Key::Space, egui::Modifiers::NONE);
        assert!(!harness.app.settings.value_tags);
        assert!(harness.painted_color("[ ]").is_some());
        harness.press(egui::Key::R, egui::Modifiers::SHIFT);
        assert!(harness.app.settings.value_tags);
        assert!(harness.painted_color("[x]").is_some());
        // A plain r is not the key.
        harness.press(egui::Key::Space, egui::Modifiers::NONE);
        harness.press(egui::Key::R, egui::Modifiers::NONE);
        assert!(!harness.app.settings.value_tags);
    }

    #[test]
    fn a_click_moves_the_settings_cursor_and_a_click_on_a_value_sets_it() {
        use crate::settings::Timestamps;
        let mut harness = settings_screen();
        harness.click("Numbers");
        assert_eq!(settings_cursor(&harness), 2);
        harness.click("Timestamps: full");
        assert_eq!(harness.app.settings.timestamps, Timestamps::Full);
        assert_eq!(settings_cursor(&harness), 1, "the cursor follows the click");
        harness.click("Value tags: off");
        assert!(!harness.app.settings.value_tags);
        harness.click("Rows per page: more");
        assert_eq!(harness.app.settings.page_size, 500);
    }

    #[test]
    fn the_settings_screen_takes_the_keys_from_what_is_under_it() {
        let mut harness = Harness::new();
        harness.set_look(crate::theme::Look::omarchy());
        let tab = harness.connect_fake();
        let pane = harness.app.workspace(tab).unwrap().pane;
        harness.press(egui::Key::Comma, egui::Modifiers::COMMAND);
        // j is the tree's key in the workspace: here it is the screen's.
        harness.press(egui::Key::J, egui::Modifiers::NONE);
        assert_eq!(settings_cursor(&harness), 1);
        assert_eq!(harness.app.workspace(tab).unwrap().pane, pane);
    }
```

`Command` and `Harness` are imported in that module already; if `StateFile` is too, use it without its path. If a helper named `settings_saves` exists there, name this one for what it counts.

- [ ] **Step 2: Run them to see them fail**

Run: `~/.cargo/bin/cargo test --locked -p tabletist --lib mod_comma_opens_the_settings`
Expected: FAIL (the key does nothing, the dialog does not open).

- [ ] **Step 3: The ways in**

`src/ui/keys.rs`, in `handle`, **after** the big `ctx.input_mut(|input| { .. })` closure that reads the other shortcuts and before the `letters` call. Not inside that closure as written: a `ctx.input_mut` inside another takes egui's lock twice and hangs. (`handle` is not called at all while a dialog is open, so a second `Mod+,` over the open screen does nothing by itself, and the shortcuts dialog's way to the screen is its button.)

```rust
    // The Settings window, wherever the keyboard is. Until the other looks
    // have their window, only the terminal look opens one.
    if app.look.terminal
        && ctx.input_mut(|input| input.consume_key(Modifiers::COMMAND, Key::Comma))
    {
        actions.push(Action::ShowSettings);
    }
```

`src/ui/help.rs`, beside the About button, drawn in the terminal look only:

```rust
            if look.terminal && widgets::button(ui, &gettext(locale, "Settings"), &look).clicked() {
                actions.push(Action::ShowSettings);
            }
```

No line in `SHORTCUTS` yet: no look may list a key that does nothing there. It comes with step 4.

- [ ] **Step 4: What both layouts share**

`src/ui/settings/mod.rs`:

```rust
//! The Settings window: every option of the General tab, changed where it
//! is shown and saved at once. The terminal look draws a screen over the
//! whole window with its keys in the footer (`terminal.rs`); the other
//! looks get their window in a later step.

mod terminal;

use crate::app::App;
use crate::model::{Action, Dialog};
use crate::settings::{OptionId, OptionValue, Settings, Timestamps};

/// The words of an option's row: its label, and the hint beside its value.
/// English: the caller translates, and the terminal look lower-cases.
pub(super) fn label(option: OptionId) -> &'static str {
    match option {
        OptionId::PageSize => "Rows per page",
        OptionId::Timestamps => "Timestamps",
        OptionId::GroupDigits => "Numbers",
        OptionId::ValueTags => "Value tags",
    }
}

/// The values a click can set on `option`'s row, each with the words that
/// name it: what the row draws as segments or as a check, and what a
/// screen reader and the tests call them.
pub(super) fn choices(option: OptionId, settings: &Settings) -> Vec<(&'static str, OptionValue)> {
    match option {
        // Stepped, not listed: one way and the other.
        OptionId::PageSize => vec![
            ("fewer", settings.stepped(option, false)),
            ("more", settings.stepped(option, true)),
        ],
        OptionId::Timestamps => vec![
            ("second", OptionValue::Timestamps(Timestamps::Second)),
            ("full", OptionValue::Timestamps(Timestamps::Full)),
        ],
        OptionId::GroupDigits => vec![
            ("1,240.50", OptionValue::GroupDigits(true)),
            ("1240.50", OptionValue::GroupDigits(false)),
        ],
        OptionId::ValueTags => vec![
            ("off", OptionValue::ValueTags(false)),
            ("on", OptionValue::ValueTags(true)),
        ],
    }
}

/// The screen's keys, taken before it draws so that nothing under it and
/// no focused button in it reads them first.
fn keys(app: &App, ctx: &egui::Context, row: usize, actions: &mut Vec<Action>) {
    use egui::{Key, Modifiers};
    let Some(option) = OptionId::ALL.get(row).copied() else {
        return;
    };
    let settings = &app.settings;
    ctx.input_mut(|input| {
        let mut pressed = |modifiers, key| input.consume_key(modifiers, key);
        // Shift first: egui ignores an extra Shift when it matches a key.
        if pressed(Modifiers::SHIFT, Key::R) {
            actions.push(Action::SetOption(option.default_value()));
        }
        if pressed(Modifiers::NONE, Key::J) || pressed(Modifiers::NONE, Key::ArrowDown) {
            actions.push(Action::MoveSettingsRow(1));
        }
        if pressed(Modifiers::NONE, Key::K) || pressed(Modifiers::NONE, Key::ArrowUp) {
            actions.push(Action::MoveSettingsRow(-1));
        }
        if pressed(Modifiers::NONE, Key::L) || pressed(Modifiers::NONE, Key::ArrowRight) {
            actions.push(Action::SetOption(settings.stepped(option, true)));
        }
        if pressed(Modifiers::NONE, Key::H) || pressed(Modifiers::NONE, Key::ArrowLeft) {
            actions.push(Action::SetOption(settings.stepped(option, false)));
        }
        if pressed(Modifiers::NONE, Key::Space)
            && let Some(value) = settings.flipped(option)
        {
            actions.push(Action::SetOption(value));
        }
        if pressed(Modifiers::NONE, Key::Escape) {
            actions.push(Action::CloseDialog);
        }
    });
}

pub fn show(app: &mut App, ctx: &egui::Context) {
    let Some(Dialog::Settings(dialog)) = &app.dialog else {
        return;
    };
    let row = dialog.row;
    let mut actions = Vec::new();
    keys(app, ctx, row, &mut actions);
    terminal::show(app, ctx, row, &mut actions);
    app.actions.extend(actions);
}
```

`src/ui/mod.rs`: `pub mod settings;` among the modules, and `settings::show(app, &ui.ctx().clone());` in `show` after `about::show`.

Notes:
- `keys` pushes `SetOption` even when the step changes nothing (the end of the page sizes): `change_settings` writes nothing then, which the test `h_and_l_change_the_cursors_option_and_save_it` holds it to.
- `src/ui/keys.rs` already stops the workspace's letters while any dialog is open (`app.dialog.is_none()`): the screen needs no change there. If one of the workspace's keys still reaches it with the screen open, the test `the_settings_screen_takes_the_keys_from_what_is_under_it` says which.
- A held `l` repeats: each repeat is a `SetOption`, each a write. Task 1 is what makes that safe.

- [ ] **Step 5: The screen**

`src/ui/settings/terminal.rs` draws what the top of this task lists, from `app.settings`, with `row` as the cursor, pushing `SelectSettingsRow` and `SetOption` for clicks. Build it the way `src/ui/connect_dialog/terminal.rs` builds its header, rows and footer; read that file first. What must hold:

- **A modal over everything.** Use `widgets::modal(egui::Id::new("settings"), &look, &palette)` with a frame of no margin, no stroke, no rounding and the window colour, and an area anchored at the window's top left (`egui::Modal::default_area(id).anchor(egui::Align2::LEFT_TOP, egui::Vec2::ZERO).fade_in(false)`, as the connection dialog places its own), and give the content the size of `ctx.content_rect()`. Nothing behind it is clickable, and the workspace is covered.
- **The title is announced.** `settings` is painted with `widgets::paint_label`, which names it for screen readers: the tests find the screen by it.
- **Painted, not laid out.** Allocate each band's rectangle (`ui.allocate_exact_size`) and paint into it with `widgets::paint_text`, `paint_label`, `hline`, `vline`, as the connection dialog's header and footer do. Text only through `Text::one(look, role, ..)` and `Text::new(look).add(role, text, color)`.
- **Every word through `gettext` and `look.label`**, among them the labels of `mod::label`, the hints and the header.
- **Each row is a button** named by its option's label (`ButtonSpec::new(&name).hidden_at(ui, row_rect)`), pushing `SelectSettingsRow(index)`. Names go through `gettext` only, not `look.label`, as the connection dialog's hidden buttons do: `Numbers`, not `numbers`.
- **Each choice is a button** over the place it is drawn, named `"<label>: <choice>"` in the words of `mod::choices` (for example `Timestamps: full`, `Value tags: off`, `Rows per page: more`), pushing `SelectSettingsRow(index)` and `SetOption(value)`. Declare a row's choice buttons after the row's own, so a click on a value is the value's. For the page size the two buttons are the two chevrons; they are there on every row, and drawn on the cursor's.
- **The cursor's row** is marked as the list says. Use `palette.selection` for the fill; paint the 2 pt bar yourself, as the artboard's nav and rows both have it.
- **The label of the cursor's row** is one painted text, `▌` and the label together (`▌rows per page`): the test looks for it whole. So is the page size with its chevrons (`‹ 500 ›`): one `Text` of three parts, each in its colour.
- **The value tags' samples** take `crate::ui::value_tags::terminal_slots(&palette)[0]` and `[1]`.
- **A window too short for the rows:** the rows scroll (`egui::ScrollArea::vertical`) between the header and the footer, which stay.

`show`'s signature: `pub(super) fn show(app: &App, ctx: &egui::Context, row: usize, actions: &mut Vec<Action>)`.

- [ ] **Step 6: Run the tests**

```bash
~/.cargo/bin/cargo test --locked -p tabletist --lib ui::tests::mod_comma
~/.cargo/bin/cargo test --locked -p tabletist --lib ui::tests::the_other_looks_have_no_settings
~/.cargo/bin/cargo test --locked -p tabletist --lib ui::tests::the_shortcuts_dialog_opens_the_settings
~/.cargo/bin/cargo test --locked -p tabletist --lib ui::tests::the_settings_
~/.cargo/bin/cargo test --locked -p tabletist --lib ui::tests::h_and_l_change
~/.cargo/bin/cargo test --locked -p tabletist --lib ui::tests::space_flips
~/.cargo/bin/cargo test --locked -p tabletist --lib ui::tests::a_click_moves_the_settings
~/.cargo/bin/cargo test --locked --workspace --all-targets
```

Expected: PASS. Where a test's text does not match what the screen paints (a label split in two texts, a name in another case), read `harness.painted` and the AccessKit labels for that frame, then make the screen paint what the list above says: the tests state the behaviour, the screen follows.

- [ ] **Step 7: Commit**

```bash
git add src/ui/settings src/ui/mod.rs src/ui/keys.rs src/ui/help.rs
git commit -S -m "Open the settings as a screen of their own in the terminal look

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 5: The file beside the options

**Files:**
- Create: `src/ui/settings/file_pane.rs`
- Modify: `src/ui/settings/terminal.rs`, `src/ui/settings/mod.rs` (`mod file_pane;`)
- Test: `src/ui/settings/file_pane.rs`, `src/ui/mod.rs`

The pane, as the artboard has it: at the right of the rows, 620 wide or 40% of the window if that is less, and not there at all in a window narrower than 1100. A 1 pt rule at its left, on `palette.panel`.

- **Header**, 34 tall, a rule under it, 14 in: the file's path with the home directory written `~`, strong; at the right, `live`, dim, only while `app.settings_file.live`.
- **Body**, 14 in at the sides and 12 at the top: `app.settings_file.text`, one line of the file per line, in the code role (`widgets::code(look)`), coloured:

| Part of a line | Colour |
|---|---|
| A comment, from its `#` to the end | `palette.dim` |
| A table header (`[data]`) | `palette.magenta` |
| A key and its `=` | `palette.text` |
| A string value | `palette.success` |
| A number or `true` / `false` | `palette.orange` |
| A line in `settings_file.invalid`, whole | `palette.danger` |

  The line of the cursor's option (from `settings_file.lines`, by the option's `key()`) has the selection fill across the pane. A line the file does not have (an option whose key the file leaves out) highlights nothing.
- **Under the body**, a rule, then 14 in and 10 over and under, dim: `edits in the file reload live · invalid lines are shown here in red and ignored`, with `red` in `palette.danger`. A file longer than the pane scrolls between the header and this note.

- [ ] **Step 1: Write the failing tests**

`src/ui/settings/file_pane.rs` gets its own tests for the colouring, which is pure:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn parts(line: &str) -> Vec<(&str, Part)> {
        spans(line)
            .into_iter()
            .map(|(range, part)| (&line[range], part))
            .collect()
    }

    #[test]
    fn a_line_of_the_file_is_told_apart_into_its_parts() {
        assert_eq!(parts("[data]"), vec![("[data]", Part::Table)]);
        assert_eq!(
            parts("# written by tabletist, safe to edit by hand"),
            vec![("# written by tabletist, safe to edit by hand", Part::Comment)]
        );
        assert_eq!(
            parts("page_size    = 300"),
            vec![("page_size    = ", Part::Key), ("300", Part::Value)]
        );
        assert_eq!(
            parts("timestamps   = \"second\"  # second | full"),
            vec![
                ("timestamps   = ", Part::Key),
                ("\"second\"", Part::String),
                ("  ", Part::Plain),
                ("# second | full", Part::Comment),
            ]
        );
        assert_eq!(
            parts("value_tags   = true"),
            vec![("value_tags   = ", Part::Key), ("true", Part::Value)]
        );
        assert_eq!(parts(""), vec![]);
    }

    #[test]
    fn a_hash_inside_a_string_is_not_a_comment() {
        assert_eq!(
            parts("theme = \"My #1 \\\"Nord\\\".json\"  # a note"),
            vec![
                ("theme = ", Part::Key),
                ("\"My #1 \\\"Nord\\\".json\"", Part::String),
                ("  ", Part::Plain),
                ("# a note", Part::Comment),
            ]
        );
    }

    #[test]
    fn a_line_that_is_none_of_these_is_plain_and_whole() {
        assert_eq!(parts("this is not toml"), vec![("this is not toml", Part::Plain)]);
        // Every byte of a line is in exactly one part, in order.
        for line in ["x = ", "= 3", "[data", "  [data]  ", "a = \"open", "é = \"ü\"  # ö"] {
            let joined: String = parts(line).into_iter().map(|(text, _)| text).collect();
            assert_eq!(joined, line);
        }
    }

    #[test]
    fn the_home_directory_is_written_as_a_tilde() {
        use std::path::Path;
        let home = Path::new("/home/ada");
        assert_eq!(
            shown_path(Path::new("/home/ada/.config/tabletist/settings.toml"), Some(home)),
            "~/.config/tabletist/settings.toml"
        );
        assert_eq!(
            shown_path(Path::new("/etc/tabletist/settings.toml"), Some(home)),
            "/etc/tabletist/settings.toml"
        );
        assert_eq!(
            shown_path(Path::new("/home/ada/settings.toml"), None),
            "/home/ada/settings.toml"
        );
    }
}
```

In `src/ui/mod.rs`, after the tests of Task 4:

```rust
    #[test]
    fn the_settings_screen_shows_the_file_beside_the_options() {
        let mut harness = settings_screen();
        // The text the app holds, a line of the file per line.
        assert!(harness.painted_color("[data]").is_some());
        assert!(harness.painted_color("page_size    = ").is_some());
        // The path, however the home directory is written.
        assert!(
            harness
                .painted
                .iter()
                .any(|(text, _)| text.ends_with("settings.toml"))
        );
        // Not watched (a test never is): nothing claims it is live.
        assert!(harness.painted_color("live").is_none());
        harness.app.settings_file.live = true;
        harness.settle();
        assert!(harness.painted_color("live").is_some());
    }

    #[test]
    fn a_line_of_the_file_that_was_ignored_is_shown_in_the_colour_of_an_error() {
        let mut harness = settings_screen();
        harness.app.apply(crate::model::Action::Backend(
            crate::backend::Event::SettingsFile {
                text: "[data]\npage_size = 500\ngroup_digits = \"yes\"\n".into(),
                own: false,
            },
        ));
        harness.settle();
        let danger = harness.app.palette.danger;
        assert_eq!(
            harness.painted_color("group_digits = \"yes\""),
            Some(danger)
        );
        // A line that was read is not.
        assert_ne!(harness.painted_color("page_size = "), Some(danger));
        // And the screen shows what the file set.
        assert!(harness.painted_color("‹ 500 ›").is_some());
    }

    #[test]
    fn the_file_pane_marks_the_line_of_the_cursors_option() {
        let mut harness = settings_screen();
        let selection = harness.app.palette.selection;
        // The fill behind the line the text `key` starts.
        let marked = |harness: &Harness, key: &str| {
            let line = harness.painted_rect(key).expect("the key's line");
            harness.fills.iter().any(|(rect, color)| {
                *color == selection
                    && rect.y_range().contains(line.center().y)
                    && rect.x_range().contains(line.center().x)
            })
        };
        assert!(marked(&harness, "page_size    = "));
        assert!(!marked(&harness, "timestamps   = "));
        harness.press(egui::Key::J, egui::Modifiers::NONE);
        assert!(marked(&harness, "timestamps   = "));
        assert!(!marked(&harness, "page_size    = "));
    }

    #[test]
    fn a_narrow_window_has_the_options_and_not_the_file() {
        let mut harness = Harness::with_size(egui::vec2(1000.0, 700.0));
        harness.set_look(crate::theme::Look::omarchy());
        harness.press(egui::Key::Comma, egui::Modifiers::COMMAND);
        assert!(harness.painted_color("▌rows per page").is_some());
        assert!(harness.painted_color("[data]").is_none());
    }
```

The last compares nothing with the design: the width at which the pane goes is the view's, and 1000 is simply a window narrower than any at which it is shown.

- [ ] **Step 2: Run them to see them fail**

Run: `~/.cargo/bin/cargo test --locked -p tabletist --lib ui::settings::file_pane::`
Expected: does not compile (the module does not exist).

- [ ] **Step 3: Implement**

`src/ui/settings/file_pane.rs`:

```rust
//! The settings file as the Settings screen shows it: its own text, a line
//! of the file per line, with each part in its colour. This is no TOML
//! parser: it tells apart what the app writes and leaves the rest plain.

use std::ops::Range;
use std::path::Path;

/// What a stretch of a line is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Part {
    /// `# ...` to the end of the line.
    Comment,
    /// `[table]`.
    Table,
    /// A key and its `=`.
    Key,
    /// A quoted value.
    String,
    /// A number, `true` or `false`.
    Value,
    /// Anything else, as it is.
    Plain,
}

/// The parts of `line`, in order: every byte of it is in exactly one.
pub(super) fn spans(line: &str) -> Vec<(Range<usize>, Part)> {
    let mut out = Vec::new();
    if line.is_empty() {
        return out;
    }
    let trimmed = line.trim();
    if trimmed.starts_with('#') {
        return vec![(0..line.len(), Part::Comment)];
    }
    if trimmed.starts_with('[') && trimmed.ends_with(']') && trimmed.len() > 2 {
        return vec![(0..line.len(), Part::Table)];
    }
    // A key is bare letters, digits, `_` and `-` before the first `=`.
    let Some(equals) = line.find('=') else {
        return vec![(0..line.len(), Part::Plain)];
    };
    let key = line[..equals].trim();
    let bare = |c: char| c.is_ascii_alphanumeric() || c == '_' || c == '-';
    if key.is_empty() || !key.chars().all(bare) {
        return vec![(0..line.len(), Part::Plain)];
    }
    // The key takes the `=` and the blanks after it.
    let rest = &line[equals + 1..];
    let value_at = equals + 1 + (rest.len() - rest.trim_start_matches([' ', '\t']).len());
    out.push((0..value_at, Part::Key));
    let value = &line[value_at..];
    // Where the value ends: a string at its closing quote, anything else at
    // the first blank or `#`.
    let end = if let Some(body) = value.strip_prefix('"') {
        let mut escaped = false;
        let mut close = None;
        for (index, c) in body.char_indices() {
            match c {
                _ if escaped => escaped = false,
                '\\' => escaped = true,
                '"' => {
                    close = Some(index);
                    break;
                }
                _ => {}
            }
        }
        match close {
            Some(close) => {
                out.push((value_at..value_at + close + 2, Part::String));
                value_at + close + 2
            }
            // Left open: plain to the end.
            None => {
                out.push((value_at..line.len(), Part::Plain));
                return out;
            }
        }
    } else {
        let len = value.find([' ', '\t', '#']).unwrap_or(value.len());
        if len > 0 {
            out.push((value_at..value_at + len, Part::Value));
        }
        value_at + len
    };
    // What follows the value: blanks, then a comment or something else.
    let tail = &line[end..];
    match tail.find('#') {
        Some(hash) => {
            if hash > 0 {
                out.push((end..end + hash, Part::Plain));
            }
            out.push((end + hash..line.len(), Part::Comment));
        }
        None if !tail.is_empty() => out.push((end..line.len(), Part::Plain)),
        None => {}
    }
    out
}

/// `path` as the screen writes it: the home directory as `~`.
pub(super) fn shown_path(path: &Path, home: Option<&Path>) -> String {
    match home.and_then(|home| path.strip_prefix(home).ok()) {
        Some(rest) => format!("~/{}", rest.display()),
        None => path.display().to_string(),
    }
}
```

If a test of `spans` shows a case this does not hold (the byte ranges must tile the line, in order, for every input), fix `spans`: the tests state the rule.

The home directory: `directories::BaseDirs::new()` gives it (`src/backend.rs` uses it for `~/.ssh`), and reads only the environment. Not `UserDirs`: on Linux that one reads a file, which is disk work on the UI thread. Pass its `home_dir()` to `shown_path`. No `HOME` reading by hand.

The pane itself is a function of `terminal.rs` or of this file, as it reads best: `pub(super) fn show(ui: &mut egui::Ui, rect: egui::Rect, app: &App, cursor: OptionId)`. It paints, per line of `app.settings_file.text` (split on `\n`, numbering from 1, as `Settings::from_toml` counts):
- the selection fill across the pane when the line is the cursor option's (`settings_file.lines` has `(option.key(), line)`);
- the whole line in `palette.danger` as one text when its number is in `settings_file.invalid`;
- otherwise each span as its own text, in its part's colour, one after the other on the line (so a key and its `=` are one painted text: the tests look for `page_size    = `).

`terminal.rs` gives the pane its rectangle when the window is at least 1100 wide, and the rows the rest.

- [ ] **Step 4: Run the tests**

```bash
~/.cargo/bin/cargo test --locked -p tabletist --lib ui::settings::
~/.cargo/bin/cargo test --locked -p tabletist --lib ui::tests::the_settings_screen_shows_the_file
~/.cargo/bin/cargo test --locked -p tabletist --lib ui::tests::a_line_of_the_file_that_was_ignored
~/.cargo/bin/cargo test --locked -p tabletist --lib ui::tests::the_file_pane_marks
~/.cargo/bin/cargo test --locked -p tabletist --lib ui::tests::a_narrow_window_has_the_options
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/ui/settings src/ui/mod.rs
git commit -S -m "Show the settings file beside its options in the terminal look

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 6: The file in the editor

**Files:**
- Modify: `src/backend.rs` (`Command`, `Event`, `Worker::handle`, the matches that list every command), `src/model.rs` (`Action`), `src/app.rs` (`App::apply`, `apply_event`), `src/ui/settings/mod.rs` (the key), `src/ui/settings/terminal.rs` (the footer's hint)
- Test: `src/backend.rs`, `src/app.rs`, `src/ui/mod.rs`

`ctrl+e` opens the settings file in the editor. The backend does it: it writes the file first when there is none (the app holds what would be written), then starts the program and does not wait for it.

| Built for | The program |
|---|---|
| Linux | `omarchy-launch-editor <path>` when that command is on `PATH`, else `xdg-open <path>` |
| macOS | `open -t <path>` |
| Windows | `explorer <path>` |

The choice follows the operating system the app was built for (`cfg`), not the look: a terminal look drawn in a test on another system still compiles and runs.

- [ ] **Step 1: Write the failing tests**

`src/backend.rs` tests:

```rust
    #[test]
    fn a_settings_file_that_is_not_there_is_written_before_the_editor_starts() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config").join("settings.toml");
        let mut seen = None;
        open_in_editor(&path, "[data]\npage_size = 300\n", |opened| {
            seen = Some((opened.to_path_buf(), std::fs::read_to_string(opened).unwrap()));
            Ok(())
        })
        .unwrap();
        assert_eq!(
            seen,
            Some((path.clone(), "[data]\npage_size = 300\n".to_owned()))
        );
        // A file that is there is opened as it is, whatever the app holds.
        std::fs::write(&path, "[data]\npage_size = 100\n").unwrap();
        open_in_editor(&path, "[data]\npage_size = 300\n", |_| Ok(())).unwrap();
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            "[data]\npage_size = 100\n"
        );
    }

    #[test]
    fn an_editor_that_cannot_start_is_told_with_the_file_still_written() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.toml");
        let error = open_in_editor(&path, "[data]\n", |_| {
            Err(std::io::Error::new(std::io::ErrorKind::NotFound, "no editor"))
        })
        .unwrap_err();
        assert!(error.contains("no editor"), "{error}");
        assert!(path.exists());
    }

    #[test]
    fn the_editor_is_the_systems_way_to_open_a_text_file() {
        let path = std::path::Path::new("/config/settings.toml");
        let (program, args) = editor_command(path, |_| false);
        #[cfg(target_os = "macos")]
        assert_eq!((program.as_str(), args.len()), ("open", 2));
        #[cfg(windows)]
        assert_eq!((program.as_str(), args.len()), ("explorer", 1));
        #[cfg(all(unix, not(target_os = "macos")))]
        {
            assert_eq!((program.as_str(), args.len()), ("xdg-open", 1));
            // Omarchy's own launcher, where it is installed.
            let (program, args) = editor_command(path, |name| name == "omarchy-launch-editor");
            assert_eq!(program, "omarchy-launch-editor");
            assert_eq!(args, vec![path.as_os_str().to_owned()]);
        }
        assert_eq!(args.last().map(|arg| arg.as_os_str()), Some(path.as_os_str()));
    }
```

`src/app.rs` tests:

```rust
    #[test]
    fn the_settings_file_is_opened_in_the_editor_through_the_backend() {
        let (mut app, _dir) = app();
        app.apply(Action::EditSettingsFile);
        let path = app.dirs.settings_file();
        let text = app.settings_file.text.clone();
        assert!(matches!(
            app.backend.sent.last(),
            Some(Command::EditSettingsFile { path: sent, text: held })
                if *sent == path && *held == text
        ));
    }

    #[test]
    fn an_editor_that_did_not_start_shows_a_notice() {
        let (mut app, _dir) = app();
        app.apply(Action::Backend(Event::SettingsFileOpened {
            result: Err("no editor".into()),
        }));
        let notice = app.notice.clone().expect("a notice");
        assert!(notice.contains("no editor"), "{notice}");
        app.notice = None;
        app.apply(Action::Backend(Event::SettingsFileOpened { result: Ok(()) }));
        assert!(app.notice.is_none());
    }
```

`src/ui/mod.rs`, after the tests of Task 5:

```rust
    #[test]
    fn ctrl_e_opens_the_settings_file_in_the_editor() {
        let mut harness = settings_screen();
        // The footer's hint is its button too.
        assert!(harness.has("Open file in editor"));
        harness.press(egui::Key::E, egui::Modifiers::CTRL);
        assert!(matches!(
            crate::testing::last_sent(&harness.app),
            Command::EditSettingsFile { .. }
        ));
        // The screen stays: the editor is another window.
        assert!(settings_open(&harness));
    }
```

- [ ] **Step 2: Run them to see them fail**

Run: `~/.cargo/bin/cargo test --locked -p tabletist --lib a_settings_file_that_is_not_there`
Expected: does not compile, "cannot find function `open_in_editor`".

- [ ] **Step 3: Implement**

`src/backend.rs`.

`Command`, before `Flush`:

```rust
    /// Opens the settings file `path` in the editor, written from `text`
    /// first when it is not there. Answered with
    /// [`Event::SettingsFileOpened`].
    EditSettingsFile {
        path: PathBuf,
        text: String,
    },
```

`Event`, after `SettingsFile`:

```rust
    /// The editor was started on the settings file, or why it was not.
    SettingsFileOpened { result: Result<(), String> },
```

`Command::EditSettingsFile { .. }` joins `Command::WatchSettings { .. }` in the four matches that list every command.

`Worker::handle`, before the catch-all arm:

```rust
            Command::EditSettingsFile { path, text } => {
                let outbox = self.outbox.clone();
                tokio::task::spawn_blocking(move || {
                    let result = open_in_editor(&path, &text, start_editor);
                    if let Err(error) = &result {
                        log::warn!("could not open {} in the editor: {error}", path.display());
                    }
                    outbox.emit(Event::SettingsFileOpened { result });
                });
            }
```

Below the watcher's functions:

```rust
/// Opens the settings file `path` with `start`, writing `text` to it first
/// when it is not there: the editor is given a file, and the UI thread
/// never looked at the disk to know.
fn open_in_editor(
    path: &std::path::Path,
    text: &str,
    start: impl FnOnce(&std::path::Path) -> std::io::Result<()>,
) -> Result<(), String> {
    if !path.exists() {
        crate::util::write_atomic(path, text.as_bytes()).map_err(|error| error.to_string())?;
    }
    start(path).map_err(|error| error.to_string())
}

/// The program that opens a text file for editing on this system, and what
/// it is given. `on_path` says whether a command of that name can be run.
fn editor_command(
    path: &std::path::Path,
    on_path: impl Fn(&str) -> bool,
) -> (String, Vec<std::ffi::OsString>) {
    let file = path.as_os_str().to_owned();
    #[cfg(target_os = "macos")]
    {
        let _ = on_path;
        ("open".into(), vec!["-t".into(), file])
    }
    #[cfg(windows)]
    {
        let _ = on_path;
        ("explorer".into(), vec![file])
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        // Omarchy opens $EDITOR in a terminal of its own; any other
        // desktop knows what edits a text file.
        let program = if on_path("omarchy-launch-editor") {
            "omarchy-launch-editor"
        } else {
            "xdg-open"
        };
        (program.into(), vec![file])
    }
}

/// Whether a command named `name` is in one of `PATH`'s directories.
fn on_path(name: &str) -> bool {
    std::env::var_os("PATH")
        .is_some_and(|paths| std::env::split_paths(&paths).any(|dir| dir.join(name).is_file()))
}

/// Starts the editor on `path` and lets it go: it is the user's window from
/// here. A thread of its own waits for it, so it leaves no zombie.
fn start_editor(path: &std::path::Path) -> std::io::Result<()> {
    use std::process::Stdio;
    let (program, args) = editor_command(path, on_path);
    let mut child = std::process::Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;
    std::thread::spawn(move || {
        let _ = child.wait();
    });
    Ok(())
}
```

`src/model.rs`, `Action`, after `SetOption`:

```rust
    /// Open the settings file in the editor.
    EditSettingsFile,
```

`src/app.rs`, `App::apply`:

```rust
            Action::EditSettingsFile => self.backend.send(Command::EditSettingsFile {
                path: self.dirs.settings_file(),
                text: self.settings_file.text.clone(),
            }),
```

and `apply_event`:

```rust
            Event::SettingsFileOpened { result } => {
                if let Err(error) = result {
                    self.notice = Some(format!(
                        "Could not open {} in the editor: {error}.",
                        self.dirs.settings_file().display()
                    ));
                }
            }
```

`src/ui/settings/mod.rs`, in `keys`, before the plain keys (Ctrl first, as Shift is). Only a fresh press: `consume_key` counts the repeats of a held key too, and each would start another editor. `src/ui/keys.rs` has `consume_press` for exactly this; make it `pub(super)` and use it:

```rust
        if super::keys::consume_press(input, Modifiers::CTRL, Key::E) {
            actions.push(Action::EditSettingsFile);
        }
```

(`pressed` borrows `input` in the closure above: take this key before that closure is made, or call `input.consume_key` directly in the others.)

`src/ui/settings/terminal.rs`: the footer gains `ctrl+e` `open file in $EDITOR` between `space` and `R`, its hint a button named `Open file in editor` that pushes `Action::EditSettingsFile`. `$EDITOR` is a name, not a word: keep it out of `look.label`, which would lower-case it.

Notes:
- `text` is what the app holds: with no file, that is what would be written; with a file, the file wins and `text` is not used.
- A file the editor creates or changes comes back through the watcher of step 2. Nothing else is needed here.
- `std::thread::spawn` for the wait and not the runtime's blocking pool: an editor stays open for hours, and the pool's threads are for work that ends.

- [ ] **Step 4: Run the tests**

```bash
~/.cargo/bin/cargo test --locked -p tabletist --lib backend::tests::a_settings_file_that_is_not_there
~/.cargo/bin/cargo test --locked -p tabletist --lib backend::tests::an_editor_that_cannot_start
~/.cargo/bin/cargo test --locked -p tabletist --lib backend::tests::the_editor_is_the_systems
~/.cargo/bin/cargo test --locked -p tabletist --lib the_settings_file_is_opened_in_the_editor
~/.cargo/bin/cargo test --locked -p tabletist --lib an_editor_that_did_not_start
~/.cargo/bin/cargo test --locked -p tabletist --lib ctrl_e_opens_the_settings_file
```

Expected: PASS. No test starts a real editor.

- [ ] **Step 5: Commit**

```bash
git add src/backend.rs src/model.rs src/app.rs src/ui/settings src/ui/mod.rs
git commit -S -m "Open the settings file in the editor from its screen

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 7: A picture of the screen, to look at

**Files:**
- Modify: `src/shots.rs`

Nobody can open the window in this session, and no test may compare the screen with the design. `src/shots.rs` renders scenes to PNG under `target/shots/` (feature `shots`), for a person to look at. Add the Settings screen to it.

- [ ] **Step 1: Add the scene**

Read `src/shots.rs` from its header down: how the scenes are listed (the `Screen` enum with `OmarchyDialog`, `Screen::ALL` and its length, each scene's name, look, size, scale and palette, and the `match` that sets a scene up), and the command the header gives for rendering them. Add `OmarchySettings`: the Omarchy look and the palette the other Omarchy scenes use, a window wide enough for the file pane (wider than 1100 points), `Action::ShowSettings` and the cursor on the second row (`Action::MoveSettingsRow(1)`), with `settings_file.live` set so the header says so. Name it `omarchy-settings-general`.

Fixtures are the neutral demo data only. The picture is never committed: `target/` is ignored. The pane's header will show the harness's temporary path: that is what the app would show for that directory, and no reason to fake a path.

- [ ] **Step 2: Render it and look**

The shots tests are `#[ignore]`d: run them as the header of `src/shots.rs` says (with `--features shots` and `-- --ignored`), then open the picture it wrote under `target/shots/` (the file's name has the prefix the others have, for example `mock-omarchy-settings-general.png`; the Read tool shows images).

Look for what a test cannot say: text that overlaps or is cut, a band of the wrong height, the cursor's bar missing, the pane's colours, the footer's hints running into each other. Fix what is wrong in `src/ui/settings/terminal.rs` and render again. If the renderer cannot start here (no GPU), say so in the report and leave the scene for the user to render.

- [ ] **Step 3: Commit**

```bash
git add src/shots.rs src/ui/settings
git commit -S -m "Render the Settings screen among the shots

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 8: The whole suite, and what is left for a person

**Files:** none, unless a check fails.

- [ ] **Step 1: Run every check of `AGENTS.md`**

```bash
~/.cargo/bin/cargo fmt --all --check
~/.cargo/bin/cargo clippy --locked --workspace --all-targets -- -D warnings
~/.cargo/bin/cargo test --locked --workspace --all-targets
RUSTDOCFLAGS='-D warnings' ~/.cargo/bin/cargo doc --locked --workspace --no-deps
~/.cargo/bin/cargo clippy --locked --workspace --all-targets --features shots -- -D warnings
```

Expected: all pass. `Cargo.lock` is unchanged since step 2.

- [ ] **Step 2: By hand, with a window (for the user)**

This session has no display. List these in the report for the user; do not claim them.

1. On Omarchy, `ctrl+,` opens the screen; `esc` closes it; `?` then Settings opens it too.
2. `j`/`k` move the cursor, and the file pane's highlight follows it.
3. `h`/`l` on each row change the value; a grid open under the screen shows the change when the screen closes. Hold `l` on rows per page: the value stops at 5,000 and does not flicker back.
4. `space` flips value tags, timestamps and numbers; `R` puts the cursor's option back.
5. `ctrl+e` opens the file in the editor. Save a change there: the screen follows without a restart. Write a bad line: it is red in the pane, and the other options keep their values.
6. A window narrower than the pane needs has the options and not the file.
7. A click on a row moves the cursor; a click on a value sets it.

- [ ] **Step 3: Report**

Say what passed, what was only compiled (macOS and Windows: the editor's program there is chosen by `cfg` and was not run), whether the shot rendered, which commits exist, and which are waiting for a signature.

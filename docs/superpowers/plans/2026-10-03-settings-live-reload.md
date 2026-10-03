# Settings: live reload Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** An edit to `settings.toml` made outside the app takes effect in the running app, without a restart and without the app writing the file back.

**Architecture:** Every change of the settings goes through `App::apply_settings`, which replaces them and runs the effects of the fields that differ (open workspaces take the timestamps option, open tables fetch their page again at the new size, a new theme is resolved). `App::change_settings` is the path of a change made in the app: it applies, renders the canonical text, and saves. The backend watches the config directory with `notify`, reads the file when it changes, and sends its text; the app drops a text equal to the one it holds (its own write) and applies any other without saving. A grid's id carries what decides its cells' widths, so a grid is fitted again when an option widens them.

**Tech Stack:** Rust 2024, egui (crmne fork, 0.36), tokio, `notify` 8.2. Headless UI tests through `src/testing.rs`; backend tests against a real watcher in a temporary directory.

**Spec:** `docs/superpowers/specs/2026-10-03-settings-general-design.md`, sections "The options" (`apply_settings`, `change_settings`, the effects table) and "Live reload". This plan is step 2 of its four. Step 1 (the store and the options) is the code this builds on. There is still no window: steps 3 and 4 draw one.

---

## Ground rules (read first)

- Work in `/home/igor/Work/tabletist/.claude/worktrees/settings-general-tab-957dcf`, on the branch `claude/settings-live-reload`. It is cut from the tip of step 1's branch (`claude/settings-general-tab-957dcf`), or from `main` once that is merged. Start from a clean working tree: nothing of step 1 may be left uncommitted, or a task's `git add` would sweep it into step 2.
- `cargo` through mise is broken here: always run `~/.cargo/bin/cargo`. Never point `CARGO_TARGET_DIR` at `/tmp`.
- Every `cargo` command takes `--locked`. `Cargo.lock` is edited by hand in Task 5 and nowhere else; if any command wants to change it, stop and report.
- One topic per commit, signed: plain `git commit -S`. Never set `SSH_AUTH_SOCK` in a git command. If signing fails with "agent refused operation", 1Password has locked: do **not** commit unsigned. Stage the task's files, run `git write-tree`, note the tree id and the commit message in your report, and go on. End every message with `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.
- Never push.
- No em dashes anywhere (code, comments, docs, commit messages).
- Tests are behavioural. Never a design or pixel conformance test. Fixtures use neutral names (`users`, `amount`).
- Views do not mutate application state; they read `app.settings` and push `Action`s. Disk and network work runs on the backend, never on the UI thread.
- Comments say why, in the voice of the code around them.
- `clippy::unwrap_used` warns outside tests.
- Every commit passes all four checks of `AGENTS.md`. Before each task's commit run `~/.cargo/bin/cargo fmt --all --check`, `~/.cargo/bin/cargo clippy --locked --workspace --all-targets -- -D warnings` and `RUSTDOCFLAGS='-D warnings' ~/.cargo/bin/cargo doc --locked --workspace --no-deps`, beside the tests the task names. A doc comment links only to what exists at that commit.
- Line numbers in this plan are where things were when it was written; find the item by name.

## What exists (from step 1)

In `src/settings.rs`: `Settings` (`page_size`, `timestamps`, `group_digits`, `value_tags`, `show_system_schemas`, `custom_theme`, `sql_limit`, `sql_timeout_secs`), `Key`, `Source`, `Loaded { settings, text, invalid, lines, source }`, `Loaded::of(settings, source)` (validates, renders the text, takes the key lines), `Settings::to_toml`, `Settings::from_toml(text) -> Loaded`, `Settings::load(&AppDirs) -> Loaded` (logs each invalid line), `Settings::save(path)`.

In `src/app.rs`: `App::new(dirs, loaded: Loaded, backend)` keeps only `loaded.settings` and calls `save_settings()` once when the source was the JSON; `save_settings` sends `Command::Save { path, file: StateFile::Settings(..) }`; the SQL editor's Limit and Timeout actions set a field and call `save_settings`.

In the views: `data_view::Shown { full_precision, grouped }`, `data_view::cell(.., shown)`, `Tags::when(on)`; `data_view::grid_id(tab, object_tab, full_precision)` and `sql_results::grid_id(tab, id, run, full_precision)`; `sql_results::Place` carries `full_precision`, `grouped`, `value_tags`.

## File structure

| File | Change |
|---|---|
| `src/settings.rs` | `SettingsFile` (what the app holds of the file), `Loaded::warn_invalid`. |
| `src/app.rs` | `settings_file`, `apply_settings`, `resize_pages`, `change_settings`, the theme flag in `logic`, `watch_settings`, the two new events. `fetch_rows` takes the page size of the settings. |
| `src/ui/data_view.rs` | `Fit`; `grid_id` and `columns_note` take it. |
| `src/ui/sql_results.rs` | `Place` carries a `Fit`; `grid_id` takes it. |
| `src/ui/workspace.rs` | The caller of `columns_note`. |
| `src/backend.rs` | `Command::WatchSettings`, `Event::SettingsWatch`, `Event::SettingsFile`, the watcher and its reader. |
| `src/ui/mod.rs` | UI tests. |
| `Cargo.toml`, `Cargo.lock` | `notify` as a direct dependency. |

---

### Task 1: The app keeps what it knows of the file

**Files:**
- Modify: `src/settings.rs`, `src/app.rs` (`App`, `App::new`)

- [ ] **Step 1: Write the failing tests**

In `src/settings.rs`'s tests:

```rust
    #[test]
    fn what_the_app_holds_of_the_file_comes_from_what_was_loaded() {
        let text = "[data]\npage_size = 100\ngroup_digits = \"yes\"\n";
        let (settings, file) = Settings::from_toml(text).into_parts();
        assert_eq!(settings.page_size, 100);
        assert_eq!(
            file,
            SettingsFile {
                text: text.into(),
                invalid: vec![3],
                lines: vec![(Key::PageSize, 2)],
                live: false,
            }
        );
    }
```

In `src/app.rs`'s tests, after `settings_read_from_the_old_json_are_written_as_toml_once`:

```rust
    #[test]
    fn the_app_keeps_the_text_and_the_lines_it_started_with() {
        let dir = tempfile::tempdir().unwrap();
        let text = "[data]\npage_size = 100\ngroup_digits = \"yes\"\n";
        let app = App::new(
            AppDirs::at(dir.path()),
            Settings::from_toml(text),
            Backend::recording(),
        );
        assert_eq!(app.settings.page_size, 100);
        assert_eq!(app.settings_file.text, text);
        assert_eq!(app.settings_file.invalid, vec![3]);
        assert_eq!(
            app.settings_file.lines,
            vec![(crate::settings::Key::PageSize, 2)]
        );
        assert!(!app.settings_file.live);
    }
```

- [ ] **Step 2: Run them to see them fail**

Run: `~/.cargo/bin/cargo test --locked -p tabletist --lib what_the_app_holds_of_the_file`
Expected: does not compile, "cannot find struct `SettingsFile`", "no method named `into_parts`".

- [ ] **Step 3: Implement**

`src/settings.rs`, after `Loaded` and its impls:

```rust
/// What the app holds of the settings file while it runs: enough to know
/// its own writes when they come back from the disk, and to show the file
/// with the lines that were ignored.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SettingsFile {
    /// The text as last read or written.
    pub text: String,
    /// The lines that were ignored, counted from 1.
    pub invalid: Vec<usize>,
    /// The line each key is on.
    pub lines: Vec<(Key, usize)>,
    /// Whether the backend watches the file for changes made outside.
    pub live: bool,
}

impl Loaded {
    /// The settings, and what the app keeps of their file.
    pub fn into_parts(self) -> (Settings, SettingsFile) {
        let file = SettingsFile {
            text: self.text,
            invalid: self.invalid,
            lines: self.lines,
            live: false,
        };
        (self.settings, file)
    }

    /// Says in the log which lines of `path` were ignored. Until a window
    /// shows them, the log is where a typo is told.
    pub fn warn_invalid(&self, path: &Path) {
        for line in &self.invalid {
            log::warn!(
                "{}: line {line} could not be read and is ignored",
                path.display()
            );
        }
    }
}
```

In `Settings::load`, the loop that logs the invalid lines becomes `loaded.warn_invalid(&path);` (its comment moves to the method, as above).

`src/app.rs`:

- Import `SettingsFile` beside `Loaded, Settings, Source`.
- A field on `App`, after `settings`:

```rust
    /// The settings file as the app last read or wrote it.
    pub settings_file: SettingsFile,
```

- `App::new`: the destructuring becomes

```rust
        let source = loaded.source;
        let (settings, settings_file) = loaded.into_parts();
```

  and `settings_file,` joins the struct literal after `settings,`.

- [ ] **Step 4: Run the tests**

```bash
~/.cargo/bin/cargo test --locked -p tabletist --lib settings::
~/.cargo/bin/cargo test --locked -p tabletist --lib app::tests::
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/settings.rs src/app.rs
git commit -S -m "Keep what the app knows of its settings file

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 2: One path for a change, with its effects

**Files:**
- Modify: `src/app.rs` (`fetch_rows`, a new `apply_settings` and `resize_pages` beside `save_settings`, `attach`, `logic`, fields)
- Test: `src/app.rs`, `src/ui/mod.rs`

`apply_settings` replaces the settings and runs the effects of the fields that differ. It writes nothing. The effects:

| Field | Effect |
|---|---|
| `timestamps` | Every open workspace takes it as its `full_precision`. |
| `page_size` | Every open table that shows a page, or waits for one, on a connected session drops the page it shows and fetches again from the offset it is at. |
| `custom_theme` | The palette is resolved again; where the desktop is followed, the theme catalog is started again with the new name first. |
| the others | None here: `show_system_schemas` is read when the sidebar draws and when objects are listed, `sql_limit` and `sql_timeout_secs` when a SQL tab opens, `group_digits` and `value_tags` when a grid draws (Task 4 has their one side effect). |

The rule that keeps paging honest: **a table's `query.limit` is the size of the page it shows or awaits, and `fetch_rows` brings it to the settings' size each time it fetches.** Next and Previous move the offset by `query.limit` before they call `fetch_rows`, so they move by the size of the page that was on screen, and no row is skipped whichever size comes next. A table that is not fetched again (its session is down, or it has nothing loaded) keeps its limit until its next fetch. A table that is fetched again drops the page it shows first, as Next and Previous do: if the new fetch then fails or is cancelled, no page of the old size is left on screen for Next to move past by the new one.

> **After review.** Next and Previous are not the only callers that fetch: Refresh and a reconnect fetch too, and keep the page on screen while they do. So the rule is held by `fetch_rows` itself: when the settings' size differs from the query's limit, it drops the page before it takes the new limit (`ObjectTab::drop_page`). `resize_pages` then needs no drop of its own. The listing below is as first written; `src/app.rs` is the record.

- [ ] **Step 1: Write the failing tests**

In `src/ui/mod.rs`, beside the settings tests of step 1 (`a_workspace_starts_with_the_timestamps_the_settings_ask_for`):

```rust
    /// The `FetchRows` sent last: its offset and its limit.
    fn last_fetch(harness: &Harness) -> (u64, u32) {
        harness
            .app
            .backend
            .sent
            .iter()
            .rev()
            .find_map(|command| match command {
                crate::backend::Command::FetchRows { query, .. } => {
                    Some((query.offset, query.limit))
                }
                _ => None,
            })
            .expect("a FetchRows was sent")
    }

    fn fetches(harness: &Harness) -> usize {
        harness
            .app
            .backend
            .sent
            .iter()
            .filter(|command| matches!(command, crate::backend::Command::FetchRows { .. }))
            .count()
    }

    #[test]
    fn a_new_page_size_fetches_an_open_page_again_from_where_it_is() {
        let mut harness = Harness::new();
        let tab = harness.connect_fake();
        harness.click("users");
        harness.answer_rows(crate::testing::page(3, true));
        let object_tab = harness.app.workspace(tab).unwrap().active_tab.unwrap();
        harness
            .app
            .apply(crate::model::Action::NextPage { tab, object_tab });
        assert_eq!(last_fetch(&harness), (300, 300));
        harness.answer_rows(crate::testing::page(3, true));
        let before = fetches(&harness);
        let settings = crate::settings::Settings {
            page_size: 500,
            ..harness.app.settings.clone()
        };
        harness.app.apply_settings(settings);
        // The same offset, the new size.
        assert_eq!(fetches(&harness), before + 1);
        assert_eq!(last_fetch(&harness), (300, 500));
        // The page of the old size is not left on screen meanwhile: were
        // this fetch to fail, Next would move past it by the new size.
        let object = harness.app.workspace(tab).unwrap().active_object_tab().unwrap();
        assert!(object.page().is_none());
        harness.answer_rows(crate::testing::page(3, true));
        // Next moves by the size of the page that is shown: no row skipped.
        harness
            .app
            .apply(crate::model::Action::NextPage { tab, object_tab });
        assert_eq!(last_fetch(&harness), (800, 500));
    }

    #[test]
    fn a_page_still_on_its_way_is_fetched_again_at_the_new_size() {
        let mut harness = Harness::new();
        harness.connect_fake();
        harness.click("users");
        // Not answered: the first page is in flight at the old size.
        let before = fetches(&harness);
        let settings = crate::settings::Settings {
            page_size: 500,
            ..harness.app.settings.clone()
        };
        harness.app.apply_settings(settings);
        assert_eq!(fetches(&harness), before + 1);
        assert_eq!(last_fetch(&harness), (0, 500));
    }

    #[test]
    fn a_table_whose_session_is_down_waits_for_its_next_fetch() {
        let mut harness = Harness::new();
        let tab = harness.connect_fake();
        harness.click("users");
        harness.answer_rows(crate::testing::page(3, true));
        harness.app.workspace_mut(tab).unwrap().status =
            crate::model::SessionStatus::Disconnected(tabletist_db::Error::query("gone"));
        let before = fetches(&harness);
        let settings = crate::settings::Settings {
            page_size: 500,
            ..harness.app.settings.clone()
        };
        harness.app.apply_settings(settings);
        assert_eq!(fetches(&harness), before, "nothing is asked of a dead session");
        // Its next fetch moves by the page it shows and asks for the new size.
        harness.app.workspace_mut(tab).unwrap().status = crate::model::SessionStatus::Connected;
        let object_tab = harness.app.workspace(tab).unwrap().active_tab.unwrap();
        harness
            .app
            .apply(crate::model::Action::NextPage { tab, object_tab });
        assert_eq!(last_fetch(&harness), (300, 500));
    }

    #[test]
    fn the_same_settings_fetch_nothing() {
        let mut harness = Harness::new();
        harness.connect_fake();
        harness.click("users");
        harness.answer_rows(crate::testing::page(3, true));
        let before = fetches(&harness);
        let settings = harness.app.settings.clone();
        harness.app.apply_settings(settings);
        assert_eq!(fetches(&harness), before);
    }

    #[test]
    fn a_change_of_the_timestamps_option_reaches_every_open_workspace() {
        let mut harness = Harness::new();
        let tab = harness.connect_fake();
        assert!(!harness.app.workspace(tab).unwrap().full_precision);
        let settings = crate::settings::Settings {
            timestamps: crate::settings::Timestamps::Full,
            ..harness.app.settings.clone()
        };
        harness.app.apply_settings(settings);
        assert!(harness.app.workspace(tab).unwrap().full_precision);
        // Another option changing leaves a workspace's own choice alone.
        harness
            .app
            .apply(crate::model::Action::ToggleFullPrecision(tab));
        let settings = crate::settings::Settings {
            group_digits: true,
            ..harness.app.settings.clone()
        };
        harness.app.apply_settings(settings);
        assert!(!harness.app.workspace(tab).unwrap().full_precision);
    }
```

If `tabletist_db::Error::query` is not the constructor for a plain error, use the one `src/testing.rs` uses (`grep -n "Error::" src/testing.rs`).

In `src/app.rs`'s tests:

```rust
    #[test]
    fn a_new_theme_name_is_resolved_at_the_next_logic_pass() {
        let mut harness = crate::testing::Harness::new();
        let settings = Settings {
            custom_theme: Some("Nord.json".into()),
            ..harness.app.settings.clone()
        };
        harness.app.apply_settings(settings);
        assert!(harness.app.theme_changed);
        harness.app.logic(&harness.ctx.clone());
        assert!(!harness.app.theme_changed);
        // Tests do not follow the desktop: no scan of the themes directory.
        assert!(!harness.app.themes.loading());
    }
```

- [ ] **Step 2: Run them to see them fail**

Run: `~/.cargo/bin/cargo test --locked -p tabletist --lib a_new_page_size_fetches`
Expected: does not compile, "no method named `apply_settings`".

- [ ] **Step 3: Implement**

`src/app.rs`, fields on `App` (private, beside `system_theme`), initialised `false` in `App::new`:

```rust
    /// Whether the desktop's themes are followed (not in tests or the demo).
    follow_desktop: bool,
    /// The settings named another theme since the last `logic`.
    theme_changed: bool,
```

`attach` records the first: `self.follow_desktop = follow_desktop;` as its first line.

`fetch_rows` brings the query to the settings' size:

```rust
    pub fn fetch_rows(&mut self, tab: ConnTabId, id: TabId) {
        let request = RequestId(self.next_id());
        let page_size = self.settings.page_size;
        let Some(workspace) = self.workspace_mut(tab) else {
            return;
        };
        let session = workspace.session;
        let Some(object) = workspace.object_tab_mut(id) else {
            return;
        };
        let superseded = object.rows.pending;
        object.rows.start(request);
        // Until here the limit was the size of the page on screen, which
        // Next and Previous have just moved by. From here it is the
        // settings': a size that changed in between costs no row.
        object.query.limit = page_size;
        let query = object.query.clone();
        self.cancel(session, superseded);
        self.backend.send(Command::FetchRows {
            session,
            request,
            query,
        });
    }
```

Beside `save_settings`:

```rust
    /// Replaces the settings and does what the ones that changed ask for.
    /// Every change comes through here, from the app or from the file, and
    /// nothing is written here.
    pub fn apply_settings(&mut self, new: Settings) {
        let old = std::mem::replace(&mut self.settings, new);
        if old.timestamps != self.settings.timestamps {
            let full = self.settings.timestamps == crate::settings::Timestamps::Full;
            for tab in &mut self.tabs {
                if let ConnTabContent::Workspace(workspace) = &mut tab.content {
                    workspace.full_precision = full;
                }
            }
        }
        if old.page_size != self.settings.page_size {
            self.resize_pages();
        }
        if old.custom_theme != self.settings.custom_theme {
            // Reading a theme needs the window: `logic` has it.
            self.theme_changed = true;
        }
    }

    /// Fetches again, at the settings' page size, every table that shows a
    /// page or waits for one on a session that can answer. The page it shows
    /// goes first, as when Next moves on: left on screen while a fetch
    /// fails, a page of the old size would be moved past by the new one.
    /// The others take the size when they next fetch (see `fetch_rows`).
    fn resize_pages(&mut self) {
        let size = self.settings.page_size;
        let mut again = Vec::new();
        for tab in &self.tabs {
            let ConnTabContent::Workspace(workspace) = &tab.content else {
                continue;
            };
            if !matches!(workspace.status, crate::model::SessionStatus::Connected) {
                continue;
            }
            again.extend(
                workspace
                    .object_tabs()
                    .filter(|object| object.query.limit != size)
                    .filter(|object| object.page().is_some() || object.rows.pending.is_some())
                    .map(|object| (tab.id, object.id)),
            );
        }
        for (tab, id) in again {
            if let Some(object) = self.object_tab_mut(tab, id) {
                object.selection = None;
                object.rows.value = None;
            }
            self.fetch_rows(tab, id);
        }
    }
```

`logic` reads the flag beside the catalog's own:

```rust
    pub fn logic(&mut self, ctx: &egui::Context) {
        // The settings named another theme: where the desktop is followed
        // the catalog reads that file first, as it does at the start.
        let renamed = std::mem::take(&mut self.theme_changed);
        if self.themes.needs_reload() || (renamed && self.follow_desktop) {
            let repaint = ctx.clone();
            self.themes.start(
                self.dirs.themes_dir(),
                self.settings.custom_theme.clone(),
                &fastframe_theme::Waker::new(move || repaint.request_repaint()),
            );
        }
        let scanned = self.themes.poll();
        let system = ctx.system_theme();
        if scanned || renamed || system != self.system_theme {
            self.system_theme = system;
            let palette = self.resolve_palette();
            if palette != self.palette {
                self.palette = palette;
                theme::apply(ctx, &palette, &self.look);
            }
        }
    }
```

Notes:
- Use the imports `src/app.rs` already has for `ConnTabContent` and `SessionStatus` if they are in scope; the paths above are written out so the code stands alone.
- If an existing test sets `object.query.limit` by hand and then fetches, it now gets the settings' size. Set `harness.app.settings.page_size` in that test instead; do not remove the line from `fetch_rows`.
- `harness.app.theme_changed` and `harness.app.themes.loading()`: the test lives in `src/app.rs`'s own test module, so the private field is in reach. If `Harness::ctx` is not `pub`, use `egui::Context::default()`.

- [ ] **Step 4: Run the tests**

```bash
~/.cargo/bin/cargo test --locked -p tabletist --lib a_new_page_size
~/.cargo/bin/cargo test --locked -p tabletist --lib a_page_still_on_its_way
~/.cargo/bin/cargo test --locked -p tabletist --lib a_table_whose_session_is_down
~/.cargo/bin/cargo test --locked -p tabletist --lib the_same_settings_fetch_nothing
~/.cargo/bin/cargo test --locked -p tabletist --lib a_change_of_the_timestamps_option
~/.cargo/bin/cargo test --locked -p tabletist --lib a_new_theme_name_is_resolved
~/.cargo/bin/cargo test --locked -p tabletist --lib
```

Expected: PASS, the whole library suite included (`fetch_rows` is on every table's path).

- [ ] **Step 5: Commit**

```bash
git add src/app.rs src/ui/mod.rs
git commit -S -m "Do what a changed setting asks for, in one place

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 3: A change made in the app

**Files:**
- Modify: `src/app.rs` (`save_settings`, the `SetSqlLimit` and `SetSqlTimeout` actions, the test `saving_settings_sends_them_to_the_backend`)

`change_settings` is what the SQL editor's menus use now, and what the Settings window will use: apply, render the canonical text, forget the invalid lines (the canonical text has none), save.

- [ ] **Step 1: Write the failing tests**

In `src/app.rs`'s tests, replace `saving_settings_sends_them_to_the_backend` with:

```rust
    fn settings_saves(app: &App) -> Vec<&Settings> {
        app.backend
            .sent
            .iter()
            .filter_map(|command| match command {
                Command::Save {
                    file: StateFile::Settings(settings),
                    ..
                } => Some(settings),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn a_change_made_in_the_app_is_applied_written_and_kept_as_the_files_text() {
        let dir = tempfile::tempdir().unwrap();
        // A file the user wrote, with a line the reader ignored.
        let text = "[data]\npage_size = 100\ngroup_digits = \"yes\"\n";
        let mut app = App::new(
            AppDirs::at(dir.path()),
            Settings::from_toml(text),
            Backend::recording(),
        );
        app.change_settings(|settings| settings.sql_limit = 100);
        assert_eq!(app.settings.sql_limit, 100);
        assert_eq!(app.settings.page_size, 100, "what the file set stays");
        let saved = settings_saves(&app);
        assert_eq!(saved.len(), 1);
        assert_eq!(*saved[0], app.settings);
        // The file is the canonical text now: nothing in it is invalid.
        assert_eq!(app.settings_file.text, app.settings.to_toml());
        assert!(app.settings_file.invalid.is_empty());
        assert_eq!(
            app.settings_file.lines,
            Settings::from_toml(&app.settings.to_toml()).lines
        );
    }

    #[test]
    fn a_change_that_changes_nothing_is_not_written() {
        let (mut app, _dir) = app();
        let limit = app.settings.sql_limit;
        app.change_settings(|settings| settings.sql_limit = limit);
        assert!(settings_saves(&app).is_empty());
    }

    #[test]
    fn a_change_made_in_the_app_is_brought_into_range() {
        let (mut app, _dir) = app();
        app.change_settings(|settings| settings.page_size = 5);
        assert_eq!(app.settings.page_size, Settings::MIN_PAGE_SIZE);
        assert_eq!(settings_saves(&app).len(), 1);
    }
```

- [ ] **Step 2: Run them to see them fail**

Run: `~/.cargo/bin/cargo test --locked -p tabletist --lib a_change_made_in_the_app`
Expected: does not compile, "no method named `change_settings`".

- [ ] **Step 3: Implement**

`src/app.rs`, beside `apply_settings`:

```rust
    /// A change made in the app (a menu, the Settings window): applied, and
    /// written as the canonical text, which is the file from then on. The
    /// lines the reader had ignored are gone with the text they were in.
    /// A change that changes nothing writes nothing.
    pub fn change_settings(&mut self, change: impl FnOnce(&mut Settings)) {
        let mut changed = self.settings.clone();
        change(&mut changed);
        let loaded = Loaded::of(changed, Source::Toml);
        if loaded.settings == self.settings {
            return;
        }
        let live = self.settings_file.live;
        let (settings, file) = loaded.into_parts();
        self.settings_file = SettingsFile { live, ..file };
        self.apply_settings(settings);
        self.save_settings();
    }
```

`apply_settings`'s comment may now end "nothing is written here: saving is [`App::change_settings`]'s."

`save_settings` loses its `pub` and its comment becomes:

```rust
    /// Sends the settings to the backend to be written. Only
    /// [`App::change_settings`] and the start that read the old
    /// settings.json call it.
    fn save_settings(&mut self) {
```

The two actions use it:

```rust
            Action::SetSqlLimit {
                tab,
                sql_tab,
                limit,
            } => {
                let limit = Settings::valid_sql_limit(limit);
                if let Some(sql) = self.sql_tab_mut(tab, sql_tab) {
                    sql.limit = limit;
                }
                self.change_settings(|settings| settings.sql_limit = limit);
            }
            Action::SetSqlTimeout { tab, sql_tab, secs } => {
                let secs = Settings::valid_sql_timeout(secs);
                if let Some(sql) = self.sql_tab_mut(tab, sql_tab) {
                    sql.timeout = Settings::timeout_of(secs);
                }
                self.change_settings(|settings| settings.sql_timeout_secs = secs);
            }
```

If anything outside `src/app.rs` called `save_settings`, the compiler names it: give it `change_settings`.

- [ ] **Step 4: Run the tests**

```bash
~/.cargo/bin/cargo test --locked -p tabletist --lib a_change_
~/.cargo/bin/cargo test --locked -p tabletist --lib the_limit_and_timeout_menus
~/.cargo/bin/cargo test --locked -p tabletist --lib
```

Expected: PASS. The existing SQL menu tests (`the_limit_and_timeout_menus_change_the_next_run_and_the_settings` and its neighbours) must pass unchanged: they assert one `Command::Save` per real change.

- [ ] **Step 5: Commit**

```bash
git add src/app.rs
git commit -S -m "Write a setting changed in the app as the file's canonical text

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 4: A grid is fitted again when an option widens its cells

**Files:**
- Modify: `src/ui/data_view.rs` (`grid_id` near line 764, `columns_note` near 772 and its caller near 841, `show` near 1055), `src/ui/workspace.rs` (the caller of `columns_note` near line 1564), `src/ui/sql_results.rs` (`grid_id` near 115, `Place`, `draw`, `results`, the test near 2314)
- Test: `src/ui/mod.rs`

A grid keeps the column widths it fitted in egui's memory under its id. `full_precision` is part of that id so that wider timestamps fit the columns again. Grouped numbers and value tags change a cell's width too; from this step on they can change while a grid is open.

- [ ] **Step 1: Write the failing test**

In `src/ui/mod.rs`, after the tests of Task 2:

```rust
    #[test]
    fn a_grid_is_fitted_again_when_an_option_widens_its_cells() {
        let mut harness = Harness::new();
        harness.connect_fake();
        harness.click("users");
        let mut page = crate::testing::page(1, false);
        page.rows[0][0] = tabletist_db::Value::Int(i64::MAX);
        harness.answer_rows(page);
        harness.settle();
        assert!(harness.painted_color("9223372036854775807").is_some());
        let settings = crate::settings::Settings {
            group_digits: true,
            ..harness.app.settings.clone()
        };
        harness.app.apply_settings(settings);
        harness.settle();
        // Six commas wider. With the widths of the plain number the cell
        // would be cut short and this text never painted whole.
        assert!(
            harness
                .painted_color("9,223,372,036,854,775,807")
                .is_some()
        );
    }
```

- [ ] **Step 2: Run it to see it fail**

Run: `~/.cargo/bin/cargo test --locked -p tabletist --lib a_grid_is_fitted_again`
Expected: FAIL at the last assertion (the grouped number is painted cut, with an ellipsis).

If it passes before any change (the column had room to spare), the test does not show the behaviour: make it observe the grid's memory instead. Before the change, take `harness.app.workspace(tab)` and assert `crate::ui::grid::remembered(&harness.ctx, <the old id>)`; after it, assert a grid is remembered under another id. `grid_id` is private to `data_view`: move the test into `src/ui/data_view.rs`'s tests to reach it. Say in your report which form you kept and why.

- [ ] **Step 3: Implement**

`src/ui/data_view.rs`, above `grid_id`:

```rust
/// What decides how wide a grid's cells are, beside its rows. A grid keeps
/// the widths it fitted under its id, so these are part of the id: when one
/// changes the columns are fitted again, as a grid of its own.
#[derive(Clone, Copy, Debug, Default, Hash, PartialEq, Eq)]
pub struct Fit {
    /// Timestamps with their fraction.
    pub full_precision: bool,
    /// Numbers with their digits in threes.
    pub grouped: bool,
    /// Values of a closed set as tags, which pad their text.
    pub value_tags: bool,
}

impl Fit {
    pub fn of(workspace: &crate::model::Workspace, settings: &crate::settings::Settings) -> Self {
        Self {
            full_precision: workspace.full_precision,
            grouped: settings.group_digits,
            value_tags: settings.value_tags,
        }
    }
}

/// The id of a table's grid, one per [`Fit`].
fn grid_id(tab: ConnTabId, object_tab: TabId, fit: Fit) -> Id {
    Id::new(("grid", tab.0, object_tab.0, fit))
}
```

`columns_note` gains a `fit: Fit` parameter after `object` and uses `grid_id(tab, object.id, fit)`. Its two callers build it with `Fit::of(workspace, &app.settings)`; where `app` is borrowed in a way that does not allow it, take the `Fit` into a local before that borrow starts.

`show`: `let fit = Fit::of(workspace, &app.settings);` where `full_precision` is read today; `grid_id(tab, object_tab, fit)`; the tags use `fit.value_tags`; each column's `Shown` is

```rust
            .map(|column| Shown {
                full_precision: fit.full_precision,
                grouped: fit.grouped && !is_key(&column.name, structure),
            })
```

and the locals `full_precision`, `group_digits` and `value_tags` that only fed these go.

`src/ui/sql_results.rs`:

- `grid_id(tab, id, run, fit: data_view::Fit)`, with `fit` in place of `full_precision` in the tuple and its doc comment saying a grid is one per run and per fit.
- `Place`'s three fields `full_precision`, `grouped`, `value_tags` become one, `fit: data_view::Fit` ("How the cells are drawn, as the workspace's tables draw them"), filled in `draw` with `data_view::Fit::of(workspace, &app.settings)`.
- `results` takes `fit` out of `*place`; the tags use `fit.value_tags`; the cell's `Shown` is `{ full_precision: fit.full_precision, grouped: fit.grouped }`; any other reader of `place.full_precision` reads `place.fit.full_precision`.
- The test near line 2314 that builds grid ids with `false` builds them with `data_view::Fit::of(harness.app.workspace(tab).unwrap(), &harness.app.settings)`.

- [ ] **Step 4: Run the tests**

```bash
~/.cargo/bin/cargo test --locked -p tabletist --lib a_grid_is_fitted_again
~/.cargo/bin/cargo test --locked -p tabletist --lib ui::
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/ui/data_view.rs src/ui/sql_results.rs src/ui/workspace.rs src/ui/mod.rs
git commit -S -m "Fit a grid again when an option changes how wide its cells are

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 5: The backend watches the file

**Files:**
- Modify: `Cargo.toml`, `Cargo.lock`, `src/backend.rs` (`Command`, `Event`, `Worker`, `Worker::handle`, the four matches that list every command, tests)

- [ ] **Step 1: Add the dependency**

In `Cargo.toml`, after the `toml` entry:

```toml
# Tells the backend when settings.toml changes on disk, so an edit made in
# an editor reaches the running app (src/backend.rs). The version and the
# features fastframe-theme already brings in for following the desktop's
# theme: nothing new is built.
notify = "8.2"
```

In `Cargo.lock`, in the `[[package]]` whose `name = "tabletist"`, add ` "notify",` between ` "log",` and ` "objc2 0.6.4",`. Change nothing else.

Run: `~/.cargo/bin/cargo check --locked -p tabletist`
Expected: compiles.

Run: `git diff --stat Cargo.lock`
Expected: `1 file changed, 1 insertion(+)`.

- [ ] **Step 2: Write the failing tests**

In `src/backend.rs`'s tests:

```rust
    /// Starts a backend watching `path`, and says whether it could.
    fn watching(path: &std::path::Path) -> (Backend, bool) {
        let mut backend = Backend::start_with(Waker::default(), Keyring::memory());
        backend.send(Command::WatchSettings {
            path: path.to_path_buf(),
        });
        match backend.wait(WAIT) {
            Some(Event::SettingsWatch { live }) => (backend, live),
            other => panic!("expected to hear whether the file is watched, got {other:?}"),
        }
    }

    /// The texts of the settings file the backend sends until `expected`
    /// comes, that one included; empty when it never does.
    fn texts_until(backend: &mut Backend, expected: &str) -> Vec<String> {
        let deadline = std::time::Instant::now() + WAIT;
        let mut texts = Vec::new();
        while std::time::Instant::now() < deadline {
            if let Some(Event::SettingsFile { text }) = backend.wait(Duration::from_millis(200)) {
                let done = text == expected;
                texts.push(text);
                if done {
                    return texts;
                }
            }
        }
        Vec::new()
    }

    #[test]
    fn the_settings_file_is_read_when_it_changes() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.toml");
        let (mut backend, live) = watching(&path);
        assert!(live);
        std::fs::write(&path, "[data]\npage_size = 500\n").unwrap();
        assert!(!texts_until(&mut backend, "[data]\npage_size = 500\n").is_empty());
        // An editor that saves by renaming another file over it.
        crate::util::write_atomic(&path, b"[data]\npage_size = 100\n").unwrap();
        assert!(!texts_until(&mut backend, "[data]\npage_size = 100\n").is_empty());
    }

    #[test]
    fn only_a_change_of_the_settings_file_wakes_the_reader() {
        use notify::event::{AccessKind, CreateKind, ModifyKind, RenameMode};
        use notify::{Event, EventKind};
        let name = std::ffi::OsStr::new("settings.toml");
        let at = |kind: EventKind, path: &str| Event::new(kind).add_path(path.into());
        let data = EventKind::Modify(ModifyKind::Any);
        assert!(concerns(&at(data, "/config/settings.toml"), Some(name)));
        assert!(concerns(
            &at(EventKind::Create(CreateKind::File), "/config/settings.toml"),
            Some(name)
        ));
        // A rename carries both names: the temporary file's and ours.
        let renamed = Event::new(EventKind::Modify(ModifyKind::Name(RenameMode::Both)))
            .add_path("/config/.tabletist-x.tmp".into())
            .add_path("/private/config/settings.toml".into());
        assert!(concerns(&renamed, Some(name)));
        // Another file of the directory, and a look at ours: Linux reports
        // every open, and the reader's own read is one.
        assert!(!concerns(&at(data, "/config/connections.json"), Some(name)));
        assert!(!concerns(
            &at(EventKind::Access(AccessKind::Any), "/config/settings.toml"),
            Some(name)
        ));
        assert!(!concerns(&at(data, "/config/settings.toml"), None));
    }

    #[test]
    fn a_settings_file_that_is_not_text_sends_nothing() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.toml");
        let (mut backend, _) = watching(&path);
        std::fs::write(&path, [0xff, 0xfe, 0x00]).unwrap();
        std::thread::sleep(Duration::from_millis(400));
        std::fs::write(&path, "[data]\npage_size = 500\n").unwrap();
        // The first text to come is the readable one.
        assert_eq!(
            texts_until(&mut backend, "[data]\npage_size = 500\n"),
            vec!["[data]\npage_size = 500\n".to_owned()]
        );
    }

    #[test]
    fn reading_the_file_does_not_send_it_again() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.toml");
        let (mut backend, _) = watching(&path);
        std::fs::write(&path, "[data]\npage_size = 500\n").unwrap();
        assert!(!texts_until(&mut backend, "[data]\npage_size = 500\n").is_empty());
        // Whatever else the system reports of the file, its text is the
        // same: nothing more is sent.
        assert!(backend.wait(Duration::from_millis(600)).is_none());
    }

    #[test]
    fn a_settings_file_that_is_deleted_sends_nothing() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.toml");
        let (mut backend, _) = watching(&path);
        std::fs::write(&path, "[data]\npage_size = 500\n").unwrap();
        assert!(!texts_until(&mut backend, "[data]\npage_size = 500\n").is_empty());
        std::fs::remove_file(&path).unwrap();
        assert!(backend.wait(Duration::from_millis(600)).is_none());
    }

    #[test]
    fn a_directory_that_is_not_there_cannot_be_watched() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("missing").join("settings.toml");
        let (_backend, live) = watching(&path);
        assert!(!live);
    }
```

`Event` derives only `Debug`, so the tests ask `is_none()` of `backend.wait` and never compare events. Whether the reader is woken is decided by one small function, `concerns`, tested with events built by hand: an end-to-end test cannot show the access filter (the reader swallows a text it already sent), and one that writes the file just before the watch starts is flaky on macOS, where FSEvents can still report it.

- [ ] **Step 3: Run them to see them fail**

Run: `~/.cargo/bin/cargo test --locked -p tabletist --lib the_settings_file_is_read_when_it_changes`
Expected: does not compile, "no variant named `WatchSettings`".

- [ ] **Step 4: Implement**

`src/backend.rs`.

`Command` gains, before `Flush`:

```rust
    /// Watches the settings file `path` for changes made outside the app.
    /// Answered with [`Event::SettingsWatch`]; each change is then an
    /// [`Event::SettingsFile`].
    WatchSettings {
        path: PathBuf,
    },
```

`Event` gains, after `Saved`:

```rust
    /// Whether the settings file is being watched.
    SettingsWatch { live: bool },
    /// The settings file changed on disk: its text. The app's own writes
    /// come this way too, and it knows them by their text.
    SettingsFile { text: String },
```

The four `match`es that list every command (the session a command belongs to, its request, the event of a lost session, the one in `run_session`): `Command::WatchSettings { .. }` joins `Command::Save { .. }` and `Command::Flush { .. }` in each. The compiler names them.

`Worker` gains a field, `None` in `Worker::new`:

```rust
    /// Watches the config directory for the settings file. Dropping it
    /// ends the watch and the task that reads the file.
    settings_watch: Option<notify::RecommendedWatcher>,
```

`Worker::handle` gains:

```rust
            Command::WatchSettings { path } => {
                let live = match watch_settings(path, self.outbox.clone()) {
                    Ok(watcher) => {
                        self.settings_watch = Some(watcher);
                        true
                    }
                    Err(error) => {
                        log::warn!("could not watch the settings file: {error}");
                        false
                    }
                };
                self.outbox.emit(Event::SettingsWatch { live });
            }
```

Below `Saves` and its impl:

```rust
/// How long the settings file is left to settle before it is read: a save
/// is several changes (a temporary file, a rename, a truncate and a write).
const SETTLE: Duration = Duration::from_millis(100);

/// Watches the directory of the settings file `path` and starts the task
/// that reads the file whenever it changes. The directory and not the
/// file: an editor saves by renaming another file over it, and a watch on
/// the file would stay with the one that was replaced.
fn watch_settings(path: PathBuf, outbox: Outbox) -> notify::Result<notify::RecommendedWatcher> {
    use notify::Watcher as _;
    let (changed, changes) = tokio_mpsc::unbounded_channel();
    let name = path.file_name().map(std::ffi::OsStr::to_owned);
    let mut watcher =
        notify::recommended_watcher(move |event: notify::Result<notify::Event>| match event {
            Ok(event) => {
                if concerns(&event, name.as_deref()) {
                    let _ = changed.send(());
                }
            }
            Err(error) => log::warn!("watching the settings file: {error}"),
        })?;
    let directory = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or(std::path::Path::new("."));
    watcher.watch(directory, notify::RecursiveMode::NonRecursive)?;
    tokio::spawn(read_settings(path, changes, outbox));
    Ok(watcher)
}

/// Whether `event` says the file named `name` may have changed. Not when it
/// was only looked at: Linux reports every open, and the reader below opens
/// the file, so answering those would have it read again for ever. The file
/// is known by its name: the paths come as the system has them, which is
/// not always as the directory was given.
fn concerns(event: &notify::Event, name: Option<&std::ffi::OsStr>) -> bool {
    !event.kind.is_access()
        && name.is_some()
        && event.paths.iter().any(|path| path.file_name() == name)
}

/// Sends the settings file's text each time it changes, once the changes
/// have settled. Ends when the watcher is dropped.
async fn read_settings(
    path: PathBuf,
    mut changes: tokio_mpsc::UnboundedReceiver<()>,
    outbox: Outbox,
) {
    // What was sent last: the same text is not news, whatever woke us.
    let mut sent: Option<String> = None;
    while changes.recv().await.is_some() {
        tokio::time::sleep(SETTLE).await;
        while changes.try_recv().is_ok() {}
        let file = path.clone();
        let read = tokio::task::spawn_blocking(move || std::fs::read(file)).await;
        let bytes = match read {
            Ok(Ok(bytes)) => bytes,
            // Deleted: the settings in memory stay, and the next change
            // made in the app writes the file again.
            Ok(Err(error)) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Ok(Err(error)) => {
                log::warn!("could not read {}: {error}", path.display());
                continue;
            }
            Err(error) => {
                log::warn!("could not read {}: {error}", path.display());
                continue;
            }
        };
        match String::from_utf8(bytes) {
            Ok(text) if sent.as_deref() == Some(text.as_str()) => {}
            Ok(text) => {
                sent = Some(text.clone());
                outbox.emit(Event::SettingsFile { text });
            }
            // Half-written, or not a settings file at all: the settings in
            // memory stay, and nothing on disk is touched.
            Err(_) => log::warn!("{} is not text; it is not read", path.display()),
        }
    }
}
```

Notes:
- `Worker::handle` runs inside the runtime (`runtime.block_on(worker.run(..))`), so `tokio::spawn` works there, as `Saves::save` spawns its writer.
- Put the `Command::WatchSettings` arm before the catch-all arm of `Worker::handle` (`query => ..`, which sends a command to its session). After it, the command would be answered as one for a session that is not there, with no event, and `watching()` would wait out its ten seconds and panic.
- `tokio_mpsc` is the alias `src/backend.rs` already uses for `tokio::sync::mpsc`.
- If `the_settings_file_is_read_when_it_changes` is slow or flaky on a platform you can run, print the events the watcher's handler gets before changing anything. Do not raise `SETTLE` to make a test pass.
- `src/app.rs` matches on `Event` in `apply_event`: until Task 6 gives the two new events their arms, add them there as arms that do nothing, so this task compiles on its own.

- [ ] **Step 5: Run the tests**

```bash
~/.cargo/bin/cargo test --locked -p tabletist --lib backend::tests::the_settings_file
~/.cargo/bin/cargo test --locked -p tabletist --lib backend::tests::only_a_change_of_the_settings_file
~/.cargo/bin/cargo test --locked -p tabletist --lib backend::tests::a_settings_file_that_is_deleted
~/.cargo/bin/cargo test --locked -p tabletist --lib backend::tests::a_settings_file_that_is_not_text
~/.cargo/bin/cargo test --locked -p tabletist --lib backend::tests::reading_the_file
~/.cargo/bin/cargo test --locked -p tabletist --lib backend::tests::a_directory_that_is_not_there
~/.cargo/bin/cargo clippy --locked --workspace --all-targets -- -D warnings
```

Expected: PASS, no warnings. Run the watcher tests three times over: a watcher test that passes once and fails once is not done.

- [ ] **Step 6: Commit**

```bash
git add Cargo.toml Cargo.lock src/backend.rs src/app.rs
git commit -S -m "Watch the settings file from the backend

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 6: The app follows the file

**Files:**
- Modify: `src/app.rs` (`apply_event`, `attach`, a new `watch_settings`)

- [ ] **Step 1: Write the failing tests**

In `src/app.rs`'s tests, after the tests of Task 3:

```rust
    #[test]
    fn a_change_of_the_file_is_applied_and_not_written_back() {
        let (mut app, _dir) = app();
        let text = "[data]\npage_size = 500\ngroup_digits = \"yes\"\ntimestamps = \"full\"\n";
        app.apply(Action::Backend(Event::SettingsFile { text: text.into() }));
        assert_eq!(app.settings.page_size, 500);
        assert_eq!(app.settings.timestamps, crate::settings::Timestamps::Full);
        assert!(!app.settings.group_digits);
        // The file as the user wrote it, with the line that was ignored.
        assert_eq!(app.settings_file.text, text);
        assert_eq!(app.settings_file.invalid, vec![3]);
        assert_eq!(
            app.settings_file.lines,
            vec![
                (crate::settings::Key::PageSize, 2),
                (crate::settings::Key::Timestamps, 4)
            ]
        );
        assert!(settings_saves(&app).is_empty(), "the user's file is theirs");
    }

    #[test]
    fn the_apps_own_text_coming_back_is_not_read_again() {
        let (mut app, _dir) = app();
        app.change_settings(|settings| settings.sql_limit = 100);
        let saved = settings_saves(&app).len();
        // Something only a second reading would change.
        app.settings_file.invalid = vec![9];
        let text = app.settings_file.text.clone();
        app.apply(Action::Backend(Event::SettingsFile { text }));
        assert_eq!(app.settings_file.invalid, vec![9]);
        assert_eq!(app.settings.sql_limit, 100);
        assert_eq!(settings_saves(&app).len(), saved);
    }

    #[test]
    fn a_change_of_the_file_keeps_whether_it_is_watched() {
        let (mut app, _dir) = app();
        app.apply(Action::Backend(Event::SettingsWatch { live: true }));
        assert!(app.settings_file.live);
        app.apply(Action::Backend(Event::SettingsFile {
            text: "[data]\npage_size = 500\n".into(),
        }));
        assert!(app.settings_file.live);
        app.change_settings(|settings| settings.sql_limit = 100);
        assert!(app.settings_file.live);
    }

    #[test]
    fn the_settings_file_is_watched_through_the_backend() {
        let (mut app, _dir) = app();
        app.watch_settings();
        let path = app.dirs.settings_file();
        assert!(matches!(
            app.backend.sent.last(),
            Some(Command::WatchSettings { path: watched }) if *watched == path
        ));
    }
```

In `src/ui/mod.rs`, after `a_grid_is_fitted_again_when_an_option_widens_its_cells`:

```rust
    #[test]
    fn an_edit_of_the_file_changes_what_an_open_grid_shows() {
        let mut harness = Harness::new();
        harness.connect_fake();
        harness.click("users");
        let mut page = crate::testing::page(1, false);
        page.rows[0][0] = tabletist_db::Value::Int(1_234_567);
        harness.answer_rows(page);
        harness.settle();
        assert!(harness.painted_color("1234567").is_some());
        harness.app.apply(crate::model::Action::Backend(
            crate::backend::Event::SettingsFile {
                text: "[data]\ngroup_digits = true\n".into(),
            },
        ));
        harness.settle();
        assert!(harness.painted_color("1,234,567").is_some());
    }
```

- [ ] **Step 2: Run them to see them fail**

Run: `~/.cargo/bin/cargo test --locked -p tabletist --lib a_change_of_the_file_is_applied`
Expected: does not compile ("no method named `watch_settings`"). With that one test commented out for a moment, the others fail: the arms added in Task 5 do nothing.

- [ ] **Step 3: Implement**

`src/app.rs`, the two arms of `apply_event`:

```rust
            Event::SettingsWatch { live } => self.settings_file.live = live,
            Event::SettingsFile { text } => {
                // The app's own write coming back from the disk, or a save
                // that changed nothing.
                if text == self.settings_file.text {
                    return;
                }
                let loaded = Settings::from_toml(&text);
                loaded.warn_invalid(&self.dirs.settings_file());
                let live = self.settings_file.live;
                let (settings, file) = loaded.into_parts();
                // The file as its writer left it: not written back, so a
                // line that was ignored stays where they can see it.
                self.settings_file = SettingsFile { live, ..file };
                self.apply_settings(settings);
            }
```

If `apply_event`'s arms cannot `return` (the function does more after the `match`), write the guard as an `if text != self.settings_file.text { .. }` block instead.

Beside `save_settings`:

```rust
    /// Asks the backend to watch the settings file, so an edit made outside
    /// the app reaches it (`Event::SettingsFile`).
    pub fn watch_settings(&mut self) {
        self.backend.send(Command::WatchSettings {
            path: self.dirs.settings_file(),
        });
    }
```

`attach`, inside its `if follow_desktop { .. }`:

```rust
            // Not in tests or the demo, which must not watch the user's
            // directories any more than they scan them.
            self.watch_settings();
```

Two things that are accepted, not bugs:
- Two quick changes in the app can be read from the disk between the two writes. The first text then comes back as if from outside and is applied for a moment; the second follows and puts things right. Nothing is written by either.
- A file deleted while the app runs sends nothing: the settings in memory stay.

- [ ] **Step 4: Run the tests**

```bash
~/.cargo/bin/cargo test --locked -p tabletist --lib a_change_of_the_file
~/.cargo/bin/cargo test --locked -p tabletist --lib the_apps_own_text
~/.cargo/bin/cargo test --locked -p tabletist --lib the_settings_file_is_watched
~/.cargo/bin/cargo test --locked -p tabletist --lib an_edit_of_the_file
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/app.rs src/ui/mod.rs
git commit -S -m "Follow an edit of the settings file in the running app

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 7: The whole suite, and the file by hand

**Files:** none, unless a check fails.

- [ ] **Step 1: Run every check of `AGENTS.md`**

```bash
~/.cargo/bin/cargo fmt --all --check
~/.cargo/bin/cargo clippy --locked --workspace --all-targets -- -D warnings
~/.cargo/bin/cargo test --locked --workspace --all-targets
RUSTDOCFLAGS='-D warnings' ~/.cargo/bin/cargo doc --locked --workspace --no-deps
```

Expected: all four pass. Run the test suite twice: the watcher tests wait on the file system and must pass both times.

- [ ] **Step 2: Confirm the lock file changed by one line since step 1**

Run: `git diff claude/settings-general-tab-957dcf --stat -- Cargo.lock` (or against `main` once step 1 is merged)
Expected: `1 file changed, 1 insertion(+)`.

- [ ] **Step 3: By hand, with a window**

This needs a display. In a session without one, say so in the report and leave these to the user; do not claim them.

1. Start the app (`~/.cargo/bin/cargo run --locked`), open a connection and a table with a numeric column and a timestamp column.
2. In an editor, set `group_digits = true` in `~/.config/tabletist/settings.toml` and save. The grid groups its numbers at once, with no cell cut short.
3. Set `timestamps = "full"`. The grid shows fractions.
4. Set `page_size = 100`. The table fetches again and the footer shows a page of 100.
5. Add a line `this is not toml`. The other settings stay, the log names the line, and the file is left as written.
6. Change the SQL editor's Limit in the app. The file becomes the canonical text (the bad line is gone), and nothing in the app flips back.

- [ ] **Step 4: Report**

Say what passed, what was only compiled (macOS and Windows: the watcher runs on FSEvents and ReadDirectoryChangesW there and was not exercised from Linux; CI runs the backend tests on all three; whether Windows lets a save rename over the file while the reader has it open for its few microseconds was not checked), which commits exist, and which are waiting for a signature.

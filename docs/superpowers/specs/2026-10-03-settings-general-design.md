# Settings: the window and its General tab

Date: 2026-10-03. Status: approved for planning.

## Intent

Tabletist has settings but no place to change them. `Settings` is a struct
saved as `settings.json`; the SQL editor's Limit and Timeout menus are the
only controls that write it, and two of its fields (`page_size`,
`show_system_schemas`) can only be changed by editing the file.

This slice adds the Settings window with its first tab, General, and moves
the file to `settings.toml`: a file that is written by the app, safe to edit
by hand, and read again when it changes on disk.

Success: a user presses `Mod+,` in any look, changes how the grid shows
rows, timestamps, numbers and value tags, and sees every open grid follow at
once. On Omarchy the file is shown beside the options, an edit made in
`$EDITOR` shows up in the running app without a restart, and a line the app
cannot read is marked red and ignored rather than costing the user their
other settings.

The designs are the "Settings, General" artboards (macOS and Omarchy) in the
design canvas Artifact. They are not copied into the repository.

## Decisions

| Question | Decision |
|---|---|
| Rows | Only the options the app can honour today: rows per page, timestamps, numbers, value tags. |
| File | `settings.toml`, in tables. An existing `settings.json` is read once and left in place. |
| Reading | Tolerant: a line that cannot be read is ignored and remembered by its number; the rest of the file applies. |
| Writing | The app's own canonical text, the same every time. Comments and unknown keys a user added are not kept. |
| Parser | The `toml` crate (`DeTable::parse_recoverable`). Our own writer. |
| Reload | The backend watches the config directory with `notify` and reads the file again when it changes. |
| Defaults | Today's behaviour. Nobody's grid changes on upgrade. |
| Applying | Every change applies and saves at once. There is no Save button. |
| Tabs | The nav lists General only. Each later tab adds its own entry. |
| Opening | `Mod+,` in every look, a Settings button in the shortcuts dialog, and "Settings…" in the macOS app menu. |
| Delivery | One spec, four steps, each shippable on its own (see Delivery). |

Rejected: `toml_edit`, patching values in place (it keeps a user's comments,
but the file drifts from the form the artboard shows and the pane would have
to draw arbitrary TOML); a hand-written line parser (it would reject valid
TOML and reimplement string escapes); keeping `settings.json` (the Omarchy
artboard is built around a file the user edits, and JSON has no comments to
say what a value may be); drawing the unbacked rows disabled (controls that
do nothing).

Both new direct dependencies are already in `Cargo.lock`: `toml` 1.1.6
(through `egui_kittest`) and `notify` 8.2 (through `fastframe-theme`). Their
entries are added to the lock by hand and checked with `--locked`.

## Out of scope

- The six rows of the artboard whose features do not exist: On launch,
  Restore tabs and filters (no session is saved), Time zone for timestamptz
  (values are shown as the server sends them), Ask before closing with
  unsaved edits (no editing), Disconnect when idle (no idle tracking), and
  New production connections open read-only. The last one looks backed but
  is not: every session is read-only in 0.1.0 and the connection form's
  read-only box is locked, so the option would have nothing to set. Each row
  arrives with its feature. The Safety section is not drawn while it is
  empty.
- The Appearance, Editor, Keyboard, AI assistant and Updates tabs.
- `ctrl+h/l` to move between the nav and the rows on Omarchy: with one tab
  the nav has nothing to move to. It arrives with the second tab.
- A control for `show_system_schemas`, `sql_limit`, `sql_timeout_secs` or
  the theme. They move to the new file and stay editable by hand.
- Importing a settings file. Export only.

## The file

`AppDirs::settings_file()` becomes `config/settings.toml`. The old path is
kept as `AppDirs::legacy_settings_file()` for the migration.

### Shape

    # written by tabletist, safe to edit by hand
    [data]
    page_size    = 300
    timestamps   = "second"    # second | full
    group_digits = false
    value_tags   = true

    [sidebar]
    show_system_schemas = false

    [editor]
    sql_limit        = 1000
    sql_timeout_secs = 30      # 0 waits forever

    [appearance]
    theme = "Nord.json"

| Key | Type | Default | Notes |
|---|---|---|---|
| `data.page_size` | integer | 300 | Clamped to 10..=10,000, as today. |
| `data.timestamps` | `"second"` or `"full"` | `"second"` | |
| `data.group_digits` | boolean | false | |
| `data.value_tags` | boolean | true | |
| `sidebar.show_system_schemas` | boolean | false | |
| `editor.sql_limit` | integer | 1000 | Clamped to 1..=10,000, as today. |
| `editor.sql_timeout_secs` | integer | 30 | 0 is no timeout (TOML has no null). |
| `appearance.theme` | string | absent | A palette file name in the themes directory. Absent follows the desktop. The table is written only when it is set. |

There is no `version` key: a missing key takes its default and an unknown
key is ignored, which is all the JSON file's version was ever used for.
`Settings::version` and `CURRENT_VERSION` are removed.

`Settings` gains `timestamps: Timestamps` (`Second`, `Full`),
`group_digits: bool` and `value_tags: bool`. It keeps its serde derive so
the old JSON still deserializes; fields the JSON lacks take their defaults.

### Writing

`Settings::to_toml(&self) -> String` renders the text above: the header
comment, the tables in that order, keys aligned within a table, and the
comment that lists a key's values where it has a closed set. It is the only
writer. `StateFile::Settings` saves that text through `util::write_atomic`.

### Reading

    pub struct Loaded {
        pub settings: Settings,
        /// The file's text as read; what the Omarchy pane shows.
        pub text: String,
        /// 1-based numbers of the lines that were ignored.
        pub invalid: Vec<usize>,
        /// The line each known key was read from.
        pub lines: Vec<(Key, usize)>,
    }

    impl Settings {
        pub fn from_toml(text: &str) -> Loaded;
        pub fn load(dirs: &AppDirs) -> Loaded;
    }

`Key` names the eight keys above.

`from_toml` parses with `toml::de::DeTable::parse_recoverable`, which
returns what it could read and one error per line it could not. Then, for
each known key:

- A value of the wrong type, or a string outside a closed set
  (`timestamps = "minute"`), is ignored: the key keeps its default and its
  line joins `invalid`.
- A number outside its range is clamped, as today. It is not invalid.
- An unknown key or table is ignored and is not invalid: a newer version may
  have written it.

Lines the parser rejected join `invalid` too.

`load` picks the source:

1. `settings.toml` exists: read it. A file that is not UTF-8 or cannot be
   read gives the defaults with a logged warning.
2. Else `settings.json` exists: read it as today (`util::load_json`, so a
   damaged one is still moved aside as `.bad`), and write `settings.toml`
   from it at startup. The JSON is not removed or changed, so an older
   Tabletist still finds its file.
3. Else: the defaults. Nothing is written until a setting changes or an
   action needs the file (Reveal, the editor key).

When no file exists yet, `Loaded::text` is `to_toml()` of the defaults: what
would be written.

A file with invalid lines is left as the user wrote it until the app next
writes a setting. That write is the canonical text, so the invalid lines go.

## The options

`App::change_settings(&mut self, change: impl FnOnce(&mut Settings))` is the
one path a change takes, whether it comes from the window, the SQL editor's
menus or the file: it applies the change, runs the effects below for the
fields that differ, renders the text, keeps it as `App::settings_text`, and
saves. `App::save_settings` goes through it.

| Option | Control | Effect |
|---|---|---|
| Rows per page | A menu: 100, 300, 500, 1,000, 5,000. A value from the file that is not in the list is shown as an extra entry. | The next page any tab fetches. Pages already shown are not fetched again. |
| Timestamps | Two segments: To the second, Full precision. | Sets `full_precision` on every open workspace and on new ones. The grid's own link still switches one workspace until the option changes again. |
| Numbers | Two segments, each showing a sample: `1,240.50`, `1240.50`. | Grid cells of numeric columns, in the data view and in SQL results. |
| Value tags | A toggle. | Off: enum, CHECK and boolean columns draw as plain text in the data view, the row panel and SQL results. |

### Grouping

`format::group_number(text: &str) -> Cow<str>` puts a comma between every
three digits of the integer part of a plain decimal number
(`-?[0-9]+(\.[0-9]+)?`). Anything else (an exponent, `NaN`, `Infinity`,
money with a currency sign) comes back unchanged. The fraction is never
touched.

It applies to a cell when the option is on, the column's kind is
`ValueKind::Numeric`, and the column is not a key. In the data view a key is
a column in the described structure's `primary_key` or in one of its
`foreign_keys`; until the structure is described no column counts as a key.
SQL results have no structure, so every numeric column is grouped there.

Grouping is display only: Copy, the row panel's field and an export give the
value as the server sent it.

## The window

`Dialog::Settings(Box<SettingsDialog>)`:

    pub struct SettingsDialog {
        pub tab: SettingsTab,        // General
        /// Omarchy: the option the keys act on.
        pub row: usize,
        /// The footer asked whether to reset.
        pub confirm_reset: bool,
    }

Actions: `ShowSettings`, `SetOption(OptionValue)`, `ResetOption(OptionId)`,
`ResetSettings`, `ConfirmResetSettings(bool)`, `MoveSettingsRow(isize)`,
`RevealSettingsFile`, `ExportSettings`, `EditSettingsFile`. The view pushes
them; `App::apply` changes the settings. `ShowSettings` opens the window
when no dialog is open or the shortcuts dialog is; `Escape` closes it.

The view is `src/ui/settings/`: `mod.rs` (the options, their labels and
hints, shared by both layouts), `sheet.rs` (macOS and Windows), and
`terminal.rs` (Omarchy), as the connection dialog is split.

One table in `mod.rs` describes the rows, so both layouts and the file
agree: each option's id, section, label, small print, the TOML key it is
stored under, and its default.

### macOS and Windows

A modal, as wide as the artboard's window (1040) or the app window less its
margins, as tall as its rows.

- Nav, 210 wide on the panel colour: the title "Settings", then "General",
  selected.
- Content: the title "General" and under it "How Tabletist shows data.
  Changes apply right away." Then the section label "Data" and its four
  rows. A row is a label column of 300 (the label, and small print under it
  where the artboard has some) and the control with its hint: the sample
  timestamp for the chosen precision, "Grouping is display only; copy gives
  the raw value", and two sample tags that are coloured when tags are on.
- Footer, on the panel colour: "Stored in" and the file's path with the
  home directory written `~`, then Reveal in Finder (Show in Explorer on
  Windows), Export… and Reset to defaults in the danger colour.

The toggle is a new `widgets::toggle` (the artboard's 30 by 18 switch); the
menu and the segments are `widgets::popup_button` and `widgets::segmented`.
Focus rings come from `src/ui/focus.rs`, as for every control.

Reset to defaults asks first, in place: the three links give way to "Reset
every option on this tab?" with Reset and Cancel. Reset puts the four
options back and saves. It leaves the keys the window does not show alone.

### Omarchy

The artboard's screen, over the whole window:

- Header: `settings` `general`, and at the right "changes apply right away
  · ctrl+, opens this".
- Nav, 220 wide: `general`, selected.
- Rows in three columns (the label, the value, a hint), under the section
  label `data`. The row with the keys carries the cursor mark and the
  selection fill. Values are drawn as the artboard draws each kind: a number
  between `‹ ›` when it is the cursor's row, segments with the chosen one
  filled, `[x]` and `[ ]`.
- The file pane, 620 wide or 40% of the window if that is less, hidden when
  the window is narrower than 1100. Its header is the path and `live` while
  the file is watched. Its body is `App::settings_text`, coloured by a small
  line classifier (comment, table header, key, string, number or boolean),
  with the line of the cursor's option highlighted (from `Loaded::lines`)
  and the lines in `Loaded::invalid` in red. Under it: "edits in the file
  reload live · invalid lines are shown here in red and ignored".
- Footer keys: `j/k` move, `h/l` change, `space` toggle, `ctrl+e` open file
  in $EDITOR, `R` reset option, `esc` close.

Keys, taken by the screen while it is open: `j`/`k` and the up and down
arrows move the cursor; `h`/`l` and left and right step the value through
its choices (a toggle: off and on); `space` flips a toggle; `R` puts the
cursor's option back to its default; `ctrl+e` opens the file; `Escape`
closes. A click on a row moves the cursor to it, and a click on a value
sets it.

### Opening

- `Mod+,` in `src/ui/keys.rs`, and a line for it in `SHORTCUTS`.
- A Settings button in the shortcuts dialog, beside About Tabletist.
- macOS: "Settings…" in the app menu with the key equivalent `⌘,`.

winit builds the app menu as About, separator, Services, Hide, Hide Others,
Show All, separator, Quit. `tabletist-appkit` gains
`SettingsItem::insert(title, chosen) -> Option<Self>`, which adds the item
and a separator after About's separator, the place the platform gives it.
It shares `about.rs`'s target class (renamed to say what it is: an object
that calls a closure when a menu item is chosen). Dropping the
`SettingsItem` removes both items. `src/macos.rs` wraps it as `SettingsMenu`
the way `AboutMenu` wraps `AboutItem`, and `Window::logic` turns a chosen
item into `Action::ShowSettings`.

AppKit takes `⌘,` for the menu item before egui sees it, so on macOS the key
arrives through the menu; if the item could not be inserted, `keys.rs` still
handles it. Either way one press opens the window once.

## The file's actions

All three run on the backend, as disk and process work does.

- **Reveal** (macOS, Windows): `open -R <path>`, `explorer /select,<path>`.
- **Export…**: a save dialog (`rfd`, as saving a binary value does) with
  the name `tabletist-settings.toml`, then the canonical text written to the
  chosen path.
- **Open in the editor** (Omarchy): `omarchy-launch-editor <path>` when that
  command is on `PATH`, else `xdg-open <path>`. The child is not waited for.

Reveal and the editor key save the file first when it does not exist yet.
A command that cannot be started, or an export that cannot be written, is
reported in the app's notice, as a failed save is.

## Live reload

`Command::WatchSettings { path }` starts a `notify` watcher on the config
directory (the directory, not the file: editors replace a file by renaming
another over it). An event for `settings.toml` waits 100 ms for the writes
to settle, then the backend reads the file and sends
`Event::SettingsFile { text }`.

`App` drops an event whose text equals `settings_text`: that is its own
write, or a change that changed nothing. Otherwise it runs
`Settings::from_toml`, keeps the text, the invalid lines and the key lines,
and applies the settings through the same effects a change in the window
has, without writing the file back.

The watcher starts in `App::attach` when `follow_desktop` is true, so tests
and demo mode never watch. If it cannot start (no inotify watches left) the
failure is logged and the pane's header does not say `live`; everything else
works.

When the file is deleted while the app runs, the settings in memory stay,
and the next change writes the file again.

On macOS and Windows, a file with ignored lines adds one line to the
footer: "2 lines in the file could not be read and were ignored".

## Errors and edge cases

- Two writers: the user saves the file in an editor while changing an
  option in the window. The last write wins; neither is merged.
- The config directory is read-only: the change applies for the session and
  the failed save is reported in the notice, as today.
- `page_size = 250` by hand: honoured, and shown in the menu as its own
  entry.
- `group_digits = "yes"`: ignored, red on Omarchy, counted in the footer
  elsewhere.
- A window too short for the rows: the content scrolls; the footer stays.
- A second `Mod+,` while the window is open does nothing.

## Testing

Every behaviour gets a focused test; UI behaviour goes through
`src/testing.rs`. Nothing compares a frame with the design.

- Store: defaults; `to_toml` then `from_toml` gives the same settings; the
  canonical text is stable; an unknown key and table are ignored and not
  invalid; a wrong type and a string outside its set are invalid by line
  number and keep the default; a syntax error on one line leaves the other
  lines applied; ranges clamp; `sql_timeout_secs = 0` is no timeout.
- Migration: only JSON present gives its values and asks for the TOML to be
  written; both present reads the TOML; damaged JSON is still moved aside;
  the JSON is untouched afterwards.
- Options: `group_number` on integers, decimals, negatives and the forms it
  leaves alone; a key column is not grouped and its neighbour is; SQL
  results group; Copy gives the raw value; tags off draws an enum, a CHECK
  column and a boolean plain in all three views; the timestamps option sets
  open and new workspaces; the page size reaches the next `FetchRows`.
- Window, in every look: `Mod+,` and the shortcuts dialog's button open it;
  each control changes its setting and saves; `Escape` closes; Reset asks,
  then resets only the four options. Omarchy: each key; the pane shows the
  text, highlights the cursor's line and is hidden in a narrow window.
- File actions: each pushes its command with the right path; a failure
  reaches the notice.
- Reload: an `Event::SettingsFile` with new text changes the settings and
  writes nothing; one with the app's own text is dropped; invalid lines
  reach `App`. A backend test in a temporary directory writes the file and
  receives the event.
- macOS menu: a test with its own `main`, beside `about_menu.rs`, that the
  item is inserted after About, carries `⌘,`, calls back, and is removed on
  drop. It runs in CI's macOS job; from Linux it can only be compile-checked.

`README.md` and `AGENTS.md` are updated where they name the settings file,
and the first design spec's list of files is left as the record it is.

## Delivery

Four steps, each one a pull request that passes the checks and is worth
having without the next:

1. **The store and the options.** `settings.toml`, the migration, tolerant
   reading, the three new fields and their effects. No window: the options
   are set by editing the file and take effect at the next start.
2. **The window on macOS and Windows.** `Dialog::Settings`, the General tab,
   `Mod+,`, the shortcuts dialog's button, the macOS menu item, and the
   footer's Reveal, Export and Reset.
3. **The Omarchy screen.** The rows, the keys, the file pane, and the
   editor key.
4. **Live reload.** The watcher, the event, the red lines on Omarchy and the
   footer's count elsewhere.

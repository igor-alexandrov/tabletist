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
| File | `settings.toml`, in tables. An existing `settings.json` is read whenever there is no `settings.toml`, and left in place. |
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
(through `egui_kittest`) and `notify` 8.2 (through `fastframe-theme`). The
lock resolved `toml` with `parse` and `serde` only, so the app asks for
`default-features = false, features = ["parse"]`: the default `display`
feature would pull in `toml_writer`, which is not in the lock and is not
needed, since the app writes the file itself. Their entries are added to the
lock by hand and checked with `--locked`.

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
    timestamps   = "second"  # second | full
    group_digits = false
    value_tags   = true

    [sidebar]
    show_system_schemas = false

    [editor]
    sql_limit        = 1000
    sql_timeout_secs = 30  # 0 waits forever

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
note beside the keys that have one (the values of a closed set, or what a
value that reads oddly means), two spaces after the value. It is the only writer. `StateFile::Settings` saves that text through `util::write_atomic`.

### Reading

    pub struct Loaded {
        pub settings: Settings,
        /// The file's text as read; what the Omarchy pane shows.
        pub text: String,
        /// 1-based numbers of the lines that were ignored.
        pub invalid: Vec<usize>,
        /// The line each known key was read from.
        pub lines: Vec<(Key, usize)>,
        /// Where the settings came from.
        pub source: Source,   // Toml, Json, Defaults
    }

    impl Settings {
        pub fn from_toml(text: &str) -> Loaded;
        pub fn load(dirs: &AppDirs) -> Loaded;
    }

`Key` names the eight keys above.

`from_toml` parses with `toml::de::DeTable::parse_recoverable`, which
returns a document and its errors. Keys, values and errors carry byte spans;
a line number is one more than the count of newlines before a span's start,
and `invalid` holds each line once. An error without a span marks no line
and is logged.

The parser's own recovery is not enough: after a line with a key and no `=`
it reads nothing more, and for a value it rejects (`timestamps = full`,
unquoted) it still returns a guess. So `from_toml` empties every line the
parser rejected, keeping its line break, and parses again until nothing more
is rejected. One bad line then costs that line and nothing else, and a
rejected line is never applied. `Loaded::text` stays the file's own text.

Two kinds of bad line cost more than themselves. A value that never closes
(a multi-line string left open) costs the lines under it, which are all
marked. A table header the parser rejects can cost the keys under it without a
mark for them: they are read into the table above (or into none), and count
only if that table knows them.

The JSON is read at every start that finds no `settings.toml`, not only the
first: after the TOML is deleted, or kept aside as `.bad`, the next start
carries the old JSON over again. That is kept: the reset a user reaches for
is the window's Reset to defaults (step 4), which writes a `settings.toml` of
defaults, so the JSON does not come back that way; and a user who deletes
the TOML by hand gets the settings they had before the TOML existed, which
is a defensible reading of "start over".

Then, for each known key:

- A value of the wrong type, or a string outside a closed set
  (`timestamps = "minute"`), is ignored: the key keeps its default and its
  line joins `invalid`.
- A number outside its range is clamped, as today. It is not invalid.
- An unknown key or table is ignored and is not invalid: a newer version may
  have written it.

Lines the parser rejected join `invalid` too. Every ignored line is logged
with its number when the file is read, so a typo is never silent, even
before a window shows it.

The reader takes a pass over the text for each bad line at worst, so it is
bounded by what a settings file can be: a text of more than 1,000 lines or
64 KiB is not read at all. The defaults are used and every line of it that
says something is marked ignored. The log names up to twenty ignored lines;
more are told by their number.

`load` picks the source:

1. `settings.toml` exists: read it (`Source::Toml`). A file that is not
   UTF-8, or that cannot be read at all, is moved aside as
   `settings.toml.bad`, as `util::load_json` does with a damaged JSON file,
   so the next save cannot replace it; the defaults are used.
2. Else `settings.json` exists: read it as today (`util::load_json`, so a
   damaged one is still moved aside as `.bad`), giving `Source::Json`. The
   JSON is not removed or changed, so an older Tabletist still finds its
   file.
3. Else: the defaults (`Source::Defaults`).

`load` writes no settings: the only thing it does to the disk is move a
file it cannot use aside. `App::new` takes the `Loaded`; when its source is `Json`
it sends `Command::Save` with `StateFile::Settings` once, which writes
`settings.toml`. With `Source::Defaults` nothing is written until a setting
changes or an action needs the file (Reveal, the editor key).

For `Json` and `Defaults`, `Loaded::text` is `to_toml()` of the settings:
what is, or would be, written.

A file with invalid lines is left as the user wrote it until the app next
writes a setting. That write is the canonical text, so the invalid lines go.

## The options

Two functions, one inside the other:

- `App::apply_settings(&mut self, new: Settings)` replaces the settings and
  runs the effects below for the fields that differ. It writes nothing.
  Every change goes through it, wherever it comes from.
- `App::change_settings(&mut self, change: impl FnOnce(&mut Settings))` is
  a change made in the app (the window, the SQL editor's menus): it calls
  `apply_settings`, renders the canonical text, keeps it as
  `App::settings_file` (`SettingsFile { text, invalid, lines, live }`, what
  the app holds of the file), with no invalid lines and the key lines of the
  new text, and saves. A change that changes nothing writes nothing.
  `App::save_settings` becomes private: only `change_settings` and the start
  that read the old JSON call it.

A change that arrives from the file (Live reload) calls `apply_settings`
only, and keeps the file's own text, invalid lines and key lines.

A grid keeps its fitted column widths in egui's memory under its id, which
is why `full_precision` is part of that id: wider timestamps fit the columns
again. Grouped numbers and value tags change a cell's width too (a tag adds
its padding), so once these options can change while a grid is open (step 2
onward) they join the grid ids, or `apply_settings` forgets the grids'
widths. In step 1 the options are fixed for a session and nothing is needed.
A view remembers which grid it last drew and forgets the one before it when
the id changes (`grid::keep`), a table as a SQL result does: an option set
back to what it was fits the rows then on screen, not the page its grid last
saw.

| Option | Control | Effect |
|---|---|---|
| Rows per page | A menu: 100, 300, 500, 1,000, 5,000. A value from the file that is not in the list is shown as an extra entry. | A table's `query.limit` is the size of the page it shows or awaits, and `fetch_rows` brings it to the settings' size each time it fetches, dropping a page of another size first, whoever asked for the fetch (a Refresh and a reconnect keep the page they have otherwise); Next moves by `query.limit` (the page on screen) and Previous by the settings' size (the page it is about to fetch, which must end where the one on screen begins), so no row is skipped in either direction whichever size comes next. Nearer the start than one page, Previous fetches only the rows before the page on screen: a shorter leading page, whose limit is its own size, so Next from it comes back to where the user was. On a change, every table that shows a page or waits for one on a connected session drops the page it shows (as Next does, so a failed fetch leaves no page of the old size on screen) and fetches again from the offset it is at. The others take the size at their next fetch. New tabs open with it. |
| Timestamps | Two segments: To the second, Full precision. | Sets `full_precision` on every open workspace and on new ones. The grid's own link still switches one workspace until the option changes again. |
| Numbers | Two segments, each showing a sample: `1,240.50`, `1240.50`. | Grid cells of numeric columns, in the data view and in SQL results. |
| Value tags | A toggle. | Off: enum, CHECK and boolean columns draw as plain text in the data view and the row panel, and booleans in SQL results (the only tags that view has). |

The keys without a control have effects too, for a change that comes from
the file: `show_system_schemas` is read when the sidebar draws and when
objects are listed, so it shows at the next frame and the next listing, and an open completion list is worked out again, since it offers the schemas that are shown;
`sql_limit` and `sql_timeout_secs` reach the SQL tabs opened afterwards, as
today; `appearance.theme` restarts the theme catalog with the new selection
and resolves the palette again. Restarting the catalog needs the egui
context, which `apply_settings` does not have: it sets a flag that
`App::logic` reads beside `themes.needs_reload()`. Where the desktop is not
followed (tests, demo mode) the catalog never starts and only the palette
is resolved.

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
A numeric column that holds years shows `2,024` with the option on; that is
what the option means, and it is off by default.

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
  the file is watched. Its body is `App::settings_file.text`, coloured by a small
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

- **Reveal**: `Command::OpenSettingsFile { path, text, how: Reveal }`.
- **Open in the editor**: the same command with `how: Editor`.
- **Export…**: `Backend::save_bytes`, the path a binary value is saved by
  (its dialog title becomes a parameter), with the name
  `tabletist-settings.toml` and the canonical text.

`OpenSettingsFile` carries the canonical text: the backend writes it to
`path` when no file is there, then starts the program, so the UI thread
never looks at the disk and the write always comes first. The child is not
waited for. `Event::SettingsFileOpened { result }` puts a failure in the
app's notice, as a failed save is; a failed export is already reported that
way.

The program follows the operating system the app was built for (`cfg`), not
the look, so a macOS look drawn in a Linux test still compiles and runs:

| | Reveal | Editor |
|---|---|---|
| macOS | `open -R <path>` | `open -t <path>` |
| Windows | `explorer /select,<path>` | `explorer <path>` |
| Linux | `xdg-open <the directory>` | `omarchy-launch-editor <path>` when it is on `PATH`, else `xdg-open <path>` |

The Reveal link's words follow the same `cfg`: Reveal in Finder, Show in
Explorer, Show in folder.

## Live reload

`Command::WatchSettings { path }` starts a `notify` watcher on the config
directory (the directory, not the file: editors replace a file by renaming
another over it). An event for `settings.toml` waits until the file has
been quiet for 100 ms (each further event starts the wait again: a save is
several of them, and a read between two would see half a file), then the
backend reads the file (a read that another change overtook is thrown
away, and the wait starts over: what was read may be half a save) and sends
`Event::SettingsFile { text }`. A file that is not UTF-8 at that moment is
logged and nothing is sent: the settings in memory stay.

The watcher ignores events that only say the file was looked at: Linux
reports every open, and the backend's own read is one, so a watcher that
answered them would read the file for ever. The file is matched by its name
in the directory, since the paths of events come as the system has them;
on macOS and Windows, which find a file whatever the case of its name, the
match ignores case, so a file kept as `Settings.toml` is still followed.
Beyond ASCII those systems' own rules cannot be reproduced here (Windows
compares by an upper case of its own, under which `Σ.toml` and `ς.toml` are
one name; macOS takes a letter with its accent for the same however the two
are encoded), so two names that both go beyond ASCII are taken for one
there. A name taken for the file's by mistake costs one read, and the same
text is not sent again.
The same text is never sent twice in a row.

The file is read once when the watch starts, as if it had just changed:
the settings were loaded before the window existed, and an edit made in
between would otherwise go unseen until the next one. The app drops a text
equal to the one it holds, so an unchanged file costs nothing.

A read that fails for a reason that may pass (an editor still holding the
file, on Windows) is tried again, waiting 100 ms and then twice as long each
time up to a second, six reads in all; a change meanwhile starts over, and a
file that is gone is not retried.

`App` drops an event whose text equals `settings_file.text`: that is its own
write, or a change that changed nothing. Otherwise it runs
`Settings::from_toml`, keeps the text, the invalid lines and the key lines,
and applies the settings through the same effects a change in the window
has, without writing the file back.

The watcher starts in `App::attach` when `follow_desktop` is true, so tests
and demo mode never watch. The backend answers with
`Event::SettingsWatch { live: bool }`. If it cannot start (no inotify
watches left) the failure is logged and the pane's header does not say
`live`; everything else works.

`settings.toml` may be a symbolic link: GNU stow, dotbot and chezmoi in
symlink mode keep the file in a repository and a link in the config
directory. An edit made in the repository raises its events there and none
beside the link, so the watcher resolves the link (`util::link_chain`,
the links at the end of the path, one after another) and, when the file is
in another directory, watches that directory too. Where one link leads to
another, the directory of each is watched, since any of them can be
turned. The reader wakes for any of their names: the link's in the config
directory, the file's where it is, and those of the links between.
Each time it wakes it resolves the link again before it reads, so a link
made, turned elsewhere or replaced by a plain file while the app runs is
followed. Once the watches are in place it resolves once more, and again
until it finds what it watched: a link turned between the look at it and
the watch on its directory would otherwise be missed for good. A link that leads where nothing can be watched (the directory is
not there, or no watch can be had on it) is logged and
`Event::SettingsWatch { live: false }` is sent. While it is not live the
watcher looks again every two seconds, since no event comes when the place
appears (a volume is mounted), and sends `true`, with the file's text, once
it can watch there. A link turned by hand among the directories on the way
(a directory that is itself a link), with nothing changing beside
`settings.toml` or a link it leads through, is not seen until the next
change that is.

A watch stays with the directory it was put on, not with its path. When a
watched directory is moved or removed (a dotfiles repository set aside and
cloned anew, the link as it was), the watcher says so for the directory
itself. The reader is woken for that too, and before it reads it lets go
of every watch and takes them again at the paths, so the directory now
there is watched; while none is, the watch is not live and is looked for
again as above. Known limits: a directory further up the path that is
moved (`~/dotfiles` where `~/dotfiles/tabletist` is watched) raises
nothing on Linux, and this is tested on Linux alone: where a system's
watcher says nothing of the watched directory itself, edits made in the
new one are not seen until the app starts again.

When the file is deleted while the app runs, the settings in memory stay,
and the next change writes the file again.

On macOS and Windows, a file with ignored lines adds one line to the
footer: "2 lines in the file could not be read and were ignored".

## Errors and edge cases

- Two writers: the user saves the file in an editor while changing an
  option in the window. The last write wins; neither is merged.
- Two quick changes in the app can be read from the disk between their two
  writes. The first text then comes back as if from outside and is applied
  for a moment, until the second follows; nothing is written by either. A
  third change made in the app inside that moment would be built on the
  older settings and lose the second. Menus cannot be clicked that fast, but
  a key held down on an option can: steps 3 and 4 must close this before
  they let a key change a setting (hold the newest text until its own write
  has come back, or step a value no faster than its save).
- A change from outside that restores, within the settle after one of the
  app's own writes, the very text the backend sent last is not seen: the
  reader never sends the same text twice in a row. The app then holds the
  newer settings and the disk the older, until the next change of either.
- A state file that is a symbolic link (`settings.toml`, and
  `connections.json` and `known_hosts.json` alike) is saved through the
  link: `util::write_atomic` resolves it and makes its temporary file beside
  the file the link leads to, so the rename replaces that file and the link
  stays. The write is as atomic as before, the temporary file still has a
  random name and is made exclusively (a link planted under a temporary
  name is not followed, in either directory), and on Unix the new file is
  0600 where it lands, in the repository too. What no longer holds is that
  a link at the path itself is never written through: that is now the
  point, and whoever can put a link in the config directory (0700, the
  user's) decides which file a save replaces. A link to a directory that is
  not there is not followed by making the directory: the save fails, is
  reported in the notice, and the link stays. So does a save where the
  repository cannot be written. A file behind a link that cannot be loaded
  is renamed to `.bad` beside itself, not the link. Where the link cannot
  be followed (what it leads to cannot be looked at for now, or the links
  lead back to themselves) nothing is renamed and the link stays: a save
  through it fails for the same reason, so nothing is replaced.
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
  column and a boolean plain in the data view and the row panel, and a
  boolean plain in SQL results; the timestamps option sets open and new
  workspaces; a new page size fetches an open tab's page again from its
  offset with the new limit, and Next then moves by that limit.
- Window, in every look (once both layouts exist): `Mod+,` and the
  shortcuts dialog's button open it;
  each control changes its setting and saves; `Escape` closes; Reset asks,
  then resets only the four options. Omarchy: each key; the pane shows the
  text, highlights the cursor's line and is hidden in a narrow window.
- File actions: Reveal and the editor key push `OpenSettingsFile` with the
  path, the text and the right `how`; Export reaches `Backend::save_bytes`
  with the name and the text; a failure reaches the notice. A backend test
  in a temporary directory checks that a missing file is written before the
  program starts.
- Reload: an `Event::SettingsFile` with new text changes the settings and
  writes nothing; one with the app's own text is dropped; invalid lines
  reach `App`. A backend test in a temporary directory writes the file and
  receives the event.
- macOS menu: a test with its own `main`, beside `about_menu.rs`, that the
  item is inserted after About, carries `⌘,`, calls back, and is removed on
  drop. It runs in CI's macOS job; from Linux it can only be compile-checked.

Neither `README.md` nor `AGENTS.md` names the settings file, so neither
changes; the first design spec's list of files is left as the record it is.

## Delivery

Four steps, each one a pull request that passes the checks and is worth
having without the next. Reload comes before either window, so the first
window to ship can already say `live` and mean it.

1. **The store and the options.** `settings.toml`, the migration, tolerant
   reading with ignored lines logged, the three new fields and their
   effects. No window: the options are set by editing the file and take
   effect at the next start.
2. **Live reload.** The watcher, `Event::SettingsFile`, `apply_settings` for
   every key. An edit to the file takes effect in the running app.
3. **The Omarchy screen.** `Dialog::Settings`, the rows, the keys, the file
   pane with its highlighted and red lines, the editor key, `Mod+,` and the
   shortcuts dialog's button. The key's line in `SHORTCUTS` waits for step
   4, so no look lists a key that does nothing there. Until step 4, both ways in are offered only
   in the terminal look: in the other looks the key does nothing and the
   button is not drawn.
4. **The window on macOS and Windows.** The sheet, the toggle, the footer's
   Reveal, Export, Reset and count of ignored lines, the macOS menu item,
   the ways in for every look, and the `SHORTCUTS` line.

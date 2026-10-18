# Settings › General: the six rows that were left out

Date: 2026-10-17. Status: draft, for Igor's review.

## Intent

The Settings window and its General tab shipped with the 2026-10-03 spec
(`2026-10-03-settings-general-design.md`): the Data rows Rows per page,
Timestamps, Numbers and Value tags, in both looks, with the file pane, live
reload, Reveal, Export…, Reset and the `$EDITOR` key. That spec left out six
rows of the "Settings, General" artboards because their features did not
exist: On launch, Restore tabs and filters, Time zone for timestamptz, New
production connections open read-only, Ask before closing with unsaved
edits, and Disconnect when idle.

This spec adds those six rows and their features, changes how the settings
file is written (only values that differ from the defaults, with the
person's comments and unknown keys kept), and brings Rows per page to the
artboard's list, default and timing.

Success: every row of both artboards is in the window, does what its helper
text says, and is stored in `settings.toml` under the key below; a file a
person wrote keeps its comments when the app writes it.

The designs are the artboards `macOS – Settings, General` and
`Omarchy – Settings, General` in the design canvas Artifact. They are not
copied into the repository.

## Where this departs from the artboards, the prompt or the app

Each row needs a yes or a no before its step is planned. The first rows
were approved by Igor on 2026-10-17; the three rows after the writer's were
settled while step 1 was drafted, and are put to him with its plan.

| What the artboard, prompt or app has | What will be built | Why |
|---|---|---|
| Omarchy board: the file pane shows every key with its default | The file holds only values that differ from the defaults (Igor, 2026-10-17). With every option at its default the pane shows the header line alone, and the cursor's option has no line to highlight | The prompt's rule, chosen over the board |
| 2026-10-03 spec: the app writes one canonical text and drops comments | The person's text is edited in place: comments, key order, unknown keys and ignored lines stay. The `toml` parser the app already uses gives the spans to edit at, so no crate is added (the prompt named `toml_edit` as an example) | The prompt's rule (Igor, 2026-10-17) |
| The prompt: macOS keeps the file in `~/Library/Application Support/Tabletist/` | It stays in `~/Library/Application Support/dev.tabletist.Tabletist/`, with every other file of the app | Moving it moves every file, and needs a migration of its own |
| 2026-10-03 spec: Export saves the canonical text | Export saves a copy of the file as the app holds it | The prompt: "Export…, which saves a copy of the file" |
| App today: a new Rows per page fetches every open table again at once | It applies when a table next loads (opened, refreshed, sorted, filtered). Next and Previous keep the size of the page on screen | The prompt's rule (Igor, 2026-10-17) |
| App today: page sizes 100, 300, 500, 1,000, 5,000, default 300 | 100, 500, 1,000, 5,000, 10,000, default 1,000, as drawn. A file that says 300 keeps 300, shown as an extra entry | The artboard |
| Numbers: macOS board grouped, Omarchy board not | The default follows the look: grouped on macOS and Windows, not on Omarchy (open question 1 of the prompt) | As drawn, until Igor picks one |
| Omarchy board: `on_launch = "conn:Bookshop"` | `on_launch = "conn:<connection id>"  # Bookshop` (open question 2) | Three connections can share a name |
| "Restore last session reopens the windows" | The app has one window: its connection tabs are what is reopened | There is one window |
| Restore covers "tabs, sort, filters and hidden columns" | Tabs, sort and filters. Hiding columns is not built, so nothing of it is saved | No feature to restore |
| "Off means each connection opens with one table tab and no filters" | Off, a connection opens as it does today: its tree, and no table tab until one is opened | Today a connection opens no table by itself, and none is "the" table to open |
| Restored tabs | Table and view tabs. SQL editor tabs are not restored | The prompt names tabs, sort and filters, all of a table's |
| Omarchy board: `timestamptz zone` as a value | `h/l` steps System, UTC and the zone the file names. Any other zone is set in the file (`ctrl+e`) | Stepping through 600 zones is no control |
| The deleted chosen connection "says so once" | The app says so in its notice and sets On launch back to Show Connections, which writes the file | What the menu then shows is what the app does, and the notice has nothing to repeat |
| Ask before closing, off | The held action goes ahead and the edits are discarded, without asking. A save that is still running is still waited for | Off means no question. A running save cannot be discarded |

## Delivery

Three steps, each its own plan, run and pull request, each shippable on its
own (Igor, 2026-10-17):

1. **The file and the Data rows.** The writer, Rows per page, Numbers by
   look, Time zone for timestamptz, and the reset question's new words.
2. **Safety.** Production opens read-only, Ask before closing with unsaved
   edits, Disconnect when idle.
3. **Startup.** The session file, Restore tabs and filters, On launch.

The Safety and Startup sections are drawn once their first row is built.

## The schema

| Section | Key | Values | Default | Step |
|---|---|---|---|---|
| Startup | `startup.on_launch` | `"restore"`, `"connections"`, `"conn:<id>"` | `"restore"` | 3 |
| Startup | `startup.restore_tabs` | boolean | true | 3 |
| Data | `data.page_size` | integer, clamped to 10..=10,000 | 1000 | 1 |
| Data | `data.timestamps` | `"second"`, `"full"` | `"second"` | (built) |
| Data | `data.tz` | `"system"`, `"utc"`, an IANA name | `"system"` | 1 |
| Data | `data.group_digits` | boolean | the look's: true on macOS and Windows, false on Omarchy | 1 |
| Data | `data.value_tags` | boolean | true | (built) |
| Safety | `safety.production_read_only` | boolean | true | 2 |
| Safety | `safety.confirm_close_edits` | boolean | true | 2 |
| Safety | `safety.idle_disconnect` | `"never"`, `"5m"`, `"15m"`, `"30m"`, `"1h"`, `"4h"` | `"30m"` | 2 |

The keys the window does not show (`sidebar.show_system_schemas`,
`editor.sql_limit`, `editor.sql_timeout_secs`, `appearance.theme`, `mcp.*`)
keep their meaning and are written by the same rules.

A value outside its set (`tz = "Mars/Olympus"`, `idle_disconnect = "2m"`,
`on_launch = "conn:"`) is invalid: the line is ignored and the key keeps its
last good value, as for the keys that exist. A time zone is valid when jiff's
database knows it.

## Step 1: the file and the Data rows

### Writing the file

`Settings::write_into(&self, text: &str, keys: &[Key]) -> String` is the
one writer; `to_toml` is `write_into("", &Key::ALL)`.

- The text the app holds (`SettingsFile::text`) is read by the `toml`
  parser, which gives each key's, value's and table header's byte span, and
  edited at those spans. Lines the parser rejects are blanked to spaces of
  their own length first, so every span is where it is in the person's
  text: those lines stay through the write, and are still ignored.
- Only the keys in `changed` are touched: a change in the window changes one
  key, Reset changes the General tab's keys. A key the person wrote at its
  default by hand stays as they wrote it until it is changed.
- A key set to its default is removed, with its line; where a rejected
  line names the same key, its default is written in place instead, or
  the rejected line would be the one read. A table left with no
  key and no comment is removed. A key set to another value replaces the
  value in place, keeping the comment beside it; a key not in the file is
  added at the end of its table, with its note (`# second | full`) beside
  it; a table not in the file is added at the end of the file. A file left
  with nothing in it stays empty.
- A table written inline (`data = { … }`) or with dotted keys gets a value
  replaced in place; to add or take out a key there, the whole file is
  written as `to_toml` has it.
- With no file (`Source::Defaults`) the text starts as the header line
  `# written by tabletist, safe to edit by hand`.
- Unknown keys, unknown tables and every comment stay where they are. A
  table's name the file gives a value (`data = 5`) gets nothing written,
  and its keys keep the values they had.
- The text is written atomically through `util::write_atomic`, as now.

`Loaded::text` for `Source::Defaults` becomes the header line alone; for
`Source::Json`, the header and the JSON's values that differ from the
defaults.

The defaults depend on the look for one key, so `group_digits` becomes an
`Option<bool>`: `None` is the look's default, read through
`Settings::grouped(layout)`. A value set equal to the look's default is
stored as `None` and its key removed. A person who switches looks keeps what
they set and gets the new look's default for what they did not.

What tells the app's own writes from someone else's (`saved`, `offered`,
`own`) is unchanged: it compares texts.

### Rows per page

`Settings::PAGE_SIZES` becomes 100, 500, 1,000, 5,000, 10,000 and
`DEFAULT_PAGE_SIZE` 1,000.

`App::resize_pages` goes: a change does not fetch anything. A table takes the
settings' size when it next loads from a place: opened, refreshed, sorted,
filtered, reconnected. Next and Previous keep the size of the page on
screen (`query.limit`), so the rows a person is paging through do not jump.
The tests of the re-fetch at once are rewritten to say the opposite.

### Numbers by look

The menu's left segment stays grouped. The window shows the value in force
for the look (`grouped(layout)`); Reset sets `None`.

### Time zone for timestamptz

`Settings::tz: Zone`, with `Zone::System`, `Zone::Utc` and
`Zone::Named(String)`.

- **Which values.** A temporal value whose text is a date, a time and an
  offset (`2026-01-12 09:14:03.482915+00`, as PostgreSQL sends a
  `timestamptz` under `DateStyle = ISO`) is an instant, on PostgreSQL
  (`edit::shows_in_zone`, every `Dialect` named). A SQLite or MySQL text
  with an offset is text the person stored, shown and edited as it is; a `timetz` has no date
  and is left alone. Text that does not parse (`infinity`, a BC date) is
  shown as sent.
- **Display.** The instant is shown in the chosen zone, with that zone's
  offset, and the fraction the server sent:
  `2026-01-12 04:14:03.482915-05`. Timestamps, To the second, then cuts the
  fraction as today. The grid, the SQL results and the row panel show it.
- **The zone database** is read whole on the backend when the app starts
  (`Command::LoadZones`, `zones::Zones`): the UI thread reads no zone
  file. Reading `settings.toml` checks a name's form only; a name the
  database does not know is an ignored line once the database is read,
  and a named zone shows values as sent until then.
- **System.** The computer's zone is read once, when the app starts
  (`App::system_zone`), never per frame; tests have UTC. The menu writes it
  `System · America/New_York`.
- **Raw values.** Copy, Save value and what an edit is compared with are
  the server's text. The editor opens on the moment as the cell shows it,
  in the zone; a text that names the loaded moment another way (another
  zone, `Z`) is no change, in a column of dates and times. Nothing is
  fetched again when the option changes.
- **Sent with their offset.** A value typed into a `timestamptz` column of a
  table (the column's described type) with no offset is given the chosen
  zone's offset for that moment before it is written, so the server reads
  it in the zone the person sees. A value with an offset is sent as typed.
  SQL in the editor is the person's own and is not touched.
- **macOS control.** A menu with a search field: System, UTC, then every
  zone of jiff's database in alphabetical order, filtered as the person
  types. Helper text: "Values are shown in this zone and sent with their
  offset".
- **Omarchy control.** `h/l` steps System, UTC and the last named zone the
  session had, so a step off the file's zone can step back to it. The value column writes `system (america/new_york)`,
  the hint `sent with offset`.

### Reset

The question becomes "Reset all General settings to their defaults?". Reset
puts back every option of the General tab, including the rows later steps
add, and touches no other key.

### Tests (step 1)

- No file: the defaults apply and nothing is written until a change; the
  text held is the header alone.
- Each Data key round-trips through `write_into` and `from_toml`. Writing a
  default removes the key; a comment above a key, a comment beside it, an
  unknown key and an unknown table survive a write; a new key lands in its
  table.
- `group_digits` reads as grouped on macOS and Windows and not on Omarchy
  with no key, and a value equal to the look's default writes no key.
- An invalid `tz` is ignored by its line, keeps the last good zone, and is
  reported (red line on Omarchy, the footer's count on macOS).
- A hand edit to `tz` applies live.
- `book_covers.created_at` as a `timestamptz` page of the Bookshop test data
  (`src/testing.rs`): UTC, a named zone and System (pinned in the test)
  change the cell, the row panel and a SQL result, and Copy gives the
  server's text. No fetch is sent on the change.
- A typed value with no offset is written with the zone's offset; one with
  an offset is written as typed.
- A new page size fetches nothing; the next Refresh and a new tab take it;
  Next keeps the size on screen.
- Reset puts back the General options and leaves `editor.sql_limit` alone.

## Step 2: Safety

### New production connections open read-only

`safety.production_read_only`, default true. Helper text: "Default for the
switch in each connection".

Today a connection whose `read_only` is unset follows its environment
(`Environment::read_only_by_default`: production is read-only). That rule
stays for every saved connection, so a change of the option changes none of
them. A new connection (the connection dialog with nothing being edited)
whose environment is production starts its switch at the option's value,
and is saved with `read_only` set, so it keeps it. Editing an existing
connection is unaffected.

### Ask before closing with unsaved edits

`safety.confirm_close_edits`, default true. The row is shown: editing is
built.

The prompt (`Dialog::Leave`) is asked today when a tab, a connection or the
window closes, or the table reloads, pages, sorts, filters or switches
database, while edits are pending. Off, `App::hold` lets the action go on
and discards the edits, as Discard would. While a save is still running the
action is held as today, since a save cannot be discarded.

### Disconnect when idle

`safety.idle_disconnect`, default `"30m"`. Helper text: "Closes server
sessions; tabs stay and reconnect on use". The menu writes "Never", "After
5 min" … "After 4 h"; Omarchy writes `30m`.

- `Workspace::used: Instant` is set whenever the app sends a command for its
  session and when its connection tab is drawn active with input.
- `App::close_idle(now: Instant)`, called from `logic`, closes the session
  of each connected workspace idle for longer than the option: it sends
  `Command::Close` and sets a new `SessionStatus::Idle`, which draws no error
  banner: the status line says "Disconnected while idle".
- **Never** with pending edits (any object tab whose `edits.holds()`), a
  run, a fetch or a save in flight (`App::is_busy`), or a connection not in
  `Connected`. The SQL editor cannot hold a transaction open (each script is
  its own and `BEGIN` is refused), so there is no open transaction to check;
  should that change, the transaction joins the list.
- **Reconnect on use.** An action that needs the session of an `Idle`
  workspace reconnects it (`App::reconnect` with the secrets the tab keeps)
  and is queued; `after_connect` replays the queue, which already reloads
  the active page. A reconnect that fails shows the usual disconnected
  banner.
- Agents' sessions are the gateway's, closed after its own 10 minutes, and
  are not touched.
- Tests pass `now` and never sleep.

### Tests (step 2)

- The option changes the switch of a new production connection only; an
  existing one with `read_only` unset keeps the built-in rule.
- Off, closing a tab with pending edits closes it without a prompt; a
  running save still holds.
- Idle: a session is closed after the time; never with pending edits, a run
  in flight, or the option at Never; the next fetch reconnects and is
  answered.

## Step 3: Startup

### The session file

`session.json` in the state directory (`AppDirs::session_file()`), written
atomically through `Command::Save` (`StateFile::Session`), JSON as the
other state files are, with a `version`:

    {
      "version": 1,
      "open": ["<connection id>", "<connection id>"],
      "active": 1,
      "connections": {
        "<connection id>": {
          "tabs": [
            { "schema": "public", "name": "books", "kind": "table",
              "sort": [["title", "asc"]], "filter": "…" }
          ],
          "active": 0
        }
      }
    }

- `open` is the connection tabs open at quit, in order; a picker tab is
  `null`.
- `connections` keeps each connection's last tabs, written when its
  connection tab closes and at quit, whatever On launch says. A deleted
  connection's entry is dropped.
- The filter is kept as the filter bar holds it.
- The file is written at quit and when a connection tab closes, not on every
  change.

### Restore tabs and filters

`startup.restore_tabs`, default true. Helper text: "Per connection,
including sort and hidden columns" (word for word, though hiding columns is
not built). On, a connection that opens (at launch, from the picker,
anywhere) reopens its saved tabs with their sort and filters, and the one
that was active is fetched. A table that no longer exists opens as a tab
that says so, as an open tab does when its table is dropped. Off, it opens
as today.

### On launch

`startup.on_launch`, default `"restore"`. The menu: Restore last session,
Show Connections, then each saved connection by name, with its host and
database beside the name so three Bookshops can be told apart. The hint
"Or: show Connections · open a chosen connection" is the artboard's.

- **Restore last session.** The connection tabs in `open` connect again, in
  order, each asking for what it needs (a password not saved, a host key)
  as a connection does; the active one is active. With no file, the
  picker.
- **Show Connections.** The picker, as today.
- **Open a chosen connection.** That connection opens, by its id. The file
  writes `on_launch = "conn:<id>"  # <name>`. A connection that is gone:
  the picker opens, the notice says "The connection chosen to open at
  launch was deleted. Tabletist showed Connections instead.", and the
  option is set to Show Connections.
- `--demo` keeps its own start.

### Tests (step 3)

- Each value of On launch at `App::new`: restore opens the saved tabs, in
  order; connections opens the picker; a chosen id opens that connection; a
  deleted id falls back, says so once, and writes `"connections"`.
- The id is what is stored for a chosen connection, with the name as its
  comment, and two connections of one name are told apart.
- Restore on: a connection opened again gets its tabs, sort and filter.
  Off: it opens with none.

## Accessibility

Each control is named by its row's label and described by its helper text.
Segmented controls are radio groups, as the built ones are. A change says
nothing beyond the control's own state.

## Keys

All built: `Mod+,` opens Settings, `⌘W` closes the window on macOS;
Omarchy's `j/k`, `h/l`, `space`, `ctrl+h/l`, `ctrl+e`, `R` and `esc` are
in the keymap. No key is added.

## The website

`_reference/settings-and-files.md` gains the new keys, the writing rules and
`session.json`, in the step that adds each.

## Out of scope

The other Settings tabs, the Agents tab's contents, syncing settings across
machines, per-connection overrides beyond the read-only switch, hiding
columns, and restoring SQL editor tabs.

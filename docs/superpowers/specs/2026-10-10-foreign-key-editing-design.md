# Editing foreign key values: the search picker

Date: 2026-10-10. Status: built in runs. Run 1
(`docs/superpowers/plans/2026-10-10-foreign-key-editing-1-lookup.md`) is
built: the three drivers look a referenced row up, the backend runs it,
and SQLite names the column an implicit reference means. Nothing in the
app asks for a lookup yet. What is left is listed under "Delivery".

This is one editor of slice 2 of `2026-10-03-value-editing-core-design.md`
("Editors by type"): the foreign key search. The other editors of that
slice (enum and CHECK lists, the calendar, JSON highlighting, array chips,
binary from a file) are not in this spec. `2026-10-07-inserting-rows-design.md`
(section 3) and `2026-10-06-row-inspector-inline-edit-design.md` (section 4)
both name this picker and leave it to slice 2.

The designs are in the design canvas Artifact and are not copied into the
repository.

## Goal

A cell of a foreign key column holds a key of another table's row: a number
or a UUID that says nothing to a reader. Today it is edited as text, so the
user has to know the id, or look it up in another tab and copy it.

With this spec the editor of such a cell searches the referenced table as
the user types, by a readable column and by the key, and sets the cell to
the key of the row the user picks. A typed or pasted key is still taken as
it is, and one that names no row is said so before the save.

Success: on each driver, a user opens `book_covers` on a writable
connection, presses Enter on a `publisher_id` cell, types `harb`, sees
"Harbor Press" and "Harbor Lights" with their ids, presses Enter, and the
cell is pending with Harbor Press's id. The same works in a new row and in
the row panel. Typing an id no publisher has leaves the cell pending with a
line that says no publisher has it, and Save still offered.

## Design references

Canvas "Tabletist", sections **Editing · Rows**, **Editing · Values** and
**Editing · Inserting rows**:

- `macOS – Editing values, editors by type` (MacEditTypes), card 6: the
  field with what was typed, the list under it, an option of two lines (the
  label with its match in bold, the key under it in the code face, muted).
  Its note: "Foreign keys search the referenced table by its first text
  column (name, title, email) and by id. Pasting an id that doesn't exist
  is flagged before save."
- `macOS – Editing a row` (MainEdit): the picker under a grid cell's
  editor. The editor has a search icon and is named "Search publishers for
  publisher_id". The list is named "Matching publishers", has a head line
  ("publishers matching "harbor" · ↑↓ to choose · Enter to set") and a foot
  with "Esc cancels" and a **Set NULL** button. The row panel's field says
  "Editing in the grid…".
- `macOS – Inserting a row` (MacInsert): the same list under a new row's
  cell, with the head line `publishers matching "harb" · ↑↓ choose · ↩ set`.
- `macOS – Inserting rows, flow and states` (MacInsertStates), card 5: a
  save the database refused, "No publisher with id 9100000000000000099".
- `macOS – Table view, editable` (Main): the row panel's
  "Open publisher →" link, which stays.
- `Omarchy – Editing values, editors by type` (OmarchyEditTypes), card 6: a
  float with a `> harb` prompt line, a count at its right, options of one
  line (label, then the key muted), the match in the accent colour. Its
  note names `gd`, which opens the referenced row.
- `Omarchy – Editing values` and `Omarchy – Inserting rows`: the same list
  in the flow of an edit and under a new row.
- `Omarchy – Components`: "pickers float over the pane with an accent
  border. j/k or ctrl+n/p move, enter picks, esc closes. Typing filters;
  matches are highlighted in accent."

## Decisions

| Question | Decision |
|---|---|
| Which columns | A column that is the one column of a foreign key. The columns of a composite key stay text editors (asked on 2026-10-10). |
| Matching | A substring search the database runs: the label contains the text, or the key starts with it. The same in both looks (asked on 2026-10-10). |
| A key that names no row | A warning on the cell. Save stays offered and the database has the last word (asked on 2026-10-10). |
| What the cell shows | The key, as today. The label shows only in the list. |
| The label column | The referenced table's first text column that is not the key. None: the list shows keys alone. |
| How many | The first 50 matches. More are said, not paged. |

Where the boards disagree, or ask for what this spec does not build:

| The boards | This spec | Why |
|---|---|---|
| Omarchy: "fuzzy search", and a count `2/48` | Substring, and the count of matches: `2`, or `50+` | One behaviour for 48 rows and for 40 million, and the table's size is not known without a count |
| Omarchy components: `j`/`k` move in a picker | Ctrl+N, Ctrl+P and the arrows | The field has the keyboard, so `j` and `k` are letters of the search |
| MainEdit marks the match in yellow, the two later boards in bold | Bold (the accent colour in the Omarchy look) | The later boards agree |
| MainEdit's pending bar says "· publisher_id still being edited" | Not built here | The app has no such line for any cell. It is the pending bar's, for every editor |
| The boards draw a cell that failed, none that is only warned about | Section 7 | "Flagged before save" needs a look that is not the failure's |

Rejected: holding the save until a missing key is fixed (a deferred
constraint, a row someone adds before the save, and a new row of the same
table that the same save writes would all be refused wrongly); no check at
all (the board asks for one); loading a small table whole and matching it
fuzzily in the app (two code paths and two behaviours); a picker that sets
every column of a composite key at once (several pending cells from one
edit is a spec of its own); showing the label in the grid beside the key (a
query per page for every foreign key column, and no board draws it).

Two rules of earlier specs change, and only for a foreign key cell:

- `2026-10-03-value-editing-core-design.md` opens every editor with the
  cursor at the text's end. Section 2 selects the whole text instead.
- `2026-10-07-inserting-rows-design.md` (section 5) has the app check only
  what it can without the database, and a foreign key failure show on the
  save. Section 7 adds one read before the save. It warns and holds
  nothing, so the save's failure is still how a broken key is refused.

## Behavior

### 1. Which cells get the picker

- The column is the only column of a foreign key of its table
  (`ForeignKeyInfo::columns` has one name). This is the rule the header's
  `int8 → publishers`, the row panel's link and `gd` already go by. A
  column that two such keys name goes by the first, as those do.
- The key column of the referenced table is the foreign key's
  `ref_columns[0]`.
- **SQLite's implicit reference.** `REFERENCES publishers` names no column
  and means the parent's primary key. Today `ref_columns` is empty for it.
  With this spec SQLite's `describe` fills it with the parent's primary
  key, where the parent is there and has one. So the picker, the row
  panel's link and `gd` (which today filters by a column with no name) get
  a column, and the Structure view names it. Still empty, or a key of
  several columns: the cell keeps the text editor.
- Everything that decides whether a cell can be edited at all is unchanged:
  the connection, the row's key, the locks. A locked cell says why and
  opens no picker. A binary column is locked, so no picker has a key of
  bytes.
- A column of a composite foreign key, and a cell of a SQL editor's result
  grid, are as today.

### 2. Opening it

- The editor opens the ways it opens on any cell: Enter, F2, a double
  click, typing a character (macOS, Windows, Linux); `i`, Enter and `cc`
  (Omarchy).
- **The picker belongs to the small field**, on the cell or in the row
  panel. An editor that is the large one, or becomes it (a value of several
  lines or too long for the cell, Alt+Enter, a pasted line break: the rules
  as they are), has no list and is the plain editor it is today.
- The field is the search field. macOS, Windows and Linux draw the search
  icon at its start (MainEdit); the Omarchy look draws the `>` prompt
  (OmarchyEditTypes). The list opens under it at once.
- **What the first search is for** goes by how the editor was opened, not
  by `Editor::touched`:
  - Opened on the cell's value (Enter, F2, a double click, `i`, a click in
    the row panel), loaded or pending: the search is for nothing until the
    text is changed. The list shows the referenced table's first rows by
    key, with the row the cell points at now first and marked (a check
    mark at the option's end; `· current` in the Omarchy look, as the enum
    card draws both).
  - Opened by a typed character, or from nothing (`cc`): the search is for
    the field's text from the start.
- Opened on the value by a key or by a double click in the grid, the
  field's whole text is selected on macOS, Windows and Linux, so the first
  character typed starts a search and does not add to an id. A click in
  the row panel puts the cursor where it was clicked, as today, and selects
  nothing. The Omarchy look keeps its own rule: `i` goes on from the end.
- A field the user emptied is a search for nothing too: the table's first
  rows by key. No row is pinned, and none is highlighted (section 4).
- A search is for the field's text without the spaces at its ends.

### 3. The search

- An option is a row of the referenced table: its key and, where the table
  has a label column, its label.
- **The label column** is the first column of the referenced table, in the
  table's order, whose class is text (`column_class` says `Text`) and that
  is not the key column. A table with no such column has no label: options
  show the key alone and the search matches the key alone.
- The referenced table's columns come from the workspace's column cache
  (`Workspace::columns`, which the SQL editors' completion fills). Not
  there yet: they are fetched when the editor opens, and the list shows
  its loading line until they are known. A fetch that failed, now or
  earlier, is the list's failure line (section 4).
- **A row matches** when its label contains the text, compared as the
  **contains** filter compares on that engine (so with that engine's sense
  of case and accents), or when its key, as text, starts with the text.
- **The pinned row** is the one row read by its key, with `=`, and listed
  first:
  - With text, it is the row whose key is the text. It is asked for only
    when the text reads as a value of the key's type. For an integer, a
    decimal, a float and a text key that is the check a typed cell of the
    type gets today. For a `uuid` key it is a rule of this spec, since such
    a cell takes any text today: 36 characters with their hyphens, or 32
    hex digits, the forms `bytes_from_text` reads. An `=` with any other
    text is an error on PostgreSQL and a wrong match on MySQL (`'12abc'` is
    12 there), so it is never sent.
  - With no text, it is the row the cell points at now.
  - A key of a type the app does not know, other than `uuid`, is never
    pinned: it matches by its start alone.
- **A search is two reads**, sent in this order: the pinned row alone,
  where there is one, then the matches. The list shows the first's answer
  without waiting for the second. A row both reads return is listed once.
- **The matches** are the first rows the database finds, 50 of them: it
  stops there, and the app is given one more to know there are more. A
  last line then says "First 50 shown. Keep typing to narrow." No statement
  of the picker sorts a whole table.
- **Order**, within the rows that were read: the pinned row, then the rows
  whose label matched, by key, then the rows that matched by the start of
  their key alone, by key. With no text the rows are the table's first by
  key, which the key's index gives.
- The database says of each row how it matched (its key is the text, its
  label matched, its key starts with the text). The app does not work that
  out again: its own search for the text in a label is only for the bold.
- **When.** The first search is sent when the editor opens. Each later one
  is sent 150 ms after the last change of the text. Sending one cancels
  what the one before it still has running or waiting, and an answer that
  is not the last search's is dropped. Closing the editor cancels what
  runs.
- The search reads and never writes: it runs as a page of rows runs, in a
  read-only transaction on PostgreSQL and MySQL and under `query_only` on
  SQLite.
- A session runs one command at a time. A search waits behind what the
  session is doing, and on a table too large to scan quickly the matches
  can take long when few rows match. The list says it is searching, and
  typing or Esc ends the wait.

### 4. The list

On macOS, Windows and Linux:

- A popup under the field, above it where there is no room below. At least
  as wide as the cell and no narrower than a label and a key need to be
  read. At most eight options show, and the rest scroll.
- A head line names what is listed: `publishers matching "harb" · ↑↓
  choose · ↩ set`. With no text: `publishers · ↑↓ choose · ↩ set`.
- An option is two lines: the label in the body face, the key under it in
  the code face, small and muted (MacEditTypes, MacInsert). With no label
  column it is one line, the key.
- A label is one line: what the grid would show for the value, cut with an
  ellipsis. A NULL label and an empty one are drawn as the grid draws a
  NULL and an empty text.
- The first place the app finds the text in the label, whatever its case,
  is bold, and so is the start of a key that begins with it. A label the
  engine matched and the app does not find the text in (an accent the
  engine ignores) has nothing bold.
- A foot line says `Esc cancels`. Where the column takes NULL it also has
  a **Set NULL** button: it closes the editor without its text and then
  sets NULL as Cmd/Ctrl+Backspace does on the cell or the field. (Set NULL
  does nothing today while an editor is open, so the close comes first.) A
  NOT NULL column has no such button.

In the Omarchy look:

- A float over the pane with an accent border (the Components sheet). Its
  first line is the field: `> harb`, with the count of options at its
  right, `2`, or `50+` when there are more.
- An option is one line: the label, then the key, muted. The match is in
  the accent colour.
- No head line and no foot. The keys are in the status line while the
  picker is open: `ctrl+n/p move · enter set · esc close`.

In every look:

- **A press on the list is the editor's own.** It does not take the
  keyboard from the field and does not leave the edit, the way a press on
  the Now button of a date cell does not.
- **Loading.** Until the first answer the list says "Searching
  publishers…". While a later search runs, the options of the one before it
  stay, so the list does not flicker.
- **Nothing found.** `No publishers match "harb"`.
- **The search failed** (no privilege on the referenced table, the table
  is gone, its columns could not be read): the database's message, on one
  line, in the failure's colour. The field goes on as the plain text editor
  it was before this spec.
- **Which option is highlighted** when an answer comes. With text: the
  first option, when the database says its key is the text or its label
  matched. When the options only begin with the text in their keys, none
  is: a typed `12` is not turned into row 120 by an Enter. With the text
  not yet changed: the current row, and none when the cell is NULL or has
  no such row. With a field the user emptied: none, so Enter takes the
  empty text as it does today.
- The arrows move the highlight from there. A new answer sets it again by
  the rule above.

### 5. Keys, and what an edit comes to

| Key | macOS, Windows, Linux | Omarchy |
|---|---|---|
| ↓ / ↑ | Move the highlight. From none, ↓ goes to the first option | The same, and Ctrl+N / Ctrl+P |
| PgDn / PgUp | Move it by a list's height | The same |
| Enter | Take the highlighted option, or the text when none is, and move as Enter moves today | The same |
| Tab, Shift+Tab | As Enter, and move right or left | The same |
| Esc | Close the editor. The cell is as it was | The same, and Ctrl+C |
| A click on an option | Take it | The same |

- **Taking an option** makes the option's key the cell's pending value,
  exactly as if its text had been typed and kept: the same check against
  the column's type, the same pending cell, the same revert (and the same
  undo, when slice 4 builds it).
- **Taking the text** (Enter with no option highlighted) is today's commit:
  the text is the value if the column takes it, and a text it does not
  take is handled as today.
- **An Enter or a Tab before the answer.** When the list is not yet the
  answer for the text that shows, and the user has not moved the highlight
  or clicked, the key is kept:
  - The search for the text is sent at once, without the 150 ms.
  - The key is applied as soon as it can be decided: when the pinned row
    comes back found, at once (a full id and Enter does not wait for the
    scan); otherwise when the matches come, taking what is then
    highlighted, or the text. A read that fails takes the text.
  - Typing, an arrow or a click drops the kept key. Esc closes the editor.
    A second Enter changes nothing.
  - So `harb`, Enter, typed fast, picks Harbor Press and never a row of a
    list that was for `har`.
- An option the user moved to with the arrows, or clicked, is taken at
  once, from whatever list shows: the user chose that row.
- **Everything else that leaves an editor today** keeps the text as typed,
  as it does for any cell, and never the highlighted option: a click
  outside the field and the list, Cmd/Ctrl+S, Review SQL, another cell, a
  new row.
- **Esc in the Omarchy look** closes the picker without a change. On a
  text cell Esc keeps the edit; here the text is a search and not a value,
  and the Components sheet gives a picker "esc closes".
- **Ctrl+N and Ctrl+P in the Omarchy look** are the picker's while it is
  open, ahead of what those keys do elsewhere, the way the completion list
  of a SQL editor takes them.
- The Home and End keys, the selection and the clipboard are the field's.
- A screen reader hears the pick: "publisher_id set to
  9100000000000000004, Harbor Press".

### 6. What is pending, and what shows

- The pending value is the key. The grid shows it as any pending cell, and
  the row panel as any pending field ("was 9100000000000000001 · revert").
- Review SQL, the save, the conflict question and a failed save are
  unchanged. A save the database refuses for the foreign key is said as
  `2026-10-07-inserting-rows-design.md` (section 9) says it.
- The label is not kept with the cell and shows nowhere but the list.
- **Following the key.** The row panel's "Open publisher →" already
  follows the value the field shows, the pending one where there is one.
  `gd` reads the loaded page today: it follows the pending value too. A
  NULL cell, and one whose pending value is to fix, have nothing to
  follow.

### 7. A key that names no row

- When a foreign key cell gets a pending value that was not taken from the
  list (it was typed and kept), and the value passes the column's own
  check, the app asks the database whether the referenced table holds a row
  with that key: the pinned read of section 3, alone.
- Not asked about: a value taken from the list (it was just read), NULL
  and DEFAULT, a value that is to fix, a text that cannot be pinned (`harb`
  in a `uuid` column, which the column's own check lets through), and a
  key of a type that is never pinned (section 3).
- The question and its answer are kept on the pending cell, beside its
  state: asked, or no such row. Nothing shows until the answer. An answer
  that the row is there, and one that failed, show nothing.
- **No such row** puts a warning on the cell. It is not the "to fix" state:
  the row's mark stays the mark of a changed row, the pending bar's Save
  stays offered, and Review SQL shows the statement as it is.
  - The cell keeps the pending fill and gets the look's caution mark at its
    start (`?` in the row's gutter in the Omarchy look, in the caution
    colour).
  - The words are `No publisher with id 9100000000000000099`, with the
    table's name made singular as the row panel's link makes it. They show
    where a cell's problem shows: under the row panel's field, and in the
    status line while the cell is selected.
  - The pending bar adds `· 1 value names no row` (`2 values name no row`).
- **A table that points at itself.** A warning does not show while a new
  row of the same tab holds that key in the referenced column: one new row
  can name another, and the save writes both. This is looked at when the
  warning is drawn, so it holds whichever of the two was typed first.
- A cell whose last save failed shows the failure and not the warning.
- The warning belongs to the cell and its value. It goes when the value
  changes, is reverted or is saved, and an answer for a value the cell no
  longer has is dropped. Discarding the changes, or loading the page again,
  cancels a question that still runs.
- It is asked once, when the value is kept. The save does not ask again:
  the database decides.

### 8. The row panel and a new row

- The row panel's field of a foreign key column edits with the same field
  and the same list, at the field's width, opened under the field. The
  panel builds no picker of its own.
- "Open publisher →" stays where it is and hides while the field is
  edited.
- While the cell is edited in the grid, the panel's field says "Editing in
  the grid…", as for any cell.
- A new row's cell and the "New row" form get the picker by the same rule.
  A required foreign key column stays required until a value is kept.
- The picker is the editor's. Whatever closes or drops an editor today (a
  save, the page loaded again, a new row dropped, the tab closed) closes
  the list with it and cancels its search.
- Showing another tab leaves an editor open behind it today, and so the
  picker. A search that runs goes on and its answer is kept for the return.
  A kept Enter or Tab is dropped.

### 9. Accessibility

- The field is a text field named `Search publishers for publisher_id`.
- The list is a list box named `Matching publishers`. An option is named
  with its label and its key, `Harbor Press, 9100000000000000004`, and
  says whether it is the highlighted one.
- Each answer is announced: `2 publishers match`, `50 or more publishers
  match`, `No publishers match`.
- The warning of section 7 is in the name of the row panel's field
  (`…, no publisher with this id`) and is announced when it appears. A
  grid's cell has no name of its own: there the words are the status
  line's.
- The keyboard never leaves the field while the list is open. The focus
  ring is the field's, drawn by `src/ui/focus.rs`.

## Out of scope

- The other editors of slice 2.
- A picker for a composite foreign key.
- Showing a label in the grid or the row panel in place of, or beside, the
  key.
- Choosing the label column by hand, or a setting for it.
- Fuzzy matching, and a count of the referenced table's rows.
- Creating a row of the referenced table from the picker.
- Columns that point at another table by convention, with no foreign key
  in the catalog.
- The pending bar's "still being edited" line.
- Pasting over cells and the paste of rows: slice 4 and the inserting rows
  spec. When they are built, a pasted key is a value "not taken from the
  list" and gets the check of section 7.

## Implementation notes

- **`crates/tabletist-db`**: a new `lookup.rs` with what a search asks and
  answers, and `Connection::lookup`.

      pub struct Lookup {
          pub object: ObjectRef,
          pub key: String,
          pub label: Option<String>,
          pub part: LookupPart,
          pub limit: u32,
      }
      pub enum LookupPart {
          /// The one row whose key is this, by `=`.
          Pinned(Value),
          /// Rows whose label contains the text or whose key starts with
          /// it. An empty text: the table's first rows by key.
          Matching(String),
      }
      pub enum Matched { Key, Label, KeyStart }
      pub struct LookupRow {
          pub key: Value,
          pub label: Option<Value>,
          pub matched: Matched,
      }
      pub struct LookupPage { pub rows: Vec<LookupRow>, pub more: bool }

  The app decides whether a text can be pinned (it has the check for a
  typed cell) and makes the value: a whole number for a key that is one,
  since MySQL compares a `BIGINT` with a string as floats. So the crate
  needs no type of the key. The statements are
  `Dialect`'s, beside `select_rows`, and reuse what the filters use:
  `as_text`, `like`, `escape_like`, `bind`. The matches are read in a
  derived table with the limit, and ordered outside it. Every engine is
  named in each match. The crate gets no UI dependency.
- **SQLite's `describe`** fills `ref_columns` for an implicit reference
  (section 1). `ForeignKeyInfo::ref_columns` says when it is still empty.
- **The backend** gets `Command::Lookup { session, request, lookup }` and
  `Event::Lookup { session, request, result }`. It runs on the backend
  runtime as `FetchRows` does, and `Command::Cancel` stops it.
- **State.** `edit::Editor` gets an optional picker: the target table, the
  key and label columns, the text last searched for, the requests that
  run, the options, the highlight, whether the user moved it, and a kept
  Enter or Tab. A new `src/key_picker.rs` holds it and what can be tested
  without a view: choosing the label column, whether a text can be pinned,
  the highlight rule, when the next search is due (from times it is given),
  where the text is found in a label. `edit::Pending` gets the question of
  section 7 as a field beside `state`. It is not a `State`: it holds
  nothing.
- **The reducer.** The edit reducer is `src/app/editing.rs`. The picker's
  part goes in a new `src/app/key_picker.rs` beside it: opening an editor
  builds the picker and sends the first search, `Action::EditorTyped` notes
  when the next is due, `Action::CommitEdit` asks the picker what to take,
  and new actions move the highlight and take an option by its index. The
  app asks egui for a repaint at the due time, and the frame that reaches
  it sends the search: nothing sleeps. The delay is a value the test
  harness sets to nothing. The view pushes actions and changes only the
  field's text, as today.
- **The view.** A new `src/ui/key_picker.rs` draws the list for both
  looks (`src/ui/picker.rs` is the saved connections' picker).
  `cell_editor::field` and `cell_editor::in_panel` call it, so the grid and
  the row panel share it. Text goes through `TextRole`s, and strings
  through `gettext`. The keys are taken in `src/ui/keys.rs`, where the
  completion list's are.
- `app.rs` and `ui/mod.rs` are very large. The picker's code and its tests
  go in the new modules, not in those files.
- A search's text and its rows are not logged: a label can be a name or an
  email.

## Tests (headless, no pixels)

Only the Bookshop's data: `publishers` (Northlight Books …001, Harbor
Press …004, Harbor Lights …007) and `book_covers.publisher_id →
publishers.id`, the rows `crates/tabletist-db/tests/insert_audit.rs`
already has (as `audit_publishers` and `audit_book_covers` on PostgreSQL
and MySQL). Its SQLite reference names its column, so test 9 adds a table
declared without one. The app's tests get a fixture of their own with that
key and a `publishers` structure. The shared `book_covers_structure()` of
`src/testing.rs` stays without the key, so the tests that type into
`publisher_id` today are not changed. Test 21 needs one table that points
at itself (`genres.parent_id → genres.id`). No pixel or snapshot tests.

`tabletist-db`, unit, on the statements:

1. Each dialect's matching read casts the key to text for the prefix,
   escapes `%` and `_` in the text, compares the label as the contains
   filter does, and says how each row matched.
2. It reads `limit + 1` rows inside the derived table and orders outside
   it. With an empty text it reads the table's first rows by key.
3. A table with no label column is searched by its key alone.
4. The pinned read is one `=` on the key, with the value bound or quoted
   as a filter's is.

`tabletist-db`, integration, on SQLite, PostgreSQL and MySQL:

5. `harb` finds Harbor Press and then Harbor Lights, with their keys.
6. A pinned key a row has answers with that row, and one no row has with
   none.
7. A text that begins keys and is in a label lists the label's rows before
   the others.
8. A lookup runs on a read-only session, and writes nothing on a writable
   one.
9. SQLite: a foreign key declared `REFERENCES publishers` has the parent's
   primary key as its referenced column, and the Structure view names it.

The app, through `src/testing.rs`:

10. Enter on a `publisher_id` cell opens a field named "Search publishers
    for publisher_id", with its text selected, and a list named "Matching
    publishers" with the current row first and marked.
11. Typing `harb` lists two options, the first highlighted. Enter makes the
    cell pending with Harbor Press's id and closes the list. Tab does the
    same and moves right.
12. ↓ and Enter take the second option. A click on an option takes it and
    does not first keep the text.
13. Esc closes the list and leaves the cell as it was, in both looks.
    Ctrl+N and Ctrl+P move the highlight in the Omarchy look and do
    nothing else.
14. A typed text that only begins keys highlights nothing, and Enter keeps
    the text.
15. An Enter sent before the answer takes the first option of that answer,
    not one of the list before it. Typing after the Enter drops it. A full
    key and Enter is applied by the pinned row's answer alone.
16. An answer for an earlier text is dropped, and sending a search cancels
    the one before it.
17. A column of a composite foreign key opens the text editor and no list.
    So does a value that opens the large editor.
18. A NOT NULL column's list has no Set NULL. A nullable one's sets NULL.
19. A search that fails shows the database's message, and the text can
    still be kept. More than 50 matches show the last line.
20. A click outside keeps the text as typed, not the highlighted option.
21. A typed key no row has leaves the cell pending and warned, with Save
    offered and the pending bar saying one value names no row. Changing
    the value drops the warning. A value taken from the list is not asked
    about. A key that a new row of the same tab holds is not warned about.
22. The row panel's field opens the same list, and a pick there marks the
    grid's cell pending.
23. A new row's foreign key cell opens the list, and a pick fills what was
    required. `INS-09b` of `src/ui/insert_audit_tests.rs` moves to the
    fixture with the key and is no longer ignored.
24. `gd` follows the pending value.

## Website

- `docs/_guide/editing-data.md`: a part on foreign key cells under "Change
  a value" (the search, the keys, the warning), and "names no row" in what
  is checked before a save.
- `docs/_guide/omarchy.md`, "Editing": Ctrl+N and Ctrl+P in the picker,
  and Esc there.
- `docs/_guide/macos.md` and `docs/_reference/keyboard-shortcuts.md` only
  if `SHORTCUTS` gains an entry, which this spec does not ask for.
- A screenshot of the open picker for each look, as a scene of
  `src/shots.rs`. Its `book_covers` has no `publisher_id` today: the
  scene gets the column, the key to `publishers` and the rows.

## Delivery

Four runs, each with its plan, its pull request and the pages it changes:

1. **The lookup.** `Connection::lookup` on the three engines, SQLite's
   implicit reference, the backend's command and event. Tests 1 to 9.
2. **The picker in the grid**, both looks, on existing and new rows:
   sections 1 to 6 and 9. Tests 10 to 20, 23 and 24.
3. **The row panel**: section 8. Test 22.
4. **A key that names no row**: section 7. Test 21.

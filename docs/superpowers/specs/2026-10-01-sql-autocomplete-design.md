# SQL editor, slice 2: autocomplete

Date: 2026-10-01. Status: step 1 (the list and keywords) is implemented;
steps 2 and 3 (schemas, tables and views; columns) are designed here and not
yet built. "Step 1 as built" below says where the code differs from this
text; where the two disagree, that section and the code win.

## Step 1 as built

- **Engine.** `FROM` and `UPDATE` are table words only where they start a
  table list: a `FROM` counts once the statement has had `SELECT`, `DELETE`,
  `UPDATE` or `SHOW`, not after `IS [NOT] DISTINCT`, and not as the first
  `FROM` inside `EXTRACT(`, `TRIM(`, `SUBSTRING(` or `OVERLAY(`; an `UPDATE`
  does not count after `FOR`, `KEY` or `DO`. A function in a `FROM` list is
  skipped with its arguments and alias. A subquery in `FROM` is scanned, not
  skipped, and a table after it in a comma list is not found. A qualifier
  counts only when its dots touch the names.
- **Matching.** Keywords and phrases match from their start only. A row
  "is typed" when accepting it would only repeat the word: a keyword
  whatever its case, any other name as spelled. That is what the Enter rule
  and the opening rule compare.
- **Opening.** A list asked for by hand opens even when its one row repeats
  the word. A list opened on an empty word stays open when a letter is typed
  and deleted. `Mod+I` asks for the list as `Ctrl+Space` does, since some
  systems take `Ctrl+Space` first.
- **Model.** The view's action is `SqlTyped`, sent only for typing.
  `Completion` also keeps the typed part of the word, how it opened and a
  serial; its site is behind an `Arc`. A list whose row was accepted is not
  worked out again before the view inserts it.
- **Keys.** They act only while this editor has the keyboard. Every press of
  a frame moves the highlight. In a frame that also types, Tab does nothing
  and Enter is the editor's line break. `Ctrl+N` and `Ctrl+P` are the list's
  in the terminal look also while it has no rows.
- **Insertion.** It is an undo step of its own in both directions, and the
  caret is scrolled into view after it.
- **The list.** The bundled faces have no Tab glyph: the macOS footer reads
  `↩ Tab insert`. The matched part of a name is drawn at weight 500 on macOS
  (Plex Mono is bundled at 400 and 500) and is not emphasised in the
  standard look, whose monospace has one weight: a known gap. Any click on
  the panel keeps the list and the editor's keyboard. A list waits, undrawn,
  while the editor scrolls to its caret. Long names are cut with an
  ellipsis.
- **Accessibility.** The list is a list box and its rows are options. The
  editor names the highlighted row as its active descendant only for a list
  opened by hand or once the highlight was moved, so a screen reader stays
  on the editor while a word is typed. Not tried with a screen reader.
- **Platforms.** Built and tested headlessly on Linux only.

## Intent

The SQL editor (slice 1, `2026-09-30-sql-editor-core-design.md`) highlights
and runs SQL but offers no help while typing. This slice adds a completion
list at the cursor: SQL keywords, the connection's schemas, tables and views,
and the columns of the tables a statement names.

Success: on each driver, a user typing a query sees matching keywords, then
tables after `FROM`, then columns after `alias.`, accepts one with Tab or
Enter, and can take the insertion back with one undo. The list never blocks
typing, never makes the UI thread wait for the database, and never turns an
Enter meant as a line break into an unwanted insertion in the cases listed
under "When the list opens".

The designs are the "SQL editor" artboards (macOS and Omarchy) in the design
canvas Artifact. They are not copied into the repository.

## Decisions

| Question | Decision |
|---|---|
| Scope | Keywords, tables and views, and columns in one spec and one plan, built in three steps that each work alone. |
| Trigger | The list opens by itself while a word is typed and after a dot, and `Ctrl+Space` opens it anywhere. |
| Accept | Tab and Enter both insert, in every look. Enter stays a line break when the highlighted item is exactly what is typed. |
| Moving | Arrow keys in every look. In the terminal look also `Ctrl+N` and `Ctrl+P`, only while the list is open. |
| Context | A scan over the tokens of the statement at the cursor, in `tabletist-db`. No SQL parser. |
| Columns | Through the existing `Command::Describe`, per table, cached on the workspace. No new driver query. |
| List state | On `SqlTab`, changed through actions. The view draws it and carries out the insertion. |
| Matching | Names that start with the typed text, then names that contain it. Not fuzzy. |

Rejected: a SQL parser crate (a new dependency for three dialects, and a
half-typed statement rarely parses); fuzzy matching (the designs emphasise
the typed part of a name, which scattered matches cannot show); one bulk
query for a schema's columns (a schema of 10,000 tables can hold hundreds of
thousands of columns); a new one-query `Columns` command in the three
drivers (worth it only if `Describe` proves slow, and then it can replace
the fetch without changing anything else here); keeping the list's state in
egui's memory (against "views push actions", and hard to test); following
the macOS artboard's footer, where Tab moves to the next row (Tab would mean
different things in different looks).

## Out of scope

Built-in functions and their signatures (the artboards colour `count` and
`max`; neither that colour nor their completion is in this slice), snippets,
aliases as completions, columns of CTEs and subqueries, PostgreSQL's
`search_path` (names are inserted bare only from the schema a dialect
searches by default, see "Candidates"), completion inside a quoted
identifier, the symbol operators (`=`, `<>`),
and the mode line's explain and history hints (slices 3 and 4).

## The three steps

1. **The list and keywords.** The engine, the list's state and keys, the
   popup, insertion with undo, accessibility. Offers keywords only.
2. **Schemas, tables and views.** Quoting, the bare schema, loading a
   schema's objects on demand.
3. **Columns.** Sources and aliases, the column cache, its fetch and its
   invalidation.

## The engine (`tabletist-db`)

A new module `complete` (`crates/tabletist-db/src/complete.rs`), with no UI
and no catalog knowledge. It reads a script's tokens, as `sql::tokenize`
gives them, and finds the statement around the cursor itself.

```rust
/// Where a completion would go, and what belongs there.
pub struct Site {
    /// The word at the cursor: the bytes a completion replaces. Empty at
    /// a fresh position (after a dot or a space).
    pub word: Range<usize>,
    /// The names before the word, joined to it by dots, quotes removed:
    /// `["public"]` in `public.bo`, `["b"]` in `b.ti`.
    pub qualifier: Vec<String>,
    pub expects: Expects,
    /// The tables the statement names, in order.
    pub sources: Vec<Source>,
    /// The names of the statement's common table expressions.
    pub ctes: Vec<String>,
}

pub enum Expects {
    /// The first word of a statement: keywords only.
    Start,
    /// After FROM, JOIN, UPDATE, INTO or TABLE, or a comma in a FROM
    /// list: schemas, tables, views and CTE names.
    Tables,
    /// After a table name in FROM or JOIN, or after AS: most likely an
    /// alias or a new name.
    Name,
    /// Anywhere else: columns of the sources, then keywords.
    Columns,
}

pub struct Source {
    pub schema: Option<String>,
    pub name: String,
    pub alias: Option<String>,
}

pub fn site(tokens: &[Token], text: &str, cursor: usize) -> Option<Site>;
```

- The statement `site` scans is the run of tokens between the last `;`
  token before the cursor and the first `;` token at or after it (a `;`
  inside a string or a comment is not a token of its own). So the cursor
  after trailing spaces (`SELECT * FROM |`) is still in its statement, and
  the cursor after a `;`, or in an empty script, is at the start of a new
  one: `Expects::Start`, an empty word, no sources. `sql::Statement` is not
  used: its range leaves out the surrounding whitespace.
- `site` is `None` when the cursor is inside a string, a comment, an
  executable comment, a number or a quoted identifier.
- The word is the identifier or keyword token that the cursor is inside of
  or at the end of. What is matched is the part of it before the cursor; an
  accepted completion replaces the whole word. With the cursor at the start
  of a token (`FROM |books`) the word is empty and nothing is replaced.
- With a qualifier, `expects` does not matter: the app resolves the
  qualifier (see "Candidates").
- Sources come from a scan of the whole statement, at every depth of
  parentheses: after `FROM`, `JOIN`, `UPDATE` and `INTO`, and after each
  comma of a `FROM` list, a `name` or `schema.name`, an optional `AS`, and
  an optional alias (a name that is not a keyword). A parenthesis in that
  place is a subquery: it is skipped, and it is not a source. Quotes are
  removed from names.
- A CTE name is the name before `AS (` after `WITH` or after a comma at the
  `WITH` list's depth.
- The scan is approximate by design: a column of an inner query can be
  offered in the outer one. It never fails.

Also in `tabletist-db`:

- `sql::keywords(dialect)`: the highlighting keywords of a dialect, now
  public, as the words to complete. A keyword worth completing is worth
  colouring, so there is one list; this slice may add words to it.
- `complete::PHRASES`: a short list of keyword phrases offered as one item:
  `GROUP BY`, `ORDER BY`, `PARTITION BY`, `LEFT JOIN`, `RIGHT JOIN`,
  `INNER JOIN`, `CROSS JOIN`, `FULL JOIN`, `IS NULL`, `IS NOT NULL`,
  `NOT IN`, `NOT LIKE`, `NOT EXISTS`, `UNION ALL`.
- `Dialect::ident(name) -> Cow<str>`: the name bare when that is safe, else
  `quote_ident(name)`. Safe means a plain word that is not reserved. A
  plain word is `[a-z_][a-z0-9_]*` on PostgreSQL (it folds unquoted names
  to lower case) and `[A-Za-z_][A-Za-z0-9_]*` on MySQL and SQLite. Reserved
  means in the dialect's highlighting keywords or in a new per-dialect
  reserved list (`crates/tabletist-db/src/reserved.rs`: PostgreSQL's
  reserved key words, MySQL 8.4's reserved words, SQLite's keywords). The
  rule errs toward quoting: a quoted name always works.

## Candidates (app, no UI)

A new module `src/completion.rs` turns a `Site` and what the workspace knows
into the rows of the list. It is a pure function over the model, with tests
of its own.

```rust
pub struct Candidate {
    pub kind: Kind,          // Keyword, Schema, Table, View, MaterializedView, Column
    /// What the row shows.
    pub label: String,
    /// What accepting inserts in place of the word.
    pub insert: String,
    /// The part of `label` that matched what was typed.
    pub matched: Range<usize>,
    /// A column's type. Empty for every other kind.
    pub detail: String,
}
```

What is offered:

- `Start`: keywords and phrases.
- `Tables`: tables, views and materialized views of the bare schema,
  inserted bare; those of every other schema whose objects are loaded,
  inserted as `schema.name`; the visible schemas; the statement's CTE names
  (as tables). No keywords.
- `Name`: keywords and phrases, and only when the list was opened by hand.
  So `WHERE` or `JOIN` typed right after a table name is not completed by
  itself; that is the price of leaving aliases alone.
- `Columns`: the columns of the statement's sources whose columns are known,
  then keywords and phrases.
- A one-part qualifier is resolved in this order: a source's alias, a
  source's table name (both give that table's columns), a visible schema
  (its tables and views, inserted bare, since the schema is already typed).
  A two-part qualifier `schema.table` gives that table's columns, whether
  or not the statement names the table elsewhere. A qualifier that resolves
  to nothing gives no list.

Every name is inserted through `Dialect::ident`. A keyword follows the case
of what was typed: `sel` gives `select`, `SEL` and an empty word give
`SELECT`.

Matching ignores case. Order: a candidate whose insertion is exactly the
typed word first; then, within the kinds in the order listed above for the
site, names that start with the typed text before names that contain it;
then by name. The list keeps the best 100 and counts the rest.

The bare schema is the one a dialect searches for an unqualified name:
`public` on PostgreSQL, the connection's database on MySQL, `main` on
SQLite. When the tree has no such schema (a MySQL connection without a
database, a PostgreSQL database without `public`), nothing is inserted bare.
It is usually the schema the sidebar opens at connect, but that rule (in the
`Event::Schemas` handler) prefers a schema named like the database and is
left as it is.

A source is matched to a loaded object by name, ignoring case, an exact
match first: `schema.name` in that schema, a bare name in the bare schema
and then in the other loaded schemas. A source that matches nothing loaded
has no columns.

## Where names come from

- **Schemas, tables and views** come from `Workspace.tree`, as the sidebar
  and quick open read it. A list opening at any site loads the bare
  schema's objects if they never were (the sidebar usually has; sources
  are matched against them), and one opening on `schema.` loads that
  schema's (`load_objects`, without marking the sidebar's node as
  expanded).
- **Columns** live in a new `Workspace.columns:
  HashMap<ObjectRef, Fetch<Vec<ColumnInfo>>>`. When a list opens or is
  recomputed at a site that offers columns, each source matched to a loaded
  object and never asked for gets one `Command::Describe`, at most eight
  per statement; so does the table a two-part qualifier names.
  `Event::Structure` goes to the table tab waiting for that request as
  today, else to the cache entry waiting for it. A table tab's structure,
  when it arrives, fills the cache too. A fetch that failed (no privilege,
  the table is gone) stays failed until the cache is cleared, so it is not
  asked again on every keystroke.
- **Waiting.** Asking is part of opening the list, whatever it holds so far.
  A list waiting for an answer it asked for stays open, also with no rows
  (see "Model" and "The list"); when the answer leaves it empty, it closes.
- **Freshness.** Every run is rolled back, so the editor cannot change the
  catalog itself; names go stale only through changes made elsewhere.
  Refreshing the tree, reconnecting and switching the database clear
  `Workspace.columns` and drop the objects of schemas the sidebar does not
  show expanded (they load again on demand). There is no timer.
- **One session, one command at a time.** A column fetch waits behind a
  running query, and a run waits behind fetches already queued. Fetches are
  short catalog queries and are not cancelled for a run. While columns are
  on their way the list shows what it has and its footer says so.

## Model

```rust
pub struct SqlTab {
    // ...
    /// The completion list, while it is open.
    pub completion: Option<Completion>,
    /// A list was asked for (by typing, or by hand) and is not worked out
    /// yet: `refresh_completion` takes this and decides.
    pub completion_wanted: Option<Wanted>,
}

pub enum Wanted { Typed, Manual }

pub struct Completion {
    /// Opened with Ctrl+Space, not by typing.
    pub manual: bool,
    /// What the list was worked out from: the text, the cursor and the
    /// catalog's generation.
    of: (TextPrint, usize, u64),
    pub site: Site,
    pub candidates: Arc<Vec<Candidate>>,
    /// Matches beyond the ones kept.
    pub more: usize,
    /// The highlighted row.
    pub selected: usize,
    /// The user moved the highlight since the typed word last changed.
    pub moved: bool,
    /// Objects or columns this site needs are being fetched.
    pub loading: bool,
    /// Insert the highlighted row on the next draw.
    pub accept: bool,
}
```

`Workspace` gains `columns` and a `catalog_generation: u64`, bumped whenever
the tree's objects or the column cache change.

New actions, each with `tab` and `sql_tab`: `SqlEdited { typed: bool }` (the
view saying the field's text changed, and whether by typing),
`OpenCompletion`, `MoveCompletion(isize)`, `AcceptCompletion(Option<usize>)`
(a row clicked, or the highlighted one) and `CloseCompletion`.

`App::apply` has no egui context and so no tokens: `SqlEdited { typed: true }`
and `OpenCompletion` only set `completion_wanted`, and the other actions only
change an open list. `App::refresh_completion(ctx)` is called from `frame_ui`
after each `apply_actions`. For the active SQL tab it takes
`completion_wanted` and applies the rules of "When the list opens", and it
recomputes an open list only when `of` no longer matches; a frame that
changes nothing, or that types with no list wanted, computes nothing. It
drops the lists of SQL tabs that are not the active one.

- **Tokens.** The editor's `Parsed` (in `src/ui/sql_text.rs`) keeps the
  tokens it now drops, and `refresh_completion` reads them through
  `sql_text::parsed`, the cache the layouter fills. A script is still
  tokenized once per change, by whichever asks first.
- **Fetching.** It sends the fetches the site needs (see "Where names come
  from") and sets `loading` while any of them is pending.
- **Closing.** It closes the list when the site is gone, when the word it
  was opened on is no longer the one at the cursor, when its tab is no
  longer the active one, and when no candidate is left and nothing is
  loading.
- **The highlight** is the first row. Once the user moved it, it stays on
  its candidate while that one is still listed, until the typed word
  changes; then it is the first row again. So an exact match, which sorts
  first, is always the highlighted row right after it is typed.

`RunSql` closes the tab's list in its handler. The view pushes
`CloseCompletion` when the editor does not have the keyboard, and when the
editor was scrolled so that the word left its pane. It checks the keyboard
after the list is drawn: a press on a row gives the editor the focus back
first, as a press on the gutter does, so the click still lands.

## When the list opens

- **By typing** (`SqlEdited { typed: true }`): when the word before the
  cursor has two characters or more, or when the character typed is a dot
  after a name. Not at a `Name` site. Not when the only candidate is
  exactly the typed word: the names the site needs are still asked for,
  but the list stays closed, also when they arrive. Text an input method
  commits is typing. Pasting, undo, deleting and an input method's text
  while it is being composed are edits that are not typing
  (`typed: false`): they never open the list, and an open one is
  recomputed for the new text. Deleting keeps an open list open while its
  word remains.
- **By hand**: `Ctrl+Space` (plain Ctrl on every platform), at any site,
  also on an empty word. With nothing to offer and nothing to fetch it
  does nothing.
- These rules exist so that Enter keeps meaning a line break where a user
  is most likely to press it: after a one-letter alias, after `AS name`,
  and at the end of a line whose last word is complete. One case is left:
  an alias of two letters or more is caught by the `Name` rule only when it
  follows a table name directly.

## Keys and focus

All in `src/ui/keys.rs`, before the editor sees the events and before the
global shortcuts, while the active SQL tab's list is open:

| Key | Does |
|---|---|
| Down, Up | `MoveCompletion(1)`, `MoveCompletion(-1)`, stopping at the ends |
| `Ctrl+N`, `Ctrl+P` | The same, in the terminal look only (plain Ctrl). With the list closed they mean what they mean today: New connection and Quick open where Mod is Ctrl. |
| Tab | `AcceptCompletion(None)` |
| Enter | `AcceptCompletion(None)`, unless the highlighted candidate's insertion is exactly the typed word: then the list closes and the editor gets its line break |
| Esc | `CloseCompletion`; the editor keeps the keyboard |
| `Mod+Return` | Runs as today; the list closes |

Tab and Enter are taken only without modifiers (matched exactly: egui's
`consume_key` ignores an extra Shift), and only while the list has a row:
`Shift+Tab` still outdents, `Shift+Enter` is still a line break, and
in a list that is empty while it loads both keys go to the editor.

`Ctrl+Space` is taken whenever a SQL editor has the keyboard.

Every other key reaches the editor: typing narrows the list, Left and Right
move the cursor (and close the list when it leaves the word).

Esc needs more than consuming the event: egui gives up a widget's focus on
Escape at the start of the pass, before any of our code runs, unless the
focused widget's lock filter holds Escape. The editor's filter holds the
arrows and (through `lock_focus(true)`) Tab, not Escape. While the list is
open the view sets the editor's focus lock filter again after the field is
drawn (`Memory::set_focus_lock_filter`, which replaces the whole filter):
the arrows and Tab as the field set them, and `escape: true`. So the first
Esc closes the list and the second leaves the editor as today. The first
commit proves this in a headless test (Esc held, arrows and Tab still the
editor's) before anything is built on it; should it not hold, the fallback
is to give the editor the focus back on the frame after Esc.

A click on a row accepts it and gives the editor the keyboard back, as a
click on the gutter does. A click anywhere else, losing focus and switching
tabs close the list.

## Insertion and undo

`AcceptCompletion` marks the list (`accept`); the editor's view carries it
out at the start of its next draw, before the field handles that frame's
events, so keys typed right after Tab land after the inserted text. The view
is where a field's text, its cursor and its undo history may be touched.

The view takes the list off the tab (as it takes `focus_editor`), replaces
`site.word` in `SqlTab.text` with the candidate's insertion, and puts the
cursor after it (a character index for egui, a byte offset for the tab).
Before changing the text it adds
an undo point holding the text and cursor as they were
(`TextEditState::undoer`, `add_undo`, `set_undoer`), because egui groups
undo by time and would otherwise merge the insertion with the typing around
it. One `Mod+Z` after accepting restores exactly the typed word.

Accepting a candidate whose insertion equals the word changes nothing and
adds no undo point.

## The list (`src/ui/sql_complete.rs`)

A foreground area anchored under the start of the word (from the frame's
galley and its screen origin), moved above the line when there is no room
below, and kept inside the window. It never takes the keyboard.

- **macOS and the standard look:** 330 pt wide, the raised fill, a hairline,
  radius 8 and the soft shadow. Rows are 26 pt: a kind letter (`K`, `S`,
  `T`, `V`, `C`), the name in the code face with the matched part semibold,
  and at the right, muted, the kind in words ("keyword", "schema", "table",
  "view", "materialized view") or a column's type. The highlighted row has
  the selection fill, radius 5.
- **Omarchy:** 360 pt wide, the dark fill, an accent border, radius 3. Rows
  are 24 pt, with no kind letter: the name with the matched part in the
  accent, and the kind or type at the right. The highlighted row has the
  selection fill across the list.
- Five rows show; the list follows the highlight. A footer under a hairline
  names the keys and counts the matches out of sight: "↩ ⇥ insert" and
  "12 more" on macOS, `tab complete · ctrl+n/p move · 12 more` on Omarchy.
  While names are being fetched the count's place reads "Loading". A list
  with no rows yet is the footer alone.
- The Omarchy mode line shows `tab complete` while the list is open.
- The help dialog's shortcut table gains `Ctrl+Space`, "Complete".
- Text is drawn through new `TextRole`s; the view names no font or size.
  macOS: the kind letter in the UI face at 10.5 pt, muted; the name in the
  code face at 12.5 pt, its matched part at weight 600; the kind or type
  and the footer in the UI face at 11 pt, muted. Omarchy: the name in the
  editor's code role; the kind or type and the footer in the monospace
  face at 11.5 pt, muted, with the footer's keys in the text colour.
- Every string goes through `gettext`.

Accessibility: the list is a labelled group "Completions". Each row is a
selectable item named by its label and kind ("books, table"; "title,
column, text"), with its selected state. The editor's AccessKit node names
the highlighted row as its active descendant, so a screen reader that
follows it announces the row while the keyboard stays in the field.

## Errors and edge cases

- A failed column fetch offers no columns for that table and shows no error
  in the editor; the table's own tab shows the error if opened.
- A lost connection closes nothing: the list goes on offering what is
  loaded. Fetches are sent only on a connected session.
- A schema of 10,000 objects: candidates are worked out once per change of
  the word, the site or the catalog, never per frame, and at most 100 are
  kept. `MAX_LISTED` already caps what a schema loads; names past it cannot
  be offered.
- Names that need quotes (`Order Items`, `user` on PostgreSQL, mixed case on
  PostgreSQL) are shown bare in the list and inserted quoted.
- A word typed inside quotes gets no list.
- Two tables of the same name in two schemas: the bare schema's is the
  bare one, the other is offered and inserted qualified.
- The text changed under an open list by something other than typing (undo,
  paste, an input method composing): the list is recomputed for the new
  text or closes.
- A SQL tab on a workspace that is not connected offers keywords and
  whatever was loaded before.

## Testing

- `tabletist-db` unit tests for `complete::site` in all three dialects: the
  word and its range at the end of a word, inside one and on an empty one;
  the cursor after trailing spaces, after a `;` and in an empty script;
  `None` in strings, comments, numbers and quoted identifiers; each
  `Expects`, including a comma in a `FROM` list and the word after a table
  name and after `AS`; qualifiers of one and two parts, quoted and not;
  sources with and without schema, `AS` and alias, in joins, in a subquery,
  with a subquery in `FROM` skipped; CTE names; an unfinished statement.
- `Dialect::ident` unit tests: plain names bare; upper and mixed case on
  PostgreSQL quoted and on MySQL bare; spaces, a leading digit, an embedded
  quote; reserved words of each dialect (`user`, `order`, `group`, `key`).
- `src/completion.rs` unit tests: what each site offers; the order (exact
  first, kinds, starts-with before contains, by name); keyword case;
  qualified insertion outside the bare schema, and nothing bare when the
  tree has no bare schema; qualifier resolution in
  its order; the cap and the count of the rest.
- Headless UI tests through `src/testing.rs`, on the Bookshop names:
  typing two letters opens the list and one does not; a dot opens it;
  `Ctrl+Space` opens it on an empty word; Down and Up move and stop at the
  ends; Tab and Enter insert; Enter on an exact match is a line break, also
  when a longer name was highlighted a letter earlier (`desc` with a
  `description` column); `Shift+Tab` and `Shift+Enter` go to the editor;
  no list after a table name or after `AS` unless opened by hand; the first
  Esc closes the list with the editor still focused and the second leaves
  the editor; arrows still move the editor's cursor, not the focus, while
  the list holds Esc; `Mod+Z` after an insertion restores the typed word; a
  click on a row inserts and the editor keeps the keyboard; `Mod+Return`
  runs and closes the list; `Ctrl+N` and `Ctrl+P` move the highlight in
  the terminal look with the list open, and with it closed, or in another
  look, do what they did before; typing `schema.` sends one `ListObjects`,
  the list waits open with no rows, Enter is a line break meanwhile, and
  the answer fills it; a column site sends one `Describe` per table, never
  twice, and a late answer fills the open list; an answer that leaves the
  list empty closes it; refreshing the tree clears the column cache; a
  script is tokenized once per change with the list open; the rows and the editor's active descendant
  in the AccessKit tree; a 10,000-object schema is not recomputed on a
  frame that changes nothing (a counter, as `TOKENIZED` does for the
  tokenizer).
- No driver tests: no driver changes.
- No design or pixel conformance checks. Screenshots for review use the
  Bookshop demo data and stay local.

## Commits

One topic each, each passing the checks:

1. The editor keeps the keyboard on an Esc it is told to hold (the focus
   lock filter, with its test).
2. `complete::site`: the word, what is expected, sources and CTE names.
3. The completion list's model, actions and candidates, offering keywords.
4. The list's view, its keys, insertion with undo, accessibility, the mode
   line and help entries. (Step 1 is complete here.)
5. `Dialect::ident` and the reserved words.
6. Schemas, tables and views in the list, the bare schema, loading a
   schema on demand, the waiting list. (Step 2.)
7. Columns: the cache, the fetch through `Describe`, invalidation.
   (Step 3.)

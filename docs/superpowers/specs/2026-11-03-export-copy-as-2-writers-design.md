# Export and Copy as, run 2: the writers and Copy as

Run 1 (PR #248) gave both grids a selection: a rectangle of cells or
whole rows, which ⌘C and `y` copy as TSV through `format::tsv_cells` and
`copied()` in `src/app.rs`. This run writes the six formats the export
needs, remembers each format's options, moves ⌘C and `y` onto the TSV
writer, and builds Copy as: the grid's menu, ⌥⌘C, `gy`, and the
inspector's Copy row ▾. Runs 3 and 4 export a table and a SQL result
through the same writers.

Design canvas: **Tabletist**, section **Export and Copy as**, boards
`macOS – Copy as` and `Omarchy – Copy as`; `macOS – Export, entry points`
and `Omarchy – Export, entry points` for the inspector and the key lines;
`macOS – Export, sheet by format` and `Omarchy – Export, sheet by format`
for each format's options.

## Delivery

Four runs, a pull request each (Igor, 2026-10-30):

1. The selection (merged, PR #248).
2. **This spec:** the six writers with golden tests, options remembered per
   format, ⌘C and `y` on the TSV writer, the Copy as menu (⌥⌘C, `gy`), the
   inspector's Copy row ▾, the clipboard limit.
3. Export a table: the sheet and the float, the background export, its
   states, the table's entry points.
4. Export a SQL result: Run again without the limit, the results bar, the
   inspector's Export row…, several exports at once, the portal picker.

## Where this build differs from the boards or the prompt

Igor approved every row as written (2026-11-03):

| # | The board or the prompt | This build | Why |
|---|---|---|---|
| D1 | The grid's context menu: Copy, Copy as ›, Copy rows as TSV, Set to… ⇧⌘⌫, Export selection… ⇧⌘E | Copy, Copy as ›, Copy rows as TSV. No Set to… and no Export selection… | Set to has not landed on `main`; Export selection… is run 3's. Each joins the menu in its own run |
| D2 | The inspector's Copy row ▾ ends with Export row… ⇧⌘E | The five Copy row as rows, no Export row… | Run 4 builds Export row… |
| D3 | Over 10,000 rows: "That's 31,400 rows. Export to a file?" with **Export…** | The same words in the toast (the status line in Omarchy), with no Export… | Nothing to export with until run 3, which adds the action |
| D4 | The board's inspector header holds only Copy row ▾ | Copy row ▾ stands left of the panel's existing buttons (Previous row, Next row, Close) | The entry board says existing screens keep their layout |
| D5 | Copy as uses each format's remembered options | Except SQL INSERT's Start with CREATE TABLE: Copy as never starts with one | A selection is a piece of a table, and the tab may not have read its DDL. The export (run 3) honours it |
| D6 | Golden files hold a value longer than 256 KiB | The golden files hold short values; one test per format writes a 300 KiB value and checks it comes out whole | Twelve copies of 300 KiB in the repository say nothing more than one check of its length |
| D7 | (Copy as in the terminal look's inspector) | `gy` in the inspector opens the same float, titled "copy row as", on the inspector's row | The Omarchy entry board lists `gy copy row as` there |
| D8 | ⌘C "stays TSV" | ⌘C and `y` write TSV's remembered options but never a header row. One cell alone is copied as today: its text, tabs and lines kept | A header row over a pasted range of cells is a surprise. Copy as › TSV writes the header when TSV's option says so |
| D9 | (⌘C over the limit) | ⌘C and `y` keep the clipboard limit too: over 10,000 rows or 20 MB they copy nothing and say so as Copy as does | ⌘A then ⌘C on a large page would otherwise put a page of hundreds of MB on the clipboard |

## Decided while specifying

Not drawn or written anywhere. Igor approved every decision as written
(2026-11-03):

| # | Question | Decision | The other way |
|---|---|---|---|
| Q1 | Each format's defaults (the boards each show a different case) | CSV: header on, `,`, When needed, NULL as Empty, no BOM, LF, Hex. TSV: header on, NULL as Empty, no BOM, LF, Hex. JSON: Nested, Indented, Hex. JSON Lines: Nested, Hex. SQL INSERT: 100 rows per INSERT, no CREATE TABLE. Markdown: NULL as Text `NULL`, In full. A Text NULL starts as `\N` in CSV and TSV and `NULL` in Markdown | The CSV board's NULL as Text `\N` as CSV's default |
| Q2 | What Column names copies | The selection's column names on one line, joined by `, `, each as the connection's dialect writes a name (quoted only when it must be), ready for a `SELECT` list. "Copied 2 column names" | Tab-separated, as a TSV header line |
| Q3 | TSV's escapes | `\t`, `\n`, `\r` and `\` itself (`\\`), so `\N` and an escape read back as they were written (PostgreSQL `COPY`'s text format) | Tabs and line breaks only, so a Windows path pastes unchanged |
| Q4 | A range's NULL under ⌘C (today `NULL`) | As TSV's NULL as says: empty by default | Keep `NULL` for ⌘C alone |
| Q5 | Binary values in a range (today a 16-byte value copies as the UUID the grid shows) | Hex `0x…` or Base64, as the option says, for every format but SQL INSERT (the dialect's literal). One cell alone is copied as today | A 16-byte value stays a UUID in every format |
| Q6 | The last line on the clipboard | No line ending after the last line, as ⌘C copies today. A file (runs 3 and 4) ends with one | A line ending after every line, the last too |
| Q7 | Markdown's alignment | The board's: each column padded to its widest value, counted in characters up to 80 (a longer value runs past); numeric columns right-aligned with `-----:` | No padding: `| a | b |` |
| Q8 | JSON's numbers | Integers and floats as numbers; a numeric column's text (PostgreSQL `numeric`, MySQL `DECIMAL`) as a number when it reads as one, so no digit is lost; anything else (`NaN`, `money`'s `$1.00`) as a string. Booleans as `true`/`false` | Exact decimals as strings |
| Q9 | JSON's json values | Nested: the value's own text, re-indented or put on one line by the writer, keys in its order and numbers as written. A value that is not valid JSON is written as a string | Parsed and written again by `serde_json`, which sorts keys |
| Q10 | JSON's dates and times | A date, a time and a timestamp keep their text; a timestamp gets `T` between date and time; one with an offset writes it as `+HH:MM` (`2026-02-06T08:05:00+00:00`) | As the database writes them, `2026-02-06 08:05:00+00` |
| Q11 | SQL INSERT's layout | One row per INSERT: one statement per line. More: `INSERT INTO public.reviews (id, rating) VALUES` then a row per line, two spaces in, `),` between and `);` after the last. Names as the dialect writes them, the table schema-qualified (`main.` in SQLite) | Every name quoted |
| Q12 | The 20 MB check | The text's bytes as it is written; the writer stops as soon as it passes 20 MB, and the message says "That's over 20 MB. Export to a file?" | Measured after writing all of it |
| Q13 | Where the options are kept | `export_options.json` in the state directory, by format (`csv`, `tsv`, `json`, `jsonl`, `sql`, `md`). An option this version does not know, or a value it cannot read, is kept and written back; a missing one takes the default | One file per format |

## The writers

`crates/tabletist-core/src/export.rs` (and `export/` for its golden
files): no UI, no driver, so runs 3 and 4 stream through them.

```rust
pub enum Format { Csv, Tsv, Json, JsonLines, SqlInsert, Markdown }

/// What every writer is given: the columns in the order written, and the
/// dialect for names and SQL literals.
pub struct Columns<'a> {
    pub names: Vec<&'a str>,
    pub kinds: Vec<ValueKind>,
    pub types: Vec<&'a str>,   // the database's type names
    pub dialect: Dialect,
}

pub struct Writer { … }
impl Writer {
    pub fn new(options: &Options, format: Format, columns: Columns<'_>, target: Target) -> Self;
    /// Appends a batch of rows. `None` is a cell nothing is set in (a new
    /// row's), written as NULL is.
    pub fn rows(&mut self, rows: impl Iterator<Item = impl Iterator<Item = Option<&Value>>>, out: &mut String);
    pub fn finish(self, out: &mut String);
    /// INSERT statements written, for "· 3 statements".
    pub fn statements(&self) -> usize;
}
```

`Target` is SQL INSERT's table (an `ObjectRef`, or `result` for a SQL
result) and its optional `CREATE TABLE` text. Markdown keeps its rows
until `finish`, to pad them (Q7); the others write each batch as it comes.

Values are the database's, never the grid's display (no digit grouping,
no value tags, no display zone). JSON's dates are the one exception (Q10).
A value the grid cuts for display is whole in the page, so it is written
whole.

| Format | Writes |
|---|---|
| CSV | Header row (on/off) · delimiter `,` `;` `\|` or another single character · quoting When needed (a value holding the delimiter, `"`, CR or LF) or Every value (every value but a NULL written Empty, so NULL and `""` differ) · NULL as Empty or Text · UTF-8 BOM · LF or CRLF · binary Hex or Base64 |
| TSV | Header row · the escapes of Q3 · NULL as · BOM · LF or CRLF · binary |
| JSON | `[` one object per row, keys in column order `]` · json values Nested or As strings · dates (Q10) · Indented (two spaces) or One line · binary |
| JSON Lines | One object per line · the JSON options without Layout |
| SQL INSERT | Target table · rows per INSERT 1, 100 or 1,000 · Start with CREATE TABLE (the DDL view's text, then a blank line) · the connection's dialect · binary as the dialect's literal (`'\x…'`, `x'…'`) |
| Markdown | Header row and separator always · NULL as · `\|` and `<br>` for pipes and line breaks · In full or Cut at 200 characters (then `…`) |

SQL literals come from `tabletist_db::Dialect`: a new public
`Dialect::value_literal(&Value, ValueKind) -> String` over the builder's
existing `literal`, `bytes_literal` and `shown`, so a copied INSERT quotes
as a save does. A numeric column's text that reads as a number is written
bare.

## Remembered options

`export::Options` holds one struct per format (`CsvOptions`, …), each
`Default` as Q1 says, read from and written to `export_options.json`
(`AppDirs::export_options_file`) through the backend's `Command::Save`,
as `table_lists.json` is. The app loads it at start into `App::export`.
Nothing in run 2 changes the options (the sheet of run 3 does); tests
store them per format and read them back.

## Copy (⌘C and `y`)

`copied()` and `format::tsv_cells` go. ⌘C, `y` in visual and ⇧⌘C (Copy
rows as TSV) write the selection through the TSV writer with TSV's
remembered options, no header row (D8). One cell alone, with no range or
held by `v`, is copied as today. What is pending reads as its new value,
and a new row's cell nothing is set in is a NULL. The clipboard limit
holds (D9). `yy` is unchanged.

## Copy as

What it copies: the selection's rows and columns (a rectangle's, or every
column of whole rows), in the grid's order, in a table's grid and in a SQL
result's; or the inspector's row, whole. With no range, the cursor's row
whole (as ⇧⌘C).

Formats and keys: CSV `C`, TSV `T`, JSON `J`, Markdown `M`, SQL INSERT
`S`, then Column names `N`. JSON Lines is the export's only.

**macOS:**

- **The grid's context menu** (D1): a secondary click on a cell keeps a
  range it falls in, else selects the cell, then opens Copy ⌘C, Copy as ›
  (the six rows above, with their letters), Copy rows as TSV ⇧⌘C. In a
  table's grid and in a result's.
- **⌥⌘C** opens the same six rows as a menu under the cursor's cell, the
  first row lit. ↑ ↓ and ↩ pick, a letter picks at once, esc closes.
- **The inspector's Copy row ▾** (D2, D4): a secondary button with the
  copy icon, "Copy row ▾", opens Copy row as TSV ⇧⌘C, Copy row as JSON,
  Copy row as CSV, Copy row as Markdown, Copy row as SQL INSERT.

**Omarchy:**

- **`gy`** in the grid and in a result opens a float at the grid: "copy
  as" and the size ("4 rows × 2 columns"), then `c csv`, `t tsv`,
  `j json`, `m markdown`, `s sql insert`, `n column names`, and the
  footer `one key picks · ctrl+n/p enter · esc close`. A letter picks;
  `ctrl+n`/`ctrl+p` move and `enter` picks; `esc` closes. A pick leaves
  visual mode, as `y` does.
- **The inspector:** `gy` opens it titled "copy row as" (D7). Its key
  line reads `yy copy row · gy copy row as`.
- **Key lines:** the visual status line gains `gy copy as` after
  `y copy tsv` (was `y copy`); the grid's and the result's normal key
  lines gain `gy copy as` after `y copy`.

**Feedback,** for 4 s in the toast (macOS) or the status line (Omarchy,
lower case as the line has it): "Copied 4 rows × 2 columns as Markdown";
whole rows say "Copied 12 rows as CSV"; SQL INSERT adds "· 1 statement"
or "· 3 statements"; Column names says "Copied 2 column names". ⌘C stays
silent, as today.

**Over the limit** (more than 10,000 rows, or over 20 MB): nothing is
copied, and the toast or status line says "That's 31,400 rows. Export to a
file?" or "That's over 20 MB. Export to a file?" (D3, Q12).

**History:** Copy as and ⌘C record nothing.

**Screen readers:** the menus' rows are menu items named as they read
(the letter is their shortcut); the float is a list named "Copy as"; the
feedback is the toast's, as today.

## Keys

New in `crates/tabletist-core/src/keymap.rs`, with their scopes, so
Settings › Keyboard lists them and the generated clash tests check them:

| Command | Scope | macOS | Omarchy |
|---|---|---|---|
| `CopyAs` ("Copy as…") | Grid, Results | ⌥⌘C | `gy` |
| `CopyRowAs` ("Copy row as…") | Inspector | (the button) | `gy` |
| `CopyAsCsv`, `CopyAsTsv`, `CopyAsJson`, `CopyAsMarkdown`, `CopyAsSql`, `CopyColumnNames` | Prompt (while the menu or float is open) | `C` `T` `J` `M` `S` `N` | `c` `t` `j` `m` `s` `n` |

⌥⌘C arrives as the window's copy event with Alt held, as ⇧⌘C does; the
key reading takes it where it takes ⇧⌘C.

## Tests (headless, no pixels)

Bookshop data only.

1. **Golden text per format,** in `crates/tabletist-core/src/export/golden/`
   (`.gitattributes`: `-text`, so CRLF and the BOM stay byte for byte on
   Windows): rows of Bookshop's `reviews` with a NULL, an empty string, a
   quote, a comma, a tab, a newline, a pipe, unicode, a jsonb value, bytea
   and a timestamptz. A file for each format's defaults and for each option
   that changes the text: CSV header off, `;`, Every value, NULL as Text,
   BOM, CRLF, Base64; TSV header off, NULL as Text; JSON As strings, One
   line; JSON Lines; SQL INSERT 1 and 100 rows per INSERT, CREATE TABLE,
   each dialect; Markdown Cut at 200, NULL as Empty.
2. **A 300 KiB value** comes out whole in every format (D6).
3. **Streaming shape:** rows given in two batches write what one batch
   does, in every format.
4. **Remembered options:** each format's options saved and read back; a
   missing file gives the defaults; an unknown key is written back.
5. **⌘C and `y`:** a rectangle and rows as TSV with no header, in a table
   and a result; one cell as today; NULL and binary as TSV's options say.
6. **Copy as:** each format for a block, rows and the inspector's row;
   the letters pick in the macOS menu and the Omarchy float; ⌥⌘C and `gy`
   open them; the context menu's rows; the feedback's words, "· N
   statements" and Column names.
7. **The limit:** 10,001 rows and a value over 20 MB copy nothing and say
   why, under Copy as and ⌘C.
8. **History:** nothing recorded by a copy.
9. **Keys:** the new commands in their scopes, free of clashes (the
   keymap's generated tests), listed in Settings › Keyboard.

## Out of scope for run 2

The export sheet and float, writing files, the background export and its
states, the entry points of Export (runs 3 and 4), Set to in the menu, and
import.

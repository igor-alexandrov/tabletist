# SQL editor: Format

Date: 2026-10-01. Status: design, not yet built.

## Intent

The SQL editor runs what is typed but does nothing for how it reads. This
slice adds Format: one key or one button that lays a query out in river
style and uppercases its reserved words, without changing what it means.

Success: a user pastes or types a query on any driver, presses
`Mod+Shift+F`, and reads it in the layout the design's sample query has;
one `Mod+Z` gives back exactly what was there; and no script means
something else after Format than before it.

The core editor's spec (`2026-09-30-sql-editor-core-design.md`) planned
"Explain and Format" as one slice. They share only the toolbar, so Format
is built alone, first. Explain keeps its own spec.

The designs are the "SQL editor" artboards (macOS and Omarchy) in the
design canvas Artifact. They are not copied into the repository.

## Decisions

| Question | Decision |
|---|---|
| Engine | Our own formatter over `sql::tokenize`, in `tabletist-db`. No new dependency. |
| Layout | River style: clause words right-aligned to one column, as in the macOS artboard's sample and the scripts in our tests. |
| What is formatted | The statements a selection touches; with no selection, the whole script. |
| Case | Reserved words that can never be a name are uppercased. Every other word keeps its case. |
| Safety | The result is tokenized again; if any token but whitespace differs, the text is left as typed. |
| Controls | `Mod+Shift+F` in every look. A Format button in the macOS and standard toolbar. Omarchy has the key only, as its artboard has no button. |
| Settings | None. One style, no options. |

Rejected: the `sqlformat` crate (a second lexer that does not know our
dialect rules, and a new dependency where the tokenizer we have does the
hard part); a parser with a pretty-printer such as `sqlparser` (drops
comments, fails on SQL it cannot parse, heavy); uppercasing every word the
highlighter colours (its list holds words that can be names, such as
`FIRST`, `ROWS` and `VIEW`, and MySQL compares table names and aliases by
case on Linux); formatting in `App::apply` (the selection and the undo
history live in egui's memory, which only a view reaches).

## Out of scope

Explain; style settings (indent, case, river width); wrapping long lines at
a width; formatting on paste or before a run; river layout for `INSERT`,
`UPDATE`, `DELETE` and DDL (the editor runs in a read-only transaction, so
they cannot run here anyway); a Format button or key hint in the Omarchy
toolbar and mode line.

## The formatter (`tabletist-db`)

A new module `sql::format` in `crates/tabletist-db/src/sql/format.rs`,
beside the tokenizer in `sql.rs`. It has no UI dependencies.

    pub struct Formatted {
        pub text: String,
        /// Byte offset of the cursor in `text`.
        pub cursor: usize,
    }

    pub fn format(
        dialect: Dialect,
        script: &str,
        selection: Option<Range<usize>>,
        cursor: usize,
    ) -> Option<Formatted>

`selection` and `cursor` are byte offsets in `script`. The result is the
whole script with the formatted part replaced, or `None` when there is
nothing to change: the layout is already the formatter's, the selection
touches no statement, or the safety check failed.

It works on tokens only. It never parses, so it never refuses a script it
does not understand: it lays out what it recognises and copies the rest.
A token is never split, joined, dropped or reordered. Strings, quoted
names, numbers, operators and comments (executable ones included) are
copied byte for byte.

### What is formatted

- No selection (`None`, or an empty range): from the script's first token
  to its last.
- A selection: from the start of the first statement it overlaps to the
  end of the last, each statement taken from its `Statement::range.start`
  to its `Statement::end` (its `;` included). The rest of the script is
  untouched, byte for byte. A selection that overlaps no statement formats
  nothing.
- Whitespace before the first token and after the last token of the
  formatted part stays as typed.

### Query statements and the rest

A statement is a query when its first token that is not a comment is
`SELECT` or `WITH`, or a `(` that opens a block (see below). Only queries
get the river.

Every other statement (`SHOW`, `PRAGMA`, `EXPLAIN ...`, `VALUES`, DML,
DDL) keeps its line breaks and its spacing as typed. Only its reserved
words change case and the whitespace at the ends of its lines is removed.

### The river

A block is a query at the top of a statement or inside parentheses. It
has a base column: 0 for a statement, the column after the `(` for a
nested block. Its river is the 6 columns from the base (the width of
`SELECT`); its content column is the base plus 7.

A clause head starts a new line. Its first word is right-aligned to the
river; a first word longer than 6 letters starts at the base column. One
space follows the head.

    SELECT a.name AS author,
           b.genre,
           count(*) AS books
      FROM books b
      LEFT JOIN authors a ON a.id = b.author_id
     WHERE b.published >= 2020
       AND b.genre <> 'draft'
     GROUP BY a.name, b.genre
     ORDER BY b.genre, books DESC
     LIMIT 10;

The heads, recognised only at the top level of their block (outside every
parenthesis that is not a block, and outside every `CASE`):

- `SELECT` (a following `DISTINCT` or `ALL` stays on its line), `FROM`,
  `WHERE`, `GROUP BY`, `HAVING`, `WINDOW`, `ORDER BY`, `LIMIT`, `OFFSET`,
  `FETCH`, `FOR`, `WITH` (with `RECURSIVE`).
- `JOIN` and `STRAIGHT_JOIN`, with what leads a join: `INNER`, `LEFT`,
  `RIGHT`, `FULL`, `CROSS`, `NATURAL`, each with an optional `OUTER`.
  These words are a head only when `JOIN` follows them, so `left(title,
  2)` is a function call. `ON` and `USING` stay on the join's line.
- `UNION`, `EXCEPT`, `INTERSECT`, with a following `ALL` or `DISTINCT`.
- `AND` and `OR` in a `WHERE`, a `HAVING` or a join's `ON`. The `AND` of
  a `BETWEEN ... AND ...` is not a head.
- `GROUP` and `ORDER` only when `BY` follows.

Because heads count only at the top level, `extract(year FROM added)`,
`count(*) OVER (PARTITION BY genre ORDER BY title)` and `id IN (1, 2)`
stay on their lines.

Lists:

- The `SELECT` list breaks after each top-level comma; the items after
  the first start at the content column.
- The `WITH` list breaks after each top-level comma; each further name
  starts at the content column.
- Every other list (`FROM a, b`, `GROUP BY`, `ORDER BY`, arguments, `IN`
  lists) stays on one line.

Blocks in parentheses: a `(` whose first token that is not a comment is
`SELECT` or `WITH` opens a nested block. It stays on its line, its base
column is the column after it, and its `)` follows the block's last token
on the same line. Every other parenthesis is inline.

    SELECT title
      FROM books
     WHERE author_id IN (SELECT id
                           FROM authors
                          WHERE country = 'PL')
     ORDER BY title;

      WITH recent AS (SELECT id,
                             title
                        FROM books
                       WHERE published >= 2020)
    SELECT title
      FROM recent;

`CASE`: each `WHEN` and `ELSE` goes on its own line, 4 columns in from
the `CASE`; `END` goes on its own line under the `CASE`. An operand
(`CASE genre`) stays on the `CASE` line. What is inside a `CASE` is
inline, apart from a nested `CASE` or block, which follow these rules
from their own column.

    SELECT title,
           CASE
               WHEN stock = 0 THEN 'out'
               WHEN stock < 5 THEN 'low'
               ELSE 'ok'
           END AS availability
      FROM books;

### Spacing inside a line

- One space between two tokens that had whitespace between them, and
  after every comma.
- None before `,`, `;` and `)`, and none after `(`.
- Where two tokens touched, they still touch (`count(*)`, `a.id`,
  `price::text`, `-1`); the formatter adds a space only where a rule
  above puts one. So a space before `(` and around an operator is there
  when it was typed (`IN (1, 2)`, `a = b`) and absent when it was not.
- No tabs: indentation is spaces. Lines end in `\n`, with no whitespace
  before it.

### Comments

- A `--` (or MySQL `#`) comment ends its line: what follows starts a new
  line at the content column, or as the head it is.
- A comment that had a line to itself keeps one: at the base column
  before a statement's first token, at the content column inside it.
- A comment after code on its line stays after that code, one space
  apart.
- A block comment is copied whole; the lines inside it are not
  re-indented.
- A `)` or `;` that would follow a `--` comment goes on the next line.

### Between statements

- A `;` follows its statement's last token on the same line.
- Two statements are one blank line apart.
- Around a comment that stands between statements, blank lines stay as
  typed, at most one in a row.

### Case

A token the tokenizer reads as a keyword in this dialect is uppercased
when it is on this list, of words no dialect lets be an unquoted name:

`ALL ALTER AND AS ASC BETWEEN BY CASE CREATE CROSS DEFAULT DELETE DESC
DESCRIBE DISTINCT DROP ELSE EXISTS EXPLAIN FALSE FETCH FOR FROM GLOB GROUP
HAVING ILIKE IN INNER INSERT INTERVAL INTO IS JOIN LATERAL LEFT LIKE LIMIT
NATURAL NOT NULL ON OR ORDER OUTER PARTITION PRAGMA REGEXP RETURNING RIGHT
SELECT SET SHOW SIMILAR STRAIGHT_JOIN TABLE THEN TO TRUE UNION UPDATE
USING VALUES WHEN WHERE WITH`

The highlighter's other keywords can be names (`ANY BEGIN CAST COMMIT
CURRENT END EXCEPT FILTER FIRST FULL INTERSECT NO OFFSET ONLY OVER
RECURSIVE ROLLBACK ROWS VIEW WINDOW`) and keep their case, with one
exception: on PostgreSQL and SQLite, where an unquoted name means the
same in any case, a word the layout read as structure is uppercased too
(`OFFSET`, `EXCEPT`, `INTERSECT` and `WINDOW` as heads, `FULL` in a join
head, `RECURSIVE` after `WITH`, `END` closing a `CASE`). On MySQL those
keep their case as well: a table alias is compared by case there, and such
a word may be one.

The list is checked when it is built: a word is on it only if MySQL 5.7,
MySQL 8 and MariaDB all reserve it. A word in doubt stays off.

### The safety check

Before returning, the formatter tokenizes its result and compares it with
the input, whitespace tokens left out: the same number of tokens, each of
the same kind and the same text (keywords compared without case). If they
differ, it returns `None` and logs that Format left the text alone,
without the text. This is what holds the promise in the intent against
cases no rule foresaw, such as two operators that would fuse into `--`.

### The cursor

The cursor keeps its place among the tokens. Counting the bytes of
tokens that are not whitespace before it: if it stood right after such a
byte, it stands right after the same byte; otherwise it stands before the
next token, or at the end of the text when there is none.

## The editor

- `Action::FormatSql { tab, sql_tab }` asks for it. `App::apply` sets a
  flag on the `SqlTab` (as `focus_editor` is one) and `focus_editor`, so
  the keys are the editor's after a click on the button.
- `sql_text::edit` takes the flag before it draws the `TextEdit`. There it
  has the field's state: it turns egui's cursor range (in characters) into
  a byte selection, calls `sql::format::format` with the workspace's
  dialect, and on `Some` replaces `SqlTab.text`. This is the text the
  field is editing, which the agent guide lets a view change.
- Undo: before the text is replaced, the state as it was (cursor range and
  text) is added to the field's undo history, and the formatted state
  after it. One `Mod+Z` gives back the typed text and its selection; a
  `Mod+Shift+Z` formats it again.
- The cursor goes where `Formatted::cursor` says, with nothing selected.
  `SqlTab.cursor` follows it in the same frame.
- A field that never had the keys has no egui state: Format then uses
  `SqlTab.cursor` and no selection.
- `None` changes nothing and adds nothing to the undo history.
- Format works while a run is in flight (the run has its own copy of the
  text). As after any edit, an error mark from the last run goes away:
  its line was a line of the text that ran.

## Controls

- `Mod+Shift+F` on a SQL tab, whether or not the editor has the keys,
  consumed before the editor sees it. It is matched before `Mod+F`. It
  does nothing on other tabs.
- The shortcuts dialog gains "Mod+Shift+F: Format SQL".
- macOS and the standard look: after Run all, a 1 pt divider 20 pt tall
  (4 pt of margin at each side, inside the bar's 8 pt gaps), then a
  borderless Format button in the secondary text colour with its key
  (`⇧⌘F`, or `Ctrl+Shift+F`), as the artboard draws it. Its accessible
  name is "Format"; on hover it reads "Format the SQL". Explain will sit
  before the divider when it is built.
- Where the toolbar is too narrow, pieces give way in this order: the
  buttons' keys (Format's with the run buttons'), the read-only note, the
  Format button with its divider, then the menus' words, the menus'
  chevrons, and last the menus. The run buttons stay.
- Omarchy: no button and no mode-line hint. The key works and the
  shortcuts dialog names it.
- New strings go through `gettext`.

## Errors and edge cases

- An empty or comment-only script, and a script already in this layout:
  nothing happens, and nothing is added to the undo history.
- Formatting is idempotent: formatting a formatted script returns `None`.
- An unterminated string, name or comment is one token to the end of the
  text; it is copied as it is and what stands before it is laid out.
- A `)` with no `(` and a `(` never closed do not stop the formatter: the
  first is inline punctuation, the second's block ends with its statement.
- A name that spells a clause word (a column called `offset`) is laid out
  as that clause. The query means the same, since only whitespace moved
  and the word's case is kept; quoting the name avoids it.
- A `\r\n` line ending between tokens becomes `\n`; inside a string or a
  comment it is copied.
- The formatter runs on the UI thread, as the tokenizer does: one pass
  over the tokens, on a key press.

## Testing

- `tabletist-db` unit tests for `sql::format`, as script-in, script-out
  cases: each head; joins with and without `OUTER`; `left(...)` as a
  function; `BETWEEN ... AND`; `AND` and `OR` in `WHERE`, `HAVING` and
  `ON`; the `SELECT` list and the lists that stay on a line; `DISTINCT`;
  nested blocks in `IN`, in `FROM` and in `WITH`, two deep; set
  operations; `CASE`, nested and with an operand; window functions and
  `extract(... FROM ...)` staying inline; comments on their own line, at
  the end of a line, before `)` and `;`, and between statements; several
  statements and the blank line between them; a non-query statement
  keeping its lines; `\r\n` input.
- Dialect cases: `$$` and `$tag$` bodies, `E'\''` and `::` (PostgreSQL);
  backticks, `#` comments, `--` with and without a following space and
  `/*! */` (MySQL); `[name]` and `PRAGMA` (SQLite).
- Case: every listed word is uppercased; the words that can be names
  keep their case on MySQL; the structural ones are uppercased on
  PostgreSQL and SQLite; a quoted name and a string that spell a keyword
  are untouched. The list is a subset of the highlighter's keywords.
- Invariants, over every case above and over the tokenizer's own test
  scripts, in all three dialects: the tokens but whitespace are the same
  before and after; formatting the result returns `None`; the safety
  check passes. A case built to fuse tokens (`- -1`, `/ *`) returns the
  operators unfused.
- The selection: one statement of three is formatted and the others are
  byte for byte the same; a selection across two statements formats both;
  a selection in the whitespace between statements formats nothing.
- The cursor: inside a word, at a word's end, at a line's start, in
  leading whitespace and at the end of the script, before and after.
- On the MySQL test server (when its URL is set): each listed word is
  refused where a table alias is expected, and a word left off the list
  (`first`) is accepted there.
- Headless UI tests through `src/testing.rs`: `Mod+Shift+F` formats the
  script, with the editor focused and not; the Format button does the
  same and gives the keys back to the editor; with a selection only the
  statements it touches change; `Mod+Z` restores the text as typed;
  Format on a formatted script leaves the undo history as it was; the key
  does nothing on a table tab; `Mod+F` still does nothing on a SQL tab;
  the Omarchy toolbar has no Format button and the key still formats; the
  shortcuts dialog names the key.
- The toolbar's give-way order, as widths: the Format button is there at
  full width, gone before the menus shorten, and the run buttons stay.
- No design or pixel conformance checks. Screenshots for review use the
  Bookshop demo data and stay local.

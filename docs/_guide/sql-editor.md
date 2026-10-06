---
title: SQL Editor
description: Run statements, choose between read-only and read-write runs, and read the results and messages.
nav_order: 5
---

Press Cmd/Ctrl+T for a new SQL editor. Each one is a tab named
**Query 1**, **Query 2** and so on, and belongs to the connection it was
opened on.

## Run

- **Run** (Cmd/Ctrl+Return) runs the statement the cursor is in.
- **Run all** (Cmd/Ctrl+Shift+Return) runs the whole script.
- Cmd/Ctrl+. cancels a run.

{% include shot.html file="macos-sql" alt="A SQL editor with two statements, the result of the one at the cursor under it" %}

Two menus in the toolbar bound every run:

| Menu | Choices | Default |
| --- | --- | --- |
| **Limit** | 100, 1,000 or 10,000 rows for each statement | 1,000 |
| **Timeout** | 10, 30, 60 or 300 seconds, or no timeout | 30 seconds |

Your choice is remembered for new tabs.

## Read-only and read-write

The toolbar's switch (Cmd/Ctrl+Shift+M) sets what a run may do.

- **Read-only.** The run happens in a read-only transaction that is
  rolled back. Nothing is changed.
- **Read-write.** A run that changes data is one transaction. It is
  committed when every statement succeeded, and rolled back on the first
  error, on cancel and on timeout. A run that only reads is still
  read-only.

{% include shot.html file="macos-sql-committed" alt="A read-write run of two statements, with Messages saying how many rows each changed and that the run was committed" %}

A new tab opens in Read-write only on a writable connection that is not
production. On a connection that opens read-only, and on any production
connection, every run is read-only.

Because Tabletist runs your script in its own transaction, statements that
would leave it are refused, and then nothing runs. `BEGIN`, `COMMIT`,
`ROLLBACK` and `SAVEPOINT` are examples.

On MySQL, statements such as `CREATE`, `ALTER` and `DROP` are committed by
the server as they run, so they cannot be rolled back. The messages say
which lines were written.

## Results and messages

**Results** shows the rows of the statement you ran. Select a row and
press Space to read every field of it in the row panel. When the limit cut a
result short, the result says so.

**Messages** has a line for every statement: how many rows it returned or
changed and how long it took, or the database's error with the
statement's line, and the column too when the database names the place. A read-write run ends with whether it was committed or rolled
back.

## Format

**Format** (Cmd/Ctrl+Shift+F) lays queries out in river style and
uppercases reserved words. It formats the statements in the selection, or
the whole script when nothing is selected. Only whitespace and the case of
keywords change.

## Completion

Keywords, schemas, tables, views and the columns of the tables in your
statement are offered while you type. Ctrl+Space or Cmd/Ctrl+I asks for
the list anywhere.

{% include shot.html file="macos-sql-complete" alt="The completion list under a table name being typed, offering the tables that match" %}

Tab accepts the highlighted row, and Esc closes the list. Enter accepts
too once you moved in the list or asked for it. When the list opened by
itself and its row is only a guess (it does not begin with what you
typed), or is what you typed already, Enter stays a line break.

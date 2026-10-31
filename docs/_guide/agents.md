---
title: Agents
description: Let coding agents such as Claude Code and Cursor read the databases you choose through Tabletist, over MCP, and see and stop what they do.
nav_order: 6
---

Tabletist can let coding agents, such as Claude Code and Cursor, read the
databases you choose. An agent asks Tabletist over the Model Context
Protocol (MCP), and Tabletist runs the query on your saved connection,
with that connection's own login and its limits. Passwords stay in the
system keyring and never reach the agent.

Agents work on Linux, macOS and Windows.

## What an agent can do

An agent has three tools:

| Tool | What it does |
| --- | --- |
| `list_connections` | Lists the connections you allowed agents, with what each is and its limits |
| `describe_schema` | Names a connection's tables and views, and describes the columns, keys and comments of those it asks about |
| `run_query` | Runs one `SELECT`, `WITH … SELECT`, `VALUES` or `EXPLAIN` of one, and returns its rows |

Every read runs in a read-only transaction that is rolled back, with the
connection's limits:
rows per answer and a statement timeout. Tabletist checks each statement
before it runs. A statement that would write, lock or change the schema is
refused, and so are several statements at once and `COPY`. An agent sees
what the connection's database user can see, and never more. You can
also [hide tables](#hide-tables) from them.

Agents cannot write in this version.

## Turn it on

The agent server is off until you turn it on, in either place:

- **Settings › Agents**: the **Let agents use Tabletist** switch.
- A connection's Agents tab, while the server is off: **Turn on**.

The server runs only while Tabletist is open, and only on this computer:
an agent reaches it through `tabletist mcp`, a small program it starts,
which talks to the running app. Turning the switch off ends every agent's
session until you turn it on again.

Turning the server on lets no agent see any connection yet. That is the
next step.

## Connect an agent

**Settings › Agents** shows how, under **Connect an agent**. Choose the
agent, then **Copy** what it takes:

- **Claude Code**: a command to run once in your project:

  ```sh
  claude mcp add tabletist -- tabletist mcp
  ```

- **Cursor**: an entry under `mcpServers` in `~/.cursor/mcp.json`, or in
  `.cursor/mcp.json` in a project. A file that has other servers already
  takes the `tabletist` entry beside them:

  ```json
  {
    "mcpServers": {
      "tabletist": { "command": "tabletist", "args": ["mcp"] }
    }
  }
  ```

- **Other**: any agent that starts an MCP server over stdio runs the
  command `tabletist mcp`.

Where `tabletist` on your `PATH` is not the app you run, the command names
the app by its whole path instead. On macOS that is the program inside the
app: `/Applications/Tabletist.app/Contents/MacOS/tabletist mcp`. On
Windows it is where the installer put it, in quotes where the path has a
space, which the Command Prompt and PowerShell both take:

```sh
claude mcp add tabletist -- "C:\Users\Ada Lovelace\AppData\Local\Programs\Tabletist\tabletist.exe" mcp
```

Copy gives the one your Tabletist needs.

In the Omarchy look, `y` on the Agents pane copies Claude Code's command.

## Allow a connection

Every connection starts with agents **Off**: they do not see it at all.
A new Files connection is the exception: it starts with **Read**, or
with **Off** on production. **Read** lets agents read a connection. Set
it in any of these places:

- In Connections: a row's "…" menu, or `a` on a row in the Omarchy
  look.
- In the connection's form: its **Agents** section, which also has the
  rows per answer and the statement timeout.
- In the connection's Agents tab, while it says agents can't use it:
  **Allow agents…**, which opens that section with **Read** chosen.

[Connections]({% link _reference/connections.md %}#agents) describes the
section.

## Hide tables

A connection can keep some of its tables from agents. In its form's
**Agents** section, set **Tables**:

- **All**: agents may read every table. This is where every connection
  starts.
- **All except**: every table but the ones you list.
- **Only**: the ones you list and no others. A table made later is hidden
  too, until you list it.

**Add table…** takes a table's name, `customers`, or with its schema,
`sales.customers`; a name without a schema is that table in every
schema. While a window of the connection is open, it lists the tables
that window's sidebar has loaded as you type. A listed name that the
loaded schemas show is not a table is marked **not in this database**:
one whose schema is loaded and has no such table, one whose schema the
database does not have, or, for a name without a schema, one that no
schema has once every schema is loaded. What the sidebar has not loaded
is not known, and not marked.

What a hidden table hides with it:

- Its rows, and its columns: `describe_schema` leaves it out, and the keys
  that point to it or from it.
- A view over it, a materialized view, and a partition or a child table of
  it. On MySQL, a view whose tables the server does not list.
- The system catalogs (`information_schema`, `pg_catalog`, `mysql`,
  `sqlite_schema` and the rest), while anything is hidden: they describe
  every table.
- A function your database's users wrote, while anything is hidden: what
  it reads cannot be seen, and a view that calls one is hidden. On
  PostgreSQL, so is a table whose row policy calls one; and while the
  database has an operator, a cast or a type whose function its users
  wrote, every query is refused, as such a function runs without the query
  naming it.

Agents are told the names in the list, so that they know why a table is
missing; they never see what is in it. If a table you hid is renamed or
dropped, agents may not use the connection at all until you take the name
out, or put in its new one: its new name would be hidden by nothing.

Tabletist checks every query before it runs, and asks the database what
the query would read. A function that comes with an extension can still
read a hidden table, and so can a foreign table that points at it. For
strict isolation, give the connection a database user without access to
those tables.

A change in `connections.toml` that hides less, made outside Tabletist,
waits for your answer: see [below](#changes-made-outside-tabletist).
[Settings and Files]({% link _reference/settings-and-files.md %}#the-connections-file)
has the keys.

## Mask columns

A connection can give agents some columns' names and types, and never
their values. In its form's **Agents** section, **Masked columns** lists
them, and **Add column…** takes a table and a column, `orders.email`, or
with the schema, `sales.orders.email`; one without a schema is that
column of the table of that name in every schema.

While a window of the connection is open, Add column… lists the tables
that window's sidebar has loaded as you type. Choose one and it goes into
the field, `orders.`, and the list shows that table's columns, which
Tabletist asks the window for. A masked column that the loaded schemas
and fetched columns show is not in the database is marked **not in this
database**. What the sidebar has not loaded is not known, and not marked.

What agents get of a masked column:

- `describe_schema` gives its name and type, marked masked, and leaves
  out its default, its comment and the values it may hold. It leaves
  out the comment of the column's table too, and of a view that reads
  it.
- `run_query` answers `•••` for every value, an empty one too, where a
  query selects the column as it is.
- A query that filters, joins, sorts, groups or computes on it is
  refused, and so is one that does that to any column of the same name in
  any table. So is a view that reads it.

Its partitions and child tables have it masked too, and so does a
generated column of its table. If a masked column is renamed or dropped,
agents may not use the connection at all until you take it out, or put in
its new name: its new name would be masked by nothing.

A function that comes with an extension can still read a masked column.
For strict isolation, give the connection a database user without access
to it.

A change in `connections.toml` that masks less, made outside Tabletist,
waits for your answer: see [below](#changes-made-outside-tabletist).

## See what agents do

Each connection window has an Agents tab for its connection. It opens
from the **Agents** entry at the bottom of the sidebar, from Cmd+Option+A
on macOS, Ctrl+Alt+A on Windows, or `ga` and `:agents` in the Omarchy
look, from quick open on macOS and Windows, from Connections, and from
**Show Agents tab** in **Settings › Agents**. Nothing an agent does opens
it for you.

The tab shows:

- whether agents can reach the connection, and if not, the way to let
  them;
- the agents working on it, each with **Stop session**, and the ones
  working elsewhere;
- the calls they make, as they make them.

**Stop agents here** ends every session on the connection and sets its
access to **Off**, so that an agent that connects again does not see it.

The tab's **Log** view keeps every call: see [below](#the-log).

When an agent connects, Tabletist tells you: a notification on Linux,
whose **Review** shows the Agents tab, the Dock icon bouncing on macOS,
and the taskbar button flashing on Windows.
**Notify when an agent connects** turns it off.

## The log

The Agents tab's **Log** view lists every call agents made, on this
connection or on all of them, the newest first: the time, the agent, the
tool, the statement or the tables asked for, how many rows it returned,
how long it took, and how it ended. Rows themselves are never logged. The
log is read from its file, so it shows past calls with the server off and
with agents' access off. **Settings › Agents › Keep the activity log**
says for how many days a call is kept. Open it with **Show log** or the
switch at the tab's top, or `l` in the Omarchy look, and `:agents log`
from anywhere in the window.

A call that brought a statement is in its connection's
[History tab]({% link _guide/sql-editor.md %}#history) too, among what you
ran yourself, marked as an agent's. History keeps none of what the agent
sent: its entry links to the call here, and **Show in Agents Log** opens
this view with the call selected.

### Filter the calls

The bar narrows the list: **This connection** or **All connections**, one
agent of those that called or **All agents**, one way a call ended
(**ok**, **refused**, **failed**, **timed out**, **cancelled**,
**stopped**) or **All results**, and **Search requests…**, which finds
the text anywhere in a statement or in the tables `describe_schema` was
asked for, English letters in either case. The count then says how many
calls match. In the Omarchy look, `L` switches between this connection
and all of them, and `/` opens the other filters over the list, with the
keyboard in the search: Tab moves to the agents and the results, Enter
or Space picks one, and Esc closes them and keeps them. A line under the
bar says what they are set to.

### A call's details

A call you select shows its details beside the list: the statement as it
ran, its values, what it returned, the columns that were masked, how it
ran and how long it took, or why it was refused or failed. **Open in SQL
editor** puts the statement in a new SQL editor of its connection, in
read-only mode and not run, in a window of that connection on its saved
database: this one for a call on this connection, another that is open,
or one that opens and connects for it. Its values come first, as the log
keeps them, in a comment line each (`-- $1 = 9100000000000000004` on
PostgreSQL, `?1` on SQLite, `? #1` on MySQL), for you to put in its
placeholders. A call on a connection deleted since cannot be opened. In
the Omarchy look `j` and `k` move among the calls, Enter shows the
details, `o` opens the statement, and Esc closes them.

### Export

**Export…**, at the bar's right, or `E` in the Omarchy look, saves every
call that passes the filters, not only those on the screen, the newest
first as the list shows them. Name the file `.jsonl` (or `.ndjson`) for
JSON Lines, one object a line; anything else is CSV, with a line of the
column names first. Each call has:

| Field | What it is |
|---|---|
| `at`, `at_ms` | When the call came, as RFC 3339 in UTC and in milliseconds since 1970 |
| `session`, `agent`, `project`, `transport` | Who called: the session, the agent as it names itself, its project, and how it connected |
| `connection_id`, `connection` | The connection, by its id and by its name then |
| `tool`, `sql`, `params`, `request` | What was asked: the statement and its values (as JSON), or the tables asked for |
| `result`, `code`, `message` | How it ended, the refusal's code, and its sentence |
| `ran_as`, `rows`, `matched`, `columns`, `bytes`, `masked`, `took_ms` | How it ran, what it returned, the columns masked (as JSON), and how long it took |

A field a call has not is empty in CSV and `null` in JSON Lines. The file
is readable by your account alone, as the log is: its statements and
values can hold data.

## Changes made outside Tabletist

Agents cannot change Tabletist's files through MCP. A program that edits
`settings.toml` or `connections.toml` to let agents do more is held back
until you answer: see
[Settings and Files]({% link _reference/settings-and-files.md %}#agent-access-changed-outside-tabletist).

The Omarchy look's keys for the Agents tab and the Settings screen are in
[The Omarchy Look]({% link _guide/omarchy.md %}#the-agents-tab).

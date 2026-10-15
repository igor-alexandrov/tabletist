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

Agents work on Linux and macOS. On Windows nothing of them shows yet.

## What an agent can do

An agent has three tools:

| Tool | What it does |
| --- | --- |
| `list_connections` | Lists the connections you allowed agents, with what each is and its limits |
| `describe_schema` | Names a connection's tables and views, and describes the columns and keys of those it asks about |
| `run_query` | Runs one `SELECT`, `WITH … SELECT`, `VALUES` or `EXPLAIN` of one, and returns its rows |

Every read runs in a read-only transaction that is rolled back, with the
connection's limits:
rows per answer and a statement timeout. Tabletist checks each statement
before it runs. A statement that would write, lock or change the schema is
refused, and so are several statements at once and `COPY`. An agent sees
what the connection's database user can see, and never more.

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
app: `/Applications/Tabletist.app/Contents/MacOS/tabletist mcp`. Copy gives
the one your Tabletist needs.

In the Omarchy look, `y` on the Agents pane copies Claude Code's command.

## Allow a connection

Every connection starts with agents **Off**: they do not see it at all.
**Read** lets them read it. Set it in any of these places:

- In Connections: a row's "…" menu, or `a` on a row in the Omarchy
  look.
- In the connection's form: its **Agents** section, which also has the
  rows per answer and the statement timeout.
- In the connection's Agents tab, while it says agents can't use it:
  **Allow agents…**, which opens that section with **Read** chosen.

[Connections]({% link _reference/connections.md %}#agents) describes the
section.

## See what agents do

Each connection window has an Agents tab for its connection. It opens
from the **Agents** entry at the bottom of the sidebar, from Cmd+Option+A
on macOS or `ga` and `:agents` in the Omarchy look, from quick open on
macOS, from Connections, and from **Show Agents tab** in **Settings ›
Agents**. Nothing
an agent does opens it for you.

The tab shows:

- whether agents can reach the connection, and if not, the way to let
  them;
- the agents working on it, each with **Stop session**, and the ones
  working elsewhere;
- the calls they make, as they make them.

**Stop agents here** ends every session on the connection and sets its
access to **Off**, so that an agent that connects again does not see it.

The tab's **Log** view lists every call agents made, on this connection or
on all of them: the tool, the statement, how many rows it returned and how
long it took. Rows themselves are never logged. **Settings › Agents ›
Keep the activity log** says for how many days a call is kept.

When an agent connects, Tabletist tells you: a notification on Linux,
whose **Review** shows the Agents tab, and the Dock icon bouncing on macOS.
**Notify when an agent connects** turns it off.

## Changes made outside Tabletist

Agents cannot change Tabletist's files through MCP. A program that edits
`settings.toml` or `connections.toml` to let agents do more is held back
until you answer: see
[Settings and Files]({% link _reference/settings-and-files.md %}#agent-access-changed-outside-tabletist).

The Omarchy look's keys for the Agents tab and the Settings screen are in
[The Omarchy Look]({% link _guide/omarchy.md %}#the-agents-tab).

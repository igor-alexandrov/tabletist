# Plan: the Tabletist MCP server

Status: a plan for review. No app code changes with it.

Written on 2026-10-13 against `main` at `89a7e88`. Line numbers are from that
commit. Designs: the "Agents · MCP server" section of the Tabletist canvas
(twelve boards, since the change below). They are not copied into the
repository.

**Changed on 2026-10-13: the Agents tab.** The designs changed after this
plan was written, and the change answers open question 6. Activity and the
log are no window of their own, and no tab of the app either: each
connection window has an **Agents** tab, next to its table and query tabs,
that covers that connection. Two boards are gone (the activity window and
the log window), five are new, and one is renamed. Section 2, A.5, D, F, G
and the open questions are revised for it, each where it changed. Nothing
else is: the tools, the enforcement, per-connection access, storage and the
threat model stand as they were. Line numbers in the revised parts are from
`main` at `cf7b78f`.

**Settled on 2026-10-13: eight questions, and Phase 1b in runs.** Questions
9, 10, 14, 21, 22, 23, 24 and 26 are answered, each where it stands in the
list at the end, and the sections they change say so. With 5, 6, 15 and 16
before them, no question marked as one Phase 1 waits for is open. Phase 1c
has two of its own (8 and 13). G cuts 1b into five runs.

**Built on 2026-10-13: Phase 1b's first run** (`2026-10-13-mcp-server-1b-1-server.md`), the
server with nothing to start it. This plan named the tools and left open
what each takes and answers: that run's plan says. A.1, A.4, A.5, B.5, B.8,
C.1, F.1 and G are corrected where its draft showed otherwise, each where it
changed.

**Built on 2026-10-14: the first part of Phase 1b's second run** (`2026-10-14-mcp-server-1b-2a-gateway.md`):
the backend's gateway, and agents in `connections.toml`. G cuts the second
run in two, as it said it might. A.1, C.1, F.4, G and question 19 are
corrected where its draft showed otherwise.

**Built on 2026-10-14: the second part of Phase 1b's second run** (`2026-10-14-mcp-server-1b-2b-stdio.md`):
the `[mcp]` settings, the endpoint, the key and the relay, and the keychain
on macOS. Four departures were agreed the same day, each where it stands:
the key beside the socket, the app's answer to the relay's first line
(A.2), the website's reference for the files (G), and the check of changes
made outside the app moved to the first control that turns agents on (C.2).

**Built on 2026-10-14: the first part of Phase 1b's third run** (`2026-10-14-mcp-server-1b-3a-agents-tab.md`):
the Agents tab, its entry in the sidebar, the ways in, the three empty
states with **Turn on** and **Copy**, and the sessions the app learns of.
G cuts the third run in two, agreed the same day: the app must know who is
connected before it can say that nobody is, so the sessions came forward
from the fourth run, and the check of changes made outside the app (C.2)
is the second part. What the run draws otherwise than the boards is in
D.4.

**Built on 2026-10-14: the second part of Phase 1b's third run** (`2026-10-14-mcp-server-1b-3b-changes-outside.md`):
C.2's check of the changes made outside the app. What it settled where C.2
left it open is there.

Each phase below gets its own plan under `docs/superpowers/plans/` before any
code is written, as every batch of work in this repository does.

**Dropped on 2026-10-15, for now: Phase 1d, HTTP with tokens.** Phase 1b
is built on Linux, macOS and Windows, and agents connect over stdio alone,
which Claude Code, Cursor and any agent that starts an MCP server take.
Nothing of 1d is planned or built: no network listener, no tokens, no
"Other" or agent list with Remove in Settings › Agents. A.3 and G's 1d are
kept as the design for if it is taken up again. Where another section
says something comes "with HTTP (1d)", it is not planned now.

**Settled on 2026-10-15: questions 8 and 13, and Phase 1c in three runs.**
An agent is told the names of the tables hidden from it, and the system
catalogs are open on a connection that hides nothing and closed on one
that hides something, each as this plan recommended. G cuts 1c into three
runs. Its first (`2026-10-15-mcp-server-1c-1-checks.md`) built the checks, with nothing visible: B.2 and
B.3 are corrected where its draft showed otherwise.

## Contents

1. [What the code says, in short](#1-what-the-code-says-in-short)
2. [Where the brief and the code disagree](#2-where-the-brief-and-the-code-disagree)
3. [A. Architecture](#a-architecture)
4. [B. Enforcement](#b-enforcement)
5. [C. Data and settings](#c-data-and-settings)
6. [D. UI work](#d-ui-work)
7. [E. Threat model](#e-threat-model)
8. [F. Testing](#f-testing)
9. [G. Phases](#g-phases)
10. [Open questions](#open-questions)
11. [Sources](#sources)

## 1. What the code says, in short

- **Most of the read-only fence exists.** `Connection::run_script(statements,
  limit, ScriptMode::ReadOnly, stop)` (`crates/tabletist-db/src/lib.rs:194`)
  already runs one read-only transaction that is always rolled back, behind a
  refusal list, a check before every statement and a cleanup, on all three
  drivers. `run_query` is that call with one statement, plus stricter checks
  in front of it.
- **The row cap exists too.** PostgreSQL reads through a cursor and fetches
  `limit + 1` rows, MySQL sets `sql_select_limit`, SQLite stops stepping
  (`pg/script.rs:71`, `mysql/script.rs:164`, `sqlite.rs:416-425`). No `LIMIT`
  wrapping is needed.
- **Four things the drivers do not have:** a statement timeout on the server
  (only a client-side timer, `src/backend.rs:2417`), bind parameters for SQL
  text (`mysql/script.rs:447` refuses a `?`), a size cap on an answer
  ("Each cell comes back whole, however large", `lib.rs:271-274`), and any
  knowledge of what a view or a function reads (`catalog.rs` has tables,
  columns, indexes and foreign keys only).
- **There is a tokenizer, not a parser.** `sql::tokenize`, `sql::statements`,
  `sql::refusal` and `sql::kind` work on tokens (`sql.rs:235`, `:595`,
  `:684`, `:1070`). `kind` errs toward "write" and is not a safety gate. It
  cannot name the tables a statement reads.
- **Nothing listens today.** No socket, pipe, lock file or second window
  exists. The CLI has flags and no subcommand (`src/entrypoint.rs:8-23`).
- **Sessions belong to tabs and run one command at a time**
  (`src/backend.rs:43`, `run_session` at `:2036`). The server needs sessions
  of its own.
- **The UI is one window with one modal dialog at a time**
  (`pub dialog: Option<Dialog>`, `src/app.rs:85`). Settings has one pane,
  "General", hard-coded in both looks (`src/ui/settings/sheet.rs:184`).
  There are no OS notifications.

## 2. Where the brief and the code disagree

Each row is something the brief or a board asks for that the code cannot give
as drawn. Each needs a yes or a no before the phase that builds it.

| # | The brief or the board | What the code has | What this plan proposes |
|---|---|---|---|
| 1 | Per-connection settings in `connections.toml` under `[connections."…".agents]` (decision 7, both Omarchy boards) | Connections are in `connections.json`, version 2 (`src/paths.rs:72-74`, `src/connections.rs:80`). No TOML holds a connection. Names are not unique: two connections named "Bookshop" form a group (`src/ui/picker.rs:63-100`), so a table keyed by name cannot hold them | An `agents` object on each connection in `connections.json`. The Omarchy editor shows that fragment where the board shows TOML. Moving connections to TOML is its own project (question 5) |
| 2 | Settings › Agents beside General, Appearance, Editor, Keyboard, AI assistant, Updates | One pane. No pane list, no pane state (`SettingsDialog { row, resetting }`, `src/model.rs:1446-1453`) | Phase 1b adds the pane switch and builds two panes: General and Agents. The other five stay unbuilt |
| 3 | An **Agents** tab in each connection window, next to its table and query tabs. (Changed on 2026-10-13: the boards first drew an activity window and a log window) | A connection window is what the app already calls a connection's tab in its one OS window (`Command::GoToConnectionWindow`, `src/keymap.rs:835-840`). Inside it is a strip of tabs, `Workspace::tabs` (`src/model.rs:815`), and a `Tab` is a table or a SQL editor (`:2446-2449`) | `Tab` gains a third kind, `Agents` (D.4). This is what the boards draw now, and it settles question 6 |
| 4 | macOS: View › Agents ⌥⌘A, in a View menu with Data, Structure and Inspector | The app builds no menu. winit's default app menu is patched for About and Settings through `crates/tabletist-appkit` (`src/macos.rs:133-200`). There is no View menu | ⌥⌘A is a command of the keymap and needs no menu. The sidebar entry, quick open, Settings and the Connections row are built in Phase 1b. The menu item waits for a View menu, which is its own work (settled, question 24) |
| 5 | Omarchy: `ga` from the grid or the sidebar, `:agents` "from anywhere", `:agents log` | The `:` prompt exists only in a workspace (`src/ui/keys.rs:1804-1814`, `App::run_command`, `src/app/editing.rs:1351`), and the keymap looks an ex command up in the Grid and SQL editor scopes alone | `:agents` answers in every scope of a connection window, the Agents tab's own included. The connections screen needs no `:` prompt: its way in is `a` on a row and then `t`, as the board draws (this replaces the first proposal) |
| 6 | Omarchy: `[a] approve and run  [d] deny with a note` in the Agents tab, and the bottom line `-- AGENT WRITE -- … a approve d deny esc later` | A single letter is safe only where nothing else reads it. In a grid's normal mode `a` would approve a write for anyone whose fingers know vim's append | `a` and `d` are keys of the Agents tab alone (a new `Agents` scope), and the tab never takes the keys by itself. The bottom line names `a` and `d` while that tab has the keys. In any other tab it names `ga` (D.4, question 12) |
| 7 | "Counted by a dry run, rolled back", before the person is asked | Nothing in the app runs a write to count it. A dry run on a MySQL table that is not transactional is a real write (`rollback_warning`, error 1196, is how the script run reports it) | Count with a read-only `SELECT count(*)` built from the statement. A dry run becomes something the person asks for on the card (B.6, question 7) |
| 8 | Production: "Can it ever be above Read?" | A SQL tab on production cannot write at all today: `Workspace::sql_writes` answers `NoWrites::Unconfirmed` because the question for a script is not built (`src/model.rs:2840-2848`) | Production is capped at Read until that question exists. Then approved writes on production pass both the approval and the production confirmation |
| 9 | "Use only the Bookshop demo data in fixtures and tests" | There is no Bookshop database. The SQL fixtures are `users`, `orders`, `events`, `big` (`crates/tabletist-db/fixtures/`). Bookshop exists as in-memory scenes for screenshots (`src/shots.rs`) | Phase 1a adds a small Bookshop SQL fixture for the three drivers, with the tables the boards use (question 15) |
| 10 | "macOS and Omarchy get the same features" | There are three looks: Windows shares the macOS code paths (`Look::standard`, `src/theme.rs:390`), and the release builds Windows packages | Windows gets the macOS variant with Ctrl for Cmd, a named pipe for the shim, and a taskbar flash where the others notify. It gets them one release after the other two, and shows nothing of Agents until then (settled, question 14) |
| 11 | `list_connections` shows `hidden: [customers]`, and the refusal says "customers is hidden" | The editor's hint says hidden tables "don't exist for the agent, not even in the schema" | Kept as drawn: the name of a hidden table is told to the agent, its columns and rows are not. Otherwise the agent cannot act on the refusal (question 8) |
| 12 | The "…" menu has "Copy URL"; the row shows a "read-only" tag; the editor is an accordion | The menu has Duplicate and Delete (`src/ui/picker.rs:983-999`). The list shows no read-only tag. The editor has fieldsets | Out of scope. This work adds the Agents column, the Agents menu entry and the Agents fieldset to the screens as they are |
| 13 | ⌘P: typing "agents" offers "Agents · Bookshop · dev" | Quick open finds the tables and views of the loaded schema and nothing else (`App::quick_open_matches`, `src/app.rs:3971`). It is bound on macOS and Windows only (`C::FindObject`, `src/keymap.rs:917`) | Quick open gains one fixed entry that is no table. Omarchy has `ga` and `:agents` in its place |
| 14 | The tab covers "only that window's connection": an agent is here, or elsewhere ("Also: Cursor on Bookshop · staging") | A session is not on a connection. An agent names the connection in each call, and one session may use several, or none yet | A session is here once it has made a call that names this connection (A.5). One that has named none yet is under "Also:" in every window (settled, question 22) |
| 15 | A notification for a write on a connection whose window is not open "opens that connection's window on its Agents tab" | Opening a connection window connects, and may ask for a password or an SSH host key first (`App::open_workspace`, `src/app.rs:2007`) | The window opens on the Agents tab and connects behind it, as any does. The card does not wait for that: an approved write runs on the server's own session, not the window's |
| 16 | **Stop agents here** stops every agent on this connection | Stopping a session gives the agent back its connection in the second it takes to connect again (A.5, question 11) | It ends every session that has used this connection, and sets the connection's access to Off, so it holds. That is the state the boards already draw, "Off here", with **Allow agents…** as the way back (settled, question 21) |
| 17 | **Turn on** "starts the server at once". The brief's list of tests has "Turn on" change nothing until confirmed | Both cannot hold | The board's reading: the button is the confirmation. It starts the server and changes no connection's access (settled, question 23) |

## A. Architecture

### A.1 Server placement

**Two new places for code, and one rule.**

- `crates/tabletist-db`, a new `agent` module: every check on SQL text and
  every driver change. The read-only guard's own spec puts its layers "all in
  `tabletist-db`, so no caller can skip them"
  (`docs/superpowers/specs/2026-09-30-sql-editor-core-design.md:140`). The
  agent checks follow it.
- `crates/tabletist-mcp`, a new crate with no UI dependency, as
  `tabletist-db` has none: the protocol, sessions, the four tools, the
  approval broker and the log.
- The rule: the types of `tabletist-mcp` hold no `ConnectSpec` and no
  `Secrets`. The crate talks to databases through a `Gateway` trait whose
  methods take a connection's agent-facing name and return rows, schema or a
  refusal. So no tool can serialize a host, a user or a password: its types
  never hold one. F.5 tests it from the outside as well.

**Runtime.** The server is a task on the backend runtime, the two-worker
tokio runtime that `Backend::start_with` builds on the `tabletist-backend`
thread (`src/backend.rs:388-423`). It is started and stopped by a new
`Command::Mcp(..)` and reports through a new `Event::Mcp(..)`, which reaches
the UI through `Outbox::emit` and wakes egui like every other event
(`src/backend.rs:350-360`). Nothing of it runs on the UI thread. The root
crate's tokio features gain `net`, `io-util` and `io-std`, which it has today
only because `tabletist-db` turns two of them on (`Cargo.toml:75`).

**Connections.** There is no pool. A `SessionId` is "one connection tab", and
`run_session` runs that tab's commands strictly one at a time
(`src/backend.rs:43`, `:2036`). Sharing a tab's session would stall the grid
behind an agent's ten-second query, and a connection with no open tab would
have no session at all. So the `Gateway`, implemented in the backend beside
`Worker` (`src/backend.rs:1565`), keeps sessions of its own:

- One read session for each connection an agent has used, opened on first
  use with `Connection::connect_with(&spec, &secrets, &host_keys,
  Access::ReadOnly)` (`lib.rs:120`). Always read-only, whatever the
  connection's own box says: PostgreSQL `default_transaction_read_only = on`
  (`pg.rs:179`), MySQL `SET SESSION TRANSACTION READ ONLY` (`mysql.rs:636`),
  SQLite `SQLITE_OPEN_READ_ONLY` (`sqlite.rs:679`).
- It runs one call at a time. A second agent's call waits, up to the
  connection's timeout, then gets `busy`.
- It is closed after ten idle minutes, when the connection's access goes
  Off, when the connection is edited or deleted, and when the server stops.
- An approved write gets a writable session of its own, opened when the
  person approves and closed when the write ends (B.7).

**The trait, as built** (Phase 1b's first run). `Gateway::connections()`
lists the connections agents may use. `Gateway::open(id, stop)` gives a
`Session` for one call, with `schemas`, `objects`, `describe` and `read`,
and cancelling `stop` ends whatever the session is doing. An agent names a
connection, the tools find that name among `connections()`, and the session
is opened by the connection's id, which is also what the log keeps: the
name is the agent's word for it and can change. One call at a time, `busy`
and the idle close are the implementation's to keep, behind `open`. The first run has the trait, and for its tests a gateway over SQLite
files. The backend's own is the second run's.

**The backend's gateway, as built** (`2026-10-14-mcp-server-1b-2a-gateway.md`). `AppGateway` keeps a session
of its own for each connection an agent has used, opened read-only with the
connection's secrets from the keyring, and hands it to one call at a time.
What its draft's review showed it must also do:

- **Ask the person nothing.** A read of the keyring for an agent never
  brings up a prompt to unlock it: on Linux the Secret Service would ask
  for a collection whose item a read finds locked, so the item is looked
  for first and a locked one is not read (`SecretStore::read_quietly`).
  Windows asks nothing of a read. On macOS the keychain is told to ask
  nothing while it reads for an agent (`SecKeychain::
  disable_user_interaction`, in `security-framework`, which the keychain
  store is built on): a locked keychain, and an item that would ask whether
  Tabletist may read it, answer that they would have to, and are not read.
  The switch is the process's, and the keyring thread, the only one that
  uses the keychain, does one job at a time (`2026-10-14-mcp-server-1b-2b-stdio.md`).
- **Look again after a wait.** A call that waited for the connection, or
  for its connect, takes the connection as it is once it has it: turned
  Off meanwhile, it runs nothing; changed, it is opened anew.
- **Open a lost session again.** A session the database ended answers that
  it is lost, and is closed when the call lets it go, so the next call
  opens it again.
- **Let a stop's cancels land.** The cancels a stop sends are awaited
  before the session is let go, so that none lands on the next call's
  statement, as the backend's own are.
- A connect may take 20 seconds, an SSH tunnel's among it. A session no
  call has used for ten minutes is closed, looked at every minute.
- The first schema is the app's own rule (`model::first_schema`), which the
  workspace's tree and the gateway share.

**Secrets.** The `Worker` owns the `Keyring` (`src/backend.rs:1567`). The
gateway reads `connection/{id}/password` and `connection/{id}/ssh-secret`
(`src/secrets.rs:18-25`) through it, inside the backend, when it opens a
session. The App pushes the saved connections whose agent access is not Off,
with the trusted host keys (`App.host_keys`, `src/app.rs:94`), to the backend
in a `Command::Mcp(Policy(..))` whenever either changes. A connection that is
Off is never sent to the server task, so it cannot be listed, named or
guessed.

**What cannot be opened without a person.** A connection whose password is
asked for each time (`PasswordMode::Ask`, `src/connections.rs:23-32`), an SSH
host whose key is not yet trusted, and a keyring that is locked. An agent
cannot answer those prompts, and a prompt that appears because an agent asked
is a way to phish the person. The call is refused with
`connection_unavailable`, and the Agents tab says why.

### A.2 Stdio

Agents start `tabletist mcp`. The GUI owns the connections. Between them
sits a relay.

**The app listens on a per-user endpoint.**

| Platform | Endpoint | Who may connect |
|---|---|---|
| Linux | A Unix socket, `$XDG_RUNTIME_DIR/tabletist/mcp.sock`, else in the state directory | The directory is 0700 and the socket 0600 |
| macOS | A Unix socket in the state directory (`AppDirs::state`, `src/paths.rs:9-34`). A socket path may have 104 bytes there; a home directory too long for that gets a private directory under `$TMPDIR` | The same |
| Windows | A named pipe, `\\.\pipe\tabletist-mcp-<user>`, through `tokio::net::windows::named_pipe`, with `first_pipe_instance(true)` and `reject_remote_clients(true)` | See the key below |

A pipe's default access list also lets other accounts open it for reading,
and setting another one takes `unsafe` code, which the workspace forbids
(`Cargo.toml`, `[workspace.lints.rust]`). So on every platform the app also
writes a random key to `mcp.key` in the state directory (0600, through
`util::write_atomic`, `src/util.rs:145`), and the relay must send it first.
The key is new on every start of the server.

**The relay.** `tabletist mcp` is a subcommand of the same binary. It
branches off right after `Cli::parse()` and before the logger, the settings
and `eframe::run_native` (`src/entrypoint.rs:81-161`):

1. It never writes to stdout but protocol bytes, never opens the log file
   (the logger creates `tabletist.log` fresh for each run, so a second
   process would wipe the app's), and never shows the startup error dialog
   (`report_startup_error`, `src/entrypoint.rs:65-79`).
2. It connects to the endpoint and sends one line: the relay's version, the
   key, and its working directory, which gives the Agents tab its project name
   ("tabletist-repo") when the client does not offer MCP roots.
3. From then on it copies bytes both ways and understands none of them. The
   app serves MCP on that connection. One connection is one session.
4. When either side closes, it exits.

So the protocol lives in one place, the app, and an old relay keeps working
with a newer app.

**As built** (`2026-10-14-mcp-server-1b-2b-stdio.md`). Agreed on 2026-10-14, where it went otherwise:

- **The key is beside the socket**, `mcp.key` in the endpoint's directory
  (`AppDirs::agents_dir`), not in the state directory: the relay finds both
  in one place, and a test points it at another with `--endpoint`. On
  Linux the directory is `/run/user/<id>/tabletist`, found by the user's
  id and not by `XDG_RUNTIME_DIR`, which an agent may leave out of the
  relay's environment; else the state directory. It is made 0700, and one
  that is not the user's to change is refused. The key is written before
  the socket is made, so that a relay never reads an ended start's.
- **The app answers the relay's line** with one of its own, `"ok"` or a
  refusal with its reason, so that a wrong key, or too many agents, reaches
  the agent as the answer to its first request and not as a closed pipe.
- The relay's line is JSON: the key, its version, and its working
  directory, whose last part names the session's project. Each line may
  have 16 KiB and must come within five seconds.
- A socket that answers is another Tabletist's: the server does not start,
  and says so. One that does not answer was left by a Tabletist that ended,
  and is replaced. The socket and the key are removed when the server
  stops, the socket only while it is the one the server made.
- At most 32 agents at once, counted once they have the key, and at most
  64 connections waiting to say their line. A second Tabletist that finds
  the socket answering does not start, and does not open the log either.

**When the app is not running.** A clear error, and no launch:

- The relay tries the endpoint for two seconds (the app may be starting).
- Then it answers the client's first request with a JSON-RPC error whose
  message is "Tabletist is not running. Open Tabletist, turn on Agents in
  Settings, then reconnect.", writes the same line to stderr, and exits 1.
  The same when the server is switched off.
- It does not start the app. Decision 1 says the server runs only while the
  app is open. A window that opens because an agent asked, holding every
  saved credential, is the wrong default, and on Omarchy it takes a tile on
  the person's workspace. A `--launch` flag can be added later for people
  who want it (question 10, settled: an error, and no flag in Phase 1).

What this costs: an agent started before Tabletist shows the server as
failed until the person reconnects it. A relay that stays alive and waits is
possible, since the tool list is the same for every user, but it has to
speak MCP itself: the protocol would live in two places, and an old relay
would no longer be sure to work with a newer app. Left for after Phase 1.
(A relay that stays up was floated when question 10 was answered. This
paragraph is why it is not in Phase 1.)

**Windows.** The release binary is built for the GUI subsystem
(`src/main.rs:3`). A GUI program still reads and writes the pipes its parent
hands it, which is how agents start servers, but this is only reasoned here,
not run. If it fails, the fix is a second, console binary target for the
relay.

**Settled on 2026-10-13 (question 14): Windows follows by one release.**
Phase 1b builds the endpoint and the relay for Linux and macOS. The pipe,
the relay from a GUI binary and its test on the Windows CI runner are a run
of their own after 1b (G). Until it lands, one function says that the server
is not available on Windows, and nothing of Agents is shown there: no
sidebar entry, no pane in Settings, no command in the keymap or in quick
open. The code is the same on all three and its tests run on all three, so
the look does not fall behind: what waits is the way in.

**As built on Windows** (`2026-10-15-mcp-server-1b-6-windows.md`). Agreed on 2026-10-15, where it went
otherwise:

- **The pipe's name is new each time the server starts**, a random one
  under `\\.\pipe\tabletist-mcp-`, and the server writes it to
  `mcp.pipe` beside `mcp.key` in the state directory, which is the
  user's own. The fixed name per user that this section planned could be
  made first by another account, and the relay would send it the key. The
  relay takes only a name of that form from the file: never another
  computer's pipe. The first instance is made with `first_pipe_instance`
  and every one with `reject_remote_clients`; the key still guards the
  pipe from the other accounts its default access list lets open it.
- **A pipe cannot be closed one way.** On Windows the end of the agent's
  stdin ends the relay and the session, where a socket lets the app still
  answer what it was asked. A client closes stdin once it has its
  answers.
- **The relay from the GUI binary works**, and CI holds it: the Windows
  job also runs the relay's tests on a release build, whose binary a test
  reads to be sure it is a GUI program. No console target was needed.
- The command for Claude Code names the installed program by its whole
  path, in double quotes where it has a space
  (`%LOCALAPPDATA%\Programs\Tabletist\tabletist.exe`), as the Command
  Prompt and PowerShell read it. Cursor's entry has the path as JSON
  writes it.
- A new agent flashes the taskbar button, as the Dock bounces on macOS.

### A.3 HTTP

**Dropped on 2026-10-15, for now** (G, 1d): nothing of this section is
built or planned. It is kept as the design.

`http://127.0.0.1:7433/mcp`, the Streamable HTTP transport.

**Listening.** `mcp.listen` must parse as a loopback IP address with a port.
`0.0.0.0`, `::`, a LAN address and a host name are refused with a notice,
and the listener does not start. Stdio is not affected by a port that is
taken.

**Who the client is.** There is no anonymous access. A client sends
`Authorization: Bearer <token>`.

- The person makes a token in Settings › Agents, under "Other": they name
  the agent, and the token is shown once, inside the configuration to paste.
- The app keeps the token's SHA-256, its name and the day it was made in
  `agents.json` in the state directory (0600). The token itself is not kept.
  It is not in `settings.toml`, which people edit, sync and paste into bug
  reports.
- The name the person gave is the agent's name in the Agents tab and in the
  log.
  What the client says about itself is shown beside it, never instead.

**Checks on every request, before the body is read:**

| Check | Answer when it fails |
|---|---|
| `Host` is `127.0.0.1:<port>` or `localhost:<port>` | 403 |
| No `Origin` header. No browser client is supported, so any `Origin` is refused, not matched against a list | 403 |
| The path is `/mcp`, the method `POST` (or `GET`/`DELETE` for the older protocol versions) | 404, 405 |
| `Content-Type: application/json` | 415 |
| The token is known. Compared as digests, in constant time | 401 with `WWW-Authenticate: Bearer` |
| The body is at most 1 MB | 413 |

No CORS header is ever sent and `OPTIONS` is not answered, so a preflight
fails. The `Host` check and the refused `Origin` are what stop DNS rebinding:
a page that has rebound its name to 127.0.0.1 still sends its own `Host` and
an `Origin`, and has no token. rmcp has both checks
(`StreamableHttpServerConfig::allowed_hosts`, `enforce_origin_validation`);
the plan keeps them in a layer of its own in front of rmcp as well, so an
upgrade of the library cannot remove them unseen.

**"Removing it here disconnects it at once."** Each token has a cancellation
token that is the parent of every request made with it. Removing an agent in
Settings deletes its digest and cancels that parent:

- a query in flight is stopped as Cancel stops one (A.5);
- a write waiting for approval is withdrawn and its card closes;
- an open event stream ends;
- the next request gets 401.

In the 2026-07-28 protocol every request stands alone and carries the token,
so there is no session to outlive the removal. Under the older versions the
session ids made with the token are dropped with it.

**Stdio agents cannot be removed in that sense.** A stdio agent has no
token: any program the person runs can start `tabletist mcp`. Its session
can be stopped, and it can start another, which is announced. The board has
one hint for both. The plan lists connected stdio agents with **Stop**, and
HTTP agents with **Remove**, and says which is which (question 17).

### A.4 MCP library

**Recommendation: `rmcp`, the official Rust SDK, behind a module of our own.**

| | `rmcp` 3.5.1 | Writing the protocol layer |
|---|---|---|
| Protocol | Serves 2024-11-05, 2025-03-26, 2025-06-18, 2025-11-25 and 2026-07-28 (`ProtocolVersion::KNOWN_VERSIONS`) | One version, the one we write for. Clients on another fail |
| The 2026-07-28 change | It removed the `initialize` handshake and protocol sessions: each request carries the client's name and capabilities in `_meta`. rmcp handles both lifecycles | We would write both, and the next change |
| Stdio | Anything that is `AsyncRead + AsyncWrite` (`transport-async-rw`), so a Unix socket or a named pipe works as it stands | Newline-delimited JSON-RPC: a few hundred lines |
| HTTP | `StreamableHttpService`, a tower service, with `Host` and `Origin` checks, a body cap and event streams. Needs an HTTP server under it | hyper, plus the event stream, by hand |
| Tool schemas | `Tool { name, title, description, input_schema, output_schema, annotations }`, each schema a JSON object. Written by hand with `serde_json::json!` for four tools. No macro (`macros` off) | The same JSON |
| Cancellation | `RequestContext::ct`, a token cancelled when `notifications/cancelled` arrives | To write |
| Progress | `peer.notify_progress(..)` when the request carried a progress token | To write |
| Cost | New crates. Stdio: `rmcp`, `schemars`, `schemars_derive`, `serde_derive_internals`, `pastey`, `dyn-clone`, `ref-cast` and `ref-cast-impl` (counted in Phase 1b's first run), and `tokio-stream` for the tests; `tracing`, `chrono`, `tokio-util`, `futures`, `thiserror` and `uuid` are in `Cargo.lock` already. HTTP: `hyper`, `hyper-util`, `http`, `http-body`, `http-body-util`, `httparse`, `tower-service`, `sse-stream` and `tokio-stream`, none of which is in the tree today | No new crate for stdio. hyper and friends for HTTP all the same |
| Risk | A release every week or two (3.4.0 on 2026-09-15, 3.5.1 on 2026-10-05). Apache-2.0. Needs Rust 1.88; the workspace is on 1.98 | Ours to keep up with a protocol that changed three times in a year |

Why rmcp: the protocol moves faster than this app should have to follow, and
the two things that are easy to get wrong, the lifecycle and the HTTP
transport, are the ones the SDK tracks. Why behind a module: only
`tabletist_mcp::protocol` names rmcp types. The tools, the policy and the
tests speak our own types, so the SDK can be replaced by the few hundred
lines if it ever has to be. Features: `default-features = false`, `server`
and `transport-async-rw` in Phase 1b, `transport-streamable-http-server` in
Phase 1d. The `auth` (OAuth), `client` and `macros` features stay off; the
`client` feature is a dev-dependency for the test harness.

In Phase 1b's first run the feature is `server` alone, which turns
`transport-async-rw` on itself. The tests also build `tracing` with its
`log` feature, as the app's build does, so that they see what the SDK
writes to the app's log (C.1).

**Tool annotations.** `readOnlyHint: true` on `list_connections`,
`describe_schema` and `run_query`. `destructiveHint: true` on
`propose_write`. They are hints for the client's own prompts. Nothing here
relies on them.

**What the tools take and answer** was left to Phase 1b's first run. Its
plan has the table (`2026-10-13-mcp-server-1b-1-server.md`, "What this run settles"). In
short:

- `list_connections()`: for each connection its `name`, `engine`,
  `parameters` (`$1, $2` or `?`), `access`, `rows` and `timeout_s`. The
  engine and the placeholder are not on the board: an agent cannot write a
  statement without them.
- `describe_schema(connection, tables?)`: without `tables`, the names of the
  tables and views of the connection's own schema. With them (at most 20),
  what each is made of.
- `run_query(connection, sql, params?)`: `columns`, `rows` as arrays,
  `row_count`, `truncated`, `truncated_by`, `took_ms`.
- An answer is its JSON as text, once, with no structured content beside
  it: a client that hands a model both would hand it the rows twice, and
  the transport would carry three times the size. A refusal has both, as
  B.8 says: it is small.
- No tool declares an output schema.
- An argument a tool does not have is refused, not ignored.

**The long wait in `propose_write`.** The call stays open until the person
answers or five minutes pass:

- Every 15 seconds the server sends a progress notification, "Waiting for
  an answer in Tabletist (4:15 left)", when the client gave a progress
  token. On HTTP the event stream also carries a keep-alive.
- A client that stops waiting (its own tool timeout, a cancel, a closed
  connection) withdraws the request. The card closes with "The agent
  stopped waiting. Nothing ran." An approval never runs a write whose
  answer nobody is there to read: the agent would not know it happened and
  might send it again.
- rmcp also has the Tasks extension (a tool call answers with a task the
  client polls) and multi round-trip requests. Neither is used in v1: not
  every client has them. They are the way out if client timeouts turn out
  shorter than five minutes. Phase 2 tries Claude Code and Cursor by hand.

### A.5 Sessions

```rust
struct AgentSession {
    id: SessionId,                 // ours, not the protocol's
    transport: Transport,          // Stdio | Http { token }
    agent: String,                 // the token's name, else what the client calls itself
    project: Option<String>,       // the client's first MCP root, else the relay's directory
    connected_at: SystemTime,
    last_call_at: SystemTime,
    calls: u64,
    connections: Vec<Used>,        // each connection it has named, with its calls and its last one
    state: State,                  // Idle | Running(call) | WaitingForYou(request)
    stop: CancellationToken,       // parent of every call in flight
}
```

- A stdio session is one connection from the relay: it begins when the
  relay connects and ends when either side closes.
- An HTTP session is a token in use. It leaves the Agents tab after thirty
  minutes without a call. Nothing is enforced through it.
- The agent's name is whatever the client says, in `initialize` or in a
  request's `_meta`. It is a label, not an identity. Only an HTTP token's
  name is something the person chose.
- Built in Phase 1b's first run: the id (a UUID, since the log keeps it
  from one start of the app to the next), the transport, the label (the
  client's title, else its name, without control characters, at most 64
  characters), the project, when it connected, its count of calls, and the
  stop. The project is given by whoever hands the transport over. Asking the
  client for its roots is left for later. `last_call_at` and `connections`
  came with the Agents tab (`2026-10-14-mcp-server-1b-3a-agents-tab.md`), which needs them to say who is here;
  `state` comes with the feed, in the fourth run. The server tells the
  backend when a session opens or ends, names its agent, or calls
  (`Server::changes`), and the backend tells the app
  (`AgentsEvent::Sessions`).

**Where a session is** (added on 2026-10-13, for the Agents tab). The tab
covers one connection, and a session belongs to none: the agent names a
connection in each call.

- A session is **here**, for a connection, from its first call that names
  that connection until it ends. `list_connections` names none.
- A session that has used two connections is here in both windows. Each
  shows its calls on that connection, and "Stop session" in either stops
  all of it.
- A session that has named no connection yet is nowhere in particular. It
  is listed under "Also:" in every Agents tab, as "Claude Code · no
  connection yet" (settled, question 22).
- The sidebar's "● Claude Code" and the status bar's "1 agent here" count
  the sessions that are here.

**Stopping.** There are three, from the narrowest:

| | Where | What it stops |
|---|---|---|
| **Stop session** | A session's line in the Agents tab | That session, wherever it works |
| **Stop agents here** | The Agents tab's header | Every session that is here, for this connection, and the connection's access. Added on 2026-10-13 |
| **Stop all agents** | Settings › Agents, the server's switch | The server (below) |

"Stop session" cancels the session's token.

- A query in flight is stopped the way the backend stops a script today:
  the `StopFlag` is set and the session's cancel is sent again and again
  until the driver says it is finishing (`keep_sending`,
  `src/backend.rs:2376`). The future of `run_script` is awaited to its end,
  never dropped, as its spec requires. The transaction is rolled back. That
  code moves out of the backend's private functions into something both can
  call.
- A write waiting for approval is withdrawn. A write that is already running
  is a `ScriptMode::Write` run: it is stopped the same way and rolled back,
  unless its `COMMIT` has been sent, in which case it ends as the commit
  ends (`stop.finish()`, the editor-writes spec, step 4).
- Each call answers `server_stopped`, then the connection is closed (stdio)
  or the stream ends (HTTP).
- The agent's read session on the database is kept for the next agent,
  unless a cleanup failed, in which case it is closed, as the backend does.
- A call the person stopped answers `server_stopped` whatever it had got
  to, an answer too, and whatever else ended it besides: its timer, or the
  SDK's own cancel when the session closes. The person's stop wins from
  wherever it was given, the server's as a session's.
- **A client that closes its end** (found in the first run's draft). The SDK
  goes on answering for five seconds after a client closed its writing end,
  which is right for one that sent its calls and waits, and it cancels
  nothing. When it has let the transport go, the server ends what still
  runs. A write that fails ends the session's calls at once: nobody will
  read an answer.
- **A client that reads nothing.** A session that was stopped is closed all
  the same. Its calls are given five seconds to answer, and three seconds
  after that the server takes the transport from the SDK, which would wait
  on it for good.
- **What a session may take.** A message of more than a megabyte ends the
  session, since the SDK reads a line into memory however long it is. A
  session runs sixteen calls at once, and the next is refused (`busy`).

"Stop agents here" stops each session that is here, as "Stop session" does,
and withdraws every write waiting on this connection. A session that also
works on another connection ends there too: a session cannot be stopped in
part.

**It holds** (settled on 2026-10-13, question 21). A stop that an agent
undoes by connecting again a second later is no stop, which is what question
11 says of "Stop all agents". So "Stop agents here" also sets the
connection's access to Off, and saves it:

- Off is the state the boards already draw for a connection agents cannot
  use. The sidebar entry reads "Off here", the tab shows that empty state,
  and no state is new.
- The level it had is kept beside it (C.1), and **Allow agents…** opens the
  editor with that level chosen. Nothing changes until Save there, as
  always.
- The tab says what happened where it says what was stopped: "Stopped 2
  agents. Access is now Off for Bookshop · dev."
- It is the one control in the tab that changes a saved setting. "Stop all
  agents" does the same to the server's switch.

"Stop all agents" switches the server off, the same as the toggle in
Settings: every session is stopped, the endpoint and the port close, and
nothing reconnects until the person turns it on. Stopping sessions and
leaving the server up would give agents back their connection in the second
it takes to reconnect (question 11). The new boards draw it nowhere but as
that toggle.

## B. Enforcement

A call passes these in order. Nothing reaches a database until the first
five say yes.

1. The connection: known, not Off, and allowed this tool.
2. One statement, by our tokenizer. The existing refusal list.
3. The parser: the statement is one of the kinds the tool takes, and nothing
   in it is on the forbidden list.
4. Names: no hidden table, no masked column out of place, no forbidden
   function.
5. The engine's own reading, before the statement runs: its prepare step,
   and on PostgreSQL and MySQL its plan.
6. The run: a read-only transaction with a timeout and a row cap, rolled
   back.
7. The answer: masked values replaced, the size capped, the call logged.

### B.1 Read-only at the database, per driver

All three exist. `run_query` uses them as they are.

| | What the code does today | Where |
|---|---|---|
| PostgreSQL | The agent's session is opened with `default_transaction_read_only = on`. A run sends `BEGIN READ ONLY`, then `SELECT 1` so the snapshot is taken and the transaction can no longer be made read-write, then `SAVEPOINT tabletist_guard`. The savepoint is swapped before every statement. `SHOW transaction_read_only` is checked before the `ROLLBACK`. Advisory locks are released after it | `pg.rs:179`, `pg/script.rs:94-95`, `:266-306` |
| MySQL | The session has `SET SESSION TRANSACTION READ ONLY`. A run sends `START TRANSACTION READ ONLY` and checks the server's status flags, reads `@@session.transaction_read_only` between statements, and ends with `ROLLBACK`, a connection reset and the session statements again | `mysql.rs:636`, `mysql/script.rs:111-165`, `:305-333` |
| SQLite | The file is opened `SQLITE_OPEN_READ_ONLY`, with `PRAGMA query_only = ON`, in `BEGIN DEFERRED … ROLLBACK`. The authorizer denies transaction statements and the pragmas that would undo this | `sqlite.rs:679`, `:546-554`, `:137`, `sqlite/fence.rs:68-87` |

To the brief's question for SQLite, a separate read-only connection or
`PRAGMA query_only`: both, and the first is the one that counts. The agent's
session is its own file handle, opened read-only.

**What is missing: the statement timeout.** No driver sets one. The only
timeout is the backend's timer on `Command::RunSql`, which cancels from the
client (`src/backend.rs:2417-2427`). The plan adds the server's own, set by
the driver after the transaction starts, with the timer kept as the second
layer at the same value plus one second:

| Driver | Statement | Notes |
|---|---|---|
| PostgreSQL | `SET LOCAL statement_timeout = <ms>` | Local to the transaction. The agent cannot change it: every `SET` and `set_config` is refused |
| MySQL | `SET SESSION max_execution_time = <ms>` | Applies to `SELECT`, which is all a read runs. MariaDB: `SET SESSION max_statement_time = <seconds>`. The cleanup's reset puts it back |
| SQLite | A deadline in the progress handler the script run already installs | `sqlite.rs:192`. In process, so it is exact |

A statement the server stops is `timeout`, and its transaction is rolled
back like any other.

**Corrected on 2026-10-13, from the draft of Phase 1a's second run**
(`2026-10-13-mcp-server-1a-2-agent-read.md`). Two things:

- This table first set PostgreSQL's `lock_timeout` beside the statement
  timeout. The statement's time includes its wait for a lock, and
  PostgreSQL's manual calls a lock timeout of the same size pointless.
- On MySQL a statement can lift the session's limits from inside: the
  optimizer hints `/*+ MAX_EXECUTION_TIME(n) */` and
  `/*+ SET_VAR(max_execution_time = n) */` replace the time limit, and
  `/*+ SET_VAR(sql_select_limit = n) */` the row limit of B.5. Seen on MySQL
  8.4. The checker refuses a hint comment on MySQL (`not_allowed`).

### B.2 Parsing

**Parser: `sqlparser` 0.63** (`sqlparser-rs`, Apache), with
`PostgreSqlDialect`, `MySqlDialect` and `SQLiteDialect`, chosen by a `match`
on `Dialect` that names every variant, as `AGENTS.md` requires. It gives a
syntax tree, its tokens, and source positions. With the `std` feature alone it
is one new crate. The `visitor` feature, which walks the tree
(`visit_relations`, `visit_expressions`), adds `sqlparser_derive`.

**Corrected on 2026-10-13, from the draft of Phase 1a.** This section first
said the parser's own depth limit refuses a statement nested too deep to
walk. It does not. The limit bounds parentheses. A chain such as `1+1+1+…`
is parsed in a loop into a tree as deep as the chain is long, about 50,000
levels under the size cap. Measured in a debug build on Linux: dropping that
tree needs 4 to 8 MB of stack, and walking it with the derived visitor needs
about 40 kB for each level, so 100 levels overflow the 2 MB a thread of the
backend's runtime has. An overflow ends the process. The default
`recursive-protection` feature guards the parser and `Display`, which are not
what overflows. What follows from it:

- Phase 1a parses on a thread with a stack of its own, and reads what it
  must find at any depth (a write, a lock, `INTO`, a function, a
  placeholder) off the tokens, never off the tree. See
  `2026-10-13-mcp-server-1a-1-read-checker.md`.
- Phase 1c needs the names of the tables a statement reads, which only the
  tree has (B.3, point 1). It must bound the depth before it walks: a cap on
  the number of tokens is the obvious bound, since a level costs at least
  one. That design is 1c's.
  As built (`2026-10-15-mcp-server-1c-1-checks.md`), the tree is not walked: 1c reads the names off the
  tokens too, which can miss one, and what the engine says the statement
  reads is the fence (B.3).

Why not the alternatives:

- **The tokenizer we have** cannot say which tables a statement reads. Its
  own completion code calls its scan "approximate" (`complete.rs:1-3`).
- **`pg_query`**, PostgreSQL's real parser, is exact for one engine only,
  and is a C library to build on three platforms.
- **SQLite's own parser** is used where it can be: its authorizer reports
  every table and column a statement reads after SQLite's parse (B.3).

**The parser allows; it does not forbid.** sqlparser reads syntax, not
meaning, and accepts text an engine would refuse. So the rule is a list of
what is let through. Anything else is refused, and so is text the parser
cannot read (`not_parsed`). An agent can rewrite a query. It cannot be given
the benefit of the doubt.

| Step | Rule | Refusal |
|---|---|---|
| Size | At most 100 kB of SQL and 100 parameters | `too_large` |
| One statement | `sql::statements` finds exactly one, and so does the parser. The driver's prepare step refuses a second one hidden from both, with the engine's own lexer (`pg/script.rs:403`, `mysql/script.rs:442`, `sqlite.rs:400-404`) | `too_many_statements` |
| The refusal list | `sql::refusal` as it stands: transaction control, `SET` of guarded settings, `PREPARE`, `COPY`, `CALL`, MySQL's executable comments, PostgreSQL's `U&"…"` names and the rest (`sql.rs:684`) | `not_allowed` |
| `run_query` | The tree is one `Statement::Query`: `SELECT`, `WITH … SELECT`, `VALUES`, and set operations of those. Also `EXPLAIN` of such a query, without `ANALYZE` | `not_read_only` for an `INSERT`, `UPDATE`, `DELETE` or `MERGE`, naming `propose_write`. `not_allowed` for the rest |
| Inside a read | No `INTO` of any kind, no `FOR UPDATE` or `FOR SHARE`, no CTE whose body changes data | `not_read_only` |
| `propose_write` | One `Statement::Insert`, `Update` or `Delete`. No `ON CONFLICT`, `ON DUPLICATE KEY`, `REPLACE`, `INSERT OR …` (upserts are out of scope), no `RETURNING`, no leading `WITH`, no `MERGE`, no `TRUNCATE` | `not_allowed`. `not_a_write` for a `SELECT` |
| Everything else | DDL, `COPY`, `SET`, `RESET`, `SHOW`, `DO`, `CALL`, `PRAGMA`, `ATTACH`, `LOCK`, `LISTEN`, `NOTIFY`, `VACUUM`, `ANALYZE`, `USE`, `KILL`, `GRANT`, `LOAD` and any other variant | `not_allowed` |

`SHOW`, `DESCRIBE` and `PRAGMA` are refused though they read: the message
points at `describe_schema`.

**Functions with side effects.** A read-only transaction stops a function
from writing to a table. It does not stop one from ending another session,
taking a lock, sleeping, reading a server file or running SQL held in a
string. The checker collects every function name, in expressions and in
`FROM`, and applies two lists for each dialect:

- **Always refused** (`function_not_allowed`). PostgreSQL: `set_config`,
  `pg_terminate_backend`, `pg_cancel_backend`, `pg_sleep*`, `pg_advisory_*`,
  `pg_read_*file`, `pg_ls_*`, `pg_stat_file`, `lo_*`, `dblink*`,
  `query_to_xml*`, `table_to_xml*`, `cursor_to_xml*`, `ts_stat`, `ts_rewrite`,
  `crosstab*`, `connectby`, `xpath_table`, `pg_stat_statements_reset`,
  `setseed`, `nextval`, `setval`,
  `pg_notify`, `pg_reload_conf`, `pg_logical_*`, `pg_replication_*`. MySQL:
  `SLEEP`, `BENCHMARK`, `GET_LOCK`, `RELEASE_LOCK`, `RELEASE_ALL_LOCKS`,
  `LOAD_FILE`, `SOURCE_POS_WAIT`, `MASTER_POS_WAIT`, MariaDB's `NEXTVAL` and
  `SETVAL`, `LAST_INSERT_ID`, and assignment to a user variable. SQLite: `load_extension`, and every table-valued `pragma_*`.
- **Functions the database's users wrote**, which the checker cannot read:
  allowed on a connection that hides and masks nothing, refused on one that
  does. Telling them apart needs the names of the database's routines, which
  the catalog does not read today.

A list of bad functions is never complete: an extension adds more. What it
misses still runs in the read-only transaction, under the timeout, as the
connection's user. On SQLite the authorizer sees every function call after
SQLite's own parse, so there the list does not depend on our parser.

**Corrected on 2026-10-13, from the draft of Phase 1a's third run**
(`2026-10-13-mcp-server-1a-3-sqlite-fence.md`). The checker looked for a
table-valued `pragma_*` as it looks for a function, by a name with a
parenthesis after it. SQLite asks for none: `SELECT * FROM pragma_table_info
WHERE arg = 'customers'` reads the pragma as a table, and passed. The
checker now refuses the name wherever it stands, and SQLite's `Fence::Agent`
refuses every pragma but the one its own full-text tables ask for. A view
is no way around either: every session has `trusted_schema = OFF`, under
which SQLite lets a view neither read a pragma as a table nor call a
function that is not marked harmless.

**When our parser and the engine disagree.** They will, somewhere. Three
things do not depend on the parser: the engine prepares the statement alone
and refuses a second one; the engine is asked what the statement reads
before it runs (B.3); and the transaction is read-only. A read that the
parser took for something else is refused. A write the parser took for a
read is refused by the database.

**Parameters.** No public call takes parameters for SQL text today. Agents
use the engine's own placeholders: `$1` on PostgreSQL, `?` on MySQL and
SQLite. A parameter is a JSON string, number, boolean or null.

| Driver | How | Why |
|---|---|---|
| SQLite | rusqlite binds them | |
| MySQL | The prepared protocol the script run already uses. It refuses a statement with a `?` today because it had no values (`mysql/script.rs:447`) | |
| PostgreSQL | The statement is prepared first, as today, which gives the server's count of its parameters. The values are then bound, each in text format, on the `DECLARE … CURSOR FOR` the script run already sends, by the extended protocol. The rows come as before, as text, through `FETCH` | Rows are read as text through the simple-query protocol, and tokio-postgres's other protocol always asks for binary results (`Some(1)` in its `query.rs`). A cursor keeps the two apart: its parameters are bound where it is declared, and its rows are fetched by a simple query. A value chooses its own format (`ToSql::encode_format`), and in text the server reads it as the parameter's type, as it reads a quoted literal |

**Corrected on 2026-10-13, from the draft of Phase 1a's second run.** This
table first had each `$n` replaced in the text by a quoted literal. That
needed a second reading of the statement to find the `$n` tokens, and a
value became SQL text. Bound on the `DECLARE`, a value is never part of the
statement, on any of the three. An `EXPLAIN` with parameters is no cursor:
it runs by the extended protocol, and its rows (text, JSON or XML) are read
as text.

The log keeps the agent's text and its parameters.

### B.3 Hidden tables

The connection's setting is one of three: **All**, **All except** a list, or
**Only** a list (the board's segmented control). "Only" hides every table
that is not named, the ones made later too. An entry without a schema
matches the table of that name in every schema.

A reference is looked for in four places:

1. **The statement's names.** `visit_relations` gives every name in a table
   position, at any depth: joins, subqueries, CTE bodies, `LATERAL`, set
   operations. (It walks the tree, so the depth is bounded first: see the
   correction in B.2.) Each is resolved against the catalog and compared, without
   regard to case (the strict reading for MySQL, whose rule depends on the
   server). A CTE named like a hidden table is refused, not reasoned about:
   getting its scope wrong would be a leak.
2. **What depends on a hidden table.** Read from the catalog when the
   agent's session opens. A view or a materialized view over a hidden table,
   directly or through another view, is hidden with it, and so is a
   partition or a child table of one.
   - PostgreSQL: `pg_depend` with `pg_rewrite`, and `pg_inherits`.
   - MySQL: `information_schema.VIEW_TABLE_USAGE` (8.0.13 and later). On a
     server without it (an older MySQL, and MariaDB as far as this plan
     could tell without a server to ask), every view is unavailable to
     agents on a connection that hides something.
   - SQLite: each view is prepared under the authorizer, which reports the
     tables it reads.
3. **The engine, before the statement runs.**
   - SQLite: a new `Fence::Agent` beside `Fence::Script` and `Fence::Filter`
     (`sqlite/fence.rs:23-31`). The authorizer is asked for every table and
     column SQLite reads, after SQLite's own parse, with the view or trigger
     the read comes from (`Action { first, second, database, source }`,
     `crates/tabletist-sqlite-ffi/src/authorizer.rs`). A read of a hidden
     table is denied whatever the text looked like. This one is exact.
   - PostgreSQL: `EXPLAIN (VERBOSE, FORMAT JSON)` of the statement in the
     same read-only transaction, which names each relation with its schema. Every relation the plan scans is resolved to its
     OID and compared with the hidden ones. A view is already expanded in
     the plan, a partition is listed, and `TABLE customers` is a scan like
     any other.
   - MySQL: `EXPLAIN FORMAT=JSON`, the same way by name. MariaDB's shape
     differs. Both are read. A plan that cannot be read refuses the query.
   - The plan is asked for only on connections that hide or mask something.
     It costs one round trip.
4. **Functions**, by the rule in B.2.

**System catalogs.** `information_schema`, `pg_catalog`, `mysql`,
`performance_schema`, `sys`, `sqlite_schema` and `sqlite_master` describe
every table, and `pg_stats` holds sample values of every column. On a
connection that hides or masks anything they are refused (`system_catalog`),
and the message points at `describe_schema`. On a connection that hides
nothing they are allowed (question 13).

**What an app-side check cannot guarantee.** These are stated in the UI and
the documentation:

- A function, a procedure or a trigger in the database that reads a hidden
  table. Its body is not in the plan. The function rule refuses what the
  person's database defines, on connections that hide something, but an
  extension's function or a `SECURITY DEFINER` one that the lists do not
  know is run.
- A view made after the catalog was read, on MySQL, for up to 30 seconds
  (C.3). PostgreSQL and SQLite have no such gap.
- A foreign table or a link to another server that shows the same data
  under another name.
- That the table exists. The refusal names it (row 11 of section 2).
- What an error message says. A message that quotes a row is passed
  through, on a connection that hides or masks something, without its
  detail line (B.8).

**What the UI says.** Under Tables in the connection editor, below the
board's hint:

> Tabletist checks every query before it runs. A view or a function in the
> database that reads a hidden table can still reach it. For strict
> isolation, use a database user without access.

The same sentence, with the list above, goes in the documentation page for
agents.

**As built in 1c's first run** (`2026-10-15-mcp-server-1c-1-checks.md`): the checks, in
`crates/tabletist-db/src/agent/hidden.rs` and a `hidden.rs` for each
server, behind a `tables` argument of `Connection::run_agent_read`. The app
passes "All" until the next run reads the setting. Agreed on 2026-10-15,
where it went otherwise:

- **The statement's names are read off its tokens, not its tree** (B.2
  keeps the tree from being walked). A name after `FROM`, a `JOIN`, a comma
  of a `FROM`'s list, `LATERAL` or `ONLY`, or in parentheses where a table
  goes, at any depth, is read, and so is what a `WITH` defines; any word
  with a parenthesis after it is a call, a keyword too (PostgreSQL lets a
  function be called `last`). The reading errs toward finding, and
  it is not the fence: what it misses, point 3 sees. It is what names the
  view the agent asked for, and what sees a hidden view whose tables are
  not.
- **The catalog is read for each call, in the call's own read-only
  transaction**, not when the session opens: the check sees the database
  as the statement will. It costs two or three queries more on PostgreSQL
  and MySQL, and nothing on a connection that hides nothing. So MySQL has
  no gap of 30 seconds for a view made later.
- **SQLite lists no dependencies.** The authorizer reports each table a
  view reads, with the view (and a table only `count(*)` or `EXISTS`
  reads, and `sqlite_schema`), so a view is refused by what it reads, and
  the refusal names it. The catalog is `pragma_table_list`, for the names
  a refusal gives. A shadow table (FTS5's, R*Tree's) goes with its
  virtual table: hidden with it, and read with it where an "Only" list
  names it alone.
- **PostgreSQL compares by schema and name**, as the plan writes them,
  not by OID. The plan names each relation it scans with its schema: a
  partition (not its parent), a child table under its parent's scan, a
  materialized view as itself, whose own dependencies the catalog lists.
  A partition is read with its table: an "Only" list that names the table
  need not name its partitions, the ones made later included, and one the
  person hides by its name stays hidden.
  A function in a `FROM` is a `Function Scan` of the plan, and one in an
  expression is found by the reading of the text. The users' functions
  (`pg_proc` outside the system's schemas, and not an extension's) are
  refused while anything is hidden. An `EXPLAIN` of the agent's own is
  checked as the query it explains.
- **A function the statement does not name is refused too** (found by the
  pull request's review). A view whose rules call a function the users
  wrote, and a table whose row policy does, is hidden while anything is:
  no plan shows what the function reads. On MySQL so is a view
  `VIEW_ROUTINE_USAGE` lists, or every view where the server has no such
  table, and a call the rewritten query writes `` `db`.`f`() `` is checked
  as a function. On PostgreSQL a function the users wrote for an operator,
  a cast, an operator class, a type's input and output, a range type or a
  domain's constraint runs wherever a value meets it, so while one is there every statement is refused on a
  connection that hides anything. PostgreSQL also calls a function as a
  column of a row (`b.f` is `f(b)`, by every lookup that would find
  `f(b)`), so a name after a dot that is a function the users wrote is
  refused.
  A view or a policy that calls a function the agent may not call (B.2),
  one of the system's or an extension's that runs SQL held in a string
  (`query_to_xml`, `crosstab`, `dblink`), is hidden while anything is too:
  its definition, as the server writes it back, is read for such a call,
  as PostgreSQL keeps no dependency on its own functions. Such a function
  in the plan's `Function Scan` is refused wherever it came from.
- **MySQL's plan is read twice.** Its JSON names a table by the
  statement's alias where it has one, so a name there is checked only
  where the catalog lists it. The note `SHOW WARNINGS` gives after it
  (code 1003) is the query as the server rewrote it, every table written
  `` `db`.`table` `` and views expanded: each of those after a `from` or a
  `join`, or in a schema of the database, is checked (a derived table's
  column is written with two parts too). Where the server gives no note
  (MariaDB may not), a name in the JSON that the text does not give in a
  table's place, an alias, refuses the query: nothing says what it stands
  for. The reading takes any word in a table's place for a name, as MySQL
  lets a table be called `last` or `view` without quotes. A plan
  that cannot be read refuses the query (`not_parsed`). The catalog's own
  queries carry a `LIMIT`, since the run sets `sql_select_limit` for the
  agent's rows.
- **On MySQL the plan is not free of the statement.** `EXPLAIN` runs some
  subqueries while it plans (a constant one, an `EXISTS` it answers at
  once), and its note leaves out what it answered. A hidden table that the
  reading of the text and the catalog both miss, and that such a subquery
  reads, is read once while planning; an error it raises there can carry
  a value. Every name the text gives in a table's place is refused before
  the plan is asked for, so this takes a name the reading misses.
- **The function of the Bookshop is on PostgreSQL alone**
  (`customer_count()`, the view `customer_tally` that calls it, and
  `book_reach(books)`, F), with a partitioned table, `stock_moves`.
  MySQL lets only a user with `SUPER` make one
  while it logs its changes, so its fixture has none, and MySQL's function
  rule is tested off the server only.
- **Not refused**: the system's functions that say something of a table
  without reading a row of it, given its name: PostgreSQL's
  `pg_relation_size`, `pg_table_size`, `has_table_privilege`,
  `to_regclass` and their like. They say that it exists and how large it
  is, which the refusal says too (question 8).
- Names are compared without regard to case everywhere, which on
  PostgreSQL refuses a quoted name that differs from a hidden one only by
  case. A statement's name without a schema is taken for a hidden one of
  any schema.

### B.4 Masked columns

A masked column keeps its name and its type. Its values reach the agent as
`•••`. The difficulty is everything that is not printing the value:
`WHERE email LIKE 'a%'` answers one bit at a time.

**Three rules compared.**

| | A. Bare projections only | B. Substitute the table | C. In the database |
|---|---|---|---|
| How | A masked column may only be selected as itself. Any other use is refused. Values are replaced after the fetch | Every reference to the table is rewritten to a subquery that selects `NULL` for the masked columns | Column privileges, a view, or an extension, set up by the person for the connection's user |
| `WHERE email LIKE 'a%'` | Refused, with a sentence the agent can act on | Runs, compares `NULL`, and returns no rows. The agent gets a wrong answer and no reason | The database's error |
| What can leak | A use the checker fails to see | Nothing through the table itself: the value never leaves it | Nothing |
| What it costs | A name resolver over the tree. Every miss is a leak | The SQL that runs is not the SQL that was sent or logged. The rewrite must find every reference: a miss is a leak here too | Nothing here. Not something Tabletist can turn on |
| Through a view | Not seen, so views over a masked table are refused (below) | Not seen, the same | Covered |

**Recommendation: A, written as an allow rule, with the engine's own
metadata for the replacing, and the engine's own masking on SQLite.** The
reasons:

- The board's `masked_column_used` is rule A: a refusal the agent can act
  on. A wrong answer with no reason is worse for an agent than a refusal.
- The SQL that runs is the SQL in the log. "Open in SQL editor" shows the
  person the query the agent ran.
- C stays the answer for strict isolation, and the UI says so.

If the test matrix of F.2 finds a leak in A that cannot be closed, B is
added under it, so that a use the checker misses reads `NULL`. That is a
decision for Phase 3a, with the matrix in hand.

**The rule.** It applies to any statement that reads a table with a masked
column.

1. A masked column may appear only as a bare column (`email`, `o.email`,
   with or without `AS`), or through `*` or `o.*`, in the select list of the
   outermost `SELECT`.
2. That outermost query must be one plain `SELECT`. With a masked column in
   its list, it may have no `DISTINCT`, no `GROUP BY`, no `HAVING`, no window
   function, no set operation, and no `ORDER BY` that names the column, its
   alias or its position.
3. Anywhere else, a masked column is refused: `WHERE`, `JOIN … ON`, `USING`,
   `GROUP BY`, `HAVING`, `ORDER BY`, a function's arguments, `CASE`, a cast,
   an operator, a subquery, a CTE, a window, a `FILTER`. A `SELECT *` over
   the table inside a subquery or a CTE is refused too, since the outer
   query could then filter on the column under another name.
4. A `NATURAL JOIN` with the table is refused. So is the table's name or
   alias used as a value (`SELECT o FROM orders o`, `to_jsonb(o)`,
   `row_to_json(o.*)`), which carries every column.
5. Allow, do not forbid: every name in the statement must resolve to a
   column that is not masked, to an alias, or to a CTE's column. A name the
   checker cannot place is refused. This is what makes a new kind of
   expression in a later sqlparser a refusal and not a leak.
6. A view that reads a masked column is refused, with a message that names
   the view. Which views do is known exactly on SQLite (the authorizer) and
   on PostgreSQL (`pg_depend` records columns). On MySQL any view over a
   table with a masked column is refused.
7. In `propose_write`, a masked column may be assigned a literal or a
   parameter. It may not be read: not in `WHERE`, not on the right of a
   `SET`, not in an `INSERT … SELECT`.

**Replacing the values.** Which result columns are masked is not taken from
the parser alone. Each engine says where a result column comes from:

| Driver | Source |
|---|---|
| PostgreSQL | The table and column of each result column, from the prepare step that already runs (`Column::table_oid`, `column_id` in tokio-postgres) |
| MySQL | `org_table` and `org_name` of each column definition (mysql_common) |
| SQLite | `sqlite3_column_origin_name`, which needs SQLite's column-metadata build option turned on in the bundled build, read through `tabletist-sqlite-ffi` |

A result column is replaced when either the parser or the engine says it is
a masked column. Every value becomes `•••`, a `NULL` too, since a `NULL`
says something. The column is marked `masked: true` in the answer.

**SQLite can do more.** Its authorizer may answer a column read with
"ignore", and SQLite then reads `NULL` in its place, in every position and
through every view. `tabletist-sqlite-ffi` left that answer out on purpose
("nothing here gives it"). Phase 3a adds it, for masked columns under
`Fence::Agent`. On SQLite a miss by the checker then reads `NULL`.

**What remains under A.** Which rows exist and how many. A function or a
view the catalog does not tie to the column. And everything in B.3's list.

### B.5 Row limits

**The database already stops early.** `run_script` keeps at most `limit`
rows and asks each engine for `limit + 1`:

| Driver | How | Where |
|---|---|---|
| PostgreSQL | `DECLARE tabletist_sql NO SCROLL CURSOR FOR <text>`, then `FETCH <limit + 1>` | `pg/script.rs:71`, `:329-340` |
| MySQL | `SET SESSION sql_select_limit = <limit + 1>` in the run's transaction | `mysql/script.rs:163-164` |
| SQLite | Stepping stops after `limit + 1` rows | `sqlite.rs:416-425` |

So the agent's SQL is not wrapped in a `LIMIT`. `truncated` on the result
says that a row past the limit existed.

**"N of M".** When the answer is not cut, M is N. When it is cut, M is not
cheap in general: it is the query run again as a count. The plan:

- The count `SELECT count(*) FROM (<the query>) q` runs in the same
  read-only transaction with a budget of one second, or what is left of the
  statement timeout if that is less.
- If it answers: "500 of 1,284".
- If it does not: "at least 501", and the answer's `total` is null.
- The planner's estimate is never shown as a count.

**What is missing: a cap on size.** A row limit does not bound an answer.
One `jsonb` cell can be megabytes, and it would go into the agent's context.
The cap is measured on the answer as JSON, so it is built where the JSON is
written: Phase 1b, not 1a as this plan first had it (changed on 2026-10-13).

- A cell longer than 2,000 characters is cut and marked, with its full
  length.
- Binary values are described ("binary, 20,480 bytes"), not sent.
- An answer is at most 256 kB as JSON. Past that, rows are dropped from the
  end and the answer says it was cut by size.

**As built** (Phase 1b's first run). A text that is cut is
`{"cut": <its first 2,000 characters>, "characters": <its length>}`, and a
binary value is `{"binary_bytes": <its size>}`: a database's value is never
an object, so neither can be taken for data. The size is 256,000 bytes, of
an answer that is sent once (A.4). The columns count: an answer whose
columns alone pass the size is refused (`too_large`), since two hundred
columns with names of two thousand letters are 400 kB without a row, from a
statement of under three kilobytes.
`describe_schema` is held to the size as well, the names it was asked for
among it. **"N of M" is not built yet**: the
count is a change to the three drivers, a run of its own in `tabletist-db`
before the fourth run of 1b shows the number. Until then an answer that was
cut says so, and not of how many.

**Settings.** Rows per answer: 100, 500 (default), 1,000 or 5,000. Statement
timeout: 5, 10 (default), 30 or 60 seconds. `MAX_SQL_LIMIT` is 10,000 for
the editor (`src/settings.rs:399-401`); agents get half of that at most.

### B.6 Write counts for approval

| | Dry run: execute, read the count, roll back | `EXPLAIN` estimate | `SELECT count(*)` from the statement |
|---|---|---|---|
| Exact | Yes, at that moment | No. It is the planner's guess, and on SQLite there is none | Yes for the rows matched, at that moment |
| Runs the write before approval | Yes | No | No |
| Triggers | Fire. What they do inside the transaction is undone. What they do outside it is not | No | No |
| Sequences | An `INSERT` uses values. They are not given back | No | No |
| Locks | Row locks, and it waits on other people's | None | A reader's |
| Tables without transactions (MySQL MyISAM) | **The dry run is a real write** | No | No |
| SQLite | Takes the write lock (`BEGIN IMMEDIATE`) | | A read |
| Works for every statement | Yes | Yes | Only shapes we can turn into a `SELECT` |
| Gives "Preview the 3 rows" | PostgreSQL and SQLite with `RETURNING`. Not MySQL | No | Yes: the same `SELECT` with the columns |

**Recommendation: the count from the statement, in the agent's read-only
session.** The board's own card says "Nothing runs until you choose", and
the setup board says writes are "proposed, not run". A dry run contradicts
both on every engine, and on a table without transactions it is the write.
The preview link needs the `SELECT` anyway.

How the count is made:

| Statement | Count | The card says |
|---|---|---|
| `UPDATE t SET … WHERE w`, `DELETE FROM t WHERE w`, one table | `SELECT count(*) FROM t WHERE w` | "Matches 3 rows in book_covers" |
| The same without a `WHERE` | `SELECT count(*) FROM t` | "No WHERE: changes all 1,204 rows of book_covers", in the warning colour. `sql::unbounded` finds these (`sql.rs:1133`) |
| `INSERT … VALUES` | The number of rows written out | "Adds 2 rows to book_covers" |
| `INSERT … SELECT q` | `SELECT count(*) FROM (q) s` | "Adds 14 rows to book_covers" |
| Several tables (`UPDATE … FROM`, `DELETE … USING`, MySQL joins), `ORDER BY … LIMIT` | None | "Row count unknown until it runs" |

- The card reads "Counted by a read-only SELECT at 10:43:02".
- "Matches", not "changes": a trigger can skip a row, and MySQL counts only
  rows whose values changed.
- The preview shows the person the first 100 matching rows, unmasked, since
  the person may see them. It is never sent to the agent.
- When a foreign key on the table deletes or updates other rows with it, the
  card says so: "Deleting also removes rows in order_items (foreign key,
  cascade)". The catalog has `on_delete` and `on_update`
  (`ForeignKeyInfo`, `catalog.rs:85-96`).
- **When the count is an estimate** it is not shown as a count. If the count
  query runs out of time the card reads "More than the count could reach in
  10 s. Row count unknown until it runs." An `EXPLAIN` figure is never used.
- **A dry run stays possible, as the person's choice.** On a card with no
  count, a "Count with a dry run" button runs the statement and rolls it
  back, and its text says what that does: "Runs the statement and rolls it
  back. Triggers fire and sequences advance." It is not offered on MySQL
  when the table is not transactional or has a trigger.

### B.7 Approved writes

**Path.** The statement is SQL text, so it takes the SQL editor's write
path, not the grid's: `run_script(&[statement], 0, ScriptMode::Write, stop)`
(`lib.rs:194`), on a session opened `Access::Writable` for this one write, as
the connection's own user.

What that path already gives (the editor-writes spec, "The read-write run"):

- one transaction, started by the run, committed only when the statement
  succeeded, rolled back on an error, a stop or a timeout;
- the refusal list first, the statement prepared alone, the stop flag read
  before `COMMIT`;
- the end told exactly: `Committed`, `RolledBack`, `CommitFailed`, and on
  MySQL a `rollback_warning` when a table without transactions kept its
  change;
- the session put back afterwards, and closed when that fails.

**What has to be added in `tabletist-db`:**

1. Parameters (B.2).
2. A check between the statement and the `COMMIT`: the count the database
   reports against the count the person approved.

**Conflicts.** The grid's conflict handling compares the values a row had
when it was loaded and expects each `UPDATE` to change exactly one row
(`pg/write.rs:157-183`, `:227-243`). An agent's statement has no loaded row
to compare. What can go stale is the count on the card: the person approved
"3 rows", up to five minutes after it was counted. So:

- more rows than approved: `ROLLBACK`. The agent gets `count_changed`, "It
  would now change 5 rows, not the 3 that were approved. Nothing was
  written." The person sees the same. The agent may propose again.
- as many, or fewer: `COMMIT`, and the actual number is reported.
- no count on the card: nothing to compare. The card said so.

This is the editing specs' rule, never write more than was shown, applied
to a statement instead of a cell.

**The production rules.** `Environment::confirms_writes()` is true for
production only (`src/env.rs:64`). A SQL tab there cannot write today
(`NoWrites::Unconfirmed`, `src/model.rs:2840-2848`), because the
confirmation for a script is not built. So:

- v1: a production connection's agent access stops at Read. The editor
  shows "Read + approved writes" disabled there, with the reason.
- When the script confirmation exists, "Approve and run" on production
  opens it after the card: the sheet with the statement on macOS, the
  database's name typed on Omarchy (`confirm_box`,
  `src/ui/write_prompts.rs:857-881`). One approval never covers both.
- "Open read-only" caps access at Read (decision 2):
  `connections::access(read_only, environment)` (`src/connections.rs:70`)
  decides, and the effective agent access is the lower of the two. It is
  worked out when the call arrives and again when the person approves.

**What the agent gets.**

| Outcome | Answer |
|---|---|
| Approved and committed | `approved`, `rows_affected: 3`, `took_ms: 11` |
| Denied | `denied`, with the person's note when they wrote one |
| No answer in five minutes | `timed_out`, "No answer in 5 minutes. Nothing ran." |
| The count grew | `count_changed`, as above |
| The database refused it | `write_failed`, with the database's code and message. Nothing was written |
| The commit failed | `write_failed`, "The database refused the commit. Nothing was written.", with its message |
| The connection was lost mid-write | `write_unknown`, "The connection was lost. Tabletist cannot say whether it was written." The log and the Agents tab say the same. The run's own error is this honest today |
| The server or the session was stopped | `server_stopped` |

### B.8 Refusal codes

A refusal is a tool result, not a protocol error: `isError: true`, a
`structuredContent` of `{ "refused": "<code>", "message": "<sentence>" }`,
and the same sentence as text. The model reads tool results and can act on
them. Protocol errors are for calls that are not well formed. On HTTP every
tool result is a 200.

The codes are stable. The sentences may be reworded.

| Code | Sentence (an example) | When |
|---|---|---|
| `unknown_connection` | No connection named Bookshop · production. | The name matches nothing an agent may see. A connection that is Off answers exactly this |
| `connection_unavailable` | Tabletist could not open Bookshop · staging. Open it in Tabletist to see why. | The password is asked for each time, the SSH host is not trusted, the keyring is locked, the connect failed. The reason is never sent: it can name a host or a user |
| `too_many_statements` | Send one statement per call. | B.2 |
| `not_parsed` | Tabletist could not read this statement, so it was not run. | B.2 |
| `not_read_only` | run_query only runs reads. Use propose_write for DELETE FROM book_covers WHERE id = 9. | A write, `INTO`, `FOR UPDATE`. Until Phase 2 has `propose_write` the sentence ends at what the statement is ("run_query only runs reads, and this is a DELETE."): the pointer is the server's to add, not the check's |
| `not_allowed` | CREATE TABLE is not available to agents. | DDL, `COPY`, `SET`, `DO`, transaction control, upserts, `RETURNING` and the rest |
| `not_a_write` | propose_write takes one INSERT, UPDATE or DELETE. Use run_query for a SELECT. | |
| `write_not_allowed` | Bookshop · staging is read-only for agents. | `propose_write` on a Read connection |
| `table_hidden` | customers is hidden for agents on this connection. | B.3. Also for a view that reads it: "recent_customers reads customers, which is hidden…" |
| `system_catalog` | The system catalogs are not available on this connection. Use describe_schema. | B.3 |
| `masked_column_used` | orders.email is masked. It can be selected, not filtered, joined, sorted or computed on. | B.4 |
| `function_not_allowed` | pg_sleep is not available to agents. | B.2 |
| `invalid_params` | The statement has 2 parameters and 1 was sent. | Count, or a value that is not a string, number, boolean or null, or a whole number past 64 bits signed |
| `too_large` | The statement is larger than 100 kB. | Also more than 20 tables in one `describe_schema`, a table's name of more than 1,000 bytes, and an answer whose columns alone pass its size |
| `timeout` | The query was stopped after 10 s, this connection's limit. | The statement timeout |
| `busy` | Another agent's query is running on this connection. Try again. | A.1. Also a session's seventeenth call at once: "Too many calls at once. Wait for an answer before the next call." |
| `rate_limited` | Too many calls. Try again in 20 s. | E.5 |
| `query_failed` | The database's own error, with its code. | A syntax error, a missing column. Without the detail and hint lines on a connection that hides or masks something. At most 2,000 characters, as every sentence is. The log keeps the code and not the words (C.1) |
| `denied` | The person's note, or "The request was denied." | B.7 |
| `timed_out` | No answer in 5 minutes. Nothing ran. | B.7 |
| `count_changed` | B.7 | |
| `write_failed`, `write_unknown` | B.7 | |
| `cancelled` | The call was cancelled. | The client cancelled |
| `server_stopped` | Tabletist's agent server was stopped. | Stop session, Stop all, quit, removal of a token |

`row_limit` from the brief's list is not a refusal. An answer that was cut
is an answer: `truncated: true`, with `truncated_by` set to `row_limit` or
`size`.

Protocol and HTTP answers, for calls that never reach a tool:

| Case | JSON-RPC | HTTP |
|---|---|---|
| An unknown tool, a missing argument, an argument of the wrong type, an argument the tool does not have | -32602 | 200 |
| An unknown method | -32601 | 200 |
| Not JSON | -32700 | 400 |
| No token, or an unknown one | | 401 |
| `Host` or `Origin` | | 403 |
| A wrong path, method, content type or size | | 404, 405, 415, 413 |
| The server is stopping | | 503 |

A protocol error's sentence never repeats what the agent sent: the SDK
writes each one to the app's log (C.1).

## C. Data and settings

### C.1 Storage

**Changed on 2026-10-13: the connections move to `connections.toml`
first** (`2026-10-13-connections-toml.md`, a run of its own before Phase
1b). What follows was written for `connections.json` and still says what
an agent's settings are. Where they are kept changes:

- Each connection is a `[[connections]]` table, found by its `id`, and its
  agent settings are its `[connections.agents]` table. That is the board's
  TOML but for the key: the boards key a connection by its name, and two
  connections may share one.
- The file's version is 3. Phase 1b's `agents` table makes it 4: the file
  is read strictly, so a Tabletist that does not know the table refuses
  the file and keeps it aside, where the JSON was read past what it did not
  know. Agents are not turned off by a downgrade. They are not read at all.
  **As built** (`2026-10-14-mcp-server-1b-2a-gateway.md`): a file whose connections have no agent settings
  is still written as version 3, so that only a file that holds some is
  refused by an older Tabletist. The table is `access` (`off` or `read`),
  `rows` (1 to 5,000) and `timeout_secs` (1 to 60), and a key left out
  takes what a connection starts with. A duplicated connection starts Off
  for agents, as every new one does.
- When each connection last connected is in the state directory
  (`last_used.json`), so a connect no longer writes the connections.
- "Stop agents here" (A.5) writes `access = "off"` and keeps the level that
  was there as `access_before_stop`, in the same table. Saving the Agents
  section in the editor removes the second key. An `access_before_stop`
  beside an access that is not Off is ignored and removed at the next save.
  The table is written whenever anything in it is not the default, not only
  when access is on (`Agents::is_off` below would drop it): a stop loses
  neither the level nor, from 1c on, the hidden tables.

**Per connection: `connections.json`.** `SavedConnection`
(`src/connections.rs:35-50`) gains one field:

```rust
#[serde(default, skip_serializing_if = "Agents::is_off")]
pub agents: Agents,
```

```json
"agents": {
  "access": "read_and_write",
  "tables": "all_except",
  "hidden": ["customers"],
  "masked": ["orders.email", "orders.phone"],
  "rows": 500,
  "timeout_secs": 10
}
```

- `access` is `off`, `read` or `read_and_write`. `tables` is `all`,
  `all_except` (with `hidden`) or `only` (with `only`).
- The read-side shape `StoredConnection` (`src/connections.rs:103-144`)
  gains the same field with `#[serde(default)]`, and `upgrade()` carries it
  across. That function builds a `SavedConnection` field by field
  (`:233-241`), so a field it forgets is lost on the next save: the test in
  F.4 loads and saves a file and compares.
- `VERSION` stays 2. The field is additive, and an older Tabletist ignores
  it. An older Tabletist that then saves the file drops it, which turns
  agents off. That is the safe direction.
- The file is not watched, and the app rewrites it whole on every connect
  (`mark_used`, `src/app.rs:2811-2823`). An edit made by hand while the app
  runs is overwritten. That is unchanged.

**The server: `settings.toml`, a `[mcp]` table.**

```toml
[mcp]
enabled  = false
listen   = "127.0.0.1:7433"
log_days = 30
notify   = true
```

- Four `Key`s with their paths, notes, literals and `take` arms, and the two
  tests that pin the canonical text (`src/settings.rs:65-101`, `:483-566`,
  `:751-770`, `:826-840`). `Settings::take` accepts integers, booleans and
  strings (`:543-566`), which is all this needs.
- `enabled` is false by default. Nothing listens until the person turns it
  on.
- Every key with a literal is always written, so the table appears in every
  user's file after their next change in the app. That is how the file
  works.
- The file is watched (`watch_settings`, `src/backend.rs:1067-1123`), so an
  edit in an editor reaches `App::apply_settings` (`src/app.rs:2894-2921`),
  which starts or stops the server. See C.2 for the one edit that is not
  applied as it stands.
- **As built** (`2026-10-14-mcp-server-1b-2b-stdio.md`): `enabled` and `log_days` (1 to 3,650, else
  brought into range). `listen` comes with HTTP (1d) and `notify` with the
  notification for a new agent (the fourth run): a key that does nothing
  would be written into every file. A change of `log_days` while the
  server runs is the log's from then on, and stops no session.

**HTTP tokens: `agents.json`** in the state directory, 0600, through
`util::write_atomic`: `{ "id", "name", "sha256", "created" }` for each. Not
in the settings file (A.3).

**The log: `agents.sqlite3`** in the state directory, 0600, through
`rusqlite`, which is in the tree. It is written from the backend runtime
with `spawn_blocking`.

```sql
CREATE TABLE calls (
  id            INTEGER PRIMARY KEY,
  at_ms         INTEGER NOT NULL,   -- when the call arrived, Unix ms
  session       TEXT    NOT NULL,
  agent         TEXT    NOT NULL,
  project       TEXT,
  transport     TEXT    NOT NULL,   -- stdio | http
  connection_id TEXT,               -- null for list_connections
  connection    TEXT,               -- its name as it was then
  tool          TEXT    NOT NULL,
  sql           TEXT,
  params        TEXT,               -- JSON
  reason        TEXT,               -- propose_write's
  request       TEXT,               -- describe_schema's tables
  result        TEXT    NOT NULL,   -- ok | refused | error | timeout | waiting |
                                    -- approved | denied | timed_out | cancelled | stopped
  code          TEXT,
  message       TEXT,               -- the refusal's sentence; a database's error by its code
  note          TEXT,               -- the person's, on a denial
  ran_as        TEXT,               -- "read-only transaction" | "one transaction, committed" | …
  rows          INTEGER,            -- returned to the agent, or affected
  matched       INTEGER,            -- the total when known, or the count on the card
  columns       INTEGER,
  bytes         INTEGER,
  masked        TEXT,               -- JSON array of column names
  took_ms       INTEGER,
  decided_ms    INTEGER             -- how long the person took
);
CREATE INDEX calls_at         ON calls (at_ms);
CREATE INDEX calls_agent      ON calls (agent, at_ms);
CREATE INDEX calls_connection ON calls (connection_id, at_ms);
CREATE INDEX calls_result     ON calls (result, at_ms);
```

- One row for each call. A `propose_write` is written as `waiting` when it
  arrives and updated when it is decided, so it is one entry.
- **Not stored:** rows, values, the preview, tokens, hosts, users.
- **Stored on purpose:** the SQL and the parameters (decision 6). Both can
  hold data: a literal in a `WHERE`, an email in a parameter. The file is
  private to the account and the retention is the control.
- The schema's version is `PRAGMA user_version`. A file that cannot be
  opened is set aside as `agents.sqlite3.bad`, as `util::keep_aside` does
  for JSON (`src/util.rs:230`), and a new one is made.
- Searching the SQL is a `LIKE` over the filtered rows. At a few hundred
  calls a day for thirty days that is fast enough without an index of its
  own.
- **Retention.** When the server starts and every six hours: `DELETE FROM
  calls WHERE at_ms < ?`, then an incremental vacuum. `log_days` is 7, 30
  (default), 90 or 365. There is no "never log": decision 6 says every call
  is logged.
- **Export.** The rows that pass the current filters, as CSV or as JSON
  Lines, through the save dialog the app already has (`rfd`), written on the
  backend runtime.
- **As built** (Phase 1b's first run): the file with the whole table, an
  entry for each call a tool answered or refused, and `forget_before`. A
  protocol error is no entry. A call whose entry cannot be written is
  answered all the same, with an error in the app's log: Phase 2 decides
  that again for a write. The schedule of retention comes with what starts
  the server (the second run), the filters, the search and the export with
  the Log view (the fourth).
- **`message` is not the database's message** (corrected in the first run).
  A database quotes what it could not read, and that can be a row's value:
  SQLite's "bad JSON path" names the path, which a statement can take from
  a column. The agent may read the row, so its sentence has the words. The
  log holds no row, so for a database's error it keeps "The database
  refused the statement (code 42703)." and nothing else. F.4's search of
  the file found this.
- **A log that is only locked is not a broken one.** A file is kept aside
  when it is no database, a damaged one, or of another schema. One that
  could not be looked into is an error, and stays where it is. What the
  server does without its log is the second run's to say.
- The file's mode is set whether the file was there or not.

**The app's own log is another thing.** `tabletist.log` never holds SQL
(`AGENTS.md`, and the editor spec's "The SQL text is never logged"). That
stays true. The MCP code logs a tool's name, a code and a duration there,
nothing more.

**The SDK writes to the app's log itself** (found in Phase 1b's first
run). `rmcp` traces through the `tracing` crate, and the app is built with
that crate's `log` feature on (through `calloop`, on Linux), which sends
what is traced to the app's log. What it writes is what a client sent:

- at `info`, what a client calls itself, every notification whole (a
  cancel's reason, a progress message), and each cancel;
- at `warn`, every protocol error with the request's id, which is the
  client's to choose, and for an unknown method its name;
- at `debug`, every request whole: the statement and its values.

`sqlparser`, which the check reads a statement with, writes each word at
`debug`, and `tracing` writes lines of its own for the SDK's spans. So the
app's log takes no line of the three. **As built** (`2026-10-14-mcp-server-1b-2a-gateway.md`), which links the
server into the app: the app's filter turns the three off
(`entrypoint::log_filter`), and because a `RUST_LOG` takes that filter's
place as a whole, the logger's rewriter blanks any line of theirs that a
`RUST_LOG` lets through (`entrypoint::without_agents_words`), so that the
line is there and none of its words. The first run's `tests/secrets.rs` holds every
other crate's lines to that, and shows what the SDK's hold. `AGENTS.md` has
the rule.

### C.2 Migration, and what agents cannot change

- A connection without an `agents` object reads as Off (`#[serde(default)]`,
  and `Agents::default()` is Off). Nothing is rewritten for it: no version
  bump, no migration pass.
- A new connection starts Off. A production one starts Off like any other,
  and in v1 cannot go above Read.
- **No tool writes anything but the log.** The four tools have no path to
  `connections.json`, `settings.toml`, `agents.json` or the keyring: the
  `Gateway` trait has no method for it. There is no "set" tool, no
  resources and no prompts.

**What MCP cannot stop.** A coding agent usually has a shell as the same
user. It can edit `settings.toml` itself, and Tabletist reloads that file
live. It can edit `connections.json` while the app is closed. That is the
account's doing, not the protocol's, and E says what it means. One cheap
control is still worth having:

- The app remembers, in the state directory, the agent settings it last
  showed the person: `enabled`, `listen`, and each connection's access and
  lists.
- A change that makes agents more able (the server turned on, an address
  changed, access raised, a table unhidden, a column unmasked) and that did
  not come from the app's own controls is not applied. The app shows a
  notice: "Agent access was changed outside Tabletist: Bookshop · dev is now
  Read + approved writes. Keep it, or put it back?" Until the person
  answers, the old, narrower setting holds.
- A change that makes agents less able applies at once.

It also catches a synced dotfile and a restored backup. It does not stop a
program that edits the remembered copy as well. Writes still wait for the
person either way.

**Moved on 2026-10-14: built with the first control that turns agents on**
(the third run's **Turn on**). Until then the files are the only way to
turn agents on, and a check that refused changes made outside the app would
refuse the only one there is. So after Phase 1b's second run a program that
can edit both files can turn agents on for itself, as E says of a shell.
**Moved again on 2026-10-14**: the third run is cut in two, and the check is
its second part (1b-3b), right after **Turn on** (`2026-10-14-mcp-server-1b-3a-agents-tab.md`).

**As built** (`2026-10-14-mcp-server-1b-3b-changes-outside.md`). What it settled where the above left it open:

- What the app last showed is `agents_shown.json` in the state directory:
  whether the server is on, and each connection's access. `listen` and the
  lists come with what they are for (1d, 1c, 3a).
- **A file that is not there, or cannot be read, shows nothing.** So the
  first start of the version that checks holds whatever was turned on by
  hand, once, and a person who set agents up through the files keeps it
  with one click. Taking the files as shown at that start would let a
  program that removes the file turn agents on unseen.
- The settings are compared at every edit of the file, the connections at
  every start (the file is read then alone) and at every save.
- The app's own controls are what changes the settings through the app
  (`change_settings`: **Turn on**, later Settings › Agents). The
  connection dialog has no control for a connection's access yet, so its
  save keeps what the file says then: a raise made outside stays held,
  and one put back stays back. The controls that raise it come with 1b-5
  (`Action::SetAgentAccess`, the editor's Agents section): each marks what
  it raises as shown, as `change_settings` does for the server.
- The notice is a bar of its own under the bar that leads the window
  (on macOS the title bar, so the window's own buttons keep their line),
  in both looks, which no dismissal takes away: what changed, "Until you
  answer, agents may not", **Keep** and **Put back**. Both answer what the
  bar said: a change that comes in after it was drawn waits for its own
  answer. **Put back** turns the server off or sets the access back in the
  files, and leaves the rest of them. While a change is held, the Agents
  entry and tab say what is in force (Server off, or Off here), and the
  server's sessions are not taken.
- Where agents cannot reach the app (Windows) nothing is held.
- Only the server and each connection's access are compared. A Read
  connection pointed at another server by hand is not held: an agent
  with the person's shell gains little by it, and the settings and
  connection files are the person's to edit.

### C.3 Hidden tables and masked columns when the schema changes

Entries are names: `customers`, `sales.customers`, `orders.email`. A schema
changes under them.

| Change | What happens |
|---|---|
| A hidden table is dropped | The entry stays, marked "not in this database" in the editor. If the table comes back it is hidden |
| A hidden table is renamed, PostgreSQL | Followed. The entry keeps the table's OID from the day it was picked, and a rename keeps the OID. The editor shows the new name. The plan check compares OIDs, so there is no gap |
| A hidden table is renamed, MySQL or SQLite | Not visible as a rename: one name is gone and another is new. **Agents are stopped on that connection** (`connection_unavailable`, "Its agent settings need a look in Tabletist") until the person removes the entry or points it at a table. A leak is the cost of guessing |
| A masked column is renamed or dropped | The same: followed by attribute number on PostgreSQL, a stop elsewhere |
| A new table | Visible under All and All except, hidden under Only. The editor's hint says so beside the control |
| A new column on a table with masked columns | Visible. Masks are per column |

How soon a change is seen:

- PostgreSQL: at once. Every check resolves names to OIDs in the run's own
  transaction.
- SQLite: at once. `PRAGMA schema_version` is read before each call and the
  catalog is read again when it moved.
- MySQL: within 30 seconds, the age at which the catalog is read again.
  This is the gap named in B.3.

## D. UI work

Views draw and push `Action`s. `App::apply` changes state. Database and disk
work stays on the backend (`AGENTS.md`). New state lives on `App`:
`agents: AgentsState` (server status, sessions, the feed's last few hundred
calls, pending write requests), fed by `Event::Mcp(..)`.

### D.1 Settings › Agents

Boards: `macOS – Agents (MCP), setup`, `Omarchy – Agents (MCP), setup`.

| What | Where |
|---|---|
| A pane switch. `SettingsDialog` gains `pane: Pane { General, Agents }`, with an `Action` to change it | `src/model.rs:1446-1453` |
| macOS and Windows: the nav lists two items, not the one hard-coded "General". The Agents pane: the toggle, the status row with **Show Agents tab**, "Connect an agent" with the three-way switch and Copy, the Access box with "Open Connections", "How it stays safe", the two Log rows with **Show log** | `src/ui/settings/sheet.rs:166-238`, a new `src/ui/settings/agents.rs` |
| Omarchy: the same nav, the rows with the cursor, `y` to copy the command, `enter` on the access row to open Connections, and the file pane showing `[mcp]` | `src/ui/settings/terminal.rs:220-236`, `file_pane.rs:195-313`, `mod.rs:126-239` for the keys |
| The settings keys | `src/settings.rs` (C.1) |
| The command to copy. macOS: the path of the running binary inside the app bundle (`std::env::current_exe`). Linux: `tabletist mcp` when `tabletist` is on the `PATH`, else the full path | New, in `tabletist-mcp` |
| The list of agents under "Connect an agent": stdio sessions with Stop, HTTP agents with Remove, "Add an agent…" under Other for a token | Not drawn on the board (A.3) |

The board's other panes (Appearance, Editor, Keyboard, AI assistant,
Updates) are not built by this work.

**Changed on 2026-10-13.** "Activity…" is **Show Agents tab**, and "Open
log" is **Show log**. Settings is the app's, and an Agents tab is a
connection's, so each needs a connection to show: the window that was in
front when Settings opened. With no connection window open both are
disabled, with the reason beside them ("Open a connection to see its
agents"). The board does not draw that case.

**As built in the second part of Phase 1b's fifth run** (`2026-10-15-mcp-server-1b-5b-settings.md`). Where
it draws otherwise than the boards:

- The nav lists General and Agents, and General alone where agents cannot
  reach the app (Windows). On Omarchy `ctrl+h` and `ctrl+l` switch panes,
  and the nav's foot says so.
- The switch is `mcp.enabled` as it is in force: off while a change made
  outside the app that turned it on is held. Turning it on is the
  person's own, as **Turn on** is, and keeps what was held. Turning it off
  ends every session: it is "Stop all agents" (question 11).
- The status says "stdio · this computer only" until HTTP (1d), and how
  many agents are connected. **Show Agents tab** and **Show log** close
  Settings and show the tab of the window in front, or with Connections
  in front of the last window open, the way Review does.
- "Connect an agent" switches between Claude Code's command, Cursor's
  entry for its `mcp.json`, and for Other the command `tabletist mcp`,
  each with **Copy** (agreed on 2026-10-15). The list of agents and
  "Removing it here disconnects it at once." come with HTTP (1d).
  Omarchy draws the board's box with `y`, and "cursor, others" with the
  command.
- The Access box says "Off or Read" and the safety box leaves out writes:
  both until Phase 2. **Open Connections** closes Settings.
- The log's row offers 7, 30, 90 and 365 days, with a number set in the
  file in its place among them. Its small print says the log keeps "never
  the rows", where the board says "what was returned". "Notify when an
  agent connects" leaves out writes until Phase 2.
- Omarchy's rows under the cursor are the switch, the status (`enter`
  shows the Agents tab), the access (`enter` opens Connections, and the
  hint names `a`), the log's days (`enter` shows the log) and the
  notification. The command, the line for other agents and the note are
  not rows. The hints are shorter than the board's, to fit beside the file
  at 1440 wide.
- Reset to defaults puts back the options of the pane it is asked on, and
  on the Agents pane leaves the server's switch as it is.

### D.2 The Connections list

Boards: `macOS – Connections, agent access`, `Omarchy – Connections, agent
access`.

| What | Where |
|---|---|
| macOS: an Agents column, 140 pt, between security and "Last used": the word "Agents" over the level, dimmed when Off | `mac_row`, `src/ui/picker.rs:733-1000`, widths at `:797-804` |
| Omarchy: `agents read` at the right of the row | `terminal_row`, `:1005-1125` |
| macOS: "Agents ▸" in the "…" menu and in the right-click menu, with Off, Read, Read + approved writes, a check on the current one, "Tables, columns and limits…" (opens the editor at the Agents section) and the line "Writes still wait for your approval each time." | `:983-999`, `:681-708`. Whether `src/ui/menu.rs` can nest a menu is to be checked in the phase plan. If not, the three levels go in the menu itself under a heading |
| Omarchy: `a` opens a small box, "agents · Bookshop dev", with the three levels, `j`/`k`, `enter`, `esc` | A new `Dialog::AgentAccess`, drawn with `src/ui/terminal_dialog.rs` |
| A new `Action::SetAgentAccess { conn, access }`, which saves through `App::save_connections` (`src/app.rs:2827-2832`) and sends the new policy to the backend | `src/model.rs:47`, `src/app.rs` |
| Levels that are not available are shown disabled with the reason: "Open read-only is on", "Production" | `connections::access`, `src/connections.rs:70` |
| The footer hints gain `a agent access` on Omarchy | `picker.rs:336-344`, from the keymap |
| **Show Agents tab** (added on 2026-10-13). macOS: an item of the "…" menu's Agents part. Omarchy: `t` in the box that `a` opens. It opens the connection's window when it has none, and shows its Agents tab | `Action::ShowAgents`, D.4 |
| The row's Agents column counts the writes waiting on a connection that has no window open ("1 waiting") | Not drawn on a board (question 25) |

**As built in the first part of Phase 1b's fifth run** (`2026-10-15-mcp-server-1b-5a-access.md`). Where it
draws otherwise than the board:

- `src/ui/menu.rs` nests no menu: the "…" menu and the right-click menu
  end with an Agents part under a heading of its own, Off and Read with a
  check on the one in force, **Tables, columns and limits…** and **Show
  Agents tab**. "Read + approved writes" and "Writes still wait for your
  approval each time." come with writes (Phase 2). No level is disabled
  until then: Read is open to every connection.
- The column's 140 come out of the row's spare room and the host while
  the host keeps 160; in a narrower window the column steps back, and the
  menu has the Agents part.
- Omarchy's row says `· agents read` at the end of its second line.
  Omarchy's box (`Dialog::AgentAccess`) is `j`/`k`, `enter` to set, `t`
  for the Agents tab and `esc`; its foot says `a opens this · j/k · t
  agents tab`. A plain Enter answers it; what a held one repeats does not
  connect the row behind it once it closes. The list's footer hints give
  way from the end where the window has no room for them.
- What the list and the box show and set is what is in force: a change
  made outside the app that is held shows as what was shown, and the
  person's own Read keeps it (1b-3b).

### D.3 The connection editor

Board: `macOS – Edit connection, agent access`, and the right half of the
Omarchy connections board.

| What | Where |
|---|---|
| `ConnectionForm` gains `agents: AgentsForm`, filled in `from_saved` and written in `to_saved` | `src/model.rs:767-837`, `:1018`, `:1232` |
| macOS: an "Agents" fieldset after `safety`: the access switch, the "Agents use this connection as dev" line, Tables (All, All except, Only, chips, "Add table…"), Masked columns (chips, "Add column…"), Rows per answer, Statement timeout, and the note about read-only and production | `src/ui/connect_dialog/sheet.rs:610-650` |
| Omarchy: an "agents" section after "Safety" with the same rows, `h`/`l` to change a value, and the JSON fragment that will be saved | `src/ui/connect_dialog/terminal.rs:312-338` |
| The segmented control | `choice.rs:292-333`, as the Environment row uses it |
| "Add table…" and "Add column…": a list from the connection's catalog when a session is open, typed names when none is. A name that matches nothing is kept and marked (C.3) | Phase 1c and 3a |
| The hint from B.3 under Tables | |

Phase 1b builds access, rows and timeout. Phase 1c adds Tables. Phase 3a
adds Masked columns.

**As built** (`2026-10-15-mcp-server-1b-5a-access.md`): the section after Safety in both looks, Off and Read
(the third level comes with writes), "Agents use this connection as dev."
where the connection has an environment, **Rows per answer** (1 to 5000)
and **Statement timeout** in seconds (1 to 60), checked on Save, and the
board's note with "Every connection starts with agents Off." for its last
sentence. The form opens with the access in force; an access the person
changed is theirs (a raise is shown, never held), and one left as it
opened keeps what the file says, a change made outside staying held.
**Allow agents…** and **Tables, columns and limits…** open the form at
this section.

### D.4 The Agents tab

Boards: `macOS – Agents tab, activity and approval`, `macOS – Agents tab, no
agents yet`, `macOS – Agents tab, ways in and empty states`, `Omarchy –
Agents tab, ways in and empty states`, `Omarchy – Agents tab, activity,
approval and log`.

**Changed on 2026-10-13.** This section first made Activity one tab of the
app, beside the connections, and put the approval in a dialog. The boards
now put both inside each connection window, in a tab of that connection.
There is no app-wide agents window and no agents dialog.

**The tab.** `Tab` gains `Agents(Box<AgentsTab>)` beside `Object` and `Sql`
(`src/model.rs:2446-2449`), drawn by a new `src/ui/agents/` with `mod.rs`,
`sheet.rs` and `terminal.rs`, as Settings and the connection editor are
split.

| Rule | How |
|---|---|
| One for each connection window, opened on demand, closable | `Workspace::tabs` holds one at most. `Action::ShowAgents { conn, view }` opens it or makes it the active tab, and opens the connection's window first when it has none. `CloseTab` closes it as any tab |
| It never opens itself and never takes the focus, even when a write arrives | No arm of `Event::Mcp` in `App::apply` adds a tab, changes `active_tab`, or moves the keys. Only what the person does can: the sidebar entry, the key, quick open, a notification they clicked, Settings, the Connections row. F.7 holds every event to it |
| Closing it stops nothing | Sessions and requests live in `App::agents`, not in the tab. A request keeps waiting, and keeps its five minutes |
| A count while a write waits | The tab's label and the sidebar entry read one number from `App::agents`: the requests waiting on this connection |
| Its place | Where it was opened, after the tabs that were there, with a number on Omarchy as any tab has (`alt+1..9`, `C::SwitchTab`, `src/keymap.rs:942`). It holds no pending edits, so closing it asks nothing, and it is not among the editors a closed tab can be brought back from |
| The strip | `src/ui/object_tabs.rs` draws tables and SQL editors. It gains the third kind: the icon, "Agents", the count |

Every `match` on `Tab` gains an arm for the new kind, where a tab is closed
or switched to among others. The phase plan lists them from the compiler's
errors.

**One connection.** `App::agents: AgentsState` stays one for the app (the
server's status, every session, the feed's last few hundred calls, the
requests waiting), fed by `Event::Mcp(..)`. A tab shows the part of it that
is its window's connection:

- the header: "Agents on Bookshop · dev · Read + approved writes · Change in
  Connections". The link opens the connection editor at its Agents section
  (D.3);
- **Stop agents here**, which also turns this connection's access Off
  (A.5);
- "Also: Cursor on Bookshop · staging · Show", for sessions that are
  elsewhere. Show brings that connection's window forward with its Agents
  tab open, and opens the window first when there is none. On Omarchy these
  are the "elsewhere" group under the sessions, and `enter` shows;
- the sessions that are here (A.5), each with its line ("Claude Code ·
  tabletist-repo · connected 10:38 · 14 calls · Runs as dev") and **Stop
  session**;
- the feed, of the calls on this connection;
- the status bar: "1 agent here · 1 write waiting", the server's addresses,
  and the key that shows the tab.

**Ways in.** There is always a visible one, whether or not an agent is
connected.

| Way | macOS and Windows | Omarchy | In the code |
|---|---|---|---|
| The sidebar, in every state | An **Agents** button in the sidebar's footer, above **SQL Editor**, highlighted while its tab is the active one | The sidebar's last line above the key hints: `agents`, the state, `ga` | `src/ui/sidebar.rs`. The footer is one button high (`SQL_FOOTER`, `:426`) and the terminal's is the hints (`terminal_footer`, `:364`). Each grows by a row |
| The key | ⌥⌘A | `ga` from the grid or the sidebar, `:agents` from anywhere in the window | D.6 |
| Quick open | ⌘P: "agents" offers "Agents · Bookshop · dev" | none | Row 13 of section 2 |
| The menu | View › Agents | none | Row 4 of section 2. Not in Phase 1b |
| A notification | For a write, or for a new agent | The same, through mako | Below |
| Settings › Agents | **Show Agents tab**, **Show log** | The same rows | D.1 |
| Connections | The row's "…" › Agents › **Show Agents tab** | `a` on a row, then `t` | D.2 |

All of them work in every state, the server off included: the tab is the
app's own view and needs no server. None of them is the sidebar entry
opening the tab by itself.

The sidebar entry's right side has five states:

| State | macOS and Windows | Omarchy | When |
|---|---|---|---|
| Waiting | an amber **1 waiting** pill | `● 1 waiting` | a write waits on this connection |
| Connected | **● Claude Code**, green | `● claude code` | a session is here |
| Idle | **None connected** | `none connected` | access allows agents, and nobody is here |
| Off here | **Off here**, muted | `off here` | this connection's access is Off |
| Server off | **Server off**, muted | `server off` | `mcp.enabled` is false |

One function gives the state from the settings, the connection's access and
`App::agents`, and the tab's empty state asks the same one. Two cases the
boards leave open: with the server off and access Off at once it says
"Server off" (settled, question 26: it is the outer switch, and after "Turn
on" the tab shows "Off here", which is the next step), and with more than
one session here it names the one that called last.

**Activity.** The sessions and the feed are Phase 1b's. The card is Phase
2's.

**The write request card** is in the tab, above the feed: "Claude Code wants
to change 3 rows in book_covers · asked 10:43:02 · waits up to 5 min", the
SQL as it will run, the agent's reason, the count and how it was made (B.6,
row 7 of section 2), "Transaction: one, then commit", "Preview the 3 rows in
a tab", the cascade and no-`WHERE` warnings, and "Nothing runs until you
choose. The agent gets the result or your note."

- macOS and Windows: **Deny…** and **Approve and run ⌘↩**. Deny opens a
  one-line note field in the card.
- Omarchy: `[a] approve and run  [d] deny with a note  [p] preview rows
  [esc] decide later`.

What keeps an approval the person's own:

- **A request does three things and no more:** the notification, the count
  on the sidebar entry and on the tab, and on Omarchy the bottom line. It
  opens nothing and moves no keys. `prompt_password` is the precedent for
  not taking over what the person is working in (`src/app.rs:2009`).
- **The keys are the tab's.** ⌘↩, `a`, `d` and `p` are bound in a new
  `Agents` scope with `When::AgentWrite`. They mean nothing in a grid, the
  sidebar or a SQL editor. This replaces the dialog the plan first had: the
  card is drawn in the tab, and the tab is only ever in front because the
  person put it there.
- **The bottom line on Omarchy** reads `-- AGENT WRITE -- claude code · 3
  rows in Bookshop · dev  a approve  d deny  esc later` while the Agents tab
  has the keys, as the board draws it. In any other tab of that window it
  ends `ga agents` in the place of the three keys: a line that named `a`
  where `a` does nothing would be worse than none.
- **A key lands on the request the person saw.** `ApproveAgentWrite` and
  `DenyAgentWrite` carry the id of the request that was drawn. One that
  arrives while the tab already has the keys takes them only once it has
  been on screen for half a second, so a key on its way does not answer a
  card that has just appeared. And the request is looked up again when the
  action is applied, as `ConfirmSqlRun` re-checks the tab's mode: a card
  left on screen cannot approve a request that is gone, or a connection that
  has since become read-only.
- One request is shown at a time for a connection, the oldest first (E.5),
  with the number of the others.
- A request that was withdrawn or timed out says so in its card and offers
  only Dismiss.
- `esc`, "decide later", leaves the request waiting and gives the keys back
  to the feed. The card stays, with the time left.
- "Preview the rows in a tab" opens a read-only SQL tab in this window with
  the `SELECT` of the rows the write would change (B.6). It needs the
  window's own session, so it waits for a window that is still connecting.

**Empty states.** In the Activity view's place, by the state the sidebar
entry shows.

**As built in Phase 1b's third run** (`2026-10-14-mcp-server-1b-3a-agents-tab.md`). Where it draws otherwise than
the boards, each comes with what it needs:

- The Activity and Log switch, **Show log**, and "The log still shows past
  calls" come with the Log view (the fourth run). "Last agent" in the box
  "On this connection" too: it is read from the log.
- **Allow agents…**, **Other agents…**, **Settings…** and the header's
  "Change in Connections" come with the editor's section and Settings ›
  Agents (the fifth run). The Off state has no button until then.
- "and propose changes you approve here" comes with writes (Phase 2), and
  so do the Waiting state of the entry and the count on the tab.
- With an agent here the tab lists the sessions that are here, without
  **Stop session**, "Also:" or the feed (the fourth run).
- A session's line says how long ago it connected ("connected 4 min
  ago"): the app reads no local clock. The status bar says "Server: stdio"
  until HTTP (1d).
- The macOS entry is lit by an accent outline while its tab is in front,
  which the boards do not draw. Omarchy's empty states are the look's
  empty state, from the top left, without the boards' box.

**As built in the first part of Phase 1b's fourth run** (`2026-10-14-mcp-server-1b-4a-activity.md`): the
sessions with **Stop session**, "Also:" with **Show**, **Stop agents
here**, and the feed. Where it draws otherwise than the boards:

- The feed's time said how long ago ("just now", "4 min ago") until the
  fourth run's second part brought the clock (`2026-10-14-mcp-server-1b-4b-log.md`): it says the time
  now ("10:43:02"), and the session's line when it connected ("connected
  10:38"), as the boards draw them.
- The feed is the last 300 calls the app heard of since it started, on
  every connection, each window showing those on its own. A call's
  statement is kept on one line and cut at 200 characters; its values are
  not kept. The log has every call whole.
- The macOS header's "Also:" names the session elsewhere that called
  last, with **Show**, and counts the others ("and 2 more"). Each window
  of theirs shows them as here. Where a session works is the connection
  it named last: a session's connections end with that one
  (`SessionInfo::connections`).
- Omarchy's list of sessions moves with `j` and `k`, which the board does
  not draw: `S` stops the selected session and `enter` shows where a
  session elsewhere works. The selection is the session's id, so a key
  lands on the session the person saw however the list reorders. The
  list is there with nobody here too, beside the way to connect one.
  Enter is the list's unless the person went to the tree, or a button has
  the keyboard; the arrows stay the tree's.
- A call stopped by **Stop session** says, in the feed and to its agent,
  that the agent server was stopped: the one sentence the server has for
  a stop. Calls cut off by turning the server off are in the log, not in
  the feed, which hears nothing once the server is stopped.
- **Stop agents here** says what it did in the toast (on Omarchy, the
  status line): "Stopped 2 agents. Access is now Off for Bookshop · dev."
  The level the access had is not kept beside it yet: **Allow agents…**,
  which offers it, comes with the fifth run, and with Off and Read alone
  the level is always Read.

**As built in the second part of Phase 1b's fourth run** (`2026-10-14-mcp-server-1b-4b-log.md`): the Log
view, **Show log**, the clock, and the notification for an agent that
connects. Where it draws otherwise than the boards:

- The log is the tab's Log view in both looks. Omarchy's board puts it
  under the feed on one screen; there it is a view of its own too, which
  `l` shows and leaves, and `:agents log` opens.
- macOS chooses This connection or All connections with a segmented
  control: the board's menu holds the agent, result and search filters
  too, which come with Phase 3b.
- The list is Time, Agent, the connection under All connections, Tool,
  Request, Rows, ms and Result, 200 calls at a time with **Show more**.
  The selected call's line under it ("Claude Code · Oct 13 10:43:40 · 3
  rows, 2 columns returned · 4 ms · rows themselves are not logged") is
  Phase 1b's detail; the pane is 3b's.
- The log is read from its file as it is: with the server off nothing is
  made and a damaged file is not set aside. A write a crash left half
  done is rolled back as SQLite does on any open, which a read-only open
  cannot do, so the file is opened to write and never written. A page on
  the screen is read again when a call on its connection ends, and once
  more after a read that was out when one ended.
- **Show log** is in the Off here and nobody here states. The Server off
  state keeps **Turn on** alone until Settings comes (the fifth run); its
  log is the switch's, or `l`.
- The clock is jiff's, in the system's time zone: dates in the log, and
  times in the feed and on a session's line.

**As built in the second part of Phase 1b's fifth run** (`2026-10-15-mcp-server-1b-5b-settings.md`):
**Settings…** beside **Turn on** with the server off, and **Other
agents…** beside **Show log** with nobody here, both opening Settings ›
Agents; on Omarchy `s` in both states, with its hint on the bottom line.

| State | Title | Body | Actions |
|---|---|---|---|
| Allowed, no agents yet | No agents connected to Bookshop · dev | "Agents like Claude Code can read this database and propose changes you approve here. Connect one with this command, then ask it about Bookshop." Then the command with **Copy**, and a box "On this connection": access, hidden tables, masked columns, last agent | **Other agents…**, **Show log** |
| Off for this connection | Agents can't use Bookshop · production | "Access is Off for this connection, so agents don't see it at all. The log still shows past calls." | **Show log**, **Allow agents…** |
| Server off | The agent server is off | "Turn it on to let agents like Claude Code use the connections you allow. It runs only while Tabletist is open, on this computer." | **Settings…**, **Turn on** |

- **Allow agents…** opens the connection editor at its Agents section, with
  the level chosen that "Stop agents here" took away, where it took one.
  Nothing changes until Save there (`ctrl+s` on Omarchy).
- **Turn on** sets `mcp.enabled` through `App::change_settings`, which
  starts the server at once. It changes no connection's access: each still
  starts Off (settled, question 23).
- **Other agents…** opens Settings › Agents. **Copy** copies D.1's command.
  **Show log** switches the tab to its Log view.
- The box "On this connection" grows with the phases: access in 1b, hidden
  tables in 1c, masked columns in 3a. "Last agent" is read from the log.
- On Omarchy: `[y]` copy, `[l]` show log, `[e]` allow agents…, `[o]` turn
  on, `[s]` settings.

**Actions.** `ShowAgents { conn, view }`, `StopAgentSession`,
`StopAgentsHere { conn }`, `ApproveAgentWrite { request }`,
`DenyAgentWrite { request, note }`, `PreviewAgentWrite { request }`,
`TurnAgentsOn`, `CopyAgentCommand` (`src/model.rs:47`), with `Command::Mcp`
and `Event::Mcp` (`src/backend.rs:52`, `:209`).

**Notifications.** There are none in the app today.

| Platform | How | Notes |
|---|---|---|
| Linux (mako on Omarchy) | `org.freedesktop.Notifications.Notify` over D-Bus with `zbus`, which is in the tree through the keyring store. One action, "Review" | A direct dependency on a crate already built. No `notify-rust` |
| macOS | `UNUserNotificationCenter`, in `crates/tabletist-appkit`, the only crate allowed `unsafe` for AppKit | Needs the signed app bundle and the person's permission. In an unsigned build: the Dock icon bounces (`ViewportCommand::RequestUserAttention`) |
| Windows | The taskbar button flashes | A toast needs an app identity the installer does not register yet |

- **A write:** "Review" brings Tabletist forward, goes to that connection's
  window and shows its Agents tab with the card. It is the same
  `Action::ShowAgents` on both looks: the notification's action carries the
  connection's id and nothing else.
- **A write on a connection whose window is not open** opens that window on
  its Agents tab. The window connects behind the tab as any does, and may
  ask for a password or an SSH host key first (row 15 of section 2). There
  is no app-wide window to fall back to.
- **A new agent** is on no connection yet. Its notification goes to the
  frontmost connection window's Agents tab, where it is under "Also:", or to
  the Connections screen when no window is open (settled, question 22).
- Where the platform has no action to click (the Dock's bounce, the
  taskbar's flash), bringing Tabletist forward shows the count on the
  sidebar entry, and the person opens the tab.
- Nothing can be approved from a notification: the SQL is not on it.
- The text holds the agent's name, the count and the connection's name. No
  SQL, since notifications are kept in a history other programs can read.
- `mcp.notify` turns off the notifications for a connect and for a write
  request. The counts and the bottom line stay.
- An agent connecting is one notification for each session, not for each
  call.

**As built** (`2026-10-14-mcp-server-1b-4b-log.md`), for an agent that connects: on Linux a
notification over D-Bus with `zbus` ("Claude Code connected to
Tabletist", and "It can use the connections you allow agents. Review
shows what it does."), whose Review brings Tabletist forward on the
Agents tab of the window in front (with Connections in front, of the last
window open), or on Connections where no window is open; on macOS the
Dock's bounce. A session is told of once, when its agent has named itself.
`[mcp] notify` is `true` unless set otherwise.

### D.5 The log

Boards: `macOS – Agents tab, log`, and the lower half of the Omarchy board.

**Changed on 2026-10-13.** The log is the Agents tab's second view
(`AgentsTab::view: Activity | Log`), behind the **Activity / Log** switch at
the tab's top. It was to be a window, and before that a view of an app-wide
tab.

| What | Where |
|---|---|
| The table: time, agent, tool, request, rows, ms, result. The connection is a column only under All connections | `src/ui/agents/log.rs`. The grid's row drawing in `src/ui/grid.rs` is for database values. A plain list is enough here |
| Filters: **This connection** (the default) or All connections, the agent, the result, and a search of the SQL. They are a query to the backend, `Command::Mcp(QueryLog { filter, before, limit })`, answered in pages | `src/backend.rs` |
| "This connection" is the connection's id. The calls of a connection that was deleted since are under All connections alone | C.1 |
| The count line: "186 calls · last 30 days · on this computer" | |
| The detail pane, for the selected call: the SQL with its parameters, the rows changed against the count the person was shown ("3 (dry run said 3)"), the time waited, the time taken, who approved, the agent's reason. For a read: what ran, the rows and columns returned, the time, what was masked. And the note that rows are not kept | The parameters are what question 18 asks about: the board shows them in full. The count's words follow question 7 |
| **Open in SQL editor**: opens a SQL tab with the statement, in Read-only mode, not run. In this window for a call on this connection, and in that connection's window for another. A connection that was deleted since disables it | `Action::NewSqlTab` and the workspace opening in `src/app.rs` |
| **Export…** | C.1 |
| Omarchy: `L` shows all connections and back, `/` filters, `enter` details, `o` opens in a SQL tab | D.6 |

The Log view works with the server off and with access Off: it reads the log
file, and both empty states say that it still shows past calls. Rows are
never stored (C.1).

Phase 1b ships the list for this connection, the switch to All connections,
and the detail line. Phase 3b adds the other filters, the search, the detail
pane, export and "Open in SQL editor".

### D.6 The keymap

Checked against `src/keymap.rs` and the keyboard brief
(`docs/superpowers/specs/2026-10-08-keyboard-shortcuts-design.md`, the
brief's `hotkeys-fix-prompt.md`). A key is spelled in the keymap and nowhere
else, and `tests/keys.rs` fails on one spelled in a view.

| Command | Scope, when | macOS | Omarchy | Checked against |
|---|---|---|---|---|
| Show the Agents tab | `Global` | `cmd+alt+a`, the sidebar's **Agents**, View › Agents | `ga` in `Grid` and `Sidebar`, `:agents` | `cmd+alt+a` is bound nowhere: the `cmd+alt` chords are the arrows, `backspace`, `e` and `l` (`:1008-1014`, `:1284`, `:1328-1352`, `:1410`). `g` begins `gd`, `gs` and `gl` in `Grid` (`:1260-1261`, `:1285`) and nothing in `Sidebar`, and `ga` is free in both. `:agents` needs an ex command in every scope of a connection window: `Keymap::ex` is asked in `Grid` and `SqlEditor` today |
| The tab on its Log view | `Global` | **Show log** | `:agents log` | The same |
| Switch to the Agents tab | `Global` | a click on the tab, `cmd+shift+[` and `]` | `alt+1..9` | `C::SwitchTab` (`:942`), `C::PreviousTab` and `C::NextTab` as they are. The tab has a number like any |
| Show the tab from Connections | `Prompt`, in the box that `a` opens | "…" › Agents › Show Agents tab | `a`, then `t` | The box is new (D.2). `t` is `C::ToggleTree` in `Sidebar` (`:991`), another scope |
| Approve agent write | `Agents`, `When::AgentWrite` | `cmd+enter` | `a` | `cmd+enter` is Apply in a large cell editor (`:1188`), Run statement in `SqlEditor` (`:1391`), Accept and run for the assistant (`:1527`), and Save and connect in `Prompt` (`:1598`). `Agents` is a scope of its own, alongside `Global` only. Plain `a` is bound nowhere |
| Deny agent write | `Agents`, `When::AgentWrite` | the button | `d` | `d` is Delete in `Grid` when `Visual` (`:1258`) and Discard in `Prompt` (`:1561`, `:1572`). Other scopes |
| Preview rows | `Agents`, `When::AgentWrite` | the link | `p` | `p` is Put row in a grid's cells (`:1242`). Another scope |
| Decide later | `Agents`, `When::AgentWrite` | `esc` | `esc` | It gives the keys back to the feed. It closes nothing |
| Stop session | `Agents` | the button | `S` | The scope is new. It sits alongside `Global`, whose keys are chords |
| Stop agents here | `Agents` | the button | `X` | `X` was to be Stop all agents, which is the switch in Settings now. The board draws no key for this one |
| All connections in the log | `Agents`, the Log view | the filter | `L` | Free in the new scope. `h/l` move in a grid (`:1030`), in lower case |
| Filter the log, open its details | `Agents`, the Log view | the fields | `/`, `enter` | As in the Connections list |
| Open in a SQL tab | `Agents`, the Log view | the button | `o` | `o` is Add row in `Grid` (`:1227`). Another scope |
| Copy the command, show the log | `Agents`, no agents yet | the buttons | `y`, `l` | |
| Allow agents…, show the log | `Agents`, Off here | the buttons | `e`, `l` | `e` edits a connection in `Connections` (`:883`), which is what this does |
| Turn on, Settings | `Agents`, server off | the buttons | `o`, `s` | `o` is also Open in a SQL tab, in the Log view. The two are never on screen together |
| Change agent access | `Connections` | the "…" menu | `a` | The scope has `e`, `n`, `yy p`, `dd`, `/`, `enter` (`:875-915`). Nothing begins with `a` |
| Copy the command, in Settings | `Prompt`, `When::SettingsScreen` | the button | `y` | The settings screen has `j`, `k`, `h`, `l`, `space`, `R`, `ctrl+e`, `esc` |

**Changed on 2026-10-13.** The commands that opened an activity view and a
log view are one now, Show the Agents tab, with a key on macOS where there
was none. The approval's keys move from a dialog's scope to the tab's. The
keys of the empty states and of the log are new. Line numbers in this table
are from `main` at `cf7b78f`.

`?` lists `ga` under Navigation (`src/ui/help.rs:15`), as the board asks.
The View menu on the board shows no shortcut for Data and Structure, on
purpose: `cmd+1..9` is Go to connection window (`:835-840`).

The brief's proposals hold, with two conditions the code sets:

1. `a` and `d` for a write are the Agents tab's keys and no other scope's,
   and the tab never takes the keys by itself. This is row 6 of section 2.
2. The keyboard brief's rule 4 already makes a production write on Omarchy
   wait for the database's name typed out. If production ever goes above
   Read, `a` opens that box and does not replace it.

Each command also needs its `Info` (name, row, group), its place in the
settled list of names the keymap's tests hold (`:2356-2417`), and its lines
in `docs/_reference/keyboard-shortcuts.md` and `docs/_guide/omarchy.md`.

## E. Threat model

**What the server is.** A narrow, logged way for an agent that cooperates
to use the person's databases, and a hard stop on writes. It is not a
sandbox around the agent.

**What it is not.** A coding agent with a shell runs as the person. It can
read a project's `.env`, run `psql`, ask the desktop's keyring for a
password on Linux, and edit Tabletist's files. None of that passes through
MCP, and nothing here stops it. Tabletist's promise is about its own tools:
what goes through them is checked, capped, logged, and never writes without
a person.

| Threat | What stops it | What remains |
|---|---|---|
| **A prompt-injected agent.** A row, a web page or a file tells the agent to delete data or to send it somewhere | It can only do what the four tools do. Reads are read-only at the database, capped in rows, size and time. A write shows the person the SQL, the count and the agent's reason, and waits. There is no "always allow". Connections that are Off, hidden tables and masked values never enter its context. Every call is in the log | Whatever it may read, it can send elsewhere with its other tools: MCP governs the query, not the agent. A person who approves without reading. The agent's "reason" is text an injection can write, and the card labels it as the agent's words |
| **A malicious local process on 127.0.0.1.** Another program of the same user, or a web page in the person's browser | The port answers only with a token, which is 256 random bits and kept as a digest. A web page is refused before that: any `Origin` is refused, `Host` must be the loopback name, no CORS header is sent, so DNS rebinding and a plain cross-site `fetch` both fail. The socket is 0600 in a 0700 directory, and the relay must present the key | A program running as the same user can read the key file and start `tabletist mcp` itself. It is then an agent like any other: it appears in the Agents tab, a notification says it connected, it gets the same policy, and it cannot write without the person. It could also read the token from another agent's configuration file. Nothing local can be kept from the account that owns it |
| **Another user on the same machine** | Unix: the socket's and the directory's modes, and the state directory is private (`AppDirs::ensure`, `src/paths.rs:57-60`). Windows: the pipe refuses remote clients, and the key file is in the user's profile. The TCP port is open to every local user, and answers none of them without a token. Tokens and the log are 0600 | A user with administrator rights, as on any machine. Another user can see that the port is open |
| **An agent that exfiltrates through errors or timing** | Errors about connecting are replaced by one fixed sentence, so no host, user or path is sent. A refusal is made from the policy, before the database is asked, so it takes the same time whatever the data is. On a connection that hides or masks something, a database error loses its detail and hint lines, where PostgreSQL quotes rows. Masked columns cannot stand in a predicate, a sort or a function, so there is no yes-or-no question to time. The system catalogs and their sample values are closed there. The statement timeout and the rate limit bound how much can be asked | The name of a hidden table (by design). Whether rows exist and how many, on any table the agent may read. A function or a view in the database that the checks cannot see into (B.3). An error message whose main line quotes a value. A timing channel through such a function |
| **An agent that floods `propose_write`** | One request waits for each session. A second one from the same session is refused at once with `busy`. Across sessions, requests queue and the person sees one card at a time, oldest first, with "2 more waiting". At most 6 requests a minute and 30 an hour for each session, then `rate_limited`. After three denials in a row from one session, the next request needs the person to open the Agents tab: there is no notification for it. "Stop session" and "Stop agents here" are in that tab. One notification is shown for a session's requests, updated in place | A patient agent can still ask thirty times an hour. The person can turn the connection to Read, which ends it. Wearing a person down until they approve is a human risk that a count and a clear card reduce and do not remove |

Also considered:

- **The relay as a way in.** It parses nothing and forwards bytes. The app
  treats what arrives on the socket exactly as it treats HTTP: untrusted.
- **A dependency that changes.** rmcp releases often. The `Host`, `Origin`
  and token checks are ours, in front of it, with tests that do not go
  through rmcp's client.
- **The log as a store of secrets.** It holds SQL and parameters, which may
  hold personal data. It is private to the account, kept for a set number
  of days, and never holds a row.
- **A lost laptop.** Nothing new: the server runs only while the app is
  open, and the tokens are digests.

## F. Testing

Everything is headless. No pixel or design snapshot is part of any test run,
and no snapshot material is committed. UI tests go through `src/testing.rs`
(the AccessKit tree and events) with `Backend::recording()`
(`src/backend.rs:443`) and injected `Event::Mcp` values.

**Data.** One Bookshop fixture for each driver, new in Phase 1a:
`publishers`, `authors`, `books`, `book_covers`, `customers`, `orders` (with
`email` and `phone`), `order_items` (with a cascading foreign key), a view
`recent_orders` that reads `orders.email`, a view `customer_names` over
`customers`, and on PostgreSQL and MySQL a function that reads `customers`.
The function comes with Phase 1c, which is the first to test one. As built,
on PostgreSQL alone: MySQL lets only a user with `SUPER` make one while it
logs its changes (B.3). On each
server the Bookshop has a database of its own, `bookshop`: its `orders` is
not the driver fixture's.
SQLite runs everywhere. PostgreSQL and MySQL run where the suite has a
server, under `TABLETIST_TEST_PG_URL` and `TABLETIST_TEST_MYSQL_URL`, and
print "skipped" without one, as the existing suites do. `ci.yml` names the
test files its PostgreSQL and MySQL jobs run, so each new file is added to
both.

### F.1 An MCP client harness, over both transports

- The harness starts the server with a `Gateway` over the SQLite fixture, a
  scripted approver and a clock it controls, and gives the test a client.
  The first run of 1b has the gateway and the client
  (`crates/tabletist-mcp/tests/support/`). The approver and the clock come
  with Phase 2, which is the first to wait for a person.
- **In process:** rmcp's client (a dev-dependency) over `tokio::io::duplex`.
  Most tests use this.
- **Stdio, end to end:** the built binary, `tabletist mcp --endpoint
  <a path in a temp directory>`, started as `tests/cli.rs` starts the binary
  today, with JSON-RPC lines written to its stdin. Also: the app is not
  running (the error, exit 1, nothing on stdout but one JSON line), the key
  is wrong, the server is switched off.
- **HTTP:** the listener on `127.0.0.1:0`. The good path through rmcp's
  client. The refusals through raw requests over `TcpStream`, so they do
  not depend on what a client library is willing to send: no token, a wrong
  token, a removed token, an `Origin` header, a `Host` of another name, a
  wrong path, an oversized body, an `OPTIONS` preflight.
- **Each protocol lifecycle:** one test with the `initialize` handshake
  (2025-11-25) and one without (2026-07-28).
- The same tool tests run over all three, from one list.

### F.2 The parser and enforcement matrix, for each driver

In `crates/tabletist-db/tests/agent.rs`, table-driven: a statement, a
policy, and the expected code or result. One list, run on each driver the
suite has.

| Group | Cases |
|---|---|
| Multiple statements | `SELECT 1; DELETE …`, a second statement behind a comment, inside a dollar quote, after a MySQL executable comment, and one only the engine's lexer sees. Each is `too_many_statements` or refused by the prepare, and nothing runs |
| Writes through `run_query` | `INSERT`, `UPDATE`, `DELETE`, `MERGE`, `WITH x AS (DELETE … RETURNING *) SELECT`, `SELECT … INTO`, `SELECT … FOR UPDATE`, `INTO OUTFILE`. Then writes the parser cannot see, which the database must refuse: `SELECT setval(…)`, a function that writes. After each, a count of every table is what it was |
| Not allowed | DDL, `COPY`, `SET`, `RESET`, `DO`, `CALL`, `BEGIN`, `COMMIT`, `PRAGMA`, `ATTACH`, `SHOW`, `EXPLAIN ANALYZE`, text that does not parse |
| Hidden tables | By name, with a schema, quoted, in another case; in a join, a subquery, a CTE, `LATERAL`, a set operation; a CTE named like the table; `TABLE customers`; through `customer_names`; through the function; in `information_schema` and the engine's own catalogs; in `describe_schema`. Under "Only": a table made after the policy |
| Masking leaks | Every case of the brief: `WHERE email LIKE 'a%'`, `lower(email)`, `ORDER BY email`, `ORDER BY 2`, `GROUP BY email`, `count(DISTINCT email)`, a join on `email`, `SELECT *`. And: `SELECT DISTINCT email`, `UNION`, `NATURAL JOIN`, `USING (email)`, `SELECT *` in a subquery filtered outside, a renamed column list (`AS t(a, b, c)`), `SELECT o FROM orders o`, `to_jsonb(o)`, a window ordered by it, `email IS NULL`, `CASE WHEN email …`, a cast, through `recent_orders`. Then the allowed ones: `SELECT id, email`, `SELECT *`, `SELECT o.email AS e`, each answering `•••` in every row, a `NULL` row too |
| Timeout | A query that outlasts the limit is `timeout`, by the server's own timeout and, with that turned off in the test, by the backend's timer. The session is usable afterwards |
| Row cap | A table of 100,000 rows answers the limit, `truncated`, and "of M" when the count is quick, "at least N" when the count is made to run out of time. A wide cell is cut. An answer over the size cap is cut and says so |
| Parameters | Strings with quotes, backslashes and a NUL (refused); numbers at the edge of 64 bits; null; a count that does not match; a `$1` inside a string, which is not a parameter |
| Functions | One refused function for each dialect. A user's function with and without something hidden |
| Session | After every case: the transaction is gone, the session is read-only, its settings are as they were (the checks the script run's own tests make) |

A property test in the same file: for any statement in the list, on a
connection where everything is hidden, nothing reaches the database (the
SQLite authorizer is never asked).

### F.3 The approval flow, with a fake approver

The broker takes an `Approver` trait. Tests script it. The five-minute wait
runs on tokio's paused clock, which the workspace already uses in tests
(`Cargo.toml`, `test-util`).

- **Approve:** the statement runs in one transaction, the answer is
  `approved` with the count, the rows are changed, the log has one entry.
- **Deny with a note:** the agent gets the note, nothing changed.
- **Timeout:** at five minutes, `timed_out`, nothing changed, the card is
  gone.
- **Stop session mid-wait:** `server_stopped`, the card is withdrawn,
  nothing changed.
- **The client cancels, or closes, mid-wait:** the request is withdrawn. An
  approval that arrives afterwards runs nothing.
- **The count grew:** rows are added after the count and before the
  approval. `count_changed`, rolled back.
- **The connection became read-only, or Read, mid-wait:** the approval runs
  nothing and says why.
- **Two requests:** the second from one session is `busy`. Two sessions
  queue, oldest first. The rate limit, on the paused clock.
- **Progress:** a notification every 15 seconds while it waits.
- **UI:** F.7, which has the card with the rest of the Agents tab.

### F.4 The log

- One entry for each call, for every tool and every outcome in B.8.
- **No row data.** A test fills a table with a marker value, runs every tool
  against it, and searches every byte of the database file for the marker.
- A `propose_write` is one entry that goes from `waiting` to its end.
- Retention: entries older than `log_days` are gone after the job runs, on
  the paused clock. Newer ones stay.
- Export matches the filtered view, in both formats.
- A file that cannot be opened is set aside and a new one made.
- `connections.json`: a file with `agents` loads and saves unchanged. The
  version 1 fixture (`tests/fixtures/connections-v1.json`) loads with every
  connection Off. Since the move to TOML that is `connections.toml`'s, in
  `src/connections/file.rs`: version 3 and 4, a table with keys left out,
  limits refused past their bounds.

### F.5 Credentials never appear

One test, run over both transports. The fixture's connection has a marker
in each secret and each private field: the password, the SSH passphrase,
the user, the host, the database file's path.

- Every tool is called, with good and bad arguments, including the calls
  that fail to connect (a wrong password, an unreachable host).
- Every byte the server writes to the transport is captured, and so are
  every row of the log, the app's log lines and the notification texts.
- None holds a marker.
- A second, static check: no type in `tabletist-mcp` names `ConnectSpec` or
  `Secrets`. It reads the crate's sources as `tests/engines.rs` reads them
  for its own rule.

### F.6 Also

- The keymap's generated tests cover the new commands as they stand: no two
  share a key where both could be read, every command has its keys or none
  on purpose.
- `tests/engines.rs` holds for the new code: every `match` on `Driver` or
  `Dialect` names each variant.
- Settings: the canonical text with `[mcp]`, each key on its line, a bad
  `listen` refused with a notice.
- The website's check (`docs/bin/check`) for the pages each phase changes.

### F.7 The Agents tab

Added on 2026-10-13. Headless, through `src/testing.rs` with
`Backend::recording()` and injected `Event::Mcp` values, on the Bookshop,
in both looks. No pixel or design snapshot is part of any test run, and no
snapshot material is committed.

- **The sidebar entry's state**, for each of the five cases: a write
  waiting, a session here, allowed with nobody here, access Off, the server
  off. And the two the boards leave open: the server off wins over access
  Off, and with two sessions here the one that called last is named.
- **The tab is never opened or focused by what arrives.** For each
  `Event::Mcp` (a session connects, a call, a write requested, a request
  withdrawn, the server stops): with the tab closed, no tab is added and the
  active tab and the keys are where they were. With the tab open and another
  tab active, the other stays active.
- **The count** on the sidebar entry and on the tab's label: none, one, two.
  A request on another connection does not count here. It goes when the
  request is answered, withdrawn or timed out.
- **One connection.** A session is here after its first call that names the
  connection and not before. One that works on two is in both windows. One
  that has named none is under "Also:". Show brings the other window forward
  on its Agents tab, and opens it first when it is not open.
- **One tab for a window.** Each way in (the sidebar, the key, quick open,
  Settings, the Connections row, a notification's action) shows the same
  tab, and each works with the server off.
- **Closing the tab stops nothing.** The sessions are there and a request
  still waits, with the time it had.
- **The log defaults to This connection.** All connections shows the rest.
  The calls of a deleted connection are under All connections alone.
- **The empty states:** which one shows for each state. "Allow agents…"
  opens the editor at its Agents section, and until Save nothing is saved
  and nothing is sent: Cancel leaves access Off. "Turn on" starts the server
  with one change of the settings, and changes no connection's access. Copy
  puts the command on the clipboard.
- **The approval's keys.** `a`, `d` and `cmd+enter` in a grid, in the
  sidebar and in a SQL editor send nothing while a request waits. In the
  Agents tab they answer the request that is drawn. A request that arrived
  less than half a second ago is not answered. A key for a request that is
  gone does nothing.
- **Omarchy's bottom line** names `ga` outside the tab and `a`, `d` and
  `esc` inside it.
- **Stop agents here** stops the sessions that are here and withdraws the
  writes waiting here, and leaves a session that is only elsewhere. It sets
  the connection's access to Off and saves it, with the level it had. An
  agent that connects again does not see the connection. The sidebar entry
  reads "Off here", and "Allow agents…" offers the level it had.
- **Windows:** with the server not available, no sidebar entry is drawn,
  Settings has no Agents pane, and none of the commands is in the keymap.

## G. Phases

The brief's three phases are kept as the numbers. Phase 1 is four parts,
since as one it would be too large to review well, and the first two of the
four are several runs each, a plan and a pull request for each run. Phase 3
is two. Each leaves the app safe: nothing listens until 1b, and 1b's server
is off until the person turns it on.

| Part | Runs | State on 2026-10-13 |
|---|---|---|
| 1a. The enforcement core | 3 | Done: pull requests 122, 123 and 124 |
| 1b. The server over stdio, with its interface | 9, below | Built (`2026-10-13-mcp-server-1b-1-server.md`, `2026-10-14-mcp-server-1b-2a-gateway.md`, `2026-10-14-mcp-server-1b-2b-stdio.md`, `2026-10-14-mcp-server-1b-3a-agents-tab.md`, `2026-10-14-mcp-server-1b-3b-changes-outside.md`, `2026-10-14-mcp-server-1b-4a-activity.md`, `2026-10-14-mcp-server-1b-4b-log.md`, `2026-10-15-mcp-server-1b-5a-access.md` and `2026-10-15-mcp-server-1b-5b-settings.md`) |
| 1b on Windows: the way in | 1 | Built (`2026-10-15-mcp-server-1b-6-windows.md`) |
| 1c. Hidden tables | 3, below | The checks built (`2026-10-15-mcp-server-1c-1-checks.md`). The setting and the server are next |
| 1d. HTTP with tokens | 1 | Dropped for now (2026-10-15) |

The move of the connections to `connections.toml` came between 1a and 1b
(pull request 125, `2026-10-13-connections-toml.md`).

**1a. The enforcement core. No server, nothing visible.**

- Runs: three, each its own plan and pull request. The read checker
  (`2026-10-13-mcp-server-1a-1-read-checker.md`), then the drivers
  (`2026-10-13-mcp-server-1a-2-agent-read.md`), then SQLite's
  `Fence::Agent` (`2026-10-13-mcp-server-1a-3-sqlite-fence.md`). The check
  for writes moves to Phase 2, and the size cap on an answer to 1b.
- Files: `crates/tabletist-db/src/agent.rs` and `agent/` (the checker, the
  function lists, the run's limits and answer), `pg/script.rs`,
  `mysql/script.rs`, `sqlite.rs` (parameters, the statement timeout),
  `sqlite/fence.rs` (`Fence::Agent`), `fixtures/` (Bookshop),
  `compose/mysql-init.sql` (its database), `tests/agent.rs`, `Cargo.toml`
  (`sqlparser`), `.github/workflows/ci.yml` (the new test file).
- Risks: binding parameters on PostgreSQL. sqlparser's reading of each
  dialect. A change to `run_script` that the SQL editor feels.
- Tests: F.2 without hidden tables and masking. The existing script suites,
  unchanged and green.

**1b. The server over stdio, access Off and Read, the three read tools, the
log, the Agents tab, and the UI to turn it on. Both looks.**

- The Agents tab (changed on 2026-10-13): the sidebar entry with its five
  states, the ways in but the View menu, the tab with its sessions, "Stop
  session", "Stop agents here" and the feed, which is read-only in this
  phase, the Log view as a plain list, and the three empty states.
- Files: `crates/tabletist-mcp/` (new), root `Cargo.toml`,
  `src/entrypoint.rs` and `src/main.rs` (the `mcp` subcommand),
  `src/backend.rs` (`Command::Mcp`, `Event::Mcp`, the gateway, the cancel
  code shared), `src/connections.rs`, `src/settings.rs`, `src/paths.rs`,
  `src/model.rs`, `src/app.rs`, `src/keymap.rs`, `src/ui/keys.rs`,
  `src/ui/settings/`, `src/ui/picker.rs`, `src/ui/connect_dialog/`,
  `src/ui/agents/` (the tab: Activity, the plain log list, the empty
  states), `src/ui/sidebar.rs` (the entry), `src/ui/object_tabs.rs` (the
  third kind of tab), `src/ui/quick_open.rs`, `src/ui/help.rs`,
  `src/ui/mod.rs`,
  `src/testing.rs`, `tests/cli.rs`, the website pages
  (`_reference/settings-and-files.md`, `connections.md`,
  `keyboard-shortcuts.md`, `_guide/omarchy.md`, `macos.md`, a new
  `_guide/agents.md`).
- Also here, from 1a: the size cap on an answer (B.5), beside the code
  that writes the answer as JSON.
- Risks: the first second process of this app. A second session to a
  database for each connection an agent uses. The settings pane switch. The
  relay on Windows (A.2) is a run of its own now.
- Tests: F.1 in process and over stdio, F.4, F.5, F.7 without the card and
  the count, the UI tests for Settings, Connections and the editor, the
  keymap.
- Notifications for a connect are in this phase on Linux, since it is one
  D-Bus call. macOS gets the attention request.

**1b in five runs** (cut on 2026-10-13). Each is a plan and a pull request,
and each leaves `main` safe to release. The order puts the interface that
lets an agent in last: until the fifth run a connection can be opened to
agents only by a hand that edits `connections.toml`, and by then the person
can see what agents do and stop them.

| Run | What it builds | What a user can do after it | Tests |
|---|---|---|---|
| **1b-1. The server, with nothing to start it** | The `tabletist-mcp` crate: the server on `rmcp`, the gateway's trait, `list_connections`, `describe_schema` and `run_query` over `agent::check_read` and `Connection::run_agent_read`, the size cap on an answer (B.5), the refusal codes (B.8), the log's store (C.1) | Nothing. No setting, no transport and no command reaches it | F.1 in process, F.4 but its interface, F.5 in process |
| **1b-2a. The backend's gateway** (built: `2026-10-14-mcp-server-1b-2a-gateway.md`) | `[connections.agents]` with access, rows and timeout, which makes the connections file version 4. `Command::Agents` and `Event::Agents`, the backend's gateway with its own sessions on each database, and the server started and stopped by the backend, with the log's retention. The SDK held out of the app's log | Nothing: no setting starts the server, and no transport reaches it | The connections file, the gateway on SQLite, PostgreSQL and MySQL, the backend's start and stop, the log's filter |
| **1b-2b. Turning it on from the files, and stdio** (built: `2026-10-14-mcp-server-1b-2b-stdio.md`) | The `[mcp]` keys in `settings.toml`, and the app that starts and stops the server with them and sends the backend what agents may use. The endpoint, the key and the relay, `tabletist mcp`, on Linux and macOS. The keychain on macOS (A.1). The website's reference for the files | With `enabled = true` and an access written by hand, an agent connects over stdio and reads. Nothing in the app shows it yet, and nothing in the app turns it on | F.1 over stdio, F.5 over stdio, the settings file, `tests/cli.rs` |
| **1b-3a. The Agents tab's frame** (built: `2026-10-14-mcp-server-1b-3a-agents-tab.md`) | The third kind of tab. The sidebar entry with its states but Waiting. The ways in: the key, `ga`, `:agents`, quick open. The three empty states, with **Turn on** and **Copy**. The sessions the app learns of, and those that are here. The keymap and the help | Open the tab in any state, see which state the connection is in and who is here, turn the server on from the tab | F.7: the entry's states, the ways in, one tab for a window, the empty states, nothing opening or focusing by itself |
| **1b-3b. Changes made outside the app** (built: `2026-10-14-mcp-server-1b-3b-changes-outside.md`) | C.2: the agent settings the app last showed, kept in the state directory, and a change from outside that makes agents more able held back until the person keeps it | A program that edits the files cannot turn agents on unseen | C.2's cases |
| **1b-4a. What agents are doing** (built: `2026-10-14-mcp-server-1b-4a-activity.md`) | `App::agents`'s feed, from `Server::calls`. The sessions that are here and "Also:". **Stop session** and **Stop agents here** | See each agent and each call as it happens, stop one agent or all on a connection | F.7: one connection, the stops, closing the tab |
| **1b-4b. The log, and a new agent** (built: `2026-10-14-mcp-server-1b-4b-log.md`) | The Log view as a list for this connection, with All connections, and **Show log**. The notification for an agent that connects | Read the log, and hear of an agent that connects | F.7: the log's default. F.4's interface |
| **1b-5a. Letting agents in, per connection** (built: `2026-10-15-mcp-server-1b-5a-access.md`) | The Connections list's Agents column, its menu and Omarchy's box. The connection editor's Agents section, with **Allow agents…** in the Agents tab and **Show Agents tab** in the list. The reference pages | A connection is opened to agents, and closed, without a file edited by hand | The interface tests of Connections and the editor. The keymap. `docs/bin/check` |
| **1b-5b. Settings › Agents** (built: `2026-10-15-mcp-server-1b-5b-settings.md`) | Settings › Agents, with the switch between panes, **Show Agents tab** and **Show log** there, the Agents tab's **Other agents…** and **Settings…**. The website's guide to Agents | Everything Phase 1 draws | The interface tests of Settings. The keymap. `docs/bin/check` |

- **A button arrives with what it opens.** The empty states of the third
  run have **Turn on** and **Copy** and no other action. **Show log** comes
  with the Log view in the fourth. **Allow agents…**, **Other agents…** and
  **Settings…** come in the fifth, with the editor's section and the pane
  they open. Omarchy's keys for them come with them.
- **What may be released between runs.** After any of them. After the
  second the server can be turned on only from a file, off by default. The
  website's reference describes the files and `tabletist mcp` from then on
  (agreed on 2026-10-14): a person who finds the keys in their file can
  read what they do. After the third and fourth a person can turn it on and
  watch it, and still cannot open a connection to agents without editing a
  file. The website's guide to Agents waits for the fifth.
- **Questions the runs meet.** Four are open and not marked as ones to
  wait for. Each is built as this plan has it unless it is answered before
  its run: what the log keeps of a call's values (18) in the first, which
  built it as planned, in full: up to 100 kB of a call's statement and as
  much of its values. Values past that are kept as their start and their
  length, which is still JSON, so that every entry can be read. The name an agent sees for a connection (19)
  in the second, with the gateway that names the connections. The
  notification for a new agent (17) in the fourth, "Stop all agents" (11) in
  the fifth.
- **Sizes.** The first had the risk of `rmcp` and of the first log. The
  gateway's own sessions on each database went to the second run with the
  backend's gateway, which made the second the largest: it is cut in two,
  the gateway and the files (1b-2a), then the settings, the endpoint and
  the relay (1b-2b).
- **"N of M"** (B.5) is a run of its own in `tabletist-db`, at any time
  before the fourth.

**1b on Windows: the way in.** One run, after 1b (question 14): the named
pipe, the relay from the GUI binary or a console target if that fails
(A.2), its test on the Windows CI runner, the taskbar's flash for a
notification, and the function that says the server is available there.
Built (`2026-10-15-mcp-server-1b-6-windows.md`): the relay from the GUI binary, tested on CI's release
build; no console target.

**1c. Hidden tables.**

- Runs: three, each its own plan and pull request, agreed on 2026-10-15.
  The checks in `tabletist-db`, nothing visible (`2026-10-15-mcp-server-1c-1-checks.md`). Then the
  setting in `connections.toml` and the server: its refusals,
  `list_connections` naming the hidden tables, `describe_schema` leaving
  them out, a rename that stops agents, and changes made outside the app
  held (C.3). Then the editor's Tables control in both looks, its hint,
  and the website.
- Files: `crates/tabletist-db/src/catalog.rs`, `pg.rs`, `mysql.rs`,
  `sqlite.rs` (what views read, routines, partitions), `agent/` (names,
  dependencies, the plan check, the system catalogs), `tabletist-sqlite-ffi`
  only if the authorizer needs more than it reports, `src/connections.rs`,
  the editor's Tables control, `tabletist-mcp` (`describe_schema`, the
  stale-entry stop).
- Risks: the plan's JSON across server versions and MariaDB. Views on
  MySQL without `VIEW_TABLE_USAGE`. This is where "not guaranteed" has to
  be written honestly.
- Tests: the hidden-table rows of F.2, C.3's changes, the hint's wording in
  both looks.
- Until this ships, the editor has no Tables control: a control that hides
  by name alone would promise more than it does.

**1d. HTTP with tokens.** Its own pull request, after stdio (settled,
question 9). **Dropped on 2026-10-15, for now:** not planned or built.
What follows is the design, kept for if it is taken up again.

- Files: `tabletist-mcp` (the listener, the checks, tokens),
  `Cargo.toml` (`hyper`, `hyper-util`, the rmcp feature), `src/paths.rs`
  (`agents.json`), Settings › Agents ("Other", the agent list, Remove), the
  status row.
- Risks: the largest new dependency set. The first network listener in the
  app.
- Tests: F.1 over HTTP, the removal test, the `listen` validation.

**2. `propose_write`, the approval, notifications, Read + approved writes.**

- Files: `crates/tabletist-db/src/agent/` (the write rules, the count),
  `script.rs` and each driver's `script/write.rs` (the check before
  `COMMIT`), `tabletist-mcp` (the broker, progress, the queue, the rate
  limit), `src/backend.rs`, `src/model.rs` (the requests in `AgentsState`),
  `src/app.rs`, `src/keymap.rs` (`When::AgentWrite` in the `Agents` scope),
  `src/ui/agents/` (the card in the tab, the note, the preview),
  `src/ui/sidebar.rs` and `src/ui/object_tabs.rs` (the count),
  `src/ui/workspace.rs` (the bottom line),
  `crates/tabletist-appkit` (the macOS notification), the access controls
  (the third level), the website.
- Risks: client timeouts shorter than the wait. The macOS notification's
  permission and signing. The first write that does not start with the
  person.
- Tests: F.3. The write rows of F.2. The card, the count and the keys of
  F.7. By hand: Claude Code and Cursor waiting five minutes, each look's
  notification.

**3a. Masked columns.**

- Files: `agent/` (the rule, the resolver), each driver (where a result
  column comes from), `tabletist-sqlite-ffi` (the column origin, and the
  authorizer's "ignore"), the bundled SQLite's build options, the editor's
  Masked columns control, `describe_schema`, the answer's marking.
- Risks: the resolver is the piece most likely to be wrong. The decision
  between A alone and A over B is made here, on the matrix.
- Tests: the masking rows of F.2, for each driver.

**3b. The Log view, in full.** (This was a log window before 2026-10-13.)

- Files: `src/ui/agents/log.rs` (the agent and result filters, the search,
  the detail pane), `tabletist-mcp` (the queries, export), `src/app.rs`
  ("Open in SQL editor").
- Risks: small.
- Tests: the rest of F.4, the UI tests of the filters.

## Open questions

The brief's four, each with what the code suggests, then the plan's own.
The ones Phase 1 waits for are marked.

1. **Should the server also run without the app window, as a background
   helper?** Not in v1. The app has no single-instance handling and no
   helper process, the sessions and the keyring live in the backend thread
   of the window's process, and approval needs the window. A helper would
   be a second program holding every credential. Revisit after Phase 2,
   when the question becomes "should closing the last window keep the
   server?".
2. **Per-agent policies: v1 or later?** Later. Only HTTP agents have an
   identity that means anything (A.5): a stdio agent names itself. A policy
   by name would be a policy anyone can pick. The log and the Agents tab already
   keep the names, so nothing is lost by waiting.
3. **Can production be set above Read, and with what extra confirm?** Not
   until the app can confirm a script on production, which it cannot today
   (B.7). Then: yes, with the existing production confirmation after the
   approval, never instead of it.
4. **Approvals from the phone, or only on this computer?** Only in the app,
   on this computer. A notification cannot show the SQL, the preview or the
   warnings, and approving without them is the thing the card exists to
   prevent. The notification's one action is "Review".
5. Settled on 2026-10-13: `connections.toml`, as a run of its own before
   Phase 1b (`2026-10-13-connections-toml.md`). A connection is found by
   its id, not by its name as the boards draw it.
6. Settled on 2026-10-13, by the designs: neither windows, as the boards
   had it, nor a tab of the app, as this plan had it. Each connection window
   has an Agents tab, with Activity and Log in it (D.4, D.5).
7. The count on the approval card: a read-only `SELECT`, with a dry run
   only when the person asks. Accepted, against the board's "Counted by a
   dry run"? (Phase 2.)
8. Settled on 2026-10-15: yes, as the boards show. The refusal names the
   hidden table, and `list_connections` lists the names (1c). Answering as
   for a table that does not exist would leave the agent guessing, and the
   names are not what is hidden: the rows are.
9. Settled on 2026-10-13: HTTP comes after stdio, as its own pull request
   (1d). Both agents the boards name connect over stdio, and HTTP brings
   the app's first network listener, the tokens and at least nine new
   crates. Dropped on 2026-10-15, for now: 1d is not planned (G).
10. Settled on 2026-10-13: `tabletist mcp` when the app is not running
    answers with an error and exits, as planned. No `--launch` flag in
    Phase 1, and no relay that stays up and waits (A.2 says why).
11. "Stop all agents" switches the server off until it is turned on again.
    Built so in Phase 1b's fifth run (`2026-10-15-mcp-server-1b-5b-settings.md`): it is the switch in
    Settings › Agents.
12. Omarchy: `a` and `d` are keys of the Agents tab alone, and in any other
    tab the bottom line ends `ga agents` where the board has `a approve  d
    deny  esc later`. Accepted? The connections screen needs no `:` prompt
    any more. (Phase 2.)
13. Settled on 2026-10-15: open on a connection that hides nothing, closed
    on one that hides or masks something (`system_catalog`, B.3). Built for
    hidden tables in 1c's first run (`2026-10-15-mcp-server-1c-1-checks.md`); masks are Phase 3a's.
14. Settled on 2026-10-13: the Windows way in (the named pipe, the relay
    from a GUI binary) follows the other two by one release, as a run of
    its own. The interface is built and tested on Windows with the others,
    and shown there when the way in is (A.2, G).
15. Settled by the work: the Bookshop fixture for the three drivers is in
    `crates/tabletist-db/fixtures/bookshop/` since Phase 1a's second run
    (pull request 123), and `tests/agent.rs` runs on it.
16. Settled on 2026-10-13: this file lives in `docs/superpowers/plans/`,
    which the website leaves out (`docs/_config.yml:113-115`). Under
    `docs/plans/` Jekyll would have copied it to the site as it is.
17. Settled on 2026-10-14, as planned: a notification (the fourth run's
    second part). A new stdio agent: a notification, as planned, or a
    question the first time a name is seen ("Let Claude Code connect?")?
    The second is a real gate for stdio, which has none.
18. Should the log keep parameters in full? They can hold personal data.
    The alternative keeps their types and lengths.
19. Settled on 2026-10-14, otherwise than planned (`2026-10-14-mcp-server-1b-2a-gateway.md`): a connection
    is its name and environment ("Bookshop · dev"). Where two saved
    connections would be called the same, Off or not, each is called that
    with the start of its id after it ("Bookshop · dev (1f0c2a9e)"). The
    planned " (2)" moved: turning the first Off made the second "Bookshop ·
    dev", and an agent's next call reached another database. A name now
    means one database for as long as the connections called the same are
    the same ones: turning one Off or on changes no name. When one of them
    is added or removed, the others' names change with it (the review of
    the pull request found that an add and then a remove gave the bare
    name to another), and an agent learns them from `list_connections`.
20. An `UPDATE` or `DELETE` without a `WHERE`: a warning on the card, as
    planned, or refused outright for agents?

The next six come from the Agents tab (2026-10-13). Five were settled the
same day. Question 25 is open, and is Phase 2's.

21. Settled on 2026-10-13: "Stop agents here" holds. It ends the sessions
    that are here and sets the connection's access to Off, keeping the level
    it had for "Allow agents…" to offer (A.5). The boards' "Off here" is the
    state, so none is new.
22. Settled on 2026-10-13, as planned: an agent that has just connected is
    under "Also:" in every window's tab until a call names a connection,
    its notification goes to the frontmost connection window, and "Stop
    session" stops a session wherever it works.
23. Settled on 2026-10-13, as the board has it: "Turn on" in the empty state
    starts the server at once. The button is the confirmation, and it
    changes no connection's access.
24. Settled on 2026-10-13: View › Agents waits for a View menu. Every other
    way in is built in Phase 1b.
25. A write waits on a connection that has no window open, and the
    notification was missed, or notifications are off. Planned: the count in
    the Connections list's Agents column, and "Also:" in any Agents tab that
    is open. The boards draw neither. (Phase 2.)
26. Settled on 2026-10-13: with the server off and this connection's access
    Off, the sidebar entry says "Server off", and the tab shows that empty
    state.

## Sources

- `rmcp` 3.5.1, read from its published source: `README.md`, `Cargo.toml`
  (features), `src/model.rs` (`ProtocolVersion`), `src/model/tool.rs`,
  `src/model/task.rs`, `src/model/mrtr.rs`, `src/service.rs`
  (`RequestContext`), `src/transport/async_rw.rs`,
  `src/transport/streamable_http_server/tower.rs`
  (`StreamableHttpServerConfig`). <https://crates.io/crates/rmcp>
- The MCP specification, transports, for the `Origin` rule and the session
  header: <https://modelcontextprotocol.io/specification/latest/basic/transports>
- `sqlparser` 0.63.0, read from its published source: `Cargo.toml`
  (features), `src/dialect/`, `src/ast/visitor.rs`, `src/ast/spans.rs`,
  `src/ast/mod.rs` (`Statement`). <https://crates.io/crates/sqlparser>
- `tokio-postgres` 0.7.18 (`src/query.rs`, `src/statement.rs`),
  `mysql_common` 0.37 (`src/packets/mod.rs`), `rusqlite` 0.37 (features),
  from the versions in `Cargo.lock`.
- In this repository:
  `docs/superpowers/specs/2026-09-30-sql-editor-core-design.md` (the
  read-only guard), `2026-10-05-sql-editor-writes-design.md` (the write
  run), `2026-10-08-keyboard-shortcuts-design.md` (the keymap),
  `2026-10-06-row-inspector-inline-edit-design.md` and
  `2026-10-07-inserting-rows-design.md` (the grid's write path and its
  conflicts).

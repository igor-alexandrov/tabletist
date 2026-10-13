# Plan: the Tabletist MCP server

Status: a plan for review. No app code changes with it.

Written on 2026-10-13 against `main` at `89a7e88`. Line numbers are from that
commit. Designs: the "Agents · MCP server" section of the Tabletist canvas
(nine boards). They are not copied into the repository.

Each phase below gets its own plan under `docs/superpowers/plans/` before any
code is written, as every batch of work in this repository does.

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
| 3 | An activity window and a log window | One OS window. No `show_viewport` call anywhere. Settings, About and Help are modal dialogs in the root viewport | Activity and Log are app tabs, a third `ConnTabContent` (`src/model.rs:25-30`), not windows and not modal dialogs. A modal would block the app while a feed updates (question 6) |
| 4 | macOS: the Agents pane and the Log are reached from a menu | The app builds no menu. winit's default app menu is patched for About and Settings through `crates/tabletist-appkit` (`src/macos.rs:133-200`) | Reached from Settings › Agents ("Activity…", "Open log"), from a header chip while an agent is connected, and from the notification. Two app-menu items can follow through `tabletist-appkit`, as Settings did |
| 5 | Omarchy: `:agents` and `:agents log` | The `:` prompt exists only in a workspace (`src/ui/keys.rs:1804-1814`, `App::run_command`, `src/app/editing.rs:1351`). The connections screen has none, and the keymap looks an ex command up by scope | Both commands answer in the Grid and SQL editor scopes. The connections screen gets the `:` prompt too, since agent access is set there (question 12) |
| 6 | Omarchy status line: `-- AGENT WRITE -- … a approve d deny esc later` | Single letters are safe only in the `Prompt` scope, which is a dialog (`keys::prompt_letter`, `src/ui/keys.rs:47`). Outside a dialog, `a` in normal mode would approve a write for anyone whose fingers know vim's append | The status line announces the request and names `:agents`. `a` and `d` act only inside the approval box, which the person opens (question 12) |
| 7 | "Counted by a dry run, rolled back", before the person is asked | Nothing in the app runs a write to count it. A dry run on a MySQL table that is not transactional is a real write (`rollback_warning`, error 1196, is how the script run reports it) | Count with a read-only `SELECT count(*)` built from the statement. A dry run becomes something the person asks for on the card (B.6, question 7) |
| 8 | Production: "Can it ever be above Read?" | A SQL tab on production cannot write at all today: `Workspace::sql_writes` answers `NoWrites::Unconfirmed` because the question for a script is not built (`src/model.rs:2840-2848`) | Production is capped at Read until that question exists. Then approved writes on production pass both the approval and the production confirmation |
| 9 | "Use only the Bookshop demo data in fixtures and tests" | There is no Bookshop database. The SQL fixtures are `users`, `orders`, `events`, `big` (`crates/tabletist-db/fixtures/`). Bookshop exists as in-memory scenes for screenshots (`src/shots.rs`) | Phase 1a adds a small Bookshop SQL fixture for the three drivers, with the tables the boards use (question 15) |
| 10 | "macOS and Omarchy get the same features" | There are three looks: Windows shares the macOS code paths (`Look::standard`, `src/theme.rs:390`), and the release builds Windows packages | Windows gets the macOS variant with Ctrl for Cmd, a named pipe for the shim, and a taskbar flash where the others notify (question 14) |
| 11 | `list_connections` shows `hidden: [customers]`, and the refusal says "customers is hidden" | The editor's hint says hidden tables "don't exist for the agent, not even in the schema" | Kept as drawn: the name of a hidden table is told to the agent, its columns and rows are not. Otherwise the agent cannot act on the refusal (question 8) |
| 12 | The "…" menu has "Copy URL"; the row shows a "read-only" tag; the editor is an accordion | The menu has Duplicate and Delete (`src/ui/picker.rs:983-999`). The list shows no read-only tag. The editor has fieldsets | Out of scope. This work adds the Agents column, the Agents menu entry and the Agents fieldset to the screens as they are |

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
`connection_unavailable`, and Activity says why.

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
   key, and its working directory, which gives Activity its project name
   ("tabletist-repo") when the client does not offer MCP roots.
3. From then on it copies bytes both ways and understands none of them. The
   app serves MCP on that connection. One connection is one session.
4. When either side closes, it exits.

So the protocol lives in one place, the app, and an old relay keeps working
with a newer app.

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
  who want it (question 10).

What this costs: an agent started before Tabletist shows the server as
failed until the person reconnects it. A relay that stays alive and waits is
possible, since the tool list is the same for every user, but it has to
speak MCP itself. Left for after Phase 1.

**Windows.** The release binary is built for the GUI subsystem
(`src/main.rs:3`). A GUI program still reads and writes the pipes its parent
hands it, which is how agents start servers, but this is only reasoned here,
not run. If it fails, the fix is a second, console binary target for the
relay. Phase 1b tests it on the Windows CI runner before the command is
shown in Settings on Windows.

### A.3 HTTP

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
- The name the person gave is the agent's name in Activity and in the log.
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
| Cost | New crates. Stdio: `rmcp`, `schemars` and `pastey` and their few dependencies; `tracing`, `chrono`, `tokio-util`, `futures`, `thiserror` and `uuid` are in `Cargo.lock` already. HTTP: `hyper`, `hyper-util`, `http`, `http-body`, `http-body-util`, `httparse`, `tower-service`, `sse-stream` and `tokio-stream`, none of which is in the tree today | No new crate for stdio. hyper and friends for HTTP all the same |
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

**Tool annotations.** `readOnlyHint: true` on `list_connections`,
`describe_schema` and `run_query`. `destructiveHint: true` on
`propose_write`. They are hints for the client's own prompts. Nothing here
relies on them.

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
    state: State,                  // Idle | Running(call) | WaitingForYou(request)
    stop: CancellationToken,       // parent of every call in flight
}
```

- A stdio session is one connection from the relay: it begins when the
  relay connects and ends when either side closes.
- An HTTP session is a token in use. It leaves Activity after thirty minutes
  without a call. Nothing is enforced through it.
- The agent's name is whatever the client says, in `initialize` or in a
  request's `_meta`. It is a label, not an identity. Only an HTTP token's
  name is something the person chose.

**Stopping.** "Stop session" cancels the session's token.

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

"Stop all agents" switches the server off, the same as the toggle in
Settings: every session is stopped, the endpoint and the port close, and
nothing reconnects until the person turns it on. Stopping sessions and
leaving the server up would give agents back their connection in the second
it takes to reconnect (question 11).

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
| PostgreSQL | `SET LOCAL statement_timeout = <ms>` and `SET LOCAL lock_timeout = <ms>` | Local to the transaction. The agent cannot change them: every `SET` and `set_config` is refused |
| MySQL | `SET SESSION max_execution_time = <ms>` | Applies to `SELECT`, which is all a read runs. MariaDB: `SET SESSION max_statement_time = <seconds>`. The cleanup's reset puts it back |
| SQLite | A deadline in the progress handler the script run already installs | `sqlite.rs:192`. In process, so it is exact |

A statement the server stops is `timeout`, and its transaction is rolled
back like any other.

### B.2 Parsing

**Parser: `sqlparser` 0.63** (`sqlparser-rs`, Apache), with
`PostgreSqlDialect`, `MySqlDialect` and `SQLiteDialect`, chosen by a `match`
on `Dialect` that names every variant, as `AGENTS.md` requires. It gives a
syntax tree, a visitor over every relation and expression (`visit_relations`,
`visit_expressions`, the `visitor` feature) and source positions. It adds two
crates, `sqlparser` and `sqlparser_derive`. Its default `recursive-protection`
feature would add `recursive` and `stacker`, which build a little assembly;
it can stay off, since the parser's own depth limit (`with_recursion_limit`)
already refuses a statement nested too deep to walk.

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
  `query_to_xml*`, `table_to_xml*`, `cursor_to_xml*`, `nextval`, `setval`,
  `pg_notify`, `pg_reload_conf`, `pg_logical_*`, `pg_replication_*`. MySQL:
  `SLEEP`, `BENCHMARK`, `GET_LOCK`, `RELEASE_LOCK`, `RELEASE_ALL_LOCKS`,
  `LOAD_FILE`, `SOURCE_POS_WAIT`, `MASTER_POS_WAIT`, and assignment to a user
  variable. SQLite: `load_extension`, and every table-valued `pragma_*`.
- **Functions the database's users wrote**, which the checker cannot read:
  allowed on a connection that hides and masks nothing, refused on one that
  does. Telling them apart needs the names of the database's routines, which
  the catalog does not read today.

A list of bad functions is never complete: an extension adds more. What it
misses still runs in the read-only transaction, under the timeout, as the
connection's user. On SQLite the authorizer sees every function call after
SQLite's own parse, so there the list does not depend on our parser.

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
| PostgreSQL | The statement is prepared first, as today, which gives each parameter's type. Each `$n` token (a token of `sql::tokenize`, never a text search) is then replaced by a literal from the writer that filters and saves already use, cast to that type | Rows are read as text through the simple-query protocol. tokio-postgres's other protocol always asks for binary results (`Some(1)` in its `query.rs`), and the crate has no binary decoders. `Dialect::bind` already inlines literals on PostgreSQL for this reason (`dialect.rs:301-309`), with `standard_conforming_strings = on` set at connect (`pg.rs:182`) and guarded by the refusal list |

The log keeps the agent's text and its parameters, not the inlined text.

### B.3 Hidden tables

The connection's setting is one of three: **All**, **All except** a list, or
**Only** a list (the board's segmented control). "Only" hides every table
that is not named, the ones made later too. An entry without a schema
matches the table of that name in every schema.

A reference is looked for in four places:

1. **The statement's names.** `visit_relations` gives every name in a table
   position, at any depth: joins, subqueries, CTE bodies, `LATERAL`, set
   operations. Each is resolved against the catalog and compared, without
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

- A cell longer than 2,000 characters is cut and marked, with its full
  length.
- Binary values are described ("binary, 20,480 bytes"), not sent.
- An answer is at most 256 kB as JSON. Past that, rows are dropped from the
  end and the answer says it was cut by size.

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
| The connection was lost mid-write | `write_unknown`, "The connection was lost. Tabletist cannot say whether it was written." The log and Activity say the same. The run's own error is this honest today |
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
| `not_read_only` | run_query only runs reads. Use propose_write for DELETE FROM book_covers WHERE id = 9. | A write, `INTO`, `FOR UPDATE` |
| `not_allowed` | CREATE TABLE is not available to agents. | DDL, `COPY`, `SET`, `DO`, transaction control, upserts, `RETURNING` and the rest |
| `not_a_write` | propose_write takes one INSERT, UPDATE or DELETE. Use run_query for a SELECT. | |
| `write_not_allowed` | Bookshop · staging is read-only for agents. | `propose_write` on a Read connection |
| `table_hidden` | customers is hidden for agents on this connection. | B.3. Also for a view that reads it: "recent_customers reads customers, which is hidden…" |
| `system_catalog` | The system catalogs are not available on this connection. Use describe_schema. | B.3 |
| `masked_column_used` | orders.email is masked. It can be selected, not filtered, joined, sorted or computed on. | B.4 |
| `function_not_allowed` | pg_sleep is not available to agents. | B.2 |
| `invalid_params` | The statement has 2 parameters and 1 was sent. | Count, or a value that is not a string, number, boolean or null |
| `too_large` | The statement is larger than 100 kB. | |
| `timeout` | The query was stopped after 10 s, this connection's limit. | The statement timeout |
| `busy` | Another agent's query is running on this connection. Try again. | A.1 |
| `rate_limited` | Too many calls. Try again in 20 s. | E.5 |
| `query_failed` | The database's own error, with its code. | A syntax error, a missing column. Without the detail and hint lines on a connection that hides or masks something |
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
| An unknown tool, a missing argument, an argument of the wrong type | -32602 | 200 |
| An unknown method | -32601 | 200 |
| Not JSON | -32700 | 400 |
| No token, or an unknown one | | 401 |
| `Host` or `Origin` | | 403 |
| A wrong path, method, content type or size | | 404, 405, 415, 413 |
| The server is stopping | | 503 |

## C. Data and settings

### C.1 Storage

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
  message       TEXT,               -- the refusal's sentence, or the database's message
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

**The app's own log is another thing.** `tabletist.log` never holds SQL
(`AGENTS.md`, and the editor spec's "The SQL text is never logged"). That
stays true. The MCP code logs a tool's name, a code and a duration there,
nothing more.

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
| macOS and Windows: the nav lists two items, not the one hard-coded "General". The Agents pane: the toggle, the status row, "Connect an agent" with the three-way switch and Copy, the Access box with "Open Connections", "How it stays safe", the two Log rows | `src/ui/settings/sheet.rs:166-238`, a new `src/ui/settings/agents.rs` |
| Omarchy: the same nav, the rows with the cursor, `y` to copy the command, `enter` on the access row to open Connections, and the file pane showing `[mcp]` | `src/ui/settings/terminal.rs:220-236`, `file_pane.rs:195-313`, `mod.rs:126-239` for the keys |
| The settings keys | `src/settings.rs` (C.1) |
| The command to copy. macOS: the path of the running binary inside the app bundle (`std::env::current_exe`). Linux: `tabletist mcp` when `tabletist` is on the `PATH`, else the full path | New, in `tabletist-mcp` |
| The list of agents under "Connect an agent": stdio sessions with Stop, HTTP agents with Remove, "Add an agent…" under Other for a token | Not drawn on the board (A.3) |

The board's other panes (Appearance, Editor, Keyboard, AI assistant,
Updates) are not built by this work.

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

### D.4 Activity, the approval card, notifications

Boards: `macOS – Agents (MCP), activity and approval`, `Omarchy – Agents
(MCP), activity, approval and log`.

**Activity is a tab.** `ConnTabContent` gains `Agents(Box<AgentsTab>)`
beside `Picker` and `Workspace` (`src/model.rs:25-30`), drawn by a new
`src/ui/agents/` with `mod.rs`, `sheet.rs` and `terminal.rs`, as Settings
and the connection editor are split. One such tab at most: opening it again
shows it.

| What | Where |
|---|---|
| The sessions column, the selected session's header with "Stop session", the call feed, "Open log", "Stop all agents" | `src/ui/agents/` |
| The header chip that opens the tab, shown while an agent is connected, with a count when a write waits | The connection bar in `src/ui/mod.rs` |
| `Action`s: `ShowAgents`, `ShowAgentLog`, `StopAgentSession`, `StopAgents`, `ReviewAgentWrite`, `ApproveAgentWrite`, `DenyAgentWrite`, `PreviewAgentWrite` | `src/model.rs:47` |
| `Command::Mcp` and `Event::Mcp` | `src/backend.rs:52`, `:209` |

**The approval card never takes the keyboard by itself.** A request does
three things: the notification, the chip's count, and on Omarchy the status
line's "-- AGENT WRITE -- claude code · 3 rows in Bookshop · dev · :agents".
It does not open a dialog over what the person is doing. `prompt_password`
is the precedent for not replacing a dialog the person is working in
(`src/app.rs:2009`).

The person opens the review: the notification, the chip, `:agents`, or the
card's place in the Activity tab. That opens `Dialog::AgentWrite`, which is
in the `Prompt` scope with a new `When::AgentWrite`:

- the SQL as it will run, the agent's reason, the count and how it was
  made, "Runs as" with the connection's user, "One transaction, then
  commit", the preview link, the cascade and no-`WHERE` warnings (B.6);
- macOS: **Deny…** and **Approve and run**. Deny opens a one-line note
  field;
- Omarchy: `[a] approve and run  [d] deny with a note  [p] preview rows
  [esc] decide later`;
- `esc` closes the box and leaves the request waiting. The card stays in
  the Activity tab with the time left;
- one request is shown at a time, the oldest first (E.5);
- a request that was withdrawn or timed out while the box is open says so
  in the box and offers only Close.

`ApproveAgentWrite` looks the request up again when it is applied, as
`ConfirmSqlRun` re-checks the tab's mode: a card left on screen cannot
approve a request that is gone or a connection that has since become
read-only.

**Notifications.** There are none in the app today.

| Platform | How | Notes |
|---|---|---|
| Linux (mako on Omarchy) | `org.freedesktop.Notifications.Notify` over D-Bus with `zbus`, which is in the tree through the keyring store. One action, "Review" | A direct dependency on a crate already built. No `notify-rust` |
| macOS | `UNUserNotificationCenter`, in `crates/tabletist-appkit`, the only crate allowed `unsafe` for AppKit | Needs the signed app bundle and the person's permission. In an unsigned build: the Dock icon bounces (`ViewportCommand::RequestUserAttention`) |
| Windows | The taskbar button flashes | A toast needs an app identity the installer does not register yet |

- "Review" brings the window forward and opens the box. Nothing can be
  approved from a notification: the SQL is not on it.
- The text holds the agent's name, the count and the connection's name. No
  SQL, since notifications are kept in a history other programs can read.
- `mcp.notify` turns off the notifications for a connect and for a write
  request. The chip and the status line stay.
- An agent connecting is one notification for each session, not for each
  call.

### D.5 The log

Boards: `macOS – Agents (MCP), log`, and the lower half of the Omarchy
activity board.

The log is the Activity tab's second view (`AgentsTab::view: Activity |
Log`), not another window.

| What | Where |
|---|---|
| The table: time, agent, connection, tool, request, rows, ms, result | `src/ui/agents/log.rs`. The grid's row drawing in `src/ui/grid.rs` is for database values. A plain list is enough here |
| Filters: agent, connection, result, and a search of the SQL. They are a query to the backend, `Command::Mcp(QueryLog { filter, before, limit })`, answered in pages | `src/backend.rs` |
| The detail pane: the SQL, "Ran as", "Matched", "Returned to agent", "Masked", "Took", and the note that rows are not kept | |
| "Open in SQL editor": opens a SQL tab on that connection with the statement, in Read-only mode, not run. A connection that was deleted since disables it | `Action::NewSqlTab` and the workspace opening in `src/app.rs` |
| "Export…" | C.1 |
| Omarchy: `/` filter, `enter` details, `o` open in a SQL tab | The keymap |

Phase 1b ships the list with the detail line. Phase 3b adds the filters, the
detail pane, the search, export and "Open in SQL editor".

### D.6 The keymap

Checked against `src/keymap.rs` and the keyboard brief
(`docs/superpowers/specs/2026-10-08-keyboard-shortcuts-design.md`, the
brief's `hotkeys-fix-prompt.md`). A key is spelled in the keymap and nowhere
else, and `tests/keys.rs` fails on one spelled in a view.

| Command | Scope, when | macOS | Omarchy | Checked against |
|---|---|---|---|---|
| Approve agent write | `Prompt`, `When::AgentWrite` | `cmd+enter` | `a` | `cmd+enter` is Run statement in `SqlEditor` (`:1279`), Apply in `CellEditor` when `LargeEditor` (`:1128`), Save and connect in `Prompt` when `ConnectionForm` (`:1486`). None shares this `when`, and `Prompt` is alongside nothing (`Scope::alongside`, `:96-106`). Plain `a` is bound nowhere |
| Deny agent write | `Prompt`, `When::AgentWrite` | the button | `d` | `d` is Discard in `Prompt` when `Leaving` (`:1451`) and when `Conflict` (`:1462`). Another `when`, so no clash |
| Preview rows | `Prompt`, `When::AgentWrite` | the link | `p` | Free in `Prompt` |
| Decide later | `Prompt`, `When::AgentWrite` | `esc` | `esc` | As every prompt |
| Change agent access | `Connections` | the "…" menu | `a` | The scope has `e`, `n`, `yy p`, `dd`, `/`, `enter` (`:828-870`). Nothing begins with `a` |
| Agent activity | `Global` | none (Settings, the chip) | `:agents` | Needs an ex command outside `Grid` and `SqlEditor`: `Keymap::ex` is asked with one of those two today |
| Agent log | `Global` | none | `:agents log` | The same |
| Stop session | a new `Agents` scope | the button | `S` | The scope is new. It sits alongside `Global`, whose keys are chords |
| Stop all agents | `Agents` | the button | `X` | |
| Open in SQL tab | `Agents` | the button | `o` | |
| Copy the command | `Prompt`, `When::SettingsScreen` | the button | `y` | The settings screen has `j`, `k`, `h`, `l`, `space`, `R`, `ctrl+e`, `esc` |

The brief's proposals hold, with two conditions the code sets:

1. `a` and `d` for a write are safe only in a dialog. This is row 6 of
   section 2.
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
| **A malicious local process on 127.0.0.1.** Another program of the same user, or a web page in the person's browser | The port answers only with a token, which is 256 random bits and kept as a digest. A web page is refused before that: any `Origin` is refused, `Host` must be the loopback name, no CORS header is sent, so DNS rebinding and a plain cross-site `fetch` both fail. The socket is 0600 in a 0700 directory, and the relay must present the key | A program running as the same user can read the key file and start `tabletist mcp` itself. It is then an agent like any other: it appears in Activity, a notification says it connected, it gets the same policy, and it cannot write without the person. It could also read the token from another agent's configuration file. Nothing local can be kept from the account that owns it |
| **Another user on the same machine** | Unix: the socket's and the directory's modes, and the state directory is private (`AppDirs::ensure`, `src/paths.rs:57-60`). Windows: the pipe refuses remote clients, and the key file is in the user's profile. The TCP port is open to every local user, and answers none of them without a token. Tokens and the log are 0600 | A user with administrator rights, as on any machine. Another user can see that the port is open |
| **An agent that exfiltrates through errors or timing** | Errors about connecting are replaced by one fixed sentence, so no host, user or path is sent. A refusal is made from the policy, before the database is asked, so it takes the same time whatever the data is. On a connection that hides or masks something, a database error loses its detail and hint lines, where PostgreSQL quotes rows. Masked columns cannot stand in a predicate, a sort or a function, so there is no yes-or-no question to time. The system catalogs and their sample values are closed there. The statement timeout and the rate limit bound how much can be asked | The name of a hidden table (by design). Whether rows exist and how many, on any table the agent may read. A function or a view in the database that the checks cannot see into (B.3). An error message whose main line quotes a value. A timing channel through such a function |
| **An agent that floods `propose_write`** | One request waits for each session. A second one from the same session is refused at once with `busy`. Across sessions, requests queue and the person sees one card at a time, oldest first, with "2 more waiting". At most 6 requests a minute and 30 an hour for each session, then `rate_limited`. After three denials in a row from one session, the next request needs the person to open Activity: there is no notification for it. "Stop session" and "Stop all agents" are on the same screen. One notification is shown for a session's requests, updated in place | A patient agent can still ask thirty times an hour. The person can turn the connection to Read, which ends it. Wearing a person down until they approve is a human risk that a count and a clear card reduce and do not remove |

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
SQLite runs everywhere. PostgreSQL and MySQL run where the suite has a
server, under `TABLETIST_TEST_PG_URL` and `TABLETIST_TEST_MYSQL_URL`, and
print "skipped" without one, as the existing suites do. `ci.yml` names the
test files its PostgreSQL and MySQL jobs run, so each new file is added to
both.

### F.1 An MCP client harness, over both transports

- The harness starts the server with a `Gateway` over the SQLite fixture, a
  scripted approver and a clock it controls, and gives the test a client.
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
- **UI:** with `Backend::recording()`, a request event shows the chip and,
  on Omarchy, the status line, and opens no dialog. "Review" opens the box.
  `cmd+enter` (macOS and Windows) and `a` (Omarchy) send the approval.
  `a` typed in a grid while a request waits sends nothing.

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
  connection Off.

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

## G. Phases

The brief's three phases are kept as the numbers. Phase 1 is four pull
requests, since as one it would be too large to review well. Phase 3 is
two. Each leaves the app safe: nothing listens until 1b, and 1b's server is
off until the person turns it on.

**1a. The enforcement core. No server, nothing visible.**

- Files: `crates/tabletist-db/src/agent.rs` and `agent/` (the checker, the
  function lists, the agent policy type), `sql.rs` (parameter tokens),
  `script.rs`, `pg/script.rs`, `mysql/script.rs`, `sqlite.rs` (parameters,
  the statement timeout, the size cap), `sqlite/fence.rs` (`Fence::Agent`),
  `fixtures/` (Bookshop), `tests/agent.rs`, `Cargo.toml` (`sqlparser`),
  `.github/workflows/ci.yml` (the new test file).
- Risks: the parameter inlining on PostgreSQL. sqlparser's reading of each
  dialect. A change to `run_script` that the SQL editor feels.
- Tests: F.2 without hidden tables and masking. The existing script suites,
  unchanged and green.

**1b. The server over stdio, access Off and Read, the three read tools, the
log, and the UI to turn it on. Both looks.**

- Files: `crates/tabletist-mcp/` (new), root `Cargo.toml`,
  `src/entrypoint.rs` and `src/main.rs` (the `mcp` subcommand),
  `src/backend.rs` (`Command::Mcp`, `Event::Mcp`, the gateway, the cancel
  code shared), `src/connections.rs`, `src/settings.rs`, `src/paths.rs`,
  `src/model.rs`, `src/app.rs`, `src/keymap.rs`, `src/ui/keys.rs`,
  `src/ui/settings/`, `src/ui/picker.rs`, `src/ui/connect_dialog/`,
  `src/ui/agents/` (Activity, the plain log list), `src/ui/mod.rs`,
  `src/testing.rs`, `tests/cli.rs`, the website pages
  (`_reference/settings-and-files.md`, `connections.md`,
  `keyboard-shortcuts.md`, `_guide/omarchy.md`, `macos.md`, a new
  `_guide/agents.md`).
- Risks: the relay on Windows (A.2). The first second process of this app.
  A second session to a database for each connection an agent uses. The
  settings pane switch.
- Tests: F.1 in process and over stdio, F.4, F.5, the UI tests for the
  three screens, the keymap.
- Notifications for a connect are in this phase on Linux, since it is one
  D-Bus call. macOS and Windows get the attention request.

**1c. Hidden tables.**

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

**1d. HTTP with tokens.**

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
  limit), `src/backend.rs`, `src/model.rs` (`Dialog::AgentWrite`),
  `src/app.rs`, `src/keymap.rs`, `src/ui/agents/` (the card, the box, the
  preview), `src/ui/workspace.rs` (the status line),
  `crates/tabletist-appkit` (the macOS notification), the access controls
  (the third level), the website.
- Risks: client timeouts shorter than the wait. The macOS notification's
  permission and signing. The first write that does not start with the
  person.
- Tests: F.3. The write rows of F.2. By hand: Claude Code and Cursor
  waiting five minutes, each look's notification.

**3a. Masked columns.**

- Files: `agent/` (the rule, the resolver), each driver (where a result
  column comes from), `tabletist-sqlite-ffi` (the column origin, and the
  authorizer's "ignore"), the bundled SQLite's build options, the editor's
  Masked columns control, `describe_schema`, the answer's marking.
- Risks: the resolver is the piece most likely to be wrong. The decision
  between A alone and A over B is made here, on the matrix.
- Tests: the masking rows of F.2, for each driver.

**3b. The full log.**

- Files: `src/ui/agents/log.rs` (filters, the search, the detail pane),
  `tabletist-mcp` (the queries, export), `src/app.rs` ("Open in SQL
  editor").
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
   by name would be a policy anyone can pick. The log and Activity already
   keep the names, so nothing is lost by waiting.
3. **Can production be set above Read, and with what extra confirm?** Not
   until the app can confirm a script on production, which it cannot today
   (B.7). Then: yes, with the existing production confirmation after the
   approval, never instead of it.
4. **Approvals from the phone, or only on this computer?** Only in the app,
   on this computer. A notification cannot show the SQL, the preview or the
   warnings, and approving without them is the thing the card exists to
   prevent. The notification's one action is "Review".
5. **(Phase 1)** `connections.json` or a new `connections.toml`? The plan
   uses the JSON file that exists. Is that accepted, with the Omarchy
   editor showing the JSON fragment in place of the board's TOML?
6. Activity and Log as a tab of the main window, not as windows. Accepted?
   (Needed for 1b's Activity.)
7. The count on the approval card: a read-only `SELECT`, with a dry run
   only when the person asks. Accepted, against the board's "Counted by a
   dry run"? (Phase 2.)
8. Is it right that an agent learns the names of hidden tables, as the
   boards show? The alternative is to answer as the database would for a
   table that does not exist. (Phase 1c.)
9. **(Phase 1)** HTTP after stdio, as its own pull request (1d)? Both
   agents the boards name connect over stdio, and the Omarchy board says
   "cursor, others: same server, stdio". HTTP brings the port, the tokens
   and at least nine new crates.
10. **(Phase 1)** `tabletist mcp` when the app is not running: an error,
    as planned, or a `--launch` flag from the start?
11. "Stop all agents" switches the server off until it is turned on again.
    Accepted?
12. Omarchy: `a` and `d` only inside the approval box, with the status line
    pointing at `:agents`. And the `:` prompt on the connections screen.
    Accepted? (Phase 2, and 1b for `:agents`.)
13. System catalogs: open on a connection that hides nothing, closed on one
    that hides or masks something. Or closed everywhere, for one rule?
    (Phase 1c.)
14. **(Phase 1)** Windows: is it in scope for each phase (the named pipe,
    the relay from a GUI binary), or may it follow the other two looks by
    one release?
15. **(Phase 1)** A Bookshop SQL fixture for the three drivers, new in
    `crates/tabletist-db/fixtures/`. Accepted? The alternative is the
    `users` and `orders` fixture the driver tests use today.
16. Settled on 2026-10-13: this file lives in `docs/superpowers/plans/`,
    which the website leaves out (`docs/_config.yml:113-115`). Under
    `docs/plans/` Jekyll would have copied it to the site as it is.
17. A new stdio agent: a notification, as planned, or a question the first
    time a name is seen ("Let Claude Code connect?")? The second is a real
    gate for stdio, which has none.
18. Should the log keep parameters in full? They can hold personal data.
    The alternative keeps their types and lengths.
19. The agent-facing name of a connection is its name and environment
    ("Bookshop · dev"). Two visible connections with the same one get
    " (2)" on the second. Or should the editor refuse to save agent access
    for a name that is taken?
20. An `UPDATE` or `DELETE` without a `WHERE`: a warning on the card, as
    planned, or refused outright for agents?

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

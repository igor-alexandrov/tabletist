# Managed policy: limits an administrator sets and nobody else can raise

Date: 2026-10-15. Status: draft, for review.

**Amended on 2026-10-17 against the MCP plan as built** (Phase 1c, hidden
tables, and Phase 3a, masked columns). A rule's hidden tables and masked
columns are names, as the person's are, and a stale one stops agents as
the person's does; a rule may list the tables agents may see instead
(`visible_only`); with any rule in the file a connection no rule matches is
Off for agents; and a loosening from a file that is not locked is held.
The floor is built after 3a. The MCP plan's B.8, C.2, C.3, E and G say what
this changes there.

## Intent

What agents may do on a connection is kept in `connections.toml` and
`settings.toml`, in the person's own folders. A coding agent with a shell
runs as the person, so it can edit both. The check of changes made outside
the app (`src/agents_shown.rs`, plan C.2) holds a change that lets agents do
more until the person answers, but it compares against `agents_shown.json`,
which the same agent can edit too. The MCP plan says so in E: "Nothing local
can be kept from the account that owns it."

What the account cannot edit is a file owned by an administrator. This spec
adds one: the **managed policy**, a TOML file at a fixed system path that
only an administrator can write. It is a floor. It can lower what agents may
do and force connections read-only. Nothing in the person's files can lift
what it sets.

Hidden tables (Phase 1c) and masked columns (Phase 3a), both built,
are what the person most wants an agent unable to undo. The floor is built
after 3a, so a rule can hide and mask from its first day, with the checks
those phases built.

The same document, signed, is what a paid service would hand out later to
everyone in a company. The file's shape is chosen for that now so the
service adds a source, not a format.

Success:

- An administrator writes `/etc/tabletist/managed.toml` (or the macOS or
  Windows path) with `hidden = ["customers"]` for the company's database.
- An agent running as the person edits `connections.toml`, `settings.toml`
  and `agents_shown.json` any way it likes, and `customers` stays hidden
  from it.
- The editor's Agents section shows the managed limits as set by the
  administrator and does not offer to change them.

## What this is not

- **Not protection against the person.** They are usually an administrator
  of their own laptop, and Tabletist is MIT-licensed, so anyone can build a
  copy that ignores the file. The policy holds the official app, and the
  agents that reach databases through it, to what the administrator set.
  Real enforcement against a person is the database's own: roles, column
  privileges, credentials the person does not hold.
- **Not protection against an agent with administrator rights.** One that
  can `sudo` without a password can rewrite the file. Its only defense is the
  OS account, as for everything else on the machine.
- **Not a sandbox around the agent.** It governs what goes through
  Tabletist's tools, as the MCP plan does. A shell can still run `psql` with
  whatever credentials it finds.

## Decisions

| Question | Decision |
|---|---|
| Where | One fixed path per system, never moved by a variable: `/etc/tabletist/managed.toml` on Linux, `/Library/Application Support/Tabletist/managed.toml` on macOS, `%ProgramData%\Tabletist\managed.toml` on Windows, the folder asked of the system (`FOLDERID_ProgramData`), never taken from the variable. That call is unsafe code, so it lives in a small crate of its own, as `tabletist-appkit` does for macOS. |
| What it is | A floor. Each limit is combined with the person's setting so the stricter wins. Nothing in the person's files can loosen it. |
| Matching | A rule names connections by what they reach (driver, host, port, database, SQLite path), with `*` globs, not by `id`. Connection ids are random and differ on each machine, and a duplicated connection must still match. |
| Several rules | Every rule that matches a connection applies. The result is the same in any order. |
| Ownership | Limits always apply, whoever owns the file: a floor written by anyone can only restrict. The app calls the policy **locked** only when the file and its folder can be changed by an administrator alone. Anything the file *grants* (Phase 2's connections, Phase 3's enrollment) is used only from a locked file. |
| Unreadable file | Read strictly, like `connections.toml`. A file that is there but cannot be read, or fails its version check, fails closed (see Failure). |
| Tables and columns | A rule's `hidden`, `visible_only` and `masked` entries are names written as the person's are (`customers`, `sales.customers`, `orders.email`), read by the checks 1c and 3a built. No patterns: a pattern matching nothing cannot be told from a renamed table. |
| Stale entries | A rule's `hidden` or `masked` entry that names nothing in a database it matches closes that connection to agents, as the person's does (MCP plan C.3). A `visible_only` entry that names nothing hides more and stops nothing. |
| Held changes | `agents_shown.json` and its check stay as they are, on the person's files. The managed floor applies after that, to what the check lets through. A change to the managed file that lets agents do less applies at once. One that lets them do more applies at once from a locked file, and is held, like a change to the person's files, from one that is not. |
| Reload | Watched while the app runs, like `settings.toml`, when its folder exists. A file that appears in a folder created later applies at the next start. |
| `--demo` and tests | `AppDirs::at` puts the policy file under its root, so the demo and tests never read the machine's file. |
| Dependencies | Nothing new is built. `toml` with `serde` is already a dependency, and the Windows folder call can use the `windows` crate already in `Cargo.lock`. Ed25519 for Phase 3 is `ed25519-dalek` 3.0.0, already in `Cargo.lock` through russh. |

Rejected:

- **Signing `agents_shown.json` with a key kept in the keyring.** It only
  helps on macOS, where the keychain's per-app ACL makes another program
  prompt. On Linux `secret-tool` reads an unlocked Secret Service without
  asking, and on Windows any process running as the person reads Credential
  Manager.
- **SQLCipher, or encrypting the person's files.** The key would have to be
  in the keyring, with the same problem. It would also end the hand-editable
  files the app is built around, and swap the SQLite every user database is
  opened with (one `links = "sqlite3"` per build).
- **Matching by connection id.** An administrator cannot know the ids, and
  a duplicate or a new connection to the same server would escape.
- **A path read from a variable or `XDG_CONFIG_DIRS`.** An agent with a
  shell can start Tabletist itself, with variables of its choosing.
- **A tolerant reader, like `settings.toml`'s.** A line skipped in a
  security floor is a limit quietly gone.

## The file

```toml
version = 1

# The agent server, on this machine.
[agents]
allowed = true           # false: the server cannot be turned on
unmatched = "off"        # the default once there is a rule: Off where no rule matches
log_days_min = 90        # the activity log keeps a call at least this long

[[rules]]
name = "Bookshop production"          # shown to the person as the reason
match = { driver = "postgres", host = "db.example.com", database = "bookshop_*" }
read_only = true
access_max = "read"
rows_max = 500
timeout_max_secs = 10
hidden = ["customers", "billing.invoices"]
masked = ["orders.email", "orders.phone"]

[[rules]]
name = "Analytics replica"
match = { driver = "postgres", host = "replica.example.com" }
visible_only = ["books", "authors", "sales.orders"]

[[rules]]
name = "Local SQLite files"
match = { driver = "sqlite", path = "/srv/shared/*.db" }
access_max = "off"
```

| Key | Values | Without it |
|---|---|---|
| `version` | `1` | Required. A higher version fails closed: an older app cannot know what a newer key forbids |
| `agents.allowed` | `true`, `false` | `true` |
| `agents.unmatched` | `"off"`, `"as_set"` | `"off"` when the file has any rule: a connection no rule matches is Off for agents. `"as_set"` leaves such a connection as the person set it |
| `agents.log_days_min` | 1 to 3650 | No minimum |
| `rules[].name` | Text | Required. Shown in the editor and in refusals |
| `rules[].match` | A table of `driver`, `host`, `port`, `database`, `path`, each optional, `*` globs on text | Required, and not empty. A rule that matches everything is `match = { driver = "*" }`, written out |
| `rules[].read_only` | `true` | No change. `false` is an error: a floor cannot open a connection for writes |
| `rules[].access_max` | `"off"`, `"read"` (and `"read_and_write"` once Phase 2 of the MCP plan has it) | No ceiling |
| `rules[].rows_max` | 1 to 5000 | No ceiling |
| `rules[].timeout_max_secs` | 1 to 60 | No ceiling |
| `rules[].hidden` | Table names, `schema.table` or a bare `table` (every schema, as B.3 has it). Agents may read every other table ("All except") | Nothing hidden |
| `rules[].visible_only` | Table names, as `hidden`. Agents may read these and no other ("Only"). Not beside `hidden` in one rule | Every table |
| `rules[].masked` | `table.column` or `schema.table.column`, as the person's masked columns are written | Nothing masked |

A key the app does not know is an error, as in `connections.toml`.

`rules` and `agents` are what version 1 reads. `connections` (Phase 2) and
`service` (Phase 3) are reserved: a version-1 app that finds either fails
closed rather than ignoring a grant it cannot apply.

### Matching

- `host` is compared with the connection's `host`, without regard to case.
  Through an SSH tunnel it is the database's host as the tunnel reaches it,
  not the bastion's. Matching on the bastion is left out of version 1.
- `port` is the connection's port, or its driver's default when it has none.
- `path` is the SQLite file's path as saved, compared after `~` is expanded
  and the path is made absolute. Symlinks are not resolved.
- A rule's keys must all match. A `match` that names a key the driver has no
  use for (`path` on PostgreSQL) does not match.

A person can point a connection at the same server by another name: an IP
address, `localhost` and a port forwarded by hand. A rule cannot see that.
`agents.unmatched = "off"` is the answer, and the default once the file has
a rule: a connection that escapes every rule is Off for agents, so evading
a rule gains nothing. The held-change check cannot do this instead (MCP
plan C.2): the program that changes the host can change
`agents_shown.json` too.

### Combining with the person's settings

For each connection, the effective setting is:

| Setting | Effective |
|---|---|
| Read-only | Read-only if the person's setting or any matching rule says so |
| Agent access | The lowest of the person's access and each matching rule's `access_max`. Off when `agents.allowed` is `false`, or when no rule matches and `unmatched` is `"off"` |
| Rows, timeout | The lowest of the person's value and each rule's ceiling |
| A table | Visible to agents only if the person's Tables setting and every matching rule (its `hidden` or its `visible_only`) let it be seen |
| A column | Masked if the person or any matching rule masks it |
| A stale entry | A `hidden` or `masked` entry, the person's or a rule's, that names nothing in the database closes the connection to agents, as MCP plan C.3 has it. A rule may match many databases, and each must have what it names: an administrator whose databases differ writes a rule for each shape, or uses `visible_only` |
| The server | Can be turned on only when `agents.allowed` is `true` |
| Log retention | The larger of the person's `mcp.log_days` and `agents.log_days_min` |

The person's own values are kept in their files as they wrote them. The
floor is applied when the app builds what it sends the backend's gateway
(`Command::Agents`), and to the read-only a session opens with, not written into
`connections.toml`. So removing a rule gives the person back exactly what
they had.

### Ownership: when the policy is locked

Locked means only an administrator can change or replace the file:

- **Linux and macOS.** The file and every folder above it are owned by root
  and writable by no group or other user. Neither is a symlink. Read through
  `symlink_metadata`: no unsafe code.
- **Windows.** The app opens the file asking for write and delete access,
  and its folder asking for delete-child access, through
  `OpenOptionsExt::access_mode` (no unsafe code). Each must be refused.
  `%ProgramData%` lets every user create files in a folder by default, so
  "the folder is writable" is not the test. Replacing the file is.

A file that is not locked still applies as a floor. The editor and Settings
› Agents say that it can be changed without administrator rights, so the
person does not take it for more than it is.

### Changes to the file

- **Agents may do less**: applied at once, locked or not.
- **Agents may do more** (a rule removed, a ceiling raised, a table no
  longer hidden, a column unmasked): applied at once from a locked file,
  since only an administrator could have made it. From a file that is not
  locked, held like a change to the person's files (MCP plan C.2), with
  the same bar. **Keep** takes it. **Put back** keeps the policy as it was
  last shown, since the app cannot write the file, until the file says
  that again or the person keeps the change.

What was last shown of the policy is kept beside the person's settings in
`agents_shown.json`. Editing that file can only make a change from an
unlocked policy look already shown, which is what an unlocked file already
allows.

### Failure

A file that is there but cannot be read, has an unknown key, a bad value or
a version above 1, or names `connections` or `service`:

- The agent server cannot be turned on. One that is running stops.
- Every connection is read-only.
- A bar under the top of the window, like the held-change bar, says that the
  administrator's policy cannot be read and which line is wrong. Like
  `connections.toml`'s notice, it never repeats a value from the line.

A file that is not there is no policy. It does not fail closed: most people
have none.

## In the app

There are no boards for this yet. Each item below needs one in the design
canvas before its step is built (the canvas wins over this spec on layout and
copy).

- **The editor's Agents section.** A value that a rule caps or sets shows the
  effective value with a lock and "Set by your administrator", plus the
  rule's `name`. A control that a rule fixes cannot be changed. One that a
  rule only caps can be lowered further.
- **Settings › Agents.** With `allowed = false` the switch is off and cannot
  be turned on, with the same wording. `log_days_min` limits the retention
  menu.
- **The read-only box** of a connection a rule makes read-only is checked and
  locked.
- **The editor's Tables and Masked columns controls** show a rule's entries
  beside the person's, locked, and mark a rule's stale entry as the person's
  is marked, with "Ask your administrator" in place of removing it.
- **Refusals to agents.** A refusal caused by the policy says so, with the
  same codes and the sentences in MCP plan B.8 ("customers is hidden for
  agents by your administrator's policy."), so the agent does not ask the
  person to change a setting they cannot change. `list_connections` gives a
  rule's entries beside the person's and says which are the
  administrator's.
- **Settings and Files** (`docs/_reference/settings-and-files.md`) gains the
  file, its paths, its keys and the failure rules.

## Phases

Each phase is its own plan and pull request.

**1. The floor.** The file, its reader, matching, combining, ownership,
failure, watching, the held change from an unlocked file, the locked
controls and the reference page. Built after Phase 3a of the MCP plan, so
that hidden tables and masked columns both work in it from the first day.
The checks take one `Tables` setting (`Connection::run_agent_read`'s
`tables`, read by `Hidden`), so this phase lets them take several. The
masked columns are already a list, and the sources' lists are joined.

**2. Managed connections.** A locked file's `[[connections]]`, in the same
shape as `connections.toml`'s, with an `id` the administrator chooses
(`acme-bookshop-prod`), never a secret. They appear in the sidebar with a
lock. They cannot be edited or deleted, only duplicated, and a duplicate
still matches the rules because rules match by target. Each person's
password is in their own keyring, as now. `version = 2`.

**3. The service.** A locked file's `[service]` enrolls the machine:

```toml
[service]
url = "https://policy.example.com/org/acme"
public_key = "ed25519:..."
```

The app fetches a bundle from `url`: the same TOML document as this file,
`rules` and `connections`, plus `serial` and `expires`, signed with Ed25519.
It keeps the last good bundle in its state folder, so it works offline. The
signature protects that copy, not its folder. A verified bundle is applied
exactly as another locked layer of the floor.

- An enrolled machine with no valid bundle (missing, bad signature,
  expired) fails closed for what the bundle governs: managed connections are
  unavailable and agents are Off on what the rules would match.
- A bundle with a lower `serial` than the last one seen is refused. The
  last serial is kept in the state folder, which the person can reset, so
  `expires` (days, not months) is what bounds a rollback to an older,
  looser bundle.
- The service never sends a secret. Short-lived database credentials issued
  through the person's single sign-on are a later idea, not part of this
  spec.
- The activity log can be sent to the service for audit. Its design is
  separate, because it carries statements and their values, and the MCP
  plan's E calls the log private to the account.
- Fetching the bundle is the app's first HTTP request. `rustls` is in
  `Cargo.lock`, but no HTTP client is, so this phase adds one, unlike
  Phases 1 and 2, which build nothing new.

Rejected for Phase 3: a privileged helper that fetches the bundle and
writes `managed.toml` itself. It needs a root service on each system, and
the signature already does what the file's owner does. The file stays the
administrator's: enrollment and anything set by hand.

## Testing

- Reader: every key, every bad value, unknown keys, a higher version, the
  reserved tables. Each fails closed and names the line without its value.
- Matching: globs, case, default ports, SQLite paths with `~`, a tunnel's
  host, a `match` with a key the driver has no use for.
- Combining: each row of the table above, several rules, `unmatched`,
  `allowed`, a rule removed giving back the person's values exactly.
- The held-change check and the floor together: a raise in
  `connections.toml` that the person keeps is still capped by a rule. A
  rule removed from an unlocked file is held, and from a locked one (the
  verdict injected) is taken at once.
- Tables and columns: a rule's `hidden`, `visible_only` and `masked` beside
  the person's, on each driver; a rule's stale `hidden` or `masked` entry
  closing the connection with the administrator's sentence; a stale
  `visible_only` entry stopping nothing.
- Ownership: Unix through files the test creates (owned by the test user,
  so not locked) and the verdict injected for the locked case. Windows: the
  access check type-checked for Windows as the other platform code is, and
  run on CI's Windows job.
- `AppDirs::at` puts the file under its root, so no test reads `/etc`.
- Server tests on PostgreSQL and MySQL for the refusal wording, added to
  `ci.yml`'s integration jobs by name.

## Open questions

1. **Read-only on failure.** An unreadable policy makes every connection
   read-only, not only the matched ones (it cannot tell which those are).
   That is the safe reading, and it also stops a person's work over an
   administrator's typo. Is that right, or should failure only stop agents?
2. **The name.** `managed.toml`, or `policy.toml`?

Settled on 2026-10-17:

- **`unmatched` by default.** `"off"` whenever the file has a rule
  (Matching).
- **Order against the MCP plan.** After Phase 3a, so that `hidden` and
  `masked` work from the first day (Phases).

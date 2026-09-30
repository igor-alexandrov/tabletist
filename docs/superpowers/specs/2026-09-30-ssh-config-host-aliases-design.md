# SSH tunnel: Host aliases from ~/.ssh/config

Date: 2026-09-30. Status: approved in conversation, awaiting spec review.

## Intent

Item 7 of the macOS review: "Read ssh config (Host aliases, IdentityAgent,
User) so that in SSH tunnel you can just pick a host, like in the terminal."

The connection dialog's SSH tunnel should work as `ssh <alias>` does: pick
`bastion` from the hosts `~/.ssh/config` names, and the tunnel reaches the
right HostName, Port and User and logs in with the right IdentityFile or
IdentityAgent, all read from the config each time it connects.

Success: with a config holding `Host bastion` (HostName, Port, User,
IdentityAgent), a user opens the dialog, picks `bastion` from a list, leaves
port, user and key empty, and Test connects through the agent the config
names. Later edits to the config reach the saved connection without editing
it.

## Decisions

- **Keep the alias, resolve at connect time.** The saved SSH host is the
  alias. HostName, Port, User, IdentityFile and IdentityAgent are resolved
  from `~/.ssh/config` on every connect. Nothing is copied into the saved
  spec.
- **An empty field means "from the config".** Like `ssh -p 2222 -l me
  bastion`: a typed port, user or key file wins; an empty one uses the
  config's value; with neither, the default applies (port 22, the local login
  name, no key file). The fields show the resolved values as hints.
- **The three login methods stay.** Password, Key file and Agent. Agent
  already uses the config's IdentityAgent. An empty Key file uses the config's
  first IdentityFile. Picking an alias presets the method.
- **Picking is a list beside the host field.** The host stays a free text
  field; a "▾" button beside it lists the concrete Host names.
- **ProxyJump is out of scope.** When the resolved host has ProxyJump or
  ProxyCommand, Test and Connect stop with a clear error rather than
  connecting directly (which ssh would not do). A later change may add it.

## Out of scope

- ProxyJump and ProxyCommand (refused with an error, see above).
- `Match` blocks (still skipped, as today).
- `CanonicalizeHostname`, `HostKeyAlias`, OpenSSH's `known_hosts` file.
- Hints for a typed host that only a wildcard block covers
  (`Host *.example.com`). Such a host is still resolved in full at connect
  time; the dialog just shows the plain defaults as hints.
- Trying several IdentityFiles, or the default `~/.ssh/id_*` keys.
- `%h`, `%r`, `%u` and other tokens in IdentityFile (only `~` and `%d`
  expand, as in IdentityAgent today).

## Design

### Config reader (`crates/tabletist-db/src/ssh_config.rs`)

The existing reader stays the one parser. Its rules do not change: the first
value a keyword gets wins; `Host` patterns with `*`, `?` and `!`; `Include`
with globs, relative to `~/.ssh`, depth bounded at 16; `Match` blocks skipped;
quotes; `Keyword=value`; `#` comments.

`Reader::agent` becomes `Reader::read(&mut self, text, &mut HostConfig)`: it
walks the text and, for each line that applies to the host, sets the matching
field only if it is still unset. Includes recurse into the same `HostConfig`,
so "first value wins" holds across files.

```rust
pub struct HostConfig {
    /// `HostName`, with `%h` (the alias) and `%%` expanded.
    pub host_name: Option<String>,
    /// `Port`; a value that is not a port number is ignored.
    pub port: Option<u16>,
    pub user: Option<String>,
    /// The first `IdentityFile`, `~` and `%d` expanded.
    pub identity_file: Option<PathBuf>,
    pub identity_agent: Option<AgentSocket>,
    /// `ProxyJump` or `ProxyCommand`, whichever comes first. They share
    /// one slot, as in OpenSSH, and `none` fills it too: an earlier
    /// `ProxyJump none` (the jump host's own block) beats a later
    /// `Host *` `ProxyJump bastion`.
    pub proxy: Option<Proxy>,
}

pub enum Proxy {
    /// `none`: connect directly.
    Off,
    Jump(String),
    Command(String),
}

/// What `~/.ssh/config` says about `host`. Empty when there is no config.
pub fn resolve(host: &str) -> HostConfig;

/// A Host name the config spells out, with what it resolves to.
pub struct ConfigHost {
    pub alias: String,
    pub config: HostConfig,
}

/// The concrete Host names in `~/.ssh/config` and its Includes.
pub fn hosts() -> Vec<ConfigHost>;
```

`hosts()` collects every `Host` pattern that contains none of `*`, `?`, `!`,
in file order (Includes read in place), dropping later duplicates
case-insensitively. An `Include` inside a `Host` block is followed for
listing regardless of the block. Each alias is then resolved with `resolve`'s
logic (without re-reading the files from disk).

`identity_agent(host)` is removed; `ssh.rs` uses `resolve(host).identity_agent`.
The reader keeps taking `read` and `list` closures so tests need no disk.

`lib.rs` makes the module public (`pub mod ssh_config`), since the backend
calls `hosts()` and the app keeps `ConfigHost`, `HostConfig`, `AgentSocket`
and `Proxy`. They derive `Debug`, `Clone`, `PartialEq` and `Eq` (the app's
`Event` and form are `Debug`).

### Saved spec (`crates/tabletist-db/src/spec.rs`)

```rust
pub struct SshSpec {
    pub host: String,
    /// Typed port; `None` uses the config's, else 22.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub port: Option<u16>,
    /// Typed user; empty uses the config's, else the local login name.
    pub user: String,
    pub auth: SshAuth,
}
```

`SshAuth::KeyFile { path }` with an empty path uses the config's
IdentityFile.

Backward compatibility: existing `connections.json` files have `"port": 22`,
which loads as `Some(22)`, so old connections keep their explicit port and
user. They do now go through the config like any other host: an intended
change. A saved `bastion` connects to its HostName (on the saved port), and
a saved host that a config block gives a ProxyJump is refused rather than
connected directly. A `None` port is left out of the file.
An older build cannot read such an entry (its `port` is required); files
only move forward, as with earlier additions.

`summary()` prints `via ops@bastion`, or `via bastion` when the user is
empty.

### Resolving at connect time (`crates/tabletist-db/src/ssh.rs`)

A pure function holds the precedence, so it can be tested without a config
file or a network:

```rust
pub(crate) struct Endpoint {
    pub host: String,       // HostName, else the alias
    pub port: u16,          // typed, else config, else 22
    pub user: String,       // typed, else config, else login
    pub key: Option<PathBuf>, // KeyFile only: typed, else config
}

pub(crate) fn endpoint(
    spec: &SshSpec,
    config: &HostConfig,
    login: Option<&str>,
) -> Result<Endpoint>;
```

Errors:

- Empty host: `InvalidSpec("enter the SSH host")` (the check `Tunnel::open`
  makes today moves here).
- `config.proxy` is `Jump` or `Command`: `SshStage::Connect`, "~/.ssh/config reaches
  bastion through ProxyJump, which Tabletist does not support yet" (naming
  ProxyCommand when that is what the config set).
- No user anywhere: `InvalidSpec("enter the SSH user")`.
- Key file method with no typed path and no IdentityFile:
  `InvalidSpec("choose a key file for SSH")`.

`login` is `$USER`, else `$USERNAME`, read by `Tunnel::open`.

`Tunnel::open` (already on the backend runtime) calls
`ssh_config::resolve(&spec.host)`, then `endpoint`, and uses the endpoint for
the TCP address, the host key lookup, the user name and the key file. The
agent is still chosen by the alias: OpenSSH matches `Host` patterns against
what was typed, not the HostName, so `Host bastion` still applies.

### Host keys

`SshStage::HostKeyUnknown` and `SshStage::HostKeyMismatch` gain the resolved
`host: String` and `port: u16`. The app's two trust paths (a tab's Connect in
`App::apply` and the dialog's Test via `TestState::Untrusted`) take host and
port from the error instead of from the spec or the form. `known_hosts.json`
stays keyed by the real server's `host:port`.

A connection saved without an alias resolves to the same host and port, so
its trusted key still matches. A connection saved as `bastion` whose HostName
differs asks for trust once.

### Listing hosts (backend and app)

- `Backend::list_ssh_hosts(request)`, like `pick_key_file`: it runs
  `ssh_config::hosts()` on the runtime's blocking pool and answers
  `Event::SshHosts { request, hosts }`. It is a method rather than a
  `Command`, as the file dialogs are, so it does not go through the
  worker's session queue (and the recording backend in tests ignores it).
- The app calls it whenever it opens the connection dialog, new or edit, and
  stores the answer in `ConnectionForm::ssh_hosts: Vec<ConfigHost>` (with
  `ssh_hosts_request` to drop stale answers). The UI thread never reads the
  config.
- `Action::PickSshHost(alias)`, applied by `App::apply`:
  - sets SSH host to the alias;
  - clears SSH port, SSH user and Key file, so the config's values show;
  - presets the login method: Agent if the alias's IdentityAgent is set and
    not `none`; else Key file if it has an IdentityFile; else unchanged.

### Form (`src/model.rs`)

- `ssh_port` starts empty instead of `"22"`; `from_saved` shows `None` as
  empty.
- `to_spec`: only SSH host is required. An empty port becomes `None`; a
  non-number is still "Enter the SSH port number." An empty user is allowed.
  The Key file method accepts an empty path.
- `ConnectionForm::ssh_config_host()` returns the listed `ConfigHost` whose
  alias equals the trimmed SSH host, compared case-insensitively.

### Dialog (`src/ui/connect_dialog.rs`, `ssh_section`)

- Beside SSH host, a small "▾" button named "Hosts from ~/.ssh/config"
  opens a popup of the aliases, each with its HostName dimmed beside it.
  Choosing one pushes `Action::PickSshHost`. No button when the list is
  empty.
- When `ssh_config_host()` finds the alias, empty fields show its values as
  hints: port `2222`; user `ops (from ~/.ssh/config)`; key file
  `~/.ssh/id_work (from ~/.ssh/config)`. Under the host row, a dim line
  `10.0.0.5 from ~/.ssh/config`, or, when the alias's proxy is `Jump` or
  `Command` (not `Off`), a warning: "Uses ProxyJump, which Tabletist does
  not support yet." (naming ProxyCommand when that is what it uses).
- Otherwise, and for any value the alias leaves unset, the hints are `22`
  for the port, the login name for the user, and nothing for the key file.
- Text goes through the existing widgets and `TextRole`s; the view names no
  font or size.

## Errors

| Situation | Where | Message |
|---|---|---|
| No `~/.ssh/config`, or unreadable | reader | none: empty `HostConfig`, empty list |
| `Port` not a number in the config | reader | ignored; typed or 22 |
| ProxyJump / ProxyCommand, not `none` | `endpoint` | `SshStage::Connect`, names the keyword |
| No user typed, in config, or in env | `endpoint` | "enter the SSH user" |
| Key file method, no path anywhere | `endpoint` | "choose a key file for SSH" |
| IdentityFile missing on disk | `load_key` | existing "could not read ..." |

## Testing

Behavioural tests next to the code; no design or pixel checks. Fixtures use
neutral names (`bastion`, `db.example.com`, `ops`).

- `ssh_config`: first value wins per keyword, across Includes; `%h` in
  HostName; an earlier `ProxyJump none` beats a later `Host *` ProxyJump;
  ProxyCommand shares the slot;
  `hosts()` skips `*`, `?`, `!` patterns, drops duplicates, follows
  Includes, resolves each alias. Existing IdentityAgent tests keep passing
  through `resolve`.
- `ssh::endpoint`: typed vs config vs default for host, port, user and key;
  the proxy error; no user; no key.
- `spec`: an old JSON port loads as `Some`; a missing port is `None`; the
  summary with an empty user.
- `model`: `to_spec` with empty port and user; `from_saved` round trip;
  `ssh_config_host` matching case-insensitively.
- `app`: opening the dialog sets `ssh_hosts_request`; `SshHosts` fills the list
  and a stale answer is dropped; `PickSshHost` fills the host, clears the
  fields and presets the method; trust uses the error's host and port for
  both Connect and Test.
- Headless UI (`src/ui/mod.rs`): the ▾ button appears only with hosts;
  its popup lists aliases and choosing one fills SSH host; the placeholder
  hints show; the ProxyJump warning shows.

The SSH integration tests (`crates/tabletist-db/tests/ssh.rs`) connect to
`localhost:52222` through `Connection::connect_with`, which now reads the machine's
real `~/.ssh/config`, as `ssh localhost` would. CI has no config; a
developer whose `Host *` sets a HostName or ProxyJump sees those tests fail
the same way `ssh -p 52222 localhost` would.

## Commits

One topic each:

1. The SSH config reader resolves HostName, Port, User, IdentityFile and
   ProxyJump, and lists hosts.
2. SSH host keys are trusted by the host and port the tunnel reports
   (still the typed ones at this point).
3. The SSH tunnel resolves a Host alias at connect time (with the
   `SshSpec` and `to_spec` changes it needs to compile), so the reported
   host becomes the resolved one.
4. The connection dialog lists and applies Host aliases.

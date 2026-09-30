# SSH config Host aliases Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Let the connection dialog's SSH tunnel take a Host alias from `~/.ssh/config`, resolved at connect time as `ssh <alias>` does.

**Architecture:** The existing `ssh_config` reader grows from one keyword (IdentityAgent) to a `HostConfig` (HostName, Port, User, IdentityFile, IdentityAgent, ProxyJump/ProxyCommand) plus a list of concrete Host names. `SshSpec` keeps the alias; a pure `endpoint()` in `ssh.rs` applies "typed, else config, else default" when the tunnel opens. The dialog gets the host list from the backend and shows the config's values as field hints.

**Tech Stack:** Rust 2024, egui (crmne fork), russh, serde, tokio. Headless UI tests through `src/testing.rs` (AccessKit).

**Spec:** `docs/superpowers/specs/2026-09-30-ssh-config-host-aliases-design.md`

---

## Ground rules (read first)

- Work in `/home/igor/Work/tabletist/.claude/worktrees/ssh-config-host-aliases-149a4c`, branch `claude/ssh-config-host-aliases`.
- `cargo` through mise is broken here: always run `~/.cargo/bin/cargo`.
- **Never commit or push unless the user asks.** Each task ends with a "commit point": stop there, report, and wait. If the user has asked to commit: one topic per commit, signed (`git commit -S`). If signing fails, ask before committing unsigned. Never set `SSH_AUTH_SOCK` in git commands. End messages with `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.
- No em dashes anywhere (code, comments, docs, commit messages). Use a full stop, comma, colon or parentheses.
- Tests are behavioural and sit next to the code (`#[cfg(test)] mod tests`). Never a design or pixel conformance test.
- Test fixtures use neutral names only: `bastion`, `replica`, `db.example.com`, `10.0.0.5`, `ops`, `deploy`.
- Views draw text only through `TextRole`s and the `widgets` helpers; never name a font or size.
- Comment density: short doc comments on items saying what and why, matching the surrounding code.

Full check commands (used in the last task, and handy any time):

```bash
~/.cargo/bin/cargo fmt --all --check
~/.cargo/bin/cargo clippy --locked --workspace --all-targets -- -D warnings
~/.cargo/bin/cargo clippy --locked --workspace --all-targets --features shots -- -D warnings
~/.cargo/bin/cargo test --locked --workspace --all-targets
RUSTDOCFLAGS='-D warnings' ~/.cargo/bin/cargo doc --locked --workspace --no-deps
```

## File map

| File | Change |
|---|---|
| `crates/tabletist-db/src/ssh_config.rs` | Reader resolves every keyword into `HostConfig`; `resolve`, `hosts`, `login`; `Proxy`, `ConfigHost` |
| `crates/tabletist-db/src/lib.rs` | `pub mod ssh_config` |
| `crates/tabletist-db/src/error.rs` | `HostKeyUnknown`/`HostKeyMismatch` carry `host`, `port` |
| `crates/tabletist-db/src/spec.rs` | `SshSpec.port: Option<u16>`; summary without user |
| `crates/tabletist-db/src/ssh.rs` | `Endpoint`, `endpoint()`; `Tunnel::open` resolves the alias |
| `crates/tabletist-db/tests/ssh.rs` | Follow the new port and host key shapes |
| `src/model.rs` | Form: empty port, optional user and key file; `ssh_hosts`, `ssh_config_host`, `pick_ssh_host`, `ssh_hints`; `Action::PickSshHost` |
| `src/backend.rs` | `Backend::list_ssh_hosts`, `Event::SshHosts` |
| `src/app.rs` | Ask for hosts on open; apply `SshHosts`, `PickSshHost`; trust the reported host |
| `src/ui/connect_dialog.rs` | ▾ host list, hints, HostName / proxy line |
| `src/ui/format.rs`, `src/ui/mod.rs`, `src/shots.rs` | Follow the new shapes; headless UI tests |

---

## Task 1: The SSH config reader resolves every keyword and lists hosts

**Files:**
- Modify: `crates/tabletist-db/src/ssh_config.rs` (whole file)
- Modify: `crates/tabletist-db/src/lib.rs:16`
- Modify: `crates/tabletist-db/src/ssh.rs:124-136` (`agent()`)

- [ ] **Step 1: Rewrite the test module against the new API (failing)**

Replace the `#[cfg(test)] mod tests` at the bottom of `ssh_config.rs` with the following. The six existing tests stay, through the new `agent` helper; seven are new.

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    /// Runs `run` over a home at /home/me whose `~/.ssh/config` is `config`
    /// (none when `None`), with `files` beside it in `~/.ssh`.
    fn with_files<R>(
        config: Option<&str>,
        files: &[(&str, &str)],
        run: impl FnOnce(&Files<'_>) -> R,
    ) -> R {
        let home = PathBuf::from("/home/me");
        let mut disk: BTreeMap<PathBuf, String> = files
            .iter()
            .map(|(name, text)| (home.join(".ssh").join(name), (*text).to_owned()))
            .collect();
        if let Some(config) = config {
            disk.insert(home.join(".ssh").join("config"), config.to_owned());
        }
        let read = |path: &Path| disk.get(path).cloned();
        let list = |dir: &Path| {
            disk.keys()
                .filter(|path| path.parent() == Some(dir))
                .cloned()
                .collect()
        };
        run(&Files {
            home: &home,
            read: &read,
            list: &list,
        })
    }

    fn resolve_in(config: &str, files: &[(&str, &str)], host: &str) -> HostConfig {
        with_files(Some(config), files, |files| files.resolve(host))
    }

    /// `host`'s agent in `config`, with `files` beside it in `~/.ssh`.
    fn agent(config: &str, files: &[(&str, &str)], host: &str) -> Option<AgentSocket> {
        resolve_in(config, files, host).identity_agent
    }

    fn hosts_in(config: &str, files: &[(&str, &str)]) -> Vec<ConfigHost> {
        with_files(Some(config), files, |files| files.hosts())
    }

    #[test]
    fn a_quoted_path_with_spaces_and_a_tilde_names_the_agent() {
        let config = "Host *\n  IdentityAgent \"~/Library/Group Containers/2BUA8C4S2C.com.1password/t/agent.sock\"\n";
        assert_eq!(
            agent(config, &[], "bastion.example.com"),
            Some(AgentSocket::Path(
                "/home/me/Library/Group Containers/2BUA8C4S2C.com.1password/t/agent.sock".into()
            ))
        );
    }

    #[test]
    fn the_first_host_that_matches_wins() {
        let config = "Host db-*.internal !db-legacy.internal\n    IdentityAgent ~/a.sock\n\
                      Host *\n    IdentityAgent=~/b.sock\n";
        assert_eq!(
            agent(config, &[], "DB-1.internal"),
            Some(AgentSocket::Path("/home/me/a.sock".into()))
        );
        assert_eq!(
            agent(config, &[], "db-legacy.internal"),
            Some(AgentSocket::Path("/home/me/b.sock".into())),
            "negated"
        );
        assert_eq!(
            agent(config, &[], "other"),
            Some(AgentSocket::Path("/home/me/b.sock".into()))
        );
    }

    #[test]
    fn none_and_the_environment_are_kept_apart() {
        assert_eq!(
            agent("Host prod\n IdentityAgent none\n", &[], "prod"),
            Some(AgentSocket::Off)
        );
        assert_eq!(
            agent("IdentityAgent SSH_AUTH_SOCK\n", &[], "prod"),
            Some(AgentSocket::Environment)
        );
        assert_eq!(agent("Host other\n IdentityAgent ~/x\n", &[], "prod"), None);
    }

    #[test]
    fn includes_are_read_in_place_and_globbed() {
        let config = "Include config.d/*\nHost *\n  IdentityAgent ~/late.sock\n";
        let files = [
            (
                "config.d/10-work",
                "Host *.work\n  IdentityAgent %d/work.sock\n",
            ),
            ("config.d/20-rest", "# nothing here\n"),
        ];
        assert_eq!(
            agent(config, &files, "api.work"),
            Some(AgentSocket::Path("/home/me/work.sock".into()))
        );
        assert_eq!(
            agent(config, &files, "api.home"),
            Some(AgentSocket::Path("/home/me/late.sock".into()))
        );
    }

    #[test]
    fn match_blocks_and_comments_are_skipped() {
        let config = "# IdentityAgent ~/commented.sock\nMatch host prod\n  IdentityAgent ~/matched.sock\n\
                      Host prod # the database's bastion\n  IdentityAgent ~/host.sock\n";
        assert_eq!(
            agent(config, &[], "prod"),
            Some(AgentSocket::Path("/home/me/host.sock".into()))
        );
    }

    #[test]
    fn wildcards_match_the_whole_name() {
        assert!(wildcard("*.example.com", "db.example.com"));
        assert!(!wildcard("*.example.com", "example.com"));
        assert!(wildcard("db-??", "db-01"));
        assert!(!wildcard("db-??", "db-001"));
        assert!(wildcard("*", ""));
    }

    #[test]
    fn the_first_value_of_each_keyword_wins_across_includes() {
        let config = "Include conf.d/*\n\
                      Host bastion\n  HostName 10.0.0.5\n  Port 2222\n  User ops\n  IdentityFile ~/.ssh/id_work\n\
                      Host *\n  User nobody\n  Port 22\n  IdentityFile ~/.ssh/id_other\n";
        let files = [("conf.d/bastion", "Host bastion\n  User deploy\n")];
        assert_eq!(
            resolve_in(config, &files, "bastion"),
            HostConfig {
                host_name: Some("10.0.0.5".into()),
                port: Some(2222),
                user: Some("deploy".into()),
                identity_file: Some("/home/me/.ssh/id_work".into()),
                identity_agent: None,
                proxy: None,
            }
        );
        let other = resolve_in(config, &files, "other");
        assert_eq!(other.host_name, None);
        assert_eq!(other.user.as_deref(), Some("nobody"));
        assert_eq!(other.port, Some(22));
    }

    #[test]
    fn host_name_stands_the_alias_in_for_percent_h() {
        let config = "Host db-*\n  HostName %h.example.com\nHost odd\n  HostName 100%%-%x\n";
        assert_eq!(
            resolve_in(config, &[], "db-1").host_name.as_deref(),
            Some("db-1.example.com")
        );
        assert_eq!(
            resolve_in(config, &[], "odd").host_name.as_deref(),
            Some("100%-%x")
        );
    }

    #[test]
    fn a_port_that_is_not_a_number_leaves_room_for_a_later_one() {
        let config = "Host bastion\n  Port ssh\nHost *\n  Port 2200\n";
        assert_eq!(resolve_in(config, &[], "bastion").port, Some(2200));
    }

    #[test]
    fn an_earlier_proxy_none_beats_a_later_jump() {
        let config = "Host bastion.internal\n  ProxyJump none\n\
                      Host *.internal\n  ProxyJump bastion.internal\n";
        assert_eq!(
            resolve_in(config, &[], "bastion.internal").proxy,
            Some(Proxy::Off)
        );
        assert_eq!(
            resolve_in(config, &[], "db.internal").proxy,
            Some(Proxy::Jump("bastion.internal".into()))
        );
        assert_eq!(resolve_in(config, &[], "elsewhere").proxy, None);
    }

    #[test]
    fn proxy_command_shares_the_slot_with_proxy_jump() {
        let config = "Host legacy\n  ProxyCommand ssh -W %h:%p gateway\n  ProxyJump bastion\n";
        assert_eq!(
            resolve_in(config, &[], "legacy").proxy,
            Some(Proxy::Command("ssh -W %h:%p gateway".into()))
        );
    }

    #[test]
    fn hosts_lists_the_names_the_config_spells_out_once_in_file_order() {
        let config = "Include conf.d/*\n\
                      Host bastion db-*.internal !db-legacy.internal\n  HostName 10.0.0.5\n\
                      Host BASTION,analytics ?ci\n  User ops\n\
                      Host *\n  User nobody\n";
        let files = [("conf.d/replica", "Host replica\n  HostName 10.0.0.9\n")];
        let hosts = hosts_in(config, &files);
        let aliases: Vec<&str> = hosts.iter().map(|host| host.alias.as_str()).collect();
        assert_eq!(aliases, ["replica", "bastion", "analytics"]);
        assert_eq!(hosts[0].config.host_name.as_deref(), Some("10.0.0.9"));
        assert_eq!(hosts[0].config.user.as_deref(), Some("nobody"));
        assert_eq!(hosts[1].config.host_name.as_deref(), Some("10.0.0.5"));
        assert_eq!(hosts[1].config.user.as_deref(), Some("ops"));
        // An Include inside a Host block still adds its names to the list.
        let nested = hosts_in("Host bastion\n  Include conf.d/*\n", &files);
        let aliases: Vec<&str> = nested.iter().map(|host| host.alias.as_str()).collect();
        assert_eq!(aliases, ["bastion", "replica"]);
    }

    #[test]
    fn no_config_resolves_to_nothing() {
        with_files(None, &[], |files| {
            assert_eq!(files.resolve("bastion"), HostConfig::default());
            assert!(files.hosts().is_empty());
        });
    }
}
```

- [ ] **Step 2: Run the tests to see them fail**

Run: `~/.cargo/bin/cargo test -p tabletist-db --lib ssh_config`
Expected: compile errors (`Files`, `HostConfig`, `ConfigHost`, `Proxy` not found).

- [ ] **Step 3: Replace everything above the test module**

Replace the module doc, `AgentSocket`, `identity_agent`, `home`, `list`, `Reader` and its impl with the following. `words`, `host_matches` and `wildcard` stay exactly as they are.

```rust
//! What `~/.ssh/config` says about an SSH host, as OpenSSH reads it:
//! HostName, Port, User, IdentityFile, IdentityAgent and ProxyJump, and the
//! Host names it spells out, so a tunnel can take a Host alias the way
//! `ssh bastion` does. Keys kept in 1Password, Secretive or another agent
//! work as they do in a terminal, including when the app starts from the
//! Dock without that agent's `SSH_AUTH_SOCK`.
//!
//! The rules OpenSSH follows, and so this: the first value a keyword gets
//! wins; a `Host` line applies to the host when one of its patterns (`*`,
//! `?`) matches and no negated one (`!`) does; `Include` reads other files
//! in place (relative to `~/.ssh`, with globs); `Match` blocks are not
//! evaluated, so what they set is ignored.

use std::cell::RefCell;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Where the SSH agent is, as the config names it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgentSocket {
    /// A socket (a named pipe on Windows).
    Path(PathBuf),
    /// `IdentityAgent none`: no agent for this host.
    Off,
    /// `IdentityAgent SSH_AUTH_SOCK`: the environment's agent.
    Environment,
}

/// How the config reaches a host through another one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Proxy {
    /// `ProxyJump none` or `ProxyCommand none`: directly.
    Off,
    /// `ProxyJump`: through these jump hosts.
    Jump(String),
    /// `ProxyCommand`: through this command.
    Command(String),
}

/// What the config says about one host; `None` where it says nothing.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HostConfig {
    /// `HostName`, `%h` standing for the alias.
    pub host_name: Option<String>,
    pub port: Option<u16>,
    pub user: Option<String>,
    /// The first `IdentityFile`.
    pub identity_file: Option<PathBuf>,
    pub identity_agent: Option<AgentSocket>,
    /// `ProxyJump` or `ProxyCommand`, whichever comes first: they share one
    /// slot, and `none` fills it too.
    pub proxy: Option<Proxy>,
}

/// A Host name the config spells out, and what it resolves to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigHost {
    pub alias: String,
    pub config: HostConfig,
}

/// What `~/.ssh/config` says about `host`: nothing when there is no config.
pub fn resolve(host: &str) -> HostConfig {
    let Some(home) = home() else {
        return HostConfig::default();
    };
    let read = |path: &Path| std::fs::read_to_string(path).ok();
    Files {
        home: &home,
        read: &read,
        list: &list,
    }
    .resolve(host)
}

/// The Host names `~/.ssh/config` and its Includes spell out (no wildcards
/// or negations), in file order, each resolved.
pub fn hosts() -> Vec<ConfigHost> {
    let Some(home) = home() else {
        return Vec::new();
    };
    // Every alias is resolved against the same files: read each once.
    let texts = RefCell::new(HashMap::new());
    let read = |path: &Path| {
        texts
            .borrow_mut()
            .entry(path.to_path_buf())
            .or_insert_with(|| std::fs::read_to_string(path).ok())
            .clone()
    };
    Files {
        home: &home,
        read: &read,
        list: &list,
    }
    .hosts()
}

/// The local login name, which ssh uses when neither the command line nor
/// the config names a user.
pub fn login() -> Option<String> {
    std::env::var("USER")
        .or_else(|_| std::env::var("USERNAME"))
        .ok()
        .filter(|name| !name.is_empty())
}

fn home() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .filter(|home| home.is_absolute())
}

/// The files in `dir`, for `Include` globs.
fn list(dir: &Path) -> Vec<PathBuf> {
    std::fs::read_dir(dir)
        .map(|entries| {
            entries
                .filter_map(|entry| Some(entry.ok()?.path()))
                .collect()
        })
        .unwrap_or_default()
}

/// How deep Includes nest, bounded as OpenSSH bounds it.
const MAX_DEPTH: u8 = 16;

/// The config files, read through `read` and `list` so tests need no disk.
struct Files<'a> {
    home: &'a Path,
    read: &'a dyn Fn(&Path) -> Option<String>,
    list: &'a dyn Fn(&Path) -> Vec<PathBuf>,
}

impl Files<'_> {
    fn config(&self) -> Option<String> {
        (self.read)(&self.home.join(".ssh").join("config"))
    }

    fn resolve(&self, host: &str) -> HostConfig {
        let mut config = HostConfig::default();
        if let Some(text) = self.config() {
            self.read_into(host, &text, 0, &mut config);
        }
        config
    }

    fn hosts(&self) -> Vec<ConfigHost> {
        let mut aliases = Vec::new();
        if let Some(text) = self.config() {
            self.aliases(&text, 0, &mut aliases);
        }
        aliases
            .into_iter()
            .map(|alias| {
                let config = self.resolve(&alias);
                ConfigHost { alias, config }
            })
            .collect()
    }

    /// Fills in what `text` says about `host`, where `config` has no value
    /// yet.
    fn read_into(&self, host: &str, text: &str, depth: u8, config: &mut HostConfig) {
        // Lines before any Host or Match apply to every host.
        let mut applies = true;
        for line in text.lines() {
            let words = words(line);
            let Some((keyword, arguments)) = words.split_first() else {
                continue;
            };
            let keyword = keyword.to_ascii_lowercase();
            match keyword.as_str() {
                "host" => applies = host_matches(host, arguments),
                "match" => applies = false,
                _ if !applies => {}
                "include" => {
                    for file in self.included(arguments, depth) {
                        if let Some(text) = (self.read)(&file) {
                            self.read_into(host, &text, depth + 1, config);
                        }
                    }
                }
                _ => self.set(host, &keyword, arguments, config),
            }
        }
    }

    /// A keyword's value, unless the keyword has one already.
    fn set(&self, host: &str, keyword: &str, arguments: &[String], config: &mut HostConfig) {
        let Some(value) = arguments.first() else {
            return;
        };
        match keyword {
            "hostname" => first(&mut config.host_name, || Some(tokens(value, host))),
            "port" => first(&mut config.port, || value.parse().ok()),
            "user" => first(&mut config.user, || Some(value.clone())),
            "identityfile" => first(&mut config.identity_file, || Some(self.expand(value))),
            "identityagent" => first(&mut config.identity_agent, || Some(self.socket(value))),
            "proxyjump" => first(&mut config.proxy, || {
                Some(proxy(value, || Proxy::Jump(value.clone())))
            }),
            "proxycommand" => first(&mut config.proxy, || {
                Some(proxy(value, || Proxy::Command(arguments.join(" "))))
            }),
            _ => {}
        }
    }

    /// The Host names in `text` and the files it includes, without
    /// patterns, added to `aliases` unless already there.
    fn aliases(&self, text: &str, depth: u8, aliases: &mut Vec<String>) {
        for line in text.lines() {
            let words = words(line);
            let Some((keyword, arguments)) = words.split_first() else {
                continue;
            };
            match keyword.to_ascii_lowercase().as_str() {
                "host" => {
                    for name in arguments.iter().flat_map(|names| names.split(',')) {
                        let concrete = !name.is_empty() && !name.contains(['*', '?', '!']);
                        if concrete && !aliases.iter().any(|known| known.eq_ignore_ascii_case(name))
                        {
                            aliases.push(name.to_owned());
                        }
                    }
                }
                // Followed whatever block it sits in: the list is every name
                // the config spells out.
                "include" => {
                    for file in self.included(arguments, depth) {
                        if let Some(text) = (self.read)(&file) {
                            self.aliases(&text, depth + 1, aliases);
                        }
                    }
                }
                _ => {}
            }
        }
    }

    /// The files an `Include` line names, none once includes nest too deep.
    fn included(&self, patterns: &[String], depth: u8) -> Vec<PathBuf> {
        if depth >= MAX_DEPTH {
            return Vec::new();
        }
        patterns
            .iter()
            .flat_map(|pattern| self.include(pattern))
            .collect()
    }
```

Then keep the existing `include`, `socket` and `expand` methods unchanged, inside this `impl Files<'_>` block (they only use `self.home` and `self.list`, which `Files` has). Close the impl, and add these helpers before `words`:

```rust
/// Sets `slot` from `value` unless it is set: the first value wins.
fn first<T>(slot: &mut Option<T>, value: impl FnOnce() -> Option<T>) {
    if slot.is_none() {
        *slot = value();
    }
}

/// `none`, or the proxy `set` builds.
fn proxy(value: &str, set: impl FnOnce() -> Proxy) -> Proxy {
    if value.eq_ignore_ascii_case("none") {
        Proxy::Off
    } else {
        set()
    }
}

/// `%h` as `host` and `%%` as `%`, as in `HostName %h.example.com`; other
/// tokens stay as written.
fn tokens(value: &str, host: &str) -> String {
    let mut out = String::with_capacity(value.len());
    let mut chars = value.chars();
    while let Some(c) = chars.next() {
        if c != '%' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('h') => out.push_str(host),
            Some('%') => out.push('%'),
            Some(other) => {
                out.push('%');
                out.push(other);
            }
            None => out.push('%'),
        }
    }
    out
}
```

- [ ] **Step 4: Make the module public and follow it in `ssh.rs`**

`crates/tabletist-db/src/lib.rs`: change `mod ssh_config;` to `pub mod ssh_config;`.

`crates/tabletist-db/src/ssh.rs`, in `async fn agent(host: &str)`: change `match crate::ssh_config::identity_agent(host) {` to `match crate::ssh_config::resolve(host).identity_agent {`.

- [ ] **Step 5: Run the tests to see them pass**

Run: `~/.cargo/bin/cargo test -p tabletist-db --lib ssh_config`
Expected: 13 passed.

Run: `~/.cargo/bin/cargo clippy --locked -p tabletist-db --all-targets -- -D warnings`
Expected: no warnings. If clippy flags `set` for too many arguments or similar, restructure rather than `allow`.

- [ ] **Step 6: Commit point**

Stop and report. If the user asks to commit:

```bash
git add crates/tabletist-db/src/ssh_config.rs crates/tabletist-db/src/lib.rs crates/tabletist-db/src/ssh.rs
git commit -S -m "Read HostName, Port, User, IdentityFile and ProxyJump from ~/.ssh/config" -m "The reader now fills a HostConfig for a host (first value wins per keyword, across Includes) and lists the concrete Host names, for the SSH tunnel to take a Host alias as ssh does." -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

## Task 2: SSH host keys are trusted by the host and port the tunnel reports

At this point the tunnel still reports the typed host and port; Task 3 makes them the resolved ones. Doing this first keeps every commit correct.

**Files:**
- Modify: `crates/tabletist-db/src/error.rs` (enum `SshStage`, `Display`, test)
- Modify: `crates/tabletist-db/src/ssh.rs` (`Tunnel::open`, the `UnknownKey` arm)
- Modify: `crates/tabletist-db/tests/ssh.rs` (`trusted`, two matches)
- Modify: `src/app.rs` (`ConnectFailed` handling near line 1310, `Event::Tested` near 1356, `refuse_to_trust` near 1588, tests in `mod ssh`)
- Modify: `src/ui/format.rs` (test near line 547)

- [ ] **Step 1: Write the failing app tests**

In `src/app.rs`, `mod ssh` inside the tests: change `unknown_key_with` to carry the host and port:

```rust
        fn unknown_key_with(fingerprint: &str) -> Error {
            Error::Ssh {
                stage: SshStage::HostKeyUnknown {
                    host: "bastion".into(),
                    port: 22,
                    fingerprint: fingerprint.into(),
                },
                message: "bastion is not a trusted host yet".into(),
            }
        }
```

In `a_changed_host_key_shows_the_error_and_no_prompt`, add `host: "bastion".into(), port: 22,` to the `HostKeyMismatch` literal. Then add two tests after `an_unknown_host_key_during_test_offers_trust_in_the_dialog`:

```rust
        /// A Host alias: the tunnel reached 10.0.0.5:2222 for "bastion".
        fn unknown_key_at_the_resolved_host() -> Error {
            Error::Ssh {
                stage: SshStage::HostKeyUnknown {
                    host: "10.0.0.5".into(),
                    port: 2222,
                    fingerprint: "SHA256:abc".into(),
                },
                message: "10.0.0.5 is not a trusted host yet".into(),
            }
        }

        #[test]
        fn connecting_trusts_the_host_and_port_the_tunnel_reached() {
            let (mut app, _dir) = app();
            let conn = ssh_saved(&mut app);
            let tab = app.active_tab_id();
            app.apply(Action::Connect { tab, conn });
            fail_last_connect(&mut app, unknown_key_at_the_resolved_host());
            match &app.dialog {
                Some(Dialog::HostKey(prompt)) => {
                    assert_eq!((prompt.host.as_str(), prompt.port), ("10.0.0.5", 2222))
                }
                other => panic!("{other:?}"),
            }
            app.apply(Action::TrustHostKey);
            assert_eq!(
                app.host_keys.fingerprint("10.0.0.5", 2222),
                Some("SHA256:abc")
            );
            assert_eq!(app.host_keys.fingerprint("bastion", 22), None);
        }

        #[test]
        fn testing_trusts_the_host_and_port_the_tunnel_reached() {
            let (mut app, _dir) = app();
            postgres_form(&mut app);
            let form = form(&mut app);
            form.ssh = true;
            form.ssh_host = "bastion".into();
            form.ssh_user = "ops".into();
            form.ssh_auth = SshAuthKind::Agent;
            app.apply(Action::TestConnection);
            let Some(Command::Test { request, .. }) = app.backend.sent.last() else {
                panic!("expected a Test");
            };
            let request = *request;
            app.apply(Action::Backend(Event::Tested {
                request,
                result: Err(unknown_key_at_the_resolved_host()),
            }));
            match &super::form(&mut app).test {
                TestState::Untrusted { host, port, .. } => {
                    assert_eq!((host.as_str(), *port), ("10.0.0.5", 2222))
                }
                other => panic!("{other:?}"),
            }
            app.apply(Action::TrustTestHostKey);
            assert_eq!(
                app.host_keys.fingerprint("10.0.0.5", 2222),
                Some("SHA256:abc")
            );
        }
```

(`HostKeyPrompt`'s fields are `pub`; if not, destructure it as the existing test does.)

- [ ] **Step 2: Run them to see them fail**

Run: `~/.cargo/bin/cargo test --locked -p tabletist --lib app::tests::ssh`
Expected: compile error, `HostKeyUnknown` has no field `host`.

- [ ] **Step 3: Add the fields**

`crates/tabletist-db/src/error.rs`:

```rust
    /// First connection to this host: the user must trust the key.
    HostKeyUnknown {
        /// The server the tunnel reached (a Host alias's HostName).
        host: String,
        port: u16,
        fingerprint: String,
    },
    /// The host's key changed since it was trusted. Refused.
    HostKeyMismatch {
        host: String,
        port: u16,
        fingerprint: String,
    },
```

In `Display`: `Self::HostKeyUnknown { fingerprint, .. } => ...` and `Self::HostKeyMismatch { fingerprint, .. } => ...`. In the test `ssh_errors_name_their_stage`, add `host: "bastion".into(), port: 22,` (the expected string does not change).

`crates/tabletist-db/src/ssh.rs`, in `Tunnel::open`'s `Err(russh::Error::UnknownKey)` arm:

```rust
                        SshStage::HostKeyMismatch {
                            host: ssh.host.clone(),
                            port: ssh.port,
                            fingerprint,
                        },
```

and likewise `SshStage::HostKeyUnknown { host: ssh.host.clone(), port: ssh.port, fingerprint }`.

`src/app.rs`, `ConnectFailed` handling (near line 1310): replace the `unknown_key` match with

```rust
                let unknown_key = match &error {
                    Error::Ssh {
                        stage:
                            SshStage::HostKeyUnknown {
                                host,
                                port,
                                fingerprint,
                            },
                        ..
                    } => Some((host.clone(), *port, fingerprint.clone())),
                    _ => None,
                };
```

`Event::Tested` (near line 1356):

```rust
                        Err(Error::Ssh {
                            stage:
                                SshStage::HostKeyUnknown {
                                    host,
                                    port,
                                    fingerprint,
                                },
                            ..
                        }) if self.host_keys_error.is_none() => TestState::Untrusted {
                            host,
                            port,
                            fingerprint,
                        },
```

`refuse_to_trust`: add `host: host.to_owned(), port,` to both the `HostKeyUnknown` and the `HostKeyMismatch` literals.

`src/ui/format.rs` test (near line 547): add `host: "bastion".into(), port: 22,`.

`crates/tabletist-db/tests/ssh.rs`:
- `trusted()`: destructure `SshStage::HostKeyUnknown { host, port, fingerprint }` and call `keys.trust(&host, port, &fingerprint);`.
- The two other matches: `SshStage::HostKeyUnknown { fingerprint, .. }` and `SshStage::HostKeyMismatch { fingerprint, .. }`.

- [ ] **Step 4: Run the tests to see them pass**

Run: `~/.cargo/bin/cargo test --locked --workspace --all-targets`
Expected: all pass (SSH integration tests print "skipped" without their variables).

- [ ] **Step 5: Commit point**

```bash
git add crates/tabletist-db/src/error.rs crates/tabletist-db/src/ssh.rs crates/tabletist-db/tests/ssh.rs src/app.rs src/ui/format.rs
git commit -S -m "Trust SSH host keys by the host and port the tunnel reached" -m "The host key errors carry the server's host and port, and both the connect prompt and the dialog's Test trust those rather than re-reading the spec or the form. A Host alias's key is then stored under its HostName, as known_hosts.json expects." -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

## Task 3: The SSH tunnel resolves a Host alias at connect time

**Files:**
- Modify: `crates/tabletist-db/src/spec.rs` (`SshAuth`, `SshSpec`, `summary`, tests)
- Modify: `crates/tabletist-db/src/ssh.rs` (`Endpoint`, `endpoint`, `authenticate`, `Tunnel::open`, tests)
- Modify: `crates/tabletist-db/tests/ssh.rs:28,113`
- Modify: `src/model.rs` (`Default`, `from_saved`, `to_spec`, tests near 1515-1570)
- Modify: `src/app.rs:4028`, `src/ui/mod.rs:1386`, `src/shots.rs:768` (`port: 22` to `port: Some(22)`)

- [ ] **Step 1: Write the failing spec tests**

In `crates/tabletist-db/src/spec.rs` tests: change `port: 22` to `port: Some(22)` in `specs_round_trip_through_json_and_old_files_load` and `the_summary_names_the_ssh_host`, and add:

```rust
    #[test]
    fn an_ssh_port_left_to_the_config_is_left_out_of_the_file() {
        let old: SshSpec = serde_json::from_str(
            r#"{"host": "bastion", "port": 22, "user": "ops", "auth": {"method": "agent"}}"#,
        )
        .unwrap();
        assert_eq!(old.port, Some(22));
        let alias: SshSpec =
            serde_json::from_str(r#"{"host": "bastion", "user": "", "auth": {"method": "agent"}}"#)
                .unwrap();
        assert_eq!(alias.port, None);
        assert!(!serde_json::to_string(&alias).unwrap().contains("port"));
    }

    #[test]
    fn the_summary_leaves_out_an_ssh_user_left_to_the_config() {
        let (mut spec, _) = ConnectSpec::from_url("postgres://me@db/app").unwrap();
        spec.ssh = Some(SshSpec {
            host: "bastion".into(),
            port: None,
            user: String::new(),
            auth: SshAuth::Agent,
        });
        assert_eq!(spec.summary(), "me@db:5432/app via bastion");
    }
```

- [ ] **Step 2: Write the failing endpoint tests**

In `crates/tabletist-db/src/ssh.rs` tests, add `use crate::ssh_config::{HostConfig, Proxy};` and:

```rust
    fn alias(auth: SshAuth) -> SshSpec {
        SshSpec {
            host: "bastion".into(),
            port: None,
            user: String::new(),
            auth,
        }
    }

    fn bastion_config() -> HostConfig {
        HostConfig {
            host_name: Some("10.0.0.5".into()),
            port: Some(2222),
            user: Some("ops".into()),
            identity_file: Some("/home/me/.ssh/id_work".into()),
            ..HostConfig::default()
        }
    }

    #[test]
    fn a_host_alias_takes_its_host_port_user_and_key_from_the_config() {
        let spec = alias(SshAuth::KeyFile {
            path: PathBuf::new(),
        });
        assert_eq!(
            endpoint(&spec, &bastion_config(), Some("me")).unwrap(),
            Endpoint {
                host: "10.0.0.5".into(),
                port: 2222,
                user: "ops".into(),
                key: Some("/home/me/.ssh/id_work".into()),
            }
        );
    }

    #[test]
    fn typed_values_win_over_the_config() {
        let spec = SshSpec {
            port: Some(2200),
            user: "deploy".into(),
            ..alias(SshAuth::KeyFile {
                path: "/keys/id_deploy".into(),
            })
        };
        assert_eq!(
            endpoint(&spec, &bastion_config(), Some("me")).unwrap(),
            Endpoint {
                host: "10.0.0.5".into(),
                port: 2200,
                user: "deploy".into(),
                key: Some("/keys/id_deploy".into()),
            }
        );
    }

    #[test]
    fn without_a_config_ssh_defaults_apply() {
        let spec = alias(SshAuth::Agent);
        assert_eq!(
            endpoint(&spec, &HostConfig::default(), Some("me")).unwrap(),
            Endpoint {
                host: "bastion".into(),
                port: 22,
                user: "me".into(),
                key: None,
            }
        );
    }

    #[test]
    fn a_proxy_the_tunnel_cannot_follow_is_refused_by_name() {
        for (proxy, keyword) in [
            (Proxy::Jump("gateway".into()), "ProxyJump"),
            (Proxy::Command("nc gateway 22".into()), "ProxyCommand"),
        ] {
            let config = HostConfig {
                proxy: Some(proxy),
                ..bastion_config()
            };
            match endpoint(&alias(SshAuth::Agent), &config, Some("me")) {
                Err(Error::Ssh {
                    stage: SshStage::Connect,
                    message,
                }) => {
                    assert!(message.contains(keyword), "{message}");
                    assert!(message.contains("bastion"), "{message}");
                }
                other => panic!("{keyword}: {other:?}"),
            }
        }
        let direct = HostConfig {
            proxy: Some(Proxy::Off),
            ..bastion_config()
        };
        assert!(endpoint(&alias(SshAuth::Agent), &direct, Some("me")).is_ok());
    }

    #[test]
    fn a_missing_host_user_or_key_is_an_invalid_spec() {
        let invalid = |result: Result<Endpoint>| match result {
            Err(Error::InvalidSpec(message)) => message,
            other => panic!("{other:?}"),
        };
        let empty = SshSpec {
            host: "  ".into(),
            ..alias(SshAuth::Agent)
        };
        assert!(invalid(endpoint(&empty, &HostConfig::default(), Some("me"))).contains("host"));
        assert!(
            invalid(endpoint(&alias(SshAuth::Agent), &HostConfig::default(), None))
                .contains("user")
        );
        let keyless = alias(SshAuth::KeyFile {
            path: PathBuf::new(),
        });
        assert!(invalid(endpoint(&keyless, &HostConfig::default(), Some("me"))).contains("key"));
    }
```

(`Endpoint` must derive `Debug, PartialEq, Eq`.)

- [ ] **Step 3: Write the failing form tests**

In `src/model.rs` tests: replace `an_ssh_form_needs_host_user_port_and_key_file` with

```rust
    #[test]
    fn an_ssh_form_needs_a_host_and_a_numeric_port() {
        type Change = fn(&mut ConnectionForm);
        let changes: [(Change, &str); 2] = [
            (|f| f.ssh_host.clear(), "SSH host"),
            (|f| f.ssh_port = "twenty-two".into(), "SSH port"),
        ];
        for (change, message) in changes {
            let mut form = ssh_form();
            change(&mut form);
            let error = form.to_spec().unwrap_err();
            assert!(error.contains(message), "{error}");
        }
    }

    #[test]
    fn empty_ssh_port_user_and_key_file_are_left_to_the_config() {
        let mut form = ssh_form();
        form.ssh_port.clear();
        form.ssh_user.clear();
        form.ssh_key_file.clear();
        let ssh = form.to_spec().unwrap().ssh.unwrap();
        assert_eq!(ssh.port, None);
        assert_eq!(ssh.user, "");
        assert_eq!(
            ssh.auth,
            SshAuth::KeyFile {
                path: PathBuf::new()
            }
        );
        let saved = form.to_saved().unwrap();
        assert_eq!(ConnectionForm::from_saved(&saved).ssh_port, "");
    }

    #[test]
    fn a_new_form_leaves_the_ssh_port_to_the_config() {
        assert_eq!(ConnectionForm::default().ssh_port, "");
    }
```

In `an_ssh_form_becomes_a_spec_and_back`, change the expected `port: 22` to `port: Some(22)`. Add `use std::path::PathBuf;` to the test module if it is not imported.

- [ ] **Step 4: Run to see them fail**

Run: `~/.cargo/bin/cargo test --locked --workspace --all-targets 2>&1 | head -40`
Expected: compile errors (`port` is `u16`, no `endpoint`/`Endpoint`).

- [ ] **Step 5: Change the spec**

`crates/tabletist-db/src/spec.rs`:

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "method", rename_all = "kebab-case")]
pub enum SshAuth {
    Password,
    /// An empty path takes `~/.ssh/config`'s IdentityFile.
    KeyFile { path: PathBuf },
    Agent,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SshSpec {
    /// A host name, or a Host alias from `~/.ssh/config`.
    pub host: String,
    /// `None` takes the config's Port, else 22.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub port: Option<u16>,
    /// Empty takes the config's User, else the login name.
    pub user: String,
    pub auth: SshAuth,
}
```

In `summary()`:

```rust
        if let Some(ssh) = &self.ssh {
            text.push_str(" via ");
            if !ssh.user.is_empty() {
                text.push_str(&ssh.user);
                text.push('@');
            }
            text.push_str(&ssh.host);
        }
```

- [ ] **Step 6: Add `endpoint` and use it in the tunnel**

`crates/tabletist-db/src/ssh.rs`: change the imports to

```rust
use std::path::{Path, PathBuf};
```

(drop the `#[cfg(test)] use std::path::PathBuf;`), and `use crate::ssh_config::{AgentSocket, HostConfig, Proxy};`. Add after `ssh_error`:

```rust
/// Where a tunnel goes and as whom, once `~/.ssh/config` has had its say.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Endpoint {
    pub host: String,
    pub port: u16,
    pub user: String,
    /// The key file, for the key file method.
    pub key: Option<PathBuf>,
}

/// `ssh`'s host, port, user and key file as `ssh -p 2222 -l me bastion`
/// takes them: a typed value wins, then the config's, then the default
/// (port 22, the `login` name). The host is the config's HostName, else the
/// alias itself.
pub(crate) fn endpoint(ssh: &SshSpec, config: &HostConfig, login: Option<&str>) -> Result<Endpoint> {
    let alias = ssh.host.trim();
    if alias.is_empty() {
        return Err(Error::InvalidSpec("enter the SSH host".into()));
    }
    let keyword = match &config.proxy {
        Some(Proxy::Jump(_)) => Some("ProxyJump"),
        Some(Proxy::Command(_)) => Some("ProxyCommand"),
        Some(Proxy::Off) | None => None,
    };
    if let Some(keyword) = keyword {
        return Err(ssh_error(
            SshStage::Connect,
            format!(
                "~/.ssh/config reaches {alias} through {keyword}, which Tabletist does not \
                 support yet"
            ),
        ));
    }
    let user = Some(ssh.user.trim())
        .filter(|user| !user.is_empty())
        .or(config.user.as_deref())
        .or(login)
        .ok_or_else(|| Error::InvalidSpec("enter the SSH user".into()))?;
    let key = match &ssh.auth {
        SshAuth::KeyFile { path } if path.as_os_str().is_empty() => Some(
            config
                .identity_file
                .clone()
                .ok_or_else(|| Error::InvalidSpec("choose a key file for SSH".into()))?,
        ),
        SshAuth::KeyFile { path } => Some(path.clone()),
        SshAuth::Password | SshAuth::Agent => None,
    };
    Ok(Endpoint {
        host: config
            .host_name
            .clone()
            .unwrap_or_else(|| alias.to_owned()),
        port: ssh.port.or(config.port).unwrap_or(22),
        user: user.to_owned(),
        key,
    })
}
```

Change `authenticate` to take the endpoint:

```rust
async fn authenticate(
    handle: &mut client::Handle<Client>,
    ssh: &SshSpec,
    endpoint: &Endpoint,
    secrets: &Secrets,
) -> Result<()> {
```

and inside it: every `&ssh.user` becomes `&endpoint.user`; the key file arm becomes

```rust
        SshAuth::KeyFile { .. } => {
            let path = endpoint.key.as_deref().unwrap_or(Path::new(""));
            let passphrase = secrets.ssh_passphrase.as_deref().filter(|p| !p.is_empty());
            let key = load_key(path, passphrase)?;
```

(the rest of the arm unchanged, with `&endpoint.user`); the agent arm keeps `agent(&ssh.host)` (OpenSSH matches Host patterns against the alias, not the HostName); the final error names `endpoint.user`.

In `Tunnel::open`, replace the empty host/user check with

```rust
        // `config` is taken below by russh's client config.
        let host_config = crate::ssh_config::resolve(ssh.host.trim());
        let login = crate::ssh_config::login();
        let endpoint = endpoint(ssh, &host_config, login.as_deref())?;
```

and from there on use `endpoint.host` / `endpoint.port` everywhere the function used `ssh.host` / `ssh.port`: the `host_keys.fingerprint(...)` lookups, `let address = (endpoint.host.clone(), endpoint.port);`, the timeout and "could not reach" messages, the two host key stages (`host: endpoint.host.clone(), port: endpoint.port`) and the "is not a trusted host yet" message. Call `authenticate(&mut handle, ssh, &endpoint, secrets).await?;`.

- [ ] **Step 7: Follow the form**

`src/model.rs`:
- `Default`: `ssh_port: String::new(),`.
- `from_saved`:

```rust
            ssh_port: spec
                .ssh
                .as_ref()
                .and_then(|ssh| ssh.port)
                .map(|port| port.to_string())
                .unwrap_or_default(),
```

- `to_spec`, the SSH block becomes:

```rust
                let ssh = if self.ssh {
                    let host = self.ssh_host.trim();
                    if host.is_empty() {
                        return Err("Enter the SSH host.".into());
                    }
                    // Empty port, user and key file are left to ~/.ssh/config.
                    let port = match self.ssh_port.trim() {
                        "" => None,
                        port => Some(
                            port.parse()
                                .map_err(|_| "Enter the SSH port number.".to_owned())?,
                        ),
                    };
                    let auth = match self.ssh_auth {
                        SshAuthKind::Password => SshAuth::Password,
                        SshAuthKind::Agent => SshAuth::Agent,
                        SshAuthKind::KeyFile => SshAuth::KeyFile {
                            path: self.ssh_key_file.trim().into(),
                        },
                    };
                    Some(SshSpec {
                        host: host.to_owned(),
                        port,
                        user: self.ssh_user.trim().to_owned(),
                        auth,
                    })
                } else {
                    None
                };
```

- [ ] **Step 8: Follow the other call sites**

- `src/app.rs:4030`, `src/ui/mod.rs:1386`, `src/shots.rs:770` (the `SshSpec` literals, not `HostKeyPrompt`'s `port`): `port: 22` to `port: Some(22)`.
- `crates/tabletist-db/tests/ssh.rs:28`: `port: Some(port.trim_end_matches('/').parse().unwrap()),`; line 113: `keys.trust(&ssh.host, ssh.port.unwrap_or(22), "SHA256:not-the-servers-key");`.
- Build and fix any remaining `SshSpec.port` use the compiler names.

- [ ] **Step 9: Run the tests to see them pass**

Run: `~/.cargo/bin/cargo test --locked --workspace --all-targets`
Expected: all pass.

- [ ] **Step 10: Commit point**

```bash
git add crates/tabletist-db src/model.rs src/app.rs src/ui/mod.rs src/shots.rs
git commit -S -m "Resolve an SSH Host alias from ~/.ssh/config at connect time" -m "The tunnel takes HostName, Port, User and IdentityFile from the config the way ssh does: a typed value wins, then the config's, then the default. The SSH port, user and key file may now be left empty. A host the config reaches through ProxyJump or ProxyCommand is refused with a clear error instead of being connected directly." -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

## Task 4: The form keeps the config's hosts and applies a picked alias

**Files:**
- Modify: `src/model.rs` (`ConnectionForm` fields, `Default`, `Debug`, new methods, `Action::PickSshHost`, tests)
- Modify: `src/backend.rs` (`Event::SshHosts`, `Backend::list_ssh_hosts`, test)
- Modify: `src/app.rs` (open dialog, `SshHosts`, `PickSshHost`, tests)

- [ ] **Step 1: Write the failing model tests**

In `src/model.rs` tests:

```rust
    use tabletist_db::ssh_config::{AgentSocket, ConfigHost, HostConfig, Proxy};

    fn config_host(alias: &str, config: HostConfig) -> ConfigHost {
        ConfigHost {
            alias: alias.into(),
            config,
        }
    }

    fn with_hosts() -> ConnectionForm {
        ConnectionForm {
            ssh_hosts: vec![
                config_host(
                    "bastion",
                    HostConfig {
                        host_name: Some("10.0.0.5".into()),
                        port: Some(2222),
                        user: Some("ops".into()),
                        identity_agent: Some(AgentSocket::Environment),
                        ..HostConfig::default()
                    },
                ),
                config_host(
                    "replica",
                    HostConfig {
                        identity_file: Some("/home/me/.ssh/id_work".into()),
                        ..HostConfig::default()
                    },
                ),
                config_host(
                    "gateway-only",
                    HostConfig {
                        proxy: Some(Proxy::Jump("bastion".into())),
                        ..HostConfig::default()
                    },
                ),
            ],
            ..ssh_form()
        }
    }

    #[test]
    fn the_typed_ssh_host_finds_its_config_host_ignoring_case() {
        let mut form = with_hosts();
        form.ssh_host = " BASTION ".into();
        assert_eq!(form.ssh_config_host().unwrap().alias, "bastion");
        form.ssh_host = "db.example.com".into();
        assert!(form.ssh_config_host().is_none());
    }

    #[test]
    fn picking_a_host_clears_what_the_config_supplies_and_presets_the_login() {
        let mut form = with_hosts();
        form.ssh_auth = SshAuthKind::Password;
        form.pick_ssh_host("bastion");
        assert_eq!(form.ssh_host, "bastion");
        assert_eq!(
            (form.ssh_port.as_str(), form.ssh_user.as_str(), form.ssh_key_file.as_str()),
            ("", "", "")
        );
        assert_eq!(form.ssh_auth, SshAuthKind::Agent, "IdentityAgent");
        form.pick_ssh_host("replica");
        assert_eq!(form.ssh_auth, SshAuthKind::KeyFile, "IdentityFile");
        form.ssh_auth = SshAuthKind::Password;
        form.pick_ssh_host("gateway-only");
        assert_eq!(form.ssh_auth, SshAuthKind::Password, "nothing to preset");
    }

    #[test]
    fn an_agent_turned_off_does_not_preset_the_agent() {
        let mut form = ConnectionForm {
            ssh_hosts: vec![config_host(
                "bastion",
                HostConfig {
                    identity_agent: Some(AgentSocket::Off),
                    ..HostConfig::default()
                },
            )],
            ssh_auth: SshAuthKind::Password,
            ..ssh_form()
        };
        form.pick_ssh_host("bastion");
        assert_eq!(form.ssh_auth, SshAuthKind::Password);
    }

    #[test]
    fn hints_come_from_the_config_host_or_are_empty() {
        let mut form = with_hosts();
        form.ssh_host = "bastion".into();
        let hints = form.ssh_hints();
        assert_eq!(hints.host_name.as_deref(), Some("10.0.0.5"));
        assert_eq!(hints.port, Some(2222));
        assert_eq!(hints.user.as_deref(), Some("ops"));
        assert_eq!(hints.key_file, None);
        assert_eq!(hints.proxy, None);
        form.ssh_host = "gateway-only".into();
        assert_eq!(
            form.ssh_hints().proxy,
            Some(Proxy::Jump("bastion".into()))
        );
        form.ssh_host = "db.example.com".into();
        assert_eq!(form.ssh_hints(), SshHints::default());
    }
```

- [ ] **Step 2: Write the failing app and backend tests**

`src/app.rs` tests (near `a_picked_key_file_fills_the_ssh_field`, in `mod ssh`):

```rust
        fn bastion_host() -> tabletist_db::ssh_config::ConfigHost {
            tabletist_db::ssh_config::ConfigHost {
                alias: "bastion".into(),
                config: tabletist_db::ssh_config::HostConfig {
                    identity_agent: Some(tabletist_db::ssh_config::AgentSocket::Environment),
                    ..Default::default()
                },
            }
        }

        #[test]
        fn opening_the_dialog_asks_for_the_config_hosts_and_takes_only_its_answer() {
            let (mut app, _dir) = app();
            app.apply(Action::NewConnection);
            let request = form(&mut app).ssh_hosts_request.expect("asked");
            app.apply(Action::Backend(Event::SshHosts {
                request: RequestId(request.0 + 1000),
                hosts: vec![bastion_host()],
            }));
            assert!(form(&mut app).ssh_hosts.is_empty(), "a stale answer");
            app.apply(Action::Backend(Event::SshHosts {
                request,
                hosts: vec![bastion_host()],
            }));
            assert_eq!(form(&mut app).ssh_hosts, vec![bastion_host()]);
            assert_eq!(form(&mut app).ssh_hosts_request, None);
        }

        #[test]
        fn editing_asks_for_the_config_hosts_too() {
            let (mut app, _dir) = app();
            let id = ssh_saved(&mut app);
            app.apply(Action::EditConnection(id));
            assert!(form(&mut app).ssh_hosts_request.is_some());
        }

        #[test]
        fn picking_a_config_host_fills_the_ssh_fields() {
            let (mut app, _dir) = app();
            postgres_form(&mut app);
            let form = form(&mut app);
            form.ssh_hosts = vec![bastion_host()];
            form.ssh_user = "someone".into();
            app.apply(Action::PickSshHost("bastion".into()));
            let form = super::form(&mut app);
            assert_eq!(form.ssh_host, "bastion");
            assert_eq!(form.ssh_user, "");
            assert_eq!(form.ssh_auth, SshAuthKind::Agent);
        }
```

`src/backend.rs` tests (next to the other `Backend::start(Waker::default())` tests):

```rust
    #[test]
    fn listing_ssh_hosts_answers_the_request() {
        let mut backend = Backend::start(Waker::default());
        backend.list_ssh_hosts(RequestId(9));
        match backend.wait(Duration::from_secs(5)) {
            Some(Event::SshHosts { request, .. }) => assert_eq!(request, RequestId(9)),
            other => panic!("{other:?}"),
        }
    }
```

(Use the `Duration` import those tests already use.)

- [ ] **Step 3: Run to see them fail**

Run: `~/.cargo/bin/cargo test --locked -p tabletist --lib 2>&1 | head -30`
Expected: compile errors (`ssh_hosts`, `SshHosts`, `PickSshHost`, ... not found).

- [ ] **Step 4: Add the backend event and method**

`src/backend.rs`, in `Event`:

```rust
    /// The Host aliases in ~/.ssh/config, for the connection dialog.
    SshHosts {
        request: RequestId,
        hosts: Vec<tabletist_db::ssh_config::ConfigHost>,
    },
```

In `impl Backend`, after `pick_key_file`:

```rust
    /// Reads the Host aliases in ~/.ssh/config off the UI thread.
    pub fn list_ssh_hosts(&mut self, request: RequestId) {
        let Some(runtime) = &self.runtime else {
            return;
        };
        let outbox = self.outbox.clone();
        runtime.spawn_blocking(move || {
            let hosts = tabletist_db::ssh_config::hosts();
            outbox.emit(Event::SshHosts { request, hosts });
        });
    }
```

- [ ] **Step 5: Add the form state and methods**

`src/model.rs`, import `use tabletist_db::ssh_config::{AgentSocket, ConfigHost, Proxy};` at the top. In `Action`, after `PickKeyFile`:

```rust
    /// Put this ~/.ssh/config Host alias in the SSH host field.
    PickSshHost(String),
```

In `ConnectionForm`, after `saved_ssh_auth`:

```rust
    /// The Host aliases in ~/.ssh/config, for the SSH host list.
    pub ssh_hosts: Vec<ConfigHost>,
    /// The request whose answer fills `ssh_hosts`.
    pub ssh_hosts_request: Option<RequestId>,
```

`Default`: `ssh_hosts: Vec::new(), ssh_hosts_request: None,`. In the manual `Debug` impl, add `.field("ssh_hosts", &self.ssh_hosts.len())` next to the other SSH fields if it lists them; otherwise leave it.

Add next to `SshAuthKind`:

```rust
/// What `~/.ssh/config` gives the typed SSH host, for the dialog's hints.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct SshHints {
    pub host_name: Option<String>,
    pub port: Option<u16>,
    pub user: Option<String>,
    pub key_file: Option<String>,
    /// A ProxyJump or ProxyCommand the tunnel cannot follow.
    pub proxy: Option<Proxy>,
}
```

and in `impl ConnectionForm`:

```rust
    /// The config's Host whose alias is the typed SSH host, compared as
    /// OpenSSH compares Host names (ignoring case).
    pub fn ssh_config_host(&self) -> Option<&ConfigHost> {
        let host = self.ssh_host.trim();
        self.ssh_hosts
            .iter()
            .find(|known| known.alias.eq_ignore_ascii_case(host))
    }

    /// Puts `alias` in SSH host and clears the fields its config fills, so
    /// the config's values show; presets the login method the config
    /// implies (its agent, else its key file).
    pub fn pick_ssh_host(&mut self, alias: &str) {
        self.ssh_host = alias.to_owned();
        self.ssh_port.clear();
        self.ssh_user.clear();
        self.ssh_key_file.clear();
        let preset = self.ssh_config_host().and_then(|host| {
            let config = &host.config;
            if matches!(
                config.identity_agent,
                Some(AgentSocket::Path(_) | AgentSocket::Environment)
            ) {
                Some(SshAuthKind::Agent)
            } else {
                config.identity_file.is_some().then_some(SshAuthKind::KeyFile)
            }
        });
        if let Some(kind) = preset {
            self.ssh_auth = kind;
        }
    }

    /// The typed SSH host's values from ~/.ssh/config: empty for a host
    /// the config does not spell out.
    pub fn ssh_hints(&self) -> SshHints {
        let Some(host) = self.ssh_config_host() else {
            return SshHints::default();
        };
        let config = &host.config;
        SshHints {
            host_name: config.host_name.clone(),
            port: config.port,
            user: config.user.clone(),
            key_file: config
                .identity_file
                .as_ref()
                .map(|path| path.display().to_string()),
            proxy: config.proxy.clone().filter(|proxy| *proxy != Proxy::Off),
        }
    }
```

- [ ] **Step 6: Wire the app**

`src/app.rs`:

```rust
            Action::NewConnection => {
                self.dialog = Some(Dialog::Connection(Box::default()));
                self.list_ssh_hosts();
            }
            Action::EditConnection(id) => {
                if let Some(saved) = self.connections.get(&id) {
                    self.dialog = Some(Dialog::Connection(Box::new(ConnectionForm::from_saved(
                        saved,
                    ))));
                    self.list_ssh_hosts();
                }
            }
```

After `Action::PickKeyFile`:

```rust
            Action::PickSshHost(alias) => {
                if let Some(Dialog::Connection(form)) = &mut self.dialog {
                    form.pick_ssh_host(&alias);
                }
            }
```

In `apply_event`, next to `Event::FilePicked`:

```rust
            Event::SshHosts { request, hosts } => {
                if let Some(Dialog::Connection(form)) = &mut self.dialog
                    && form.ssh_hosts_request == Some(request)
                {
                    form.ssh_hosts_request = None;
                    form.ssh_hosts = hosts;
                }
            }
```

And a method next to `trust`:

```rust
    /// Asks the backend for ~/.ssh/config's Host aliases for the open
    /// connection dialog.
    fn list_ssh_hosts(&mut self) {
        let request = RequestId(self.next_id());
        if let Some(Dialog::Connection(form)) = &mut self.dialog {
            form.ssh_hosts_request = Some(request);
            self.backend.list_ssh_hosts(request);
        }
    }
```

If a `match` over `Event` elsewhere (for example a log or a test helper) is exhaustive, add the `SshHosts` arm the compiler asks for.

- [ ] **Step 7: Run to see them pass**

Run: `~/.cargo/bin/cargo test --locked --workspace --all-targets`
Expected: all pass. Existing tests that assert on `backend.sent` do not shift, because `list_ssh_hosts` is not a `Command`.

- [ ] **Step 8: No commit point yet** (Task 5 finishes the same topic). Go on.

---

## Task 5: The dialog lists the aliases and shows the config's values

**Files:**
- Modify: `src/ui/connect_dialog.rs` (`ssh_section`)
- Test: `src/ui/mod.rs` (headless tests next to `the_ssh_section_shows_the_fields_for_each_method`)

- [ ] **Step 1: Write the failing UI tests**

In `src/ui/mod.rs` tests, after `the_ssh_section_shows_the_fields_for_each_method`:

```rust
    use tabletist_db::ssh_config::{AgentSocket, ConfigHost, HostConfig, Proxy};

    fn config_host(alias: &str, config: HostConfig) -> ConfigHost {
        ConfigHost {
            alias: alias.into(),
            config,
        }
    }

    fn bastion() -> ConfigHost {
        config_host(
            "bastion",
            HostConfig {
                host_name: Some("10.0.0.5".into()),
                port: Some(2222),
                user: Some("ops".into()),
                identity_agent: Some(AgentSocket::Environment),
                ..HostConfig::default()
            },
        )
    }

    /// A new PostgreSQL connection with the SSH section open and these
    /// hosts from ~/.ssh/config.
    fn ssh_dialog(hosts: Vec<ConfigHost>) -> Harness {
        let mut harness = Harness::new();
        harness.press(Key::N, Modifiers::COMMAND);
        harness.click("PostgreSQL");
        harness.click("SSH tunnel");
        harness.click("Connect through SSH");
        ssh_form(&mut harness).ssh_hosts = hosts;
        harness.settle();
        harness
    }

    fn ssh_form(harness: &mut Harness) -> &mut crate::model::ConnectionForm {
        match &mut harness.app.dialog {
            Some(crate::model::Dialog::Connection(form)) => form,
            other => panic!("{other:?}"),
        }
    }

    /// The placeholder of the field the `label` label names.
    fn placeholder(harness: &mut Harness, label: &str) -> Option<String> {
        let tree = harness.settle();
        let (label, _) = tree
            .nodes
            .iter()
            .find(|(_, node)| node.value() == Some(label))
            .unwrap_or_else(|| panic!("no {label:?} label"));
        tree.nodes
            .iter()
            .find(|(_, node)| {
                node.role() == egui::accesskit::Role::TextInput
                    && node.labelled_by().contains(label)
            })
            .expect("a field")
            .1
            .placeholder()
            .map(str::to_owned)
    }

    #[test]
    fn the_config_hosts_button_shows_only_when_the_config_names_hosts() {
        let mut harness = ssh_dialog(Vec::new());
        assert!(!harness.has("Hosts from ~/.ssh/config"));
        let mut harness = ssh_dialog(vec![bastion()]);
        assert!(harness.has("Hosts from ~/.ssh/config"));
    }

    #[test]
    fn choosing_a_config_host_fills_the_ssh_host() {
        let replica = config_host("replica", HostConfig::default());
        let mut harness = ssh_dialog(vec![bastion(), replica]);
        harness.click("Hosts from ~/.ssh/config");
        assert!(harness.has("replica"));
        harness.click("bastion");
        let form = ssh_form(&mut harness);
        assert_eq!(form.ssh_host, "bastion");
        assert_eq!(form.ssh_auth, crate::model::SshAuthKind::Agent);
    }

    #[test]
    fn a_config_host_shows_its_values_as_hints() {
        let mut harness = ssh_dialog(vec![bastion()]);
        assert_eq!(placeholder(&mut harness, "SSH port").as_deref(), Some("22"));
        ssh_form(&mut harness).ssh_host = "bastion".into();
        assert_eq!(placeholder(&mut harness, "SSH port").as_deref(), Some("2222"));
        assert_eq!(
            placeholder(&mut harness, "SSH user").as_deref(),
            Some("ops (from ~/.ssh/config)")
        );
        assert!(harness.has("10.0.0.5 from ~/.ssh/config"));
    }

    #[test]
    fn a_config_host_behind_a_proxy_warns() {
        let jump = config_host(
            "gateway-only",
            HostConfig {
                proxy: Some(Proxy::Jump("bastion".into())),
                ..HostConfig::default()
            },
        );
        let mut harness = ssh_dialog(vec![jump]);
        ssh_form(&mut harness).ssh_host = "gateway-only".into();
        assert!(harness.has("Uses ProxyJump, which Tabletist does not support yet."));
    }
```

- [ ] **Step 2: Run them to see them fail**

Run: `~/.cargo/bin/cargo test --locked -p tabletist --lib config_host`
Expected: the four new tests fail (no button, no hints, no line).

- [ ] **Step 3: Draw the host list, the hints and the host line**

`src/ui/connect_dialog.rs`: import `use tabletist_db::ssh_config::Proxy;` and `use crate::theme::Icon;` (check the path `Icon` is exported from, as `conn_tabs.rs` uses it). In `ssh_section`, compute once before the grid:

```rust
            let hints = form.ssh_hints();
            let from_config = gettext(locale, "from ~/.ssh/config");
            let login = tabletist_db::ssh_config::login().unwrap_or_default();
```

Replace the SSH host row with:

```rust
                        let label_ssh_host = ui.label(gettext(locale, "SSH host")).id;
                        ui.horizontal(|ui| {
                            let listed = !form.ssh_hosts.is_empty();
                            ui.add(
                                crate::ui::widgets::single(ui, &mut form.ssh_host, look)
                                    .desired_width(if listed { 222.0 } else { 250.0 }),
                            )
                            .labelled_by(label_ssh_host);
                            if listed {
                                let name = gettext(locale, "Hosts from ~/.ssh/config");
                                let button = crate::ui::widgets::icon_button(
                                    ui,
                                    Icon::ChevronDown,
                                    &name,
                                    look,
                                    palette,
                                );
                                egui::Popup::menu(&button).show(|ui| {
                                    for host in &form.ssh_hosts {
                                        ui.horizontal(|ui| {
                                            if crate::ui::widgets::button(ui, &host.alias, look)
                                                .clicked()
                                            {
                                                actions.push(Action::PickSshHost(
                                                    host.alias.clone(),
                                                ));
                                            }
                                            if let Some(name) = &host.config.host_name {
                                                crate::ui::widgets::label(
                                                    ui,
                                                    crate::ui::widgets::secondary(look),
                                                    name,
                                                    palette.dim,
                                                    look,
                                                );
                                            }
                                        });
                                    }
                                });
                            }
                            let label_ssh_port = ui.label(gettext(locale, "SSH port")).id;
                            let port_hint = hints.port.unwrap_or(22).to_string();
                            ui.add(
                                crate::ui::widgets::single(ui, &mut form.ssh_port, look)
                                    .hint_text(port_hint)
                                    .desired_width(60.0),
                            )
                            .labelled_by(label_ssh_port);
                        });
                        ui.end_row();

                        // What the alias resolves to, or why the tunnel
                        // cannot follow it.
                        let proxy = match &hints.proxy {
                            Some(Proxy::Jump(_)) => Some(gettext(
                                locale,
                                "Uses ProxyJump, which Tabletist does not support yet.",
                            )),
                            Some(Proxy::Command(_)) => Some(gettext(
                                locale,
                                "Uses ProxyCommand, which Tabletist does not support yet.",
                            )),
                            Some(Proxy::Off) | None => None,
                        };
                        if let Some(warning) = proxy {
                            ui.label("");
                            crate::ui::widgets::label(
                                ui,
                                crate::ui::widgets::secondary(look),
                                &warning,
                                palette.warning,
                                look,
                            );
                            ui.end_row();
                        } else if let Some(name) = &hints.host_name {
                            ui.label("");
                            crate::ui::widgets::label(
                                ui,
                                crate::ui::widgets::secondary(look),
                                &format!("{name} {from_config}"),
                                palette.dim,
                                look,
                            );
                            ui.end_row();
                        }
```

The SSH user field gets a hint:

```rust
                        let user_hint = match &hints.user {
                            Some(user) => format!("{user} ({from_config})"),
                            None => login.clone(),
                        };
                        let label_ssh_user = ui.label(gettext(locale, "SSH user")).id;
                        ui.add(
                            crate::ui::widgets::single(ui, &mut form.ssh_user, look)
                                .hint_text(user_hint)
                                .desired_width(f32::INFINITY),
                        )
                        .labelled_by(label_ssh_user);
                        ui.end_row();
```

And the key file field:

```rust
                                let key_hint = hints
                                    .key_file
                                    .as_ref()
                                    .map(|path| format!("{path} ({from_config})"))
                                    .unwrap_or_default();
                                ui.add(
                                    crate::ui::widgets::single(ui, &mut form.ssh_key_file, look)
                                        .hint_text(key_hint)
                                        .desired_width(300.0),
                                )
                                .labelled_by(label_key_file);
```

Notes:
- `hints` is computed before the grid, so it holds no borrow of `form` while the fields borrow it mutably. `form.ssh_hosts` is read inside the popup while `form.ssh_host` was borrowed by the earlier `ui.add`, which has ended; if the borrow checker still objects, clone the list (`let hosts = form.ssh_hosts.clone();`) before the row.
- `gettext` returns a `Cow`; use `&*` / `.to_string()` where a `&str` or `String` is needed, as the surrounding code does.
- `widgets::button`, `widgets::label`, `widgets::secondary` draw through `TextRole`s; do not add fonts or sizes.

- [ ] **Step 4: Run the tests to see them pass**

Run: `~/.cargo/bin/cargo test --locked -p tabletist --lib ui::tests`
Expected: all pass. If `choosing_a_config_host_fills_the_ssh_host` cannot find "replica" after the click, the popup did not open from an AccessKit click. Check `egui::Popup::menu`'s open trigger (it opens on `response.clicked()`), and read how `src/ui/workspace.rs:360` gets opened in tests before changing the approach.

- [ ] **Step 5: Look at it (local only)**

If a shots scene with the connection dialog exists (`grep -n "Connection" src/shots.rs`), run it with `--features shots`, using the neutral Bookshop data, and look at the PNG locally to check the ▾ button lines up with the host field and the hint line reads well. Do not commit or upload screenshots, and do not add a pixel test.

- [ ] **Step 6: Commit point**

```bash
git add src/model.rs src/backend.rs src/app.rs src/ui/connect_dialog.rs src/ui/mod.rs
git commit -S -m "List ~/.ssh/config Host aliases in the connection dialog" -m "A button beside SSH host lists the concrete Host names from ~/.ssh/config, read on the backend when the dialog opens. Choosing one fills the alias, clears the fields the config supplies and presets the login method. Empty fields show the config's values as hints, with the HostName under the host, or a warning when the host needs ProxyJump or ProxyCommand." -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

## Task 6: Verification before completion

- [ ] **Step 1: Run every check and read the output**

```bash
~/.cargo/bin/cargo fmt --all --check
~/.cargo/bin/cargo clippy --locked --workspace --all-targets -- -D warnings
~/.cargo/bin/cargo clippy --locked --workspace --all-targets --features shots -- -D warnings
~/.cargo/bin/cargo test --locked --workspace --all-targets
RUSTDOCFLAGS='-D warnings' ~/.cargo/bin/cargo doc --locked --workspace --no-deps
```

Expected: fmt prints nothing; both clippy runs finish without warnings; tests all pass; doc builds without warnings. Fix and re-run until all five are clean.

- [ ] **Step 2: Grep for em dashes in the change**

Run: `git diff main --stat && git diff main | grep -nP "\x{2014}"`
Expected: no output from the grep.

- [ ] **Step 3: Optional integration run**

If Docker is available: `docker compose up -d --build --wait postgres mysql ssh`, then run the SSH suite with `TABLETIST_TEST_SSH_URL=ssh://tabletist:tabletist@localhost:52222`. These read the machine's real `~/.ssh/config`; a failure caused by a local `Host *` HostName or ProxyJump is expected and should be reported as such, not "fixed".

- [ ] **Step 4: Report honestly**

Say what ran and what passed, that it was built and tested on Linux only (macOS and Windows compiled paths unchanged but not run), and whether the integration suite ran.

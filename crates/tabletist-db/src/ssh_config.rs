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

    /// The files an `Include` pattern names: relative to `~/.ssh`, the last
    /// part a glob, in sorted order.
    fn include(&self, pattern: &str) -> Vec<PathBuf> {
        let path = self.expand(pattern);
        let path = if path.is_absolute() {
            path
        } else {
            self.home.join(".ssh").join(path)
        };
        let name = path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default();
        if !name.contains(['*', '?']) {
            return vec![path];
        }
        let dir = path.parent().unwrap_or(Path::new("/"));
        let mut files: Vec<PathBuf> = (self.list)(dir)
            .into_iter()
            .filter(|file| {
                file.file_name()
                    .is_some_and(|file| wildcard(&name, &file.to_string_lossy()))
            })
            .collect();
        files.sort();
        files
    }

    fn socket(&self, value: &str) -> AgentSocket {
        if value.eq_ignore_ascii_case("none") {
            return AgentSocket::Off;
        }
        if value == "SSH_AUTH_SOCK" {
            return AgentSocket::Environment;
        }
        if let Some(variable) = value.strip_prefix('$') {
            return std::env::var_os(variable)
                .map_or(AgentSocket::Off, |path| AgentSocket::Path(path.into()));
        }
        AgentSocket::Path(self.expand(value))
    }

    /// `~` and `%d` as the home directory.
    fn expand(&self, value: &str) -> PathBuf {
        let home = self.home.to_string_lossy();
        let value = value.replace("%d", &home);
        match value.strip_prefix("~/") {
            Some(rest) => self.home.join(rest),
            None if value == "~" => self.home.to_path_buf(),
            None => PathBuf::from(value),
        }
    }
}

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

/// A config line's words: the keyword (`Keyword value` or `Keyword=value`)
/// and its arguments, double quotes keeping spaces; `#` starts a comment.
fn words(line: &str) -> Vec<String> {
    let line = line.trim();
    if line.is_empty() || line.starts_with('#') {
        return Vec::new();
    }
    let (keyword, rest) = match line.find(|c: char| c.is_whitespace() || c == '=') {
        Some(at) => (&line[..at], &line[at..]),
        None => (line, ""),
    };
    let rest = rest.trim_start();
    let rest = rest.strip_prefix('=').unwrap_or(rest);
    let mut words = vec![keyword.to_owned()];
    let mut word = String::new();
    let mut quoted = false;
    let mut started = false;
    for c in rest.chars() {
        match c {
            '"' => {
                quoted = !quoted;
                started = true;
            }
            '#' if !quoted && !started => break,
            c if c.is_whitespace() && !quoted => {
                if started {
                    words.push(std::mem::take(&mut word));
                    started = false;
                }
            }
            c => {
                word.push(c);
                started = true;
            }
        }
    }
    if started {
        words.push(word);
    }
    words
}

/// Whether `Host` `patterns` (split on spaces and commas) take in `host`.
fn host_matches(host: &str, patterns: &[String]) -> bool {
    let host = host.to_lowercase();
    let mut matched = false;
    for pattern in patterns.iter().flat_map(|patterns| patterns.split(',')) {
        let pattern = pattern.to_lowercase();
        if let Some(negated) = pattern.strip_prefix('!') {
            if wildcard(negated, &host) {
                return false;
            }
        } else if wildcard(&pattern, &host) {
            matched = true;
        }
    }
    matched
}

/// `*` and `?` wildcards over the whole of `text`.
fn wildcard(pattern: &str, text: &str) -> bool {
    let pattern: Vec<char> = pattern.chars().collect();
    let text: Vec<char> = text.chars().collect();
    let (mut p, mut t) = (0, 0);
    let (mut star, mut resume) = (None, 0);
    while t < text.len() {
        if p < pattern.len() && (pattern[p] == '?' || pattern[p] == text[t]) {
            p += 1;
            t += 1;
        } else if p < pattern.len() && pattern[p] == '*' {
            star = Some(p);
            resume = t;
            p += 1;
        } else if let Some(at) = star {
            p = at + 1;
            resume += 1;
            t = resume;
        } else {
            return false;
        }
    }
    pattern[p..].iter().all(|c| *c == '*')
}

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

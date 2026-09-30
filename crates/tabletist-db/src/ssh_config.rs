//! What `~/.ssh/config` says about an SSH host, as OpenSSH reads it. Only
//! `IdentityAgent` for now: keys kept in 1Password, Secretive or another
//! agent work as they do in a terminal, including when the app starts from
//! the Dock without that agent's `SSH_AUTH_SOCK`.
//!
//! The rules OpenSSH follows, and so this: the first value a keyword gets
//! wins; a `Host` line applies to the host when one of its patterns (`*`,
//! `?`) matches and no negated one (`!`) does; `Include` reads other files
//! in place (relative to `~/.ssh`, with globs); `Match` blocks are not
//! evaluated, so what they set is ignored.

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

/// The agent `~/.ssh/config` names for `host`, if it names one.
pub fn identity_agent(host: &str) -> Option<AgentSocket> {
    let home = home()?;
    let text = std::fs::read_to_string(home.join(".ssh").join("config")).ok()?;
    let mut reader = Reader {
        host,
        home: &home,
        read: &|path| std::fs::read_to_string(path).ok(),
        list: &list,
        depth: 0,
    };
    reader.agent(&text)
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

/// Reads config text for one host. Files and directories come through
/// `read` and `list`, so tests need no disk.
struct Reader<'a> {
    host: &'a str,
    home: &'a Path,
    read: &'a dyn Fn(&Path) -> Option<String>,
    list: &'a dyn Fn(&Path) -> Vec<PathBuf>,
    /// Include depth, bounded as OpenSSH bounds it.
    depth: u8,
}

impl Reader<'_> {
    /// The first `IdentityAgent` that applies to the host in `text`.
    fn agent(&mut self, text: &str) -> Option<AgentSocket> {
        // Lines before any Host or Match apply to every host.
        let mut applies = true;
        for line in text.lines() {
            let words = words(line);
            let Some((keyword, arguments)) = words.split_first() else {
                continue;
            };
            match keyword.to_ascii_lowercase().as_str() {
                "host" => applies = host_matches(self.host, arguments),
                "match" => applies = false,
                "include" if applies && self.depth < 16 => {
                    for pattern in arguments {
                        for file in self.include(pattern) {
                            let Some(text) = (self.read)(&file) else {
                                continue;
                            };
                            self.depth += 1;
                            let found = self.agent(&text);
                            self.depth -= 1;
                            if found.is_some() {
                                return found;
                            }
                        }
                    }
                }
                "identityagent" if applies => {
                    if let Some(value) = arguments.first() {
                        return Some(self.socket(value));
                    }
                }
                _ => {}
            }
        }
        None
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

    /// `host`'s agent in `config`, with `files` beside it in `~/.ssh`.
    fn agent(config: &str, files: &[(&str, &str)], host: &str) -> Option<AgentSocket> {
        let home = PathBuf::from("/home/me");
        let files: BTreeMap<PathBuf, String> = files
            .iter()
            .map(|(name, text)| (home.join(".ssh").join(name), (*text).to_owned()))
            .collect();
        let read = |path: &Path| files.get(path).cloned();
        let list = |dir: &Path| {
            files
                .keys()
                .filter(|path| path.parent() == Some(dir))
                .cloned()
                .collect()
        };
        Reader {
            host,
            home: &home,
            read: &read,
            list: &list,
            depth: 0,
        }
        .agent(config)
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
}

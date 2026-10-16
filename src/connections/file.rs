//! `connections.toml` as text: the one form the app writes, and what it
//! reads.
//!
//! The app writes the file itself, as it writes `settings.toml`: the same
//! text for the same connections, a key on each line. It reads the file
//! strictly. A key it does not know is an error and not a key it skips: in
//! a file a person edits, `read_onyl = true` must not pass for a
//! connection that may write.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use serde::Deserialize;
use tabletist_db::agent::{TableName, Tables};
use tabletist_db::{ConnectSpec, Driver, SshAuth, SshSpec, TlsMode};

use super::{
    AgentAccess, Agents, ConnectionId, MAX_AGENT_ROWS, MAX_AGENT_TIMEOUT_SECS, PasswordMode,
    SavedConnection,
};
use crate::env::Environment;
use crate::util::toml_string;

/// The file's format. 3 is the first that is TOML: 1 and 2 were
/// `connections.json`. 4 has a connection's settings for agents, and 5 the
/// tables they may not read. A file is written as the oldest that holds
/// what it has: a Tabletist that does not know a key refuses only a file it
/// would lose it from, and one that would read agents' tables as all of
/// them never reads the file.
pub(super) const VERSION: u32 = 5;

/// The version that has agents' settings and not their tables.
const AGENTS_VERSION: u32 = 4;

/// The first version of this file.
const FIRST_VERSION: u32 = 3;

/// The file's name, as the user is told it.
pub(super) const NAME: &str = "connections.toml";

const HEADER: &str = "\
# Tabletist's saved connections. No password is kept here: those are in the
# system's keyring.
#
# Edit it while Tabletist is closed. A connection without an `id` line gets
# one at the next start. Tabletist writes the whole file again when a
# connection changes, and keeps no comment but these.
";

/// What the file holds, read from its text.
#[derive(Debug, PartialEq)]
pub(super) struct Read {
    pub connections: Vec<SavedConnection>,
    /// Whether a connection had no id and was given one: the file is to
    /// be written again, so that it keeps the id.
    pub named: bool,
}

/// Why a file's text could not be read: the line, where the parser gives
/// one, and what is wrong there.
#[derive(Debug, PartialEq, Eq)]
pub(super) struct Problem {
    pub line: Option<usize>,
    pub what: String,
}

impl std::fmt::Display for Problem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.line {
            Some(line) => write!(f, "line {line}: {}", self.what),
            None => f.write_str(&self.what),
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct File {
    version: u32,
    #[serde(default)]
    connections: Vec<Entry>,
}

/// A connection as the file has it: its spec's keys beside its own, and
/// the SSH tunnel as a table of its own.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Entry {
    id: Option<String>,
    name: String,
    #[serde(default)]
    environment: Environment,
    read_only: Option<bool>,
    #[serde(default)]
    password: PasswordMode,
    #[serde(default)]
    ssh_secret: PasswordMode,
    driver: Driver,
    #[serde(default)]
    host: String,
    port: Option<u16>,
    #[serde(default)]
    user: String,
    #[serde(default)]
    database: String,
    sqlite_path: Option<PathBuf>,
    #[serde(default)]
    tls: TlsMode,
    ca_file: Option<PathBuf>,
    ssh: Option<Tunnel>,
    agents: Option<AgentsTable>,
}

/// What agents may do on a connection, as the file has it. A key left out
/// takes what a connection starts with.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct AgentsTable {
    #[serde(default)]
    access: AgentAccess,
    rows: Option<u32>,
    timeout_secs: Option<u32>,
    /// The tables agents may not read, every other being readable.
    hidden: Option<Vec<String>>,
    /// The tables agents may read, and no others.
    only: Option<Vec<String>>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Tunnel {
    host: String,
    port: Option<u16>,
    #[serde(default)]
    user: String,
    auth: Auth,
    /// The key file, for `auth = "key-file"` and no other.
    key: Option<PathBuf>,
}

#[derive(Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
enum Auth {
    Password,
    KeyFile,
    Agent,
}

/// The line `offset` is on in `text`, counted from 1.
fn line_of(text: &str, offset: usize) -> usize {
    let before = &text.as_bytes()[..offset.min(text.len())];
    before.iter().filter(|byte| **byte == b'\n').count() + 1
}

/// The parser's word for what is wrong, without the value it quotes: a
/// problem is logged and shown, and a person may have written a password
/// where the file asks how the password is kept (`password = "hunter2"`).
/// The line says where to look. A key's name is no value, and stays.
fn without_values(message: &str) -> String {
    let said = if message.starts_with("unknown variant ") || message.starts_with("invalid value: ")
    {
        "a value the key cannot have"
    } else if message.starts_with("invalid type: ") {
        "a value of the wrong kind"
    } else {
        return message.to_owned();
    };
    // The last one: a value may hold the same words.
    match message.rsplit_once(", expected ") {
        Some((_, expected)) => format!("{said}, expected {expected}"),
        None => said.to_owned(),
    }
}

/// Reads the file's text. Any error is the whole file's: nothing of a file
/// that is wrong somewhere is used, so nothing of it is lost to a save.
pub(super) fn read(text: &str) -> Result<Read, Problem> {
    let file: File = toml::from_str(text).map_err(|error| Problem {
        line: error.span().map(|span| line_of(text, span.start)),
        what: without_values(error.message()),
    })?;
    let whole = |what: String| Problem { line: None, what };
    if file.version > VERSION {
        return Err(whole(format!(
            "it is version {} of the file, written by a newer Tabletist; this one reads \
             version {VERSION}",
            file.version
        )));
    }
    if file.version < FIRST_VERSION {
        return Err(whole(format!(
            "it says version {}, and the first version of this file is {FIRST_VERSION}",
            file.version
        )));
    }
    let mut seen = HashSet::new();
    let mut named = false;
    let mut connections = Vec::with_capacity(file.connections.len());
    for entry in file.connections {
        let connection = entry.connection(&mut named).map_err(&whole)?;
        if !seen.insert(connection.id.clone()) {
            return Err(whole(format!(
                "two connections have the id {:?} (the second is {:?}); remove the line from \
                 one of them and it gets a new one",
                connection.id.0, connection.name
            )));
        }
        connections.push(connection);
    }
    Ok(Read { connections, named })
}

impl Entry {
    /// The connection the entry describes. `named` is set when it had no
    /// id and was given one.
    fn connection(self, named: &mut bool) -> Result<SavedConnection, String> {
        let id = match self.id {
            Some(id) if !id.trim().is_empty() => ConnectionId(id),
            Some(_) => return Err(format!("connection {:?} has an empty id", self.name)),
            None => {
                *named = true;
                ConnectionId::new()
            }
        };
        let ssh = match self.ssh {
            Some(tunnel) => Some(tunnel.spec(&self.name)?),
            None => None,
        };
        let agents = match self.agents {
            Some(table) => table.agents(&self.name)?,
            None => Agents::default(),
        };
        Ok(SavedConnection {
            id,
            name: self.name,
            environment: self.environment,
            read_only: self.read_only,
            password: self.password,
            ssh_secret: self.ssh_secret,
            spec: ConnectSpec {
                driver: self.driver,
                host: self.host,
                port: self.port.unwrap_or(self.driver.default_port()),
                user: self.user,
                database: self.database,
                sqlite_path: self.sqlite_path,
                tls: self.tls,
                ca_file: self.ca_file,
                ssh,
            },
            agents,
        })
    }
}

impl AgentsTable {
    fn agents(self, connection: &str) -> Result<Agents, String> {
        let starts = Agents::default();
        // An entry by its place in the list: a value that does not read
        // is not repeated, as no other in this file is.
        let entries = |key: &str, written: Vec<String>| {
            written
                .iter()
                .enumerate()
                .map(|(at, entry)| {
                    entry.parse::<TableName>().map_err(|why| {
                        let place = at + 1;
                        format!("connection {connection:?}: agents' `{key}` entry {place} {why}")
                    })
                })
                .collect::<Result<Vec<_>, String>>()
        };
        let tables = match (self.hidden, self.only) {
            (Some(_), Some(_)) => {
                return Err(format!(
                    "connection {connection:?}: agents have `hidden` or `only`, not both"
                ));
            }
            (Some(hidden), None) if hidden.is_empty() => Tables::All,
            (Some(hidden), None) => Tables::AllExcept(entries("hidden", hidden)?),
            (None, Some(only)) => Tables::Only(entries("only", only)?),
            (None, None) => Tables::All,
        };
        let agents = Agents {
            access: self.access,
            rows: self.rows.unwrap_or(starts.rows),
            timeout_secs: self.timeout_secs.unwrap_or(starts.timeout_secs),
            tables,
        };
        if !(1..=MAX_AGENT_ROWS).contains(&agents.rows) {
            return Err(format!(
                "connection {connection:?}: agents' `rows` is from 1 to {MAX_AGENT_ROWS}"
            ));
        }
        if !(1..=MAX_AGENT_TIMEOUT_SECS).contains(&agents.timeout_secs) {
            return Err(format!(
                "connection {connection:?}: agents' `timeout_secs` is from 1 to \
                 {MAX_AGENT_TIMEOUT_SECS}"
            ));
        }
        Ok(agents)
    }
}

impl Tunnel {
    fn spec(self, connection: &str) -> Result<SshSpec, String> {
        let auth = match (self.auth, self.key) {
            (Auth::KeyFile, key) => SshAuth::KeyFile {
                // No key named takes `~/.ssh/config`'s IdentityFile.
                path: key.unwrap_or_default(),
            },
            (Auth::Password, None) => SshAuth::Password,
            (Auth::Agent, None) => SshAuth::Agent,
            (Auth::Password | Auth::Agent, Some(_)) => {
                return Err(format!(
                    "connection {connection:?}: an SSH `key` goes with `auth = \"key-file\"`"
                ));
            }
        };
        Ok(SshSpec {
            host: self.host,
            port: self.port,
            user: self.user,
            auth,
        })
    }
}

/// A table's lines as they are written: its keys aligned.
#[derive(Default)]
struct Lines(Vec<(&'static str, String)>);

impl Lines {
    fn text(&mut self, key: &'static str, value: &str) {
        self.0.push((key, toml_string(value)));
    }

    fn bare(&mut self, key: &'static str, value: impl std::fmt::Display) {
        self.0.push((key, value.to_string()));
    }

    /// A path, which the file can hold only as text.
    fn path(&mut self, key: &'static str, path: &Path) -> std::io::Result<()> {
        let Some(text) = path.to_str() else {
            return Err(std::io::Error::other(format!(
                "{} is not valid UTF-8, so it cannot be saved",
                path.display()
            )));
        };
        self.text(key, text);
        Ok(())
    }

    fn write(&self, out: &mut String) {
        let width = self.0.iter().map(|(key, _)| key.len()).max().unwrap_or(0);
        for (key, value) in &self.0 {
            out.push_str(&format!("{key:width$} = {value}\n"));
        }
    }
}

fn password(mode: PasswordMode) -> &'static str {
    match mode {
        PasswordMode::None => "none",
        PasswordMode::Keyring => "keyring",
        PasswordMode::Ask => "ask",
    }
}

fn driver(driver: Driver) -> &'static str {
    match driver {
        Driver::Postgres => "postgres",
        Driver::MySql => "mysql",
        Driver::Sqlite => "sqlite",
    }
}

fn tls(mode: TlsMode) -> &'static str {
    match mode {
        TlsMode::Disable => "disable",
        TlsMode::Prefer => "prefer",
        TlsMode::Require => "require",
        TlsMode::VerifyCa => "verify-ca",
        TlsMode::VerifyFull => "verify-full",
    }
}

/// The file's text for `connections`: the one form the app writes. A key
/// is left out where the reader gives the same value for a missing one,
/// except a server's own (host, port, user, database, TLS), which a person
/// looks for.
pub(super) fn write(connections: &[SavedConnection]) -> std::io::Result<String> {
    let hides = connections
        .iter()
        .any(|saved| saved.agents.tables.hides_anything());
    let agents = connections.iter().any(|saved| !saved.agents.is_default());
    let version = match (hides, agents) {
        (true, _) => VERSION,
        (false, true) => AGENTS_VERSION,
        (false, false) => FIRST_VERSION,
    };
    let mut out = format!("{HEADER}version = {version}\n");
    for connection in connections {
        let spec = &connection.spec;
        let server = match spec.driver {
            Driver::Postgres | Driver::MySql => true,
            Driver::Sqlite => false,
        };
        let mut lines = Lines::default();
        lines.text("id", &connection.id.0);
        lines.text("name", &connection.name);
        lines.text("environment", connection.environment.key());
        if let Some(read_only) = connection.read_only {
            lines.bare("read_only", read_only);
        }
        if server || connection.password != PasswordMode::None {
            lines.text("password", password(connection.password));
        }
        if spec.ssh.is_some() || connection.ssh_secret != PasswordMode::None {
            lines.text("ssh_secret", password(connection.ssh_secret));
        }
        lines.text("driver", driver(spec.driver));
        if let Some(path) = &spec.sqlite_path {
            lines.path("sqlite_path", path)?;
        }
        if server || !spec.host.is_empty() {
            lines.text("host", &spec.host);
        }
        if server || spec.port != spec.driver.default_port() {
            lines.bare("port", spec.port);
        }
        if server || !spec.user.is_empty() {
            lines.text("user", &spec.user);
        }
        if server || !spec.database.is_empty() {
            lines.text("database", &spec.database);
        }
        if server || spec.tls != TlsMode::default() {
            lines.text("tls", tls(spec.tls));
        }
        if let Some(path) = &spec.ca_file {
            lines.path("ca_file", path)?;
        }
        out.push_str("\n[[connections]]\n");
        lines.write(&mut out);
        if let Some(ssh) = &spec.ssh {
            tunnel(ssh, &mut out)?;
        }
        if !connection.agents.is_default() {
            let mut lines = Lines::default();
            lines.text("access", connection.agents.access.key());
            lines.bare("rows", connection.agents.rows);
            lines.bare("timeout_secs", connection.agents.timeout_secs);
            match &connection.agents.tables {
                Tables::All => {}
                Tables::AllExcept(names) if names.is_empty() => {}
                Tables::AllExcept(names) => lines.bare("hidden", entries(names)),
                Tables::Only(names) => lines.bare("only", entries(names)),
            }
            out.push_str("\n[connections.agents]\n");
            lines.write(&mut out);
        }
    }
    Ok(out)
}

/// Tables as a list of TOML strings, each written as the person writes it.
fn entries(names: &[TableName]) -> String {
    let written: Vec<String> = names
        .iter()
        .map(|name| toml_string(&name.to_string()))
        .collect();
    format!("[{}]", written.join(", "))
}

/// A connection's SSH tunnel, as a table of its own.
fn tunnel(ssh: &SshSpec, out: &mut String) -> std::io::Result<()> {
    let mut lines = Lines::default();
    lines.text("host", &ssh.host);
    if let Some(port) = ssh.port {
        lines.bare("port", port);
    }
    if !ssh.user.is_empty() {
        lines.text("user", &ssh.user);
    }
    match &ssh.auth {
        SshAuth::Password => lines.text("auth", "password"),
        SshAuth::Agent => lines.text("auth", "agent"),
        SshAuth::KeyFile { path } => {
            lines.text("auth", "key-file");
            if !path.as_os_str().is_empty() {
                lines.path("key", path)?;
            }
        }
    }
    out.push_str("\n[connections.ssh]\n");
    lines.write(out);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn saved(id: &str, name: &str, spec: ConnectSpec) -> SavedConnection {
        SavedConnection {
            id: ConnectionId(id.into()),
            name: name.into(),
            environment: Environment::None,
            read_only: None,
            password: PasswordMode::None,
            ssh_secret: PasswordMode::None,
            spec,
            agents: Default::default(),
        }
    }

    fn server(driver: Driver, host: &str, database: &str) -> ConnectSpec {
        ConnectSpec {
            driver,
            host: host.into(),
            port: driver.default_port(),
            user: "app".into(),
            database: database.into(),
            ..ConnectSpec::default()
        }
    }

    /// The three kinds of connection, with everything a file can say of
    /// the first.
    fn bookshop() -> Vec<SavedConnection> {
        let production = SavedConnection {
            environment: Environment::Production,
            read_only: Some(true),
            password: PasswordMode::Keyring,
            ssh_secret: PasswordMode::Ask,
            spec: ConnectSpec {
                tls: TlsMode::VerifyFull,
                ca_file: Some("/etc/ssl/bookshop-ca.pem".into()),
                ssh: Some(SshSpec {
                    host: "bastion.example.com".into(),
                    port: Some(2222),
                    user: "deploy".into(),
                    auth: SshAuth::KeyFile {
                        path: "/home/ada/.ssh/id_ed25519".into(),
                    },
                }),
                ..server(Driver::Postgres, "db.example.com", "bookshop_production")
            },
            ..saved("prod", "Bookshop production", ConnectSpec::default())
        };
        let preview = SavedConnection {
            environment: Environment::Staging,
            password: PasswordMode::Ask,
            ..saved(
                "preview",
                "Bookshop preview",
                server(Driver::MySql, "preview.example.com", "bookshop_preview"),
            )
        };
        let file = SavedConnection {
            environment: Environment::Local,
            ..saved(
                "file",
                "Bookshop on disk",
                ConnectSpec::sqlite("/home/ada/bookshop.db"),
            )
        };
        vec![production, preview, file]
    }

    #[test]
    fn the_file_is_written_in_one_form() {
        let expected = format!(
            "{HEADER}{}",
            r#"version = 3

[[connections]]
id          = "prod"
name        = "Bookshop production"
environment = "production"
read_only   = true
password    = "keyring"
ssh_secret  = "ask"
driver      = "postgres"
host        = "db.example.com"
port        = 5432
user        = "app"
database    = "bookshop_production"
tls         = "verify-full"
ca_file     = "/etc/ssl/bookshop-ca.pem"

[connections.ssh]
host = "bastion.example.com"
port = 2222
user = "deploy"
auth = "key-file"
key  = "/home/ada/.ssh/id_ed25519"

[[connections]]
id          = "preview"
name        = "Bookshop preview"
environment = "staging"
password    = "ask"
driver      = "mysql"
host        = "preview.example.com"
port        = 3306
user        = "app"
database    = "bookshop_preview"
tls         = "prefer"

[[connections]]
id          = "file"
name        = "Bookshop on disk"
environment = "local"
driver      = "sqlite"
sqlite_path = "/home/ada/bookshop.db"
"#
        );
        assert_eq!(write(&bookshop()).unwrap(), expected);
        // No connection is a file all the same, with its version.
        assert_eq!(write(&[]).unwrap(), format!("{HEADER}version = 3\n"));
    }

    #[test]
    fn what_is_written_is_read_back_as_it_was() {
        let mut all = bookshop();
        let tunnel = |auth: SshAuth| SshSpec {
            host: "bastion".into(),
            port: None,
            user: String::new(),
            auth,
        };
        // What the writer leaves out where it can: each must come back.
        let odd = [
            // A file with a server's keys set, each away from what a
            // missing one reads as.
            ConnectSpec {
                host: "elsewhere".into(),
                port: 7,
                user: "someone".into(),
                database: "something".into(),
                tls: TlsMode::Disable,
                ..ConnectSpec::sqlite("/odd.db")
            },
            // A server with nothing filled in.
            ConnectSpec {
                host: String::new(),
                port: 0,
                user: String::new(),
                ..server(Driver::Postgres, "", "")
            },
            ConnectSpec {
                ssh: Some(tunnel(SshAuth::Password)),
                ..server(Driver::MySql, "db", "shop")
            },
            ConnectSpec {
                ssh: Some(tunnel(SshAuth::Agent)),
                ..server(Driver::Postgres, "db", "shop")
            },
            // A key file left to `~/.ssh/config`.
            ConnectSpec {
                ssh: Some(tunnel(SshAuth::KeyFile {
                    path: PathBuf::new(),
                })),
                ..server(Driver::Postgres, "db", "shop")
            },
            // A path as Windows writes one.
            ConnectSpec::sqlite(r"C:\Users\ada\My Databases\bookshop.db"),
        ];
        for (index, spec) in odd.into_iter().enumerate() {
            all.push(saved(&format!("odd-{index}"), "Odd", spec));
        }
        // Text that needs care in a TOML string.
        for (index, name) in [
            "Zoë's \"shop\"",
            r"back\slash",
            "two\nlines\tand a tab",
            "a bell \u{7} and a null \u{0}",
            "# not a comment",
            "",
            "  spaces  ",
            "[[connections]]",
        ]
        .into_iter()
        .enumerate()
        {
            all.push(saved(
                &format!("name-{index}"),
                name,
                ConnectSpec::sqlite("/x.db"),
            ));
        }
        // Each choice a connection can carry beside its spec.
        for (index, (read_only, password, ssh_secret)) in [
            (Some(false), PasswordMode::None, PasswordMode::Keyring),
            (Some(true), PasswordMode::Ask, PasswordMode::None),
            (None, PasswordMode::Keyring, PasswordMode::Ask),
        ]
        .into_iter()
        .enumerate()
        {
            for environment in Environment::ALL {
                all.push(SavedConnection {
                    environment,
                    read_only,
                    password,
                    ssh_secret,
                    ..saved(
                        &format!("choice-{index}-{}", environment.key()),
                        "Choices",
                        ConnectSpec::sqlite("/x.db"),
                    )
                });
            }
        }
        let text = write(&all).unwrap();
        let read = read(&text).unwrap_or_else(|problem| panic!("{problem}\n{text}"));
        assert!(!read.named);
        assert_eq!(read.connections.len(), all.len());
        for (read, written) in read.connections.iter().zip(&all) {
            assert_eq!(read, written, "{text}");
        }
        // And written again it is the same text.
        assert_eq!(write(&read.connections).unwrap(), text);
    }

    /// What a person writes: the keys that matter, and no more.
    #[test]
    fn a_key_that_is_left_out_reads_as_its_default() {
        let text = r#"
version = 3

[[connections]]
id = "one"
name = "By hand"
driver = "postgres"
host = "db.example.com"
database = "bookshop"

[[connections]]
id = "two"
name = "Also by hand"
driver = "mysql"

[connections.ssh]
host = "bastion"
auth = "key-file"
"#;
        let read = read(text).unwrap();
        let [one, two] = read.connections.as_slice() else {
            panic!("{read:?}");
        };
        assert_eq!(
            *one,
            saved(
                "one",
                "By hand",
                ConnectSpec {
                    user: String::new(),
                    ..server(Driver::Postgres, "db.example.com", "bookshop")
                }
            )
        );
        assert_eq!(two.spec.port, 3306);
        assert_eq!(two.spec.host, "");
        assert_eq!(
            two.spec.ssh,
            Some(SshSpec {
                host: "bastion".into(),
                port: None,
                user: String::new(),
                auth: SshAuth::KeyFile {
                    path: PathBuf::new()
                },
            })
        );
        // A file with its version and nothing else has no connections.
        assert_eq!(read_ok("version = 3\n"), Vec::new());
    }

    fn read_ok(text: &str) -> Vec<SavedConnection> {
        read(text).unwrap().connections
    }

    /// The problem with a file that is `version = 3` and then `rest`.
    fn problem(rest: &str) -> Problem {
        read(&format!("version = 3\n{rest}")).unwrap_err()
    }

    #[test]
    fn a_key_the_file_does_not_have_is_an_error_at_its_line() {
        let entry = "\n[[connections]]\nid = \"a\"\nname = \"A\"\ndriver = \"sqlite\"\n";
        for (rest, line, what) in [
            // A key spelled wrong must not pass for one left out.
            (
                format!("{entry}read_onyl = true\n"),
                7,
                "unknown field `read_onyl`",
            ),
            (
                format!("{entry}\n[connections.agents]\naccess = \"read\"\nrowz = 9\n"),
                10,
                "unknown field `rowz`",
            ),
            (
                format!(
                    "{entry}\n[connections.ssh]\nhost = \"b\"\nauth = \"agent\"\nkeyfile = \"k\"\n"
                ),
                11,
                "unknown field `keyfile`",
            ),
            (
                "\nlast_used = 1\n".to_owned(),
                3,
                "unknown field `last_used`",
            ),
            // A value that is none of the key's.
            (
                entry.replace("\"sqlite\"", "\"oracle\""),
                6,
                "a value the key cannot have, expected one of `postgres`",
            ),
            (
                format!("{entry}environment = \"test\"\n"),
                7,
                "a value the key cannot have, expected one of `local`",
            ),
            (
                format!("{entry}tls = \"on\"\n"),
                7,
                "a value the key cannot have, expected one of `disable`",
            ),
            (
                format!("{entry}password = \"hunter2\"\n"),
                7,
                "a value the key cannot have, expected one of `none`, `keyring`, `ask`",
            ),
            (
                format!("{entry}port = 70000\n"),
                7,
                "a value the key cannot have, expected u16",
            ),
            (
                format!("{entry}read_only = \"yes\"\n"),
                7,
                "a value of the wrong kind, expected a boolean",
            ),
            // A key that must be there.
            (
                "\n[[connections]]\nid = \"a\"\ndriver = \"sqlite\"\n".to_owned(),
                3,
                "missing field `name`",
            ),
            (
                "\n[[connections]]\nid = \"a\"\nname = \"A\"\n".to_owned(),
                3,
                "missing field `driver`",
            ),
            // Text that is no TOML.
            ("\n[[connections]\n".to_owned(), 3, ""),
        ] {
            let problem = problem(&rest);
            assert_eq!(problem.line, Some(line), "{rest}: {problem}");
            assert!(problem.what.contains(what), "{rest}: {problem}");
            assert!(
                problem.to_string().starts_with(&format!("line {line}: ")),
                "{problem}"
            );
        }
    }

    /// A person may well write a password where the file asks how the
    /// password is kept. What is wrong with the file is logged and shown,
    /// so it says where and never what was written.
    #[test]
    fn a_value_that_is_wrong_is_never_repeated() {
        let entry = "\n[[connections]]\nid = \"a\"\nname = \"A\"\ndriver = \"postgres\"\n";
        for line in [
            "password = \"hunter2\"",
            "ssh_secret = \"hunter2\"",
            "password = hunter2",
            "password = 'hunter2'",
            "port = \"hunter2\"",
            "read_only = \"hunter2\"",
            "tls = \"hunter2\"",
            "environment = \"hunter2\"",
            "password = [\"hunter2\"]",
            "password = { hunter2 = true }",
        ] {
            let problem = problem(&format!("{entry}{line}\n"));
            let said = problem.to_string();
            assert!(!said.contains("hunter2"), "{line}: {said}");
            assert_eq!(problem.line, Some(7), "{line}: {said}");
        }
        // And as a driver's name, which is on another line.
        let said = problem(&entry.replace("postgres", "hunter2")).to_string();
        assert!(!said.contains("hunter2"), "{said}");
    }

    #[test]
    fn the_file_says_which_version_it_is() {
        let said = |text: &str| read(text).unwrap_err().to_string();
        assert!(said("").contains("missing field `version`"), "{}", said(""));
        assert_eq!(
            said("version = 6\n"),
            "it is version 6 of the file, written by a newer Tabletist; this one reads version 5"
        );
        assert_eq!(
            said("version = 2\n"),
            "it says version 2, and the first version of this file is 3"
        );
    }

    fn with_agents(access: AgentAccess, rows: u32, timeout_secs: u32) -> Vec<SavedConnection> {
        let mut connections = bookshop();
        connections[2].agents = Agents {
            access,
            rows,
            timeout_secs,
            ..Agents::default()
        };
        connections
    }

    #[test]
    fn what_agents_may_do_is_a_table_of_its_own_in_version_four() {
        let connections = with_agents(AgentAccess::Read, 1_000, 30);
        let text = write(&connections).unwrap();
        assert!(text.contains("\nversion = 4\n"), "{text}");
        assert!(
            text.ends_with(
                r#"sqlite_path = "/home/ada/bookshop.db"

[connections.agents]
access       = "read"
rows         = 1000
timeout_secs = 30
"#
            ),
            "{text}"
        );
        assert_eq!(read(&text).unwrap().connections, connections);
        // Limits set while access is Off are kept, and so is the table.
        let off = with_agents(AgentAccess::Off, 100, 10);
        let text = write(&off).unwrap();
        assert!(text.contains("access       = \"off\""), "{text}");
        assert_eq!(read(&text).unwrap().connections, off);
    }

    #[test]
    fn a_file_without_agents_is_written_and_read_as_version_three() {
        let text = write(&bookshop()).unwrap();
        assert!(
            text.contains("\nversion = 3\n") && !text.contains("agents"),
            "{text}"
        );
        let read = read(&text).unwrap();
        assert!(
            read.connections
                .iter()
                .all(|saved| saved.agents == Agents::default())
        );
        // And one of version 4 without them is read the same.
        let four = text.replace("\nversion = 3\n", "\nversion = 4\n");
        assert_eq!(super::read(&four).unwrap().connections, bookshop());
    }

    #[test]
    fn what_agents_may_do_takes_a_connections_start_for_a_key_left_out() {
        let text = format!(
            "{}\n[connections.agents]\naccess = \"read\"\n",
            write(&bookshop()).unwrap()
        );
        let agents = read(&text).unwrap().connections[2].agents.clone();
        assert_eq!(
            agents,
            Agents {
                access: AgentAccess::Read,
                ..Agents::default()
            }
        );
        // A table that says nothing is Off.
        let empty = format!("{}\n[connections.agents]\n", write(&bookshop()).unwrap());
        assert_eq!(
            read(&empty).unwrap().connections[2].agents,
            Agents::default()
        );
    }

    #[test]
    fn what_agents_may_do_is_refused_outside_its_limits() {
        let said = |table: &str| {
            let text = format!(
                "{}\n[connections.agents]\n{table}\n",
                write(&bookshop()).unwrap()
            );
            read(&text).unwrap_err().to_string()
        };
        let rows = "connection \"Bookshop on disk\": agents' `rows` is from 1 to 5000";
        assert_eq!(said("rows = 0"), rows);
        assert_eq!(said("rows = 5001"), rows);
        assert_eq!(
            said("timeout_secs = 61"),
            "connection \"Bookshop on disk\": agents' `timeout_secs` is from 1 to 60"
        );
        // An access there is not, and a key there is not, without the value.
        let access = said("access = \"read_and_write\"");
        assert!(access.contains("a value the key cannot have"), "{access}");
        assert!(!access.contains("read_and_write"), "{access}");
        let key = said("rowz = 9");
        assert!(key.contains("unknown field `rowz`"), "{key}");
        // The tables: one list or the other, and each entry a table.
        assert_eq!(
            said("hidden = [\"customers\"]\nonly = [\"books\"]"),
            "connection \"Bookshop on disk\": agents have `hidden` or `only`, not both"
        );
        let entry = said("hidden = [\"customers\", \"a.b.c\"]");
        assert_eq!(
            entry,
            "connection \"Bookshop on disk\": agents' `hidden` entry 2 has more than one dot: it \
             is a table, or a schema and a table"
        );
        let secret = said("hidden = [\"hunter2\\\"x\"]");
        assert!(!secret.contains("hunter2"), "{secret}");
    }

    #[test]
    fn the_tables_agents_may_not_read_make_the_file_version_five() {
        let mut connections = with_agents(AgentAccess::Read, 500, 10);
        connections[2].agents.tables = Tables::AllExcept(vec![
            "customers".parse().unwrap(),
            "\"odd.schema\".orders".parse().unwrap(),
        ]);
        let text = write(&connections).unwrap();
        assert!(text.contains("\nversion = 5\n"), "{text}");
        assert!(
            text.ends_with(
                r#"timeout_secs = 10
hidden       = ["customers", "\"odd.schema\".orders"]
"#
            ),
            "{text}"
        );
        assert_eq!(read(&text).unwrap().connections, connections);
        // Only a list: the rest is hidden, and an empty one hides it all.
        connections[2].agents.tables = Tables::Only(Vec::new());
        let text = write(&connections).unwrap();
        assert!(text.contains("only         = []"), "{text}");
        assert_eq!(read(&text).unwrap().connections, connections);
        // An empty "hidden" hides nothing: the file is version 4.
        let four = text.replace("only         = []", "hidden = []");
        connections[2].agents.tables = Tables::All;
        assert_eq!(read(&four).unwrap().connections, connections);
        assert!(write(&connections).unwrap().contains("\nversion = 4\n"));
    }

    #[test]
    fn every_connection_has_an_id_of_its_own() {
        let entry = |id: &str, name: &str| {
            format!("\n[[connections]]\n{id}name = \"{name}\"\ndriver = \"sqlite\"\n")
        };
        // Two with one id, as a block copied by hand has.
        let copied = problem(&format!(
            "{}{}",
            entry("id = \"a\"\n", "First"),
            entry("id = \"a\"\n", "Second")
        ));
        assert_eq!(
            copied.to_string(),
            "two connections have the id \"a\" (the second is \"Second\"); remove the line \
             from one of them and it gets a new one"
        );
        let empty = problem(&entry("id = \" \"\n", "Blank"));
        assert_eq!(empty.to_string(), "connection \"Blank\" has an empty id");
        // None at all: each gets its own, and the file is to be written.
        let text = format!("version = 3\n{}{}", entry("", "First"), entry("", "Second"));
        let read = read(&text).unwrap();
        assert!(read.named);
        let [first, second] = read.connections.as_slice() else {
            panic!("{read:?}");
        };
        assert_ne!(first.id, second.id);
        assert_eq!(first.id.0.len(), 36, "{:?}", first.id);
    }

    #[test]
    fn an_ssh_key_goes_with_a_key_file_and_nothing_else() {
        let tunnel = |auth: &str| {
            format!(
                "\n[[connections]]\nid = \"a\"\nname = \"Tunnelled\"\ndriver = \"postgres\"\n\n\
                 [connections.ssh]\nhost = \"bastion\"\nauth = \"{auth}\"\nkey = \"/id\"\n"
            )
        };
        for auth in ["password", "agent"] {
            assert_eq!(
                problem(&tunnel(auth)).to_string(),
                "connection \"Tunnelled\": an SSH `key` goes with `auth = \"key-file\"`",
                "{auth}"
            );
        }
        let read = read(&format!("version = 3\n{}", tunnel("key-file"))).unwrap();
        let ssh = read.connections[0].spec.ssh.clone().unwrap();
        assert_eq!(ssh.auth, SshAuth::KeyFile { path: "/id".into() });
    }

    /// The example on the website's page of files is a file the app reads,
    /// and its first connection is one the app writes just so.
    #[test]
    fn the_websites_example_is_read_and_written_as_it_shows() {
        // Windows checks the page out with CRLF line ends.
        let page =
            include_str!("../../docs/_reference/settings-and-files.md").replace("\r\n", "\n");
        let (_, section) = page.split_once("## The connections file").unwrap();
        let (_, block) = section.split_once("```toml\n").unwrap();
        let (example, _) = block.split_once("```").unwrap();
        let read = read(example).unwrap();
        // The second connection is shown without an id.
        assert!(read.named);
        let [production, on_disk] = read.connections.as_slice() else {
            panic!("{read:?}");
        };
        assert_eq!(on_disk.name, "Bookshop on disk");
        let written = write(std::slice::from_ref(production)).unwrap();
        let (shown, _) = example.split_once("\n[[connections]]\nname").unwrap();
        assert_eq!(written, format!("{HEADER}{shown}"));
    }

    /// TOML is text, and so is JSON, which could not hold such a path
    /// either: the save fails and says which path.
    #[cfg(unix)]
    #[test]
    fn a_path_that_is_no_text_is_not_written() {
        use std::os::unix::ffi::OsStringExt;
        let path = PathBuf::from(std::ffi::OsString::from_vec(b"/caf\xC3\x28.db".to_vec()));
        let error = write(&[saved("a", "A", ConnectSpec::sqlite(path))]).unwrap_err();
        assert!(
            error
                .to_string()
                .ends_with("is not valid UTF-8, so it cannot be saved"),
            "{error}"
        );
    }
}

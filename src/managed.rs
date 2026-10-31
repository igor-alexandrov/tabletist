//! An administrator's policy: `managed.toml`, at a fixed path that only an
//! administrator can write to on each system (`AppDirs::managed`). It is a
//! floor under the person's own settings. Its rules lower what agents may
//! do on the connections they match and make connections read-only.
//! Nothing in the person's files lifts what it sets, and nothing of it is
//! written into them: removing a rule gives back what the person had.
//!
//! A coding agent with a shell runs as the person and can edit every file
//! Tabletist keeps for them, `agents_shown.json` included. It cannot edit
//! this one unless it can act as an administrator.
//! `docs/superpowers/specs/2026-10-15-managed-policy-design.md` is the
//! design. Its runs 1a and 1b: the file, the limits, and a rule's tables
//! and columns, which the agent checks enforce beside the person's own
//! ([`Managed::sources`]).

use std::path::{Path, PathBuf};

use serde::Deserialize;
use serde::de::IgnoredAny;
use tabletist_db::agent::{ColumnName, Source, TableName, Tables};
use tabletist_db::{ConnectSpec, Driver};

use crate::connections::file::{Problem, line_of, without_values};
use crate::connections::{AgentAccess, Agents, MAX_AGENT_ROWS, MAX_AGENT_TIMEOUT_SECS};

/// The version of the file this Tabletist reads.
const VERSION: u32 = 1;

/// How long the log may be asked to keep a call, in days: the settings'
/// own range.
const LOG_DAYS: std::ops::RangeInclusive<u32> =
    crate::settings::Settings::MIN_MCP_LOG_DAYS..=crate::settings::Settings::MAX_MCP_LOG_DAYS;

/// What the app says when agents are turned on and the policy forbids them.
pub const AGENTS_NOT_ALLOWED: &str =
    "Your administrator's policy does not let agents use Tabletist on this computer.";

/// The policy, as the file says it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Policy {
    /// Whether the agent server may be turned on at all.
    pub allowed: bool,
    /// Whether a connection no rule matches is Off for agents.
    pub unmatched_off: bool,
    /// The fewest days the log of agents' calls keeps a call.
    pub log_days_min: Option<u32>,
    pub rules: Vec<Rule>,
}

/// One rule: the connections it matches, and what it holds them to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rule {
    /// The administrator's name for it, for the person to be told why.
    pub name: String,
    pub matches: Match,
    /// Whether the person's own sessions on it are read-only.
    pub read_only: bool,
    pub access_max: Option<AgentAccess>,
    pub rows_max: Option<u32>,
    pub timeout_max_secs: Option<u32>,
    /// Tables agents may not read ("All except").
    pub hidden: Vec<TableName>,
    /// The only tables agents may read ("Only"). An empty list is no table.
    pub visible_only: Option<Vec<TableName>>,
    pub masked: Vec<ColumnName>,
}

impl Rule {
    /// What the rule hides and masks, as the agent checks take it: its
    /// `visible_only` as an "Only" list (empty, no table), else its
    /// `hidden` as an "All except" one.
    fn source(&self) -> Source {
        let tables = match &self.visible_only {
            Some(only) => Tables::Only(only.clone()),
            None if self.hidden.is_empty() => Tables::All,
            None => Tables::AllExcept(self.hidden.clone()),
        };
        Source {
            tables,
            masked: self.masked.clone(),
        }
    }
}

/// What a rule matches: each key that is there must match, and `*` in its
/// text stands for any run of characters.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Match {
    pub driver: Option<String>,
    pub host: Option<String>,
    pub port: Option<u16>,
    pub database: Option<String>,
    pub path: Option<String>,
}

impl Match {
    /// Whether `spec` is a connection the rule is about. `host` is compared
    /// without regard to case, the rest as written. Through an SSH tunnel
    /// the host is the database's, as the tunnel reaches it. `path` is the
    /// SQLite file's, with `~` read as `home` and made absolute; links are
    /// not followed. A key the driver has no use for does not match.
    pub fn matches(&self, spec: &ConnectSpec, home: Option<&Path>) -> bool {
        let server = match spec.driver {
            Driver::Postgres | Driver::MySql => true,
            Driver::Sqlite | Driver::Files => false,
        };
        if let Some(driver) = &self.driver
            && !glob(driver, driver_key(spec.driver))
        {
            return false;
        }
        let names_a_server = self.host.is_some() || self.port.is_some() || self.database.is_some();
        if names_a_server && !server || self.path.is_some() && server {
            return false;
        }
        if let Some(host) = &self.host
            && !glob(&host.to_lowercase(), &spec.host.to_lowercase())
        {
            return false;
        }
        if let Some(port) = self.port
            && port != spec.port
        {
            return false;
        }
        if let Some(database) = &self.database
            && !glob(database, &spec.database)
        {
            return false;
        }
        if let Some(path) = &self.path {
            // A Files connection's path is its folder's.
            let named = match spec.driver {
                Driver::Sqlite => spec.sqlite_path.as_deref(),
                Driver::Files => spec
                    .files
                    .as_ref()
                    .and_then(tabletist_db::files::Source::folder),
                Driver::Postgres | Driver::MySql => None,
            };
            let Some(file) = named.and_then(|file| absolute(file, home)) else {
                return false;
            };
            if !glob(path, &file.to_string_lossy()) {
                return false;
            }
        }
        true
    }

    fn is_empty(&self) -> bool {
        *self == Self::default()
    }
}

/// The driver's word in the file, as in `connections.toml`.
fn driver_key(driver: Driver) -> &'static str {
    match driver {
        Driver::Postgres => "postgres",
        Driver::MySql => "mysql",
        Driver::Sqlite => "sqlite",
        Driver::Files => "files",
    }
}

/// `file`, with a leading `~` read as `home`, made absolute without asking
/// the disk.
fn absolute(file: &Path, home: Option<&Path>) -> Option<PathBuf> {
    let file = match (file.strip_prefix("~"), home) {
        (Ok(rest), Some(home)) => home.join(rest),
        (Ok(_), None) => return None,
        (Err(_), _) => file.to_path_buf(),
    };
    std::path::absolute(file).ok()
}

/// Whether `text` matches `pattern`, in which `*` stands for any run of
/// characters, none included. Nothing else is special.
fn glob(pattern: &str, text: &str) -> bool {
    let pattern: Vec<char> = pattern.chars().collect();
    let text: Vec<char> = text.chars().collect();
    let (mut p, mut t) = (0, 0);
    // The last star, and where in the text it was tried up to.
    let mut star: Option<(usize, usize)> = None;
    while t < text.len() {
        if p < pattern.len() && pattern[p] == '*' {
            star = Some((p, t));
            p += 1;
        } else if p < pattern.len() && pattern[p] == text[t] {
            p += 1;
            t += 1;
        } else if let Some((at, from)) = star {
            // The star takes one more character.
            star = Some((at, from + 1));
            p = at + 1;
            t = from + 1;
        } else {
            return false;
        }
    }
    pattern[p..].iter().all(|c| *c == '*')
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct File {
    version: u32,
    #[serde(default)]
    agents: AgentsTable,
    #[serde(default)]
    rules: Vec<RuleTable>,
    /// Kept for later versions (managed connections, a service): read so
    /// that a file with them is refused in words of its own.
    #[serde(default)]
    connections: Option<IgnoredAny>,
    #[serde(default)]
    service: Option<IgnoredAny>,
}

#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct AgentsTable {
    allowed: Option<bool>,
    unmatched: Option<Unmatched>,
    log_days_min: Option<u32>,
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum Unmatched {
    Off,
    AsSet,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RuleTable {
    name: String,
    #[serde(rename = "match")]
    matches: MatchTable,
    read_only: Option<bool>,
    access_max: Option<AgentAccess>,
    rows_max: Option<u32>,
    timeout_max_secs: Option<u32>,
    hidden: Option<Vec<String>>,
    visible_only: Option<Vec<String>>,
    masked: Option<Vec<String>>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct MatchTable {
    driver: Option<String>,
    host: Option<String>,
    port: Option<u16>,
    database: Option<String>,
    path: Option<String>,
}

/// Reads the file's text. Any problem is the whole file's: a security
/// floor with a line skipped is a limit quietly gone. What it says of a
/// problem never repeats a value from the file.
pub(crate) fn read(text: &str) -> Result<Policy, Problem> {
    let file: File = toml::from_str(text).map_err(|error| Problem {
        line: error.span().map(|span| line_of(text, span.start)),
        what: without_values(error.message()),
    })?;
    let whole = |what: String| Problem { line: None, what };
    if file.version > VERSION {
        return Err(whole(format!(
            "it is version {} of the file, for a newer Tabletist; this one reads version \
             {VERSION}",
            file.version
        )));
    }
    if file.version < VERSION {
        return Err(whole(format!(
            "it says version {}, and the first version of this file is {VERSION}",
            file.version
        )));
    }
    if file.connections.is_some() {
        return Err(whole(
            "it has connections, which a newer Tabletist reads from this file".into(),
        ));
    }
    if file.service.is_some() {
        return Err(whole(
            "it has a service, which a newer Tabletist reads from this file".into(),
        ));
    }
    if let Some(days) = file.agents.log_days_min
        && !LOG_DAYS.contains(&days)
    {
        return Err(whole(format!(
            "agents.log_days_min is out of its range, {} to {}",
            LOG_DAYS.start(),
            LOG_DAYS.end()
        )));
    }
    let rules = file
        .rules
        .into_iter()
        .enumerate()
        .map(|(at, rule)| {
            rule.rule()
                .map_err(|what| whole(format!("rule {}: {what}", at + 1)))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let unmatched_off = match file.agents.unmatched {
        Some(Unmatched::Off) => true,
        Some(Unmatched::AsSet) => false,
        None => !rules.is_empty(),
    };
    Ok(Policy {
        allowed: file.agents.allowed.unwrap_or(true),
        unmatched_off,
        log_days_min: file.agents.log_days_min,
        rules,
    })
}

impl RuleTable {
    /// The rule the table describes, or what is wrong with it, in words
    /// that hold none of its values.
    fn rule(self) -> Result<Rule, String> {
        if self.name.trim().is_empty() {
            return Err("its name is empty".into());
        }
        let matches = self.matches.matches()?;
        if self.read_only == Some(false) {
            return Err(
                "read_only = false: a policy can make a connection read-only, never writable"
                    .into(),
            );
        }
        let within = |key: &str, value: Option<u32>, most: u32| match value {
            Some(value) if !(1..=most).contains(&value) => {
                Err(format!("{key} is out of its range, 1 to {most}"))
            }
            _ => Ok(value),
        };
        let rows_max = within("rows_max", self.rows_max, MAX_AGENT_ROWS)?;
        let timeout_max_secs = within(
            "timeout_max_secs",
            self.timeout_max_secs,
            MAX_AGENT_TIMEOUT_SECS,
        )?;
        if self.hidden.is_some() && self.visible_only.is_some() {
            return Err("it has both hidden and visible_only; a rule has one of them".into());
        }
        let tables = |key: &str, entries: Vec<String>| {
            entries
                .iter()
                .enumerate()
                .map(|(at, entry)| {
                    entry
                        .parse::<TableName>()
                        .map_err(|why| format!("{key} entry {}: {why}", at + 1))
                })
                .collect::<Result<Vec<_>, _>>()
        };
        let hidden = tables("hidden", self.hidden.unwrap_or_default())?;
        let visible_only = self
            .visible_only
            .map(|entries| tables("visible_only", entries))
            .transpose()?;
        let masked = self
            .masked
            .unwrap_or_default()
            .iter()
            .enumerate()
            .map(|(at, entry)| {
                entry
                    .parse::<ColumnName>()
                    .map_err(|why| format!("masked entry {}: {why}", at + 1))
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Rule {
            name: self.name,
            matches,
            read_only: self.read_only.unwrap_or(false),
            access_max: self.access_max,
            rows_max,
            timeout_max_secs,
            hidden,
            visible_only,
            masked,
        })
    }
}

impl MatchTable {
    fn matches(self) -> Result<Match, String> {
        let matches = Match {
            driver: self.driver,
            host: self.host,
            port: self.port,
            database: self.database,
            path: self.path,
        };
        if matches.is_empty() {
            return Err(
                "its match is empty; a rule for every connection says match = { driver = \"*\" }"
                    .into(),
            );
        }
        if let Some(driver) = &matches.driver
            && !driver.contains('*')
            && ![Driver::Postgres, Driver::MySql, Driver::Sqlite]
                .into_iter()
                .any(|known| driver_key(known) == driver)
        {
            return Err(
                "its driver is none that Tabletist knows: postgres, mysql or sqlite".into(),
            );
        }
        Ok(matches)
    }
}

/// The policy on this computer, as the app found it when it started.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum Managed {
    /// There is no policy file: nothing is lowered.
    #[default]
    None,
    Read {
        path: PathBuf,
        policy: Policy,
        /// Whether only an administrator can change the file. A file that
        /// is not locked applies all the same: a floor can only restrict.
        locked: bool,
    },
    /// The file is there and could not be read: agents are off and every
    /// connection is read-only until it is mended. `problem` says where
    /// and what, without a value from the file.
    Unreadable { path: PathBuf, problem: String },
}

impl Managed {
    /// Reads the policy at `path`, where the system has one.
    pub fn load(path: Option<&Path>) -> Self {
        let Some(path) = path else {
            return Self::None;
        };
        let text = match std::fs::read_to_string(path) {
            Ok(text) => text,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Self::None,
            Err(error) => {
                let problem = error.kind().to_string();
                log::error!(
                    "the administrator's policy {} could not be read: {problem}",
                    path.display()
                );
                return Self::Unreadable {
                    path: path.to_path_buf(),
                    problem,
                };
            }
        };
        match read(&text) {
            Ok(policy) => {
                let locked = locked(path);
                log::info!(
                    "the administrator's policy {} has {} rules, and {}",
                    path.display(),
                    policy.rules.len(),
                    match locked {
                        true => "only an administrator can change it",
                        false => "it can be changed without administrator rights",
                    }
                );
                Self::Read {
                    path: path.to_path_buf(),
                    policy,
                    locked,
                }
            }
            Err(problem) => {
                log::error!(
                    "the administrator's policy {} could not be read: {problem}",
                    path.display()
                );
                Self::Unreadable {
                    path: path.to_path_buf(),
                    problem: problem.to_string(),
                }
            }
        }
    }

    /// What the person is told of a policy that could not be read.
    pub fn notice(&self) -> Option<String> {
        match self {
            Self::None | Self::Read { .. } => None,
            Self::Unreadable { path, problem } => Some(format!(
                "Your administrator's policy could not be read ({}, {problem}). Until it is \
                 mended, agents are off and every connection is read-only.",
                path.display()
            )),
        }
    }

    /// Whether the agent server may be on.
    pub fn server_allowed(&self) -> bool {
        match self {
            Self::None => true,
            Self::Read { policy, .. } => policy.allowed,
            Self::Unreadable { .. } => false,
        }
    }

    /// Whether the person's own sessions on `spec` are read-only whatever
    /// its box says.
    pub fn forces_read_only(&self, spec: &ConnectSpec, home: Option<&Path>) -> bool {
        match self {
            Self::None => false,
            Self::Read { policy, .. } => policy
                .rules
                .iter()
                .any(|rule| rule.read_only && rule.matches.matches(spec, home)),
            Self::Unreadable { .. } => true,
        }
    }

    /// Lowers `agents`, what agents may do on `spec` as the person set it,
    /// to what the policy lets them: the lowest of the person's value and
    /// each matching rule's.
    pub fn floor(&self, spec: &ConnectSpec, home: Option<&Path>, agents: &mut Agents) {
        let policy = match self {
            Self::None => return,
            Self::Read { policy, .. } => policy,
            Self::Unreadable { .. } => {
                agents.access = AgentAccess::Off;
                return;
            }
        };
        if !policy.allowed {
            agents.access = AgentAccess::Off;
        }
        let mut matched = false;
        for rule in policy
            .rules
            .iter()
            .filter(|rule| rule.matches.matches(spec, home))
        {
            matched = true;
            if let Some(most) = rule.access_max {
                agents.access = agents.access.min(most);
            }
            if let Some(most) = rule.rows_max {
                agents.rows = agents.rows.min(most);
            }
            if let Some(most) = rule.timeout_max_secs {
                agents.timeout_secs = agents.timeout_secs.min(most);
            }
        }
        if !matched && policy.unmatched_off {
            agents.access = AgentAccess::Off;
        }
    }

    /// What each rule that matches `spec` hides and masks, for the agent
    /// checks to take beside the person's own: a table is visible only
    /// where each lets it be seen, and a column is masked where any masks
    /// it. A rule that hides and masks nothing is left out. An unreadable
    /// policy has none: [`Self::floor`] keeps agents off.
    pub fn sources(&self, spec: &ConnectSpec, home: Option<&Path>) -> Vec<Source> {
        match self {
            Self::None | Self::Unreadable { .. } => Vec::new(),
            Self::Read { policy, .. } => policy
                .rules
                .iter()
                .filter(|rule| rule.matches.matches(spec, home))
                .map(Rule::source)
                .filter(Source::guards_anything)
                .collect(),
        }
    }

    /// How many days the log of agents' calls keeps a call: the person's
    /// `days`, or the policy's least where that is more.
    pub fn log_days(&self, days: u32) -> u32 {
        match self {
            Self::Read {
                policy:
                    Policy {
                        log_days_min: Some(least),
                        ..
                    },
                ..
            } => days.max(*least),
            Self::None | Self::Read { .. } | Self::Unreadable { .. } => days,
        }
    }
}

/// Whether only an administrator can change or replace the file at `path`:
/// it and every folder above it are owned by root and writable by no group
/// and no other user, and none is a link.
#[cfg(unix)]
fn locked(path: &Path) -> bool {
    use std::os::unix::fs::MetadataExt;
    let mut at = Some(path);
    while let Some(here) = at {
        let Ok(found) = std::fs::symlink_metadata(here) else {
            return false;
        };
        if !held_by_root(found.file_type().is_symlink(), found.uid(), found.mode()) {
            return false;
        }
        at = here
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty());
    }
    true
}

/// Windows is read from run 1c, with a check of its own.
#[cfg(not(unix))]
fn locked(_path: &Path) -> bool {
    false
}

/// Whether one entry of a path leaves it to root alone.
#[cfg(unix)]
fn held_by_root(link: bool, owner: u32, mode: u32) -> bool {
    !link && owner == 0 && mode & 0o022 == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    const EXAMPLE: &str = r#"
version = 1

[agents]
allowed = true
log_days_min = 90

[[rules]]
name = "Bookshop production"
match = { driver = "postgres", host = "db.example.com", database = "bookshop_*" }
read_only = true
access_max = "read"
rows_max = 500
timeout_max_secs = 10
hidden = ["customers", "billing.invoices"]
masked = ["orders.email"]

[[rules]]
name = "Analytics replica"
match = { driver = "postgres", host = "replica.example.com" }
visible_only = ["books", "sales.orders"]

[[rules]]
name = "Local files"
match = { driver = "sqlite", path = "/srv/shared/*.db" }
access_max = "off"
"#;

    fn problem(text: &str) -> String {
        read(text).expect_err("the file is refused").to_string()
    }

    fn postgres(host: &str, database: &str) -> ConnectSpec {
        ConnectSpec {
            host: host.into(),
            database: database.into(),
            ..ConnectSpec::default()
        }
    }

    fn policy(text: &str) -> Managed {
        Managed::Read {
            path: PathBuf::from("/etc/tabletist/managed.toml"),
            policy: read(text).expect("the policy reads"),
            locked: true,
        }
    }

    fn read_agents(rows: u32, timeout_secs: u32) -> Agents {
        Agents {
            access: AgentAccess::Read,
            rows,
            timeout_secs,
            ..Agents::default()
        }
    }

    #[test]
    fn the_specs_example_reads() {
        let read = read(EXAMPLE).unwrap();
        assert!(read.allowed);
        assert!(read.unmatched_off, "a file with rules turns the rest off");
        assert_eq!(read.log_days_min, Some(90));
        assert_eq!(read.rules.len(), 3);
        let first = &read.rules[0];
        assert_eq!(first.name, "Bookshop production");
        assert!(first.read_only);
        assert_eq!(first.access_max, Some(AgentAccess::Read));
        assert_eq!(
            (first.rows_max, first.timeout_max_secs),
            (Some(500), Some(10))
        );
        assert_eq!(
            first.hidden,
            vec![
                "customers".parse().unwrap(),
                "billing.invoices".parse().unwrap()
            ]
        );
        assert_eq!(first.masked, vec!["orders.email".parse().unwrap()]);
        assert_eq!(first.matches.database.as_deref(), Some("bookshop_*"));
        assert_eq!(read.rules[1].visible_only.as_ref().map(Vec::len), Some(2));
        assert_eq!(read.rules[2].access_max, Some(AgentAccess::Off));
    }

    #[test]
    fn a_problem_says_its_line_and_never_its_value() {
        let unknown = problem("version = 1\n[agents]\nalowed = true\n");
        assert!(unknown.starts_with("line 3: "), "{unknown}");
        let value = problem(
            "version = 1\n[[rules]]\nname = \"a\"\nmatch = { driver = \"*\" }\naccess_max = \"write\"\n",
        );
        assert!(
            value.starts_with("line 5: a value the key cannot have"),
            "{value}"
        );
        assert!(!value.contains("write"), "{value}");
        let entry = problem(
            "version = 1\n[[rules]]\nname = \"a\"\nmatch = { driver = \"*\" }\nhidden = [\"a.b.c\"]\n",
        );
        assert!(entry.starts_with("rule 1: hidden entry 1: "), "{entry}");
        assert!(!entry.contains("a.b.c"), "{entry}");
    }

    #[test]
    fn a_version_this_tabletist_does_not_read_is_refused() {
        assert!(problem("version = 2\n").contains("for a newer Tabletist"));
        assert!(problem("version = 0\n").contains("first version of this file is 1"));
        assert!(problem("[agents]\n").contains("missing field `version`"));
    }

    #[test]
    fn what_later_versions_hold_is_refused_and_not_skipped() {
        let connections = problem("version = 1\n[[connections]]\nname = \"a\"\n");
        assert!(connections.contains("it has connections"), "{connections}");
        let service = problem("version = 1\n[service]\nurl = \"https://example.com\"\n");
        assert!(service.contains("it has a service"), "{service}");
    }

    #[test]
    fn a_rule_must_say_what_it_matches_and_may_only_restrict() {
        let rule = |body: &str| format!("version = 1\n[[rules]]\nname = \"a\"\n{body}\n");
        assert!(problem(&rule("")).contains("missing field `match`"));
        assert!(problem(&rule("match = {}")).contains("rule 1: its match is empty"));
        assert!(
            problem("version = 1\n[[rules]]\nname = \" \"\nmatch = { driver = \"*\" }\n")
                .contains("its name is empty")
        );
        let any = "match = { driver = \"*\" }";
        assert!(problem(&rule(&format!("{any}\nread_only = false"))).contains("never writable"));
        for (key, value) in [
            ("rows_max", 0),
            ("rows_max", 5001),
            ("timeout_max_secs", 0),
            ("timeout_max_secs", 61),
        ] {
            let said = problem(&rule(&format!("{any}\n{key} = {value}")));
            assert!(
                said.contains(&format!("{key} is out of its range")),
                "{said}"
            );
        }
        for days in [0, 3651] {
            let said = problem(&format!("version = 1\n[agents]\nlog_days_min = {days}\n"));
            assert!(said.contains("log_days_min is out of its range"), "{said}");
        }
        let both = problem(&rule(&format!(
            "{any}\nhidden = [\"a\"]\nvisible_only = [\"b\"]"
        )));
        assert!(both.contains("both hidden and visible_only"), "{both}");
        let driver = problem(&rule("match = { driver = \"postgresql\" }"));
        assert!(driver.contains("none that Tabletist knows"), "{driver}");
        assert!(!driver.contains("postgresql"), "{driver}");
        assert!(read(&rule("match = { driver = \"my*\" }")).is_ok());
    }

    #[test]
    fn connections_no_rule_matches_are_off_once_there_is_a_rule() {
        assert!(!read("version = 1\n").unwrap().unmatched_off);
        assert!(
            read("version = 1\n[agents]\nunmatched = \"off\"\n")
                .unwrap()
                .unmatched_off
        );
        let set = "version = 1\n[agents]\nunmatched = \"as_set\"\n[[rules]]\nname = \"a\"\nmatch = { driver = \"*\" }\n";
        assert!(!read(set).unwrap().unmatched_off);
    }

    #[test]
    fn a_star_stands_for_any_run_of_characters() {
        for (pattern, text, matches) in [
            ("db", "db", true),
            ("db", "db2", false),
            ("*", "", true),
            ("*", "anything", true),
            ("book*", "bookshop_production", true),
            ("*_production", "bookshop_production", true),
            ("b*p", "bookshop", true),
            ("b*p", "bookshops", false),
            ("a*b*c", "aXbYbZc", true),
            ("a*b*c", "aXbYbZ", false),
            ("", "", true),
            ("", "x", false),
            ("**", "x", true),
        ] {
            assert_eq!(glob(pattern, text), matches, "{pattern} against {text}");
        }
    }

    #[test]
    fn a_rule_matches_what_a_connection_reaches() {
        let host = Match {
            host: Some("DB.example.*".into()),
            port: Some(5432),
            database: Some("bookshop_*".into()),
            ..Match::default()
        };
        assert!(host.matches(&postgres("db.Example.com", "bookshop_production"), None));
        assert!(!host.matches(&postgres("db.example.com", "Bookshop_production"), None));
        let elsewhere = ConnectSpec {
            port: 5433,
            ..postgres("db.example.com", "bookshop_production")
        };
        assert!(!host.matches(&elsewhere, None));
        // Through a tunnel, the database's host.
        let tunnelled = ConnectSpec {
            ssh: Some(tabletist_db::SshSpec {
                host: "bastion.example.com".into(),
                port: None,
                user: String::new(),
                auth: tabletist_db::SshAuth::Agent,
            }),
            ..postgres("db.example.com", "bookshop_production")
        };
        assert!(host.matches(&tunnelled, None));
        let driver = Match {
            driver: Some("mysql".into()),
            ..Match::default()
        };
        assert!(!driver.matches(&postgres("db", "x"), None));
        // A key the driver has no use for does not match. The pattern is
        // written as the system writes an absolute path.
        let folder = std::path::absolute("/home/ada").unwrap();
        let path = Match {
            path: Some(folder.join("*.db").to_string_lossy().into_owned()),
            ..Match::default()
        };
        assert!(!path.matches(&postgres("db", "x"), None));
        let on_sqlite = Match {
            host: Some("*".into()),
            ..Match::default()
        };
        assert!(!on_sqlite.matches(&ConnectSpec::sqlite("/home/ada/a.db"), None));
        // A path with `~` is the person's home.
        let home = Path::new("/home/ada");
        assert!(path.matches(&ConnectSpec::sqlite("~/books.db"), Some(home)));
        assert!(!path.matches(&ConnectSpec::sqlite("~/books.db"), None));
        assert!(path.matches(&ConnectSpec::sqlite("/home/ada/books.db"), None));
        assert!(!path.matches(&ConnectSpec::sqlite("/srv/books.db"), Some(home)));
    }

    #[cfg(unix)]
    #[test]
    fn only_root_holding_every_step_of_the_path_locks_it() {
        assert!(held_by_root(false, 0, 0o100644));
        assert!(held_by_root(false, 0, 0o40755));
        assert!(!held_by_root(false, 0, 0o100664), "its group may write");
        assert!(!held_by_root(false, 0, 0o100646), "anyone may write");
        assert!(!held_by_root(true, 0, 0o120777), "a link");
        assert!(!held_by_root(false, 1000, 0o100644), "a user's");
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("managed.toml");
        std::fs::write(&file, "version = 1\n").unwrap();
        use std::os::unix::fs::MetadataExt;
        if std::fs::metadata(&file).unwrap().uid() != 0 {
            assert!(!locked(&file), "the test's own file");
        }
    }

    #[test]
    fn a_missing_file_is_no_policy_and_a_bad_one_fails_closed() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("managed.toml");
        assert_eq!(Managed::load(Some(&file)), Managed::None);
        assert_eq!(Managed::load(None), Managed::None);
        std::fs::write(&file, "version = 1\n[agents]\nallowed = \"maybe\"\n").unwrap();
        let bad = Managed::load(Some(&file));
        assert!(matches!(bad, Managed::Unreadable { .. }), "{bad:?}");
        let notice = bad.notice().unwrap();
        assert!(
            notice.contains("line 3: a value of the wrong kind"),
            "{notice}"
        );
        assert!(!notice.contains("maybe"), "{notice}");
        assert!(notice.contains("agents are off and every connection is read-only"));
        assert!(!bad.server_allowed());
        assert!(bad.forces_read_only(&postgres("db", "x"), None));
        let mut agents = read_agents(500, 10);
        bad.floor(&postgres("db", "x"), None, &mut agents);
        assert_eq!(agents.access, AgentAccess::Off);
        // A folder where the file should be cannot be read.
        let folder = dir.path().join("folder");
        std::fs::create_dir(&folder).unwrap();
        assert!(matches!(
            Managed::load(Some(&folder)),
            Managed::Unreadable { .. }
        ));
        std::fs::write(&file, "version = 1\n").unwrap();
        assert!(matches!(Managed::load(Some(&file)), Managed::Read { .. }));
        assert_eq!(Managed::load(Some(&file)).notice(), None);
    }

    #[test]
    fn the_floor_is_the_lowest_of_the_person_and_each_matching_rule() {
        let managed = policy(
            r#"version = 1
[[rules]]
name = "caps"
match = { host = "db.example.com" }
rows_max = 200
timeout_max_secs = 30
[[rules]]
name = "tighter"
match = { database = "bookshop" }
timeout_max_secs = 5
"#,
        );
        let mut agents = read_agents(500, 10);
        managed.floor(&postgres("db.example.com", "bookshop"), None, &mut agents);
        assert_eq!(
            (agents.access, agents.rows, agents.timeout_secs),
            (AgentAccess::Read, 200, 5)
        );
        // Below the ceiling, the person's own.
        let mut agents = read_agents(50, 10);
        managed.floor(&postgres("db.example.com", "other"), None, &mut agents);
        assert_eq!((agents.rows, agents.timeout_secs), (50, 10));
        // No rule matches, and there are rules: off.
        let mut agents = read_agents(500, 10);
        managed.floor(&postgres("elsewhere", "bookshop_x"), None, &mut agents);
        assert_eq!(agents.access, AgentAccess::Off);
        assert_eq!(agents.rows, 500, "only the access is lowered");
    }

    #[test]
    fn a_rules_access_and_unmatched_as_set_leave_the_rest_as_the_person_has_it() {
        let managed = policy(
            r#"version = 1
[agents]
unmatched = "as_set"
[[rules]]
name = "off"
match = { host = "db.example.com" }
access_max = "off"
"#,
        );
        let mut agents = read_agents(500, 10);
        managed.floor(&postgres("db.example.com", "x"), None, &mut agents);
        assert_eq!(agents.access, AgentAccess::Off);
        let mut agents = read_agents(500, 10);
        managed.floor(&postgres("elsewhere", "x"), None, &mut agents);
        assert_eq!(agents.access, AgentAccess::Read);
    }

    #[test]
    fn a_rules_tables_and_columns_go_to_the_checks_and_leave_access_as_it_is() {
        let named = |name: &str| name.parse::<TableName>().unwrap();
        for (lists, tables, masked) in [
            (
                "hidden = [\"customers\"]",
                Tables::AllExcept(vec![named("customers")]),
                Vec::new(),
            ),
            ("visible_only = []", Tables::Only(Vec::new()), Vec::new()),
            (
                "visible_only = [\"books\"]",
                Tables::Only(vec![named("books")]),
                Vec::new(),
            ),
            (
                "masked = [\"orders.email\"]",
                Tables::All,
                vec!["orders.email".parse::<ColumnName>().unwrap()],
            ),
        ] {
            let managed = policy(&format!(
                "version = 1\n[[rules]]\nname = \"a\"\nmatch = {{ driver = \"*\" }}\n{lists}\n"
            ));
            let mut agents = read_agents(500, 10);
            managed.floor(&postgres("db", "x"), None, &mut agents);
            assert_eq!(agents.access, AgentAccess::Read, "{lists}");
            assert_eq!(
                managed.sources(&postgres("db", "x"), None),
                vec![Source { tables, masked }],
                "{lists}"
            );
        }
        // Each matching rule is its own source; one that matches not, or
        // hides and masks nothing, is none.
        let managed = policy(
            "version = 1
[[rules]]
name = \"a\"
match = { driver = \"*\" }
hidden = [\"customers\"]
[[rules]]
name = \"b\"
match = { host = \"db\" }
visible_only = [\"books\"]
[[rules]]
name = \"c\"
match = { host = \"elsewhere\" }
hidden = [\"orders\"]
[[rules]]
name = \"d\"
match = { driver = \"*\" }
hidden = []
masked = []
",
        );
        assert_eq!(
            managed.sources(&postgres("db", "x"), None),
            vec![
                Source {
                    tables: Tables::AllExcept(vec![named("customers")]),
                    masked: Vec::new(),
                },
                Source {
                    tables: Tables::Only(vec![named("books")]),
                    masked: Vec::new(),
                },
            ]
        );
        assert_eq!(
            Managed::None.sources(&postgres("db", "x"), None),
            Vec::new()
        );
        let unreadable = Managed::Unreadable {
            path: PathBuf::from("/m"),
            problem: "bad".into(),
        };
        assert_eq!(unreadable.sources(&postgres("db", "x"), None), Vec::new());
    }

    #[test]
    fn the_server_read_only_and_log_days_follow_the_policy() {
        assert!(Managed::None.server_allowed());
        assert!(policy("version = 1\n").server_allowed());
        let forbids = policy("version = 1\n[agents]\nallowed = false\nunmatched = \"as_set\"\n");
        assert!(!forbids.server_allowed());
        let mut agents = read_agents(500, 10);
        forbids.floor(&postgres("db", "x"), None, &mut agents);
        assert_eq!(agents.access, AgentAccess::Off);
        let managed = policy(EXAMPLE);
        assert!(managed.forces_read_only(&postgres("db.example.com", "bookshop_production"), None));
        assert!(!managed.forces_read_only(&postgres("replica.example.com", "x"), None));
        assert!(!Managed::None.forces_read_only(&postgres("db", "x"), None));
        assert_eq!(managed.log_days(30), 90);
        assert_eq!(managed.log_days(365), 365);
        assert_eq!(Managed::None.log_days(30), 30);
    }
}

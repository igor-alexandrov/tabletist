//! A Redis connection's window, as data: the key names a scan brought,
//! grouped by the separator into a tree, the scan's progress, the types
//! read for the rows on screen, and the keys open in tabs with their
//! values. Nothing here draws or talks to the server: `ui::redis` draws
//! it, and the app sends what it asks for to the backend.
//!
//! Redis has no index of prefixes, so the tree is made of the names
//! alone, as they arrive: each part of a name but its last is a group
//! (`book:reviews:12` sits under `book:` › `reviews:`), and a name with no
//! separator at the top. A scan can return a name twice, and each is held
//! once.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::time::Duration;

use tabletist_db::redis::{Cursor, KeyInfo, KeyType, Loaded, Page, Probe};
use tabletist_db::{Browsing, Error, StopFlag};

use crate::backend::RequestId;
use crate::model::Fetch;

/// A key's name, as the server has it: any bytes.
pub type Key = Box<[u8]>;

/// A group of the tree: the groups under it by their part, the keys whose
/// last part is in it, and how many keys it holds in all.
#[derive(Debug, Default, Clone)]
pub struct Group {
    groups: BTreeMap<Key, Group>,
    keys: BTreeSet<Key>,
    count: usize,
}

impl Group {
    /// How many keys it holds, its groups' with its own.
    pub fn count(&self) -> usize {
        self.count
    }

    /// A group of one key and no groups is shown as that key.
    fn lone(&self) -> Option<&Key> {
        (self.count == 1 && self.groups.is_empty())
            .then(|| self.keys.iter().next())
            .flatten()
    }
}

/// The names held, grouped by the separator.
#[derive(Debug, Clone)]
pub struct KeyTree {
    separator: Vec<u8>,
    root: Group,
}

impl KeyTree {
    pub fn new(separator: &str) -> Self {
        Self {
            separator: separator.as_bytes().to_vec(),
            root: Group::default(),
        }
    }

    /// The number of names held, each once.
    pub fn len(&self) -> usize {
        self.root.count
    }

    pub fn is_empty(&self) -> bool {
        self.root.count == 0
    }

    pub fn separator(&self) -> &[u8] {
        &self.separator
    }

    /// The parts of `key`: every part but its last is a group.
    fn parts<'a>(&self, key: &'a [u8]) -> Vec<&'a [u8]> {
        if self.separator.is_empty() {
            return vec![key];
        }
        let mut parts = Vec::new();
        let mut rest = key;
        while let Some(at) = find(rest, &self.separator) {
            parts.push(&rest[..at]);
            rest = &rest[at + self.separator.len()..];
        }
        parts.push(rest);
        parts
    }

    /// Holds `key`, once: false when it was held already.
    pub fn insert(&mut self, key: &[u8]) -> bool {
        let parts = self.parts(key);
        let groups = &parts[..parts.len() - 1];
        // Where it goes first, then the counts on the way: a name held
        // already counts nowhere again.
        let mut group = &mut self.root;
        for part in groups {
            group = group.groups.entry(Box::from(*part)).or_default();
        }
        if !group.keys.insert(Box::from(key)) {
            return false;
        }
        let mut group = &mut self.root;
        group.count += 1;
        for part in groups {
            let Some(next) = group.groups.get_mut(*part) else {
                break;
            };
            group = next;
            group.count += 1;
        }
        true
    }

    /// The group whose prefix is `prefix` (`book:reviews:`), the root for
    /// an empty one.
    pub fn group(&self, prefix: &[u8]) -> Option<&Group> {
        if prefix.is_empty() {
            return Some(&self.root);
        }
        let inner = prefix.strip_suffix(self.separator.as_slice())?;
        let mut group = &self.root;
        for part in self.parts(inner) {
            group = group.groups.get(part)?;
        }
        Some(group)
    }

    /// Every name held, in order: the groups' first, then the top's.
    pub fn names(&self) -> Vec<&Key> {
        fn walk<'a>(group: &'a Group, out: &mut Vec<&'a Key>) {
            for inner in group.groups.values() {
                walk(inner, out);
            }
            out.extend(group.keys.iter());
        }
        let mut out = Vec::with_capacity(self.len());
        walk(&self.root, &mut out);
        out
    }

    /// The groups held, by their prefixes (`book:`, `book:reviews:`).
    pub fn prefixes(&self) -> Vec<(Vec<u8>, usize)> {
        fn walk(group: &Group, prefix: &[u8], separator: &[u8], out: &mut Vec<(Vec<u8>, usize)>) {
            for (part, inner) in &group.groups {
                if inner.lone().is_some() {
                    continue;
                }
                let mut own = prefix.to_vec();
                own.extend_from_slice(part);
                own.extend_from_slice(separator);
                out.push((own.clone(), inner.count));
                walk(inner, &own, separator, out);
            }
        }
        let mut out = Vec::new();
        walk(&self.root, &[], &self.separator, &mut out);
        out
    }

    /// The rows the sidebar shows: each group with its count, its own rows
    /// under it where it is open, and its keys a page at a time.
    /// `shown` says how many keys of a group are shown (a page until More
    /// shows the next), `page` what a group shows first.
    pub fn rows(
        &self,
        open: &HashSet<Vec<u8>>,
        shown: &HashMap<Vec<u8>, usize>,
        page: usize,
    ) -> Vec<Row> {
        let mut rows = Vec::new();
        self.rows_of(&self.root, &[], 0, (open, shown, page), &mut rows);
        rows
    }

    fn rows_of(
        &self,
        group: &Group,
        prefix: &[u8],
        depth: usize,
        how: (&HashSet<Vec<u8>>, &HashMap<Vec<u8>, usize>, usize),
        rows: &mut Vec<Row>,
    ) {
        let (open, shown, page) = how;
        let mut keys: Vec<&Key> = Vec::new();
        for (part, inner) in &group.groups {
            // A group of one is its key, among this group's keys.
            if let Some(key) = inner.lone() {
                keys.push(key);
                continue;
            }
            let mut own = prefix.to_vec();
            own.extend_from_slice(part);
            own.extend_from_slice(&self.separator);
            let is_open = open.contains(&own);
            rows.push(Row::Group {
                name: [part.as_ref(), self.separator.as_slice()].concat(),
                count: inner.count,
                open: is_open,
                depth,
                prefix: own.clone(),
            });
            if is_open {
                self.rows_of(inner, &own, depth + 1, how, rows);
            }
        }
        keys.extend(group.keys.iter());
        let limit = shown.get(prefix).copied().unwrap_or(page).max(1);
        // Only the keys shown are put in order: a group of a hundred
        // thousand is not sorted whole on every frame for its first page.
        if keys.len() > limit {
            keys.select_nth_unstable_by(limit, |a, b| natural(a, b));
            keys[..limit].sort_unstable_by(|a, b| natural(a, b));
        } else {
            keys.sort_unstable_by(|a, b| natural(a, b));
        }
        for key in keys.iter().take(limit) {
            rows.push(Row::Key {
                key: Box::from(key.as_ref()),
                name: key.strip_prefix(prefix).unwrap_or(key).to_vec(),
                depth,
            });
        }
        if keys.len() > limit {
            rows.push(Row::More {
                prefix: prefix.to_vec(),
                left: keys.len() - limit,
                depth,
            });
        }
    }
}

/// Where `text` first holds `query`, without regard to case, as a range of
/// `text`'s own bytes. The two are compared lowered, and a match that
/// starts or ends inside what one character lowers to (`İ` lowers to `i̇`,
/// so `i` and `\u{307}x` both match in `İx`) takes that character whole:
/// never a slice of its bytes.
pub fn find_folded(text: &str, query: &str) -> Option<std::ops::Range<usize>> {
    let wanted: Vec<char> = query.chars().flat_map(char::to_lowercase).collect();
    if wanted.is_empty() {
        return None;
    }
    // Each lowered character, with the bytes of the one it came from.
    let lowered: Vec<(char, std::ops::Range<usize>)> = text
        .char_indices()
        .flat_map(|(at, c)| {
            let bytes = at..at + c.len_utf8();
            c.to_lowercase().map(move |lower| (lower, bytes.clone()))
        })
        .collect();
    let at = lowered
        .windows(wanted.len())
        .position(|window| window.iter().map(|(c, _)| *c).eq(wanted.iter().copied()))?;
    let first = &lowered[at].1;
    let last = &lowered[at + wanted.len() - 1].1;
    Some(first.start..last.end)
}

/// A key's name as a `SCAN MATCH` pattern that matches it as it is: the
/// glob's own characters (`*`, `?`, `[`, `]`, `\`) escaped.
pub fn glob_escaped(name: &[u8]) -> String {
    let mut out = String::new();
    for c in lossy(name).chars() {
        if matches!(c, '*' | '?' | '[' | ']' | '\\') {
            out.push('\\');
        }
        out.push(c);
    }
    out
}

/// `a` before `b` as a person counts: runs of digits by their number
/// (`book:2` before `book:10`), the rest byte by byte.
pub fn natural(a: &[u8], b: &[u8]) -> std::cmp::Ordering {
    use std::cmp::Ordering;
    let (mut i, mut j) = (0, 0);
    while i < a.len() && j < b.len() {
        if a[i].is_ascii_digit() && b[j].is_ascii_digit() {
            let start = (i, j);
            while i < a.len() && a[i].is_ascii_digit() {
                i += 1;
            }
            while j < b.len() && b[j].is_ascii_digit() {
                j += 1;
            }
            let (x, y) = (&a[start.0..i], &b[start.1..j]);
            let trim = |run: &[u8]| -> usize { run.iter().take_while(|d| **d == b'0').count() };
            let (x, y) = (&x[trim(x)..], &y[trim(y)..]);
            let order = x.len().cmp(&y.len()).then_with(|| x.cmp(y));
            if order != Ordering::Equal {
                return order;
            }
        } else {
            let order = a[i].cmp(&b[j]);
            if order != Ordering::Equal {
                return order;
            }
            i += 1;
            j += 1;
        }
    }
    (a.len() - i).cmp(&(b.len() - j)).then_with(|| a.cmp(b))
}

/// Where `needle` first is in `haystack`.
fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

/// One row of the sidebar's tree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Row {
    /// A group: its last part with the separator (`reviews:`), the keys
    /// under it, and its whole prefix (`book:reviews:`).
    Group {
        name: Vec<u8>,
        prefix: Vec<u8>,
        count: usize,
        open: bool,
        depth: usize,
    },
    /// A key: its name, and the part of it the row shows (after its
    /// group's prefix).
    Key {
        key: Key,
        name: Vec<u8>,
        depth: usize,
    },
    /// The keys of a group not shown yet, which More shows a page of.
    More {
        prefix: Vec<u8>,
        left: usize,
        depth: usize,
    },
}

impl Row {
    pub fn depth(&self) -> usize {
        match self {
            Self::Group { depth, .. } | Self::Key { depth, .. } | Self::More { depth, .. } => {
                *depth
            }
        }
    }

    /// What a keyboard cursor on the row is.
    pub fn place(&self) -> Place {
        match self {
            Self::Group { prefix, .. } => Place::Group(prefix.clone()),
            Self::Key { key, .. } => Place::Key(key.clone()),
            Self::More { prefix, .. } => Place::More(prefix.clone()),
        }
    }
}

/// Where the tree's cursor is, by what is there rather than by its row:
/// rows come and go as a scan brings names.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Place {
    Group(Vec<u8>),
    Key(Key),
    More(Vec<u8>),
}

/// How a scan ended, or that it runs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScanState {
    /// Nothing asked yet.
    Idle,
    /// Steps of `SCAN` arrive; `request` is theirs.
    Running {
        request: RequestId,
    },
    /// The cursor came back to 0: every name that matched is held.
    Done,
    /// Stopped by the person.
    Stopped,
    /// Stopped at the key limit: the groups are counted from a sample.
    Limited,
    Failed(Error),
}

/// A scan of the database's names: its progress, and what it held.
#[derive(Debug, Clone)]
pub struct Scan {
    pub state: ScanState,
    /// The pattern it matches (`*` for all).
    pub pattern: String,
    /// `DBSIZE`, read first: the keys of the database, matching or not.
    pub total: Option<u64>,
    /// Where it goes on from, for Scan all after the limit.
    pub cursor: u64,
    /// The steps it took, and how long they took in all.
    pub steps: u32,
    pub took: Duration,
    /// Set to stop it: the backend's task asks it between steps.
    pub stop: StopFlag,
    /// Whether Scan all lifted the key limit.
    pub unlimited: bool,
}

impl Default for Scan {
    fn default() -> Self {
        Self {
            state: ScanState::Idle,
            pattern: "*".into(),
            total: None,
            cursor: 0,
            steps: 0,
            took: Duration::ZERO,
            stop: StopFlag::new(),
            unlimited: false,
        }
    }
}

impl Scan {
    pub fn running(&self) -> bool {
        matches!(self.state, ScanState::Running { .. })
    }

    /// Whether the names held are all that match: counts are exact.
    pub fn complete(&self) -> bool {
        matches!(self.state, ScanState::Done)
    }
}

/// One step of a scan, as the backend sends it.
#[derive(Debug, Clone)]
pub struct ScanStep {
    /// `DBSIZE`, on the first step.
    pub total: Option<u64>,
    pub names: Vec<Vec<u8>>,
    /// Where the next step starts; 0 after the last.
    pub cursor: u64,
    pub end: ScanEnd,
    pub took: Duration,
}

/// Whether a step is the scan's last, and why.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScanEnd {
    More,
    Done,
    Limited,
    Stopped,
}

/// The rows of a value as the main area shows them.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Rows {
    /// A string or a JSON value whole, as it is shown.
    pub text: Option<Shown>,
    /// The columns of a hash, list, set, sorted set or stream.
    pub columns: Vec<&'static str>,
    pub cells: Vec<Vec<Vec<u8>>>,
    /// A type this version does not show, by its name.
    pub unsupported: Option<String>,
}

/// A string as it is shown: text (JSON pretty-printed), or hex for one
/// that is not text.
#[derive(Debug, Clone, PartialEq)]
pub enum Shown {
    Text(String),
    Json(String),
    Binary { hex: String, length: u64 },
}

/// A key open in a tab: its header, its value's rows so far, and where the
/// next page starts.
#[derive(Debug)]
pub struct KeyTab {
    pub key: Key,
    pub info: Fetch<KeyInfo>,
    pub rows: Rows,
    /// What `Load more` asks for; `None` after the last page.
    pub next: Option<Cursor>,
    pub loading: Option<RequestId>,
    pub error: Option<Error>,
    /// The bytes of the string read, and its whole length, where the
    /// value is a string.
    pub text_length: Option<(u64, u64)>,
    /// The command that read the last page, how many it holds, and how
    /// long it took: the line under the value.
    pub command: Option<(&'static str, Duration)>,
    /// The cell selected in the value's table, by row and column: the
    /// copy key copies it.
    pub selected: Option<(usize, usize)>,
    /// Members of a set or fields of a hash read so far, so an `SSCAN`
    /// that returns one twice shows it once.
    seen: HashSet<Vec<u8>>,
}

impl KeyTab {
    pub fn new(key: Key) -> Self {
        Self {
            key,
            info: Fetch::default(),
            rows: Rows::default(),
            next: None,
            loading: None,
            error: None,
            text_length: None,
            command: None,
            selected: None,
            seen: HashSet::new(),
        }
    }

    /// The selected cell's text, as Copy copies it.
    pub fn selected_text(&self) -> Option<String> {
        let (row, column) = self.selected?;
        Some(lossy(self.rows.cells.get(row)?.get(column)?))
    }

    /// Takes a page read for the tab: the rows it holds after the ones
    /// shown, and where the next starts.
    pub fn take(&mut self, loaded: Loaded) {
        self.command = Some((loaded.command, loaded.took));
        self.next = loaded.next;
        match loaded.page {
            Page::Text { bytes, length } => {
                self.text_length = Some((bytes.len() as u64, length));
                self.rows.text = Some(shown_text(&bytes, length));
            }
            Page::Json(text) => self.rows.text = Some(Shown::Json(pretty(&text))),
            Page::Pairs(pairs) => {
                self.rows.columns = vec!["Field", "Value"];
                for (field, value) in pairs {
                    if self.seen.insert(field.clone()) {
                        self.rows.cells.push(vec![field, value]);
                    }
                }
            }
            Page::Items(items) => {
                let list = matches!(
                    self.info.value.as_ref().map(|info| &info.kind),
                    Some(KeyType::List)
                );
                if list {
                    self.rows.columns = vec!["Index", "Value"];
                    let start = self.rows.cells.len();
                    for (at, item) in items.into_iter().enumerate() {
                        self.rows
                            .cells
                            .push(vec![(start + at).to_string().into_bytes(), item]);
                    }
                } else {
                    self.rows.columns = vec!["Value"];
                    for item in items {
                        if self.seen.insert(item.clone()) {
                            self.rows.cells.push(vec![item]);
                        }
                    }
                }
            }
            Page::Scored(members) => {
                self.rows.columns = vec!["Rank", "Score", "Member"];
                let start = self.rows.cells.len();
                for (at, (member, score)) in members.into_iter().enumerate() {
                    self.rows.cells.push(vec![
                        (start + at).to_string().into_bytes(),
                        score.into_bytes(),
                        member,
                    ]);
                }
            }
            Page::Entries(entries) => {
                self.rows.columns = vec!["Entry ID", "Fields"];
                for (id, fields) in entries {
                    let pairs: Vec<String> = fields
                        .iter()
                        .map(|(field, value)| format!("{}={}", lossy(field), lossy(value)))
                        .collect();
                    self.rows
                        .cells
                        .push(vec![id.into_bytes(), pairs.join(" ").into_bytes()]);
                }
            }
            Page::Unsupported(name) => self.rows.unsupported = Some(name),
        }
    }

    /// The value as Copy value copies it: a string as it is shown, the
    /// rows of the others as JSON.
    pub fn copied(&self) -> Option<String> {
        if let Some(text) = &self.rows.text {
            return Some(match text {
                Shown::Text(text) | Shown::Json(text) => text.clone(),
                Shown::Binary { hex, .. } => hex.clone(),
            });
        }
        let kind = self.info.value.as_ref().map(|info| &info.kind)?;
        let text = |bytes: &[u8]| serde_json::Value::String(lossy(bytes));
        let value = match kind {
            KeyType::Hash => serde_json::Value::Object(
                self.rows
                    .cells
                    .iter()
                    .map(|row| (lossy(&row[0]), text(&row[1])))
                    .collect(),
            ),
            KeyType::List => {
                serde_json::Value::Array(self.rows.cells.iter().map(|row| text(&row[1])).collect())
            }
            KeyType::Set => {
                serde_json::Value::Array(self.rows.cells.iter().map(|row| text(&row[0])).collect())
            }
            KeyType::ZSet => serde_json::Value::Array(
                self.rows
                    .cells
                    .iter()
                    .map(|row| {
                        let score = lossy(&row[1]);
                        let score = score
                            .parse::<f64>()
                            .ok()
                            .and_then(serde_json::Number::from_f64)
                            .map_or_else(
                                || serde_json::Value::String(score),
                                serde_json::Value::Number,
                            );
                        serde_json::json!({ "member": text(&row[2]), "score": score })
                    })
                    .collect(),
            ),
            KeyType::Stream => serde_json::Value::Array(
                self.rows
                    .cells
                    .iter()
                    .map(|row| serde_json::json!({ "id": text(&row[0]), "fields": text(&row[1]) }))
                    .collect(),
            ),
            KeyType::String | KeyType::Json | KeyType::Other(_) | KeyType::Missing => return None,
        };
        serde_json::to_string_pretty(&value).ok()
    }
}

/// Bytes as text, a byte that is not UTF-8 as `\xNN`.
pub fn lossy(bytes: &[u8]) -> String {
    match std::str::from_utf8(bytes) {
        Ok(text) => text.to_owned(),
        Err(_) => bytes
            .iter()
            .map(|byte| match byte {
                0x20..=0x7e if *byte != b'\\' => char::from(*byte).to_string(),
                _ => format!("\\x{byte:02x}"),
            })
            .collect(),
    }
}

/// JSON pretty-printed where it parses, else as it is.
fn pretty(text: &str) -> String {
    serde_json::from_str::<serde_json::Value>(text)
        .ok()
        .and_then(|value| serde_json::to_string_pretty(&value).ok())
        .unwrap_or_else(|| text.to_owned())
}

/// A string as it is shown: JSON pretty-printed where the whole of it
/// parses as an object or an array, text where it is UTF-8 (a cut one may
/// end in the middle of a character), hex otherwise.
fn shown_text(bytes: &[u8], length: u64) -> Shown {
    let cut = (bytes.len() as u64) < length;
    let text = match std::str::from_utf8(bytes) {
        Ok(text) => Some(text),
        // A cut that splits the last character still reads as text.
        Err(error) if cut && error.error_len().is_none() => {
            std::str::from_utf8(&bytes[..error.valid_up_to()]).ok()
        }
        Err(_) => None,
    };
    match text {
        Some(text) if !cut && looks_like_json(text) => {
            match serde_json::from_str::<serde_json::Value>(text) {
                Ok(value) => Shown::Json(
                    serde_json::to_string_pretty(&value).unwrap_or_else(|_| text.to_owned()),
                ),
                Err(_) => Shown::Text(text.to_owned()),
            }
        }
        Some(text) if !text.contains('\0') => Shown::Text(text.to_owned()),
        _ => Shown::Binary {
            hex: hex(bytes),
            length,
        },
    }
}

fn looks_like_json(text: &str) -> bool {
    let text = text.trim_start();
    text.starts_with('{') || text.starts_with('[')
}

/// Bytes as hex, sixteen to a line, in pairs.
fn hex(bytes: &[u8]) -> String {
    bytes
        .chunks(16)
        .map(|line| {
            line.iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<Vec<_>>()
                .join(" ")
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// A Redis connection's window.
#[derive(Debug)]
pub struct Browser {
    /// The database the window is on: the connection's first, then the
    /// one switched to.
    pub database: u32,
    /// How the keys are browsed, as the connection was saved.
    pub browsing: Browsing,
    /// Whether the connection is production's: scans wait between steps.
    pub production: bool,
    /// The server, its databases and their keys.
    pub probe: Fetch<Probe>,
    /// The pattern typed in the filter field; Enter scans with it.
    pub filter: String,
    /// Put the keyboard in the filter on the next frame.
    pub focus_filter: bool,
    pub tree: KeyTree,
    pub scan: Scan,
    /// The groups open, by their prefixes.
    pub open: HashSet<Vec<u8>>,
    /// The keys shown of each group, past the first page.
    pub shown: HashMap<Vec<u8>, usize>,
    /// What `TYPE` said of the keys asked for, and the ones on their way.
    pub types: HashMap<Key, KeyType>,
    pub asking: HashSet<Key>,
    /// The tree's keyboard cursor.
    pub cursor: Option<Place>,
    /// Scroll the cursor's row into view on the next frame.
    pub reveal: bool,
    /// The keys open in tabs, in strip order, and the one in front.
    pub tabs: Vec<KeyTab>,
    pub active: Option<usize>,
    /// The database picker is open.
    pub picking_database: bool,
    /// Scan all asked: the warning shows until it is answered.
    pub confirm_scan_all: bool,
}

/// What Tabletist holds for a name beside its own bytes: its box and its
/// place in its group's tree. Measured by `memory_per_hundred_thousand_keys`
/// (resident memory): 41 bytes for names in one group, 62 for names in a
/// thousand. The larger, so Scan all's warning does not say too little.
pub const BYTES_PER_KEY: u64 = 64;

/// The label of database `number` where its keys are known: its number
/// and its keys ("0 · 1,204 keys", "2 · empty"), as the database menus
/// list them.
pub fn database_label(number: u32, keys: u64) -> String {
    match keys {
        0 => format!("{number} · empty"),
        1 => format!("{number} · 1 key"),
        keys => format!("{number} · {} keys", crate::format::group_digits(keys)),
    }
}

/// Why a Test of a Redis server failed, in words, with the server's own
/// code in brackets where it gave one.
pub fn test_failure(
    error: &Error,
    spec: &tabletist_db::ConnectSpec,
    locale: crate::i18n::Locale,
) -> String {
    use tabletist_db::redis::{CLOSED_IN_HANDSHAKE, CONNECT_TIMEOUT, REFUSED, code};
    let say = |text: &'static str| crate::i18n::gettext(locale, text).into_owned();
    let at = format!("{}:{}", spec.host, spec.port);
    match (error, code(error)) {
        (Error::Auth(_), Some("WRONGPASS")) => say("Wrong username or password (WRONGPASS)"),
        (Error::Auth(_), Some("NOAUTH")) => say("This server needs a password (NOAUTH)"),
        (Error::Connect(said), _) if said == CLOSED_IN_HANDSHAKE => {
            say("The server closed the connection. It may need TLS: turn on Use TLS.")
        }
        (Error::Connect(said), _) if said == REFUSED => {
            say("Nothing is listening on {at} (connection refused)").replace("{at}", &at)
        }
        (Error::Timeout, _) => say("No answer from {at} after {secs} s")
            .replace("{at}", &at)
            .replace("{secs}", &CONNECT_TIMEOUT.as_secs().to_string()),
        // The server's words already start with its code.
        (Error::Query { message, .. }, Some(code)) if !message.starts_with(code) => {
            format!("{message} ({code})")
        }
        (other, _) => other.to_string(),
    }
}

/// What a passed Test of a Redis server says: the server, its database
/// and keys, and how long it took. `Err` with the warning where the user
/// can't list keys.
pub fn test_passed(
    probe: &Probe,
    took: Option<Duration>,
    locale: crate::i18n::Locale,
) -> Result<String, String> {
    let say = |text: &'static str| crate::i18n::gettext(locale, text).into_owned();
    if let Some(command) = &probe.refused {
        let _ = command;
        return Err(say(
            "Connected, but this user can't run INFO or SCAN, so keys can't be listed (NOPERM)",
        ));
    }
    let mut server = probe.server.label();
    if probe.server.compatible() {
        server = format!("{server}, {}", say("Redis-compatible"));
    }
    let keys = probe.keys.unwrap_or_else(|| probe.keys_in(probe.database));
    let keys = match keys {
        0 => say("empty"),
        1 => say("1 key"),
        keys => format!("{} {}", crate::format::group_digits(keys), say("keys")),
    };
    let mut text = format!(
        "{} · {server} · db {} · {keys}",
        say("Connected"),
        probe.database
    );
    if let Some(took) = took {
        text = format!("{text} · {}", crate::format::elapsed(took));
    }
    Ok(text)
}

/// What a Redis window's views and keys ask of it.
#[derive(Debug, Clone, PartialEq)]
pub enum RedisAction {
    /// Opens or closes a group, by its prefix.
    ToggleGroup(Vec<u8>),
    /// Shows the next page of a group's keys.
    More(Vec<u8>),
    /// Opens a key in a tab.
    OpenKey(Key),
    SelectTab(usize),
    CloseTab(usize),
    /// The keys on screen whose types are not known yet.
    Visible(Vec<Key>),
    /// Scans again with the filter's pattern (Enter in the filter).
    Filter,
    /// Scans again from the start (Reload).
    Reload,
    Stop,
    /// Scan all: asks first.
    ScanAll,
    /// The warning's answer.
    ConfirmScanAll(bool),
    /// Moves the window to another database.
    SwitchDatabase(u32),
    /// The next page of the key in front.
    LoadMore,
    CopyValue,
    CopyKey,
    /// Selects a cell of the value's table, by row and column.
    SelectCell(usize, usize),
    /// Copies the selected cell.
    CopyCell,
    /// The keyboard: the cursor set, moved, stepped into or out of a
    /// group, and what it is on opened.
    Cursor(Place),
    Move(isize),
    Step(bool),
    Enter,
    FoldGroup,
    FocusFilter,
}

/// A window that goes stops its scan: the task holds the connection.
impl Drop for Browser {
    fn drop(&mut self) {
        self.scan.stop.stop();
    }
}

impl Browser {
    pub fn new(database: u32, browsing: Browsing, production: bool) -> Self {
        let filter = match browsing.pattern.as_str() {
            "*" => String::new(),
            pattern => pattern.to_owned(),
        };
        Self {
            database,
            tree: KeyTree::new(&browsing.separator),
            scan: Scan {
                pattern: browsing.pattern.clone(),
                ..Scan::default()
            },
            browsing,
            production,
            probe: Fetch::default(),
            filter,
            focus_filter: false,
            open: HashSet::new(),
            shown: HashMap::new(),
            types: HashMap::new(),
            asking: HashSet::new(),
            cursor: None,
            reveal: false,
            tabs: Vec::new(),
            active: None,
            picking_database: false,
            confirm_scan_all: false,
        }
    }

    /// The page of keys a group shows, and the SCAN COUNT hint.
    pub fn page(&self) -> usize {
        self.browsing.page.max(1) as usize
    }

    /// The rows of the tree as they stand.
    pub fn rows(&self) -> Vec<Row> {
        self.tree.rows(&self.open, &self.shown, self.page())
    }

    /// Starts a scan of `pattern` from the beginning: the names held so
    /// far go, and so do the types read for them.
    pub fn restart(&mut self, request: RequestId, pattern: &str) {
        self.scan.stop.stop();
        self.tree = KeyTree::new(&self.browsing.separator);
        self.types.clear();
        self.asking.clear();
        self.shown.clear();
        self.scan = Scan {
            state: ScanState::Running { request },
            pattern: pattern.to_owned(),
            ..Scan::default()
        };
    }

    /// Goes on from where the scan stopped, with no limit (Scan all).
    pub fn resume(&mut self, request: RequestId) {
        self.scan.stop = StopFlag::new();
        self.scan.unlimited = true;
        self.scan.state = ScanState::Running { request };
    }

    /// The most names the scan holds before it stops: none after Scan all.
    pub fn limit(&self) -> Option<u64> {
        (!self.scan.unlimited).then_some(self.browsing.key_limit)
    }

    /// Takes a step of the scan `request`: false when it is not the
    /// running scan's (an old one's, which is dropped).
    pub fn take_step(&mut self, request: RequestId, step: Result<ScanStep, Error>) -> bool {
        if self.scan.state != (ScanState::Running { request }) {
            return false;
        }
        let step = match step {
            Ok(step) => step,
            Err(error) => {
                self.scan.state = ScanState::Failed(error);
                return true;
            }
        };
        if step.total.is_some() {
            self.scan.total = step.total;
        }
        for name in &step.names {
            self.tree.insert(name);
        }
        self.scan.steps += 1;
        self.scan.took += step.took;
        self.scan.cursor = step.cursor;
        self.scan.state = match step.end {
            ScanEnd::More => ScanState::Running { request },
            ScanEnd::Done => ScanState::Done,
            ScanEnd::Limited => ScanState::Limited,
            ScanEnd::Stopped => ScanState::Stopped,
        };
        true
    }

    /// Stops the running scan: what it held stays.
    pub fn stop(&mut self) {
        if self.scan.running() {
            self.scan.stop.stop();
            self.scan.state = ScanState::Stopped;
        }
    }

    /// The keys of `rows` whose types are neither known nor asked for:
    /// marked as asked, for the caller to ask.
    pub fn types_to_ask(&mut self, keys: impl IntoIterator<Item = Key>) -> Vec<Key> {
        keys.into_iter()
            .filter(|key| !self.types.contains_key(key) && self.asking.insert(key.clone()))
            .collect()
    }

    /// Takes the types read for `keys`: on an error they may be asked
    /// again.
    pub fn take_types(&mut self, keys: Vec<Vec<u8>>, result: Result<Vec<KeyType>, Error>) {
        match result {
            Ok(types) => {
                for (key, kind) in keys.into_iter().zip(types) {
                    let key: Key = key.into();
                    self.asking.remove(&key);
                    self.types.insert(key, kind);
                }
            }
            Err(_) => {
                for key in keys {
                    self.asking.remove(key.as_slice());
                }
            }
        }
    }

    /// The key open in front.
    pub fn active_tab(&self) -> Option<&KeyTab> {
        self.tabs.get(self.active?)
    }

    pub fn active_tab_mut(&mut self) -> Option<&mut KeyTab> {
        let active = self.active?;
        self.tabs.get_mut(active)
    }

    /// The tab of `key`, where it is open.
    pub fn tab_of(&mut self, key: &[u8]) -> Option<&mut KeyTab> {
        self.tabs.iter_mut().find(|tab| *tab.key == *key)
    }

    /// Opens `key` in front, in a tab of its own or the one it has:
    /// whether it needs reading.
    pub fn open_key(&mut self, key: &[u8]) -> bool {
        if let Some(at) = self.tabs.iter().position(|tab| *tab.key == *key) {
            self.active = Some(at);
            return false;
        }
        self.tabs.push(KeyTab::new(Box::from(key)));
        self.active = Some(self.tabs.len() - 1);
        true
    }

    pub fn close_tab(&mut self, at: usize) {
        if at >= self.tabs.len() {
            return;
        }
        self.tabs.remove(at);
        self.active = match self.active {
            _ if self.tabs.is_empty() => None,
            Some(active) if active > at => Some(active - 1),
            Some(active) => Some(active.min(self.tabs.len() - 1)),
            None => None,
        };
    }

    /// Opens a group that is closed, closes one that is open.
    pub fn toggle(&mut self, prefix: &[u8]) {
        if !self.open.remove(prefix) {
            self.open.insert(prefix.to_vec());
        }
    }

    /// Shows the next page of a group's keys.
    pub fn more(&mut self, prefix: &[u8]) {
        let page = self.page();
        let shown = self.shown.entry(prefix.to_vec()).or_insert(page);
        *shown += page;
    }

    /// The keys the database holds, as the window knows: the probe's
    /// count for it, else the scan's `DBSIZE`.
    pub fn keys(&self) -> Option<u64> {
        self.scan.total.or_else(|| {
            let probe = self.probe.value.as_ref()?;
            Some(probe.keys_in(self.database))
        })
    }

    /// About how much Tabletist would hold for `keys` names, by the names
    /// held so far.
    pub fn bytes_for(&self, keys: u64) -> u64 {
        let held = self.tree.len().max(1) as u64;
        let names: u64 = self
            .tree
            .names()
            .iter()
            .take(1000)
            .map(|name| name.len() as u64)
            .sum();
        let sampled = held.min(1000);
        let average = names / sampled.max(1);
        keys * (average + BYTES_PER_KEY)
    }

    /// Moves the cursor `by` rows (`isize::MIN` and `MAX` to the ends).
    pub fn move_cursor(&mut self, by: isize) {
        let rows = self.rows();
        if rows.is_empty() {
            return;
        }
        let at = self
            .cursor
            .as_ref()
            .and_then(|place| rows.iter().position(|row| row.place() == *place));
        let last = rows.len() - 1;
        let to = match (at, by) {
            (_, isize::MIN) => 0,
            (_, isize::MAX) => last,
            (None, _) => 0,
            (Some(at), by) => at.saturating_add_signed(by).min(last),
        };
        self.cursor = Some(rows[to].place());
        self.reveal = true;
    }

    /// Right on a closed group opens it, on an open one steps into it;
    /// left on an open group closes it, elsewhere climbs to the group.
    pub fn step(&mut self, right: bool) {
        let rows = self.rows();
        let Some(at) = self
            .cursor
            .as_ref()
            .and_then(|place| rows.iter().position(|row| row.place() == *place))
        else {
            self.move_cursor(0);
            return;
        };
        match (&rows[at], right) {
            (
                Row::Group {
                    prefix,
                    open: false,
                    ..
                },
                true,
            ) => {
                let prefix = prefix.clone();
                self.toggle(&prefix);
            }
            (Row::Group { open: true, .. }, true) => self.move_cursor(1),
            (
                Row::Group {
                    prefix, open: true, ..
                },
                false,
            ) => {
                let prefix = prefix.clone();
                self.toggle(&prefix);
            }
            (row, false) => {
                let depth = row.depth();
                if depth == 0 {
                    return;
                }
                if let Some(parent) = rows[..at]
                    .iter()
                    .rev()
                    .find(|row| matches!(row, Row::Group { .. }) && row.depth() < depth)
                {
                    self.cursor = Some(parent.place());
                    self.reveal = true;
                }
            }
            (Row::Key { .. } | Row::More { .. }, true) => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tree(names: &[&str]) -> KeyTree {
        let mut tree = KeyTree::new(":");
        for name in names {
            tree.insert(name.as_bytes());
        }
        tree
    }

    fn shown(rows: &[Row]) -> Vec<String> {
        rows.iter()
            .map(|row| match row {
                Row::Group {
                    name,
                    count,
                    open,
                    depth,
                    ..
                } => format!(
                    "{}{} {count}{}",
                    "  ".repeat(*depth),
                    lossy(name),
                    if *open { " ▾" } else { "" }
                ),
                Row::Key { name, depth, .. } => format!("{}{}", "  ".repeat(*depth), lossy(name)),
                Row::More { left, depth, .. } => format!("{}{left} more", "  ".repeat(*depth)),
            })
            .collect()
    }

    #[test]
    fn names_group_by_the_separator_and_nest() {
        let tree = tree(&[
            "book:1",
            "book:2",
            "book:reviews:12",
            "book:reviews:13",
            "cart:1",
            "cart:2",
            "config",
            "queue:emails",
        ]);
        assert_eq!(tree.len(), 8);
        let mut open = HashSet::new();
        open.insert(b"book:".to_vec());
        open.insert(b"book:reviews:".to_vec());
        let rows = tree.rows(&open, &HashMap::new(), 1000);
        assert_eq!(
            shown(&rows),
            vec![
                "book: 4 ▾",
                "  reviews: 2 ▾",
                "    12",
                "    13",
                "  1",
                "  2",
                "cart: 2",
                // A group of one key is that key, at the top.
                "config",
                "queue:emails",
            ]
        );
        assert_eq!(tree.group(b"book:").map(Group::count), Some(4));
        assert_eq!(tree.group(b"book:reviews:").map(Group::count), Some(2));
        assert_eq!(
            tree.prefixes(),
            vec![
                (b"book:".to_vec(), 4),
                (b"book:reviews:".to_vec(), 2),
                (b"cart:".to_vec(), 2)
            ]
        );
    }

    /// The process's resident memory, in KiB, as `ps` reads it.
    fn resident() -> Option<u64> {
        let out = std::process::Command::new("ps")
            .args(["-o", "rss=", "-p", &std::process::id().to_string()])
            .output()
            .ok()?;
        String::from_utf8_lossy(&out.stdout).trim().parse().ok()
    }

    /// What 100,000 names held cost, for the PR's figure: run alone, with
    /// `--ignored --nocapture`. A guess, by the process's resident memory.
    #[test]
    #[ignore = "measures memory; run alone with --ignored --nocapture"]
    fn memory_per_hundred_thousand_keys() {
        for (name, make) in [
            (
                "item:<n>",
                (|n: u32| format!("item:{n}")) as fn(u32) -> String,
            ),
            ("session:<uuid>", |n: u32| {
                format!("session:{n:08x}-7c3e-4b1a-9f00-{n:012x}")
            }),
            ("book:<id>:reviews:<n>", |n: u32| {
                format!("book:{}:reviews:{n}", n % 1000)
            }),
        ] {
            // One shape a process: memory a tree gave back is not resident
            // memory the next one adds.
            if std::env::var("KEY_SHAPE").is_ok_and(|shape| shape != name) {
                continue;
            }
            let before = resident().unwrap_or(0);
            let mut tree = KeyTree::new(":");
            for n in 0..100_000 {
                tree.insert(make(n).as_bytes());
            }
            let after = resident().unwrap_or(0);
            println!(
                "{name}: {} names, about {} KiB held ({} bytes a name)",
                tree.len(),
                after.saturating_sub(before),
                after.saturating_sub(before) * 1024 / 100_000
            );
            drop(tree);
        }
    }

    #[test]
    fn a_match_is_found_without_regard_to_case_on_the_names_own_bytes() {
        assert_eq!(find_folded("Book:12", "book"), Some(0..4));
        assert_eq!(find_folded("cart:Éa", "éa"), Some(5..8));
        // `İ` lowers to two characters: a match inside them takes it whole,
        // never a slice of its bytes.
        assert_eq!(find_folded("İx", "i"), Some(0..2));
        assert_eq!(find_folded("İx", "x"), Some(2..3));
        // A match that starts inside the expansion takes `İ` whole too.
        assert_eq!(find_folded("İx", "\u{307}x"), Some(0..3));
        assert_eq!(find_folded("aİx", "ix"), None);
        assert_eq!(find_folded("abc", ""), None);
    }

    #[test]
    fn a_names_glob_characters_are_escaped_for_match() {
        assert_eq!(glob_escaped(b"book:"), "book:");
        assert_eq!(glob_escaped(b"a[b]*?\\:"), "a\\[b\\]\\*\\?\\\\:");
    }

    #[test]
    fn keys_are_ordered_as_a_person_counts() {
        let mut names = vec!["book:10", "book:2", "book:1", "book:02", "a", "book:x"];
        names.sort_by(|a, b| natural(a.as_bytes(), b.as_bytes()));
        assert_eq!(
            names,
            vec!["a", "book:1", "book:02", "book:2", "book:10", "book:x"]
        );
    }

    #[test]
    fn a_name_scan_returns_twice_is_held_once() {
        let mut tree = tree(&["session:1", "session:2"]);
        assert!(!tree.insert(b"session:1"));
        assert!(tree.insert(b"session:3"));
        assert_eq!(tree.len(), 3);
        assert_eq!(tree.group(b"session:").map(Group::count), Some(3));
    }

    #[test]
    fn another_separator_groups_by_itself() {
        let mut tree = KeyTree::new("/");
        for name in ["shop/books/1", "shop/books/2", "shop:cart:1", "a/b"] {
            tree.insert(name.as_bytes());
        }
        let rows = tree.rows(&HashSet::new(), &HashMap::new(), 1000);
        assert_eq!(shown(&rows), vec!["shop/ 2", "a/b", "shop:cart:1"]);
        // No separator: one flat list.
        let mut flat = KeyTree::new("");
        flat.insert(b"a:b");
        flat.insert(b"a:c");
        assert_eq!(flat.rows(&HashSet::new(), &HashMap::new(), 9).len(), 2);
    }

    #[test]
    fn a_large_group_shows_a_page_then_more() {
        let names: Vec<String> = (0..25).map(|n| format!("item:{n:02}")).collect();
        let names: Vec<&str> = names.iter().map(String::as_str).collect();
        let tree = tree(&names);
        let mut open = HashSet::new();
        open.insert(b"item:".to_vec());
        let rows = tree.rows(&open, &HashMap::new(), 10);
        assert_eq!(rows.len(), 1 + 10 + 1);
        assert_eq!(
            rows.last(),
            Some(&Row::More {
                prefix: b"item:".to_vec(),
                left: 15,
                depth: 1
            })
        );
        let mut shown = HashMap::new();
        shown.insert(b"item:".to_vec(), 20);
        let rows = tree.rows(&open, &shown, 10);
        assert!(matches!(rows.last(), Some(Row::More { left: 5, .. })));
        // The page shown is the first in order, and in order, though only
        // it is sorted.
        let keys: Vec<String> = rows
            .iter()
            .filter_map(|row| match row {
                Row::Key { name, .. } => Some(lossy(name)),
                _ => None,
            })
            .collect();
        let first: Vec<String> = (0..20).map(|n| format!("{n:02}")).collect();
        assert_eq!(keys, first);
    }

    fn browser() -> Browser {
        Browser::new(0, Browsing::default(), false)
    }

    fn step(names: &[&str], end: ScanEnd) -> ScanStep {
        ScanStep {
            total: Some(10),
            names: names.iter().map(|name| name.as_bytes().to_vec()).collect(),
            cursor: if end == ScanEnd::Done { 0 } else { 7 },
            end,
            took: Duration::from_millis(2),
        }
    }

    #[test]
    fn a_scan_builds_the_tree_as_its_steps_arrive() {
        let mut browser = browser();
        browser.restart(RequestId(1), "*");
        assert!(browser.take_step(RequestId(1), Ok(step(&["book:1", "book:2"], ScanEnd::More))));
        assert!(browser.scan.running());
        assert_eq!(browser.tree.len(), 2);
        // A step of an old scan is dropped.
        assert!(!browser.take_step(RequestId(9), Ok(step(&["x"], ScanEnd::Done))));
        assert!(browser.take_step(RequestId(1), Ok(step(&["book:2", "cart:1"], ScanEnd::Done))));
        assert_eq!(browser.scan.state, ScanState::Done);
        assert_eq!(browser.tree.len(), 3);
        assert_eq!(browser.scan.total, Some(10));
        assert_eq!(browser.scan.steps, 2);
        // A new pattern starts again, with nothing held.
        browser.restart(RequestId(2), "book:*");
        assert!(browser.tree.is_empty());
        assert_eq!(browser.scan.pattern, "book:*");
    }

    #[test]
    fn the_limit_stops_and_scan_all_goes_on() {
        let mut browser = browser();
        assert_eq!(browser.limit(), Some(100_000));
        browser.restart(RequestId(1), "*");
        browser.take_step(RequestId(1), Ok(step(&["a"], ScanEnd::Limited)));
        assert_eq!(browser.scan.state, ScanState::Limited);
        assert_eq!(browser.scan.cursor, 7);
        browser.resume(RequestId(2));
        assert_eq!(browser.limit(), None);
        browser.take_step(RequestId(2), Ok(step(&["b"], ScanEnd::Done)));
        assert_eq!(browser.tree.len(), 2);
        assert!(browser.scan.complete());
    }

    #[test]
    fn stop_keeps_what_was_held() {
        let mut browser = browser();
        browser.restart(RequestId(1), "*");
        browser.take_step(RequestId(1), Ok(step(&["a", "b"], ScanEnd::More)));
        let flag = browser.scan.stop.clone();
        browser.stop();
        assert!(flag.is_stopped());
        assert_eq!(browser.scan.state, ScanState::Stopped);
        assert_eq!(browser.tree.len(), 2);
        // What the task still sends is not taken.
        assert!(!browser.take_step(RequestId(1), Ok(step(&["c"], ScanEnd::More))));
    }

    #[test]
    fn types_are_asked_once_for_each_key() {
        let mut browser = browser();
        let keys: Vec<Key> = vec![Box::from(&b"a"[..]), Box::from(&b"b"[..])];
        assert_eq!(browser.types_to_ask(keys.clone()).len(), 2);
        assert!(browser.types_to_ask(keys.clone()).is_empty());
        browser.take_types(vec![b"a".to_vec()], Ok(vec![KeyType::Hash]));
        assert_eq!(browser.types.get(&keys[0]), Some(&KeyType::Hash));
        // A failed read may be asked again.
        browser.take_types(vec![b"b".to_vec()], Err(Error::Timeout));
        assert_eq!(browser.types_to_ask(keys).len(), 1);
    }

    #[test]
    fn the_cursor_moves_opens_and_climbs() {
        let mut browser = browser();
        for name in ["book:1", "book:2", "config"] {
            browser.tree.insert(name.as_bytes());
        }
        browser.move_cursor(1);
        assert_eq!(browser.cursor, Some(Place::Group(b"book:".to_vec())));
        browser.step(true);
        assert!(browser.open.contains(b"book:".as_slice()));
        browser.step(true);
        assert_eq!(browser.cursor, Some(Place::Key(Box::from(&b"book:1"[..]))));
        browser.step(false);
        assert_eq!(browser.cursor, Some(Place::Group(b"book:".to_vec())));
        browser.move_cursor(isize::MAX);
        assert_eq!(browser.cursor, Some(Place::Key(Box::from(&b"config"[..]))));
    }

    fn opened(kind: KeyType, page: Page) -> KeyTab {
        let mut tab = KeyTab::new(Box::from(&b"k"[..]));
        tab.info.value = Some(KeyInfo {
            kind,
            ttl_ms: None,
            memory: None,
            encoding: None,
            length: None,
        });
        tab.take(Loaded {
            page,
            next: None,
            command: "X",
            took: Duration::ZERO,
        });
        tab
    }

    #[test]
    fn strings_show_as_text_json_or_hex() {
        let tab = opened(
            KeyType::String,
            Page::Text {
                bytes: br#"{"currency":"EUR"}"#.to_vec(),
                length: 18,
            },
        );
        assert_eq!(
            tab.rows.text,
            Some(Shown::Json("{\n  \"currency\": \"EUR\"\n}".into()))
        );
        let tab = opened(
            KeyType::String,
            Page::Text {
                bytes: vec![0, 159, 146, 150],
                length: 4,
            },
        );
        assert_eq!(
            tab.rows.text,
            Some(Shown::Binary {
                hex: "00 9f 92 96".into(),
                length: 4
            })
        );
        let tab = opened(
            KeyType::String,
            Page::Text {
                bytes: "é".repeat(3).into_bytes()[..5].to_vec(),
                length: 1000,
            },
        );
        assert_eq!(tab.rows.text, Some(Shown::Text("éé".into())));
        assert_eq!(tab.text_length, Some((5, 1000)));
    }

    #[test]
    fn copy_value_copies_a_hash_as_json() {
        let tab = opened(
            KeyType::Hash,
            Page::Pairs(vec![
                (b"title".to_vec(), b"Lanterns".to_vec()),
                (b"title".to_vec(), b"twice".to_vec()),
                (b"price".to_vec(), b"12.99".to_vec()),
            ]),
        );
        assert_eq!(tab.rows.cells.len(), 2);
        let copied: serde_json::Value = serde_json::from_str(&tab.copied().unwrap()).unwrap();
        assert_eq!(
            copied,
            serde_json::json!({ "title": "Lanterns", "price": "12.99" })
        );
        let tab = opened(
            KeyType::ZSet,
            Page::Scored(vec![(b"book:9".to_vec(), "3".into())]),
        );
        assert_eq!(tab.rows.columns, vec!["Rank", "Score", "Member"]);
        let copied: serde_json::Value = serde_json::from_str(&tab.copied().unwrap()).unwrap();
        assert_eq!(
            copied,
            serde_json::json!([{ "member": "book:9", "score": 3.0 }])
        );
    }

    #[test]
    fn a_list_numbers_its_items_across_pages() {
        let mut tab = opened(
            KeyType::List,
            Page::Items(vec![b"a".to_vec(), b"b".to_vec()]),
        );
        tab.take(Loaded {
            page: Page::Items(vec![b"c".to_vec()]),
            next: None,
            command: "LRANGE",
            took: Duration::ZERO,
        });
        assert_eq!(tab.rows.cells[2], vec![b"2".to_vec(), b"c".to_vec()]);
    }
}

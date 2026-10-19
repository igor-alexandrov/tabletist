//! What was open when the app last quit, and each connection's tables as
//! they were when its window last closed: `session.json`, in the state
//! directory. Of this computer, and no setting: what the next launch and
//! the next open of a connection make of it is the settings' (`startup`).

use std::collections::BTreeMap;
use std::path::Path;

use serde::{Deserialize, Serialize};
use tabletist_db::{FilterOp, ObjectKind, ObjectRef, Sort, SortDir};

use crate::connections::ConnectionId;
use crate::model::{FilterBar, FilterRow, ObjectTab, TabId, Workspace};

/// The version this one writes.
const VERSION: u32 = 1;

/// The file, as the app holds it.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Session {
    /// The connection tabs open at quit, in the order the bar had them:
    /// each a saved connection's id, or `None` for the picker.
    pub open: Vec<Option<ConnectionId>>,
    /// Which of `open` was in front.
    pub active: usize,
    /// Each saved connection's table tabs as its window last had them, by
    /// its id. Changed through [`Session::keep`] and [`Session::forget`].
    pub connections: BTreeMap<String, Kept>,
    /// The number of the newest version that wrote the file, where newer
    /// than this one (0 where not), written back: a newer reader that
    /// branches on it would misread what is kept for it under an older
    /// number.
    version: u32,
    /// The connections' parts this version could not read, by id, as the
    /// file had them: a newer version may have written them, and they are
    /// written back until the connection's window closes again here.
    unread: BTreeMap<String, serde_json::Value>,
    /// The file's keys this version does not know, written back as they
    /// were.
    rest: BTreeMap<String, serde_json::Value>,
}

/// One connection's window as it was when it closed: its tables, and
/// whether its row panel was shown.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Kept {
    /// The database the tables are of ([`database_of`]): the one the
    /// window had switched to, where it had.
    pub database: String,
    pub tabs: Vec<KeptTab>,
    /// Which of `tabs` was in front. `None` where none was: a SQL editor,
    /// or the Agents tab.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active: Option<usize>,
    /// Whether the row panel was shown. Left out where it was, as a file
    /// from before it was kept leaves it: a window opens with it.
    #[serde(skip_serializing_if = "is_on")]
    pub row_panel: bool,
    /// What a newer version wrote of the connection that this one does
    /// not know: written back until the connection's window closes here.
    #[serde(flatten)]
    pub rest: BTreeMap<String, serde_json::Value>,
}

impl Default for Kept {
    /// A window as it opens: no tables, and the row panel shown.
    fn default() -> Self {
        Self {
            database: String::new(),
            tabs: Vec::new(),
            active: None,
            row_panel: true,
            rest: BTreeMap::new(),
        }
    }
}

/// A table or a view, with its sort and its filter bar.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KeptTab {
    pub schema: String,
    pub name: String,
    /// `table`, `view` or `materialized_view`.
    pub kind: String,
    /// Each column the rows are sorted by, first to last, with `asc` or
    /// `desc`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sort: Vec<(String, String)>,
    #[serde(default, skip_serializing_if = "KeptFilter::is_empty")]
    pub filter: KeptFilter,
    /// A preview tab is not pinned: the next single click replaces it.
    #[serde(default = "on", skip_serializing_if = "is_on")]
    pub pinned: bool,
    /// What a newer version wrote of the tab that this one does not know.
    #[serde(flatten)]
    pub rest: BTreeMap<String, serde_json::Value>,
}

/// A filter bar as it was: its rows, and its WHERE text.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct KeptFilter {
    #[serde(skip_serializing_if = "is_off")]
    pub open: bool,
    /// Each row: its column, its operator and its value.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub rows: Vec<(String, String, String)>,
    /// Whether the WHERE text is used.
    #[serde(skip_serializing_if = "is_off")]
    pub raw: bool,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub raw_text: String,
    /// What a newer version wrote of the bar that this one does not know.
    #[serde(flatten)]
    pub rest: BTreeMap<String, serde_json::Value>,
}

fn on() -> bool {
    true
}

fn is_on(on: &bool) -> bool {
    *on
}

fn is_off(on: &bool) -> bool {
    !*on
}

impl KeptFilter {
    fn is_empty(&self) -> bool {
        *self == Self::default()
    }
}

/// The database a connection's tables are of: its name, or a SQLite
/// file's path, which names no database of its own.
pub fn database_of(spec: &tabletist_db::ConnectSpec) -> String {
    match spec.driver {
        tabletist_db::Driver::Sqlite => spec
            .sqlite_path
            .as_ref()
            .map(|path| path.display().to_string())
            .unwrap_or_default(),
        tabletist_db::Driver::Postgres | tabletist_db::Driver::MySql => spec.database.clone(),
    }
}

/// The file as it is written and read, each part on its own: a value of
/// the wrong kind costs itself, not the file.
#[derive(Default, Serialize, Deserialize)]
#[serde(default)]
struct File {
    version: serde_json::Value,
    open: Vec<serde_json::Value>,
    active: serde_json::Value,
    connections: BTreeMap<String, serde_json::Value>,
    #[serde(flatten)]
    rest: BTreeMap<String, serde_json::Value>,
}

impl Session {
    /// The file at `path`. A missing file is no session; one that cannot be
    /// read is kept aside (`util::load_json`), and there is none either.
    pub fn load(path: &Path) -> Self {
        let file: File = crate::util::load_json(path);
        let mut session = Self::default();
        for (id, kept) in file.connections {
            match serde_json::from_value(kept.clone()) {
                Ok(kept) => drop(session.connections.insert(id, kept)),
                Err(error) => {
                    // Not the error's text: it can quote a stored filter value.
                    log::warn!("the tabs of {id} were not read: {:?}", error.classify());
                    session.unread.insert(id, kept);
                }
            }
        }
        // A window that is neither an id nor the picker is left out.
        let mut active = None;
        let wanted = file.active.as_u64();
        for (at, entry) in file.open.into_iter().enumerate() {
            let window = match entry {
                serde_json::Value::String(id) => Some(ConnectionId(id)),
                serde_json::Value::Null => None,
                _ => continue,
            };
            if wanted == Some(at as u64) {
                active = Some(session.open.len());
            }
            session.open.push(window);
        }
        // Counted against the windows read: one that pointed at a window
        // left out, or past the end, is the first.
        session.active = active.unwrap_or(0);
        // Kept only where newer than this one's, so that what is read
        // back of a file this version wrote is the session written.
        session.version = file
            .version
            .as_u64()
            .and_then(|number| u32::try_from(number).ok())
            .filter(|number| *number > VERSION)
            .unwrap_or(0);
        session.rest = file.rest;
        session
    }

    /// Keeps `kept` as the tables of the connection `id`, in place of
    /// what the file had of it.
    pub fn keep(&mut self, id: &ConnectionId, kept: Kept) {
        self.unread.remove(&id.0);
        self.connections.insert(id.0.clone(), kept);
    }

    /// Keeps whether the row panel of the connection `id`'s window is
    /// shown, and leaves its tables as they are kept: for a window whose
    /// tables are not kept (switched to another database). A part a
    /// newer version wrote stays as it is.
    pub fn keep_row_panel(&mut self, id: &ConnectionId, database: String, shown: bool) {
        if self.unread.contains_key(&id.0) {
            return;
        }
        self.connections
            .entry(id.0.clone())
            .or_insert_with(|| Kept {
                database,
                ..Kept::default()
            })
            .row_panel = shown;
    }

    /// Forgets what is kept of the connection `id`. Whether there was
    /// anything.
    pub fn forget(&mut self, id: &ConnectionId) -> bool {
        let unread = self.unread.remove(&id.0).is_some();
        self.connections.remove(&id.0).is_some() || unread
    }

    /// Keeps what is kept, read or not, of the connections `keep` says yes
    /// to by id, and forgets the rest.
    pub fn retain(&mut self, keep: impl Fn(&str) -> bool) {
        self.connections.retain(|id, _| keep(id));
        self.unread.retain(|id, _| keep(id));
    }

    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        let mut connections = self.unread.clone();
        connections.extend(
            self.connections
                .iter()
                .filter_map(|(id, kept)| Some((id.clone(), serde_json::to_value(kept).ok()?))),
        );
        let file = File {
            version: VERSION.max(self.version).into(),
            open: self
                .open
                .iter()
                .map(|id| {
                    id.as_ref()
                        .map_or(serde_json::Value::Null, |id| id.0.clone().into())
                })
                .collect(),
            active: self.active.into(),
            connections,
            rest: self.rest.clone(),
        };
        crate::util::save_json(path, &file)
    }
}

impl Kept {
    /// The table tabs of `workspace`, in strip order, and which was in
    /// front. SQL editors and the Agents tab are not kept.
    pub fn of(workspace: &Workspace) -> Self {
        let objects: Vec<&ObjectTab> = workspace.object_tabs().collect();
        Self {
            database: database_of(&workspace.spec),
            tabs: objects.iter().map(|object| KeptTab::of(object)).collect(),
            active: objects
                .iter()
                .position(|object| Some(object.id) == workspace.active_tab),
            row_panel: workspace.row_panel,
            rest: BTreeMap::new(),
        }
    }
}

impl KeptTab {
    fn of(object: &ObjectTab) -> Self {
        let bar = &object.filter;
        Self {
            schema: object.object.schema.clone(),
            name: object.object.name.clone(),
            kind: kind_name(object.kind).to_owned(),
            sort: object
                .query
                .sort
                .iter()
                .map(|sort| (sort.column.clone(), dir_name(sort.dir).to_owned()))
                .collect(),
            filter: KeptFilter {
                open: bar.open,
                rows: bar
                    .rows
                    .iter()
                    .map(|row| {
                        let op = op_name(row.op).to_owned();
                        (row.column.clone(), op, row.value.clone())
                    })
                    .collect(),
                raw: bar.raw,
                raw_text: bar.raw_text.clone(),
                ..KeptFilter::default()
            },
            pinned: object.pinned,
            rest: BTreeMap::new(),
        }
    }

    /// The tab this one was, as `id`, paging by `page_size`: its sort, and
    /// its filter bar with what the bar says applied. Nothing is loaded.
    /// `None` for a kind this version does not know. A sort or a row whose
    /// words it does not know is left out.
    pub fn reopen(&self, id: TabId, page_size: u32) -> Option<ObjectTab> {
        let kind = kind_of(&self.kind)?;
        let object = ObjectRef::new(self.schema.clone(), self.name.clone());
        let mut tab = ObjectTab::new(id, object, kind, self.pinned, page_size, None);
        tab.query.sort = self
            .sort
            .iter()
            .filter_map(|(column, dir)| {
                Some(Sort {
                    column: column.clone(),
                    dir: dir_of(dir)?,
                })
            })
            .collect();
        let filter = &self.filter;
        tab.filter = FilterBar {
            open: filter.open,
            rows: filter
                .rows
                .iter()
                .filter_map(|(column, op, value)| {
                    Some(FilterRow {
                        column: column.clone(),
                        op: op_of(op)?,
                        value: value.clone(),
                    })
                })
                .collect(),
            raw: filter.raw,
            raw_text: filter.raw_text.clone(),
            ..FilterBar::default()
        };
        (tab.query.filters, tab.query.raw_where) = tab.filter.to_query();
        tab.reopened = true;
        Some(tab)
    }
}

fn kind_name(kind: ObjectKind) -> &'static str {
    match kind {
        ObjectKind::Table => "table",
        ObjectKind::View => "view",
        ObjectKind::MaterializedView => "materialized_view",
    }
}

fn kind_of(name: &str) -> Option<ObjectKind> {
    [
        ObjectKind::Table,
        ObjectKind::View,
        ObjectKind::MaterializedView,
    ]
    .into_iter()
    .find(|kind| kind_name(*kind) == name)
}

fn dir_name(dir: SortDir) -> &'static str {
    match dir {
        SortDir::Asc => "asc",
        SortDir::Desc => "desc",
    }
}

fn dir_of(name: &str) -> Option<SortDir> {
    [SortDir::Asc, SortDir::Desc]
        .into_iter()
        .find(|dir| dir_name(*dir) == name)
}

/// An operator as the file writes it: its own word, not what the bar
/// shows, which can be translated.
fn op_name(op: FilterOp) -> &'static str {
    match op {
        FilterOp::Eq => "eq",
        FilterOp::Ne => "ne",
        FilterOp::Lt => "lt",
        FilterOp::Gt => "gt",
        FilterOp::Le => "le",
        FilterOp::Ge => "ge",
        FilterOp::Contains => "contains",
        FilterOp::StartsWith => "starts_with",
        FilterOp::IsNull => "is_null",
        FilterOp::IsNotNull => "is_not_null",
        FilterOp::In => "in",
    }
}

fn op_of(name: &str) -> Option<FilterOp> {
    FilterOp::ALL.into_iter().find(|op| op_name(*op) == name)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn books() -> KeptTab {
        KeptTab {
            schema: "public".into(),
            name: "books".into(),
            kind: "table".into(),
            sort: vec![("title".into(), "asc".into())],
            filter: KeptFilter {
                open: true,
                rows: vec![("format".into(), "eq".into(), "print".into())],
                raw: true,
                raw_text: "price > 10".into(),
                ..KeptFilter::default()
            },
            pinned: true,
            rest: BTreeMap::new(),
        }
    }

    #[test]
    fn a_session_is_written_and_read_back_the_same() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("session.json");
        let bookshop = ConnectionId("6a0c1f0e-5a52-4b7b-9c56-3f0c8f2f7a11".into());
        let session = Session {
            open: vec![Some(bookshop.clone()), None],
            active: 1,
            connections: BTreeMap::from([(
                bookshop.0.clone(),
                Kept {
                    database: "bookshop".into(),
                    tabs: vec![books()],
                    active: Some(0),
                    row_panel: false,
                    rest: BTreeMap::new(),
                },
            )]),
            ..Session::default()
        };
        session.save(&path).unwrap();
        assert_eq!(Session::load(&path), session);
        let text = std::fs::read_to_string(&path).unwrap();
        let json: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert_eq!(json["version"], 1);
        assert_eq!(json["open"][1], serde_json::Value::Null);
        assert_eq!(json["connections"][&bookshop.0]["row_panel"], false);
        let tab = &json["connections"][&bookshop.0]["tabs"][0];
        assert_eq!(tab["sort"], serde_json::json!([["title", "asc"]]));
        assert_eq!(tab["filter"]["rows"][0][1], "eq");
        // What is as a new tab has it is left out.
        assert!(tab.get("pinned").is_none(), "{text}");
    }

    #[test]
    fn a_window_whose_row_panel_was_shown_says_nothing_of_it() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("session.json");
        let mut session = Session::default();
        session.keep(&ConnectionId("a".into()), Kept::default());
        session.save(&path).unwrap();
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(!text.contains("row_panel"), "{text}");
        // A file from before the panel was kept opens with it.
        std::fs::write(
            &path,
            r#"{"version":1,"connections":{"a":{"database":"","tabs":[]}}}"#,
        )
        .unwrap();
        assert!(Session::load(&path).connections["a"].row_panel);
    }

    #[test]
    fn the_row_panel_is_kept_on_its_own_with_the_tables_left_as_they_are() {
        let mut session = Session::default();
        let id = ConnectionId("a".into());
        // Nothing kept yet: an entry of the connection's own database,
        // with no table.
        session.keep_row_panel(&id, "bookshop".into(), false);
        assert_eq!(
            session.connections["a"],
            Kept {
                database: "bookshop".into(),
                row_panel: false,
                ..Kept::default()
            }
        );
        // With tables kept, they stay.
        let kept = Kept {
            database: "bookshop".into(),
            tabs: vec![books()],
            active: Some(0),
            row_panel: false,
            rest: BTreeMap::new(),
        };
        session.keep(&id, kept.clone());
        session.keep_row_panel(&id, "archive".into(), true);
        assert_eq!(
            session.connections["a"],
            Kept {
                row_panel: true,
                ..kept
            }
        );
        // A part a newer version wrote is not touched.
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("session.json");
        let text = serde_json::json!({"version": 2, "connections": {"b": {"tabs": "not a list"}}});
        std::fs::write(&path, text.to_string()).unwrap();
        let mut session = Session::load(&path);
        session.keep_row_panel(&ConnectionId("b".into()), "bookshop".into(), false);
        assert!(session.connections.is_empty());
        assert_eq!(session.unread["b"]["tabs"], "not a list");
    }

    #[test]
    fn no_file_is_no_session_and_a_bad_one_is_kept_aside() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("session.json");
        assert_eq!(Session::load(&path), Session::default());
        std::fs::write(&path, "{ not json").unwrap();
        assert_eq!(Session::load(&path), Session::default());
        assert!(dir.path().join("session.json.bad").exists());
    }

    #[test]
    fn a_connection_whose_part_does_not_read_costs_only_itself() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("session.json");
        std::fs::write(
            &path,
            r#"{"version":1,"open":["a"],"active":0,"connections":{
                "a":{"database":"bookshop","tabs":[{"schema":"public","name":"books","kind":"table"}]},
                "b":{"tabs":"not a list"}}}"#,
        )
        .unwrap();
        let session = Session::load(&path);
        assert_eq!(session.open, vec![Some(ConnectionId("a".into()))]);
        assert_eq!(session.connections.len(), 1);
        assert_eq!(session.connections["a"].tabs[0].name, "books");
    }

    #[test]
    fn a_window_or_a_place_of_the_wrong_kind_costs_only_itself() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("session.json");
        std::fs::write(
            &path,
            r#"{"version":"one","open":[7,"a",null],"active":"first","connections":{
                "a":{"database":"","tabs":[{"schema":"main","name":"books","kind":"table"}]}}}"#,
        )
        .unwrap();
        let session = Session::load(&path);
        assert_eq!(session.open, vec![Some(ConnectionId("a".into())), None]);
        assert_eq!(session.active, 0);
        assert_eq!(session.connections["a"].tabs[0].name, "books");
        // The front window is counted among the ones read.
        std::fs::write(&path, r#"{"open":[7,"a",null],"active":2}"#).unwrap();
        let session = Session::load(&path);
        assert_eq!(session.open, vec![Some(ConnectionId("a".into())), None]);
        assert_eq!(session.active, 1);
        // Past the end is the first.
        std::fs::write(&path, r#"{"open":[7,"a",null],"active":5}"#).unwrap();
        assert_eq!(Session::load(&path).active, 0);
    }

    #[test]
    fn a_part_this_version_cannot_read_is_written_back_until_it_is_kept_again() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("session.json");
        let newer = serde_json::json!({
            "database": "",
            "tabs": [{"schema": "main", "name": "books", "kind": "table",
                      "sort": [{"column": "title", "dir": "asc"}]}]
        });
        let text = serde_json::json!({"version": 2, "connections": {"a": newer.clone()}});
        std::fs::write(&path, text.to_string()).unwrap();
        let mut session = Session::load(&path);
        assert!(session.connections.is_empty());
        session.save(&path).unwrap();
        let written: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(written["connections"]["a"], newer);
        // Its window closed here: what this version kept takes its place.
        session.keep(&ConnectionId("a".into()), Kept::default());
        session.save(&path).unwrap();
        assert_eq!(Session::load(&path).connections["a"], Kept::default());
        // And a connection deleted takes it out.
        let mut session = Session::load(&path);
        assert!(session.forget(&ConnectionId("a".into())));
        assert!(!session.forget(&ConnectionId("a".into())));
    }

    #[test]
    fn retain_forgets_the_connections_not_kept_read_or_not() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("session.json");
        let text = serde_json::json!({
            "version": 2,
            "connections": {
                "a": {"database": "", "tabs": []},
                "b": {"database": "", "tabs": []},
                "c": {"tabs": "not a list"},
                "d": {"tabs": "not a list"},
            }
        });
        std::fs::write(&path, text.to_string()).unwrap();
        let mut session = Session::load(&path);
        session.retain(|id| id == "a" || id == "c");
        assert_eq!(
            session.connections.keys().collect::<Vec<_>>(),
            [&"a".to_owned()]
        );
        assert_eq!(session.unread.keys().collect::<Vec<_>>(), [&"c".to_owned()]);
        // And so is what the file has written.
        session.save(&path).unwrap();
        let written: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        let ids: Vec<&String> = written["connections"].as_object().unwrap().keys().collect();
        assert_eq!(ids, [&"a".to_owned(), &"c".to_owned()]);
    }

    #[test]
    fn keys_a_newer_version_added_are_written_back() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("session.json");
        let text = serde_json::json!({
            "version": 2,
            "windows": {"width": 1280},
            "open": ["a"],
            "connections": {"a": {
                "database": "",
                "pane": "grid",
                "tabs": [{"schema": "main", "name": "books", "kind": "table",
                          "hidden": ["price"], "filter": {"open": true, "split": 2}}]
            }}
        });
        std::fs::write(&path, text.to_string()).unwrap();
        let session = Session::load(&path);
        assert_eq!(session.connections["a"].tabs[0].name, "books");
        session.save(&path).unwrap();
        let written: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(written["windows"]["width"], 1280);
        let kept = &written["connections"]["a"];
        assert_eq!(kept["pane"], "grid");
        assert_eq!(kept["tabs"][0]["hidden"][0], "price");
        assert_eq!(kept["tabs"][0]["filter"]["split"], 2);
        // The number of the newest version that wrote it is kept.
        assert_eq!(written["version"], 2);
    }

    #[test]
    fn a_sqlite_connections_tables_are_of_its_file() {
        let spec = tabletist_db::ConnectSpec::sqlite("/tmp/bookshop.db");
        assert_eq!(database_of(&spec), "/tmp/bookshop.db");
        let (spec, _) =
            tabletist_db::ConnectSpec::from_url("postgres://demo@localhost:5432/bookshop").unwrap();
        assert_eq!(database_of(&spec), "bookshop");
    }

    #[test]
    fn a_tab_reopens_with_its_sort_and_its_filter_applied() {
        let tab = books().reopen(TabId(7), 1_000).unwrap();
        assert_eq!(tab.object, ObjectRef::new("public", "books"));
        assert_eq!(tab.kind, ObjectKind::Table);
        assert_eq!(tab.query.limit, 1_000);
        assert_eq!(
            tab.query.sort,
            vec![Sort {
                column: "title".into(),
                dir: SortDir::Asc
            }]
        );
        assert!(tab.filter.open);
        assert_eq!(tab.query.filters.len(), 1);
        assert_eq!(tab.query.filters[0].op, FilterOp::Eq);
        assert_eq!(tab.query.raw_where.as_deref(), Some("price > 10"));
        assert!(tab.rows.needs_load());
        // Kept again, it is what it was.
        assert_eq!(KeptTab::of(&tab), books());
    }

    #[test]
    fn words_this_version_does_not_know_are_left_out() {
        let mut tab = books();
        tab.sort.push(("price".into(), "sideways".into()));
        tab.filter
            .rows
            .push(("price".into(), "near".into(), "10".into()));
        let reopened = tab.reopen(TabId(7), 1_000).unwrap();
        assert_eq!(reopened.query.sort.len(), 1);
        assert_eq!(reopened.filter.rows.len(), 1);
        tab.kind = "sequence".into();
        assert!(tab.reopen(TabId(7), 1_000).is_none());
    }

    #[test]
    fn every_operator_and_kind_has_its_own_word() {
        for op in FilterOp::ALL {
            assert_eq!(op_of(op_name(op)), Some(op));
        }
        for kind in [
            ObjectKind::Table,
            ObjectKind::View,
            ObjectKind::MaterializedView,
        ] {
            assert_eq!(kind_of(kind_name(kind)), Some(kind));
        }
    }
}

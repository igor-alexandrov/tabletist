//! The column that labels the rows of a table foreign keys point at: what
//! the user chose, kept on this computer, and what the choice comes to
//! against the table's columns. No UI, and nothing here talks to the
//! backend or to a database.

use std::collections::BTreeMap;
use std::path::Path;

use serde::{Deserialize, Serialize};
use tabletist_db::{ColumnClass, ColumnInfo, Dialect, ObjectRef, column_class};

use crate::connections::ConnectionId;

/// What labels a table's rows, as the user left it.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum LabelChoice {
    /// The app picks: see [`automatic`].
    #[default]
    Automatic,
    /// No label: the rows are listed by their keys alone.
    NoLabel,
    /// This column.
    Column(String),
}

impl LabelChoice {
    fn is_automatic(&self) -> bool {
        *self == Self::Automatic
    }
}

/// Who picked what a table's rows are labelled by.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    /// The app.
    Automatic,
    /// The user: a column, or no label.
    User,
    /// Nobody: the table has no column that could be a label.
    KeysOnly,
}

/// What a choice comes to against a table's columns.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Resolved {
    /// The column the rows are labelled and searched by. None: by their
    /// keys alone.
    pub column: Option<String>,
    pub source: Source,
    /// What the app picks for the table, whoever chose.
    pub automatic: Option<String>,
    /// The column that was chosen and that the table no longer has.
    pub gone: Option<String>,
}

/// Why a column cannot label its table's rows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Excluded {
    /// It is the key the rows are found by.
    Key,
    /// It holds bytes.
    Binary,
}

/// Why `column` cannot be a label, where it cannot. `keys` are the columns
/// the rows are found by: the one a foreign key references, or the
/// table's primary key.
pub fn excluded(dialect: Dialect, column: &ColumnInfo, keys: &[&str]) -> Option<Excluded> {
    if keys.contains(&column.name.as_str()) {
        return Some(Excluded::Key);
    }
    match column_class(dialect, &column.type_name) {
        ColumnClass::Binary => Some(Excluded::Binary),
        ColumnClass::Integer { .. }
        | ColumnClass::Decimal { .. }
        | ColumnClass::Float
        | ColumnClass::Text { .. }
        | ColumnClass::Boolean
        | ColumnClass::Json
        | ColumnClass::Other => None,
    }
}

/// The column the app picks to read a row of a table by: its first text
/// column that is not the key.
pub fn automatic(dialect: Dialect, columns: &[ColumnInfo], key: &str) -> Option<String> {
    columns
        .iter()
        .filter(|column| column.name != key)
        .find(|column| {
            matches!(
                column_class(dialect, &column.type_name),
                ColumnClass::Text { .. }
            )
        })
        .map(|column| column.name.clone())
}

/// What `choice` comes to for a table of `columns`, in a picker whose
/// foreign key references `key`. A column that was chosen and is not among
/// them is `gone`, and the app's pick stands in for it. A chosen column
/// that cannot be a label any more (it is this picker's key, or holds
/// bytes now) labels nothing: the choice is still the user's.
pub fn resolve(
    choice: &LabelChoice,
    dialect: Dialect,
    columns: &[ColumnInfo],
    key: &str,
) -> Resolved {
    let automatic = automatic(dialect, columns, key);
    let allowed = |column: &ColumnInfo| excluded(dialect, column, &[key]).is_none();
    let chosen = match choice {
        LabelChoice::Column(name) => Some(name),
        LabelChoice::Automatic | LabelChoice::NoLabel => None,
    };
    let gone = chosen
        .filter(|name| !columns.iter().any(|column| column.name == **name))
        .cloned();
    if !columns.iter().any(allowed) {
        return Resolved {
            column: None,
            source: Source::KeysOnly,
            automatic,
            gone,
        };
    }
    let (column, source) = match choice {
        LabelChoice::Automatic => (automatic.clone(), Source::Automatic),
        LabelChoice::NoLabel => (None, Source::User),
        LabelChoice::Column(_) if gone.is_some() => (automatic.clone(), Source::Automatic),
        LabelChoice::Column(name) => {
            let found = columns.iter().find(|column| column.name == *name);
            (
                found
                    .filter(|column| allowed(column))
                    .map(|column| column.name.clone()),
                Source::User,
            )
        }
    };
    Resolved {
        column,
        source,
        automatic,
        gone,
    }
}

/// The file's format.
pub const VERSION: u32 = 1;

/// What is kept of one table.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default)]
struct Kept {
    #[serde(skip_serializing_if = "LabelChoice::is_automatic")]
    choice: LabelChoice,
    /// A chosen column that was found gone, until the user is told. The
    /// choice went back to Automatic when it was noted.
    #[serde(skip_serializing_if = "Option::is_none")]
    gone: Option<String>,
}

/// Every choice, stored as `labels.json` in the state directory: by the
/// saved connection's id, then by schema, then by table. Nothing of it is
/// ever written to a database.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Labels {
    version: u32,
    connections: BTreeMap<String, BTreeMap<String, BTreeMap<String, Kept>>>,
}

impl Default for Labels {
    fn default() -> Self {
        Self {
            version: VERSION,
            connections: BTreeMap::new(),
        }
    }
}

impl Labels {
    /// The store at `path`. A missing file is an empty store; one that
    /// cannot be read is kept aside (`util::load_json`), and the store
    /// starts empty.
    pub fn load(path: &Path) -> Self {
        crate::util::load_json(path)
    }

    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        crate::util::save_json(path, self)
    }

    fn kept(&self, conn: &ConnectionId, table: &ObjectRef) -> Option<&Kept> {
        self.connections
            .get(&conn.0)?
            .get(&table.schema)?
            .get(&table.name)
    }

    fn kept_mut(&mut self, conn: &ConnectionId, table: &ObjectRef) -> &mut Kept {
        self.connections
            .entry(conn.0.clone())
            .or_default()
            .entry(table.schema.clone())
            .or_default()
            .entry(table.name.clone())
            .or_default()
    }

    /// Drops what is kept of `table` where it says nothing any more, and
    /// the schema and the connection it leaves empty.
    fn tidy(&mut self, conn: &ConnectionId, table: &ObjectRef) {
        let Some(schemas) = self.connections.get_mut(&conn.0) else {
            return;
        };
        if let Some(tables) = schemas.get_mut(&table.schema) {
            if tables.get(&table.name) == Some(&Kept::default()) {
                tables.remove(&table.name);
            }
            if tables.is_empty() {
                schemas.remove(&table.schema);
            }
        }
        if schemas.is_empty() {
            self.connections.remove(&conn.0);
        }
    }

    /// What was chosen for `table` on the saved connection `conn`.
    pub fn choice(&self, conn: &ConnectionId, table: &ObjectRef) -> LabelChoice {
        self.kept(conn, table)
            .map(|kept| kept.choice.clone())
            .unwrap_or_default()
    }

    /// Makes `choice` the choice for `table`. A column that was noted gone
    /// and not said yet is forgotten: the user chose again. Returns
    /// whether anything changed.
    pub fn choose(&mut self, conn: &ConnectionId, table: &ObjectRef, choice: LabelChoice) -> bool {
        let before = self.kept(conn, table).cloned().unwrap_or_default();
        let after = Kept { choice, gone: None };
        if before == after {
            return false;
        }
        *self.kept_mut(conn, table) = after;
        self.tidy(conn, table);
        true
    }

    /// The column chosen for `table` is not there any more: the choice
    /// goes back to Automatic, and `column` is kept to be said once.
    pub fn lost(&mut self, conn: &ConnectionId, table: &ObjectRef, column: &str) {
        *self.kept_mut(conn, table) = Kept {
            choice: LabelChoice::Automatic,
            gone: Some(column.to_owned()),
        };
    }

    /// The chosen column of `table` that was found gone, and that the user
    /// has not been told of.
    pub fn gone(&self, conn: &ConnectionId, table: &ObjectRef) -> Option<&str> {
        self.kept(conn, table)?.gone.as_deref()
    }

    /// The user was told. Returns whether there was anything to tell.
    pub fn told(&mut self, conn: &ConnectionId, table: &ObjectRef) -> bool {
        if self.gone(conn, table).is_none() {
            return false;
        }
        self.kept_mut(conn, table).gone = None;
        self.tidy(conn, table);
        true
    }

    /// Forgets every choice of a saved connection that was deleted.
    /// Returns whether there was any.
    pub fn forget(&mut self, conn: &ConnectionId) -> bool {
        self.connections.remove(&conn.0).is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn column(name: &str, type_name: &str) -> ColumnInfo {
        ColumnInfo {
            name: name.into(),
            type_name: type_name.into(),
            ..ColumnInfo::default()
        }
    }

    /// The Bookshop's `genres`: a key, a number and a name.
    fn genres() -> Vec<ColumnInfo> {
        vec![
            column("id", "INTEGER"),
            column("parent_id", "INTEGER"),
            column("name", "TEXT"),
        ]
    }

    fn chosen(name: &str) -> LabelChoice {
        LabelChoice::Column(name.into())
    }

    fn resolved(choice: &LabelChoice, columns: &[ColumnInfo]) -> Resolved {
        resolve(choice, Dialect::Sqlite, columns, "id")
    }

    #[test]
    fn the_apps_pick_is_the_first_text_column_that_is_not_the_key() {
        // As the structure names PostgreSQL's types, which is how
        // `column_class` reads them.
        let columns = [
            column("code", "text"),
            column("founded", "integer"),
            column("name", "character varying(64)"),
            column("email", "text"),
        ];
        assert_eq!(
            automatic(Dialect::Postgres, &columns, "code"),
            Some("name".into())
        );
        assert_eq!(
            automatic(Dialect::Postgres, &columns, "founded"),
            Some("code".into())
        );
        assert_eq!(automatic(Dialect::Postgres, &columns[1..2], "id"), None);
    }

    #[test]
    fn automatic_resolves_to_the_apps_pick() {
        assert_eq!(
            resolved(&LabelChoice::Automatic, &genres()),
            Resolved {
                column: Some("name".into()),
                source: Source::Automatic,
                automatic: Some("name".into()),
                gone: None,
            }
        );
    }

    #[test]
    fn a_chosen_column_and_no_label_are_the_users() {
        let by_number = resolved(&chosen("parent_id"), &genres());
        assert_eq!(by_number.column.as_deref(), Some("parent_id"));
        assert_eq!(by_number.source, Source::User);
        // The chooser still names what Automatic would use.
        assert_eq!(by_number.automatic.as_deref(), Some("name"));
        let none = resolved(&LabelChoice::NoLabel, &genres());
        assert_eq!((none.column, none.source), (None, Source::User));
    }

    #[test]
    fn a_table_of_its_key_alone_has_nothing_to_choose() {
        let isbns = [column("isbn", "TEXT")];
        for choice in [LabelChoice::Automatic, LabelChoice::NoLabel, chosen("isbn")] {
            let resolved = resolve(&choice, Dialect::Sqlite, &isbns, "isbn");
            assert_eq!(
                (resolved.column, resolved.source),
                (None, Source::KeysOnly),
                "{choice:?}"
            );
        }
        // So with a key and bytes.
        let covers = [column("id", "INTEGER"), column("image", "BLOB")];
        let resolved = resolved(&LabelChoice::Automatic, &covers);
        assert_eq!(resolved.source, Source::KeysOnly);
    }

    #[test]
    fn a_table_with_no_text_column_is_listed_by_keys_until_one_is_chosen() {
        let shelves = [column("id", "INTEGER"), column("floor", "INTEGER")];
        let by_app = resolved(&LabelChoice::Automatic, &shelves);
        assert_eq!((by_app.column, by_app.source), (None, Source::Automatic));
        let by_floor = resolved(&chosen("floor"), &shelves);
        assert_eq!(by_floor.column.as_deref(), Some("floor"));
    }

    #[test]
    fn a_chosen_column_that_is_gone_falls_back_to_the_apps_pick() {
        let resolved = resolved(&chosen("legal_name"), &genres());
        assert_eq!(
            resolved,
            Resolved {
                column: Some("name".into()),
                source: Source::Automatic,
                automatic: Some("name".into()),
                gone: Some("legal_name".into()),
            }
        );
    }

    #[test]
    fn the_key_and_bytes_are_never_a_label() {
        for (dialect, binary) in [
            (Dialect::Postgres, "bytea"),
            (Dialect::MySql, "blob"),
            (Dialect::MySql, "binary(16)"),
            (Dialect::MySql, "varbinary(255)"),
            (Dialect::Sqlite, "BLOB"),
        ] {
            let cover = column("cover", binary);
            assert_eq!(
                excluded(dialect, &cover, &["id"]),
                Some(Excluded::Binary),
                "{binary}"
            );
        }
        let id = column("id", "INTEGER");
        assert_eq!(excluded(Dialect::Sqlite, &id, &["id"]), Some(Excluded::Key));
        assert_eq!(excluded(Dialect::Sqlite, &id, &["isbn"]), None);
        // A date and a number can be.
        for kind in ["date", "numeric(10,2)", "boolean", "jsonb"] {
            let other = column("other", kind);
            assert_eq!(excluded(Dialect::Postgres, &other, &["id"]), None, "{kind}");
        }
        // Chosen where another key references it: this picker lists keys,
        // and the choice stays the user's.
        let resolved = resolve(&chosen("name"), Dialect::Sqlite, &genres(), "name");
        assert_eq!((resolved.column, resolved.source), (None, Source::User));
    }

    fn publishers() -> ObjectRef {
        ObjectRef::new("public", "publishers")
    }

    #[test]
    fn a_choice_is_kept_by_connection_and_table() {
        let (dev, production) = (ConnectionId("dev".into()), ConnectionId("prod".into()));
        let mut labels = Labels::default();
        assert_eq!(labels.choice(&dev, &publishers()), LabelChoice::Automatic);
        assert!(labels.choose(&dev, &publishers(), chosen("name")));
        assert!(!labels.choose(&dev, &publishers(), chosen("name")));
        assert_eq!(labels.choice(&dev, &publishers()), chosen("name"));
        // Another saved connection, another table, another schema: not theirs.
        assert_eq!(
            labels.choice(&production, &publishers()),
            LabelChoice::Automatic
        );
        let authors = ObjectRef::new("public", "authors");
        assert_eq!(labels.choice(&dev, &authors), LabelChoice::Automatic);
        let archived = ObjectRef::new("archive", "publishers");
        assert_eq!(labels.choice(&dev, &archived), LabelChoice::Automatic);
    }

    #[test]
    fn automatic_is_kept_as_nothing() {
        let dev = ConnectionId("dev".into());
        let mut labels = Labels::default();
        labels.choose(&dev, &publishers(), LabelChoice::NoLabel);
        assert!(labels.choose(&dev, &publishers(), LabelChoice::Automatic));
        assert_eq!(labels, Labels::default());
    }

    #[test]
    fn a_lost_column_is_said_once() {
        let dev = ConnectionId("dev".into());
        let mut labels = Labels::default();
        labels.choose(&dev, &publishers(), chosen("legal_name"));
        labels.lost(&dev, &publishers(), "legal_name");
        assert_eq!(labels.choice(&dev, &publishers()), LabelChoice::Automatic);
        assert_eq!(labels.gone(&dev, &publishers()), Some("legal_name"));
        assert!(labels.told(&dev, &publishers()));
        assert!(!labels.told(&dev, &publishers()));
        assert_eq!(labels, Labels::default());
        // A new choice before it was said: there is nothing left to say.
        labels.lost(&dev, &publishers(), "legal_name");
        labels.choose(&dev, &publishers(), chosen("name"));
        assert_eq!(labels.gone(&dev, &publishers()), None);
    }

    #[test]
    fn a_deleted_connections_choices_are_forgotten() {
        let (dev, production) = (ConnectionId("dev".into()), ConnectionId("prod".into()));
        let mut labels = Labels::default();
        labels.choose(&dev, &publishers(), chosen("name"));
        labels.choose(&production, &publishers(), LabelChoice::NoLabel);
        assert!(labels.forget(&dev));
        assert!(!labels.forget(&dev));
        assert_eq!(labels.choice(&dev, &publishers()), LabelChoice::Automatic);
        assert_eq!(
            labels.choice(&production, &publishers()),
            LabelChoice::NoLabel
        );
    }

    #[test]
    fn the_file_reads_as_it_was_written_and_a_person_can_read_it() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("labels.json");
        assert_eq!(Labels::load(&path), Labels::default());
        let dev = ConnectionId("dev".into());
        let mut labels = Labels::default();
        labels.choose(&dev, &publishers(), chosen("name"));
        labels.choose(
            &dev,
            &ObjectRef::new("public", "authors"),
            LabelChoice::NoLabel,
        );
        labels.lost(&dev, &ObjectRef::new("public", "genres"), "title");
        labels.save(&path).unwrap();
        assert_eq!(Labels::load(&path), labels);
        let written: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        assert_eq!(
            written,
            serde_json::json!({
                "version": 1,
                "connections": { "dev": { "public": {
                    "authors": { "choice": "no-label" },
                    "genres": { "gone": "title" },
                    "publishers": { "choice": { "column": "name" } },
                } } },
            })
        );
        // A file with less in it reads: every part has its default.
        std::fs::write(
            &path,
            br#"{"connections":{"dev":{"public":{"genres":{}}}}}"#,
        )
        .unwrap();
        let read = Labels::load(&path);
        assert_eq!(
            read.choice(&dev, &ObjectRef::new("public", "genres")),
            LabelChoice::Automatic
        );
    }
}

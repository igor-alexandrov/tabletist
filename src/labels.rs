//! The column that labels the rows of a table foreign keys point at: what
//! the user chose, kept on this computer, and what the choice comes to
//! against the table's columns. No UI, and nothing here talks to the
//! backend or to a database.

use std::collections::BTreeMap;
use std::path::Path;

use serde::{Deserialize, Serialize};
use tabletist_db::{ColumnClass, ColumnInfo, Dialect, Driver, ObjectRef, Structure, column_class};

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

/// The columns a table's rows are found by, for the table's own Structure
/// view, which knows no foreign key that points at the table: its primary
/// key. A table with none can be pointed at only through a column that is
/// unique alone, so there those are its keys.
///
/// A picker asks the rule about the one column its foreign key references
/// instead. Where that is a unique column beside a primary key, the two
/// differ for that key, and the picker's own line says what labels it.
pub fn own_keys(structure: &Structure) -> Vec<&str> {
    if !structure.primary_key.is_empty() {
        return structure.primary_key.iter().map(String::as_str).collect();
    }
    let mut keys: Vec<&str> = Vec::new();
    for index in structure.indexes.iter().filter(|index| index.unique) {
        if let Some([column]) = index.key_columns.as_deref()
            && !keys.contains(&column.as_str())
        {
            keys.push(column);
        }
    }
    keys
}

/// The column the app picks to read a row of a table by: its first text
/// column that is not a key. `keys` as for [`excluded`].
pub fn automatic(dialect: Dialect, columns: &[ColumnInfo], keys: &[&str]) -> Option<String> {
    columns
        .iter()
        .filter(|column| !keys.contains(&column.name.as_str()))
        .find(|column| {
            matches!(
                column_class(dialect, &column.type_name),
                ColumnClass::Text { .. }
            )
        })
        .map(|column| column.name.clone())
}

/// Why a column that was named for a table's label is not taken.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refused {
    /// The table has no column of that name.
    NoColumn,
    /// It has, and the column cannot be a label.
    Excluded(Excluded),
}

/// Whether the column `name` of a table of `columns` can label its rows.
/// `keys` as for [`excluded`]. What a chooser's line offers needs no
/// asking: this is for a column that was named by hand.
pub fn choosable(
    dialect: Dialect,
    columns: &[ColumnInfo],
    keys: &[&str],
    name: &str,
) -> Result<(), Refused> {
    let column = columns.iter().find(|column| column.name == name);
    let column = column.ok_or(Refused::NoColumn)?;
    match excluded(dialect, column, keys) {
        Some(why) => Err(Refused::Excluded(why)),
        None => Ok(()),
    }
}

/// What `choice` comes to for a table of `columns`. `keys` are the
/// columns its rows are found by: the one a picker's foreign key
/// references, or the table's primary key in its Structure view. A column
/// that was chosen and is not among the columns is `gone`, and the app's
/// pick stands in for it. A chosen column that cannot be a label any more
/// (it is a key here, or holds bytes now) labels nothing: the choice is
/// still the user's.
pub fn resolve(
    choice: &LabelChoice,
    dialect: Dialect,
    columns: &[ColumnInfo],
    keys: &[&str],
) -> Resolved {
    let automatic = automatic(dialect, columns, keys);
    let allowed = |column: &ColumnInfo| excluded(dialect, column, keys).is_none();
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

/// One line of the chooser of a table's label.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Line {
    /// The app picks.
    Automatic,
    /// The rows are listed by their keys alone.
    NoLabel,
    /// A column of the table, with its type as the catalog names it.
    /// `excluded` says why it cannot be chosen, where it cannot: it is
    /// listed all the same, so the chooser says what the table holds.
    Column {
        name: String,
        type_name: String,
        excluded: Option<Excluded>,
    },
}

impl Line {
    /// What choosing the line keeps. None for a column that cannot be a
    /// label.
    pub fn choice(&self) -> Option<LabelChoice> {
        match self {
            Self::Automatic => Some(LabelChoice::Automatic),
            Self::NoLabel => Some(LabelChoice::NoLabel),
            Self::Column {
                name,
                excluded: None,
                ..
            } => Some(LabelChoice::Column(name.clone())),
            Self::Column {
                excluded: Some(_), ..
            } => None,
        }
    }
}

/// The chooser's lines for a table of `columns`: Automatic, No label, the
/// columns that can be chosen in the table's order, then the ones that
/// cannot. While the columns are not known (`None`) the first two are all
/// there is, and both can be chosen. `keys` as for [`excluded`].
pub fn lines(dialect: Dialect, columns: Option<&[ColumnInfo]>, keys: &[&str]) -> Vec<Line> {
    let mut lines = vec![Line::Automatic, Line::NoLabel];
    let columns = columns.unwrap_or_default();
    let line = |column: &ColumnInfo| Line::Column {
        name: column.name.clone(),
        type_name: column.type_name.clone(),
        excluded: excluded(dialect, column, keys),
    };
    let (allowed, refused): (Vec<Line>, Vec<Line>) = columns
        .iter()
        .map(line)
        .partition(|line| line.choice().is_some());
    lines.extend(allowed);
    lines.extend(refused);
    lines
}

/// The chooser of a table's label, while it is open: where its keys are.
/// Its lines are made from the table's columns each time they are read
/// ([`lines`]), so columns that arrive while it is open show in it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Chooser {
    /// The line the keys are on, among [`lines`].
    pub at: usize,
    /// What was chosen when it opened: the line that is marked.
    pub choice: LabelChoice,
    /// It opened before the table's columns were known.
    pub waited: bool,
}

impl Chooser {
    /// A chooser that opens on the line of `choice`, or on Automatic where
    /// no line is the choice's (its column cannot be chosen any more).
    pub fn open(lines: &[Line], choice: LabelChoice, waited: bool) -> Self {
        let chosen = |line: &Line| line.choice().as_ref() == Some(&choice);
        Self {
            at: lines.iter().position(chosen).unwrap_or(0),
            choice,
            waited,
        }
    }

    /// Moves `by` lines that can be chosen, stopping at the last of them
    /// either way: a line that cannot be chosen is stepped over.
    pub fn step(&mut self, lines: &[Line], by: isize) {
        let open = |at: &usize| lines.get(*at).is_some_and(|line| line.choice().is_some());
        for _ in 0..by.unsigned_abs() {
            let next = if by > 0 {
                (self.at + 1..lines.len()).find(open)
            } else {
                (0..self.at).rev().find(open)
            };
            match next {
                Some(at) => self.at = at,
                None => break,
            }
        }
    }

    /// Goes to the line at `index`, if it can be chosen: the pointer came
    /// over it.
    pub fn to(&mut self, lines: &[Line], index: usize) {
        if lines.get(index).is_some_and(|line| line.choice().is_some()) {
            self.at = index;
        }
    }

    /// Goes to the next column after the line the keys are on whose name
    /// begins with `typed`, whatever its case, and round from the last to
    /// the first: typing a letter again goes on to the next such column.
    /// Only a column that can be chosen is gone to.
    pub fn jump(&mut self, lines: &[Line], typed: char) {
        let begins = |line: &Line| match line {
            Line::Column {
                name,
                excluded: None,
                ..
            } => name
                .chars()
                .next()
                .is_some_and(|first| first.to_lowercase().eq(typed.to_lowercase())),
            Line::Column {
                excluded: Some(_), ..
            }
            | Line::Automatic
            | Line::NoLabel => false,
        };
        let count = lines.len();
        let found = (1..=count)
            .map(|step| (self.at + step) % count)
            .find(|at| begins(&lines[*at]));
        if let Some(at) = found {
            self.at = at;
        }
    }

    /// What Enter keeps: the choice of the line the keys are on.
    pub fn chosen(&self, lines: &[Line]) -> Option<LabelChoice> {
        lines.get(self.at).and_then(Line::choice)
    }
}

/// The file's format. The first kept its choices by connection alone.
pub const VERSION: u32 = 2;

/// Where a table is, for what is kept of it: the saved connection, and
/// the database the tab is on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Place {
    pub conn: ConnectionId,
    /// As [`database_key`] names it.
    pub database: String,
}

/// The database a choice is kept under, for a connection of `driver` that
/// is on `database`. One PostgreSQL server holds several databases, each
/// with schemas of its own, and a tab can be switched from one to another:
/// a table of one is not the table of the same name in the next. Nothing
/// elsewhere: a MySQL table's schema is its database, and a SQLite
/// connection is its one file.
pub fn database_key(driver: Driver, database: &str) -> String {
    match driver {
        Driver::Postgres => database.to_owned(),
        Driver::MySql | Driver::Sqlite | Driver::Files => String::new(),
    }
}

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

/// What is kept of the tables of one database: by schema, then by table.
type Tables = BTreeMap<String, BTreeMap<String, Kept>>;

/// Every choice, stored as `labels.json` in the state directory: by the
/// saved connection's id, then by database ([`database_key`]), then by
/// schema, then by table. Nothing of it is ever written to a database.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(from = "File")]
pub struct Labels {
    version: u32,
    connections: BTreeMap<String, BTreeMap<String, Tables>>,
    /// What a file of the first format kept, by connection alone, until
    /// [`Labels::adopt`] says which database each connection is saved
    /// with.
    #[serde(skip)]
    legacy: BTreeMap<String, Tables>,
}

impl Default for Labels {
    fn default() -> Self {
        Self {
            version: VERSION,
            connections: BTreeMap::new(),
            legacy: BTreeMap::new(),
        }
    }
}

/// The file as it is read, in either format: what is kept of a connection
/// is read by the file's version.
#[derive(Default, Deserialize)]
#[serde(default)]
struct File {
    version: u32,
    connections: BTreeMap<String, serde_json::Value>,
}

impl From<File> for Labels {
    fn from(file: File) -> Self {
        let mut labels = Self::default();
        for (conn, kept) in file.connections {
            // A connection whose part does not read is left out, and the
            // others are kept.
            if file.version < VERSION {
                match serde_json::from_value(kept) {
                    Ok(tables) => drop(labels.legacy.insert(conn, tables)),
                    Err(error) => log::warn!("labels of {conn} were not read: {error}"),
                }
            } else {
                match serde_json::from_value(kept) {
                    Ok(databases) => drop(labels.connections.insert(conn, databases)),
                    Err(error) => log::warn!("labels of {conn} were not read: {error}"),
                }
            }
        }
        labels
    }
}

impl Labels {
    /// The store at `path`. A missing file is an empty store; one that
    /// cannot be read is kept aside (`util::load_json`), and the store
    /// starts empty. A file of the first format is read whole, and its
    /// choices wait for [`Labels::adopt`].
    pub fn load(path: &Path) -> Self {
        crate::util::load_json(path)
    }

    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        crate::util::save_json(path, self)
    }

    /// Gives what a file of the first format kept to the database each
    /// connection is saved with, which `database_of` names as
    /// [`database_key`] does: there was no other database a choice could
    /// have been made on that the file could tell apart. A connection
    /// that is saved no more (`None`) keeps nothing. Returns whether the
    /// file was of that format and held anything.
    pub fn adopt(&mut self, database_of: impl Fn(&ConnectionId) -> Option<String>) -> bool {
        let legacy = std::mem::take(&mut self.legacy);
        let any = !legacy.is_empty();
        for (conn, tables) in legacy {
            let Some(database) = database_of(&ConnectionId(conn.clone())) else {
                continue;
            };
            let kept = self.connections.entry(conn).or_default();
            kept.entry(database).or_default().extend(tables);
        }
        any
    }

    fn kept(&self, place: &Place, table: &ObjectRef) -> Option<&Kept> {
        self.connections
            .get(&place.conn.0)?
            .get(&place.database)?
            .get(&table.schema)?
            .get(&table.name)
    }

    fn kept_mut(&mut self, place: &Place, table: &ObjectRef) -> &mut Kept {
        self.connections
            .entry(place.conn.0.clone())
            .or_default()
            .entry(place.database.clone())
            .or_default()
            .entry(table.schema.clone())
            .or_default()
            .entry(table.name.clone())
            .or_default()
    }

    /// Drops what is kept of `table` where it says nothing any more, and
    /// the schema, the database and the connection it leaves empty.
    fn tidy(&mut self, place: &Place, table: &ObjectRef) {
        let Some(databases) = self.connections.get_mut(&place.conn.0) else {
            return;
        };
        if let Some(schemas) = databases.get_mut(&place.database) {
            if let Some(tables) = schemas.get_mut(&table.schema) {
                if tables.get(&table.name) == Some(&Kept::default()) {
                    tables.remove(&table.name);
                }
                if tables.is_empty() {
                    schemas.remove(&table.schema);
                }
            }
            if schemas.is_empty() {
                databases.remove(&place.database);
            }
        }
        if databases.is_empty() {
            self.connections.remove(&place.conn.0);
        }
    }

    /// What was chosen for `table` at `place`.
    pub fn choice(&self, place: &Place, table: &ObjectRef) -> LabelChoice {
        self.kept(place, table)
            .map(|kept| kept.choice.clone())
            .unwrap_or_default()
    }

    /// Makes `choice` the choice for `table`. A column that was noted gone
    /// and not said yet is forgotten: the user chose again. Returns
    /// whether anything changed.
    pub fn choose(&mut self, place: &Place, table: &ObjectRef, choice: LabelChoice) -> bool {
        let before = self.kept(place, table).cloned().unwrap_or_default();
        let after = Kept { choice, gone: None };
        if before == after {
            return false;
        }
        *self.kept_mut(place, table) = after;
        self.tidy(place, table);
        true
    }

    /// The column chosen for `table` is not there any more: the choice
    /// goes back to Automatic, and `column` is kept to be said once.
    pub fn lost(&mut self, place: &Place, table: &ObjectRef, column: &str) {
        *self.kept_mut(place, table) = Kept {
            choice: LabelChoice::Automatic,
            gone: Some(column.to_owned()),
        };
    }

    /// The chosen column of `table` that was found gone, and that the user
    /// has not been told of.
    pub fn gone(&self, place: &Place, table: &ObjectRef) -> Option<&str> {
        self.kept(place, table)?.gone.as_deref()
    }

    /// The user was told. Returns whether there was anything to tell.
    pub fn told(&mut self, place: &Place, table: &ObjectRef) -> bool {
        if self.gone(place, table).is_none() {
            return false;
        }
        self.kept_mut(place, table).gone = None;
        self.tidy(place, table);
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
        resolve(choice, Dialect::Sqlite, columns, &["id"])
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
            automatic(Dialect::Postgres, &columns, &["code"]),
            Some("name".into())
        );
        assert_eq!(
            automatic(Dialect::Postgres, &columns, &["founded"]),
            Some("code".into())
        );
        assert_eq!(automatic(Dialect::Postgres, &columns[1..2], &["id"]), None);
    }

    /// PostgreSQL's `citext` is text: an email kept in one is the app's
    /// pick, before a text column that comes after it.
    #[test]
    fn the_apps_pick_takes_text_that_compares_without_case() {
        let authors = [
            column("id", "bigint"),
            column("email", "citext"),
            column("born_on", "date"),
            column("pen_name", "character varying"),
        ];
        assert_eq!(
            automatic(Dialect::Postgres, &authors, &["id"]),
            Some("email".into())
        );
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
            let resolved = resolve(&choice, Dialect::Sqlite, &isbns, &["isbn"]);
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
        let resolved = resolve(&chosen("name"), Dialect::Sqlite, &genres(), &["name"]);
        assert_eq!((resolved.column, resolved.source), (None, Source::User));
    }

    /// The Bookshop's `editions` as the screenshot scenes have it, cut to
    /// what matters: a key, two texts, bytes and a date.
    fn editions() -> Vec<ColumnInfo> {
        vec![
            column("id", "bigint"),
            column("title", "text"),
            column("subtitle", "text"),
            column("cover", "bytea"),
            column("published_on", "date"),
        ]
    }

    fn named(lines: &[Line]) -> Vec<(&str, Option<Excluded>)> {
        lines
            .iter()
            .map(|line| match line {
                Line::Automatic => ("Automatic", None),
                Line::NoLabel => ("No label", None),
                Line::Column { name, excluded, .. } => (name.as_str(), *excluded),
            })
            .collect()
    }

    #[test]
    fn the_chooser_lists_what_can_be_chosen_then_what_cannot() {
        let lines = lines(Dialect::Postgres, Some(&editions()), &["id"]);
        assert_eq!(
            named(&lines),
            [
                ("Automatic", None),
                ("No label", None),
                ("title", None),
                ("subtitle", None),
                ("published_on", None),
                ("id", Some(Excluded::Key)),
                ("cover", Some(Excluded::Binary)),
            ]
        );
        // A column's type is the catalog's word for it.
        assert!(matches!(&lines[4], Line::Column { type_name, .. } if type_name == "date"));
        // What a line keeps, and that a key and bytes keep nothing.
        assert_eq!(lines[0].choice(), Some(LabelChoice::Automatic));
        assert_eq!(lines[1].choice(), Some(LabelChoice::NoLabel));
        assert_eq!(lines[3].choice(), Some(chosen("subtitle")));
        assert_eq!(lines[5].choice(), None);
        assert_eq!(lines[6].choice(), None);
    }

    #[test]
    fn a_chooser_without_columns_offers_automatic_and_no_label() {
        let waiting = lines(Dialect::Postgres, None, &["id"]);
        assert_eq!(named(&waiting), [("Automatic", None), ("No label", None)]);
        let mut chooser = Chooser::open(&waiting, LabelChoice::Automatic, true);
        assert!(chooser.waited);
        chooser.step(&waiting, 1);
        assert_eq!(chooser.chosen(&waiting), Some(LabelChoice::NoLabel));
        // The columns came: the keys are where they were.
        let known = lines(Dialect::Postgres, Some(&editions()), &["id"]);
        assert_eq!(chooser.chosen(&known), Some(LabelChoice::NoLabel));
    }

    #[test]
    fn a_chooser_opens_on_the_choice_and_steps_over_what_cannot_be_chosen() {
        let lines = lines(Dialect::Postgres, Some(&editions()), &["id"]);
        let mut chooser = Chooser::open(&lines, chosen("subtitle"), false);
        assert_eq!(chooser.at, 3);
        // Down to the last column that can be chosen, and no further.
        chooser.step(&lines, 1);
        assert_eq!(chooser.chosen(&lines), Some(chosen("published_on")));
        chooser.step(&lines, 5);
        assert_eq!(chooser.at, 4);
        // Up to Automatic, and no further.
        chooser.step(&lines, -9);
        assert_eq!(chooser.chosen(&lines), Some(LabelChoice::Automatic));
        // The pointer goes to a line that can be chosen, and to no other.
        chooser.to(&lines, 2);
        assert_eq!(chooser.at, 2);
        chooser.to(&lines, 6);
        chooser.to(&lines, 60);
        assert_eq!(chooser.at, 2);
        // A choice no line is (the column is bytes now) opens on Automatic.
        assert_eq!(Chooser::open(&lines, chosen("cover"), false).at, 0);
    }

    #[test]
    fn a_letter_jumps_to_the_next_column_it_begins() {
        let lines = lines(Dialect::Postgres, Some(&editions()), &["id"]);
        let mut chooser = Chooser::open(&lines, LabelChoice::Automatic, false);
        chooser.jump(&lines, 's');
        assert_eq!(chooser.chosen(&lines), Some(chosen("subtitle")));
        // Whatever its case, and on to the next and round again.
        chooser.jump(&lines, 'T');
        assert_eq!(chooser.chosen(&lines), Some(chosen("title")));
        chooser.jump(&lines, 't');
        assert_eq!(chooser.chosen(&lines), Some(chosen("title")));
        chooser.jump(&lines, 'p');
        assert_eq!(chooser.chosen(&lines), Some(chosen("published_on")));
        // Never to a column that cannot be chosen, nor to the first two
        // lines: `a` is no Automatic, `c` no cover.
        for letter in ['a', 'n', 'c', 'i', 'z'] {
            chooser.jump(&lines, letter);
            assert_eq!(chooser.at, 4, "{letter}");
        }
    }

    #[test]
    fn a_column_named_by_hand_is_refused_with_its_reason() {
        let columns = editions();
        let asked = |name| choosable(Dialect::Postgres, &columns, &["id"], name);
        assert_eq!(asked("title"), Ok(()));
        assert_eq!(asked("published_on"), Ok(()));
        assert_eq!(asked("id"), Err(Refused::Excluded(Excluded::Key)));
        assert_eq!(asked("cover"), Err(Refused::Excluded(Excluded::Binary)));
        assert_eq!(asked("legal_name"), Err(Refused::NoColumn));
        // A name is a column's as it is written, and no other's.
        assert_eq!(asked("Title"), Err(Refused::NoColumn));
    }

    /// A table's label is asked for a key: the column a foreign key
    /// references. Where that is a unique column and not the primary key,
    /// the app's pick for that key leaves it out, and a choice of it lists
    /// that key's rows by the key alone, still as the user's.
    #[test]
    fn a_key_to_a_unique_column_is_labelled_without_that_column() {
        let isbns = [
            column("id", "INTEGER"),
            column("isbn", "TEXT"),
            column("note", "TEXT"),
        ];
        let label = |choice: &LabelChoice, key: &'static str| {
            let resolved = resolve(choice, Dialect::Sqlite, &isbns, &[key]);
            (resolved.column, resolved.source)
        };
        let auto = LabelChoice::Automatic;
        assert_eq!(label(&auto, "id"), (Some("isbn".into()), Source::Automatic));
        assert_eq!(
            label(&auto, "isbn"),
            (Some("note".into()), Source::Automatic)
        );
        let chosen = chosen("isbn");
        assert_eq!(label(&chosen, "id"), (Some("isbn".into()), Source::User));
        assert_eq!(label(&chosen, "isbn"), (None, Source::User));
    }

    /// The keys of a table's own Structure view: its primary key, and for
    /// a table with none the columns that are unique alone, which are all
    /// a foreign key can reference there.
    #[test]
    fn a_table_with_no_primary_key_is_found_by_its_unique_columns() {
        use tabletist_db::{IndexInfo, Structure};
        let unique = |name: &str, columns: &[&str]| IndexInfo {
            name: name.into(),
            columns: columns.iter().map(|column| (*column).to_owned()).collect(),
            key_columns: Some(columns.iter().map(|column| (*column).to_owned()).collect()),
            unique: true,
            ..IndexInfo::default()
        };
        let mut isbns = Structure {
            columns: vec![
                column("isbn", "TEXT"),
                column("note", "TEXT"),
                column("shelf", "TEXT"),
            ],
            indexes: vec![
                unique("isbns_isbn", &["isbn"]),
                // Two columns together name a row; neither does alone.
                unique("isbns_place", &["note", "shelf"]),
                IndexInfo {
                    unique: false,
                    ..unique("isbns_shelf", &["shelf"])
                },
                // An index over an expression names no column.
                IndexInfo {
                    key_columns: None,
                    ..unique("isbns_lower", &["<expression>"])
                },
            ],
            ..Structure::default()
        };
        assert_eq!(own_keys(&isbns), ["isbn"]);
        let keys = own_keys(&isbns);
        let resolved = resolve(
            &LabelChoice::Automatic,
            Dialect::Sqlite,
            &isbns.columns,
            &keys,
        );
        assert_eq!(resolved.column.as_deref(), Some("note"));
        // With a primary key, that alone: a unique column beside it can
        // label the rows a key to the primary key lists.
        isbns.columns.insert(0, column("id", "INTEGER"));
        isbns.primary_key = vec!["id".into()];
        assert_eq!(own_keys(&isbns), ["id"]);
    }

    #[test]
    fn a_table_with_no_key_or_a_key_of_several_columns_excludes_them_all() {
        let columns = editions();
        // No key: nothing is the key, and the app's pick is its first text.
        let none = resolve(&LabelChoice::Automatic, Dialect::Postgres, &columns, &[]);
        assert_eq!(none.column.as_deref(), Some("title"));
        assert_eq!(excluded(Dialect::Postgres, &columns[0], &[]), None);
        // Two columns: neither labels, and the pick is the next text.
        let keys = ["id", "title"];
        let both = resolve(&LabelChoice::Automatic, Dialect::Postgres, &columns, &keys);
        assert_eq!(both.column.as_deref(), Some("subtitle"));
        assert_eq!(
            choosable(Dialect::Postgres, &columns, &keys, "title"),
            Err(Refused::Excluded(Excluded::Key))
        );
    }

    fn publishers() -> ObjectRef {
        ObjectRef::new("public", "publishers")
    }

    /// A saved connection that reaches one database: MySQL's or SQLite's.
    fn at(conn: &str) -> Place {
        on(conn, "")
    }

    /// A saved connection of PostgreSQL, on `database`.
    fn on(conn: &str, database: &str) -> Place {
        Place {
            conn: ConnectionId(conn.into()),
            database: database.into(),
        }
    }

    #[test]
    fn a_choice_is_kept_by_connection_and_table() {
        let (dev, production) = (at("dev"), at("prod"));
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
        let dev = at("dev");
        let mut labels = Labels::default();
        labels.choose(&dev, &publishers(), LabelChoice::NoLabel);
        assert!(labels.choose(&dev, &publishers(), LabelChoice::Automatic));
        assert_eq!(labels, Labels::default());
    }

    #[test]
    fn a_lost_column_is_said_once() {
        let dev = at("dev");
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
        let (dev, production) = (at("dev"), at("prod"));
        let mut labels = Labels::default();
        labels.choose(&dev, &publishers(), chosen("name"));
        labels.choose(&production, &publishers(), LabelChoice::NoLabel);
        assert!(labels.forget(&dev.conn));
        assert!(!labels.forget(&dev.conn));
        assert_eq!(labels.choice(&dev, &publishers()), LabelChoice::Automatic);
        assert_eq!(
            labels.choice(&production, &publishers()),
            LabelChoice::NoLabel
        );
    }

    /// One PostgreSQL connection reaches several databases, and a tab is
    /// switched from one to another: a table of one is not the table of
    /// the same name in the next. MySQL's schema is its database, and a
    /// SQLite connection is one file.
    #[test]
    fn a_choice_is_its_databases_alone() {
        let (app, other) = (on("dev", "app"), on("dev", "other"));
        let mut labels = Labels::default();
        labels.choose(&app, &publishers(), chosen("legal_name"));
        assert_eq!(labels.choice(&other, &publishers()), LabelChoice::Automatic);
        // What the other database's table lacks is not gone from this one.
        labels.lost(&other, &publishers(), "legal_name");
        assert_eq!(labels.choice(&app, &publishers()), chosen("legal_name"));
        assert_eq!(labels.gone(&app, &publishers()), None);
        assert!(labels.told(&other, &publishers()));
        // Forgetting the connection forgets every database of it.
        assert!(labels.forget(&app.conn));
        assert_eq!(labels, Labels::default());
        assert_eq!(database_key(Driver::Postgres, "app"), "app");
        assert_eq!(database_key(Driver::MySql, "app"), "");
        assert_eq!(database_key(Driver::Sqlite, "app"), "");
    }

    /// The first format kept a choice by connection alone. Its file still
    /// reads: each connection's choices are those of the database it is
    /// saved with, and a connection that is saved no more keeps nothing.
    #[test]
    fn a_file_of_the_first_format_is_its_connections_own_database() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("labels.json");
        std::fs::write(
            &path,
            br#"{"version":1,"connections":{
                "dev":{"public":{"publishers":{"choice":{"column":"name"}},"genres":{"gone":"title"}}},
                "deleted":{"public":{"publishers":{"choice":"no-label"}}}}}"#,
        )
        .unwrap();
        let mut labels = Labels::load(&path);
        let own = |conn: &ConnectionId| (conn.0 == "dev").then(|| "app".to_owned());
        assert!(labels.adopt(own));
        assert!(!labels.adopt(own), "once");
        let (app, other) = (on("dev", "app"), on("dev", "other"));
        assert_eq!(labels.choice(&app, &publishers()), chosen("name"));
        let genres = ObjectRef::new("public", "genres");
        assert_eq!(labels.gone(&app, &genres), Some("title"));
        assert_eq!(labels.choice(&other, &publishers()), LabelChoice::Automatic);
        let deleted = on("deleted", "app");
        assert_eq!(
            labels.choice(&deleted, &publishers()),
            LabelChoice::Automatic
        );
        // It is written in the format of now, and reads the same again.
        labels.save(&path).unwrap();
        let mut read = Labels::load(&path);
        assert!(!read.adopt(own));
        assert_eq!(read, labels);
        // A part that does not read costs that connection's choices, and
        // no other's.
        std::fs::write(
            &path,
            br#"{"version":2,"connections":{
                "dev":{"app":{"public":{"publishers":{"choice":"no-label"}}}},
                "odd":{"app":"not a schema"}}}"#,
        )
        .unwrap();
        let read = Labels::load(&path);
        assert_eq!(read.choice(&app, &publishers()), LabelChoice::NoLabel);
    }

    #[test]
    fn the_file_reads_as_it_was_written_and_a_person_can_read_it() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("labels.json");
        assert_eq!(Labels::load(&path), Labels::default());
        let dev = on("dev", "bookshop");
        let mut labels = Labels::default();
        labels.choose(&dev, &publishers(), chosen("name"));
        labels.choose(
            &dev,
            &ObjectRef::new("public", "authors"),
            LabelChoice::NoLabel,
        );
        // A connection with one database keeps its tables under no name.
        labels.choose(
            &at("file"),
            &ObjectRef::new("main", "isbns"),
            chosen("note"),
        );
        labels.lost(&dev, &ObjectRef::new("public", "genres"), "title");
        labels.save(&path).unwrap();
        assert_eq!(Labels::load(&path), labels);
        let written: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        assert_eq!(
            written,
            serde_json::json!({
                "version": 2,
                "connections": {
                    "dev": { "bookshop": { "public": {
                        "authors": { "choice": "no-label" },
                        "genres": { "gone": "title" },
                        "publishers": { "choice": { "column": "name" } },
                    } } },
                    "file": { "": { "main": {
                        "isbns": { "choice": { "column": "note" } },
                    } } },
                },
            })
        );
        // A file with less in it reads: every part has its default.
        std::fs::write(
            &path,
            br#"{"version":2,"connections":{"dev":{"bookshop":{"public":{"genres":{}}}}}}"#,
        )
        .unwrap();
        let read = Labels::load(&path);
        assert_eq!(
            read.choice(&dev, &ObjectRef::new("public", "genres")),
            LabelChoice::Automatic
        );
    }
}

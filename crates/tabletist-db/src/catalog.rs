//! Database objects and their structure.

/// The most schemas, or objects in one schema, a listing returns, so a
/// server with a huge catalog cannot make the sidebar hold it all.
pub const MAX_LISTED: u32 = 10_000;

/// A table, view or materialized view, by schema and name.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ObjectRef {
    /// Schema (PostgreSQL), database (MySQL) or attached database (SQLite).
    pub schema: String,
    pub name: String,
}

impl ObjectRef {
    pub fn new(schema: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            schema: schema.into(),
            name: name.into(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ObjectKind {
    Table,
    View,
    /// PostgreSQL only.
    MaterializedView,
}

/// An entry in the sidebar.
#[derive(Debug, Clone, PartialEq)]
pub struct ObjectInfo {
    pub name: String,
    pub kind: ObjectKind,
    /// From the planner's statistics; `None` when the database has none.
    pub estimated_rows: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct ColumnInfo {
    pub name: String,
    pub type_name: String,
    pub nullable: bool,
    pub default: Option<String>,
    pub comment: Option<String>,
    /// The values the column may hold, in the database's order: a
    /// PostgreSQL enum's labels, or a text column's `CHECK (col IN (...))`
    /// list. `None` for every other column.
    pub allowed_values: Option<Vec<String>>,
    /// Whether the database computes the column itself and refuses a value
    /// for it: a generated column, or an identity column that is always
    /// generated.
    pub generated: bool,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct IndexInfo {
    pub name: String,
    /// Column names in index order; expressions appear as `<expression>`.
    pub columns: Vec<String>,
    pub unique: bool,
    pub primary: bool,
    /// `btree`, `hash`, `gin`...; `None` when the database does not say.
    pub method: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct ForeignKeyInfo {
    /// SQLite foreign keys have no names.
    pub name: Option<String>,
    pub columns: Vec<String>,
    pub ref_schema: String,
    pub ref_table: String,
    /// Empty when the key references the other table's primary key implicitly.
    pub ref_columns: Vec<String>,
    pub on_update: String,
    pub on_delete: String,
}

/// What the Structure view shows.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Structure {
    pub columns: Vec<ColumnInfo>,
    pub primary_key: Vec<String>,
    pub indexes: Vec<IndexInfo>,
    pub foreign_keys: Vec<ForeignKeyInfo>,
}

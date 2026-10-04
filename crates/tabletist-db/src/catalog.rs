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
    /// Whether the index covers only the rows its condition keeps. Such an
    /// index says nothing about the rest, so it cannot name a row.
    pub partial: bool,
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

impl Structure {
    /// The columns that tell one row from every other, for a save to find
    /// the row by: the primary key, else the first unique index (by name)
    /// that covers every row and whose entries are all columns that cannot
    /// be NULL. `None` when the table has neither: a row there cannot be
    /// targeted safely.
    pub fn row_key(&self) -> Option<Vec<String>> {
        if !self.primary_key.is_empty() {
            return Some(self.primary_key.clone());
        }
        let mut candidates: Vec<&IndexInfo> = self
            .indexes
            .iter()
            .filter(|index| index.unique && !index.partial && !index.columns.is_empty())
            .collect();
        candidates.sort_by(|a, b| a.name.cmp(&b.name));
        candidates.into_iter().find_map(|index| {
            index
                .columns
                .iter()
                .map(|entry| {
                    let column = self.columns.iter().find(|column| {
                        // PostgreSQL gives an index's column as it would be
                        // written, so one that needs quotes comes quoted.
                        column.name == *entry
                            || format!("\"{}\"", column.name.replace('"', "\"\"")) == *entry
                    })?;
                    (!column.nullable).then(|| column.name.clone())
                })
                .collect()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn column(name: &str, nullable: bool) -> ColumnInfo {
        ColumnInfo {
            name: name.into(),
            nullable,
            ..ColumnInfo::default()
        }
    }

    fn unique(name: &str, columns: &[&str]) -> IndexInfo {
        IndexInfo {
            name: name.into(),
            columns: columns.iter().map(|column| (*column).to_owned()).collect(),
            unique: true,
            ..IndexInfo::default()
        }
    }

    fn table(indexes: Vec<IndexInfo>) -> Structure {
        Structure {
            columns: vec![
                column("id", false),
                column("email", false),
                column("code", false),
                column("nick", true),
                column("odd name", false),
            ],
            indexes,
            ..Structure::default()
        }
    }

    #[test]
    fn the_primary_key_is_the_row_key() {
        let mut structure = table(vec![unique("by_email", &["email"])]);
        structure.primary_key = vec!["id".into()];
        assert_eq!(structure.row_key(), Some(vec!["id".to_owned()]));
    }

    #[test]
    fn without_one_the_first_usable_unique_index_is() {
        let key = |indexes| table(indexes).row_key();
        // By name, whatever order the catalog gave them in.
        assert_eq!(
            key(vec![unique("z", &["code"]), unique("a", &["email"])]),
            Some(vec!["email".to_owned()])
        );
        // Several columns, in the index's order.
        assert_eq!(
            key(vec![unique("pair", &["code", "email"])]),
            Some(vec!["code".to_owned(), "email".to_owned()])
        );
        // PostgreSQL gives a name that needs quotes quoted.
        assert_eq!(
            key(vec![unique("odd", &["\"odd name\""])]),
            Some(vec!["odd name".to_owned()])
        );
    }

    #[test]
    fn an_index_that_cannot_name_a_row_is_passed_over() {
        let key = |indexes| table(indexes).row_key();
        let plain = IndexInfo {
            unique: false,
            ..unique("plain", &["email"])
        };
        let partial = IndexInfo {
            partial: true,
            ..unique("partial", &["email"])
        };
        for index in [
            plain,
            partial,
            // A column that can be NULL: two rows may both hold NULL.
            unique("nullable", &["nick"]),
            unique("mixed", &["email", "nick"]),
            // An expression is not a column.
            unique("expression", &["<expression>"]),
            unique("lowered", &["lower(email)"]),
            unique("empty", &[]),
        ] {
            let name = index.name.clone();
            assert_eq!(key(vec![index]), None, "{name}");
        }
        assert_eq!(key(Vec::new()), None);
        // The next usable one is taken.
        assert_eq!(
            key(vec![unique("a", &["nick"]), unique("b", &["code"])]),
            Some(vec!["code".to_owned()])
        );
    }
}

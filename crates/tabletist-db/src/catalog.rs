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
    /// Whether the database gives the column its value from a counter of
    /// its own when an insert names none: an identity column (always or by
    /// default), a serial, MySQL's `AUTO_INCREMENT`, SQLite's alias of the
    /// rowid. Its default, where it has one, says nothing to a user.
    pub identity: bool,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct IndexInfo {
    pub name: String,
    /// Column names in index order; expressions appear as `<expression>`.
    pub columns: Vec<String>,
    /// The index's columns by name, when every entry is one whole column of
    /// the table, compared as the column compares; `None` for an index over
    /// an expression, a prefix, or another collation. On SQLite another
    /// collation goes unseen (it does not say a column's declared one), so
    /// `Some` there is no proof that the columns name one row: a save still
    /// reads the row by them and refuses more than one.
    pub key_columns: Option<Vec<String>>,
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
    /// that covers every row and is over whole columns that cannot be NULL.
    /// `None` when the table has neither: a row there cannot be targeted
    /// safely. The caller checks the object's kind: a PostgreSQL
    /// materialized view can have a unique index and still is not editable.
    /// On SQLite the key can be one that compares otherwise than its
    /// columns (see `IndexInfo::key_columns`), so a save does not take it
    /// on trust.
    pub fn row_key(&self) -> Option<Vec<String>> {
        // MySQL takes a primary key over a prefix of a column, which lets
        // two rows agree on the column. The key's index says so, where the
        // catalog lists one (SQLite has none for a rowid's alias).
        let primary = self.indexes.iter().find(|index| index.primary);
        if !self.primary_key.is_empty() && primary.is_none_or(|index| index.key_columns.is_some()) {
            return Some(self.primary_key.clone());
        }
        self.indexes
            .iter()
            .filter(|index| index.unique && !index.partial)
            .filter_map(|index| Some((&index.name, index.key_columns.as_ref()?)))
            .filter(|(_, key)| {
                !key.is_empty()
                    && key.iter().all(|name| {
                        self.columns
                            .iter()
                            .any(|column| column.name == *name && !column.nullable)
                    })
            })
            .min_by_key(|(name, _)| *name)
            .map(|(_, key)| key.clone())
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

    /// A unique index over whole columns, shown by their names.
    fn unique(name: &str, columns: &[&str]) -> IndexInfo {
        let columns: Vec<String> = columns.iter().map(|column| (*column).to_owned()).collect();
        IndexInfo {
            name: name.into(),
            columns: columns.clone(),
            key_columns: Some(columns),
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
                column("<expression>", false),
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
        // Its own index, where the catalog lists one, changes nothing.
        structure.indexes.push(IndexInfo {
            primary: true,
            ..unique("PRIMARY", &["id"])
        });
        assert_eq!(structure.row_key(), Some(vec!["id".to_owned()]));
    }

    #[test]
    fn a_primary_key_that_is_not_over_whole_columns_is_passed_over() {
        // MySQL's `PRIMARY KEY (email(1))`: the catalog names the column.
        let prefix = IndexInfo {
            primary: true,
            key_columns: None,
            ..unique("PRIMARY", &["email"])
        };
        let mut structure = table(vec![prefix.clone()]);
        structure.primary_key = vec!["email".into()];
        assert_eq!(structure.row_key(), None);
        let mut structure = table(vec![prefix, unique("by_code", &["code"])]);
        structure.primary_key = vec!["email".into()];
        assert_eq!(structure.row_key(), Some(vec!["code".to_owned()]));
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
        // By the key's columns, not by the text the index shows for them.
        let shown_otherwise = IndexInfo {
            columns: vec!["email DESC".into()],
            ..unique("shown", &["email"])
        };
        assert_eq!(key(vec![shown_otherwise]), Some(vec!["email".to_owned()]));
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
        // A prefix or another collation: shown as the column, and not it.
        let not_whole = IndexInfo {
            key_columns: None,
            ..unique("not_whole", &["email"])
        };
        // An expression is not a column, whatever a column is called.
        let expression = IndexInfo {
            key_columns: None,
            ..unique("expression", &["<expression>"])
        };
        for index in [
            plain,
            partial,
            not_whole,
            expression,
            // A column that can be NULL: two rows may both hold NULL.
            unique("nullable", &["nick"]),
            unique("mixed", &["email", "nick"]),
            unique("unknown", &["gone"]),
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

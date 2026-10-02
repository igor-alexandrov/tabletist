//! The rows of a SQL editor's completion list: what a site offers, from
//! what the workspace knows. No UI.

use std::collections::{HashMap, HashSet};
use std::ops::Range;

use tabletist_db::complete::{Expects, PHRASES, Site, Source};
use tabletist_db::{ColumnInfo, Dialect, ObjectInfo, ObjectKind, ObjectRef};

use crate::model::{Fetch, Tree};

/// How many candidates a list keeps; the rest are only counted. Every
/// keyword and phrase of a dialect fits, so a list opened by hand on an
/// empty word is whole.
pub const KEPT: usize = 120;

/// How many of a statement's tables have their columns fetched.
pub const TABLES: usize = 8;

#[cfg(test)]
thread_local! {
    /// How many times this thread worked out a list.
    pub static LISTED: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

/// What a candidate is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Keyword,
    Schema,
    Table,
    View,
    MaterializedView,
    Column,
}

/// One row of a completion list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Candidate {
    pub kind: Kind,
    /// What the row shows.
    pub label: String,
    /// What accepting inserts in place of the word.
    pub insert: String,
    /// The part of `label` that matched what was typed: a byte range on
    /// character boundaries of `label`.
    pub matched: Range<usize>,
    /// A column's type. Empty for every other kind.
    pub detail: String,
}

impl Candidate {
    /// Whether accepting this row would only repeat what is typed: a
    /// keyword whatever its case (`From` is `FROM`), any other name as
    /// spelled (`Users` and `users` can be two tables, and quotes change
    /// the text).
    pub fn is_typed(&self, typed: &str) -> bool {
        match self.kind {
            Kind::Keyword => self.insert.eq_ignore_ascii_case(typed),
            _ => self.insert == typed,
        }
    }
}

/// What the workspace knows, as a list reads it.
pub struct Catalog<'a> {
    pub dialect: Dialect,
    pub tree: &'a Tree,
    /// The schema a bare name is looked up in (`public`, the connection's
    /// database, `main`), when the tree has it.
    pub bare: Option<&'a str>,
    /// The schemas the sidebar shows.
    pub schemas: &'a [String],
    /// The columns of the tables and views whose columns were fetched.
    pub columns: &'a HashMap<ObjectRef, Fetch<Vec<ColumnInfo>>>,
}

/// Names a list reads that the workspace may not have loaded yet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Need {
    /// The tables and views of this schema.
    Objects(String),
    /// The columns of this table or view.
    Columns(ObjectRef),
}

/// A list's rows, best first, and how many more matched.
pub struct Listed {
    pub candidates: Vec<Candidate>,
    pub more: usize,
}

impl Listed {
    /// Whether the list would only say what is already there: one row,
    /// none more, and it is what is `typed`.
    pub fn only_repeats(&self, typed: &str) -> bool {
        self.more == 0 && matches!(self.candidates.as_slice(), [only] if only.is_typed(typed))
    }
}

/// Whether a list may open at `site` for the `typed` part of its word,
/// before any candidate is looked for: always when asked for by hand
/// (`manual`); by typing, on a word of two characters or more, or right
/// after a dot, and not where a new name goes (most likely an alias).
pub fn may_open(site: &Site, typed: &str, manual: bool) -> bool {
    if manual {
        return true;
    }
    let after_dot = typed.is_empty() && !site.qualifier.is_empty();
    let new_name = site.expects == Expects::Name && site.qualifier.is_empty();
    (typed.chars().count() >= 2 || after_dot) && !new_name
}

/// Where a kind of candidate comes in a site's list: lower first.
type Group = u8;

/// The bare schema's tables; after a dot, the named schema's.
const BARE: Group = 0;
/// The columns of the statement's tables, or of the table before the dot.
const COLUMNS: Group = 0;
/// The other schemas' tables, qualified.
const QUALIFIED: Group = 1;
const SCHEMAS: Group = 2;
const CTES: Group = 3;
/// Keywords come after every name.
const KEYWORDS: Group = 9;

/// What a site asks for, once its qualifier is read.
enum Target<'a> {
    Nothing,
    Keywords,
    /// Tables and views of every loaded schema, schemas and CTE names.
    Tables,
    /// The tables and views of this schema: the site is after `schema.`.
    SchemaTables(&'a str),
    /// The columns of these tables, and then keywords when the site is
    /// not after a dot.
    Columns {
        objects: Vec<ObjectRef>,
        keywords: bool,
    },
}

/// What `site` asks for. `manual` says the list was asked for by hand.
/// Keywords belong neither after a dot nor where a table goes, and where
/// a new name goes only when asked for by hand.
fn target<'a>(site: &Site, manual: bool, catalog: &Catalog<'a>) -> Target<'a> {
    // The columns of the one table a qualifier names, if the workspace
    // knows it.
    let of = |object: Option<ObjectRef>| Target::Columns {
        objects: object.into_iter().collect(),
        keywords: false,
    };
    match site.qualifier.as_slice() {
        [] => match site.expects {
            Expects::Start => Target::Keywords,
            Expects::Columns => Target::Columns {
                objects: sources(site, catalog),
                keywords: true,
            },
            Expects::Name if manual => Target::Keywords,
            Expects::Name => Target::Nothing,
            Expects::Tables => Target::Tables,
        },
        // A source's alias first, then a source's own name, then a schema.
        [one] => {
            let alias = |source: &&Source| {
                let alias = source.alias.as_deref();
                alias.is_some_and(|alias| alias.eq_ignore_ascii_case(one))
            };
            let table = |source: &&Source| source.name.eq_ignore_ascii_case(one);
            let mut named = site.sources.iter();
            let found = named
                .find(alias)
                .or_else(|| site.sources.iter().find(table));
            let object =
                found.and_then(|source| resolve(source.schema.as_deref(), &source.name, catalog));
            match (object, schema_named(catalog, one)) {
                (Some(object), _) => of(Some(object)),
                // A half-typed `FROM billing.` reads as a table called
                // `billing`, which is none: a schema is what it is.
                (None, Some(schema)) => Target::SchemaTables(schema),
                // A table the workspace does not know (yet).
                (None, None) if found.is_some() => of(None),
                (None, None) => Target::Nothing,
            }
        }
        [schema, table] => of(resolve(Some(schema), table, catalog)),
        _ => Target::Nothing,
    }
}

/// The loaded table or view a statement names: `schema.name` in that
/// schema, a bare name in the bare schema and then in the other loaded
/// ones, in their order. The name as spelled first, else whatever its
/// case.
fn resolve(schema: Option<&str>, name: &str, catalog: &Catalog<'_>) -> Option<ObjectRef> {
    let within = |schema: &str| {
        let objects = objects_of(catalog, schema);
        let spelled = objects.iter().find(|info| info.name == name);
        let any_case = || {
            objects
                .iter()
                .find(|info| info.name.eq_ignore_ascii_case(name))
        };
        let found = spelled.or_else(any_case)?;
        Some(ObjectRef::new(schema, found.name.as_str()))
    };
    match schema {
        Some(schema) => within(schema_named(catalog, schema)?),
        None => catalog.bare.and_then(within).or_else(|| {
            let mut loaded: Vec<&String> = catalog.tree.nodes.keys().collect();
            loaded.sort();
            loaded.into_iter().find_map(|schema| within(schema))
        }),
    }
}

/// The statement's tables the workspace knows, each once, the first
/// [`TABLES`] of them.
fn sources(site: &Site, catalog: &Catalog<'_>) -> Vec<ObjectRef> {
    let mut objects: Vec<ObjectRef> = Vec::new();
    for source in &site.sources {
        if objects.len() == TABLES {
            break;
        }
        if let Some(object) = resolve(source.schema.as_deref(), &source.name, catalog)
            && !objects.contains(&object)
        {
            objects.push(object);
        }
    }
    objects
}

/// The shown schema called `name`: as spelled, else whatever its case.
fn schema_named<'a>(catalog: &Catalog<'a>, name: &str) -> Option<&'a str> {
    let schemas = catalog.schemas;
    let spelled = schemas.iter().find(|schema| *schema == name);
    let any_case = || {
        schemas
            .iter()
            .find(|schema| schema.eq_ignore_ascii_case(name))
    };
    spelled.or_else(any_case).map(String::as_str)
}

/// The loaded tables and views of `schema`; none while they are not.
fn objects_of<'a>(catalog: &Catalog<'a>, schema: &str) -> &'a [ObjectInfo] {
    let node = catalog.tree.nodes.get(schema);
    node.and_then(|node| node.objects.value.as_deref())
        .unwrap_or_default()
}

/// The names `site` reads that may still have to be loaded: the caller
/// asks for the ones the workspace never loaded.
pub fn needs(site: &Site, catalog: &Catalog<'_>) -> Vec<Need> {
    let schema_objects = |schema: &str| Need::Objects(schema.to_owned());
    // Asked for by hand or not, a site reads the same names.
    match target(site, true, catalog) {
        Target::Nothing | Target::Keywords => Vec::new(),
        Target::Tables => catalog.bare.map(schema_objects).into_iter().collect(),
        Target::SchemaTables(schema) => vec![schema_objects(schema)],
        Target::Columns { objects, .. } => {
            // The statement's tables are matched against loaded objects:
            // the bare schema's, and those of a `schema.table.` qualifier.
            let mut needs: Vec<Need> = catalog.bare.map(schema_objects).into_iter().collect();
            if let [schema, _] = site.qualifier.as_slice()
                && let Some(schema) = schema_named(catalog, schema)
                && catalog.bare != Some(schema)
            {
                needs.push(schema_objects(schema));
            }
            needs.extend(objects.into_iter().map(Need::Columns));
            needs
        }
    }
}

/// What `site` offers for the `typed` part of its word, from `catalog`.
/// `manual` says the list was asked for by hand.
pub fn list(site: &Site, typed: &str, manual: bool, catalog: &Catalog<'_>) -> Listed {
    #[cfg(test)]
    LISTED.with(|count| count.set(count.get() + 1));
    let dialect = catalog.dialect;
    let mut found: Vec<(Group, Candidate)> = Vec::new();
    match target(site, manual, catalog) {
        Target::Nothing => {}
        Target::Keywords => keywords(&mut found, typed, dialect),
        Target::Columns {
            objects,
            keywords: with_keywords,
        } => {
            // A column several tables share is offered once.
            let mut seen = HashSet::new();
            for object in &objects {
                let fetched = catalog.columns.get(object);
                let columns = fetched.and_then(|fetched| fetched.value.as_deref());
                for info in columns.unwrap_or_default() {
                    if seen.insert(info.name.as_str())
                        && let Some(candidate) = column(info, typed, dialect)
                    {
                        found.push((COLUMNS, candidate));
                    }
                }
            }
            if with_keywords {
                keywords(&mut found, typed, dialect);
            }
        }
        Target::Tables => {
            // In the schemas' order: rows that read the same keep one order.
            let mut loaded: Vec<&String> = catalog.tree.nodes.keys().collect();
            loaded.sort();
            for schema in loaded {
                let (group, qualifier) = if catalog.bare == Some(schema.as_str()) {
                    (BARE, None)
                } else {
                    (QUALIFIED, Some(schema.as_str()))
                };
                let objects = objects_of(catalog, schema).iter();
                let objects = objects.filter_map(|info| object(info, qualifier, typed, dialect));
                found.extend(objects.map(|candidate| (group, candidate)));
            }
            let schemas = catalog.schemas.iter();
            let schemas = schemas.filter_map(|schema| name(schema, Kind::Schema, typed, dialect));
            found.extend(schemas.map(|candidate| (SCHEMAS, candidate)));
            let ctes = site.ctes.iter();
            let ctes = ctes.filter_map(|cte| name(cte, Kind::Table, typed, dialect));
            found.extend(ctes.map(|candidate| (CTES, candidate)));
        }
        Target::SchemaTables(schema) => {
            let objects = objects_of(catalog, schema).iter();
            let objects = objects.filter_map(|info| object(info, None, typed, dialect));
            found.extend(objects.map(|candidate| (BARE, candidate)));
        }
    }
    rank(found, typed)
}

/// A table or view as a candidate when it holds `typed`: bare, or as
/// `schema.name` when it lives outside the bare schema.
fn object(
    info: &ObjectInfo,
    schema: Option<&str>,
    typed: &str,
    dialect: Dialect,
) -> Option<Candidate> {
    let label = match schema {
        Some(schema) => format!("{schema}.{}", info.name),
        None => info.name.clone(),
    };
    let matched = find_ignoring_case(&label, typed)?;
    let insert = match schema {
        Some(schema) => format!("{}.{}", dialect.ident(schema), dialect.ident(&info.name)),
        None => dialect.ident(&info.name).into_owned(),
    };
    let kind = match info.kind {
        ObjectKind::Table => Kind::Table,
        ObjectKind::View => Kind::View,
        ObjectKind::MaterializedView => Kind::MaterializedView,
    };
    Some(Candidate {
        kind,
        label,
        insert,
        matched,
        detail: String::new(),
    })
}

/// Adds the keywords and phrases that start with `typed`.
fn keywords(found: &mut Vec<(Group, Candidate)>, typed: &str, dialect: Dialect) {
    let words = tabletist_db::sql::keywords(dialect).chain(PHRASES);
    let words = words.filter_map(|word| keyword(word, typed));
    found.extend(words.map(|candidate| (KEYWORDS, candidate)));
}

/// A column as a candidate when its name holds `typed`, with its type.
fn column(info: &ColumnInfo, typed: &str, dialect: Dialect) -> Option<Candidate> {
    Some(Candidate {
        kind: Kind::Column,
        label: info.name.clone(),
        insert: dialect.ident(&info.name).into_owned(),
        matched: find_ignoring_case(&info.name, typed)?,
        detail: info.type_name.clone(),
    })
}

/// A schema or a CTE as a candidate when its name holds `typed`.
fn name(name: &str, kind: Kind, typed: &str, dialect: Dialect) -> Option<Candidate> {
    Some(Candidate {
        kind,
        label: name.to_owned(),
        insert: dialect.ident(name).into_owned(),
        matched: find_ignoring_case(name, typed)?,
        detail: String::new(),
    })
}

/// `word` as a candidate when it starts with `typed`, in the case typed:
/// lower case when every typed letter is lower case, upper case otherwise.
fn keyword(word: &str, typed: &str) -> Option<Candidate> {
    let matched = find_ignoring_case(word, typed).filter(|found| found.start == 0)?;
    let lower = typed.chars().any(char::is_lowercase) && !typed.chars().any(char::is_uppercase);
    let label = if lower {
        word.to_lowercase()
    } else {
        word.to_owned()
    };
    Some(Candidate {
        kind: Kind::Keyword,
        insert: label.clone(),
        label,
        matched,
        detail: String::new(),
    })
}

/// Where `typed` first occurs in `label`, whatever the case of either:
/// both are lower-cased character by character; a character whose
/// lower-case form is several characters matches only as a whole.
fn find_ignoring_case(label: &str, typed: &str) -> Option<Range<usize>> {
    let needle: Vec<char> = typed.chars().flat_map(char::to_lowercase).collect();
    if needle.is_empty() {
        return Some(0..0);
    }
    label.char_indices().find_map(|(start, _)| {
        let (mut matched, mut end) = (0, start);
        for (offset, character) in label[start..].char_indices() {
            if matched == needle.len() {
                break;
            }
            for lower in character.to_lowercase() {
                if needle.get(matched) != Some(&lower) {
                    return None;
                }
                matched += 1;
            }
            end = start + offset + character.len_utf8();
        }
        (matched == needle.len()).then_some(start..end)
    })
}

/// `found` in a list's order, cut at [`KEPT`]: a candidate that
/// [`is_typed`](Candidate::is_typed) first, then by group, then names that start with
/// what is typed before names that only hold it, then by name.
fn rank(mut found: Vec<(Group, Candidate)>, typed: &str) -> Listed {
    found.sort_by_cached_key(|(group, candidate)| {
        (
            !candidate.is_typed(typed),
            *group,
            candidate.matched.start != 0,
            candidate.label.to_lowercase(),
            candidate.label.clone(),
        )
    });
    let more = found.len().saturating_sub(KEPT);
    found.truncate(KEPT);
    Listed {
        candidates: found.into_iter().map(|(_, candidate)| candidate).collect(),
        more,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::SchemaNode;
    use std::sync::LazyLock;
    use tabletist_db::sql::tokenize;

    type Columns = HashMap<ObjectRef, Fetch<Vec<ColumnInfo>>>;

    /// No columns known.
    static NO_COLUMNS: LazyLock<Columns> = LazyLock::new(Columns::new);

    fn catalog<'a>(
        dialect: Dialect,
        tree: &'a Tree,
        bare: Option<&'a str>,
        schemas: &'a [String],
    ) -> Catalog<'a> {
        Catalog {
            dialect,
            tree,
            bare,
            schemas,
            columns: &NO_COLUMNS,
        }
    }

    /// A schema, a table, and its columns: each a name and a type.
    type TableColumns<'a> = (&'a str, &'a str, &'a [(&'a str, &'a str)]);

    /// A column cache holding these tables' columns.
    fn columns_of(tables: &[TableColumns<'_>]) -> Columns {
        let mut kept = Columns::new();
        for (schema, table, columns) in tables {
            let columns = columns
                .iter()
                .map(|(name, type_name)| ColumnInfo {
                    name: (*name).to_owned(),
                    type_name: (*type_name).to_owned(),
                    ..Default::default()
                })
                .collect();
            let fetched = Fetch {
                value: Some(columns),
                ..Default::default()
            };
            kept.insert(ObjectRef::new(*schema, *table), fetched);
        }
        kept
    }

    fn shop_columns() -> Columns {
        columns_of(&[
            (
                "public",
                "users",
                &[
                    ("id", "integer"),
                    ("name", "text"),
                    ("Created At", "timestamptz"),
                ],
            ),
            (
                "billing",
                "invoices",
                &[("id", "integer"), ("total", "numeric")],
            ),
        ])
    }

    /// What a PostgreSQL list offers at `marked` with the shop's columns
    /// known.
    fn offered_columns(marked: &str) -> Vec<Candidate> {
        let (tree, schemas) = shop();
        let columns = shop_columns();
        let (site, typed) = site_at(marked);
        let catalog = Catalog {
            columns: &columns,
            ..catalog(Dialect::Postgres, &tree, Some("public"), &schemas)
        };
        list(&site, &typed, false, &catalog).candidates
    }

    fn labels_of(candidates: &[Candidate]) -> Vec<&str> {
        candidates.iter().map(|c| c.label.as_str()).collect()
    }

    /// A tree with these schemas loaded, each with its objects.
    fn tree_of(schemas: &[(&str, &[(&str, ObjectKind)])]) -> Tree {
        let mut tree = Tree::default();
        for (schema, objects) in schemas {
            let objects = objects
                .iter()
                .map(|(name, kind)| ObjectInfo {
                    name: (*name).to_owned(),
                    kind: *kind,
                    estimated_rows: None,
                })
                .collect();
            let node = SchemaNode {
                objects: Fetch {
                    value: Some(objects),
                    ..Default::default()
                },
                ..Default::default()
            };
            tree.nodes.insert((*schema).to_owned(), node);
        }
        tree
    }

    fn shop() -> (Tree, Vec<String>) {
        use ObjectKind::{MaterializedView, Table, View};
        let tree = tree_of(&[
            (
                "public",
                &[
                    ("users", Table),
                    ("active_users", View),
                    ("Order Items", Table),
                    ("user_totals", MaterializedView),
                ],
            ),
            ("billing", &[("invoices", Table), ("users", Table)]),
        ]);
        (tree, vec!["billing".to_owned(), "public".to_owned()])
    }

    /// `(label, insert, kind)` of what a PostgreSQL list offers at `marked`.
    fn offered(marked: &str, bare: Option<&str>) -> Vec<(String, String, Kind)> {
        let (tree, schemas) = shop();
        let (site, typed) = site_at(marked);
        let catalog = catalog(Dialect::Postgres, &tree, bare, &schemas);
        list(&site, &typed, false, &catalog)
            .candidates
            .into_iter()
            .map(|c| (c.label, c.insert, c.kind))
            .collect()
    }

    fn row(label: &str, insert: &str, kind: Kind) -> (String, String, Kind) {
        (label.to_owned(), insert.to_owned(), kind)
    }

    #[test]
    fn a_table_site_offers_tables_schemas_and_cte_names() {
        assert_eq!(
            offered("SELECT * FROM |", Some("public")),
            [
                // The bare schema's, bare.
                row("active_users", "active_users", Kind::View),
                row("Order Items", "\"Order Items\"", Kind::Table),
                row("user_totals", "user_totals", Kind::MaterializedView),
                row("users", "users", Kind::Table),
                // Every other loaded schema's, qualified.
                row("billing.invoices", "billing.invoices", Kind::Table),
                row("billing.users", "billing.users", Kind::Table),
                // The schemas.
                row("billing", "billing", Kind::Schema),
                row("public", "public", Kind::Schema),
            ]
        );
        // Names that start with what is typed lead the ones that hold it.
        assert_eq!(
            offered("SELECT * FROM us|", Some("public")),
            [
                row("user_totals", "user_totals", Kind::MaterializedView),
                row("users", "users", Kind::Table),
                row("active_users", "active_users", Kind::View),
                row("billing.users", "billing.users", Kind::Table),
            ]
        );
        assert_eq!(
            offered(
                "WITH recent AS (SELECT 1) SELECT * FROM rec|",
                Some("public")
            ),
            [row("recent", "recent", Kind::Table)]
        );
    }

    #[test]
    fn nothing_is_bare_without_a_bare_schema() {
        assert_eq!(
            offered("SELECT * FROM inv|", None),
            [row("billing.invoices", "billing.invoices", Kind::Table)]
        );
        let users = offered("SELECT * FROM users|", None);
        assert_eq!(users.len(), 3);
        assert!(users.iter().all(|(label, ..)| label.contains('.')));
    }

    #[test]
    fn a_schema_and_a_dot_offer_that_schemas_tables() {
        assert_eq!(
            offered("SELECT * FROM billing.|", Some("public")),
            [
                row("invoices", "invoices", Kind::Table),
                row("users", "users", Kind::Table),
            ]
        );
        // Whatever the case the schema is typed in.
        assert_eq!(
            offered("SELECT * FROM BILLING.inv|", Some("public")),
            [row("invoices", "invoices", Kind::Table)]
        );
        // Not a schema: nothing.
        assert!(offered("SELECT * FROM nope.|", Some("public")).is_empty());
        // After a dot there are no keywords, wherever it is.
        assert!(offered("SELECT billing.sel|", Some("public")).is_empty());
    }

    #[test]
    fn a_name_that_is_not_a_plain_word_is_inserted_quoted() {
        use ObjectKind::Table;
        let tree = tree_of(&[
            ("Sales Data", &[("order", Table), ("Totals", Table)]),
            ("public", &[("Order Items", Table)]),
        ]);
        let schemas = ["Sales Data".to_owned(), "public".to_owned()];
        let offered = |marked: &str| {
            let (site, typed) = site_at(marked);
            let catalog = catalog(Dialect::Postgres, &tree, Some("public"), &schemas);
            let listed = list(&site, &typed, false, &catalog);
            let rows = listed.candidates.into_iter();
            rows.map(|c| (c.label, c.insert)).collect::<Vec<_>>()
        };
        // Each part of a qualified name is quoted by itself; the row shows
        // the names as they are.
        assert_eq!(
            offered("SELECT * FROM sal|"),
            [
                (
                    "Sales Data.order".to_owned(),
                    "\"Sales Data\".\"order\"".to_owned()
                ),
                (
                    "Sales Data.Totals".to_owned(),
                    "\"Sales Data\".\"Totals\"".to_owned()
                ),
                ("Sales Data".to_owned(), "\"Sales Data\"".to_owned()),
            ]
        );
        // A CTE's name too.
        assert_eq!(
            offered("WITH \"Recent Ones\" AS (SELECT 1) SELECT * FROM rec|"),
            [("Recent Ones".to_owned(), "\"Recent Ones\"".to_owned())]
        );
    }

    #[test]
    fn a_site_needs_the_objects_of_the_schema_it_reads() {
        let (tree, schemas) = shop();
        let needed = |marked: &str, bare| {
            let (site, _) = site_at(marked);
            needs(&site, &catalog(Dialect::Postgres, &tree, bare, &schemas))
        };
        assert_eq!(
            needed("SELECT * FROM |", Some("public")),
            [Need::Objects("public".into())]
        );
        assert_eq!(
            needed("SELECT * FROM billing.|", Some("public")),
            [Need::Objects("billing".into())]
        );
        assert!(needed("SELECT * FROM |", None).is_empty());
        assert!(needed("SEL|", Some("public")).is_empty());
        // Where a new name goes nothing is read, by hand or not.
        assert!(needed("SELECT * FROM users wh|", Some("public")).is_empty());
    }

    #[test]
    fn columns_of_the_statements_tables_come_before_keywords() {
        let found = offered_columns("SELECT na| FROM users");
        assert_eq!(labels_of(&found), ["name", "natural"]);
        assert_eq!(
            (found[0].kind, found[0].detail.as_str()),
            (Kind::Column, "text")
        );
        assert_eq!(found[1].kind, Kind::Keyword);
        // A name that needs quotes is shown bare and inserted quoted.
        let quoted = offered_columns("SELECT cr| FROM users");
        assert_eq!(quoted[0].label, "Created At");
        assert_eq!(quoted[0].insert, "\"Created At\"");
        // A column two tables share is offered once.
        let shared = offered_columns("SELECT id| FROM users u JOIN billing.invoices i ON true");
        let ids = labels_of(&shared)
            .into_iter()
            .filter(|label| *label == "id");
        assert_eq!(ids.count(), 1);
        // A table the workspace does not know has no columns.
        assert_eq!(
            labels_of(&offered_columns("SELECT na| FROM nope")),
            ["natural"]
        );
        // Nor has one whose columns were not fetched (yet), or could not be.
        assert!(offered_columns("SELECT id| FROM active_users").is_empty());
    }

    #[test]
    fn a_qualifier_is_an_alias_then_a_table_then_a_schema() {
        let join = "FROM users u JOIN billing.invoices i ON true";
        assert_eq!(
            labels_of(&offered_columns(&format!("SELECT u.| {join}"))),
            ["Created At", "id", "name"]
        );
        assert_eq!(
            labels_of(&offered_columns(&format!("SELECT i.to| {join}"))),
            ["total"]
        );
        // Whatever the case the alias is typed in.
        assert_eq!(
            labels_of(&offered_columns(&format!("SELECT I.to| {join}"))),
            ["total"]
        );
        // A table's own name.
        assert_eq!(
            labels_of(&offered_columns("SELECT users.na| FROM users")),
            ["name"]
        );
        // An alias is looked for before a table's name: `users` here is the
        // invoices.
        assert_eq!(
            labels_of(&offered_columns(
                "SELECT users.| FROM users u JOIN billing.invoices users ON true"
            )),
            ["id", "total"]
        );
        // An alias spelled like a schema is the alias.
        assert_eq!(
            labels_of(&offered_columns("SELECT billing.| FROM users billing")),
            ["Created At", "id", "name"]
        );
        // A schema, where no table of the statement is called that: its
        // tables, as before. A half-typed `FROM billing.` reads as a table
        // called `billing`, which is none.
        assert_eq!(
            labels_of(&offered_columns("SELECT * FROM billing.|")),
            ["invoices", "users"]
        );
        assert_eq!(
            labels_of(&offered_columns("SELECT billing.| FROM users")),
            ["invoices", "users"]
        );
        // `schema.table.`: that table's columns, named by the statement or not.
        assert_eq!(
            labels_of(&offered_columns("SELECT billing.invoices.| FROM users")),
            ["id", "total"]
        );
        assert!(offered_columns("SELECT billing.nope.| FROM users").is_empty());
        // An alias of a table the workspace does not know: nothing.
        assert!(offered_columns("SELECT x.| FROM nope x").is_empty());
        // After a dot there are no keywords.
        assert_eq!(
            labels_of(&offered_columns("SELECT u.na| FROM users u")),
            ["name"]
        );
    }

    #[test]
    fn a_source_is_matched_as_spelled_and_then_whatever_its_case() {
        use ObjectKind::Table;
        let tree = tree_of(&[
            ("public", &[("Users", Table), ("users", Table)]),
            ("audit", &[("events", Table), ("users", Table)]),
        ]);
        let schemas = ["audit".to_owned(), "public".to_owned()];
        let found = |schema: Option<&str>, name: &str, bare| {
            let catalog = catalog(Dialect::Postgres, &tree, bare, &schemas);
            resolve(schema, name, &catalog)
        };
        let public = Some("public");
        let object = |schema: &str, name: &str| Some(ObjectRef::new(schema, name));
        // As spelled first, else whatever the case.
        assert_eq!(found(None, "users", public), object("public", "users"));
        assert_eq!(found(None, "Users", public), object("public", "Users"));
        assert_eq!(found(None, "USERS", public), object("public", "Users"));
        // A bare name: the bare schema, then the other loaded ones.
        assert_eq!(found(None, "events", public), object("audit", "events"));
        assert_eq!(found(None, "users", None), object("audit", "users"));
        assert_eq!(found(None, "nope", public), None);
        // `schema.name`: in that schema only, whatever the schema's case.
        assert_eq!(
            found(Some("AUDIT"), "users", public),
            object("audit", "users")
        );
        assert_eq!(found(Some("public"), "events", public), None);
        assert_eq!(found(Some("nope"), "users", public), None);
    }

    #[test]
    fn a_column_site_needs_its_tables_columns_up_to_eight() {
        let (tree, schemas) = shop();
        let needed = |marked: &str| {
            let (site, _) = site_at(marked);
            needs(
                &site,
                &catalog(Dialect::Postgres, &tree, Some("public"), &schemas),
            )
        };
        let object = |schema: &str, name: &str| Need::Columns(ObjectRef::new(schema, name));
        assert_eq!(
            needed("SELECT | FROM users u JOIN billing.invoices i ON true"),
            [
                Need::Objects("public".into()),
                object("public", "users"),
                object("billing", "invoices"),
            ]
        );
        // After an alias and a dot: that table's only.
        assert_eq!(
            needed("SELECT i.| FROM users u JOIN billing.invoices i ON true"),
            [
                Need::Objects("public".into()),
                object("billing", "invoices")
            ]
        );
        assert_eq!(
            needed("SELECT billing.invoices.| FROM users"),
            [
                Need::Objects("public".into()),
                Need::Objects("billing".into()),
                object("billing", "invoices"),
            ]
        );
        // The bare schema's own table: its objects are asked for once.
        assert_eq!(
            needed("SELECT public.users.| FROM users"),
            [Need::Objects("public".into()), object("public", "users")]
        );
        // The same table twice is asked for once.
        assert_eq!(
            needed("SELECT | FROM users a JOIN users b ON true").len(),
            2
        );
        // An alias of a table the workspace does not know (yet): the bare
        // schema's objects, which may name it once they are loaded.
        assert_eq!(
            needed("SELECT x.| FROM nope x"),
            [Need::Objects("public".into())]
        );

        let names: Vec<String> = (0..10).map(|index| format!("t{index}")).collect();
        let tables: Vec<(&str, ObjectKind)> = names
            .iter()
            .map(|name| (name.as_str(), ObjectKind::Table))
            .collect();
        let wide = tree_of(&[("public", tables.as_slice())]);
        let (site, _) = site_at(&format!("SELECT | FROM {}", names.join(", ")));
        let schemas = vec!["public".to_owned()];
        let needed = needs(
            &site,
            &catalog(Dialect::Postgres, &wide, Some("public"), &schemas),
        );
        let columns: Vec<&Need> = needed
            .iter()
            .filter(|need| matches!(need, Need::Columns(_)))
            .collect();
        assert_eq!(columns.len(), TABLES);
        // The first of them, in the statement's order.
        assert_eq!(columns[0], &object("public", "t0"));
        assert_eq!(columns[TABLES - 1], &object("public", "t7"));
    }

    /// The site at the `|` of `marked` (read as SQLite), and what is typed
    /// of its word.
    fn site_at(marked: &str) -> (Site, String) {
        let cursor = marked.find('|').expect("a cursor mark");
        let text = marked.replacen('|', "", 1);
        let tokens = tokenize(Dialect::Sqlite, &text);
        let site = tabletist_db::complete::site(&tokens, &text, cursor).expect("a site");
        let typed = text[site.word.start..cursor].to_owned();
        (site, typed)
    }

    fn keywords_at(marked: &str, manual: bool) -> Vec<String> {
        keywords_in(Dialect::Sqlite, marked, manual)
    }

    /// The labels listed at `marked` (its site read as SQLite) when the
    /// catalog is of `dialect`.
    fn keywords_in(dialect: Dialect, marked: &str, manual: bool) -> Vec<String> {
        let (site, typed) = site_at(marked);
        let tree = Tree::default();
        let catalog = catalog(dialect, &tree, None, &[]);
        let listed = list(&site, &typed, manual, &catalog);
        listed.candidates.into_iter().map(|c| c.label).collect()
    }

    fn keyword_row(word: &str) -> Candidate {
        Candidate {
            kind: Kind::Keyword,
            label: word.into(),
            insert: word.into(),
            matched: 0..2,
            detail: String::new(),
        }
    }

    fn name_row(kind: Kind, label: &str, matched: Range<usize>) -> Candidate {
        Candidate {
            kind,
            label: label.into(),
            insert: label.into(),
            matched,
            detail: String::new(),
        }
    }

    #[test]
    fn keywords_follow_the_case_that_is_typed() {
        assert_eq!(keywords_at("sel|", false), ["select"]);
        assert_eq!(keywords_at("SEL|", false), ["SELECT"]);
        assert_eq!(keywords_at("Sel|", false), ["SELECT"]);
        // Nothing typed: upper case, in order, phrases among them.
        let all = keywords_at("|", true);
        assert_eq!(all[0], "ALL");
        assert!(all.contains(&"GROUP BY".to_owned()));
        // Keywords and phrases must fit under KEPT, or the last ones fall
        // out of the list: in every dialect, each with its own words.
        let (site, typed) = site_at("|");
        let tree = Tree::default();
        for dialect in [Dialect::Postgres, Dialect::MySql, Dialect::Sqlite] {
            let catalog = catalog(dialect, &tree, None, &[]);
            let listed = list(&site, &typed, true, &catalog);
            assert_eq!(listed.more, 0, "{dialect:?}");
            let last = listed.candidates.last().map(|c| c.label.as_str());
            assert_eq!(last, Some("WITH"), "{dialect:?}");
        }
    }

    #[test]
    fn the_dialect_of_the_catalog_picks_the_keywords() {
        assert_eq!(keywords_in(Dialect::Sqlite, "prag|", false), ["pragma"]);
        assert!(keywords_in(Dialect::Postgres, "prag|", false).is_empty());
    }

    #[test]
    fn a_keyword_typed_in_any_case_is_what_is_typed() {
        assert_eq!(
            keywords_at("SELECT * FROM users WHERE Is|", false)[..3],
            ["IS", "IS NOT NULL", "IS NULL"]
        );
        assert!(keyword_row("FROM").is_typed("From"));
        assert!(keyword_row("FROM").is_typed("from"));
        assert!(!keyword_row("FROM").is_typed("Fro"));
        // A keyword that is typed leads even names that start the same.
        let found = vec![
            (0, name_row(Kind::Column, "order_id", 0..2)),
            (KEYWORDS, keyword_row("OR")),
        ];
        let labels: Vec<String> = rank(found, "Or")
            .candidates
            .into_iter()
            .map(|c| c.label)
            .collect();
        assert_eq!(labels, ["OR", "order_id"]);
    }

    #[test]
    fn a_name_is_what_is_typed_only_as_spelled() {
        // `Users` and `users` can be two tables, and quotes change the text.
        let users = name_row(Kind::Table, "Users", 0..5);
        assert!(!users.is_typed("users"));
        assert!(users.is_typed("Users"));
        assert!(name_row(Kind::Table, "users", 0..5).is_typed("users"));
        let found = vec![
            (1, users),
            (0, name_row(Kind::Table, "users_archive", 0..5)),
        ];
        let labels: Vec<String> = rank(found, "users")
            .candidates
            .into_iter()
            .map(|c| c.label)
            .collect();
        assert_eq!(labels, ["users_archive", "Users"]);
    }

    #[test]
    fn names_are_ordered_ignoring_case() {
        let found = vec![
            (0, name_row(Kind::Table, "Zebra", 0..0)),
            (0, name_row(Kind::Table, "apple", 0..0)),
        ];
        let labels: Vec<String> = rank(found, "")
            .candidates
            .into_iter()
            .map(|c| c.label)
            .collect();
        assert_eq!(labels, ["apple", "Zebra"]);
    }

    #[test]
    fn keywords_match_from_their_start_only() {
        // Not JOIN or UNION, which only hold `in`. The exact match leads.
        assert_eq!(
            keywords_at("SELECT * FROM users WHERE in|", false),
            [
                "in",
                "inner",
                "inner join",
                "insert",
                "intersect",
                "interval",
                "into"
            ]
        );
        assert_eq!(keywords_at("gr|", false), ["group", "group by"]);
    }

    #[test]
    fn a_keyword_is_inserted_as_it_reads_and_marks_what_matched() {
        let (site, typed) = site_at("sel|");
        let tree = Tree::default();
        let catalog = catalog(Dialect::Sqlite, &tree, None, &[]);
        let listed = list(&site, &typed, false, &catalog);
        assert_eq!(
            listed.candidates,
            [Candidate {
                kind: Kind::Keyword,
                label: "select".into(),
                insert: "select".into(),
                matched: 0..3,
                detail: String::new(),
            }]
        );
        assert_eq!(listed.more, 0);
    }

    #[test]
    fn no_keywords_where_a_table_or_a_new_name_belongs() {
        assert!(keywords_at("SELECT * FROM se|", false).is_empty());
        // A new name: keywords only when the list was asked for by hand.
        assert!(keywords_at("SELECT * FROM users wh|", false).is_empty());
        assert_eq!(
            keywords_at("SELECT * FROM users wh|", true),
            ["when", "where"]
        );
        // After a dot: a name of that table or schema, never a keyword.
        assert!(keywords_at("SELECT u.se|", false).is_empty());
    }

    #[test]
    fn a_list_may_open_on_two_letters_after_a_dot_or_by_hand() {
        let opens = |marked: &str, manual: bool| {
            let (site, typed) = site_at(marked);
            may_open(&site, &typed, manual)
        };
        assert!(!opens("s|", false));
        assert!(opens("se|", false));
        // Characters, not bytes.
        assert!(!opens("ż|", false));
        assert!(opens("żó|", false));
        // Right after a dot, before anything is typed. One letter there is
        // one letter anywhere.
        assert!(opens("SELECT u.|", false));
        assert!(!opens("SELECT u.n|", false));
        assert!(opens("SELECT u.na|", false));
        // Not after a space, with nothing typed.
        assert!(!opens("SELECT |", false));
        // Where a new name goes (most likely an alias): not by typing.
        assert!(!opens("SELECT * FROM users wh|", false));
        assert!(!opens("SELECT 1 AS to|", false));
        // A name after a dot is a name that exists.
        let (mut site, _) = site_at("SELECT * FROM users wh|");
        assert_eq!(site.expects, Expects::Name);
        site.qualifier = vec!["u".to_owned()];
        assert!(may_open(&site, "", false));
        assert!(may_open(&site, "wh", false));
        // By hand: anywhere.
        for marked in ["|", "s|", "SELECT |", "SELECT * FROM users wh|"] {
            assert!(opens(marked, true), "{marked}");
        }
    }

    #[test]
    fn a_list_only_repeats_what_is_typed_when_its_one_row_is_that() {
        let listed = |candidates: Vec<Candidate>, more: usize| Listed { candidates, more };
        assert!(listed(vec![keyword_row("set")], 0).only_repeats("set"));
        // A keyword, whatever the case it is typed in.
        assert!(listed(vec![keyword_row("SET")], 0).only_repeats("Set"));
        // A name only as spelled.
        let users = || vec![name_row(Kind::Table, "Users", 0..5)];
        assert!(listed(users(), 0).only_repeats("Users"));
        assert!(!listed(users(), 0).only_repeats("users"));
        // Two rows, the first of them what is typed.
        let two = vec![keyword_row("as"), keyword_row("asc")];
        assert!(!listed(two, 0).only_repeats("as"));
        // One row kept and more counted.
        assert!(!listed(vec![keyword_row("set")], 1).only_repeats("set"));
        // One row that goes on from what is typed, and no row.
        assert!(!listed(vec![keyword_row("select")], 0).only_repeats("sel"));
        assert!(!listed(Vec::new(), 0).only_repeats(""));
    }

    #[test]
    fn a_list_keeps_the_best_and_counts_the_rest() {
        let found = (0..150)
            .map(|index| {
                let label = format!("name_{index:03}");
                let candidate = Candidate {
                    kind: Kind::Table,
                    insert: label.clone(),
                    label,
                    matched: 0..2,
                    detail: String::new(),
                };
                (0, candidate)
            })
            .rev()
            .collect();
        let listed = rank(found, "na");
        assert_eq!(listed.candidates.len(), KEPT);
        assert_eq!(listed.more, 150 - KEPT);
        assert_eq!(listed.candidates[0].label, "name_000");
        let last = KEPT - 1;
        assert_eq!(listed.candidates[last].label, format!("name_{last:03}"));
    }

    #[test]
    fn candidates_are_ordered_exact_then_group_then_start_then_name() {
        let named = |label: &str, group: Group, matched: Range<usize>| {
            let candidate = Candidate {
                kind: Kind::Column,
                label: label.into(),
                insert: label.into(),
                matched,
                detail: String::new(),
            };
            (group, candidate)
        };
        let found = vec![
            named("order", 1, 0..2),
            named("author", 0, 4..6),
            named("origin", 0, 0..2),
            named("Oracle", 0, 0..2),
            named("or", 1, 0..2),
        ];
        let labels: Vec<String> = rank(found, "or")
            .candidates
            .into_iter()
            .map(|c| c.label)
            .collect();
        assert_eq!(labels, ["or", "Oracle", "origin", "author", "order"]);
    }

    #[test]
    fn text_is_found_whatever_its_case() {
        assert_eq!(find_ignoring_case("first_name", "NAME"), Some(6..10));
        assert_eq!(find_ignoring_case("Żółw", "żó"), Some(0..4));
        assert_eq!(find_ignoring_case("abc", "x"), None);
        assert_eq!(find_ignoring_case("abc", "abcd"), None);
        assert_eq!(find_ignoring_case("abc", ""), Some(0..0));
        // Lower-cased character by character, not case-folded: an `İ`
        // lower-cases to two characters and matches only as a whole.
        assert_eq!(find_ignoring_case("İstanbul", "i"), None);
        assert_eq!(find_ignoring_case("İstanbul", "İ"), Some(0..2));
    }
}

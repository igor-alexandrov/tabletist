//! Hidden tables: the tables of a connection an agent may not read, and
//! the checks that keep it from them (the MCP plan's B.3).
//!
//! A connection's [`Tables`] says which tables the person hides: none, all
//! but a list, or every one but a list. [`Hidden`] is what that comes to
//! on a database, from its [`Catalog`]: a view over a hidden table is
//! hidden with it, and so is a partition or a child table of one, and
//! while anything is hidden the system catalogs are too, since they
//! describe every table.
//!
//! Two checks use it. [`Hidden::check_statement`] reads the names the
//! statement's own text gives, off its tokens, before anything is sent:
//! it gives the agent a sentence that names what it asked for, and it is
//! the one that sees a hidden view whose tables are not. It is not the
//! fence: a reading of the text is never trusted alone (B.2). The fence is
//! what the engine says it reads, which each driver asks of it and hands
//! to [`Hidden::check_engine_read`]: SQLite's authorizer, and the plan of
//! the statement on PostgreSQL and MySQL.
//!
//! Names are compared without regard to case, which is the strict reading
//! where a database tells case apart: a quoted `"Customers"` on PostgreSQL
//! is taken for `customers` too, and refused where that is hidden.
//!
//! The masked columns are checked with the hidden tables ([`masked`]): a
//! connection that masks a column closes the system catalogs and the
//! users' functions as one that hides a table does, and a view that reads
//! a masked column is refused.
//!
//! The person is not the only one with a say. An administrator's policy
//! may hide tables and mask columns too, a rule at a time ([`Sources`]):
//! a table is visible only where each lets it be seen, a column is masked
//! where any masks it, and each refusal says whose entry caused it ([`By`]),
//! so that an agent does not ask the person for a change they cannot make.

mod masked;

use std::collections::{HashMap, HashSet};

pub use masked::ColumnName;
use masked::Masks;

use super::{MaskedUse, Read, Refusal};
use crate::Dialect;
use crate::sql::{self, TokenKind};

/// What agents may read of a connection's tables.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum Tables {
    /// Every table.
    #[default]
    All,
    /// Every table but these.
    AllExcept(Vec<TableName>),
    /// These tables and no others, the ones made later included.
    Only(Vec<TableName>),
}

/// Whose setting hides a table or masks a column.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum By {
    /// The person's own, in their connection's settings.
    #[default]
    Person,
    /// A rule of the administrator's policy, which the person cannot change.
    Administrator,
}

impl By {
    /// Where a refusal's sentence says the setting is.
    pub(crate) fn whose(self) -> &'static str {
        match self {
            Self::Person => "on this connection",
            Self::Administrator => "by your administrator's policy",
        }
    }
}

/// One place's say on what agents may read of a connection: the person's
/// own settings, or one rule of the administrator's policy.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Source {
    pub tables: Tables,
    pub masked: Vec<ColumnName>,
}

impl Source {
    /// Whether it hides a table or masks a column.
    pub fn guards_anything(&self) -> bool {
        self.tables.hides_anything() || !self.masked.is_empty()
    }
}

/// Every place's say on what agents may read of a connection: the
/// person's, and each rule of the administrator's policy that matches it.
/// A table is visible only where each lets it be seen; a column is masked
/// where any masks it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Sources {
    pub person: Source,
    pub administrator: Vec<Source>,
}

impl Sources {
    /// The person's own settings, and no rule.
    pub fn person(tables: Tables, masked: Vec<ColumnName>) -> Self {
        Self {
            person: Source { tables, masked },
            administrator: Vec::new(),
        }
    }

    /// Whether any source hides a table.
    pub fn hides_anything(&self) -> bool {
        self.each()
            .any(|(_, source)| source.tables.hides_anything())
    }

    /// Whether any source hides a table or masks a column: then the
    /// catalog is read, and the statements are checked.
    pub fn guards_anything(&self) -> bool {
        self.each().any(|(_, source)| source.guards_anything())
    }

    /// Every source's masked columns, joined: the administrator's first,
    /// so that a column both mask is refused as theirs.
    pub fn masked(&self) -> Vec<ColumnName> {
        self.masked_by().map(|(_, entry)| entry.clone()).collect()
    }

    /// Each masked column's entry, with whose it is, in [`Self::masked`]'s
    /// order.
    fn masked_by(&self) -> impl Iterator<Item = (By, &ColumnName)> {
        self.each()
            .flat_map(|(by, source)| source.masked.iter().map(move |entry| (by, entry)))
    }

    /// Each source with whose it is, the administrator's first.
    fn each(&self) -> impl Iterator<Item = (By, &Source)> {
        self.administrator
            .iter()
            .map(|source| (By::Administrator, source))
            .chain([(By::Person, &self.person)])
    }
}

/// A table as the person names it: in its schema, or by its name alone,
/// which is that name in every schema.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TableName {
    pub schema: Option<String>,
    pub name: String,
}

/// As the person writes it: `customers`, or `sales.customers`. A part that
/// holds a dot or a double quote is written in double quotes, a quote in it
/// doubled.
impl std::fmt::Display for TableName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let part = |part: &str| match part.contains(['.', '"']) {
            true => format!("\"{}\"", part.replace('"', "\"\"")),
            false => part.to_owned(),
        };
        match &self.schema {
            Some(schema) => write!(f, "{}.{}", part(schema), part(&self.name)),
            None => f.write_str(&part(&self.name)),
        }
    }
}

/// An entry as the person writes it, as [`TableName`]'s `Display` writes it
/// back: a name, or a schema and a name with a dot between, a part in
/// double quotes where it holds a dot or a quote. The error says what is
/// wrong without the entry's words, for whoever gives it a place.
impl std::str::FromStr for TableName {
    type Err = String;

    fn from_str(written: &str) -> Result<Self, String> {
        match <[String; 2]>::try_from(parts(written)?) {
            Ok([schema, name]) => Ok(Self {
                schema: Some(schema),
                name,
            }),
            Err(parts) => match <[String; 1]>::try_from(parts) {
                Ok([name]) => Ok(Self { schema: None, name }),
                Err(_) => {
                    Err("has more than one dot: it is a table, or a schema and a table".into())
                }
            },
        }
    }
}

/// The parts of an entry as the person writes it, joined by dots, a part
/// in double quotes where it holds a dot or a quote. The error says what
/// is wrong without the entry's words.
fn parts(written: &str) -> Result<Vec<String>, String> {
    if written.is_empty() {
        return Err("is empty".into());
    }
    let mut parts: Vec<String> = Vec::new();
    let mut chars = written.chars().peekable();
    loop {
        let mut part = String::new();
        let quoted = chars.peek() == Some(&'"');
        if quoted {
            chars.next();
            loop {
                match chars.next() {
                    Some('"') if chars.peek() == Some(&'"') => {
                        chars.next();
                        part.push('"');
                    }
                    Some('"') => break,
                    Some(c) => part.push(c),
                    None => return Err("has a quote that is not closed".into()),
                }
            }
        } else {
            while let Some(&c) = chars.peek()
                && c != '.'
            {
                if c == '"' {
                    return Err("has a quote inside a name: put the whole name in quotes".into());
                }
                part.push(c);
                chars.next();
            }
        }
        match (part.is_empty(), quoted) {
            (true, true) => return Err("has a quoted name that is empty".into()),
            (true, false) => return Err("has nothing before or after a dot".into()),
            (false, _) => {}
        }
        parts.push(part);
        match chars.next() {
            None => break,
            Some('.') => {}
            Some(_) => {
                return Err("goes on after a quoted name: a dot comes next".into());
            }
        }
    }
    Ok(parts)
}

impl TableName {
    /// Whether this names the relation `name` of `schema`.
    fn names(&self, schema: Option<&str>, name: &str) -> bool {
        same(&self.name, name)
            && match (&self.schema, schema) {
                (None, _) => true,
                (Some(own), Some(schema)) => same(own, schema),
                // A relation without its schema, of a statement that did
                // not give one, may be in any: it is taken for this one.
                (Some(_), None) => true,
            }
    }

    /// Whether this names one of `dialect`'s system catalogs, which no
    /// setting hides and whose absence stops no agent: what the editor
    /// leaves unmarked is what [`Hidden::check_settings`] passes.
    pub fn is_system(&self, dialect: Dialect) -> bool {
        system_catalog(dialect, self.schema.as_deref(), &self.name)
    }
}

impl Tables {
    /// Whether any table is hidden. Nothing is checked, and nothing more
    /// is asked of the database, on a connection that hides nothing.
    pub fn hides_anything(&self) -> bool {
        match self {
            Self::All => false,
            Self::AllExcept(names) => !names.is_empty(),
            Self::Only(_) => true,
        }
    }

    /// Whether the person hides the relation `name` of `schema` by its own
    /// name, before what it reads is looked at.
    fn hides(&self, schema: Option<&str>, name: &str) -> bool {
        match self {
            Self::All => false,
            Self::AllExcept(names) => names.iter().any(|named| named.names(schema, name)),
            Self::Only(names) => !names.iter().any(|named| named.names(schema, name)),
        }
    }
}

/// A table, a view, a materialized view or a foreign table of the
/// database, as its catalog lists it, with the relations it depends on: a
/// view's and a materialized view's tables, and a partition's or a child
/// table's parent. A system catalog is not listed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Relation {
    pub schema: String,
    pub name: String,
    /// By schema and name. One the catalog does not list is passed over.
    pub depends_on: Vec<(String, String)>,
    /// Whether what it reads cannot be known: a view on a MySQL that does
    /// not say what its views read, and a view or a table with a policy
    /// that calls a function the users wrote. Hidden while anything is.
    pub opaque: bool,
    /// Whether it is part of the relation it depends on: a partition of a
    /// partitioned table, or a table a virtual table keeps its data in
    /// (SQLite's shadow tables). Hidden with that one, and read with it,
    /// not hidden for want of its own name in an "Only" list.
    pub part: bool,
    /// Whether it is a view or a materialized view, whose own data is
    /// what it reads: one that reads a masked column is refused, where a
    /// partition or a child table of a table with one has it masked too.
    pub view: bool,
}

/// A column a view reads, as a catalog that says so column by column
/// gives it: the view by its schema and name, the column by its table's
/// schema and name and its own.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ColumnRead {
    pub schema: String,
    pub view: String,
    pub table_schema: String,
    pub table: String,
    pub column: String,
}

/// What a driver reads of its database's catalog for the checks: its
/// relations, and the functions and procedures its users wrote.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Catalog {
    pub relations: Vec<Relation>,
    /// By schema and name. An extension's own are not among them.
    pub routines: Vec<(String, String)>,
    /// The functions the users wrote that a statement can call without
    /// naming them: an operator's, a cast's, a type's. While anything is
    /// hidden, every statement is refused while one is there.
    pub unnamed_calls: Vec<String>,
    /// Functions read as tables that keep no data of their own, which the
    /// engine reports reads of as a table's (SQLite's `json_each`): one the
    /// catalog does not list as a relation is never hidden.
    pub table_functions: Vec<String>,
    /// The columns of the tables the masked columns' entries name, by
    /// schema, table and column: what says an entry still names a column.
    /// Read only where something is masked.
    pub columns: Vec<(String, String, String)>,
    /// The masked columns each view reads, where the catalog says what a
    /// view reads column by column. `None` where it does not: a view over a
    /// table with a masked column is then taken to read it.
    pub column_reads: Option<Vec<ColumnRead>>,
    /// Each view's definition as the server writes it back, by the view's
    /// schema and name, where the catalog keeps no column of a whole row a
    /// view reads (PostgreSQL's): it is read for one. Only where something
    /// is masked.
    pub definitions: Vec<(String, String, String)>,
    /// The generated columns of the tables the masked columns' entries
    /// name, by schema, table and column: one may be computed from a
    /// masked column, and is masked with it.
    pub generated: Vec<(String, String, String)>,
}

/// A relation of the catalog, as [`Hidden`] keeps it.
#[derive(Debug, Clone)]
struct Listed {
    schema: String,
    name: String,
    why: Option<Why>,
    /// For a view that reads a masked column, the column, as its entry is
    /// written, and whose the entry is.
    masks: Option<(String, By)>,
}

/// Why a relation is hidden, and whose setting hides it.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Why {
    /// A setting hides it by its name, or it is opaque.
    Named(By),
    /// It depends on this hidden relation, by its name.
    DependsOn(String, By),
}

/// What [`Sources`] come to on one database: each relation that is hidden
/// and why, and the functions its users wrote, which are refused while
/// anything is hidden (B.2).
#[derive(Debug, Clone)]
pub struct Hidden {
    dialect: Dialect,
    /// Each source's Tables setting, the administrator's first.
    tables: Vec<(By, Tables)>,
    /// Every listed relation, by its name folded to lower case: its schema
    /// and its name as the catalog writes them, and why it is hidden where
    /// it is.
    relations: HashMap<String, Vec<Listed>>,
    /// The users' functions' names, folded, while anything is hidden.
    routines: HashSet<String>,
    /// [`Catalog::unnamed_calls`], while anything is hidden.
    unnamed_calls: Vec<String>,
    /// [`Catalog::table_functions`], folded.
    table_functions: HashSet<String>,
    /// The entries of the person's "All except" list that name no
    /// relation of the catalog, as the person writes them.
    missing: Vec<String>,
    /// The masked columns, every source's, the administrator's first.
    masked: Vec<ColumnName>,
    /// Whose each of `masked` is, in its order.
    masked_by: Vec<By>,
    /// The tables that have a masked column, by schema and name, with the
    /// entry's place in `masked`: the entries' own, and the partitions and
    /// child tables of those.
    masked_tables: Vec<(String, String, usize)>,
    /// The person's masked columns' entries that name no column of the
    /// catalog, as the person writes them.
    masked_missing: Vec<String>,
    /// Whether an entry of a rule's "All except" list or masked columns
    /// names nothing in the catalog.
    policy_stale: bool,
}

impl Hidden {
    /// What the person's `tables` hide of `catalog`, on a database that
    /// speaks `dialect`.
    pub fn new(dialect: Dialect, catalog: &Catalog, tables: &Tables) -> Self {
        Self::masking(dialect, catalog, tables, &[])
    }

    /// What the person's `tables` hide of `catalog`, and what their
    /// `masked` mask.
    pub fn masking(
        dialect: Dialect,
        catalog: &Catalog,
        tables: &Tables,
        masked: &[ColumnName],
    ) -> Self {
        let sources = Sources::person(tables.clone(), masked.to_vec());
        Self::of(dialect, catalog, &sources)
    }

    /// What `sources` hide and mask of `catalog`, on a database that
    /// speaks `dialect`.
    pub fn of(dialect: Dialect, catalog: &Catalog, sources: &Sources) -> Self {
        let guards = sources.guards_anything();
        let tables: Vec<(By, Tables)> = sources
            .each()
            .map(|(by, source)| (by, source.tables.clone()))
            .collect();
        let (masked_by, masked): (Vec<By>, Vec<ColumnName>) = sources
            .masked_by()
            .map(|(by, entry)| (by, entry.clone()))
            .unzip();
        // What is hidden while anything is, whose setting that is: the
        // administrator's where a rule guards anything, since then the
        // person cannot open it.
        let guarding = match sources.administrator.iter().any(Source::guards_anything) {
            true => By::Administrator,
            false => By::Person,
        };
        let index: HashMap<(String, String), usize> = catalog
            .relations
            .iter()
            .enumerate()
            .map(|(at, relation)| ((fold(&relation.schema), fold(&relation.name)), at))
            .collect();
        let depends: Vec<Vec<usize>> = catalog
            .relations
            .iter()
            .map(|relation| {
                relation
                    .depends_on
                    .iter()
                    .filter_map(|(schema, name)| index.get(&(fold(schema), fold(name))).copied())
                    .collect()
            })
            .collect();
        let mut why: Vec<Option<Option<Why>>> = vec![None; catalog.relations.len()];
        for at in 0..catalog.relations.len() {
            hidden_why(at, catalog, &depends, &tables, guarding, &mut why);
        }
        // A generated column of a table with a masked column may be
        // computed from it (`lower(email)`): it is masked too, by an entry
        // of its own, the administrator's where one of theirs is of the
        // table.
        let named = masked_tables(catalog, &depends, &masked);
        let mut every = masked.clone();
        let mut every_by = masked_by.clone();
        for (schema, table, column) in &catalog.generated {
            let of_masked: Vec<By> = named
                .iter()
                .filter(|(own, name, _)| same(own, schema) && same(name, table))
                .map(|(_, _, at)| masked_by[*at])
                .collect();
            if !of_masked.is_empty()
                && !every
                    .iter()
                    .any(|entry| entry.names(Some(schema), table, column))
            {
                every.push(ColumnName {
                    schema: Some(schema.clone()),
                    table: table.clone(),
                    column: column.clone(),
                });
                every_by.push(match of_masked.contains(&By::Administrator) {
                    true => By::Administrator,
                    false => By::Person,
                });
            }
        }
        let masked = every.as_slice();
        let masked_tables = masked_tables(catalog, &depends, masked);
        let masks = masked_views(dialect, catalog, &depends, masked, &masked_tables);
        let mut relations: HashMap<String, Vec<Listed>> = HashMap::new();
        for ((relation, why), masks) in catalog.relations.iter().zip(why).zip(masks) {
            relations
                .entry(fold(&relation.name))
                .or_default()
                .push(Listed {
                    schema: relation.schema.clone(),
                    name: relation.name.clone(),
                    why: why.flatten(),
                    masks: masks.map(|at| (masked[at].to_string(), every_by[at])),
                });
        }
        let routines = match guards {
            true => catalog
                .routines
                .iter()
                .map(|(_, name)| fold(name))
                .collect(),
            false => HashSet::new(),
        };
        let unnamed_calls = match guards {
            true => catalog.unnamed_calls.clone(),
            false => Vec::new(),
        };
        let stale = |entry: &&ColumnName| {
            !system_catalog(dialect, entry.schema.as_deref(), &entry.table)
                && !catalog
                    .columns
                    .iter()
                    .any(|(schema, table, column)| entry.names(Some(schema), table, column))
        };
        let masked_missing = sources
            .person
            .masked
            .iter()
            .filter(stale)
            .map(ToString::to_string)
            .collect();
        let policy_stale = sources.administrator.iter().any(|source| {
            !missing(dialect, catalog, &source.tables).is_empty()
                || source.masked.iter().any(|entry| stale(&entry))
        });
        Self {
            dialect,
            tables,
            relations,
            routines,
            unnamed_calls,
            table_functions: catalog
                .table_functions
                .iter()
                .map(|name| fold(name))
                .collect(),
            missing: missing(dialect, catalog, &sources.person.tables),
            masked: masked.to_vec(),
            masked_by: every_by,
            masked_tables,
            masked_missing,
            policy_stale,
        }
    }

    /// Whether anything is hidden.
    pub fn hides_anything(&self) -> bool {
        self.tables
            .iter()
            .any(|(_, tables)| tables.hides_anything())
    }

    /// Whether anything is hidden or masked: then the system catalogs and
    /// the functions the users wrote are closed, and the statements are
    /// checked.
    pub fn guards_anything(&self) -> bool {
        self.hides_anything() || !self.masked.is_empty()
    }

    /// Whether agents may use the connection at all: not while an "All
    /// except" list names a table it does not have, or a masked column's
    /// entry names no column. A hidden table or a masked column that was
    /// renamed is that, and its new name is hidden or masked by nothing.
    /// A rule's entry is told first: the person cannot fix it.
    pub fn check_settings(&self) -> Result<(), Refusal> {
        if self.policy_stale {
            return Err(Refusal::PolicyNeedsFixing);
        }
        match self.missing.is_empty() && self.masked_missing.is_empty() {
            true => Ok(()),
            false => Err(Refusal::SettingsNeedALook {
                tables: self.missing.clone(),
                columns: self.masked_missing.clone(),
            }),
        }
    }

    /// Whether the relation `name` of `schema` is hidden from agents: by
    /// the person's names, by what it reads, or as a system catalog. A view
    /// that reads a masked column is refused, and not hidden by this.
    pub fn hides(&self, schema: &str, name: &str) -> bool {
        matches!(
            self.check_engine_read(Some(schema), name),
            Err(refusal) if !matches!(refusal, Refusal::MaskedColumnUsed { .. })
        )
    }

    /// Whether a column is masked.
    pub fn masks_anything(&self) -> bool {
        !self.masked.is_empty()
    }

    /// The masked column the view `name` of `schema` reads, as the person
    /// writes its entry, where it is a view that reads one: a statement
    /// that reads it is refused, though the view is not hidden.
    pub fn reads_masked(&self, schema: &str, name: &str) -> Option<String> {
        self.relations
            .get(&fold(name))?
            .iter()
            .find(|listed| same(&listed.schema, schema))?
            .masks
            .as_ref()
            .map(|(entry, _)| entry.clone())
    }

    /// A masked column of the relation `table` of `schema`, as the person
    /// writes its entry, where it has one: for a whole row of it.
    pub fn masks_table(&self, schema: Option<&str>, table: &str) -> Option<String> {
        self.masked_tables
            .iter()
            .find(|(own, name, _)| {
                same(name, table) && schema.is_none_or(|schema| same(own, schema))
            })
            .map(|(_, _, at)| self.masked[*at].to_string())
    }

    /// The masked column `column` of the relation `table` of `schema`, as
    /// the person writes its entry, where it is one: a column the entries
    /// name, or one of a partition or a child table of their tables.
    pub fn masks(&self, schema: Option<&str>, table: &str, column: &str) -> Option<String> {
        self.masked_at(schema, table, column)
            .map(|at| self.masked[at].to_string())
    }

    /// [`Self::masks`]'s entry, by its place in `masked`.
    fn masked_at(&self, schema: Option<&str>, table: &str, column: &str) -> Option<usize> {
        let by_entry = self
            .masked
            .iter()
            .position(|entry| entry.names(schema, table, column));
        let by_table = || {
            self.masked_tables
                .iter()
                .filter(|(own, name, _)| {
                    same(name, table) && schema.is_none_or(|schema| same(own, schema))
                })
                .map(|(_, _, at)| *at)
                .find(|at| same(&self.masked[*at].column, column))
        };
        by_entry.or_else(by_table)
    }

    /// Whose the masked column's entry `entry` is, as it is written: the
    /// administrator's where both have it.
    pub fn masked_by(&self, entry: &str) -> By {
        self.masked
            .iter()
            .zip(&self.masked_by)
            .find(|(own, _)| own.to_string() == entry)
            .map(|(_, by)| *by)
            .unwrap_or_default()
    }

    /// Checks a column the engine says a statement reads, with the view or
    /// the common table expression it reads it for (SQLite's authorizer):
    /// a masked one read for one is refused, naming it. One the statement
    /// reads itself is its check's.
    pub fn check_column_read(
        &self,
        schema: Option<&str>,
        table: &str,
        column: &str,
        source: Option<&str>,
    ) -> Result<(), Refusal> {
        match (source, self.masked_at(schema, table, column)) {
            (Some(view), Some(at)) => Err(Refusal::MaskedColumnUsed {
                column: self.masked[at].to_string(),
                how: MaskedUse::Through(view.to_owned()),
                by: self.masked_by[at],
            }),
            _ => Ok(()),
        }
    }

    /// What the reading of a statement knows of the masked columns.
    fn masks_known(&self) -> Masks<'_> {
        let more = self
            .masked_tables
            .iter()
            .map(|(_, name, at)| (name.clone(), &self.masked[*at]));
        Masks::new(&self.masked, &self.masked_by, more)
    }

    /// Whether `schema` is a schema of the database: one the catalog lists
    /// a relation of, or the system's.
    pub fn knows_schema(&self, schema: &str) -> bool {
        system_catalog(self.dialect, Some(schema), "")
            || self
                .relations
                .values()
                .flatten()
                .any(|listed| same(&listed.schema, schema))
    }

    /// Checks a relation the engine says the statement reads: by its
    /// schema where the engine gives it. One the catalog did not list (a
    /// table made since it was read) is hidden as the person's own names
    /// hide it.
    pub fn check_engine_read(&self, schema: Option<&str>, name: &str) -> Result<(), Refusal> {
        if !self.guards_anything() {
            return Ok(());
        }
        if system_catalog(self.dialect, schema, name) {
            return Err(Refusal::SystemCatalog);
        }
        match self.listed(schema, name) {
            Some(found) => refused(found),
            None if self.table_functions.contains(&fold(name)) => Ok(()),
            None => match self
                .tables
                .iter()
                .find(|(_, tables)| tables.hides(schema, name))
            {
                Some((by, _)) => Err(Refusal::TableHidden {
                    name: name.to_owned(),
                    through: None,
                    by: *by,
                }),
                None => Ok(()),
            },
        }
    }

    /// Checks a relation the engine names where what it gives may be an
    /// alias of the statement's own (MySQL's plan): refused when it is a
    /// system catalog, or a listed relation that is hidden; a name the
    /// catalog does not list is passed over.
    pub fn check_name(&self, schema: Option<&str>, name: &str) -> Result<(), Refusal> {
        if !self.guards_anything() {
            return Ok(());
        }
        if system_catalog(self.dialect, schema, name) {
            return Err(Refusal::SystemCatalog);
        }
        match self.listed(schema, name) {
            Some(found) => refused(found),
            None => Ok(()),
        }
    }

    /// Checks a function the statement calls, by the name it is called by:
    /// one the database's users wrote is refused while anything is hidden,
    /// since what it reads cannot be seen (B.2).
    pub fn check_function(&self, name: &str) -> Result<(), Refusal> {
        match self.routines.contains(&fold(name)) {
            true => Err(Refusal::FunctionNotAllowed(name.to_owned())),
            false => Ok(()),
        }
    }

    /// Checks the names the statement's own text gives, read off its
    /// tokens (see the module's doc): every relation it names in a table's
    /// place, every common table expression it defines, and every function
    /// it calls. A name the catalog does not list is the database's to
    /// answer, as a common table expression's is: the engine's own check
    /// sees what it reads.
    pub fn check_statement(&self, read: &Read) -> Result<(), Refusal> {
        if !self.guards_anything() {
            return Ok(());
        }
        self.check_settings()?;
        // Called by any statement that compares, sorts or casts the wrong
        // value, whatever it names.
        if let Some(function) = self.unnamed_calls.first() {
            return Err(Refusal::FunctionNotAllowed(function.clone()));
        }
        let names = names(read.dialect(), read.text());
        for defined in &names.defined {
            // Named like a hidden table: refused, not reasoned about. Which
            // a name in its scope meant, the table or the expression, is
            // a question a wrong answer to would leak.
            if let Some(found) = self.listed(None, defined)
                && found.why.is_some()
            {
                return refused(found);
            }
        }
        for relation in &names.relations {
            let (schema, name) = (relation.schema.as_deref(), relation.name.as_str());
            if system_catalog(self.dialect, schema, name) {
                return Err(Refusal::SystemCatalog);
            }
            if schema.is_none() && names.defined.iter().any(|defined| same(defined, name)) {
                continue;
            }
            if let Some(found) = self.listed(schema, name) {
                refused(found)?;
            }
        }
        for function in &names.functions {
            self.check_function(&function.name)?;
        }
        // On PostgreSQL a function is called as a column of a row, `b.f`
        // as `f(b)`, by every lookup that would find `f(b)`: a name after a
        // dot that is a function the users wrote is taken for one.
        // DuckDB calls one so too: `name.upper()` is `upper(name)`.
        match self.dialect {
            Dialect::Postgres | Dialect::DuckDb => {
                for name in &names.after_dot {
                    self.check_function(name)?;
                }
            }
            Dialect::MySql | Dialect::Sqlite => {}
        }
        masked::check(read, &self.masks_known())
    }

    /// The listed relation `name` of `schema` that is hidden, or else any
    /// listed one: without a schema, every one of that name, and a hidden
    /// one among them is the one taken.
    fn listed(&self, schema: Option<&str>, name: &str) -> Option<&Listed> {
        let found = self.relations.get(&fold(name))?;
        let mut matching = found
            .iter()
            .filter(|listed| schema.is_none_or(|schema| same(&listed.schema, schema)));
        let first = matching.clone().next()?;
        Some(
            matching
                .clone()
                .find(|listed| listed.why.is_some())
                .or_else(|| matching.find(|listed| listed.masks.is_some()))
                .unwrap_or(first),
        )
    }
}

/// The entries of `tables`' "All except" list that name no relation of
/// `catalog`, written as the person writes them. A system catalog's name
/// is none of them: those are hidden with anything.
fn missing(dialect: Dialect, catalog: &Catalog, tables: &Tables) -> Vec<String> {
    let Tables::AllExcept(names) = tables else {
        return Vec::new();
    };
    names
        .iter()
        .filter(|named| {
            !system_catalog(dialect, named.schema.as_deref(), &named.name)
                && !catalog
                    .relations
                    .iter()
                    .any(|relation| named.names(Some(&relation.schema), &relation.name))
        })
        .map(ToString::to_string)
        .collect()
}

/// Refuses `found` where it is hidden, by the names the catalog writes,
/// or where it is a view that reads a masked column.
fn refused(found: &Listed) -> Result<(), Refusal> {
    match &found.why {
        None => match &found.masks {
            Some((column, by)) => Err(Refusal::MaskedColumnUsed {
                column: column.clone(),
                how: MaskedUse::Through(found.name.clone()),
                by: *by,
            }),
            None => Ok(()),
        },
        Some(Why::Named(by)) => Err(Refusal::TableHidden {
            name: found.name.clone(),
            through: None,
            by: *by,
        }),
        Some(Why::DependsOn(hidden, by)) => Err(Refusal::TableHidden {
            name: hidden.clone(),
            through: Some(found.name.clone()),
            by: *by,
        }),
    }
}

/// The tables that have a masked column, by schema and name, with an
/// entry's place in `masked`, once for each entry: each relation an entry
/// names, a view too, and the partitions and the child tables of those
/// tables, at any depth. An entry that names a partition or a child table
/// masks the column of its parent too, and so of the parent's other parts:
/// a read of the parent reads the part's rows.
fn masked_tables(
    catalog: &Catalog,
    depends: &[Vec<usize>],
    masked: &[ColumnName],
) -> Vec<(String, String, usize)> {
    let mut found = Vec::new();
    for (entry_at, entry) in masked.iter().enumerate() {
        let mut has: Vec<bool> = catalog
            .relations
            .iter()
            .map(|relation| entry.names_table(Some(&relation.schema), &relation.name))
            .collect();
        // A table's parent is what it depends on: taken from it, and given
        // it, until nothing more is, which a cycle cannot keep from ending.
        loop {
            let mut more = false;
            for at in 0..catalog.relations.len() {
                if catalog.relations[at].view {
                    continue;
                }
                if !has[at] && depends[at].iter().any(|&on| has[on]) {
                    has[at] = true;
                    more = true;
                }
                if has[at] {
                    for &on in &depends[at] {
                        if !has[on] && !catalog.relations[on].view {
                            has[on] = true;
                            more = true;
                        }
                    }
                }
            }
            if !more {
                break;
            }
        }
        for (relation, _) in catalog.relations.iter().zip(has).filter(|(_, has)| *has) {
            found.push((relation.schema.clone(), relation.name.clone(), entry_at));
        }
    }
    found
}

/// For each relation, the place in `masked` of the masked column it reads,
/// where it is a view that reads one: by what the catalog says it reads
/// column by column, or, where it does not say, by a table with a masked
/// column that it reads; and by a whole row of such a table that its
/// definition reads, a partition's or a child table's too. A view over
/// such a view reads it too.
fn masked_views(
    dialect: Dialect,
    catalog: &Catalog,
    depends: &[Vec<usize>],
    masked: &[ColumnName],
    tables: &[(String, String, usize)],
) -> Vec<Option<usize>> {
    // An entry of the table `name` of `schema`, of the column `column`
    // where one is given.
    let entry_of = |schema: &str, name: &str, column: Option<&str>| {
        tables
            .iter()
            .filter(|(own, table, _)| same(own, schema) && same(table, name))
            .map(|(_, _, at)| *at)
            .find(|&at| column.is_none_or(|column| same(&masked[at].column, column)))
    };
    let more = tables
        .iter()
        .map(|(_, name, at)| (name.clone(), &masked[*at]));
    let masks = Masks::new(masked, &[], more);
    // The entry of a table whose whole row the view's definition reads.
    let whole_row = |relation: &Relation| {
        catalog
            .definitions
            .iter()
            .filter(|(schema, view, _)| {
                same(schema, &relation.schema) && same(view, &relation.name)
            })
            .find_map(|(_, _, definition)| masked::reads_whole_row(dialect, definition, &masks))
            .and_then(|entry| masked.iter().position(|own| std::ptr::eq(own, entry)))
    };
    let mut found: Vec<Option<usize>> = catalog
        .relations
        .iter()
        .enumerate()
        .map(|(at, relation)| {
            if !relation.view {
                return None;
            }
            match &catalog.column_reads {
                Some(reads) => reads
                    .iter()
                    .filter(|read| {
                        same(&read.schema, &relation.schema) && same(&read.view, &relation.name)
                    })
                    .find_map(|read| entry_of(&read.table_schema, &read.table, Some(&read.column)))
                    .or_else(|| whole_row(relation)),
                None => depends[at].iter().find_map(|&on| {
                    let on = &catalog.relations[on];
                    entry_of(&on.schema, &on.name, None)
                }),
            }
        })
        .collect();
    loop {
        let mut more = false;
        for at in 0..catalog.relations.len() {
            if found[at].is_some() || !catalog.relations[at].view {
                continue;
            }
            if let Some(entry) = depends[at].iter().find_map(|&on| found[on]) {
                found[at] = Some(entry);
                more = true;
            }
        }
        if !more {
            break;
        }
    }
    found
}

/// The columns an expression of PostgreSQL's plan names (`o.email`,
/// `"Orders".email`, and `email` alone where the plan reads one relation),
/// each by its qualifier, where it has one, and its column, `*` for a
/// whole row (`o.*`); and whether the expression is one such name and
/// nothing else. A word before a parenthesis is a function's, and the
/// words after a cast's `::` are a type's (`timestamp without time zone`).
pub(crate) fn column_references(
    dialect: Dialect,
    text: &str,
) -> (Vec<(Option<String>, String)>, bool) {
    let pieces = pieces(dialect, text);
    // Which pieces are a cast's type: the tokens are the pieces, but for
    // white space and comments.
    let tokens: Vec<&str> = sql::tokenize(dialect, text)
        .into_iter()
        .filter(|token| !matches!(token.kind, TokenKind::Whitespace | TokenKind::Comment))
        .map(|token| &text[token.range])
        .collect();
    // A type of more than one word goes on only with these (`character
    // varying`, `double precision`, `timestamp without time zone`): any
    // other word after it is the expression's again (`ELSE email`).
    const MORE: &[&str] = &["varying", "precision", "with", "without", "time", "zone"];
    let mut typed = vec![false; pieces.len()];
    for at in 0..pieces.len() {
        let cast = at >= 2 && tokens.get(at - 1) == Some(&":") && tokens.get(at - 2) == Some(&":");
        let more = at > 0 && typed[at - 1] && MORE.iter().any(|word| pieces[at].is(word));
        typed[at] = matches!(pieces[at], Piece::Word { .. }) && (cast || more);
    }
    let mut found = Vec::new();
    for (at, piece) in pieces.iter().enumerate() {
        if typed[at] {
            continue;
        }
        let before = at.checked_sub(1).map(|before| &pieces[before]);
        let after = pieces.get(at + 1);
        match piece {
            // `schema.table.column`: the qualifier is the table.
            Piece::Word { .. } | Piece::Star if before == Some(&Piece::Dot) => {
                let qualifier = at.checked_sub(2).and_then(|back| pieces[back].word());
                let column = match piece {
                    Piece::Star => "*",
                    other => other.word().unwrap_or_default(),
                };
                if after != Some(&Piece::Dot)
                    && after != Some(&Piece::Open)
                    && let Some(qualifier) = qualifier
                {
                    found.push((Some(qualifier.to_owned()), column.to_owned()));
                }
            }
            Piece::Word { .. } if after != Some(&Piece::Dot) && after != Some(&Piece::Open) => {
                if let Some(word) = piece.word() {
                    found.push((None, word.to_owned()));
                }
            }
            _ => {}
        }
    }
    let bare = matches!(
        pieces.as_slice(),
        [Piece::Word { .. }]
            | [Piece::Word { .. }, Piece::Dot, Piece::Word { .. }]
            | [
                Piece::Word { .. },
                Piece::Dot,
                Piece::Word { .. },
                Piece::Dot,
                Piece::Word { .. },
            ]
    );
    (found, bare)
}

/// Whether the relation at `at` is hidden, and why, worked out once: by
/// its own name, in the first of `tables` that hides it, else as opaque,
/// by `guarding`, else by the first hidden relation it depends on. A
/// relation met again while its own answer is being worked out (a cycle
/// the catalog should not have) is taken for hidden.
fn hidden_why(
    at: usize,
    catalog: &Catalog,
    depends: &[Vec<usize>],
    tables: &[(By, Tables)],
    guarding: By,
    why: &mut [Option<Option<Why>>],
) -> Option<Why> {
    if let Some(known) = &why[at] {
        return known.clone();
    }
    let relation = &catalog.relations[at];
    // Hidden until worked out, for a cycle.
    why[at] = Some(Some(Why::Named(guarding)));
    let by_name = tables
        .iter()
        .find(|(_, tables)| match (relation.part, tables) {
            (true, Tables::Only(_)) => false,
            _ => tables.hides(Some(&relation.schema), &relation.name),
        })
        .map(|(by, _)| *by)
        .or(relation.opaque.then_some(guarding));
    let found = match by_name {
        Some(by) => Some(Why::Named(by)),
        None => depends[at].iter().find_map(|&on| {
            hidden_why(on, catalog, depends, tables, guarding, why).map(|hidden| {
                let name = &catalog.relations[on].name;
                Why::DependsOn(name.clone(), hidden.by())
            })
        }),
    };
    // What a relation depends on through another is named by the one it
    // reaches first, which is hidden itself: its name is told, not the
    // hidden table at the end of the chain.
    why[at] = Some(found.clone());
    found
}

impl Why {
    /// Whose setting hides the relation.
    fn by(&self) -> By {
        match self {
            Self::Named(by) | Self::DependsOn(_, by) => *by,
        }
    }
}

/// Whether the relation `name` of `schema` is one of the system catalogs,
/// which describe every table: its schema's, or by its name where the
/// database finds such a name in its catalog first.
fn system_catalog(dialect: Dialect, schema: Option<&str>, name: &str) -> bool {
    let in_schema = |systems: &[&str]| {
        schema.is_some_and(|schema| systems.iter().any(|system| same(system, schema)))
    };
    match dialect {
        // PostgreSQL finds a `pg_` name in `pg_catalog` first, whatever
        // the search path; `pg_temp` schemas are the sessions' own.
        Dialect::Postgres => {
            in_schema(&["pg_catalog", "information_schema", "pg_toast"])
                || schema.is_some_and(|schema| fold(schema).starts_with("pg_temp"))
                || (schema.is_none() && fold(name).starts_with("pg_"))
        }
        Dialect::MySql => in_schema(&["information_schema", "mysql", "performance_schema", "sys"]),
        // SQLite's own tables are `sqlite_` ones, in any schema, and
        // `dbstat` describes every table's pages.
        Dialect::Sqlite => fold(name).starts_with("sqlite_") || same(name, "dbstat"),
        // DuckDB's catalog is its `duckdb_` functions and views, and the
        // two schemas it keeps for PostgreSQL's and the standard's names.
        Dialect::DuckDb => {
            in_schema(&["information_schema", "pg_catalog"])
                || fold(name).starts_with("duckdb_")
                || fold(name).starts_with("pragma_")
                || same(name, "sqlite_master")
        }
    }
}

/// The query a statement has the database plan: the statement itself, or
/// the one an `EXPLAIN` explains, from its first word that begins a query
/// (past `EXPLAIN`'s options in parentheses, `VERBOSE`, MySQL's
/// `FORMAT=…`). An `EXPLAIN` is planned so too: what its plan names, the
/// engine reads to make it, statistics included.
pub(crate) fn planned(read: &Read) -> &str {
    let text = read.text();
    if !read.explains() {
        return text;
    }
    let tokens = sql::tokenize(read.dialect(), text);
    let mut depth = 0_usize;
    for token in &tokens {
        let written = &text[token.range.clone()];
        match (token.kind, written) {
            (TokenKind::Punctuation, "(") if depth > 0 => depth += 1,
            (TokenKind::Punctuation, "(") => {
                // `EXPLAIN (…)` is options; `EXPLAIN (SELECT …)` a query.
                let after = &text[token.range.end..];
                let next = sql::words(read.dialect(), after);
                match next.first().map(String::as_str) {
                    Some("SELECT" | "WITH" | "VALUES" | "TABLE") => {
                        return &text[token.range.start..];
                    }
                    _ => depth = 1,
                }
            }
            (TokenKind::Punctuation, ")") => depth = depth.saturating_sub(1),
            (TokenKind::Keyword | TokenKind::Identifier, word)
                if depth == 0
                    && ["SELECT", "WITH", "VALUES", "TABLE"]
                        .iter()
                        .any(|start| start.eq_ignore_ascii_case(word)) =>
            {
                return &text[token.range.start..];
            }
            _ => {}
        }
    }
    text
}

/// Whether `definition`, a view's or a policy's as the database writes it
/// back, calls a function an agent may not call (B.2): one that runs SQL
/// held in a string, which no check reads, and so may read a hidden table.
/// PostgreSQL keeps no dependency on its own functions, so the text is read.
pub(crate) fn calls_a_refused_function(dialect: Dialect, definition: &str) -> bool {
    names(dialect, definition)
        .functions
        .iter()
        .any(|function| super::function_allowed(dialect, &function.name).is_err())
}

/// The names the statement's text gives in a table's place, as written,
/// without their schemas.
pub(crate) fn named_relations(read: &Read) -> Vec<String> {
    names(read.dialect(), read.text())
        .relations
        .into_iter()
        .map(|name| name.name)
        .collect()
}

/// `name` as names are compared.
fn fold(name: &str) -> String {
    name.to_lowercase()
}

/// Whether two names are the same name.
fn same(a: &str, b: &str) -> bool {
    fold(a) == fold(b)
}

/// A name a statement gives, as written, without its quotes.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Name {
    schema: Option<String>,
    name: String,
}

/// The names a statement gives, as [`names`] reads them.
#[derive(Debug, Default, PartialEq, Eq)]
struct Names {
    /// In a table's place: after `FROM`, after a `JOIN`, after a comma in
    /// a `FROM`.
    relations: Vec<Name>,
    /// Common table expressions, and windows, which are written alike.
    defined: Vec<String>,
    /// Called: a word with a parenthesis after it, a keyword's too.
    functions: Vec<Name>,
    /// Every name with a dot before it: a column, or on PostgreSQL a
    /// function called as a column of a row.
    after_dot: Vec<String>,
    /// Each table's place, every time one is read, by where its pieces
    /// are: for the masked columns, whose reading needs to know which
    /// words name a table.
    places: Vec<Place>,
}

/// A relation's name in a table's place, by where it stands among the
/// pieces it was read from.
#[derive(Debug, PartialEq, Eq)]
struct Place {
    name: Name,
    /// The pieces of the name, its schema's with it.
    from: usize,
    to: usize,
    /// The piece of its alias, where it is given one.
    alias: Option<usize>,
    /// Whether a list of column names follows the alias, which gives the
    /// relation's columns other names: `orders AS o(a, b, c)`.
    renames: bool,
}

/// A token as [`names`] reads it.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Piece {
    Word {
        text: String,
        quoted: bool,
        keyword: bool,
    },
    Dot,
    Open,
    Close,
    Comma,
    /// `*`, which selects or multiplies.
    Star,
    /// A number, as written: an `ORDER BY` of one sorts by a place.
    Number(String),
    /// A string in single quotes, without them, which SQLite reads as a
    /// name where only a name can stand (`USING ('email')`, `o.'email'`).
    Text(String),
    Other,
}

impl Piece {
    /// Whether this is the unquoted word `word`, in any case.
    fn is(&self, word: &str) -> bool {
        matches!(self, Self::Word { text, quoted: false, .. } if text.eq_ignore_ascii_case(word))
    }

    /// The word, whatever it is.
    fn word(&self) -> Option<&str> {
        match self {
            Self::Word { text, .. } => Some(text),
            _ => None,
        }
    }

    /// The word, where it can be a name: quoted, or not a keyword.
    fn name(&self) -> Option<&str> {
        match self {
            Self::Word {
                text,
                quoted,
                keyword,
            } if *quoted || !*keyword => Some(text),
            _ => None,
        }
    }
}

/// The words that end a `FROM`'s list at its own depth.
const ENDS_FROM: &[&str] = &[
    "WHERE",
    "GROUP",
    "HAVING",
    "ORDER",
    "LIMIT",
    "OFFSET",
    "FETCH",
    "UNION",
    "INTERSECT",
    "EXCEPT",
    "WINDOW",
    "QUALIFY",
    "SELECT",
    "VALUES",
];

/// The words a table's place begins after, besides `FROM`.
const JOINS: &[&str] = &["JOIN", "STRAIGHT_JOIN"];

/// Reads the names a statement gives off the app's own tokens (B.2: the
/// tree is not walked). It errs toward finding: a name that is a column's
/// in a table's place is checked as a table's too.
fn names(dialect: Dialect, text: &str) -> Names {
    read_names(&pieces(dialect, text))
}

/// [`names`], of the pieces already read.
fn read_names(pieces: &[Piece]) -> Names {
    let mut found = Names::default();
    // Depth of parentheses, and the depths at which a `FROM`'s list is
    // being read.
    let mut depth = 0_usize;
    let mut from_at: Vec<usize> = Vec::new();
    // Whether the piece before was where a table goes: a parenthesis there
    // that opens no query holds a table, or a join of tables, `(customers
    // JOIN books ON …)`, whose names are read as a `FROM`'s list is.
    let mut table_goes = false;
    let mut at = 0;
    while at < pieces.len() {
        let piece = &pieces[at];
        let places_table = matches!(piece, Piece::Comma) && from_at.last() == Some(&depth)
            || piece.is("FROM")
            || JOINS.iter().any(|join| piece.is(join))
            || (table_goes && (piece.is("LATERAL") || piece.is("ONLY")));
        match piece {
            Piece::Open => {
                depth += 1;
                let query = pieces.get(at + 1).is_some_and(|next| {
                    ["SELECT", "WITH", "VALUES", "TABLE"]
                        .iter()
                        .any(|start| next.is(start))
                });
                if table_goes && !query {
                    from_at.push(depth);
                    table_place(pieces, at + 1, &mut found);
                }
            }
            Piece::Close => {
                from_at.retain(|&level| level < depth);
                depth = depth.saturating_sub(1);
            }
            Piece::Comma if from_at.last() == Some(&depth) => {
                table_place(pieces, at + 1, &mut found);
            }
            Piece::Word { .. } if piece.is("FROM") => {
                from_at.retain(|&level| level < depth);
                from_at.push(depth);
                table_place(pieces, at + 1, &mut found);
            }
            Piece::Word { .. } if JOINS.iter().any(|join| piece.is(join)) => {
                table_place(pieces, at + 1, &mut found);
            }
            Piece::Word { .. } if ENDS_FROM.iter().any(|end| piece.is(end)) => {
                from_at.retain(|&level| level < depth);
            }
            Piece::Word { .. } if piece.is("WITH") => defined(pieces, at + 1, &mut found),
            _ => {}
        }
        table_goes = places_table || (table_goes && matches!(piece, Piece::Open));
        // A function may be called by a word that is a keyword too
        // (PostgreSQL's `last`, `first`, `view`): any word is read.
        if at > 0
            && pieces[at - 1] == Piece::Dot
            && let Some(name) = piece.word()
            && !found.after_dot.iter().any(|known| known == name)
        {
            found.after_dot.push(name.to_owned());
        }
        // A word with a parenthesis after it is a call, wherever it is.
        if let Some((name, after)) = dotted(pieces, at, Piece::word)
            && pieces.get(after) == Some(&Piece::Open)
            && !found.functions.contains(&name)
        {
            found.functions.push(name);
        }
        at += 1;
    }
    found
}

/// Reads what stands in a table's place from `at`: past `LATERAL` and
/// `ONLY`, a name, which is a relation's, or a function's where a
/// parenthesis follows it. A parenthesis there opens a query or a join,
/// whose own names are read as the rest are. Any word there is taken for
/// a name, a keyword too: MySQL lets a table be called `last` or `view`
/// without quotes.
fn table_place(pieces: &[Piece], mut at: usize, found: &mut Names) {
    while pieces
        .get(at)
        .is_some_and(|piece| piece.is("LATERAL") || piece.is("ONLY"))
    {
        at += 1;
    }
    let Some((name, after)) = dotted(pieces, at, Piece::word) else {
        return;
    };
    // A table function: [`names`] takes it for a call.
    if pieces.get(after) == Some(&Piece::Open) {
        return;
    }
    // Its alias: after `AS` any word, without it a name only.
    let (alias, next) = match pieces.get(after) {
        Some(piece) if piece.is("AS") => match pieces.get(after + 1).and_then(Piece::word) {
            Some(_) => (Some(after + 1), after + 2),
            None => (None, after + 1),
        },
        Some(piece) if piece.name().is_some() => (Some(after), after + 1),
        _ => (None, after),
    };
    found.places.push(Place {
        name: name.clone(),
        from: at,
        to: after,
        alias,
        renames: alias.is_some() && pieces.get(next) == Some(&Piece::Open),
    });
    if !found.relations.contains(&name) {
        found.relations.push(name);
    }
}

/// Reads the names a `WITH` defines from `at`: each `name [(columns)] AS
/// [NOT] [MATERIALIZED] (`, the ones after a comma too. What is not so
/// written (`WITH ORDINALITY`, `WITH TIME ZONE`) defines nothing.
fn defined(pieces: &[Piece], mut at: usize, found: &mut Names) {
    if pieces.get(at).is_some_and(|piece| piece.is("RECURSIVE")) {
        at += 1;
    }
    loop {
        let Some(name) = pieces.get(at).and_then(Piece::name) else {
            return;
        };
        let name = name.to_owned();
        at += 1;
        if pieces.get(at) == Some(&Piece::Open) {
            at = closing(pieces, at) + 1;
        }
        if !pieces.get(at).is_some_and(|piece| piece.is("AS")) {
            return;
        }
        at += 1;
        while pieces
            .get(at)
            .is_some_and(|piece| piece.is("NOT") || piece.is("MATERIALIZED"))
        {
            at += 1;
        }
        if pieces.get(at) != Some(&Piece::Open) {
            return;
        }
        found.defined.push(name);
        at = closing(pieces, at) + 1;
        if pieces.get(at) != Some(&Piece::Comma) {
            return;
        }
        at += 1;
    }
}

/// The place of the parenthesis that closes the one at `open`, or the end.
fn closing(pieces: &[Piece], open: usize) -> usize {
    let mut depth = 0_usize;
    for (at, piece) in pieces.iter().enumerate().skip(open) {
        match piece {
            Piece::Open => depth += 1,
            Piece::Close => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return at;
                }
            }
            _ => {}
        }
    }
    pieces.len()
}

/// A name at `at`, with the parts after it joined by dots: its last part
/// is the name and the one before it the schema (a third, a database's or
/// a catalog's, is passed over). Gives where it ends;
/// `part` says which pieces may be a part of the name.
fn dotted(pieces: &[Piece], at: usize, part: fn(&Piece) -> Option<&str>) -> Option<(Name, usize)> {
    let mut parts = vec![part(pieces.get(at)?)?.to_owned()];
    let mut next = at + 1;
    while pieces.get(next) == Some(&Piece::Dot)
        && let Some(part) = pieces.get(next + 1).and_then(part)
    {
        parts.push(part.to_owned());
        next += 2;
    }
    let name = parts.pop()?;
    Some((
        Name {
            schema: parts.pop(),
            name,
        },
        next,
    ))
}

/// The statement as [`names`] reads it: words, dots, parentheses and
/// commas, with comments and white space left out.
fn pieces(dialect: Dialect, text: &str) -> Vec<Piece> {
    sql::tokenize(dialect, text)
        .into_iter()
        .filter_map(|token| {
            let written = &text[token.range.clone()];
            Some(match token.kind {
                TokenKind::Whitespace | TokenKind::Comment => return None,
                TokenKind::Keyword | TokenKind::Identifier => Piece::Word {
                    text: written.to_owned(),
                    quoted: false,
                    keyword: sql::is_keyword(dialect, written) || read_as_keyword(written),
                },
                TokenKind::QuotedIdentifier => Piece::Word {
                    text: unquoted(written),
                    quoted: true,
                    keyword: false,
                },
                TokenKind::Punctuation => match written {
                    "." => Piece::Dot,
                    "(" => Piece::Open,
                    ")" => Piece::Close,
                    "," => Piece::Comma,
                    _ => Piece::Other,
                },
                TokenKind::Operator if written == "*" => Piece::Star,
                TokenKind::Number => Piece::Number(written.to_owned()),
                TokenKind::String if written.starts_with('\'') => Piece::Text(unquoted(written)),
                TokenKind::String
                | TokenKind::ExecutableComment
                | TokenKind::Operator
                | TokenKind::Semicolon => Piece::Other,
            })
        })
        .collect()
}

/// Whether `word` is one [`names`] reads as a keyword whether or not the
/// tokenizer's list has it.
fn read_as_keyword(word: &str) -> bool {
    ENDS_FROM
        .iter()
        .chain(JOINS)
        .chain(&[
            "FROM",
            "WITH",
            "LATERAL",
            "ONLY",
            "AS",
            "RECURSIVE",
            "MATERIALIZED",
            // Not an end of a `FROM`: an index hint's `FOR JOIN` is in one,
            // and `FOR UPDATE` is refused before this is asked.
            "FOR",
        ])
        .any(|known| known.eq_ignore_ascii_case(word))
}

/// A quoted name without its quotes, and with a doubled quote inside it
/// read as one.
fn unquoted(written: &str) -> String {
    let (open, close) = match written.chars().next() {
        Some('`') => ('`', '`'),
        Some('[') => ('[', ']'),
        Some('\'') => ('\'', '\''),
        _ => ('"', '"'),
    };
    let inner = written
        .strip_prefix(open)
        .unwrap_or(written)
        .strip_suffix(close)
        .unwrap_or(written);
    let doubled: String = [close, close].iter().collect();
    inner.replace(&doubled, &close.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn table(schema: Option<&str>, name: &str) -> TableName {
        TableName {
            schema: schema.map(str::to_owned),
            name: name.to_owned(),
        }
    }

    fn relation(schema: &str, name: &str, depends_on: &[(&str, &str)]) -> Relation {
        Relation {
            schema: schema.to_owned(),
            name: name.to_owned(),
            depends_on: depends_on
                .iter()
                .map(|(schema, name)| ((*schema).to_owned(), (*name).to_owned()))
                .collect(),
            opaque: false,
            part: false,
            view: false,
        }
    }

    /// The Bookshop as PostgreSQL lists it, with a few relations more:
    /// another schema's `customers`, a materialized view over the
    /// customers, a partitioned table and a child table.
    fn bookshop() -> Catalog {
        Catalog {
            relations: vec![
                relation("public", "books", &[]),
                relation("public", "customers", &[]),
                relation("public", "orders", &[]),
                relation("sales", "customers", &[]),
                relation("public", "customer_names", &[("public", "customers")]),
                relation("public", "recent_orders", &[("public", "orders")]),
                relation("public", "customer_count", &[("public", "customer_names")]),
                relation("public", "vip", &[("public", "customers")]),
                relation("public", "events", &[]),
                relation("public", "events_2026", &[("public", "events")]),
            ],
            routines: vec![("public".into(), "names".into())],
            ..Catalog::default()
        }
    }

    fn checked(dialect: Dialect, sql: &str) -> Read {
        super::super::check_read(dialect, sql, &[]).unwrap()
    }

    fn hidden(tables: Tables) -> Hidden {
        Hidden::new(Dialect::Postgres, &bookshop(), &tables)
    }

    fn statement(hidden: &Hidden, sql: &str) -> Result<(), Refusal> {
        hidden.check_statement(&checked(Dialect::Postgres, sql))
    }

    fn table_hidden(name: &str, through: Option<&str>) -> Result<(), Refusal> {
        Err(Refusal::TableHidden {
            name: name.to_owned(),
            through: through.map(str::to_owned),
            by: crate::agent::By::Person,
        })
    }

    #[test]
    fn a_name_in_any_place_of_a_query_is_read_off_its_tokens() {
        let read = |sql: &str| names(Dialect::Postgres, sql);
        let relations = |sql: &str| {
            read(sql)
                .relations
                .into_iter()
                .map(|name| match name.schema {
                    Some(schema) => format!("{schema}.{}", name.name),
                    None => name.name,
                })
                .collect::<Vec<_>>()
        };
        assert_eq!(relations("SELECT * FROM books"), ["books"]);
        assert_eq!(
            relations("SELECT * FROM public.books b, \"Sales\".\"Customers\" AS c"),
            ["public.books", "Sales.Customers"]
        );
        assert_eq!(
            relations(
                "SELECT * FROM books b JOIN orders o ON o.id = b.id \
                 LEFT JOIN LATERAL (SELECT * FROM customers) c ON true, events"
            ),
            ["books", "orders", "customers", "events"]
        );
        // Nested, and in a set operation; the commas of a select list and
        // of an IN are not a FROM's.
        assert_eq!(
            relations(
                "SELECT a, (SELECT count(*) FROM orders), b FROM books \
                 WHERE id IN (1, 2) UNION SELECT 1, 2, 3 FROM customers ORDER BY a, b"
            ),
            ["orders", "books", "customers"]
        );
        // A comment between a name and its dot, and a database's name
        // before the schema.
        assert_eq!(
            relations("SELECT * FROM shop.public /* the schema */ . books"),
            ["public.books"]
        );
        // A table in parentheses, a join of them, and a list in them.
        assert_eq!(
            relations("SELECT * FROM (customers JOIN books ON true), ((orders), events)"),
            ["customers", "books", "orders", "events"]
        );
        assert_eq!(
            relations("SELECT * FROM books JOIN (customers) ON true"),
            ["books", "customers"]
        );
        // A table function is a call.
        let found = read("SELECT * FROM names() n, generate_series(1, 3)");
        assert!(found.relations.is_empty(), "{found:?}");
        assert!(found.functions.iter().any(|name| name.name == "names"));
        // What a WITH defines, and what it does not.
        assert_eq!(
            read(
                "WITH RECURSIVE a (x) AS (SELECT 1), b AS MATERIALIZED (SELECT * FROM a) \
                 SELECT * FROM b"
            )
            .defined,
            ["a", "b"]
        );
        assert!(
            read("SELECT * FROM unnest(ARRAY[1]) WITH ORDINALITY")
                .defined
                .is_empty()
        );
        // An index hint's FOR is in the FROM: the list goes on after it.
        let hinted = names(
            Dialect::MySql,
            "SELECT * FROM books FORCE INDEX FOR JOIN (PRIMARY), customers c",
        );
        assert!(
            hinted.relations.iter().any(|name| name.name == "customers"),
            "{hinted:?}"
        );
        // A keyword a table may be called, in a table's place.
        let last = names(Dialect::MySql, "SELECT * FROM last books, shop.view");
        assert_eq!(
            last.relations
                .iter()
                .map(|name| name.name.as_str())
                .collect::<Vec<_>>(),
            ["last", "view"]
        );
        // MySQL's backquotes, a doubled one inside.
        let mysql = names(Dialect::MySql, "SELECT * FROM `shop`.`odd``name`");
        assert_eq!(
            mysql.relations,
            [Name {
                schema: Some("shop".into()),
                name: "odd`name".into()
            }]
        );
    }

    #[test]
    fn a_hidden_table_is_refused_by_any_name_it_goes_by() {
        let hidden = hidden(Tables::AllExcept(vec![table(Some("public"), "customers")]));
        assert_eq!(statement(&hidden, "SELECT * FROM books"), Ok(()));
        for sql in [
            "SELECT * FROM customers",
            "SELECT * FROM public.customers",
            "SELECT * FROM \"Customers\"",
            "SELECT * FROM CUSTOMERS c",
            "SELECT * FROM books JOIN customers ON true",
            "SELECT * FROM books WHERE EXISTS (SELECT 1 FROM customers)",
            "SELECT * FROM books, LATERAL (SELECT * FROM customers) c",
            "SELECT 1 FROM books UNION SELECT 1 FROM customers",
            "WITH c AS (SELECT * FROM customers) SELECT * FROM c",
        ] {
            assert_eq!(
                statement(&hidden, sql),
                table_hidden("customers", None),
                "{sql}"
            );
        }
        // Another schema's table of the same name is not this one.
        assert_eq!(statement(&hidden, "SELECT * FROM sales.customers"), Ok(()));
        // A common table expression named like the hidden table.
        assert_eq!(
            statement(
                &hidden,
                "WITH customers AS (SELECT 1) SELECT * FROM customers"
            ),
            table_hidden("customers", None)
        );
    }

    #[test]
    fn what_depends_on_a_hidden_table_is_hidden_with_it_and_says_so() {
        let hidden = hidden(Tables::AllExcept(vec![table(None, "customers")]));
        // A view, a materialized view over that view, and a child table.
        assert_eq!(
            statement(&hidden, "SELECT * FROM customer_names"),
            table_hidden("customers", Some("customer_names"))
        );
        assert_eq!(
            statement(&hidden, "SELECT * FROM customer_count"),
            table_hidden("customer_names", Some("customer_count"))
        );
        assert_eq!(
            statement(&hidden, "SELECT * FROM vip"),
            table_hidden("customers", Some("vip"))
        );
        // Without a schema the entry is every schema's.
        assert_eq!(
            statement(&hidden, "SELECT * FROM sales.customers"),
            table_hidden("customers", None)
        );
        // A partition of a hidden table.
        let events = self::hidden(Tables::AllExcept(vec![table(None, "events")]));
        assert_eq!(
            events.check_engine_read(Some("public"), "events_2026"),
            table_hidden("events", Some("events_2026"))
        );
    }

    #[test]
    fn only_a_list_hides_the_rest_and_the_tables_made_later() {
        let hidden = hidden(Tables::Only(vec![
            table(None, "books"),
            table(None, "customer_names"),
        ]));
        assert_eq!(statement(&hidden, "SELECT * FROM books"), Ok(()));
        assert_eq!(
            statement(&hidden, "SELECT * FROM orders"),
            table_hidden("orders", None)
        );
        // A listed view over a table that is not listed is hidden too.
        assert_eq!(
            statement(&hidden, "SELECT * FROM customer_names"),
            table_hidden("customers", Some("customer_names"))
        );
        // A common table expression is no table to list.
        assert_eq!(
            statement(&hidden, "WITH x AS (SELECT * FROM books) SELECT * FROM x"),
            Ok(())
        );
        // A table the catalog did not list yet, as the engine reports it.
        assert_eq!(
            hidden.check_engine_read(Some("public"), "made_later"),
            table_hidden("made_later", None)
        );
        assert_eq!(hidden.check_engine_read(Some("public"), "books"), Ok(()));
    }

    #[test]
    fn the_system_catalogs_and_the_users_functions_are_refused_only_while_something_is_hidden() {
        let open = hidden(Tables::All);
        let hiding = hidden(Tables::AllExcept(vec![table(None, "customers")]));
        for sql in [
            "SELECT * FROM information_schema.tables",
            "SELECT * FROM pg_catalog.pg_class",
            "SELECT * FROM pg_stats",
        ] {
            assert_eq!(statement(&open, sql), Ok(()), "{sql}");
            assert_eq!(
                statement(&hiding, sql),
                Err(Refusal::SystemCatalog),
                "{sql}"
            );
        }
        assert_eq!(
            hiding.check_engine_read(Some("pg_catalog"), "pg_class"),
            Err(Refusal::SystemCatalog)
        );
        assert_eq!(
            open.check_engine_read(Some("pg_catalog"), "pg_class"),
            Ok(())
        );
        // A function a user wrote, in an expression and in a FROM.
        for sql in ["SELECT names()", "SELECT * FROM public.names()"] {
            assert_eq!(statement(&open, sql), Ok(()), "{sql}");
            assert_eq!(
                statement(&hiding, sql),
                Err(Refusal::FunctionNotAllowed("names".into())),
                "{sql}"
            );
        }
        assert_eq!(statement(&hiding, "SELECT lower(title) FROM books"), Ok(()));
        // Called by a word that is a keyword, as a call and as a column.
        let last = Catalog {
            routines: vec![("public".into(), "last".into())],
            ..bookshop()
        };
        let hiding_last = Hidden::new(
            Dialect::Postgres,
            &last,
            &Tables::AllExcept(vec![table(None, "customers")]),
        );
        for sql in ["SELECT last()", "SELECT b.last FROM books b"] {
            assert_eq!(
                statement(&hiding_last, sql),
                Err(Refusal::FunctionNotAllowed("last".into())),
                "{sql}"
            );
        }
        // SQLite's own tables, in any schema; MySQL's system schemas.
        let sqlite = Hidden::new(
            Dialect::Sqlite,
            &Catalog {
                relations: vec![relation("main", "customers", &[])],
                ..Catalog::default()
            },
            &Tables::AllExcept(vec![table(None, "customers")]),
        );
        let read = checked(Dialect::Sqlite, "SELECT * FROM temp.sqlite_master");
        assert_eq!(sqlite.check_statement(&read), Err(Refusal::SystemCatalog));
        let mysql = Hidden::new(
            Dialect::MySql,
            &Catalog::default(),
            &Tables::Only(Vec::new()),
        );
        assert_eq!(
            mysql.check_engine_read(Some("mysql"), "tbl"),
            Err(Refusal::SystemCatalog)
        );
    }

    #[test]
    fn an_explain_is_planned_as_the_query_it_explains() {
        let planned_of = |dialect, sql| planned(&checked(dialect, sql)).to_owned();
        assert_eq!(
            planned_of(Dialect::Postgres, "SELECT * FROM books"),
            "SELECT * FROM books"
        );
        assert_eq!(
            planned_of(
                Dialect::Postgres,
                "EXPLAIN (VERBOSE, FORMAT JSON) WITH x AS (SELECT 1) SELECT * FROM x"
            ),
            "WITH x AS (SELECT 1) SELECT * FROM x"
        );
        assert_eq!(
            planned_of(Dialect::Postgres, "EXPLAIN VERBOSE SELECT 1"),
            "SELECT 1"
        );
        assert_eq!(
            planned_of(Dialect::MySql, "EXPLAIN FORMAT=JSON SELECT 1"),
            "SELECT 1"
        );
        assert_eq!(
            planned_of(Dialect::Sqlite, "EXPLAIN QUERY PLAN SELECT 1"),
            "SELECT 1"
        );
    }

    #[test]
    fn a_function_no_statement_names_refuses_every_statement_while_anything_is_hidden() {
        let catalog = Catalog {
            unnamed_calls: vec!["shelf_eq".into()],
            ..bookshop()
        };
        let hiding = Tables::AllExcept(vec![table(None, "customers")]);
        let hidden = Hidden::new(Dialect::Postgres, &catalog, &hiding);
        // An operator the users wrote, which `=` on their type calls.
        assert_eq!(
            statement(&hidden, "SELECT * FROM books"),
            Err(Refusal::FunctionNotAllowed("shelf_eq".into()))
        );
        let open = Hidden::new(Dialect::Postgres, &catalog, &Tables::All);
        assert_eq!(statement(&open, "SELECT * FROM books"), Ok(()));
    }

    #[test]
    fn a_definition_that_runs_sql_in_a_string_is_found() {
        for definition in [
            " SELECT t.id, t.a FROM crosstab('SELECT id, name, email FROM customers'::text) \
             t(id integer, a text, b text);",
            " SELECT query_to_xml('SELECT * FROM customers'::text, true, false, ''::text) AS x;",
            " SELECT public.dblink('', 'SELECT 1'::text) AS d;",
        ] {
            assert!(
                calls_a_refused_function(Dialect::Postgres, definition),
                "{definition}"
            );
        }
        assert!(!calls_a_refused_function(
            Dialect::Postgres,
            " SELECT lower(name) AS name FROM customers;"
        ));
    }

    #[test]
    fn a_function_called_as_a_column_is_refused_on_postgres() {
        let hidden = hidden(Tables::AllExcept(vec![table(None, "customers")]));
        for sql in [
            "SELECT b.names FROM books b",
            "SELECT (b).\"names\" FROM books b",
        ] {
            assert_eq!(
                statement(&hidden, sql),
                Err(Refusal::FunctionNotAllowed("names".into())),
                "{sql}"
            );
        }
        // A column of another name, and the function's name not after a dot.
        assert_eq!(
            statement(&hidden, "SELECT b.title, 1 AS names FROM books b"),
            Ok(())
        );
        let open = self::hidden(Tables::All);
        assert_eq!(statement(&open, "SELECT b.names FROM books b"), Ok(()));
    }

    #[test]
    fn a_partition_goes_with_its_table() {
        let catalog = Catalog {
            relations: vec![
                relation("public", "events", &[]),
                Relation {
                    part: true,
                    ..relation("public", "events_2026", &[("public", "events")])
                },
            ],
            ..Catalog::default()
        };
        let only = Hidden::new(
            Dialect::Postgres,
            &catalog,
            &Tables::Only(vec![table(None, "events")]),
        );
        assert_eq!(
            only.check_engine_read(Some("public"), "events_2026"),
            Ok(())
        );
        // A partition the person hides by its name stays hidden.
        let except = Hidden::new(
            Dialect::Postgres,
            &catalog,
            &Tables::AllExcept(vec![table(None, "events_2026")]),
        );
        assert_eq!(
            except.check_engine_read(Some("public"), "events_2026"),
            table_hidden("events_2026", None)
        );
    }

    #[test]
    fn a_shadow_table_goes_with_its_virtual_table() {
        let catalog = Catalog {
            relations: vec![
                relation("main", "notes", &[]),
                Relation {
                    part: true,
                    ..relation("main", "notes_data", &[("main", "notes")])
                },
            ],
            ..Catalog::default()
        };
        // Listed alone, the virtual table reads its shadow tables.
        let only = Hidden::new(
            Dialect::Sqlite,
            &catalog,
            &Tables::Only(vec![table(None, "notes")]),
        );
        assert_eq!(only.check_engine_read(Some("main"), "notes_data"), Ok(()));
        // Hidden, it hides them.
        let except = Hidden::new(
            Dialect::Sqlite,
            &catalog,
            &Tables::AllExcept(vec![table(None, "notes")]),
        );
        assert_eq!(
            except.check_engine_read(Some("main"), "notes_data"),
            table_hidden("notes", Some("notes_data"))
        );
        // Not listed, neither reads.
        let other = Hidden::new(
            Dialect::Sqlite,
            &catalog,
            &Tables::Only(vec![table(None, "books")]),
        );
        assert_eq!(
            other.check_engine_read(Some("main"), "notes_data"),
            table_hidden("notes", Some("notes_data"))
        );
    }

    #[test]
    fn an_entry_that_names_no_table_closes_the_connection_until_it_is_looked_at() {
        // Renamed, or dropped: the "All except" list names what is gone.
        let renamed = hidden(Tables::AllExcept(vec![
            table(None, "customers"),
            table(None, "clients"),
            table(Some("sales"), "orders"),
        ]));
        assert_eq!(
            renamed.check_settings(),
            Err(Refusal::SettingsNeedALook {
                tables: vec!["clients".into(), "sales.orders".into()],
                columns: Vec::new(),
            })
        );
        assert_eq!(
            statement(&renamed, "SELECT * FROM books"),
            Err(Refusal::SettingsNeedALook {
                tables: vec!["clients".into(), "sales.orders".into()],
                columns: Vec::new(),
            })
        );
        // A name in another case is the table's; a system catalog's is
        // hidden with anything.
        let fine = hidden(Tables::AllExcept(vec![
            table(None, "CUSTOMERS"),
            table(Some("pg_catalog"), "pg_class"),
        ]));
        assert_eq!(fine.check_settings(), Ok(()));
        // An "Only" list that names what is gone hides more, not less.
        let only = hidden(Tables::Only(vec![table(None, "clients")]));
        assert_eq!(only.check_settings(), Ok(()));
    }

    #[test]
    fn an_entry_is_read_as_it_is_written() {
        for written in [
            "customers",
            "sales.customers",
            "\"odd.schema\".\"say \"\"hi\"\"\"",
            "\"a.b\"",
        ] {
            let read: TableName = written.parse().unwrap();
            assert_eq!(read.to_string(), written);
        }
        assert_eq!(
            "\"sales\".customers".parse::<TableName>(),
            Ok(table(Some("sales"), "customers"))
        );
        // What is wrong is said as it is: an empty quoted name has no dot.
        let why = |written: &str| written.parse::<TableName>().unwrap_err();
        assert_eq!(why("\"\""), "has a quoted name that is empty");
        assert_eq!(why("sales.\"\""), "has a quoted name that is empty");
        assert_eq!(why("sales."), "has nothing before or after a dot");
        assert_eq!(why(".orders"), "has nothing before or after a dot");
        assert_eq!(why(""), "is empty");
        for wrong in ["", "a.", ".b", "a.b.c", "\"a", "a\"b", "\"a\"b", "\"\""] {
            assert!(wrong.parse::<TableName>().is_err(), "{wrong:?}");
        }
    }

    #[test]
    fn an_entry_is_written_as_the_person_writes_it() {
        assert_eq!(table(None, "customers").to_string(), "customers");
        assert_eq!(
            table(Some("sales"), "customers").to_string(),
            "sales.customers"
        );
        assert_eq!(
            table(Some("odd.schema"), "say \"hi\"").to_string(),
            "\"odd.schema\".\"say \"\"hi\"\"\""
        );
    }

    #[test]
    fn nothing_is_hidden_by_an_empty_list_or_by_all() {
        for tables in [Tables::All, Tables::AllExcept(Vec::new())] {
            let hidden = hidden(tables);
            assert!(!hidden.hides_anything());
            assert_eq!(statement(&hidden, "SELECT * FROM customers"), Ok(()));
            assert_eq!(hidden.check_engine_read(None, "customers"), Ok(()));
        }
        // An empty "Only" hides every table.
        assert!(Tables::Only(Vec::new()).hides_anything());
    }

    /// The Bookshop with masked columns: `orders` and its partition, a
    /// view that reads `orders.email`, one that reads only its `id`, one
    /// over the first, and the columns of `orders`.
    fn masking(masked: &[&str], column_reads: Option<Vec<ColumnRead>>) -> Hidden {
        let view = |name: &str, on: &[(&str, &str)]| Relation {
            view: true,
            ..relation("public", name, on)
        };
        let catalog = Catalog {
            relations: vec![
                relation("public", "books", &[]),
                relation("public", "orders", &[]),
                Relation {
                    part: true,
                    ..relation("public", "orders_2026", &[("public", "orders")])
                },
                view("recent_orders", &[("public", "orders")]),
                view("order_ids", &[("public", "orders")]),
                view("recent_mail", &[("public", "recent_orders")]),
                view("partition_rows", &[("public", "orders_2026")]),
            ],
            routines: vec![("public".into(), "names".into())],
            columns: ["id", "email", "phone"]
                .iter()
                .map(|column| ("public".into(), "orders".into(), (*column).into()))
                .chain([
                    ("public".into(), "order_ids".into(), "id".into()),
                    ("public".into(), "orders_2026".into(), "email".into()),
                ])
                .collect(),
            column_reads,
            // A whole row of the partition, which `pg_depend` keeps no
            // column of.
            definitions: vec![(
                "public".into(),
                "partition_rows".into(),
                " SELECT to_jsonb(p.*) AS j\n   FROM orders_2026 p;".into(),
            )],
            ..Catalog::default()
        };
        let masked: Vec<ColumnName> = masked.iter().map(|entry| entry.parse().unwrap()).collect();
        Hidden::masking(Dialect::Postgres, &catalog, &Tables::All, &masked)
    }

    /// What PostgreSQL says the views read, column by column.
    fn reads() -> Option<Vec<ColumnRead>> {
        let read = |view: &str, table: &str, column: &str| ColumnRead {
            schema: "public".into(),
            view: view.into(),
            table_schema: "public".into(),
            table: table.into(),
            column: column.into(),
        };
        Some(vec![
            read("recent_orders", "orders", "email"),
            read("recent_orders", "orders", "id"),
            read("order_ids", "orders", "id"),
        ])
    }

    fn masked_used(column: &str, how: MaskedUse) -> Result<(), Refusal> {
        Err(Refusal::MaskedColumnUsed {
            column: column.into(),
            how,
            by: crate::agent::By::Person,
        })
    }

    #[test]
    fn a_view_that_reads_a_masked_column_is_refused() {
        let hidden = masking(&["orders.email"], reads());
        let through = |view: &str| masked_used("orders.email", MaskedUse::Through(view.into()));
        assert_eq!(
            statement(&hidden, "SELECT * FROM recent_orders"),
            through("recent_orders")
        );
        // A view over it reads it too; one that reads another column does
        // not.
        assert_eq!(
            statement(&hidden, "SELECT * FROM recent_mail"),
            through("recent_mail")
        );
        assert_eq!(statement(&hidden, "SELECT * FROM order_ids"), Ok(()));
        // As the engine names it, and not hidden by it.
        assert_eq!(
            hidden.check_engine_read(Some("public"), "recent_orders"),
            through("recent_orders")
        );
        assert!(!hidden.hides("public", "recent_orders"));
        assert_eq!(
            hidden.reads_masked("public", "recent_orders"),
            Some("orders.email".into())
        );
        assert_eq!(hidden.reads_masked("public", "order_ids"), None);
        assert_eq!(hidden.reads_masked("public", "orders"), None);
        // Where the catalog does not say what a view reads column by
        // column, a view over the table reads it.
        let unsaid = masking(&["orders.email"], None);
        assert_eq!(
            statement(&unsaid, "SELECT * FROM order_ids"),
            through("order_ids")
        );
        // A whole row a view's definition reads is every column of it,
        // a partition's too.
        assert_eq!(
            statement(&hidden, "SELECT j FROM partition_rows"),
            through("partition_rows")
        );
    }

    #[test]
    fn a_masked_column_is_masked_in_the_partitions_of_its_table() {
        let hidden = masking(&["orders.email"], reads());
        assert_eq!(
            hidden.masks(Some("public"), "orders_2026", "email"),
            Some("orders.email".into())
        );
        assert_eq!(hidden.masks(Some("public"), "orders_2026", "id"), None);
        assert_eq!(
            hidden.masks(Some("public"), "ORDERS", "Email"),
            Some("orders.email".into())
        );
        // An entry that names the partition masks its table too, which
        // reads the partition's rows.
        let part = masking(&["orders_2026.email"], reads());
        assert_eq!(
            part.masks(Some("public"), "orders", "email"),
            Some("orders_2026.email".into())
        );
        assert_eq!(
            statement(&part, "SELECT * FROM orders ORDER BY 1"),
            masked_used("orders_2026.email", MaskedUse::Sorted("1".into()))
        );
        // So a star over the partition is a star over the table.
        assert_eq!(
            statement(&hidden, "SELECT x.id FROM (SELECT * FROM orders_2026) x"),
            masked_used("orders.email", MaskedUse::Star)
        );
        // The engine's read of it for a view, or for a common table
        // expression, is refused; one the statement makes is its check's.
        assert_eq!(
            hidden.check_column_read(Some("main"), "orders", "email", Some("w")),
            masked_used("orders.email", MaskedUse::Through("w".into()))
        );
        assert_eq!(
            hidden.check_column_read(Some("main"), "orders", "email", None),
            Ok(())
        );
        assert_eq!(
            hidden.check_column_read(Some("main"), "orders", "id", Some("w")),
            Ok(())
        );
        // A view's own column, masked: the view reads, as a table with a
        // masked column does.
        let view = masking(&["order_ids.id"], reads());
        assert_eq!(view.check_settings(), Ok(()));
        assert_eq!(
            view.masks(Some("public"), "order_ids", "id"),
            Some("order_ids.id".into())
        );
        assert_eq!(statement(&view, "SELECT * FROM order_ids"), Ok(()));
        assert_eq!(
            statement(&view, "SELECT x.n FROM (SELECT * FROM order_ids) x"),
            masked_used("order_ids.id", MaskedUse::Star)
        );
    }

    #[test]
    fn a_generated_column_of_a_table_with_a_masked_column_is_masked_with_it() {
        let catalog = Catalog {
            relations: vec![
                relation("public", "orders", &[]),
                relation("public", "books", &[]),
            ],
            columns: vec![
                ("public".into(), "orders".into(), "email".into()),
                ("public".into(), "orders".into(), "low".into()),
                ("public".into(), "books".into(), "slug".into()),
            ],
            generated: vec![
                ("public".into(), "orders".into(), "low".into()),
                ("public".into(), "books".into(), "slug".into()),
            ],
            ..Catalog::default()
        };
        let masked: Vec<ColumnName> = vec!["orders.email".parse().unwrap()];
        let hidden = Hidden::masking(Dialect::Postgres, &catalog, &Tables::All, &masked);
        assert_eq!(
            hidden.masks(Some("public"), "orders", "low"),
            Some("public.orders.low".into())
        );
        assert_eq!(hidden.masks(Some("public"), "books", "slug"), None);
        assert_eq!(
            statement(&hidden, "SELECT id FROM orders WHERE low LIKE 'a%'"),
            masked_used("public.orders.low", MaskedUse::Named("low".into()))
        );
        assert_eq!(statement(&hidden, "SELECT id, low FROM orders"), Ok(()));
        assert_eq!(hidden.check_settings(), Ok(()));
    }

    #[test]
    fn a_masked_column_renamed_or_dropped_stops_agents_on_the_connection() {
        let gone = masking(
            &["orders.email", "orders.mail", "sales.orders.email"],
            reads(),
        );
        let look = Refusal::SettingsNeedALook {
            tables: Vec::new(),
            columns: vec!["orders.mail".into(), "sales.orders.email".into()],
        };
        assert_eq!(gone.check_settings(), Err(look.clone()));
        assert_eq!(statement(&gone, "SELECT * FROM books"), Err(look.clone()));
        assert_eq!(masking(&["ORDERS.EMAIL"], reads()).check_settings(), Ok(()));
        // The sentence names them.
        let said = look.to_string();
        assert!(
            said.contains("they mask orders.mail, sales.orders.email"),
            "{said}"
        );
    }

    #[test]
    fn a_connection_that_masks_a_column_closes_the_catalogs_and_the_users_functions() {
        let hidden = masking(&["orders.email"], reads());
        assert!(hidden.guards_anything());
        assert!(!hidden.hides_anything());
        assert_eq!(
            statement(&hidden, "SELECT relname FROM pg_catalog.pg_class"),
            Err(Refusal::SystemCatalog)
        );
        assert_eq!(
            statement(&hidden, "SELECT names()"),
            Err(Refusal::FunctionNotAllowed("names".into()))
        );
        // Masking nothing, they are open.
        let open = masking(&[], reads());
        assert!(!open.guards_anything());
        assert_eq!(
            statement(&open, "SELECT relname FROM pg_catalog.pg_class"),
            Ok(())
        );
    }

    /// The Bookshop with the person's `tables` and `masked`, and one rule
    /// of the administrator's for each of `rules`.
    fn sourced(person: Source, rules: Vec<Source>) -> Hidden {
        let sources = Sources {
            person,
            administrator: rules,
        };
        Hidden::of(Dialect::Postgres, &bookshop(), &sources)
    }

    fn hiding(tables: Tables) -> Source {
        Source {
            tables,
            masked: Vec::new(),
        }
    }

    fn by(refusal: Result<(), Refusal>, by: By) -> Result<(), Refusal> {
        refusal.map_err(|refusal| match refusal {
            Refusal::TableHidden { name, through, .. } => {
                Refusal::TableHidden { name, through, by }
            }
            Refusal::MaskedColumnUsed { column, how, .. } => {
                Refusal::MaskedColumnUsed { column, how, by }
            }
            other => other,
        })
    }

    #[test]
    fn a_rules_tables_are_hidden_beside_the_persons_and_say_whose_they_are() {
        let hidden = sourced(
            hiding(Tables::AllExcept(vec![table(None, "books")])),
            vec![hiding(Tables::AllExcept(vec![table(None, "customers")]))],
        );
        assert!(hidden.hides_anything());
        assert_eq!(
            statement(&hidden, "SELECT * FROM customers"),
            by(table_hidden("customers", None), By::Administrator)
        );
        assert_eq!(
            statement(&hidden, "SELECT * FROM customer_count"),
            by(
                table_hidden("customer_names", Some("customer_count")),
                By::Administrator
            )
        );
        assert_eq!(
            hidden.check_engine_read(Some("public"), "customer_names"),
            by(
                table_hidden("customers", Some("customer_names")),
                By::Administrator
            )
        );
        assert_eq!(
            statement(&hidden, "SELECT * FROM books"),
            table_hidden("books", None)
        );
        assert_eq!(statement(&hidden, "SELECT * FROM orders"), Ok(()));
        // A table the catalog does not list yet, by the names alone.
        let only = sourced(
            Source::default(),
            vec![hiding(Tables::Only(vec![table(None, "books")]))],
        );
        assert_eq!(
            only.check_engine_read(Some("public"), "made_today"),
            by(table_hidden("made_today", None), By::Administrator)
        );
        // The sentence says whose.
        let said = by(table_hidden("customers", None), By::Administrator)
            .unwrap_err()
            .to_string();
        assert_eq!(
            said,
            "customers is hidden for agents by your administrator's policy."
        );
    }

    #[test]
    fn two_only_lists_leave_the_tables_both_name() {
        let hidden = sourced(
            hiding(Tables::Only(vec![
                table(None, "books"),
                table(None, "orders"),
            ])),
            vec![hiding(Tables::Only(vec![
                table(None, "orders"),
                table(None, "events"),
            ]))],
        );
        assert_eq!(statement(&hidden, "SELECT * FROM orders"), Ok(()));
        assert_eq!(
            statement(&hidden, "SELECT * FROM books"),
            by(table_hidden("books", None), By::Administrator)
        );
        assert_eq!(
            statement(&hidden, "SELECT * FROM events"),
            table_hidden("events", None)
        );
        // A rule's Only list over the person's All except.
        let over = sourced(
            hiding(Tables::AllExcept(vec![table(None, "orders")])),
            vec![hiding(Tables::Only(vec![
                table(None, "books"),
                table(None, "orders"),
            ]))],
        );
        assert_eq!(statement(&over, "SELECT * FROM books"), Ok(()));
        assert_eq!(
            statement(&over, "SELECT * FROM orders"),
            by(table_hidden("orders", None), By::Person)
        );
        assert!(over.hides("public", "customers"));
    }

    #[test]
    fn a_rules_stale_entry_closes_the_connection_and_names_nothing_of_the_file() {
        let person_stale = hiding(Tables::AllExcept(vec![table(None, "gone")]));
        let rule_stale = hiding(Tables::AllExcept(vec![table(None, "secret_ledger")]));
        let hidden = sourced(person_stale.clone(), vec![rule_stale.clone()]);
        assert_eq!(hidden.check_settings(), Err(Refusal::PolicyNeedsFixing));
        assert_eq!(
            statement(&hidden, "SELECT * FROM books"),
            Err(Refusal::PolicyNeedsFixing)
        );
        let said = Refusal::PolicyNeedsFixing.to_string();
        assert_eq!(
            said,
            "Agents may not use this connection until your administrator's policy is fixed."
        );
        assert_eq!(Refusal::PolicyNeedsFixing.code(), "connection_unavailable");
        // The person's alone is theirs to look at.
        assert!(matches!(
            sourced(person_stale, Vec::new()).check_settings(),
            Err(Refusal::SettingsNeedALook { .. })
        ));
        // A rule's stale masked entry closes it too.
        let masks_gone = Source {
            tables: Tables::All,
            masked: vec!["orders.secret".parse().unwrap()],
        };
        assert_eq!(
            Hidden::of(
                Dialect::Postgres,
                &Catalog {
                    columns: vec![("public".into(), "orders".into(), "email".into())],
                    ..bookshop()
                },
                &Sources {
                    person: Source::default(),
                    administrator: vec![masks_gone],
                },
            )
            .check_settings(),
            Err(Refusal::PolicyNeedsFixing)
        );
        // A stale Only entry hides more and stops nothing.
        let only = sourced(
            Source::default(),
            vec![hiding(Tables::Only(vec![
                table(None, "books"),
                table(None, "secret_ledger"),
            ]))],
        );
        assert_eq!(only.check_settings(), Ok(()));
        assert_eq!(statement(&only, "SELECT * FROM books"), Ok(()));
    }

    #[test]
    fn a_rules_masked_column_is_masked_beside_the_persons_and_says_whose_it_is() {
        let catalog = Catalog {
            relations: vec![
                relation("public", "orders", &[]),
                Relation {
                    view: true,
                    ..relation("public", "recent_orders", &[("public", "orders")])
                },
            ],
            columns: ["id", "email", "phone"]
                .iter()
                .map(|column| ("public".into(), "orders".into(), (*column).into()))
                .collect(),
            ..Catalog::default()
        };
        let source = |masked: &[&str]| Source {
            tables: Tables::All,
            masked: masked.iter().map(|entry| entry.parse().unwrap()).collect(),
        };
        let hidden = Hidden::of(
            Dialect::Postgres,
            &catalog,
            &Sources {
                person: source(&["orders.phone", "orders.email"]),
                administrator: vec![source(&["orders.email"])],
            },
        );
        assert!(hidden.masks_anything());
        assert_eq!(
            statement(&hidden, "SELECT id FROM orders ORDER BY email"),
            by(
                masked_used("orders.email", MaskedUse::Named("email".into())),
                By::Administrator
            )
        );
        assert_eq!(
            statement(&hidden, "SELECT id FROM orders WHERE phone = '1'"),
            masked_used("orders.phone", MaskedUse::Named("phone".into()))
        );
        assert_eq!(hidden.masked_by("orders.email"), By::Administrator);
        assert_eq!(hidden.masked_by("orders.phone"), By::Person);
        assert_eq!(
            hidden.check_column_read(Some("public"), "orders", "email", Some("recent_orders")),
            by(
                masked_used("orders.email", MaskedUse::Through("recent_orders".into())),
                By::Administrator
            )
        );
        assert_eq!(
            hidden.check_engine_read(Some("public"), "recent_orders"),
            by(
                masked_used("orders.email", MaskedUse::Through("recent_orders".into())),
                By::Administrator
            )
        );
        let said = by(
            masked_used("orders.email", MaskedUse::Named("email".into())),
            By::Administrator,
        )
        .unwrap_err()
        .to_string();
        assert!(
            said.starts_with("orders.email is masked by your administrator's policy."),
            "{said}"
        );
    }
}

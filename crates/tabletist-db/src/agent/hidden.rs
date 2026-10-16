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

use std::collections::{HashMap, HashSet};

use super::{Read, Refusal};
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

/// A table as the person names it: in its schema, or by its name alone,
/// which is that name in every schema.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TableName {
    pub schema: Option<String>,
    pub name: String,
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
}

/// A relation of the catalog, as [`Hidden`] keeps it.
#[derive(Debug, Clone)]
struct Listed {
    schema: String,
    name: String,
    why: Option<Why>,
}

/// Why a relation is hidden.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Why {
    /// The person hides it.
    Named,
    /// It depends on this hidden relation, by its name.
    DependsOn(String),
}

/// What [`Tables`] comes to on one database: each relation that is hidden
/// and why, and the functions its users wrote, which are refused while
/// anything is hidden (B.2).
#[derive(Debug, Clone)]
pub struct Hidden {
    dialect: Dialect,
    tables: Tables,
    /// Every listed relation, by its name folded to lower case: its schema
    /// and its name as the catalog writes them, and why it is hidden where
    /// it is.
    relations: HashMap<String, Vec<Listed>>,
    /// The users' functions' names, folded, while anything is hidden.
    routines: HashSet<String>,
    /// [`Catalog::unnamed_calls`], while anything is hidden.
    unnamed_calls: Vec<String>,
}

impl Hidden {
    /// What `tables` hides of `catalog`, on a database that speaks
    /// `dialect`.
    pub fn new(dialect: Dialect, catalog: &Catalog, tables: &Tables) -> Self {
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
            hidden_why(at, catalog, &depends, tables, &mut why);
        }
        let mut relations: HashMap<String, Vec<Listed>> = HashMap::new();
        for (relation, why) in catalog.relations.iter().zip(why) {
            relations
                .entry(fold(&relation.name))
                .or_default()
                .push(Listed {
                    schema: relation.schema.clone(),
                    name: relation.name.clone(),
                    why: why.flatten(),
                });
        }
        let routines = match tables.hides_anything() {
            true => catalog
                .routines
                .iter()
                .map(|(_, name)| fold(name))
                .collect(),
            false => HashSet::new(),
        };
        let unnamed_calls = match tables.hides_anything() {
            true => catalog.unnamed_calls.clone(),
            false => Vec::new(),
        };
        Self {
            dialect,
            tables: tables.clone(),
            relations,
            routines,
            unnamed_calls,
        }
    }

    /// Whether anything is hidden.
    pub fn hides_anything(&self) -> bool {
        self.tables.hides_anything()
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
        if !self.hides_anything() {
            return Ok(());
        }
        if system_catalog(self.dialect, schema, name) {
            return Err(Refusal::SystemCatalog);
        }
        match self.listed(schema, name) {
            Some(found) => refused(found),
            None if self.tables.hides(schema, name) => Err(Refusal::TableHidden {
                name: name.to_owned(),
                through: None,
            }),
            None => Ok(()),
        }
    }

    /// Checks a relation the engine names where what it gives may be an
    /// alias of the statement's own (MySQL's plan): refused when it is a
    /// system catalog, or a listed relation that is hidden; a name the
    /// catalog does not list is passed over.
    pub fn check_name(&self, schema: Option<&str>, name: &str) -> Result<(), Refusal> {
        if !self.hides_anything() {
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
        if !self.hides_anything() {
            return Ok(());
        }
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
        match self.dialect {
            Dialect::Postgres => {
                for name in &names.after_dot {
                    self.check_function(name)?;
                }
            }
            Dialect::MySql | Dialect::Sqlite => {}
        }
        Ok(())
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
                .find(|listed| listed.why.is_some())
                .unwrap_or(first),
        )
    }
}

/// Refuses `found` where it is hidden, by the names the catalog writes.
fn refused(found: &Listed) -> Result<(), Refusal> {
    match &found.why {
        None => Ok(()),
        Some(Why::Named) => Err(Refusal::TableHidden {
            name: found.name.clone(),
            through: None,
        }),
        Some(Why::DependsOn(hidden)) => Err(Refusal::TableHidden {
            name: hidden.clone(),
            through: Some(found.name.clone()),
        }),
    }
}

/// Whether the relation at `at` is hidden, and why, worked out once: by
/// its own name, else by the first hidden relation it depends on. A
/// relation met again while its own answer is being worked out (a cycle
/// the catalog should not have) is taken for hidden.
fn hidden_why(
    at: usize,
    catalog: &Catalog,
    depends: &[Vec<usize>],
    tables: &Tables,
    why: &mut [Option<Option<Why>>],
) -> Option<Why> {
    if let Some(known) = &why[at] {
        return known.clone();
    }
    let relation = &catalog.relations[at];
    // Hidden until worked out, for a cycle.
    why[at] = Some(Some(Why::Named));
    let by_name = match (relation.part, tables) {
        (true, Tables::Only(_)) => false,
        _ => tables.hides(Some(&relation.schema), &relation.name),
    };
    let found = if relation.opaque || by_name {
        Some(Why::Named)
    } else {
        depends[at].iter().find_map(|&on| {
            hidden_why(on, catalog, depends, tables, why).map(|_| {
                let name = &catalog.relations[on].name;
                Why::DependsOn(name.clone())
            })
        })
    };
    // What a relation depends on through another is named by the one it
    // reaches first, which is hidden itself: its name is told, not the
    // hidden table at the end of the chain.
    why[at] = Some(found.clone());
    found
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
        // SQLite's own tables are `sqlite_` ones, in any schema.
        Dialect::Sqlite => fold(name).starts_with("sqlite_"),
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
    /// Called: a name with a parenthesis after it.
    functions: Vec<Name>,
    /// Every name with a dot before it: a column, or on PostgreSQL a
    /// function called as a column of a row.
    after_dot: Vec<String>,
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
    let pieces = pieces(dialect, text);
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
                    table_place(&pieces, at + 1, &mut found);
                }
            }
            Piece::Close => {
                from_at.retain(|&level| level < depth);
                depth = depth.saturating_sub(1);
            }
            Piece::Comma if from_at.last() == Some(&depth) => {
                table_place(&pieces, at + 1, &mut found);
            }
            Piece::Word { .. } if piece.is("FROM") => {
                from_at.retain(|&level| level < depth);
                from_at.push(depth);
                table_place(&pieces, at + 1, &mut found);
            }
            Piece::Word { .. } if JOINS.iter().any(|join| piece.is(join)) => {
                table_place(&pieces, at + 1, &mut found);
            }
            Piece::Word { .. } if ENDS_FROM.iter().any(|end| piece.is(end)) => {
                from_at.retain(|&level| level < depth);
            }
            Piece::Word { .. } if piece.is("WITH") => defined(&pieces, at + 1, &mut found),
            _ => {}
        }
        table_goes = places_table || (table_goes && matches!(piece, Piece::Open));
        if at > 0
            && pieces[at - 1] == Piece::Dot
            && let Some(name) = piece.name()
            && !found.after_dot.iter().any(|known| known == name)
        {
            found.after_dot.push(name.to_owned());
        }
        // A name with a parenthesis after it is a call, wherever it is.
        if let Some((name, after)) = dotted(&pieces, at)
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
    let Some((name, after)) = dotted_by(pieces, at, Piece::word) else {
        return;
    };
    // A table function: [`names`] takes it for a call.
    if pieces.get(after) == Some(&Piece::Open) {
        return;
    }
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
/// a catalog's, is passed over). Gives where it ends.
fn dotted(pieces: &[Piece], at: usize) -> Option<(Name, usize)> {
    dotted_by(pieces, at, Piece::name)
}

/// [`dotted`], with `part` saying which pieces may be a part of the name.
fn dotted_by(
    pieces: &[Piece],
    at: usize,
    part: fn(&Piece) -> Option<&str>,
) -> Option<(Name, usize)> {
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
                TokenKind::String
                | TokenKind::Number
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
            unnamed_calls: Vec::new(),
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
        // SQLite's own tables, in any schema; MySQL's system schemas.
        let sqlite = Hidden::new(
            Dialect::Sqlite,
            &Catalog::default(),
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
}

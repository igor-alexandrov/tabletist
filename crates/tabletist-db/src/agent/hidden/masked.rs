//! Masked columns: the columns of a connection whose values agents get as
//! `•••` (the MCP plan's B.4), and the reading of a statement that keeps
//! what is in them from being filtered, joined, sorted or computed on.
//!
//! The statement is read off its tokens, as its tables' names are (B.2:
//! the tree is not walked), and by a masked column's name alone: a word
//! that is one, whatever table or alias it is written with, may stand only
//! bare in the outermost `SELECT`'s list, where its value is replaced as it
//! is answered. That errs toward refusing. What the reading misses is each
//! engine's to stop.

use std::collections::{HashMap, HashSet};

use super::{Piece, Read, Refusal, closing, fold, parts, pieces, read_names, same};
use crate::agent::MaskedUse;

/// A column the person masks, as they name it: in its table, and in its
/// schema where they give one. One without a schema is that column of the
/// table of that name in every schema.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ColumnName {
    pub schema: Option<String>,
    pub table: String,
    pub column: String,
}

/// As the person writes it: `orders.email`, or `sales.orders.email`, a
/// part in double quotes where it holds a dot or a quote, as
/// [`super::TableName`]'s.
impl std::fmt::Display for ColumnName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let part = |part: &str| match part.contains(['.', '"']) {
            true => format!("\"{}\"", part.replace('"', "\"\"")),
            false => part.to_owned(),
        };
        if let Some(schema) = &self.schema {
            write!(f, "{}.", part(schema))?;
        }
        write!(f, "{}.{}", part(&self.table), part(&self.column))
    }
}

/// An entry as the person writes it, as `Display` writes it back. The
/// error says what is wrong without the entry's words.
impl std::str::FromStr for ColumnName {
    type Err = String;

    fn from_str(written: &str) -> Result<Self, String> {
        let mut parts = parts(written)?;
        let (Some(column), Some(table)) = (parts.pop(), parts.pop()) else {
            return Err("has no table: write the table, a dot, and the column".into());
        };
        let schema = parts.pop();
        if !parts.is_empty() {
            return Err(
                "has more than two dots: it is a table and a column, or a schema, a table and a \
                 column"
                    .into(),
            );
        }
        Ok(Self {
            schema,
            table,
            column,
        })
    }
}

impl ColumnName {
    /// Whether this names the column `column` of the relation `table` of
    /// `schema`.
    pub(crate) fn names(&self, schema: Option<&str>, table: &str, column: &str) -> bool {
        same(&self.column, column) && self.names_table(schema, table)
    }

    /// Whether this is a column of the relation `table` of `schema`.
    pub(crate) fn names_table(&self, schema: Option<&str>, table: &str) -> bool {
        same(&self.table, table)
            && match (&self.schema, schema) {
                (Some(own), Some(schema)) => same(own, schema),
                _ => true,
            }
    }
}

/// What the reading knows of the masked columns: their names, and the
/// names of the tables that have one, their parts and their child tables
/// with them, each with the entry a refusal names.
pub(super) struct Masks<'a> {
    columns: HashMap<String, &'a ColumnName>,
    tables: HashMap<String, &'a ColumnName>,
}

impl<'a> Masks<'a> {
    /// `masked`, and `more`: the other tables that have one of its
    /// columns, by name, each with its entry.
    pub(super) fn new(
        masked: &'a [ColumnName],
        more: impl IntoIterator<Item = (String, &'a ColumnName)>,
    ) -> Self {
        let mut columns = HashMap::new();
        let mut tables = HashMap::new();
        for entry in masked {
            columns.entry(fold(&entry.column)).or_insert(entry);
            tables.entry(fold(&entry.table)).or_insert(entry);
        }
        for (table, entry) in more {
            tables.entry(fold(&table)).or_insert(entry);
        }
        Self { columns, tables }
    }

    fn column(&self, word: &str) -> Option<&'a ColumnName> {
        self.columns.get(&fold(word)).copied()
    }

    pub(super) fn table(&self, word: &str) -> Option<&'a ColumnName> {
        self.tables.get(&fold(word)).copied()
    }

    /// The entry a refusal names when the statement names no table of
    /// one.
    fn any(&self) -> Option<&'a ColumnName> {
        self.columns.values().next().copied()
    }
}

/// The refusal for a use `how` of the masked column `entry`.
fn used(entry: &ColumnName, how: MaskedUse) -> Refusal {
    Refusal::MaskedColumnUsed {
        column: entry.to_string(),
        how,
    }
}

/// The words that end the outermost `SELECT`'s list at its depth.
const ENDS_LIST: &[&str] = &[
    "FROM",
    "INTO",
    "WHERE",
    "GROUP",
    "HAVING",
    "WINDOW",
    "QUALIFY",
    "ORDER",
    "LIMIT",
    "OFFSET",
    "FETCH",
    "UNION",
    "INTERSECT",
    "EXCEPT",
    "MINUS",
    "FOR",
    "LOCK",
];

/// The words between `SELECT` and its list that change nothing of what it
/// selects: `ALL`, and MySQL's hints.
const MODIFIERS: &[&str] = &[
    "ALL",
    "HIGH_PRIORITY",
    "STRAIGHT_JOIN",
    "SQL_SMALL_RESULT",
    "SQL_BIG_RESULT",
    "SQL_BUFFER_RESULT",
    "SQL_NO_CACHE",
    "SQL_CACHE",
    "SQL_CALC_FOUND_ROWS",
];

/// The clauses an outermost query may not have beside a masked column,
/// by the word each begins with, and as a refusal names it.
const CLAUSES: &[(&str, &str)] = &[
    ("GROUP", "GROUP BY"),
    ("HAVING", "HAVING"),
    ("WINDOW", "WINDOW"),
    ("QUALIFY", "QUALIFY"),
    ("OVER", "a window function"),
    ("UNION", "UNION, INTERSECT or EXCEPT"),
    ("INTERSECT", "UNION, INTERSECT or EXCEPT"),
    ("EXCEPT", "UNION, INTERSECT or EXCEPT"),
    ("MINUS", "UNION, INTERSECT or EXCEPT"),
];

/// What stands in the outermost list.
enum Item<'p> {
    /// A name by itself: its parts joined by dots, the last the column's,
    /// and the alias it is given.
    Bare {
        parts: usize,
        column: &'p str,
        alias: Option<&'p str>,
    },
    /// `*`, or `q.*` with its qualifier.
    Star { qualifier: Option<&'p str> },
    /// Anything else.
    Other,
}

/// Reads one item of a list for what it is.
fn item(slice: &[Piece]) -> Item<'_> {
    if let [rest @ .., Piece::Star] = slice {
        return match rest {
            [] => Item::Star { qualifier: None },
            [name @ .., Piece::Dot] if dotted_only(name) => Item::Star {
                qualifier: name.last().and_then(Piece::word),
            },
            _ => Item::Other,
        };
    }
    let mut at = 0;
    let mut parts = 0;
    let mut column = None;
    while let Some(word) = slice.get(at).and_then(Piece::word) {
        column = Some(word);
        parts += 1;
        at += 1;
        if slice.get(at) != Some(&Piece::Dot) {
            break;
        }
        at += 1;
    }
    let Some(column) = column else {
        return Item::Other;
    };
    if slice.get(at - 1) == Some(&Piece::Dot) {
        return Item::Other;
    }
    // An alias: after `AS` any word, without it a name only.
    let alias = match &slice[at..] {
        [] => None,
        [as_, alias] if as_.is("AS") => match alias.word() {
            Some(alias) => Some(alias),
            None => return Item::Other,
        },
        [alias] => match alias.name() {
            Some(alias) => Some(alias),
            None => return Item::Other,
        },
        _ => return Item::Other,
    };
    Item::Bare {
        parts,
        column,
        alias,
    }
}

/// Whether `slice` is a name and nothing else: words joined by dots.
fn dotted_only(slice: &[Piece]) -> bool {
    slice.len() % 2 == 1
        && slice.iter().enumerate().all(|(at, piece)| match at % 2 {
            0 => piece.word().is_some(),
            _ => *piece == Piece::Dot,
        })
}

/// `pieces[from..to]` split by its commas at the depth of `from`.
fn split(pieces: &[Piece], depth: &[usize], from: usize, to: usize) -> Vec<(usize, usize)> {
    let mut items = Vec::new();
    let mut start = from;
    for at in from..to {
        if pieces[at] == Piece::Comma && depth[at] == depth[from] {
            items.push((start, at));
            start = at + 1;
        }
    }
    if start < to {
        items.push((start, to));
    }
    items
}

/// The depth of each piece: a parenthesis is at the depth it opens from,
/// and what it holds one deeper.
fn depths(pieces: &[Piece]) -> Vec<usize> {
    let mut depth = Vec::with_capacity(pieces.len());
    let mut level = 0_usize;
    for piece in pieces {
        if *piece == Piece::Close {
            level = level.saturating_sub(1);
        }
        depth.push(level);
        if *piece == Piece::Open {
            level += 1;
        }
    }
    depth
}

/// Checks `read` against the masked columns `masks` knows: refused where
/// a word that names one stands anywhere but bare in the outermost
/// `SELECT`'s list, and where a statement over a table that has one reads
/// it some other way (B.4's rules 1 to 4).
pub(super) fn check(read: &Read, masks: &Masks<'_>) -> Result<(), Refusal> {
    let Some(any) = masks.any() else {
        return Ok(());
    };
    // A name written with escapes (`U&"…"`), which the reading by names
    // would not see, is refused before this is asked (`check_read`).
    let pieces = pieces(read.dialect(), read.text());
    let depth = depths(&pieces);
    let names = read_names(&pieces);
    // The first table the statement reads that has a masked column, the
    // names that stand for those tables (their own and their aliases), and
    // the pieces of every table's place, which name tables, not columns.
    let mut over: Option<&ColumnName> = None;
    let mut standing: HashMap<String, &ColumnName> = HashMap::new();
    let mut in_place: HashSet<usize> = HashSet::new();
    for place in &names.places {
        in_place.extend(place.from..place.to);
        in_place.extend(place.alias);
        let Some(entry) = masks.table(&place.name.name) else {
            continue;
        };
        over.get_or_insert(entry);
        standing.insert(fold(&place.name.name), entry);
        let alias = place.alias.and_then(|at| pieces[at].word());
        if let Some(alias) = alias {
            standing.insert(fold(alias), entry);
        }
        // `orders AS o(a, b, c)` gives its columns other names.
        if place.renames {
            let written = alias.unwrap_or(&place.name.name);
            return Err(used(entry, MaskedUse::Renamed(written.to_owned())));
        }
    }
    // The outermost `SELECT` and its list: what stands bare there is
    // allowed, and is what a plain query must keep to.
    let mut allowed: HashSet<usize> = HashSet::new();
    let mut masked_items: Vec<Option<&str>> = Vec::new();
    let mut distinct = false;
    let top = (0..pieces.len()).find(|&at| depth[at] == 0 && pieces[at].is("SELECT"));
    let mut list_end = pieces.len();
    if let Some(top) = top {
        let mut start = top + 1;
        while let Some(piece) = pieces.get(start) {
            if piece.is("DISTINCT") || piece.is("DISTINCTROW") {
                distinct = true;
                start += 1;
                // PostgreSQL's `DISTINCT ON (…)`.
                if pieces.get(start).is_some_and(|on| on.is("ON"))
                    && pieces.get(start + 1) == Some(&Piece::Open)
                {
                    start = closing(&pieces, start + 1) + 1;
                }
            } else if MODIFIERS.iter().any(|word| piece.is(word)) {
                start += 1;
            } else {
                break;
            }
        }
        list_end = (start..pieces.len())
            .find(|&at| depth[at] == 0 && ENDS_LIST.iter().any(|word| pieces[at].is(word)))
            .unwrap_or(pieces.len());
        for (from, to) in split(&pieces, &depth, start, list_end) {
            match item(&pieces[from..to]) {
                Item::Bare {
                    parts,
                    column,
                    alias,
                } => {
                    if parts == 1
                        && let Some(entry) = standing.get(&fold(column))
                    {
                        return Err(used(entry, MaskedUse::Row(column.to_owned())));
                    }
                    allowed.extend(from..to);
                    let masked = masks.column(column).is_some()
                        || alias.is_some_and(|alias| masks.column(alias).is_some());
                    if masked {
                        masked_items.push(alias);
                    }
                }
                Item::Star { qualifier } => {
                    allowed.extend(from..to);
                    let masked = match qualifier {
                        Some(qualifier) => standing.contains_key(&fold(qualifier)),
                        None => over.is_some(),
                    };
                    if masked {
                        masked_items.push(None);
                    }
                }
                Item::Other => {}
            }
        }
    }
    // `TABLE orders` is refused before this is asked (`check_read`).
    // Rule 2: beside a masked column, the outermost query is a plain one.
    if !masked_items.is_empty() {
        let entry = over.unwrap_or(any);
        if distinct {
            return Err(used(entry, MaskedUse::Clause("DISTINCT")));
        }
        for at in (0..pieces.len()).filter(|&at| depth[at] == 0) {
            if let Some((_, clause)) = CLAUSES.iter().find(|(word, _)| pieces[at].is(word)) {
                return Err(used(entry, MaskedUse::Clause(clause)));
            }
        }
        // `ORDER BY` a place, which may be a masked column's: SQLite reads
        // `(2)` and `+2` as the place too.
        let order = (list_end..pieces.len()).find(|&at| {
            depth[at] == 0
                && pieces[at].is("ORDER")
                && pieces.get(at + 1).is_some_and(|by| by.is("BY"))
        });
        if let Some(order) = order {
            let end = (order + 2..pieces.len())
                .find(|&at| {
                    depth[at] == 0
                        && ["LIMIT", "OFFSET", "FETCH", "FOR", "LOCK"]
                            .iter()
                            .any(|word| pieces[at].is(word))
                })
                .unwrap_or(pieces.len());
            for (from, to) in split(&pieces, &depth, order + 2, end) {
                let place = pieces[from..to]
                    .iter()
                    .find(|piece| !matches!(piece, Piece::Open | Piece::Other));
                if let Some(Piece::Number(written)) = place {
                    return Err(used(entry, MaskedUse::Sorted(written.clone())));
                }
            }
        }
    }
    // A masked column's alias stands for it: SQLite reads one in a
    // `WHERE`, and SQLite and MySQL in an `ORDER BY`'s expressions.
    let aliases: Vec<&str> = masked_items.iter().flatten().copied().collect();
    // Rules 3 and 4, everywhere else.
    for (at, piece) in pieces.iter().enumerate() {
        if allowed.contains(&at) || in_place.contains(&at) {
            continue;
        }
        match piece {
            Piece::Word { text, .. } => {
                if let Some(entry) = masks.column(text) {
                    return Err(used(entry, MaskedUse::Named(entry.column.clone())));
                }
                if let Some(alias) = aliases.iter().find(|alias| same(alias, text)) {
                    let entry = over.unwrap_or(any);
                    return Err(used(entry, MaskedUse::Alias((*alias).to_owned())));
                }
                if piece.is("NATURAL")
                    && let Some(entry) = over
                {
                    return Err(used(entry, MaskedUse::Natural));
                }
                // The table or its alias as a value: its whole row.
                let qualifies = pieces.get(at + 1) == Some(&Piece::Dot);
                let after_dot = at > 0 && pieces[at - 1] == Piece::Dot;
                if !qualifies
                    && !after_dot
                    && let Some(entry) = standing.get(&fold(text))
                {
                    return Err(used(entry, MaskedUse::Row(text.clone())));
                }
            }
            Piece::Star if selects(&pieces, at) => {
                let qualifier = (at >= 2 && pieces[at - 1] == Piece::Dot)
                    .then(|| pieces[at - 2].word())
                    .flatten();
                match qualifier {
                    Some(qualifier) => {
                        if let Some(entry) = standing.get(&fold(qualifier)) {
                            let row = format!("{qualifier}.*");
                            return Err(used(entry, MaskedUse::Row(row)));
                        }
                    }
                    None => {
                        if let Some(entry) = over {
                            return Err(used(entry, MaskedUse::Star));
                        }
                    }
                }
            }
            _ => {}
        }
    }
    Ok(())
}

/// Whether the star at `at` selects columns, as in a list (`SELECT *`,
/// `, *`, `q.*`, `DISTINCT ON (…) *`), and is not `count(*)` or a product.
fn selects(pieces: &[Piece], at: usize) -> bool {
    let Some(before) = at.checked_sub(1).map(|before| &pieces[before]) else {
        return false;
    };
    match before {
        Piece::Comma | Piece::Dot => true,
        Piece::Word { .. } => ["SELECT", "DISTINCT", "DISTINCTROW"]
            .iter()
            .chain(MODIFIERS)
            .any(|word| before.is(word)),
        Piece::Close => {
            let mut depth = 0_usize;
            let open = (0..at).rev().find(|&back| {
                match pieces[back] {
                    Piece::Close => depth += 1,
                    Piece::Open => depth = depth.saturating_sub(1),
                    _ => {}
                }
                depth == 0
            });
            open.is_some_and(|open| {
                open >= 2 && pieces[open - 1].is("ON") && pieces[open - 2].is("DISTINCT")
            })
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Dialect;
    use crate::agent::check_read;

    fn column(written: &str) -> ColumnName {
        written.parse().unwrap()
    }

    /// `sql` checked on a connection that masks `orders.email`.
    fn on(dialect: Dialect, sql: &str) -> Result<(), MaskedUse> {
        let masked = [column("orders.email")];
        let read = check_read(dialect, sql, &[]).unwrap_or_else(|error| panic!("{sql}: {error:?}"));
        check(&read, &Masks::new(&masked, [])).map_err(|refusal| match refusal {
            Refusal::MaskedColumnUsed { column, how } => {
                assert_eq!(column, "orders.email", "{sql}");
                how
            }
            other => panic!("{sql}: {other:?}"),
        })
    }

    fn pg(sql: &str) -> Result<(), MaskedUse> {
        on(Dialect::Postgres, sql)
    }

    #[test]
    fn a_masked_column_is_written_as_the_person_writes_it() {
        let read = column("orders.email");
        assert_eq!(
            (
                read.schema.as_deref(),
                read.table.as_str(),
                read.column.as_str()
            ),
            (None, "orders", "email")
        );
        let full = column("sales.orders.email");
        assert_eq!(full.schema.as_deref(), Some("sales"));
        assert_eq!(full.to_string(), "sales.orders.email");
        let quoted = column(r#""odd.table".email"#);
        assert_eq!(quoted.table, "odd.table");
        assert_eq!(quoted.to_string(), r#""odd.table".email"#);
        // A column alone is no entry, nor are four parts; the error never
        // says the entry's words.
        let alone = "email".parse::<ColumnName>().unwrap_err();
        assert!(alone.contains("no table"), "{alone}");
        assert!(!alone.contains("email"), "{alone}");
        assert!("a.b.c.d".parse::<ColumnName>().is_err());
        assert!("orders.".parse::<ColumnName>().is_err());
        // In another case, and in any schema without one.
        assert!(read.names(Some("public"), "Orders", "EMAIL"));
        assert!(!full.names(Some("public"), "orders", "email"));
        assert!(full.names(None, "orders", "email"));
    }

    #[test]
    fn a_masked_column_is_selected_as_it_is_and_used_no_other_way() {
        let named = || Err(MaskedUse::Named("email".into()));
        // The brief's cases.
        assert_eq!(pg("SELECT id FROM orders WHERE email LIKE 'a%'"), named());
        assert_eq!(pg("SELECT lower(email) FROM orders"), named());
        assert_eq!(pg("SELECT id FROM orders ORDER BY email"), named());
        assert_eq!(
            pg("SELECT id, email FROM orders ORDER BY 2"),
            Err(MaskedUse::Sorted("2".into()))
        );
        for sorted in ["(2)", "+2", "((2))", "2 COLLATE nocase"] {
            let sql = format!("SELECT id, email FROM orders ORDER BY {sorted}");
            assert_eq!(
                on(Dialect::Sqlite, &sql),
                Err(MaskedUse::Sorted("2".into())),
                "{sql}"
            );
        }
        assert!(pg("SELECT email FROM orders GROUP BY email").is_err());
        assert_eq!(pg("SELECT count(DISTINCT email) FROM orders"), named());
        assert_eq!(
            pg("SELECT o.id FROM orders o JOIN customers c ON c.email = o.email"),
            named()
        );
        // And the plan's.
        assert_eq!(
            pg("SELECT DISTINCT email FROM orders"),
            Err(MaskedUse::Clause("DISTINCT"))
        );
        assert_eq!(
            pg("SELECT DISTINCT ON (id) email FROM orders"),
            Err(MaskedUse::Clause("DISTINCT"))
        );
        assert_eq!(
            pg("SELECT email FROM orders UNION SELECT name FROM customers"),
            Err(MaskedUse::Clause("UNION, INTERSECT or EXCEPT"))
        );
        assert_eq!(
            pg("SELECT * FROM orders NATURAL JOIN customers"),
            Err(MaskedUse::Natural)
        );
        assert_eq!(
            pg("SELECT o.id FROM orders o JOIN customers c USING (email)"),
            named()
        );
        assert_eq!(
            pg("SELECT x.id FROM (SELECT * FROM orders) x WHERE x.id > 1"),
            Err(MaskedUse::Star)
        );
        assert_eq!(
            pg("SELECT c FROM orders AS t(a, b, c)"),
            Err(MaskedUse::Renamed("t".into()))
        );
        assert_eq!(
            pg("SELECT o FROM orders o"),
            Err(MaskedUse::Row("o".into()))
        );
        assert_eq!(
            pg("SELECT to_jsonb(o) FROM orders o"),
            Err(MaskedUse::Row("o".into()))
        );
        assert_eq!(
            pg("SELECT row_to_json(o.*) FROM orders o"),
            Err(MaskedUse::Row("o.*".into()))
        );
        assert_eq!(
            pg("SELECT id, row_number() OVER (ORDER BY email) FROM orders"),
            named()
        );
        assert_eq!(
            pg("SELECT email, row_number() OVER (ORDER BY id) FROM orders"),
            Err(MaskedUse::Clause("a window function"))
        );
        assert_eq!(pg("SELECT id FROM orders WHERE email IS NULL"), named());
        assert_eq!(
            pg("SELECT CASE WHEN email LIKE 'a%' THEN 1 END FROM orders"),
            named()
        );
        assert_eq!(pg("SELECT email::text FROM orders"), named());
        assert_eq!(pg("SELECT CAST(email AS text) FROM orders"), named());
        // Its alias stands for it, wherever else it is written.
        assert_eq!(
            pg("SELECT email AS e FROM orders ORDER BY e"),
            Err(MaskedUse::Alias("e".into()))
        );
        assert_eq!(
            pg("WITH w AS (SELECT email FROM orders) SELECT * FROM w"),
            named()
        );
        // By its name, whatever its table.
        assert_eq!(pg("SELECT id FROM customers WHERE email = 'x'"), named());
        assert_eq!(pg(r#"SELECT id FROM orders WHERE "EMAIL" = 'x'"#), named());
    }

    #[test]
    fn what_selects_a_masked_column_as_it_is_is_taken() {
        for sql in [
            "SELECT id, email FROM orders",
            "SELECT * FROM orders",
            "SELECT *, id FROM orders",
            "SELECT o.email AS e FROM orders o",
            "SELECT public.orders.email FROM public.orders",
            "SELECT o.* FROM orders o JOIN customers c ON c.id = o.customer_id",
            "SELECT count(*) FROM orders",
            "SELECT email FROM orders ORDER BY id LIMIT 5",
            "SELECT id FROM orders WHERE id IN (SELECT order_id FROM order_items)",
            "SELECT 2 * 3, email FROM orders",
            "EXPLAIN SELECT email FROM orders",
            "SELECT email FROM orders WHERE id = 3",
            // Beside no masked column, a query is as free as ever.
            "SELECT DISTINCT id FROM orders",
            "SELECT id FROM orders GROUP BY id",
            "SELECT * FROM customers UNION SELECT * FROM customers",
            "SELECT * FROM customers NATURAL JOIN books",
        ] {
            assert_eq!(pg(sql), Ok(()), "{sql}");
        }
    }

    #[test]
    fn mysql_and_sqlite_are_read_the_same_way() {
        let named = || Err(MaskedUse::Named("email".into()));
        assert_eq!(
            on(
                Dialect::MySql,
                "SELECT `email` FROM orders WHERE `email` = 'x'"
            ),
            named()
        );
        assert_eq!(
            on(Dialect::MySql, "SELECT SQL_NO_CACHE email FROM orders"),
            Ok(())
        );
        assert_eq!(
            on(Dialect::MySql, "SELECT DISTINCTROW email FROM orders"),
            Err(MaskedUse::Clause("DISTINCT"))
        );
        assert_eq!(
            on(
                Dialect::Sqlite,
                "SELECT id FROM orders WHERE email GLOB 'a*'"
            ),
            named()
        );
        assert_eq!(
            on(
                Dialect::Sqlite,
                "SELECT [email] FROM orders ORDER BY [email]"
            ),
            named()
        );
        assert_eq!(on(Dialect::Sqlite, "SELECT id, email FROM orders"), Ok(()));
        // SQLite reads an alias in a WHERE, and SQLite and MySQL in an
        // ORDER BY's expressions.
        let alias = || Err(MaskedUse::Alias("e".into()));
        assert_eq!(
            on(
                Dialect::Sqlite,
                "SELECT email AS e FROM orders WHERE e LIKE 'a%'"
            ),
            alias()
        );
        assert_eq!(
            on(
                Dialect::Sqlite,
                "SELECT id, email e FROM orders ORDER BY lower(e)"
            ),
            alias()
        );
        assert_eq!(
            on(
                Dialect::MySql,
                "SELECT id, email AS e FROM orders ORDER BY concat(e, '')"
            ),
            alias()
        );
    }
}

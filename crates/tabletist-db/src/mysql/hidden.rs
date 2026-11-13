//! What an agent may not read on MySQL and MariaDB, asked of the server
//! in the run's own transaction, before the statement runs (the MCP
//! plan's B.3): the catalog as it is at that moment, and the statement's
//! plan.
//!
//! MySQL's plan is two readings, and neither is enough alone. Its JSON
//! names each table it reads by the statement's alias where there is one,
//! and keeps a table that the optimizer read while planning (an `EXISTS`
//! it answered at once). The note `SHOW WARNINGS` gives after it is the
//! query as the server rewrote it, every table written `` `db`.`table` ``
//! with views expanded, and leaves out what the optimizer already
//! answered. So a table either names is checked, and the reading of the
//! statement's own text beside them. MariaDB writes the note on
//! `EXPLAIN EXTENDED` only, which is asked for where the JSON plan gave
//! none.
//!
//! What each view reads, MySQL's catalog lists. MariaDB's lists nothing of
//! it, and keeps each view's definition as the server writes it back,
//! every relation by its database and its name, which is read for it
//! ([`definition_reads`]).

use mysql_async::prelude::Queryable;

use super::query_error;
use crate::agent::{
    Catalog, ColumnName, Hidden, Read, Refusal, Relation, Sources, named_relations, planned,
};
use crate::sql::{self, TokenKind};
use crate::{Dialect, Error, Result};

/// The system's own schemas, which the catalog leaves out.
const SYSTEM: &str = "('information_schema', 'mysql', 'performance_schema', 'sys')";

/// The run sets `sql_select_limit` for the agent's rows; a `LIMIT` of a
/// query's own takes its place, so the catalog is read whole.
const WHOLE: &str = "LIMIT 18446744073709551615";

/// Checks `read` against what `sources` hide and mask, in the open
/// transaction. `Ok(Ok(masked))`: it may run, and `masked` says which
/// of its result columns are a masked column's (none where nothing is
/// masked). `Ok(Err(error))`: it may not, and `error` is why, a refusal or
/// the server's own error on the plan (the statement would have failed
/// with it). `Err` is a lost session or a cancel.
pub(super) async fn check(
    conn: &mut mysql_async::Conn,
    read: &Read,
    sources: &Sources,
) -> Result<std::result::Result<Vec<bool>, Error>> {
    let catalog = catalog(conn, &sources.masked()).await?;
    let hidden = Hidden::of(Dialect::MySql, &catalog, sources);
    if let Err(refusal) = hidden.check_statement(read) {
        return Ok(Err(Error::Agent(refusal)));
    }
    let explain = format!("EXPLAIN FORMAT=JSON {}", planned(read));
    let plan: Option<String> = match conn
        .exec_first(explain, super::script::bound(read.params()))
        .await
    {
        Ok(plan) => plan,
        Err(error) => {
            let error = query_error(error);
            if error.is_connection_lost() || error == Error::Cancelled {
                return Err(error);
            }
            return Ok(Err(error));
        }
    };
    let Some(Ok(plan)) = plan.map(|plan| serde_json::from_str::<serde_json::Value>(&plan)) else {
        return Ok(Err(Error::Agent(Refusal::NotParsed)));
    };
    // The note on the plan: the query as the server rewrote it. MariaDB
    // gives none on a JSON plan, only on `EXPLAIN EXTENDED`.
    let mut notes: Vec<(String, u32, String)> =
        conn.query("SHOW WARNINGS").await.map_err(query_error)?;
    if !notes.iter().any(|(_, code, _)| *code == 1003) {
        let extended = format!("EXPLAIN EXTENDED {}", planned(read));
        match conn
            .exec_drop(extended, super::script::bound(read.params()))
            .await
        {
            Ok(()) => notes = conn.query("SHOW WARNINGS").await.map_err(query_error)?,
            Err(error) => {
                let error = query_error(error);
                if error.is_connection_lost() || error == Error::Cancelled {
                    return Err(error);
                }
                // A server that takes no EXTENDED: without the note, as
                // before.
            }
        }
    }
    let rewritten: Vec<&String> = notes
        .iter()
        .filter(|(_, code, _)| *code == 1003)
        .map(|(_, _, note)| note)
        .collect();
    // Without the note, nothing says what an alias in the JSON stands
    // for: a name there must be one the text gives in a table's place.
    let named = named_relations(read);
    let strict = rewritten.is_empty().then_some(named.as_slice());
    if let Err(refusal) = json_reads(&plan, &hidden, strict) {
        return Ok(Err(Error::Agent(refusal)));
    }
    for note in rewritten {
        if let Err(refusal) = rewritten_reads(note, &hidden) {
            return Ok(Err(Error::Agent(refusal)));
        }
    }
    if !hidden.masks_anything() {
        return Ok(Ok(Vec::new()));
    }
    // Which result columns are a masked column's: by the table and the
    // column each comes from, as the prepared statement's column
    // definitions say (`org_table`, `org_name`), through an alias too. A
    // column the statement computes comes from none.
    let prepared = match conn.prep(read.text()).await {
        Ok(prepared) => prepared,
        Err(error) => {
            let error = query_error(error);
            if error.is_connection_lost() || error == Error::Cancelled {
                return Err(error);
            }
            return Ok(Err(error));
        }
    };
    Ok(Ok(prepared
        .columns()
        .iter()
        .map(|column| {
            let (schema, table) = (column.schema_str(), column.org_table_str());
            !table.is_empty()
                && hidden
                    .masks(Some(&schema), &table, &column.org_name_str())
                    .is_some()
        })
        .collect()))
}

/// Checks every table a JSON plan names. A name may be the statement's
/// alias, which the catalog does not list: such a name is passed over,
/// and the note and the text say what it stands for. `strict` is there
/// where there is no note: the names the text gives in a table's place,
/// and a name in the plan that is none of them is refused, as what it
/// stands for is not known. A derived table's name (`<derived2>`) is the
/// server's own.
fn json_reads(
    plan: &serde_json::Value,
    hidden: &Hidden,
    strict: Option<&[String]>,
) -> std::result::Result<(), Refusal> {
    match plan {
        serde_json::Value::Object(node) => {
            if let Some(name) = node.get("table_name").and_then(serde_json::Value::as_str)
                && !name.starts_with('<')
            {
                hidden.check_name(None, name)?;
                if let Some(named) = strict
                    && hidden.hides_anything()
                    && !named.iter().any(|named| named.eq_ignore_ascii_case(name))
                {
                    return Err(Refusal::NotParsed);
                }
            }
            node.values()
                .try_for_each(|value| json_reads(value, hidden, strict))
        }
        serde_json::Value::Array(values) => values
            .iter()
            .try_for_each(|value| json_reads(value, hidden, strict)),
        _ => Ok(()),
    }
}

/// Checks every table and every stored function the rewritten query
/// names. A name of two parts, `` `db`.`table` ``, is a table where a
/// `from` or a `join` comes before it or its first part is a schema of the
/// database, and a function where a parenthesis follows it. A base table's
/// column is written with three parts, a derived table's with two (its
/// alias and the column's), and an alias with one.
fn rewritten_reads(note: &str, hidden: &Hidden) -> std::result::Result<(), Refusal> {
    let tokens: Vec<sql::Token> = sql::tokenize(Dialect::MySql, note)
        .into_iter()
        .filter(|token| !matches!(token.kind, TokenKind::Whitespace | TokenKind::Comment))
        .collect();
    let quoted = |token: &sql::Token| {
        let written = &note[token.range.clone()];
        (token.kind == TokenKind::QuotedIdentifier && written.starts_with('`'))
            .then(|| written[1..written.len() - 1].replace("``", "`"))
    };
    let written = |token: Option<&sql::Token>, text: &str| {
        token.is_some_and(|token| note[token.range.clone()].eq_ignore_ascii_case(text))
    };
    let dot = |token: Option<&sql::Token>| written(token, ".");
    let mut at = 0;
    while at < tokens.len() {
        let Some(first) = quoted(&tokens[at]) else {
            at += 1;
            continue;
        };
        let mut parts = vec![first];
        let mut next = at + 1;
        while dot(tokens.get(next))
            && let Some(part) = tokens.get(next + 1).and_then(quoted)
        {
            parts.push(part);
            next += 2;
        }
        if let [schema, name] = parts.as_slice() {
            let before = at.checked_sub(1).and_then(|before| tokens.get(before));
            let placed = ["from", "join", "straight_join"]
                .iter()
                .any(|word| written(before, word));
            if written(tokens.get(next), "(") {
                hidden.check_function(name)?;
            } else if placed || hidden.knows_schema(schema) {
                hidden.check_engine_read(Some(schema), name)?;
            }
        }
        at = next;
    }
    Ok(())
}

/// The catalog as the open transaction sees it: the relations of every
/// schema but the system's, what each view reads where the server says so,
/// and the users' routines. MySQL 8.0.13 and later list what each view
/// reads (`VIEW_TABLE_USAGE`) and which views call a user's function
/// (`VIEW_ROUTINE_USAGE`); MariaDB keeps each view's definition as it
/// writes it back, which [`definition_reads`] reads. Where neither says,
/// a view is taken to read what is hidden. With the columns of the tables
/// `masked` names; neither server says what a view reads column by
/// column, so a view over a table with a masked column is taken to read
/// it.
pub(super) async fn catalog(
    conn: &mut mysql_async::Conn,
    masked: &[ColumnName],
) -> Result<Catalog> {
    let listed: Vec<(String, String, String)> = conn
        .query(format!(
            "SELECT TABLE_SCHEMA, TABLE_NAME, TABLE_TYPE FROM information_schema.TABLES \
             WHERE TABLE_SCHEMA NOT IN {SYSTEM} {WHOLE}"
        ))
        .await
        .map_err(query_error)?;
    let routines: Vec<(String, String)> = conn
        .query(format!(
            "SELECT ROUTINE_SCHEMA, ROUTINE_NAME FROM information_schema.ROUTINES \
             WHERE ROUTINE_SCHEMA NOT IN {SYSTEM} {WHOLE}"
        ))
        .await
        .map_err(query_error)?;
    let usage: Option<Vec<(String, String, String, String)>> = conn
        .query(format!(
            "SELECT VIEW_SCHEMA, VIEW_NAME, TABLE_SCHEMA, TABLE_NAME \
             FROM information_schema.VIEW_TABLE_USAGE {WHOLE}"
        ))
        .await
        .map(Some)
        .map_err(query_error)
        .or_else(|error| match error {
            Error::ConnectionLost(_) | Error::Cancelled => Err(error),
            // No such table on this server.
            _ => Ok(None),
        })?;
    // What each view reads, by the view, and the views of which that is
    // all they read: any other is taken to read what is hidden.
    let mut reads: std::collections::HashMap<(String, String), Vec<(String, String)>> =
        std::collections::HashMap::new();
    let mut told: std::collections::HashSet<(String, String)> = std::collections::HashSet::new();
    match usage {
        Some(usage) => {
            // The views that call a stored function the users wrote, whose
            // reads no catalog lists. Where the server does not say, every
            // view.
            let calling: Option<std::collections::HashSet<(String, String)>> = conn
                .query(format!(
                    "SELECT TABLE_SCHEMA, TABLE_NAME FROM information_schema.VIEW_ROUTINE_USAGE \
                     WHERE SPECIFIC_SCHEMA NOT IN {SYSTEM} {WHOLE}"
                ))
                .await
                .map(|rows: Vec<(String, String)>| Some(rows.into_iter().collect()))
                .map_err(query_error)
                .or_else(|error| match error {
                    Error::ConnectionLost(_) | Error::Cancelled => Err(error),
                    // No such table on this server.
                    _ => Ok(None),
                })?;
            for (view_schema, view_name, on_schema, on_name) in usage {
                let view = (view_schema, view_name);
                if calling
                    .as_ref()
                    .is_some_and(|calling| !calling.contains(&view))
                {
                    told.insert(view.clone());
                }
                reads.entry(view).or_default().push((on_schema, on_name));
            }
            // A view the server lists no table of reads what it does not
            // say: a user without SHOW VIEW is told nothing of any view.
            // It is not told, as one that calls a function is not.
        }
        None => {
            // Only MariaDB's own writing is read so: elsewhere (MySQL
            // before 8.0.13) no row comes back, and every view is taken to
            // read what is hidden.
            let definitions: Vec<(String, String, String)> = conn
                .query(format!(
                    "SELECT TABLE_SCHEMA, TABLE_NAME, VIEW_DEFINITION FROM information_schema.VIEWS \
                     WHERE TABLE_SCHEMA NOT IN {SYSTEM} AND VERSION() LIKE '%MariaDB%' {WHOLE}"
                ))
                .await
                .map_err(query_error)?;
            let functions: std::collections::HashSet<String> = routines
                .iter()
                .map(|(_, name)| name.to_lowercase())
                .collect();
            for (schema, name, definition) in definitions {
                if let Some(read) = definition_reads(&definition, &functions) {
                    told.insert((schema.clone(), name.clone()));
                    reads.insert((schema, name), read);
                }
            }
        }
    }
    let relations = listed
        .into_iter()
        .map(|(schema, name, kind)| {
            let view = kind.eq_ignore_ascii_case("VIEW");
            let key = (schema, name);
            let opaque = view && !told.contains(&key);
            let depends_on = reads.remove(&key).unwrap_or_default();
            let (schema, name) = key;
            Relation {
                opaque,
                schema,
                name,
                depends_on,
                part: false,
                view,
            }
        })
        .collect();
    // With whether each is generated, which `EXTRA` says on every server
    // (`VIRTUAL GENERATED`, `STORED GENERATED`, MariaDB's `PERSISTENT`).
    let listed: Vec<(String, String, String, String)> = match masked.is_empty() {
        true => Vec::new(),
        false => {
            let tables: Vec<String> = masked
                .iter()
                .map(|entry| entry.table.to_lowercase())
                .collect();
            let marks = vec!["?"; tables.len()].join(", ");
            conn.exec(
                format!(
                    "SELECT TABLE_SCHEMA, TABLE_NAME, COLUMN_NAME, UPPER(EXTRA) \
                     FROM information_schema.COLUMNS \
                     WHERE TABLE_SCHEMA NOT IN {SYSTEM} AND LOWER(TABLE_NAME) IN ({marks}) {WHOLE}"
                ),
                tables,
            )
            .await
            .map_err(query_error)?
        }
    };
    let generated = listed
        .iter()
        .filter(|(_, _, _, extra)| extra.contains("GENERATED") || extra.contains("PERSISTENT"))
        .map(|(schema, table, column, _)| (schema.clone(), table.clone(), column.clone()))
        .collect();
    let columns = listed
        .into_iter()
        .map(|(schema, table, column, _)| (schema, table, column))
        .collect();
    Ok(Catalog {
        relations,
        routines,
        columns,
        generated,
        ..Catalog::default()
    })
}

/// What a view reads, by its definition as MariaDB writes it back
/// (`information_schema.VIEWS`), whatever the text it was made with: every
/// table, view and sequence as `` `db`.`name` ``, a stored function it
/// calls in backquotes (`` `name`( `` or `` `db`.`name`( ``, the built-in
/// ones bare), a common table expression by its name alone. Every name of
/// two parts that is not called is taken for a relation it reads; one the
/// catalog does not list is passed over (a derived table's column,
/// `` `d`.`id` ``).
///
/// `None` where what it reads cannot be told: an empty definition (a user
/// without SHOW VIEW is shown none), a call of a function the users wrote
/// (`functions`, folded) or of any name in backquotes, a name alone in a
/// table's place that no common table expression of the definition has,
/// and what the server never writes, which a misreading could hide a name
/// in: a comment, or a string with a backquote in it.
fn definition_reads(
    definition: &str,
    functions: &std::collections::HashSet<String>,
) -> Option<Vec<(String, String)>> {
    if definition.trim().is_empty() {
        return None;
    }
    let tokens: Vec<sql::Token> = sql::tokenize(Dialect::MySql, definition)
        .into_iter()
        .filter(|token| token.kind != TokenKind::Whitespace)
        .collect();
    let text = |at: usize| tokens.get(at).map(|token| &definition[token.range.clone()]);
    let is = |at: usize, word: &str| text(at).is_some_and(|text| text.eq_ignore_ascii_case(word));
    let quoted = |at: usize| {
        tokens
            .get(at)
            .filter(|token| token.kind == TokenKind::QuotedIdentifier)
            .and_then(|_| text(at))
            .filter(|text| text.starts_with('`'))
            .map(|text| text[1..text.len() - 1].replace("``", "`"))
    };
    // A name, in backquotes or bare (a word the tokenizer takes for a
    // keyword too).
    let name = |at: usize| match tokens.get(at).map(|token| token.kind) {
        Some(TokenKind::Identifier | TokenKind::Keyword) => text(at).map(str::to_owned),
        Some(TokenKind::QuotedIdentifier) => quoted(at),
        _ => None,
    };
    // The common table expressions' names: each one a name that `AS (`
    // follows, or a list of its columns and then `AS (`.
    let mut defined = std::collections::HashSet::new();
    for at in 0..tokens.len() {
        let Some(cte) = name(at) else { continue };
        let mut after = at + 1;
        if is(after, "(") {
            after += 1;
            while name(after).is_some() || is(after, ",") {
                after += 1;
            }
            if !is(after, ")") {
                continue;
            }
            after += 1;
        }
        if is(after, "as") && is(after + 1, "(") {
            defined.insert(cte.to_lowercase());
        }
    }
    let mut reads = Vec::new();
    for (at, token) in tokens.iter().enumerate() {
        match token.kind {
            TokenKind::Comment | TokenKind::ExecutableComment => return None,
            TokenKind::String if text(at)?.contains('`') => return None,
            _ => {}
        }
        if ["from", "join", "straight_join"]
            .iter()
            .any(|word| is(at, word))
            && let Some(alone) = name(at + 1)
            && !is(at + 2, ".")
            && !is(at + 2, "(")
            && !defined.contains(&alone.to_lowercase())
        {
            return None;
        }
        if is(at + 1, "(")
            && let Some(called) = name(at)
            && !defined.contains(&called.to_lowercase())
            && (quoted(at).is_some() || functions.contains(&called.to_lowercase()))
        {
            return None;
        }
        // A name of two parts, not of three nor called.
        if (at == 0 || !is(at - 1, "."))
            && is(at + 1, ".")
            && !is(at + 3, ".")
            && !is(at + 3, "(")
            && let (Some(schema), Some(relation)) = (name(at), name(at + 2))
        {
            reads.push((schema, relation));
        }
    }
    Some(reads)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::{TableName, Tables};

    fn bookshop(tables: Tables) -> Hidden {
        let relation = |name: &str| Relation {
            schema: "bookshop".into(),
            name: name.into(),
            depends_on: Vec::new(),
            opaque: false,
            part: false,
            view: false,
        };
        let catalog = Catalog {
            relations: vec![relation("books"), relation("customers")],
            routines: vec![("bookshop".into(), "customer_count".into())],
            ..Catalog::default()
        };
        Hidden::new(Dialect::MySql, &catalog, &tables)
    }

    #[test]
    fn the_rewritten_query_names_each_table_by_its_database_and_its_name() {
        let hidden = bookshop(Tables::AllExcept(vec![TableName {
            schema: None,
            name: "customers".into(),
        }]));
        let books = "/* select#1 */ select `bookshop`.`b`.`title` AS `title` \
                     from `bookshop`.`books` `b`";
        assert_eq!(rewritten_reads(books, &hidden), Ok(()));
        // Its alias is `b`, and its own name is still said.
        let customers = "/* select#1 */ select `bookshop`.`b`.`id` AS `id` \
                         from `bookshop`.`customers` `b` where (`bookshop`.`b`.`id` = 1)";
        assert_eq!(
            rewritten_reads(customers, &hidden),
            Err(Refusal::TableHidden {
                name: "customers".into(),
                through: None,
                by: crate::agent::By::Person,
            })
        );
        // A stored function, as a view that calls it is rewritten.
        let function = "/* select#1 */ select `bookshop`.`customer_count`() AS `n` \
                        from `bookshop`.`books`";
        assert_eq!(
            rewritten_reads(function, &hidden),
            Err(Refusal::FunctionNotAllowed("customer_count".into()))
        );
        // A derived table's column is no table, also where an "Only" list
        // hides every name it does not give.
        let only = bookshop(Tables::Only(vec![TableName {
            schema: None,
            name: "books".into(),
        }]));
        let derived = "/* select#1 */ select `d`.`id` AS `id` from (/* select#2 */ select \
                       `bookshop`.`books`.`id` AS `id` from `bookshop`.`books` group by \
                       `bookshop`.`books`.`id`) `d`";
        assert_eq!(rewritten_reads(derived, &only), Ok(()));
        // The system's own, as the information schema is rewritten.
        let system = "select `mysql`.`tbl`.`name` AS `TABLE_NAME` from `mysql`.`tables` `tbl`";
        assert_eq!(
            rewritten_reads(system, &hidden),
            Err(Refusal::SystemCatalog)
        );
        // An alias that is a hidden table's name is not read as one in the
        // plan's JSON unless the catalog lists it so.
        let plan: serde_json::Value = serde_json::json!({
            "query_block": { "table": { "table_name": "b" } }
        });
        assert_eq!(json_reads(&plan, &hidden, None), Ok(()));
        // Without the rewritten query, an alias is not known: refused,
        // also where it is a table's name that the text does not give
        // (`FROM last books`, `last` hidden).
        let named = ["books".to_owned()];
        assert_eq!(
            json_reads(&plan, &hidden, Some(&named)),
            Err(Refusal::NotParsed)
        );
        let books = serde_json::json!({
            "query_block": { "table": { "table_name": "books" } }
        });
        assert_eq!(json_reads(&books, &hidden, Some(&named)), Ok(()));
        let last = ["last".to_owned()];
        assert_eq!(
            json_reads(&books, &hidden, Some(&last)),
            Err(Refusal::NotParsed)
        );
        let plan: serde_json::Value = serde_json::json!({
            "query_block": { "nested_loop": [
                { "table": { "table_name": "<derived2>" } },
                { "table": { "table_name": "customers" } }
            ] }
        });
        assert!(json_reads(&plan, &hidden, None).is_err());
    }

    /// Definitions as MariaDB 11.4 wrote them back.
    #[test]
    fn a_mariadb_view_reads_each_relation_its_definition_names() {
        let functions: std::collections::HashSet<String> =
            ["customer_count".to_owned()].into_iter().collect();
        let reads = |definition: &str| {
            definition_reads(definition, &functions).map(|reads| {
                reads
                    .iter()
                    .map(|(schema, name)| format!("{schema}.{name}"))
                    .collect::<Vec<_>>()
            })
        };
        let recent = "select `bookshop`.`orders`.`id` AS `id`,`bookshop`.`orders`.`email` AS \
                      `email` from `bookshop`.`orders` where `bookshop`.`orders`.`placed_at` \
                      >= '2026-01-01'";
        assert_eq!(reads(recent), Some(vec!["bookshop.orders".into()]));
        // By aliases, in a join in parentheses, and on another database:
        // a column by its alias is no relation the catalog lists.
        let in_stock = "select `b`.`id` AS `book_id`,`p`.`name` AS `publisher` from \
                        (`bookshop`.`books` `b` join `shop`.`publishers` `p` \
                        on(`p`.`id` = `b`.`publisher_id`)) where `b`.`in_print` <> 0";
        assert_eq!(
            reads(in_stock),
            Some(vec![
                "b.id".into(),
                "p.name".into(),
                "bookshop.books".into(),
                "shop.publishers".into(),
                "p.id".into(),
                "b.publisher_id".into(),
                "b.in_print".into(),
            ])
        );
        // Under a subquery, EXISTS, IN, a derived table, a union, and a
        // sequence.
        let nested = "select (select max(`d`.`t`.`id`) from `d`.`t`) AS `m` from `d`.`u` \
                      where exists(select 1 from `d`.`x` `x` limit 1) and `d`.`u`.`id` in \
                      (select `d`.`y`.`id` from `d`.`y`) union select nextval(`d`.`s`) AS `n` \
                      from ((select `d`.`z`.`id` AS `id` from `d`.`z`) `q` join `d`.`w`)";
        let found = reads(nested).unwrap();
        for relation in ["d.t", "d.u", "d.x", "d.y", "d.s", "d.z", "d.w"] {
            assert!(
                found.contains(&relation.to_owned()),
                "{relation}: {found:?}"
            );
        }
        // A common table expression, by its name alone, with its columns
        // or without.
        let cte = "with c as (select `d`.`t`.`id` AS `id` from `d`.`t`)select `c`.`id` AS \
                   `id` from `c`";
        assert_eq!(reads(cte), Some(vec!["d.t".into(), "c.id".into()]));
        let recursive = "with recursive r(`n`) as (select 1 AS `n` union all select `r`.`n` + 1 \
                         AS `n+1` from `r` where `r`.`n` < 3)select `r`.`n` AS `n` from `r`";
        assert!(reads(recursive).is_some());
        // A table function, and a column of it by its alias.
        let table_function = "select `j`.`a` AS `a` from JSON_TABLE('[1]', '$[*]' COLUMNS \
                              (`a` int(11) PATH '$')) `j`";
        assert_eq!(reads(table_function), Some(vec!["j.a".into()]));
        // Nothing read: a literal, a FROM of a function's own, and a string
        // that reads like a name.
        for definition in [
            "select 'Welcome' AS `greeting`",
            "values (1),(2)",
            "select trim(both 'x' from 'xax') AS `y`,extract(year from current_timestamp()) \
             AS `z`",
            "select 'it\\'s from \\\\ a' AS `s`",
        ] {
            assert_eq!(reads(definition), Some(Vec::new()), "{definition}");
        }
        // Not told: a stored function called, however it is written; none
        // shown; a name alone in a table's place; a comment; a backquote in
        // a string.
        for definition in [
            "select `customer_count`() AS `n`",
            "select `bookshop`.`customer_count`() AS `n` from `bookshop`.`books`",
            "select customer_count() AS `n`",
            "select `own_function`(1) AS `n`",
            "",
            "select `customers`.`id` AS `id` from `customers`",
            "select 1 AS `1` from `d`.`t` join `customers`",
            "select 1 AS `1` /* from `d`.`customers` */",
            "select 'a`b' AS `s` from `d`.`t`",
        ] {
            assert_eq!(reads(definition), None, "{definition}");
        }
    }
}

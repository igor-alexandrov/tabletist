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
//! statement's own text beside them.

use mysql_async::prelude::Queryable;

use super::query_error;
use crate::agent::{Catalog, Hidden, Read, Refusal, Relation, Tables, planned};
use crate::sql::{self, TokenKind};
use crate::{Dialect, Error, Result};

/// The system's own schemas, which the catalog leaves out.
const SYSTEM: &str = "('information_schema', 'mysql', 'performance_schema', 'sys')";

/// The run sets `sql_select_limit` for the agent's rows; a `LIMIT` of a
/// query's own takes its place, so the catalog is read whole.
const WHOLE: &str = "LIMIT 18446744073709551615";

/// Checks `read` against what `tables` hides, in the open transaction.
/// `Ok(None)`: it may run. `Ok(Some(error))`: it may not, and `error` is
/// why, a refusal or the server's own error on the plan (the statement
/// would have failed with it). `Err` is a lost session or a cancel.
pub(super) async fn check(
    conn: &mut mysql_async::Conn,
    read: &Read,
    tables: &Tables,
) -> Result<Option<Error>> {
    let catalog = catalog(conn).await?;
    let hidden = Hidden::new(Dialect::MySql, &catalog, tables);
    if let Err(refusal) = hidden.check_statement(read) {
        return Ok(Some(Error::Agent(refusal)));
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
            return Ok(Some(error));
        }
    };
    let Some(Ok(plan)) = plan.map(|plan| serde_json::from_str::<serde_json::Value>(&plan)) else {
        return Ok(Some(Error::Agent(Refusal::NotParsed)));
    };
    if let Err(refusal) = json_reads(&plan, &hidden) {
        return Ok(Some(Error::Agent(refusal)));
    }
    // The note on the plan: the query as the server rewrote it. MariaDB
    // may give none.
    let notes: Vec<(String, u32, String)> =
        conn.query("SHOW WARNINGS").await.map_err(query_error)?;
    for (_, code, note) in notes {
        if code == 1003
            && let Err(refusal) = rewritten_reads(&note, &hidden)
        {
            return Ok(Some(Error::Agent(refusal)));
        }
    }
    Ok(None)
}

/// Checks every table a JSON plan names. A name may be the statement's
/// alias, which the catalog does not list: such a name is passed over,
/// and the note and the text say what it stands for. A derived table's
/// name (`<derived2>`) is the server's own.
fn json_reads(plan: &serde_json::Value, hidden: &Hidden) -> std::result::Result<(), Refusal> {
    match plan {
        serde_json::Value::Object(node) => {
            if let Some(name) = node.get("table_name").and_then(serde_json::Value::as_str)
                && !name.starts_with('<')
            {
                hidden.check_name(None, name)?;
            }
            node.values()
                .try_for_each(|value| json_reads(value, hidden))
        }
        serde_json::Value::Array(values) => values
            .iter()
            .try_for_each(|value| json_reads(value, hidden)),
        _ => Ok(()),
    }
}

/// Checks every table the rewritten query names: each name of two parts,
/// `` `db`.`table` ``, is one. A column is written with three, and an
/// alias with one.
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
    let dot =
        |token: Option<&sql::Token>| token.is_some_and(|token| &note[token.range.clone()] == ".");
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
        if let [schema, table] = parts.as_slice() {
            hidden.check_engine_read(Some(schema), table)?;
        }
        at = next;
    }
    Ok(())
}

/// The catalog as the open transaction sees it: the relations of every
/// schema but the system's, what each view reads where the server says so
/// (`VIEW_TABLE_USAGE`, MySQL 8.0.13 and later), and the users' routines.
/// Where it does not say, every view is taken to read what is hidden.
async fn catalog(conn: &mut mysql_async::Conn) -> Result<Catalog> {
    let listed: Vec<(String, String, String)> = conn
        .query(format!(
            "SELECT TABLE_SCHEMA, TABLE_NAME, TABLE_TYPE FROM information_schema.TABLES \
             WHERE TABLE_SCHEMA NOT IN {SYSTEM} {WHOLE}"
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
    // What each view reads, by the view.
    let mut reads: std::collections::HashMap<(String, String), Vec<(String, String)>> =
        std::collections::HashMap::new();
    for (view_schema, view_name, on_schema, on_name) in usage.iter().flatten() {
        reads
            .entry((view_schema.clone(), view_name.clone()))
            .or_default()
            .push((on_schema.clone(), on_name.clone()));
    }
    let relations = listed
        .into_iter()
        .map(|(schema, name, kind)| {
            let view = kind.eq_ignore_ascii_case("VIEW");
            let depends_on = reads
                .remove(&(schema.clone(), name.clone()))
                .unwrap_or_default();
            Relation {
                schema,
                name,
                depends_on,
                opaque: view && usage.is_none(),
            }
        })
        .collect();
    let routines: Vec<(String, String)> = conn
        .query(format!(
            "SELECT ROUTINE_SCHEMA, ROUTINE_NAME FROM information_schema.ROUTINES \
             WHERE ROUTINE_SCHEMA NOT IN {SYSTEM} {WHOLE}"
        ))
        .await
        .map_err(query_error)?;
    Ok(Catalog {
        relations,
        routines,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::TableName;

    fn bookshop(tables: Tables) -> Hidden {
        let relation = |name: &str| Relation {
            schema: "bookshop".into(),
            name: name.into(),
            depends_on: Vec::new(),
            opaque: false,
        };
        let catalog = Catalog {
            relations: vec![relation("books"), relation("customers")],
            routines: Vec::new(),
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
                through: None
            })
        );
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
        assert_eq!(json_reads(&plan, &hidden), Ok(()));
        let plan: serde_json::Value = serde_json::json!({
            "query_block": { "nested_loop": [
                { "table": { "table_name": "<derived2>" } },
                { "table": { "table_name": "customers" } }
            ] }
        });
        assert!(json_reads(&plan, &hidden).is_err());
    }
}

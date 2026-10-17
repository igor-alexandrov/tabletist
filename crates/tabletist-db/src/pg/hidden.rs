//! What an agent may not read on PostgreSQL, asked of the server in the
//! run's own transaction, before the statement runs (the MCP plan's B.3):
//! the catalog as it is at that moment, and the statement's plan.
//!
//! The plan is the fence. `EXPLAIN (VERBOSE, FORMAT JSON)` names every
//! relation the statement would scan, with its schema, after the server
//! has expanded its views, found its partitions and its child tables, and
//! resolved every name by the session's own search path. What the plan
//! cannot name (a function's body) is the catalog's: the functions the
//! database's users wrote are refused while anything is hidden.

use tokio_postgres::SimpleQueryMessage;

use super::query_error;
use crate::agent::{
    Catalog, ColumnName, ColumnRead, Hidden, Read, Refusal, Relation, Tables,
    calls_a_refused_function, function_allowed, planned,
};
use crate::{Dialect, Error, Result};

/// The functions the database's users wrote: not the system's, and not an
/// extension's. A common table expression of the queries below.
const USER_ROUTINES: &str = "\
user_routines AS (
    SELECT p.oid, n.nspname, p.proname
    FROM pg_proc p
    JOIN pg_namespace n ON n.oid = p.pronamespace
    WHERE n.nspname NOT IN ('pg_catalog', 'information_schema')
        AND NOT EXISTS (
            SELECT 1 FROM pg_depend e
            WHERE e.classid = 'pg_proc'::regclass AND e.objid = p.oid AND e.deptype = 'e'
        )
)";

/// Every relation of the database but the system's, each with what it
/// depends on: the relations a view's or a materialized view's rules
/// read, and a partition's or a child table's parent. One row for each
/// relation and each thing it depends on; then whether its rules or a
/// policy on it call a function the users wrote, which makes what it reads
/// unknown, whether it is a partition, and its view's definition and its
/// policies' expressions as the server writes them back, read for the
/// system's and the extensions' functions that run SQL held in a string.
fn relations_sql() -> String {
    format!(
        "\
WITH {USER_ROUTINES},
depends AS (
    SELECT DISTINCT r.ev_class AS relation, d.refobjid AS depends_on
    FROM pg_rewrite r
    JOIN pg_depend d ON d.classid = 'pg_rewrite'::regclass AND d.objid = r.oid
        AND d.refclassid = 'pg_class'::regclass AND d.refobjid <> r.ev_class
    UNION
    SELECT inhrelid, inhparent FROM pg_inherits
),
calls AS (
    SELECT r.ev_class AS relation
    FROM pg_rewrite r
    JOIN pg_depend d ON d.classid = 'pg_rewrite'::regclass AND d.objid = r.oid
    WHERE d.refclassid = 'pg_proc'::regclass
        AND d.refobjid IN (SELECT oid FROM user_routines)
    UNION
    SELECT pol.polrelid
    FROM pg_policy pol
    JOIN pg_depend d ON d.classid = 'pg_policy'::regclass AND d.objid = pol.oid
    WHERE d.refclassid = 'pg_proc'::regclass
        AND d.refobjid IN (SELECT oid FROM user_routines)
)
SELECT n.nspname, c.relname, dn.nspname, dc.relname, c.oid IN (SELECT relation FROM calls),
    c.relispartition,
    CASE WHEN c.relkind IN ('v', 'm') THEN pg_get_viewdef(c.oid) ELSE '' END
        || coalesce((
            SELECT string_agg(pg_get_expr(pol.polqual, pol.polrelid), ' ')
            FROM pg_policy pol WHERE pol.polrelid = c.oid
        ), ''),
    c.relkind IN ('v', 'm')
FROM pg_class c
JOIN pg_namespace n ON n.oid = c.relnamespace
LEFT JOIN depends ON depends.relation = c.oid
LEFT JOIN pg_class dc ON dc.oid = depends.depends_on
LEFT JOIN pg_namespace dn ON dn.oid = dc.relnamespace
WHERE c.relkind IN ('r', 'v', 'm', 'p', 'f')
    AND n.nspname NOT IN ('pg_catalog', 'information_schema')
    AND n.nspname NOT LIKE 'pg\\_toast%'
    AND n.nspname NOT LIKE 'pg\\_temp%'
ORDER BY n.nspname, c.relname"
    )
}

/// The columns of the relations of the names in `$1`, folded to lower
/// case: the masked columns' tables.
const COLUMNS_SQL: &str = "\
SELECT n.nspname, c.relname, a.attname
FROM pg_attribute a
JOIN pg_class c ON c.oid = a.attrelid
JOIN pg_namespace n ON n.oid = c.relnamespace
WHERE a.attnum > 0 AND NOT a.attisdropped
    AND c.relkind IN ('r', 'v', 'm', 'p', 'f')
    AND lower(c.relname) = ANY($1)";

/// The columns of the names in `$1`, folded to lower case, that a view's or
/// a materialized view's rules read, as `pg_depend` keeps them: the view,
/// and the table and column it reads.
const COLUMN_READS_SQL: &str = "\
SELECT DISTINCT vn.nspname, v.relname, tn.nspname, t.relname, a.attname
FROM pg_rewrite r
JOIN pg_class v ON v.oid = r.ev_class
JOIN pg_namespace vn ON vn.oid = v.relnamespace
JOIN pg_depend d ON d.classid = 'pg_rewrite'::regclass AND d.objid = r.oid
    AND d.refclassid = 'pg_class'::regclass AND d.refobjid <> r.ev_class AND d.refobjsubid > 0
JOIN pg_class t ON t.oid = d.refobjid
JOIN pg_namespace tn ON tn.oid = t.relnamespace
JOIN pg_attribute a ON a.attrelid = t.oid AND a.attnum = d.refobjsubid
WHERE lower(a.attname) = ANY($1)";

/// The functions the users wrote, by schema and name.
fn routines_sql() -> String {
    format!("WITH {USER_ROUTINES} SELECT DISTINCT nspname, proname FROM user_routines")
}

/// The functions the users wrote that a statement can call without naming
/// them: an operator's (and its estimates'), a cast's, an operator class's,
/// a type's input, output and modifiers, a range type's, and one a domain's
/// constraint calls, which a cast to the domain runs.
fn unnamed_calls_sql() -> String {
    format!(
        "\
WITH {USER_ROUTINES}
SELECT DISTINCT u.proname
FROM user_routines u
WHERE EXISTS (SELECT 1 FROM pg_operator o WHERE u.oid IN (o.oprcode, o.oprrest, o.oprjoin))
    OR EXISTS (SELECT 1 FROM pg_cast c WHERE c.castfunc = u.oid)
    OR EXISTS (SELECT 1 FROM pg_amproc a WHERE a.amproc = u.oid)
    OR EXISTS (
        SELECT 1 FROM pg_type t
        WHERE u.oid IN (t.typinput, t.typoutput, t.typreceive, t.typsend, t.typmodin, t.typmodout)
    )
    OR EXISTS (SELECT 1 FROM pg_range r WHERE u.oid IN (r.rngcanonical, r.rngsubdiff))
    OR EXISTS (
        SELECT 1 FROM pg_constraint k
        JOIN pg_depend d ON d.classid = 'pg_constraint'::regclass AND d.objid = k.oid
        WHERE k.contypid <> 0 AND d.refclassid = 'pg_proc'::regclass AND d.refobjid = u.oid
    )
ORDER BY 1"
    )
}

/// Checks `read` against what `tables` hides, in the open transaction.
/// `Ok(Ok(masked))`: it may run, and `masked` says which of its result
/// columns are a masked column's (none where nothing is masked).
/// `Ok(Err(error))`: it may not, and `error` is why, a refusal or the
/// server's own error on the plan (the statement would have failed with
/// it). `Err` is a lost session or a cancel.
pub(super) async fn check(
    client: &tokio_postgres::Client,
    read: &Read,
    tables: &Tables,
    masked: &[ColumnName],
) -> Result<std::result::Result<Vec<bool>, Error>> {
    let catalog = catalog(client, masked).await?;
    let hidden = Hidden::masking(Dialect::Postgres, &catalog, tables, masked);
    if let Err(refusal) = hidden.check_statement(read) {
        return Ok(Err(Error::Agent(refusal)));
    }
    let explain = format!("EXPLAIN (VERBOSE, FORMAT JSON) {}", planned(read));
    let params: Vec<&(dyn tokio_postgres::types::ToSql + Sync)> = read
        .params()
        .iter()
        .map(|param| param as &(dyn tokio_postgres::types::ToSql + Sync))
        .collect();
    let rows = match client.query(explain.as_str(), &params).await {
        Ok(rows) => rows,
        Err(error) => return failed(error),
    };
    let plan: Option<super::script::Plan> = rows.first().and_then(|row| row.try_get(0).ok());
    let Some(plan) = plan else {
        return Ok(Err(Error::Agent(Refusal::NotParsed)));
    };
    let Ok(plan) = serde_json::from_str::<serde_json::Value>(&plan.0) else {
        return Ok(Err(Error::Agent(Refusal::NotParsed)));
    };
    if let Err(refusal) = planned_reads(&plan, &hidden) {
        return Ok(Err(Error::Agent(refusal)));
    }
    if let Err(refusal) = planned_masks(&plan, &hidden) {
        return Ok(Err(Error::Agent(refusal)));
    }
    match hidden.masks_anything() {
        true => masked_columns(client, read, &hidden).await,
        false => Ok(Ok(Vec::new())),
    }
}

/// The server's error on what the check asked of it: the statement's own,
/// which it would have failed with, or a lost session or a cancel.
fn failed<T>(error: tokio_postgres::Error) -> Result<std::result::Result<T, Error>> {
    let error = query_error(error);
    match error.is_connection_lost() || error == Error::Cancelled {
        true => Err(error),
        false => Ok(Err(error)),
    }
}

/// The table, the column and the schema of each column of the relations
/// whose oids are in `$1`.
const ORIGINS_SQL: &str = "\
SELECT c.oid, a.attnum, n.nspname, c.relname, a.attname
FROM pg_class c
JOIN pg_namespace n ON n.oid = c.relnamespace
JOIN pg_attribute a ON a.attrelid = c.oid
WHERE c.oid = ANY($1) AND a.attnum > 0";

/// Which result columns of `read` are a masked column's, by the table and
/// the column each comes from, as the server says of it prepared
/// (`Column::table_oid` and `column_id`): a column of a partition, of a
/// child table, and of a view an entry names too. A column the statement
/// computes comes from none.
async fn masked_columns(
    client: &tokio_postgres::Client,
    read: &Read,
    hidden: &Hidden,
) -> Result<std::result::Result<Vec<bool>, Error>> {
    let prepared = match client.prepare(read.text()).await {
        Ok(prepared) => prepared,
        Err(error) => return failed(error),
    };
    let origins: Vec<Option<(u32, i16)>> = prepared
        .columns()
        .iter()
        .map(|column| column.table_oid().zip(column.column_id()))
        .collect();
    let mut oids: Vec<u32> = origins.iter().flatten().map(|(oid, _)| *oid).collect();
    oids.sort_unstable();
    oids.dedup();
    if oids.is_empty() {
        return Ok(Ok(vec![false; origins.len()]));
    }
    let rows = match client.query(ORIGINS_SQL, &[&oids]).await {
        Ok(rows) => rows,
        Err(error) => return failed(error),
    };
    let named: std::collections::HashMap<(u32, i16), (String, String, String)> = rows
        .iter()
        .map(|row| {
            (
                (row.get(0), row.get(1)),
                (row.get(2), row.get(3), row.get(4)),
            )
        })
        .collect();
    Ok(Ok(origins
        .iter()
        .map(|origin| {
            origin
                .and_then(|origin| named.get(&origin))
                .is_some_and(|(schema, table, column)| {
                    hidden.masks(Some(schema), table, column).is_some()
                })
        })
        .collect()))
}

/// Checks where the plan uses a masked column: only passed on, as a name
/// by itself in a node's output, and never its table's whole row. The plan
/// writes a column with its scan's alias, or without one where the plan
/// reads a single relation; above a partitioned table's parts, with the
/// parent's alias, which no scan has (the parts' are it with `_1`, `_2`).
/// A subquery's or a common table expression's scan whose own plan passes
/// a masked column on is taken to pass it in every column it has: its
/// columns' names say nothing of where they come from.
fn planned_masks(plan: &serde_json::Value, hidden: &Hidden) -> std::result::Result<(), Refusal> {
    if !hidden.masks_anything() {
        return Ok(());
    }
    let mut scans: std::collections::HashMap<String, Vec<(String, String)>> = Default::default();
    relation_scans(plan, &mut scans);
    // A scan is kept under its alias and, where that ends in `_n`, under
    // what is before it: one relation, for all that.
    let relations: std::collections::BTreeSet<&(String, String)> =
        scans.values().flatten().collect();
    let single = match relations.into_iter().collect::<Vec<_>>().as_slice() {
        [only] => Some((*only).clone()),
        _ => None,
    };
    // The masked column a name of the plan is: by its scan's alias, or
    // the single relation's where it has none.
    let masked = |qualifier: Option<&str>, column: &str| {
        let relations: Vec<&(String, String)> = match qualifier {
            Some(qualifier) => scans
                .get(&qualifier.to_lowercase())
                .into_iter()
                .flatten()
                .collect(),
            None => single.iter().collect(),
        };
        relations
            .into_iter()
            .find_map(|(schema, table)| match column {
                "*" => hidden.masks_table(Some(schema), table),
                column => hidden.masks(Some(schema), table, column),
            })
    };
    // The first masked column a node passes on in its own output, which
    // is what a subquery's plan answers, if it passes one. A scan below
    // it may output every column of its table.
    let passes = |node: &serde_json::Value| {
        node.get("Output")
            .and_then(serde_json::Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(serde_json::Value::as_str)
            .find_map(|text| {
                let (names, bare) = crate::agent::column_references(Dialect::Postgres, text);
                bare.then(|| {
                    names
                        .iter()
                        .find_map(|(qualifier, column)| masked(qualifier.as_deref(), column))
                })
                .flatten()
            })
    };
    let mut carriers: std::collections::HashMap<String, String> = Default::default();
    carrier_scans(plan, plan, &passes, &mut carriers);
    let mut found = None;
    expressions(plan, &mut |key, text| {
        if found.is_some() {
            return;
        }
        let (names, bare) = crate::agent::column_references(Dialect::Postgres, text);
        let passing = key == "Output" && bare;
        for (qualifier, column) in &names {
            let entry = match masked(qualifier.as_deref(), column) {
                Some(entry) if !passing || column == "*" => Some(entry),
                Some(_) => None,
                None if !passing => qualifier
                    .as_ref()
                    .and_then(|qualifier| carriers.get(&qualifier.to_lowercase()).cloned()),
                None => None,
            };
            if entry.is_some() {
                found = entry;
                return;
            }
        }
    });
    match found {
        Some(column) => Err(Refusal::MaskedColumnUsed {
            column,
            how: crate::agent::MaskedUse::Planned,
        }),
        None => Ok(()),
    }
}

/// The relations a plan scans, by their scans' aliases folded to lower
/// case, with their schemas and names; a part's under its parent's alias
/// too, which is its own without its `_n`.
fn relation_scans(
    plan: &serde_json::Value,
    scans: &mut std::collections::HashMap<String, Vec<(String, String)>>,
) {
    match plan {
        serde_json::Value::Object(node) => {
            let text = |key: &str| node.get(key).and_then(serde_json::Value::as_str);
            if let (Some(alias), Some(relation), Some(schema)) =
                (text("Alias"), text("Relation Name"), text("Schema"))
            {
                let scanned = (schema.to_owned(), relation.to_owned());
                let alias = alias.to_lowercase();
                if let Some((parent, n)) = alias.rsplit_once('_')
                    && !n.is_empty()
                    && n.bytes().all(|byte| byte.is_ascii_digit())
                {
                    scans
                        .entry(parent.to_owned())
                        .or_default()
                        .push(scanned.clone());
                }
                scans.entry(alias).or_default().push(scanned);
            }
            node.values().for_each(|value| relation_scans(value, scans));
        }
        serde_json::Value::Array(values) => {
            values.iter().for_each(|value| relation_scans(value, scans));
        }
        _ => {}
    }
}

/// The aliases of the scans of a subquery or a common table expression
/// (of `plan`) whose own plan passes a masked column on, each with the one
/// it passes, as `passes` tells it of the plan's top node. A common table
/// expression's plan is found in `root` by its name.
fn carrier_scans(
    plan: &serde_json::Value,
    root: &serde_json::Value,
    passes: &impl Fn(&serde_json::Value) -> Option<String>,
    carriers: &mut std::collections::HashMap<String, String>,
) {
    match plan {
        serde_json::Value::Object(node) => {
            let text = |key: &str| node.get(key).and_then(serde_json::Value::as_str);
            let own = match text("Node Type") {
                Some("Subquery Scan") => node
                    .get("Plans")
                    .and_then(serde_json::Value::as_array)
                    .and_then(|plans| plans.first())
                    .and_then(passes),
                Some("CTE Scan" | "WorkTable Scan") => text("CTE Name")
                    .and_then(|name| subplan(root, &format!("CTE {name}")))
                    .and_then(passes),
                _ => None,
            };
            if let (Some(alias), Some(passed)) = (text("Alias"), own) {
                carriers.insert(alias.to_lowercase(), passed);
            }
            node.values()
                .for_each(|value| carrier_scans(value, root, passes, carriers));
        }
        serde_json::Value::Array(values) => values
            .iter()
            .for_each(|value| carrier_scans(value, root, passes, carriers)),
        _ => {}
    }
}

/// The node of `plan` whose subplan name is `name`.
fn subplan<'a>(plan: &'a serde_json::Value, name: &str) -> Option<&'a serde_json::Value> {
    match plan {
        serde_json::Value::Object(node) => {
            if node.get("Subplan Name").and_then(serde_json::Value::as_str) == Some(name) {
                return Some(plan);
            }
            node.values().find_map(|value| subplan(value, name))
        }
        serde_json::Value::Array(values) => values.iter().find_map(|value| subplan(value, name)),
        _ => None,
    }
}

/// The fields of a plan's node that hold names or words, and no
/// expression: a table named `city` with a masked column `city` is no use
/// of it by its scan's `Relation Name`. Any other field's text is read as
/// an expression, so a field a later server adds is read too.
const NAMED: &[&str] = &[
    "Node Type",
    "Relation Name",
    "Schema",
    "Alias",
    "Index Name",
    "Function Name",
    "CTE Name",
    "Subplan Name",
    "Parent Relationship",
    "Join Type",
    "Strategy",
    "Partial Mode",
    "Scan Direction",
    "Sort Method",
    "Sort Space Type",
    "Command",
    "Operation",
    "Conflict Resolution",
    "Conflict Arbiter Indexes",
];

/// Each expression the plan writes, with the key of the node's field it is
/// in: a field's text, or each of a list's (`Output`, `Sort Key`), at any
/// depth (a grouping set's `Group Keys` are lists in a list).
fn expressions(plan: &serde_json::Value, each: &mut impl FnMut(&str, &str)) {
    within(plan, None, each);
}

/// [`expressions`], of `value`, which is in the field `key` where it is in
/// one.
fn within(value: &serde_json::Value, key: Option<&str>, each: &mut impl FnMut(&str, &str)) {
    match value {
        serde_json::Value::Object(node) => {
            for (key, value) in node {
                if !NAMED.contains(&key.as_str()) {
                    within(value, Some(key), each);
                }
            }
        }
        serde_json::Value::Array(values) => {
            values.iter().for_each(|value| within(value, key, each));
        }
        serde_json::Value::String(text) => {
            if let Some(key) = key {
                each(key, text);
            }
        }
        _ => {}
    }
}

/// Checks every relation and every function a plan names.
fn planned_reads(plan: &serde_json::Value, hidden: &Hidden) -> std::result::Result<(), Refusal> {
    match plan {
        serde_json::Value::Object(node) => {
            let text = |key: &str| node.get(key).and_then(serde_json::Value::as_str);
            if let Some(name) = text("Relation Name") {
                hidden.check_engine_read(text("Schema"), name)?;
            }
            // A function in a FROM. One in an expression is in no node of
            // its own: the reading of the text finds it.
            if let Some(name) = text("Function Name") {
                // One that runs SQL held in a string, wherever the
                // statement reached it from: a view's body too.
                function_allowed(Dialect::Postgres, name)?;
                if !matches!(text("Schema"), Some("pg_catalog" | "information_schema")) {
                    hidden.check_function(name)?;
                }
            }
            node.values()
                .try_for_each(|value| planned_reads(value, hidden))
        }
        serde_json::Value::Array(values) => values
            .iter()
            .try_for_each(|value| planned_reads(value, hidden)),
        _ => Ok(()),
    }
}

/// The catalog as the open transaction sees it, with what it says of the
/// columns `masked` names.
pub(super) async fn catalog(
    client: &tokio_postgres::Client,
    masked: &[ColumnName],
) -> Result<Catalog> {
    let mut relations: Vec<Relation> = Vec::new();
    // The views' definitions, read for a whole row they read, which
    // `pg_depend` keeps no column of.
    let mut definitions: Vec<(String, String, String)> = Vec::new();
    for row in rows(client, &relations_sql()).await? {
        let (Some(schema), Some(name)) = (row.get(0), row.get(1)) else {
            continue;
        };
        let same = relations
            .last()
            .is_some_and(|last| last.schema == schema && last.name == name);
        if !same {
            relations.push(Relation {
                schema: schema.to_owned(),
                name: name.to_owned(),
                depends_on: Vec::new(),
                // `t` or `f`, as the simple protocol writes a boolean; what
                // is neither is taken for a call.
                opaque: row.get(4) != Some("f")
                    || calls_a_refused_function(Dialect::Postgres, row.get(6).unwrap_or("")),
                part: row.get(5) == Some("t"),
                view: row.get(7) == Some("t"),
            });
            if !masked.is_empty() && row.get(7) == Some("t") {
                let definition = row.get(6).unwrap_or("").to_owned();
                definitions.push((schema.to_owned(), name.to_owned(), definition));
            }
        }
        if let (Some(on_schema), Some(on_name), Some(last)) =
            (row.get(2), row.get(3), relations.last_mut())
        {
            last.depends_on
                .push((on_schema.to_owned(), on_name.to_owned()));
        }
    }
    let routines = rows(client, &routines_sql())
        .await?
        .iter()
        .filter_map(|row| Some((row.get(0)?.to_owned(), row.get(1)?.to_owned())))
        .collect();
    let unnamed_calls = rows(client, &unnamed_calls_sql())
        .await?
        .iter()
        .filter_map(|row| Some(row.get(0)?.to_owned()))
        .collect();
    let (columns, column_reads) = match masked.is_empty() {
        true => (Vec::new(), None),
        false => {
            let tables: Vec<String> = masked
                .iter()
                .map(|entry| entry.table.to_lowercase())
                .collect();
            let names: Vec<String> = masked
                .iter()
                .map(|entry| entry.column.to_lowercase())
                .collect();
            let columns = client
                .query(COLUMNS_SQL, &[&tables])
                .await
                .map_err(query_error)?
                .iter()
                .map(|row| (row.get(0), row.get(1), row.get(2)))
                .collect();
            let reads: Vec<ColumnRead> = client
                .query(COLUMN_READS_SQL, &[&names])
                .await
                .map_err(query_error)?
                .iter()
                .map(|row| ColumnRead {
                    schema: row.get(0),
                    view: row.get(1),
                    table_schema: row.get(2),
                    table: row.get(3),
                    column: row.get(4),
                })
                .collect();
            (columns, Some(reads))
        }
    };
    Ok(Catalog {
        relations,
        routines,
        unnamed_calls,
        columns,
        column_reads,
        definitions,
        ..Catalog::default()
    })
}

/// The rows `sql` answers, as text.
async fn rows(
    client: &tokio_postgres::Client,
    sql: &str,
) -> Result<Vec<tokio_postgres::SimpleQueryRow>> {
    let messages = client.simple_query(sql).await.map_err(query_error)?;
    Ok(messages
        .into_iter()
        .filter_map(|message| match message {
            SimpleQueryMessage::Row(row) => Some(row),
            _ => None,
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::TableName;

    /// A plan shaped as PostgreSQL 17's for `SELECT * FROM customers JOIN
    /// events ON …`: the parent's scan names its child table too, and a
    /// partitioned table is named by the partition it scans.
    fn plan() -> serde_json::Value {
        serde_json::json!([{ "Plan": {
            "Node Type": "Hash Join",
            "Plans": [
                { "Node Type": "Append", "Plans": [
                    { "Node Type": "Seq Scan", "Relation Name": "customers", "Schema": "public", "Alias": "customers_1" },
                    { "Node Type": "Index Only Scan", "Relation Name": "vip", "Schema": "public", "Alias": "customers_2", "Index Name": "vip_pkey" }
                ] },
                { "Node Type": "Hash", "Plans": [
                    { "Node Type": "Bitmap Heap Scan", "Relation Name": "events_2026", "Schema": "public", "Alias": "events",
                      "Plans": [{ "Node Type": "Bitmap Index Scan", "Index Name": "events_2026_at" }] }
                ] }
            ]
        } }])
    }

    fn hidden(tables: Tables) -> Hidden {
        let relation = |name: &str, on: &[&str]| Relation {
            schema: "public".into(),
            name: name.into(),
            depends_on: on
                .iter()
                .map(|on| ("public".into(), (*on).into()))
                .collect(),
            opaque: false,
            part: false,
            view: false,
        };
        let catalog = Catalog {
            relations: vec![
                relation("customers", &[]),
                relation("vip", &["customers"]),
                relation("events", &[]),
                relation("events_2026", &["events"]),
            ],
            routines: vec![("public".into(), "names".into())],
            ..Catalog::default()
        };
        Hidden::new(Dialect::Postgres, &catalog, &tables)
    }

    fn except(name: &str) -> Tables {
        Tables::AllExcept(vec![TableName {
            schema: None,
            name: name.into(),
        }])
    }

    #[test]
    fn every_scan_the_plan_names_is_checked() {
        assert_eq!(planned_reads(&plan(), &hidden(except("books"))), Ok(()));
        // A child table, by its scan under the parent's.
        assert_eq!(
            planned_reads(&plan(), &hidden(except("vip"))),
            Err(Refusal::TableHidden {
                name: "vip".into(),
                through: None
            })
        );
        // A partition of a hidden table.
        assert_eq!(
            planned_reads(&plan(), &hidden(except("events"))),
            Err(Refusal::TableHidden {
                name: "events".into(),
                through: Some("events_2026".into())
            })
        );
        // A function in a FROM, the users' own; and the system's catalog.
        let function = serde_json::json!({
            "Node Type": "Function Scan", "Function Name": "names", "Schema": "public"
        });
        assert_eq!(
            planned_reads(&function, &hidden(except("vip"))),
            Err(Refusal::FunctionNotAllowed("names".into()))
        );
        // One that runs SQL held in a string, an extension's, as a view
        // over it is planned.
        let crosstab = serde_json::json!({
            "Node Type": "Function Scan", "Function Name": "crosstab", "Schema": "public"
        });
        assert_eq!(
            planned_reads(&crosstab, &hidden(except("vip"))),
            Err(Refusal::FunctionNotAllowed("crosstab".into()))
        );
        let catalog = serde_json::json!({
            "Node Type": "Seq Scan", "Relation Name": "pg_class", "Schema": "pg_catalog"
        });
        assert_eq!(
            planned_reads(&catalog, &hidden(except("vip"))),
            Err(Refusal::SystemCatalog)
        );
    }

    /// The Bookshop's orders, with a partition, and the customers, with
    /// `orders.email` masked.
    fn masking() -> Hidden {
        let relation = |name: &str, on: &[&str]| Relation {
            schema: "public".into(),
            name: name.into(),
            depends_on: on
                .iter()
                .map(|on| ("public".into(), (*on).into()))
                .collect(),
            opaque: false,
            part: !on.is_empty(),
            view: false,
        };
        let catalog = Catalog {
            relations: vec![
                relation("orders", &[]),
                relation("orders_2026", &["orders"]),
                relation("customers", &[]),
            ],
            columns: vec![("public".into(), "orders".into(), "email".into())],
            ..Catalog::default()
        };
        let masked = ["orders.email".parse().unwrap()];
        Hidden::masking(Dialect::Postgres, &catalog, &Tables::All, &masked)
    }

    /// A scan of `relation` as `alias`, with these fields more.
    fn scan(relation: &str, alias: &str, more: serde_json::Value) -> serde_json::Value {
        let mut node = serde_json::json!({
            "Node Type": "Seq Scan", "Relation Name": relation, "Schema": "public", "Alias": alias
        });
        if let (Some(node), serde_json::Value::Object(more)) = (node.as_object_mut(), more) {
            node.extend(more);
        }
        node
    }

    #[test]
    fn a_plan_may_pass_a_masked_column_on_and_use_it_no_other_way() {
        let hidden = masking();
        let planned = |plan: serde_json::Value| planned_masks(&plan, &hidden);
        let refused = || {
            Err(Refusal::MaskedColumnUsed {
                column: "orders.email".into(),
                how: crate::agent::MaskedUse::Planned,
            })
        };
        let output = |items: &[&str]| serde_json::json!({ "Output": items });
        // Passed on, as a name by itself.
        assert_eq!(
            planned(scan("orders", "o", output(&["o.id", "o.email"]))),
            Ok(())
        );
        let sorted = serde_json::json!({
            "Node Type": "Sort", "Output": ["o.email", "o.id"], "Sort Key": ["o.id"],
            "Plans": [scan("orders", "o", output(&["o.email", "o.id"]))]
        });
        assert_eq!(planned(sorted), Ok(()));
        // Filtered, computed, sorted on, or its table's whole row.
        let filtered = scan(
            "orders",
            "o",
            serde_json::json!({ "Output": ["o.id"], "Filter": "(o.email ~~ 'a%'::text)" }),
        );
        assert_eq!(planned(filtered), refused());
        assert_eq!(
            planned(scan("orders", "o", output(&["lower(o.email)"]))),
            refused()
        );
        assert_eq!(planned(scan("orders", "o", output(&["o.*"]))), refused());
        let sort_key = serde_json::json!({
            "Node Type": "Sort", "Output": ["o.id"], "Sort Key": ["o.email"],
            "Plans": [scan("orders", "o", output(&["o.id", "o.email"]))]
        });
        assert_eq!(planned(sort_key), refused());
        // In a partition's scan, by its own alias.
        let partition = scan(
            "orders_2026",
            "orders_1",
            serde_json::json!({ "Output": ["orders_1.id"], "Index Cond": "(orders_1.email = 'x'::text)" }),
        );
        assert_eq!(planned(partition), refused());
        // Through a subquery's scan, which passes it under another name.
        let carried = serde_json::json!({
            "Node Type": "Subquery Scan", "Alias": "s", "Output": ["s.x"],
            "Filter": "(s.x ~~ 'a%'::text)",
            "Plans": [scan("orders", "o", output(&["o.email"]))]
        });
        assert_eq!(planned(carried), refused());
        let counted = serde_json::json!({
            "Node Type": "Hash Join", "Output": ["o.id", "o.email"],
            "Hash Cond": "(s.customer_id = o.customer_id)",
            "Plans": [
                scan("orders", "o", output(&["o.id", "o.email", "o.customer_id"])),
                { "Node Type": "Subquery Scan", "Alias": "s", "Output": ["s.customer_id"],
                  "Filter": "(s.n > 1)",
                  "Plans": [{ "Node Type": "HashAggregate",
                    "Output": ["orders_1.customer_id", "count(*)"],
                    "Plans": [scan("orders", "orders_1",
                        output(&["orders_1.id", "orders_1.customer_id", "orders_1.email"]))] }] }
            ]
        });
        assert_eq!(planned(counted), Ok(()));
        // A subquery or a common table expression that passes none on is
        // read as any scan, though a scan in it outputs one.
        let plain = serde_json::json!({
            "Node Type": "Hash Join", "Output": ["o.id", "o.email"],
            "Hash Cond": "(o.customer_id = c.id)",
            "Plans": [
                scan("orders", "o", output(&["o.id", "o.email", "o.customer_id"])),
                { "Node Type": "CTE Scan", "CTE Name": "c", "Alias": "c", "Output": ["c.id"] },
                { "Node Type": "Seq Scan", "Subplan Name": "CTE c", "Relation Name": "customers",
                  "Schema": "public", "Alias": "customers", "Output": ["customers.id"] }
            ]
        });
        assert_eq!(planned(plain), Ok(()));
        // A grouping set's keys, which are lists in a list.
        let rolled = serde_json::json!({
            "Node Type": "Aggregate", "Output": ["o.id"],
            "Grouping Sets": [{ "Group Keys": [["o.email"], []] }],
            "Plans": [scan("orders", "o", output(&["o.id", "o.email"]))]
        });
        assert_eq!(planned(rolled), refused());
        // A cast's type ends where the expression goes on.
        let case = scan(
            "orders",
            "orders",
            output(&["CASE WHEN (id > 1) THEN 'x'::text ELSE email END"]),
        );
        assert_eq!(planned(case), refused());
        // A plan of one relation writes its columns without the alias, its
        // name ending in `_n` or not.
        let lone = scan(
            "orders_2026",
            "orders_2026",
            serde_json::json!({ "Filter": "(email ~~ 'a%'::text)" }),
        );
        assert_eq!(planned(lone), refused());
        assert_eq!(
            planned(scan("orders", "orders", output(&["id", "email"]))),
            Ok(())
        );
        assert_eq!(
            planned(scan("orders", "orders", output(&["lower(email)"]))),
            refused()
        );
        // Above a partitioned table's parts, by the parent's alias.
        let parts = serde_json::json!({
            "Node Type": "Sort", "Output": ["o.id"], "Sort Key": ["o.email"],
            "Plans": [{ "Node Type": "Append", "Plans": [
                scan("orders_2026", "o_1", output(&["o_1.id", "o_1.email"])),
                scan("orders_2026", "o_2", output(&["o_2.id", "o_2.email"]))
            ] }]
        });
        assert_eq!(planned(parts), refused());
        // A cast's type is no column, though a masked one has its name.
        let events = || {
            let catalog = Catalog {
                relations: vec![Relation {
                    schema: "public".into(),
                    name: "events".into(),
                    depends_on: Vec::new(),
                    opaque: false,
                    part: false,
                    view: false,
                }],
                columns: vec![("public".into(), "events".into(), "date".into())],
                ..Catalog::default()
            };
            let masked = ["events.date".parse().unwrap()];
            Hidden::masking(Dialect::Postgres, &catalog, &Tables::All, &masked)
        };
        for filter in [
            "(created_at > '2026-01-01'::date)",
            "(created_at > '2026-01-01 00:00:00'::timestamp without time zone)",
        ] {
            let cast = scan("events", "events", serde_json::json!({ "Filter": filter }));
            assert_eq!(planned_masks(&cast, &events()), Ok(()), "{filter}");
        }
        let used = scan(
            "events",
            "events",
            serde_json::json!({ "Filter": "(date > now())" }),
        );
        assert!(planned_masks(&used, &events()).is_err());
        // A table with its column's name is no use of the column.
        let city = || {
            let catalog = Catalog {
                relations: vec![Relation {
                    schema: "public".into(),
                    name: "city".into(),
                    depends_on: Vec::new(),
                    opaque: false,
                    part: false,
                    view: false,
                }],
                columns: vec![("public".into(), "city".into(), "city".into())],
                ..Catalog::default()
            };
            let masked = ["city.city".parse().unwrap()];
            Hidden::masking(Dialect::Postgres, &catalog, &Tables::All, &masked)
        };
        let ids = scan("city", "city", output(&["city_id"]));
        assert_eq!(planned_masks(&ids, &city()), Ok(()));
        let filtered = scan(
            "city",
            "city",
            serde_json::json!({ "Filter": "(city ~~ 'a%'::text)" }),
        );
        assert_eq!(
            planned_masks(&filtered, &city()),
            Err(Refusal::MaskedColumnUsed {
                column: "city.city".into(),
                how: crate::agent::MaskedUse::Planned,
            })
        );
        // Another table's column of the same name is its own.
        let customers = scan(
            "customers",
            "c",
            serde_json::json!({ "Output": ["c.id"], "Filter": "(c.email = 'x'::text)" }),
        );
        assert_eq!(planned(customers), Ok(()));
    }
}

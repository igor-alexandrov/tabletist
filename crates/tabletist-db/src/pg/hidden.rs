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
    Catalog, Hidden, Read, Refusal, Relation, Tables, calls_a_refused_function, function_allowed,
    planned,
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
/// `Ok(None)`: it may run. `Ok(Some(error))`: it may not, and `error` is
/// why, a refusal or the server's own error on the plan (the statement
/// would have failed with it). `Err` is a lost session or a cancel.
pub(super) async fn check(
    client: &tokio_postgres::Client,
    read: &Read,
    tables: &Tables,
) -> Result<Option<Error>> {
    let catalog = catalog(client).await?;
    let hidden = Hidden::new(Dialect::Postgres, &catalog, tables);
    if let Err(refusal) = hidden.check_statement(read) {
        return Ok(Some(Error::Agent(refusal)));
    }
    let explain = format!("EXPLAIN (VERBOSE, FORMAT JSON) {}", planned(read));
    let params: Vec<&(dyn tokio_postgres::types::ToSql + Sync)> = read
        .params()
        .iter()
        .map(|param| param as &(dyn tokio_postgres::types::ToSql + Sync))
        .collect();
    let rows = match client.query(explain.as_str(), &params).await {
        Ok(rows) => rows,
        Err(error) => {
            let error = query_error(error);
            if error.is_connection_lost() || error == Error::Cancelled {
                return Err(error);
            }
            return Ok(Some(error));
        }
    };
    let plan: Option<super::script::Plan> = rows.first().and_then(|row| row.try_get(0).ok());
    let Some(plan) = plan else {
        return Ok(Some(Error::Agent(Refusal::NotParsed)));
    };
    let Ok(plan) = serde_json::from_str::<serde_json::Value>(&plan.0) else {
        return Ok(Some(Error::Agent(Refusal::NotParsed)));
    };
    Ok(planned_reads(&plan, &hidden).err().map(Error::Agent))
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

/// The catalog as the open transaction sees it.
pub(super) async fn catalog(client: &tokio_postgres::Client) -> Result<Catalog> {
    let mut relations: Vec<Relation> = Vec::new();
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
    Ok(Catalog {
        relations,
        routines,
        unnamed_calls,
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
}

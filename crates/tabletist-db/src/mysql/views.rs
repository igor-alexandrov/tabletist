//! What the views an agent's statement reaches call, on MySQL and
//! MariaDB, on every connection ([`crate::agent::views`]). Neither server
//! lists the built-in functions a view calls (`VIEW_ROUTINE_USAGE` names
//! only the users'), so each view's body is read as the server writes it
//! back (`information_schema.VIEWS`).
//!
//! It costs no round trip of its own where no view calls what an agent
//! may not and none is run with another user's rights: the open's one
//! message asks for the views whose body holds one of the refused
//! functions' names, that the server shows no body of, or that it runs
//! with their definer's rights where the definer is another user
//! ([`candidates`]). Only where one of those is found does the check read
//! every view's body, with the tables and views the user is shown, in one
//! more round trip, to know which views reach what an agent may not call
//! and which read what the user is not shown.

use std::collections::HashSet;

use mysql_async::prelude::Queryable;

use super::{from_row, query_error};
use crate::agent::views::{self, View};
use crate::{Dialect, Error, Result};

/// The run sets `sql_select_limit` for the agent's rows; a `LIMIT` of a
/// query's own takes its place, so the views are read whole.
const WHOLE: &str = "LIMIT 18446744073709551615";

/// A view as [`candidates`] lists it, and whether the server runs it with
/// another user's rights: its definer's, where that is not the
/// connection's user. Such a view may read what the user is not shown.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Candidate {
    view: View,
    others_rights: bool,
}

/// Whether the server runs a view with another user's rights, as 1 or 0.
const OTHERS_RIGHTS: &str = "IF(SECURITY_TYPE = 'DEFINER' AND DEFINER <> CURRENT_USER(), 1, 0)";

/// The views whose body may call a function an agent may not call: one
/// that holds a refused function's name anywhere, in any letters, or that
/// the server shows to no one but its owner (an empty body), and those it
/// runs with another user's rights. A superset of what [`views::any_refused`]
/// finds. Every schema's but the server's own ([`views::MYSQL_OWN_SCHEMAS`]).
pub(super) fn candidates() -> String {
    let holds: Vec<String> = views::refused_words(Dialect::MySql)
        .map(|word| format!("LOWER(VIEW_DEFINITION) LIKE '%{word}%'"))
        .collect();
    format!(
        "SELECT TABLE_SCHEMA, TABLE_NAME, COALESCE(VIEW_DEFINITION, ''), {OTHERS_RIGHTS} \
         FROM information_schema.VIEWS \
         WHERE {} AND (COALESCE(VIEW_DEFINITION, '') = '' OR {OTHERS_RIGHTS} = 1 OR {}) {WHOLE}",
        users(),
        holds.join(" OR ")
    )
}

/// Every view's body but the server's own, and after them every table and
/// view the user is shown, the server's own too, with no body: one query,
/// one round trip. The first column tells which.
fn whole() -> String {
    format!(
        "SELECT 1, TABLE_SCHEMA, TABLE_NAME, COALESCE(VIEW_DEFINITION, ''), {OTHERS_RIGHTS} \
         FROM information_schema.VIEWS WHERE {} \
         UNION ALL SELECT 0, TABLE_SCHEMA, TABLE_NAME, '', 0 FROM information_schema.TABLES \
         {WHOLE}",
        users()
    )
}

/// The views of the users' schemas, not the server's own.
fn users() -> String {
    let own: Vec<String> = views::MYSQL_OWN_SCHEMAS
        .iter()
        .map(|schema| format!("'{schema}'"))
        .collect();
    format!("LOWER(TABLE_SCHEMA) NOT IN ({})", own.join(", "))
}

/// The views [`candidates`] answered.
pub(super) fn listed(rows: Vec<mysql_async::Row>) -> Result<Vec<Candidate>> {
    rows.into_iter()
        .map(|row| {
            let (schema, name, definition, others_rights): (String, String, String, i64) =
                from_row(row)?;
            Ok(Candidate {
                view: View {
                    schema,
                    name,
                    definition,
                },
                others_rights: others_rights != 0,
            })
        })
        .collect()
}

/// What [`whole`] answered: every view, those run with another user's
/// rights, and the folded names of what the user is shown.
struct Whole {
    views: Vec<View>,
    others: HashSet<(String, String)>,
    shown: HashSet<(String, String)>,
}

fn whole_listed(rows: Vec<mysql_async::Row>) -> Result<Whole> {
    let mut whole = Whole {
        views: Vec::new(),
        others: HashSet::new(),
        shown: HashSet::new(),
    };
    for row in rows {
        let (view, schema, name, definition, others_rights): (i64, String, String, String, i64) =
            from_row(row)?;
        let folded = (schema.to_lowercase(), name.to_lowercase());
        if view == 0 {
            whole.shown.insert(folded);
            continue;
        }
        if others_rights != 0 {
            whole.others.insert(folded);
        }
        whole.views.push(View {
            schema,
            name,
            definition,
        });
    }
    Ok(whole)
}

/// Checks `texts`, an agent's statement, against the views it reaches, in
/// the open transaction. `candidates` are what [`candidates`] answered in
/// the open. `Ok(Err(error))`: it may not run, and `error` is why, a
/// refusal, a cancel or the server's own error. `Err` is a lost session.
pub(super) async fn check(
    conn: &mut mysql_async::Conn,
    texts: &[String],
    candidates: &[Candidate],
) -> Result<std::result::Result<(), Error>> {
    let found: Vec<View> = candidates
        .iter()
        .map(|candidate| candidate.view.clone())
        .collect();
    let others = candidates.iter().any(|candidate| candidate.others_rights);
    if !others && !views::any_refused(Dialect::MySql, &found) {
        return Ok(Ok(()));
    }
    let rows: Vec<mysql_async::Row> = match conn.query(whole()).await {
        Ok(rows) => rows,
        Err(error) => {
            let error = query_error(error);
            return match error.is_connection_lost() {
                true => Err(error),
                false => Ok(Err(error)),
            };
        }
    };
    let whole = match whole_listed(rows) {
        Ok(whole) => whole,
        Err(error) => return Ok(Err(error)),
    };
    let unseen = |view: &View| {
        let folded = (view.schema.to_lowercase(), view.name.to_lowercase());
        match whole.others.contains(&folded) {
            true => views::reads_unseen(Dialect::MySql, view, &whole.shown),
            false => None,
        }
    };
    Ok(texts
        .iter()
        .try_for_each(|text| views::check(Dialect::MySql, text, &whole.views, &unseen))
        .map_err(Error::Agent))
}

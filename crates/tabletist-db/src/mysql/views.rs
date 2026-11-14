//! What the views an agent's statement reaches call, on MySQL and
//! MariaDB, on every connection ([`crate::agent::views`]). Neither server
//! lists the built-in functions a view calls (`VIEW_ROUTINE_USAGE` names
//! only the users'), so each view's body is read as the server writes it
//! back (`information_schema.VIEWS`).
//!
//! It costs no round trip of its own where no view calls what an agent
//! may not: the open's one message asks for the views whose body holds
//! one of the refused functions' names, or that the server shows no body
//! of ([`candidates`]), which on most databases is none. Only where one of
//! those is found to call one does the check read every view's body, in
//! one more round trip, to know which views reach it.

use mysql_async::prelude::Queryable;

use super::{from_row, query_error};
use crate::agent::views::{self, View};
use crate::{Dialect, Error, Result};

/// The run sets `sql_select_limit` for the agent's rows; a `LIMIT` of a
/// query's own takes its place, so the views are read whole.
const WHOLE: &str = "LIMIT 18446744073709551615";

/// The views whose body may call a function an agent may not call: one
/// that holds a refused function's name anywhere, in any letters, or that
/// the server shows to no one but its owner (an empty body). A superset of
/// what [`views::any_refused`] finds. Every schema's but the server's own
/// ([`views::MYSQL_OWN_SCHEMAS`]).
pub(super) fn candidates() -> String {
    let holds: Vec<String> = views::refused_words(Dialect::MySql)
        .map(|word| format!("LOWER(VIEW_DEFINITION) LIKE '%{word}%'"))
        .collect();
    format!(
        "SELECT TABLE_SCHEMA, TABLE_NAME, COALESCE(VIEW_DEFINITION, '') \
         FROM information_schema.VIEWS \
         WHERE {} AND (COALESCE(VIEW_DEFINITION, '') = '' OR {}) {WHOLE}",
        users(),
        holds.join(" OR ")
    )
}

/// Every view's body but the server's own.
fn every_view() -> String {
    format!(
        "SELECT TABLE_SCHEMA, TABLE_NAME, COALESCE(VIEW_DEFINITION, '') \
         FROM information_schema.VIEWS WHERE {} {WHOLE}",
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

/// The views [`candidates`] or [`every_view`] answered.
pub(super) fn listed(rows: Vec<mysql_async::Row>) -> Result<Vec<View>> {
    rows.into_iter()
        .map(|row| {
            let (schema, name, definition) = from_row(row)?;
            Ok(View {
                schema,
                name,
                definition,
            })
        })
        .collect()
}

/// Checks `texts`, an agent's statement, against the views it reaches, in
/// the open transaction. `candidates` are what [`candidates`] answered in
/// the open. `Ok(Err(error))`: it may not run, and `error` is why, a
/// refusal, a cancel or the server's own error. `Err` is a lost session.
pub(super) async fn check(
    conn: &mut mysql_async::Conn,
    texts: &[String],
    candidates: &[View],
) -> Result<std::result::Result<(), Error>> {
    if !views::any_refused(Dialect::MySql, candidates) {
        return Ok(Ok(()));
    }
    let every: Vec<mysql_async::Row> = match conn.query(every_view()).await {
        Ok(rows) => rows,
        Err(error) => {
            let error = query_error(error);
            return match error.is_connection_lost() {
                true => Err(error),
                false => Ok(Err(error)),
            };
        }
    };
    let every = match listed(every) {
        Ok(every) => every,
        Err(error) => return Ok(Err(error)),
    };
    Ok(texts
        .iter()
        .try_for_each(|text| views::check(Dialect::MySql, text, &every))
        .map_err(Error::Agent))
}

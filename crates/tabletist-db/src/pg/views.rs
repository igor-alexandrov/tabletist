//! What the views an agent's statement reaches call, on PostgreSQL, on
//! every connection ([`crate::agent::views`]). PostgreSQL keeps no
//! dependency on its own functions, the ones an agent may not call among
//! them (`pg_sleep`, `pg_advisory_lock`, `set_config`), so what a view
//! calls is read from its rule.
//!
//! It costs no round trip of its own: the open's one message asks for it
//! ([`reaching`]) in the place of the query that takes the transaction's
//! snapshot, and the answer holds all the check needs. The rule of each
//! view is read as PostgreSQL stores it, the parsed tree in text
//! (`pg_rewrite.ev_action`), for a call of one of the refused functions by
//! its number in `pg_proc`: nothing is decompiled for that. The views that
//! reach one of those, at any depth, are found by `pg_depend`, which keeps
//! what relations a view's rule reads. Only those views, on most databases
//! none, are written back as SQL (`pg_get_viewdef`), and their text is what
//! [`views::check`] reads, as it reads MySQL's.
//!
//! The system's own views in `pg_catalog` are not read: PostgreSQL wrote
//! them, none of them calls such a function, and no one can change them
//! but a superuser who turned on `allow_system_table_mods`. Those of
//! `information_schema` are read, as a superuser may replace one.

use tokio_postgres::SimpleQueryMessage;

use crate::Dialect;
use crate::agent::views::{self, View};

/// The views that may call a function an agent may not call, at any depth,
/// with their definitions as the server writes them back: every view whose
/// rule calls a function of that name, or of a name that starts so, in any
/// schema, and every view whose rule reads one of those. More than
/// [`views::check`] refuses: a call of a function whose name only starts
/// as a refused one's does is found too, and the definition tells.
///
/// The functions are found by their names as `pg_proc` holds them, each a
/// range of its index: the system's and the extensions' are lower-case. A
/// function the users wrote and named in capitals is not, which an agent
/// may call on such a connection as any other they wrote (B.2).
///
/// A call stands in the rule as `:funcid`, `:aggfnoid` or `:winfnoid`
/// followed by the function's number; nothing that is written back as a
/// call by its name stands otherwise.
pub(super) fn reaching() -> String {
    let starts: Vec<String> = views::refused_words(Dialect::Postgres)
        .map(|word| format!("(p.proname >= '{word}' AND p.proname < '{}')", past(word)))
        .collect();
    format!(
        "\
WITH RECURSIVE refused AS (
    SELECT p.oid FROM pg_catalog.pg_proc p WHERE {starts}
),
pattern AS (
    SELECT ':(funcid|aggfnoid|winfnoid) (' || string_agg(oid::text, '|') || ')([^0-9]|$)'
        AS calls
    FROM refused
),
reaching AS (
    SELECT r.ev_class AS view
    FROM pg_catalog.pg_rewrite r
    JOIN pg_catalog.pg_class c ON c.oid = r.ev_class
    CROSS JOIN pattern
    -- The rule is read only once the view is known to be one to read:
    -- a CASE is evaluated in its order.
    WHERE r.ev_type = '1' AND CASE
        WHEN c.relkind = 'v' AND c.relnamespace <> 'pg_catalog'::regnamespace
            THEN r.ev_action::text ~ pattern.calls
    END
    UNION
    SELECT r.ev_class
    FROM reaching
    JOIN pg_catalog.pg_depend d ON d.refclassid = 'pg_catalog.pg_class'::regclass AND d.refobjid = reaching.view
        AND d.classid = 'pg_catalog.pg_rewrite'::regclass
    JOIN pg_catalog.pg_rewrite r ON r.oid = d.objid AND r.ev_type = '1' AND r.ev_class <> reaching.view
    JOIN pg_catalog.pg_class c ON c.oid = r.ev_class AND c.relkind = 'v'
)
SELECT n.nspname, c.relname, coalesce(pg_catalog.pg_get_viewdef(c.oid), '')
FROM reaching
JOIN pg_catalog.pg_class c ON c.oid = reaching.view
JOIN pg_catalog.pg_namespace n ON n.oid = c.relnamespace",
        starts = starts.join(" OR ")
    )
}

/// The first name past every name that starts with `word`, as `name`
/// sorts (byte by byte): its last letter, the next one.
fn past(word: &str) -> String {
    let mut past = word.to_owned();
    let last = past.pop().map_or(0, |last| last as u8);
    past.push(char::from(last + 1));
    past
}

/// The views [`reaching`] answered, among the open's other answers: its
/// rows are the only rows the open has.
pub(super) fn listed(messages: &[SimpleQueryMessage]) -> Vec<View> {
    messages
        .iter()
        .filter_map(|message| match message {
            SimpleQueryMessage::Row(row) => {
                let text = |index: usize| row.try_get(index).ok().flatten().unwrap_or_default();
                Some(View {
                    schema: text(0).to_owned(),
                    name: text(1).to_owned(),
                    definition: text(2).to_owned(),
                })
            }
            _ => None,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::Refusal;

    /// Definitions as PostgreSQL 17 writes them back.
    #[test]
    fn a_view_over_one_that_takes_a_lock_is_refused() {
        let view = |name: &str, definition: &str| View {
            schema: "public".into(),
            name: name.into(),
            definition: definition.into(),
        };
        let views = [
            view(
                "lock_takers",
                " SELECT id,\n    (pg_advisory_lock((id)::bigint) IS NULL) AS took\n   FROM books;",
            ),
            view("over_lock_takers", " SELECT id\n   FROM lock_takers;"),
        ];
        let calls = Err(Refusal::FunctionNotAllowed("pg_advisory_lock".into()));
        let check = |text: &str| views::check(Dialect::Postgres, text, &views, &|_| None);
        assert_eq!(check("SELECT count(*) FROM public.over_lock_takers"), calls);
        assert_eq!(check("SELECT count(*) FROM \"lock_takers\""), calls);
        assert_eq!(check("SELECT count(*) FROM books"), Ok(()));
        // A double quote in a name is written twice, by the agent and by
        // the server in a definition that names the view.
        let views = [
            view(
                "lock\"takers",
                " SELECT (pg_advisory_lock((1)::bigint) IS NULL) AS took;",
            ),
            view("over\"lock", " SELECT took\n   FROM \"lock\"\"takers\";"),
        ];
        let check = |text: &str| views::check(Dialect::Postgres, text, &views, &|_| None);
        assert_eq!(check("SELECT * FROM \"lock\"\"takers\""), calls);
        assert_eq!(check("SELECT * FROM public.\"over\"\"lock\""), calls);
    }

    /// The words stand in a string as they are, and each is the start of
    /// a range that ends before the next letter.
    #[test]
    fn every_refused_word_is_asked_for_as_the_start_of_a_name() {
        let sql = reaching();
        for word in views::refused_words(Dialect::Postgres) {
            let plain = |character: char| character.is_ascii_lowercase() || character == '_';
            assert!(word.chars().all(plain), "{word}");
            assert!(
                sql.contains(&format!("p.proname >= '{word}' AND")),
                "{word}"
            );
        }
        assert_eq!(past("pg_sleep"), "pg_sleeq");
        assert_eq!(past("pg_advisory_"), "pg_advisory`");
        assert!(*"pg_advisory_lock" < *past("pg_advisory_"));
    }
}

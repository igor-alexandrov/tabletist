//! What the views an agent's statement reaches call (B.2), on every
//! connection, the ones that hide nothing too. The statement names only
//! the view, and the server runs what the view's body calls: `load_file`,
//! `get_lock`, `sleep`. So a statement that names a view whose body calls
//! a function an agent may not call, at any depth, is refused as if it
//! called it itself.
//!
//! A view is reached only by its name: an agent's statement holds no
//! executable comment, and the server's own writing of a view's body none
//! either. Names are matched as words of the text, in any schema and in
//! any letters, wherever they stand, so a column or a string that bears a
//! refused view's name is refused with it. That errs toward refusing, and
//! only ever where a view calls what it may not.
//!
//! A view the user holds no privilege on is listed to it nowhere, and a
//! table it may not read neither: the two cannot be told apart. A view the
//! server runs with another user's rights (`SQL SECURITY DEFINER`) may read
//! either, and what such a view calls is not seen. So a view run with
//! another's rights whose body reads what the user is not shown is refused
//! as one whose definition it may not see ([`reads_unseen`]), a view that
//! grants a user its rows from tables it may not read too.

use std::collections::HashSet;

use super::Refusal;
use super::hidden::{qualified_relations, refused_call};
use crate::Dialect;

/// A view as the catalog gives it: its definition as the server writes it
/// back, empty where the server shows it to no one but its owner (a user
/// without `SHOW VIEW`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct View {
    pub(crate) schema: String,
    pub(crate) name: String,
    pub(crate) definition: String,
}

/// Why a view may not be read by an agent: the function its body calls,
/// at some depth, or that what it calls cannot be seen, its body or what
/// it reads with another user's rights.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Why {
    Calls(String),
    Unseen(String),
    ReadsUnseen { view: String, relation: String },
}

impl Why {
    fn refusal(self) -> Refusal {
        match self {
            Self::Calls(function) => Refusal::FunctionNotAllowed(function),
            Self::Unseen(view) => Refusal::NotAllowed(format!(
                "The view {view}, whose definition this connection's user may not see,"
            )),
            Self::ReadsUnseen { view, relation } => Refusal::NotAllowed(format!(
                "The view {view}, which reads {relation} that this connection's user \
                 may not see,"
            )),
        }
    }
}

/// The schemas MySQL and MariaDB make and keep for themselves. Their views
/// come with the server and call nothing an agent may not; a user who may
/// read every table but not see a view's body (`GRANT SELECT ON *.*`) is
/// shown each of them, `sys`'s `session` and `metrics` among them, with
/// none.
pub(crate) const MYSQL_OWN_SCHEMAS: [&str; 4] =
    ["information_schema", "mysql", "performance_schema", "sys"];

/// Whether `view` is one the server made for itself: in one of its
/// schemas, by the name's exact letters. A user's `SYS` is another schema
/// where names are told by their case (`lower_case_table_names = 0`).
fn servers_own(dialect: Dialect, view: &View) -> bool {
    let own: &[&str] = match dialect {
        Dialect::MySql => &MYSQL_OWN_SCHEMAS,
        Dialect::Postgres | Dialect::Sqlite | Dialect::BigQuery => &[],
    };
    own.contains(&view.schema.as_str())
}

/// The longest name the engine keeps, in bytes, where it cuts a longer one
/// to it rather than refuse it: PostgreSQL's 63 (`NAMEDATALEN` less one),
/// cut where a character ends. MySQL refuses a name over its 64
/// characters.
fn cuts_names_at(dialect: Dialect) -> Option<usize> {
    match dialect {
        Dialect::Postgres => Some(63),
        Dialect::MySql | Dialect::Sqlite | Dialect::BigQuery => None,
    }
}

/// What a view of `views` reads that the user is not shown, by name: see
/// [`reads_unseen`]. None where every view is run with the reader's own
/// rights.
pub(crate) type Unseen<'a> = &'a dyn Fn(&View) -> Option<String>;

/// Why `view` itself may not be read, by its own body alone and what it
/// reads that the user is not shown.
fn by_itself(dialect: Dialect, view: &View, unseen: Unseen<'_>) -> Option<Why> {
    if servers_own(dialect, view) {
        return None;
    }
    let named = || format!("{}.{}", view.schema, view.name);
    if view.definition.trim().is_empty() {
        return Some(Why::Unseen(named()));
    }
    if let Some(function) = refused_call(dialect, &view.definition) {
        return Some(Why::Calls(function));
    }
    unseen(view).map(|relation| Why::ReadsUnseen {
        view: named(),
        relation,
    })
}

/// The relations `view`'s body reads by a name with its schema, as written:
/// its tables and views.
pub(crate) fn relations(dialect: Dialect, view: &View) -> Vec<(String, String)> {
    qualified_relations(dialect, &view.definition)
}

/// The first relation `view`'s body reads that the user is not shown, as
/// `schema.name`, for a view the server runs with another user's rights:
/// `shown` are the tables and views the user is shown, by their schema and
/// name in the server's own letters. Matched by those exact letters: where
/// the server tells names by their case, `PEEK` shown says nothing of a
/// `peek`; where it does not, it writes a body's names as it keeps them,
/// and a name written otherwise is refused, which errs toward refusing.
/// What it reads may be a view that calls what an agent may not, which the
/// user is shown nothing of.
pub(crate) fn reads_unseen(
    dialect: Dialect,
    view: &View,
    shown: &HashSet<(String, String)>,
) -> Option<String> {
    qualified_relations(dialect, &view.definition)
        .into_iter()
        .find(|relation| !shown.contains(relation))
        .map(|(schema, name)| format!("{schema}.{name}"))
}

/// The words, lower-cased, of which a body that calls a function an agent
/// may not call holds one: each such function's name, or the start of
/// one. What a server can look for in its views' bodies before it sends
/// any, finding every view [`any_refused`] would and some more.
pub(crate) fn refused_words(dialect: Dialect) -> impl Iterator<Item = &'static str> {
    let (names, prefixes) = super::effects(dialect);
    names.iter().chain(prefixes).copied()
}

/// Whether any of `views` may not be read by its own body alone. Where
/// none may not, no view reaches one, and the others need not be read.
pub(crate) fn any_refused(dialect: Dialect, views: &[View]) -> bool {
    views
        .iter()
        .any(|view| by_itself(dialect, view, &|_| None).is_some())
}

/// Refuses `text`, an agent's statement, where it names a view of `views`
/// (all the database's) whose body calls a function an agent may not
/// call, or reads what the user is not shown (`unseen`), or names such a
/// view, at any depth.
pub(crate) fn check(
    dialect: Dialect,
    text: &str,
    views: &[View],
    unseen: Unseen<'_>,
) -> Result<(), Refusal> {
    // The refused views, by their folded names, and why each is.
    let mut refused: Vec<(String, Why)> = views
        .iter()
        .filter_map(|view| Some((view.name.to_lowercase(), by_itself(dialect, view, unseen)?)))
        .collect();
    if refused.is_empty() {
        return Ok(());
    }
    let bodies: Vec<(String, String)> = views
        .iter()
        .map(|view| (view.name.to_lowercase(), view.definition.to_lowercase()))
        .collect();
    // A view whose body names a refused view is refused for the same
    // reason, until no more are.
    loop {
        let more: Vec<(String, Why)> = bodies
            .iter()
            .filter(|(name, _)| !refused.iter().any(|(known, _)| known == name))
            .filter_map(|(name, body)| {
                let (_, why) = refused
                    .iter()
                    .find(|(known, _)| names(dialect, body, known))?;
                Some((name.clone(), why.clone()))
            })
            .collect();
        if more.is_empty() {
            break;
        }
        refused.extend(more);
    }
    let text = text.to_lowercase();
    match refused
        .into_iter()
        .find(|(name, _)| names(dialect, &text, name))
    {
        Some((_, why)) => Err(why.refusal()),
        None => Ok(()),
    }
}

/// Whether `text` holds `name` as a word: not inside a longer name. Both
/// folded. A name that begins or ends with what no unquoted name holds
/// (`` `a b` ``) is found wherever it stands at that end, and so is one as
/// long as the engine cuts a longer name to: a longer word that begins
/// with it names it too ([`cuts_names_at`]). A character is at most four
/// bytes, so a cut name is at most three short of the longest.
fn names(dialect: Dialect, text: &str, name: &str) -> bool {
    if name.is_empty() {
        return false;
    }
    let word = |character: char| character.is_alphanumeric() || matches!(character, '_' | '$');
    let cut = cuts_names_at(dialect).is_some_and(|longest| name.len() + 3 >= longest);
    let starts = name.starts_with(word);
    let ends = name.ends_with(word) && !cut;
    // A quote in a name is written twice inside quotes of its kind: a
    // backquote in MySQL's, a double quote in PostgreSQL's (and MySQL's
    // under ANSI_QUOTES). Each spelling is looked for in any dialect.
    let mut spellings = vec![name.to_owned()];
    for quote in ['`', '"'] {
        if name.contains(quote) {
            spellings.push(name.replace(quote, &format!("{quote}{quote}")));
        }
    }
    spellings.iter().any(|spelling| {
        text.match_indices(spelling).any(|(at, _)| {
            let before = text[..at].chars().next_back();
            let after = text[at + spelling.len()..].chars().next();
            !(starts && before.is_some_and(word)) && !(ends && after.is_some_and(word))
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn view(name: &str, definition: &str) -> View {
        View {
            schema: "bookshop".into(),
            name: name.into(),
            definition: definition.into(),
        }
    }

    /// Definitions as MySQL 8.4 and MariaDB 11.4 write them back.
    fn views() -> Vec<View> {
        vec![
            view(
                "titles",
                "select `bookshop`.`books`.`title` AS `title` from `bookshop`.`books`",
            ),
            view("leaked", "select load_file('/etc/hostname') AS `contents`"),
            view(
                "over_leaked",
                "select `bookshop`.`leaked`.`contents` AS `contents` from `bookshop`.`leaked`",
            ),
            view(
                "over_over_leaked",
                "select `o`.`contents` AS `contents` from `bookshop`.`over_leaked` `o`",
            ),
            // A string that says a refused function's name calls nothing.
            view(
                "idle",
                "select `p`.`id` AS `id` from `bookshop`.`sessions` `p` where `p`.`command` = 'Sleep'",
            ),
        ]
    }

    #[test]
    fn a_view_that_calls_a_refused_function_is_refused_at_any_depth() {
        let views = views();
        let calls = Err(Refusal::FunctionNotAllowed("load_file".into()));
        let check = |text: &str| check(Dialect::MySql, text, &views, &|_| None);
        assert_eq!(check("SELECT * FROM leaked"), calls);
        assert_eq!(check("SELECT * FROM bookshop.`Over_Leaked`"), calls);
        assert_eq!(
            check("SELECT t.title FROM titles t JOIN (SELECT 1 FROM over_over_leaked) o"),
            calls
        );
        assert_eq!(check("EXPLAIN SELECT * FROM leaked"), calls);
        // What reaches none of them reads, a longer name that holds one's
        // too.
        assert_eq!(check("SELECT * FROM titles"), Ok(()));
        assert_eq!(check("SELECT * FROM idle"), Ok(()));
        assert_eq!(check("SELECT * FROM leaked_copies"), Ok(()));
        assert_eq!(check("SELECT sleep_hours FROM books"), Ok(()));
        // Where none is refused, nothing is.
        let fine = [views[0].clone(), views[4].clone()];
        assert!(!any_refused(Dialect::MySql, &fine));
        assert!(any_refused(Dialect::MySql, &views));
    }

    #[test]
    fn a_view_whose_definition_is_not_shown_is_refused() {
        let views = [
            view("titles", ""),
            view("over_titles", "select 1 from `bookshop`.`titles`"),
        ];
        let unseen = Err(Refusal::NotAllowed(
            "The view bookshop.titles, whose definition this connection's user may not see,".into(),
        ));
        assert_eq!(
            check(Dialect::MySql, "SELECT * FROM titles", &views, &|_| None),
            unseen
        );
        assert_eq!(
            check(Dialect::MySql, "SELECT * FROM over_titles", &views, &|_| {
                None
            }),
            unseen
        );
        assert_eq!(
            check(Dialect::MySql, "SELECT * FROM books", &views, &|_| None),
            Ok(())
        );
    }

    /// The server's own views, shown without a body to a user who may read
    /// every table, refuse nothing, a table of the same name least of all.
    #[test]
    fn the_servers_own_views_are_not_refused() {
        let views: Vec<View> = ["sys", "mysql", "performance_schema"]
            .into_iter()
            .map(|schema| View {
                schema: schema.into(),
                name: "session".into(),
                definition: String::new(),
            })
            .collect();
        assert!(!any_refused(Dialect::MySql, &views));
        for text in [
            "SELECT * FROM bookshop.session",
            "SELECT @@session.time_zone",
            "SELECT * FROM sys.session",
        ] {
            assert_eq!(
                check(Dialect::MySql, text, &views, &|_| None),
                Ok(()),
                "{text}"
            );
        }
    }

    /// A view run with another user's rights that reads what the user is
    /// not shown, a view or a table, is refused, and a view over it; one
    /// that reads only what it is shown reads.
    #[test]
    fn a_view_that_reads_what_the_user_is_not_shown_is_refused() {
        let views = [
            view(
                "outer_v",
                "select `v`.`id` AS `id` from `other`.`inner_v` `v`",
            ),
            view(
                "over_outer",
                "select `bookshop`.`outer_v`.`id` AS `id` from `bookshop`.`outer_v`",
            ),
            view(
                "named",
                "with `c` as (select 1 AS `n`) select `bookshop`.`books`.`id` AS `id` \
                 from (`bookshop`.`books` join `c`)",
            ),
            // `PEEK` shown is another relation than `peek` where the
            // server tells names by their case.
            view(
                "cased",
                "select `bookshop`.`peek`.`id` AS `id` from `bookshop`.`peek`",
            ),
        ];
        let shown: HashSet<(String, String)> = [
            ("bookshop", "books"),
            ("bookshop", "outer_v"),
            ("bookshop", "PEEK"),
        ]
        .into_iter()
        .map(|(schema, name)| (schema.to_owned(), name.to_owned()))
        .collect();
        let unseen = |view: &View| reads_unseen(Dialect::MySql, view, &shown);
        let refused = Err(Refusal::NotAllowed(
            "The view bookshop.outer_v, which reads other.inner_v that this connection's user \
             may not see,"
                .into(),
        ));
        let shown_check = |text: &str| check(Dialect::MySql, text, &views, &unseen);
        assert_eq!(shown_check("SELECT * FROM outer_v"), refused);
        assert_eq!(shown_check("SELECT * FROM over_outer"), refused);
        assert_eq!(shown_check("SELECT * FROM named"), Ok(()));
        assert_eq!(
            shown_check("SELECT * FROM cased"),
            Err(Refusal::NotAllowed(
                "The view bookshop.cased, which reads bookshop.peek that this connection's \
                 user may not see,"
                    .into()
            ))
        );
        // Without what the user is shown, as for views run with their
        // reader's own rights, nothing is refused for it.
        assert!(!any_refused(Dialect::MySql, &views));
        assert_eq!(
            check(Dialect::MySql, "SELECT * FROM outer_v", &views, &|_| None),
            Ok(())
        );
    }

    /// A schema of the user's whose name differs from the server's own only
    /// in its case is the user's: its views are checked.
    #[test]
    fn a_users_schema_named_like_the_servers_is_checked() {
        let views = [View {
            schema: "SYS".into(),
            name: "lock_takers".into(),
            definition: "select get_lock('a', 0) AS `took`".into(),
        }];
        assert!(any_refused(Dialect::MySql, &views));
        assert_eq!(
            check(
                Dialect::MySql,
                "SELECT * FROM SYS.lock_takers",
                &views,
                &|_| None
            ),
            Err(Refusal::FunctionNotAllowed("get_lock".into()))
        );
    }

    /// PostgreSQL cuts a longer name to its 63 bytes, so a longer word that
    /// begins with a view's name that long reaches the view.
    #[test]
    fn a_longer_name_postgresql_cuts_to_a_views_is_found() {
        let long = "a".repeat(63);
        let views = [View {
            schema: "public".into(),
            name: long.clone(),
            definition: "SELECT pg_sleep(1) AS pg_sleep".into(),
        }];
        let longer = format!("SELECT * FROM {long}bbbb");
        let refused = Err(Refusal::FunctionNotAllowed("pg_sleep".into()));
        assert_eq!(
            check(Dialect::Postgres, &longer, &views, &|_| None),
            refused
        );
        assert!(names(Dialect::Postgres, &longer, &long));
        // A shorter name is not cut to, and MySQL cuts none.
        assert!(!names(Dialect::Postgres, "from leaked_x", "leaked"));
        assert!(!names(Dialect::MySql, &longer, &long));
    }

    #[test]
    fn a_name_is_found_as_a_word() {
        assert!(names(Dialect::MySql, "select * from `a b`", "a b"));
        assert!(names(Dialect::MySql, "from x.`a``b`", "a`b"));
        assert!(names(Dialect::MySql, "from x.\"a\"\"b\"", "a\"b"));
        assert!(names(Dialect::MySql, "from 1abc", "1abc"));
        assert!(!names(Dialect::MySql, "from x1abc", "1abc"));
        assert!(!names(Dialect::MySql, "from leaked_x", "leaked"));
        assert!(!names(Dialect::MySql, "from $leaked", "leaked"));
        assert!(names(Dialect::MySql, "from leaked;", "leaked"));
        assert!(names(Dialect::MySql, "from \"leaked\"", "leaked"));
    }
}

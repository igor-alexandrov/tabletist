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

use super::Refusal;
use super::hidden::refused_call;
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
/// at some depth, or that what it calls cannot be seen.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Why {
    Calls(String),
    Unseen(String),
}

impl Why {
    fn refusal(self) -> Refusal {
        match self {
            Self::Calls(function) => Refusal::FunctionNotAllowed(function),
            Self::Unseen(view) => Refusal::NotAllowed(format!(
                "The view {view}, whose definition this connection's user may not see,"
            )),
        }
    }
}

/// Why `view` itself may not be read, by its own body alone.
fn by_itself(dialect: Dialect, view: &View) -> Option<Why> {
    if view.definition.trim().is_empty() {
        return Some(Why::Unseen(format!("{}.{}", view.schema, view.name)));
    }
    refused_call(dialect, &view.definition).map(Why::Calls)
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
    views.iter().any(|view| by_itself(dialect, view).is_some())
}

/// Refuses `text`, an agent's statement, where it names a view of `views`
/// (all the database's) whose body calls a function an agent may not
/// call, or names such a view, at any depth.
pub(crate) fn check(dialect: Dialect, text: &str, views: &[View]) -> Result<(), Refusal> {
    // The refused views, by their folded names, and why each is.
    let mut refused: Vec<(String, Why)> = views
        .iter()
        .filter_map(|view| Some((view.name.to_lowercase(), by_itself(dialect, view)?)))
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
                let (_, why) = refused.iter().find(|(known, _)| names(body, known))?;
                Some((name.clone(), why.clone()))
            })
            .collect();
        if more.is_empty() {
            break;
        }
        refused.extend(more);
    }
    let text = text.to_lowercase();
    match refused.into_iter().find(|(name, _)| names(&text, name)) {
        Some((_, why)) => Err(why.refusal()),
        None => Ok(()),
    }
}

/// Whether `text` holds `name` as a word: not inside a longer name. Both
/// folded. A name that begins or ends with what no unquoted name holds
/// (`` `a b` ``) is found wherever it stands at that end.
fn names(text: &str, name: &str) -> bool {
    if name.is_empty() {
        return false;
    }
    let word = |character: char| character.is_alphanumeric() || matches!(character, '_' | '$');
    let starts = name.starts_with(word);
    let ends = name.ends_with(word);
    // A backquote in a name is written twice in backquotes.
    let doubled = name.replace('`', "``");
    let spellings: &[&str] = if doubled == name {
        &[name]
    } else {
        &[name, &doubled]
    };
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
        let check = |text: &str| check(Dialect::MySql, text, &views);
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
            check(Dialect::MySql, "SELECT * FROM titles", &views),
            unseen
        );
        assert_eq!(
            check(Dialect::MySql, "SELECT * FROM over_titles", &views),
            unseen
        );
        assert_eq!(check(Dialect::MySql, "SELECT * FROM books", &views), Ok(()));
    }

    #[test]
    fn a_name_is_found_as_a_word() {
        assert!(names("select * from `a b`", "a b"));
        assert!(names("from x.`a``b`", "a`b"));
        assert!(names("from 1abc", "1abc"));
        assert!(!names("from x1abc", "1abc"));
        assert!(!names("from leaked_x", "leaked"));
        assert!(!names("from $leaked", "leaked"));
        assert!(names("from leaked;", "leaked"));
        assert!(names("from \"leaked\"", "leaked"));
    }
}

//! What a coding agent may send through Tabletist's MCP server, decided on
//! the statement's text alone, before anything reaches a database.
//!
//! [`check_read`] is the check in front of the server's `run_query`. It
//! lets through what it knows, one statement that reads, and refuses
//! everything else, text it cannot read included: an agent can write its
//! query another way, and it is never given the benefit of the doubt.
//! [`Refusal`] is why: a code an agent may match on, and a sentence it can
//! act on.
//!
//! The check is the first fence, not the only one. What passes it runs as
//! any read-only script does ([`crate::ScriptMode::ReadOnly`]): prepared
//! alone by the database, in a read-only transaction that is rolled back.
//! A write this check took for a read is refused there.

use std::fmt;

use sqlparser::ast::{Query, SetExpr, Statement, UtilityOption};
use sqlparser::dialect::{MySqlDialect, PostgreSqlDialect, SQLiteDialect};
use sqlparser::parser::Parser;
use sqlparser::tokenizer::Tokenizer;

use crate::{Dialect, sql};

/// The longest statement that is read at all, in bytes.
pub const MAX_SQL_BYTES: usize = 100_000;

/// The most parameters one statement may be sent with.
pub const MAX_PARAMS: usize = 100;

/// The stack a statement is parsed on. sqlparser builds `1+1+1+…` into a
/// tree as deep as the chain is long, and dropping that tree takes a stack
/// frame for each level. The size cap bounds the depth, at two bytes of
/// text a level. Measured in a debug build, the deepest chain the cap lets
/// through needs between 4 and 8 MB to be dropped, which a thread of the
/// backend's runtime (2 MB) does not have, and a stack that overflows ends
/// the process. The pages are only touched as deep as a statement goes.
const CHECK_STACK_BYTES: usize = 64 * 1024 * 1024;

/// A value an agent sends for a placeholder of its statement.
#[derive(Clone, PartialEq)]
pub enum Param {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    Text(String),
}

/// Without the value: what an agent sends is not logged.
impl fmt::Debug for Param {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Null => "Null",
            Self::Bool(_) => "Bool(..)",
            Self::Int(_) => "Int(..)",
            Self::Float(_) => "Float(..)",
            Self::Text(_) => "Text(..)",
        })
    }
}

/// A statement that passed [`check_read`]. Its `Debug` output leaves the
/// text out, as [`sql::Statement`]'s does.
#[derive(Clone, PartialEq, Eq)]
pub struct Read {
    text: String,
    explains: bool,
}

impl Read {
    /// The statement, without the white space and the `;` around it.
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Whether it is an `EXPLAIN` of a query, which answers with a plan
    /// and reads no rows.
    pub fn explains(&self) -> bool {
        self.explains
    }
}

impl fmt::Debug for Read {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Read")
            .field("explains", &self.explains)
            .finish_non_exhaustive()
    }
}

/// Why a statement an agent sent was not run. Nothing reached the database.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Refusal {
    TooLarge(Large),
    /// More than one statement, or none.
    TooManyStatements,
    /// Text the checker could not read. It is refused, not guessed at.
    NotParsed,
    /// Something that writes, or takes locks, where only a read may run.
    NotReadOnly(Writes),
    /// A statement, a clause or an expression agents do not get: what it
    /// is, for the sentence.
    NotAllowed(String),
    /// A function with an effect outside its result, by its name as
    /// written.
    FunctionNotAllowed(String),
    InvalidParams(ParamFault),
}

/// What was too large.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Large {
    Statement,
    Params,
}

/// What in a statement is not a read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Writes {
    Insert,
    Update,
    Delete,
    Merge,
    /// `SELECT … INTO`, which makes a table, a file or a variable.
    Into,
    /// `FOR UPDATE`, `FOR SHARE` and the like.
    Lock,
}

/// What is wrong with a statement's parameters.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParamFault {
    /// The statement has `expected` placeholders and `sent` values came.
    Count { expected: usize, sent: usize },
    /// A placeholder that is not the dialect's own (`$1` on PostgreSQL,
    /// `?` on MySQL and SQLite), as written.
    Placeholder { written: String, dialect: Dialect },
    /// The parameter at this 1-based position holds a NUL character.
    Nul(usize),
    /// The parameter at this 1-based position is not a finite number.
    NotFinite(usize),
}

impl Refusal {
    /// The refusal's code, which an agent may match on. It never changes;
    /// the sentence may be reworded.
    pub fn code(&self) -> &'static str {
        match self {
            Self::TooLarge(_) => "too_large",
            Self::TooManyStatements => "too_many_statements",
            Self::NotParsed => "not_parsed",
            Self::NotReadOnly(_) => "not_read_only",
            Self::NotAllowed(_) => "not_allowed",
            Self::FunctionNotAllowed(_) => "function_not_allowed",
            Self::InvalidParams(_) => "invalid_params",
        }
    }
}

/// The sentence an agent is given.
impl fmt::Display for Refusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooLarge(Large::Statement) => write!(
                f,
                "The statement is larger than {} kB.",
                MAX_SQL_BYTES / 1_000
            ),
            Self::TooLarge(Large::Params) => {
                write!(f, "A statement takes at most {MAX_PARAMS} parameters.")
            }
            Self::TooManyStatements => f.write_str("Send one statement per call."),
            Self::NotParsed => {
                f.write_str("Tabletist could not read this statement, so it was not run.")
            }
            Self::NotReadOnly(writes) => {
                let what = match writes {
                    Writes::Insert => "an INSERT",
                    Writes::Update => "an UPDATE",
                    Writes::Delete => "a DELETE",
                    Writes::Merge => "a MERGE",
                    Writes::Into => {
                        return f.write_str(
                            "run_query only runs reads, and SELECT … INTO writes what it selects.",
                        );
                    }
                    Writes::Lock => {
                        return f.write_str(
                            "run_query only runs reads, and a locking clause such as FOR UPDATE \
                             is not one.",
                        );
                    }
                };
                write!(
                    f,
                    "run_query only runs reads, and this is {what}. Use propose_write to change \
                     data."
                )
            }
            Self::NotAllowed(what) | Self::FunctionNotAllowed(what) => {
                write!(f, "{what} is not available to agents.")
            }
            Self::InvalidParams(ParamFault::Count { expected, sent }) => {
                let (has, were) = (parameters(*expected), was(*sent));
                write!(f, "The statement has {has} and {were} sent.")
            }
            Self::InvalidParams(ParamFault::Placeholder { written, dialect }) => {
                let own = match dialect {
                    Dialect::Postgres => "$1, $2 and so on",
                    Dialect::MySql | Dialect::Sqlite => "?",
                };
                write!(
                    f,
                    "{written} is not a parameter Tabletist can fill. Write {own}."
                )
            }
            Self::InvalidParams(ParamFault::Nul(position)) => write!(
                f,
                "Parameter {position} holds a NUL character, which a database does not take."
            ),
            Self::InvalidParams(ParamFault::NotFinite(position)) => {
                write!(f, "Parameter {position} is not a finite number.")
            }
        }
    }
}

/// "1 parameter", "2 parameters".
fn parameters(count: usize) -> String {
    match count {
        1 => "1 parameter".to_owned(),
        _ => format!("{count} parameters"),
    }
}

/// "1 was", "2 were".
fn was(count: usize) -> String {
    match count {
        1 => "1 was".to_owned(),
        _ => format!("{count} were"),
    }
}

/// Whether `sql` is one statement that reads and may run for an agent with
/// `params`, on a database that speaks `dialect`. A statement that passes
/// is a query (`SELECT`, `WITH … SELECT`, `VALUES`, or those joined by
/// `UNION` and the like) or an `EXPLAIN` of one that does not run it.
///
/// Refused, with nothing run: more than one statement; a statement
/// [`sql::refusal`] refuses; a statement that writes; every other kind of
/// statement; and text the parser cannot read.
pub fn check_read(dialect: Dialect, sql: &str, params: &[Param]) -> Result<Read, Refusal> {
    if sql.len() > MAX_SQL_BYTES {
        return Err(Refusal::TooLarge(Large::Statement));
    }
    if params.len() > MAX_PARAMS {
        return Err(Refusal::TooLarge(Large::Params));
    }
    // No driver can send one, and each fails in its own way.
    if sql.contains('\0') {
        return Err(Refusal::NotParsed);
    }
    let statements = sql::statements(dialect, sql);
    let [statement] = statements.as_slice() else {
        return Err(Refusal::TooManyStatements);
    };
    if let Some(what) = sql::refusal(dialect, &statement.text) {
        return Err(Refusal::NotAllowed(what));
    }
    let text = statement.text.as_str();
    let explains = on_a_deep_stack(|| read_parsed(dialect, text))??;
    Ok(Read {
        text: text.to_owned(),
        explains,
    })
}

/// Runs `check` on a thread with [`CHECK_STACK_BYTES`] of stack. A thread
/// that cannot be started refuses the statement. So does a check that
/// panicked, in a build that unwinds: the app's release build aborts on a
/// panic, here as anywhere, so nothing in the check may rely on this.
fn on_a_deep_stack<T: Send>(check: impl FnOnce() -> T + Send) -> Result<T, Refusal> {
    std::thread::scope(|scope| {
        std::thread::Builder::new()
            .name("tabletist-agent-check".into())
            .stack_size(CHECK_STACK_BYTES)
            .spawn_scoped(scope, check)
            .map_err(|_| Refusal::NotParsed)?
            .join()
            .map_err(|_| Refusal::NotParsed)
    })
}

/// Parses `text` and checks it. Says whether it is an `EXPLAIN`. The tree
/// is built and dropped here, on the deep stack.
fn read_parsed(dialect: Dialect, text: &str) -> Result<bool, Refusal> {
    let parser: &dyn sqlparser::dialect::Dialect = match dialect {
        Dialect::Postgres => &PostgreSqlDialect {},
        Dialect::MySql => &MySqlDialect {},
        Dialect::Sqlite => &SQLiteDialect {},
    };
    let unread = || unread(dialect, text);
    let tokens = Tokenizer::new(parser, text)
        .tokenize()
        .map_err(|_| unread())?;
    let statements = Parser::new(parser)
        .with_tokens(tokens)
        .parse_statements()
        .map_err(|_| unread())?;
    // The splitter found one. A parser that finds another count read the
    // text differently, and neither reading is trusted.
    let [statement] = statements.as_slice() else {
        return Err(Refusal::TooManyStatements);
    };
    let explains = match statement {
        Statement::Query(query) => {
            query_reads(query)?;
            false
        }
        Statement::Explain {
            analyze,
            statement,
            options,
            ..
        } => {
            if *analyze {
                return Err(Refusal::NotAllowed("EXPLAIN ANALYZE".into()));
            }
            if let Some(option) = options.iter().flatten().find(|option| !plans_only(option)) {
                let name = option.name.value.to_ascii_uppercase();
                return Err(Refusal::NotAllowed(format!("EXPLAIN with {name}")));
            }
            match statement.as_ref() {
                Statement::Query(query) => query_reads(query)?,
                _ => {
                    return Err(Refusal::NotAllowed(
                        "EXPLAIN of anything but a query".into(),
                    ));
                }
            }
            true
        }
        other => return Err(not_a_read(dialect, text, other)),
    };
    Ok(explains)
}

/// Text the parser could not read. What does not even begin as a query is
/// named, so the agent learns that the statement is the trouble, not its
/// spelling: sqlparser does not read every statement a database takes
/// (`DO`, a `PRAGMA` with arguments).
fn unread(dialect: Dialect, text: &str) -> Refusal {
    let words = sql::words(dialect, text);
    let begins_as_a_query = match words.first().map(String::as_str) {
        Some("SELECT" | "WITH" | "VALUES" | "TABLE" | "EXPLAIN" | "DESCRIBE" | "DESC") | None => {
            true
        }
        Some(_) => false,
    };
    if begins_as_a_query {
        Refusal::NotParsed
    } else {
        Refusal::NotAllowed(named(dialect, text))
    }
}

/// Whether an option of `EXPLAIN (…)` only changes what the plan shows.
/// `ANALYZE` runs the statement, and what is not on this list is not known
/// to be harmless.
fn plans_only(option: &UtilityOption) -> bool {
    matches!(
        option.name.value.to_ascii_uppercase().as_str(),
        "FORMAT" | "VERBOSE" | "COSTS" | "SETTINGS" | "SUMMARY" | "GENERIC_PLAN"
    )
}

/// Why a statement that is not a query is refused: a write is told about
/// `propose_write`, everything else is named by its first word.
fn not_a_read(dialect: Dialect, text: &str, statement: &Statement) -> Refusal {
    match statement {
        Statement::Insert(_) => Refusal::NotReadOnly(Writes::Insert),
        Statement::Update(_) => Refusal::NotReadOnly(Writes::Update),
        Statement::Delete(_) => Refusal::NotReadOnly(Writes::Delete),
        Statement::Merge(_) => Refusal::NotReadOnly(Writes::Merge),
        _ => Refusal::NotAllowed(named(dialect, text)),
    }
}

/// What a statement is, for the sentence: its first word, and the second
/// one too after `CREATE`, `ALTER` and `DROP`.
fn named(dialect: Dialect, text: &str) -> String {
    let words = sql::words(dialect, text);
    match words.as_slice() {
        [first, second, ..] if matches!(first.as_str(), "CREATE" | "ALTER" | "DROP") => {
            format!("{first} {second}")
        }
        [first, ..] => first.clone(),
        [] => "This statement".to_owned(),
    }
}

/// Whether the statement's outermost query is one the three databases
/// have, and its body reads. What is nested in it is not walked: a tree is
/// as deep as its text is long.
fn query_reads(query: &Query) -> Result<(), Refusal> {
    // Named field by field: a field a later sqlparser adds must be decided
    // for here before this compiles.
    let Query {
        with: _,
        body,
        order_by: _,
        limit_clause: _,
        fetch: _,
        locks,
        for_clause,
        settings,
        format_clause,
        pipe_operators,
    } = query;
    if !locks.is_empty() {
        return Err(Refusal::NotReadOnly(Writes::Lock));
    }
    // Other databases' clauses, which none of the three parsers gives.
    if for_clause.is_some()
        || settings.is_some()
        || format_clause.is_some()
        || !pipe_operators.is_empty()
    {
        return Err(Refusal::NotParsed);
    }
    match body.as_ref() {
        SetExpr::Select(_)
        | SetExpr::Query(_)
        | SetExpr::Values(_)
        | SetExpr::SetOperation { .. } => Ok(()),
        // `WITH … INSERT`: a query to the parser, with a body that writes.
        SetExpr::Insert(_) => Err(Refusal::NotReadOnly(Writes::Insert)),
        SetExpr::Update(_) => Err(Refusal::NotReadOnly(Writes::Update)),
        SetExpr::Delete(_) => Err(Refusal::NotReadOnly(Writes::Delete)),
        SetExpr::Merge(_) => Err(Refusal::NotReadOnly(Writes::Merge)),
        SetExpr::Table(_) => Err(Refusal::NotAllowed("TABLE".into())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL: [Dialect; 3] = [Dialect::Postgres, Dialect::MySql, Dialect::Sqlite];

    /// The check of a statement sent without parameters.
    fn read(dialect: Dialect, sql: &str) -> Result<Read, Refusal> {
        check_read(dialect, sql, &[])
    }

    /// What a statement sent without parameters is refused for.
    fn refused(dialect: Dialect, sql: &str) -> Refusal {
        match read(dialect, sql) {
            Ok(_) => panic!("{dialect:?} let through: {sql}"),
            Err(refusal) => refusal,
        }
    }

    fn not_allowed(what: &str) -> Refusal {
        Refusal::NotAllowed(what.into())
    }

    #[test]
    fn every_refusal_has_its_code() {
        let count = ParamFault::Count {
            expected: 2,
            sent: 1,
        };
        for (refusal, code) in [
            (Refusal::TooLarge(Large::Statement), "too_large"),
            (Refusal::TooLarge(Large::Params), "too_large"),
            (Refusal::TooManyStatements, "too_many_statements"),
            (Refusal::NotParsed, "not_parsed"),
            (Refusal::NotReadOnly(Writes::Delete), "not_read_only"),
            (Refusal::NotReadOnly(Writes::Lock), "not_read_only"),
            (Refusal::NotAllowed("COPY".into()), "not_allowed"),
            (
                Refusal::FunctionNotAllowed("pg_sleep".into()),
                "function_not_allowed",
            ),
            (Refusal::InvalidParams(count), "invalid_params"),
        ] {
            assert_eq!(refusal.code(), code, "{refusal:?}");
        }
    }

    #[test]
    fn a_refusal_reads_as_a_sentence_for_the_agent() {
        let params = |fault| Refusal::InvalidParams(fault);
        let placeholder = |written: &str, dialect| ParamFault::Placeholder {
            written: written.into(),
            dialect,
        };
        for (refusal, sentence) in [
            (
                Refusal::TooLarge(Large::Statement),
                "The statement is larger than 100 kB.",
            ),
            (
                Refusal::TooLarge(Large::Params),
                "A statement takes at most 100 parameters.",
            ),
            (Refusal::TooManyStatements, "Send one statement per call."),
            (
                Refusal::NotParsed,
                "Tabletist could not read this statement, so it was not run.",
            ),
            (
                Refusal::NotReadOnly(Writes::Insert),
                "run_query only runs reads, and this is an INSERT. Use propose_write to change \
                 data.",
            ),
            (
                Refusal::NotReadOnly(Writes::Update),
                "run_query only runs reads, and this is an UPDATE. Use propose_write to change \
                 data.",
            ),
            (
                Refusal::NotReadOnly(Writes::Delete),
                "run_query only runs reads, and this is a DELETE. Use propose_write to change \
                 data.",
            ),
            (
                Refusal::NotReadOnly(Writes::Merge),
                "run_query only runs reads, and this is a MERGE. Use propose_write to change \
                 data.",
            ),
            (
                Refusal::NotReadOnly(Writes::Into),
                "run_query only runs reads, and SELECT … INTO writes what it selects.",
            ),
            (
                Refusal::NotReadOnly(Writes::Lock),
                "run_query only runs reads, and a locking clause such as FOR UPDATE is not one.",
            ),
            (
                Refusal::NotAllowed("CREATE TABLE".into()),
                "CREATE TABLE is not available to agents.",
            ),
            (
                Refusal::FunctionNotAllowed("pg_sleep".into()),
                "pg_sleep is not available to agents.",
            ),
            (
                params(ParamFault::Count {
                    expected: 2,
                    sent: 1,
                }),
                "The statement has 2 parameters and 1 was sent.",
            ),
            (
                params(ParamFault::Count {
                    expected: 1,
                    sent: 0,
                }),
                "The statement has 1 parameter and 0 were sent.",
            ),
            (
                params(placeholder(":id", Dialect::Postgres)),
                ":id is not a parameter Tabletist can fill. Write $1, $2 and so on.",
            ),
            (
                params(placeholder("?1", Dialect::Sqlite)),
                "?1 is not a parameter Tabletist can fill. Write ?.",
            ),
            (
                params(placeholder(":id", Dialect::MySql)),
                ":id is not a parameter Tabletist can fill. Write ?.",
            ),
            (
                params(ParamFault::Nul(2)),
                "Parameter 2 holds a NUL character, which a database does not take.",
            ),
            (
                params(ParamFault::NotFinite(1)),
                "Parameter 1 is not a finite number.",
            ),
        ] {
            assert_eq!(refusal.to_string(), sentence, "{refusal:?}");
        }
    }

    #[test]
    fn a_parameter_prints_without_its_value() {
        let printed = format!(
            "{:?}",
            [
                Param::Null,
                Param::Bool(true),
                Param::Int(9100000000000000004),
                Param::Float(19.5),
                Param::Text("ada@example.com".into()),
            ]
        );
        assert_eq!(printed, "[Null, Bool(..), Int(..), Float(..), Text(..)]");
    }

    #[test]
    fn a_query_is_let_through_on_every_dialect() {
        for dialect in ALL {
            for sql in [
                "SELECT id, title FROM books",
                "select b.title, count(*) from books b join book_covers c on c.book_id = b.id \
                 group by b.title order by 2 desc limit 10",
                "WITH recent AS (SELECT * FROM orders WHERE id > 100) SELECT count(*) FROM recent",
                "(SELECT id FROM books) UNION (SELECT id FROM authors)",
                "SELECT title FROM books WHERE id IN (SELECT book_id FROM book_covers)",
                "-- the covers of one publisher\nSELECT kind FROM book_covers /* all */",
            ] {
                let read = read(dialect, sql).unwrap_or_else(|refusal| {
                    panic!("{dialect:?} refused {sql}: {refusal:?}");
                });
                assert!(!read.explains(), "{dialect:?}: {sql}");
            }
        }
        assert!(read(Dialect::Postgres, "VALUES (1, 'print'), (2, 'ebook')").is_ok());
        assert!(read(Dialect::Sqlite, "VALUES (1, 'print'), (2, 'ebook')").is_ok());
    }

    #[test]
    fn what_passed_keeps_its_text_without_the_semicolon() {
        let read = read(Dialect::Postgres, "  SELECT title FROM books ;\n").unwrap();
        assert_eq!(read.text(), "SELECT title FROM books");
    }

    #[test]
    fn what_passed_prints_without_its_text() {
        let read = read(Dialect::Postgres, "SELECT title FROM books").unwrap();
        assert_eq!(format!("{read:?}"), "Read { explains: false, .. }");
    }

    #[test]
    fn more_than_one_statement_or_none_is_refused() {
        for dialect in ALL {
            for sql in [
                "SELECT 1; SELECT 2",
                "SELECT * FROM books; DELETE FROM book_covers",
                "SELECT 1;;SELECT 2;",
                "",
                "   ",
                ";",
                "-- only a comment",
            ] {
                assert_eq!(
                    refused(dialect, sql),
                    Refusal::TooManyStatements,
                    "{dialect:?}: {sql}"
                );
            }
        }
    }

    #[test]
    fn a_statement_or_a_list_of_parameters_over_the_cap_is_refused_unread() {
        let mut sql = String::from("SELECT '");
        sql.push_str(&"a".repeat(MAX_SQL_BYTES));
        sql.push('\'');
        assert_eq!(
            refused(Dialect::Postgres, &sql),
            Refusal::TooLarge(Large::Statement)
        );
        let params = vec![Param::Null; MAX_PARAMS + 1];
        assert_eq!(
            check_read(Dialect::Postgres, "SELECT 1", &params),
            Err(Refusal::TooLarge(Large::Params))
        );
    }

    #[test]
    fn a_nul_in_the_text_is_refused() {
        for dialect in ALL {
            assert_eq!(refused(dialect, "SELECT 'a\0b'"), Refusal::NotParsed);
        }
    }

    #[test]
    fn what_the_script_guard_refuses_is_refused_by_the_guards_name() {
        for (dialect, sql, what) in [
            (Dialect::Postgres, "COMMIT", "COMMIT"),
            (Dialect::Postgres, "COPY books TO STDOUT", "COPY"),
            (Dialect::Postgres, "PREPARE q AS SELECT 1", "PREPARE"),
            (
                Dialect::Postgres,
                "SELECT set_config('search_path', 'x', false)",
                "set_config",
            ),
            (Dialect::MySql, "SELECT /*! 1, */ 2", "a /*! */ comment"),
            (
                Dialect::MySql,
                "SELECT * FROM books INTO OUTFILE '/tmp/books'",
                "INTO OUTFILE",
            ),
            (Dialect::MySql, "CALL refill(1)", "CALL"),
            (
                Dialect::Sqlite,
                "PRAGMA query_only = OFF",
                "PRAGMA QUERY_ONLY",
            ),
            (Dialect::Sqlite, "ROLLBACK", "ROLLBACK"),
        ] {
            assert_eq!(
                refused(dialect, sql),
                not_allowed(what),
                "{dialect:?}: {sql}"
            );
        }
    }

    #[test]
    fn a_write_is_refused_as_what_it_is() {
        for dialect in ALL {
            for (sql, writes) in [
                ("INSERT INTO books (title) VALUES ('Emma')", Writes::Insert),
                ("UPDATE book_covers SET kind = 'ebook'", Writes::Update),
                ("DELETE FROM book_covers WHERE id = 9", Writes::Delete),
                (
                    "WITH n AS (SELECT 1 AS id) INSERT INTO books (id) SELECT id FROM n",
                    Writes::Insert,
                ),
            ] {
                assert_eq!(
                    refused(dialect, sql),
                    Refusal::NotReadOnly(writes),
                    "{dialect:?}: {sql}"
                );
            }
        }
        for (dialect, sql, writes) in [
            (
                Dialect::Postgres,
                "MERGE INTO books b USING authors a ON b.id = a.id WHEN MATCHED THEN DO NOTHING",
                Writes::Merge,
            ),
            (
                Dialect::MySql,
                "REPLACE INTO books (id, title) VALUES (1, 'Emma')",
                Writes::Insert,
            ),
            (
                Dialect::Sqlite,
                "INSERT OR REPLACE INTO books (id, title) VALUES (1, 'Emma')",
                Writes::Insert,
            ),
        ] {
            assert_eq!(
                refused(dialect, sql),
                Refusal::NotReadOnly(writes),
                "{dialect:?}: {sql}"
            );
        }
    }

    #[test]
    fn a_locking_clause_on_the_query_is_refused() {
        for (dialect, sql) in [
            (Dialect::Postgres, "SELECT * FROM books FOR UPDATE"),
            (
                Dialect::Postgres,
                "SELECT * FROM books FOR SHARE SKIP LOCKED",
            ),
            (Dialect::MySql, "SELECT * FROM books FOR UPDATE"),
            (Dialect::MySql, "SELECT * FROM books FOR SHARE"),
        ] {
            assert_eq!(
                refused(dialect, sql),
                Refusal::NotReadOnly(Writes::Lock),
                "{dialect:?}: {sql}"
            );
        }
    }

    #[test]
    fn every_other_statement_is_refused_by_its_name() {
        for (dialect, sql, what) in [
            (
                Dialect::Postgres,
                "CREATE TABLE notes (id int)",
                "CREATE TABLE",
            ),
            (Dialect::Postgres, "DROP TABLE books", "DROP TABLE"),
            (
                Dialect::Postgres,
                "ALTER TABLE books ADD COLUMN isbn text",
                "ALTER TABLE",
            ),
            (Dialect::Postgres, "TRUNCATE book_covers", "TRUNCATE"),
            (Dialect::Postgres, "SET search_path = public", "SET"),
            (Dialect::Postgres, "SHOW search_path", "SHOW"),
            (Dialect::Postgres, "CALL refill(1)", "CALL"),
            (Dialect::Postgres, "LISTEN orders", "LISTEN"),
            (Dialect::Postgres, "VACUUM books", "VACUUM"),
            (Dialect::Postgres, "ANALYZE books", "ANALYZE"),
            (Dialect::Postgres, "LOCK TABLE books", "LOCK"),
            (Dialect::Postgres, "GRANT SELECT ON books TO agent", "GRANT"),
            // Statements sqlparser does not read are named all the same.
            (Dialect::Postgres, "DO $$ BEGIN END $$", "DO"),
            (Dialect::MySql, "SHOW TABLES", "SHOW"),
            (Dialect::MySql, "DESCRIBE books", "DESCRIBE"),
            (Dialect::MySql, "USE bookshop_test", "USE"),
            (Dialect::MySql, "KILL 5", "KILL"),
            (Dialect::MySql, "HANDLER books OPEN", "HANDLER"),
            (Dialect::Sqlite, "PRAGMA table_info(books)", "PRAGMA"),
            (Dialect::Sqlite, "ATTACH 'other.db' AS other", "ATTACH"),
            (Dialect::Sqlite, "VACUUM", "VACUUM"),
            (
                Dialect::Sqlite,
                "CREATE INDEX books_title ON books (title)",
                "CREATE INDEX",
            ),
        ] {
            assert_eq!(
                refused(dialect, sql),
                not_allowed(what),
                "{dialect:?}: {sql}"
            );
        }
    }

    #[test]
    fn an_explain_passes_when_it_does_not_run_its_query() {
        for (dialect, sql) in [
            (Dialect::Postgres, "EXPLAIN SELECT * FROM books"),
            (
                Dialect::Postgres,
                "EXPLAIN (FORMAT JSON, VERBOSE) SELECT * FROM books",
            ),
            (Dialect::MySql, "EXPLAIN SELECT * FROM books"),
            (Dialect::MySql, "EXPLAIN FORMAT=JSON SELECT * FROM books"),
            (Dialect::MySql, "DESCRIBE SELECT * FROM books"),
            (Dialect::Sqlite, "EXPLAIN QUERY PLAN SELECT * FROM books"),
            (Dialect::Sqlite, "EXPLAIN SELECT * FROM books"),
        ] {
            let read = read(dialect, sql).unwrap_or_else(|refusal| {
                panic!("{dialect:?} refused {sql}: {refusal:?}");
            });
            assert!(read.explains(), "{dialect:?}: {sql}");
        }
        for (dialect, sql, what) in [
            (
                Dialect::Postgres,
                "EXPLAIN ANALYZE SELECT * FROM books",
                "EXPLAIN ANALYZE",
            ),
            (
                Dialect::Postgres,
                "EXPLAIN (ANALYZE, FORMAT JSON) SELECT * FROM books",
                "EXPLAIN with ANALYZE",
            ),
            (
                Dialect::Postgres,
                "EXPLAIN (analyse) SELECT * FROM books",
                "EXPLAIN with ANALYSE",
            ),
            (
                Dialect::Postgres,
                "EXPLAIN (BUFFERS) SELECT * FROM books",
                "EXPLAIN with BUFFERS",
            ),
            (
                Dialect::Postgres,
                "EXPLAIN DELETE FROM book_covers",
                "EXPLAIN of anything but a query",
            ),
            (
                Dialect::MySql,
                "EXPLAIN ANALYZE SELECT * FROM books",
                "EXPLAIN ANALYZE",
            ),
        ] {
            assert_eq!(
                refused(dialect, sql),
                not_allowed(what),
                "{dialect:?}: {sql}"
            );
        }
    }

    #[test]
    fn text_the_parser_cannot_read_is_refused_and_not_guessed_at() {
        let nested = format!("SELECT {}1{}", "(".repeat(60), ")".repeat(60));
        for dialect in ALL {
            for sql in [
                "SELECT title FROM",
                "SELECT 'unterminated FROM books",
                "select this is not sql",
                "WITH",
                nested.as_str(),
            ] {
                assert_eq!(
                    refused(dialect, sql),
                    Refusal::NotParsed,
                    "{dialect:?}: {sql}"
                );
            }
        }
    }

    /// A chain is a tree as deep as it is long. The check must come back
    /// from the deepest one the size cap lets through, on a thread with
    /// less stack than the backend's have.
    #[test]
    fn the_deepest_statements_under_the_cap_are_checked_from_a_small_stack() {
        for piece in [
            "+1",
            "::int",
            " AND 1",
            " UNION SELECT 1",
            "[1]",
            "||1",
            " IS NULL",
        ] {
            let mut sql = String::from("SELECT 1");
            while sql.len() + piece.len() <= MAX_SQL_BYTES {
                sql.push_str(piece);
            }
            let answer = std::thread::scope(|scope| {
                std::thread::Builder::new()
                    .stack_size(1024 * 1024)
                    .spawn_scoped(scope, || read(Dialect::Postgres, &sql).map(|_| ()))
                    .expect("start the thread")
                    .join()
                    .expect("the check came back")
            });
            assert_eq!(answer, Ok(()), "{piece}");
        }
    }
}

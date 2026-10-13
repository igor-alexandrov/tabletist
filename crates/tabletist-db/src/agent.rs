//! What a coding agent may send through Tabletist's MCP server, decided on
//! the statement's text alone, before anything reaches a database.
//!
//! [`check_read`] is the check in front of the server's `run_query`. It
//! lets through what it knows, one statement that reads, and refuses
//! everything else, text it cannot read included: an agent can write its
//! query another way, and it is never given the benefit of the doubt. What
//! a read may not hold is looked for in two readings of the text, by the
//! parser's tokenizer and by the app's own, and refused when either finds
//! it.
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
use sqlparser::tokenizer::{Token, Tokenizer};

use crate::sql::TokenKind;
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
/// [`sql::refusal`] refuses; anything that writes, locks or selects `INTO`,
/// at any depth; every other kind of statement; a function on the dialect's
/// list of those with effects; placeholders that do not match `params`; and
/// text the parser cannot read.
///
/// It blocks until a thread of its own has parsed the statement. A caller
/// on an async runtime calls it through `spawn_blocking`.
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
    let explains = on_a_deep_stack(|| read_parsed(dialect, text, params))??;
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
fn read_parsed(dialect: Dialect, text: &str, params: &[Param]) -> Result<bool, Refusal> {
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
        .with_tokens(tokens.clone())
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
    let placeholders = tokens_read(dialect, &tokens)?;
    // The second reading, by the tokenizer written to agree with each
    // database on where a string, a comment and a dollar quote end.
    words_read(dialect, &own_words(dialect, text))?;
    params_match(dialect, &placeholders, params)?;
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
/// as deep as its text is long, and [`words_read`] reads every level of it
/// without descending.
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

/// A token as far as this check reads it, from either tokenizer.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Piece {
    /// A keyword or a name, without its quotes.
    Word {
        text: String,
        quoted: bool,
    },
    /// `(`
    Open,
    /// `:=`
    Assign,
    Other,
}

/// Reads the tokens of a statement the parser took for a query, and
/// refuses what [`words_read`] refuses in them. Gives the placeholders it
/// met, as written, in the order of the text.
///
/// On tokens and not on the tree, for two reasons. A tree is as deep as
/// its text is long, and walking one costs far more stack than dropping it.
/// And a word is refused wherever the parser put it: this does not depend
/// on which kind of expression sqlparser makes of it.
fn tokens_read(dialect: Dialect, tokens: &[Token]) -> Result<Vec<String>, Refusal> {
    let code: Vec<&Token> = tokens
        .iter()
        .filter(|token| !matches!(token, Token::Whitespace(_)))
        .collect();
    let mut placeholders = Vec::new();
    let mut pieces = Vec::with_capacity(code.len());
    for (index, token) in code.iter().enumerate() {
        let next = code.get(index + 1).copied();
        pieces.push(match token {
            Token::Word(word) => Piece::Word {
                text: word.value.clone(),
                quoted: word.quote_style.is_some(),
            },
            Token::LParen => Piece::Open,
            Token::Assignment => Piece::Assign,
            _ => Piece::Other,
        });
        match token {
            Token::Placeholder(written) => placeholders.push(written.clone()),
            Token::Colon => placeholders.extend(named_placeholder(dialect, ':', next)),
            Token::AtSign => placeholders.extend(named_placeholder(dialect, '@', next)),
            _ => {}
        }
    }
    words_read(dialect, &pieces)?;
    Ok(placeholders)
}

/// The statement as the app's own tokenizer reads it, which was written
/// to end a string, a comment and a dollar quote where each database ends
/// it. sqlparser reads some text another way: `$1$ … $1$` is one string to
/// it, and a parameter followed by code to PostgreSQL. So what a read may
/// not hold is looked for in both readings, and found in either it is
/// refused.
fn own_words(dialect: Dialect, text: &str) -> Vec<Piece> {
    let tokens = sql::tokenize(dialect, text);
    let written = |token: &sql::Token| &text[token.range.clone()];
    let mut pieces = Vec::with_capacity(tokens.len());
    for (index, token) in tokens.iter().enumerate() {
        pieces.push(match token.kind {
            TokenKind::Whitespace | TokenKind::Comment => continue,
            TokenKind::Keyword | TokenKind::Identifier => Piece::Word {
                text: written(token).to_owned(),
                quoted: false,
            },
            TokenKind::QuotedIdentifier => Piece::Word {
                text: written(token)
                    .trim_start_matches(['"', '`', '['])
                    .trim_end_matches(['"', '`', ']'])
                    .to_owned(),
                quoted: true,
            },
            TokenKind::Punctuation if written(token) == "(" => Piece::Open,
            // `:=` is two tokens here, with nothing between them.
            TokenKind::Operator
                if written(token) == ":"
                    && tokens.get(index + 1).is_some_and(|next| {
                        next.range.start == token.range.end && written(next) == "="
                    }) =>
            {
                Piece::Assign
            }
            TokenKind::Punctuation
            | TokenKind::String
            | TokenKind::Number
            | TokenKind::ExecutableComment
            | TokenKind::Operator
            | TokenKind::Semicolon => Piece::Other,
        });
    }
    pieces
}

/// Refuses what a read may not hold at any depth, in one reading of its
/// tokens: a function with effects, a word that writes or locks, and on
/// MySQL an assignment to a variable.
///
/// It errs toward refusing. `SELECT delete FROM notes` reads a column, on
/// a database that lets one be named so without quotes, and is refused as
/// a `DELETE`: a name in quotes passes.
fn words_read(dialect: Dialect, pieces: &[Piece]) -> Result<(), Refusal> {
    for (index, piece) in pieces.iter().enumerate() {
        let (text, quoted) = match piece {
            Piece::Word { text, quoted } => (text, *quoted),
            // MySQL's `@total := @total + 1`: session state a rollback
            // does not undo. PostgreSQL spells a named argument so.
            Piece::Assign => match dialect {
                Dialect::MySql => {
                    return Err(Refusal::NotAllowed("An assignment to a variable".into()));
                }
                Dialect::Postgres | Dialect::Sqlite => continue,
            },
            Piece::Open | Piece::Other => continue,
        };
        let next = pieces.get(index + 1);
        // A name with a parenthesis after it is a call, in an expression
        // or in a table's place, whatever its quotes.
        if matches!(next, Some(Piece::Open)) {
            function_allowed(dialect, text)?;
            continue;
        }
        if quoted {
            continue;
        }
        let writes = match text.to_ascii_uppercase().as_str() {
            "INSERT" => Writes::Insert,
            "UPDATE" => Writes::Update,
            "DELETE" => Writes::Delete,
            "MERGE" => Writes::Merge,
            "INTO" => Writes::Into,
            // `FOR UPDATE`, `FOR NO KEY UPDATE`, `FOR SHARE`, `FOR KEY
            // SHARE`. `substring(title FROM 1 FOR 3)` is not one.
            "FOR" if locks_after(next) => Writes::Lock,
            // `TABLE books`: a table named where no later check of table
            // names looks. `SELECT * FROM books` says the same.
            "TABLE" => return Err(Refusal::NotAllowed("TABLE".into())),
            _ => continue,
        };
        return Err(Refusal::NotReadOnly(writes));
    }
    Ok(())
}

/// Whether the word after a `FOR` begins a locking clause.
fn locks_after(next: Option<&Piece>) -> bool {
    let Some(Piece::Word {
        text,
        quoted: false,
    }) = next
    else {
        return false;
    };
    matches!(
        text.to_ascii_uppercase().as_str(),
        "UPDATE" | "SHARE" | "NO" | "KEY"
    )
}

/// Refuses a function on the dialect's list, by its name without its
/// schema: `pg_catalog.pg_sleep(1)` is `pg_sleep`.
fn function_allowed(dialect: Dialect, name: &str) -> Result<(), Refusal> {
    let lower = name.to_ascii_lowercase();
    let (names, prefixes) = effects(dialect);
    if names.contains(&lower.as_str()) || prefixes.iter().any(|prefix| lower.starts_with(prefix)) {
        return Err(Refusal::FunctionNotAllowed(name.to_owned()));
    }
    Ok(())
}

/// The functions an agent's statement may not call, by name and by the
/// start of a name, lower-cased. They do something a read-only transaction
/// does not undo or refuse: end another session, take a lock, sleep, read a
/// file on the server, change a setting, or run SQL held in a string, which
/// no check here reads.
///
/// The lists are not complete and cannot be: an extension brings more. What
/// they miss still runs in the read-only transaction, as the connection's
/// own user.
fn effects(dialect: Dialect) -> (&'static [&'static str], &'static [&'static str]) {
    match dialect {
        Dialect::Postgres => (
            &[
                "set_config",
                "pg_terminate_backend",
                "pg_cancel_backend",
                "pg_reload_conf",
                "pg_rotate_logfile",
                "pg_notify",
                "pg_promote",
                "pg_switch_wal",
                "pg_stat_file",
                "pg_export_snapshot",
                "pg_start_backup",
                "pg_stop_backup",
                "nextval",
                "setval",
                // The session's random seed, which a rollback leaves set.
                "setseed",
                // The statistics of the whole server, from the extension
                // most often installed.
                "pg_stat_statements_reset",
                // They take a query as text and run it: full text search's
                // two, and those of the tablefunc and xml2 extensions.
                "ts_stat",
                "ts_rewrite",
                "connectby",
                "xpath_table",
                // Large objects to and from files on the server.
                "lo_import",
                "lo_export",
            ],
            &[
                "pg_sleep",
                "pg_advisory_",
                "pg_try_advisory_",
                "pg_read_",
                "pg_ls_",
                "pg_file_",
                "pg_logical_",
                "pg_replication_",
                "pg_create_",
                "pg_drop_",
                "pg_backup_",
                "pg_wal_replay_",
                "pg_stat_reset",
                "dblink",
                "crosstab",
                "query_to_xml",
                "table_to_xml",
                "cursor_to_xml",
                "schema_to_xml",
                "database_to_xml",
            ],
        ),
        Dialect::MySql => (
            &[
                "sleep",
                "benchmark",
                "get_lock",
                "release_lock",
                "release_all_locks",
                "load_file",
                "master_pos_wait",
                "source_pos_wait",
                "master_gtid_wait",
                "wait_for_executed_gtid_set",
                "wait_until_sql_thread_after_gtids",
                "sys_exec",
                "sys_eval",
                // MariaDB's sequences, which a rollback does not put back.
                "nextval",
                "setval",
            ],
            &[],
        ),
        Dialect::Sqlite => (
            &[
                "load_extension",
                "readfile",
                "writefile",
                "edit",
                "fts3_tokenizer",
            ],
            // The table-valued forms of the pragmas, `pragma_table_info`
            // and the rest: a `PRAGMA` statement is refused, and so is its
            // other spelling.
            &["pragma_"],
        ),
    }
}

/// A parameter by name, which the tokenizer gives as a sign and a word:
/// `:name` on MySQL and SQLite, `@name` on SQLite. PostgreSQL has neither
/// (its colon stands in `tags[1:n]`), and to MySQL `@name` is a variable of
/// the session, in one token.
fn named_placeholder(dialect: Dialect, sign: char, next: Option<&Token>) -> Option<String> {
    let Some(Token::Word(name)) = next else {
        return None;
    };
    let named = match (dialect, sign) {
        (Dialect::MySql | Dialect::Sqlite, ':') | (Dialect::Sqlite, '@') => true,
        (Dialect::Postgres | Dialect::MySql | Dialect::Sqlite, _) => false,
    };
    named.then(|| format!("{sign}{}", name.value))
}

/// Checks the placeholders a statement holds against the values sent.
fn params_match(
    dialect: Dialect,
    placeholders: &[String],
    params: &[Param],
) -> Result<(), Refusal> {
    let mut expected = 0;
    for written in placeholders {
        let position = match dialect {
            // `$1` may stand twice; the highest number is the count.
            Dialect::Postgres => written
                .strip_prefix('$')
                .and_then(|number| number.parse::<usize>().ok())
                .filter(|number| *number > 0),
            Dialect::MySql | Dialect::Sqlite => (written == "?").then_some(expected + 1),
        };
        let Some(position) = position else {
            return Err(Refusal::InvalidParams(ParamFault::Placeholder {
                written: written.clone(),
                dialect,
            }));
        };
        expected = expected.max(position);
    }
    if expected != params.len() {
        return Err(Refusal::InvalidParams(ParamFault::Count {
            expected,
            sent: params.len(),
        }));
    }
    for (index, param) in params.iter().enumerate() {
        let fault = match param {
            Param::Text(text) if text.contains('\0') => ParamFault::Nul(index + 1),
            Param::Float(number) if !number.is_finite() => ParamFault::NotFinite(index + 1),
            Param::Null | Param::Bool(_) | Param::Int(_) | Param::Float(_) | Param::Text(_) => {
                continue;
            }
        };
        return Err(Refusal::InvalidParams(fault));
    }
    Ok(())
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
            // A name in Unicode escapes can spell any function's.
            (
                Dialect::Postgres,
                "SELECT U&\"\\0070g_advisory_lock\"(1)",
                "a U& name",
            ),
            (
                Dialect::Postgres,
                "SELECT u&\"pg\\005fsleep\"(10)",
                "a U& name",
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

    #[test]
    fn a_write_inside_a_query_is_refused() {
        for (sql, writes) in [
            (
                "WITH gone AS (DELETE FROM book_covers WHERE id = 9 RETURNING *) \
                 SELECT * FROM gone",
                Writes::Delete,
            ),
            (
                "WITH added AS (INSERT INTO books (title) VALUES ('Emma') RETURNING id) \
                 SELECT * FROM added",
                Writes::Insert,
            ),
            (
                "WITH changed AS (UPDATE book_covers SET kind = 'ebook' RETURNING id) \
                 SELECT * FROM changed",
                Writes::Update,
            ),
            (
                "SELECT * FROM books WHERE id IN (WITH gone AS (DELETE FROM book_covers \
                 RETURNING book_id) SELECT book_id FROM gone)",
                Writes::Delete,
            ),
        ] {
            assert_eq!(
                refused(Dialect::Postgres, sql),
                Refusal::NotReadOnly(writes),
                "{sql}"
            );
        }
    }

    #[test]
    fn select_into_is_refused() {
        for (dialect, sql) in [
            (Dialect::Postgres, "SELECT * INTO books_copy FROM books"),
            (
                Dialect::Postgres,
                "SELECT * INTO TEMP books_copy FROM books",
            ),
            (
                Dialect::MySql,
                "SELECT title INTO @title FROM books LIMIT 1",
            ),
        ] {
            assert_eq!(
                refused(dialect, sql),
                Refusal::NotReadOnly(Writes::Into),
                "{dialect:?}: {sql}"
            );
        }
    }

    #[test]
    fn a_locking_clause_is_refused_at_any_depth() {
        for (dialect, sql) in [
            (
                Dialect::Postgres,
                "SELECT * FROM books WHERE id IN (SELECT book_id FROM book_covers FOR UPDATE)",
            ),
            (
                Dialect::Postgres,
                "SELECT * FROM (SELECT * FROM books FOR SHARE) b",
            ),
            (
                Dialect::MySql,
                "SELECT * FROM (SELECT * FROM books FOR UPDATE) b",
            ),
        ] {
            assert_eq!(
                refused(dialect, sql),
                Refusal::NotReadOnly(Writes::Lock),
                "{dialect:?}: {sql}"
            );
        }
        // FOR in another place is no lock.
        for dialect in [Dialect::Postgres, Dialect::MySql] {
            let sql = "SELECT substring(title FROM 1 FOR 3) FROM books";
            assert!(read(dialect, sql).is_ok(), "{dialect:?}");
        }
    }

    #[test]
    fn table_in_a_querys_place_is_refused() {
        assert_eq!(
            refused(Dialect::Postgres, "SELECT 1 UNION TABLE books"),
            not_allowed("TABLE")
        );
        // sqlparser does not read these at all.
        for sql in [
            "TABLE books",
            "SELECT * FROM authors WHERE id IN (TABLE books)",
        ] {
            assert_eq!(refused(Dialect::Postgres, sql), Refusal::NotParsed, "{sql}");
        }
    }

    #[test]
    fn an_assignment_to_a_variable_is_refused_on_mysql() {
        assert_eq!(
            refused(Dialect::MySql, "SELECT @total := @total + 1 FROM orders"),
            not_allowed("An assignment to a variable")
        );
        // Reading one changes nothing.
        assert!(read(Dialect::MySql, "SELECT @total").is_ok());
        // PostgreSQL's older spelling of a named argument.
        assert!(read(Dialect::Postgres, "SELECT make_interval(days := 2)").is_ok());
    }

    #[test]
    fn a_word_in_quotes_in_a_string_or_in_a_comment_is_no_statement() {
        for (dialect, sql) in [
            (
                Dialect::Postgres,
                "SELECT \"delete\", \"into\", \"table\", \"update\" FROM notes",
            ),
            (
                Dialect::Postgres,
                "SELECT 'DELETE FROM books', $$ INSERT INTO books $$ -- UPDATE books\nFROM notes",
            ),
            (
                Dialect::MySql,
                "SELECT `delete`, `into` FROM notes /* FOR UPDATE */",
            ),
            (Dialect::Sqlite, "SELECT [delete], \"into\" FROM notes"),
        ] {
            assert!(read(dialect, sql).is_ok(), "{dialect:?}: {sql}");
        }
        // Without quotes it is taken for the statement: the check errs
        // toward refusing.
        assert_eq!(
            refused(Dialect::Postgres, "SELECT delete FROM notes"),
            Refusal::NotReadOnly(Writes::Delete)
        );
    }

    #[test]
    fn a_function_named_like_a_write_is_a_function() {
        assert!(
            read(
                Dialect::MySql,
                "SELECT INSERT(title, 1, 1, 'E'), REPLACE(title, 'a', 'b') FROM books"
            )
            .is_ok()
        );
        assert!(
            read(
                Dialect::Sqlite,
                "SELECT replace(title, 'a', 'b') FROM books"
            )
            .is_ok()
        );
    }

    #[test]
    fn a_function_with_effects_is_refused_by_its_name_as_written() {
        for (dialect, sql, name) in [
            (Dialect::Postgres, "SELECT pg_sleep(1)", "pg_sleep"),
            (Dialect::Postgres, "SELECT PG_SLEEP(1)", "PG_SLEEP"),
            (
                Dialect::Postgres,
                "SELECT pg_sleep_for('1 minute')",
                "pg_sleep_for",
            ),
            (
                Dialect::Postgres,
                "SELECT pg_catalog . \"pg_sleep\" /* quietly */ (1)",
                "pg_sleep",
            ),
            (
                Dialect::Postgres,
                "SELECT * FROM pg_read_file('/etc/passwd')",
                "pg_read_file",
            ),
            (
                Dialect::Postgres,
                "SELECT * FROM books, LATERAL pg_sleep(1)",
                "pg_sleep",
            ),
            (
                Dialect::Postgres,
                "SELECT title FROM books WHERE id = (SELECT pg_advisory_lock(1))",
                "pg_advisory_lock",
            ),
            (
                Dialect::Postgres,
                "SELECT query_to_xml('SELECT * FROM customers', true, false, '')",
                "query_to_xml",
            ),
            (
                Dialect::Postgres,
                "SELECT nextval('books_id_seq')",
                "nextval",
            ),
            (Dialect::Postgres, "SELECT setseed(0.5)", "setseed"),
            // They run the query their text holds, and the call in it.
            (
                Dialect::Postgres,
                "SELECT * FROM ts_stat('SELECT to_tsvector(w) FROM (SELECT \
                 pg_advisory_lock(1)::text AS w) s')",
                "ts_stat",
            ),
            (
                Dialect::Postgres,
                "SELECT ts_rewrite('a'::tsquery, 'SELECT pg_sleep(1)::text::tsquery, NULL')",
                "ts_rewrite",
            ),
            (
                Dialect::Postgres,
                "SELECT * FROM crosstab('SELECT pg_sleep(1), 1, 1') AS t(a text, b int)",
                "crosstab",
            ),
            (
                Dialect::Postgres,
                "SELECT pg_stat_statements_reset()",
                "pg_stat_statements_reset",
            ),
            (
                Dialect::Postgres,
                "SELECT pg_terminate_backend(pid) FROM pg_stat_activity",
                "pg_terminate_backend",
            ),
            (
                Dialect::Postgres,
                "SELECT * FROM dblink('dbname=bookshop', 'SELECT 1') AS t(n int)",
                "dblink",
            ),
            (
                Dialect::Postgres,
                "SELECT lo_export(16411, '/tmp/cover')",
                "lo_export",
            ),
            (Dialect::MySql, "SELECT SLEEP(5)", "SLEEP"),
            (Dialect::MySql, "SELECT `sleep`(5)", "sleep"),
            (Dialect::MySql, "SELECT SLEEP (5)", "SLEEP"),
            // The tokens end the string where MySQL ends it.
            (Dialect::MySql, "SELECT 'it\\'s', sleep(1)", "sleep"),
            (Dialect::MySql, "SELECT GET_LOCK('covers', 10)", "GET_LOCK"),
            (
                Dialect::MySql,
                "SELECT LOAD_FILE('/etc/passwd')",
                "LOAD_FILE",
            ),
            (
                Dialect::MySql,
                "SELECT BENCHMARK(1000000, SHA2(title, 512)) FROM books",
                "BENCHMARK",
            ),
            // MariaDB, which this dialect also speaks to.
            (Dialect::MySql, "SELECT NEXTVAL(order_numbers)", "NEXTVAL"),
            (
                Dialect::MySql,
                "SELECT SETVAL(order_numbers, 1000000)",
                "SETVAL",
            ),
            (
                Dialect::Sqlite,
                "SELECT load_extension('covers')",
                "load_extension",
            ),
            (
                Dialect::Sqlite,
                "SELECT * FROM pragma_table_info('customers')",
                "pragma_table_info",
            ),
            (
                Dialect::Sqlite,
                "SELECT name FROM pragma_table_list()",
                "pragma_table_list",
            ),
        ] {
            assert_eq!(
                refused(dialect, sql),
                Refusal::FunctionNotAllowed(name.into()),
                "{dialect:?}: {sql}"
            );
        }
    }

    #[test]
    fn other_functions_and_a_listed_name_that_is_not_called_pass() {
        for (dialect, sql) in [
            (
                Dialect::Postgres,
                "SELECT count(*), lower(title), now() FROM books GROUP BY 2",
            ),
            (Dialect::Postgres, "SELECT * FROM generate_series(1, 3)"),
            (
                Dialect::Postgres,
                "SELECT ceil(1.5), extract(year FROM now()), trim(both ' ' from title) FROM books",
            ),
            (
                Dialect::Postgres,
                "SELECT 'pg_sleep(1)', \"pg_sleep\" FROM notes",
            ),
            // The large-object functions that write are left to the
            // read-only transaction.
            (
                Dialect::Postgres,
                "WITH lo_big(n) AS (SELECT 1) SELECT n FROM lo_big",
            ),
            // SLEEP is no function of PostgreSQL's, and pg_sleep none of
            // MySQL's: each list is its dialect's own.
            (Dialect::Postgres, "SELECT sleep(1)"),
            (Dialect::MySql, "SELECT pg_sleep(1)"),
            (
                Dialect::MySql,
                "SELECT COUNT(*), UPPER(title), IS_FREE_LOCK('covers') FROM books",
            ),
            (Dialect::Sqlite, "SELECT * FROM json_each('[1, 2]')"),
            (Dialect::Sqlite, "SELECT 'load_extension(''x'')'"),
        ] {
            assert!(read(dialect, sql).is_ok(), "{dialect:?}: {sql}");
        }
    }

    /// A name with a parenthesis after it is taken for a call wherever it
    /// stands: the check errs toward refusing.
    #[test]
    fn a_table_expression_named_like_a_listed_function_is_refused_too() {
        assert_eq!(
            refused(
                Dialect::Postgres,
                "WITH pg_sleep_log(n) AS (SELECT 1) SELECT n FROM pg_sleep_log"
            ),
            Refusal::FunctionNotAllowed("pg_sleep_log".into())
        );
    }

    /// Where a comment or a string ends decides what is code. Each case
    /// is read as its database reads it.
    #[test]
    fn a_call_is_seen_where_the_database_ends_a_comment_or_a_string() {
        for (dialect, sql, name) in [
            // PostgreSQL ends a line comment at a carriage return.
            (
                Dialect::Postgres,
                "SELECT 1 -- c\r, pg_sleep(1)",
                "pg_sleep",
            ),
            (
                Dialect::Postgres,
                "SELECT 1 -- c\r\n, pg_sleep(1)",
                "pg_sleep",
            ),
            // Its block comments nest.
            (
                Dialect::Postgres,
                "SELECT 1 /* a /* b */ c */ , pg_sleep(1)",
                "pg_sleep",
            ),
            // A backslash escapes nothing in its plain strings.
            (
                Dialect::Postgres,
                "SELECT 'a\\' , pg_sleep(1) --'",
                "pg_sleep",
            ),
            // To MySQL `--` opens a comment only before white space.
            (Dialect::MySql, "SELECT 1--1, SLEEP(5)", "SLEEP"),
            (Dialect::MySql, "SELECT 1 --1, SLEEP(5)", "SLEEP"),
            (Dialect::MySql, "SELECT 1 # c\n, SLEEP(5)", "SLEEP"),
            // Its block comments do not nest, and neither do SQLite's.
            (Dialect::MySql, "SELECT 1 /* a /* b */ , SLEEP(5)", "SLEEP"),
            (
                Dialect::Sqlite,
                "SELECT 1 /* a /* b */ , load_extension('x')",
                "load_extension",
            ),
            // SQLite has no backslash escapes.
            (
                Dialect::Sqlite,
                "SELECT \"a\\\" , load_extension('x') --\"",
                "load_extension",
            ),
        ] {
            assert_eq!(
                refused(dialect, sql),
                Refusal::FunctionNotAllowed(name.into()),
                "{dialect:?}: {sql:?}"
            );
        }
        // And the same names where the database reads a comment or a
        // string: they are text.
        for (dialect, sql) in [
            (Dialect::Postgres, "SELECT 1 /* a /* b */ , pg_sleep(1) */"),
            (Dialect::Postgres, "SELECT E'a\\' , pg_sleep(1) --'"),
            (Dialect::Postgres, "SELECT $a$ $b$ , pg_sleep(1) -- $a$"),
            // MySQL and SQLite end a line comment at a line feed only.
            (Dialect::MySql, "SELECT 1 -- c\r, SLEEP(5)"),
            (Dialect::MySql, "SELECT 1 # c\r, SLEEP(5)"),
            (Dialect::Sqlite, "SELECT 1 -- c\r, load_extension('x')"),
            (Dialect::MySql, "SELECT \"a\\\" , SLEEP(5) -- \""),
            (Dialect::MySql, "SELECT 'a'' , SLEEP(5) -- '"),
        ] {
            assert!(read(dialect, sql).is_ok(), "{dialect:?}: {sql:?}");
        }
    }

    /// The statement is read twice, by sqlparser's tokenizer and by the
    /// app's own, and refused for what either finds. Here sqlparser takes
    /// `$1$ … $1$` for one dollar-quoted string, which hides the call. The
    /// app's tokenizer reads a parameter and then code, as PostgreSQL does:
    /// a dollar quote's tag cannot begin with a digit.
    #[test]
    fn what_either_tokenizer_reads_as_a_call_or_a_write_is_refused() {
        assert_eq!(
            refused(Dialect::Postgres, "SELECT $1$ , pg_sleep(1) -- $1$"),
            Refusal::FunctionNotAllowed("pg_sleep".into())
        );
        assert_eq!(
            refused(Dialect::Postgres, "SELECT $1$ FROM books FOR UPDATE -- $1$"),
            Refusal::NotReadOnly(Writes::Lock)
        );
    }

    /// No text was found that MySQL takes and in which only one of the
    /// two tokenizers reads an assignment, so the app's own reading is
    /// asked by itself here.
    #[test]
    fn the_apps_own_reading_refuses_an_assignment_on_mysql_too() {
        let own = |dialect, sql| words_read(dialect, &own_words(dialect, sql));
        assert_eq!(
            own(Dialect::MySql, "SELECT @total := @total + 1 FROM orders"),
            Err(not_allowed("An assignment to a variable"))
        );
        // In a string, in a name and in a comment it is text.
        assert_eq!(
            own(
                Dialect::MySql,
                "SELECT @total, 'a := b', `:=` FROM orders -- c := d"
            ),
            Ok(())
        );
        assert_eq!(
            own(Dialect::Postgres, "SELECT make_interval(days := 2)"),
            Ok(())
        );
    }

    #[test]
    fn placeholders_are_counted_in_each_dialects_own_spelling() {
        let two = [Param::Int(4), Param::Text("print".into())];
        for (dialect, sql, params) in [
            (
                Dialect::Postgres,
                "SELECT id FROM book_covers WHERE publisher_id = $1 AND kind = $2",
                &two[..],
            ),
            // A number may stand twice, and in any order.
            (Dialect::Postgres, "SELECT $2, $1, $2", &two[..]),
            (
                Dialect::Postgres,
                "SELECT title FROM books LIMIT $1",
                &two[..1],
            ),
            (
                Dialect::MySql,
                "SELECT id FROM book_covers WHERE publisher_id = ? AND kind = ?",
                &two[..],
            ),
            (
                Dialect::Sqlite,
                "SELECT id FROM book_covers WHERE publisher_id = ? AND kind = ?",
                &two[..],
            ),
            (
                Dialect::Sqlite,
                "SELECT title FROM books LIMIT ?",
                &two[..1],
            ),
        ] {
            assert!(
                check_read(dialect, sql, params).is_ok(),
                "{dialect:?}: {sql}"
            );
        }
    }

    #[test]
    fn a_count_that_does_not_match_is_refused() {
        let count = |expected, sent| Refusal::InvalidParams(ParamFault::Count { expected, sent });
        let one = [Param::Int(4)];
        for dialect in ALL {
            assert_eq!(
                check_read(dialect, "SELECT title FROM books", &one),
                Err(count(0, 1)),
                "{dialect:?}"
            );
        }
        assert_eq!(refused(Dialect::Postgres, "SELECT $1, $2, $1"), count(2, 0));
        assert_eq!(
            check_read(Dialect::Postgres, "SELECT $3", &one),
            Err(count(3, 1))
        );
        assert_eq!(
            check_read(Dialect::MySql, "SELECT ?, ?", &one),
            Err(count(2, 1))
        );
        assert_eq!(refused(Dialect::Sqlite, "SELECT ?"), count(1, 0));
    }

    #[test]
    fn a_placeholder_in_a_string_is_text() {
        for (dialect, sql) in [
            (Dialect::Postgres, "SELECT '$1', $$ $2 $$"),
            (Dialect::MySql, "SELECT '?', \"?\""),
            (Dialect::Sqlite, "SELECT '?', ':id' -- ?"),
        ] {
            assert!(read(dialect, sql).is_ok(), "{dialect:?}: {sql}");
        }
    }

    #[test]
    fn a_placeholder_that_is_not_the_dialects_own_is_refused() {
        for (dialect, sql, written) in [
            (Dialect::Postgres, "SELECT $0", "$0"),
            (Dialect::Postgres, "SELECT $id", "$id"),
            (Dialect::MySql, "SELECT id FROM books WHERE id = :id", ":id"),
            (Dialect::Sqlite, "SELECT id FROM books WHERE id = ?1", "?1"),
            (
                Dialect::Sqlite,
                "SELECT id FROM books WHERE id = :id",
                ":id",
            ),
            (
                Dialect::Sqlite,
                "SELECT id FROM books WHERE id = @id",
                "@id",
            ),
            (
                Dialect::Sqlite,
                "SELECT id FROM books WHERE id = $id",
                "$id",
            ),
        ] {
            assert_eq!(
                refused(dialect, sql),
                Refusal::InvalidParams(ParamFault::Placeholder {
                    written: written.into(),
                    dialect,
                }),
                "{dialect:?}: {sql}"
            );
        }
        // PostgreSQL has no `?` parameter, and its parser none either.
        assert_eq!(
            refused(Dialect::Postgres, "SELECT id FROM books WHERE id = ?"),
            Refusal::NotParsed
        );
        // A variable of MySQL's session, and a slice of a PostgreSQL array.
        assert!(read(Dialect::MySql, "SELECT @id").is_ok());
        assert!(read(Dialect::Postgres, "SELECT tags[1:2] FROM books").is_ok());
    }

    #[test]
    fn a_parameter_no_database_takes_is_refused_by_its_position() {
        let sql = "SELECT $1, $2";
        assert_eq!(
            check_read(
                Dialect::Postgres,
                sql,
                &[Param::Int(1), Param::Text("a\0b".into())]
            ),
            Err(Refusal::InvalidParams(ParamFault::Nul(2)))
        );
        for number in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            assert_eq!(
                check_read(Dialect::Postgres, sql, &[Param::Float(number), Param::Null]),
                Err(Refusal::InvalidParams(ParamFault::NotFinite(1)))
            );
        }
        assert!(
            check_read(
                Dialect::Postgres,
                sql,
                &[Param::Float(19.5), Param::Bool(true)]
            )
            .is_ok()
        );
    }
}

//! SQL text as tokens, for highlighting, for splitting a script into
//! statements, and for the read-only guard. Tokenizing never fails; it only
//! has to agree with the database on where strings, comments and statements
//! end.

use std::ops::Range;

use crate::Dialect;

pub mod format;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenKind {
    Keyword,
    Identifier,
    /// `"name"`, `` `name` `` or `[name]`, quotes included.
    QuotedIdentifier,
    /// Quotes included; PostgreSQL `$tag$` bodies too.
    String,
    Number,
    Comment,
    /// MySQL's `/*! ... */`, whose contents MySQL runs.
    ExecutableComment,
    Operator,
    Punctuation,
    Semicolon,
    Whitespace,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    pub kind: TokenKind,
    /// Byte range in the text, always on char boundaries.
    pub range: Range<usize>,
}

/// Reserved words and common clauses, shared by every dialect.
const KEYWORDS: &[&str] = &[
    "ALL",
    "ALTER",
    "AND",
    "ANY",
    "AS",
    "ASC",
    "BEGIN",
    "BETWEEN",
    "BY",
    "CASE",
    "CAST",
    "COMMIT",
    "CREATE",
    "CROSS",
    "CURRENT",
    "DEFAULT",
    "DELETE",
    "DESC",
    "DISTINCT",
    "DROP",
    "ELSE",
    "END",
    "EXCEPT",
    "EXISTS",
    "EXPLAIN",
    "FALSE",
    "FETCH",
    "FILTER",
    "FIRST",
    "FOLLOWING",
    "FOR",
    "FROM",
    "FULL",
    "GROUP",
    "HAVING",
    "IN",
    "INNER",
    "INSERT",
    "INTERSECT",
    "INTERVAL",
    "INTO",
    "IS",
    "JOIN",
    "LAST",
    "LEFT",
    "LIKE",
    "LIMIT",
    "LOCKED",
    "NATURAL",
    "NO",
    "NOT",
    "NOTHING",
    "NOWAIT",
    "NULL",
    "NULLS",
    "OFFSET",
    "ON",
    "ONLY",
    "OR",
    "ORDER",
    "OUTER",
    "OVER",
    "PARTITION",
    "PRECEDING",
    "RECURSIVE",
    "RIGHT",
    "ROLLBACK",
    "ROW",
    "ROWS",
    "SELECT",
    "SET",
    "SHOW",
    "SKIP",
    "TABLE",
    "THEN",
    "TO",
    "TRUE",
    "UNBOUNDED",
    "UNION",
    "UPDATE",
    "USING",
    "VALUES",
    "VIEW",
    "WHEN",
    "WHERE",
    "WINDOW",
    "WITH",
];

const POSTGRES_KEYWORDS: &[&str] = &["ILIKE", "LATERAL", "RETURNING", "SIMILAR"];
const MYSQL_KEYWORDS: &[&str] = &["DESCRIBE", "REGEXP", "STRAIGHT_JOIN"];
const SQLITE_KEYWORDS: &[&str] = &["GLOB", "PRAGMA", "RETURNING"];

/// The keywords `dialect` highlights, in upper case: the words the editor
/// completes too. A keyword worth completing is worth colouring, so there
/// is one list.
pub fn keywords(dialect: Dialect) -> impl Iterator<Item = &'static str> {
    let own = match dialect {
        Dialect::Postgres => POSTGRES_KEYWORDS,
        Dialect::MySql => MYSQL_KEYWORDS,
        Dialect::Sqlite => SQLITE_KEYWORDS,
    };
    KEYWORDS.iter().chain(own).copied()
}

/// Whether `word` (any case) is highlighted as a keyword in `dialect`.
pub fn is_keyword(dialect: Dialect, word: &str) -> bool {
    keywords(dialect).any(|keyword| keyword.eq_ignore_ascii_case(word))
}

/// Splits `text` into tokens that cover it end to end.
pub fn tokenize(dialect: Dialect, text: &str) -> Vec<Token> {
    let bytes = text.as_bytes();
    let mut tokens = Vec::new();
    let mut at = 0;
    while at < bytes.len() {
        let start = at;
        let (kind, end) = next(dialect, text, start);
        // A hard assert: an arm that stopped short would loop forever.
        assert!(end > start && text.is_char_boundary(end));
        tokens.push(Token {
            kind,
            range: start..end,
        });
        at = end;
    }
    tokens
}

fn next(dialect: Dialect, text: &str, start: usize) -> (TokenKind, usize) {
    let bytes = text.as_bytes();
    let byte = bytes[start];
    let peek = |offset: usize| bytes.get(start + offset).copied();
    match byte {
        b if is_space(b) => (TokenKind::Whitespace, scan(bytes, start, is_space)),
        b'-' if peek(1) == Some(b'-') && dash_comment(dialect, peek(2)) => {
            (TokenKind::Comment, line_end(dialect, bytes, start))
        }
        b'#' if dialect == Dialect::MySql => (TokenKind::Comment, line_end(dialect, bytes, start)),
        b'/' if peek(1) == Some(b'*') => {
            // `/*!` and MariaDB's `/*M!`: MySQL runs what is inside.
            let executable =
                peek(2) == Some(b'!') || (peek(2) == Some(b'M') && peek(3) == Some(b'!'));
            let kind = if dialect == Dialect::MySql && executable {
                TokenKind::ExecutableComment
            } else {
                TokenKind::Comment
            };
            (
                kind,
                block_comment_end(bytes, start, dialect == Dialect::Postgres),
            )
        }
        b'\'' => (
            TokenKind::String,
            quoted_end(bytes, start, b'\'', dialect == Dialect::MySql),
        ),
        b'E' | b'e' if dialect == Dialect::Postgres && peek(1) == Some(b'\'') => {
            (TokenKind::String, escape_string_end(bytes, start))
        }
        b'"' if dialect == Dialect::MySql => {
            (TokenKind::String, quoted_end(bytes, start, b'"', true))
        }
        b'"' => (
            TokenKind::QuotedIdentifier,
            quoted_end(bytes, start, b'"', false),
        ),
        b'`' if dialect != Dialect::Postgres => (
            TokenKind::QuotedIdentifier,
            quoted_end(bytes, start, b'`', false),
        ),
        b'[' if dialect == Dialect::Sqlite => {
            let end = bytes[start..]
                .iter()
                .position(|&b| b == b']')
                .map_or(bytes.len(), |offset| start + offset + 1);
            (TokenKind::QuotedIdentifier, end)
        }
        b'$' if dialect == Dialect::Postgres => match dollar_tag(bytes, start) {
            Some(tag_end) => {
                let tag = &bytes[start..tag_end];
                let body = find(bytes, tag_end, tag).map_or(bytes.len(), |at| at + tag.len());
                (TokenKind::String, body)
            }
            // `$1`: a parameter, one piece of punctuation.
            None => (
                TokenKind::Punctuation,
                scan(bytes, start + 1, |b| b.is_ascii_digit()),
            ),
        },
        b'0'..=b'9' => (TokenKind::Number, number_end(bytes, start)),
        b'.' if peek(1).is_some_and(|b| b.is_ascii_digit()) => {
            (TokenKind::Number, number_end(bytes, start))
        }
        b';' => (TokenKind::Semicolon, start + 1),
        b if is_word_start(b) => {
            let end = scan(bytes, start, |b| is_word_part(dialect, b));
            let kind = if is_keyword(dialect, &text[start..end]) {
                TokenKind::Keyword
            } else {
                TokenKind::Identifier
            };
            (kind, end)
        }
        b'+' | b'-' | b'*' | b'/' | b'<' | b'>' | b'=' | b'~' | b'!' | b'@' | b'#' | b'%'
        | b'^' | b'&' | b'|' | b'?' | b':' => (TokenKind::Operator, start + 1),
        _ => {
            // One whole character, however many bytes.
            let width = text[start..].chars().next().map_or(1, char::len_utf8);
            (TokenKind::Punctuation, start + width)
        }
    }
}

/// `--` starts a comment everywhere but MySQL, where whitespace or a
/// control character (or the end) must follow: `1--1` is arithmetic.
fn dash_comment(dialect: Dialect, after: Option<u8>) -> bool {
    dialect != Dialect::MySql || after.is_none_or(|b| is_space(b) || b.is_ascii_control())
}

fn is_word_start(byte: u8) -> bool {
    byte.is_ascii_alphabetic() || byte == b'_' || byte >= 0x80
}

fn is_word_part(dialect: Dialect, byte: u8) -> bool {
    byte.is_ascii_alphanumeric()
        || byte == b'_'
        || byte >= 0x80
        || (byte == b'$' && dialect != Dialect::Sqlite)
}

fn scan(bytes: &[u8], from: usize, keep: impl Fn(u8) -> bool) -> usize {
    bytes[from..]
        .iter()
        .position(|&b| !keep(b))
        .map_or(bytes.len(), |offset| from + offset)
}

/// Whitespace as MySQL, SQLite and PostgreSQL 17+ read it (vertical tab
/// included, which Rust's `is_ascii_whitespace` leaves out). Earlier
/// PostgreSQL rejects a bare vertical tab, so treating it as space is safe.
fn is_space(byte: u8) -> bool {
    byte.is_ascii_whitespace() || byte == 0x0B
}

/// The end of a line comment: the newline stays outside it. PostgreSQL ends
/// it at a carriage return too; MySQL and SQLite only at a line feed.
fn line_end(dialect: Dialect, bytes: &[u8], from: usize) -> usize {
    let cr_ends = dialect == Dialect::Postgres;
    scan(bytes, from, |b| b != b'\n' && !(cr_ends && b == b'\r'))
}

fn find(bytes: &[u8], from: usize, needle: &[u8]) -> Option<usize> {
    bytes[from..]
        .windows(needle.len())
        .position(|window| window == needle)
        .map(|offset| from + offset)
}

fn block_comment_end(bytes: &[u8], start: usize, nests: bool) -> usize {
    let mut depth = 0usize;
    let mut at = start;
    while at + 1 < bytes.len() {
        match (bytes[at], bytes[at + 1]) {
            (b'/', b'*') if depth == 0 || nests => {
                depth += 1;
                at += 2;
            }
            (b'*', b'/') => {
                depth -= 1;
                at += 2;
                if depth == 0 {
                    return at;
                }
            }
            _ => at += 1,
        }
    }
    bytes.len()
}

/// A quoted run from `start` (the opening quote): a doubled quote stays
/// inside, and with `backslash` a backslash escapes the next byte.
fn quoted_end(bytes: &[u8], start: usize, quote: u8, backslash: bool) -> usize {
    let mut at = start + 1;
    while at < bytes.len() {
        match bytes[at] {
            b'\\' if backslash => at += 2,
            b if b == quote => {
                if bytes.get(at + 1) == Some(&quote) {
                    at += 2;
                } else {
                    return at + 1;
                }
            }
            _ => at += 1,
        }
    }
    bytes.len()
}

/// An `E'...'` string from `start` (the `E`). PostgreSQL continues it over a
/// newline into the next quote, still with backslash escapes, so the whole
/// run is one token. Plain strings are left alone: with no escapes their
/// pieces end where the continued string would, and each piece is a string.
fn escape_string_end(bytes: &[u8], start: usize) -> usize {
    let mut end = quoted_end(bytes, start + 1, b'\'', true);
    while let Some(quote) = string_continuation(bytes, end) {
        end = quoted_end(bytes, quote, b'\'', true);
    }
    end
}

/// After a string closes at `from`: blanks and comments, a newline, then
/// more blanks and comment lines, then a quote. That quote's offset, if so.
fn string_continuation(bytes: &[u8], from: usize) -> Option<usize> {
    let non_newline = |b: u8| b != b'\n' && b != b'\r';
    let is_newline = |at: usize| matches!(bytes.get(at), Some(b'\n' | b'\r'));
    let is_comment = |at: usize| bytes.get(at) == Some(&b'-') && bytes.get(at + 1) == Some(&b'-');
    let mut at = from;
    loop {
        match bytes.get(at) {
            Some(b' ' | b'\t' | 0x0B | 0x0C) => at += 1,
            _ if is_comment(at) => at = scan(bytes, at, non_newline),
            _ => break,
        }
    }
    if !is_newline(at) {
        return None;
    }
    loop {
        match bytes.get(at) {
            Some(b'\'') => return Some(at),
            Some(&b) if is_space(b) => at += 1,
            _ if is_comment(at) => {
                at = scan(bytes, at, non_newline);
                // A comment here must end in a newline.
                if !is_newline(at) {
                    return None;
                }
            }
            _ => return None,
        }
    }
}

/// `$tag$` or `$$` at `start`: the end of the opening tag.
fn dollar_tag(bytes: &[u8], start: usize) -> Option<usize> {
    let tag_end = scan(bytes, start + 1, |b| {
        b.is_ascii_alphanumeric() || b == b'_' || b >= 0x80
    });
    let tag = &bytes[start + 1..tag_end];
    let starts_well = tag.first().is_none_or(|b| !b.is_ascii_digit());
    (starts_well && bytes.get(tag_end) == Some(&b'$')).then_some(tag_end + 1)
}

fn number_end(bytes: &[u8], start: usize) -> usize {
    let mut at = scan(bytes, start, |b| b.is_ascii_digit());
    if bytes.get(at) == Some(&b'.') {
        at = scan(bytes, at + 1, |b| b.is_ascii_digit());
    }
    if matches!(bytes.get(at), Some(b'e' | b'E')) {
        let sign = usize::from(matches!(bytes.get(at + 1), Some(b'+' | b'-')));
        if bytes.get(at + 1 + sign).is_some_and(u8::is_ascii_digit) {
            at = scan(bytes, at + 1 + sign, |b| b.is_ascii_digit());
        }
    }
    at
}

/// One statement of a script, as the editor shows it. Its `Debug` output
/// leaves the text out: what a user types into the editor is never logged.
#[derive(Clone, PartialEq, Eq)]
pub struct Statement {
    /// Byte range in the script, without surrounding whitespace or the `;`.
    pub range: Range<usize>,
    /// Byte offset just past its `;`, or `range.end` when it has none.
    pub end: usize,
    /// 1-based line of `range.start`.
    pub start_line: usize,
    /// Characters before `range.start` on its line.
    pub start_column: usize,
    /// 1-based line of its first token that is not a comment: the SQL,
    /// not a comment above it.
    pub first_line: usize,
    /// The script's text in `range`.
    pub text: String,
}

impl std::fmt::Debug for Statement {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Statement")
            .field("range", &self.range)
            .field("end", &self.end)
            .field("start_line", &self.start_line)
            .field("start_column", &self.start_column)
            .field("first_line", &self.first_line)
            .finish_non_exhaustive()
    }
}

impl Statement {
    /// The 1-based line and column in the script of a 1-based character
    /// `position` in `text` (as PostgreSQL reports error positions).
    pub fn line_col(&self, position: usize) -> (usize, usize) {
        let before: String = self.text.chars().take(position.saturating_sub(1)).collect();
        let lines = before.matches('\n').count();
        let column = before
            .rsplit('\n')
            .next()
            .map_or(0, |line| line.chars().count())
            + 1;
        let first_column = if lines == 0 { self.start_column } else { 0 };
        (self.start_line + lines, column + first_column)
    }
}

/// Whether a token is SQL the server runs. An executable comment counts:
/// MySQL and MariaDB run its contents.
pub(crate) fn is_code(kind: TokenKind) -> bool {
    !matches!(kind, TokenKind::Whitespace | TokenKind::Comment)
}

/// A line and column that only moves forward through a text, so asking
/// for many increasing positions costs one pass in all.
struct Walker<'a> {
    text: &'a str,
    byte: usize,
    line: usize,
    column: usize,
}

impl<'a> Walker<'a> {
    fn new(text: &'a str) -> Self {
        Self {
            text,
            byte: 0,
            line: 1,
            column: 0,
        }
    }

    /// The 1-based line and the characters before `byte` on it. `byte` must
    /// not be before the last one asked for.
    fn at(&mut self, byte: usize) -> (usize, usize) {
        for ch in self.text[self.byte..byte].chars() {
            if ch == '\n' {
                self.line += 1;
                self.column = 0;
            } else {
                self.column += 1;
            }
        }
        self.byte = byte;
        (self.line, self.column)
    }
}

/// The statements of `script`, split on `;` tokens. Pieces holding only
/// whitespace and comments are dropped.
pub fn statements(dialect: Dialect, script: &str) -> Vec<Statement> {
    statements_from(script, &tokenize(dialect, script))
}

/// [`statements`] of a `script` already tokenized into `tokens`, for a
/// caller that needs the tokens as well (the editor colours them).
pub fn statements_from(script: &str, tokens: &[Token]) -> Vec<Statement> {
    let mut found = Vec::new();
    let mut walker = Walker::new(script);
    for piece in tokens.split_inclusive(|token| token.kind == TokenKind::Semicolon) {
        let body: &[Token] = match piece.last() {
            Some(last) if last.kind == TokenKind::Semicolon => &piece[..piece.len() - 1],
            _ => piece,
        };
        let Some(first_code) = body.iter().find(|token| is_code(token.kind)) else {
            continue;
        };
        let mut visible = body
            .iter()
            .filter(|token| token.kind != TokenKind::Whitespace);
        let (Some(first), Some(last)) = (visible.clone().next(), visible.next_back()) else {
            continue;
        };
        let range = first.range.start..last.range.end;
        let end = match piece.last() {
            Some(last) if last.kind == TokenKind::Semicolon => last.range.end,
            _ => range.end,
        };
        let (start_line, start_column) = walker.at(range.start);
        let (first_line, _) = walker.at(first_code.range.start);
        found.push(Statement {
            start_line,
            start_column,
            first_line,
            text: script[range.clone()].to_owned(),
            range,
            end,
        });
    }
    found
}

/// The statement Run executes for a cursor at byte `cursor`: the one whose
/// range, extended to its `;`, holds it; else the nearest one ending
/// before it; else the first after it.
pub fn statement_at(statements: &[Statement], cursor: usize) -> Option<&Statement> {
    statements
        .iter()
        .find(|statement| statement.range.start <= cursor && cursor <= statement.end)
        .or_else(|| {
            statements
                .iter()
                .rev()
                .find(|statement| statement.end <= cursor)
        })
        .or_else(|| statements.first())
}

/// The statement's words (keywords and names, quotes removed), upper-cased,
/// in order. Only keywords and names count: strings, numbers, operators,
/// punctuation and comments (executable ones included) are skipped. The
/// read-only guard checks executable comments separately.
pub fn words(dialect: Dialect, text: &str) -> Vec<String> {
    tokenize(dialect, text)
        .iter()
        .filter_map(|token| word_of(text, token))
        .collect()
}

/// The word a token spells, if it is a keyword or a name.
fn word_of(text: &str, token: &Token) -> Option<String> {
    let word = &text[token.range.clone()];
    match token.kind {
        TokenKind::Keyword | TokenKind::Identifier => Some(word.to_ascii_uppercase()),
        // Strip the quotes, not a byte count: an unterminated name can
        // end inside a multi-byte character.
        TokenKind::QuotedIdentifier => Some(
            word.trim_start_matches(['"', '`', '['])
                .trim_end_matches(['"', '`', ']'])
                .to_ascii_uppercase(),
        ),
        _ => None,
    }
}

/// Why `statement` must not run in Tabletist's read-only transaction: the
/// statement kind it is, for the message. `None` when it may run. Matched
/// on tokens, so `SELECT 'COMMIT'` and a column named `end_date` pass.
pub fn refusal(dialect: Dialect, statement: &str) -> Option<String> {
    let tokens = tokenize(dialect, statement);
    if dialect == Dialect::MySql
        && tokens
            .iter()
            .any(|token| token.kind == TokenKind::ExecutableComment)
    {
        return Some("a /*! */ comment".into());
    }
    // A U&"..." name can spell any name in escapes, set_config included.
    if dialect == Dialect::Postgres && unicode_name(statement, &tokens) {
        return Some("a U& name".into());
    }
    let mut words: Vec<String> = tokens
        .iter()
        .filter_map(|token| word_of(statement, token))
        .collect();
    // MariaDB's CREATE OR REPLACE USER / ROLE: match them as CREATE USER.
    if dialect == Dialect::MySql
        && words.len() > 2
        && words[0] == "CREATE"
        && words[1] == "OR"
        && words[2] == "REPLACE"
    {
        words.drain(1..3);
    }
    let word = |index: usize| words.get(index).map(String::as_str).unwrap_or_default();
    let guarded_name = || words.iter().skip(1).find(|name| is_guarded_setting(name));
    let postgres_or_mysql = dialect != Dialect::Sqlite;
    match word(0) {
        first @ ("BEGIN" | "START" | "COMMIT" | "END" | "ROLLBACK" | "ABORT" | "SAVEPOINT"
        | "RELEASE") => return Some(first.to_owned()),
        // The SQL they run is a string the guard cannot read, and prepared
        // statements outlive the rollback.
        "PREPARE" if dialect == Dialect::Postgres && word(1) == "TRANSACTION" => {
            return Some("PREPARE TRANSACTION".into());
        }
        first @ ("PREPARE" | "EXECUTE" | "DEALLOCATE") if postgres_or_mysql => {
            return Some(first.to_owned());
        }
        "SET" => {
            if let Some(refused) = set_refusal(dialect, statement, &tokens, &words) {
                return Some(refused);
            }
        }
        "RESET" if dialect == Dialect::MySql => {
            return Some(match word(1) {
                "" => "RESET".to_owned(),
                second => format!("RESET {second}"),
            });
        }
        "RESET" if word(1) == "ALL" => return Some("RESET ALL".into()),
        "RESET" => {
            if let Some(name) = guarded_name() {
                return Some(format!("RESET {name}"));
            }
        }
        "DISCARD" if word(1) == "ALL" => return Some("DISCARD ALL".into()),
        "COPY" if dialect == Dialect::Postgres => return Some("COPY".into()),
        _ => {}
    }
    // set_config() changes the same settings as SET, from any statement.
    if dialect == Dialect::Postgres && words.iter().any(|word| word == "SET_CONFIG") {
        return Some("set_config".into());
    }
    if dialect != Dialect::MySql {
        return None;
    }
    // The session's default database is not reset by the cleanup; a server
    // that prepares USE would carry it into browsing and the next run.
    if word(0) == "USE" {
        return Some("USE".into());
    }
    if matches!(
        word(0),
        "XA" | "LOCK"
            | "UNLOCK"
            | "CALL"
            | "GRANT"
            | "REVOKE"
            | "FLUSH"
            | "INSTALL"
            | "UNINSTALL"
            | "PURGE"
            | "CHANGE"
            | "STOP"
            | "SHUTDOWN"
            | "RESTART"
            | "CLONE"
            | "BACKUP"
    ) {
        return Some(word(0).to_owned());
    }
    if let Some((first, second)) = [
        ("CREATE", "USER"),
        ("ALTER", "USER"),
        ("DROP", "USER"),
        ("RENAME", "USER"),
        ("CREATE", "ROLE"),
        ("DROP", "ROLE"),
        ("DROP", "PREPARE"),
        ("ALTER", "INSTANCE"),
        ("SET", "PASSWORD"),
        ("SET", "DEFAULT"),
    ]
    .into_iter()
    .find(|(first, second)| word(0) == *first && word(1) == *second)
    {
        return Some(format!("{first} {second}"));
    }
    into_file(statement, &tokens).map(|file| format!("INTO {file}"))
}

/// Settings that leave read-only, or that change how later statements are
/// lexed (the tokenizer assumes the connect-time values). MySQL's
/// `completion_type` changes what the cleanup's `ROLLBACK` does: with
/// `RELEASE` it closes the session, with `CHAIN` it starts a transaction.
fn is_guarded_setting(name: &str) -> bool {
    name.ends_with("READ_ONLY")
        || name == "AUTOCOMMIT"
        || name == "COMPLETION_TYPE"
        || name == "SQL_MODE"
        || name == "STANDARD_CONFORMING_STRINGS"
        || name == "CLIENT_ENCODING"
        || name.starts_with("CHARACTER_SET")
}

/// The refusal for a `SET` statement, if any. Everything here errs toward
/// refusing: a guarded word anywhere in the statement counts.
fn set_refusal(
    dialect: Dialect,
    statement: &str,
    tokens: &[Token],
    words: &[String],
) -> Option<String> {
    // MariaDB's SET STATEMENT var=value FOR <statement> wraps any statement
    // behind a leading SET; refuse it outright rather than read inside.
    // Only the token right after SET counts: `SET @statement = 1` has an
    // `@` there.
    let mut code = tokens
        .iter()
        .filter(|token| !matches!(token.kind, TokenKind::Whitespace | TokenKind::Comment));
    if dialect == Dialect::MySql
        && let Some(second) = code.nth(1)
        && matches!(second.kind, TokenKind::Keyword | TokenKind::Identifier)
        && statement[second.range.clone()].eq_ignore_ascii_case("STATEMENT")
    {
        return Some("SET STATEMENT".into());
    }
    let has = |wanted: &str| words.iter().skip(1).any(|word| word == wanted);
    if has("CHARACTERISTICS") {
        return Some("SET SESSION CHARACTERISTICS".into());
    }
    if has("TRANSACTION") {
        return Some("SET TRANSACTION".into());
    }
    // Each assignment on its own: `SET @a = 1, NAMES gbk` sets a charset.
    for assignment in assignments(statement, tokens) {
        let mut rest = assignment.iter().map(String::as_str).peekable();
        while rest
            .next_if(|word| {
                matches!(
                    *word,
                    "GLOBAL" | "SESSION" | "LOCAL" | "PERSIST" | "PERSIST_ONLY"
                )
            })
            .is_some()
        {}
        match (rest.next(), rest.next()) {
            (Some("NAMES"), _) => return Some("SET NAMES".into()),
            (Some("CHARSET"), _) => return Some("SET CHARSET".into()),
            (Some("CHARACTER"), Some("SET")) => return Some("SET CHARACTER SET".into()),
            _ => {}
        }
    }
    if let Some(name) = words.iter().skip(1).find(|name| is_guarded_setting(name)) {
        return Some(format!("SET {name}"));
    }
    // Server state that is neither rolled back nor reset by the cleanup.
    if dialect == Dialect::MySql
        && let Some(scope) = words
            .iter()
            .skip(1)
            .find(|word| matches!(word.as_str(), "GLOBAL" | "PERSIST" | "PERSIST_ONLY"))
    {
        return Some(format!("SET {scope}"));
    }
    None
}

/// The words of each top-level assignment of a `SET` (split on commas
/// outside parentheses), without the leading `SET`. Unbalanced parentheses
/// leave the rest of the statement in one assignment, which fails open, but
/// the server rejects such text as a syntax error, so nothing runs.
fn assignments(statement: &str, tokens: &[Token]) -> Vec<Vec<String>> {
    let mut found = vec![Vec::new()];
    let mut depth = 0_usize;
    for token in tokens {
        match (token.kind, &statement[token.range.clone()]) {
            (TokenKind::Punctuation, "(") => depth += 1,
            (TokenKind::Punctuation, ")") => depth = depth.saturating_sub(1),
            (TokenKind::Punctuation, ",") if depth == 0 => found.push(Vec::new()),
            _ => {
                if let (Some(word), Some(current)) = (word_of(statement, token), found.last_mut()) {
                    current.push(word);
                }
            }
        }
    }
    if let Some(first) = found.first_mut()
        && first.first().is_some_and(|word| word == "SET")
    {
        first.remove(0);
    }
    found
}

/// `OUTFILE` or `DUMPFILE` when the statement has `INTO` straight before it,
/// as the next unquoted word with only whitespace and comments between.
fn into_file(statement: &str, tokens: &[Token]) -> Option<&'static str> {
    let mut previous_into = false;
    for token in tokens {
        let text = &statement[token.range.clone()];
        match token.kind {
            TokenKind::Whitespace | TokenKind::Comment => {}
            TokenKind::Keyword | TokenKind::Identifier => {
                if previous_into && text.eq_ignore_ascii_case("OUTFILE") {
                    return Some("OUTFILE");
                }
                if previous_into && text.eq_ignore_ascii_case("DUMPFILE") {
                    return Some("DUMPFILE");
                }
                previous_into = text.eq_ignore_ascii_case("INTO");
            }
            _ => previous_into = false,
        }
    }
    None
}

/// Whether a PostgreSQL statement names something with `U&"..."`. The
/// tokenizer sees the identifier `U`, the operator `&`, then a quoted name.
fn unicode_name(statement: &str, tokens: &[Token]) -> bool {
    let code: Vec<&Token> = tokens
        .iter()
        .filter(|token| token.kind != TokenKind::Whitespace)
        .collect();
    code.windows(3).any(|three| {
        statement[three[0].range.clone()].eq_ignore_ascii_case("U")
            && &statement[three[1].range.clone()] == "&"
            && three[2].kind == TokenKind::QuotedIdentifier
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kinds(dialect: Dialect, text: &str) -> Vec<(TokenKind, &str)> {
        tokenize(dialect, text)
            .into_iter()
            .filter(|token| token.kind != TokenKind::Whitespace)
            .map(|token| (token.kind, &text[token.range]))
            .collect()
    }

    #[test]
    fn a_statement_does_not_print_its_text() {
        let script = statements(
            Dialect::Postgres,
            "SELECT 1;\nSELECT 'hunter2' FROM secrets",
        );
        let printed = format!("{script:?}");
        assert!(!printed.contains("hunter2"), "{printed}");
        assert!(!printed.contains("secrets"), "{printed}");
        assert!(!printed.contains("SELECT"), "{printed}");
        // Where each statement sits is still there.
        assert!(printed.contains("first_line: 2"), "{printed}");
        assert!(printed.contains("range: 10..39"), "{printed}");
    }

    #[test]
    fn words_numbers_strings_and_punctuation() {
        use TokenKind::*;
        assert_eq!(
            kinds(
                Dialect::Postgres,
                "SELECT a, 1.5e3 FROM t WHERE b = 'x''y';"
            ),
            vec![
                (Keyword, "SELECT"),
                (Identifier, "a"),
                (Punctuation, ","),
                (Number, "1.5e3"),
                (Keyword, "FROM"),
                (Identifier, "t"),
                (Keyword, "WHERE"),
                (Identifier, "b"),
                (Operator, "="),
                (String, "'x''y'"),
                (Semicolon, ";"),
            ]
        );
    }

    #[test]
    fn keywords_are_case_insensitive() {
        assert_eq!(kinds(Dialect::Sqlite, "select")[0].0, TokenKind::Keyword);
    }

    #[test]
    fn postgres_dollar_bodies_and_escape_strings() {
        use TokenKind::*;
        let text = "SELECT $$a;b$$, $fn$ x $$ y $fn$, E'it\\'s;', $1";
        assert_eq!(
            kinds(Dialect::Postgres, text),
            vec![
                (Keyword, "SELECT"),
                (String, "$$a;b$$"),
                (Punctuation, ","),
                (String, "$fn$ x $$ y $fn$"),
                (Punctuation, ","),
                (String, "E'it\\'s;'"),
                (Punctuation, ","),
                (Punctuation, "$1"),
            ]
        );
    }

    #[test]
    fn postgres_block_comments_nest() {
        use TokenKind::*;
        assert_eq!(
            kinds(Dialect::Postgres, "/* a /* b */ c */ SELECT"),
            vec![(Comment, "/* a /* b */ c */"), (Keyword, "SELECT")]
        );
        // SQLite does not nest: the first */ ends it.
        assert_eq!(
            kinds(Dialect::Sqlite, "/* a /* b */ c")[0],
            (Comment, "/* a /* b */")
        );
    }

    #[test]
    fn quoted_identifiers_per_dialect() {
        use TokenKind::*;
        assert_eq!(
            kinds(Dialect::Postgres, r#""we""ird""#),
            vec![(QuotedIdentifier, r#""we""ird""#)]
        );
        assert_eq!(
            kinds(Dialect::MySql, "`a;b`"),
            vec![(QuotedIdentifier, "`a;b`")]
        );
        assert_eq!(
            kinds(Dialect::Sqlite, "[a;b]"),
            vec![(QuotedIdentifier, "[a;b]")]
        );
        // MySQL's default sql_mode reads "..." as a string.
        assert_eq!(
            kinds(Dialect::MySql, r#""a;b""#),
            vec![(String, r#""a;b""#)]
        );
    }

    #[test]
    fn mysql_comments() {
        use TokenKind::*;
        assert_eq!(
            kinds(Dialect::MySql, "SELECT 1--no comment"),
            vec![
                (Keyword, "SELECT"),
                (Number, "1"),
                (Operator, "-"),
                (Operator, "-"),
                (Keyword, "no"),
                (Identifier, "comment"),
            ]
        );
        assert_eq!(
            kinds(Dialect::MySql, "SELECT 1 -- a comment\n# another\n"),
            vec![
                (Keyword, "SELECT"),
                (Number, "1"),
                (Comment, "-- a comment"),
                (Comment, "# another"),
            ]
        );
        assert_eq!(
            kinds(Dialect::MySql, "/*!50000 COMMIT */"),
            vec![(ExecutableComment, "/*!50000 COMMIT */")]
        );
        assert_eq!(
            kinds(Dialect::MySql, r"'it\'s'"),
            vec![(String, r"'it\'s'")]
        );
    }

    #[test]
    fn unterminated_strings_and_comments_run_to_the_end() {
        use TokenKind::*;
        assert_eq!(kinds(Dialect::Postgres, "'abc"), vec![(String, "'abc")]);
        assert_eq!(
            kinds(Dialect::Postgres, "/* abc"),
            vec![(Comment, "/* abc")]
        );
        assert_eq!(kinds(Dialect::Postgres, "$$abc"), vec![(String, "$$abc")]);
    }

    #[test]
    fn non_ascii_text_keeps_char_boundaries() {
        let text = "SELECT 'Zoë 🚀', naïve FROM t";
        for token in tokenize(Dialect::Postgres, text) {
            assert!(text.is_char_boundary(token.range.start));
            assert!(text.is_char_boundary(token.range.end));
        }
        assert!(kinds(Dialect::Postgres, text).contains(&(TokenKind::Identifier, "naïve")));
    }

    #[test]
    fn tokens_cover_the_whole_text() {
        let text = "SELECT a -- x\n/* y */ FROM `t` WHERE c = 'd' ; §";
        for dialect in [Dialect::Postgres, Dialect::MySql, Dialect::Sqlite] {
            let tokens = tokenize(dialect, text);
            let mut at = 0;
            for token in &tokens {
                assert_eq!(token.range.start, at);
                at = token.range.end;
            }
            assert_eq!(at, text.len());
        }
    }

    #[test]
    fn postgres_line_comments_end_at_a_carriage_return() {
        use TokenKind::*;
        assert_eq!(
            kinds(Dialect::Postgres, "--x\rCOMMIT\nAND"),
            vec![(Comment, "--x"), (Keyword, "COMMIT"), (Keyword, "AND")]
        );
        // MySQL and SQLite run a comment to the line feed only.
        assert_eq!(
            kinds(Dialect::Sqlite, "--x\rCOMMIT\nAND"),
            vec![(Comment, "--x\rCOMMIT"), (Keyword, "AND")]
        );
        assert_eq!(
            kinds(Dialect::MySql, "-- x\rCOMMIT\nAND"),
            vec![(Comment, "-- x\rCOMMIT"), (Keyword, "AND")]
        );
        assert_eq!(
            kinds(Dialect::MySql, "#x\rCOMMIT\nAND"),
            vec![(Comment, "#x\rCOMMIT"), (Keyword, "AND")]
        );
    }

    #[test]
    fn mariadb_executable_comments() {
        use TokenKind::*;
        let tokens = kinds(Dialect::MySql, "/*M! COMMIT */ SELECT 1");
        assert_eq!(tokens[0], (ExecutableComment, "/*M! COMMIT */"));
        assert_eq!(tokens[1], (Keyword, "SELECT"));
        // Only MySQL's dialect runs them; elsewhere they are comments.
        assert_eq!(
            kinds(Dialect::Postgres, "/*M! COMMIT */")[0],
            (Comment, "/*M! COMMIT */")
        );
        assert_eq!(
            kinds(Dialect::MySql, "/*M COMMIT */")[0],
            (Comment, "/*M COMMIT */")
        );
    }

    #[test]
    fn postgres_escape_strings_continue_over_newlines() {
        use TokenKind::*;
        assert_eq!(
            kinds(Dialect::Postgres, "SELECT E'a'\n'\\''; COMMIT; --'"),
            vec![
                (Keyword, "SELECT"),
                (String, "E'a'\n'\\''"),
                (Semicolon, ";"),
                (Keyword, "COMMIT"),
                (Semicolon, ";"),
                (Comment, "--'"),
            ]
        );
        // Blanks and a comment before the newline, comment lines and blank
        // lines after it, and several pieces.
        let text = "E'a' -- c\r\n  -- d\n\n 'b\\'' \n'c'";
        assert_eq!(kinds(Dialect::Postgres, text), vec![(String, text)]);
        // A vertical tab may precede the newline too.
        assert_eq!(
            kinds(Dialect::Postgres, "E'a'\x0b\n'\\''x"),
            vec![(String, "E'a'\x0b\n'\\''"), (Identifier, "x")]
        );
        // No newline, no continuation.
        assert_eq!(
            kinds(Dialect::Postgres, "E'a' 'b'"),
            vec![(String, "E'a'"), (String, "'b'")]
        );
        // A comment after the newline must end in a newline itself.
        assert_eq!(
            kinds(Dialect::Postgres, "E'a'\n-- x"),
            vec![(String, "E'a'"), (Comment, "-- x")]
        );
        // A non-quote ends it.
        assert_eq!(
            kinds(Dialect::Postgres, "E'a'\nSELECT"),
            vec![(String, "E'a'"), (Keyword, "SELECT")]
        );
        // Plain strings stay separate tokens.
        assert_eq!(
            kinds(Dialect::Postgres, "'a'\n'b'"),
            vec![(String, "'a'"), (String, "'b'")]
        );
        // MySQL has no continuation.
        assert_eq!(
            kinds(Dialect::MySql, "'a'\n'b'"),
            vec![(String, "'a'"), (String, "'b'")]
        );
    }

    #[test]
    fn postgres_dollar_tags_may_hold_non_ascii_letters() {
        use TokenKind::*;
        assert_eq!(
            kinds(Dialect::Postgres, "SELECT $é$ ' $é$; COMMIT"),
            vec![
                (Keyword, "SELECT"),
                (String, "$é$ ' $é$"),
                (Semicolon, ";"),
                (Keyword, "COMMIT"),
            ]
        );
        // A dollar sign inside a word belongs to the word.
        assert_eq!(
            kinds(Dialect::Postgres, "a$$b$$"),
            vec![(Identifier, "a$$b$$")]
        );
    }

    #[test]
    fn a_vertical_tab_is_whitespace_everywhere() {
        for dialect in [Dialect::Postgres, Dialect::MySql, Dialect::Sqlite] {
            assert_eq!(
                kinds(dialect, "\x0bCOMMIT"),
                vec![(TokenKind::Keyword, "COMMIT")]
            );
        }
    }

    #[test]
    fn a_trailing_backslash_is_part_of_the_string() {
        assert_eq!(
            kinds(Dialect::MySql, "'abc\\"),
            vec![(TokenKind::String, "'abc\\")]
        );
        assert_eq!(
            kinds(Dialect::Postgres, "E'abc\\"),
            vec![(TokenKind::String, "E'abc\\")]
        );
    }

    #[test]
    fn edges_of_comments_brackets_and_numbers() {
        use TokenKind::*;
        assert_eq!(kinds(Dialect::MySql, "--"), vec![(Comment, "--")]);
        assert_eq!(
            kinds(Dialect::Sqlite, "[a]b]"),
            vec![
                (QuotedIdentifier, "[a]"),
                (Identifier, "b"),
                (Punctuation, "]")
            ]
        );
        assert_eq!(
            kinds(Dialect::Postgres, "/* a /* b */ c"),
            vec![(Comment, "/* a /* b */ c")]
        );
        assert_eq!(kinds(Dialect::Postgres, "1."), vec![(Number, "1.")]);
    }

    #[test]
    fn every_short_text_is_covered_without_panics() {
        let alphabet = [
            "'", "\"", "\\", "$", "/", "*", "-", "#", "[", "]", "`", "E", "é", "\n", "\r", "!",
            "1", ".", "e", "\x0b",
        ];
        let mut texts = vec![String::new()];
        let mut level = vec![String::new()];
        for _ in 0..4 {
            level = level
                .iter()
                .flat_map(|prefix| alphabet.iter().map(move |piece| format!("{prefix}{piece}")))
                .collect();
            texts.extend(level.iter().cloned());
        }
        for dialect in [Dialect::Postgres, Dialect::MySql, Dialect::Sqlite] {
            for text in &texts {
                let mut at = 0;
                for token in tokenize(dialect, text) {
                    assert_eq!(token.range.start, at, "{dialect:?} {text:?}");
                    assert!(token.range.end > at, "{dialect:?} {text:?}");
                    assert!(text.is_char_boundary(token.range.end));
                    at = token.range.end;
                }
                assert_eq!(at, text.len(), "{dialect:?} {text:?}");
            }
        }
    }

    fn texts(dialect: Dialect, text: &str) -> Vec<String> {
        statements(dialect, text)
            .into_iter()
            .map(|s| s.text)
            .collect()
    }

    #[test]
    fn splits_on_semicolons_outside_strings_and_bodies() {
        let script = "SELECT ';'; SELECT $$a;b$$;\n-- only a comment;\nSELECT 3";
        assert_eq!(
            texts(Dialect::Postgres, script),
            // A leading comment belongs to the statement after it (its `;`
            // is inside the comment).
            vec![
                "SELECT ';'",
                "SELECT $$a;b$$",
                "-- only a comment;\nSELECT 3"
            ]
        );
        assert_eq!(
            texts(Dialect::MySql, "SELECT `a;b`; SELECT \"c;d\""),
            vec!["SELECT `a;b`", "SELECT \"c;d\""]
        );
    }

    #[test]
    fn empty_and_comment_only_pieces_are_dropped() {
        assert!(statements(Dialect::Sqlite, " ;; -- nothing\n/* x */ ;").is_empty());
    }

    #[test]
    fn statements_come_from_tokens_made_once() {
        for dialect in [Dialect::Postgres, Dialect::MySql, Dialect::Sqlite] {
            for script in [
                "",
                "SELECT 1",
                "-- note\nSELECT 'a;b';\r\n  SELECT 2 ;\n/* done */",
                "/*! COMMIT */; SELECT `x`",
            ] {
                let tokens = tokenize(dialect, script);
                assert_eq!(
                    statements_from(script, &tokens),
                    statements(dialect, script),
                    "{dialect:?}"
                );
            }
        }
    }

    #[test]
    fn an_empty_script_has_no_statements() {
        assert!(statements(Dialect::Postgres, "").is_empty());
        assert!(statements(Dialect::Postgres, "  \n\t").is_empty());
    }

    #[test]
    fn first_line_skips_leading_comments() {
        let script = "SELECT 1;\n\n-- counts\nSELECT\n  2;";
        let found = statements(Dialect::Postgres, script);
        assert_eq!(found[0].first_line, 1);
        assert_eq!(found[1].start_line, 3);
        assert_eq!(found[1].first_line, 4);
        assert_eq!(found[1].text, "-- counts\nSELECT\n  2");
    }

    #[test]
    fn many_statements_are_numbered_by_line() {
        let script = "SELECT 1;\n".repeat(5000);
        let found = statements(Dialect::Postgres, &script);
        assert_eq!(found.len(), 5000);
        assert_eq!(found[4999].first_line, 5000);
        assert_eq!(found[4999].start_line, 5000);
    }

    #[test]
    fn executable_comments_are_code() {
        let found = statements(Dialect::MySql, "/*! COMMIT */; SELECT 1");
        assert_eq!(found.len(), 2);
        assert_eq!(found[0].text, "/*! COMMIT */");
        let found = statements(Dialect::MySql, "/*M! SET x = 1 */");
        assert_eq!(found.len(), 1);
        let same_line = statements(Dialect::MySql, "-- note\n/*! COMMIT */ SELECT 1");
        assert_eq!(same_line[0].first_line, 2);
        let next_line = statements(Dialect::MySql, "-- note\n/*! COMMIT */\nSELECT 1");
        assert_eq!(next_line[0].first_line, 2);
    }

    #[test]
    fn end_is_past_the_semicolon_or_the_text() {
        let found = statements(Dialect::Postgres, "SELECT 1 ;  SELECT 3\n\n");
        assert_eq!((found[0].range.end, found[0].end), (8, 10));
        assert_eq!((found[1].range.end, found[1].end), (20, 20));
    }

    #[test]
    fn the_statement_at_the_cursor() {
        let script = "SELECT 1;  SELECT 2;\n\nSELECT 3";
        let found = statements(Dialect::Postgres, script);
        let at = |cursor| statement_at(&found, cursor).map(|s| s.text.as_str());
        assert_eq!(at(0), Some("SELECT 1"));
        assert_eq!(at(8), Some("SELECT 1")); // just before the ;
        assert_eq!(at(9), Some("SELECT 1")); // just after it
        assert_eq!(at(10), Some("SELECT 1")); // between: the one before
        assert_eq!(at(14), Some("SELECT 2"));
        assert_eq!(at(21), Some("SELECT 2")); // blank line after it
        assert_eq!(at(script.len()), Some("SELECT 3"));
        assert_eq!(at(script.len() + 50), Some("SELECT 3")); // past the end
        // Adjacent statements: just after a `;` is still the one before it.
        let adjacent = statements(Dialect::Postgres, "SELECT 1;SELECT 2");
        let tie = |cursor| statement_at(&adjacent, cursor).map(|s| s.text.as_str());
        assert_eq!(tie(9), Some("SELECT 1"));
        assert_eq!(tie(10), Some("SELECT 2"));
        let leading = statements(Dialect::Postgres, "\n\n  SELECT 1");
        assert_eq!(statement_at(&leading, 0).map(|s| s.first_line), Some(3));
        assert!(statement_at(&[], 0).is_none());
    }

    #[test]
    fn a_position_maps_to_a_line_and_column() {
        let script = "SELECT 0;\nSELECT a,\n  bogus FROM t";
        let second = &statements(Dialect::Postgres, script)[1];
        // PostgreSQL positions are 1-based characters in the statement.
        let position = second.text.find("bogus").unwrap() + 1;
        assert_eq!(second.line_col(position), (3, 3));
        assert_eq!(second.line_col(1), (2, 1));
    }

    #[test]
    fn line_col_counts_characters_and_the_statement_offset() {
        // The statement starts after "é; " on its line: 3 characters.
        let script = "é; SELECT 'ü', zz";
        let second = &statements(Dialect::Postgres, script)[1];
        assert_eq!(second.start_column, 3);
        let position = second.text.chars().position(|c| c == 'z').unwrap() + 1;
        assert_eq!(second.line_col(position), (1, 3 + position));
        // A position of 0 or past the end clamps instead of panicking.
        assert_eq!(second.line_col(0), (1, 4));
        assert_eq!(second.line_col(1000), (1, 4 + second.text.chars().count()));
    }

    #[test]
    fn crlf_line_endings_keep_lines_and_columns() {
        let script = "SELECT 1;\r\nSELECT a,\r\n  bogus";
        let found = statements(Dialect::Postgres, script);
        assert_eq!(found[0].text, "SELECT 1");
        assert_eq!(found[1].start_line, 2);
        assert_eq!(found[1].text, "SELECT a,\r\n  bogus");
        let position = found[1].text.find("bogus").unwrap() + 1;
        assert_eq!(found[1].line_col(position), (3, 3));
    }

    #[test]
    fn an_unterminated_quoted_name_does_not_panic() {
        assert_eq!(
            words(Dialect::Postgres, "SELECT \"Zoë"),
            vec!["SELECT", "ZOë"]
        );
        assert_eq!(words(Dialect::Sqlite, "[Zoë"), vec!["ZOë"]);
    }

    #[test]
    fn words_skip_comments_and_upper_case() {
        assert_eq!(
            words(Dialect::MySql, "/* x */ set @@session.`tx_read_only` = 0"),
            vec!["SET", "SESSION", "TX_READ_ONLY"]
        );
    }

    #[test]
    fn transaction_and_session_statements_are_refused() {
        for text in [
            "BEGIN",
            "start transaction",
            "COMMIT",
            "end",
            "ROLLBACK",
            "abort",
            "SAVEPOINT a",
            "RELEASE a",
            "PREPARE TRANSACTION 'x'",
            "COMMIT PREPARED 'x'",
            "SET TRANSACTION READ WRITE",
            "SET SESSION CHARACTERISTICS AS TRANSACTION READ WRITE",
            "set default_transaction_read_only = off",
            r#"SET "default_transaction_read_only" = off"#,
            "RESET ALL",
            "reset default_transaction_read_only",
            "SET standard_conforming_strings = off",
            r#"SET U&"default_transaction_read\005fonly" = off"#,
            "SELECT set_config('standard_conforming_strings', 'off', false)",
            "SET client_encoding = 'SJIS'",
            "DISCARD ALL",
            "COPY users TO STDOUT",
            "/* hi */ commit",
        ] {
            assert!(refusal(Dialect::Postgres, text).is_some(), "{text}");
        }
        for text in [
            "SET SESSION TRANSACTION READ WRITE",
            "SET GLOBAL TRANSACTION READ WRITE",
            "SET @@session.transaction_read_only = 0",
            "set `tx_read_only` = 0",
            "SET autocommit = 1",
            "XA START 'x'",
            "LOCK TABLES t READ",
            "UNLOCK TABLES",
            "CALL p()",
            "CREATE USER x",
            "ALTER USER x IDENTIFIED BY 'y'",
            "DROP USER x",
            "RENAME USER x TO y",
            "CREATE ROLE r",
            "DROP ROLE r",
            "GRANT ALL ON *.* TO x",
            "REVOKE ALL ON *.* FROM x",
            "SET PASSWORD = 'x'",
            "SET DEFAULT ROLE ALL TO x",
            "FLUSH PRIVILEGES",
            "INSTALL PLUGIN p SONAME 'p.so'",
            "UNINSTALL PLUGIN p",
            "SELECT * FROM t INTO OUTFILE '/tmp/x'",
            "SELECT 1 INTO DUMPFILE '/tmp/x'",
            "SELECT 1 /*! , 2 */",
            "SELECT 1 /*M! , 2 */",
            "SET sql_mode = 'ANSI_QUOTES'",
            "SET @@session.sql_mode = ''",
            "SET NAMES gbk",
            "SET CHARACTER SET gbk",
            "SET character_set_client = gbk",
            "RESET sql_mode",
        ] {
            assert!(refusal(Dialect::MySql, text).is_some(), "{text}");
        }
        assert!(refusal(Dialect::Sqlite, "BEGIN IMMEDIATE").is_some());
    }

    #[test]
    fn unterminated_names_are_not_refused_or_a_panic() {
        assert_eq!(refusal(Dialect::MySql, "SELECT `ë"), None);
    }

    #[test]
    fn look_alikes_are_allowed() {
        for (dialect, text) in [
            (Dialect::Postgres, "SELECT 'COMMIT'"),
            (Dialect::Postgres, "SELECT end_date FROM t"),
            (Dialect::Postgres, "SET search_path = public"),
            (Dialect::Postgres, "SELECT 1 -- COMMIT"),
            (Dialect::Postgres, "WITH x AS (SELECT 1) SELECT * FROM x"),
            (Dialect::MySql, r#"SELECT "into outfile""#),
            (Dialect::MySql, "SET time_zone = '+00:00'"),
            (Dialect::MySql, "SELECT * FROM users"),
            (Dialect::Sqlite, "PRAGMA table_info(users)"),
            (Dialect::Sqlite, "SELECT * FROM `call`"),
        ] {
            assert_eq!(refusal(dialect, text), None, "{text}");
        }
    }

    #[test]
    fn more_spellings_of_the_same_statements_are_refused() {
        for text in [
            "ROLLBACK PREPARED 'x'",
            "SET SESSION TRANSACTION READ WRITE",
            "SET LOCAL transaction_read_only = off",
            "RESET character_set_client",
            "RESET client_encoding",
            "RESET \"standard_conforming_strings\"",
            "  -- note\n  END",
            "set U&\"x\" = 1",
        ] {
            assert!(refusal(Dialect::Postgres, text).is_some(), "{text}");
        }
        for text in [
            "SET CHARSET utf8mb4",
            "SET @@global.sql_mode = ''",
            "SET SESSION sql_mode = ''",
            "SET @a = 1, @@autocommit = 1",
            "set `character_set_results` = NULL",
            "COMMIT",
            "SELECT 1 INTO\nOUTFILE '/tmp/x'",
            "SELECT 1 INTO /* x */ DUMPFILE '/tmp/x'",
        ] {
            assert!(refusal(Dialect::MySql, text).is_some(), "{text}");
        }
    }

    #[test]
    fn the_refusal_names_the_statement() {
        assert_eq!(
            refusal(Dialect::Postgres, "commit").as_deref(),
            Some("COMMIT")
        );
        assert_eq!(
            refusal(Dialect::MySql, "SELECT 1 INTO OUTFILE 'x'").as_deref(),
            Some("INTO OUTFILE")
        );
        assert_eq!(
            refusal(Dialect::Postgres, "SET client_encoding = 'x'").as_deref(),
            Some("SET CLIENT_ENCODING")
        );
    }

    #[test]
    fn dialect_specific_refusals_stay_in_their_dialect() {
        // `/*!` is an ordinary comment outside MySQL; COPY and CALL are
        // ordinary names or other statements elsewhere.
        assert_eq!(refusal(Dialect::Postgres, "SELECT 1 /*! , 2 */"), None);
        assert_eq!(refusal(Dialect::Sqlite, "SELECT 1 /*! , 2 */"), None);
        assert_eq!(refusal(Dialect::MySql, "SELECT set_config FROM t"), None);
        assert_eq!(refusal(Dialect::Sqlite, "SELECT * FROM copy"), None);
        assert_eq!(
            refusal(Dialect::MySql, "SELECT 'x' AS `into`, outfile FROM t"),
            None
        );
    }

    #[test]
    fn odd_text_never_panics_the_guard() {
        for dialect in [Dialect::Postgres, Dialect::MySql, Dialect::Sqlite] {
            for text in [
                "",
                "U",
                "U&",
                "U&\"",
                "SET U&\"ë",
                "set `ë",
                "/*!",
                "/*M!",
                "SET",
                "RESET",
                "SET ë",
                "'ë",
                "$$ë",
                "[ë",
                "SELECT 1 INTO",
            ] {
                let _ = refusal(dialect, text);
            }
        }
    }

    #[test]
    fn every_assignment_of_a_set_is_checked() {
        for (dialect, text) in [
            (Dialect::MySql, "SET @a = 1, NAMES gbk"),
            (
                Dialect::MySql,
                "SET time_zone = '+00:00', CHARACTER SET gbk",
            ),
            (Dialect::MySql, "SET @a = 1, CHARSET gbk"),
            (Dialect::Postgres, "SET SESSION NAMES 'SJIS'"),
            (Dialect::Postgres, "SET LOCAL NAMES 'SJIS'"),
            (Dialect::MySql, "SET @a = (1, 2), GLOBAL NAMES gbk"),
        ] {
            assert!(refusal(dialect, text).is_some(), "{text}");
        }
        assert_eq!(
            refusal(Dialect::MySql, "SET @a = 1, CHARSET gbk").as_deref(),
            Some("SET CHARSET")
        );
        assert_eq!(
            refusal(Dialect::MySql, "SET CHARACTER SET gbk").as_deref(),
            Some("SET CHARACTER SET")
        );
        assert_eq!(
            refusal(Dialect::MySql, "SET NAMES gbk").as_deref(),
            Some("SET NAMES")
        );
        // Commas inside parentheses do not start an assignment.
        assert_eq!(refusal(Dialect::MySql, "SET @a = f(1, 2), @b = 3"), None);
    }

    #[test]
    fn a_u_and_name_is_refused_in_any_postgres_statement() {
        for text in [
            r#"SELECT U&"set\005fconfig"('x', 'y', false)"#,
            r#"SELECT u&"a" FROM t"#,
            r#"SET U&"x" = 1"#,
            r#"RESET U&"x""#,
        ] {
            assert_eq!(
                refusal(Dialect::Postgres, text).as_deref(),
                Some("a U& name"),
                "{text}"
            );
        }
        assert_eq!(
            refusal(Dialect::Postgres, r"SELECT U&'d\0061t\+000061'"),
            None
        );
        assert_eq!(refusal(Dialect::Postgres, "SELECT a & b FROM t"), None);
    }

    #[test]
    fn create_or_replace_user_and_role_are_refused() {
        for text in ["CREATE OR REPLACE USER x", "CREATE OR REPLACE ROLE r"] {
            assert!(refusal(Dialect::MySql, text).is_some(), "{text}");
        }
        assert_eq!(
            refusal(Dialect::MySql, "create or replace user x").as_deref(),
            Some("CREATE USER")
        );
        assert_eq!(
            refusal(Dialect::MySql, "CREATE OR REPLACE VIEW v AS SELECT 1"),
            None
        );
    }

    #[test]
    fn prepared_sql_and_server_state_are_refused() {
        for (dialect, text) in [
            (Dialect::Postgres, "PREPARE s AS SELECT 1"),
            (Dialect::Postgres, "EXECUTE s"),
            (Dialect::Postgres, "DEALLOCATE ALL"),
            (Dialect::MySql, "PREPARE s FROM 'SELECT 1'"),
            (Dialect::MySql, "EXECUTE s"),
            (Dialect::MySql, "EXECUTE IMMEDIATE 'SELECT 1'"),
            (Dialect::MySql, "DEALLOCATE PREPARE s"),
            (Dialect::MySql, "SET PERSIST max_connections = 1"),
            (Dialect::MySql, "SET PERSIST_ONLY max_connections = 1"),
            (Dialect::MySql, "SET GLOBAL general_log_file = 'x'"),
            (Dialect::MySql, "SET @@global.x = 1"),
            (Dialect::MySql, "SET @@persist.x = 1"),
            (Dialect::MySql, "RESET MASTER"),
            (Dialect::MySql, "RESET PERSIST"),
            (Dialect::MySql, "RESET REPLICA"),
            (Dialect::MySql, "PURGE BINARY LOGS TO 'x'"),
            (
                Dialect::MySql,
                "CHANGE REPLICATION SOURCE TO SOURCE_HOST='x'",
            ),
            (Dialect::MySql, "CHANGE MASTER TO MASTER_HOST='x'"),
            (Dialect::MySql, "STOP REPLICA"),
            (Dialect::MySql, "SHUTDOWN"),
            (Dialect::MySql, "RESTART"),
            (Dialect::MySql, "CLONE LOCAL DATA DIRECTORY = '/tmp/x'"),
        ] {
            assert!(refusal(dialect, text).is_some(), "{text}");
        }
        // KILL stays allowed: an accepted side effect.
        assert_eq!(refusal(Dialect::MySql, "KILL QUERY 4"), None);
        // PostgreSQL's own PREPARE TRANSACTION keeps its name.
        assert_eq!(
            refusal(Dialect::Postgres, "PREPARE TRANSACTION 'x'").as_deref(),
            Some("PREPARE TRANSACTION")
        );
        // MySQL's SET GLOBAL is MySQL's; PostgreSQL has no such scope.
        assert_eq!(refusal(Dialect::Postgres, "SET search_path = global"), None);
    }

    #[test]
    fn refusal_messages_name_what_was_refused() {
        for (dialect, text, what) in [
            (
                Dialect::Postgres,
                "SET SESSION CHARACTERISTICS AS TRANSACTION READ WRITE",
                "SET SESSION CHARACTERISTICS",
            ),
            (Dialect::MySql, "RESET sql_mode", "RESET SQL_MODE"),
            (Dialect::Postgres, "RESET sql_mode", "RESET SQL_MODE"),
            (Dialect::Postgres, "RESET ALL", "RESET ALL"),
            (Dialect::Postgres, "PREPARE s AS SELECT 1", "PREPARE"),
            (Dialect::MySql, "SET GLOBAL x = 1", "SET GLOBAL"),
            (Dialect::MySql, "RESET MASTER", "RESET MASTER"),
        ] {
            assert_eq!(refusal(dialect, text).as_deref(), Some(what), "{text}");
        }
    }

    #[test]
    fn set_statement_wraps_nothing_past_the_guard() {
        for text in [
            "SET STATEMENT max_statement_time=0 FOR COMMIT",
            "SET STATEMENT max_statement_time=0 FOR ROLLBACK",
            "SET STATEMENT max_statement_time=0 FOR GRANT ALL ON *.* TO x",
            "SET STATEMENT max_statement_time=0 FOR CALL p()",
            "SET STATEMENT max_statement_time=0 FOR EXECUTE IMMEDIATE 'COMMIT'",
            "set /* x */ statement a=1 for commit",
        ] {
            assert_eq!(
                refusal(Dialect::MySql, text).as_deref(),
                Some("SET STATEMENT"),
                "{text}"
            );
        }
        // Only the token right after SET counts.
        assert_eq!(refusal(Dialect::MySql, "SET @statement = 1"), None);
        assert_eq!(refusal(Dialect::MySql, "SET time_zone = 'statement'"), None);
    }

    #[test]
    fn the_last_mysql_server_statements_are_refused() {
        for (text, what) in [
            ("DROP PREPARE s", "DROP PREPARE"),
            ("ALTER INSTANCE ROTATE INNODB MASTER KEY", "ALTER INSTANCE"),
            ("BACKUP STAGE START", "BACKUP"),
            ("BACKUP LOCK db.t", "BACKUP"),
        ] {
            assert_eq!(
                refusal(Dialect::MySql, text).as_deref(),
                Some(what),
                "{text}"
            );
        }
        assert_eq!(refusal(Dialect::MySql, "SELECT backup FROM t"), None);
        assert_eq!(refusal(Dialect::Postgres, "DROP PREPARE s"), None);
    }

    #[test]
    fn completion_type_is_a_guarded_setting() {
        for text in [
            "SET SESSION completion_type = 2",
            "SET completion_type = RELEASE",
            "set @@session.`completion_type` = 'CHAIN'",
            "SET @a = 1, @@completion_type = 1",
            "SET LOCAL completion_type = DEFAULT",
        ] {
            assert_eq!(
                refusal(Dialect::MySql, text).as_deref(),
                Some("SET COMPLETION_TYPE"),
                "{text}"
            );
        }
        // Reading it, or a name like it, is fine.
        for text in [
            "SELECT @@session.completion_type",
            "SET @completion = 'completion_type'",
            "SHOW VARIABLES LIKE 'completion_type'",
        ] {
            assert_eq!(refusal(Dialect::MySql, text), None, "{text}");
        }
    }

    #[test]
    fn use_is_refused_on_mysql() {
        for text in ["USE other", "use `other`", "-- note\n  Use other", "USE"] {
            assert_eq!(
                refusal(Dialect::MySql, text).as_deref(),
                Some("USE"),
                "{text}"
            );
        }
        // A column or a table of that name is a name.
        for text in [
            "SELECT `use` FROM t",
            "SELECT * FROM `use`",
            "SELECT t.`use`, 'USE other' FROM `use` AS t",
            "SELECT * FROM t USE INDEX (i)",
            "EXPLAIN SELECT `use` FROM `use`",
        ] {
            assert_eq!(refusal(Dialect::MySql, text), None, "{text}");
        }
        // Not a statement of the others.
        assert_eq!(refusal(Dialect::Postgres, "USE other"), None);
        assert_eq!(refusal(Dialect::Sqlite, "USE other"), None);
    }

    #[test]
    fn the_keywords_of_a_dialect_are_the_ones_it_highlights() {
        let mysql: Vec<&str> = keywords(Dialect::MySql).collect();
        assert!(mysql.contains(&"SELECT") && mysql.contains(&"REGEXP"));
        assert!(!mysql.contains(&"ILIKE"));
        // Shared words, each dialect's own, and no word twice.
        for (dialect, own) in [
            (Dialect::Postgres, "ILIKE"),
            (Dialect::MySql, "REGEXP"),
            (Dialect::Sqlite, "PRAGMA"),
        ] {
            let words: Vec<&str> = keywords(dialect).collect();
            assert!(
                words.contains(&"WHERE") && words.contains(&own),
                "{dialect:?}"
            );
            for word in &words {
                assert_eq!(*word, word.to_ascii_uppercase());
                assert_eq!(
                    words.iter().filter(|other| *other == word).count(),
                    1,
                    "{dialect:?}: {word}"
                );
            }
        }
        // Each list is kept in alphabetical order.
        for words in [KEYWORDS, POSTGRES_KEYWORDS, MYSQL_KEYWORDS, SQLITE_KEYWORDS] {
            assert!(words.is_sorted(), "{words:?}");
        }
    }

    #[test]
    fn the_words_that_end_a_clause_are_keywords() {
        // A whole word the list does not know is completed to a name that
        // only starts like it: `NULLS LAST` to `last_name`.
        let words = [
            "FOLLOWING",
            "LAST",
            "LOCKED",
            "NOTHING",
            "NOWAIT",
            "NULLS",
            "PRECEDING",
            "ROW",
            "SKIP",
            "UNBOUNDED",
        ];
        for dialect in [Dialect::Postgres, Dialect::MySql, Dialect::Sqlite] {
            for word in words {
                assert!(is_keyword(dialect, word), "{dialect:?}: {word}");
                assert!(
                    is_keyword(dialect, &word.to_ascii_lowercase()),
                    "{dialect:?}: {word}"
                );
            }
        }
        use TokenKind::{Identifier, Keyword};
        assert_eq!(
            kinds(
                Dialect::Postgres,
                "ORDER BY created_at DESC NULLS LAST FOR UPDATE SKIP LOCKED"
            ),
            vec![
                (Keyword, "ORDER"),
                (Keyword, "BY"),
                (Identifier, "created_at"),
                (Keyword, "DESC"),
                (Keyword, "NULLS"),
                (Keyword, "LAST"),
                (Keyword, "FOR"),
                (Keyword, "UPDATE"),
                (Keyword, "SKIP"),
                (Keyword, "LOCKED"),
            ]
        );
        // A name that only starts like one is still a name.
        assert_eq!(
            kinds(Dialect::Postgres, "last_name locked_at rows_read"),
            vec![
                (Identifier, "last_name"),
                (Identifier, "locked_at"),
                (Identifier, "rows_read"),
            ]
        );
    }
}

//! SQL text as tokens, for highlighting, for splitting a script into
//! statements, and for the read-only guard. Tokenizing never fails; it only
//! has to agree with the database on where strings, comments and statements
//! end.

use std::ops::Range;

use crate::Dialect;

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
    "LEFT",
    "LIKE",
    "LIMIT",
    "NATURAL",
    "NO",
    "NOT",
    "NULL",
    "OFFSET",
    "ON",
    "ONLY",
    "OR",
    "ORDER",
    "OUTER",
    "OVER",
    "PARTITION",
    "RECURSIVE",
    "RIGHT",
    "ROLLBACK",
    "ROWS",
    "SELECT",
    "SET",
    "SHOW",
    "TABLE",
    "THEN",
    "TO",
    "TRUE",
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

/// Whether `word` (any case) is highlighted as a keyword in `dialect`.
pub fn is_keyword(dialect: Dialect, word: &str) -> bool {
    let own = match dialect {
        Dialect::Postgres => POSTGRES_KEYWORDS,
        Dialect::MySql => MYSQL_KEYWORDS,
        Dialect::Sqlite => SQLITE_KEYWORDS,
    };
    KEYWORDS
        .iter()
        .chain(own)
        .any(|keyword| keyword.eq_ignore_ascii_case(word))
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

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;

    fn kinds(dialect: Dialect, text: &str) -> Vec<(TokenKind, &str)> {
        tokenize(dialect, text)
            .into_iter()
            .filter(|token| token.kind != TokenKind::Whitespace)
            .map(|token| (token.kind, &text[token.range]))
            .collect()
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
}

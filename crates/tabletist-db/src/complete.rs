//! Where a completion would go in a SQL script and what belongs there,
//! read off the script's tokens. There is no parser: the scan is
//! approximate, and it never fails.

use std::ops::Range;

use crate::sql::{Token, TokenKind, is_code};

/// Where a completion would go, and what belongs there.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Site {
    /// The word at the cursor: the bytes a completion replaces. Empty at
    /// a fresh position (after a dot or a space, at the start of a word).
    pub word: Range<usize>,
    /// The names before the word, joined to it by dots, quotes removed:
    /// `["public"]` in `public.bo`, `["b"]` in `b.ti`.
    pub qualifier: Vec<String>,
    /// What belongs at the word when it has no qualifier.
    pub expects: Expects,
    /// The tables the statement names, in order.
    pub sources: Vec<Source>,
    /// The names of the statement's common table expressions.
    pub ctes: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Expects {
    /// The first word of a statement: keywords only.
    Start,
    /// After FROM, JOIN, STRAIGHT_JOIN, UPDATE, INTO or TABLE, or a comma in
    /// a FROM list: schemas, tables, views and CTE names. Only a FROM and
    /// an UPDATE that take a table count. A FROM does in a statement that
    /// has had its SELECT, DELETE, UPDATE or SHOW, unless it is the one of
    /// `IS DISTINCT FROM` or the first one inside the parentheses of
    /// EXTRACT, TRIM, SUBSTRING or OVERLAY. An UPDATE does unless it
    /// follows FOR, KEY or DO.
    Tables,
    /// After a table name in FROM or JOIN, or after AS: most likely an
    /// alias or a new name.
    Name,
    /// Anywhere else: columns of the sources, then keywords.
    Columns,
}

/// A table a statement names.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Source {
    pub schema: Option<String>,
    pub name: String,
    pub alias: Option<String>,
}

/// The keywords a source's name follows wherever they stand. FROM and
/// UPDATE are followed by one in some places only: `table_words` tells.
const SOURCE_WORDS: [&str; 3] = ["JOIN", "STRAIGHT_JOIN", "INTO"];

/// A table's name follows TABLE as well, but that table is no source: in
/// `CREATE TABLE` and `DROP TABLE` it is what the statement makes or
/// removes, not a table whose columns the statement reads.
const TABLE: &str = "TABLE";

/// Keyword phrases the editor offers as one item.
pub const PHRASES: [&str; 14] = [
    "GROUP BY",
    "ORDER BY",
    "PARTITION BY",
    "LEFT JOIN",
    "RIGHT JOIN",
    "INNER JOIN",
    "CROSS JOIN",
    "FULL JOIN",
    "IS NULL",
    "IS NOT NULL",
    "NOT IN",
    "NOT LIKE",
    "NOT EXISTS",
    "UNION ALL",
];

/// A token the server reads: not white space, not a comment.
struct Piece<'a> {
    kind: TokenKind,
    text: &'a str,
    range: Range<usize>,
}

impl Piece<'_> {
    fn is(&self, text: &str) -> bool {
        self.text == text
    }

    /// Whether it spells one of `words`, in any case, whatever its kind.
    fn is_word(&self, words: &[&str]) -> bool {
        words
            .iter()
            .any(|word| word.eq_ignore_ascii_case(self.text))
    }

    fn is_keyword(&self, words: &[&str]) -> bool {
        self.kind == TokenKind::Keyword && self.is_word(words)
    }

    /// A name a user gave: not a keyword. A source and an alias must be
    /// one, because the word after FROM or after a table is most often a
    /// real keyword (`FROM users WHERE`, `JOIN LATERAL`). So a table named
    /// like a highlighted keyword (`first`) is not found as a source unless
    /// it is quoted: a known limit. Only `qualifier` takes a keyword too.
    fn is_name(&self) -> bool {
        matches!(
            self.kind,
            TokenKind::Identifier | TokenKind::QuotedIdentifier
        )
    }

    /// The name it spells, without its quotes.
    fn name(&self) -> String {
        if self.kind != TokenKind::QuotedIdentifier {
            return self.text.to_owned();
        }
        let mut chars = self.text.chars();
        let Some(open) = chars.next() else {
            return String::new();
        };
        let close = if open == '[' { ']' } else { open };
        // An unterminated name has no closing quote to strip.
        let inner = chars.as_str();
        let inner = inner.strip_suffix(close).unwrap_or(inner);
        if open == '[' {
            inner.to_owned()
        } else {
            inner.replace(&format!("{open}{open}"), &open.to_string())
        }
    }
}

/// The site at byte `cursor` of `text`, whose tokens are `tokens`. `None`
/// where nothing is completed: in a string, a comment, a number or a quoted
/// name, and after a dot that follows no name.
pub fn site(tokens: &[Token], text: &str, cursor: usize) -> Option<Site> {
    // The statement: from the last `;` before the cursor to the first one
    // at or after it.
    let first = tokens
        .iter()
        .rposition(|token| token.kind == TokenKind::Semicolon && token.range.end <= cursor)
        .map_or(0, |index| index + 1);
    let last = tokens[first..]
        .iter()
        .position(|token| token.kind == TokenKind::Semicolon)
        .map_or(tokens.len(), |index| first + index);
    let statement = &tokens[first..last];
    // The token the cursor is inside of or at the end of.
    let held = statement
        .iter()
        .find(|token| token.range.start < cursor && cursor <= token.range.end);
    let word = match held {
        Some(token) => match token.kind {
            TokenKind::Identifier | TokenKind::Keyword => token.range.clone(),
            TokenKind::String
            | TokenKind::Comment
            | TokenKind::ExecutableComment
            | TokenKind::Number
            | TokenKind::QuotedIdentifier => return None,
            TokenKind::Operator
            | TokenKind::Punctuation
            | TokenKind::Semicolon
            | TokenKind::Whitespace => cursor..cursor,
        },
        None => cursor..cursor,
    };
    let pieces: Vec<Piece<'_>> = statement
        .iter()
        .filter(|token| is_code(token.kind))
        .map(|token| Piece {
            kind: token.kind,
            // Tokens of another text have ranges this one may not hold:
            // they give an approximate result rather than a panic.
            text: text.get(token.range.clone()).unwrap_or_default(),
            range: token.range.clone(),
        })
        .collect();
    let table_words = table_words(&pieces);
    let before = pieces.partition_point(|piece| piece.range.start < word.start);
    let qualifier = qualifier(&pieces[..before], word.start)?;
    let expects = if qualifier.is_empty() {
        expects(&pieces, &table_words, before)
    } else {
        Expects::Columns
    };
    Some(Site {
        word,
        qualifier,
        expects,
        sources: sources(&pieces, &table_words),
        ctes: ctes(&pieces),
    })
}

/// The functions whose parentheses may hold a FROM of their own:
/// `EXTRACT(year FROM x)`, `trim(both FROM x)`, `substring(x FROM 1)`,
/// `overlay(x PLACING y FROM 1)`. None is a highlighted keyword.
const FROM_CALLS: [&str; 4] = ["EXTRACT", "TRIM", "SUBSTRING", "OVERLAY"];

/// For each piece, whether a table's name belongs after it. JOIN,
/// STRAIGHT_JOIN, INTO and TABLE always say so. FROM and UPDATE say so
/// only where they take a table, which one pass over the pieces tells:
///
/// - UPDATE does not after FOR, KEY or DO: `FOR UPDATE`,
///   `ON DUPLICATE KEY UPDATE`, `ON CONFLICT DO UPDATE`.
/// - FROM does once the statement has had a SELECT, DELETE, UPDATE or
///   SHOW, at whatever depth of parentheses: the editor closes none, so
///   in `SELECT count(na| FROM users` the FROM is still the statement's.
/// - FROM does not in `IS [NOT] DISTINCT FROM`, nor as the first FROM
///   inside the parentheses of one of `FROM_CALLS`, which is the call's
///   own. Such a call left open in its comma form
///   (`SELECT trim(na| FROM users`) takes the statement's FROM for its
///   own: a known limit.
fn table_words(pieces: &[Piece<'_>]) -> Vec<bool> {
    // Whether the statement has had a word that a FROM with tables follows.
    let mut headed = false;
    // Whether the next FROM is a call's own, at this depth of parentheses,
    // and the same for each enclosing depth, innermost last.
    let mut own_from = false;
    let mut outer = Vec::new();
    let mut words = Vec::with_capacity(pieces.len());
    for (index, piece) in pieces.iter().enumerate() {
        let previous = pieces[..index].last();
        let word = if piece.is("(") {
            outer.push(own_from);
            own_from = previous.is_some_and(|name| name.is_word(&FROM_CALLS));
            false
        } else if piece.is(")") {
            // A `)` that closes nothing leaves the outermost depth as it is.
            if let Some(enclosing) = outer.pop() {
                own_from = enclosing;
            }
            false
        } else if piece.is_keyword(&["SELECT", "DELETE", "SHOW"]) {
            headed = true;
            false
        } else if piece.is_keyword(&["UPDATE"]) {
            headed = true;
            // KEY and DO are not highlighted, so they are not keywords here.
            !previous.is_some_and(|clause| clause.is_word(&["FOR", "KEY", "DO"]))
        } else if piece.is_keyword(&["FROM"]) {
            if is_distinct_from(&pieces[..index]) {
                false
            } else if own_from {
                own_from = false;
                false
            } else {
                headed
            }
        } else {
            piece.is_keyword(&SOURCE_WORDS) || piece.is_keyword(&[TABLE])
        };
        words.push(word);
    }
    words
}

/// Whether a FROM after the pieces `before` is the one of
/// `IS [NOT] DISTINCT FROM`. After `SELECT DISTINCT`, with the columns
/// still to be typed, it is the statement's.
fn is_distinct_from(before: &[Piece<'_>]) -> bool {
    matches!(
        before,
        [.., comparison, distinct]
            if distinct.is_keyword(&["DISTINCT"]) && comparison.is_keyword(&["IS", "NOT"])
    )
}

/// The names joined by dots to a word starting at `start`, in order.
/// `None` when a dot touching the word follows no name.
fn qualifier(before: &[Piece<'_>], start: usize) -> Option<Vec<String>> {
    let mut names = Vec::new();
    let (mut at, mut edge) = (before.len(), start);
    while at >= 1 && before[at - 1].is(".") && before[at - 1].range.end == edge {
        let dot = &before[at - 1];
        let name = at
            .checked_sub(2)
            .map(|index| &before[index])
            .filter(|piece| {
                // Before a dot, a word is a name whatever the highlighter
                // calls it: `first.name` is a column of `first`.
                (piece.is_name() || piece.kind == TokenKind::Keyword)
                    && piece.range.end == dot.range.start
            })?;
        names.push(name.name());
        edge = name.range.start;
        at -= 2;
    }
    names.reverse();
    Some(names)
}

/// What belongs at a word that the first `before` of `pieces` come before.
/// `table_words` are the ones of `pieces`.
fn expects(pieces: &[Piece<'_>], table_words: &[bool], before: usize) -> Expects {
    let Some(previous) = pieces[..before].last() else {
        return Expects::Start;
    };
    if table_position(pieces, table_words, before) {
        return Expects::Tables;
    }
    if previous.is_keyword(&["AS"]) {
        return Expects::Name;
    }
    if previous.is_name() {
        // The name may be the end of `schema.table`: look before the chain.
        let mut start = before - 1;
        while start >= 2 && pieces[start - 1].is(".") && pieces[start - 2].is_name() {
            start -= 2;
        }
        if table_position(pieces, table_words, start) {
            return Expects::Name;
        }
    }
    Expects::Columns
}

/// Whether a table's name belongs after the first `before` of `pieces`:
/// they end in a table word, or in a comma of a FROM list.
fn table_position(pieces: &[Piece<'_>], table_words: &[bool], before: usize) -> bool {
    let Some(last) = before.checked_sub(1) else {
        return false;
    };
    table_words[last] || (pieces[last].is(",") && in_from_list(pieces, table_words, last))
}

/// Whether the comma at `comma` separates the tables of a FROM list: the
/// nearest keyword before it at its depth, `AS` aside, is a FROM that
/// takes tables.
fn in_from_list(pieces: &[Piece<'_>], table_words: &[bool], comma: usize) -> bool {
    let mut depth = 0_usize;
    for (index, piece) in pieces[..comma].iter().enumerate().rev() {
        if piece.is(")") {
            depth += 1;
        } else if piece.is("(") {
            // The comma is inside these parentheses: arguments, a list.
            let Some(inside) = depth.checked_sub(1) else {
                return false;
            };
            depth = inside;
        } else if depth == 0 && piece.kind == TokenKind::Keyword && !piece.is_keyword(&["AS"]) {
            return piece.is_keyword(&["FROM"]) && table_words[index];
        }
    }
    false
}

/// The tables a statement names, at every depth. `table_words` are the
/// ones of `pieces`.
fn sources(pieces: &[Piece<'_>], table_words: &[bool]) -> Vec<Source> {
    let mut found = Vec::new();
    let mut at = 0;
    while at < pieces.len() {
        let piece = &pieces[at];
        let names_source = table_words[at] && !piece.is_keyword(&[TABLE]);
        at += 1;
        if !names_source {
            continue;
        }
        // FROM takes a list; the others one table.
        let list = piece.is_keyword(&["FROM"]);
        // After INTO, parentheses hold the table's columns, not arguments.
        let calls = !piece.is_keyword(&["INTO"]);
        loop {
            let (source, next) = table(pieces, at, calls);
            found.extend(source);
            at = next;
            if !(list && pieces.get(at).is_some_and(|piece| piece.is(","))) {
                break;
            }
            at += 1;
        }
    }
    found
}

/// The table reference at `at`: the source it names, if it names one, and
/// the index after it.
///
/// With `calls`, a name followed directly by `(` is a function call
/// (`generate_series(1, 10) g`), not a table: it gives no source, and the
/// index after its arguments and its alias, so a FROM list goes on past
/// it. Its arguments are skipped, not read.
///
/// A bare parenthesis (a subquery) is left where it is, so the scan goes
/// on inside it. A FROM list is not continued after a subquery: in
/// `FROM (SELECT 1) s, users u`, `users u` is not found.
fn table(pieces: &[Piece<'_>], mut at: usize, calls: bool) -> (Option<Source>, usize) {
    while pieces
        .get(at)
        .is_some_and(|piece| piece.is_keyword(&["ONLY", "LATERAL"]))
    {
        at += 1;
    }
    let Some(first) = pieces.get(at).filter(|piece| piece.is_name()) else {
        return (None, at);
    };
    at += 1;
    let (mut schema, mut name) = (None, first.name());
    if pieces.get(at).is_some_and(|piece| piece.is("."))
        && let Some(second) = pieces.get(at + 1).filter(|piece| piece.is_name())
    {
        schema = Some(name);
        name = second.name();
        at += 2;
    }
    let call = calls && pieces.get(at).is_some_and(|piece| piece.is("("));
    if call {
        at = after_parens(pieces, at);
    }
    if pieces
        .get(at)
        .is_some_and(|piece| piece.is_keyword(&["AS"]))
    {
        at += 1;
    }
    let alias = pieces
        .get(at)
        .filter(|piece| piece.is_name())
        .map(Piece::name);
    if alias.is_some() {
        at += 1;
    }
    let source = (!call).then_some(Source {
        schema,
        name,
        alias,
    });
    (source, at)
}

/// The index after the parenthesis that closes the one at `at`, or the end.
fn after_parens(pieces: &[Piece<'_>], at: usize) -> usize {
    let mut depth = 0_usize;
    for (index, piece) in pieces.iter().enumerate().skip(at) {
        if piece.is("(") {
            depth += 1;
        } else if piece.is(")") {
            depth = depth.saturating_sub(1);
            if depth == 0 {
                return index + 1;
            }
        }
    }
    pieces.len()
}

/// The names of the common table expressions a statement starts with: each
/// name of the WITH list that an `AS` follows, after its optional list of
/// column names. The name counts once its `AS` is typed, before the query
/// in parentheses is.
fn ctes(pieces: &[Piece<'_>]) -> Vec<String> {
    let mut names = Vec::new();
    if !pieces
        .first()
        .is_some_and(|piece| piece.is_keyword(&["WITH"]))
    {
        return names;
    }
    let mut at = 1;
    if pieces
        .get(at)
        .is_some_and(|piece| piece.is_keyword(&["RECURSIVE"]))
    {
        at += 1;
    }
    while let Some(name) = pieces.get(at).filter(|piece| piece.is_name()) {
        at += 1;
        // An optional list of column names.
        if pieces.get(at).is_some_and(|piece| piece.is("(")) {
            at = after_parens(pieces, at);
        }
        if !pieces
            .get(at)
            .is_some_and(|piece| piece.is_keyword(&["AS"]))
        {
            break;
        }
        at += 1;
        names.push(name.name());
        // `NOT MATERIALIZED` and the like, then the query in parentheses.
        let mut skipped = 0;
        while skipped < 2 && pieces.get(at).is_some_and(|piece| !piece.is("(")) {
            at += 1;
            skipped += 1;
        }
        if !pieces.get(at).is_some_and(|piece| piece.is("(")) {
            break;
        }
        at = after_parens(pieces, at);
        if !pieces.get(at).is_some_and(|piece| piece.is(",")) {
            break;
        }
        at += 1;
    }
    names
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Dialect;
    use crate::sql::tokenize;

    /// The site at the `|` of `marked`.
    fn at(dialect: Dialect, marked: &str) -> Option<Site> {
        let cursor = marked.find('|').expect("a cursor mark");
        let text = marked.replacen('|', "", 1);
        site(&tokenize(dialect, &text), &text, cursor)
    }

    fn pg(marked: &str) -> Site {
        at(Dialect::Postgres, marked).expect("a site")
    }

    fn source(schema: Option<&str>, name: &str, alias: Option<&str>) -> Source {
        Source {
            schema: schema.map(str::to_owned),
            name: name.to_owned(),
            alias: alias.map(str::to_owned),
        }
    }

    #[test]
    fn the_word_is_the_one_the_cursor_is_in_or_ends() {
        assert_eq!(pg("SELECT na|").word, 7..9);
        // Inside a word: the whole word is replaced.
        assert_eq!(pg("SELECT na|me FROM users").word, 7..11);
        // After a space, and at the start of a word: empty.
        assert_eq!(pg("SELECT |").word, 7..7);
        assert_eq!(pg("SELECT * FROM |users").word, 14..14);
        // A keyword being typed is a word too.
        assert_eq!(pg("SELECT * FRO|").word, 9..12);
    }

    #[test]
    fn nothing_inside_strings_comments_numbers_and_quoted_names() {
        for dialect in [Dialect::Postgres, Dialect::MySql, Dialect::Sqlite] {
            for marked in [
                "SELECT 'us|ers'",
                "SELECT 'unterminated us|",
                "SELECT 1 -- from us|",
                "SELECT /* us| */ 1",
                "SELECT 12|",
            ] {
                assert_eq!(at(dialect, marked), None, "{dialect:?}: {marked}");
            }
        }
        // Each dialect's own quotes (PostgreSQL has no backtick names).
        assert_eq!(at(Dialect::MySql, "SELECT `us|ers`"), None);
        assert_eq!(at(Dialect::Sqlite, "SELECT `us|ers`"), None);
        assert_eq!(at(Dialect::Postgres, "SELECT \"Us|ers\""), None);
        assert_eq!(at(Dialect::Sqlite, "SELECT [Us|ers]"), None);
        assert_eq!(at(Dialect::Postgres, "SELECT $$ us| $$"), None);
        assert_eq!(at(Dialect::MySql, "SELECT 1 # us|"), None);
    }

    #[test]
    fn the_statement_is_the_one_between_the_semicolons_around_the_cursor() {
        // Trailing spaces still belong to the statement.
        let site = pg("SELECT 1; SELECT * FROM users u WHERE   |");
        assert_eq!(site.expects, Expects::Columns);
        assert_eq!(site.sources, [source(None, "users", Some("u"))]);
        // After a `;`: a new statement.
        let fresh = pg("SELECT * FROM users; |");
        assert_eq!(fresh.expects, Expects::Start);
        assert!(fresh.sources.is_empty());
        // The statement before the cursor's does not leak in, nor the one
        // after it.
        let middle = pg("SELECT * FROM a; SELECT x| FROM b; SELECT * FROM c");
        assert_eq!(middle.sources, [source(None, "b", None)]);
        // An empty script.
        let empty = pg("|");
        assert_eq!((empty.expects, empty.word), (Expects::Start, 0..0));
        // A `;` in a string is not a statement's end.
        assert_eq!(pg("SELECT ';' FROM users WHERE |").sources.len(), 1);
    }

    #[test]
    fn what_is_expected() {
        let expects = |marked| pg(marked).expects;
        assert_eq!(expects("SEL|"), Expects::Start);
        assert_eq!(expects("-- a comment\n  sel|"), Expects::Start);
        for marked in [
            "SELECT * FROM us|",
            "SELECT * FROM users JOIN or|",
            "UPDATE us|",
            "INSERT INTO us|",
            "TABLE us|",
            "SELECT * FROM users, or|",
            "SELECT * FROM users u, orders AS o, ac|",
        ] {
            assert_eq!(expects(marked), Expects::Tables, "{marked}");
        }
        for marked in [
            // Most likely an alias, or a new name.
            "SELECT * FROM users u|",
            "SELECT * FROM main.users wh|",
            "SELECT * FROM users JOIN orders o|",
            "SELECT count(*) AS to|",
            "SELECT * FROM users AS u|",
        ] {
            assert_eq!(expects(marked), Expects::Name, "{marked}");
        }
        for marked in [
            "SELECT na|",
            "SELECT id, na|",
            "SELECT * FROM users WHERE na|",
            "SELECT * FROM users u WHERE na|",
            "SELECT * FROM users JOIN orders ON us|",
            "SELECT * FROM users u JOIN orders o ON o.id = 1, na|",
            "SELECT lower(a, na|",
            "SELECT * FROM users u ORDER BY na|",
            // The word after an alias is past the name.
            "SELECT * FROM users u wh|",
            // Only FROM takes a list of tables: these are variables.
            "SELECT id, name INTO v_id, v_na|",
        ] {
            assert_eq!(expects(marked), Expects::Columns, "{marked}");
        }
        assert_eq!(
            at(Dialect::MySql, "SELECT * FROM a STRAIGHT_JOIN us|").map(|site| site.expects),
            Some(Expects::Tables)
        );
    }

    #[test]
    fn qualifiers_are_the_names_before_the_dots() {
        assert_eq!(pg("SELECT u.na|").qualifier, ["u"]);
        assert_eq!(pg("SELECT u.|").qualifier, ["u"]);
        assert_eq!(pg("SELECT u.|").word, 9..9);
        assert_eq!(pg("SELECT * FROM public.us|").qualifier, ["public"]);
        assert_eq!(pg("SELECT public.users.na|").qualifier, ["public", "users"]);
        assert_eq!(
            pg("SELECT \"My Schema\".\"Us\"\"ers\".na|").qualifier,
            ["My Schema", "Us\"ers"]
        );
        let mysql = at(Dialect::MySql, "SELECT `shop`.`users`.na|").unwrap();
        assert_eq!(mysql.qualifier, ["shop", "users"]);
        let sqlite = at(Dialect::Sqlite, "SELECT [main].na|").unwrap();
        assert_eq!(sqlite.qualifier, ["main"]);
        // The dots must touch the names.
        assert!(pg("SELECT u. na|").qualifier.is_empty());
        // A dot with no name before it offers nothing.
        assert_eq!(at(Dialect::Postgres, "SELECT (1).|"), None);
    }

    #[test]
    fn sources_are_the_tables_a_statement_names() {
        assert_eq!(
            pg("SELECT | FROM users u JOIN main.orders AS o ON o.user_id = u.id").sources,
            [
                source(None, "users", Some("u")),
                source(Some("main"), "orders", Some("o")),
            ]
        );
        assert_eq!(
            pg("SELECT | FROM users, orders o, active_users").sources,
            [
                source(None, "users", None),
                source(None, "orders", Some("o")),
                source(None, "active_users", None),
            ]
        );
        assert_eq!(
            pg("UPDATE users SET na|").sources,
            [source(None, "users", None)]
        );
        assert_eq!(
            pg("INSERT INTO users (na|").sources,
            [source(None, "users", None)]
        );
        assert_eq!(
            pg("DELETE FROM users WHERE na|").sources,
            [source(None, "users", None)]
        );
        assert_eq!(
            pg("SELECT | FROM \"Order Items\" oi").sources,
            [source(None, "Order Items", Some("oi"))]
        );
        // A keyword after a table is not its alias.
        assert_eq!(
            pg("SELECT | FROM users WHERE id = 1").sources,
            [source(None, "users", None)]
        );
        // The table after TABLE is made, dropped or altered, not read.
        let dropped = pg("DROP TABLE us|");
        assert_eq!(dropped.expects, Expects::Tables);
        assert!(dropped.sources.is_empty());
        // PostgreSQL's ONLY and LATERAL come before the name.
        assert_eq!(
            pg("SELECT | FROM ONLY users u").sources,
            [source(None, "users", Some("u"))]
        );
    }

    #[test]
    fn a_subquerys_tables_are_sources_too() {
        // Approximate by design: every depth counts.
        assert_eq!(
            pg("SELECT | FROM (SELECT * FROM orders o) s JOIN users u ON true").sources,
            [
                source(None, "orders", Some("o")),
                source(None, "users", Some("u"))
            ]
        );
        assert_eq!(
            pg("SELECT * FROM users WHERE id IN (SELECT user_id FROM orders WHERE |)").sources,
            [source(None, "users", None), source(None, "orders", None)]
        );
    }

    #[test]
    fn cte_names_are_listed() {
        let site = pg(
            "WITH recent AS (SELECT * FROM orders), big (id) AS NOT MATERIALIZED (SELECT 1) SELECT * FROM re|",
        );
        assert_eq!(site.ctes, ["recent", "big"]);
        assert_eq!(site.expects, Expects::Tables);
        assert_eq!(
            pg("WITH RECURSIVE tree AS (SELECT 1) SELECT |").ctes,
            ["tree"]
        );
        // Unfinished: the name before `AS (` already counts.
        assert_eq!(pg("WITH recent AS (SELECT * FROM us|").ctes, ["recent"]);
        assert!(pg("SELECT * FROM us|").ctes.is_empty());
    }

    #[test]
    fn keywords_are_read_in_any_case() {
        assert_eq!(pg("select * from us|").expects, Expects::Tables);
        let site = pg("select * from users u where na|");
        assert_eq!(site.expects, Expects::Columns);
        assert_eq!(site.sources, [source(None, "users", Some("u"))]);
        assert_eq!(pg("select * from users u|").expects, Expects::Name);
        assert_eq!(pg("select count(*) as to|").expects, Expects::Name);
        let site = pg("with recent as (select 1) select * from re|");
        assert_eq!(site.ctes, ["recent"]);
        assert_eq!(site.expects, Expects::Tables);
        // The words that keep FROM and UPDATE from naming a table, too.
        let site = pg("select extract(year from cre|) from only orders o");
        assert_eq!(site.expects, Expects::Columns);
        assert_eq!(site.sources, [source(None, "orders", Some("o"))]);
        let site = pg("select * from users u where u.id is not distinct from na|");
        assert_eq!(site.expects, Expects::Columns);
        assert_eq!(site.sources, [source(None, "users", Some("u"))]);
        let site = pg("insert into users (id) values (1) on conflict (id) do update na|");
        assert_eq!(site.expects, Expects::Columns);
        assert_eq!(site.sources, [source(None, "users", None)]);
        assert_eq!(
            pg("select * from users for update sk|").expects,
            Expects::Columns
        );
    }

    #[test]
    fn a_from_inside_a_function_call_names_no_table() {
        for dialect in [Dialect::Postgres, Dialect::MySql, Dialect::Sqlite] {
            let site = |marked| at(dialect, marked).expect("a site");
            for marked in [
                "SELECT EXTRACT(year FROM cre|) FROM orders o",
                // Typed before the call's parentheses are closed.
                "SELECT EXTRACT(year FROM cre| FROM orders o",
                "SELECT lower(trim(both FROM cre| FROM orders o",
            ] {
                let site = site(marked);
                assert_eq!(site.expects, Expects::Columns, "{dialect:?}: {marked}");
                assert_eq!(
                    site.sources,
                    [source(None, "orders", Some("o"))],
                    "{dialect:?}: {marked}"
                );
            }
            let trim = site("SELECT trim(both ' ' FROM na|) FROM users");
            assert_eq!(trim.expects, Expects::Columns, "{dialect:?}");
            assert_eq!(trim.sources, [source(None, "users", None)], "{dialect:?}");
            let substring = site("SELECT substring(name FROM 1 FOR 2), | FROM users");
            assert_eq!(substring.expects, Expects::Columns, "{dialect:?}");
            assert_eq!(
                substring.sources,
                [source(None, "users", None)],
                "{dialect:?}"
            );
            // A subquery in a call's arguments has its own FROM.
            let subquery = site("SELECT coalesce((SELECT max(id) FROM orders o), |) FROM users");
            assert_eq!(
                subquery.sources,
                [
                    source(None, "orders", Some("o")),
                    source(None, "users", None)
                ],
                "{dialect:?}"
            );
            assert_eq!(
                site("SELECT coalesce((SELECT max(id) FROM or|").expects,
                Expects::Tables,
                "{dialect:?}"
            );
        }
    }

    #[test]
    fn a_from_after_an_open_parenthesis_is_the_statements() {
        // The editor closes no parenthesis: this is what a select list
        // looks like while a call in it is typed.
        for dialect in [Dialect::Postgres, Dialect::MySql, Dialect::Sqlite] {
            let site = |marked| at(dialect, marked).expect("a site");
            let users = [source(None, "users", Some("u"))];
            let counted = site("SELECT count(na| FROM users u");
            assert_eq!(counted.expects, Expects::Columns, "{dialect:?}");
            assert_eq!(counted.sources, users, "{dialect:?}");
            let qualified = site("SELECT lower(u.| FROM users u");
            assert_eq!(qualified.qualifier, ["u"], "{dialect:?}");
            assert_eq!(qualified.sources, users, "{dialect:?}");
            let argument = site("SELECT coalesce(u.name, | FROM users u JOIN orders o ON true");
            assert_eq!(argument.expects, Expects::Columns, "{dialect:?}");
            assert_eq!(
                argument.sources,
                [
                    source(None, "users", Some("u")),
                    source(None, "orders", Some("o"))
                ],
                "{dialect:?}"
            );
            let window = site("SELECT sum(a) OVER (PARTITION BY | FROM users u");
            assert_eq!(window.expects, Expects::Columns, "{dialect:?}");
            assert_eq!(window.sources, users, "{dialect:?}");
            let bare = site("SELECT (| FROM users u");
            assert_eq!(bare.expects, Expects::Columns, "{dialect:?}");
            assert_eq!(bare.sources, users, "{dialect:?}");
            // A known limit: a call that may hold a FROM of its own, left
            // open in its comma form, takes the statement's FROM for its
            // own, and the sources are lost until it is closed.
            let open = site("SELECT trim(na| FROM users u");
            assert_eq!(open.expects, Expects::Columns, "{dialect:?}");
            assert!(open.sources.is_empty(), "{dialect:?}");
            assert_eq!(
                site("SELECT trim(na|) FROM users u").sources,
                users,
                "{dialect:?}"
            );
        }
    }

    #[test]
    fn a_call_with_a_from_of_its_own_keeps_the_statements() {
        for dialect in [Dialect::Postgres, Dialect::MySql, Dialect::Sqlite] {
            let site = |marked| at(dialect, marked).expect("a site");
            let t = [source(None, "t", None)];
            let substring = site("SELECT substring(a FROM 1 FOR 2), b| FROM t");
            assert_eq!(substring.expects, Expects::Columns, "{dialect:?}");
            assert_eq!(substring.sources, t, "{dialect:?}");
            let overlay = site("SELECT overlay(a PLACING b FROM 1 FOR 2), b| FROM t");
            assert_eq!(overlay.expects, Expects::Columns, "{dialect:?}");
            assert_eq!(overlay.sources, t, "{dialect:?}");
            // What follows the call's FROM is a value, not a table.
            for marked in [
                "SELECT substring(a FROM st|) FROM t",
                "SELECT overlay(a PLACING b FROM st|) FROM t",
            ] {
                let start = site(marked);
                assert_eq!(start.expects, Expects::Columns, "{dialect:?}: {marked}");
                assert_eq!(start.sources, t, "{dialect:?}: {marked}");
            }
            let condition = site("SELECT a FROM t WHERE x = EXTRACT(year FROM d) AND na|");
            assert_eq!(condition.expects, Expects::Columns, "{dialect:?}");
            assert_eq!(condition.sources, t, "{dialect:?}");
            // Only the call's first FROM is its own: a subquery in it has
            // one that takes tables.
            let subquery = site("SELECT EXTRACT(year FROM (SELECT max(d) FROM t)) FROM u WHERE |");
            assert_eq!(
                subquery.sources,
                [source(None, "t", None), source(None, "u", None)],
                "{dialect:?}"
            );
            // A call inside such a call has no FROM of its own, closed or
            // still open.
            let nested = site("SELECT trim(lower(a)) FROM t WHERE |");
            assert_eq!(nested.sources, t, "{dialect:?}");
            let inner = site("SELECT trim(lower(a| FROM t");
            assert_eq!(inner.sources, t, "{dialect:?}");
            // After it, the outer call's FROM is still to come.
            let around = site("SELECT trim(lower(a) FROM b), c| FROM t");
            assert_eq!(around.expects, Expects::Columns, "{dialect:?}");
            assert_eq!(around.sources, t, "{dialect:?}");
        }
    }

    #[test]
    fn show_takes_a_from_too() {
        for marked in [
            "SHOW COLUMNS FROM us|",
            "SHOW INDEX FROM us|",
            "SHOW TABLES FROM sh|",
            "show full columns from us|",
        ] {
            let site = at(Dialect::MySql, marked).expect("a site");
            assert_eq!(site.expects, Expects::Tables, "{marked}");
        }
    }

    #[test]
    fn from_names_tables_only_after_a_statements_head() {
        assert_eq!(pg("DELETE FROM us|").expects, Expects::Tables);
        // PostgreSQL's UPDATE takes a FROM list.
        let update = pg("UPDATE users SET a = 1 FROM other o, th|");
        assert_eq!(update.expects, Expects::Tables);
        assert_eq!(
            pg("UPDATE users SET a = 1 FROM other o WHERE |").sources,
            [
                source(None, "users", None),
                source(None, "other", Some("o"))
            ]
        );
        // No statement here reads a table after FROM.
        assert_eq!(pg("REVOKE ALL ON users FROM ad|").expects, Expects::Columns);
        assert!(pg("EXTRACT(year FROM cre|").sources.is_empty());
    }

    #[test]
    fn is_distinct_from_names_no_table() {
        for dialect in [Dialect::Postgres, Dialect::Sqlite] {
            let site = |marked| at(dialect, marked).expect("a site");
            let compared = site("SELECT * FROM users u WHERE u.id IS DISTINCT FROM na|");
            assert_eq!(compared.expects, Expects::Columns, "{dialect:?}");
            assert_eq!(
                compared.sources,
                [source(None, "users", Some("u"))],
                "{dialect:?}"
            );
            // Its comma is not a FROM list's.
            let listed = site("SELECT a IS DISTINCT FROM b, c| FROM users");
            assert_eq!(listed.expects, Expects::Columns, "{dialect:?}");
            assert_eq!(listed.sources, [source(None, "users", None)], "{dialect:?}");
            let negated = site("SELECT * FROM users u WHERE a IS NOT DISTINCT FROM b AND |");
            assert_eq!(
                negated.sources,
                [source(None, "users", Some("u"))],
                "{dialect:?}"
            );
            // SELECT DISTINCT with its columns still to come is not that.
            let select = site("SELECT DISTINCT | FROM users u");
            assert_eq!(select.expects, Expects::Columns, "{dialect:?}");
            assert_eq!(
                select.sources,
                [source(None, "users", Some("u"))],
                "{dialect:?}"
            );
            assert_eq!(
                site("SELECT DISTINCT FROM us|").expects,
                Expects::Tables,
                "{dialect:?}"
            );
        }
    }

    #[test]
    fn an_update_that_is_a_clause_names_no_table() {
        // KEY is no keyword to the tokenizer: the word is read as it is.
        let upsert = at(
            Dialect::MySql,
            "INSERT INTO users (id) VALUES (1) ON DUPLICATE KEY UPDATE na|",
        )
        .expect("a site");
        assert_eq!(upsert.expects, Expects::Columns);
        assert_eq!(upsert.sources, [source(None, "users", None)]);
        for dialect in [Dialect::Postgres, Dialect::Sqlite] {
            let upsert = at(
                dialect,
                "INSERT INTO users (id) VALUES (1) ON CONFLICT (id) DO UPDATE na|",
            )
            .expect("a site");
            assert_eq!(upsert.expects, Expects::Columns, "{dialect:?}");
            assert_eq!(upsert.sources, [source(None, "users", None)], "{dialect:?}");
        }
        for dialect in [Dialect::Postgres, Dialect::MySql] {
            let locked = at(dialect, "SELECT * FROM users FOR UPDATE sk|").expect("a site");
            assert_eq!(locked.expects, Expects::Columns, "{dialect:?}");
            assert_eq!(locked.sources, [source(None, "users", None)], "{dialect:?}");
        }
        let locked = pg("SELECT * FROM users FOR NO KEY UPDATE sk|");
        assert_eq!(locked.expects, Expects::Columns);
        assert_eq!(locked.sources, [source(None, "users", None)]);
        // The statement's own UPDATE still does.
        for dialect in [Dialect::Postgres, Dialect::MySql, Dialect::Sqlite] {
            let update = at(dialect, "SELECT 1; UPDATE us|").expect("a site");
            assert_eq!(update.expects, Expects::Tables, "{dialect:?}");
        }
    }

    #[test]
    fn a_function_in_a_from_list_is_no_source() {
        for marked in [
            "SELECT * FROM generate_series(1, 10) g, users u WHERE |",
            "SELECT * FROM generate_series(1, 10) AS g, users u WHERE |",
            "SELECT * FROM pg_catalog.generate_series(1, 10), users u WHERE |",
            "SELECT * FROM users u JOIN LATERAL generate_series(1, u.n) g ON |",
            "SELECT * FROM users u JOIN generate_series(1, 10) AS g ON |",
        ] {
            let site = pg(marked);
            assert_eq!(site.sources, [source(None, "users", Some("u"))], "{marked}");
            assert_eq!(site.expects, Expects::Columns, "{marked}");
        }
        // The list goes on after the call's parentheses, not inside them.
        let after = pg("SELECT * FROM generate_series(1, 10) g, us|");
        assert_eq!(after.expects, Expects::Tables);
        assert_eq!(after.sources, [source(None, "us", None)]);
        let inside = pg("SELECT * FROM generate_series(1, |");
        assert_eq!(inside.expects, Expects::Columns);
        assert!(inside.sources.is_empty());
        // After INTO the parentheses hold the table's columns.
        assert_eq!(
            pg("INSERT INTO main.users (id, na|").sources,
            [source(Some("main"), "users", None)]
        );
    }

    #[test]
    fn lateral_comes_before_a_joined_table() {
        assert_eq!(
            pg("SELECT | FROM users u JOIN LATERAL orders o ON true").sources,
            [
                source(None, "users", Some("u")),
                source(None, "orders", Some("o"))
            ]
        );
    }

    #[test]
    fn every_cte_of_a_with_list_is_found() {
        // Past a query with parentheses of its own.
        assert_eq!(
            pg("WITH a AS (SELECT count(*) FROM t), b AS (SELECT 1) SELECT |").ctes,
            ["a", "b"]
        );
        // Past `NOT MATERIALIZED` and `MATERIALIZED`.
        assert_eq!(
            pg(
                "WITH a AS NOT MATERIALIZED (SELECT 1), b AS MATERIALIZED (SELECT 2), c AS (SELECT 3) SELECT |"
            )
            .ctes,
            ["a", "b", "c"]
        );
        // The name counts as soon as its AS is typed.
        assert_eq!(pg("WITH a AS (SELECT 1), b AS |").ctes, ["a", "b"]);
        assert_eq!(pg("WITH a AS (SELECT 1), b |").ctes, ["a"]);
    }

    #[test]
    fn a_qualifier_may_be_spelled_like_a_keyword() {
        // FIRST is highlighted as a keyword; before a dot it is a name.
        assert_eq!(pg("SELECT first.na|").qualifier, ["first"]);
        assert_eq!(pg("SELECT first.na|").expects, Expects::Columns);
        // After FROM it is not taken for a table: a known limit.
        assert!(pg("SELECT | FROM first").sources.is_empty());
    }

    #[test]
    fn a_dot_apart_from_its_name_offers_nothing() {
        assert_eq!(at(Dialect::Postgres, "SELECT u .na|"), None);
        assert_eq!(at(Dialect::Postgres, "SELECT u .|"), None);
    }

    #[test]
    fn a_semicolon_right_before_the_cursor_ends_its_statement() {
        let fresh = pg("SELECT 1 FROM users;|");
        assert_eq!((fresh.expects, fresh.word), (Expects::Start, 20..20));
        assert!(fresh.sources.is_empty());
        // The statement after the cursor's `;` is the cursor's.
        let next = pg("SELECT 1 FROM users;| SELECT 2 FROM orders");
        assert_eq!(next.expects, Expects::Start);
        assert_eq!(next.sources, [source(None, "orders", None)]);
    }

    #[test]
    fn an_unterminated_quoted_name_is_still_a_name() {
        assert_eq!(
            pg("SELECT | FROM \"users").sources,
            [source(None, "users", None)]
        );
        // Only the opening quote: a name with nothing in it.
        assert_eq!(pg("SELECT | FROM \"").sources, [source(None, "", None)]);
        let mysql = at(Dialect::MySql, "SELECT | FROM `").expect("a site");
        assert_eq!(mysql.sources, [source(None, "", None)]);
        let sqlite = at(Dialect::Sqlite, "SELECT | FROM [users").expect("a site");
        assert_eq!(sqlite.sources, [source(None, "users", None)]);
        let sqlite = at(Dialect::Sqlite, "SELECT | FROM [").expect("a site");
        assert_eq!(sqlite.sources, [source(None, "", None)]);
    }

    #[test]
    fn an_executable_comment_is_code() {
        // MySQL runs what is inside: the word after it starts no statement.
        let after = at(Dialect::MySql, "/*!40101 x */ sel|").expect("a site");
        assert_eq!(after.expects, Expects::Columns);
        // Elsewhere it is a comment.
        let after = at(Dialect::Postgres, "/*!40101 x */ sel|").expect("a site");
        assert_eq!(after.expects, Expects::Start);
    }

    #[test]
    fn tokens_of_another_text_do_not_panic() {
        let tokens = tokenize(Dialect::Postgres, "SELECT u.name FROM users u WHERE na");
        // Shorter texts, and one whose characters the ranges split.
        for text in ["", "SELECT u.name", "\u{e9}\u{e9}\u{e9}\u{e9}\u{e9}"] {
            for cursor in 0..=40 {
                let _ = site(&tokens, text, cursor);
            }
        }
        let start = site(&tokens, "", 0).expect("a site");
        assert_eq!(start.expects, Expects::Start);
    }

    #[test]
    fn a_long_run_of_words_names_one_table() {
        let words = "a ".repeat(200_000);
        let text = format!("SELECT * FROM {words}");
        let tokens = tokenize(Dialect::Postgres, &text);
        let site = site(&tokens, &text, text.len()).expect("a site");
        assert_eq!(site.expects, Expects::Columns);
        assert_eq!(site.word, text.len()..text.len());
        // The first two words are a table and its alias; the rest are not
        // read as anything.
        assert_eq!(site.sources, [source(None, "a", Some("a"))]);
        assert!(site.ctes.is_empty());
    }

    #[test]
    fn a_large_statement_is_handled() {
        let tables: Vec<String> = (0..50_000).map(|index| format!("t{index}")).collect();
        let terms = " AND x = 1".repeat(100_000);
        let text = format!("SELECT * FROM {} WHERE x = 1{terms} ", tables.join(", "));
        let tokens = tokenize(Dialect::Postgres, &text);
        let end = site(&tokens, &text, text.len()).expect("a site");
        assert_eq!(end.expects, Expects::Columns);
        assert_eq!(end.sources.len(), 50_000);
        assert_eq!(end.sources[0], source(None, "t0", None));
        assert_eq!(end.sources[49_999], source(None, "t49999", None));
        // At the end of the FROM list, whose every comma is behind it.
        let list = format!("SELECT * FROM {}, ", tables.join(", "));
        let tokens = tokenize(Dialect::Postgres, &list);
        let end = site(&tokens, &list, list.len()).expect("a site");
        assert_eq!(end.expects, Expects::Tables);
        assert_eq!(end.sources.len(), 50_000);
    }

    #[test]
    fn deep_and_unbalanced_parentheses_are_handled() {
        // Each depth is kept on a stack of its own, not the call stack.
        let text = format!("SELECT {}SELECT a FROM users u WHERE ", "(".repeat(100_000));
        let tokens = tokenize(Dialect::Postgres, &text);
        let deep = site(&tokens, &text, text.len()).expect("a site");
        assert_eq!(deep.expects, Expects::Columns);
        assert_eq!(deep.sources, [source(None, "users", Some("u"))]);
        // More closed than were opened.
        let text = format!("SELECT a{} FROM users u WHERE ", ")".repeat(100_000));
        let tokens = tokenize(Dialect::Postgres, &text);
        let closed = site(&tokens, &text, text.len()).expect("a site");
        assert_eq!(closed.expects, Expects::Columns);
        assert_eq!(closed.sources, [source(None, "users", Some("u"))]);
    }
}

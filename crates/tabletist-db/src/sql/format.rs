//! Format: lays a script's queries out in river style and uppercases
//! reserved words. It works on the tokens and never parses: only
//! whitespace and the case of keywords change, and a check at the end
//! holds it to that.

use std::borrow::Cow;
use std::ops::Range;

use super::{Token, TokenKind, statements_from, tokenize};
use crate::Dialect;

/// A script after Format. It has no `Debug`: what a user types into the
/// editor is never logged.
#[derive(Clone, PartialEq, Eq)]
pub struct Formatted {
    pub text: String,
    /// Byte offset of the cursor in `text`.
    pub cursor: usize,
}

/// The keywords Format uppercases: the ones that cannot be an unquoted
/// table name or alias where case would matter. Every one the MySQL
/// dialect reads as a keyword is reserved in MySQL 5.7, MySQL 8 and
/// MariaDB (the MySQL tests ask the server they run against); the words
/// only PostgreSQL or SQLite read as keywords are here because those
/// compare unquoted names without case.
pub const UPPERCASED: &[&str] = &[
    "ALL",
    "ALTER",
    "AND",
    "AS",
    "ASC",
    "BETWEEN",
    "BY",
    "CASE",
    "CREATE",
    "CROSS",
    "DEFAULT",
    "DELETE",
    "DESC",
    "DESCRIBE",
    "DISTINCT",
    "DROP",
    "ELSE",
    "EXISTS",
    "EXPLAIN",
    "FALSE",
    "FETCH",
    "FOR",
    "FROM",
    "GLOB",
    "GROUP",
    "HAVING",
    "ILIKE",
    "IN",
    "INNER",
    "INSERT",
    "INTERVAL",
    "INTO",
    "IS",
    "JOIN",
    "LATERAL",
    "LEFT",
    "LIKE",
    "LIMIT",
    "NATURAL",
    "NOT",
    "NULL",
    "ON",
    "OR",
    "ORDER",
    "OUTER",
    "PARTITION",
    "PRAGMA",
    "REGEXP",
    "RETURNING",
    "RIGHT",
    "SELECT",
    "SET",
    "SHOW",
    "SIMILAR",
    "STRAIGHT_JOIN",
    "TABLE",
    "THEN",
    "TO",
    "TRUE",
    "UNION",
    "UPDATE",
    "USING",
    "VALUES",
    "WHEN",
    "WHERE",
    "WITH",
];

/// Keywords that can be names, uppercased where the layout read them as
/// structure, on the dialects that compare unquoted names without case.
const STRUCTURE: &[&str] = &[
    "EXCEPT",
    "FULL",
    "INTERSECT",
    "OFFSET",
    "RECURSIVE",
    "WINDOW",
];

/// The one-word heads that are a head wherever they stand at the top level
/// of a block.
const HEADS: &[&str] = &[
    "EXCEPT",
    "FETCH",
    "FOR",
    "INTERSECT",
    "LIMIT",
    "OFFSET",
    "UNION",
    "WINDOW",
];

/// The words that lead a join.
const JOIN_LEADERS: &[&str] = &[
    "CROSS", "FULL", "INNER", "LEFT", "NATURAL", "OUTER", "RIGHT",
];

/// The width of the river: `SELECT`.
const RIVER: usize = 6;

/// How far a `WHEN`, an `ELSE` and what follows them stand in from their
/// `CASE`.
const CASE_INDENT: usize = 4;

/// How deep queries in parentheses are laid out as blocks. The layout
/// recurses once for each, so a script nested without end (a generated
/// one, a hostile paste) must not be followed all the way down: past
/// this depth a parenthesis stays on its line, whatever it holds.
const MAX_DEPTH: usize = 64;

/// `script` with the statements `selection` overlaps formatted, or all of
/// them when it is `None` or empty, and where `cursor` went. All three are
/// byte offsets. `None` when there is nothing to change.
pub fn format(
    dialect: Dialect,
    script: &str,
    selection: Option<Range<usize>>,
    cursor: usize,
) -> Option<Formatted> {
    let tokens = tokenize(dialect, script);
    let (region, laid) = laid_out(dialect, script, &tokens, selection)?;
    let text = [&script[..region.start], &laid, &script[region.end..]].concat();
    if text == script {
        return None;
    }
    let after = tokenize(dialect, &text);
    if !same_tokens(script, &tokens, &text, &after) {
        log::warn!("Format would have changed a token: the text is left as typed");
        return None;
    }
    let cursor = if cursor <= region.start {
        cursor
    } else if cursor >= region.end {
        region.start + laid.len() + (cursor.min(script.len()) - region.end)
    } else {
        moved(&tokens, &after, cursor).unwrap_or(text.len())
    };
    Some(Formatted { text, cursor })
}

/// The bytes Format lays out and what it makes of them, unchecked.
fn laid_out(
    dialect: Dialect,
    script: &str,
    tokens: &[Token],
    selection: Option<Range<usize>>,
) -> Option<(Range<usize>, String)> {
    let region = region(script, tokens, selection)?;
    // The indentation of its first line is Format's to set.
    let region = line_start(script, region.start)..region.end;
    // A statement that does not start its line goes on from where it
    // stands: no line is started for it, and nothing is put before it.
    let line = script[..region.start].rfind('\n').map_or(0, |end| end + 1);
    let mut layout = Layout {
        dialect,
        items: items(script, tokens, &region),
        at: 0,
        writer: Writer {
            out: String::new(),
            column: script[line..region.start].chars().count(),
        },
        ended: false,
    };
    layout.run();
    Some((region, layout.writer.out))
}

/// The bytes Format lays out: from the first statement `selection`
/// overlaps to the last, or from the script's first token to its last.
fn region(script: &str, tokens: &[Token], selection: Option<Range<usize>>) -> Option<Range<usize>> {
    match selection.filter(|selection| !selection.is_empty()) {
        None => {
            let mut visible = tokens
                .iter()
                .filter(|token| token.kind != TokenKind::Whitespace);
            let first = visible.next()?;
            let last = visible.next_back().unwrap_or(first);
            Some(first.range.start..last.range.end)
        }
        Some(selection) => {
            let statements = statements_from(script, tokens);
            let mut touched = statements.iter().filter(|statement| {
                statement.range.start < selection.end && selection.start < statement.end
            });
            let first = touched.next()?;
            let last = touched.next_back().unwrap_or(first);
            Some(first.range.start..last.end)
        }
    }
}

/// The start of the line `at` is on, when only blanks stand between; else
/// `at`.
fn line_start(script: &str, at: usize) -> usize {
    let line = script[..at].rfind('\n').map_or(0, |end| end + 1);
    let blank = script[line..at]
        .bytes()
        .all(|byte| byte == b' ' || byte == b'\t');
    if blank { line } else { at }
}

/// A token that is not whitespace, and the whitespace before it.
#[derive(Clone, Copy)]
struct Item<'a> {
    kind: TokenKind,
    text: &'a str,
    /// Empty when the token touches the one before it, and for the first.
    space: &'a str,
}

impl Item<'_> {
    fn is(&self, punctuation: &str) -> bool {
        self.kind == TokenKind::Punctuation && self.text == punctuation
    }

    /// `--` and MySQL's `#`: a comment that ends its line.
    fn ends_line(&self) -> bool {
        self.kind == TokenKind::Comment && !self.text.starts_with("/*")
    }

    /// `@`, `:`, a `$` and a number: what a server reads as one token
    /// with a word that touches it (a user variable, a parameter, MySQL's
    /// names that start with digits), though the tokenizer cuts the two
    /// apart.
    fn binds(&self) -> bool {
        match self.kind {
            TokenKind::Number => true,
            TokenKind::Operator => matches!(self.text, "@" | ":"),
            TokenKind::Punctuation => self.text.starts_with('$'),
            _ => false,
        }
    }

    /// A `$`, and a number that starts with its `.`: what a server reads
    /// as more of the word that touches it from the left (SQLite's
    /// `limit$x`, the qualified name `offset.1st`).
    fn extends(&self) -> bool {
        match self.kind {
            TokenKind::Number => self.text.starts_with('.'),
            TokenKind::Punctuation => self.text.starts_with('$'),
            _ => false,
        }
    }
}

fn items<'a>(script: &'a str, tokens: &[Token], region: &Range<usize>) -> Vec<Item<'a>> {
    let mut items = Vec::new();
    let mut space = "";
    let inside = tokens
        .iter()
        .filter(|token| region.start <= token.range.start && token.range.end <= region.end);
    for token in inside {
        let text = &script[token.range.clone()];
        if token.kind == TokenKind::Whitespace {
            // What stands before the first item is not between two.
            if !items.is_empty() {
                space = text;
            }
        } else {
            items.push(Item {
                kind: token.kind,
                text,
                space,
            });
            space = "";
        }
    }
    items
}

fn newlines(space: &str) -> usize {
    space.bytes().filter(|byte| *byte == b'\n').count()
}

/// The whitespace of a statement that keeps its lines: its line breaks and
/// the indentation after the last, without what stood before a line's end.
fn kept(space: &str) -> Cow<'_, str> {
    match space.rfind('\n') {
        None => Cow::Borrowed(space),
        Some(last) => Cow::Owned("\n".repeat(newlines(space)) + &space[last + 1..]),
    }
}

/// The text laid out so far, and the column the next character lands on.
struct Writer {
    out: String,
    /// In characters, not bytes.
    column: usize,
}

impl Writer {
    fn push(&mut self, text: &str) {
        match text.rfind('\n') {
            Some(last) => self.column = text[last + 1..].chars().count(),
            None => self.column += text.chars().count(),
        }
        self.out.push_str(text);
    }

    /// Starts a line at `column`. A line with nothing on it yet is used.
    fn line(&mut self, column: usize) {
        if self.column > 0 {
            self.out.push('\n');
        }
        self.out.extend(std::iter::repeat_n(' ', column));
        self.column = column;
    }
}

/// What stands before the next statement or comment of a script.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Before {
    Nothing,
    Statement,
    /// A comment, and whether it ends its line.
    Comment(bool),
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Clause {
    Other,
    Select,
    With,
    Join,
}

/// What a token of a block is inside of, apart from the block.
#[derive(Clone, Copy)]
enum Frame {
    /// A parenthesis that opens no block, and the column of its `(`.
    Paren(usize),
    /// A `CASE`, and the column it starts at.
    Case(usize),
}

/// Where a token goes.
#[derive(Clone, Copy)]
enum Place {
    /// After the token before it, on its line.
    Inline,
    /// At the start of a line, at this column.
    Line(usize),
}

/// A query being laid out: a statement's, or one in parentheses.
struct Block {
    /// The column its river starts at.
    base: usize,
    /// How many blocks it is inside of. A statement's own block is
    /// inside none, and no `)` ends it.
    depth: usize,
    clause: Clause,
    /// `AND` and `OR` start lines: in a `WHERE`, a `HAVING`, a join's `ON`.
    conditions: bool,
    /// A `BETWEEN` waits for its `AND`.
    between: bool,
    frames: Vec<Frame>,
    /// No token of it is written yet.
    first: bool,
    /// A list's comma was written: its next item starts a line.
    broken: bool,
    /// A head was written: one space follows it.
    headed: bool,
}

impl Block {
    fn new(base: usize, depth: usize) -> Self {
        Self {
            base,
            depth,
            clause: Clause::Other,
            conditions: false,
            between: false,
            frames: Vec::new(),
            first: true,
            broken: false,
            headed: false,
        }
    }

    /// Where what follows a head's line goes: list items, comments.
    fn content(&self) -> usize {
        self.base + RIVER + 1
    }
}

struct Layout<'a> {
    dialect: Dialect,
    items: Vec<Item<'a>>,
    /// The item to write next.
    at: usize,
    writer: Writer,
    /// What was written last ends its line: a `--` comment, or a comment
    /// that had a line to itself.
    ended: bool,
}

impl<'a> Layout<'a> {
    /// Statements and the comments between them, each from column 0.
    fn run(&mut self) {
        let mut before = Before::Nothing;
        while let Some(item) = self.items.get(self.at).copied() {
            let comment = item.kind == TokenKind::Comment;
            let lines = newlines(item.space);
            let breaks = match before {
                Before::Nothing => 0,
                // A comment after a statement on its line stays there.
                Before::Statement if comment && lines == 0 => 0,
                Before::Statement if comment || item.kind == TokenKind::Semicolon => lines.min(2),
                Before::Statement => 2,
                Before::Comment(ends) if lines == 0 => usize::from(ends),
                Before::Comment(_) => lines.min(2),
            };
            self.writer.push(&"\n".repeat(breaks));
            if comment {
                let trails = before == Before::Statement && lines == 0;
                if self.writer.column > 0 && !item.space.is_empty() {
                    self.writer.push(" ");
                }
                self.writer.push(item.text);
                before = Before::Comment(trails || item.ends_line() || self.own_line(self.at));
                self.at += 1;
            } else {
                self.ended = false;
                if self.starts_block(self.at) || self.opens_block(self.at) {
                    self.query();
                } else {
                    self.plain();
                }
                before = Before::Statement;
            }
        }
    }

    /// A statement that is no query: its lines stay as typed.
    fn plain(&mut self) {
        let start = self.at;
        while let Some(item) = self.items.get(self.at).copied() {
            if self.at > start {
                let strings = item.kind == TokenKind::String
                    && self.items[self.at - 1].kind == TokenKind::String;
                if strings {
                    self.writer.push(item.space);
                } else {
                    self.writer.push(&kept(item.space));
                }
            } else if self.writer.column > 0 && !item.space.is_empty() {
                self.writer.push(" ");
            }
            let text = self.cased(self.at, false);
            self.writer.push(&text);
            self.at += 1;
            if item.kind == TokenKind::Semicolon {
                break;
            }
        }
    }

    /// A query statement and its `;`.
    fn query(&mut self) {
        self.block(0, 0);
        if self.items.get(self.at).is_some() {
            // The block stopped at the statement's `;`.
            if self.ended {
                self.writer.line(0);
                self.ended = false;
            }
            self.writer.push(";");
            self.at += 1;
        }
    }

    /// Lays out a block from its first token to the `)` that ends it (left
    /// for the caller), the statement's `;` (left too) or the end.
    fn block(&mut self, base: usize, depth: usize) {
        let mut block = Block::new(base, depth);
        while let Some(item) = self.items.get(self.at).copied() {
            match item.kind {
                TokenKind::Semicolon => return,
                TokenKind::Comment => self.comment(&block),
                TokenKind::Keyword if !self.named(self.at) => self.keyword(&mut block),
                _ if item.is("(") => self.open(&mut block),
                _ if item.is(")") => {
                    let open = block
                        .frames
                        .iter()
                        .rposition(|frame| matches!(frame, Frame::Paren(_)));
                    match open {
                        Some(open) => {
                            let Frame::Paren(column) = block.frames[open] else {
                                unreachable!("the frame found is a parenthesis");
                            };
                            block.frames.truncate(open);
                            let place = if self.ended {
                                Place::Line(column)
                            } else {
                                Place::Inline
                            };
                            self.put(&mut block, place, false);
                        }
                        None if block.depth == 0 => self.put(&mut block, Place::Inline, false),
                        None => return,
                    }
                }
                _ if item.is(",") => {
                    let list = block.frames.is_empty()
                        && matches!(block.clause, Clause::Select | Clause::With);
                    self.put(&mut block, Place::Inline, false);
                    block.broken = list;
                }
                _ => self.put(&mut block, Place::Inline, false),
            }
        }
    }

    /// A `(`: a block of its own, or a parenthesis on its line.
    fn open(&mut self, block: &mut Block) {
        let nested = block.depth < MAX_DEPTH && self.opens_block(self.at);
        self.put(block, Place::Inline, false);
        let column = self.writer.column;
        if !nested {
            block.frames.push(Frame::Paren(column - 1));
            return;
        }
        self.block(column, block.depth + 1);
        if self.items.get(self.at).is_some_and(|item| item.is(")")) {
            if self.ended {
                self.writer.line(column - 1);
                self.ended = false;
            }
            self.writer.push(")");
            self.at += 1;
        }
    }

    /// A keyword that is not a name: a head, a part of a `CASE`, or a word
    /// on its line.
    fn keyword(&mut self, block: &mut Block) {
        let at = self.at;
        let text = self.items[at].text;
        let is = |word: &str| text.eq_ignore_ascii_case(word);
        if is("CASE") {
            self.put(block, Place::Inline, false);
            let column = self.writer.column - text.len();
            block.frames.push(Frame::Case(column));
            return;
        }
        if let Some(Frame::Case(column)) = block.frames.last().copied() {
            if is("WHEN") || is("ELSE") {
                return self.put(block, Place::Line(column + CASE_INDENT), false);
            }
            if is("END") {
                block.frames.pop();
                return self.put(block, Place::Line(column), true);
            }
        }
        let head = if block.frames.is_empty() {
            self.head(block, at)
        } else {
            None
        };
        let Some(words) = head else {
            return self.put(block, Place::Inline, false);
        };
        let column = block.base + RIVER.saturating_sub(text.chars().count());
        // A block's first head follows what it is in: its `(`, or a
        // comment before the statement on its line.
        let place = if block.first && !self.ended && self.writer.column > 0 {
            Place::Inline
        } else {
            Place::Line(column)
        };
        self.put(block, place, self.structure(at));
        for word in at + 1..at + words {
            self.put(block, Place::Inline, self.structure(word));
        }
        block.headed = true;
    }

    /// The head the keyword at `at` starts at the top level of `block`, as
    /// the number of its words, with the block moved on to its clause.
    /// `None` for a word that stays on its line.
    fn head(&self, block: &mut Block, at: usize) -> Option<usize> {
        let text = self.items[at].text;
        let any = |words: &[&str]| words.iter().any(|word| word.eq_ignore_ascii_case(text));
        if any(&["AND"]) && block.between {
            block.between = false;
            return None;
        }
        if any(&["AND", "OR"]) {
            return block.conditions.then_some(1);
        }
        let distinct = at > 0 && self.word(at - 1, &["DISTINCT"]);
        let (words, clause, conditions) = if any(&["SELECT"]) {
            (1, Clause::Select, false)
        } else if any(&["WITH"]) && block.first {
            let recursive = self.word(at + 1, &["RECURSIVE"]);
            (1 + usize::from(recursive), Clause::With, false)
        } else if any(&["WHERE", "HAVING"]) {
            (1, Clause::Other, true)
        } else if any(HEADS) || (any(&["FROM"]) && !distinct) {
            (1, Clause::Other, false)
        } else if any(&["GROUP", "ORDER"]) && self.word(at + 1, &["BY"]) {
            (2, Clause::Other, false)
        } else if any(&["STRAIGHT_JOIN"]) && block.clause != Clause::Select {
            (1, Clause::Join, false)
        } else if let Some(words) = self.join(at) {
            (words, Clause::Join, false)
        } else {
            block.between |= any(&["BETWEEN"]);
            block.conditions |= any(&["ON"]) && block.clause == Clause::Join;
            return None;
        };
        block.clause = clause;
        block.conditions = conditions;
        Some(words)
    }

    /// How many words the join head at `at` has: leaders, then `JOIN`.
    fn join(&self, at: usize) -> Option<usize> {
        // No join has more leaders than `NATURAL LEFT OUTER`: a longer run
        // of them is not read to its end for every word of it.
        let mut end = at;
        while end - at < 3 && self.word(end, JOIN_LEADERS) {
            end += 1;
        }
        self.word(end, &["JOIN"]).then_some(end - at + 1)
    }

    /// Whether the keyword at `index`, read as structure, is uppercased
    /// though it can be a name: not on MySQL, where it may be an alias and
    /// an alias's case can matter.
    fn structure(&self, index: usize) -> bool {
        match self.dialect {
            Dialect::MySql => false,
            Dialect::Postgres | Dialect::Sqlite => self.word(index, STRUCTURE),
        }
    }

    /// A comment inside a query.
    fn comment(&mut self, block: &Block) {
        let item = self.items[self.at];
        let own = self.own_line(self.at);
        if own || self.ended {
            self.writer.line(block.content());
        } else if self.writer.column > 0 && !item.space.is_empty() {
            self.writer.push(" ");
        }
        self.writer.push(item.text);
        self.ended = own || item.ends_line();
        self.at += 1;
    }

    /// Writes the item at `at` and what separates it from the one before.
    /// `structure`: it is a word read as structure, uppercased whatever
    /// the list says.
    fn put(&mut self, block: &mut Block, place: Place, structure: bool) {
        let at = self.at;
        let item = self.items[at];
        let line = match place {
            // A line break here would make a comment of two minus signs.
            _ if self.dashes(at) => None,
            Place::Line(column) => Some(column),
            Place::Inline if block.broken || self.ended => Some(block.content()),
            Place::Inline => None,
        };
        if let Some(column) = line {
            self.writer.line(column);
        } else if at > 0 && self.writer.column > 0 {
            let before = self.items[at - 1];
            let strings = item.kind == TokenKind::String && before.kind == TokenKind::String;
            if strings && !item.space.is_empty() {
                // PostgreSQL joins two strings only across a line break.
                self.writer.push(item.space);
            } else if item.is(",") || item.is(")") || before.is("(") {
                // Nothing stands before a `,` or a `)`, nor after a `(`.
            } else if block.headed || before.is(",") || !item.space.is_empty() {
                self.writer.push(" ");
            }
        }
        let text = self.cased(at, structure);
        self.writer.push(&text);
        self.at += 1;
        self.ended = false;
        block.first = false;
        block.broken = false;
        block.headed = false;
    }

    /// The item's text, uppercased when it is a keyword on the list (or
    /// `structure` says so) and not a name.
    fn cased(&self, index: usize, structure: bool) -> Cow<'a, str> {
        let item = self.items[index];
        let upper = item.kind == TokenKind::Keyword
            && !self.named(index)
            && (structure
                || UPPERCASED
                    .iter()
                    .any(|word| word.eq_ignore_ascii_case(item.text)));
        if upper && item.text.bytes().any(|byte| byte.is_ascii_lowercase()) {
            Cow::Owned(item.text.to_ascii_uppercase())
        } else {
            Cow::Borrowed(item.text)
        }
    }

    /// Whether the item at `index` touches two touching minus signs on
    /// MySQL, which reads `--` as a comment once whitespace follows it.
    fn dashes(&self, index: usize) -> bool {
        let minus = |index: usize| {
            let item = self.items[index];
            item.kind == TokenKind::Operator && item.text == "-"
        };
        match self.dialect {
            Dialect::MySql => {
                index >= 2
                    && self.items[index].space.is_empty()
                    && minus(index - 1)
                    && self.items[index - 1].space.is_empty()
                    && minus(index - 2)
            }
            Dialect::Postgres | Dialect::Sqlite => false,
        }
    }

    /// Whether the item at `index` is a name, whatever it spells: it
    /// stands beside a `.` (a qualified name), it touches a sigil or a
    /// number before it (`@from`, `:limit`, `1st`), or what touches it
    /// from the right goes on with it (`limit$x`, `offset.1st`). Such a
    /// word is no head and keeps its case, so the layout never parts it
    /// from what it touches: the check at the end, which reads the two as
    /// the tokenizer does, would not see that.
    fn named(&self, index: usize) -> bool {
        let dot = |index: usize| self.items.get(index).is_some_and(|item| item.is("."));
        let follows =
            index > 0 && self.items[index].space.is_empty() && self.items[index - 1].binds();
        let leads = self
            .items
            .get(index + 1)
            .is_some_and(|next| next.space.is_empty() && next.extends());
        dot(index + 1) || (index > 0 && dot(index - 1)) || follows || leads
    }

    /// Whether the item at `index` is one of `words` as a keyword, not a
    /// name.
    fn word(&self, index: usize, words: &[&str]) -> bool {
        self.items.get(index).is_some_and(|item| {
            item.kind == TokenKind::Keyword
                && words
                    .iter()
                    .any(|word| word.eq_ignore_ascii_case(item.text))
        }) && !self.named(index)
    }

    /// Whether a query starts at `index`.
    fn starts_block(&self, index: usize) -> bool {
        self.word(index, &["SELECT", "WITH"])
    }

    /// Whether the item at `index` is a `(` with a query inside: its
    /// first token that is not a comment starts one.
    fn opens_block(&self, index: usize) -> bool {
        self.items.get(index).is_some_and(|item| item.is("("))
            && (index + 1..self.items.len())
                .find(|inner| self.items[*inner].kind != TokenKind::Comment)
                .is_some_and(|inner| self.starts_block(inner))
    }

    /// Whether the comment at `index` had a line to itself.
    fn own_line(&self, index: usize) -> bool {
        let item = self.items[index];
        let starts = index == 0 || newlines(item.space) > 0;
        let ends = item.ends_line()
            || self
                .items
                .get(index + 1)
                .is_none_or(|next| newlines(next.space) > 0);
        starts && ends
    }
}

/// Whether two texts hold the same tokens, whitespace aside and keywords
/// compared without case: what Format promises of its result.
fn same_tokens(old: &str, old_tokens: &[Token], new: &str, new_tokens: &[Token]) -> bool {
    let visible = |tokens: &'_ [Token]| {
        tokens
            .iter()
            .filter(|token| token.kind != TokenKind::Whitespace)
            .cloned()
            .collect::<Vec<Token>>()
    };
    let (old_tokens, new_tokens) = (visible(old_tokens), visible(new_tokens));
    old_tokens.len() == new_tokens.len()
        && old_tokens.iter().zip(&new_tokens).all(|(a, b)| {
            let (before, after) = (&old[a.range.clone()], &new[b.range.clone()]);
            a.kind == b.kind
                && if a.kind == TokenKind::Keyword {
                    before.eq_ignore_ascii_case(after)
                } else {
                    before == after
                }
        })
}

/// Where a cursor at byte `cursor` of the old text stands in the new one:
/// after the same byte of the same token, or, from whitespace, before the
/// next token.
fn moved(old: &[Token], new: &[Token], cursor: usize) -> Option<usize> {
    let visible = |tokens: &'_ [Token]| {
        tokens
            .iter()
            .filter(|token| token.kind != TokenKind::Whitespace)
            .map(|token| token.range.clone())
            .collect::<Vec<Range<usize>>>()
    };
    // The bytes of tokens before the cursor, and whether it stands in a
    // token or right after one.
    let mut count = 0;
    let mut inside = false;
    for range in visible(old) {
        if range.start >= cursor {
            break;
        }
        count += cursor.min(range.end) - range.start;
        inside = cursor <= range.end;
    }
    let mut passed = 0;
    for range in visible(new) {
        if inside && count <= passed + range.len() {
            return Some(range.start + (count - passed));
        }
        if !inside && passed == count {
            return Some(range.start);
        }
        passed += range.len();
    }
    None
}

#[cfg(test)]
mod tests {
    use super::super::is_keyword;
    use super::*;

    const DIALECTS: [Dialect; 3] = [Dialect::Postgres, Dialect::MySql, Dialect::Sqlite];

    /// `script` with the statements `selection` overlaps laid out (all of
    /// them for `None`), and where the laid out part is in the result. The
    /// layout itself, before the check that would hide a fault: the tokens
    /// are compared here.
    fn laid(
        dialect: Dialect,
        script: &str,
        selection: Option<Range<usize>>,
    ) -> (String, Range<usize>) {
        let old = tokenize(dialect, script);
        let Some((region, laid)) = laid_out(dialect, script, &old, selection) else {
            return (script.to_owned(), 0..0);
        };
        let result = [&script[..region.start], &laid, &script[region.end..]].concat();
        let new = tokenize(dialect, &result);
        assert!(same_tokens(script, &old, &result, &new), "{script:?}");
        assert!(sigils_hold(script, &old, &result, &new), "{script:?}");
        (result, region.start..region.start + laid.len())
    }

    /// Whether every word that touched a sigil or a number before it
    /// (`@from`, `:limit`, `1st`) still touches it. The tokenizer cuts
    /// these apart where a server reads one token, so `same_tokens` cannot
    /// tell when the layout parts them.
    fn sigils_hold(old: &str, old_tokens: &[Token], new: &str, new_tokens: &[Token]) -> bool {
        let visible = |tokens: &[Token]| {
            let tokens = tokens
                .iter()
                .filter(|token| token.kind != TokenKind::Whitespace);
            tokens.cloned().collect::<Vec<Token>>()
        };
        let (old_tokens, new_tokens) = (visible(old_tokens), visible(new_tokens));
        let touching =
            |tokens: &[Token], at: usize| tokens[at - 1].range.end == tokens[at].range.start;
        let word = |token: &Token| matches!(token.kind, TokenKind::Keyword | TokenKind::Identifier);
        (1..old_tokens.len()).all(|at| {
            let (before, after) = (&old_tokens[at - 1], &old_tokens[at]);
            let (first, second) = (&old[before.range.clone()], &old[after.range.clone()]);
            // `@from`, `:limit`, `$offset`, `1st`: the word is the second.
            let follows = word(after)
                && match before.kind {
                    TokenKind::Number => true,
                    TokenKind::Operator => matches!(first, "@" | ":"),
                    TokenKind::Punctuation => first.starts_with('$'),
                    _ => false,
                };
            // `limit$x`, `offset.1st`: the word is the first.
            let leads = word(before)
                && match after.kind {
                    TokenKind::Number => second.starts_with('.'),
                    TokenKind::Punctuation => second.starts_with('$'),
                    _ => false,
                };
            let bound = (follows || leads) && touching(&old_tokens, at);
            // The words' case is as it was, too.
            !bound
                || (touching(&new_tokens, at)
                    && first == &new[new_tokens[at - 1].range.clone()]
                    && second == &new[new_tokens[at].range.clone()])
        })
    }

    /// The whole of `script` formatted, or the script itself when Format
    /// has nothing to change. Checks what every result owes: the same
    /// tokens, and nothing left for a second pass.
    fn formatted(dialect: Dialect, script: &str) -> String {
        let (result, _) = laid(dialect, script, None);
        let checked = format(dialect, script, None, 0).map(|formatted| formatted.text);
        assert_eq!(checked.as_deref().unwrap_or(script), result, "{script:?}");
        // The second pass is the layout's too: `format` returns `None` as
        // well for a layout its check refused.
        assert_eq!(laid(dialect, &result, None).0, result, "{script:?}");
        result
    }

    fn pg(script: &str) -> String {
        formatted(Dialect::Postgres, script)
    }

    fn mysql(script: &str) -> String {
        formatted(Dialect::MySql, script)
    }

    fn sqlite(script: &str) -> String {
        formatted(Dialect::Sqlite, script)
    }

    /// `lines` joined by line feeds: a formatted script, a line a string.
    fn lines(lines: &[&str]) -> String {
        lines.join("\n")
    }

    #[test]
    fn the_first_word_of_each_head_ends_at_the_river() {
        let script = "select a.name as author, b.genre, count(*) as books from books b \
            left join authors a on a.id = b.author_id where b.published >= 2020 \
            and b.genre <> 'draft' group by a.name, b.genre order by b.genre, books desc limit 10;";
        assert_eq!(
            pg(script),
            lines(&[
                "SELECT a.name AS author,",
                "       b.genre,",
                "       count(*) AS books",
                "  FROM books b",
                "  LEFT JOIN authors a ON a.id = b.author_id",
                " WHERE b.published >= 2020",
                "   AND b.genre <> 'draft'",
                " GROUP BY a.name, b.genre",
                " ORDER BY b.genre, books DESC",
                " LIMIT 10;",
            ])
        );
        // Every other head; a word longer than the river starts at its base.
        let script = "select genre from books group by genre having count(*) > 1 \
            window w as (partition by genre) order by 1 limit 5 offset 2 \
            fetch first 3 rows only for update";
        assert_eq!(
            pg(script),
            lines(&[
                "SELECT genre",
                "  FROM books",
                " GROUP BY genre",
                "HAVING count(*) > 1",
                "WINDOW w AS (PARTITION BY genre)",
                " ORDER BY 1",
                " LIMIT 5",
                "OFFSET 2",
                " FETCH first 3 rows only",
                "   FOR UPDATE",
            ])
        );
    }

    #[test]
    fn a_run_of_join_leaders_is_one_head() {
        let script = "select * from a join b on a.id = b.id inner join c using (id) \
            left outer join d on true right join e on true full outer join f on true \
            cross join g natural left outer join h";
        assert_eq!(
            pg(script),
            lines(&[
                "SELECT *",
                "  FROM a",
                "  JOIN b ON a.id = b.id",
                " INNER JOIN c USING (id)",
                "  LEFT OUTER JOIN d ON TRUE",
                " RIGHT JOIN e ON TRUE",
                "  FULL OUTER JOIN f ON TRUE",
                " CROSS JOIN g",
                "NATURAL LEFT OUTER JOIN h",
            ])
        );
        // A leader with no JOIN after it is a word on its line.
        assert_eq!(
            pg("select left(title, 2), right(title, 1) from books"),
            lines(&[
                "SELECT LEFT(title, 2),",
                "       RIGHT(title, 1)",
                "  FROM books"
            ])
        );
        assert_eq!(
            mysql("select * from a straight_join b on a.id = b.id"),
            lines(&["SELECT *", "  FROM a", "STRAIGHT_JOIN b ON a.id = b.id"])
        );
    }

    #[test]
    fn and_and_or_start_lines_in_conditions_only() {
        let script = "select a and b, c or d from t join u on t.id = u.id and u.x or u.y \
            where p between 1 and 2 and q or r group by s having v and w";
        assert_eq!(
            pg(script),
            lines(&[
                "SELECT a AND b,",
                "       c OR d",
                "  FROM t",
                "  JOIN u ON t.id = u.id",
                "   AND u.x",
                "    OR u.y",
                " WHERE p BETWEEN 1 AND 2",
                "   AND q",
                "    OR r",
                " GROUP BY s",
                "HAVING v",
                "   AND w",
            ])
        );
        // Inside parentheses they stay on their line.
        assert_eq!(
            pg("select 1 where (a and b) or (c or d)"),
            lines(&["SELECT 1", " WHERE (a AND b)", "    OR (c OR d)"])
        );
        assert_eq!(
            pg("select 1 from t group by a having b or c"),
            lines(&[
                "SELECT 1",
                "  FROM t",
                " GROUP BY a",
                "HAVING b",
                "    OR c"
            ])
        );
    }

    #[test]
    fn only_the_select_and_with_lists_break() {
        assert_eq!(
            pg("select distinct a, f(b, c), d from t, u group by a, b order by a, b"),
            lines(&[
                "SELECT DISTINCT a,",
                "       f(b, c),",
                "       d",
                "  FROM t, u",
                " GROUP BY a, b",
                " ORDER BY a, b",
            ])
        );
        assert_eq!(pg("select * from t"), lines(&["SELECT *", "  FROM t"]));
        assert_eq!(
            pg(
                "with recursive a as (select 1), b(n) as materialized (select 2) select * from a, b"
            ),
            lines(&[
                "  WITH RECURSIVE a AS (SELECT 1),",
                "       b(n) AS materialized (SELECT 2)",
                "SELECT *",
                "  FROM a, b",
            ])
        );
    }

    #[test]
    fn a_query_in_parentheses_has_its_own_river() {
        assert_eq!(
            pg(
                "select title from books where author_id in (select id from authors \
                where country = 'PL') order by title;"
            ),
            lines(&[
                "SELECT title",
                "  FROM books",
                " WHERE author_id IN (SELECT id",
                "                       FROM authors",
                "                      WHERE country = 'PL')",
                " ORDER BY title;",
            ])
        );
        assert_eq!(
            pg(
                "with recent as (select id, title from books where published >= 2020) \
                select title from recent;"
            ),
            lines(&[
                "  WITH recent AS (SELECT id,",
                "                         title",
                "                    FROM books",
                "                   WHERE published >= 2020)",
                "SELECT title",
                "  FROM recent;",
            ])
        );
        // Two deep, in FROM and in a condition.
        assert_eq!(
            pg("select * from (select * from (select 1) x) y where exists \
                (select 1 from books where id in (select book_id from loans))"),
            lines(&[
                "SELECT *",
                "  FROM (SELECT *",
                "          FROM (SELECT 1) x) y",
                " WHERE EXISTS (SELECT 1",
                "                 FROM books",
                "                WHERE id IN (SELECT book_id",
                "                               FROM loans))",
            ])
        );
        // A WITH inside parentheses starts right after them.
        assert_eq!(
            pg("select * from (with c as (select 2) select * from c) d"),
            lines(&[
                "SELECT *",
                "  FROM (WITH c AS (SELECT 2)",
                "        SELECT *",
                "          FROM c) d",
            ])
        );
        // Columns are characters, not bytes.
        assert_eq!(
            pg("select żółw from t where 'é' in (select 1 from u)"),
            lines(&[
                "SELECT żółw",
                "  FROM t",
                " WHERE 'é' IN (SELECT 1",
                "                 FROM u)",
            ])
        );
    }

    #[test]
    fn nesting_without_end_stops_being_laid_out_as_blocks() {
        // Deeper than any stack could follow: the layout must not recurse
        // for each.
        let deep = 100_000;
        let script = "(select 1 from ".repeat(deep) + "t" + &")".repeat(deep);
        let formatted = format(Dialect::Postgres, &script, None, 0).unwrap();
        // Each block puts its FROM on a line; past the depth it is all on
        // the last one.
        assert_eq!(formatted.text.lines().count(), MAX_DEPTH + 1);
        let last = "(SELECT 1 FROM ".repeat(deep - MAX_DEPTH) + "t" + &")".repeat(deep);
        assert!(formatted.text.ends_with(&last));
        // Nor is a long run of join leaders read to its end for each word.
        let script = "select ".to_owned() + &"left ".repeat(deep) + "join t";
        assert!(format(Dialect::Postgres, &script, None, 0).is_some());
    }

    #[test]
    fn set_operations_are_heads() {
        assert_eq!(
            pg("select 1 union select 2 union all select 3 except select 4 intersect select 5"),
            lines(&[
                "SELECT 1",
                " UNION",
                "SELECT 2",
                " UNION ALL",
                "SELECT 3",
                "EXCEPT",
                "SELECT 4",
                "INTERSECT",
                "SELECT 5",
            ])
        );
        assert_eq!(
            pg("(select 1) union (select 2)"),
            lines(&["(SELECT 1)", " UNION (SELECT 2)"])
        );
        assert_eq!(
            pg("select 1 union distinct select 2"),
            lines(&["SELECT 1", " UNION DISTINCT", "SELECT 2"])
        );
    }

    #[test]
    fn a_case_puts_its_parts_on_lines() {
        assert_eq!(
            pg(
                "select title, case when stock = 0 then 'out' when stock < 5 and stock > 0 \
                then 'low' else 'ok' end as availability from books;"
            ),
            lines(&[
                "SELECT title,",
                "       CASE",
                "           WHEN stock = 0 THEN 'out'",
                "           WHEN stock < 5 AND stock > 0 THEN 'low'",
                "           ELSE 'ok'",
                "       END AS availability",
                "  FROM books;",
            ])
        );
        // An operand stays on the CASE's line; a CASE in a CASE has its
        // own column.
        assert_eq!(
            pg("select case genre when 'a' then case when x then 1 else 2 end else 3 end from t"),
            lines(&[
                "SELECT CASE genre",
                "           WHEN 'a' THEN CASE",
                "                             WHEN x THEN 1",
                "                             ELSE 2",
                "                         END",
                "           ELSE 3",
                "       END",
                "  FROM t",
            ])
        );
        // An END closes only a CASE of its own block: the table `end` in
        // the block inside is a plain word, in every dialect.
        for dialect in DIALECTS {
            assert_eq!(
                formatted(
                    dialect,
                    "select case when x in (select id from end) then 1 end from t"
                ),
                lines(&[
                    "SELECT CASE",
                    "           WHEN x IN (SELECT id",
                    "                        FROM end) THEN 1",
                    "       END",
                    "  FROM t",
                ]),
                "{dialect:?}"
            );
        }
    }

    #[test]
    fn heads_inside_parentheses_stay_on_their_line() {
        assert_eq!(
            pg(
                "select extract(year from added), count(*) over (partition by genre \
                order by title), id in (1, 2), trim(both from title) from books"
            ),
            lines(&[
                "SELECT extract(year FROM added),",
                "       count(*) over (PARTITION BY genre ORDER BY title),",
                "       id IN (1, 2),",
                "       trim(both FROM title)",
                "  FROM books",
            ])
        );
    }

    #[test]
    fn a_word_beside_a_dot_and_three_more_are_not_heads() {
        // A qualified name, whatever it spells, in every dialect.
        for dialect in DIALECTS {
            assert_eq!(
                formatted(
                    dialect,
                    "select r.from, order.id from shop.order where r.select = 1"
                ),
                lines(&[
                    "SELECT r.from,",
                    "       order.id",
                    "  FROM shop.order",
                    " WHERE r.select = 1",
                ]),
                "{dialect:?}"
            );
        }
        assert_eq!(
            pg("select 1 where a is distinct from b and c is not distinct from d"),
            lines(&[
                "SELECT 1",
                " WHERE a IS DISTINCT FROM b",
                "   AND c IS NOT DISTINCT FROM d",
            ])
        );
        assert_eq!(
            pg("select * from unnest(x) with ordinality as t(a, n)"),
            lines(&["SELECT *", "  FROM unnest(x) WITH ordinality AS t(a, n)"])
        );
        assert_eq!(
            mysql("select distinct straight_join a from t"),
            lines(&["SELECT DISTINCT STRAIGHT_JOIN a", "  FROM t"])
        );
    }

    #[test]
    fn a_word_that_touches_a_sigil_or_a_number_is_a_name() {
        // The tokenizer cuts `@from` into `@` and `from`; a server reads
        // one token. Whatever it spells, the word stays where it is and
        // as it is: a MySQL user variable,
        assert_eq!(
            mysql("select * from t where d between @from and @to and x = @limit"),
            lines(&[
                "SELECT *",
                "  FROM t",
                " WHERE d BETWEEN @from AND @to",
                "   AND x = @limit",
            ])
        );
        assert_eq!(
            mysql("select case when d < @end then 1 end, @@global.select"),
            lines(&[
                "SELECT CASE",
                "           WHEN d < @end THEN 1",
                "       END,",
                "       @@global.select",
            ])
        );
        assert_eq!(mysql("set @from = 1"), "SET @from = 1");
        // a parameter,
        assert_eq!(
            sqlite("select :limit, @from, $offset from t where a = ?1"),
            lines(&[
                "SELECT :limit,",
                "       @from,",
                "       $offset",
                "  FROM t",
                " WHERE a = ?1",
            ])
        );
        // and a name that starts with digits.
        assert_eq!(
            mysql("select 1from, 2 from t"),
            lines(&["SELECT 1from,", "       2", "  FROM t"])
        );
        // A word may lead such a name as well: SQLite's `$` inside one,
        // and a qualified name whose second part starts with a digit,
        // which the tokenizer reads as a number with its `.`.
        assert_eq!(
            sqlite("select a from t where limit$x > 0"),
            lines(&["SELECT a", "  FROM t", " WHERE limit$x > 0"])
        );
        assert_eq!(
            mysql("select offset.1st, order.2nd from t"),
            lines(&["SELECT offset.1st,", "       order.2nd", "  FROM t"])
        );
        // Apart from the sigil it is the word it spells.
        assert_eq!(pg("select 1 from t"), lines(&["SELECT 1", "  FROM t"]));
        assert_eq!(
            pg("select a::interval from t"),
            lines(&["SELECT a::interval", "  FROM t"])
        );
    }

    #[test]
    fn spacing_follows_what_was_typed_where_no_rule_sets_it() {
        // Tokens that touched still touch; a space that was typed stays
        // one space; commas and parentheses have their own rule.
        assert_eq!(
            pg("select  count( * ) ,f (x),a>=1 , - -1, 1 / *, price::text, a ->> 'b'  from  t"),
            lines(&[
                "SELECT count(*),",
                "       f (x),",
                "       a>=1,",
                "       - -1,",
                "       1 / *,",
                "       price::text,",
                "       a ->> 'b'",
                "  FROM t",
            ])
        );
        // A head is followed by one space, typed or not.
        assert_eq!(
            pg("select(1)from(select 2)t group by(a)"),
            lines(&["SELECT (1)", "  FROM (SELECT 2)t", " GROUP BY (a)"])
        );
        // PostgreSQL joins two strings only across a line break: what
        // stands between two strings is copied.
        assert_eq!(
            pg("select 'a'\r\n   'b', 'c'  'd' from t"),
            lines(&["SELECT 'a'\r", "   'b',", "       'c'  'd'", "  FROM t"])
        );
    }

    #[test]
    fn comments_keep_their_lines() {
        let script = lines(&[
            "-- top",
            "select a, -- one",
            "  -- own",
            "  b /* in */ , c",
            "  from t -- tail",
            "  where x in (1 -- last",
            "  ) and y = (select 1 -- q",
            "  )",
            "  /* block",
            "     two */",
            " order by 1 -- end",
            ";",
            "-- after",
        ]);
        assert_eq!(
            pg(&script),
            lines(&[
                "-- top",
                "SELECT a, -- one",
                "       -- own",
                "       b /* in */,",
                "       c",
                "  FROM t -- tail",
                " WHERE x IN (1 -- last",
                "            )",
                "   AND y = (SELECT 1 -- q",
                "           )",
                "       /* block",
                "     two */",
                " ORDER BY 1 -- end",
                ";",
                "-- after",
            ])
        );
        // What follows a comment that ends its line starts at the content
        // column, unless it is a head.
        assert_eq!(
            pg("select a -- one\n+ b from t"),
            lines(&["SELECT a -- one", "       + b", "  FROM t"])
        );
        // A comment before a statement on its line stays before it.
        assert_eq!(pg("/* a */   select 1"), "/* a */ SELECT 1");
        // Touching the code before it, it still touches: on MySQL a space
        // before `--` can turn two minus signs into a comment.
        assert_eq!(pg("select 1--x"), "SELECT 1--x");
        assert_eq!(mysql("select 1---- x"), "SELECT 1---- x");
        // Nor does a line start right after them there: a head that
        // touches them stays on their line.
        assert_eq!(
            mysql("select 1--from t where a--and b"),
            lines(&["SELECT 1--FROM t", " WHERE a--AND b"])
        );
        assert_eq!(
            pg("select 1 - -from t"),
            lines(&["SELECT 1 - -", "  FROM t"])
        );
    }

    #[test]
    fn statements_are_a_blank_line_apart() {
        assert_eq!(
            pg("select 1; select 2;\n\n\n\nselect 3"),
            lines(&["SELECT 1;", "", "SELECT 2;", "", "SELECT 3"])
        );
        // Around a comment between statements the blank lines stay as
        // typed, one at most; a comment after a statement stays on its
        // line.
        assert_eq!(
            pg(
                "select 1; -- a\nselect 2;\n-- b\nselect 3;\n\n\n\n-- c\n\n\nselect 4; /* d */ select 5"
            ),
            lines(&[
                "SELECT 1; -- a",
                "SELECT 2;",
                "-- b",
                "SELECT 3;",
                "",
                "-- c",
                "",
                "SELECT 4; /* d */",
                "SELECT 5",
            ])
        );
        // Whitespace around the script stays; the first line's
        // indentation is Format's.
        assert_eq!(pg("\n\n   select 1 ;  \n"), "\n\nSELECT 1;  \n");
        assert_eq!(pg("select 1;;"), "SELECT 1;;");
    }

    #[test]
    fn a_statement_that_is_no_query_keeps_its_lines() {
        let script = lines(&[
            "create table t (  ",
            "  a int,\t",
            "",
            "  b text default 'x'",
            ");",
            "explain select * from t;",
            "insert into t",
            "   values (1)   \r",
            "  returning id;",
            "show tables; values (1), (2)",
        ]);
        assert_eq!(
            pg(&script),
            lines(&[
                "CREATE TABLE t (",
                "  a int,",
                "",
                "  b text DEFAULT 'x'",
                ");",
                "",
                "EXPLAIN SELECT * FROM t;",
                "",
                "INSERT INTO t",
                "   VALUES (1)",
                "  RETURNING id;",
                "",
                "SHOW tables;",
                "",
                "VALUES (1), (2)",
            ])
        );
    }

    #[test]
    fn each_dialect_keeps_its_own_tokens() {
        assert_eq!(
            pg(
                "select $$ select\n  from $$, $tag$ a ; b $tag$, E'x'\n'y', a[1], $1 from t; select 2"
            ),
            lines(&[
                "SELECT $$ select",
                "  from $$,",
                "       $tag$ a ; b $tag$,",
                "       E'x'",
                "'y',",
                "       a[1],",
                "       $1",
                "  FROM t;",
                "",
                "SELECT 2",
            ])
        );
        // A backslash escapes the quote in an E string.
        assert_eq!(
            pg("select E'\\'' as q, 'x' from t where a = E'it\\'s' and b"),
            lines(&[
                "SELECT E'\\'' AS q,",
                "       'x'",
                "  FROM t",
                " WHERE a = E'it\\'s'",
                "   AND b",
            ])
        );
        // MySQL: `#` comments, `--` only before a blank, the `\r` a line
        // comment keeps, and an executable comment, which is code: a
        // statement that starts with one is no query.
        assert_eq!(
            mysql(
                "select `from`, \"str\" from t # c\nwhere a = 1 -- x\r\nand b = 1--1 /*! straight_join */;\r\n/*! select */ select 1\n from t"
            ),
            lines(&[
                "SELECT `from`,",
                "       \"str\"",
                "  FROM t # c",
                " WHERE a = 1 -- x\r",
                "   AND b = 1--1 /*! straight_join */;",
                "",
                "/*! select */ SELECT 1",
                " FROM t",
            ])
        );
        assert_eq!(
            sqlite("select [from], glob('a', b) from t; pragma table_info(books)"),
            lines(&[
                "SELECT [from],",
                "       GLOB('a', b)",
                "  FROM t;",
                "",
                "PRAGMA table_info(books)",
            ])
        );
    }

    #[test]
    fn reserved_words_are_uppercased_and_names_are_not() {
        // Every listed word, where it is a keyword.
        for dialect in DIALECTS {
            for word in UPPERCASED {
                let lower = word.to_ascii_lowercase();
                let script = std::format!("show {lower}");
                let expected = if is_keyword(dialect, word) {
                    std::format!("SHOW {word}")
                } else {
                    std::format!("SHOW {lower}")
                };
                assert_eq!(formatted(dialect, &script), expected, "{dialect:?}");
            }
        }
        // The keywords that can be names keep their case.
        let names =
            "any begin cast commit current end filter first no only over rollback rows view";
        for dialect in DIALECTS {
            assert_eq!(
                formatted(dialect, &std::format!("show {names}")),
                std::format!("SHOW {names}"),
                "{dialect:?}"
            );
        }
        // Read as structure they are uppercased, but not on MySQL, where
        // such a word may be an alias and an alias's case can matter.
        let script = "with recursive a as (select 1) select * from a full join b on true \
            window w as () limit 1 offset 2";
        for dialect in [Dialect::Postgres, Dialect::Sqlite] {
            assert_eq!(
                formatted(dialect, script),
                lines(&[
                    "  WITH RECURSIVE a AS (SELECT 1)",
                    "SELECT *",
                    "  FROM a",
                    "  FULL JOIN b ON TRUE",
                    "WINDOW w AS ()",
                    " LIMIT 1",
                    "OFFSET 2",
                ])
            );
        }
        assert_eq!(
            mysql(script),
            lines(&[
                "  WITH recursive a AS (SELECT 1)",
                "SELECT *",
                "  FROM a",
                "  full JOIN b ON TRUE",
                "window w AS ()",
                " LIMIT 1",
                "offset 2",
            ])
        );
        assert_eq!(
            pg("select 1 except select 2 intersect select 3"),
            lines(&["SELECT 1", "EXCEPT", "SELECT 2", "INTERSECT", "SELECT 3"])
        );
        assert_eq!(
            mysql("select 1 except select 2"),
            lines(&["SELECT 1", "except", "SELECT 2"])
        );
        // The END of a CASE is uppercased in every dialect.
        assert_eq!(
            mysql("select case when a then 1 end"),
            lines(&["SELECT CASE", "           WHEN a THEN 1", "       END"])
        );
        // A quoted name and a string that spell a keyword are untouched.
        assert_eq!(
            pg("select \"from\", 'select' from t"),
            lines(&["SELECT \"from\",", "       'select'", "  FROM t"])
        );
    }

    #[test]
    fn the_lists_hold_only_keywords_the_highlighter_knows() {
        for word in UPPERCASED
            .iter()
            .chain(STRUCTURE)
            .chain(HEADS)
            .chain(JOIN_LEADERS)
        {
            assert!(
                DIALECTS.iter().any(|dialect| is_keyword(*dialect, word)),
                "{word}"
            );
        }
        // A word is uppercased by the list or as structure, not both.
        assert!(STRUCTURE.iter().all(|word| !UPPERCASED.contains(word)));
    }

    #[test]
    fn awkward_scripts_keep_their_tokens() {
        // `formatted` checks the tokens and the second pass.
        let scripts = [
            "",
            " ",
            ";",
            ";;",
            "-- only a comment",
            "/* open",
            "select 'unterminated",
            "select \"unterminated",
            "select 1 -- no newline",
            "select ) from ( t",
            "select ((select 1)), (((2)))",
            "select case when a then case when b then 1",
            "select a end end from t",
            "select 1;\r\n\r\n-- the count\r\nselect count(*)\r\n  from books\r\n;\nselect 2",
            "select a - -1, b - - 2, c / * d, e /* x */ * f, 1--1, 1 -- 1\n, 2",
            "select 'a' 'b'\n'c', $$x$$ $$y$$, E'a'\n'b'",
            "ż ó\r\n\tł;;",
            "SELECT 1; SELECT 'a;b'; SELECT $$c;d$$; SELECT `e;f`",
            "select * from t where a = 1 and b between 2 and 3 and c or d",
            "with a as (select 1), b as (select 2) (select * from a) union (select * from b)",
            "select /*! straight_join */ 1 # x\nfrom t -- y\r\nwhere a",
            "select 1--from t where a--and b--or c--,d from--(select 1)--union select--case--when 1",
            "insert into t (a, b) select 1, 2 from u on conflict do nothing returning *",
        ];
        for dialect in DIALECTS {
            for script in scripts {
                formatted(dialect, script);
            }
        }
    }

    #[test]
    fn scripts_built_at_random_keep_their_tokens() {
        // What a script is made of, the awkward pieces included. A fixed
        // seed: the same scripts on every run.
        let words = "select from where and or between group order by join left outer on with \
            as case when then else end union distinct limit offset in exists insert values \
            straight_join natural full cross inner having window fetch for intersect except \
            all recursive is not over a b.c t x. .y 's' \"q\" `q` $$d$$ E'e' $1 1 2.5 \
            @from :limit $offset @end 1from limit$x offset.1st @ : $ .5 \
            ( ) [ ] , ; . * - / = : < > ! | # @ /*c*/ /*!e*/";
        let awkward = [
            "--x\n",
            "-- y\r\n",
            "# z\n",
            "/* m\n n */",
            "'a'\n'b'",
            " ",
            "\n",
            "\r\n",
            "\n\n\n",
            "\t",
        ];
        let pieces: Vec<&str> = words.split_whitespace().chain(awkward).collect();
        let mut seed = 0x9e37_79b9_7f4a_7c15_u64;
        let mut next = |below: usize| {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            (seed % below as u64) as usize
        };
        for _ in 0..2_000 {
            let mut script = String::new();
            for _ in 0..next(40) {
                script.push_str(pieces[next(pieces.len())]);
                // Mostly apart, sometimes touching.
                if next(4) > 0 {
                    script.push(' ');
                }
            }
            // The pieces are ASCII, so every offset is a character's.
            let (from, to) = (next(script.len() + 1), next(script.len() + 1));
            let selection = from.min(to)..from.max(to);
            for dialect in DIALECTS {
                formatted(dialect, &script);
                // A selection's statements: the same tokens, and laid out
                // again they stay as they are.
                let (result, part) = laid(dialect, &script, Some(selection.clone()));
                if !part.is_empty() {
                    assert_eq!(laid(dialect, &result, Some(part)).0, result, "{script:?}");
                }
            }
        }
    }

    #[test]
    fn the_check_tells_a_changed_token() {
        let same = |old: &str, new: &str| {
            let dialect = Dialect::Postgres;
            same_tokens(old, &tokenize(dialect, old), new, &tokenize(dialect, new))
        };
        assert!(same("select  a\nfrom t", "SELECT a FROM t"));
        // Two operators fused into a comment; a name's case; a missing
        // token; a string's contents.
        assert!(!same("select - -1", "select --1"));
        assert!(!same("select Books", "select books"));
        assert!(!same("select 1, 2", "select 1 2"));
        assert!(!same("select 'a  b'", "select 'a b'"));
    }

    #[test]
    fn nothing_to_change_is_none() {
        for script in [
            "",
            "  \n",
            "-- a note\n",
            "SELECT 1;",
            "SELECT a,\n       b\n  FROM t;\n\nSELECT 2;\n",
        ] {
            assert!(
                format(Dialect::Postgres, script, None, 0).is_none(),
                "{script:?}"
            );
        }
    }

    #[test]
    fn a_selection_formats_the_statements_it_overlaps() {
        let script = "select 1 ;\nselect a,b from t ;\n  select 3 ;";
        let second = script.find("a,b").unwrap();
        let text = |selection: Range<usize>| {
            format(Dialect::Postgres, script, Some(selection), 0).map(|formatted| formatted.text)
        };
        // One statement of three: the others are as typed.
        assert_eq!(
            text(second..second + 1).as_deref(),
            Some("select 1 ;\nSELECT a,\n       b\n  FROM t;\n  select 3 ;")
        );
        // Across two statements, from anywhere in the first to anywhere
        // in the second.
        assert_eq!(
            text(second..script.len() - 3).as_deref(),
            Some("select 1 ;\nSELECT a,\n       b\n  FROM t;\n\nSELECT 3;")
        );
        // In the whitespace between statements: nothing.
        let between = script.find(";\n").unwrap() + 1;
        assert_eq!(text(between..between + 1), None);
        // An empty selection is no selection.
        assert_eq!(
            text(second..second).as_deref(),
            Some("SELECT 1;\n\nSELECT a,\n       b\n  FROM t;\n\nSELECT 3;")
        );
        // A statement that does not start its line is laid out from
        // column 0 and its first line stays where it stood.
        let script = "select 1; select a,b";
        let formatted = format(Dialect::Postgres, script, Some(12..13), 0).unwrap();
        assert_eq!(formatted.text, "select 1; SELECT a,\n       b");
        // It goes on from where it stands: a head the river would pad is
        // not padded there, so a second press finds nothing to change.
        let script = "select 1; with a as (select 2) select * from a";
        let selection = script.find("with").unwrap()..script.len();
        let formatted = format(Dialect::Postgres, script, Some(selection.clone()), 0).unwrap();
        assert_eq!(
            formatted.text,
            "select 1; WITH a AS (SELECT 2)\nSELECT *\n  FROM a"
        );
        let selection = selection.start..formatted.text.len();
        assert!(format(Dialect::Postgres, &formatted.text, Some(selection), 0).is_none());
    }

    #[test]
    fn the_cursor_keeps_its_place_among_the_tokens() {
        let script = "  select a,b\n\n from t ; select 2";
        let moved = |cursor: usize| {
            let formatted = format(Dialect::Postgres, script, None, cursor).unwrap();
            assert_eq!(formatted.text, "SELECT a,\n       b\n  FROM t;\n\nSELECT 2");
            formatted.cursor
        };
        // Before the script's first token, in what was its indentation.
        assert_eq!(moved(0), 0);
        assert_eq!(moved(1), 0);
        // In a word, and at a word's end.
        assert_eq!(moved(4), 2);
        assert_eq!(moved(8), 6);
        // Right after a token it stays after it; from whitespace it goes
        // before the next token.
        assert_eq!(moved(11), 9);
        assert_eq!(moved(12), 18);
        assert_eq!(moved(13), 21);
        assert_eq!(moved(15), 21);
        // At the script's end.
        assert_eq!(moved(script.len()), 38);
        // Outside a selection's statements it moves with the text.
        let script = "select 1 ;\nselect a,b ;\n-- end";
        let formatted = format(Dialect::Postgres, script, Some(12..13), script.len()).unwrap();
        assert_eq!(formatted.text, "select 1 ;\nSELECT a,\n       b;\n-- end");
        assert_eq!(formatted.cursor, formatted.text.len());
        let formatted = format(Dialect::Postgres, script, Some(12..13), 3).unwrap();
        assert_eq!(formatted.cursor, 3);
    }
}

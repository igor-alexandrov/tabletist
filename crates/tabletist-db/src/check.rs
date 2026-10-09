//! The value list a CHECK constraint allows, when the constraint is nothing
//! but that list: `col IN ('a', 'b')` or `col = ANY (ARRAY['a', 'b'])`, as
//! written or as PostgreSQL prints it back (`(col)::text = ANY
//! ((ARRAY['a'::character varying, ...])::text[])`).

/// The values `expr` allows `column`, in the constraint's order, or `None`
/// when the constraint is anything more than a plain list of strings.
pub(crate) fn allowed_values(expr: &str, column: &str) -> Option<Vec<String>> {
    let mut parser = Parser { rest: expr };
    parser.word("CHECK");
    let values = parser.expr(column)?;
    parser.space();
    if !parser.rest.is_empty() || values.is_empty() {
        return None;
    }
    let mut unique: Vec<String> = Vec::with_capacity(values.len());
    for value in values {
        if !unique.contains(&value) {
            unique.push(value);
        }
    }
    Some(unique)
}

/// As [`allowed_values`], of a CHECK's condition as MySQL keeps it in
/// `information_schema.check_constraints`: escaped once more than it was
/// written, with its names in backticks and each string behind its
/// character set. It is written over into what the parser reads.
pub(crate) fn mysql_allowed_values(clause: &str, column: &str) -> Option<Vec<String>> {
    // MySQL's own escaping leaves no quote without a backslash before it.
    // A clause with a bare one is not in that form (MariaDB keeps it as
    // written) and is read as it stands.
    let mut before = ' ';
    let escaped = clause.chars().all(|character| {
        let bare = character == '\'' && before != '\\';
        before = character;
        !bare
    });
    let clause = if escaped {
        let mut plain = String::with_capacity(clause.len());
        let mut characters = clause.chars();
        while let Some(character) = characters.next() {
            match character {
                '\\' => plain.push(characters.next()?),
                other => plain.push(other),
            }
        }
        plain
    } else {
        clause.to_owned()
    };
    // The clause has the column's name as the constraint wrote it, and
    // MySQL matches it without regard to its case: both in lower case.
    allowed_values(&from_mysql(&clause)?, &column.to_lowercase())
}

/// A MySQL condition in the parser's own writing: a name in double quotes
/// and in lower case, a string with its quotes doubled and nothing before
/// it. `None` for a string with an escape that stands for another character
/// (`\n`), which is no plain value.
fn from_mysql(clause: &str) -> Option<String> {
    let mut written = String::with_capacity(clause.len());
    let mut rest = clause;
    while let Some(character) = rest.chars().next() {
        if character == '`' {
            // A name: a doubled backtick is one of its own.
            let mut name = String::new();
            let mut body = rest[1..].char_indices().peekable();
            let mut end = None;
            while let Some((at, letter)) = body.next() {
                if letter != '`' {
                    name.push(letter);
                } else if body.next_if(|(_, next)| *next == '`').is_some() {
                    name.push('`');
                } else {
                    end = Some(at + 2);
                    break;
                }
            }
            written.push('"');
            written.push_str(&name.to_lowercase().replace('"', "\"\""));
            written.push('"');
            rest = &rest[end?..];
        } else if character == '\'' {
            // A string: `\'` and `\\` are the quote and the backslash.
            let mut value = String::new();
            let mut body = rest[1..].char_indices().peekable();
            let mut end = None;
            while let Some((at, letter)) = body.next() {
                match letter {
                    '\\' => match body.next()?.1 {
                        escaped @ ('\'' | '\\') => value.push(escaped),
                        _ => return None,
                    },
                    '\'' if body.next_if(|(_, next)| *next == '\'').is_some() => value.push('\''),
                    '\'' => {
                        end = Some(at + 2);
                        break;
                    }
                    other => value.push(other),
                }
            }
            written.push('\'');
            written.push_str(&value.replace('\'', "''"));
            written.push('\'');
            rest = &rest[end?..];
        } else if character == '_' && introduces(rest) {
            // A string's character set (`_utf8mb4'...'`): not part of it.
            rest = &rest[rest.find('\'')?..];
        } else {
            written.push(character);
            rest = &rest[character.len_utf8()..];
        }
    }
    Some(written)
}

/// Whether `rest` begins with a character set's name right before a
/// string: `_utf8mb4'`. MySQL writes a column's name in backticks, so a
/// bare `_word'` can only be one.
fn introduces(rest: &str) -> bool {
    let name = rest[1..]
        .find(|character: char| !character.is_ascii_alphanumeric())
        .map(|end| &rest[1..][end..]);
    name.is_some_and(|after| after.starts_with('\''))
}

#[derive(Clone, Copy)]
struct Parser<'a> {
    rest: &'a str,
}

impl<'a> Parser<'a> {
    fn space(&mut self) {
        self.rest = self.rest.trim_start();
    }

    /// Consumes `token` (after any space) if it comes next.
    fn token(&mut self, token: &str) -> bool {
        self.space();
        match self.rest.strip_prefix(token) {
            Some(rest) => {
                self.rest = rest;
                true
            }
            None => false,
        }
    }

    /// Consumes the keyword `word`, in any case, if it comes next as a whole
    /// word.
    fn word(&mut self, word: &str) -> bool {
        self.space();
        let len = word.len();
        let matches = self
            .rest
            .get(..len)
            .is_some_and(|head| head.eq_ignore_ascii_case(word))
            && !self.rest[len..].starts_with(is_ident_char);
        if matches {
            self.rest = &self.rest[len..];
        }
        matches
    }

    /// Runs `step` on a copy and keeps its progress only if it succeeds.
    fn attempt<T>(&mut self, step: impl FnOnce(&mut Self) -> Option<T>) -> Option<T> {
        let mut copy = *self;
        let result = step(&mut copy)?;
        *self = copy;
        Some(result)
    }

    /// The whole condition, in any number of parentheses.
    fn expr(&mut self, column: &str) -> Option<Vec<String>> {
        if let Some(values) = self.attempt(|p| p.comparison(column)) {
            return Some(values);
        }
        self.attempt(|p| {
            p.token("(").then_some(())?;
            let values = p.expr(column)?;
            p.token(")").then_some(values)
        })
    }

    fn comparison(&mut self, column: &str) -> Option<Vec<String>> {
        self.column(column)?;
        if self.word("IN") {
            self.token("(").then_some(())?;
            let values = self.list()?;
            return self.token(")").then_some(values);
        }
        self.token("=").then_some(())?;
        if self.word("ANY") {
            self.token("(").then_some(())?;
            let values = self.array()?;
            return self.token(")").then_some(values);
        }
        // `col IN ('a')` is stored as `col = 'a'`.
        Some(vec![self.literal()?])
    }

    /// The column, possibly parenthesised and cast.
    fn column(&mut self, column: &str) -> Option<()> {
        self.space();
        if self.token("(") {
            self.column(column)?;
            self.token(")").then_some(())?;
        } else if self.ident()? != column {
            return None;
        }
        self.casts()
    }

    fn array(&mut self) -> Option<Vec<String>> {
        let values = if self.token("(") {
            let values = self.array()?;
            self.token(")").then_some(values)?
        } else {
            self.word("ARRAY").then_some(())?;
            self.token("[").then_some(())?;
            let values = self.list()?;
            self.token("]").then_some(values)?
        };
        self.casts()?;
        Some(values)
    }

    fn list(&mut self) -> Option<Vec<String>> {
        let mut values = vec![self.literal()?];
        while self.token(",") {
            values.push(self.literal()?);
        }
        Some(values)
    }

    /// A string literal, possibly parenthesised and cast.
    fn literal(&mut self) -> Option<String> {
        let value = if self.token("(") {
            let value = self.literal()?;
            self.token(")").then_some(value)?
        } else {
            self.string()?
        };
        self.casts()?;
        Some(value)
    }

    /// `'...'` with `''` for a quote. Escape strings (`E'...'`) are not
    /// plain values.
    fn string(&mut self) -> Option<String> {
        self.space();
        let mut chars = self.rest.strip_prefix('\'')?.char_indices();
        let mut value = String::new();
        let body = &self.rest[1..];
        while let Some((index, character)) = chars.next() {
            if character != '\'' {
                value.push(character);
            } else if body[index + 1..].starts_with('\'') {
                value.push('\'');
                chars.next();
            } else {
                self.rest = &body[index + 1..];
                return Some(value);
            }
        }
        None
    }

    /// An identifier: `"quoted ""name"""` as written, or a bare one folded
    /// to lower case.
    fn ident(&mut self) -> Option<String> {
        self.space();
        if let Some(body) = self.rest.strip_prefix('"') {
            let mut chars = body.char_indices();
            let mut name = String::new();
            while let Some((index, character)) = chars.next() {
                if character != '"' {
                    name.push(character);
                } else if body[index + 1..].starts_with('"') {
                    name.push('"');
                    chars.next();
                } else {
                    self.rest = &body[index + 1..];
                    return Some(name);
                }
            }
            return None;
        }
        let end = self
            .rest
            .find(|character: char| !is_ident_char(character))
            .unwrap_or(self.rest.len());
        let name = &self.rest[..end];
        if name.is_empty() || name.starts_with(|c: char| c.is_ascii_digit()) {
            return None;
        }
        self.rest = &self.rest[end..];
        Some(name.to_lowercase())
    }

    /// Any number of `::type` casts: `::text`, `::character varying(20)`,
    /// `::text[]`.
    fn casts(&mut self) -> Option<()> {
        while self.token("::") {
            self.ident()?;
            while self.token(".") {
                self.ident()?;
            }
            self.word("varying");
            if self.token("(") {
                self.space();
                let digits = self.rest.trim_start_matches(|c: char| c.is_ascii_digit());
                (digits.len() < self.rest.len()).then_some(())?;
                self.rest = digits;
                self.token(")").then_some(())?;
            }
            while self.token("[") {
                self.token("]").then_some(())?;
            }
        }
        Some(())
    }
}

fn is_ident_char(character: char) -> bool {
    character.is_alphanumeric() || character == '_' || character == '$'
}

#[cfg(test)]
mod tests {
    use super::allowed_values;

    fn values(expr: &str, column: &str) -> Option<Vec<String>> {
        allowed_values(expr, column)
    }

    fn list(items: &[&str]) -> Option<Vec<String>> {
        Some(items.iter().map(|item| (*item).to_owned()).collect())
    }

    #[test]
    fn in_lists_parse_as_written_and_as_postgres_prints_them() {
        assert_eq!(
            values("CHECK (status IN ('draft', 'live'))", "status"),
            list(&["draft", "live"])
        );
        assert_eq!(
            values(
                "((status)::text = ANY ((ARRAY['draft'::character varying, 'live'::character varying])::text[]))",
                "status"
            ),
            list(&["draft", "live"])
        );
        assert_eq!(
            values("status in ('b','a')", "status"),
            list(&["b", "a"]),
            "the constraint's order, not sorted"
        );
    }

    #[test]
    fn any_arrays_parse_as_written_and_as_postgres_prints_them() {
        assert_eq!(
            values("CHECK (kind = ANY (ARRAY['cover', 'preview']))", "kind"),
            list(&["cover", "preview"])
        );
        assert_eq!(
            values(
                "(kind = ANY (ARRAY['cover'::text, 'preview'::text]))",
                "kind"
            ),
            list(&["cover", "preview"])
        );
        assert_eq!(
            values(
                "((kind)::text = ANY (ARRAY[('a'::character varying(20))::text]))",
                "kind"
            ),
            list(&["a"])
        );
    }

    #[test]
    fn a_single_value_in_list_is_stored_as_equality() {
        assert_eq!(values("(c = 'm'::text)", "c"), list(&["m"]));
    }

    #[test]
    fn quotes_names_and_duplicates() {
        assert_eq!(
            values(
                r#"("Odd Col" = ANY (ARRAY['it''s'::text, 'a\b'::text]))"#,
                "Odd Col"
            ),
            list(&["it's", r"a\b"])
        );
        assert_eq!(values("STATUS IN ('x')", "status"), list(&["x"]));
        assert_eq!(values("(s IN ('x', 'y', 'x'))", "s"), list(&["x", "y"]));
    }

    #[test]
    fn a_list_parses_as_mysql_keeps_it() {
        use super::mysql_allowed_values as values;
        // As `information_schema.check_constraints` holds it (MySQL 8.4).
        let kept = r"(`kind` in (_utf8mb4\'print\',_utf8mb4\'e\\\'book\',_utf8mb4\'au\\\\dio\'))";
        assert_eq!(values(kept, "kind"), list(&["print", "e'book", r"au\dio"]));
        // Another column's list is not this one's.
        assert_eq!(values(kept, "format"), None);
        // A name with a space, as it was declared.
        assert_eq!(
            values(r"(`odd name` in (_utf8mb4\'x\',_utf8mb4\'y\'))", "odd name"),
            list(&["x", "y"])
        );
        // The name is kept as the constraint wrote it, and MySQL matches a
        // column's without regard to its case.
        assert_eq!(
            values(r"(`FORMAT` in (_utf8mb4\'a\',_utf8mb4\'b\'))", "Format"),
            list(&["a", "b"])
        );
        // A list of one is kept as an equality.
        assert_eq!(
            values(r"(`kind` = _utf8mb4\'print\')", "kind"),
            list(&["print"])
        );
        // Anything more than a plain list is not one.
        assert_eq!(values("(`n` > 0)", "n"), None);
        assert_eq!(values("(`n` in (1,2))", "n"), None);
        assert_eq!(
            values(
                r"((`kind` = _utf8mb4\'print\') or (`kind` = _utf8mb4\'ebook\'))",
                "kind"
            ),
            None
        );
        // An escape that stands for another character is no plain value.
        assert_eq!(values(r"(`kind` in (_utf8mb4\'a\\nb\'))", "kind"), None);
        // A clause that is not escaped (MariaDB's) is read as it stands.
        assert_eq!(
            values("`kind` in ('print','ebook')", "kind"),
            list(&["print", "ebook"])
        );
    }

    #[test]
    fn anything_more_than_a_plain_list_is_not_one() {
        for expr in [
            "((c = 'm'::text) OR (c IS NULL))",
            "(char_length(e) > 1)",
            "((d)::text = ANY ('{u,v}'::text[]))",
            "(other IN ('a', 'b'))",
            "(c IN ('a', NULL))",
            "(c IN (E'a\\n', 'b'))",
            "(c IN ('a', 'b')) AND (c <> 'a')",
            "(c NOT IN ('a'))",
            "(c IN ('a', 1))",
            "(c IN ())",
            "(c IN ('a')",
            "(c IN ('unterminated))",
            "(lower(c) IN ('a'))",
            "",
        ] {
            assert_eq!(values(expr, "c"), None, "{expr}");
        }
    }
}

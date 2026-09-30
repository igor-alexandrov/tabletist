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

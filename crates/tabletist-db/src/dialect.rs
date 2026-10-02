//! SQL for each database: identifier quoting, placeholders, filters, paging.

use std::fmt::Write as _;

use crate::{Filter, FilterOp, ObjectRef, RowQuery, SortDir, Value};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dialect {
    Postgres,
    MySql,
    Sqlite,
}

/// SQL text with its bound parameters.
#[derive(Debug, Clone, PartialEq)]
pub struct Sql {
    pub text: String,
    pub params: Vec<Value>,
}

/// Escapes `\`, `%` and `_` so `text` matches literally inside a LIKE
/// pattern whose escape character is `\`.
pub fn escape_like(text: &str) -> String {
    let mut escaped = String::with_capacity(text.len());
    for character in text.chars() {
        if matches!(character, '\\' | '%' | '_') {
            escaped.push('\\');
        }
        escaped.push(character);
    }
    escaped
}

/// A PostgreSQL escape-string literal (`E'...'`) with `\` and `'` doubled.
/// It reads the same whatever `standard_conforming_strings` is, so a pooler
/// that hands out a session with the setting off cannot turn a backslash in
/// the value into an escape that ends the literal early.
pub fn quote_literal(text: &str) -> String {
    format!("E'{}'", text.replace('\\', "\\\\").replace('\'', "''"))
}

/// Whether SQLite reads `text` as ending inside a `/* ... */` comment.
/// SQLite accepts an unterminated block comment and ignores everything
/// after it, which in a raw WHERE would swallow the builder's ORDER BY,
/// LIMIT and OFFSET. Quoted strings and names, and `--` comments, are
/// skipped the way SQLite's tokenizer skips them.
pub fn sqlite_ends_in_block_comment(text: &str) -> bool {
    let mut chars = text.chars().peekable();
    while let Some(character) = chars.next() {
        match character {
            '\'' | '"' | '`' => loop {
                match chars.next() {
                    // Unterminated: SQLite rejects it anyway.
                    None => return false,
                    Some(next) if next == character => {
                        // A doubled quote stays inside.
                        if chars.peek() != Some(&character) {
                            break;
                        }
                        chars.next();
                    }
                    Some(_) => {}
                }
            },
            '[' => {
                if !chars.by_ref().any(|next| next == ']') {
                    return false;
                }
            }
            '-' if chars.peek() == Some(&'-') => {
                if !chars.by_ref().any(|next| next == '\n') {
                    return false;
                }
            }
            '/' if chars.peek() == Some(&'*') => {
                chars.next();
                let mut previous = None;
                let closed = chars.by_ref().any(|next| {
                    let closes = previous == Some('*') && next == '/';
                    previous = Some(next);
                    closes
                });
                if !closed {
                    return true;
                }
            }
            _ => {}
        }
    }
    false
}

/// The bytes `text` stands for when it is written the way the app shows
/// binary: `0x` hex, or a UUID (hyphenated, or its 32 hex digits) for the
/// sixteen bytes of a `blob(16)` or `binary(16)` key.
fn bytes_from_text(text: &str) -> Option<Vec<u8>> {
    let text = text.trim();
    let hex: String = if let Some(hex) = text.strip_prefix("0x").or(text.strip_prefix("0X")) {
        hex.to_owned()
    } else if text.len() == 36 {
        let mut groups = text.split('-');
        let hex: String = [8, 4, 4, 4, 12]
            .into_iter()
            .map(|len| groups.next().filter(|group| group.len() == len))
            .collect::<Option<_>>()?;
        groups.next().is_none().then_some(hex)?
    } else if text.len() == 32 {
        text.to_owned()
    } else {
        return None;
    };
    if !hex.len().is_multiple_of(2) || !hex.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return None;
    }
    (0..hex.len())
        .step_by(2)
        .map(|index| u8::from_str_radix(&hex[index..index + 2], 16).ok())
        .collect()
}

/// The values of an In filter: split on commas, blanks dropped.
fn in_values(value: &str) -> impl Iterator<Item = &str> {
    value
        .split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

/// Whether a filter of `query` would compare bytes if its column held
/// them. Only then does a driver need the table's binary columns, which
/// costs it a catalog query.
pub(crate) fn reads_bytes(query: &RowQuery) -> bool {
    query.filters.iter().any(|filter| match filter.op {
        FilterOp::Eq | FilterOp::Ne => bytes_from_text(&filter.value).is_some(),
        FilterOp::In => in_values(&filter.value).any(|value| bytes_from_text(value).is_some()),
        _ => false,
    })
}

impl Dialect {
    pub fn quote_ident(self, ident: &str) -> String {
        match self {
            Self::MySql => format!("`{}`", ident.replace('`', "``")),
            Self::Postgres | Self::Sqlite => format!("\"{}\"", ident.replace('"', "\"\"")),
        }
    }

    /// `name` as it is written into a statement the user reads: bare when
    /// that is safe, quoted otherwise. Safe is a plain word the dialect
    /// does not reserve. PostgreSQL folds a bare name to lower case, so
    /// there a plain word is in lower case. The rule errs toward quoting:
    /// a quoted name always works.
    pub fn ident(self, name: &str) -> std::borrow::Cow<'_, str> {
        let word = |byte: u8| match self {
            Self::Postgres => byte.is_ascii_lowercase() || byte == b'_',
            Self::MySql | Self::Sqlite => byte.is_ascii_alphabetic() || byte == b'_',
        };
        let mut bytes = name.bytes();
        let plain =
            bytes.next().is_some_and(word) && bytes.all(|byte| word(byte) || byte.is_ascii_digit());
        if plain && !crate::reserved::is_reserved(self, name) {
            std::borrow::Cow::Borrowed(name)
        } else {
            std::borrow::Cow::Owned(self.quote_ident(name))
        }
    }

    pub fn qualified(self, object: &ObjectRef) -> String {
        format!(
            "{}.{}",
            self.quote_ident(&object.schema),
            self.quote_ident(&object.name)
        )
    }

    /// MySQL and SQLite bind parameters with `?`.
    fn placeholder(self) -> &'static str {
        "?"
    }

    fn as_text(self, column: &str) -> String {
        match self {
            Self::MySql => format!("CAST({column} AS CHAR)"),
            Self::Postgres | Self::Sqlite => format!("CAST({column} AS TEXT)"),
        }
    }

    fn like(self) -> &'static str {
        match self {
            Self::Postgres => "ILIKE",
            Self::MySql | Self::Sqlite => "LIKE",
        }
    }

    /// PostgreSQL and MySQL already treat `\` as LIKE's escape character.
    fn like_escape(self) -> &'static str {
        match self {
            Self::Sqlite => " ESCAPE '\\'",
            Self::Postgres | Self::MySql => "",
        }
    }

    /// A filter value in SQL. PostgreSQL rows are read through the
    /// simple-query protocol, which has no parameters, so its values become
    /// escape-string literals that do not depend on the session's
    /// `standard_conforming_strings`. The others bind.
    fn bind(self, params: &mut Vec<Value>, value: String) -> String {
        match self {
            Self::Postgres => quote_literal(&value),
            Self::MySql | Self::Sqlite => {
                params.push(Value::Text(value.into()));
                self.placeholder().to_owned()
            }
        }
    }

    /// Bytes as a filter value: bound, or for PostgreSQL a `bytea` literal
    /// in hex.
    fn bind_bytes(self, params: &mut Vec<Value>, bytes: Vec<u8>) -> String {
        match self {
            Self::Postgres => {
                let mut hex = String::from("\\x");
                for byte in bytes {
                    let _ = write!(hex, "{byte:02x}");
                }
                quote_literal(&hex)
            }
            Self::MySql | Self::Sqlite => {
                params.push(Value::Bytes(bytes.into()));
                self.placeholder().to_owned()
            }
        }
    }

    /// What an Eq, Ne or In filter compares its column with: the value as
    /// typed, and in a binary column first the bytes it reads as. A UUID or
    /// `0x` hex as text never equals the bytes the grid shows that way. The
    /// text stays, since such a column may hold the text itself.
    fn operands(self, value: &str, binary: bool, params: &mut Vec<Value>) -> Vec<String> {
        let mut operands = Vec::with_capacity(2);
        if binary && let Some(bytes) = bytes_from_text(value) {
            operands.push(self.bind_bytes(params, bytes));
        }
        operands.push(self.bind(params, value.to_owned()));
        operands
    }

    /// `binary` names the table's columns that hold bytes.
    fn filter(self, filter: &Filter, binary: &[String], params: &mut Vec<Value>) -> String {
        let column = self.quote_ident(&filter.column);
        let compare = |op: &str, params: &mut Vec<Value>| {
            let placeholder = self.bind(params, filter.value.clone());
            format!("{column} {op} {placeholder}")
        };
        let binary = binary.contains(&filter.column);
        let equals = |op: &str, list: &str, params: &mut Vec<Value>| {
            let operands = self.operands(&filter.value, binary, params);
            match operands.as_slice() {
                [operand] => format!("{column} {op} {operand}"),
                _ => format!("{column} {list} ({})", operands.join(", ")),
            }
        };
        match filter.op {
            FilterOp::Eq => equals("=", "IN", params),
            FilterOp::Ne => equals("<>", "NOT IN", params),
            FilterOp::Lt => compare("<", params),
            FilterOp::Gt => compare(">", params),
            FilterOp::Le => compare("<=", params),
            FilterOp::Ge => compare(">=", params),
            FilterOp::IsNull => format!("{column} IS NULL"),
            FilterOp::IsNotNull => format!("{column} IS NOT NULL"),
            FilterOp::Contains | FilterOp::StartsWith => {
                let escaped = escape_like(&filter.value);
                let pattern = if filter.op == FilterOp::Contains {
                    format!("%{escaped}%")
                } else {
                    format!("{escaped}%")
                };
                let placeholder = self.bind(params, pattern);
                format!(
                    "{} {} {placeholder}{}",
                    self.as_text(&column),
                    self.like(),
                    self.like_escape()
                )
            }
            FilterOp::In => {
                let placeholders: Vec<String> = in_values(&filter.value)
                    .flat_map(|value| self.operands(value, binary, params))
                    .collect();
                if placeholders.is_empty() {
                    return "1 = 0".into();
                }
                format!("{column} IN ({})", placeholders.join(", "))
            }
        }
    }

    fn where_clause(self, query: &RowQuery, binary: &[String], params: &mut Vec<Value>) -> String {
        let mut conditions: Vec<String> = query
            .filters
            .iter()
            .map(|filter| self.filter(filter, binary, params))
            .collect();
        if let Some(raw) = query.raw_where.as_deref().map(str::trim)
            && !raw.is_empty()
        {
            // On its own lines, so a trailing `--` comment in the raw text
            // cannot reach the builder's LIMIT. An unterminated `/*` still
            // could: PostgreSQL and MySQL reject one, SQLite does not, so the
            // SQLite adapter refuses such text first
            // (`sqlite_ends_in_block_comment`).
            conditions.push(format!("(\n{raw}\n)"));
        }
        if conditions.is_empty() {
            String::new()
        } else {
            format!(" WHERE {}", conditions.join(" AND "))
        }
    }

    /// One page plus one row (to learn whether there is a next page).
    /// `key` is the primary key, used for a stable default order and as a
    /// tiebreaker after the user's sort. `binary` names the columns that
    /// hold bytes: a filter value written as a UUID or as `0x` hex is
    /// compared with them as those bytes.
    pub fn select_rows(self, query: &RowQuery, key: &[String], binary: &[String]) -> Sql {
        let mut params = Vec::new();
        let mut text = format!("SELECT * FROM {}", self.qualified(&query.object));
        text.push_str(&self.where_clause(query, binary, &mut params));
        let mut order: Vec<String> = query
            .sort
            .iter()
            .map(|sort| {
                let dir = match sort.dir {
                    SortDir::Asc => "ASC",
                    SortDir::Desc => "DESC",
                };
                format!("{} {dir}", self.quote_ident(&sort.column))
            })
            .collect();
        for column in key {
            if !query.sort.iter().any(|sort| &sort.column == column) {
                order.push(format!("{} ASC", self.quote_ident(column)));
            }
        }
        if !order.is_empty() {
            text.push_str(" ORDER BY ");
            text.push_str(&order.join(", "));
        }
        text.push_str(&format!(
            " LIMIT {} OFFSET {}",
            u64::from(query.limit) + 1,
            query.offset
        ));
        Sql { text, params }
    }

    /// `binary` as for [`Dialect::select_rows`].
    pub fn count_rows(self, query: &RowQuery, binary: &[String]) -> Sql {
        let mut params = Vec::new();
        let mut text = format!("SELECT count(*) FROM {}", self.qualified(&query.object));
        text.push_str(&self.where_clause(query, binary, &mut params));
        Sql { text, params }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Filter, FilterOp, ObjectRef, RowQuery, Sort, SortDir, Value};

    fn query() -> RowQuery {
        RowQuery::new(ObjectRef::new("public", "users"), 300)
    }

    fn text(value: &str) -> Value {
        Value::Text(value.into())
    }

    #[test]
    fn identifiers_with_quotes_are_doubled() {
        assert_eq!(
            Dialect::Postgres.quote_ident(r#"weird "name""#),
            r#""weird ""name""""#
        );
        assert_eq!(Dialect::Sqlite.quote_ident("select"), r#""select""#);
        assert_eq!(Dialect::MySql.quote_ident("a`b"), "`a``b`");
        assert_eq!(
            Dialect::MySql.qualified(&ObjectRef::new("shop", "orders")),
            "`shop`.`orders`"
        );
    }

    #[test]
    fn a_plain_page_orders_by_the_key_and_asks_for_one_extra_row() {
        let sql = Dialect::Postgres.select_rows(&query(), &["id".into()], &[]);
        assert_eq!(
            sql.text,
            r#"SELECT * FROM "public"."users" ORDER BY "id" ASC LIMIT 301 OFFSET 0"#
        );
        assert!(sql.params.is_empty());
    }

    #[test]
    fn without_a_key_there_is_no_order() {
        let mut q = query();
        q.offset = 600;
        let sql = Dialect::Sqlite.select_rows(&q, &[], &[]);
        assert_eq!(
            sql.text,
            r#"SELECT * FROM "public"."users" LIMIT 301 OFFSET 600"#
        );
    }

    #[test]
    fn user_sorts_come_first_with_key_tiebreakers() {
        let mut q = query();
        q.sort = vec![Sort {
            column: "email".into(),
            dir: SortDir::Desc,
        }];
        let sql = Dialect::MySql.select_rows(&q, &["id".into()], &[]);
        assert_eq!(
            sql.text,
            "SELECT * FROM `public`.`users` ORDER BY `email` DESC, `id` ASC LIMIT 301 OFFSET 0"
        );
    }

    #[test]
    fn comparison_filters_bind_parameters_with_dialect_placeholders() {
        let mut q = query();
        q.filters = vec![
            Filter {
                column: "age".into(),
                op: FilterOp::Ge,
                value: "18".into(),
            },
            Filter {
                column: "name".into(),
                op: FilterOp::Ne,
                value: "bob".into(),
            },
        ];
        let pg = Dialect::Postgres.select_rows(&q, &[], &[]);
        assert_eq!(
            pg.text,
            r#"SELECT * FROM "public"."users" WHERE "age" >= E'18' AND "name" <> E'bob' LIMIT 301 OFFSET 0"#
        );
        assert!(pg.params.is_empty());
        let my = Dialect::MySql.select_rows(&q, &[], &[]);
        assert!(
            my.text.contains("WHERE `age` >= ? AND `name` <> ?"),
            "{}",
            my.text
        );
        assert_eq!(my.params, vec![text("18"), text("bob")]);
    }

    #[test]
    fn null_filters_take_no_parameter() {
        let mut q = query();
        q.filters = vec![
            Filter {
                column: "deleted_at".into(),
                op: FilterOp::IsNull,
                value: "ignored".into(),
            },
            Filter {
                column: "email".into(),
                op: FilterOp::IsNotNull,
                value: String::new(),
            },
        ];
        let sql = Dialect::Sqlite.select_rows(&q, &[], &[]);
        assert!(
            sql.text
                .contains(r#"WHERE "deleted_at" IS NULL AND "email" IS NOT NULL"#)
        );
        assert!(sql.params.is_empty());
    }

    #[test]
    fn in_filters_split_on_commas_and_an_empty_list_matches_nothing() {
        let mut q = query();
        q.filters = vec![Filter {
            column: "id".into(),
            op: FilterOp::In,
            value: " 1, 2 ,,3 ".into(),
        }];
        let sql = Dialect::Postgres.select_rows(&q, &[], &[]);
        assert!(
            sql.text.contains(r#""id" IN (E'1', E'2', E'3')"#),
            "{}",
            sql.text
        );
        let lite = Dialect::Sqlite.select_rows(&q, &[], &[]);
        assert!(lite.text.contains(r#""id" IN (?, ?, ?)"#), "{}", lite.text);
        assert_eq!(lite.params, vec![text("1"), text("2"), text("3")]);
        q.filters[0].value = " , ".into();
        let sql = Dialect::Postgres.select_rows(&q, &[], &[]);
        assert!(sql.text.contains("WHERE 1 = 0"), "{}", sql.text);
    }

    const UUID: &str = "0199a3f2-7c1e-7abc-8def-0123456789ab";
    const UUID_BYTES: [u8; 16] = [
        0x01, 0x99, 0xa3, 0xf2, 0x7c, 0x1e, 0x7a, 0xbc, 0x8d, 0xef, 0x01, 0x23, 0x45, 0x67, 0x89,
        0xab,
    ];

    fn bytes(value: &[u8]) -> Value {
        Value::Bytes(value.into())
    }

    fn filtered(column: &str, op: FilterOp, value: &str) -> RowQuery {
        let mut q = query();
        q.filters = vec![Filter {
            column: column.into(),
            op,
            value: value.into(),
        }];
        q
    }

    #[test]
    fn uuids_and_0x_hex_read_as_bytes() {
        for text in [
            UUID,
            "0199A3F2-7C1E-7ABC-8DEF-0123456789AB",
            "0199a3f27c1e7abc8def0123456789ab",
            "0x0199a3f27c1e7abc8def0123456789ab",
            "0X0199A3F27C1E7ABC8DEF0123456789AB",
            "  0199a3f2-7c1e-7abc-8def-0123456789ab\n",
        ] {
            assert_eq!(
                bytes_from_text(text).as_deref(),
                Some(&UUID_BYTES[..]),
                "{text}"
            );
        }
        assert_eq!(bytes_from_text("0x00ff10"), Some(vec![0x00, 0xff, 0x10]));
        // What the app copies for an empty value.
        assert_eq!(bytes_from_text("0x"), Some(Vec::new()));
        for text in [
            "",
            "bob",
            "18",
            // Hex without `0x` is bytes only at a UUID's length.
            "cafe",
            "0199a3f27c1e7abc8def0123456789a",
            "0199a3f27c1e7abc8def0123456789abcd",
            // Half a byte, and digits that are not hex.
            "0xabc",
            "0xzz",
            "0199a3f27c1e7abc8def0123456789ag",
            // Hyphens anywhere else.
            "0199a3f27-c1e-7abc-8def-0123456789ab",
            "0199a3f2-7c1e-7abc-8def-0123456789a-",
            "0199a3f2-7c1e-7abc-8def-012345678-ab",
            "-199a3f2-7c1e-7abc-8def-0123456789ab",
            "0199a3f2-7c1e-7abc-8def-0123456789é",
        ] {
            assert_eq!(bytes_from_text(text), None, "{text}");
        }
    }

    #[test]
    fn a_binary_column_is_compared_with_the_bytes_and_the_text() {
        let binary = ["id".to_owned()];
        let q = filtered("id", FilterOp::Eq, UUID);
        for dialect in [Dialect::Sqlite, Dialect::MySql] {
            let sql = dialect.select_rows(&q, &[], &binary);
            let id = dialect.quote_ident("id");
            assert!(
                sql.text.contains(&format!("WHERE {id} IN (?, ?) LIMIT")),
                "{}",
                sql.text
            );
            assert_eq!(sql.params, vec![bytes(&UUID_BYTES), text(UUID)]);
            // The count asks the same.
            let count = dialect.count_rows(&q, &binary);
            assert!(count.text.ends_with(&format!("WHERE {id} IN (?, ?)")));
            assert_eq!(count.params, sql.params);
        }
        let pg = Dialect::Postgres.select_rows(&q, &[], &binary);
        assert!(
            pg.text.contains(&format!(
                r#"WHERE "id" IN (E'\\x0199a3f27c1e7abc8def0123456789ab', E'{UUID}') LIMIT"#
            )),
            "{}",
            pg.text
        );
        assert!(pg.params.is_empty());

        let q = filtered("id", FilterOp::Ne, "0x00FF");
        let sql = Dialect::Sqlite.select_rows(&q, &[], &binary);
        assert!(
            sql.text.contains(r#"WHERE "id" NOT IN (?, ?) LIMIT"#),
            "{}",
            sql.text
        );
        assert_eq!(sql.params, vec![bytes(&[0x00, 0xff]), text("0x00FF")]);
        let pg = Dialect::Postgres.select_rows(&q, &[], &binary);
        assert!(
            pg.text
                .contains(r#"WHERE "id" NOT IN (E'\\x00ff', E'0x00FF') LIMIT"#),
            "{}",
            pg.text
        );
    }

    #[test]
    fn an_in_filter_on_a_binary_column_takes_each_value_both_ways() {
        let q = filtered("id", FilterOp::In, &format!("{UUID}, 7, 0xcafe"));
        let sql = Dialect::MySql.select_rows(&q, &[], &["id".to_owned()]);
        assert!(
            sql.text.contains("WHERE `id` IN (?, ?, ?, ?, ?) LIMIT"),
            "{}",
            sql.text
        );
        assert_eq!(
            sql.params,
            vec![
                bytes(&UUID_BYTES),
                text(UUID),
                text("7"),
                bytes(&[0xca, 0xfe]),
                text("0xcafe"),
            ]
        );
    }

    #[test]
    fn only_binary_columns_and_equality_bind_bytes() {
        let binary = ["id".to_owned()];
        // A text column holding the UUID as a string matches as text.
        let q = filtered("token", FilterOp::Eq, UUID);
        let sql = Dialect::Sqlite.select_rows(&q, &[], &binary);
        assert!(
            sql.text.contains(r#"WHERE "token" = ? LIMIT"#),
            "{}",
            sql.text
        );
        assert_eq!(sql.params, vec![text(UUID)]);
        let pg = Dialect::Postgres.select_rows(&q, &[], &binary);
        assert!(
            pg.text
                .contains(&format!(r#"WHERE "token" = E'{UUID}' LIMIT"#)),
            "{}",
            pg.text
        );
        // So does a binary column's value that is not bytes.
        let q = filtered("id", FilterOp::Eq, "bob");
        let sql = Dialect::Sqlite.select_rows(&q, &[], &binary);
        assert!(sql.text.contains(r#"WHERE "id" = ? LIMIT"#), "{}", sql.text);
        assert_eq!(sql.params, vec![text("bob")]);
        // Ordering and LIKE compare the text as typed.
        for op in [
            FilterOp::Lt,
            FilterOp::Gt,
            FilterOp::Le,
            FilterOp::Ge,
            FilterOp::Contains,
            FilterOp::StartsWith,
        ] {
            let sql = Dialect::Sqlite.select_rows(&filtered("id", op, UUID), &[], &binary);
            assert_eq!(sql.params.len(), 1, "{op:?}");
            assert!(matches!(sql.params[0], Value::Text(_)), "{op:?}");
        }
    }

    #[test]
    fn a_driver_asks_for_binary_columns_only_when_a_value_reads_as_bytes() {
        for (op, value, expected) in [
            (FilterOp::Eq, UUID, true),
            (FilterOp::Ne, "0xcafe", true),
            (FilterOp::In, "1, 0xcafe", true),
            (FilterOp::Eq, "bob", false),
            (FilterOp::In, "1, 2", false),
            (FilterOp::Gt, UUID, false),
            (FilterOp::Contains, "0xcafe", false),
            (FilterOp::IsNull, UUID, false),
        ] {
            assert_eq!(
                reads_bytes(&filtered("id", op, value)),
                expected,
                "{op:?} {value}"
            );
        }
        assert!(!reads_bytes(&query()));
    }

    #[test]
    fn like_patterns_escape_wildcards() {
        assert_eq!(escape_like(r"50%_off\now"), r"50\%\_off\\now");
    }

    #[test]
    fn contains_casts_to_text_and_is_case_insensitive_per_dialect() {
        let mut q = query();
        q.filters = vec![Filter {
            column: "id".into(),
            op: FilterOp::Contains,
            value: "5%".into(),
        }];
        let pg = Dialect::Postgres.select_rows(&q, &[], &[]);
        assert!(
            pg.text.contains(r#"CAST("id" AS TEXT) ILIKE E'%5\\%%'"#),
            "{}",
            pg.text
        );
        let lite = Dialect::Sqlite.select_rows(&q, &[], &[]);
        assert!(
            lite.text
                .contains(r#"CAST("id" AS TEXT) LIKE ? ESCAPE '\'"#),
            "{}",
            lite.text
        );
        assert_eq!(lite.params, vec![text(r"%5\%%")]);
        let my = Dialect::MySql.select_rows(&q, &[], &[]);
        assert!(my.text.contains("CAST(`id` AS CHAR) LIKE ?"), "{}", my.text);
        q.filters[0].op = FilterOp::StartsWith;
        assert!(
            Dialect::Postgres
                .select_rows(&q, &[], &[])
                .text
                .contains(r"ILIKE E'5\\%%'")
        );
    }

    #[test]
    fn raw_where_is_wrapped_in_parentheses_after_filters() {
        let mut q = query();
        q.filters = vec![Filter {
            column: "a".into(),
            op: FilterOp::Eq,
            value: "1".into(),
        }];
        q.raw_where = Some("b = 2 OR c = 3".into());
        let sql = Dialect::Postgres.select_rows(&q, &[], &[]);
        assert!(
            sql.text
                .contains("WHERE \"a\" = E'1' AND (\nb = 2 OR c = 3\n)"),
            "{}",
            sql.text
        );
        q.raw_where = Some("   ".into());
        assert!(
            !Dialect::Postgres
                .select_rows(&q, &[], &[])
                .text
                .contains("AND (")
        );
    }

    #[test]
    fn counts_use_the_same_filters_without_order_or_paging() {
        let mut q = query();
        q.filters = vec![Filter {
            column: "a".into(),
            op: FilterOp::Eq,
            value: "1".into(),
        }];
        q.sort = vec![Sort {
            column: "a".into(),
            dir: SortDir::Asc,
        }];
        q.offset = 300;
        let sql = Dialect::Sqlite.count_rows(&q, &[]);
        assert_eq!(
            sql.text,
            r#"SELECT count(*) FROM "public"."users" WHERE "a" = ?"#
        );
        assert_eq!(sql.params, vec![text("1")]);
    }

    #[test]
    fn sqlite_block_comments_are_found_outside_quotes_only() {
        for open in ["1=1) /*", "a = 1 /* note", "/*", "x = '*/' /*", "a /*/"] {
            assert!(sqlite_ends_in_block_comment(open), "{open}");
        }
        for closed in [
            "1 = 1",
            "a = 1 /* note */",
            "name = '/*'",
            "\"/*\" = 1",
            "[/*] = 1",
            "`/*` = 1",
            "a = 1 -- /*\n",
            "a = 1 -- /*",
            "name = 'it''s /*'",
            "name = 'unterminated /*",
            "/**/",
        ] {
            assert!(!sqlite_ends_in_block_comment(closed), "{closed}");
        }
    }

    #[test]
    fn postgres_values_become_quoted_literals() {
        assert_eq!(quote_literal("O'Brien"), "E'O''Brien'");
        assert_eq!(quote_literal(r"C:\temp"), r"E'C:\\temp'");
        assert_eq!(quote_literal(""), "E''");
        // A backslash cannot escape the closing quote, whatever
        // standard_conforming_strings is.
        assert_eq!(quote_literal(r"x\' OR 1=1 --"), r"E'x\\'' OR 1=1 --'");
        let mut q = query();
        q.filters = vec![Filter {
            column: "name".into(),
            op: FilterOp::Eq,
            value: "x' OR '1'='1".into(),
        }];
        let pg = Dialect::Postgres.select_rows(&q, &[], &[]);
        assert!(
            pg.text.contains(r#""name" = E'x'' OR ''1''=''1'"#),
            "{}",
            pg.text
        );
    }

    #[test]
    fn a_plain_name_is_inserted_bare_and_any_other_quoted() {
        let pg = Dialect::Postgres;
        assert_eq!(pg.ident("books"), "books");
        assert_eq!(pg.ident("book_reviews2"), "book_reviews2");
        assert_eq!(pg.ident("_private"), "_private");
        // PostgreSQL folds a bare name to lower case.
        assert_eq!(pg.ident("Books"), "\"Books\"");
        assert_eq!(pg.ident("BOOKS"), "\"BOOKS\"");
        for dialect in [Dialect::MySql, Dialect::Sqlite] {
            assert_eq!(dialect.ident("Books"), "Books", "{dialect:?}");
        }
        // Not a plain word.
        assert_eq!(pg.ident("Order Items"), "\"Order Items\"");
        assert_eq!(pg.ident("2fa"), "\"2fa\"");
        assert_eq!(pg.ident("a\"b"), "\"a\"\"b\"");
        assert_eq!(pg.ident("żółw"), "\"żółw\"");
        assert_eq!(pg.ident(""), "\"\"");
        assert_eq!(Dialect::MySql.ident("order-items"), "`order-items`");
        assert_eq!(Dialect::MySql.ident("a`b"), "`a``b`");
        assert_eq!(Dialect::Sqlite.ident("order items"), "\"order items\"");
    }

    #[test]
    fn a_reserved_word_is_quoted() {
        for word in [
            "user", "order", "group", "table", "select", "end", "primary",
        ] {
            assert_eq!(
                Dialect::Postgres.ident(word),
                format!("\"{word}\""),
                "{word}"
            );
        }
        for word in ["order", "group", "key", "keys", "index", "rank", "SELECT"] {
            assert_eq!(Dialect::MySql.ident(word), format!("`{word}`"), "{word}");
        }
        for word in ["order", "group", "key", "index", "transaction", "Values"] {
            assert_eq!(Dialect::Sqlite.ident(word), format!("\"{word}\""), "{word}");
        }
        // Reserved elsewhere, not here.
        assert_eq!(Dialect::Postgres.ident("key"), "key");
        assert_eq!(Dialect::MySql.ident("user"), "user");
    }
}

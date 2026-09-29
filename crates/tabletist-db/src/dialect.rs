//! SQL for each database: identifier quoting, placeholders, filters, paging.

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

/// A SQL string literal with `'` doubled (standard-conforming strings).
pub fn quote_literal(text: &str) -> String {
    format!("'{}'", text.replace('\'', "''"))
}

impl Dialect {
    pub fn quote_ident(self, ident: &str) -> String {
        match self {
            Self::MySql => format!("`{}`", ident.replace('`', "``")),
            Self::Postgres | Self::Sqlite => format!("\"{}\"", ident.replace('"', "\"\"")),
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
    /// quoted literals; sessions set `standard_conforming_strings = on`, so
    /// `'` is the only character that needs escaping. The others bind.
    fn bind(self, params: &mut Vec<Value>, value: String) -> String {
        match self {
            Self::Postgres => quote_literal(&value),
            Self::MySql | Self::Sqlite => {
                params.push(Value::Text(value.into()));
                self.placeholder().to_owned()
            }
        }
    }

    fn filter(self, filter: &Filter, params: &mut Vec<Value>) -> String {
        let column = self.quote_ident(&filter.column);
        let compare = |op: &str, params: &mut Vec<Value>| {
            let placeholder = self.bind(params, filter.value.clone());
            format!("{column} {op} {placeholder}")
        };
        match filter.op {
            FilterOp::Eq => compare("=", params),
            FilterOp::Ne => compare("<>", params),
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
                let values: Vec<&str> = filter
                    .value
                    .split(',')
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .collect();
                if values.is_empty() {
                    return "1 = 0".into();
                }
                let placeholders: Vec<String> = values
                    .into_iter()
                    .map(|value| self.bind(params, value.to_owned()))
                    .collect();
                format!("{column} IN ({})", placeholders.join(", "))
            }
        }
    }

    fn where_clause(self, query: &RowQuery, params: &mut Vec<Value>) -> String {
        let mut conditions: Vec<String> = query
            .filters
            .iter()
            .map(|filter| self.filter(filter, params))
            .collect();
        if let Some(raw) = query.raw_where.as_deref().map(str::trim)
            && !raw.is_empty()
        {
            // On its own lines, so a trailing `--` comment in the raw text
            // cannot reach the builder's LIMIT.
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
    /// tiebreaker after the user's sort.
    pub fn select_rows(self, query: &RowQuery, key: &[String]) -> Sql {
        let mut params = Vec::new();
        let mut text = format!("SELECT * FROM {}", self.qualified(&query.object));
        text.push_str(&self.where_clause(query, &mut params));
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

    pub fn count_rows(self, query: &RowQuery) -> Sql {
        let mut params = Vec::new();
        let mut text = format!("SELECT count(*) FROM {}", self.qualified(&query.object));
        text.push_str(&self.where_clause(query, &mut params));
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
        let sql = Dialect::Postgres.select_rows(&query(), &["id".into()]);
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
        let sql = Dialect::Sqlite.select_rows(&q, &[]);
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
        let sql = Dialect::MySql.select_rows(&q, &["id".into()]);
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
        let pg = Dialect::Postgres.select_rows(&q, &[]);
        assert_eq!(
            pg.text,
            r#"SELECT * FROM "public"."users" WHERE "age" >= '18' AND "name" <> 'bob' LIMIT 301 OFFSET 0"#
        );
        assert!(pg.params.is_empty());
        let my = Dialect::MySql.select_rows(&q, &[]);
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
        let sql = Dialect::Sqlite.select_rows(&q, &[]);
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
        let sql = Dialect::Postgres.select_rows(&q, &[]);
        assert!(
            sql.text.contains(r#""id" IN ('1', '2', '3')"#),
            "{}",
            sql.text
        );
        let lite = Dialect::Sqlite.select_rows(&q, &[]);
        assert!(lite.text.contains(r#""id" IN (?, ?, ?)"#), "{}", lite.text);
        assert_eq!(lite.params, vec![text("1"), text("2"), text("3")]);
        q.filters[0].value = " , ".into();
        let sql = Dialect::Postgres.select_rows(&q, &[]);
        assert!(sql.text.contains("WHERE 1 = 0"), "{}", sql.text);
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
        let pg = Dialect::Postgres.select_rows(&q, &[]);
        assert!(
            pg.text.contains(r#"CAST("id" AS TEXT) ILIKE '%5\%%'"#),
            "{}",
            pg.text
        );
        let lite = Dialect::Sqlite.select_rows(&q, &[]);
        assert!(
            lite.text
                .contains(r#"CAST("id" AS TEXT) LIKE ? ESCAPE '\'"#),
            "{}",
            lite.text
        );
        assert_eq!(lite.params, vec![text(r"%5\%%")]);
        let my = Dialect::MySql.select_rows(&q, &[]);
        assert!(my.text.contains("CAST(`id` AS CHAR) LIKE ?"), "{}", my.text);
        q.filters[0].op = FilterOp::StartsWith;
        assert!(
            Dialect::Postgres
                .select_rows(&q, &[])
                .text
                .contains(r"ILIKE '5\%%'")
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
        let sql = Dialect::Postgres.select_rows(&q, &[]);
        assert!(
            sql.text
                .contains("WHERE \"a\" = '1' AND (\nb = 2 OR c = 3\n)"),
            "{}",
            sql.text
        );
        q.raw_where = Some("   ".into());
        assert!(
            !Dialect::Postgres
                .select_rows(&q, &[])
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
        let sql = Dialect::Sqlite.count_rows(&q);
        assert_eq!(
            sql.text,
            r#"SELECT count(*) FROM "public"."users" WHERE "a" = ?"#
        );
        assert_eq!(sql.params, vec![text("1")]);
    }

    #[test]
    fn postgres_values_become_quoted_literals() {
        assert_eq!(quote_literal("O'Brien"), "'O''Brien'");
        assert_eq!(quote_literal(r"C:\temp"), r"'C:\temp'");
        assert_eq!(quote_literal(""), "''");
        let mut q = query();
        q.filters = vec![Filter {
            column: "name".into(),
            op: FilterOp::Eq,
            value: "x' OR '1'='1".into(),
        }];
        let pg = Dialect::Postgres.select_rows(&q, &[]);
        assert!(
            pg.text.contains(r#""name" = 'x'' OR ''1''=''1'"#),
            "{}",
            pg.text
        );
    }
}

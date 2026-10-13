//! SQL for each database: identifier quoting, placeholders, filters, paging.

use std::fmt::Write as _;

use crate::{
    ColumnClass, Error, Filter, FilterOp, Lookup, LookupPart, Matched, NewValue, ObjectRef, Result,
    RowChange, RowDelete, RowInsert, RowQuery, SortDir, Temporal, Value, column_class, temporal,
};

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

/// A row's `UPDATE`, as a user reads it and as the driver runs it. Both
/// come from the same values, so they cannot drift apart.
#[derive(Debug, Clone, PartialEq)]
pub struct RowUpdate {
    /// The statement with its values as literals.
    pub shown: String,
    /// What the driver sends: for PostgreSQL the shown text, for MySQL and
    /// SQLite the same statement with the values bound.
    pub sql: Sql,
    /// Where the parts of `shown` stand.
    pub parts: UpdateParts,
}

/// A new row's `INSERT`, as a user reads it and as the driver runs it. Both
/// come from the same values, as a [`RowUpdate`]'s do.
#[derive(Debug, Clone, PartialEq)]
pub struct InsertStatement {
    /// The statement with its values as literals.
    pub shown: String,
    /// What the driver sends: for PostgreSQL the shown text, for MySQL and
    /// SQLite the same statement with the values bound.
    pub sql: Sql,
    /// Where the parts of `shown` stand.
    pub parts: InsertParts,
}

/// Where the parts of a shown `INSERT` stand, as byte offsets into its
/// text, noted as the builder writes it: as [`UpdateParts`], so a name or
/// a value that holds ` VALUES (` cannot move them.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct InsertParts {
    /// Where the clause that gives the values begins: `VALUES`, or
    /// `DEFAULT VALUES`. The table and its columns end a space before it.
    pub values: usize,
    /// Each value's place, in the order its columns are named.
    pub literals: Vec<std::ops::Range<usize>>,
    /// Where `RETURNING` begins, where the statement hands its row back.
    pub back: Option<usize>,
}

/// Where the parts of a shown `UPDATE` stand, as byte offsets into its
/// text. The builder notes them as it writes the statement, so what lays
/// the statement out in lines, or shortens a long value where it is shown,
/// reads no SQL to find them: a name or a value that holds ` SET ` or a
/// quote cannot move them.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct UpdateParts {
    /// Where `SET` begins: the table's name ends a space before it.
    pub set: usize,
    /// Where each value's column begins: one for each of `values`.
    pub columns: Vec<usize>,
    /// Each value's place: the new ones in the order they are set, then
    /// the key's.
    pub values: Vec<std::ops::Range<usize>>,
}

/// A row's `DELETE`, as a user reads it and as the driver runs it. Both
/// come from the same key, as a [`RowUpdate`]'s do.
#[derive(Debug, Clone, PartialEq)]
pub struct RowDeletion {
    /// The statement with its key's values as literals.
    pub shown: String,
    /// What the driver sends: for PostgreSQL the shown text, for MySQL and
    /// SQLite the same statement with the values bound.
    pub sql: Sql,
    /// Where the parts of `shown` stand.
    pub parts: DeleteParts,
}

/// Where the parts of a shown `DELETE` stand, as byte offsets into its
/// text, noted as the builder writes it: as [`UpdateParts`], so a name or
/// a value that holds ` WHERE ` cannot move them.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DeleteParts {
    /// Where `WHERE` begins: the table's name ends a space before it.
    pub filter: usize,
    /// Where each key column begins: one for each of `values`. A run's
    /// ([`Dialect::delete_run`]) has each key column once.
    pub columns: Vec<usize>,
    /// Each key value's place. A run's row by row: as many as its rows
    /// times its columns.
    pub values: Vec<std::ops::Range<usize>>,
}

/// A row's key as a `WHERE`, shown and sent, and where its parts stand in
/// the shown clause.
struct KeyClause {
    shown: String,
    sent: String,
    columns: Vec<usize>,
    values: Vec<std::ops::Range<usize>>,
}

/// A value as a statement holds it. Decided once, here, so the literal a
/// user reads is the value the driver sends.
#[derive(Debug, Clone, PartialEq)]
enum Operand {
    Null,
    Int(i64),
    Float(f64),
    Text(String),
    Bytes(Vec<u8>),
    /// Written into the statement as it is, shown and sent alike: a
    /// keyword, or a default's expression as the catalog gave it.
    Raw(String),
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

    /// `column`, as text, like `pattern`: what a contains filter writes,
    /// with this engine's sense of case.
    fn matches(self, column: &str, pattern: String, params: &mut Vec<Value>) -> String {
        let placeholder = self.bind(params, pattern);
        format!(
            "{} {} {placeholder}{}",
            self.as_text(column),
            self.like(),
            self.like_escape()
        )
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
                self.matches(&column, pattern, params)
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

    /// `text` as a string literal. PostgreSQL takes the plain form unless
    /// the text holds a backslash, which only the escape form keeps whatever
    /// `standard_conforming_strings` is. MySQL reads a backslash as an
    /// escape in every string (the session keeps `NO_BACKSLASH_ESCAPES`
    /// off), so it is doubled there, and a NUL is written as the escape
    /// MySQL has for it: the literal is for a person to read and to copy,
    /// and a clipboard, or what the text is pasted into, may end text at a
    /// raw NUL and leave a statement without its `WHERE`. After the
    /// doubling, so the escape's own backslash stays single.
    pub(crate) fn literal(self, text: &str) -> String {
        match self {
            Self::Postgres if text.contains('\\') => quote_literal(text),
            Self::Postgres | Self::Sqlite => format!("'{}'", text.replace('\'', "''")),
            Self::MySql => format!(
                "'{}'",
                text.replace('\\', "\\\\")
                    .replace('\0', "\\0")
                    .replace('\'', "''")
            ),
        }
    }

    /// Bytes as a literal: PostgreSQL's hex `bytea`, `x'..'` elsewhere.
    pub(crate) fn bytes_literal(self, bytes: &[u8]) -> String {
        let mut hex = String::new();
        for byte in bytes {
            let _ = write!(hex, "{byte:02x}");
        }
        match self {
            Self::Postgres => quote_literal(&format!("\\x{hex}")),
            Self::MySql | Self::Sqlite => format!("x'{hex}'"),
        }
    }

    /// The operand as a literal. What SQLite's parser would not read as the
    /// value has a form of its own there, so the text a person copies runs
    /// and stores what the bound statement does.
    fn shown(self, operand: &Operand) -> String {
        match operand {
            Operand::Null => "NULL".to_owned(),
            Operand::Int(number) => number.to_string(),
            // SQLite reads `inf` as a column's name. A number past the
            // largest real is infinity to it, and is how it writes one.
            Operand::Float(number) if self == Self::Sqlite && number.is_infinite() => {
                if number.is_sign_positive() {
                    "9e999".to_owned()
                } else {
                    "-9e999".to_owned()
                }
            }
            // With its point, so a real never reads as a whole number.
            Operand::Float(number) => format!("{number:?}"),
            // SQLite's parser ends the statement at a NUL, inside a string
            // too, so such text is joined around `char(0)`. Not a cast of
            // its bytes: that reads them in the file's encoding, and a
            // UTF-16 file would store other text.
            Operand::Text(text) if self == Self::Sqlite && text.contains('\0') => {
                let parts: Vec<String> = text.split('\0').map(|part| self.literal(part)).collect();
                format!("({})", parts.join(" || char(0) || "))
            }
            Operand::Text(text) => self.literal(text),
            Operand::Bytes(bytes) => self.bytes_literal(bytes),
            Operand::Raw(text) => text.clone(),
        }
    }

    /// The time a statement runs, as this engine writes it for a column
    /// that keeps `kind`.
    fn now(self, kind: Temporal) -> &'static str {
        match (self, kind) {
            (Self::Postgres | Self::MySql | Self::Sqlite, Temporal::Date) => "CURRENT_DATE",
            (Self::Postgres | Self::Sqlite, Temporal::Time) => "CURRENT_TIME",
            (Self::Postgres, Temporal::Timestamp) => "now()",
            // To the microsecond: without the precision MySQL gives whole
            // seconds, whatever the column keeps.
            (Self::MySql, Temporal::Time) => "CURRENT_TIME(6)",
            (Self::MySql, Temporal::Timestamp) => "CURRENT_TIMESTAMP(6)",
            (Self::Sqlite, Temporal::Timestamp) => "CURRENT_TIMESTAMP",
        }
    }

    /// The operand in the statement the driver runs: its literal for
    /// PostgreSQL, whose rows go through the simple-query protocol, and a
    /// bound value elsewhere. NULL is written out in both.
    fn sent(self, operand: &Operand, params: &mut Vec<Value>) -> String {
        if self == Self::Postgres {
            return self.shown(operand);
        }
        params.push(match operand {
            Operand::Null => return "NULL".to_owned(),
            // A keyword is no value to bind.
            Operand::Raw(text) => return text.clone(),
            Operand::Int(number) => Value::Int(*number),
            Operand::Float(number) => Value::Float(*number),
            Operand::Text(text) => Value::Text(text.as_str().into()),
            Operand::Bytes(bytes) => Value::Bytes(bytes.as_slice().into()),
        });
        self.placeholder().to_owned()
    }

    /// A key's loaded value as the operand that finds its row again.
    /// PostgreSQL converts a literal to the column's type, so everything
    /// but a whole number goes as text there.
    fn key_operand(self, value: &Value) -> Operand {
        match (self, value) {
            (_, Value::Null) => Operand::Null,
            (_, Value::Int(number)) => Operand::Int(*number),
            (_, Value::Text(text)) => Operand::Text(text.to_string()),
            (_, Value::Bytes(bytes)) => Operand::Bytes(bytes.to_vec()),
            (Self::Postgres, Value::Bool(flag)) => Operand::Text(flag.to_string()),
            (Self::Postgres, Value::Float(number)) => Operand::Text(number.to_string()),
            (Self::MySql | Self::Sqlite, Value::Bool(flag)) => Operand::Int(i64::from(*flag)),
            (Self::MySql | Self::Sqlite, Value::Float(number)) => Operand::Float(*number),
        }
    }

    /// A cell's new value as an operand. PostgreSQL and MySQL convert text
    /// to the column's type themselves. Where the database would store the
    /// text as it is, it is converted here, by the column's class: numbers
    /// on SQLite, and a boolean on SQLite and MySQL, which keep one as 1 or
    /// 0. Text that cannot be converted is refused, naming the column.
    /// `loaded` is what the cell held, and `None` for a new row's.
    fn new_operand(
        self,
        column: &str,
        type_name: &str,
        loaded: Option<&Value>,
        new: &NewValue,
    ) -> Result<Operand> {
        let class = column_class(self, type_name);
        // A binary column is never sent text (MySQL would store a `bit`'s
        // text as the characters' codes), and is not edited at all yet: a
        // NULL for one is refused with the rest.
        // By what the cell held too: a SQLite column of any declared type
        // can hold a blob.
        if class == ColumnClass::Binary || matches!(loaded, Some(Value::Bytes(_))) {
            return Err(Error::query(format!(
                "{column}: binary values cannot be edited yet"
            )));
        }
        let refused = |expects: &str| {
            Error::query(format!(
                "{column}: {} expects {expects}",
                if type_name.is_empty() {
                    "the column"
                } else {
                    type_name
                }
            ))
        };
        let text = match new {
            NewValue::Null => return Ok(Operand::Null),
            NewValue::Default { expression } => {
                return Ok(Operand::Raw(match self {
                    Self::Postgres | Self::MySql => "DEFAULT".to_owned(),
                    // SQLite has no DEFAULT to write in an UPDATE or among
                    // VALUES: the column's own expression, which is what
                    // the word would have come to, or NULL where it has
                    // none.
                    Self::Sqlite => match expression {
                        Some(expression) => format!("({expression})"),
                        None => "NULL".to_owned(),
                    },
                }));
            }
            NewValue::Now => {
                let kind = temporal(type_name)
                    .ok_or_else(|| refused("a value, not the time of the save"))?;
                return Ok(Operand::Raw(self.now(kind).to_owned()));
            }
            NewValue::Text(text) => text,
        };
        let typed = text.trim();
        let whole = || typed.parse::<i64>().ok().map(Operand::Int);
        let real = || {
            typed
                .parse::<f64>()
                .ok()
                .filter(|number| number.is_finite())
                .map(Operand::Float)
        };
        match (self, class) {
            (Self::Postgres, _) => Ok(Operand::Text(text.clone())),
            // A tinyint(1) holds any tinyint, and some tables keep more
            // than a flag in one.
            (Self::MySql, ColumnClass::Boolean) => match typed.to_ascii_lowercase().as_str() {
                "true" => Ok(Operand::Int(1)),
                "false" => Ok(Operand::Int(0)),
                _ => typed
                    .parse::<i8>()
                    .map(|number| Operand::Int(i64::from(number)))
                    .map_err(|_| refused("true, false or a whole number from -128 to 127")),
            },
            (Self::Sqlite, ColumnClass::Boolean) => match typed.to_ascii_lowercase().as_str() {
                "true" | "1" => Ok(Operand::Int(1)),
                "false" | "0" => Ok(Operand::Int(0)),
                _ => Err(refused("true or false")),
            },
            // No declared type, or one SQLite gives no affinity: nothing
            // converts the text, so a number stays a number only where the
            // cell held one.
            (Self::Sqlite, ColumnClass::Other)
                if matches!(loaded, Some(Value::Int(_) | Value::Float(_))) =>
            {
                Ok(whole()
                    .or_else(real)
                    .unwrap_or_else(|| Operand::Text(text.clone())))
            }
            (Self::Sqlite, ColumnClass::Integer { .. }) => {
                whole().ok_or_else(|| refused("a whole number"))
            }
            (Self::Sqlite, ColumnClass::Float) => real().ok_or_else(|| refused("a number")),
            (Self::Sqlite, ColumnClass::Decimal { .. }) => {
                whole().or_else(real).ok_or_else(|| refused("a number"))
            }
            _ => Ok(Operand::Text(text.clone())),
        }
    }

    /// ` WHERE "a" = .. AND "b" = ..` for a row's key.
    fn key_clause(self, key: &[(String, Value)], params: &mut Vec<Value>) -> KeyClause {
        let mut clause = KeyClause {
            shown: String::from(" WHERE "),
            sent: String::from(" WHERE "),
            columns: Vec::with_capacity(key.len()),
            values: Vec::with_capacity(key.len()),
        };
        for (index, (column, value)) in key.iter().enumerate() {
            let operand = self.key_operand(value);
            let column = self.quote_ident(column);
            let lead = if index == 0 { "" } else { " AND " };
            clause.shown.push_str(lead);
            clause.columns.push(clause.shown.len());
            let _ = write!(clause.shown, "{column} = ");
            let literal = self.shown(&operand);
            let at = clause.shown.len();
            clause.values.push(at..at + literal.len());
            clause.shown.push_str(&literal);
            let sent = self.sent(&operand, params);
            let _ = write!(clause.sent, "{lead}{column} = {sent}");
        }
        clause
    }

    /// Refuses a SQLite key that may not be the row's. Text that is not
    /// UTF-8 is read with U+FFFD for its bad bytes, so a key that holds one
    /// may stand for other bytes, and bound as it reads it finds another
    /// row, whose key really is that text. A key that really holds U+FFFD
    /// is refused with it, since the page's value cannot tell the two
    /// apart. That is accepted.
    fn key_read_exactly(self, key: &[(String, Value)]) -> Result<()> {
        let lossy = |value: &Value| matches!(value, Value::Text(text) if text.contains('\u{FFFD}'));
        if self == Self::Sqlite && key.iter().any(|(_, value)| lossy(value)) {
            return Err(Error::query(
                "the row's key holds text that may not have been read exactly, so the save \
                 cannot be sure which row it names",
            ));
        }
        Ok(())
    }

    /// The `UPDATE` of one row of a save. `Err` names the value that cannot
    /// be sent in its column's form, or says why no save sends the row at
    /// all: a SQLite key that may not be the row's, PostgreSQL text that
    /// holds a NUL. Refused here and nowhere after it, so what a review
    /// shows of a row is what a save does with it: each driver builds its
    /// statements with this before it sends any, and fails the row whose
    /// statement is refused.
    pub fn update_row(self, object: &ObjectRef, row: &RowChange) -> Result<RowUpdate> {
        // Before any value of the row is looked at: without a key that is
        // the row's there is no row to say anything of.
        self.key_read_exactly(&row.key)?;
        let mut params = Vec::new();
        let mut shown = format!("UPDATE {} SET ", self.qualified(object));
        let mut sent = shown.clone();
        let mut parts = UpdateParts {
            set: shown.len() - "SET ".len(),
            ..UpdateParts::default()
        };
        for (index, change) in row.set.iter().enumerate() {
            let operand = self.new_operand(
                &change.column,
                &change.type_name,
                Some(&change.loaded),
                &change.new,
            )?;
            let column = self.quote_ident(&change.column);
            let lead = if index == 0 { "" } else { ", " };
            shown.push_str(lead);
            parts.columns.push(shown.len());
            let _ = write!(shown, "{column} = ");
            let literal = self.shown(&operand);
            parts.values.push(shown.len()..shown.len() + literal.len());
            shown.push_str(&literal);
            let value = self.sent(&operand, &mut params);
            let _ = write!(sent, "{lead}{column} = {value}");
        }
        let clause = self.key_clause(&row.key, &mut params);
        let base = shown.len();
        parts
            .columns
            .extend(clause.columns.iter().map(|column| base + column));
        parts.values.extend(
            clause
                .values
                .iter()
                .map(|value| base + value.start..base + value.end),
        );
        shown.push_str(&clause.shown);
        sent.push_str(&clause.sent);
        // PostgreSQL text cannot hold a NUL, and the driver cannot put one
        // in a message: it fails in a way that reads as a lost session.
        // The key's values and the new ones are all in the statement, and
        // its names with them, so the reads by key need no check of their
        // own.
        if self == Self::Postgres && sent.contains('\0') {
            return Err(Error::query("PostgreSQL text cannot hold a NUL character"));
        }
        Ok(RowUpdate {
            shown,
            sql: Sql { text: sent, params },
            parts,
        })
    }

    /// The `INSERT` of one new row of a save: the columns that were set,
    /// and nothing for the others, which the database fills. `Err` names
    /// the value that cannot be sent in its column's form. Refused here and
    /// nowhere after it, as [`Dialect::update_row`] refuses, so what a
    /// review shows of a new row is what a save does with it.
    ///
    /// PostgreSQL and SQLite hand the row back as the statement left it;
    /// MySQL hands nothing back, and its save finds the row again.
    pub fn insert_row(self, object: &ObjectRef, row: &RowInsert) -> Result<InsertStatement> {
        let table = self.qualified(object);
        let back = match self {
            Self::Postgres | Self::Sqlite => " RETURNING *",
            Self::MySql => "",
        };
        let lead = format!("INSERT INTO {table}");
        if row.set.is_empty() {
            let (text, values) = match self {
                Self::Postgres | Self::Sqlite => (format!("{lead} DEFAULT VALUES"), lead.len() + 1),
                Self::MySql => (format!("{lead} () VALUES ()"), lead.len() + 4),
            };
            let parts = InsertParts {
                values,
                literals: Vec::new(),
                // `back` begins with the space before its word.
                back: (!back.is_empty()).then_some(text.len() + 1),
            };
            let text = format!("{text}{back}");
            return Ok(InsertStatement {
                shown: text.clone(),
                sql: Sql {
                    text,
                    params: Vec::new(),
                },
                parts,
            });
        }
        let mut params = Vec::new();
        let mut names = Vec::with_capacity(row.set.len());
        let mut shown = Vec::with_capacity(row.set.len());
        let mut sent = Vec::with_capacity(row.set.len());
        for cell in &row.set {
            let operand = self.new_operand(&cell.column, &cell.type_name, None, &cell.new)?;
            names.push(self.quote_ident(&cell.column));
            shown.push(self.shown(&operand));
            sent.push(self.sent(&operand, &mut params));
        }
        let head = format!("{lead} ({}) VALUES (", names.join(", "));
        let mut parts = InsertParts {
            values: head.len() - "VALUES (".len(),
            ..InsertParts::default()
        };
        let text = format!("{head}{}){back}", sent.join(", "));
        // The shown statement, each value's place noted as it is written.
        let mut written = head;
        for (index, literal) in shown.iter().enumerate() {
            if index > 0 {
                written.push_str(", ");
            }
            let start = written.len();
            written.push_str(literal);
            parts.literals.push(start..written.len());
        }
        written.push(')');
        parts.back = (!back.is_empty()).then_some(written.len() + 1);
        written.push_str(back);
        // PostgreSQL text cannot hold a NUL, and the driver cannot put one
        // in a message: see `update_row`.
        let refused = match self {
            Self::Postgres => text.contains('\0'),
            Self::MySql | Self::Sqlite => false,
        };
        if refused {
            return Err(Error::query("PostgreSQL text cannot hold a NUL character"));
        }
        Ok(InsertStatement {
            shown: written,
            sql: Sql { text, params },
            parts,
        })
    }

    /// The `DELETE` of one row of a save, by the row's key and nothing
    /// else: no other column is compared. `Err` says why no save sends
    /// the row at all, as [`Dialect::update_row`] does: a SQLite key that
    /// may not be the row's, PostgreSQL text that holds a NUL. Refused
    /// here and nowhere after it, so what a review shows of a row is what
    /// a save does with it.
    pub fn delete_row(self, object: &ObjectRef, row: &RowDelete) -> Result<RowDeletion> {
        self.key_read_exactly(&row.key)?;
        let mut params = Vec::new();
        let lead = format!("DELETE FROM {}", self.qualified(object));
        let clause = self.key_clause(&row.key, &mut params);
        let base = lead.len();
        let parts = DeleteParts {
            // The clause begins with the space before its word.
            filter: base + 1,
            columns: clause.columns.iter().map(|column| base + column).collect(),
            values: clause
                .values
                .iter()
                .map(|value| base + value.start..base + value.end)
                .collect(),
        };
        let shown = format!("{lead}{}", clause.shown);
        let text = format!("{lead}{}", clause.sent);
        // PostgreSQL text cannot hold a NUL, and the driver cannot put one
        // in a message: see `update_row`.
        let refused = match self {
            Self::Postgres => text.contains('\0'),
            Self::MySql | Self::Sqlite => false,
        };
        if refused {
            return Err(Error::query("PostgreSQL text cannot hold a NUL character"));
        }
        Ok(RowDeletion {
            shown,
            sql: Sql { text, params },
            parts,
        })
    }

    /// One `DELETE` for a run of a save's rows ([`crate::delete_runs`]):
    /// `WHERE "id" IN (..)`, or `WHERE ("a", "b") IN ((..), (..))` for a
    /// key of several columns. `Err` where it is no run (fewer than two
    /// rows, or keys that do not name the same columns in the same order
    /// with values of one kind),
    /// and where one of its rows is one no save sends
    /// ([`Dialect::delete_row`]). The rows it finds are the rows its
    /// rows' own statements find: each value is spelled as there, and
    /// compared as `=` compares it.
    pub fn delete_run(self, object: &ObjectRef, rows: &[&RowDelete]) -> Result<RowDeletion> {
        let [first, _, ..] = rows else {
            return Err(Error::query("a run of deletes is two rows or more"));
        };
        let same = |row: &&RowDelete| crate::write::same_key_shape(&row.key, &first.key);
        if first.key.is_empty() || !rows.iter().all(same) {
            return Err(Error::query(
                "the rows of a run have the same key columns, with values of one kind",
            ));
        }
        for row in rows {
            self.key_read_exactly(&row.key)?;
        }
        let several = first.key.len() > 1;
        let mut params = Vec::new();
        let mut shown = format!("DELETE FROM {}", self.qualified(object));
        let mut parts = DeleteParts {
            // The clause begins with the space before its word.
            filter: shown.len() + 1,
            ..DeleteParts::default()
        };
        shown.push_str(" WHERE ");
        let mut sent = shown.clone();
        if several {
            shown.push('(');
            sent.push('(');
        }
        for (index, (column, _)) in first.key.iter().enumerate() {
            let column = self.quote_ident(column);
            let lead = if index == 0 { "" } else { ", " };
            shown.push_str(lead);
            parts.columns.push(shown.len());
            shown.push_str(&column);
            let _ = write!(sent, "{lead}{column}");
        }
        // A list of rows on the right of `IN`: SQLite takes one only as
        // `VALUES`.
        let open = match (self, several) {
            (Self::Postgres | Self::MySql | Self::Sqlite, false) => " IN (",
            (Self::Postgres | Self::MySql, true) => ") IN (",
            (Self::Sqlite, true) => ") IN (VALUES ",
        };
        shown.push_str(open);
        sent.push_str(open);
        for (row, delete) in rows.iter().enumerate() {
            let lead = match (row, several) {
                (0, false) => "",
                (_, false) => ", ",
                (0, true) => "(",
                (_, true) => ", (",
            };
            shown.push_str(lead);
            sent.push_str(lead);
            for (index, (_, value)) in delete.key.iter().enumerate() {
                let operand = self.key_operand(value);
                let lead = if index == 0 { "" } else { ", " };
                shown.push_str(lead);
                let literal = self.shown(&operand);
                let at = shown.len();
                parts.values.push(at..at + literal.len());
                shown.push_str(&literal);
                let bound = self.sent(&operand, &mut params);
                let _ = write!(sent, "{lead}{bound}");
            }
            if several {
                shown.push(')');
                sent.push(')');
            }
        }
        shown.push(')');
        sent.push(')');
        // PostgreSQL text cannot hold a NUL: see `delete_row`.
        let refused = match self {
            Self::Postgres => sent.contains('\0'),
            Self::MySql | Self::Sqlite => false,
        };
        if refused {
            return Err(Error::query("PostgreSQL text cannot hold a NUL character"));
        }
        Ok(RowDeletion {
            shown,
            sql: Sql { text: sent, params },
            parts,
        })
    }

    /// Whether `error` says the database was busy, not that a statement
    /// was wrong: a lock that was waited for too long, a deadlock, a
    /// transaction it could not put in order with another. A run of
    /// deletes that ends so is not sent again row by row: each row could
    /// wait as long once more.
    pub fn busy(self, error: &Error) -> bool {
        let Error::Query { code, named, .. } = error else {
            return false;
        };
        match self {
            // Deadlock detected, lock not available, serialization failure.
            Self::Postgres => matches!(code.as_deref(), Some("40P01" | "55P03" | "40001")),
            // Lock wait timeout, deadlock.
            Self::MySql => matches!(named.number, Some(1205 | 1213)),
            // SQLITE_BUSY and SQLITE_LOCKED, under whatever the extended
            // code adds above its low byte.
            Self::Sqlite => code
                .as_deref()
                .and_then(|code| code.parse::<u32>().ok())
                .is_some_and(|code| matches!(code & 0xff, 5 | 6)),
        }
    }

    /// The row of `key`, whole. With `lock` it is held until the
    /// transaction ends, where the database has row locks. Two rows at
    /// most: a caller only needs to tell none, one and more than one
    /// apart, and a key that is not one (SQLite cannot say that an index
    /// compares otherwise than its column) could match a whole table.
    pub fn select_row(self, object: &ObjectRef, key: &[(String, Value)], lock: bool) -> Sql {
        let mut params = Vec::new();
        let clause = self.key_clause(key, &mut params).sent;
        let lock = if lock && self != Self::Sqlite {
            " FOR UPDATE"
        } else {
            ""
        };
        Sql {
            text: format!(
                "SELECT * FROM {}{clause} LIMIT 2{lock}",
                self.qualified(object)
            ),
            params,
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

    /// The one statement of a [`Lookup`]. Its rows are the key, the label
    /// where the lookup names one, and how the row matched
    /// (`Matched::code`).
    ///
    /// The matches are read inside a derived table that holds the limit,
    /// so the database stops at the first rows it finds, and they are
    /// ordered outside it: no table is sorted whole. The columns are
    /// ordered by their place, since a column may be named as the `CASE`
    /// would be.
    pub fn lookup(self, lookup: &Lookup) -> Sql {
        let table = self.qualified(&lookup.object);
        let key = self.quote_ident(&lookup.key);
        let label = lookup.label.as_deref().map(|label| self.quote_ident(label));
        let columns = match &label {
            Some(label) => format!("{key}, {label}"),
            None => key.clone(),
        };
        let limit = u64::from(lookup.limit) + 1;
        let mut params = Vec::new();
        let text = match &lookup.part {
            LookupPart::Pinned(value) => {
                let operand = self.sent(&self.key_operand(value), &mut params);
                format!(
                    "SELECT {columns}, {} FROM {table} WHERE {key} = {operand} LIMIT {limit}",
                    Matched::Key.code()
                )
            }
            LookupPart::Matching(text) if text.is_empty() => format!(
                "SELECT {columns}, {} FROM {table} ORDER BY 1 LIMIT {limit}",
                Matched::KeyStart.code()
            ),
            LookupPart::Matching(text) => {
                let escaped = escape_like(text);
                // Each value is bound where the statement names it: MySQL
                // and SQLite bind by position. The key is compared as
                // text on purpose: SQLite binds a text that reads as a
                // number as one, and only the cast makes `91` equal it.
                let is_key = format!(
                    "{} = {}",
                    self.as_text(&key),
                    self.bind(&mut params, text.clone())
                );
                let how = match &label {
                    Some(label) => format!(
                        "CASE WHEN {is_key} THEN {} WHEN {} THEN {} ELSE {} END",
                        Matched::Key.code(),
                        self.matches(label, format!("%{escaped}%"), &mut params),
                        Matched::Label.code(),
                        Matched::KeyStart.code()
                    ),
                    None => format!(
                        "CASE WHEN {is_key} THEN {} ELSE {} END",
                        Matched::Key.code(),
                        Matched::KeyStart.code()
                    ),
                };
                let mut found = Vec::with_capacity(2);
                if let Some(label) = &label {
                    found.push(self.matches(label, format!("%{escaped}%"), &mut params));
                }
                found.push(self.matches(&key, format!("{escaped}%"), &mut params));
                let order = if label.is_some() { "3, 1" } else { "2, 1" };
                // `OFFSET 0` is for SQLite: without it the derived table
                // is folded into the outer query, and the whole table is
                // scanned and sorted before the limit.
                format!(
                    "SELECT {columns}, {how} FROM (SELECT {columns} FROM {table} WHERE {} \
                     LIMIT {limit} OFFSET 0) AS found ORDER BY {order}",
                    found.join(" OR ")
                )
            }
        };
        Sql { text, params }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        CellChange, Filter, FilterOp, InsertValue, NewValue, ObjectRef, RowChange, RowQuery, Sort,
        SortDir, Value,
    };

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

    fn change(column: &str, type_name: &str, new: NewValue) -> CellChange {
        CellChange {
            column: column.into(),
            type_name: type_name.into(),
            loaded: Value::Null,
            new,
        }
    }

    fn typed(column: &str, type_name: &str, new: &str) -> CellChange {
        change(column, type_name, NewValue::Text(new.into()))
    }

    fn one(key: Vec<(&str, Value)>, set: Vec<CellChange>) -> RowChange {
        RowChange {
            key: key
                .into_iter()
                .map(|(name, value)| (name.to_owned(), value))
                .collect(),
            set,
        }
    }

    fn books() -> ObjectRef {
        ObjectRef::new("public", "books")
    }

    /// The bound statement with each parameter written in place of its
    /// placeholder, as the builder would show it.
    fn inlined(dialect: Dialect, sql: &Sql) -> String {
        let mut params = sql.params.iter();
        let mut text = String::new();
        for (index, piece) in sql.text.split('?').enumerate() {
            if index > 0 {
                text.push_str(
                    &match params.next().expect("a value for each placeholder") {
                        Value::Int(number) => number.to_string(),
                        Value::Float(number) => format!("{number:?}"),
                        Value::Text(value) => dialect.literal(value),
                        Value::Bytes(bytes) => dialect.bytes_literal(bytes),
                        other => panic!("{other:?} is never bound"),
                    },
                );
            }
            text.push_str(piece);
        }
        assert!(params.next().is_none(), "a value without a placeholder");
        text
    }

    fn covers(schema: &str) -> ObjectRef {
        ObjectRef::new(schema, "book_covers")
    }

    fn sets(column: &str, type_name: &str, new: NewValue) -> InsertValue {
        InsertValue {
            column: column.into(),
            type_name: type_name.into(),
            new,
        }
    }

    fn doomed(key: Vec<(&str, Value)>) -> RowDelete {
        RowDelete {
            key: key
                .into_iter()
                .map(|(name, value)| (name.to_owned(), value))
                .collect(),
        }
    }

    /// Spec 4 and test 7: one statement for a row, by its key.
    #[test]
    fn the_same_delete_on_each_engine() {
        let row = doomed(vec![("id", Value::Int(9))]);
        let delete = Dialect::Postgres
            .delete_row(&covers("public"), &row)
            .unwrap();
        assert_eq!(
            delete.shown,
            r#"DELETE FROM "public"."book_covers" WHERE "id" = 9"#
        );
        // PostgreSQL runs exactly what it shows.
        assert_eq!(delete.sql.text, delete.shown);
        assert!(delete.sql.params.is_empty());

        let delete = Dialect::MySql
            .delete_row(&covers("bookshop"), &row)
            .unwrap();
        assert_eq!(
            delete.shown,
            "DELETE FROM `bookshop`.`book_covers` WHERE `id` = 9"
        );
        assert_eq!(
            delete.sql.text,
            "DELETE FROM `bookshop`.`book_covers` WHERE `id` = ?"
        );
        assert_eq!(delete.sql.params, [Value::Int(9)]);

        let delete = Dialect::Sqlite.delete_row(&covers("main"), &row).unwrap();
        assert_eq!(
            delete.shown,
            r#"DELETE FROM "main"."book_covers" WHERE "id" = 9"#
        );
        assert_eq!(
            delete.sql.text,
            r#"DELETE FROM "main"."book_covers" WHERE "id" = ?"#
        );
        assert_eq!(delete.sql.params, [Value::Int(9)]);
    }

    /// Spec 4: a key of several columns uses every one of them.
    #[test]
    fn a_delete_names_every_column_of_its_rows_key() {
        let authors = ObjectRef::new("public", "book_authors");
        let row = doomed(vec![
            ("book_id", Value::Int(101)),
            ("author", text("Sam O'Neil")),
        ]);
        let delete = Dialect::Postgres.delete_row(&authors, &row).unwrap();
        assert_eq!(
            delete.shown,
            r#"DELETE FROM "public"."book_authors" WHERE "book_id" = 101 AND "author" = 'Sam O''Neil'"#
        );
        for dialect in [Dialect::MySql, Dialect::Sqlite] {
            let delete = dialect.delete_row(&authors, &row).unwrap();
            assert_eq!(delete.sql.params, [Value::Int(101), text("Sam O'Neil")]);
            // What is shown is what is bound.
            assert_eq!(inlined(dialect, &delete.sql), delete.shown);
        }
    }

    #[test]
    fn a_delete_says_where_its_parts_stand() {
        let authors = ObjectRef::new("public", "book_authors");
        let row = doomed(vec![
            ("book_id", Value::Int(101)),
            ("author", text("Sam O'Neil")),
        ]);
        let delete = Dialect::Postgres.delete_row(&authors, &row).unwrap();
        let parts = &delete.parts;
        assert!(delete.shown[parts.filter..].starts_with("WHERE "));
        assert_eq!(
            &delete.shown[..parts.filter],
            r#"DELETE FROM "public"."book_authors" "#
        );
        assert!(delete.shown[parts.columns[0]..].starts_with(r#""book_id" = "#));
        assert!(delete.shown[parts.columns[1]..].starts_with(r#""author" = "#));
        assert_eq!(&delete.shown[parts.values[0].clone()], "101");
        assert_eq!(&delete.shown[parts.values[1].clone()], "'Sam O''Neil'");
    }

    /// Spec 4 and test 13 of the many-rows spec: a run of rows is one
    /// statement, `IN` its keys.
    #[test]
    fn a_run_of_deletes_on_each_engine() {
        let rows = [9, 10, 11].map(|id| doomed(vec![("id", Value::Int(id))]));
        let rows: Vec<&RowDelete> = rows.iter().collect();
        let run = Dialect::Postgres
            .delete_run(&covers("public"), &rows)
            .unwrap();
        assert_eq!(
            run.shown,
            r#"DELETE FROM "public"."book_covers" WHERE "id" IN (9, 10, 11)"#
        );
        assert_eq!(run.sql.text, run.shown);
        assert!(run.sql.params.is_empty());
        let run = Dialect::MySql
            .delete_run(&covers("bookshop"), &rows)
            .unwrap();
        assert_eq!(
            run.sql.text,
            "DELETE FROM `bookshop`.`book_covers` WHERE `id` IN (?, ?, ?)"
        );
        assert_eq!(run.sql.params, [9, 10, 11].map(Value::Int));
        assert_eq!(inlined(Dialect::MySql, &run.sql), run.shown);
        let run = Dialect::Sqlite.delete_run(&covers("main"), &rows).unwrap();
        assert_eq!(
            run.sql.text,
            r#"DELETE FROM "main"."book_covers" WHERE "id" IN (?, ?, ?)"#
        );
        assert_eq!(inlined(Dialect::Sqlite, &run.sql), run.shown);
    }

    /// A key of several columns: rows on the right of `IN`, which SQLite
    /// takes only as `VALUES`.
    #[test]
    fn a_run_by_a_key_of_two_columns_on_each_engine() {
        let authors = ObjectRef::new("public", "book_authors");
        let row =
            |author: &str| doomed(vec![("book_id", Value::Int(101)), ("author", text(author))]);
        let rows = [row("Ada Quill"), row("Sam O'Neil")];
        let rows: Vec<&RowDelete> = rows.iter().collect();
        let run = Dialect::Postgres.delete_run(&authors, &rows).unwrap();
        assert_eq!(
            run.shown,
            r#"DELETE FROM "public"."book_authors" WHERE ("book_id", "author") IN ((101, 'Ada Quill'), (101, 'Sam O''Neil'))"#
        );
        // Its parts: each column once, each value in the rows' order.
        let parts = &run.parts;
        assert!(run.shown[parts.filter..].starts_with("WHERE "));
        assert_eq!(parts.columns.len(), 2);
        assert!(run.shown[parts.columns[0]..].starts_with(r#""book_id""#));
        assert!(run.shown[parts.columns[1]..].starts_with(r#""author""#));
        let values: Vec<&str> = parts
            .values
            .iter()
            .map(|at| &run.shown[at.clone()])
            .collect();
        assert_eq!(values, ["101", "'Ada Quill'", "101", "'Sam O''Neil'"]);
        let run = Dialect::MySql.delete_run(&authors, &rows).unwrap();
        assert_eq!(
            run.sql.text,
            "DELETE FROM `public`.`book_authors` WHERE (`book_id`, `author`) IN ((?, ?), (?, ?))"
        );
        assert_eq!(inlined(Dialect::MySql, &run.sql), run.shown);
        let run = Dialect::Sqlite.delete_run(&authors, &rows).unwrap();
        assert_eq!(
            run.sql.text,
            r#"DELETE FROM "public"."book_authors" WHERE ("book_id", "author") IN (VALUES (?, ?), (?, ?))"#
        );
        assert_eq!(inlined(Dialect::Sqlite, &run.sql), run.shown);
    }

    /// A run of rows whose keys differ, or of fewer than two, is no run.
    #[test]
    fn a_run_needs_two_rows_with_the_same_key_columns() {
        let one = doomed(vec![("id", Value::Int(9))]);
        let other = doomed(vec![("isbn", Value::Int(9))]);
        let covers = covers("public");
        assert!(Dialect::Postgres.delete_run(&covers, &[&one]).is_err());
        assert!(
            Dialect::Postgres
                .delete_run(&covers, &[&one, &other])
                .is_err()
        );
        // A row no save sends refuses the run it is in.
        let lossy = doomed(vec![("id", text("a\u{FFFD}"))]);
        let fine = doomed(vec![("id", text("a"))]);
        assert!(
            Dialect::Sqlite
                .delete_run(&covers, &[&fine, &lossy])
                .is_err()
        );
        assert!(Dialect::Sqlite.delete_run(&covers, &[&fine, &fine]).is_ok());
        // Nor do a number and a text in one column make a run.
        let number = doomed(vec![("id", Value::Int(9))]);
        let words = doomed(vec![("id", text("18446744073709551615"))]);
        assert!(
            Dialect::MySql
                .delete_run(&covers, &[&number, &words])
                .is_err()
        );
    }

    /// Spec 4, step 5: what each engine says when it was busy, not when a
    /// row was wrong.
    #[test]
    fn each_engine_says_it_was_busy_in_its_own_codes() {
        let coded = |code: &str, number: Option<u32>| Error::Query {
            code: Some(code.into()),
            message: "no".into(),
            detail: None,
            hint: None,
            named: Box::new(crate::Named {
                number,
                constraint: None,
                columns: Vec::new(),
            }),
        };
        for code in ["40P01", "55P03", "40001"] {
            assert!(Dialect::Postgres.busy(&coded(code, None)), "{code}");
        }
        assert!(!Dialect::Postgres.busy(&coded("23503", None)));
        for number in [1205, 1213] {
            assert!(
                Dialect::MySql.busy(&coded("HY000", Some(number))),
                "{number}"
            );
        }
        assert!(!Dialect::MySql.busy(&coded("23000", Some(1451))));
        // SQLite's code is the extended one: busy (5) and locked (6) under
        // whatever stands above them.
        for code in ["5", "6", "261", "517", "262"] {
            assert!(Dialect::Sqlite.busy(&coded(code, None)), "{code}");
        }
        assert!(!Dialect::Sqlite.busy(&coded("787", None)));
        assert!(!Dialect::Sqlite.busy(&coded("1811", None)));
        assert!(!Dialect::Sqlite.busy(&Error::query("no")));
    }

    /// A key that may not be the row's deletes no row: it could be
    /// another's.
    #[test]
    fn a_delete_is_refused_where_its_key_is_not_known_to_be_the_rows() {
        let lossy = doomed(vec![("isbn", text("978\u{FFFD}"))]);
        assert!(Dialect::Sqlite.delete_row(&books(), &lossy).is_err());
        // The other engines read a key exactly.
        assert!(Dialect::Postgres.delete_row(&books(), &lossy).is_ok());
        assert!(Dialect::MySql.delete_row(&books(), &lossy).is_ok());
        let nul = doomed(vec![("isbn", text("978\0"))]);
        assert_eq!(
            Dialect::Postgres.delete_row(&books(), &nul),
            Err(Error::query("PostgreSQL text cannot hold a NUL character"))
        );
    }

    /// The canvas' new row: a cover of the publisher Harbor Press.
    const PUBLISHER: &str = "9100000000000000004";

    #[test]
    fn default_and_now_are_written_as_keywords_never_bound() {
        let key = vec![("id".to_owned(), Value::Int(2))];
        let change = |column: &str, type_name: &str, new: NewValue| CellChange {
            column: column.into(),
            type_name: type_name.into(),
            loaded: Value::Null,
            new,
        };
        let default = || NewValue::Default {
            expression: Some("'print'".into()),
        };
        for (dialect, schema, stamp, kind) in [
            (Dialect::Postgres, "public", "now()", "DEFAULT"),
            (
                Dialect::MySql,
                "bookshop",
                "CURRENT_TIMESTAMP(6)",
                "DEFAULT",
            ),
            // SQLite has no DEFAULT to write: the expression itself.
            (Dialect::Sqlite, "main", "CURRENT_TIMESTAMP", "('print')"),
        ] {
            let row = RowChange {
                key: key.clone(),
                set: vec![
                    change("kind", "varchar(20)", default()),
                    change("created_at", "timestamp", NewValue::Now),
                    change("printed_on", "date", NewValue::Now),
                ],
            };
            let update = dialect.update_row(&covers(schema), &row).unwrap();
            let literals: Vec<&str> = update.parts.values[..3]
                .iter()
                .map(|range| &update.shown[range.clone()])
                .collect();
            assert_eq!(literals, [kind, stamp, "CURRENT_DATE"], "{dialect:?}");
            // Shown and sent alike: only the key's value is bound.
            for literal in &literals {
                assert!(update.sql.text.contains(literal), "{dialect:?}: {literal}");
            }
            let bound = match dialect {
                Dialect::Postgres => 0,
                Dialect::MySql | Dialect::Sqlite => 1,
            };
            assert_eq!(update.sql.params.len(), bound, "{dialect:?}");
            // A new row's values are written the same way.
            let insert = RowInsert {
                set: vec![sets("created_at", "timestamp", NewValue::Now)],
            };
            let insert = dialect.insert_row(&covers(schema), &insert).unwrap();
            let range = insert.parts.literals[0].clone();
            assert_eq!(&insert.shown[range], stamp, "{dialect:?}");
            assert!(insert.sql.params.is_empty(), "{dialect:?}");
        }
        // The time of day, where the column keeps only that.
        let time = |dialect: Dialect| {
            let row = RowChange {
                key: key.clone(),
                set: vec![change("opens_at", "time", NewValue::Now)],
            };
            let update = dialect.update_row(&covers("main"), &row).unwrap();
            update.shown[update.parts.values[0].clone()].to_owned()
        };
        assert_eq!(time(Dialect::Postgres), "CURRENT_TIME");
        assert_eq!(time(Dialect::MySql), "CURRENT_TIME(6)");
        assert_eq!(time(Dialect::Sqlite), "CURRENT_TIME");
        // A column with no default, on SQLite: NULL is what it would get.
        let none = RowChange {
            key: key.clone(),
            set: vec![change(
                "note",
                "text",
                NewValue::Default { expression: None },
            )],
        };
        let update = Dialect::Sqlite.update_row(&covers("main"), &none).unwrap();
        assert_eq!(&update.shown[update.parts.values[0].clone()], "NULL");
        // The time of the save is no value for a number.
        let wrong = RowChange {
            key,
            set: vec![change("pages", "integer", NewValue::Now)],
        };
        let error = Dialect::Postgres
            .update_row(&covers("public"), &wrong)
            .unwrap_err();
        assert_eq!(
            error.to_string(),
            "pages: integer expects a value, not the time of the save"
        );
    }

    #[test]
    fn a_new_rows_parts_are_where_its_statement_has_them() {
        let object = covers("public");
        // A value that reads like the statement's own words moves nothing.
        let wordy = ") VALUES ('x') RETURNING *";
        let row = RowInsert {
            set: vec![
                sets("kind", "text", NewValue::Text("print".into())),
                sets("note", "text", NewValue::Text(wordy.into())),
            ],
        };
        let nothing = RowInsert { set: Vec::new() };
        for dialect in [Dialect::Postgres, Dialect::MySql, Dialect::Sqlite] {
            let InsertStatement { shown, parts, .. } = dialect.insert_row(&object, &row).unwrap();
            assert!(shown[parts.values..].starts_with("VALUES ("), "{shown}");
            assert!(shown[..parts.values].ends_with(") "), "{shown}");
            let literals: Vec<&str> = parts
                .literals
                .iter()
                .map(|range| &shown[range.clone()])
                .collect();
            // Each is the whole literal, quotes and all, and nothing else.
            assert_eq!(literals.len(), 2, "{shown}");
            assert_eq!(literals[0], "'print'", "{shown}");
            let long = literals[1];
            assert!(long.starts_with('\'') && long.ends_with('\''), "{long}");
            assert!(
                long.contains("VALUES (") && long.contains("RETURNING"),
                "{long}"
            );
            assert_eq!(&shown[parts.literals[0].end..parts.literals[1].start], ", ");
            let after = &shown[parts.literals[1].end..];
            let back = parts.back.map(|at| &shown[at..]);
            match dialect {
                Dialect::Postgres | Dialect::Sqlite => {
                    assert_eq!(after, ") RETURNING *");
                    assert_eq!(back, Some("RETURNING *"));
                }
                Dialect::MySql => {
                    assert_eq!(after, ")");
                    assert_eq!(back, None);
                }
            }
            // With nothing set: the clause that takes every default.
            let InsertStatement { shown, parts, .. } =
                dialect.insert_row(&object, &nothing).unwrap();
            assert!(parts.literals.is_empty());
            let clause = &shown[parts.values..];
            assert!(shown[..parts.values].ends_with(' '), "{shown}");
            match dialect {
                Dialect::Postgres | Dialect::Sqlite => {
                    assert_eq!(clause, "DEFAULT VALUES RETURNING *");
                    assert_eq!(parts.back.map(|at| &shown[at..]), Some("RETURNING *"));
                }
                Dialect::MySql => {
                    assert_eq!(clause, "VALUES ()");
                    assert_eq!(parts.back, None);
                }
            }
        }
    }

    #[test]
    fn the_same_new_row_on_each_engine() {
        let row = |type_name: &str| RowInsert {
            set: vec![sets(
                "publisher_id",
                type_name,
                NewValue::Text(PUBLISHER.into()),
            )],
        };
        let insert = Dialect::Postgres
            .insert_row(&covers("public"), &row("bigint"))
            .unwrap();
        assert_eq!(
            insert.shown,
            r#"INSERT INTO "public"."book_covers" ("publisher_id") VALUES ('9100000000000000004') RETURNING *"#
        );
        // PostgreSQL runs exactly what it shows.
        assert_eq!(insert.sql.text, insert.shown);
        assert!(insert.sql.params.is_empty());

        let insert = Dialect::MySql
            .insert_row(&covers("bookshop"), &row("bigint"))
            .unwrap();
        assert_eq!(
            insert.shown,
            "INSERT INTO `bookshop`.`book_covers` (`publisher_id`) VALUES ('9100000000000000004')"
        );
        assert_eq!(
            insert.sql.text,
            "INSERT INTO `bookshop`.`book_covers` (`publisher_id`) VALUES (?)"
        );
        assert_eq!(insert.sql.params, [text(PUBLISHER)]);

        // SQLite stores text as text: a number is made one here.
        let insert = Dialect::Sqlite
            .insert_row(&covers("main"), &row("INTEGER"))
            .unwrap();
        assert_eq!(
            insert.shown,
            r#"INSERT INTO "main"."book_covers" ("publisher_id") VALUES (9100000000000000004) RETURNING *"#
        );
        assert_eq!(
            insert.sql.text,
            r#"INSERT INTO "main"."book_covers" ("publisher_id") VALUES (?) RETURNING *"#
        );
        assert_eq!(insert.sql.params, [Value::Int(9_100_000_000_000_000_004)]);
    }

    #[test]
    fn a_new_row_names_only_what_was_set() {
        let row = RowInsert {
            set: vec![
                sets("publisher_id", "bigint", NewValue::Text(PUBLISHER.into())),
                sets("image_data", "jsonb", NewValue::Null),
            ],
        };
        let insert = Dialect::Postgres
            .insert_row(&covers("public"), &row)
            .unwrap();
        // Not `kind`, `created_at` or `id`: the database fills those.
        assert_eq!(
            insert.shown,
            r#"INSERT INTO "public"."book_covers" ("publisher_id", "image_data") VALUES ('9100000000000000004', NULL) RETURNING *"#
        );
        let insert = Dialect::MySql
            .insert_row(&covers("bookshop"), &row)
            .unwrap();
        // NULL is written out, never bound, as in an UPDATE.
        assert_eq!(
            insert.sql.text,
            "INSERT INTO `bookshop`.`book_covers` (`publisher_id`, `image_data`) VALUES (?, NULL)"
        );
        assert_eq!(insert.sql.params, [text(PUBLISHER)]);
    }

    #[test]
    fn a_new_row_with_nothing_set_takes_every_default() {
        let row = RowInsert { set: Vec::new() };
        let shown = |dialect: Dialect, schema: &str| {
            let insert = dialect.insert_row(&covers(schema), &row).unwrap();
            assert_eq!(insert.sql.text, insert.shown);
            assert!(insert.sql.params.is_empty());
            insert.shown
        };
        assert_eq!(
            shown(Dialect::Postgres, "public"),
            r#"INSERT INTO "public"."book_covers" DEFAULT VALUES RETURNING *"#
        );
        assert_eq!(
            shown(Dialect::Sqlite, "main"),
            r#"INSERT INTO "main"."book_covers" DEFAULT VALUES RETURNING *"#
        );
        // MySQL has no DEFAULT VALUES.
        assert_eq!(
            shown(Dialect::MySql, "bookshop"),
            "INSERT INTO `bookshop`.`book_covers` () VALUES ()"
        );
    }

    #[test]
    fn a_new_value_its_column_cannot_take_is_refused_before_it_is_sent() {
        let row = |type_name: &str, new: &str| RowInsert {
            set: vec![sets("n", type_name, NewValue::Text(new.into()))],
        };
        let built = |dialect: Dialect, schema: &str, type_name: &str, new: &str| {
            dialect.insert_row(&covers(schema), &row(type_name, new))
        };
        // SQLite would store the text as it is.
        assert!(built(Dialect::Sqlite, "main", "INTEGER", "seven").is_err());
        // Bytes are not sent as text, for a new row as for a changed one.
        assert!(built(Dialect::Postgres, "public", "bytea", "x").is_err());
        // PostgreSQL text cannot hold a NUL.
        assert!(built(Dialect::Postgres, "public", "text", "a\0b").is_err());
        assert!(built(Dialect::Sqlite, "main", "TEXT", "a\0b").is_ok());
    }

    #[test]
    fn an_update_is_shown_with_its_values_as_literals() {
        let row = one(
            vec![("id", Value::Int(2))],
            vec![
                typed("kind", "character varying(20)", "ebook"),
                change("alt_text", "text", NewValue::Null),
            ],
        );
        let update = Dialect::Postgres.update_row(&books(), &row).unwrap();
        assert_eq!(
            update.shown,
            r#"UPDATE "public"."books" SET "kind" = 'ebook', "alt_text" = NULL WHERE "id" = 2"#
        );
        // PostgreSQL runs exactly what it shows.
        assert_eq!(update.sql.text, update.shown);
        assert!(update.sql.params.is_empty());
        let update = Dialect::MySql.update_row(&books(), &row).unwrap();
        assert_eq!(
            update.shown,
            "UPDATE `public`.`books` SET `kind` = 'ebook', `alt_text` = NULL WHERE `id` = 2"
        );
        assert_eq!(
            update.sql.text,
            "UPDATE `public`.`books` SET `kind` = ?, `alt_text` = NULL WHERE `id` = ?"
        );
        assert_eq!(update.sql.params, [text("ebook"), Value::Int(2)]);
    }

    #[test]
    fn an_update_says_where_its_parts_stand() {
        // Names and values that hold what the statement is made of: no
        // reading of the text would find its parts.
        let row = one(
            vec![("id", Value::Int(2)), ("code", text("a WHERE b"))],
            vec![
                typed("kind", "text", "it's, SET = 'x' WHERE 1"),
                change("alt_text", "text", NewValue::Null),
                typed("note", "text", "caf\u{e9} \\ \u{1F600}"),
            ],
        );
        let object = ObjectRef::new("public", "a SET b");
        for dialect in [Dialect::Postgres, Dialect::MySql, Dialect::Sqlite] {
            let update = dialect.update_row(&object, &row).unwrap();
            let (shown, parts) = (&update.shown, &update.parts);
            assert!(shown[parts.set..].starts_with("SET "), "{dialect:?}");
            // The table's name ends where ` SET` begins.
            assert_eq!(
                &shown[..parts.set - 1],
                format!("UPDATE {}", dialect.qualified(&object)),
                "{dialect:?}"
            );
            let values: Vec<&str> = parts
                .values
                .iter()
                .map(|range| &shown[range.clone()])
                .collect();
            assert_eq!(
                values,
                [
                    dialect.literal("it's, SET = 'x' WHERE 1").as_str(),
                    "NULL",
                    dialect.literal("caf\u{e9} \\ \u{1F600}").as_str(),
                    "2",
                    dialect.literal("a WHERE b").as_str(),
                ],
                "{dialect:?}"
            );
            // In the statement's order, the new values before `WHERE` and
            // the key's after it.
            assert!(parts.values.is_sorted_by(|a, b| a.end <= b.start));
            let key = &shown[parts.values[2].end..parts.columns[3]];
            assert_eq!(key, " WHERE ", "{dialect:?}");
            // From where its column begins to the value: the name and `=`,
            // whatever stands between two of them.
            let leads: Vec<&str> = parts
                .columns
                .iter()
                .zip(&parts.values)
                .map(|(column, value)| &shown[*column..value.start])
                .collect();
            let names = ["kind", "alt_text", "note", "id", "code"];
            let expected: Vec<String> = names
                .iter()
                .map(|name| format!("{} = ", dialect.quote_ident(name)))
                .collect();
            assert_eq!(leads, expected, "{dialect:?}");
        }
        // SQLite's text around a NUL is one value, its brackets with it.
        let row = one(
            vec![("id", Value::Int(2))],
            vec![typed("note", "text", "a\0b"), typed("kind", "text", "c")],
        );
        let update = Dialect::Sqlite.update_row(&object, &row).unwrap();
        let values: Vec<&str> = update
            .parts
            .values
            .iter()
            .map(|range| &update.shown[range.clone()])
            .collect();
        assert_eq!(values, ["('a' || char(0) || 'b')", "'c'", "2"]);
    }

    #[test]
    fn what_is_shown_is_what_is_bound() {
        let row = one(
            vec![
                ("id", Value::Int(7)),
                ("code", text("it's")),
                ("uid", Value::Bytes(vec![0x01, 0xab].into())),
            ],
            vec![
                typed("title", "TEXT", "O'Brien \\ co"),
                typed("pages", "INTEGER", "612"),
                typed("price", "REAL", "12.5"),
                typed("in_print", "BOOLEAN", "true"),
                change("note", "TEXT", NewValue::Null),
            ],
        );
        for dialect in [Dialect::MySql, Dialect::Sqlite] {
            let row = match dialect {
                // MySQL's own names for the same columns.
                Dialect::MySql => one(
                    row.key
                        .iter()
                        .map(|(name, value)| (name.as_str(), value.clone()))
                        .collect(),
                    vec![
                        typed("title", "varchar(200)", "O'Brien \\ co"),
                        typed("pages", "int", "612"),
                        typed("price", "double", "12.5"),
                        typed("in_print", "tinyint(1)", "true"),
                        change("note", "text", NewValue::Null),
                    ],
                ),
                _ => row.clone(),
            };
            let update = dialect.update_row(&books(), &row).unwrap();
            assert_eq!(inlined(dialect, &update.sql), update.shown, "{dialect:?}");
            let select = dialect.select_row(&books(), &row.key, true);
            assert_eq!(select.params.len(), 3, "{dialect:?}");
        }
    }

    #[test]
    fn a_values_form_follows_its_columns_class() {
        let shown = |dialect: Dialect, cell: CellChange| {
            dialect
                .update_row(&books(), &one(vec![("id", Value::Int(1))], vec![cell]))
                .map(|update| update.shown)
        };
        let set = |dialect: Dialect, cell: CellChange| {
            let shown = shown(dialect, cell).unwrap();
            let start = shown.find(" = ").unwrap() + 3;
            shown[start..shown.find(" WHERE").unwrap()].to_owned()
        };
        // PostgreSQL converts text itself, whatever the type.
        assert_eq!(set(Dialect::Postgres, typed("n", "integer", "12")), "'12'");
        assert_eq!(
            set(Dialect::Postgres, typed("b", "boolean", "true")),
            "'true'"
        );
        // A backslash needs the escape form there, and only then.
        assert_eq!(
            set(Dialect::Postgres, typed("t", "text", r"a\b")),
            r"E'a\\b'"
        );
        assert_eq!(
            set(Dialect::Postgres, typed("t", "text", "it's")),
            "'it''s'"
        );
        // MySQL converts text too, but a boolean is 1 or 0.
        assert_eq!(set(Dialect::MySql, typed("n", "int", "12")), "'12'");
        assert_eq!(set(Dialect::MySql, typed("b", "tinyint(1)", "false")), "0");
        assert_eq!(set(Dialect::MySql, typed("b", "tinyint(1)", "1")), "1");
        assert_eq!(
            set(Dialect::MySql, typed("t", "text", r"a\b'c")),
            r"'a\\b''c'"
        );
        // SQLite stores what it is given, so numbers go as numbers.
        assert_eq!(set(Dialect::Sqlite, typed("n", "INTEGER", " 12 ")), "12");
        assert_eq!(set(Dialect::Sqlite, typed("x", "REAL", "1")), "1.0");
        assert_eq!(set(Dialect::Sqlite, typed("d", "NUMERIC", "12")), "12");
        assert_eq!(set(Dialect::Sqlite, typed("d", "NUMERIC", "12.50")), "12.5");
        assert_eq!(set(Dialect::Sqlite, typed("b", "BOOLEAN", "TRUE")), "1");
        assert_eq!(set(Dialect::Sqlite, typed("t", "TEXT", "12")), "'12'");
        assert_eq!(set(Dialect::Sqlite, typed("t", "", "12")), "'12'");
        // A column with no type keeps a number a number, where it held one.
        let held = |loaded: Value, new: &str| CellChange {
            loaded,
            ..typed("t", "", new)
        };
        assert_eq!(set(Dialect::Sqlite, held(Value::Int(5), "6")), "6");
        assert_eq!(set(Dialect::Sqlite, held(Value::Float(1.5), "2")), "2");
        assert_eq!(set(Dialect::Sqlite, held(Value::Int(5), "six")), "'six'");
        assert_eq!(
            set(Dialect::Sqlite, held(Value::Text("5".into()), "6")),
            "'6'"
        );
        // A MySQL tinyint(1) takes what a tinyint holds.
        assert_eq!(set(Dialect::MySql, typed("b", "tinyint(1)", "true")), "1");
        assert_eq!(set(Dialect::MySql, typed("b", "tinyint(1)", "5")), "5");
        // What cannot be converted is refused, with the column and its type.
        for (dialect, cell) in [
            (Dialect::Sqlite, typed("pages", "INTEGER", "many")),
            (Dialect::Sqlite, typed("pages", "INTEGER", "1.5")),
            (Dialect::Sqlite, typed("price", "REAL", "NaN")),
            (Dialect::Sqlite, typed("in_print", "BOOLEAN", "maybe")),
            (Dialect::MySql, typed("in_print", "tinyint(1)", "yes")),
            (Dialect::MySql, typed("in_print", "tinyint(1)", "128")),
            // Binary columns are never sent as text.
            (Dialect::MySql, typed("flags", "bit(8)", "1")),
            (Dialect::Postgres, typed("cover", "bytea", "x")),
            (Dialect::Sqlite, typed("cover", "BLOB", "x")),
            // Nor set to NULL: they are not edited at all.
            (Dialect::MySql, change("flags", "bit(8)", NewValue::Null)),
            (Dialect::Postgres, change("cover", "bytea", NewValue::Null)),
            (Dialect::Sqlite, change("cover", "BLOB", NewValue::Null)),
            // Nor a blob held by a column of another type, or of none.
            (
                Dialect::Sqlite,
                CellChange {
                    loaded: Value::Bytes(vec![1, 2].into()),
                    ..typed("note", "TEXT", "x")
                },
            ),
            (
                Dialect::Sqlite,
                CellChange {
                    loaded: Value::Bytes(vec![1, 2].into()),
                    ..change("loose", "", NewValue::Null)
                },
            ),
        ] {
            let column = cell.column.clone();
            let refused = shown(dialect, cell).unwrap_err().to_string();
            assert!(refused.starts_with(&column), "{refused}");
        }
    }

    #[test]
    fn sqlite_is_shown_what_its_parser_reads() {
        let update = |dialect: Dialect, key: Value, new: &str| {
            dialect
                .update_row(
                    &books(),
                    &one(vec![("k", key)], vec![typed("t", "text", new)]),
                )
                .unwrap()
        };
        // SQLite's parser stops at a NUL, in a string too, so the text is
        // joined around `char(0)`. The bound statement holds the text
        // itself.
        let nul = update(Dialect::Sqlite, text("a\0'"), "x\0y");
        assert_eq!(
            nul.shown,
            r#"UPDATE "public"."books" SET "t" = ('x' || char(0) || 'y') WHERE "k" = ('a' || char(0) || '''')"#
        );
        // A NUL at either end leaves an empty string beside it.
        assert!(
            update(Dialect::Sqlite, Value::Int(1), "\0")
                .shown
                .contains(r#""t" = ('' || char(0) || '')"#)
        );
        assert_eq!(nul.sql.params, [text("x\0y"), text("a\0'")]);
        // Text without one stays a plain string.
        assert!(
            update(Dialect::Sqlite, Value::Int(1), "x0y")
                .shown
                .contains(r#""t" = 'x0y'"#)
        );
        // `inf` would be read as a column's name. A number past the largest
        // real is how SQLite itself writes infinity.
        for (number, shown) in [(f64::INFINITY, "9e999"), (f64::NEG_INFINITY, "-9e999")] {
            let update = update(Dialect::Sqlite, Value::Float(number), "x");
            assert!(
                update.shown.ends_with(&format!(r#" WHERE "k" = {shown}"#)),
                "{}",
                update.shown
            );
            assert_eq!(update.sql.params[1], Value::Float(number));
        }
        // The others keep their own forms. MySQL's is the escape its
        // strings have for a NUL.
        assert!(
            update(Dialect::MySql, text("a\0'"), "x\0y")
                .shown
                .ends_with(r"SET `t` = 'x\0y' WHERE `k` = 'a\0'''")
        );
        assert!(
            update(Dialect::MySql, Value::Float(f64::INFINITY), "x")
                .shown
                .ends_with("WHERE `k` = inf")
        );
        assert!(
            update(Dialect::Postgres, Value::Float(f64::INFINITY), "x")
                .shown
                .ends_with(r#"WHERE "k" = 'inf'"#)
        );
    }

    #[test]
    fn a_row_no_save_would_send_has_no_statement() {
        let update = |dialect: Dialect, key: Value, new: &str| {
            dialect.update_row(
                &books(),
                &one(vec![("k", key)], vec![typed("t", "text", new)]),
            )
        };
        // PostgreSQL text cannot hold a NUL: in a new value and in the key
        // alike. What is shown is what a save runs, so a row a save
        // refuses is refused here, where the review reads it.
        for (key, new) in [(text("a"), "b\0c"), (text("a\0"), "b")] {
            let refused = update(Dialect::Postgres, key, new).unwrap_err();
            assert_eq!(
                refused.to_string(),
                "PostgreSQL text cannot hold a NUL character"
            );
        }
        assert!(update(Dialect::Postgres, text("a"), "b").is_ok());
        // The others hold one, each in its own writing.
        for dialect in [Dialect::MySql, Dialect::Sqlite] {
            assert!(update(dialect, text("a\0"), "b\0c").is_ok(), "{dialect:?}");
        }
        // SQLite reads text that is not UTF-8 with U+FFFD for its bad
        // bytes: a key that holds one may be another row's.
        let inexact = "the row's key holds text that may not have been read exactly, so the \
                       save cannot be sure which row it names";
        let refused = update(Dialect::Sqlite, text("caf\u{FFFD}"), "b").unwrap_err();
        assert_eq!(refused.to_string(), inexact);
        // Said before any value of the row is looked at.
        let row = one(
            vec![("id", Value::Int(1)), ("k", text("caf\u{FFFD}"))],
            vec![typed("n", "INTEGER", "abc")],
        );
        let refused = Dialect::Sqlite.update_row(&books(), &row).unwrap_err();
        assert_eq!(refused.to_string(), inexact);
        // Only in the key, and only there: a new value may hold the
        // character, and the other two read their text exactly.
        assert!(update(Dialect::Sqlite, text("a"), "caf\u{FFFD}").is_ok());
        for dialect in [Dialect::Postgres, Dialect::MySql] {
            assert!(
                update(dialect, text("caf\u{FFFD}"), "b").is_ok(),
                "{dialect:?}"
            );
        }
    }

    #[test]
    fn mysql_writes_a_nul_as_its_escape() {
        let literal = |text: &str| Dialect::MySql.literal(text);
        // No raw NUL in what is shown and copied: a clipboard, or what the
        // text is pasted into, may end text at one, and the statement
        // would lose its `WHERE`.
        assert_eq!(literal("a\0b'\\"), r"'a\0b''\\'");
        assert_eq!(literal("\0"), r"'\0'");
        // Backslashes are doubled first, so the one the escape is written
        // with is no half of a pair: a backslash and a NUL are three.
        assert_eq!(literal("\\\0"), r"'\\\0'");
        // And a backslash and a zero that were typed are no NUL.
        assert_eq!(literal(r"\0"), r"'\\0'");
        for text in ["a\0b'\\", "\0", "\\\0\0'", "s:1:\"\0\";"] {
            assert!(!literal(text).contains('\0'), "{text:?}");
        }
        // Only what is shown: the save binds the text as it is.
        let update = Dialect::MySql
            .update_row(
                &books(),
                &one(vec![("k", text("a\0'"))], vec![typed("t", "text", "x\0y")]),
            )
            .unwrap();
        assert_eq!(
            update.shown,
            r"UPDATE `public`.`books` SET `t` = 'x\0y' WHERE `k` = 'a\0'''"
        );
        assert_eq!(update.sql.params, [text("x\0y"), text("a\0'")]);
        assert_eq!(inlined(Dialect::MySql, &update.sql), update.shown);
    }

    #[test]
    fn a_row_is_found_by_its_key_in_the_drivers_own_form() {
        let key = |value: Value| vec![("id".to_owned(), value)];
        let select = |dialect: Dialect, value: Value, lock: bool| {
            dialect.select_row(&books(), &key(value), lock)
        };
        assert_eq!(
            select(Dialect::Postgres, Value::Int(2), true).text,
            r#"SELECT * FROM "public"."books" WHERE "id" = 2 LIMIT 2 FOR UPDATE"#
        );
        assert_eq!(
            select(Dialect::Postgres, text("a-b"), false).text,
            r#"SELECT * FROM "public"."books" WHERE "id" = 'a-b' LIMIT 2"#
        );
        assert_eq!(
            select(
                Dialect::Postgres,
                Value::Bytes(vec![0x01, 0xab].into()),
                false
            )
            .text,
            r#"SELECT * FROM "public"."books" WHERE "id" = E'\\x01ab' LIMIT 2"#
        );
        assert_eq!(
            select(Dialect::Postgres, Value::Bool(true), false).text,
            r#"SELECT * FROM "public"."books" WHERE "id" = 'true' LIMIT 2"#
        );
        let mysql = select(Dialect::MySql, Value::Bytes(vec![0x01, 0xab].into()), true);
        assert_eq!(
            mysql.text,
            "SELECT * FROM `public`.`books` WHERE `id` = ? LIMIT 2 FOR UPDATE"
        );
        assert_eq!(mysql.params, [Value::Bytes(vec![0x01, 0xab].into())]);
        // SQLite has no row locks: its transaction holds the file.
        assert_eq!(
            select(Dialect::Sqlite, Value::Int(2), true).text,
            r#"SELECT * FROM "public"."books" WHERE "id" = ? LIMIT 2"#
        );
        // Several columns are all asked for.
        let pair = vec![("a".to_owned(), Value::Int(1)), ("b".to_owned(), text("x"))];
        assert_eq!(
            Dialect::Postgres.select_row(&books(), &pair, false).text,
            r#"SELECT * FROM "public"."books" WHERE "a" = 1 AND "b" = 'x' LIMIT 2"#
        );
    }

    fn looking(label: Option<&str>, part: LookupPart) -> Lookup {
        Lookup {
            object: ObjectRef::new("main", "publishers"),
            key: "id".into(),
            label: label.map(str::to_owned),
            part,
            limit: 50,
        }
    }

    fn texts(sql: &Sql) -> Vec<&str> {
        sql.params
            .iter()
            .map(|value| match value {
                Value::Text(text) => &**text,
                other => panic!("a bound value is {other:?}"),
            })
            .collect()
    }

    #[test]
    fn a_lookup_reads_the_matches_inside_the_limit_and_orders_outside_it() {
        let sql =
            Dialect::Sqlite.lookup(&looking(Some("name"), LookupPart::Matching("harb".into())));
        assert_eq!(
            sql.text,
            "SELECT \"id\", \"name\", \
             CASE WHEN CAST(\"id\" AS TEXT) = ? THEN 0 \
             WHEN CAST(\"name\" AS TEXT) LIKE ? ESCAPE '\\' THEN 1 ELSE 2 END \
             FROM (SELECT \"id\", \"name\" FROM \"main\".\"publishers\" \
             WHERE CAST(\"name\" AS TEXT) LIKE ? ESCAPE '\\' \
             OR CAST(\"id\" AS TEXT) LIKE ? ESCAPE '\\' LIMIT 51 OFFSET 0) AS found \
             ORDER BY 3, 1"
        );
        // In the order the statement names them: these bind by position.
        assert_eq!(texts(&sql), ["harb", "%harb%", "%harb%", "harb%"]);
    }

    #[test]
    fn a_lookup_compares_as_the_contains_filter_does_on_each_engine() {
        let lookup = looking(Some("name"), LookupPart::Matching("harb".into()));
        let pg = Dialect::Postgres.lookup(&lookup);
        assert!(pg.params.is_empty());
        assert!(
            pg.text.contains(r#"CAST("name" AS TEXT) ILIKE E'%harb%'"#),
            "{}",
            pg.text
        );
        assert!(
            pg.text.contains(r#"CAST("id" AS TEXT) ILIKE E'harb%'"#),
            "{}",
            pg.text
        );
        assert!(
            pg.text.contains(r#"FROM "main"."publishers" WHERE"#),
            "{}",
            pg.text
        );
        let my = Dialect::MySql.lookup(&lookup);
        assert!(
            my.text.contains("CAST(`name` AS CHAR) LIKE ?"),
            "{}",
            my.text
        );
        assert!(
            my.text
                .contains("LIMIT 51 OFFSET 0) AS found ORDER BY 3, 1"),
            "{}",
            my.text
        );
        assert_eq!(texts(&my), ["harb", "%harb%", "%harb%", "harb%"]);
    }

    #[test]
    fn a_lookup_escapes_the_wildcards_of_what_was_typed() {
        let sql =
            Dialect::Sqlite.lookup(&looking(Some("name"), LookupPart::Matching("50%_".into())));
        assert_eq!(texts(&sql), ["50%_", r"%50\%\_%", r"%50\%\_%", r"50\%\_%"]);
    }

    #[test]
    fn a_lookup_of_a_table_with_no_label_goes_by_the_key_alone() {
        let sql = Dialect::Sqlite.lookup(&looking(None, LookupPart::Matching("12".into())));
        assert_eq!(
            sql.text,
            "SELECT \"id\", CASE WHEN CAST(\"id\" AS TEXT) = ? THEN 0 ELSE 2 END \
             FROM (SELECT \"id\" FROM \"main\".\"publishers\" \
             WHERE CAST(\"id\" AS TEXT) LIKE ? ESCAPE '\\' LIMIT 51 OFFSET 0) AS found \
             ORDER BY 2, 1"
        );
        assert_eq!(texts(&sql), ["12", "12%"]);
    }

    #[test]
    fn a_lookup_for_nothing_reads_the_first_rows_by_key() {
        let sql =
            Dialect::MySql.lookup(&looking(Some("name"), LookupPart::Matching(String::new())));
        assert_eq!(
            sql.text,
            "SELECT `id`, `name`, 2 FROM `main`.`publishers` ORDER BY 1 LIMIT 51"
        );
        assert!(sql.params.is_empty());
    }

    #[test]
    fn a_pinned_lookup_is_one_equality_on_the_key() {
        let pinned = |value: Value| looking(Some("name"), LookupPart::Pinned(value));
        let number = Value::Int(9_100_000_000_000_000_004);
        // A whole number goes as one: MySQL would compare a string with a
        // BIGINT as floats.
        let my = Dialect::MySql.lookup(&pinned(number.clone()));
        assert_eq!(
            my.text,
            "SELECT `id`, `name`, 0 FROM `main`.`publishers` WHERE `id` = ? LIMIT 51"
        );
        assert_eq!(my.params, std::slice::from_ref(&number));
        // PostgreSQL has no parameters here: the value is a literal.
        let pg = Dialect::Postgres.lookup(&pinned(number));
        assert_eq!(
            pg.text,
            "SELECT \"id\", \"name\", 0 FROM \"main\".\"publishers\" \
             WHERE \"id\" = 9100000000000000004 LIMIT 51"
        );
        let text = Dialect::Postgres.lookup(&pinned(Value::Text("it's".into())));
        assert!(
            text.text.contains(r#"WHERE "id" = 'it''s' LIMIT 51"#),
            "{}",
            text.text
        );
    }

    #[test]
    fn a_lookup_escapes_what_was_typed_into_postgres_literals() {
        // The text is: i t ' s backslash.
        let sql = Dialect::Postgres.lookup(&looking(
            Some("name"),
            LookupPart::Matching("it's\\".into()),
        ));
        assert!(sql.params.is_empty());
        // The key is compared as typed: the quote and the backslash doubled.
        assert!(
            sql.text.contains(r#"CAST("id" AS TEXT) = E'it''s\\' THEN"#),
            "{}",
            sql.text
        );
        // LIKE escapes the backslash first (two), then the literal doubles
        // each of those (four).
        assert!(
            sql.text
                .contains(r#"CAST("name" AS TEXT) ILIKE E'%it''s\\\\%' THEN"#),
            "{}",
            sql.text
        );
        assert!(
            sql.text
                .contains(r#"CAST("id" AS TEXT) ILIKE E'it''s\\\\%' LIMIT"#),
            "{}",
            sql.text
        );
    }

    #[test]
    fn a_lookup_is_one_statement_with_no_parameters_on_postgres() {
        let sql =
            Dialect::Postgres.lookup(&looking(Some("name"), LookupPart::Matching("harb".into())));
        assert_eq!(
            sql.text,
            "SELECT \"id\", \"name\", \
             CASE WHEN CAST(\"id\" AS TEXT) = E'harb' THEN 0 \
             WHEN CAST(\"name\" AS TEXT) ILIKE E'%harb%' THEN 1 ELSE 2 END \
             FROM (SELECT \"id\", \"name\" FROM \"main\".\"publishers\" \
             WHERE CAST(\"name\" AS TEXT) ILIKE E'%harb%' \
             OR CAST(\"id\" AS TEXT) ILIKE E'harb%' LIMIT 51 OFFSET 0) AS found \
             ORDER BY 3, 1"
        );
        assert!(sql.params.is_empty());
    }

    #[test]
    fn the_codes_of_a_match_are_the_ones_the_statement_writes() {
        assert_eq!(
            [
                Matched::Key.code(),
                Matched::Label.code(),
                Matched::KeyStart.code()
            ],
            [0, 1, 2]
        );
    }
}

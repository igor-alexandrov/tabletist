//! A read's answer as the JSON an agent gets, kept under a size: a row
//! limit does not bound an answer, and what is sent goes into the agent's
//! context.

use serde_json::{Map, Value as Json, json};
use tabletist_db::Value;
use tabletist_db::agent::Answer;

/// The most characters of one text value. A longer one is cut, and says
/// how long it was.
pub const MAX_CELL_CHARS: usize = 2_000;

/// The most bytes of an answer as JSON. Past it, rows are left out from
/// the end.
pub const MAX_ANSWER_BYTES: usize = 256_000;

/// Why an answer does not hold every row of the statement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cut {
    /// The statement had more rows than the connection lets an answer hold.
    RowLimit,
    /// The rows did not fit [`MAX_ANSWER_BYTES`].
    Size,
}

impl Cut {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::RowLimit => "row_limit",
            Self::Size => "size",
        }
    }
}

/// An answer, written.
#[derive(Debug, Clone, PartialEq)]
pub struct Written {
    /// `columns`, `rows`, `row_count`, `truncated`, `truncated_by` when it
    /// is, and `took_ms`.
    pub json: Map<String, Json>,
    /// The rows that were sent.
    pub rows: usize,
    pub columns: usize,
    pub cut: Option<Cut>,
}

/// An answer whose columns alone, before its first row, are more than an
/// answer may weigh.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TooWide;

/// Writes `answer` within `max_bytes`, which is [`MAX_ANSWER_BYTES`]
/// everywhere but in a test.
pub fn write(answer: &Answer, max_bytes: usize) -> Result<Written, TooWide> {
    let columns: Vec<Json> = answer
        .columns
        .iter()
        .map(|column| {
            let mut entry = Map::new();
            entry.insert("name".into(), json!(column.name));
            if !column.type_name.is_empty() {
                entry.insert("type".into(), json!(column.type_name));
            }
            Json::Object(entry)
        })
        .collect();
    let took_ms = u64::try_from(answer.elapsed.as_millis()).unwrap_or(u64::MAX);
    // What the answer weighs before its first row, with the longest words
    // it may come to hold.
    let frame = json!({
        "columns": columns,
        "rows": [],
        "row_count": answer.rows.len(),
        "truncated": true,
        "truncated_by": Cut::RowLimit.as_str(),
        "took_ms": took_ms,
    });
    let mut bytes = frame.to_string().len();
    if bytes > max_bytes {
        return Err(TooWide);
    }
    let mut rows = Vec::with_capacity(answer.rows.len());
    let mut cut = answer.truncated.then_some(Cut::RowLimit);
    for row in &answer.rows {
        let row = Json::Array(row.iter().map(cell).collect());
        // The row, and the comma between it and the one before.
        bytes += row.to_string().len() + usize::from(!rows.is_empty());
        if bytes > max_bytes {
            cut = Some(Cut::Size);
            break;
        }
        rows.push(row);
    }
    let sent = rows.len();
    let mut json = Map::new();
    json.insert("columns".into(), Json::Array(columns));
    json.insert("rows".into(), Json::Array(rows));
    json.insert("row_count".into(), json!(sent));
    json.insert("truncated".into(), json!(cut.is_some()));
    if let Some(cut) = cut {
        json.insert("truncated_by".into(), json!(cut.as_str()));
    }
    json.insert("took_ms".into(), json!(took_ms));
    Ok(Written {
        json,
        rows: sent,
        columns: answer.columns.len(),
        cut,
    })
}

/// One value. A database's value is null, true or false, a number or a
/// text, so an object is never one: it stands for a value that was not
/// sent whole.
fn cell(value: &Value) -> Json {
    match value {
        Value::Null => Json::Null,
        Value::Bool(value) => json!(value),
        Value::Int(value) => json!(value),
        Value::Float(value) => match serde_json::Number::from_f64(*value) {
            Some(number) => Json::Number(number),
            // JSON has no word for these.
            None if value.is_nan() => json!("NaN"),
            None if *value > 0.0 => json!("Infinity"),
            None => json!("-Infinity"),
        },
        Value::Text(value) => text(value),
        Value::Bytes(bytes) => json!({ "binary_bytes": bytes.len() }),
    }
}

/// A text, cut where it is longer than [`MAX_CELL_CHARS`].
pub(crate) fn text(text: &str) -> Json {
    match text.char_indices().nth(MAX_CELL_CHARS) {
        Some((end, _)) => json!({
            "cut": &text[..end],
            "characters": text.chars().count(),
        }),
        None => json!(text),
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use tabletist_db::{ColumnMeta, ValueKind};

    use super::*;

    fn column(name: &str, type_name: &str) -> ColumnMeta {
        ColumnMeta {
            name: name.into(),
            type_name: type_name.into(),
            kind: ValueKind::Other,
        }
    }

    fn answer(rows: Vec<Vec<Value>>, truncated: bool) -> Answer {
        Answer {
            columns: vec![column("id", "INTEGER"), column("note", "")],
            rows,
            truncated,
            elapsed: Duration::from_millis(4),
        }
    }

    fn wrote(answer: &Answer, max_bytes: usize) -> Written {
        write(answer, max_bytes).unwrap()
    }

    fn text(text: &str) -> Value {
        Value::Text(text.into())
    }

    #[test]
    fn an_answer_is_its_columns_its_rows_and_how_long_it_took() {
        let written = wrote(
            &answer(
                vec![
                    vec![Value::Int(1), text("print")],
                    vec![Value::Int(9_100_000_000_000_000_004), Value::Null],
                ],
                false,
            ),
            MAX_ANSWER_BYTES,
        );
        assert_eq!(
            Json::Object(written.json.clone()),
            json!({
                "columns": [{ "name": "id", "type": "INTEGER" }, { "name": "note" }],
                "rows": [[1, "print"], [9_100_000_000_000_000_004_i64, null]],
                "row_count": 2,
                "truncated": false,
                "took_ms": 4,
            })
        );
        assert_eq!((written.rows, written.columns, written.cut), (2, 2, None));
    }

    #[test]
    fn an_answer_the_row_limit_cut_says_so() {
        let written = wrote(
            &answer(vec![vec![Value::Int(1), Value::Null]], true),
            MAX_ANSWER_BYTES,
        );
        assert_eq!(written.json["truncated"], json!(true));
        assert_eq!(written.json["truncated_by"], json!("row_limit"));
        assert_eq!(written.cut, Some(Cut::RowLimit));
    }

    #[test]
    fn a_long_text_is_cut_and_says_how_long_it_was() {
        // Counted in characters, not bytes: each of these is two bytes.
        let long = "é".repeat(MAX_CELL_CHARS + 1);
        let exact = "é".repeat(MAX_CELL_CHARS);
        let written = wrote(
            &answer(
                vec![
                    vec![Value::Int(1), text(&long)],
                    vec![Value::Int(2), text(&exact)],
                ],
                false,
            ),
            MAX_ANSWER_BYTES,
        );
        let rows = written.json["rows"].as_array().unwrap();
        assert_eq!(
            rows[0][1],
            json!({ "cut": exact, "characters": MAX_CELL_CHARS + 1 })
        );
        assert_eq!(rows[1][1], json!(exact));
    }

    #[test]
    fn a_binary_value_is_described_and_not_sent() {
        let bytes = Value::Bytes(vec![0xde, 0xad, 0xbe, 0xef].into());
        let written = wrote(
            &answer(vec![vec![Value::Int(1), bytes]], false),
            MAX_ANSWER_BYTES,
        );
        assert_eq!(written.json["rows"][0][1], json!({ "binary_bytes": 4 }));
    }

    #[test]
    fn a_number_json_cannot_write_is_its_name() {
        let row = vec![
            Value::Float(f64::NAN),
            Value::Float(f64::INFINITY),
            Value::Float(f64::NEG_INFINITY),
            Value::Float(1.5),
            Value::Bool(true),
        ];
        let mut answer = answer(vec![row], false);
        answer.columns = (0..5).map(|n| column(&format!("c{n}"), "")).collect();
        let written = wrote(&answer, MAX_ANSWER_BYTES);
        assert_eq!(
            written.json["rows"][0],
            json!(["NaN", "Infinity", "-Infinity", 1.5, true])
        );
    }

    #[test]
    fn an_answer_over_the_size_loses_rows_from_the_end_and_says_so() {
        let rows: Vec<Vec<Value>> = (0..100)
            .map(|n| vec![Value::Int(n), text(&"x".repeat(90))])
            .collect();
        let max = 2_000;
        let written = wrote(&answer(rows.clone(), false), max);
        assert_eq!(written.cut, Some(Cut::Size));
        assert_eq!(written.json["truncated_by"], json!("size"));
        assert!(written.rows > 0 && written.rows < 100, "{}", written.rows);
        assert_eq!(written.json["row_count"], json!(written.rows));
        // The rows that stayed are the first ones.
        assert_eq!(written.json["rows"][0][0], json!(0));
        // What is sent is under the size, and one more row would not be.
        let size = Json::Object(written.json.clone()).to_string().len();
        assert!(size <= max, "{size}");
        assert!(size + 100 > max, "{size}");
        // The size cut is the reason given, whatever the row limit did.
        assert_eq!(wrote(&answer(rows, true), max).cut, Some(Cut::Size));
    }

    #[test]
    fn columns_that_are_too_much_by_themselves_are_no_answer() {
        let mut wide = answer(Vec::new(), false);
        wide.columns = (0..2_000).map(|_| column(&"a".repeat(2_000), "")).collect();
        assert_eq!(write(&wide, MAX_ANSWER_BYTES), Err(TooWide));
        // Many columns of an ordinary width are an answer.
        wide.columns = (0..1_600)
            .map(|n| column(&format!("column_{n}"), "TEXT"))
            .collect();
        assert!(write(&wide, MAX_ANSWER_BYTES).is_ok());
    }

    #[test]
    fn an_answer_that_fits_exactly_is_sent_whole() {
        let rows = vec![
            vec![Value::Int(1), text("a")],
            vec![Value::Int(2), text("b")],
        ];
        let whole = wrote(&answer(rows.clone(), true), MAX_ANSWER_BYTES);
        let size = Json::Object(whole.json).to_string().len();
        assert_eq!(wrote(&answer(rows.clone(), true), size).rows, 2);
        assert_eq!(wrote(&answer(rows, true), size - 1).rows, 1);
    }
}

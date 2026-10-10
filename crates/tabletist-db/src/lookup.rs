//! Looking a row up in the table a foreign key points at: by a text that
//! is in its label or begins its key, or by its key alone.

use std::fmt;

use crate::{Error, ObjectRef, Result, Value};

/// One read of the table a foreign key points at. A search of the app is
/// two of these: the pinned row, then the matches.
#[derive(Clone, PartialEq)]
pub struct Lookup {
    pub object: ObjectRef,
    /// The column the foreign key references.
    pub key: String,
    /// The column a person reads a row by. `None`: the table has no text
    /// column but its key, and rows are found by the key alone. Never the
    /// key column: the statement names both in one select list.
    pub label: Option<String>,
    pub part: LookupPart,
    /// The most rows wanted. One more is read, to learn whether there are
    /// more.
    pub limit: u32,
}

/// Which rows a [`Lookup`] reads.
#[derive(Clone, PartialEq)]
pub enum LookupPart {
    /// The row whose key is this, compared with `=`. The caller sends a
    /// whole number for a key that is one: MySQL compares a number with a
    /// string as floats, and two large keys are then the same.
    Pinned(Value),
    /// The rows whose label contains the text, compared as a contains
    /// filter compares, or whose key as text begins with it. An empty
    /// text: the table's first rows by key.
    Matching(String),
}

/// How a row of a [`LookupPage`] was found.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Matched {
    /// Its key is the value, or the text.
    Key,
    /// Its label contains the text.
    Label,
    /// Its key begins with the text and its label does not hold it. Every
    /// row of an empty text's read.
    KeyStart,
}

impl Matched {
    /// What the statement's last column answers for this.
    pub(crate) fn code(self) -> i64 {
        match self {
            Self::Key => 0,
            Self::Label => 1,
            Self::KeyStart => 2,
        }
    }

    /// `None` for a cell the statement does not answer: a row that says
    /// nothing true about how it matched is an error, not a key start.
    fn from_cell(cell: &Value) -> Option<Self> {
        match cell {
            Value::Int(0) => Some(Self::Key),
            Value::Int(1) => Some(Self::Label),
            Value::Int(2) => Some(Self::KeyStart),
            _ => None,
        }
    }
}

/// A row the lookup found.
#[derive(Clone, PartialEq)]
pub struct LookupRow {
    pub key: Value,
    /// `None` when the lookup named no label column.
    pub label: Option<Value>,
    pub matched: Matched,
}

/// The rows of one read.
#[derive(Clone, PartialEq)]
pub struct LookupPage {
    pub rows: Vec<LookupRow>,
    /// There are more rows than `limit`.
    pub more: bool,
}

/// The rows a driver read for `lookup`, at most one more than its limit,
/// as its page. Each row is the key, the label where the lookup names one,
/// and how the row matched.
pub(crate) fn page(mut rows: Vec<Vec<Value>>, lookup: &Lookup) -> Result<LookupPage> {
    let limit = lookup.limit as usize;
    let more = rows.len() > limit;
    rows.truncate(limit);
    let shape = || Error::query("the lookup returned a row of another shape");
    let rows = rows
        .into_iter()
        .map(|row| {
            let (key, label, cell) = if lookup.label.is_some() {
                let [key, label, cell] = <[Value; 3]>::try_from(row).map_err(|_| shape())?;
                (key, Some(label), cell)
            } else {
                let [key, cell] = <[Value; 2]>::try_from(row).map_err(|_| shape())?;
                (key, None, cell)
            };
            let matched = Matched::from_cell(&cell).ok_or_else(shape)?;
            Ok(LookupRow {
                key,
                label,
                matched,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(LookupPage { rows, more })
}

// What was typed, and what a table holds, stay out of a log line: a label
// can be a name or an email.
impl fmt::Debug for Lookup {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let part = match &self.part {
            LookupPart::Pinned(_) => "Pinned",
            LookupPart::Matching(_) => "Matching",
        };
        f.debug_struct("Lookup")
            .field("object", &self.object)
            .field("key", &self.key)
            .field("label", &self.label)
            .field("part", &part)
            .field("limit", &self.limit)
            .finish()
    }
}

impl fmt::Debug for LookupPage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LookupPage")
            .field("rows", &self.rows.len())
            .field("more", &self.more)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lookup(label: Option<&str>, limit: u32) -> Lookup {
        Lookup {
            object: ObjectRef::new("main", "publishers"),
            key: "id".into(),
            label: label.map(str::to_owned),
            part: LookupPart::Matching("Ada Lovelace".into()),
            limit,
        }
    }

    #[test]
    fn a_lookup_prints_without_what_was_typed_or_read() {
        let printed = format!("{:?}", lookup(Some("name"), 50));
        assert!(!printed.contains("Ada"), "{printed}");
        assert!(printed.contains("Matching"), "{printed}");
        let page = LookupPage {
            rows: vec![LookupRow {
                key: Value::Int(1),
                label: Some(Value::Text("Ada Lovelace".into())),
                matched: Matched::Label,
            }],
            more: false,
        };
        assert!(!format!("{page:?}").contains("Ada"));
        let pinned = Lookup {
            part: LookupPart::Pinned(Value::Text("Ada".into())),
            ..lookup(Some("name"), 50)
        };
        let printed = format!("{pinned:?}");
        assert!(!printed.contains("Ada"), "{printed}");
        assert!(printed.contains("Pinned"), "{printed}");
    }

    #[test]
    fn a_page_keeps_the_limit_and_says_there_are_more() {
        let rows = vec![
            vec![
                Value::Int(4),
                Value::Text("Harbor Press".into()),
                Value::Int(1),
            ],
            vec![
                Value::Int(7),
                Value::Text("Harbor Lights".into()),
                Value::Int(1),
            ],
            vec![Value::Int(40), Value::Null, Value::Int(2)],
        ];
        let page = page(rows, &lookup(Some("name"), 2)).unwrap();
        assert!(page.more);
        assert_eq!(page.rows.len(), 2);
        assert_eq!(page.rows[0].key, Value::Int(4));
        assert_eq!(page.rows[0].label, Some(Value::Text("Harbor Press".into())));
        assert_eq!(page.rows[0].matched, Matched::Label);
    }

    #[test]
    fn a_page_without_a_label_has_two_cells_to_a_row() {
        let rows = vec![vec![Value::Int(4), Value::Int(0)]];
        let page = page(rows, &lookup(None, 50)).unwrap();
        assert!(!page.more);
        assert_eq!(page.rows[0].label, None);
        assert_eq!(page.rows[0].matched, Matched::Key);
    }

    #[test]
    fn a_match_cell_the_statement_does_not_answer_is_an_error() {
        for cell in [Value::Int(7), Value::Text("1".into())] {
            let rows = vec![vec![Value::Int(4), Value::Text("Ada".into()), cell]];
            assert!(page(rows, &lookup(Some("name"), 50)).is_err());
        }
    }

    #[test]
    fn a_row_of_another_shape_is_an_error() {
        let rows = vec![vec![Value::Int(4)]];
        assert!(page(rows, &lookup(Some("name"), 50)).is_err());
    }
}

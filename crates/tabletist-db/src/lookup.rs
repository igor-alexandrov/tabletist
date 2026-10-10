//! Looking a row up in the table a foreign key points at: by a text that
//! is in its label or begins its key, or by its key alone.

use std::fmt;

use crate::{ObjectRef, Value};

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
}

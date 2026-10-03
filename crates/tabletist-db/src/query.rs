//! Browsing a table's rows: filters, sorting, paging.

use std::time::Duration;

use crate::{ColumnMeta, ObjectRef, Value};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterOp {
    Eq,
    Ne,
    Lt,
    Gt,
    Le,
    Ge,
    /// Case-insensitive substring match on the value as text.
    Contains,
    StartsWith,
    IsNull,
    IsNotNull,
    /// Comma-separated values.
    In,
}

impl FilterOp {
    pub const ALL: [FilterOp; 11] = [
        Self::Eq,
        Self::Ne,
        Self::Lt,
        Self::Gt,
        Self::Le,
        Self::Ge,
        Self::Contains,
        Self::StartsWith,
        Self::IsNull,
        Self::IsNotNull,
        Self::In,
    ];

    pub fn takes_value(self) -> bool {
        !matches!(self, Self::IsNull | Self::IsNotNull)
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Eq => "=",
            Self::Ne => "≠",
            Self::Lt => "<",
            Self::Gt => ">",
            Self::Le => "≤",
            Self::Ge => "≥",
            Self::Contains => "contains",
            Self::StartsWith => "starts with",
            Self::IsNull => "is NULL",
            Self::IsNotNull => "is not NULL",
            Self::In => "in",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Filter {
    pub column: String,
    pub op: FilterOp,
    /// As typed. The database converts it to the column's type. In a binary
    /// column, Eq, Ne and In also read a UUID or `0x` hex as the bytes it
    /// stands for, the way the app shows them.
    pub value: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortDir {
    Asc,
    Desc,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sort {
    pub column: String,
    pub dir: SortDir,
}

/// One page of one object's rows.
#[derive(Debug, Clone, PartialEq)]
pub struct RowQuery {
    pub object: ObjectRef,
    /// Combined with AND.
    pub filters: Vec<Filter>,
    /// Appended as `AND (<raw>)`, on lines of its own so a trailing `--`
    /// comment ends there. It cannot write, in a writable session either:
    /// PostgreSQL and MySQL run it inside a read-only transaction; SQLite
    /// runs it under `query_only`, refuses a `;` and a NUL in it, and has
    /// its authorizer's filter fence behind those. On SQLite, text ending
    /// inside a `/*` comment is refused too.
    pub raw_where: Option<String>,
    pub sort: Vec<Sort>,
    pub offset: u64,
    pub limit: u32,
}

impl RowQuery {
    pub fn new(object: ObjectRef, limit: u32) -> Self {
        Self {
            object,
            filters: Vec::new(),
            raw_where: None,
            sort: Vec::new(),
            offset: 0,
            limit,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RowPage {
    pub columns: Vec<ColumnMeta>,
    /// At most `limit` rows.
    pub rows: Vec<Vec<Value>>,
    /// Whether a next page exists.
    pub has_more: bool,
    /// Whether rows are ordered by a key, so paging is stable.
    pub ordered_by_key: bool,
    pub elapsed: Duration,
}

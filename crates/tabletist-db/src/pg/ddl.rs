//! A table's or a view's definition, built from PostgreSQL's catalog, which
//! has no `SHOW CREATE TABLE`: in the order pg_dump writes it, the
//! `CREATE` first, then the indexes that back no constraint, the triggers,
//! and the comments. The owner, the grants, the row-level policies and the
//! sequences the table owns are left out.
//!
//! It is read with the search path empty, so the server writes every name
//! with its schema, and the text runs whatever the search path it is run
//! with.

use super::{column, query_error};
use crate::ddl::{Part, PartKind};
use crate::{Ddl, Driver, Error, ObjectRef, Result};

/// The relation itself, by schema and name.
const RELATION: &str = "\
SELECT c.oid, c.relkind::text, c.relpersistence::text, c.relispartition, \
       quote_ident(n.nspname) || '.' || quote_ident(c.relname), \
       array_to_string(c.reloptions, ', '), \
       CASE WHEN c.relkind = 'p' THEN pg_get_partkeydef(c.oid) END, \
       CASE WHEN c.relispartition THEN pg_get_expr(c.relpartbound, c.oid) END, \
       ARRAY(SELECT i.inhparent::regclass::text FROM pg_inherits i \
             WHERE i.inhrelid = c.oid ORDER BY i.inhseqno), \
       quote_literal(obj_description(c.oid, 'pg_class')), \
       CASE WHEN c.relkind IN ('v', 'm') THEN pg_get_viewdef(c.oid, true) END, \
       current_setting('server_version') \
FROM pg_class c JOIN pg_namespace n ON n.oid = c.relnamespace \
WHERE n.nspname = $1 AND c.relname = $2";

/// Its columns. A collation is the column's own where it is not its
/// type's. Whether the column is an identity or generated is read through
/// `to_jsonb`, as `describe` reads it: `attgenerated` is PostgreSQL 12's.
/// For a column a parent gives the table, whether the parent's is NOT
/// NULL and its default (the first parent's that has the column): what
/// the table set itself is what differs.
const COLUMNS: &str = "\
SELECT quote_ident(a.attname), format_type(a.atttypid, a.atttypmod), \
       CASE WHEN a.attcollation <> 0 AND a.attcollation <> t.typcollation \
            THEN quote_ident(cn.nspname) || '.' || quote_ident(co.collname) END, \
       pg_get_expr(d.adbin, d.adrelid), \
       COALESCE(to_jsonb(a) ->> 'attidentity', ''), \
       COALESCE(to_jsonb(a) ->> 'attgenerated', ''), \
       a.attnotnull, a.attislocal, \
       quote_literal(col_description(a.attrelid, a.attnum)), \
       EXISTS (SELECT 1 FROM pg_inherits i \
               JOIN pg_attribute pa ON pa.attrelid = i.inhparent AND pa.attname = a.attname \
               WHERE i.inhrelid = a.attrelid AND pa.attnotnull), \
       (SELECT pg_get_expr(pd.adbin, pd.adrelid) FROM pg_inherits i \
        JOIN pg_attribute pa ON pa.attrelid = i.inhparent AND pa.attname = a.attname \
        JOIN pg_attrdef pd ON pd.adrelid = pa.attrelid AND pd.adnum = pa.attnum \
        WHERE i.inhrelid = a.attrelid ORDER BY i.inhseqno LIMIT 1) \
FROM pg_attribute a \
JOIN pg_type t ON t.oid = a.atttypid \
LEFT JOIN pg_attrdef d ON d.adrelid = a.attrelid AND d.adnum = a.attnum \
LEFT JOIN pg_collation co ON co.oid = a.attcollation \
LEFT JOIN pg_namespace cn ON cn.oid = co.collnamespace \
WHERE a.attrelid = $1 AND a.attnum > 0 AND NOT a.attisdropped \
ORDER BY a.attnum";

/// The table's own constraints, not those a partition or a child has from
/// its parent: primary key, unique, check, foreign key, exclusion, each
/// kind by name, and whether each holds for the rows already there. A NOT
/// NULL (a constraint of its own from PostgreSQL 18) is the column's.
const CONSTRAINTS: &str = "\
SELECT quote_ident(conname), pg_get_constraintdef(oid), convalidated \
FROM pg_constraint \
WHERE conrelid = $1 AND contype IN ('p', 'u', 'c', 'f', 'x') AND conislocal \
ORDER BY array_position(ARRAY['p', 'u', 'c', 'f', 'x'], contype::text), conname";

/// The indexes that back no constraint, by name. A partition's index that
/// is its parent's is the parent's to make. Whether the index is a
/// partitioned table's comes with it.
const INDEXES: &str = "\
SELECT pg_get_indexdef(i.indexrelid), ic.relkind = 'I' \
FROM pg_index i JOIN pg_class ic ON ic.oid = i.indexrelid \
WHERE i.indrelid = $1 \
  AND NOT EXISTS (SELECT 1 FROM pg_constraint k \
                  WHERE k.conrelid = i.indrelid AND k.conindid = i.indexrelid \
                    AND k.contype IN ('p', 'u', 'x')) \
  AND NOT EXISTS (SELECT 1 FROM pg_inherits h WHERE h.inhrelid = i.indexrelid) \
ORDER BY ic.relname";

/// The triggers a person made, by name: not the internal ones of a foreign
/// key, nor a partition's that is its parent's (`tgparentid`, read
/// through `to_jsonb`: PostgreSQL 13's).
const TRIGGERS: &str = "\
SELECT pg_get_triggerdef(t.oid), quote_ident(t.tgname), t.tgenabled::text \
FROM pg_trigger t \
WHERE t.tgrelid = $1 AND NOT t.tgisinternal \
  AND COALESCE((to_jsonb(t) ->> 'tgparentid')::bigint, 0) = 0 \
ORDER BY t.tgname";

/// A column as the `CREATE TABLE` and the comments need it.
struct Column {
    name: String,
    type_name: String,
    collation: Option<String>,
    expression: Option<String>,
    identity: String,
    generated: String,
    not_null: bool,
    local: bool,
    comment: Option<String>,
    /// For a column a parent gives the table: whether the parent's is NOT
    /// NULL, and its default.
    parent_not_null: bool,
    parent_default: Option<String>,
}

/// What a relation is, for its `CREATE` and its `COMMENT ON`.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Table,
    View,
    MaterializedView,
}

impl Kind {
    fn noun(self) -> &'static str {
        match self {
            Self::Table => "TABLE",
            Self::View => "VIEW",
            Self::MaterializedView => "MATERIALIZED VIEW",
        }
    }
}

impl super::Conn {
    /// See [`crate::Connection::ddl`]: in a read-only transaction, with
    /// the search path empty for the transaction alone.
    pub(super) async fn read_ddl(&self, object: &ObjectRef) -> Result<Ddl> {
        let mut client = self.client.lock().await;
        let transaction = client
            .build_transaction()
            .read_only(true)
            .start()
            .await
            .map_err(query_error)?;
        transaction
            .batch_execute("SET LOCAL search_path = ''")
            .await
            .map_err(query_error)?;
        let built = build(&transaction, object).await;
        // The read's own error wins over one from the rollback.
        let rolled_back = transaction.rollback().await;
        let ddl = built?;
        rolled_back.map_err(query_error)?;
        Ok(ddl)
    }
}

async fn build(transaction: &tokio_postgres::Transaction<'_>, object: &ObjectRef) -> Result<Ddl> {
    let rows = transaction
        .query(RELATION, &[&object.schema, &object.name])
        .await
        .map_err(query_error)?;
    let Some(relation) = rows.first() else {
        return Err(Error::query(format!(
            "no such table or view: {}",
            object.name
        )));
    };
    let oid: u32 = column(relation, 0)?;
    let relkind: String = column(relation, 1)?;
    let kind = match relkind.as_str() {
        "r" | "p" => Kind::Table,
        "v" => Kind::View,
        "m" => Kind::MaterializedView,
        "f" => return Err(Error::Unsupported("the DDL of a foreign table")),
        _ => {
            return Err(Error::query(format!(
                "not a table or a view: {}",
                object.name
            )));
        }
    };
    let unlogged = column::<String>(relation, 2)? == "u";
    let partition: bool = column(relation, 3)?;
    let name: String = column(relation, 4)?;
    let options: Option<String> = column(relation, 5)?;
    let options = options.filter(|options| !options.is_empty());
    let partition_key: Option<String> = column(relation, 6)?;
    let bound: Option<String> = column(relation, 7)?;
    let parents: Vec<String> = column(relation, 8)?;
    let comment: Option<String> = column(relation, 9)?;
    let view: Option<String> = column(relation, 10)?;
    let version: String = column(relation, 11)?;
    // "17.2 (Debian 17.2-1.pgdg120+1)" reads as "17.2".
    let version = version.split_whitespace().next().unwrap_or_default();
    let columns = transaction
        .query(COLUMNS, &[&oid])
        .await
        .map_err(query_error)?
        .iter()
        .map(|row| {
            Ok(Column {
                name: column(row, 0)?,
                type_name: column(row, 1)?,
                collation: column(row, 2)?,
                expression: column(row, 3)?,
                identity: column(row, 4)?,
                generated: column(row, 5)?,
                not_null: column(row, 6)?,
                local: column(row, 7)?,
                comment: column(row, 8)?,
                parent_not_null: column(row, 9)?,
                parent_default: column(row, 10)?,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let with = options
        .as_deref()
        .map(|options| format!(" WITH ({options})"))
        .unwrap_or_default();
    let own = match kind {
        Kind::View => {
            let view = view.unwrap_or_default();
            format!("CREATE OR REPLACE VIEW {name}{with} AS\n{view}")
        }
        Kind::MaterializedView => {
            let view = view.unwrap_or_default();
            let view = view.trim_end().trim_end_matches(';');
            format!("CREATE MATERIALIZED VIEW {name}{with} AS\n{view}\nWITH NO DATA;")
        }
        Kind::Table => {
            let mut constraints = Vec::new();
            let mut not_valid = Vec::new();
            for row in transaction
                .query(CONSTRAINTS, &[&oid])
                .await
                .map_err(query_error)?
            {
                let (constraint, definition): (String, String) =
                    (column(&row, 0)?, column(&row, 1)?);
                let valid: bool = column(&row, 2)?;
                // One that does not hold for the rows already there is
                // added after the table: in `CREATE TABLE` it would be
                // checked, and made valid.
                match valid {
                    true => constraints.push(format!("    CONSTRAINT {constraint} {definition}")),
                    false => not_valid.push(format!(
                        "ALTER TABLE ONLY {name} ADD CONSTRAINT {constraint} {definition};"
                    )),
                }
            }
            let table = Table {
                name: &name,
                unlogged,
                columns: &columns,
                constraints,
                // A partition's parent is its one parent, and its bound
                // says it is one.
                parent: bound.as_ref().filter(|_| partition).and(parents.first()),
                bound: bound.as_deref().filter(|_| partition),
                parents: if partition { &[] } else { &parents },
                partition_key: partition_key.as_deref(),
                options: options.as_deref(),
                not_valid,
            };
            table.create()
        }
    };
    let indexes = transaction
        .query(INDEXES, &[&oid])
        .await
        .map_err(query_error)?
        .iter()
        .map(|row| {
            let definition: String = column(row, 0)?;
            let partitioned: bool = column(row, 1)?;
            // A partitioned table's index is written `ON ONLY`, which
            // makes it on the table alone, for pg_dump to attach each
            // partition's to it. Without the partitions' (left out
            // above), the index is made on the table and its partitions.
            // The table's name follows: an index's name can hold the words.
            Ok(match partitioned {
                true => definition.replacen(
                    &format!(" ON ONLY {name} USING "),
                    &format!(" ON {name} USING "),
                    1,
                ),
                false => definition,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let triggers = transaction
        .query(TRIGGERS, &[&oid])
        .await
        .map_err(query_error)?
        .iter()
        .map(|row| {
            let definition: String = column(row, 0)?;
            let trigger: String = column(row, 1)?;
            let enabled: String = column(row, 2)?;
            // As a new trigger is: fires unless the session replicates.
            let set = match enabled.as_str() {
                "D" => Some("DISABLE"),
                "R" => Some("ENABLE REPLICA"),
                "A" => Some("ENABLE ALWAYS"),
                _ => None,
            };
            Ok(match set {
                Some(set) => format!("{definition};\nALTER TABLE {name} {set} TRIGGER {trigger};"),
                None => definition,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let noun = kind.noun();
    let comments = comment
        .map(|comment| format!("COMMENT ON {noun} {name} IS {comment};"))
        .into_iter()
        .chain(columns.iter().filter_map(|column| {
            let comment = column.comment.as_ref()?;
            Some(format!(
                "COMMENT ON COLUMN {name}.{} IS {comment};",
                column.name
            ))
        }))
        .collect();
    let parts = vec![
        Part {
            kind: PartKind::Object,
            statements: vec![own],
        },
        Part {
            kind: PartKind::Indexes,
            statements: indexes,
        },
        Part {
            kind: PartKind::Triggers,
            statements: triggers,
        },
        Part {
            kind: PartKind::Comments,
            statements: comments,
        },
    ];
    let server = format!("PostgreSQL {version}");
    let heading = format!("{name} · built from the catalog · {server}");
    Ok(Ddl::new(Driver::Postgres, server, Some(heading), parts))
}

/// What a table's `CREATE TABLE` says.
struct Table<'a> {
    name: &'a str,
    unlogged: bool,
    columns: &'a [Column],
    /// `CONSTRAINT name ...`, each a line of the list.
    constraints: Vec<String>,
    /// The table it is a partition of, with the bound it holds.
    parent: Option<&'a String>,
    bound: Option<&'a str>,
    /// The tables it inherits from, where it is no partition.
    parents: &'a [String],
    partition_key: Option<&'a str>,
    options: Option<&'a str>,
    /// `ALTER TABLE … ADD CONSTRAINT … NOT VALID;`, each.
    not_valid: Vec<String>,
}

impl Table<'_> {
    fn create(&self) -> String {
        let unlogged = if self.unlogged { "UNLOGGED " } else { "" };
        let mut text = format!("CREATE {unlogged}TABLE {}", self.name);
        if let Some(parent) = self.parent {
            text.push_str(&format!(" PARTITION OF {parent}"));
        }
        // The columns a parent gives the table are the parent's to make.
        let mut lines: Vec<String> = self
            .columns
            .iter()
            .filter(|column| column.local)
            .map(column_line)
            .collect();
        lines.extend(self.constraints.iter().cloned());
        match (self.parent.is_some(), lines.is_empty()) {
            // A partition with nothing of its own lists nothing.
            (true, true) => {}
            (false, true) => text.push_str(" (\n)"),
            (_, false) => text.push_str(&format!(" (\n{}\n)", lines.join(",\n"))),
        }
        if let Some(bound) = self.bound {
            text.push_str(&format!("\n{bound}"));
        }
        if !self.parents.is_empty() {
            text.push_str(&format!("\nINHERITS ({})", self.parents.join(", ")));
        }
        if let Some(key) = self.partition_key {
            text.push_str(&format!("\nPARTITION BY {key}"));
        }
        if let Some(options) = self.options {
            text.push_str(&format!("\nWITH ({options})"));
        }
        text.push(';');
        // What the table set itself on a column its parent gives it.
        for column in self.columns.iter().filter(|column| !column.local) {
            let generates = !column.identity.is_empty() || !column.generated.is_empty();
            if generates {
                continue;
            }
            let alter = format!(
                "\nALTER TABLE ONLY {} ALTER COLUMN {}",
                self.name, column.name
            );
            match (&column.expression, &column.parent_default) {
                (Some(own), parents) if Some(own) != parents.as_ref() => {
                    text.push_str(&format!("{alter} SET DEFAULT {own};"));
                }
                (None, Some(_)) => text.push_str(&format!("{alter} DROP DEFAULT;")),
                _ => {}
            }
            if column.not_null && !column.parent_not_null {
                text.push_str(&format!("{alter} SET NOT NULL;"));
            }
        }
        for constraint in &self.not_valid {
            text.push('\n');
            text.push_str(constraint);
        }
        text
    }
}

/// A column's line of the `CREATE TABLE`: its name and type, its own
/// collation, then its default, or what generates it, then `NOT NULL`.
fn column_line(column: &Column) -> String {
    let mut line = format!("    {} {}", column.name, column.type_name);
    if let Some(collation) = &column.collation {
        line.push_str(&format!(" COLLATE {collation}"));
    }
    let expression = column.expression.as_deref().unwrap_or_default();
    match (column.identity.as_str(), column.generated.as_str()) {
        ("a", _) => line.push_str(" GENERATED ALWAYS AS IDENTITY"),
        ("d", _) => line.push_str(" GENERATED BY DEFAULT AS IDENTITY"),
        (_, "s") => line.push_str(&format!(" GENERATED ALWAYS AS ({expression}) STORED")),
        (_, "v") => line.push_str(&format!(" GENERATED ALWAYS AS ({expression}) VIRTUAL")),
        _ if column.expression.is_some() => line.push_str(&format!(" DEFAULT {expression}")),
        _ => {}
    }
    if column.not_null {
        line.push_str(" NOT NULL");
    }
    line
}

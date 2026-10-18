//! `describe_schema`: the tables of a connection, and what a table is made
//! of.

use std::collections::HashMap;

use serde_json::{Map, Value as Json, json};
use tabletist_db::agent::Hidden;
use tabletist_db::{Error, ObjectInfo, ObjectKind, ObjectRef, Structure};

use super::{Arguments, Done, Invalid, Note, Tool, find, kept, schema, settle, within};
use crate::answer::{self, MAX_ANSWER_BYTES};
use crate::gateway::{Failure, Gateway, Schemas, Session};
use crate::refusal::{Refused, code};
use crate::stop::Stop;

pub const NAME: &str = "describe_schema";

/// The most tables one call describes.
pub const MAX_TABLES: usize = 20;

/// The most bytes of a name an agent asks for: more than any of the three
/// databases lets a table be called by, bar SQLite, which has no limit.
pub const MAX_NAME_BYTES: usize = 1_000;

pub fn tool() -> Tool {
    Tool {
        name: NAME,
        title: "Describe the schema",
        description: "Without `tables`: the names of the tables and views of a connection, \
                      with a rough count of each one's rows, and the names of its other \
                      schemas. With `tables` (at most 20 names): each one's columns with their \
                      types, its primary key, its foreign keys and the keys that point at it, \
                      and its indexes. Write a table of another schema as schema.table. A \
                      table the person hides from you is left out of the list, and named under \
                      `hidden` when you ask for it. A column marked `masked` may be selected, \
                      and answers •••, but not filtered, joined, sorted or computed on, and \
                      neither may any column of its name, in any table; its default, its \
                      comment and the values it may hold are left out. A view marked \
                      `reads_masked` may not be read: it reads such a column, or on MySQL \
                      reads its table.",
        input: schema(
            json!({
                "connection": {
                    "type": "string",
                    "description": "The connection's name, from list_connections.",
                },
                "tables": {
                    "type": "array",
                    "description": "The tables to describe. Leave it out to list every table.",
                    "items": { "type": "string" },
                },
            }),
            &["connection"],
        ),
    }
}

pub async fn call<G: Gateway>(
    gateway: &G,
    stop: &Stop,
    arguments: &Map<String, Json>,
) -> Result<Done, Invalid> {
    let arguments = Arguments::new(NAME, arguments, &["connection", "tables"])?;
    let connection = arguments.text("connection")?;
    let tables = match arguments.list("tables")? {
        None => None,
        Some(given) => Some(
            given
                .iter()
                .map(|name| name.as_str().map(str::to_owned))
                .collect::<Option<Vec<String>>>()
                .ok_or_else(|| {
                    Invalid::Arguments(format!("{NAME}: `tables` is a list of strings."))
                })?,
        ),
    };
    let mut note = Note {
        request: tables.as_ref().map(|tables| kept(&tables.join(", "))),
        ..Note::default()
    };
    let answer = run(gateway, stop, connection, tables.as_deref(), &mut note).await;
    Ok(Done { answer, note })
}

async fn run<G: Gateway>(
    gateway: &G,
    stop: &Stop,
    connection: &str,
    tables: Option<&[String]>,
    note: &mut Note,
) -> Result<Map<String, Json>, Refused> {
    let listed = find(gateway, connection)?;
    note.connection = Some((listed.id.clone(), listed.name.clone()));
    let asked = tables.unwrap_or_default();
    if asked.len() > MAX_TABLES {
        return Err(Refused::new(
            code::TOO_LARGE,
            format!("{NAME} describes at most {MAX_TABLES} tables in one call."),
        ));
    }
    if asked.iter().any(|name| name.len() > MAX_NAME_BYTES) {
        return Err(Refused::new(
            code::TOO_LARGE,
            format!("A table's name is at most {MAX_NAME_BYTES} bytes."),
        ));
    }
    let session = settle(gateway.open(&listed.id, stop.token()).await, stop, &listed)?;
    let work = async {
        let hidden = session.hidden(&listed.tables, &listed.masked).await?;
        hidden
            .check_settings()
            .map_err(|refusal| Failure::Database(Error::Agent(refusal)))?;
        match tables {
            None => list(&session, &hidden).await,
            Some(tables) => describe(&session, stop, tables, &hidden).await,
        }
    };
    let described = settle(within(stop, listed.limits, work).await, stop, &listed)?;
    note.rows = Some(described.tables);
    note.columns = described.columns;
    Ok(described.json)
}

/// An answer, with what the log says of it.
struct Described {
    json: Map<String, Json>,
    tables: u64,
    columns: Option<u64>,
}

/// The tables and views of the connection's own schema, by name, and the
/// names of its other schemas. What is hidden is left out.
async fn list<S: Session>(session: &S, hidden: &Hidden) -> Result<Described, Failure> {
    let Schemas { names, home } = session.schemas().await?;
    let objects = match &home {
        Some(home) => session
            .objects(home)
            .await?
            .into_iter()
            .filter(|object| !hidden.hides(home, &object.name))
            .collect(),
        None => Vec::new(),
    };
    let mut room = Room::new();
    let mut truncated = false;
    let mut json = Map::new();
    if let Some(home) = &home
        && room.take(&json!(home))
    {
        json.insert("schema".into(), json!(home));
    }
    // The other schemas first, in a quarter of the room at most: a server
    // can have thousands, and the tables are what was asked for.
    let mut for_schemas = Room {
        left: room.left / 4,
    };
    let mut others = Vec::new();
    for name in names.iter().filter(|name| Some(*name) != home.as_ref()) {
        let name = json!(name);
        if !for_schemas.take(&name) {
            truncated = true;
            break;
        }
        others.push(name);
    }
    room.left -= room.left / 4 - for_schemas.left;
    let mut tables = Vec::new();
    for object in &objects {
        let mut entry = Map::new();
        entry.insert("name".into(), json!(object.name));
        entry.insert("kind".into(), json!(kind(object.kind)));
        if let Some(rows) = object.estimated_rows {
            entry.insert("rows_estimate".into(), json!(rows));
        }
        if let Some(home) = &home
            && let Some(column) = hidden.reads_masked(home, &object.name)
        {
            entry.insert("reads_masked".into(), json!(column));
        }
        let entry = Json::Object(entry);
        if !room.take(&entry) {
            truncated = true;
            break;
        }
        tables.push(entry);
    }
    let listed = tables.len() as u64;
    json.insert("tables".into(), Json::Array(tables));
    if !others.is_empty() {
        json.insert("other_schemas".into(), Json::Array(others));
    }
    if truncated {
        json.insert("truncated".into(), json!(true));
    }
    Ok(Described {
        json,
        tables: listed,
        columns: None,
    })
}

/// What each of `asked` is made of. A name that is no table of the
/// connection is listed as not found, one that is hidden as hidden, and the
/// others are described all the same. Tables that do not fit the answer
/// are named as left out.
async fn describe<S: Session>(
    session: &S,
    stop: &Stop,
    asked: &[String],
    hidden: &Hidden,
) -> Result<Described, Failure> {
    let Schemas { names, home } = session.schemas().await?;
    let mut room = Room::new();
    // Each name comes back at most once more: as not found, or as left out.
    for name in asked {
        room.take(&json!(name));
    }
    // Each schema's tables, read once.
    let mut listings: HashMap<String, Vec<ObjectInfo>> = HashMap::new();
    let mut tables = Vec::new();
    let (mut not_found, mut left_out, mut hidden_ones) = (Vec::new(), Vec::new(), Vec::new());
    let (mut columns, mut full) = (0, false);
    for name in asked {
        if full {
            left_out.push(name);
            continue;
        }
        // Twenty tables are a hundred catalog queries. A call that was
        // ended stops between two tables, whatever the gateway does.
        if stop.token().is_cancelled() {
            return Err(Error::Cancelled.into());
        }
        let mut found = None;
        for (schema, table) in places(name, home.as_deref(), &names) {
            if !listings.contains_key(schema) {
                listings.insert(schema.to_owned(), session.objects(schema).await?);
            }
            if let Some(object) = listings[schema].iter().find(|object| object.name == table) {
                found = Some((ObjectRef::new(schema, table), object.kind));
                break;
            }
        }
        let Some((object, object_kind)) = found else {
            not_found.push(name);
            continue;
        };
        if hidden.hides(&object.schema, &object.name) {
            hidden_ones.push(name);
            continue;
        }
        let structure = session.describe(&object).await?;
        let foreign = home.as_deref() != Some(object.schema.as_str());
        let entry = table(&object, object_kind, foreign, &structure, hidden);
        if room.take(&entry) {
            tables.push(entry);
            columns += structure.columns.len() as u64;
        } else {
            full = true;
            left_out.push(name);
        }
    }
    let mut json = Map::new();
    let described = tables.len() as u64;
    json.insert("tables".into(), Json::Array(tables));
    if !not_found.is_empty() {
        json.insert("not_found".into(), json!(not_found));
    }
    if !hidden_ones.is_empty() {
        json.insert("hidden".into(), json!(hidden_ones));
    }
    if !left_out.is_empty() {
        json.insert("truncated".into(), json!(true));
        json.insert("left_out".into(), json!(left_out));
    }
    Ok(Described {
        json,
        tables: described,
        columns: Some(columns),
    })
}

/// Where a name an agent wrote may be: in the connection's own schema as
/// it stands, and, when it has a dot, in the schema before the dot.
fn places<'a>(
    name: &'a str,
    home: Option<&'a str>,
    schemas: &'a [String],
) -> Vec<(&'a str, &'a str)> {
    let mut places = Vec::new();
    if let Some(home) = home {
        places.push((home, name));
    }
    if let Some((schema, table)) = name.split_once('.')
        && schemas.iter().any(|known| known == schema)
    {
        places.push((schema, table));
    }
    places
}

/// What is left of the size an answer may have.
struct Room {
    left: usize,
}

impl Room {
    /// The size, less the answer's own words: its keys and its brackets.
    fn new() -> Self {
        Self {
            left: MAX_ANSWER_BYTES - 200,
        }
    }

    /// Takes the room `value` needs, with the comma after it, and says
    /// whether there was that much.
    fn take(&mut self, value: &Json) -> bool {
        let needs = value.to_string().len() + 1;
        if needs > self.left {
            return false;
        }
        self.left -= needs;
        true
    }
}

/// A column's type as it is told: a masked one's without the values
/// MySQL writes into it (`enum('new','paid')` is `enum`).
fn type_of(type_name: &str, masked: bool) -> &str {
    let listed = ["enum(", "set("].iter().any(|kind| {
        type_name.len() >= kind.len() && type_name[..kind.len()].eq_ignore_ascii_case(kind)
    });
    match (masked, listed) {
        (true, true) => type_name.split('(').next().unwrap_or(type_name),
        _ => type_name,
    }
}

fn kind(kind: ObjectKind) -> &'static str {
    match kind {
        ObjectKind::Table => "table",
        ObjectKind::View => "view",
        ObjectKind::MaterializedView => "materialized view",
    }
}

/// A table's entry. A key to or from a hidden table is left out: it would
/// name that table's columns.
fn table(
    object: &ObjectRef,
    object_kind: ObjectKind,
    foreign: bool,
    structure: &Structure,
    hidden: &Hidden,
) -> Json {
    let mut entry = Map::new();
    entry.insert("name".into(), json!(object.name));
    if foreign {
        entry.insert("schema".into(), json!(object.schema));
    }
    entry.insert("kind".into(), json!(kind(object_kind)));
    if let Some(column) = hidden.reads_masked(&object.schema, &object.name) {
        entry.insert("reads_masked".into(), json!(column));
    }
    let columns: Vec<Json> = structure
        .columns
        .iter()
        .map(|column| {
            let masked = hidden
                .masks(Some(&object.schema), &object.name, &column.name)
                .is_some();
            let mut entry = Map::new();
            entry.insert("name".into(), json!(column.name));
            entry.insert("type".into(), json!(type_of(&column.type_name, masked)));
            entry.insert("nullable".into(), json!(column.nullable));
            if masked {
                entry.insert("masked".into(), json!(true));
            }
            if column.generated {
                entry.insert("generated".into(), json!(true));
            }
            // What says what a masked column's values are is left out: its
            // default, which is one, or what it is computed from; the
            // values it may hold; and its comment, whose words are its
            // author's and may give one as an example. A default and a
            // comment are as long as their author liked.
            if !masked {
                if let Some(default) = &column.default {
                    entry.insert("default".into(), answer::text(default));
                }
                if let Some(values) = &column.allowed_values {
                    entry.insert("values".into(), json!(values));
                }
                if let Some(comment) = &column.comment {
                    entry.insert("comment".into(), answer::text(comment));
                }
            }
            Json::Object(entry)
        })
        .collect();
    entry.insert("columns".into(), Json::Array(columns));
    if !structure.primary_key.is_empty() {
        entry.insert("primary_key".into(), json!(structure.primary_key));
    }
    let named = |schema: &str, table: &str| match schema == object.schema {
        true => table.to_owned(),
        false => format!("{schema}.{table}"),
    };
    let foreign_keys: Vec<_> = structure
        .foreign_keys
        .iter()
        .filter(|key| !hidden.hides(&key.ref_schema, &key.ref_table))
        .collect();
    if !foreign_keys.is_empty() {
        let keys: Vec<Json> = foreign_keys
            .iter()
            .map(|key| {
                json!({
                    "columns": key.columns,
                    "references": named(&key.ref_schema, &key.ref_table),
                    "referenced_columns": key.ref_columns,
                    "on_delete": key.on_delete,
                })
            })
            .collect();
        entry.insert("foreign_keys".into(), Json::Array(keys));
    }
    let referenced_by: Vec<_> = structure
        .referenced_by
        .iter()
        .filter(|key| !hidden.hides(&key.schema, &key.table))
        .collect();
    if !referenced_by.is_empty() {
        let keys: Vec<Json> = referenced_by
            .iter()
            .map(|key| {
                json!({
                    "table": named(&key.schema, &key.table),
                    "columns": key.columns,
                    "on_delete": key.on_delete,
                })
            })
            .collect();
        entry.insert("referenced_by".into(), Json::Array(keys));
    }
    let indexes: Vec<Json> = structure
        .indexes
        .iter()
        .filter(|index| !index.primary)
        .map(|index| {
            json!({
                "name": index.name,
                "columns": index.columns,
                "unique": index.unique,
            })
        })
        .collect();
    if !indexes.is_empty() {
        entry.insert("indexes".into(), Json::Array(indexes));
    }
    Json::Object(entry)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_masked_columns_type_is_told_without_the_values_mysql_writes_into_it() {
        assert_eq!(type_of("enum('new','paid','shipped')", true), "enum");
        assert_eq!(type_of("SET('a','b')", true), "SET");
        assert_eq!(type_of("enum('new','paid')", false), "enum('new','paid')");
        assert_eq!(type_of("varchar(255)", true), "varchar(255)");
        assert_eq!(type_of("text", true), "text");
    }

    #[test]
    fn a_name_is_looked_for_at_home_first_and_then_by_its_schema() {
        let schemas = ["public".to_owned(), "sales".to_owned()];
        assert_eq!(
            places("book_covers", Some("public"), &schemas),
            [("public", "book_covers")]
        );
        assert_eq!(
            places("sales.customers", Some("public"), &schemas),
            [("public", "sales.customers"), ("sales", "customers")]
        );
        // A dot in a name of the connection's own schema.
        assert_eq!(
            places("v1.events", Some("public"), &schemas),
            [("public", "v1.events")]
        );
        assert_eq!(places("book_covers", None, &schemas), []);
    }

    #[test]
    fn room_is_taken_until_there_is_no_more() {
        let mut room = Room::new();
        let entry = json!("x".repeat(MAX_ANSWER_BYTES / 2 - 200));
        assert!(room.take(&entry));
        assert!(room.take(&entry));
        let left = room.left;
        assert!(!room.take(&entry));
        assert_eq!(room.left, left, "what did not fit took nothing");
        assert!(room.take(&json!("small")));
    }
}

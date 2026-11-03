//! The data the tests stand on: the fixture connection and its schema, the
//! Bookshop's structures and pages, what a statement or a script did, a
//! workspace without an app, the zones and a palette. Data only: the
//! harness that drives the app is `crate::testing`'s, which re-exports
//! these, so the lower layer's tests use them without the interface.

use crate::backend::{RequestId, SessionId};
use crate::connections::{ConnectionId, SavedConnection};
use crate::model::Workspace;
use tabletist_db::{
    ColumnMeta, ConnectSpec, Driver, ObjectInfo, ObjectKind, RowPage, Value, ValueKind,
};

/// What the fake session lists in schema `main`: `users`, `orders` and
/// the view `active_users`.
pub fn fixture_objects() -> Vec<ObjectInfo> {
    vec![
        ObjectInfo {
            name: "active_users".into(),
            kind: ObjectKind::View,
            estimated_rows: None,
        },
        ObjectInfo {
            name: "orders".into(),
            kind: ObjectKind::Table,
            estimated_rows: Some(3),
        },
        ObjectInfo {
            name: "users".into(),
            kind: ObjectKind::Table,
            estimated_rows: Some(1_200_000),
        },
    ]
}

/// The Bookshop's objects as a catalog lists them, with the planner's
/// estimate of each table's rows: what a start screen's first visit sorts
/// by. In schema `main`, as the fake session's are.
pub fn bookshop_objects() -> Vec<ObjectInfo> {
    let table = |name: &str, rows: u64| ObjectInfo {
        name: name.into(),
        kind: ObjectKind::Table,
        estimated_rows: Some(rows),
    };
    let view = |name: &str| ObjectInfo {
        name: name.into(),
        kind: ObjectKind::View,
        estimated_rows: None,
    };
    vec![
        table("authors", 3_700),
        table("book_covers", 13),
        view("cover_counts"),
        table("covers", 13),
        table("editions", 9_400),
        view("missing_covers"),
        table("publishers", 6),
        table("ratings", 31_900),
        table("reviews", 48_200),
    ]
}

/// The saved connection the fake session stands for.
pub fn fixture_connection() -> SavedConnection {
    SavedConnection {
        id: ConnectionId::new(),
        name: "Fixture".into(),
        environment: crate::env::Environment::Dev,
        // Read-only, as every connection was when most tests were written.
        // A test of a writable connection says so itself.
        read_only: Some(true),
        password: crate::connections::PasswordMode::None,
        ssh_secret: crate::connections::PasswordMode::None,
        spec: ConnectSpec::sqlite("/tmp/fixture.db"),
        agents: Default::default(),
    }
}

/// A workspace for the fixture connection, still connecting and with no
/// tabs, for tests that need one without an app.
pub fn workspace() -> Workspace {
    Workspace::new(
        SessionId(1),
        RequestId(2),
        fixture_connection(),
        tabletist_db::Secrets::default(),
    )
}

/// A page shaped like the fixture's users table.
pub fn page(rows: usize, has_more: bool) -> RowPage {
    RowPage {
        columns: vec![
            ColumnMeta {
                name: "id".into(),
                type_name: "INTEGER".into(),
                kind: ValueKind::Numeric,
            },
            ColumnMeta {
                name: "email".into(),
                type_name: "TEXT".into(),
                kind: ValueKind::Text,
            },
            ColumnMeta {
                name: "meta".into(),
                type_name: "JSON".into(),
                kind: ValueKind::Json,
            },
        ],
        rows: (0..rows)
            .map(|index| {
                vec![
                    Value::Int(index as i64 + 1),
                    Value::Text(format!("user{}@example.com", index + 1).into()),
                    if index == 0 {
                        Value::Text(r#"{"plan":"pro"}"#.into())
                    } else {
                        Value::Null
                    },
                ]
            })
            .collect(),
        has_more,
        ordered_by_key: true,
        elapsed: std::time::Duration::from_millis(12),
    }
}

/// The structure of the table `page` holds: `id` is the primary key,
/// `email` is NOT NULL, `meta` is JSON and may be NULL.
pub fn fixture_structure() -> tabletist_db::Structure {
    let column = |name: &str, type_name: &str, nullable: bool| tabletist_db::ColumnInfo {
        name: name.into(),
        type_name: type_name.into(),
        nullable,
        ..tabletist_db::ColumnInfo::default()
    };
    tabletist_db::Structure {
        columns: vec![
            column("id", "INTEGER", false),
            column("email", "TEXT", false),
            column("meta", "JSON", true),
        ],
        primary_key: vec!["id".into()],
        ..tabletist_db::Structure::default()
    }
}

/// The structure of the Bookshop's `book_covers`: the database numbers
/// `id`, `publisher_id` must be given, `kind` is one of three and has a
/// default, `image_data` may be NULL, and `created_at` has an expression
/// for a default.
pub fn book_covers_structure() -> tabletist_db::Structure {
    use tabletist_db::ColumnInfo;
    let column = |name: &str, type_name: &str| ColumnInfo {
        name: name.into(),
        type_name: type_name.into(),
        ..ColumnInfo::default()
    };
    tabletist_db::Structure {
        columns: vec![
            ColumnInfo {
                identity: true,
                ..column("id", "INTEGER")
            },
            column("publisher_id", "INTEGER"),
            ColumnInfo {
                default: Some("'print'".into()),
                allowed_values: Some(vec!["print".into(), "ebook".into(), "audio".into()]),
                ..column("kind", "TEXT")
            },
            ColumnInfo {
                nullable: true,
                ..column("image_data", "JSON")
            },
            ColumnInfo {
                default: Some("CURRENT_TIMESTAMP".into()),
                ..column("created_at", "TEXT")
            },
        ],
        primary_key: vec!["id".into()],
        ..tabletist_db::Structure::default()
    }
}

/// A page of `book_covers`, `rows` long.
pub fn book_covers_page(rows: usize) -> RowPage {
    let meta = |name: &str, type_name: &str, kind: ValueKind| ColumnMeta {
        name: name.into(),
        type_name: type_name.into(),
        kind,
    };
    RowPage {
        columns: vec![
            meta("id", "INTEGER", ValueKind::Numeric),
            meta("publisher_id", "INTEGER", ValueKind::Numeric),
            meta("kind", "TEXT", ValueKind::Text),
            meta("image_data", "JSON", ValueKind::Json),
            meta("created_at", "TEXT", ValueKind::Text),
        ],
        rows: (0..rows)
            .map(|index| {
                vec![
                    Value::Int(index as i64 + 1),
                    Value::Int(9_100_000_000_000_000_001 + (index as i64 % 3)),
                    Value::Text(if index % 2 == 0 { "print" } else { "ebook" }.into()),
                    Value::Null,
                    Value::Text("2026-01-12 09:14:03".into()),
                ]
            })
            .collect(),
        has_more: false,
        ordered_by_key: true,
        elapsed: std::time::Duration::from_millis(2),
    }
}

/// The zone database, read once for every test.
pub fn zones_shared() -> std::sync::Arc<crate::zones::Zones> {
    static ZONES: std::sync::OnceLock<std::sync::Arc<crate::zones::Zones>> =
        std::sync::OnceLock::new();
    std::sync::Arc::clone(ZONES.get_or_init(|| std::sync::Arc::new(crate::zones::Zones::load())))
}

/// The zone database, as the backend reads it.
pub fn zones() -> &'static crate::zones::Zones {
    static ZONES: std::sync::OnceLock<std::sync::Arc<crate::zones::Zones>> =
        std::sync::OnceLock::new();
    ZONES.get_or_init(zones_shared)
}

/// UTC, for a test's [`crate::edit::Table`] to hold a zone by reference.
pub static UTC: jiff::tz::TimeZone = jiff::tz::TimeZone::UTC;

/// When the Bookshop's covers were made, as PostgreSQL sends a
/// `timestamptz` in a session whose zone is UTC.
pub const COVERS_MADE: &str = "2026-01-12 09:14:03.482915+00";

/// The structure of `book_covers` on PostgreSQL, where `created_at` is a
/// `timestamptz`.
pub fn stamped_covers_structure() -> tabletist_db::Structure {
    let mut structure = book_covers_structure();
    structure.columns[4].type_name = "timestamp with time zone".into();
    structure
}

/// A page of `book_covers` with `created_at` as a `timestamptz`, made at
/// [`COVERS_MADE`].
pub fn stamped_covers_page(rows: usize) -> RowPage {
    let mut page = book_covers_page(rows);
    page.columns[4].type_name = "timestamptz".into();
    page.columns[4].kind = ValueKind::Temporal;
    for row in &mut page.rows {
        row[4] = Value::Text(COVERS_MADE.into());
    }
    page
}

/// The Bookshop's publishers, as (id, name).
pub const PUBLISHERS: [(i64, &str); 3] = [
    (9_100_000_000_000_000_001, "Northlight Books"),
    (9_100_000_000_000_000_004, "Harbor Press"),
    (9_100_000_000_000_000_007, "Harbor Lights"),
];

/// The structure of the Bookshop's `publishers`.
pub fn publishers_structure() -> tabletist_db::Structure {
    use tabletist_db::ColumnInfo;
    let column = |name: &str, type_name: &str| ColumnInfo {
        name: name.into(),
        type_name: type_name.into(),
        ..ColumnInfo::default()
    };
    // The keys that point at the table: `book_covers` loses its covers
    // with a publisher, `books` is cleared of it, and `book_reviews`
    // holds it.
    let key = |table: &str, on_delete: &str| tabletist_db::ReferencingKey {
        name: Some(format!("{table}_publisher_id_fkey")),
        schema: "main".into(),
        table: table.into(),
        columns: vec!["publisher_id".into()],
        ref_columns: vec!["id".into()],
        on_delete: on_delete.into(),
    };
    tabletist_db::Structure {
        columns: vec![column("id", "INTEGER"), column("name", "TEXT")],
        primary_key: vec!["id".into()],
        referenced_by: vec![
            key("book_covers", "CASCADE"),
            key("books", "SET NULL"),
            key("book_reviews", "NO ACTION"),
        ],
        ..tabletist_db::Structure::default()
    }
}

/// A page of `publishers`: the three of [`PUBLISHERS`].
pub fn publishers_page() -> RowPage {
    let meta = |name: &str, type_name: &str, kind: ValueKind| ColumnMeta {
        name: name.into(),
        type_name: type_name.into(),
        kind,
    };
    RowPage {
        columns: vec![
            meta("id", "INTEGER", ValueKind::Numeric),
            meta("name", "TEXT", ValueKind::Text),
        ],
        rows: PUBLISHERS
            .iter()
            .map(|(id, name)| vec![Value::Int(*id), Value::Text((*name).into())])
            .collect(),
        has_more: false,
        ordered_by_key: true,
        elapsed: std::time::Duration::from_millis(2),
    }
}

/// `book_covers` with its foreign key: `publisher_id` points at
/// `publishers.id`. The tests that type into `publisher_id` as a number
/// keep [`book_covers_structure`], which has no key.
pub fn keyed_covers_structure() -> tabletist_db::Structure {
    tabletist_db::Structure {
        foreign_keys: vec![tabletist_db::ForeignKeyInfo {
            columns: vec!["publisher_id".into()],
            ref_schema: "main".into(),
            ref_table: "publishers".into(),
            ref_columns: vec!["id".into()],
            ..tabletist_db::ForeignKeyInfo::default()
        }],
        ..book_covers_structure()
    }
}

/// `genres`: a table that points at itself. `parent_id` is a foreign key
/// to `genres.id`, and `id` is typed by hand: no database numbers it.
pub fn genres_structure() -> tabletist_db::Structure {
    use tabletist_db::ColumnInfo;
    let column = |name: &str, type_name: &str, nullable: bool| ColumnInfo {
        name: name.into(),
        type_name: type_name.into(),
        nullable,
        ..ColumnInfo::default()
    };
    tabletist_db::Structure {
        columns: vec![
            column("id", "INTEGER", false),
            column("parent_id", "INTEGER", true),
            column("name", "TEXT", false),
        ],
        primary_key: vec!["id".into()],
        foreign_keys: vec![tabletist_db::ForeignKeyInfo {
            columns: vec!["parent_id".into()],
            ref_schema: "main".into(),
            ref_table: "genres".into(),
            ref_columns: vec!["id".into()],
            ..tabletist_db::ForeignKeyInfo::default()
        }],
        // The same key, seen from the row it points at: it holds a genre
        // that has genres under it.
        referenced_by: vec![tabletist_db::ReferencingKey {
            name: None,
            schema: "main".into(),
            table: "genres".into(),
            columns: vec!["parent_id".into()],
            ref_columns: vec!["id".into()],
            on_delete: "NO ACTION".into(),
        }],
        ..tabletist_db::Structure::default()
    }
}

/// The two genres the tests hold: Fiction (1), and Crime (2) under it.
pub fn genres_page() -> RowPage {
    let meta = |name: &str, type_name: &str, kind: ValueKind| ColumnMeta {
        name: name.into(),
        type_name: type_name.into(),
        kind,
    };
    RowPage {
        columns: vec![
            meta("id", "INTEGER", ValueKind::Numeric),
            meta("parent_id", "INTEGER", ValueKind::Numeric),
            meta("name", "TEXT", ValueKind::Text),
        ],
        rows: vec![
            vec![Value::Int(1), Value::Null, Value::Text("Fiction".into())],
            vec![Value::Int(2), Value::Int(1), Value::Text("Crime".into())],
        ],
        has_more: false,
        ordered_by_key: true,
        elapsed: std::time::Duration::from_millis(2),
    }
}

/// The genres, as (id, name), for [`Harness::answer_lookups`].
pub const GENRES: [(i64, &str); 2] = [(1, "Fiction"), (2, "Crime")];

/// The Bookshop's `isbns`: a table of its key alone, with nothing to
/// label a row by.
pub fn isbns_structure() -> tabletist_db::Structure {
    tabletist_db::Structure {
        columns: vec![tabletist_db::ColumnInfo {
            name: "isbn".into(),
            type_name: "TEXT".into(),
            ..tabletist_db::ColumnInfo::default()
        }],
        primary_key: vec!["isbn".into()],
        ..tabletist_db::Structure::default()
    }
}

/// The Bookshop's `editions`, cut to what a key to `isbns` and the chooser
/// of a label need: `isbn` is a foreign key to `isbns.isbn`, and `cover`
/// holds bytes.
pub fn editions_structure() -> tabletist_db::Structure {
    use tabletist_db::ColumnInfo;
    let column = |name: &str, type_name: &str| ColumnInfo {
        name: name.into(),
        type_name: type_name.into(),
        ..ColumnInfo::default()
    };
    tabletist_db::Structure {
        columns: vec![
            column("id", "INTEGER"),
            column("isbn", "TEXT"),
            column("title", "TEXT"),
            column("cover", "BLOB"),
        ],
        primary_key: vec!["id".into()],
        foreign_keys: vec![tabletist_db::ForeignKeyInfo {
            columns: vec!["isbn".into()],
            ref_schema: "main".into(),
            ref_table: "isbns".into(),
            ref_columns: vec!["isbn".into()],
            ..tabletist_db::ForeignKeyInfo::default()
        }],
        ..tabletist_db::Structure::default()
    }
}

/// Two editions, each with its ISBN.
pub fn editions_page() -> RowPage {
    let meta = |name: &str, type_name: &str, kind: ValueKind| ColumnMeta {
        name: name.into(),
        type_name: type_name.into(),
        kind,
    };
    let edition = |id: i64, isbn: &str, title: &str| {
        vec![
            Value::Int(id),
            Value::Text(isbn.into()),
            Value::Text(title.into()),
            Value::Null,
        ]
    };
    RowPage {
        columns: vec![
            meta("id", "INTEGER", ValueKind::Numeric),
            meta("isbn", "TEXT", ValueKind::Text),
            meta("title", "TEXT", ValueKind::Text),
            meta("cover", "BLOB", ValueKind::Binary),
        ],
        rows: vec![
            edition(1, "978-1-4028-9462-6", "The Lighthouse"),
            edition(2, "978-1-4028-9463-3", "Low Tide"),
        ],
        has_more: false,
        ordered_by_key: true,
        elapsed: std::time::Duration::from_millis(2),
    }
}

/// A page of `book_covers` whose three rows point at the three
/// publishers in turn: Northlight Books, Harbor Press, Harbor Lights.
/// [`book_covers_page`] numbers its publishers on from the first, and
/// two of those are no publisher.
pub fn keyed_covers_page() -> RowPage {
    let mut page = book_covers_page(3);
    for (row, (key, _)) in page.rows.iter_mut().zip(PUBLISHERS) {
        row[1] = Value::Int(key);
    }
    page
}

/// What a database holding `rows` as a table of (key, label) answers
/// `lookup`: by the rules of `Connection::lookup`, folding case as Rust's
/// `to_lowercase` does. It differs from the real statement in one thing:
/// it sorts every match before it applies the limit, where the database
/// limits first.
pub fn fake_lookup(
    lookup: &tabletist_db::Lookup,
    rows: &[(i64, &str)],
) -> tabletist_db::LookupPage {
    use tabletist_db::{LookupPart, LookupRow, Matched};
    let rank = |matched: Matched| match matched {
        Matched::Key => 0,
        Matched::Label => 1,
        Matched::KeyStart => 2,
    };
    let mut found: Vec<(Matched, i64, &str)> = rows
        .iter()
        .filter_map(|&(key, label)| {
            let written = key.to_string();
            let matched = match &lookup.part {
                LookupPart::Pinned(Value::Int(pinned)) => (*pinned == key).then_some(Matched::Key),
                LookupPart::Pinned(Value::Text(pinned)) => {
                    (**pinned == *written).then_some(Matched::Key)
                }
                LookupPart::Pinned(_) => None,
                LookupPart::Matching(text) if text.is_empty() => Some(Matched::KeyStart),
                LookupPart::Matching(text) if written == *text => Some(Matched::Key),
                LookupPart::Matching(text)
                    if lookup.label.is_some()
                        && label.to_lowercase().contains(&text.to_lowercase()) =>
                {
                    Some(Matched::Label)
                }
                LookupPart::Matching(text) => written
                    .starts_with(text.as_str())
                    .then_some(Matched::KeyStart),
            };
            Some((matched?, key, label))
        })
        .collect();
    found.sort_by_key(|&(matched, key, _)| (rank(matched), key));
    let limit = lookup.limit as usize;
    let more = found.len() > limit;
    found.truncate(limit);
    tabletist_db::LookupPage {
        rows: found
            .into_iter()
            .map(|(matched, key, label)| LookupRow {
                key: Value::Int(key),
                label: lookup.label.as_ref().map(|_| Value::Text(label.into())),
                matched,
            })
            .collect(),
        more,
    }
}

/// A SQL editor statement's result, shaped like the fixture's users table.
pub fn rows_outcome(rows: usize) -> tabletist_db::StatementOutcome {
    let page = page(rows, false);
    tabletist_db::StatementOutcome::Rows {
        columns: page.columns,
        rows: page.rows,
        truncated: false,
    }
}

/// A SQL editor statement's result that the row limit cut at `rows`.
pub fn cut_outcome(rows: usize) -> tabletist_db::StatementOutcome {
    let page = page(rows, false);
    tabletist_db::StatementOutcome::Rows {
        columns: page.columns,
        rows: page.rows,
        truncated: true,
    }
}

/// A SQL editor statement that failed with `message`, at the 1-based
/// character `position` of its text when the database gives one.
pub fn error_outcome(message: &str, position: Option<usize>) -> tabletist_db::StatementOutcome {
    tabletist_db::StatementOutcome::Error {
        error: tabletist_db::Error::query(message),
        position,
    }
}

/// A write refused in a read-only transaction, as PostgreSQL and MySQL say
/// it (SQLSTATE 25006).
pub fn refused_write() -> tabletist_db::StatementOutcome {
    tabletist_db::StatementOutcome::Error {
        error: tabletist_db::Error::Query {
            code: Some("25006".into()),
            message: "cannot execute UPDATE in a read-only transaction".into(),
            detail: None,
            hint: None,
            named: Box::default(),
        },
        position: None,
    }
}

/// What a script did, as a driver reports it: one outcome per statement
/// that started, each taking 14 ms. A `Cancelled` outcome is a stopped run,
/// as it is for every driver.
pub fn script_outcome(
    outcomes: Vec<tabletist_db::StatementOutcome>,
) -> tabletist_db::ScriptOutcome {
    let stopped = outcomes.contains(&tabletist_db::StatementOutcome::Cancelled);
    tabletist_db::ScriptOutcome {
        results: outcomes
            .into_iter()
            .map(|outcome| tabletist_db::StatementResult {
                elapsed: std::time::Duration::from_millis(14),
                outcome,
            })
            .collect(),
        stopped,
        ..Default::default()
    }
}

/// What a script sent to write did: [`script_outcome`], with how its
/// transaction ended.
pub fn write_outcome(
    outcomes: Vec<tabletist_db::StatementOutcome>,
    end: tabletist_db::ScriptEnd,
) -> tabletist_db::ScriptOutcome {
    tabletist_db::ScriptOutcome {
        end,
        ..script_outcome(outcomes)
    }
}

/// A statement that ran and gave no result set: how many rows it changed,
/// where it counts them.
pub fn done_outcome(affected: Option<u64>) -> tabletist_db::StatementOutcome {
    tabletist_db::StatementOutcome::Done {
        affected,
        warnings: 0,
    }
}

/// A script stopped before it began: cancelled while it was queued, or
/// while its transaction opened. No statement has a result.
pub fn stopped_before_it_began() -> tabletist_db::ScriptOutcome {
    tabletist_db::ScriptOutcome {
        results: Vec::new(),
        stopped: true,
        ..Default::default()
    }
}

/// Types `text` into `sql`, runs all of it (split as SQLite does) and
/// finishes the run with `outcomes`, without an app.
pub fn run_script(
    sql: &mut crate::model::SqlTab,
    text: &str,
    outcomes: Vec<tabletist_db::StatementOutcome>,
) {
    sql.text = text.into();
    let request = RequestId(sql.run.loaded.map_or(9, |last| last.0 + 1));
    let statements = tabletist_db::sql::statements(Driver::Sqlite.dialect(), text);
    let _ = sql.start_run(request, statements, tabletist_db::ScriptMode::ReadOnly);
    assert!(sql.finish_run(request, Ok(script_outcome(outcomes)), None));
}

/// The Omarchy template rendered for Tokyo Night.
pub fn tokyo_night() -> crate::theme::Palette {
    let colors = "mode\tdark\nbackground\t#1a1b26\ndark_background\t#16161e\n\
                  lighter_background\t#292e42\nforeground\t#c0caf5\nmuted\t#565f89\n\
                  accent\t#7aa2f7\nselection\t#283457\nred\t#f7768e\n\
                  green\t#9ece6a\nyellow\t#e0af68\ncyan\t#7dcfff\n\
                  orange\t#ff9e64\nmagenta\t#bb9af7\nblue\t#7aa2f7\n\
                  bright_red\t#f7768e\nbright_green\t#9ece6a\n";
    let rendered = fastframe_theme::omarchy::render_seed::<crate::theme::Palette>(
        include_str!("../../../contrib/omarchy/tabletist.json.tpl"),
        colors,
    )
    .expect("the template renders Tokyo Night");
    fastframe_theme::parse_palette(&rendered).expect("Tokyo Night's palette parses")
}

/// The latest release as the fake server gives it: Bookshop's notes are
/// the app's own.
pub const RELEASE: &str = include_str!("../../../tests/fixtures/update/latest.json");

//! Editing a Files connection's table, in both looks: a save finds each
//! row by its place in the file, and a table its connection does not edit
//! is not. On the Bookshop's `orders.csv`.

use tabletist_db::files::Format;
use tabletist_db::{
    ColumnInfo, ColumnMeta, Driver, FILE_POSITION, FileFacts, ObjectKind, ObjectRef, RowPage,
    Structure, Unedited, Value, ValueKind,
};

use crate::backend::Command;
use crate::model::{Action, ConnTabId, Pane, TabId};
use crate::testing::Harness;
use crate::theme::Look;

fn looks() -> [Look; 2] {
    [Look::macos(), Look::omarchy()]
}

/// `orders` as DuckDB reads `orders.csv`: three columns, no key.
fn orders_structure(format: Format, unedited: Option<Unedited>) -> Structure {
    let column = |name: &str, type_name: &str| ColumnInfo {
        name: name.into(),
        type_name: type_name.into(),
        nullable: true,
        ..ColumnInfo::default()
    };
    Structure {
        columns: vec![
            column("id", "BIGINT"),
            column("email", "VARCHAR"),
            column("total", "DOUBLE"),
        ],
        file: Some(FileFacts {
            path: "/data/bookshop-exports/orders.csv".into(),
            size: 1_600_000_000,
            format,
            options: String::new(),
        }),
        unedited,
        ..Structure::default()
    }
}

/// Three of its rows, each at its place in the file where `placed`.
fn orders_page(placed: bool) -> RowPage {
    let meta = |name: &str, type_name: &str, kind| ColumnMeta {
        name: name.into(),
        type_name: type_name.into(),
        kind,
    };
    RowPage {
        columns: vec![
            meta("id", "BIGINT", ValueKind::Numeric),
            meta("email", "VARCHAR", ValueKind::Text),
            meta("total", "DOUBLE", ValueKind::Numeric),
        ],
        rows: (1..=3)
            .map(|id| {
                vec![
                    Value::Int(id),
                    Value::Text(format!("reader{id}@example.com").into()),
                    Value::Float(id as f64 * 10.5),
                ]
            })
            .collect(),
        has_more: false,
        ordered_by_key: placed,
        elapsed: std::time::Duration::from_millis(3),
        positions: placed.then(|| vec![0, 1, 2]),
    }
}

/// A Files connection that may write, with `orders` open on `structure`
/// and `page`.
fn orders(look: Look, structure: Structure, page: RowPage) -> (Harness, ConnTabId, TabId) {
    let mut harness = Harness::new();
    harness.set_look(look);
    let tab = harness.connect_fake_as(false);
    harness.app.workspace_mut(tab).unwrap().driver = Driver::Files;
    harness.app.apply(Action::OpenObject {
        tab,
        object: ObjectRef::new("main", "orders"),
        kind: ObjectKind::Table,
        pin: true,
    });
    harness.answer_structure(structure);
    harness.answer_rows(page);
    let id = harness.app.workspace(tab).unwrap().active_tab.unwrap();
    harness.app.workspace_mut(tab).unwrap().pane = Pane::Grid;
    harness.settle();
    (harness, tab, id)
}

/// The last save that was sent.
fn sent(harness: &Harness) -> &tabletist_db::ChangeSet {
    let mut sent = harness.app.backend.sent.iter().rev();
    sent.find_map(|command| match command {
        Command::Write { changes, .. } => Some(changes),
        _ => None,
    })
    .expect("a save was sent")
}

fn place(at: i64) -> Vec<(String, Value)> {
    vec![(FILE_POSITION.to_owned(), Value::Int(at))]
}

#[test]
fn a_save_finds_each_row_of_a_file_by_its_place() {
    for look in looks() {
        let (mut harness, tab, id) =
            orders(look, orders_structure(Format::Csv, None), orders_page(true));
        super::insert_audit_tests::set(&mut harness, tab, id, (1, 2), "21.50");
        harness.app.apply(Action::WriteEdits { tab, id });
        let changes = sent(&harness);
        assert_eq!(changes.rows.len(), 1, "{}", look.name);
        assert_eq!(changes.rows[0].key, place(1), "{}", look.name);
        assert_eq!(changes.rows[0].set[0].column, "total");
        // Written: the page is read again, every row at its new place.
        let fetches = |harness: &Harness| {
            harness
                .app
                .backend
                .sent
                .iter()
                .filter(|command| matches!(command, Command::FetchRows { .. }))
                .count()
        };
        let before = fetches(&harness);
        harness.answer_written(Ok(tabletist_db::WriteOutcome::Written {
            inserted: Vec::new(),
            rows: vec![vec![
                Value::Int(2),
                Value::Text("reader2@example.com".into()),
                Value::Float(21.5),
            ]],
            gone: Vec::new(),
            elapsed: std::time::Duration::from_millis(14),
        }));
        harness.settle();
        assert_eq!(fetches(&harness), before + 1, "{}", look.name);
    }
}

#[test]
fn an_excel_file_opens_read_only() {
    for look in looks() {
        let (mut harness, tab, id) = orders(
            look,
            orders_structure(Format::Excel, Some(Unedited::Excel)),
            orders_page(false),
        );
        // No cell of it is edited, and no row added.
        let workspace = harness.app.workspace(tab).unwrap();
        let object = workspace.object_tab(id).unwrap();
        let table = crate::edit::Table::of(workspace, object).unwrap();
        let lock = Some(crate::edit::Lock::Unedited(Unedited::Excel));
        assert_eq!(table.never(), lock, "{}", look.name);
        assert_eq!(table.no_rows(), lock, "{}", look.name);
        harness.settle();
    }
}

#[test]
fn a_file_whose_rows_have_no_place_is_not_edited() {
    let (harness, tab, id) = orders(
        Look::macos(),
        orders_structure(Format::Csv, None),
        orders_page(false),
    );
    let workspace = harness.app.workspace(tab).unwrap();
    let object = workspace.object_tab(id).unwrap();
    let table = crate::edit::Table::of(workspace, object).unwrap();
    assert_eq!(table.never(), Some(crate::edit::Lock::Unmatched));
    assert_eq!(
        super::cell_editor::lock_text(
            crate::edit::Lock::Unmatched,
            "orders",
            crate::i18n::Locale::English
        ),
        "orders opens read-only: Tabletist couldn't match its rows to the file's lines"
    );
}

#[test]
fn a_files_script_only_reads_and_says_where_files_are_edited() {
    let (harness, tab, _) = orders(
        Look::macos(),
        orders_structure(Format::Csv, None),
        orders_page(true),
    );
    let workspace = harness.app.workspace(tab).unwrap();
    assert_eq!(workspace.sql_writes(), Err(crate::model::NoWrites::Files));
}

//! The partial banner on a Files table's view: a page that reaches a row
//! the file's reading cannot take says which file, where and why, and
//! offers the three fixes. A fix is saved in the connection and the
//! session opens again with it; Skip closes the tab.

use std::path::PathBuf;

use egui::accesskit::Role;
use tabletist_db::files::{Change, Failure, Format, Override, Source, Why};
use tabletist_db::{ConnectSpec, Driver, ObjectInfo, ObjectKind};

use crate::backend::{Command, Event, StateFile};
use crate::connections::{ConnectionId, PasswordMode, SavedConnection};
use crate::env::Environment;
use crate::model::{Action, ConnTabId, FileFix, TabId};
use crate::testing::{Harness, last_sent, node};
use crate::theme::Look;

fn looks() -> [Look; 2] {
    [Look::macos(), Look::omarchy()]
}

const FOLDER: &str = "/data/bookshop-exports";

/// `returns.csv`'s row 48,112, whose `N/A` its date column cannot take.
fn late_na() -> Failure {
    Failure {
        file: "returns.csv".into(),
        path: PathBuf::from(FOLDER).join("returns.csv"),
        format: Format::Csv,
        why: Why::Value {
            row: 48_112,
            column: "returned_at".into(),
            value: "N/A".into(),
            kind: "DATE".into(),
        },
    }
}

/// The error a page of `returns` comes back with: DuckDB's words, and the
/// file they are of.
fn unread(failure: Failure) -> tabletist_db::Error {
    let mut error = tabletist_db::Error::query(
        "Conversion Error: CSV Error on Line: 48113\nCould not convert string \"N/A\" to 'DATE'",
    );
    if let tabletist_db::Error::Query { named, .. } = &mut error {
        named.file = Some(failure);
    }
    error
}

/// The saved Bookshop exports connection, over its folder.
fn exports() -> SavedConnection {
    SavedConnection {
        id: ConnectionId::new(),
        name: "Bookshop exports".into(),
        group: None,
        environment: Environment::Local,
        read_only: Some(true),
        password: PasswordMode::None,
        ssh_secret: PasswordMode::None,
        spec: ConnectSpec::files(Source::Folder(FOLDER.into())),
        agents: Default::default(),
    }
}

fn table(name: &str) -> ObjectInfo {
    ObjectInfo {
        name: name.into(),
        kind: ObjectKind::Table,
        estimated_rows: None,
    }
}

/// Connects the Bookshop exports in `look` through the recording backend.
fn connected(look: Look) -> (Harness, ConnTabId, ConnectionId) {
    let mut harness = Harness::new();
    harness.set_look(look);
    let saved = exports();
    let conn = saved.id.clone();
    harness.app.connections.upsert(saved);
    let tab = harness.app.active_tab_id();
    harness.app.apply(Action::Connect {
        tab,
        conn: conn.clone(),
    });
    answer_connect(&mut harness);
    crate::testing::answer_tree(
        &mut harness.app,
        vec!["main".into()],
        vec![table("orders"), table("returns")],
    );
    harness.settle();
    (harness, tab, conn)
}

/// Answers the newest Connect as a Files session that opened.
fn answer_connect(harness: &mut Harness) {
    let Command::Connect {
        session, request, ..
    } = *last_sent(&harness.app)
    else {
        panic!("expected Connect: {:?}", last_sent(&harness.app));
    };
    harness.app.apply(Action::Backend(Event::Connected {
        session,
        request,
        driver: Driver::Files,
        encrypted: false,
        access: crate::testing::asked_access(&harness.app),
    }));
}

/// Opens `returns` and answers its page with the file failure `failure`.
/// Returns the table's tab.
fn failed(harness: &mut Harness, failure: Failure) -> TabId {
    harness.click("returns");
    let (session, request) = match last_sent(&harness.app) {
        Command::Open { session, rows, .. } => (*session, *rows),
        other => panic!("{other:?}"),
    };
    harness.app.apply(Action::Backend(Event::Rows {
        session,
        request,
        result: Err(unread(failure)),
    }));
    harness.settle();
    let (_, object_tab) = harness.app.active_workspace_tab().expect("the returns tab");
    object_tab
}

/// The overrides the saved connection reads with.
fn saved_overrides(harness: &Harness, conn: &ConnectionId) -> Vec<Override> {
    harness
        .app
        .connections
        .get(conn)
        .expect("saved")
        .spec
        .csv
        .overrides
        .clone()
}

/// The overrides the newest Connect asked for.
fn connected_with(harness: &Harness) -> Vec<Override> {
    harness
        .app
        .backend
        .sent
        .iter()
        .rev()
        .find_map(|command| match command {
            Command::Connect { spec, .. } => Some(spec.csv.overrides.clone()),
            _ => None,
        })
        .expect("a Connect was sent")
}

/// How many times the connections file was saved.
fn saves(harness: &Harness) -> usize {
    harness
        .app
        .backend
        .sent
        .iter()
        .filter(|command| {
            matches!(
                command,
                Command::Save {
                    file: StateFile::Connections(_),
                    ..
                }
            )
        })
        .count()
}

fn text_override() -> Override {
    Override {
        file: "returns.csv".into(),
        change: Change::Text {
            column: "returned_at".into(),
        },
    }
}

#[test]
fn reading_the_column_as_text_saves_it_and_opens_the_session_again() {
    let (mut harness, tab, conn) = connected(Look::macos());
    let object_tab = failed(&mut harness, late_na());
    let before = saves(&harness);
    harness.app.apply(Action::FixTableFile {
        tab,
        object_tab,
        fix: FileFix::Text,
    });
    assert_eq!(saved_overrides(&harness, &conn), [text_override()]);
    assert_eq!(saves(&harness), before + 1);
    // The session opens again with it, and the tab stays.
    assert_eq!(connected_with(&harness), [text_override()]);
    assert!(
        harness
            .app
            .workspace(tab)
            .and_then(|workspace| workspace.object_tab(object_tab))
            .is_some()
    );
    // Once it is up, the table reads again: a read sent after the
    // session's answer, not the one that failed.
    let sent = harness.app.backend.sent.len();
    answer_connect(&mut harness);
    let reads = harness.app.backend.sent[sent..].iter().any(|command| {
        matches!(
            command,
            Command::FetchRows { query, .. } | Command::Open { query, .. }
                if query.object.name == "returns"
        )
    });
    assert!(reads, "{:?}", harness.app.backend.sent.last());
}

/// A fix takes the banner off at once: a second press while the session
/// opens again neither saves the override twice nor opens it again.
#[test]
fn a_fix_pressed_twice_saves_it_once() {
    let (mut harness, tab, conn) = connected(Look::macos());
    let object_tab = failed(&mut harness, late_na());
    harness.app.apply(Action::FixTableFile {
        tab,
        object_tab,
        fix: FileFix::Text,
    });
    let connects = |harness: &Harness| {
        let sent = harness.app.backend.sent.iter();
        sent.filter(|command| matches!(command, Command::Connect { .. }))
            .count()
    };
    let opened = connects(&harness);
    let file_failure = |harness: &Harness| {
        harness
            .app
            .workspace(tab)
            .and_then(|workspace| workspace.object_tab(object_tab))
            .and_then(crate::model::ObjectTab::file_failure)
            .cloned()
    };
    assert_eq!(file_failure(&harness), None);
    harness.app.apply(Action::FixTableFile {
        tab,
        object_tab,
        fix: FileFix::Empty,
    });
    assert_eq!(saved_overrides(&harness, &conn), [text_override()]);
    assert_eq!(connects(&harness), opened);
}

/// The saved connection was changed to read another folder while the tab
/// stayed open: the fix mends the tab's session, and is not saved where a
/// file of that name may be another.
#[test]
fn a_fix_is_not_saved_in_a_connection_that_reads_another_source_now() {
    let (mut harness, tab, conn) = connected(Look::macos());
    let object_tab = failed(&mut harness, late_na());
    let mut saved = harness.app.connections.get(&conn).expect("saved").clone();
    saved.spec.files = Some(Source::Folder("/data/bookshop-archive".into()));
    harness.app.connections.upsert(saved);
    let before = saves(&harness);
    harness.app.apply(Action::FixTableFile {
        tab,
        object_tab,
        fix: FileFix::Text,
    });
    assert!(saved_overrides(&harness, &conn).is_empty());
    assert_eq!(saves(&harness), before);
    assert_eq!(connected_with(&harness), [text_override()]);
}

#[test]
fn reading_the_value_as_empty_saves_that_value_for_the_column() {
    let (mut harness, tab, conn) = connected(Look::macos());
    let object_tab = failed(&mut harness, late_na());
    harness.app.apply(Action::FixTableFile {
        tab,
        object_tab,
        fix: FileFix::Empty,
    });
    let empty = Override {
        file: "returns.csv".into(),
        change: Change::Empty {
            column: "returned_at".into(),
            values: vec!["N/A".into()],
        },
    };
    assert_eq!(
        saved_overrides(&harness, &conn),
        std::slice::from_ref(&empty)
    );
    assert_eq!(connected_with(&harness), [empty]);
}

#[test]
fn skipping_the_file_closes_its_tab_and_leaves_it_out() {
    let (mut harness, tab, conn) = connected(Look::macos());
    let object_tab = failed(&mut harness, late_na());
    harness.app.apply(Action::FixTableFile {
        tab,
        object_tab,
        fix: FileFix::Skip,
    });
    let skip = Override {
        file: "returns.csv".into(),
        change: Change::Skip,
    };
    assert_eq!(
        saved_overrides(&harness, &conn),
        std::slice::from_ref(&skip)
    );
    assert_eq!(connected_with(&harness), [skip]);
    assert!(
        harness
            .app
            .workspace(tab)
            .and_then(|workspace| workspace.object_tab(object_tab))
            .is_none()
    );
}

/// A line of an NDJSON file that is not JSON has no column to read as
/// text or a value to read as empty: only Skip mends it.
#[test]
fn a_column_fix_for_a_failure_not_of_a_value_changes_nothing() {
    let (mut harness, tab, conn) = connected(Look::macos());
    let object_tab = failed(
        &mut harness,
        Failure {
            file: "returns.csv".into(),
            path: PathBuf::from(FOLDER).join("returns.csv"),
            format: Format::Csv,
            why: Why::Columns {
                row: 48_112,
                expected: 3,
                found: 4,
            },
        },
    );
    let sent = harness.app.backend.sent.len();
    harness.app.apply(Action::FixTableFile {
        tab,
        object_tab,
        fix: FileFix::Text,
    });
    assert!(saved_overrides(&harness, &conn).is_empty());
    assert_eq!(harness.app.backend.sent.len(), sent);
}

/// The fixes' buttons in `look`, as the banner names them.
fn fixes(look: Look) -> [String; 3] {
    match look.terminal {
        true => [
            "read returned_at as text".into(),
            "read 'N/A' as empty".into(),
            "skip returns.csv".into(),
        ],
        false => [
            "Read returned_at as text".into(),
            "Read 'N/A' as empty".into(),
            "Skip returns.csv".into(),
        ],
    }
}

#[test]
fn the_table_says_which_file_and_why_with_its_three_fixes() {
    for look in looks() {
        let (mut harness, _, _) = connected(look);
        failed(&mut harness, late_na());
        let tree = harness.settle();
        let labels = crate::testing::labels(&tree);
        let said = match look.terminal {
            true => "! returns.csv row 48,112 has 'N/A' in returned_at, a date column",
            false => {
                "returns.csv couldn't be read. Row 48,112 has 'N/A' in returned_at, a date column."
            }
        };
        assert!(
            node(&tree, said, Role::Alert).is_some(),
            "{}: {labels:?}",
            look.name
        );
        for fix in fixes(look) {
            assert!(
                node(&tree, &fix, Role::Button).is_some(),
                "{}: {fix} in {labels:?}",
                look.name
            );
        }
        // The banner stands for the error card: no Retry.
        assert!(
            node(&tree, &look.label("Retry"), Role::Button).is_none(),
            "{}",
            look.name
        );
    }
}

#[test]
fn a_fix_pressed_on_the_table_is_saved_in_the_connection() {
    for look in looks() {
        let (mut harness, _, conn) = connected(look);
        failed(&mut harness, late_na());
        harness.click(&fixes(look)[0]);
        assert_eq!(
            saved_overrides(&harness, &conn),
            [text_override()],
            "{}",
            look.name
        );
    }
}

/// A row with more values than the file's others has no column to read
/// as text: Skip alone is offered.
#[test]
fn a_failure_not_of_a_value_offers_skip_alone() {
    let (mut harness, _, _) = connected(Look::macos());
    failed(
        &mut harness,
        Failure {
            file: "returns.csv".into(),
            path: PathBuf::from(FOLDER).join("returns.csv"),
            format: Format::Csv,
            why: Why::Columns {
                row: 48_112,
                expected: 3,
                found: 4,
            },
        },
    );
    let tree = harness.settle();
    let labels = crate::testing::labels(&tree);
    assert!(
        labels
            .iter()
            .any(|label| label.contains("Row 48,112 has 4 values, where the file's rows have 3.")),
        "{labels:?}"
    );
    let [text, empty, skip] = fixes(Look::macos());
    assert!(node(&tree, &text, Role::Button).is_none());
    assert!(node(&tree, &empty, Role::Button).is_none());
    assert!(node(&tree, &skip, Role::Button).is_some());
}

/// Any other error keeps the error card and its Retry.
#[test]
fn an_error_not_of_the_file_keeps_the_error_card() {
    let (mut harness, _, _) = connected(Look::macos());
    harness.click("returns");
    let (session, request) = match last_sent(&harness.app) {
        Command::Open { session, rows, .. } => (*session, *rows),
        other => panic!("{other:?}"),
    };
    harness.app.apply(Action::Backend(Event::Rows {
        session,
        request,
        result: Err(tabletist_db::Error::query(
            "Binder Error: Referenced column \"nope\" not found",
        )),
    }));
    let tree = harness.settle();
    assert!(node(&tree, "Retry", Role::Button).is_some());
    assert!(node(&tree, &fixes(Look::macos())[2], Role::Button).is_none());
}

/// The returns table's banner in the terminal look, with the keyboard in
/// its grid.
fn failed_in_the_grid() -> (Harness, ConnTabId, ConnectionId, TabId) {
    let (mut harness, tab, conn) = connected(Look::omarchy());
    let object_tab = failed(&mut harness, late_na());
    harness.app.workspace_mut(tab).expect("a workspace").pane = crate::model::Pane::Grid;
    harness.settle();
    (harness, tab, conn, object_tab)
}

#[test]
fn t_and_e_read_the_column_as_text_and_the_value_as_empty() {
    use egui::{Key, Modifiers};
    let (mut harness, _, conn, _) = failed_in_the_grid();
    harness.press(Key::T, Modifiers::NONE);
    assert_eq!(saved_overrides(&harness, &conn), [text_override()]);
    let (mut harness, _, conn, _) = failed_in_the_grid();
    harness.press(Key::E, Modifiers::NONE);
    assert_eq!(
        saved_overrides(&harness, &conn),
        [Override {
            file: "returns.csv".into(),
            change: Change::Empty {
                column: "returned_at".into(),
                values: vec!["N/A".into()],
            },
        }]
    );
}

#[test]
fn x_skips_the_file_and_closes_its_tab() {
    use egui::{Key, Modifiers};
    let (mut harness, tab, conn, object_tab) = failed_in_the_grid();
    harness.press(Key::X, Modifiers::NONE);
    assert_eq!(
        saved_overrides(&harness, &conn),
        [Override {
            file: "returns.csv".into(),
            change: Change::Skip,
        }]
    );
    assert!(
        harness
            .app
            .workspace(tab)
            .and_then(|workspace| workspace.object_tab(object_tab))
            .is_none()
    );
}

/// `s` stays the table's Structure while the banner shows, and no fix
/// runs from there.
#[test]
fn s_still_shows_the_structure() {
    use egui::{Key, Modifiers};
    let (mut harness, tab, conn, object_tab) = failed_in_the_grid();
    harness.press(Key::S, Modifiers::NONE);
    let view = harness
        .app
        .workspace(tab)
        .and_then(|workspace| workspace.object_tab(object_tab))
        .map(|object| object.view);
    assert_eq!(view, Some(crate::model::ObjectView::Structure));
    harness.press(Key::T, Modifiers::NONE);
    assert!(saved_overrides(&harness, &conn).is_empty());
}

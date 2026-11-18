//! A Files connection's Reading CSV section and its closed Agents row, in
//! the connection dialog, in both looks: the rows open and close by click
//! and by Enter, say their state and their summary, each option changes
//! the form, Reset (and Omarchy's `R`) takes an override off, and Cancel
//! keeps what was saved.

use egui::accesskit::Role;
use egui::{Key, Modifiers};
use tabletist_db::ConnectSpec;
use tabletist_db::files::{Change, Delimiter, Header, Override, Reading, Source, Types};

use crate::connections::{AgentAccess, ConnectionId, PasswordMode, SavedConnection};
use crate::env::Environment;
use crate::model::{Action, ConnectionForm, Dialog};
use crate::testing::{Harness, node};
use crate::theme::Look;

fn looks() -> [Look; 2] {
    [Look::macos(), Look::omarchy()]
}

fn form(harness: &Harness) -> &ConnectionForm {
    match &harness.app.dialog {
        Some(Dialog::Connection(form)) => form,
        _ => panic!("no connection dialog"),
    }
}

fn form_mut(harness: &mut Harness) -> &mut ConnectionForm {
    match &mut harness.app.dialog {
        Some(Dialog::Connection(form)) => form,
        _ => panic!("no connection dialog"),
    }
}

/// A new connection's dialog in `look`, its type Files.
fn new_files(look: Look) -> Harness {
    let mut harness = Harness::new();
    harness.set_look(look);
    harness.app.apply(Action::NewConnection);
    harness.settle();
    harness.click(&look.label("Files"));
    harness
}

/// The closed Reading CSV row's name in `look`, for a reading with no
/// override.
fn closed(look: Look) -> String {
    let types = match look.terminal {
        true => "types first 20,000 rows",
        false => "types from the first 20,000 rows",
    };
    look.label(&format!(
        "Reading CSV, Header detect, delimiter detect, {types}"
    ))
}

/// Whether the button named `name` is there, and open.
fn expanded(harness: &mut Harness, name: &str) -> Option<bool> {
    let tree = harness.settle();
    let id = node(&tree, name, Role::Button)?;
    tree.nodes
        .iter()
        .find(|(node, _)| *node == id)
        .and_then(|(_, node)| node.is_expanded())
}

/// The label of what has the keyboard.
fn focused_label(harness: &mut Harness) -> Option<String> {
    let id = harness.ctx.memory(|memory| memory.focused())?;
    let tree = harness.settle();
    tree.nodes
        .iter()
        .find(|(node, _)| *node == id.accesskit_id())
        .and_then(|(_, node)| node.label().map(str::to_owned))
}

/// Tabs until what has the keyboard is named `name`.
fn tab_to(harness: &mut Harness, name: &str) {
    for _ in 0..40 {
        if focused_label(harness).as_deref() == Some(name) {
            return;
        }
        harness.press(Key::Tab, Modifiers::NONE);
    }
    panic!("Tab never reached {name:?}");
}

fn returns_as_text() -> Override {
    Override {
        file: "returns.csv".into(),
        change: Change::Text {
            column: "returned_at".into(),
        },
    }
}

fn skip_archive() -> Override {
    Override {
        file: "archive.csv".into(),
        change: Change::Skip,
    }
}

#[test]
fn reading_csv_is_a_closed_row_that_opens_by_click_and_by_enter() {
    for look in looks() {
        let mut harness = new_files(look);
        let name = closed(look);
        assert_eq!(expanded(&mut harness, &name), Some(false), "{}", look.name);
        assert!(!harness.has(&look.label("Header row")), "{}", look.name);
        harness.click(&name);
        assert!(form(&harness).csv_open, "{}", look.name);
        let title = look.label("Reading CSV");
        assert_eq!(expanded(&mut harness, &title), Some(true), "{}", look.name);
        for option in ["Header row", "Delimiter", "Column types"] {
            assert!(harness.has(&look.label(option)), "{}: {option}", look.name);
        }
        let hint = match look.terminal {
            true => "defaults for every csv",
            false => "Defaults for every CSV in this connection",
        };
        assert!(harness.has(hint), "{}", look.name);
        harness.click(&title);
        assert!(!form(&harness).csv_open, "{}", look.name);
        // The keyboard: a Tab stop, and Enter opens it and closes it.
        tab_to(&mut harness, &name);
        harness.press(Key::Enter, Modifiers::NONE);
        assert!(form(&harness).csv_open, "{}: Enter opens", look.name);
        // Tab goes from the heading into the section's choices.
        tab_to(&mut harness, &title);
        harness.press(Key::Tab, Modifiers::NONE);
        assert_eq!(
            focused_label(&mut harness).as_deref(),
            Some(look.label("Detect").as_str()),
            "{}: Tab into the section",
            look.name
        );
        tab_to(&mut harness, &title);
        harness.press(Key::Enter, Modifiers::NONE);
        assert!(!form(&harness).csv_open, "{}: Enter closes", look.name);
    }
    // Closed again each time the dialog opens.
    let mut harness = new_files(Look::macos());
    form_mut(&mut harness).csv_open = true;
    harness.app.apply(Action::CloseDialog);
    harness.app.apply(Action::NewConnection);
    harness.settle();
    harness.click("Files");
    assert!(!form(&harness).csv_open);
}

#[test]
fn each_option_changes_how_the_form_reads_csv() {
    for look in looks() {
        let mut harness = new_files(look);
        form_mut(&mut harness).csv_open = true;
        harness.click(&look.label("Yes"));
        assert_eq!(
            form(&harness).files_csv.header,
            Header::Yes,
            "{}",
            look.name
        );
        harness.click(";");
        assert_eq!(
            form(&harness).files_csv.delimiter,
            Delimiter::Semicolon,
            "{}",
            look.name
        );
        harness.click(&look.label("Tab"));
        assert_eq!(form(&harness).files_csv.delimiter, Delimiter::Tab);
        match look.terminal {
            true => harness.click("all text"),
            false => {
                harness.click("Column types");
                harness.click("Read everything as text");
            }
        }
        assert_eq!(form(&harness).files_csv.types, Types::Text, "{}", look.name);
        // The closed row says them.
        let title = look.label("Reading CSV");
        harness.click(&title);
        assert_eq!(
            expanded(
                &mut harness,
                &look.label("Reading CSV, Header yes, delimiter tab, everything as text")
            ),
            Some(false),
            "{}",
            look.name
        );
        // And the connection saves them.
        assert_eq!(
            form(&harness).files_csv,
            Reading {
                header: Header::Yes,
                delimiter: Delimiter::Tab,
                types: Types::Text,
                overrides: Vec::new(),
            }
        );
    }
}

#[test]
fn an_override_shows_under_the_options_and_reset_takes_it_off() {
    for look in looks() {
        let mut harness = new_files(look);
        form_mut(&mut harness).files_csv.overrides = vec![returns_as_text(), skip_archive()];
        let summary = closed(look).replace("rows", &look.label("rows, 2 overrides"));
        assert_eq!(
            expanded(&mut harness, &summary),
            Some(false),
            "{}",
            look.name
        );
        form_mut(&mut harness).csv_open = true;
        match look.terminal {
            false => {
                assert!(harness.has("returns.csv"));
                assert!(harness.has("· returned_at read as text"));
                assert!(harness.has("· skipped"));
                assert!(harness.has("Reset archive.csv, skipped"));
                harness.click("Reset returns.csv, returned_at read as text");
            }
            true => {
                let row = "override returns.csv, returned_at as text";
                assert!(node(&harness.settle(), row, Role::Button).is_some());
                tab_to(&mut harness, row);
                harness.press(Key::R, Modifiers::SHIFT);
            }
        }
        assert_eq!(
            form(&harness).files_csv.overrides,
            [skip_archive()],
            "{}",
            look.name
        );
    }
    // The file's own words keep their case in the terminal look.
    let mut harness = new_files(Look::omarchy());
    form_mut(&mut harness).files_csv.overrides = vec![Override {
        file: "Returns.CSV".into(),
        change: Change::Empty {
            column: "Reason".into(),
            values: vec!["N/A".into()],
        },
    }];
    form_mut(&mut harness).csv_open = true;
    let row = "override Returns.CSV, 'N/A' as empty in Reason";
    assert!(node(&harness.settle(), row, Role::Button).is_some());
    // `R` with nothing of the section's under the keyboard resets nothing.
    let mut harness = new_files(Look::omarchy());
    form_mut(&mut harness).files_csv.overrides = vec![returns_as_text()];
    form_mut(&mut harness).csv_open = true;
    harness.press(Key::R, Modifiers::SHIFT);
    assert_eq!(form(&harness).files_csv.overrides, [returns_as_text()]);
}

#[test]
fn a_skipped_file_is_left_out_of_the_found_line() {
    let mut harness = new_files(Look::macos());
    let scan = |names: &[&str]| tabletist_db::files::Scan {
        files: names
            .iter()
            .map(|name| tabletist_db::files::Found {
                path: format!("/data/bookshop-exports/{name}.csv").into(),
                format: tabletist_db::files::Format::Csv,
                size: 1024,
                name: (*name).to_owned(),
            })
            .collect(),
        missing: Vec::new(),
    };
    form_mut(&mut harness).files_found =
        crate::model::FilesFound::Read(scan(&["orders", "returns"]));
    assert!(harness.has("2 files · 2.0 KB · CSV"));
    form_mut(&mut harness).files_csv.overrides = vec![Override {
        file: "returns.csv".into(),
        change: Change::Skip,
    }];
    assert!(harness.has("1 file · 1.0 KB · CSV"));
    assert!(!harness.has("returns"));
}

#[test]
fn cancel_keeps_how_the_saved_connection_reads_csv() {
    let mut harness = Harness::new();
    let mut spec = ConnectSpec::files(Source::Folder("/data/bookshop-exports".into()));
    spec.csv.overrides = vec![returns_as_text()];
    let saved = SavedConnection {
        id: ConnectionId::new(),
        name: "Bookshop exports".into(),
        environment: Environment::Local,
        read_only: Some(true),
        password: PasswordMode::None,
        ssh_secret: PasswordMode::None,
        spec,
        agents: Default::default(),
    };
    let id = saved.id.clone();
    harness.app.connections.upsert(saved);
    harness.app.apply(Action::EditConnection(id.clone()));
    harness.settle();
    assert_eq!(form(&harness).files_csv.overrides, [returns_as_text()]);
    form_mut(&mut harness).csv_open = true;
    harness.click("Reset returns.csv, returned_at read as text");
    assert!(form(&harness).files_csv.overrides.is_empty());
    harness.app.apply(Action::CloseDialog);
    harness.settle();
    assert_eq!(
        harness.app.connections.get(&id).unwrap().spec.csv.overrides,
        [returns_as_text()]
    );
}

#[test]
fn agents_on_files_is_a_closed_row_that_says_the_access() {
    if !crate::endpoint::AVAILABLE {
        return;
    }
    for look in looks() {
        let mut harness = new_files(look);
        let read = look.label("Agents, Read, agents can query the files, never change them");
        assert_eq!(expanded(&mut harness, &read), Some(false), "{}", look.name);
        assert!(
            !harness.has(&look.label("Rows per answer")),
            "{}",
            look.name
        );
        harness.click(&read);
        assert!(form(&harness).agents_open, "{}", look.name);
        assert!(harness.has(&look.label("Rows per answer")), "{}", look.name);
        harness.click(&look.label("Off"));
        harness.click(&look.label("Agents"));
        let off = look.label("Agents, Off, agents can't reach this connection");
        assert_eq!(expanded(&mut harness, &off), Some(false), "{}", look.name);
        assert_eq!(form(&harness).agents.access, AgentAccess::Off);
    }
    // The list menu's Agents… opens the row.
    let mut harness = Harness::new();
    let saved = SavedConnection {
        id: ConnectionId::new(),
        name: "Bookshop exports".into(),
        environment: Environment::Local,
        read_only: Some(true),
        password: PasswordMode::None,
        ssh_secret: PasswordMode::None,
        spec: ConnectSpec::files(Source::Folder("/data/bookshop-exports".into())),
        agents: Default::default(),
    };
    let id = saved.id.clone();
    harness.app.connections.upsert(saved);
    harness.app.apply(Action::EditAgents {
        conn: id,
        allow: false,
    });
    harness.settle();
    assert!(form(&harness).agents_open);
    assert!(harness.has("Rows per answer"));
    // Every other type keeps its Agents part open, as before.
    let mut harness = new_files(Look::macos());
    harness.click("PostgreSQL");
    assert!(harness.has("Rows per answer"));
}

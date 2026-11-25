//! A Files connection's folder that macOS's privacy settings did not let
//! Tabletist open: the dialog's Test and the window's connect card say so
//! in the boards' words, and offer Open Privacy & Security, which opens
//! System Settings at Files and Folders.

use std::path::PathBuf;

use egui::accesskit::Role;
use tabletist_db::files::{Format, Found, Scan, Source};
use tabletist_db::{ConnectSpec, Error};

use crate::backend::{Command, Event};
use crate::connections::{ConnectionId, PasswordMode, SavedConnection};
use crate::env::Environment;
use crate::model::{Action, ConnectionForm, Dialog, FilesFound, PickTarget, TestState};
use crate::testing::{Harness, last_sent, node};
use crate::theme::Look;

fn looks() -> [Look; 2] {
    [Look::macos(), Look::omarchy()]
}

/// With capitals, which the path keeps in the terminal look's lower case.
const FOLDER: &str = "/data/Bookshop-Exports";

/// What the backend says of the folder macOS's privacy settings refused.
fn refused() -> String {
    format!("Can't open {FOLDER}: macOS hasn't allowed Tabletist to open this folder")
}

fn form(harness: &Harness) -> &ConnectionForm {
    match &harness.app.dialog {
        Some(Dialog::Connection(form)) => form,
        _ => panic!("no connection dialog"),
    }
}

/// A new Files connection's dialog in `look`, its folder the Bookshop's
/// exports, read.
fn exports(look: Look) -> Harness {
    let mut harness = Harness::new();
    harness.set_look(look);
    harness.app.apply(Action::NewConnection);
    harness.settle();
    harness.click(&look.label("Files"));
    harness.app.apply(Action::PickFolder);
    let request = form(&harness).pick_request.expect("a pick in flight");
    assert_eq!(form(&harness).pick_target, PickTarget::Folder);
    harness.app.apply(Action::Backend(Event::FilePicked {
        request,
        path: Some(FOLDER.into()),
    }));
    let FilesFound::Reading(request) = form(&harness).files_found else {
        panic!("no read of the source");
    };
    harness.app.apply(Action::Backend(Event::SourceRead {
        request,
        result: Ok(Scan {
            files: vec![Found {
                path: PathBuf::from(FOLDER).join("orders.csv"),
                format: Format::Csv,
                size: 1_000,
                name: "orders".into(),
            }],
            missing: Vec::new(),
            unreadable: Vec::new(),
        }),
    }));
    harness.settle();
    harness
}

/// Runs the dialog's Test and answers it with `error`.
fn tested(harness: &mut Harness, error: Error) {
    harness.app.apply(Action::TestConnection);
    let TestState::Running(request) = form(harness).test else {
        panic!("no test: {:?}", form(harness).test);
    };
    harness.app.apply(Action::Backend(Event::FilesTested {
        request,
        result: Err(error),
    }));
    harness.settle();
}

/// The banner's and the card's first words in `look`: the path keeps its
/// case.
fn lead(look: Look) -> String {
    look.label("Can't read {path}.").replace("{path}", FOLDER)
}

/// What the banner says in `look`, as a screen reader reads it.
fn said(look: Look) -> String {
    format!(
        "{} {}",
        lead(look),
        look.label("macOS hasn't allowed Tabletist to open this folder.")
    )
}

fn privacy(look: Look) -> String {
    look.label("Open Privacy & Security")
}

fn opened_privacy(harness: &Harness) -> bool {
    harness
        .app
        .backend
        .sent
        .iter()
        .any(|command| matches!(command, Command::OpenPrivacySettings))
}

#[test]
fn a_test_macos_refused_says_so_and_opens_privacy_and_security() {
    for look in looks() {
        let mut harness = exports(look);
        tested(&mut harness, Error::Connect(refused()));
        let tree = harness.settle();
        let labels = crate::testing::labels(&tree);
        assert!(
            node(&tree, &said(look), Role::Alert).is_some(),
            "{}: {labels:?}",
            look.name
        );
        // The banner says it: not the backend's words again over it.
        assert!(
            !labels.iter().any(|label| label.contains(&refused())),
            "{}: {labels:?}",
            look.name
        );
        assert!(!opened_privacy(&harness));
        harness.click(&privacy(look));
        assert!(opened_privacy(&harness), "{}", look.name);
    }
}

/// A folder that is gone, or that its own permissions refuse, is not
/// macOS's to allow: no way to Privacy & Security.
#[test]
fn other_failures_offer_no_privacy_and_security() {
    for why in ["no such folder", "not allowed to read it"] {
        let mut harness = exports(Look::macos());
        tested(
            &mut harness,
            Error::Connect(format!("Can't open {FOLDER}: {why}")),
        );
        let tree = harness.settle();
        assert!(
            node(&tree, &privacy(Look::macos()), Role::Button).is_none(),
            "{why}"
        );
    }
}

/// The found line under Source says it in the banner's words.
#[test]
fn the_found_line_says_macos_refused_the_folder() {
    let mut harness = exports(Look::macos());
    harness.app.apply(Action::ReadSource);
    let FilesFound::Reading(request) = form(&harness).files_found else {
        panic!("no read of the source");
    };
    harness.app.apply(Action::Backend(Event::SourceRead {
        request,
        result: Err(refused()),
    }));
    let tree = harness.settle();
    let labels = crate::testing::labels(&tree);
    assert!(
        labels
            .iter()
            .any(|label| label == "macOS hasn't allowed Tabletist to open this folder."),
        "{labels:?}"
    );
}

/// A Test that read past a file macOS's privacy settings refused: the
/// partial banner says so in its words, with Open Privacy & Security
/// first and Skip.
#[test]
fn a_test_past_a_file_macos_refused_offers_privacy_and_security() {
    use tabletist_db::FilesRead;
    use tabletist_db::files::{Failure, Why};
    for look in looks() {
        let mut harness = exports(look);
        harness.app.apply(Action::TestConnection);
        let TestState::Running(request) = form(&harness).test else {
            panic!("no test: {:?}", form(&harness).test);
        };
        harness.app.apply(Action::Backend(Event::FilesTested {
            request,
            result: Ok(FilesRead {
                files: 1,
                bytes: 1_000,
                failed: vec![Failure {
                    file: "orders.csv".into(),
                    path: PathBuf::from(FOLDER).join("orders.csv"),
                    format: Format::Csv,
                    why: Why::Other("macOS hasn't allowed Tabletist to open this file".into()),
                }],
                checks: Vec::new(),
            }),
        }));
        let tree = harness.settle();
        let labels = crate::testing::labels(&tree);
        let said = match look.terminal {
            true => "! orders.csv macos hasn't allowed tabletist to open this file",
            false => {
                "orders.csv couldn't be read. macOS hasn't allowed Tabletist to open this file."
            }
        };
        assert!(
            node(&tree, said, Role::Alert).is_some(),
            "{}: {labels:?}",
            look.name
        );
        assert!(
            node(&tree, &look.label("Skip orders.csv"), Role::Button).is_some(),
            "{}: {labels:?}",
            look.name
        );
        harness.click(&privacy(look));
        assert!(opened_privacy(&harness), "{}", look.name);
        // Privacy & Security mends nothing in the form.
        assert!(form(&harness).files_csv.overrides.is_empty());
    }
}

/// A list's found line names the file macOS refused.
#[test]
fn the_found_line_names_a_listed_file_macos_refused() {
    let mut harness = exports(Look::macos());
    harness.app.apply(Action::ReadSource);
    let FilesFound::Reading(request) = form(&harness).files_found else {
        panic!("no read of the source");
    };
    harness.app.apply(Action::Backend(Event::SourceRead {
        request,
        result: Err(format!(
            "Can't open {FOLDER}/orders.csv: macOS hasn't allowed Tabletist to open this file"
        )),
    }));
    let tree = harness.settle();
    let labels = crate::testing::labels(&tree);
    let line = format!(
        "Can't read {FOLDER}/orders.csv. macOS hasn't allowed Tabletist to open this file."
    );
    assert!(labels.contains(&line), "{labels:?}");
}

/// The saved Bookshop exports connection, over its folder.
fn saved_exports() -> SavedConnection {
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

#[test]
fn a_connect_macos_refused_says_so_with_privacy_and_security_first() {
    for look in looks() {
        let mut harness = Harness::new();
        harness.set_look(look);
        let saved = saved_exports();
        let conn = saved.id.clone();
        harness.app.connections.upsert(saved);
        let tab = harness.app.active_tab_id();
        harness.app.apply(Action::Connect { tab, conn });
        let Command::Connect {
            session, request, ..
        } = *last_sent(&harness.app)
        else {
            panic!("expected Connect");
        };
        harness.app.apply(Action::Backend(Event::ConnectFailed {
            session,
            request,
            error: Error::Connect(refused()),
        }));
        let tree = harness.settle();
        let labels = crate::testing::labels(&tree);
        let title = lead(look);
        assert!(labels.contains(&title), "{}: {labels:?}", look.name);
        assert!(
            labels
                .iter()
                .any(|label| *label
                    == look.label("macOS hasn't allowed Tabletist to open this folder.")),
            "{}: {labels:?}",
            look.name
        );
        // The card's buttons are named as the macOS look shows them.
        assert!(node(&tree, "Retry", Role::Button).is_some());
        harness.click("Open Privacy & Security");
        assert!(opened_privacy(&harness), "{}", look.name);
    }
}

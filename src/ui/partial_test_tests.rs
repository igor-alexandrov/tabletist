//! A Files connection's partial Test in the connection dialog: the
//! check in full that follows a Test, stopped once the dialog no
//! longer waits for it.

use std::path::PathBuf;

use tabletist_db::FilesRead;
use tabletist_db::files::{Failure, Format, Found, Scan, Why};

use crate::backend::{Command, Event, RequestId};
use crate::model::{Action, ConnectionForm, CsvChange, Dialog, FilesFound, PickTarget, TestState};
use crate::testing::Harness;
use crate::theme::Look;

fn form(harness: &Harness) -> &ConnectionForm {
    match &harness.app.dialog {
        Some(Dialog::Connection(form)) => form,
        _ => panic!("no connection dialog"),
    }
}

const FOLDER: &str = "/data/bookshop-exports";

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
    let found = |name: &str, format: Format| Found {
        path: PathBuf::from(FOLDER).join(name),
        format,
        size: 1_000,
        name: name.split('.').next().unwrap_or_default().to_owned(),
    };
    harness.app.apply(Action::Backend(Event::SourceRead {
        request,
        result: Ok(Scan {
            files: vec![
                found("orders.csv", Format::Csv),
                found("customers.parquet", Format::Parquet),
                found("events.ndjson", Format::Ndjson),
                found("returns.csv", Format::Csv),
            ],
            missing: Vec::new(),
        }),
    }));
    harness.settle();
    harness
}

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

/// Starts a Test and answers it: `failed` the files it could not read, and
/// `checks` the ones it checks in full next. Returns the Test's request.
fn tested(harness: &mut Harness, failed: Vec<Failure>, checks: &[&str]) -> RequestId {
    harness.app.apply(Action::TestConnection);
    let TestState::Running(request) = form(harness).test else {
        panic!("no test: {:?}", form(harness).test);
    };
    harness.app.apply(Action::Backend(Event::FilesTested {
        request,
        result: Ok(FilesRead {
            files: 4,
            bytes: 4_000,
            failed,
            checks: checks.iter().map(|name| (*name).to_owned()).collect(),
        }),
    }));
    harness.settle();
    request
}

fn checked(harness: &mut Harness, request: RequestId, failure: Option<Failure>, left: usize) {
    harness.app.apply(Action::Backend(Event::FileChecked {
        request,
        failure,
        left,
    }));
    harness.settle();
}

#[test]
fn the_check_stops_once_the_dialog_no_longer_waits_for_it() {
    let stopped = |harness: &Harness, request: RequestId| {
        harness.app.backend.sent.iter().any(
            |command| matches!(command, Command::StopCheck { request: stop } if *stop == request),
        )
    };
    // The reading changes: the Test is forgotten, its check stopped.
    let mut harness = exports(Look::macos());
    let request = tested(&mut harness, Vec::new(), &["orders", "returns"]);
    assert!(!stopped(&harness, request));
    harness.app.apply(Action::SetCsv(CsvChange::Header(
        tabletist_db::files::Header::Yes,
    )));
    assert!(stopped(&harness, request));
    // The dialog closes.
    let request = tested(&mut harness, Vec::new(), &["orders", "returns"]);
    harness.app.apply(Action::CloseDialog);
    assert!(stopped(&harness, request));
    // The Test runs again: the last one's check is stopped, and what it
    // still says is not taken.
    let mut harness = exports(Look::macos());
    let request = tested(&mut harness, Vec::new(), &["orders", "returns"]);
    harness.app.apply(Action::TestConnection);
    assert!(stopped(&harness, request));
    checked(&mut harness, request, Some(late_na()), 0);
    assert!(form(&harness).first_failure().is_none());
    // Let go of before it answers: the check that would follow is
    // stopped, the dialog closed or the Test run again.
    let mut harness = exports(Look::macos());
    harness.app.apply(Action::TestConnection);
    let TestState::Running(request) = form(&harness).test else {
        panic!("no test");
    };
    harness.app.apply(Action::CloseDialog);
    assert!(stopped(&harness, request));
    let mut harness = exports(Look::macos());
    harness.app.apply(Action::TestConnection);
    let TestState::Running(request) = form(&harness).test else {
        panic!("no test");
    };
    harness.app.apply(Action::TestConnection);
    assert!(stopped(&harness, request));
    // A check that ends is not stopped.
    let mut harness = exports(Look::macos());
    let request = tested(&mut harness, Vec::new(), &["returns"]);
    checked(&mut harness, request, None, 0);
    harness.app.apply(Action::CloseDialog);
    assert!(!stopped(&harness, request));
}

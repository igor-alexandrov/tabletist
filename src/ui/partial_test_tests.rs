//! A Files connection's partial Test in the connection dialog, in both
//! looks: the check in full that follows a Test and what the footer says
//! while it runs, the banner over the footer for the first file that could
//! not be read, its fixes (buttons, and `t`, `e`, `s` in the Omarchy look)
//! that save an override and run the Test again, and the check stopped
//! once the dialog no longer waits for it.

use std::path::PathBuf;

use egui::accesskit::Role;
use egui::{Key, Modifiers};
use tabletist_db::FilesRead;
use tabletist_db::files::{Change, Failure, Format, Found, Override, Scan, Why};

use crate::backend::{Command, Event, RequestId};
use crate::model::{Action, ConnectionForm, CsvChange, Dialog, FilesFound, PickTarget, TestState};
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
            unreadable: Vec::new(),
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

/// Whether a label starts with `text`.
fn shows(harness: &mut Harness, text: &str) -> bool {
    let tree = harness.settle();
    crate::testing::labels(&tree)
        .iter()
        .any(|label| label.starts_with(text))
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
fn the_check_in_full_says_what_is_left_and_a_failure_brings_the_banner() {
    for look in looks() {
        let mut harness = exports(look);
        let request = tested(&mut harness, Vec::new(), &["orders", "events", "returns"]);
        assert!(
            shows(&mut harness, &look.label("Read 4 files · 4.0 KB")),
            "{}",
            look.name
        );
        assert!(
            shows(&mut harness, &look.label("checking 3 files…")) || {
                let tree = harness.settle();
                crate::testing::labels(&tree)
                    .iter()
                    .any(|label| label.ends_with(&look.label("checking 3 files…")))
            }
        );
        checked(&mut harness, request, None, 2);
        checked(&mut harness, request, None, 1);
        let tree = harness.settle();
        assert!(
            crate::testing::labels(&tree)
                .iter()
                .any(|label| label.ends_with(&look.label("checking 1 file…"))),
            "{}: {:?}",
            look.name,
            crate::testing::labels(&tree)
        );
        // No banner while every file reads.
        assert!(node(&tree, &fixes(look)[2], Role::Button).is_none());
        checked(&mut harness, request, Some(late_na()), 0);
        let tree = harness.settle();
        let labels = crate::testing::labels(&tree);
        // The footer: what it read of how many, the file that failed.
        let footer = format!(
            "{} · returns.csv {}",
            look.label("Read 3 of 4 files"),
            look.label("failed")
        );
        assert!(labels.contains(&footer), "{}: {labels:?}", look.name);
        // The banner, an alert, names the file, the row, the value, the
        // column and its type, and offers three fixes.
        let said = match look.terminal {
            true => "! returns.csv row 48,112 has 'N/A' in returned_at, a date column",
            false => {
                "returns.csv couldn't be read. Row 48,112 has 'N/A' in returned_at, a date column."
            }
        };
        assert!(
            tree.nodes
                .iter()
                .any(|(_, node)| node.role() == Role::Alert && node.label() == Some(said)),
            "{}: {labels:?}",
            look.name
        );
        for fix in fixes(look) {
            assert!(
                node(&tree, &fix, Role::Button).is_some(),
                "{}: {fix}",
                look.name
            );
        }
    }
}

#[test]
fn each_fix_saves_an_override_under_reading_csv_and_tests_again() {
    let overrides = [
        Change::Text {
            column: "returned_at".into(),
        },
        Change::Empty {
            column: "returned_at".into(),
            values: vec!["N/A".into()],
        },
        Change::Skip,
    ];
    for look in looks() {
        for (at, change) in overrides.iter().enumerate() {
            let mut harness = exports(look);
            tested(&mut harness, vec![late_na()], &[]);
            let sent = harness.app.backend.sent.len();
            let reads = harness.app.backend.sources_read.len();
            harness.click(&fixes(look)[at]);
            // A skip reads the folder again, past the file it skips.
            assert_eq!(
                harness.app.backend.sources_read.len() - reads,
                usize::from(matches!(change, Change::Skip)),
                "{}: {change:?}",
                look.name
            );
            assert_eq!(
                form(&harness).files_csv.overrides,
                [Override {
                    file: "returns.csv".into(),
                    change: change.clone(),
                }],
                "{}: {change:?}",
                look.name
            );
            // The Test runs again, with the override.
            assert!(matches!(form(&harness).test, TestState::Running(_)));
            assert!(
                harness.app.backend.sent[sent..]
                    .iter()
                    .any(|command| matches!(
                        command,
                        Command::Test { spec, .. } if spec.csv.overrides.len() == 1
                    )),
                "{}: {change:?}",
                look.name
            );
        }
    }
}

#[test]
fn omarchy_runs_the_fixes_by_their_keys() {
    let look = Look::omarchy();
    for (key, change) in [
        (
            Key::T,
            Change::Text {
                column: "returned_at".into(),
            },
        ),
        (
            Key::E,
            Change::Empty {
                column: "returned_at".into(),
                values: vec!["N/A".into()],
            },
        ),
        (Key::S, Change::Skip),
    ] {
        let mut harness = exports(look);
        tested(&mut harness, vec![late_na()], &[]);
        // Off the name field, whose letters are typed.
        harness.ctx.memory_mut(|memory| {
            if let Some(focused) = memory.focused() {
                memory.surrender_focus(focused);
            }
        });
        harness.press(key, Modifiers::NONE);
        assert_eq!(
            form(&harness).files_csv.overrides,
            [Override {
                file: "returns.csv".into(),
                change,
            }],
            "{key:?}"
        );
        assert!(matches!(form(&harness).test, TestState::Running(_)));
        assert_eq!(form(&harness).name, "bookshop-exports", "{key:?} was typed");
    }
}

#[test]
fn a_failure_no_column_change_mends_offers_skip_alone() {
    for look in looks() {
        let mut harness = exports(look);
        let wide = Failure {
            why: Why::Columns {
                row: 30_001,
                expected: 3,
                found: 4,
            },
            ..late_na()
        };
        tested(&mut harness, vec![wide], &[]);
        let tree = harness.settle();
        let [text, empty, skip] = fixes(look);
        assert!(node(&tree, &skip, Role::Button).is_some(), "{}", look.name);
        assert!(node(&tree, &text, Role::Button).is_none(), "{}", look.name);
        assert!(node(&tree, &empty, Role::Button).is_none(), "{}", look.name);
        let row = look.label("Row 30,001 has 4 values, where the file's rows have 3");
        assert!(
            tree.nodes.iter().any(|(_, node)| node.role() == Role::Alert
                && node.label().is_some_and(|label| label.contains(&row))),
            "{}",
            look.name
        );
        // A column's fix does nothing here, by its key either.
        if look.terminal {
            harness.ctx.memory_mut(|memory| {
                if let Some(focused) = memory.focused() {
                    memory.surrender_focus(focused);
                }
            });
            harness.press(Key::T, Modifiers::NONE);
            assert!(form(&harness).files_csv.overrides.is_empty());
        }
    }
}

#[test]
fn with_several_failed_the_footer_counts_them_and_the_banner_says_the_first() {
    for look in looks() {
        let mut harness = exports(look);
        let events = Failure {
            file: "events.ndjson".into(),
            path: PathBuf::from(FOLDER).join("events.ndjson"),
            format: Format::Ndjson,
            why: Why::Json {
                line: 7,
                malformed: true,
            },
        };
        tested(&mut harness, vec![events, late_na()], &[]);
        let footer = look.label("Read 2 of 4 files · 2 failed");
        assert!(shows(&mut harness, &footer), "{}", look.name);
        let tree = harness.settle();
        let skip = look.label("Skip ") + "events.ndjson";
        assert!(node(&tree, &skip, Role::Button).is_some(), "{}", look.name);
        assert!(
            tree.nodes.iter().any(|(_, node)| node.role() == Role::Alert
                && node
                    .label()
                    .is_some_and(|label| label.contains(&look.label("Line 7 isn't JSON")))),
            "{}",
            look.name
        );
    }
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

/// A file named in a list beside another of its name is named by its path.
#[test]
fn a_fix_names_a_file_by_its_path_where_the_list_has_two_of_its_name() {
    let mut harness = exports(Look::macos());
    match &mut harness.app.dialog {
        Some(Dialog::Connection(form)) => {
            form.files_kind = crate::model::FilesKind::Files;
            form.files_list = vec![
                "/data/current/returns.csv".into(),
                "/data/archive/returns.csv".into(),
            ];
        }
        _ => panic!("no dialog"),
    }
    let archived = Failure {
        path: PathBuf::from("/data/archive/returns.csv"),
        ..late_na()
    };
    tested(&mut harness, vec![archived], &[]);
    harness.click("Skip returns.csv");
    assert_eq!(
        form(&harness).files_csv.overrides,
        [Override {
            file: "/data/archive/returns.csv".into(),
            change: Change::Skip,
        }]
    );
}

/// An entry of the list with spaces round it is the file the Test read:
/// another of its name, it names the failed file by its path.
#[test]
fn a_padded_entry_of_the_list_still_shares_the_failed_file_s_name() {
    let mut harness = exports(Look::macos());
    match &mut harness.app.dialog {
        Some(Dialog::Connection(form)) => {
            form.files_kind = crate::model::FilesKind::Files;
            form.files_list = vec![
                "/data/current/returns.csv".into(),
                "/data/archive/returns.csv ".into(),
            ];
        }
        _ => panic!("no dialog"),
    }
    let current = Failure {
        path: PathBuf::from("/data/current/returns.csv"),
        ..late_na()
    };
    tested(&mut harness, vec![current], &[]);
    harness.click("Skip returns.csv");
    assert_eq!(
        form(&harness).files_csv.overrides,
        [Override {
            file: "/data/current/returns.csv".into(),
            change: Change::Skip,
        }]
    );
}

/// A file listed by a relative path beside another of its name is named
/// by that path, which no other file of its name matches.
#[test]
fn a_fix_names_a_relative_file_by_its_path_where_the_list_has_two_of_its_name() {
    let mut harness = exports(Look::macos());
    match &mut harness.app.dialog {
        Some(Dialog::Connection(form)) => {
            form.files_kind = crate::model::FilesKind::Files;
            form.files_list = vec!["returns.csv".into(), "archive/returns.csv".into()];
        }
        _ => panic!("no dialog"),
    }
    let here = Failure {
        path: PathBuf::from("returns.csv"),
        ..late_na()
    };
    tested(&mut harness, vec![here], &[]);
    harness.click("Skip returns.csv");
    let overrides = &form(&harness).files_csv.overrides;
    assert_eq!(overrides.len(), 1);
    assert!(overrides[0].is_of(std::path::Path::new("returns.csv")));
    assert!(!overrides[0].is_of(std::path::Path::new("archive/returns.csv")));
}

/// A long value and name are cut short on the buttons, the banner says
/// them in full, and the actions stay inside the banner, wrapping to more
/// rows where they must.
#[test]
fn long_values_are_cut_short_on_the_buttons_and_the_actions_stay_inside() {
    let value = "lost in the post on its way back to the shop".repeat(3);
    for look in looks() {
        let mut harness = exports(look);
        let long = Failure {
            why: Why::Value {
                row: 9,
                column: "returned_at_according_to_the_courier_service".into(),
                value: value.clone(),
                kind: "INTERVAL".into(),
            },
            ..late_na()
        };
        tested(&mut harness, vec![long], &[]);
        let tree = harness.settle();
        let cut: String = value.chars().take(31).collect();
        let empty = format!("{} '{cut}…' {}", look.label("Read"), look.label("as empty"));
        let empty = node(&tree, &empty, Role::Button)
            .unwrap_or_else(|| panic!("{}: {:?}", look.name, crate::testing::labels(&tree)));
        let (alert, said) = tree
            .nodes
            .iter()
            .find(|(_, node)| node.role() == Role::Alert)
            .map(|(id, node)| (*id, node.label().unwrap_or_default()))
            .unwrap();
        assert!(said.contains(&value), "{}: {said}", look.name);
        assert!(
            said.contains(&look.label("an interval column")),
            "{}: {said}",
            look.name
        );
        let bounds = |id| {
            tree.nodes
                .iter()
                .find(|(each, _)| *each == id)
                .and_then(|(_, node)| node.bounds())
                .unwrap_or_else(|| panic!("no bounds: {id:?} alert {alert:?}"))
        };
        let (outer, inner) = (bounds(alert), bounds(empty));
        assert!(
            inner.x1 <= outer.x1 + 0.5 && inner.y1 <= outer.y1 + 0.5,
            "{}",
            look.name
        );
        let skip = node(&tree, &fixes(look)[2], Role::Button).unwrap();
        let skip = bounds(skip);
        assert!(
            skip.x1 <= outer.x1 + 0.5 && skip.y1 <= outer.y1 + 0.5,
            "{}",
            look.name
        );
    }
}

/// A held fix key mends one file: its repeats do not mend the next once
/// the Test has run again.
#[test]
fn a_held_fix_key_mends_one_file() {
    let mut harness = exports(Look::omarchy());
    let events = Failure {
        file: "events.ndjson".into(),
        path: PathBuf::from(FOLDER).join("events.ndjson"),
        format: Format::Ndjson,
        why: Why::Json {
            line: 7,
            malformed: true,
        },
    };
    tested(&mut harness, vec![late_na(), events.clone()], &[]);
    harness.ctx.memory_mut(|memory| {
        if let Some(focused) = memory.focused() {
            memory.surrender_focus(focused);
        }
    });
    harness.frame(vec![crate::testing::key(Key::S, Modifiers::NONE)]);
    // The Test runs again, and answers before the key comes up.
    let TestState::Running(request) = form(&harness).test else {
        panic!("no test");
    };
    harness.app.apply(Action::Backend(Event::FilesTested {
        request,
        result: Ok(FilesRead {
            files: 3,
            bytes: 3_000,
            failed: vec![events],
            checks: Vec::new(),
        }),
    }));
    harness.frame(vec![crate::testing::key(Key::S, Modifiers::NONE)]);
    harness.frame(vec![crate::testing::release(Key::S, Modifiers::NONE)]);
    harness.settle();
    assert_eq!(
        form(&harness).files_csv.overrides,
        [Override {
            file: "returns.csv".into(),
            change: Change::Skip,
        }]
    );
}

#[test]
fn one_file_that_failed_reads_as_one_file() {
    let mut harness = exports(Look::macos());
    harness.app.apply(Action::TestConnection);
    let TestState::Running(request) = form(&harness).test else {
        panic!("no test");
    };
    harness.app.apply(Action::Backend(Event::FilesTested {
        request,
        result: Ok(FilesRead {
            files: 1,
            bytes: 1_000,
            failed: vec![late_na()],
            checks: Vec::new(),
        }),
    }));
    assert!(shows(&mut harness, "Read 0 of 1 file · returns.csv failed"));
}

/// A value of any length leaves the fixes and the footer inside the
/// window: the banner's sentence ends in an ellipsis past its rows, and a
/// screen reader hears it all.
#[test]
fn a_very_long_value_leaves_the_fixes_and_the_footer_in_the_window() {
    let value = "N/A ".repeat(2_500);
    for look in looks() {
        let mut harness = exports(look);
        let long = Failure {
            why: Why::Value {
                row: 9,
                column: "returned_at".into(),
                value: value.clone(),
                kind: "DATE".into(),
            },
            ..late_na()
        };
        tested(&mut harness, vec![long], &[]);
        let tree = harness.settle();
        let said = tree
            .nodes
            .iter()
            .find(|(_, node)| node.role() == Role::Alert)
            .and_then(|(_, node)| node.label())
            .unwrap_or_default();
        assert!(said.contains(value.trim_end()), "{}", look.name);
        let bottom = |label: &str| {
            let id = node(&tree, label, Role::Button)
                .unwrap_or_else(|| panic!("{}: no {label:?}", look.name));
            tree.nodes
                .iter()
                .find(|(each, _)| *each == id)
                .and_then(|(_, node)| node.bounds())
                .unwrap_or_else(|| panic!("{}: {label:?} has no bounds", look.name))
                .y1
        };
        let window = f64::from(harness.size.y);
        for label in [fixes(look)[2].as_str(), "Cancel", "Save"] {
            assert!(bottom(label) <= window, "{}: {label:?}", look.name);
        }
    }
}

/// Delete stops the check, and Keep brings the form back without it: the
/// Test it followed comes back idle, as a running Test does, and nothing
/// says files are being checked.
#[test]
fn keep_after_delete_brings_back_no_check_that_is_not_running() {
    use crate::connections::{ConnectionId, PasswordMode, SavedConnection};
    let mut harness = Harness::new();
    let saved = SavedConnection {
        id: ConnectionId::new(),
        name: "Bookshop exports".into(),
        environment: crate::env::Environment::Local,
        read_only: Some(true),
        password: PasswordMode::None,
        ssh_secret: PasswordMode::None,
        spec: tabletist_db::ConnectSpec::files(tabletist_db::files::Source::Folder(FOLDER.into())),
        agents: Default::default(),
    };
    let id = saved.id.clone();
    harness.app.connections.upsert(saved);
    harness.app.apply(Action::EditConnection(id.clone()));
    harness.settle();
    let request = tested(&mut harness, Vec::new(), &["orders", "returns"]);
    assert!(form(&harness).files_check.is_some());
    harness.app.apply(Action::AskDeleteConnection(id));
    harness.settle();
    assert!(harness.app.backend.sent.iter().any(
        |command| matches!(command, Command::StopCheck { request: stop } if *stop == request),
    ));
    let tab = harness.app.active_tab_id();
    harness
        .app
        .apply(Action::AnswerDeleteConnection { tab, delete: false });
    let tree = harness.settle();
    assert_eq!(form(&harness).files_check, None);
    assert_eq!(form(&harness).test, TestState::Idle);
    assert!(
        !crate::testing::labels(&tree)
            .iter()
            .any(|label| label.contains("checking")),
        "{:?}",
        crate::testing::labels(&tree)
    );
}

/// The footer's partial line is whole phrases a catalog translates, each
/// count with its own plural: in Russian "of 4 files" and "checking 2
/// files" take forms the English words glued together could not.
#[test]
fn the_footer_reads_as_whole_phrases_in_russian() {
    use crate::i18n::Locale;
    use crate::settings::{Language, OptionValue};
    for look in looks() {
        let mut harness = exports(look);
        harness
            .app
            .apply(Action::SetOption(OptionValue::Language(Language::Fixed(
                Locale::Russian,
            ))));
        let request = tested(&mut harness, vec![late_na()], &["customers", "orders"]);
        let first =
            look.label("Прочитано 3 из 4 файлов") + " · returns.csv " + &look.label("не прочитан");
        assert!(shows(&mut harness, &first), "{}", look.name);
        let tree = harness.settle();
        assert!(
            crate::testing::labels(&tree)
                .iter()
                .any(|label| label.ends_with(&look.label("проверяются 2 файла…"))),
            "{}",
            look.name
        );
        checked(&mut harness, request, Some(late_na()), 1);
        let tree = harness.settle();
        assert!(
            crate::testing::labels(&tree)
                .iter()
                .any(|label| label.ends_with(&look.label("проверяется 1 файл…"))),
            "{}",
            look.name
        );
    }
}

/// The banner's sentence and fixes are in the catalogs: in Russian they
/// read in Russian, the file's and the column's names and the value as
/// they are.
#[test]
fn the_banner_and_its_fixes_read_in_russian() {
    use crate::i18n::Locale;
    use crate::settings::{Language, OptionValue};
    for look in looks() {
        let mut harness = exports(look);
        harness
            .app
            .apply(Action::SetOption(OptionValue::Language(Language::Fixed(
                Locale::Russian,
            ))));
        tested(&mut harness, vec![late_na()], &[]);
        let tree = harness.settle();
        let sentence = look.label("в returned_at стоит 'n/a', а это столбец дат");
        assert!(
            tree.nodes.iter().any(|(_, node)| node.role() == Role::Alert
                && node
                    .label()
                    .is_some_and(|label| label.to_lowercase().contains(&sentence.to_lowercase()))),
            "{}",
            look.name
        );
        let skip = look.label("Пропустить ") + "returns.csv";
        assert!(node(&tree, &skip, Role::Button).is_some(), "{}", look.name);
        let text = look.label("Читать ") + "returned_at" + &look.label(" как текст");
        assert!(node(&tree, &text, Role::Button).is_some(), "{}", look.name);
    }
}

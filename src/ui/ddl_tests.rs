//! The DDL view, checked against its spec
//! (`docs/superpowers/specs/2026-10-24-ddl-view-design.md`): the third
//! view of a table, read once and kept with the tab, its text read-only,
//! and a read that failed said as the database said it.

use std::time::{Duration, Instant};

use egui::accesskit::Role;
use egui::{Key, Modifiers};
use tabletist_db::ddl::{Part, PartKind};
use tabletist_db::{Ddl, Driver, Error, Named, ObjectKind, ObjectRef};

use crate::backend::Command;
use crate::model::{Action, ConnTabId, ObjectView, TabId};
use crate::testing::{Harness, book_covers_page, book_covers_structure, node};
use crate::theme::Look;

/// The Bookshop's `book_covers` as `driver` writes it: a table, two
/// indexes, two triggers (PostgreSQL's built from its catalog, with
/// comments).
pub(super) fn book_covers(driver: Driver) -> Ddl {
    let part = |kind, statements: &[&str]| Part {
        kind,
        statements: statements.iter().map(|s| s.to_string()).collect(),
    };
    match driver {
        Driver::Postgres => Ddl {
            driver,
            server: "PostgreSQL 17.2".into(),
            heading: Some("public.book_covers · built from the catalog · PostgreSQL 17.2".into()),
            parts: vec![
                part(
                    PartKind::Object,
                    &[
                        "CREATE TABLE public.book_covers (\n    id integer NOT NULL,\n    \
                       kind text NOT NULL,\n    CONSTRAINT book_covers_pkey PRIMARY KEY (id)\n);",
                    ],
                ),
                part(
                    PartKind::Indexes,
                    &[
                        "CREATE INDEX book_covers_book_id ON public.book_covers USING btree (book_id)",
                    ],
                ),
                part(
                    PartKind::Comments,
                    &[
                        "COMMENT ON TABLE public.book_covers IS 'Cover images, one for each side of a book';",
                    ],
                ),
            ],
        },
        Driver::MySql => Ddl {
            driver,
            server: "MySQL 8.4.3".into(),
            heading: None,
            parts: vec![part(
                PartKind::Object,
                &[
                    "CREATE TABLE `book_covers` (\n  `id` int NOT NULL,\n  PRIMARY KEY (`id`)\n) ENGINE=InnoDB",
                ],
            )],
        },
        Driver::Sqlite => Ddl {
            driver,
            server: "SQLite 3.46.0".into(),
            heading: None,
            parts: vec![
                part(
                    PartKind::Object,
                    &["CREATE TABLE book_covers (\n    id INTEGER PRIMARY KEY\n)"],
                ),
                part(
                    PartKind::Indexes,
                    &["CREATE INDEX book_covers_book_id ON book_covers (book_id)"],
                ),
            ],
        },
    }
}

/// The Bookshop's covers on `driver`, in `look`, their rows read.
pub(super) fn covers(look: Look, driver: Driver) -> (Harness, ConnTabId, TabId) {
    let mut harness = Harness::new();
    harness.set_look(look);
    let tab = harness.connect_fake();
    harness.app.workspace_mut(tab).unwrap().driver = driver;
    harness.app.apply(Action::OpenObject {
        tab,
        object: ObjectRef::new("main", "book_covers"),
        kind: ObjectKind::Table,
        pin: true,
    });
    harness.answer_structure(book_covers_structure());
    harness.answer_rows(book_covers_page(3));
    let id = harness.app.workspace(tab).unwrap().active_tab.unwrap();
    harness.settle();
    (harness, tab, id)
}

pub(super) fn set_view(harness: &mut Harness, tab: ConnTabId, id: TabId, view: ObjectView) {
    harness.app.apply(Action::SetView {
        tab,
        object_tab: id,
        view,
    });
    harness.settle();
}

/// The `ReadDdl`s sent since `from`, with what each asked for.
pub(super) fn reads(harness: &Harness, from: usize) -> Vec<ObjectRef> {
    harness.app.backend.sent[from..]
        .iter()
        .filter_map(|command| match command {
            Command::ReadDdl { object, .. } => Some(object.clone()),
            _ => None,
        })
        .collect()
}

/// The covers' DDL view on `driver` in `look`, its definition read.
pub(super) fn ddl_view(look: Look, driver: Driver) -> (Harness, ConnTabId, TabId) {
    let (mut harness, tab, id) = covers(look, driver);
    set_view(&mut harness, tab, id, ObjectView::Ddl);
    harness.answer_ddl(Ok(book_covers(driver)));
    harness.settle();
    (harness, tab, id)
}

/// The DDL view is the switch's third tab, after Rows and Structure,
/// chosen when it shows.
#[test]
fn the_switch_is_rows_structure_ddl_as_tabs() {
    for look in Look::ALL {
        let (mut harness, _tab, _id) = ddl_view(look, Driver::Postgres);
        let tree = harness.settle();
        let list = node(&tree, "View", Role::TabList).expect("the switch is a tab list");
        let of = |id| {
            tree.nodes
                .iter()
                .find(|(node, _)| *node == id)
                .map(|(_, node)| node)
        };
        // Its children in their order, which a screen reader reads.
        let tabs: Vec<(String, bool)> = of(list)
            .unwrap()
            .children()
            .iter()
            .filter_map(|id| of(*id))
            .filter(|node| node.role() == Role::Tab)
            .map(|node| {
                (
                    node.label().unwrap_or_default().to_owned(),
                    node.is_selected() == Some(true),
                )
            })
            .collect();
        assert_eq!(
            tabs,
            [
                ("Rows".to_owned(), false),
                ("Structure".to_owned(), false),
                ("DDL".to_owned(), true)
            ],
            "{}",
            look.name
        );
    }
}

/// The view reads the definition once, with the
/// app's own command (no run of the SQL editor's, nothing kept in
/// History), and shows it as a read-only text named for the table.
#[test]
fn the_view_reads_the_definition_and_shows_it_read_only() {
    for look in Look::ALL {
        let (mut harness, tab, id) = covers(look, Driver::Postgres);
        let from = harness.app.backend.sent.len();
        set_view(&mut harness, tab, id, ObjectView::Ddl);
        let sent = &harness.app.backend.sent[from..];
        assert!(
            sent.iter()
                .all(|command| matches!(command, Command::ReadDdl { .. })),
            "{sent:?}"
        );
        assert_eq!(
            reads(&harness, from),
            [ObjectRef::new("main", "book_covers")]
        );
        harness.answer_ddl(Ok(book_covers(Driver::Postgres)));
        let tree = harness.settle();
        let text = node(&tree, "DDL of book_covers", Role::MultilineTextInput)
            .expect("the text is a multi-line field");
        let (_, field) = tree.nodes.iter().find(|(node, _)| *node == text).unwrap();
        assert!(field.is_read_only(), "{}", look.name);
        assert_eq!(
            field.value(),
            Some(book_covers(Driver::Postgres).text().as_str()),
            "{}",
            look.name
        );
        // A number for each line, beside it.
        for number in ["1", "2", "8"] {
            assert!(
                harness.painted.iter().any(|(text, _)| text == number),
                "{number} in {}",
                look.name
            );
        }
        // Nothing of it reached a SQL editor's run or the history.
        let sent = &harness.app.backend.sent[from..];
        assert!(
            !sent
                .iter()
                .any(|command| matches!(command, Command::RunSql { .. } | Command::History(_))),
            "{sent:?}"
        );
    }
}

/// A definition read is kept with the tab; the view shown
/// again reads nothing, and Reload reads it again.
#[test]
fn a_second_open_reads_nothing_and_reload_reads_again() {
    let (mut harness, tab, id) = ddl_view(Look::macos(), Driver::Postgres);
    let from = harness.app.backend.sent.len();
    set_view(&mut harness, tab, id, ObjectView::Data);
    set_view(&mut harness, tab, id, ObjectView::Ddl);
    assert_eq!(reads(&harness, from), []);
    assert!(harness.has("DDL of book_covers"));
    harness.app.apply(Action::Refresh(tab));
    assert_eq!(
        reads(&harness, from),
        [ObjectRef::new("main", "book_covers")]
    );
    // The text stays while it is read again.
    assert!(harness.has("DDL of book_covers"));
}

/// Reload in another view forgets the definition: the DDL view reads it
/// when it is shown next.
#[test]
fn reload_in_another_view_forgets_the_definition() {
    let (mut harness, tab, id) = ddl_view(Look::macos(), Driver::Postgres);
    set_view(&mut harness, tab, id, ObjectView::Data);
    let from = harness.app.backend.sent.len();
    harness.app.apply(Action::Refresh(tab));
    assert_eq!(reads(&harness, from), []);
    set_view(&mut harness, tab, id, ObjectView::Ddl);
    assert_eq!(
        reads(&harness, from),
        [ObjectRef::new("main", "book_covers")]
    );
}

/// Leaving the view while it reads stops the read, and
/// coming back reads again.
#[test]
fn leaving_the_view_cancels_its_read() {
    let (mut harness, tab, id) = covers(Look::macos(), Driver::Postgres);
    set_view(&mut harness, tab, id, ObjectView::Ddl);
    let request = harness
        .app
        .backend
        .sent
        .iter()
        .rev()
        .find_map(|command| match command {
            Command::ReadDdl { request, .. } => Some(*request),
            _ => None,
        })
        .expect("a read was sent");
    let from = harness.app.backend.sent.len();
    set_view(&mut harness, tab, id, ObjectView::Structure);
    assert!(
        harness.app.backend.sent[from..]
            .iter()
            .any(|command| matches!(command, Command::Cancel { request: r, .. } if *r == request)),
        "{:?}",
        &harness.app.backend.sent[from..]
    );
    let object = harness.app.workspace(tab).unwrap().object_tab(id).unwrap();
    assert!(!object.ddl.is_loading());
    let from = harness.app.backend.sent.len();
    set_view(&mut harness, tab, id, ObjectView::Ddl);
    assert_eq!(
        reads(&harness, from),
        [ObjectRef::new("main", "book_covers")]
    );
}

/// MySQL's refusal, as the server words it.
fn denied() -> Error {
    Error::Query {
        code: Some("42000".into()),
        message: "SHOW command denied to user 'reader'@'%' for table 'book_covers'".into(),
        detail: None,
        hint: None,
        named: Box::new(Named {
            number: Some(1142),
            ..Named::default()
        }),
    }
}

/// A read the server refused says whose definition, the
/// server's number and words, and what it means; no text shows, and the
/// other views work on.
#[test]
fn a_refused_read_says_the_servers_error_and_shows_no_text() {
    for look in Look::ALL {
        let (mut harness, tab, id) = covers(look, Driver::MySql);
        set_view(&mut harness, tab, id, ObjectView::Ddl);
        harness.answer_ddl(Err(denied()));
        let title = look.label("Couldn't read the definition of book_covers");
        assert!(harness.has(&title), "{}", look.name);
        assert!(
            harness.has("1142 · SHOW command denied to user 'reader'@'%' for table 'book_covers'"),
            "{}",
            look.name
        );
        assert!(
            harness.has(
                "The connection's user can't run SHOW CREATE TABLE here. Rows and Structure still work."
            ),
            "{}",
            look.name
        );
        assert!(!harness.has("DDL of book_covers"), "{}", look.name);
        // Copy error copies the number and the words.
        harness.click("Copy error");
        assert_eq!(
            harness.copied.as_deref(),
            Some("1142 · SHOW command denied to user 'reader'@'%' for table 'book_covers'")
        );
        // Try again reads again.
        let from = harness.app.backend.sent.len();
        harness.click("Try again");
        assert_eq!(
            reads(&harness, from),
            [ObjectRef::new("main", "book_covers")]
        );
        // The rows and the structure show as they did.
        harness.answer_ddl(Err(denied()));
        set_view(&mut harness, tab, id, ObjectView::Data);
        assert!(harness.has("Row 1"), "{}", look.name);
        set_view(&mut harness, tab, id, ObjectView::Structure);
        assert!(harness.has("Columns"), "{}", look.name);
    }
}

/// A read that goes on says so once it has lasted: the macOS header and
/// the terminal's line, each announced.
#[test]
fn a_long_read_says_it_reads_the_definition() {
    for look in Look::ALL {
        let (mut harness, tab, id) = covers(look, Driver::Postgres);
        set_view(&mut harness, tab, id, ObjectView::Ddl);
        let words = if look.terminal {
            "reading the definition…"
        } else {
            "Reading the definition…"
        };
        assert!(!harness.has(words), "not at once, in {}", look.name);
        harness
            .app
            .workspace_mut(tab)
            .unwrap()
            .object_tab_mut(id)
            .unwrap()
            .ddl
            .started = Some(Instant::now() - Duration::from_secs(1));
        let tree = harness.settle();
        let said = node(&tree, words, Role::Label).expect("the wait is said");
        let (_, said) = tree.nodes.iter().find(|(id, _)| *id == said).unwrap();
        assert_eq!(
            said.live(),
            Some(egui::accesskit::Live::Polite),
            "{}",
            look.name
        );
    }
}

/// Where the text comes from, by engine: PostgreSQL's built from the
/// catalog, with what it leaves out; MySQL's and SQLite's as stored.
#[test]
fn the_view_says_where_its_text_comes_from() {
    // Each engine's source, its name, and whether the text leaves
    // anything out.
    let cases = [
        (
            Driver::Postgres,
            "built from the catalog",
            "PostgreSQL",
            true,
        ),
        (Driver::MySql, "as stored in the database", "MySQL", false),
        (Driver::Sqlite, "as stored in the database", "SQLite", false),
    ];
    for (driver, source, engine, leaves_out) in cases {
        let (mut harness, _, _) = ddl_view(Look::macos(), driver);
        assert!(
            harness.has(&format!("Read-only · {source} · read just now")),
            "{driver:?}"
        );
        let lines = book_covers(driver).text().lines().count();
        assert!(
            harness.has(&format!("{lines} lines · {engine}")),
            "{driver:?}"
        );
        let left_out =
            "Not included: owner, grants, sequences owned by the table, row-level policies";
        assert_eq!(harness.has(left_out), leaves_out, "{driver:?}");
        // Reload, with its key.
        let reload =
            crate::ui::keys::written(&harness.ctx, &Look::macos(), crate::keymap::Command::Reload);
        assert!(harness.has("Reload"), "{driver:?}");
        assert!(
            harness.painted.iter().any(|(text, _)| *text == *reload),
            "{driver:?}"
        );
        // The terminal says it over the text.
        let (mut harness, _, _) = ddl_view(Look::omarchy(), driver);
        let server = book_covers(driver).server.to_lowercase();
        assert!(
            harness.has(&format!("ddl · read-only · {source} · {server}")),
            "{driver:?}"
        );
    }
}

/// The source line's Reload reads the definition again.
#[test]
fn the_source_lines_reload_reads_again() {
    let (mut harness, _, _) = ddl_view(Look::macos(), Driver::Sqlite);
    let from = harness.app.backend.sent.len();
    harness.click("Reload");
    assert_eq!(
        reads(&harness, from),
        [ObjectRef::new("main", "book_covers")]
    );
}

/// The text is selected and copied as any text: all of it with the
/// keyboard.
#[test]
fn the_text_is_selected_and_copied_with_the_keyboard() {
    let (mut harness, tab, id) = ddl_view(Look::macos(), Driver::Postgres);
    let field = super::ddl::text_id(tab, id);
    harness.ctx.memory_mut(|memory| memory.request_focus(field));
    harness.settle();
    harness.press(Key::A, Modifiers::COMMAND);
    harness.copy(false);
    assert_eq!(
        harness.copied.as_deref(),
        Some(book_covers(Driver::Postgres).text().as_str())
    );
    // And it cannot be typed into.
    harness.frame(vec![egui::Event::Text("x".into())]);
    let object = harness.app.workspace(tab).unwrap().object_tab(id).unwrap();
    assert_eq!(
        object.ddl.value.as_ref().map(Ddl::text),
        Some(book_covers(Driver::Postgres).text())
    );
}

/// Every engine's definition shows in the view.
#[test]
fn every_engine_shows_its_definition() {
    for driver in [Driver::Postgres, Driver::MySql, Driver::Sqlite] {
        let (mut harness, _, _) = ddl_view(Look::macos(), driver);
        let tree = harness.settle();
        let text = node(&tree, "DDL of book_covers", Role::MultilineTextInput);
        assert!(text.is_some(), "{driver:?}");
    }
}

/// "read just now" to "read 2 h ago".
#[test]
fn the_age_is_said_in_its_largest_unit() {
    let locale = crate::i18n::Locale::English;
    let said = |seconds| super::ddl::read_ago(Duration::from_secs(seconds), locale);
    assert_eq!(said(0), "read just now");
    assert_eq!(said(12), "read 12 s ago");
    assert_eq!(said(180), "read 3 min ago");
    assert_eq!(said(7_300), "read 2 h ago");
}

/// The keys of `c` typed, as a window sends them: its key, then its text.
pub(super) fn letter(harness: &mut Harness, c: char) {
    let key = Key::from_name(&c.to_ascii_uppercase().to_string()).expect("a letter's key");
    let shift = if c.is_uppercase() {
        Modifiers::SHIFT
    } else {
        Modifiers::NONE
    };
    harness.frame(vec![
        crate::testing::key(key, shift),
        egui::Event::Text(c.to_string()),
    ]);
    harness.frame(vec![crate::testing::release(key, shift)]);
    harness.settle();
}

/// Gives the text the keyboard, as F6 does.
pub(super) fn focus_text(harness: &mut Harness, tab: ConnTabId, id: TabId) {
    let field = super::ddl::text_id(tab, id);
    harness.ctx.memory_mut(|memory| memory.request_focus(field));
    harness.settle();
}

/// The text field's value, as a screen reader reads it.
fn field_value(harness: &mut Harness) -> Option<String> {
    let tree = harness.settle();
    let field = node(&tree, "DDL of book_covers", Role::MultilineTextInput)?;
    let (_, field) = tree.nodes.iter().find(|(id, _)| *id == field)?;
    field.value().map(str::to_owned)
}

/// The sentence of a refusal shows only where the database refused: a
/// table gone since is no refusal. PostgreSQL's refusal has its own.
#[test]
fn only_a_refusal_says_what_the_user_may_not_do() {
    let (mut harness, tab, id) = covers(Look::macos(), Driver::MySql);
    set_view(&mut harness, tab, id, ObjectView::Ddl);
    harness.answer_ddl(Err(Error::query("no such table or view: book_covers")));
    assert!(harness.has("Couldn't read the definition of book_covers"));
    assert!(harness.has("no such table or view: book_covers"));
    assert!(!harness.has(
        "The connection's user can't run SHOW CREATE TABLE here. Rows and Structure still work."
    ));
    let (mut harness, tab, id) = covers(Look::macos(), Driver::Postgres);
    set_view(&mut harness, tab, id, ObjectView::Ddl);
    harness.answer_ddl(Err(Error::Query {
        code: Some("42501".into()),
        message: "permission denied for table book_covers".into(),
        detail: None,
        hint: None,
        named: Box::default(),
    }));
    assert!(harness.has("42501 · permission denied for table book_covers"));
    assert!(harness.has(
        "The connection's user can't read this part of the catalog. Rows and Structure still work."
    ));
}

/// Try again drops what the error replaced, and reads again.
#[test]
fn try_again_drops_the_text_under_the_error() {
    let (mut harness, tab, id) = ddl_view(Look::macos(), Driver::Postgres);
    harness.app.apply(Action::Refresh(tab));
    harness.answer_ddl(Err(denied()));
    assert!(harness.has("Try again"));
    let from = harness.app.backend.sent.len();
    harness.click("Try again");
    let object = harness.app.workspace(tab).unwrap().object_tab(id).unwrap();
    assert!(object.ddl.value.is_none());
    assert!(object.ddl.is_loading());
    assert_eq!(
        reads(&harness, from),
        [ObjectRef::new("main", "book_covers")]
    );
}

/// A new session reads the definition the tab in front shows. One that
/// was on its way on a tab behind is forgotten, and read when that tab
/// comes in front again.
#[test]
fn a_new_session_reads_the_definition_in_front_and_later_the_one_behind() {
    let (mut harness, tab, id) = covers(Look::macos(), Driver::Postgres);
    set_view(&mut harness, tab, id, ObjectView::Ddl);
    // The read is on its way when another tab comes in front.
    harness.click("users");
    let users = harness.app.workspace(tab).unwrap().active_tab.unwrap();
    assert_ne!(users, id);
    let from = harness.app.backend.sent.len();
    harness.reconnect_fake_as(tab, true);
    harness.settle();
    assert_eq!(reads(&harness, from), []);
    let object = harness.app.workspace(tab).unwrap().object_tab(id).unwrap();
    assert!(!object.ddl.is_loading());
    harness.app.apply(Action::ActivateTab { tab, id });
    harness.settle();
    assert_eq!(
        reads(&harness, from),
        [ObjectRef::new("main", "book_covers")]
    );
    // In front, it is read on the next new session at once.
    harness.answer_ddl(Ok(book_covers(Driver::Postgres)));
    let from = harness.app.backend.sent.len();
    harness.reconnect_fake_as(tab, true);
    assert_eq!(
        reads(&harness, from),
        [ObjectRef::new("main", "book_covers")]
    );
    harness.settle();
    assert_eq!(reads(&harness, from).len(), 1, "once");
}

/// Closing the tab stops its read.
#[test]
fn closing_the_tab_cancels_its_read() {
    let (mut harness, tab, id) = covers(Look::macos(), Driver::Postgres);
    set_view(&mut harness, tab, id, ObjectView::Ddl);
    let request = harness
        .app
        .workspace(tab)
        .unwrap()
        .object_tab(id)
        .unwrap()
        .ddl
        .pending
        .expect("a read on its way");
    let from = harness.app.backend.sent.len();
    harness.app.apply(Action::CloseTab { tab, id });
    assert!(
        harness.app.backend.sent[from..]
            .iter()
            .any(|command| matches!(command, Command::Cancel { request: r, .. } if *r == request))
    );
}

/// The DDL view has no Add row; the other views keep theirs.
#[test]
fn the_ddl_view_has_no_add_row() {
    let (mut harness, tab, id) = covers(Look::macos(), Driver::Postgres);
    harness.app.workspace_mut(tab).unwrap().access = tabletist_db::Access::Writable;
    assert!(harness.has("Add row"));
    set_view(&mut harness, tab, id, ObjectView::Structure);
    assert!(harness.has("Add row"));
    set_view(&mut harness, tab, id, ObjectView::Ddl);
    harness.answer_ddl(Ok(book_covers(Driver::Postgres)));
    assert!(!harness.has("Add row"));
    // And under the text, how to copy part of it.
    assert!(harness.has("Select text to copy part of it"));
}

/// The terminal's line names the view's keys, not the rows', and where
/// its cursor is of how many lines.
#[test]
fn the_terminal_line_names_the_views_keys_and_its_lines() {
    let (harness, _, _) = ddl_view(Look::omarchy(), Driver::Postgres);
    let lines = book_covers(Driver::Postgres).text().lines().count();
    let painted = |said: &str| harness.painted.iter().any(|(text, _)| text == said);
    assert!(painted(&format!("1/{lines}")));
    // Each key with its word, as the line paints a hint.
    let key = |command| crate::ui::keys::written(&harness.ctx, &Look::omarchy(), command);
    use crate::keymap::Command;
    for (command, word) in [
        (Command::MoveInDdl, "line"),
        (Command::SelectDdlLines, "select lines"),
        (Command::CopyDdl, "copy"),
        (Command::OpenDdlInEditor, "open in sql"),
        (Command::ShowRows, "rows"),
        (Command::ShowStructure, "structure"),
    ] {
        assert!(painted(&format!("{} {word}", &*key(command))), "{word}");
    }
    let rows = key(Command::MoveRow);
    assert!(
        !painted(&format!("{} row", &*rows)),
        "the rows' keys are not named"
    );
}

/// The arrows stay off the sidebar while the DDL view shows.
#[test]
fn the_arrows_do_not_go_to_the_sidebar_from_the_ddl_view() {
    let (mut harness, tab, _) = ddl_view(Look::macos(), Driver::Postgres);
    harness.app.workspace_mut(tab).unwrap().pane = crate::model::Pane::Grid;
    let before = harness.app.workspace(tab).unwrap().tree.cursor.clone();
    harness.press(Key::ArrowDown, Modifiers::NONE);
    assert_eq!(harness.app.workspace(tab).unwrap().tree.cursor, before);
}

/// F6 brings the keyboard to the text, in every look.
#[test]
fn f6_comes_to_the_text() {
    for look in Look::ALL {
        let (mut harness, _, _) = ddl_view(look, Driver::Postgres);
        let mut seen = Vec::new();
        for _ in 0..8 {
            harness.press(Key::F6, Modifiers::NONE);
            seen.push(super::tests::focused_name(&harness.settle()));
        }
        assert!(
            seen.iter().any(|name| name == "DDL of book_covers"),
            "{seen:?} in {}",
            look.name
        );
    }
}

/// Nothing typed, pasted or cut changes the text.
#[test]
fn the_text_cannot_be_changed() {
    let (mut harness, tab, id) = ddl_view(Look::macos(), Driver::Postgres);
    focus_text(&mut harness, tab, id);
    let text = book_covers(Driver::Postgres).text();
    harness.frame(vec![egui::Event::Text("x".into())]);
    harness.press(Key::Backspace, Modifiers::NONE);
    harness.press(Key::Enter, Modifiers::NONE);
    harness.frame(vec![egui::Event::Paste("DROP TABLE books;".into())]);
    harness.frame(vec![egui::Event::Cut]);
    assert_eq!(field_value(&mut harness).as_deref(), Some(text.as_str()));
}

/// With the keyboard in the text, Reload reads it again: Omarchy's `R`
/// and the others' chord. The rows' letters do nothing there.
#[test]
fn reload_reads_again_from_the_text() {
    let (mut harness, tab, id) = ddl_view(Look::omarchy(), Driver::Postgres);
    focus_text(&mut harness, tab, id);
    let from = harness.app.backend.sent.len();
    for c in ['o', 'x', 'd', 'd', 'i'] {
        letter(&mut harness, c);
    }
    let object = harness.app.workspace(tab).unwrap().object_tab(id).unwrap();
    assert!(object.edits.added.is_empty() && object.edits.deleted.is_empty());
    assert!(object.edits.editor.is_none());
    assert_eq!(reads(&harness, from), []);
    letter(&mut harness, 'R');
    assert_eq!(
        reads(&harness, from),
        [ObjectRef::new("main", "book_covers")]
    );
    let (mut harness, tab, id) = ddl_view(Look::macos(), Driver::Postgres);
    focus_text(&mut harness, tab, id);
    let from = harness.app.backend.sent.len();
    harness.press(Key::R, Modifiers::COMMAND);
    assert_eq!(
        reads(&harness, from),
        [ObjectRef::new("main", "book_covers")]
    );
}

/// The keyboard selects in the text as in any: Shift with an arrow from
/// where it came in, at the top. (The terminal look selects by line.)
#[test]
fn shift_with_an_arrow_selects_from_the_top() {
    for look in [Look::macos(), Look::standard()] {
        let (mut harness, tab, id) = ddl_view(look, Driver::Postgres);
        focus_text(&mut harness, tab, id);
        harness.press(Key::ArrowDown, Modifiers::SHIFT);
        harness.copy(false);
        let first = book_covers(Driver::Postgres).text();
        let first = format!("{}\n", first.lines().next().unwrap());
        assert_eq!(
            harness.copied.as_deref(),
            Some(first.as_str()),
            "{}",
            look.name
        );
    }
}

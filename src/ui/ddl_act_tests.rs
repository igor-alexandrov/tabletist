//! What the DDL view does with its text, checked against its spec
//! (`docs/superpowers/specs/2026-10-25-ddl-view-2-act-design.md`): Copy and
//! Open in SQL editor, the links to its parts and the terminal's outline,
//! the views' keys, and the terminal look's lines.

use egui::accesskit::Role;
use egui::{Key, Modifiers};
use tabletist_db::{Driver, ObjectKind, ObjectRef};

use super::ddl_tests::{book_covers, covers, ddl_view, focus_text, letter, reads, set_view};
use super::tests::{type_key, type_text};
use crate::backend::Command;
use crate::model::{Action, ConnTabId, ObjectView, RunMode, TabId};
use crate::testing::{Harness, node};
use crate::theme::Look;

/// ⌃⌘ as a Mac reports it: Control, and the command key by both names.
const CONTROL_CMD: Modifiers = Modifiers {
    alt: false,
    ctrl: true,
    shift: false,
    mac_cmd: true,
    command: true,
};

/// ⌘ alone, as a Mac reports it.
const CMD: Modifiers = Modifiers {
    alt: false,
    ctrl: false,
    shift: false,
    mac_cmd: true,
    command: true,
};

fn view(harness: &Harness, tab: ConnTabId, id: TabId) -> ObjectView {
    harness
        .app
        .workspace(tab)
        .unwrap()
        .object_tab(id)
        .unwrap()
        .view
}

fn cursor(harness: &Harness, tab: ConnTabId, id: TabId) -> (usize, Option<usize>) {
    let object = harness.app.workspace(tab).unwrap().object_tab(id).unwrap();
    (object.ddl_cursor, object.ddl_visual)
}

/// What the toast or the terminal's line says now.
fn said(harness: &Harness) -> Option<String> {
    crate::ui::toast::said(&harness.ctx)
}

/// The lines `from` to `to` of PostgreSQL's `book_covers`, as copied.
fn lines(from: usize, to: usize) -> String {
    let text = book_covers(Driver::Postgres).text();
    text.lines()
        .skip(from)
        .take(to - from)
        .collect::<Vec<_>>()
        .join("\n")
}

/// Copy copies all of the text where nothing is selected, and says so to a
/// screen reader too.
#[test]
fn copy_copies_all_of_it_and_says_so() {
    for look in [Look::macos(), Look::standard()] {
        let (mut harness, _, _) = ddl_view(look, Driver::Postgres);
        harness.click("Copy");
        assert_eq!(
            harness.copied.as_deref(),
            Some(book_covers(Driver::Postgres).text().as_str()),
            "{}",
            look.name
        );
        assert_eq!(said(&harness).as_deref(), Some("Copied DDL"));
        let tree = harness.settle();
        let toast = node(&tree, "Copied DDL", Role::Label).expect("the toast");
        let (_, toast) = tree.nodes.iter().find(|(id, _)| *id == toast).unwrap();
        assert_eq!(toast.live(), Some(egui::accesskit::Live::Polite));
    }
}

/// Copy copies only what is selected where something is: the button and
/// the chord alike.
#[test]
fn copy_copies_the_selection_where_there_is_one() {
    let (mut harness, tab, id) = ddl_view(Look::macos(), Driver::Postgres);
    focus_text(&mut harness, tab, id);
    harness.press(Key::ArrowDown, Modifiers::SHIFT);
    harness.press(Key::ArrowDown, Modifiers::SHIFT);
    harness.click("Copy");
    assert_eq!(
        harness.copied.as_deref(),
        Some(format!("{}\n", lines(0, 2)).as_str())
    );
    assert_eq!(said(&harness).as_deref(), Some("Copied 2 lines"));
    // ⇧⌘C, from the text: the same.
    harness.copied = None;
    focus_text(&mut harness, tab, id);
    harness.copy(true);
    assert_eq!(
        harness.copied.as_deref(),
        Some(format!("{}\n", lines(0, 2)).as_str())
    );
}

/// ⇧⌘C copies all of it with the keyboard anywhere in the view, and
/// Ctrl+Shift+C on Windows.
#[test]
fn the_chord_copies_all_of_it() {
    for look in [Look::macos(), Look::standard()] {
        let (mut harness, _, _) = ddl_view(look, Driver::Postgres);
        harness.copy(true);
        assert_eq!(
            harness.copied.as_deref(),
            Some(book_covers(Driver::Postgres).text().as_str()),
            "{}",
            look.name
        );
    }
}

/// The SQL editors open and in front.
fn sql_tabs(harness: &Harness, tab: ConnTabId) -> Vec<(String, RunMode)> {
    let workspace = harness.app.workspace(tab).unwrap();
    workspace
        .sql_tabs()
        .map(|sql| (sql.text.clone(), sql.mode))
        .collect()
}

/// Open in SQL editor puts the text in a new SQL editor, in front, which
/// runs nothing. It opens as Settings › Editor says.
#[test]
fn open_in_sql_editor_opens_a_new_tab_that_runs_nothing() {
    for writable in [false, true] {
        let (mut harness, tab, _) = ddl_view(Look::macos(), Driver::Postgres);
        harness.app.workspace_mut(tab).unwrap().access = match writable {
            true => tabletist_db::Access::Writable,
            false => tabletist_db::Access::ReadOnly,
        };
        harness.app.settings.new_tab = crate::settings::NewTab::Connection;
        let from = harness.app.backend.sent.len();
        harness.click("Open in SQL editor");
        let text = book_covers(Driver::Postgres).text();
        // On a connection that cannot write it is read-only, whatever the
        // settings say.
        let mode = if writable {
            RunMode::ReadWrite
        } else {
            RunMode::ReadOnly
        };
        assert_eq!(
            sql_tabs(&harness, tab),
            [(text, mode)],
            "writable {writable}"
        );
        let workspace = harness.app.workspace(tab).unwrap();
        assert!(workspace.active_sql_tab().is_some());
        assert!(
            !harness.app.backend.sent[from..]
                .iter()
                .any(|command| matches!(command, Command::RunSql { .. }))
        );
    }
}

/// On production the new editor is read-only, whatever the settings and
/// the connection say. ⌥↩ opens it too.
#[test]
fn on_production_the_editor_opens_read_only() {
    let (mut harness, tab, _) = ddl_view(Look::macos(), Driver::Postgres);
    let workspace = harness.app.workspace_mut(tab).unwrap();
    workspace.access = tabletist_db::Access::Writable;
    workspace.environment = crate::env::Environment::Production;
    harness.app.settings.new_tab = crate::settings::NewTab::Connection;
    harness.press(Key::Enter, Modifiers::ALT);
    let text = book_covers(Driver::Postgres).text();
    assert_eq!(sql_tabs(&harness, tab), [(text, RunMode::ReadOnly)]);
}

/// Production is named beside the table's name, in every view.
#[test]
fn production_is_named_beside_the_table() {
    let (mut harness, tab, id) = ddl_view(Look::macos(), Driver::Postgres);
    assert!(!harness.has("production"));
    harness.app.workspace_mut(tab).unwrap().environment = crate::env::Environment::Production;
    for shown in [ObjectView::Ddl, ObjectView::Data, ObjectView::Structure] {
        set_view(&mut harness, tab, id, shown);
        assert!(harness.has("production"), "{shown:?}");
    }
}

/// ⌃⌘1, ⌃⌘2 and ⌃⌘3 show the rows, the structure and the DDL, wherever
/// the keyboard is; ⌘ with a digit shows none.
#[test]
fn control_cmd_and_a_digit_show_a_view() {
    let (mut harness, tab, id) = covers(Look::macos(), Driver::Postgres);
    for (key, shown) in [
        (Key::Num3, ObjectView::Ddl),
        (Key::Num2, ObjectView::Structure),
        (Key::Num1, ObjectView::Data),
    ] {
        harness.press(key, CONTROL_CMD);
        assert_eq!(view(&harness, tab, id), shown, "{key:?}");
    }
    harness.press(Key::Num3, CMD);
    assert_eq!(
        view(&harness, tab, id),
        ObjectView::Data,
        "the command key alone with 3 is no view's"
    );
    // From the DDL view's text too.
    harness.press(Key::Num3, CONTROL_CMD);
    harness.answer_ddl(Ok(book_covers(Driver::Postgres)));
    focus_text(&mut harness, tab, id);
    harness.press(Key::Num2, CONTROL_CMD);
    assert_eq!(view(&harness, tab, id), ObjectView::Structure);
}

/// With two connections open, ⌃⌘2 shows the structure and stays in the
/// window; ⌘2 still goes to the second connection's.
#[test]
fn control_cmd_and_a_digit_is_no_connections_window() {
    let (mut harness, tab, id) = covers(Look::macos(), Driver::Postgres);
    harness.app.apply(Action::ShowConnections);
    let mut saved = crate::testing::fixture_connection();
    saved.id = crate::connections::ConnectionId::new();
    saved.name = "Other".into();
    let other = harness.connect_with(saved, crate::testing::fixture_objects());
    harness.app.apply(Action::ActivateConnection(0));
    harness.settle();
    assert_eq!(harness.app.active_tab_id(), tab);
    harness.press(Key::Num2, CONTROL_CMD);
    assert_eq!(harness.app.active_tab_id(), tab);
    assert_eq!(view(&harness, tab, id), ObjectView::Structure);
    harness.press(Key::Num2, CMD);
    assert_eq!(harness.app.active_tab_id(), other);
}

/// On Windows, where Ctrl with a digit goes to a connection's window, the
/// views' keys are Alt's.
#[test]
fn alt_and_a_digit_show_a_view_on_windows() {
    let (mut harness, tab, id) = covers(Look::standard(), Driver::Postgres);
    harness.press(Key::Num3, Modifiers::ALT);
    assert_eq!(view(&harness, tab, id), ObjectView::Ddl);
    harness.press(Key::Num2, Modifiers::ALT);
    assert_eq!(view(&harness, tab, id), ObjectView::Structure);
    harness.press(Key::Num3, Modifiers::COMMAND);
    assert_eq!(
        view(&harness, tab, id),
        ObjectView::Structure,
        "the command key with 3 goes to a window"
    );
}

/// The terminal's `r`, `s` and `S` show the rows, the structure and the
/// DDL, from any of them.
#[test]
fn r_s_and_shift_s_show_a_view_in_the_terminal() {
    let (mut harness, tab, id) = covers(Look::omarchy(), Driver::Postgres);
    harness.app.workspace_mut(tab).unwrap().pane = crate::model::Pane::Grid;
    letter(&mut harness, 'S');
    assert_eq!(view(&harness, tab, id), ObjectView::Ddl);
    harness.answer_ddl(Ok(book_covers(Driver::Postgres)));
    letter(&mut harness, 's');
    assert_eq!(view(&harness, tab, id), ObjectView::Structure);
    letter(&mut harness, 'r');
    assert_eq!(view(&harness, tab, id), ObjectView::Data);
    // The header names each with its key.
    for said in ["r rows", "s structure", "S ddl"] {
        assert!(
            harness.painted.iter().any(|(text, _)| text == said),
            "{said}"
        );
    }
}

/// `:ddl` shows the DDL view of the tab in front, or opens the table under
/// the tree's cursor in it; with neither it is no command there.
#[test]
fn ddl_opens_the_view_from_any_pane() {
    let (mut harness, tab, id) = covers(Look::omarchy(), Driver::Postgres);
    harness.app.workspace_mut(tab).unwrap().pane = crate::model::Pane::Grid;
    run_line(&mut harness, "ddl");
    assert_eq!(view(&harness, tab, id), ObjectView::Ddl);
    // From the tree, of the table under its cursor.
    let orders = ObjectRef::new("main", "orders");
    let workspace = harness.app.workspace_mut(tab).unwrap();
    workspace.pane = crate::model::Pane::Tree;
    workspace.tree.cursor = Some(crate::model::TreeNode::Object(
        orders.clone(),
        ObjectKind::Table,
    ));
    let from = harness.app.backend.sent.len();
    run_line(&mut harness, "ddl");
    let workspace = harness.app.workspace(tab).unwrap();
    let front = workspace.active_object_tab().unwrap();
    assert_eq!(
        (front.object.clone(), front.view),
        (orders.clone(), ObjectView::Ddl)
    );
    assert_eq!(reads(&harness, from), [orders]);
    // With no table or view to show (a SQL editor in front), nothing: no
    // tab opens or comes in front, and nothing is read.
    harness.app.apply(Action::NewSqlTab(tab));
    harness.app.workspace_mut(tab).unwrap().pane = crate::model::Pane::Grid;
    let (from, tabs) = (
        harness.app.backend.sent.len(),
        harness.app.workspace(tab).unwrap().tabs.len(),
    );
    harness.app.apply(Action::OpenDdl(tab));
    let workspace = harness.app.workspace(tab).unwrap();
    assert!(workspace.active_sql_tab().is_some());
    assert_eq!(workspace.tabs.len(), tabs);
    assert_eq!(workspace.command_error, None);
    assert!(reads(&harness, from).is_empty());
}

/// Runs `line` in the terminal's `:` prompt.
fn run_line(harness: &mut Harness, line: &str) {
    type_key(harness, Key::Colon, ":");
    type_text(harness, line);
    harness.press(Key::Enter, Modifiers::NONE);
}

/// The terminal's DDL view with the keys on its lines.
fn terminal_lines() -> (Harness, ConnTabId, TabId) {
    let (mut harness, tab, id) = ddl_view(Look::omarchy(), Driver::Postgres);
    harness.app.workspace_mut(tab).unwrap().pane = crate::model::Pane::Grid;
    harness.settle();
    (harness, tab, id)
}

/// `j` and `k` move the cursor by line, `gg` and `G` to the ends, `]]`
/// and `[[` to the next part and the one before.
#[test]
fn the_terminal_moves_through_the_text_by_line_and_by_part() {
    let (mut harness, tab, id) = terminal_lines();
    let at = |harness: &Harness| cursor(harness, tab, id).0;
    letter(&mut harness, 'j');
    letter(&mut harness, 'j');
    assert_eq!(at(&harness), 2);
    letter(&mut harness, 'k');
    assert_eq!(at(&harness), 1);
    harness.press(Key::ArrowDown, Modifiers::NONE);
    assert_eq!(at(&harness), 2);
    letter(&mut harness, 'G');
    assert_eq!(at(&harness), 11);
    letter(&mut harness, 'g');
    letter(&mut harness, 'g');
    assert_eq!(at(&harness), 0);
    // The parts start at lines 0 (the heading), 7 (`-- Indexes`) and 10
    // (`-- Comments`).
    for (keys, line) in [("]]", 7), ("]]", 10), ("]]", 10), ("[[", 7), ("[[", 0)] {
        for c in keys.chars() {
            letter(&mut harness, c);
        }
        assert_eq!(at(&harness), line, "{keys}");
    }
    // The cursor's line of all of them, at the line's end.
    letter(&mut harness, 'j');
    assert!(harness.painted.iter().any(|(text, _)| text == "2/12"));
}

/// `V` selects whole lines from the cursor's; `y` copies them and ends
/// the selection; `y` with none copies all of it. Esc ends it too.
#[test]
fn the_terminal_selects_lines_and_copies_them() {
    let (mut harness, tab, id) = terminal_lines();
    letter(&mut harness, 'j');
    letter(&mut harness, 'V');
    letter(&mut harness, 'j');
    assert_eq!(cursor(&harness, tab, id), (2, Some(1)));
    let painted =
        |harness: &Harness, said: &str| harness.painted.iter().any(|(text, _)| text == said);
    assert!(painted(&harness, "-- VISUAL LINE --"));
    assert!(painted(&harness, "2 lines selected · 3/12"));
    letter(&mut harness, 'y');
    assert_eq!(harness.copied.as_deref(), Some(lines(1, 3).as_str()));
    assert_eq!(said(&harness).as_deref(), Some("copied 2 lines"));
    assert_eq!(cursor(&harness, tab, id), (2, None));
    letter(&mut harness, 'y');
    assert_eq!(
        harness.copied.as_deref(),
        Some(book_covers(Driver::Postgres).text().as_str())
    );
    assert_eq!(said(&harness).as_deref(), Some("copied ddl"));
    letter(&mut harness, 'V');
    harness.press(Key::Escape, Modifiers::NONE);
    assert_eq!(cursor(&harness, tab, id).1, None);
    assert!(!painted(&harness, "-- VISUAL LINE --"));
}

/// `e` opens the text in a new SQL editor, as the button does.
#[test]
fn e_opens_the_text_in_a_sql_editor() {
    let (mut harness, tab, _) = terminal_lines();
    letter(&mut harness, 'e');
    let text = book_covers(Driver::Postgres).text();
    assert_eq!(sql_tabs(&harness, tab), [(text, RunMode::ReadOnly)]);
}

/// The line numbers painted down the left of `field`, top to bottom.
fn gutter(harness: &Harness, field: egui::Rect) -> Vec<usize> {
    let mut numbers: Vec<(usize, f32)> = harness
        .text_rects
        .iter()
        .filter(|(_, rect)| rect.left() < field.left() + 80.0 && rect.center().y > field.top())
        .filter_map(|(text, rect)| Some((text.parse().ok()?, rect.center().y)))
        .collect();
    numbers.sort_by(|one, other| one.1.total_cmp(&other.1));
    numbers.into_iter().map(|(number, _)| number).collect()
}

/// The macOS links name the parts that hold anything, with their counts.
/// One puts its part's first line at the top of a text too long for the
/// window, and is marked then; one whose part is too near the end to come
/// to the top is marked all the same, until the text is scrolled.
#[test]
fn a_link_takes_its_part_to_the_top() {
    let (mut harness, tab, id) = covers(Look::macos(), Driver::Postgres);
    set_view(&mut harness, tab, id, ObjectView::Ddl);
    let mut long = book_covers(Driver::Postgres);
    long.parts[1].statements = (0..80)
        .map(|n| format!("CREATE INDEX book_covers_{n} ON public.book_covers USING btree (id)"))
        .collect();
    long.parts[2].statements = (0..80)
        .map(|n| format!("COMMENT ON COLUMN public.book_covers.c{n} IS 'c'"))
        .collect();
    let placed = long.laid_out().1;
    harness.answer_ddl(Ok(long));
    let selected = |harness: &mut Harness, name: &str| {
        let tree = harness.settle();
        let link = node(&tree, name, Role::Link).unwrap_or_else(|| panic!("{name}"));
        let (_, link) = tree.nodes.iter().find(|(id, _)| *id == link).unwrap();
        link.is_selected() == Some(true)
    };
    let tree = harness.settle();
    let field = crate::testing::bounds(&tree, "DDL of book_covers", Role::MultilineTextInput)
        .expect("the text");
    assert!(selected(&mut harness, "Table"));
    assert!(!selected(&mut harness, "Comments 80"));
    assert!(
        !harness.has("Triggers"),
        "a part with nothing is not linked"
    );
    harness.click("Indexes 80");
    harness.settle();
    assert_eq!(
        gutter(&harness, field).first(),
        Some(&(placed[1].1.start + 1))
    );
    assert!(selected(&mut harness, "Indexes 80"));
    assert!(!selected(&mut harness, "Table"));
    // The comments end the text: their first line stays under the top.
    harness.click("Comments 80");
    harness.settle();
    assert!(gutter(&harness, field).contains(&(placed[2].1.start + 1)));
    assert!(selected(&mut harness, "Comments 80"));
    assert!(!selected(&mut harness, "Indexes 80"));
    harness.click("Table");
    harness.settle();
    assert_eq!(gutter(&harness, field).first(), Some(&1));
    assert!(selected(&mut harness, "Table"));
}

/// The links per engine: MySQL's table and its triggers, SQLite's table
/// and its indexes.
#[test]
fn each_engine_links_the_parts_it_has() {
    // MySQL's text here is given a trigger.
    let trigger = "CREATE TRIGGER book_covers_kind BEFORE INSERT ON `book_covers` \
                   FOR EACH ROW SET NEW.kind = LOWER(NEW.kind)";
    for (driver, triggers, links, gone) in [
        (
            Driver::MySql,
            &[trigger][..],
            &["Table", "Triggers 1"][..],
            "Indexes 1",
        ),
        (
            Driver::Sqlite,
            &[][..],
            &["Table", "Indexes 1"][..],
            "Comments 1",
        ),
    ] {
        let (mut harness, tab, id) = covers(Look::macos(), driver);
        set_view(&mut harness, tab, id, ObjectView::Ddl);
        let mut ddl = book_covers(driver);
        if !triggers.is_empty() {
            ddl.parts.push(tabletist_db::ddl::Part {
                kind: tabletist_db::ddl::PartKind::Triggers,
                statements: triggers.iter().map(|text| text.to_string()).collect(),
            });
        }
        harness.answer_ddl(Ok(ddl));
        harness.settle();
        for link in links {
            assert!(harness.has(link), "{driver:?}: {link}");
        }
        assert!(!harness.has(gone), "{driver:?}: {gone}");
    }
}

/// The terminal's outline lists the parts with their counts, the cursor's
/// marked, and takes the cursor to one clicked; PostgreSQL's says what is
/// not in the text.
#[test]
fn the_outline_lists_the_parts_and_moves_to_one() {
    let (mut harness, tab, id) = terminal_lines();
    for said in ["table 6", "indexes 1", "comments 1"] {
        assert!(harness.has(said), "{said}");
    }
    assert!(harness.has("not included"));
    let tree = harness.settle();
    let table = node(&tree, "table 6", Role::Button).unwrap();
    let (_, table) = tree.nodes.iter().find(|(id, _)| *id == table).unwrap();
    assert_eq!(table.is_selected(), Some(true));
    harness.click("comments 1");
    assert_eq!(cursor(&harness, tab, id).0, 10);
    // SQLite's text leaves nothing out.
    let (mut harness, tab, _) = ddl_view(Look::omarchy(), Driver::Sqlite);
    harness.app.workspace_mut(tab).unwrap().pane = crate::model::Pane::Grid;
    assert!(!harness.has("not included"));
}

/// A text read again that is shorter keeps the cursor on a line of it,
/// and drops what was selected.
#[test]
fn a_shorter_text_keeps_the_cursor_on_it() {
    let (mut harness, tab, id) = terminal_lines();
    letter(&mut harness, 'G');
    letter(&mut harness, 'V');
    harness.app.apply(Action::Refresh(tab));
    harness.answer_ddl(Ok(book_covers(Driver::MySql)));
    let lines = book_covers(Driver::MySql).text().lines().count();
    assert_eq!(cursor(&harness, tab, id), (lines - 1, None));
}

/// A Reload that failed shows its error, and the text it read before stays
/// held under it: nothing copies or opens that text, and no line of it is
/// selected. Nor while the first read is on its way, or after it failed.
#[test]
fn nothing_acts_on_a_text_the_view_does_not_show() {
    let refused = || tabletist_db::Error::query("permission denied for table book_covers");
    // The terminal look: `y`, `e`, `V`.
    let (mut harness, tab, id) = terminal_lines();
    harness.app.apply(Action::Refresh(tab));
    harness.answer_ddl(Err(refused()));
    harness.settle();
    harness.copied = None;
    for key in ['y', 'e', 'V'] {
        letter(&mut harness, key);
    }
    assert_eq!(harness.copied, None);
    assert!(sql_tabs(&harness, tab).is_empty());
    assert_eq!(cursor(&harness, tab, id).1, None);
    assert!(!harness.has("-- VISUAL LINE --"));
    assert!(!harness.has("copy"), "no key to copy is offered");
    // The looks with a pointer: the chords.
    let (mut harness, tab, _) = ddl_view(Look::macos(), Driver::Postgres);
    harness.app.apply(Action::Refresh(tab));
    harness.answer_ddl(Err(refused()));
    harness.settle();
    assert!(!harness.has("Copy"));
    harness.copied = None;
    harness.copy(true);
    harness.press(Key::Enter, Modifiers::ALT);
    assert_eq!(harness.copied, None);
    assert!(sql_tabs(&harness, tab).is_empty());
    // A first read on its way, then failed: `V` starts nothing.
    let (mut harness, tab, id) = covers(Look::omarchy(), Driver::Postgres);
    harness.app.workspace_mut(tab).unwrap().pane = crate::model::Pane::Grid;
    set_view(&mut harness, tab, id, ObjectView::Ddl);
    letter(&mut harness, 'V');
    assert_eq!(cursor(&harness, tab, id).1, None);
    harness.answer_ddl(Err(refused()));
    letter(&mut harness, 'V');
    assert_eq!(cursor(&harness, tab, id).1, None);
    assert!(!harness.has("-- VISUAL LINE --"));
}

/// The terminal: Esc ends a selection of lines, and with none it is what it
/// is in the other views: it closes Review SQL.
#[test]
fn esc_with_no_lines_selected_closes_review_sql() {
    let mut harness = Harness::new();
    harness.set_look(Look::omarchy());
    let (tab, id) = harness.editable();
    pending_change(&mut harness, tab, id);
    run_line(&mut harness, "diff");
    letter(&mut harness, 'S');
    harness.answer_ddl(Ok(book_covers(Driver::Postgres)));
    harness.settle();
    let reviewing = |harness: &Harness| {
        let workspace = harness.app.workspace(tab).unwrap();
        workspace.object_tab(id).unwrap().edits.reviewing
    };
    assert!(reviewing(&harness));
    letter(&mut harness, 'V');
    harness.press(Key::Escape, Modifiers::NONE);
    assert_eq!(cursor(&harness, tab, id).1, None);
    assert!(reviewing(&harness), "the first Esc ends the selection");
    harness.press(Key::Escape, Modifiers::NONE);
    assert!(!reviewing(&harness));
}

/// The terminal: with the row panel open on the rows, the DDL view's
/// brackets and Esc are its own, and the rows' cursor stays.
#[test]
fn with_the_row_panel_open_the_brackets_are_the_views() {
    let (mut harness, tab, id) = covers(Look::omarchy(), Driver::Postgres);
    harness.app.workspace_mut(tab).unwrap().pane = crate::model::Pane::Grid;
    harness.app.apply(Action::SelectCell {
        tab,
        id,
        cell: crate::model::CellPos { row: 1, col: 1 },
    });
    if !harness.app.workspace(tab).unwrap().row_panel {
        harness.app.apply(Action::ToggleRowPanel(tab));
    }
    harness.settle();
    letter(&mut harness, 'S');
    harness.answer_ddl(Ok(book_covers(Driver::Postgres)));
    harness.settle();
    let selection = |harness: &Harness| {
        let workspace = harness.app.workspace(tab).unwrap();
        workspace.object_tab(id).unwrap().selection
    };
    let before = selection(&harness);
    letter(&mut harness, ']');
    letter(&mut harness, ']');
    assert_eq!(cursor(&harness, tab, id).0, 7, "]] goes to the indexes");
    letter(&mut harness, '[');
    letter(&mut harness, '[');
    assert_eq!(cursor(&harness, tab, id).0, 0);
    assert_eq!(selection(&harness), before);
}

/// The terminal's line in the DDL view still says what is pending in the
/// table's rows, and offers to write it and to review it; it strikes out
/// none of the rows' keys.
#[test]
fn the_terminal_line_keeps_what_is_pending() {
    let mut harness = Harness::new();
    harness.set_look(Look::omarchy());
    let (tab, id) = harness.editable();
    pending_change(&mut harness, tab, id);
    assert!(harness.has("1 pending · 1 row"));
    letter(&mut harness, 'S');
    harness.answer_ddl(Ok(book_covers(Driver::Sqlite)));
    harness.settle();
    assert_eq!(view(&harness, tab, id), ObjectView::Ddl);
    let (save, review) = (
        crate::ui::keys::written(
            &harness.ctx,
            &Look::omarchy(),
            crate::keymap::Command::SaveChanges,
        ),
        crate::ui::keys::written(
            &harness.ctx,
            &Look::omarchy(),
            crate::keymap::Command::ReviewSql,
        ),
    );
    let lines = book_covers(Driver::Sqlite).text().lines().count();
    for said in [
        "1 pending · 1 row".to_string(),
        format!("{} write", &*save),
        format!("{} review", &*review),
        format!("1/{lines}"),
    ] {
        let painted = harness.painted.iter().any(|(text, _)| *text == said);
        assert!(painted, "{said}: {:?}", harness.painted);
    }
    assert!(
        !harness
            .painted
            .iter()
            .any(|(text, _)| text.ends_with(" new row"))
    );
}

/// The rows' `v y` copies a cell only where the rows show: not the one the
/// DDL view hides.
#[test]
fn the_rows_v_y_copies_no_cell_in_the_ddl_view() {
    let (mut harness, tab, id) = covers(Look::omarchy(), Driver::Postgres);
    harness.app.workspace_mut(tab).unwrap().pane = crate::model::Pane::Grid;
    harness.app.apply(Action::SelectCell {
        tab,
        id,
        cell: crate::model::CellPos { row: 1, col: 1 },
    });
    letter(&mut harness, 'S');
    harness.answer_ddl(Ok(book_covers(Driver::Postgres)));
    harness.settle();
    harness.copied = None;
    letter(&mut harness, 'v');
    letter(&mut harness, 'y');
    let text = book_covers(Driver::Postgres).text();
    assert!(
        harness
            .copied
            .as_deref()
            .is_none_or(|copied| copied == text),
        "{:?}",
        harness.copied
    );
}

/// The terminal: a click on a line puts the cursor on it, and the keys
/// move on from there.
#[test]
fn a_click_on_a_line_puts_the_cursor_there() {
    let (mut harness, tab, id) = terminal_lines();
    let tree = harness.settle();
    let lines = crate::testing::bounds(&tree, "DDL of book_covers", Role::MultilineTextInput)
        .expect("the lines");
    let at = lines.left_top() + egui::vec2(120.0, 10.0 + 28.0 * 3.5);
    let press = |pressed| egui::Event::PointerButton {
        pos: at,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: Modifiers::NONE,
    };
    harness.frame(vec![egui::Event::PointerMoved(at)]);
    harness.frame(vec![press(true)]);
    harness.frame(vec![press(false)]);
    harness.settle();
    let line = cursor(&harness, tab, id).0;
    assert!(line > 0, "the click put the cursor on no line");
    harness.press(Key::ArrowDown, Modifiers::NONE);
    letter(&mut harness, 'j');
    assert_eq!(cursor(&harness, tab, id).0, line + 2);
}

/// Leaving the DDL view ends its selection of lines.
#[test]
fn leaving_the_view_ends_the_selection_of_lines() {
    let (mut harness, tab, id) = terminal_lines();
    letter(&mut harness, 'V');
    letter(&mut harness, 'j');
    assert_eq!(cursor(&harness, tab, id), (1, Some(0)));
    letter(&mut harness, 'r');
    assert_eq!(view(&harness, tab, id), ObjectView::Data);
    letter(&mut harness, 'S');
    assert_eq!(cursor(&harness, tab, id), (1, None));
    assert!(!harness.has("-- VISUAL LINE --"));
}

/// A cell of the table changed and not written, with the keyboard on the
/// rows.
fn pending_change(harness: &mut Harness, tab: ConnTabId, id: TabId) {
    harness.app.workspace_mut(tab).unwrap().pane = crate::model::Pane::Grid;
    harness.app.apply(Action::SelectCell {
        tab,
        id,
        cell: crate::model::CellPos { row: 1, col: 1 },
    });
    harness.settle();
    for key in ['c', 'c'] {
        letter(harness, key);
    }
    harness.frame(vec![egui::Event::Text("zzz".into())]);
    harness.press(Key::Escape, Modifiers::NONE);
    harness.settle();
}

/// The header's Copy and Open in SQL editor give way as the window narrows:
/// their keys go, then their words (a screen reader still reads them), and
/// they never cover the view switch.
#[test]
fn the_header_buttons_give_way_in_a_narrow_window() {
    let (mut harness, _, _) = ddl_view(Look::macos(), Driver::Postgres);
    let keys = crate::ui::keys::written(
        &harness.ctx,
        &Look::macos(),
        crate::keymap::Command::CopyDdl,
    );
    let painted =
        |harness: &Harness, said: &str| harness.painted.iter().any(|(text, _)| text == said);
    harness.settle();
    assert!(painted(&harness, &keys));
    for width in [1000.0, 820.0, 720.0] {
        harness.size = egui::vec2(width, 600.0);
        let tree = harness.settle();
        let ddl = crate::testing::bounds(&tree, "DDL", Role::Tab).expect("the DDL tab");
        let copy = crate::testing::bounds(&tree, "Copy", Role::Button).expect("Copy");
        let open = crate::testing::bounds(&tree, "Open in SQL editor", Role::Button)
            .expect("Open in SQL editor");
        assert!(ddl.right() <= copy.left(), "{width}: {ddl:?} {copy:?}");
        assert!(copy.right() <= open.left(), "{width}");
    }
    assert!(!painted(&harness, &keys));
    assert!(!painted(&harness, "Copy"), "the words go");
}

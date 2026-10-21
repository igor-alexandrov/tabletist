//! Settings › Keyboard, headless: the list of the keymap's rows in each
//! look, recording a key, conflicts and the swap, the keys the system and
//! Tabletist keep, unbinding and resetting, the search, the file it
//! writes, and the cheat sheet. The brief's tests, by their numbers
//! (`docs/superpowers/specs/2026-10-20-settings-keyboard-design.md`).

use egui::accesskit::Role;
use egui::{Event, Key, Modifiers};

use crate::keymap::{Command, Layout, Row, Scope, When};
use crate::model::{Action, Dialog, SettingsPane};
use crate::testing::{Harness, labels};
use crate::theme::Look;

/// ⌥⌘ on macOS, as the window reports it.
const OPTION_COMMAND: Modifiers = Modifiers {
    alt: true,
    ctrl: false,
    shift: false,
    mac_cmd: true,
    command: true,
};

/// ⇧⌘ on macOS.
const SHIFT_COMMAND: Modifiers = Modifiers {
    alt: false,
    ctrl: false,
    shift: true,
    mac_cmd: true,
    command: true,
};

fn row(command: Command, scope: Scope) -> Row {
    Row {
        command,
        scope,
        when: When::Always,
    }
}

/// Settings › Keyboard open in `look`.
fn keyboard(look: Look) -> Harness {
    let mut harness = Harness::new();
    harness.set_look(look);
    open_keyboard(&mut harness);
    harness
}

fn open_keyboard(harness: &mut Harness) {
    harness.app.apply(Action::ShowSettings);
    harness
        .app
        .apply(Action::ShowSettingsPane(SettingsPane::Keyboard));
    harness.settle();
}

/// The names of the rows the list shows, in order.
fn rows(harness: &mut Harness) -> Vec<String> {
    let tree = harness.settle();
    tree.nodes
        .iter()
        .filter(|(_, node)| node.role() == Role::Row)
        .filter_map(|(_, node)| node.label().map(str::to_owned))
        .collect()
}

/// The text of the settings file the app holds.
fn file(harness: &Harness) -> &str {
    &harness.app.settings_file.text
}

/// What is being recorded, if anything.
fn recording(harness: &Harness) -> Option<crate::model::Recording> {
    match &harness.app.dialog {
        Some(Dialog::Settings(dialog)) => dialog.keyboard.recording.clone(),
        _ => None,
    }
}

/// The macOS caps' button of `name`, whose keys are spoken `keys`.
fn caps(name: &str, keys: &str) -> String {
    format!("Shortcut for {name}: {keys}. Press Return to record.")
}

/// Whether a node with `role` is named `label`.
fn has_role(harness: &mut Harness, label: &str, role: Role) -> bool {
    let tree = harness.settle();
    crate::testing::node(&tree, label, role).is_some()
}

/// Types `text` as the window sends it: the text, with no key.
fn type_text(harness: &mut Harness, text: &str) {
    harness.frame(vec![Event::Text(text.into())]);
    harness.settle();
}

// macOS.

/// Test 1: every command of the keymap the look has, with its defaults
/// from code, the recent ones among them, by group, in a table.
#[test]
fn the_pane_lists_every_row_of_the_look_with_its_defaults() {
    let mut harness = keyboard(Look::macos());
    let listed = rows(&mut harness);
    assert_eq!(listed.len(), harness.app.keymap.rows(Layout::Mac).len());
    for said in [
        "Find any object, Global, Command P",
        "History, Global, Command Y",
        // A command nothing answers yet keeps its keys, and says so.
        "Reopen closed tabs, Global, Shift Command T, not built yet",
        "New SQL editor tab, Global, Command T",
        "Format SQL or JSON, Editor, Shift Command F",
        "Explain analyze, Editor, Option Command E, not built yet",
        // No key, and one can be given.
        "Compare with previous run, Editor, not set, not built yet",
        "Switch database, Global, not set",
    ] {
        assert!(listed.iter().any(|row| row == said), "{said}: {listed:?}");
    }
    if crate::endpoint::AVAILABLE {
        assert!(
            listed
                .iter()
                .any(|row| row == "Show the Agents tab, Global, Option Command A")
        );
    }
    assert!(has_role(&mut harness, "Keyboard shortcuts", Role::Table));
    for group in [
        "Window and connections",
        "Navigation",
        "Grid and editing",
        "SQL editor",
        "AI assistant",
    ] {
        assert!(has_role(&mut harness, group, Role::RowGroup), "{group}");
    }
    assert!(harness.has(&caps("Format SQL or JSON", "Shift Command F")));
}

/// Test 2: a free chord is saved at once, written as one line of its
/// scope's table, and every reader of keys has it: the key runs, and quick
/// open writes it.
#[test]
fn a_free_chord_is_saved_and_every_reader_has_it_at_once() {
    let mut harness = Harness::new();
    harness.set_look(Look::macos());
    let tab = harness.connect_fake();
    open_keyboard(&mut harness);
    harness.click(&caps("Toggle sidebar", "Command B"));
    assert!(recording(&harness).is_some());
    assert!(harness.has("Recording. Press the new shortcut, Escape to cancel."));
    harness.press(Key::B, OPTION_COMMAND);
    assert!(recording(&harness).is_none());
    assert!(
        file(&harness).ends_with("[keys.global]\n\"cmd+alt+b\" = \"toggle_sidebar\"\n"),
        "{}",
        file(&harness)
    );
    let listed = rows(&mut harness);
    assert!(
        listed
            .iter()
            .any(|row| row == "Toggle sidebar, Global, Option Command B, changed")
    );
    // Closed, the new chord hides the sidebar, and the old one does not.
    harness.press(Key::Escape, Modifiers::NONE);
    assert!(harness.app.dialog.is_none());
    let hidden = |harness: &Harness| harness.app.workspace(tab).unwrap().sidebar_hidden;
    harness.press(Key::B, Modifiers::COMMAND);
    assert!(!hidden(&harness));
    harness.press(Key::B, OPTION_COMMAND);
    assert!(hidden(&harness));
    // Quick open writes the new chord beside the command.
    harness.press(Key::P, Modifiers::COMMAND);
    type_text(&mut harness, "sidebar");
    assert!(harness.painted_color("⌥⌘B").is_some());
    assert!(harness.painted_color("⌘B").is_none());
}

/// Test 3: ⌘N is Connections' and a table's: given to the SQL editor's
/// Format it stands. ⇧⌘↩ is Run all's in the same scope, and ⌘P the
/// global Find any object's: both are refused, and the recording goes on.
#[test]
fn the_same_keys_in_two_scopes_stand_and_in_one_or_global_conflict() {
    let mut harness = keyboard(Look::macos());
    let format = caps("Format SQL or JSON", "Shift Command F");
    harness.click(&format);
    harness.press(Key::N, Modifiers::COMMAND);
    assert!(file(&harness).contains("\"cmd+n\" = \"format\""));
    harness.click(&caps("Format SQL or JSON", "Command N"));
    harness.press(Key::Enter, SHIFT_COMMAND);
    assert!(harness.has("⇧⌘↩ is already “Run all”."));
    assert!(has_role(
        &mut harness,
        "⇧⌘↩ is already “Run all”.",
        Role::Alert
    ));
    assert!(recording(&harness).is_some());
    // Swap and Cancel take the keyboard, in that order.
    assert_eq!(super::tests::focused_name(&harness.settle()), "Swap");
    harness.press(Key::P, Modifiers::COMMAND);
    assert!(harness.has("⌘P is already “Find any object”."));
    // Nothing was written by either.
    assert!(file(&harness).contains("\"cmd+n\" = \"format\""));
    assert!(!file(&harness).contains("cmd+p"));
}

/// Test 4: Swap gives the key to the row recorded and the row that had it
/// this row's keys, and writes both lines; with none, the other row is
/// left with none. Cancel and Esc change nothing.
#[test]
fn swap_moves_the_keys_both_ways_and_cancel_and_esc_change_nothing() {
    let mut harness = keyboard(Look::macos());
    let before = file(&harness).to_owned();
    // ⌥⌘E is Explain analyze's (the brief's open question 1).
    harness.click(&caps("Compare with previous run", "not set"));
    harness.press(Key::E, OPTION_COMMAND);
    assert!(harness.has("⌥⌘E is already “Explain analyze”."));
    // Cancel: as it was.
    harness.click("Cancel · esc");
    assert!(recording(&harness).is_none());
    assert_eq!(file(&harness), before);
    // Esc: as it was.
    harness.click(&caps("Compare with previous run", "not set"));
    harness.press(Key::E, OPTION_COMMAND);
    harness.press(Key::Escape, Modifiers::NONE);
    assert!(recording(&harness).is_none());
    assert_eq!(file(&harness), before);
    assert!(matches!(harness.app.dialog, Some(Dialog::Settings(_))));
    // Swap.
    harness.click(&caps("Compare with previous run", "not set"));
    harness.press(Key::E, OPTION_COMMAND);
    harness.click("Swap");
    assert!(
        file(&harness).contains(
            "[keys.editor]\n\"cmd+alt+e\" = \"compare_previous_run\"  # after swap · was explain_analyze\n"
        ),
        "{}",
        file(&harness)
    );
    assert!(file(&harness).contains("[keys.unbind]\neditor = [\"cmd+alt+e\"]\n"));
    let listed = rows(&mut harness);
    assert!(
        listed
            .iter()
            .any(|row| row == "Explain analyze, Editor, not set, changed, not built yet")
    );
    assert!(
        listed.iter().any(|row| row
            == "Compare with previous run, Editor, Option Command E, changed, not built yet")
    );
    // Both ways: Format and Run all trade theirs.
    harness.click(&caps("Format SQL or JSON", "Shift Command F"));
    harness.press(Key::Enter, SHIFT_COMMAND);
    harness.click("Swap");
    let format = row(Command::Format, Scope::SqlEditor);
    let run_all = row(Command::RunAll, Scope::SqlEditor);
    assert_eq!(
        harness.app.keymap.keys_of(Layout::Mac, format),
        ["cmd+shift+enter"]
    );
    assert_eq!(
        harness.app.keymap.keys_of(Layout::Mac, run_all),
        ["cmd+shift+f"]
    );
    assert!(
        file(&harness).contains("\"cmd+shift+enter\" = \"format\"  # after swap · was run_all\n")
    );
    assert!(file(&harness).contains("\"cmd+shift+f\" = \"run_all\"\n"));
}

/// Test 5: the keys macOS keeps and Tabletist's own are refused with
/// whose they are, and the recording goes on.
#[test]
fn reserved_keys_are_refused_with_whose_they_are() {
    let mut harness = keyboard(Look::macos());
    harness.click(&caps("Format SQL or JSON", "Shift Command F"));
    harness.press(Key::Q, Modifiers::COMMAND);
    assert!(harness.has("⌘Q is used by macOS"));
    harness.press(Key::W, Modifiers::COMMAND);
    assert!(harness.has("⌘W is used by macOS"));
    // The window turns ⌘C into a copy before any key.
    harness.frame(vec![Event::Copy]);
    assert!(harness.has("⌘C is used by macOS"));
    harness.press(Key::Z, SHIFT_COMMAND);
    assert!(harness.has("⇧⌘Z is used by macOS"));
    // ⌘1 goes to the first connection window: a range that is fixed.
    harness.press(Key::Num1, Modifiers::COMMAND);
    assert!(harness.has("⌘1 is used by Tabletist"));
    assert!(recording(&harness).is_some());
    assert!(!file(&harness).contains("[keys"));
}

/// Test 6: a letter alone is refused; a function key alone is taken.
#[test]
fn a_letter_alone_is_refused_and_a_function_key_taken() {
    let mut harness = keyboard(Look::macos());
    harness.click(&caps("Format SQL or JSON", "Shift Command F"));
    harness.press(Key::F, Modifiers::NONE);
    assert!(harness.has("Add ⌘, ⌃ or ⌥"));
    harness.press(Key::F, Modifiers::SHIFT);
    assert!(harness.has("Add ⌘, ⌃ or ⌥"));
    // The keyboard went to Cancel: a key that is no Return, Space or Tab
    // is still recorded.
    harness.press(Key::F5, Modifiers::NONE);
    assert!(recording(&harness).is_none());
    assert!(file(&harness).contains("\"f5\" = \"format\""));
}

/// Test 7: Delete while recording unbinds: the default goes into
/// `[keys.unbind]`, and the row says it has none.
#[test]
fn delete_while_recording_unbinds_the_row() {
    let mut harness = keyboard(Look::macos());
    harness.click(&caps("Format SQL or JSON", "Shift Command F"));
    harness.press(Key::Backspace, Modifiers::NONE);
    assert!(recording(&harness).is_none());
    assert!(file(&harness).contains("[keys.unbind]\neditor = [\"cmd+shift+f\"]\n"));
    let listed = rows(&mut harness);
    assert!(
        listed
            .iter()
            .any(|row| row == "Format SQL or JSON, Editor, not set, changed")
    );
    assert!(harness.has(&caps("Format SQL or JSON", "not set")));
}

/// Test 8: a row's reset takes its lines away; Reset all… asks, then takes
/// every keys table away and nothing else.
#[test]
fn a_rows_reset_takes_its_lines_and_reset_all_every_table() {
    let mut harness = keyboard(Look::macos());
    harness
        .app
        .apply(Action::SetOption(crate::settings::OptionValue::PageSize(
            500,
        )));
    harness.click(&caps("Format SQL or JSON", "Shift Command F"));
    harness.press(Key::F5, Modifiers::NONE);
    harness.click(&caps("Run all", "Shift Command Return"));
    harness.press(Key::F6, OPTION_COMMAND);
    assert!(file(&harness).contains("format"));
    assert!(file(&harness).contains("run_all"));
    // A changed row's reset, which the pointer shows and the keyboard and
    // a screen reader reach.
    harness.click("Reset Format SQL or JSON to Shift Command F");
    assert!(!file(&harness).contains("format"));
    assert!(file(&harness).contains("run_all"));
    // Reset all asks, with the count.
    harness.click("Reset all…");
    assert!(harness.has("Reset all keyboard shortcuts? Your 1 change will be removed."));
    harness.click("Reset all");
    assert!(!file(&harness).contains("keys"));
    assert!(file(&harness).contains("page_size = 500"));
    assert!(
        harness
            .app
            .keymap
            .keys_of(Layout::Mac, row(Command::RunAll, Scope::SqlEditor))
            == ["cmd+shift+enter"]
    );
}

/// Test 9: the search lists the rows a name or a group names, a chord
/// pressed in it the rows it is a key of in every scope, and Changed the
/// changed rows alone, its count in its name.
#[test]
fn the_search_and_changed_cut_the_list() {
    let mut harness = keyboard(Look::macos());
    // No changes: Changed says so.
    assert!(harness.has("Changed, none"));
    harness.click("Changed, none");
    assert!(harness.has("No shortcuts changed"));
    harness.click("All");
    // By name.
    harness.app.apply(Action::KeyboardSearch("explain".into()));
    let mut listed = rows(&mut harness);
    listed.sort();
    assert_eq!(
        listed,
        [
            "Explain analyze, Editor, Option Command E, not built yet",
            "Explain, Editor, Command E, not built yet",
        ]
    );
    // By a group's name.
    harness
        .app
        .apply(Action::KeyboardSearch("ai assistant".into()));
    assert!(rows(&mut harness).iter().all(|row| row.contains("AI")));
    harness.app.apply(Action::KeyboardSearch(String::new()));
    // By a chord pressed in the field: ⌘N is two rows'.
    harness.ctx.memory_mut(|memory| {
        memory.request_focus(crate::ui::settings::keyboard_search_id());
    });
    harness.settle();
    harness.press(Key::N, Modifiers::COMMAND);
    assert!(harness.has("Commands bound to ⌘N"));
    let mut listed = rows(&mut harness);
    listed.sort();
    assert_eq!(
        listed,
        [
            "Add row, Table, Command N",
            "New connection, Connections, Command N"
        ]
    );
    harness.click("Show all");
    assert!(rows(&mut harness).len() > 2);
    // Changed lists the changed rows, and counts them.
    harness.click(&caps("Format SQL or JSON", "Shift Command F"));
    harness.press(Key::F5, Modifiers::NONE);
    harness.click("Changed, 1");
    assert_eq!(
        rows(&mut harness),
        ["Format SQL or JSON, Editor, F5, changed"]
    );
}

/// Footer: the link to the editor's vim keys opens Editor at Keybindings,
/// and Export cheat sheet saves Markdown, never the settings (test 13).
#[test]
fn the_footer_links_to_the_vim_keys_and_exports_the_cheat_sheet() {
    let mut harness = keyboard(Look::macos());
    let before = file(&harness).to_owned();
    harness.click("Export cheat sheet");
    let saves = harness.app.backend.saves.clone();
    assert_eq!(
        saves
            .last()
            .map(|(title, name, _)| (title.as_str(), name.as_str())),
        Some(("Export cheat sheet", "Tabletist shortcuts.md"))
    );
    assert_eq!(file(&harness), before);
    harness.click("Editor · Keybindings");
    match &harness.app.dialog {
        Some(Dialog::Settings(dialog)) => {
            assert_eq!(dialog.pane, SettingsPane::Editor);
            assert_eq!(dialog.asked, Some(crate::settings::OptionId::EditorKeys));
        }
        _ => panic!("Settings closed"),
    }
}

// Omarchy.

/// The Omarchy pane's cursor on `row`.
fn cursor_on(harness: &mut Harness, on: Row) {
    harness.app.apply(Action::KeyboardCursor(on));
    harness.settle();
}

/// Frames until a key of several is over its wait.
fn wait_out(harness: &mut Harness) {
    for _ in 0..60 {
        harness.frame(Vec::new());
    }
    harness.settle();
}

/// Test 1, Omarchy: the rows of the look, by its groups, with its keys.
#[test]
fn omarchy_lists_its_rows_by_group_with_its_keys() {
    let mut harness = keyboard(Look::omarchy());
    let listed = rows(&mut harness);
    assert_eq!(listed.len(), harness.app.keymap.rows(Layout::Omarchy).len());
    for said in [
        "find any object, global, ctrl+p",
        "history, global, :history",
        "new sql editor tab, global, ctrl+t",
        "reopen closed tabs, start, u, not built yet",
        "delete row, grid, dd",
        "save changes, grid, :w or ctrl+s",
    ] {
        assert!(listed.iter().any(|row| row == said), "{said}: {listed:?}");
    }
    for group in ["window", "navigate", "edit", "sql"] {
        assert!(has_role(&mut harness, group, Role::RowGroup), "{group}");
    }
    assert!(harness.has("super is never bound: hyprland owns it · insert-mode keys follow vim"));
}

/// Test 11: a key of several records, ends after 800 ms, and runs; one
/// ended by Enter is taken at once; a line of the prompt records too.
#[test]
fn omarchy_records_keys_of_several_and_lines_and_they_run() {
    let mut harness = Harness::new();
    harness.set_look(Look::omarchy());
    let (tab, id) = harness.book_covers();
    open_keyboard(&mut harness);
    let add = row(Command::AddRow, Scope::Grid);
    cursor_on(&mut harness, add);
    harness.press(Key::Enter, Modifiers::NONE);
    assert!(recording(&harness).is_some());
    type_text(&mut harness, "g");
    type_text(&mut harness, "n");
    assert_eq!(
        recording(&harness).map(|recording| recording.typed),
        Some("gn".into())
    );
    // Nothing yet: the key may go on.
    assert!(!file(&harness).contains("[keys"));
    wait_out(&mut harness);
    assert!(recording(&harness).is_none());
    assert!(
        file(&harness).contains("[keys.grid]\n\"gn\" = \"add_row\"\n"),
        "{}",
        file(&harness)
    );
    // A line of the prompt, ended by Enter.
    let save = row(Command::SaveChanges, Scope::Grid);
    cursor_on(&mut harness, save);
    harness.press(Key::Enter, Modifiers::NONE);
    type_text(&mut harness, ":wr");
    harness.press(Key::Enter, Modifiers::NONE);
    assert!(
        file(&harness).contains("\":wr\" = \"save_changes\""),
        "{}",
        file(&harness)
    );
    assert_eq!(
        harness.app.keymap.keys_of(Layout::Omarchy, save),
        [":wr", "ctrl+s"]
    );
    // A key of several, ended by Enter at once.
    let null = row(Command::SetNull, Scope::Grid);
    cursor_on(&mut harness, null);
    harness.press(Key::Enter, Modifiers::NONE);
    type_text(&mut harness, "gz");
    harness.press(Key::Enter, Modifiers::NONE);
    assert!(file(&harness).contains("\"gz\" = \"set_null\""));
    // They run: on the rows, `gn` adds a row, and `o` no longer does.
    harness.app.apply(Action::CloseDialog);
    harness.settle();
    let added = |harness: &Harness| {
        harness
            .app
            .workspace(tab)
            .unwrap()
            .object_tab(id)
            .unwrap()
            .edits
            .added
            .len()
    };
    harness.press(Key::O, Modifiers::NONE);
    assert_eq!(added(&harness), 0);
    harness.press(Key::G, Modifiers::NONE);
    harness.press(Key::N, Modifiers::NONE);
    assert_eq!(added(&harness), 1);
}

/// Test 5, Omarchy: Super, a colon alone and a key of the wrong kind are
/// refused with why, and Esc gives up.
#[test]
fn omarchy_refuses_super_a_colon_and_another_kind() {
    let mut harness = keyboard(Look::omarchy());
    cursor_on(&mut harness, row(Command::DeleteRow, Scope::Grid));
    harness.press(Key::Enter, Modifiers::NONE);
    let super_key = Modifiers {
        mac_cmd: true,
        ..Modifiers::NONE
    };
    harness.press(Key::T, super_key);
    assert!(harness.has("super is never bound: hyprland owns it"));
    type_text(&mut harness, ":");
    harness.press(Key::Enter, Modifiers::NONE);
    assert!(harness.has(": opens the prompt and is never bound"));
    // Delete row is typed keys: a chord is of another kind.
    harness.press(Key::D, Modifiers::CTRL);
    assert!(harness.has("delete row takes typed keys"));
    // A count's digit first.
    type_text(&mut harness, "3x");
    harness.press(Key::Enter, Modifiers::NONE);
    assert!(harness.has("a digit first is a count"));
    harness.press(Key::Escape, Modifiers::NONE);
    assert!(recording(&harness).is_none());
    assert!(!file(&harness).contains("[keys"));
}

/// Test 4, Omarchy: a key another row has offers the swap, and `s` makes
/// it: the board's own case, Delete row given `D`.
#[test]
fn omarchy_swaps_with_its_letter() {
    let mut harness = keyboard(Look::omarchy());
    let delete = row(Command::DeleteRow, Scope::Grid);
    cursor_on(&mut harness, delete);
    harness.press(Key::Enter, Modifiers::NONE);
    type_text(&mut harness, "D");
    wait_out(&mut harness);
    let said = "D is set default";
    assert!(
        has_role(&mut harness, said, Role::Alert),
        "{:?}",
        labels(&harness.settle())
    );
    assert!(harness.has("[s] swap  [esc] cancel"));
    type_text(&mut harness, "s");
    assert!(recording(&harness).is_none());
    assert!(
        file(&harness).contains(
            "[keys.grid]\n\"D\" = \"delete_row\"  # after swap · was set_default\n\"dd\" = \"set_default\"\n"
        ),
        "{}",
        file(&harness)
    );
    assert_eq!(harness.app.keymap.keys_of(Layout::Omarchy, delete), ["D"]);
}

/// Tests 7 and 8, Omarchy: `dd` unbinds the row, `R` resets it.
#[test]
fn omarchy_unbinds_and_resets_with_its_keys() {
    let mut harness = keyboard(Look::omarchy());
    let follow = row(Command::OpenReferencedRow, Scope::Grid);
    cursor_on(&mut harness, follow);
    harness.press(Key::D, Modifiers::NONE);
    harness.press(Key::D, Modifiers::NONE);
    assert!(
        file(&harness).contains("[keys.unbind]\ngrid = [\"gd\"]\n"),
        "{}",
        file(&harness)
    );
    assert!(
        rows(&mut harness)
            .iter()
            .any(|row| row == "open referenced row, grid, not set, changed")
    );
    harness.press(Key::R, Modifiers::SHIFT);
    assert!(!file(&harness).contains("keys"));
}

/// Test 9, Omarchy: `/` filters by name, `?` then keys finds by a key.
#[test]
fn omarchy_filters_with_slash_and_finds_with_question_mark() {
    let mut harness = keyboard(Look::omarchy());
    type_text(&mut harness, "/");
    type_text(&mut harness, "referenced");
    assert_eq!(rows(&mut harness), ["open referenced row, grid, gd"]);
    harness.press(Key::Enter, Modifiers::NONE);
    harness.app.apply(Action::KeyboardSearch(String::new()));
    harness.settle();
    type_text(&mut harness, "?");
    harness.press(Key::T, Modifiers::CTRL);
    assert!(harness.has("keys ctrl+t · esc shows every command"));
    assert_eq!(rows(&mut harness), ["new sql editor tab, global, ctrl+t"]);
    harness.press(Key::Escape, Modifiers::NONE);
    assert!(rows(&mut harness).len() > 1);
    assert!(matches!(harness.app.dialog, Some(Dialog::Settings(_))));
}

/// Test 12: `ctrl+e` opens the file from the pane. In a SQL editor the
/// same chord is Explain's (claimed: nothing answers it yet), in another
/// scope than the screen's.
#[test]
fn omarchy_ctrl_e_opens_the_file_and_stays_the_sql_editors() {
    let mut harness = keyboard(Look::omarchy());
    harness.press(Key::E, Modifiers::CTRL);
    assert!(matches!(
        crate::testing::last_sent(&harness.app),
        crate::backend::Command::EditSettingsFile { .. }
    ));
    let keymap = &harness.app.keymap;
    let sql: Vec<&str> = keymap
        .of(Command::Explain)
        .flat_map(|binding| binding.omarchy.chords.iter().map(|chord| chord.keys))
        .collect();
    assert_eq!(sql, ["ctrl+e"]);
    assert!(!Scope::SqlEditor.alongside(Scope::Prompt));
}

/// Test 13: `:keys default` opens the full list in the editor, and
/// `:keys cheatsheet` saves the sheet: neither changes the settings.
#[test]
fn the_keys_lines_never_change_the_settings() {
    let mut harness = Harness::new();
    harness.set_look(Look::omarchy());
    let (tab, _) = harness.book_covers();
    let before = file(&harness).to_owned();
    harness.app.apply(Action::OpenCommand(tab));
    harness.app.workspace_mut(tab).unwrap().command = Some("keys default".into());
    harness.app.apply(Action::RunCommand(tab));
    match crate::testing::last_sent(&harness.app) {
        crate::backend::Command::OpenKeysList { path, text } => {
            assert!(path.ends_with("keys.default.toml"));
            assert_ne!(*path, harness.app.dirs.settings_file());
            assert!(text.contains("# [keys.grid]\n"));
            assert!(text.contains("# \"dd\" = \"delete_row\"\n"));
            assert!(text.contains("# \"ctrl+k\" = \"ask_ai\"  # not built yet\n"));
        }
        other => panic!("{other:?}"),
    }
    harness.app.apply(Action::OpenCommand(tab));
    harness.app.workspace_mut(tab).unwrap().command = Some("keys cheatsheet".into());
    harness.app.apply(Action::RunCommand(tab));
    let saves = harness.app.backend.saves.clone();
    assert_eq!(
        saves.last().map(|(_, name, _)| name.as_str()),
        Some("Tabletist shortcuts.md")
    );
    assert_eq!(file(&harness), before);
}

/// The cheat sheet: a table for each group, the keys as the look writes
/// them, a changed row marked so, and one nothing answers yet.
#[test]
fn the_cheat_sheet_lists_the_groups_with_their_keys_as_the_look_writes_them() {
    let mut harness = keyboard(Look::macos());
    harness.click(&caps("Format SQL or JSON", "Shift Command F"));
    harness.press(Key::F5, Modifiers::NONE);
    let sheet = crate::app::cheat_sheet(&harness.app.keymap, Layout::Mac);
    assert!(sheet.starts_with("# Tabletist shortcuts\n"));
    assert!(
        sheet.contains("\n## SQL editor\n\n| Command | Scope | Shortcut |\n| --- | --- | --- |\n")
    );
    assert!(sheet.contains("| Format SQL or JSON | Editor | F5 (changed) |\n"));
    assert!(sheet.contains("| Find any object | Global | ⌘P |\n"));
    assert!(sheet.contains("| Compare with previous run (not built yet) | Editor | Not set |\n"));
    assert!(sheet.contains("| Ask AI (not built yet) | Editor | ⌘K |\n"));
    let omarchy = crate::app::cheat_sheet(&crate::keymap::Keymap::default(), Layout::Omarchy);
    assert!(omarchy.contains("| Delete row | grid | dd |\n"));
    assert!(omarchy.contains("\n## edit\n"));
}

/// Another pane shown ends a recording: coming back finds none.
#[test]
fn another_pane_ends_a_recording() {
    let mut harness = keyboard(Look::macos());
    harness.click(&caps("Format SQL or JSON", "Shift Command F"));
    assert!(recording(&harness).is_some());
    harness.click("General");
    harness.click("Keyboard");
    assert!(recording(&harness).is_none());
    // And the keys pressed meanwhile were nobody's recording.
    harness.press(Key::F5, Modifiers::NONE);
    assert!(!file(&harness).contains("[keys"));
}

/// The reset button takes a pointer's click: the caps' button stops short
/// of it.
#[test]
fn a_click_on_the_reset_button_resets() {
    let mut harness = keyboard(Look::macos());
    harness.click(&caps("Format SQL or JSON", "Shift Command F"));
    harness.press(Key::F5, Modifiers::NONE);
    harness.app.apply(Action::KeyboardSearch("format".into()));
    let tree = harness.settle();
    let place = crate::testing::bounds(
        &tree,
        "Reset Format SQL or JSON to Shift Command F",
        Role::Button,
    )
    .expect("the reset button");
    let at = place.center();
    harness.frame(vec![Event::PointerMoved(at)]);
    let button = |pressed| Event::PointerButton {
        pos: at,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: Modifiers::NONE,
    };
    harness.frame(vec![button(true)]);
    harness.frame(vec![button(false)]);
    harness.settle();
    assert!(recording(&harness).is_none());
    assert!(!file(&harness).contains("format"));
}

/// No Swap is offered where the exchange would not stand.
#[test]
fn a_swap_that_would_not_stand_is_not_offered() {
    let mut harness = keyboard(Look::macos());
    harness.app.apply(Action::KeyboardSearch("add row".into()));
    harness.click(&caps("Add row", "Command N"));
    harness.press(Key::B, Modifiers::COMMAND);
    assert!(harness.has("⌘B is already “Toggle sidebar”."));
    assert!(!harness.has("Swap"));
    assert!(harness.has("Cancel · esc"));
}

/// A chord pressed in the search finds a range it is one of.
#[test]
fn the_search_finds_a_range_of_digits_by_one_of_them() {
    let mut harness = keyboard(Look::macos());
    harness.ctx.memory_mut(|memory| {
        memory.request_focus(crate::ui::settings::keyboard_search_id());
    });
    harness.settle();
    harness.press(Key::Num3, Modifiers::COMMAND);
    assert!(harness.has("Commands bound to ⌘3"));
    assert_eq!(
        rows(&mut harness),
        ["Go to connection window, Global, ⌘1…9"]
    );
}

/// A line of the prompt keeps its spaces.
#[test]
fn omarchy_records_a_line_with_words() {
    let mut harness = keyboard(Look::omarchy());
    cursor_on(&mut harness, row(Command::KeysDefault, Scope::Global));
    harness.press(Key::Enter, Modifiers::NONE);
    type_text(&mut harness, ":keys");
    type_text(&mut harness, " ");
    type_text(&mut harness, "all");
    harness.press(Key::Enter, Modifiers::NONE);
    assert!(
        file(&harness).contains("[keys.window]\n\":keys all\" = \"keys_default\"\n"),
        "{}",
        file(&harness)
    );
}

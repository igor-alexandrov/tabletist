//! Settings › Editor's Keybindings: vim's normal, insert and visual modes
//! in the SQL editor, the mode in the status line, and the rows that set
//! them.

use egui::{Key, Modifiers};

use crate::model::{Action, ConnTabId, RunMode};
use crate::settings::{EditorKeys, OptionValue};
use crate::testing::Harness;
use crate::theme::Look;
use crate::vim::Mode;

/// A connected fixture in `look`, with a new SQL editor that has the
/// keyboard and holds `text`, its cursor at the end. A new tab types: in
/// vim it is in insert mode.
fn editor(look: Look, text: &str) -> (Harness, ConnTabId) {
    let mut harness = Harness::new();
    harness.set_look(look);
    let tab = harness.connect_fake();
    harness.press(Key::T, Modifiers::COMMAND);
    if !text.is_empty() {
        harness.frame(vec![egui::Event::Paste(text.into())]);
    }
    harness.settle();
    (harness, tab)
}

fn text(harness: &Harness, tab: ConnTabId) -> String {
    let workspace = harness.app.workspace(tab).unwrap();
    workspace.active_sql_tab().unwrap().text.clone()
}

fn cursor(harness: &Harness, tab: ConnTabId) -> usize {
    let workspace = harness.app.workspace(tab).unwrap();
    workspace.active_sql_tab().unwrap().cursor
}

fn mode(harness: &Harness, tab: ConnTabId) -> Mode {
    let workspace = harness.app.workspace(tab).unwrap();
    workspace.active_sql_tab().unwrap().vim.mode
}

fn selected(harness: &Harness, tab: ConnTabId) -> Option<std::ops::Range<usize>> {
    let workspace = harness.app.workspace(tab).unwrap();
    workspace.active_sql_tab().unwrap().selected.clone()
}

/// Types `keys` as the window sends them: for each character the press of
/// its key, where it has one, with the character.
fn keys(harness: &mut Harness, keys: &str) {
    for c in keys.chars() {
        let key = Key::from_name(&c.to_ascii_uppercase().to_string());
        let shift = if c.is_uppercase() {
            Modifiers::SHIFT
        } else {
            Modifiers::NONE
        };
        let mut events = Vec::new();
        if let Some(key) = key {
            events.push(crate::testing::key(key, shift));
        }
        events.push(egui::Event::Text(c.to_string()));
        if let Some(key) = key {
            events.push(crate::testing::release(key, shift));
        }
        harness.frame(events);
        harness.settle();
    }
}

fn escape(harness: &mut Harness) {
    harness.press(Key::Escape, Modifiers::NONE);
}

fn set(harness: &mut Harness, value: OptionValue) {
    harness.app.apply(Action::SetOption(value));
    harness.settle();
}

#[test]
fn a_new_tab_types_and_esc_goes_to_normal_mode_then_out_of_the_editor() {
    let (mut harness, tab) = editor(Look::omarchy(), "");
    keys(&mut harness, "select 1");
    assert_eq!(text(&harness, tab), "select 1");
    assert_eq!(mode(&harness, tab), Mode::Insert);
    // The line starts with the mode, and Esc is normal mode's way.
    assert!(harness.painted_rect("-- INSERT --").is_some());
    assert!(harness.painted_rect("esc normal").is_some());
    escape(&mut harness);
    assert_eq!(mode(&harness, tab), Mode::Normal);
    assert!(harness.ctx.text_edit_focused(), "the editor keeps the keys");
    assert_eq!(cursor(&harness, tab), 7, "back on the last character");
    assert!(harness.painted_rect("-- INSERT --").is_none());
    assert!(harness.painted_rect("esc leave editor").is_some());
    // Letters are commands now, not text.
    keys(&mut harness, "x");
    assert_eq!(text(&harness, tab), "select ");
    escape(&mut harness);
    assert!(!harness.ctx.text_edit_focused());
}

#[test]
fn normal_mode_edits_in_undo_steps_and_puts_back_what_it_took() {
    let (mut harness, tab) = editor(Look::omarchy(), "select 1;\nselect 2;");
    escape(&mut harness);
    keys(&mut harness, "ggdd");
    assert_eq!(text(&harness, tab), "select 2;");
    keys(&mut harness, "p");
    assert_eq!(text(&harness, tab), "select 2;\nselect 1;");
    keys(&mut harness, "u");
    assert_eq!(text(&harness, tab), "select 2;");
    keys(&mut harness, "u");
    assert_eq!(text(&harness, tab), "select 1;\nselect 2;");
    harness.press(Key::R, Modifiers::CTRL);
    assert_eq!(text(&harness, tab), "select 2;");
    // A motion with a count, then insert mode where `cw` leaves it.
    keys(&mut harness, "wcw3");
    escape(&mut harness);
    assert_eq!(text(&harness, tab), "select 3;");
    assert_eq!(mode(&harness, tab), Mode::Normal);
}

#[test]
fn ctrl_w_in_normal_mode_deletes_nothing_and_closes_no_tab() {
    let (mut harness, tab) = editor(Look::omarchy(), "select 1");
    escape(&mut harness);
    let tabs = |harness: &Harness| harness.app.workspace(tab).unwrap().tabs.len();
    let open = tabs(&harness);
    harness.press(Key::W, Modifiers::CTRL);
    assert_eq!(text(&harness, tab), "select 1");
    assert_eq!(tabs(&harness), open);
}

#[test]
fn the_field_s_deleting_chords_delete_nothing_in_normal_mode() {
    let (mut harness, tab) = editor(Look::macos(), "select title");
    set(&mut harness, OptionValue::EditorKeys(Some(EditorKeys::Vim)));
    keys(&mut harness, "0w");
    for (key, modifiers) in [
        (Key::Backspace, Modifiers::COMMAND),
        (Key::Backspace, Modifiers::ALT),
        (Key::Delete, Modifiers::ALT),
        (Key::K, Modifiers::CTRL),
    ] {
        harness.press(key, modifiers);
        assert_eq!(text(&harness, tab), "select title", "{key:?} {modifiers:?}");
    } // Page Down is the field's: it moves, and types nothing.
    harness.press(Key::PageDown, Modifiers::NONE);
    assert_eq!(text(&harness, tab), "select title");
}

#[test]
fn a_visual_selection_is_what_runs() {
    let (mut harness, tab) = editor(Look::omarchy(), "select 1;\nselect 2;");
    escape(&mut harness);
    keys(&mut harness, "ggV");
    assert_eq!(mode(&harness, tab), Mode::VisualLine);
    assert_eq!(selected(&harness, tab), Some(0..9));
    assert!(harness.painted_rect("-- VISUAL LINE --").is_some());
    assert!(harness.painted_rect("esc normal").is_some());
    keys(&mut harness, "j");
    assert_eq!(selected(&harness, tab), Some(0..19));
    keys(&mut harness, "k");
    harness.press(Key::Enter, Modifiers::CTRL);
    let sent = harness
        .app
        .backend
        .sent
        .iter()
        .rev()
        .find_map(|command| match command {
            crate::backend::Command::RunSql { statements, .. } => Some(
                statements
                    .iter()
                    .map(|statement| statement.text.clone())
                    .collect::<Vec<_>>(),
            ),
            _ => None,
        })
        .expect("a run");
    assert_eq!(sent, ["select 1"]);
    // By character: both ends in.
    escape(&mut harness);
    assert_eq!(mode(&harness, tab), Mode::Normal);
    assert_eq!(selected(&harness, tab), None);
    keys(&mut harness, "0ve");
    assert_eq!(selected(&harness, tab), Some(0..6));
    assert_eq!(
        cursor(&harness, tab),
        5,
        "on the selection's last character"
    );
    keys(&mut harness, "d");
    assert_eq!(text(&harness, tab), " 1;\nselect 2;");
}

#[test]
fn equals_formats_the_cursor_s_statement_and_gg_equals_g_the_tab() {
    let script = "select 1 from t;\nselect 2 from u;";
    let (mut harness, tab) = editor(Look::omarchy(), script);
    escape(&mut harness);
    keys(&mut harness, "gg==");
    assert_eq!(text(&harness, tab), "select 1\n  from t;\nselect 2 from u;");
    assert!(harness.ctx.text_edit_focused());
    assert_eq!(mode(&harness, tab), Mode::Normal);
    assert_eq!(
        selected(&harness, tab),
        None,
        "the lines are not left selected"
    );
    // One undo step, as Format's, and the cursor back, not the lines.
    keys(&mut harness, "u");
    assert_eq!(text(&harness, tab), script);
    assert_eq!(selected(&harness, tab), None);
    // A statement Format leaves as it is: nothing selected either.
    keys(&mut harness, "gg==");
    keys(&mut harness, "==");
    assert_eq!(selected(&harness, tab), None);
    keys(&mut harness, "gg=G");
    assert_eq!(
        text(&harness, tab),
        "select 1\n  from t;\n\nselect 2\n  from u;"
    );
}

#[test]
fn colon_opens_the_prompt_which_gives_the_keyboard_back() {
    let (mut harness, tab) = editor(Look::omarchy(), "select 1");
    escape(&mut harness);
    let sql = harness.app.workspace(tab).unwrap().active_tab.unwrap();
    keys(&mut harness, ":");
    assert_eq!(
        harness.app.workspace(tab).unwrap().command.as_deref(),
        Some("")
    );
    assert_eq!(text(&harness, tab), "select 1", "the colon is not typed");
    harness.frame(vec![egui::Event::Text("ro".into())]);
    harness.press(Key::Enter, Modifiers::NONE);
    let workspace = harness.app.workspace(tab).unwrap();
    assert_eq!(workspace.sql_tab(sql).unwrap().mode, RunMode::ReadOnly);
    assert!(harness.ctx.text_edit_focused(), "back in the editor");
    assert_eq!(mode(&harness, tab), Mode::Normal);
    // `:w` writes nothing and says nothing.
    keys(&mut harness, ":");
    harness.frame(vec![egui::Event::Text("w".into())]);
    harness.press(Key::Enter, Modifiers::NONE);
    let workspace = harness.app.workspace(tab).unwrap();
    assert_eq!(workspace.command_error, None);
    assert!(harness.ctx.text_edit_focused());
    assert_eq!(text(&harness, tab), "select 1");
    // Esc in the prompt gives it back too; the prompt losing the keyboard
    // to anything else does not.
    keys(&mut harness, ":");
    escape(&mut harness);
    assert!(harness.ctx.text_edit_focused());
    keys(&mut harness, ":");
    harness.app.apply(Action::CloseCommand(tab));
    harness.settle();
    assert!(!harness.ctx.text_edit_focused());
    // Esc out and keys in one frame: the keys are nobody's.
    harness.leave_editor();
    let workspace = harness.app.workspace_mut(tab).unwrap();
    workspace.sql_tab_mut(sql).unwrap().focus_editor = true;
    harness.settle();
    let mut events = vec![
        crate::testing::key(Key::Escape, Modifiers::NONE),
        crate::testing::release(Key::Escape, Modifiers::NONE),
    ];
    events.extend(typed('d'));
    events.extend(typed('d'));
    harness.frame(events);
    harness.settle();
    assert!(!harness.ctx.text_edit_focused());
    assert_eq!(text(&harness, tab), "select 1");
}

#[test]
fn an_open_completion_list_takes_esc_and_insert_mode_stays() {
    let (mut harness, tab) = editor(Look::omarchy(), "");
    keys(&mut harness, "sel");
    let open = |harness: &Harness| {
        let workspace = harness.app.workspace(tab).unwrap();
        workspace.active_sql_tab().unwrap().completion.is_some()
    };
    assert!(open(&harness), "the list for `sel`");
    escape(&mut harness);
    assert!(!open(&harness));
    assert_eq!(mode(&harness, tab), Mode::Insert);
    escape(&mut harness);
    assert_eq!(mode(&harness, tab), Mode::Normal);
}

#[test]
fn switching_the_keybindings_applies_at_once_and_keeps_the_keyboard_where_it_is() {
    let (mut harness, tab) = editor(Look::omarchy(), "select 1");
    assert_eq!(mode(&harness, tab), Mode::Insert);
    // The standard keys: Esc leaves at once, as before vim.
    set(
        &mut harness,
        OptionValue::EditorKeys(Some(EditorKeys::Standard)),
    );
    assert!(harness.ctx.text_edit_focused());
    assert!(harness.painted_rect("-- INSERT --").is_none());
    assert_eq!(mode(&harness, tab), Mode::Normal, "out of insert mode");
    escape(&mut harness);
    assert!(!harness.ctx.text_edit_focused());
    // Back to vim: in normal mode, the keyboard where it was put.
    let workspace = harness.app.workspace_mut(tab).unwrap();
    let id = workspace.active_tab.unwrap();
    workspace.sql_tab_mut(id).unwrap().focus_editor = true;
    harness.settle();
    set(&mut harness, OptionValue::EditorKeys(Some(EditorKeys::Vim)));
    assert!(harness.ctx.text_edit_focused());
    assert_eq!(mode(&harness, tab), Mode::Normal);
    keys(&mut harness, "x");
    assert_eq!(text(&harness, tab), "select ");
}

#[test]
fn the_macos_look_has_vim_too_with_its_mode_in_the_footer_and_a_block_cursor() {
    let (mut harness, tab) = editor(Look::macos(), "SELECT 1");
    // Standard keys: Esc leaves at once.
    assert!(harness.painted_rect("-- INSERT --").is_none());
    set(&mut harness, OptionValue::EditorKeys(Some(EditorKeys::Vim)));
    assert_eq!(mode(&harness, tab), Mode::Normal);
    let block = harness.app.palette.accent.gamma_multiply(0.5);
    let blocks = |harness: &Harness| {
        harness
            .fills
            .iter()
            .filter(|(_, fill)| *fill == block)
            .count()
    };
    assert_eq!(blocks(&harness), 1, "normal mode's cursor is a block");
    keys(&mut harness, "i");
    assert!(harness.painted_rect("-- INSERT --").is_some());
    assert_eq!(blocks(&harness), 0, "insert mode's is a line");
    keys(&mut harness, "x");
    assert_eq!(text(&harness, tab), "SELECT x1");
    escape(&mut harness);
    assert_eq!(mode(&harness, tab), Mode::Normal);
    assert!(harness.ctx.text_edit_focused());
    escape(&mut harness);
    assert!(!harness.ctx.text_edit_focused());
}

/// The events of `c` typed, as the window sends them.
fn typed(c: char) -> Vec<egui::Event> {
    let key = Key::from_name(&c.to_ascii_uppercase().to_string()).expect("a letter");
    let shift = if c.is_uppercase() {
        Modifiers::SHIFT
    } else {
        Modifiers::NONE
    };
    vec![
        crate::testing::key(key, shift),
        egui::Event::Text(c.to_string()),
        crate::testing::release(key, shift),
    ]
}

#[test]
fn keys_in_one_frame_are_read_in_the_mode_the_ones_before_left() {
    // Into insert mode and typing, in one frame.
    let (mut harness, tab) = editor(Look::omarchy(), "select 1");
    escape(&mut harness);
    let mut events = typed('A');
    events.extend(typed('x'));
    harness.frame(events);
    harness.settle();
    assert_eq!(mode(&harness, tab), Mode::Insert);
    assert_eq!(text(&harness, tab), "select 1x");
    // Typing, out of insert mode and a command, in one frame.
    let mut events = typed('y');
    events.push(crate::testing::key(Key::Escape, Modifiers::NONE));
    events.push(crate::testing::release(Key::Escape, Modifiers::NONE));
    events.extend(typed('x'));
    harness.frame(events);
    harness.settle();
    assert_eq!(mode(&harness, tab), Mode::Normal);
    assert_eq!(text(&harness, tab), "select 1x");
    // An undo and the keys after it, in one frame: they read the script
    // the undo gave back.
    let (mut harness, tab) = editor(Look::omarchy(), "abc");
    escape(&mut harness);
    keys(&mut harness, "0x");
    assert_eq!(text(&harness, tab), "bc");
    let mut events = typed('u');
    events.extend(typed('l'));
    events.extend(typed('x'));
    harness.frame(events);
    harness.settle();
    assert_eq!(text(&harness, tab), "ac");
}

#[test]
fn tab_in_normal_mode_types_nothing() {
    let (mut harness, tab) = editor(Look::omarchy(), "select 1");
    escape(&mut harness);
    harness.press(Key::Tab, Modifiers::NONE);
    assert_eq!(text(&harness, tab), "select 1");
}

#[test]
fn undo_after_a_visual_delete_leaves_no_selection() {
    let (mut harness, tab) = editor(Look::omarchy(), "select 1;\nselect 2;");
    escape(&mut harness);
    keys(&mut harness, "ggVd");
    keys(&mut harness, "u");
    assert_eq!(text(&harness, tab), "select 1;\nselect 2;");
    assert_eq!(mode(&harness, tab), Mode::Normal);
    assert_eq!(selected(&harness, tab), None);
}

#[test]
fn a_cut_in_visual_mode_ends_it() {
    let (mut harness, tab) = editor(Look::omarchy(), "select 1");
    escape(&mut harness);
    keys(&mut harness, "0ve");
    assert!(harness.painted_rect("esc leave editor").is_none());
    harness.frame(vec![egui::Event::Cut]);
    harness.settle();
    assert_eq!(text(&harness, tab), " 1");
    assert_eq!(mode(&harness, tab), Mode::Normal);
}

#[test]
fn a_held_esc_in_insert_mode_stays_in_the_editor() {
    let (mut harness, tab) = editor(Look::omarchy(), "select 1");
    // Held: the press again with no release between, which egui reads as
    // a repeat.
    harness.frame(vec![crate::testing::key(Key::Escape, Modifiers::NONE)]);
    harness.frame(vec![crate::testing::key(Key::Escape, Modifiers::NONE)]);
    harness.frame(vec![crate::testing::release(Key::Escape, Modifiers::NONE)]);
    harness.settle();
    assert_eq!(mode(&harness, tab), Mode::Normal);
    assert!(harness.ctx.text_edit_focused());
    // Ctrl+[ is Esc too.
    keys(&mut harness, "i");
    harness.press(Key::OpenBracket, Modifiers::CTRL);
    assert_eq!(mode(&harness, tab), Mode::Normal);
}

#[test]
fn ct_to_the_next_character_changes_what_is_between() {
    let (mut harness, tab) = editor(Look::omarchy(), "select count(a)");
    escape(&mut harness);
    keys(&mut harness, "$hct)n");
    assert_eq!(mode(&harness, tab), Mode::Insert);
    assert_eq!(text(&harness, tab), "select count(n)");
}

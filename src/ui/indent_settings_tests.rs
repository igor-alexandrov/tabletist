//! Settings › Editor's Indent, in both looks and with both keybindings:
//! what each value changes in the SQL editor's text. Tab and Shift+Tab,
//! vim's `>` and `<`, Format, Enter (which keeps the line's own) and
//! vim's `o`, and completion.

use egui::{Key, Modifiers};

use crate::model::{Action, ConnTabId};
use crate::settings::{EditorKeys, Indent, OptionValue};
use crate::testing::Harness;
use crate::theme::Look;

const COMMAND_SHIFT: Modifiers = Modifiers::COMMAND.plus(Modifiers::SHIFT);

/// Each value, and what one indent of it is.
const INDENTS: [(Indent, &str); 3] = [
    (Indent::Two, "  "),
    (Indent::Four, "    "),
    (Indent::Tab, "\t"),
];

/// A connected fixture in `look` with `keys`, the indent set to `indent`,
/// and a new SQL editor that has the keyboard and holds `text`, its
/// cursor at the end (with vim, in insert mode).
fn editor(look: Look, keys: EditorKeys, indent: Indent, text: &str) -> (Harness, ConnTabId) {
    let mut harness = Harness::new();
    harness.set_look(look);
    set(&mut harness, OptionValue::EditorKeys(Some(keys)));
    set(&mut harness, OptionValue::Indent(indent));
    let tab = harness.connect_fake();
    harness.press(Key::T, Modifiers::COMMAND);
    harness.start_typing();
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

fn set(harness: &mut Harness, value: OptionValue) {
    harness.app.apply(Action::SetOption(value));
    harness.settle();
}

/// Types `keys` as the window sends them, as `vim_tests`' helper does.
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

/// Both looks, each with both keybindings.
fn every_editor() -> Vec<(Look, EditorKeys)> {
    let mut all = Vec::new();
    for look in [Look::macos(), Look::omarchy()] {
        for keys in [EditorKeys::Standard, EditorKeys::Vim] {
            all.push((look, keys));
        }
    }
    all
}

#[test]
fn tab_and_shift_tab_put_in_and_take_out_the_indent_set_while_typing() {
    for (look, keys) in every_editor() {
        for (indent, unit) in INDENTS {
            let (mut harness, tab) = editor(look, keys, indent, "SELECT 1\n");
            let case = format!("{} {keys:?} {indent:?}", look.name);
            harness.press(Key::Tab, Modifiers::NONE);
            harness.settle();
            harness.frame(vec![egui::Event::Text("FROM t".into())]);
            harness.settle();
            assert_eq!(
                text(&harness, tab),
                format!("SELECT 1\n{unit}FROM t"),
                "{case}"
            );
            harness.press(Key::Tab, Modifiers::SHIFT);
            harness.settle();
            assert_eq!(text(&harness, tab), "SELECT 1\nFROM t", "{case}");
            // Set again with the editor open, it follows at once.
            set(&mut harness, OptionValue::Indent(Indent::Four));
            harness.press(Key::Tab, Modifiers::SHIFT);
            harness.press(Key::Tab, Modifiers::NONE);
            harness.settle();
            assert_eq!(text(&harness, tab), "SELECT 1\nFROM t    ", "{case}");
        }
    }
}

#[test]
fn tab_in_vim_s_normal_mode_puts_in_nothing() {
    for look in [Look::macos(), Look::omarchy()] {
        for (indent, _) in INDENTS {
            let (mut harness, tab) = editor(look, EditorKeys::Vim, indent, "SELECT 1");
            harness.press(Key::Escape, Modifiers::NONE);
            harness.press(Key::Tab, Modifiers::NONE);
            harness.press(Key::Tab, Modifiers::SHIFT);
            harness.settle();
            assert_eq!(text(&harness, tab), "SELECT 1", "{} {indent:?}", look.name);
        }
    }
}

#[test]
fn vim_s_shifts_put_in_and_take_out_the_indent_set() {
    for look in [Look::macos(), Look::omarchy()] {
        for (indent, unit) in INDENTS {
            let (mut harness, tab) = editor(look, EditorKeys::Vim, indent, "SELECT 1\nFROM t");
            let case = format!("{} {indent:?}", look.name);
            harness.press(Key::Escape, Modifiers::NONE);
            keys(&mut harness, "gg>j");
            assert_eq!(
                text(&harness, tab),
                format!("{unit}SELECT 1\n{unit}FROM t"),
                "{case}"
            );
            keys(&mut harness, ">>");
            assert_eq!(
                text(&harness, tab),
                format!("{unit}{unit}SELECT 1\n{unit}FROM t"),
                "{case}"
            );
            keys(&mut harness, "<<j<<");
            assert_eq!(
                text(&harness, tab),
                format!("{unit}SELECT 1\nFROM t"),
                "{case}"
            );
        }
    }
}

/// Formats: the chord of the macOS look, or the Omarchy screen's `=` with
/// the editor left.
fn format(harness: &mut Harness, look: &Look) {
    if look.terminal {
        harness.leave_editor();
        super::tests::type_key(harness, Key::Equals, "=");
    } else {
        harness.press(Key::F, COMMAND_SHIFT);
    }
    harness.settle();
}

#[test]
fn format_sets_a_case_s_parts_in_by_the_indent_and_nothing_else() {
    let script = "select p.name, case when c.kind = 'ebook' then 1 else 0 end as ebook, \
        count(*) from book_covers c join publishers p on p.id = c.publisher_id \
        where c.id in (select id from book_covers where kind = 'print') group by p.name";
    // The river, a list's continuation and a subquery are the same with
    // each value; a CASE's WHEN and ELSE go in two, four, and four for a
    // tab (spaces, never a tab character).
    for look in [Look::macos(), Look::omarchy()] {
        for (indent, case_in) in [(Indent::Two, 2), (Indent::Four, 4), (Indent::Tab, 4)] {
            let (mut harness, tab) = editor(look, EditorKeys::Standard, indent, script);
            format(&mut harness, &look);
            let when = " ".repeat(7 + case_in);
            let want = [
                "SELECT p.name,".to_owned(),
                "       CASE".to_owned(),
                format!("{when}WHEN c.kind = 'ebook' THEN 1"),
                format!("{when}ELSE 0"),
                "       END AS ebook,".to_owned(),
                "       count(*)".to_owned(),
                "  FROM book_covers c".to_owned(),
                "  JOIN publishers p ON p.id = c.publisher_id".to_owned(),
                " WHERE c.id IN (SELECT id".to_owned(),
                "                  FROM book_covers".to_owned(),
                "                 WHERE kind = 'print')".to_owned(),
                " GROUP BY p.name".to_owned(),
            ]
            .join("\n");
            let want = if look.terminal {
                want.to_lowercase()
            } else {
                want
            };
            let got = text(&harness, tab);
            assert_eq!(got, want, "{} {indent:?}", look.name);
            assert!(!got.contains('\t'), "{} {indent:?}", look.name);
        }
    }
}

/// Selects from the character `anchor` to `head`, the moving end, as a
/// drag does; with the two the same, puts the cursor there.
fn select(harness: &mut Harness, tab: ConnTabId, anchor: usize, head: usize) {
    use egui::text::{CCursor, CCursorRange};
    let workspace = harness.app.workspace(tab).unwrap();
    let id = workspace.active_sql_tab().unwrap().id;
    let editor = crate::ui::sql_text::editor_id(tab, id);
    let mut state = egui::TextEdit::load_state(&harness.ctx, editor).expect("an editor");
    let range = CCursorRange::two(CCursor::new(anchor), CCursor::new(head));
    state.cursor.set_char_range(Some(range));
    egui::TextEdit::store_state(&harness.ctx, editor, state);
    harness.settle();
}

/// Presses Enter, then types `typed`, a frame each.
fn enter_then(harness: &mut Harness, typed: &str) {
    harness.press(Key::Enter, Modifiers::NONE);
    harness.settle();
    harness.frame(vec![egui::Event::Text(typed.into())]);
    harness.settle();
}

#[test]
fn enter_starts_the_new_line_at_the_line_s_indentation() {
    for (look, keys) in every_editor() {
        for (indent, unit) in INDENTS {
            let case = format!("{} {keys:?} {indent:?}", look.name);
            // Each value's own indentation is kept, and so is another's:
            // it is the line's whitespace, not the setting's.
            for lead in [unit.to_owned(), format!("{unit}{unit}"), " \t ".to_owned()] {
                let line = format!("{lead}SELECT 1");
                let (mut harness, tab) = editor(look, keys, indent, &line);
                enter_then(&mut harness, "x");
                assert_eq!(
                    text(&harness, tab),
                    format!("{line}\n{lead}x"),
                    "{case} {lead:?}"
                );
            }
            // Shift+Enter too.
            let line = format!("{unit}SELECT 1");
            let (mut harness, tab) = editor(look, keys, indent, &line);
            harness.press(Key::Enter, Modifiers::SHIFT);
            harness.settle();
            assert_eq!(text(&harness, tab), format!("{line}\n{unit}"), "{case}");
        }
    }
}

#[test]
fn enter_at_or_inside_the_indentation_keeps_what_follows_where_it_was() {
    for (look, keys) in every_editor() {
        let case = format!("{} {keys:?}", look.name);
        // At the line's start: nothing added, the line moves down whole.
        let (mut harness, tab) = editor(look, keys, Indent::Two, "SELECT 1\n    FROM t");
        select(&mut harness, tab, 9, 9);
        harness.press(Key::Enter, Modifiers::NONE);
        harness.settle();
        assert_eq!(text(&harness, tab), "SELECT 1\n\n    FROM t", "{case}");
        // Two into four spaces: FROM keeps its column.
        let (mut harness, tab) = editor(look, keys, Indent::Two, "SELECT 1\n    FROM t");
        select(&mut harness, tab, 11, 11);
        harness.press(Key::Enter, Modifiers::NONE);
        harness.settle();
        assert_eq!(text(&harness, tab), "SELECT 1\n  \n    FROM t", "{case}");
    }
}

#[test]
fn enter_over_a_selection_takes_the_indentation_of_its_start_s_line() {
    for look in [Look::macos(), Look::omarchy()] {
        let script = "    SELECT 1\nFROM t";
        let (mut harness, tab) = editor(look, EditorKeys::Standard, Indent::Two, script);
        // `SELECT 1\nFR` selected.
        select(&mut harness, tab, 4, 15);
        harness.press(Key::Enter, Modifiers::NONE);
        harness.settle();
        assert_eq!(text(&harness, tab), "    \n    OM t", "{}", look.name);
    }
}

#[test]
fn typing_and_enters_in_one_frame_keep_their_order_and_each_line_s_indentation() {
    for look in [Look::macos(), Look::omarchy()] {
        let (mut harness, tab) = editor(look, EditorKeys::Standard, Indent::Two, "  ");
        let enter = || {
            [
                crate::testing::key(Key::Enter, Modifiers::NONE),
                crate::testing::release(Key::Enter, Modifiers::NONE),
            ]
        };
        let mut frame = vec![egui::Event::Text("a".into())];
        frame.extend(enter());
        frame.push(egui::Event::Text("b".into()));
        frame.extend(enter());
        frame.extend(enter());
        frame.push(egui::Event::Text("c".into()));
        harness.frame(frame);
        harness.settle();
        assert_eq!(text(&harness, tab), "  a\n  b\n  \n  c", "{}", look.name);
    }
}

#[test]
fn enter_under_an_open_list_breaks_the_line_with_its_indentation_or_accepts() {
    for look in [Look::macos(), Look::omarchy()] {
        // On what is already typed: a line break, and the list is done.
        let (mut harness, tab) = editor(look, EditorKeys::Standard, Indent::Two, "  ");
        harness.frame(vec![egui::Event::Text("as".into())]);
        harness.settle();
        harness.press(Key::Enter, Modifiers::NONE);
        harness.settle();
        assert_eq!(text(&harness, tab), "  as\n  ", "{}", look.name);
        let workspace = harness.app.workspace(tab).unwrap();
        let open = workspace.active_sql_tab().unwrap().completion.is_some();
        assert!(!open, "{}", look.name);
        // On a row: it is accepted, and no line break.
        let (mut harness, tab) = editor(look, EditorKeys::Standard, Indent::Two, "  ");
        harness.frame(vec![egui::Event::Text("sel".into())]);
        harness.settle();
        harness.press(Key::Enter, Modifiers::NONE);
        harness.settle();
        let word = if look.terminal { "select" } else { "SELECT" };
        assert_eq!(text(&harness, tab), format!("  {word}"), "{}", look.name);
    }
}

#[test]
fn enter_in_vim_s_normal_mode_moves_and_types_nothing() {
    for look in [Look::macos(), Look::omarchy()] {
        let script = "  SELECT 1\n  FROM t";
        let (mut harness, tab) = editor(look, EditorKeys::Vim, Indent::Two, script);
        harness.press(Key::Escape, Modifiers::NONE);
        keys(&mut harness, "gg");
        harness.press(Key::Enter, Modifiers::NONE);
        harness.settle();
        assert_eq!(text(&harness, tab), script, "{}", look.name);
        let workspace = harness.app.workspace(tab).unwrap();
        let cursor = workspace.active_sql_tab().unwrap().cursor;
        assert_eq!(cursor, 13, "{}", look.name);
    }
}

#[test]
fn an_enter_and_its_indentation_undo_with_what_is_typed_beside_them() {
    let (mut harness, tab) = editor(
        Look::macos(),
        EditorKeys::Standard,
        Indent::Two,
        "  SELECT 1",
    );
    enter_then(&mut harness, "x");
    // No undo point between: undo takes back all that was typed, the line
    // break and its indentation with it, as it does for a completion
    // (`complete_tests::tab_on_the_row_that_is_already_typed_changes_nothing`).
    harness.press(Key::Z, Modifiers::COMMAND);
    harness.settle();
    assert_eq!(text(&harness, tab), "");
}

#[test]
fn vim_s_open_line_keeps_the_line_s_indentation() {
    for look in [Look::macos(), Look::omarchy()] {
        for (indent, unit) in INDENTS {
            let line = format!("{unit}SELECT 1");
            let (mut harness, tab) = editor(look, EditorKeys::Vim, indent, &line);
            harness.press(Key::Escape, Modifiers::NONE);
            keys(&mut harness, "ox");
            assert_eq!(
                text(&harness, tab),
                format!("{line}\n{unit}x"),
                "{} {indent:?}",
                look.name
            );
        }
    }
}

#[test]
fn completion_leaves_the_line_s_indentation_as_it_was() {
    for (look, keys) in every_editor() {
        for (indent, unit) in INDENTS {
            let (mut harness, tab) = editor(look, keys, indent, unit);
            harness.frame(vec![egui::Event::Text("sel".into())]);
            harness.settle();
            harness.press(Key::Tab, Modifiers::NONE);
            harness.settle();
            let word = if look.terminal { "select" } else { "SELECT" };
            assert_eq!(
                text(&harness, tab),
                format!("{unit}{word}"),
                "{} {keys:?} {indent:?}",
                look.name
            );
        }
    }
}

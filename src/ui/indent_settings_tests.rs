//! Settings › Editor's Indent, in both looks and with both keybindings:
//! what each value changes in the SQL editor's text. Tab and Shift+Tab,
//! vim's `>` and `<`, Format, Enter and vim's `o`, and completion.

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

#[test]
fn enter_starts_the_new_line_at_its_start_whatever_the_indent() {
    for (look, keys) in every_editor() {
        for (indent, unit) in INDENTS {
            let line = format!("{unit}SELECT 1");
            let (mut harness, tab) = editor(look, keys, indent, &line);
            harness.press(Key::Enter, Modifiers::NONE);
            harness.settle();
            harness.frame(vec![egui::Event::Text("x".into())]);
            harness.settle();
            assert_eq!(
                text(&harness, tab),
                format!("{line}\nx"),
                "{} {keys:?} {indent:?}",
                look.name
            );
        }
    }
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

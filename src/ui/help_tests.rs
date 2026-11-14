//! The keyboard shortcuts dialog's search (`?`), headless: what it keeps
//! of the list by words, typed keys and a chord pressed, what it says when
//! it keeps nothing, Esc taking the search back before it closes, and
//! the field having the keyboard from the start, in each look.

use egui::accesskit::Role;
use egui::{Event, Key, Modifiers, Rect};

use crate::i18n::Locale;
use crate::keymap::{Command, Group};
use crate::model::{Action, Dialog, HelpSearch};
use crate::testing::Harness;
use crate::theme::Look;

/// Command held, as each look's window reports it: ⌘ on macOS, Ctrl
/// elsewhere.
fn command_held(look: &Look) -> Modifiers {
    if look == &Look::macos() {
        Modifiers::MAC_CMD | Modifiers::COMMAND
    } else {
        Modifiers::CTRL | Modifiers::COMMAND
    }
}

/// The dialog open over the start screen in `look`.
fn opened(look: Look) -> Harness {
    let mut harness = Harness::new();
    harness.set_look(look);
    harness.app.apply(Action::ShowHelp);
    harness.finish_animations();
    harness
}

/// The dialog's search, or a failure if the dialog is not open.
fn search(harness: &Harness) -> (String, Option<&'static str>) {
    match &harness.app.dialog {
        Some(Dialog::Help(help)) => (help.search.clone(), help.found),
        other => panic!("the shortcuts are not open: {other:?}"),
    }
}

/// The commands the dialog lists for `text` typed and `found` pressed.
fn listed(harness: &Harness, text: &str, found: Option<&'static str>) -> Vec<Command> {
    let search = HelpSearch {
        search: text.into(),
        found,
    };
    let layout = harness.app.layout();
    crate::ui::help::listed(&harness.app.keymap, layout, Locale::default(), &search)
        .into_iter()
        .map(|row| row.command)
        .collect()
}

/// Types `text` as the window sends it.
fn type_text(harness: &mut Harness, text: &str) {
    harness.frame(vec![Event::Text(text.into())]);
    harness.settle();
}

/// Where the Close button is: the dialog's foot.
fn close_button_top(harness: &mut Harness) -> f64 {
    let tree = harness.settle();
    let id = crate::testing::node(&tree, "Close", Role::Button).expect("a Close button");
    let (_, node) = tree.nodes.iter().find(|(at, _)| *at == id).unwrap();
    node.bounds().expect("bounds").y0
}

/// A group's heading as the dialog paints it: capitals, or Omarchy's
/// lower case.
fn painted_heading(look: &Look, group: Group) -> String {
    let name = crate::ui::help::heading(group);
    if look.terminal {
        name.to_lowercase()
    } else {
        name.to_uppercase()
    }
}

/// A heading or a command's name, painted in the list.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Shown {
    Heading(Group),
    Row(Group),
}

/// The groups and commands the dialog lists in its look.
fn grouped(harness: &Harness) -> Vec<(Group, Command)> {
    let layout = harness.app.layout();
    crate::ui::help::rows(&harness.app.keymap, layout)
        .into_iter()
        .map(|(group, command, _)| (group, command))
        .collect()
}

/// What the list's scroll area shows of the last frame: the texts painted
/// inside its clip, which a heading names. Not the window behind the
/// dialog, nor its buttons.
fn in_list(harness: &Harness) -> Vec<(String, Rect)> {
    let look = harness.app.look;
    let headings: Vec<String> = grouped(harness)
        .iter()
        .map(|(group, _)| painted_heading(&look, *group))
        .collect();
    let Some(list) = harness
        .clipped_text
        .iter()
        .find(|(text, ..)| headings.contains(text))
        .map(|(.., clip)| *clip)
    else {
        return Vec::new();
    };
    let same =
        |clip: &Rect| (clip.min - list.min).length() < 1.0 && (clip.max - list.max).length() < 1.0;
    harness
        .clipped_text
        .iter()
        .filter(|(_, rect, clip)| same(clip) && clip.contains_rect(*rect))
        .map(|(text, rect, _)| (text.clone(), *rect))
        .collect()
}

/// Where `text` is painted in the list last frame, if it is.
fn visible(harness: &Harness, text: &str) -> Vec<Rect> {
    in_list(harness)
        .into_iter()
        .filter(|(painted, _)| painted == text)
        .map(|(_, rect)| rect)
        .collect()
}

/// What the list shows, top to bottom: each heading and each command's
/// name painted in it, with where it stands.
fn shown(harness: &mut Harness, rows: &[(Group, Command)]) -> Vec<(Shown, Rect)> {
    harness.settle();
    let look = harness.app.look;
    let mut shown: Vec<(Shown, Rect)> = in_list(harness)
        .iter()
        .filter_map(|(text, rect)| {
            let heading = rows
                .iter()
                .find(|(group, _)| painted_heading(&look, *group) == *text)
                .map(|(group, _)| Shown::Heading(*group));
            let name = rows
                .iter()
                .find(|(_, command)| command.info().name == text)
                .map(|(group, _)| Shown::Row(*group));
            heading.or(name).map(|shown| (shown, *rect))
        })
        .collect();
    shown.sort_by(|a, b| a.1.top().total_cmp(&b.1.top()));
    shown
}

#[test]
fn the_search_is_the_dialogs_own_and_a_find_drops_the_words() {
    let mut harness = Harness::new();
    harness.app.apply(Action::ShowHelp);
    harness.app.apply(Action::HelpSearch("row".into()));
    assert_eq!(search(&harness), ("row".into(), None));
    harness.app.apply(Action::HelpFind(Some("cmd+r")));
    assert_eq!(search(&harness), (String::new(), Some("cmd+r")));
    harness.app.apply(Action::HelpSearch("re".into()));
    assert_eq!(search(&harness), ("re".into(), None));
    // Opened again, it starts with nothing.
    harness.app.apply(Action::CloseDialog);
    harness.app.apply(Action::ShowHelp);
    assert_eq!(search(&harness), (String::new(), None));
}

#[test]
fn words_typed_keys_and_chords_cut_the_list_in_every_look() {
    for (look, typed) in [
        (Look::macos(), ["cmd+r", "⌘R", "cmd r"]),
        (Look::standard(), ["ctrl+r", "Ctrl-R", "ctrl r"]),
        (Look::omarchy(), ["R", "R", "R"]),
    ] {
        let harness = opened(look);
        let name = look.name;
        let layout = harness.app.layout();
        let all = listed(&harness, "", None);
        let rows = crate::ui::help::rows(&harness.app.keymap, layout);
        assert_eq!(all.len(), rows.len(), "{name}");
        // By the command's name, in any case.
        let row = listed(&harness, "ROW", None);
        assert!(row.contains(&Command::AddRow), "{name}: {row:?}");
        assert!(!row.contains(&Command::Reload), "{name}: {row:?}");
        // By its group's name.
        let sql = listed(&harness, "sql editor", None);
        assert!(sql.contains(&Command::RunStatement), "{name}: {sql:?}");
        // By its keys as the look writes them, or as they are spelled.
        for text in typed {
            let keys = listed(&harness, text, None);
            assert!(keys.contains(&Command::Reload), "{name}, {text}: {keys:?}");
            // One letter is a word too, in many names.
            if text.chars().count() > 1 {
                assert!(!keys.contains(&Command::AddRow), "{name}, {text}: {keys:?}");
            }
        }
        // By a chord pressed: Find any object's own.
        let keymap = &harness.app.keymap;
        let find = keymap.command_keys(layout, Command::FindObject);
        let chord = find
            .iter()
            .find(|keys| keys.contains('+'))
            .copied()
            .unwrap();
        let found = listed(&harness, "", Some(chord));
        assert!(found.contains(&Command::FindObject), "{name}: {found:?}");
        // And by the same chord typed, as the look writes it or spells it.
        let written = crate::keymap::key_written(layout, chord);
        for text in [chord, written.as_str()] {
            let typed = listed(&harness, text, None);
            assert!(
                typed.contains(&Command::FindObject),
                "{name}, {text}: {typed:?}"
            );
            assert!(
                !typed.contains(&Command::AddRow),
                "{name}, {text}: {typed:?}"
            );
        }
        assert!(found.len() < all.len(), "{name}");
        assert!(listed(&harness, "zzz", None).is_empty(), "{name}");
    }
}

#[test]
fn the_field_has_the_keyboard_as_the_dialog_opens() {
    for look in Look::ALL {
        let mut harness = opened(look);
        type_text(&mut harness, "row");
        assert_eq!(search(&harness).0, "row", "{}", look.name);
        assert!(harness.has("Add row"), "{}", look.name);
        assert!(!harness.has("Reload"), "{}", look.name);
    }
}

#[test]
fn one_letter_commands_type_into_the_field() {
    for look in Look::ALL {
        let mut harness = opened(look);
        super::tests::type_key(&mut harness, Key::Questionmark, "?");
        super::tests::type_key(&mut harness, Key::R, "R");
        super::tests::type_key(&mut harness, Key::Slash, "/");
        super::tests::type_key(&mut harness, Key::T, "t");
        assert_eq!(search(&harness).0, "?R/t", "{}", look.name);
    }
}

#[test]
fn a_chord_pressed_in_the_field_finds_its_commands() {
    for (look, found, said) in [
        (Look::macos(), "cmd+r", "Commands bound to ⌘R"),
        (Look::standard(), "cmd+r", "Commands bound to Ctrl+R"),
        (Look::omarchy(), "ctrl+r", "commands bound to ctrl+r"),
    ] {
        let mut harness = opened(look);
        type_text(&mut harness, "row");
        harness.press(Key::R, command_held(&look));
        assert_eq!(
            search(&harness),
            (String::new(), Some(found)),
            "{}",
            look.name
        );
        assert!(harness.has(said), "{}", look.name);
        let listed = listed(&harness, "", Some(found));
        assert!(!listed.is_empty(), "{}", look.name);
        if look != Look::omarchy() {
            assert!(harness.has("Reload"), "{}", look.name);
        }
        // Show all lists them all again.
        harness.app.apply(Action::HelpFind(None));
        harness.settle();
        assert!(harness.has("Add row"), "{}", look.name);
    }
}

#[test]
fn the_fields_editing_chords_edit_its_text() {
    for look in Look::ALL {
        let name = look.name;
        let mut harness = opened(look);
        type_text(&mut harness, "row");
        // Select all, then type over it.
        harness.press(Key::A, command_held(&look));
        assert_eq!(search(&harness), ("row".into(), None), "{name}");
        type_text(&mut harness, "sql");
        assert_eq!(search(&harness), ("sql".into(), None), "{name}");
        // Ctrl+U deletes back to the line's start.
        harness.press(Key::U, Modifiers::CTRL);
        assert_eq!(search(&harness), (String::new(), None), "{name}");
    }
}

#[test]
fn a_text_size_chord_is_found_not_done() {
    for (look, found) in [
        (Look::macos(), "cmd+="),
        (Look::standard(), "cmd+="),
        (Look::omarchy(), "ctrl+="),
    ] {
        let mut harness = opened(look);
        let sizes = |harness: &Harness| {
            let settings = &harness.app.settings;
            format!("{:?} {:?}", settings.ui_scale, settings.font_size)
        };
        let before = sizes(&harness);
        harness.press(Key::Equals, command_held(&look));
        assert_eq!(
            search(&harness),
            (String::new(), Some(found)),
            "{}",
            look.name
        );
        assert_eq!(sizes(&harness), before, "{}", look.name);
    }
}

#[test]
fn what_an_alt_chord_types_does_not_drop_it() {
    let mut harness = opened(Look::macos());
    type_text(&mut harness, "row");
    let alt = Modifiers::ALT;
    harness.frame(vec![
        crate::testing::key(Key::R, alt),
        Event::Text("®".into()),
    ]);
    harness.frame(vec![crate::testing::release(Key::R, alt)]);
    harness.settle();
    assert_eq!(search(&harness), (String::new(), Some("alt+r")));
    // A chord with Cmd types nothing: text after it is typed.
    let cmd = command_held(&Look::macos());
    harness.frame(vec![
        crate::testing::key(Key::R, cmd),
        Event::Text("x".into()),
    ]);
    harness.frame(vec![crate::testing::release(Key::R, cmd)]);
    harness.settle();
    assert_eq!(search(&harness).0, "x");
}

#[test]
fn a_search_that_keeps_nothing_says_so() {
    for look in Look::ALL {
        let mut harness = opened(look);
        type_text(&mut harness, "zzz");
        assert!(harness.has("No commands match “zzz”") || harness.has("no commands match “zzz”"));
        assert!(!harness.has("Add row"), "{}", look.name);
    }
}

#[test]
fn esc_takes_the_search_back_then_closes() {
    for look in Look::ALL {
        let name = look.name;
        let mut harness = opened(look);
        type_text(&mut harness, "row");
        harness.press(Key::Escape, Modifiers::NONE);
        assert_eq!(search(&harness), (String::new(), None), "{name}");
        // The field kept the keyboard.
        type_text(&mut harness, "sql");
        assert_eq!(search(&harness).0, "sql", "{name}");
        harness.press(Key::Escape, Modifiers::NONE);
        // A chord looked for is taken back the same way.
        harness.press(Key::R, command_held(&look));
        assert!(search(&harness).1.is_some(), "{name}");
        harness.press(Key::Escape, Modifiers::NONE);
        assert_eq!(search(&harness), (String::new(), None), "{name}");
        harness.press(Key::Escape, Modifiers::NONE);
        assert!(harness.app.dialog.is_none(), "{name}");
    }
}

#[test]
fn the_dialog_keeps_its_size_while_it_is_searched() {
    for look in Look::ALL {
        let mut harness = opened(look);
        let before = close_button_top(&mut harness);
        for text in ["zzz", "row"] {
            harness.app.apply(Action::HelpSearch(text.into()));
            harness.finish_animations();
            assert_eq!(
                close_button_top(&mut harness),
                before,
                "{}, {text}",
                look.name
            );
        }
        // A chord looked for: its line takes its place from the list's.
        harness.press(Key::R, command_held(&look));
        harness.finish_animations();
        assert!(search(&harness).1.is_some(), "{}", look.name);
        assert_eq!(
            close_button_top(&mut harness),
            before,
            "{}, a chord",
            look.name
        );
    }
}

#[test]
fn each_group_has_room_above_its_heading_but_the_first() {
    for look in Look::ALL {
        let name = look.name;
        let mut harness = opened(look);
        let rows = grouped(&harness);
        let shown = shown(&mut harness, &rows);
        assert!(matches!(shown[0].0, Shown::Heading(_)), "{name}: {shown:?}");
        let first_row = shown
            .iter()
            .find(|(at, _)| matches!(at, Shown::Row(_)))
            .expect("a row");
        // A heading starts where the names start: across the list.
        assert!(
            (shown[0].1.left() - first_row.1.left()).abs() < 1.0,
            "{name}"
        );
        // Each group is a row group named by its heading.
        let tree = harness.settle();
        for (group, _) in &rows {
            let said = look.label(crate::ui::help::heading(*group));
            assert!(
                crate::testing::node(&tree, &said, Role::RowGroup).is_some(),
                "{name}: {said}"
            );
        }
        // The first group fills the list; a search shows several at once.
        type_text(&mut harness, "new");
        let shown = self::shown(&mut harness, &rows);
        let (mut row_gaps, mut heading_gaps) = (Vec::new(), Vec::new());
        for pair in shown.windows(2) {
            let gap = pair[1].1.top() - pair[0].1.bottom();
            match pair[1].0 {
                Shown::Heading(_) => heading_gaps.push(gap),
                Shown::Row(_) => row_gaps.push(gap),
            }
        }
        assert!(!heading_gaps.is_empty(), "{name}: one group shown");
        let widest = row_gaps.iter().copied().fold(0.0_f32, f32::max);
        for gap in heading_gaps {
            assert!(gap >= widest + 10.0, "{name}: {gap} after rows {widest}");
        }
    }
}

#[test]
fn keys_stand_at_the_right_after_their_names() {
    for look in Look::ALL {
        let name = look.name;
        let mut harness = opened(look);
        harness.settle();
        let layout = harness.app.layout();
        let rows = crate::ui::help::rows(&harness.app.keymap, layout);
        let mut rights = Vec::new();
        for (_, command, keys) in &rows {
            let (Some(what), Some(keys)) = (
                visible(&harness, command.info().name).first().copied(),
                visible(&harness, keys.as_str()).first().copied(),
            ) else {
                continue;
            };
            assert!(keys.left() >= what.right() + 20.0, "{name}: {command:?}");
            rights.push(keys.right());
        }
        assert!(rights.len() > 5, "{name}: {rights:?}");
        let least = rights.iter().copied().fold(f32::MAX, f32::min);
        let most = rights.iter().copied().fold(f32::MIN, f32::max);
        assert!(most - least < 1.0, "{name}: {rights:?}");
    }
}

#[test]
fn a_heading_stays_on_top_while_its_group_scrolls() {
    for look in Look::ALL {
        let name = look.name;
        let mut harness = opened(look);
        let rows = grouped(&harness);
        let before = shown(&mut harness, &rows);
        let (Shown::Heading(first), top) = before[0] else {
            panic!("{name}: {before:?}")
        };
        // Nothing is pinned until the list moves: the first heading shows once.
        let once = before
            .iter()
            .filter(|(at, _)| *at == Shown::Heading(first))
            .count();
        assert_eq!(once, 1, "{name}");
        for _ in 0..3 {
            harness.frame(vec![
                Event::PointerMoved(top.center()),
                Event::MouseWheel {
                    unit: egui::MouseWheelUnit::Point,
                    delta: egui::vec2(0.0, -150.0),
                    modifiers: Modifiers::NONE,
                    phase: egui::TouchPhase::Move,
                },
            ]);
        }
        for _ in 0..5 {
            harness.finish_animations();
        }
        let after = shown(&mut harness, &rows);
        // A heading stands where the first one did: the group of the rows
        // under it.
        let (pinned, at) = after
            .iter()
            .find(|(at, rect)| {
                matches!(at, Shown::Heading(_)) && (rect.top() - top.top()).abs() < 1.0
            })
            .copied()
            .unwrap_or_else(|| panic!("{name}: nothing pinned: {after:?}"));
        let Shown::Heading(group) = pinned else {
            unreachable!()
        };
        let under = after
            .iter()
            .find(|(shown, rect)| matches!(shown, Shown::Row(_)) && rect.top() >= at.bottom());
        assert_eq!(
            under.map(|(shown, _)| *shown),
            Some(Shown::Row(group)),
            "{name}: {after:?}"
        );
        // It scrolled: the list's first row is not under it any more.
        let first_row = rows[0].1.info().name;
        assert!(
            visible(&harness, first_row)
                .iter()
                .all(|rect| rect.top() < at.bottom()),
            "{name}"
        );
    }
}

#[test]
fn a_search_keeps_the_headings_of_the_groups_it_keeps() {
    for look in Look::ALL {
        let name = look.name;
        let mut harness = opened(look);
        let rows = grouped(&harness);
        type_text(&mut harness, "new");
        let kept = listed(&harness, "new", None);
        let mut groups: Vec<Group> = rows
            .iter()
            .filter(|(_, command)| kept.contains(command))
            .map(|(group, _)| *group)
            .collect();
        groups.dedup();
        assert!(groups.len() >= 2, "{name}: {groups:?}");
        let shown = shown(&mut harness, &rows);
        let headings: Vec<Group> = shown
            .iter()
            .filter_map(|(at, _)| match at {
                Shown::Heading(group) => Some(*group),
                Shown::Row(_) => None,
            })
            .collect();
        assert_eq!(headings, groups, "{name}");
        // Each heading stands above its own rows.
        let mut under = None;
        for (at, _) in &shown {
            match at {
                Shown::Heading(group) => under = Some(*group),
                Shown::Row(group) => assert_eq!(under, Some(*group), "{name}"),
            }
        }
    }
}

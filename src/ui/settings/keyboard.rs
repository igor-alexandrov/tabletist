//! What the Keyboard pane is in every look: the rows its search, its
//! Changed switch and a key looked for leave, by their groups; how a row is
//! said; what a refusal says; and what a frame's keys come to while a row
//! is recorded. The sheet (`keyboard_sheet.rs`) and the terminal screen
//! (`keyboard_terminal.rs`) draw it.

use egui::{Event, Key, Modifiers};

use crate::i18n::{Locale, gettext};
use crate::keymap::{
    Group, Keymap, Layout, Mods, Never, Row, Spelled, Why, intern, same, spell, spelling,
};
use crate::model::{Action, KeyboardPage, Recorded, Recording};
use crate::theme::Look;

/// A group of the pane, with the rows it lists now.
pub(super) struct Listed {
    pub group: Group,
    pub rows: Vec<Row>,
}

/// The rows the pane lists now, by group: those the search's words name
/// (by the command's name or its group's), those a key looked for is a
/// key of (in every scope), the changed ones alone where the switch says.
pub(super) fn listed(
    keymap: &Keymap,
    layout: Layout,
    locale: Locale,
    page: &KeyboardPage,
) -> Vec<Listed> {
    let words = page.search.trim().to_lowercase();
    let found = page.found.and_then(|keys| spell(keys).ok());
    let mut listed: Vec<Listed> = Vec::new();
    for row in keymap.rows(layout) {
        if page.changed_only && !keymap.changed(layout, row) {
            continue;
        }
        let group = row.command.info().group;
        if !words.is_empty() {
            let name = gettext(locale, row.command.info().name).to_lowercase();
            let heading = gettext(locale, group.name(layout)).to_lowercase();
            if !name.contains(&words) && !heading.contains(&words) {
                continue;
            }
        }
        if let Some(found) = &found {
            let has = keymap
                .keys_of(layout, row)
                .iter()
                .filter_map(|keys| spell(keys).ok())
                // A range of digits is a key for each of them: ⌘3 is one of
                // ⌘1…9.
                .any(|keys| match keys {
                    Spelled::Digits(_) => crate::keymap::clash(layout, &keys, found),
                    _ => same(layout, &keys, found),
                });
            if !has {
                continue;
            }
        }
        match listed.last_mut() {
            Some(last) if last.group == group => last.rows.push(row),
            _ => listed.push(Listed {
                group,
                rows: vec![row],
            }),
        }
    }
    listed
}

/// How many rows differ from their defaults.
pub(super) fn changed_count(keymap: &Keymap, layout: Layout) -> usize {
    keymap
        .rows(layout)
        .into_iter()
        .filter(|row| keymap.changed(layout, *row))
        .count()
}

/// A row's keys as a screen reader says them: "Shift Command F", several
/// with "or" between them, or "not set".
pub(super) fn spoken_keys(keymap: &Keymap, layout: Layout, locale: Locale, row: Row) -> String {
    let keys: Vec<String> = keymap
        .keys_of(layout, row)
        .into_iter()
        .map(|keys| crate::keymap::spoken(layout, keys))
        .collect();
    if keys.is_empty() {
        gettext(locale, "not set").into_owned()
    } else {
        keys.join(&format!(" {} ", gettext(locale, "or")))
    }
}

/// A row's defaults, as a screen reader says them.
pub(super) fn spoken_defaults(layout: Layout, locale: Locale, row: Row) -> String {
    let keys: Vec<String> = crate::keymap::defaults(layout, row)
        .into_iter()
        .map(|keys| crate::keymap::spoken(layout, keys))
        .collect();
    if keys.is_empty() {
        gettext(locale, "not set").into_owned()
    } else {
        keys.join(&format!(" {} ", gettext(locale, "or")))
    }
}

/// A command's name as the look writes it, translated.
pub(super) fn name(look: &Look, locale: Locale, command: crate::keymap::Command) -> String {
    look.label(&gettext(locale, command.info().name))
}

/// What a screen reader says of a row: its name, scope and keys as the
/// look has them, then whether it is changed and whether nothing answers
/// its command yet.
pub(super) fn said(
    keymap: &Keymap,
    look: &Look,
    locale: Locale,
    row: Row,
    [name, scope, keys]: [&str; 3],
) -> String {
    let say = |text| look.label(&gettext(locale, text));
    let mut said = format!("{name}, {scope}, {keys}");
    if keymap.changed(Layout::of(look), row) {
        said.push_str(&format!(", {}", say("changed")));
    }
    if !row.command.info().built {
        said.push_str(&format!(", {}", say("not built yet")));
    }
    said
}

/// What a refusal of `keys` for `row` says, translated: the key as the look
/// writes it, the name of the command it is about, and the scope.
pub(super) fn refusal(
    look: &Look,
    locale: Locale,
    row: Row,
    keys: &'static str,
    why: &Why,
) -> String {
    let layout = Layout::of(look);
    let about = match why {
        Why::Taken(other) => other.command,
        Why::Tabletist(command) => *command,
        _ => row.command,
    };
    let written = if keys.is_empty() {
        String::new()
    } else {
        crate::keymap::key_written(layout, keys).as_str().to_owned()
    };
    let scope = look.label(&gettext(locale, row.scope.name(layout)));
    gettext(locale, why.text(layout))
        .replace("{keys}", &written)
        .replace("{name}", &name(look, locale, about))
        .replace("{scope}", &scope)
}

/// What is held, as `layout` spells it: on macOS Cmd and Control apart,
/// on Windows one Ctrl for both (the keymap's `cmd`), on Omarchy Ctrl.
fn held(layout: Layout, modifiers: Modifiers) -> Mods {
    match layout {
        Layout::Mac => Mods {
            cmd: modifiers.mac_cmd || modifiers.command,
            ctrl: modifiers.ctrl,
            alt: modifiers.alt,
            shift: modifiers.shift,
        },
        Layout::Windows => Mods {
            cmd: modifiers.ctrl || modifiers.command,
            ctrl: false,
            alt: modifiers.alt,
            shift: modifiers.shift,
        },
        Layout::Omarchy => Mods {
            cmd: false,
            ctrl: modifiers.ctrl || (modifiers.command && !modifiers.mac_cmd),
            alt: modifiers.alt,
            shift: modifiers.shift,
        },
    }
}

/// The key the window sends a clipboard event in place of, held with the
/// command key: it turns C, X and V into a copy, a cut and a paste before
/// any key event is made.
fn clipboard_key(event: &Event) -> Option<Key> {
    match event {
        Event::Copy => Some(Key::C),
        Event::Cut => Some(Key::X),
        Event::Paste(_) => Some(Key::V),
        _ => None,
    }
}

/// The chord the frame's first press with Cmd, Ctrl or Alt held makes,
/// taken from the frame: what the macOS search field reads as a key to
/// find, in place of typing it. None, and nothing taken, where there is
/// none, or where it is the field's own (the system's text editing).
pub(super) fn find_chord(ctx: &egui::Context, layout: Layout) -> Option<&'static str> {
    ctx.input_mut(|input| {
        let at = input.events.iter().position(|event| {
            matches!(event, Event::Key { pressed: true, modifiers, .. }
                if modifiers.command || modifiers.ctrl || modifiers.alt || modifiers.mac_cmd)
        })?;
        let Event::Key { key, modifiers, .. } = input.events[at] else {
            return None;
        };
        // The keys that move and delete in the field's text, with whatever
        // held, are the field's own.
        if matches!(
            key,
            Key::Backspace
                | Key::Delete
                | Key::ArrowLeft
                | Key::ArrowRight
                | Key::ArrowUp
                | Key::ArrowDown
                | Key::Home
                | Key::End
        ) {
            return None;
        }
        let keys = intern(&spelling(held(layout, modifiers), key)?);
        let spelled = spell(keys).ok()?;
        if crate::keymap::reserved_by(layout, &spelled).is_some() {
            return None;
        }
        input.events.remove(at);
        Some(keys)
    })
}

/// Whether `event` is a key that presses a focused button: Return, Space
/// or Tab with nothing held, and Space's text.
fn presses_a_button(event: &Event) -> bool {
    match event {
        Event::Key { key, modifiers, .. } => {
            modifiers.is_none() && matches!(key, Key::Enter | Key::Space | Key::Tab)
        }
        Event::Text(text) => text == " ",
        _ => false,
    }
}

/// What the frame's keys come to while `recording`: one key recorded, or
/// refused, or the recording given up, and every key and character of the
/// frame taken, so that nothing else acts on them (Tab moves no focus).
/// While the keyboard is on a button of the refusal's line (`on_buttons`),
/// the keys that press it and move from it are the button's. The input's
/// time ends a key of several on Omarchy.
pub(super) fn record(
    ctx: &egui::Context,
    keymap: &Keymap,
    layout: Layout,
    (recording, on_buttons): (&Recording, bool),
    actions: &mut Vec<Action>,
) {
    let now = ctx.input(|input| input.time);
    let events: Vec<Event> = ctx.input_mut(|input| {
        let (taken, left): (Vec<Event>, Vec<Event>) = std::mem::take(&mut input.events)
            .into_iter()
            .partition(|event| {
                let key = matches!(
                    event,
                    Event::Key { .. } | Event::Text(_) | Event::Copy | Event::Cut | Event::Paste(_)
                );
                key && !(on_buttons && presses_a_button(event))
            });
        input.events = left;
        taken
    });
    match layout {
        Layout::Mac | Layout::Windows => record_chord(layout, recording, &events, actions),
        Layout::Omarchy => {
            // While a key another row has is offered as a swap, the swap's
            // letter takes it before anything is typed again.
            let offered = recording.swaps && recording.typed.is_empty();
            let swaps = offered
                && events.iter().any(|event| {
                    let typed = match event {
                        Event::Text(text) => text.chars().next().map(|char| char.to_string()),
                        Event::Key {
                            key,
                            pressed: true,
                            modifiers,
                            ..
                        } if !(modifiers.ctrl || modifiers.alt || modifiers.command) => {
                            crate::keymap::char_of(*key, modifiers.shift)
                                .map(|char| char.to_string())
                        }
                        _ => None,
                    };
                    typed.is_some_and(|typed| {
                        crate::ui::keys::prompt_letter(
                            keymap,
                            Layout::Omarchy,
                            crate::keymap::When::SettingsScreen,
                            &typed,
                        ) == Some(crate::keymap::Command::SettingsSwap)
                    })
                });
            if swaps {
                actions.push(Action::SwapKeys);
            } else {
                record_typed(ctx, recording, &events, now, actions);
            }
        }
    }
}

/// macOS and Windows: the first chord recorded. Esc gives up, and Delete
/// unbinds the row.
fn record_chord(
    layout: Layout,
    recording: &Recording,
    events: &[Event],
    actions: &mut Vec<Action>,
) {
    for event in events {
        let (key, modifiers) = match event {
            Event::Key {
                key,
                modifiers,
                pressed: true,
                repeat: false,
                ..
            } => (*key, *modifiers),
            other => match clipboard_key(other) {
                Some(key) => (key, Modifiers::COMMAND),
                None => continue,
            },
        };
        let mods = held(layout, modifiers);
        if mods.none() && key == Key::Escape {
            actions.push(Action::StopRecording);
            return;
        }
        if mods.none() && matches!(key, Key::Backspace | Key::Delete) {
            actions.push(match recording.of {
                Recorded::Row(row) => Action::UnbindRow(row),
                Recorded::Find => Action::StopRecording,
            });
            return;
        }
        if let Some(keys) = spelling(mods, key) {
            actions.push(Action::RecordPress(intern(&keys)));
            return;
        }
    }
}

/// Whether a key is one Omarchy records as a chord of its own though
/// nothing is held with it: a key that types no character.
fn named_key(key: Key) -> bool {
    crate::keymap::char_of(key, false).is_none() && crate::keymap::char_of(key, true).is_none()
}

/// Omarchy: typed characters gather into a key of several (`gd`) or a
/// line of the prompt (`:w`), which `enter` ends; a key of several ends
/// [`Recording::WAIT`] after its last character too. A chord is taken at
/// once. Esc gives up, and Backspace takes a character back.
fn record_typed(
    ctx: &egui::Context,
    recording: &Recording,
    events: &[Event],
    now: f64,
    actions: &mut Vec<Action>,
) {
    let mut typed = recording.typed.clone();
    let mut at = recording.at;
    // The frame's characters: its text where it brings some, else the
    // characters its bare keys type (a test's press brings no text).
    let texts = events.iter().any(|event| matches!(event, Event::Text(_)));
    let finish = |typed: &str, actions: &mut Vec<Action>| {
        if typed == ":" {
            actions.push(Action::RecordNever(Never::Colon));
        } else {
            actions.push(Action::RecordPress(intern(typed)));
        }
    };
    for event in events {
        match event {
            Event::Text(text) => {
                // A space is a word's end in a line of the prompt, and
                // nothing anywhere else: Space is a key of its own there.
                for char in text.chars() {
                    if char.is_control() || (char == ' ' && !typed.starts_with(':')) {
                        continue;
                    }
                    typed.push(char);
                    at = now;
                    actions.push(Action::RecordTyped { char, at: now });
                }
            }
            Event::Key {
                key,
                modifiers,
                pressed: true,
                ..
            } => {
                let key = *key;
                if modifiers.mac_cmd {
                    actions.push(Action::RecordNever(Never::Super));
                    return;
                }
                let mods = held(Layout::Omarchy, *modifiers);
                let chord = mods.ctrl || mods.alt;
                if !chord && key == Key::Escape {
                    actions.push(Action::StopRecording);
                    return;
                }
                if !chord && key == Key::Enter {
                    if typed.is_empty() {
                        // Enter alone is a key of its own, as the keymap
                        // spells it.
                        if let Some(enter) = spelling(Mods::default(), Key::Enter) {
                            actions.push(Action::RecordPress(intern(&enter)));
                        }
                    } else {
                        finish(&typed, actions);
                    }
                    return;
                }
                if !chord && key == Key::Backspace {
                    if typed.pop().is_some() {
                        actions.push(Action::RecordBack);
                    }
                    continue;
                }
                if !chord && key == Key::Space && typed.starts_with(':') {
                    // In a line of the prompt: its text (where the frame
                    // brings one) is the space.
                    if !texts {
                        typed.push(' ');
                        at = now;
                        actions.push(Action::RecordTyped { char: ' ', at: now });
                    }
                    continue;
                }
                if chord || named_key(key) {
                    if !typed.is_empty() && !typed.starts_with(':') && !chord {
                        // A key that types nothing ends a key of several.
                        finish(&typed, actions);
                        return;
                    }
                    if let Some(keys) = spelling(mods, key) {
                        actions.push(Action::RecordPress(intern(&keys)));
                    }
                    return;
                }
                // A letter, a digit or a mark: its character, where the
                // frame brings no text for it.
                if !texts && let Some(char) = crate::keymap::char_of(key, modifiers.shift) {
                    typed.push(char);
                    at = now;
                    actions.push(Action::RecordTyped { char, at: now });
                }
            }
            _ => {}
        }
    }
    // A key of several ends when its wait is over. A line of the prompt
    // waits for Enter: its words take longer to type.
    if !typed.is_empty() && !typed.starts_with(':') {
        let waited = now - at;
        if waited >= Recording::WAIT {
            finish(&typed, actions);
        } else {
            let left = Recording::WAIT - waited;
            ctx.request_repaint_after(std::time::Duration::from_secs_f64(left.max(0.0)));
        }
    }
}

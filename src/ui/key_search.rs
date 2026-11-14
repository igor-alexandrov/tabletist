//! Finding commands by what is typed or pressed: the Keyboard settings'
//! list and the keyboard shortcuts dialog (`?`) read this one search.
//! Words are found in a command's name or its group's, in any case, or
//! name one of its keys as the look writes it (`ctrl+r` finds `Ctrl+R`)
//! or spells it (`cmd+r` finds `⌘R`); a chord pressed finds the commands
//! it is a key of.

use std::ops::Range;

use egui::{Event, Key, Modifiers};

use crate::keymap::{
    Layout, Mods, Spelled, clash, intern, key_written, same, spell, spell_typed, spelling,
};

/// A search: the words typed, the key they spell if they spell one, and
/// a chord pressed to find what it does.
pub(crate) struct Search {
    words: String,
    typed: Option<Spelled>,
    found: Option<Spelled>,
}

impl Search {
    /// The search for `text` typed and the chord `found` pressed, if one
    /// was.
    pub(crate) fn new(text: &str, found: Option<&'static str>) -> Self {
        let words = text.trim().to_owned();
        Search {
            typed: spell_typed(&words),
            words,
            found: found.and_then(|keys| spell(keys).ok()),
        }
    }

    /// Whether a command with this name and group heading, these keys (as
    /// spelled) and these caps (as the look writes them) stays listed.
    pub(crate) fn keeps(
        &self,
        layout: Layout,
        (name, heading): (&str, &str),
        keys: &[&'static str],
        caps: &[&str],
    ) -> bool {
        if let Some(found) = &self.found
            && !keys
                .iter()
                .filter_map(|keys| spell(keys).ok())
                .any(|keys| is(layout, &keys, found))
        {
            return false;
        }
        self.words.is_empty()
            || find_folded(name, &self.words).is_some()
            || find_folded(heading, &self.words).is_some()
            || self.cap_hit(layout, keys, caps).is_some()
    }

    /// Which of `caps` the words name: one written as they are typed, or
    /// the one of the key they spell.
    pub(crate) fn cap_hit(
        &self,
        layout: Layout,
        keys: &[&'static str],
        caps: &[&str],
    ) -> Option<usize> {
        if self.words.is_empty() {
            return None;
        }
        let typed = fold(&self.words);
        if let Some(at) = caps.iter().position(|cap| fold(cap) == typed) {
            return Some(at);
        }
        let spelled = self.typed.as_ref()?;
        let key = keys.iter().copied().find(|keys| {
            spell(keys)
                .ok()
                .is_some_and(|keys| is(layout, &keys, spelled))
        })?;
        let written = key_written(layout, key);
        // A cap that writes several keys together (`j/k`) holds it.
        Some(
            caps.iter()
                .position(|cap| cap.contains(written.as_str()))
                .unwrap_or(0),
        )
    }

    /// Where the words are in a command's name, to mark them.
    pub(crate) fn name_spans(&self, name: &str) -> Vec<Range<usize>> {
        if self.words.is_empty() {
            return Vec::new();
        }
        find_folded(name, &self.words).into_iter().collect()
    }
}

/// Whether `keys` is the key `wanted`: a range of digits holds each digit.
fn is(layout: Layout, keys: &Spelled, wanted: &Spelled) -> bool {
    match keys {
        Spelled::Digits(_) => clash(layout, keys, wanted),
        _ => same(layout, keys, wanted),
    }
}

/// A key's text without its case, its `+`, `-` and spaces alike: `Ctrl+R`,
/// `ctrl-r` and `ctrl r` are one.
fn fold(text: &str) -> String {
    let mut folded = String::new();
    let mut joined = false;
    for char in text.trim().chars() {
        if char == '+' || char == '-' || char.is_whitespace() {
            if !joined {
                folded.push('+');
                joined = true;
            }
        } else {
            folded.extend(char.to_lowercase());
            joined = false;
        }
    }
    folded
}

/// Where `needle` is in `hay`, in any case: its byte range in `hay`.
pub(crate) fn find_folded(hay: &str, needle: &str) -> Option<Range<usize>> {
    let wanted: Vec<char> = needle.chars().flat_map(char::to_lowercase).collect();
    if wanted.is_empty() {
        return None;
    }
    for (start, _) in hay.char_indices() {
        let mut left = wanted.as_slice();
        for (at, char) in hay[start..].char_indices() {
            let lower: Vec<char> = char.to_lowercase().collect();
            if !left.starts_with(&lower) {
                break;
            }
            left = &left[lower.len()..];
            if left.is_empty() {
                return Some(start..start + at + char.len_utf8());
            }
        }
    }
    None
}

/// What is held, as `layout` spells it: on macOS Cmd and Control apart,
/// on Windows one Ctrl for both (the keymap's `cmd`), on Omarchy Ctrl.
pub(crate) fn held(layout: Layout, modifiers: Modifiers) -> Mods {
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

/// The chord the frame's first press with Cmd, Ctrl or Alt held makes,
/// taken from the frame: what a search field reads as a key to find, in
/// place of typing it. None, and nothing taken, where there is none, or
/// where it is the field's own (the system's or the field's text editing).
pub(crate) fn find_chord(ctx: &egui::Context, layout: Layout) -> Option<&'static str> {
    let mac = ctx.os() == egui::os::OperatingSystem::Mac;
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
        ) || edits_text(key, modifiers, mac)
        {
            return None;
        }
        let keys = intern(&spelling(held(layout, modifiers), key)?);
        let spelled = spell(keys).ok()?;
        if crate::keymap::reserved_by(layout, &spelled).is_some() {
            return None;
        }
        input.events.remove(at);
        // What an Alt chord types (Alt+R's `®`) is taken with it: the field
        // would take it as words, which drop the chord. A chord with Cmd
        // or Ctrl types nothing, so text after one is the person's own.
        let types = modifiers.alt && !modifiers.command && !modifiers.ctrl && !modifiers.mac_cmd;
        if types && let Some(Event::Text(_)) = input.events.get(at) {
            input.events.remove(at);
        }
        Some(keys)
    })
}

/// Whether the field does something with this chord in its text: select
/// all, undo and redo, the Ctrl deletes, and on macOS Control's moves.
fn edits_text(key: Key, modifiers: Modifiers, mac: bool) -> bool {
    let command = matches!(key, Key::A | Key::Z | Key::Y) && modifiers.command;
    let deletes = matches!(key, Key::H | Key::K | Key::U | Key::W) && modifiers.ctrl;
    let moves = mac
        && modifiers.ctrl
        && !modifiers.shift
        && matches!(key, Key::P | Key::N | Key::B | Key::F | Key::A | Key::E);
    command || deletes || moves
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn words_are_found_in_any_case_where_they_are() {
        assert_eq!(find_folded("Add row above", "ROW"), Some(4..7));
        assert_eq!(find_folded("Größe ändern", "ÄN"), Some(8..11));
        assert_eq!(find_folded("Reload", "x"), None);
        assert_eq!(find_folded("Reload", ""), None);
    }

    #[test]
    fn a_command_stays_by_its_name_its_group_or_its_keys() {
        let mac = Layout::Mac;
        let keeps = |text: &str, found| {
            Search::new(text, found).keeps(mac, ("Reload", "Navigation"), &["cmd+r"], &["⌘R"])
        };
        for text in ["", "rel", "RELOAD", "navig", "⌘r", "⌘R", "cmd+r", "cmd r"] {
            assert!(keeps(text, None), "{text}");
        }
        for text in ["cmd+t", "row", "zzz", "r+"] {
            assert!(!keeps(text, None), "{text}");
        }
        assert!(keeps("", Some("cmd+r")));
        assert!(!keeps("", Some("cmd+t")));
        assert!(!keeps("row", Some("cmd+r")));
        // Windows writes its keys in words: they are matched as written.
        for text in ["ctrl+r", "ctrl-r", "Ctrl R"] {
            let search = Search::new(text, None);
            let caps = ["Ctrl+R"];
            let kept = search.keeps(Layout::Windows, ("Reload", "Navigation"), &["cmd+r"], &caps);
            assert!(kept, "{text}");
        }
        // A range of digits holds each of them.
        let digit = Search::new("", Some("cmd+3"));
        let name = ("Go to connection window", "Window and connections");
        assert!(digit.keeps(mac, name, &["cmd+1..9"], &["⌘1…9"]));
    }

    #[test]
    fn the_keys_typed_mark_their_cap_and_the_words_their_name() {
        let caps = ["u", "ctrl+r"];
        let keys = ["u", "ctrl+r"];
        let omarchy = Layout::Omarchy;
        let hit = |text| Search::new(text, None).cap_hit(omarchy, &keys, &caps);
        assert_eq!(hit("ctrl+r"), Some(1));
        assert_eq!(hit("ctrl r"), Some(1));
        assert_eq!(hit("redo"), None);
        let mac = Search::new("cmd+r", None);
        assert_eq!(mac.cap_hit(Layout::Mac, &["cmd+r"], &["⌘R"]), Some(0));
        assert_eq!(Search::new("rel", None).name_spans("Reload"), vec![0..3]);
        assert!(Search::new("", None).name_spans("Reload").is_empty());
    }
}

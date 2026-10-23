//! A menu item's key, how an item was chosen, and the characters AppKit
//! matches a key equivalent by.

/// The chord a menu shows beside an item and, on macOS, takes as its key
/// equivalent: the app's keymap's chord, field for field.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Shortcut {
    pub key: egui::Key,
    /// Cmd on macOS, Ctrl on Windows.
    pub cmd: bool,
    /// The Control key on macOS.
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
}

/// How an item was chosen.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Via {
    /// A click, Return in an open menu, the Help menu's search, a screen
    /// reader: anything but the item's own key equivalent.
    Click,
    /// The item's key equivalent, which AppKit took from the window.
    Key,
}

/// The key-down AppKit was handling when an item was chosen.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeyDown {
    /// `charactersIgnoringModifiers`: Shift still applies to it.
    pub chars: String,
    pub cmd: bool,
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
}

/// Whether `down` is the key equivalent of an item whose key is `key`.
/// Only a press holding Cmd or Ctrl can be one (every menu key does), with
/// the same modifiers exactly, and, without Shift, the same character:
/// with Shift the character is the shifted one, which depends on the
/// keyboard, so the modifiers decide.
pub fn via_of(down: Option<&KeyDown>, key: Option<Shortcut>) -> Via {
    let (Some(down), Some(key)) = (down, key) else {
        return Via::Click;
    };
    let held = down.cmd == key.cmd
        && down.ctrl == key.ctrl
        && down.alt == key.alt
        && down.shift == key.shift;
    let same =
        key.shift || character(key.key).is_some_and(|char| down.chars.eq_ignore_ascii_case(&char));
    if (key.cmd || key.ctrl) && held && same {
        Via::Key
    } else {
        Via::Click
    }
}

/// The character AppKit gives for `key` in a key equivalent and in
/// `charactersIgnoringModifiers`: a letter in lower case, a digit or a
/// mark as a US keyboard types it, and AppKit's own for the named keys
/// (`NSUpArrowFunctionKey` and its kin are U+F700 on).
pub(crate) fn character(key: egui::Key) -> Option<String> {
    use egui::Key as K;
    let named = match key {
        K::Enter => "\r",
        K::Tab => "\t",
        K::Escape => "\u{1b}",
        K::Backspace => "\u{7f}",
        K::Space => " ",
        // egui's name for it is the minus sign, U+2212.
        K::Minus => "-",
        K::ArrowUp => "\u{F700}",
        K::ArrowDown => "\u{F701}",
        K::ArrowLeft => "\u{F702}",
        K::ArrowRight => "\u{F703}",
        K::F1 => "\u{F704}",
        K::F2 => "\u{F705}",
        K::F3 => "\u{F706}",
        K::F4 => "\u{F707}",
        K::F5 => "\u{F708}",
        K::F6 => "\u{F709}",
        K::F7 => "\u{F70A}",
        K::F8 => "\u{F70B}",
        K::F9 => "\u{F70C}",
        K::F10 => "\u{F70D}",
        K::F11 => "\u{F70E}",
        K::F12 => "\u{F70F}",
        K::Delete => "\u{F728}",
        K::Home => "\u{F729}",
        K::End => "\u{F72B}",
        K::PageUp => "\u{F72C}",
        K::PageDown => "\u{F72D}",
        other => {
            let name = other.symbol_or_name();
            let mut chars = name.chars();
            return match (chars.next(), chars.next()) {
                (Some(char), None) => Some(char.to_ascii_lowercase().to_string()),
                _ => None,
            };
        }
    };
    Some(named.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cmd(key: egui::Key) -> Shortcut {
        Shortcut {
            key,
            cmd: true,
            ctrl: false,
            alt: false,
            shift: false,
        }
    }

    fn down(chars: &str, cmd: bool, ctrl: bool, shift: bool) -> KeyDown {
        KeyDown {
            chars: chars.to_owned(),
            cmd,
            ctrl,
            alt: false,
            shift,
        }
    }

    #[test]
    fn an_item_s_own_key_equivalent_is_a_key() {
        let history = cmd(egui::Key::Y);
        assert_eq!(
            via_of(Some(&down("y", true, false, false)), Some(history)),
            Via::Key
        );
        // AppKit hands the character as typed: Cmd keeps it lower case.
        assert_eq!(
            via_of(Some(&down("Y", true, false, false)), Some(history)),
            Via::Key
        );
    }

    #[test]
    fn every_other_choice_is_a_click() {
        let history = cmd(egui::Key::Y);
        // No event: a click, or a test.
        assert_eq!(via_of(None, Some(history)), Via::Click);
        // Return in an open menu, the Help menu's search, Ctrl+F2 and on.
        assert_eq!(
            via_of(Some(&down("\r", false, false, false)), Some(history)),
            Via::Click
        );
        // Another chord.
        assert_eq!(
            via_of(Some(&down("j", true, false, false)), Some(history)),
            Via::Click
        );
        // The right letter with other modifiers.
        assert_eq!(
            via_of(Some(&down("y", true, true, false)), Some(history)),
            Via::Click
        );
        // An item with no key is always clicked.
        assert_eq!(
            via_of(Some(&down("\r", false, false, false)), None),
            Via::Click
        );
    }

    #[test]
    fn a_shifted_chord_is_matched_by_what_is_held() {
        // A US keyboard gives "{" for Shift+[: the character is the
        // shifted one, so only the modifiers are compared.
        let previous_tab = Shortcut {
            shift: true,
            ..cmd(egui::Key::OpenBracket)
        };
        assert_eq!(
            via_of(Some(&down("{", true, false, true)), Some(previous_tab)),
            Via::Key
        );
    }

    #[test]
    fn appkit_s_characters_for_named_keys() {
        assert_eq!(character(egui::Key::Tab).as_deref(), Some("\t"));
        assert_eq!(character(egui::Key::Enter).as_deref(), Some("\r"));
        assert_eq!(character(egui::Key::ArrowUp).as_deref(), Some("\u{F700}"));
        assert_eq!(character(egui::Key::A).as_deref(), Some("a"));
        assert_eq!(character(egui::Key::OpenBracket).as_deref(), Some("["));
        assert_eq!(character(egui::Key::Num1).as_deref(), Some("1"));
        // egui names Minus with U+2212; AppKit types the hyphen.
        assert_eq!(character(egui::Key::Minus).as_deref(), Some("-"));
    }
}

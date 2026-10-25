//! A menu choice made by a key, or a clicked Edit item, put into the
//! frame's input as the events a real press of its chord gives egui, so
//! `keys.rs` and the text fields read it as they read the key.
//!
//! egui decides what is a repeat from the keys it holds down, and lets a
//! key go only on a release; egui 0.36 takes what is held from
//! `ModifiersChanged` events, the last in a frame winning.

use egui::{Event, Modifiers};
use tabletist_menubar::Via;

use crate::keymap::{MenuKey, Mods};

/// Menu choices, waiting to become a frame's input.
#[derive(Debug, Default)]
pub struct Replay {
    /// What winit last said is held.
    real: Modifiers,
    /// A frame held a chord's modifiers (a click, or Copy and Cut however
    /// chosen): give back `real` next frame.
    restore: bool,
}

impl Replay {
    /// Adds `presses` to a frame's `events`, which hold what winit sent.
    /// Gives whether a restore is pending: then the caller asks for
    /// another frame (`request_repaint`), so what is really held comes
    /// back at once rather than with the next input.
    /// Whether a restore waits for the next frame.
    pub fn restoring(&self) -> bool {
        self.restore
    }

    pub fn frame(&mut self, events: &mut Vec<Event>, presses: &[(MenuKey, Via)]) -> bool {
        // egui lets everything go when the window loses focus.
        let told = events.iter().rev().find_map(|event| match event {
            Event::ModifiersChanged(held) => Some(*held),
            Event::WindowFocused(false) => Some(Modifiers::NONE),
            _ => None,
        });
        if let Some(held) = told {
            self.real = held;
        }
        if std::mem::take(&mut self.restore) && told.is_none() {
            events.insert(0, Event::ModifiersChanged(self.real));
        }
        for &(key, via) in presses {
            let held = modifiers(key.mods);
            match clipboard(key) {
                // The window's own clipboard event, read with what the
                // frame holds: hold the chord's for this frame.
                Some(event) => {
                    events.push(Event::ModifiersChanged(held));
                    events.push(event);
                    self.restore = true;
                }
                None => {
                    let press = Event::Key {
                        key: key.key,
                        physical_key: None,
                        pressed: true,
                        repeat: false,
                        modifiers: held,
                    };
                    match via {
                        // The key is down and winit sends its release:
                        // the press goes before the frame's first end of
                        // it (its release, or a focus loss, which lets
                        // every key go), so egui does not keep it down.
                        // A second press of the key in one frame goes
                        // there too and is a repeat: AppKit sends a held
                        // chord's autorepeats as more choices, which must
                        // stay repeats, and two taps in one frame are
                        // rarer than that.
                        Via::Key => {
                            let at = events
                                .iter()
                                .position(|event| ends(event, key.key))
                                .unwrap_or(events.len());
                            events.insert(at, press);
                        }
                        // Nothing is down and no release will come.
                        Via::Click => {
                            events.push(Event::ModifiersChanged(held));
                            events.push(press);
                            events.push(Event::Key {
                                key: key.key,
                                physical_key: None,
                                pressed: false,
                                repeat: false,
                                modifiers: held,
                            });
                            self.restore = true;
                        }
                    }
                }
            }
        }
        self.restore
    }
}

/// What egui-winit makes of Cmd+C and Cmd+X (Ctrl on Windows): a copy
/// and a cut, not a key. Paste needs the clipboard's text, which the
/// window reads (`ViewportCommand::RequestPaste`), so it is never
/// replayed.
fn clipboard(key: MenuKey) -> Option<Event> {
    let command = key.mods.cmd && !key.mods.ctrl && !key.mods.alt && !key.mods.shift;
    match key.key {
        egui::Key::C if command => Some(Event::Copy),
        egui::Key::X if command => Some(Event::Cut),
        _ => None,
    }
}

/// Whether `event` lets `key` go: its release, or the window losing focus.
fn ends(event: &Event, key: egui::Key) -> bool {
    match event {
        Event::Key {
            key: up,
            pressed: false,
            ..
        } => *up == key,
        Event::WindowFocused(focused) => !focused,
        _ => false,
    }
}

/// The modifiers egui-winit reports for `mods` on this platform:
/// `command` and `mac_cmd` on macOS, `command` and `ctrl` elsewhere.
/// `keys.rs`'s `held_is` reads these.
pub(crate) fn modifiers(mods: Mods) -> Modifiers {
    let mac = cfg!(target_os = "macos");
    let command = mods.cmd || (!mac && mods.ctrl);
    Modifiers {
        alt: mods.alt,
        shift: mods.shift,
        ctrl: if mac { mods.ctrl } else { command },
        mac_cmd: mac && mods.cmd,
        command,
    }
}

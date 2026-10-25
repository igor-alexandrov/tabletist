# A native menu bar on macOS and Windows Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A native menu bar on macOS and Windows, built with muda from one
model, holding Quick Open's commands and the platform's own items, with
the keymap's keys. The spec is
`docs/superpowers/specs/2026-10-23-native-menu-bar-design.md`; read it
first.

**Architecture:** `src/menu.rs` holds the model (which items, in which
menus, with which titles) and, in `src/app/menu.rs`, which items are
enabled and what a click does. `src/menu/bar.rs` keeps the live bar in
step with the app each frame, and `src/menu/replay.rs` gives egui back a
key AppKit took for a menu item. `crates/tabletist-menubar` turns the
description into muda menus and routes muda's one event handler. The
keymap gains `MenuKey`, `EditItem`, `menu_key` and `edit_key`, so no key is
spelled outside it.

**Tech stack:** Rust 2024, egui 0.36 and eframe (the crmne fork), winit
0.30 (the fork, `macos-quit-as-close`), muda 0.21 with
`default-features = false`, objc2-app-kit 0.3. Two pull requests: macOS
(Tasks 1 to 8), then Windows (Tasks 9 to 11).

---

## Before Task 1

Work on a branch from `main` with the spec commit on it (this worktree's
branch has it). Run the four checks of `AGENTS.md` once to see them pass
before anything changes:

    cargo fmt --all --check
    cargo clippy --locked --workspace --all-targets -- -D warnings
    cargo test --locked --workspace --all-targets
    RUSTDOCFLAGS='-D warnings' cargo doc --locked --workspace --no-deps

Every task ends with all four passing and one commit. Commits are signed
(1Password must be unlocked); never commit unsigned.

## Departures from the spec, for a yes or a no before code

1. **winit's default menu stays on.** The spec turns it off with
   `with_default_menu(false)`. Kept on, it is what the app has if muda
   fails to attach (Quit, Hide, the Window menu), and muda's
   `init_for_nsapp` replaces it with `setMainMenu` once the window exists.
   winit sets its menu once, at launch, before eframe calls the app's
   creation closure, so nothing puts it back.
2. **A menu key needs Cmd or Ctrl.** `menu_key` gives `None` for a chord
   with neither: AppKit would take a typing key from every text field,
   and a replay carries no `Event::Text`. No `RUNNABLE` command has such a
   chord on the Mac today; the rule keeps it so.
3. **`MenuKey` is the keymap's own `Mods` and `Key`.** The spec drew five
   booleans; `Mods` already is four of them.
4. **Titles are set once, and again only for the toggles.** English is
   the only locale (`src/i18n.rs`), so the bar is never retitled for a
   locale change.
5. **The Windows test builds the bar without attaching it.** A hidden
   window needs a winit event loop or raw Win32 calls in a test. The test
   checks the bar's items and the routing of a choice; attaching is
   checked by running the app on Windows (listed under "Needs eyes").
6. **`Entry::Native` carries its title, and menus a role.** Hide and Quit
   are titled "Hide Tabletist" and "Quit Tabletist", and the Window and
   Help menus are registered with NSApp; the crate's types say both.
7. **Keys change by rebuilding the bar, not `set_shortcut`.** muda cannot
   take a key equivalent off an item on macOS (`set_accelerator(None)`
   writes nothing there), so after an unbind AppKit would still take the
   old key. When the keymap or the look changes, the bar is built again
   and put in place (`MenuBar::rebuild`).
8. **The window's commands answer without a workspace.** `run_of` starts
   with `self.workspace(tab)?`, and the connection picker has none, so at
   a launch with nothing restored every item would be greyed. Settings,
   Keyboard Shortcuts, New Connection, Connections and the three text
   sizes need no connection: `run_of` answers them before it looks for
   one (Task 5).
9. **Under a dialog, a key the keymap would not read there is a click.**
   Settings is enabled over the shortcuts dialog, so AppKit takes `⌘,`
   there, but `keys::handle` does not run under a dialog: replayed, the
   key would do nothing. Such a choice runs as a click (`menu::handling`,
   Task 6).
10. **On macOS an item is greyed only while its menu is open.** Task 3's
    AppKit test showed that NSMenu takes a disabled item's key equivalent
    and performs nothing, so the key never reaches the window. The crate
    keeps every item enabled for AppKit and stores the state the app
    wants; an `NSMenuDelegate` on each menu applies it in `menuWillOpen:`
    and enables the items again in `menuDidClose:`. Every key then reaches
    the app (and is replayed), and an open menu greys what does not
    apply. The delegate class needs `unsafe` (defining an Objective-C
    class), in one module of `tabletist-menubar` with SAFETY notes.
    Windows greys as usual: it installs no accelerator table.
11. **On macOS a chord with Shift and a mark shows no key.** The same
    test showed AppKit does not match `[` with Shift against the `{` a
    keyboard sends, so ⇧⌘[ and ⇧⌘] (Previous and Next Tab) get no key
    equivalent; the keys still work through the keymap. The crate drops
    such a key when it builds an item.

## Files

| File | What it holds |
|---|---|
| `src/keymap.rs` (modify) | `EditItem`, `MenuKey`, `Keymap::menu_key`, `Keymap::edit_key`, tests |
| `src/keymap/overrides.rs` (modify, tests only) | an override changes a menu key |
| `crates/tabletist-menubar/Cargo.toml` (create) | the crate, muda for macOS (then Windows) only |
| `crates/tabletist-menubar/src/lib.rs` (create) | `Menu`, `Entry`, `Native`, `Role`, `MenuBar<T>` |
| `crates/tabletist-menubar/src/keys.rs` (create) | `Shortcut`, `Via`, `KeyDown`, `via_of`, AppKit's characters, unit tests |
| `crates/tabletist-menubar/src/native.rs` (create) | the muda side: build, route, set, chosen |
| `crates/tabletist-menubar/src/macos.rs` (create) | the key-down AppKit was handling |
| `crates/tabletist-menubar/src/windows.rs` (create, Task 9) | `init_for_hwnd`, the crate's one `unsafe` |
| `crates/tabletist-menubar/tests/macos_menu.rs` (create) | harness-less AppKit test |
| `src/menu.rs` (create) | `Platform`, `Item`, `Menu`, `bar`, `title`, `key_of`, `Handling`, `handling` |
| `src/menu/replay.rs` (create) | `Replay`: menu choices into a frame's input |
| `src/menu/bar.rs` (create) | `Bar`: the live bar, synced each frame |
| `src/menu/tests.rs` (create) | the model, the state, clicks and replays, headless |
| `src/app/menu.rs` (create) | `App::menu_state`, `App::choose` |
| `src/app.rs` (modify) | `mod menu;` |
| `src/lib.rs` (modify) | `pub mod menu;` beside `pub mod macos;` |
| `src/entrypoint.rs` (modify) | attach the bar, `raw_input_hook`, clicks, sync; old menus out |
| `src/macos.rs` (modify) | the four `*Menu` wrappers out, doc comment |
| `crates/tabletist-appkit/*` (modify) | `about.rs`, `settings.rs`, `view.rs`, `target.rs` and their tests out |
| `Cargo.toml` (modify) | workspace member, the crate's dependency line, appkit's comment |
| `AGENTS.md`, `docs/_guide/macos.md`, `docs/_reference/keyboard-shortcuts.md` (modify) | Task 8 |

`src/lib.rs` declares the crate's modules (`pub mod macos;` is at line
21, behind its `cfg`): add `pub mod menu;` in order, not behind a `cfg`
(the model is tested everywhere).

---

## Part 1: macOS

### Task 1: The keymap's menu keys

**Files:**
- Modify: `src/keymap.rs` (beside `pub struct Keymap`, and its `mod tests`)
- Modify (tests): `src/keymap/overrides.rs`

- [ ] **Step 1: Write the failing tests**

In `src/keymap.rs`'s `mod tests`:

```rust
#[test]
fn a_menu_shows_a_command_s_one_chord() {
    let keymap = Keymap::default();
    let held = |cmd, shift| Mods {
        cmd,
        shift,
        ..Mods::default()
    };
    // One line, one chord.
    assert_eq!(
        keymap.menu_key(Command::History, Layout::Mac),
        Some(MenuKey { mods: held(true, false), key: Key::Y })
    );
    // Two lines with the same chord are one chord.
    let format = keymap.menu_key(Command::Format, Layout::Mac);
    assert!(format.is_some(), "Format SQL shows its chord");
    // No key on the Mac.
    assert_eq!(keymap.menu_key(Command::SwitchDatabase, Layout::Mac), None);
    // A typed character, not a chord.
    assert_eq!(keymap.menu_key(Command::Help, Layout::Mac), None);
}

#[test]
fn a_menu_key_holds_cmd_or_ctrl_and_is_one_press() {
    let keymap = Keymap::default();
    for layout in [Layout::Mac, Layout::Windows] {
        for command in crate::app::RUNNABLE {
            if let Some(key) = keymap.menu_key(command, layout) {
                assert!(key.mods.cmd || key.mods.ctrl, "{command:?}: {key:?}");
                // It is one of the command's own chords, spelled as one
                // press.
                let own = keymap
                    .of(command)
                    .flat_map(|binding| binding.bound(layout).chords.iter())
                    .filter(|chord| chord.kind == Kind::Own)
                    .any(|chord| {
                        spell(chord.keys).is_ok_and(|spelled| {
                            spelled == Spelled::Strokes(vec![Stroke::Key { mods: key.mods, key: key.key }])
                        })
                    });
                assert!(own, "{command:?}");
            }
        }
    }
}

#[test]
fn the_edit_items_keys_are_the_ones_the_system_keeps() {
    let keymap = Keymap::default();
    let cmd = Mods {
        cmd: true,
        ..Mods::default()
    };
    assert_eq!(
        keymap.edit_key(EditItem::Copy, Layout::Mac),
        Some(MenuKey { mods: cmd, key: Key::C })
    );
    assert_eq!(
        keymap.edit_key(EditItem::Redo, Layout::Windows),
        Some(MenuKey {
            mods: Mods { shift: true, ..cmd },
            key: Key::Z
        })
    );
    for item in EditItem::ALL {
        assert!(keymap.edit_key(item, Layout::Mac).is_some(), "{item:?}");
        // Omarchy keeps none: Super is Hyprland's.
        assert_eq!(keymap.edit_key(item, Layout::Omarchy), None);
    }
}
```

`crate::app::RUNNABLE` is `pub const` in `src/app/commands.rs`; if
`app::commands` does not re-export it at `crate::app`, write the path it
has (`grep -rn "pub use commands" src/app.rs`).

In `src/keymap/overrides.rs`'s tests, beside
`recording_gives_a_row_a_key_and_its_default_back_writes_nothing`:

```rust
#[test]
fn a_recorded_key_is_the_menu_s_key() {
    // The sidebar's toggle has one line (GLOBAL), so the recorded chord is
    // its only one. (Format would not do: its large editor's line keeps
    // its own chord, and two chords show none.)
    let mac = Layout::Mac;
    let sidebar = row(Command::ToggleSidebar, Scope::Global);
    let recorded = Overrides::default().record(mac, sidebar, "cmd+alt+b");
    let (keymap, refused) = Keymap::with(mac, &recorded);
    assert!(refused.is_empty(), "{refused:?}");
    assert_eq!(
        keymap.menu_key(Command::ToggleSidebar, mac),
        Some(crate::keymap::MenuKey {
            mods: crate::keymap::Mods {
                cmd: true,
                alt: true,
                ..Default::default()
            },
            key: egui::Key::B,
        })
    );
}
```

If `cmd+alt+b` is refused (another command holds it), pick a chord the
keymap leaves free; the assert on `refused` says so. Check first that
`ToggleSidebar` has one line on the Mac (`grep -n "C::ToggleSidebar"
src/keymap.rs`).

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test --locked -p tabletist -- a_menu_shows a_menu_key_holds edit_items a_recorded_key`
Expected: compile errors (`MenuKey`, `EditItem`, `menu_key`, `edit_key` do
not exist).

- [ ] **Step 3: Write the keymap's part**

In `src/keymap.rs`, after `impl Default for Keymap`:

```rust
/// An item of the Edit menu: the text editing every field reads, whose
/// keys the system keeps (`reserved`), so no command has them.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum EditItem {
    Undo,
    Redo,
    Cut,
    Copy,
    Paste,
    SelectAll,
}

impl EditItem {
    pub const ALL: [EditItem; 6] = [
        Self::Undo,
        Self::Redo,
        Self::Cut,
        Self::Copy,
        Self::Paste,
        Self::SelectAll,
    ];

    /// Its chord, one of `reserved`'s.
    fn keys(self) -> &'static str {
        match self {
            Self::Undo => "cmd+z",
            Self::Redo => "cmd+shift+z",
            Self::Cut => "cmd+x",
            Self::Copy => "cmd+c",
            Self::Paste => "cmd+v",
            Self::SelectAll => "cmd+a",
        }
    }
}

/// One chord as a menu shows it beside an item.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct MenuKey {
    pub mods: Mods,
    pub key: Key,
}

/// `keys` as one press, if it is one.
fn one_press(keys: &'static str) -> Option<MenuKey> {
    let Ok(Spelled::Strokes(strokes)) = spell(keys) else {
        return None;
    };
    let [Stroke::Key { mods, key }] = strokes.as_slice() else {
        return None;
    };
    Some(MenuKey {
        mods: *mods,
        key: *key,
    })
}
```

And in `impl Keymap` (the first one, beside `of`):

```rust
    /// The chord a menu shows for `command` in `layout`: the one its own
    /// keys share. None where it has no key, more than one, a key that is
    /// no single press (a sequence, a count, a line of the prompt, a typed
    /// character), or a press with neither Cmd nor Ctrl held: AppKit would
    /// take that from every text field, and a key it gives back carries no
    /// text.
    pub fn menu_key(&self, command: Command, layout: Layout) -> Option<MenuKey> {
        if !command.info().built {
            return None;
        }
        let mut found: Option<MenuKey> = None;
        for binding in self.of(command) {
            for chord in binding.bound(layout).chords {
                if chord.kind != Kind::Own {
                    continue;
                }
                let key = one_press(chord.keys)?;
                match found {
                    Some(seen) if seen != key => return None,
                    _ => found = Some(key),
                }
            }
        }
        found.filter(|key| key.mods.cmd || key.mods.ctrl)
    }

    /// The chord of an Edit item in `layout`, where the system keeps one.
    pub fn edit_key(&self, item: EditItem, layout: Layout) -> Option<MenuKey> {
        let keys = item.keys();
        reserved(layout)
            .contains(&keys)
            .then(|| one_press(keys))
            .flatten()
    }
```

`edit_key` takes `&self` though it reads only `reserved`: the Edit keys
are the keymap's, and a later override of them would land here.

- [ ] **Step 4: Run the tests**

Run: `cargo test --locked -p tabletist -- a_menu_shows a_menu_key_holds edit_items a_recorded_key`
Expected: PASS. Then the four checks.

- [ ] **Step 5: Commit**

```bash
git add src/keymap.rs src/keymap/overrides.rs
git commit -m "Give the keymap the one chord a menu can show for a command"
```

### Task 2: The crate's keys

**Files:**
- Create: `crates/tabletist-menubar/Cargo.toml`, `crates/tabletist-menubar/src/lib.rs`, `crates/tabletist-menubar/src/keys.rs`
- Modify: `Cargo.toml` (workspace `members`)

- [ ] **Step 1: The crate, empty but for its keys**

`crates/tabletist-menubar/Cargo.toml`:

```toml
[package]
name = "tabletist-menubar"
version = "0.1.0"
description = "Tabletist's native menu bar on macOS and Windows, built with muda"
edition.workspace = true
rust-version.workspace = true
publish = false

[lints]
workspace = true

[dependencies]
# `egui::Key`, the key a `Shortcut` names: the app's keymap reads keys as
# egui does. The crate draws nothing.
egui = "0.36"
# The window the bar attaches to on Windows; eframe already builds it.
raw-window-handle = "0.6"
log = "0.4"
```

Copy `log`'s version from the root `Cargo.toml` so the lock adds nothing.
Add `"crates/tabletist-menubar"` to `[workspace] members` in the root
`Cargo.toml`, in order after `tabletist-appkit`. A new member changes
`Cargo.lock`, so run `cargo check --workspace` once without `--locked`
to write it; the `--locked` checks pass from then on. Do the same in
Task 3 after adding muda.

`crates/tabletist-menubar/src/lib.rs`:

```rust
//! Tabletist's menu bar on macOS and Windows, built with muda from a
//! description the app gives: menus of items, each with the app's id and
//! key, and the platform's own items. Off those platforms there is none,
//! and attaching one gives `None`.

mod keys;

pub use keys::{KeyDown, Shortcut, Via, via_of};
```

- [ ] **Step 2: Write the failing tests**

`crates/tabletist-menubar/src/keys.rs`, tests first (the module's code
comes in Step 4):

```rust
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
        assert_eq!(via_of(Some(&down("y", true, false, false)), Some(history)), Via::Key);
        // AppKit hands the character as typed: Cmd keeps it lower case.
        assert_eq!(via_of(Some(&down("Y", true, false, false)), Some(history)), Via::Key);
    }

    #[test]
    fn every_other_choice_is_a_click() {
        let history = cmd(egui::Key::Y);
        // No event: a click, or a test.
        assert_eq!(via_of(None, Some(history)), Via::Click);
        // Return in an open menu, the Help menu's search, Ctrl+F2 and on.
        assert_eq!(via_of(Some(&down("\r", false, false, false)), Some(history)), Via::Click);
        // Another chord.
        assert_eq!(via_of(Some(&down("j", true, false, false)), Some(history)), Via::Click);
        // The right letter with other modifiers.
        assert_eq!(via_of(Some(&down("y", true, true, false)), Some(history)), Via::Click);
        // An item with no key is always clicked.
        assert_eq!(via_of(Some(&down("\r", false, false, false)), None), Via::Click);
    }

    #[test]
    fn a_shifted_chord_is_matched_by_what_is_held() {
        // A US keyboard gives "{" for Shift+[: the character is the
        // shifted one, so only the modifiers are compared.
        let previous_tab = Shortcut {
            shift: true,
            ..cmd(egui::Key::OpenBracket)
        };
        assert_eq!(via_of(Some(&down("{", true, false, true)), Some(previous_tab)), Via::Key);
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
```

- [ ] **Step 3: Run them to see them fail**

Run: `cargo test --locked -p tabletist-menubar`
Expected: compile errors.

- [ ] **Step 4: Write the keys**

Above the tests in `keys.rs`:

```rust
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
    let same = key.shift
        || character(key.key).is_some_and(|char| down.chars.eq_ignore_ascii_case(&char));
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
```

Check `egui::Key::symbol_or_name` gives `"["` for `OpenBracket` and `"1"`
for `Num1` in the fork (`grep -n "fn symbol_or_name" -A30
~/.cargo/git/checkouts/egui-*/6ce633b/crates/egui/src/data/key.rs`); the
test above says so if not. This table is AppKit's characters, not a key
binding: `tests/keys.rs` reads only `src/`, so it does not see it.

- [ ] **Step 5: Run the tests, then the four checks**

Run: `cargo test --locked -p tabletist-menubar`
Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add Cargo.toml Cargo.lock crates/tabletist-menubar
git commit -m "Start tabletist-menubar with a menu item's key and how it was chosen"
```

### Task 3: The crate's bar on macOS

**Files:**
- Modify: `crates/tabletist-menubar/Cargo.toml`, `crates/tabletist-menubar/src/lib.rs`
- Create: `crates/tabletist-menubar/src/native.rs`, `crates/tabletist-menubar/src/macos.rs`, `crates/tabletist-menubar/tests/macos_menu.rs`

- [ ] **Step 1: Dependencies**

Append to the crate's `Cargo.toml`:

```toml
# The menu bar itself, on macOS (and Windows, from Task 9). Its GTK crates
# are optional and Linux-only: with default features off, Linux builds
# neither muda nor GTK. It adds keyboard-types and crossbeam-channel;
# its objc2 crates are the ones winit builds.
[target.'cfg(target_os = "macos")'.dependencies]
muda = { version = "0.21", default-features = false }
# The event AppKit was handling when an item was chosen (`macos.rs`):
# safe calls, the versions winit and tabletist-appkit build.
objc2 = "0.6"
objc2-foundation = { version = "0.3", default-features = false, features = ["std", "NSString"] }
objc2-app-kit = { version = "0.3", default-features = false, features = ["std", "NSApplication", "NSEvent", "NSResponder"] }

[dev-dependencies]
raw-window-handle = "0.6"

[target.'cfg(target_os = "macos")'.dev-dependencies]
objc2 = "0.6"
objc2-foundation = { version = "0.3", default-features = false, features = ["std", "NSString", "NSArray", "NSGeometry"] }
objc2-app-kit = { version = "0.3", default-features = false, features = ["std", "NSApplication", "NSEvent", "NSMenu", "NSMenuItem", "NSResponder", "NSGraphicsContext"] }

# AppKit's menus belong to the main thread, which the test harness keeps
# for itself: this test has its own `main`.
[[test]]
name = "macos_menu"
harness = false
```

Run `cargo tree -p tabletist-menubar --target aarch64-apple-darwin -e normal`
and note what `Cargo.lock` gains: the commit message lists it.

- [ ] **Step 2: Write the failing AppKit test**

`crates/tabletist-menubar/tests/macos_menu.rs`:

```rust
//! The bar on NSApp, chosen as a click chooses and as a key equivalent
//! does. No test harness: AppKit's menus belong to the main thread, so
//! this runs as a plain `main` (on macOS; nothing elsewhere).

#[cfg(not(target_os = "macos"))]
fn main() {}

#[cfg(target_os = "macos")]
fn main() {
    use objc2::MainThreadMarker;
    use objc2_app_kit::{NSApplication, NSEvent, NSEventModifierFlags, NSEventType, NSMenu};
    use objc2_foundation::{NSPoint, NSString};
    use raw_window_handle::{HandleError, HasWindowHandle, WindowHandle};
    use tabletist_menubar::{Entry, Menu, MenuBar, Native, Role, Shortcut, Via};

    /// AppKit needs no window for the menu bar.
    struct NoWindow;
    impl HasWindowHandle for NoWindow {
        fn window_handle(&self) -> Result<WindowHandle<'_>, HandleError> {
            Err(HandleError::Unavailable)
        }
    }

    let mtm = MainThreadMarker::new().expect("a test without a harness runs on the main thread");
    let app = NSApplication::sharedApplication(mtm);
    let cmd = |key| Shortcut { key, cmd: true, ctrl: false, alt: false, shift: false };
    let menus = vec![
        Menu {
            title: "Tabletist".to_owned(),
            role: Role::Plain,
            entries: vec![
                Entry::Item { id: 1, title: "About Tabletist".to_owned(), key: None },
                Entry::Separator,
                Entry::Native(Native::Quit, Some("Quit Tabletist".to_owned())),
            ],
        },
        Menu {
            title: "View".to_owned(),
            role: Role::Plain,
            entries: vec![
                Entry::Item { id: 2, title: "History".to_owned(), key: Some(cmd(egui::Key::Y)) },
                Entry::Item { id: 3, title: "Greyed".to_owned(), key: Some(cmd(egui::Key::J)) },
                Entry::Item {
                    id: 4,
                    title: "Previous Tab".to_owned(),
                    key: Some(Shortcut { shift: true, ..cmd(egui::Key::OpenBracket) }),
                },
                Entry::Item {
                    id: 5,
                    title: "Next Connection".to_owned(),
                    key: Some(Shortcut { cmd: false, ctrl: true, ..cmd(egui::Key::Tab) }),
                },
            ],
        },
        Menu { title: "Window".to_owned(), role: Role::Window, entries: vec![] },
    ];
    let woken = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let mut bar = MenuBar::attach(&NoWindow, menus, {
        let woken = woken.clone();
        move || {
            woken.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        }
    })
    .expect("the bar attaches to NSApp");

    let main = app.mainMenu().expect("the bar is the main menu");
    let titles = |menu: &NSMenu| -> Vec<String> {
        menu.itemArray().to_vec().iter().map(|item| item.title().to_string()).collect()
    };
    assert_eq!(titles(&main), ["Tabletist", "View", "Window"]);
    let view = main.itemAtIndex(1).and_then(|holder| holder.submenu()).expect("View");
    assert_eq!(titles(&view), ["History", "Greyed", "Previous Tab", "Next Connection"]);
    let history = view.itemAtIndex(0).expect("History");
    assert_eq!(history.keyEquivalent().to_string(), "y");
    assert!(history.keyEquivalentModifierMask().contains(NSEventModifierFlags::Command));
    assert!(app.windowsMenu().is_some(), "the Window menu is NSApp's");

    // A click: AppKit's current event is none here.
    view.performActionForItemAtIndex(0);
    assert_eq!(bar.chosen(), [(2, Via::Click)]);
    assert!(woken.load(std::sync::atomic::Ordering::SeqCst) > 0, "choosing wakes the app");

    // A key equivalent, as a US keyboard sends it.
    let key = |chars: &str, flags: NSEventModifierFlags, code: u16| {
        let chars = NSString::from_str(chars);
        NSEvent::keyEventWithType_location_modifierFlags_timestamp_windowNumber_context_characters_charactersIgnoringModifiers_isARepeat_keyCode(
            NSEventType::KeyDown, NSPoint::new(0.0, 0.0), flags, 0.0, 0, None, &chars, &chars, false, code,
        )
        .expect("a key event")
    };
    assert!(main.performKeyEquivalent(&key("y", NSEventModifierFlags::Command, 16)), "⌘Y");
    assert_eq!(bar.chosen().len(), 1, "⌘Y chose History");

    // A greyed item does not take its key: it goes on to the window.
    bar.set_enabled(3, false);
    assert!(!main.performKeyEquivalent(&key("j", NSEventModifierFlags::Command, 38)), "⌘J");
    assert!(bar.chosen().is_empty());

    // ⇧⌘[ and ⌃⇥, as AppKit receives them.
    let shift_cmd = NSEventModifierFlags::Command | NSEventModifierFlags::Shift;
    assert!(main.performKeyEquivalent(&key("{", shift_cmd, 33)), "⇧⌘[");
    assert!(main.performKeyEquivalent(&key("\t", NSEventModifierFlags::Control, 48)), "⌃⇥");
    assert_eq!(bar.chosen().len(), 2);

    // A title changes in place.
    bar.set_title(2, "Show History");
    assert_eq!(history.title().to_string(), "Show History");

    // Rebuilt without History's key: the new bar is the main menu, and
    // ⌘Y is no item's.
    let rebuilt = vec![Menu {
        title: "View".to_owned(),
        role: Role::Plain,
        entries: vec![Entry::Item { id: 2, title: "History".to_owned(), key: None }],
    }];
    assert!(bar.rebuild(rebuilt));
    let main = app.mainMenu().expect("the rebuilt bar is the main menu");
    assert_eq!(titles(&main), ["View"]);
    assert!(!main.performKeyEquivalent(&key("y", NSEventModifierFlags::Command, 16)), "⌘Y is gone");

    println!("macos_menu: ok");
}
```

If `keyEventWithType_…` is `unsafe` in objc2-app-kit 0.3, put
`#![allow(unsafe_code)]` at the top of this test file with a SAFETY note
(the arguments are a key-down's, with no window and no context) and add
`tests/macos_menu.rs` to the crate's `Cargo.toml` comment on where
`unsafe` is allowed. The method's exact name is whatever objc2-app-kit
generates; find it with `grep -rn "fn keyEventWithType" ~/.cargo/registry/src/*/objc2-app-kit-0.3*/src`.

- [ ] **Step 3: Run it to see it fail**

Run: `cargo test --locked -p tabletist-menubar --test macos_menu`
Expected: compile errors (`Menu`, `Entry`, `MenuBar` do not exist).

- [ ] **Step 4: Write the bar**

Append to `lib.rs`:

```rust
#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
mod native;

use std::hash::Hash;

use raw_window_handle::HasWindowHandle;

/// An item the platform answers itself.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Native {
    Services,
    Hide,
    HideOthers,
    ShowAll,
    Quit,
    Minimize,
    Zoom,
    BringAllToFront,
}

/// What AppKit makes of a menu besides showing it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    Plain,
    /// AppKit lists the windows here (`windowsMenu`).
    Window,
    /// AppKit adds its search field here (`helpMenu`).
    Help,
}

/// One line of a menu.
pub enum Entry<T> {
    Item {
        id: T,
        title: String,
        key: Option<Shortcut>,
    },
    /// The platform's own item, with its title where the default will not
    /// do ("Quit Tabletist").
    Native(Native, Option<String>),
    Separator,
}

pub struct Menu<T> {
    pub title: String,
    pub role: Role,
    pub entries: Vec<Entry<T>>,
}

/// The menu bar, attached. Dropped, its items are gone with it.
pub struct MenuBar<T> {
    #[cfg(target_os = "macos")]
    bar: native::Bar<T>,
    #[cfg(not(target_os = "macos"))]
    never: std::convert::Infallible,
    #[cfg(not(target_os = "macos"))]
    _ids: std::marker::PhantomData<T>,
}

impl<T: Copy + Eq + Hash + Send + 'static> MenuBar<T> {
    /// Builds the bar and attaches it: to NSApp on macOS, where `window` is
    /// not read. `wake` runs, on the main thread, after each choice.
    /// `None` off macOS, or if muda refuses an item.
    pub fn attach(
        window: &impl HasWindowHandle,
        menus: Vec<Menu<T>>,
        wake: impl Fn() + Send + Sync + 'static,
    ) -> Option<Self> {
        #[cfg(target_os = "macos")]
        {
            let window = window.window_handle().ok().map(|handle| handle.as_raw());
            native::Bar::attach(window, menus, wake).map(|bar| Self { bar })
        }
        #[cfg(not(target_os = "macos"))]
        {
            let _ = (window, menus, wake);
            None
        }
    }

    /// Builds the bar again from `menus` and puts it in the old one's
    /// place, on the same window. How a key changes: muda cannot take a
    /// key equivalent off an item on macOS (`set_accelerator(None)` does
    /// nothing there), so a key that goes away needs a new item. False if
    /// muda refused it; the old bar stays.
    pub fn rebuild(&mut self, menus: Vec<Menu<T>>) -> bool {
        #[cfg(target_os = "macos")]
        return self.bar.rebuild(menus);
        #[cfg(not(target_os = "macos"))]
        {
            let _ = menus;
            match self.never {}
        }
    }

    pub fn set_enabled(&mut self, id: T, enabled: bool) {
        #[cfg(target_os = "macos")]
        self.bar.set_enabled(id, enabled);
        #[cfg(not(target_os = "macos"))]
        {
            let _ = (id, enabled);
            match self.never {}
        }
    }

    pub fn set_title(&mut self, id: T, title: &str) {
        #[cfg(target_os = "macos")]
        self.bar.set_title(id, title);
        #[cfg(not(target_os = "macos"))]
        {
            let _ = (id, title);
            match self.never {}
        }
    }

    /// The items chosen since the last call, oldest first, each with how.
    pub fn chosen(&mut self) -> Vec<(T, Via)> {
        #[cfg(target_os = "macos")]
        return self.bar.chosen();
        #[cfg(not(target_os = "macos"))]
        match self.never {}
    }
}
```

The `#[cfg]` pairs widen to `any(target_os = "macos", windows)` in
Task 9. The stubs name every argument (`let _ = …`) so clippy's
`-D warnings` passes on Linux. If clippy reports `needless_return` on
`rebuild` or `chosen`, write each as two `#[cfg]`-gated tail expressions
(`#[cfg(target_os = "macos")] { self.bar.chosen() }` and the other).

`crates/tabletist-menubar/src/native.rs`:

```rust
//! The muda side: the bar built from the app's description, and muda's
//! one event handler routed to the bar attached last.

use std::collections::HashMap;
use std::hash::Hash;
use std::sync::{Mutex, Once, mpsc};

use muda::{Menu as Bar_, MenuEvent, MenuId, MenuItem, PredefinedMenuItem, Submenu};
use raw_window_handle::RawWindowHandle;

use crate::{Entry, KeyDown, Menu, Native, Role, Shortcut, Via, via_of};

/// Where muda's handler sends a choice: the bar attached last. muda keeps
/// one handler for the process (its first `set_event_handler` wins), so
/// the crate installs it once and moves this.
struct Route {
    send: mpsc::Sender<(MenuId, Option<KeyDown>)>,
    wake: Box<dyn Fn() + Send + Sync>,
}

static ROUTE: Mutex<Option<Route>> = Mutex::new(None);
static HANDLER: Once = Once::new();

fn route(event: MenuEvent) {
    // Read while AppKit still handles the event that chose the item.
    #[cfg(target_os = "macos")]
    let down = crate::macos::key_down();
    #[cfg(not(target_os = "macos"))]
    let down = None;
    let Ok(route) = ROUTE.lock() else {
        return;
    };
    if let Some(route) = route.as_ref() {
        let _ = route.send.send((event.id, down));
        (route.wake)();
    }
}

/// The muda menus built from a description, not yet attached.
struct Built<T> {
    menu: Bar_,
    items: HashMap<T, MenuItem>,
    ids: HashMap<MenuId, T>,
    keys: HashMap<T, Option<Shortcut>>,
    roles: Vec<(Role, Submenu)>,
}

fn build<T: Copy + Eq + Hash>(menus: Vec<Menu<T>>) -> Option<Built<T>> {
    let menu = Bar_::new();
    let mut items = HashMap::new();
    let mut ids = HashMap::new();
    let mut keys = HashMap::new();
    let mut roles = Vec::new();
    for described in menus {
        let submenu = Submenu::new(&described.title, true);
        for entry in described.entries {
            match entry {
                Entry::Item { id, title, key } => {
                    let item = MenuItem::new(&title, true, None);
                    set_key(&item, key);
                    submenu.append(&item).ok()?;
                    ids.insert(item.id().clone(), id);
                    keys.insert(id, key);
                    items.insert(id, item);
                }
                Entry::Native(native, title) => {
                    submenu.append(&predefined(native, title.as_deref())).ok()?;
                }
                Entry::Separator => submenu.append(&PredefinedMenuItem::separator()).ok()?,
            }
        }
        menu.append(&submenu).ok()?;
        roles.push((described.role, submenu));
    }
    Some(Built { menu, items, ids, keys, roles })
}

/// Puts `built` on the platform: NSApp's main menu on macOS (which
/// replaces the one before), with its Window and Help menus.
fn put<T>(built: &Built<T>, window: Option<RawWindowHandle>) -> Option<()> {
    // Only AppKit gives a menu a role; Windows reads none (Task 9).
    #[cfg(not(target_os = "macos"))]
    let _ = &built.roles;
    #[cfg(target_os = "macos")]
    {
        let _ = window;
        built.menu.init_for_nsapp();
        for (role, submenu) in &built.roles {
            match role {
                Role::Plain => {}
                Role::Window => submenu.set_as_windows_menu_for_nsapp(),
                Role::Help => submenu.set_as_help_menu_for_nsapp(),
            }
        }
    }
    Some(())
}

pub(crate) struct Bar<T> {
    built: Built<T>,
    window: Option<RawWindowHandle>,
    chosen: mpsc::Receiver<(MenuId, Option<KeyDown>)>,
}

impl<T: Copy + Eq + Hash + Send + 'static> Bar<T> {
    pub(crate) fn attach(
        window: Option<RawWindowHandle>,
        menus: Vec<Menu<T>>,
        wake: impl Fn() + Send + Sync + 'static,
    ) -> Option<Self> {
        let built = build(menus)?;
        put(&built, window)?;
        let (send, chosen) = mpsc::channel();
        HANDLER.call_once(|| MenuEvent::set_event_handler(Some(route)));
        *ROUTE.lock().ok()? = Some(Route {
            send,
            wake: Box::new(wake),
        });
        Some(Self { built, window, chosen })
    }

    pub(crate) fn rebuild(&mut self, menus: Vec<Menu<T>>) -> bool {
        let Some(built) = build(menus) else {
            return false;
        };
        if put(&built, self.window).is_none() {
            return false;
        }
        // A choice of an old item still waiting maps to nothing: dropped.
        self.built = built;
        true
    }

    pub(crate) fn set_enabled(&mut self, id: T, enabled: bool) {
        if let Some(item) = self.built.items.get(&id) {
            item.set_enabled(enabled);
        }
    }

    pub(crate) fn set_title(&mut self, id: T, title: &str) {
        if let Some(item) = self.built.items.get(&id) {
            item.set_text(title);
        }
    }

    pub(crate) fn chosen(&mut self) -> Vec<(T, Via)> {
        self.chosen
            .try_iter()
            .filter_map(|(menu_id, down)| {
                let id = *self.built.ids.get(&menu_id)?;
                let key = self.built.keys.get(&id).copied().flatten();
                Some((id, via_of(down.as_ref(), key)))
            })
            .collect()
    }
}

fn set_key(item: &MenuItem, key: Option<Shortcut>) {
    if let Err(error) = item.set_key_accelerator(key.and_then(accelerator)) {
        log::warn!("a menu item's key was refused: {error}");
    }
}

/// muda's logical accelerator for `key`: AppKit matches a key equivalent
/// by its character, so a layout other than US matches as the keymap
/// reads it.
fn accelerator(key: Shortcut) -> Option<muda::accelerator::KeyAccelerator> {
    use muda::accelerator::{CMD_OR_CTRL, Key, KeyAccelerator, Modifiers, NamedKey};
    use egui::Key as K;
    let named = match key.key {
        K::Enter => Some(NamedKey::Enter),
        K::Tab => Some(NamedKey::Tab),
        K::Escape => Some(NamedKey::Escape),
        K::Backspace => Some(NamedKey::Backspace),
        K::Delete => Some(NamedKey::Delete),
        K::ArrowUp => Some(NamedKey::ArrowUp),
        K::ArrowDown => Some(NamedKey::ArrowDown),
        K::ArrowLeft => Some(NamedKey::ArrowLeft),
        K::ArrowRight => Some(NamedKey::ArrowRight),
        K::Home => Some(NamedKey::Home),
        K::End => Some(NamedKey::End),
        K::PageUp => Some(NamedKey::PageUp),
        K::PageDown => Some(NamedKey::PageDown),
        K::F1 => Some(NamedKey::F1),
        K::F2 => Some(NamedKey::F2),
        K::F3 => Some(NamedKey::F3),
        K::F4 => Some(NamedKey::F4),
        K::F5 => Some(NamedKey::F5),
        K::F6 => Some(NamedKey::F6),
        K::F7 => Some(NamedKey::F7),
        K::F8 => Some(NamedKey::F8),
        K::F9 => Some(NamedKey::F9),
        K::F10 => Some(NamedKey::F10),
        K::F11 => Some(NamedKey::F11),
        K::F12 => Some(NamedKey::F12),
        _ => None,
    };
    let key_ = match named {
        Some(named) => Key::Named(named),
        None => Key::Character(crate::keys::character(key.key)?),
    };
    let mut mods = Modifiers::empty();
    if key.cmd {
        mods |= CMD_OR_CTRL;
    }
    if key.ctrl {
        mods |= Modifiers::CONTROL;
    }
    if key.alt {
        mods |= Modifiers::ALT;
    }
    if key.shift {
        mods |= Modifiers::SHIFT;
    }
    Some(KeyAccelerator::new(mods, key_))
}

fn predefined(native: Native, title: Option<&str>) -> PredefinedMenuItem {
    match native {
        Native::Services => PredefinedMenuItem::services(title),
        Native::Hide => PredefinedMenuItem::hide(title),
        Native::HideOthers => PredefinedMenuItem::hide_others(title),
        Native::ShowAll => PredefinedMenuItem::show_all(title),
        Native::Quit => PredefinedMenuItem::quit(title),
        Native::Minimize => PredefinedMenuItem::minimize(title),
        Native::Zoom => PredefinedMenuItem::maximize(title),
        Native::BringAllToFront => PredefinedMenuItem::bring_all_to_front(title),
    }
}
```

Rename the `Bar_` alias to something clearer if clippy or rustfmt
objects (`use muda::Menu as MudaMenu`). `crate::keys::character` is
`pub(crate)`, so `native.rs` reaches it. Confirm `NamedKey` has the
variants named (`keyboard-types` 0.8) and that `PredefinedMenuItem::maximize`
is AppKit's Zoom (`performZoom:`) in muda's macOS code; muda also has
`zoom`, use whichever sends `performZoom:`.

`crates/tabletist-menubar/src/macos.rs`:

```rust
//! The key-down AppKit is handling: what tells a key equivalent from a
//! click. Safe calls only.

use objc2::MainThreadMarker;
use objc2_app_kit::{NSApplication, NSEventModifierFlags, NSEventType};

use crate::KeyDown;

pub(crate) fn key_down() -> Option<KeyDown> {
    let mtm = MainThreadMarker::new()?;
    let event = NSApplication::sharedApplication(mtm).currentEvent()?;
    if event.r#type() != NSEventType::KeyDown {
        return None;
    }
    let flags = event.modifierFlags();
    Some(KeyDown {
        chars: event
            .charactersIgnoringModifiers()
            .map(|chars| chars.to_string())
            .unwrap_or_default(),
        cmd: flags.contains(NSEventModifierFlags::Command),
        ctrl: flags.contains(NSEventModifierFlags::Control),
        alt: flags.contains(NSEventModifierFlags::Option),
        shift: flags.contains(NSEventModifierFlags::Shift),
    })
}
```

If any of these calls is `unsafe` in objc2-app-kit 0.3.2, stop: the
review found them safe, and the crate's only `unsafe` module is the
delegate of Departure 10 (`src/macos/opening.rs`).

- [ ] **Step 5: Run the AppKit test**

Run: `cargo test --locked -p tabletist-menubar --test macos_menu`
Expected on a Mac: `macos_menu: ok`. Elsewhere it compiles and prints
nothing.

If ⇧⌘[ fails (AppKit does not match `[` with Shift against `{`), do not
guess a fix: write in the plan's PR description what AppKit did, set the
assertion to what it does, and leave Previous and Next Tab without a
shortcut on macOS by giving `key_of` (Task 4) no key for a chord with
Shift and a mark. Ask before changing anything else.

If ⌘J on the greyed item is swallowed (`performKeyEquivalent` returns
true, or the key never reaches the window in the running app), stop and
report it: the spec rests on AppKit passing a disabled item's key on, and
the fallback (giving a greyed item no key equivalent, rebuilding as keys
come and go) is a design change to agree first.

If this machine is not a Mac, the step is CI's (`macos-latest`); say so
in the commit message ("macOS test written, run by CI").

- [ ] **Step 6: The four checks, then commit**

```bash
git add Cargo.lock crates/tabletist-menubar
git commit -m "Build the menu bar on NSApp with muda and route its choices"
```

The message's body lists the crates `Cargo.lock` gained (Step 1).

### Task 4: The model

**Files:**
- Create: `src/menu.rs`, `src/menu/tests.rs`
- Modify: `src/lib.rs` (`pub mod menu;`), root `Cargo.toml` (the crate as a dependency)

- [ ] **Step 1: The dependency**

In the root `Cargo.toml`, beside `tabletist-appkit`:

```toml
# The menu bar on macOS and Windows, built with muda from `src/menu.rs`'s
# model. Empty elsewhere.
tabletist-menubar = { path = "crates/tabletist-menubar" }
```

It is a plain dependency, not behind a `cfg`: the crate is empty off
macOS and Windows, and the model is tested on every host.

- [ ] **Step 2: Write the failing tests**

`src/menu/tests.rs`:

```rust
//! The menu bar's model, which items are enabled, what a click does, and
//! what a replayed key gives egui. Headless.

use crate::app::RUNNABLE;
use crate::keymap::{Command, EditItem};

use super::{Item, Platform, bar};

fn items(platform: Platform) -> Vec<Item> {
    bar(platform).into_iter().flat_map(|menu| menu.items).collect()
}

#[test]
fn every_runnable_command_is_in_one_item() {
    for platform in [Platform::MacOs, Platform::Windows] {
        let items = items(platform);
        for command in RUNNABLE {
            if command == Command::ShowAgents && !crate::endpoint::AVAILABLE {
                assert!(!items.contains(&Item::Command(command)));
                continue;
            }
            let count = items.iter().filter(|item| **item == Item::Command(command)).count();
            assert_eq!(count, 1, "{platform:?}: {command:?}");
        }
        // Nothing but RUNNABLE's commands.
        for item in &items {
            if let Item::Command(command) = item {
                assert!(RUNNABLE.contains(command), "{command:?}");
            }
        }
    }
}

#[test]
fn every_item_has_a_title_and_separators_only_between_items() {
    for platform in [Platform::MacOs, Platform::Windows] {
        for menu in bar(platform) {
            assert!(!menu.title.is_empty());
            let first = menu.items.first();
            let last = menu.items.last();
            assert_ne!(first, Some(&Item::Separator), "{}", menu.title);
            assert_ne!(last, Some(&Item::Separator), "{}", menu.title);
            for pair in menu.items.windows(2) {
                assert!(pair != [Item::Separator, Item::Separator], "{}", menu.title);
            }
            for item in &menu.items {
                if *item != Item::Separator {
                    assert!(!super::title(*item, false).is_empty(), "{item:?}");
                }
            }
        }
    }
}

#[test]
fn each_platform_has_its_own_items() {
    let mac = bar(Platform::MacOs);
    let windows = bar(Platform::Windows);
    let titles = |menus: &[super::Menu]| menus.iter().map(|menu| menu.title).collect::<Vec<_>>();
    assert_eq!(titles(&mac), ["Tabletist", "File", "Edit", "View", "Query", "Window", "Help"]);
    assert_eq!(titles(&windows), ["File", "Edit", "View", "Query", "Window", "Help"]);
    // Exit and About in File and Help on Windows; About in the app menu
    // on the Mac, which has no Exit.
    assert!(items(Platform::Windows).contains(&Item::Exit));
    assert!(!items(Platform::MacOs).contains(&Item::Exit));
    assert_eq!(windows.last().map(|help| help.items.last()), Some(Some(&Item::About)));
    assert_eq!(mac[0].items[0], Item::About);
    // The Edit menu is the app's own on both.
    for platform in [Platform::MacOs, Platform::Windows] {
        for edit in EditItem::ALL {
            assert!(items(platform).contains(&Item::Edit(edit)), "{platform:?}: {edit:?}");
        }
    }
}
```

- [ ] **Step 3: Run them to see them fail**

Run: `cargo test --locked -p tabletist menu::tests`
Expected: compile errors.

- [ ] **Step 4: Write the model**

`src/menu.rs`:

```rust
//! The menu bar on macOS and Windows: which items, in which menus, with
//! which titles and keys. What is enabled and what a click does are the
//! app's (`src/app/menu.rs`); the live bar is `bar.rs`, and a key AppKit
//! took for an item goes back to egui through `replay.rs`.
//!
//! A key stays the keymap's: a key press does what it does without a
//! menu, and a click runs the command as quick open runs it.

mod bar;
mod replay;
#[cfg(test)]
mod tests;

pub use bar::Bar;
pub use replay::Replay;
pub use tabletist_menubar::{Native, Role};

use crate::keymap::{Command, EditItem, Keymap, Layout, MenuKey};

/// The platforms with a menu bar.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Platform {
    MacOs,
    Windows,
}

impl Platform {
    /// The platform this runs on, where it has a menu bar.
    pub fn current() -> Option<Self> {
        if cfg!(target_os = "macos") {
            Some(Self::MacOs)
        } else if cfg!(windows) {
            Some(Self::Windows)
        } else {
            None
        }
    }
}

/// One line of a menu.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Item {
    /// A command quick open runs (`RUNNABLE`): clicked, it does what
    /// quick open's row does.
    Command(Command),
    About,
    QuickOpen,
    /// Windows: the window's close, which asks about pending changes.
    Exit,
    Edit(EditItem),
    Native(Native),
    Separator,
}

pub struct Menu {
    /// An msgid.
    pub title: &'static str,
    pub role: Role,
    pub items: Vec<Item>,
}

/// The bar on `platform`.
pub fn bar(platform: Platform) -> Vec<Menu> {
    use Command as C;
    use Item::{Command as Run, Separator as Sep};
    let mac = match platform {
        Platform::MacOs => true,
        Platform::Windows => false,
    };
    let native = Item::Native;
    let mut menus = Vec::new();
    if mac {
        menus.push(Menu {
            title: "Tabletist",
            role: Role::Plain,
            items: vec![
                Item::About,
                Sep,
                Run(C::Settings),
                Sep,
                native(Native::Services),
                Sep,
                native(Native::Hide),
                native(Native::HideOthers),
                native(Native::ShowAll),
                Sep,
                native(Native::Quit),
            ],
        });
    }
    let mut file = vec![
        Run(C::NewConnection),
        Run(C::NewSqlTab),
        Run(C::Connections),
        Sep,
        Run(C::EditConnection),
        Run(C::CloseConnection),
        Sep,
        Run(C::CloseTab),
    ];
    if !mac {
        file.extend([Sep, Run(C::Settings), Sep, Item::Exit]);
    }
    menus.push(Menu { title: "File", role: Role::Plain, items: file });
    menus.push(Menu {
        title: "Edit",
        role: Role::Plain,
        items: vec![
            Item::Edit(EditItem::Undo),
            Item::Edit(EditItem::Redo),
            Sep,
            Item::Edit(EditItem::Cut),
            Item::Edit(EditItem::Copy),
            Item::Edit(EditItem::Paste),
            Item::Edit(EditItem::SelectAll),
        ],
    });
    let mut view = vec![
        Run(C::ToggleSidebar),
        Run(C::ToggleInspector),
        Run(C::FilterBar),
        Sep,
        Run(C::History),
        Item::QuickOpen,
    ];
    if crate::endpoint::AVAILABLE {
        view.push(Run(C::ShowAgents));
    }
    view.extend([Sep, Run(C::LargerText), Run(C::SmallerText), Run(C::DefaultTextSize)]);
    menus.push(Menu { title: "View", role: Role::Plain, items: view });
    menus.push(Menu {
        title: "Query",
        role: Role::Plain,
        items: vec![
            Run(C::RunAll),
            Run(C::CancelQuery),
            Run(C::Format),
            Run(C::ToggleSqlMode),
            Sep,
            Run(C::ReviewSql),
            Run(C::SaveChanges),
            Run(C::DiscardChanges),
            Sep,
            Run(C::Reload),
            Run(C::SwitchDatabase),
        ],
    });
    let cycle = [
        Run(C::NextTab),
        Run(C::PreviousTab),
        Run(C::NextConnection),
        Run(C::PreviousConnection),
    ];
    let window = if mac {
        let mut items = vec![native(Native::Minimize), native(Native::Zoom), Sep];
        items.extend(cycle);
        items.extend([Sep, native(Native::BringAllToFront)]);
        items
    } else {
        cycle.to_vec()
    };
    menus.push(Menu { title: "Window", role: Role::Window, items: window });
    let mut help = vec![Run(C::Help)];
    if !mac {
        help.extend([Sep, Item::About]);
    }
    menus.push(Menu { title: "Help", role: Role::Help, items: help });
    menus
}

/// `item`'s title, an msgid. `on` picks a toggle's second title: the
/// sidebar or the row panel shown, the tab read-only.
pub fn title(item: Item, on: bool) -> &'static str {
    use Command as C;
    match item {
        Item::Command(command) => match command {
            C::NewConnection => "New Connection",
            C::NewSqlTab => "New SQL Query",
            C::Connections => "Connections",
            C::EditConnection => "Edit Connection",
            C::CloseConnection => "Close Connection",
            C::CloseTab => "Close Tab",
            C::Settings => "Settings…",
            C::ToggleSidebar if on => "Hide Sidebar",
            C::ToggleSidebar => "Show Sidebar",
            C::ToggleInspector if on => "Hide Row Panel",
            C::ToggleInspector => "Show Row Panel",
            C::FilterBar => "Filter Rows",
            C::History => "History",
            C::ShowAgents => "Agents",
            C::LargerText => "Larger Text",
            C::SmallerText => "Smaller Text",
            C::DefaultTextSize => "Default Text Size",
            C::RunAll => "Run All",
            C::CancelQuery => "Cancel Query",
            C::Format => "Format SQL",
            C::ToggleSqlMode if on => "Make Tab Read-Write",
            C::ToggleSqlMode => "Make Tab Read-Only",
            C::ReviewSql => "Review Pending SQL",
            C::SaveChanges => "Save Changes",
            C::DiscardChanges => "Discard All Changes",
            C::Reload => "Reload Schema",
            C::SwitchDatabase => "Switch Database",
            C::NextTab => "Next Tab",
            C::PreviousTab => "Previous Tab",
            C::NextConnection => "Next Connection",
            C::PreviousConnection => "Previous Connection",
            C::Help => "Keyboard Shortcuts",
            // Never in the bar (the model's tests hold it to RUNNABLE).
            _ => "",
        },
        Item::About => "About Tabletist",
        Item::QuickOpen => "Quick Open",
        Item::Exit => "Exit",
        Item::Edit(edit) => match edit {
            EditItem::Undo => "Undo",
            EditItem::Redo => "Redo",
            EditItem::Cut => "Cut",
            EditItem::Copy => "Copy",
            EditItem::Paste => "Paste",
            EditItem::SelectAll => "Select All",
        },
        Item::Native(native) => match native {
            Native::Services => "Services",
            Native::Hide => "Hide Tabletist",
            Native::HideOthers => "Hide Others",
            Native::ShowAll => "Show All",
            Native::Quit => "Quit Tabletist",
            Native::Minimize => "Minimize",
            Native::Zoom => "Zoom",
            Native::BringAllToFront => "Bring All to Front",
        },
        Item::Separator => "",
    }
}

/// The key `item` shows in `layout`: its command's chord, quick open's
/// (the key that opens it), or the Edit item's.
pub fn key_of(keymap: &Keymap, layout: Layout, item: Item) -> Option<MenuKey> {
    match item {
        Item::Command(command) => keymap.menu_key(command, layout),
        Item::QuickOpen => keymap.menu_key(Command::FindObject, layout),
        Item::Edit(edit) => keymap.edit_key(edit, layout),
        Item::About | Item::Exit | Item::Native(_) | Item::Separator => None,
    }
}
```

Create `src/menu/bar.rs` and `src/menu/replay.rs` with only their doc
comment and an empty public struct for now (`pub struct Bar;`,
`#[derive(Default)] pub struct Replay;`), so the module compiles; Tasks 5
and 6 fill them. Confirm `Command::FindObject` is quick open's key
(`grep -n "FindObject" src/keymap.rs src/ui/keys.rs`); if quick open's
command is another, use it.

- [ ] **Step 5: Run the tests, the four checks, commit**

Run: `cargo test --locked -p tabletist menu::tests`
Expected: PASS.

```bash
git add Cargo.toml Cargo.lock src/menu.rs src/menu src/lib.rs
git commit -m "Lay out the menu bar's menus for macOS and Windows"
```

### Task 5: What is enabled, and what a click does

**Files:**
- Create: `src/app/menu.rs`
- Modify: `src/app.rs` (`mod menu;`, `pub use menu::ItemState;`), `src/app/commands.rs`, `src/menu/tests.rs`

- [ ] **Step 1: Write the failing tests**

Append to `src/menu/tests.rs`:

```rust
use crate::model::{Action, Dialog};
use crate::testing::Harness;

#[test]
fn the_window_s_commands_are_enabled_on_the_connection_picker() {
    // A new harness shows the picker, which has no workspace.
    let harness = Harness::new();
    let app = &harness.app;
    for command in [
        Command::Settings,
        Command::Help,
        Command::NewConnection,
        Command::Connections,
        Command::LargerText,
        Command::SmallerText,
    ] {
        assert!(app.menu_state(Item::Command(command)).enabled, "{command:?}");
    }
    // A connection's commands are not.
    assert!(!app.menu_state(Item::Command(Command::CloseConnection)).enabled);
}

#[test]
fn an_item_is_greyed_where_quick_open_hides_its_command() {
    let mut harness = Harness::new();
    let tab = harness.connect_fake();
    harness.settle();
    // One database: nothing to switch to.
    let app = &harness.app;
    assert!(!app.menu_state(Item::Command(Command::SwitchDatabase)).enabled);
    // A connection is there to close.
    assert!(app.menu_state(Item::Command(Command::CloseConnection)).enabled);
    // No SQL editor in front: nothing to run.
    assert!(!app.menu_state(Item::Command(Command::RunAll)).enabled);
    let _ = tab;
}

#[test]
fn under_a_dialog_only_what_its_keys_reach_stays_enabled() {
    let mut harness = Harness::new();
    harness.connect_fake();
    harness.app.apply(Action::ShowAbout);
    harness.settle();
    let app = &harness.app;
    assert!(matches!(app.dialog, Some(Dialog::About)));
    // The text sizes' keys work under a dialog.
    assert!(app.menu_state(Item::Command(Command::LargerText)).enabled);
    // A new SQL editor's do not.
    assert!(!app.menu_state(Item::Command(Command::NewSqlTab)).enabled);
    // About, Quick Open and the Edit items stay enabled: their actions
    // and egui decide.
    assert!(app.menu_state(Item::About).enabled);
    assert!(app.menu_state(Item::Edit(EditItem::Copy)).enabled);
}

#[test]
fn settings_stays_enabled_under_the_shortcuts_dialog() {
    let mut harness = Harness::new();
    harness.app.apply(Action::ShowHelp);
    harness.settle();
    assert!(harness.app.menu_state(Item::Command(Command::Settings)).enabled);
    harness.app.choose(Item::Command(Command::Settings));
    harness.settle();
    assert!(matches!(harness.app.dialog, Some(Dialog::Settings(_))));
}

#[test]
fn a_toggle_says_what_a_click_does() {
    let mut harness = Harness::new();
    let tab = harness.connect_fake();
    harness.settle();
    let sidebar = Item::Command(Command::ToggleSidebar);
    assert_eq!(harness.app.menu_state(sidebar).title, "Hide Sidebar");
    harness.app.apply(Action::ToggleSidebar(tab));
    harness.settle();
    assert_eq!(harness.app.menu_state(sidebar).title, "Show Sidebar");
    // Greyed, a toggle shows its first title.
    let mode = Item::Command(Command::ToggleSqlMode);
    assert!(!harness.app.menu_state(mode).enabled);
    assert_eq!(harness.app.menu_state(mode).title, "Make Tab Read-Only");
}

#[test]
fn a_clicked_command_does_what_quick_open_s_row_does() {
    let mut harness = Harness::new();
    let tab = harness.connect_fake();
    harness.settle();
    harness.app.actions.clear();
    harness.app.choose(Item::Command(Command::NewSqlTab));
    // Quick open's row for it pushes `NewSqlTab` for the window in front
    // (`run_of`). `Action` derives only `Debug`, so match it.
    assert!(
        matches!(harness.app.actions.as_slice(), [Action::NewSqlTab(t)] if *t == tab),
        "{:?}",
        harness.app.actions
    );
    // A command that does not apply pushes nothing.
    harness.app.choose(Item::Command(Command::SaveChanges));
    assert_eq!(harness.app.actions.len(), 1);
}
```

`harness.app.actions` is `pub actions: Vec<Action>` (`src/app.rs:131`).

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test --locked -p tabletist menu::tests`
Expected: compile errors (`menu_state`, `choose`).

- [ ] **Step 3: The window's commands without a workspace**

In `src/app/commands.rs`, move the arms of `Connections`,
`NewConnection`, `Settings`, `Help`, `LargerText`, `SmallerText` and
`DefaultTextSize` (and the `stepped` closure they use) out of `run_of`'s
`match` into a method `run_of` calls first:

```rust
    /// The commands of the window, not of a connection: they apply on the
    /// connection picker too, which has no workspace.
    fn window_run(&self, command: Command) -> Option<(&'static str, About, Vec<Action>)> {
        let stepped = |larger| {
            let terminal = self.look.terminal;
            let settings = &self.settings;
            settings.stepped_font_size(terminal, larger) != settings.font_size_in(terminal)
        };
        let run = match command {
            Command::Connections => ("Connections", About::Nothing, vec![Action::ShowConnections]),
            // ... the other six arms, moved as they are ...
            _ => return None,
        };
        Some(run)
    }
```

and at the top of `run_of`, before `let workspace = self.workspace(tab)?;`:

```rust
        if let Some((name, about, actions)) = self.window_run(command) {
            return Some((Runnable { command, name, about }, actions));
        }
```

Where a workspace exists nothing changes: the arms are the same. Read
`open_quick_open` (`src/app.rs`): if quick open can open on the picker,
it now lists these there too; say so in the commit message. Run the whole
suite: quick open's tests (`grep -rln "runnable(" src`) must still pass.

- [ ] **Step 4: Write the app's part**

`src/app/menu.rs`:

```rust
//! The menu bar's items as the app has them now: which are enabled, what
//! a toggle says, and what a click does.

use crate::keymap::{Command, Scope};
use crate::menu::{Item, title};
use crate::model::{Action, ConnTabId, Dialog, RunMode, Tab};

use super::App;

/// An item as the bar shows it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ItemState {
    pub enabled: bool,
    /// An msgid.
    pub title: &'static str,
}

impl App {
    /// `item` as the bar should show it now. Enabling is for clicks: a
    /// key goes where it goes without a menu either way.
    pub fn menu_state(&self, item: Item) -> ItemState {
        match item {
            Item::Command(command) => {
                let tab = self.active_tab_id();
                let enabled =
                    self.command_actions(tab, command).is_some() && self.reaches(command);
                let on = enabled && self.toggled(tab, command);
                ItemState {
                    enabled,
                    title: title(item, on),
                }
            }
            // Their actions decide (the About reducer, the close's
            // `hold_close`), and egui decides what an Edit item reaches.
            Item::About | Item::QuickOpen | Item::Exit | Item::Edit(_) | Item::Native(_) => {
                ItemState {
                    enabled: true,
                    title: title(item, false),
                }
            }
            Item::Separator => ItemState {
                enabled: false,
                title: "",
            },
        }
    }

    /// Whether `command` can be clicked with the dialog open, if one is:
    /// where its key reaches it (the text sizes), and Settings over the
    /// shortcuts dialog, which gives way to it (the `ShowSettings`
    /// reducer). `keys::handle` does not run under a dialog, so a greyed
    /// Settings would lose `⌘,` there.
    fn reaches(&self, command: Command) -> bool {
        let over_help = matches!(self.dialog, Some(Dialog::Help));
        self.key_reaches(command) || (command == Command::Settings && over_help)
    }

    /// Whether a press of `command`'s key is read now: always without a
    /// dialog, and under one only where a line of it holds in a prompt.
    /// A key AppKit took for a menu item is given back to egui only where
    /// it is (`menu::handling`); elsewhere the choice is a click.
    pub fn key_reaches(&self, command: Command) -> bool {
        self.dialog.is_none()
            || self
                .keymap
                .of(command)
                .any(|binding| binding.scopes.contains(&Scope::Prompt))
    }

    /// Whether a toggle is on: the sidebar shown, the row panel shown,
    /// the SQL editor in front read-only.
    fn toggled(&self, tab: ConnTabId, command: Command) -> bool {
        let Some(workspace) = self.workspace(tab) else {
            return false;
        };
        match command {
            Command::ToggleSidebar => !workspace.sidebar_hidden,
            Command::ToggleInspector => workspace.row_panel,
            Command::ToggleSqlMode => workspace
                .active_tab
                .and_then(|id| workspace.tab(id))
                .and_then(Tab::as_sql)
                .is_some_and(|sql| sql.mode == RunMode::ReadOnly),
            _ => false,
        }
    }

    /// What a click on `item` does. Exit and Paste need the window and
    /// are `Window::logic`'s; the other Edit items are replayed into the
    /// frame's input (`menu::Replay`).
    pub fn choose(&mut self, item: Item) {
        match item {
            Item::Command(command) => {
                let tab = self.active_tab_id();
                if let Some(actions) = self.command_actions(tab, command) {
                    self.actions.extend(actions);
                }
            }
            Item::About => self.actions.push(Action::ShowAbout),
            Item::QuickOpen => self.actions.push(Action::OpenQuickOpen),
            Item::Exit | Item::Edit(_) | Item::Native(_) | Item::Separator => {}
        }
    }
}
```

Declare `mod menu;` in `src/app.rs` beside `mod commands;`, and
`pub use menu::ItemState;` beside `pub use commands::…` (`bar.rs` names
the type).

- [ ] **Step 5: Run the tests, the four checks, commit**

Run: `cargo test --locked -p tabletist menu::tests`, then the whole suite.
Expected: PASS.

```bash
git add src/app.rs src/app/menu.rs src/app/commands.rs src/menu/tests.rs
git commit -m "Say which menu items are enabled and what a click on one does"
```

### Task 6: Replaying a key

**Files:**
- Modify: `src/menu.rs` (`Handling`, `handling`), `src/menu/replay.rs`, `src/menu/tests.rs`

- [ ] **Step 1: Write the failing tests**

Append to `src/menu/tests.rs`:

```rust
use egui::{Event, Key, Modifiers};
use tabletist_menubar::Via;

use crate::keymap::{MenuKey, Mods};
use super::{Handling, Replay, handling};

fn cmd_key(key: Key, shift: bool) -> MenuKey {
    MenuKey {
        mods: Mods {
            cmd: true,
            shift,
            ..Mods::default()
        },
        key,
    }
}

/// The modifiers a real press of `key` holds, as `Replay` writes them.
fn held(key: MenuKey) -> Modifiers {
    super::replay::modifiers(key.mods)
}

#[test]
fn a_key_s_replay_comes_before_its_release() {
    let run_all = cmd_key(Key::Enter, true);
    let mut replay = Replay::default();
    // A quick tap: the release is already in the frame's input.
    let mut events = vec![crate::testing::release(Key::Enter, held(run_all))];
    replay.frame(&mut events, &[(run_all, Via::Key)]);
    assert!(matches!(events[0], Event::Key { key: Key::Enter, pressed: true, .. }));
    assert!(matches!(events[1], Event::Key { key: Key::Enter, pressed: false, .. }));
}

#[test]
fn a_click_s_replay_holds_its_modifiers_for_one_frame_and_releases_its_key() {
    let undo = cmd_key(Key::Z, false);
    let mut replay = Replay::default();
    let mut events = Vec::new();
    replay.frame(&mut events, &[(undo, Via::Click)]);
    assert_eq!(events.len(), 3, "{events:?}");
    assert_eq!(events[0], Event::ModifiersChanged(held(undo)));
    assert!(matches!(events[1], Event::Key { key: Key::Z, pressed: true, .. }));
    assert!(matches!(events[2], Event::Key { key: Key::Z, pressed: false, .. }));
    // The next frame gives back what is really held.
    let mut next = Vec::new();
    replay.frame(&mut next, &[]);
    assert_eq!(next, [Event::ModifiersChanged(Modifiers::NONE)]);
    // And then nothing more.
    let mut after = Vec::new();
    replay.frame(&mut after, &[]);
    assert!(after.is_empty());
}

#[test]
fn copy_and_cut_are_the_window_s_clipboard_events() {
    let mut replay = Replay::default();
    let mut events = Vec::new();
    replay.frame(&mut events, &[(cmd_key(Key::C, false), Via::Click)]);
    assert!(events.contains(&Event::Copy), "{events:?}");
    let mut events = Vec::new();
    replay.frame(&mut events, &[(cmd_key(Key::X, false), Via::Key)]);
    assert!(events.contains(&Event::Cut), "{events:?}");
}

/// `RunSql` commands the backend was sent, as
/// `src/ui/editor_results_tests.rs`'s `runs_sent` counts them.
fn runs_sent(harness: &Harness) -> usize {
    let sent = harness.app.backend.sent.iter();
    sent.filter(|command| matches!(command, crate::backend::Command::RunSql { .. }))
        .count()
}

#[test]
fn run_all_replayed_runs_once_when_held_and_again_for_a_second_tap() {
    // A SQL editor in front with a statement, as `src/ui/mod.rs`'s
    // `sql_harness` and `set_sql` make one.
    let mut harness = Harness::new();
    let tab = harness.connect_fake();
    harness.app.apply(Action::NewSqlTab(tab));
    {
        let workspace = harness.app.workspace_mut(tab).unwrap();
        let id = workspace.active_tab.unwrap();
        let sql = workspace.sql_tab_mut(id).unwrap();
        sql.text = "select 1".into();
        sql.cursor = 0;
    }
    harness.settle();
    let key = harness
        .app
        .keymap
        .menu_key(Command::RunAll, harness.app.layout())
        .expect("Run All has a chord");
    let mut replay = Replay::default();
    let before = runs_sent(&harness);
    // Held: AppKit sends the key equivalent again; egui still holds the
    // key, so the later ones are repeats.
    for _ in 0..3 {
        let mut events = Vec::new();
        replay.frame(&mut events, &[(key, Via::Key)]);
        harness.frame(events);
    }
    harness.frame(vec![crate::testing::release(key.key, held(key))]);
    assert_eq!(runs_sent(&harness), before + 1, "held, it runs once");
    // The run comes back, as `src/ui/editor_results_tests.rs:123` answers
    // one.
    harness.answer_sql(
        Ok(crate::testing::script_outcome(vec![crate::testing::rows_outcome(1)])),
        None,
    );
    harness.settle();
    // Then a second tap, whose release is already in the frame's input.
    let mut events = vec![crate::testing::release(key.key, held(key))];
    replay.frame(&mut events, &[(key, Via::Key)]);
    harness.frame(events);
    assert_eq!(runs_sent(&harness), before + 2, "a second tap runs again");
}

#[test]
fn a_clicked_copy_copies_the_grid_s_cell() {
    // A cell selected with the grid having the keyboard, as
    // `src/ui/mod.rs` around line 1010 sets it up before `harness.copy`.
    let mut harness = Harness::new();
    let (tab, id) = harness.book_covers();
    harness.app.apply(Action::SelectCell {
        tab,
        id,
        cell: crate::model::CellPos { row: 0, col: 1 },
    });
    harness.settle();
    harness.app.workspace_mut(tab).unwrap().pane = crate::model::Pane::Grid;
    let copy = harness
        .app
        .keymap
        .edit_key(EditItem::Copy, harness.app.layout())
        .expect("Copy's chord");
    let mut replay = Replay::default();
    let mut events = Vec::new();
    replay.frame(&mut events, &[(copy, Via::Click)]);
    harness.frame(events);
    assert!(harness.copied.is_some(), "the grid copied its cell");
    // The next frame lets the modifiers go, as `harness.copy` does.
    let mut next = Vec::new();
    replay.frame(&mut next, &[]);
    harness.frame(next);
}

#[test]
fn a_key_under_a_dialog_that_the_keymap_would_not_read_is_a_click() {
    let mut harness = Harness::new();
    harness.app.apply(Action::ShowHelp);
    harness.settle();
    let settings = Item::Command(Command::Settings);
    let key = super::key_of(&harness.app.keymap, harness.app.layout(), settings);
    // ⌘, over the shortcuts dialog: `keys::handle` does not run there, so
    // a replay would do nothing; it runs as a click and opens Settings.
    assert_eq!(handling(&harness.app, settings, Via::Key, key), Handling::Click);
    // Without a dialog it is the key's.
    harness.app.dialog = None;
    assert!(matches!(handling(&harness.app, settings, Via::Key, key), Handling::Replay(_, Via::Key)));
    // Paste is always the window's; the other Edit items always replay.
    let paste = Item::Edit(EditItem::Paste);
    let copy = Item::Edit(EditItem::Copy);
    let layout = harness.app.layout();
    let copy_key = harness.app.keymap.edit_key(EditItem::Copy, layout);
    assert_eq!(handling(&harness.app, paste, Via::Click, None), Handling::Click);
    assert!(matches!(handling(&harness.app, copy, Via::Click, copy_key), Handling::Replay(_, Via::Click)));
}

#[test]
fn a_greyed_command_chosen_by_a_key_or_a_script_does_nothing() {
    let mut harness = Harness::new();
    let tab = harness.connect_fake();
    harness.app.apply(Action::ShowAbout);
    harness.settle();
    let new_sql = Item::Command(Command::NewSqlTab);
    let key = super::key_of(&harness.app.keymap, harness.app.layout(), new_sql);
    // Greyed under the dialog, and its key not read there: AppKit still
    // performs it (the item is enabled outside an open menu), and the app
    // drops it rather than open a tab behind the dialog.
    assert!(!harness.app.menu_state(new_sql).enabled);
    assert_eq!(handling(&harness.app, new_sql, Via::Key, key), Handling::Ignore);
    // A script's click on the greyed item, likewise.
    assert_eq!(handling(&harness.app, new_sql, Via::Click, key), Handling::Ignore);
    let _ = tab;
}
```

Two more, written the same way from the tests they mirror:

- `a_replayed_cmd_y_in_a_text_field_is_redo_not_history`: model it on
  `src/ui/complete_tests.rs` around lines 800 to 815 (its `editor()`,
  `type_text` and `sql()` helpers, inlined): type in the SQL editor,
  undo, then replay History's `menu_key` with `Via::Key`. The text comes
  back and no History tab opens.
- `a_replayed_cmd_r_on_a_table_s_rows_reloads_the_rows`: model it on
  `src/ui/mod.rs` around lines 4462 to 4468 (`with_page`, `focus_grid`,
  inlined): replay Reload's `menu_key` with `Via::Key` and check with
  `crate::testing::last_sent` that the rows were fetched, not the schema.

Those helpers are private to their modules: inline what they do.

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test --locked -p tabletist menu::tests`
Expected: compile errors (`Replay::frame`, `replay::modifiers`).

- [ ] **Step 3: Write the replay**

`src/menu/replay.rs`:

```rust
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

#[derive(Debug, Default)]
pub struct Replay {
    /// What winit last said is held.
    real: Modifiers,
    /// A click held its chord's modifiers: give back `real` next frame.
    restore: bool,
}

impl Replay {
    /// Adds `presses` to a frame's `events`, which hold what winit sent.
    pub fn frame(&mut self, events: &mut Vec<Event>, presses: &[(MenuKey, Via)]) {
        let told = events.iter().rev().find_map(|event| match event {
            Event::ModifiersChanged(held) => Some(*held),
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
                        // the press goes before a release already here.
                        Via::Key => {
                            let at = events
                                .iter()
                                .position(|event| releases(event, key.key))
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

fn releases(event: &Event, key: egui::Key) -> bool {
    matches!(event, Event::Key { key: up, pressed: false, .. } if *up == key)
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
```

`tests.rs` reads `super::replay::modifiers`, so `replay` is a private
module with a `pub(crate)` function; that is reachable from the sibling
`tests` module.

And in `src/menu.rs`, what becomes of a choice, kept apart from muda so
it is tested headlessly:

```rust
/// What a choice of `item` (made `via`, `key` its menu key) does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Handling {
    /// Into the frame's input as the press of `key` (`Replay`).
    Replay(MenuKey, tabletist_menubar::Via),
    /// A click: `App::choose`, or `Window::logic` for Exit and Paste.
    Click,
    /// Nothing: a command the app greys, chosen by a key or a script.
    Ignore,
}

pub fn handling(
    app: &crate::app::App,
    item: Item,
    via: tabletist_menubar::Via,
    key: Option<MenuKey>,
) -> Handling {
    use tabletist_menubar::Via;
    match (item, via, key) {
        // The window reads the clipboard: never a replay.
        (Item::Edit(EditItem::Paste), _, _) => Handling::Click,
        // Clicked or by key, the Edit items are the presses they stand for.
        (Item::Edit(_), _, Some(key)) => Handling::Replay(key, via),
        // A command's key, given back where the keymap reads it; under a
        // dialog that does not (`⌘,` over the shortcuts), a click.
        (Item::Command(command), Via::Key, Some(key)) if app.key_reaches(command) => {
            Handling::Replay(key, Via::Key)
        }
        (Item::QuickOpen, Via::Key, Some(key)) => Handling::Replay(key, Via::Key),
        // On macOS an item stays enabled for AppKit outside an open menu
        // (Departure 10), so a key the keymap would not read here (⌘T
        // under a dialog) or a script can choose one the app greys: it
        // does nothing, as without a menu.
        _ if !app.menu_state(item).enabled => Handling::Ignore,
        _ => Handling::Click,
    }
}
```

Quick open's key (`FindObject`'s ⌘P) is read while quick open is open
(it selects the typed text), so its replay needs no dialog rule.

- [ ] **Step 4: Run the tests**

Run: `cargo test --locked -p tabletist menu::tests`
Expected: PASS. If `a_clicked_copy_copies_the_grid_s_cell` fails, read
`press_of` and `held_copy` in `src/ui/keys.rs` before changing `Replay`:
the grid's copy reads the frame's modifiers, which the replay sets with
`ModifiersChanged`. Do not change `press_of`'s rule that a copy with
nothing held is no chord (its test, "A copy with nothing held …", must
still pass).

- [ ] **Step 5: The four checks, commit**

```bash
git add src/menu.rs src/menu/replay.rs src/menu/tests.rs
git commit -m "Give egui back a key a menu took, as the press it was"
```

### Task 7: The live bar, and the old menu items out

**Files:**
- Modify: `src/menu/bar.rs`, `src/entrypoint.rs`, `src/macos.rs`, `Cargo.toml`
- Modify: `crates/tabletist-appkit/Cargo.toml`, `crates/tabletist-appkit/src/lib.rs`
- Delete: `crates/tabletist-appkit/src/{about,settings,view,target}.rs`, `crates/tabletist-appkit/tests/{about_menu,settings_menu,view_menu}.rs`

- [ ] **Step 1: The live bar**

`src/menu/bar.rs`:

```rust
//! The menu bar attached to the window, kept in step with the app: each
//! frame it takes what was chosen, and after each frame it shows what is
//! enabled and, when the keymap or the look changes, is built again with
//! the new keys.

use std::collections::HashMap;
use std::sync::Arc;

use tabletist_menubar::{Entry, MenuBar, Shortcut};

use crate::app::{App, ItemState};
use crate::i18n::gettext;
use crate::keymap::{Keymap, Layout, MenuKey};

use super::{Handling, Item, Platform, Replay, bar, handling, key_of, title};

/// An item's place in the bar: menu, then item.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Id {
    menu: usize,
    item: usize,
}

pub struct Bar {
    menu: MenuBar<Id>,
    items: HashMap<Id, Item>,
    /// What each item shows now: at first what it was built with.
    shown: HashMap<Id, ItemState>,
    /// The key each item was built with.
    keys: HashMap<Id, Option<MenuKey>>,
    keymap: Arc<Keymap>,
    layout: Layout,
    replay: Replay,
    /// Clicks for `Window::logic`.
    clicks: Vec<Item>,
}

fn shortcut(key: MenuKey) -> Shortcut {
    Shortcut {
        key: key.key,
        cmd: key.mods.cmd,
        ctrl: key.mods.ctrl,
        alt: key.mods.alt,
        shift: key.mods.shift,
    }
}

/// The bar on `platform` for muda, with `app`'s keys and words; its items
/// by id; and what each shows as built (enabled, its first title).
fn describe(
    platform: Platform,
    app: &App,
) -> (
    Vec<tabletist_menubar::Menu<Id>>,
    HashMap<Id, Item>,
    HashMap<Id, ItemState>,
) {
    let layout = app.layout();
    let text = |msgid: &'static str| gettext(app.locale, msgid).to_string();
    let mut items = HashMap::new();
    let mut shown = HashMap::new();
    let mut menus = Vec::new();
    for (m, menu) in bar(platform).into_iter().enumerate() {
        let mut entries = Vec::new();
        for (i, item) in menu.items.into_iter().enumerate() {
            let id = Id { menu: m, item: i };
            entries.push(match item {
                Item::Separator => Entry::Separator,
                Item::Native(native) => Entry::Native(native, Some(text(title(item, false)))),
                _ => {
                    items.insert(id, item);
                    shown.insert(
                        id,
                        ItemState {
                            enabled: true,
                            title: title(item, false),
                        },
                    );
                    Entry::Item {
                        id,
                        title: text(title(item, false)),
                        key: key_of(&app.keymap, layout, item).map(shortcut),
                    }
                }
            });
        }
        let title = text(menu.title);
        let title = match platform {
            Platform::MacOs => title,
            Platform::Windows => format!("&{title}"),
        };
        menus.push(tabletist_menubar::Menu {
            title,
            role: menu.role,
            entries,
        });
    }
    (menus, items, shown)
}

/// The key `app` gives each of `items` now.
fn keys_of(items: &HashMap<Id, Item>, app: &App) -> HashMap<Id, Option<MenuKey>> {
    let layout = app.layout();
    items
        .iter()
        .map(|(&id, &item)| (id, key_of(&app.keymap, layout, item)))
        .collect()
}

impl Bar {
    /// Builds the bar for this platform and attaches it. `None` where
    /// there is no menu bar, or muda refused it.
    pub fn attach(
        window: &impl raw_window_handle::HasWindowHandle,
        app: &App,
        ctx: &egui::Context,
    ) -> Option<Self> {
        let platform = Platform::current()?;
        let (menus, items, shown) = describe(platform, app);
        let wake = ctx.clone();
        let menu = MenuBar::attach(window, menus, move || wake.request_repaint())?;
        let keys = keys_of(&items, app);
        let mut bar = Self {
            menu,
            items,
            shown,
            keys,
            keymap: app.keymap.clone(),
            layout: app.layout(),
            replay: Replay::default(),
            clicks: Vec::new(),
        };
        bar.sync(app);
        Some(bar)
    }

    /// Takes what was chosen since the last frame: what replays goes into
    /// `raw`'s events (`menu::handling`); the clicks wait for
    /// [`Bar::clicks`].
    pub fn input(&mut self, raw: &mut egui::RawInput, app: &App) {
        let mut presses = Vec::new();
        for (id, via) in self.menu.chosen() {
            let Some(&item) = self.items.get(&id) else {
                continue;
            };
            let key = key_of(&self.keymap, self.layout, item);
            match handling(app, item, via, key) {
                Handling::Replay(key, via) => presses.push((key, via)),
                Handling::Click => self.clicks.push(item),
                Handling::Ignore => {}
            }
        }
        self.replay.frame(&mut raw.events, &presses);
    }

    /// The clicks [`Bar::input`] kept, oldest first.
    pub fn clicks(&mut self) -> Vec<Item> {
        std::mem::take(&mut self.clicks)
    }

    /// Shows the app as it is after the frame: built again where an
    /// item's key changed, then each item's state.
    pub fn sync(&mut self, app: &App) {
        let layout = app.layout();
        // Every settings change makes a new keymap (`refresh_keymap`), so
        // the pointer only says the keys may have changed; the bar is
        // built again only when one did. On Windows a rebuild sets the
        // window's menu twice.
        if !Arc::ptr_eq(&self.keymap, &app.keymap) || layout != self.layout {
            self.keymap = app.keymap.clone();
            self.layout = layout;
            let keys = keys_of(&self.items, app);
            if keys != self.keys {
                if let Some(platform) = Platform::current() {
                    let (menus, items, shown) = describe(platform, app);
                    if self.menu.rebuild(menus) {
                        self.keys = keys_of(&items, app);
                        self.items = items;
                        self.shown = shown;
                    }
                }
                // A refused rebuild keeps the old bar, asked once: the
                // keymap and look above are recorded either way.
            }
        }
        for (&id, &item) in &self.items {
            let state = app.menu_state(item);
            let Some(before) = self.shown.insert(id, state) else {
                continue;
            };
            if before.enabled != state.enabled {
                self.menu.set_enabled(id, state.enabled);
            }
            if before.title != state.title {
                self.menu.set_title(id, &gettext(app.locale, state.title));
            }
        }
    }
}
```

`menu_state` runs `run_of` for every item after every frame; Run All's
tokenizes the script in front (`tabletist_db::sql::statements`). Measure
it once with a 5,000-line script in the editor (time `sync` with
`std::time::Instant` in a test, or `--release` by hand). If it is over a
millisecond, keep the last answer for Run All per SQL tab and text (the
editor's text changes only on edits) and say so in the commit; if not,
change nothing.

Check `gettext`'s signature (`fastframe_i18n::gettext`; existing calls
are `crate::i18n::gettext(app.locale, "…")`) and its return type, and
drop `.to_string()` where it is already a `String`. eframe does not
re-export `raw_window_handle`: add it to the root `Cargo.toml`'s
`[dependencies]` as

```toml
# The window the menu bar attaches to (`src/menu/bar.rs`), which eframe's
# creation context hands out; eframe already builds this version.
raw-window-handle = "0.6"
```

`Cargo.lock` gains no crate for it (0.6.2 is there).

- [ ] **Step 2: Wire it into the window**

In `src/entrypoint.rs`:

1. In the creation closure, replace the four `#[cfg(target_os = "macos")]
   let …_menu = …` blocks and their warnings with:

   ```rust
   // The menu bar is up by now: winit builds its own before the window,
   // and this one takes its place.
   let menu = crate::menu::Bar::attach(cc, &app, &cc.egui_ctx);
   if menu.is_none() && crate::menu::Platform::current().is_some() {
       log::warn!("the menu bar could not be attached");
   }
   ```

   and in `Ok(Box::new(Window { … }))` replace the four fields with
   `menu,`.
2. In `struct Window`, replace the four fields with:

   ```rust
   /// macOS and Windows: the menu bar.
   menu: Option<crate::menu::Bar>,
   ```

3. In `impl eframe::App for Window`, add:

   ```rust
   fn raw_input_hook(&mut self, _ctx: &egui::Context, raw: &mut egui::RawInput) {
       // Before the frame's `logic`: a key a menu took goes back into
       // this frame's input.
       if let Some(menu) = &mut self.menu {
           menu.input(raw, &self.app);
       }
   }
   ```

4. In `logic`, replace the four `if … take` blocks with:

   ```rust
   let clicks = self.menu.as_mut().map(crate::menu::Bar::clicks).unwrap_or_default();
   for item in clicks {
       match item {
           // The window's close, as its close button: pending changes
           // ask first.
           crate::menu::Item::Exit => ctx.send_viewport_cmd(egui::ViewportCommand::Close),
           // The window reads the clipboard and hands egui a paste.
           crate::menu::Item::Edit(crate::keymap::EditItem::Paste) => {
               ctx.send_viewport_cmd(egui::ViewportCommand::RequestPaste);
               ctx.request_repaint();
           }
           item => self.app.choose(item),
       }
   }
   ```

5. At the end of `ui`, after `self.app.frame_ui(ui)` (the frame's actions
   are applied there) and the title bar's buttons:

   ```rust
   if let Some(menu) = &mut self.menu {
       menu.sync(&self.app);
   }
   ```

`cc` (`eframe::CreationContext`) implements `HasWindowHandle`
(`epi.rs:101` in the fork).

- [ ] **Step 3: The old items out**

- `src/macos.rs`: delete `AboutMenu`, `SettingsMenu`, `HistoryMenu`,
  `QuickOpenMenu`, the `use std::cell::Cell; use std::rc::Rc;` they
  needed, and the doc comment's last paragraph ("The app menu's About
  item …"), replaced by: "The menu bar is `crate::menu`'s."
- `crates/tabletist-appkit`: delete `src/about.rs`, `src/settings.rs`,
  `src/view.rs`, `src/target.rs` and `tests/about_menu.rs`,
  `tests/settings_menu.rs`, `tests/view_menu.rs`; their `mod`/`pub use`
  lines in `lib.rs`; their three `[[test]]` sections; `"NSMenu",
  "NSMenuItem"` (and `"NSResponder"` if `fonts.rs` does not need it) from
  `objc2-app-kit`'s features. The lints comment now reads: "…listing the
  installed fonts means reading CoreText's C statics and untyped
  collections, so this crate carries the workspace's lints with
  `unsafe_code` lowered to `deny`, allowed only in `fonts.rs` and the
  tests, each use with a SAFETY note." The crate's description and
  `lib.rs` doc say CoreText's calls.
- Root `Cargo.toml`: `tabletist-appkit`'s comment becomes "The installed
  fonts CoreText lists for the data font: CoreText calls that need unsafe
  code, kept in a crate of their own because this one forbids it."

- [ ] **Step 4: Build, the four checks**

Run the four checks. Then, on a Mac, `cargo run -- --demo` and look:
the menus of the spec's table; ⌘Y outside a text field shows History,
and in the SQL editor redoes; ⌘, opens Settings, also over the shortcuts
dialog; Edit › Copy copies a cell; ⌘Q with a pending change asks. Write
what was and was not seen in the commit message.

- [ ] **Step 5: Commit**

```bash
git add -A src/entrypoint.rs src/macos.rs src/menu Cargo.toml Cargo.lock crates/tabletist-appkit
git commit -m "Put the muda menu bar on macOS in place of the four AppKit items"
```

### Task 8: The docs, and the pull request

**Files:**
- Modify: `AGENTS.md`, `docs/_guide/macos.md`, `docs/_reference/keyboard-shortcuts.md`

- [ ] **Step 1: AGENTS.md**

In "Architecture", the `unsafe` paragraph becomes:

> - The workspace forbids `unsafe`. AppKit and CoreText calls that cannot
>   be made without it go in `crates/tabletist-appkit`, the menu bar's
>   (an AppKit menu delegate, muda's Windows attach) in
>   `crates/tabletist-menubar`, and SQLite
>   calls in `crates/tabletist-sqlite-ffi`, behind a safe API, each with a
>   SAFETY note; `src/macos.rs`, `crates/tabletist-db` and everything else
>   stay free of it.

And after the keymap paragraph in "Checks":

> The menu bar (`src/menu.rs`) shows the keymap's keys and changes none:
> a key AppKit takes for a menu item goes back to egui as the same press
> (`src/menu/replay.rs`), and only a click runs a command as quick open
> does.

- [ ] **Step 2: The site**

- `docs/_guide/macos.md`: replace the bullet "**The app menu has the
  usual two.** …" with one on the menu bar: File, Edit, View, Query,
  Window and Help hold the commands quick open runs, each with its key;
  a greyed item is a command that does not apply now; a key does what it
  does where you are (⌘N in the grid still adds a row; the menu's New
  Connection is a click away); Tabletist › About Tabletist and Settings…
  as before. In "### Quick open", "View › Quick
  Open in the menu bar" stays.
- `docs/_reference/keyboard-shortcuts.md`: one sentence near the top:
  the menus on macOS and Windows show the keys of this page, and your
  own from Settings › Keyboard.
- `cd docs && bin/check`.

- [ ] **Step 3: Commit, push, open the pull request**

```bash
git add AGENTS.md docs/_guide/macos.md docs/_reference/keyboard-shortcuts.md
git commit -m "Describe the macOS menu bar"
```

Push the branch and open the pull request "A native menu bar on macOS".
Its description: what the menus hold, that a key stays the keymap's,
what `Cargo.lock` gained, which checks ran where, and "Needs eyes" below
for macOS.

---

## Part 2: Windows

A second branch from `main` once Part 1 is merged.

### Task 9: Attaching to the window

**Files:**
- Create: `crates/tabletist-menubar/src/windows.rs`
- Modify: `crates/tabletist-menubar/Cargo.toml`, `src/lib.rs`, `src/native.rs`

- [ ] **Step 1: Write the failing test**

In `crates/tabletist-menubar/src/native.rs`'s existing `mod tests`
(Task 3 made one), a test for Windows only:

```rust
    #[cfg(windows)]
    #[test]
    fn a_choice_reaches_the_bar_attached_last() {
        let menus = vec![Menu {
            title: "&File".to_owned(),
            role: Role::Plain,
            entries: vec![Entry::Item { id: 7_u8, title: "Exit".to_owned(), key: None }],
        }];
        // No window: built and routed, not attached.
        let mut bar = Bar::attach(None, menus, || {}).expect("the bar builds");
        let id = bar.built.items[&7].item.id().clone();
        route(MenuEvent { id });
        assert_eq!(bar.chosen(), [(7, Via::Click)]);
    }
```

Task 3 keeps each item as an `Rc<Slot>` holding the muda `MenuItem`;
reach its id the way `Built` holds it now. `MenuEvent`'s `id` field is
public in muda 0.21 (`menu_event.rs`). Also on Windows: `set_enabled`
applies at once (Task 3's `Slot::want` has the branch), with a test
that a greyed item reads disabled without any menu opening; and
`character()`'s `cfg_attr(not(any(test, target_os = "macos")), expect(dead_code, …))`
gains `windows` (`not(any(test, target_os = "macos", windows))`).

- [ ] **Step 2: Run it to see it fail (on Windows or in CI)**

Run: `cargo test --locked -p tabletist-menubar`
Expected on Windows: compile errors (muda is not built there yet).

- [ ] **Step 3: Write the Windows part**

- `Cargo.toml` of the crate: the muda dependency's target becomes
  `cfg(any(target_os = "macos", windows))`, with the comment saying
  Windows adds `windows-sys` features; the lints comment says
  `unsafe_code` stays lowered to `deny`, now allowed in `windows.rs` as
  well as `macos/opening.rs`:

  ```toml
  # Defining the AppKit menu delegate (`macos/opening.rs`) and muda's
  # Windows attach (`windows.rs`), which subclasses the window, need unsafe:
  # this crate carries the workspace's lints with `unsafe_code` lowered to
  # `deny`, allowed only in those two modules, each use with a SAFETY note.
  [lints.rust]
  unsafe_code = "deny"

  [lints.clippy]
  unwrap_used = "warn"
  ```

  (Copy the workspace's clippy lints as `tabletist-appkit` does, rather
  than `[lints] workspace = true`, which cannot lower one lint.)
- `lib.rs` and `native.rs`: every `cfg(target_os = "macos")` that guards
  muda becomes `cfg(any(target_os = "macos", windows))`; `macos.rs`
  stays macOS-only. On Windows `MenuBar::attach` returns `None` when
  `window.window_handle()` fails: an app bar is always attached; only the
  crate's own test builds one with no window. In `lib.rs`'s `attach`:

  ```rust
  let window = window.window_handle().ok().map(|handle| handle.as_raw());
  #[cfg(windows)]
  let window = Some(window?);
  native::Bar::attach(window, menus, wake).map(|bar| Self { bar })
  ```
- `native.rs`'s `put` gains the Windows branch, and `rebuild` takes the
  old bar off the window first:

  ```rust
  #[cfg(windows)]
  if let Some(window) = window {
      crate::windows::attach(&built.menu, window)?;
  }
  ```

  ```rust
  // In `rebuild`, before `put(&built, self.window)`:
  #[cfg(windows)]
  if let Some(window) = self.window {
      crate::windows::detach(&self.built.menu, window);
  }
  ```

  If `put` then fails, attach the old bar again (`put(&self.built,
  self.window)`) before returning false.
- `crates/tabletist-menubar/src/windows.rs`:

  ```rust
  //! The bar on the window: muda subclasses it, which it marks unsafe.

  #![allow(unsafe_code)]

  use raw_window_handle::RawWindowHandle;

  fn hwnd(window: RawWindowHandle) -> Option<isize> {
      match window {
          RawWindowHandle::Win32(win32) => Some(win32.hwnd.get()),
          _ => None,
      }
  }

  pub(crate) fn attach(menu: &muda::Menu, window: RawWindowHandle) -> Option<()> {
      let hwnd = hwnd(window)?;
      // SAFETY: the handle is the live window eframe created, used on its
      // thread (the creation closure and the frames run on the event
      // loop's), and eframe drops the app, and with it the bar, before
      // the window.
      unsafe { menu.init_for_hwnd(hwnd) }
          .map_err(|error| log::warn!("could not attach the menu bar: {error}"))
          .ok()
  }

  pub(crate) fn detach(menu: &muda::Menu, window: RawWindowHandle) {
      let Some(hwnd) = hwnd(window) else {
          return;
      };
      // SAFETY: as `attach`'s; this menu was attached to this window.
      if let Err(error) = unsafe { menu.remove_for_hwnd(hwnd) } {
          log::warn!("could not take the menu bar off: {error}");
      }
  }
  ```

  `RawWindowHandle` is `Copy` and not `Send`; `MenuBar` stays on the
  main thread (in `Window`), so that is fine.

- `src/entrypoint.rs`: the warning "the menu bar could not be attached"
  is gated on `cfg!(target_os = "macos")` (Task 7, while Windows had no
  bar to attach); take the gate off, so it warns wherever
  `menu::Platform::current()` has a bar.

The app installs no accelerator table and no message hook: the menu
writes its keys, and the keys stay egui's.

- [ ] **Step 4: Run, the four checks, commit**

Run: `cargo test --locked -p tabletist-menubar` (Windows or CI) and the
four checks; cross-check `cargo clippy --locked --workspace --all-targets
--target x86_64-pc-windows-msvc -- -D warnings` from a Mac or Linux if
the target is installed.

```bash
git add Cargo.lock crates/tabletist-menubar
git commit -m "Attach the menu bar to the window on Windows"
```

### Task 10: What attaching changes on Windows

**Files:**
- Modify: `src/entrypoint.rs` (only if the size check fails), `src/menu/tests.rs`

- [ ] **Step 1: F10 and Alt**

Add to `src/menu/tests.rs`:

```rust
#[test]
fn no_windows_key_is_f10_or_alt_alone() {
    // With a menu, Windows opens it on F10 and a lone Alt.
    let keymap = crate::keymap::Keymap::default();
    for binding in keymap.bindings() {
        for chord in binding.bound(crate::keymap::Layout::Windows).chords {
            if let Ok(crate::keymap::Spelled::Strokes(strokes)) = crate::keymap::spell(chord.keys) {
                for stroke in strokes {
                    if let crate::keymap::Stroke::Key { key, .. } = stroke {
                        assert_ne!(key, Key::F10, "{:?}", binding.command);
                    }
                }
            }
        }
    }
}
```

Run it; it passes today (the review found no F10 binding).

- [ ] **Step 2: The persisted size**

On Windows: run the app, note the window's inner size, quit, run again,
quit, run a third time. If the inner size shrinks by the menu's height
each launch, the menu was set after eframe restored the size: then, in
`src/entrypoint.rs`, after attaching on Windows, send
`ViewportCommand::InnerSize` with the size eframe restored
(`ctx.input(|i| i.viewport().inner_rect)` before attaching) once. If it
does not shrink, write that in the commit message and change nothing.

- [ ] **Step 3: The four checks, commit**

```bash
git add src/menu/tests.rs src/entrypoint.rs
git commit -m "Keep F10 Windows' and the window's size its own with a menu bar"
```

### Task 11: The Windows docs, and the pull request

Two sentences Part 1 wrote for macOS alone widen here: AGENTS.md's
`unsafe` paragraph names muda's Windows attach (`windows.rs`) beside the
menu delegate, and `_reference/keyboard-shortcuts.md`'s "The menu bar on
macOS shows the keys of this page" says macOS and Windows.

- [ ] **Step 1**: `docs/_guide/what-is-tabletist.md` names the platforms
  in one sentence (line 11): no list of features per platform, so it
  stays. `docs/_guide/getting-started.md` names nothing per platform
  that the menu changes. So the site changes only where Part 1 already
  said "macOS and Windows". `cd docs && bin/check`.
- [ ] **Step 2**: push and open "A native menu bar on Windows", with
  "Needs eyes" below for Windows.

---

## Checks

Each task ends with the four checks of `AGENTS.md`. The integration
suites need no change (no database code moves).

## Needs eyes

Not run by any test; the pull request says each is not seen until
someone runs it.

macOS:
- The menus as drawn, with their keys; AppKit's own Enter Full Screen in
  View, Dictation and Emoji in Edit, the window list in Window, the
  search field in Help; nothing twice.
- ⌘Q, and Tabletist › Quit, with a pending change: the app asks.
- Edit › Paste, clicked, pastes into a text field (it goes through
  `Window::logic` and eframe's clipboard, which no headless test runs).
- After unbinding a menu command's key in Settings › Keyboard, the menu
  shows no key and AppKit no longer takes it (the rebuild).
- A held ⇧⌘↩ runs once; two quick taps run twice.
- A non-US layout (German): ⇧⌘[ and the letters' chords.
- Departure 11 rests on synthetic key events, which also failed to match
  ⇧⌘Z (a key every app's Redo answers). On a real keyboard in the
  running app, with the filter taken out for the check: does ⇧⌘[ choose
  Previous Tab, ⇧⌘Z Redo, ⌃⇧⇥ Previous Connection? If ⇧⌘[ works, drop
  the filter; if ⌃⇧⇥ does not, extend it to Shift with Tab.
- Departure 10 in the running app: opening a menu greys what does not
  apply (AppKit calls `menuWillOpen:`), closing it gives every key back,
  and a greyed item's key still does what it does without a menu.
- A choice made by a real key equivalent arrives as `Via::Key` (the
  AppKit test's synthetic events leave `currentEvent` empty).
- A key equivalent during IME composition (Japanese input in the SQL
  editor): the key comes back released.

Windows:
- The menu strip under the title bar, its mnemonics (Alt+F opens File),
  F10 and a lone Alt opening it.
- File › Exit with a pending change asks (it is `Window::logic`'s, which
  no headless test runs).
- Edit › Paste, clicked, pastes into a text field.
- The window's size across three launches (Task 10).
- Dark mode: muda's menu strip follows the system or not; say which.

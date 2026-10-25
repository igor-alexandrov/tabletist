//! The muda side: the bar built from the app's description, and muda's
//! one event handler routed to the bar attached last.

use std::cell::Cell;
use std::collections::HashMap;
use std::hash::Hash;
use std::rc::Rc;
use std::sync::{Mutex, Once, mpsc};

use muda::{Menu as MudaMenu, MenuEvent, MenuId, MenuItem, PredefinedMenuItem, Submenu};
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

/// An item, and the state the app wants it in.
struct Slot {
    item: MenuItem,
    wanted: Cell<bool>,
    /// Whether its menu is open: on macOS an item is greyed only then.
    #[cfg(target_os = "macos")]
    open: Rc<Cell<bool>>,
}

impl Slot {
    fn want(&self, enabled: bool) {
        self.wanted.set(enabled);
        // AppKit takes a disabled item's key equivalent and performs
        // nothing; enabled, the key reaches the app. `Shown` greys the
        // item when its menu opens.
        #[cfg(target_os = "macos")]
        if !self.open.get() {
            return;
        }
        self.item.set_enabled(enabled);
    }
}

/// A menu's items, which its delegate greys while AppKit shows it open.
#[cfg(target_os = "macos")]
pub(crate) struct Shown {
    open: Rc<Cell<bool>>,
    slots: Vec<Rc<Slot>>,
}

#[cfg(target_os = "macos")]
impl Shown {
    pub(crate) fn opened(&self) {
        self.open.set(true);
        for slot in &self.slots {
            slot.item.set_enabled(slot.wanted.get());
        }
    }

    pub(crate) fn closed(&self) {
        self.open.set(false);
        for slot in &self.slots {
            slot.item.set_enabled(true);
        }
    }
}

/// The muda menus built from a description, not yet attached.
struct Built<T> {
    menu: MudaMenu,
    items: HashMap<T, Rc<Slot>>,
    ids: HashMap<MenuId, T>,
    keys: HashMap<T, Option<Shortcut>>,
    roles: Vec<(Role, Submenu)>,
    /// Each menu's items, in the bar's order.
    #[cfg(target_os = "macos")]
    shown: Vec<Rc<Shown>>,
    /// The menus' delegates, which NSMenu holds weakly: `put` makes them.
    #[cfg(target_os = "macos")]
    openings: Vec<objc2::rc::Retained<crate::macos::Opening>>,
}

fn build<T: Copy + Eq + Hash>(menus: Vec<Menu<T>>) -> Option<Built<T>> {
    let menu = MudaMenu::new();
    let mut items = HashMap::new();
    let mut ids = HashMap::new();
    let mut keys = HashMap::new();
    let mut roles = Vec::new();
    #[cfg(target_os = "macos")]
    let mut shown = Vec::new();
    for described in menus {
        let submenu = Submenu::new(&described.title, true);
        #[cfg(target_os = "macos")]
        let open = Rc::new(Cell::new(false));
        #[cfg(target_os = "macos")]
        let mut slots = Vec::new();
        for entry in described.entries {
            match entry {
                Entry::Item { id, title, key } => {
                    // AppKit cannot match a mark with Shift: the item has
                    // no key, so `via_of` never reads one either.
                    #[cfg(target_os = "macos")]
                    let key = key.filter(|&key| !shifted_mark(key));
                    let item = MenuItem::new(&title, true, None);
                    set_key(&item, key);
                    submenu.append(&item).ok()?;
                    ids.insert(item.id().clone(), id);
                    keys.insert(id, key);
                    let slot = Rc::new(Slot {
                        item,
                        wanted: Cell::new(true),
                        #[cfg(target_os = "macos")]
                        open: open.clone(),
                    });
                    #[cfg(target_os = "macos")]
                    slots.push(slot.clone());
                    items.insert(id, slot);
                }
                Entry::Native(native, title) => {
                    submenu.append(&predefined(native, title.as_deref())).ok()?;
                }
                Entry::Separator => submenu.append(&PredefinedMenuItem::separator()).ok()?,
            }
        }
        menu.append(&submenu).ok()?;
        roles.push((described.role, submenu));
        #[cfg(target_os = "macos")]
        shown.push(Rc::new(Shown { open, slots }));
    }
    Some(Built {
        menu,
        items,
        ids,
        keys,
        roles,
        #[cfg(target_os = "macos")]
        shown,
        #[cfg(target_os = "macos")]
        openings: Vec::new(),
    })
}

/// Puts `built` on the platform: NSApp's main menu on macOS (which
/// replaces the one before), with its Window and Help menus and a
/// delegate on each menu.
fn put<T>(built: &mut Built<T>, window: Option<RawWindowHandle>) -> Option<()> {
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
        built.openings = crate::macos::watch(&built.shown);
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
        let mut built = build(menus)?;
        put(&mut built, window)?;
        let (send, chosen) = mpsc::channel();
        HANDLER.call_once(|| MenuEvent::set_event_handler(Some(route)));
        *ROUTE.lock().ok()? = Some(Route {
            send,
            wake: Box::new(wake),
        });
        Some(Self {
            built,
            window,
            chosen,
        })
    }

    pub(crate) fn rebuild(&mut self, menus: Vec<Menu<T>>) -> bool {
        let Some(mut built) = build(menus) else {
            return false;
        };
        if put(&mut built, self.window).is_none() {
            return false;
        }
        // A choice of an old item still waiting maps to nothing: dropped.
        self.built = built;
        true
    }

    pub(crate) fn set_enabled(&mut self, id: T, enabled: bool) {
        if let Some(slot) = self.built.items.get(&id) {
            slot.want(enabled);
        }
    }

    pub(crate) fn set_title(&mut self, id: T, title: &str) {
        if let Some(slot) = self.built.items.get(&id) {
            slot.item.set_text(title);
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
    use egui::Key as K;
    use muda::accelerator::{CMD_OR_CTRL, Key, KeyAccelerator, Modifiers, NamedKey};
    let named = match key.key {
        K::Enter => Some(NamedKey::Enter),
        K::Tab => Some(NamedKey::Tab),
        K::Escape => Some(NamedKey::Escape),
        K::Backspace => Some(NamedKey::Backspace),
        // muda's macOS table gives `NamedKey::Delete` the Delete key's
        // U+7F, which is egui's Backspace; forward delete is AppKit's
        // U+F728, which `character` gives.
        #[cfg(target_os = "macos")]
        K::Delete => None,
        #[cfg(not(target_os = "macos"))]
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

/// Whether `key` holds Shift with a mark: AppKit matches `[` with Shift
/// against nothing a keyboard sends (it sends `{`), so such an item shows
/// no key, and the keymap still reads the chord.
#[cfg(target_os = "macos")]
fn shifted_mark(key: Shortcut) -> bool {
    let Some(character) = crate::keys::character(key.key) else {
        return false;
    };
    let mut chars = character.chars();
    let mark = match (chars.next(), chars.next()) {
        (Some(char), None) => char.is_ascii_punctuation(),
        _ => false,
    };
    key.shift && mark
}

fn predefined(native: Native, title: Option<&str>) -> PredefinedMenuItem {
    match native {
        Native::Services => PredefinedMenuItem::services(title),
        Native::Hide => PredefinedMenuItem::hide(title),
        Native::HideOthers => PredefinedMenuItem::hide_others(title),
        Native::ShowAll => PredefinedMenuItem::show_all(title),
        Native::Quit => PredefinedMenuItem::quit(title),
        Native::Minimize => PredefinedMenuItem::minimize(title),
        // muda's alias for `maximize`, which sends AppKit's `performZoom:`.
        Native::Zoom => PredefinedMenuItem::zoom(title),
        Native::BringAllToFront => PredefinedMenuItem::bring_all_to_front(title),
    }
}

#[cfg(test)]
mod tests {
    use muda::accelerator::{CMD_OR_CTRL, Key, Modifiers, NamedKey};

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

    #[test]
    fn a_letter_is_its_character_with_the_platform_s_command_key() {
        let history = accelerator(cmd(egui::Key::Y)).expect("a key");
        assert_eq!(history.key(), &Key::Character("y".to_owned()));
        assert_eq!(history.modifiers(), CMD_OR_CTRL);
    }

    #[test]
    fn every_modifier_is_carried() {
        let next = accelerator(Shortcut {
            ctrl: true,
            alt: true,
            shift: true,
            ..cmd(egui::Key::Tab)
        })
        .expect("a key");
        assert_eq!(next.key(), &Key::Named(NamedKey::Tab));
        assert_eq!(
            next.modifiers(),
            CMD_OR_CTRL | Modifiers::CONTROL | Modifiers::ALT | Modifiers::SHIFT
        );
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn a_mark_with_shift_has_no_key_on_macos() {
        let shifted = |key| Shortcut {
            shift: true,
            ..cmd(key)
        };
        assert!(shifted_mark(shifted(egui::Key::OpenBracket)));
        assert!(shifted_mark(shifted(egui::Key::CloseBracket)));
        assert!(shifted_mark(shifted(egui::Key::Minus)));
        assert!(shifted_mark(shifted(egui::Key::Quote)));
        // Without Shift a mark matches.
        assert!(!shifted_mark(cmd(egui::Key::OpenBracket)));
        // A letter, a digit and a named key keep theirs.
        assert!(!shifted_mark(shifted(egui::Key::Z)));
        assert!(!shifted_mark(shifted(egui::Key::Num1)));
        assert!(!shifted_mark(shifted(egui::Key::Enter)));
        assert!(!shifted_mark(shifted(egui::Key::Tab)));
        assert!(!shifted_mark(shifted(egui::Key::Space)));
        assert!(!shifted_mark(shifted(egui::Key::ArrowUp)));
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn forward_delete_is_appkit_s_and_not_the_delete_key() {
        let delete = accelerator(cmd(egui::Key::Delete)).expect("a key");
        assert_eq!(delete.key(), &Key::Character("\u{F728}".to_owned()));
        let backspace = accelerator(cmd(egui::Key::Backspace)).expect("a key");
        assert_eq!(backspace.key(), &Key::Named(NamedKey::Backspace));
    }
}

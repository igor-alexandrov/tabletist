//! The menu bar attached to the window, kept in step with the app: each
//! frame it takes what was chosen, and after each frame it shows what is
//! enabled and, when the keymap or the look changes, is built again with
//! the new keys.
//!
//! What it decides is in plain functions and types (`describe`, `route`,
//! `changes`, `Keyed`, `Feed`), tested headlessly in `tests.rs`; [`Bar`]
//! only hands their answers to muda.

use std::collections::HashMap;
use std::sync::Arc;

use tabletist_menubar::{Entry, MenuBar, Shortcut, Via};

use crate::app::{App, ItemState};
use crate::i18n::gettext;
use crate::keymap::{EditItem, Keymap, Layout, MenuKey};

use super::{Handling, Item, Platform, Replay, bar, handling, key_of, title};

/// An item's place in the bar: menu, then item.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Id {
    menu: usize,
    item: usize,
}

/// The bar as muda takes it: its menus, its items by id, and what each
/// item shows as built (enabled, its first title).
pub(super) struct Described {
    pub menus: Vec<tabletist_menubar::Menu<Id>>,
    pub items: HashMap<Id, Item>,
    pub shown: HashMap<Id, ItemState>,
}

/// The bar on `platform` for muda, with `app`'s keys and words.
pub(super) fn describe(platform: Platform, app: &App) -> Described {
    let layout = app.layout();
    let text = |msgid: &'static str| gettext(app.locale, msgid).into_owned();
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
        let name = text(menu.title);
        let name = match platform {
            Platform::MacOs => name,
            // The menu's access key: Alt and its first letter.
            Platform::Windows => format!("&{name}"),
        };
        menus.push(tabletist_menubar::Menu {
            title: name,
            role: menu.role,
            entries,
        });
    }
    Described {
        menus,
        items,
        shown,
    }
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

/// The key `app` gives each of `items` now.
pub(super) fn keys_of(items: &HashMap<Id, Item>, app: &App) -> HashMap<Id, Option<MenuKey>> {
    let layout = app.layout();
    items
        .iter()
        .map(|(&id, &item)| (id, key_of(&app.keymap, layout, item)))
        .collect()
}

/// What `chosen` does in `app` (`menu::handling`): the presses to replay
/// into the frame's input, and the clicks, each oldest first. An item's
/// key is the one it was built with (`keys`): the key AppKit matched,
/// given back as it was pressed. An id the bar does not have is skipped.
pub(super) fn route(
    app: &App,
    items: &HashMap<Id, Item>,
    keys: &HashMap<Id, Option<MenuKey>>,
    chosen: Vec<(Id, Via)>,
) -> (Vec<(MenuKey, Via)>, Vec<Item>) {
    let mut presses = Vec::new();
    let mut clicks = Vec::new();
    for (id, via) in chosen {
        let Some(&item) = items.get(&id) else {
            continue;
        };
        let key = keys.get(&id).copied().flatten();
        match handling(app, item, via, key) {
            Handling::Replay(key, via) => presses.push((key, via)),
            Handling::Click => clicks.push(item),
            Handling::Ignore => {}
        }
    }
    (presses, clicks)
}

/// A change the bar shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Change {
    Enabled(Id, bool),
    /// An msgid.
    Title(Id, &'static str),
}

/// What differs between `shown` and `app` now, for each of `items`;
/// `shown` takes the new states. Nothing for an item as it was.
pub(super) fn changes(
    shown: &mut HashMap<Id, ItemState>,
    items: &HashMap<Id, Item>,
    app: &App,
) -> Vec<Change> {
    let mut changes = Vec::new();
    for (&id, &item) in items {
        let state = app.menu_state(item);
        let Some(before) = shown.insert(id, state) else {
            continue;
        };
        if before.enabled != state.enabled {
            changes.push(Change::Enabled(id, state.enabled));
        }
        if before.title != state.title {
            changes.push(Change::Title(id, state.title));
        }
    }
    changes
}

/// The keys the bar was built with, and the keymap and look they came
/// from: what decides when the bar is built again.
pub(super) struct Keyed {
    keymap: Arc<Keymap>,
    layout: Layout,
    /// The key each item was built with.
    pub keys: HashMap<Id, Option<MenuKey>>,
}

impl Keyed {
    /// The keys `app` gives `items` now, as the bar is built with them.
    pub fn new(items: &HashMap<Id, Item>, app: &App) -> Self {
        Self {
            keymap: app.keymap.clone(),
            layout: app.layout(),
            keys: keys_of(items, app),
        }
    }

    /// The keys to build the bar again with, where `app`'s keymap or look
    /// changed and with it an item's key; None where the bar's still
    /// hold. Every settings change makes a new keymap (`refresh_keymap`),
    /// so the pointer only says the keys may have changed. The keymap and
    /// look are recorded either way, so a rebuild muda refuses is asked
    /// once; the keys are the caller's to keep once the bar is built.
    pub fn rekey(
        &mut self,
        items: &HashMap<Id, Item>,
        app: &App,
    ) -> Option<HashMap<Id, Option<MenuKey>>> {
        let layout = app.layout();
        if Arc::ptr_eq(&self.keymap, &app.keymap) && layout == self.layout {
            return None;
        }
        self.keymap = app.keymap.clone();
        self.layout = layout;
        let keys = keys_of(items, app);
        (keys != self.keys).then_some(keys)
    }
}

/// Replays, put only into frames the window reads keys in.
///
/// eframe runs a frame of `logic` alone while the window is minimized or
/// hidden, and keeps its input, `raw_input_hook`'s events among them,
/// for the next frame, which the hook sees again. Read again, a replay's
/// own `ModifiersChanged` would pass for what winit says is held, and a
/// press would land frames late with no release to follow. So a frame
/// the window cannot take keys in gets no replay, and nor does a frame
/// after one that held some of ours and was never drawn.
#[derive(Debug, Default)]
pub(super) struct Feed {
    replay: Replay,
    /// The last frame fed holds events of ours and no pass has drawn it.
    fed: bool,
}

impl Feed {
    /// Puts `presses` into `raw`'s events where the window takes keys
    /// now, and drops them elsewhere: a key down then reaches nothing
    /// without a menu either. Gives whether to ask for another frame
    /// (`Replay::frame`).
    pub fn input(&mut self, raw: &mut egui::RawInput, presses: &[(MenuKey, Via)]) -> bool {
        if self.fed || !takes_keys(raw) {
            return false;
        }
        let before = raw.events.len();
        let again = self.replay.frame(&mut raw.events, presses);
        self.fed = raw.events.len() != before;
        again
    }

    /// A pass drew the frame last fed: egui read its events. Gives
    /// whether a restore waits, for the caller to ask for a frame: the
    /// one `input` asked for may have gone to a frame it fed nothing.
    pub fn drawn(&mut self) -> bool {
        self.fed = false;
        self.replay.restoring()
    }
}

/// The clicks of `clicks` that run in the frame `raw` is the input of:
/// all but a paste where the window takes no keys, which would land in
/// whatever field has the keyboard when it comes back.
pub(super) fn admitted(raw: &egui::RawInput, mut clicks: Vec<Item>) -> Vec<Item> {
    if !takes_keys(raw) {
        clicks.retain(|item| *item != Item::Edit(EditItem::Paste));
    }
    clicks
}

/// Whether the window reads keys in the frame `raw` is the input of:
/// focused, and neither minimized nor hidden.
fn takes_keys(raw: &egui::RawInput) -> bool {
    let shown = raw
        .viewports
        .get(&raw.viewport_id)
        .is_none_or(|info| info.visible() != Some(false));
    raw.focused && shown
}

/// The live bar.
pub struct Bar {
    menu: MenuBar<Id>,
    items: HashMap<Id, Item>,
    /// What each item shows now: at first what it was built with.
    shown: HashMap<Id, ItemState>,
    keyed: Keyed,
    feed: Feed,
    /// Asked for another frame when a replay's modifiers are to be let go.
    ctx: egui::Context,
    /// Clicks for `Window::logic`.
    clicks: Vec<Item>,
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
        let Described {
            menus,
            items,
            shown,
        } = describe(platform, app);
        let wake = ctx.clone();
        let menu = MenuBar::attach(window, menus, move || wake.request_repaint())?;
        let keyed = Keyed::new(&items, app);
        let mut bar = Self {
            menu,
            items,
            shown,
            keyed,
            feed: Feed::default(),
            ctx: ctx.clone(),
            clicks: Vec::new(),
        };
        bar.sync(app);
        Some(bar)
    }

    /// Takes what was chosen since the last frame: what replays goes into
    /// `raw`'s events (`route`, `Feed`); the clicks wait for
    /// [`Bar::clicks`], whether or not the window takes keys, a paste
    /// only where it does (`admitted`).
    pub fn input(&mut self, raw: &mut egui::RawInput, app: &App) {
        let (presses, clicks) = route(app, &self.items, &self.keyed.keys, self.menu.chosen());
        self.clicks.extend(admitted(raw, clicks));
        if self.feed.input(raw, &presses) {
            // What is really held comes back next frame, not with the
            // next input.
            self.ctx.request_repaint();
        }
    }

    /// The clicks [`Bar::input`] kept, oldest first.
    pub fn clicks(&mut self) -> Vec<Item> {
        std::mem::take(&mut self.clicks)
    }

    /// Shows the app as it is after the frame, which a pass drew: built
    /// again where an item's key changed, then each item's state.
    pub fn sync(&mut self, app: &App) {
        if self.feed.drawn() {
            // What is really held comes back next frame, not with the
            // next input.
            self.ctx.request_repaint();
        }
        if let Some(keys) = self.keyed.rekey(&self.items, app)
            && let Some(platform) = Platform::current()
        {
            let Described {
                menus,
                items,
                shown,
            } = describe(platform, app);
            // A refused rebuild keeps the old bar and its keys.
            if self.menu.rebuild(menus) {
                // The same items, from the same model: `keys` are theirs.
                self.keyed.keys = keys;
                self.items = items;
                // A rebuilt bar has every item enabled again.
                self.shown = shown;
            }
        }
        for change in changes(&mut self.shown, &self.items, app) {
            match change {
                Change::Enabled(id, enabled) => self.menu.set_enabled(id, enabled),
                Change::Title(id, msgid) => self.menu.set_title(id, &gettext(app.locale, msgid)),
            }
        }
    }
}

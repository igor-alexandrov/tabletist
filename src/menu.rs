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

/// One menu of the bar: its title and its items, in order.
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
    menus.push(Menu {
        title: "File",
        role: Role::Plain,
        items: file,
    });
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
    view.extend([
        Sep,
        Run(C::LargerText),
        Run(C::SmallerText),
        Run(C::DefaultTextSize),
    ]);
    menus.push(Menu {
        title: "View",
        role: Role::Plain,
        items: view,
    });
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
    menus.push(Menu {
        title: "Window",
        role: Role::Window,
        items: window,
    });
    let mut help = vec![Run(C::Help)];
    if !mac {
        help.extend([Sep, Item::About]);
    }
    menus.push(Menu {
        title: "Help",
        role: Role::Help,
        items: help,
    });
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

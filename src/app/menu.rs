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
                let enabled = self.command_actions(tab, command).is_some() && self.reaches(command);
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

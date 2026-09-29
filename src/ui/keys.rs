//! Keyboard shortcuts. Command means Cmd on macOS and Ctrl elsewhere.

use egui::{Key, Modifiers};

use crate::app::App;
use crate::model::Action;

const NUMBERS: [Key; 9] = [
    Key::Num1,
    Key::Num2,
    Key::Num3,
    Key::Num4,
    Key::Num5,
    Key::Num6,
    Key::Num7,
    Key::Num8,
    Key::Num9,
];

/// Every shortcut, for the help dialog. `Mod` is Cmd on macOS, Ctrl elsewhere.
pub const SHORTCUTS: &[(&str, &str)] = &[
    ("Mod+T", "New connection tab"),
    ("Mod+Shift+W", "Close connection tab"),
    ("Mod+1…9, Ctrl+Tab, Ctrl+Shift+Tab", "Switch connection tab"),
    ("Mod+N", "New connection"),
    ("Mod+W", "Close object tab"),
    ("Mod+Shift+[ / ]", "Previous / next object tab"),
    ("Mod+R", "Refresh"),
    ("Mod+F", "Filter bar"),
    ("Mod+P", "Quick open"),
    ("Mod+Alt+Left / Right", "Previous / next page"),
    ("Mod+.", "Cancel running query"),
    ("Space, Mod+Shift+R", "Toggle row panel"),
    ("Mod+C, Mod+Shift+C", "Copy cell / copy row"),
    ("Arrows, Home/End, Enter", "Move in the tree"),
    ("Arrows, Page Up/Down, Home/End", "Move in the grid"),
    ("?", "Shortcuts"),
];

/// `keys` with `Mod` named for this platform.
pub fn keys_label(keys: &str) -> String {
    let command = if cfg!(target_os = "macos") {
        "Cmd"
    } else {
        "Ctrl"
    };
    keys.replace("Mod", command)
}

pub fn handle(app: &mut App, ctx: &egui::Context) {
    let active = app.active_tab_id();
    let object = app.active_object();
    let editing = ctx.text_edit_focused();
    // Grid keys act only on a visible grid: the Data view of the active tab.
    let grid = object.is_some_and(|(tab, id)| {
        app.workspace(tab)
            .and_then(|workspace| workspace.object_tab(id))
            .is_some_and(|object| object.view == crate::model::ObjectView::Data)
    });
    // Space activates a focused button; it only toggles the panel otherwise.
    let focused = ctx.memory(|memory| memory.focused().is_some());
    let filter_open = object.is_some_and(|(tab, id)| {
        app.workspace(tab)
            .and_then(|workspace| workspace.object_tab(id))
            .is_some_and(|object| {
                object.filter.open && object.view == crate::model::ObjectView::Data
            })
    });
    // Arrows go to the tree when the user last worked there (or no grid is
    // showing), else to the grid.
    let tree_arrows = !editing
        && app
            .workspace(active)
            .is_some_and(|workspace| workspace.pane == crate::model::Pane::Tree || !grid);
    let mut actions = Vec::new();
    ctx.input_mut(|input| {
        let mut key = |modifiers: Modifiers, key: Key, action: Action| {
            if input.consume_key(modifiers, key) {
                actions.push(action);
            }
        };
        // Shift variants first: egui ignores an extra Shift when matching.
        key(
            Modifiers::COMMAND | Modifiers::SHIFT,
            Key::W,
            Action::CloseConnTab(active),
        );
        key(
            Modifiers::COMMAND | Modifiers::SHIFT,
            Key::R,
            Action::ToggleRowPanel(active),
        );
        key(
            Modifiers::CTRL | Modifiers::SHIFT,
            Key::Tab,
            Action::CycleConnTab(-1),
        );
        key(Modifiers::CTRL, Key::Tab, Action::CycleConnTab(1));
        key(Modifiers::COMMAND, Key::T, Action::NewConnTab);
        key(Modifiers::COMMAND, Key::N, Action::NewConnection);
        for (index, number) in NUMBERS.into_iter().enumerate() {
            key(
                Modifiers::COMMAND,
                number,
                Action::ActivateConnTabIndex(index),
            );
        }
        // The tree refreshes itself when it has the arrows (spec 5.10).
        let refresh = if tree_arrows {
            Action::RefreshTree(active)
        } else {
            Action::Refresh(active)
        };
        key(Modifiers::COMMAND, Key::R, refresh);
        key(Modifiers::COMMAND, Key::Period, Action::CancelQuery(active));
        // An open bar without focus gets it back rather than closing.
        let filter_key = if filter_open && !editing {
            Action::FocusFilterBar(active)
        } else {
            Action::ToggleFilterBar(active)
        };
        key(Modifiers::COMMAND, Key::F, filter_key);
        key(Modifiers::COMMAND, Key::P, Action::OpenQuickOpen);
        if tree_arrows {
            use crate::model::TreeKey;
            for (pressed, tree_key) in [
                (Key::ArrowUp, TreeKey::Up),
                (Key::ArrowDown, TreeKey::Down),
                (Key::ArrowLeft, TreeKey::Left),
                (Key::ArrowRight, TreeKey::Right),
                (Key::Home, TreeKey::Home),
                (Key::End, TreeKey::End),
            ] {
                key(
                    Modifiers::NONE,
                    pressed,
                    Action::TreeKey {
                        tab: active,
                        key: tree_key,
                    },
                );
            }
            // Enter belongs to a focused button.
            if !focused {
                key(
                    Modifiers::NONE,
                    Key::Enter,
                    Action::TreeKey {
                        tab: active,
                        key: TreeKey::Enter,
                    },
                );
            }
        }
        if let Some((tab, object_tab)) = object {
            // With Shift held, US layouts report `{` and `}`, so match both.
            for (pressed, step) in [
                (Key::OpenBracket, -1),
                (Key::OpenCurlyBracket, -1),
                (Key::CloseBracket, 1),
                (Key::CloseCurlyBracket, 1),
            ] {
                key(
                    Modifiers::COMMAND | Modifiers::SHIFT,
                    pressed,
                    Action::CycleObjectTab { tab, step },
                );
            }
            key(
                Modifiers::COMMAND | Modifiers::ALT,
                Key::ArrowLeft,
                Action::PrevPage { tab, object_tab },
            );
            key(
                Modifiers::COMMAND | Modifiers::ALT,
                Key::ArrowRight,
                Action::NextPage { tab, object_tab },
            );
            key(
                Modifiers::COMMAND,
                Key::W,
                Action::CloseObjectTab { tab, object_tab },
            );
            if !editing && grid && !tree_arrows {
                let page = 20;
                for (pressed, rows, cols) in [
                    (Key::ArrowUp, -1, 0),
                    (Key::ArrowDown, 1, 0),
                    (Key::ArrowLeft, 0, -1),
                    (Key::ArrowRight, 0, 1),
                    (Key::PageUp, -page, 0),
                    (Key::PageDown, page, 0),
                    (Key::Home, isize::MIN, 0),
                    (Key::End, isize::MAX, 0),
                ] {
                    key(
                        Modifiers::NONE,
                        pressed,
                        Action::MoveSelection {
                            tab,
                            object_tab,
                            rows,
                            cols,
                        },
                    );
                }
                if !focused {
                    key(Modifiers::NONE, Key::Space, Action::ToggleRowPanel(tab));
                }
            }
        }
    });
    if !editing && grid {
        let (copy, shift) = ctx.input(|input| {
            (
                input
                    .events
                    .iter()
                    .any(|event| matches!(event, egui::Event::Copy)),
                input.modifiers.shift,
            )
        });
        if copy && let Some(text) = app.copy_text(shift) {
            ctx.copy_text(text);
        }
    }
    // `?` as typed text, so it works on every keyboard layout.
    if !editing
        && app.dialog.is_none()
        && ctx.input(|input| {
            input
                .events
                .iter()
                .any(|event| matches!(event, egui::Event::Text(text) if text == "?"))
        })
    {
        actions.push(Action::ShowHelp);
    }
    app.actions.extend(actions);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_shortcut_table_covers_the_spec_map() {
        let descriptions: Vec<&str> = SHORTCUTS.iter().map(|(_, what)| *what).collect();
        for expected in [
            "New connection tab",
            "Close connection tab",
            "Switch connection tab",
            "New connection",
            "Close object tab",
            "Previous / next object tab",
            "Refresh",
            "Filter bar",
            "Quick open",
            "Previous / next page",
            "Cancel running query",
            "Toggle row panel",
            "Copy cell / copy row",
            "Move in the tree",
            "Move in the grid",
            "Shortcuts",
        ] {
            assert!(descriptions.contains(&expected), "{expected}");
        }
    }

    #[test]
    fn mod_is_named_for_the_platform() {
        let label = keys_label("Mod+Shift+W");
        if cfg!(target_os = "macos") {
            assert_eq!(label, "Cmd+Shift+W");
        } else {
            assert_eq!(label, "Ctrl+Shift+W");
        }
    }
}

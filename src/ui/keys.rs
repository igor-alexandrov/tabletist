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
    ("Mod+O", "New connection tab"),
    ("Mod+Shift+W", "Close connection tab"),
    ("Mod+1…9, Ctrl+Tab, Ctrl+Shift+Tab", "Switch connection tab"),
    ("Mod+N", "New connection"),
    ("Mod+T", "New SQL editor"),
    ("Mod+Return, Mod+Shift+Return", "Run statement / run all"),
    ("Mod+W", "Close tab"),
    ("Mod+Shift+[ / ]", "Previous / next tab"),
    ("Mod+R", "Refresh"),
    ("Mod+F", "Filter bar"),
    ("Mod+P", "Quick open"),
    ("Mod+B", "Show or hide the sidebar"),
    ("Mod+Alt+Left / Right", "Previous / next page"),
    ("Mod+.", "Cancel running query"),
    ("Space, Mod+Shift+R", "Toggle row panel"),
    ("Mod+C, Mod+Shift+C", "Copy cell / copy row"),
    (
        "Arrows, Enter, Mod+E, Mod+D, Mod+Backspace",
        "Pick, edit, duplicate or delete a connection",
    ),
    ("Arrows, Home/End, Enter", "Move in the tree"),
    ("Arrows, Page Up/Down, Home/End", "Move in the grid"),
    (
        "j/k, h/l, [ ], i, Esc, /, y, s, d, gd, za, t, 1…9",
        "Omarchy: vim keys (shown in the status line)",
    ),
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
    let sql = app.active_sql();
    let any_tab = app.active_workspace_tab();
    let in_workspace = app.workspace(active).is_some();
    let editing = ctx.text_edit_focused();
    // Grid keys act only on a visible grid: the Data view of the active tab.
    let grid = object.is_some_and(|(tab, id)| {
        app.workspace(tab)
            .and_then(|workspace| workspace.object_tab(id))
            .is_some_and(|object| object.view == crate::model::ObjectView::Data)
    });
    // A SQL editor's result is a grid too, while its Results pane shows it.
    let sql_grid = sql.is_some_and(|(tab, id)| {
        app.workspace(tab)
            .and_then(|workspace| workspace.sql_tab(id))
            .is_some_and(shows_grid)
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
        && app.workspace(active).is_some_and(|workspace| {
            workspace.pane == crate::model::Pane::Tree || !(grid || sql_grid)
        });
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
        // Running works while typing: the editor never sees these.
        if let Some((tab, sql_tab)) = sql {
            key(
                Modifiers::COMMAND | Modifiers::SHIFT,
                Key::Enter,
                Action::RunSql {
                    tab,
                    sql_tab,
                    all: true,
                },
            );
            key(
                Modifiers::COMMAND,
                Key::Enter,
                Action::RunSql {
                    tab,
                    sql_tab,
                    all: false,
                },
            );
        }
        // A SQL editor has no row panel, and nothing to refresh or filter.
        let on_sql = sql.is_some();
        if !on_sql {
            key(
                Modifiers::COMMAND | Modifiers::SHIFT,
                Key::R,
                Action::ToggleRowPanel(active),
            );
        }
        key(
            Modifiers::CTRL | Modifiers::SHIFT,
            Key::Tab,
            Action::CycleConnTab(-1),
        );
        key(Modifiers::CTRL, Key::Tab, Action::CycleConnTab(1));
        // A SQL editor belongs to a workspace: the picker has none to open.
        if in_workspace {
            key(Modifiers::COMMAND, Key::T, Action::NewSqlTab(active));
        }
        key(Modifiers::COMMAND, Key::O, Action::NewConnTab);
        key(Modifiers::COMMAND, Key::N, Action::NewConnection);
        for (index, number) in NUMBERS.into_iter().enumerate() {
            key(
                Modifiers::COMMAND,
                number,
                Action::ActivateConnTabIndex(index),
            );
        }
        if !on_sql {
            // The tree refreshes itself when it has the arrows (spec 5.10).
            let refresh = if tree_arrows {
                Action::RefreshTree(active)
            } else {
                Action::Refresh(active)
            };
            key(Modifiers::COMMAND, Key::R, refresh);
        }
        key(Modifiers::COMMAND, Key::Period, Action::CancelQuery(active));
        if !on_sql {
            // An open bar without focus gets it back rather than closing.
            let filter_key = if filter_open && !editing {
                Action::FocusFilterBar(active)
            } else {
                Action::ToggleFilterBar(active)
            };
            key(Modifiers::COMMAND, Key::F, filter_key);
        }
        key(Modifiers::COMMAND, Key::P, Action::OpenQuickOpen);
        key(Modifiers::COMMAND, Key::B, Action::ToggleSidebar(active));
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
        // Paging is a table's: a SQL result has one page.
        if let Some((tab, object_tab)) = object {
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
        }
        // The rest acts on the tab the workspace shows, of either kind.
        if let Some((tab, id)) = any_tab {
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
                    Action::CycleTab { tab, step },
                );
            }
            key(Modifiers::COMMAND, Key::W, Action::CloseTab { tab, id });
            if !editing && (grid || sql_grid) && !tree_arrows {
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
                            id,
                            rows,
                            cols,
                        },
                    );
                }
                // The row panel shows a table's row, not a SQL result's.
                if grid && !focused {
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
    if !editing && app.dialog.is_none() {
        letters(app, ctx, &mut actions);
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

/// Whether `sql` shows a result grid for the keys to move in: its last
/// rows, unless the Messages pane covers them.
fn shows_grid(sql: &crate::model::SqlTab) -> bool {
    sql.pane == crate::model::ResultPane::Results && sql.dims().0 > 0
}

/// The first key of a two-key command (`dd`, `gd`), kept between frames.
fn pending_id() -> egui::Id {
    egui::Id::new("pending-key")
}

/// Whether `text` was typed this frame (consumed): keys named by the
/// character they type, so they work on every keyboard layout.
fn typed(ctx: &egui::Context, text: &str) -> bool {
    ctx.input_mut(|input| {
        let before = input.events.len();
        input
            .events
            .retain(|event| !matches!(event, egui::Event::Text(typed) if typed == text));
        input.events.len() != before
    })
}

/// Single letters: the picker's keys, and in the terminal look the
/// workspace's vim keys. Only when no text field has the keyboard.
fn letters(app: &mut App, ctx: &egui::Context, actions: &mut Vec<Action>) {
    // A frame without keys keeps a waiting first key (`d` of `dd`).
    let keys = ctx.input(|input| {
        input
            .events
            .iter()
            .any(|event| matches!(event, egui::Event::Key { pressed: true, .. }))
    });
    if !keys {
        return;
    }
    let tab = app.active_tab_id();
    let terminal = app.look.terminal;
    let pending: Option<char> = ctx.data(|data| data.get_temp(pending_id())).flatten();
    let mut next_pending = None;
    let pressed = |key: Key| ctx.input_mut(|input| input.consume_key(Modifiers::NONE, key));
    let command = |key: Key| ctx.input_mut(|input| input.consume_key(Modifiers::COMMAND, key));
    if let crate::model::ConnTabContent::Picker(picker) = &app.active_tab().content {
        let selected = picker.selected.clone();
        if pressed(Key::ArrowDown) || (terminal && pressed(Key::J)) {
            actions.push(Action::MovePickerSelection { tab, step: 1 });
        }
        if pressed(Key::ArrowUp) || (terminal && pressed(Key::K)) {
            actions.push(Action::MovePickerSelection { tab, step: -1 });
        }
        if let Some(conn) = selected {
            if ctx.memory(|memory| memory.focused().is_none()) && pressed(Key::Enter) {
                actions.push(Action::Connect {
                    tab,
                    conn: conn.clone(),
                });
            }
            if command(Key::E) || (terminal && pressed(Key::E)) {
                actions.push(Action::EditConnection(conn.clone()));
            }
            if command(Key::D) {
                actions.push(Action::DuplicateConnection(conn.clone()));
            }
            if ctx.input_mut(|input| input.consume_key(Modifiers::COMMAND, Key::Backspace)) {
                actions.push(Action::DeleteConnection(conn.clone()));
            }
            if terminal && pressed(Key::Y) {
                if pending == Some('y') {
                    actions.push(Action::DuplicateConnection(conn.clone()));
                } else {
                    next_pending = Some('y');
                }
            }
            if terminal && pressed(Key::D) {
                if pending == Some('d') {
                    actions.push(Action::DeleteConnection(conn));
                } else {
                    next_pending = Some('d');
                }
            }
        }
        if terminal && pressed(Key::N) {
            actions.push(Action::NewConnection);
        }
        if terminal && typed(ctx, "/") {
            actions.push(Action::FocusPickerSearch(tab));
        }
        ctx.data_mut(|data| data.insert_temp(pending_id(), next_pending));
        return;
    }
    if !terminal {
        return;
    }
    let Some(workspace) = app.workspace(tab) else {
        return;
    };
    let tree = workspace.pane == crate::model::Pane::Tree;
    let panel = workspace.row_panel;
    // The digits follow the strip, so they reach SQL editors too.
    let tabs: Vec<_> = workspace.tabs.iter().map(crate::model::Tab::id).collect();
    let active = workspace.active_object_tab().map(|object| object.id);
    // A SQL editor showing its result takes the letters that move in it.
    let sql_grid = workspace
        .active_sql_tab()
        .filter(|sql| shows_grid(sql))
        .map(|sql| sql.id);
    for (index, number) in NUMBERS.into_iter().enumerate() {
        if pressed(number)
            && let Some(id) = tabs.get(index)
        {
            actions.push(Action::ActivateTab { tab, id: *id });
        }
    }
    if pressed(Key::T) {
        actions.push(Action::ToggleFlatTree(tab));
    }
    if tree {
        if pressed(Key::J) {
            actions.push(Action::TreeKey {
                tab,
                key: crate::model::TreeKey::Down,
            });
        }
        if pressed(Key::K) {
            actions.push(Action::TreeKey {
                tab,
                key: crate::model::TreeKey::Up,
            });
        }
    }
    // What `j/k/h/l` move in: the object's grid, or a SQL result.
    if !tree && let Some(id) = active.or(sql_grid) {
        for (key, rows, cols) in [
            (Key::J, 1, 0),
            (Key::K, -1, 0),
            (Key::H, 0, -1),
            (Key::L, 0, 1),
        ] {
            if pressed(key) {
                actions.push(Action::MoveSelection {
                    tab,
                    id,
                    rows,
                    cols,
                });
            }
        }
    }
    // The letters below act on an object tab: none of them on a SQL editor.
    let Some(object_tab) = active else {
        ctx.data_mut(|data| data.insert_temp(pending_id(), next_pending));
        return;
    };
    let step = |rows: isize, cols: isize| Action::MoveSelection {
        tab,
        id: object_tab,
        rows,
        cols,
    };
    if !tree && !panel && pressed(Key::Enter) {
        actions.push(Action::ToggleRowPanel(tab));
    }
    if typed(ctx, "[") {
        actions.push(step(-1, 0));
    }
    if typed(ctx, "]") {
        actions.push(step(1, 0));
    }
    if pressed(Key::I) {
        actions.push(Action::ToggleRowPanel(tab));
    }
    if panel && pressed(Key::Escape) {
        actions.push(Action::ToggleRowPanel(tab));
    }
    if typed(ctx, "/") {
        actions.push(Action::FocusWhere(tab));
    }
    if pressed(Key::S) {
        actions.push(Action::SetView {
            tab,
            object_tab,
            view: crate::model::ObjectView::Structure,
        });
    }
    if pressed(Key::Y)
        && let Some(text) = app.copy_text(false)
    {
        ctx.copy_text(text);
    }
    if pressed(Key::G) {
        next_pending = Some('g');
    }
    if pressed(Key::Z) {
        next_pending = Some('z');
    }
    if pressed(Key::A) && pending == Some('z') {
        actions.push(Action::FoldDocuments { tab, object_tab });
    }
    if pressed(Key::D) {
        if pending == Some('g') {
            actions.push(Action::FollowSelectedKey { tab, object_tab });
        } else {
            actions.push(Action::SetView {
                tab,
                object_tab,
                view: crate::model::ObjectView::Data,
            });
        }
    }
    ctx.data_mut(|data| data.insert_temp(pending_id(), next_pending));
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
            "New SQL editor",
            "Run statement / run all",
            "Close tab",
            "Previous / next tab",
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
    fn the_shortcut_table_names_the_keys_that_open_tabs() {
        let keys = |what: &str| {
            SHORTCUTS
                .iter()
                .find(|(_, description)| *description == what)
                .map(|(keys, _)| *keys)
        };
        assert_eq!(keys("New SQL editor"), Some("Mod+T"));
        assert_eq!(keys("New connection tab"), Some("Mod+O"));
        assert_eq!(
            keys("Run statement / run all"),
            Some("Mod+Return, Mod+Shift+Return")
        );
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

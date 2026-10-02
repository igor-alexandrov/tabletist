//! Keyboard shortcuts. Command means Cmd on macOS and Ctrl elsewhere.

use egui::{Key, Modifiers};

use crate::app::App;
use crate::model::{Action, ConnTabId, TabId};

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
    ("Mod+O", "Connections"),
    ("Mod+Shift+W", "Close connection"),
    ("Mod+1…9, Ctrl+Tab, Ctrl+Shift+Tab", "Switch connection"),
    ("Mod+N", "New connection"),
    (
        "Mod+S, Mod+T, Mod+Enter",
        "Save, test, or save and connect in the connection dialog",
    ),
    ("Mod+T", "New SQL editor"),
    ("Mod+Return, Mod+Shift+Return", "Run statement / run all"),
    ("Mod+Shift+F", "Format SQL"),
    ("Ctrl+Space, Mod+I", "Complete in the SQL editor"),
    ("Mod+W", "Close tab"),
    ("Mod+Shift+[ / ]", "Previous / next tab"),
    ("Mod+R", "Refresh"),
    ("Mod+F", "Filter bar"),
    ("Mod+P", "Quick open"),
    ("Mod+B", "Show or hide the sidebar"),
    ("Mod+Alt+Left / Right", "Previous / next page"),
    ("Mod+.", "Cancel running query"),
    ("Esc", "Cancel connecting"),
    ("Space, Mod+Shift+R", "Toggle row panel"),
    ("Mod+C, Mod+Shift+C", "Copy cell / copy row"),
    (
        "Arrows, Enter, Shift+Enter, Mod+E, Mod+D, Mod+Backspace",
        "Pick, open again, edit, duplicate or delete a connection",
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
    // A connect with nothing to lose: Esc gives up, as its Cancel does. An
    // open popup keeps its Esc: this runs before the popup is drawn.
    let give_up = !egui::Popup::is_any_open(ctx)
        && app
            .workspace(active)
            .is_some_and(|workspace| workspace.can_give_up());
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
    // A SQL editor's row panel is its selected row's: with none selected
    // the panel's keys have nothing to show or hide.
    let sql_row = sql.is_some_and(|(tab, id)| {
        app.workspace(tab)
            .and_then(|workspace| workspace.sql_tab(id))
            .is_some_and(|sql| sql.selected_row().is_some())
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
    let any_grid = grid || sql_grid;
    let tree_arrows = !editing
        && app
            .workspace(active)
            .is_some_and(|workspace| workspace.pane == crate::model::Pane::Tree || !any_grid);
    // The SQL editor on screen, while it has the keyboard: the completion
    // list's keys are its keys, and no other pane's. (A text field then
    // has the keyboard: no need to ask `editing` as well.)
    let in_editor = sql.filter(|(tab, id)| {
        let editor = crate::ui::sql_text::editor_id(*tab, *id);
        ctx.memory(|memory| memory.has_focus(editor))
    });
    // Its open list. One whose row is about to be inserted is not open any
    // more: the keys of that frame are the editor's.
    let completing = in_editor.and_then(|(tab, id)| {
        let sql = app.workspace(tab)?.sql_tab(id)?;
        let list = sql.completion.as_ref().filter(|list| !list.accept)?;
        Some(Completing {
            has_row: !list.candidates.is_empty(),
            enter_breaks: list.enter_is_a_line_break(&sql.text),
        })
    });
    let terminal = app.look.terminal;
    let mut actions = Vec::new();
    ctx.input_mut(|input| {
        // The completion list's keys come first: the editor never sees
        // them, nor do the shortcuts below (Ctrl+N, Ctrl+P).
        if let Some(editor) = in_editor {
            completion_keys(input, editor, completing, terminal, &mut actions);
        }
        // Running works while typing: the editor never sees these. A held
        // chord runs once, or each repeat would cancel the run before it.
        // Shift variants first: egui ignores an extra Shift when matching.
        if let Some((tab, sql_tab)) = sql {
            for (modifiers, all) in [
                (Modifiers::COMMAND | Modifiers::SHIFT, true),
                (Modifiers::COMMAND, false),
            ] {
                if consume_press(input, modifiers, Key::Enter) {
                    actions.push(Action::RunSql { tab, sql_tab, all });
                }
            }
        }
        // Format is a SQL editor's. The press is taken on every tab, or
        // Mod+F, below, would take it for its own.
        let format = consume_press(input, Modifiers::COMMAND | Modifiers::SHIFT, Key::F);
        if format && let Some((tab, sql_tab)) = sql {
            actions.push(Action::FormatSql { tab, sql_tab });
        }
        // A fresh press only: an Esc held to close a dialog over the tab
        // repeats after the dialog is gone.
        if give_up && consume_press(input, Modifiers::NONE, Key::Escape) {
            actions.push(Action::Disconnect(active));
        }
        let mut key = |modifiers: Modifiers, key: Key, action: Action| {
            if input.consume_key(modifiers, key) {
                actions.push(action);
            }
        };
        key(
            Modifiers::COMMAND | Modifiers::SHIFT,
            Key::W,
            Action::CloseConnTab(active),
        );
        // A SQL editor has nothing to refresh or filter, and a row panel
        // only for a selected row of its result.
        let on_sql = sql.is_some();
        if !on_sql || sql_row {
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
        key(Modifiers::COMMAND, Key::O, Action::ShowConnections);
        key(Modifiers::COMMAND, Key::N, Action::NewConnection);
        for (index, number) in NUMBERS.into_iter().enumerate() {
            key(
                Modifiers::COMMAND,
                number,
                Action::ActivateConnection(index),
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
            if !editing && any_grid && !tree_arrows {
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
                // The row panel shows a table's row, or the selected row
                // of a SQL result.
                if (grid || sql_row) && !focused {
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

/// Whether `key` with `modifiers` went down this frame. Consumes the
/// repeats of a held key as well, which do not count as a press.
fn consume_press(input: &mut egui::InputState, modifiers: Modifiers, key: Key) -> bool {
    let mut fresh = false;
    input.events.retain(|event| match event {
        egui::Event::Key {
            key: pressed,
            modifiers: held,
            pressed: true,
            repeat,
            ..
        } if *pressed == key && held.matches_logically(modifiers) => {
            fresh |= !repeat;
            false
        }
        _ => true,
    });
    fresh
}

/// The open completion list of the editor that has the keyboard, as its
/// keys read it.
struct Completing {
    /// It has a row to move to and to insert.
    has_row: bool,
    /// Enter stays the editor's line break: the highlighted row is what
    /// is already typed, or only a guess at a word that may be whole (see
    /// `Completion::enter_is_a_line_break`).
    enter_breaks: bool,
}

/// The keys of the completion list of `editor`, which has the keyboard:
/// `Ctrl+Space` asks for a list, and an open one (`completing`) takes the
/// keys that move in it, insert from it and close it.
fn completion_keys(
    input: &mut egui::InputState,
    (tab, sql_tab): (ConnTabId, TabId),
    completing: Option<Completing>,
    terminal: bool,
    actions: &mut Vec<Action>,
) {
    // Two chords ask for a list: some systems take Ctrl+Space before the
    // app sees it (macOS switches input sources with it, and an input
    // method may be woken by it).
    let asked = take_press(input, Modifiers::CTRL, Key::Space)
        + take_press(input, Modifiers::COMMAND, Key::I);
    if asked > 0 {
        actions.push(Action::OpenCompletion { tab, sql_tab });
    }
    let Some(list) = completing else {
        return;
    };
    // The arrows are the list's once it has a row to move to. The
    // terminal's Ctrl+N and Ctrl+P are its own as long as it is open, rows
    // or not: they never reach New connection and Quick open.
    for (modifiers, key, step, taken) in [
        (Modifiers::NONE, Key::ArrowDown, 1, list.has_row),
        (Modifiers::NONE, Key::ArrowUp, -1, list.has_row),
        (Modifiers::CTRL, Key::N, 1, terminal),
        (Modifiers::CTRL, Key::P, -1, terminal),
    ] {
        if taken {
            // One row for each press: a frame may hold several.
            let presses = take_press(input, modifiers, key);
            actions.extend((0..presses).map(|_| Action::MoveCompletion { tab, sql_tab, step }));
        }
    }
    let accept = Action::AcceptCompletion {
        tab,
        sql_tab,
        row: None,
    };
    let close = Action::CloseCompletion { tab, sql_tab };
    let pressed = |input: &egui::InputState, key: Key| {
        let is_it = |event: &egui::Event| is_press(event, Modifiers::NONE, key);
        input.events.iter().any(is_it)
    };
    if list.has_row {
        if input.events.iter().any(inserts_text) {
            // What this frame types is not what the list was worked out
            // from: no row goes in. Tab is taken all the same, or it would
            // put a tab character into the word being typed; whether the
            // list goes on is the refresh's to say, for the word as it
            // reads after this frame. Enter stays the editor's line break,
            // and the list is done.
            take_press(input, Modifiers::NONE, Key::Tab);
            if pressed(input, Key::Enter) {
                actions.push(close);
            }
        } else if take_press(input, Modifiers::NONE, Key::Tab) > 0 {
            actions.push(accept);
        } else if list.enter_breaks {
            // The editor gets its line break; the list is done.
            if pressed(input, Key::Enter) {
                actions.push(close);
            }
        } else if take_press(input, Modifiers::NONE, Key::Enter) > 0 {
            actions.push(accept);
        }
    }
    if take_press(input, Modifiers::NONE, Key::Escape) > 0 {
        actions.push(Action::CloseCompletion { tab, sql_tab });
    }
}

/// Whether `event` puts text into the field that has the keyboard: typed
/// text, as the SQL editor takes it, or a paste.
fn inserts_text(event: &egui::Event) -> bool {
    crate::ui::sql_text::is_typed(event)
        || matches!(event, egui::Event::Paste(text) if !text.is_empty())
}

/// Whether `event` is `key` going down with exactly `modifiers`: Shift and
/// Alt as named (`consume_key` and `matches_logically` let an extra Shift
/// through), and no Ctrl or Cmd that is not named.
///
/// Ctrl is named by `CTRL` and matches as every platform reports it: alone
/// on macOS, with `command` set elsewhere. Ctrl held with Cmd on macOS is
/// not Ctrl. Mod is named by `COMMAND`: Cmd on macOS, where an extra Ctrl
/// changes nothing (as for the other Mod shortcuts), and Ctrl elsewhere.
fn is_press(event: &egui::Event, modifiers: Modifiers, key: Key) -> bool {
    matches!(
        event,
        egui::Event::Key {
            key: pressed,
            modifiers: held,
            pressed: true,
            ..
        } if *pressed == key
            && held.matches_exact(modifiers)
            // egui lets the pattern's Ctrl match Ctrl+Cmd. Cmd is held
            // only where the pattern names it, or names Mod.
            && (modifiers.mac_cmd || modifiers.command || !held.mac_cmd)
    )
}

/// How many times `key` went down this frame with exactly `modifiers` (see
/// [`is_press`]; a held key's repeats count). Takes the presses, so
/// neither the editor nor a shortcut after this sees them.
fn take_press(input: &mut egui::InputState, modifiers: Modifiers, key: Key) -> usize {
    let before = input.events.len();
    input
        .events
        .retain(|event| !is_press(event, modifiers, key));
    before - input.events.len()
}

/// Whether `sql` shows a result grid for the keys to move in: its last
/// run's rows, unless the Messages pane covers them.
fn shows_grid(sql: &crate::model::SqlTab) -> bool {
    sql.pane == crate::model::ResultPane::Results && sql.dims().0 > 0
}

/// The first key of a two-key command (`dd`, `gd`), kept between frames.
fn pending_id() -> egui::Id {
    egui::Id::new("pending-key")
}

/// Whether a text field had the keyboard when the last frame ended.
fn was_editing_id() -> egui::Id {
    egui::Id::new("was-editing")
}

/// Notes, once the frame is drawn, whether a text field has the keyboard.
/// egui takes it away on Escape before the next frame's keys are read,
/// so only this tells an Escape that left a field from one pressed
/// outside it.
pub fn after_frame(ctx: &egui::Context) {
    let editing = ctx.text_edit_focused();
    ctx.data_mut(|data| data.insert_temp(was_editing_id(), editing));
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
    let left_field: bool = ctx
        .data(|data| data.get_temp(was_editing_id()))
        .unwrap_or(false);
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
            if ctx.memory(|memory| memory.focused().is_none()) {
                // Shift first: egui ignores an extra Shift when matching.
                let again = ctx.input_mut(|input| input.consume_key(Modifiers::SHIFT, Key::Enter));
                if again || pressed(Key::Enter) {
                    // Enter shows a connection that is open already; with
                    // Shift it opens once more.
                    actions.push(match app.tab_showing(&conn).filter(|_| !again) {
                        Some(open) => Action::ActivateConnTab(open),
                        None => Action::Connect {
                            tab,
                            conn: conn.clone(),
                        },
                    });
                }
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
    let shown = workspace.row_panel_tab();
    // The digits follow the strip, so they reach SQL editors too.
    let tabs: Vec<_> = workspace.tabs.iter().map(crate::model::Tab::id).collect();
    let active = workspace.active_object_tab().map(|object| object.id);
    // A SQL editor showing its result takes the letters that move in it.
    let sql_grid = workspace
        .active_sql_tab()
        .filter(|sql| shows_grid(sql))
        .map(|sql| sql.id);
    // And, with a row of it selected, the letters of the row panel.
    let sql_row = workspace
        .active_sql_tab()
        .filter(|sql| sql.selected_row().is_some())
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
    // `[` and `]` step through the rows of the grid `j/k` move in.
    if let Some(id) = active.or(sql_grid) {
        for (text, rows) in [("[", -1), ("]", 1)] {
            if typed(ctx, text) {
                actions.push(Action::MoveSelection {
                    tab,
                    id,
                    rows,
                    cols: 0,
                });
            }
        }
    }
    // The row panel's keys: an object tab's, or a SQL result's while a row
    // of it is selected (with none its panel has nothing to show).
    if let Some(id) = active.or(sql_row) {
        // Enter belongs to a focused button.
        let focused = ctx.memory(|memory| memory.focused().is_some());
        if !tree && !panel && !focused && pressed(Key::Enter) {
            actions.push(Action::ToggleRowPanel(tab));
        }
        if pressed(Key::I) {
            actions.push(Action::ToggleRowPanel(tab));
        }
        // An Esc that left a text field did only that.
        if panel && !left_field && pressed(Key::Escape) {
            actions.push(Action::ToggleRowPanel(tab));
        }
        if pressed(Key::Z) {
            next_pending = Some('z');
        }
        // Only the documents of a panel that shows.
        if pressed(Key::A) && pending == Some('z') && shown == Some(id) {
            actions.push(Action::FoldDocuments { tab, id });
        }
    }
    // The letters below act on an object tab: none of them on a SQL editor.
    let Some(object_tab) = active else {
        ctx.data_mut(|data| data.insert_temp(pending_id(), next_pending));
        return;
    };
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
    fn the_shortcut_table_lists_the_connection_dialogs_keys() {
        assert!(
            SHORTCUTS
                .iter()
                .any(|(keys, what)| keys.contains("Mod+S") && what.contains("connection dialog"))
        );
    }

    #[test]
    fn the_shortcut_table_covers_the_spec_map() {
        let descriptions: Vec<&str> = SHORTCUTS.iter().map(|(_, what)| *what).collect();
        for expected in [
            "Connections",
            "Close connection",
            "Switch connection",
            "New connection",
            "New SQL editor",
            "Run statement / run all",
            "Format SQL",
            "Complete in the SQL editor",
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
        assert_eq!(keys("Connections"), Some("Mod+O"));
        assert_eq!(
            keys("Run statement / run all"),
            Some("Mod+Return, Mod+Shift+Return")
        );
        assert_eq!(keys("Format SQL"), Some("Mod+Shift+F"));
        // Ctrl+Space is taken by some systems before the app sees it.
        assert_eq!(
            keys("Complete in the SQL editor"),
            Some("Ctrl+Space, Mod+I")
        );
    }

    #[test]
    fn a_press_matches_its_modifiers_as_each_platform_reports_them() {
        let press = |held| crate::testing::key(Key::I, held);
        let is = |held, pattern| is_press(&press(held), pattern, Key::I);
        let cmd = Modifiers::MAC_CMD | Modifiers::COMMAND;
        let ctrl_elsewhere = Modifiers::CTRL | Modifiers::COMMAND;
        // Mod: Cmd on macOS, with Ctrl or without, and Ctrl elsewhere.
        for held in [
            Modifiers::COMMAND,
            cmd,
            cmd | Modifiers::CTRL,
            ctrl_elsewhere,
        ] {
            assert!(is(held, Modifiers::COMMAND), "{held:?}");
        }
        for held in [
            Modifiers::NONE,
            Modifiers::CTRL,
            cmd | Modifiers::SHIFT,
            ctrl_elsewhere | Modifiers::ALT,
        ] {
            assert!(!is(held, Modifiers::COMMAND), "{held:?}");
        }
        // Ctrl: alone on macOS, with the command key elsewhere, never Cmd.
        for held in [Modifiers::CTRL, ctrl_elsewhere] {
            assert!(is(held, Modifiers::CTRL), "{held:?}");
        }
        for held in [
            Modifiers::NONE,
            cmd,
            cmd | Modifiers::CTRL,
            Modifiers::CTRL | Modifiers::SHIFT,
        ] {
            assert!(!is(held, Modifiers::CTRL), "{held:?}");
        }
        // No modifier is none at all.
        assert!(is(Modifiers::NONE, Modifiers::NONE));
        for held in [Modifiers::SHIFT, Modifiers::ALT, Modifiers::CTRL, cmd] {
            assert!(!is(held, Modifiers::NONE), "{held:?}");
        }
        // Another key, and a key coming up, are not this press.
        assert!(!is_press(&press(Modifiers::NONE), Modifiers::NONE, Key::O));
        let up = crate::testing::release(Key::I, Modifiers::NONE);
        assert!(!is_press(&up, Modifiers::NONE, Key::I));
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

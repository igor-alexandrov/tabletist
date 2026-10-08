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

/// The looks a shortcut holds in. The help dialog lists a row only where
/// its keys are the look's own: a cell is edited with chords on macOS and
/// Windows, and with letters and the `:` prompt on Omarchy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Holds {
    All,
    /// macOS and Windows.
    Desktop,
    /// Omarchy.
    Terminal,
}

const ALL: Holds = Holds::All;
const DESKTOP: Holds = Holds::Desktop;
const TERMINAL: Holds = Holds::Terminal;

/// Every shortcut, for the help dialog, with the looks it holds in. `Mod`
/// is Cmd on macOS, Ctrl elsewhere.
pub const SHORTCUTS: &[(&str, &str, Holds)] = &[
    ("Mod+O", "Connections", ALL),
    ("Mod+Shift+W", "Close connection", ALL),
    (
        "Mod+1…9, Ctrl+Tab, Ctrl+Shift+Tab",
        "Switch connection",
        ALL,
    ),
    ("Mod+N", "New connection", ALL),
    (
        "Mod+S, Mod+T, Mod+Enter",
        "Save, test, or save and connect in the connection dialog",
        ALL,
    ),
    ("Mod+T", "New SQL editor", ALL),
    (
        "Mod+Return, Mod+Shift+Return",
        "Run statement / run all",
        ALL,
    ),
    ("Mod+Shift+F", "Format SQL", ALL),
    ("Mod+Shift+F", "Format a document being edited", ALL),
    (
        "Mod+Shift+M",
        "Read-only or read-write runs in the SQL editor",
        ALL,
    ),
    ("Ctrl+Space, Mod+I", "Complete in the SQL editor", ALL),
    ("Mod+W", "Close tab", ALL),
    ("Mod+Shift+[ / ]", "Previous / next tab", ALL),
    ("Mod+R", "Refresh", ALL),
    ("Mod+F", "Filter bar", ALL),
    ("Mod+P", "Quick open", ALL),
    ("Mod+B", "Show or hide the sidebar", ALL),
    ("F6, Shift+F6", "Next / previous part of the window", ALL),
    ("Mod+Alt+Left / Right", "Previous / next page", ALL),
    ("Mod+.", "Cancel running query", ALL),
    ("Esc", "Cancel connecting", ALL),
    ("Space, Mod+Shift+R", "Toggle row panel", ALL),
    ("Mod+C, Mod+Shift+C", "Copy cell / copy row", ALL),
    (
        "Arrows, Enter, Shift+Enter, Mod+E, Mod+D, Mod+Backspace",
        "Pick, open again, edit, duplicate or delete a connection",
        ALL,
    ),
    ("Arrows, Home/End, Enter", "Move in the tree", ALL),
    ("Arrows, Page Up/Down, Home/End", "Move in the grid", ALL),
    // Editing a table's cells: each look's own keys for the same things.
    ("Enter, F2", "Edit the cell", DESKTOP),
    ("i, Enter", "Edit the cell", TERMINAL),
    ("Mod+N", "Add a row", DESKTOP),
    ("o, O", "Add a row below or above the cursor", TERMINAL),
    ("Delete", "Drop a new row", DESKTOP),
    ("dd", "Drop a new row", TERMINAL),
    ("Mod+I", "Focus inspector fields", DESKTOP),
    ("Ctrl+L, Mod+I", "Focus inspector fields", TERMINAL),
    (
        "j/k, Ctrl+H",
        "Step the inspector's fields, back to the grid",
        TERMINAL,
    ),
    (
        "Up, Down, Esc",
        "Step the inspector's fields, back to the grid",
        DESKTOP,
    ),
    ("cc", "Edit the cell from nothing", TERMINAL),
    ("Tab, Shift+Tab", "Commit and move right or left", ALL),
    ("Esc", "Cancel the edit", DESKTOP),
    ("Esc", "Leave the editor and keep the edit", TERMINAL),
    ("Ctrl+C", "Drop the edit", TERMINAL),
    ("Mod+Backspace", "Set NULL", DESKTOP),
    ("x", "Set NULL", TERMINAL),
    ("Mod+Z", "Revert the cell", DESKTOP),
    ("u", "Revert the cell", TERMINAL),
    ("Mod+S", "Save all pending changes", DESKTOP),
    (":w, Mod+S", "Save all pending changes", TERMINAL),
    ("Mod+Alt+Backspace", "Discard all pending changes", DESKTOP),
    (":e!", "Discard all pending changes", TERMINAL),
    (
        "Mod+Shift+D",
        "Show or hide the SQL of the pending changes",
        ALL,
    ),
    (":diff", "Show the SQL of the pending changes", TERMINAL),
    ("Esc", "Close the SQL of the pending changes", TERMINAL),
    ("Y", "Copy the SQL of the pending changes", TERMINAL),
    (
        "j/k, h/l, Ctrl+H/L, [ ], i, Enter, cc, x, u, o, O, dd, Mod+S, :w, :e!, :diff, Y, Space, Esc, /, y, s, d, gd, za, t, 1…9",
        "Omarchy: vim keys (shown in the status line)",
        ALL,
    ),
    ("Mod+,", "Settings", ALL),
    ("?", "Shortcuts", ALL),
];

/// The shortcuts that hold in `look`, as the help dialog lists them: its
/// keys and what they do. None that only another look has.
pub fn shortcuts(look: &crate::theme::Look) -> impl Iterator<Item = (&'static str, &'static str)> {
    let terminal = look.terminal;
    SHORTCUTS
        .iter()
        .filter(move |(_, _, holds)| match holds {
            Holds::All => true,
            Holds::Desktop => !terminal,
            Holds::Terminal => terminal,
        })
        .map(|(keys, what, _)| (*keys, *what))
}

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
    // Whether the table on screen has an editor open on a cell.
    let open = object.is_some_and(|(tab, id)| {
        app.workspace(tab)
            .and_then(|workspace| workspace.object_tab(id))
            .is_some_and(|object| object.edits.editor.is_some())
    });
    // The field of a cell's editor that just closed holds egui's focus
    // until the frame it is not drawn in ends. The keys are the grid's
    // already: what is pressed right after a commit is not lost.
    if let Some((tab, id)) = object
        && !open
    {
        let field = crate::ui::cell_editor::field_id(tab, id);
        ctx.memory_mut(|memory| memory.surrender_focus(field));
    }
    // Grid keys act only on a visible grid: the Data view of the active tab.
    let grid = object.is_some_and(|(tab, id)| {
        app.workspace(tab)
            .and_then(|workspace| workspace.object_tab(id))
            .is_some_and(|object| object.view == crate::model::ObjectView::Data)
    });
    let terminal = app.look.terminal;
    // Mod+N adds a row where a table's rows are in front and the table
    // takes one, as the design scopes the key to a table, in the looks
    // whose key for it this is. Everywhere else it is a new connection.
    let adds = object.filter(|_| grid && !terminal).filter(|&(tab, id)| {
        app.workspace(tab)
            .and_then(|workspace| {
                let object = workspace.object_tab(id)?;
                crate::edit::Table::of(workspace, object)
            })
            .is_some_and(|table| table.no_rows().is_none())
    });
    // The row panel's field that has the keyboard, of the table on screen:
    // its keys are read here with the grid's, so one key is never both's.
    let field = object.filter(|_| grid).and_then(|(tab, id)| {
        let object = app.workspace(tab)?.object_tab(id)?;
        let columns = object.rows.value.as_ref()?.columns.len();
        crate::ui::row_panel::focused_field(ctx, tab, id, columns)
    });
    // The terminal's WHERE line was asked for, and takes the keyboard when
    // it is drawn, later in this frame (it is drawn with the grid, and
    // nowhere else). What is typed until then is no letter of the grid's:
    // an `x` meant for the clause must not set a cell NULL. So with the
    // table's filter bar, which Mod+F opens in every look: it is drawn
    // with the grid too, and takes its flag and the keyboard then, and a
    // character typed before that must not start an edit of the cell.
    let asked = grid
        && app.workspace(active).is_some_and(|workspace| {
            let bar = workspace
                .active_object_tab()
                .is_some_and(|object| object.filter.open && object.filter.focus);
            workspace.opened() && (workspace.focus_where || bar)
        });
    // The terminal's `:` prompt is open, and what it last refused is still
    // said, or that `:diff` found nothing pending, or why a save was not
    // made. The prompt's field is in the status line, which is drawn as
    // long as the workspace is. A save refused under the open prompt
    // (Mod+S) leaves it open: the keys typed into it next close nothing.
    let (prompt, refused) = app
        .workspace(active)
        .filter(|workspace| terminal && workspace.opened())
        .map_or((false, false), |workspace| {
            let prompt = workspace.command.is_some();
            let save = workspace.save_refused && !prompt;
            let said = workspace.command_error.is_some() || workspace.review_refused;
            (prompt, said || save)
        });
    // An editor that just opened takes the keyboard when its field is
    // first drawn, later in this frame: the keys are its own already, and
    // none of them is the grid's (Space, an arrow). So with the prompt.
    let editing = ctx.text_edit_focused() || open || asked || prompt;
    // What the prompt refused is said until the next key, whatever the key
    // goes on to do. A fresh press: the Enter that ran the line may still
    // be held, or the Mod+S that asked for the save.
    let dismiss = refused
        && ctx.input(|input| {
            input.events.iter().any(|event| {
                matches!(
                    event,
                    egui::Event::Key {
                        pressed: true,
                        repeat: false,
                        ..
                    }
                )
            })
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
    // Space and Enter press a focused button, and the arrows move focus
    // from it; with the keyboard on the tree, on a grid or nowhere they act
    // on what those show.
    let focused = crate::ui::focus::on_control(ctx);
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
        // Format is a SQL editor's, and a table's while a document is
        // edited in its large editor. The press is taken on every tab, or
        // Mod+F, below, would take it for its own.
        let format = consume_press(input, Modifiers::COMMAND | Modifiers::SHIFT, Key::F);
        if format && let Some((tab, sql_tab)) = sql {
            actions.push(Action::FormatSql { tab, sql_tab });
        }
        if format
            && open
            && let Some((tab, id)) = object
        {
            actions.push(Action::FormatEditor { tab, id });
        }
        // The other mode of the editor's runs. Not Mod+W, as its design
        // has it: that closes the tab.
        if let Some((tab, sql_tab)) = sql
            && consume_press(input, Modifiers::COMMAND | Modifiers::SHIFT, Key::M)
        {
            actions.push(Action::ToggleSqlMode { tab, sql_tab });
        }
        // A fresh press only: an Esc held to close a dialog over the tab
        // repeats after the dialog is gone.
        if give_up && consume_press(input, Modifiers::NONE, Key::Escape) {
            actions.push(Action::Disconnect(active));
        }
        // A fresh press only: held down, it would add a row a frame. The
        // press is taken, so the line below that reads the same chord for
        // a new connection finds none.
        if let Some((tab, id)) = adds
            && consume_press(input, Modifiers::COMMAND, Key::N)
        {
            let place = crate::edit::Place::Top;
            actions.push(Action::AddRow { tab, id, place });
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
        if tree_arrows && !focused {
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
            key(
                Modifiers::NONE,
                Key::Enter,
                Action::TreeKey {
                    tab: active,
                    key: TreeKey::Enter,
                },
            );
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
            if !editing && any_grid && !tree_arrows && !focused {
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
                if grid || sql_row {
                    key(Modifiers::NONE, Key::Space, Action::ToggleRowPanel(tab));
                }
            }
        }
    });
    // F6 steps through the parts of the window (the bar, the filter, the
    // tree, the tabs, the table's header, the rows, the row panel), and
    // Shift+F6 back. The terminal look steps through its panes alone with
    // ctrl+l and ctrl+h.
    if let Some(workspace) = app.workspace(active) {
        use crate::ui::focus::{self, Region};
        let from = match workspace.pane {
            crate::model::Pane::Tree => Region::Tree,
            crate::model::Pane::Grid => Region::Grid,
        };
        // The first that matches: egui ignores an extra Shift.
        let pressed = |keys: [(Modifiers, Key); 2]| {
            ctx.input_mut(|input| {
                keys.into_iter()
                    .position(|(modifiers, key)| input.consume_key(modifiers, key))
            })
        };
        match pressed([(Modifiers::SHIFT, Key::F6), (Modifiers::NONE, Key::F6)]) {
            Some(index) => focus::step(ctx, index == 1, None, Some(from)),
            None if app.look.terminal && !editing => {
                if let Some(index) = pressed([(Modifiers::CTRL, Key::H), (Modifiers::CTRL, Key::L)])
                {
                    // A row of the table on screen is selected, and no
                    // control has the keyboard: the keys are the grid's.
                    let on_grid = grid && !focused && from == Region::Grid;
                    let row = object.filter(|(tab, id)| {
                        let object = app.workspace(*tab).and_then(|w| w.object_tab(*id));
                        on_grid && object.is_some_and(|object| object.selection.is_some())
                    });
                    match (index, field, object, row) {
                        // From a field of the row panel, back to the grid,
                        // on the cell of that column.
                        (0, Some(col), Some((tab, id)), _) => {
                            let stop = crate::ui::row_panel::field_stop(tab, id, col);
                            ctx.memory_mut(|memory| memory.surrender_focus(stop));
                            actions.push(Action::FieldFocused { tab, id, col });
                            actions.push(Action::GridKeys(tab));
                        }
                        // From the grid, to the row's fields: the panel is
                        // shown for it.
                        (1, None, _, Some((tab, id))) => {
                            actions.push(Action::FocusFields { tab, id });
                        }
                        _ => focus::step(ctx, index == 1, Some(&Region::PANES), Some(from)),
                    }
                }
            }
            None => {}
        }
    }
    // The Ctrl+C that drops the terminal's edit may be held. Its repeats
    // come as copies like the first (a copy says nothing of being a
    // repeat), and with the editor gone they would copy the cell.
    if terminal {
        held_copy(ctx, open);
    }
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
    // The Settings window, wherever the keyboard is.
    if ctx.input_mut(|input| input.consume_key(Modifiers::COMMAND, Key::Comma)) {
        actions.push(Action::ShowSettings);
    }
    // Before `?`, which is typed text too and stays the shortcuts'. The
    // terminal look saves with the same chord, and edits with its letters.
    if app.dialog.is_none() {
        let keyboard = !terminal && !editing && grid && !tree_arrows && !focused;
        let on_field = field.filter(|_| !terminal && !editing);
        editing_keys(app, ctx, keyboard, on_field, &mut actions);
    }
    if !editing && app.dialog.is_none() {
        // A click ends a first key's wait: what is typed after it is typed
        // at what the click chose.
        if ctx.input(|input| input.pointer.any_pressed()) {
            forget_pending(ctx);
        }
        letters(app, ctx, field, &mut actions);
    } else {
        // The keys are a field's: a first key that waited is not the
        // first of what is typed once they are the grid's again.
        forget_pending(ctx);
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
    // Ahead of what the key asked for, which may be the prompt again. Put
    // there only now: a letter that edits acts only where nothing else was
    // asked for before it.
    if dismiss {
        actions.insert(0, Action::CloseCommand(active));
    }
    app.actions.extend(actions);
}

/// Whether a copy that comes this frame is Ctrl+C where Ctrl is the command
/// key, by what is held: Ctrl and nothing else. Not Cmd+C, the copy where
/// Cmd is the command key; nor Ctrl+Shift+C, a Copy key or a menu's Copy,
/// which come with something else held or with nothing.
pub(crate) fn copy_is_ctrl_c(input: &egui::InputState) -> bool {
    let held = input.modifiers;
    held.matches_exact(Modifiers::CTRL) && !held.mac_cmd
}

/// Whether the Ctrl+C that dropped an edit is still down.
fn copy_held_id() -> egui::Id {
    egui::Id::new("copy-held")
}

/// Takes the repeats of a Ctrl+C that an editor (`open`, at this frame's
/// start) took to drop its edit: the copies that come until the key or Ctrl
/// comes up. The next Ctrl+C is the grid's again.
fn held_copy(ctx: &egui::Context, open: bool) {
    let held: bool = ctx
        .data(|data| data.get_temp(copy_held_id()))
        .unwrap_or(false);
    let held = ctx.input_mut(|input| {
        let copy = |event: &egui::Event| matches!(event, egui::Event::Copy);
        let up = |event: &egui::Event| {
            matches!(
                event,
                egui::Event::Key {
                    key: Key::C,
                    pressed: false,
                    ..
                }
            )
        };
        let ctrl_c = copy_is_ctrl_c(input);
        // The editor's own: it drops the edit when its field is drawn.
        let taken = open && ctrl_c && input.events.iter().any(copy);
        if held && !taken {
            input.events.retain(|event| !copy(event));
        }
        (held || taken) && ctrl_c && !input.events.iter().any(up)
    });
    ctx.data_mut(|data| data.insert_temp(copy_held_id(), held));
}

/// Whether `key` with `modifiers` went down this frame. Consumes the
/// repeats of a held key as well, which do not count as a press.
pub(crate) fn consume_press(input: &mut egui::InputState, modifiers: Modifiers, key: Key) -> bool {
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

/// Takes every Enter that went down this frame out of it, whatever is held
/// with it, and says whether one of them was a plain Enter: pressed, not
/// repeated, with nothing held. For a prompt whose buttons Enter must not
/// press: a button that has the keyboard reads any Enter left in the frame
/// as a press of itself, with Ctrl, Cmd, Shift or Alt held as without. An
/// Enter with one of them held is no plain Enter either: it answers nothing.
pub(crate) fn take_enter(input: &mut egui::InputState) -> bool {
    let mut plain = false;
    input.events.retain(|event| match event {
        egui::Event::Key {
            key: Key::Enter,
            modifiers: held,
            pressed: true,
            repeat,
            ..
        } => {
            plain |= !repeat && held.is_none();
            false
        }
        _ => true,
    });
    plain
}

/// Takes what a held `key` repeats out of the frame, whatever is held with
/// it. For a question whose buttons must not be pressed by a key that was
/// down before they came up: a button that has the keyboard reads every
/// repeat of Space as a press of itself.
pub(crate) fn drop_repeats(input: &mut egui::InputState, key: Key) {
    input.events.retain(|event| {
        !matches!(
            event,
            egui::Event::Key {
                key: held,
                pressed: true,
                repeat: true,
                ..
            } if *held == key
        )
    });
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
pub(crate) fn is_press(event: &egui::Event, modifiers: Modifiers, key: Key) -> bool {
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

/// The keys that edit the cells of the table on screen: they open an editor
/// on the selected cell, act on it without one, and save or drop what is
/// pending. `keyboard` says the grid's keys are the grid's: no field or
/// button has them, nor the tree, and the look is not the terminal's, which
/// edits with letters (see `editing_letters`). Mod+S does not wait for
/// that, in any look: it saves from wherever the table's tab shows, and
/// Mod+Shift+D shows and hides Review SQL there. A SQL editor's result
/// takes none of them.
///
/// `field` is the column of the row panel's field that has the keyboard,
/// in those looks: the same keys then act on its cell, and open its editor
/// in the panel. Up and Down step the fields, and Esc gives the keys back
/// to the grid.
fn editing_keys(
    app: &App,
    ctx: &egui::Context,
    keyboard: bool,
    field: Option<usize>,
    actions: &mut Vec<Action>,
) {
    let Some((tab, id)) = app.active_object() else {
        return;
    };
    let Some(object) = app
        .workspace(tab)
        .and_then(|workspace| workspace.object_tab(id))
    else {
        return;
    };
    let save = |input: &mut egui::InputState| take_press(input, Modifiers::COMMAND, Key::S) > 0;
    let open = object.edits.editor.is_some();
    let editor = crate::ui::cell_editor::field_id(tab, id);
    let typing = open && ctx.memory(|memory| memory.has_focus(editor));
    // Mod+S saves wherever the pending bar offers it with that key: there
    // is something to save, whatever has the keyboard (the grid, the tree,
    // a button, the filter's field) and in the Structure view as well. The
    // terminal's Ctrl+S is the same chord.
    if (open || object.edits.pending()) && ctx.input_mut(save) {
        // While the editor's field has the keyboard a save takes what is
        // being typed. Whether this frame changed the text is not known
        // yet: noting it as typed is harmless, since a text left as it was
        // is no change.
        if typing {
            actions.push(Action::EditorTyped { tab, id });
        }
        actions.push(Action::WriteEdits { tab, id });
    }
    // Review SQL by the same rule: wherever the table's tab shows, with
    // something pending or an editor open, in every look. A fresh press
    // only: a held chord would show and hide it by turns.
    let review = |input: &mut egui::InputState| {
        consume_press(input, Modifiers::COMMAND | Modifiers::SHIFT, Key::D)
    };
    if (open || object.edits.pending()) && ctx.input_mut(review) {
        // Shown, the review takes what is being typed, as a save does:
        // noted as typed for the reason the save notes it.
        if typing {
            actions.push(Action::EditorTyped { tab, id });
        }
        let show = !object.edits.reviewing;
        actions.push(Action::ReviewEdits { tab, id, show });
    }
    // Mod+I puts the keyboard on the row's fields in the row panel, which
    // it shows: from wherever the table's rows show, whatever has the
    // keyboard, in every look, and with nothing pending. Not while an
    // editor is open, which is where the user is, and not in a frame that
    // brings a click: the click selects its row once the frame is drawn,
    // and the key would reach the row the selection leaves. A fresh press
    // only. (In a SQL editor the chord asks for completions: no table is
    // in front there.)
    let focus_fields = |input: &mut egui::InputState| {
        let clicked = input
            .events
            .iter()
            .any(|event| matches!(event, egui::Event::PointerButton { .. }));
        consume_press(input, Modifiers::COMMAND, Key::I) && !clicked
    };
    let rows = object.view == crate::model::ObjectView::Data;
    if !open && rows && ctx.input_mut(focus_fields) {
        actions.push(Action::FocusFields { tab, id });
    }
    // An open editor has the keyboard or is about to take it, and takes
    // the keys below with it: none of them opens another, and what is typed
    // is its text.
    if !(keyboard || field.is_some()) || open {
        return;
    }
    // The field the keys were given back from: it gives up the keyboard
    // once the input is read.
    let mut left = None;
    ctx.input_mut(|input| {
        // Each chord with exactly its modifiers: Mod+Alt+Backspace drops
        // every change and Mod+Backspace touches one cell, and Mod+Shift+Z
        // is not Mod+Z.
        if take_press(input, Modifiers::COMMAND | Modifiers::ALT, Key::Backspace) > 0 {
            actions.push(Action::DiscardEdits { tab, id });
        }
        // A click selects its cell once the frame is drawn, after these
        // keys were read: what acts on the active cell, or opens its
        // editor, would land on the cell the selection leaves.
        let clicked = input
            .events
            .iter()
            .any(|event| matches!(event, egui::Event::PointerButton { .. }));
        if clicked {
            return;
        }
        // On a field of the row panel the keys act on its cell, and open
        // its editor in the panel. Its cell is the selected one from the
        // frame after the field took the keyboard: a key that comes before
        // that says so first.
        let on_cell = |actions: &mut Vec<Action>| {
            let selected = object.selection.map(|cell| cell.col);
            if let Some(col) = field.filter(|col| Some(*col) != selected) {
                actions.push(Action::FieldFocused { tab, id, col });
            }
        };
        if take_press(input, Modifiers::COMMAND, Key::Backspace) > 0 {
            on_cell(actions);
            actions.push(Action::SetNull { tab, id });
        }
        if take_press(input, Modifiers::COMMAND, Key::Z) > 0 {
            on_cell(actions);
            actions.push(Action::RevertCell {
                tab,
                id,
                cell: None,
            });
        }
        let Some(selected) = object.selection else {
            return;
        };
        // Delete (the key a Mac labels so too) drops a new row: nothing of
        // it is in the table. On a row of the page neither key does
        // anything yet.
        let on_new = crate::edit::new_id(selected.row).is_some() && field.is_none();
        let deletes = take_press(input, Modifiers::NONE, Key::Delete)
            + take_press(input, Modifiers::NONE, Key::Backspace);
        if on_new && deletes > 0 {
            actions.push(Action::DropRow { tab, id });
            return;
        }
        let cell = crate::model::CellPos {
            row: selected.row,
            col: field.unwrap_or(selected.col),
        };
        let edit = |start| match field {
            Some(_) => Action::EditField {
                tab,
                id,
                cell,
                start,
            },
            None => Action::EditCell {
                tab,
                id,
                cell,
                start,
            },
        };
        // A fresh press only: a held Enter would open, commit and move
        // down the whole column.
        let enter = consume_press(input, Modifiers::NONE, Key::Enter);
        if enter || consume_press(input, Modifiers::NONE, Key::F2) {
            actions.push(edit(crate::model::EditStart::Value));
            return;
        }
        if let Some(col) = field {
            // Up and Down step the fields; Esc gives the keys back to the
            // grid, on the cell of this field's column.
            for (key, by) in [(Key::ArrowUp, -1), (Key::ArrowDown, 1)] {
                if input.consume_key(Modifiers::NONE, key) {
                    let from = col;
                    actions.push(Action::MoveField { tab, id, from, by });
                }
            }
            if consume_press(input, Modifiers::NONE, Key::Escape) {
                left = Some(crate::ui::row_panel::field_stop(tab, id, col));
                on_cell(actions);
                actions.push(Action::GridKeys(tab));
                return;
            }
        }
        // Typing starts the edit with what was typed. The text is taken,
        // or the field that opens would get it again. Space and `?` keep
        // their meaning (the row panel, the shortcuts), and a chord types
        // nothing.
        let chord = input.modifiers.command || input.modifiers.ctrl || input.modifiers.mac_cmd;
        if chord {
            return;
        }
        let starts =
            |text: &str| !matches!(text, "" | " " | "?") && !text.chars().any(char::is_control);
        let mut typed = String::new();
        input.events.retain(|event| match event {
            egui::Event::Text(text) if starts(text) => {
                typed.push_str(text);
                false
            }
            _ => true,
        });
        if !typed.is_empty() {
            actions.push(edit(crate::model::EditStart::Typed(typed)));
        }
    });
    if let Some(stop) = left {
        ctx.memory_mut(|memory| memory.surrender_focus(stop));
    }
}

/// The terminal look's normal mode on the grid of the table `id`: `i` and
/// Enter edit the selected cell from its value, `cc` from nothing, `x` sets
/// the cell NULL, `u` puts back what was loaded, `o` and `O` open a new row
/// below the cursor's and above it, and `:` opens the prompt
/// that writes, discards and shows the SQL. The letters are read as the
/// text they type, in the order they came, and taken: a letter that opens
/// an editor or the prompt is no part of its text, and what follows it in
/// its frame does nothing (the field is not there yet to be typed into).
///
/// They act on the selected cell, so they act only in a frame that brings
/// nothing else: beside a key that moves the selection, or anything that
/// was asked for already (`actions`), the order the two came in is lost,
/// and the letter might reach another cell than the one it was typed on.
/// It is dropped there. So beside a pointer's button, going down or coming
/// up: a click selects its cell when the grid is drawn, after these keys
/// were read, and the letter would act on the cell the selection leaves.
///
/// `waiting` says a first `c` came in an earlier frame; returns whether one
/// waits now.
fn editing_letters(
    ctx: &egui::Context,
    (tab, id): (ConnTabId, TabId),
    (selection, field): (Option<crate::model::CellPos>, Option<usize>),
    waiting: bool,
    actions: &mut Vec<Action>,
) -> bool {
    use crate::model::{CellPos, EditStart};
    // On a field of the row panel the letters act on its cell, and edit
    // it in the panel.
    let edit = |start: EditStart| {
        let selected = selection?;
        Some(match field {
            Some(col) => Action::EditField {
                tab,
                id,
                cell: CellPos {
                    row: selected.row,
                    col,
                },
                start,
            },
            None => Action::EditCell {
                tab,
                id,
                cell: selected,
                start,
            },
        })
    };
    // The field's cell is the selected one from the frame after the field
    // took the keyboard: a letter that comes before that says so first.
    let on_cell = |actions: &mut Vec<Action>| {
        let selected = selection.map(|cell| cell.col);
        if let Some(col) = field.filter(|col| Some(*col) != selected) {
            actions.push(Action::FieldFocused { tab, id, col });
        }
    };
    let alone = actions.is_empty();
    let mine = |text: &str| matches!(text, "i" | "c" | "x" | "u" | ":" | "o" | "O");
    let enter = |event: &egui::Event| is_press(event, Modifiers::NONE, Key::Enter);
    ctx.input_mut(|input| {
        // A chord types nothing, though some systems send its letter as
        // text too: Ctrl+X is not `x`.
        let held = input.modifiers;
        if held.command || held.ctrl || held.mac_cmd || held.alt {
            return false;
        }
        // Every key of the frame is one of these letters, or Enter.
        let (mut keys, mut letters, mut other) = (0, 0, false);
        for event in &input.events {
            match event {
                event if enter(event) => {}
                egui::Event::Key { pressed: true, .. } => keys += 1,
                egui::Event::Text(text) if mine(text) => letters += 1,
                egui::Event::Text(_) | egui::Event::PointerButton { .. } => other = true,
                _ => {}
            }
        }
        let alone = alone && !other && keys <= letters;
        // A first `c` waits for its second, and for nothing else.
        let mut first = waiting;
        let mut waits = false;
        // Whether a field was asked for: the frame's typing ends there.
        let mut opened = false;
        input.events.retain(|event| {
            if enter(event) {
                // A fresh press only: a held Enter would open, commit and
                // move down the whole column.
                let fresh = !matches!(event, egui::Event::Key { repeat: true, .. });
                if fresh && alone && !opened {
                    opened = true;
                    actions.extend(edit(EditStart::Value));
                }
                return false;
            }
            let egui::Event::Text(text) = event else {
                return true;
            };
            if !mine(text) {
                return true;
            }
            let second = std::mem::take(&mut first);
            waits = false;
            if !alone || opened {
                return false;
            }
            match text.as_str() {
                "i" => {
                    opened = true;
                    actions.extend(edit(EditStart::Value));
                }
                "c" if second => {
                    opened = true;
                    actions.extend(edit(EditStart::Replace(String::new())));
                }
                "c" => {
                    first = true;
                    waits = true;
                }
                "x" => {
                    on_cell(actions);
                    actions.push(Action::SetNull { tab, id });
                }
                "u" => {
                    on_cell(actions);
                    actions.push(Action::RevertCell {
                        tab,
                        id,
                        cell: None,
                    });
                }
                ":" => {
                    opened = true;
                    actions.push(Action::OpenCommand(tab));
                }
                // A row below the cursor's, or above it with the capital.
                // With no cursor, at the top.
                "o" | "O" => {
                    opened = true;
                    let place = match (selection, text.as_str()) {
                        (Some(cell), "o") => crate::edit::Place::Below(cell.row),
                        (Some(cell), _) => crate::edit::Place::Above(cell.row),
                        (None, _) => crate::edit::Place::Top,
                    };
                    actions.push(Action::AddRow { tab, id, place });
                }
                _ => {}
            }
            false
        });
        waits
    })
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

/// Ends the wait of a first key (`c` of `cc`, `g` of `gd`): the key after
/// it is a first key again. It waits through frames that bring no key, and
/// no longer than the keys stay the grid's: not past a click, a field that
/// has the keyboard, the `:` prompt or a dialog.
pub(crate) fn forget_pending(ctx: &egui::Context) {
    ctx.data_mut(|data| data.insert_temp(pending_id(), None::<char>));
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
/// workspace's vim keys. Only when no text field has the keyboard. `field`
/// is the column of the row panel's field that has it: `j` and `k` step
/// the fields then, and the letters that edit a cell edit that field.
fn letters(app: &mut App, ctx: &egui::Context, field: Option<usize>, actions: &mut Vec<Action>) {
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
    // An open menu keeps its Esc: this runs before the menu is drawn, and
    // the key closes the menu before it closes anything under it.
    let menu_open = egui::Popup::is_any_open(ctx);
    let mut next_pending = None;
    let pressed = |key: Key| ctx.input_mut(|input| input.consume_key(Modifiers::NONE, key));
    // A press of its own: not the repeats of a key held since it did
    // something else (the Esc that left insert mode).
    let fresh = |key: Key| ctx.input_mut(|input| consume_press(input, Modifiers::NONE, key));
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
            if !crate::ui::focus::on_control(ctx) {
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
    // The table on screen while its Review SQL is open, in either of its
    // views: the panel stands under both.
    let reviewing = workspace
        .active_object_tab()
        .filter(|object| object.edits.reviewing)
        .map(|object| object.id);
    // Esc closes that panel, as its foot says. Before the row panel's Esc,
    // and in its place: the key does no more. Nor does an Esc that left a
    // text field, or the repeats of one held since.
    if let Some(id) = reviewing
        && !left_field
        && !menu_open
        && fresh(Key::Escape)
    {
        let show = false;
        actions.push(Action::ReviewEdits { tab, id, show });
    }
    // What the last save came to, with nothing pending any more: Esc takes
    // it off the status line, as Dismiss does in the other looks. Before
    // the row panel's Esc, and in its place: the key does no more.
    let note = workspace
        .active_object_tab()
        .filter(|object| object.edits.note.is_some() && !object.edits.holds())
        .map(|object| object.id);
    if let Some(id) = note
        && !left_field
        && !menu_open
        && fresh(Key::Escape)
    {
        actions.push(Action::DismissNote { tab, id });
    }
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
    // On a field of the row panel `j` and `k` step the fields.
    if let (Some(from), Some(id)) = (field, active) {
        for (key, by) in [(Key::J, 1), (Key::K, -1)] {
            if pressed(key) {
                actions.push(Action::MoveField { tab, id, from, by });
            }
        }
    }
    // What `j/k/h/l` move in: the object's grid, or a SQL result.
    if !tree
        && field.is_none()
        && let Some(id) = active.or(sql_grid)
    {
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
    // The letters that edit a table's cells, where its grid has the keys
    // or a field of its row panel has: not with the arrows on the tree, in
    // the Structure view, or on a button, whose key Enter is.
    let table = workspace
        .active_object_tab()
        .filter(|object| !tree && object.view == crate::model::ObjectView::Data)
        .map(|object| (object.id, object.selection));
    if let Some((id, selection)) = table
        && (field.is_some() || !crate::ui::focus::on_control(ctx))
    {
        let waiting = pending == Some('c');
        if editing_letters(ctx, (tab, id), (selection, field), waiting, actions) {
            next_pending = Some('c');
        }
    }
    // The prompt opens wherever a table's tab shows, as the status line
    // offers it there (`:w`) and as Mod+S saves there: with the arrows on
    // the tree, in the Structure view, on a button. On the grid the
    // letters above took the colon.
    if active.is_some() && typed(ctx, ":") {
        actions.push(Action::OpenCommand(tab));
    }
    // The row panel's keys: an object tab's, or a SQL result's while a row
    // of it is selected (with none its panel has nothing to show).
    if let Some(id) = active.or(sql_row) {
        // Enter and `i` open and close the panel of a SQL result's row. On
        // a table they edit the cell (above): Space is its panel's key.
        if active.is_none() {
            // Enter belongs to a focused button.
            let focused = crate::ui::focus::on_control(ctx);
            if !tree && !panel && !focused && pressed(Key::Enter) {
                actions.push(Action::ToggleRowPanel(tab));
            }
            if pressed(Key::I) {
                actions.push(Action::ToggleRowPanel(tab));
            }
        }
        // An Esc that left a text field did only that, and so do its
        // repeats while it is held.
        if panel && !left_field && !menu_open && fresh(Key::Escape) {
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
    // The card of a refused write, while a SQL editor's Messages show it:
    // the letter its button names, which it does in this look alone (the
    // others returned above, and `card_key` answers none of them). A
    // press of its own: once writes are
    // allowed the same letter runs the statements again, and the repeats
    // of a key held since must not answer an offer nobody has read.
    if let Some((key, action)) = crate::ui::sql_results::card_key(app, tab)
        && fresh(key)
    {
        actions.push(action);
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
    // `Y` with the review open is its SQL: the cell's `y` is the same key.
    // The typed text is read first, so the one press does one thing.
    let sql = reviewing.filter(|_| typed(ctx, "Y"));
    if pressed(Key::Y) || sql.is_some() {
        // The whole statements, never the lines as the panel shows them.
        let text = match sql {
            Some(id) => crate::ui::review::copy_text(app, tab, id),
            None => app.copy_text(false),
        };
        if let Some(text) = text {
            ctx.copy_text(text);
            // The cell's `y` shows in the grid; the review's `Y` shows
            // nowhere, so the status line says it.
            if sql.is_some() {
                let said = crate::i18n::gettext(app.locale, crate::ui::review::COPIED_SQL);
                crate::ui::toast::say(ctx, &said);
            }
        }
    }
    if pressed(Key::G) {
        next_pending = Some('g');
    }
    // Whether the grid's rows have the keys: where `dd` is read.
    let on_rows = !tree
        && field.is_none()
        && app
            .workspace(tab)
            .and_then(|workspace| workspace.object_tab(object_tab))
            .is_some_and(|object| object.view == crate::model::ObjectView::Data);
    if pressed(Key::D) {
        if pending == Some('g') {
            actions.push(Action::FollowSelectedKey { tab, object_tab });
        } else {
            // The second `d` of `dd` drops the row under the cursor where
            // it is a new one. Each `d` is the Data view's key still,
            // which changes nothing where the rows show already. Only
            // there does a first `d` wait for a second: one that brought
            // the rows up from the Structure view was that key and no more,
            // and a dropped row is not brought back.
            if on_rows && pending == Some('d') {
                actions.push(Action::DropRow {
                    tab,
                    id: object_tab,
                });
            } else if on_rows {
                next_pending = Some('d');
            }
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
                .any(|(keys, what, _)| keys.contains("Mod+S") && what.contains("connection dialog"))
        );
    }

    #[test]
    fn the_shortcut_table_names_the_keys_that_review_sql() {
        for look in crate::theme::Look::ALL {
            let listed = shortcuts(&look)
                .any(|row| row == ("Mod+Shift+D", "Show or hide the SQL of the pending changes"));
            assert!(listed, "{}", look.name);
            // Omarchy's prompt, its Esc and its `Y`: no other look's.
            for row in [
                (":diff", "Show the SQL of the pending changes"),
                ("Esc", "Close the SQL of the pending changes"),
                ("Y", "Copy the SQL of the pending changes"),
            ] {
                let listed = shortcuts(&look).any(|listed| listed == row);
                assert_eq!(listed, look.terminal, "{}: {row:?}", look.name);
            }
        }
    }

    #[test]
    fn the_shortcut_table_lists_the_settings_key() {
        assert!(
            SHORTCUTS
                .iter()
                .any(|(keys, what, _)| (*keys, *what) == ("Mod+,", "Settings"))
        );
    }

    #[test]
    fn the_shortcut_table_covers_the_spec_map() {
        let descriptions: Vec<&str> = SHORTCUTS.iter().map(|(_, what, _)| *what).collect();
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
            "Edit the cell",
            "Commit and move right or left",
            "Cancel the edit",
            "Set NULL",
            "Revert the cell",
            "Save all pending changes",
            "Discard all pending changes",
            "Shortcuts",
        ] {
            assert!(descriptions.contains(&expected), "{expected}");
        }
    }

    #[test]
    fn the_shortcut_table_names_the_keys_that_edit_a_cell() {
        use crate::theme::Look;
        // The keys of `what` as `look` has them.
        let keys = |look: Look, what: &str| {
            let rows: Vec<&str> = shortcuts(&look)
                .filter(|(_, description)| *description == what)
                .map(|(keys, _)| keys)
                .collect();
            match rows.as_slice() {
                [keys] => Some(*keys),
                [] => None,
                several => panic!("{}: {what} twice: {several:?}", look.name),
            }
        };
        // macOS and Windows edit with chords.
        for look in [Look::standard(), Look::macos()] {
            let keys = |what| keys(look, what);
            assert_eq!(keys("Edit the cell"), Some("Enter, F2"));
            assert_eq!(
                keys("Commit and move right or left"),
                Some("Tab, Shift+Tab")
            );
            assert_eq!(keys("Cancel the edit"), Some("Esc"));
            assert_eq!(keys("Set NULL"), Some("Mod+Backspace"));
            assert_eq!(keys("Revert the cell"), Some("Mod+Z"));
            assert_eq!(keys("Save all pending changes"), Some("Mod+S"));
            assert_eq!(
                keys("Discard all pending changes"),
                Some("Mod+Alt+Backspace")
            );
            // Omarchy's own are not theirs.
            for what in [
                "Edit the cell from nothing",
                "Leave the editor and keep the edit",
                "Drop the edit",
            ] {
                assert_eq!(keys(what), None, "{}: {what}", look.name);
            }
        }
        // Omarchy edits with letters and its `:` prompt, and its Esc keeps
        // the edit where theirs drops it.
        let keys = |what| keys(Look::omarchy(), what);
        assert_eq!(keys("Edit the cell"), Some("i, Enter"));
        assert_eq!(keys("Edit the cell from nothing"), Some("cc"));
        assert_eq!(
            keys("Commit and move right or left"),
            Some("Tab, Shift+Tab")
        );
        assert_eq!(keys("Leave the editor and keep the edit"), Some("Esc"));
        assert_eq!(keys("Drop the edit"), Some("Ctrl+C"));
        assert_eq!(keys("Cancel the edit"), None);
        assert_eq!(keys("Set NULL"), Some("x"));
        assert_eq!(keys("Revert the cell"), Some("u"));
        assert_eq!(keys("Save all pending changes"), Some(":w, Mod+S"));
        assert_eq!(keys("Discard all pending changes"), Some(":e!"));
    }

    #[test]
    fn every_look_lists_what_holds_in_all_of_them() {
        use crate::theme::Look;
        for look in Look::ALL {
            let listed: Vec<_> = shortcuts(&look).collect();
            for (keys, what, holds) in SHORTCUTS {
                let own = match holds {
                    Holds::All => true,
                    Holds::Desktop => !look.terminal,
                    Holds::Terminal => look.terminal,
                };
                assert_eq!(
                    listed.contains(&(*keys, *what)),
                    own,
                    "{}: {keys} {what}",
                    look.name
                );
            }
            // In the table's order.
            let places = listed.iter().map(|(keys, what)| {
                let mut rows = SHORTCUTS.iter();
                rows.position(|(row, said, _)| row == keys && said == what)
            });
            assert!(places.is_sorted(), "{}", look.name);
        }
    }

    #[test]
    fn the_shortcut_table_names_the_keys_that_add_a_row() {
        for look in crate::theme::Look::ALL {
            let keys = |what: &str| {
                let mut rows = shortcuts(&look).filter(|(_, listed)| *listed == what);
                let found = rows.next().map(|(keys, _)| keys);
                assert!(rows.next().is_none(), "{}: {what} twice", look.name);
                found
            };
            if look.terminal {
                assert_eq!(keys("Add a row below or above the cursor"), Some("o, O"));
                assert_eq!(keys("Drop a new row"), Some("dd"));
                assert_eq!(keys("Add a row"), None);
            } else {
                assert_eq!(keys("Add a row"), Some("Mod+N"));
                assert_eq!(keys("Drop a new row"), Some("Delete"));
            }
            // A new connection is still Mod+N, where no table is in front.
            assert_eq!(keys("New connection"), Some("Mod+N"));
        }
    }

    #[test]
    fn the_shortcut_table_names_omarchys_keys_that_edit() {
        let (keys, _, _) = SHORTCUTS
            .iter()
            .find(|(_, what, _)| what.starts_with("Omarchy:"))
            .expect("Omarchy's row");
        let keys: Vec<&str> = keys.split(", ").collect();
        for key in [
            "i", "Enter", "cc", "x", "u", "Mod+S", ":w", ":e!", ":diff", "Y", "Space",
        ] {
            assert!(keys.contains(&key), "{key}: {keys:?}");
        }
        // Saving is Mod+S, as the row of the terminal's own keys has it:
        // Cmd where Cmd is the command key.
        assert!(!keys.contains(&"Ctrl+S"), "{keys:?}");
        // `s` is the Structure view's still.
        assert!(keys.contains(&"s"));
    }

    #[test]
    fn the_shortcut_table_names_the_keys_that_open_tabs() {
        let keys = |what: &str| {
            SHORTCUTS
                .iter()
                .find(|(_, description, _)| *description == what)
                .map(|(keys, _, _)| *keys)
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
    fn every_enter_is_taken_and_only_a_plain_one_counts() {
        let enter = |held| crate::testing::key(Key::Enter, held);
        let taken = |events: Vec<egui::Event>| {
            let mut input = egui::InputState::default();
            input.events = events;
            (take_enter(&mut input), input.events)
        };
        assert_eq!(taken(vec![enter(Modifiers::NONE)]), (true, Vec::new()));
        // With anything held it is taken all the same, and is no Enter.
        let cmd = Modifiers::MAC_CMD | Modifiers::COMMAND;
        for held in [
            Modifiers::CTRL,
            Modifiers::CTRL | Modifiers::COMMAND,
            Modifiers::COMMAND,
            cmd,
            Modifiers::MAC_CMD,
            Modifiers::SHIFT,
            Modifiers::ALT,
            cmd | Modifiers::SHIFT,
        ] {
            assert_eq!(taken(vec![enter(held)]), (false, Vec::new()), "{held:?}");
        }
        // So is what a held Enter repeats.
        let repeated = egui::Event::Key {
            key: Key::Enter,
            physical_key: None,
            pressed: true,
            repeat: true,
            modifiers: Modifiers::NONE,
        };
        assert_eq!(taken(vec![repeated]), (false, Vec::new()));
        // A plain one counts beside the others. Another key stays, and so
        // does Enter coming up.
        let space = crate::testing::key(Key::Space, Modifiers::CTRL);
        let up = crate::testing::release(Key::Enter, Modifiers::NONE);
        let events = vec![
            enter(Modifiers::CTRL),
            space.clone(),
            enter(Modifiers::NONE),
            up.clone(),
        ];
        assert_eq!(taken(events), (true, vec![space, up]));
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

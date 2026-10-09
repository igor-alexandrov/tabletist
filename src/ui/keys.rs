//! The keys' handler. Which key does what is the keymap's to say
//! (`crate::keymap`): this reads what the window sends, asks the keymap
//! whether it is a command's key where the keyboard is, and pushes what
//! the command does. It names no key of its own.

use egui::{Key, Modifiers};

use crate::app::App;
use crate::keymap::{Command, Keymap, Layout, Mods, Scope, Spelled, Stroke, Typed, Written};
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

/// Where a frame's drawing finds the keymap in force.
fn keymap_id() -> egui::Id {
    egui::Id::new("keymap")
}

/// Leaves the keymap in force where what is drawn this frame reads a
/// key's spelling from: a label names a key by its command, wherever it is
/// drawn, and none is written by hand.
pub fn publish(ctx: &egui::Context, keymap: &std::sync::Arc<Keymap>) {
    ctx.data_mut(|data| data.insert_temp(keymap_id(), keymap.clone()));
}

/// The keymap in force, or the defaults where none was published (a
/// widget drawn on its own, in a test). For what reads its own keys: a
/// dialog, which the workspace's handler does not run under. Asked for
/// before the input is borrowed: both sit behind the context's lock.
pub(crate) fn published(ctx: &egui::Context) -> std::sync::Arc<Keymap> {
    ctx.data(|data| data.get_temp(keymap_id()))
        .unwrap_or_default()
}

/// The command a letter typed in a prompt stands for there: `when` says
/// which prompt, whose letters are its own and nobody else's.
pub(crate) fn prompt_letter(
    keymap: &Keymap,
    layout: Layout,
    when: crate::keymap::When,
    typed: &str,
) -> Option<Command> {
    match keymap.typed(layout, Scope::Prompt, &|holds| holds == when, typed) {
        Typed::Command(command) => Some(command),
        Typed::Claimed | Typed::Waiting | Typed::Nothing => None,
    }
}

/// The first key of `command`, as `look` writes it.
pub fn written(ctx: &egui::Context, look: &crate::theme::Look, command: Command) -> Written {
    published(ctx).key(Layout::of(look), command)
}

/// Every key of `command`, as `look` writes them.
pub fn written_all(ctx: &egui::Context, look: &crate::theme::Look, command: Command) -> Written {
    published(ctx).label(Layout::of(look), command)
}

/// The first key `command` has in `scope`, as `look` writes it.
pub fn written_in(
    ctx: &egui::Context,
    look: &crate::theme::Look,
    command: Command,
    scope: Scope,
) -> Written {
    published(ctx).key_in(Layout::of(look), command, scope)
}

/// The first keys of `commands` as one hint, as `look` writes them.
pub fn written_together(
    ctx: &egui::Context,
    look: &crate::theme::Look,
    commands: &[Command],
) -> Written {
    published(ctx).together(Layout::of(look), commands)
}

/// Whether `held` is exactly `mods`, as `layout` reads what is held. Shift
/// and Alt are exact everywhere (`consume_key` lets an extra Shift
/// through, so `ctrl+shift+c` would be read as `ctrl+c`).
pub(crate) fn held_is(layout: Layout, held: Modifiers, mods: Mods) -> bool {
    if held.shift != mods.shift || held.alt != mods.alt {
        return false;
    }
    match layout {
        // One command key, Ctrl. Linux reports it as `ctrl` and `command`
        // both; either names it. Cmd, where a Mac draws this look, is no
        // key of its layout: Cmd+C stays the copy of whatever has the
        // keyboard.
        Layout::Omarchy => !mods.cmd && !held.mac_cmd && (held.ctrl || held.command) == mods.ctrl,
        Layout::Mac | Layout::Windows => {
            let pattern = Modifiers {
                alt: mods.alt,
                ctrl: mods.ctrl,
                shift: mods.shift,
                mac_cmd: false,
                command: mods.cmd,
            };
            // As `is_press`: Cmd is held only where the chord names it.
            held.matches_exact(pattern) && (mods.cmd || !held.mac_cmd)
        }
    }
}

/// How a chord went down in a frame.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Presses {
    /// A press of its own: not only what a held key repeats.
    pub fresh: bool,
    /// Every time it went down, repeats too.
    pub count: usize,
}

/// With Shift held a US keyboard reports the brace for a bracket.
fn same_key(named: Key, pressed: Key) -> bool {
    named == pressed
        || matches!(
            (named, pressed),
            (Key::OpenBracket, Key::OpenCurlyBracket) | (Key::CloseBracket, Key::CloseCurlyBracket)
        )
}

/// Whether `event` is what the window sends in place of `key` held with
/// the command key: it turns C, X and V into a copy, a cut and a paste
/// before any key event is made, whatever else is held.
fn clipboard_event(event: &egui::Event, key: Key) -> bool {
    matches!(
        (event, key),
        (egui::Event::Copy, Key::C) | (egui::Event::Cut, Key::X) | (egui::Event::Paste(_), Key::V)
    )
}

/// Whether `event` is a press of the chord spelled `keys`, and whether a
/// fresh one (a held key's repeat is a press too). `held` is what the frame
/// holds: a clipboard event says nothing of it itself, and one with
/// nothing held is a Copy key or a menu's, and no chord. A spelling that
/// is no single chord (letters, a line of the prompt, a range of digits)
/// is never pressed.
pub(crate) fn press_of(
    layout: Layout,
    held: Modifiers,
    event: &egui::Event,
    keys: &'static str,
) -> Option<bool> {
    let Ok(Spelled::Strokes(strokes)) = crate::keymap::spell(keys) else {
        return None;
    };
    let [Stroke::Key { mods, key }] = strokes.as_slice() else {
        return None;
    };
    match event {
        egui::Event::Key {
            key: down,
            modifiers,
            pressed: true,
            repeat,
            ..
        } if same_key(*key, *down) && held_is(layout, *modifiers, *mods) => Some(!repeat),
        // A held chord's repeats come as more of the same event, and none
        // says it is a repeat: `held_copy` tells them apart where it
        // matters.
        event
            if (mods.cmd || mods.ctrl)
                && held_is(layout, held, *mods)
                && clipboard_event(event, *key) =>
        {
            Some(true)
        }
        _ => None,
    }
}

/// Takes the presses of the chord spelled `keys` out of the frame and
/// counts them.
pub(crate) fn presses(input: &mut egui::InputState, layout: Layout, keys: &'static str) -> Presses {
    let held = input.modifiers;
    let mut pressed = Presses::default();
    input
        .events
        .retain(|event| match press_of(layout, held, event, keys) {
            Some(fresh) => {
                pressed.fresh |= fresh;
                pressed.count += 1;
                false
            }
            None => true,
        });
    pressed
}

/// How `command` was asked for in `scope` this frame, by any of the chords
/// it has there in this mode (`normal`: no text field has the keyboard).
pub(crate) fn asked(
    input: &mut egui::InputState,
    keymap: &Keymap,
    layout: Layout,
    command: Command,
    scope: Scope,
    normal: bool,
) -> Presses {
    let mut all = Presses::default();
    for keys in keymap.chords_now(layout, command, scope, normal) {
        let pressed = presses(input, layout, keys);
        all.fresh |= pressed.fresh;
        all.count += pressed.count;
    }
    all
}

/// The digit a command bound to a range (`alt+1..9`) was asked with, from
/// nought. With Shift held the key reports what the digit types (`!`), so
/// the key it is on counts too.
pub(crate) fn digit_asked(
    input: &mut egui::InputState,
    keymap: &Keymap,
    layout: Layout,
    command: Command,
    scope: Scope,
    normal: bool,
) -> Option<usize> {
    let mut found = None;
    for keys in keymap.chords_now(layout, command, scope, normal) {
        let Ok(Spelled::Digits(mods)) = crate::keymap::spell(keys) else {
            continue;
        };
        for (index, number) in NUMBERS.into_iter().enumerate() {
            let before = input.events.len();
            input.events.retain(|event| {
                !matches!(event, egui::Event::Key { key, physical_key, modifiers, pressed: true, .. }
                    if (*key == number || *physical_key == Some(number))
                        && held_is(layout, *modifiers, mods))
            });
            if input.events.len() != before {
                found = Some(index);
            }
        }
    }
    found
}

/// The characters this frame typed with nothing held but Shift: the text
/// events where there are any, else the keys that went down, as the
/// characters they type on a US keyboard (a test's bare key press sends
/// no text; the real keyboard sends both).
fn typed_chars(ctx: &egui::Context) -> Vec<char> {
    ctx.input(|input| {
        let held = input.modifiers;
        if held.command || held.ctrl || held.mac_cmd || held.alt {
            return Vec::new();
        }
        let text: Vec<char> = input
            .events
            .iter()
            .filter_map(|event| match event {
                egui::Event::Text(text) => Some(text.chars()),
                _ => None,
            })
            .flatten()
            .collect();
        if !text.is_empty() {
            return text;
        }
        input
            .events
            .iter()
            .filter_map(|event| match event {
                egui::Event::Key {
                    key,
                    pressed: true,
                    modifiers,
                    ..
                } if !(modifiers.command
                    || modifiers.ctrl
                    || modifiers.mac_cmd
                    || modifiers.alt) =>
                {
                    crate::keymap::char_of(*key, modifiers.shift)
                }
                _ => None,
            })
            .collect()
    })
}

/// Takes a typed character out of the frame, and the key that typed it:
/// what it asked for may open a field, which must not get it again.
fn take_char(ctx: &egui::Context, char: char) {
    ctx.input_mut(|input| {
        input.events.retain(|event| match event {
            egui::Event::Text(text) => !text.chars().eq([char]),
            egui::Event::Key {
                key,
                pressed: true,
                modifiers,
                ..
            } => {
                // A test's key says nothing of Shift beside the text it
                // sends: the letter's key goes with either case.
                let typed = crate::keymap::char_of(*key, modifiers.shift);
                !typed.is_some_and(|typed| typed.eq_ignore_ascii_case(&char))
            }
            _ => true,
        });
    });
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
    let field_asked = grid
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
    let editing = ctx.text_edit_focused() || open || field_asked || prompt;
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
    // Space flips a boolean cell where the grid has the keys and the cell
    // can be edited. Everywhere else it is no key.
    let flips = object
        .filter(|_| grid && !editing && !tree_arrows && !focused)
        .filter(|&(tab, id)| app.flips(tab, id));
    let layout = app.layout();
    let on_picker = matches!(
        app.active_tab().content,
        crate::model::ConnTabContent::Picker(_)
    );
    // The tab in front, whatever has the keyboard in it: a cancel, a run
    // and the `:` prompt are its own from the tree too.
    let front = if on_picker {
        Scope::Connections
    } else if sql.is_some() {
        Scope::SqlEditor
    } else {
        Scope::Grid
    };
    // What has the keyboard, for the letters: the nearest scope first.
    // `pane` says where the user last worked, not `tree_arrows`, which
    // also sends the arrows to the tree where there is no grid to move in.
    let on_tree = app
        .workspace(active)
        .is_some_and(|workspace| workspace.pane == crate::model::Pane::Tree);
    let scopes: &[Scope] = if on_picker {
        &[Scope::Connections]
    } else if open {
        &[Scope::CellEditor]
    } else if field.is_some() {
        &[Scope::Inspector]
    } else if on_tree {
        &[Scope::Sidebar]
    } else if sql.is_some() && sql_grid {
        &[Scope::Results, Scope::SqlEditor]
    } else if sql.is_some() {
        &[Scope::SqlEditor]
    } else {
        &[Scope::Grid]
    };
    // Normal mode: no text field has the keyboard. A chord the keymap
    // gives to normal mode is the field's own while one has it.
    let normal = !editing;
    // Something of the tab in front is loading or running: its cancel has
    // something to cancel.
    let busy = app.is_busy(active);
    let mut actions = Vec::new();
    // The place of the tab asked for with its digit.
    let mut tab_asked = None;
    ctx.input_mut(|input| {
        // The keymap says which key: each line asks whether its command's
        // chord went down, in the scope it is read in. The guards say when
        // the command makes sense, as they did.
        let keymap = &app.keymap;
        let on = |input: &mut egui::InputState, command: Command, scope: Scope| {
            asked(input, keymap, layout, command, scope, normal)
        };
        // The completion list's keys come first: the editor never sees
        // them, nor do the chords below.
        if let Some(editor) = in_editor {
            completion_keys(input, editor, completing, (keymap, layout), &mut actions);
        }
        if let Some((tab, sql_tab)) = sql {
            // Running works while typing: the editor never sees these. A
            // held chord runs once, or each repeat would cancel the run
            // before it.
            for (command, all) in [(Command::RunAll, true), (Command::RunStatement, false)] {
                if on(input, command, Scope::SqlEditor).fresh {
                    actions.push(Action::RunSql { tab, sql_tab, all });
                }
            }
            // The chord that formats the script, in the looks that have
            // one: Omarchy's key is a letter, read with the letters.
            if on(input, Command::Format, Scope::SqlEditor).fresh {
                actions.push(Action::FormatSql { tab, sql_tab });
            }
            // The other mode of the editor's runs, in the looks that have
            // a chord for it: Omarchy sets the mode from its prompt.
            if on(input, Command::ToggleSqlMode, Scope::SqlEditor).fresh {
                actions.push(Action::ToggleSqlMode { tab, sql_tab });
            }
        }
        // A document being edited in a table's large editor is formatted
        // by the editor's own chord.
        if open
            && let Some((tab, id)) = object
            && on(input, Command::Format, Scope::CellEditor).fresh
        {
            actions.push(Action::FormatEditor { tab, id });
        }
        // A fresh press only: an Esc held to close a dialog over the tab
        // repeats after the dialog is gone.
        if give_up && on(input, Command::CancelConnecting, Scope::Global).fresh {
            actions.push(Action::Disconnect(active));
        }
        // A fresh press only: held down, it would add a row a frame.
        if let Some((tab, id)) = adds
            && on(input, Command::AddRow, Scope::Grid).fresh
        {
            let place = crate::edit::Place::Top;
            actions.push(Action::AddRow { tab, id, place });
        }
        if let Some((tab, id)) = flips
            && on(input, Command::BooleanCycle, Scope::Grid).count > 0
        {
            actions.push(Action::CycleBoolean { tab, id });
        }
        if on(input, Command::CloseConnection, Scope::Global).count > 0 {
            actions.push(Action::CloseConnTab(active));
        }
        // A SQL editor has nothing to reload or filter, and a row panel
        // only for a selected row of its result.
        let on_sql = sql.is_some();
        let rows = if on_sql { Scope::Results } else { Scope::Grid };
        if (!on_sql || sql_row) && on(input, Command::ToggleInspector, rows).count > 0 {
            actions.push(Action::ToggleRowPanel(active));
        }
        if on(input, Command::PreviousConnection, Scope::Global).count > 0 {
            actions.push(Action::CycleConnTab(-1));
        }
        if on(input, Command::NextConnection, Scope::Global).count > 0 {
            actions.push(Action::CycleConnTab(1));
        }
        // A SQL editor belongs to a workspace: the picker has none to open.
        if in_workspace && on(input, Command::NewSqlTab, Scope::Global).count > 0 {
            actions.push(Action::NewSqlTab(active));
        }
        if on(input, Command::Connections, Scope::Global).count > 0 {
            actions.push(Action::ShowConnections);
        }
        // A new connection is the picker's key. On a table's rows the
        // same chord added a row, above.
        if on_picker && on(input, Command::NewConnection, Scope::Connections).count > 0 {
            actions.push(Action::NewConnection);
        }
        let window = Command::GoToConnectionWindow;
        if let Some(index) = digit_asked(input, keymap, layout, window, Scope::Global, normal) {
            actions.push(Action::ActivateConnection(index));
        }
        let switch = Command::SwitchTab;
        if in_workspace {
            tab_asked = digit_asked(input, keymap, layout, switch, Scope::Global, normal);
        }
        if !on_sql && on(input, Command::Reload, Scope::Grid).count > 0 {
            // The tree reloads itself when it has the arrows (spec 5.10).
            actions.push(if tree_arrows {
                Action::RefreshTree(active)
            } else {
                Action::Refresh(active)
            });
        }
        // With nothing loading or running, Omarchy's chord is the text
        // field's copy, and the others' has nothing to do.
        if busy && on(input, Command::CancelQuery, front).count > 0 {
            actions.push(Action::CancelQuery(active));
        }
        if !on_sql && on(input, Command::FilterBar, Scope::Grid).count > 0 {
            // An open bar without focus gets it back rather than closing.
            actions.push(if filter_open && !editing {
                Action::FocusFilterBar(active)
            } else {
                Action::ToggleFilterBar(active)
            });
        }
        if on(input, Command::FindObject, Scope::Global).count > 0 {
            actions.push(Action::OpenQuickOpen);
        }
        if on(input, Command::ToggleSidebar, Scope::Global).count > 0 {
            actions.push(Action::ToggleSidebar(active));
        }
        if tree_arrows && !focused {
            use crate::model::TreeKey;
            let tab = active;
            for keys in keymap.chords_now(layout, Command::MoveInTree, Scope::Sidebar, normal) {
                let key = match keys {
                    "up" => TreeKey::Up,
                    "down" => TreeKey::Down,
                    "left" => TreeKey::Left,
                    "right" => TreeKey::Right,
                    "home" => TreeKey::Home,
                    "end" => TreeKey::End,
                    // A letter: read with the letters.
                    _ => continue,
                };
                if presses(input, layout, keys).count > 0 {
                    actions.push(Action::TreeKey { tab, key });
                }
            }
            if on(input, Command::OpenObject, Scope::Sidebar).count > 0 {
                let key = TreeKey::Enter;
                actions.push(Action::TreeKey { tab, key });
            }
        }
        // Paging is a table's: a SQL result has one page.
        if let Some((tab, object_tab)) = object {
            if on(input, Command::PreviousPage, Scope::Grid).count > 0 {
                actions.push(Action::PrevPage { tab, object_tab });
            }
            if on(input, Command::NextPage, Scope::Grid).count > 0 {
                actions.push(Action::NextPage { tab, object_tab });
            }
        }
        // The rest acts on the tab the workspace shows, of either kind.
        if let Some((tab, id)) = any_tab {
            for (command, step) in [(Command::PreviousTab, -1), (Command::NextTab, 1)] {
                if on(input, command, Scope::Global).count > 0 {
                    actions.push(Action::CycleTab { tab, step });
                }
            }
            if on(input, Command::CloseTab, Scope::Global).count > 0 {
                actions.push(Action::CloseTab { tab, id });
            }
            if !editing && any_grid && !tree_arrows && !focused {
                let page = 20;
                let moves = [Command::MoveRow, Command::MoveColumn, Command::MovePage];
                for command in moves {
                    for keys in keymap.chords_now(layout, command, rows, normal) {
                        let (rows, cols) = match keys {
                            "up" => (-1, 0),
                            "down" => (1, 0),
                            "left" => (0, -1),
                            "right" => (0, 1),
                            "pgup" => (-page, 0),
                            "pgdn" => (page, 0),
                            "home" => (isize::MIN, 0),
                            "end" => (isize::MAX, 0),
                            // A letter: read with the letters.
                            _ => continue,
                        };
                        if presses(input, layout, keys).count > 0 {
                            actions.push(Action::MoveSelection {
                                tab,
                                id,
                                rows,
                                cols,
                            });
                        }
                    }
                }
            }
        }
    });
    // The tab at the place its digit named.
    if let Some(index) = tab_asked
        && let Some(workspace) = app.workspace(active)
        && let Some(tab) = workspace.tabs.get(index)
    {
        let (tab, id) = (active, tab.id());
        actions.push(Action::ActivateTab { tab, id });
    }
    // F6 steps through the parts of the window (the bar, the filter, the
    // tree, the tabs, the table's header, the rows, the row panel), and
    // Shift+F6 back. The terminal look steps through its panes alone with
    // ctrl+l and ctrl+h, out of a text field: the keymap has those two for
    // it only, in normal mode.
    if let Some(workspace) = app.workspace(active) {
        use crate::ui::focus::{self, Region};
        let from = match workspace.pane {
            crate::model::Pane::Tree => Region::Tree,
            crate::model::Pane::Grid => Region::Grid,
        };
        // Which of `commands` was asked for first, by its place.
        let first = |commands: [Command; 2]| {
            ctx.input_mut(|input| {
                commands.into_iter().position(|command| {
                    let scope = Scope::Global;
                    asked(input, &app.keymap, layout, command, scope, normal).count > 0
                })
            })
        };
        if let Some(index) = first([Command::PreviousPart, Command::NextPart]) {
            focus::step(ctx, index == 1, None, Some(from));
        } else if let Some(index) = first([Command::PaneLeft, Command::PaneRight]) {
            // A row of the table on screen is selected, and no control
            // has the keyboard: the keys are the grid's.
            let on_grid = grid && !focused && from == Region::Grid;
            let row = object.filter(|(tab, id)| {
                let object = app.workspace(*tab).and_then(|w| w.object_tab(*id));
                on_grid && object.is_some_and(|object| object.selection.is_some())
            });
            match (index, field, object, row) {
                // From a field of the row panel, back to the grid, on the
                // cell of that column.
                (0, Some(col), Some((tab, id)), _) => {
                    let stop = crate::ui::row_panel::field_stop(tab, id, col);
                    ctx.memory_mut(|memory| memory.surrender_focus(stop));
                    actions.push(Action::FieldFocused { tab, id, col });
                    actions.push(Action::GridKeys(tab));
                }
                // From the grid, to the row's fields: the panel is shown
                // for it.
                (1, None, _, Some((tab, id))) => {
                    actions.push(Action::FocusFields { tab, id });
                }
                _ => focus::step(ctx, index == 1, Some(&Region::PANES), Some(from)),
            }
        }
    }
    // The Ctrl+C that drops the terminal's edit may be held. Its repeats
    // come as copies like the first (a copy says nothing of being a
    // repeat), and with the editor gone they would copy the cell.
    if terminal {
        held_copy(ctx, open);
    }
    if !editing && grid {
        // The row's chord first: with Shift exact neither is the other's.
        // They come as the window's copy, which `presses` reads. A look
        // with no chord for them copies nothing here.
        let copied = ctx.input_mut(|input| {
            [Command::CopyRow, Command::CopyCells]
                .into_iter()
                .find(|command| {
                    let scope = Scope::Grid;
                    asked(input, &app.keymap, layout, *command, scope, normal).count > 0
                })
        });
        let row = copied == Some(Command::CopyRow);
        if copied.is_some()
            && let Some(text) = app.copy_text(row)
        {
            ctx.copy_text(text);
        }
    }
    // The Settings window, wherever the keyboard is.
    let settings = |input: &mut egui::InputState| {
        let scope = Scope::Global;
        asked(input, &app.keymap, layout, Command::Settings, scope, normal).count > 0
    };
    if ctx.input_mut(settings) {
        actions.push(Action::ShowSettings);
    }
    // Before the letters: a chord that edits is no letter's, and the key
    // that opens a field is taken before anything reads what it typed.
    if app.dialog.is_none() {
        let keyboard = !editing && grid && !tree_arrows && !focused;
        let on_field = field.filter(|_| !editing);
        editing_keys(app, ctx, (keyboard, normal), on_field, &mut actions);
    }
    if !editing && app.dialog.is_none() {
        // A click ends a first key's wait: what is typed after it is typed
        // at what the click chose.
        if ctx.input(|input| input.pointer.any_pressed()) {
            forget_pending(ctx);
        }
        letters(app, ctx, field, (scopes, tree_arrows), &mut actions);
    } else {
        // The keys are a field's: a first key that waited is not the
        // first of what is typed once they are the grid's again.
        forget_pending(ctx);
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
/// one asks for a list, and an open one (`completing`) takes the keys that
/// move in it, insert from it and close it. Which keys is the keymap's to
/// say; they are read with the keyboard in the text.
fn completion_keys(
    input: &mut egui::InputState,
    (tab, sql_tab): (ConnTabId, TabId),
    completing: Option<Completing>,
    (keymap, layout): (&Keymap, Layout),
    actions: &mut Vec<Action>,
) {
    let scope = Scope::SqlEditor;
    let chords = |command: Command| -> Vec<&'static str> {
        keymap.chords_now(layout, command, scope, false).collect()
    };
    // Every time any of `chords` went down, repeats too. All are taken.
    let take = |input: &mut egui::InputState, chords: &[&'static str]| -> usize {
        let pressed = chords.iter().map(|keys| presses(input, layout, keys).count);
        pressed.sum()
    };
    // The Mac has two chords that ask for a list: some systems take
    // Ctrl+Space before the app sees it (macOS switches input sources with
    // it, and an input method may be woken by it).
    if take(input, &chords(Command::Complete)) > 0 {
        actions.push(Action::OpenCompletion { tab, sql_tab });
    }
    let Some(list) = completing else {
        return;
    };
    // The arrows are the list's once it has a row to move to. Its other
    // keys (Omarchy's Ctrl+N and Ctrl+P) are its own as long as it is
    // open, rows or not.
    for (command, step) in [
        (Command::NextCompletion, 1),
        (Command::PreviousCompletion, -1),
    ] {
        for keys in chords(command) {
            let arrow = matches!(keys, "up" | "down");
            if arrow && !list.has_row {
                continue;
            }
            // One row for each press: a frame may hold several.
            let presses = presses(input, layout, keys).count;
            actions.extend((0..presses).map(|_| Action::MoveCompletion { tab, sql_tab, step }));
        }
    }
    let accept = Action::AcceptCompletion {
        tab,
        sql_tab,
        row: None,
    };
    let close = Action::CloseCompletion { tab, sql_tab };
    // The keys that insert: Enter, which is also the editor's line break,
    // and the others (Tab), which are the list's alone.
    let inserts = chords(Command::InsertCompletion);
    let (enters, others): (Vec<_>, Vec<_>) = inserts.into_iter().partition(|keys| *keys == "enter");
    // Whether one of `chords` went down, left in the frame for the editor.
    let pressed = |input: &egui::InputState, chords: &[&'static str]| {
        let held = input.modifiers;
        input.events.iter().any(|event| {
            chords
                .iter()
                .any(|keys| press_of(layout, held, event, keys).is_some())
        })
    };
    if list.has_row {
        if input.events.iter().any(inserts_text) {
            // What this frame types is not what the list was worked out
            // from: no row goes in. Tab is taken all the same, or it would
            // put a tab character into the word being typed; whether the
            // list goes on is the refresh's to say, for the word as it
            // reads after this frame. Enter stays the editor's line break,
            // and the list is done.
            take(input, &others);
            if pressed(input, &enters) {
                actions.push(close);
            }
        } else if take(input, &others) > 0 {
            actions.push(accept);
        } else if list.enter_breaks {
            // The editor gets its line break; the list is done.
            if pressed(input, &enters) {
                actions.push(close);
            }
        } else if take(input, &enters) > 0 {
            actions.push(accept);
        }
    }
    if take(input, &chords(Command::CloseCompletion)) > 0 {
        actions.push(Action::CloseCompletion { tab, sql_tab });
    }
}

/// Whether `event` puts text into the field that has the keyboard: typed
/// text, as the SQL editor takes it, or a paste.
fn inserts_text(event: &egui::Event) -> bool {
    crate::ui::sql_text::is_typed(event)
        || matches!(event, egui::Event::Paste(text) if !text.is_empty())
}

/// The chords that edit the cells of the table on screen: they open an
/// editor on the selected cell, act on it without one, and save or drop
/// what is pending. Which chord does what is the keymap's to say, for the
/// look in use: Omarchy has few of them, and edits with letters (see
/// `letters`). `keyboard` says the grid's keys are the grid's: no field or
/// button has them, nor the tree. A save does not wait for that, in any
/// look: it is read from wherever the table's tab shows, and so are the
/// chords the keymap gives to any mode. A SQL editor's result takes none
/// of them.
///
/// `field` is the column of the row panel's field that has the keyboard:
/// the same keys then act on its cell, and open its editor in the panel.
/// Its own keys step the fields and give the keyboard back to the grid.
fn editing_keys(
    app: &App,
    ctx: &egui::Context,
    (keyboard, normal): (bool, bool),
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
    let (keymap, layout) = (&app.keymap, app.layout());
    let on = |input: &mut egui::InputState, command: Command, scope: Scope, normal: bool| {
        asked(input, keymap, layout, command, scope, normal)
    };
    let open = object.edits.editor.is_some();
    let editor = crate::ui::cell_editor::field_id(tab, id);
    let typing = open && ctx.memory(|memory| memory.has_focus(editor));
    // A save is read wherever the pending bar offers it with its key:
    // there is something to save, whatever has the keyboard (the grid, the
    // tree, a button, the filter's field) and in the Structure view too.
    let save = |input: &mut egui::InputState| {
        on(input, Command::SaveChanges, Scope::Grid, normal).count > 0
    };
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
    // something pending or an editor open, in the looks that have a chord
    // for it. A fresh press only: a held chord would show and hide it by
    // turns.
    let review =
        |input: &mut egui::InputState| on(input, Command::ReviewSql, Scope::Grid, normal).fresh;
    if (open || object.edits.pending()) && ctx.input_mut(review) {
        // Shown, the review takes what is being typed, as a save does:
        // noted as typed for the reason the save notes it.
        if typing {
            actions.push(Action::EditorTyped { tab, id });
        }
        let show = !object.edits.reviewing;
        actions.push(Action::ReviewEdits { tab, id, show });
    }
    // The inspector's chord puts the keyboard on the row's fields in the
    // row panel, which it shows: from wherever the table's rows show,
    // whatever has the keyboard, and with nothing pending. Only the chords
    // the keymap gives to any mode are read here (asked for as if a field
    // had the keyboard): a plain key among the command's (Omarchy's Enter)
    // is the rows' own, and is read below, where the keys are the grid's.
    // Not while an editor is open, which is where the user is, and not in
    // a frame that brings a click: the click selects its row once the
    // frame is drawn, and the key would reach the row the selection
    // leaves. A fresh press only.
    let focus_fields = |input: &mut egui::InputState| {
        let clicked = input
            .events
            .iter()
            .any(|event| matches!(event, egui::Event::PointerButton { .. }));
        on(input, Command::OpenInspector, Scope::Grid, false).fresh && !clicked
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
    // The keys are the rows' or a field's: normal mode's, of that scope.
    let scope = if field.is_some() {
        Scope::Inspector
    } else {
        Scope::Grid
    };
    let terminal = app.look.terminal;
    // The field the keys were given back from: it gives up the keyboard
    // once the input is read.
    let mut left = None;
    ctx.input_mut(|input| {
        // Each chord with exactly what it names held: the one that drops
        // every change is not the one that touches one cell.
        if on(input, Command::DiscardChanges, Scope::Grid, true).count > 0 {
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
        if on(input, Command::SetNull, scope, true).count > 0 {
            on_cell(actions);
            // A refusal is said where it was asked for.
            actions.push(match field {
                Some(_) => Action::SetFieldNull { tab, id },
                None => Action::SetNull { tab, id },
            });
        }
        if on(input, Command::SetDefault, scope, true).count > 0 {
            on_cell(actions);
            actions.push(Action::SetDefault { tab, id });
        }
        if on(input, Command::UndoCell, scope, true).count > 0 {
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
        // The row's delete drops a new row: nothing of it is in the
        // table. On a row of the page it does nothing yet.
        let on_new = crate::edit::new_id(selected.row).is_some() && field.is_none();
        let deletes = on(input, Command::DeleteRow, Scope::Grid, true).count;
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
        // The rows' own key for the inspector, in the look that has one
        // (Omarchy's Enter, which edits nothing there). A fresh press: a
        // held Enter would open the panel and then edit its field.
        if field.is_none() && on(input, Command::OpenInspector, scope, true).fresh {
            actions.push(Action::FocusFields { tab, id });
            return;
        }
        // A fresh press only: a held Enter would open, commit and move
        // down the whole column.
        if on(input, Command::EditCell, scope, true).fresh {
            actions.push(edit(crate::model::EditStart::Value));
            return;
        }
        if let Some(col) = field {
            // The fields are stepped, and the keys given back to the grid,
            // on the cell of this field's column. The letters that step
            // them are read with the letters.
            for keys in keymap.chords_now(layout, Command::MoveField, scope, true) {
                let by = match keys {
                    "up" => -1,
                    "down" => 1,
                    _ => continue,
                };
                if presses(input, layout, keys).count > 0 {
                    let from = col;
                    actions.push(Action::MoveField { tab, id, from, by });
                }
            }
            if on(input, Command::BackToGrid, scope, true).fresh {
                left = Some(crate::ui::row_panel::field_stop(tab, id, col));
                on_cell(actions);
                actions.push(Action::GridKeys(tab));
                return;
            }
        }
        // Typing starts the edit with what was typed, in the looks whose
        // letters are no keys. The text is taken, or the field that opens
        // would get it again. Space and `?` keep their meaning (the
        // booleans' key, the list of keys), and a chord types nothing.
        if terminal {
            return;
        }
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

/// Whether `sql` shows a result grid for the keys to move in: its last
/// run's rows, unless the Messages pane covers them.
fn shows_grid(sql: &crate::model::SqlTab) -> bool {
    sql.pane == crate::model::ResultPane::Results && sql.dims().0 > 0
}

/// Ends the wait of a key of several (`cc`, `gd`, `yy p`): the character
/// after it is a first one again. It waits through frames that bring no
/// key, and no longer than the keys stay the grid's: not past a click, a
/// field that has the keyboard, the `:` prompt or a dialog.
pub(crate) fn forget_pending(ctx: &egui::Context) {
    ctx.data_mut(|data| data.insert_temp(typed_id(), String::new()));
}

/// The characters typed so far of a key of several (`yy p`), as the
/// keymap reads them, kept between frames.
fn typed_id() -> egui::Id {
    egui::Id::new("typed-keys")
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

/// Single letters: the picker's keys, and in the terminal look the
/// workspace's vim keys. Only when no text field has the keyboard. `field`
/// is the column of the row panel's field that has it: `j` and `k` step
/// the fields then, and the letters that edit a cell edit that field.
fn letters(
    app: &mut App,
    ctx: &egui::Context,
    field: Option<usize>,
    (scopes, tree_arrows): (&[Scope], bool),
    actions: &mut Vec<Action>,
) {
    // The letters the keymap has been given so far, read before anything
    // returns: a frame can bring a typed `?` and no key press, and the
    // list of keys is every look's. A character that is one of them is
    // taken from the frame, its key with it; any other is left for the
    // code below.
    let layout = app.layout();
    let nothing = |_: crate::keymap::When| false;
    let mine = [
        Command::Reload,
        Command::FindObject,
        Command::ToggleTree,
        Command::Help,
    ];
    for char in typed_chars(ctx) {
        let read = scopes
            .iter()
            .map(|scope| {
                let typed = char.to_string();
                app.keymap.typed(layout, *scope, &nothing, &typed)
            })
            .find(|read| *read != Typed::Nothing);
        let Some(Typed::Command(command)) = read else {
            continue;
        };
        if !mine.contains(&command) {
            continue;
        }
        take_char(ctx, char);
        let tab = app.active_tab_id();
        match command {
            // A SQL editor has nothing to reload.
            Command::Reload if app.active_sql().is_some() => {}
            Command::Reload if tree_arrows => actions.push(Action::RefreshTree(tab)),
            Command::Reload => actions.push(Action::Refresh(tab)),
            Command::FindObject => {
                use crate::ui::focus::{self, Region};
                focus::step(ctx, true, Some(&[Region::Search]), None);
            }
            Command::ToggleTree => actions.push(Action::ToggleFlatTree(tab)),
            Command::Help => actions.push(Action::ShowHelp),
            _ => {}
        }
    }
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
    let left_field: bool = ctx
        .data(|data| data.get_temp(was_editing_id()))
        .unwrap_or(false);
    // An open menu keeps its Esc: this runs before the menu is drawn, and
    // the key closes the menu before it closes anything under it.
    let menu_open = egui::Popup::is_any_open(ctx);
    if let crate::model::ConnTabContent::Picker(picker) = &app.active_tab().content {
        let selected = picker.selected.clone();
        let scope = Scope::Connections;
        // No text field has the keyboard here: these are normal mode's.
        let on = |command: Command| {
            ctx.input_mut(|input| asked(input, &app.keymap, layout, command, scope, true))
        };
        // The list's arrows. Its letters are read with the letters, below.
        for keys in app
            .keymap
            .chords_now(layout, Command::MoveInConnections, scope, true)
        {
            let step = match keys {
                "up" => -1,
                "down" => 1,
                _ => continue,
            };
            if ctx.input_mut(|input| presses(input, layout, keys)).count > 0 {
                actions.push(Action::MovePickerSelection { tab, step });
            }
        }
        // Enter shows a connection that is open already; with Shift it
        // opens once more.
        let connect = |conn: &crate::connections::ConnectionId, again: bool| match app
            .tab_showing(conn)
            .filter(|_| !again)
        {
            Some(open) => Action::ActivateConnTab(open),
            None => Action::Connect {
                tab,
                conn: conn.clone(),
            },
        };
        if let Some(conn) = &selected {
            if !crate::ui::focus::on_control(ctx) {
                let again = on(Command::ConnectAgain).count > 0;
                if again || on(Command::Connect).count > 0 {
                    actions.push(connect(conn, again));
                }
            }
            if on(Command::EditConnection).count > 0 {
                actions.push(Action::EditConnection(conn.clone()));
            }
            if on(Command::DuplicateConnection).count > 0 {
                actions.push(Action::DuplicateConnection(conn.clone()));
            }
            if on(Command::DeleteConnection).count > 0 {
                actions.push(Action::DeleteConnection(conn.clone()));
            }
        }
        // The letters, with the wait the keymap needs for a key of
        // several (`yy p`): the characters so far, kept between frames.
        let typed = typed_chars(ctx);
        let mut waiting: String = ctx
            .data(|data| data.get_temp(typed_id()))
            .unwrap_or_default();
        // A key that types nothing (an arrow, Esc) ends a wait.
        if typed.is_empty() {
            waiting.clear();
        }
        let nothing = |_: crate::keymap::When| false;
        for char in typed {
            waiting.push(char);
            let read = app.keymap.typed(layout, scope, &nothing, &waiting);
            // No text field is under these keys: the character is taken
            // whatever it came to.
            take_char(ctx, char);
            let command = match read {
                Typed::Waiting => continue,
                Typed::Command(command) => Some(command),
                // Nobody's, or the key of something not built yet. A
                // character that ends a wait is not read again as a first.
                Typed::Claimed | Typed::Nothing => None,
            };
            let step = if char == 'k' { -1 } else { 1 };
            match (command, &selected) {
                (Some(Command::NewConnection), _) => actions.push(Action::NewConnection),
                (Some(Command::FilterConnections), _) => {
                    actions.push(Action::FocusPickerSearch(tab));
                }
                (Some(Command::MoveInConnections), _) => {
                    actions.push(Action::MovePickerSelection { tab, step });
                }
                (Some(Command::EditConnection), Some(conn)) => {
                    actions.push(Action::EditConnection(conn.clone()));
                }
                (Some(Command::DuplicateConnection), Some(conn)) => {
                    actions.push(Action::DuplicateConnection(conn.clone()));
                }
                (Some(Command::DeleteConnection), Some(conn)) => {
                    actions.push(Action::DeleteConnection(conn.clone()));
                }
                _ => {}
            }
            waiting.clear();
        }
        ctx.data_mut(|data| data.insert_temp(typed_id(), waiting));
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
    let object = workspace.active_object_tab();
    let active = object.map(|object| object.id);
    let selection = object.and_then(|object| object.selection);
    // The rows of the table show: where the letters that edit a cell and
    // the one that drops a row are read, and not in the Structure view.
    let rows_show = object.is_some_and(|object| object.view == crate::model::ObjectView::Data);
    let on_sql = workspace.active_sql_tab().is_some();
    // A SQL editor showing its result takes the letters that move in it.
    let sql_grid = workspace
        .active_sql_tab()
        .filter(|sql| shows_grid(sql))
        .map(|sql| sql.id);
    // And, with a row of it selected, the keys of the row panel.
    let sql_row = workspace
        .active_sql_tab()
        .filter(|sql| sql.selected_row().is_some())
        .map(|sql| sql.id);
    // The table on screen while its Review SQL is open, in either of its
    // views: the panel stands under both.
    let reviewing = object
        .filter(|object| object.edits.reviewing)
        .map(|object| object.id);
    // What the last save came to, with nothing pending any more.
    let note = object
        .filter(|object| object.edits.note.is_some() && !object.edits.holds())
        .map(|object| object.id);
    // Enter belongs to a focused button, and so do the letters that edit.
    let focused = crate::ui::focus::on_control(ctx);
    let layout = app.layout();
    // A chord of the keymap, read in normal mode: no field has the keys.
    let on = |command: Command, scope: Scope| {
        ctx.input_mut(|input| asked(input, &app.keymap, layout, command, scope, true))
    };
    let rows = if on_sql { Scope::Results } else { Scope::Grid };
    // Esc, by what is up, the first that answers taking the key: it does
    // no more. Nor does an Esc that left a text field, or the repeats of
    // one held since. It closes the Review SQL panel, as its foot says;
    // then it takes what the last save came to off the status line, as
    // Dismiss does in the other looks; then it closes the row panel. (On a
    // field of the panel it gave the keys back to the grid, before this.)
    let quiet = !left_field && !menu_open;
    if let Some(id) = reviewing
        && quiet
        && on(Command::CloseReview, Scope::Grid).fresh
    {
        let show = false;
        actions.push(Action::ReviewEdits { tab, id, show });
    }
    if let Some(id) = note
        && quiet
        && on(Command::DismissNote, Scope::Grid).fresh
    {
        actions.push(Action::DismissNote { tab, id });
    }
    // The row panel's keys: an object tab's, or a SQL result's while a row
    // of it is selected (with none its panel has nothing to show).
    let panel_tab = active.or(sql_row);
    if panel_tab.is_some() && panel && quiet && on(Command::CloseInspector, rows).fresh {
        actions.push(Action::ToggleRowPanel(tab));
    }
    // Enter shows the panel of a SQL result's row. (A table's rows are
    // read with its chords, where Enter puts the keyboard on the fields.)
    let result_row = active.is_none() && sql_row.is_some() && !tree && !panel && !focused;
    if result_row && on(Command::OpenInspector, Scope::Results).count > 0 {
        actions.push(Action::ToggleRowPanel(tab));
    }

    // The letters, read as the text they type, in the order they came,
    // and taken: a letter that opens an editor or the prompt is no part of
    // its text, and what follows it in its frame does nothing (the field
    // is not there yet to be typed into).
    //
    // The ones that edit act on the selected cell, so they act only in a
    // frame that brings nothing else: beside a key that moves the
    // selection, or anything that was asked for already (`actions`), the
    // order the two came in is lost, and the letter might reach another
    // cell than the one it was typed on. It is dropped there. So beside a
    // pointer's button, going down or coming up: a click selects its cell
    // when the grid is drawn, after these keys were read, and the letter
    // would act on the cell the selection leaves.
    let typed = typed_chars(ctx);
    let (keys, pointer, fresh_key) = ctx.input(|input| {
        let down = |event: &&egui::Event| matches!(event, egui::Event::Key { pressed: true, .. });
        let button = |event: &egui::Event| matches!(event, egui::Event::PointerButton { .. });
        let fresh = |event: &egui::Event| {
            matches!(
                event,
                egui::Event::Key {
                    pressed: true,
                    repeat: false,
                    ..
                }
            )
        };
        (
            input.events.iter().filter(down).count(),
            input.events.iter().any(button),
            input.events.iter().any(fresh),
        )
    });
    // Where the letters that edit a cell are read: on its rows, or on a
    // field of its row panel, and not on a button, whose key Enter is.
    let edits = rows_show && !tree && (field.is_some() || !focused);
    // On a field of the row panel the letters act on its cell, and edit
    // it in the panel.
    let edit = |start: crate::model::EditStart| {
        let selected = selection?;
        let id = active?;
        Some(match field {
            Some(col) => Action::EditField {
                tab,
                id,
                cell: crate::model::CellPos {
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
        if let (Some(col), Some(id)) = (field.filter(|col| Some(*col) != selected), active) {
            actions.push(Action::FieldFocused { tab, id, col });
        }
    };
    // The card of a refused write, while a SQL editor's Messages show it:
    // the command its button stands for, and what the button does.
    let mut card = crate::ui::sql_results::card_key(app, tab);
    let has_card = card.is_some();
    let holds = |when: crate::keymap::When| match when {
        crate::keymap::When::InspectorOpen => panel,
        crate::keymap::When::Review => reviewing.is_some(),
        crate::keymap::When::Note => note.is_some(),
        crate::keymap::When::RefusedWrite => has_card,
        _ => false,
    };
    let review_only = |when: crate::keymap::When| when == crate::keymap::When::Review;
    // What the characters typed so far come to, for the scope that has
    // the keys. The Review SQL panel's own letter is read wherever the
    // keys are in its tab: the panel stands under the tree's rows too.
    let read = |waiting: &str| {
        let read = scopes
            .iter()
            .map(|scope| app.keymap.typed(layout, *scope, &holds, waiting))
            .find(|read| *read != Typed::Nothing)
            .unwrap_or(Typed::Nothing);
        if read == Typed::Nothing && reviewing.is_some() {
            let panels = app.keymap.typed(layout, Scope::Grid, &review_only, waiting);
            if panels == Typed::Command(Command::CopyReviewSql) {
                return panels;
            }
        }
        read
    };
    // The characters so far of a key of several (`cc`, `gd`, `yy p`). A
    // key that types nothing (an arrow, Esc) ends the wait.
    let mut waiting: String = ctx
        .data(|data| data.get_temp(typed_id()))
        .unwrap_or_default();
    if typed.is_empty() {
        waiting.clear();
    }
    // Whether the frame brings nothing but letters that edit, the colon
    // among them: read through once before any of them acts, since one
    // that moves the selection may come after the one that edits.
    let only_edits = {
        let mut waiting = waiting.clone();
        typed.iter().all(|char| {
            if *char == ':' && waiting.is_empty() {
                return true;
            }
            waiting.push(*char);
            let read = read(&waiting);
            if read != Typed::Waiting {
                waiting.clear();
            }
            matches!(
                read,
                Typed::Waiting
                    | Typed::Command(
                        Command::EditCell
                            | Command::ReplaceCell
                            | Command::SetNull
                            | Command::SetDefault
                            | Command::UndoCell
                            | Command::AddRow
                            | Command::AddRowAbove
                    )
            )
        })
    };
    let alone = actions.is_empty() && !pointer && keys <= typed.len() && only_edits;
    // Whether a field or the prompt was asked for: the frame's typing ends
    // there.
    let mut opened = false;
    for char in typed {
        if opened {
            take_char(ctx, char);
            continue;
        }
        // The colon opens the prompt, wherever a tab of the workspace is
        // in front: a table's or a SQL editor's, with the keys on the tree
        // too. It is no key of the keymap's: its lines are.
        if char == ':' && waiting.is_empty() {
            take_char(ctx, char);
            if alone {
                opened = true;
                actions.push(Action::OpenCommand(tab));
            }
            continue;
        }
        waiting.push(char);
        let command = match read(&waiting) {
            // Nobody's: left in the frame, as any other text is.
            Typed::Nothing => {
                waiting.clear();
                continue;
            }
            Typed::Waiting => {
                take_char(ctx, char);
                continue;
            }
            // The key of something that is not built yet: taken.
            Typed::Claimed => None,
            Typed::Command(command) => Some(command),
        };
        take_char(ctx, char);
        waiting.clear();
        match (command, active) {
            (Some(Command::EditCell), _) if edits && alone => {
                opened = true;
                actions.extend(edit(crate::model::EditStart::Value));
            }
            (Some(Command::ReplaceCell), _) if edits && alone => {
                opened = true;
                actions.extend(edit(crate::model::EditStart::Replace(String::new())));
            }
            (Some(Command::SetNull), Some(id)) if edits && alone => {
                on_cell(actions);
                // A refusal is said where it was asked for.
                actions.push(match field {
                    Some(_) => Action::SetFieldNull { tab, id },
                    None => Action::SetNull { tab, id },
                });
            }
            (Some(Command::SetDefault), Some(id)) if edits && alone => {
                on_cell(actions);
                actions.push(Action::SetDefault { tab, id });
            }
            (Some(Command::UndoCell), Some(id)) if edits && alone => {
                on_cell(actions);
                let cell = None;
                actions.push(Action::RevertCell { tab, id, cell });
            }
            // A row below the cursor's, or above it with the capital.
            // With no cursor, at the top.
            (Some(command @ (Command::AddRow | Command::AddRowAbove)), Some(id))
                if edits && alone =>
            {
                opened = true;
                let place = match (selection, command) {
                    (Some(cell), Command::AddRow) => crate::edit::Place::Below(cell.row),
                    (Some(cell), _) => crate::edit::Place::Above(cell.row),
                    (None, _) => crate::edit::Place::Top,
                };
                actions.push(Action::AddRow { tab, id, place });
            }
            // The row under the cursor is dropped where it is a new one,
            // and only with the keys on the rows.
            (Some(Command::DeleteRow), Some(id)) if rows_show && !tree && field.is_none() => {
                actions.push(Action::DropRow { tab, id });
            }
            (Some(Command::OpenReferencedRow), Some(object_tab)) => {
                actions.push(Action::FollowSelectedKey { tab, object_tab });
            }
            (Some(Command::CopyCells), Some(_)) => {
                if let Some(text) = app.copy_text(false) {
                    ctx.copy_text(text);
                }
            }
            // The whole statements, never the lines as the panel shows
            // them. The copy shows nowhere, so the status line says it.
            (Some(Command::CopyReviewSql), Some(id)) => {
                if let Some(text) = crate::ui::review::copy_text(app, tab, id) {
                    ctx.copy_text(text);
                    let said = crate::i18n::gettext(app.locale, crate::ui::review::COPIED_SQL);
                    crate::ui::toast::say(ctx, &said);
                }
            }
            (Some(Command::WhereFilter), Some(_)) => actions.push(Action::FocusWhere(tab)),
            // What `j/k/h/l` move in: the object's grid, or a SQL result.
            (Some(Command::MoveRow | Command::MoveColumn), _) => {
                let (rows, cols) = match char {
                    'j' => (1, 0),
                    'k' => (-1, 0),
                    'h' => (0, -1),
                    _ => (0, 1),
                };
                if let Some(id) = active.or(sql_grid) {
                    actions.push(Action::MoveSelection {
                        tab,
                        id,
                        rows,
                        cols,
                    });
                }
            }
            // On a field of the row panel `j` and `k` step the fields.
            (Some(Command::MoveField), Some(id)) => {
                if let Some(from) = field {
                    let by = if char == 'k' { -1 } else { 1 };
                    actions.push(Action::MoveField { tab, id, from, by });
                }
            }
            (Some(Command::MoveInTree), _) => {
                let key = if char == 'k' {
                    crate::model::TreeKey::Up
                } else {
                    crate::model::TreeKey::Down
                };
                actions.push(Action::TreeKey { tab, key });
            }
            // The row before and the one after, from a field of the
            // panel or from the rows it shows one of.
            (Some(command @ (Command::PreviousRow | Command::NextRow)), _) => {
                let rows = if command == Command::NextRow { 1 } else { -1 };
                let cols = 0;
                if let Some(id) = active.or(sql_grid) {
                    actions.push(Action::MoveSelection {
                        tab,
                        id,
                        rows,
                        cols,
                    });
                }
            }
            // The script of the SQL editor in front, with the keyboard
            // out of its text: formatting gives it back.
            (Some(Command::Format), _) => {
                if let Some(sql_tab) = app.active_sql().map(|(_, id)| id) {
                    actions.push(Action::FormatSql { tab, sql_tab });
                }
            }
            // The letter the card's button names. A press of its own:
            // once writes are allowed the same letter runs the statements
            // again, and the repeats of a key held since must not answer
            // an offer nobody has read.
            (Some(command @ (Command::EditRefusedConnection | Command::AllowRefusedWrite)), _) => {
                let offered = card.as_ref().map(|(offered, _)| *offered);
                if fresh_key
                    && offered == Some(command)
                    && let Some((_, action)) = card.take()
                {
                    actions.push(action);
                }
            }
            // Only the documents of a panel that shows.
            (Some(Command::FoldDocuments), _) => {
                if let Some(id) = panel_tab.filter(|id| shown == Some(*id)) {
                    actions.push(Action::FoldDocuments { tab, id });
                }
            }
            _ => {}
        }
    }
    ctx.data_mut(|data| data.insert_temp(typed_id(), waiting));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_press_matches_its_modifiers_as_each_platform_reports_them() {
        // The Mac's layout and Windows', whose chords are spelled with
        // `cmd` for the command key: Cmd on macOS, Ctrl elsewhere.
        let named = |cmd, ctrl| Mods {
            cmd,
            ctrl,
            ..Mods::default()
        };
        let is = |held, mods| {
            [Layout::Mac, Layout::Windows]
                .into_iter()
                .all(|layout| held_is(layout, held, mods))
        };
        let is_not = |held, mods| {
            [Layout::Mac, Layout::Windows]
                .into_iter()
                .all(|layout| !held_is(layout, held, mods))
        };
        let cmd = Modifiers::MAC_CMD | Modifiers::COMMAND;
        let ctrl_elsewhere = Modifiers::CTRL | Modifiers::COMMAND;
        // The command key: Cmd on macOS, with Ctrl or without, and Ctrl
        // elsewhere.
        for held in [
            Modifiers::COMMAND,
            cmd,
            cmd | Modifiers::CTRL,
            ctrl_elsewhere,
        ] {
            assert!(is(held, named(true, false)), "{held:?}");
        }
        for held in [
            Modifiers::NONE,
            Modifiers::CTRL,
            cmd | Modifiers::SHIFT,
            ctrl_elsewhere | Modifiers::ALT,
        ] {
            assert!(is_not(held, named(true, false)), "{held:?}");
        }
        // Ctrl: alone on macOS, with the command key elsewhere, never Cmd.
        for held in [Modifiers::CTRL, ctrl_elsewhere] {
            assert!(is(held, named(false, true)), "{held:?}");
        }
        for held in [
            Modifiers::NONE,
            cmd,
            cmd | Modifiers::CTRL,
            Modifiers::CTRL | Modifiers::SHIFT,
        ] {
            assert!(is_not(held, named(false, true)), "{held:?}");
        }
        // No modifier is none at all.
        assert!(is(Modifiers::NONE, Mods::default()));
        for held in [Modifiers::SHIFT, Modifiers::ALT, Modifiers::CTRL, cmd] {
            assert!(is_not(held, Mods::default()), "{held:?}");
        }
        // Another key, and a key coming up, are not this press.
        let none = Modifiers::NONE;
        let press = crate::testing::key(Key::I, none);
        assert_eq!(press_of(Layout::Mac, none, &press, "i"), None, "a letter");
        assert_eq!(press_of(Layout::Mac, none, &press, "f2"), None);
        let up = crate::testing::release(Key::F2, none);
        assert_eq!(press_of(Layout::Mac, none, &up, "f2"), None);
        let down = crate::testing::key(Key::F2, none);
        assert_eq!(press_of(Layout::Mac, none, &down, "f2"), Some(true));
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
    fn what_is_held_is_read_as_the_layout_names_it() {
        let mods = |cmd, ctrl, alt, shift| Mods {
            cmd,
            ctrl,
            alt,
            shift,
        };
        let cmd = Modifiers::MAC_CMD | Modifiers::COMMAND;
        // As Linux reports Ctrl: both.
        let linux_ctrl = Modifiers::CTRL | Modifiers::COMMAND;
        // Omarchy has one command key. Either name of it is Ctrl.
        for held in [Modifiers::CTRL, Modifiers::COMMAND, linux_ctrl] {
            assert!(held_is(
                Layout::Omarchy,
                held,
                mods(false, true, false, false)
            ));
            assert!(!held_is(Layout::Omarchy, held, Mods::default()));
            assert!(!held_is(
                Layout::Omarchy,
                held,
                mods(false, true, false, true)
            ));
        }
        assert!(held_is(
            Layout::Omarchy,
            linux_ctrl | Modifiers::SHIFT,
            mods(false, true, false, true)
        ));
        // Cmd is no key of Omarchy's, alone or with Ctrl.
        for held in [cmd, cmd | Modifiers::CTRL] {
            for wanted in [Mods::default(), mods(false, true, false, false)] {
                assert!(!held_is(Layout::Omarchy, held, wanted), "{held:?}");
            }
        }
        // Shift and Alt are exact everywhere: Ctrl+Shift+C is not Ctrl+C.
        assert!(!held_is(
            Layout::Omarchy,
            linux_ctrl | Modifiers::SHIFT,
            mods(false, true, false, false)
        ));
        // The Mac tells Cmd from Control.
        let (command, control) = (
            mods(true, false, false, false),
            mods(false, true, false, false),
        );
        assert!(held_is(Layout::Mac, cmd, command));
        assert!(!held_is(Layout::Mac, cmd, control));
        assert!(held_is(Layout::Mac, Modifiers::CTRL, control));
        assert!(!held_is(Layout::Mac, Modifiers::CTRL, command));
        // Windows' Ctrl is the command key, and the Control of Ctrl+Space.
        assert!(held_is(Layout::Windows, linux_ctrl, command));
        assert!(held_is(Layout::Windows, linux_ctrl, control));
        assert!(held_is(Layout::Mac, Modifiers::NONE, Mods::default()));
    }

    #[test]
    fn a_chord_is_counted_fresh_and_repeated_and_taken() {
        let mut input = egui::InputState::default();
        let held = Modifiers::CTRL | Modifiers::COMMAND | Modifiers::SHIFT;
        let repeat = egui::Event::Key {
            key: Key::D,
            physical_key: None,
            pressed: true,
            repeat: true,
            modifiers: held,
        };
        let other = crate::testing::key(Key::D, Modifiers::CTRL | Modifiers::COMMAND);
        input.events = vec![crate::testing::key(Key::D, held), repeat, other.clone()];
        let pressed = presses(&mut input, Layout::Omarchy, "ctrl+shift+d");
        assert_eq!((pressed.fresh, pressed.count), (true, 2));
        // Ctrl+D was another chord, and stays.
        assert_eq!(input.events, vec![other]);
        // A bracket with Shift comes as a brace on a US keyboard.
        input.events = vec![crate::testing::key(Key::OpenCurlyBracket, held)];
        assert!(presses(&mut input, Layout::Omarchy, "ctrl+shift+[").fresh);
        // Letters and a line of the prompt are no chord.
        input.events = vec![crate::testing::key(Key::D, Modifiers::NONE)];
        assert_eq!(
            presses(&mut input, Layout::Omarchy, "dd"),
            Presses::default()
        );
        assert_eq!(
            presses(&mut input, Layout::Omarchy, ":w"),
            Presses::default()
        );
    }

    #[test]
    fn the_command_key_with_c_x_or_v_is_read_from_the_clipboard_event() {
        // What the window sends for Ctrl+Shift+C: no key, a copy.
        let mut input = egui::InputState::default();
        input.modifiers = Modifiers::CTRL | Modifiers::COMMAND | Modifiers::SHIFT;
        input.events = vec![egui::Event::Copy];
        assert!(
            !presses(&mut input, Layout::Omarchy, "ctrl+c").fresh,
            "Shift is held"
        );
        assert_eq!(input.events, vec![egui::Event::Copy], "left for its owner");
        assert!(presses(&mut input, Layout::Omarchy, "ctrl+shift+c").fresh);
        assert!(input.events.is_empty(), "taken");
        // A cut, and a paste.
        input.modifiers = Modifiers::CTRL | Modifiers::COMMAND;
        input.events = vec![egui::Event::Paste("a\tb".into()), egui::Event::Cut];
        assert!(presses(&mut input, Layout::Omarchy, "ctrl+x").fresh);
        assert!(presses(&mut input, Layout::Omarchy, "ctrl+v").fresh);
        assert!(input.events.is_empty());
        // The Mac's copy, with Cmd.
        input.modifiers = Modifiers::MAC_CMD | Modifiers::COMMAND;
        input.events = vec![egui::Event::Copy];
        assert!(!presses(&mut input, Layout::Mac, "cmd+shift+c").fresh);
        assert!(presses(&mut input, Layout::Mac, "cmd+c").fresh);
        // A copy with nothing held (a Copy key, a menu's Copy) is no chord.
        input.modifiers = Modifiers::NONE;
        input.events = vec![egui::Event::Copy];
        assert!(!presses(&mut input, Layout::Omarchy, "ctrl+c").fresh);
        assert!(!presses(&mut input, Layout::Mac, "cmd+c").fresh);
        assert_eq!(input.events, vec![egui::Event::Copy]);
    }

    #[test]
    fn a_digit_is_read_by_the_key_it_is_on_when_shift_changes_what_it_types() {
        let keymap = Keymap::default();
        let held = Modifiers::CTRL | Modifiers::COMMAND | Modifiers::SHIFT;
        let mut input = egui::InputState::default();
        let window = Command::GoToConnectionWindow;
        let asked = |input: &mut egui::InputState, layout| {
            digit_asked(input, &keymap, layout, window, Scope::Global, true)
        };
        // Ctrl+Shift+1 on a US keyboard: the key is `!`, on the key of 1.
        input.events = vec![egui::Event::Key {
            key: Key::Exclamationmark,
            physical_key: Some(Key::Num1),
            pressed: true,
            repeat: false,
            modifiers: held,
        }];
        assert_eq!(asked(&mut input, Layout::Omarchy), Some(0));
        assert!(input.events.is_empty());
        // The Mac's is Cmd and the digit, and no other layout's chord is.
        let cmd = Modifiers::MAC_CMD | Modifiers::COMMAND;
        input.events = vec![crate::testing::key(Key::Num3, cmd)];
        assert_eq!(asked(&mut input, Layout::Omarchy), None);
        assert_eq!(asked(&mut input, Layout::Mac), Some(2));
    }

    #[test]
    fn a_frames_typing_is_its_text_or_else_its_bare_keys() {
        let ctx = egui::Context::default();
        let typed = |events: Vec<egui::Event>| {
            let input = egui::RawInput {
                events,
                ..Default::default()
            };
            let mut typed = Vec::new();
            ctx.run_ui(input, |ui| typed = typed_chars(ui.ctx()))
                .textures_delta
                .clear();
            typed
        };
        let key = |key, held| crate::testing::key(key, held);
        // The real keyboard: a key and its text. The text is what counts.
        let both = vec![key(Key::R, Modifiers::SHIFT), egui::Event::Text("R".into())];
        assert_eq!(typed(both), ['R']);
        // A test's bare key: the character it types on a US keyboard.
        assert_eq!(typed(vec![key(Key::J, Modifiers::NONE)]), ['j']);
        assert_eq!(typed(vec![key(Key::R, Modifiers::SHIFT)]), ['R']);
        // A chord types nothing.
        let ctrl = Modifiers::CTRL | Modifiers::COMMAND;
        assert!(typed(vec![key(Key::R, ctrl)]).is_empty());
        assert!(typed(vec![key(Key::Num2, Modifiers::ALT)]).is_empty());
        assert!(typed(vec![key(Key::Enter, Modifiers::NONE)]).is_empty());
    }
}

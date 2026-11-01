//! The row panel as a row form: what editing adds to the panel for the row
//! of a table that can be edited. Which of its fields can be edited, the
//! editor the panel draws in a field's place, and what stands there while
//! the grid edits that cell. The panel itself (`row_panel`) draws the
//! fields; the reducer owns every change (`app/editing.rs`).

use egui::{Sense, WidgetInfo, WidgetType, vec2};

use crate::edit::{Editor, EditorPlace, Lock, Table};
use crate::i18n::{Locale, gettext};
use crate::model::{Action, CellPos, ConnTabId, ObjectTab, TabId, Workspace};
use crate::theme::{Look, Palette};
use crate::typography::{Text, TextRole};
use crate::ui::cell_editor::{self, Target};
use crate::ui::widgets;

/// What the form makes of one field of the row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Part {
    /// Nothing: the field is read, as in a panel that edits nothing. A SQL
    /// editor's result, and a row no cell of which can be edited.
    Read,
    /// Its value can be edited: a click on it opens the editor, or on its
    /// pencil where the value has clicks of its own.
    Editable,
    /// It cannot, for a reason of its own.
    Locked(Lock),
    /// The grid is editing its cell.
    InGrid,
    /// The panel's editor is on it.
    Editing,
}

/// The form of the row the panel shows.
pub struct Form<'a> {
    /// The tab's editor, where the panel draws it: on a cell of this row.
    editor: Option<&'a mut Editor>,
    /// That editor's cell, as its field needs it.
    target: Option<Target>,
    /// One per column of the page. Empty where the form has no part in
    /// the panel.
    parts: Vec<Part>,
    /// Why no field of the row can be edited: what the panel's footer
    /// says, once, for a row whose fields the form leaves as they are.
    pub locked: Option<Lock>,
    /// The column whose edit was asked for in the panel and refused: its
    /// field says why under its value. The terminal look says it in its
    /// mode line.
    refused: Option<(usize, Lock)>,
    /// The column whose field gets the keyboard back, until that field
    /// takes it.
    pub focus: Option<usize>,
    /// What ends the open field (a commit, a cancel, the keyboard going
    /// elsewhere). The panel queues it ahead of everything else it asks
    /// for in the frame: its field can be left for a control of the panel
    /// that is drawn before it, which asks for another editor on the cell
    /// this one is leaving.
    pub ending: Vec<Action>,
}

/// Takes the tab's editor out of the tab for the frame, where the panel
/// draws it: its field edits the text while the panel reads the rest of
/// the workspace. [`put_editor`] gives it back when the panel is drawn.
pub fn take_editor(object: &mut ObjectTab) -> Option<Editor> {
    object
        .edits
        .editor
        .take_if(|editor| editor.place == EditorPlace::Panel)
}

/// Gives back what [`take_editor`] took.
pub fn put_editor(object: &mut ObjectTab, editor: Option<Editor>) {
    if let Some(editor) = editor {
        object.edits.editor = Some(editor);
    }
}

impl<'a> Form<'a> {
    /// A panel the form has no part in: a SQL editor's result.
    pub fn none() -> Self {
        Self {
            editor: None,
            target: None,
            parts: Vec::new(),
            locked: None,
            refused: None,
            focus: None,
            ending: Vec::new(),
        }
    }

    /// The form of the page's row `row` of the table `object` shows.
    /// `editor` is the panel's, taken out of the tab; `hold` says no dialog
    /// is up, so an open field has the keyboard; `terminal` is the look,
    /// which says a refused edit's reason in its mode line.
    pub fn of(
        workspace: &Workspace,
        object: &ObjectTab,
        tab: ConnTabId,
        row: usize,
        editor: Option<&'a mut Editor>,
        (hold, terminal, keys, calendar): (
            bool,
            bool,
            &crate::ui::cell_editor::Keys,
            Option<&crate::ui::cell_editor::Calendar>,
        ),
    ) -> Self {
        // Only an editor on this row is the panel's to draw.
        let editor = editor.filter(|editor| editor.cell.row == row);
        let Some(table) = Table::of(workspace, object) else {
            return Self::none();
        };
        // No cell of the row can be edited: the fields are read as ever,
        // and the footer says why, once.
        if let Some(lock) = table.row_lock(row) {
            return Self {
                locked: Some(lock),
                ..Self::none()
            };
        }
        // The row's lock and the table's key are found once for the row:
        // each field is then asked only for what is its own.
        let key = table.key().unwrap_or_default();
        // What an edit asked for in the panel was refused for, where it
        // is one of this row's cells.
        let refused = object
            .edits
            .why
            .filter(|(cell, _)| cell.row == row)
            .filter(|_| object.edits.why_place == EditorPlace::Panel && !terminal)
            .map(|(cell, lock)| (cell.col, lock));
        let editing = editor.as_ref().map(|editor| editor.cell.col);
        // The editor still in the tab is the grid's.
        let in_grid = object
            .edits
            .editor
            .as_ref()
            .filter(|editor| editor.cell.row == row)
            .map(|editor| editor.cell.col);
        let parts = (0..table.page.columns.len())
            .map(|col| {
                if editing == Some(col) {
                    Part::Editing
                } else if in_grid == Some(col) {
                    Part::InGrid
                } else {
                    match table.own_lock(CellPos { row, col }, &key) {
                        Some(lock) => Part::Locked(lock),
                        None => Part::Editable,
                    }
                }
            })
            .collect();
        let target = editor.as_ref().and_then(|editor| {
            let how = (hold, keys, calendar);
            crate::ui::data_view::editor_target(workspace, object, tab, editor, how)
        });
        Self {
            editor,
            target,
            parts,
            locked: None,
            refused,
            focus: None,
            ending: Vec::new(),
        }
    }

    /// What the form makes of the field of the column `col`.
    pub fn part(&self, col: usize) -> Part {
        self.parts.get(col).copied().unwrap_or(Part::Read)
    }

    /// Whether the panel's open editor is one whose text can be laid out:
    /// the large editor of a JSON column. A text column that holds a
    /// document is edited as the text it is.
    pub fn formats(&self) -> bool {
        let large = self.editor.as_ref().is_some_and(|editor| editor.large);
        large && self.target.as_ref().is_some_and(|target| target.json)
    }

    /// Whether an edit of the column `col` was asked for in the panel and
    /// refused: its field says why.
    pub fn refused(&self, col: usize) -> bool {
        self.refusal(col).is_some()
    }

    /// Why the edit asked for on the field of the column `col` was refused,
    /// where one was.
    pub fn refusal(&self, col: usize) -> Option<Lock> {
        self.refused
            .filter(|(refused, _)| *refused == col)
            .map(|(_, lock)| lock)
    }

    /// Draws the panel's editor in its field's place, its text in `role`,
    /// and queues what its frame came to. Returns whether there was an
    /// editor to draw.
    pub fn editing(
        &mut self,
        ui: &mut egui::Ui,
        (tab, id): (ConnTabId, TabId),
        role: TextRole,
        skin: (&Look, &Palette, Locale),
    ) -> bool {
        let (Some(editor), Some(target)) = (self.editor.as_deref_mut(), &self.target) else {
            return false;
        };
        let cell = editor.cell;
        // In the box its value showed under the pointer: the text stays
        // where it was read.
        let place = super::row_panel::field_box(skin.0);
        let mut outcome = if editor.large {
            // A value of several lines, a long one or a document: the
            // large editor, in the field's place.
            cell_editor::tall(ui, editor, target, place.1, skin)
        } else {
            cell_editor::in_panel(ui, editor, target, (role, place), skin)
        };
        // A foreign key field's list, under the field. A press on it is
        // the editor's own: the field keeps the keyboard, and the frame
        // does not read as an edit that was left.
        let mut pick = None;
        if !editor.large
            && let Some(picker) = &editor.picker
            && let (Some(anchor), Some(search)) = (outcome.rect, &target.search)
            // Only while the field is in sight, its top edge clear of the
            // panel's view: the list hangs from the part that shows, as a
            // cell's does from the grid's body.
            && let Some(anchor) = crate::ui::key_picker::hung_from(anchor, ui.clip_rect())
        {
            let names = crate::ui::key_picker::Names {
                table: &picker.target.name,
                nullable: search.nullable,
                chooser: search.chooser.as_ref(),
            };
            let shown = crate::ui::key_picker::show(
                ui,
                picker,
                &editor.text,
                anchor,
                target.id,
                &names,
                skin,
            );
            if shown.pressed {
                ui.memory_mut(|memory| memory.request_focus(target.id));
                outcome.left = false;
                outcome.kept = false;
            }
            if let Some(index) = shown.picked {
                pick = Some(Action::PickOption { tab, id, index });
            } else if shown.null {
                pick = Some(Action::PickerNull { tab, id });
            }
            // What was asked of the label's chooser ends no edit.
            if let Some(asked) = shown.label {
                self.ending.push(asked.action(tab, id));
            }
        }
        // A date or time field's line, under the field while it is in
        // sight.
        if !editor.large
            && let Some(anchor) = outcome.rect
            && let Some(anchor) = crate::ui::key_picker::hung_from(anchor, ui.clip_rect())
        {
            crate::ui::date_line::under(ui, anchor, editor, target, &mut outcome, skin);
        }
        // The text is noted as typed before anything ends the edit, as for
        // a cell's field.
        if outcome.changed {
            self.ending.push(Action::EditorTyped { tab, id });
        }
        // A pick ends the edit: the list was pressed, so nothing in this
        // frame reads as the edit left, and the pick goes first.
        if let Some(pick) = pick {
            self.ending.push(pick);
        } else if outcome.now {
            self.ending.push(Action::SetNow { tab, id });
        } else if outcome.large {
            self.ending.push(Action::EditorBreak { tab, id });
        } else if let Some(then) = outcome.commit {
            self.ending.push(Action::CommitEdit { tab, id, then });
        } else if outcome.cancel {
            self.ending.push(Action::CancelEdit { tab, id });
        } else if outcome.left {
            let place = EditorPlace::Panel;
            self.ending.push(Action::LeaveEdit {
                tab,
                id,
                cell,
                place,
            });
            // The terminal look's Esc left insert mode: the keyboard is
            // on the field again, as after a commit.
            if outcome.kept {
                let from = cell.col;
                let by = 0;
                self.ending.push(Action::MoveField { tab, id, from, by });
            }
        }
        true
    }
}

/// What stands in a value's place while the grid edits its cell: a dashed
/// box that says so. One editor is open at a time, and it is there.
pub fn in_grid(ui: &mut egui::Ui, look: &Look, palette: &Palette, locale: Locale) {
    let height = if look.terminal { 26.0 } else { 32.0 };
    let size = vec2(ui.available_width(), height);
    let (rect, response) = ui.allocate_exact_size(size, Sense::hover());
    let text = look.label(&gettext(locale, "Editing in the grid…"));
    response.widget_info(|| WidgetInfo::labeled(WidgetType::Label, true, &text));
    // The design's dashes are the accent drawn towards the panel: the
    // words are what is read, not the box.
    let dashes = palette.window.lerp_to_gamma(palette.accent, 0.45);
    super::row_panel::dashed(ui, rect, dashes);
    widgets::paint_text(
        ui,
        rect.left() + 10.0,
        rect.center().y,
        Text::one(look, widgets::secondary(look), &text, palette.accent),
    );
}

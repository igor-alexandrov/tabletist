//! The two questions about pending changes: before they are dropped, and
//! before they are saved to production. A stand-in until both are drawn as
//! the design has them: what is asked, what is at stake and the answers.
//! The confirmation lists every statement it would send, in any form: no
//! save to production is offered without them on screen.

use crate::app::App;
use crate::i18n::{gettext, ngettext};
use crate::model::{Action, Dialog};
use crate::typography::Text;
use crate::ui::widgets;

/// The tallest the confirmation's statements stand before they scroll, so
/// the buttons under them stay on screen.
const STATEMENTS_HEIGHT: f32 = 240.0;

pub fn show(app: &mut App, ctx: &egui::Context) {
    leave(app, ctx);
    confirm_write(app, ctx);
}

/// `count` with its noun: "1 change", "3 changes".
fn counted(
    locale: crate::i18n::Locale,
    count: usize,
    one: &'static str,
    many: &'static str,
) -> String {
    let plural = u32::try_from(count).unwrap_or(u32::MAX);
    format!("{count} {}", ngettext(locale, one, many, plural))
}

/// Asks before pending changes are dropped. Enter answers nothing here: it
/// never discards.
fn leave(app: &mut App, ctx: &egui::Context) {
    let locale = app.locale;
    let palette = app.palette;
    let look = app.look;
    let Some(Dialog::Leave(prompt)) = &app.dialog else {
        return;
    };
    let mut actions = Vec::new();
    let modal = widgets::modal(egui::Id::new("leave-prompt"), &look, &palette).show(ctx, |ui| {
        ui.set_width(420.0);
        let title = if prompt.can_save {
            gettext(locale, "Save the pending changes first?")
        } else {
            gettext(locale, "Discard the pending changes?")
        };
        widgets::label(
            ui,
            widgets::dialog_title(&look),
            &title,
            palette.text,
            &look,
        );
        ui.add_space(4.0);
        let message = format!(
            "{} {}",
            counted(locale, prompt.changes, "change", "changes"),
            ngettext(
                locale,
                "has not been saved, and going on drops it.",
                "have not been saved, and going on drops them.",
                u32::try_from(prompt.changes).unwrap_or(u32::MAX),
            )
        );
        let width = ui.available_width();
        Text::one(&look, widgets::body(&look), &message, palette.secondary)
            .wrap(width)
            .layout(ui.ctx())
            .label(ui);
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if prompt.can_save
                    && widgets::primary_button(ui, &gettext(locale, "Save"), &look, &palette)
                        .clicked()
                {
                    actions.push(Action::LeaveSave);
                }
                if widgets::button(ui, &gettext(locale, "Discard"), &look).clicked() {
                    actions.push(Action::LeaveDiscard);
                }
                if widgets::button(ui, &gettext(locale, "Cancel"), &look).clicked() {
                    actions.push(Action::LeaveStay);
                }
            });
        });
    });
    if modal.is_top_modal
        && ctx.input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::Escape))
    {
        actions.push(Action::LeaveStay);
    }
    app.actions.extend(actions);
}

/// Asks before a save to production, with every statement it would send.
fn confirm_write(app: &mut App, ctx: &egui::Context) {
    let locale = app.locale;
    let palette = app.palette;
    let look = app.look;
    let Some(Dialog::ConfirmWrite(prompt)) = &app.dialog else {
        return;
    };
    let mut actions = Vec::new();
    let modal = widgets::modal(egui::Id::new("write-prompt"), &look, &palette).show(ctx, |ui| {
        ui.set_width(520.0);
        widgets::label(
            ui,
            widgets::dialog_title(&look),
            &gettext(locale, "Save to production?"),
            palette.text,
            &look,
        );
        ui.add_space(4.0);
        let message = format!(
            "{} · {}",
            counted(locale, prompt.changes, "change", "changes"),
            counted(locale, prompt.rows, "row", "rows"),
        );
        widgets::label(ui, widgets::body(&look), &message, palette.secondary, &look);
        ui.add_space(6.0);
        egui::ScrollArea::vertical()
            .max_height(STATEMENTS_HEIGHT)
            .show(ui, |ui| {
                let width = ui.available_width();
                for statement in &prompt.statements {
                    let laid = Text::one(&look, widgets::code(&look), statement, palette.text)
                        .wrap(width)
                        .layout(ui.ctx());
                    ui.add(egui::Label::new(laid.galley).selectable(true));
                }
            });
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let save = gettext(locale, "Save to production");
                if widgets::primary_button(ui, &save, &look, &palette).clicked() {
                    actions.push(Action::ConfirmWrite);
                }
                if widgets::button(ui, &gettext(locale, "Cancel"), &look).clicked() {
                    actions.push(Action::CancelWrite);
                }
            });
        });
    });
    if modal.is_top_modal
        && ctx.input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::Escape))
    {
        actions.push(Action::CancelWrite);
    }
    app.actions.extend(actions);
}

#[cfg(test)]
mod tests {
    use crate::backend::Command;
    use crate::model::{Action, CellPos, ConnTabId, Dialog, EditStart, TabId};
    use crate::testing::Harness;

    /// Makes `text` the pending value of column `col` in row `row`, as an
    /// editor that was typed into and left does.
    fn change(harness: &mut Harness, at: (ConnTabId, TabId), row: usize, col: usize, text: &str) {
        let (tab, id) = at;
        harness.app.apply(Action::EditCell {
            tab,
            id,
            cell: CellPos { row, col },
            start: EditStart::Replace(text.into()),
        });
        harness.app.apply(Action::LeaveEdit { tab, id });
    }

    fn writes(harness: &Harness) -> usize {
        let sent = harness.app.backend.sent.iter();
        sent.filter(|command| matches!(command, Command::Write { .. }))
            .count()
    }

    /// The buttons named `label` that can be pressed, highest first. A
    /// dialog's stand above the pending bar, which has a Save of its own.
    fn buttons(harness: &mut Harness, label: &str) -> Vec<egui::accesskit::NodeId> {
        let tree = harness.settle();
        let mut found: Vec<_> = tree
            .nodes
            .iter()
            .filter(|(_, node)| {
                node.role() == egui::accesskit::Role::Button
                    && node.label() == Some(label)
                    && !node.is_disabled()
            })
            .filter_map(|(id, node)| Some((*id, node.bounds()?.y0)))
            .collect();
        found.sort_by(|(_, a), (_, b)| a.total_cmp(b));
        found.into_iter().map(|(id, _)| id).collect()
    }

    /// Presses the dialog's button `label`.
    fn click_dialog(harness: &mut Harness, label: &str) {
        let target = *buttons(harness, label)
            .first()
            .unwrap_or_else(|| panic!("no button {label}"));
        harness.frame(vec![egui::Event::AccessKitActionRequest(
            egui::accesskit::ActionRequest {
                target_tree: egui::accesskit::TreeId::ROOT,
                target_node: target,
                action: egui::accesskit::Action::Click,
                data: None,
            },
        )]);
        harness.settle();
    }

    fn pending(harness: &Harness, (tab, id): (ConnTabId, TabId)) -> usize {
        let workspace = harness.app.workspace(tab).unwrap();
        workspace.object_tab(id).unwrap().edits.cells.len()
    }

    #[test]
    fn the_production_confirmation_lists_every_statement_and_sends_only_when_confirmed() {
        let mut harness = Harness::new();
        let (tab, id) = harness.editable();
        harness.app.workspace_mut(tab).unwrap().environment = crate::env::Environment::Production;
        change(&mut harness, (tab, id), 1, 1, "bob@example.com");
        change(&mut harness, (tab, id), 3, 1, "dan@example.com");
        harness.app.apply(Action::WriteEdits { tab, id });
        harness.settle();
        let statements = match &harness.app.dialog {
            Some(Dialog::ConfirmWrite(prompt)) => prompt.statements.clone(),
            other => panic!("expected the confirmation, got {other:?}"),
        };
        assert_eq!(statements.len(), 2);
        for statement in &statements {
            assert!(statement.starts_with("UPDATE"), "{statement}");
            assert!(
                harness.painted.iter().any(|(text, _)| text == statement),
                "{statement} is on screen"
            );
        }
        assert_eq!(writes(&harness), 0, "nothing is sent before the answer");
        harness.click("Cancel");
        assert!(harness.app.dialog.is_none());
        assert_eq!((writes(&harness), pending(&harness, (tab, id))), (0, 2));
        // Esc cancels too.
        harness.app.apply(Action::WriteEdits { tab, id });
        harness.press(egui::Key::Escape, egui::Modifiers::NONE);
        assert!(harness.app.dialog.is_none());
        assert_eq!((writes(&harness), pending(&harness, (tab, id))), (0, 2));
        harness.app.apply(Action::WriteEdits { tab, id });
        harness.click("Save to production");
        assert!(harness.app.dialog.is_none());
        assert_eq!(writes(&harness), 1);
    }

    #[test]
    fn the_leave_prompt_offers_save_only_where_a_save_can_run() {
        let mut harness = Harness::new();
        let (tab, id) = harness.editable();
        change(&mut harness, (tab, id), 1, 1, "bob@example.com");
        harness.app.apply(Action::CloseTab { tab, id });
        assert!(harness.has("Discard") && harness.has("Cancel"));
        // The dialog's Save, and the pending bar's behind it.
        assert_eq!(buttons(&mut harness, "Save").len(), 2);
        click_dialog(&mut harness, "Save");
        assert!(harness.app.dialog.is_none());
        assert_eq!(writes(&harness), 1);
        // A value to fix disables the save, and the prompt has no Save.
        let mut harness = Harness::new();
        let (tab, id) = harness.editable();
        change(&mut harness, (tab, id), 1, 2, "{oops");
        harness.app.apply(Action::CloseTab { tab, id });
        assert!(matches!(harness.app.dialog, Some(Dialog::Leave(_))));
        // No Save in the dialog, and the bar's cannot be pressed.
        assert!(buttons(&mut harness, "Save").is_empty());
        assert!(harness.has("Discard") && harness.has("Cancel"));
        // Enter never discards.
        harness.press(egui::Key::Enter, egui::Modifiers::NONE);
        assert!(matches!(harness.app.dialog, Some(Dialog::Leave(_))));
        assert_eq!(pending(&harness, (tab, id)), 1);
    }
}

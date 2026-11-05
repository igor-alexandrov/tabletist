//! The terminal look's `:export cancel` with two or more exports running
//! (spec 4, F5): a picker of them over the status line's right end, drawn
//! as the Copy as menu is. A digit or Enter cancels the one picked; Esc
//! closes it. The exports are the app's: this draws them and asks.

use egui::{Align2, Area, Id, Order, Ui, vec2};

use crate::export_sheet::Exporting;
use crate::i18n::gettext;
use crate::keymap::{Command, Layout, Scope};
use crate::model::{Action, CancelExportPicker};
use crate::typography::{Text, TextRole};
use crate::ui::{Paint, menu, widgets};

/// What a pick asks: the picker closes, and the export is cancelled.
fn cancel(exporting: &Exporting) -> Vec<Action> {
    vec![
        Action::CloseDialog,
        Action::CancelExport(Some(exporting.job.id)),
    ]
}

/// The picker's keys, before any view reads them: a digit picks its row,
/// `ctrl+n` and `ctrl+p` (↓ ↑) move, Enter picks the lit one, Esc closes.
pub fn keys(
    ctx: &egui::Context,
    paint: Paint,
    picker: &CancelExportPicker,
    rows: &[&Exporting],
) -> Vec<Action> {
    let (keymap, layout) = (crate::ui::keys::published(ctx), Layout::of(&paint.look));
    if rows.is_empty() {
        return Vec::new();
    }
    let lit = picker.lit.min(rows.len() - 1);
    let asked = |command: Command| {
        ctx.input_mut(|input| {
            crate::ui::keys::asked(input, &keymap, layout, command, Scope::Prompt, true).count
        })
    };
    let digit = ctx.input_mut(|input| {
        crate::ui::keys::digit_asked(
            input,
            &keymap,
            layout,
            Command::CancelWhichExport,
            Scope::Prompt,
            true,
        )
    });
    let step =
        asked(Command::CancelExportNext) as isize - asked(Command::CancelExportPrevious) as isize;
    if let Some(digit) = digit {
        return rows.get(digit).map(|row| cancel(row)).unwrap_or_default();
    }
    if asked(Command::CancelExportPick) > 0 {
        return cancel(rows[lit]);
    }
    if asked(Command::CloseCancelExport) > 0 {
        return vec![Action::CloseDialog];
    }
    if step != 0 {
        return vec![Action::MoveInCancelExport(step)];
    }
    Vec::new()
}

/// Draws the picker over the status line's right end, and returns what a
/// click asks.
pub fn show(
    ctx: &egui::Context,
    paint: Paint,
    picker: &CancelExportPicker,
    rows: &[&Exporting],
) -> Vec<Action> {
    let Paint {
        locale,
        palette,
        look,
    } = paint;
    let mut actions = Vec::new();
    if rows.is_empty() {
        return actions;
    }
    let lit = picker.lit.min(rows.len() - 1);
    let title = look.label(&gettext(locale, "Cancel export"));
    let count = gettext(locale, "{n} running").replace("{n}", &rows.len().to_string());
    let area = Area::new(Id::new("cancel-export"))
        .order(Order::Foreground)
        .kind(egui::UiKind::Menu)
        .anchor(Align2::RIGHT_BOTTOM, vec2(-8.0, -36.0))
        .show(ctx, |ui| {
            menu::frame(&look, &palette).show(ui, |ui| {
                ui.spacing_mut().item_spacing = egui::Vec2::ZERO;
                ui.set_min_width(300.0);
                title_line(ui, paint, &title, &count);
                for (index, exporting) in rows.iter().enumerate() {
                    let words = crate::ui::workspace::export_brief(exporting, locale);
                    let shown = format!("{}  {words}", index + 1);
                    let item = menu::Item::action(&shown)
                        .name(&words)
                        .kept_out(index == lit);
                    if item.show(ui, &look, &palette).clicked() {
                        actions.extend(cancel(exporting));
                    }
                }
                foot_line(ui, paint);
            });
            let place = ui.min_rect();
            widgets::announce(ui, place, &title);
        });
    if area.response.clicked_elsewhere() && actions.is_empty() {
        actions.push(Action::CloseDialog);
    }
    actions
}

/// "cancel export", then how many run.
fn title_line(ui: &mut Ui, paint: Paint, title: &str, count: &str) {
    let Paint { palette, look, .. } = paint;
    let role = TextRole::OBody;
    let height = role.row_height(ui.ctx(), look.faces) + 2.0;
    let (rect, _) =
        ui.allocate_exact_size(vec2(ui.available_width(), height), egui::Sense::hover());
    let y = rect.center().y;
    let width = widgets::paint_text(
        ui,
        rect.left() + 10.0,
        y,
        Text::one(&look, TextRole::OGroup, title, palette.text),
    );
    widgets::paint_text(
        ui,
        rect.left() + 10.0 + width + 10.0,
        y,
        Text::one(&look, role, count, palette.dim),
    );
    widgets::hline(ui, rect.x_range(), rect.bottom() - 0.5, palette.outline);
}

/// "one key picks · ctrl+n/p enter · esc close".
fn foot_line(ui: &mut Ui, paint: Paint) {
    let Paint {
        locale,
        palette,
        look,
    } = paint;
    let ctx = ui.ctx().clone();
    let moves = crate::ui::keys::written_together(
        &ctx,
        &look,
        &[Command::CancelExportNext, Command::CancelExportPrevious],
    );
    let enter = crate::ui::keys::written(&ctx, &look, Command::CancelExportPick);
    let esc = crate::ui::keys::written(&ctx, &look, Command::CloseCancelExport);
    let picks = look.label(&gettext(locale, "one key picks"));
    let close = look.label(&gettext(locale, "close"));
    let role = widgets::secondary(&look);
    let height = TextRole::OBody.row_height(&ctx, look.faces) + 2.0;
    let (rect, _) =
        ui.allocate_exact_size(vec2(ui.available_width(), height), egui::Sense::hover());
    widgets::hline(ui, rect.x_range(), rect.top() + 0.5, palette.outline);
    let y = rect.center().y;
    let x = rect.left() + 10.0;
    let width = widgets::paint_text(ui, x, y, Text::one(&look, role, &picks, palette.dim));
    let hints: [widgets::Hint<'_>; 2] = [(&moves, enter.as_str(), true), (&esc, &close, true)];
    widgets::key_hints(ui, (x + width + 14.0, y), &hints, 14.0, &look, &palette);
}

//! The names listed under a path line in the terminal look, as a path is
//! typed: the export float's save to line (spec 4, S15; the component
//! board's path line) and a Files connection's folder field and add line
//! (the "Omarchy – Files, path completions" board). The lit name on the
//! selection's fill, the rest muted, in an area of its own over the
//! dialog, as the dialog's rows scroll and clip. The line keeps the
//! keyboard: the dialog reads the list's keys before it draws.

use egui::{Id, Rect, Response, Sense, Ui, vec2};

use crate::export_sheet::PathList;
use crate::theme::{Look, Palette};
use crate::typography::{Text, TextRole};
use crate::ui::widgets;

/// What the list under a path line asks for, this frame.
#[derive(Default)]
pub struct Asked {
    /// The keyboard left the line: the list goes too, until the next key
    /// typed there.
    pub close: bool,
    /// The name clicked, by its place.
    pub taken: Option<usize>,
    /// A press on the list, which takes the keyboard from the line for the
    /// moment of a click: no leaving of the line.
    pub on_list: bool,
}

/// The list under the path line `field`, which holds `line`: `list` while
/// it is the line's, in the area `id`. A click on a name takes it and
/// gives the line the keyboard back.
pub fn under(
    ui: &Ui,
    field: &Response,
    line: &str,
    list: Option<&PathList>,
    id: Id,
    skin: (&Look, &Palette),
) -> Asked {
    let (look, _) = skin;
    let mut asked = Asked::default();
    let height = row_height(ui.ctx(), look);
    // Where the list is drawn (where egui put it, as a small window may
    // move it up): a press on it takes the keyboard from the line for the
    // moment of a click, a pointer only resting there does not keep it.
    let list_rect = list.map(|list| {
        let below = Rect::from_min_size(
            field.rect.left_bottom(),
            vec2(field.rect.width(), height * list.names.len() as f32),
        );
        ui.ctx()
            .memory(|memory| memory.area_rect(id))
            .unwrap_or(below)
    });
    let clicking = ui
        .ctx()
        .input(|input| input.pointer.any_down() || input.pointer.any_released());
    let pointer_on_list = clicking
        && list_rect.is_some_and(|rect| {
            ui.ctx()
                .pointer_latest_pos()
                .is_some_and(|pos| rect.contains(pos))
        });
    asked.on_list = pointer_on_list;
    if !field.has_focus() && !pointer_on_list && list.is_some() {
        asked.close = true;
    }
    let shown = list.filter(|list| list.line == line && !list.names.is_empty());
    if let Some(list) = shown
        && (field.has_focus() || pointer_on_list)
    {
        // Esc and ↑ ↓ are the list's, not egui's to move or drop the
        // keyboard with.
        let filter = egui::EventFilter {
            horizontal_arrows: true,
            vertical_arrows: true,
            escape: true,
            ..Default::default()
        };
        ui.ctx()
            .memory_mut(|memory| memory.set_focus_lock_filter(field.id, filter));
        asked.taken = names(ui.ctx(), id, list, field.rect, skin);
        if asked.taken.is_some() {
            // Back to the line, for a folder's names next.
            ui.ctx().memory_mut(|memory| memory.request_focus(field.id));
        }
    }
    asked
}

/// Gives the line `id` the keyboard, its cursor at the end of `text`, once
/// for each new `asked`: after a name is taken from its list. Nought is
/// never asked, so a dialog opened anew takes no keyboard to the line.
pub fn to_line(ctx: &egui::Context, id: Id, asked: u64, text: &str) {
    let seen = id.with("to-line");
    let new = ctx.data_mut(|data| {
        let before = data.get_temp::<u64>(seen).unwrap_or(0);
        data.insert_temp(seen, asked);
        before != asked
    });
    if !new || asked == 0 {
        return;
    }
    let mut state = egui::text_edit::TextEditState::load(ctx, id).unwrap_or_default();
    let end = egui::text::CCursor::new(text.chars().count());
    state
        .cursor
        .set_char_range(Some(egui::text::CCursorRange::one(end)));
    state.store(ctx, id);
    ctx.memory_mut(|memory| memory.request_focus(id));
}

fn row_height(ctx: &egui::Context, look: &Look) -> f32 {
    TextRole::OBody.row_height(ctx, look.faces) + 4.0
}

/// The names, under `field`: the one clicked, if one was.
fn names(
    ctx: &egui::Context,
    id: Id,
    list: &PathList,
    field: Rect,
    (look, palette): (&Look, &Palette),
) -> Option<usize> {
    let lit = list.lit.min(list.names.len().saturating_sub(1));
    let height = row_height(ctx, look);
    let mut taken = None;
    egui::Area::new(id)
        .order(egui::Order::Tooltip)
        .fixed_pos(field.left_bottom())
        .show(ctx, |ui: &mut Ui| {
            ui.spacing_mut().item_spacing = egui::Vec2::ZERO;
            for (at, listed) in list.names.iter().enumerate() {
                // No keyboard of its own: the line keeps it.
                let (row, response) =
                    ui.allocate_exact_size(vec2(field.width(), height), Sense::CLICK);
                let name = match listed.folder {
                    true => format!("{}/", listed.name),
                    false => listed.name.clone(),
                };
                let ink = match at == lit {
                    true => {
                        ui.painter().rect_filled(row, 0, palette.selection);
                        palette.text
                    }
                    false => {
                        ui.painter().rect_filled(row, 0, palette.panel);
                        palette.dim
                    }
                };
                widgets::paint_text(
                    ui,
                    row.left() + 8.0,
                    row.center().y,
                    Text::one(look, TextRole::OBody, &name, ink),
                );
                response.widget_info(|| {
                    egui::WidgetInfo::selected(
                        egui::WidgetType::SelectableLabel,
                        true,
                        at == lit,
                        &name,
                    )
                });
                if response.clicked() {
                    taken = Some(at);
                }
            }
            let outline = ui.min_rect();
            ui.painter().rect_stroke(
                outline,
                0,
                egui::Stroke::new(1.0, palette.outline),
                egui::StrokeKind::Inside,
            );
        });
    taken
}

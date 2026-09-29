//! Small widgets shared by the views.

use egui::{
    Color32, CornerRadius, Rect, Response, Sense, Stroke, StrokeKind, Ui, WidgetInfo, WidgetType,
    vec2,
};

use crate::theme::{DialogStyle, Icon, Look, Palette, Selection, TabStyle};

/// A single-line text field as tall as a button, so fields and buttons in
/// one row line up.
pub fn single<'a>(ui: &Ui, text: &'a mut String, look: &Look) -> egui::TextEdit<'a> {
    let font = egui::TextStyle::Body.resolve(ui.style());
    single_in(ui, text, look, font)
}

/// [`single`] drawn in `font`.
pub fn single_in<'a>(
    ui: &Ui,
    text: &'a mut String,
    look: &Look,
    font: egui::FontId,
) -> egui::TextEdit<'a> {
    padded(ui, text, look, font, [8, 8])
}

/// A field `look.control_height` tall with `left` and `right` points of
/// padding. egui ignores the height of `TextEdit::min_size`, so the height
/// comes from padding the text above and below, measured for `font`.
fn padded<'a>(
    ui: &Ui,
    text: &'a mut String,
    look: &Look,
    font: egui::FontId,
    [left, right]: [i8; 2],
) -> egui::TextEdit<'a> {
    let line = ui.fonts_mut(|fonts| fonts.row_height(&font)) + ui.spacing().extra_text_line_spacing;
    let padding = (look.control_height - line).max(0.0);
    let top = (padding / 2.0).floor() as i8;
    let bottom = (padding - f32::from(top)).round() as i8;
    egui::TextEdit::singleline(text)
        .font(font)
        .margin(egui::Margin {
            left,
            right,
            top,
            bottom,
        })
        .vertical_align(egui::Align::Center)
}

/// A square icon button. `label` is its accessible name and tooltip, so it
/// is reachable by screen readers and by the headless tests.
pub fn icon_button(
    ui: &mut Ui,
    icon: Icon,
    label: &str,
    look: &Look,
    palette: &Palette,
) -> Response {
    let size = vec2(24.0, 24.0);
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    response.widget_info(|| WidgetInfo::labeled(WidgetType::Button, ui.is_enabled(), label));
    if ui.is_rect_visible(rect) {
        if response.hovered() {
            ui.painter().rect_filled(
                rect,
                egui::CornerRadius::same(look.radius),
                palette.surface_hover,
            );
        }
        let tint = if response.hovered() {
            palette.text
        } else {
            palette.secondary
        };
        icon.image(tint, 16.0).paint_at(
            ui,
            egui::Rect::from_center_size(rect.center(), vec2(16.0, 16.0)),
        );
    }
    response.on_hover_text(label)
}

/// Lays out only the rows that intersect the visible area of the enclosing
/// scroll view. Every row must be exactly `row_height` tall. One extra row on
/// each side stays built so Tab can move focus into it.
pub fn virtual_rows(
    ui: &mut Ui,
    count: usize,
    row_height: f32,
    mut row: impl FnMut(&mut Ui, usize),
) {
    if count == 0 {
        return;
    }
    let previous_spacing = ui.spacing().item_spacing;
    ui.spacing_mut().item_spacing.y = 0.0;
    let clip = ui.clip_rect();
    let start_y = ui.cursor().top();
    let width = ui.available_width();
    let first = (((clip.top() - start_y) / row_height).floor().max(0.0) as usize)
        .min(count)
        .saturating_sub(1);
    let last = (((clip.bottom() - start_y) / row_height).ceil().max(0.0) as usize + 1).min(count);
    if first > 0 {
        ui.allocate_space(vec2(width, first as f32 * row_height));
    }
    for index in first..last {
        row(ui, index);
    }
    if last < count {
        ui.allocate_space(vec2(width, (count - last) as f32 * row_height));
    }
    ui.spacing_mut().item_spacing = previous_spacing;
}

/// Where a row's highlight sits in the row's `rect`: the whole row, or
/// inset for the macOS pill. The sidebar's keyboard cursor follows it.
pub fn selection_rect(rect: Rect, look: &Look) -> Rect {
    match look.selection {
        Selection::Pill => rect.shrink2(vec2(6.0, 1.0)),
        Selection::Tint | Selection::Bar => rect,
    }
}

/// The highlight behind a selected or hovered row: sidebar, picker, quick
/// open. Paints only; the caller allocated `rect`.
pub fn selection(
    ui: &Ui,
    rect: Rect,
    selected: bool,
    hovered: bool,
    look: &Look,
    palette: &Palette,
) {
    let painter = ui.painter();
    let corner = CornerRadius::same(look.radius);
    match look.selection {
        Selection::Tint => {
            if selected {
                painter.rect_filled(rect, corner, palette.accent.gamma_multiply(0.2));
            } else if hovered {
                painter.rect_filled(rect, corner, palette.surface_hover);
            }
        }
        Selection::Pill => {
            let pill = selection_rect(rect, look);
            if selected {
                painter.rect_filled(pill, corner, palette.accent.gamma_multiply(0.22));
            } else if hovered {
                painter.rect_filled(pill, corner, palette.surface_hover);
            }
        }
        Selection::Bar => {
            if selected {
                painter.rect_filled(rect, CornerRadius::ZERO, palette.text.gamma_multiply(0.18));
                let edge = Rect::from_min_size(rect.min, vec2(2.0, rect.height()));
                painter.rect_filled(edge, CornerRadius::ZERO, palette.accent);
            } else if hovered {
                painter.rect_filled(rect, CornerRadius::ZERO, palette.text.gamma_multiply(0.08));
            }
        }
    }
}

/// The text colour of a row: Omarchy marks the selected row in the accent.
pub fn selection_text(selected: bool, look: &Look, palette: &Palette) -> Color32 {
    if selected && look.selection == Selection::Bar {
        palette.accent
    } else {
        palette.text
    }
}

/// A tab's background: connection tabs (`radius` = `look.tab_radius`) and
/// object tabs (`look.radius`).
pub fn tab(
    ui: &Ui,
    rect: Rect,
    active: bool,
    hovered: bool,
    radius: u8,
    look: &Look,
    palette: &Palette,
) {
    let painter = ui.painter();
    let corner = CornerRadius::same(radius);
    match look.tabs {
        TabStyle::Outlined | TabStyle::Raised => {
            if active {
                if look.tabs == TabStyle::Raised {
                    let shadow = egui::epaint::Shadow {
                        offset: [0, 1],
                        blur: 4,
                        spread: 0,
                        color: palette.shadow.gamma_multiply(0.6),
                    };
                    painter.add(shadow.as_shape(rect, corner));
                }
                painter.rect_filled(rect, corner, active_tab_fill(look, palette));
                if look.tabs == TabStyle::Outlined {
                    painter.rect_stroke(
                        rect,
                        corner,
                        Stroke::new(1.0, palette.outline),
                        StrokeKind::Inside,
                    );
                }
            } else if hovered {
                painter.rect_filled(rect, corner, palette.surface_hover);
            }
        }
        TabStyle::Underline => {
            if active {
                painter.rect_filled(rect, CornerRadius::ZERO, palette.window);
                let line =
                    Rect::from_min_max(egui::pos2(rect.left(), rect.bottom() - 2.0), rect.max);
                painter.rect_filled(line, CornerRadius::ZERO, palette.accent);
            } else if hovered {
                painter.rect_filled(rect, CornerRadius::ZERO, palette.text.gamma_multiply(0.08));
            }
        }
    }
}

/// The fill of the active tab: the window colour, so the tab joins the
/// content below it. A raised tab in a dark palette takes the lighter
/// surface instead: the window is darker than the bar there, and a dark tab
/// would read as a slot rather than lifted.
pub fn active_tab_fill(look: &Look, palette: &Palette) -> Color32 {
    if look.tabs == TabStyle::Raised && palette.dark {
        palette.surface
    } else {
        palette.window
    }
}

/// How an Omarchy toggle draws in one state.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ToggleColors {
    pub fill: Color32,
    pub stroke: Stroke,
    pub text: Color32,
}

/// A toggle's colours on Omarchy, from the shell's control states:
/// selected is foreground @ 18% with accent text, hover foreground @ 8%, and
/// keyboard focus adds the 1 px foreground @ 25% border. `None` elsewhere:
/// the other looks keep egui's selected button, tinted with the accent.
pub fn toggle_colors(
    selected: bool,
    hovered: bool,
    focused: bool,
    look: &Look,
    palette: &Palette,
) -> Option<ToggleColors> {
    if !look.bordered_controls {
        return None;
    }
    let fg = palette.text;
    let fill = if selected {
        fg.gamma_multiply(0.18)
    } else if hovered || focused {
        fg.gamma_multiply(0.08)
    } else {
        Color32::TRANSPARENT
    };
    let stroke = if focused {
        Stroke::new(1.0, fg.gamma_multiply(0.25))
    } else {
        Stroke::NONE
    };
    Some(ToggleColors {
        fill,
        stroke,
        text: if selected { palette.accent } else { fg },
    })
}

/// A button in a group where one is selected: the footer's Data/Structure
/// switch and the connection dialog's drivers. Announced as a toggle with
/// its text as the name.
pub fn toggle(ui: &mut Ui, selected: bool, text: &str, look: &Look, palette: &Palette) -> Response {
    // Read last frame's state as egui's Button does, to pick this frame's colours.
    let state = ui.ctx().read_response(ui.next_auto_id());
    let hovered = state.as_ref().is_some_and(Response::hovered);
    let focused = state.as_ref().is_some_and(Response::has_focus);
    let Some(colors) = toggle_colors(selected, hovered, focused, look, palette) else {
        return ui.add(egui::Button::selectable(selected, text));
    };
    ui.add(
        egui::Button::new(egui::RichText::new(text).color(colors.text))
            .selected(selected)
            .fill(colors.fill)
            .stroke(colors.stroke),
    )
}

/// A card: the picker's saved connections.
pub fn card(ui: &Ui, rect: Rect, hovered: bool, look: &Look, palette: &Palette) {
    let painter = ui.painter();
    // Cards round a little more than rows: 6 standard (today), 10 macOS, 0 Omarchy.
    let corner = CornerRadius::same(if look.radius == 0 { 0 } else { look.radius + 2 });
    let fill = if hovered {
        palette.surface_hover
    } else {
        palette.panel
    };
    painter.rect_filled(rect, corner, fill);
    if look.bordered_controls {
        let border = if hovered { 0.40 } else { 0.25 };
        painter.rect_stroke(
            rect,
            corner,
            Stroke::new(1.0, palette.text.gamma_multiply(border)),
            StrokeKind::Inside,
        );
    }
}

/// A search field: a capsule on macOS, square on Omarchy. [`add_search`]
/// draws the magnifier in the room left of the text.
pub fn search_field<'t>(
    ui: &Ui,
    text: &'t mut String,
    hint: &str,
    look: &Look,
) -> egui::TextEdit<'t> {
    let font = egui::TextStyle::Body.resolve(ui.style());
    let right = search_icon_inset(look);
    let left = right + SEARCH_ICON as i8 + 6;
    padded(ui, text, look, font, [left, right]).hint_text(hint)
}

/// The magnifier's size in a search field.
const SEARCH_ICON: f32 = 14.0;

/// The gap left of the magnifier, and right of the text: more in a capsule,
/// whose round end would crowd it.
fn search_icon_inset(look: &Look) -> i8 {
    if look.capsule_search { 11 } else { 8 }
}

/// Where a search field's magnifier sits inside the field's `rect`.
pub fn search_icon_rect(rect: Rect, look: &Look) -> Rect {
    let left = rect.left() + f32::from(search_icon_inset(look));
    Rect::from_center_size(
        egui::pos2(left + SEARCH_ICON / 2.0, rect.center().y),
        vec2(SEARCH_ICON, SEARCH_ICON),
    )
}

/// Adds a search field with its magnifier; macOS rounds it into a capsule.
pub fn add_search(ui: &mut Ui, field: egui::TextEdit<'_>, look: &Look) -> Response {
    let response = add_search_field(ui, field, look);
    if ui.is_rect_visible(response.rect) {
        let tint = ui.visuals().weak_text_color();
        Icon::Search
            .image(tint, SEARCH_ICON)
            .paint_at(ui, search_icon_rect(response.rect, look));
    }
    response
}

fn add_search_field(ui: &mut Ui, field: egui::TextEdit<'_>, look: &Look) -> Response {
    ui.scope(|ui| {
        if look.capsule_search {
            let corner = CornerRadius::same((look.control_height / 2.0) as u8);
            let widgets = &mut ui.visuals_mut().widgets;
            for state in [
                &mut widgets.inactive,
                &mut widgets.hovered,
                &mut widgets.active,
            ] {
                state.corner_radius = corner;
            }
        }
        ui.add(field)
    })
    .inner
}

/// A checkbox whose box stays square, or slightly rounded: the look's
/// control radius would turn the 14 pt box into a circle on macOS, where
/// it reads as a radio button. Borderless looks outline the box so it
/// stands off its background.
pub fn checkbox(
    ui: &mut Ui,
    checked: &mut bool,
    text: &str,
    look: &Look,
    palette: &Palette,
) -> Response {
    ui.scope(|ui| {
        let corner = checkbox_corner(look);
        let stroke = checkbox_stroke(look, palette);
        let widgets = &mut ui.visuals_mut().widgets;
        for state in [
            &mut widgets.inactive,
            &mut widgets.hovered,
            &mut widgets.active,
        ] {
            state.corner_radius = corner;
            if let Some(stroke) = stroke {
                state.bg_stroke = stroke;
            }
        }
        ui.checkbox(checked, text)
    })
    .inner
}

/// The box's outline where controls have no border of their own: the dim
/// text colour, which keeps 3:1 against every surface a box sits on.
fn checkbox_stroke(look: &Look, palette: &Palette) -> Option<Stroke> {
    (!look.bordered_controls).then(|| Stroke::new(1.0, palette.dim))
}

/// A checkbox's corners: the look's radius, at most 4 pt.
fn checkbox_corner(look: &Look) -> CornerRadius {
    CornerRadius::same(look.radius.min(4))
}

/// The primary button's fill: the accent, its hover colour when hovered or
/// focused, and darker while pressed.
pub fn primary_fill(hovered: bool, focused: bool, pressed: bool, palette: &Palette) -> Color32 {
    if pressed {
        palette.accent.lerp_to_gamma(Color32::BLACK, 0.15)
    } else if hovered || focused {
        palette.accent_hover
    } else {
        palette.accent
    }
}

/// The ring round a focused primary button: the full accent, 1 pt off the
/// button so it keeps 3:1 against the background rather than blending into
/// the fill.
pub fn primary_focus_ring(palette: &Palette) -> Stroke {
    Stroke::new(2.0, palette.accent)
}

/// The one accent-filled button in a dialog or view. Its own fill replaces
/// egui's state visuals, so it draws hover, press and keyboard focus itself.
pub fn primary_button(ui: &mut Ui, text: &str, look: &Look, palette: &Palette) -> Response {
    // Last frame's state, read as egui's Button does, picks this frame's fill.
    let state = ui.ctx().read_response(ui.next_auto_id());
    let hovered = state.as_ref().is_some_and(Response::hovered);
    let focused = state.as_ref().is_some_and(Response::has_focus);
    let pressed = state
        .as_ref()
        .is_some_and(Response::is_pointer_button_down_on);
    let fill = primary_fill(hovered, focused, pressed, palette);
    let mut button = egui::Button::new(
        egui::RichText::new(text)
            .font(crate::theme::medium(crate::theme::TEXT))
            .color(palette.on_accent),
    )
    .fill(fill)
    .corner_radius(CornerRadius::same(look.radius))
    .min_size(vec2(0.0, look.control_height));
    if look.bordered_controls {
        // The fill edges itself, not the grey border of other controls.
        button = button.stroke(Stroke::new(1.0, fill));
    }
    let response = ui.add(button);
    if response.has_focus() {
        // One point out, the corners stay concentric (square on Omarchy).
        let corner = if look.radius == 0 { 0 } else { look.radius + 1 };
        ui.painter().rect_stroke(
            response.rect.expand(1.0),
            CornerRadius::same(corner),
            primary_focus_ring(palette),
            StrokeKind::Outside,
        );
    }
    response
}

/// A dialog: a soft shadow over a dimmed window, or Omarchy's accent border
/// over a scrim.
pub fn modal(id: egui::Id, look: &Look, palette: &Palette) -> egui::Modal {
    let frame = egui::Frame::new()
        .fill(palette.overlay)
        .corner_radius(CornerRadius::same(look.dialog_radius))
        .inner_margin(egui::Margin::same(20));
    match look.dialog {
        DialogStyle::Shadow => {
            egui::Modal::new(id)
                .frame(frame.stroke(Stroke::new(1.0, palette.outline)).shadow(
                    egui::epaint::Shadow {
                        offset: [0, 10],
                        blur: 36,
                        spread: 0,
                        color: palette.shadow,
                    },
                ))
                .backdrop_color(Color32::from_black_alpha(if palette.dark {
                    120
                } else {
                    70
                }))
        }
        DialogStyle::AccentBorder => {
            let [r, g, b, _] = palette.window.to_array();
            egui::Modal::new(id)
                .frame(frame.stroke(Stroke::new(2.0, palette.accent)))
                .backdrop_color(Color32::from_rgba_unmultiplied(r, g, b, 128))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::{Rect, pos2, vec2};

    fn run(clip: Rect, f: impl FnMut(&mut egui::Ui)) {
        let ctx = egui::Context::default();
        let mut f = f;
        let mut output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), vec2(400.0, 4000.0))),
                ..Default::default()
            },
            |ui| {
                ui.set_clip_rect(clip);
                f(ui);
            },
        );
        output.textures_delta.clear();
    }

    #[test]
    fn virtual_rows_only_build_visible_rows() {
        let mut built = Vec::new();
        run(
            Rect::from_min_size(pos2(0.0, 0.0), vec2(400.0, 240.0)),
            |ui| {
                virtual_rows(ui, 10_000, 24.0, |ui, index| {
                    built.push(index);
                    ui.allocate_exact_size(vec2(ui.available_width(), 24.0), egui::Sense::hover());
                });
            },
        );
        assert!(!built.is_empty());
        assert!(built.len() < 20, "built {} rows", built.len());
        assert_eq!(built[0], 0);
    }

    #[test]
    fn virtual_rows_keep_the_full_height() {
        let mut span = 0.0;
        run(
            Rect::from_min_max(pos2(0.0, 400.0), pos2(400.0, 600.0)),
            |ui| {
                let start = ui.cursor().top();
                virtual_rows(ui, 1_000, 24.0, |ui, _| {
                    ui.allocate_exact_size(vec2(ui.available_width(), 24.0), egui::Sense::hover());
                });
                span = ui.cursor().top() - start;
            },
        );
        assert!((span - 24_000.0).abs() < 1.0, "span {span}");
    }

    #[test]
    fn the_primary_button_is_announced_by_its_text() {
        let mut harness = crate::testing::Harness::new();
        let look = crate::theme::Look::omarchy();
        let palette = crate::theme::Palette::dark();
        let tree = harness.frame_with(|ui| {
            super::primary_button(ui, "Save & Connect", &look, &palette);
        });
        assert!(
            crate::testing::node(&tree, "Save & Connect", egui::accesskit::Role::Button).is_some()
        );
    }

    #[test]
    fn only_the_pill_is_inset_in_its_row() {
        use crate::theme::Look;
        let row = Rect::from_min_size(pos2(0.0, 0.0), vec2(240.0, 26.0));
        assert_eq!(
            super::selection_rect(row, &Look::macos()),
            row.shrink2(vec2(6.0, 1.0))
        );
        assert_eq!(super::selection_rect(row, &Look::standard()), row);
        assert_eq!(super::selection_rect(row, &Look::omarchy()), row);
    }

    #[test]
    fn primary_buttons_show_hover_focus_and_press() {
        for palette in [
            crate::theme::Palette::light(),
            crate::theme::Palette::dark(),
        ] {
            let fill = |hovered, focused, pressed| {
                super::primary_fill(hovered, focused, pressed, &palette)
            };
            assert_eq!(fill(false, false, false), palette.accent);
            assert_eq!(fill(true, false, false), palette.accent_hover);
            assert_eq!(fill(false, true, false), palette.accent_hover);
            let pressed = fill(true, false, true);
            assert_ne!(pressed, palette.accent);
            assert_ne!(pressed, palette.accent_hover);
            let sum = |c: Color32| u32::from(c.r()) + u32::from(c.g()) + u32::from(c.b());
            assert!(sum(pressed) < sum(palette.accent), "pressing darkens");
            assert_eq!(fill(false, true, true), pressed, "pressing wins");
        }
    }

    /// Whether `shapes` hold a rectangle outlined with `stroke`.
    fn has_ring(shapes: &[egui::epaint::ClippedShape], stroke: Stroke) -> bool {
        shapes.iter().any(|clipped| match &clipped.shape {
            egui::Shape::Rect(rect) => {
                rect.stroke == stroke && rect.stroke_kind == StrokeKind::Outside
            }
            _ => false,
        })
    }

    #[test]
    fn a_focused_primary_button_draws_a_focus_ring() {
        for look in crate::theme::Look::ALL {
            let palette = crate::theme::Palette::dark();
            let ctx = egui::Context::default();
            crate::theme::install(&ctx, false, &look);
            crate::theme::apply(&ctx, &palette, &look);
            let frame = |ctx: &egui::Context| {
                let mut id = egui::Id::NULL;
                let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
                    id = super::primary_button(ui, "Apply", &look, &palette).id;
                });
                output.textures_delta.clear();
                (output.shapes, id)
            };
            let ring = super::primary_focus_ring(&palette);
            assert_eq!(ring.width, 2.0);
            let (shapes, id) = frame(&ctx);
            assert!(!has_ring(&shapes, ring), "{}: no ring unfocused", look.name);
            ctx.memory_mut(|memory| memory.request_focus(id));
            frame(&ctx);
            let (shapes, _) = frame(&ctx);
            assert!(
                has_ring(&shapes, ring),
                "{}: a ring when focused",
                look.name
            );
        }
    }

    #[test]
    fn search_fields_are_as_tall_as_the_looks_controls() {
        for look in crate::theme::Look::ALL {
            let mut harness = crate::testing::Harness::new();
            harness.set_look(look);
            let mut text = String::new();
            let mut height = 0.0;
            harness.frame_with(|ui| {
                let field = super::search_field(ui, &mut text, "Search", &look);
                height = super::add_search(ui, field, &look).rect.height();
            });
            assert_eq!(height, look.control_height, "search field, {}", look.name);
        }
    }

    #[test]
    fn search_fields_show_a_magnifier_the_text_clears() {
        for look in crate::theme::Look::ALL {
            let mut harness = crate::testing::Harness::new();
            harness.set_look(look);
            let mut text = String::from("users");
            let mut field = Rect::NOTHING;
            let tree = harness.frame_with(|ui| {
                let edit = super::search_field(ui, &mut text, "Search", &look);
                field = super::add_search(ui, edit, &look).rect;
            });
            let icon = super::search_icon_rect(field, &look);
            assert!(field.contains_rect(icon), "{}", look.name);
            assert_eq!(icon.center().y, field.center().y, "{}", look.name);
            let run = tree
                .nodes
                .iter()
                .find(|(_, node)| node.role() == egui::accesskit::Role::TextRun)
                .and_then(|(_, node)| node.bounds())
                .expect("the field's text");
            assert!(
                run.x0 as f32 >= icon.right() + 4.0,
                "text at {} overlaps the icon ending at {}, {}",
                run.x0,
                icon.right(),
                look.name
            );
        }
    }

    #[test]
    fn single_line_fields_are_as_tall_as_the_looks_controls() {
        for look in crate::theme::Look::ALL {
            let mut harness = crate::testing::Harness::new();
            harness.set_look(look);
            let mut text = String::new();
            let mut height = 0.0;
            harness.frame_with(|ui| {
                height = ui.add(super::single(ui, &mut text, &look)).rect.height();
            });
            assert_eq!(height, look.control_height, "single field, {}", look.name);
            let mut mono = 0.0;
            harness.frame_with(|ui| {
                let font = crate::theme::mono(crate::theme::TEXT_MONO);
                mono = ui
                    .add(super::single_in(ui, &mut text, &look, font))
                    .rect
                    .height();
            });
            assert_eq!(mono, look.control_height, "monospace field, {}", look.name);
        }
    }

    #[test]
    fn checkboxes_stay_square_enough_and_keep_their_name() {
        for look in crate::theme::Look::ALL {
            let mut harness = crate::testing::Harness::new();
            harness.set_look(look);
            let mut checked = false;
            let mut radius = None;
            let palette = harness.app.palette;
            let tree = harness.frame_with(|ui| {
                super::checkbox(ui, &mut checked, "Raw WHERE", &look, &palette);
                radius = Some(ui.visuals().widgets.inactive.corner_radius);
            });
            assert!(
                crate::testing::node(&tree, "Raw WHERE", egui::accesskit::Role::CheckBox).is_some(),
                "{}",
                look.name
            );
            assert_eq!(
                radius,
                Some(CornerRadius::same(look.radius)),
                "the scope leaves other widgets alone, {}",
                look.name
            );
        }
        let corner = |look| super::checkbox_corner(&look).nw;
        assert_eq!(
            corner(crate::theme::Look::macos()),
            4,
            "a rounded box, not a circle"
        );
        assert_eq!(corner(crate::theme::Look::standard()), 4);
        assert_eq!(corner(crate::theme::Look::omarchy()), 0);
    }

    #[test]
    fn unchecked_boxes_stand_off_their_background() {
        use crate::theme::{Look, Palette, contrast};
        for palette in [Palette::light(), Palette::dark()] {
            for look in [Look::standard(), Look::macos()] {
                let stroke = super::checkbox_stroke(&look, &palette)
                    .unwrap_or_else(|| panic!("{} outlines its boxes", look.name));
                assert_eq!(stroke.width, 1.0);
                for (surface, background) in [
                    ("window", palette.window),
                    ("panel", palette.panel),
                    ("overlay", palette.overlay),
                ] {
                    let ratio = contrast(stroke.color, background);
                    assert!(
                        ratio >= 3.0,
                        "{} box on {surface}, dark {}: {ratio:.2}",
                        look.name,
                        palette.dark
                    );
                }
            }
            assert_eq!(
                super::checkbox_stroke(&Look::omarchy(), &palette),
                None,
                "Omarchy's controls are bordered already"
            );
        }
    }

    #[test]
    fn a_raised_active_tab_stands_above_the_bar_in_dark_palettes() {
        use crate::theme::{Look, Palette};
        let dark = Palette::dark();
        let light = Palette::light();
        assert_eq!(super::active_tab_fill(&Look::macos(), &dark), dark.surface);
        assert_eq!(super::active_tab_fill(&Look::macos(), &light), light.window);
        for look in [Look::standard(), Look::omarchy()] {
            for palette in [dark, light] {
                assert_eq!(
                    super::active_tab_fill(&look, &palette),
                    palette.window,
                    "{}",
                    look.name
                );
            }
        }
        // Tab bars take the panel colour; the raised tab is lighter than it.
        let sum = |c: Color32| u32::from(c.r()) + u32::from(c.g()) + u32::from(c.b());
        assert!(sum(dark.surface) > sum(dark.panel));
    }

    #[test]
    fn omarchy_toggles_use_the_shells_selected_state() {
        use crate::theme::{Look, Palette};
        for palette in [Palette::dark(), Palette::light()] {
            let look = Look::omarchy();
            let colors = |selected, hovered, focused| {
                super::toggle_colors(selected, hovered, focused, &look, &palette).unwrap()
            };
            let selected = colors(true, false, false);
            assert_eq!(selected.fill, palette.text.gamma_multiply(0.18));
            assert_eq!(selected.stroke, Stroke::NONE);
            assert_eq!(selected.text, palette.accent);
            assert_eq!(colors(true, true, false).fill, selected.fill);
            let hovered = colors(false, true, false);
            assert_eq!(hovered.fill, palette.text.gamma_multiply(0.08));
            assert_eq!(hovered.text, palette.text);
            let idle = colors(false, false, false);
            assert_eq!(idle.fill, Color32::TRANSPARENT);
            assert_eq!(idle.stroke, Stroke::NONE);
            // Keyboard focus shows the hover fill and a border, as the shell does.
            let focused = colors(false, false, true);
            assert_eq!(focused.fill, palette.text.gamma_multiply(0.08));
            assert_eq!(
                focused.stroke,
                Stroke::new(1.0, palette.text.gamma_multiply(0.25))
            );
            assert_eq!(colors(true, false, true).fill, selected.fill);
            assert_ne!(colors(true, false, true).stroke, Stroke::NONE);
        }
        for look in [Look::standard(), Look::macos()] {
            assert_eq!(
                super::toggle_colors(true, false, false, &look, &Palette::dark()),
                None,
                "{} keeps egui's selected button",
                look.name
            );
        }
    }

    #[test]
    fn toggles_keep_their_name_and_state_in_every_look() {
        for look in crate::theme::Look::ALL {
            let mut harness = crate::testing::Harness::new();
            harness.set_look(look);
            let palette = harness.app.palette;
            let tree = harness.frame_with(|ui| {
                super::toggle(ui, true, "Data", &look, &palette);
                super::toggle(ui, false, "Structure", &look, &palette);
            });
            let state = |label| {
                let id = crate::testing::node(&tree, label, egui::accesskit::Role::Button)
                    .unwrap_or_else(|| panic!("{label}, {}", look.name));
                let (_, node) = tree.nodes.iter().find(|(n, _)| *n == id).unwrap();
                node.toggled()
            };
            assert_eq!(state("Data"), Some(egui::accesskit::Toggled::True));
            assert_eq!(state("Structure"), Some(egui::accesskit::Toggled::False));
        }
    }

    #[test]
    fn omarchy_dialogs_use_a_scrim_and_an_accent_border() {
        use crate::theme::{Look, Palette};
        for palette in [Palette::dark(), Palette::light()] {
            let modal = super::modal(egui::Id::new("t"), &Look::omarchy(), &palette);
            assert_eq!(modal.backdrop_color.a(), 128);
            let frame = modal.frame.expect("a frame");
            assert_eq!(frame.stroke, Stroke::new(2.0, palette.accent));
            assert_eq!(frame.corner_radius, CornerRadius::ZERO);
            assert_eq!(frame.shadow, egui::epaint::Shadow::NONE);

            let shadowed = super::modal(egui::Id::new("t"), &Look::macos(), &palette);
            let alpha = if palette.dark { 120 } else { 70 };
            assert_eq!(shadowed.backdrop_color.a(), alpha);
            let frame = shadowed.frame.expect("a frame");
            assert_eq!(frame.stroke, Stroke::new(1.0, palette.outline));
            assert_eq!(frame.corner_radius, CornerRadius::same(12));
            assert_ne!(frame.shadow, egui::epaint::Shadow::NONE);
            assert_eq!(frame.shadow.color, palette.shadow);
        }
    }
}

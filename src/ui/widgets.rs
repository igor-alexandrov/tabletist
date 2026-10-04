//! Small widgets shared by the views.

use egui::{
    Color32, CornerRadius, Rect, Response, Sense, Stroke, StrokeKind, Ui, WidgetInfo, WidgetType,
    vec2,
};

use crate::theme::{DialogStyle, Icon, Look, Palette, Selection, TabStyle};
use crate::typography::{Text, TextRole};
use crate::ui::focus::{self, Ring};

/// The body role in `look`.
pub fn body(look: &Look) -> TextRole {
    TextRole::pick(look, TextRole::UiBody, TextRole::OBody)
}

/// The secondary role in `look`: counts, subtitles, status text.
pub fn secondary(look: &Look) -> TextRole {
    TextRole::pick(look, TextRole::Secondary, TextRole::OSecondary)
}

/// A single-line text field as tall as a button, so fields and buttons in
/// one row line up.
pub fn single<'a>(ui: &Ui, text: &'a mut String, look: &Look) -> egui::TextEdit<'a> {
    single_in(ui, text, look, body(look))
}

/// [`single`] drawn in `role`.
pub fn single_in<'a>(
    ui: &Ui,
    text: &'a mut String,
    look: &Look,
    role: TextRole,
) -> egui::TextEdit<'a> {
    field(ui, text, look, role, look.control_height, 8)
}

/// A single-line field `height` tall in `role`, with `pad` points at each
/// side of the text. The height is met to within half a point, and is
/// never less than one line of text.
pub fn field<'a>(
    ui: &Ui,
    text: &'a mut String,
    look: &Look,
    role: TextRole,
    height: f32,
    pad: i8,
) -> egui::TextEdit<'a> {
    padded(ui, text, height, role.font_id(look.faces), [pad, pad])
}

/// A field `height` tall with `left` and `right` points of padding. egui
/// ignores the height of `TextEdit::min_size`, so the height comes from
/// padding the text above and below, measured for `font`.
fn padded<'a>(
    ui: &Ui,
    text: &'a mut String,
    height: f32,
    font: egui::FontId,
    [left, right]: [i8; 2],
) -> egui::TextEdit<'a> {
    let line = ui.fonts_mut(|fonts| fonts.row_height(&font)) + ui.spacing().extra_text_line_spacing;
    let padding = (height - line).max(0.0);
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

/// [`icon_button`] at `size`, its icon `icon_size` points.
pub fn icon_button_sized(
    ui: &mut Ui,
    icon: Icon,
    label: &str,
    size: egui::Vec2,
    icon_size: f32,
    look: &Look,
    palette: &Palette,
) -> Response {
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    response.widget_info(|| WidgetInfo::labeled(WidgetType::Button, ui.is_enabled(), label));
    if ui.is_rect_visible(rect) {
        if response.hovered() {
            ui.painter().rect_filled(
                rect,
                CornerRadius::same(look.radius.saturating_sub(2)),
                palette.surface_hover,
            );
        }
        let tint = if !ui.is_enabled() {
            palette.faint
        } else if response.hovered() {
            palette.text
        } else {
            palette.secondary
        };
        icon.image(tint, icon_size).paint_at(
            ui,
            Rect::from_center_size(rect.center(), vec2(icon_size, icon_size)),
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

/// [`virtual_rows`] for rows of different heights: `heights[i]` is row
/// `i`'s. Only the rows in view are built, plus one on each side.
pub fn virtual_rows_varying(ui: &mut Ui, heights: &[f32], mut row: impl FnMut(&mut Ui, usize)) {
    if heights.is_empty() {
        return;
    }
    let previous_spacing = ui.spacing().item_spacing;
    ui.spacing_mut().item_spacing.y = 0.0;
    let clip = ui.clip_rect();
    let start_y = ui.cursor().top();
    let width = ui.available_width();
    let mut tops = Vec::with_capacity(heights.len() + 1);
    let mut y = 0.0;
    for height in heights {
        tops.push(y);
        y += height;
    }
    tops.push(y);
    let visible_top = clip.top() - start_y;
    let visible_bottom = clip.bottom() - start_y;
    let first = tops
        .partition_point(|top| *top <= visible_top)
        .saturating_sub(2);
    let last = (tops.partition_point(|top| *top < visible_bottom) + 1).min(heights.len());
    let first = first.min(last);
    if first > 0 {
        ui.allocate_space(vec2(width, tops[first]));
    }
    for index in first..last {
        row(ui, index);
    }
    if last < heights.len() {
        ui.allocate_space(vec2(width, tops[heights.len()] - tops[last]));
    }
    ui.spacing_mut().item_spacing = previous_spacing;
}

/// Where a row's highlight sits in the row's `rect`: the whole row, or
/// inset for the macOS pill. The sidebar's keyboard cursor follows it.
pub fn selection_rect(rect: Rect, look: &Look) -> Rect {
    match look.selection {
        Selection::Pill => rect.shrink2(vec2(8.0, 0.0)),
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
            let corner = CornerRadius::same(look.radius.saturating_sub(2));
            if selected {
                painter.rect_filled(pill, corner, palette.selection);
            } else if hovered {
                painter.rect_filled(pill, corner, palette.surface);
            }
        }
        Selection::Bar => {
            if selected {
                painter.rect_filled(rect, CornerRadius::ZERO, palette.selection);
                let edge = Rect::from_min_size(rect.min, vec2(2.0, rect.height()));
                painter.rect_filled(edge, CornerRadius::ZERO, palette.accent);
            } else if hovered {
                painter.rect_filled(rect, CornerRadius::ZERO, palette.text.gamma_multiply(0.08));
            }
        }
    }
}

/// The text colour of a row: macOS marks the selected row in the strong accent.
pub fn selection_text(selected: bool, look: &Look, palette: &Palette) -> Color32 {
    if selected && look.selection == Selection::Pill {
        palette.accent_hover
    } else {
        palette.text
    }
}

/// How far a raised tab sits inside its track.
const TRACK_INSET: f32 = 2.0;

/// A tab's background: the object tabs, with `radius` = `look.radius`.
/// Raised tabs sit in a [`TabTrack`], which fills the inactive ones.
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
        TabStyle::Outlined => {
            if active {
                painter.rect_filled(rect, corner, active_tab_fill(look, palette));
                painter.rect_stroke(
                    rect,
                    corner,
                    Stroke::new(1.0, palette.outline),
                    StrokeKind::Inside,
                );
            } else if hovered {
                painter.rect_filled(rect, corner, palette.surface_hover);
            }
        }
        TabStyle::Raised => {
            // Inset and concentric with the track, as a segmented control.
            let rect = rect.shrink(TRACK_INSET);
            let corner = CornerRadius::same(radius.saturating_sub(TRACK_INSET as u8));
            if active {
                painter.add(raised_shadow(palette).as_shape(rect, corner));
                painter.rect_filled(rect, corner, active_tab_fill(look, palette));
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

/// The track raised tabs sit in, as Finder's path bar and Safari's tabs:
/// one rounded surface behind every tab, so inactive tabs read as tabs
/// rather than as text on the bar. Its extent is known only once the tabs
/// are laid out, so its shape is reserved first and painted behind them.
pub struct TabTrack {
    shape: Option<egui::layers::ShapeIdx>,
    rect: Rect,
}

impl TabTrack {
    /// Reserves the track's place, under everything painted after it.
    /// Tabs in a track touch: the caller lays them out with no spacing.
    pub fn begin(ui: &Ui, look: &Look) -> Self {
        Self {
            shape: (look.tabs == TabStyle::Raised).then(|| ui.painter().add(egui::Shape::Noop)),
            rect: Rect::NOTHING,
        }
    }

    /// Whether tabs sit in a track, and so touch.
    pub fn is_shown(&self) -> bool {
        self.shape.is_some()
    }

    pub fn add(&mut self, tab: Rect) {
        self.rect = self.rect.union(tab);
    }

    pub fn end(self, ui: &Ui, radius: u8, palette: &Palette) {
        if let Some(shape) = self.shape
            && self.rect.is_positive()
        {
            ui.painter().set(
                shape,
                egui::epaint::RectShape::filled(
                    self.rect,
                    CornerRadius::same(radius),
                    track_fill(palette),
                ),
            );
        }
    }
}

/// The track under raised tabs: the surface tone, one step off the bar.
pub fn track_fill(palette: &Palette) -> Color32 {
    palette.surface
}

/// The fill of something raised off its bar (a raised tab, a pop-up
/// button): the window colour, which is lighter than the bar in a light
/// palette. In a dark palette the window is darker than the bar and would
/// read as a slot, so it takes the lightest surface instead.
pub fn raised_fill(palette: &Palette) -> Color32 {
    if palette.dark {
        palette.surface_active
    } else {
        palette.window
    }
}

/// The soft shadow under something raised.
fn raised_shadow(palette: &Palette) -> egui::epaint::Shadow {
    egui::epaint::Shadow {
        offset: [0, 1],
        blur: 4,
        spread: 0,
        color: palette.shadow.gamma_multiply(0.6),
    }
}

/// The fill of the active tab: the window colour, so the tab joins the
/// content below it, or the [`raised_fill`] of a raised tab.
pub fn active_tab_fill(look: &Look, palette: &Palette) -> Color32 {
    if look.tabs == TabStyle::Raised {
        raised_fill(palette)
    } else {
        palette.window
    }
}

/// A pop-up button: `combo` with `contents` as its menu. With
/// `look.raised_popups` it is drawn as macOS draws one: raised off its
/// background with a soft shadow and a hairline, and marked with up and
/// down chevrons. Other looks keep egui's combo box.
pub fn popup_button<R>(
    ui: &mut Ui,
    combo: egui::ComboBox,
    look: &Look,
    palette: &Palette,
    contents: impl FnOnce(&mut Ui) -> R,
) -> egui::InnerResponse<Option<R>> {
    if !look.raised_popups {
        return combo.show_ui(ui, contents);
    }
    let shadow = ui.painter().add(egui::Shape::Noop);
    let fill = raised_fill(palette);
    let rim = Stroke::new(1.0, palette.text.gamma_multiply(0.1));
    let inner = ui.scope(|ui| {
        let widgets = &mut ui.visuals_mut().widgets;
        for widget in [
            &mut widgets.inactive,
            &mut widgets.hovered,
            &mut widgets.open,
        ] {
            widget.weak_bg_fill = fill;
            widget.bg_stroke = rim;
        }
        widgets.active.bg_stroke = rim;
        combo
            .icon(|ui, rect, visuals, _open| {
                paint_chevrons(ui.painter(), rect, visuals.fg_stroke.color);
            })
            .show_ui(ui, contents)
    });
    let rect = inner.inner.response.rect;
    if ui.is_rect_visible(rect) {
        let corner = ui.visuals().widgets.inactive.corner_radius;
        ui.painter()
            .set(shadow, raised_shadow(palette).as_shape(rect, corner));
    }
    inner.inner
}

/// One choice of a [`popup_menu`].
pub struct MenuChoice {
    /// What the choice reads.
    pub text: String,
    /// Its name for screen readers, when it is not what it reads (the
    /// terminal look's lower case).
    pub name: Option<String>,
    /// The choice in use.
    pub selected: bool,
}

/// The menu that `button` opens when clicked: `choices` under it, at least
/// `min_width` wide. Returns the index of the choice picked this frame.
/// `choices` is asked for only while the menu is open.
///
/// egui closes a menu on a pointer's click; a pick by key or by a screen
/// reader closes it here. After a pick or Escape the keyboard is back on
/// `button`, where it was before the menu opened.
pub fn popup_menu(
    button: &Response,
    min_width: f32,
    look: &Look,
    choices: impl FnOnce() -> Vec<MenuChoice>,
) -> Option<usize> {
    let mut picked = None;
    let open = egui::Popup::menu(button).show(|ui| {
        ui.set_min_width(min_width);
        for (index, choice) in choices().into_iter().enumerate() {
            let text = galley(ui, &choice.text, Color32::PLACEHOLDER, look);
            let response = ui.add(egui::Button::selectable(choice.selected, text));
            if let Some(name) = &choice.name {
                response.widget_info(|| {
                    WidgetInfo::selected(WidgetType::Button, true, choice.selected, name)
                });
            }
            if response.clicked() {
                picked = Some(index);
                ui.close();
            }
        }
    });
    let escaped = || {
        button
            .ctx
            .input(|input| input.key_pressed(egui::Key::Escape))
    };
    if open.is_some() && (picked.is_some() || escaped()) {
        button.request_focus();
    }
    picked
}

/// Up and down chevrons, centred in `rect`: the mark of a macOS pop-up
/// button.
fn paint_chevrons(painter: &egui::Painter, rect: Rect, color: Color32) {
    let center = rect.center();
    let half_width = rect.width() * 0.25;
    let rise = half_width * 0.75;
    let gap = rect.height() * 0.1;
    let stroke = Stroke::new(1.5, color);
    // Up, then down: the wings sit `gap` off the centre, the tip `rise`
    // further out.
    for direction in [-1.0, 1.0] {
        let wings = center.y + direction * gap;
        painter.line(
            vec![
                egui::pos2(center.x - half_width, wings),
                egui::pos2(center.x, wings + direction * rise),
                egui::pos2(center.x + half_width, wings),
            ],
            stroke,
        );
    }
}

/// `text` in the body role as a galley for a widget egui draws (a combo
/// box's value, a menu item, a field's hint).
pub fn galley(ui: &Ui, text: &str, color: Color32, look: &Look) -> std::sync::Arc<egui::Galley> {
    Text::one(look, body(look), text, color)
        .layout(ui.ctx())
        .galley
}

/// A label in the layout: `text` in `role`.
pub fn label(ui: &mut Ui, role: TextRole, text: &str, color: Color32, look: &Look) -> Response {
    Text::one(look, role, text, color)
        .layout(ui.ctx())
        .label(ui)
}

/// A dialog's title role: 17 semibold, the terminal's 14 bold.
pub fn dialog_title(look: &Look) -> TextRole {
    TextRole::pick(look, TextRole::DialogTitle, TextRole::OScreenTitle)
}

/// Monospace for code in dialogs and the structure view: raw SQL, types,
/// fingerprints, keys.
pub fn code(look: &Look) -> TextRole {
    TextRole::pick(look, TextRole::MonoSecondary, TextRole::OSecondary)
}

/// egui's button with `text` in the body role.
pub fn button(ui: &mut Ui, text: &str, look: &Look) -> Response {
    let laid = Text::one(look, body(look), text, Color32::PLACEHOLDER).layout(ui.ctx());
    ui.add(egui::Button::new(laid.galley))
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
    let font = body(look).font_id(look.faces);
    let right = search_icon_inset(look);
    let left = right + SEARCH_ICON as i8 + 6;
    padded(ui, text, look.control_height, font, [left, right]).hint_text(hint)
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
        let laid = Text::one(look, body(look), text, Color32::PLACEHOLDER).layout(ui.ctx());
        ui.add(egui::Checkbox::new(checked, laid.galley))
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

/// The colour of text on `fill`: whichever of the window's two tones reads
/// on it. A dark theme's danger colour is a light one.
pub fn ink_on(fill: Color32, palette: &Palette) -> Color32 {
    if crate::theme::contrast(palette.window, fill) >= crate::theme::contrast(palette.text, fill) {
        palette.window
    } else {
        palette.text
    }
}

/// The one accent-filled button in a dialog or view. Its own fill replaces
/// egui's state visuals, so it draws hover and press itself; its focus ring
/// is the one every control gets (see [`crate::ui::focus`]).
pub fn primary_button(ui: &mut Ui, text: &str, look: &Look, palette: &Palette) -> Response {
    // Last frame's state, read as egui's Button does, picks this frame's fill.
    let state = ui.ctx().read_response(ui.next_auto_id());
    let hovered = state.as_ref().is_some_and(Response::hovered);
    let focused = state.as_ref().is_some_and(Response::has_focus);
    let pressed = state
        .as_ref()
        .is_some_and(Response::is_pointer_button_down_on);
    let fill = primary_fill(hovered, focused, pressed, palette);
    let role = TextRole::pick(look, TextRole::UiBodyStrong, TextRole::OGroup);
    let laid = Text::one(look, role, text, palette.on_accent).layout(ui.ctx());
    let mut button = egui::Button::new(laid.galley)
        .fill(fill)
        .corner_radius(CornerRadius::same(look.radius))
        .min_size(vec2(0.0, look.control_height));
    if look.bordered_controls {
        // The fill edges itself, not the grey border of other controls.
        button = button.stroke(Stroke::new(1.0, fill));
    }
    ui.add(button)
}

/// A dialog's frame: a soft shadow round it, or Omarchy's accent border.
/// A dialog whose parts fill it edge to edge takes the margin off.
pub fn modal_frame(look: &Look, palette: &Palette) -> egui::Frame {
    let frame = egui::Frame::new()
        .fill(palette.overlay)
        .corner_radius(CornerRadius::same(look.dialog_radius))
        .inner_margin(egui::Margin::same(20));
    match look.dialog {
        DialogStyle::Shadow => {
            frame
                .stroke(Stroke::new(1.0, palette.outline))
                .shadow(egui::epaint::Shadow {
                    offset: [0, 10],
                    blur: 36,
                    spread: 0,
                    color: palette.shadow,
                })
        }
        DialogStyle::AccentBorder => frame.stroke(Stroke::new(2.0, palette.accent)),
    }
}

/// A dialog: a soft shadow over a dimmed window, or Omarchy's accent border
/// over a scrim.
pub fn modal(id: egui::Id, look: &Look, palette: &Palette) -> egui::Modal {
    let modal = egui::Modal::new(id).frame(modal_frame(look, palette));
    match look.dialog {
        DialogStyle::Shadow => modal.backdrop_color(Color32::from_black_alpha(if palette.dark {
            120
        } else {
            70
        })),
        DialogStyle::AccentBorder => {
            let [r, g, b, _] = palette.window.to_array();
            modal.backdrop_color(Color32::from_rgba_unmultiplied(r, g, b, 128))
        }
    }
}

/// The top `height` points of `rect` with its top corners rounded at
/// `radius`: a stripe that follows the dialog's corners, which a rounded
/// rectangle that thin cannot.
pub fn top_cap(rect: Rect, radius: f32, height: f32) -> Vec<egui::Pos2> {
    let bottom = rect.top() + height;
    let radius = radius.max(0.0);
    let from = if radius > 0.0 {
        ((radius - height).max(0.0) / radius).asin()
    } else {
        std::f32::consts::FRAC_PI_2
    };
    // From the stripe's lower edge up to the top: how far in from the
    // side, how far down from the top.
    const STEPS: usize = 8;
    let arc: Vec<(f32, f32)> = (0..=STEPS)
        .map(|step| {
            let angle = from + (std::f32::consts::FRAC_PI_2 - from) * step as f32 / STEPS as f32;
            (radius - radius * angle.cos(), radius - radius * angle.sin())
        })
        .collect();
    let mut points = Vec::with_capacity(2 * arc.len() + 2);
    if height > radius {
        points.push(egui::pos2(rect.left(), bottom));
    }
    points.extend(
        arc.iter()
            .map(|(dx, dy)| egui::pos2(rect.left() + dx, rect.top() + dy)),
    );
    points.extend(
        arc.iter()
            .rev()
            .map(|(dx, dy)| egui::pos2(rect.right() - dx, rect.top() + dy)),
    );
    if height > radius {
        points.push(egui::pos2(rect.right(), bottom));
    }
    points
}

/// One physical pixel, in points: the width of hairlines, which the
/// designs draw as a single device pixel at any scale.
pub fn hairline(ui: &Ui) -> f32 {
    1.0 / ui.pixels_per_point()
}

/// A horizontal hairline across `x` at `y`, snapped to the pixel grid.
pub fn hline(ui: &Ui, x: egui::Rangef, y: f32, color: Color32) {
    let width = hairline(ui);
    let y = ui.painter().round_to_pixel_center(y);
    ui.painter().hline(x, y, Stroke::new(width, color));
}

/// A vertical hairline across `y` at `x`, snapped to the pixel grid.
pub fn vline(ui: &Ui, x: f32, y: egui::Rangef, color: Color32) {
    let width = hairline(ui);
    let x = ui.painter().round_to_pixel_center(x);
    ui.painter().vline(x, y, Stroke::new(width, color));
}

/// Paints `text` with its left edge at `x`, centred on `y`. Returns its width.
pub fn paint_text(ui: &Ui, x: f32, y: f32, text: Text) -> f32 {
    text.layout(ui.ctx()).paint_left(ui.painter(), x, y)
}

/// Paints `text` right-aligned at `right`, centred on `y`. Returns its width.
pub fn paint_text_right(ui: &Ui, right: f32, y: f32, text: Text) -> f32 {
    text.layout(ui.ctx()).paint_right(ui.painter(), right, y)
}

/// Names painted text for screen readers (and the headless tests): a
/// label node over `rect`.
pub fn announce(ui: &Ui, rect: Rect, text: &str) {
    let id = ui
        .id()
        .with(("announce", text, rect.min.x as i32, rect.min.y as i32));
    ui.interact(rect, id, Sense::hover())
        .widget_info(|| WidgetInfo::labeled(WidgetType::Label, true, text));
}

/// [`paint_text`], announced as a label.
pub fn paint_label(ui: &Ui, x: f32, y: f32, text: Text) -> f32 {
    let laid = text.layout(ui.ctx());
    let width = laid.paint_left(ui.painter(), x, y);
    announce(
        ui,
        Rect::from_min_size(egui::pos2(x, y - 8.0), vec2(width.max(1.0), 16.0)),
        laid.galley.text(),
    );
    width
}

/// The width of `text` laid out.
pub fn measure(ui: &Ui, text: Text) -> f32 {
    text.layout(ui.ctx()).width()
}

/// A section label: RECENT, PUBLIC. Small semibold capitals with a little
/// tracking on macOS; the terminal's lower-case caption.
pub fn section_label(text: &str, look: &Look, palette: &Palette) -> Text {
    if look.terminal {
        Text::one(look, TextRole::OCaption, &text.to_lowercase(), palette.text)
    } else {
        Text::one(look, TextRole::SectionLabel, text, palette.dim)
    }
}

/// One keyboard hint: its key, what it does, and whether it is possible.
pub type Hint<'a> = (&'a str, &'a str, bool);

/// Keyboard hints: each key in the text colour, then what it does in the
/// muted one. Disabled hints are struck through. Returns their width.
pub fn key_hints(
    ui: &Ui,
    (x, y): (f32, f32),
    hints: &[Hint<'_>],
    gap: f32,
    look: &Look,
    palette: &Palette,
) -> f32 {
    key_hints_in(ui, (x, y), hints, gap, secondary(look), look, palette)
}

/// One hint as text: the key, a space, and what it does.
fn hint_text(hint: &Hint<'_>, role: TextRole, look: &Look, palette: &Palette) -> Text {
    let (key, label, enabled) = *hint;
    let (key_color, label_color) = if enabled {
        (palette.text, palette.dim)
    } else {
        (palette.faint, palette.faint)
    };
    let mut text = Text::new(look);
    if !key.is_empty() {
        text = text.add(role, key, key_color);
        if !label.is_empty() {
            text = text.space(role, " ");
        }
    }
    if !label.is_empty() {
        text = text.add(role, label, label_color);
    }
    text
}

/// [`key_hints`] in `role`.
pub fn key_hints_in(
    ui: &Ui,
    (x, y): (f32, f32),
    hints: &[Hint<'_>],
    gap: f32,
    role: TextRole,
    look: &Look,
    palette: &Palette,
) -> f32 {
    let mut left = x;
    for (index, hint) in hints.iter().enumerate() {
        if index > 0 {
            left += gap;
        }
        let start = left;
        left += paint_text(ui, left, y, hint_text(hint, role, look, palette));
        if !hint.2 {
            ui.painter().hline(
                egui::Rangef::new(start, left),
                y,
                Stroke::new(hairline(ui).max(1.0), palette.faint),
            );
        }
    }
    left - x
}

/// The width `hints` take in [`key_hints`].
pub fn key_hints_width(
    ui: &Ui,
    hints: &[Hint<'_>],
    gap: f32,
    look: &Look,
    palette: &Palette,
) -> f32 {
    hints
        .iter()
        .map(|hint| measure(ui, hint_text(hint, secondary(look), look, palette)))
        .sum::<f32>()
        + gap * hints.len().saturating_sub(1) as f32
}

/// A text field in a drawn box, with a magnifier (or, in the terminal look,
/// an accent `/`) before the text: the sidebar and picker filters.
pub fn filter_field(
    ui: &mut Ui,
    text: &mut String,
    hint: &str,
    size: egui::Vec2,
    style: FieldStyle,
    look: &Look,
    palette: &Palette,
) -> Response {
    let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
    let corner = CornerRadius::same(if look.terminal { 3 } else { look.radius });
    // The box goes under the text, so its place is kept before the field.
    let backdrop = ui.painter().add(egui::Shape::Noop);
    let slash = Text::one(look, TextRole::OGroup, "/", palette.accent).layout(ui.ctx());
    // As the design's fields: a 1 pt border outside the padding (boxed), 10
    // in, a 14 pt magnifier and 8 (macOS), or 8 in, a bold accent slash and
    // 8, 10 on a bare line (terminal); then the input's own 2 pt padding,
    // which the design keeps.
    let border = if style.boxed { 1.0 } else { 0.0 };
    let (pad, mark, gap) = match (look.terminal, style.boxed) {
        (true, true) => (8.0, slash.width(), 8.0),
        (true, false) => (8.0, slash.width(), 10.0),
        (false, _) => (10.0, 14.0, 8.0),
    };
    let lead = border + pad + mark + gap + 2.0;
    // The text's line, centred in the box as painted text is: level with
    // the magnifier.
    let line = style.role.row_height(ui.ctx(), look.faces);
    let top = rect.center().y - style.role.middle(ui.ctx(), look.faces);
    let inner = Rect::from_min_max(
        egui::pos2(rect.left() + lead, top),
        egui::pos2(rect.right() - 8.0, top + line),
    );
    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(inner));
    let mut layouter = crate::typography::layouter(look, style.role, palette.text);
    let placeholder = Text::one(look, style.role, hint, palette.dim).layout(ui.ctx());
    let response = child.add(
        egui::TextEdit::singleline(text)
            .font(style.role.font_id(look.faces))
            .frame(egui::Frame::NONE)
            .margin(egui::Margin::ZERO)
            .desired_width(inner.width())
            .hint_text(placeholder.galley)
            .layouter(&mut layouter),
    );
    let focused = response.has_focus();
    // The box is the field, not the line of text inside it. A bare line
    // has its caret and its accent mark.
    let ring = if style.boxed {
        Ring::Field { radius: corner.nw }
    } else {
        Ring::Own
    };
    focus::hint(ui, &response, rect, ring);
    let painter = ui.painter();
    if let Some(fill) = style.fill {
        painter.set(
            backdrop,
            egui::epaint::RectShape::filled(rect, corner, fill),
        );
    }
    if style.boxed {
        let border = if focused {
            palette.accent
        } else if look.terminal {
            palette.outline
        } else {
            palette.border
        };
        let width = if look.terminal { 1.0 } else { hairline(ui) };
        painter.rect_stroke(rect, corner, Stroke::new(width, border), StrokeKind::Inside);
    }
    if look.terminal {
        slash.paint_left(ui.painter(), rect.left() + border + pad, rect.center().y);
    } else {
        Icon::Search.image(palette.dim, 14.0).paint_at(
            ui,
            Rect::from_center_size(
                egui::pos2(rect.left() + border + pad + 7.0, rect.center().y),
                vec2(14.0, 14.0),
            ),
        );
    }
    response
}

/// How a [`filter_field`] draws: its fill (none on its bar's colour), its
/// box, and its text's role.
#[derive(Clone, Copy)]
pub struct FieldStyle {
    pub fill: Option<Color32>,
    pub boxed: bool,
    pub role: TextRole,
}

/// One segment of a [`segmented`] control.
#[derive(Clone, Copy)]
pub enum Segment<'a> {
    Icon(Icon, &'a str),
    Text(&'a str),
}

/// A segmented control; returns the index of a clicked segment. macOS: a
/// sunken track with the chosen segment raised in white; the terminal
/// look: a bordered row with the chosen segment in the selection colour.
pub fn segmented(
    ui: &mut Ui,
    segments: &[Segment<'_>],
    selected: usize,
    segment: egui::Vec2,
    look: &Look,
    palette: &Palette,
) -> Option<usize> {
    let role = TextRole::pick(look, TextRole::FieldLabel, TextRole::OCaption);
    let label = |text: &str, color| Text::one(look, role, text, color);
    // Terminal text segments fit their words, 6 each side; macOS icons sit
    // in fixed cells.
    let widths: Vec<f32> = segments
        .iter()
        .map(|part| match part {
            Segment::Text(text) if look.terminal => {
                measure(ui, label(text, Color32::PLACEHOLDER)) + 12.0
            }
            _ => segment.x,
        })
        .collect();
    let pad = if look.terminal { 1.0 } else { 2.0 };
    let size = vec2(
        widths.iter().sum::<f32>() + 2.0 * pad,
        segment.y + 2.0 * pad,
    );
    let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
    let painter = ui.painter().clone();
    if look.terminal {
        painter.rect_stroke(
            rect,
            CornerRadius::same(3),
            Stroke::new(1.0, palette.outline),
            StrokeKind::Inside,
        );
    } else {
        painter.rect_filled(
            rect,
            CornerRadius::same(look.radius.saturating_sub(2)),
            palette.surface,
        );
    }
    let mut clicked = None;
    let mut left = rect.left() + pad;
    for (index, part) in segments.iter().enumerate() {
        let cell = Rect::from_min_size(
            egui::pos2(left, rect.top() + pad),
            vec2(widths[index], segment.y),
        );
        left += widths[index];
        let (name, icon, text) = match part {
            Segment::Icon(icon, name) => (*name, Some(*icon), None),
            Segment::Text(text) => (*text, None, Some(*text)),
        };
        // One Tab stop for the control; the arrows choose inside it.
        let (response, arrow) = focus::segment(
            ui,
            cell,
            ui.id().with(("segment", name)),
            ui.id().with("segments"),
            (index, segments.len()),
            index == selected,
        );
        response.widget_info(|| {
            WidgetInfo::selected(WidgetType::Button, true, index == selected, name)
        });
        if response.clicked() {
            clicked = Some(index);
        }
        clicked = arrow.or(clicked);
        // On the segment's own edge: a ring further out would leave the
        // track.
        let ring = if look.terminal {
            Ring::Inset { radius: 0 }
        } else {
            Ring::Edge {
                radius: look.radius.saturating_sub(4),
            }
        };
        focus::hint(ui, &response, cell, ring);
        let active = index == selected;
        if look.terminal && index > 0 {
            vline(ui, cell.left(), cell.y_range(), palette.outline);
        }
        if active {
            if look.terminal {
                painter.rect_filled(cell, CornerRadius::ZERO, palette.selection);
            } else {
                let corner = CornerRadius::same(look.radius.saturating_sub(4));
                painter.add(
                    egui::epaint::Shadow {
                        offset: [0, 1],
                        blur: 2,
                        spread: 0,
                        color: Color32::from_black_alpha(31),
                    }
                    .as_shape(cell, corner),
                );
                painter.rect_filled(cell, corner, palette.window);
            }
        } else if response.hovered() && look.terminal {
            painter.rect_filled(cell, CornerRadius::ZERO, palette.text.gamma_multiply(0.08));
        }
        let color = match (active, look.terminal) {
            (true, true) => palette.accent,
            (true, false) => palette.text,
            _ => palette.dim,
        };
        if let Some(icon) = icon {
            icon.image(color, 13.0)
                .paint_at(ui, Rect::from_center_size(cell.center(), vec2(13.0, 13.0)));
        }
        if let Some(text) = text {
            let laid = label(text, color).layout(ui.ctx());
            laid.paint_center(&painter, cell.center());
        }
        if response.hovered() {
            let _ = response.on_hover_text(name);
        }
    }
    clicked
}

/// What a [`ButtonSpec`] does when clicked.
#[derive(Clone, Copy, PartialEq, Eq)]
enum ButtonKind {
    Secondary,
    Primary,
    /// The one button of a dialog that writes where a write is asked
    /// about first: filled with the danger colour.
    Danger,
    /// Shown but not yet possible; the text says why.
    Disabled,
}

/// A button in the designs' style: secondary (bordered), primary (ink on
/// macOS, accent outline in the terminal look), danger (primary, in the
/// danger colour) or disabled with a reason, with an optional icon and
/// shortcut.
pub struct ButtonSpec<'a> {
    text: &'a str,
    /// The accessible name, when it says more than the text.
    label: Option<&'a str>,
    salt: Option<&'a str>,
    icon: Option<Icon>,
    shortcut: Option<&'a str>,
    kind: ButtonKind,
    reason: Option<&'a str>,
    /// Space at each side, between the icon and the text, and before the
    /// shortcut.
    padding: f32,
    gap: f32,
    radius: Option<u8>,
    /// The text's role; by default the body, medium for primary buttons
    /// on macOS.
    role: Option<TextRole>,
    shortcut_role: Option<TextRole>,
    icon_size: f32,
    /// The contents start at the left and the shortcut ends at the right.
    justified: bool,
    /// The text muted and the shortcut in the text colour.
    hint: bool,
    /// No border, and the text in the secondary colour.
    quiet: bool,
}

impl<'a> ButtonSpec<'a> {
    pub fn new(text: &'a str) -> Self {
        Self {
            text,
            label: None,
            salt: None,
            icon: None,
            shortcut: None,
            kind: ButtonKind::Secondary,
            reason: None,
            padding: 12.0,
            gap: 6.0,
            radius: None,
            role: None,
            shortcut_role: None,
            icon_size: 14.0,
            justified: false,
            hint: false,
            quiet: false,
        }
    }

    /// For a button as wide as its container: the icon and the text start
    /// at its left, the shortcut ends at its right.
    pub fn justified(mut self) -> Self {
        self.justified = true;
        self
    }

    /// A secondary button that reads as a key hint does: the text muted,
    /// the shortcut in the text colour.
    pub fn hint(mut self) -> Self {
        self.hint = true;
        self
    }

    /// A secondary button that stands back from the ones beside it: no
    /// border, a fill only under the pointer, the text in the secondary
    /// colour.
    pub fn quiet(mut self) -> Self {
        self.quiet = true;
        self
    }

    /// The icon's size (14 by default).
    pub fn icon_size(mut self, size: f32) -> Self {
        self.icon_size = size;
        self
    }

    /// The shortcut's role ([`TextRole::Shortcut`] by default).
    pub fn shortcut_role(mut self, role: TextRole) -> Self {
        self.shortcut_role = Some(role);
        self
    }

    /// Space at each side of the contents.
    pub fn padding(mut self, padding: f32) -> Self {
        self.padding = padding;
        self
    }

    /// Space between the icon, the text and the shortcut.
    pub fn gap(mut self, gap: f32) -> Self {
        self.gap = gap;
        self
    }

    pub fn radius(mut self, radius: u8) -> Self {
        self.radius = Some(radius);
        self
    }

    /// Draws the text in `role`.
    pub fn role(mut self, role: TextRole) -> Self {
        self.role = Some(role);
        self
    }

    pub fn icon(mut self, icon: Icon) -> Self {
        self.icon = Some(icon);
        self
    }

    /// Tells apart buttons with the same name (one per list row).
    pub fn salt(mut self, salt: &'a str) -> Self {
        self.salt = Some(salt);
        self
    }

    fn id(&self, ui: &Ui) -> egui::Id {
        ui.id()
            .with(("button", self.label.unwrap_or(self.text), self.salt))
    }

    /// The name screen readers announce ("Connect to Bookshop").
    pub fn label(mut self, label: &'a str) -> Self {
        self.label = Some(label);
        self
    }

    /// The button in `rect`, working but not drawn: an action that shows
    /// only under the pointer stays reachable by keyboard and screen reader.
    pub fn hidden_at(self, ui: &mut Ui, rect: Rect) -> Response {
        self.hidden_sensing(ui, rect, Sense::click())
    }

    /// [`Self::hidden_at`] that Tab passes by: for a place too small for a
    /// ring to show where the keyboard is. The pointer and a screen reader
    /// press it all the same.
    pub fn hidden_off_tab(self, ui: &mut Ui, rect: Rect) -> Response {
        self.hidden_sensing(ui, rect, Sense::CLICK)
    }

    fn hidden_sensing(self, ui: &mut Ui, rect: Rect, sense: Sense) -> Response {
        let name = self.label.unwrap_or(self.text);
        let response = ui.interact(rect, self.id(ui), sense);
        response.widget_info(|| WidgetInfo::labeled(WidgetType::Button, true, name));
        // Not drawn, so the ring is all that shows where the keyboard is.
        focus::hint(ui, &response, rect, Ring::Outer { radius: 0 });
        response
    }

    pub fn shortcut(mut self, shortcut: &'a str) -> Self {
        self.shortcut = Some(shortcut);
        self
    }

    pub fn primary(mut self) -> Self {
        self.kind = ButtonKind::Primary;
        self
    }

    /// The primary button of a question about a write that cannot be
    /// taken back: filled with the danger colour.
    pub fn danger(mut self) -> Self {
        self.kind = ButtonKind::Danger;
        self
    }

    /// Shown, but not clickable; `reason` is its tooltip.
    pub fn disabled(mut self, reason: &'a str) -> Self {
        self.kind = ButtonKind::Disabled;
        self.reason = Some(reason);
        self
    }

    fn text_role(&self, look: &Look) -> TextRole {
        let leads = matches!(self.kind, ButtonKind::Primary | ButtonKind::Danger);
        self.role.unwrap_or(if leads {
            TextRole::pick(look, TextRole::UiBodyStrong, TextRole::OGroup)
        } else {
            body(look)
        })
    }

    fn shortcut_text_role(&self, look: &Look) -> TextRole {
        self.shortcut_role.unwrap_or(TextRole::pick(
            look,
            TextRole::Shortcut,
            TextRole::OColumnType,
        ))
    }

    fn label_text(&self, look: &Look, color: Color32) -> Text {
        Text::one(look, self.text_role(look), self.text, color)
    }

    fn shortcut_text(&self, keys: &str, look: &Look, color: Color32) -> Text {
        Text::one(look, self.shortcut_text_role(look), keys, color)
    }

    /// The button's width.
    pub fn width(&self, ui: &Ui, look: &Look) -> f32 {
        let mut width =
            2.0 * self.padding + measure(ui, self.label_text(look, Color32::PLACEHOLDER));
        if self.icon.is_some() {
            width += self.icon_size + self.gap;
        }
        if let Some(shortcut) = self.shortcut {
            width +=
                self.gap + measure(ui, self.shortcut_text(shortcut, look, Color32::PLACEHOLDER));
        }
        width.ceil()
    }

    /// Allocates the button in the layout, `height` tall.
    pub fn show(self, ui: &mut Ui, height: f32, look: &Look, palette: &Palette) -> Response {
        let (rect, _) = ui.allocate_exact_size(vec2(self.width(ui, look), height), Sense::hover());
        self.show_at(ui, rect, look, palette)
    }

    /// Draws the button in `rect`.
    pub fn show_at(self, ui: &mut Ui, rect: Rect, look: &Look, palette: &Palette) -> Response {
        let enabled = self.kind != ButtonKind::Disabled;
        // A button that cannot be pressed still takes the Tab key, so the
        // keyboard can read why.
        let sense = if enabled {
            Sense::click()
        } else {
            Sense::focusable_noninteractive()
        };
        let name = self.label.unwrap_or(self.text);
        let response = ui.interact(rect, self.id(ui), sense);
        response.widget_info(|| WidgetInfo::labeled(WidgetType::Button, enabled, name));
        let hovered = response.hovered() && enabled;
        let pressed = response.is_pointer_button_down_on() && enabled;
        let corner =
            CornerRadius::same(
                self.radius
                    .unwrap_or(if look.terminal { 3 } else { look.radius }),
            );
        let hair = hairline(ui);
        let (fill, border, text, shortcut) = match (self.kind, look.terminal) {
            (ButtonKind::Primary, false) => {
                let fill = if pressed {
                    palette.text.lerp_to_gamma(palette.window, 0.25)
                } else if hovered {
                    palette.text.lerp_to_gamma(palette.window, 0.15)
                } else {
                    palette.text
                };
                (
                    fill,
                    None,
                    palette.window,
                    palette.window.gamma_multiply(0.72),
                )
            }
            (ButtonKind::Primary, true) => (
                palette
                    .panel
                    .lerp_to_gamma(palette.accent, if hovered { 0.24 } else { 0.15 }),
                Some(Stroke::new(1.0, palette.accent)),
                palette.accent,
                palette.dim,
            ),
            (ButtonKind::Danger, false) => {
                let fill = if pressed {
                    palette.danger.lerp_to_gamma(palette.window, 0.25)
                } else if hovered {
                    palette.danger.lerp_to_gamma(palette.window, 0.15)
                } else {
                    palette.danger
                };
                let ink = ink_on(fill, palette);
                (fill, None, ink, ink.gamma_multiply(0.72))
            }
            (ButtonKind::Danger, true) => (
                palette
                    .panel
                    .lerp_to_gamma(palette.danger, if hovered { 0.24 } else { 0.15 }),
                Some(Stroke::new(1.0, palette.danger)),
                palette.danger,
                palette.dim,
            ),
            (ButtonKind::Secondary, false) => (
                if hovered {
                    palette.panel
                } else {
                    palette.window
                },
                Some(Stroke::new(hair, palette.border)),
                palette.text,
                palette.dim,
            ),
            (ButtonKind::Secondary, true) => (
                if hovered {
                    palette.text.gamma_multiply(0.08)
                } else {
                    Color32::TRANSPARENT
                },
                Some(Stroke::new(1.0, palette.outline)),
                palette.text,
                palette.dim,
            ),
            (ButtonKind::Disabled, false) => {
                (palette.surface_hover, None, palette.faint, palette.faint)
            }
            (ButtonKind::Disabled, true) => (
                palette.panel.lerp_to_gamma(palette.text, 0.03),
                None,
                palette.faint,
                palette.faint,
            ),
        };
        let (text, shortcut) = if self.hint && self.kind == ButtonKind::Secondary {
            (palette.dim, palette.text)
        } else {
            (text, shortcut)
        };
        let (fill, border, text) = if self.quiet && self.kind == ButtonKind::Secondary {
            // Under the pointer, the fill its look gives a secondary button.
            let fill = if hovered { fill } else { Color32::TRANSPARENT };
            (fill, None, palette.secondary)
        } else {
            (fill, border, text)
        };
        // With the keyboard on it the terminal's button is reversed, as a
        // terminal marks its cursor; the other looks ring it.
        let reversed = look.terminal && focus::shown(&response);
        let (fill, border, text, shortcut) = if reversed {
            (palette.accent, None, palette.window, palette.window)
        } else {
            (fill, border, text, shortcut)
        };
        let ring = if reversed {
            Ring::Own
        } else {
            Ring::Outer { radius: corner.nw }
        };
        focus::hint(ui, &response, rect, ring);
        let painter = ui.painter();
        painter.rect_filled(rect, corner, fill);
        if let Some(border) = border {
            painter.rect_stroke(rect, corner, border, StrokeKind::Inside);
        }
        let content = self.width(ui, look) - 2.0 * self.padding;
        let mut x = if self.justified {
            rect.left() + self.padding
        } else {
            rect.center().x - content / 2.0
        };
        let y = rect.center().y;
        if let Some(icon) = self.icon {
            let size = self.icon_size;
            icon.image(text, size).paint_at(
                ui,
                Rect::from_center_size(egui::pos2(x + size / 2.0, y), vec2(size, size)),
            );
            x += size + self.gap;
        }
        x += paint_text(ui, x, y, self.label_text(look, text));
        if let Some(keys) = self.shortcut {
            let keys = self.shortcut_text(keys, look, shortcut);
            if self.justified {
                paint_text_right(ui, rect.right() - self.padding, y, keys);
            } else {
                paint_text(ui, x + self.gap, y, keys);
            }
        }
        match self.reason {
            Some(reason) => {
                // Said to a screen reader, and shown while the keyboard is
                // on the button as it is under the pointer.
                ui.ctx().accesskit_node_builder(response.id, |node| {
                    node.set_description(reason);
                });
                if focus::shown(&response) {
                    response.show_tooltip_text(reason);
                }
                response.on_hover_text(reason)
            }
            None => response,
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
        harness.set_look(look);
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
            row.shrink2(vec2(8.0, 0.0))
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
            // Margins are whole points; Plex and JetBrains Mono lines are
            // not, so a field lands within half a point of the height.
            assert!(
                (height - look.control_height).abs() <= 0.5,
                "search field, {}: {height}",
                look.name
            );
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
            // Margins are whole points; Plex and JetBrains Mono lines are
            // not, so a field lands within half a point of the height.
            assert!(
                (height - look.control_height).abs() <= 0.5,
                "single field, {}: {height}",
                look.name
            );
            let mut mono = 0.0;
            harness.frame_with(|ui| {
                let role = crate::typography::TextRole::MonoSecondary;
                mono = ui
                    .add(super::single_in(ui, &mut text, &look, role))
                    .rect
                    .height();
            });
            assert!(
                (mono - look.control_height).abs() <= 0.5,
                "monospace field, {}: {mono}",
                look.name
            );
        }
    }

    #[test]
    fn a_field_is_as_tall_as_it_is_asked_to_be() {
        for look in crate::theme::Look::ALL {
            let mut harness = crate::testing::Harness::new();
            harness.set_look(look);
            let mut text = String::new();
            for height in [30.0, 34.0] {
                let mut drawn = 0.0;
                harness.frame_with(|ui| {
                    let role = super::body(&look);
                    drawn = ui
                        .add(super::field(ui, &mut text, &look, role, height, 10))
                        .rect
                        .height();
                });
                // Margins are whole points; Plex and JetBrains Mono lines are
                // not, so a field lands within half a point of the height.
                assert!(
                    (drawn - height).abs() <= 0.5,
                    "{}: {drawn} for {height}",
                    look.name
                );
            }
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
    fn a_raised_active_tab_stands_above_its_track() {
        use crate::theme::{Look, Palette};
        let dark = Palette::dark();
        let light = Palette::light();
        assert_eq!(
            super::active_tab_fill(&Look::macos(), &dark),
            dark.surface_active
        );
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
        // Tab bars take the panel colour, the track is a step off it, and
        // the raised tab a step further: lighter in the dark palette.
        let sum = |c: Color32| u32::from(c.r()) + u32::from(c.g()) + u32::from(c.b());
        assert!(sum(super::track_fill(&dark)) > sum(dark.panel));
        assert!(sum(super::raised_fill(&dark)) > sum(super::track_fill(&dark)));
        assert!(sum(super::track_fill(&light)) < sum(light.panel));
        assert!(sum(super::raised_fill(&light)) > sum(light.panel));
    }

    #[test]
    fn only_raised_tabs_sit_in_a_track() {
        for look in crate::theme::Look::ALL {
            let mut harness = crate::testing::Harness::new();
            harness.set_look(look);
            harness.frame_with(|ui| {
                let track = super::TabTrack::begin(ui, &look);
                assert_eq!(
                    track.is_shown(),
                    look.tabs == crate::theme::TabStyle::Raised,
                    "{}",
                    look.name
                );
            });
        }
    }

    #[test]
    fn pop_up_buttons_keep_their_name_and_value_in_every_look() {
        for look in crate::theme::Look::ALL {
            let mut harness = crate::testing::Harness::new();
            harness.set_look(look);
            let palette = harness.app.palette;
            let tree = harness.frame_with(|ui| {
                let mut chosen = "bookshop_test".to_owned();
                let combo = egui::ComboBox::from_id_salt("database").selected_text(&chosen);
                let response = super::popup_button(ui, combo, &look, &palette, |ui| {
                    ui.selectable_value(&mut chosen, "postgres".into(), "postgres");
                })
                .response;
                response.widget_info(|| {
                    let mut info = WidgetInfo::labeled(WidgetType::ComboBox, true, "Database");
                    info.current_text_value = Some("bookshop_test".into());
                    info
                });
            });
            let id = crate::testing::node(&tree, "Database", egui::accesskit::Role::ComboBox)
                .unwrap_or_else(|| panic!("{}", look.name));
            let (_, node) = tree.nodes.iter().find(|(n, _)| *n == id).unwrap();
            assert_eq!(node.value(), Some("bookshop_test"), "{}", look.name);
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

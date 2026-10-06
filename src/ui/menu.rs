//! Menus, and the dropdowns that open them: one panel and one kind of row
//! for every menu in the app, as the designs' Components sheet draws them.
//!
//! macOS and Windows: a panel with a 1 pt border, corners of 8 and a soft
//! shadow, its rows 4 in from its edge, 28 high with corners of 5. The row
//! under the pointer or the keyboard takes the selection's tint and the
//! strong accent. The choice in use takes a check mark and a heavier face,
//! never a fill.
//!
//! The terminal: a picker. A 1 pt accent border with corners of 3 round
//! rows a line high; `▌` marks the row under the pointer or the keyboard
//! and a green `✓` the choice in use.

use egui::{
    Color32, CornerRadius, Rect, Response, Sense, Stroke, StrokeKind, Ui, WidgetInfo, WidgetType,
    pos2, vec2,
};

use crate::theme::{Icon, Look, Palette};
use crate::typography::{Text, TextRole};
use crate::ui::focus::{self, Ring};
use crate::ui::widgets;

/// How far a menu opens from what it hangs under.
const GAP: f32 = 4.0;
/// The room between a row's words and the note at its right.
const DETAIL_GAP: f32 = 16.0;

/// The terminal's mark of the choice in use. The second for a face without
/// the first: the look's font is the desktop's own.
const CURRENT: &[&str] = &["✓", "√"];

/// A row's measures in a look.
struct Shape {
    height: f32,
    corner: u8,
    /// How far in the row's first mark or word starts, and its last ends.
    inset: f32,
    /// The column a choice's mark sits in, and the room after it.
    gutter: f32,
    gap: f32,
}

impl Shape {
    fn of(ui: &Ui, look: &Look) -> Self {
        if look.terminal {
            let role = TextRole::OBody;
            let width = |text| role.width(ui.ctx(), look.faces, text);
            Self {
                // A line of text, 1 above and 1 below.
                height: role.row_height(ui.ctx(), look.faces) + 2.0,
                corner: 0,
                inset: 10.0,
                gutter: width("▌"),
                gap: width(" "),
            }
        } else {
            Self {
                height: 28.0,
                corner: 5,
                inset: 8.0,
                gutter: 14.0,
                gap: 6.0,
            }
        }
    }
}

/// The panel a menu's rows sit in.
pub fn frame(look: &Look, palette: &Palette) -> egui::Frame {
    if look.terminal {
        egui::Frame::new()
            .fill(palette.panel)
            .stroke(Stroke::new(1.0, palette.accent))
            .corner_radius(3)
    } else {
        egui::Frame::new()
            .fill(palette.overlay)
            .stroke(Stroke::new(1.0, palette.border))
            .corner_radius(8)
            .inner_margin(4)
            .shadow(egui::epaint::Shadow {
                offset: [0, 8],
                blur: 24,
                spread: 0,
                // Half a dialog's: a menu stands a step off the window.
                color: palette.shadow.gamma_multiply(0.5),
            })
    }
}

/// The heavier face the choice in use is written in.
fn strong(role: TextRole) -> TextRole {
    match role {
        TextRole::UiBody => TextRole::UiBodyStrong,
        TextRole::MonoSecondary => TextRole::MonoGroup,
        other => other,
    }
}

/// One row of a menu.
pub struct Item<'a> {
    text: &'a str,
    name: Option<&'a str>,
    current: Option<bool>,
    role: Option<TextRole>,
    detail: Option<&'a str>,
}

impl<'a> Item<'a> {
    /// A row that does something.
    pub fn action(text: &'a str) -> Self {
        Self {
            text,
            name: None,
            current: None,
            role: None,
            detail: None,
        }
    }

    /// One of a set of choices: `current` is the one in use.
    pub fn choice(text: &'a str, current: bool) -> Self {
        Self {
            current: Some(current),
            ..Self::action(text)
        }
    }

    /// Its name for screen readers, when it is not what it reads.
    pub fn name(mut self, name: &'a str) -> Self {
        self.name = Some(name);
        self
    }

    /// The role it reads in, when not the look's body.
    pub fn role(mut self, role: TextRole) -> Self {
        self.role = Some(role);
        self
    }

    /// A muted note at the row's right.
    pub fn detail(mut self, detail: &'a str) -> Self {
        self.detail = Some(detail);
        self
    }

    /// Draws the row across the menu. A click on it closes the menu by
    /// itself only when a pointer made it: see [`choices`].
    pub fn show(self, ui: &mut Ui, look: &Look, palette: &Palette) -> Response {
        let shape = Shape::of(ui, look);
        let current = self.current == Some(true);
        let role = self.role.unwrap_or_else(|| widgets::body(look));
        let role = if current && !look.terminal {
            strong(role)
        } else {
            role
        };
        // The terminal's cursor is a mark too: every row has its column.
        let marked = self.current.is_some() || look.terminal;
        let lead = shape.inset
            + if marked {
                shape.gutter + shape.gap
            } else {
                0.0
            };
        let note = widgets::secondary(look);
        let measure = |role: TextRole, text| role.width(ui.ctx(), look.faces, text);
        let noted = self
            .detail
            .map_or(0.0, |detail| DETAIL_GAP + measure(note, detail));
        let wanted = lead + measure(role, self.text) + noted + shape.inset;
        let (rect, response) = ui.allocate_at_least(vec2(wanted, shape.height), Sense::click());
        let name = self.name.unwrap_or(self.text);
        response.widget_info(|| match self.current {
            Some(current) => WidgetInfo::selected(WidgetType::Button, true, current, name),
            None => WidgetInfo::labeled(WidgetType::Button, true, name),
        });
        // The note is drawn apart from the words: a screen reader hears it
        // as what the row is described by.
        if let Some(detail) = self.detail {
            ui.ctx().accesskit_node_builder(response.id, |node| {
                node.set_description(detail);
            });
        }
        // The pointer and the keyboard light a row the same way.
        let lit = response.hovered() || focus::shown(&response);
        focus::hint(ui, &response, rect, Ring::Own);
        // A long menu scrolls: the row the keyboard came to is brought
        // into view. Jump, not animate: the next key may come at once.
        if response.gained_focus() {
            response.scroll_to_me_animation(None, egui::style::ScrollAnimation::none());
        }
        if !ui.is_rect_visible(rect) {
            return response;
        }
        let center = rect.center().y;
        let painter = ui.painter();
        if lit {
            painter.rect_filled(rect, CornerRadius::same(shape.corner), palette.selection);
        }
        let ink = if lit && !look.terminal {
            palette.accent_hover
        } else {
            palette.text
        };
        let mark = rect.left() + shape.inset;
        if look.terminal {
            let glyph = if current {
                super::workspace::drawable(ui, TextRole::OBody, look, CURRENT)
                    .map(|glyph| (glyph, palette.success))
            } else if lit {
                Some(("▌", palette.accent))
            } else {
                None
            };
            if let Some((glyph, color)) = glyph {
                Text::one(look, TextRole::OBody, glyph, color)
                    .layout(ui.ctx())
                    .paint_left(painter, mark, center);
            }
        } else if current {
            let place =
                Rect::from_center_size(pos2(mark + shape.gutter / 2.0, center), vec2(12.0, 12.0));
            Icon::Check.image(ink, 12.0).paint_at(ui, place);
        }
        Text::one(look, role, self.text, ink)
            .layout(ui.ctx())
            .paint_left(painter, rect.left() + lead, center);
        if let Some(detail) = self.detail {
            Text::one(look, note, detail, palette.dim)
                .layout(ui.ctx())
                .paint_right(painter, rect.right() - shape.inset, center);
        }
        response
    }
}

/// A menu's rows: no room between them, at least `width` across, and a
/// scroll bar once they are taller than the look lets a menu grow.
fn rows<R>(ui: &mut Ui, width: f32, add: impl FnOnce(&mut Ui) -> R) -> R {
    ui.spacing_mut().item_spacing = egui::Vec2::ZERO;
    ui.set_min_width(width);
    egui::ScrollArea::vertical()
        .max_height(ui.spacing().combo_height)
        .show(ui, add)
        .inner
}

/// The menu `button` opens when clicked: `add`'s rows under it, the panel
/// at least `min_width` wide. `add` runs only while the menu is open.
pub fn under<R>(
    button: &Response,
    min_width: f32,
    look: &Look,
    palette: &Palette,
    add: impl FnOnce(&mut Ui) -> R,
) -> Option<R> {
    let frame = frame(look, palette);
    let inside = min_width - frame.total_margin().sum().x;
    egui::Popup::menu(button)
        .frame(frame)
        .gap(GAP)
        .show(|ui| rows(ui, inside.max(0.0), add))
        .map(|shown| shown.inner)
}

/// The menu a secondary click on `target` opens, at the pointer.
pub fn context<R>(
    target: &Response,
    look: &Look,
    palette: &Palette,
    add: impl FnOnce(&mut Ui) -> R,
) -> Option<R> {
    egui::Popup::context_menu(target)
        .frame(frame(look, palette))
        .show(|ui| rows(ui, 0.0, add))
        .map(|shown| shown.inner)
}

/// One choice of [`choices`].
pub struct Choice {
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
pub fn choices(
    button: &Response,
    min_width: f32,
    look: &Look,
    palette: &Palette,
    choices: impl FnOnce() -> Vec<Choice>,
) -> Option<usize> {
    let mut picked = None;
    let open = under(button, min_width, look, palette, |ui| {
        for (index, choice) in choices().into_iter().enumerate() {
            let item = Item::choice(&choice.text, choice.selected);
            let item = match &choice.name {
                Some(name) => item.name(name),
                None => item,
            };
            if item.show(ui, look, palette).clicked() {
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

/// A choice that reads `text` and sets `value` to `own` when picked. The
/// menu closes on the pick, whatever made it.
pub fn value<T: PartialEq>(
    ui: &mut Ui,
    value: &mut T,
    own: T,
    text: &str,
    look: &Look,
    palette: &Palette,
) -> Response {
    let response = Item::choice(text, *value == own).show(ui, look, palette);
    if response.clicked() {
        *value = own;
        ui.close();
    }
    response
}

/// A dropdown: a field that reads the choice in use, with `▾` at its end,
/// and opens a menu of the choices under it.
pub struct Dropdown<'a> {
    id: egui::Id,
    text: &'a str,
    width: f32,
    height: Option<f32>,
    fill: Option<Color32>,
}

impl<'a> Dropdown<'a> {
    /// A dropdown `width` wide that reads `text`. `salt` tells it from the
    /// others in its `Ui`.
    pub fn new(salt: impl egui::AsIdSalt, text: &'a str, width: f32) -> Self {
        Self {
            id: egui::Id::new(salt),
            text,
            width,
            height: None,
            fill: None,
        }
    }

    /// As tall as the fields beside it, where they are not the look's
    /// controls' height.
    pub fn height(mut self, height: f32) -> Self {
        self.height = Some(height);
        self
    }

    /// The fill of the fields beside it, where it is not the window's.
    pub fn fill(mut self, fill: Color32) -> Self {
        self.fill = Some(fill);
        self
    }

    /// Draws the field, and `add`'s rows in its menu while that is open.
    /// It is announced as a combo box whose value is what it reads: name
    /// it with a `widget_info` of the caller's own, or `labelled_by`.
    pub fn show<R>(
        self,
        ui: &mut Ui,
        look: &Look,
        palette: &Palette,
        add: impl FnOnce(&mut Ui) -> R,
    ) -> egui::InnerResponse<Option<R>> {
        let height = self.height.unwrap_or(look.control_height);
        let (_, rect) = ui.allocate_space(vec2(self.width, height));
        let response = ui.interact(rect, ui.id().with(self.id), Sense::click());
        response.widget_info(|| {
            let mut info = WidgetInfo::new(WidgetType::ComboBox);
            info.enabled = ui.is_enabled();
            info.current_text_value = Some(self.text.to_owned());
            info
        });
        // The terminal: an outline with corners of 3. Elsewhere a field's
        // box, corners one tighter than a button's.
        let (fill, border, radius, line) = if look.terminal {
            (Color32::TRANSPARENT, palette.outline, 3, 1.0)
        } else {
            (
                self.fill.unwrap_or(palette.window),
                palette.border,
                look.radius.saturating_sub(1),
                widgets::hairline(ui),
            )
        };
        if ui.is_rect_visible(rect) {
            let open =
                egui::Popup::is_id_open(ui.ctx(), egui::Popup::default_response_id(&response));
            // A step toward the text under the pointer, and while open.
            let border = if response.hovered() || open {
                border.lerp_to_gamma(palette.text, 0.1)
            } else {
                border
            };
            let corner = CornerRadius::same(radius);
            let painter = ui.painter();
            painter.rect(
                rect,
                corner,
                fill,
                Stroke::new(line, border),
                StrokeKind::Inside,
            );
            // The border, then a field's padding: 8 in the terminal, 10.
            let inset = if look.terminal { 9.0 } else { 11.0 };
            let center = rect.center().y;
            let mark = if look.terminal {
                let mark = Text::one(look, widgets::body(look), "▾", palette.dim).layout(ui.ctx());
                let width = mark.paint_right(painter, rect.right() - inset, center);
                rect.right() - inset - width
            } else {
                let place = Rect::from_center_size(
                    pos2(rect.right() - inset - 5.0, center),
                    vec2(10.0, 10.0),
                );
                Icon::ChevronDown
                    .image(palette.dim, 10.0)
                    .paint_at(ui, place);
                place.left()
            };
            // What does not fit ends at the mark.
            let words = Rect::from_min_max(
                pos2(rect.left() + inset, rect.top()),
                pos2(mark - 6.0, rect.bottom()),
            );
            Text::one(look, widgets::body(look), self.text, palette.text)
                .layout(ui.ctx())
                .paint_left(
                    &painter.with_clip_rect(words.intersect(ui.clip_rect())),
                    words.left(),
                    center,
                );
        }
        focus::hint(ui, &response, rect, Ring::Field { radius });
        let inner = under(&response, rect.width(), look, palette, add);
        // The menu took the keyboard down its rows. Once a key or a screen
        // reader closes it, by a pick or by Escape, the keyboard is back on
        // the field, as after [`choices`]. A pointer that closed it put the
        // keyboard where it clicked.
        let popup = egui::Popup::default_response_id(&response);
        let (escaped, clicked) = ui.input(|input| {
            (
                input.key_pressed(egui::Key::Escape),
                input.pointer.any_click(),
            )
        });
        let closed = !egui::Popup::is_id_open(ui.ctx(), popup);
        if inner.is_some() && (escaped || (closed && !clicked)) {
            response.request_focus();
        }
        egui::InnerResponse::new(inner, response)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use egui::accesskit::Role;
    use egui::{Key, Modifiers};

    use crate::model::Action;
    use crate::testing::{Harness, bounds, node};
    use crate::theme::Look;
    use crate::ui::tests::focused_name;

    /// A SQL editor with its Limit menu open: three choices, the second in
    /// use.
    fn limit_menu(look: Look) -> Harness {
        let mut harness = Harness::new();
        harness.set_look(look);
        let tab = harness.connect_fake();
        harness.app.apply(Action::NewSqlTab(tab));
        harness.click("Limit");
        harness
    }

    fn row(harness: &mut Harness, name: &str) -> egui::Rect {
        let tree = harness.settle();
        bounds(&tree, name, Role::Button).unwrap_or_else(|| panic!("no row {name}"))
    }

    /// Gives the keyboard to the row named `name`, as a screen reader does.
    fn give_keyboard(harness: &mut Harness, name: &str) {
        let tree = harness.settle();
        let target = node(&tree, name, Role::Button).unwrap_or_else(|| panic!("no row {name}"));
        harness.frame(vec![egui::Event::AccessKitActionRequest(
            egui::accesskit::ActionRequest {
                target_tree: egui::accesskit::TreeId::ROOT,
                target_node: target,
                action: egui::accesskit::Action::Focus,
                data: None,
            },
        )]);
        harness.settle();
    }

    /// What the last frame filled the row at `rect` with, if anything.
    fn fill(harness: &Harness, rect: egui::Rect) -> Option<egui::Color32> {
        harness
            .fills
            .iter()
            .find(|(filled, _)| {
                (filled.center() - rect.center()).length() < 1.0
                    && (filled.height() - rect.height()).abs() < 1.0
            })
            .map(|(_, color)| *color)
    }

    #[test]
    fn the_choice_in_use_is_marked_and_not_filled() {
        for look in Look::ALL {
            let mut harness = limit_menu(look);
            let tree = harness.settle();
            let chosen = |name: &str| {
                let id = node(&tree, name, Role::Button).unwrap();
                let (_, node) = tree.nodes.iter().find(|(n, _)| *n == id).unwrap();
                node.toggled() == Some(egui::accesskit::Toggled::True)
            };
            assert!(chosen("Limit 1,000"), "{}", look.name);
            assert!(!chosen("Limit 100"), "{}", look.name);
            let current = row(&mut harness, "Limit 1,000");
            assert_eq!(fill(&harness, current), None, "{}", look.name);
        }
    }

    #[test]
    fn a_menus_rows_touch_and_are_as_tall_as_each_other() {
        for look in Look::ALL {
            let mut harness = limit_menu(look);
            let rows =
                ["Limit 100", "Limit 1,000", "Limit 10,000"].map(|name| row(&mut harness, name));
            for pair in rows.windows(2) {
                assert!(
                    (pair[0].bottom() - pair[1].top()).abs() < 0.5,
                    "{pair:?} in {}",
                    look.name
                );
                assert!(
                    (pair[0].height() - pair[1].height()).abs() < 0.5,
                    "{pair:?} in {}",
                    look.name
                );
                assert!(
                    (pair[0].width() - pair[1].width()).abs() < 0.5,
                    "{pair:?} in {}",
                    look.name
                );
            }
        }
    }

    #[test]
    fn the_pointer_and_the_keyboard_light_a_row_the_same_way() {
        for look in Look::ALL {
            let mut harness = limit_menu(look);
            let selection = harness.app.palette.selection;
            let first = row(&mut harness, "Limit 100");
            assert_eq!(fill(&harness, first), None, "{}", look.name);
            harness.frame(vec![egui::Event::PointerMoved(first.center())]);
            harness.settle();
            assert_eq!(
                fill(&harness, first),
                Some(selection),
                "the row under the pointer in {}",
                look.name
            );
            harness.frame(vec![egui::Event::PointerGone]);
            harness.settle();
            assert_eq!(fill(&harness, first), None, "{}", look.name);
            // A screen reader puts the keyboard on a row, and a key moves
            // it: only then is it shown.
            give_keyboard(&mut harness, "Limit 100");
            harness.press(Key::ArrowDown, Modifiers::NONE);
            let name = focused_name(&harness.settle());
            assert_eq!(name, "Limit 1,000", "{}", look.name);
            let lit = row(&mut harness, &name);
            assert_eq!(
                fill(&harness, lit),
                Some(selection),
                "the row with the keyboard in {}",
                look.name
            );
        }
    }

    #[test]
    fn the_database_menus_rows_touch() {
        for look in Look::ALL {
            let mut harness = Harness::new();
            harness.set_look(look);
            let tab = harness.connect_fake();
            let workspace = harness.app.workspace_mut(tab).unwrap();
            workspace.spec.database = "tabletist".into();
            workspace.databases.value = Some(vec!["tabletist".into(), "postgres".into()]);
            harness.click("Database");
            let rows = ["tabletist", "postgres"].map(|name| row(&mut harness, name));
            assert!(
                (rows[0].bottom() - rows[1].top()).abs() < 0.5,
                "{rows:?} in {}",
                look.name
            );
        }
    }

    #[test]
    fn a_dropdown_keeps_its_name_and_value_in_every_look() {
        for look in Look::ALL {
            let mut harness = Harness::new();
            harness.set_look(look);
            let palette = harness.app.palette;
            let tree = harness.frame_with(|ui| {
                let dropdown = super::Dropdown::new("database", "bookshop_test", 160.0);
                let response = dropdown.show(ui, &look, &palette, |_| {}).response;
                assert_eq!(response.rect.width(), 160.0, "{}", look.name);
                response.widget_info(|| {
                    let mut info =
                        egui::WidgetInfo::labeled(egui::WidgetType::ComboBox, true, "Database");
                    info.current_text_value = Some("bookshop_test".into());
                    info
                });
            });
            let id =
                node(&tree, "Database", Role::ComboBox).unwrap_or_else(|| panic!("{}", look.name));
            let (_, node) = tree.nodes.iter().find(|(n, _)| *n == id).unwrap();
            assert_eq!(node.value(), Some("bookshop_test"), "{}", look.name);
            assert_eq!(harness.painted_color("bookshop_test"), Some(palette.text));
        }
    }

    #[test]
    fn a_pick_in_a_dropdown_sets_the_value_and_closes_its_menu() {
        for look in Look::ALL {
            let mut harness = Harness::new();
            harness.set_look(look);
            let (tab, id) = harness.editable();
            harness.press(Key::F, Modifiers::COMMAND);
            harness.click("Filter operator");
            assert!(harness.has("contains"), "the menu opens in {}", look.name);
            harness.click("contains");
            assert!(
                !harness.has("starts with"),
                "the menu stays open in {}",
                look.name
            );
            // The row that had the keyboard is gone: the dropdown has it.
            assert_eq!(
                focused_name(&harness.settle()),
                "Filter operator",
                "{}",
                look.name
            );
            let workspace = harness.app.workspace(tab).unwrap();
            let rows = &workspace.object_tab(id).unwrap().filter.rows;
            assert_eq!(
                rows[0].op,
                tabletist_db::FilterOp::Contains,
                "{}",
                look.name
            );
        }
    }

    /// The filter bar of an open table, its operator's menu open. The
    /// operator is `=`, the first of the menu's rows.
    fn operator_menu(look: Look) -> (Harness, crate::model::ConnTabId, crate::model::TabId) {
        let mut harness = Harness::new();
        harness.set_look(look);
        let (tab, id) = harness.editable();
        harness.press(Key::F, Modifiers::COMMAND);
        harness.click("Filter operator");
        (harness, tab, id)
    }

    fn operator(
        harness: &Harness,
        tab: crate::model::ConnTabId,
        id: crate::model::TabId,
    ) -> tabletist_db::FilterOp {
        let workspace = harness.app.workspace(tab).unwrap();
        workspace.object_tab(id).unwrap().filter.rows[0].op
    }

    #[test]
    fn enter_picks_the_row_the_arrows_came_to_and_the_keyboard_is_back_on_the_dropdown() {
        for look in Look::ALL {
            let (mut harness, tab, id) = operator_menu(look);
            give_keyboard(&mut harness, "=");
            harness.press(Key::ArrowDown, Modifiers::NONE);
            assert_eq!(focused_name(&harness.settle()), "≠", "{}", look.name);
            harness.press(Key::Enter, Modifiers::NONE);
            assert_eq!(
                operator(&harness, tab, id),
                tabletist_db::FilterOp::Ne,
                "{}",
                look.name
            );
            assert!(
                !harness.has("contains"),
                "the menu stays open in {}",
                look.name
            );
            assert_eq!(
                focused_name(&harness.settle()),
                "Filter operator",
                "{}",
                look.name
            );
        }
    }

    #[test]
    fn escape_closes_a_dropdowns_menu_and_keeps_its_value() {
        for look in Look::ALL {
            let (mut harness, tab, id) = operator_menu(look);
            give_keyboard(&mut harness, "=");
            harness.press(Key::ArrowDown, Modifiers::NONE);
            harness.press(Key::Escape, Modifiers::NONE);
            assert!(
                !harness.has("contains"),
                "the menu stays open in {}",
                look.name
            );
            assert_eq!(
                operator(&harness, tab, id),
                tabletist_db::FilterOp::Eq,
                "{}",
                look.name
            );
            // The filter bar is open still, and the keyboard is on the
            // dropdown the menu hung under.
            assert_eq!(
                focused_name(&harness.settle()),
                "Filter operator",
                "{}",
                look.name
            );
        }
    }

    #[test]
    fn a_rows_note_is_its_description_for_screen_readers() {
        for look in Look::ALL {
            let mut harness = Harness::new();
            harness.set_look(look);
            let palette = harness.app.palette;
            let tree = harness.frame_with(|ui| {
                super::Item::action("bastion")
                    .detail("bastion.example.com")
                    .show(ui, &look, &palette);
                super::Item::action("plain").show(ui, &look, &palette);
            });
            let described = |name: &str| {
                let id = node(&tree, name, Role::Button).unwrap();
                let (_, node) = tree.nodes.iter().find(|(n, _)| *n == id).unwrap();
                node.description().map(str::to_owned)
            };
            assert_eq!(
                described("bastion").as_deref(),
                Some("bastion.example.com"),
                "{}",
                look.name
            );
            assert_eq!(described("plain"), None, "{}", look.name);
        }
    }

    #[test]
    fn the_keyboard_brings_a_row_that_is_scrolled_away_into_view() {
        for look in Look::ALL {
            let mut harness = Harness::new();
            harness.set_look(look);
            harness.editable();
            harness.press(Key::F, Modifiers::COMMAND);
            harness.click("Filter operator");
            // The last of eleven: under the menu's edge where the rows
            // are taller than a menu grows, and so not drawn.
            let last = "is not NULL";
            if !look.terminal {
                assert_eq!(harness.painted_rect(last), None, "{}", look.name);
            }
            give_keyboard(&mut harness, last);
            assert!(
                harness.painted_rect(last).is_some(),
                "the row with the keyboard is not drawn in {}",
                look.name
            );
        }
    }
}

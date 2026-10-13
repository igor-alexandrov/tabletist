//! The chooser of what labels a table's rows for the foreign keys that
//! point at it: a menu beside a picker's list. Like the list it never
//! takes the keyboard: the picker's field keeps it, and `ui::keys` routes
//! the chooser's keys. What it lists and where its keys are is in
//! `crate::labels`.

use egui::accesskit::Role;
use egui::{Id, Rect, Sense, Ui, UiBuilder, WidgetInfo, WidgetType, pos2, vec2};

use crate::env::{Environment, Platform};
use crate::i18n::{Locale, gettext};
use crate::keymap::{Command, Scope};
use crate::labels::{Excluded, LabelChoice, Line};
use crate::theme::{Look, Palette};
use crate::typography::{Text, TextRole};
use crate::ui::format::display_safe;
use crate::ui::{menu, states, widgets};

/// What an open chooser draws.
pub struct Open {
    /// The line its keys are on.
    pub at: usize,
    pub lines: Vec<Line>,
    /// What is chosen now: the line that is marked.
    pub choice: LabelChoice,
    /// The column the app picks for the table. None where it finds none
    /// to read a row by, and while the columns are not known.
    pub automatic: Option<String>,
    /// The table's columns are not known yet.
    pub loading: bool,
    /// They came while the chooser was open.
    pub arrived: bool,
    /// The table whose label is chosen.
    pub table: String,
    /// The saved connection the choice is kept for: its name, and what it
    /// is.
    pub connection: (String, Environment),
}

/// Where the chooser stands.
#[derive(Debug, Clone, Copy)]
pub enum Place {
    /// Beside a picker's list: to its right where the window has the
    /// room, else to its left.
    Beside(Rect),
    /// Under what opened it in the Structure view, their left edges
    /// together.
    Under(Rect),
}

/// What was asked of the chooser this frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Asked {
    /// The pointer came over this line.
    Point(usize),
    /// This line was clicked.
    Choose(usize),
}

/// What the pointer did to the chooser this frame.
pub struct Shown {
    /// It is pressed or was clicked: the press landed outside the field,
    /// which must keep the keyboard.
    pub pressed: bool,
    pub asked: Option<Asked>,
    /// Where it stands.
    pub rect: Rect,
}

/// The chooser's width, as each look's board has it.
fn width(look: &Look) -> f32 {
    if look.terminal { 380.0 } else { 286.0 }
}

/// The narrowest the chooser is drawn, where the window has less room
/// beside the list than it wants.
const NARROW: f32 = 240.0;

/// The room between the list and the chooser beside it.
fn gap(look: &Look) -> f32 {
    if look.terminal { 8.0 } else { 6.0 }
}

/// What the chooser is called: "Label publishers by". A menu's name for a
/// screen reader, and what the Omarchy look's status line says while it
/// is open.
pub fn title(table: &str, look: &Look, locale: Locale) -> String {
    look.label(&gettext(locale, "Label {table} by"))
        .replace("{table}", &display_safe(table))
}

/// The saved connection as the chooser's foot names it: its name, and
/// what it is where it is anything, in lower case (`Bookshop · dev`).
fn connection_name((name, environment): &(String, Environment), look: &Look) -> String {
    let name = display_safe(name);
    match environment.named(Platform::of(look)) {
        Some(what) => format!("{name} · {what}"),
        None => name.into_owned(),
    }
}

/// Whom the choice is for, and that it stays on this computer.
fn foot_text(open: &Open, look: &Look, locale: Locale) -> String {
    let words = if look.terminal {
        "For {table} in {connection}, on this computer · nothing is written to the database"
    } else {
        "For {table} in {connection}, on this computer. Nothing is written to the database."
    };
    // The app's own words are the look's; the names are as they are.
    look.label(&gettext(locale, words))
        .replace("{table}", &display_safe(&open.table))
        .replace("{connection}", &connection_name(&open.connection, look))
}

/// The keys that move in the chooser, choose and close, as the keymap has
/// them for the look: `↑↓ · ↩ · esc`, `j/k · enter · esc`.
fn keys_line(ui: &Ui, look: &Look) -> String {
    let key = |command| crate::ui::keys::written_in(ui.ctx(), look, command, Scope::Prompt);
    let (next, previous) = (key(Command::NextLabel), key(Command::PreviousLabel));
    let one = |keys: &str| keys.chars().count() == 1;
    let letters = |keys: &str| keys.chars().all(char::is_alphanumeric);
    let moves = if one(&next) && one(&previous) && !letters(&next) {
        // Two arrows stand together, up first.
        format!("{previous}{next}")
    } else if one(&next) && one(&previous) {
        // Two letters, in the order a hand finds them.
        format!("{next}/{previous}")
    } else {
        format!("{previous}/{next}")
    };
    let (choose, close) = (key(Command::ChooseLabel), key(Command::CloseLabelChooser));
    format!("{moves} · {choose} · {close}")
}

/// Why a column cannot be chosen, in the look's words.
pub fn reason(excluded: Excluded, look: &Look, locale: Locale) -> String {
    let words = match excluded {
        Excluded::Key => "the key",
        Excluded::Binary => "binary",
    };
    look.label(&gettext(locale, words))
}

/// One line's words.
struct Row {
    /// What it is: `Automatic`, a column's name.
    text: String,
    /// Muted words after it: what Automatic and No label come to.
    note: Option<String>,
    /// At the row's right: a column's type, and why it cannot be chosen.
    detail: Option<String>,
    /// What a screen reader calls it.
    name: String,
    /// Why it cannot be chosen, where it cannot.
    refused: Option<String>,
}

fn row(line: &Line, open: &Open, look: &Look, locale: Locale) -> Row {
    let say = |words: &'static str| look.label(&gettext(locale, words));
    // The other looks set what a choice comes to off with a dot.
    let lead = if look.terminal { "" } else { "· " };
    match line {
        Line::Automatic => {
            let comes_to = match (&open.automatic, open.loading) {
                (_, true) => say("checking…"),
                (Some(column), false) => {
                    say("uses {column}").replace("{column}", &display_safe(column))
                }
                (None, false) => say("keys only"),
            };
            let text = say("Automatic");
            Row {
                name: format!("{text}, {comes_to}"),
                note: Some(format!("{lead}{comes_to}")),
                text,
                detail: None,
                refused: None,
            }
        }
        Line::NoLabel => {
            let (text, comes_to) = (say("No label"), say("keys only"));
            Row {
                name: format!("{text}, {comes_to}"),
                note: Some(format!("{lead}{comes_to}")),
                text,
                detail: None,
                refused: None,
            }
        }
        Line::Column {
            name,
            type_name,
            excluded,
        } => {
            let (text, kind) = (display_safe(name), display_safe(type_name));
            match excluded {
                None => Row {
                    name: format!("{text}, {kind}"),
                    detail: Some(kind.into_owned()),
                    text: text.into_owned(),
                    note: None,
                    refused: None,
                },
                Some(excluded) => {
                    let why = reason(*excluded, look, locale);
                    // The Omarchy look says first that it is not on offer.
                    let said = if look.terminal {
                        format!("{}, {why}", say("n/a"))
                    } else {
                        why.clone()
                    };
                    Row {
                        name: format!("{text}, {kind}, {why}, {}", say("unavailable")),
                        detail: Some(format!("{kind} · {said}")),
                        text: text.into_owned(),
                        note: None,
                        refused: Some(why),
                    }
                }
            }
        }
    }
}

/// One line of small words across the chooser, cut to `ui`'s width.
fn words(ui: &mut Ui, text: &str, color: egui::Color32, look: &Look) {
    let role = TextRole::pick(look, TextRole::Shortcut, TextRole::OCaption);
    Text::one(look, role, text, color)
        .wrap(ui.available_width())
        .layout(ui.ctx())
        .label(ui);
}

/// Draws the chooser at `place`. `field` is the picker's field, which
/// has the keyboard: its node says which line the keys are on. The
/// Structure view has no field, and the keys are the view's.
pub fn show(
    ui: &Ui,
    open: &Open,
    place: Place,
    field: Option<Id>,
    (look, palette, locale): (&Look, &Palette, Locale),
) -> Shown {
    let frame = menu::frame(look, palette);
    let (wanted, gap) = (width(look), gap(look));
    let screen = ui.ctx().content_rect();
    let (left, top, width) = match place {
        Place::Beside(beside) => {
            // To the list's right where the window has the room, else to
            // its left. Where neither side has it, on the side with more,
            // as wide as that side lets it be and no narrower than a menu
            // reads at: then it stands over the list's edge.
            let right = screen.right() - beside.right() - gap;
            let before = beside.left() - gap - screen.left();
            let (left, width) = if right >= wanted {
                (beside.right() + gap, wanted)
            } else if before >= wanted {
                (beside.left() - gap - wanted, wanted)
            } else if right >= before {
                (beside.right() + gap, right.clamp(NARROW, wanted))
            } else {
                let width = before.clamp(NARROW, wanted);
                (beside.left() - gap - width, width)
            };
            (left, beside.top(), width)
        }
        // From the anchor's left edge, or from its right where that would
        // pass the window's: a button that ends its line.
        Place::Under(anchor) => {
            let left = if anchor.left() + wanted > screen.right() {
                anchor.right() - wanted
            } else {
                anchor.left()
            };
            (left, anchor.bottom() + gap, wanted)
        }
    };
    let id = Id::new("label-chooser");
    let menu_id = id.with("menu");
    let title = title(&open.table, look, locale);
    let muted = if look.terminal {
        palette.dim
    } else {
        palette.secondary
    };
    let inset = if look.terminal { 10.0 } else { 8.0 };
    let mut asked = None;
    let mut pressed = false;
    let moving = ui.input(|input| input.pointer.is_moving());
    let area = egui::Area::new(id)
        .order(egui::Order::Foreground)
        .fade_in(false)
        .constrain(true)
        .fixed_pos(pos2(left, top))
        .show(ui.ctx(), |ui| {
            frame.show(ui, |ui| {
                ui.set_width(width - frame.total_margin().sum().x);
                ui.spacing_mut().item_spacing = egui::Vec2::ZERO;
                let justified = egui::Layout::top_down_justified(egui::Align::LEFT);
                ui.with_layout(justified, |ui| {
                    // The head: what is chosen here, and the keys.
                    let head = ui
                        .horizontal(|ui| {
                            ui.add_space(inset);
                            words(ui, &title, muted, look);
                            let keys = keys_line(ui, look);
                            let right = egui::Layout::right_to_left(egui::Align::Center);
                            ui.with_layout(right, |ui| {
                                ui.add_space(inset);
                                words(ui, &keys, muted, look);
                            });
                        })
                        .response
                        .rect;
                    ui.add_space(4.0);
                    // The menu itself, for a screen reader: its lines are
                    // its own.
                    let whole = ui.interact(head, menu_id, Sense::hover());
                    whole.widget_info(|| WidgetInfo::labeled(WidgetType::Other, true, &title));
                    ui.ctx().accesskit_node_builder(menu_id, |node| {
                        node.set_role(Role::Menu);
                    });
                    let lines = UiBuilder::new().accessibility_parent(menu_id);
                    ui.scope_builder(lines, |ui| {
                        for (index, line) in open.lines.iter().enumerate() {
                            // The columns stand under a word of their own.
                            if index == 2 {
                                section(ui, inset, (look, palette, locale));
                            }
                            let row = row(line, open, look, locale);
                            let current = line.choice().as_ref() == Some(&open.choice);
                            let lit = index == open.at;
                            let mut item = menu::Item::choice(&row.text, current)
                                .name(&row.name)
                                .kept_out(lit);
                            if matches!(line, Line::Column { .. }) {
                                item = item.role(widgets::code(look));
                            }
                            if let Some(note) = &row.note {
                                item = item.note(note);
                            }
                            if let Some(detail) = &row.detail {
                                item = item.detail(detail);
                            }
                            if let Some(why) = &row.refused {
                                item = item.disabled(why);
                            }
                            let response = item.show(ui, look, palette);
                            ui.ctx().accesskit_node_builder(response.id, |node| {
                                node.set_role(Role::MenuItemRadio);
                            });
                            if let (true, Some(field)) = (lit, field) {
                                // The keyboard stays in the field: its node
                                // says which line is the current one.
                                ui.ctx().accesskit_node_builder(field, |node| {
                                    node.set_active_descendant(response.id.accesskit_id());
                                });
                            }
                            pressed |= response.is_pointer_button_down_on() || response.clicked();
                            if response.clicked() {
                                asked = Some(Asked::Choose(index));
                            } else if response.hovered() && moving && !lit && asked.is_none() {
                                asked = Some(Asked::Point(index));
                            }
                        }
                        if open.loading {
                            section(ui, inset, (look, palette, locale));
                            loading(ui, inset, muted, (look, palette, locale));
                        }
                    });
                    // The foot: whom the choice is for.
                    ui.add_space(4.0);
                    let rule = ui.available_rect_before_wrap();
                    ui.painter()
                        .hline(rule.x_range(), rule.top(), (1.0, palette.border));
                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        ui.add_space(inset);
                        ui.scope(|ui| {
                            ui.set_width(ui.available_width() - inset);
                            words(ui, &foot_text(open, look, locale), muted, look);
                        });
                    });
                    ui.add_space(4.0);
                });
            });
        });
    let rect = area.response.rect;
    // A press between its lines chooses nothing, and is the chooser's.
    pressed |= ui.input(|input| {
        let over = input
            .pointer
            .interact_pos()
            .is_some_and(|at| rect.contains(at));
        over && (input.pointer.any_down() || input.pointer.any_click())
    });
    if open.arrived {
        let at = Rect::from_min_size(rect.min, vec2(1.0, 1.0));
        widgets::announce_polite(ui, at, &gettext(locale, "Columns loaded."));
    }
    Shown {
        pressed,
        asked,
        rect,
    }
}

/// The word the columns stand under.
fn section(ui: &mut Ui, inset: f32, (look, palette, locale): (&Look, &Palette, Locale)) {
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        ui.add_space(inset);
        widgets::section_label(&gettext(locale, "Columns"), look, palette)
            .layout(ui.ctx())
            .label(ui);
    });
    ui.add_space(2.0);
}

/// What stands in the columns' place while they are on their way: that
/// they are loading, and that the two lines above can be chosen at once.
fn loading(
    ui: &mut Ui,
    inset: f32,
    muted: egui::Color32,
    (look, palette, locale): (&Look, &Palette, Locale),
) {
    let say = |words: &'static str| look.label(&gettext(locale, words));
    const SPINNER: f32 = 12.0;
    ui.horizontal(|ui| {
        ui.add_space(inset);
        let (ring, _) = ui.allocate_exact_size(vec2(SPINNER, SPINNER), Sense::hover());
        states::spinner(ui, ring, palette.accent, palette);
        ui.add_space(6.0);
        widgets::label(
            ui,
            widgets::body(look),
            &say("Loading columns…"),
            muted,
            look,
        );
    });
    ui.horizontal(|ui| {
        ui.add_space(inset);
        let text = say("Automatic and No label work now");
        widgets::label(ui, widgets::body(look), &text, palette.dim, look);
    });
}

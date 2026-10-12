//! The keyboard shortcuts dialog (`?`): the keymap, as the look in use
//! has it.

use crate::app::App;
use crate::i18n::gettext;
use crate::keymap::{Command, Group, Keymap, Layout, Written};
use crate::model::{Action, Dialog};
use crate::typography::{Text, TextRole};
use crate::ui::widgets;

/// The groups the list is in, in its order, as the Keyboard page of the
/// design names them.
const GROUPS: [(Group, &str); 6] = [
    (Group::Window, "Window and connections"),
    (Group::Navigation, "Navigation"),
    (Group::Editing, "Grid and editing"),
    (Group::Sql, "SQL editor"),
    (Group::Ai, "AI assistant"),
    (Group::Prompts, "Prompts and dialogs"),
];

/// What the list holds for `layout`: every command that is built and has
/// a key there, with its keys as the layout writes them, in the keymap's
/// order. None that is only claimed, and none the layout has no key for.
pub(crate) fn rows(keymap: &Keymap, layout: Layout) -> Vec<(Group, Command, Written)> {
    let mut rows = Vec::new();
    for (group, _) in GROUPS {
        for command in Command::ALL {
            let info = command.info();
            let keys = keymap.label(layout, *command);
            if info.group == group && info.built && !keys.is_empty() {
                rows.push((group, *command, keys));
            }
        }
    }
    rows
}

/// Space between the keys and what they do.
const GAP: f32 = 16.0;

pub fn show(app: &mut App, ctx: &egui::Context) {
    if !matches!(app.dialog, Some(Dialog::Help)) {
        return;
    }
    let locale = app.locale;
    let palette = app.palette;
    let look = app.look;
    let rows = rows(&app.keymap, app.layout());
    let heading = TextRole::pick(&look, TextRole::UiBodySemibold, TextRole::OGroup);
    let mut actions = Vec::new();
    let modal = crate::ui::widgets::modal(egui::Id::new("help"), &look, &palette).show(ctx, |ui| {
        // As wide as its widest keys and description side by side, and no
        // wider or taller than the window leaves room for: the list scrolls
        // when the window is small, so the buttons stay on screen.
        let (mut keys_width, mut what_width) = (0.0_f32, 0.0_f32);
        // The look's own keys: none that only another look has.
        for (_, command, keys) in &rows {
            let keys = Text::one(&look, widgets::code(&look), keys, palette.text);
            let what = Text::one(
                &look,
                widgets::body(&look),
                &gettext(locale, command.info().name),
                palette.text,
            );
            keys_width = keys_width.max(widgets::measure(ui, keys));
            what_width = what_width.max(widgets::measure(ui, what));
        }
        let screen = ui.ctx().content_rect();
        let width = (keys_width + GAP + what_width).ceil();
        ui.set_width(width.min(screen.width() - 80.0).max(120.0));
        widgets::label(
            ui,
            widgets::dialog_title(&look),
            &gettext(locale, "Keyboard shortcuts"),
            palette.text,
            &look,
        );
        ui.add_space(8.0);
        egui::ScrollArea::both()
            .max_height((screen.height() - 180.0).max(120.0))
            .auto_shrink([false, true])
            .show(ui, |ui| {
                egui::Grid::new("shortcuts")
                    .num_columns(2)
                    .spacing([GAP, 6.0])
                    .show(ui, |ui| {
                        let mut last = None;
                        for (group, command, keys) in &rows {
                            // Each group under its name, in the first
                            // column: the keys' own.
                            if last != Some(*group) {
                                last = Some(*group);
                                let named = GROUPS.iter().find(|(of, _)| of == group);
                                let name = named.map_or("", |(_, name)| name);
                                let name = look.label(&gettext(locale, name));
                                widgets::label(ui, heading, &name, palette.dim, &look);
                                ui.end_row();
                            }
                            widgets::label(
                                ui,
                                widgets::code(&look),
                                keys,
                                palette.secondary,
                                &look,
                            );
                            widgets::label(
                                ui,
                                widgets::body(&look),
                                &gettext(locale, command.info().name),
                                palette.text,
                                &look,
                            );
                            ui.end_row();
                        }
                    });
            });
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            if widgets::button(ui, &gettext(locale, "About Tabletist"), &look).clicked() {
                actions.push(Action::ShowAbout);
            }
            if widgets::button(ui, &gettext(locale, "Settings"), &look).clicked() {
                actions.push(Action::ShowSettings);
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if widgets::button(ui, &gettext(locale, "Close"), &look).clicked() {
                    actions.push(Action::CloseDialog);
                }
            });
        });
    });
    if modal.is_top_modal
        && ctx.input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::Escape))
    {
        actions.push(Action::CloseDialog);
    }
    app.actions.extend(actions);
}

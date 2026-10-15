//! The terminal look's box of what agents may do on a saved connection:
//! `a` on a row of the Connections list opens it. The levels, the one in
//! force marked, `j` and `k` among them, `enter` to set one, `t` for the
//! connection's Agents tab, `esc` to leave it as it was.

use egui::{Id, Key, Margin, Modifiers, Rect, Sense, vec2};

use crate::app::App;
use crate::connections::AgentAccess;
use crate::i18n::gettext;
use crate::keymap::{Command, Layout, When};
use crate::model::{Action, Dialog};
use crate::typography::{Text, TextRole};
use crate::ui::{terminal_dialog, widgets};

/// The levels, as the box lists them.
const LEVELS: [(AgentAccess, &str); 2] = [(AgentAccess::Off, "off"), (AgentAccess::Read, "read")];

pub fn show(app: &mut App, ctx: &egui::Context) {
    let Some(Dialog::AgentAccess(asked)) = &app.dialog else {
        return;
    };
    let (conn, name, chosen) = (asked.conn.clone(), asked.name.clone(), asked.chosen);
    let (look, palette, locale) = (app.look, app.palette, app.locale);
    let Some(current) = app
        .connections
        .get(&conn)
        .map(|saved| app.agents_access(saved))
    else {
        // Deleted while it was open: nothing to set.
        app.actions.push(Action::CloseDialog);
        return;
    };
    let mut actions = Vec::new();
    // The letters, as typed text: the box's own, by the keymap.
    let (keymap, layout) = (crate::ui::keys::published(ctx), Layout::of(&look));
    let typed = ctx.input_mut(|input| {
        let mut typed = Vec::new();
        input.events.retain(|event| match event {
            egui::Event::Text(text) => {
                match crate::ui::keys::prompt_letter(&keymap, layout, When::AgentAccess, text) {
                    Some(command) => {
                        typed.push((command, text.clone()));
                        false
                    }
                    None => true,
                }
            }
            _ => true,
        });
        typed
    });
    for (command, text) in typed {
        match command {
            Command::MoveInAgentLevels => {
                let step = if text == "k" { -1 } else { 1 };
                actions.push(Action::MoveAgentAccess(step));
            }
            Command::ShowAgentsFromBox => {
                actions.push(Action::CloseDialog);
                actions.push(Action::ShowAgentsOn(conn.clone()));
            }
            _ => {}
        }
    }
    let set = |actions: &mut Vec<Action>, level: AgentAccess| {
        if level != current {
            actions.push(Action::SetAgentAccess {
                conn: conn.clone(),
                access: level,
            });
        }
        actions.push(Action::CloseDialog);
    };
    // A plain Enter that went down here: with a modifier it is no answer,
    // and what a held one repeats answers nothing either.
    ctx.input_mut(|input| {
        crate::ui::keys::drop_repeats(input, Key::Enter);
        crate::ui::keys::drop_repeats(input, Key::Escape);
    });
    if ctx.input_mut(crate::ui::keys::take_enter) {
        set(&mut actions, chosen);
    }
    if ctx.input_mut(|input| input.consume_key(Modifiers::NONE, Key::Escape)) {
        actions.push(Action::CloseDialog);
    }
    let frame = widgets::modal_frame(&look, &palette).inner_margin(Margin::ZERO);
    let radius = look.dialog_radius.saturating_sub(frame.stroke.width as u8);
    let say = |text: &'static str| look.label(&gettext(locale, text));
    widgets::modal(Id::new("agent-access"), &look, &palette)
        .frame(frame)
        .show(ctx, |ui| {
            ui.set_width(320.0);
            ui.spacing_mut().item_spacing = egui::Vec2::ZERO;
            let head = terminal_dialog::head(ui, radius, palette.panel, &palette);
            let title = format!("{} · {name}", say("Agents"));
            widgets::paint_text(
                ui,
                head.left() + 18.0,
                terminal_dialog::head_line(head),
                Text::one(&look, TextRole::OGroup, &title, palette.text),
            );
            widgets::announce(ui, head, &title);
            ui.add_space(8.0);
            let row = TextRole::OBody.row_height(ui.ctx(), look.faces) + 6.0;
            for (level, word) in LEVELS {
                let (rect, response) =
                    ui.allocate_exact_size(vec2(ui.available_width(), row), Sense::click());
                if level == chosen {
                    ui.painter().rect_filled(rect, 0.0, palette.selection);
                    let edge = Rect::from_min_size(rect.min, vec2(2.0, rect.height()));
                    ui.painter().rect_filled(edge, 0.0, palette.accent);
                }
                // The level in force is checked: painted, as not every
                // face has the glyph.
                if level == current {
                    let place = Rect::from_center_size(
                        egui::pos2(rect.left() + 24.0, rect.center().y),
                        vec2(12.0, 12.0),
                    );
                    crate::theme::Icon::Check
                        .image(palette.text, 12.0)
                        .paint_at(ui, place);
                }
                widgets::paint_text(
                    ui,
                    rect.left() + 38.0,
                    rect.center().y,
                    Text::one(&look, TextRole::OBody, &say(word), palette.text),
                );
                widgets::announce(ui, rect, &format!("{} {}", say("Agents"), say(word)));
                if response.clicked() {
                    set(&mut actions, level);
                }
            }
            ui.add_space(8.0);
            let foot = terminal_dialog::foot(ui, radius, &palette);
            let opens = crate::ui::keys::written(ui.ctx(), &look, Command::ChangeAgentAccess);
            let moves = crate::ui::keys::written(ui.ctx(), &look, Command::MoveInAgentLevels);
            let tab = crate::ui::keys::written(ui.ctx(), &look, Command::ShowAgentsFromBox);
            let words = format!(
                "{opens} {} · {moves} · {tab} {}",
                say("opens this"),
                say("agents tab")
            );
            widgets::paint_text(
                ui,
                foot.left() + 18.0,
                terminal_dialog::foot_line(foot),
                Text::one(&look, widgets::secondary(&look), &words, palette.dim),
            );
        });
    app.actions.extend(actions);
}

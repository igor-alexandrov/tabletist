//! Settings › Updates as the terminal look draws it: the status block
//! with its keys, the row of how often a check runs, and what a package
//! manager's copy does, with what the check sends.

use egui::accesskit::Live;
use egui::{CornerRadius, Rect, Sense, Stroke, StrokeKind, Ui, pos2, vec2};

use crate::app::App;
use crate::i18n::gettext;
use crate::keymap::Command;
use crate::model::Action;
use crate::settings::{OptionId, Settings};
use crate::typography::{Text, TextRole};
use crate::ui::widgets;
use crate::update::release;

use super::terminal::{ROW_SIDE, Skin, columns, heading, key, option_row, widths};
use super::updates::{Card, announce_live, released, when};

/// What the note under the row says, word for word as the board has it.
const NOTE: &str = "installed with a package manager (pacman, an AUR helper, nix, cargo install)? then that tool updates tabletist: this pane only checks and shows the command, for example";
const NOTE_COMMAND: &str = "cargo install tabletist --locked";
const PRIVACY: &str = "the check asks github for the latest release and sends this version, os and cpu, nothing else.";

/// The pane: the block, then its one row under `automatic`, then the
/// note. The cursor's row is the pane's only one.
pub(super) fn rows(
    ui: &mut Ui,
    app: &App,
    settings: &Settings,
    cursor: usize,
    skin: &Skin,
    actions: &mut Vec<Action>,
) {
    block(ui, app, skin);
    heading(ui, "Automatic", 8.0, skin);
    let room = ui.available_width() - 2.0 * ROW_SIDE;
    let columns = columns(
        ui,
        room,
        widths(ui, &OptionId::UPDATES, settings, skin),
        skin,
    );
    option_row(
        ui,
        (0, OptionId::UpdatesCheck),
        cursor == 0,
        columns,
        settings,
        skin,
        actions,
    );
    ui.add_space(14.0);
    note(ui, skin);
}

/// One line of the block: its pieces, each its text and colour.
type Line = Vec<(String, egui::Color32)>;

/// The status block: what the card of the other looks says, a line at a
/// time, with the pane's keys that act on it.
fn block(ui: &mut Ui, app: &App, skin: &Skin) {
    let Skin {
        look,
        palette,
        locale,
        ..
    } = *skin;
    let shown = Card::of(app);
    let now = app.now_secs();
    let current = crate::update::CURRENT;
    // As the board writes it here: `0.4.0 available`.
    let title = match shown {
        Card::Available(offer) | Card::NoBuild(offer) => {
            gettext(locale, "{version} available").replace("{version}", &offer.version)
        }
        _ => look.label(&shown.title(locale)),
    };
    let title_color = match shown {
        Card::Failed { .. } => palette.danger,
        Card::UpToDate { .. } => palette.success,
        _ => palette.accent,
    };
    let say = |text: &'static str| skin.say(text);
    let mut sub: Vec<String> = Vec::new();
    let mut lines: Vec<Line> = Vec::new();
    match shown {
        Card::NotChecked => sub.push(say("not checked yet")),
        Card::Checking => sub.push(crate::update::check::ASKED.to_owned()),
        Card::UpToDate { checked } => sub.push(look.label(
            &gettext(locale, "Checked {when}").replace("{when}", &when(locale, checked, now)),
        )),
        Card::Available(offer) | Card::NoBuild(offer) => {
            sub.push(
                look.label(&gettext(locale, "You have {version}").replace("{version}", current)),
            );
            if let Some(at) = offer.published {
                sub.push(look.label(&released(locale, at, now)));
            }
            if let Some((_, size)) = offer.download {
                sub.push(look.label(&release::megabytes(size)));
            }
            sub.push(format!(
                "{} {}",
                std::env::consts::OS,
                std::env::consts::ARCH
            ));
            for point in &offer.notes {
                let plain: String = release::runs(point)
                    .into_iter()
                    .map(|(run, _)| run)
                    .collect();
                lines.push(vec![(format!("  - {}", look.label(&plain)), palette.text)]);
            }
            if matches!(shown, Card::NoBuild(_)) {
                let said = gettext(locale, "{version} has no build for this computer yet")
                    .replace("{version}", &offer.version);
                lines.push(vec![(look.label(&said), palette.warning)]);
            }
        }
        Card::Elsewhere(_, managed) => {
            sub.push(say(managed.said));
            if let Some(command) = managed.command {
                lines.push(vec![(format!("  {command}"), palette.text)]);
            }
        }
        Card::Failed { failed, last } => {
            sub.push(match last {
                Some(at) => look.label(
                    &gettext(locale, "Last successful check: {when}")
                        .replace("{when}", &when(locale, at, now)),
                ),
                None => say("no check has worked yet"),
            });
            let said = failed.with_numbers(&gettext(locale, failed.said()));
            lines.push(vec![(look.label(&said), palette.danger)]);
        }
    }
    // The pane's keys that act on what the block shows.
    let keys: Vec<(Command, String)> = match shown {
        Card::NotChecked | Card::UpToDate { .. } => {
            vec![(Command::SettingsCheckNow, say("check now"))]
        }
        Card::Failed { .. } => vec![(Command::SettingsCheckNow, say("try again"))],
        Card::Checking => Vec::new(),
        Card::Available(offer) | Card::NoBuild(offer) => vec![
            (Command::SettingsReleaseNotes, say("release notes")),
            (
                Command::SettingsSkipVersion,
                format!("{} {}", say("skip"), offer.version),
            ),
        ],
        Card::Elsewhere(_, managed) => match managed.command {
            Some(_) => vec![
                (Command::SettingsCopyCommand, say("copy")),
                (Command::SettingsReleaseNotes, say("release notes")),
            ],
            None => vec![(Command::SettingsReleaseNotes, say("release notes"))],
        },
    };
    let mut keyed: Line = Vec::new();
    for (index, (command, what)) in keys.iter().enumerate() {
        if index > 0 {
            keyed.push(("  ".to_owned(), palette.dim));
        }
        keyed.push((key(ui, look, *command).as_str().to_owned(), palette.accent));
        keyed.push((format!(" {what}"), palette.text));
    }

    let role = TextRole::OBody;
    let line = (role.row_height(ui.ctx(), look.faces) * 1.35).round();
    let count = 1 + lines.len() + usize::from(!keyed.is_empty());
    let height = count as f32 * line + 20.0 + if keyed.is_empty() { 0.0 } else { 6.0 };
    let (rect, _) =
        ui.allocate_exact_size(vec2(ui.available_width(), height + 14.0), Sense::hover());
    let band = Rect::from_min_max(
        pos2(rect.left() + ROW_SIDE, rect.top() + 8.0),
        pos2(
            (rect.right() - ROW_SIDE).max(rect.left() + ROW_SIDE),
            rect.bottom() - 6.0,
        ),
    );
    ui.painter()
        .rect_filled(band, CornerRadius::ZERO, palette.panel);
    ui.painter().rect_stroke(
        band,
        CornerRadius::ZERO,
        Stroke::new(1.0, palette.accent),
        StrokeKind::Inside,
    );
    // The region, named; its lines are its own.
    let region_id = ui.id().with("update-status");
    let region = ui.interact(band, region_id, Sense::hover());
    let named = skin.say("Update status");
    region.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Other, true, &named));
    ui.ctx().accesskit_node_builder(region_id, |node| {
        node.set_role(egui::accesskit::Role::Region);
    });
    let inside = egui::UiBuilder::new()
        .accessibility_parent(region_id)
        .max_rect(band);
    let mut inside = ui.new_child(inside);
    let ui = &mut inside;
    let left = band.left() + 16.0;
    let right = band.right() - 16.0;
    let mut y = band.top() + 10.0 + line / 2.0;
    // A line is cut at the block's edge, and read whole.
    let ctx = ui.ctx().clone();
    let cut = |text: &str, room: f32| {
        crate::ui::grid::ellipsize(text, room.max(0.0), false, |text| {
            role.width(&ctx, look.faces, text)
        })
        .into_owned()
    };
    // The title, strong, and what it says beside it.
    let strong = Text::one(look, TextRole::OBody, &title, title_color).layout(ui.ctx());
    strong.paint_left(ui.painter(), left, y);
    // Told politely as it changes; an error is told by its own line.
    announce_live(
        ui,
        Rect::from_center_size(pos2(left + strong.width() / 2.0, y), strong.size()),
        (&title, "title"),
        Live::Polite,
    );
    if !sub.is_empty() {
        let said = format!("  {}", sub.join(" · "));
        let shown_sub = cut(&said, right - left - strong.width());
        let width = widgets::paint_text(
            ui,
            left + strong.width(),
            y,
            Text::one(look, role, &shown_sub, palette.dim),
        );
        widgets::announce(
            ui,
            Rect::from_min_size(
                pos2(left + strong.width(), y - line / 2.0),
                vec2(width.max(1.0), line),
            ),
            said.trim(),
        );
    }
    for pieces in lines.iter().chain((!keyed.is_empty()).then_some(&keyed)) {
        y += line;
        if std::ptr::eq(pieces, &keyed) {
            y += 6.0;
        }
        let mut text = Text::new(look);
        for (piece, color) in pieces {
            // A line of one piece is a note or what is said: cut to fit.
            let piece = match pieces.len() {
                1 => cut(piece, right - left),
                _ => piece.clone(),
            };
            text = text.add(role, &piece, *color);
        }
        let laid = text.layout(ui.ctx());
        laid.paint_left(ui.painter(), left, y);
        let said: String = pieces.iter().map(|(piece, _)| piece.as_str()).collect();
        let place = Rect::from_center_size(
            pos2(left + laid.width() / 2.0, y),
            vec2(laid.width().max(1.0), line),
        );
        let error = matches!(shown, Card::Failed { .. })
            && pieces
                .first()
                .is_some_and(|(_, color)| *color == palette.danger);
        if error {
            announce_live(ui, place, (said.trim(), "error"), Live::Assertive);
        } else {
            widgets::announce(ui, place, said.trim());
        }
    }
}

/// What a package manager's copy does, and what the check sends, in a
/// box under the row.
fn note(ui: &mut Ui, skin: &Skin) {
    let Skin { look, palette, .. } = *skin;
    let role = TextRole::OBody;
    let width = (ui.available_width() - 2.0 * ROW_SIDE - 24.0).max(0.0);
    let first = Text::new(look)
        .add(role, &skin.say(NOTE), palette.dim)
        .space(role, " ")
        .add(role, NOTE_COMMAND, palette.text)
        .add(role, ".", palette.dim)
        .wrap(width)
        .layout(ui.ctx());
    let second = Text::one(look, role, &skin.say(PRIVACY), palette.dim)
        .wrap(width)
        .layout(ui.ctx());
    let height = first.height() + second.height() + 20.0;
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::hover());
    let band = Rect::from_min_max(
        pos2(rect.left() + ROW_SIDE, rect.top()),
        pos2(
            (rect.right() - ROW_SIDE).max(rect.left() + ROW_SIDE),
            rect.bottom(),
        ),
    );
    ui.painter().rect_stroke(
        band,
        CornerRadius::ZERO,
        Stroke::new(1.0, palette.outline),
        StrokeKind::Inside,
    );
    let at = pos2(band.left() + 12.0, band.top() + 10.0);
    first.paint(ui.painter(), at);
    widgets::announce(
        ui,
        Rect::from_min_size(at, first.size()),
        first.galley.text(),
    );
    let at = pos2(at.x, at.y + first.height());
    second.paint(ui.painter(), at);
    widgets::announce(
        ui,
        Rect::from_min_size(at, second.size()),
        second.galley.text(),
    );
}

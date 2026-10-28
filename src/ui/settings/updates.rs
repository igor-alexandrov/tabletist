//! Settings › Updates: the status card that says where updates stand,
//! how often a check runs, and what the check sends. What the card says
//! is worked out here for both looks (`Card`); the sheet's drawing is
//! here too, the terminal look's in `updates_terminal.rs`.

use std::time::{Duration, SystemTime};

use egui::accesskit::{Live, Role};
use egui::{CornerRadius, Rect, Sense, Stroke, StrokeKind, Ui, pos2, vec2};

use crate::app::App;
use crate::clock::Day;
use crate::i18n::{Locale, gettext};
use crate::model::Action;
use crate::settings::OptionId;
use crate::theme::Icon;
use crate::typography::{Laid, Text, TextRole};
use crate::ui::widgets::{self, ButtonSpec};
use crate::update::{Failed, Installed, Managed, Offer, Phase, release};

use super::sheet::Skin;

/// Where updates stand, one state at a time, as the status card shows it.
#[derive(Clone, Copy, Debug)]
pub(super) enum Card<'a> {
    /// No check has worked yet, and none runs.
    NotChecked,
    Checking,
    UpToDate {
        checked: u64,
    },
    /// A newer version this copy can take.
    Available(&'a Offer),
    /// A newer version with no download for this computer.
    NoBuild(&'a Offer),
    /// A newer version that something else installs.
    Elsewhere(&'a Offer, &'a Managed),
    Failed {
        failed: &'a Failed,
        last: Option<u64>,
    },
}

impl<'a> Card<'a> {
    pub(super) fn of(app: &'a App) -> Self {
        let updates = &app.updates;
        if updates.checking() {
            return Self::Checking;
        }
        let last = updates.seen.last_checked;
        if let Phase::Failed {
            failed,
            asked_for: true,
        } = &updates.phase
        {
            return Self::Failed { failed, last };
        }
        if let Some(offer) = updates.offer(app.settings.update_skip.as_deref()) {
            return match &updates.installed {
                Some(Installed::Elsewhere(managed)) => Self::Elsewhere(offer, managed),
                _ if offer.download.is_none() => Self::NoBuild(offer),
                Some(Installed::Itself(_)) | None => Self::Available(offer),
            };
        }
        match (&updates.phase, last) {
            (Phase::Failed { failed, .. }, last) => Self::Failed { failed, last },
            (Phase::Idle | Phase::Checking { .. }, Some(checked)) => Self::UpToDate { checked },
            (Phase::Idle | Phase::Checking { .. }, None) => Self::NotChecked,
        }
    }

    /// The card's title, translated: what a screen reader is told as it
    /// changes.
    pub(super) fn title(self, locale: Locale) -> String {
        let current = crate::update::CURRENT;
        let (words, version) = match self {
            Self::NotChecked => ("Tabletist {version}", current),
            Self::Checking => ("Checking for updates…", current),
            Self::UpToDate { .. } => ("Tabletist {version} is up to date", current),
            Self::Available(offer) | Self::NoBuild(offer) => {
                ("Tabletist {version} is available", offer.version.as_str())
            }
            Self::Elsewhere(offer, _) => ("{version} is available", offer.version.as_str()),
            Self::Failed { .. } => ("Couldn't check for updates", current),
        };
        gettext(locale, words).replace("{version}", version)
    }
}

/// Whether a newer version is offered, whatever the card says while a
/// check runs or after one failed: the dot on Updates in the nav.
pub(super) fn offered(app: &App) -> bool {
    app.updates
        .offer(app.settings.update_skip.as_deref())
        .is_some()
}

/// The time `at` (seconds since the Unix epoch) as the card says it, by
/// the clock as it is `now`: `today at 09:12`, `yesterday at 09:05`, `on
/// Oct 21 at 09:05`. Translated.
pub(super) fn when(locale: Locale, at: u64, now: u64) -> String {
    let (at, now) = (time_of(at), time_of(now));
    let hour = crate::clock::hour(at);
    let (words, day) = match crate::clock::day_of(at, now) {
        Day::Today => ("today at {time}", String::new()),
        Day::Yesterday => ("yesterday at {time}", String::new()),
        Day::On(day) => ("on {day} at {time}", day),
    };
    gettext(locale, words)
        .replace("{time}", &hour)
        .replace("{day}", &day)
}

/// The day a release came out, as the card says it: `released today`.
pub(super) fn released(locale: Locale, at: u64, now: u64) -> String {
    let (words, day) = match crate::clock::day_of(time_of(at), time_of(now)) {
        Day::Today => ("released today", String::new()),
        Day::Yesterday => ("released yesterday", String::new()),
        Day::On(day) => ("released {day}", day),
    };
    gettext(locale, words).replace("{day}", &day)
}

/// `secs` as a time; one past what the system's clock holds is the epoch,
/// so the card never panics on it.
fn time_of(secs: u64) -> SystemTime {
    SystemTime::UNIX_EPOCH
        .checked_add(Duration::from_secs(secs))
        .unwrap_or(SystemTime::UNIX_EPOCH)
}

/// When the last check that worked ran, as the row under the card says
/// it: `Last checked today at 09:12`, or in the terminal look `last
/// checked 09:12`. None before any has.
pub(crate) fn checked_text(app: &App, terminal: bool) -> Option<String> {
    let at = app.updates.seen.last_checked?;
    let now = app.now_secs();
    if terminal {
        let at = time_of(at);
        let time = match crate::clock::day_of(at, time_of(now)) {
            Day::Today => crate::clock::hour(at),
            Day::Yesterday | Day::On(_) => crate::clock::day(at),
        };
        let said = gettext(app.locale, "last checked {time}").replace("{time}", &time);
        return Some(app.look.label(&said));
    }
    Some(gettext(app.locale, "Last checked {when}").replace("{when}", &when(app.locale, at, now)))
}

/// What the pane says under its title: on Windows no release is
/// notarized.
const ABOUT: &str = if cfg!(windows) {
    "New versions come from Tabletist's GitHub releases, signed. Nothing installs without asking."
} else {
    "New versions come from Tabletist's GitHub releases, signed and notarized. Nothing installs without asking."
};

/// What the check sends, word for word as the pane says it: on Windows
/// the User-Agent names the system, not its version.
pub(super) const PRIVACY: &str = if cfg!(windows) {
    "The check asks GitHub for the latest release. It sends this version, Windows and the processor, nothing else: no ID, no usage data, no database details."
} else {
    "The check asks GitHub for the latest release. It sends this version, the macOS version and the chip, nothing else: no ID, no usage data, no database details."
};

/// The space inside the card, at its sides and over and under.
const PAD: f32 = 16.0;
/// The icon's tile, and the gap after it.
const TILE: f32 = 40.0;
const TILE_GAP: f32 = 14.0;
/// Between the card's lines.
const LINE_GAP: f32 = 6.0;

/// The pane, from the top of `ui` down.
pub(super) fn content(ui: &mut Ui, app: &App, limit: f32, skin: &Skin, actions: &mut Vec<Action>) {
    let Skin { look, palette, .. } = *skin;
    let width = ui.available_width();
    widgets::label(
        ui,
        TextRole::DialogTitle,
        &skin.say("Updates"),
        palette.text,
        look,
    );
    ui.add_space(2.0);
    Text::one(look, TextRole::UiBody, &skin.say(ABOUT), palette.dim)
        .wrap(width)
        .layout(ui.ctx())
        .label(ui);
    ui.add_space(10.0);
    let room = (limit - ui.cursor().top()).max(56.0);
    egui::ScrollArea::vertical()
        .id_salt("updates")
        .max_height(room)
        .auto_shrink([false, true])
        .show(ui, |ui| {
            card(ui, app, skin, actions);
            ui.add_space(14.0);
            widgets::section_label(&skin.say("Automatic"), look, palette)
                .layout(ui.ctx())
                .label(ui);
            ui.add_space(2.0);
            check_row(ui, app, skin, actions);
            ui.add_space(18.0);
            tip(ui, skin);
        });
}

/// The status card: an icon, the title, what it says under it, and its
/// actions. A region named Update status, whose title a screen reader is
/// told of as it changes, and an error at once.
fn card(ui: &mut Ui, app: &App, skin: &Skin, actions: &mut Vec<Action>) {
    let Skin {
        look,
        palette,
        locale,
        ..
    } = *skin;
    let shown = Card::of(app);
    let now = app.now_secs();
    let width = ui.available_width();
    let words = (width - 2.0 * PAD - TILE - TILE_GAP).max(0.0);
    let title = shown.title(locale);
    // What it says under the title, then its notes or its box.
    let sub = match shown {
        Card::NotChecked => skin.say("Not checked yet"),
        Card::Checking => crate::update::check::ASKED.to_owned(),
        Card::UpToDate { checked } => {
            gettext(locale, "Checked {when}").replace("{when}", &when(locale, checked, now))
        }
        Card::Available(offer) | Card::NoBuild(offer) => {
            let mut parts = vec![
                gettext(locale, "You have {version}").replace("{version}", crate::update::CURRENT),
            ];
            if let Some(at) = offer.published {
                parts.push(released(locale, at, now));
            }
            if let Some((_, size)) = offer.download {
                parts.push(release::megabytes(size));
            }
            parts.join(" · ")
        }
        Card::Elsewhere(_, managed) => skin.say(managed.said),
        Card::Failed { last, .. } => match last {
            Some(at) => gettext(locale, "Last successful check: {when}")
                .replace("{when}", &when(locale, at, now)),
            None => skin.say("No check has worked yet"),
        },
    };
    let title_laid = Text::one(look, TextRole::StateTitle, &title, palette.text)
        .wrap(words)
        .layout(ui.ctx());
    let sub_laid = Text::one(look, TextRole::Secondary, &sub, palette.dim)
        .wrap(words)
        .layout(ui.ctx());
    let notes: Vec<Laid> = match shown {
        Card::Available(offer) => offer
            .notes
            .iter()
            .map(|point| note(point, words - 18.0, skin).layout(ui.ctx()))
            .collect(),
        _ => Vec::new(),
    };
    // The box under the line: an error, the release with no build, or the
    // command another tool updates with.
    let boxed = match shown {
        Card::Failed { failed, .. } => {
            let said = failed.with_numbers(&gettext(locale, failed.said()));
            Some((said, palette.danger))
        }
        Card::NoBuild(offer) => Some((
            gettext(locale, "{version} has no build for this computer yet")
                .replace("{version}", &offer.version),
            palette.warning,
        )),
        _ => None,
    };
    let boxed_laid = boxed.as_ref().map(|(said, color)| {
        Text::one(look, TextRole::Secondary, said, *color)
            .wrap((words - 20.0).max(0.0))
            .layout(ui.ctx())
    });
    let command = match shown {
        Card::Elsewhere(_, managed) => managed.command,
        _ => None,
    };
    let line = TextRole::Secondary.row_height(ui.ctx(), look.faces);
    let mut inner = title_laid.height() + LINE_GAP + sub_laid.height();
    for laid in &notes {
        inner += 2.0 + laid.height();
    }
    if let Some(laid) = &boxed_laid {
        inner += LINE_GAP + laid.height() + 16.0;
    }
    if command.is_some() {
        inner += LINE_GAP + look.control_height;
    }
    let has_actions = command.is_none();
    if has_actions {
        inner += LINE_GAP + 4.0 + look.control_height;
    }
    let height = (inner.max(TILE) + 2.0 * PAD).ceil();
    let (rect, _) = ui.allocate_exact_size(vec2(width, height), Sense::hover());
    let corner = CornerRadius::same(look.radius.saturating_add(4));
    ui.painter().rect_filled(rect, corner, palette.window);
    ui.painter().rect_stroke(
        rect,
        corner,
        Stroke::new(1.0, palette.surface),
        StrokeKind::Inside,
    );
    // The region, named; what is in it is its own.
    let region_id = ui.id().with("update-status");
    let region = ui.interact(rect, region_id, Sense::hover());
    let named = skin.say("Update status");
    region.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Other, true, &named));
    ui.ctx().accesskit_node_builder(region_id, |node| {
        node.set_role(Role::Region);
    });
    let inside = egui::UiBuilder::new()
        .accessibility_parent(region_id)
        .max_rect(rect);
    let mut inside = ui.new_child(inside);
    let ui = &mut inside;

    // The icon in its tile.
    let tile = Rect::from_min_size(pos2(rect.left() + PAD, rect.top() + PAD), vec2(TILE, TILE));
    let (icon, tint) = match shown {
        Card::NotChecked => (Some(Icon::Info), palette.secondary),
        Card::Checking => (None, palette.dim),
        Card::UpToDate { .. } => (Some(Icon::CircleCheck), palette.success),
        Card::Available(_) | Card::NoBuild(_) => (Some(Icon::Download), palette.accent),
        Card::Elsewhere(..) => (Some(Icon::Download), palette.secondary),
        Card::Failed { .. } => (Some(Icon::CircleAlert), palette.danger),
    };
    let fill = match shown {
        Card::Checking | Card::Elsewhere(..) | Card::NotChecked => palette.surface,
        _ => tint.gamma_multiply(0.14),
    };
    ui.painter().rect_filled(tile, CornerRadius::same(10), fill);
    match icon {
        Some(icon) => icon
            .image(tint, 22.0)
            .paint_at(ui, Rect::from_center_size(tile.center(), vec2(22.0, 22.0))),
        None => crate::ui::states::spinner(
            ui,
            Rect::from_center_size(tile.center(), vec2(16.0, 16.0)),
            palette.accent,
            palette,
        ),
    }

    let left = tile.right() + TILE_GAP;
    let mut y = rect.top() + PAD;
    title_laid.paint(ui.painter(), pos2(left, y));
    // Told politely as it changes; an error is told by its own line.
    announce_live(
        ui,
        Rect::from_min_size(pos2(left, y), title_laid.size()),
        (&title, "title"),
        Live::Polite,
    );
    y += title_laid.height() + LINE_GAP;
    paint(ui, &sub_laid, pos2(left, y));
    y += sub_laid.height();
    if !notes.is_empty() {
        y += 2.0;
        let list_id = ui.id().with("update-notes");
        let list_rect = Rect::from_min_size(
            pos2(left, y),
            vec2(words, notes.iter().map(|laid| laid.height() + 2.0).sum()),
        );
        let list = ui.interact(list_rect, list_id, Sense::hover());
        let named = skin.say("What's new");
        list.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Other, true, &named));
        ui.ctx().accesskit_node_builder(list_id, |node| {
            node.set_role(Role::List);
        });
        // The items are the list's children, not the card's.
        let items = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(list_rect)
                .accessibility_parent(list_id),
        );
        for (index, laid) in notes.iter().enumerate() {
            let dot = pos2(left + 6.0, y + laid.height().min(line) / 2.0);
            ui.painter().circle_filled(dot, 2.0, palette.secondary);
            laid.paint(ui.painter(), pos2(left + 18.0, y));
            let item = items.interact(
                Rect::from_min_size(pos2(left + 18.0, y), laid.size()),
                list_id.with(index),
                Sense::hover(),
            );
            let text = laid.galley.text().to_owned();
            item.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Label, true, &text));
            ui.ctx().accesskit_node_builder(item.id, |node| {
                node.set_role(Role::ListItem);
            });
            y += laid.height() + 2.0;
        }
    }
    if let Some(laid) = &boxed_laid {
        y += LINE_GAP;
        let color = boxed.as_ref().map_or(palette.danger, |(_, color)| *color);
        let band = Rect::from_min_size(pos2(left, y), vec2(words, laid.height() + 16.0));
        ui.painter()
            .rect_filled(band, CornerRadius::same(8), color.gamma_multiply(0.1));
        ui.painter().rect_stroke(
            band,
            CornerRadius::same(8),
            Stroke::new(1.0, color.gamma_multiply(0.35)),
            StrokeKind::Inside,
        );
        laid.paint(ui.painter(), pos2(left + 10.0, y + 8.0));
        // Told once, at once, where it is an error.
        let said = laid.galley.text().to_owned();
        match shown {
            Card::Failed { .. } => announce_live(ui, band, (&said, "error"), Live::Assertive),
            _ => widgets::announce(ui, band, &said),
        }
        y = band.bottom();
    }
    if let Some(command) = command {
        y += LINE_GAP;
        command_box(
            ui,
            Rect::from_min_size(pos2(left, y), vec2(words, look.control_height)),
            command,
            skin,
        );
    }
    if has_actions {
        y += LINE_GAP + 4.0;
        let place = Rect::from_min_size(pos2(left, y), vec2(words, look.control_height));
        let centred = egui::Layout::left_to_right(egui::Align::Center);
        let builder = egui::UiBuilder::new()
            .id_salt("update-actions")
            .max_rect(place);
        let mut row = ui.new_child(builder.layout(centred));
        row.spacing_mut().item_spacing.x = 8.0;
        card_actions(&mut row, shown, skin, actions);
    }
}

/// A point of the notes, its strong runs strong.
fn note(point: &str, width: f32, skin: &Skin) -> Text {
    let Skin { look, palette, .. } = *skin;
    let mut text = Text::new(look);
    for (run, strong) in release::runs(point) {
        let role = if strong {
            TextRole::UiBodyStrong
        } else {
            TextRole::UiBody
        };
        let color = if strong {
            palette.text
        } else {
            palette.secondary
        };
        text = text.add(role, &run, color);
    }
    text.wrap(width.max(0.0))
}

/// The card's buttons and links, by its state.
fn card_actions(ui: &mut Ui, shown: Card<'_>, skin: &Skin, actions: &mut Vec<Action>) {
    let Skin {
        look,
        palette,
        locale,
        ..
    } = *skin;
    let height = look.control_height;
    let current = crate::update::CURRENT;
    let whats_new = gettext(locale, "What's new in {version}").replace("{version}", current);
    let on_github = skin.say("Release notes on GitHub");
    let open = |ui: &Ui, url: &str| ui.ctx().open_url(egui::OpenUrl::new_tab(url));
    match shown {
        Card::NotChecked | Card::UpToDate { .. } => {
            if ButtonSpec::new(&skin.say("Check now"))
                .show(ui, height, look, palette)
                .clicked()
            {
                actions.push(Action::CheckForUpdates);
            }
            if ButtonSpec::new(&whats_new)
                .link()
                .show(ui, height, look, palette)
                .clicked()
            {
                open(ui, &crate::update::this_release());
            }
        }
        Card::Checking => {
            if ButtonSpec::new(&skin.say("Cancel"))
                .show(ui, height, look, palette)
                .clicked()
            {
                actions.push(Action::CancelUpdateCheck);
            }
        }
        Card::Available(offer) | Card::NoBuild(offer) => {
            // Downloading comes with the next step: the page has the
            // download until then.
            let notes = ButtonSpec::new(&on_github);
            let notes = if matches!(shown, Card::Available(_)) {
                notes.primary()
            } else {
                notes.link()
            };
            if notes.show(ui, height, look, palette).clicked() {
                open(ui, &offer.url);
            }
            let skip = skin.say("Skip this version");
            if ButtonSpec::new(&skip)
                .link()
                .show(ui, height, look, palette)
                .clicked()
            {
                actions.push(Action::SkipUpdate);
            }
        }
        // The command and its Copy stand in the actions' place.
        Card::Elsewhere(offer, _) => {
            if ButtonSpec::new(&on_github)
                .link()
                .show(ui, height, look, palette)
                .clicked()
            {
                open(ui, &offer.url);
            }
        }
        Card::Failed { .. } => {
            if ButtonSpec::new(&skin.say("Try again"))
                .show(ui, height, look, palette)
                .clicked()
            {
                actions.push(Action::CheckForUpdates);
            }
        }
    }
}

/// The command another tool updates with, in the code's face, with Copy
/// at its right.
fn command_box(ui: &mut Ui, rect: Rect, command: &str, skin: &Skin) {
    let Skin {
        look,
        palette,
        locale,
        ..
    } = *skin;
    let corner = CornerRadius::same(look.radius);
    ui.painter().rect_filled(rect, corner, palette.panel);
    ui.painter().rect_stroke(
        rect,
        corner,
        Stroke::new(1.0, palette.surface),
        StrokeKind::Inside,
    );
    let mono = TextRole::MonoSecondary;
    widgets::paint_text(
        ui,
        rect.left() + 10.0,
        rect.center().y,
        Text::one(look, mono, command, palette.text),
    );
    widgets::announce(ui, rect, command);
    let label = gettext(locale, "Copy");
    let copy = ButtonSpec::new(&label).link();
    let width = copy.width(ui, look);
    let at = Rect::from_min_size(
        pos2(rect.right() - width - 4.0, rect.top()),
        vec2(width, rect.height()),
    );
    if copy.show_at(ui, at, look, palette).clicked() {
        copy_command(ui.ctx(), command, locale);
    }
}

/// Puts the command that updates this copy on the clipboard, and says so.
pub(super) fn copy_command(ctx: &egui::Context, command: &str, locale: Locale) {
    ctx.copy_text(command.to_owned());
    crate::ui::toast::say(ctx, &gettext(locale, "Copied the command"));
}

/// The command another tool updates this copy with, where it has one.
pub(super) fn managed_command(app: &App) -> Option<&'static str> {
    match Card::of(app) {
        Card::Elsewhere(_, managed) => managed.command,
        _ => None,
    }
}

/// The row of how often a check runs: its label over when the last check
/// ran, with Check now; the menu; and the other choices.
fn check_row(ui: &mut Ui, app: &App, skin: &Skin, actions: &mut Vec<Action>) {
    let settings = super::shown(app);
    let column = super::sheet::label_column(ui, &OptionId::UPDATES, skin);
    let print = checked_text(app, false).unwrap_or_else(|| skin.say("Not checked yet"));
    super::sheet::row_with(
        ui,
        (OptionId::UpdatesCheck, false, column),
        &settings,
        skin,
        actions,
        super::sheet::Print {
            text: &print,
            link: Some((&skin.say("Check now"), Action::CheckForUpdates)),
        },
    );
}

/// What the check sends, in a box of its own.
fn tip(ui: &mut Ui, skin: &Skin) {
    let Skin { look, palette, .. } = *skin;
    let width = ui.available_width();
    let said = Text::one(
        look,
        TextRole::Secondary,
        &skin.say(PRIVACY),
        palette.secondary,
    )
    .wrap((width - 24.0).max(0.0))
    .layout(ui.ctx());
    let (rect, _) = ui.allocate_exact_size(vec2(width, said.height() + 20.0), Sense::hover());
    let corner = CornerRadius::same(8);
    ui.painter().rect_filled(rect, corner, palette.panel);
    ui.painter().rect_stroke(
        rect,
        corner,
        Stroke::new(1.0, palette.outline),
        StrokeKind::Inside,
    );
    paint(ui, &said, pos2(rect.left() + 12.0, rect.top() + 10.0));
}

/// Paints `text` with its top left at `at`, and names it for screen
/// readers.
fn paint(ui: &Ui, text: &Laid, at: egui::Pos2) {
    text.paint(ui.painter(), at);
    widgets::announce(ui, Rect::from_min_size(at, text.size()), text.galley.text());
}

/// A label node over `rect` that a screen reader reads out as it changes:
/// one node for `which` of the card's lines whatever it says, so that a
/// change is a change and not a new node.
pub(super) fn announce_live(ui: &Ui, rect: Rect, (text, which): (&str, &str), live: Live) {
    let id = ui.id().with(("announce-live", which));
    let response = ui.interact(rect, id, Sense::hover());
    response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Label, true, text));
    ui.ctx().accesskit_node_builder(response.id, |node| {
        node.set_live(live);
    });
}

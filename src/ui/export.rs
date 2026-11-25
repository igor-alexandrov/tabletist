//! The Export sheet (macOS and Windows) and float (the terminal look) over
//! a table's tab: format, rows, columns, the format's options and where
//! the text goes; Review SQL in place of them. The export itself is the
//! app's (`src/app/export.rs`): this draws and reads the keys.

use egui::{Id, Rect, ScrollArea, Sense, Ui, pos2, vec2};

use crate::app::ExportFacts;
use crate::export::{
    Binary, Format, JsonValues, Layout, LineEnding, LongValues, NullAs, Options, Quoting,
};
use crate::export_sheet::{
    ColumnPick, DestinationChoice, ExportChange, ExportSheet, RowsChoice, count_words,
};
use crate::i18n::{Msgid, gettext, msgid, ngettext};
use crate::keymap::{Command, Scope, When};
use crate::model::Action;
use crate::theme::{Look, Palette};
use crate::typography::{Text, TextRole};
use crate::ui::keys::{asked, prompt_letter, published, take_enter};
use crate::ui::widgets::{self, ButtonSpec, Group, Segment};
use crate::ui::{Paint, menu};

/// The path line's field: Enter there exports.
pub fn path_id(sheet: &ExportSheet) -> Id {
    Id::new(("export-path", sheet.asked))
}

/// How the sheet draws: the paint, and the facts the app gave it.
#[derive(Clone, Copy)]
struct Skin<'a> {
    look: &'a Look,
    palette: &'a Palette,
    locale: crate::i18n::Locale,
    facts: &'a ExportFacts,
}

impl Skin<'_> {
    /// `text` translated, in the look's case.
    fn say(&self, text: Msgid) -> String {
        self.look.label(&text.say(self.locale))
    }
}

/// The sheet's primary button: "Export ≈ 31,400 rows…", "Copy 12 rows".
pub fn primary_words(
    sheet: &ExportSheet,
    facts: &ExportFacts,
    look: &Look,
    locale: crate::i18n::Locale,
) -> String {
    let again = sheet.rows == RowsChoice::RunAgain;
    let words = match (sheet.destination, facts.rows) {
        // A result run again: counted as it runs (spec 4).
        (DestinationChoice::File, None) if again && look.terminal => {
            gettext(locale, "Run and export").into_owned()
        }
        (DestinationChoice::File, None) if again => gettext(locale, "Run and export…").into_owned(),
        (DestinationChoice::Clipboard, Some(rows)) => {
            ngettext(locale, "Copy {rows} row", "Copy {rows} rows", rows.rows)
                .replace("{rows}", &count_words(rows, locale))
        }
        (DestinationChoice::Clipboard, None) => gettext(locale, "Copy rows").into_owned(),
        (DestinationChoice::File, Some(rows)) if look.terminal => {
            ngettext(locale, "Export {rows} row", "Export {rows} rows", rows.rows)
                .replace("{rows}", &count_words(rows, locale))
        }
        (DestinationChoice::File, Some(rows)) => ngettext(
            locale,
            "Export {rows} row…",
            "Export {rows} rows…",
            rows.rows,
        )
        .replace("{rows}", &count_words(rows, locale)),
        (DestinationChoice::File, None) if look.terminal => gettext(locale, "Export").into_owned(),
        (DestinationChoice::File, None) => gettext(locale, "Export…").into_owned(),
    };
    look.label(&words)
}

/// The sheet's title: "Export reviews".
pub fn title(facts: &ExportFacts, look: &Look, locale: crate::i18n::Locale) -> String {
    let title = match facts.result {
        Some(_) => gettext(locale, "Export result").into_owned(),
        None => gettext(locale, "Export {table}").replace("{table}", &facts.table),
    };
    match look.terminal {
        true => title.to_lowercase(),
        false => title,
    }
}

/// Draws the sheet and reads its keys. Returns what was asked.
pub fn show(
    ctx: &egui::Context,
    paint: Paint,
    sheet: &ExportSheet,
    facts: Option<&ExportFacts>,
    said: Option<&str>,
) -> Vec<Action> {
    let Some(facts) = facts else {
        // Its tab is gone: so is the sheet.
        return vec![Action::CloseDialog];
    };
    let Paint {
        locale,
        palette,
        look,
    } = paint;
    let skin = Skin {
        look: &look,
        palette: &palette,
        locale,
        facts,
    };
    let mut actions = Vec::new();
    keys(ctx, sheet, skin, &mut actions);
    let width = crate::ui::write_prompts::fitted(ctx, if look.terminal { 760.0 } else { 600.0 });
    let modal = widgets::modal(Id::new(("export-sheet", sheet.asked)), &look, &palette)
        .frame(sheet_frame(&look, &palette))
        .show(ctx, |ui| {
            ui.set_width(width);
            widgets::announce(ui, ui.max_rect(), &title(facts, &look, locale));
            if look.terminal {
                terminal(ui, sheet, skin, &mut actions);
            } else {
                mac(ui, sheet, skin, &mut actions);
            }
            if let Some(said) = said {
                widgets::announce_polite(
                    ui,
                    Rect::from_min_size(ui.min_rect().min, vec2(1.0, 1.0)),
                    said,
                );
            }
        });
    if modal.should_close() && !sheet.asking {
        actions.push(match sheet.review {
            true => Action::ChangeExport(ExportChange::Review(false)),
            false => Action::CloseDialog,
        });
    }
    actions
}

/// The modal's frame, with no margin: the parts fill it edge to edge.
fn sheet_frame(look: &Look, palette: &Palette) -> egui::Frame {
    widgets::modal_frame(look, palette).inner_margin(egui::Margin::ZERO)
}

/// The sheet's own keys, before anything is drawn: ↩ exports (in the
/// terminal look from the path line or where nothing has the keyboard,
/// else Enter chooses what has it), Review SQL, and the column list's
/// moves.
fn keys(ctx: &egui::Context, sheet: &ExportSheet, skin: Skin<'_>, actions: &mut Vec<Action>) {
    let look = skin.look;
    let keymap = published(ctx);
    let layout = crate::keymap::Layout::of(look);
    let focused = ctx.memory(|memory| memory.focused());
    let on_path = focused == Some(path_id(sheet));
    let typing = ctx.text_edit_focused();
    let exports = !look.terminal || focused.is_none() || on_path;
    let presses = |command: Command| {
        ctx.input_mut(|input| asked(input, &keymap, layout, command, Scope::Prompt, true))
    };
    let on = |command: Command| presses(command).count > 0;
    // The path line's list, while it shows: its keys come before the
    // float's Enter and Esc (spec 4, S15). Enter held from the press that
    // took a name takes nothing more, nor starts the export.
    if look.terminal && on_path && !sheet.asking && sheet.lit_name().is_some() {
        let pick = presses(Command::PathPick);
        if pick.count > 0 {
            ctx.input_mut(take_enter);
            if pick.fresh {
                actions.push(Action::PickPath(None));
            }
            return;
        }
        if on(Command::ClosePathList) {
            actions.push(Action::ChangeExport(ExportChange::ClosePathList));
            return;
        }
        let step = on(Command::PathNext) as isize - on(Command::PathPrevious) as isize;
        if step != 0 {
            actions.push(Action::ChangeExport(ExportChange::PathMove(step)));
        }
    }
    // `ctrl+o`: the system's save dialog (spec 4, S16).
    if look.terminal
        && !sheet.asking
        && !sheet.review
        && sheet.destination == DestinationChoice::File
        && on(Command::ExportPickFile)
    {
        actions.push(Action::ExportPickFile);
    }
    // The keymap's Enter; then no other Enter of the frame reaches a
    // control that has the keyboard (plan, B6).
    let now = presses(Command::ExportNow);
    if exports && !sheet.asking && now.count > 0 {
        ctx.input_mut(take_enter);
        // A press of its own: Enter held starts nothing.
        if now.fresh {
            actions.push(Action::StartExport);
        }
    }
    if !sheet.asking && on(Command::CloseExport) {
        actions.push(match sheet.review {
            true => Action::ChangeExport(ExportChange::Review(false)),
            false => Action::CloseDialog,
        });
    }
    // A chord: read from the save to line too, which types no Ctrl.
    if look.terminal && on(Command::ExportReviewSql) && skin.facts.statement.is_some() {
        actions.push(Action::ChangeExport(ExportChange::Review(!sheet.review)));
    }
    if sheet.choose && !sheet.review {
        let at = sheet.column;
        if !look.terminal && !typing {
            if on(Command::ExportColumnUp) {
                actions.push(Action::ChangeExport(ExportChange::Move {
                    from: at,
                    by: -1,
                }));
            }
            if on(Command::ExportColumnDown) {
                actions.push(Action::ChangeExport(ExportChange::Move { from: at, by: 1 }));
            }
        } else if !typing {
            // The letters, as typed text.
            let letter = |text: &str| prompt_letter(&keymap, layout, When::Export, text);
            let typed = ctx.input_mut(|input| {
                let mut typed = Vec::new();
                input.events.retain(|event| match event {
                    egui::Event::Text(text) => match letter(text) {
                        Some(command) => {
                            typed.push(command);
                            false
                        }
                        None => true,
                    },
                    _ => true,
                });
                typed
            });
            for command in typed {
                actions.push(Action::ChangeExport(match command {
                    Command::ExportColumnUp => ExportChange::Move { from: at, by: -1 },
                    Command::ExportColumnDown => ExportChange::Move { from: at, by: 1 },
                    Command::ExportAllColumns => ExportChange::AllColumns,
                    Command::ExportNoColumns => ExportChange::NoColumns,
                    _ => continue,
                }));
            }
        }
    }
}

/// "dev Bookshop · public.reviews · 1 filter · sorted by created_at ↓".
fn subtitle(skin: Skin<'_>, sheet: &ExportSheet) -> String {
    let facts = skin.facts;
    if let Some(result) = &facts.result {
        return result_subtitle(skin, result);
    }
    // The float's head: `bookshop · dev · csv` (its badge says PROD).
    if skin.look.terminal {
        let platform = crate::env::Platform::Omarchy;
        let mut parts = vec![facts.connection.clone()];
        if !facts.environment.is_production()
            && let Some(named) = facts.environment.named(platform)
        {
            parts.push(named);
        }
        if facts.read_only && facts.environment.is_production() {
            parts.push(gettext(skin.locale, "read-only").into_owned());
        }
        parts.push(sheet.format.key().to_owned());
        return parts.join(" · ").to_lowercase();
    }
    let mut parts = vec![facts.connection.clone(), facts.qualified.clone()];
    if facts.filters > 0 {
        parts.push(crate::i18n::counted(
            skin.locale,
            facts.filters,
            "filter",
            "filters",
        ));
    }
    if let Some(sort) = &facts.sort {
        parts.push(gettext(skin.locale, "sorted by {sort}").replace("{sort}", sort));
    }
    if facts.read_only && facts.environment.is_production() {
        parts.push(gettext(skin.locale, "read-only connection").into_owned());
    }
    parts.join(" · ")
}

/// A result's: "Bookshop · result of Query 1, statement 2 · read-only
/// transaction"; the float's `bookshop · dev · query 1, statement 2`.
fn result_subtitle(skin: Skin<'_>, result: &crate::app::ResultFacts) -> String {
    let facts = skin.facts;
    let (query, statement) = (result.query.to_string(), result.statement.to_string());
    if skin.look.terminal {
        let platform = crate::env::Platform::Omarchy;
        let mut parts = vec![facts.connection.clone()];
        if !facts.environment.is_production()
            && let Some(named) = facts.environment.named(platform)
        {
            parts.push(named);
        }
        parts.push(
            gettext(skin.locale, "query {query}, statement {statement}")
                .replace("{query}", &query)
                .replace("{statement}", &statement),
        );
        return parts.join(" · ").to_lowercase();
    }
    let of = gettext(
        skin.locale,
        "result of Query {query}, statement {statement}",
    )
    .replace("{query}", &query)
    .replace("{statement}", &statement);
    let transaction = gettext(skin.locale, "read-only transaction").into_owned();
    [facts.connection.clone(), of, transaction].join(" · ")
}

/// macOS: title, subtitle, the rows of choices, the notes and the footer;
/// or Review SQL.
fn mac(ui: &mut Ui, sheet: &ExportSheet, skin: Skin<'_>, actions: &mut Vec<Action>) {
    let Skin {
        look,
        palette,
        locale,
        facts,
    } = skin;
    let production = facts.environment.is_production();
    if production {
        // The production strip, 4 high under the top corners.
        let top = ui.max_rect();
        let strip = Rect::from_min_size(top.min, vec2(top.width(), 4.0));
        ui.painter().rect_filled(
            strip,
            egui::CornerRadius {
                nw: look.dialog_radius,
                ne: look.dialog_radius,
                sw: 0,
                se: 0,
            },
            palette.danger,
        );
    }
    egui::Frame::new()
        .inner_margin(egui::Margin::symmetric(20, 18))
        .show(ui, |ui| {
            let title = match sheet.review {
                true => gettext(locale, "Review SQL").into_owned(),
                false => self::title(facts, look, locale),
            };
            widgets::label(ui, widgets::dialog_title(look), &title, palette.text, look);
            ui.add_space(2.0);
            ui.horizontal(|ui| {
                let platform = crate::env::Platform::of(look);
                if facts.environment.named(platform).is_some() {
                    let colors = crate::env::env_colors(facts.environment, platform, palette);
                    let (rect, _) = ui.allocate_exact_size(
                        vec2(
                            crate::ui::workspace::env_badge_width(
                                ui,
                                facts.environment,
                                crate::ui::workspace::Badge::Mac,
                                look,
                            ),
                            16.0,
                        ),
                        Sense::hover(),
                    );
                    crate::ui::workspace::env_badge(
                        ui,
                        rect.left(),
                        rect.center().y,
                        facts.environment,
                        &colors,
                        crate::ui::workspace::Badge::Mac,
                        look,
                    );
                }
                let sub = match sheet.review {
                    true => format!(
                        "{} · {} · {}",
                        self::title(facts, look, locale),
                        sheet.format.name(),
                        facts
                            .rows
                            .map(|rows| count_words(rows, skin.locale))
                            .unwrap_or_default()
                    ),
                    false => subtitle(skin, sheet),
                };
                widgets::label(ui, widgets::secondary(look), &sub, palette.secondary, look);
            });
            ui.add_space(12.0);
            let room = (ui.ctx().content_rect().height() - 220.0).max(160.0);
            ScrollArea::vertical()
                .max_height(room)
                .auto_shrink([false, true])
                .show(ui, |ui| {
                    if sheet.review {
                        review(ui, skin);
                    } else {
                        choices(ui, sheet, skin, actions);
                    }
                });
            if !sheet.review {
                ui.add_space(8.0);
                notes(ui, sheet, skin);
            }
            ui.add_space(14.0);
            footer(ui, sheet, skin, actions);
        });
}

/// The statement, and what it does.
fn review(ui: &mut Ui, skin: Skin<'_>) {
    let Skin { look, palette, .. } = skin;
    let statement = skin.facts.statement.clone().unwrap_or_default();
    egui::Frame::new()
        .fill(palette.panel)
        .stroke(egui::Stroke::new(1.0, palette.border))
        .inner_margin(egui::Margin::same(10))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            // In the editor's colours.
            let laid = crate::ui::sql_text::highlighted(
                ui.ctx(),
                look,
                palette,
                skin.facts.dialect,
                &statement,
                ui.available_width(),
            );
            laid.label(ui);
        });
    ui.add_space(8.0);
    let note = match skin.look.terminal {
        true => skin.say(msgid("runs once, read-only, streamed in batches of 5,000 rows")),
        false => gettext(
            skin.locale,
            "Runs once, read-only, streamed to the file in batches of 5,000 rows. Nothing is written to the database.",
        )
        .into_owned(),
    };
    Text::one(look, widgets::secondary(look), &note, palette.secondary)
        .wrap(ui.available_width())
        .layout(ui.ctx())
        .label(ui);
}

/// A labelled row: the label in its column, what `add` draws after it.
fn row(ui: &mut Ui, label: &str, skin: Skin<'_>, add: impl FnOnce(&mut Ui)) {
    let width = if skin.look.terminal { 150.0 } else { 120.0 };
    ui.horizontal_top(|ui| {
        let (rect, _) =
            ui.allocate_exact_size(vec2(width, skin.look.control_height), Sense::hover());
        let role = match skin.look.terminal {
            true => TextRole::OSecondary,
            false => TextRole::FieldLabel,
        };
        let color = match skin.look.terminal {
            true => skin.palette.dim,
            false => skin.palette.text,
        };
        widgets::paint_label(
            ui,
            rect.left(),
            rect.center().y,
            Text::one(skin.look, role, label, color),
        );
        ui.vertical(|ui| add(ui));
    });
    ui.add_space(if skin.look.terminal { 4.0 } else { 10.0 });
}

/// A segmented choice of `options`; returns the one clicked.
fn choice<T: Copy + PartialEq>(
    ui: &mut Ui,
    name: &str,
    options: &[(T, String)],
    current: T,
    skin: Skin<'_>,
) -> Option<T> {
    let words: Vec<&str> = options.iter().map(|(_, word)| word.as_str()).collect();
    let segments: Vec<Segment<'_>> = words.iter().map(|word| Segment::Text(word)).collect();
    let selected = options
        .iter()
        .position(|(value, _)| *value == current)
        .unwrap_or(0);
    let widest = words
        .iter()
        .map(|word| {
            widgets::measure(
                ui,
                Text::one(skin.look, widgets::body(skin.look), word, skin.palette.text),
            )
        })
        .fold(0.0_f32, f32::max);
    let size = vec2(widest + 24.0, skin.look.control_height);
    let group = Group {
        name,
        description: None,
    };
    widgets::segmented(
        ui,
        &segments,
        selected,
        size,
        Some(group),
        skin.look,
        skin.palette,
    )
    .and_then(|at| options.get(at).map(|(value, _)| *value))
}

/// Format, rows, columns, the format's options, destination, the notes.
fn choices(ui: &mut Ui, sheet: &ExportSheet, skin: Skin<'_>, actions: &mut Vec<Action>) {
    let Skin {
        look,
        palette,
        locale,
        facts,
    } = skin;
    let change = |actions: &mut Vec<Action>, change| actions.push(Action::ChangeExport(change));
    // Format.
    row(ui, &skin.say(msgid("Format")), skin, |ui| {
        if look.terminal {
            let options: Vec<(Format, String)> = Format::ALL
                .iter()
                .map(|format| (*format, format.key().to_owned()))
                .collect();
            if let Some(format) =
                choice(ui, &skin.say(msgid("Format")), &options, sheet.format, skin)
            {
                change(actions, ExportChange::Format(format));
            }
        } else {
            let dropdown =
                menu::Dropdown::new(("export-format", sheet.asked), sheet.format.name(), 200.0);
            dropdown.show(ui, look, palette, |ui| {
                for format in Format::ALL {
                    let mut chosen = sheet.format;
                    if menu::value(ui, &mut chosen, format, format.name(), look, palette).clicked()
                    {
                        change(actions, ExportChange::Format(format));
                    }
                }
            });
        }
    });
    // Rows.
    row(ui, &skin.say(msgid("Rows")), skin, |ui| {
        rows_list(ui, sheet, skin, actions);
    });
    // Columns.
    row(ui, &skin.say(msgid("Columns")), skin, |ui| {
        ui.horizontal(|ui| {
            let options = [
                (
                    false,
                    skin.say(if look.terminal {
                        msgid("all")
                    } else {
                        msgid("All columns")
                    }),
                ),
                (true, skin.say(msgid("Choose"))),
            ];
            if let Some(choose) = choice(
                ui,
                &skin.say(msgid("Columns")),
                &options,
                sheet.choose,
                skin,
            ) {
                change(actions, ExportChange::Choose(choose));
            }
            let total = sheet.columns.len();
            let chosen = sheet.chosen().len();
            let summary = match sheet.choose {
                false => crate::i18n::counted(locale, total, "column", "columns"),
                true => gettext(locale, "{n} of {total}, in this order")
                    .replace("{n}", &crate::i18n::number(locale, chosen))
                    .replace("{total}", &crate::i18n::number(locale, total)),
            };
            widgets::label(
                ui,
                widgets::secondary(look),
                &look.label(&summary),
                palette.secondary,
                look,
            );
        });
        if sheet.choose {
            column_list(ui, sheet, skin, actions);
        }
    });
    // The format's options, edited on a copy: what changed is sent whole.
    let mut options = sheet.options.clone();
    let heading = gettext(locale, "{format} options").replace("{format}", sheet.format.name());
    if look.terminal {
        widgets::label(
            ui,
            TextRole::OBody,
            &heading.to_lowercase(),
            palette.text,
            look,
        );
        ui.add_space(4.0);
    } else {
        widgets::label(
            ui,
            TextRole::SectionLabel,
            &heading,
            palette.secondary,
            look,
        );
        ui.add_space(6.0);
    }
    format_options(ui, sheet.format, &mut options, skin);
    if options != sheet.options {
        change(actions, ExportChange::Options(Box::new(options)));
    }
    // Destination.
    row(ui, &skin.say(msgid("Destination")), skin, |ui| {
        let options = [
            (
                DestinationChoice::File,
                skin.say(if look.terminal {
                    msgid("to file")
                } else {
                    msgid("File")
                }),
            ),
            (DestinationChoice::Clipboard, skin.say(msgid("Clipboard"))),
        ];
        if let Some(destination) = choice(
            ui,
            &skin.say(msgid("Destination")),
            &options,
            sheet.destination,
            skin,
        ) && !(destination == DestinationChoice::Clipboard && facts.clipboard_refused)
        {
            change(actions, ExportChange::Destination(destination));
        }
        let note = match (sheet.destination, facts.clipboard_refused) {
            (_, true) => skin.say(msgid("Clipboard: more than 10,000 rows, file only")),
            (DestinationChoice::Clipboard, false) => {
                skin.say(msgid("Up to 10,000 rows and 20 MB on the clipboard."))
            }
            (DestinationChoice::File, false) if look.terminal => String::new(),
            (DestinationChoice::File, false) => gettext(
                locale,
                "Choose the name and folder next. Saved readable by your account only.",
            )
            .into_owned(),
        };
        if !note.is_empty() {
            widgets::label(ui, widgets::secondary(look), &note, palette.secondary, look);
        }
    });
    if look.terminal && sheet.destination == DestinationChoice::File {
        row(ui, &skin.say(msgid("save to")), skin, |ui| {
            // `ctrl+o pick` at the line's right end, as the boards draw it;
            // the path stops short of it.
            let pick =
                crate::ui::keys::written_in(ui.ctx(), look, Command::ExportPickFile, Scope::Prompt);
            let words = format!("{pick} {}", skin.say(msgid("pick")));
            let hint = || Text::one(look, TextRole::OSecondary, &words, palette.dim);
            let room = widgets::measure(ui, hint()) + 16.0;
            let id = path_id(sheet);
            take_line(ui.ctx(), id, sheet);
            let mut path = sheet.path.clone();
            let field = widgets::single_in_short(ui, &mut path, look, TextRole::OField, room)
                .id(id)
                .desired_width(ui.available_width().min(460.0));
            let response = ui.add(field);
            response.widget_info(|| {
                egui::WidgetInfo::text_edit(true, sheet.path.clone(), sheet.path.clone(), "save to")
            });
            if response.changed() {
                change(actions, ExportChange::Path(path));
            }
            widgets::paint_text_right(
                ui,
                response.rect.right() - 8.0,
                response.rect.center().y,
                hint(),
            );
            // The keyboard gone from the line: its list goes too, until the
            // next key typed there (plan 4b, B9).
            let asked = crate::ui::path_list::under(
                ui,
                &response,
                &sheet.path,
                sheet.listed.as_ref(),
                list_id(sheet),
                (look, palette),
            );
            if asked.close {
                change(actions, ExportChange::ClosePathList);
            }
            if let Some(at) = asked.taken {
                actions.push(Action::PickPath(Some(at)));
            }
            if sheet.exists {
                widgets::label(
                    ui,
                    TextRole::OSecondary,
                    &skin.say(msgid("! exists · enter again replaces it")),
                    palette.warning,
                    look,
                );
            }
        });
    }
}

/// The path line's list's area.
fn list_id(sheet: &ExportSheet) -> Id {
    Id::new(("export-path-list", sheet.asked))
}

/// The keyboard to the path line, its cursor at the end, once for each
/// time the app asks (`ExportSheet::to_line`): a name taken from the list,
/// `ctrl+o`'s answer.
fn take_line(ctx: &egui::Context, id: Id, sheet: &ExportSheet) {
    crate::ui::path_list::to_line(ctx, id, sheet.to_line, &sheet.path);
}

/// The notes under the choices, always in sight above the footer: the
/// unsaved changes not exported, and what the values and the options are.
fn notes(ui: &mut Ui, sheet: &ExportSheet, skin: Skin<'_>) {
    let Skin {
        look,
        palette,
        locale,
        facts,
    } = skin;
    ui.add_space(4.0);
    // The float's note after a write (the board's).
    let after_write = facts
        .result
        .as_ref()
        .is_some_and(|result| result.again == Err(crate::export_sheet::NotAgain::ChangedData));
    if look.terminal && after_write {
        let note = gettext(
            locale,
            "after a read-write run only \"rows already loaded\" is offered",
        );
        Text::one(look, widgets::secondary(look), &note, palette.dim)
            .wrap(ui.available_width())
            .layout(ui.ctx())
            .label(ui);
        ui.add_space(4.0);
    }
    if facts.pending > 0 {
        let save = crate::ui::keys::written_in(ui.ctx(), look, Command::SaveChanges, Scope::Grid);
        let pending =
            crate::i18n::counted(locale, facts.pending, "unsaved change", "unsaved changes");
        let one = facts.pending == 1;
        let words = match (look.terminal, one) {
            (true, true) => gettext(
                locale,
                "! {pending} in {table} isn't exported · :w first to include it",
            ),
            (true, false) => gettext(
                locale,
                "! {pending} in {table} aren't exported · :w first to include them",
            ),
            (false, true) => gettext(
                locale,
                "{pending} in {table} isn't exported. Export reads what is saved in the database. Save first {key}",
            ),
            (false, false) => gettext(
                locale,
                "{pending} in {table} aren't exported. Export reads what is saved in the database. Save first {key}",
            ),
        };
        let note = crate::i18n::fill(
            &words,
            &[
                ("{pending}", &pending),
                ("{table}", &facts.table),
                ("{key}", save.as_str()),
            ],
        );
        Text::one(look, widgets::secondary(look), &note, palette.warning)
            .wrap(ui.available_width())
            .layout(ui.ctx())
            .label(ui);
        ui.add_space(4.0);
    }
    let note = match look.terminal {
        true => gettext(locale, "values cut at 256 KiB in the grid are written in full · file readable by you only").into_owned(),
        false => gettext(
            locale,
            "Values the grid cuts off at 256 KiB are exported in full. These options are remembered for {format}.",
        )
        .replace("{format}", sheet.format.name()),
    };
    Text::one(look, widgets::secondary(look), &note, palette.dim)
        .wrap(ui.available_width())
        .layout(ui.ctx())
        .label(ui);
}

/// The rows radio group: each choice with its second line and its count;
/// Count exactly beside a count not known.
fn rows_list(ui: &mut Ui, sheet: &ExportSheet, skin: Skin<'_>, actions: &mut Vec<Action>) {
    let Skin {
        look,
        palette,
        locale,
        facts,
    } = skin;
    let say = |text: Msgid| skin.say(text);
    let (first, last) = facts.page;
    let page_line = match look.terminal {
        true => format!(
            "{}–{}",
            crate::i18n::number(locale, first),
            crate::i18n::number(locale, last)
        ),
        false => gettext(locale, "rows {first} to {last}, as sorted")
            .replace("{first}", &crate::i18n::number(locale, first))
            .replace("{last}", &crate::i18n::number(locale, last)),
    };
    let matching_line = {
        let mut parts: Vec<String> = facts.filter_text.iter().cloned().collect();
        if let Some(sort) = &facts.sort {
            parts.push(match look.terminal {
                true => sort.clone(),
                false => gettext(locale, "sorted by {sort}").replace("{sort}", sort),
            });
        }
        parts.join(" · ")
    };
    let whole_line = match look.terminal {
        true => String::new(),
        false => gettext(locale, "no filter, primary key order").into_owned(),
    };
    let known = |count: Option<crate::export_sheet::Count>| {
        count.map(|count| count_words(count, skin.locale))
    };
    let rows: Vec<(RowsChoice, String, String, Option<String>, bool)> = match &facts.result {
        Some(result) => result_rows(skin, result),
        None => vec![
            (
                RowsChoice::Selected,
                say(msgid("Selected rows")),
                String::new(),
                Some(crate::i18n::number(locale, facts.selected as u64)),
                facts.selected > 0,
            ),
            (
                RowsChoice::Page,
                say(msgid("Current page")),
                page_line,
                Some(crate::i18n::number(
                    locale,
                    last.saturating_sub(first.saturating_sub(1)),
                )),
                true,
            ),
            (
                RowsChoice::Matching,
                say(if look.terminal {
                    msgid("matching the filter")
                } else {
                    msgid("All rows matching the filter")
                }),
                matching_line,
                known(facts.matching),
                true,
            ),
            (
                RowsChoice::Whole,
                say(msgid("Whole table")),
                whole_line,
                known(facts.whole),
                true,
            ),
        ],
    };
    let width = ui
        .available_width()
        .min(if look.terminal { 560.0 } else { 440.0 });
    for (choice, name, line, count, enabled) in rows {
        // Run again's count is "counted as it runs" (spec 4, S5).
        let count = match (choice, count) {
            (RowsChoice::RunAgain, None) => Some(say(msgid("counted as it runs"))),
            (_, count) => count,
        };
        let chosen = sheet.rows == choice;
        let height = if look.terminal || line.is_empty() {
            24.0
        } else {
            38.0
        };
        let (rect, response) = ui.allocate_exact_size(
            vec2(width, height),
            if enabled {
                Sense::click()
            } else {
                Sense::hover()
            },
        );
        let spoken = match &count {
            Some(count) => format!("{name}, {count}"),
            None => format!("{name}, {}", say(msgid("not counted"))),
        };
        response.widget_info(|| {
            let mut info =
                egui::WidgetInfo::selected(egui::WidgetType::RadioButton, enabled, chosen, &spoken);
            info.enabled = enabled;
            info
        });
        if response.clicked() && !chosen {
            actions.push(Action::ChangeExport(ExportChange::Rows(choice)));
        }
        let ink = if enabled { palette.text } else { palette.dim };
        let body = widgets::body(look);
        let y = rect.top() + 12.0;
        crate::ui::focus::hint(
            ui,
            &response,
            rect,
            crate::ui::focus::Ring::Inset { radius: 3 },
        );
        let mut x = rect.left() + 2.0;
        if look.terminal {
            let mark = if chosen { "(•)" } else { "( )" };
            x += widgets::paint_text(
                ui,
                x,
                y,
                Text::one(look, body, mark, if chosen { palette.accent } else { ink }),
            ) + 10.0;
        } else {
            let center = pos2(x + 7.0, y);
            ui.painter().circle_stroke(
                center,
                6.5,
                egui::Stroke::new(
                    1.0,
                    if chosen {
                        palette.accent
                    } else {
                        palette.border
                    },
                ),
            );
            if chosen {
                ui.painter().circle_filled(center, 6.5, palette.accent);
                ui.painter().circle_filled(center, 2.5, palette.on_accent);
            }
            x += 22.0;
        }
        let name_width = widgets::paint_text(ui, x, y, Text::one(look, body, &name, ink));
        if !line.is_empty() {
            let (lx, ly) = match look.terminal {
                true => (x + name_width + 10.0, y),
                false => (x, y + 16.0),
            };
            widgets::paint_text(
                ui,
                lx,
                ly,
                Text::one(look, widgets::secondary(look), &line, palette.secondary),
            );
        }
        let right = rect.right() - 4.0;
        let count_text = count.clone().unwrap_or_else(|| say(msgid("not counted")));
        let width = widgets::paint_text_right(
            ui,
            right,
            y,
            Text::one(look, body, &count_text, palette.secondary),
        );
        // Count exactly beside a count that is an estimate or not there.
        let estimate = matches!(choice, RowsChoice::Matching)
            && facts.matching.is_none_or(|count| !count.exact);
        if estimate {
            let words = match facts.counting {
                true => say(msgid("Counting…")),
                false => say(msgid("Count exactly")),
            };
            let text = Text::one(look, widgets::secondary(look), &words, palette.accent);
            let laid = text.layout(ui.ctx());
            let link = Rect::from_min_size(
                pos2(right - width - 14.0 - laid.width(), y - 8.0),
                vec2(laid.width(), 16.0),
            );
            laid.paint_left(ui.painter(), link.left(), y);
            let clicked = ui
                .interact(link, Id::new(("export-count", sheet.asked)), Sense::click())
                .on_hover_text(gettext(locale, "Count the rows exactly"));
            clicked.widget_info(|| {
                egui::WidgetInfo::labeled(egui::WidgetType::Button, !facts.counting, &words)
            });
            if clicked.clicked() && !facts.counting {
                actions.push(Action::CountForExport);
            }
        }
    }
}

/// A result's rows (spec 4, F1 and F3): Selected rows, Rows already
/// loaded, and Run again without the limit or why it is greyed.
fn result_rows(
    skin: Skin<'_>,
    result: &crate::app::ResultFacts,
) -> Vec<(RowsChoice, String, String, Option<String>, bool)> {
    use crate::export_sheet::NotAgain;
    let (look, locale, facts) = (skin.look, skin.locale, skin.facts);
    let say = |text: Msgid| skin.say(text);
    let loaded_line = match (look.terminal, result.cut) {
        (true, true) => say(msgid("limit reached")),
        (true, false) => String::new(),
        (false, true) => say(msgid("the result as shown · limit reached")),
        (false, false) => say(msgid("the result as shown")),
    };
    let again_line = match (result.again, look.terminal) {
        (Ok(()), true) => say(msgid("reads only")),
        (Ok(()), false) => gettext(
            locale,
            "reads only · same statement, no LIMIT, no timeout from Settings › Editor",
        )
        .into_owned(),
        (Err(NotAgain::ChangedData), true) => say(msgid("not after a write")),
        (Err(NotAgain::ChangedData), false) => say(msgid("Not offered: this run changed data")),
        (Err(NotAgain::AllLoaded), true) => say(msgid("every row is loaded")),
        (Err(NotAgain::AllLoaded), false) => say(msgid("Not needed: every row is loaded")),
        (Err(NotAgain::Edited), true) => say(msgid("edited since it ran")),
        (Err(NotAgain::Edited), false) => {
            say(msgid("Not offered: the statement was edited since it ran"))
        }
        (Err(NotAgain::Disconnected), true) => say(msgid("not connected")),
        (Err(NotAgain::Disconnected), false) => {
            say(msgid("Not offered: the connection is not ready"))
        }
    };
    vec![
        (
            RowsChoice::Selected,
            say(msgid("Selected rows")),
            String::new(),
            Some(crate::i18n::number(locale, facts.selected as u64)),
            facts.selected > 0,
        ),
        (
            RowsChoice::Loaded,
            say(msgid("Rows already loaded")),
            loaded_line,
            Some(crate::i18n::number(locale, result.loaded)),
            true,
        ),
        (
            RowsChoice::RunAgain,
            say(msgid("Run again without the limit")),
            again_line,
            None,
            result.again.is_ok(),
        ),
    ]
}

/// The column list: a row a column, its box, its name, its type and its
/// handle; Space includes it, ⌥↑ ⌥↓ (`K` `J`) and a drag on the handle
/// move it; All and None under it.
fn column_list(ui: &mut Ui, sheet: &ExportSheet, skin: Skin<'_>, actions: &mut Vec<Action>) {
    let Skin {
        look,
        palette,
        locale,
        ..
    } = skin;
    let width = ui
        .available_width()
        .min(if look.terminal { 560.0 } else { 440.0 });
    let body = widgets::body(look);
    ui.add_space(6.0);
    egui::Frame::new()
        .stroke(egui::Stroke::new(1.0, palette.border))
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 0.0;
            for (at, column) in sheet.columns.iter().enumerate() {
                let (rect, response) = ui.allocate_exact_size(vec2(width, 28.0), Sense::click());
                let name = column_name(column, locale);
                response.widget_info(|| {
                    egui::WidgetInfo::selected(
                        egui::WidgetType::Checkbox,
                        true,
                        column.included,
                        &name,
                    )
                });
                if response.clicked() {
                    actions.push(Action::ChangeExport(ExportChange::Toggle(at)));
                }
                if response.gained_focus() && at != sheet.column {
                    actions.push(Action::ChangeExport(ExportChange::Cursor(at)));
                }
                crate::ui::focus::hint(
                    ui,
                    &response,
                    rect,
                    crate::ui::focus::Ring::Inset { radius: 0 },
                );
                // The terminal look's cursor, as its lists draw one.
                if look.terminal && at == sheet.column {
                    ui.painter().rect_filled(rect, 0, palette.selection);
                    if look.terminal {
                        ui.painter().rect_filled(
                            Rect::from_min_size(rect.min, vec2(2.0, rect.height())),
                            0,
                            palette.accent,
                        );
                    }
                }
                let y = rect.center().y;
                let mut x = rect.left() + 8.0;
                if look.terminal {
                    let number = (at + 1).to_string();
                    widgets::paint_text_right(
                        ui,
                        x + 20.0,
                        y,
                        Text::one(look, body, &number, palette.dim),
                    );
                    x += 28.0;
                    let mark = if column.included { "[x]" } else { "[ ]" };
                    let ink = if column.included {
                        palette.success
                    } else {
                        palette.dim
                    };
                    x += widgets::paint_text(ui, x, y, Text::one(look, body, mark, ink)) + 10.0;
                } else {
                    let mark = if column.included { "✓" } else { " " };
                    widgets::paint_text(ui, x, y, Text::one(look, body, mark, palette.accent));
                    x += 20.0;
                }
                let ink = if column.included {
                    palette.text
                } else {
                    palette.dim
                };
                widgets::paint_text(ui, x, y, Text::one(look, body, &column.name, ink));
                let handle_x = rect.right() - 24.0;
                widgets::paint_text_right(
                    ui,
                    handle_x - 8.0,
                    y,
                    Text::one(
                        look,
                        widgets::secondary(look),
                        &column.type_name,
                        palette.secondary,
                    ),
                );
                if !look.terminal {
                    // The handle: a drag on it moves the column.
                    let handle = Rect::from_center_size(pos2(handle_x + 10.0, y), vec2(16.0, 20.0));
                    widgets::paint_text(
                        ui,
                        handle.left(),
                        y,
                        Text::one(look, body, "⋮⋮", palette.dim),
                    );
                    let drag = ui.interact(
                        handle,
                        Id::new(("export-column-drag", sheet.asked, at)),
                        Sense::drag(),
                    );
                    if drag.drag_started() {
                        egui::DragAndDrop::set_payload(ui.ctx(), at);
                    }
                    if let Some(from) = response.dnd_release_payload::<usize>()
                        && *from != at
                    {
                        let by = at as isize - *from as isize;
                        actions.push(Action::ChangeExport(ExportChange::Move { from: *from, by }));
                    }
                    if egui::DragAndDrop::has_payload_of_type::<usize>(ui.ctx())
                        && response.contains_pointer()
                    {
                        ui.painter().hline(
                            rect.x_range(),
                            rect.top(),
                            egui::Stroke::new(2.0, palette.accent),
                        );
                    }
                }
            }
        });
    ui.add_space(6.0);
    ui.horizontal(|ui| {
        let hint = match look.terminal {
            true => skin.say(msgid("space include · K/J move up/down · a all · n none")),
            false => {
                let up = crate::ui::keys::written_in(
                    ui.ctx(),
                    look,
                    Command::ExportColumnUp,
                    Scope::Prompt,
                );
                let down = crate::ui::keys::written_in(
                    ui.ctx(),
                    look,
                    Command::ExportColumnDown,
                    Scope::Prompt,
                );
                gettext(
                    locale,
                    "Space includes or leaves out · {up} {down} move · drag ⋮⋮ to reorder",
                )
                .replace("{up}", up.as_str())
                .replace("{down}", down.as_str())
            }
        };
        widgets::label(ui, widgets::secondary(look), &hint, palette.secondary, look);
        if !look.terminal {
            let all = gettext(locale, "All");
            let none = gettext(locale, "None");
            if ButtonSpec::new(&all)
                .link()
                .show(ui, 20.0, look, palette)
                .clicked()
            {
                actions.push(Action::ChangeExport(ExportChange::AllColumns));
            }
            if ButtonSpec::new(&none)
                .link()
                .show(ui, 20.0, look, palette)
                .clicked()
            {
                actions.push(Action::ChangeExport(ExportChange::NoColumns));
            }
        }
    });
}

/// A column as a screen reader names its row: "title, text".
fn column_name(column: &ColumnPick, _locale: crate::i18n::Locale) -> String {
    format!("{}, {}", column.name, column.type_name)
}

/// The rows of a format's options, as the boards draw them.
fn format_options(ui: &mut Ui, format: Format, options: &mut Options, skin: Skin<'_>) {
    let Skin {
        look,
        palette,
        locale,
        facts,
    } = skin;
    let say = |text: Msgid| skin.say(text);
    let checkbox = |ui: &mut Ui, value: &mut bool, text: &str| {
        widgets::checkbox(ui, value, text, look, palette);
    };
    let null_as = |ui: &mut Ui, null_as: &mut NullAs, text: &mut String| {
        row(ui, &say(msgid("NULL as")), skin, |ui| {
            ui.horizontal(|ui| {
                let choices = [
                    (NullAs::Empty, say(msgid("Empty"))),
                    (NullAs::Text, say(msgid("Text"))),
                ];
                if let Some(chosen) = choice(ui, &say(msgid("NULL as")), &choices, *null_as, skin) {
                    *null_as = chosen;
                }
                if *null_as == NullAs::Text {
                    let field =
                        widgets::single_in(ui, text, look, widgets::body(look)).desired_width(80.0);
                    ui.add(field).widget_info(|| {
                        egui::WidgetInfo::text_edit(
                            true,
                            text.clone(),
                            text.clone(),
                            "NULL as text",
                        )
                    });
                }
            });
        });
    };
    let line_ending = |ui: &mut Ui, ending: &mut LineEnding| {
        row(ui, &say(msgid("Line endings")), skin, |ui| {
            let choices = [
                (LineEnding::Lf, say(msgid("LF"))),
                (LineEnding::Crlf, say(msgid("CRLF"))),
            ];
            if let Some(chosen) = choice(ui, &say(msgid("Line endings")), &choices, *ending, skin) {
                *ending = chosen;
            }
        });
    };
    let binary = |ui: &mut Ui, binary: &mut Binary| {
        row(ui, &say(msgid("Binary values")), skin, |ui| {
            let choices = [
                (Binary::Hex, say(msgid("Hex"))),
                (Binary::Base64, say(msgid("Base64"))),
            ];
            if let Some(chosen) = choice(ui, &say(msgid("Binary values")), &choices, *binary, skin)
            {
                *binary = chosen;
            }
        });
    };
    let note = |ui: &mut Ui, label: Msgid, text: &str| {
        row(ui, &say(label), skin, |ui| {
            widgets::label(
                ui,
                widgets::secondary(look),
                &look.label(text),
                palette.secondary,
                look,
            );
        });
    };
    let json_values = |ui: &mut Ui, values: &mut JsonValues| {
        row(ui, &say(msgid("json and jsonb values")), skin, |ui| {
            let choices = [
                (JsonValues::Nested, say(msgid("Nested as JSON"))),
                (JsonValues::Strings, say(msgid("As strings"))),
            ];
            if let Some(chosen) = choice(
                ui,
                &say(msgid("json and jsonb values")),
                &choices,
                *values,
                skin,
            ) {
                *values = chosen;
            }
        });
    };
    match format {
        Format::Csv => {
            let csv = &mut options.csv;
            row(ui, &say(msgid("Header row")), skin, |ui| {
                checkbox(
                    ui,
                    &mut csv.header,
                    &say(msgid("Column names as the first row")),
                )
            });
            row(ui, &say(msgid("Delimiter")), skin, |ui| {
                ui.horizontal(|ui| {
                    let known = [',', ';', '|'];
                    let current = if known.contains(&csv.delimiter) {
                        Some(csv.delimiter)
                    } else {
                        None
                    };
                    let choices = [
                        (Some(','), ",".to_owned()),
                        (Some(';'), ";".to_owned()),
                        (Some('|'), "|".to_owned()),
                        (None, say(msgid("Other…"))),
                    ];
                    if let Some(chosen) =
                        choice(ui, &say(msgid("Delimiter")), &choices, current, skin)
                    {
                        csv.delimiter = chosen.unwrap_or('\t');
                    }
                    if current.is_none() {
                        let mut text = csv.delimiter.to_string();
                        let field = widgets::single_in(ui, &mut text, look, widgets::body(look))
                            .desired_width(28.0)
                            .char_limit(1);
                        if ui.add(field).changed()
                            && let Some(char) = text.chars().next()
                        {
                            csv.delimiter = char;
                        }
                    }
                });
            });
            row(ui, &say(msgid("Quoting")), skin, |ui| {
                let choices = [
                    (Quoting::WhenNeeded, say(msgid("When needed"))),
                    (Quoting::Every, say(msgid("Every value"))),
                ];
                if let Some(chosen) =
                    choice(ui, &say(msgid("Quoting")), &choices, csv.quoting, skin)
                {
                    csv.quoting = chosen;
                }
            });
            null_as(ui, &mut csv.null_as, &mut csv.null_text);
            row(ui, &say(msgid("For Excel")), skin, |ui| {
                checkbox(ui, &mut csv.bom, &say(msgid("Add a UTF-8 BOM")))
            });
            line_ending(ui, &mut csv.line_ending);
            binary(ui, &mut csv.binary);
        }
        Format::Tsv => {
            let tsv = &mut options.tsv;
            row(ui, &say(msgid("Header row")), skin, |ui| {
                checkbox(
                    ui,
                    &mut tsv.header,
                    &say(msgid("Column names as the first row")),
                )
            });
            note(
                ui,
                msgid("Tabs and line breaks"),
                &gettext(locale, "inside values are written as \\t and \\n"),
            );
            null_as(ui, &mut tsv.null_as, &mut tsv.null_text);
            row(ui, &say(msgid("For Excel")), skin, |ui| {
                checkbox(ui, &mut tsv.bom, &say(msgid("Add a UTF-8 BOM")))
            });
            line_ending(ui, &mut tsv.line_ending);
            binary(ui, &mut tsv.binary);
        }
        Format::Json => {
            let json = &mut options.json;
            note(
                ui,
                msgid("Shape"),
                &gettext(locale, "one array of objects, keys in column order"),
            );
            json_values(ui, &mut json.values);
            note(
                ui,
                msgid("Dates and times"),
                &gettext(locale, "ISO 8601, with the offset for timestamptz"),
            );
            row(ui, &say(msgid("Layout")), skin, |ui| {
                let choices = [
                    (Layout::Indented, say(msgid("Indented"))),
                    (Layout::OneLine, say(msgid("One line"))),
                ];
                if let Some(chosen) = choice(ui, &say(msgid("Layout")), &choices, json.layout, skin)
                {
                    json.layout = chosen;
                }
            });
            binary(ui, &mut json.binary);
        }
        Format::JsonLines => {
            let jsonl = &mut options.jsonl;
            note(ui, msgid("Shape"), &gettext(locale, "one object per line"));
            json_values(ui, &mut jsonl.values);
            note(
                ui,
                msgid("Dates and times"),
                &gettext(locale, "ISO 8601, with the offset for timestamptz"),
            );
            binary(ui, &mut jsonl.binary);
        }
        Format::SqlInsert => {
            let sql = &mut options.sql;
            note(ui, msgid("Target table"), &facts.qualified);
            row(ui, &say(msgid("Rows per INSERT")), skin, |ui| {
                let choices = [
                    (1, "1".to_owned()),
                    (100, "100".to_owned()),
                    (1000, "1,000".to_owned()),
                ];
                if let Some(chosen) = choice(
                    ui,
                    &say(msgid("Rows per INSERT")),
                    &choices,
                    sql.rows_per_insert,
                    skin,
                ) {
                    sql.rows_per_insert = chosen;
                }
            });
            row(ui, &say(msgid("Table definition")), skin, |ui| {
                match facts.result {
                    // Greyed, with why: no table to read a definition of
                    // (spec 4, S4). The remembered choice stays as it is.
                    Some(_) => {
                        let why = gettext(locale, "Not for a result: no table to read");
                        let mut off = false;
                        ui.add_enabled_ui(false, |ui| {
                            checkbox(
                                ui,
                                &mut off,
                                &say(msgid("Start with CREATE TABLE from the DDL view")),
                            );
                        })
                        .response
                        .on_disabled_hover_text(why.as_ref());
                        widgets::label(
                            ui,
                            widgets::secondary(look),
                            &look.label(&why),
                            palette.secondary,
                            look,
                        );
                    }
                    None => checkbox(
                        ui,
                        &mut sql.create_table,
                        &say(msgid("Start with CREATE TABLE from the DDL view")),
                    ),
                }
            });
            let dialect = match facts.dialect {
                tabletist_db::Dialect::Postgres => "PostgreSQL",
                tabletist_db::Dialect::MySql => "MySQL",
                tabletist_db::Dialect::Sqlite => "SQLite",
                tabletist_db::Dialect::BigQuery => "BigQuery",
                tabletist_db::Dialect::DuckDb => "DuckDB",
            };
            let literal = match facts.dialect {
                tabletist_db::Dialect::Postgres => r"'\x…'",
                tabletist_db::Dialect::BigQuery => r"b'\x…'",
                tabletist_db::Dialect::MySql | tabletist_db::Dialect::Sqlite => "x'…'",
                tabletist_db::Dialect::DuckDb => r"'\x…'::BLOB",
            };
            note(
                ui,
                msgid("Dialect"),
                &gettext(locale, "{dialect}, from this connection").replace("{dialect}", dialect),
            );
            note(
                ui,
                msgid("Binary values"),
                &gettext(locale, "written as {literal} literals").replace("{literal}", literal),
            );
        }
        Format::Markdown => {
            let md = &mut options.md;
            note(
                ui,
                msgid("Header row"),
                &gettext(locale, "always, with a separator line"),
            );
            null_as(ui, &mut md.null_as, &mut md.null_text);
            note(
                ui,
                msgid("Pipes and line breaks"),
                &gettext(locale, "escaped as \\| and <br>"),
            );
            row(ui, &say(msgid("Long values")), skin, |ui| {
                let choices = [
                    (LongValues::Full, say(msgid("In full"))),
                    (LongValues::Cut, say(msgid("Cut at 200 characters"))),
                ];
                if let Some(chosen) = choice(
                    ui,
                    &say(msgid("Long values")),
                    &choices,
                    md.long_values,
                    skin,
                ) {
                    md.long_values = chosen;
                }
            });
        }
    }
}

/// macOS: quiet Review SQL on the left; Cancel and the primary button on
/// the right (Back in Review SQL).
fn footer(ui: &mut Ui, sheet: &ExportSheet, skin: Skin<'_>, actions: &mut Vec<Action>) {
    let Skin {
        look,
        palette,
        locale,
        facts,
    } = skin;
    let height = 30.0;
    let primary = primary_words(sheet, facts, look, locale);
    let cancel = gettext(locale, "Cancel");
    let esc = crate::ui::keys::written_in(ui.ctx(), look, Command::CloseExport, Scope::Prompt);
    let review = gettext(locale, "Review SQL");
    let back = gettext(locale, "Back");
    let nothing_runs = gettext(locale, "Nothing runs: these rows are the ones loaded");
    let no_columns = gettext(locale, "Choose at least one column");
    ui.horizontal(|ui| {
        if sheet.review {
            if ButtonSpec::new(&back)
                .show(ui, height, look, palette)
                .clicked()
            {
                actions.push(Action::ChangeExport(ExportChange::Review(false)));
            }
        } else {
            let button = ButtonSpec::new(&review).link();
            let button = match facts.statement {
                Some(_) => button,
                None => button.disabled(&nothing_runs),
            };
            if button.show(ui, height, look, palette).clicked() {
                actions.push(Action::ChangeExport(ExportChange::Review(true)));
            }
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let button = ButtonSpec::new(&primary).primary();
            let button = match (sheet.chosen().is_empty(), sheet.asking) {
                (true, _) => button.disabled(&no_columns),
                (_, true) => button.disabled(&primary),
                _ => button,
            };
            if button.show(ui, height, look, palette).clicked() {
                actions.push(Action::StartExport);
            }
            if ButtonSpec::new(&cancel)
                .shortcut(&esc)
                .show(ui, height, look, palette)
                .clicked()
            {
                actions.push(Action::CloseDialog);
            }
        });
    });
}

/// The terminal look's float: its head, the rows, its foot of keys.
fn terminal(ui: &mut Ui, sheet: &ExportSheet, skin: Skin<'_>, actions: &mut Vec<Action>) {
    let Skin {
        look,
        palette,
        locale,
        facts,
    } = skin;
    let radius = look.dialog_radius.saturating_sub(1);
    let head = crate::ui::terminal_dialog::head(ui, radius, palette.panel, palette);
    let y = crate::ui::terminal_dialog::head_line(head);
    let title = match sheet.review {
        true => skin.say(msgid("review sql")),
        false => self::title(facts, look, locale),
    };
    let mut x = head.left() + 14.0;
    x += widgets::paint_label(
        ui,
        x,
        y,
        Text::one(look, TextRole::OBody, &title, palette.text),
    ) + 12.0;
    if facts.environment.is_production() {
        let platform = crate::env::Platform::Omarchy;
        let colors = crate::env::env_colors(facts.environment, platform, palette);
        x += crate::ui::workspace::env_badge(
            ui,
            x,
            y,
            facts.environment,
            &colors,
            crate::ui::workspace::Badge::Plain,
            look,
        ) + 12.0;
    }
    let sub = match sheet.review {
        true => format!(
            "{} · {} · {}",
            self::title(facts, look, locale),
            sheet.format.key(),
            facts
                .rows
                .map(|rows| count_words(rows, skin.locale))
                .unwrap_or_default()
        ),
        false => subtitle(skin, sheet),
    };
    widgets::paint_label(
        ui,
        x,
        y,
        Text::one(look, TextRole::OSecondary, &sub, palette.dim),
    );
    egui::Frame::new()
        .inner_margin(egui::Margin::symmetric(14, 10))
        .show(ui, |ui| {
            let room = (ui.ctx().content_rect().height() - 200.0).max(160.0);
            ScrollArea::vertical()
                .max_height(room)
                .auto_shrink([false, true])
                .show(ui, |ui| {
                    if sheet.review {
                        review(ui, skin);
                    } else {
                        choices(ui, sheet, skin, actions);
                    }
                });
            if !sheet.review {
                ui.add_space(6.0);
                notes(ui, sheet, skin);
            }
        });
    let foot = crate::ui::terminal_dialog::foot(ui, radius, palette);
    let written =
        |command: Command| crate::ui::keys::written_in(ui.ctx(), look, command, Scope::Prompt);
    let tab = written(Command::ExportNext);
    let enter = written(Command::ExportNow);
    let space = written(Command::ExportChoose);
    let esc = written(Command::CloseExport);
    let review_key = written(Command::ExportReviewSql);
    let pick_key = written(Command::ExportPickFile);
    let primary = match (sheet.review, sheet.destination) {
        (true, _) | (_, DestinationChoice::File) => skin.say(msgid("export")),
        (_, DestinationChoice::Clipboard) => skin.say(msgid("copy")),
    };
    let (next, choose, review_word, cancel, back) = (
        skin.say(msgid("next")),
        skin.say(msgid("choose")),
        skin.say(msgid("review sql")),
        skin.say(msgid("cancel")),
        skin.say(msgid("back")),
    );
    let pick_word = skin.say(msgid("pick file"));
    let primary_name = primary_words(sheet, facts, look, locale);
    let mut keys = Vec::new();
    if !sheet.review {
        keys.push(crate::ui::terminal_dialog::Key {
            key: &tab,
            label: &next,
            button: None,
            lead: false,
            disabled: None,
        });
    }
    keys.push(crate::ui::terminal_dialog::Key {
        key: &enter,
        label: &primary,
        button: Some(&primary_name),
        lead: true,
        disabled: None,
    });
    if !sheet.review {
        keys.push(crate::ui::terminal_dialog::Key {
            key: &space,
            label: &choose,
            button: None,
            lead: false,
            disabled: None,
        });
        // The system's dialog, for a file (spec 4, S16).
        if sheet.destination == DestinationChoice::File {
            keys.push(crate::ui::terminal_dialog::Key {
                key: &pick_key,
                label: &pick_word,
                button: None,
                lead: false,
                disabled: None,
            });
        }
        if facts.statement.is_some() {
            keys.push(crate::ui::terminal_dialog::Key {
                key: &review_key,
                label: &review_word,
                button: None,
                lead: false,
                disabled: None,
            });
        }
    }
    let close = if sheet.review { &back } else { &cancel };
    keys.push(crate::ui::terminal_dialog::Key {
        key: &esc,
        label: close,
        button: Some(close),
        lead: false,
        disabled: None,
    });
    let pressed = crate::ui::terminal_dialog::keys(ui, foot, 0.0, &keys, look, palette);
    if let Some(pressed) = pressed {
        let key = &keys[pressed];
        if key.label == primary {
            actions.push(Action::StartExport);
        } else if sheet.review {
            actions.push(Action::ChangeExport(ExportChange::Review(false)));
        } else {
            actions.push(Action::CloseDialog);
        }
    }
}

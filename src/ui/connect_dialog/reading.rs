//! A Files connection's Reading CSV section, in either look: how its CSV
//! and TSV files are read (header row, delimiter, column types), and the
//! overrides of one file, each with Reset. Closed, it says the choices on
//! one line.

use egui::{CornerRadius, Sense, Ui, vec2};
use tabletist_db::files::{Change, Delimiter, Header, Override, Reading, Types};

use super::Skin;
use super::choice::{Choice, Group, choose};
use crate::i18n::gettext;
use crate::model::{Action, ConnectionForm, CsvChange};
use crate::typography::{Text, TextRole};
use crate::ui::disclosure::{self, Section};
use crate::ui::{focus, menu, widgets};

/// What the open section says under its title: macOS, then the terminal
/// look's shorter words, as its board writes them.
const HINT: &str = "Defaults for every CSV in this connection";
const TERMINAL_HINT: &str = "Defaults for every CSV";

/// The width of the macOS options' labels.
const LABEL: f32 = 110.0;

/// The id of the override at `at`'s row in the terminal look: what `R`
/// resets while it has the keyboard.
pub(super) fn override_id(at: usize) -> egui::Id {
    egui::Id::new(("csv-override", at))
}

/// The override whose row has the keyboard (the terminal look).
pub(super) fn focused_override(ctx: &egui::Context, form: &ConnectionForm) -> Option<usize> {
    let focused = ctx.memory(|memory| memory.focused())?;
    (0..form.files_csv.overrides.len()).find(|at| override_id(*at) == focused)
}

/// The closed row's summary: each choice as a few words, then how many
/// overrides there are. `terminal` says the types as its choices do.
pub(super) fn summary(reading: &Reading, terminal: bool) -> String {
    let header = match reading.header {
        Header::Detect => "Header detect",
        Header::Yes => "Header yes",
        Header::No => "Header no",
    };
    let delimiter = match reading.delimiter {
        Delimiter::Detect => "delimiter detect",
        Delimiter::Comma => "delimiter ,",
        Delimiter::Semicolon => "delimiter ;",
        Delimiter::Tab => "delimiter tab",
        Delimiter::Pipe => "delimiter |",
    };
    let types = match (reading.types, terminal) {
        (Types::Sample, false) => "types from the first 20,000 rows",
        (Types::Sample, true) => "types first 20,000 rows",
        (Types::All, false) => "types from all rows",
        (Types::All, true) => "types all rows",
        (Types::Text, _) => "everything as text",
    };
    let mut said = format!("{header} · {delimiter} · {types}");
    match reading.overrides.len() {
        0 => {}
        1 => said.push_str(" · 1 override"),
        many => said.push_str(&format!(" · {many} overrides")),
    }
    said
}

/// What an override's row says after the file's name: "returned_at read
/// as text" (macOS), "returned_at as text" (Omarchy). The words are in
/// either look's case already: the column and the values are the file's,
/// never lowercased.
pub(super) fn override_words(change: &Change, terminal: bool) -> String {
    let read = if terminal { "" } else { " read" };
    match change {
        Change::Text { column } => format!("{column}{read} as text"),
        Change::Empty { column, values } => {
            let values: Vec<String> = values.iter().map(|value| format!("'{value}'")).collect();
            format!("{}{read} as empty in {column}", values.join(", "))
        }
        Change::Skip => "skipped".to_owned(),
    }
}

/// The header row's choices.
fn headers(skin: &Skin) -> Vec<(Header, String)> {
    let words = ["Detect", "Yes", "No"];
    Header::ALL
        .into_iter()
        .zip(words)
        .map(|(header, word)| (header, skin.say(word)))
        .collect()
}

/// The delimiter's choices.
fn delimiters(skin: &Skin) -> Vec<(Delimiter, String)> {
    Delimiter::ALL
        .into_iter()
        .map(|delimiter| {
            let word = match delimiter {
                Delimiter::Detect => skin.say("Detect"),
                Delimiter::Tab => skin.say("Tab"),
                Delimiter::Comma | Delimiter::Semicolon | Delimiter::Pipe => {
                    delimiter.char().map(String::from).unwrap_or_default()
                }
            };
            (delimiter, word)
        })
        .collect()
}

/// The column types' choices: the dropdown's words on macOS, the choice
/// row's in the terminal look.
fn types(skin: &Skin) -> Vec<(Types, String)> {
    let words: [&'static str; 3] = match skin.look.terminal {
        true => ["First 20,000 rows", "All rows", "All text"],
        false => [
            "Detect from the first 20,000 rows",
            "Detect from all rows (slower)",
            "Read everything as text",
        ],
    };
    Types::ALL
        .into_iter()
        .zip(words)
        .map(|(types, word)| (types, skin.say(word)))
        .collect()
}

/// A choice among `options`, as a group named `name`: the change it makes.
fn choice<T: Copy + PartialEq>(
    ui: &mut Ui,
    salt: &str,
    name: &str,
    options: &[(T, String)],
    current: T,
    group: Group,
    skin: &Skin,
) -> Option<T> {
    let options: Vec<(T, Choice<'_>)> = options
        .iter()
        .map(|(value, word)| (*value, Choice::plain(word)))
        .collect();
    choose(ui, salt, Some(name), &options, current, group, skin)
}

/// The section after Source, in either look.
pub(super) fn section(
    ui: &mut Ui,
    form: &mut ConnectionForm,
    skin: &Skin,
    actions: &mut Vec<Action>,
) {
    let Skin { look, palette, .. } = *skin;
    let title = skin.say("Reading CSV");
    let summary = look.label(&summary(&form.files_csv, look.terminal));
    let hint = skin.say(match look.terminal {
        true => TERMINAL_HINT,
        false => HINT,
    });
    let section = Section {
        salt: "reading-csv",
        title: &title,
        summary: &summary,
        hint: &hint,
    };
    let mut open = form.csv_open;
    disclosure::show(
        ui,
        &section,
        &mut open,
        look,
        palette,
        skin.fill,
        skin.rule(),
        |ui| match look.terminal {
            true => terminal_rows(ui, form, skin, actions),
            false => sheet_rows(ui, form, skin, actions),
        },
    );
    form.csv_open = open;
}

/// macOS: a label in the options' column, then what `add` draws.
fn option_row(ui: &mut Ui, label: &str, height: f32, skin: &Skin, add: impl FnOnce(&mut Ui)) {
    let Skin { look, palette, .. } = *skin;
    let center = egui::Layout::left_to_right(egui::Align::Center);
    ui.allocate_ui_with_layout(vec2(ui.available_width(), height), center, |ui| {
        ui.allocate_ui_with_layout(vec2(LABEL, height), center, |ui| {
            ui.set_min_width(LABEL);
            widgets::label(ui, TextRole::Secondary, label, palette.secondary, look);
        });
        add(ui);
    });
}

/// macOS: the open section's options and overrides.
fn sheet_rows(ui: &mut Ui, form: &ConnectionForm, skin: &Skin, actions: &mut Vec<Action>) {
    let reading = &form.files_csv;
    let group = Group::Track(24.0);
    let header = skin.say("Header row");
    option_row(ui, &header, group.height(), skin, |ui| {
        if let Some(chosen) = choice(
            ui,
            "csv-header",
            &header,
            &headers(skin),
            reading.header,
            group,
            skin,
        ) {
            actions.push(Action::SetCsv(CsvChange::Header(chosen)));
        }
    });
    ui.add_space(8.0);
    let delimiter = skin.say("Delimiter");
    option_row(ui, &delimiter, group.height(), skin, |ui| {
        if let Some(chosen) = choice(
            ui,
            "csv-delimiter",
            &delimiter,
            &delimiters(skin),
            reading.delimiter,
            group,
            skin,
        ) {
            actions.push(Action::SetCsv(CsvChange::Delimiter(chosen)));
        }
    });
    ui.add_space(8.0);
    let label = skin.say("Column types");
    option_row(ui, &label, skin.field_height(), skin, |ui| {
        let options = types(skin);
        let shown = options
            .iter()
            .find(|(types, _)| *types == reading.types)
            .map(|(_, word)| word.as_str())
            .unwrap_or_default();
        let picked = super::sheet::select(ui, "csv-types", shown, 280.0, skin).show(
            ui,
            skin.look,
            skin.palette,
            |ui| {
                let mut chosen = reading.types;
                for (types, word) in &options {
                    menu::value(ui, &mut chosen, *types, word, skin.look, skin.palette);
                }
                chosen
            },
        );
        picked.response.widget_info(|| {
            let mut info = egui::WidgetInfo::labeled(egui::WidgetType::ComboBox, true, &label);
            info.current_text_value = Some(shown.to_owned());
            info
        });
        if let Some(chosen) = picked.inner
            && chosen != reading.types
        {
            actions.push(Action::SetCsv(CsvChange::Types(chosen)));
        }
    });
    for (at, each) in reading.overrides.iter().enumerate() {
        ui.add_space(8.0);
        sheet_override(ui, at, each, skin, actions);
    }
}

/// macOS: an override's row, on the sunken tone: the file's name, what
/// changes, and Reset at its end.
fn sheet_override(ui: &mut Ui, at: usize, each: &Override, skin: &Skin, actions: &mut Vec<Action>) {
    let Skin { look, palette, .. } = *skin;
    let words = format!("· {}", override_words(&each.change, false));
    egui::Frame::new()
        .fill(palette.panel)
        .corner_radius(CornerRadius::same(8))
        .inner_margin(egui::Margin::symmetric(10, 8))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 6.0;
                widgets::label(
                    ui,
                    TextRole::MonoSecondary,
                    &each.file,
                    palette.secondary,
                    look,
                );
                widgets::label(ui, TextRole::Secondary, &words, palette.secondary, look);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let name = format!(
                        "{} {}, {}",
                        gettext(skin.locale, "Reset"),
                        each.file,
                        override_words(&each.change, false)
                    );
                    let reset = reset_link(ui, at, &skin.say("Reset"), &name, skin);
                    if reset.clicked() {
                        actions.push(Action::SetCsv(CsvChange::Reset(at)));
                    }
                });
            });
        });
}

/// macOS: Reset, as a link that says `text`, named `name`: the override
/// it resets.
fn reset_link(ui: &mut Ui, at: usize, text: &str, name: &str, skin: &Skin) -> egui::Response {
    let Skin { look, palette, .. } = *skin;
    let laid = Text::one(look, TextRole::Secondary, text, palette.accent).layout(ui.ctx());
    let (rect, _) = ui.allocate_exact_size(laid.size(), Sense::hover());
    let response = ui.interact(rect, ui.id().with(("csv-reset", at)), Sense::click());
    response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Link, true, name));
    laid.paint(ui.painter(), rect.min);
    focus::hint(ui, &response, rect, focus::Ring::Outer { radius: 3 });
    response.on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// The terminal look: the open section's rows.
fn terminal_rows(ui: &mut Ui, form: &ConnectionForm, skin: &Skin, actions: &mut Vec<Action>) {
    let Skin { look, palette, .. } = *skin;
    let reading = &form.files_csv;
    let group = Group::Words;
    let header = skin.say("Header row");
    super::terminal::terminal_row_of(ui, &header, group.height(), skin, |ui, _| {
        if let Some(chosen) = choice(
            ui,
            "csv-header",
            &header,
            &headers(skin),
            reading.header,
            group,
            skin,
        ) {
            actions.push(Action::SetCsv(CsvChange::Header(chosen)));
        }
    });
    let delimiter = skin.say("Delimiter");
    super::terminal::terminal_row_of(ui, &delimiter, group.height(), skin, |ui, _| {
        if let Some(chosen) = choice(
            ui,
            "csv-delimiter",
            &delimiter,
            &delimiters(skin),
            reading.delimiter,
            group,
            skin,
        ) {
            actions.push(Action::SetCsv(CsvChange::Delimiter(chosen)));
        }
    });
    let label = skin.say("Column types");
    super::terminal::terminal_row_of(ui, &label, group.height(), skin, |ui, _| {
        if let Some(chosen) = choice(
            ui,
            "csv-types",
            &label,
            &types(skin),
            reading.types,
            group,
            skin,
        ) {
            actions.push(Action::SetCsv(CsvChange::Types(chosen)));
        }
    });
    let line = TextRole::OBody.row_height(ui.ctx(), look.faces);
    let reset = crate::ui::keys::written(ui.ctx(), look, crate::keymap::Command::FormResetOverride);
    for (at, each) in reading.overrides.iter().enumerate() {
        let said = format!(
            "{} · {} · {} {}",
            each.file,
            override_words(&each.change, true),
            reset.as_str(),
            skin.say("reset")
        );
        super::terminal::terminal_row_of(ui, &skin.say("Override"), line, skin, |ui, _| {
            let laid = Text::one(look, TextRole::OBody, &said, palette.dim).layout(ui.ctx());
            let (rect, _) =
                ui.allocate_exact_size(vec2(ui.available_width(), line), Sense::hover());
            let response = ui.interact(rect, override_id(at), Sense::click());
            let name = format!(
                "{} {}, {}",
                skin.say("Override"),
                each.file,
                override_words(&each.change, true)
            );
            response
                .widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, &name));
            if focus::shown(&response) {
                ui.painter()
                    .rect_filled(rect, CornerRadius::ZERO, palette.selection);
            }
            laid.paint_left(ui.painter(), rect.left(), rect.center().y);
            focus::hint(ui, &response, rect, focus::Ring::Bar);
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_summary_says_each_choice_and_the_overrides() {
        let mut reading = Reading::default();
        assert_eq!(
            summary(&reading, false),
            "Header detect · delimiter detect · types from the first 20,000 rows"
        );
        assert_eq!(
            summary(&reading, true),
            "Header detect · delimiter detect · types first 20,000 rows"
        );
        reading.header = Header::Yes;
        reading.delimiter = Delimiter::Tab;
        reading.types = Types::Text;
        reading.overrides.push(Override {
            file: "returns.csv".into(),
            change: Change::Skip,
        });
        assert_eq!(
            summary(&reading, false),
            "Header yes · delimiter tab · everything as text · 1 override"
        );
        reading.types = Types::All;
        reading.delimiter = Delimiter::Pipe;
        reading.overrides.push(reading.overrides[0].clone());
        assert_eq!(
            summary(&reading, false),
            "Header yes · delimiter | · types from all rows · 2 overrides"
        );
    }

    #[test]
    fn an_override_says_what_it_changes() {
        let text = Change::Text {
            column: "returned_at".into(),
        };
        let empty = Change::Empty {
            column: "returned_at".into(),
            values: vec!["N/A".into(), "-".into()],
        };
        assert_eq!(override_words(&text, false), "returned_at read as text");
        assert_eq!(override_words(&text, true), "returned_at as text");
        assert_eq!(
            override_words(&empty, false),
            "'N/A', '-' read as empty in returned_at"
        );
        assert_eq!(
            override_words(&empty, true),
            "'N/A', '-' as empty in returned_at"
        );
        assert_eq!(override_words(&Change::Skip, false), "skipped");
    }
}

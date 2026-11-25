//! The Files form's Source: a folder, or a list of files, and what was
//! found there, in either look. The backend reads the folder (or each
//! file's size) after every change; this draws what it last read.

use crate::i18n::{Msgid, msgid};
use egui::{CornerRadius, Rect, Sense, Stroke, StrokeKind, Ui, pos2, vec2};

use super::choice::{Choice, Group, choose};
use super::{Skin, field_with_button, input};
use crate::model::{Action, ConnectionForm, FilesFound, FilesKind, SourceNames};
use crate::style::Icon;
use crate::typography::{Text, TextRole};
use crate::ui::{file_list, widgets};

/// What the Folder choice says under its field.
const FOLDER_HINT: Msgid = msgid(
    "Every CSV, Parquet, JSON, NDJSON, Excel and .duckdb file in the \
                           folder becomes a table. Files added later show up on the next open.",
);

/// What the hint adds while an override skips a file.
const SKIPPED_HINT: Msgid = msgid("Skipped files are left out.");

/// What the field says of a folder that is gone.
const GONE: Msgid = msgid("This folder isn't there any more. Choose it again, or pick another.");

/// The found line: how many files, their size and their formats, and in
/// the terminal look the tables they become after an arrow. `None` until
/// the Source is read. Words in the look's case come from `say`.
pub(super) fn found_line(found: &FilesFound, terminal: bool) -> Option<String> {
    let FilesFound::Read(scan) = found else {
        return None;
    };
    let count = scan.files.len();
    let files = match count {
        1 => "1 file".to_owned(),
        count => format!("{count} files"),
    };
    let formats: Vec<&str> = scan.formats().into_iter().map(|f| f.label()).collect();
    let mut line = format!(
        "{files} · {}",
        tabletist_db::files::size_text(scan.total_size())
    );
    if !formats.is_empty() {
        line = format!("{line} · {}", formats.join(", "));
    }
    if terminal {
        line = line.to_lowercase();
        if count > 0 {
            line = format!("{line} → {}", scan.names().join(", "));
        }
    }
    Some(line)
}

/// Why the Source cannot be read, as the field says it: a folder that is
/// gone, or that macOS's privacy settings refused, in the boards' words
/// (translated), anything else as the backend said it.
fn trouble(found: &FilesFound, skin: &Skin) -> Option<String> {
    match found {
        FilesFound::Failed(why) if why.ends_with(": no such folder") => Some(skin.say(GONE)),
        // macOS's privacy settings in the banner's words, naming the file
        // where it is one of a list's; anything else as it is, a path in it
        // keeping its case in either look.
        FilesFound::Failed(why) => Some(
            match crate::ui::not_allowed::NotAllowed::of(why, skin.look, skin.locale) {
                Some(refused) if refused.what == tabletist_db::files::Refused::File => {
                    format!("{} {}", refused.lead, refused.text)
                }
                Some(refused) => refused.text,
                None => why.clone(),
            },
        ),
        FilesFound::Unread | FilesFound::Reading(_) | FilesFound::Read(_) => None,
    }
}

/// The list's rows: each file of the list, with its format and size where
/// the backend found it, and marked where it is not there.
pub(super) fn rows(form: &ConnectionForm) -> Vec<file_list::Row> {
    let scan = match &form.files_found {
        FilesFound::Read(scan) => Some(scan),
        FilesFound::Unread | FilesFound::Reading(_) | FilesFound::Failed(_) => None,
    };
    form.files_list
        .iter()
        .map(|shown| {
            let path = tabletist_db::files::expanded(std::path::Path::new(shown));
            let found = scan.and_then(|scan| scan.files.iter().find(|file| file.path == path));
            let kind = tabletist_db::files::Format::of(&path)
                .map(|format| format.label().to_owned())
                .unwrap_or_default();
            file_list::Row {
                path: shown.clone(),
                kind,
                size: found
                    .map(|file| tabletist_db::files::size_text(file.size))
                    .unwrap_or_default(),
                missing: scan.is_some_and(|scan| scan.missing.contains(&path)),
            }
        })
        .collect()
}

/// The Source's kind, as a group of choices headed `heading`.
fn kind(
    ui: &mut Ui,
    form: &ConnectionForm,
    heading: &str,
    group: Group,
    skin: &Skin,
    actions: &mut Vec<Action>,
) {
    let labels = FilesKind::ALL.map(|kind| skin.say(kind.label()));
    let options: Vec<(FilesKind, Choice<'_>)> = FilesKind::ALL
        .iter()
        .zip(&labels)
        .map(|(kind, label)| (*kind, Choice::plain(label)))
        .collect();
    if let Some(kind) = choose(
        ui,
        "files-kind",
        Some(heading),
        &options,
        form.files_kind,
        group,
        skin,
    ) && kind != form.files_kind
    {
        actions.push(Action::SetFilesKind(kind));
    }
}

/// macOS: the folder as a read-only field with its icon, red where it is
/// gone.
fn folder_box(ui: &mut Ui, form: &ConnectionForm, skin: &Skin) -> egui::Response {
    let Skin { look, palette, .. } = *skin;
    let height = skin.field_height();
    let (rect, response) =
        ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::hover());
    let gone = trouble(&form.files_found, skin).is_some();
    let corner = CornerRadius::same(look.radius.saturating_sub(1));
    ui.painter()
        .rect_filled(rect, corner, palette.surface_hover);
    ui.painter().rect_stroke(
        rect,
        corner,
        Stroke::new(
            1.0,
            if gone {
                palette.danger
            } else {
                palette.outline
            },
        ),
        StrokeKind::Inside,
    );
    let center = rect.center().y;
    Icon::Folder.image(palette.dim, 14.0).paint_at(
        ui,
        Rect::from_center_size(pos2(rect.left() + 17.0, center), vec2(14.0, 14.0)),
    );
    let (text, color) = match form.files_folder.trim() {
        "" => (skin.say(msgid("No folder chosen")), palette.dim),
        folder => (folder.to_owned(), palette.text),
    };
    let role = TextRole::MonoSecondary;
    let room = rect.width() - 42.0;
    let shown = crate::ui::grid::ellipsize(&text, room, true, |text| {
        role.width(ui.ctx(), look.faces, text)
    });
    widgets::paint_label(
        ui,
        rect.left() + 32.0,
        center,
        Text::one(look, role, &shown, color),
    );
    widgets::announce(ui, rect, &format!("{} {text}", skin.say(msgid("Folder"))));
    response
}

/// What the Source holds as the found line says it: a file the Reading
/// CSV section skips is left out, and the others named as the connection
/// names them without it (`orders`, not `orders_csv`, once `orders.tsv`
/// is skipped).
fn read(form: &ConnectionForm) -> FilesFound {
    let mut found = form.files_found.clone();
    if let FilesFound::Read(scan) = &mut found {
        let reading = &form.files_csv;
        scan.files.retain(|file| !reading.skips(&file.path));
        scan.files = tabletist_db::files::renamed(std::mem::take(&mut scan.files));
    }
    found
}

/// macOS: the "Source" fieldset's inside.
pub(super) fn sheet(
    ui: &mut Ui,
    form: &mut ConnectionForm,
    skin: &Skin,
    actions: &mut Vec<Action>,
) {
    let Skin { look, palette, .. } = *skin;
    let small = TextRole::Secondary;
    kind(
        ui,
        form,
        &skin.say(msgid("Source")),
        Group::Track(24.0),
        skin,
        actions,
    );
    ui.add_space(10.0);
    match form.files_kind {
        FilesKind::Folder => {
            let (_, clicked) = field_with_button(
                ui,
                &skin.say(msgid("Choose…")),
                msgid("Choose a folder"),
                skin,
                |ui| folder_box(ui, form, skin),
            );
            if clicked {
                actions.push(Action::PickFolder);
            }
        }
        FilesKind::Files => {
            let rows = rows(form);
            if !rows.is_empty() {
                let asked = file_list::show(
                    ui,
                    &skin.say(msgid("Files")),
                    &rows,
                    form.files_cursor,
                    (look, palette, skin.locale),
                );
                if let Some(at) = asked.removed {
                    actions.push(Action::RemoveFile(at));
                }
                if let Some(at) = asked.chosen {
                    form.files_cursor = Some(at);
                }
                ui.add_space(8.0);
            }
            let name = crate::i18n::gettext(skin.locale, "Add files");
            let text = skin.say(msgid("Add files…"));
            let spec = super::side_button(&text, &name, skin);
            if spec.show(ui, skin.field_height(), look, palette).clicked() {
                actions.push(Action::PickDataFiles);
            }
        }
    }
    ui.add_space(8.0);
    match trouble(&form.files_found, skin) {
        Some(why) => {
            super::note(ui, &why, palette.danger, skin);
        }
        None => {
            let found = read(form);
            if let Some(line) = found_line(&found, false) {
                widgets::label(ui, small, &line, palette.secondary, look);
                if let FilesFound::Read(scan) = &found
                    && !scan.files.is_empty()
                {
                    ui.add_space(6.0);
                    file_list::tags(ui, &scan.names(), look, palette);
                }
            }
            match form.files_kind {
                FilesKind::Folder => {
                    ui.add_space(6.0);
                    // The boards' words, and that a skipped file is no
                    // table while one is.
                    let skips = form
                        .files_csv
                        .overrides
                        .iter()
                        .any(|each| matches!(each.change, tabletist_db::files::Change::Skip));
                    let hint = match skips {
                        true => format!("{} {}", skin.say(FOLDER_HINT), skin.say(SKIPPED_HINT)),
                        false => skin.say(FOLDER_HINT),
                    };
                    super::note(ui, &hint, palette.dim, skin);
                }
                FilesKind::Files => {}
            }
        }
    }
}

/// The path line names are typed into for the Source's `kind`, in the
/// terminal look: the folder field, or the list's add line. Each its own,
/// so neither takes the other's text back with undo.
pub(crate) fn source_line_id(kind: FilesKind) -> egui::Id {
    match kind {
        FilesKind::Folder => egui::Id::new("files-folder-line"),
        FilesKind::Files => egui::Id::new("files-add-line"),
    }
}

/// The names listed under the path line, over the dialog.
fn names_id() -> egui::Id {
    egui::Id::new("files-source-names")
}

/// The names under the path line `field` (the "Omarchy – Files, path
/// completions" board), and what they ask for.
fn names(
    ui: &Ui,
    field: &egui::Response,
    form: &ConnectionForm,
    skin: &Skin,
    actions: &mut Vec<Action>,
) -> crate::ui::path_list::Asked {
    let asked = crate::ui::path_list::under(
        ui,
        field,
        form.source_line().0,
        form.files_names.as_ref(),
        names_id(),
        (skin.look, skin.palette),
    );
    if asked.close {
        actions.push(Action::SourceNames(SourceNames::Close));
    }
    if let Some(at) = asked.taken {
        actions.push(Action::SourceNames(SourceNames::Take(Some(at))));
    }
    asked
}

/// The terminal look: the "source" section's rows. The folder is typed,
/// its folders listed under it as it is, or picked (`ctrl+o`); the list's
/// rows are moved through with the arrows, `a` adds files and `x` takes
/// the row off, and a file's path is typed in the add line under it.
pub(super) fn terminal(
    ui: &mut Ui,
    form: &mut ConnectionForm,
    skin: &Skin,
    actions: &mut Vec<Action>,
) {
    let Skin { look, palette, .. } = *skin;
    let role = TextRole::OBody;
    let line = role.row_height(ui.ctx(), look.faces);
    super::terminal::terminal_section(ui, &skin.say(msgid("Source")), skin);
    // The row says `kind`, as the board writes it; a screen reader hears
    // the choice as "Source", as on macOS.
    let source = skin.say(msgid("Source"));
    super::terminal::terminal_row_of(
        ui,
        &skin.say(msgid("Kind")),
        Group::Words.height(),
        skin,
        |ui, _| {
            kind(ui, form, &source, Group::Words, skin, actions);
        },
    );
    match form.files_kind {
        FilesKind::Folder => {
            let pick = crate::ui::keys::written(ui.ctx(), look, crate::keymap::Command::FormPick);
            super::terminal::terminal_row(ui, &skin.say(msgid("Folder")), skin, |ui, label| {
                let hint = super::hint(
                    ui,
                    &format!("{} {}", pick.as_str(), skin.say(msgid("pick"))),
                    role,
                    skin,
                );
                let id = source_line_id(FilesKind::Folder);
                crate::ui::path_list::to_line(ui.ctx(), id, form.files_to_line, &form.files_folder);
                let field = ui
                    .add(
                        input(ui, &mut form.files_folder, role, skin.field_height(), skin)
                            .id(id)
                            .hint_text(hint),
                    )
                    .labelled_by(label);
                if field.changed() {
                    actions.push(Action::SourceNames(SourceNames::Typed));
                }
                let asked = names(ui, &field, form, skin, actions);
                // A click on a name is no leaving: the folder is read when
                // the keyboard goes on.
                if field.lost_focus() && !asked.on_list {
                    actions.push(Action::ReadSource);
                }
            });
        }
        FilesKind::Files => {
            let rows = rows(form);
            if rows.is_empty() {
                super::terminal::terminal_row_of(
                    ui,
                    &skin.say(msgid("Files")),
                    line,
                    skin,
                    |ui, _| {
                        let add = crate::ui::keys::written(
                            ui.ctx(),
                            look,
                            crate::keymap::Command::FormAddFiles,
                        );
                        widgets::label(
                            ui,
                            TextRole::OSecondary,
                            &skin
                                .say(msgid("none · {key} adds files"))
                                .replace("{key}", add.as_str()),
                            palette.dim,
                            look,
                        );
                    },
                );
            } else {
                let height = rows.len() as f32 * 26.0 + 2.0;
                super::terminal::terminal_row_of(
                    ui,
                    &skin.say(msgid("Files")),
                    height,
                    skin,
                    |ui, _| {
                        let asked = file_list::show(
                            ui,
                            &skin.say(msgid("Files")),
                            &rows,
                            form.files_cursor.or(Some(0)),
                            (look, palette, skin.locale),
                        );
                        if let Some(at) = asked.removed {
                            actions.push(Action::RemoveFile(at));
                        }
                        if let Some(at) = asked.chosen {
                            form.files_cursor = Some(at);
                        }
                    },
                );
            }
            add_line(ui, form, skin, actions);
        }
    }
    let (text, color) = match trouble(&form.files_found, skin) {
        Some(why) => (Some(why), palette.danger),
        None => (found_line(&read(form), true), palette.dim),
    };
    if let Some(text) = text {
        super::terminal::terminal_row_of(ui, "", line, skin, |ui, _| {
            Text::one(look, TextRole::OSecondary, &text, color)
                .wrap(ui.available_width())
                .layout(ui.ctx())
                .label(ui);
        });
    }
}

/// The line under the list a file's path is typed in: its folders and
/// the files a Files connection reads listed under it as it is, Enter on
/// one taking it, and Enter with none listed adding the path as typed.
fn add_line(ui: &mut Ui, form: &mut ConnectionForm, skin: &Skin, actions: &mut Vec<Action>) {
    let look = skin.look;
    let role = TextRole::OBody;
    let pick = crate::ui::keys::written(ui.ctx(), look, crate::keymap::Command::FormPick);
    super::terminal::terminal_row(ui, &skin.say(msgid("Add")), skin, |ui, label| {
        let hint = super::hint(
            ui,
            &format!("{} {}", pick.as_str(), skin.say(msgid("pick"))),
            role,
            skin,
        );
        let id = source_line_id(FilesKind::Files);
        crate::ui::path_list::to_line(ui.ctx(), id, form.files_to_line, &form.files_add);
        let field = ui
            .add(
                input(ui, &mut form.files_add, role, skin.field_height(), skin)
                    .id(id)
                    .hint_text(hint),
            )
            .labelled_by(label);
        if field.changed() {
            actions.push(Action::SourceNames(SourceNames::Typed));
        }
        let asked = names(ui, &field, form, skin, actions);
        // Enter with no name listed (the list's Enter is the dialog's,
        // read before it draws): the path as typed.
        let entered = ui.input(|input| input.key_pressed(egui::Key::Enter));
        if field.lost_focus() && entered && !asked.on_list {
            actions.push(Action::SourceNames(SourceNames::Add));
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use tabletist_db::files::{Format, Found, Scan};

    fn scan() -> Scan {
        let found = |name: &str, format: Format, size: u64| Found {
            path: format!("/data/{name}").into(),
            format,
            size,
            name: name.split('.').next().unwrap_or_default().to_owned(),
        };
        Scan {
            files: vec![
                found("orders.csv", Format::Csv, 1_600_000_000),
                found("customers.parquet", Format::Parquet, 400_000_000),
                found("events.ndjson", Format::Ndjson, 450_000_000),
                found("returns.csv", Format::Csv, 50_000_000),
            ],
            missing: Vec::new(),
            unreadable: Vec::new(),
        }
    }

    #[test]
    fn the_found_line_says_the_files_their_size_and_formats() {
        let found = FilesFound::Read(scan());
        assert_eq!(
            found_line(&found, false).as_deref(),
            Some("4 files · 2.5 GB · CSV, Parquet, NDJSON")
        );
        assert_eq!(
            found_line(&found, true).as_deref(),
            Some("4 files · 2.5 gb · csv, parquet, ndjson → orders, customers, events, returns")
        );
        assert_eq!(found_line(&FilesFound::Unread, false), None);
        assert_eq!(
            found_line(&FilesFound::Read(Scan::default()), true).as_deref(),
            Some("0 files · 0 b")
        );
    }

    #[test]
    fn a_folder_that_is_gone_is_said_in_the_boards_words() {
        let look = crate::theme::Look::macos();
        let palette = crate::theme::Palette::light();
        let skin = Skin::new(
            &look,
            &palette,
            crate::i18n::Locale::default(),
            crate::env::Environment::Local,
        );
        let gone = FilesFound::Failed("Can't open ~/Data/bookshop-exports: no such folder".into());
        assert_eq!(trouble(&gone, &skin).as_deref(), Some(GONE.english()));
        let other = FilesFound::Failed("Can't open ~/x: not allowed to read it".into());
        assert_eq!(
            trouble(&other, &skin).as_deref(),
            Some("Can't open ~/x: not allowed to read it")
        );
        assert_eq!(trouble(&FilesFound::Read(scan()), &skin), None);
    }
}

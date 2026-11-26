//! A Files connection's Join with saved connections section, in either
//! look: the saved PostgreSQL, MySQL and SQLite connections it can
//! attach, to join a file to their live tables in one query, each by the
//! name SQL calls it by. Closed, it says which are attached.

use egui::{CornerRadius, Ui};
use tabletist_db::Driver;

use super::Skin;
use crate::connections::join::{Blocked, Joinable};
use crate::i18n::{Locale, Msgid, gettext, msgid};
use crate::model::{Action, ConnectionForm};
use crate::typography::TextRole;
use crate::ui::attach_list::{self, Row};
use crate::ui::disclosure::{self, Section};
use crate::ui::widgets;

/// What the open section says under its title.
const HINT: Msgid =
    msgid("Attach a saved connection to join a file to its live tables in one query. Optional.");

/// What the open section says under the list: an attached connection
/// keeps its own safety, read-only as the files are, or though they are
/// not.
const SAFETY: Msgid = msgid("Attached connections open read-only here, as the files do.");
const SAFETY_EDITING: Msgid =
    msgid("Attached connections open read-only here, even when the files are editable.");

/// The closed row's summary: none attached, else their names.
pub(super) fn summary(form: &ConnectionForm, locale: Locale) -> String {
    match form.files_attach.as_slice() {
        [] => gettext(locale, "None attached · optional").into_owned(),
        attached => {
            let names: Vec<&str> = attached.iter().map(|each| each.name.as_str()).collect();
            gettext(locale, "Attached as {names}").replace("{names}", &names.join(", "))
        }
    }
}

/// The engine's name, as a row says it.
fn engine(driver: Driver) -> &'static str {
    match driver {
        Driver::Postgres => "PostgreSQL",
        Driver::MySql => "MySQL",
        Driver::Sqlite => "SQLite",
        Driver::BigQuery => "BigQuery",
        Driver::Files => "Files",
    }
}

/// Why a row cannot be ticked, in `locale`.
fn blocked(why: Blocked, locale: Locale) -> String {
    let words = match why {
        Blocked::AsksPassword => msgid("Asks for its password on every connect"),
        Blocked::NoDatabase => msgid("Names no database"),
    };
    words.say(locale).into_owned()
}

/// The sample query, for the first attached connection's `name`.
fn sample(name: &str) -> String {
    // Quoted where SQL would read it as a keyword: `select`.
    let name = tabletist_db::Dialect::DuckDb.ident(name);
    format!(
        "SELECT r.order_id, p.name\n  FROM returns r\n  JOIN {name}.publishers p ON p.id = r.publisher_id"
    )
}

/// The section after Reading CSV, in either look: the saved connections
/// the form lists ([`ConnectionForm::joinable`]), in the list's order.
pub(super) fn section(
    ui: &mut Ui,
    form: &mut ConnectionForm,
    skin: &Skin,
    actions: &mut Vec<Action>,
) {
    let joinable = std::mem::take(&mut form.joinable);
    let Skin { look, palette, .. } = *skin;
    let title = skin.say(msgid("Join with saved connections"));
    let summary = look.label(&summary(form, skin.locale));
    let hint = match look.terminal {
        true => skin.say(msgid("optional")),
        false => String::new(),
    };
    let opens = skin.opens(ui.ctx());
    let section = Section {
        salt: "join",
        title: &title,
        summary: &summary,
        hint: &hint,
        opens: &opens,
    };
    let mut open = form.join_open;
    disclosure::show(
        ui,
        &section,
        &mut open,
        look,
        palette,
        skin.fill,
        skin.rule(),
        |ui| rows(ui, form, &joinable, skin, actions),
    );
    form.join_open = open;
    form.joinable = joinable;
}

/// The open section: the hint (macOS), the list, the sample query and the
/// safety line.
fn rows(
    ui: &mut Ui,
    form: &mut ConnectionForm,
    joinable: &[Joinable],
    skin: &Skin,
    actions: &mut Vec<Action>,
) {
    let Skin { look, palette, .. } = *skin;
    let small = TextRole::pick(look, TextRole::Secondary, TextRole::OSecondary);
    widgets::label(ui, small, &look.label(&skin.say(HINT)), palette.dim, look);
    ui.add_space(8.0);
    let first = form.files_attach.first().map(|each| each.name.clone());
    if joinable.is_empty() {
        let none = skin.say(msgid(
            "No saved PostgreSQL, MySQL or SQLite connection to attach.",
        ));
        widgets::label(ui, small, &none, palette.dim, look);
    } else {
        let typed: Vec<String> = form
            .files_attach
            .iter()
            .map(|each| each.name.clone())
            .collect();
        let mut names: std::collections::HashMap<&str, &mut String> = form
            .files_attach
            .iter_mut()
            .map(|each| (each.connection.as_str(), &mut each.name))
            .collect();
        let mut listed: Vec<Row<'_>> = joinable
            .iter()
            .map(|each| Row {
                name: &each.name,
                environment: each.environment,
                engine: engine(each.driver),
                database: &each.database,
                read_only: each.read_only,
                blocked: each
                    .blocked
                    .map(|why| look.label(&blocked(why, skin.locale))),
                attached: names.remove(each.id.0.as_str()),
            })
            .collect();
        let name = skin.say(msgid("Saved connections to attach"));
        let mut draw =
            |ui: &mut Ui| attach_list::show(ui, &name, &mut listed, (look, palette, skin.locale));
        let toggled = match look.terminal {
            true => {
                let mut toggled = None;
                let label = skin.say(msgid("Attach"));
                let height = 28.0 * joinable.len() as f32;
                super::terminal::terminal_row_of(ui, &label, height, skin, |ui, _| {
                    toggled = draw(ui);
                });
                toggled
            }
            false => draw(ui),
        };
        drop(listed);
        // A name typed over: a Test of the names before says nothing of it.
        if !form
            .files_attach
            .iter()
            .map(|each| &each.name)
            .eq(typed.iter())
        {
            actions.push(Action::AttachNameTyped);
        }
        if let Some(at) = toggled {
            actions.push(Action::ToggleAttach(joinable[at].id.clone()));
        }
    }
    // SQL keeps its case in either look.
    if let Some(name) = first {
        ui.add_space(8.0);
        let role = TextRole::pick(look, TextRole::MonoSecondary, TextRole::OBody);
        egui::Frame::new()
            .fill(match look.terminal {
                true => palette.window,
                false => palette.panel,
            })
            .corner_radius(CornerRadius::same(if look.terminal { 0 } else { 8 }))
            .inner_margin(egui::Margin::symmetric(10, 8))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                for line in sample(&name).lines() {
                    widgets::label(ui, role, line, palette.secondary, look);
                }
            });
    }
    ui.add_space(8.0);
    let safety = match form.read_only() {
        true => SAFETY,
        false => SAFETY_EDITING,
    };
    let safety = look.label(&skin.say(safety));
    widgets::label(ui, small, &safety, palette.dim, look);
}

#[cfg(test)]
mod tests {
    use super::*;
    use tabletist_db::files::Attach;

    #[test]
    fn the_summary_says_what_is_attached() {
        let mut form = ConnectionForm::default();
        assert_eq!(summary(&form, Locale::English), "None attached · optional");
        form.files_attach = vec![
            Attach {
                connection: "a".into(),
                name: "bookshop_dev".into(),
            },
            Attach {
                connection: "b".into(),
                name: "bookshop_production".into(),
            },
        ];
        assert_eq!(
            summary(&form, Locale::English),
            "Attached as bookshop_dev, bookshop_production"
        );
    }

    #[test]
    fn the_sample_uses_the_first_name() {
        assert_eq!(
            sample("bookshop_dev"),
            "SELECT r.order_id, p.name\n  FROM returns r\n  JOIN bookshop_dev.publishers p ON \
             p.id = r.publisher_id"
        );
        assert!(sample("select").contains("JOIN \"select\".publishers"));
    }
}

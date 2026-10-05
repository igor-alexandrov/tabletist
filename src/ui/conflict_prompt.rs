//! The question about the rows a save found changed on the server: what
//! to do with each, one after another.

use egui::{Id, Key, Modifiers};

use crate::app::App;
use crate::edit::Answer;
use crate::i18n::{Locale, gettext};
use crate::model::{Action, Dialog, ObjectTab, Workspace};
use crate::theme::Look;
use crate::ui::format;
use crate::ui::widgets::{self, ButtonSpec};

/// What the question about a row is headed with, in the parts its line is
/// laid out from: only the row's name gives way when the line is too long.
pub struct Title {
    /// "Row", in the look's case.
    pub before: String,
    /// The row by its key, as the row panel names it: `id 2`. The key is
    /// the database's, and stays as it is.
    pub name: String,
    /// What became of the row, in the look's case.
    pub after: String,
}

impl Title {
    /// The whole of it: what a screen reader and the pointer are told.
    pub fn whole(&self) -> String {
        format!("{} {} {}", self.before, self.name, self.after)
    }
}

/// The heading of the question about the page's row `row`.
pub fn title(object: &ObjectTab, row: usize, gone: bool, look: &Look, locale: Locale) -> Title {
    let say = |text: &'static str| look.label(&gettext(locale, text));
    Title {
        before: say("Row"),
        name: super::pending_bar::row_name(object, row),
        after: if gone {
            say("no longer exists on the server")
        } else {
            say("changed on the server")
        },
    }
}

/// Where the row is: the connection's name and the table's, since a table
/// of one name can be open on two connections and the question can come
/// up over either. With `of` rows found changed by the save, also which of
/// them this is: "· 1 of 2".
pub fn place(
    workspace: &Workspace,
    object: &ObjectTab,
    (at, of): (usize, usize),
    look: &Look,
    locale: Locale,
) -> String {
    let table = format::display_safe(&object.object.name);
    let mut place = format!("{} · {table}", workspace.name);
    if of > 1 {
        let word = look.label(&gettext(locale, "of"));
        place.push_str(&format!(" · {} {word} {of}", at + 1));
    }
    place
}

/// The answers the question about a row offers, by the names of their
/// buttons: one for a row that is gone, three for one that changed.
pub fn answers(gone: bool) -> &'static [(&'static str, Answer)] {
    if gone {
        &[("Discard my changes", Answer::Discard)]
    } else {
        &[
            ("Keep mine, reload row", Answer::KeepMine),
            ("Use server values", Answer::UseServer),
            ("Overwrite", Answer::Overwrite),
        ]
    }
}

pub fn show(app: &mut App, ctx: &egui::Context) {
    let (look, palette, locale) = (app.look, app.palette, app.locale);
    let Some(Dialog::Conflict(prompt)) = &app.dialog else {
        return;
    };
    let at = prompt.at;
    let shown = app.workspace(prompt.tab).and_then(|workspace| {
        let object = workspace.object_tab(prompt.id)?;
        Some((workspace, object, prompt.rows.get(at)?))
    });
    let Some((workspace, object, conflict)) = shown else {
        // Nothing to draw, so nothing to answer it with: it closes, or it
        // would keep the keyboard for good.
        app.actions.push(Action::CloseDialog);
        return;
    };
    let gone = conflict.server.is_none();
    let title = title(object, conflict.row, gone, &look, locale).whole();
    let place = place(workspace, object, (at, prompt.rows.len()), &look, locale);
    // No answer in the question's first moment: what was on its way to
    // the grid when it came up is not one. It is dropped without a sign.
    let ripe = crate::edit::answers_taken(prompt.shown);
    let mut actions = Vec::new();
    let modal = widgets::modal(Id::new("conflict-prompt"), &look, &palette).show(ctx, |ui| {
        let role = widgets::dialog_title(&look);
        widgets::label(ui, role, &title, palette.text, &look);
        ui.add_space(4.0);
        widgets::label(ui, widgets::body(&look), &place, palette.secondary, &look);
        ui.add_space(14.0);
        for (name, answer) in answers(gone) {
            let name = gettext(locale, name);
            let button = ButtonSpec::new(&name);
            if button.show(ui, 30.0, &look, &palette).clicked() {
                let answer = *answer;
                actions.push(Action::AnswerConflict { at, answer });
            }
        }
    });
    // Esc is Keep mine for the row shown: nothing of the user's is dropped
    // and nothing is written.
    let escape = |input: &mut egui::InputState| input.consume_key(Modifiers::NONE, Key::Escape);
    if modal.is_top_modal && ctx.input_mut(escape) {
        let answer = Answer::KeepMine;
        actions.push(Action::AnswerConflict { at, answer });
    }
    if ripe {
        app.actions.extend(actions);
    }
}

#[cfg(test)]
mod tests {
    use tabletist_db::{Conflict, Value, WriteOutcome};

    use crate::backend::Command;
    use crate::model::{Action, CellPos, ConnTabId, Dialog, EditStart, TabId};
    use crate::testing::Harness;
    use crate::theme::Look;
    use crate::ui::tests::click_dialog;

    /// The looks that ask the question. The terminal look keeps the line
    /// until its box is drawn.
    fn looks() -> impl Iterator<Item = Look> {
        Look::ALL.into_iter().filter(|look| !look.terminal)
    }

    /// Makes `text` the pending email of the row `id 2` of the table `id`.
    fn retype(harness: &mut Harness, tab: ConnTabId, id: TabId, text: &str) {
        harness.app.apply(Action::EditCell {
            tab,
            id,
            cell: CellPos { row: 1, col: 1 },
            start: EditStart::Replace(text.into()),
        });
        harness.app.apply(Action::LeaveEdit { tab, id });
    }

    /// The fixture's row `id 2` as the server holds it with `email`.
    fn server(email: &str) -> Vec<Value> {
        vec![Value::Int(2), Value::Text(email.into()), Value::Null]
    }

    /// The fixture's table in `look`, the email of its row `id 2` pending
    /// as `bob@example.com` and saved, and the save answered: the row
    /// holds `email` now, or is gone. The question is up, and has been for
    /// long enough to take an answer.
    fn conflict_in(look: Look, email: Option<&str>) -> (Harness, ConnTabId, TabId) {
        let mut harness = Harness::new();
        harness.set_look(look);
        let (tab, id) = harness.editable();
        retype(&mut harness, tab, id, "bob@example.com");
        harness.app.apply(Action::WriteEdits { tab, id });
        let conflict = Conflict {
            row: 0,
            server: email.map(server),
        };
        harness.answer_written(Ok(WriteOutcome::Conflicts(vec![conflict])));
        harness.finish_animations();
        assert!(asking(&harness), "{}", look.name);
        shown(&mut harness, true);
        (harness, tab, id)
    }

    /// Makes the question look as if it had been on screen for a while
    /// (`long`), or as if it had only just come up, however long the test
    /// has taken.
    fn shown(harness: &mut Harness, long: bool) {
        let now = std::time::Instant::now();
        let hour = std::time::Duration::from_secs(3600);
        if let Some(Dialog::Conflict(prompt)) = &mut harness.app.dialog {
            prompt.shown = if long {
                now.checked_sub(crate::edit::ANSWER_AFTER)
                    .expect("an earlier instant")
            } else {
                now + hour
            };
        }
    }

    fn asking(harness: &Harness) -> bool {
        matches!(harness.app.dialog, Some(Dialog::Conflict(_)))
    }

    fn writes(harness: &Harness) -> usize {
        let sent = harness.app.backend.sent.iter();
        sent.filter(|command| matches!(command, Command::Write { .. }))
            .count()
    }

    /// How many cells are pending, and the email the page holds for the
    /// row `id 2`.
    fn state(harness: &Harness, tab: ConnTabId, id: TabId) -> (usize, Value) {
        let workspace = harness.app.workspace(tab).unwrap();
        let object = workspace.object_tab(id).unwrap();
        let email = object.page().unwrap().rows[1][1].clone();
        (object.edits.cells.len(), email)
    }

    fn text(value: &str) -> Value {
        Value::Text(value.into())
    }

    fn painted(harness: &Harness, text: &str) -> bool {
        harness.painted.iter().any(|(piece, _)| piece == text)
    }

    #[test]
    fn each_answer_is_a_button_and_escape_keeps_mine() {
        for look in looks() {
            let said = look.name;
            let eve = || text("eve@example.com");
            let (mut harness, tab, id) = conflict_in(look, Some("eve@example.com"));
            click_dialog(&mut harness, "Use server values");
            assert!(!asking(&harness), "{said}");
            assert_eq!(state(&harness, tab, id), (0, eve()), "{said}");
            let (mut harness, tab, id) = conflict_in(look, Some("eve@example.com"));
            click_dialog(&mut harness, "Keep mine, reload row");
            assert!(!asking(&harness), "{said}");
            assert_eq!(state(&harness, tab, id), (1, eve()), "{said}");
            assert_eq!(writes(&harness), 1, "{said}: nothing is saved again");
            let (mut harness, tab, id) = conflict_in(look, Some("eve@example.com"));
            click_dialog(&mut harness, "Overwrite");
            assert!(!asking(&harness), "{said}");
            assert_eq!(state(&harness, tab, id), (1, eve()), "{said}");
            // Esc is Keep mine.
            let (mut harness, tab, id) = conflict_in(look, Some("eve@example.com"));
            harness.press(egui::Key::Escape, egui::Modifiers::NONE);
            assert!(!asking(&harness), "{said}");
            assert_eq!(state(&harness, tab, id), (1, eve()), "{said}");
            assert_eq!(writes(&harness), 1, "{said}");
            // A row that is gone has one answer. Esc leaves its change
            // pending, and the bar says that the row is not there (the
            // terminal look's status line, in its own words).
            let loaded = || text("user2@example.com");
            let (mut harness, tab, id) = conflict_in(look, None);
            assert!(!harness.has("Overwrite"), "{said}");
            harness.press(egui::Key::Escape, egui::Modifiers::NONE);
            assert!(!asking(&harness), "{said}");
            assert_eq!(state(&harness, tab, id), (1, loaded()), "{said}");
            let line = if look.terminal {
                "conflict row id 2 no longer exists on the server. nothing was written."
            } else {
                "Row id 2 no longer exists on the server. Nothing was written."
            };
            assert!(harness.has(line), "{said}");
            let (mut harness, tab, id) = conflict_in(look, None);
            click_dialog(&mut harness, "Discard my changes");
            assert!(!asking(&harness), "{said}");
            assert_eq!(state(&harness, tab, id), (0, loaded()), "{said}");
            let workspace = harness.app.workspace(tab).unwrap();
            let edits = &workspace.object_tab(id).unwrap().edits;
            assert!(edits.gone.contains(&1) && edits.note.is_none(), "{said}");
        }
    }

    #[test]
    fn an_answer_in_the_questions_first_moment_is_not_taken() {
        for look in looks() {
            let said = look.name;
            let (mut harness, tab, id) = conflict_in(look, Some("eve@example.com"));
            shown(&mut harness, false);
            click_dialog(&mut harness, "Use server values");
            harness.press(egui::Key::Escape, egui::Modifiers::NONE);
            assert!(asking(&harness), "{said}");
            let loaded = text("user2@example.com");
            assert_eq!(state(&harness, tab, id), (1, loaded), "{said}");
            // Once it has been on screen for a moment, it is.
            shown(&mut harness, true);
            click_dialog(&mut harness, "Use server values");
            assert!(!asking(&harness), "{said}");
        }
    }

    #[test]
    fn the_question_is_headed_with_the_row_by_its_key() {
        let (harness, tab, id) = conflict_in(Look::standard(), Some("eve@example.com"));
        let workspace = harness.app.workspace(tab).unwrap();
        let object = workspace.object_tab(id).unwrap();
        let locale = harness.app.locale;
        let title = |gone: bool, look: Look| super::title(object, 1, gone, &look, locale);
        let changed = title(false, Look::standard());
        assert_eq!(changed.whole(), "Row id 2 changed on the server");
        // The name is a part of its own: it is what gives way in a line
        // that is too long.
        assert_eq!(changed.name, "id 2");
        assert_eq!(
            title(true, Look::standard()).whole(),
            "Row id 2 no longer exists on the server"
        );
        // The terminal's lower case is for the app's own words.
        assert_eq!(
            title(false, Look::omarchy()).whole(),
            "row id 2 changed on the server"
        );
        assert!(
            painted(&harness, "Row id 2 changed on the server"),
            "{:?}",
            harness.painted
        );
    }

    #[test]
    fn the_question_says_which_connection_and_table_its_row_is_of() {
        for look in looks() {
            let said = look.name;
            // Two connections, each with a table `users` open.
            let mut harness = Harness::new();
            harness.set_look(look);
            let (first, users) = harness.editable();
            harness.app.apply(Action::ShowConnections);
            let (second, others) = harness.editable();
            harness.app.workspace_mut(second).unwrap().name = "Bookshop".into();
            // A save of the first is answered while the second shows.
            retype(&mut harness, first, users, "bob@example.com");
            harness.app.apply(Action::WriteEdits {
                tab: first,
                id: users,
            });
            let changed = |row: usize, email: &str| Conflict {
                row,
                server: Some(server(email)),
            };
            let conflict = changed(0, "eve@example.com");
            harness.answer_written(Ok(WriteOutcome::Conflicts(vec![conflict])));
            harness.finish_animations();
            assert!(painted(&harness, "Fixture · users"), "{said}");
            assert!(!painted(&harness, "Bookshop · users"), "{said}");
            shown(&mut harness, true);
            click_dialog(&mut harness, "Use server values");
            // The same row of the other connection's table.
            retype(&mut harness, second, others, "bob@example.com");
            harness.app.apply(Action::WriteEdits {
                tab: second,
                id: others,
            });
            let conflict = changed(0, "eve@example.com");
            harness.answer_written(Ok(WriteOutcome::Conflicts(vec![conflict])));
            harness.finish_animations();
            assert!(painted(&harness, "Bookshop · users"), "{said}");
            assert!(!painted(&harness, "Fixture · users"), "{said}");
        }
        // With several rows, which of them: in the look's case.
        let (harness, tab, id) = conflict_in(Look::standard(), Some("eve@example.com"));
        let workspace = harness.app.workspace(tab).unwrap();
        let object = workspace.object_tab(id).unwrap();
        let locale = harness.app.locale;
        let place =
            |at: (usize, usize)| super::place(workspace, object, at, &Look::standard(), locale);
        assert_eq!(place((0, 1)), "Fixture · users");
        assert_eq!(place((0, 2)), "Fixture · users · 1 of 2");
        assert_eq!(place((1, 2)), "Fixture · users · 2 of 2");
    }
}

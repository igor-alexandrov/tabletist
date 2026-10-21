//! The Keyboard pane of the Settings window: its list's search, the
//! recorder, and what a recorded key does to the settings' keys. A key is
//! judged by the keymap, laid over the settings' keys with the new line as
//! the file would be ([`Keymap::with`]): what the page refuses, the file
//! would refuse, for the same reason.

use super::*;
use crate::keymap::{Fixed, Group, Keymap, Layout, Line, Row, Why, fixed, spell};
use crate::model::{KeyboardPage, Recorded, Recording};

/// The name of the cheat sheet the save panel offers.
pub const CHEAT_SHEET: &str = "Tabletist shortcuts.md";

/// The full list's file, in the state folder: written again each time.
const KEYS_LIST: &str = "keys.default.toml";

impl App {
    /// The Keyboard pane's state, while the Settings window is open.
    fn keyboard_page(&mut self) -> Option<&mut KeyboardPage> {
        match &mut self.dialog {
            Some(Dialog::Settings(dialog)) => Some(&mut dialog.keyboard),
            _ => None,
        }
    }

    /// What one of the Keyboard pane's actions does.
    pub(super) fn keyboard_action(&mut self, action: Action) {
        match action {
            Action::KeyboardSearch(text) => {
                if let Some(page) = self.keyboard_page() {
                    page.search = text;
                    page.found = None;
                    page.cursor = None;
                }
            }
            Action::KeyboardFind(keys) => {
                if let Some(page) = self.keyboard_page() {
                    page.found = keys;
                    page.cursor = None;
                }
            }
            Action::KeyboardChanged(only) => {
                if let Some(page) = self.keyboard_page() {
                    page.changed_only = only;
                    page.cursor = None;
                }
            }
            Action::KeyboardCursor(row) => {
                if let Some(page) = self.keyboard_page() {
                    page.cursor = Some(row);
                }
            }
            Action::KeyboardFiltering(on) => {
                if let Some(page) = self.keyboard_page() {
                    page.filtering = on;
                }
            }
            Action::RecordKeys(of) => {
                let layout = self.layout();
                // A row whose keys cannot be recorded is not recorded.
                if let Recorded::Row(row) = of
                    && fixed(layout, row).is_some()
                {
                    return;
                }
                if let Some(page) = self.keyboard_page() {
                    page.recording = Some(Recording::of(of));
                    page.filtering = false;
                    page.resetting = false;
                }
            }
            Action::StopRecording => {
                if let Some(page) = self.keyboard_page() {
                    page.recording = None;
                }
            }
            Action::RecordTyped { char, at } => {
                if let Some(recording) = self
                    .keyboard_page()
                    .and_then(|page| page.recording.as_mut())
                {
                    recording.typed.push(char);
                    recording.at = at;
                    recording.refused = None;
                }
            }
            Action::RecordBack => {
                if let Some(recording) = self
                    .keyboard_page()
                    .and_then(|page| page.recording.as_mut())
                {
                    recording.typed.pop();
                }
            }
            Action::RecordNever(never) => {
                if let Some(recording) = self
                    .keyboard_page()
                    .and_then(|page| page.recording.as_mut())
                {
                    recording.typed.clear();
                    recording.refused = Some(("", Why::Never(never)));
                }
            }
            Action::RecordPress(keys) => self.record_press(keys),
            Action::UnbindRow(row) => {
                let layout = self.layout();
                if fixed(layout, row).is_none() {
                    let next = self.settings.keys.unbind(layout, row);
                    self.change_settings(|settings| settings.keys = next);
                }
                if let Some(page) = self.keyboard_page() {
                    page.recording = None;
                }
            }
            Action::ResetRow(row) => {
                let layout = self.layout();
                // The defaults another row's line has taken since stay
                // that row's.
                let next = self.settings.keys.reset_free(layout, row);
                self.change_settings(|settings| settings.keys = next);
            }
            Action::SwapKeys => self.swap_keys(),
            Action::AskResetAllKeys(ask) => {
                if let Some(page) = self.keyboard_page() {
                    page.resetting = ask;
                    page.recording = None;
                }
            }
            Action::ResetAllKeys => {
                let asked = self
                    .keyboard_page()
                    .is_some_and(|page| std::mem::take(&mut page.resetting));
                if asked {
                    self.reset_all_keys();
                }
            }
            Action::ExportCheatSheet => {
                let text = cheat_sheet(&self.keymap, self.layout());
                self.backend.save_bytes(
                    "Export cheat sheet",
                    CHEAT_SHEET.to_owned(),
                    text.into_bytes(),
                );
            }
            Action::KeysDefault => {
                let text = keys_list(&self.keymap, self.layout());
                let path = self.dirs.state.join(KEYS_LIST);
                self.backend.send(Command::OpenKeysList { path, text });
            }
            _ => {}
        }
    }

    /// The keys pressed while recording, ended: given to the row where the
    /// keymap takes them, or refused with why, and the recording goes on.
    fn record_press(&mut self, keys: &'static str) {
        let layout = self.layout();
        let Some(recording) = self.keyboard_page().and_then(|page| page.recording.clone()) else {
            return;
        };
        let row = match recording.of {
            Recorded::Find => {
                if let Some(page) = self.keyboard_page() {
                    page.found = Some(keys);
                    page.cursor = None;
                    page.recording = None;
                }
                return;
            }
            Recorded::Row(row) => row,
        };
        let refused = |app: &mut Self, why: Why| {
            // A key another row has is offered as a swap where the
            // exchange would stand.
            let swaps = match why {
                Why::Taken(other) => app.settings.keys.swaps(layout, row, keys, other),
                _ => false,
            };
            if let Some(recording) = app.keyboard_page().and_then(|page| page.recording.as_mut()) {
                recording.typed.clear();
                recording.refused = Some((keys, why));
                recording.swaps = swaps;
            }
        };
        let Ok(spelled) = spell(keys) else {
            return refused(self, Why::Kind(crate::keymap::takes(layout, row)));
        };
        if let Some(why) = crate::keymap::refuse_key(layout, row, &spelled) {
            return refused(self, why);
        }
        let next = self.settings.keys.record(layout, row, keys);
        if let Some(why) = own_refusal(layout, &next, row, keys) {
            return refused(self, why);
        }
        self.change_settings(|settings| settings.keys = next);
        if let Some(page) = self.keyboard_page() {
            page.recording = None;
        }
    }

    /// The swap a refused key offers: the key to its row, and that row's
    /// old keys of its kind to the row that had it.
    fn swap_keys(&mut self) {
        let layout = self.layout();
        let Some(recording) = self.keyboard_page().and_then(|page| page.recording.clone()) else {
            return;
        };
        let (Recorded::Row(row), Some((keys, Why::Taken(other))), true) =
            (recording.of, recording.refused, recording.swaps)
        else {
            return;
        };
        let next = self.settings.keys.swap(layout, row, keys, other);
        if let Some(why) = own_refusal(layout, &next, row, keys) {
            if let Some(recording) = self
                .keyboard_page()
                .and_then(|page| page.recording.as_mut())
            {
                recording.refused = Some((keys, why));
                recording.swaps = false;
            }
            return;
        }
        self.change_settings(|settings| settings.keys = next);
        if let Some(page) = self.keyboard_page() {
            page.recording = None;
        }
    }
}

/// Why the keymap would not take `keys` for `row` from `overrides`, the
/// line that gives them: what another line of the file may say does not
/// stop this one.
fn own_refusal(
    layout: Layout,
    overrides: &crate::keymap::Overrides,
    row: Row,
    keys: &'static str,
) -> Option<Why> {
    let (_, refused) = Keymap::with(layout, overrides);
    refused.into_iter().find_map(|refusal| {
        let Line::Bind(index) = refusal.line else {
            return None;
        };
        let bind = overrides.binds.get(index)?;
        let mine = bind.scope == row.scope
            && bind.command == row.command
            && match (spell(bind.keys), spell(keys)) {
                (Ok(a), Ok(b)) => crate::keymap::same(layout, &a, &b),
                _ => false,
            };
        mine.then_some(refusal.why)
    })
}

/// A row's keys as `layout` writes them in a list, or "Not set".
fn row_keys(keymap: &Keymap, layout: Layout, row: Row) -> String {
    let keys: Vec<String> = keymap
        .row_keys(layout, row)
        .iter()
        .map(|key| key.as_str().to_owned())
        .collect();
    if keys.is_empty() {
        "Not set".to_owned()
    } else {
        keys.join(", ")
    }
}

/// A cell of a Markdown table: what would end it or start code escaped.
fn cell(text: &str) -> String {
    text.replace('\\', "\\\\")
        .replace('|', "\\|")
        .replace('`', "\\`")
}

/// The cheat sheet of the keys in force: a Markdown table for each group
/// of the Keyboard pane, its rows as the pane lists them, with their scope
/// and their keys as `layout` writes them, a changed row marked so, and a
/// row whose command nothing answers yet.
pub fn cheat_sheet(keymap: &Keymap, layout: Layout) -> String {
    let mut out = String::from("# Tabletist shortcuts\n");
    let rows = keymap.rows(layout);
    for group in Group::LISTED {
        let listed: Vec<Row> = rows
            .iter()
            .copied()
            .filter(|row| row.command.info().group == group)
            .collect();
        if listed.is_empty() {
            continue;
        }
        out.push_str(&format!(
            "\n## {}\n\n| Command | Scope | Shortcut |\n| --- | --- | --- |\n",
            group.name(layout)
        ));
        for row in listed {
            let mut keys = row_keys(keymap, layout, row);
            if keymap.changed(layout, row) {
                keys.push_str(" (changed)");
            }
            let mut name = row.command.info().name.to_owned();
            if !row.command.info().built {
                name.push_str(" (not built yet)");
            }
            out.push_str(&format!(
                "| {} | {} | {} |\n",
                cell(&name),
                cell(row.scope.name(layout)),
                cell(&keys)
            ));
        }
    }
    out
}

/// The full list of the keys in force, as `settings.toml` spells them and
/// commented out: a table for each scope, a line for each key of a row, a
/// row whose keys cannot be recorded marked so, and one whose command
/// nothing answers yet. For copying lines from:
/// nothing reads it.
pub fn keys_list(keymap: &Keymap, layout: Layout) -> String {
    let mut out = String::from(
        "# Every key of Tabletist in force, as settings.toml spells them.\n\
         # Nothing reads this file: it is written again each time :keys default asks.\n\
         # To change a key, copy its line into settings.toml, under its table,\n\
         # without its #, and change the key.\n",
    );
    let rows = keymap.rows(layout);
    for scope in crate::keymap::Scope::ALL {
        let listed: Vec<Row> = rows
            .iter()
            .copied()
            .filter(|row| row.scope == scope && row.when == crate::keymap::When::Always)
            .collect();
        let lines: Vec<String> = listed
            .iter()
            .flat_map(|row| {
                let note = match fixed(layout, *row) {
                    Some(Fixed::System) => "  # the system's",
                    Some(_) => "  # fixed",
                    None if !row.command.info().built => "  # not built yet",
                    None => "",
                };
                keymap
                    .keys_of(layout, *row)
                    .into_iter()
                    .map(move |keys| format!("# \"{keys}\" = \"{}\"{note}\n", row.command.id()))
            })
            .collect();
        if lines.is_empty() {
            continue;
        }
        out.push_str(&format!("\n# [keys.{}]\n", scope.table(layout)));
        for line in lines {
            out.push_str(&line);
        }
    }
    out
}

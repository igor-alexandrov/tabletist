//! The partial Test of a Files connection: what the banner over the
//! footer says of the first file the Test could not read, and the fixes it
//! offers. A value its column's type cannot take in a CSV or TSV has three
//! (read the column as text, read the value as empty, skip the file); any
//! other failure has the last alone. With several, the next shows once the
//! first is mended and the Test has run again.

use tabletist_db::files::{Failure, Why};

use super::Skin;
use crate::i18n::gettext;
use crate::keymap::Command;
use crate::model::{ConnectionForm, FileFix};
use crate::ui::banner::{Action, Banner};
use crate::ui::states::Tone;

/// The banner's words and fixes, made for one frame.
pub(super) struct Partial {
    lead: String,
    text: String,
    fixes: Vec<Fix>,
}

struct Fix {
    label: String,
    fix: FileFix,
    key: Option<crate::keymap::Written>,
    quiet: bool,
}

impl Partial {
    pub(super) fn banner(&self) -> Banner<'_> {
        Banner {
            tone: Tone::Warning,
            lead: &self.lead,
            text: &self.text,
            actions: self
                .fixes
                .iter()
                .map(|fix| Action {
                    label: &fix.label,
                    key: fix.key.clone(),
                    quiet: fix.quiet,
                })
                .collect(),
            salt: "partial",
        }
    }

    /// The fix of the action at `at`, as the banner counts them.
    pub(super) fn fix(&self, at: usize) -> Option<FileFix> {
        self.fixes.get(at).map(|fix| fix.fix)
    }
}

/// `template` with each `{name}` in it put as `pairs` say: the template,
/// translated where its literal is (so the catalogs have it), is put in
/// the look's case, what goes in it as it is (a file's
/// or a column's name, a value). One pass: what goes in is not read for
/// names again, whatever a file's value says.
fn put(skin: &Skin, template: &str, pairs: &[(&str, &str)]) -> String {
    let template = skin.look.label(template);
    let mut text = String::with_capacity(template.len());
    let mut rest = template.as_str();
    while let Some(open) = rest.find('{') {
        text.push_str(&rest[..open]);
        let after = &rest[open + 1..];
        let named = after.find('}').and_then(|close| {
            let name = &after[..close];
            pairs
                .iter()
                .find(|(each, _)| *each == name)
                .map(|(_, value)| (close, *value))
        });
        match named {
            Some((close, value)) => {
                text.push_str(value);
                rest = &after[close + 1..];
            }
            None => {
                text.push('{');
                rest = after;
            }
        }
    }
    text.push_str(rest);
    text
}

/// The most of a file's value or name a fix's button says, cut short with
/// `…` past it: the banner says it in full.
const SHORT: usize = 32;

fn short(text: &str) -> String {
    match text.chars().count() > SHORT {
        true => format!("{}…", text.chars().take(SHORT - 1).collect::<String>()),
        false => text.to_owned(),
    }
}

/// The banner for the first file `form`'s last Test could not read.
pub(super) fn of(ctx: &egui::Context, form: &ConnectionForm, skin: &Skin) -> Option<Partial> {
    let failure = form.first_failure()?;
    let terminal = skin.look.terminal;
    let file = failure.file.as_str();
    let lead = match terminal {
        true => format!("! {file}"),
        false => put(
            skin,
            &gettext(skin.locale, "{file} couldn't be read."),
            &[("file", file)],
        ),
    };
    let mut text = sentence(failure, skin);
    // The terminal look's line is a clause, with no stop.
    if terminal {
        text = text.trim_end_matches('.').to_owned();
    }
    let key =
        |command: Command| terminal.then(|| crate::ui::keys::written(ctx, skin.look, command));
    let mut fixes = Vec::new();
    if let Why::Value { column, value, .. } = &failure.why
        && failure.takes_column_fixes()
    {
        fixes.push(Fix {
            label: put(
                skin,
                &gettext(skin.locale, "Read {column} as text"),
                &[("column", &short(column))],
            ),
            fix: FileFix::Text,
            key: key(Command::FormFixText),
            quiet: false,
        });
        fixes.push(Fix {
            label: put(
                skin,
                &gettext(skin.locale, "Read '{value}' as empty"),
                &[("value", &short(value))],
            ),
            fix: FileFix::Empty,
            key: key(Command::FormFixEmpty),
            quiet: false,
        });
    }
    fixes.push(Fix {
        label: put(
            skin,
            &gettext(skin.locale, "Skip {file}"),
            &[("file", &short(file))],
        ),
        fix: FileFix::Skip,
        key: key(Command::FormFixSkip),
        quiet: true,
    });
    Some(Partial { lead, text, fixes })
}

/// What the banner says of why the file could not be read.
fn sentence(failure: &Failure, skin: &Skin) -> String {
    match &failure.why {
        Why::Value {
            row,
            column,
            value,
            kind,
        } => put(
            skin,
            &gettext(skin.locale, "Row {row} has '{value}' in {column}, {kind}."),
            &[
                ("row", &crate::i18n::number(skin.locale, *row)),
                ("value", value),
                ("column", column),
                ("kind", &kind_word(kind, skin)),
            ],
        ),
        Why::Columns {
            row,
            expected,
            found,
        } => put(
            skin,
            &gettext(
                skin.locale,
                "Row {row} has {found} values, where the file's rows have {expected}.",
            ),
            &[
                ("row", &crate::i18n::number(skin.locale, *row)),
                ("found", &crate::i18n::number(skin.locale, *found)),
                ("expected", &crate::i18n::number(skin.locale, *expected)),
            ],
        ),
        Why::Json {
            line,
            malformed: true,
        } => put(
            skin,
            &gettext(skin.locale, "Line {line} isn't JSON."),
            &[("line", &crate::i18n::number(skin.locale, *line))],
        ),
        Why::Json {
            line,
            malformed: false,
        } => put(
            skin,
            &gettext(
                skin.locale,
                "Line {line} has a value of another type than the lines before it.",
            ),
            &[("line", &crate::i18n::number(skin.locale, *line))],
        ),
        Why::Other(message) => message.clone(),
    }
}

/// DuckDB's type as the words for what its column holds: `DATE` is "a
/// date column".
fn kind_word(kind: &str, skin: &Skin) -> String {
    let kind = kind.to_ascii_uppercase();
    let words = match kind.as_str() {
        "DATE" => gettext(skin.locale, "a date column"),
        "TIME" => gettext(skin.locale, "a time column"),
        kind if kind.starts_with("TIMESTAMP") => gettext(skin.locale, "a timestamp column"),
        "BIGINT" | "INTEGER" | "SMALLINT" | "TINYINT" | "HUGEINT" | "UBIGINT" | "UINTEGER"
        | "USMALLINT" | "UTINYINT" | "UHUGEINT" => gettext(skin.locale, "a whole number column"),
        "DOUBLE" | "FLOAT" => gettext(skin.locale, "a number column"),
        kind if kind.starts_with("DECIMAL") => gettext(skin.locale, "a number column"),
        "BOOLEAN" => gettext(skin.locale, "a true or false column"),
        "UUID" => gettext(skin.locale, "a UUID column"),
        "INTERVAL" => gettext(skin.locale, "an interval column"),
        // DuckDB's own name in lower case: "a column of type blob".
        _ => {
            return put(
                skin,
                &gettext(skin.locale, "a column of type {kind}"),
                &[("kind", &kind.to_ascii_lowercase())],
            );
        }
    };
    skin.look.label(&words)
}

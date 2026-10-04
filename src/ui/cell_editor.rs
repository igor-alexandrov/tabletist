//! Editing a cell: the field that sits on the cell, the keys that end an
//! edit, and the words for what a typed value fails and for why a cell
//! cannot be edited.

use egui::text::{CCursor, CCursorRange};
use egui::{CornerRadius, Id, Key, Margin, Modifiers, Rect, Stroke, StrokeKind, Ui, pos2, vec2};

use crate::edit::{Editor, Lock, Problem};
use crate::i18n::{Locale, gettext, ngettext};
use crate::model::{Advance, ConnTabId, TabId};
use crate::theme::{Look, Palette};
use crate::typography::Text;
use crate::ui::focus::{self, Ring};
use crate::ui::format::display_safe;
use crate::ui::keys::consume_press;
use crate::ui::states::Tone;
use crate::ui::{grid, widgets};

/// What a frame of an open editor asks for.
#[derive(Debug, Default, PartialEq)]
pub struct Outcome {
    /// The text changed.
    pub changed: bool,
    /// Enter, Tab or Shift+Tab ended the edit.
    pub commit: Option<Advance>,
    /// Esc.
    pub cancel: bool,
    /// The keyboard went elsewhere.
    pub left: bool,
    /// Alt+Enter.
    pub large: bool,
}

/// The cell an editor is open on, as its field needs it.
pub struct Target {
    /// The field's id: one for a table's tab, whatever cell it is on.
    pub id: Id,
    /// The column's name.
    pub name: String,
    /// The column's type as the grid's header shows it.
    pub type_name: String,
    /// The most characters the column holds, where its type says.
    pub max_chars: Option<u32>,
    /// No dialog is up: an editor that is open has the keyboard.
    pub hold: bool,
}

/// The id of the field that edits a cell of the table `id` shows.
pub fn field_id(tab: ConnTabId, id: TabId) -> Id {
    Id::new(("cell-editor", tab.0, id.0))
}

/// Whether the field `id` had the keyboard when it was last drawn. egui
/// says a widget lost the keyboard only when it had it a frame ago, and a
/// field is not drawn every frame (its tab may be behind another).
fn had_id(id: Id) -> Id {
    id.with("had-keyboard")
}

/// Gives the field `id` the keyboard, its cursor at the end of `text` and
/// nothing to undo: it is one field for every cell, and each edit is its
/// own.
fn take_keyboard(ui: &Ui, id: Id, text: &str) {
    let mut state = egui::text_edit::TextEditState::default();
    let end = CCursor::new(text.chars().count());
    state.cursor.set_char_range(Some(CCursorRange::one(end)));
    state.store(ui.ctx(), id);
    ui.memory_mut(|memory| memory.request_focus(id));
    // The keys a field holds (Tab, Esc) are its own from the frame after
    // it has the keyboard: ask for that frame, since no event need follow
    // the one that opened the editor.
    ui.ctx().request_repaint();
}

/// The keys that end an edit, read before the field is added: it would
/// take Enter as giving the keyboard up, and Alt+Enter with it. Tab and
/// Esc are egui's to move and drop the keyboard with, unless the field
/// holds them (see `hold_keys`).
fn ending_keys(ui: &Ui, has: bool, had: bool, outcome: &mut Outcome) {
    ui.input_mut(|input| {
        if has {
            // Alt first: a match lets an extra Alt and Shift through.
            if consume_press(input, Modifiers::ALT, Key::Enter) {
                outcome.large = true;
            } else if consume_press(input, Modifiers::NONE, Key::Enter) {
                outcome.commit = Some(Advance::Down);
            } else if consume_press(input, Modifiers::SHIFT, Key::Tab) {
                outcome.commit = Some(Advance::Left);
            } else if consume_press(input, Modifiers::NONE, Key::Tab) {
                outcome.commit = Some(Advance::Right);
            }
        }
        // In the first frames of a field egui still drops the keyboard on
        // Esc before any of this runs: the key is the editor's all the same.
        if (has || had) && consume_press(input, Modifiers::NONE, Key::Escape) {
            outcome.cancel = true;
        }
    });
}

/// Keeps Tab and Esc for the field `id`, which has the keyboard. The field
/// set its own filter while it drew (the arrows, Tab); egui replaces the
/// whole filter, so those are named again. It holds from the frame after
/// the field first had the keyboard.
fn hold_keys(ui: &Ui, id: Id) {
    ui.memory_mut(|memory| {
        memory.set_focus_lock_filter(
            id,
            egui::EventFilter {
                tab: true,
                horizontal_arrows: true,
                vertical_arrows: true,
                escape: true,
            },
        );
    });
}

/// The editor on its cell: a one-line field over `rect`, the cell's. It
/// edits `editor.text` and takes `editor.focus`; everything else it says in
/// its [`Outcome`].
pub fn field(
    ui: &mut Ui,
    rect: Rect,
    editor: &mut Editor,
    target: &Target,
    (look, palette, locale): (&Look, &Palette, Locale),
) -> Outcome {
    let mut outcome = Outcome::default();
    let id = target.id;
    if std::mem::take(&mut editor.focus) {
        take_keyboard(ui, id, &editor.text);
    }
    let has = ui.memory(|memory| memory.has_focus(id));
    let had: bool = ui.data(|data| data.get_temp(had_id(id))).unwrap_or(false);
    ending_keys(ui, has, had, &mut outcome);

    // Over the cell: the row's fill would show through a field with none.
    ui.painter()
        .rect_filled(rect, CornerRadius::ZERO, palette.window);
    let role = grid::data_role(look);
    let pad = grid::cell_pad(look);
    let center = rect.center().y;
    // How much of the column's length the text takes, at the field's right.
    let mut right = rect.right() - pad;
    if let Some(max) = target.max_chars {
        let count = format!("{} / {max}", editor.text.chars().count());
        let laid = Text::one(look, widgets::secondary(look), &count, palette.dim).layout(ui.ctx());
        right -= laid.paint_right(ui.painter(), right, center) + pad;
    }
    let line = role.row_height(ui.ctx(), look.faces);
    let place = Rect::from_min_max(
        pos2(rect.left() + pad, center - line / 2.0),
        pos2(right.max(rect.left() + pad), center + line / 2.0),
    );
    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(place));
    let mut layouter = crate::typography::layouter(look, role, palette.text);
    let response = child.add(
        egui::TextEdit::singleline(&mut editor.text)
            .id(id)
            .font(role.font_id(look.faces))
            .frame(egui::Frame::NONE)
            .margin(Margin::ZERO)
            .desired_width(place.width())
            // Tab ends the edit; it is not egui's to move the keyboard with.
            .lock_focus(true)
            .layouter(&mut layouter),
    );
    // The name only: the field keeps the role and the value egui gave it.
    let name = format!("{} {}", gettext(locale, "Edit"), display_safe(&target.name));
    ui.ctx().accesskit_node_builder(id, |node| {
        node.set_label(name);
    });
    let has = response.has_focus();
    if has {
        hold_keys(ui, id);
    }
    // The ring is the cell's: red while the text fails its check.
    let ring = if editor.problem.is_some() {
        Ring::Failing { radius: 0 }
    } else {
        Ring::Field { radius: 0 }
    };
    focus::hint(ui, &response, rect, ring);
    if let (Some(problem), true) = (&editor.problem, target.hold) {
        let message = problem_text(problem, &target.type_name, Some(&editor.text), locale);
        say_under(ui, rect, id, &message, look, palette);
    }
    outcome.changed = response.changed();
    let ended = outcome.commit.is_some() || outcome.cancel || outcome.large;
    // The keyboard went elsewhere: to a click, to another field.
    outcome.left = had && !has && !ended;
    if target.hold && !has && !had {
        // An editor whose leaving was not taken (a prompt was up) is still
        // open: it has the keyboard again once nothing else is asked.
        response.request_focus();
        ui.ctx().request_repaint();
    }
    ui.data_mut(|data| data.insert_temp(had_id(id), has));
    outcome
}

/// Paints `message` under the field at `rect`, or over it where the grid
/// ends under it: what the text fails, in red. Over the rows, which are
/// drawn after the field's.
fn say_under(ui: &Ui, rect: Rect, id: Id, message: &str, look: &Look, palette: &Palette) {
    let clip = ui.clip_rect();
    let laid = Text::one(look, widgets::secondary(look), message, palette.danger).layout(ui.ctx());
    let size = laid.size() + vec2(12.0, 6.0);
    // Clear of the field's halo.
    let gap = 4.0;
    let below = rect.bottom() + gap;
    let top = if below + size.y > clip.bottom() {
        rect.top() - gap - size.y
    } else {
        below
    };
    let place = Rect::from_min_size(pos2(rect.left(), top), size);
    let layer = egui::LayerId::new(egui::Order::Foreground, id.with("problem"));
    let painter = ui.ctx().layer_painter(layer).with_clip_rect(clip);
    let corner = CornerRadius::same(look.radius.min(4));
    painter.rect_filled(place, corner, Tone::Danger.fill(look, palette));
    painter.rect_stroke(
        place,
        corner,
        Stroke::new(1.0, Tone::Danger.line(look, palette)),
        StrokeKind::Inside,
    );
    laid.paint_left(&painter, place.left() + 6.0, place.center().y);
    widgets::announce(ui, place, message);
}

/// What a typed value fails, as the user reads it. `type_name` is the
/// column's type as the grid's header shows it (the page's `ColumnMeta`:
/// `int8`, where the structure says `bigint`); `typed` is the text that
/// failed, where the caller has it.
pub fn problem_text(
    problem: &Problem,
    type_name: &str,
    typed: Option<&str>,
    locale: Locale,
) -> String {
    let say = |text: &'static str| gettext(locale, text);
    let counted = |count: u32, one: &'static str, many: &'static str| {
        format!("{count} {}", ngettext(locale, one, many, count))
    };
    // As the check read it: without the spaces a database overlooks.
    let typed = typed.map(str::trim_ascii).filter(|typed| !typed.is_empty());
    // What the database would have kept in place of the typed number.
    let stored_as = |stored: &str| match typed {
        Some(typed) => format!("{typed} {} {stored}", say("would be stored as")),
        None => format!("{} {stored}", say("Would be stored as")),
    };
    match problem {
        Problem::WholeNumber => format!("{type_name} {}", say("expects a whole number")),
        Problem::OutOfRange { min, max } => {
            format!("{type_name} {} {min} {} {max}", say("holds"), say("to"))
        }
        Problem::Number => format!("{type_name} {}", say("expects a number")),
        Problem::Decimals { scale, stored } => {
            let limit = format!(
                "{} {}.",
                say("Up to"),
                counted(*scale, "decimal", "decimals")
            );
            match typed {
                Some(_) => format!("{limit} {}.", stored_as(stored)),
                None => limit,
            }
        }
        Problem::Digits { whole } => format!(
            "{} {} {}",
            say("At most"),
            counted(*whole, "digit", "digits"),
            say("before the point")
        ),
        Problem::Inexact { stored } => stored_as(stored),
        Problem::Boolean => format!("{type_name} {}", say("expects true or false")),
        Problem::NotOneOf(allowed) => {
            // The values come from the server: nothing hidden in them.
            let values: Vec<_> = allowed.iter().map(|value| display_safe(value)).collect();
            format!("{} {}", say("Not one of:"), values.join(", "))
        }
        Problem::Json {
            message,
            line,
            column,
        } => {
            // The parser starts its sentence in lower case.
            let mut letters = message.chars();
            let message: String = match letters.next() {
                Some(first) => first.to_uppercase().chain(letters).collect(),
                None => String::new(),
            };
            format!("{message} {} {line}:{column}", say("at"))
        }
        Problem::TooLong { max } => format!(
            "{} {}",
            say("At most"),
            counted(*max, "character", "characters")
        ),
    }
}

/// Why a cell cannot be edited. `table` is the table's name as it is
/// shown. A cell that is not there has no reason to give.
pub fn lock_text(lock: Lock, table: &str, locale: Locale) -> String {
    let say = |text: &'static str| gettext(locale, text).into_owned();
    match lock {
        Lock::ReadOnly => say("This connection opens read-only"),
        Lock::NotATable => say("Views cannot be edited"),
        Lock::StructureLoading => say("The table's structure is still loading"),
        Lock::NoKey => format!(
            "{table} {}",
            say("has no primary key or unique index, so a row can't be targeted safely")
        ),
        Lock::KeyType => format!(
            "{table}{}",
            say("'s key cannot be matched exactly, so a row can't be targeted safely")
        ),
        Lock::Saving => say("A save is running"),
        Lock::Refreshing => say("The page is loading"),
        Lock::KeyIsNull => say("This row's key is NULL"),
        Lock::KeyInexact => say("This row's key holds text that was not read exactly"),
        Lock::UnknownColumn => say("This column cannot be told apart in the table"),
        Lock::Generated => say("Computed by the database"),
        Lock::KeyColumn => say("Part of the row's key"),
        Lock::Binary => say("Binary values cannot be edited yet"),
        Lock::TooLarge => say("Values over 256 KiB cannot be edited yet"),
        Lock::NoSuchCell => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_problem_and_every_lock_has_words() {
        let locale = Locale::English;
        let say =
            |problem: &Problem, typed: Option<&str>| problem_text(problem, "int8", typed, locale);
        assert_eq!(
            say(&Problem::WholeNumber, None),
            "int8 expects a whole number"
        );
        assert_eq!(
            say(
                &Problem::OutOfRange {
                    min: -128,
                    max: 127
                },
                None
            ),
            "int8 holds -128 to 127"
        );
        assert_eq!(say(&Problem::Number, None), "int8 expects a number");
        let decimals = Problem::Decimals {
            scale: 2,
            stored: "12.51".into(),
        };
        assert_eq!(
            say(&decimals, Some("12.505")),
            "Up to 2 decimals. 12.505 would be stored as 12.51."
        );
        // The spaces a database overlooks round a value are not the value.
        assert_eq!(
            say(&decimals, Some(" 12.505 ")),
            "Up to 2 decimals. 12.505 would be stored as 12.51."
        );
        assert_eq!(say(&decimals, None), "Up to 2 decimals.");
        let one = Problem::Decimals {
            scale: 1,
            stored: "0.3".into(),
        };
        assert_eq!(say(&one, None), "Up to 1 decimal.");
        assert_eq!(
            say(&Problem::Digits { whole: 8 }, None),
            "At most 8 digits before the point"
        );
        assert_eq!(
            say(&Problem::Digits { whole: 1 }, None),
            "At most 1 digit before the point"
        );
        let inexact = Problem::Inexact {
            stored: "0.30000000000000004".into(),
        };
        assert_eq!(
            say(&inexact, Some("0.300000000000000044")),
            "0.300000000000000044 would be stored as 0.30000000000000004"
        );
        assert_eq!(
            say(&inexact, None),
            "Would be stored as 0.30000000000000004"
        );
        assert_eq!(say(&Problem::Boolean, None), "int8 expects true or false");
        let allowed = vec!["print".to_owned(), "ebook".to_owned(), "audio".to_owned()];
        assert_eq!(
            say(&Problem::NotOneOf(allowed), None),
            "Not one of: print, ebook, audio"
        );
        let json = Problem::Json {
            message: "expected , or }".into(),
            line: 3,
            column: 23,
        };
        assert_eq!(say(&json, None), "Expected , or } at 3:23");
        assert_eq!(
            say(&Problem::TooLong { max: 200 }, None),
            "At most 200 characters"
        );
        assert_eq!(
            say(&Problem::TooLong { max: 1 }, None),
            "At most 1 character"
        );

        let why = |lock| lock_text(lock, "book_covers", locale);
        assert_eq!(why(Lock::ReadOnly), "This connection opens read-only");
        assert_eq!(why(Lock::NotATable), "Views cannot be edited");
        assert_eq!(
            why(Lock::NoKey),
            "book_covers has no primary key or unique index, so a row can't be targeted safely"
        );
        assert_eq!(
            why(Lock::KeyType),
            "book_covers's key cannot be matched exactly, so a row can't be targeted safely"
        );
        assert_eq!(why(Lock::KeyIsNull), "This row's key is NULL");
        assert_eq!(why(Lock::Generated), "Computed by the database");
        assert_eq!(why(Lock::KeyColumn), "Part of the row's key");
        // A cell that is not there has nothing to say; every other has.
        assert_eq!(why(Lock::NoSuchCell), "");
        for lock in [
            Lock::ReadOnly,
            Lock::NotATable,
            Lock::StructureLoading,
            Lock::NoKey,
            Lock::KeyType,
            Lock::Saving,
            Lock::Refreshing,
            Lock::KeyIsNull,
            Lock::KeyInexact,
            Lock::UnknownColumn,
            Lock::Generated,
            Lock::KeyColumn,
            Lock::Binary,
            Lock::TooLarge,
        ] {
            assert!(!why(lock).is_empty(), "{lock:?}");
        }
    }
}

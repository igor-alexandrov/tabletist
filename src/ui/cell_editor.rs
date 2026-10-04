//! Editing a cell: the words for what a typed value fails and for why a
//! cell cannot be edited.

use crate::edit::{Lock, Problem};
use crate::i18n::{Locale, gettext, ngettext};
use crate::ui::format::display_safe;

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

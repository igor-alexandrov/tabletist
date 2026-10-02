//! The rows of a SQL editor's completion list: what a site offers, from
//! what the workspace knows. No UI.

use std::ops::Range;

use tabletist_db::Dialect;
use tabletist_db::complete::{Expects, PHRASES, Site};

/// How many candidates a list keeps; the rest are only counted.
pub const KEPT: usize = 100;

#[cfg(test)]
thread_local! {
    /// How many times this thread worked out a list.
    pub static LISTED: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

/// What a candidate is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Keyword,
    Schema,
    Table,
    View,
    MaterializedView,
    Column,
}

/// One row of a completion list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Candidate {
    pub kind: Kind,
    /// What the row shows.
    pub label: String,
    /// What accepting inserts in place of the word.
    pub insert: String,
    /// The part of `label` that matched what was typed: a byte range on
    /// character boundaries of `label`.
    pub matched: Range<usize>,
    /// A column's type. Empty for every other kind.
    pub detail: String,
}

impl Candidate {
    /// Whether accepting this row would only repeat what is typed: a
    /// keyword whatever its case (`From` is `FROM`), any other name as
    /// spelled (`Users` and `users` can be two tables, and quotes change
    /// the text).
    pub fn is_typed(&self, typed: &str) -> bool {
        match self.kind {
            Kind::Keyword => self.insert.eq_ignore_ascii_case(typed),
            _ => self.insert == typed,
        }
    }
}

/// What the workspace knows, as a list reads it.
pub struct Catalog {
    pub dialect: Dialect,
}

/// A list's rows, best first, and how many more matched.
pub struct Listed {
    pub candidates: Vec<Candidate>,
    pub more: usize,
}

/// Where a kind of candidate comes in a site's list: lower first.
type Group = u8;

/// Keywords come after every name.
const KEYWORDS: Group = 9;

/// What `site` offers for the `typed` part of its word, from `catalog`.
/// `manual` says the list was asked for by hand.
pub fn list(site: &Site, typed: &str, manual: bool, catalog: &Catalog) -> Listed {
    #[cfg(test)]
    LISTED.with(|count| count.set(count.get() + 1));
    let mut found: Vec<(Group, Candidate)> = Vec::new();
    if offers_keywords(site, manual) {
        let words = tabletist_db::sql::keywords(catalog.dialect).chain(PHRASES);
        found.extend(
            words
                .filter_map(|word| keyword(word, typed))
                .map(|candidate| (KEYWORDS, candidate)),
        );
    }
    rank(found, typed)
}

/// Whether keywords belong at `site`: not after a dot, not where a table
/// goes, and where a new name goes only when asked for by hand.
fn offers_keywords(site: &Site, manual: bool) -> bool {
    if !site.qualifier.is_empty() {
        return false;
    }
    match site.expects {
        Expects::Start | Expects::Columns => true,
        Expects::Name => manual,
        Expects::Tables => false,
    }
}

/// `word` as a candidate when it starts with `typed`, in the case typed:
/// lower case when every typed letter is lower case, upper case otherwise.
fn keyword(word: &str, typed: &str) -> Option<Candidate> {
    let matched = find_ignoring_case(word, typed).filter(|found| found.start == 0)?;
    let lower = typed.chars().any(char::is_lowercase) && !typed.chars().any(char::is_uppercase);
    let label = if lower {
        word.to_lowercase()
    } else {
        word.to_owned()
    };
    Some(Candidate {
        kind: Kind::Keyword,
        insert: label.clone(),
        label,
        matched,
        detail: String::new(),
    })
}

/// Where `typed` first occurs in `label`, whatever the case of either:
/// both are lower-cased character by character; a character whose
/// lower-case form is several characters matches only as a whole.
fn find_ignoring_case(label: &str, typed: &str) -> Option<Range<usize>> {
    let needle: Vec<char> = typed.chars().flat_map(char::to_lowercase).collect();
    if needle.is_empty() {
        return Some(0..0);
    }
    label.char_indices().find_map(|(start, _)| {
        let (mut matched, mut end) = (0, start);
        for (offset, character) in label[start..].char_indices() {
            if matched == needle.len() {
                break;
            }
            for lower in character.to_lowercase() {
                if needle.get(matched) != Some(&lower) {
                    return None;
                }
                matched += 1;
            }
            end = start + offset + character.len_utf8();
        }
        (matched == needle.len()).then_some(start..end)
    })
}

/// `found` in a list's order, cut at [`KEPT`]: a candidate that
/// [`is_typed`](Candidate::is_typed) first, then by group, then names that start with
/// what is typed before names that only hold it, then by name.
fn rank(mut found: Vec<(Group, Candidate)>, typed: &str) -> Listed {
    found.sort_by_cached_key(|(group, candidate)| {
        (
            !candidate.is_typed(typed),
            *group,
            candidate.matched.start != 0,
            candidate.label.to_lowercase(),
            candidate.label.clone(),
        )
    });
    let more = found.len().saturating_sub(KEPT);
    found.truncate(KEPT);
    Listed {
        candidates: found.into_iter().map(|(_, candidate)| candidate).collect(),
        more,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tabletist_db::sql::tokenize;

    /// The site at the `|` of `marked` (read as SQLite), and what is typed
    /// of its word.
    fn site_at(marked: &str) -> (Site, String) {
        let cursor = marked.find('|').expect("a cursor mark");
        let text = marked.replacen('|', "", 1);
        let tokens = tokenize(Dialect::Sqlite, &text);
        let site = tabletist_db::complete::site(&tokens, &text, cursor).expect("a site");
        let typed = text[site.word.start..cursor].to_owned();
        (site, typed)
    }

    fn keywords_at(marked: &str, manual: bool) -> Vec<String> {
        keywords_in(Dialect::Sqlite, marked, manual)
    }

    /// The labels listed at `marked` (its site read as SQLite) when the
    /// catalog is of `dialect`.
    fn keywords_in(dialect: Dialect, marked: &str, manual: bool) -> Vec<String> {
        let (site, typed) = site_at(marked);
        let catalog = Catalog { dialect };
        let listed = list(&site, &typed, manual, &catalog);
        listed.candidates.into_iter().map(|c| c.label).collect()
    }

    fn keyword_row(word: &str) -> Candidate {
        Candidate {
            kind: Kind::Keyword,
            label: word.into(),
            insert: word.into(),
            matched: 0..2,
            detail: String::new(),
        }
    }

    fn name_row(kind: Kind, label: &str, matched: Range<usize>) -> Candidate {
        Candidate {
            kind,
            label: label.into(),
            insert: label.into(),
            matched,
            detail: String::new(),
        }
    }

    #[test]
    fn keywords_follow_the_case_that_is_typed() {
        assert_eq!(keywords_at("sel|", false), ["select"]);
        assert_eq!(keywords_at("SEL|", false), ["SELECT"]);
        assert_eq!(keywords_at("Sel|", false), ["SELECT"]);
        // Nothing typed: upper case, in order, phrases among them.
        let all = keywords_at("|", true);
        assert_eq!(all[0], "ALL");
        assert!(all.contains(&"GROUP BY".to_owned()));
        // Keywords and phrases must fit under KEPT, or the last ones fall
        // out of the list.
        let (site, typed) = site_at("|");
        let catalog = Catalog {
            dialect: Dialect::Sqlite,
        };
        assert_eq!(list(&site, &typed, true, &catalog).more, 0);
    }

    #[test]
    fn the_dialect_of_the_catalog_picks_the_keywords() {
        assert_eq!(keywords_in(Dialect::Sqlite, "prag|", false), ["pragma"]);
        assert!(keywords_in(Dialect::Postgres, "prag|", false).is_empty());
    }

    #[test]
    fn a_keyword_typed_in_any_case_is_what_is_typed() {
        assert_eq!(
            keywords_at("SELECT * FROM users WHERE Is|", false)[..3],
            ["IS", "IS NOT NULL", "IS NULL"]
        );
        assert!(keyword_row("FROM").is_typed("From"));
        assert!(keyword_row("FROM").is_typed("from"));
        assert!(!keyword_row("FROM").is_typed("Fro"));
        // A keyword that is typed leads even names that start the same.
        let found = vec![
            (0, name_row(Kind::Column, "order_id", 0..2)),
            (KEYWORDS, keyword_row("OR")),
        ];
        let labels: Vec<String> = rank(found, "Or")
            .candidates
            .into_iter()
            .map(|c| c.label)
            .collect();
        assert_eq!(labels, ["OR", "order_id"]);
    }

    #[test]
    fn a_name_is_what_is_typed_only_as_spelled() {
        // `Users` and `users` can be two tables, and quotes change the text.
        let users = name_row(Kind::Table, "Users", 0..5);
        assert!(!users.is_typed("users"));
        assert!(users.is_typed("Users"));
        assert!(name_row(Kind::Table, "users", 0..5).is_typed("users"));
        let found = vec![
            (1, users),
            (0, name_row(Kind::Table, "users_archive", 0..5)),
        ];
        let labels: Vec<String> = rank(found, "users")
            .candidates
            .into_iter()
            .map(|c| c.label)
            .collect();
        assert_eq!(labels, ["users_archive", "Users"]);
    }

    #[test]
    fn names_are_ordered_ignoring_case() {
        let found = vec![
            (0, name_row(Kind::Table, "Zebra", 0..0)),
            (0, name_row(Kind::Table, "apple", 0..0)),
        ];
        let labels: Vec<String> = rank(found, "")
            .candidates
            .into_iter()
            .map(|c| c.label)
            .collect();
        assert_eq!(labels, ["apple", "Zebra"]);
    }

    #[test]
    fn keywords_match_from_their_start_only() {
        // Not JOIN or UNION, which only hold `in`. The exact match leads.
        assert_eq!(
            keywords_at("SELECT * FROM users WHERE in|", false),
            [
                "in",
                "inner",
                "inner join",
                "insert",
                "intersect",
                "interval",
                "into"
            ]
        );
        assert_eq!(keywords_at("gr|", false), ["group", "group by"]);
    }

    #[test]
    fn a_keyword_is_inserted_as_it_reads_and_marks_what_matched() {
        let (site, typed) = site_at("sel|");
        let catalog = Catalog {
            dialect: Dialect::Sqlite,
        };
        let listed = list(&site, &typed, false, &catalog);
        assert_eq!(
            listed.candidates,
            [Candidate {
                kind: Kind::Keyword,
                label: "select".into(),
                insert: "select".into(),
                matched: 0..3,
                detail: String::new(),
            }]
        );
        assert_eq!(listed.more, 0);
    }

    #[test]
    fn no_keywords_where_a_table_or_a_new_name_belongs() {
        assert!(keywords_at("SELECT * FROM se|", false).is_empty());
        // A new name: keywords only when the list was asked for by hand.
        assert!(keywords_at("SELECT * FROM users wh|", false).is_empty());
        assert_eq!(
            keywords_at("SELECT * FROM users wh|", true),
            ["when", "where"]
        );
        // After a dot: a name of that table or schema, never a keyword.
        assert!(keywords_at("SELECT u.se|", false).is_empty());
    }

    #[test]
    fn a_list_keeps_the_best_and_counts_the_rest() {
        let found = (0..150)
            .map(|index| {
                let label = format!("name_{index:03}");
                let candidate = Candidate {
                    kind: Kind::Table,
                    insert: label.clone(),
                    label,
                    matched: 0..2,
                    detail: String::new(),
                };
                (0, candidate)
            })
            .rev()
            .collect();
        let listed = rank(found, "na");
        assert_eq!(listed.candidates.len(), KEPT);
        assert_eq!(listed.more, 50);
        assert_eq!(listed.candidates[0].label, "name_000");
        assert_eq!(listed.candidates[99].label, "name_099");
    }

    #[test]
    fn candidates_are_ordered_exact_then_group_then_start_then_name() {
        let named = |label: &str, group: Group, matched: Range<usize>| {
            let candidate = Candidate {
                kind: Kind::Column,
                label: label.into(),
                insert: label.into(),
                matched,
                detail: String::new(),
            };
            (group, candidate)
        };
        let found = vec![
            named("order", 1, 0..2),
            named("author", 0, 4..6),
            named("origin", 0, 0..2),
            named("Oracle", 0, 0..2),
            named("or", 1, 0..2),
        ];
        let labels: Vec<String> = rank(found, "or")
            .candidates
            .into_iter()
            .map(|c| c.label)
            .collect();
        assert_eq!(labels, ["or", "Oracle", "origin", "author", "order"]);
    }

    #[test]
    fn text_is_found_whatever_its_case() {
        assert_eq!(find_ignoring_case("first_name", "NAME"), Some(6..10));
        assert_eq!(find_ignoring_case("Żółw", "żó"), Some(0..4));
        assert_eq!(find_ignoring_case("abc", "x"), None);
        assert_eq!(find_ignoring_case("abc", "abcd"), None);
        assert_eq!(find_ignoring_case("abc", ""), Some(0..0));
        // Lower-cased character by character, not case-folded: an `İ`
        // lower-cases to two characters and matches only as a whole.
        assert_eq!(find_ignoring_case("İstanbul", "i"), None);
        assert_eq!(find_ignoring_case("İstanbul", "İ"), Some(0..2));
    }
}

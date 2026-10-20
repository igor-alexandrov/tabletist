//! How a name matches what is typed in quick open and in the start
//! screen's search: the one search of a connection's objects. Case is
//! ignored, and so is white space at the ends of what is typed. A name
//! matches in the best of four tiers: the whole name, its start, the
//! starts of its words (`bc` is `book_covers`), or a run anywhere in it.
//! Nothing else matches: no letters scattered through the name.

use std::ops::Range;

/// How well a name matches, best last: the order a list ranks by.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Tier {
    /// What is typed is somewhere in the name.
    Substring,
    /// What is typed is the starts of the name's words, in order: `bc`
    /// for `book_covers`, `cov` for `book_covers` too, `bocov` for both
    /// words' first letters and more.
    WordStarts,
    /// The name starts with what is typed.
    Prefix,
    /// The name is what is typed.
    Exact,
}

/// A name's match: its tier, and the runs of the name that matched, as
/// byte ranges of the name, in order and apart.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Hit {
    pub tier: Tier,
    pub spans: Vec<Range<usize>>,
}

/// One character of a name or of what is typed, as compared: lower case
/// (the first character of its lower case where that is more than one).
fn fold(char: char) -> char {
    char.to_lowercase().next().unwrap_or(char)
}

/// Whether `char` parts a name's words.
fn parts(char: char) -> bool {
    matches!(char, '_' | '.' | '-' | ' ')
}

/// What `query` is compared as: its characters folded, without the white
/// space at its ends. Empty where nothing but white space is typed.
pub fn folded(query: &str) -> Vec<char> {
    query.trim().chars().map(fold).collect()
}

/// A name taken apart for matching, once: its characters folded, where
/// each starts in the name, and where its words start. An index keeps one
/// per object, so a keystroke compares and takes nothing apart.
#[derive(Clone, Debug)]
pub struct Name {
    /// The characters as compared.
    text: Vec<char>,
    /// Where each character starts in the name, in bytes, and then the
    /// name's length.
    places: Vec<usize>,
    /// Where each word starts, as character indexes: at the name's start,
    /// after `_`, `.`, `-` or a space, and where a lower case letter is
    /// followed by an upper case one (`bookCovers`).
    starts: Vec<usize>,
}

impl Name {
    pub fn new(name: &str) -> Self {
        let mut text = Vec::with_capacity(name.len());
        let mut places = Vec::with_capacity(name.len() + 1);
        let mut starts = Vec::new();
        let mut before: Option<char> = None;
        for (index, (at, char)) in name.char_indices().enumerate() {
            let start = match before {
                None => !parts(char),
                Some(before) => {
                    !parts(char)
                        && (parts(before) || (before.is_lowercase() && char.is_uppercase()))
                }
            };
            if start {
                starts.push(index);
            }
            before = Some(char);
            text.push(fold(char));
            places.push(at);
        }
        places.push(name.len());
        Self {
            text,
            places,
            starts,
        }
    }

    /// How many characters the name has.
    pub fn len(&self) -> usize {
        self.text.len()
    }

    /// Whether the name has no characters.
    pub fn is_empty(&self) -> bool {
        self.text.is_empty()
    }

    /// The bytes of `len` characters from the character `from`.
    fn span(&self, from: usize, len: usize) -> Range<usize> {
        self.places[from]..self.places[from + len]
    }

    /// Whether every character of `query` is in the name, in order: no
    /// tier matches a name where they are not, which most names of a
    /// schema are not, so this is asked first.
    fn in_order(&self, query: &[char]) -> bool {
        let mut typed = query.iter().peekable();
        for char in &self.text {
            match typed.peek() {
                Some(next) if char == *next => {
                    typed.next();
                }
                Some(_) => {}
                None => break,
            }
        }
        typed.peek().is_none()
    }

    /// How the name matches `query` (as [`folded`] gives it), or `None`
    /// where it does not. An empty query matches nothing.
    pub fn find(&self, query: &[char]) -> Option<Hit> {
        if query.is_empty() || self.text.len() < query.len() || !self.in_order(query) {
            return None;
        }
        if self.text.starts_with(query) {
            let tier = if self.text.len() == query.len() {
                Tier::Exact
            } else {
                Tier::Prefix
            };
            let spans = vec![self.span(0, query.len())];
            return Some(Hit { tier, spans });
        }
        if let Some(pieces) = pieces(&self.text, &self.starts, query) {
            let spans = pieces
                .into_iter()
                .map(|(from, len)| self.span(from, len))
                .collect();
            return Some(Hit {
                tier: Tier::WordStarts,
                spans,
            });
        }
        let at = self
            .text
            .windows(query.len())
            .position(|run| run == query)?;
        Some(Hit {
            tier: Tier::Substring,
            spans: vec![self.span(at, query.len())],
        })
    }
}

/// How `name` matches `query` (as [`folded`] gives it), or `None` where it
/// does not: [`Name::find`] for a name matched once.
pub fn find(query: &[char], name: &str) -> Option<Hit> {
    Name::new(name).find(query)
}

/// What is typed, as runs that each start a word, in order: each run's
/// first character and length. The longest run is tried first at each
/// word, so `bocov` takes `bo` then `cov`. `None` where no such runs
/// exist. A run may go on past its word's end (`book_c` in
/// `old_book_covers`).
fn pieces(text: &[char], starts: &[usize], query: &[char]) -> Option<Vec<(usize, usize)>> {
    // One character: the first word it starts, with nothing to try again.
    if let [char] = query {
        let start = starts.iter().find(|&&start| text[start] == *char)?;
        return Some(vec![(*start, 1)]);
    }
    // Where the rest of the query from `typed` was tried from the word
    // `word` on and found no runs: each pair is tried once, so a name of
    // many like words costs no more than its words times the query.
    let mut failed = vec![false; (query.len() + 1) * (starts.len() + 1)];
    let mut found = Vec::new();
    walk(text, starts, query, (0, 0), &mut failed, &mut found).then_some(found)
}

fn walk(
    text: &[char],
    starts: &[usize],
    query: &[char],
    (typed, word): (usize, usize),
    failed: &mut [bool],
    found: &mut Vec<(usize, usize)>,
) -> bool {
    if typed == query.len() {
        return true;
    }
    let key = typed * (starts.len() + 1) + word;
    if failed[key] {
        return false;
    }
    for (offset, &start) in starts[word..].iter().enumerate() {
        let longest = text[start..]
            .iter()
            .zip(&query[typed..])
            .take_while(|(a, b)| a == b)
            .count();
        for len in (1..=longest).rev() {
            // The next run starts at a word after this run's end.
            let end = start + len;
            let next =
                word + offset + 1 + starts[word + offset + 1..].partition_point(|&s| s < end);
            found.push((start, len));
            if walk(text, starts, query, (typed + len, next), failed, found) {
                return true;
            }
            found.pop();
        }
    }
    failed[key] = true;
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hit(query: &str, name: &str) -> Option<Hit> {
        find(&folded(query), name)
    }

    fn tier(query: &str, name: &str) -> Option<Tier> {
        hit(query, name).map(|hit| hit.tier)
    }

    /// The runs a match marks, as the name's text.
    fn marked(query: &str, name: &str) -> Vec<String> {
        hit(query, name)
            .expect("a match")
            .spans
            .into_iter()
            .map(|span| name[span].to_owned())
            .collect()
    }

    #[test]
    fn the_tiers_go_exact_prefix_word_starts_substring() {
        assert_eq!(tier("publishers", "publishers"), Some(Tier::Exact));
        assert_eq!(tier("pub", "publishers"), Some(Tier::Prefix));
        assert_eq!(tier("bc", "book_covers"), Some(Tier::WordStarts));
        assert_eq!(tier("cov", "book_covers"), Some(Tier::WordStarts));
        assert_eq!(tier("over", "book_covers"), Some(Tier::Substring));
        assert_eq!(tier("zzz", "book_covers"), None);
        assert!(Tier::Exact > Tier::Prefix);
        assert!(Tier::Prefix > Tier::WordStarts);
        assert!(Tier::WordStarts > Tier::Substring);
    }

    #[test]
    fn case_and_the_spaces_round_what_is_typed_are_ignored() {
        assert_eq!(tier("PUB", "publishers"), Some(Tier::Prefix));
        assert_eq!(tier("pub", "Publishers"), Some(Tier::Prefix));
        assert_eq!(tier(" bookcovers ", "BookCovers"), Some(Tier::Exact));
        assert_eq!(hit("   ", "publishers"), None);
        // A space inside what is typed is a character of the name's.
        assert_eq!(tier("order items", "Order Items"), Some(Tier::Exact));
        assert_eq!(tier("r i", "Order Items"), Some(Tier::Substring));
    }

    #[test]
    fn words_part_at_underscores_dots_and_case_changes() {
        assert_eq!(tier("bc", "bookCovers"), Some(Tier::WordStarts));
        assert_eq!(tier("pb", "public.books"), Some(Tier::WordStarts));
        assert_eq!(marked("bc", "BookCovers"), ["B", "C"]);
        // Upper case all through is one word.
        assert_eq!(tier("bc", "BOOKCOVERS"), None);
        assert_eq!(tier("kc", "BOOKCOVERS"), Some(Tier::Substring));
    }

    #[test]
    fn word_starts_take_the_longest_runs_and_may_skip_words() {
        assert_eq!(marked("bocov", "book_covers"), ["bo", "cov"]);
        assert_eq!(marked("bc", "book_extra_covers"), ["b", "c"]);
        // A run may go on into the next word.
        assert_eq!(tier("book_c", "old_book_covers"), Some(Tier::WordStarts));
        assert_eq!(marked("book_c", "old_book_covers"), ["book_c"]);
        // A run starts a word: `k` starts none.
        assert_eq!(tier("kc", "book_covers"), None);
    }

    #[test]
    fn the_runs_marked_are_what_matched() {
        assert_eq!(marked("pub", "publishers"), ["pub"]);
        assert_eq!(marked("bc", "book_covers"), ["b", "c"]);
        assert_eq!(marked("over", "book_covers"), ["over"]);
        assert_eq!(marked("cov", "book_covers"), ["cov"]);
    }

    #[test]
    fn names_that_are_not_ascii_are_marked_on_their_characters() {
        assert_eq!(marked("ÉD", "éditions"), ["éd"]);
        assert_eq!(marked("ü", "bücher"), ["ü"]);
        assert_eq!(tier("ß", "straße"), Some(Tier::Substring));
    }

    #[test]
    fn many_like_words_do_not_take_long() {
        let name = vec!["a"; 40].join("_");
        let started = std::time::Instant::now();
        assert_eq!(hit(&format!("{}b", "a".repeat(30)), &name), None);
        assert!(started.elapsed() < std::time::Duration::from_millis(50));
    }
}

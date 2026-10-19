//! What was opened lately on each saved connection: the start screen's
//! recents, kept in `recents.json` by connection id.
//!
//! A connection keeps up to [`KEPT`] tables and views, newest first, and
//! the start screen shows [`SHOWN`] of them: those of the database its
//! window is on. An entry this version cannot read (a kind a newer version
//! added, a value that is no entry) is kept as the file had it and written
//! back, so a newer version's recents survive an older one.

use std::collections::BTreeMap;
use std::path::Path;

use jiff::Timestamp;
use jiff::tz::TimeZone;
use serde::{Deserialize, Serialize};
use tabletist_db::{ObjectKind, ObjectRef};

/// How many a connection keeps.
pub const KEPT: usize = 8;

/// How many the start screen shows.
pub const SHOWN: usize = 5;

/// One table or view opened on a connection.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Recent {
    /// `table`, `view` or `materialized view`.
    pub kind: String,
    /// The database it is in, as [`crate::session::database_of`] names
    /// it: a window on another database of the connection does not show
    /// it, and does not drop it.
    pub database: String,
    pub schema: String,
    pub name: String,
    /// When it was last opened, in seconds since the Unix epoch.
    pub opened: u64,
    /// What a newer version wrote beside these, written back as it was.
    #[serde(flatten)]
    pub rest: BTreeMap<String, serde_json::Value>,
}

impl Recent {
    pub fn of(object: &ObjectRef, kind: ObjectKind, database: &str, opened: u64) -> Self {
        Self {
            kind: kind_name(kind).to_owned(),
            database: database.to_owned(),
            schema: object.schema.clone(),
            name: object.name.clone(),
            opened,
            rest: BTreeMap::new(),
        }
    }

    pub fn object(&self) -> ObjectRef {
        ObjectRef::new(&self.schema, &self.name)
    }

    /// The kind as the sidebar knows it; none for a kind this version
    /// does not know, which is kept and not shown.
    pub fn object_kind(&self) -> Option<ObjectKind> {
        match self.kind.as_str() {
            "table" => Some(ObjectKind::Table),
            "view" => Some(ObjectKind::View),
            "materialized view" => Some(ObjectKind::MaterializedView),
            _ => None,
        }
    }

    /// Whether this is `object` of `database`.
    fn is(&self, database: &str, object: &ObjectRef) -> bool {
        self.database == database && self.schema == object.schema && self.name == object.name
    }
}

/// A kind as the file writes it.
pub fn kind_name(kind: ObjectKind) -> &'static str {
    match kind {
        ObjectKind::Table => "table",
        ObjectKind::View => "view",
        ObjectKind::MaterializedView => "materialized view",
    }
}

/// One entry of a connection's list: read, or kept as the file had it.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Entry {
    Read(Recent),
    Unread(serde_json::Value),
}

impl Entry {
    fn read(&self) -> Option<&Recent> {
        match self {
            Self::Read(recent) => Some(recent),
            Self::Unread(_) => None,
        }
    }

    fn value(&self) -> serde_json::Value {
        match self {
            Self::Read(recent) => serde_json::to_value(recent).unwrap_or_default(),
            Self::Unread(value) => value.clone(),
        }
    }
}

/// The file: each saved connection's recents by its id, newest first.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Recents {
    lists: BTreeMap<String, Vec<Entry>>,
    /// A connection's value that is no list: a newer version's shape,
    /// kept as it was and written back, until something opens on that
    /// connection here.
    unread: BTreeMap<String, serde_json::Value>,
}

/// The file's shape on disk: each connection's value, a list here.
type File = BTreeMap<String, serde_json::Value>;

impl Recents {
    /// Reads the file. A missing file is empty; one that is not JSON is
    /// kept aside (see [`crate::util::load_json`]). An entry that is not
    /// one of this version's, and a connection's value that is no list,
    /// are kept to be written back.
    pub fn load(path: &Path) -> Self {
        let file: File = crate::util::load_json(path);
        let mut recents = Self::default();
        for (id, value) in file {
            let serde_json::Value::Array(entries) = value else {
                recents.unread.insert(id, value);
                continue;
            };
            let entries = entries
                .into_iter()
                .map(
                    |value| match serde_json::from_value::<Recent>(value.clone()) {
                        Ok(recent) => Entry::Read(recent),
                        Err(_) => Entry::Unread(value),
                    },
                )
                .collect();
            recents.lists.insert(id, entries);
        }
        recents
    }

    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        let mut file: File = self
            .lists
            .iter()
            .map(|(id, entries)| {
                let entries = entries.iter().map(Entry::value).collect();
                (id.clone(), serde_json::Value::Array(entries))
            })
            .collect();
        file.extend(
            self.unread
                .iter()
                .map(|(id, value)| (id.clone(), value.clone())),
        );
        crate::util::save_json(path, &file)
    }

    /// Puts `recent` first in `conn`'s list, in place of the same object
    /// opened before, and keeps the list to [`KEPT`]. What this version
    /// cannot read counts and is never the one dropped: the oldest read
    /// entry after the new one is. A value of the connection that was no
    /// list gives way to the list.
    pub fn note(&mut self, conn: &str, recent: Recent) {
        self.unread.remove(conn);
        let entries = self.lists.entry(conn.to_owned()).or_default();
        entries.retain(|entry| {
            entry
                .read()
                .is_none_or(|read| !read.is(&recent.database, &recent.object()))
        });
        entries.insert(0, Entry::Read(recent));
        while entries.len() > KEPT {
            let Some(oldest) = entries[1..]
                .iter()
                .rposition(|entry| entry.read().is_some())
            else {
                break;
            };
            entries.remove(oldest + 1);
        }
    }

    /// `conn`'s recents of `database`, newest first.
    pub fn of(&self, conn: &str, database: &str) -> Vec<&Recent> {
        let mut read: Vec<&Recent> = self
            .lists
            .get(conn)
            .into_iter()
            .flatten()
            .filter_map(Entry::read)
            .filter(|recent| recent.database == database)
            .collect();
        read.sort_by_key(|recent| std::cmp::Reverse(recent.opened));
        read
    }

    /// Takes `conn`'s whole list out, and a value of it that was no list:
    /// Clear, or a connection deleted. Whether there was anything.
    pub fn clear(&mut self, conn: &str) -> bool {
        let list = self.lists.remove(conn).is_some();
        self.unread.remove(conn).is_some() || list
    }

    /// Drops the entries of `database`'s `schema` whose name `present`
    /// denies: the schema was listed, and they are not in it. An entry of
    /// a kind this version does not know is not judged by a list of
    /// tables and views. Whether any went.
    pub fn drop_missing(
        &mut self,
        conn: &str,
        database: &str,
        schema: &str,
        present: impl Fn(&str) -> bool,
    ) -> bool {
        let Some(entries) = self.lists.get_mut(conn) else {
            return false;
        };
        let before = entries.len();
        entries.retain(|entry| {
            entry.read().is_none_or(|recent| {
                recent.object_kind().is_none()
                    || recent.database != database
                    || recent.schema != schema
                    || present(&recent.name)
            })
        });
        if entries.is_empty() {
            self.lists.remove(conn);
        }
        entries_len(&self.lists, conn) != before
    }
}

/// How long `conn`'s list is, none without one.
fn entries_len(lists: &BTreeMap<String, Vec<Entry>>, conn: &str) -> usize {
    lists.get(conn).map_or(0, Vec::len)
}

/// The first comment line of a script (`-- ebook covers per publisher`),
/// without its dashes: what the start screen says of a closed SQL editor
/// beside its number. None where the script has no line comment.
pub fn first_comment(script: &str) -> Option<String> {
    script
        .lines()
        .map(str::trim)
        .filter_map(|line| line.strip_prefix("--"))
        .map(str::trim)
        .find(|comment| !comment.is_empty())
        .map(str::to_owned)
}

/// When something was opened, as the start screen says it: `Just now`,
/// `5 min ago`, `2 h ago`, `Yesterday`, the weekday (`Mon`) within the
/// week, else the date (`Oct 14`, and with its year in another year).
/// `then` and `now` are seconds since the Unix epoch; the days are
/// `zone`'s. Nothing for a time no calendar has.
pub fn when(then: u64, now: u64, zone: &TimeZone) -> String {
    let ago = now.saturating_sub(then);
    if ago < 60 {
        return "Just now".into();
    }
    if ago < 3_600 {
        return format!("{} min ago", ago / 60);
    }
    let date = |secs: u64| {
        let at = Timestamp::from_second(i64::try_from(secs).ok()?).ok()?;
        Some(at.to_zoned(zone.clone()).date())
    };
    let (Some(then_date), Some(now_date)) = (date(then), date(now)) else {
        return String::new();
    };
    let days = now_date.since(then_date).map_or(0, |span| span.get_days());
    match days {
        i32::MIN..=0 => format!("{} h ago", ago / 3_600),
        1 => "Yesterday".into(),
        2..=6 => then_date.strftime("%a").to_string(),
        _ if then_date.year() == now_date.year() => then_date.strftime("%b %-d").to_string(),
        _ => then_date.strftime("%b %-d, %Y").to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn table(name: &str, opened: u64) -> Recent {
        Recent::of(
            &ObjectRef::new("public", name),
            ObjectKind::Table,
            "bookshop_development",
            opened,
        )
    }

    fn names(recents: &Recents, conn: &str) -> Vec<String> {
        recents
            .of(conn, "bookshop_development")
            .iter()
            .map(|recent| recent.name.clone())
            .collect()
    }

    #[test]
    fn an_object_opened_again_moves_to_the_top_and_eight_are_kept() {
        let mut recents = Recents::default();
        for (index, name) in [
            "authors",
            "book_covers",
            "covers",
            "editions",
            "genres",
            "publishers",
            "ratings",
            "reviews",
            "series",
        ]
        .into_iter()
        .enumerate()
        {
            recents.note("dev", table(name, 100 + index as u64));
        }
        assert_eq!(
            names(&recents, "dev"),
            [
                "series",
                "reviews",
                "ratings",
                "publishers",
                "genres",
                "editions",
                "covers",
                "book_covers"
            ]
        );
        recents.note("dev", table("covers", 200));
        let listed = names(&recents, "dev");
        assert_eq!(listed[0], "covers");
        assert_eq!(listed.len(), KEPT);
    }

    #[test]
    fn each_connection_and_each_database_has_its_own_list() {
        let mut recents = Recents::default();
        recents.note("dev", table("book_covers", 1));
        let mut test = table("publishers", 2);
        test.database = "bookshop_test".into();
        recents.note("dev", test);
        assert_eq!(names(&recents, "dev"), ["book_covers"]);
        assert_eq!(names(&recents, "production"), Vec::<String>::new());
        assert!(recents.clear("dev"));
        assert!(!recents.clear("dev"));
        assert_eq!(recents.of("dev", "bookshop_test").len(), 0);
    }

    #[test]
    fn what_the_schema_no_longer_has_is_dropped() {
        let mut recents = Recents::default();
        recents.note("dev", table("book_covers", 1));
        recents.note("dev", table("ghost", 2));
        let mut other = table("publishers", 3);
        other.schema = "shop".into();
        recents.note("dev", other);
        let present = |name: &str| name == "book_covers";
        assert!(recents.drop_missing("dev", "bookshop_development", "public", present));
        assert!(!recents.drop_missing("dev", "bookshop_development", "public", present));
        let kept: Vec<String> = names(&recents, "dev");
        // The other schema's entry was not judged.
        assert_eq!(kept, ["publishers", "book_covers"]);
    }

    #[test]
    fn the_file_keeps_what_this_version_cannot_read() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("recents.json");
        let text = r#"{
  "dev": [
    { "kind": "table", "database": "bookshop_development", "schema": "public", "name": "book_covers", "opened": 5, "pinned": true },
    { "kind": "saved query", "name": "ebook covers per publisher", "opened": 4 },
    "not an entry"
  ]
}"#;
        std::fs::write(&path, text).unwrap();
        let mut recents = Recents::load(&path);
        assert_eq!(names(&recents, "dev"), ["book_covers"]);
        recents.note("dev", table("publishers", 6));
        recents.save(&path).unwrap();
        let written: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        let dev = written["dev"].as_array().unwrap();
        assert_eq!(dev.len(), 4);
        assert_eq!(dev[0]["name"], "publishers");
        assert_eq!(dev[1]["pinned"], true, "a newer key stays with its entry");
        assert_eq!(dev[2]["kind"], "saved query");
        assert_eq!(dev[3], "not an entry");
        // Kept entries count, and are never the ones dropped.
        for index in 0..KEPT {
            recents.note("dev", table(&format!("t{index}"), 10 + index as u64));
        }
        recents.save(&path).unwrap();
        let again = Recents::load(&path);
        assert_eq!(again.of("dev", "bookshop_development").len(), KEPT - 2);
        let written: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        assert_eq!(written["dev"].as_array().unwrap().len(), KEPT);
    }

    #[test]
    fn the_first_comment_line_names_a_script() {
        assert_eq!(
            first_comment("SELECT 1;\n  -- ebook covers per publisher \nSELECT 2"),
            Some("ebook covers per publisher".into())
        );
        assert_eq!(first_comment("--\nSELECT 1"), None);
        assert_eq!(
            first_comment("--\n-- ebook covers\nSELECT 1"),
            Some("ebook covers".into()),
            "an empty comment line is passed over"
        );
        assert_eq!(first_comment("SELECT 1 -- not a line comment"), None);
        assert_eq!(first_comment(""), None);
    }

    #[test]
    fn an_entry_of_an_unknown_kind_is_kept_and_never_judged() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("recents.json");
        let text = r#"{"dev": [
  { "kind": "function", "database": "bookshop_development", "schema": "public", "name": "f", "opened": 7 },
  { "kind": "table", "database": "bookshop_development", "schema": "public", "name": "t", "opened": 6 }
]}"#;
        std::fs::write(&path, text).unwrap();
        let mut recents = Recents::load(&path);
        let shown: Vec<String> = recents
            .of("dev", "bookshop_development")
            .iter()
            .filter(|recent| recent.object_kind().is_some())
            .map(|recent| recent.name.clone())
            .collect();
        assert_eq!(shown, ["t"]);
        let present = |name: &str| name == "t";
        assert!(
            !recents.drop_missing("dev", "bookshop_development", "public", present),
            "a list of tables and views says nothing of a function"
        );
        recents.save(&path).unwrap();
        let written: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        assert_eq!(written["dev"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn a_connection_written_in_another_shape_is_kept() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("recents.json");
        let text = r#"{
  "dev": [
    { "kind": "table", "database": "bookshop_development", "schema": "public", "name": "book_covers", "opened": 5 }
  ],
  "newer": { "pinned": ["a"], "entries": [] }
}"#;
        std::fs::write(&path, text).unwrap();
        let mut recents = Recents::load(&path);
        assert_eq!(names(&recents, "dev"), ["book_covers"]);
        recents.save(&path).unwrap();
        let written: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        assert_eq!(written["newer"]["pinned"][0], "a", "written back as it was");
        // Something opened on that connection here takes its place.
        recents.note("newer", table("authors", 9));
        recents.save(&path).unwrap();
        let written: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        assert_eq!(written["newer"][0]["name"], "authors");
        assert!(recents.clear("newer"));
    }

    #[test]
    fn note_never_drops_what_was_just_noted() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("recents.json");
        let entries: Vec<String> = (0..KEPT)
            .map(|index| {
                format!(r#"{{ "kind": "saved query", "name": "q{index}", "opened": {index} }}"#)
            })
            .collect();
        std::fs::write(&path, format!(r#"{{ "dev": [{}] }}"#, entries.join(","))).unwrap();
        let mut recents = Recents::load(&path);
        recents.note("dev", table("authors", 100));
        assert_eq!(names(&recents, "dev"), ["authors"]);
        recents.save(&path).unwrap();
        let written: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        assert_eq!(written["dev"].as_array().unwrap().len(), KEPT + 1);
    }

    #[test]
    fn a_missing_file_is_empty() {
        let dir = tempfile::tempdir().unwrap();
        let recents = Recents::load(&dir.path().join("recents.json"));
        assert_eq!(recents, Recents::default());
    }

    #[test]
    fn when_says_how_long_ago_in_the_zone() {
        // 2026-10-19 10:00:00 UTC, a Monday.
        let now = 1_792_404_000;
        let berlin = TimeZone::fixed(jiff::tz::offset(2));
        let say = |then: u64| when(then, now, &berlin);
        assert_eq!(say(now - 30), "Just now");
        assert_eq!(say(now - 5 * 60), "5 min ago");
        assert_eq!(say(now - 2 * 3_600), "2 h ago");
        // 01:30 Berlin the same day: today, 10 hours ago.
        assert_eq!(say(now - 10 * 3_600 - 30 * 60), "10 h ago");
        assert_eq!(say(now - 24 * 3_600), "Yesterday");
        assert_eq!(say(now - 3 * 86_400), "Fri");
        assert_eq!(say(now - 6 * 86_400), "Tue");
        assert_eq!(say(now - 7 * 86_400), "Oct 12");
        assert_eq!(say(now - 400 * 86_400), "Sep 14, 2025");
        // The day is the zone's: ahead of UTC it is already the 20th, and
        // eleven hours ago was the day before.
        let east = TimeZone::fixed(jiff::tz::offset(14));
        assert_eq!(when(now - 11 * 3_600 - 60, now, &east), "Yesterday");
    }
}

//! The names a Files connection's path lines offer while a path is typed,
//! in the terminal look: under the folder field, its folders; under the
//! file list's add line, its folders and the files a Files connection
//! reads. They are listed as the export float's save to line lists its
//! own ([`crate::export_sheet::completions`]). No disk is read here.

use std::path::{MAIN_SEPARATOR, Path, PathBuf};

use crate::export_sheet::PathList;

/// Which of a folder's names a path line offers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Keep {
    /// Every name: the export float's save to line.
    All,
    /// Folders only: a Files connection's folder field.
    Folders,
    /// Folders, and the files a Files connection reads: the add line.
    Readable,
}

impl Keep {
    /// Whether the name, a folder's or a file's, is offered.
    pub fn keeps(self, name: &str, folder: bool) -> bool {
        match self {
            Self::All => true,
            Self::Folders => folder,
            Self::Readable => folder || tabletist_db::files::Format::of(Path::new(name)).is_some(),
        }
    }
}

/// What a Files path line `typed` asks to list: the folder its last part
/// is in, `~` being `home`, and that part. `None` for a line with no
/// folder, or one with no root (`Data/ord`): the connection would read it
/// from wherever the app was started. A root is enough, with no drive
/// (`/data` on Windows): the connection reads it on the current drive.
pub fn ask(typed: &str, home: Option<&Path>) -> Option<(PathBuf, String)> {
    let typed = typed.trim();
    let at = typed.rfind(['/', MAIN_SEPARATOR])? + 1;
    let (folder, start) = typed.split_at(at);
    let folder = match folder.strip_prefix('~') {
        Some(rest) if rest.starts_with(['/', MAIN_SEPARATOR]) => home?.join(&rest[1..]),
        Some(_) => return None,
        None => PathBuf::from(folder),
    };
    folder.has_root().then(|| (folder, start.to_owned()))
}

/// `list`'s lit name moved by `by`, round.
pub fn moved(list: &mut PathList, by: isize) {
    if list.names.is_empty() {
        return;
    }
    let lit = list.lit.min(list.names.len() - 1) as isize;
    list.lit = (lit + by).rem_euclid(list.names.len() as isize) as usize;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::export_sheet::Listed;

    #[test]
    fn a_line_asks_for_its_folder_when_it_is_absolute() {
        let home = Some(Path::new("/home/reader"));
        assert_eq!(
            ask("/data/book", home),
            Some((PathBuf::from("/data/"), "book".to_owned()))
        );
        assert_eq!(
            ask("~/Data/bookshop-exports/", home),
            Some((
                PathBuf::from("/home/reader/Data/bookshop-exports"),
                String::new()
            ))
        );
        assert_eq!(
            ask("  ~/Data/ret ", home),
            Some((PathBuf::from("/home/reader/Data"), "ret".to_owned()))
        );
        // No folder, or not one the connection reads from here.
        assert_eq!(ask("orders", home), None);
        assert_eq!(ask("Data/ord", home), None);
        assert_eq!(ask("~reader/Data/", home), None);
        assert_eq!(ask("~/Data/", None), None);
        assert_eq!(ask("", home), None);
    }

    #[test]
    fn each_line_keeps_its_own_names() {
        assert!(Keep::All.keeps("notes.txt", false));
        assert!(Keep::Folders.keeps("2025", true));
        assert!(!Keep::Folders.keeps("orders.csv", false));
        assert!(Keep::Readable.keeps("archive", true));
        for name in [
            "orders.csv",
            "returns.TSV",
            "customers.parquet",
            "events.ndjson",
            "events.jsonl",
            "authors.json",
            "stock.xlsx",
            "shop.duckdb",
        ] {
            assert!(Keep::Readable.keeps(name, false), "{name}");
        }
        assert!(!Keep::Readable.keeps("returns-notes.txt", false));
        assert!(!Keep::Readable.keeps("orders", false));
    }

    #[test]
    fn the_lit_name_moves_round() {
        let name = |name: &str| Listed {
            name: name.into(),
            folder: false,
        };
        let mut list = PathList {
            line: "/data/ret".into(),
            names: vec![name("returns.csv"), name("returns.xlsx")],
            lit: 0,
        };
        moved(&mut list, 1);
        assert_eq!(list.lit, 1);
        moved(&mut list, 1);
        assert_eq!(list.lit, 0);
        moved(&mut list, -1);
        assert_eq!(list.lit, 1);
    }
}

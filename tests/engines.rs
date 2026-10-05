//! Code that depends on the database engine answers for every engine.

use std::path::{Path, PathBuf};

/// Every Rust file of the app and of the database layer, by its path from
/// the repository root, with its text.
fn sources() -> Vec<(String, String)> {
    fn walk(dir: &Path, into: &mut Vec<PathBuf>) {
        for entry in std::fs::read_dir(dir).expect("read a source directory") {
            let path = entry.expect("read a directory entry").path();
            if path.is_dir() {
                walk(&path, into);
            } else if path.extension().is_some_and(|ext| ext == "rs") {
                into.push(path);
            }
        }
    }
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut paths = Vec::new();
    walk(&root.join("src"), &mut paths);
    walk(&root.join("crates/tabletist-db/src"), &mut paths);
    assert!(paths.len() > 40, "found the sources");
    paths.sort();
    paths
        .into_iter()
        .map(|path| {
            let name = path.strip_prefix(root).expect("a path under the root");
            // Windows checks the sources out with CRLF line ends.
            let text = std::fs::read_to_string(&path).expect("read a source file");
            (
                name.display().to_string().replace('\\', "/"),
                text.replace("\r\n", "\n"),
            )
        })
        .collect()
}

/// Files whose comparisons are still to become matches. Each task of the
/// plan takes its own out; the last one removes the list.
const PENDING: &[&str] = &[
    "src/app.rs",
    "src/edit.rs",
    "src/model.rs",
    "src/ui/connect_dialog/sheet.rs",
    "src/ui/connect_dialog/terminal.rs",
    "src/ui/picker.rs",
    "src/ui/workspace.rs",
];

/// The code of a file on one line: no comment lines, and nothing from its
/// `tests` module on. A comparison split over lines reads as one, and a
/// test may compare what it likes.
fn code(text: &str) -> String {
    let text = text
        .find("#[cfg(test)]\nmod tests {")
        .map_or(text, |at| &text[..at]);
    text.lines()
        .map(str::trim)
        .filter(|line| !line.starts_with("//"))
        .collect::<Vec<_>>()
        .join(" ")
}

/// The comparisons of a `Driver` or a `Dialect` with one of its variants in
/// `code`: `==`, `!=` and `matches!`, each with a little of what is around.
/// A variant behind `&` or in `Some(` is compared all the same. One written
/// `Self::` is not found: only the two enums' own methods can write that.
fn comparisons(code: &str) -> Vec<String> {
    let mut found = Vec::new();
    for name in ["Driver::", "Dialect::"] {
        for (at, _) in code.match_indices(name) {
            let before = code[..at]
                .trim_end_matches("tabletist_db::")
                .trim_end_matches("crate::")
                .trim_end_matches('&')
                .trim_end_matches("Some(")
                .trim_end();
            let variant = code[at + name.len()..]
                .chars()
                .take_while(char::is_ascii_alphanumeric)
                .count();
            let end = at + name.len() + variant;
            let after = code[end..].trim_start();
            // Inside a `matches!(` that has not closed yet.
            let in_matches = before.rfind("matches!(").is_some_and(|call| {
                let mut depth = 1;
                for c in before[call + "matches!(".len()..].chars() {
                    match c {
                        '(' => depth += 1,
                        ')' => depth -= 1,
                        _ => {}
                    }
                    if depth == 0 {
                        return false;
                    }
                }
                true
            });
            if before.ends_with("==")
                || before.ends_with("!=")
                || after.starts_with("==")
                || after.starts_with("!=")
                || in_matches
            {
                let start = code[..at]
                    .char_indices()
                    .rev()
                    .nth(29)
                    .map_or(0, |(index, _)| index);
                found.push(code[start..end].to_owned());
            }
        }
    }
    found
}

#[test]
fn the_scan_finds_each_way_to_compare() {
    for text in [
        "if spec.driver == Driver::Sqlite {",
        "let remote = tabletist_db::Driver::Sqlite != workspace.driver;",
        "b'#' if dialect\n    == Dialect::MySql => comment(),",
        "if matches!(self.dialect, Dialect::Postgres | Dialect::Sqlite) {",
        "if matches!(driver.dialect(), Dialect::MySql) {",
        "let file = saved.map(|saved| saved.driver) == Some(Driver::Sqlite);",
        "if drivers.iter().any(|driver| driver == &Driver::Sqlite) {",
    ] {
        assert!(!comparisons(&code(text)).is_empty(), "{text}");
    }
    for text in [
        "match driver {\n    Driver::Sqlite => true,\n    Driver::Postgres | Driver::MySql => false,\n}",
        "form.driver = Driver::Sqlite;",
        "// if driver == Driver::Sqlite",
        "let tokens = tokenize(Dialect::Postgres, text);",
        "#[cfg(test)]\nmod tests {\n    assert!(driver == Driver::Sqlite);\n}",
        "Dialect::Postgres if first == \"PREPARE\" => refuse(),",
        "if matches!(kind, TokenKind::Word) && is_keyword(Dialect::Postgres, word) {",
    ] {
        assert_eq!(comparisons(&code(text)), [""; 0], "{text}");
    }
}

#[test]
fn no_code_compares_a_driver_or_a_dialect() {
    // A comparison gives an engine added later the other side's answer
    // without anyone choosing it. A `match` that names every engine does
    // not compile until someone has.
    let found: Vec<String> = sources()
        .iter()
        .filter(|(path, _)| !PENDING.contains(&path.as_str()))
        .flat_map(|(path, text)| {
            comparisons(&code(text))
                .into_iter()
                .map(move |found| format!("{path}: {found}"))
        })
        .collect();
    assert!(
        found.is_empty(),
        "match on every engine instead:\n{}",
        found.join("\n")
    );
}

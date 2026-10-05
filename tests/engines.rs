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
    for dir in ["src", "crates/tabletist-db/src"] {
        let before = paths.len();
        walk(&root.join(dir), &mut paths);
        assert!(paths.len() > before, "no sources under {dir}");
    }
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

/// Where a file's `tests` module starts: `#[cfg(test)]` at the margin, then
/// `mod tests {`, with nothing between them but other attributes. Only that
/// module by that name: a `#[cfg(test)] mod helpers;` half way down a file
/// must not hide the code after it.
fn tests_start(text: &str) -> Option<usize> {
    let mut at = 0;
    let mut marked = None;
    for line in text.split_inclusive('\n') {
        let code = line.trim_end();
        if code == "#[cfg(test)]" {
            marked = Some(at);
        } else if let Some(start) = marked {
            let module = code.strip_suffix("mod tests {");
            if module.is_some_and(|before| before.is_empty() || before.starts_with("pub")) {
                return Some(start);
            }
            if !code.starts_with("#[") {
                marked = None;
            }
        }
        at += line.len();
    }
    None
}

/// The code of a file on one line: no comment lines, and nothing from its
/// `tests` module on. A comparison split over lines reads as one, and a
/// test may compare what it likes.
fn code(text: &str) -> String {
    let text = tests_start(text).map_or(text, |at| &text[..at]);
    text.lines()
        .map(str::trim)
        .filter(|line| !line.starts_with("//"))
        .collect::<Vec<_>>()
        .join(" ")
}

/// `before` without the parentheses that only group what follows it, and
/// how many there were. One that follows a name opens a call's arguments,
/// and stays: `label(Driver::Sqlite) == text` compares the label.
fn ungrouped(mut before: &str) -> (&str, usize) {
    let mut groups = 0;
    while let Some(rest) = before.trim_end().strip_suffix('(') {
        let called = |c: char| c.is_alphanumeric() || matches!(c, '_' | ')' | ']' | '>' | '!');
        if rest.ends_with(called) {
            break;
        }
        before = rest;
        groups += 1;
    }
    (before.trim_end(), groups)
}

/// The comparisons of a `Driver` or a `Dialect` with one of its variants in
/// `code`: `==`, `!=` and `matches!`, each with a little of what is around.
/// A variant behind `&`, in `Some(` or in parentheses of its own is compared
/// all the same. One written `Self::` is not found: only the two enums' own
/// methods can write that.
fn comparisons(code: &str) -> Vec<String> {
    let mut found = Vec::new();
    for name in ["Driver::", "Dialect::"] {
        for (at, _) in code.match_indices(name) {
            // The end of a longer name is another type's.
            if code[..at].ends_with(|c: char| c.is_alphanumeric() || c == '_') {
                continue;
            }
            let left = code[..at]
                .trim_end_matches("tabletist_db::")
                .trim_end_matches("crate::")
                .trim_end_matches('&');
            let (before, groups) = ungrouped(left);
            let before = before.trim_end_matches("Some(").trim_end();
            let variant = code[at + name.len()..]
                .chars()
                .take_while(|c| c.is_alphanumeric() || *c == '_')
                .count();
            let end = at + name.len() + variant;
            let mut after = code[end..].trim_start();
            for _ in 0..groups {
                after = after.strip_prefix(')').map_or(after, str::trim_start);
            }
            // Inside a `matches!(` that has not closed yet.
            let in_matches = left.rfind("matches!(").is_some_and(|call| {
                let mut depth = 1;
                for c in left[call + "matches!(".len()..].chars() {
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
        "if Dialect::Ms_Sql == dialect {",
        "if driver == (Driver::Sqlite) {",
        "if (Dialect::MySql) != dialect {",
        "#[cfg(test)]\nmod helpers;\n\nfn file(driver: Driver) -> bool {\n    driver == Driver::Sqlite\n}",
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
        "#[cfg(test)]\n#[allow(clippy::too_many_lines)]\npub(crate) mod tests {\n    assert!(driver == Driver::Sqlite);\n}",
        "Dialect::Postgres if first == \"PREPARE\" => refuse(),",
        "if matches!(kind, TokenKind::Word) && is_keyword(Dialect::Postgres, word) {",
        "if error == DriverError::Closed || kind != SqlDialect::Ansi {",
        "if kind == MyDriver::Sqlite {",
        "if label(Driver::Sqlite) == text {",
        "if text == quote(Dialect::MySql, name) {",
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

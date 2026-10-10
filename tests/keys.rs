//! No key is spelled outside the keymap. What names a key gets its text
//! from `src/keymap.rs`, by the key's command, so a key that is rebound, or
//! a decision that moves one, shows everywhere at once. The painters of a
//! key take only what the keymap wrote; this finds the rest: a key written
//! by hand into a sentence, a tooltip or a comparison.

use std::path::{Path, PathBuf};

/// What a key written by hand looks like: a modifier and its plus, in any
/// of the ways the app has written one, or a glyph of the Mac's keys.
const SPELLED: [&str; 18] = [
    "ctrl+", "Ctrl+", "cmd+", "Cmd+", "Cmd/", "alt+", "Alt+", "shift+", "Shift+", "Mod+", "⌘", "⇧",
    "⌥", "⌃", "↩", "⌫", "⌦", "⇥",
];

/// The files that may spell a key: the keymap, which is where they are
/// spelled, and the fonts, whose notes name the glyphs the faces hold.
const MAY: [&str; 2] = ["src/keymap.rs", "src/typography/fonts.rs"];

/// The files that are tests and nothing else, or scenes for screenshots: a
/// test says what the user presses and sees, in its own words.
const TESTS: [&str; 5] = [
    "src/testing.rs",
    "src/shots.rs",
    "src/ui/complete_tests.rs",
    "src/ui/env_tests.rs",
    "src/ui/insert_audit_tests.rs",
];

/// Every Rust file of the app, by its path from the repository root, with
/// its text.
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
    assert!(!paths.is_empty(), "no sources under src");
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
/// `mod tests {`, with nothing between them but other attributes.
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

/// The string literals of a line of code: what stands between its double
/// quotes, a quote behind a backslash staying inside. A line that is a
/// comment has none, and a quote that is a character (`'"'`) opens none.
fn literals(line: &str) -> Vec<String> {
    let code = line.trim_start();
    if code.starts_with("//") {
        return Vec::new();
    }
    let mut found = Vec::new();
    let mut inside: Option<String> = None;
    let mut chars = code.chars().peekable();
    let mut last = ' ';
    while let Some(char) = chars.next() {
        match (&mut inside, char) {
            (Some(text), '\\') => {
                text.push(char);
                text.extend(chars.next());
            }
            (Some(_), '"') => found.extend(inside.take()),
            (Some(text), _) => text.push(char),
            // A comment after the code ends the line.
            (None, '/') if chars.peek() == Some(&'/') => break,
            (None, '"') if last != '\'' => inside = Some(String::new()),
            (None, _) => {}
        }
        last = char;
    }
    // A literal the line leaves open goes on below: what it holds so far
    // is read all the same.
    found.extend(inside);
    found
}

#[test]
fn the_literals_of_a_line_are_found() {
    assert_eq!(literals(r#"let a = "ctrl+s";"#), ["ctrl+s"]);
    assert_eq!(literals(r#"f("a", "b \" c") // "d""#), ["a", r#"b \" c"#]);
    assert_eq!(
        literals(r#"    // "ctrl+s" in a comment"#),
        Vec::<String>::new()
    );
    assert_eq!(literals(r#"if c == '"' { "x" }"#), ["x"]);
    assert_eq!(literals(r#"let open = "goes on"#), ["goes on"]);
}

#[test]
fn no_key_is_spelled_outside_the_keymap() {
    let mut found = Vec::new();
    for (name, text) in sources() {
        if MAY.contains(&name.as_str()) || TESTS.contains(&name.as_str()) {
            continue;
        }
        let code = tests_start(&text).map_or(text.as_str(), |at| &text[..at]);
        for (number, line) in code.lines().enumerate() {
            for literal in literals(line) {
                if SPELLED.iter().any(|spelled| literal.contains(spelled)) {
                    found.push(format!("{name}:{}: {literal:?}", number + 1));
                }
            }
        }
    }
    assert!(
        found.is_empty(),
        "keys written by hand, where the keymap should be asked:\n{}",
        found.join("\n")
    );
}

#[test]
fn the_search_finds_a_key_written_by_hand() {
    // What the search is for, so it cannot pass by finding nothing ever.
    for line in [
        r#"let note = "ctrl+shift+w disconnect";"#,
        r#"format!("{}↩ apply", key)"#,
        r#".on_hover_text(gettext(locale, "Cmd/Ctrl+F edits the filter"))"#,
        r#"let chord = "⇧⌘F";"#,
    ] {
        let spelled = literals(line)
            .iter()
            .any(|literal| SPELLED.iter().any(|spelled| literal.contains(spelled)));
        assert!(spelled, "{line}");
    }
}

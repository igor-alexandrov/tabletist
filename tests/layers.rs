//! The lower layer knows nothing of the interface. The modules of the
//! app's state, its edits, the backend, the settings and the text they
//! write never name `crate::ui` or `crate::app`: the views and the reducer
//! stand on them, never the other way round. What one of them needs from
//! above comes in as an argument, or moves down.

use std::path::Path;

/// The modules of the lower layer, each with its directory where it has
/// one.
const LOWER: [&str; 34] = [
    "agents_shown",
    "backend",
    "clock",
    "completion",
    "connections",
    "dates",
    "desktop_theme",
    "edit",
    "endpoint",
    "env",
    "format",
    "history",
    "i18n",
    "json",
    "key_picker",
    "keymap",
    "known_hosts",
    "labels",
    "managed",
    "model",
    "paths",
    "recents",
    "relay",
    "review",
    "search",
    "secrets",
    "session",
    "settings",
    "theme",
    "typography",
    "update",
    "util",
    "vim",
    "zones",
];

/// The modules above it: the interface and the reducer, what starts and
/// drives the window, and the tests' harness and scenes.
const ABOVE: [&str; 10] = [
    "app",
    "entrypoint",
    "lib",
    "macos",
    "main",
    "menu",
    "promo",
    "shots",
    "testing",
    "ui",
];

/// The Rust files of the module `name`, by their path from the repository
/// root, with their text and how many `super::` reach the crate's root
/// from them: `src/name.rs` (or `src/name/mod.rs`) and whatever is under
/// `src/name/`.
fn module(name: &str) -> Vec<(String, String, usize)> {
    fn walk(dir: &Path, into: &mut Vec<std::path::PathBuf>) {
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
    let src = root.join("src");
    let file = src.join(format!("{name}.rs"));
    let dir = src.join(name);
    assert!(
        file.is_file() || dir.join("mod.rs").is_file(),
        "no module src/{name}.rs or src/{name}/mod.rs"
    );
    let mut paths = Vec::new();
    if file.is_file() {
        paths.push(file);
    }
    if dir.is_dir() {
        walk(&dir, &mut paths);
    }
    paths.sort();
    paths
        .into_iter()
        .map(|path| {
            // `src/a/b.rs` is `crate::a::b`, and `src/a/mod.rs` is
            // `crate::a`.
            let inside = path.strip_prefix(&src).expect("a path under src");
            let mod_rs = inside.file_name().is_some_and(|file| file == "mod.rs");
            let depth = inside.components().count() - usize::from(mod_rs);
            let name = path.strip_prefix(root).expect("a path under the root");
            // Windows checks the sources out with CRLF line ends.
            let text = std::fs::read_to_string(&path).expect("read a source file");
            (
                name.display().to_string().replace('\\', "/"),
                text.replace("\r\n", "\n"),
                depth,
            )
        })
        .collect()
}

/// Where a file's `tests` module starts: `#[cfg(test)]` at the margin, then
/// `mod tests {`, with nothing between them but other attributes. A test
/// may drive the interface to see what the module does there.
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
/// `tests` module on. A `use crate::{…}` over many lines reads as one.
fn code(text: &str) -> String {
    let text = tests_start(text).map_or(text, |at| &text[..at]);
    text.lines()
        .map(str::trim)
        .filter(|line| !line.starts_with("//"))
        .collect::<Vec<_>>()
        .join(" ")
}

/// Whether `path` starts with the module `ui` or `app`: `ui::focus`,
/// `app;`, `ui as shown`, but not `uiza` or `apply`.
fn upward(path: &str) -> bool {
    ["ui", "app"].iter().any(|name| {
        path.strip_prefix(name)
            .is_some_and(|rest| !rest.starts_with(|c: char| c.is_alphanumeric() || c == '_'))
    })
}

/// What `code`, in a file `depth` modules below the crate's root, takes
/// from `ui` or `app`: each path, with a little of what follows it. The
/// path starts at `crate::` or at as many `super::` as reach the root. A
/// group (`crate::{app::App, model::Tab}`) is read item by item.
fn upward_paths(code: &str, depth: usize) -> Vec<String> {
    let mut found = Vec::new();
    let supers = "super::".repeat(depth);
    let roots = code
        .match_indices("crate::")
        .chain(code.match_indices(supers.as_str()));
    for (at, root) in roots {
        // The end of a longer name or path is another path's:
        // `tabletist_crate::`, a `super::` further up.
        if code[..at].ends_with(|c: char| c.is_alphanumeric() || c == '_' || c == ':') {
            continue;
        }
        let after = &code[at + root.len()..];
        let shown = |text: &str| text.chars().take(40).collect::<String>();
        if upward(after) {
            found.push(shown(&code[at..]));
        } else if let Some(group) = after.strip_prefix('{') {
            // The items of the group: its first, and each after a comma at
            // the group's own depth.
            let mut depth = 0;
            let mut starts = true;
            for (index, char) in group.char_indices() {
                if starts && upward(&group[index..]) {
                    found.push(format!("{root}{{…{}", shown(&group[index..])));
                }
                if char.is_whitespace() {
                    continue;
                }
                starts = false;
                match char {
                    '{' => depth += 1,
                    '}' if depth == 0 => break,
                    '}' => depth -= 1,
                    ',' => starts = depth == 0,
                    _ => {}
                }
            }
        }
    }
    found
}

#[test]
fn the_search_finds_a_path_up() {
    // What the search is for, so it cannot pass by finding nothing ever.
    for line in [
        "let ring = crate::ui::focus::Ring::Outer { radius: 3 };",
        "pub picked: Option<crate::app::QuickItem>,",
        "use crate::ui::format;",
        "use crate::ui as shown;",
        "use crate::{\n    model::Tab,\n    app::{App, TitleBar},\n};",
        "use crate::{model::{Tab, TabId}, ui};",
        "use super::ui::focus;",
        "use super::{app::App, model::Tab};",
    ] {
        assert!(!upward_paths(&code(line), 1).is_empty(), "{line}");
    }
    // Two modules down, `super::super` is the root.
    assert!(!upward_paths("use super::super::ui::focus;", 2).is_empty());
    for line in [
        "use crate::model::{Tab, apply};",
        "use crate::{model::Tab, uiza::Thing};",
        "// crate::ui::focus in a comment",
        "let url = \"crate::apple\";",
        "use crate::{application, model::{ui_row, Tab}};",
    ] {
        assert!(upward_paths(&code(line), 1).is_empty(), "{line}");
    }
    // A module of the lower one's own that is named `ui` is not the
    // interface.
    assert!(upward_paths("use super::ui::Row;", 2).is_empty());
    assert!(upward_paths("use super::super::super::ui::Row;", 2).is_empty());
}

#[test]
fn every_module_is_below_or_above() {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut unplaced = Vec::new();
    for entry in std::fs::read_dir(&src).expect("read src") {
        let path = entry.expect("read a directory entry").path();
        let Some(name) = path.file_stem().and_then(|stem| stem.to_str()) else {
            continue;
        };
        // A module is a file, or a directory with or without a file of
        // the same name beside it (`src/ui/mod.rs`).
        if (path.is_dir() || path.extension().is_some_and(|ext| ext == "rs"))
            && !LOWER.contains(&name)
            && !ABOVE.contains(&name)
        {
            unplaced.push(name.to_owned());
        }
    }
    unplaced.sort();
    unplaced.dedup();
    assert!(
        unplaced.is_empty(),
        "modules in neither list of tests/layers.rs: {}",
        unplaced.join(", ")
    );
}

#[test]
fn the_lower_layer_names_neither_the_interface_nor_the_app() {
    let mut found = Vec::new();
    for name in LOWER {
        for (file, text, depth) in module(name) {
            for path in upward_paths(&code(&text), depth) {
                found.push(format!("{file}: {path}"));
            }
        }
    }
    assert!(
        found.is_empty(),
        "the lower layer reaches up, where an argument should come down:\n{}",
        found.join("\n")
    );
}

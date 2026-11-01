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

/// Where a file's `tests` module stands: from `#[cfg(test)]` at the
/// margin, then `mod tests {` with nothing between them but other
/// attributes, to its `}` at the margin. A test may drive the interface to
/// see what the module does there; what follows the module is code again.
fn tests_module(text: &str) -> Option<std::ops::Range<usize>> {
    let mut at = 0;
    let mut marked = None;
    let mut start = None;
    for line in text.split_inclusive('\n') {
        let code = line.trim_end();
        if let Some(start) = start {
            if code == "}" {
                return Some(start..at + line.len());
            }
        } else if code == "#[cfg(test)]" {
            marked = Some(at);
        } else if let Some(from) = marked {
            let module = code.strip_suffix("mod tests {");
            if module.is_some_and(|before| before.is_empty() || before.starts_with("pub")) {
                start = Some(from);
            } else if !code.starts_with("#[") {
                marked = None;
            }
        }
        at += line.len();
    }
    // A module left open runs to the end.
    start.map(|start| start..text.len())
}

/// The code of a file on one line: no comment lines, and nothing of its
/// `tests` module. A `use crate::{…}` over many lines reads as one.
fn code(text: &str) -> String {
    let (before, after) = match tests_module(text) {
        Some(tests) => (&text[..tests.start], &text[tests.end..]),
        None => (text, ""),
    };
    before
        .lines()
        .chain(after.lines())
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

/// The items of the group that `group` opens after its `{`: each up to
/// its comma at the group's own depth, or the group's `}`.
fn items(group: &str) -> Vec<&str> {
    let mut found = Vec::new();
    let mut depth = 0;
    let mut start = 0;
    for (index, char) in group.char_indices() {
        match char {
            '{' => depth += 1,
            '}' if depth == 0 => {
                found.push(&group[start..index]);
                return found;
            }
            '}' => depth -= 1,
            ',' if depth == 0 => {
                found.push(&group[start..index]);
                start = index + 1;
            }
            _ => {}
        }
    }
    found.push(&group[start..]);
    found
}

/// Whether `path`, read `up` modules below the crate's root, leads to `ui`
/// or `app`: each `super::` goes one up, and a group's items go on from
/// where the group stands (`super::{super::ui::focus}`).
fn leads_up(path: &str, up: usize) -> bool {
    let path = path.trim_start();
    if let Some(group) = path.strip_prefix('{') {
        return items(group).into_iter().any(|item| leads_up(item, up));
    }
    match (up, path.strip_prefix("super::")) {
        (0, _) => upward(path),
        (_, Some(rest)) => leads_up(rest, up - 1),
        // A module of this one's own, whatever its name.
        (_, None) => false,
    }
}

/// What `code`, in a file `depth` modules below the crate's root, takes
/// from `ui` or `app`: each path, with a little of what follows it. A path
/// starts at `crate::`, or at `super::` and climbs from the file; a group
/// (`crate::{app::App, model::Tab}`) is read item by item.
fn upward_paths(code: &str, depth: usize) -> Vec<String> {
    let mut found = Vec::new();
    let roots = code
        .match_indices("crate::")
        .chain(code.match_indices("super::"));
    for (at, root) in roots {
        // Only where a path starts: not the end of a longer name
        // (`tabletist_crate::`), nor a `super::` that a path or a group
        // already climbs.
        let name = code[..at].ends_with(|c: char| c.is_alphanumeric() || c == '_');
        let path = code[..at].trim_end().ends_with([':', '{', ',']);
        if name || path {
            continue;
        }
        let after = &code[at + root.len()..];
        let up = if root == "crate::" { 0 } else { depth - 1 };
        if leads_up(after, up) {
            found.push(code[at..].chars().take(40).collect());
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
    // Two modules down, `super::super` is the root, also through a group.
    for line in [
        "use super::super::ui::focus;",
        "use super::{super::ui::focus, Row};",
        "use super::{Row, super::{app::App}};",
    ] {
        assert!(!upward_paths(line, 2).is_empty(), "{line}");
    }
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
    assert!(upward_paths("use super::{ui::Row, super::model::Tab};", 2).is_empty());
}

#[test]
fn the_code_after_a_tests_module_is_read() {
    let text = "use crate::model::Tab;\n#[cfg(test)]\nmod tests {\n    use crate::ui::focus;\n}\nuse crate::app::App;\n";
    let code = code(text);
    assert!(!code.contains("crate::ui"), "{code}");
    assert_eq!(upward_paths(&code, 1).len(), 1, "{code}");
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

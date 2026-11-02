//! The lower layer knows nothing of the interface. The modules of the
//! app's state, its edits, the backend, the settings and the text they
//! write never name `crate::ui`, `crate::typography`, `crate::style` or
//! `crate::app`: the views, the text and style they draw with and the
//! reducer stand on them, never the other way round. What one of them needs from
//! above comes in as an argument, or moves down. Their tests stand on the
//! lower layer too: they name neither the interface nor the harness of
//! `crate::testing`, and take their data from `crate::fixtures`.

use std::path::Path;

/// The modules of the lower layer, each with its directory where it has
/// one.
const LOWER: [&str; 35] = [
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
    "fixtures",
    "font_files",
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
    "update",
    "util",
    "vim",
    "zones",
];

/// The modules above it: the interface and the reducer, what starts and
/// drives the window, and the tests' harness and scenes.
const ABOVE: [&str; 12] = [
    "app",
    "entrypoint",
    "lib",
    "macos",
    "main",
    "menu",
    "promo",
    "shots",
    "style",
    "testing",
    "typography",
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

/// What the lower layer's code may not name: the interface (its views, the
/// text roles they draw with and the style it sets up on egui) and the
/// reducer.
const ABOVE_CODE: [&str; 4] = ["ui", "typography", "style", "app"];

/// What the lower layer's tests may not name: the same, and the harness,
/// which drives the whole app.
const ABOVE_TESTS: [&str; 5] = ["ui", "typography", "style", "app", "testing"];

/// Whether `code`, a line trimmed at its end, opens a module at the
/// margin: `mod tests {`, `pub(crate) mod parse_tests {`.
fn opens_module(code: &str) -> bool {
    let Some((before, name)) = code
        .strip_suffix(" {")
        .and_then(|head| head.rsplit_once("mod "))
    else {
        return false;
    };
    (before.is_empty() || before.starts_with("pub"))
        && !name.is_empty()
        && name.chars().all(|c| c.is_alphanumeric() || c == '_')
}

/// Where a file's test modules stand: each from `#[cfg(test)]` at the
/// margin, then `mod … {` with nothing between them but other attributes,
/// to its `}` at the margin. A test may drive the interface to see what the
/// module does there; what follows a test module is code again.
fn test_modules(text: &str) -> Vec<std::ops::Range<usize>> {
    let mut found = Vec::new();
    let mut at = 0;
    let mut marked = None;
    let mut start = None;
    for line in text.split_inclusive('\n') {
        let code = line.trim_end();
        if let Some(from) = start {
            if code == "}" {
                found.push(from..at + line.len());
                start = None;
            }
        } else if code == "#[cfg(test)]" {
            marked = Some(at);
        } else if let Some(from) = marked {
            if opens_module(code) {
                start = Some(from);
                marked = None;
            } else if !code.starts_with("#[") {
                marked = None;
            }
        }
        at += line.len();
    }
    // A module left open runs to the end.
    if let Some(from) = start {
        found.push(from..text.len());
    }
    found
}

/// `lines` on one line, without comment lines: a `use crate::{…}` over
/// many lines reads as one.
fn one_line<'a>(lines: impl Iterator<Item = &'a str>) -> String {
    lines
        .map(str::trim)
        .filter(|line| !line.starts_with("//"))
        .collect::<Vec<_>>()
        .join(" ")
}

/// The code of a file on one line: nothing of its test modules.
fn code(text: &str) -> String {
    let mut parts = Vec::new();
    let mut from = 0;
    for tests in test_modules(text) {
        parts.push(&text[from..tests.start]);
        from = tests.end;
    }
    parts.push(&text[from..]);
    one_line(parts.into_iter().flat_map(str::lines))
}

/// Each of a file's test modules on one line.
fn tests(text: &str) -> Vec<String> {
    test_modules(text)
        .into_iter()
        .map(|tests| one_line(text[tests].lines()))
        .collect()
}

/// Whether `path` starts with one of the modules `names`: for `ui` and
/// `app`, `ui::focus`, `app;`, `ui as shown`, but not `uiza` or `apply`.
fn upward(path: &str, names: &[&str]) -> bool {
    names.iter().any(|name| {
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

/// Whether `path`, read `up` modules below the crate's root, leads to one
/// of the modules `names`: each `super::` goes one up, and a group's items
/// go on from where the group stands (`super::{super::ui::focus}`).
fn leads_up(path: &str, up: usize, names: &[&str]) -> bool {
    let path = path.trim_start();
    if let Some(group) = path.strip_prefix('{') {
        return items(group)
            .into_iter()
            .any(|item| leads_up(item, up, names));
    }
    match (up, path.strip_prefix("super::")) {
        (0, _) => upward(path, names),
        (_, Some(rest)) => leads_up(rest, up - 1, names),
        // A module of this one's own, whatever its name.
        (_, None) => false,
    }
}

/// What `code`, in a file `depth` modules below the crate's root, takes
/// from the interface or `app`.
fn upward_paths(code: &str, depth: usize) -> Vec<String> {
    paths_to(code, depth, &ABOVE_CODE)
}

/// What `code`, `depth` modules below the crate's root, takes from the
/// modules `names`: each path, with a little of what follows it. A path
/// starts at `crate::`, or at `super::` and climbs from where the code
/// stands; a group (`crate::{app::App, model::Tab}`) is read item by item.
fn paths_to(code: &str, depth: usize, names: &[&str]) -> Vec<String> {
    let mut found = Vec::new();
    let roots = code
        .match_indices("crate::")
        .chain(code.match_indices("super::"));
    for (at, root) in roots {
        // Only where a path starts: not the end of a longer name
        // (`tabletist_crate::`), nor a `super::` that a path already
        // climbs. One in a group (`super::{super::ui}`) is read again on
        // its own, from fewer modules up than it stands, so it finds no
        // more than the group does.
        let name = code[..at].ends_with(|c: char| c.is_alphanumeric() || c == '_');
        let path = code[..at].trim_end().ends_with("::");
        if name || path {
            continue;
        }
        let after = &code[at + root.len()..];
        let up = if root == "crate::" { 0 } else { depth - 1 };
        if leads_up(after, up, names) {
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
        "pub picked: crate::app::QuickItem,",
        "fn show(link: crate::ui::Link) {}",
        "shown(1, crate::ui::focus::hint());",
        "{ crate::ui::focus::hint() }",
        "let shown = f(1, super::ui::Row);",
        // The text the views draw with, and the style set up on egui.
        "use crate::typography::{Text, TextRole};",
        "let kind = crate::typography::Kind::Mono;",
        "use crate::style::Icon;",
        "crate::style::apply(ctx, &palette, &look);",
        "use crate::{theme::Look, style};",
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
fn the_search_reads_every_test_module_one_module_down() {
    let text = "use crate::model::Tab;\n\
        #[cfg(test)]\n\
        mod tests {\n    use crate::testing::Harness;\n    use super::super::ui::focus;\n    \
        use crate::fixtures::page;\n}\n\
        fn shown() {}\n\
        #[cfg(test)]\n#[allow(clippy::unwrap_used)]\n\
        mod parse_tests {\n    use super::*;\n    use crate::{fixtures::page, testing::labels};\n}\n";
    let code = code(text);
    assert!(
        code.contains("crate::model::Tab") && code.contains("fn shown"),
        "{code}"
    );
    assert!(!code.contains("testing"), "{code}");
    let tests = tests(text);
    assert_eq!(tests.len(), 2, "{tests:?}");
    // In `src/a.rs`, a test module is two modules down: `super::super` is
    // the root there.
    let found: Vec<String> = tests
        .iter()
        .flat_map(|tests| paths_to(tests, 2, &ABOVE_TESTS))
        .collect();
    assert_eq!(found.len(), 3, "{found:?}");
    // A test that names only the fixtures, or its own module, passes.
    assert!(paths_to("use super::*; use crate::fixtures::page;", 2, &ABOVE_TESTS).is_empty());
    // Nor the text and style the views draw with.
    for line in [
        "use crate::typography::TextRole;",
        "use super::super::style::Icon;",
        "crate::style::install(&ctx, false, &look);",
    ] {
        assert!(!paths_to(line, 2, &ABOVE_TESTS).is_empty(), "{line}");
    }
    // A module named like one, but not it, passes.
    assert!(
        paths_to(
            "use crate::styles::Row; use crate::font_files::read_font;",
            2,
            &ABOVE_TESTS
        )
        .is_empty()
    );
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

#[test]
fn the_lower_layers_tests_name_neither_the_interface_nor_the_harness() {
    let mut found = Vec::new();
    for name in LOWER {
        for (file, text, depth) in module(name) {
            for tests in tests(&text) {
                for path in paths_to(&tests, depth + 1, &ABOVE_TESTS) {
                    found.push(format!("{file}: {path}"));
                }
            }
        }
    }
    assert!(
        found.is_empty(),
        "the lower layer's tests reach up, where `crate::fixtures` should serve them:\n{}",
        found.join("\n")
    );
}

//! The core stays off the interface. It depends on egui for three data
//! types (`Key`, `Color32`, `FontData`), because fastframe-theme stands on
//! egui anyway, but it never draws: no crate of the window, the widgets or
//! the text they lay out in its `Cargo.toml`, and no `egui::Context`,
//! `egui::Ui`, `egui::Painter` or `egui::Response` in its sources, tests
//! included. What sets up egui or draws on it is the `tabletist` crate's.
//! That the core names nothing of the interface the compiler holds: this
//! crate does not depend on that one.

use std::path::{Path, PathBuf};

/// The crates of the window, the widgets and the text they draw, which the
/// core may not depend on.
const DRAWING_CRATES: [&str; 12] = [
    "eframe",
    "winit",
    "raw-window-handle",
    "glow",
    "egui-winit",
    "egui_glow",
    "egui-wgpu",
    "egui_extras",
    "egui_kittest",
    "fastframe-text",
    "fastframe-fonts",
    "fastframe-icons",
];

/// What of egui only the interface names: what draws, and what it answers.
const DRAWING_TYPES: [&str; 4] = ["Context", "Ui", "Painter", "Response"];

/// Every Rust file of this crate, its tests' too, by its path from the
/// crate's directory, with its text.
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
    for dir in ["src", "tests"] {
        walk(&root.join(dir), &mut paths);
    }
    assert!(paths.len() > 50, "found the sources: {paths:?}");
    paths.sort();
    paths
        .into_iter()
        .map(|path| {
            let name = path.strip_prefix(root).expect("a path under the crate");
            // Windows checks the sources out with CRLF line ends.
            let text = std::fs::read_to_string(&path).expect("read a source file");
            (
                name.display().to_string().replace('\\', "/"),
                text.replace("\r\n", "\n"),
            )
        })
        .collect()
}

/// The names of the packages `manifest` depends on, in every table of
/// dependencies: `[dependencies]`, `[dev-dependencies]`,
/// `[build-dependencies]`, a target's. Read as TOML, so a dependency
/// written as a table of its own (`[dependencies.eframe]`) or as a dotted
/// key (`eframe.workspace = true`) is found, and a rename gives both its
/// name and the package it renames (`package = "eframe"`).
fn dependencies(manifest: &str) -> Vec<String> {
    const TABLES: [&str; 3] = ["dependencies", "dev-dependencies", "build-dependencies"];
    let manifest: toml::Table = toml::from_str(manifest).expect("Cargo.toml parses");
    let mut tables: Vec<&toml::Table> = TABLES
        .iter()
        .filter_map(|name| manifest.get(*name)?.as_table())
        .collect();
    if let Some(targets) = manifest.get("target").and_then(toml::Value::as_table) {
        for target in targets.values().filter_map(toml::Value::as_table) {
            tables.extend(
                TABLES
                    .iter()
                    .filter_map(|name| target.get(*name)?.as_table()),
            );
        }
    }
    let mut found = Vec::new();
    for (name, spec) in tables.into_iter().flatten() {
        found.push(name.clone());
        if let Some(package) = spec.get("package").and_then(toml::Value::as_str) {
            found.push(package.to_owned());
        }
    }
    found
}

/// What `text` names of [`DRAWING_TYPES`] through `egui::`: a path
/// (`egui::Context`, `egui::UiBuilder`) or an item of a group
/// (`use egui::{Color32, Ui};`), with a little of what follows.
fn drawing(text: &str) -> Vec<String> {
    let mut found = Vec::new();
    for (at, _) in text.match_indices("egui::") {
        // Not the end of a longer name (`fastframe_egui::`).
        if text[..at].ends_with(|c: char| c.is_alphanumeric() || c == '_') {
            continue;
        }
        let after = &text[at + "egui::".len()..];
        let items: Vec<&str> = match after.strip_prefix('{') {
            Some(group) => group
                .split('}')
                .next()
                .unwrap_or_default()
                .split(',')
                .collect(),
            None => vec![after],
        };
        if items.iter().any(|item| {
            DRAWING_TYPES
                .iter()
                .any(|name| item.trim_start().starts_with(name))
        }) {
            found.push(text[at..].chars().take(40).collect());
        }
    }
    found
}

#[test]
fn the_search_finds_what_draws() {
    // What the search is for, so it cannot pass by finding nothing ever.
    for line in [
        "fn show(ctx: &egui::Context) {}",
        "fn show(ui: &mut egui::Ui) {}",
        "let builder = egui::UiBuilder::new();",
        "use egui::{Color32, Painter};",
        "use egui::{\n    Key,\n    Response,\n};",
    ] {
        assert!(!drawing(line).is_empty(), "{line}");
    }
    for line in [
        "use egui::Color32;",
        "use egui::{Color32, Key};",
        "let data = egui::FontData::from_owned(bytes);",
        "use fastframe_egui::Context;",
        "// a Context of its own: crate::backend::Context",
    ] {
        assert!(drawing(line).is_empty(), "{line}");
    }
    let manifest = "[dependencies]\negui = \"0.36\"\n# eframe = \"0.36\"\n\
        glow.workspace = true\nwindow = { package = \"egui-winit\", version = \"0.36\" }\n\
        [dependencies.egui_extras]\nversion = \"0.36\"\n\
        [target.'cfg(windows)'.dependencies]\nwinit = \"0.30\"\n\
        [target.'cfg(unix)'.dev-dependencies.eframe]\nversion = \"0.36\"\n\
        [build-dependencies]\ncc = \"1\"\n[features]\nfastframe-text = []\n";
    let mut named = dependencies(manifest);
    named.sort();
    assert_eq!(
        named,
        [
            "cc",
            "eframe",
            "egui",
            "egui-winit",
            "egui_extras",
            "glow",
            "window",
            "winit"
        ]
    );
}

#[test]
fn the_core_depends_on_nothing_that_draws() {
    let manifest = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml"))
        .expect("read Cargo.toml");
    let named = dependencies(&manifest);
    assert!(named.iter().any(|name| name == "egui"), "{named:?}");
    let found: Vec<&String> = named
        .iter()
        .filter(|name| DRAWING_CRATES.contains(&name.as_str()))
        .collect();
    assert!(
        found.is_empty(),
        "the interface's crates belong to the tabletist crate: {found:?}"
    );
}

#[test]
fn the_core_names_nothing_of_egui_that_draws() {
    let mut found = Vec::new();
    for (path, text) in sources() {
        // This file names them to look for them.
        if path == "tests/interface.rs" {
            continue;
        }
        for hit in drawing(&text) {
            found.push(format!("{path}: {hit}"));
        }
    }
    assert!(
        found.is_empty(),
        "what draws on egui belongs to the tabletist crate (src/style.rs, src/ui/):\n{}",
        found.join("\n")
    );
}

//! An environment's colours and its matches live in the core's `env.rs`,
//! and only the surfaces that show an environment ask for its colours.
//! These read the whole app, its interface's sources and its core's, so
//! they stand here and not in the core, whose tests stand on it alone.

use std::path::{Path, PathBuf};

use tabletist::env::{Environment, Platform, env_colors};
use tabletist::theme::Palette;

const OWNER: &str = "crates/tabletist-core/src/env.rs";
const THEME: &str = "crates/tabletist-core/src/theme.rs";

/// Every Rust file of the app, by its path from the repository root, with
/// its text: the interface's under `src/` and the core's under
/// `crates/tabletist-core/src/`.
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
    for dir in ["src", "crates/tabletist-core/src"] {
        let before = paths.len();
        walk(&root.join(dir), &mut paths);
        assert!(paths.len() - before > 20, "found the sources under {dir}");
    }
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

/// `text` lower-cased without whitespace, so `0xC2, 0x26, 0x1F` and a match
/// arm split over lines read the same wherever they appear.
fn squeezed(text: &str) -> String {
    text.chars()
        .filter(|c| !c.is_whitespace())
        .flat_map(char::to_lowercase)
        .collect()
}

/// The code of a file: no comment lines, and nothing from its `tests`
/// module on. Colours named in prose or as test data are not drawn.
fn code(text: &str) -> String {
    let text = text
        .find("#[cfg(test)]\nmod tests {")
        .map_or(text, |at| &text[..at]);
    text.lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn environment_colours_are_written_only_in_their_module() {
    // The light palette's text colours the design also gives badge text
    // (danger, warning, magenta, secondary): the palette may name them.
    let light = Palette::light();
    let shared = [light.danger, light.warning, light.magenta, light.secondary];
    // In the light, the native colours are the design's fixed ones as they
    // stand: base, badge fill, badge text.
    let native = |env| {
        let colors = env_colors(env, Platform::Native, &light);
        [colors.base(), colors.badge_bg(), colors.badge_fg()]
    };
    // The value tags' violet slot is the local badge's fill and text.
    // Pending a decision on recolouring it, only those two, only there.
    let [_, local_bg, local_fg] = native(Environment::Local);
    let value_tags = [local_bg, local_fg];
    let mut found = Vec::new();
    for env in Environment::ALL {
        for color in native(env) {
            let [r, g, b, _] = color.to_array();
            let forms = [
                format!("#{r:02x}{g:02x}{b:02x}"),
                format!("0x{r:02x}{g:02x}{b:02x}"),
                format!("0x{r:02x},0x{g:02x},0x{b:02x}"),
            ];
            for (path, text) in sources() {
                if path == OWNER
                    || (path == THEME && shared.contains(&color))
                    || (path == "src/ui/value_tags.rs" && value_tags.contains(&color))
                {
                    continue;
                }
                let text = squeezed(&text);
                for form in &forms {
                    if text.contains(form.as_str()) {
                        found.push(format!("{path}: {form} ({env:?})"));
                    }
                }
            }
        }
    }
    assert!(found.is_empty(), "use env_colors():\n{}", found.join("\n"));
}

#[test]
fn no_view_writes_a_hex_colour() {
    // The palette and its theme files, and the screenshots' and the tests'
    // copies of a theme, are colours by name, not a connection's.
    let allowed = [
        OWNER,
        THEME,
        "src/shots.rs",
        "crates/tabletist-core/src/fixtures.rs",
    ];
    let mut found = Vec::new();
    for (path, text) in sources() {
        if allowed.contains(&path.as_str()) {
            continue;
        }
        let text = code(&text);
        for (at, _) in text.match_indices('#') {
            let digits = text[at + 1..]
                .chars()
                .take_while(char::is_ascii_hexdigit)
                .count();
            if digits == 6 {
                found.push(format!("{path}: {}", &text[at..at + 7]));
            }
        }
    }
    assert!(found.is_empty(), "{}", found.join("\n"));
}

#[test]
fn environments_are_matched_only_in_their_module() {
    // There the match is exhaustive (the lint below), so a new environment
    // cannot be forgotten anywhere.
    let mut found = Vec::new();
    for (path, text) in sources() {
        if path == OWNER {
            continue;
        }
        let text = squeezed(&text);
        for (at, _) in text.match_indices("environment::") {
            let rest = &text[at + "environment::".len()..];
            let name = rest
                .chars()
                .take_while(|c| c.is_alphanumeric() || *c == '_')
                .count();
            let after = &rest[name..];
            if after.starts_with("=>") || (after.starts_with('|') && !after.starts_with("||")) {
                found.push(format!("{path}: {}", &text[at..at + 13 + name]));
            }
        }
    }
    assert!(
        found.is_empty(),
        "match in env.rs instead:\n{}",
        found.join("\n")
    );
    let owner = sources()
        .into_iter()
        .find(|(path, _)| path == OWNER)
        .expect("env.rs is among the sources");
    assert!(
        owner
            .1
            .contains("#![deny(clippy::wildcard_enum_match_arm)]")
    );
}

#[test]
fn only_the_environment_surfaces_ask_for_its_colours() {
    // A view that shows an environment calls env_colors(); nothing else
    // colours a connection, and nothing stores a colour for one.
    let surfaces = [
        OWNER,
        "src/ui/workspace.rs",
        // The table's header names production beside the table.
        "src/ui/data_view.rs",
        "src/ui/picker.rs",
        "src/ui/connect_dialog/mod.rs",
        "src/ui/env_tests.rs",
        "src/ui/settings/preview.rs",
        "src/ui/start.rs",
        "src/ui/start_tests.rs",
        "src/ui/quick_open.rs",
        // The Export sheet's subtitle, and the float's head.
        "src/ui/export.rs",
    ];
    let mut found = Vec::new();
    for (path, text) in sources() {
        if text.contains("env_colors(") && !surfaces.contains(&path.as_str()) {
            found.push(format!("{path}: calls env_colors()"));
        }
        // Spelt in two so this test does not find itself.
        if text.contains(concat!("Color", "Tag")) {
            found.push(format!("{path}: names the removed colour tag"));
        }
        let stored = [
            "crates/tabletist-core/src/connections.rs",
            "crates/tabletist-core/src/model.rs",
            "crates/tabletist-core/src/settings.rs",
        ];
        if stored.contains(&path.as_str()) && text.contains("Color32") {
            found.push(format!("{path}: a stored colour"));
        }
    }
    assert!(found.is_empty(), "{}", found.join("\n"));
}

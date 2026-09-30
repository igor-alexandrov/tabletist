//! A connection's environment, and everything derived from it: its colours
//! on every surface, its badge label, and its safety defaults. Nothing else
//! decides a connection's colour; the colours are never stored.

// A new environment must be handled everywhere it is matched.
#![deny(clippy::wildcard_enum_match_arm)]

use egui::Color32;
use serde::{Deserialize, Serialize};
use tabletist_db::{ConnectSpec, Driver};

use crate::theme::{Look, Palette, contrast, mix};

/// What a connection is. It decides the connection's colour everywhere and
/// whether it is read-only by default.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Environment {
    Local,
    Dev,
    Staging,
    Production,
    #[default]
    None,
}

impl Environment {
    /// In the order the dialog offers them.
    pub const ALL: [Environment; 5] = [
        Self::Local,
        Self::Dev,
        Self::Staging,
        Self::Production,
        Self::None,
    ];

    /// A new connection's environment until one is chosen: `Local` for a
    /// file or a server on this machine reached directly, else `None`.
    /// Through an SSH tunnel `localhost` is the far host's, so not local.
    pub fn for_spec(spec: &ConnectSpec) -> Self {
        Self::for_target(spec.driver, &spec.host, spec.ssh.is_some())
    }

    /// [`Environment::for_spec`] from the parts the dialog holds.
    pub fn for_target(driver: Driver, host: &str, tunnelled: bool) -> Self {
        let local = match driver {
            Driver::Sqlite => true,
            Driver::Postgres | Driver::MySql => !tunnelled && crate::model::is_local_host(host),
        };
        if local { Self::Local } else { Self::None }
    }

    /// Whether a connection in this environment is read-only unless its
    /// user says otherwise.
    pub fn read_only_by_default(self) -> bool {
        match self {
            Self::Production => true,
            Self::Local | Self::Dev | Self::Staging | Self::None => false,
        }
    }

    /// The badge text: the lower-case name in a macOS or Windows pill, the
    /// terminal's upper-case tag on Omarchy.
    pub fn label(self, platform: Platform) -> &'static str {
        match platform {
            Platform::Native => match self {
                Self::Local => "local",
                Self::Dev => "dev",
                Self::Staging => "staging",
                Self::Production => "production",
                Self::None => "none",
            },
            Platform::Omarchy => match self {
                Self::Local => "LOCAL",
                Self::Dev => "DEV",
                Self::Staging => "STAGING",
                Self::Production => "PROD",
                Self::None => "NONE",
            },
        }
    }
}

/// Which colour scheme environments take.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Platform {
    /// macOS and Windows: the design's fixed colours.
    Native,
    /// Omarchy and every other Linux desktop: the theme's own colours.
    Omarchy,
}

impl Platform {
    /// The scheme a look draws with. Every Linux desktop gets the terminal
    /// look.
    pub fn of(look: &Look) -> Self {
        if look.terminal {
            Self::Omarchy
        } else {
            Self::Native
        }
    }
}

/// How an environment draws. Only [`env_colors`] makes one.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EnvColors {
    base: Color32,
    badge_bg: Color32,
    badge_fg: Color32,
    bar_bg: Color32,
    bar_border: Color32,
}

impl EnvColors {
    /// The environment's own colour: the macOS bar's stripe, the list's
    /// row stripe, the tab dot.
    pub fn base(&self) -> Color32 {
        self.base
    }

    pub fn badge_bg(&self) -> Color32 {
        self.badge_bg
    }

    pub fn badge_fg(&self) -> Color32 {
        self.badge_fg
    }

    /// The connection bar's tint.
    pub fn bar_bg(&self) -> Color32 {
        self.bar_bg
    }

    /// The connection bar's rule, and the lines drawn over its tint.
    pub fn bar_border(&self) -> Color32 {
        self.bar_border
    }
}

/// An environment's colours on `platform` with `palette`. macOS and Windows
/// take fixed colours (softened into the panel in dark mode); Linux takes
/// the theme's red, yellow, green, magenta and muted, so it follows a
/// theme change.
pub fn env_colors(env: Environment, platform: Platform, palette: &Palette) -> EnvColors {
    match platform {
        Platform::Native => native(env, palette),
        Platform::Omarchy => omarchy(env, palette),
    }
}

/// The design's fixed colours: base, badge fill, badge text.
fn native_table(env: Environment) -> (Color32, Color32, Color32) {
    let rgb = Color32::from_rgb;
    match env {
        Environment::Local => (
            rgb(0x7c, 0x4d, 0xdb),
            rgb(0xef, 0xea, 0xf9),
            rgb(0x5b, 0x3a, 0xa8),
        ),
        Environment::Dev => (
            rgb(0x2c, 0x7a, 0x4b),
            rgb(0xe3, 0xf1, 0xe6),
            rgb(0x1f, 0x6b, 0x35),
        ),
        Environment::Staging => (
            rgb(0xd0, 0x8a, 0x12),
            rgb(0xfb, 0xef, 0xd6),
            rgb(0x8a, 0x5a, 0x00),
        ),
        Environment::Production => (
            rgb(0xc2, 0x26, 0x1f),
            rgb(0xfb, 0xe3, 0xe1),
            rgb(0xa3, 0x23, 0x1b),
        ),
        Environment::None => (
            rgb(0xb5, 0xb3, 0xad),
            rgb(0xef, 0xee, 0xe9),
            rgb(0x4d, 0x4c, 0x48),
        ),
    }
}

fn native(env: Environment, palette: &Palette) -> EnvColors {
    let (base, badge_bg, badge_fg) = native_table(env);
    if !palette.dark {
        return EnvColors {
            base,
            badge_bg,
            badge_fg,
            bar_bg: mix(Color32::WHITE, base, 0.12),
            bar_border: mix(Color32::WHITE, base, 0.28),
        };
    }
    // Dark mode keeps the colour but mixes it into the panel, and lightens
    // the badge text until it reads on its fill.
    let badge_bg = mix(palette.panel, base, 0.2);
    EnvColors {
        base,
        badge_bg,
        badge_fg: readable(base, badge_bg),
        bar_bg: mix(palette.panel, base, 0.12),
        bar_border: mix(palette.panel, base, 0.28),
    }
}

/// `color` lightened towards white until it has 4.5:1 against `background`.
fn readable(color: Color32, background: Color32) -> Color32 {
    let mut color = color;
    let mut step = 0;
    while contrast(color, background) < 4.5 && step < 50 {
        color = color.lerp_to_gamma(Color32::WHITE, 0.04);
        step += 1;
    }
    color
}

/// The theme key each environment takes, as the Omarchy template maps it
/// onto the palette (`dark_background` is the panel).
fn omarchy(env: Environment, palette: &Palette) -> EnvColors {
    let base = match env {
        // `magenta`: Omarchy resolves it from `purple` when a theme names
        // only that.
        Environment::Local => palette.magenta,
        Environment::Dev => palette.success,
        Environment::Staging => palette.warning,
        Environment::Production => palette.danger,
        // `muted`, lightened only if it would not read as a label.
        Environment::None => palette.dim,
    };
    EnvColors {
        base,
        badge_bg: base,
        badge_fg: palette.window,
        bar_bg: mix(palette.panel, base, 0.16),
        bar_border: mix(palette.panel, base, 0.4),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hex(value: u32) -> Color32 {
        let [_, r, g, b] = value.to_be_bytes();
        Color32::from_rgb(r, g, b)
    }

    #[test]
    fn macos_takes_the_designs_colours() {
        let light = Palette::light();
        for (env, base, badge_bg, badge_fg) in [
            (Environment::Local, 0x7c4ddb, 0xefeaf9, 0x5b3aa8),
            (Environment::Dev, 0x2c7a4b, 0xe3f1e6, 0x1f6b35),
            (Environment::Staging, 0xd08a12, 0xfbefd6, 0x8a5a00),
            (Environment::Production, 0xc2261f, 0xfbe3e1, 0xa3231b),
            (Environment::None, 0xb5b3ad, 0xefeee9, 0x4d4c48),
        ] {
            let colors = env_colors(env, Platform::Native, &light);
            assert_eq!(colors.base(), hex(base), "{env:?}");
            assert_eq!(colors.badge_bg(), hex(badge_bg), "{env:?}");
            assert_eq!(colors.badge_fg(), hex(badge_fg), "{env:?}");
            assert_eq!(colors.bar_bg(), mix(Color32::WHITE, hex(base), 0.12));
            assert_eq!(colors.bar_border(), mix(Color32::WHITE, hex(base), 0.28));
        }
    }

    #[test]
    fn macos_dark_mode_keeps_the_colour_and_mixes_it_into_the_panel() {
        let dark = Palette::dark();
        for env in Environment::ALL {
            let light = env_colors(env, Platform::Native, &Palette::light());
            let colors = env_colors(env, Platform::Native, &dark);
            assert_eq!(colors.base(), light.base(), "{env:?}");
            assert_eq!(colors.bar_bg(), mix(dark.panel, colors.base(), 0.12));
            assert_eq!(colors.bar_border(), mix(dark.panel, colors.base(), 0.28));
            assert_eq!(colors.badge_bg(), mix(dark.panel, colors.base(), 0.2));
            assert!(
                contrast(colors.badge_fg(), colors.badge_bg()) >= 4.5,
                "{env:?}"
            );
        }
    }

    /// `omarchy-theme-color --all` for Omarchy's bundled themes, cut to the
    /// keys the template reads.
    const THEMES: [(&str, &str); 5] = [
        (
            "tokyo-night",
            "mode dark background #1a1b26 dark_background #13141c \
             lighter_background #24283b foreground #a9b1d6 muted #414868 \
             accent #7aa2f7 selection #292e42 red #f7768e green #9ece6a \
             yellow #e0af68 cyan #449dab orange #eb927b magenta #ad8ee6 \
             blue #7aa2f7 bright_green #b9f27c bright_red #ff7a93",
        ),
        (
            "catppuccin-latte",
            "mode light background #eff1f5 dark_background #e3e4e8 \
             lighter_background #dce0e8 foreground #4c4f69 muted #acb0be \
             accent #1e66f5 selection #ccd0da red #d20f39 green #40a02b \
             yellow #df8e1d cyan #179299 orange #d84e2b magenta #ea76cb \
             blue #1e66f5 bright_green #40a02b bright_red #d20f39",
        ),
        (
            "gruvbox",
            "mode dark background #282828 dark_background #1e1e1e \
             lighter_background #3c3836 foreground #d4be98 muted #665c54 \
             accent #7daea3 selection #504945 red #ea6962 green #a9b665 \
             yellow #d8a657 cyan #89b482 orange #e1875c magenta #d3869b \
             blue #7daea3 bright_green #a9b665 bright_red #ea6962",
        ),
        (
            "rose-pine",
            "mode light background #faf4ed dark_background #ede7e1 \
             lighter_background #f2e9e1 foreground #575279 muted #cecacd \
             accent #56949f selection #dfdad9 red #b4637a green #286983 \
             yellow #ea9d34 cyan #d7827e orange #cf8057 magenta #907aa9 \
             blue #56949f bright_green #286983 bright_red #b4637a",
        ),
        (
            "flexoki-light",
            "mode light background #FFFCF0 dark_background #f2efe4 \
             lighter_background #E6E4D9 foreground #100F0F muted #B7B5AC \
             accent #205EA6 selection #CECDC3 red #D14D41 green #879A39 \
             yellow #D0A215 cyan #3AA99F orange #d0772b magenta #CE5D97 \
             blue #205EA6 bright_green #879A39 bright_red #D14D41",
        ),
    ];

    /// A theme's colours by key, and the palette the Omarchy template
    /// renders from them (as the app resolves it).
    fn render(colors: &str) -> (std::collections::BTreeMap<&str, Color32>, Palette) {
        let words: Vec<&str> = colors.split_whitespace().collect();
        let keys: std::collections::BTreeMap<&str, &str> =
            words.chunks(2).map(|pair| (pair[0], pair[1])).collect();
        let seed: String = keys.iter().map(|(k, v)| format!("{k}\t{v}\n")).collect();
        let rendered = fastframe_theme::omarchy::render_seed::<Palette>(
            include_str!("../contrib/omarchy/tabletist.json.tpl"),
            &seed,
        )
        .unwrap();
        let palette: Palette = fastframe_theme::parse_palette(&rendered).unwrap();
        let rgb = keys
            .iter()
            .filter_map(|(key, value)| {
                let value = u32::from_str_radix(value.strip_prefix('#')?, 16).ok()?;
                Some((*key, hex(value)))
            })
            .collect();
        (rgb, palette.with_readable_labels())
    }

    #[test]
    fn omarchy_takes_each_environment_from_its_theme_key() {
        for (theme, colors) in THEMES {
            let (keys, palette) = render(colors);
            for (env, key) in [
                (Environment::Local, "magenta"),
                (Environment::Dev, "green"),
                (Environment::Staging, "yellow"),
                (Environment::Production, "red"),
            ] {
                let colors = env_colors(env, Platform::Omarchy, &palette);
                assert_eq!(colors.base(), keys[key], "{theme} {env:?}");
            }
            // Muted, lightened only when a label in it would not read.
            let none = env_colors(Environment::None, Platform::Omarchy, &palette).base();
            assert_eq!(none, palette.dim, "{theme}");
            if contrast(keys["muted"], palette.window) >= 4.5 {
                assert_eq!(none, keys["muted"], "{theme}");
            }
            for env in Environment::ALL {
                let colors = env_colors(env, Platform::Omarchy, &palette);
                let dark_background = keys["dark_background"];
                assert_eq!(colors.bar_bg(), mix(dark_background, colors.base(), 0.16));
                assert_eq!(
                    colors.bar_border(),
                    mix(dark_background, colors.base(), 0.4)
                );
            }
        }
    }

    #[test]
    fn omarchy_colours_follow_a_theme_change() {
        let (_, first) = render(THEMES[0].1);
        let (_, second) = render(THEMES[2].1);
        for env in Environment::ALL {
            assert_ne!(
                env_colors(env, Platform::Omarchy, &first),
                env_colors(env, Platform::Omarchy, &second),
                "{env:?}"
            );
        }
    }

    #[test]
    fn labels_are_lower_case_pills_and_upper_case_tags() {
        let native: Vec<_> = Environment::ALL
            .iter()
            .map(|env| env.label(Platform::Native))
            .collect();
        assert_eq!(native, ["local", "dev", "staging", "production", "none"]);
        let omarchy: Vec<_> = Environment::ALL
            .iter()
            .map(|env| env.label(Platform::Omarchy))
            .collect();
        assert_eq!(omarchy, ["LOCAL", "DEV", "STAGING", "PROD", "NONE"]);
    }

    #[test]
    fn a_new_connection_on_this_machine_is_local() {
        for host in [
            "localhost",
            "127.0.0.1",
            "::1",
            "[::1]",
            "/var/run/postgresql",
        ] {
            assert_eq!(
                Environment::for_target(Driver::Postgres, host, false),
                Environment::Local,
                "{host}"
            );
        }
        assert_eq!(
            Environment::for_target(Driver::Sqlite, "", false),
            Environment::Local
        );
        assert_eq!(
            Environment::for_target(Driver::MySql, "db.example.com", false),
            Environment::None
        );
        // Through a tunnel, localhost is the far side's.
        assert_eq!(
            Environment::for_target(Driver::Postgres, "localhost", true),
            Environment::None
        );
    }

    #[test]
    fn only_production_is_read_only_by_default() {
        for env in Environment::ALL {
            assert_eq!(
                env.read_only_by_default(),
                env == Environment::Production,
                "{env:?}"
            );
        }
    }

    /// Every Rust file under `src/`, by its path from the crate root, with
    /// its text.
    fn sources() -> Vec<(String, String)> {
        fn walk(dir: &std::path::Path, into: &mut Vec<std::path::PathBuf>) {
            for entry in std::fs::read_dir(dir).unwrap() {
                let path = entry.unwrap().path();
                if path.is_dir() {
                    walk(&path, into);
                } else if path.extension().is_some_and(|ext| ext == "rs") {
                    into.push(path);
                }
            }
        }
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let mut paths = Vec::new();
        walk(&root.join("src"), &mut paths);
        assert!(paths.len() > 20, "found the sources");
        paths
            .into_iter()
            .map(|path| {
                let name = path.strip_prefix(root).unwrap().display().to_string();
                // Windows checks the sources out with CRLF line ends.
                let text = std::fs::read_to_string(&path).unwrap();
                (name.replace('\\', "/"), text.replace("\r\n", "\n"))
            })
            .collect()
    }

    /// `text` lower-cased without whitespace, so `0xC2, 0x26, 0x1F` and a
    /// match arm split over lines read the same wherever they appear.
    fn squeezed(text: &str) -> String {
        text.chars()
            .filter(|c| !c.is_whitespace())
            .flat_map(char::to_lowercase)
            .collect()
    }

    const OWNER: &str = "src/env.rs";

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
    fn environment_colours_are_written_only_in_this_module() {
        // The light palette's text colours the design also gives badge text
        // (danger, warning, magenta, secondary): the palette may name them.
        let light = Palette::light();
        let shared = [light.danger, light.warning, light.magenta, light.secondary];
        // The value tags' violet slot is the local badge's fill and text.
        // Pending a decision on recolouring it, only those two, only there.
        let (_, local_bg, local_fg) = native_table(Environment::Local);
        let value_tags = [local_bg, local_fg];
        let mut found = Vec::new();
        for env in Environment::ALL {
            let (base, badge_bg, badge_fg) = native_table(env);
            for color in [base, badge_bg, badge_fg] {
                let [r, g, b, _] = color.to_array();
                let forms = [
                    format!("#{r:02x}{g:02x}{b:02x}"),
                    format!("0x{r:02x}{g:02x}{b:02x}"),
                    format!("0x{r:02x},0x{g:02x},0x{b:02x}"),
                ];
                for (path, text) in sources() {
                    if path == OWNER
                        || (path == "src/theme.rs" && shared.contains(&color))
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
        // The palette and its theme files, and the screenshots' copy of a
        // theme, are colours by name, not a connection's.
        let allowed = [OWNER, "src/theme.rs", "src/shots.rs"];
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
    fn environments_are_matched_only_in_this_module() {
        // Here the match is exhaustive (the lint above), so a new
        // environment cannot be forgotten anywhere.
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
            .unwrap();
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
            "src/ui/picker.rs",
            "src/ui/conn_tabs.rs",
            "src/ui/connect_dialog.rs",
            "src/ui/env_tests.rs",
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
            let stored = ["src/connections.rs", "src/model.rs", "src/settings.rs"];
            if stored.contains(&path.as_str()) && text.contains("Color32") {
                found.push(format!("{path}: a stored colour"));
            }
        }
        assert!(found.is_empty(), "{}", found.join("\n"));
    }
}

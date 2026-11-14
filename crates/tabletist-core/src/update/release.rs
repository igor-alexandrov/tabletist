//! The latest release as GitHub's API says it, and what Settings ›
//! Updates makes of it: whether it is newer than this version, what it
//! says is new, and the download this computer would take.

use serde::{Deserialize, Serialize};

/// The fields of `GET /repos/<owner>/<name>/releases/latest` the pane
/// reads. Anything else in the answer is left.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq)]
pub struct Latest {
    #[serde(default)]
    pub tag_name: String,
    #[serde(default)]
    pub html_url: String,
    #[serde(default)]
    pub draft: bool,
    #[serde(default)]
    pub prerelease: bool,
    /// The release's notes, in Markdown.
    #[serde(default)]
    pub body: Option<String>,
    /// When it was published, as RFC 3339.
    #[serde(default)]
    pub published_at: Option<String>,
    #[serde(default)]
    pub assets: Vec<Asset>,
}

/// A file of a release.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq)]
pub struct Asset {
    #[serde(default)]
    pub name: String,
    /// In bytes.
    #[serde(default)]
    pub size: u64,
}

/// A release, as the pane and the state file keep it.
#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
pub struct Offer {
    /// Its version, without the tag's `v`: `0.4.0`.
    pub version: String,
    /// Its page, with the notes and every download.
    pub url: String,
    /// The first [`NOTES`] points of its notes, each as Markdown.
    #[serde(default)]
    pub notes: Vec<String>,
    /// All of its notes, as Markdown: what the pager shows.
    #[serde(default)]
    pub body: String,
    /// When it was published, in seconds since the Unix epoch.
    #[serde(default)]
    pub published: Option<u64>,
    /// The download this computer would take, and its size in bytes.
    /// None where the release has no build for it.
    #[serde(default)]
    pub download: Option<(String, u64)>,
}

/// How many points of a release's notes the pane shows.
pub const NOTES: usize = 4;

/// What a check found, against the version that runs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Found {
    /// A newer release, to offer.
    Newer(Offer),
    /// Nothing newer, or only the version the person said to skip.
    UpToDate,
}

/// The version a tag names: `v0.4.0` and `0.4.0` are `0.4.0`.
pub fn version_of(tag: &str) -> &str {
    tag.strip_prefix('v').unwrap_or(tag)
}

/// What `latest` is to the version `current`: newer, or not. A draft or a
/// pre-release is never offered (GitHub's latest release is neither, but
/// a feed could say so). A release equal to `skip` is not offered while
/// `skip` holds; one newer than it is.
pub fn found(
    latest: &Latest,
    current: &str,
    skip: Option<&str>,
    os: Os,
    kind: Option<Kind>,
) -> Found {
    if latest.draft || latest.prerelease {
        return Found::UpToDate;
    }
    let version = version_of(&latest.tag_name);
    if !fastframe_update::is_newer(version, current) || skip == Some(version) {
        return Found::UpToDate;
    }
    Found::Newer(offer(latest, os, kind))
}

/// `latest` as the pane keeps it, with the download for `os` and `kind`.
pub fn offer(latest: &Latest, os: Os, kind: Option<Kind>) -> Offer {
    let version = version_of(&latest.tag_name).to_owned();
    let download = asset_name(&version, os, kind).and_then(|name| {
        latest
            .assets
            .iter()
            .find(|asset| asset.name == name)
            .map(|asset| (asset.name.clone(), asset.size))
    });
    Offer {
        url: latest.html_url.clone(),
        notes: notes(latest.body.as_deref().unwrap_or_default()),
        body: latest.body.clone().unwrap_or_default(),
        published: latest
            .published_at
            .as_deref()
            .and_then(|at| at.parse::<jiff::Timestamp>().ok())
            .and_then(|at| u64::try_from(at.as_second()).ok()),
        download,
        version,
    }
}

/// The first [`NOTES`] points of a release's notes, each without its
/// bullet: a line that starts with `- `, `* ` or `+ `, at any depth.
pub fn notes(body: &str) -> Vec<String> {
    body.lines()
        .map(str::trim_start)
        .filter_map(|line| {
            ["- ", "* ", "+ "]
                .iter()
                .find_map(|bullet| line.strip_prefix(bullet))
        })
        .map(str::trim)
        .filter(|point| !point.is_empty())
        .take(NOTES)
        .map(str::to_owned)
        .collect()
}

/// A point of the notes in runs of text, each strong or not: `**bold**`
/// and `__bold__` are strong, a link is its words, code is its text.
pub fn runs(point: &str) -> Vec<(String, bool)> {
    let mut runs: Vec<(String, bool)> = Vec::new();
    let mut strong = false;
    let mut text = String::new();
    let mut rest = point;
    let flush = |runs: &mut Vec<(String, bool)>, text: &mut String, strong: bool| {
        if !text.is_empty() {
            runs.push((std::mem::take(text), strong));
        }
    };
    while let Some(next) = rest.chars().next() {
        if rest.starts_with("**") || rest.starts_with("__") {
            flush(&mut runs, &mut text, strong);
            strong = !strong;
            rest = &rest[2..];
            continue;
        }
        match next {
            // Code is its text, as written.
            '`' if let Some(end) = rest[1..].find('`') => {
                text.push_str(&rest[1..=end]);
                rest = &rest[end + 2..];
            }
            '`' => rest = &rest[1..],
            // `[words](url)`: the words, where no other bracket comes
            // between.
            '[' if let Some(close) = rest.find("](")
                && !rest[1..close].contains(['[', ']'])
                && let Some(end) = rest[close..].find(')') =>
            {
                text.push_str(&rest[1..close]);
                rest = &rest[close + end + 1..];
            }
            _ => {
                text.push(next);
                rest = &rest[next.len_utf8()..];
            }
        }
    }
    flush(&mut runs, &mut text, strong);
    runs
}

/// The systems a release builds for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Os {
    Linux,
    MacOs,
    Windows,
    /// One no release builds for.
    Other,
}

impl Os {
    /// The one this runs on.
    pub fn current() -> Self {
        if cfg!(target_os = "linux") {
            Self::Linux
        } else if cfg!(target_os = "macos") {
            Self::MacOs
        } else if cfg!(windows) {
            Self::Windows
        } else {
            Self::Other
        }
    }
}

/// How this copy was installed, where it updates itself, which picks its
/// download.
pub use fastframe_update::Kind;

/// The name of the download a release has for `os` on this processor,
/// for a copy installed as `kind` (None while that is not known yet: the
/// archive). As `release.yml` names them, and as fastframe-update
/// downloads them.
pub fn asset_name(version: &str, os: Os, kind: Option<Kind>) -> Option<String> {
    asset_name_on(version, os, std::env::consts::ARCH, kind)
}

fn asset_name_on(version: &str, os: Os, arch: &str, kind: Option<Kind>) -> Option<String> {
    let arch = match arch {
        "x86_64" | "aarch64" => arch,
        _ if os == Os::MacOs => arch,
        _ => return None,
    };
    match (os, kind) {
        (Os::MacOs, _) => Some(format!("tabletist-v{version}-macos-universal.dmg")),
        (Os::Linux, Some(Kind::AppImage)) => Some(format!("tabletist-{version}-{arch}.AppImage")),
        (Os::Linux, _) => Some(format!(
            "tabletist-v{version}-{arch}-unknown-linux-gnu.tar.gz"
        )),
        (Os::Windows, Some(Kind::WindowsInstaller)) => Some(format!(
            "tabletist-v{version}-{arch}-pc-windows-msvc-setup.exe"
        )),
        (Os::Windows, _) => Some(format!("tabletist-v{version}-{arch}-pc-windows-msvc.zip")),
        (Os::Other, _) => None,
    }
}

/// A size in megabytes to a tenth, without its unit: `18.4`.
pub fn megabytes_figure(bytes: u64) -> String {
    format!("{:.1}", megabytes_value(bytes))
}

/// A size in megabytes: what a screen reader is told of a download.
pub fn megabytes_value(bytes: u64) -> f64 {
    // Told to a tenth: the precision of f64 is no matter here.
    #[allow(clippy::cast_precision_loss)]
    let megabytes = bytes as f64 / 1_000_000.0;
    megabytes
}

/// A size as the pane writes it: `31.2 MB`.
pub fn megabytes(bytes: u64) -> String {
    format!("{} MB", megabytes_figure(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn latest(tag: &str) -> Latest {
        Latest {
            tag_name: tag.into(),
            html_url: format!("https://github.com/igor-alexandrov/tabletist/releases/tag/{tag}"),
            body: Some("## What's new\n\n- **Edit rows** in the grid\n- one\n* two\n  - three\n- four\n- five".into()),
            published_at: Some("2026-10-24T03:16:13Z".into()),
            assets: vec![
                Asset {
                    name: "tabletist-v0.4.0-x86_64-unknown-linux-gnu.tar.gz".into(),
                    size: 31_200_000,
                },
                Asset {
                    name: "tabletist-v0.4.0-macos-universal.dmg".into(),
                    size: 41_000_000,
                },
            ],
            ..Latest::default()
        }
    }

    #[test]
    fn a_newer_tag_is_offered_and_an_equal_or_older_one_is_not() {
        let linux = |tag: &str, current: &str| found(&latest(tag), current, None, Os::Linux, None);
        assert!(matches!(linux("v0.4.0", "0.3.0"), Found::Newer(_)));
        // Without the `v` too.
        assert!(matches!(linux("0.4.0", "0.3.0"), Found::Newer(_)));
        assert_eq!(linux("v0.3.0", "0.3.0"), Found::UpToDate);
        assert_eq!(linux("v0.2.2", "0.3.0"), Found::UpToDate);
        assert!(matches!(linux("v0.10.0", "0.9.9"), Found::Newer(_)));
        // A tag that is no version.
        assert_eq!(linux("nightly", "0.3.0"), Found::UpToDate);
    }

    #[test]
    fn drafts_and_pre_releases_are_never_offered() {
        let mut draft = latest("v0.4.0");
        draft.draft = true;
        assert_eq!(
            found(&draft, "0.3.0", None, Os::Linux, None),
            Found::UpToDate
        );
        let mut pre = latest("v0.4.0");
        pre.prerelease = true;
        assert_eq!(found(&pre, "0.3.0", None, Os::Linux, None), Found::UpToDate);
        // A pre-release's tag, unflagged, is no newer stable version.
        assert_eq!(
            found(&latest("v0.4.0-rc.1"), "0.3.0", None, Os::Linux, None),
            Found::UpToDate
        );
    }

    #[test]
    fn a_skipped_version_is_not_offered_and_a_newer_one_is() {
        let skip = Some("0.4.0");
        assert_eq!(
            found(&latest("v0.4.0"), "0.3.0", skip, Os::Linux, None),
            Found::UpToDate
        );
        assert!(matches!(
            found(&latest("v0.4.1"), "0.3.0", skip, Os::Linux, None),
            Found::Newer(_)
        ));
    }

    #[test]
    fn an_offer_has_the_first_four_points_its_date_and_this_computers_download() {
        let Found::Newer(offer) = found(&latest("v0.4.0"), "0.3.0", None, Os::Linux, None) else {
            panic!("not offered");
        };
        assert_eq!(offer.version, "0.4.0");
        assert_eq!(
            offer.notes,
            ["**Edit rows** in the grid", "one", "two", "three"]
        );
        assert_eq!(offer.published, Some(1_792_811_773));
        if std::env::consts::ARCH == "x86_64" {
            assert_eq!(
                offer.download,
                Some((
                    "tabletist-v0.4.0-x86_64-unknown-linux-gnu.tar.gz".into(),
                    31_200_000
                ))
            );
        }
        let mac = offer_on_mac();
        assert_eq!(
            mac.download,
            Some(("tabletist-v0.4.0-macos-universal.dmg".into(), 41_000_000))
        );
        // Windows has no build in this release.
        assert_eq!(
            super::offer(&latest("v0.4.0"), Os::Windows, None).download,
            None
        );
    }

    fn offer_on_mac() -> Offer {
        super::offer(&latest("v0.4.0"), Os::MacOs, Some(Kind::MacBundle))
    }

    #[test]
    fn each_system_and_processor_takes_its_own_download() {
        let name = |os, arch, kind| asset_name_on("0.4.0", os, arch, kind);
        assert_eq!(
            name(Os::Linux, "x86_64", Some(Kind::Portable)).as_deref(),
            Some("tabletist-v0.4.0-x86_64-unknown-linux-gnu.tar.gz")
        );
        assert_eq!(
            name(Os::Linux, "aarch64", None).as_deref(),
            Some("tabletist-v0.4.0-aarch64-unknown-linux-gnu.tar.gz")
        );
        assert_eq!(
            name(Os::Linux, "x86_64", Some(Kind::AppImage)).as_deref(),
            Some("tabletist-0.4.0-x86_64.AppImage")
        );
        // One universal disk image, whichever chip.
        for arch in ["aarch64", "x86_64"] {
            assert_eq!(
                name(Os::MacOs, arch, Some(Kind::MacBundle)).as_deref(),
                Some("tabletist-v0.4.0-macos-universal.dmg")
            );
        }
        assert_eq!(
            name(Os::Windows, "x86_64", Some(Kind::WindowsInstaller)).as_deref(),
            Some("tabletist-v0.4.0-x86_64-pc-windows-msvc-setup.exe")
        );
        assert_eq!(
            name(Os::Windows, "aarch64", Some(Kind::Portable)).as_deref(),
            Some("tabletist-v0.4.0-aarch64-pc-windows-msvc.zip")
        );
        // No release builds for these.
        assert_eq!(name(Os::Linux, "riscv64", None), None);
        assert_eq!(name(Os::Other, "x86_64", None), None);
    }

    #[test]
    fn a_point_is_read_in_runs_of_strong_and_plain_text() {
        assert_eq!(
            runs("**Edit rows** in the grid, `NULL` and [the docs](https://x)"),
            [
                ("Edit rows".to_owned(), true),
                (" in the grid, NULL and the docs".to_owned(), false),
            ]
        );
        assert_eq!(runs("plain"), [("plain".to_owned(), false)]);
    }

    #[test]
    fn a_size_is_written_in_megabytes_to_a_tenth() {
        assert_eq!(megabytes_figure(18_400_000), "18.4");
        assert_eq!(megabytes(31_200_000), "31.2 MB");
        assert_eq!(megabytes(29_849_999), "29.8 MB");
    }

    /// A bracket before a link is kept as written, and the link's words
    /// are its own.
    #[test]
    fn a_bracket_before_a_link_is_kept_as_written() {
        let text: String = runs("[breaking] Rename the key ([#12](https://x/12))")
            .into_iter()
            .map(|(run, _)| run)
            .collect();
        assert_eq!(text, "[breaking] Rename the key (#12)");
    }

    /// Code is its text: `__init__` in backticks is not strong.
    #[test]
    fn code_is_its_text() {
        assert_eq!(
            runs("Fix `__init__` handling"),
            [("Fix __init__ handling".to_owned(), false)]
        );
    }
}

//! Small file helpers shared by the settings and connection stores.

use std::io::Write;
use std::path::{Path, PathBuf};

use serde::Serialize;
use serde::de::DeserializeOwned;

/// Seconds since the Unix epoch. Tests can pin it (`pin_now`) so dates
/// the interface prints ("yesterday", "Sep 12") stay the same.
pub fn now_secs() -> u64 {
    #[cfg(test)]
    if let Some(now) = PINNED_NOW.with(std::cell::Cell::get) {
        return now;
    }
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_secs())
}

#[cfg(test)]
thread_local! {
    static PINNED_NOW: std::cell::Cell<Option<u64>> = const { std::cell::Cell::new(None) };
}

/// Pins [`now_secs`] on this thread (`None` lets the clock run again).
#[cfg(test)]
pub fn pin_now(now: Option<u64>) {
    PINNED_NOW.with(|pinned| pinned.set(now));
}

/// Makes a symbolic link at `link` to the file `target`, for the tests.
#[cfg(test)]
pub fn symlink_file(target: impl AsRef<Path>, link: impl AsRef<Path>) {
    #[cfg(unix)]
    std::os::unix::fs::symlink(target, link).unwrap();
    #[cfg(windows)]
    std::os::windows::fs::symlink_file(target, link).unwrap();
}

/// Whether this user may make symbolic links, for the tests of them:
/// always on Unix, on Windows only with the privilege or in developer mode.
/// Without it such a test prints "skipped" and passes, as the tests that
/// need a server do.
#[cfg(test)]
pub fn can_symlink() -> bool {
    #[cfg(windows)]
    {
        let dir = tempfile::tempdir().unwrap();
        let (target, link) = (dir.path().join("target"), dir.path().join("link"));
        if let Err(error) = std::os::windows::fs::symlink_file(target, link) {
            eprintln!("skipped: this user cannot make symbolic links ({error})");
            return false;
        }
    }
    true
}

/// `path` with `suffix` appended to its file name (`a.json` -> `a.json.tmp`).
fn with_suffix(path: &Path, suffix: &str) -> PathBuf {
    let mut name = path.as_os_str().to_owned();
    name.push(suffix);
    PathBuf::from(name)
}

/// Creates `dir` and any missing parents. On Unix, directories it creates are
/// private to the user (0700); existing ones keep their permissions.
pub fn create_private_dir(dir: &Path) -> std::io::Result<()> {
    let mut builder = std::fs::DirBuilder::new();
    builder.recursive(true);
    #[cfg(unix)]
    std::os::unix::fs::DirBuilderExt::mode(&mut builder, 0o700);
    builder.create(dir)
}

/// The directory `path` is in: the current one for a bare file name.
pub fn directory_of(path: &Path) -> &Path {
    match path.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent,
        _ => Path::new("."),
    }
}

/// How many symbolic links [`resolve_link`] follows before it gives up, as
/// Linux does.
const MAX_LINKS: usize = 40;

/// The paths `path` leads through as the symbolic links at its end are
/// followed (a dotfiles manager keeps the file in its repository and a
/// link here): `path` first, then what each link leads to, the file last.
/// Only `path` itself when it is not a link. The file need not be there.
/// Links among the directories on the way are the system's to follow. More
/// links than the limit, as links that lead back to themselves are, is an
/// error, and so is a path that cannot be looked at: it is not taken for a
/// plain file.
pub fn link_chain(path: &Path) -> std::io::Result<Vec<PathBuf>> {
    let mut chain = vec![path.to_path_buf()];
    loop {
        let file = &chain[chain.len() - 1];
        let linked = match file.symlink_metadata() {
            Ok(metadata) => metadata.file_type().is_symlink(),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => false,
            Err(error) => return Err(error),
        };
        if !linked {
            return Ok(chain);
        }
        if chain.len() > MAX_LINKS {
            return Err(std::io::Error::other(format!(
                "{} leads through more than {MAX_LINKS} symbolic links",
                path.display()
            )));
        }
        // A relative target starts at the link's directory; an absolute
        // one replaces it.
        let target = std::fs::read_link(file)?;
        let next = directory_of(file).join(target);
        chain.push(next);
    }
}

/// The file `path` names once the symbolic links at its end are followed:
/// the last of its [`link_chain`], `path` itself when it is not a link.
pub fn resolve_link(path: &Path) -> std::io::Result<PathBuf> {
    let file = link_chain(path)?.pop();
    Ok(file.unwrap_or_else(|| path.to_path_buf()))
}

/// Writes `bytes` to a new temporary file beside `path`, flushes it to disk,
/// then renames it over `path`, so a crash never leaves a half-written file.
///
/// When `path` is a symbolic link the link stays: the temporary file is
/// made beside the file it leads to (so the rename stays within one
/// directory) and replaces that file. A rename over the link would leave a
/// plain file there, and the file the user keeps elsewhere would no longer
/// be the one the app uses. The directory is created only for a path that
/// is not a link: where a link leads is the user's to make, and a save
/// through a link to a directory that is not there fails.
///
/// The temporary file has a random name and is created exclusively, so a
/// leftover file or a symlink planted beside the file is never followed:
/// the only link written through is one at `path` itself. On Unix the new
/// file is private to the user (0600) wherever it is, and its directory is
/// flushed after the rename so the new name survives a crash too.
pub fn write_atomic(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let destination = resolve_link(path)?;
    let dir = directory_of(&destination);
    if destination == path {
        create_private_dir(dir)?;
    }
    let mut file = tempfile::Builder::new()
        .prefix(".tabletist-")
        .suffix(".tmp")
        .tempfile_in(dir)?;
    file.write_all(bytes)?;
    file.as_file().sync_all()?;
    file.persist(&destination).map_err(|error| error.error)?;
    #[cfg(unix)]
    {
        // Best effort: some filesystems cannot sync a directory, and the
        // file itself is already in place.
        if let Ok(dir) = std::fs::File::open(dir) {
            let _ = dir.sync_all();
        }
    }
    Ok(())
}

/// Saves `value` as pretty JSON, atomically.
pub fn save_json<T: Serialize>(path: &Path, value: &T) -> std::io::Result<()> {
    let bytes = serde_json::to_vec_pretty(value).map_err(std::io::Error::other)?;
    write_atomic(path, &bytes)
}

/// Loads JSON from `path`. A missing file gives the default. A file that does
/// not parse is renamed to `<name>.bad` (so the user's data is not lost and the
/// next save does not overwrite it) and the default is used.
pub fn load_json<T: DeserializeOwned + Default>(path: &Path) -> T {
    let bytes = match std::fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return T::default(),
        Err(error) => {
            // An unreadable file (permissions, I/O error) is kept aside just
            // like a damaged one, so the next save cannot replace it.
            keep_aside(path, &error.to_string());
            return T::default();
        }
    };
    match serde_json::from_slice(&bytes) {
        Ok(value) => value,
        Err(error) => {
            keep_aside(path, &error.to_string());
            T::default()
        }
    }
}

/// Renames a file that could not be loaded to `<name>.bad`, or to
/// `<name>.bad.1`, `<name>.bad.2`... when earlier ones exist, so a second
/// failure never replaces the first copy. Behind a symbolic link it is the
/// file the link leads to that is renamed, beside itself: the link stays,
/// and the next save writes a new file where it leads.
pub fn keep_aside(path: &Path, problem: &str) {
    // Links without an end have no file to move: the link itself goes.
    let file = resolve_link(path).unwrap_or_else(|_| path.to_path_buf());
    let path = file.as_path();
    let Some(aside) = (0..1000)
        .map(|n| match n {
            0 => with_suffix(path, ".bad"),
            n => with_suffix(path, &format!(".bad.{n}")),
        })
        // symlink_metadata also sees a dangling symlink, which rename would replace.
        .find(|candidate| candidate.symlink_metadata().is_err())
    else {
        log::warn!(
            "could not load {} ({problem}) and every .bad name is taken; using defaults",
            path.display()
        );
        return;
    };
    log::warn!(
        "could not load {} ({problem}); keeping it as {} and using defaults",
        path.display(),
        aside.display()
    );
    if let Err(error) = std::fs::rename(path, &aside) {
        log::warn!("could not keep {} aside: {error}", path.display());
    }
}

/// How well `candidate` matches `query` typed in quick open (ignoring
/// case), best first by tier: the exact name, a prefix, a whole run at a
/// word start, a run anywhere, then a scattered subsequence (runs and word
/// starts score higher there). Within a tier, shorter names win. `None`
/// when the query's characters do not all appear in order.
pub fn fuzzy_score(query: &str, candidate: &str) -> Option<i64> {
    const TIER: i64 = 1_000_000;
    let query: String = query
        .to_lowercase()
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect();
    if query.is_empty() {
        return Some(0);
    }
    let text = candidate.to_lowercase();
    let length = text.chars().count().min(10_000) as i64;
    let word_start = |at: usize| {
        at == 0
            || text[..at]
                .chars()
                .next_back()
                .is_some_and(|c| matches!(c, '.' | '_' | '-' | ' '))
    };
    let tier = if text == query {
        4
    } else if text.starts_with(&query) {
        3
    } else if text.match_indices(&query).any(|(at, _)| word_start(at)) {
        2
    } else if text.contains(&query) {
        1
    } else {
        0
    };
    if tier > 0 {
        return Some(tier * TIER - length);
    }
    // Scattered: every character in order, greedily.
    let query: Vec<char> = query.chars().collect();
    let chars: Vec<char> = text.chars().collect();
    let mut score = 0i64;
    let mut next = 0;
    let mut previous: Option<usize> = None;
    for (index, &c) in chars.iter().enumerate() {
        if next == query.len() {
            break;
        }
        if c != query[next] {
            continue;
        }
        score += 10;
        if previous.is_some_and(|p| p + 1 == index) {
            score += 15;
        }
        if index == 0 || matches!(chars[index - 1], '.' | '_' | '-' | ' ') {
            score += 20;
        }
        previous = Some(index);
        next += 1;
    }
    (next == query.len()).then(|| score.min(TIER / 2) - length)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
    #[serde(default)]
    struct Sample {
        name: String,
        count: u32,
    }

    #[test]
    fn json_round_trips_through_an_atomic_write() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nested").join("sample.json");
        let value = Sample {
            name: "a".into(),
            count: 3,
        };
        save_json(&path, &value).unwrap();
        assert_eq!(load_json::<Sample>(&path), value);
        let names: Vec<_> = std::fs::read_dir(dir.path().join("nested"))
            .unwrap()
            .map(|entry| entry.unwrap().file_name())
            .collect();
        assert_eq!(names, ["sample.json"], "no temporary file is left behind");
    }

    #[cfg(unix)]
    #[test]
    fn atomic_writes_are_private_and_never_follow_a_planted_symlink() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("config");
        let path = config.join("sample.json");
        write_atomic(&path, b"first").unwrap();
        let mode = |path: &Path| std::fs::metadata(path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode(&config), 0o700, "a new config directory is private");
        assert_eq!(mode(&path), 0o600);

        // A world-readable file is replaced by a private one.
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).unwrap();
        // The old fixed temporary name, pointing somewhere else.
        let victim = dir.path().join("victim");
        std::fs::write(&victim, b"untouched").unwrap();
        std::os::unix::fs::symlink(&victim, config.join("sample.json.tmp")).unwrap();

        write_atomic(&path, b"second").unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), b"second");
        assert_eq!(mode(&path), 0o600);
        assert_eq!(std::fs::read(&victim).unwrap(), b"untouched");
    }

    /// The names in `dir`, sorted.
    fn names_in(dir: &Path) -> Vec<String> {
        let mut names: Vec<_> = std::fs::read_dir(dir)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        names
    }

    #[test]
    fn a_write_through_a_symlink_replaces_the_file_it_leads_to() {
        if !can_symlink() {
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        let (config, dotfiles) = (dir.path().join("config"), dir.path().join("dotfiles"));
        std::fs::create_dir_all(&config).unwrap();
        std::fs::create_dir_all(&dotfiles).unwrap();
        let (link, target) = (config.join("sample.json"), dotfiles.join("tabletist.json"));
        std::fs::write(&target, b"first").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&target, std::fs::Permissions::from_mode(0o644)).unwrap();
        }
        symlink_file(&target, &link);
        // A link planted under a temporary name beside the target.
        let victim = dir.path().join("victim");
        std::fs::write(&victim, b"untouched").unwrap();
        symlink_file(&victim, dotfiles.join("tabletist.json.tmp"));

        write_atomic(&link, b"second").unwrap();
        assert_eq!(std::fs::read_link(&link).unwrap(), target, "the link stays");
        assert!(!target.symlink_metadata().unwrap().file_type().is_symlink());
        assert_eq!(std::fs::read(&target).unwrap(), b"second");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(&target).unwrap().permissions().mode() & 0o777;
            assert_eq!(mode, 0o600, "the new file is private there too");
        }
        assert_eq!(std::fs::read(&victim).unwrap(), b"untouched");
        assert_eq!(names_in(&config), ["sample.json"]);
        assert_eq!(
            names_in(&dotfiles),
            ["tabletist.json", "tabletist.json.tmp"],
            "no temporary file is left behind"
        );
    }

    #[test]
    fn a_write_follows_relative_links_one_after_another() {
        if !can_symlink() {
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        let (config, dotfiles) = (dir.path().join("config"), dir.path().join("dotfiles"));
        std::fs::create_dir_all(&config).unwrap();
        std::fs::create_dir_all(&dotfiles).unwrap();
        // config/sample.json -> ../dotfiles/sample.json -> real.json
        let link = config.join("sample.json");
        let beside = Path::new("..").join("dotfiles");
        symlink_file(beside.join("sample.json"), &link);
        symlink_file("real.json", dotfiles.join("sample.json"));
        assert_eq!(
            resolve_link(&link).unwrap(),
            config.join(&beside).join("real.json")
        );
        assert_eq!(resolve_link(&config).unwrap(), config, "not a link");

        // The last link leads to a file that is not there yet: it is made.
        write_atomic(&link, b"first").unwrap();
        write_atomic(&link, b"second").unwrap();
        assert_eq!(
            std::fs::read(dotfiles.join("real.json")).unwrap(),
            b"second"
        );
        assert_eq!(names_in(&config), ["sample.json"]);
        assert_eq!(names_in(&dotfiles), ["real.json", "sample.json"]);
        for (link, target) in [
            (link, beside.join("sample.json")),
            (dotfiles.join("sample.json"), "real.json".into()),
        ] {
            assert_eq!(std::fs::read_link(&link).unwrap(), target);
        }
    }

    #[test]
    fn a_write_through_a_link_to_nowhere_fails_and_keeps_the_link() {
        if !can_symlink() {
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        // The directory the link leads to is not there (a volume that is
        // not mounted): it is not the app's to make.
        let link = dir.path().join("sample.json");
        let target = dir.path().join("away").join("sample.json");
        symlink_file(&target, &link);
        assert!(write_atomic(&link, b"lost").is_err());
        assert_eq!(std::fs::read_link(&link).unwrap(), target);
        assert_eq!(names_in(dir.path()), ["sample.json"]);

        // Two links that lead to each other.
        let (one, other) = (dir.path().join("one.json"), dir.path().join("other.json"));
        symlink_file(&other, &one);
        symlink_file(&one, &other);
        assert!(resolve_link(&one).is_err());
        assert!(write_atomic(&one, b"lost").is_err());
        assert_eq!(std::fs::read_link(&one).unwrap(), other);
        assert_eq!(std::fs::read_link(&other).unwrap(), one);
    }

    #[test]
    fn forty_links_are_followed_and_one_more_is_not() {
        if !can_symlink() {
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        let link = |n: usize| dir.path().join(format!("link-{n}"));
        // link-1 -> link-2 -> ... -> link-40 -> file: forty links.
        let file = dir.path().join("file");
        std::fs::write(&file, b"first").unwrap();
        symlink_file("file", link(MAX_LINKS));
        for n in (1..MAX_LINKS).rev() {
            symlink_file(format!("link-{}", n + 1), link(n));
        }
        assert_eq!(resolve_link(&link(1)).unwrap(), file);
        write_atomic(&link(1), b"second").unwrap();
        assert_eq!(std::fs::read(&file).unwrap(), b"second");
        // One more is one too many.
        symlink_file("link-1", link(0));
        let error = resolve_link(&link(0)).unwrap_err().to_string();
        assert!(
            error.ends_with("leads through more than 40 symbolic links"),
            "{error}"
        );
        assert!(write_atomic(&link(0), b"lost").is_err());
        assert_eq!(std::fs::read(&file).unwrap(), b"second");
    }

    #[cfg(unix)]
    #[test]
    fn a_path_that_cannot_be_looked_at_is_not_taken_for_a_plain_file() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let closed = dir.path().join("closed");
        std::fs::create_dir(&closed).unwrap();
        let path = closed.join("sample.json");
        let allow = |mode| {
            std::fs::set_permissions(&closed, std::fs::Permissions::from_mode(mode)).unwrap()
        };
        // Nothing in the directory may be looked at, a link there neither.
        allow(0o000);
        if closed.read_dir().is_ok() {
            return allow(0o700); // running as root: permissions do not apply
        }
        let error = resolve_link(&path).unwrap_err();
        assert_eq!(error.kind(), std::io::ErrorKind::PermissionDenied);
        assert!(write_atomic(&path, b"lost").is_err());
        allow(0o700);
        // A path with nothing at it is no error: it is where a file is made.
        assert_eq!(resolve_link(&path).unwrap(), path);
        assert_eq!(link_chain(&path).unwrap(), [path]);
    }

    #[test]
    fn a_damaged_file_behind_a_symlink_is_kept_aside_beside_itself() {
        if !can_symlink() {
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        let (config, dotfiles) = (dir.path().join("config"), dir.path().join("dotfiles"));
        std::fs::create_dir_all(&config).unwrap();
        std::fs::create_dir_all(&dotfiles).unwrap();
        let (link, target) = (config.join("sample.json"), dotfiles.join("sample.json"));
        std::fs::write(&target, b"{\"name\": ").unwrap();
        symlink_file(&target, &link);

        assert_eq!(load_json::<Sample>(&link), Sample::default());
        assert_eq!(names_in(&config), ["sample.json"], "the link stays");
        assert_eq!(names_in(&dotfiles), ["sample.json.bad"]);
        // The next save writes a new file where the link leads.
        let value = Sample {
            name: "a".into(),
            count: 3,
        };
        save_json(&link, &value).unwrap();
        assert_eq!(std::fs::read_link(&link).unwrap(), target);
        assert_eq!(load_json::<Sample>(&target), value);
        assert_eq!(names_in(&dotfiles), ["sample.json", "sample.json.bad"]);
    }

    #[test]
    fn a_missing_file_loads_the_default() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(
            load_json::<Sample>(&dir.path().join("none.json")),
            Sample::default()
        );
    }

    #[test]
    fn a_damaged_file_is_kept_aside_and_the_default_used() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sample.json");
        std::fs::write(&path, b"{\"name\": ").unwrap();
        assert_eq!(load_json::<Sample>(&path), Sample::default());
        assert!(!path.exists());
        assert_eq!(
            std::fs::read(dir.path().join("sample.json.bad")).unwrap(),
            b"{\"name\": "
        );
    }

    #[test]
    fn repeated_failures_never_overwrite_an_earlier_copy() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sample.json");
        for attempt in ["first", "second", "third"] {
            std::fs::write(&path, attempt).unwrap();
            assert_eq!(load_json::<Sample>(&path), Sample::default());
        }
        let read = |name: &str| std::fs::read_to_string(dir.path().join(name)).unwrap();
        assert_eq!(read("sample.json.bad"), "first");
        assert_eq!(read("sample.json.bad.1"), "second");
        assert_eq!(read("sample.json.bad.2"), "third");
    }

    #[cfg(unix)]
    #[test]
    fn an_unreadable_file_is_kept_aside_so_a_save_cannot_overwrite_it() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sample.json");
        std::fs::write(&path, br#"{"name": "precious", "count": 3}"#).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o000)).unwrap();
        if std::fs::read(&path).is_ok() {
            return; // running as root: permissions do not apply
        }
        assert_eq!(load_json::<Sample>(&path), Sample::default());
        assert!(
            !path.exists(),
            "the unreadable file must not stay where a save would replace it"
        );
        assert!(dir.path().join("sample.json.bad").exists());
    }

    #[test]
    fn fuzzy_scores_prefer_prefixes_and_runs() {
        assert!(fuzzy_score("usr", "users").is_some());
        assert!(fuzzy_score("xyz", "users").is_none());
        assert_eq!(fuzzy_score("", "users"), Some(0));
        let prefix = fuzzy_score("ord", "orders").unwrap();
        let scattered = fuzzy_score("ord", "product_reviews_old").unwrap();
        assert!(prefix > scattered, "{prefix} vs {scattered}");
        let word = fuzzy_score("items", "order_items").unwrap();
        let inside = fuzzy_score("items", "subitemset").unwrap();
        assert!(word > inside, "{word} vs {inside}");
        assert!(fuzzy_score("USERS", "users").is_some(), "case-insensitive");
    }

    #[test]
    fn fuzzy_tiers_beat_length_and_scatter() {
        let score = |query, candidate| fuzzy_score(query, candidate).unwrap();
        // A prefix beats a word-start scatter, however long the prefix name.
        assert!(
            score("user", "user_notification_preferences_history")
                > score("user", "used_resources")
        );
        assert!(
            score("ord", "orders_with_a_much_longer_name_than_usual_x")
                > score("ord", "product_reviews_old")
        );
        // A whole word inside the name beats a scattered one.
        assert!(score("rev", "orders_reviews") > score("rev", "row_event_view"));
        // The exact name comes first.
        assert!(score("items", "items") > score("items", "i_t_e_m_s"));
        assert!(score("items", "items") > score("items", "items_archive"));
    }
}

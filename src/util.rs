//! Small file helpers shared by the settings and connection stores.

use std::io::Write;
use std::path::{Path, PathBuf};

use serde::Serialize;
use serde::de::DeserializeOwned;

/// `path` with `suffix` appended to its file name (`a.json` -> `a.json.tmp`).
fn with_suffix(path: &Path, suffix: &str) -> PathBuf {
    let mut name = path.as_os_str().to_owned();
    name.push(suffix);
    PathBuf::from(name)
}

/// Writes `bytes` to a temporary file beside `path`, flushes it to disk, then
/// renames it over `path`, so a crash never leaves a half-written file.
/// `std::fs::rename` replaces an existing file on every platform.
pub fn write_atomic(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let temporary = with_suffix(path, ".tmp");
    {
        let mut file = std::fs::File::create(&temporary)?;
        file.write_all(bytes)?;
        file.sync_all()?;
    }
    std::fs::rename(&temporary, path)
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

/// Renames a file that could not be loaded to `<name>.bad`.
fn keep_aside(path: &Path, problem: &str) {
    let aside = with_suffix(path, ".bad");
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
        assert!(!dir.path().join("nested").join("sample.json.tmp").exists());
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

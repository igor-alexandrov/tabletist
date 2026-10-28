//! What the checks found, kept in the state folder (`updates.json`), not
//! in the settings: when the last one that worked ran, the newest version
//! seen and its notes, and when the status line last told of it.

use serde::{Deserialize, Serialize};

use super::release::Offer;

/// The file's contents. Every field may be missing: a file of an older or
/// a newer version reads as far as it goes.
#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
pub struct Seen {
    /// When the last check that got an answer ran, in seconds since the
    /// Unix epoch. A failed check leaves it.
    #[serde(default)]
    pub last_checked: Option<u64>,
    /// The newest version a check found, newer than the version that ran
    /// then or not.
    #[serde(default)]
    pub latest_seen: Option<String>,
    /// That release, with its notes, while it is newer than this version.
    #[serde(default)]
    pub offer: Option<Offer>,
    /// When the terminal look's status line last said a version is
    /// available: at most once a day.
    #[serde(default)]
    pub announced: Option<u64>,
}

/// How often the status line tells of a version available, at most.
pub const ANNOUNCE_EVERY: u64 = 24 * 60 * 60;

impl Seen {
    /// Reads `path`. A missing file is no check yet; one that does not
    /// parse is kept aside, as every state file is. A time no clock can
    /// show (a file written by hand) is left out.
    pub fn load(path: &std::path::Path) -> Self {
        let mut seen: Self = crate::util::load_json(path);
        let shown =
            |at: &u64| i64::try_from(*at).is_ok_and(|at| jiff::Timestamp::from_second(at).is_ok());
        seen.last_checked = seen.last_checked.filter(shown);
        if let Some(offer) = &mut seen.offer {
            offer.published = offer.published.filter(shown);
        }
        seen
    }

    /// The release to offer, when the file holds one newer than `current`.
    pub fn offered(&self, current: &str) -> Option<&Offer> {
        self.offer
            .as_ref()
            .filter(|offer| fastframe_update::is_newer(&offer.version, current))
    }

    /// Whether the status line may tell of a version at `now`. A time
    /// after `now` (a clock that was ahead) does not hold it back.
    pub fn may_announce(&self, now: u64) -> bool {
        self.announced
            .is_none_or(|then| then > now || now - then >= ANNOUNCE_EVERY)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_offer_older_than_this_version_is_not_offered_again() {
        let seen = Seen {
            offer: Some(Offer {
                version: "0.4.0".into(),
                ..Offer::default()
            }),
            ..Seen::default()
        };
        assert!(seen.offered("0.3.0").is_some());
        assert!(seen.offered("0.4.0").is_none());
    }

    #[test]
    fn the_file_reads_as_far_as_it_goes() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("updates.json");
        std::fs::write(&path, r#"{"last_checked": 1792811773, "later": true}"#).unwrap();
        let seen = Seen::load(&path);
        assert_eq!(seen.last_checked, Some(1_792_811_773));
        assert_eq!(seen.offer, None);
        // No file: no check yet.
        assert_eq!(
            Seen::load(&dir.path().join("missing.json")),
            Seen::default()
        );
    }

    #[test]
    fn a_time_no_clock_can_show_is_left_out() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("updates.json");
        std::fs::write(
            &path,
            r#"{"last_checked": 18446744073709551615, "offer": {"version": "9.9.9", "url": "https://github.com/igor-alexandrov/tabletist/releases", "published": 253402300800}}"#,
        )
        .unwrap();
        let seen = Seen::load(&path);
        assert_eq!(seen.last_checked, None);
        let offer = seen.offer.expect("the offer is kept");
        assert_eq!(offer.version, "9.9.9");
        assert_eq!(offer.published, None);
    }

    #[test]
    fn the_status_line_tells_at_most_once_a_day() {
        let seen = Seen {
            announced: Some(1_000),
            ..Seen::default()
        };
        assert!(!seen.may_announce(1_000 + ANNOUNCE_EVERY - 1));
        assert!(seen.may_announce(1_000 + ANNOUNCE_EVERY));
        assert!(Seen::default().may_announce(0));
    }
}

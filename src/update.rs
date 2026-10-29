//! Updates from Tabletist's GitHub releases: the check Settings › Updates
//! shows (`check`, `release`), what it found as the state folder keeps it
//! (`seen`), how this copy was installed (`installed`), and the updater
//! that installs one, fastframe-update, over a client of our own
//! (`transport`), with what the card makes of its download (`download`).
//!
//! What a later release's helper relies on from this build, which must not
//! change: `tabletist --version` prints `tabletist <version>`;
//! `--update-receipt <job>` and `--update-error <message>` are taken out
//! before clap reads the command line ([`fastframe_update::intercept`] in
//! `entrypoint::run`); the receipt is confirmed once the window is up; and
//! `--relaunch`, which the helper adds to the old copy's arguments, is
//! read.
//! The release side of the contract (asset names, markers, the signed
//! checksums) is in `release.yml` and `tests/update_release.rs`.

pub mod check;
pub mod download;
pub mod installed;
pub mod relaunch;
pub mod release;
pub mod seen;
pub mod transport;

use std::time::{Duration, Instant};

use fastframe_update::{MacConfig, UpdateConfig, Updater};

pub use check::Failed;
pub use download::Failure;
pub use installed::{Installed, Managed};
pub use release::{Found, Offer};
pub use seen::Seen;

use crate::settings::UpdateCheck;

/// The version that runs.
pub const CURRENT: &str = env!("CARGO_PKG_VERSION");

/// What the updater knows about Tabletist.
pub const CONFIG: UpdateConfig = UpdateConfig {
    macos: MacConfig {
        bundle_ids: &["dev.tabletist.Tabletist"],
        // The slug: Contents/MacOS/tabletist.
        executable_names: &[],
        legacy_bundle_names: &[],
    },
    // Releases sign checksums.txt (packaging/sign-checksums.sh); nothing is
    // downloaded from a release whose signature does not verify.
    publisher_key: Some(include_str!("../assets/update-public-key.hex")),
    // Releases list tabletist-<version>-<arch>.AppImage in the signed
    // checksums, so an AppImage replaces itself with the next one.
    appimage: true,
    ..UpdateConfig::new(
        "igor-alexandrov/tabletist",
        "Tabletist",
        "tabletist",
        CURRENT,
    )
};

/// An updater, on an HTTP client of its own. Its calls block: run them on
/// a thread of their own, never on the interface thread.
pub fn updater() -> Updater {
    Updater::new(CONFIG, transport::UreqTransport::new())
}

/// The release page of the version that runs: What's new in it.
pub fn this_release() -> String {
    release_page(CURRENT)
}

/// The release page of `version`.
pub fn release_page(version: &str) -> String {
    format!("https://github.com/igor-alexandrov/tabletist/releases/tag/v{version}")
}

/// Whether this build downloads updates: not on Windows, whose copies
/// get the release page.
pub const DOWNLOADS: bool = !cfg!(windows);

/// How long `check` waits between scheduled checks; None where checks
/// are only asked for.
pub fn interval(check: UpdateCheck) -> Option<u64> {
    match check {
        UpdateCheck::Daily => Some(24 * 60 * 60),
        UpdateCheck::Weekly => Some(7 * 24 * 60 * 60),
        UpdateCheck::Never => None,
    }
}

/// How long from `now` until the next scheduled check, in seconds: 0
/// when it is due. An interval from the last check that worked, or from
/// the last that failed where that came later. None with `"never"`. A
/// time after `now` (a clock that was ahead) is left out: with no other,
/// the check is due, and its answer writes the time it ran.
pub fn due(check: UpdateCheck, seen: &Seen, tried: Option<u64>, now: u64) -> Option<u64> {
    let every = interval(check)?;
    let since = [seen.last_checked, tried]
        .into_iter()
        .flatten()
        .filter(|since| *since <= now)
        .max();
    Some(since.map_or(0, |since| (since + every).saturating_sub(now)))
}

/// Where the check stands while the app runs.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum Phase {
    /// Nothing asked.
    #[default]
    Idle,
    /// Asked, and not answered yet: answers to any other ask are dropped.
    Checking {
        asked: u64,
        /// Whether the person asked for it, which offers a version they
        /// said to skip.
        asked_for: bool,
    },
    /// The last check found nothing, for this reason. One the person
    /// asked for is shown over a version found before.
    Failed { failed: Failed, asked_for: bool },
}

/// Where a download, and then its install, stands while the app runs.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum Install {
    /// No download asked for, or the last was cancelled.
    #[default]
    Idle,
    /// Downloading, then verifying once the last byte came.
    Downloading(Downloading),
    /// Downloaded and verified: the backend holds it for Restart now.
    /// Later keeps it so; a quit drops it. Or, `handed`, given to the
    /// helper already, which then installs it at the next quit within
    /// [`HELPER_WAITS`] of that time.
    Ready {
        version: String,
        handed: Option<Instant>,
    },
    /// Restart now found work open, which the card lists. `when_done`:
    /// Restart when done was chosen, and the restart goes ahead once
    /// nothing is listed. `handed` as for `Ready`.
    Held {
        version: String,
        when_done: bool,
        handed: Option<Instant>,
    },
    /// Handed to the helper: the app quits once it stands ready.
    Restarting { version: String },
    /// The download gave nothing to install.
    Failed { version: String, failure: Failure },
}

/// A download under way.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Downloading {
    /// Answers to any other ask are dropped.
    pub asked: u64,
    pub version: String,
    pub received: u64,
    /// The file's size, as its release says.
    pub total: u64,
    /// It waits for the network to come back.
    pub waiting: bool,
    /// Its progress lately, for the time left: the time since it started,
    /// and the bytes it had then.
    pub samples: Vec<(Duration, u64)>,
    /// Restart once it is ready: `:update`'s yes.
    pub then_restart: bool,
}

impl Downloading {
    pub fn new(asked: u64, version: String, total: u64) -> Self {
        Self {
            asked,
            version,
            received: 0,
            total,
            waiting: false,
            samples: vec![(Duration::ZERO, 0)],
            then_restart: false,
        }
    }

    /// The last byte came: fastframe-update checks the file now.
    pub fn verifying(&self) -> bool {
        self.total > 0 && self.received >= self.total
    }

    /// How long it has left, where that can be told.
    pub fn time_left(&self) -> Option<Duration> {
        download::time_left(&self.samples, self.total)
    }

    /// Notes `received` of `total`, `after` this long.
    pub fn progressed(&mut self, received: u64, total: u64, after: Duration) {
        self.received = received;
        self.total = total;
        self.samples.push((after, received));
        download::keep_window(&mut self.samples);
    }
}

/// How long fastframe-update's helper waits for the app to quit once it
/// stands ready, a little short of its minute: after that it has given up,
/// and nothing installs.
pub const HELPER_WAITS: Duration = Duration::from_secs(55);

/// How late a quit may still begin after the helper said it stands
/// ready: the app takes a few seconds to go.
pub const QUIT_IN_TIME: Duration = Duration::from_secs(50);

/// How long the helper waits for the quit once it stands ready, as
/// fastframe-update has it: until then a quit installs, whatever the card
/// says.
pub const HELPER_GIVES_UP: Duration = Duration::from_secs(60);

impl Install {
    /// When the helper said it stands ready, where it waits for the quit.
    pub fn handed(&self) -> Option<Instant> {
        match self {
            Self::Ready { handed, .. } | Self::Held { handed, .. } => *handed,
            Self::Idle | Self::Downloading(_) | Self::Restarting { .. } | Self::Failed { .. } => {
                None
            }
        }
    }

    /// The version it is about, but while idle.
    pub fn version(&self) -> Option<&str> {
        match self {
            Self::Idle => None,
            Self::Downloading(downloading) => Some(&downloading.version),
            Self::Ready { version, .. }
            | Self::Held { version, .. }
            | Self::Restarting { version }
            | Self::Failed { version, .. } => Some(version),
        }
    }

    /// Whether a download runs, or one is ready: no check runs meanwhile,
    /// and no other download starts.
    pub fn busy(&self) -> bool {
        match self {
            Self::Downloading(_)
            | Self::Ready { .. }
            | Self::Held { .. }
            | Self::Restarting { .. } => true,
            Self::Idle | Self::Failed { .. } => false,
        }
    }
}

/// What Settings › Updates and the signals elsewhere know.
#[derive(Clone, Debug, Default)]
pub struct Updates {
    /// The state file's.
    pub seen: Seen,
    pub phase: Phase,
    /// How this copy was installed, once the backend has looked.
    pub installed: Option<Installed>,
    /// When the last scheduled check that failed ran: the next waits an
    /// interval from it.
    pub tried: Option<u64>,
    /// A version found by a check the person asked for, though they had
    /// said to skip it: offered until the next check.
    pub asked_offer: Option<Offer>,
    /// When a helper last said it stands ready. Kept past Couldn't
    /// install: a quit within [`HELPER_GIVES_UP`] of it still installs.
    pub handoff_ready: Option<Instant>,
    /// A check's answer that came before the detection did: taken in once
    /// it does, so the offer keeps this copy's kind's download.
    pub held: Option<(u64, Result<release::Latest, Failed>)>,
    /// Where the download of the version offered stands.
    pub install: Install,
    /// `:update` waits for its check, to ask whether to install what it
    /// finds.
    pub ask_after_check: bool,
}

impl Updates {
    /// The release to offer now, if any: newer than this version, and not
    /// the one the settings skip unless a check that was asked for found
    /// it.
    pub fn offer(&self, skip: Option<&str>) -> Option<&Offer> {
        self.asked_offer
            .as_ref()
            .or_else(|| {
                self.seen
                    .offered(CURRENT)
                    .filter(|offer| skip != Some(offer.version.as_str()))
            })
            .filter(|offer| fastframe_update::is_newer(&offer.version, CURRENT))
    }

    pub fn checking(&self) -> bool {
        matches!(self.phase, Phase::Checking { .. })
    }

    /// The version offered, where this copy can download and install it:
    /// it replaces itself, the release has its build, and this is no
    /// Windows build.
    pub fn installable(&self, skip: Option<&str>) -> Option<&Offer> {
        let itself = matches!(self.installed, Some(Installed::Itself(_)));
        self.offer(skip)
            .filter(|offer| DOWNLOADS && itself && offer.download.is_some())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_configuration_is_valid() {
        CONFIG.validate().unwrap();
        assert_eq!(CONFIG.current_version, env!("CARGO_PKG_VERSION"));
    }

    /// Without a key the updater trusts checksums.txt as GitHub serves it;
    /// with one, a release must be signed (packaging/sign-checksums.sh).
    #[test]
    fn updates_need_the_release_signature() {
        assert!(CONFIG.publisher_key.is_some());
    }

    /// The release names its AppImages for the updater (release.yml), so
    /// an AppImage may replace itself.
    #[test]
    fn appimages_update_themselves() {
        const { assert!(CONFIG.appimage) };
    }

    const DAY: u64 = 24 * 60 * 60;

    #[test]
    fn a_daily_or_weekly_check_is_due_an_interval_after_the_last() {
        let checked = |at| Seen {
            last_checked: Some(at),
            ..Seen::default()
        };
        let now = 100 * DAY;
        // Never checked: due at once.
        assert_eq!(
            due(UpdateCheck::Daily, &Seen::default(), None, now),
            Some(0)
        );
        assert_eq!(
            due(UpdateCheck::Daily, &checked(now - 2 * DAY), None, now),
            Some(0)
        );
        assert_eq!(
            due(UpdateCheck::Daily, &checked(now - 3_600), None, now),
            Some(DAY - 3_600)
        );
        assert_eq!(
            due(UpdateCheck::Weekly, &checked(now - 2 * DAY), None, now),
            Some(5 * DAY)
        );
        assert_eq!(
            due(UpdateCheck::Weekly, &checked(now - 8 * DAY), None, now),
            Some(0)
        );
        // Only when asked.
        assert_eq!(due(UpdateCheck::Never, &Seen::default(), None, now), None);
    }

    /// A failed check is no check: the next scheduled one waits an
    /// interval from the failure, and the last check that worked stays.
    #[test]
    fn a_check_that_failed_retries_at_the_next_interval() {
        let seen = Seen {
            last_checked: Some(0),
            ..Seen::default()
        };
        let now = 10 * DAY;
        assert_eq!(
            due(UpdateCheck::Daily, &seen, Some(now - 60), now),
            Some(DAY - 60)
        );
    }

    fn offered(version: &str) -> Seen {
        Seen {
            offer: Some(Offer {
                version: version.into(),
                ..Offer::default()
            }),
            ..Seen::default()
        }
    }

    #[test]
    fn the_skipped_version_is_offered_only_after_a_check_that_was_asked_for() {
        let updates = Updates {
            seen: offered("99.0.0"),
            ..Updates::default()
        };
        assert!(updates.offer(None).is_some());
        assert!(updates.offer(Some("99.0.0")).is_none());
        assert!(updates.offer(Some("98.0.0")).is_some());
        let asked = Updates {
            asked_offer: updates.seen.offer.clone(),
            ..updates
        };
        assert!(asked.offer(Some("99.0.0")).is_some());
        // Nothing older than this version, ever.
        let old = Updates {
            seen: offered("0.0.1"),
            ..Updates::default()
        };
        assert!(old.offer(None).is_none());
    }

    /// A clock that was ahead once (a dead RTC battery, a VM) leaves the
    /// last check "in the future": it is left out, and the check is due,
    /// on this tick and the next.
    #[test]
    fn a_last_check_in_the_future_does_not_hold_checks_back() {
        let now = 100 * DAY;
        let ahead = now + 365 * DAY;
        let seen = Seen {
            last_checked: Some(ahead),
            ..Seen::default()
        };
        assert_eq!(due(UpdateCheck::Daily, &seen, None, now), Some(0));
        assert_eq!(due(UpdateCheck::Daily, &seen, None, now + DAY + 1), Some(0));
        assert_eq!(
            due(UpdateCheck::Daily, &Seen::default(), Some(ahead), now),
            Some(0)
        );
        // A failure since still holds the next check back.
        assert_eq!(
            due(UpdateCheck::Daily, &seen, Some(now - 60), now),
            Some(DAY - 60)
        );
    }
}

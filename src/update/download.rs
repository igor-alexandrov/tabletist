//! What the card makes of a download: why one gave nothing to install,
//! sorted from fastframe-update's words, how long it has left, and the
//! issue a failure can be reported with.

use std::time::Duration;

/// Why a download gave no update to install.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Failure {
    /// Verification failed: the file is not what the release signed. It
    /// was deleted.
    Unverified(Unverified),
    /// Couldn't download: the file never came whole, or could not be
    /// kept beside the app.
    Undone(Undone),
    /// Couldn't install: the helper that swaps the app did not start.
    /// fastframe-update's words.
    NotHanded(String),
    /// Couldn't install: work was opened while the helper got ready, and
    /// was not done before the helper gave up.
    WorkBegan,
    /// Couldn't install: the helper's answer came when it was too late to
    /// quit in time (the window was hidden).
    TooLate,
}

/// What did not verify.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Unverified {
    /// The signature of the release's checksums.
    Signature,
    /// The checksums themselves: missing, twice, or of the wrong size.
    Checksums,
    /// The file's own hash.
    Hash,
    /// The new version says another version than the release.
    Version,
    /// The new version did not start, or not in time.
    Startup,
    /// macOS: what was downloaded is no Tabletist app.
    Bundle,
    /// macOS: signed by another developer.
    Publisher,
    /// macOS: its signature did not check, or Gatekeeper refused it.
    Approval,
}

/// Why the file never came whole.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Undone {
    /// The connection dropped and did not come back in time.
    Lost,
    /// No answer at the start.
    Unreachable,
    /// The server answered with this status.
    Status(u16),
    /// The folder beside the app takes no files.
    NotWritable,
    /// The release is not the one checked.
    Changed,
    /// Something else, in fastframe-update's words.
    Other(String),
}

impl Failure {
    /// The failure an error of `Updater::download` says, from its words:
    /// fastframe-update's errors carry no type.
    pub fn of(error: &anyhow::Error) -> Self {
        Self::of_words(&format!("{error:#}"))
    }

    fn of_words(words: &str) -> Self {
        let has = |part: &str| words.contains(part);
        let unverified = if has("signed update metadata size") || has("checksum") {
            Some(Unverified::Checksums)
        } else if has("publisher signature") {
            Some(Unverified::Signature)
        } else if has("The download couldn't be verified") {
            Some(Unverified::Hash)
        } else if has("app bundle") {
            Some(Unverified::Bundle)
        } else if has("has the wrong version") {
            Some(Unverified::Version)
        } else if has("startup check") || has("cannot run on this computer") {
            Some(Unverified::Startup)
        } else if has("different publisher") {
            Some(Unverified::Publisher)
        } else if has("could not approve")
            || has("app signature")
            || has("signing identity")
            || has("disk image")
        {
            Some(Unverified::Approval)
        } else {
            None
        };
        if let Some(unverified) = unverified {
            return Self::Unverified(unverified);
        }
        let status = words
            .split("The update server answered ")
            .nth(1)
            .and_then(|rest| {
                let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
                digits.parse().ok()
            });
        Self::Undone(if let Some(status) = status {
            Undone::Status(status)
        } else if has(super::transport::LOST) || has("was interrupted") {
            Undone::Lost
        } else if has(super::transport::UNREACHABLE) {
            Undone::Unreachable
        } else if has("Cannot write to the installation directory") {
            Undone::NotWritable
        } else if has("The release changed") {
            Undone::Changed
        } else {
            Undone::Other(words.to_owned())
        })
    }

    /// What the card's box says of it, in English, `{status}` and
    /// `{error}` put in by [`Failure::with_parts`]: the caller translates
    /// first. Verification failed's box then says the file was deleted.
    pub fn said(&self) -> &'static str {
        match self {
            Self::Unverified(unverified) => match unverified {
                Unverified::Signature => "The signature doesn't match Tabletist's release key.",
                Unverified::Checksums => "The release's checksums couldn't be read.",
                Unverified::Hash => "The file doesn't match its checksum in the release.",
                Unverified::Version => "The file holds another version than the release.",
                Unverified::Startup => "The new version didn't pass its startup check.",
                Unverified::Bundle => "The download isn't a Tabletist app.",
                Unverified::Publisher => "The app was signed by another developer.",
                Unverified::Approval => "macOS couldn't check the app's signature.",
            },
            Self::Undone(undone) => match undone {
                Undone::Lost => "The connection was lost and didn't come back.",
                Undone::Unreachable => {
                    "Couldn't reach github.com. Check the connection to the internet."
                }
                Undone::Status(_) => "GitHub answered with an error ({status}).",
                Undone::NotWritable => "Tabletist can't write to the folder it is in.",
                Undone::Changed => "The release changed. Check for updates again.",
                Undone::Other(_) => "{error}",
            },
            Self::NotHanded(_) => "The installer didn't start: {error}",
            Self::WorkBegan => {
                "Work was still open when the installer stopped waiting, so Tabletist didn't restart."
            }
            Self::TooLate => "Tabletist couldn't quit in time, so the installer stopped waiting.",
        }
    }

    /// [`Failure::said`], translated, with its parts in.
    pub fn with_parts(&self, said: &str) -> String {
        let status = match self {
            Self::Undone(Undone::Status(status)) => status.to_string(),
            _ => String::new(),
        };
        let error = match self {
            Self::Undone(Undone::Other(error)) | Self::NotHanded(error) => error.as_str(),
            _ => "",
        };
        said.replace("{status}", &status).replace("{error}", error)
    }

    /// Whether the file was refused: Verification failed.
    pub fn unverified(&self) -> bool {
        matches!(self, Self::Unverified(_))
    }
}

/// How long a download has left at the speed of its last [`WINDOW`],
/// from `samples` of it (the time since it started, and the bytes it had
/// then, oldest first), out of `total` bytes. None for its first
/// [`SETTLING`], before there are two samples, and while nothing came.
pub fn time_left(samples: &[(Duration, u64)], total: u64) -> Option<Duration> {
    let &(now, received) = samples.last()?;
    if now < SETTLING {
        return None;
    }
    // The newest sample at least a window old, else the oldest: progress
    // comes a megabyte at a time, which a slow network takes longer for.
    let &(then, before) = samples
        .iter()
        .rev()
        .find(|(at, _)| now.saturating_sub(*at) >= WINDOW)
        .or(samples.first())?;
    let took = now.checked_sub(then).filter(|took| !took.is_zero())?;
    let came = received.checked_sub(before).filter(|came| *came > 0)?;
    // Seconds and bytes: the precision of f64 is no matter here.
    #[allow(clippy::cast_precision_loss)]
    let speed = came as f64 / took.as_secs_f64();
    #[allow(clippy::cast_precision_loss)]
    let left = total.saturating_sub(received) as f64 / speed;
    Some(Duration::from_secs_f64(left.min(1e9)))
}

/// The time [`time_left`] reads the speed over.
pub const WINDOW: Duration = Duration::from_secs(5);

/// How long a download runs before [`time_left`] says anything.
pub const SETTLING: Duration = Duration::from_secs(2);

/// The samples worth keeping for [`time_left`]: those of the last
/// [`WINDOW`], and the newest before it.
pub fn keep_window(samples: &mut Vec<(Duration, u64)>) {
    let Some(&(now, _)) = samples.last() else {
        return;
    };
    let old = samples
        .iter()
        .rposition(|(at, _)| now.saturating_sub(*at) >= WINDOW);
    if let Some(old) = old {
        samples.drain(..old);
    }
}

/// A time left as the card says it, English with `{n}` for the caller to
/// translate and fill: under a minute to the next 5 seconds, then to the
/// next minute.
pub fn left_words(left: Duration) -> (&'static str, u64) {
    let secs = left.as_secs() + u64::from(left.subsec_nanos() > 0);
    let fives = secs.div_ceil(5).max(1) * 5;
    match fives {
        0..60 => ("about {n} s left", fives),
        _ => ("about {n} min left", secs.div_ceil(60)),
    }
}

/// A new issue on GitHub about `failure`, the download of `version` on
/// this computer, its title and text filled in: nothing is sent until the
/// person submits it there.
pub fn issue_url(version: &str, failure: &Failure, said: &str) -> String {
    let kind = match failure {
        Failure::Unverified(_) => "didn't verify",
        Failure::Undone(_) => "didn't download",
        Failure::NotHanded(_) | Failure::WorkBegan | Failure::TooLate => "didn't install",
    };
    let title = format!("Update to {version} {kind}");
    let body = format!(
        "Tabletist {current} on {os} {arch} could not update to {version}.\n\n{said}\n",
        current = super::CURRENT,
        os = std::env::consts::OS,
        arch = std::env::consts::ARCH,
    );
    format!(
        "https://github.com/igor-alexandrov/tabletist/issues/new?title={}&body={}",
        encoded(&title),
        encoded(&body)
    )
}

/// `text` as a part of a query: every byte but the unreserved ones
/// percent-encoded.
fn encoded(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for byte in text.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(char::from(byte));
            }
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sorted(words: &str) -> Failure {
        Failure::of(&anyhow::anyhow!(words.to_owned()))
    }

    /// Each of fastframe-update's verification failures is Verification
    /// failed.
    #[test]
    fn a_file_that_did_not_verify_is_verification_failed() {
        for (words, what) in [
            ("Invalid update publisher signature", Unverified::Signature),
            (
                "The update publisher signature could not be verified",
                Unverified::Signature,
            ),
            ("Duplicate checksum for the update", Unverified::Checksums),
            ("Invalid update checksum", Unverified::Checksums),
            (
                "The release is missing the update checksum",
                Unverified::Checksums,
            ),
            ("Invalid signed update metadata size", Unverified::Checksums),
            (
                "The download couldn't be verified. Try downloading it again.",
                Unverified::Hash,
            ),
            (
                "The downloaded app has the wrong version",
                Unverified::Version,
            ),
            (
                "The downloaded app cannot run on this computer",
                Unverified::Startup,
            ),
            (
                "The downloaded app failed its startup check",
                Unverified::Startup,
            ),
            (
                "The downloaded app did not answer its startup check",
                Unverified::Startup,
            ),
            (
                "The download is not a Tabletist app bundle",
                Unverified::Bundle,
            ),
            ("The app bundle has the wrong version", Unverified::Bundle),
            (
                "The update was signed by a different publisher",
                Unverified::Publisher,
            ),
            (
                "macOS could not approve this update for launch",
                Unverified::Approval,
            ),
            (
                "The app signature could not be verified",
                Unverified::Approval,
            ),
            ("Cannot read the app signing identity", Unverified::Approval),
            (
                "macOS could not open the downloaded disk image",
                Unverified::Approval,
            ),
        ] {
            assert_eq!(sorted(words), Failure::Unverified(what), "{words}");
            assert!(sorted(words).unverified(), "{words}");
        }
    }

    /// A network that went, a server's error or a folder that takes no
    /// files is Couldn't download: the file was never bad.
    #[test]
    fn a_file_that_never_came_whole_is_couldnt_download() {
        for (words, why) in [
            (super::super::transport::LOST, Undone::Lost),
            ("The update download was interrupted", Undone::Lost),
            ("The update server answered 503", Undone::Status(503)),
            (
                "The update server could not be reached: io: Connection refused",
                Undone::Unreachable,
            ),
            (
                "Cannot write to the installation directory: Permission denied",
                Undone::NotWritable,
            ),
            (
                "The release changed. Check for updates again.",
                Undone::Changed,
            ),
            (
                "Update redirect is not allowed",
                Undone::Other("Update redirect is not allowed".into()),
            ),
        ] {
            assert_eq!(sorted(words), Failure::Undone(why), "{words}");
            assert!(!sorted(words).unverified(), "{words}");
        }
    }

    /// The error's whole chain is read: the crate wraps an I/O error of
    /// the body in context of its own.
    #[test]
    fn the_words_are_read_down_the_chain() {
        let error = anyhow::Error::new(std::io::Error::other(super::super::transport::LOST))
            .context("Could not download the update");
        assert_eq!(Failure::of(&error), Failure::Undone(Undone::Lost));
    }

    #[test]
    fn what_a_failure_says_has_its_parts() {
        let status = Failure::Undone(Undone::Status(502));
        assert_eq!(
            status.with_parts(status.said()),
            "GitHub answered with an error (502)."
        );
        let other = Failure::Undone(Undone::Other("Update redirect is not allowed".into()));
        assert_eq!(
            other.with_parts(other.said()),
            "Update redirect is not allowed"
        );
    }

    const MB: u64 = 1_000_000;

    fn secs(secs: u64) -> Duration {
        Duration::from_secs(secs)
    }

    #[test]
    fn the_time_left_is_read_from_the_last_five_seconds() {
        // 1 MB a second for 10 s, then 2 MB a second for 5.
        let mut samples: Vec<(Duration, u64)> = (0..=10).map(|at| (secs(at), at * MB)).collect();
        samples.extend((11..=15).map(|at| (secs(at), 10 * MB + (at - 10) * 2 * MB)));
        let left = time_left(&samples, 40 * MB).unwrap();
        // 20 MB to go at 2 MB a second.
        assert_eq!(left.as_secs(), 10);
        // Not in the first two seconds.
        assert_eq!(time_left(&[(secs(0), 0), (secs(1), 3 * MB)], 40 * MB), None);
        // Nothing came: no telling.
        assert_eq!(time_left(&[(secs(0), 0), (secs(4), 0)], 40 * MB), None);
    }

    #[test]
    fn a_slow_download_reads_its_speed_from_the_sample_before_the_window() {
        // A megabyte every 8 s.
        let samples = [(secs(0), 0), (secs(8), MB), (secs(16), 2 * MB)];
        let left = time_left(&samples, 4 * MB).unwrap();
        assert_eq!(left.as_secs(), 16);
    }

    #[test]
    fn only_the_samples_of_the_window_and_one_before_are_kept() {
        let mut samples: Vec<(Duration, u64)> = (0..=10).map(|at| (secs(at), at * MB)).collect();
        keep_window(&mut samples);
        assert_eq!(samples.first(), Some(&(secs(5), 5 * MB)));
        assert_eq!(samples.len(), 6);
    }

    #[test]
    fn the_time_left_is_said_to_five_seconds_then_to_the_minute() {
        assert_eq!(
            left_words(Duration::from_millis(400)),
            ("about {n} s left", 5)
        );
        assert_eq!(left_words(secs(18)), ("about {n} s left", 20));
        assert_eq!(left_words(secs(20)), ("about {n} s left", 20));
        assert_eq!(left_words(secs(54)), ("about {n} s left", 55));
        assert_eq!(left_words(secs(59)), ("about {n} min left", 1));
        assert_eq!(left_words(secs(61)), ("about {n} min left", 2));
    }

    #[test]
    fn the_issue_names_the_versions_the_system_and_the_error() {
        let failure = Failure::Unverified(Unverified::Signature);
        let url = issue_url("99.0.0", &failure, failure.said());
        assert!(
            url.starts_with("https://github.com/igor-alexandrov/tabletist/issues/new?title=Update%20to%2099.0.0%20didn%27t%20verify&body="),
            "{url}"
        );
        assert!(url.contains("signature%20doesn%27t%20match"), "{url}");
        assert!(url.contains(&encoded(std::env::consts::OS)), "{url}");
        assert!(!url.contains(' '), "{url}");
    }
}

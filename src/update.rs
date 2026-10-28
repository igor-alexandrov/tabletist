//! Updates from Tabletist's GitHub releases: the check Settings › Updates
//! shows (`check`, `release`), what it found as the state folder keeps it
//! (`seen`), how this copy was installed (`installed`), and the updater
//! that installs one, fastframe-update.
//!
//! What a later release's helper relies on from this build, which must not
//! change: `tabletist --version` prints `tabletist <version>`;
//! `--update-receipt <job>` and `--update-error <message>` are taken out
//! before clap reads the command line ([`fastframe_update::intercept`] in
//! `entrypoint::run`); and the receipt is confirmed once the window is up.
//! The release side of the contract (asset names, markers, the signed
//! checksums) is in `release.yml` and `tests/update_release.rs`.

pub mod check;
pub mod installed;
pub mod release;
pub mod seen;

use std::time::Duration;

use fastframe_update::{MacConfig, Request, Response, Transport, UpdateConfig, Updater};

pub use check::Failed;
pub use installed::{Installed, Managed};
pub use release::{Found, Offer};
pub use seen::Seen;

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
    Updater::new(CONFIG, UreqTransport::new())
}

/// The release page of the version that runs: What's new in it.
pub fn this_release() -> String {
    format!("https://github.com/igor-alexandrov/tabletist/releases/tag/v{CURRENT}")
}

/// The updater's HTTP client: one GET at a time, redirects handed back
/// rather than followed (the updater follows them, to GitHub's hosts
/// only), any status returned as an answer, 15 seconds to connect and 15
/// minutes for a whole download, as fastframe-update asks of a transport.
/// It says what the check says of this copy, and nothing else.
struct UreqTransport {
    agent: ureq::Agent,
    user_agent: String,
}

impl UreqTransport {
    fn new() -> Self {
        let config = ureq::Agent::config_builder()
            .max_redirects(0)
            .http_status_as_error(false)
            .timeout_connect(Some(Duration::from_secs(15)))
            .timeout_global(Some(Duration::from_secs(15 * 60)))
            .build();
        Self {
            agent: config.into(),
            user_agent: check::user_agent(CURRENT, check::macos_version().as_deref()),
        }
    }
}

impl Transport for UreqTransport {
    fn get(&self, request: &Request<'_>) -> anyhow::Result<Response> {
        let response = self
            .agent
            .get(request.url)
            .header("Accept", request.accept)
            .header("User-Agent", &self.user_agent)
            .call()?;
        let location = response
            .headers()
            .get("location")
            .and_then(|value| value.to_str().ok())
            .map(str::to_owned);
        Ok(Response {
            status: response.status().as_u16(),
            location,
            body: Box::new(response.into_body().into_reader()),
        })
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
}

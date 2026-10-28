//! The check for updates and how this copy was installed, on the
//! backend's blocking pool: each is a request to GitHub, or a look at the
//! disk and the package manager, which a frame never waits for.

use std::path::PathBuf;
use std::time::Duration;

use crate::update::check::Failed;
use crate::update::release::Latest;

/// What the UI asks of the updater.
#[derive(Debug)]
pub enum UpdatesCommand {
    /// Asks `url` for the latest release, waiting `timeout` at most.
    /// Answered with [`UpdatesEvent::Checked`].
    Check {
        asked: u64,
        url: String,
        timeout: Duration,
    },
    /// Works out how this copy was installed. Answered with
    /// [`UpdatesEvent::Detected`].
    Detect,
    /// Shows `text` (a release's notes) in `$PAGER` in a terminal, written
    /// to `path` first. Answered with [`UpdatesEvent::NotesUnshown`] where
    /// no terminal could be started.
    ShowNotes {
        path: PathBuf,
        text: String,
        url: String,
    },
}

/// What came of an [`UpdatesCommand`].
#[derive(Debug)]
pub enum UpdatesEvent {
    Checked {
        asked: u64,
        result: Result<Latest, Failed>,
    },
    Detected(crate::update::Installed),
    /// The notes could not be shown in a terminal: their page is opened
    /// instead.
    NotesUnshown {
        url: String,
    },
}

pub fn handle(command: UpdatesCommand, outbox: &super::Outbox) {
    let outbox = outbox.clone();
    match command {
        UpdatesCommand::Check {
            asked,
            url,
            timeout,
        } => {
            tokio::task::spawn_blocking(move || {
                let agent = crate::update::check::user_agent(
                    crate::update::CURRENT,
                    crate::update::check::macos_version().as_deref(),
                );
                let result = crate::update::check::latest(&url, &agent, timeout);
                if let Err(failed) = &result {
                    log::info!("the check for updates failed: {failed:?}");
                }
                outbox.emit(super::Event::Updates(UpdatesEvent::Checked {
                    asked,
                    result,
                }));
            });
        }
        UpdatesCommand::Detect => {
            tokio::task::spawn_blocking(move || {
                let installed = crate::update::Installed::detect();
                outbox.emit(super::Event::Updates(UpdatesEvent::Detected(installed)));
            });
        }
        UpdatesCommand::ShowNotes { path, text, url } => {
            tokio::task::spawn_blocking(move || {
                let shown = crate::util::write_atomic(&path, text.as_bytes())
                    .and_then(|()| start_pager(&path));
                if let Err(error) = shown {
                    log::info!("could not show the release notes in a terminal: {error}");
                    outbox.emit(super::Event::Updates(UpdatesEvent::NotesUnshown { url }));
                }
            });
        }
    }
}

/// Starts `$PAGER` (else `less`) on `path` in a terminal of the desktop's:
/// `xdg-terminal-exec`, which Omarchy sets up. Fails where there is none,
/// or where it finds no terminal to start.
fn start_pager(path: &std::path::Path) -> std::io::Result<()> {
    let pager = std::env::var("PAGER")
        .ok()
        .filter(|pager| !pager.trim().is_empty())
        .unwrap_or_else(|| "less".to_owned());
    let mut command = std::process::Command::new("xdg-terminal-exec");
    // The pager as the shell would run it: `$PAGER` may hold its flags.
    command
        .arg("sh")
        .arg("-c")
        .arg(format!("{pager} \"$1\""))
        .arg("sh")
        .arg(path);
    launched(command, LAUNCH_FAILS_WITHIN)
}

/// How long a launcher has to fail before its terminal counts as shown.
/// `xdg-terminal-exec` ends at once, with a failure, where it finds no
/// terminal, and otherwise runs as long as the terminal it started.
const LAUNCH_FAILS_WITHIN: Duration = Duration::from_secs(2);

/// Starts `command`, a launcher, and waits `within` for it to fail. One
/// still running then, or ended well, has shown its terminal: a thread of
/// its own waits for it from there, so it leaves no zombie.
fn launched(mut command: std::process::Command, within: Duration) -> std::io::Result<()> {
    use std::process::Stdio;
    let mut child = command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;
    let deadline = std::time::Instant::now() + within;
    while std::time::Instant::now() < deadline {
        if let Some(status) = child.try_wait()? {
            return match status.success() {
                true => Ok(()),
                false => Err(std::io::Error::other(format!(
                    "the terminal's launcher ended with {status}"
                ))),
            };
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    std::thread::spawn(move || {
        let _ = child.wait();
    });
    Ok(())
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;

    fn sh(script: &str) -> std::process::Command {
        let mut command = std::process::Command::new("sh");
        command.arg("-c").arg(script);
        command
    }

    #[test]
    fn a_launcher_that_finds_no_terminal_has_shown_nothing() {
        assert!(launched(sh("exit 1"), LAUNCH_FAILS_WITHIN).is_err());
    }

    #[test]
    fn a_launcher_that_ends_well_or_runs_on_has_shown_its_terminal() {
        assert!(launched(sh("exit 0"), LAUNCH_FAILS_WITHIN).is_ok());
        // A terminal open past the wait, failing later or not, was shown.
        let started = std::time::Instant::now();
        assert!(launched(sh("sleep 1; exit 1"), Duration::from_millis(200)).is_ok());
        assert!(started.elapsed() < Duration::from_secs(1));
    }

    #[test]
    fn a_launcher_missing_has_shown_nothing() {
        let command = std::process::Command::new("tabletist-no-such-launcher");
        assert!(launched(command, LAUNCH_FAILS_WITHIN).is_err());
    }
}

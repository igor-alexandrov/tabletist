//! The check for updates and how this copy was installed, on the
//! backend's blocking pool: each is a request to GitHub, or a look at the
//! disk and the package manager, which a frame never waits for.

use std::path::PathBuf;

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
        timeout: std::time::Duration,
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
/// `xdg-terminal-exec`, which Omarchy sets up. Fails where there is none.
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
    super::start("xdg-terminal-exec".to_owned(), command)
}

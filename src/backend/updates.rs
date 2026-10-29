//! The check for updates, how this copy was installed, and the download
//! and install of an update, on the backend's blocking pool: each is a
//! request to GitHub, or a look at the disk and the package manager,
//! which a frame never waits for. A download that verified waits here,
//! for Restart now: fastframe-update's `Prepared` cannot be saved, so a
//! quit drops it.

use std::path::PathBuf;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};

use fastframe_update::{Prepared, Release};

use super::StateFile;
use crate::update::check::Failed;
use crate::update::download::Failure;
use crate::update::release::Latest;
use crate::update::transport::{Download, UreqTransport};

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
    /// Downloads and verifies `version`, whose release page is `url`,
    /// dropping any other under way and any held. Told with
    /// [`UpdatesEvent::Progress`] and [`UpdatesEvent::Waiting`], and
    /// answered with [`UpdatesEvent::Downloaded`]; a download that verified
    /// is held for [`UpdatesCommand::Install`].
    Download {
        asked: u64,
        version: String,
        url: String,
    },
    /// Stops the download under way: fastframe-update deletes what it
    /// wrote. Nothing answers it.
    CancelDownload,
    /// Queues `relaunch` for `path` with the state files, then hands the
    /// download held to fastframe-update's helper, which installs it once
    /// the app quits and starts it with `arguments`. The file goes again
    /// where the helper does not start. Answered with
    /// [`UpdatesEvent::Handed`].
    Install {
        arguments: Vec<String>,
        relaunch: crate::update::relaunch::Relaunch,
        path: PathBuf,
    },
    /// Deletes the relaunch file at `path`, after any write of it queued
    /// before: the handoff did not end in the quit.
    ForgetRelaunch { path: PathBuf },
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
    /// The download `asked` has `received` of `total` bytes, `after` this
    /// long: told each megabyte and at the end.
    Progress {
        asked: u64,
        received: u64,
        total: u64,
        after: Duration,
    },
    /// The download `asked` waits for the network, or goes on.
    Waiting {
        asked: u64,
        waiting: bool,
    },
    /// The download `asked` verified, and is held, or failed. A cancelled
    /// one is not answered.
    Downloaded {
        asked: u64,
        result: Result<(), Failure>,
    },
    /// The helper stands ready since `at`, and the app is to quit now; or
    /// it did not start, in fastframe-update's words. The download held is
    /// gone either way.
    Handed {
        result: Result<(), String>,
        at: Instant,
    },
}

/// What the backend keeps of updates between commands.
#[derive(Default)]
pub struct Updates {
    /// The download under way, by its ask, and what its body shares.
    downloading: Option<(u64, Arc<Download>)>,
    /// The download that verified, until it is installed or another
    /// starts.
    held: Arc<Mutex<Option<Prepared>>>,
}

impl Updates {
    pub fn handle(
        &mut self,
        command: UpdatesCommand,
        saves: &super::Saves,
        outbox: &super::Outbox,
    ) {
        match command {
            UpdatesCommand::Download {
                asked,
                version,
                url,
            } => self.download(asked, Release { version, url }, outbox),
            // A download that verified as the cancel came is held by
            // now: it goes too.
            UpdatesCommand::CancelDownload => {
                if let Some((_, download)) = self.downloading.take() {
                    download.cancel();
                }
                let held = self
                    .held
                    .lock()
                    .unwrap_or_else(PoisonError::into_inner)
                    .take();
                if let Some(held) = held {
                    tokio::task::spawn_blocking(move || held.discard());
                }
            }
            UpdatesCommand::Install {
                arguments,
                relaunch,
                path,
            } => {
                let held = self
                    .held
                    .lock()
                    .unwrap_or_else(PoisonError::into_inner)
                    .take();
                // Through the state files' queue, as the quit's own write
                // of it is: an older write cannot land after a newer one.
                if held.is_some() {
                    saves.save(path.clone(), StateFile::Relaunch(Some(relaunch)), outbox);
                }
                let saves = saves.clone();
                let outbox = outbox.clone();
                tokio::task::spawn_blocking(move || {
                    let handed = match held {
                        Some(prepared) => crate::update::updater()
                            .handoff(prepared, arguments)
                            .map_err(|error| format!("{error:#}")),
                        None => Err("no update is ready to install".to_owned()),
                    };
                    if let Err(error) = &handed {
                        log::error!("could not hand the update over: {error}");
                        saves.save(path, StateFile::Relaunch(None), &outbox);
                    }
                    outbox.emit(super::Event::Updates(UpdatesEvent::Handed {
                        result: handed,
                        at: Instant::now(),
                    }));
                });
            }
            UpdatesCommand::ForgetRelaunch { path } => {
                saves.save(path, StateFile::Relaunch(None), outbox);
            }
            command => handle(command, outbox),
        }
    }

    /// Starts downloading `release` on a thread of the blocking pool,
    /// in place of any download under way or held.
    fn download(&mut self, asked: u64, release: Release, outbox: &super::Outbox) {
        if let Some((_, download)) = self.downloading.take() {
            download.cancel();
        }
        let told = outbox.clone();
        let download = Arc::new(Download::new(move |waiting| {
            told.emit(super::Event::Updates(UpdatesEvent::Waiting {
                asked,
                waiting,
            }));
        }));
        self.downloading = Some((asked, Arc::clone(&download)));
        let held = Arc::clone(&self.held);
        let outbox = outbox.clone();
        tokio::task::spawn_blocking(move || {
            // The next download's folder takes the place of this one's.
            let before = held.lock().unwrap_or_else(PoisonError::into_inner).take();
            if let Some(before) = before {
                before.discard();
            }
            let updater = fastframe_update::Updater::new(
                crate::update::CONFIG,
                UreqTransport::downloading(Arc::clone(&download)),
            );
            let started = Instant::now();
            let result = updater.download(&release, |received, total| {
                outbox.emit(super::Event::Updates(UpdatesEvent::Progress {
                    asked,
                    received,
                    total,
                    after: started.elapsed(),
                }));
            });
            let result = match (result, download.cancelled()) {
                (Ok(prepared), true) => {
                    prepared.discard();
                    return;
                }
                (Err(error), true) => {
                    log::info!("the update download stopped: {error:#}");
                    return;
                }
                (Ok(prepared), false) => {
                    *held.lock().unwrap_or_else(PoisonError::into_inner) = Some(prepared);
                    Ok(())
                }
                (Err(error), false) => {
                    log::error!("the update download failed: {error:#}");
                    Err(Failure::of(&error))
                }
            };
            outbox.emit(super::Event::Updates(UpdatesEvent::Downloaded {
                asked,
                result,
            }));
        });
    }
}

fn handle(command: UpdatesCommand, outbox: &super::Outbox) {
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
        // Kept between commands: [`Updates::handle`]'s.
        UpdatesCommand::Download { .. }
        | UpdatesCommand::CancelDownload
        | UpdatesCommand::Install { .. }
        | UpdatesCommand::ForgetRelaunch { .. } => {}
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
        // A terminal open past the wait, failing later or not, was shown:
        // the launcher runs until the test lets it go, after the answer.
        let dir = tempfile::tempdir().unwrap();
        let go = dir.path().join("go");
        let mut command = sh(r#"while [ ! -e "$1" ]; do sleep 0.05; done; exit 1"#);
        command.arg("sh").arg(&go);
        let (answer, answered) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let _ = answer.send(launched(command, Duration::from_millis(200)).is_ok());
        });
        let shown = answered.recv_timeout(Duration::from_secs(30));
        std::fs::write(&go, "").unwrap();
        assert_eq!(shown, Ok(true));
    }

    #[test]
    fn a_launcher_missing_has_shown_nothing() {
        let command = std::process::Command::new("tabletist-no-such-launcher");
        assert!(launched(command, LAUNCH_FAILS_WITHIN).is_err());
    }
}

#[cfg(test)]
mod handed {
    use super::*;
    use crate::backend::{Backend, Command, Event, Waker};

    /// The first update event the backend sends within a few seconds.
    fn answer(backend: &mut Backend) -> UpdatesEvent {
        let deadline = Instant::now() + Duration::from_secs(20);
        while Instant::now() < deadline {
            for event in backend.poll() {
                if let Event::Updates(event) = event {
                    return event;
                }
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        panic!("no answer");
    }

    /// Nothing held: nothing is handed over, and no relaunch file is
    /// left for a next copy that will not come.
    #[test]
    fn install_with_no_download_held_hands_nothing_over() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("relaunch.json");
        std::fs::write(&path, "{}").unwrap();
        let mut backend = Backend::start_with(Waker::default(), crate::secrets::Keyring::memory());
        backend.send(Command::Updates(UpdatesCommand::Install {
            arguments: vec!["--relaunch".into()],
            relaunch: crate::update::relaunch::Relaunch::default(),
            path: path.clone(),
        }));
        assert!(matches!(
            answer(&mut backend),
            UpdatesEvent::Handed { result: Err(_), .. }
        ));
        // Deleted through the state files' queue: it lands soon after.
        let deadline = Instant::now() + Duration::from_secs(20);
        while path.exists() && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(10));
        }
        assert!(!path.exists());
    }

    #[test]
    fn forget_relaunch_deletes_the_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("relaunch.json");
        std::fs::write(&path, "{}").unwrap();
        let mut backend = Backend::start_with(Waker::default(), crate::secrets::Keyring::memory());
        backend.send(Command::Updates(UpdatesCommand::ForgetRelaunch {
            path: path.clone(),
        }));
        let deadline = Instant::now() + Duration::from_secs(20);
        while path.exists() && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(10));
        }
        assert!(!path.exists());
    }

    /// A test binary is no installation fastframe-update can replace: it
    /// asks nothing of the network, says Couldn't download, and holds
    /// nothing to install.
    #[test]
    fn a_copy_that_cannot_replace_itself_downloads_nothing() {
        let mut backend = Backend::start_with(Waker::default(), crate::secrets::Keyring::memory());
        backend.send(Command::Updates(UpdatesCommand::Download {
            asked: 7,
            version: "99.0.0".into(),
            // Never asked: the copy is refused first.
            url: "http://127.0.0.1:9/releases/tag/v99.0.0".into(),
        }));
        match answer(&mut backend) {
            UpdatesEvent::Downloaded {
                asked: 7,
                result: Err(Failure::Undone(_)),
            } => {}
            other => panic!("{other:?}"),
        }
        let dir = tempfile::tempdir().unwrap();
        backend.send(Command::Updates(UpdatesCommand::Install {
            arguments: Vec::new(),
            relaunch: crate::update::relaunch::Relaunch::default(),
            path: dir.path().join("relaunch.json"),
        }));
        assert!(matches!(
            answer(&mut backend),
            UpdatesEvent::Handed { result: Err(_), .. }
        ));
    }

    /// A download that verified as the cancel came is not left held: the
    /// next Install hands nothing over.
    #[tokio::test]
    async fn a_cancel_drops_a_download_that_verified_meanwhile() {
        let (events, _received) = std::sync::mpsc::channel();
        let outbox = crate::backend::Outbox {
            events,
            waker: Waker::default(),
        };
        let mut updates = Updates::default();
        let installation = fastframe_update::Installation {
            executable: std::path::PathBuf::from("/opt/tabletist/tabletist"),
            kind: fastframe_update::Kind::Portable,
        };
        *updates.held.lock().unwrap() = Some(Prepared::sample(installation, "99.0.0"));
        updates.handle(
            UpdatesCommand::CancelDownload,
            &crate::backend::Saves::default(),
            &outbox,
        );
        assert!(updates.held.lock().unwrap().is_none());
    }
}

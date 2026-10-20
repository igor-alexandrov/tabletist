//! The query history's file, written and read on the backend's runtime.
//!
//! Its commands are taken one at a time, in the order they came, by one
//! task of the blocking pool, which opens the file the first time it is
//! needed: an entry is written before the trim that follows it and before
//! a read sent after it. A statement never waits for any of it. An entry
//! the file cannot take is dropped with a line in the app's log.

use std::path::PathBuf;

use tokio::sync::mpsc as tokio_mpsc;

use crate::history::{Entry, Store};

/// What the UI asks of the history's file. Every one names the file.
#[derive(Debug)]
pub enum HistoryCommand {
    /// Adds an entry, then keeps the newest `keep` of its connection's.
    Append {
        file: PathBuf,
        entry: Box<Entry>,
        keep: u32,
    },
    /// Reads the newest `keep` entries of a connection. Answered with
    /// [`HistoryEvent::Loaded`].
    Load {
        file: PathBuf,
        connection: String,
        keep: u32,
        asked: u64,
    },
    /// Deletes a connection's entries, or every connection's.
    Clear {
        file: PathBuf,
        connection: Option<String>,
    },
    /// Keeps the newest `keep` entries of each connection.
    Trim { file: PathBuf, keep: u32 },
    /// Which of these agents' calls the Agents Log at `log` still holds.
    /// Answered with [`HistoryEvent::Held`].
    Held {
        log: PathBuf,
        calls: Vec<i64>,
        asked: u64,
    },
}

/// What came of a [`HistoryCommand`] that is answered.
#[derive(Debug)]
pub enum HistoryEvent {
    Loaded {
        asked: u64,
        result: Result<Vec<Entry>, String>,
    },
    Held {
        asked: u64,
        held: Result<Vec<i64>, String>,
    },
}

/// The task that takes the file's commands, started by the first one.
#[derive(Default)]
pub struct History {
    jobs: Option<tokio_mpsc::UnboundedSender<HistoryCommand>>,
}

impl History {
    pub fn handle(&mut self, command: HistoryCommand, outbox: &super::Outbox) {
        if let HistoryCommand::Held { log, calls, asked } = command {
            let outbox = outbox.clone();
            tokio::spawn(async move {
                let held = held(log, calls).await;
                outbox.emit(super::Event::History(HistoryEvent::Held { asked, held }));
            });
            return;
        }
        let jobs = self.jobs.get_or_insert_with(|| {
            let (jobs, queue) = tokio_mpsc::unbounded_channel();
            let outbox = outbox.clone();
            tokio::task::spawn_blocking(move || work(queue, &outbox));
            jobs
        });
        // The task ends only with the backend.
        let _ = jobs.send(command);
    }
}

/// Takes the commands in `queue` until the backend ends.
fn work(mut queue: tokio_mpsc::UnboundedReceiver<HistoryCommand>, outbox: &super::Outbox) {
    let mut open: Option<(PathBuf, Store)> = None;
    while let Some(command) = queue.blocking_recv() {
        let file = match &command {
            HistoryCommand::Append { file, .. }
            | HistoryCommand::Load { file, .. }
            | HistoryCommand::Clear { file, .. }
            | HistoryCommand::Trim { file, .. } => file.clone(),
            HistoryCommand::Held { .. } => continue,
        };
        if open.as_ref().is_none_or(|(path, _)| *path != file) {
            open = match Store::open(&file) {
                Ok(opened) => {
                    if let Some(aside) = opened.kept_aside {
                        log::warn!(
                            "the query history could not be read; it was kept as {}",
                            aside.display()
                        );
                    }
                    Some((file.clone(), opened.store))
                }
                Err(error) => {
                    log::error!("the query history could not be opened: {error}");
                    None
                }
            };
        }
        let store = open.as_ref().map(|(_, store)| store);
        match command {
            HistoryCommand::Append { entry, keep, .. } => {
                let written = store.map(|store| store.append(&entry, keep));
                if !matches!(written, Some(Ok(()))) {
                    if let Some(Err(error)) = written {
                        log::error!("a statement is not in the query history: {error}");
                    }
                    // A file that failed is opened again for the next one.
                    open = None;
                }
            }
            HistoryCommand::Load {
                connection,
                keep,
                asked,
                ..
            } => {
                let result = match store {
                    Some(store) => store
                        .load(&connection, keep)
                        .map_err(|error| error.to_string()),
                    None => Err("the file could not be opened".to_owned()),
                };
                outbox.emit(super::Event::History(HistoryEvent::Loaded {
                    asked,
                    result,
                }));
            }
            HistoryCommand::Clear { connection, .. } => {
                let cleared = store.map(|store| store.clear(connection.as_deref()));
                if let Some(Err(error)) = cleared {
                    log::error!("the query history could not be cleared: {error}");
                }
            }
            HistoryCommand::Trim { keep, .. } => {
                if let Some(Err(error)) = store.map(|store| store.trim(keep)) {
                    log::error!("the query history could not be trimmed: {error}");
                }
            }
            HistoryCommand::Held { .. } => {}
        }
    }
}

/// Which of `calls` the Agents Log at `log` still holds. No log yet holds
/// none.
async fn held(log: PathBuf, calls: Vec<i64>) -> Result<Vec<i64>, String> {
    let opened = tokio::task::spawn_blocking(move || tabletist_mcp::Log::open_to_read(&log))
        .await
        .map_err(|_| "opening it failed".to_owned())?
        .map_err(|error| error.to_string())?;
    match opened {
        Some(log) => log.holds(calls).await.map_err(|error| error.to_string()),
        None => Ok(Vec::new()),
    }
}

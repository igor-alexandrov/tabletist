//! Updates, as the app keeps them: when a check runs (on launch where one
//! is due, at the interval while the app is open, and when the person
//! asks), what it found, and what the window says of it; then the
//! download of the version offered, and the restart that installs it once
//! no window has work open. The requests are the backend's
//! ([`crate::backend::UpdatesCommand`]); what the checks found is kept in
//! the state folder (`updates.json`).

use std::time::Duration;

use super::App;
use crate::backend::{Command, StateFile, UpdatesCommand, UpdatesEvent};
use crate::model::{Action, ConnTabContent, ConnTabId, Dialog, SettingsPane, Tab, TabId};
use crate::update::{Downloading, Failed, Failure, Found, Install, Phase, download, release};

/// The flag an update's helper starts the new version with, after the
/// launch's own: it reopens the windows of the quit (`entrypoint.rs`).
pub const RELAUNCH: &str = "--relaunch";

/// What the terminal look's status line says of a version available,
/// `{version}` for it and `{key}` for Check for Updates' key as the keymap
/// has it, for [`ANNOUNCED_FOR`]. With no key, [`ANNOUNCEMENT_KEYLESS`].
pub const ANNOUNCEMENT: &str = "{version} available · {key}";
pub const ANNOUNCEMENT_KEYLESS: &str = "{version} available";

/// How long the status line says it.
pub const ANNOUNCED_FOR: f64 = 8.0;

/// What a restart would interrupt: a line of the Work in progress card.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Work {
    /// The window that holds it, and its tab.
    pub tab: ConnTabId,
    pub id: TabId,
    /// The window as the card names it: `Bookshop · dev`.
    pub window: String,
    pub what: Pending,
}

/// What is pending in a window.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Pending {
    /// Unsaved edits in a table: how many, and its name.
    Edits { count: usize, table: String },
    /// A save running in a table.
    Saving { table: String },
    /// A run of a SQL editor sent to write: the nearest Tabletist has to
    /// an open transaction. Query 2.
    Writing { query: u32 },
    /// Another run of a SQL editor, and how long it has gone.
    Running { query: u32, secs: u64 },
    /// A table's page, count or structure being read.
    Loading { table: String },
}

impl App {
    /// What a restart would interrupt now, window by window in the bar's
    /// order: each table's unsaved edits, its save or its read, then each
    /// SQL editor's run. Nothing is ever dropped to restart.
    pub fn work_in_progress(&self) -> Vec<Work> {
        let platform = crate::env::Platform::of(&self.look);
        let mut work = Vec::new();
        for (tab, workspace) in self.open_connections() {
            // As the window's title names it.
            let environment = workspace.environment.label(platform);
            let window = format!("{} · {environment}", workspace.name);
            for object in workspace.object_tabs() {
                let table = crate::ui::format::object_title(
                    &object.object,
                    workspace.name_is_shared(&object.object),
                );
                let what = if object.edits.saving.is_some() {
                    Pending::Saving { table }
                } else if object.edits.holds() {
                    Pending::Edits {
                        count: self.unwritten(tab, object.id),
                        table,
                    }
                } else if object.pending().next().is_some() {
                    Pending::Loading { table }
                } else {
                    continue;
                };
                work.push(Work {
                    tab,
                    id: object.id,
                    window: window.clone(),
                    what,
                });
            }
            for sql in workspace.tabs.iter().filter_map(|open| open.as_sql()) {
                let what = if sql.is_writing() {
                    Pending::Writing { query: sql.number }
                } else if sql.is_running() {
                    let secs = sql.running_for().unwrap_or_default().as_secs();
                    Pending::Running {
                        query: sql.number,
                        secs,
                    }
                } else {
                    continue;
                };
                work.push(Work {
                    tab,
                    id: sql.id,
                    window: window.clone(),
                    what,
                });
            }
        }
        work
    }
}

impl App {
    /// The time now, in seconds since the Unix epoch: the stopped clock in
    /// a test.
    pub(crate) fn now_secs(&self) -> u64 {
        let now = self.stopped_clock.unwrap_or_else(jiff::Timestamp::now);
        u64::try_from(now.as_second()).unwrap_or_default()
    }

    /// Starts what updates need once the window is up: how this copy was
    /// installed is worked out, and the scheduled checks run from here on.
    /// Not for a demo, whose profile is throwaway: it checks only when
    /// asked.
    pub fn watch_updates(&mut self) {
        self.updates_scheduled = true;
        self.backend.send(Command::Updates(UpdatesCommand::Detect));
    }

    /// Whether a query runs in a window: a scheduled check waits for it.
    fn query_runs(&self) -> bool {
        self.tabs.iter().any(|tab| match &tab.content {
            ConnTabContent::Workspace(workspace) => workspace.waits(),
            ConnTabContent::Picker(_) => false,
        })
    }

    /// Runs the scheduled check where it is due, unless a query runs: then
    /// it runs once none does. Says how long until the next is due, for a
    /// frame to be asked for then.
    pub fn tick_updates(&mut self) -> Option<Duration> {
        self.tick_restart();
        // The helper waits for the quit a minute at most: looked at each
        // second until it is over.
        if self.updates.install.handed().is_some() {
            return Some(Duration::from_secs(1));
        }
        if !self.updates_scheduled {
            return None;
        }
        let now = self.now_secs();
        self.announce_update(now);
        // The asset an offer keeps is this copy's kind's: the check waits
        // for the detection, which answers within the launch. None runs
        // while a download is under way or ready.
        if self.updates.checking()
            || self.updates.installed.is_none()
            || self.updates.install.busy()
        {
            return None;
        }
        let due = crate::update::due(
            self.settings.update_check,
            &self.updates.seen,
            self.updates.tried,
            now,
        )?;
        if due > 0 {
            return Some(Duration::from_secs(due));
        }
        if self.query_runs() {
            // Looked at again in a second: the query's answer asks for a
            // frame too.
            return Some(Duration::from_secs(1));
        }
        self.start_update_check(false);
        None
    }

    /// Asks GitHub for the latest release. `asked_for` where the person
    /// asked: that check offers a version the settings skip.
    fn start_update_check(&mut self, asked_for: bool) {
        let asked = self.next_id();
        self.updates.phase = Phase::Checking { asked, asked_for };
        self.backend.send(Command::Updates(UpdatesCommand::Check {
            asked,
            url: self.updates_url.clone(),
            timeout: crate::update::check::TIMEOUT,
        }));
    }

    /// The actions of Settings › Updates.
    pub(super) fn update_action(&mut self, action: Action) {
        match action {
            // One at a time: a check under way is answered first, as one
            // the person asked for. None while a download is under way or
            // ready, which the card shows; one after a download that
            // failed shows what it finds.
            Action::CheckForUpdates => {
                if self.updates.install.busy() {
                    return;
                }
                self.updates.install = Install::Idle;
                match &mut self.updates.phase {
                    Phase::Checking { asked_for, .. } => *asked_for = true,
                    Phase::Idle | Phase::Failed { .. } => self.start_update_check(true),
                }
            }
            Action::DownloadUpdate => self.download_update(),
            Action::CancelDownload => {
                if let Install::Downloading(_) = self.updates.install {
                    self.updates.install = Install::Idle;
                    self.backend
                        .send(Command::Updates(UpdatesCommand::CancelDownload));
                }
            }
            Action::RestartToUpdate => self.restart_to_update(),
            Action::RestartWhenDone => {
                if let Install::Held { when_done, .. } = &mut self.updates.install {
                    *when_done = true;
                }
                self.tick_restart();
            }
            // Back to Ready. Where the helper waits already, a quit in its
            // minute still installs: Ready says so, then the minute ends it.
            Action::DontRestart => {
                if let Install::Held {
                    version, handed, ..
                } = &self.updates.install
                {
                    self.updates.install = Install::Ready {
                        version: version.clone(),
                        handed: *handed,
                    };
                }
            }
            // Ready stays: the next Restart now installs it.
            Action::UpdateLater => self.close_settings(),
            Action::ShowWork => self.show_work(),
            Action::UpdateNow => self.update_now(),
            Action::AnswerUpdateQuestion(yes) => self.answer_update_question(yes),
            Action::ReportUpdateFailure => {
                if let Install::Failed { version, failure } = &self.updates.install {
                    let said = failure.with_parts(failure.said());
                    self.url_to_open = Some(download::issue_url(version, failure, &said));
                }
            }
            // The schedule's next check waits an interval from it, as
            // from one that failed.
            Action::CancelUpdateCheck => {
                self.updates.ask_after_check = false;
                if self.updates.checking() {
                    self.updates.phase = Phase::Idle;
                    self.updates.tried = Some(self.now_secs());
                }
            }
            // Only what the card offers to skip: never a version that
            // downloads, waits to install or failed to.
            Action::SkipUpdate => {
                if self.updates.install != Install::Idle {
                    return;
                }
                let skip = self.settings.update_skip.as_deref();
                let Some(version) = self.updates.offer(skip).map(|offer| offer.version.clone())
                else {
                    return;
                };
                self.updates.asked_offer = None;
                self.change_settings(|settings| settings.update_skip = Some(version));
            }
            Action::ShowReleaseNotes => {
                let skip = self.settings.update_skip.as_deref();
                let Some(offer) = self.updates.offer(skip).cloned() else {
                    return;
                };
                self.backend
                    .send(Command::Updates(UpdatesCommand::ShowNotes {
                        path: self.dirs.release_notes_file(&offer.version),
                        text: notes_text(&offer),
                        url: offer.url,
                    }));
            }
            _ => {}
        }
    }

    pub(super) fn updates_event(&mut self, event: UpdatesEvent) {
        match event {
            UpdatesEvent::Detected(installed) => {
                self.updates.installed = Some(installed);
                if let Some((asked, result)) = self.updates.held.take() {
                    self.updates_event(UpdatesEvent::Checked { asked, result });
                }
            }
            UpdatesEvent::NotesUnshown { url } => self.url_to_open = Some(url),
            UpdatesEvent::Progress {
                asked,
                received,
                total,
                after,
            } => {
                if let Some(downloading) = self.downloading(asked) {
                    downloading.progressed(received, total, after);
                }
            }
            UpdatesEvent::Waiting { asked, waiting } => {
                if let Some(downloading) = self.downloading(asked) {
                    downloading.waiting = waiting;
                }
            }
            UpdatesEvent::Downloaded { asked, result } => {
                let Some(downloading) = self.downloading(asked) else {
                    return;
                };
                let version = downloading.version.clone();
                let then_restart = downloading.then_restart;
                match result {
                    Ok(()) => {
                        self.updates.install = Install::Ready {
                            version,
                            handed: None,
                        };
                        if then_restart {
                            self.restart_to_update();
                        }
                    }
                    Err(failure) => self.updates.install = Install::Failed { version, failure },
                }
            }
            UpdatesEvent::Handed { result, at } => self.handed(result, at),
            UpdatesEvent::Checked { asked, result } => {
                let Phase::Checking {
                    asked: waited,
                    asked_for,
                } = self.updates.phase
                else {
                    return;
                };
                if waited != asked {
                    return;
                }
                // The detection is under way (a demo has none): the card
                // says Checking until it answers.
                if self.updates_scheduled && self.updates.installed.is_none() {
                    self.updates.held = Some((asked, result));
                    return;
                }
                let now = self.now_secs();
                match result {
                    Ok(latest) => self.checked(&latest, asked_for, now),
                    Err(failed) => self.check_failed(failed, asked_for, now),
                }
            }
        }
    }

    /// The download under way, where it is the one `asked`: an answer to
    /// any other is dropped.
    fn downloading(&mut self, asked: u64) -> Option<&mut Downloading> {
        match &mut self.updates.install {
            Install::Downloading(downloading) if downloading.asked == asked => Some(downloading),
            _ => None,
        }
    }

    /// Starts downloading the version offered, where this copy can
    /// install it and no download is under way or ready.
    fn download_update(&mut self) {
        if self.updates.install.busy() {
            return;
        }
        let skip = self.settings.update_skip.as_deref();
        let Some(offer) = self.updates.installable(skip) else {
            return;
        };
        let version = offer.version.clone();
        let url = offer.url.clone();
        let total = offer.download.as_ref().map_or(0, |(_, size)| *size);
        let asked = self.next_id();
        self.updates.install =
            Install::Downloading(Downloading::new(asked, version.clone(), total));
        self.backend
            .send(Command::Updates(UpdatesCommand::Download {
                asked,
                version,
                url,
            }));
    }

    /// `:update`: the pane, checking. In the terminal look a version this
    /// copy can install is asked about in the footer: the one downloading
    /// or ready at once, else the one the check finds.
    fn update_now(&mut self) {
        self.apply(Action::ShowUpdates);
        if !self.look.terminal {
            return;
        }
        let version = match &self.updates.install {
            Install::Downloading(downloading) => Some(downloading.version.clone()),
            Install::Ready { version, .. } => Some(version.clone()),
            Install::Held { .. } | Install::Restarting { .. } => None,
            Install::Idle | Install::Failed { .. } => {
                self.updates.ask_after_check = self.updates.checking();
                None
            }
        };
        if let Some(version) = version {
            self.ask_to_install(version);
        }
    }

    /// Asks in the Updates pane's footer whether to install `version` and
    /// restart.
    fn ask_to_install(&mut self, version: String) {
        if let Some(Dialog::Settings(dialog)) = &mut self.dialog
            && dialog.pane == SettingsPane::Updates
        {
            dialog.update_question = Some(version);
        }
    }

    /// The footer's answer: yes downloads, then restarts once the
    /// download is ready, through the same check for work as Restart now.
    fn answer_update_question(&mut self, yes: bool) {
        let asked = match &mut self.dialog {
            Some(Dialog::Settings(dialog)) => dialog.update_question.take(),
            _ => None,
        };
        if asked.is_none() || !yes {
            return;
        }
        if let Install::Idle | Install::Failed { .. } = self.updates.install {
            self.download_update();
        }
        match &mut self.updates.install {
            Install::Downloading(downloading) => downloading.then_restart = true,
            Install::Ready { .. } => self.restart_to_update(),
            Install::Idle
            | Install::Held { .. }
            | Install::Restarting { .. }
            | Install::Failed { .. } => {}
        }
    }

    /// Whether the app may quit for the restart now: no window has work
    /// open, and no dialog but Settings is up, which a quit would drop.
    fn may_restart(&self) -> bool {
        matches!(self.dialog, None | Some(Dialog::Settings(_)))
            && self.work_in_progress().is_empty()
    }

    /// Restart now: the restart goes ahead where it may. Otherwise the
    /// card lists the work, in Settings › Updates, and waits for Restart
    /// when done. Where only a dialog Settings cannot open over is in the
    /// way (`:update`'s yes, a download that turned ready under quick
    /// open), the restart waits for it to close, armed.
    fn restart_to_update(&mut self) {
        let (version, handed) = match &self.updates.install {
            Install::Ready { version, handed }
            | Install::Held {
                version, handed, ..
            } => (version.clone(), *handed),
            _ => return,
        };
        // Too late for a quit to land inside the helper's minute: it has
        // as good as given up.
        if handed.is_some_and(|at| at.elapsed() >= crate::update::QUIT_IN_TIME) {
            self.not_installed(version, Failure::WorkBegan);
            return;
        }
        if self.may_restart() {
            self.go_restart(version, handed);
            return;
        }
        if let Install::Ready { .. } = self.updates.install {
            self.updates.install = Install::Held {
                version,
                when_done: self.work_in_progress().is_empty(),
                handed,
            };
        }
        let shown = matches!(
            &self.dialog,
            Some(Dialog::Settings(dialog)) if dialog.pane == SettingsPane::Updates
        );
        if !shown {
            self.apply(Action::ShowSettingsPane(SettingsPane::Updates));
        }
    }

    /// The restart, now that nothing is in its way: handed over, or, where
    /// the helper waits already, the quit it waits for.
    fn go_restart(&mut self, version: String, handed: Option<std::time::Instant>) {
        match handed {
            Some(_) => self.closing = true,
            None => self.restart(version),
        }
    }

    /// Read after every frame's actions, and in `logic`: Restart when done
    /// goes ahead once nothing is listed; a list that emptied without it
    /// is Ready again; and the helper's minute, where it waits, ends in
    /// Couldn't install.
    pub(super) fn tick_restart(&mut self) {
        let (version, when_done, handed) = match &self.updates.install {
            Install::Held {
                version,
                when_done,
                handed,
            } => (version.clone(), *when_done, *handed),
            Install::Ready {
                version,
                handed: Some(at),
            } => (version.clone(), false, Some(*at)),
            _ => return,
        };
        if handed.is_some_and(|at| at.elapsed() >= crate::update::HELPER_WAITS) {
            self.not_installed(version, Failure::WorkBegan);
            return;
        }
        let Install::Held { .. } = self.updates.install else {
            return;
        };
        // A quit begun later would land after the helper gave up.
        let in_time = handed.is_none_or(|at| at.elapsed() < crate::update::QUIT_IN_TIME);
        let empty = self.work_in_progress().is_empty();
        match when_done {
            true if in_time && self.may_restart() => self.go_restart(version, handed),
            true => {}
            false if empty => {
                self.updates.install = Install::Ready { version, handed };
            }
            false => {}
        }
    }

    /// The helper's answer: it stands ready, and the app quits now, unless
    /// work was opened meanwhile; then the card waits for it, armed, while
    /// the helper does. An answer read too late to quit in time (the
    /// window hidden) installs nothing.
    fn handed(&mut self, result: Result<(), String>, at: std::time::Instant) {
        let Install::Restarting { version } = &self.updates.install else {
            return;
        };
        let version = version.clone();
        if result.is_ok() {
            self.updates.handoff_ready = Some(at);
        }
        match result {
            Err(error) => self.not_installed(version, Failure::NotHanded(error)),
            Ok(()) if at.elapsed() >= crate::update::QUIT_IN_TIME => {
                self.not_installed(version, Failure::TooLate);
            }
            Ok(()) if self.may_restart() => self.closing = true,
            Ok(()) => {
                self.updates.install = Install::Held {
                    version,
                    when_done: true,
                    handed: Some(at),
                };
            }
        }
    }

    /// Couldn't install: the relaunch file goes, for no next copy comes.
    fn not_installed(&mut self, version: String, failure: Failure) {
        self.updates.install = Install::Failed { version, failure };
        self.backend
            .send(Command::Updates(UpdatesCommand::ForgetRelaunch {
                path: self.dirs.relaunch_file(),
            }));
    }

    /// At the quit, while a helper may still wait for it, or be handed
    /// the update still: the SQL editors as they are now, for the copy it
    /// starts. The file Restart now wrote is older, and Couldn't install
    /// deleted it. The window calls it as the app quits, before the
    /// backend's flush; queued after Restart now's own write, it lands
    /// after it.
    pub fn save_relaunch(&mut self) {
        let handing = matches!(self.updates.install, Install::Restarting { .. });
        let waits = self
            .updates
            .handoff_ready
            .is_some_and(|at| at.elapsed() < crate::update::HELPER_GIVES_UP);
        if !handing && !waits {
            return;
        }
        let file = StateFile::Relaunch(Some(self.kept_for_relaunch()));
        self.backend.send(Command::Save {
            path: self.dirs.relaunch_file(),
            file,
        });
    }

    /// The helper's answer, taken in `logic` too: with the window hidden
    /// no frame reads the backend, and the helper waits a minute at most.
    /// The other events wait for the frame, in their order.
    pub(super) fn take_handoff(&mut self) {
        if !matches!(self.updates.install, Install::Restarting { .. }) {
            return;
        }
        self.poll_backend();
        let (handed, rest): (Vec<Action>, Vec<Action>) = std::mem::take(&mut self.actions)
            .into_iter()
            .partition(|action| {
                matches!(
                    action,
                    Action::Backend(crate::backend::Event::Updates(UpdatesEvent::Handed { .. }))
                )
            });
        self.actions = rest;
        for action in handed {
            self.apply(action);
        }
    }

    /// Keeps the SQL editors for the next copy, and hands the download to
    /// fastframe-update's helper, which starts that copy with this one's
    /// arguments and [`RELAUNCH`]. The app quits once the helper stands
    /// ready.
    fn restart(&mut self, version: String) {
        let mut arguments: Vec<String> = self
            .launch_arguments
            .iter()
            .filter(|argument| *argument != RELAUNCH)
            .cloned()
            .collect();
        arguments.push(RELAUNCH.to_owned());
        self.backend.send(Command::Updates(UpdatesCommand::Install {
            arguments,
            relaunch: self.kept_for_relaunch(),
            path: self.dirs.relaunch_file(),
        }));
        self.updates.install = Install::Restarting { version };
    }

    /// Each window's SQL editors, in the bar's order, each with its place
    /// among the window's tables and editors, and which of those was in
    /// front: what the session does not keep.
    pub(crate) fn kept_for_relaunch(&self) -> crate::update::relaunch::Relaunch {
        use crate::update::relaunch::{Editor, Relaunch, Window};
        let windows = self
            .open_connections()
            .map(|(_, workspace)| {
                // The tabs a relaunch puts back: the tables and editors.
                let kept: Vec<&Tab> = workspace
                    .tabs
                    .iter()
                    .filter(|open| open.as_object().is_some() || open.as_sql().is_some())
                    .collect();
                Window {
                    connection: workspace.conn_id.0.clone(),
                    editors: kept
                        .iter()
                        .enumerate()
                        .filter_map(|(at, open)| Some((at, open.as_sql()?)))
                        .map(|(at, sql)| Editor {
                            number: sql.number,
                            text: sql.text.clone(),
                            writes: sql.mode == crate::model::RunMode::ReadWrite,
                            at,
                        })
                        .collect(),
                    front: kept
                        .iter()
                        .position(|open| Some(open.id()) == workspace.active_tab),
                }
            })
            .collect();
        Relaunch { windows }
    }

    /// Show them: Settings closes, and the first window with work comes
    /// forward, on the tab that holds it.
    fn show_work(&mut self) {
        let Some(first) = self.work_in_progress().into_iter().next() else {
            return;
        };
        self.close_settings();
        self.window_asks.push(egui::ViewportCommand::Focus);
        self.apply(Action::ActivateConnTab(first.tab));
        self.apply(Action::ActivateTab {
            tab: first.tab,
            id: first.id,
        });
    }

    fn close_settings(&mut self) {
        if let Some(Dialog::Settings(_)) = self.dialog {
            self.apply(Action::CloseDialog);
        }
    }

    /// A check that got its answer: kept as the last check, with the
    /// release it found where that is newer.
    fn checked(&mut self, latest: &release::Latest, asked_for: bool, now: u64) {
        self.updates.phase = Phase::Idle;
        self.updates.tried = None;
        let kind = self.updates.installed.as_ref().and_then(|i| i.kind());
        let os = release::Os::current();
        let current = crate::update::CURRENT;
        let skip = self.settings.update_skip.as_deref();
        let seen = &mut self.updates.seen;
        seen.last_checked = Some(now);
        seen.latest_seen = Some(release::version_of(&latest.tag_name).to_owned());
        // Kept whether it is skipped or not: the person may ask again.
        let found = release::found(latest, current, None, os, kind);
        seen.offer = match &found {
            Found::Newer(offer) => Some(offer.clone()),
            Found::UpToDate => None,
        };
        self.updates.asked_offer = match &found {
            Found::Newer(offer) if asked_for && skip == Some(offer.version.as_str()) => {
                Some(offer.clone())
            }
            Found::Newer(_) | Found::UpToDate => None,
        };
        self.save_updates();
        if std::mem::take(&mut self.updates.ask_after_check) {
            let skip = self.settings.update_skip.as_deref();
            if let Some(offer) = self.updates.installable(skip) {
                let version = offer.version.clone();
                self.ask_to_install(version);
            }
        }
    }

    /// In the terminal look, the status line tells of a version offered,
    /// whether a check found it now or one before: at most once a day.
    fn announce_update(&mut self, now: u64) {
        if !self.look.terminal || !self.updates.seen.may_announce(now) {
            return;
        }
        let skip = self.settings.update_skip.as_deref();
        let Some(version) = self.updates.offer(skip).map(|offer| offer.version.clone()) else {
            return;
        };
        self.updates.seen.announced = Some(now);
        self.update_announcement = Some(version);
        self.save_updates();
    }

    /// A check that got no answer: it does not count as checked, and the
    /// schedule tries again an interval later, whoever asked.
    fn check_failed(&mut self, failed: Failed, asked_for: bool, now: u64) {
        self.updates.ask_after_check = false;
        self.updates.tried = Some(now);
        self.updates.phase = Phase::Failed { failed, asked_for };
    }

    /// Saves what the checks found, on the backend.
    fn save_updates(&mut self) {
        self.backend.send(Command::Save {
            path: self.dirs.updates_file(),
            file: StateFile::Updates(self.updates.seen.clone()),
        });
    }

    /// What the window says once, outside the app's state: the status
    /// line's line of a version available (terminal look), and a page to
    /// open. Taken by the frame that says it.
    pub(crate) fn say_updates(&mut self, ctx: &egui::Context) {
        if let Some(version) = self.update_announcement.take()
            && self.look.terminal
        {
            let layout = crate::keymap::Layout::of(&self.look);
            let key = self
                .keymap
                .key(layout, crate::keymap::Command::CheckForUpdates);
            let words = match key.is_empty() {
                true => ANNOUNCEMENT_KEYLESS,
                false => ANNOUNCEMENT,
            };
            let said = crate::i18n::gettext(self.locale, words)
                .replace("{version}", &version)
                .replace("{key}", key.as_str());
            crate::ui::toast::say_for(ctx, &said, ANNOUNCED_FOR);
        }
        if let Some(url) = self.url_to_open.take() {
            ctx.open_url(egui::OpenUrl::new_tab(url));
        }
    }
}

/// A release's notes as the pager shows them: its version and page over
/// the points the card shows.
fn notes_text(offer: &crate::update::Offer) -> String {
    let mut text = format!("# Tabletist {}\n\n{}\n\n", offer.version, offer.url);
    if offer.body.trim().is_empty() {
        for point in &offer.notes {
            text.push_str(&format!("- {point}\n"));
        }
    } else {
        text.push_str(offer.body.trim());
        text.push('\n');
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::Event;
    use crate::settings::UpdateCheck;
    use crate::testing::Harness;
    use crate::update::check::Failed;

    const DAY: u64 = 24 * 60 * 60;
    /// 2026-10-27 09:12 UTC.
    const NOW: i64 = 1_793_092_320;

    fn harness(check: UpdateCheck, last_checked: Option<u64>) -> Harness {
        let mut harness = Harness::new();
        harness.app.stopped_clock = Some(jiff::Timestamp::from_second(NOW).unwrap());
        harness.app.settings.update_check = check;
        harness.app.updates.seen.last_checked = last_checked;
        harness.app.updates_scheduled = true;
        harness.app.updates.installed = Some(crate::update::Installed::Itself(
            crate::update::release::Kind::Portable,
        ));
        harness.app.backend.sent.clear();
        harness
    }

    fn checks(harness: &Harness) -> Vec<u64> {
        harness
            .app
            .backend
            .sent
            .iter()
            .filter_map(|sent| match sent {
                Command::Updates(UpdatesCommand::Check { asked, .. }) => Some(*asked),
                _ => None,
            })
            .collect()
    }

    fn now() -> u64 {
        u64::try_from(NOW).unwrap()
    }

    fn latest(tag: &str) -> release::Latest {
        serde_json::from_str(&crate::update::check::tests::RELEASE.replace("v0.4.0", tag)).unwrap()
    }

    fn answer(harness: &mut Harness, result: Result<release::Latest, Failed>) {
        let asked = *checks(harness).last().expect("a check");
        harness
            .app
            .apply(Action::Backend(Event::Updates(UpdatesEvent::Checked {
                asked,
                result,
            })));
    }

    #[test]
    fn a_daily_check_runs_on_launch_only_when_it_is_due() {
        let mut due = harness(UpdateCheck::Daily, Some(now() - 2 * DAY));
        due.app.tick_updates();
        assert_eq!(checks(&due).len(), 1);
        // Not twice while it runs.
        due.app.tick_updates();
        assert_eq!(checks(&due).len(), 1);

        let mut early = harness(UpdateCheck::Daily, Some(now() - 3_600));
        let wait = early.app.tick_updates();
        assert!(checks(&early).is_empty());
        assert_eq!(wait, Some(Duration::from_secs(DAY - 3_600)));
    }

    #[test]
    fn a_weekly_check_waits_a_week() {
        let mut harness = harness(UpdateCheck::Weekly, Some(now() - 2 * DAY));
        harness.app.tick_updates();
        assert!(checks(&harness).is_empty());
        harness.app.updates.seen.last_checked = Some(now() - 8 * DAY);
        harness.app.tick_updates();
        assert_eq!(checks(&harness).len(), 1);
    }

    #[test]
    fn only_when_i_ask_checks_only_when_asked() {
        let mut harness = harness(UpdateCheck::Never, None);
        assert_eq!(harness.app.tick_updates(), None);
        assert!(checks(&harness).is_empty());
        harness.app.apply(Action::CheckForUpdates);
        assert_eq!(checks(&harness).len(), 1);
    }

    /// A demo, and the tests, ask GitHub nothing until told to.
    #[test]
    fn nothing_is_checked_before_the_window_watches_for_updates() {
        let mut harness = harness(UpdateCheck::Daily, None);
        harness.app.updates_scheduled = false;
        harness.app.tick_updates();
        assert!(checks(&harness).is_empty());
    }

    #[test]
    fn a_scheduled_check_waits_for_a_running_query() {
        let mut harness = harness(UpdateCheck::Daily, None);
        let tab = harness.connect_fake();
        let sql = harness.add_sql_tab(tab);
        let workspace = harness.app.workspace_mut(tab).unwrap();
        let editor = workspace
            .tabs
            .iter_mut()
            .filter_map(|open| open.as_sql_mut())
            .find(|open| open.id == sql)
            .unwrap();
        editor.text = "SELECT * FROM book_covers".into();
        harness.app.apply(Action::RunSql {
            tab,
            sql_tab: sql,
            all: true,
        });
        harness
            .app
            .backend
            .sent
            .retain(|sent| matches!(sent, Command::RunSql { .. }));
        assert!(harness.app.query_runs());
        assert_eq!(harness.app.tick_updates(), Some(Duration::from_secs(1)));
        assert!(checks(&harness).is_empty());
        // The query ends: the check runs right after.
        let outcome = crate::testing::script_outcome(vec![crate::testing::rows_outcome(3)]);
        harness.answer_sql(Ok(outcome), None);
        assert!(!harness.app.query_runs());
        harness.app.tick_updates();
        assert_eq!(checks(&harness).len(), 1);
    }

    #[test]
    fn a_check_that_finds_a_newer_release_keeps_it_and_when_it_ran() {
        let mut harness = harness(UpdateCheck::Daily, None);
        harness.app.tick_updates();
        answer(&mut harness, Ok(latest("v99.0.0")));
        let updates = &harness.app.updates;
        assert_eq!(updates.phase, Phase::Idle);
        assert_eq!(updates.seen.last_checked, Some(now()));
        assert_eq!(updates.seen.latest_seen.as_deref(), Some("99.0.0"));
        let offer = updates.offer(None).expect("offered");
        assert_eq!(offer.version, "99.0.0");
        assert_eq!(offer.notes.len(), 4);
        // Saved in the state folder.
        let saved = harness.app.backend.sent.iter().any(|sent| {
            matches!(sent, Command::Save { path, file: StateFile::Updates(seen) }
                if *path == harness.app.dirs.updates_file() && seen.last_checked == Some(now()))
        });
        assert!(saved);
    }

    #[test]
    fn a_timeout_or_no_connection_fails_the_check_and_is_no_check() {
        for failed in [
            Failed::TimedOut(Duration::from_secs(10)),
            Failed::Unreachable,
        ] {
            let before = now() - 2 * DAY;
            let mut harness = harness(UpdateCheck::Daily, Some(before));
            harness.app.tick_updates();
            answer(&mut harness, Err(failed.clone()));
            assert_eq!(
                harness.app.updates.phase,
                Phase::Failed {
                    failed,
                    asked_for: false
                }
            );
            assert_eq!(harness.app.updates.seen.last_checked, Some(before));
            // Tried again at the next interval, not at once.
            assert_eq!(harness.app.tick_updates(), Some(Duration::from_secs(DAY)));
            assert_eq!(checks(&harness).len(), 1);
        }
    }

    #[test]
    fn a_cancelled_check_drops_what_it_brings() {
        let mut harness = harness(UpdateCheck::Never, None);
        harness.app.apply(Action::CheckForUpdates);
        harness.app.apply(Action::CancelUpdateCheck);
        answer(&mut harness, Ok(latest("v99.0.0")));
        assert_eq!(harness.app.updates.phase, Phase::Idle);
        assert_eq!(harness.app.updates.seen.last_checked, None);
        assert!(harness.app.updates.offer(None).is_none());
    }

    #[test]
    fn skip_this_version_hides_it_until_a_check_is_asked_for() {
        let mut harness = harness(UpdateCheck::Daily, None);
        harness.app.tick_updates();
        answer(&mut harness, Ok(latest("v99.0.0")));
        harness.app.apply(Action::SkipUpdate);
        assert_eq!(harness.app.settings.update_skip.as_deref(), Some("99.0.0"));
        assert!(
            harness
                .app
                .settings_file
                .text
                .contains("[updates]\nskip = \"99.0.0\"")
        );
        let skip = harness.app.settings.update_skip.clone();
        assert!(harness.app.updates.offer(skip.as_deref()).is_none());
        // A scheduled check finds it again, and does not offer it.
        harness.app.updates.seen.last_checked = Some(now() - 2 * DAY);
        harness.app.tick_updates();
        answer(&mut harness, Ok(latest("v99.0.0")));
        assert!(harness.app.updates.offer(skip.as_deref()).is_none());
        // A newer one is offered.
        harness.app.updates.seen.last_checked = Some(now() - 2 * DAY);
        harness.app.tick_updates();
        answer(&mut harness, Ok(latest("v99.0.1")));
        assert!(harness.app.updates.offer(skip.as_deref()).is_some());
        // A check asked for offers the skipped one.
        harness.app.apply(Action::CheckForUpdates);
        answer(&mut harness, Ok(latest("v99.0.0")));
        assert_eq!(
            harness
                .app
                .updates
                .offer(skip.as_deref())
                .map(|offer| offer.version.as_str()),
            Some("99.0.0")
        );
    }

    #[test]
    fn the_status_line_tells_of_a_version_at_most_once_a_day() {
        let mut harness = harness(UpdateCheck::Daily, None);
        harness.set_look(crate::theme::Look::omarchy());
        harness.app.tick_updates();
        answer(&mut harness, Ok(latest("v99.0.0")));
        harness.app.tick_updates();
        assert_eq!(
            harness.app.update_announcement.take().as_deref(),
            Some("99.0.0")
        );
        assert_eq!(harness.app.updates.seen.announced, Some(now()));
        // A day has not passed.
        harness.app.updates.seen.last_checked = Some(now() - 2 * DAY);
        harness.app.tick_updates();
        answer(&mut harness, Ok(latest("v99.0.0")));
        harness.app.tick_updates();
        assert_eq!(harness.app.update_announcement, None);
    }

    /// A version found before this launch is told of too, and a check
    /// the person asked for counts as any other.
    #[test]
    fn the_status_line_tells_of_a_version_kept_from_before() {
        let mut harness = harness(UpdateCheck::Weekly, Some(now() - DAY));
        harness.set_look(crate::theme::Look::omarchy());
        harness.app.updates.seen.offer = Some(release::offer(
            &latest("v99.0.0"),
            release::Os::current(),
            None,
        ));
        harness.app.updates.seen.announced = Some(now() - 2 * DAY);
        harness.app.tick_updates();
        assert!(checks(&harness).is_empty());
        assert_eq!(harness.app.update_announcement.as_deref(), Some("99.0.0"));
    }

    /// The macOS look has no status line: nothing is told, and nothing
    /// noted as told.
    #[test]
    fn the_macos_look_tells_nothing() {
        let mut harness = harness(UpdateCheck::Daily, None);
        harness.set_look(crate::theme::Look::macos());
        harness.app.tick_updates();
        answer(&mut harness, Ok(latest("v99.0.0")));
        harness.app.tick_updates();
        assert_eq!(harness.app.update_announcement, None);
        assert_eq!(harness.app.updates.seen.announced, None);
    }

    /// The check waits for the detection, so the offer keeps the asset of
    /// this copy's kind.
    #[test]
    fn the_scheduled_check_waits_to_know_how_this_copy_was_installed() {
        let mut harness = harness(UpdateCheck::Daily, None);
        harness.app.updates.installed = None;
        harness.app.tick_updates();
        assert!(checks(&harness).is_empty());
        harness
            .app
            .apply(Action::Backend(Event::Updates(UpdatesEvent::Detected(
                crate::update::Installed::Itself(release::Kind::Portable),
            ))));
        harness.app.tick_updates();
        assert_eq!(checks(&harness).len(), 1);
    }

    #[test]
    fn the_notes_go_to_the_pager_whole() {
        let mut harness = harness(UpdateCheck::Never, None);
        harness.app.apply(Action::CheckForUpdates);
        answer(&mut harness, Ok(latest("v99.0.0")));
        harness.app.apply(Action::ShowReleaseNotes);
        let shown = harness.app.backend.sent.iter().find_map(|sent| match sent {
            Command::Updates(UpdatesCommand::ShowNotes { path, text, url }) => {
                Some((path.clone(), text.clone(), url.clone()))
            }
            _ => None,
        });
        let (path, text, url) = shown.expect("shown");
        assert_eq!(path, harness.app.dirs.release_notes_file("99.0.0"));
        assert!(text.starts_with("# Tabletist 99.0.0\n"), "{text}");
        // All five points, not the card's four.
        assert!(text.contains("- Settings › Updates"), "{text}");
        assert!(url.ends_with("/v99.0.0"), "{url}");
        // No terminal: the page opens.
        harness.app.apply(Action::Backend(Event::Updates(
            UpdatesEvent::NotesUnshown { url: url.clone() },
        )));
        assert_eq!(harness.app.url_to_open.as_deref(), Some(url.as_str()));
    }

    /// Cancel holds: the schedule's next check waits an interval.
    #[test]
    fn a_cancelled_scheduled_check_waits_an_interval() {
        let mut harness = harness(UpdateCheck::Daily, None);
        harness.app.tick_updates();
        assert_eq!(checks(&harness).len(), 1);
        harness.app.apply(Action::CancelUpdateCheck);
        assert!(!harness.app.updates.checking());
        assert_eq!(harness.app.tick_updates(), Some(Duration::from_secs(DAY)));
        assert_eq!(checks(&harness).len(), 1);
    }

    /// A check that failed holds the schedule an interval, though Check
    /// now joined it, and so does one the person asked for: the error
    /// they see stays.
    #[test]
    fn a_failed_check_holds_the_schedule_whoever_asked() {
        let mut harness = harness(UpdateCheck::Daily, None);
        harness.app.tick_updates();
        harness.app.apply(Action::CheckForUpdates);
        answer(&mut harness, Err(Failed::Unreachable));
        assert_eq!(harness.app.tick_updates(), Some(Duration::from_secs(DAY)));
        assert_eq!(checks(&harness).len(), 1);

        let mut harness = harness_detecting(UpdateCheck::Daily);
        harness.app.apply(Action::CheckForUpdates);
        answer(&mut harness, Err(Failed::Unreachable));
        harness
            .app
            .apply(Action::Backend(Event::Updates(UpdatesEvent::Detected(
                crate::update::Installed::Itself(release::Kind::Portable),
            ))));
        harness.app.tick_updates();
        assert_eq!(checks(&harness).len(), 1);
        assert!(matches!(
            harness.app.updates.phase,
            Phase::Failed {
                asked_for: true,
                ..
            }
        ));
    }

    /// A harness whose installation is not known yet.
    /// A check asked for before the detection answers keeps this copy's
    /// download: its answer waits for the detection, and says Checking.
    #[test]
    fn a_check_answered_before_the_detection_keeps_this_copy_s_download() {
        let mut harness = harness_detecting(UpdateCheck::Never);
        harness.app.apply(Action::CheckForUpdates);
        let latest = latest("v99.0.0");
        answer(&mut harness, Ok(latest.clone()));
        assert!(harness.app.updates.checking());
        assert_eq!(harness.app.updates.seen.offer, None);
        let kind = release::Kind::AppImage;
        harness
            .app
            .apply(Action::Backend(Event::Updates(UpdatesEvent::Detected(
                crate::update::Installed::Itself(kind),
            ))));
        assert!(!harness.app.updates.checking());
        let os = release::Os::current();
        let Found::Newer(expected) =
            release::found(&latest, crate::update::CURRENT, None, os, Some(kind))
        else {
            panic!("newer");
        };
        let offer = harness.app.updates.seen.offer.clone().expect("offered");
        assert_eq!(offer.download, expected.download);
    }

    /// An answer held for the detection is dropped where the check was
    /// cancelled meanwhile.
    #[test]
    fn a_held_answer_to_a_cancelled_check_is_dropped() {
        let mut harness = harness_detecting(UpdateCheck::Never);
        harness.app.apply(Action::CheckForUpdates);
        answer(&mut harness, Ok(latest("v99.0.0")));
        harness.app.apply(Action::CancelUpdateCheck);
        harness
            .app
            .apply(Action::Backend(Event::Updates(UpdatesEvent::Detected(
                crate::update::Installed::Itself(release::Kind::AppImage),
            ))));
        assert!(!harness.app.updates.checking());
        assert_eq!(harness.app.updates.seen.offer, None);
    }

    fn harness_detecting(check: UpdateCheck) -> Harness {
        let mut harness = harness(check, None);
        harness.app.updates.installed = None;
        harness
    }

    /// Check now while a scheduled check runs makes it one asked for: it
    /// offers the version skipped.
    #[test]
    fn check_now_during_a_scheduled_check_counts_as_asked() {
        let mut harness = harness(UpdateCheck::Daily, None);
        harness.app.settings.update_skip = Some("99.0.0".into());
        harness.app.tick_updates();
        harness.app.apply(Action::CheckForUpdates);
        assert_eq!(checks(&harness).len(), 1);
        answer(&mut harness, Ok(latest("v99.0.0")));
        assert!(harness.app.updates.offer(Some("99.0.0")).is_some());
    }

    // The download and the restart.

    /// A harness whose copy replaces itself, offered 99.0.0 with its build.
    fn offered() -> Harness {
        let mut harness = harness(UpdateCheck::Never, Some(now()));
        let mut offer = release::offer(&latest("v99.0.0"), release::Os::current(), None);
        offer
            .download
            .get_or_insert(("tabletist-v99.0.0.tar.gz".into(), 31_200_000));
        harness.app.updates.seen.offer = Some(offer);
        harness
    }

    fn downloads(harness: &Harness) -> Vec<(u64, String)> {
        harness
            .app
            .backend
            .sent
            .iter()
            .filter_map(|sent| match sent {
                Command::Updates(UpdatesCommand::Download { asked, version, .. }) => {
                    Some((*asked, version.clone()))
                }
                _ => None,
            })
            .collect()
    }

    fn installs(harness: &Harness) -> Vec<Vec<String>> {
        harness
            .app
            .backend
            .sent
            .iter()
            .filter_map(|sent| match sent {
                Command::Updates(UpdatesCommand::Install { arguments, .. }) => {
                    Some(arguments.clone())
                }
                _ => None,
            })
            .collect()
    }

    /// What the last Install keeps for the next copy.
    fn kept(harness: &Harness) -> crate::update::relaunch::Relaunch {
        harness
            .app
            .backend
            .sent
            .iter()
            .rev()
            .find_map(|sent| match sent {
                Command::Updates(UpdatesCommand::Install { relaunch, path, .. })
                    if *path == harness.app.dirs.relaunch_file() =>
                {
                    Some(relaunch.clone())
                }
                _ => None,
            })
            .expect("an Install")
    }

    fn forgot(harness: &Harness) -> bool {
        harness.app.backend.sent.iter().any(|sent| {
            matches!(sent, Command::Updates(UpdatesCommand::ForgetRelaunch { path })
                if *path == harness.app.dirs.relaunch_file())
        })
    }

    fn handed(harness: &mut Harness, result: Result<(), String>, ago: Duration) {
        let at = std::time::Instant::now().checked_sub(ago).unwrap();
        event(harness, UpdatesEvent::Handed { result, at });
    }

    fn ready_state() -> Install {
        Install::Ready {
            version: "99.0.0".into(),
            handed: None,
        }
    }

    fn event(harness: &mut Harness, event: UpdatesEvent) {
        harness.app.apply(Action::Backend(Event::Updates(event)));
    }

    /// Downloads 99.0.0 and says it verified: Ready. Windows downloads
    /// nothing: it is put there.
    fn ready(harness: &mut Harness) {
        if !crate::update::DOWNLOADS {
            harness.app.updates.install = ready_state();
            return;
        }
        harness.app.apply(Action::DownloadUpdate);
        let (asked, _) = downloads(harness)[0].clone();
        event(
            harness,
            UpdatesEvent::Downloaded {
                asked,
                result: Ok(()),
            },
        );
        assert_eq!(harness.app.updates.install, ready_state());
    }

    #[test]
    fn download_and_install_downloads_the_version_offered() {
        // Windows downloads nothing (row 16).
        if !crate::update::DOWNLOADS {
            return;
        }
        let mut harness = offered();
        harness.app.apply(Action::DownloadUpdate);
        let sent = downloads(&harness);
        assert_eq!(sent.len(), 1);
        assert_eq!(sent[0].1, "99.0.0");
        let Install::Downloading(downloading) = &harness.app.updates.install else {
            panic!("{:?}", harness.app.updates.install);
        };
        // The size its release gives the build.
        let size = harness
            .app
            .updates
            .seen
            .offer
            .as_ref()
            .unwrap()
            .download
            .as_ref()
            .unwrap()
            .1;
        assert_eq!(downloading.total, size);
        // Once at a time.
        harness.app.apply(Action::DownloadUpdate);
        assert_eq!(downloads(&harness).len(), 1);
    }

    #[test]
    fn a_copy_that_cannot_install_downloads_nothing() {
        // Something else installed it.
        let mut elsewhere = offered();
        elsewhere.app.updates.installed = Some(crate::update::Installed::Elsewhere(
            crate::update::Managed {
                said: "Installed with Homebrew, so Homebrew updates it",
                command: Some("brew upgrade --cask tabletist"),
            },
        ));
        elsewhere.app.apply(Action::DownloadUpdate);
        assert!(downloads(&elsewhere).is_empty());
        // The release has no build for this computer.
        let mut no_build = offered();
        if let Some(offer) = &mut no_build.app.updates.seen.offer {
            offer.download = None;
        }
        no_build.app.apply(Action::DownloadUpdate);
        assert!(downloads(&no_build).is_empty());
        // Not known yet how this copy was installed.
        let mut unknown = offered();
        unknown.app.updates.installed = None;
        unknown.app.apply(Action::DownloadUpdate);
        assert!(downloads(&unknown).is_empty());
    }

    #[test]
    fn the_download_counts_its_bytes_and_the_time_left() {
        // Windows downloads nothing (row 16).
        if !crate::update::DOWNLOADS {
            return;
        }
        let mut harness = offered();
        harness.app.apply(Action::DownloadUpdate);
        let (asked, _) = downloads(&harness)[0].clone();
        for (secs, received) in [(1, 2_000_000), (3, 6_000_000), (6, 12_000_000)] {
            event(
                &mut harness,
                UpdatesEvent::Progress {
                    asked,
                    received,
                    total: 31_200_000,
                    after: Duration::from_secs(secs),
                },
            );
        }
        // An answer to another download is dropped.
        event(
            &mut harness,
            UpdatesEvent::Progress {
                asked: asked + 100,
                received: 30_000_000,
                total: 31_200_000,
                after: Duration::from_secs(7),
            },
        );
        let Install::Downloading(downloading) = &harness.app.updates.install else {
            panic!("downloading");
        };
        assert_eq!(downloading.received, 12_000_000);
        // 2 MB a second over the last five seconds, 19.2 MB to go.
        assert_eq!(downloading.time_left().map(|left| left.as_secs()), Some(9));
        assert!(!downloading.verifying());
        event(
            &mut harness,
            UpdatesEvent::Progress {
                asked,
                received: 31_200_000,
                total: 31_200_000,
                after: Duration::from_secs(16),
            },
        );
        let Install::Downloading(downloading) = &harness.app.updates.install else {
            panic!("downloading");
        };
        assert!(downloading.verifying());
    }

    #[test]
    fn the_download_says_when_it_waits_for_the_network() {
        // Windows downloads nothing (row 16).
        if !crate::update::DOWNLOADS {
            return;
        }
        let mut harness = offered();
        harness.app.apply(Action::DownloadUpdate);
        let (asked, _) = downloads(&harness)[0].clone();
        event(
            &mut harness,
            UpdatesEvent::Waiting {
                asked,
                waiting: true,
            },
        );
        assert!(matches!(
            &harness.app.updates.install,
            Install::Downloading(downloading) if downloading.waiting
        ));
        event(
            &mut harness,
            UpdatesEvent::Waiting {
                asked,
                waiting: false,
            },
        );
        assert!(matches!(
            &harness.app.updates.install,
            Install::Downloading(downloading) if !downloading.waiting
        ));
    }

    #[test]
    fn a_download_that_failed_says_why_and_offers_another() {
        // Windows downloads nothing (row 16).
        if !crate::update::DOWNLOADS {
            return;
        }
        let failure = Failure::Unverified(download::Unverified::Signature);
        let mut harness = offered();
        harness.app.apply(Action::DownloadUpdate);
        let (asked, _) = downloads(&harness)[0].clone();
        event(
            &mut harness,
            UpdatesEvent::Downloaded {
                asked,
                result: Err(failure.clone()),
            },
        );
        assert_eq!(
            harness.app.updates.install,
            Install::Failed {
                version: "99.0.0".into(),
                failure
            }
        );
        // Download again.
        harness.app.apply(Action::DownloadUpdate);
        assert_eq!(downloads(&harness).len(), 2);
    }

    #[test]
    fn report_on_github_opens_an_issue_filled_in() {
        // Windows downloads nothing (row 16).
        if !crate::update::DOWNLOADS {
            return;
        }
        let mut harness = offered();
        harness.app.apply(Action::DownloadUpdate);
        let (asked, _) = downloads(&harness)[0].clone();
        event(
            &mut harness,
            UpdatesEvent::Downloaded {
                asked,
                result: Err(Failure::Unverified(download::Unverified::Hash)),
            },
        );
        harness.app.apply(Action::ReportUpdateFailure);
        let opened = harness.app.url_to_open.clone().expect("a page to open");
        assert!(
            opened.starts_with("https://github.com/igor-alexandrov/tabletist/issues/new?"),
            "{opened}"
        );
        assert!(opened.contains("99.0.0"), "{opened}");
    }

    #[test]
    fn cancel_stops_the_download_and_drops_its_answer() {
        // Windows downloads nothing (row 16).
        if !crate::update::DOWNLOADS {
            return;
        }
        let mut harness = offered();
        harness.app.apply(Action::DownloadUpdate);
        let (asked, _) = downloads(&harness)[0].clone();
        harness.app.apply(Action::CancelDownload);
        assert_eq!(harness.app.updates.install, Install::Idle);
        let cancelled = harness
            .app
            .backend
            .sent
            .iter()
            .any(|sent| matches!(sent, Command::Updates(UpdatesCommand::CancelDownload)));
        assert!(cancelled);
        event(
            &mut harness,
            UpdatesEvent::Downloaded {
                asked,
                result: Ok(()),
            },
        );
        assert_eq!(harness.app.updates.install, Install::Idle);
    }

    /// No check runs while a download runs or waits to be installed:
    /// the card shows that.
    #[test]
    fn no_check_runs_while_a_download_is_under_way_or_ready() {
        // Windows downloads nothing (row 16).
        if !crate::update::DOWNLOADS {
            return;
        }
        let mut harness = offered();
        harness.app.settings.update_check = UpdateCheck::Daily;
        harness.app.updates.seen.last_checked = Some(now() - 2 * DAY);
        harness.app.apply(Action::DownloadUpdate);
        harness.app.tick_updates();
        harness.app.apply(Action::CheckForUpdates);
        assert!(checks(&harness).is_empty());
    }

    #[test]
    fn restart_now_with_nothing_open_hands_the_update_over_and_quits() {
        let mut harness = offered();
        harness.app.launch_arguments = vec!["--verbose".into()];
        let tab = harness.connect_fake();
        let sql = harness.add_sql_tab(tab);
        harness.app.sql_tab_mut(tab, sql).unwrap().text = "SELECT * FROM book_covers".into();
        ready(&mut harness);
        harness.app.apply(Action::RestartToUpdate);
        // The launch's arguments, then the relaunch's flag.
        assert_eq!(installs(&harness), [vec!["--verbose", RELAUNCH]]);
        // The SQL editors go with it, for the next copy, in front.
        let kept = kept(&harness);
        assert_eq!(kept.windows.len(), 1);
        assert_eq!(kept.windows[0].editors[0].text, "SELECT * FROM book_covers");
        assert!(!harness.app.closing);
        handed(&mut harness, Ok(()), Duration::ZERO);
        assert!(harness.app.closing);
        assert!(!forgot(&harness));
    }

    /// The helper that did not start leaves this copy running, and says
    /// why.
    #[test]
    fn a_helper_that_did_not_start_leaves_the_app_open() {
        let mut harness = offered();
        ready(&mut harness);
        harness.app.apply(Action::RestartToUpdate);
        handed(&mut harness, Err("no helper".into()), Duration::ZERO);
        assert!(!harness.app.closing);
        // No next copy reads the relaunch file: it goes.
        assert!(forgot(&harness));
        assert_eq!(
            harness.app.updates.install,
            Install::Failed {
                version: "99.0.0".into(),
                failure: Failure::NotHanded("no helper".into())
            }
        );
    }

    /// The relaunch's flag is not given twice to a copy started with it.
    #[test]
    fn a_relaunched_copy_restarts_with_the_flag_once() {
        let mut harness = offered();
        harness.app.launch_arguments = vec![RELAUNCH.into()];
        ready(&mut harness);
        harness.app.apply(Action::RestartToUpdate);
        assert_eq!(installs(&harness), [vec![RELAUNCH]]);
    }

    /// `kind` of the first row of `book_covers` changed, and not saved.
    fn unsaved_edit(harness: &mut Harness) -> (crate::model::ConnTabId, crate::model::TabId) {
        let (tab, id) = harness.book_covers();
        harness.app.workspace_mut(tab).unwrap().name = "Bookshop".into();
        harness.app.apply(Action::EditCell {
            tab,
            id,
            cell: crate::model::CellPos { row: 0, col: 2 },
            start: crate::model::EditStart::Replace("ebook".into()),
        });
        harness.app.apply(Action::CommitEdit {
            tab,
            id,
            then: crate::model::Advance::Stay,
        });
        (tab, id)
    }

    #[test]
    fn restart_now_with_unsaved_edits_lists_them_and_waits() {
        let mut harness = offered();
        let (tab, id) = unsaved_edit(&mut harness);
        ready(&mut harness);
        harness.app.apply(Action::RestartToUpdate);
        assert!(installs(&harness).is_empty());
        assert_eq!(
            harness.app.updates.install,
            Install::Held {
                version: "99.0.0".into(),
                when_done: false,
                handed: None,
            }
        );
        assert_eq!(
            harness.app.work_in_progress(),
            [Work {
                tab,
                id,
                window: "Bookshop · dev".into(),
                what: Pending::Edits {
                    count: 1,
                    table: "book_covers".into()
                },
            }]
        );
        // Shown where the list is.
        assert!(matches!(
            &harness.app.dialog,
            Some(Dialog::Settings(dialog)) if dialog.pane == SettingsPane::Updates
        ));
        // Nothing was dropped.
        let edits = &harness
            .app
            .workspace(tab)
            .unwrap()
            .object_tab(id)
            .unwrap()
            .edits;
        assert!(edits.holds());
    }

    #[test]
    fn restart_when_done_restarts_once_the_work_is_saved() {
        let mut harness = offered();
        let (tab, id) = unsaved_edit(&mut harness);
        ready(&mut harness);
        harness.app.apply(Action::RestartToUpdate);
        harness.app.apply(Action::RestartWhenDone);
        assert!(installs(&harness).is_empty());
        // The save runs: still work.
        harness.app.apply(Action::WriteEdits { tab, id });
        assert_eq!(
            harness.app.work_in_progress()[0].what,
            Pending::Saving {
                table: "book_covers".into()
            }
        );
        harness.app.tick_updates();
        assert!(installs(&harness).is_empty());
        harness.answer_written(Ok(tabletist_db::WriteOutcome::Written {
            inserted: Vec::new(),
            rows: Vec::new(),
            elapsed: Duration::ZERO,
            gone: Vec::new(),
        }));
        // The table reads its page again after the save: work too.
        assert_eq!(
            harness
                .app
                .work_in_progress()
                .iter()
                .map(|work| work.what.clone())
                .collect::<Vec<_>>(),
            [Pending::Loading {
                table: "book_covers".into()
            }]
        );
        harness.app.tick_updates();
        assert!(installs(&harness).is_empty());
        harness.answer_rows(crate::testing::book_covers_page(3));
        assert!(harness.app.work_in_progress().is_empty());
        harness.app.tick_updates();
        assert_eq!(installs(&harness).len(), 1);
    }

    #[test]
    fn dont_restart_goes_back_to_ready() {
        let mut harness = offered();
        let (tab, id) = unsaved_edit(&mut harness);
        ready(&mut harness);
        harness.app.apply(Action::RestartToUpdate);
        harness.app.apply(Action::RestartWhenDone);
        harness.app.apply(Action::DontRestart);
        harness.app.apply(Action::DiscardEdits { tab, id });
        harness.app.tick_updates();
        assert!(installs(&harness).is_empty());
        assert_eq!(harness.app.updates.install, ready_state());
    }

    #[test]
    fn show_them_brings_the_window_with_work_forward_on_its_tab() {
        let mut harness = offered();
        let (tab, id) = unsaved_edit(&mut harness);
        // Another tab in front, then another window.
        let sql = harness.add_sql_tab(tab);
        harness.app.apply(Action::ActivateTab { tab, id: sql });
        harness.app.apply(Action::ShowConnections);
        ready(&mut harness);
        harness.app.apply(Action::RestartToUpdate);
        harness.app.window_asks.clear();
        harness.app.apply(Action::ShowWork);
        assert!(harness.app.dialog.is_none());
        assert_eq!(harness.app.active_tab_id(), tab);
        assert_eq!(harness.app.workspace(tab).unwrap().active_tab, Some(id));
        assert_eq!(harness.app.window_asks, [egui::ViewportCommand::Focus]);
    }

    #[test]
    fn a_run_of_the_sql_editor_is_work_and_a_write_is_named_so() {
        for write in [false, true] {
            let mut harness = offered();
            let tab = harness.connect_fake_as(!write);
            harness.app.workspace_mut(tab).unwrap().name = "Bookshop".into();
            let sql = harness.add_sql_tab(tab);
            let editor = harness.app.sql_tab_mut(tab, sql).unwrap();
            editor.text = match write {
                true => "UPDATE book_covers SET kind = 'print' WHERE id = 1".into(),
                false => "SELECT * FROM book_covers".into(),
            };
            if write {
                editor.mode = crate::model::RunMode::ReadWrite;
            }
            let number = editor.number;
            harness.app.apply(Action::RunSql {
                tab,
                sql_tab: sql,
                all: true,
            });
            let work = harness.app.work_in_progress();
            assert_eq!(work.len(), 1, "{work:?}");
            match write {
                true => assert_eq!(work[0].what, Pending::Writing { query: number }),
                false => assert!(
                    matches!(work[0].what, Pending::Running { query, .. } if query == number)
                ),
            }
            assert_eq!(work[0].window, "Bookshop · dev");
        }
    }

    /// Work opened while the helper got ready is never dropped: the card
    /// waits for it, armed, while the helper does, and the quit comes as
    /// soon as it is done.
    #[test]
    fn work_opened_while_the_helper_got_ready_is_waited_for() {
        let mut harness = offered();
        ready(&mut harness);
        harness.app.apply(Action::RestartToUpdate);
        let (tab, id) = unsaved_edit(&mut harness);
        handed(&mut harness, Ok(()), Duration::ZERO);
        assert!(!harness.app.closing);
        assert!(matches!(
            harness.app.updates.install,
            Install::Held {
                when_done: true,
                handed: Some(_),
                ..
            }
        ));
        harness.app.apply(Action::DiscardEdits { tab, id });
        harness.app.apply_actions();
        assert!(harness.app.closing);
        // The helper has it: nothing is handed over again.
        assert_eq!(installs(&harness).len(), 1);
    }

    /// Past the helper's minute nothing installs: Couldn't install says
    /// so, and the relaunch file goes.
    #[test]
    fn work_still_open_when_the_helper_gives_up_is_couldnt_install() {
        let mut harness = offered();
        ready(&mut harness);
        harness.app.apply(Action::RestartToUpdate);
        unsaved_edit(&mut harness);
        handed(&mut harness, Ok(()), Duration::from_secs(30));
        harness.app.tick_updates();
        assert!(matches!(harness.app.updates.install, Install::Held { .. }));
        let late = std::time::Instant::now()
            .checked_sub(crate::update::HELPER_WAITS)
            .unwrap();
        if let Install::Held { handed, .. } = &mut harness.app.updates.install {
            *handed = Some(late);
        }
        harness.app.tick_updates();
        assert!(matches!(
            harness.app.updates.install,
            Install::Failed {
                failure: Failure::WorkBegan,
                ..
            }
        ));
        assert!(forgot(&harness));
        assert!(!harness.app.closing);
    }

    /// Don't restart while the helper waits goes back to Ready, which a
    /// quit in that minute still installs; then the minute ends it.
    #[test]
    fn dont_restart_while_the_helper_waits_is_ready_until_it_gives_up() {
        let mut harness = offered();
        ready(&mut harness);
        harness.app.apply(Action::RestartToUpdate);
        unsaved_edit(&mut harness);
        handed(&mut harness, Ok(()), Duration::ZERO);
        harness.app.apply(Action::DontRestart);
        assert!(matches!(
            harness.app.updates.install,
            Install::Ready {
                handed: Some(_),
                ..
            }
        ));
        if let Install::Ready { handed, .. } = &mut harness.app.updates.install {
            *handed = std::time::Instant::now().checked_sub(crate::update::HELPER_WAITS);
        }
        harness.app.tick_updates();
        assert!(matches!(
            harness.app.updates.install,
            Install::Failed {
                failure: Failure::WorkBegan,
                ..
            }
        ));
        assert!(forgot(&harness));
    }

    /// An answer read too late to quit within the helper's minute (the
    /// window was hidden) installs nothing, and says so.
    #[test]
    fn a_handoff_read_too_late_quits_nothing() {
        let mut harness = offered();
        ready(&mut harness);
        harness.app.apply(Action::RestartToUpdate);
        handed(&mut harness, Ok(()), crate::update::QUIT_IN_TIME);
        assert!(!harness.app.closing);
        assert!(matches!(
            harness.app.updates.install,
            Install::Failed {
                failure: Failure::TooLate,
                ..
            }
        ));
        assert!(forgot(&harness));
    }

    /// A helper's answer a second too old for a quit to begin.
    fn too_late() -> std::time::Instant {
        std::time::Instant::now()
            .checked_sub(crate::update::QUIT_IN_TIME + Duration::from_secs(1))
            .unwrap()
    }

    /// Work done after `QUIT_IN_TIME` quits nothing: the quit would land
    /// after the helper gave up, and no copy would start. The card waits
    /// for the minute to end in Couldn't install.
    #[test]
    fn work_done_too_late_in_the_helper_s_minute_quits_nothing() {
        let mut harness = offered();
        ready(&mut harness);
        harness.app.apply(Action::RestartToUpdate);
        let (tab, id) = unsaved_edit(&mut harness);
        handed(&mut harness, Ok(()), Duration::ZERO);
        if let Install::Held { handed, .. } = &mut harness.app.updates.install {
            *handed = Some(too_late());
        }
        harness.app.apply(Action::DiscardEdits { tab, id });
        harness.app.apply_actions();
        assert!(!harness.app.closing);
        assert!(matches!(
            harness.app.updates.install,
            Install::Held {
                when_done: true,
                ..
            }
        ));
        assert_eq!(installs(&harness).len(), 1);
    }

    /// Restart now after Don't restart, too late in the helper's minute
    /// for the quit to land in it, is Couldn't install.
    #[test]
    fn restart_now_too_late_in_the_helper_s_minute_is_couldnt_install() {
        let mut harness = offered();
        ready(&mut harness);
        harness.app.apply(Action::RestartToUpdate);
        let (tab, id) = unsaved_edit(&mut harness);
        handed(&mut harness, Ok(()), Duration::ZERO);
        harness.app.apply(Action::DontRestart);
        harness.app.apply(Action::DiscardEdits { tab, id });
        harness.app.apply_actions();
        if let Install::Ready { handed, .. } = &mut harness.app.updates.install {
            *handed = Some(too_late());
        }
        harness.app.apply(Action::RestartToUpdate);
        assert!(!harness.app.closing);
        assert!(matches!(
            harness.app.updates.install,
            Install::Failed {
                failure: Failure::WorkBegan,
                ..
            }
        ));
        assert!(forgot(&harness));
        assert_eq!(installs(&harness).len(), 1);
    }

    /// The quit writes the relaunch file again: an editor opened and
    /// typed in while the helper got ready goes to the next copy too.
    #[test]
    fn the_quit_keeps_the_editors_as_they_are_then() {
        let mut harness = offered();
        ready(&mut harness);
        harness.app.apply(Action::RestartToUpdate);
        let (tab, id) = unsaved_edit(&mut harness);
        handed(&mut harness, Ok(()), Duration::ZERO);
        let sql = harness.add_sql_tab(tab);
        harness.app.sql_tab_mut(tab, sql).unwrap().text = "SELECT * FROM book_covers".into();
        harness.app.apply(Action::DiscardEdits { tab, id });
        harness.app.apply_actions();
        assert!(harness.app.closing);
        harness.app.save_relaunch();
        let path = harness.app.dirs.relaunch_file();
        let saved = harness
            .app
            .backend
            .sent
            .iter()
            .rev()
            .find_map(|sent| match sent {
                Command::Save {
                    path: to,
                    file: StateFile::Relaunch(Some(saved)),
                } if *to == path => Some(saved),
                _ => None,
            });
        let editors = &saved.expect("the quit saves the relaunch file").windows[0].editors;
        assert_eq!(editors.len(), 1, "{editors:?}");
        assert_eq!(editors[0].text, "SELECT * FROM book_covers");
    }

    /// A quit while the helper is still being started: the editors as
    /// they are then go after Restart now's write, for the helper may yet
    /// start and install.
    #[test]
    fn a_quit_while_installing_keeps_the_editors_as_they_are_then() {
        let mut harness = offered();
        ready(&mut harness);
        harness.app.apply(Action::RestartToUpdate);
        assert!(matches!(
            harness.app.updates.install,
            Install::Restarting { .. }
        ));
        let (tab, _) = harness.book_covers();
        let sql = harness.add_sql_tab(tab);
        harness.app.sql_tab_mut(tab, sql).unwrap().text = "SELECT * FROM book_covers".into();
        harness.app.save_relaunch();
        let path = harness.app.dirs.relaunch_file();
        let sent = &harness.app.backend.sent;
        let install = sent
            .iter()
            .position(|sent| matches!(sent, Command::Updates(UpdatesCommand::Install { .. })));
        let (quit, saved) = sent
            .iter()
            .enumerate()
            .find_map(|(at, sent)| match sent {
                Command::Save {
                    path: to,
                    file: StateFile::Relaunch(Some(saved)),
                } if *to == path => Some((at, saved)),
                _ => None,
            })
            .expect("the quit saves the relaunch file");
        assert!(install.is_some_and(|install| install < quit));
        let editors = &saved.windows[0].editors;
        assert_eq!(editors.len(), 1, "{editors:?}");
        assert_eq!(editors[0].text, "SELECT * FROM book_covers");
    }

    /// With the window hidden no frame runs, only `logic`: it takes the
    /// helper's answer and quits.
    #[test]
    fn the_helper_s_answer_is_taken_with_the_window_hidden() {
        let mut harness = offered();
        ready(&mut harness);
        harness.app.apply(Action::RestartToUpdate);
        harness
            .app
            .backend
            .inject(Event::Updates(UpdatesEvent::Handed {
                result: Ok(()),
                at: std::time::Instant::now(),
            }));
        let ctx = harness.ctx.clone();
        harness.app.logic(&ctx);
        assert!(harness.app.closing);
    }

    /// Restart when done goes ahead in the frame the work ends in: no
    /// input is waited for.
    #[test]
    fn restart_when_done_goes_ahead_in_the_frame_the_work_ends() {
        let mut harness = offered();
        let tab = harness.connect_fake();
        let sql = harness.add_sql_tab(tab);
        harness.app.sql_tab_mut(tab, sql).unwrap().text = "SELECT 1".into();
        harness.app.apply(Action::RunSql {
            tab,
            sql_tab: sql,
            all: true,
        });
        ready(&mut harness);
        harness.app.apply(Action::RestartToUpdate);
        harness.app.apply(Action::RestartWhenDone);
        assert!(installs(&harness).is_empty());
        let (session, request) = harness
            .app
            .backend
            .sent
            .iter()
            .rev()
            .find_map(|sent| match sent {
                Command::RunSql {
                    session, request, ..
                } => Some((*session, *request)),
                _ => None,
            })
            .expect("a run");
        harness.app.backend.inject(Event::SqlRan {
            session,
            request,
            result: Err(tabletist_db::Error::query("stopped")),
            cancel: None,
        });
        harness.frame(Vec::new());
        assert_eq!(installs(&harness).len(), 1);
    }

    /// A dialog open over the window (here the shortcuts) would go with
    /// the quit: Restart when done waits for it.
    #[test]
    fn restart_when_done_waits_for_a_dialog_to_close() {
        let mut harness = offered();
        let (tab, id) = unsaved_edit(&mut harness);
        ready(&mut harness);
        harness.app.apply(Action::RestartToUpdate);
        harness.app.apply(Action::RestartWhenDone);
        harness.app.apply(Action::DiscardEdits { tab, id });
        harness.app.dialog = Some(Dialog::Help);
        harness.app.apply_actions();
        assert!(installs(&harness).is_empty());
        harness.app.dialog = None;
        harness.app.apply_actions();
        assert_eq!(installs(&harness).len(), 1);
    }

    /// A restart asked for under a dialog Settings cannot open over, with
    /// no work open: it waits for the dialog to close, then goes ahead,
    /// as `:update`'s yes does when the download turns ready under About.
    #[test]
    fn restart_now_under_a_dialog_waits_for_it_to_close() {
        let mut harness = offered();
        ready(&mut harness);
        harness.app.dialog = Some(Dialog::About);
        harness.app.apply(Action::RestartToUpdate);
        harness.app.apply_actions();
        assert!(installs(&harness).is_empty());
        assert!(matches!(
            harness.app.updates.install,
            Install::Held {
                when_done: true,
                ..
            }
        ));
        harness.app.dialog = None;
        harness.app.apply_actions();
        assert_eq!(installs(&harness).len(), 1);
    }

    /// Work that went before Restart when done was chosen leaves nothing
    /// to wait for: Ready again.
    #[test]
    fn a_list_that_empties_is_ready_again() {
        let mut harness = offered();
        let (tab, id) = unsaved_edit(&mut harness);
        ready(&mut harness);
        harness.app.apply(Action::RestartToUpdate);
        harness.app.apply(Action::DiscardEdits { tab, id });
        harness.app.apply_actions();
        assert_eq!(harness.app.updates.install, ready_state());
        assert!(installs(&harness).is_empty());
    }

    /// Skip this version is the card's only while it offers the version:
    /// one that downloads, waits to install or failed is never skipped.
    #[test]
    fn skip_is_ignored_once_the_version_downloads() {
        let mut harness = offered();
        ready(&mut harness);
        harness.app.apply(Action::SkipUpdate);
        assert_eq!(harness.app.settings.update_skip, None);
    }

    /// A table's read running is work too: a restart would drop it.
    #[test]
    fn a_table_being_read_is_work() {
        let mut harness = offered();
        let tab = harness.connect_fake();
        harness.app.workspace_mut(tab).unwrap().name = "Bookshop".into();
        harness.app.apply(Action::OpenObject {
            tab,
            object: tabletist_db::ObjectRef::new("main", "book_covers"),
            kind: tabletist_db::ObjectKind::Table,
            pin: true,
        });
        let work = harness.app.work_in_progress();
        assert_eq!(
            work.iter()
                .map(|work| work.what.clone())
                .collect::<Vec<_>>(),
            [Pending::Loading {
                table: "book_covers".into()
            }]
        );
    }

    /// Each editor keeps its place among the tables, its mode, and which
    /// tab was in front.
    #[test]
    fn the_relaunch_keeps_each_editor_s_place_and_mode_and_the_tab_in_front() {
        let mut harness = offered();
        let (tab, _) = harness.book_covers();
        let sql = harness.add_sql_tab(tab);
        harness.app.sql_tab_mut(tab, sql).unwrap().mode = crate::model::RunMode::ReadWrite;
        harness.app.apply(Action::OpenObject {
            tab,
            object: tabletist_db::ObjectRef::new("main", "publishers"),
            kind: tabletist_db::ObjectKind::Table,
            pin: true,
        });
        harness.app.apply(Action::ActivateTab { tab, id: sql });
        let kept = harness.app.kept_for_relaunch();
        let window = &kept.windows[0];
        assert_eq!(window.front, Some(1));
        assert_eq!((window.editors[0].at, window.editors[0].writes), (1, true));
    }

    /// Later keeps it ready, and closes Settings.
    #[test]
    fn later_keeps_the_update_ready() {
        let mut harness = offered();
        harness
            .app
            .apply(Action::ShowSettingsPane(SettingsPane::Updates));
        ready(&mut harness);
        harness.app.apply(Action::UpdateLater);
        assert!(harness.app.dialog.is_none());
        assert!(matches!(harness.app.updates.install, Install::Ready { .. }));
        assert!(installs(&harness).is_empty());
    }
}

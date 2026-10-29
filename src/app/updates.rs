//! Updates, as the app keeps them: when a check runs (on launch where one
//! is due, at the interval while the app is open, and when the person
//! asks), what it found, and what the window says of it. The request is
//! the backend's ([`crate::backend::UpdatesCommand`]); what the checks
//! found is kept in the state folder (`updates.json`).

use std::time::Duration;

use super::App;
use crate::backend::{Command, StateFile, UpdatesCommand, UpdatesEvent};
use crate::model::{Action, ConnTabContent};
use crate::update::{Failed, Found, Phase, release};

/// What the terminal look's status line says of a version available,
/// `{version}` for it and `{key}` for Check for Updates' key as the keymap
/// has it, for [`ANNOUNCED_FOR`]. With no key, [`ANNOUNCEMENT_KEYLESS`].
pub const ANNOUNCEMENT: &str = "{version} available · {key}";
pub const ANNOUNCEMENT_KEYLESS: &str = "{version} available";

/// How long the status line says it.
pub const ANNOUNCED_FOR: f64 = 8.0;

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
        if !self.updates_scheduled {
            return None;
        }
        let now = self.now_secs();
        self.announce_update(now);
        // The asset an offer keeps is this copy's kind's: the check waits
        // for the detection, which answers within the launch.
        if self.updates.checking() || self.updates.installed.is_none() {
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
            // the person asked for.
            Action::CheckForUpdates => match &mut self.updates.phase {
                Phase::Checking { asked_for, .. } => *asked_for = true,
                Phase::Idle | Phase::Failed { .. } => self.start_update_check(true),
            },
            // The schedule's next check waits an interval from it, as
            // from one that failed.
            Action::CancelUpdateCheck => {
                if self.updates.checking() {
                    self.updates.phase = Phase::Idle;
                    self.updates.tried = Some(self.now_secs());
                }
            }
            Action::SkipUpdate => {
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
            // Nothing asks for a download yet.
            UpdatesEvent::Progress { .. }
            | UpdatesEvent::Waiting { .. }
            | UpdatesEvent::Downloaded { .. }
            | UpdatesEvent::Handed(_) => {}
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
}

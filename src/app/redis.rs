//! A Redis connection's window, as the app runs it: what is asked of the
//! session when it connects, what each answer changes, and what the
//! views and keys ask (`RedisAction`). The state is the workspace's
//! `redis`, a `keyspace::Browser`.

use std::time::Duration;

use super::App;
use crate::backend::{Command, RedisAnswer, RedisAsk, RequestId, SessionId};
use crate::keyspace::{Browser, Place, RedisAction, Row, database_label};
use crate::model::ConnTabId;

/// The wait between two steps of a production connection's scan, so it
/// never competes with the traffic the server is there for.
pub const PRODUCTION_PACE: Duration = Duration::from_millis(20);

/// The number a database menu's label starts with.
fn database_of(label: &str) -> Option<u32> {
    label
        .split(|c: char| !c.is_ascii_digit())
        .next()
        .and_then(|number| number.parse().ok())
}

impl App {
    fn browser_mut(&mut self, tab: ConnTabId) -> Option<&mut Browser> {
        self.workspace_mut(tab)?.redis.as_deref_mut()
    }

    /// Sends `ask` for `tab`'s session; its request, where the window is
    /// there to send it.
    fn redis_send(&mut self, tab: ConnTabId, ask: RedisAsk) -> Option<RequestId> {
        let session = self.workspace(tab)?.session;
        let request = RequestId(self.next_id());
        self.backend.send(Command::Redis {
            session,
            request,
            ask,
        });
        Some(request)
    }

    /// The session is up: the window opens on the connection's database,
    /// asks what the server holds, and scans its names. Connected again
    /// (a reconnect, or after an idle close), it goes back to the
    /// database it was on and scans again where a scan had not ended.
    pub(super) fn redis_after_connect(&mut self, tab: ConnTabId) {
        let Some(workspace) = self.workspace_mut(tab) else {
            return;
        };
        let first = workspace.spec.redis_spec().database;
        let again = match &workspace.redis {
            Some(browser) => Some((browser.database, browser.scan.running())),
            None => {
                let redis = workspace.spec.redis_spec();
                let production = workspace.environment == crate::env::Environment::Production;
                workspace.redis = Some(Box::new(Browser::new(
                    redis.database,
                    redis.browsing,
                    production,
                )));
                None
            }
        };
        let database = match again {
            Some((database, _)) if database != first => {
                self.redis_send(tab, RedisAsk::Select { database });
                database
            }
            Some((database, _)) => database,
            None => first,
        };
        self.redis_probe(tab, database);
        match again {
            Some((_, false)) => {}
            Some((_, true)) | None => self.redis_scan(tab),
        }
    }

    fn redis_probe(&mut self, tab: ConnTabId, database: u32) {
        if let Some(request) = self.redis_send(tab, RedisAsk::Probe { database })
            && let Some(browser) = self.browser_mut(tab)
        {
            browser.probe.start(request);
        }
    }

    /// Scans the names again from the start, with the filter's pattern.
    fn redis_scan(&mut self, tab: ConnTabId) {
        let request = RequestId(self.next_id());
        let Some(workspace) = self.workspace_mut(tab) else {
            return;
        };
        let session = workspace.session;
        let Some(browser) = workspace.redis.as_deref_mut() else {
            return;
        };
        let pattern = match browser.filter.trim() {
            "" => "*".to_owned(),
            typed => typed.to_owned(),
        };
        browser.restart(request, &pattern);
        let ask = RedisAsk::Scan {
            pattern,
            count: browser.browsing.page,
            cursor: 0,
            limit: browser.limit(),
            pace: browser.production.then_some(PRODUCTION_PACE),
            stop: browser.scan.stop.clone(),
        };
        self.backend.send(Command::Redis {
            session,
            request,
            ask,
        });
    }

    /// Goes on from where the limit stopped the scan, with no limit.
    fn redis_scan_all(&mut self, tab: ConnTabId) {
        let request = RequestId(self.next_id());
        let Some(workspace) = self.workspace_mut(tab) else {
            return;
        };
        let session = workspace.session;
        let Some(browser) = workspace.redis.as_deref_mut() else {
            return;
        };
        browser.confirm_scan_all = false;
        browser.resume(request);
        let ask = RedisAsk::Scan {
            pattern: browser.scan.pattern.clone(),
            count: browser.browsing.page,
            cursor: browser.scan.cursor,
            limit: None,
            pace: browser.production.then_some(PRODUCTION_PACE),
            stop: browser.scan.stop.clone(),
        };
        self.backend.send(Command::Redis {
            session,
            request,
            ask,
        });
    }

    /// Opens `key` in front, and reads it where it is new.
    fn redis_open(&mut self, tab: ConnTabId, key: &[u8]) {
        let Some(browser) = self.browser_mut(tab) else {
            return;
        };
        let count = browser.browsing.page;
        if !browser.open_key(key) {
            return;
        }
        let ask = RedisAsk::Open {
            key: key.to_vec(),
            count,
        };
        if let Some(request) = self.redis_send(tab, ask)
            && let Some(open) = self
                .browser_mut(tab)
                .and_then(|browser| browser.tab_of(key))
        {
            open.info.start(request);
            open.loading = Some(request);
        }
    }

    /// The next page of the key in front.
    fn redis_load_more(&mut self, tab: ConnTabId) {
        let Some(browser) = self.browser_mut(tab) else {
            return;
        };
        let count = browser.browsing.page;
        let Some(open) = browser.active_tab() else {
            return;
        };
        let (Some(from), Some(info), None) = (&open.next, &open.info.value, open.loading) else {
            return;
        };
        let ask = RedisAsk::Read {
            key: open.key.to_vec(),
            kind: info.kind.clone(),
            length: info.length,
            from: from.clone(),
            count,
        };
        let key = open.key.clone();
        if let Some(request) = self.redis_send(tab, ask)
            && let Some(open) = self
                .browser_mut(tab)
                .and_then(|browser| browser.tab_of(&key))
        {
            open.loading = Some(request);
        }
    }

    pub(super) fn apply_redis(&mut self, tab: ConnTabId, action: RedisAction) {
        match action {
            RedisAction::ToggleGroup(prefix) => {
                if let Some(browser) = self.browser_mut(tab) {
                    browser.toggle(&prefix);
                    browser.cursor = Some(Place::Group(prefix));
                }
            }
            RedisAction::More(prefix) => {
                if let Some(browser) = self.browser_mut(tab) {
                    browser.more(&prefix);
                }
            }
            RedisAction::OpenKey(key) => {
                if let Some(browser) = self.browser_mut(tab) {
                    browser.cursor = Some(Place::Key(key.clone()));
                }
                self.redis_open(tab, &key);
            }
            RedisAction::SelectTab(at) => {
                if let Some(browser) = self.browser_mut(tab)
                    && at < browser.tabs.len()
                {
                    browser.active = Some(at);
                }
            }
            RedisAction::CloseTab(at) => {
                if let Some(browser) = self.browser_mut(tab) {
                    browser.close_tab(at);
                }
            }
            RedisAction::Visible(keys) => {
                let Some(browser) = self.browser_mut(tab) else {
                    return;
                };
                let asked = browser.types_to_ask(keys);
                if !asked.is_empty() {
                    let keys = asked.into_iter().map(|key| key.into_vec()).collect();
                    self.redis_send(tab, RedisAsk::Types { keys });
                }
            }
            RedisAction::Filter | RedisAction::Reload => self.redis_scan(tab),
            RedisAction::Stop => {
                if let Some(browser) = self.browser_mut(tab) {
                    browser.stop();
                }
            }
            RedisAction::ScanAll => {
                if let Some(browser) = self.browser_mut(tab) {
                    browser.confirm_scan_all = true;
                }
            }
            RedisAction::ConfirmScanAll(true) => self.redis_scan_all(tab),
            RedisAction::ConfirmScanAll(false) => {
                if let Some(browser) = self.browser_mut(tab) {
                    browser.confirm_scan_all = false;
                }
            }
            RedisAction::SwitchDatabase(database) => {
                if let Some(browser) = self.browser_mut(tab) {
                    browser.stop();
                }
                self.redis_send(tab, RedisAsk::Select { database });
            }
            RedisAction::LoadMore => self.redis_load_more(tab),
            RedisAction::CopyValue => {
                let copied = self
                    .workspace(tab)
                    .and_then(|workspace| workspace.redis.as_deref())
                    .and_then(Browser::active_tab)
                    .and_then(crate::keyspace::KeyTab::copied);
                if let Some(text) = copied {
                    self.to_clipboard = Some(text);
                    self.said =
                        Some(crate::i18n::gettext(self.locale, "Value copied").into_owned());
                }
            }
            RedisAction::CopyKey => {
                let copied = self
                    .workspace(tab)
                    .and_then(|workspace| workspace.redis.as_deref())
                    .and_then(Browser::active_tab)
                    .map(|open| crate::keyspace::lossy(&open.key));
                if let Some(text) = copied {
                    self.to_clipboard = Some(text);
                    self.said = Some(crate::i18n::gettext(self.locale, "Key copied").into_owned());
                }
            }
            RedisAction::SelectCell(row, column) => {
                if let Some(open) = self.browser_mut(tab).and_then(Browser::active_tab_mut) {
                    open.selected = Some((row, column));
                }
            }
            RedisAction::CopyCell => {
                let copied = self
                    .workspace(tab)
                    .and_then(|workspace| workspace.redis.as_deref())
                    .and_then(Browser::active_tab)
                    .and_then(crate::keyspace::KeyTab::selected_text);
                if let Some(text) = copied {
                    self.to_clipboard = Some(text);
                }
            }
            RedisAction::Cursor(place) => {
                if let Some(browser) = self.browser_mut(tab) {
                    browser.cursor = Some(place);
                }
            }
            RedisAction::Move(by) => {
                if let Some(browser) = self.browser_mut(tab) {
                    browser.move_cursor(by);
                }
            }
            RedisAction::Step(right) => {
                if let Some(browser) = self.browser_mut(tab) {
                    browser.step(right);
                }
            }
            RedisAction::Enter | RedisAction::FoldGroup => {
                let Some(browser) = self.browser_mut(tab) else {
                    return;
                };
                let fold = action == RedisAction::FoldGroup;
                let rows = browser.rows();
                let Some(at) = browser
                    .cursor
                    .as_ref()
                    .and_then(|place| rows.iter().position(|row| row.place() == *place))
                else {
                    browser.move_cursor(0);
                    return;
                };
                match &rows[at] {
                    Row::Group { prefix, .. } => {
                        let prefix = prefix.clone();
                        browser.toggle(&prefix);
                    }
                    // `za` on a key folds the group it is in.
                    Row::Key { .. } | Row::More { .. } if fold => {
                        let depth = rows[at].depth();
                        let parent = rows[..at]
                            .iter()
                            .rev()
                            .find(|row| matches!(row, Row::Group { .. }) && row.depth() < depth);
                        if let Some(Row::Group { prefix, .. }) = parent {
                            let prefix = prefix.clone();
                            browser.toggle(&prefix);
                            browser.cursor = Some(Place::Group(prefix));
                        }
                    }
                    Row::Key { key, .. } => {
                        let key = key.clone();
                        self.redis_open(tab, &key);
                    }
                    Row::More { prefix, .. } => {
                        let prefix = prefix.clone();
                        browser.more(&prefix);
                    }
                }
            }
            RedisAction::FocusFilter => {
                if let Some(browser) = self.browser_mut(tab) {
                    browser.focus_filter = true;
                }
            }
        }
    }

    /// The database menu's choice, for a Redis window: its label's number.
    pub(super) fn redis_switch_label(&mut self, tab: ConnTabId, label: &str) {
        let current = self
            .workspace(tab)
            .and_then(|workspace| workspace.redis.as_deref())
            .map(|browser| browser.database);
        // The database the window is on already is no switch: no SELECT,
        // no scan again.
        if let Some(database) = database_of(label).filter(|database| Some(*database) != current) {
            self.apply_redis(tab, RedisAction::SwitchDatabase(database));
        }
    }

    pub(super) fn apply_redis_event(
        &mut self,
        session: SessionId,
        request: RequestId,
        answer: RedisAnswer,
    ) {
        let Some(tab) = self.tab_for_session(session) else {
            return;
        };
        let Some(workspace) = self.workspace_mut(tab) else {
            return;
        };
        let Some(browser) = workspace.redis.as_deref_mut() else {
            return;
        };
        match answer {
            RedisAnswer::Probed(result) => {
                if !browser.probe.finish(request, result) {
                    return;
                }
                // The menu of databases, with their keys.
                let labels = browser.probe.value.as_ref().map(|probe| {
                    (0..probe.databases.max(1))
                        .map(|number| database_label(number, probe.keys_in(number)))
                        .collect::<Vec<_>>()
                });
                let version = browser
                    .probe
                    .value
                    .as_ref()
                    .map(|probe| probe.server.label());
                if let Some(labels) = labels {
                    workspace.databases.value = Some(labels);
                }
                if let Some(version) = version {
                    workspace.server_version.value = Some(version);
                }
                let counted = browser.probe.value.as_ref().map(|probe| {
                    let keys = probe.keys.unwrap_or_else(|| probe.keys_in(probe.database));
                    (probe.database, keys)
                });
                if let Some(counted) = counted {
                    let conn = workspace.conn_id.clone();
                    self.redis_keys.insert(conn, counted);
                }
            }
            RedisAnswer::Selected { database, result } => match result {
                Ok(()) => {
                    browser.database = database;
                    // Its keys are another database's: they go, and so do
                    // the tabs of the old one's.
                    browser.tabs.clear();
                    browser.active = None;
                    browser.open.clear();
                    browser.cursor = None;
                    self.redis_probe(tab, database);
                    self.redis_scan(tab);
                }
                Err(error) => self.said = Some(error.to_string()),
            },
            RedisAnswer::Scanned(step) => {
                browser.take_step(request, step);
            }
            RedisAnswer::Types { keys, result } => browser.take_types(keys, result),
            RedisAnswer::Opened { key, result } => {
                let Some(open) = browser.tab_of(&key) else {
                    return;
                };
                if open.loading != Some(request) {
                    return;
                }
                open.loading = None;
                match result {
                    Ok((info, loaded)) => {
                        let kind = info.kind.clone();
                        open.info.finish(request, Ok(info));
                        open.take(loaded);
                        // What the header says of its type, the tree says.
                        browser.types.insert(key.into(), kind);
                    }
                    Err(error) => {
                        open.info.finish(request, Err(error.clone()));
                        open.error = Some(error);
                    }
                }
            }
            RedisAnswer::Read { key, result } => {
                let Some(open) = browser.tab_of(&key) else {
                    return;
                };
                if open.loading != Some(request) {
                    return;
                }
                open.loading = None;
                match result {
                    Ok(loaded) => open.take(loaded),
                    Err(error) => open.error = Some(error),
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_database_menu_names_each_with_its_keys() {
        assert_eq!(database_label(0, 1204), "0 · 1,204 keys");
        assert_eq!(database_label(2, 0), "2 · empty");
        assert_eq!(database_label(3, 1), "3 · 1 key");
        assert_eq!(database_of(&database_label(12, 4)), Some(12));
        assert_eq!(database_of("x"), None);
    }
}

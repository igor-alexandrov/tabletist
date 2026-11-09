//! A Redis connection, headless: what its window asks of the backend, how
//! its keys show (the AccessKit tree a screen reader reads), its keys, and
//! the dialog's Redis groups and Test. The backend records what is sent;
//! answers are given by hand, from the Bookshop's keys.

use std::time::Duration;

use egui::accesskit::Role;
use egui::{Key, Modifiers};
use tabletist_db::redis::{Cursor, KeyInfo, KeyType, Loaded, Page};

use super::ddl_tests::letter;
use crate::backend::{Command, Event, RedisAnswer, RedisAsk};
use crate::keyspace::{RedisAction, ScanEnd, ScanState, ScanStep};
use crate::model::{Action, ConnTabId, Dialog, TestState};
use crate::testing::{Harness, labels, node};

/// The Redis asks sent so far, oldest first.
fn asks(harness: &Harness) -> Vec<&RedisAsk> {
    harness
        .app
        .backend
        .sent
        .iter()
        .filter_map(|command| match command {
            Command::Redis { ask, .. } => Some(ask),
            _ => None,
        })
        .collect()
}

fn browser(harness: &Harness, tab: ConnTabId) -> &crate::keyspace::Browser {
    harness
        .app
        .workspace(tab)
        .and_then(|workspace| workspace.redis.as_deref())
        .expect("a Redis window")
}

/// Answers the running scan with one step.
fn step(harness: &mut Harness, names: &[&str], total: u64, end: ScanEnd) {
    let (session, request) = harness.last_redis(|ask| matches!(ask, RedisAsk::Scan { .. }));
    let step = ScanStep {
        total: Some(total),
        names: names.iter().map(|name| name.as_bytes().to_vec()).collect(),
        cursor: if end == ScanEnd::Done { 0 } else { 4242 },
        end,
        took: Duration::from_millis(3),
    };
    harness.app.apply(Action::Backend(Event::Redis {
        session,
        request,
        answer: RedisAnswer::Scanned(Ok(step)),
    }));
}

/// Answers the newest Open with `info` and `page`.
fn answer_open(harness: &mut Harness, info: KeyInfo, page: Page, command: &'static str) {
    let (session, request) = harness.last_redis(|ask| matches!(ask, RedisAsk::Open { .. }));
    let key = match asks(harness).last() {
        Some(RedisAsk::Open { key, .. }) => key.clone(),
        other => panic!("expected an Open, not {other:?}"),
    };
    harness.app.apply(Action::Backend(Event::Redis {
        session,
        request,
        answer: RedisAnswer::Opened {
            key,
            result: Ok((
                info,
                Loaded {
                    page,
                    next: None,
                    command,
                    took: Duration::from_millis(1),
                },
            )),
        },
    }));
}

fn book(fields: u64) -> KeyInfo {
    KeyInfo {
        kind: KeyType::Hash,
        ttl_ms: None,
        memory: Some(412),
        encoding: Some("listpack".into()),
        length: Some(fields),
    }
}

fn book_fields() -> Page {
    Page::Pairs(vec![
        (b"title".to_vec(), b"Lanterns".to_vec()),
        (b"price".to_vec(), b"12.99".to_vec()),
    ])
}

#[test]
fn a_redis_connection_probes_and_scans_and_asks_for_no_schema() {
    let mut harness = Harness::new();
    let tab = harness.connect_redis_unscanned();
    let sent = &harness.app.backend.sent;
    assert!(
        !sent.iter().any(|command| matches!(
            command,
            Command::ListSchemas { .. }
                | Command::ServerVersion { .. }
                | Command::ListDatabases { .. }
        )),
        "{sent:?}"
    );
    let asks = asks(&harness);
    assert!(matches!(asks[0], RedisAsk::Probe { database: 0 }));
    let Some(RedisAsk::Scan {
        pattern,
        count,
        cursor,
        limit,
        pace,
        ..
    }) = asks.get(1)
    else {
        panic!("expected a Scan: {asks:?}");
    };
    assert_eq!((pattern.as_str(), *count, *cursor), ("*", 1000, 0));
    assert_eq!(*limit, Some(100_000));
    // Dev: no pacing.
    assert_eq!(*pace, None);
    assert!(browser(&harness, tab).scan.running());
    // The database menu lists the probe's databases with their keys.
    let databases = harness
        .app
        .workspace(tab)
        .unwrap()
        .databases
        .value
        .clone()
        .unwrap();
    assert_eq!(databases.len(), 16);
    assert_eq!(databases[0], "0 · 65 keys");
    assert_eq!(databases[1], "1 · empty");
}

#[test]
fn a_production_scan_waits_between_its_steps() {
    let mut harness = Harness::new();
    let mut saved = crate::testing::bookshop_cache();
    saved.environment = crate::env::Environment::Production;
    let conn = saved.id.clone();
    harness.app.connections.upsert(saved);
    let tab = harness.app.active_tab_id();
    harness.app.apply(Action::Connect { tab, conn });
    let Command::Connect {
        session, request, ..
    } = *crate::testing::last_sent(&harness.app)
    else {
        panic!("expected Connect");
    };
    harness.app.apply(Action::Backend(Event::Connected {
        session,
        request,
        driver: tabletist_db::Driver::Redis,
        encrypted: false,
        access: crate::testing::asked_access(&harness.app),
    }));
    let pace = asks(&harness).iter().find_map(|ask| match ask {
        RedisAsk::Scan { pace, .. } => Some(*pace),
        _ => None,
    });
    assert_eq!(pace, Some(Some(Duration::from_millis(20))));
}

#[test]
fn the_tree_groups_the_keys_with_their_counts_and_types() {
    let mut harness = Harness::new();
    harness.connect_redis();
    harness.frame(Vec::new());
    harness.answer_redis_types();
    let tree = harness.settle();
    assert!(
        node(&tree, "Keys", Role::Tree).is_some(),
        "{:?}",
        labels(&tree)
    );
    for item in [
        "book:, 13 keys",
        "cart:, 4 keys",
        "session:, 31 keys",
        "tags:, 13 keys",
        "config, string",
        "queue:emails, list",
        "top:books, zset",
        "events, stream",
    ] {
        assert!(
            node(&tree, item, Role::TreeItem).is_some(),
            "{item}: {:?}",
            labels(&tree)
        );
    }
    let footer = harness
        .painted
        .iter()
        .any(|(text, _)| text.starts_with("Scanned 65 of 65 keys · 3"));
    assert!(footer, "{:?}", harness.painted);
    let key = harness.app.keymap.key_in(
        harness.app.layout(),
        crate::keymap::Command::ReloadKeys,
        crate::keymap::Scope::Redis,
    );
    assert!(harness.has(&format!("Reload {}", key.as_str())));
}

#[test]
fn a_group_opens_and_a_key_shows_its_fields_in_a_table() {
    let mut harness = Harness::new();
    let tab = harness.connect_redis();
    harness.click("book:, 13 keys");
    harness.answer_redis_types();
    let tree = harness.settle();
    assert!(
        node(&tree, "4, hash", Role::TreeItem).is_some(),
        "{:?}",
        labels(&tree)
    );
    // A group of one key is that key: the review sits among the books.
    assert!(node(&tree, "reviews:12, hash", Role::TreeItem).is_some());
    harness.click("4, hash");
    let Some(RedisAsk::Open { key, count }) = asks(&harness).last().copied() else {
        panic!("expected an Open");
    };
    assert_eq!((key.as_slice(), *count), (b"book:4".as_slice(), 1000));
    answer_open(&mut harness, book(2), book_fields(), "HGETALL");
    let tree = harness.settle();
    assert!(
        node(&tree, "Value of book:4", Role::Table).is_some(),
        "{:?}",
        labels(&tree)
    );
    assert!(node(&tree, "Field", Role::ColumnHeader).is_some());
    assert!(node(&tree, "Value", Role::ColumnHeader).is_some());
    assert!(node(&tree, "Lanterns", Role::Cell).is_some());
    assert!(harness.has("Copy value"));
    assert!(harness.has("Copy key"));
    let header = harness
        .painted
        .iter()
        .any(|(text, _)| text == "2 fields · 412 B · listpack · No expiry");
    assert!(header, "{:?}", harness.painted);
    assert!(
        harness
            .painted
            .iter()
            .any(|(text, _)| text.starts_with("HGETALL · 2 fields · "))
    );
    // Copy value copies the hash as JSON.
    harness.click("Copy value");
    let copied: serde_json::Value =
        serde_json::from_str(harness.copied.as_deref().expect("copied")).unwrap();
    assert_eq!(
        copied,
        serde_json::json!({ "title": "Lanterns", "price": "12.99" })
    );
    harness.click("Copy key");
    assert_eq!(harness.copied.as_deref(), Some("book:4"));
    assert_eq!(browser(&harness, tab).tabs.len(), 1);
}

#[test]
fn types_are_asked_only_for_the_rows_on_screen() {
    let mut harness = Harness::new();
    let tab = harness.connect_redis_unscanned();
    let names: Vec<String> = (0..5000).map(|n| format!("k{n:04}")).collect();
    let names: Vec<&str> = names.iter().map(String::as_str).collect();
    step(&mut harness, &names, 5000, ScanEnd::Done);
    harness.settle();
    let asked: Vec<&Vec<u8>> = asks(&harness)
        .into_iter()
        .filter_map(|ask| match ask {
            RedisAsk::Types { keys } => Some(keys),
            _ => None,
        })
        .flatten()
        .collect();
    assert!(!asked.is_empty());
    // A screen of rows, not the thousand the first page holds.
    assert!(asked.len() < 60, "{} asked", asked.len());
    assert!(asked.iter().all(|key| key.as_slice() < b"k0060".as_slice()));
    // Asked once each, however many frames are drawn.
    harness.frame(Vec::new());
    harness.frame(Vec::new());
    let again = asks(&harness)
        .into_iter()
        .filter(|ask| matches!(ask, RedisAsk::Types { .. }))
        .count();
    assert_eq!(again, 1);
    let shown = browser(&harness, tab).rows().len();
    // 1,000 keys and the row that shows the next page.
    assert_eq!(shown, 1001);
    assert!(matches!(
        browser(&harness, tab).rows().last(),
        Some(crate::keyspace::Row::More { left: 4000, .. })
    ));
}

#[test]
fn a_running_scan_counts_up_and_stop_stops_it() {
    let mut harness = Harness::new();
    let tab = harness.connect_redis_unscanned();
    step(
        &mut harness,
        &["book:1", "book:2", "config"],
        65,
        ScanEnd::More,
    );
    harness.settle();
    assert!(harness.painted.iter().any(|(text, _)| text == "2+"));
    assert!(
        harness
            .painted
            .iter()
            .any(|(text, _)| text == "Scanned 3+ of 65 keys")
    );
    let stop = browser(&harness, tab).scan.stop.clone();
    harness.click("Stop");
    assert!(stop.is_stopped());
    assert_eq!(browser(&harness, tab).scan.state, ScanState::Stopped);
    harness.settle();
    assert!(harness.painted.iter().any(|(text, _)| text == "≈2"));
    assert!(
        harness
            .painted
            .iter()
            .any(|(text, _)| text == "Stopped at 3 of 65 keys")
    );
    // What the task still sends is dropped.
    step(&mut harness, &["cart:1"], 65, ScanEnd::More);
    assert_eq!(browser(&harness, tab).tree.len(), 3);
}

#[test]
fn the_key_limit_stops_the_scan_and_scan_all_asks_first() {
    let mut harness = Harness::new();
    let tab = harness.connect_redis_unscanned();
    step(
        &mut harness,
        &["book:1", "book:2"],
        2_300_000,
        ScanEnd::Limited,
    );
    harness.settle();
    assert!(
        harness
            .painted
            .iter()
            .any(|(text, _)| text == "Showing groups from the first 2 of 2.3M keys"),
        "{:?}",
        harness.painted
    );
    assert!(
        harness
            .painted
            .iter()
            .any(|(text, _)| text == "or filter with a pattern")
    );
    assert!(harness.painted.iter().any(|(text, _)| text == "≈2"));
    let before = asks(&harness).len();
    harness.click("Scan all");
    // It asks first, and sends nothing yet.
    assert_eq!(asks(&harness).len(), before);
    assert!(
        harness
            .painted
            .iter()
            .any(|(text, _)| text.starts_with("Scanning 2.3M keys may take a while and hold about")),
        "{:?}",
        harness.painted
    );
    harness.click("Scan all");
    let Some(RedisAsk::Scan { cursor, limit, .. }) = asks(&harness).last().copied() else {
        panic!("expected a Scan");
    };
    assert_eq!((*cursor, *limit), (4242, None));
    assert!(browser(&harness, tab).scan.running());
    // What it held stays: Scan all goes on from there.
    assert_eq!(browser(&harness, tab).tree.len(), 2);
}

#[test]
fn a_pattern_scans_again_holding_only_what_matches() {
    let mut harness = Harness::new();
    let tab = harness.connect_redis();
    harness
        .app
        .apply(Action::Redis(tab, RedisAction::FocusFilter));
    harness.settle();
    assert!(harness.ctx.text_edit_focused());
    harness.frame(vec![egui::Event::Text("book:*".into())]);
    harness.press(Key::Enter, Modifiers::NONE);
    let Some(RedisAsk::Scan {
        pattern, cursor, ..
    }) = asks(&harness).last().copied()
    else {
        panic!("expected a Scan");
    };
    assert_eq!((pattern.as_str(), *cursor), ("book:*", 0));
    // The names of the scan before go: only the new one's are held.
    assert!(browser(&harness, tab).tree.is_empty());
    step(&mut harness, &["book:1", "book:2"], 65, ScanEnd::Done);
    assert_eq!(browser(&harness, tab).tree.len(), 2);
}

#[test]
fn switching_the_database_selects_it_and_scans_again() {
    let mut harness = Harness::new();
    let tab = harness.connect_redis();
    harness.app.apply(Action::SwitchDatabase {
        tab,
        database: "2 · 38 keys".into(),
    });
    let Some(RedisAsk::Select { database }) = asks(&harness).last().copied() else {
        panic!("expected a Select");
    };
    assert_eq!(*database, 2);
    let (session, request) = harness.last_redis(|ask| matches!(ask, RedisAsk::Select { .. }));
    harness.app.apply(Action::Backend(Event::Redis {
        session,
        request,
        answer: RedisAnswer::Selected {
            database: 2,
            result: Ok(()),
        },
    }));
    let asks = asks(&harness);
    let n = asks.len();
    assert!(matches!(asks[n - 2], RedisAsk::Probe { database: 2 }));
    assert!(matches!(asks[n - 1], RedisAsk::Scan { cursor: 0, .. }));
    assert_eq!(browser(&harness, tab).database, 2);
    assert!(browser(&harness, tab).tree.is_empty());
    harness.settle();
    assert!(harness.painted.iter().any(|(text, _)| text == "DB 2"));
}

#[test]
fn the_arrows_move_in_the_keys_and_enter_opens_one() {
    let mut harness = Harness::new();
    let tab = harness.connect_redis();
    harness.press(Key::ArrowDown, Modifiers::NONE);
    assert_eq!(
        browser(&harness, tab).cursor,
        Some(crate::keyspace::Place::Group(b"book:".to_vec()))
    );
    harness.press(Key::ArrowRight, Modifiers::NONE);
    assert!(browser(&harness, tab).open.contains(b"book:".as_slice()));
    harness.press(Key::ArrowRight, Modifiers::NONE);
    harness.press(Key::Enter, Modifiers::NONE);
    let Some(RedisAsk::Open { key, .. }) = asks(&harness).last().copied() else {
        panic!("expected an Open");
    };
    // The first row under the group: its group of one, then its keys.
    assert_eq!(key.as_slice(), b"book:1");
    // ⌘R scans again from the start.
    let before = asks(&harness).len();
    harness.press(Key::R, Modifiers::COMMAND);
    assert_eq!(asks(&harness).len(), before + 1);
    assert!(matches!(
        asks(&harness).last(),
        Some(RedisAsk::Scan { cursor: 0, .. })
    ));
    // ⌘F takes the keyboard to the filter.
    harness.press(Key::F, Modifiers::COMMAND);
    let field = crate::ui::redis::filter_id(tab);
    let _ = field;
    assert!(harness.ctx.text_edit_focused() || browser(&harness, tab).focus_filter);
}

#[test]
fn the_terminal_looks_letters_move_fold_and_copy() {
    let mut harness = Harness::new();
    harness.set_look(crate::theme::Look::omarchy());
    let tab = harness.connect_redis();
    letter(&mut harness, 'j');
    assert_eq!(
        browser(&harness, tab).cursor,
        Some(crate::keyspace::Place::Group(b"book:".to_vec()))
    );
    harness.press(Key::Enter, Modifiers::NONE);
    assert!(browser(&harness, tab).open.contains(b"book:".as_slice()));
    letter(&mut harness, 'j');
    letter(&mut harness, 'z');
    letter(&mut harness, 'a');
    // `za` on a key folds the group it is in.
    assert!(!browser(&harness, tab).open.contains(b"book:".as_slice()));
    harness.app.apply(Action::Redis(
        tab,
        RedisAction::OpenKey(Box::from(&b"book:4"[..])),
    ));
    answer_open(&mut harness, book(2), book_fields(), "HGETALL");
    letter(&mut harness, 'Y');
    assert_eq!(harness.copied.as_deref(), Some("book:4"));
    letter(&mut harness, 'y');
    assert!(
        harness
            .copied
            .as_deref()
            .is_some_and(|text| text.contains("Lanterns"))
    );
    let before = asks(&harness).len();
    letter(&mut harness, 'r');
    assert_eq!(asks(&harness).len(), before + 1);
}

#[test]
fn sql_only_features_are_not_offered_on_redis() {
    let mut harness = Harness::new();
    let tab = harness.connect_redis();
    for action in [
        Action::NewSqlTab(tab),
        Action::ShowHistory(tab),
        Action::ShowAgents(tab),
        Action::PickTableToExport(tab),
    ] {
        harness.app.apply(action);
    }
    let workspace = harness.app.workspace(tab).unwrap();
    assert!(workspace.tabs.is_empty());
    assert!(harness.app.dialog.is_none());
    assert!(!harness.app.has_sql(tab));
    // ⌘T, the new SQL editor's key, opens none either.
    harness.press(Key::T, Modifiers::COMMAND);
    assert!(harness.app.workspace(tab).unwrap().tabs.is_empty());
}

#[test]
fn a_long_string_says_it_is_cut_and_a_binary_one_shows_as_hex() {
    let mut harness = Harness::new();
    let tab = harness.connect_redis();
    harness.app.apply(Action::Redis(
        tab,
        RedisAction::OpenKey(Box::from(&b"config"[..])),
    ));
    let info = KeyInfo {
        kind: KeyType::String,
        ttl_ms: Some(12 * 60 * 1000),
        memory: None,
        encoding: None,
        length: Some(1024 * 1024),
    };
    let page = Page::Text {
        bytes: vec![b'a'; 256 * 1024],
        length: 1024 * 1024,
    };
    answer_open(&mut harness, info, page, "GETRANGE");
    harness.settle();
    assert!(
        harness
            .painted
            .iter()
            .any(|(text, _)| text == "Showing the first 256 KiB")
    );
    assert!(
        harness
            .painted
            .iter()
            .any(|(text, _)| text.ends_with("Expires in 12 min"))
    );
    harness.app.apply(Action::Redis(
        tab,
        RedisAction::OpenKey(Box::from(&b"cover"[..])),
    ));
    let info = KeyInfo {
        kind: KeyType::String,
        ttl_ms: None,
        memory: None,
        encoding: None,
        length: Some(4),
    };
    let page = Page::Text {
        bytes: vec![0, 159, 146, 150],
        length: 4,
    };
    answer_open(&mut harness, info, page, "GET");
    harness.settle();
    assert!(
        harness
            .painted
            .iter()
            .any(|(text, _)| text == "Binary, 4 B")
    );
    assert!(
        harness
            .painted
            .iter()
            .any(|(text, _)| text == "00 9f 92 96")
    );
}

#[test]
fn load_more_reads_the_next_page_from_where_the_last_ended() {
    let mut harness = Harness::new();
    let tab = harness.connect_redis();
    harness.app.apply(Action::Redis(
        tab,
        RedisAction::OpenKey(Box::from(&b"queue:emails"[..])),
    ));
    let (session, request) = harness.last_redis(|ask| matches!(ask, RedisAsk::Open { .. }));
    let info = KeyInfo {
        kind: KeyType::List,
        ttl_ms: None,
        memory: None,
        encoding: None,
        length: Some(3),
    };
    harness.app.apply(Action::Backend(Event::Redis {
        session,
        request,
        answer: RedisAnswer::Opened {
            key: b"queue:emails".to_vec(),
            result: Ok((
                info,
                Loaded {
                    page: Page::Items(vec![b"a".to_vec(), b"b".to_vec()]),
                    next: Some(Cursor::Offset(2)),
                    command: "LRANGE",
                    took: Duration::ZERO,
                },
            )),
        },
    }));
    harness.click("Load more (2 of 3)");
    let Some(RedisAsk::Read { from, kind, .. }) = asks(&harness).last().copied() else {
        panic!("expected a Read");
    };
    assert_eq!((from, kind), (&Cursor::Offset(2), &KeyType::List));
}

/// The dialog with Redis chosen.
fn redis_dialog(harness: &mut Harness) {
    harness.app.apply(Action::NewConnection);
    harness.click("Redis");
}

fn form(harness: &mut Harness) -> &mut crate::model::ConnectionForm {
    match &mut harness.app.dialog {
        Some(Dialog::Connection(form)) => form,
        _ => panic!("the connection dialog is open"),
    }
}

#[test]
fn choosing_redis_shows_its_groups() {
    let mut harness = Harness::new();
    redis_dialog(&mut harness);
    assert_eq!(form(&mut harness).port, "6379");
    let tree = harness.settle();
    let labels = labels(&tree);
    for label in [
        "Server",
        "Sign in",
        "Security",
        "Browsing",
        "Username",
        "Database",
        "Use TLS (rediss://)",
        "Connect through SSH tunnel",
        "Open read-only",
    ] {
        assert!(
            labels.iter().any(|found| found == label),
            "{label}: {labels:?}"
        );
    }
    // TLS shows what it needs once it is on.
    harness.click("Use TLS (rediss://)");
    assert!(harness.has("Verify the server's hostname"));
    assert!(harness.has("Client certificate"));
    // Browsing opens on its fields.
    harness.click("Browsing");
    assert!(harness.has("Key separator"));
    assert!(harness.has("1,000"));
    assert!(harness.has("Key limit"));
}

#[test]
fn the_url_tab_and_the_fields_stay_in_step_without_the_password() {
    let mut harness = Harness::new();
    redis_dialog(&mut harness);
    {
        let form = form(&mut harness);
        form.name = "Bookshop cache".into();
        form.host = "cache.example.com".into();
        form.user = "reader".into();
        form.password = "hunter2".into();
        form.redis_database = 3;
        form.redis_tls = true;
    }
    harness.click("URL");
    // Not typed in yet: it shows the fields' URL, the password as dots.
    harness.frame(vec![egui::Event::Key {
        key: Key::Tab,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: Modifiers::NONE,
    }]);
    harness.settle();
    let url = form(&mut harness).url.clone();
    assert_eq!(url, "rediss://reader:••••@cache.example.com:6379/3");
    // What is typed fills the fields.
    form(&mut harness).url = "redis://app:s3cret@db.example.com:7000/5".into();
    harness.app.apply(Action::RedisUrlTyped);
    let form_now = form(&mut harness);
    assert_eq!(form_now.host, "db.example.com");
    assert_eq!(form_now.port, "7000");
    assert_eq!(form_now.user, "app");
    assert_eq!(form_now.redis_database, 5);
    assert!(!form_now.redis_tls);
    assert_eq!(form_now.password, "s3cret");
    // The dots stand for the password kept.
    form_now.url = "redis://app:••••@db.example.com:7000/5".into();
    harness.app.apply(Action::RedisUrlTyped);
    assert_eq!(form(&mut harness).password, "s3cret");
    let saved = form(&mut harness).to_saved().unwrap();
    assert_eq!(saved.spec.redis_spec().database, 5);
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("connections.toml");
    let mut connections = crate::connections::SavedConnections::default();
    connections.upsert(saved);
    connections.save(&path).unwrap();
    let text = std::fs::read_to_string(&path).unwrap();
    assert!(!text.contains("s3cret"), "{text}");
    assert!(!text.contains("••••"), "{text}");
}

#[test]
fn a_pasted_redis_cli_line_fills_the_connection() {
    let mut harness = Harness::new();
    redis_dialog(&mut harness);
    form(&mut harness).url_mode = true;
    form(&mut harness).url =
        "redis-cli -h cache.example.com -p 6380 -n 2 --user reader -a hunter2 --tls".into();
    harness.app.apply(Action::ApplyUrl);
    let form = form(&mut harness);
    assert_eq!(form.message, None);
    assert_eq!(form.host, "cache.example.com");
    assert_eq!(form.port, "6380");
    assert_eq!(form.user, "reader");
    assert_eq!(form.redis_database, 2);
    assert!(form.redis_tls);
    assert_eq!(form.password, "hunter2");
    let saved = form.to_saved().unwrap();
    assert_eq!(saved.spec.driver, tabletist_db::Driver::Redis);
}

/// What the footer says after a Test that answered `result`.
fn tested(result: Result<Option<tabletist_db::redis::Probe>, tabletist_db::Error>) -> Harness {
    let mut harness = Harness::new();
    redis_dialog(&mut harness);
    harness.click("Test");
    let request = match form(&mut harness).test {
        TestState::Running(request) => request,
        ref other => panic!("a Test runs: {other:?}"),
    };
    harness
        .app
        .apply(Action::Backend(Event::Tested { request, result }));
    harness.settle();
    harness
}

#[test]
fn each_test_state_says_its_cause_and_keeps_the_servers_code() {
    let probe = crate::testing::bookshop_probe(0);
    let harness = tested(Ok(Some(probe.clone())));
    assert!(
        harness
            .painted
            .iter()
            .any(|(text, _)| text.starts_with("Connected · Redis 7.4.1 · db 0 · 65 keys")),
        "{:?}",
        harness.painted
    );
    let mut valkey = probe.clone();
    valkey.server = tabletist_db::redis::Server {
        name: "Valkey".into(),
        version: "8.0.1".into(),
    };
    let harness = tested(Ok(Some(valkey)));
    assert!(harness.painted.iter().any(|(text, _)| {
        text.starts_with("Connected · Valkey 8.0.1, Redis-compatible · db 0 · 65 keys")
    }));
    let mut refused = probe;
    refused.refused = Some("SCAN".into());
    let harness = tested(Ok(Some(refused)));
    let warning =
        "Connected, but this user can't run INFO or SCAN, so keys can't be listed (NOPERM)";
    assert!(harness.painted.iter().any(|(text, _)| text == warning));
    use tabletist_db::Error;
    for (error, said) in [
        (
            Error::Auth("WRONGPASS invalid username-password pair".into()),
            "Wrong username or password (WRONGPASS)",
        ),
        (
            Error::Auth("NOAUTH Authentication required.".into()),
            "This server needs a password (NOAUTH)",
        ),
        (
            Error::Connect(tabletist_db::redis::CLOSED_IN_HANDSHAKE.into()),
            "The server closed the connection. It may need TLS: turn on Use TLS.",
        ),
        (
            Error::Connect(tabletist_db::redis::REFUSED.into()),
            "Nothing is listening on localhost:6379 (connection refused)",
        ),
        (Error::Timeout, "No answer from localhost:6379 after 10 s"),
    ] {
        let mut harness = tested(Err(error));
        assert_eq!(
            form(&mut harness).test,
            TestState::Failed(said.into()),
            "{said}"
        );
    }
}

#[test]
fn the_agents_row_says_redis_has_none_yet() {
    if !crate::endpoint::AVAILABLE {
        return;
    }
    let mut harness = Harness::new();
    redis_dialog(&mut harness);
    assert!(harness.has("Agents for Redis come later"));
}

/// Quick open's results in a Redis window for `query`, as listed.
fn quick_results(harness: &mut Harness, query: &str) -> Vec<crate::model::QuickItem> {
    harness.app.apply(Action::QuickOpenTyped(query.into()));
    harness.settle();
    let Some(Dialog::QuickOpen(open)) = &harness.app.dialog else {
        panic!("quick open is open");
    };
    harness.app.quick_open_list(open).items().cloned().collect()
}

#[test]
fn quick_open_lists_a_redis_windows_groups_and_keys() {
    let mut harness = Harness::new();
    let tab = harness.connect_redis();
    let results = quick_results(&mut harness, "book");
    let names: Vec<(String, bool)> = results
        .iter()
        .filter_map(|item| match item {
            crate::model::QuickItem::Key { found, .. } => {
                Some((crate::keyspace::lossy(&found.name), found.group.is_some()))
            }
            _ => None,
        })
        .collect();
    // The group first, then its keys as a person counts them.
    assert_eq!(names[0], ("book:".into(), true));
    assert_eq!(names[1], ("book:1".into(), false));
    assert_eq!(names[2], ("book:2".into(), false));
    // A key's type, where it was read; no table or view of SQL.
    assert!(
        results
            .iter()
            .all(|item| !matches!(item, crate::model::QuickItem::Object { .. }))
    );
    // The group opens as its keys: the filter says `book:*`, and the scan
    // starts again with it.
    harness.app.apply(Action::QuickOpenPick {
        index: Some(0),
        in_editor: false,
    });
    assert_eq!(browser(&harness, tab).filter, "book:*");
    let Some(RedisAsk::Scan { pattern, .. }) = asks(&harness).last().copied() else {
        panic!("expected a Scan");
    };
    assert_eq!(pattern, "book:*");
    // A key opens in a tab.
    harness.connect_redis();
    let tab = harness.app.active_tab_id();
    let results = quick_results(&mut harness, "queue");
    let at = results
        .iter()
        .position(|item| matches!(item, crate::model::QuickItem::Key { found, .. } if found.name == b"queue:emails"))
        .expect("queue:emails is listed");
    harness.app.apply(Action::QuickOpenPick {
        index: Some(at),
        in_editor: false,
    });
    let Some(RedisAsk::Open { key, .. }) = asks(&harness).last().copied() else {
        panic!("expected an Open");
    };
    assert_eq!(key.as_slice(), b"queue:emails");
    assert_eq!(browser(&harness, tab).tabs.len(), 1);
}

#[test]
fn a_test_of_a_saved_redis_connection_keeps_its_key_count_for_the_list() {
    let mut harness = Harness::new();
    let saved = crate::testing::bookshop_cache();
    let conn = saved.id.clone();
    harness.app.connections.upsert(saved);
    let mut test = |probe: tabletist_db::redis::Probe| {
        harness.app.apply(Action::EditConnection(conn.clone()));
        harness.click("Test");
        let request = match form(&mut harness).test {
            TestState::Running(request) => request,
            ref other => panic!("a Test runs: {other:?}"),
        };
        harness.app.apply(Action::Backend(Event::Tested {
            request,
            result: Ok(Some(probe)),
        }));
        let counted = harness.app.redis_keys.get(&conn).copied();
        harness.app.apply(Action::CloseDialog);
        counted
    };
    // DBSIZE's count where the server gave it.
    let probe = crate::testing::bookshop_probe(0);
    assert_eq!(test(probe.clone()), Some((0, 65)));
    // Else INFO keyspace's for the database.
    let mut without = probe;
    without.keys = None;
    without.database = 2;
    assert_eq!(test(without), Some((2, 38)));
}

#[test]
fn a_half_client_pair_with_tls_off_saves_a_file_that_reads_back() {
    let mut harness = Harness::new();
    redis_dialog(&mut harness);
    {
        let form = form(&mut harness);
        form.name = "Bookshop cache".into();
        form.redis_tls = false;
        form.redis_cert = "/etc/ssl/app.pem".into();
    }
    let saved = form(&mut harness).to_saved().expect("TLS off saves");
    assert_eq!(saved.spec.redis_spec().tls.cert, None);
    let dir = tempfile::tempdir().unwrap();
    let dirs = crate::paths::AppDirs::at(dir.path());
    let mut connections = crate::connections::SavedConnections::default();
    connections.upsert(saved);
    connections.save(&dirs.connections_file()).unwrap();
    let loaded = crate::connections::SavedConnections::load(&dirs);
    assert_eq!(loaded.store.connections.len(), 1, "{:?}", loaded.problem);
    // With TLS on, half a pair is said, not saved.
    form(&mut harness).redis_tls = true;
    assert!(form(&mut harness).to_saved().is_err());
}

#[test]
fn a_lost_redis_connection_says_so_with_reconnect() {
    let mut harness = Harness::new();
    let tab = harness.connect_redis();
    assert!(!harness.app.workspace(tab).unwrap().on_start());
    let session = harness.app.workspace(tab).unwrap().session;
    harness.app.apply(Action::Backend(Event::Disconnected {
        session,
        error: tabletist_db::Error::ConnectionLost("server closed the connection".into()),
    }));
    assert!(harness.has("Reconnect"), "{:?}", labels(&harness.settle()));
}

#[test]
fn the_database_menu_marks_the_one_the_window_is_on() {
    let mut harness = Harness::new();
    let tab = harness.connect_redis();
    let info = super::workspace::bar_info(&harness.app, tab).unwrap();
    assert_eq!(info.database, "0 · 65 keys");
    assert!(info.databases.contains(&info.database));
    // Choosing it switches nothing.
    let before = asks(&harness).len();
    harness.app.apply(Action::SwitchDatabase {
        tab,
        database: info.database.clone(),
    });
    assert_eq!(asks(&harness).len(), before);
}

#[test]
fn every_window_has_its_own_group_when_quick_open_searches_them_all() {
    let mut harness = Harness::new();
    let first = harness.connect_redis();
    harness.app.apply(Action::ShowConnections);
    let second = harness.connect_redis();
    assert_ne!(first, second);
    harness.app.apply(Action::QuickOpenTyped("session".into()));
    harness.app.apply(Action::QuickOpenScope);
    harness.settle();
    let Some(Dialog::QuickOpen(open)) = &harness.app.dialog else {
        panic!("quick open is open");
    };
    assert!(open.everywhere);
    let groups: Vec<crate::model::QuickGroup> = harness
        .app
        .quick_open_list(open)
        .items()
        .filter_map(|item| match item {
            crate::model::QuickItem::More { group, .. } => Some(*group),
            _ => None,
        })
        .collect();
    // One "more" row each, of its own window (the one quick open was
    // opened from first).
    assert_eq!(groups.len(), 2);
    assert!(groups.contains(&crate::model::QuickGroup::Connection(first)));
    assert!(groups.contains(&crate::model::QuickGroup::Connection(second)));
}

#[test]
fn quick_opens_group_escapes_the_globs_characters() {
    let mut harness = Harness::new();
    let tab = harness.connect_redis_unscanned();
    step(
        &mut harness,
        &["a[b]:1", "a[b]:2", "a[b]:3"],
        3,
        ScanEnd::Done,
    );
    let results = quick_results(&mut harness, "a[b");
    assert!(matches!(
        results.first(),
        Some(crate::model::QuickItem::Key { found, .. }) if found.group.is_some()
    ));
    harness.app.apply(Action::QuickOpenPick {
        index: Some(0),
        in_editor: false,
    });
    assert_eq!(browser(&harness, tab).filter, "a\\[b\\]:*");
}

#[test]
fn a_reconnect_goes_back_to_the_database_switched_to_and_keeps_the_keys_open() {
    let mut harness = Harness::new();
    let tab = harness.connect_redis();
    // Switched to database 2, its scan done, a key open.
    harness.app.apply(Action::SwitchDatabase {
        tab,
        database: "2 · 38 keys".into(),
    });
    let selected = |harness: &mut Harness, database: u32| {
        let (session, request) = harness.last_redis(|ask| matches!(ask, RedisAsk::Select { .. }));
        harness.app.apply(Action::Backend(Event::Redis {
            session,
            request,
            answer: RedisAnswer::Selected {
                database,
                result: Ok(()),
            },
        }));
    };
    selected(&mut harness, 2);
    step(&mut harness, &["item:1"], 38, ScanEnd::Done);
    harness.app.apply(Action::Redis(
        tab,
        RedisAction::OpenKey(Box::from(&b"item:1"[..])),
    ));
    // The connection drops, and comes back.
    let session = harness.app.workspace(tab).unwrap().session;
    harness.app.apply(Action::Backend(Event::Disconnected {
        session,
        error: tabletist_db::Error::ConnectionLost("gone".into()),
    }));
    harness.app.apply(Action::Reconnect(tab));
    let before = asks(&harness).len();
    let (session, request) = harness
        .app
        .backend
        .sent
        .iter()
        .rev()
        .find_map(|command| match command {
            Command::Connect {
                session, request, ..
            } => Some((*session, *request)),
            _ => None,
        })
        .expect("a Connect");
    harness.app.apply(Action::Backend(Event::Connected {
        session,
        request,
        driver: tabletist_db::Driver::Redis,
        encrypted: false,
        access: crate::testing::asked_access(&harness.app),
    }));
    // Only SELECT: nothing is asked of the first database meanwhile.
    let after: Vec<&RedisAsk> = asks(&harness)[before..].to_vec();
    assert!(
        matches!(after.as_slice(), [RedisAsk::Select { database: 2 }]),
        "{after:?}"
    );
    selected(&mut harness, 2);
    let after: Vec<&RedisAsk> = asks(&harness)[before..].to_vec();
    // A probe of it; no scan, which had ended; the key stays open.
    assert!(
        matches!(
            after.as_slice(),
            [RedisAsk::Select { .. }, RedisAsk::Probe { database: 2 }]
        ),
        "{after:?}"
    );
    assert_eq!(browser(&harness, tab).database, 2);
    assert_eq!(browser(&harness, tab).tabs.len(), 1);
    assert_eq!(browser(&harness, tab).tree.len(), 1);
}

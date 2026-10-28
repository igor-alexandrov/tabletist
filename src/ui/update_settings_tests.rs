//! Settings › Updates, in both looks: the pane, the status card in each of
//! its states, its signals elsewhere, how often a check runs, and the
//! terminal look's keys and `:update`.

use egui::accesskit::{Live, Role};
use egui::{Key, Modifiers};

use crate::backend::{Command, UpdatesCommand};
use crate::model::{Action, Dialog, SettingsPane};
use crate::settings::UpdateCheck;
use crate::testing::{Harness, node};
use crate::theme::Look;
use crate::update::Phase;
use crate::update::check::Failed;
use crate::update::installed::{Installed, Managed};
use crate::update::release::{self, Kind, Latest, Offer, Os};

fn looks() -> [Look; 2] {
    [Look::macos(), Look::omarchy()]
}

/// 2026-10-27 09:12 UTC.
const NOW: i64 = 1_793_092_320;

/// A release far newer than this version, as the fake server gives it.
fn offer() -> Offer {
    let json = crate::update::check::tests::RELEASE
        .replace("v0.4.0", "v99.0.0")
        .replace("0.4.0", "99.0.0");
    let latest: Latest = serde_json::from_str(&json).unwrap();
    let mut offer = release::offer(&latest, Os::current(), Some(Kind::Portable));
    offer
        .download
        .get_or_insert(("tabletist-v99.0.0.tar.gz".into(), 31_200_000));
    offer
}

fn harness(look: Look) -> Harness {
    let mut harness = Harness::new();
    harness.set_look(look);
    harness.app.stopped_clock = Some(jiff::Timestamp::from_second(NOW).unwrap());
    harness
}

/// Settings, on its Updates pane.
fn on_the_pane(harness: &mut Harness) {
    harness
        .app
        .apply(Action::ShowSettingsPane(SettingsPane::Updates));
    harness.settle();
}

fn pane(harness: &Harness) -> Option<SettingsPane> {
    match &harness.app.dialog {
        Some(Dialog::Settings(dialog)) => Some(dialog.pane),
        _ => None,
    }
}

/// The words as `look` writes them.
fn said(look: &Look, words: &str) -> String {
    look.label(words)
}

/// A capital typed: its key with Shift, and the letter.
fn capital(harness: &mut Harness, key: Key) {
    harness.settle();
    let letter = format!("{key:?}");
    harness.frame(vec![
        crate::testing::key(key, Modifiers::SHIFT),
        egui::Event::Text(letter),
    ]);
    harness.frame(vec![crate::testing::release(key, Modifiers::SHIFT)]);
    harness.settle();
}

/// The items of the notes' list a screen reader hears. Between them and
/// the list is the node egui makes for the `Ui` they are drawn in: a
/// generic container, which AccessKit leaves out of what is told, so the
/// items are the list's there.
fn list_items(tree: &egui::accesskit::TreeUpdate) -> usize {
    let node_of = |wanted: egui::accesskit::NodeId| {
        tree.nodes
            .iter()
            .find(|(id, _)| *id == wanted)
            .map(|(_, node)| node)
            .expect("a node of the tree")
    };
    let list = node_of(node(tree, "What's new", Role::List).expect("the notes"));
    let [between] = list.children() else {
        panic!("{:?}", list.children());
    };
    let between = node_of(*between);
    assert_eq!(between.role(), Role::GenericContainer);
    let roles: Vec<Role> = between
        .children()
        .iter()
        .map(|child| node_of(*child).role())
        .collect();
    assert!(
        roles.iter().all(|role| *role == Role::ListItem),
        "{roles:?}"
    );
    roles.len()
}

fn checks(harness: &Harness) -> usize {
    harness
        .app
        .backend
        .sent
        .iter()
        .filter(|sent| matches!(sent, Command::Updates(UpdatesCommand::Check { .. })))
        .count()
}

/// Check for Updates… stays enabled with Settings open on another pane,
/// and a click there shows Updates and checks once.
#[test]
fn check_for_updates_in_the_menu_works_with_settings_open() {
    use crate::keymap::Command as Keyed;
    use crate::menu::Item;
    let mut harness = harness(Look::macos());
    harness
        .app
        .apply(Action::ShowSettingsPane(SettingsPane::General));
    harness.settle();
    let item = Item::Command(Keyed::CheckForUpdates);
    assert!(harness.app.menu_state(item).enabled);
    harness.app.choose(item);
    harness.settle();
    assert_eq!(pane(&harness), Some(SettingsPane::Updates));
    assert_eq!(checks(&harness), 1);
}

#[test]
fn settings_lists_an_updates_pane_last() {
    for look in looks() {
        let mut harness = harness(look);
        harness.app.apply(Action::ShowSettings);
        harness.settle();
        let listed = SettingsPane::listed();
        assert_eq!(listed.last(), Some(&SettingsPane::Updates));
        if look.terminal {
            for _ in 1..listed.len() {
                harness.press(Key::L, Modifiers::CTRL);
            }
        } else {
            harness.click("Updates");
        }
        harness.settle();
        assert_eq!(pane(&harness), Some(SettingsPane::Updates), "{}", look.name);
        // The row's control, named by the option in either look.
        assert!(harness.has("Check for updates"), "{}", look.name);
    }
}

#[test]
fn the_card_says_where_updates_stand_in_a_region_of_its_own() {
    for look in looks() {
        let mut harness = harness(look);
        on_the_pane(&mut harness);
        let tree = harness.settle();
        assert!(node(&tree, &said(&look, "Update status"), Role::Region).is_some());
        let current = crate::update::CURRENT;
        assert!(
            harness.has(&said(&look, &format!("Tabletist {current}"))),
            "{}",
            look.name
        );

        // Up to date, checked today.
        harness.app.updates.seen.last_checked = Some(u64::try_from(NOW).unwrap() - 60);
        let title = said(&look, &format!("Tabletist {current} is up to date"));
        assert!(harness.has(&title), "{}", look.name);
        let tree = harness.settle();
        let id = node(&tree, &title, Role::Label).expect("the title");
        let title_node = tree.nodes.iter().find(|(at, _)| *at == id).unwrap();
        assert_eq!(title_node.1.live(), Some(Live::Polite), "{}", look.name);

        // Checking, with what it asks.
        harness.app.apply(Action::CheckForUpdates);
        assert!(
            harness.has(&said(&look, "Checking for updates…")),
            "{}",
            look.name
        );
        assert!(harness.has(crate::update::check::ASKED), "{}", look.name);
        if !look.terminal {
            harness.click("Cancel");
            harness.settle();
            assert!(!harness.app.updates.checking());
        }

        // Couldn't check: the error told at once.
        harness.app.updates.phase = Phase::Failed {
            failed: Failed::TimedOut(std::time::Duration::from_secs(10)),
            asked_for: true,
        };
        let tree = harness.settle();
        assert!(
            harness.has(&said(&look, "Couldn't check for updates")),
            "{}: {:?}",
            look.name,
            crate::testing::labels(&tree)
        );
        let error = said(
            &look,
            "Couldn't reach github.com: the connection timed out after 10 s.",
        );
        let tree = harness.settle();
        let id = node(&tree, &error, Role::Label).expect("the error");
        let error_node = tree.nodes.iter().find(|(at, _)| *at == id).unwrap();
        assert_eq!(error_node.1.live(), Some(Live::Assertive), "{}", look.name);
    }
}

#[test]
fn an_update_available_shows_its_notes_and_a_dot_on_updates() {
    for look in looks() {
        let mut harness = harness(look);
        harness.app.updates.installed = Some(Installed::Itself(Kind::Portable));
        on_the_pane(&mut harness);
        let dot = "Update available";
        assert!(!harness.has(dot), "{}", look.name);
        harness.app.updates.seen.offer = Some(offer());
        // The terminal look's block says it as its board does.
        let title = match look.terminal {
            true => "99.0.0 available",
            false => "Tabletist 99.0.0 is available",
        };
        assert!(harness.has(title), "{}", look.name);
        assert!(harness.has(dot), "{}", look.name);
        // Four of the five points.
        if look.terminal {
            assert!(harness.has("- date and time picker, ddl view and query history"));
            assert!(!harness.has("- settings › updates"));
        } else {
            let tree = harness.settle();
            assert_eq!(list_items(&tree), 4);
            assert!(harness.has("Date and time picker, DDL view and query history"));
        }
        // You have this version, the day it came out, and the download's
        // size.
        let offer = offer();
        let size = release::megabytes(offer.download.unwrap().1);
        let then = std::time::UNIX_EPOCH + std::time::Duration::from_secs(offer.published.unwrap());
        let now =
            std::time::UNIX_EPOCH + std::time::Duration::from_secs(u64::try_from(NOW).unwrap());
        let day = match crate::clock::day_of(then, now) {
            crate::clock::Day::Today => "today".to_owned(),
            crate::clock::Day::Yesterday => "yesterday".to_owned(),
            crate::clock::Day::On(day) => day,
        };
        let sub = format!(
            "You have {} · released {day} · {size}",
            crate::update::CURRENT
        );
        if look.terminal {
            let sub = format!(
                "{} · {} {}",
                look.label(&sub),
                std::env::consts::OS,
                std::env::consts::ARCH
            );
            assert!(harness.has(&sub), "{}: {sub}", look.name);
        } else {
            assert!(harness.has(&sub), "{}: {sub}", look.name);
        }
    }
}

#[test]
fn release_notes_on_github_opens_the_page_and_skip_stops_offering_it() {
    let look = Look::macos();
    let mut harness = harness(look);
    harness.app.updates.installed = Some(Installed::Itself(Kind::Portable));
    harness.app.updates.seen.offer = Some(offer());
    on_the_pane(&mut harness);
    harness.click("Release notes on GitHub");
    harness.settle();
    assert_eq!(
        harness.opened.as_deref(),
        Some("https://github.com/igor-alexandrov/tabletist/releases/tag/v99.0.0")
    );
    harness.click("Skip this version");
    harness.settle();
    assert_eq!(harness.app.settings.update_skip.as_deref(), Some("99.0.0"));
    assert!(!harness.has("Tabletist 99.0.0 is available"));
    assert!(!harness.has("Update available"));
}

#[test]
fn a_release_with_no_build_here_offers_its_page_only() {
    for look in looks() {
        let mut harness = harness(look);
        harness.app.updates.installed = Some(Installed::Itself(Kind::Portable));
        let mut offer = offer();
        offer.download = None;
        harness.app.updates.seen.offer = Some(offer);
        on_the_pane(&mut harness);
        let said_here = said(&look, "99.0.0 has no build for this computer yet");
        assert!(harness.has(&said_here), "{}", look.name);
    }
}

#[test]
fn a_copy_something_else_installed_says_what_updates_it_and_copies_the_command() {
    for look in looks() {
        let mut harness = harness(look);
        harness.app.updates.installed = Some(Installed::Elsewhere(Managed {
            said: "Installed with Homebrew, so Homebrew updates it",
            command: Some("brew upgrade --cask tabletist"),
        }));
        harness.app.updates.seen.offer = Some(offer());
        on_the_pane(&mut harness);
        assert!(
            harness.has(&said(&look, "99.0.0 is available")),
            "{}",
            look.name
        );
        let managed = said(&look, "Installed with Homebrew, so Homebrew updates it");
        assert!(harness.has(&managed), "{}", look.name);
        if look.terminal {
            assert!(harness.has("brew upgrade --cask tabletist"));
            crate::ui::tests::type_key(&mut harness, Key::Y, "y");
        } else {
            assert!(harness.has("brew upgrade --cask tabletist"));
            // No download offered: something else installs it.
            assert!(!harness.has("Skip this version"));
            harness.click("Copy");
        }
        harness.settle();
        assert_eq!(
            harness.copied.as_deref(),
            Some("brew upgrade --cask tabletist"),
            "{}",
            look.name
        );
    }
}

#[test]
fn check_now_and_try_again_check() {
    let mut harness = harness(Look::macos());
    harness.app.updates.seen.last_checked = Some(u64::try_from(NOW).unwrap() - 3_600);
    on_the_pane(&mut harness);
    // In the computer's zone, as the clock writes an hour.
    let then =
        std::time::UNIX_EPOCH + std::time::Duration::from_secs(u64::try_from(NOW).unwrap() - 3_600);
    let hour = crate::clock::hour(then);
    assert!(harness.has(&format!("Last checked today at {hour}")));
    // The card's button, and the row's link: either checks.
    harness.click("Check now");
    harness.settle();
    assert_eq!(checks(&harness), 1);
    harness.app.updates.phase = Phase::Failed {
        failed: Failed::Unreachable,
        asked_for: true,
    };
    harness.click("Try again");
    harness.settle();
    assert_eq!(checks(&harness), 2);
}

#[test]
fn how_often_a_check_runs_is_set_and_written() {
    for look in looks() {
        let mut harness = harness(look);
        on_the_pane(&mut harness);
        if look.terminal {
            crate::ui::tests::type_key(&mut harness, Key::L, "l");
        } else {
            harness.click("Check for updates");
            harness.click("Check for updates Every week");
        }
        harness.settle();
        assert_eq!(
            harness.app.settings.update_check,
            UpdateCheck::Weekly,
            "{}",
            look.name
        );
        let text = &harness.app.settings_file.text;
        assert!(
            text.contains("[updates]\ncheck = \"weekly\"  # daily | weekly | never"),
            "{}: {text}",
            look.name
        );
        // Only when I ask, the end: no further.
        if look.terminal {
            crate::ui::tests::type_key(&mut harness, Key::L, "l");
            crate::ui::tests::type_key(&mut harness, Key::L, "l");
            assert_eq!(harness.app.settings.update_check, UpdateCheck::Never);
            // R puts back the default, which the file leaves out.
            capital(&mut harness, Key::R);
            assert_eq!(harness.app.settings.update_check, UpdateCheck::Daily);
            assert!(!harness.app.settings_file.text.contains("[updates]"));
        }
    }
}

#[test]
fn the_pane_says_what_the_check_sends() {
    for look in looks() {
        let mut harness = harness(look);
        on_the_pane(&mut harness);
        let words = if look.terminal {
            "the check asks github for the latest release and sends this version, os and cpu, nothing else."
        } else if cfg!(windows) {
            "The check asks GitHub for the latest release. It sends this version, Windows and the processor, nothing else: no ID, no usage data, no database details."
        } else {
            "The check asks GitHub for the latest release. It sends this version, the macOS version and the chip, nothing else: no ID, no usage data, no database details."
        };
        assert!(harness.has(words), "{}", look.name);
    }
}

#[test]
fn the_terminal_keys_check_show_the_notes_and_skip() {
    let look = Look::omarchy();
    let mut harness = harness(look);
    harness.app.updates.installed = Some(Installed::Itself(Kind::Portable));
    on_the_pane(&mut harness);
    crate::ui::tests::type_key(&mut harness, Key::C, "c");
    assert_eq!(checks(&harness), 1);
    harness.app.apply(Action::CancelUpdateCheck);
    harness.app.updates.seen.offer = Some(offer());
    // The block's keys, as the keymap writes them.
    assert!(harness.has("n release notes  S skip 99.0.0"));
    crate::ui::tests::type_key(&mut harness, Key::N, "n");
    let shown = harness
        .app
        .backend
        .sent
        .iter()
        .any(|sent| matches!(sent, Command::Updates(UpdatesCommand::ShowNotes { .. })));
    assert!(shown);
    capital(&mut harness, Key::S);
    assert_eq!(harness.app.settings.update_skip.as_deref(), Some("99.0.0"));
    // Off this pane, `n` is the history's, and `c` nothing.
    harness.press(Key::H, Modifiers::CTRL);
    harness.settle();
    let before = checks(&harness);
    crate::ui::tests::type_key(&mut harness, Key::C, "c");
    assert_eq!(checks(&harness), before);
}

#[test]
fn update_on_the_prompt_shows_the_pane_and_checks() {
    let mut harness = harness(Look::omarchy());
    let tab = harness.connect_fake();
    harness.app.workspace_mut(tab).unwrap().command = Some("update".into());
    harness.app.apply(Action::RunCommand(tab));
    harness.settle();
    assert_eq!(pane(&harness), Some(SettingsPane::Updates));
    assert_eq!(checks(&harness), 1);
}

#[test]
fn the_status_line_tells_of_a_version_for_eight_seconds() {
    let mut harness = harness(Look::omarchy());
    let _tab = harness.connect_fake();
    harness.app.update_announcement = Some("99.0.0".into());
    let ctx = harness.ctx.clone();
    harness.app.say_updates(&ctx);
    assert_eq!(
        crate::ui::toast::said(&harness.ctx).as_deref(),
        Some("99.0.0 available · :update")
    );
    // macOS and Windows say nothing of it but the dot.
    let mut mac = Harness::new();
    mac.set_look(Look::macos());
    mac.app.update_announcement = Some("99.0.0".into());
    let ctx = mac.ctx.clone();
    mac.app.say_updates(&ctx);
    assert_eq!(crate::ui::toast::said(&mac.ctx), None);
}

#[test]
fn the_settings_file_sets_the_check_live_and_marks_a_bad_value() {
    let mut harness = harness(Look::omarchy());
    let text = "[updates]\ncheck = \"never\"\nskip = \"v0.9.0\"\n".to_owned();
    let file = crate::backend::Event::SettingsFile { text, own: false };
    harness.app.apply(Action::Backend(file));
    assert_eq!(harness.app.settings.update_check, UpdateCheck::Never);
    assert_eq!(harness.app.settings.update_skip.as_deref(), Some("0.9.0"));
    assert!(harness.app.settings_file.invalid.is_empty());
    let text = "[updates]\ncheck = \"hourly\"\nskip = \"soon\"\n".to_owned();
    let file = crate::backend::Event::SettingsFile { text, own: false };
    harness.app.apply(Action::Backend(file));
    // Each bad line is marked, and leaves the value as it was.
    assert_eq!(harness.app.settings_file.invalid, vec![2, 3]);
    assert_eq!(harness.app.settings.update_check, UpdateCheck::Never);
    assert_eq!(harness.app.settings.update_skip.as_deref(), Some("0.9.0"));
}

/// Two notes alike are two points of the list a screen reader hears.
#[test]
fn two_notes_alike_are_two_items() {
    let mut harness = harness(Look::macos());
    harness.app.updates.installed = Some(Installed::Itself(Kind::Portable));
    let mut offer = offer();
    offer.notes = vec![
        "Fixes".into(),
        "Fixes".into(),
        "Faster grid".into(),
        "Dark mode".into(),
    ];
    harness.app.updates.seen.offer = Some(offer);
    on_the_pane(&mut harness);
    let tree = harness.settle();
    assert_eq!(list_items(&tree), 4, "{:?}", crate::testing::labels(&tree));
}

/// While a check runs, the version found before is still available: the
/// dot on Updates stays.
#[test]
fn the_dot_stays_while_a_check_runs() {
    for look in looks() {
        let mut harness = harness(look);
        harness.app.updates.installed = Some(Installed::Itself(Kind::Portable));
        harness.app.updates.seen.offer = Some(offer());
        on_the_pane(&mut harness);
        assert!(harness.has("Update available"), "{}", look.name);
        harness.app.apply(Action::CheckForUpdates);
        harness.settle();
        assert!(harness.app.updates.checking());
        assert!(
            harness.has("Update available"),
            "{}: the dot went while a check runs",
            look.name
        );
    }
}

/// An error is told once, assertively, by its own line; the title
/// politely.
#[test]
fn an_error_is_told_once() {
    for look in looks() {
        let mut harness = harness(look);
        on_the_pane(&mut harness);
        harness.app.updates.phase = Phase::Failed {
            failed: Failed::Unreachable,
            asked_for: true,
        };
        let tree = harness.settle();
        let assertive: Vec<_> = tree
            .nodes
            .iter()
            .filter(|(_, node)| node.live() == Some(Live::Assertive))
            .map(|(_, node)| node.label().or(node.value()).map(str::to_owned))
            .collect();
        assert_eq!(assertive.len(), 1, "{}: {assertive:?}", look.name);
    }
}

/// Cancel holds as the person sees it, with the frames' own ticks: the
/// schedule's check does not start again.
#[test]
fn cancel_on_the_card_holds() {
    let mut harness = harness(Look::macos());
    harness.app.settings.update_check = UpdateCheck::Daily;
    harness.app.updates_scheduled = true;
    harness.app.updates.installed = Some(Installed::Itself(Kind::Portable));
    on_the_pane(&mut harness);
    harness.app.logic(&harness.ctx.clone());
    harness.settle();
    assert!(harness.has("Checking for updates…"));
    harness.click("Cancel");
    harness.app.logic(&harness.ctx.clone());
    harness.settle();
    assert!(!harness.has("Checking for updates…"));
}

/// A check the person asked for that fails is shown, though a version
/// found before is kept, and the dot still tells of it. One of the
/// schedule's that fails leaves the version shown.
#[test]
fn a_failed_check_asked_for_is_shown_over_a_version_found_before() {
    for look in looks() {
        let mut harness = harness(look);
        harness.app.updates.installed = Some(Installed::Itself(Kind::Portable));
        harness.app.updates.seen.offer = Some(offer());
        on_the_pane(&mut harness);
        harness.app.updates.phase = Phase::Failed {
            failed: Failed::Unreachable,
            asked_for: true,
        };
        harness.settle();
        let error = said(&look, "Couldn't check for updates");
        assert!(harness.has(&error), "{}", look.name);
        assert!(harness.has("Update available"), "{}", look.name);
        harness.app.updates.phase = Phase::Failed {
            failed: Failed::Unreachable,
            asked_for: false,
        };
        harness.settle();
        assert!(!harness.has(&error), "{}", look.name);
    }
}

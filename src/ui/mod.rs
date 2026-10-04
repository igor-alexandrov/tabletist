//! The interface. Views read `App` and push `Action`s; they never change
//! state directly.

pub mod about;
pub mod cell_editor;
#[cfg(test)]
mod complete_tests;
pub mod connect_dialog;
pub mod data_view;
#[cfg(test)]
mod env_tests;
pub mod filter_bar;
pub mod focus;
pub mod format;
pub mod grid;
pub mod help;
pub mod host_key_prompt;
pub mod json_view;
pub mod keys;
pub mod object_tabs;
pub mod password_prompt;
pub mod picker;
pub mod quick_open;
pub mod row_panel;
pub mod settings;
pub mod sidebar;
pub mod sql_complete;
pub mod sql_editor;
pub mod sql_results;
pub mod sql_text;
pub mod states;
pub mod structure;
pub mod value_tags;
pub mod widgets;
pub mod workspace;
pub mod write_prompts;

use egui::Frame;

use crate::app::App;
use crate::model::{Action, ConnTabContent};

pub fn show(app: &mut App, ui: &mut egui::Ui) {
    notice(app, ui);
    let fill = app.palette.window;
    egui::CentralPanel::default()
        .frame(Frame::new().fill(fill))
        .show(ui, |ui| {
            let tab = app.active_tab_id();
            if matches!(app.active_tab().content, ConnTabContent::Picker(_)) {
                picker::show(app, ui);
            } else {
                workspace::show(app, ui, tab);
            }
        });
    connect_dialog::show(app, &ui.ctx().clone());
    password_prompt::show(app, &ui.ctx().clone());
    host_key_prompt::show(app, &ui.ctx().clone());
    quick_open::show(app, &ui.ctx().clone());
    help::show(app, &ui.ctx().clone());
    about::show(app, &ui.ctx().clone());
    settings::show(app, &ui.ctx().clone());
    write_prompts::show(app, &ui.ctx().clone());
}

/// A problem worth the user's attention that belongs to no one tab (a
/// keyring write that failed), until dismissed.
fn notice(app: &mut App, ui: &mut egui::Ui) {
    let Some(message) = app.notice.clone() else {
        return;
    };
    let mut actions = Vec::new();
    egui::Panel::top(egui::Id::new("notice"))
        .resizable(false)
        .show_separator_line(false)
        .frame(
            Frame::new()
                .fill(notice_fill(&app.palette))
                .inner_margin(egui::Margin::symmetric(12, 8)),
        )
        .show(ui, |ui| notice_line(app, ui, &message, &mut actions));
    app.actions.extend(actions);
}

/// What a notice stands on.
fn notice_fill(palette: &crate::theme::Palette) -> egui::Color32 {
    palette.warning.gamma_multiply(0.15)
}

/// A notice on one line: what it says, and at the right the button that
/// dismisses it. The bar has it, and so has a screen that covers the bar.
fn notice_line(app: &App, ui: &mut egui::Ui, message: &str, actions: &mut Vec<Action>) {
    let look = &app.look;
    ui.horizontal(|ui| {
        crate::typography::Text::one(look, widgets::body(look), message, app.palette.text)
            .layout(ui.ctx())
            .label(ui);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let dismiss = crate::i18n::gettext(app.locale, "Dismiss");
            if widgets::button(ui, &dismiss, look).clicked() {
                actions.push(Action::DismissNotice);
            }
        });
    });
}

/// Where the window's own buttons (the macOS traffic lights) centre, in egui
/// points below the window's top: on the line of the bar that leads the
/// window, so the buttons and that bar's contents share one line.
pub fn window_buttons_line(app: &App, zoom: f32) -> f32 {
    if matches!(app.active_tab().content, ConnTabContent::Picker(_)) {
        picker::header_line(&app.look)
    } else {
        workspace::bar_line(app, zoom)
    }
}

#[cfg(test)]
mod tests {
    use crate::testing::Harness;
    use egui::{Key, Modifiers};

    #[test]
    fn every_look_lays_out_at_small_and_large_sizes() {
        for look in crate::theme::Look::ALL {
            for size in [egui::vec2(720.0, 480.0), egui::vec2(2560.0, 1440.0)] {
                let mut harness = Harness::with_size(size);
                harness.set_look(look);
                let tab = harness.connect_fake();
                harness.app.apply(crate::model::Action::OpenObject {
                    tab,
                    object: tabletist_db::ObjectRef::new("main", "users"),
                    kind: tabletist_db::ObjectKind::Table,
                    pin: true,
                });
                harness.answer_rows(crate::testing::page(3, false));
                let tree = harness.settle();
                for (label, role) in [
                    ("Connections", egui::accesskit::Role::Button),
                    ("Disconnect", egui::accesskit::Role::Button),
                    ("Refresh objects", egui::accesskit::Role::Button),
                    ("Row 1", egui::accesskit::Role::Button),
                ] {
                    // The terminal look refreshes by key, not a button.
                    if look.terminal && label == "Refresh objects" {
                        continue;
                    }
                    assert!(
                        crate::testing::node(&tree, label, role).is_some(),
                        "{label} missing in {} at {size:?}",
                        look.name
                    );
                }
                assert!(
                    tree.nodes.iter().any(|(_, node)| {
                        node.role() == egui::accesskit::Role::TextInput
                            && matches!(
                                node.placeholder(),
                                Some("Find any object…" | "filter objects")
                            )
                    }),
                    "the sidebar filter is missing in {} at {size:?}",
                    look.name
                );
                // A SQL editor keeps its controls on screen and apart.
                harness.app.apply(crate::model::Action::NewSqlTab(tab));
                let tree = harness.settle();
                let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, size);
                let controls: Vec<(&str, egui::Rect)> = [
                    ("Run", egui::accesskit::Role::Button),
                    ("Run all", egui::accesskit::Role::Button),
                    ("Limit", egui::accesskit::Role::ComboBox),
                    ("Timeout", egui::accesskit::Role::ComboBox),
                ]
                .into_iter()
                .map(|(label, role)| {
                    let rect = crate::testing::bounds(&tree, label, role)
                        .unwrap_or_else(|| panic!("{label} missing in {} at {size:?}", look.name));
                    assert!(
                        screen.contains_rect(rect),
                        "{label} at {rect:?} is off screen in {} at {size:?}",
                        look.name
                    );
                    (label, rect)
                })
                .collect();
                for (index, (label, rect)) in controls.iter().enumerate() {
                    for (other, other_rect) in &controls[index + 1..] {
                        assert!(
                            !rect.intersects(*other_rect),
                            "{label} overlaps {other} in {} at {size:?}",
                            look.name
                        );
                    }
                }
                // The connection dialog keeps its buttons on screen; the
                // PostgreSQL form with the SSH tunnel open is the tallest.
                harness.press(Key::N, Modifiers::COMMAND);
                harness.click(&look.label("PostgreSQL"));
                harness.click("Connect through SSH tunnel");
                let tree = harness.settle();
                let button =
                    crate::testing::bounds(&tree, "Save & Connect", egui::accesskit::Role::Button)
                        .expect("Save & Connect");
                let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, size);
                assert!(
                    screen.contains_rect(button),
                    "Save & Connect at {button:?} is off screen in {} at {size:?}",
                    look.name
                );
            }
        }
    }

    /// Where the buttons named `label` are, from the top of the window down.
    fn buttons_named(
        tree: &egui::accesskit::TreeUpdate,
        label: &str,
    ) -> Vec<(egui::accesskit::NodeId, egui::Rect)> {
        let mut buttons: Vec<_> = tree
            .nodes
            .iter()
            .filter(|(_, node)| {
                node.label() == Some(label) && node.role() == egui::accesskit::Role::Button
            })
            .filter_map(|(id, node)| {
                let rect = node.bounds()?;
                Some((
                    *id,
                    egui::Rect::from_min_max(
                        egui::pos2(rect.x0 as f32, rect.y0 as f32),
                        egui::pos2(rect.x1 as f32, rect.y1 as f32),
                    ),
                ))
            })
            .collect();
        buttons.sort_by(|(_, a), (_, b)| a.top().total_cmp(&b.top()));
        buttons
    }

    #[test]
    fn the_lost_strip_keeps_its_buttons_inside_the_window() {
        // What a server that is down answers a reconnect with: more than
        // the strip has room for on one line.
        let refused = "connection to server at \"db.internal.example.com\" (10.20.30.40), \
                       port 5432 failed: Connection refused. Is the server running on that \
                       host and accepting TCP/IP connections?";
        for look in crate::theme::Look::ALL {
            for long in [false, true] {
                let mut harness = Harness::with_size(egui::vec2(720.0, 480.0));
                harness.set_look(look);
                let tab = harness.connect_fake();
                let session = harness.app.workspace(tab).unwrap().session;
                harness.app.apply(crate::model::Action::Backend(
                    crate::backend::Event::Disconnected {
                        session,
                        error: tabletist_db::Error::ConnectionLost("server went away".into()),
                    },
                ));
                let error = tabletist_db::Error::Connect(refused.into());
                if long {
                    harness.app.apply(crate::model::Action::Reconnect(tab));
                    let Command::Connect {
                        session, request, ..
                    } = *crate::testing::last_sent(&harness.app)
                    else {
                        panic!("expected Connect");
                    };
                    harness.app.apply(crate::model::Action::Backend(
                        crate::backend::Event::ConnectFailed {
                            session,
                            request,
                            error: error.clone(),
                        },
                    ));
                }
                let tree = harness.settle();
                let window = egui::Rect::from_min_size(egui::Pos2::ZERO, harness.size);
                // The connection bar has a Disconnect of its own: the
                // strip's is the lower one.
                let reconnect = buttons_named(&tree, "Reconnect");
                let disconnect = buttons_named(&tree, "Disconnect");
                assert_eq!((reconnect.len(), disconnect.len()), (1, 2), "{}", look.name);
                if long {
                    // The error is there in full, before the buttons.
                    let said = error.to_string();
                    let text = crate::testing::bounds(&tree, &said, egui::accesskit::Role::Label);
                    let text = text.expect("the exact error");
                    assert!(text.right() <= reconnect[0].1.left(), "{}", look.name);
                }
                for (name, (_, button)) in
                    [("Reconnect", reconnect[0]), ("Disconnect", disconnect[1])]
                {
                    assert!(
                        window.contains_rect(button),
                        "{name} at {button:?} in {} (long error: {long})",
                        look.name
                    );
                }
            }
        }
    }

    #[test]
    fn opening_an_object_puts_it_first_in_recent() {
        let mut harness = Harness::new();
        let tab = harness.connect_fake();
        harness.click("orders");
        harness.click("users");
        let recent: Vec<String> = harness
            .app
            .workspace(tab)
            .unwrap()
            .recent
            .iter()
            .map(|(object, _)| object.name.clone())
            .collect();
        assert_eq!(recent, ["users", "orders"]);
        harness.set_look(crate::theme::Look::macos());
        assert!(harness.has("Recent orders"), "macOS lists them");
    }

    #[test]
    fn following_a_foreign_key_opens_the_target_filtered_to_the_row() {
        let mut harness = Harness::new();
        let tab = with_page(&mut harness);
        harness.app.apply(crate::model::Action::FollowForeignKey {
            tab,
            object: tabletist_db::ObjectRef::new("main", "orders"),
            column: "id".into(),
            value: "3".into(),
        });
        let workspace = harness.app.workspace(tab).unwrap();
        let object = workspace.active_object_tab().unwrap();
        assert_eq!(object.object.name, "orders");
        assert_eq!(object.query.filters.len(), 1);
        assert_eq!(object.query.filters[0].column, "id");
        assert_eq!(object.query.filters[0].value, "3");
    }

    #[test]
    fn a_filter_chip_drops_its_filter_and_the_sort_chip_its_sort() {
        let mut harness = Harness::new();
        let tab = with_page(&mut harness);
        let id = harness.app.workspace(tab).unwrap().active_tab.unwrap();
        let object = harness
            .app
            .workspace_mut(tab)
            .unwrap()
            .object_tab_mut(id)
            .unwrap();
        object.filter.rows = vec![crate::model::FilterRow {
            column: "id".into(),
            op: tabletist_db::FilterOp::Eq,
            value: "5".into(),
        }];
        harness.app.apply(crate::model::Action::ApplyFilters {
            tab,
            object_tab: id,
        });
        harness.answer_rows(crate::testing::page(1, false));
        harness.app.apply(crate::model::Action::SortBy {
            tab,
            object_tab: id,
            column: "email".into(),
        });
        harness.answer_rows(crate::testing::page(1, false));
        harness.click("Remove filter id = 5");
        let object = harness
            .app
            .workspace(tab)
            .unwrap()
            .active_object_tab()
            .unwrap();
        assert!(object.query.filters.is_empty());
        harness.answer_rows(crate::testing::page(5, false));
        harness.click("Clear sort");
        let object = harness
            .app
            .workspace(tab)
            .unwrap()
            .active_object_tab()
            .unwrap();
        assert!(object.query.sort.is_empty());
    }

    #[test]
    fn the_grid_shows_timestamps_to_the_second_until_asked_for_more() {
        let mut harness = Harness::new();
        let tab = harness.connect_fake();
        harness.click("users");
        let mut page = crate::testing::page(1, false);
        page.columns[1].kind = tabletist_db::ValueKind::Temporal;
        page.rows[0][1] = tabletist_db::Value::Text("2026-01-12 09:14:03.482915".into());
        harness.answer_rows(page);
        harness.settle();
        assert!(harness.painted_color("2026-01-12 09:14:03").is_some());
        harness.click("Full precision");
        assert!(harness.app.workspace(tab).unwrap().full_precision);
        harness.settle();
        assert!(
            harness
                .painted_color("2026-01-12 09:14:03.482915")
                .is_some()
        );
    }

    #[test]
    fn a_workspace_starts_with_the_timestamps_the_settings_ask_for() {
        let mut harness = Harness::new();
        harness.app.settings.timestamps = crate::settings::Timestamps::Full;
        let tab = harness.connect_fake();
        assert!(harness.app.workspace(tab).unwrap().full_precision);
        harness.click("users");
        let mut page = crate::testing::page(1, false);
        page.columns[1].kind = tabletist_db::ValueKind::Temporal;
        page.rows[0][1] = tabletist_db::Value::Text("2026-01-12 09:14:03.482915".into());
        harness.answer_rows(page);
        harness.settle();
        assert!(
            harness
                .painted_color("2026-01-12 09:14:03.482915")
                .is_some()
        );
        // The grid's own link still switches this workspace.
        harness
            .app
            .apply(crate::model::Action::ToggleFullPrecision(tab));
        assert!(!harness.app.workspace(tab).unwrap().full_precision);
    }

    /// The `FetchRows` sent last: its offset and its limit.
    fn last_fetch(harness: &Harness) -> (u64, u32) {
        harness
            .app
            .backend
            .sent
            .iter()
            .rev()
            .find_map(|command| match command {
                Command::FetchRows { query, .. } => Some((query.offset, query.limit)),
                _ => None,
            })
            .expect("a FetchRows was sent")
    }

    #[test]
    fn a_new_page_size_fetches_an_open_page_again_from_where_it_is() {
        let mut harness = Harness::new();
        let tab = harness.connect_fake();
        harness.click("users");
        harness.answer_rows(crate::testing::page(3, true));
        let object_tab = harness.app.workspace(tab).unwrap().active_tab.unwrap();
        harness
            .app
            .apply(crate::model::Action::NextPage { tab, object_tab });
        assert_eq!(last_fetch(&harness), (300, 300));
        harness.answer_rows(crate::testing::page(3, true));
        let before = fetches(&harness);
        let settings = crate::settings::Settings {
            page_size: 500,
            ..harness.app.settings.clone()
        };
        harness.app.apply_settings(settings);
        // The same offset, the new size.
        assert_eq!(fetches(&harness), before + 1);
        assert_eq!(last_fetch(&harness), (300, 500));
        // The page of the old size is not left on screen meanwhile: were
        // this fetch to fail, Next would move past it by the new size.
        let object = harness
            .app
            .workspace(tab)
            .unwrap()
            .active_object_tab()
            .unwrap();
        assert!(object.page().is_none());
        harness.answer_rows(crate::testing::page(3, true));
        // Next moves by the size of the page that is shown: no row skipped.
        harness
            .app
            .apply(crate::model::Action::NextPage { tab, object_tab });
        assert_eq!(last_fetch(&harness), (800, 500));
    }

    #[test]
    fn a_page_still_on_its_way_is_fetched_again_at_the_new_size() {
        let mut harness = Harness::new();
        harness.connect_fake();
        harness.click("users");
        // Not answered: the first page is in flight at the old size.
        let before = fetches(&harness);
        let settings = crate::settings::Settings {
            page_size: 500,
            ..harness.app.settings.clone()
        };
        harness.app.apply_settings(settings);
        assert_eq!(fetches(&harness), before + 1);
        assert_eq!(last_fetch(&harness), (0, 500));
    }

    #[test]
    fn a_table_whose_session_is_down_waits_for_its_next_fetch() {
        let mut harness = Harness::new();
        let tab = harness.connect_fake();
        harness.click("users");
        harness.answer_rows(crate::testing::page(3, true));
        harness.app.workspace_mut(tab).unwrap().status =
            crate::model::SessionStatus::Disconnected(tabletist_db::Error::query("gone"));
        let before = fetches(&harness);
        let settings = crate::settings::Settings {
            page_size: 500,
            ..harness.app.settings.clone()
        };
        harness.app.apply_settings(settings);
        assert_eq!(
            fetches(&harness),
            before,
            "nothing is asked of a dead session"
        );
        // Its next fetch moves by the page it shows and asks for the new size.
        harness.app.workspace_mut(tab).unwrap().status = crate::model::SessionStatus::Connected;
        let object_tab = harness.app.workspace(tab).unwrap().active_tab.unwrap();
        harness
            .app
            .apply(crate::model::Action::NextPage { tab, object_tab });
        assert_eq!(last_fetch(&harness), (300, 500));
    }

    #[test]
    fn previous_moves_by_the_size_of_the_page_it_fetches() {
        let mut harness = Harness::new();
        let tab = harness.connect_fake();
        harness.click("users");
        harness.answer_rows(crate::testing::page(3, true));
        let object_tab = harness.app.workspace(tab).unwrap().active_tab.unwrap();
        for _ in 0..3 {
            harness
                .app
                .apply(crate::model::Action::NextPage { tab, object_tab });
            harness.answer_rows(crate::testing::page(3, true));
        }
        assert_eq!(last_fetch(&harness), (900, 300));
        // The size changes while the session is down: the page keeps its own.
        harness.app.workspace_mut(tab).unwrap().status =
            crate::model::SessionStatus::Disconnected(tabletist_db::Error::query("gone"));
        let settings = crate::settings::Settings {
            page_size: 100,
            ..harness.app.settings.clone()
        };
        harness.app.apply_settings(settings);
        harness.app.workspace_mut(tab).unwrap().status = crate::model::SessionStatus::Connected;
        harness
            .app
            .apply(crate::model::Action::PrevPage { tab, object_tab });
        // The page fetched has the new size and ends where the one that was
        // shown begins. Back by the old size, rows 700 to 899 are skipped.
        assert_eq!(last_fetch(&harness), (800, 100));
    }

    #[test]
    fn previous_near_the_start_fetches_only_the_rows_before_the_page() {
        let mut harness = Harness::new();
        let tab = harness.connect_fake();
        harness.click("users");
        harness.answer_rows(crate::testing::page(3, true));
        let object_tab = harness.app.workspace(tab).unwrap().active_tab.unwrap();
        harness
            .app
            .apply(crate::model::Action::NextPage { tab, object_tab });
        assert_eq!(last_fetch(&harness), (300, 300));
        harness.answer_rows(crate::testing::page(3, true));
        // The size grows: the page on screen is rows 300 to 799.
        let settings = crate::settings::Settings {
            page_size: 500,
            ..harness.app.settings.clone()
        };
        harness.app.apply_settings(settings);
        assert_eq!(last_fetch(&harness), (300, 500));
        harness.answer_rows(crate::testing::page(3, true));
        harness
            .app
            .apply(crate::model::Action::PrevPage { tab, object_tab });
        // Only 300 rows are before that page. A whole page from the start
        // would show rows 300 to 499 a second time.
        assert_eq!(last_fetch(&harness), (0, 300));
        harness.answer_rows(crate::testing::page(3, true));
        // Next moves by the short page, back to where the user was, and the
        // page fetched there has the settings' size again.
        harness
            .app
            .apply(crate::model::Action::NextPage { tab, object_tab });
        assert_eq!(last_fetch(&harness), (300, 500));
    }

    #[test]
    fn a_refresh_at_a_new_page_size_leaves_no_page_of_the_old_size_to_move_from() {
        let mut harness = Harness::new();
        let tab = harness.connect_fake();
        harness.click("users");
        harness.answer_rows(crate::testing::page(3, true));
        // The size changes while the session is down: the page keeps its own.
        harness.app.workspace_mut(tab).unwrap().status =
            crate::model::SessionStatus::Disconnected(tabletist_db::Error::query("gone"));
        let settings = crate::settings::Settings {
            page_size: 500,
            ..harness.app.settings.clone()
        };
        harness.app.apply_settings(settings);
        harness.app.workspace_mut(tab).unwrap().status = crate::model::SessionStatus::Connected;
        harness.app.apply(crate::model::Action::Refresh(tab));
        assert_eq!(last_fetch(&harness), (0, 500));
        // A refresh keeps the page it fetches again, but not one of another
        // size: a cancel would bring it back under a limit it was not
        // fetched with.
        let object = harness
            .app
            .workspace(tab)
            .unwrap()
            .active_object_tab()
            .unwrap();
        assert!(object.page().is_none());
        cancel_fetches(&mut harness, tab);
        // Nothing is on screen for Next to move past by the new size.
        let before = fetches(&harness);
        let object_tab = harness.app.workspace(tab).unwrap().active_tab.unwrap();
        harness
            .app
            .apply(crate::model::Action::NextPage { tab, object_tab });
        assert_eq!(fetches(&harness), before, "{:?}", last_fetch(&harness));
    }

    #[test]
    fn the_same_settings_fetch_nothing() {
        let mut harness = Harness::new();
        harness.connect_fake();
        harness.click("users");
        harness.answer_rows(crate::testing::page(3, true));
        let before = fetches(&harness);
        let settings = harness.app.settings.clone();
        harness.app.apply_settings(settings);
        assert_eq!(fetches(&harness), before);
    }

    #[test]
    fn an_option_that_is_not_the_page_size_fetches_no_page_again() {
        let mut harness = Harness::new();
        let tab = harness.connect_fake();
        harness.click("users");
        harness.answer_rows(crate::testing::page(3, true));
        // A page of the old size on a session that answers again: the size
        // changed while it was down, and nothing was fetched since.
        harness.app.workspace_mut(tab).unwrap().status =
            crate::model::SessionStatus::Disconnected(tabletist_db::Error::query("gone"));
        let settings = crate::settings::Settings {
            page_size: 500,
            ..harness.app.settings.clone()
        };
        harness.app.apply_settings(settings);
        harness.app.workspace_mut(tab).unwrap().status = crate::model::SessionStatus::Connected;
        let before = fetches(&harness);
        // A new page size would fetch this page again. Another option is no
        // reason to.
        let settings = crate::settings::Settings {
            group_digits: true,
            ..harness.app.settings.clone()
        };
        harness.app.apply_settings(settings);
        assert_eq!(fetches(&harness), before);
    }

    #[test]
    fn a_change_of_the_timestamps_option_reaches_every_open_workspace() {
        let mut harness = Harness::new();
        let tab = harness.connect_fake();
        assert!(!harness.app.workspace(tab).unwrap().full_precision);
        let settings = crate::settings::Settings {
            timestamps: crate::settings::Timestamps::Full,
            ..harness.app.settings.clone()
        };
        harness.app.apply_settings(settings);
        assert!(harness.app.workspace(tab).unwrap().full_precision);
        // Another option changing leaves a workspace's own choice alone.
        harness
            .app
            .apply(crate::model::Action::ToggleFullPrecision(tab));
        let settings = crate::settings::Settings {
            group_digits: true,
            ..harness.app.settings.clone()
        };
        harness.app.apply_settings(settings);
        assert!(!harness.app.workspace(tab).unwrap().full_precision);
    }

    #[test]
    fn a_grid_is_fitted_again_when_an_option_widens_its_cells() {
        let mut harness = Harness::new();
        harness.connect_fake();
        harness.click("users");
        let mut page = crate::testing::page(1, false);
        page.rows[0][0] = tabletist_db::Value::Int(i64::MAX);
        harness.answer_rows(page);
        harness.settle();
        assert!(harness.painted_color("9223372036854775807").is_some());
        let settings = crate::settings::Settings {
            group_digits: true,
            ..harness.app.settings.clone()
        };
        harness.app.apply_settings(settings);
        harness.settle();
        // Not described yet: `id` is not known to be a key, so it is grouped.
        // Six commas wider. With the widths of the plain number the cell
        // would be cut short and this text never painted whole.
        assert!(harness.painted_color("9,223,372,036,854,775,807").is_some());
    }

    #[test]
    fn an_option_turned_back_on_fits_the_rows_then_on_screen() {
        let mut harness = Harness::new();
        let tab = harness.connect_fake();
        let group = |harness: &mut Harness, group_digits| {
            let settings = crate::settings::Settings {
                group_digits,
                ..harness.app.settings.clone()
            };
            harness.app.apply_settings(settings);
            harness.settle();
        };
        // Grouped, over a page of small numbers: the columns fit those.
        group(&mut harness, true);
        harness.click("users");
        harness.answer_rows(crate::testing::page(1, true));
        harness.settle();
        group(&mut harness, false);
        // The next page has a far longer one.
        let object_tab = harness.app.workspace(tab).unwrap().active_tab.unwrap();
        harness
            .app
            .apply(crate::model::Action::NextPage { tab, object_tab });
        let mut page = crate::testing::page(1, false);
        page.rows[0][0] = tabletist_db::Value::Int(i64::MAX);
        harness.answer_rows(page);
        harness.settle();
        // Grouped again. The widths the grouped grid had over the first page
        // would cut this number short.
        group(&mut harness, true);
        assert!(harness.painted_color("9,223,372,036,854,775,807").is_some());
    }

    #[test]
    fn an_edit_of_the_file_changes_what_an_open_grid_shows() {
        let mut harness = Harness::new();
        harness.connect_fake();
        harness.click("users");
        let mut page = crate::testing::page(1, false);
        page.rows[0][0] = tabletist_db::Value::Int(1_234_567);
        harness.answer_rows(page);
        harness.settle();
        assert!(harness.painted_color("1234567").is_some());
        harness.app.apply(crate::model::Action::Backend(
            crate::backend::Event::SettingsFile {
                text: "[data]\ngroup_digits = true\n".into(),
                own: false,
            },
        ));
        harness.settle();
        assert!(harness.painted_color("1,234,567").is_some());
    }

    #[test]
    fn numbers_are_grouped_when_the_settings_say_so_but_keys_never_are() {
        let mut harness = Harness::new();
        harness.app.settings.group_digits = true;
        let tab = harness.connect_fake();
        harness.click("users");
        let mut page = crate::testing::page(1, false);
        page.rows[0][0] = tabletist_db::Value::Int(1_234_567);
        page.columns[1].name = "amount".into();
        page.columns[1].kind = tabletist_db::ValueKind::Numeric;
        page.rows[0][1] = tabletist_db::Value::Text("1240.50".into());
        harness.answer_rows(page);
        harness.settle();
        // Not described yet: no column is known to be a key.
        assert!(harness.painted_color("1,234,567").is_some());
        assert!(harness.painted_color("1,240.50").is_some());
        harness.answer_structure(tabletist_db::Structure {
            primary_key: vec!["id".into()],
            ..Default::default()
        });
        harness.settle();
        assert!(harness.painted_color("1234567").is_some());
        assert!(harness.painted_color("1,234,567").is_none());
        assert!(harness.painted_color("1,240.50").is_some());
        assert!(harness.painted_color("1240.50").is_none());
        let object_tab = harness.app.workspace(tab).unwrap().active_tab.unwrap();
        harness.app.apply(crate::model::Action::SelectCell {
            tab,
            id: object_tab,
            cell: crate::model::CellPos { row: 0, col: 1 },
        });
        harness.settle();
        // The row panel's field gives the value as it is, beside the cell
        // that groups it.
        assert!(harness.painted_color("1240.50").is_some());
        assert!(harness.painted_color("1,240.50").is_some());
        // So does a copy.
        harness.app.workspace_mut(tab).unwrap().pane = crate::model::Pane::Grid;
        harness.copy(false);
        assert_eq!(harness.copied.as_deref(), Some("1240.50"));
    }

    #[test]
    fn numbers_are_plain_when_the_settings_do_not_group() {
        let mut harness = Harness::new();
        harness.connect_fake();
        harness.click("users");
        let mut page = crate::testing::page(1, false);
        page.rows[0][0] = tabletist_db::Value::Int(1_234_567);
        harness.answer_rows(page);
        harness.settle();
        assert!(harness.painted_color("1234567").is_some());
    }

    /// The colours a table's grid paints `true` and a plain text cell in,
    /// with the row selected so the row panel shows the value too: every
    /// `true` the frame painted, then the plain cell.
    fn true_and_plain(value_tags: bool) -> (Vec<egui::Color32>, egui::Color32) {
        let mut harness = Harness::new();
        harness.app.settings.value_tags = value_tags;
        let tab = harness.connect_fake();
        harness.click("users");
        let mut page = crate::testing::page(1, false);
        page.columns[2].name = "active".into();
        page.columns[2].kind = tabletist_db::ValueKind::Bool;
        page.rows[0][2] = tabletist_db::Value::Bool(true);
        harness.answer_rows(page);
        let object_tab = harness.app.workspace(tab).unwrap().active_tab.unwrap();
        harness.app.apply(crate::model::Action::SelectCell {
            tab,
            id: object_tab,
            cell: crate::model::CellPos { row: 0, col: 1 },
        });
        harness.settle();
        let trues = harness
            .painted
            .iter()
            .filter(|(text, _)| text == "true")
            .map(|(_, color)| *color)
            .collect();
        let plain = harness
            .painted_color("user1@example.com")
            .expect("the email cell");
        (trues, plain)
    }

    #[test]
    fn value_tags_colour_a_boolean_until_the_settings_turn_them_off() {
        let (on, plain) = true_and_plain(true);
        assert!(!on.is_empty());
        assert!(
            on.iter().any(|color| *color != plain),
            "a tag has its colour"
        );
        let (off, plain) = true_and_plain(false);
        // The grid's cell and the row panel's field.
        assert!(off.len() >= 2, "{off:?}");
        assert!(off.iter().all(|color| *color == plain), "{off:?}");
    }

    /// A harness in the terminal look with the Settings screen open.
    fn settings_screen() -> Harness {
        let mut harness = Harness::new();
        harness.set_look(crate::theme::Look::omarchy());
        harness.press(egui::Key::Comma, egui::Modifiers::COMMAND);
        assert!(settings_open(&harness));
        harness
    }

    fn settings_open(harness: &Harness) -> bool {
        matches!(harness.app.dialog, Some(crate::model::Dialog::Settings(_)))
    }

    fn settings_cursor(harness: &Harness) -> usize {
        match &harness.app.dialog {
            Some(crate::model::Dialog::Settings(dialog)) => dialog.row,
            _ => panic!("the Settings screen is not open"),
        }
    }

    fn settings_saves(harness: &Harness) -> usize {
        harness
            .app
            .backend
            .sent
            .iter()
            .filter(|command| {
                matches!(
                    command,
                    Command::Save {
                        file: crate::backend::StateFile::Settings(_),
                        ..
                    }
                )
            })
            .count()
    }

    #[test]
    fn mod_comma_opens_the_settings_in_the_terminal_look_and_escape_closes_them() {
        let mut harness = settings_screen();
        assert!(harness.has("settings"));
        // The cursor starts on the first option.
        assert!(harness.painted_color("▌rows per page").is_some());
        // A second press leaves the screen as it is.
        harness.press(egui::Key::J, egui::Modifiers::NONE);
        harness.press(egui::Key::Comma, egui::Modifiers::COMMAND);
        assert_eq!(settings_cursor(&harness), 1);
        harness.press(egui::Key::Escape, egui::Modifiers::NONE);
        assert!(harness.app.dialog.is_none());
    }

    /// The two looks that draw the Settings window as a sheet.
    const SHEET_LOOKS: [fn() -> crate::theme::Look; 2] =
        [crate::theme::Look::standard, crate::theme::Look::macos];

    /// What the sheet's link to the file manager says on this system.
    fn reveal_link() -> &'static str {
        if cfg!(target_os = "macos") {
            "Reveal in Finder"
        } else if cfg!(windows) {
            "Show in Explorer"
        } else {
            "Show in folder"
        }
    }

    /// A harness in `look` with the Settings window open.
    fn settings_sheet(look: crate::theme::Look) -> Harness {
        let mut harness = Harness::new();
        harness.set_look(look);
        harness.press(egui::Key::Comma, egui::Modifiers::COMMAND);
        assert!(settings_open(&harness), "{}", look.name);
        harness
    }

    #[test]
    fn mod_comma_opens_the_settings_window_in_every_look_and_escape_closes_it() {
        for look in crate::theme::Look::ALL {
            let mut harness = Harness::new();
            harness.set_look(look);
            harness.press(egui::Key::Comma, egui::Modifiers::COMMAND);
            assert!(settings_open(&harness), "{}", look.name);
            // A second press leaves it as it is.
            harness.press(egui::Key::Comma, egui::Modifiers::COMMAND);
            assert!(settings_open(&harness), "{}", look.name);
            harness.press(egui::Key::Escape, egui::Modifiers::NONE);
            assert!(harness.app.dialog.is_none(), "{}", look.name);
        }
    }

    #[test]
    fn the_shortcuts_dialog_opens_the_settings_in_every_look() {
        for look in crate::theme::Look::ALL {
            let mut harness = Harness::new();
            harness.set_look(look);
            harness.frame(vec![egui::Event::Text("?".into())]);
            harness.click("Settings");
            assert!(settings_open(&harness), "{}", look.name);
        }
    }

    #[test]
    fn the_settings_sheet_shows_the_general_tab() {
        for look in SHEET_LOOKS {
            let mut harness = settings_sheet(look());
            for text in [
                "Settings",
                "General",
                "How Tabletist shows data. Changes apply right away.",
                "Rows per page",
                "Table view; the SQL editor has its own limit",
                "Timestamps",
                "Numbers",
                "Grouping is display only; copy gives the raw value",
                "Value tags",
            ] {
                assert!(harness.has(text), "{text}");
            }
            // The sample is of the precision that is set.
            assert!(harness.painted_color("2026-01-12 09:14:03").is_some());
        }
    }

    #[test]
    fn each_control_of_the_settings_sheet_changes_its_setting_and_saves() {
        for look in SHEET_LOOKS {
            let mut harness = settings_sheet(look());
            let before = settings_saves(&harness);
            // A segment is named by its option and its own word: the word
            // alone would not say what it sets.
            harness.click("Timestamps: Full precision");
            assert_eq!(
                harness.app.settings.timestamps,
                crate::settings::Timestamps::Full
            );
            harness.settle();
            assert!(
                harness
                    .painted_color("2026-01-12 09:14:03.482915")
                    .is_some()
            );
            // It is written as before: the word alone.
            assert!(harness.painted_color("Full precision").is_some());
            harness.click("Numbers: 1,240.50");
            assert!(harness.app.settings.group_digits);
            harness.click("Numbers: 1240.50");
            assert!(!harness.app.settings.group_digits);
            let tags = harness.app.settings.value_tags;
            harness.click("Value tags");
            assert_eq!(harness.app.settings.value_tags, !tags);
            // The menu: opened, and a size picked from it.
            harness.click("Rows per page");
            harness.click("Rows per page 500");
            assert_eq!(harness.app.settings.page_size, 500);
            assert_eq!(settings_saves(&harness), before + 5);
            assert!(settings_open(&harness), "the window stays open");
        }
    }

    #[test]
    fn a_page_size_from_the_file_that_is_not_in_the_list_is_in_the_menu() {
        let mut harness = settings_sheet(crate::theme::Look::standard());
        harness.app.apply(crate::model::Action::SetOption(
            crate::settings::OptionValue::PageSize(250),
        ));
        harness.click("Rows per page");
        for size in ["100", "250", "300", "500", "1,000", "5,000"] {
            let entry = format!("Rows per page {size}");
            assert!(harness.has(&entry), "{entry}");
        }
    }

    #[test]
    fn the_letters_of_the_terminal_screen_do_nothing_in_the_sheet() {
        let mut harness = settings_sheet(crate::theme::Look::macos());
        let before = harness.app.settings.clone();
        for key in [egui::Key::J, egui::Key::L, egui::Key::H, egui::Key::Space] {
            harness.press(key, egui::Modifiers::NONE);
        }
        harness.press(egui::Key::R, egui::Modifiers::SHIFT);
        harness.press(egui::Key::E, egui::Modifiers::CTRL);
        assert_eq!(harness.app.settings, before);
        // Nothing may have been sent at all: `last_sent` would panic.
        let sent = &harness.app.backend.sent;
        assert!(
            !sent
                .iter()
                .any(|command| matches!(command, Command::EditSettingsFile { .. }))
        );
    }

    #[test]
    fn the_settings_sheet_says_where_the_file_is_and_what_can_be_done_with_it() {
        for look in SHEET_LOOKS {
            let mut harness = settings_sheet(look());
            assert!(harness.has("Stored in"));
            harness.click(reveal_link());
            match crate::testing::last_sent(&harness.app) {
                Command::RevealSettingsFile { path, text } => {
                    assert_eq!(path, &harness.app.dirs.settings_file());
                    assert_eq!(text, &harness.app.settings_file.text);
                }
                other => panic!("{other:?}"),
            }
            harness.click("Export…");
            assert_eq!(
                harness
                    .app
                    .backend
                    .saves
                    .last()
                    .map(|(_, name, _)| name.as_str()),
                Some("tabletist-settings.toml")
            );
            assert!(settings_open(&harness), "the window stays open");
        }
    }

    #[test]
    fn reset_to_defaults_asks_in_place_of_the_footers_links() {
        for look in SHEET_LOOKS {
            let mut harness = settings_sheet(look());
            harness.click("Timestamps: Full precision");
            harness.click("Reset to defaults");
            assert!(harness.has("Reset every option on this tab?"));
            assert!(!harness.has("Export…"), "the links give way");
            harness.click("Cancel");
            assert!(harness.has("Export…"));
            assert_eq!(
                harness.app.settings.timestamps,
                crate::settings::Timestamps::Full
            );
            harness.click("Reset to defaults");
            harness.click("Reset");
            assert_eq!(
                harness.app.settings.timestamps,
                crate::settings::Timestamps::Second
            );
            assert!(harness.has("Export…"));
        }
    }

    #[test]
    fn the_settings_sheet_counts_the_lines_the_app_ignores() {
        let mut harness = settings_sheet(crate::theme::Look::standard());
        let ignored = "1 line in the file could not be read and was ignored";
        assert!(!harness.has(ignored));
        harness.app.apply(crate::model::Action::Backend(
            crate::backend::Event::SettingsFile {
                text: "[data]\ngroup_digits = \"yes\"\npage_size = 500\n".into(),
                own: false,
            },
        ));
        assert!(harness.has(ignored));
        assert_eq!(harness.app.settings.page_size, 500);
    }

    #[test]
    fn escape_answers_the_reset_question_before_it_closes_the_window() {
        let mut harness = settings_sheet(crate::theme::Look::macos());
        harness.click("Reset to defaults");
        harness.press(egui::Key::Escape, egui::Modifiers::NONE);
        assert!(settings_open(&harness));
        assert!(harness.has("Export…"));
        harness.press(egui::Key::Escape, egui::Modifiers::NONE);
        assert!(harness.app.dialog.is_none());
    }

    #[test]
    fn the_keyboard_that_asks_for_a_reset_is_on_cancel_and_then_back_on_the_link() {
        use crate::settings::Timestamps;
        let mut harness = settings_sheet(crate::theme::Look::standard());
        harness.click("Timestamps: Full precision");
        tab_to_settings(&mut harness, "Reset to defaults");
        harness.press(egui::Key::Enter, egui::Modifiers::NONE);
        assert!(harness.has("Reset every option on this tab?"));
        assert_eq!(focused_name(&harness.settle()), "Cancel");
        // A second Enter is the answer that changes nothing.
        harness.press(egui::Key::Enter, egui::Modifiers::NONE);
        assert_eq!(harness.app.settings.timestamps, Timestamps::Full);
        assert_eq!(focused_name(&harness.settle()), "Reset to defaults");
        // Reset is a Tab stop of its own, before Cancel.
        harness.press(egui::Key::Enter, egui::Modifiers::NONE);
        harness.press(egui::Key::Tab, egui::Modifiers::SHIFT);
        assert_eq!(focused_name(&harness.settle()), "Reset");
        // Space is held down on it: its first press resets.
        let held = |repeat| egui::Event::Key {
            key: egui::Key::Space,
            physical_key: None,
            pressed: true,
            repeat,
            modifiers: egui::Modifiers::NONE,
        };
        harness.frame(vec![held(false)]);
        assert_eq!(harness.app.settings.timestamps, Timestamps::Second);
        // Export stands where Reset stood, and is not taken for it: a key
        // pressed in the very next frame presses nothing.
        let (enter, none) = (egui::Key::Enter, egui::Modifiers::NONE);
        harness.frame(vec![crate::testing::key(enter, none)]);
        harness.frame(vec![crate::testing::release(enter, none)]);
        assert!(harness.app.backend.saves.is_empty());
        // The keyboard is back on the link that asked.
        assert_eq!(focused_name(&harness.settle()), "Reset to defaults");
        // What the key still held repeats presses nothing either: the
        // question is not asked again.
        for _ in 0..4 {
            harness.frame(vec![held(true)]);
        }
        assert!(!harness.has("Reset every option on this tab?"));
        harness.frame(vec![crate::testing::release(egui::Key::Space, none)]);
        // Escape answers no, and takes the keyboard from where it was: it
        // is put back on the link too.
        harness.press(egui::Key::Enter, egui::Modifiers::NONE);
        assert_eq!(focused_name(&harness.settle()), "Cancel");
        harness.press(egui::Key::Escape, egui::Modifiers::NONE);
        assert!(settings_open(&harness));
        assert!(!harness.has("Reset every option on this tab?"));
        assert_eq!(focused_name(&harness.settle()), "Reset to defaults");
    }

    #[test]
    fn the_settings_sheet_writes_the_files_path_from_the_home_directory() {
        let mut harness = settings_sheet(crate::theme::Look::standard());
        let file = harness.app.dirs.settings_file();
        assert!(harness.has(&file.display().to_string()));
        // Under the home directory the app found at its start, it is
        // written from `~`, with the system's separator.
        let home = harness.app.dirs.config.parent();
        harness.app.dirs.home = home.map(std::path::Path::to_path_buf);
        let shown = ["~", "config", "settings.toml"].join(std::path::MAIN_SEPARATOR_STR);
        assert!(harness.has(&shown));
    }

    #[test]
    fn a_window_too_short_for_the_settings_sheet_keeps_its_footer() {
        for look in SHEET_LOOKS {
            let size = egui::vec2(1280.0, 300.0);
            let mut harness = Harness::with_size(size);
            harness.set_look(look());
            harness.press(egui::Key::Comma, egui::Modifiers::COMMAND);
            harness.settle();
            // The rows gave way: the footer is whole, in the window, under
            // the first of them.
            let stored = harness.painted_rect("Stored in").expect("the footer");
            assert!(stored.bottom() < size.y, "{stored:?}");
            let row = harness.painted_rect("Rows per page").expect("a row");
            assert!(row.bottom() < stored.top(), "{row:?} {stored:?}");
            harness.click("Export…");
            assert_eq!(harness.app.backend.saves.len(), 1);
        }
    }

    #[test]
    fn tab_brings_a_control_the_rows_scrolled_away_into_view() {
        use egui::accesskit::Role;
        for look in SHEET_LOOKS {
            let size = egui::vec2(1280.0, 300.0);
            let mut harness = Harness::with_size(size);
            harness.set_look(look());
            harness.press(egui::Key::Comma, egui::Modifiers::COMMAND);
            // The last row's switch: the window has room for one row or
            // two, and it is under them.
            tab_to_settings(&mut harness, "Value tags");
            let tree = harness.settle();
            let switch =
                crate::testing::bounds(&tree, "Value tags", Role::CheckBox).expect("the switch");
            // The rows came with the keyboard: the switch is in the
            // window, between the rows' heading and the footer.
            let window = egui::Rect::from_min_size(egui::Pos2::ZERO, size);
            assert!(window.contains_rect(switch), "{switch:?}");
            let stored = harness.painted_rect("Stored in").expect("the footer");
            assert!(switch.bottom() < stored.top(), "{switch:?} {stored:?}");
            let lead = "How Tabletist shows data. Changes apply right away.";
            let lead = harness.painted_rect(lead).expect("the tab's lead");
            assert!(switch.top() > lead.bottom(), "{switch:?} {lead:?}");
            // And back up with it: the first row's menu.
            for _ in 0..3 {
                harness.press(egui::Key::Tab, egui::Modifiers::SHIFT);
            }
            let tree = harness.settle();
            assert_eq!(focused_name(&tree), "Rows per page");
            let menu =
                crate::testing::bounds(&tree, "Rows per page", Role::ComboBox).expect("the menu");
            assert!(menu.top() > lead.bottom(), "{menu:?} {lead:?}");
            assert!(menu.bottom() < stored.top(), "{menu:?} {stored:?}");
        }
    }

    #[test]
    fn a_click_outside_the_settings_sheet_closes_it() {
        use crate::settings::Timestamps;
        for look in SHEET_LOOKS {
            let mut harness = settings_sheet(look());
            // In the margin the sheet leaves of the window.
            let outside = egui::pos2(20.0, 20.0);
            // While the footer asks, the click answers that first: no.
            harness.click("Timestamps: Full precision");
            harness.click("Reset to defaults");
            click_at(&mut harness, outside);
            assert!(settings_open(&harness), "the question is answered first");
            assert!(harness.has("Export…"));
            assert_eq!(harness.app.settings.timestamps, Timestamps::Full);
            // With the menu open it closes the menu, as Escape does.
            harness.click("Rows per page");
            assert!(harness.has("Rows per page 500"));
            click_at(&mut harness, outside);
            assert!(settings_open(&harness), "the menu closes first");
            assert!(!harness.has("Rows per page 500"));
            // A click on the sheet is not one outside it.
            let stored = harness.painted_rect("Stored in").expect("the footer");
            click_at(&mut harness, stored.center());
            assert!(settings_open(&harness));
            click_at(&mut harness, outside);
            assert!(harness.app.dialog.is_none());
        }
    }

    #[test]
    fn escape_closes_the_page_size_menu_before_the_settings_sheet() {
        for look in SHEET_LOOKS {
            let mut harness = settings_sheet(look());
            harness.click("Rows per page");
            assert!(harness.has("Rows per page 500"));
            harness.press(egui::Key::Escape, egui::Modifiers::NONE);
            assert!(settings_open(&harness), "the menu closes first");
            assert!(!harness.has("Rows per page 500"));
            assert_eq!(harness.app.settings.page_size, 300);
            harness.press(egui::Key::Escape, egui::Modifiers::NONE);
            assert!(harness.app.dialog.is_none());
        }
    }

    #[test]
    fn a_key_held_down_on_a_link_of_the_settings_sheet_presses_it_once() {
        let mut harness = settings_sheet(crate::theme::Look::standard());
        let held = |repeat| egui::Event::Key {
            key: egui::Key::Enter,
            physical_key: None,
            pressed: true,
            repeat,
            modifiers: egui::Modifiers::NONE,
        };
        let revealed = |harness: &Harness| {
            let sent = harness.app.backend.sent.iter();
            sent.filter(|command| matches!(command, Command::RevealSettingsFile { .. }))
                .count()
        };
        tab_to_settings(&mut harness, reveal_link());
        harness.frame(vec![held(false)]);
        harness.settle();
        assert_eq!(revealed(&harness), 1);
        // Every repeat would start another file manager.
        for _ in 0..3 {
            harness.frame(vec![held(true)]);
        }
        harness.settle();
        assert_eq!(revealed(&harness), 1);
        harness.frame(vec![crate::testing::release(
            egui::Key::Enter,
            egui::Modifiers::NONE,
        )]);
        // And every repeat another dialog to save in.
        tab_to_settings(&mut harness, "Export…");
        harness.frame(vec![held(false)]);
        for _ in 0..3 {
            harness.frame(vec![held(true)]);
        }
        harness.settle();
        assert_eq!(harness.app.backend.saves.len(), 1);
    }

    #[test]
    fn the_settings_sheet_is_whole_at_once_in_a_window_grown_tall() {
        use egui::accesskit::Role;
        for look in SHEET_LOOKS {
            let mut harness = Harness::with_size(egui::vec2(1280.0, 240.0));
            harness.set_look(look());
            harness.press(egui::Key::Comma, egui::Modifiers::COMMAND);
            harness.settle();
            // The window is made tall: the rows are all there in the frames
            // it takes any layout to settle, not a little more of them in
            // each frame after.
            harness.size = egui::vec2(1280.0, 800.0);
            let tree = harness.settle();
            let switch =
                crate::testing::bounds(&tree, "Value tags", Role::CheckBox).expect("the switch");
            let stored = harness.painted_rect("Stored in").expect("the footer");
            assert!(switch.bottom() < stored.top(), "{switch:?} {stored:?}");
        }
    }

    #[test]
    fn the_settings_cursor_moves_with_j_and_k_and_the_arrows() {
        let mut harness = settings_screen();
        assert_eq!(settings_cursor(&harness), 0);
        harness.press(egui::Key::J, egui::Modifiers::NONE);
        harness.press(egui::Key::ArrowDown, egui::Modifiers::NONE);
        assert_eq!(settings_cursor(&harness), 2);
        harness.press(egui::Key::K, egui::Modifiers::NONE);
        harness.press(egui::Key::ArrowUp, egui::Modifiers::NONE);
        harness.press(egui::Key::K, egui::Modifiers::NONE);
        assert_eq!(settings_cursor(&harness), 0);
        // The cursor's row says so.
        assert!(harness.painted_color("▌rows per page").is_some());
        assert!(harness.painted_color("timestamps").is_some());
    }

    #[test]
    fn h_and_l_change_the_cursors_option_and_save_it() {
        use crate::settings::Timestamps;
        let mut harness = settings_screen();
        // Rows per page.
        harness.press(egui::Key::L, egui::Modifiers::NONE);
        assert_eq!(harness.app.settings.page_size, 500);
        assert_eq!(settings_saves(&harness), 1);
        harness.press(egui::Key::ArrowLeft, egui::Modifiers::NONE);
        harness.press(egui::Key::H, egui::Modifiers::NONE);
        assert_eq!(harness.app.settings.page_size, 100);
        // At the end of the choices a step changes and writes nothing.
        let saved = settings_saves(&harness);
        harness.press(egui::Key::H, egui::Modifiers::NONE);
        assert_eq!(harness.app.settings.page_size, 100);
        assert_eq!(settings_saves(&harness), saved);
        // Timestamps.
        harness.press(egui::Key::J, egui::Modifiers::NONE);
        harness.press(egui::Key::ArrowRight, egui::Modifiers::NONE);
        assert_eq!(harness.app.settings.timestamps, Timestamps::Full);
        assert!(
            harness
                .painted_color("2026-01-12 09:14:03.482915")
                .is_some()
        );
        // Numbers: grouped is the left of the two.
        harness.press(egui::Key::J, egui::Modifiers::NONE);
        harness.press(egui::Key::H, egui::Modifiers::NONE);
        assert!(harness.app.settings.group_digits);
        assert!(harness.painted_color("grouping on").is_some());
    }

    #[test]
    fn space_flips_an_option_of_two_values_and_shift_r_resets_the_cursors() {
        let mut harness = settings_screen();
        // Space on the page size does nothing: it has more than two values.
        harness.press(egui::Key::Space, egui::Modifiers::NONE);
        assert_eq!(settings_saves(&harness), 0);
        for _ in 0..3 {
            harness.press(egui::Key::J, egui::Modifiers::NONE);
        }
        harness.press(egui::Key::Space, egui::Modifiers::NONE);
        assert!(!harness.app.settings.value_tags);
        assert!(harness.painted_color("[ ]").is_some());
        harness.press(egui::Key::R, egui::Modifiers::SHIFT);
        assert!(harness.app.settings.value_tags);
        assert!(harness.painted_color("[x]").is_some());
        // A plain r is not the key.
        harness.press(egui::Key::Space, egui::Modifiers::NONE);
        harness.press(egui::Key::R, egui::Modifiers::NONE);
        assert!(!harness.app.settings.value_tags);
    }

    #[test]
    fn keys_that_come_in_one_frame_act_in_their_order() {
        let key = |key| egui::Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        };
        // Down a row and a step, in one frame: the step is of the row the
        // cursor came to, not of the one it left.
        let mut harness = settings_screen();
        let before = harness.app.settings.clone();
        harness.frame(vec![key(egui::Key::J), key(egui::Key::L)]);
        assert_eq!(settings_cursor(&harness), 1);
        assert_eq!(harness.app.settings.page_size, before.page_size);
        assert_eq!(
            harness.app.settings.timestamps,
            crate::settings::Timestamps::Full
        );
        // Two steps in one frame are two steps: the second starts where
        // the first ended.
        let mut harness = settings_screen();
        let option = crate::settings::OptionId::PageSize;
        let once = harness.app.settings.stepped(option, true);
        let mut stepped = harness.app.settings.clone();
        once.set(&mut stepped);
        let twice = stepped.stepped(option, true);
        assert_ne!(once, twice);
        harness.frame(vec![key(egui::Key::L), key(egui::Key::L)]);
        assert_eq!(harness.app.settings.value(option), twice);
        // And a step, then a row down, then a step back: each on its row.
        let mut harness = settings_screen();
        harness.frame(vec![
            key(egui::Key::L),
            key(egui::Key::J),
            key(egui::Key::L),
            key(egui::Key::K),
        ]);
        assert_eq!(settings_cursor(&harness), 0);
        assert_eq!(harness.app.settings.value(option), once);
        assert_eq!(
            harness.app.settings.timestamps,
            crate::settings::Timestamps::Full
        );
    }

    #[test]
    fn a_click_moves_the_settings_cursor_and_a_click_on_a_value_sets_it() {
        use crate::settings::Timestamps;
        let mut harness = settings_screen();
        harness.click("Numbers");
        assert_eq!(settings_cursor(&harness), 2);
        harness.click("Timestamps: full");
        assert_eq!(harness.app.settings.timestamps, Timestamps::Full);
        assert_eq!(settings_cursor(&harness), 1, "the cursor follows the click");
        harness.click("Value tags: off");
        assert!(!harness.app.settings.value_tags);
        harness.click("Rows per page: more");
        assert_eq!(harness.app.settings.page_size, 500);
    }

    #[test]
    fn the_settings_screen_takes_the_keys_from_what_is_under_it() {
        let mut harness = Harness::new();
        harness.set_look(crate::theme::Look::omarchy());
        let tab = harness.connect_fake();
        let cursor = |harness: &Harness| harness.app.workspace(tab).unwrap().tree.cursor.clone();
        // j is the tree's key in the workspace: it moves the tree's cursor.
        harness.press(egui::Key::J, egui::Modifiers::NONE);
        let before = cursor(&harness);
        harness.press(egui::Key::J, egui::Modifiers::NONE);
        let under = cursor(&harness);
        assert_ne!(under, before);
        // Over the workspace it is the screen's, and the tree's cursor
        // stays where it was.
        harness.press(egui::Key::Comma, egui::Modifiers::COMMAND);
        harness.press(egui::Key::J, egui::Modifiers::NONE);
        assert_eq!(settings_cursor(&harness), 1);
        assert_eq!(cursor(&harness), under);
    }

    #[test]
    fn a_pointer_click_on_a_segment_sets_it() {
        let mut harness = settings_screen();
        let full = harness.painted_rect("full").expect("the segment");
        click_at(&mut harness, full.center());
        assert_eq!(
            harness.app.settings.timestamps,
            crate::settings::Timestamps::Full
        );
        assert_eq!(settings_cursor(&harness), 1, "the cursor follows the click");
        assert_eq!(settings_saves(&harness), 1);
    }

    #[test]
    fn the_set_segment_under_the_cursor_is_written_in_the_colour_for_the_accent() {
        use crate::theme::Palette;
        for palette in [Palette::dark(), Palette::light()] {
            let mut harness = settings_screen();
            harness.app.palette = palette;
            harness.press(egui::Key::J, egui::Modifiers::NONE);
            assert_eq!(
                harness.painted_color("second"),
                Some(palette.on_accent),
                "dark: {}",
                palette.dark
            );
        }
    }

    #[test]
    fn a_click_on_another_rows_page_size_only_moves_the_settings_cursor() {
        let mut harness = settings_screen();
        harness.press(egui::Key::J, egui::Modifiers::NONE);
        // No chevron is drawn on that row: the number starts where the
        // cursor's row has one.
        let number = harness.painted_rect("300").expect("the page size");
        click_at(&mut harness, number.left_center() + egui::vec2(1.0, 0.0));
        assert_eq!(settings_cursor(&harness), 0);
        assert_eq!(harness.app.settings.page_size, 300);
        assert_eq!(settings_saves(&harness), 0);
        // On the cursor's row the chevrons are drawn, and each takes a click.
        let stepper = harness.painted_rect("‹ 300 ›").expect("the chevrons");
        click_at(&mut harness, stepper.left_center() + egui::vec2(1.0, 0.0));
        assert_eq!(harness.app.settings.page_size, 100);
    }

    #[test]
    fn the_keys_in_the_settings_footer_are_buttons_too() {
        let mut harness = settings_screen();
        // The page size has nothing to flip.
        harness.click("Toggle");
        assert_eq!(settings_saves(&harness), 0);
        harness.click("Value tags");
        harness.click("Toggle");
        assert!(!harness.app.settings.value_tags);
        harness.click("Reset option");
        assert!(harness.app.settings.value_tags);
        harness.click("Close");
        assert!(harness.app.dialog.is_none());
    }

    #[test]
    fn the_settings_screen_shows_the_file_beside_the_options() {
        let mut harness = settings_screen();
        // The text the app holds, a line of the file per line.
        assert!(harness.painted_color("[data]").is_some());
        assert!(harness.painted_color("page_size    = ").is_some());
        // The path, however the home directory is written.
        assert!(
            harness
                .painted
                .iter()
                .any(|(text, _)| text.ends_with("settings.toml"))
        );
        // Under the home directory the app found at its start, it is
        // written from `~`, with the system's separator.
        let home = harness
            .app
            .dirs
            .config
            .parent()
            .map(|home| home.to_path_buf());
        harness.app.dirs.home = home;
        harness.settle();
        let shown = ["~", "config", "settings.toml"].join(std::path::MAIN_SEPARATOR_STR);
        assert!(harness.painted_color(&shown).is_some());
        // Not watched (a test never is): nothing claims it is live.
        assert!(harness.painted_color("live").is_none());
        harness.app.settings_file.live = true;
        harness.settle();
        assert!(harness.painted_color("live").is_some());
    }

    #[test]
    fn a_line_of_the_file_that_was_ignored_is_shown_in_the_colour_of_an_error() {
        let mut harness = settings_screen();
        harness.app.apply(crate::model::Action::Backend(
            crate::backend::Event::SettingsFile {
                text: "[data]\npage_size = 500\ngroup_digits = \"yes\"\n".into(),
                own: false,
            },
        ));
        harness.settle();
        let danger = harness.app.palette.danger;
        assert_eq!(
            harness.painted_color("group_digits = \"yes\""),
            Some(danger)
        );
        // A line that was read is not.
        assert_ne!(harness.painted_color("page_size = "), Some(danger));
        // And the screen shows what the file set.
        assert!(harness.painted_color("‹ 500 ›").is_some());
    }

    #[test]
    fn the_file_pane_marks_the_line_of_the_cursors_option() {
        let mut harness = settings_screen();
        let selection = harness.app.palette.selection;
        // The fill behind the line the text `key` starts.
        let marked = |harness: &Harness, key: &str| {
            let line = harness.painted_rect(key).expect("the key's line");
            harness.fills.iter().any(|(rect, color)| {
                *color == selection
                    && rect.y_range().contains(line.center().y)
                    && rect.x_range().contains(line.center().x)
            })
        };
        assert!(marked(&harness, "page_size    = "));
        assert!(!marked(&harness, "timestamps   = "));
        harness.press(egui::Key::J, egui::Modifiers::NONE);
        assert!(marked(&harness, "timestamps   = "));
        assert!(!marked(&harness, "page_size    = "));
    }

    #[test]
    fn the_file_panes_note_reads_as_one_sentence_without_its_marks() {
        let harness = settings_screen();
        // The marks say which word is coloured, and are not written.
        let note =
            "edits in the file reload live · invalid lines are shown here in red and ignored";
        assert_eq!(harness.painted_color(note), Some(harness.app.palette.dim));
    }

    /// The Settings screen over a settings file that goes on for `notes`
    /// lines of comments, with one line that is ignored among the first.
    fn settings_screen_over_a_long_file(notes: usize) -> Harness {
        let mut harness = settings_screen();
        let settings = harness.app.settings.to_toml();
        let settings = settings.replace("group_digits = false", "group_digits = \"yes\"");
        let notes: String = (1..=notes).map(|note| format!("# note {note}\n")).collect();
        harness.app.apply(crate::model::Action::Backend(
            crate::backend::Event::SettingsFile {
                text: format!("{settings}{notes}"),
                own: false,
            },
        ));
        harness.settle();
        harness
    }

    #[test]
    fn a_long_settings_file_is_marked_and_coloured_where_it_is_in_view() {
        let mut harness = settings_screen_over_a_long_file(900);
        let palette = harness.app.palette;
        let marked = |harness: &Harness, key: &str| {
            let line = harness.painted_rect(key).expect("the key's line");
            harness.fills.iter().any(|(rect, color)| {
                *color == palette.selection
                    && rect.y_range().contains(line.center().y)
                    && rect.x_range().contains(line.center().x)
            })
        };
        assert!(marked(&harness, "page_size    = "));
        assert!(!marked(&harness, "timestamps   = "));
        harness.press(egui::Key::J, egui::Modifiers::NONE);
        assert!(marked(&harness, "timestamps   = "));
        assert!(!marked(&harness, "page_size    = "));
        assert_eq!(
            harness.painted_color("group_digits = \"yes\""),
            Some(palette.danger)
        );
        // The lines under the settings are the file's too, as far as the
        // pane reaches.
        assert_eq!(harness.painted_color("# note 1"), Some(palette.dim));
        assert!(harness.painted_color("# note 900").is_none());
    }

    #[test]
    fn a_text_too_long_for_a_settings_file_is_shown_to_its_last_line() {
        let mut harness = settings_screen_over_a_long_file(5_000);
        let danger = harness.app.palette.danger;
        // It was not read: every line of it is one that was ignored.
        assert!(harness.app.settings_file.lines.is_empty());
        let first = "# written by tabletist, safe to edit by hand";
        assert_eq!(harness.painted_color(first), Some(danger));
        assert_eq!(harness.painted_color("[data]"), Some(danger));
        assert!(harness.painted_color("# note 5000").is_none());
        // The pane is as tall as all of its lines: the wheel reaches the
        // last, and the first is then out of view.
        let over = harness.painted_rect("[data]").expect("the file").center();
        harness.frame(vec![egui::Event::PointerMoved(over)]);
        for _ in 0..200 {
            if harness.painted_color("# note 5000").is_some() {
                break;
            }
            harness.frame(vec![egui::Event::MouseWheel {
                unit: egui::MouseWheelUnit::Page,
                delta: egui::vec2(0.0, -10.0),
                phase: egui::TouchPhase::Move,
                modifiers: Modifiers::NONE,
            }]);
        }
        harness.finish_animations();
        assert_eq!(harness.painted_color("# note 5000"), Some(danger));
        assert!(harness.painted_color(first).is_none());
        // It is in the pane, under the file's path and in the window: a
        // line painted where the pane does not show it is not shown.
        let last = harness.painted_rect("# note 5000").expect("the last line");
        let mut painted = harness.text_rects.iter();
        let (_, path) = painted
            .find(|(text, _)| text.ends_with("settings.toml"))
            .expect("the file's path");
        assert!(last.top() >= path.bottom(), "{last:?} under {path:?}");
        assert!(last.bottom() <= harness.size.y, "{last:?}");
    }

    #[test]
    fn a_screen_reader_is_told_the_lines_of_the_file_and_which_are_ignored() {
        let mut harness = settings_screen();
        harness.app.apply(crate::model::Action::Backend(
            crate::backend::Event::SettingsFile {
                text: "[data]\npage_size = 500\ngroup_digits = \"yes\"\n".into(),
                own: false,
            },
        ));
        // A line as it is written, whole, though it is painted in pieces.
        assert!(harness.has("[data]"));
        assert!(harness.has("page_size = 500"));
        // The one that was ignored says so: its colour tells no one who
        // does not see it.
        assert!(harness.has("Ignored: group_digits = \"yes\""));
        assert!(!harness.has("group_digits = \"yes\""));
        assert!(!harness.has("Ignored: page_size = 500"));
    }

    #[test]
    fn a_line_of_megabytes_is_drawn_no_further_than_the_pane_could_show() {
        let mut harness = settings_screen();
        // No settings file: one line, far past what one is read to.
        let line = "x".repeat(2_000_000);
        harness.app.apply(crate::model::Action::Backend(
            crate::backend::Event::SettingsFile {
                text: format!("{line}\n# after\n"),
                own: false,
            },
        ));
        harness.settle();
        // The text is kept as it is.
        assert_eq!(harness.app.settings_file.text.len(), line.len() + 9);
        let danger = harness.app.palette.danger;
        let longest = harness
            .text_rects
            .iter()
            .map(|(text, _)| text.len())
            .max()
            .expect("something is painted");
        assert!(longest <= 400, "{longest} bytes laid out for one line");
        assert!(harness.text_rects.iter().any(|(text, _)| text.len() == 400));
        // And the line after it is where the second line goes.
        assert_eq!(harness.painted_color("# after"), Some(danger));
    }

    #[test]
    fn a_narrow_window_has_the_options_and_not_the_file() {
        let mut harness = Harness::with_size(egui::vec2(1000.0, 700.0));
        harness.set_look(crate::theme::Look::omarchy());
        harness.press(egui::Key::Comma, egui::Modifiers::COMMAND);
        assert!(harness.painted_color("▌rows per page").is_some());
        assert!(harness.painted_color("[data]").is_none());
    }

    #[test]
    fn ctrl_e_opens_the_settings_file_in_the_editor() {
        let mut harness = settings_screen();
        // The footer's hint is its button too.
        assert!(harness.has("Open file in editor"));
        harness.press(egui::Key::E, egui::Modifiers::CTRL);
        assert!(matches!(
            crate::testing::last_sent(&harness.app),
            Command::EditSettingsFile { .. }
        ));
        // The screen stays: the editor is another window.
        assert!(settings_open(&harness));
    }

    #[test]
    fn ctrl_e_held_down_opens_the_editor_once() {
        let mut harness = settings_screen();
        let held = |repeat| egui::Event::Key {
            key: egui::Key::E,
            physical_key: None,
            pressed: true,
            repeat,
            modifiers: egui::Modifiers::CTRL,
        };
        harness.frame(vec![held(false)]);
        harness.frame(vec![held(true)]);
        harness.frame(vec![held(true)]);
        let opened = harness
            .app
            .backend
            .sent
            .iter()
            .filter(|command| matches!(command, Command::EditSettingsFile { .. }))
            .count();
        assert_eq!(opened, 1);
    }

    /// Where the Settings screen shows `message`: in a band of its own,
    /// under the header and over the rows. The bar under the screen paints
    /// the message as well, where nobody sees it.
    fn settings_notice(harness: &mut Harness, message: &str) -> Option<egui::Rect> {
        harness.settle();
        let title = harness.painted_rect("settings").expect("the title");
        let heading = harness.painted_rect("data").expect("the rows' heading");
        let mut painted = harness.text_rects.iter();
        painted
            .find(|(text, rect)| {
                text == message && rect.top() >= title.bottom() && rect.bottom() <= heading.top()
            })
            .map(|(_, rect)| *rect)
    }

    #[test]
    fn the_settings_screen_shows_a_notice_until_it_is_dismissed() {
        let mut harness = settings_screen();
        let message = "Could not save the password in the keyring.";
        harness.app.notice = Some(message.into());
        let shown = settings_notice(&mut harness, message).expect("the notice on the screen");
        // The file's pane starts under it, as the rows do.
        let file = harness.painted_rect("[data]").expect("the file");
        assert!(file.top() >= shown.bottom());
        // The covered bar has a button of this name too: the screen's is
        // the one on the message's line, where the pointer reaches it.
        let tree = harness.settle();
        let buttons = buttons_named(&tree, "Dismiss");
        let on_the_line =
            |(_, button): &(_, egui::Rect)| button.y_range().contains(shown.center().y);
        let (_, dismiss) = buttons
            .into_iter()
            .find(on_the_line)
            .expect("the screen's Dismiss");
        click_at(&mut harness, dismiss.center());
        assert!(harness.app.notice.is_none());
        assert!(settings_open(&harness), "the screen stays");
        assert!(settings_notice(&mut harness, message).is_none());
    }

    #[test]
    fn a_save_that_fails_while_the_settings_screen_is_open_is_shown_on_it() {
        let mut harness = settings_screen();
        harness.press(egui::Key::L, egui::Modifiers::NONE);
        assert_eq!(settings_saves(&harness), 1);
        let path = harness.app.dirs.settings_file();
        harness.app.apply(crate::model::Action::Backend(
            crate::backend::Event::Saved {
                path,
                result: Err("Permission denied".into()),
            },
        ));
        let notice = harness.app.notice.clone().expect("a notice");
        assert!(settings_notice(&mut harness, &notice).is_some(), "{notice}");
    }

    #[test]
    fn a_screen_reader_hears_which_value_of_an_option_is_set() {
        use egui::accesskit::{Role, Toggled};
        let mut harness = settings_screen();
        let state = |harness: &mut Harness, name: &str| {
            let tree = harness.settle();
            let id = crate::testing::node(&tree, name, Role::RadioButton)
                .unwrap_or_else(|| panic!("{name} is no radio button"));
            let (_, node) = tree.nodes.iter().find(|(node, _)| *node == id).unwrap();
            node.toggled()
        };
        let second = "Timestamps: second";
        let full = "Timestamps: full";
        assert_eq!(state(&mut harness, second), Some(Toggled::True));
        assert_eq!(state(&mut harness, full), Some(Toggled::False));
        harness.press(egui::Key::J, egui::Modifiers::NONE);
        harness.press(egui::Key::L, egui::Modifiers::NONE);
        assert_eq!(state(&mut harness, second), Some(Toggled::False));
        assert_eq!(state(&mut harness, full), Some(Toggled::True));
        // The check is one mark for two values: each says whether it is set.
        assert_eq!(state(&mut harness, "Value tags: on"), Some(Toggled::True));
        assert_eq!(state(&mut harness, "Value tags: off"), Some(Toggled::False));
        // The page size is a number between two steps: its row says it.
        let tree = harness.settle();
        let row = crate::testing::node(&tree, "Rows per page", Role::Button).expect("the row");
        let (_, row) = tree.nodes.iter().find(|(node, _)| *node == row).unwrap();
        assert_eq!(row.value(), Some("300"));
    }

    /// Tabs until the control named `name` has the keyboard.
    fn tab_to_settings(harness: &mut Harness, name: &str) {
        for _ in 0..40 {
            harness.press(egui::Key::Tab, egui::Modifiers::NONE);
            if focused_name(&harness.settle()) == name {
                return;
            }
        }
        panic!("{name} is no Tab stop of the Settings screen");
    }

    #[test]
    fn a_focused_button_of_the_settings_screen_keeps_space_and_the_arrows() {
        let mut harness = settings_screen();
        // The cursor on an option Space flips.
        for _ in 0..3 {
            harness.press(egui::Key::J, egui::Modifiers::NONE);
        }
        tab_to_settings(&mut harness, "Numbers");
        // Space presses the button, which is another row's: the cursor
        // goes there, and the option it was on is as it was.
        harness.press(egui::Key::Space, egui::Modifiers::NONE);
        assert_eq!(settings_cursor(&harness), 2);
        assert!(harness.app.settings.value_tags);
        // An arrow moves the keyboard from the button, and nothing else.
        // Where it takes the keyboard is egui's to say: each arrow starts
        // from a button again.
        harness.press(egui::Key::ArrowLeft, egui::Modifiers::NONE);
        assert!(!harness.app.settings.group_digits);
        assert_eq!(settings_saves(&harness), 0);
        tab_to_settings(&mut harness, "Numbers");
        harness.press(egui::Key::ArrowUp, egui::Modifiers::NONE);
        assert_eq!(settings_cursor(&harness), 2);
        // The letters are the screen's wherever the keyboard is.
        tab_to_settings(&mut harness, "Close");
        harness.press(egui::Key::K, egui::Modifiers::NONE);
        assert_eq!(settings_cursor(&harness), 1);
        harness.press(egui::Key::L, egui::Modifiers::NONE);
        assert_eq!(
            harness.app.settings.timestamps,
            crate::settings::Timestamps::Full
        );
        // And so is Escape.
        assert!(harness.ctx.memory(|memory| memory.focused().is_some()));
        harness.press(egui::Key::Escape, egui::Modifiers::NONE);
        assert!(harness.app.dialog.is_none());
    }

    #[test]
    fn space_on_the_settings_close_button_presses_it_and_flips_nothing() {
        let mut harness = settings_screen();
        for _ in 0..3 {
            harness.press(egui::Key::J, egui::Modifiers::NONE);
        }
        tab_to_settings(&mut harness, "Close");
        harness.press(egui::Key::Space, egui::Modifiers::NONE);
        assert!(harness.app.settings.value_tags);
        assert_eq!(settings_saves(&harness), 0);
        assert!(harness.app.dialog.is_none());
    }

    #[test]
    fn escape_closes_the_settings_screen_wherever_its_cursor_is() {
        let mut harness = settings_screen();
        // No key or click puts the cursor past the last row. If something
        // ever does, the screen must not be one that cannot be closed.
        if let Some(crate::model::Dialog::Settings(dialog)) = &mut harness.app.dialog {
            dialog.row = crate::settings::OptionId::ALL.len();
        }
        harness.press(egui::Key::Escape, egui::Modifiers::NONE);
        assert!(harness.app.dialog.is_none());
    }

    #[test]
    fn tab_stops_at_the_page_size_steps_only_where_they_are_drawn() {
        let stops = |harness: &mut Harness| {
            let mut names = std::collections::HashSet::new();
            for _ in 0..40 {
                harness.press(egui::Key::Tab, egui::Modifiers::NONE);
                names.insert(focused_name(&harness.settle()));
            }
            names
        };
        let steps = ["Rows per page: fewer", "Rows per page: more"];
        // On the cursor's row each step is its chevron.
        let mut harness = settings_screen();
        let reached = stops(&mut harness);
        for step in steps {
            assert!(reached.contains(step), "{step}: {reached:?}");
        }
        // On another row no chevron is drawn: Tab passes the steps by, and
        // a screen reader still has them.
        harness.press(egui::Key::J, egui::Modifiers::NONE);
        let reached = stops(&mut harness);
        assert!(reached.contains("Rows per page"), "{reached:?}");
        for step in steps {
            assert!(!reached.contains(step), "{step}: {reached:?}");
            assert!(harness.has(step), "{step}");
        }
    }

    #[test]
    fn the_timestamp_hint_gives_way_to_the_filter_chips() {
        use egui::accesskit::Role;
        for look in [crate::theme::Look::standard(), crate::theme::Look::macos()] {
            let mut harness = Harness::new();
            harness.set_look(look);
            let tab = harness.connect_fake();
            harness.click("users");
            let mut page = crate::testing::page(1, false);
            page.columns[1].kind = tabletist_db::ValueKind::Temporal;
            harness.answer_rows(page.clone());
            let id = harness.app.workspace(tab).unwrap().active_tab.unwrap();
            let object = harness
                .app
                .workspace_mut(tab)
                .unwrap()
                .object_tab_mut(id)
                .unwrap();
            object.filter.rows = vec![
                crate::model::FilterRow {
                    column: "id".into(),
                    op: tabletist_db::FilterOp::Gt,
                    value: "5".into(),
                },
                crate::model::FilterRow {
                    column: "email".into(),
                    op: tabletist_db::FilterOp::Contains,
                    value: "example".into(),
                },
            ];
            harness.app.apply(crate::model::Action::ApplyFilters {
                tab,
                object_tab: id,
            });
            harness.answer_rows(page);
            let sentence = "Timestamps shown to the second ·";
            // From a pane with no room for the hint to one with room for all of it.
            for width in (700..=1900).step_by(30) {
                harness.size.x = width as f32;
                let tree = harness.settle();
                let at = format!("in {} at {width} wide", look.name);
                // Each chip: its text and its button.
                let chips: Vec<(String, egui::Rect)> = tree
                    .nodes
                    .iter()
                    .filter(|(_, node)| node.role() == Role::Button)
                    .filter_map(|(_, node)| node.label()?.strip_prefix("Remove filter "))
                    .map(|text| {
                        let label = format!("Remove filter {text}");
                        let button = crate::testing::bounds(&tree, &label, Role::Button)
                            .unwrap_or_else(|| panic!("{label} has no bounds {at}"));
                        let text_rect = harness
                            .painted_rect(text)
                            .unwrap_or_else(|| panic!("the chip {text:?} is not painted {at}"));
                        (text.to_owned(), button.union(text_rect))
                    })
                    .collect();
                assert_eq!(chips.len(), 2, "both filters have a chip {at}");
                let link = crate::testing::bounds(&tree, "Full precision", Role::Link);
                let said = harness.painted_rect(sentence);
                assert!(
                    said.is_none() || link.is_some(),
                    "the sentence goes before its link does {at}"
                );
                for (part, rect) in [("link", link), ("sentence", said)] {
                    let Some(rect) = rect else {
                        continue;
                    };
                    for (text, chip) in &chips {
                        assert!(
                            !rect.intersects(*chip),
                            "the hint's {part} at {rect:?} is on the chip {text:?} at {chip:?} {at}"
                        );
                    }
                }
                if width == 1900 {
                    assert!(
                        link.is_some() && said.is_some(),
                        "a wide pane shows the whole hint {at}"
                    );
                }
            }
        }
    }

    #[test]
    fn ctrl_b_hides_and_shows_the_sidebar() {
        let (mut harness, _tab) = tree_harness();
        assert!(harness.has("orders"));
        harness.press(Key::B, Modifiers::COMMAND);
        assert!(!harness.has("orders"));
        harness.press(Key::B, Modifiers::COMMAND);
        assert!(harness.has("orders"));
    }

    #[test]
    fn the_terminal_look_moves_through_the_grid_with_vim_keys() {
        let mut harness = Harness::new();
        harness.set_look(crate::theme::Look::omarchy());
        let tab = with_page(&mut harness);
        focus_grid(&mut harness, tab);
        harness.press(Key::J, Modifiers::NONE);
        harness.press(Key::J, Modifiers::NONE);
        harness.press(Key::L, Modifiers::NONE);
        assert_eq!(
            selection(&harness, tab),
            Some(crate::model::CellPos { row: 1, col: 1 })
        );
        harness.press(Key::S, Modifiers::NONE);
        let view = harness
            .app
            .workspace(tab)
            .unwrap()
            .active_object_tab()
            .unwrap()
            .view;
        assert_eq!(view, crate::model::ObjectView::Structure);
        harness.press(Key::D, Modifiers::NONE);
        let view = harness
            .app
            .workspace(tab)
            .unwrap()
            .active_object_tab()
            .unwrap()
            .view;
        assert_eq!(view, crate::model::ObjectView::Data);
    }

    #[test]
    fn letters_do_nothing_outside_the_terminal_look() {
        let mut harness = Harness::new();
        let tab = with_page(&mut harness);
        focus_grid(&mut harness, tab);
        harness.press(Key::J, Modifiers::NONE);
        assert_eq!(selection(&harness, tab), None);
    }

    #[test]
    fn the_picker_moves_with_the_arrows_and_connects_with_enter() {
        let mut harness = Harness::new();
        add_saved(&mut harness, "Alpha");
        add_saved(&mut harness, "Beta");
        harness.press(Key::ArrowDown, Modifiers::NONE);
        harness.press(Key::ArrowDown, Modifiers::NONE);
        harness.press(Key::Enter, Modifiers::NONE);
        let tab = harness.app.active_tab_id();
        assert_eq!(harness.app.workspace(tab).unwrap().name, "Beta");
    }

    #[test]
    fn the_terminal_picker_deletes_only_on_dd() {
        let mut harness = Harness::new();
        harness.set_look(crate::theme::Look::omarchy());
        add_saved(&mut harness, "Alpha");
        harness.press(Key::J, Modifiers::NONE);
        harness.press(Key::D, Modifiers::NONE);
        assert_eq!(harness.app.connections.connections.len(), 1, "one d waits");
        harness.press(Key::D, Modifiers::NONE);
        assert!(harness.app.connections.connections.is_empty());
    }

    #[test]
    fn connecting_notes_when_the_connection_was_last_used() {
        let mut harness = Harness::new();
        let tab = harness.connect_fake();
        let conn = harness.app.workspace(tab).unwrap().conn_id.clone();
        assert!(harness.app.connections.last_used(&conn).is_some());
    }

    #[test]
    fn ctrl_o_shows_the_connections() {
        let mut harness = Harness::new();
        // Already on the picker: nothing to open.
        harness.press(Key::O, Modifiers::COMMAND);
        assert_eq!(harness.app.tabs.len(), 1);
        let tab = harness.connect_fake();
        harness.press(Key::O, Modifiers::COMMAND);
        assert_eq!(harness.app.tabs.len(), 2);
        assert!(matches!(
            harness.app.active_tab().content,
            crate::model::ConnTabContent::Picker(_)
        ));
        // From the connection again, the same picker.
        harness
            .app
            .apply(crate::model::Action::ActivateConnTab(tab));
        harness.press(Key::O, Modifiers::COMMAND);
        assert_eq!(harness.app.tabs.len(), 2);
    }

    #[test]
    fn ctrl_shift_w_closes_the_connection_tab() {
        let mut harness = Harness::new();
        harness.connect_fake();
        harness.app.apply(crate::model::Action::ShowConnections);
        let active = harness.app.active_tab_id();
        harness.press(Key::W, Modifiers::COMMAND | Modifiers::SHIFT);
        assert_eq!(harness.app.tabs.len(), 1);
        assert!(harness.app.tabs.iter().all(|tab| tab.id != active));
    }

    #[test]
    fn plain_ctrl_w_does_not_close_the_connection_tab() {
        let mut harness = Harness::new();
        harness.connect_fake();
        harness.app.apply(crate::model::Action::ShowConnections);
        harness.press(Key::W, Modifiers::COMMAND);
        assert_eq!(harness.app.tabs.len(), 2);
    }

    #[test]
    fn number_shortcuts_switch_connections_and_ctrl_tab_every_tab() {
        let mut harness = Harness::new();
        let first = harness.connect_fake();
        let second = connect_another(&mut harness, "Second");
        harness.app.apply(crate::model::Action::ShowConnections);
        harness.press(Key::Num1, Modifiers::COMMAND);
        assert_eq!(harness.app.active_tab_id(), first);
        harness.press(Key::Num2, Modifiers::COMMAND);
        assert_eq!(harness.app.active_tab_id(), second);
        // The picker has no number, and is one of the tabs Ctrl+Tab visits.
        harness.press(Key::Num3, Modifiers::COMMAND);
        assert_eq!(harness.app.active_tab_id(), second);
        harness.press(Key::Tab, Modifiers::CTRL);
        assert_eq!(harness.app.active, 2);
        harness.press(Key::Tab, Modifiers::CTRL | Modifiers::SHIFT);
        assert_eq!(harness.app.active, 1);
    }

    #[test]
    fn first_launch_says_why_the_list_is_empty_and_offers_a_connection() {
        use egui::accesskit::{self, Role};
        for look in crate::theme::Look::ALL {
            let mut harness = Harness::new();
            harness.set_look(look);
            let title = look.label("No connections yet");
            assert!(harness.has(&title), "{}", look.name);
            // The header's button and the empty state's: the lower one is
            // the empty state's.
            let tree = harness.settle();
            let top = |node: &accesskit::Node| node.bounds().map_or(0.0, |rect| rect.y0);
            let mut buttons: Vec<_> = tree
                .nodes
                .iter()
                .filter(|(_, node)| {
                    node.label() == Some("New connection") && node.role() == Role::Button
                })
                .collect();
            assert_eq!(buttons.len(), 2, "{}", look.name);
            buttons.sort_by(|(_, a), (_, b)| top(a).total_cmp(&top(b)));
            let lower = buttons[1].0;
            harness.frame(vec![egui::Event::AccessKitActionRequest(
                accesskit::ActionRequest {
                    target_tree: accesskit::TreeId::ROOT,
                    target_node: lower,
                    action: accesskit::Action::Click,
                    data: None,
                },
            )]);
            harness.settle();
            assert!(harness.app.dialog.is_some(), "{}", look.name);
        }
    }

    #[test]
    fn the_window_lays_out_at_small_and_large_sizes() {
        for size in [egui::vec2(720.0, 480.0), egui::vec2(2560.0, 1440.0)] {
            let mut harness = Harness::with_size(size);
            let tree = harness.settle();
            assert!(
                crate::testing::node(&tree, "New connection", egui::accesskit::Role::Button)
                    .is_some()
            );
        }
    }

    #[test]
    fn ctrl_n_opens_the_connection_dialog_and_escape_closes_it() {
        let mut harness = Harness::new();
        harness.press(Key::N, Modifiers::COMMAND);
        assert!(harness.app.dialog.is_some());
        assert!(harness.has("Save & Connect"));
        harness.press(Key::Escape, Modifiers::NONE);
        assert!(harness.app.dialog.is_none());
    }

    #[test]
    fn dialogs_open_and_close_in_every_look() {
        for look in crate::theme::Look::ALL {
            let mut harness = Harness::new();
            harness.set_look(look);
            harness.press(Key::N, Modifiers::COMMAND);
            assert!(harness.has("Save & Connect"), "{}", look.name);
            harness.press(Key::Escape, Modifiers::NONE);
            assert!(harness.app.dialog.is_none(), "{}", look.name);
        }
    }

    /// Opens another connection beside the open ones, named `name`: the
    /// picker (Mod+O), then a connect in it. Returns its tab, which shows.
    fn connect_another(harness: &mut Harness, name: &str) -> crate::model::ConnTabId {
        harness.press(Key::O, Modifiers::COMMAND);
        let tab = harness.connect_fake();
        harness.app.workspace_mut(tab).unwrap().name = name.into();
        tab
    }

    fn add_saved(harness: &mut Harness, name: &str) -> crate::connections::ConnectionId {
        let saved = crate::connections::SavedConnection {
            id: crate::connections::ConnectionId::new(),
            name: name.into(),
            environment: crate::env::Environment::Production,
            read_only: None,
            password: crate::connections::PasswordMode::None,
            ssh_secret: crate::connections::PasswordMode::None,
            spec: tabletist_db::ConnectSpec::sqlite(format!("/tmp/{name}.db")),
        };
        let id = saved.id.clone();
        harness.app.connections.upsert(saved);
        id
    }

    #[test]
    fn the_picker_names_a_connections_environment() {
        let mut harness = Harness::new();
        let id = add_saved(&mut harness, "Shop");
        let mut saved = harness.app.connections.get(&id).unwrap().clone();
        saved.environment = crate::env::Environment::Local;
        harness.app.connections.upsert(saved);
        harness.settle();
        assert!(harness.painted_color("local").is_some());
        assert!(harness.painted_color("production").is_none());
    }

    #[test]
    fn the_picker_lists_saved_connections_and_filters_them() {
        let mut harness = Harness::new();
        add_saved(&mut harness, "Production");
        add_saved(&mut harness, "Staging");
        assert!(harness.has("Production"));
        assert!(harness.has("Staging"));
        let title = harness.app.look.label("No connections yet");
        assert!(!harness.has(&title));
        if let crate::model::ConnTabContent::Picker(picker) = &mut harness.app.tabs[0].content {
            picker.search = "stag".into();
        }
        assert!(!harness.has("Production"));
        assert!(harness.has("Staging"));
    }

    #[test]
    fn the_picker_search_row_spans_the_cards() {
        for look in crate::theme::Look::ALL {
            let mut harness = Harness::new();
            harness.set_look(look);
            add_saved(&mut harness, "Production");
            let tree = harness.settle();
            let card = crate::testing::bounds(&tree, "Production", egui::accesskit::Role::Button)
                .expect("a card");
            let button =
                crate::testing::bounds(&tree, "New connection", egui::accesskit::Role::Button)
                    .expect("New connection");
            let field = tree
                .nodes
                .iter()
                .find(|(_, node)| node.role() == egui::accesskit::Role::TextInput)
                .and_then(|(_, node)| node.bounds())
                .expect("the search field");
            // The header holds the search; the terminal look puts it on a
            // line of its own under the header.
            assert!(
                (field.x1 as f32) <= button.left() || field.y0 as f32 >= button.bottom(),
                "{}: the field runs into New connection",
                look.name
            );
            // The terminal's rows span the window; macOS cards line up
            // with the header's button, their rows inside a 1 pt border.
            assert!(
                look.terminal || (button.right() - 1.0 - card.right()).abs() < 1.0,
                "{}: button ends at {}, cards at {}",
                look.name,
                button.right(),
                card.right()
            );
        }
    }

    #[test]
    fn picker_rows_name_the_driver_and_where_it_points() {
        let mut harness = Harness::new();
        add_saved(&mut harness, "Production");
        assert!(harness.has("SQLite"));
        assert!(harness.has("Production.db"));
        assert!(harness.has("Last used never"));
    }

    #[test]
    fn a_long_database_name_stays_in_the_host_column_of_a_narrow_window() {
        for look in [crate::theme::Look::standard(), crate::theme::Look::macos()] {
            let mut harness = Harness::with_size(egui::vec2(1000.0, 650.0));
            harness.set_look(look);
            let (mut spec, _) = tabletist_db::ConnectSpec::from_url(
                "postgres://clerk@db.bookshop.example/bookshop_development",
            )
            .unwrap();
            spec.tls = tabletist_db::TlsMode::Require;
            harness
                .app
                .connections
                .upsert(crate::connections::SavedConnection {
                    id: crate::connections::ConnectionId::new(),
                    name: "Bookshop".into(),
                    environment: crate::env::Environment::Dev,
                    read_only: None,
                    password: crate::connections::PasswordMode::None,
                    ssh_secret: crate::connections::PasswordMode::None,
                    spec,
                });
            let tree = harness.settle();
            // The row may cut the name short, so find it by how it starts.
            let database = tree
                .nodes
                .iter()
                .find(|(_, node)| node.value().is_some_and(|text| text.starts_with("/book")))
                .and_then(|(_, node)| node.bounds())
                .expect("the database");
            let tls = crate::testing::bounds(&tree, "require", egui::accesskit::Role::Label)
                .expect("the TLS mode");
            assert!(
                (database.x1 as f32) < tls.left(),
                "{}: the database ends at {}, TLS starts at {}",
                look.name,
                database.x1,
                tls.left()
            );
        }
    }

    #[test]
    fn the_connect_button_on_a_row_connects_in_this_tab() {
        let mut harness = Harness::new();
        add_saved(&mut harness, "Production");
        harness.click("Connect to Production");
        let tab = harness.app.active_tab_id();
        assert_eq!(harness.app.workspace(tab).unwrap().name, "Production");
        assert!(
            harness.has("Production"),
            "the tab is now titled after the connection"
        );
    }

    #[test]
    fn a_connect_that_was_lost_offers_retry() {
        let mut harness = Harness::new();
        add_saved(&mut harness, "Production");
        harness.click("Connect to Production");
        let tab = harness.app.active_tab_id();
        let session = harness.app.workspace(tab).unwrap().session;
        harness.app.apply(crate::model::Action::Backend(
            crate::backend::Event::Disconnected {
                session,
                error: tabletist_db::Error::ConnectionLost("server went away".into()),
            },
        ));
        assert!(harness.has("Retry"));
        harness.click("Retry");
        assert!(matches!(
            harness.app.workspace(tab).unwrap().status,
            crate::model::SessionStatus::Connecting { .. }
        ));
    }

    #[test]
    fn cancelling_the_password_prompt_says_so_and_offers_retry() {
        let mut harness = Harness::new();
        let (spec, _) = tabletist_db::ConnectSpec::from_url("postgres://me@db.example.com/app")
            .expect("a valid URL");
        harness
            .app
            .connections
            .upsert(crate::connections::SavedConnection {
                id: crate::connections::ConnectionId::new(),
                name: "Production".into(),
                environment: crate::env::Environment::Production,
                read_only: None,
                password: crate::connections::PasswordMode::Ask,
                ssh_secret: crate::connections::PasswordMode::None,
                spec,
            });
        harness.click("Connect to Production");
        harness.click("Cancel");
        let cancelled = harness.app.look.label("Connection cancelled");
        assert!(harness.has(&cancelled));
        assert!(!harness.has("The server refused the login. Check the user and password."));
        assert!(harness.has("Retry"));
        // Nothing failed: there are no details to copy.
        assert!(!harness.has("Copy details"));
    }

    #[test]
    fn the_top_bar_disconnect_button_returns_to_the_picker() {
        let mut harness = Harness::new();
        add_saved(&mut harness, "Production");
        harness.click("Connect to Production");
        harness.click("Disconnect");
        assert!(matches!(
            harness.app.active_tab().content,
            crate::model::ConnTabContent::Picker(_)
        ));
    }

    #[test]
    fn a_connecting_tab_shows_its_steps_and_cancel_returns_to_the_picker() {
        for look in crate::theme::Look::ALL {
            let mut harness = Harness::new();
            harness.set_look(look);
            add_saved(&mut harness, "Production");
            harness.click("Connect to Production");
            let tab = harness.app.active_tab_id();
            // A SQLite file is opened; a server is connected to.
            let open = format!("{} Production.db", look.label("Open"));
            assert!(harness.has(&open), "{open} in {}", look.name);
            assert!(harness.has(&look.label("Load schema")), "{}", look.name);
            harness.click("Cancel connecting");
            assert!(harness.app.workspace(tab).is_none(), "{}", look.name);
        }
    }

    #[test]
    fn the_first_step_names_the_server_and_its_tunnel() {
        let mut harness = Harness::new();
        harness.set_look(crate::theme::Look::macos());
        add_saved_with_tunnel(&mut harness);
        harness.click("Connect to Prod");
        assert!(harness.has("Connect to db.example.com:5432 via bastion"));
    }

    #[test]
    fn a_tunnelled_connects_first_step_is_on_screen_in_full() {
        for look in crate::theme::Look::ALL {
            let mut harness = Harness::new();
            harness.set_look(look);
            add_saved_with_tunnel(&mut harness);
            harness.click("Connect to Prod");
            harness.settle();
            // What is painted, not what the step is named: none of it cut.
            let step = format!(
                "{} db.example.com:5432 {} bastion",
                look.label("Connect to"),
                look.label("via")
            );
            assert!(
                harness.painted.iter().any(|(text, _)| *text == step),
                "{}: {:?}",
                look.name,
                harness.painted
            );
        }
    }

    #[test]
    fn a_step_too_long_for_the_window_is_cut_short() {
        for look in crate::theme::Look::ALL {
            let mut harness = Harness::with_size(egui::vec2(420.0, 480.0));
            harness.set_look(look);
            let id = add_saved_with_tunnel(&mut harness);
            let mut saved = harness.app.connections.get(&id).unwrap().clone();
            saved.spec.host = "an-uncommonly-long-host-name.internal.example.com".into();
            harness.app.connections.upsert(saved);
            harness.click("Connect to Prod");
            harness.settle();
            let start = look.label("Connect to");
            let painted = harness
                .painted
                .iter()
                .find(|(text, _)| text.starts_with(&start))
                .unwrap_or_else(|| panic!("{}: {:?}", look.name, harness.painted));
            assert!(painted.0.ends_with('…'), "{}: {}", look.name, painted.0);
        }
    }

    #[test]
    fn escape_cancels_a_connect_and_does_nothing_once_the_tab_opened() {
        let mut harness = Harness::new();
        add_saved(&mut harness, "Production");
        harness.click("Connect to Production");
        let tab = harness.app.active_tab_id();
        harness.press(Key::Escape, Modifiers::NONE);
        assert!(harness.app.workspace(tab).is_none());

        let mut harness = Harness::new();
        let tab = harness.connect_fake();
        harness.press(Key::Escape, Modifiers::NONE);
        assert!(harness.app.workspace(tab).is_some());
    }

    #[test]
    fn a_switch_of_database_keeps_its_editors_from_escape_and_cancel() {
        for look in crate::theme::Look::ALL {
            let mut harness = Harness::new();
            harness.set_look(look);
            let tab = harness.connect_fake();
            let query = harness.add_sql_tab(tab);
            harness.app.apply(crate::model::Action::SwitchDatabase {
                tab,
                database: "other".into(),
            });
            // The tab shows the steps of its connect, and nothing that
            // would close the editor with it.
            let tree = harness.settle();
            let labels = crate::testing::labels(&tree);
            assert!(labels.contains(&look.label("Load schema")), "{}", look.name);
            assert!(
                !labels.iter().any(|label| label == "Cancel connecting"),
                "{}",
                look.name
            );
            harness.press(Key::Escape, Modifiers::NONE);
            let workspace = harness.app.workspace(tab);
            assert!(
                workspace.is_some_and(|workspace| workspace.sql_tab(query).is_some()),
                "{}",
                look.name
            );
        }
    }

    #[test]
    fn escape_closes_an_open_list_before_it_cancels_a_connect() {
        let mut harness = Harness::new();
        let tab = harness.connect_fake();
        with_database_picker(&mut harness, tab);
        harness.app.apply(crate::model::Action::SwitchDatabase {
            tab,
            database: "postgres".into(),
        });
        harness.click("Database");
        assert!(egui::Popup::is_any_open(&harness.ctx), "the list is open");
        harness.press(Key::Escape, Modifiers::NONE);
        assert!(!egui::Popup::is_any_open(&harness.ctx), "the list closed");
        assert!(harness.app.workspace(tab).is_some());
        // With no list open the key is the connect's again.
        harness.press(Key::Escape, Modifiers::NONE);
        assert!(harness.app.workspace(tab).is_none());
    }

    #[test]
    fn an_escape_held_to_close_a_dialog_does_not_cancel_the_connect_under_it() {
        let mut harness = Harness::new();
        add_saved(&mut harness, "Production");
        harness.click("Connect to Production");
        let tab = harness.app.active_tab_id();
        harness.frame(vec![egui::Event::Text("?".into())]);
        harness.settle();
        assert!(harness.app.dialog.is_some(), "the shortcuts are open");
        let escape = || crate::testing::key(Key::Escape, Modifiers::NONE);
        harness.frame(vec![escape()]);
        harness.settle();
        assert!(harness.app.dialog.is_none(), "the press closed them");
        // The key is still down: egui reads these as repeats.
        harness.frame(vec![escape()]);
        harness.frame(vec![escape()]);
        assert!(harness.app.workspace(tab).is_some());
        // Let go and pressed again, it cancels.
        harness.frame(vec![crate::testing::release(Key::Escape, Modifiers::NONE)]);
        harness.press(Key::Escape, Modifiers::NONE);
        assert!(harness.app.workspace(tab).is_none());
    }

    #[test]
    fn the_top_bar_connections_button_opens_the_picker_in_a_new_tab() {
        for look in crate::theme::Look::ALL {
            let mut harness = Harness::new();
            harness.set_look(look);
            let tab = harness.connect_fake();
            harness.click("Connections");
            assert_eq!(harness.app.tabs.len(), 2, "{}", look.name);
            assert!(
                matches!(
                    harness.app.active_tab().content,
                    crate::model::ConnTabContent::Picker(_)
                ),
                "{}",
                look.name
            );
            // The connection stays open in its own tab.
            assert!(harness.app.workspace(tab).is_some(), "{}", look.name);
        }
    }

    #[test]
    fn the_connections_button_keeps_clear_of_the_connection() {
        for look in crate::theme::Look::ALL {
            let mut harness = Harness::with_size(egui::vec2(720.0, 480.0));
            harness.set_look(look);
            let tab = harness.connect_fake();
            harness.app.workspace_mut(tab).unwrap().name = "Bar check".into();
            let tree = harness.settle();
            let button =
                crate::testing::bounds(&tree, "Connections", egui::accesskit::Role::Button)
                    .unwrap_or_else(|| panic!("Connections missing in {}", look.name));
            let name = crate::testing::bounds(&tree, "Bar check", egui::accesskit::Role::Label)
                .unwrap_or_else(|| panic!("the name is missing in {}", look.name));
            let disconnect =
                crate::testing::bounds(&tree, "Disconnect", egui::accesskit::Role::Button)
                    .unwrap_or_else(|| panic!("Disconnect missing in {}", look.name));
            let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, harness.size);
            assert!(screen.contains_rect(button), "{}", look.name);
            // It leads the bar: before the connection's name, off Disconnect.
            assert!(button.right() < name.left(), "{}", look.name);
            assert!(!button.intersects(disconnect), "{}", look.name);
        }
    }

    #[test]
    fn the_bar_has_a_chip_for_each_open_connection() {
        for look in crate::theme::Look::ALL {
            let mut harness = Harness::new();
            harness.set_look(look);
            let first = harness.connect_fake();
            harness.app.workspace_mut(first).unwrap().name = "First".into();
            let second = connect_another(&mut harness, "Second");
            let env = crate::env::Environment::Dev.label(crate::env::Platform::of(&look));
            // The second shows: its chip is the database switcher, and
            // reads its name.
            let tree = harness.settle();
            for (label, role) in [
                ("Database", egui::accesskit::Role::ComboBox),
                ("Second", egui::accesskit::Role::Label),
            ] {
                assert!(
                    crate::testing::node(&tree, label, role).is_some(),
                    "{label} missing in {}",
                    look.name
                );
            }
            // The first's chip switches to it, and back.
            harness.click(&format!("Switch to First · {env}"));
            assert_eq!(harness.app.active_tab_id(), first, "{}", look.name);
            harness.click(&format!("Switch to Second · {env}"));
            assert_eq!(harness.app.active_tab_id(), second, "{}", look.name);
        }
    }

    #[test]
    fn a_chip_says_how_its_session_stands_while_it_is_not_connected() {
        for look in crate::theme::Look::ALL {
            let mut harness = Harness::new();
            harness.set_look(look);
            let tab = harness.connect_fake();
            // Connected, it says where it points: the fixture's file.
            assert!(harness.has("fixture.db"), "{}", look.name);
            let session = harness.app.workspace(tab).unwrap().session;
            harness.app.apply(crate::model::Action::Backend(
                crate::backend::Event::Disconnected {
                    session,
                    error: tabletist_db::Error::ConnectionLost("server went away".into()),
                },
            ));
            assert!(harness.has(&look.label("Disconnected")), "{}", look.name);
            assert!(!harness.has("fixture.db"), "{}", look.name);
        }
    }

    #[test]
    fn many_chips_share_the_bar_and_the_one_showing_stays_whole() {
        for look in crate::theme::Look::ALL {
            let mut harness = Harness::with_size(egui::vec2(720.0, 480.0));
            harness.set_look(look);
            harness.connect_fake();
            for number in 2..=8 {
                connect_another(&mut harness, &format!("Connection number {number}"));
            }
            let tree = harness.settle();
            let bounds = |label: &str, role| {
                crate::testing::bounds(&tree, label, role)
                    .unwrap_or_else(|| panic!("{label} missing in {}", look.name))
            };
            let connections = bounds("Connections", egui::accesskit::Role::Button);
            let disconnect = bounds("Disconnect", egui::accesskit::Role::Button);
            // The last one shows: its chip is whole, between the buttons.
            let own = bounds("Database", egui::accesskit::Role::ComboBox);
            assert!(own.left() > connections.right(), "{}", look.name);
            assert!(own.right() < disconnect.left(), "{}", look.name);
            assert!(own.width() > 60.0, "{}: {own:?}", look.name);
            // No other chip reaches over either button.
            let others: Vec<_> = tree
                .nodes
                .iter()
                .filter(|(_, node)| {
                    node.label()
                        .is_some_and(|label| label.starts_with("Switch to"))
                })
                .filter_map(|(_, node)| node.bounds())
                .collect();
            assert!(!others.is_empty(), "{}", look.name);
            for chip in others {
                assert!(chip.x0 as f32 >= connections.right(), "{}", look.name);
                assert!(chip.x1 as f32 <= disconnect.left(), "{}", look.name);
            }
        }
    }

    #[test]
    fn the_picker_shows_an_open_connection_instead_of_connecting_again() {
        for look in crate::theme::Look::ALL {
            let mut harness = Harness::new();
            harness.set_look(look);
            assert!(!harness.has("open"), "{}", look.name);
            let tab = harness.connect_fake();
            harness.app.apply(crate::model::Action::ShowConnections);
            // The saved connection is marked, and its button shows it.
            assert!(harness.has("open"), "{}", look.name);
            assert!(!harness.has("Connect to Fixture"), "{}", look.name);
            let sent = harness.app.backend.sent.len();
            harness.click("Show Fixture");
            assert_eq!(harness.app.active_tab_id(), tab, "{}", look.name);
            assert_eq!(harness.app.backend.sent.len(), sent, "{}", look.name);
            assert_eq!(harness.app.tabs.len(), 2, "the picker stays for next time");
        }
    }

    #[test]
    fn enter_shows_an_open_connection_and_shift_enter_opens_it_again() {
        let mut harness = Harness::new();
        let tab = harness.connect_fake();
        let conn = harness.app.workspace(tab).unwrap().conn_id.clone();
        harness.app.apply(crate::model::Action::ShowConnections);
        let picker = harness.app.active_tab_id();
        harness.app.apply(crate::model::Action::SelectConnection {
            tab: picker,
            conn: Some(conn),
        });
        harness.press(Key::Enter, Modifiers::NONE);
        assert_eq!(harness.app.active_tab_id(), tab);
        assert_eq!(harness.app.open_connections().count(), 1);
        harness.app.apply(crate::model::Action::ShowConnections);
        harness.press(Key::Enter, Modifiers::SHIFT);
        assert_eq!(
            harness.app.open_connections().count(),
            2,
            "a second session of the same connection"
        );
        assert_eq!(harness.app.active_tab_id(), picker, "in the picker's tab");
    }

    #[test]
    fn the_picker_has_a_new_connection_button() {
        let mut harness = Harness::new();
        harness.click("New connection");
        assert!(harness.app.dialog.is_some());
    }

    use crate::backend::Command;

    fn fetches(harness: &Harness) -> usize {
        harness
            .app
            .backend
            .sent
            .iter()
            .filter(|c| matches!(c, Command::FetchRows { .. }))
            .count()
    }

    /// A saved PostgreSQL connection whose connect fails with `error`.
    fn fail_connect(harness: &mut Harness, error: tabletist_db::Error) -> crate::model::ConnTabId {
        let (spec, _) = tabletist_db::ConnectSpec::from_url("postgres://reader@db.example.com/app")
            .expect("a valid URL");
        harness
            .app
            .connections
            .upsert(crate::connections::SavedConnection {
                id: crate::connections::ConnectionId::new(),
                name: "Production".into(),
                environment: crate::env::Environment::Production,
                read_only: None,
                password: crate::connections::PasswordMode::None,
                ssh_secret: crate::connections::PasswordMode::None,
                spec,
            });
        harness.click("Connect to Production");
        let Command::Connect {
            session, request, ..
        } = *crate::testing::last_sent(&harness.app)
        else {
            panic!("expected Connect");
        };
        harness.app.apply(crate::model::Action::Backend(
            crate::backend::Event::ConnectFailed {
                session,
                request,
                error,
            },
        ));
        harness.app.active_tab_id()
    }

    #[test]
    fn an_unreachable_server_says_which_and_retry_connects_again() {
        let mut harness = Harness::new();
        harness.set_look(crate::theme::Look::macos());
        let refused = tabletist_db::Error::Connect("Connection refused (os error 111)".into());
        let tab = fail_connect(&mut harness, refused);
        assert!(harness.has("Can't reach db.example.com:5432"));
        assert!(harness.has(
            "Could not reach the server. Check the host and port, and that the server is running."
        ));
        // The exact error stays on screen.
        assert!(harness.has("could not connect: Connection refused (os error 111)"));
        harness.click("Retry");
        assert!(matches!(
            harness.app.workspace(tab).unwrap().status,
            crate::model::SessionStatus::Connecting { .. }
        ));
    }

    #[test]
    fn a_refused_login_names_the_user_and_offers_the_connection() {
        let mut harness = Harness::new();
        harness.set_look(crate::theme::Look::macos());
        let refused =
            tabletist_db::Error::Auth("password authentication failed for user \"reader\"".into());
        fail_connect(&mut harness, refused);
        assert!(harness.has("Password rejected for reader"));
        harness.click("Edit connection");
        assert!(matches!(
            harness.app.dialog,
            Some(crate::model::Dialog::Connection(_))
        ));
    }

    #[test]
    fn a_tls_failure_says_there_is_no_way_round_it() {
        let mut harness = Harness::new();
        harness.set_look(crate::theme::Look::macos());
        let failed = tabletist_db::Error::Tls("invalid peer certificate: NotValidForName".into());
        fail_connect(&mut harness, failed);
        assert!(harness.has("TLS or certificate problem"));
        // The plain sentence under it is `describe_error`'s.
        assert!(
            harness.has(
                "The secure connection failed. Try another TLS mode, or check the certificate."
            )
        );
        assert!(harness.has(
            "There is no \"connect anyway\". Change the TLS mode or the host in the connection."
        ));
    }

    #[test]
    fn a_failed_connect_offers_its_buttons_in_every_look() {
        use egui::accesskit::Role;
        for look in crate::theme::Look::ALL {
            let mut harness = Harness::new();
            harness.set_look(look);
            fail_connect(&mut harness, tabletist_db::Error::Timeout);
            let tree = harness.settle();
            for name in ["Retry", "Edit connection", "Copy details"] {
                assert!(
                    crate::testing::node(&tree, name, Role::Button).is_some(),
                    "{name} in {}",
                    look.name
                );
            }
        }
    }

    #[test]
    fn the_sidebar_lists_objects_and_a_click_opens_a_preview_tab() {
        let mut harness = Harness::new();
        harness.connect_fake();
        assert!(harness.has("Schema"));
        assert!(harness.has("active_users"));
        harness.click("users");
        assert_eq!(fetches(&harness), 1);
        assert!(harness.has("users tab"));
        assert!(harness.has("Close users"));
    }

    #[test]
    fn a_loaded_page_shows_headers_rows_and_the_range() {
        let mut harness = Harness::new();
        harness.connect_fake();
        harness.click("users");
        harness.answer_rows(crate::testing::page(5, false));
        assert!(harness.has("email"));
        assert!(harness.has("Row 5"));
        assert!(harness.has("Rows 1–5 of 5"));
    }

    #[test]
    fn the_footer_says_it_waits_while_the_first_page_is_on_its_way() {
        // The terminal look has no footer.
        for look in crate::theme::Look::ALL
            .into_iter()
            .filter(|look| !look.terminal)
        {
            let mut harness = Harness::new();
            harness.set_look(look);
            harness.connect_fake();
            harness.click("users");
            assert!(harness.has("Waiting for server"), "{}", look.name);
            assert!(!harness.has("No rows"), "{}", look.name);
            harness.answer_rows(crate::testing::page(5, false));
            assert!(!harness.has("Waiting for server"), "{}", look.name);
            assert!(harness.has("Rows 1–5 of 5"), "{}", look.name);
        }
    }

    #[test]
    fn header_clicks_sort_and_the_next_page_button_pages() {
        let mut harness = Harness::new();
        harness.connect_fake();
        harness.click("users");
        harness.answer_rows(crate::testing::page(300, true));
        harness.click("email");
        match crate::testing::last_sent(&harness.app) {
            Command::FetchRows { query, .. } => assert_eq!(query.sort[0].column, "email"),
            other => panic!("{other:?}"),
        }
        harness.answer_rows(crate::testing::page(300, true));
        harness.click("Next page");
        match crate::testing::last_sent(&harness.app) {
            Command::FetchRows { query, .. } => assert_eq!(query.offset, 300),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn a_failed_page_shows_the_error_and_retry_refetches() {
        let mut harness = Harness::new();
        harness.connect_fake();
        harness.click("users");
        let (session, request) = match crate::testing::last_sent(&harness.app) {
            Command::FetchRows {
                session, request, ..
            } => (*session, *request),
            other => panic!("{other:?}"),
        };
        harness
            .app
            .apply(crate::model::Action::Backend(crate::backend::Event::Rows {
                session,
                request,
                result: Err(tabletist_db::Error::query("no such column: nope")),
            }));
        assert!(harness.has("no such column: nope"));
        let before = fetches(&harness);
        harness.click("Retry");
        assert_eq!(fetches(&harness), before + 1);
    }

    #[test]
    fn a_very_long_error_is_cut_and_keeps_its_buttons_in_the_window() {
        use egui::accesskit::Role;
        // PostgreSQL repeats a literal it cannot read.
        let long = format!(
            "invalid input syntax for type integer: \"{}\"",
            "9".repeat(5_000)
        );
        let error = tabletist_db::Error::Query {
            code: Some("22P02".into()),
            message: long.clone(),
            detail: Some(long.clone()),
            hint: Some(long.clone()),
        };
        for look in crate::theme::Look::ALL {
            let mut harness = Harness::with_size(egui::vec2(720.0, 480.0));
            harness.set_look(look);
            harness.connect_fake();
            harness.click("users");
            let (session, request) = match crate::testing::last_sent(&harness.app) {
                Command::FetchRows {
                    session, request, ..
                } => (*session, *request),
                other => panic!("{other:?}"),
            };
            harness
                .app
                .apply(crate::model::Action::Backend(crate::backend::Event::Rows {
                    session,
                    request,
                    result: Err(error.clone()),
                }));
            let tree = harness.settle();
            let window = egui::Rect::from_min_size(egui::Pos2::ZERO, harness.size);
            // The footer, by the arrow it always has. The terminal look
            // has no footer.
            let footer = crate::testing::bounds(&tree, "Previous page", Role::Button);
            assert_eq!(footer.is_none(), look.terminal, "{}", look.name);
            for name in ["Retry", "Copy details"] {
                let button = crate::testing::bounds(&tree, name, Role::Button)
                    .unwrap_or_else(|| panic!("{name} is missing in {}", look.name));
                assert!(
                    window.contains_rect(button),
                    "{name} at {button:?} in {}",
                    look.name
                );
                // In the view's own room, not over what is under it.
                if let Some(footer) = footer {
                    assert!(
                        button.bottom() <= footer.top(),
                        "{name} at {button:?} over the footer at {footer:?} in {}",
                        look.name
                    );
                }
            }
            // What a frame lays out and names: no piece longer than a
            // message may be, with its label before it.
            let most = super::format::MESSAGE_MAX_CHARS + 64;
            let named = crate::testing::labels(&tree);
            let painted = harness.painted.iter().map(|(piece, _)| piece);
            for piece in named.iter().chain(painted) {
                let length = piece.chars().count();
                assert!(length <= most, "{length} characters in {}", look.name);
            }
            // The clipboard gets all of it.
            harness.click("Copy details");
            let copied = harness.copied.clone().expect("the details were copied");
            assert!(copied.starts_with(&long), "{}", look.name);
            assert!(copied.ends_with(&format!("Hint: {long}")), "{}", look.name);
        }
    }

    /// Makes the active object tab's fetch look a second old.
    fn age_fetch(harness: &mut Harness, tab: crate::model::ConnTabId) {
        let workspace = harness.app.workspace_mut(tab).unwrap();
        let id = workspace.active_tab.unwrap();
        let object = workspace.object_tab_mut(id).unwrap();
        let earlier = std::time::Instant::now().checked_sub(std::time::Duration::from_secs(1));
        object.rows.started = earlier;
        object.structure.started = earlier;
    }

    /// Makes the active object tab's fetch look only just sent, however
    /// long the test has taken: a start still to come has lasted no time.
    fn pin_fetch(harness: &mut Harness, tab: crate::model::ConnTabId) {
        let workspace = harness.app.workspace_mut(tab).unwrap();
        let id = workspace.active_tab.unwrap();
        let object = workspace.object_tab_mut(id).unwrap();
        let later = std::time::Instant::now() + std::time::Duration::from_secs(3600);
        object.rows.started = Some(later);
        object.structure.started = Some(later);
    }

    #[test]
    fn a_query_that_lasts_says_so_and_can_be_cancelled() {
        for look in crate::theme::Look::ALL {
            let mut harness = Harness::new();
            harness.set_look(look);
            let tab = harness.connect_fake();
            harness.click("users");
            // A query that has only just been sent shows nothing of a wait.
            // The clock is held: a slow run must not age the fetch.
            pin_fetch(&mut harness, tab);
            assert!(!harness.has(&look.label("Running query…")), "{}", look.name);
            age_fetch(&mut harness, tab);
            assert!(harness.has(&look.label("Running query…")), "{}", look.name);
            // The time the box shows ticks on the frames its spinner asks
            // for: the next one at once, with no event to bring it.
            assert_eq!(
                harness.repaint_after,
                std::time::Duration::ZERO,
                "{}",
                look.name
            );
            harness.click("Cancel query");
            assert!(
                matches!(
                    crate::testing::last_sent(&harness.app),
                    Command::Cancel { .. }
                ),
                "{}",
                look.name
            );
        }
    }

    #[test]
    fn a_structure_that_takes_long_says_so_over_the_one_it_has() {
        for look in crate::theme::Look::ALL {
            let mut harness = Harness::new();
            harness.set_look(look);
            let tab = with_page(&mut harness);
            let loading = look.label("Loading structure…");
            harness.click("Structure");
            // The first fetch, with nothing to show yet.
            age_fetch(&mut harness, tab);
            assert!(harness.has(&loading), "{}", look.name);
            harness.answer_structure(tabletist_db::Structure {
                indexes: vec![tabletist_db::IndexInfo {
                    name: "users_email_idx".into(),
                    columns: vec!["email".into()],
                    key_columns: Some(vec!["email".into()]),
                    unique: true,
                    primary: false,
                    method: None,
                    partial: false,
                }],
                ..Default::default()
            });
            assert!(!harness.has(&loading), "{}", look.name);
            // A refresh keeps the structure under its wait, and can be
            // cancelled as the first fetch can.
            harness.app.apply(crate::model::Action::Refresh(tab));
            age_fetch(&mut harness, tab);
            assert!(harness.has(&loading), "{}", look.name);
            assert!(harness.has("users_email_idx"), "{}", look.name);
            harness.click("Cancel query");
            assert!(
                matches!(
                    crate::testing::last_sent(&harness.app),
                    Command::Cancel { .. }
                ),
                "{}",
                look.name
            );
        }
    }

    #[test]
    fn a_refresh_keeps_the_page_under_the_running_card() {
        let mut harness = Harness::new();
        let tab = with_page(&mut harness);
        // Opening from the sidebar leaves the keys with the tree, where
        // Mod+R refreshes the tree: give them to the grid.
        focus_grid(&mut harness, tab);
        harness.press(Key::R, Modifiers::COMMAND);
        age_fetch(&mut harness, tab);
        let running = harness.app.look.label("Running query…");
        assert!(harness.has(&running));
        assert!(harness.has("Row 1"));
    }

    #[test]
    fn the_running_card_takes_the_clicks_the_page_under_it_would_get() {
        let mut harness = Harness::new();
        let tab = harness.connect_fake();
        harness.click("users");
        // Enough rows that the middle of the area is over one.
        harness.answer_rows(crate::testing::page(60, false));
        focus_grid(&mut harness, tab);
        harness.press(Key::R, Modifiers::COMMAND);
        age_fetch(&mut harness, tab);
        let tree = harness.settle();
        let text = crate::testing::bounds(
            &tree,
            &harness.app.look.label("Running query…"),
            egui::accesskit::Role::Label,
        )
        .expect("the running text");
        let under = crate::testing::bounds(&tree, "Row 1", egui::accesskit::Role::Button)
            .expect("the first row");
        // The text is over the grid's rows: a click there would select.
        assert!(text.center().y > under.top(), "{text:?} {under:?}");
        assert_eq!(selection(&harness, tab), None);
        click_at(&mut harness, text.center());
        assert_eq!(selection(&harness, tab), None);
    }

    #[test]
    fn the_object_tab_close_button_closes_it() {
        let mut harness = Harness::new();
        let tab = harness.connect_fake();
        harness.click("users");
        harness.click("Close users");
        assert!(harness.app.workspace(tab).unwrap().tabs.is_empty());
        assert!(harness.has("Select a table or view in the sidebar"));
    }

    #[test]
    fn the_strip_shows_sql_editors_beside_object_tabs() {
        let mut harness = Harness::new();
        let tab = harness.connect_fake();
        harness.click("users");
        let users = harness.app.workspace(tab).unwrap().active_tab;
        let query = harness.add_sql_tab(tab);
        assert!(harness.has("users tab") && harness.has("Query 1 tab"));
        harness.click("Query 1 tab");
        assert_eq!(harness.app.workspace(tab).unwrap().active_tab, Some(query));
        harness.click("Close Query 1");
        let workspace = harness.app.workspace(tab).unwrap();
        assert!(workspace.sql_tab(query).is_none());
        assert_eq!(workspace.active_tab, users);
        assert!(!harness.has("Query 1 tab"));
    }

    #[test]
    fn the_terminal_digits_reach_every_tab_in_the_strip() {
        let mut harness = Harness::new();
        harness.set_look(crate::theme::Look::omarchy());
        let tab = with_page(&mut harness);
        let users = harness.app.workspace(tab).unwrap().active_tab;
        let query = harness.add_sql_tab(tab);
        harness.press(Key::Num2, Modifiers::NONE);
        assert_eq!(harness.app.workspace(tab).unwrap().active_tab, Some(query));
        // An editor shown for the first time takes the keys.
        harness.press(Key::Escape, Modifiers::NONE);
        harness.press(Key::Num1, Modifiers::NONE);
        assert_eq!(harness.app.workspace(tab).unwrap().active_tab, users);
    }

    #[test]
    fn the_terminal_letters_for_an_object_do_nothing_on_a_sql_editor() {
        let mut harness = Harness::new();
        harness.set_look(crate::theme::Look::omarchy());
        let tab = with_page(&mut harness);
        focus_grid(&mut harness, tab);
        let users = harness.app.workspace(tab).unwrap().active_tab.unwrap();
        let query = harness.add_sql_tab(tab);
        harness
            .app
            .apply(crate::model::Action::ActivateTab { tab, id: query });
        let panel = harness.app.workspace(tab).unwrap().row_panel;
        harness.settle();
        // A real key press sends the key and its text.
        harness.frame(vec![
            crate::testing::key(Key::Slash, Modifiers::NONE),
            egui::Event::Text("/".into()),
        ]);
        // Read before the next frame, which would take the flag.
        assert!(
            !harness.app.workspace(tab).unwrap().focus_where,
            "a SQL editor has no WHERE line to focus"
        );
        harness.press(Key::I, Modifiers::NONE);
        harness.press(Key::S, Modifiers::NONE);
        let workspace = harness.app.workspace(tab).unwrap();
        assert_eq!(workspace.row_panel, panel);
        assert_eq!(
            workspace.object_tab(users).unwrap().view,
            crate::model::ObjectView::Data
        );
        assert_eq!(workspace.active_tab, Some(query));
    }

    fn with_page(harness: &mut Harness) -> crate::model::ConnTabId {
        let tab = harness.connect_fake();
        harness.click("users");
        harness.answer_rows(crate::testing::page(5, false));
        tab
    }

    /// Gives the grid the arrow keys, as clicking into it does (opening a
    /// table from the sidebar leaves them with the tree).
    fn focus_grid(harness: &mut Harness, tab: crate::model::ConnTabId) {
        harness.app.workspace_mut(tab).unwrap().pane = crate::model::Pane::Grid;
    }

    /// Opens a SQL editor in `tab`, runs `SELECT 1` in it and answers with
    /// `rows` rows.
    fn with_sql_result(
        harness: &mut Harness,
        tab: crate::model::ConnTabId,
        rows: usize,
    ) -> crate::model::TabId {
        with_sql_outcome(harness, tab, crate::testing::rows_outcome(rows))
    }

    /// Opens a SQL editor in `tab`, runs `SELECT 1` in it and answers with
    /// `outcome`.
    fn with_sql_outcome(
        harness: &mut Harness,
        tab: crate::model::ConnTabId,
        outcome: tabletist_db::StatementOutcome,
    ) -> crate::model::TabId {
        harness.app.apply(crate::model::Action::NewSqlTab(tab));
        let id = harness.app.workspace(tab).unwrap().active_tab.unwrap();
        set_sql(harness, tab, "SELECT 1", 0);
        run_sql(harness, tab, id);
        harness.answer_sql(Ok(crate::testing::script_outcome(vec![outcome])), None);
        id
    }

    /// Starts a run of the SQL editor `id` and leaves it in flight.
    fn run_sql(harness: &mut Harness, tab: crate::model::ConnTabId, id: crate::model::TabId) {
        harness.app.apply(crate::model::Action::RunSql {
            tab,
            sql_tab: id,
            all: false,
        });
    }

    /// Puts `text` in the active SQL editor with the cursor at byte `cursor`.
    fn set_sql(harness: &mut Harness, tab: crate::model::ConnTabId, text: &str, cursor: usize) {
        let workspace = harness.app.workspace_mut(tab).unwrap();
        let id = workspace.active_tab.unwrap();
        let sql = workspace.sql_tab_mut(id).unwrap();
        sql.text = text.into();
        sql.cursor = cursor;
    }

    fn sql_selection(
        harness: &Harness,
        tab: crate::model::ConnTabId,
        id: crate::model::TabId,
    ) -> Option<crate::model::CellPos> {
        let workspace = harness.app.workspace(tab).unwrap();
        workspace.sql_tab(id).unwrap().selection
    }

    #[test]
    fn command_t_opens_a_sql_editor_and_command_o_a_connection_tab() {
        let mut harness = Harness::new();
        let tab = harness.connect_fake();
        harness.press(Key::T, Modifiers::COMMAND);
        let workspace = harness.app.workspace(tab).unwrap();
        assert!(workspace.active_sql_tab().is_some());
        assert!(harness.has("Query 1 tab"));
        let tabs = harness.app.tabs.len();
        harness.press(Key::O, Modifiers::COMMAND);
        assert_eq!(harness.app.tabs.len(), tabs + 1);
        // On the picker, Mod+T does nothing.
        harness.press(Key::T, Modifiers::COMMAND);
        assert_eq!(harness.app.tabs.len(), tabs + 1);
        let workspace = harness.app.workspace(tab).unwrap();
        assert_eq!(workspace.tabs.len(), 1, "nor in the workspace behind it");
    }

    #[test]
    fn command_t_opens_a_sql_editor_without_a_connection() {
        let mut harness = Harness::new();
        let tab = harness.connect_fake();
        let session = harness.app.workspace(tab).unwrap().session;
        harness.app.apply(crate::model::Action::Backend(
            crate::backend::Event::Disconnected {
                session,
                error: tabletist_db::Error::ConnectionLost("server went away".into()),
            },
        ));
        harness.press(Key::T, Modifiers::COMMAND);
        let workspace = harness.app.workspace(tab).unwrap();
        assert!(workspace.active_sql_tab().is_some());
    }

    #[test]
    fn command_return_runs_while_typing() {
        let mut harness = Harness::new();
        let tab = harness.connect_fake();
        harness.press(Key::T, Modifiers::COMMAND);
        type_text(&mut harness, "SELECT 1;\nSELECT 2");
        assert!(harness.ctx.text_edit_focused(), "the editor has the keys");
        harness.press(Key::Enter, Modifiers::COMMAND);
        assert!(matches!(
            harness.app.backend.sent.last(),
            Some(crate::backend::Command::RunSql { statements, .. })
                if statements.len() == 1 && statements[0].text == "SELECT 2"
        ));
        harness.press(Key::Enter, Modifiers::COMMAND | Modifiers::SHIFT);
        assert!(matches!(
            harness.app.backend.sent.last(),
            Some(crate::backend::Command::RunSql { statements, .. }) if statements.len() == 2
        ));
        // Mod+Return did not type a newline.
        let workspace = harness.app.workspace(tab).unwrap();
        assert_eq!(
            workspace.active_sql_tab().unwrap().text,
            "SELECT 1;\nSELECT 2"
        );
    }

    #[test]
    fn command_return_runs_the_statement_at_the_cursor_and_with_shift_all() {
        let mut harness = Harness::new();
        let tab = harness.connect_fake();
        harness.press(Key::T, Modifiers::COMMAND);
        let text = "SELECT 1;\nSELECT 2";
        type_text(&mut harness, text);
        // The cursor goes where the editor's own keys put it.
        harness.press(Key::ArrowUp, Modifiers::NONE);
        let sent = harness.app.backend.sent.len();
        harness.press(Key::Enter, Modifiers::COMMAND);
        assert!(matches!(
            harness.app.backend.sent.last(),
            Some(Command::RunSql { statements, .. })
                if statements.len() == 1 && statements[0].text == "SELECT 1"
        ));
        harness.press(Key::Enter, Modifiers::COMMAND | Modifiers::SHIFT);
        assert!(matches!(
            harness.app.backend.sent.last(),
            Some(Command::RunSql { statements, .. }) if statements.len() == 2
        ));
        let runs = harness.app.backend.sent[sent..]
            .iter()
            .filter(|command| matches!(command, Command::RunSql { .. }))
            .count();
        assert_eq!(runs, 2, "each press runs once");
        let workspace = harness.app.workspace(tab).unwrap();
        assert_eq!(workspace.active_sql_tab().unwrap().text, text);
    }

    /// Types `text` into whatever has the keyboard.
    fn type_text(harness: &mut Harness, text: &str) {
        harness.frame(vec![egui::Event::Text(text.into())]);
        harness.settle();
    }

    /// Clicks the pointer at `pos`.
    fn click_at(harness: &mut Harness, pos: egui::Pos2) {
        let button = |pressed| egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: Modifiers::NONE,
        };
        harness.frame(vec![egui::Event::PointerMoved(pos)]);
        harness.frame(vec![button(true)]);
        harness.frame(vec![button(false)]);
        harness.settle();
    }

    /// Where the SQL editor's text field is.
    fn editor_rect(harness: &mut Harness) -> egui::Rect {
        let tree = harness.settle();
        crate::testing::bounds(&tree, "SQL", egui::accesskit::Role::MultilineTextInput)
            .expect("the editor")
    }

    /// Whether the last frame painted `text` as one piece in `color`.
    fn painted_in(harness: &Harness, text: &str, color: egui::Color32) -> bool {
        let mut pieces = harness.painted.iter();
        pieces.any(|(piece, painted)| piece == text && *painted == color)
    }

    #[test]
    fn the_editor_is_a_named_text_field_with_the_query() {
        for look in crate::theme::Look::ALL {
            let mut harness = Harness::new();
            harness.set_look(look);
            let tab = harness.connect_fake();
            harness.press(Key::T, Modifiers::COMMAND);
            harness.frame(vec![egui::Event::Text("SELECT 1".into())]);
            let tree = harness.settle();
            let editor =
                crate::testing::node(&tree, "SQL", egui::accesskit::Role::MultilineTextInput);
            assert!(editor.is_some(), "{}", look.name);
            let sql = active_sql(&harness, tab);
            assert_eq!(sql.text, "SELECT 1", "{}", look.name);
            assert_eq!(sql.cursor, 8, "{}", look.name);
        }
    }

    #[test]
    fn the_editor_reports_its_cursor_in_bytes() {
        let mut harness = Harness::new();
        let tab = harness.connect_fake();
        harness.press(Key::T, Modifiers::COMMAND);
        // Two bytes each, and a line ending of two characters.
        type_text(&mut harness, "-- żółw\r\nSELECT 'é'");
        let sql = active_sql(&harness, tab);
        assert_eq!(sql.cursor, sql.text.len());
        assert_eq!(sql.line_col(), (2, 11));
        harness.press(Key::ArrowLeft, Modifiers::NONE);
        harness.press(Key::ArrowLeft, Modifiers::NONE);
        let sql = active_sql(&harness, tab);
        assert_eq!(&sql.text[sql.cursor..], "é'");
        assert_eq!(sql.line_col(), (2, 9));
        // Typed there, text lands between the characters, not inside one.
        type_text(&mut harness, "ü");
        let sql = active_sql(&harness, tab);
        assert_eq!(sql.text, "-- żółw\r\nSELECT 'üé'");
        assert_eq!(&sql.text[sql.cursor..], "é'");
        harness.press(Key::ArrowUp, Modifiers::NONE);
        assert_eq!(active_sql(&harness, tab).line_col().0, 1);
    }

    #[test]
    fn a_new_editor_asks_for_the_frame_that_gives_it_the_keys() {
        let mut harness = Harness::new();
        let tab = harness.connect_fake();
        harness.settle();
        harness.app.apply(crate::model::Action::NewSqlTab(tab));
        // The frame that draws the editor asks for the keyboard, and for
        // the frame that has it: no event need come first.
        harness.frame(Vec::new());
        assert_eq!(harness.repaint_after, std::time::Duration::ZERO);
        harness.frame(Vec::new());
        assert!(harness.ctx.text_edit_focused());
        harness.settle();
        assert!(harness.repaint_after > std::time::Duration::ZERO, "idle");
        // The footer is drawn before the editor moves the cursor: the
        // frame of a key is followed by one that says where it went.
        type_text(&mut harness, "SELECT 1");
        harness.frame(vec![crate::testing::key(Key::ArrowLeft, Modifiers::NONE)]);
        assert_eq!(active_sql(&harness, tab).cursor, 7);
        assert_eq!(harness.repaint_after, std::time::Duration::ZERO);
        assert!(!harness.has("Ln 1, Col 9") && harness.has("Ln 1, Col 8"));
        assert!(harness.repaint_after > std::time::Duration::ZERO, "idle");
    }

    #[test]
    fn escape_leaves_the_editor() {
        let mut harness = Harness::new();
        let tab = harness.connect_fake();
        harness.press(Key::T, Modifiers::COMMAND);
        assert!(harness.ctx.text_edit_focused(), "a new editor has the keys");
        harness.press(Key::Escape, Modifiers::NONE);
        assert!(!harness.ctx.text_edit_focused());
        type_text(&mut harness, "SELECT 1");
        assert_eq!(active_sql(&harness, tab).text, "");
    }

    #[test]
    fn tab_indents_in_the_editor() {
        let mut harness = Harness::new();
        let tab = harness.connect_fake();
        harness.press(Key::T, Modifiers::COMMAND);
        harness.press(Key::Tab, Modifiers::NONE);
        type_text(&mut harness, "SELECT 1");
        assert!(
            harness.ctx.text_edit_focused(),
            "the keys stay in the editor"
        );
        assert_eq!(active_sql(&harness, tab).text, "\tSELECT 1");
    }

    #[test]
    fn arrows_move_the_cursor_not_the_result_while_the_editor_has_the_keys() {
        let mut harness = Harness::new();
        let tab = harness.connect_fake();
        let id = with_sql_result(&mut harness, tab, 30);
        harness.settle();
        assert!(harness.ctx.text_edit_focused());
        assert_eq!(active_sql(&harness, tab).cursor, 8, "after the text");
        harness.press(Key::ArrowLeft, Modifiers::NONE);
        assert_eq!(active_sql(&harness, tab).cursor, 7);
        harness.press(Key::Home, Modifiers::NONE);
        assert_eq!(active_sql(&harness, tab).cursor, 0);
        for key in [
            Key::ArrowDown,
            Key::ArrowRight,
            Key::ArrowUp,
            Key::PageDown,
            Key::PageUp,
            Key::End,
        ] {
            harness.press(key, Modifiers::NONE);
        }
        assert_eq!(sql_selection(&harness, tab, id), None);
        assert_eq!(active_sql(&harness, tab).text, "SELECT 1");
    }

    #[test]
    fn the_terminal_letters_are_typed_while_the_editor_has_the_keys() {
        let mut harness = Harness::new();
        harness.set_look(crate::theme::Look::omarchy());
        let tab = harness.connect_fake();
        let id = with_sql_result(&mut harness, tab, 3);
        harness.settle();
        // A real key press sends the key and its text.
        harness.frame(vec![
            crate::testing::key(Key::J, Modifiers::NONE),
            egui::Event::Text("j".into()),
        ]);
        harness.settle();
        assert_eq!(active_sql(&harness, tab).text, "SELECT 1j");
        assert_eq!(sql_selection(&harness, tab, id), None);
    }

    #[test]
    fn clicking_the_editor_takes_the_arrows_from_the_tree() {
        let (mut harness, tab) = tree_harness();
        let id = with_sql_result(&mut harness, tab, 3);
        harness.press(Key::Escape, Modifiers::NONE);
        // A table opened from the tree leaves the arrows there.
        harness.click("users");
        harness.click("Query 1 tab");
        let pane = |harness: &Harness| harness.app.workspace(tab).unwrap().pane;
        assert_eq!(pane(&harness), crate::model::Pane::Tree);
        assert!(!harness.ctx.text_edit_focused());
        let editor = editor_rect(&mut harness);
        click_at(&mut harness, editor.center());
        assert!(harness.ctx.text_edit_focused(), "a click gives it the keys");
        assert_eq!(pane(&harness), crate::model::Pane::Grid);
        // So the arrows the editor gives up go to its result.
        let cursor = harness.app.workspace(tab).unwrap().tree.cursor.clone();
        harness.press(Key::Escape, Modifiers::NONE);
        harness.press(Key::ArrowDown, Modifiers::NONE);
        harness.press(Key::ArrowDown, Modifiers::NONE);
        assert_eq!(
            sql_selection(&harness, tab, id),
            Some(crate::model::CellPos { row: 1, col: 0 })
        );
        assert_eq!(harness.app.workspace(tab).unwrap().tree.cursor, cursor);
    }

    /// A script of `lines` lines, the last of them a statement.
    fn script_of(lines: usize) -> String {
        format!("{}SELECT x", "-- a note\n".repeat(lines - 1))
    }

    #[test]
    fn the_gutter_numbers_every_line() {
        for look in [crate::theme::Look::standard(), crate::theme::Look::macos()] {
            let (mut harness, _) = sql_harness(look);
            harness.settle();
            type_text(&mut harness, &script_of(12));
            for line in ["10", "11", "12"] {
                assert!(painted(&harness, line), "{line} in {}", look.name);
            }
            assert!(!painted(&harness, "13"), "{}", look.name);
            // The numbers do not follow the cursor.
            for _ in 0..11 {
                harness.press(Key::ArrowUp, Modifiers::NONE);
            }
            assert!(painted(&harness, "11") && painted(&harness, "12"));
        }
    }

    #[test]
    fn the_terminal_gutter_counts_lines_from_the_cursor() {
        let (mut harness, tab) = sql_harness(crate::theme::Look::omarchy());
        harness.settle();
        type_text(&mut harness, &script_of(12));
        // The cursor's line is 12; the first line is 11 lines away.
        assert_eq!(active_sql(&harness, tab).line_col().0, 12);
        assert!(painted(&harness, "12") && painted(&harness, "11"));
        // On the first line, the last one is 11 away and none is 12.
        for _ in 0..11 {
            harness.press(Key::ArrowUp, Modifiers::NONE);
        }
        assert_eq!(active_sql(&harness, tab).line_col().0, 1);
        assert!(painted(&harness, "11"));
        assert!(!painted(&harness, "12"));
    }

    #[test]
    fn a_long_line_does_not_wrap() {
        for look in crate::theme::Look::ALL {
            let (mut harness, tab) = sql_harness(look);
            harness.settle();
            let long = format!("SELECT {}\n", "a_column, ".repeat(400));
            type_text(&mut harness, &format!("{long}{}", script_of(10)));
            assert_eq!(active_sql(&harness, tab).line_col().0, 11);
            // Eleven lines are eleven rows, however long the first is.
            assert!(painted(&harness, "11"), "{}", look.name);
            assert!(!painted(&harness, "12"), "{}", look.name);
        }
    }

    #[test]
    fn the_gutter_marks_the_line_that_failed_until_the_text_changes() {
        for look in crate::theme::Look::ALL {
            let (mut harness, tab) = sql_harness(look);
            let danger = harness.app.palette.danger;
            harness.settle();
            type_text(&mut harness, &script_of(11));
            assert!(painted(&harness, "11") && !painted_in(&harness, "11", danger));
            harness.press(Key::Enter, Modifiers::COMMAND);
            harness.answer_sql(
                Ok(crate::testing::script_outcome(vec![
                    crate::testing::error_outcome("no such column: x", None),
                ])),
                None,
            );
            harness.settle();
            assert!(painted_in(&harness, "11", danger), "{}", look.name);
            // The statement moves down a line: the mark would be a line off.
            for _ in 0..10 {
                harness.press(Key::ArrowUp, Modifiers::NONE);
            }
            harness.press(Key::Enter, Modifiers::NONE);
            assert_eq!(active_sql(&harness, tab).line_col().0, 2);
            let marked = harness
                .painted
                .iter()
                .any(|(piece, color)| *color == danger && piece.parse::<usize>().is_ok());
            assert!(!marked, "{}", look.name);
        }
    }

    #[test]
    fn the_script_is_tokenized_once_when_it_changes_and_not_otherwise() {
        use crate::ui::sql_text::TOKENIZED;
        let count = || TOKENIZED.with(std::cell::Cell::get);
        let mut harness = Harness::new();
        let tab = harness.connect_fake();
        harness.press(Key::T, Modifiers::COMMAND);
        let line = "SELECT id, 'a name', 42 FROM users WHERE id > 7; -- a note\n";
        harness.frame(vec![egui::Event::Paste(line.repeat(500))]);
        harness.settle();
        assert_eq!(active_sql(&harness, tab).line_col().0, 501);
        // Frames that leave the text alone tokenize nothing: idle ones, a
        // moved cursor, a moved pointer.
        let before = count();
        harness.settle();
        harness.press(Key::ArrowUp, Modifiers::NONE);
        harness.frame(vec![egui::Event::PointerMoved(egui::pos2(700.0, 300.0))]);
        assert_eq!(active_sql(&harness, tab).line_col().0, 500);
        assert_eq!(count(), before);
        // A frame that changes it tokenizes it once, for the colours and
        // for the statements both.
        harness.frame(vec![egui::Event::Text("x".into())]);
        assert_eq!(count(), before + 1);
        harness.settle();
        assert_eq!(count(), before + 1);
        // Running splits the script in the app, not in the view.
        harness.press(Key::Enter, Modifiers::COMMAND);
        assert!(matches!(
            harness.app.backend.sent.last(),
            Some(Command::RunSql { .. })
        ));
        assert_eq!(count(), before + 1);
        // Another editor's script is its own: coming back costs nothing.
        harness.press(Key::T, Modifiers::COMMAND);
        type_text(&mut harness, "SELECT 2");
        let after = count();
        harness.click("Query 1 tab");
        harness.click("Query 2 tab");
        assert_eq!(count(), after);
    }

    #[test]
    fn a_new_palette_recolours_the_script() {
        let mut harness = Harness::new();
        harness.connect_fake();
        harness.press(Key::T, Modifiers::COMMAND);
        type_text(&mut harness, "SELECT 1");
        let old = harness.app.palette;
        let new = if old.dark {
            crate::theme::Palette::light()
        } else {
            crate::theme::Palette::dark()
        };
        assert_ne!(old.magenta, new.magenta);
        // The script is painted as one piece, in its first token's colour.
        assert!(painted_in(&harness, "SELECT 1", old.magenta));
        harness.app.palette = new;
        harness.settle();
        assert!(painted_in(&harness, "SELECT 1", new.magenta));
    }

    #[test]
    fn a_closed_editor_leaves_nothing_in_eguis_memory() {
        use crate::ui::sql_text::{editor_id, remembered};
        let mut harness = Harness::new();
        let tab = harness.connect_fake();
        let open = |harness: &mut Harness| {
            harness.press(Key::T, Modifiers::COMMAND);
            type_text(harness, "SELECT 1");
            harness.app.workspace(tab).unwrap().active_tab.unwrap()
        };
        let everything = ["text", "parsed", "scroll id", "scroll"];
        // The text field's state is its cursor and its undo history, which
        // holds copies of the script.
        let first = open(&mut harness);
        let state = |harness: &Harness, id| {
            egui::TextEdit::load_state(&harness.ctx, editor_id(tab, id)).is_some()
        };
        assert!(state(&harness, first));
        assert_eq!(remembered(&harness.ctx, tab, first), everything);
        harness.press(Key::W, Modifiers::COMMAND);
        assert!(!state(&harness, first));
        assert!(remembered(&harness.ctx, tab, first).is_empty());
        // Editors that go with their connection tab are forgotten too,
        // the one shown and the one behind it.
        let second = open(&mut harness);
        let third = open(&mut harness);
        for id in [second, third] {
            assert_eq!(remembered(&harness.ctx, tab, id), everything);
        }
        harness.press(Key::W, Modifiers::COMMAND | Modifiers::SHIFT);
        assert!(harness.app.workspace(tab).is_none());
        for id in [second, third] {
            assert!(!state(&harness, id));
            assert!(remembered(&harness.ctx, tab, id).is_empty());
        }
        assert!(harness.app.closed_editors.is_empty(), "each forgotten once");
    }

    #[test]
    fn the_statement_at_the_cursor_is_marked_and_a_comment_is_not() {
        for look in crate::theme::Look::ALL {
            let (mut harness, _) = sql_harness(look);
            // The bar's colour, which other things are filled with too.
            let bar = crate::ui::sql_text::bar_color(&look, &harness.app.palette);
            let bars = |harness: &Harness| {
                harness
                    .fills
                    .iter()
                    .filter(|(_, fill)| *fill == bar)
                    .count()
            };
            harness.settle();
            let none = bars(&harness);
            // A script of comments holds no statement to run.
            type_text(&mut harness, "-- only a note");
            assert_eq!(bars(&harness), none, "{}", look.name);
            type_text(&mut harness, "\nSELECT 1");
            assert_eq!(bars(&harness), none + 1, "{}", look.name);
            // It stays with the statement when the keys leave the editor.
            harness.press(Key::Escape, Modifiers::NONE);
            assert_eq!(bars(&harness), none + 1, "{}", look.name);
            let editor = editor_rect(&mut harness);
            click_at(&mut harness, editor.center());
            for _ in 0.."SELECT 1".len() {
                harness.press(Key::Backspace, Modifiers::NONE);
            }
            assert_eq!(bars(&harness), none, "{}", look.name);
        }
    }

    #[test]
    fn a_press_on_the_gutter_gives_the_editor_the_keys_on_that_line() {
        for look in crate::theme::Look::ALL {
            let (mut harness, tab) = sql_harness(look);
            harness.settle();
            type_text(&mut harness, "SELECT 1;\nSELECT 2;\nSELECT 3");
            harness.press(Key::Escape, Modifiers::NONE);
            assert!(!harness.ctx.text_edit_focused());
            // Left of the text, beside its first line.
            let editor = editor_rect(&mut harness);
            let gutter = editor.left() - 20.0;
            click_at(&mut harness, egui::pos2(gutter, editor.top() + 12.0));
            assert!(harness.ctx.text_edit_focused(), "{}", look.name);
            assert_eq!(active_sql(&harness, tab).line_col(), (1, 1));
            type_text(&mut harness, "x");
            assert!(active_sql(&harness, tab).text.starts_with("xSELECT 1;"));
            // With the keys in the editor, a press there keeps them. Under
            // the last line, it is the end of the script.
            click_at(&mut harness, egui::pos2(gutter, editor.bottom() - 4.0));
            assert!(harness.ctx.text_edit_focused(), "{}", look.name);
            assert_eq!(active_sql(&harness, tab).line_col(), (3, 9));
            // Screen readers find the gutter by its name.
            let tree = harness.settle();
            let numbers =
                crate::testing::node(&tree, "Line numbers", egui::accesskit::Role::Unknown);
            assert!(numbers.is_some(), "{}", look.name);
        }
    }

    #[test]
    fn a_held_run_key_runs_once() {
        let mut harness = Harness::new();
        let tab = harness.connect_fake();
        harness.press(Key::T, Modifiers::COMMAND);
        set_sql(&mut harness, tab, "SELECT 1;\nSELECT 2", 0);
        let runs = |harness: &Harness| {
            let sent = &harness.app.backend.sent;
            sent.iter()
                .filter(|command| matches!(command, Command::RunSql { .. }))
                .count()
        };
        // What the window sends while the chord is held. egui decides for
        // itself what repeats: a key going down that has not come up.
        let repeat = |modifiers: Modifiers| egui::Event::Key {
            key: Key::Enter,
            physical_key: None,
            pressed: true,
            repeat: true,
            modifiers,
        };
        let down = |modifiers: Modifiers| crate::testing::key(Key::Enter, modifiers);
        let up = |modifiers: Modifiers| crate::testing::release(Key::Enter, modifiers);
        let command = Modifiers::COMMAND;
        let shift = Modifiers::COMMAND | Modifiers::SHIFT;
        harness.settle();
        harness.frame(vec![down(command), repeat(command)]);
        harness.frame(vec![repeat(command)]);
        harness.frame(vec![repeat(command), repeat(command)]);
        harness.settle();
        assert_eq!(runs(&harness), 1, "the press runs, its repeats do not");
        harness.frame(vec![up(command)]);
        harness.frame(vec![down(shift)]);
        harness.frame(vec![repeat(shift)]);
        harness.settle();
        assert_eq!(runs(&harness), 2, "nor does a repeat run the statement");
        assert!(matches!(
            harness.app.backend.sent.last(),
            Some(Command::RunSql { statements, .. }) if statements.len() == 2
        ));
        // Let go and pressed again, it runs again.
        harness.frame(vec![up(shift)]);
        harness.frame(vec![down(command)]);
        harness.settle();
        assert_eq!(runs(&harness), 3);
    }

    #[test]
    fn command_return_does_nothing_on_an_object_tab() {
        let mut harness = Harness::new();
        let tab = with_page(&mut harness);
        let sent = harness.app.backend.sent.len();
        harness.press(Key::Enter, Modifiers::COMMAND);
        harness.press(Key::Enter, Modifiers::COMMAND | Modifiers::SHIFT);
        assert_eq!(harness.app.backend.sent.len(), sent);
        assert!(harness.app.workspace(tab).unwrap().row_panel);
    }

    /// Selects the bytes `range` of the active SQL editor's text, as a
    /// drag over them would.
    fn select_sql(
        harness: &mut Harness,
        tab: crate::model::ConnTabId,
        range: std::ops::Range<usize>,
    ) {
        // A field without the keys drops its selection when it is drawn.
        assert!(harness.ctx.text_edit_focused());
        let sql = active_sql(harness, tab);
        let chars = |byte: usize| sql.text[..byte].chars().count();
        let selection = egui::text::CCursorRange::two(
            egui::text::CCursor::new(chars(range.start)),
            egui::text::CCursor::new(chars(range.end)),
        );
        let id = crate::ui::sql_text::editor_id(tab, sql.id);
        let mut state = egui::TextEdit::load_state(&harness.ctx, id).unwrap_or_default();
        state.cursor.set_char_range(Some(selection));
        egui::TextEdit::store_state(&harness.ctx, id, state);
        harness.settle();
    }

    const COMMAND_SHIFT: Modifiers = Modifiers::COMMAND.plus(Modifiers::SHIFT);

    #[test]
    fn command_shift_f_formats_the_script_and_the_editor_keeps_the_keys() {
        let mut harness = Harness::new();
        let tab = harness.connect_fake();
        harness.press(Key::T, Modifiers::COMMAND);
        type_text(&mut harness, "select a,b from t");
        harness.press(Key::F, COMMAND_SHIFT);
        let formatted = "SELECT a,\n       b\n  FROM t";
        let sql = active_sql(&harness, tab);
        assert_eq!(sql.text, formatted);
        assert_eq!(
            sql.cursor,
            formatted.len(),
            "the cursor is still at the end"
        );
        assert!(harness.ctx.text_edit_focused());
        // Typing goes on where the cursor is.
        type_text(&mut harness, ";");
        assert_eq!(active_sql(&harness, tab).text, format!("{formatted};"));
        // With the keys given up (Esc) it formats too, and takes them back.
        harness.press(Key::Escape, Modifiers::NONE);
        assert!(!harness.ctx.text_edit_focused());
        set_sql(&mut harness, tab, "select 1", 0);
        harness.press(Key::F, COMMAND_SHIFT);
        assert_eq!(active_sql(&harness, tab).text, "SELECT 1");
        assert!(harness.ctx.text_edit_focused());
    }

    #[test]
    fn one_undo_gives_back_the_script_as_typed() {
        let mut harness = Harness::new();
        let tab = harness.connect_fake();
        harness.press(Key::T, Modifiers::COMMAND);
        let typed = "select a,b from t";
        let formatted = "SELECT a,\n       b\n  FROM t";
        type_text(&mut harness, typed);
        harness.press(Key::F, COMMAND_SHIFT);
        assert_eq!(active_sql(&harness, tab).text, formatted);
        harness.press(Key::Z, Modifiers::COMMAND);
        let sql = active_sql(&harness, tab);
        assert_eq!(sql.text, typed);
        assert_eq!(sql.cursor, typed.len(), "and the cursor where it was");
        // Redo formats it again.
        harness.press(Key::Z, COMMAND_SHIFT);
        assert_eq!(active_sql(&harness, tab).text, formatted);
        // Formatting what is formatted adds nothing to undo: one undo is
        // still all it takes.
        harness.press(Key::F, COMMAND_SHIFT);
        harness.press(Key::F, COMMAND_SHIFT);
        assert_eq!(active_sql(&harness, tab).text, formatted);
        harness.press(Key::Z, Modifiers::COMMAND);
        assert_eq!(active_sql(&harness, tab).text, typed);
    }

    #[test]
    fn format_leaves_nothing_to_redo_as_an_edit_does() {
        let mut harness = Harness::new();
        let tab = harness.connect_fake();
        harness.press(Key::T, Modifiers::COMMAND);
        let typed = "select a,b from t";
        let formatted = "SELECT a,\n       b\n  FROM t";
        type_text(&mut harness, typed);
        harness.press(Key::F, COMMAND_SHIFT);
        harness.press(Key::Z, Modifiers::COMMAND);
        assert_eq!(active_sql(&harness, tab).text, typed);
        // Formatted again, what that undo left to redo is gone: a redo
        // changes nothing, and one undo is still all it takes.
        harness.press(Key::F, COMMAND_SHIFT);
        harness.press(Key::Z, COMMAND_SHIFT);
        assert_eq!(active_sql(&harness, tab).text, formatted);
        harness.press(Key::Z, Modifiers::COMMAND);
        assert_eq!(active_sql(&harness, tab).text, typed);
    }

    #[test]
    fn format_brings_the_cursor_into_view() {
        let mut harness = Harness::new();
        let tab = harness.connect_fake();
        harness.press(Key::T, Modifiers::COMMAND);
        // One line that Format lays out as more lines than the pane shows.
        let columns: Vec<String> = (0..80).map(|n| format!("c{n}")).collect();
        type_text(
            &mut harness,
            &format!("select {} from t", columns.join(",")),
        );
        let id = active_sql(&harness, tab).id;
        let scrolled = |harness: &Harness| {
            crate::ui::sql_text::scroll_offset(&harness.ctx, tab, id).expect("a scroll area")
        };
        assert_eq!(scrolled(&harness).y, 0.0);
        harness.press(Key::F, COMMAND_SHIFT);
        harness.finish_animations();
        let sql = active_sql(&harness, tab);
        assert_eq!(sql.text.lines().count(), 81);
        assert_eq!(sql.cursor, sql.text.len(), "the cursor is on the last line");
        assert!(scrolled(&harness).y > 0.0, "and that line is in view");
    }

    #[test]
    fn format_with_a_selection_formats_the_statements_it_touches() {
        let mut harness = Harness::new();
        let tab = harness.connect_fake();
        harness.press(Key::T, Modifiers::COMMAND);
        // Letters of two bytes each before the selection: egui counts a
        // cursor in characters, Format in bytes.
        let typed = "select 'żółw' ;\nselect a,b from t ;\nselect 3 ;";
        let formatted = "select 'żółw' ;\nSELECT a,\n       b\n  FROM t;\nselect 3 ;";
        type_text(&mut harness, typed);
        let list = typed.find("a,b").unwrap();
        select_sql(&mut harness, tab, list..list + 3);
        let id = crate::ui::sql_text::editor_id(tab, active_sql(&harness, tab).id);
        let selection = |harness: &Harness| {
            let state = egui::TextEdit::load_state(&harness.ctx, id).unwrap();
            state.cursor.char_range().unwrap()
        };
        let selected = selection(&harness);
        assert!(!selected.is_empty());
        harness.press(Key::F, COMMAND_SHIFT);
        let sql = active_sql(&harness, tab);
        assert_eq!(sql.text, formatted);
        // The cursor is where the selection ended, after the `b`, and
        // nothing is selected.
        let cursor = "select 'żółw' ;\nSELECT a,\n       b";
        assert_eq!(&sql.text[..sql.cursor], cursor);
        assert!(selection(&harness).is_empty());
        // Undo gives the selection back with the text.
        harness.press(Key::Z, Modifiers::COMMAND);
        assert_eq!(active_sql(&harness, tab).text, typed);
        assert_eq!(selection(&harness), selected);
        // Formatted again (redo), typing adds to the text at the cursor.
        harness.press(Key::Z, COMMAND_SHIFT);
        type_text(&mut harness, "2");
        assert_eq!(
            active_sql(&harness, tab).text,
            formatted.replace("       b\n", "       b2\n")
        );
    }

    #[test]
    fn the_format_button_formats_and_gives_the_keys_back() {
        for look in [crate::theme::Look::standard(), crate::theme::Look::macos()] {
            let (mut harness, tab) = sql_harness(look);
            // Wide enough for the buttons' keys.
            harness.size.x = 1600.0;
            harness.settle();
            type_text(&mut harness, "select 1");
            let keys = if look == crate::theme::Look::macos() {
                "⇧⌘F"
            } else {
                "Ctrl+Shift+F"
            };
            assert!(painted(&harness, keys), "{keys} in {}", look.name);
            // The editor gives the keys up (Esc); the button formats and
            // gives them back.
            harness.press(Key::Escape, Modifiers::NONE);
            assert!(!harness.ctx.text_edit_focused(), "{}", look.name);
            harness.click("Format");
            assert_eq!(active_sql(&harness, tab).text, "SELECT 1", "{}", look.name);
            assert!(harness.ctx.text_edit_focused(), "{}", look.name);
        }
        // The terminal has the key alone.
        let (mut harness, tab) = sql_harness(crate::theme::Look::omarchy());
        harness.settle();
        type_text(&mut harness, "select 1");
        assert!(!harness.has("Format"));
        harness.press(Key::F, COMMAND_SHIFT);
        assert_eq!(active_sql(&harness, tab).text, "SELECT 1");
    }

    #[test]
    fn command_shift_f_does_nothing_on_a_table_tab() {
        let mut harness = Harness::new();
        let tab = harness.connect_fake();
        harness.app.apply(crate::model::Action::OpenObject {
            tab,
            object: tabletist_db::ObjectRef::new("main", "users"),
            kind: tabletist_db::ObjectKind::Table,
            pin: true,
        });
        harness.answer_rows(crate::testing::page(3, false));
        // Mod+F would take the press for its own were it not consumed.
        harness.press(Key::F, COMMAND_SHIFT);
        assert!(!harness.has("Apply"), "the filter bar stays shut");
        harness.press(Key::F, Modifiers::COMMAND);
        assert!(harness.has("Apply"));
    }

    #[test]
    fn command_period_cancels_a_sql_run() {
        let mut harness = Harness::new();
        let tab = harness.connect_fake();
        harness.press(Key::T, Modifiers::COMMAND);
        set_sql(&mut harness, tab, "SELECT 1", 0);
        harness.press(Key::Enter, Modifiers::COMMAND);
        let Some(Command::RunSql { request: run, .. }) = harness.app.backend.sent.last() else {
            panic!("expected RunSql");
        };
        let run = *run;
        harness.press(Key::Period, Modifiers::COMMAND);
        assert!(matches!(
            crate::testing::last_sent(&harness.app),
            Command::Cancel { request, .. } if *request == run
        ));
    }

    #[test]
    fn refresh_and_filter_do_nothing_on_a_sql_tab() {
        let mut harness = Harness::new();
        let tab = harness.connect_fake();
        harness.press(Key::T, Modifiers::COMMAND);
        let sent = harness.app.backend.sent.len();
        // The two Mod+R steps are the ones that need the gate in
        // `keys::handle`: the tree has the arrows (here because the editor
        // shows no grid, below because the tree was used last), so without
        // the gate the key would refresh the tree.
        harness.press(Key::R, Modifiers::COMMAND);
        assert_eq!(harness.app.backend.sent.len(), sent, "no grid showing");
        harness.app.workspace_mut(tab).unwrap().pane = crate::model::Pane::Tree;
        harness.press(Key::R, Modifiers::COMMAND);
        assert_eq!(harness.app.backend.sent.len(), sent, "the tree pane");
        // Mod+F shows nothing of the gate: the reducers ignore the filter
        // bar on a SQL editor as well. So does Mod+R once a result's grid
        // has the arrows (the reducer has nothing to refresh).
        harness.press(Key::F, Modifiers::COMMAND);
        with_sql_result(&mut harness, tab, 3);
        let sent = harness.app.backend.sent.len();
        harness.press(Key::R, Modifiers::COMMAND);
        assert_eq!(harness.app.backend.sent.len(), sent);
        assert!(
            harness
                .app
                .workspace(tab)
                .unwrap()
                .active_sql_tab()
                .is_some()
        );
    }

    #[test]
    fn the_paging_keys_do_nothing_on_a_sql_tab_nor_the_row_panels_with_no_row_selected() {
        let mut harness = Harness::new();
        let tab = harness.connect_fake();
        harness.app.apply(crate::model::Action::OpenObject {
            tab,
            object: tabletist_db::ObjectRef::new("main", "users"),
            kind: tabletist_db::ObjectKind::Table,
            pin: true,
        });
        harness.answer_rows(crate::testing::page(300, true));
        let users = harness.app.workspace(tab).unwrap().active_tab.unwrap();
        let query = with_sql_result(&mut harness, tab, 3);
        // With the keys out of the editor, where Space would be typed. No
        // row of the result is selected, so its panel has nothing to show
        // and the keys leave it as it is.
        harness.press(Key::Escape, Modifiers::NONE);
        let panel = harness.app.workspace(tab).unwrap().row_panel;
        let sent = harness.app.backend.sent.len();
        harness.press(Key::Space, Modifiers::NONE);
        assert_eq!(harness.app.workspace(tab).unwrap().row_panel, panel);
        harness.press(Key::R, Modifiers::COMMAND | Modifiers::SHIFT);
        assert_eq!(harness.app.workspace(tab).unwrap().row_panel, panel);
        harness.press(Key::ArrowRight, Modifiers::COMMAND | Modifiers::ALT);
        harness.press(Key::ArrowLeft, Modifiers::COMMAND | Modifiers::ALT);
        assert_eq!(
            harness.app.backend.sent.len(),
            sent,
            "the table behind the editor did not page or refresh"
        );
        let workspace = harness.app.workspace(tab).unwrap();
        assert_eq!(workspace.object_tab(users).unwrap().query.offset, 0);
        assert_eq!(
            sql_selection(&harness, tab, query),
            None,
            "nor did the result's selection move"
        );
    }

    #[test]
    fn command_w_closes_a_sql_tab() {
        let mut harness = Harness::new();
        let tab = harness.connect_fake();
        harness.press(Key::T, Modifiers::COMMAND);
        harness.press(Key::W, Modifiers::COMMAND);
        assert!(harness.app.workspace(tab).unwrap().tabs.is_empty());
        assert_eq!(harness.app.tabs.len(), 1, "not the connection tab");
    }

    #[test]
    fn command_shift_brackets_cycle_through_tabs_of_both_kinds() {
        let mut harness = Harness::new();
        let tab = with_page(&mut harness);
        let users = harness.app.workspace(tab).unwrap().active_tab;
        harness.press(Key::T, Modifiers::COMMAND);
        let query = harness.app.workspace(tab).unwrap().active_tab;
        assert_ne!(query, users);
        let shift = Modifiers::COMMAND | Modifiers::SHIFT;
        harness.press(Key::OpenBracket, shift);
        assert_eq!(harness.app.workspace(tab).unwrap().active_tab, users);
        harness.press(Key::CloseBracket, shift);
        assert_eq!(harness.app.workspace(tab).unwrap().active_tab, query);
        // US layouts report the curly bracket with Shift held.
        harness.press(Key::CloseCurlyBracket, shift);
        assert_eq!(harness.app.workspace(tab).unwrap().active_tab, users);
    }

    #[test]
    fn the_shortcuts_of_any_tab_work_on_a_sql_tab() {
        let (mut harness, tab) = tree_harness();
        harness.app.apply(crate::model::Action::ShowConnections);
        harness.press(Key::Num1, Modifiers::COMMAND);
        assert_eq!(harness.app.active_tab_id(), tab);
        harness.press(Key::T, Modifiers::COMMAND);
        let workspace = harness.app.workspace(tab).unwrap();
        assert!(workspace.active_sql_tab().is_some());
        harness.press(Key::Tab, Modifiers::CTRL);
        assert_eq!(harness.app.active, 1);
        harness.press(Key::Num1, Modifiers::COMMAND);
        assert_eq!(harness.app.active, 0);
        assert!(harness.has("orders"));
        harness.press(Key::B, Modifiers::COMMAND);
        assert!(!harness.has("orders"));
        harness.press(Key::B, Modifiers::COMMAND);
        harness.press(Key::P, Modifiers::COMMAND);
        assert!(harness.has("Open table or view"));
        harness.press(Key::Escape, Modifiers::NONE);
        assert!(harness.app.dialog.is_none());
        harness.frame(vec![egui::Event::Text("?".into())]);
        assert!(harness.has("Keyboard shortcuts"));
    }

    #[test]
    fn after_escape_arrows_move_in_a_sql_result() {
        let mut harness = Harness::new();
        let tab = harness.connect_fake();
        let id = with_sql_result(&mut harness, tab, 3);
        harness.settle();
        assert!(harness.ctx.text_edit_focused(), "the editor has the keys");
        harness.press(Key::Escape, Modifiers::NONE);
        assert!(!harness.ctx.text_edit_focused());
        harness.press(Key::ArrowDown, Modifiers::NONE);
        harness.press(Key::ArrowDown, Modifiers::NONE);
        assert_eq!(
            sql_selection(&harness, tab, id),
            Some(crate::model::CellPos { row: 1, col: 0 })
        );
    }

    #[test]
    fn the_grid_keys_move_in_a_sql_result() {
        let mut harness = Harness::new();
        let tab = harness.connect_fake();
        let id = with_sql_result(&mut harness, tab, 30);
        let at = |row, col| Some(crate::model::CellPos { row, col });
        harness.press(Key::Escape, Modifiers::NONE);
        harness.press(Key::End, Modifiers::NONE);
        // The first key selects the first cell.
        assert_eq!(sql_selection(&harness, tab, id), at(0, 0));
        harness.press(Key::End, Modifiers::NONE);
        assert_eq!(sql_selection(&harness, tab, id), at(29, 0));
        harness.press(Key::Home, Modifiers::NONE);
        assert_eq!(sql_selection(&harness, tab, id), at(0, 0));
        harness.press(Key::PageDown, Modifiers::NONE);
        assert_eq!(sql_selection(&harness, tab, id), at(20, 0));
        harness.press(Key::ArrowRight, Modifiers::NONE);
        harness.press(Key::ArrowUp, Modifiers::NONE);
        assert_eq!(sql_selection(&harness, tab, id), at(19, 1));
        harness.press(Key::PageUp, Modifiers::NONE);
        harness.press(Key::ArrowLeft, Modifiers::NONE);
        assert_eq!(sql_selection(&harness, tab, id), at(0, 0));
    }

    #[test]
    fn arrows_stay_in_the_tree_while_a_sql_tab_shows_no_grid() {
        let (mut harness, tab) = tree_harness();
        let cursor = |harness: &Harness| harness.app.workspace(tab).unwrap().tree.cursor.clone();
        // An editor that has run nothing has no grid.
        harness.press(Key::T, Modifiers::COMMAND);
        harness.press(Key::Escape, Modifiers::NONE);
        let before = cursor(&harness);
        harness.press(Key::ArrowDown, Modifiers::NONE);
        let moved = cursor(&harness);
        assert_ne!(moved, before);
        // Nor has one showing its messages over a result.
        harness.press(Key::W, Modifiers::COMMAND);
        let id = with_sql_result(&mut harness, tab, 3);
        harness.press(Key::Escape, Modifiers::NONE);
        harness.app.apply(crate::model::Action::SetResultPane {
            tab,
            sql_tab: id,
            pane: crate::model::ResultPane::Messages,
        });
        harness.press(Key::ArrowDown, Modifiers::NONE);
        assert_ne!(cursor(&harness), moved);
        assert_eq!(sql_selection(&harness, tab, id), None);
        // With the tree clicked last, a shown grid leaves the arrows there.
        harness.app.apply(crate::model::Action::SetResultPane {
            tab,
            sql_tab: id,
            pane: crate::model::ResultPane::Results,
        });
        harness.app.workspace_mut(tab).unwrap().pane = crate::model::Pane::Tree;
        harness.press(Key::ArrowUp, Modifiers::NONE);
        assert_eq!(cursor(&harness), moved);
        assert_eq!(sql_selection(&harness, tab, id), None);
    }

    #[test]
    fn the_terminal_letters_move_in_a_sql_result() {
        let mut harness = Harness::new();
        harness.set_look(crate::theme::Look::omarchy());
        let tab = harness.connect_fake();
        let id = with_sql_result(&mut harness, tab, 3);
        let at = |row, col| Some(crate::model::CellPos { row, col });
        harness.press(Key::Escape, Modifiers::NONE);
        harness.press(Key::J, Modifiers::NONE);
        harness.press(Key::J, Modifiers::NONE);
        harness.press(Key::L, Modifiers::NONE);
        assert_eq!(sql_selection(&harness, tab, id), at(1, 1));
        harness.press(Key::K, Modifiers::NONE);
        harness.press(Key::H, Modifiers::NONE);
        assert_eq!(sql_selection(&harness, tab, id), at(0, 0));
        // Not in the messages, which have no cells.
        harness.app.apply(crate::model::Action::SetResultPane {
            tab,
            sql_tab: id,
            pane: crate::model::ResultPane::Messages,
        });
        harness.press(Key::J, Modifiers::NONE);
        assert_eq!(sql_selection(&harness, tab, id), at(0, 0));
    }

    #[test]
    fn letters_do_not_move_in_a_sql_result_outside_the_terminal_look() {
        let mut harness = Harness::new();
        let tab = harness.connect_fake();
        let id = with_sql_result(&mut harness, tab, 3);
        harness.press(Key::Escape, Modifiers::NONE);
        harness.press(Key::J, Modifiers::NONE);
        assert_eq!(sql_selection(&harness, tab, id), None);
    }

    /// Opens a SQL editor in `tab` of a harness drawn with `look`.
    fn sql_harness(look: crate::theme::Look) -> (Harness, crate::model::ConnTabId) {
        let mut harness = Harness::new();
        harness.set_look(look);
        let tab = harness.connect_fake();
        harness.app.apply(crate::model::Action::NewSqlTab(tab));
        (harness, tab)
    }

    fn active_sql(harness: &Harness, tab: crate::model::ConnTabId) -> &crate::model::SqlTab {
        let workspace = harness.app.workspace(tab).unwrap();
        workspace.active_sql_tab().unwrap()
    }

    /// Whether the last frame painted `text` as one piece.
    fn painted(harness: &Harness, text: &str) -> bool {
        harness.painted.iter().any(|(piece, _)| piece == text)
    }

    #[test]
    fn a_sql_tab_shows_its_toolbar_in_every_look() {
        for look in crate::theme::Look::ALL {
            let (mut harness, _tab) = sql_harness(look);
            let tree = harness.settle();
            for name in ["Run", "Run all", "Query 1 tab"] {
                assert!(
                    crate::testing::node(&tree, name, egui::accesskit::Role::Button).is_some(),
                    "{name} in {}",
                    look.name
                );
            }
            for name in ["Limit", "Timeout"] {
                assert!(
                    crate::testing::node(&tree, name, egui::accesskit::Role::ComboBox).is_some(),
                    "{name} in {}",
                    look.name
                );
            }
            // What the menus are set to is their value.
            let values: Vec<_> = tree
                .nodes
                .iter()
                .filter(|(_, node)| node.role() == egui::accesskit::Role::ComboBox)
                .filter_map(|(_, node)| node.value().map(str::to_owned))
                .collect();
            // Named alike in every look; only what is painted is lower
            // case in the terminal's.
            for value in ["Limit 1,000", "Timeout 30 s"] {
                assert!(
                    values.iter().any(|found| found == value),
                    "{value} in {}: {values:?}",
                    look.name
                );
            }
            assert!(
                harness.has("Read-only transaction"),
                "the transaction note in {}",
                look.name
            );
            let reads = if look.terminal {
                ["limit 1000", "timeout 30s", "read-only transaction"]
            } else {
                ["Limit 1,000", "Timeout 30 s", "Read-only transaction"]
            };
            for text in reads {
                assert!(painted(&harness, text), "{text} in {}", look.name);
            }
            // Nothing of a table tab is drawn for an editor.
            assert!(!harness.has("Add filter") && !harness.has("Structure"));
        }
    }

    #[test]
    fn the_sidebar_button_and_the_terminal_plus_open_a_sql_editor() {
        for look in crate::theme::Look::ALL {
            let mut harness = Harness::new();
            harness.set_look(look);
            let tab = harness.connect_fake();
            // "+ sql" ends the terminal's strip; the others have a button
            // under the tree. Both are named for what they open.
            let name = if look.terminal {
                "New SQL editor"
            } else {
                "SQL Editor"
            };
            harness.click(name);
            let workspace = harness.app.workspace(tab).unwrap();
            assert!(workspace.active_sql_tab().is_some(), "{}", look.name);
            // Still there with an editor open, for the next one.
            harness.click(name);
            assert_eq!(
                harness.app.workspace(tab).unwrap().sql_tabs().count(),
                2,
                "{}",
                look.name
            );
        }
    }

    #[test]
    fn clicking_run_sends_the_statement_and_run_all_the_script() {
        for look in crate::theme::Look::ALL {
            let (mut harness, tab) = sql_harness(look);
            set_sql(&mut harness, tab, "SELECT 1;\nSELECT 2", 0);
            harness.click("Run");
            assert!(
                matches!(
                    harness.app.backend.sent.last(),
                    Some(crate::backend::Command::RunSql { statements, .. })
                        if statements.len() == 1
                ),
                "Run in {}",
                look.name
            );
            harness.click("Run all");
            assert!(
                matches!(
                    harness.app.backend.sent.last(),
                    Some(crate::backend::Command::RunSql { statements, .. })
                        if statements.len() == 2
                ),
                "Run all in {}",
                look.name
            );
        }
    }

    #[test]
    fn the_limit_and_timeout_menus_change_the_editor() {
        for look in crate::theme::Look::ALL {
            let (mut harness, tab) = sql_harness(look);
            assert_eq!(active_sql(&harness, tab).limit, 1_000);
            harness.click("Limit");
            // The choices read in the look's case and are named in one.
            let reads = if look.terminal {
                "limit 10000"
            } else {
                "Limit 10,000"
            };
            assert!(painted(&harness, reads), "{reads} in {}", look.name);
            harness.click("Limit 100");
            assert_eq!(active_sql(&harness, tab).limit, 100, "{}", look.name);
            assert_eq!(harness.app.settings.sql_limit, 100);
            // The menu closes on a pick that came from no pointer too (a
            // key, a screen reader).
            assert!(
                !harness.has("Limit 10,000"),
                "the Limit menu stays open in {}",
                look.name
            );
            harness.click("Timeout");
            harness.click("No timeout");
            assert_eq!(active_sql(&harness, tab).timeout, None, "{}", look.name);
            harness.click("Timeout");
            harness.click("Timeout 60 s");
            assert_eq!(
                active_sql(&harness, tab).timeout,
                Some(std::time::Duration::from_secs(60)),
                "{}",
                look.name
            );
        }
    }

    /// Gives the keyboard to the node named `label` with `role`, as a
    /// screen reader does.
    fn focus(harness: &mut Harness, label: &str, role: egui::accesskit::Role) {
        let tree = harness.settle();
        let target = crate::testing::node(&tree, label, role)
            .unwrap_or_else(|| panic!("nothing named {label:?}"));
        harness.frame(vec![egui::Event::AccessKitActionRequest(
            egui::accesskit::ActionRequest {
                target_tree: egui::accesskit::TreeId::ROOT,
                target_node: target,
                action: egui::accesskit::Action::Focus,
                data: None,
            },
        )]);
        harness.settle();
    }

    #[test]
    fn the_keyboard_is_back_on_a_menus_button_after_a_pick_or_escape() {
        for look in crate::theme::Look::ALL {
            let (mut harness, tab) = sql_harness(look);
            harness.click("Limit");
            harness.click("Limit 100");
            assert_eq!(
                focused_name(&harness.settle()),
                "Limit",
                "after a pick in {}",
                look.name
            );
            harness.click("Timeout");
            assert!(harness.has("No timeout"));
            harness.press(Key::Escape, Modifiers::NONE);
            assert!(!harness.has("No timeout"), "Escape closes the menu");
            assert_eq!(
                focused_name(&harness.settle()),
                "Timeout",
                "after Escape in {}",
                look.name
            );
            assert_eq!(
                active_sql(&harness, tab).timeout,
                Some(std::time::Duration::from_secs(30))
            );
        }
    }

    #[test]
    fn tab_walks_the_toolbar_in_the_order_it_reads() {
        for look in crate::theme::Look::ALL {
            let (mut harness, _tab) = sql_harness(look);
            // The terminal's toolbar leads with its menus, the others'
            // with Run.
            let order: &[&str] = if look.terminal {
                &["Limit", "Timeout", "Run", "Run all"]
            } else {
                &["Run", "Run all", "Format", "Limit", "Timeout"]
            };
            let role = if look.terminal {
                egui::accesskit::Role::ComboBox
            } else {
                egui::accesskit::Role::Button
            };
            focus(&mut harness, order[0], role);
            let mut reached = vec![focused_name(&harness.settle())];
            for _ in 1..order.len() {
                harness.press(Key::Tab, Modifiers::NONE);
                reached.push(focused_name(&harness.settle()));
            }
            assert_eq!(reached, order, "{}", look.name);
        }
    }

    #[test]
    fn the_terminal_strip_keeps_its_row_panel_toggle_on_a_sql_tab() {
        let (mut harness, tab) = sql_harness(crate::theme::Look::omarchy());
        let toggle = "Show or hide the row panel";
        let open = |harness: &Harness| harness.app.workspace(tab).unwrap().row_panel;
        assert!(harness.has(toggle), "a result has a row panel too");
        assert!(open(&harness));
        harness.click(toggle);
        assert!(!open(&harness));
        // The same toggle as a table tab's: one panel for the workspace.
        harness.click("users");
        harness.answer_rows(crate::testing::page(5, false));
        assert!(harness.has(toggle));
        assert!(!harness.has("Select a row to see its fields"));
        harness.click(toggle);
        assert!(open(&harness));
        harness.click("Query 1 tab");
        assert!(harness.has(toggle));
    }

    #[test]
    fn the_terminal_strips_toggle_hides_and_shows_a_result_rows_panel() {
        let mut harness = Harness::new();
        harness.set_look(crate::theme::Look::omarchy());
        let tab = harness.connect_fake();
        with_sql_result(&mut harness, tab, 3);
        let toggle = "Show or hide the row panel";
        harness.click("Row 2");
        assert!(panel_shows(&mut harness));
        harness.click(toggle);
        assert!(!panel_shows(&mut harness));
        harness.click(toggle);
        assert!(panel_shows(&mut harness));
    }

    #[test]
    fn the_strips_toggle_opens_a_closed_row_panel_in_every_look() {
        for look in crate::theme::Look::ALL {
            let mut harness = Harness::new();
            harness.set_look(look);
            let tab = with_page(&mut harness);
            let open = |harness: &Harness| harness.app.workspace(tab).unwrap().row_panel;
            harness.click("Row 1");
            harness.click("Close the row panel");
            assert!(!open(&harness), "{}", look.name);
            // The panel's own button went with it, and the tree has the
            // arrows, so Space is not the panel's: the strip brings it back.
            harness.click("Show or hide the row panel");
            assert!(open(&harness), "{}", look.name);
            assert!(harness.has("Close the row panel"), "{}", look.name);
        }
    }

    #[test]
    fn the_last_tab_scrolls_clear_of_the_strips_toggle_in_every_look() {
        use egui::accesskit::Role;
        for look in crate::theme::Look::ALL {
            // A narrow window and more tabs than its strip has room for.
            let mut harness = Harness::with_size(egui::vec2(700.0, 500.0));
            harness.set_look(look);
            let tab = harness.connect_fake();
            for _ in 0..8 {
                harness.add_sql_tab(tab);
            }
            harness.click("Query 1 tab");
            let place = |harness: &mut Harness, label| {
                let tree = harness.settle();
                crate::testing::bounds(&tree, label, Role::Button)
                    .unwrap_or_else(|| panic!("{label} missing in {}", look.name))
            };
            let (last, toggle) = ("Query 8 tab", "Show or hide the row panel");
            let before = place(&mut harness, toggle);
            assert!(
                place(&mut harness, last).right() > before.left(),
                "{}: the strip overflows",
                look.name
            );
            // The strip scrolled to its end.
            let over = place(&mut harness, "Query 1 tab").center();
            harness.frame(vec![egui::Event::PointerMoved(over)]);
            harness.frame(vec![
                egui::Event::PointerMoved(over),
                egui::Event::MouseWheel {
                    unit: egui::MouseWheelUnit::Point,
                    delta: egui::vec2(-5_000.0, 0.0),
                    modifiers: Modifiers::NONE,
                    phase: egui::TouchPhase::Move,
                },
            ]);
            harness.finish_animations();
            let after = place(&mut harness, toggle);
            assert_eq!(after, before, "{}: the toggle stays put", look.name);
            let last = place(&mut harness, last);
            assert!(last.right() <= after.left(), "{}", look.name);
            assert!(last.y_range().contains(after.center().y), "{}", look.name);
        }
    }

    #[test]
    fn the_strips_toggle_says_whether_the_row_panel_is_open() {
        use egui::accesskit::{Role, Toggled};
        for look in crate::theme::Look::ALL {
            let mut harness = Harness::new();
            harness.set_look(look);
            with_page(&mut harness);
            let toggle = "Show or hide the row panel";
            let state = |harness: &mut Harness| {
                let tree = harness.settle();
                let id = crate::testing::node(&tree, toggle, Role::Button);
                let (_, node) = tree
                    .nodes
                    .iter()
                    .find(|(node, _)| Some(*node) == id)
                    .unwrap_or_else(|| panic!("{toggle} missing in {}", look.name));
                node.toggled()
            };
            assert_eq!(state(&mut harness), Some(Toggled::True), "{}", look.name);
            harness.click(toggle);
            assert_eq!(state(&mut harness), Some(Toggled::False), "{}", look.name);
        }
    }

    /// The fields of the row panel that shows, by their copy buttons.
    fn panel_shows(harness: &mut Harness) -> bool {
        harness.has("Copy email")
    }

    #[test]
    fn selecting_a_result_row_opens_the_row_panel() {
        for look in crate::theme::Look::ALL {
            let mut harness = Harness::new();
            harness.set_look(look);
            let tab = harness.connect_fake();
            let id = with_sql_result(&mut harness, tab, 3);
            // Until a row is selected the editor and the results keep the
            // whole width: no panel, and no placeholder for one.
            assert!(!panel_shows(&mut harness), "{}", look.name);
            assert!(!harness.has("Close the row panel"), "{}", look.name);
            assert!(!harness.has("Select a row to see its fields"));
            // The grid paints its cells: only the panel announces a value.
            assert!(!harness.has("user2@example.com"), "{}", look.name);
            // The band under the editor is as wide as the editor.
            let wide = band(&mut harness).width();
            harness.click("Row 2");
            assert!(harness.has("user2@example.com"), "{}", look.name);
            assert_eq!(
                sql_selection(&harness, tab, id),
                Some(crate::model::CellPos { row: 1, col: 0 })
            );
            for label in [
                "Close the row panel",
                "id · INTEGER",
                "email · TEXT",
                "Copy email",
            ] {
                assert!(harness.has(label), "{label} in {}", look.name);
            }
            // The panel stands beside the editor as well as the results.
            assert!(band(&mut harness).width() < wide, "{}", look.name);
            harness.click("Copy email");
            assert_eq!(harness.copied.as_deref(), Some("user2@example.com"));
        }
    }

    #[test]
    fn a_result_rows_panel_is_titled_by_its_number_and_its_query() {
        for look in [crate::theme::Look::standard(), crate::theme::Look::macos()] {
            let mut harness = Harness::new();
            harness.set_look(look);
            let tab = harness.connect_fake();
            with_sql_result(&mut harness, tab, 3);
            let named = |harness: &Harness| {
                harness
                    .painted
                    .iter()
                    .filter(|(text, _)| text == "Query 1")
                    .count()
            };
            harness.settle();
            let before = named(&harness);
            harness.click("Row 3");
            let tree = harness.settle();
            // A result has no key to name its row by: its number does.
            assert!(
                crate::testing::node(&tree, "Row 3", egui::accesskit::Role::Label).is_some(),
                "{}: {:?}",
                look.name,
                crate::testing::labels(&tree)
            );
            assert_eq!(named(&harness), before + 1, "under it, the query's name");
        }
    }

    #[test]
    fn a_result_rows_panel_has_no_editing_controls() {
        for look in crate::theme::Look::ALL {
            let controls = if look.terminal {
                "yy p duplicate"
            } else {
                "Duplicate"
            };
            let mut harness = Harness::new();
            harness.set_look(look);
            let tab = with_page(&mut harness);
            harness.click("Row 1");
            assert!(harness.has(controls), "a table's row in {}", look.name);
            with_sql_result(&mut harness, tab, 3);
            harness.click("Row 1");
            assert!(panel_shows(&mut harness), "{}", look.name);
            assert!(!harness.has(controls), "a result's row in {}", look.name);
        }
    }

    #[test]
    fn the_terminal_panel_offers_a_result_row_no_copy_key() {
        let hints = |harness: &Harness| -> Vec<String> {
            let painted = harness.painted.iter();
            painted
                .map(|(text, _)| text.clone())
                .filter(|text| text.starts_with("za fold"))
                .collect()
        };
        let mut harness = Harness::new();
        harness.set_look(crate::theme::Look::omarchy());
        // The first row's `meta` is a document, with its keys beside it.
        let tab = with_page(&mut harness);
        harness.click("Row 1");
        harness.settle();
        assert_eq!(hints(&harness), ["za fold · y copy"]);
        // `y` copies from a table's grid only: a result's row does not
        // offer it.
        with_sql_result(&mut harness, tab, 3);
        harness.click("Row 1");
        harness.settle();
        assert_eq!(hints(&harness), ["za fold"]);
    }

    #[test]
    fn the_panels_buttons_step_through_a_results_rows_and_close_it() {
        for look in crate::theme::Look::ALL {
            let mut harness = Harness::new();
            harness.set_look(look);
            let tab = harness.connect_fake();
            let id = with_sql_result(&mut harness, tab, 3);
            let at = |row, col| Some(crate::model::CellPos { row, col });
            harness.click("Row 2");
            harness.click("Next row");
            assert_eq!(sql_selection(&harness, tab, id), at(2, 0), "{}", look.name);
            assert!(harness.has("user3@example.com"), "{}", look.name);
            harness.click("Previous row");
            harness.click("Previous row");
            assert_eq!(sql_selection(&harness, tab, id), at(0, 0), "{}", look.name);
            assert!(harness.has("user1@example.com"), "{}", look.name);
            harness.click("Close the row panel");
            assert!(!harness.app.workspace(tab).unwrap().row_panel);
            assert!(!panel_shows(&mut harness), "{}", look.name);
            // Closed, it stays closed for the next row, as a table's does.
            harness.click("Row 3");
            assert_eq!(sql_selection(&harness, tab, id), at(2, 0));
            assert!(!panel_shows(&mut harness), "{}", look.name);
        }
    }

    #[test]
    fn the_messages_pane_hides_a_result_rows_panel() {
        for look in crate::theme::Look::ALL {
            let mut harness = Harness::new();
            harness.set_look(look);
            let tab = harness.connect_fake();
            let id = with_sql_result(&mut harness, tab, 3);
            harness.click("Row 2");
            assert!(panel_shows(&mut harness), "{}", look.name);
            harness.click("Messages");
            assert!(!panel_shows(&mut harness), "{}", look.name);
            assert!(!harness.has("Close the row panel"), "{}", look.name);
            // The row stays selected, so the panel is back with its grid.
            harness.click("Results");
            assert_eq!(
                sql_selection(&harness, tab, id),
                Some(crate::model::CellPos { row: 1, col: 0 })
            );
            assert!(panel_shows(&mut harness), "{}", look.name);
        }
    }

    #[test]
    fn a_new_result_closes_the_row_panel_until_a_row_of_it_is_selected() {
        let mut harness = Harness::new();
        let tab = harness.connect_fake();
        let id = with_sql_result(&mut harness, tab, 3);
        harness.click("Row 2");
        assert!(panel_shows(&mut harness));
        // The run in flight leaves the last result, and its row, in place.
        run_sql(&mut harness, tab, id);
        assert!(panel_shows(&mut harness));
        // The second result differs from the first, so stale text shows.
        let mut page = crate::testing::page(5, false);
        page.rows[3][1] = tabletist_db::Value::Text("fourth@example.com".into());
        let second = tabletist_db::StatementOutcome::Rows {
            columns: page.columns,
            rows: page.rows,
            truncated: false,
        };
        harness.answer_sql(Ok(crate::testing::script_outcome(vec![second])), None);
        assert_eq!(sql_selection(&harness, tab, id), None);
        assert!(!panel_shows(&mut harness));
        assert!(harness.app.workspace(tab).unwrap().row_panel, "still open");
        harness.click("Row 4");
        assert!(panel_shows(&mut harness));
        assert!(harness.has("fourth@example.com"));
        // A run that failed as a whole leaves no rows, and so no panel.
        run_sql(&mut harness, tab, id);
        let lost = tabletist_db::Error::ConnectionLost("the server went away".into());
        harness.answer_sql(Err(lost), None);
        // The failure opens the Messages pane, which hides the panel by
        // itself: look at the results.
        harness.click("Results");
        assert!(!panel_shows(&mut harness));
    }

    #[test]
    fn a_table_tab_keeps_its_row_panel_beside_a_sql_tab() {
        let mut harness = Harness::new();
        let tab = with_page(&mut harness);
        with_sql_result(&mut harness, tab, 3);
        assert!(!harness.has("Select a row to see its fields"));
        harness.click("users tab");
        assert!(harness.has("Select a row to see its fields"));
        harness.click("Structure");
        assert!(!harness.has("Select a row to see its fields"));
    }

    #[test]
    fn a_result_rows_text_is_formatted_once_not_every_frame() {
        use crate::ui::format::FULL_TEXTS;
        let mut harness = Harness::new();
        let tab = harness.connect_fake();
        let mut page = crate::testing::page(2, false);
        page.rows[0][1] = tabletist_db::Value::Text("x".repeat(1_000_000).into());
        let columns = page.columns.len();
        let outcome = tabletist_db::StatementOutcome::Rows {
            columns: page.columns,
            rows: page.rows,
            truncated: false,
        };
        let id = with_sql_outcome(&mut harness, tab, outcome);
        harness.settle();
        let idle = FULL_TEXTS.with(std::cell::Cell::get);
        harness.click("Row 1");
        harness.settle();
        let before = FULL_TEXTS.with(std::cell::Cell::get);
        assert_eq!(before - idle, columns, "the selected row, once");
        for step in 0..5 {
            let at = egui::pos2(900.0 + step as f32 * 10.0, 300.0);
            harness.frame(vec![egui::Event::PointerMoved(at)]);
        }
        assert_eq!(FULL_TEXTS.with(std::cell::Cell::get), before);
        // Another row is formatted, once.
        harness.click("Row 2");
        let after = FULL_TEXTS.with(std::cell::Cell::get);
        assert_eq!(after - before, columns);
        harness.settle();
        assert_eq!(FULL_TEXTS.with(std::cell::Cell::get), after);
        // Nothing keeps the text of a row no panel shows.
        let fields = |harness: &Harness| {
            let workspace = harness.app.workspace(tab).unwrap();
            workspace.sql_tab(id).unwrap().fields.is_some()
        };
        assert!(fields(&harness));
        harness.click("Close the row panel");
        assert!(!fields(&harness));
    }

    #[test]
    fn a_result_groups_every_number_when_the_settings_say_so() {
        let mut harness = Harness::new();
        harness.app.settings.group_digits = true;
        let tab = harness.connect_fake();
        let mut page = crate::testing::page(1, false);
        page.rows[0][0] = tabletist_db::Value::Int(1_234_567);
        with_sql_outcome(
            &mut harness,
            tab,
            tabletist_db::StatementOutcome::Rows {
                columns: page.columns,
                rows: page.rows,
                truncated: false,
            },
        );
        harness.settle();
        // No structure to name a key: `id` is grouped here.
        assert!(harness.painted_color("1,234,567").is_some());
    }

    #[test]
    fn a_result_draws_a_boolean_plain_when_value_tags_are_off() {
        let painted = |value_tags: bool| {
            let mut harness = Harness::new();
            harness.app.settings.value_tags = value_tags;
            let tab = harness.connect_fake();
            let mut page = crate::testing::page(1, false);
            page.columns[2].kind = tabletist_db::ValueKind::Bool;
            page.rows[0][2] = tabletist_db::Value::Bool(true);
            with_sql_outcome(
                &mut harness,
                tab,
                tabletist_db::StatementOutcome::Rows {
                    columns: page.columns,
                    rows: page.rows,
                    truncated: false,
                },
            );
            harness.settle();
            (
                harness.painted_color("true").expect("the boolean cell"),
                harness
                    .painted_color("user1@example.com")
                    .expect("the email cell"),
            )
        };
        let (tag, plain) = painted(true);
        assert_ne!(tag, plain);
        let (flat, plain) = painted(false);
        assert_eq!(flat, plain);
    }

    #[test]
    fn an_edit_of_the_file_changes_how_an_open_result_draws_a_boolean() {
        let mut harness = Harness::new();
        let tab = harness.connect_fake();
        let mut page = crate::testing::page(1, false);
        page.columns[2].kind = tabletist_db::ValueKind::Bool;
        page.rows[0][2] = tabletist_db::Value::Bool(true);
        with_sql_outcome(
            &mut harness,
            tab,
            tabletist_db::StatementOutcome::Rows {
                columns: page.columns,
                rows: page.rows,
                truncated: false,
            },
        );
        // The boolean cell and the email cell, as the result on screen
        // draws them once the file reads `text`.
        let painted = |harness: &mut Harness, text: Option<&str>| {
            if let Some(text) = text {
                harness.app.apply(crate::model::Action::Backend(
                    crate::backend::Event::SettingsFile {
                        text: text.into(),
                        own: false,
                    },
                ));
            }
            harness.settle();
            (
                harness.painted_color("true").expect("the boolean cell"),
                harness
                    .painted_color("user1@example.com")
                    .expect("the email cell"),
            )
        };
        let (tag, plain) = painted(&mut harness, None);
        assert_ne!(tag, plain, "a tag has its colour");
        // Nothing is run again: the result that is up is drawn anew.
        let (flat, plain) = painted(&mut harness, Some("[data]\nvalue_tags = false\n"));
        assert_eq!(flat, plain);
        let (again, plain) = painted(&mut harness, Some("[data]\nvalue_tags = true\n"));
        assert_eq!(again, tag);
        assert_ne!(again, plain);
    }

    #[test]
    fn a_tables_row_text_is_back_after_the_structure_view() {
        let mut harness = Harness::new();
        with_page(&mut harness);
        harness.click("Row 1");
        assert!(harness.has("user1@example.com"));
        harness.click("Structure");
        harness.click("Data");
        assert!(harness.has("user1@example.com"));
    }

    #[test]
    fn each_sql_tab_keeps_its_own_selected_row() {
        let mut harness = Harness::new();
        let tab = harness.connect_fake();
        with_sql_result(&mut harness, tab, 3);
        let second = with_sql_result(&mut harness, tab, 3);
        harness.click("Row 3");
        harness.click("Query 1 tab");
        harness.click("Row 1");
        assert!(harness.has("user1@example.com"));
        assert!(!harness.has("user3@example.com"));
        harness.click("Query 2 tab");
        assert!(harness.has("user3@example.com"));
        assert!(!harness.has("user1@example.com"));
        harness
            .app
            .apply(crate::model::Action::CloseTab { tab, id: second });
        assert!(harness.has("user1@example.com"));
    }

    #[test]
    fn space_and_ctrl_shift_r_toggle_a_result_rows_panel() {
        let mut harness = Harness::new();
        let tab = harness.connect_fake();
        let id = with_sql_result(&mut harness, tab, 3);
        let open = |harness: &Harness| harness.app.workspace(tab).unwrap().row_panel;
        // Out of the editor, where Space would be typed, and onto a row.
        harness.press(Key::Escape, Modifiers::NONE);
        harness.press(Key::ArrowDown, Modifiers::NONE);
        assert!(panel_shows(&mut harness));
        harness.press(Key::Space, Modifiers::NONE);
        assert!(!open(&harness));
        assert!(!panel_shows(&mut harness));
        harness.press(Key::R, Modifiers::COMMAND | Modifiers::SHIFT);
        assert!(open(&harness));
        assert!(panel_shows(&mut harness));
        // Mod+Shift+R works from the editor too; Space is typed there.
        let workspace = harness.app.workspace_mut(tab).unwrap();
        workspace.sql_tab_mut(id).unwrap().focus_editor = true;
        harness.settle();
        assert!(harness.ctx.text_edit_focused());
        harness.press(Key::R, Modifiers::COMMAND | Modifiers::SHIFT);
        assert!(!open(&harness));
        harness.press(Key::Space, Modifiers::NONE);
        assert!(!open(&harness));
        // Under the messages no row shows, and the keys leave the panel be.
        harness.click("Messages");
        harness.press(Key::R, Modifiers::COMMAND | Modifiers::SHIFT);
        assert!(!open(&harness));
    }

    /// A key that `keys::letters` reads as the character it types.
    fn type_key(harness: &mut Harness, key: Key, text: &str) {
        harness.settle();
        harness.frame(vec![
            crate::testing::key(key, Modifiers::NONE),
            egui::Event::Text(text.into()),
        ]);
        harness.frame(vec![crate::testing::release(key, Modifiers::NONE)]);
        harness.settle();
    }

    #[test]
    fn the_terminal_keys_of_the_row_panel_work_on_a_result_row() {
        let mut harness = Harness::new();
        harness.set_look(crate::theme::Look::omarchy());
        let tab = harness.connect_fake();
        let id = with_sql_result(&mut harness, tab, 3);
        let open = |harness: &Harness| harness.app.workspace(tab).unwrap().row_panel;
        let at = |row, col| Some(crate::model::CellPos { row, col });
        harness.press(Key::Escape, Modifiers::NONE);
        // With no row selected the panel's keys have nothing to show.
        for key in [Key::I, Key::Enter, Key::Escape] {
            harness.press(key, Modifiers::NONE);
            assert!(open(&harness), "{key:?} with no row");
        }
        // Nor with the panel closed, where Enter and `i` would open it.
        harness.app.apply(crate::model::Action::ToggleRowPanel(tab));
        for key in [Key::Enter, Key::I] {
            harness.press(key, Modifiers::NONE);
            assert!(!open(&harness), "{key:?} with no row");
        }
        harness.app.apply(crate::model::Action::ToggleRowPanel(tab));
        // `]` and `[` step through the rows as `j` and `k` do.
        type_key(&mut harness, Key::CloseBracket, "]");
        assert_eq!(sql_selection(&harness, tab, id), at(0, 0));
        assert!(panel_shows(&mut harness));
        type_key(&mut harness, Key::CloseBracket, "]");
        assert_eq!(sql_selection(&harness, tab, id), at(1, 0));
        type_key(&mut harness, Key::OpenBracket, "[");
        assert_eq!(sql_selection(&harness, tab, id), at(0, 0));
        // Esc closes the panel, Enter opens it, `i` does either.
        harness.press(Key::Escape, Modifiers::NONE);
        assert!(!open(&harness));
        assert!(!panel_shows(&mut harness));
        // `za` with the panel closed leaves nothing folded for its next opening.
        harness.press(Key::Z, Modifiers::NONE);
        harness.press(Key::A, Modifiers::NONE);
        assert!(harness.app.workspace(tab).unwrap().fold_documents.is_none());
        harness.press(Key::Escape, Modifiers::NONE);
        assert!(!open(&harness), "Esc only closes");
        harness.press(Key::Enter, Modifiers::NONE);
        assert!(open(&harness));
        harness.press(Key::Enter, Modifiers::NONE);
        assert!(open(&harness), "Enter only opens");
        harness.press(Key::I, Modifiers::NONE);
        assert!(!open(&harness));
        harness.press(Key::I, Modifiers::NONE);
        assert!(open(&harness));
        assert!(panel_shows(&mut harness));
        // `za` folds the row's documents: the first row's `meta`.
        assert!(harness.has(r#""plan": "pro""#));
        harness.press(Key::Z, Modifiers::NONE);
        harness.press(Key::A, Modifiers::NONE);
        assert!(harness.has("{ 1 key }"));
        assert!(!harness.has(r#""plan": "pro""#));
    }

    #[test]
    fn the_terminal_keys_of_the_row_panel_work_on_a_table_row() {
        let mut harness = Harness::new();
        harness.set_look(crate::theme::Look::omarchy());
        let tab = with_page(&mut harness);
        focus_grid(&mut harness, tab);
        let open = |harness: &Harness| harness.app.workspace(tab).unwrap().row_panel;
        let at = |row, col| Some(crate::model::CellPos { row, col });
        // A table tab keeps its panel open with no row selected.
        assert!(open(&harness));
        assert!(harness.has("Select a row to see its fields"));
        // `]` and `[` step through the rows as `j` and `k` do.
        type_key(&mut harness, Key::CloseBracket, "]");
        assert_eq!(selection(&harness, tab), at(0, 0));
        assert!(panel_shows(&mut harness));
        type_key(&mut harness, Key::CloseBracket, "]");
        assert_eq!(selection(&harness, tab), at(1, 0));
        type_key(&mut harness, Key::OpenBracket, "[");
        assert_eq!(selection(&harness, tab), at(0, 0));
        // Esc closes the panel, Enter opens it, `i` does either.
        harness.press(Key::Escape, Modifiers::NONE);
        assert!(!open(&harness));
        assert!(!panel_shows(&mut harness));
        // `za` with the panel closed leaves nothing folded for its next opening.
        harness.press(Key::Z, Modifiers::NONE);
        harness.press(Key::A, Modifiers::NONE);
        assert!(harness.app.workspace(tab).unwrap().fold_documents.is_none());
        harness.press(Key::Escape, Modifiers::NONE);
        assert!(!open(&harness), "Esc only closes");
        harness.press(Key::Enter, Modifiers::NONE);
        assert!(open(&harness));
        harness.press(Key::Enter, Modifiers::NONE);
        assert!(open(&harness), "Enter only opens");
        harness.press(Key::I, Modifiers::NONE);
        assert!(!open(&harness));
        harness.press(Key::I, Modifiers::NONE);
        assert!(open(&harness));
        assert!(panel_shows(&mut harness));
        // `za` folds the row's documents: the first row's `meta`.
        assert!(harness.has(r#""plan": "pro""#));
        harness.press(Key::Z, Modifiers::NONE);
        harness.press(Key::A, Modifiers::NONE);
        assert!(harness.has("{ 1 key }"));
        assert!(!harness.has(r#""plan": "pro""#));
    }

    #[test]
    fn escape_out_of_the_editor_leaves_the_row_panel_open() {
        let mut harness = Harness::new();
        harness.set_look(crate::theme::Look::omarchy());
        let tab = harness.connect_fake();
        let id = with_sql_result(&mut harness, tab, 3);
        let open = |harness: &Harness| harness.app.workspace(tab).unwrap().row_panel;
        harness.click("Row 2");
        let workspace = harness.app.workspace_mut(tab).unwrap();
        workspace.sql_tab_mut(id).unwrap().focus_editor = true;
        harness.settle();
        assert!(harness.ctx.text_edit_focused());
        // The first Esc only leaves the editor; the next one closes the panel.
        harness.press(Key::Escape, Modifiers::NONE);
        assert!(!harness.ctx.text_edit_focused());
        assert!(open(&harness));
        assert!(panel_shows(&mut harness));
        harness.press(Key::Escape, Modifiers::NONE);
        assert!(!open(&harness));
    }

    #[test]
    fn escape_out_of_the_where_line_leaves_the_row_panel_open() {
        let mut harness = Harness::new();
        harness.set_look(crate::theme::Look::omarchy());
        let tab = with_page(&mut harness);
        focus_grid(&mut harness, tab);
        let open = |harness: &Harness| harness.app.workspace(tab).unwrap().row_panel;
        type_key(&mut harness, Key::Slash, "/");
        assert!(
            harness.ctx.text_edit_focused(),
            "the WHERE line has the keys"
        );
        // The first Esc only leaves the field; the next one closes the panel.
        harness.press(Key::Escape, Modifiers::NONE);
        assert!(!harness.ctx.text_edit_focused());
        assert!(open(&harness));
        harness.press(Key::Escape, Modifiers::NONE);
        assert!(!open(&harness));
    }

    #[test]
    fn enter_on_a_focused_button_is_the_buttons_not_the_row_panels() {
        let mut harness = Harness::new();
        harness.set_look(crate::theme::Look::omarchy());
        let tab = harness.connect_fake();
        let id = with_sql_result(&mut harness, tab, 3);
        harness.click("Row 2");
        harness.app.apply(crate::model::Action::ToggleRowPanel(tab));
        assert!(!harness.app.workspace(tab).unwrap().row_panel);
        focus(&mut harness, "Messages", egui::accesskit::Role::Button);
        harness.press(Key::Enter, Modifiers::NONE);
        let workspace = harness.app.workspace(tab).unwrap();
        assert_eq!(
            workspace.sql_tab(id).unwrap().pane,
            crate::model::ResultPane::Messages
        );
        assert!(!workspace.row_panel);
    }

    #[test]
    fn enter_on_a_focused_button_is_the_buttons_on_a_table_tab() {
        let mut harness = Harness::new();
        harness.set_look(crate::theme::Look::omarchy());
        let tab = with_page(&mut harness);
        focus_grid(&mut harness, tab);
        harness.app.apply(crate::model::Action::ToggleRowPanel(tab));
        assert!(!harness.app.workspace(tab).unwrap().row_panel);
        focus(&mut harness, "Structure", egui::accesskit::Role::Button);
        assert_eq!(focused_name(&harness.settle()), "Structure");
        harness.press(Key::Enter, Modifiers::NONE);
        let workspace = harness.app.workspace(tab).unwrap();
        assert_eq!(
            workspace.active_object_tab().unwrap().view,
            crate::model::ObjectView::Structure
        );
        assert!(!workspace.row_panel);
    }

    #[test]
    fn opening_the_row_panel_keeps_the_keyboard_on_the_results_rows() {
        for look in crate::theme::Look::ALL {
            let mut harness = Harness::new();
            harness.set_look(look);
            let tab = harness.connect_fake();
            let id = with_sql_result(&mut harness, tab, 3);
            // The keyboard comes to the rows and selects nothing; an arrow
            // picks a row, which opens the panel.
            focus(&mut harness, "Rows", egui::accesskit::Role::Group);
            assert!(!panel_shows(&mut harness), "{}", look.name);
            harness.press(Key::ArrowDown, Modifiers::NONE);
            let workspace = harness.app.workspace(tab).unwrap();
            assert!(workspace.sql_tab(id).unwrap().selection.is_some());
            assert!(panel_shows(&mut harness), "{}", look.name);
            assert_eq!(focused_name(&harness.settle()), "Rows", "{}", look.name);
        }
    }

    #[test]
    fn a_new_result_does_not_inherit_an_expanded_value() {
        let long_row_outcome = || {
            let mut page = crate::testing::page(2, false);
            page.rows[0][1] = tabletist_db::Value::Text("x".repeat(1_000_000).into());
            tabletist_db::StatementOutcome::Rows {
                columns: page.columns,
                rows: page.rows,
                truncated: false,
            }
        };
        let show_all = |harness: &mut Harness| {
            let tree = harness.settle();
            crate::testing::labels(&tree)
                .into_iter()
                .find(|label| label.starts_with("Show all"))
        };
        let mut harness = Harness::new();
        let tab = harness.connect_fake();
        let id = with_sql_outcome(&mut harness, tab, long_row_outcome());
        harness.click("Row 1");
        let link = show_all(&mut harness).expect("a link to the whole value");
        harness.click(&link);
        assert!(harness.has("Show less"));
        // The same row of another result keeps nothing expanded.
        run_sql(&mut harness, tab, id);
        harness.answer_sql(
            Ok(crate::testing::script_outcome(vec![long_row_outcome()])),
            None,
        );
        harness.click("Row 1");
        assert!(!harness.has("Show less"));
        assert!(show_all(&mut harness).is_some());
    }

    #[test]
    fn a_refreshed_page_does_not_inherit_an_expanded_value() {
        let long_row_page = || {
            let mut page = crate::testing::page(2, false);
            page.rows[0][1] = tabletist_db::Value::Text("x".repeat(1_000_000).into());
            page
        };
        let show_all = |harness: &mut Harness| {
            let tree = harness.settle();
            crate::testing::labels(&tree)
                .into_iter()
                .find(|label| label.starts_with("Show all"))
        };
        let mut harness = Harness::new();
        let tab = harness.connect_fake();
        harness.click("users");
        harness.answer_rows(long_row_page());
        harness.click("Row 1");
        let link = show_all(&mut harness).expect("a link to the whole value");
        harness.click(&link);
        assert!(harness.has("Show less"));
        // The same row of the refreshed page keeps nothing expanded.
        focus_grid(&mut harness, tab);
        let sent = harness.app.backend.sent.len();
        harness.press(Key::R, Modifiers::COMMAND);
        assert!(harness.app.backend.sent.len() > sent, "a refresh was sent");
        assert!(matches!(
            crate::testing::last_sent(&harness.app),
            Command::FetchRows { .. }
        ));
        harness.answer_rows(long_row_page());
        assert_eq!(
            selection(&harness, tab),
            Some(crate::model::CellPos { row: 0, col: 0 })
        );
        assert!(!harness.has("Show less"));
        assert!(show_all(&mut harness).is_some());
    }

    #[test]
    fn two_result_columns_of_one_name_fold_apart() {
        // `SELECT a.meta, b.meta ...`: the fixture's columns, twice.
        let page = crate::testing::page(1, false);
        let outcome = tabletist_db::StatementOutcome::Rows {
            columns: [page.columns.clone(), page.columns].concat(),
            rows: page
                .rows
                .into_iter()
                .map(|row| [row.clone(), row].concat())
                .collect(),
            truncated: false,
        };
        let named = |harness: &mut Harness, name: &str| {
            let tree = harness.settle();
            let labels = crate::testing::labels(&tree);
            labels.iter().filter(|label| *label == name).count()
        };
        for look in crate::theme::Look::ALL {
            let mut harness = Harness::new();
            harness.set_look(look);
            let tab = harness.connect_fake();
            with_sql_outcome(&mut harness, tab, outcome.clone());
            harness.click("Row 1");
            // Each document has its own toggle.
            assert_eq!(named(&mut harness, "Collapse meta"), 2, "{}", look.name);
            // Folding the first leaves the second open.
            let open = named(&mut harness, r#""plan": "pro""#);
            harness.click("Collapse meta");
            assert_eq!(named(&mut harness, "Expand meta"), 1, "{}", look.name);
            assert_eq!(named(&mut harness, "Collapse meta"), 1, "{}", look.name);
            assert!(harness.has("{ 1 key }"), "{}", look.name);
            assert_eq!(
                named(&mut harness, r#""plan": "pro""#),
                open / 2,
                "{}",
                look.name
            );
        }
    }

    #[test]
    fn the_row_panel_leaves_the_editor_half_the_room() {
        for look in crate::theme::Look::ALL {
            let mut harness = Harness::with_size(egui::vec2(1000.0, 650.0));
            harness.set_look(look);
            let tab = harness.connect_fake();
            with_sql_result(&mut harness, tab, 3);
            let room = band(&mut harness).width();
            harness.click("Row 2");
            assert!(panel_shows(&mut harness), "{}", look.name);
            let editor = band(&mut harness).width();
            assert!(editor < room, "{}", look.name);
            // A point of slack for rounding to the pixel.
            assert!(
                editor >= room / 2.0 - 1.0,
                "{editor} of {room} in {}",
                look.name
            );
        }
    }

    #[test]
    fn the_row_panel_gets_its_width_back_when_the_window_grows() {
        let mut harness = Harness::new();
        harness.set_look(crate::theme::Look::omarchy());
        let tab = harness.connect_fake();
        with_sql_result(&mut harness, tab, 3);
        let room = band(&mut harness).width();
        harness.click("Row 2");
        let wide = band(&mut harness).width();
        // A narrower window: the panel gives way to the editor.
        let narrower = 380.0;
        harness.size.x -= narrower;
        let editor = band(&mut harness).width();
        assert!(editor >= (room - narrower) / 2.0 - 1.0, "{editor}");
        // And back: the panel is as wide as it was.
        harness.size.x += narrower;
        harness.settle();
        assert!((band(&mut harness).width() - wide).abs() < 1.0);
    }

    #[test]
    fn a_tables_row_panel_gives_way_in_a_narrow_window_and_comes_back() {
        let mut harness = Harness::new();
        let tab = with_page(&mut harness);
        focus_grid(&mut harness, tab);
        harness.click("Row 1");
        // A field's label is as wide as the panel lets it be.
        let label = |harness: &mut Harness| {
            let tree = harness.settle();
            crate::testing::bounds(&tree, "id · INTEGER", egui::accesskit::Role::Label)
                .expect("the id field")
                .width()
        };
        let wide = label(&mut harness);
        harness.size.x = 800.0;
        assert!(label(&mut harness) < wide - 1.0, "the panel gave way");
        harness.size.x = 1280.0;
        harness.settle();
        assert!((label(&mut harness) - wide).abs() < 1.0, "and came back");
    }

    #[test]
    fn a_dragged_row_panel_width_is_the_one_that_comes_back() {
        for look in crate::theme::Look::ALL {
            let mut harness = Harness::new();
            harness.set_look(look);
            let tab = harness.connect_fake();
            with_sql_result(&mut harness, tab, 3);
            harness.click("Row 2");
            let opened = band(&mut harness);
            // The panel's edge is where the editor ends. It is caught just
            // inside the panel (a press on the editor's last point is the
            // editor's) and beside the editor, clear of the band, which is
            // a handle of its own.
            let grip = |band: egui::Rect| egui::pos2(band.right() + 2.0, band.top() - 30.0);
            // Towards the editor: a wider panel, its edge under the pointer.
            let to = grip(opened) - egui::vec2(30.0, 0.0);
            drag(&mut harness, grip(opened), to);
            let wider = band(&mut harness);
            assert!(
                (wider.right() - to.x).abs() <= 1.0,
                "the edge follows the pointer in {}: {opened:?} to {wider:?}",
                look.name
            );
            assert!(wider.width() < opened.width() - 1.0, "{}", look.name);
            // And away from it, past where it was: a narrower one.
            let to = grip(wider) + egui::vec2(70.0, 0.0);
            drag(&mut harness, grip(wider), to);
            let dragged = band(&mut harness);
            assert!(
                (dragged.right() - to.x).abs() <= 1.0,
                "and back out in {}: {wider:?} to {dragged:?}",
                look.name
            );
            assert!(dragged.width() > opened.width() + 1.0, "{}", look.name);
            // It stays there.
            harness.settle();
            assert_eq!(band(&mut harness), dragged, "{}", look.name);
            // A narrow window cuts the panel down: the editor loses less
            // than the window does.
            let narrower = 500.0;
            harness.size.x -= narrower;
            let cut = band(&mut harness).width();
            assert!(
                cut > dragged.width() - narrower + 1.0,
                "{cut} in {}",
                look.name
            );
            // With room again the panel is as wide as it was dragged to,
            // not as it opened.
            harness.size.x += narrower;
            harness.settle();
            let back = band(&mut harness).width();
            assert!(
                (back - dragged.width()).abs() < 1.0,
                "{back} for {} in {}",
                dragged.width(),
                look.name
            );
        }
    }

    #[test]
    fn a_window_resized_while_the_edge_is_held_keeps_the_wanted_width() {
        let mut harness = Harness::new();
        harness.set_look(crate::theme::Look::omarchy());
        let tab = harness.connect_fake();
        with_sql_result(&mut harness, tab, 3);
        harness.click("Row 2");
        let wide = band(&mut harness).width();
        // A window that cuts the panel down.
        harness.size.x = 1000.0;
        let cut = band(&mut harness);
        // The edge is held and pulled well into the editor: the panel is
        // as wide as it may be already, and stays.
        let grip = egui::pos2(cut.right() + 2.0, cut.top() - 30.0);
        let button = |pressed| egui::Event::PointerButton {
            pos: grip,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: Modifiers::NONE,
        };
        harness.frame(vec![egui::Event::PointerMoved(grip)]);
        harness.frame(vec![button(true)]);
        let pulled = grip - egui::vec2(150.0, 0.0);
        harness.frame(vec![egui::Event::PointerMoved(pulled)]);
        // The window gets narrower still while it is held, the pointer
        // still past where the edge may go.
        harness.size.x = 900.0;
        harness.frame(Vec::new());
        harness.frame(vec![egui::Event::PointerButton {
            pos: pulled,
            button: egui::PointerButton::Primary,
            pressed: false,
            modifiers: Modifiers::NONE,
        }]);
        harness.settle();
        // With room again the panel is as wide as it opened: no width a
        // small window cut it to was taken for a dragged one.
        harness.size.x = 1280.0;
        harness.settle();
        let back = band(&mut harness).width();
        assert!((back - wide).abs() < 1.0, "{back} for {wide}");
    }

    #[test]
    fn the_row_panel_gives_way_in_the_smallest_window() {
        for look in crate::theme::Look::ALL {
            let mut harness = Harness::with_size(egui::vec2(720.0, 480.0));
            harness.set_look(look);
            let tab = harness.connect_fake();
            with_sql_result(&mut harness, tab, 3);
            let room = band(&mut harness).width();
            harness.click("Row 2");
            assert!(panel_shows(&mut harness), "{}", look.name);
            // The panel is under its usual least width here, and the
            // editor still has its half.
            let editor = band(&mut harness).width();
            assert!(editor < room, "{}", look.name);
            assert!(
                editor >= room / 2.0 - 1.0,
                "{editor} of {room} in {}",
                look.name
            );
        }
    }

    #[test]
    fn a_sliver_of_a_row_panel_does_not_panic() {
        for look in crate::theme::Look::ALL {
            for sql in [false, true] {
                let mut harness = Harness::new();
                harness.set_look(look);
                if sql {
                    let tab = harness.connect_fake();
                    with_sql_result(&mut harness, tab, 3);
                } else {
                    let tab = with_page(&mut harness);
                    focus_grid(&mut harness, tab);
                }
                harness.click("Row 1");
                assert!(panel_shows(&mut harness), "{}", look.name);
                // Narrower and narrower, down to no room at all: drawing
                // the panel only has to go on.
                for width in [380.0, 300.0, 120.0, 40.0] {
                    harness.size.x = width;
                    harness.settle();
                }
            }
        }
    }

    #[test]
    fn the_terminal_footer_fits_a_narrow_row_panel() {
        let mut harness = Harness::new();
        harness.set_look(crate::theme::Look::omarchy());
        let tab = with_page(&mut harness);
        focus_grid(&mut harness, tab);
        harness.click("Row 1");
        let note = "read-only connection · editing arrives in a later version";
        let pieces = |harness: &mut Harness| -> Vec<String> {
            harness.settle();
            let painted = harness.painted.iter();
            painted.map(|(text, _)| text.clone()).collect()
        };
        // With room for them, the note and each cell's words are whole (a
        // cell's key and its word are one painted piece).
        let wide = pieces(&mut harness);
        assert!(wide.iter().any(|piece| piece == note));
        assert!(wide.iter().any(|piece| piece == "yy p duplicate"));
        // A narrower window cuts the panel: the note is cut to fit it.
        harness.size.x = 1000.0;
        let narrow = pieces(&mut harness);
        assert!(!narrow.iter().any(|piece| piece == note), "{narrow:?}");
        assert!(
            narrow
                .iter()
                .any(|piece| piece.starts_with("read-only connection") && piece.ends_with('…')),
            "{narrow:?}"
        );
        // And a cell too narrow for its words keeps its key alone.
        assert!(narrow.iter().any(|piece| piece == "yy p"), "{narrow:?}");
        assert!(
            !narrow.iter().any(|piece| piece.contains("duplicate")),
            "{narrow:?}"
        );
    }

    #[test]
    fn only_a_read_only_connection_carries_the_mark() {
        for look in crate::theme::Look::ALL {
            let mut harness = Harness::new();
            harness.set_look(look);
            let tab = with_page(&mut harness);
            focus_grid(&mut harness, tab);
            harness.click("Row 1");
            // What says "read-only": by name, and as the frame painted it
            // (the status line's tag has no name of its own).
            let marks = |harness: &mut Harness| -> (Vec<String>, Vec<String>) {
                let tree = harness.settle();
                let said = |text: &String| text.to_lowercase().contains("read-only");
                let named = crate::testing::labels(&tree).into_iter().filter(said);
                let painted = harness.painted.iter().map(|(text, _)| text.clone());
                let mut painted: Vec<String> = painted.filter(said).collect();
                painted.sort();
                (named.collect(), painted)
            };
            // The fixture connection is read-only.
            let (named, painted) = marks(&mut harness);
            if look.terminal {
                assert_eq!(named, ["read-only"], "{}", look.name);
                // The bar's tag, the status line's, and the row panel's
                // note.
                assert_eq!(
                    painted,
                    [
                        "read-only",
                        "read-only",
                        "read-only connection · editing arrives in a later version"
                    ],
                    "{}",
                    look.name
                );
            } else {
                // The bar's pill and the footer.
                let marks = ["Read-only", "1 row selected · read-only"];
                assert_eq!(named, marks, "{}", look.name);
            }
            harness.app.workspace_mut(tab).unwrap().access = tabletist_db::Access::Writable;
            let (named, painted) = marks(&mut harness);
            assert_eq!(named, Vec::<String>::new(), "{}", look.name);
            assert_eq!(painted, Vec::<String>::new(), "{}", look.name);
            if look.terminal {
                // The row panel's note says the rest of what it said.
                let note = "editing arrives in a later version";
                let mut pieces = harness.painted.iter();
                assert!(pieces.any(|(text, _)| text == note), "{}", look.name);
            }
        }
    }

    #[test]
    fn the_terminal_header_names_its_row_in_the_smallest_window() {
        // What the last frame painted. The header's hint is one piece, its
        // keys and its words together.
        let pieces = |harness: &mut Harness| -> Vec<String> {
            harness.settle();
            let painted = harness.painted.iter();
            painted.map(|(text, _)| text.clone()).collect()
        };
        for sql in [false, true] {
            let mut harness = Harness::new();
            harness.set_look(crate::theme::Look::omarchy());
            if sql {
                let tab = harness.connect_fake();
                with_sql_result(&mut harness, tab, 3);
            } else {
                let tab = with_page(&mut harness);
                focus_grid(&mut harness, tab);
            }
            harness.click("Row 1");
            // With room, the hint says what its keys do.
            let wide = pieces(&mut harness);
            assert!(
                wide.iter().any(|piece| piece == "[ ] prev/next"),
                "sql {sql}: {wide:?}"
            );
            // In the smallest window the hint gives way before the row's
            // name: its keys alone, and the name is no bare ellipsis.
            harness.size = egui::vec2(720.0, 480.0);
            let small = pieces(&mut harness);
            assert!(panel_shows(&mut harness), "sql {sql}");
            assert!(
                !small.iter().any(|piece| piece == "…"),
                "sql {sql}: {small:?}"
            );
            assert!(
                !small.iter().any(|piece| piece.contains("prev/next")),
                "sql {sql}: {small:?}"
            );
            assert!(
                small.iter().any(|piece| piece == "[ ]"),
                "sql {sql}: {small:?}"
            );
        }
    }

    /// Presses the pointer at `from`, moves it to `to` and lets go.
    fn drag(harness: &mut Harness, from: egui::Pos2, to: egui::Pos2) {
        let button = |pos, pressed| egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: Modifiers::NONE,
        };
        harness.frame(vec![egui::Event::PointerMoved(from)]);
        harness.frame(vec![button(from, true)]);
        harness.frame(vec![egui::Event::PointerMoved(to)]);
        harness.frame(vec![button(to, false)]);
        harness.settle();
    }

    /// Where the band between a SQL editor and its results is.
    fn band(harness: &mut Harness) -> egui::Rect {
        let tree = harness.settle();
        crate::testing::bounds(&tree, "Resize the editor", egui::accesskit::Role::Unknown)
            .expect("the splitter")
    }

    #[test]
    fn dragging_the_band_changes_the_editors_share() {
        for look in crate::theme::Look::ALL {
            let (mut harness, tab) = sql_harness(look);
            assert_eq!(active_sql(&harness, tab).split, 0.45);
            let before = band(&mut harness);
            let at = before.center();
            drag(&mut harness, at, at + egui::vec2(0.0, 40.0));
            let after = band(&mut harness);
            assert!(
                (after.top() - before.top() - 40.0).abs() <= 1.0,
                "the band follows the pointer in {}: {before:?} to {after:?}",
                look.name
            );
            assert!(active_sql(&harness, tab).split > 0.45, "{}", look.name);
            // Up again, past where it was.
            let at = after.center();
            drag(&mut harness, at, at - egui::vec2(0.0, 90.0));
            assert!(active_sql(&harness, tab).split < 0.45, "{}", look.name);
            assert!(band(&mut harness).top() < before.top());
        }
    }

    #[test]
    fn the_band_is_no_stop_for_the_tab_key() {
        let (mut harness, _tab) = sql_harness(crate::theme::Look::macos());
        for _ in 0..40 {
            harness.press(Key::Tab, Modifiers::NONE);
            assert_ne!(focused_name(&harness.settle()), "Resize the editor");
        }
    }

    #[test]
    fn the_editor_and_its_results_keep_their_least_height() {
        for look in crate::theme::Look::ALL {
            let (mut harness, tab) = sql_harness(look);
            let run =
                crate::testing::bounds(&harness.settle(), "Run", egui::accesskit::Role::Button)
                    .unwrap();
            // Far past the toolbar: the editor stays 80 tall under it.
            let at = band(&mut harness).center();
            drag(&mut harness, at, egui::pos2(at.x, 0.0));
            let top = band(&mut harness).top() - run.bottom();
            assert!((80.0..100.0).contains(&top), "{top} in {}", look.name);
            assert!(active_sql(&harness, tab).split > 0.0);
            // Far past the window's end: the results stay 80 tall over the
            // footer (or the terminal's status line).
            let at = band(&mut harness).center();
            drag(&mut harness, at, egui::pos2(at.x, 5_000.0));
            let rest = harness.size.y - band(&mut harness).bottom();
            assert!((80.0..120.0).contains(&rest), "{rest} in {}", look.name);
            assert!(active_sql(&harness, tab).split < 1.0);
        }
    }

    #[test]
    fn the_split_survives_other_tabs_and_a_resized_window() {
        for look in crate::theme::Look::ALL {
            let (mut harness, tab) = sql_harness(look);
            let at = band(&mut harness).center();
            drag(&mut harness, at, at + egui::vec2(0.0, 60.0));
            let split = active_sql(&harness, tab).split;
            let place = band(&mut harness);
            assert!(split > 0.45);
            // A table, another editor (which starts at the default), back.
            harness.click("users");
            harness.answer_rows(crate::testing::page(5, false));
            harness.app.apply(crate::model::Action::NewSqlTab(tab));
            assert_eq!(active_sql(&harness, tab).split, 0.45);
            assert!(band(&mut harness).top() < place.top());
            harness.click("Query 1 tab");
            assert_eq!(active_sql(&harness, tab).split, split, "{}", look.name);
            assert_eq!(band(&mut harness), place, "{}", look.name);
            // A taller window gives the editor its share of the new room.
            harness.size.y += 200.0;
            let taller = band(&mut harness);
            assert_eq!(active_sql(&harness, tab).split, split);
            let grown = taller.top() - place.top();
            assert!(
                (grown - split * 200.0).abs() <= 1.0,
                "the editor grew {grown} of 200 at {split} in {}",
                look.name
            );
            // And back: the window passing through a size changes nothing.
            harness.size.y -= 200.0;
            assert_eq!(band(&mut harness), place, "{}", look.name);
        }
    }

    /// Drags the sidebar's edge as far right as it goes.
    fn widen_sidebar(harness: &mut Harness) {
        let filter = |harness: &mut Harness| {
            crate::testing::bounds(
                &harness.settle(),
                "Filter",
                egui::accesskit::Role::TextInput,
            )
            .expect("the sidebar's filter")
        };
        let before = filter(harness);
        // The edge is a few points right of the filter field.
        for step in 0..40 {
            let at = egui::pos2(before.right() + step as f32, before.bottom() + 150.0);
            drag(harness, at, at + egui::vec2(600.0, 0.0));
            if filter(harness).right() > before.right() + 100.0 {
                return;
            }
        }
        panic!("the sidebar did not widen");
    }

    #[test]
    fn the_menus_stay_in_reach_in_the_smallest_window_beside_the_widest_sidebar() {
        for look in crate::theme::Look::ALL {
            let size = egui::vec2(720.0, 480.0);
            let mut harness = Harness::with_size(size);
            harness.set_look(look);
            let tab = harness.connect_fake();
            harness.app.apply(crate::model::Action::NewSqlTab(tab));
            widen_sidebar(&mut harness);
            let tree = harness.settle();
            let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, size);
            let controls: Vec<(&str, egui::Rect)> = [
                ("Run", egui::accesskit::Role::Button),
                ("Run all", egui::accesskit::Role::Button),
                ("Limit", egui::accesskit::Role::ComboBox),
                ("Timeout", egui::accesskit::Role::ComboBox),
            ]
            .into_iter()
            .map(|(label, role)| {
                let rect = crate::testing::bounds(&tree, label, role)
                    .unwrap_or_else(|| panic!("{label} missing in {}", look.name));
                assert!(
                    screen.contains_rect(rect),
                    "{label} at {rect:?} is off screen in {}",
                    look.name
                );
                (label, rect)
            })
            .collect();
            // Run sits right of the sidebar, so the sidebar is as wide as
            // it goes and the toolbar as narrow.
            assert!(controls[0].1.left() > 400.0, "{:?}", controls[0].1);
            for (index, (label, rect)) in controls.iter().enumerate() {
                for (other, other_rect) in &controls[index + 1..] {
                    assert!(
                        !rect.intersects(*other_rect),
                        "{label} overlaps {other} in {}",
                        look.name
                    );
                }
            }
            // The menus read short there and keep their whole value for
            // screen readers; they still open.
            let short = if look.terminal { "1000" } else { "1,000" };
            assert!(painted(&harness, short), "{short} in {}", look.name);
            harness.click("Limit");
            harness.click("Limit 100");
            assert_eq!(active_sql(&harness, tab).limit, 100, "{}", look.name);
        }
    }

    #[test]
    fn the_toolbar_gives_way_in_order_and_shortens_its_menus_last() {
        // The keys go first: the help and the terminal's status line say
        // them too, while the menus' words are all that say what "1,000"
        // and "30 s" are. So the labels read in full until only they are
        // left to give way.
        for look in crate::theme::Look::ALL {
            let (mut harness, _tab) = sql_harness(look);
            let (keys, note, full, short) = if look.terminal {
                ("ctrl+enter", "read-only transaction", "limit 1000", "1000")
            } else if look == crate::theme::Look::macos() {
                ("⌘↩", "Read-only transaction", "Limit 1,000", "1,000")
            } else {
                (
                    "Ctrl+Enter",
                    "Read-only transaction",
                    "Limit 1,000",
                    "1,000",
                )
            };
            // Format's key, which goes when the run buttons' keys do.
            let format_keys = if look == crate::theme::Look::macos() {
                "⇧⌘F"
            } else {
                "Ctrl+Shift+F"
            };
            // Every state the toolbar passes through as the window narrows,
            // in the order it meets them.
            let mut states = Vec::new();
            for width in (500..=1600).rev().step_by(10) {
                harness.size.x = width as f32;
                let tree = harness.settle();
                let has = |role, name: &str| crate::testing::node(&tree, name, role).is_some();
                let state = [
                    painted(&harness, keys),
                    painted(&harness, note),
                    // The terminal has no Format button.
                    look.terminal || has(egui::accesskit::Role::Button, "Format"),
                    painted(&harness, full),
                    // The terminal's title, which the others do not have.
                    !look.terminal || has(egui::accesskit::Role::Label, "query 1"),
                    has(egui::accesskit::Role::ComboBox, "Limit"),
                ];
                assert!(
                    has(egui::accesskit::Role::Button, "Run")
                        && has(egui::accesskit::Role::Button, "Run all"),
                    "the run buttons at {width} in {}",
                    look.name
                );
                assert_eq!(
                    painted(&harness, format_keys),
                    state[0] && !look.terminal,
                    "{format_keys} at {width} in {}",
                    look.name
                );
                // A menu reads in full or short, never neither.
                assert_eq!(
                    state[5] && !state[3],
                    painted(&harness, short),
                    "{short} at {width} in {}",
                    look.name
                );
                if states.last() != Some(&state) {
                    states.push(state);
                }
            }
            // keys, note, Format, full labels, title, menus
            let mut expected = vec![
                [true, true, true, true, true, true],
                [false, true, true, true, true, true],
                [false, false, true, true, true, true],
            ];
            if look.terminal {
                // The title goes before the labels shorten: the tab says
                // it too.
                expected.push([false, false, true, true, false, true]);
            } else {
                // Format goes before them: its key formats too.
                expected.push([false, false, false, true, true, true]);
            }
            let (format, title) = (look.terminal, !look.terminal);
            expected.push([false, false, format, false, title, true]);
            expected.push([false, false, format, false, title, false]);
            assert_eq!(states, expected, "{}", look.name);
        }
    }

    #[test]
    fn a_toolbar_too_narrow_for_its_menus_keeps_run_and_run_all() {
        for look in crate::theme::Look::ALL {
            // Narrower than the window gets, beside the widest sidebar.
            let size = egui::vec2(640.0, 480.0);
            let mut harness = Harness::with_size(size);
            harness.set_look(look);
            let tab = harness.connect_fake();
            harness.app.apply(crate::model::Action::NewSqlTab(tab));
            widen_sidebar(&mut harness);
            let tree = harness.settle();
            let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, size);
            let bounds = |label: &str| {
                crate::testing::bounds(&tree, label, egui::accesskit::Role::Button)
                    .unwrap_or_else(|| panic!("{label} missing in {}", look.name))
            };
            let (run, all) = (bounds("Run"), bounds("Run all"));
            assert!(screen.contains_rect(run) && screen.contains_rect(all));
            assert!(!run.intersects(all), "{}", look.name);
            for menu in ["Limit", "Timeout"] {
                let menu = crate::testing::bounds(&tree, menu, egui::accesskit::Role::ComboBox);
                assert!(
                    menu.is_none_or(|menu| !menu.intersects(run) && !menu.intersects(all)),
                    "{menu:?} under a run button in {}",
                    look.name
                );
            }
        }
    }

    #[test]
    fn the_footer_tells_the_cursor_the_server_and_the_last_run() {
        for look in [crate::theme::Look::standard(), crate::theme::Look::macos()] {
            let (mut harness, tab) = sql_harness(look);
            harness.app.workspace_mut(tab).unwrap().server_version.value =
                Some("SQLite 3.46.0".into());
            harness.settle();
            type_text(&mut harness, "SELECT 1;\nSELECT 2");
            for _ in 0..6 {
                harness.press(Key::ArrowLeft, Modifiers::NONE);
            }
            assert!(harness.has("Ln 2, Col 3"), "{}", look.name);
            assert!(harness.has("SQLite 3.46.0"), "{}", look.name);
            // Nothing ran yet, so nothing was rolled back.
            assert!(!harness.has("Read-only transaction · rolled back"));
            let id = harness.app.workspace(tab).unwrap().active_tab.unwrap();
            harness.app.apply(crate::model::Action::RunSql {
                tab,
                sql_tab: id,
                all: false,
            });
            // Nor while the run is on its way.
            assert!(!harness.has("Read-only transaction · rolled back"));
            harness.answer_sql(
                Ok(crate::testing::script_outcome(vec![
                    crate::testing::rows_outcome(3),
                ])),
                None,
            );
            assert!(harness.has("3 rows · 14 ms"), "{}", look.name);
            assert!(harness.has("Read-only transaction · rolled back"));
            // A run that failed as a whole ran nothing, and the rows of
            // the run before it are gone with it.
            harness.app.apply(crate::model::Action::RunSql {
                tab,
                sql_tab: id,
                all: false,
            });
            harness.answer_sql(Err(tabletist_db::Error::query("no such server")), None);
            assert!(!harness.has("3 rows · 14 ms"), "{}", look.name);
            assert!(!harness.has("Read-only transaction · rolled back"));
            // Nor do they come back with a run that is never answered
            // (its session went away), which leaves no error either.
            harness.app.apply(crate::model::Action::RunSql {
                tab,
                sql_tab: id,
                all: false,
            });
            let workspace = harness.app.workspace_mut(tab).unwrap();
            workspace.forget_session_requests();
            let sql = active_sql(&harness, tab);
            assert!(!sql.is_running() && sql.run.error.is_none());
            assert!(!harness.has("3 rows · 14 ms"), "{}", look.name);
            assert!(!harness.has("Read-only transaction · rolled back"));
        }
    }

    #[test]
    fn a_result_of_one_row_is_counted_as_one_row() {
        let mut harness = Harness::new();
        let tab = harness.connect_fake();
        with_sql_result(&mut harness, tab, 1);
        assert!(harness.has("1 row · 14 ms"));
    }

    #[test]
    fn the_terminal_status_line_is_the_editors_on_a_sql_tab() {
        let (mut harness, tab) = sql_harness(crate::theme::Look::omarchy());
        harness.settle();
        type_text(&mut harness, "SELECT 1;\nSELECT 2");
        for _ in 0..6 {
            harness.press(Key::ArrowLeft, Modifiers::NONE);
        }
        assert!(painted(&harness, "ln 2:3"), "{:?}", harness.painted);
        for hint in [
            "ctrl+enter run",
            "ctrl+shift+enter run all",
            "ctrl+. cancel",
        ] {
            assert!(painted(&harness, hint), "{hint}: {:?}", harness.painted);
        }
        // A table's keys do nothing here, so the line does not offer them.
        for hint in ["j/k row", "/ filter", "s structure", "e edit"] {
            assert!(!painted(&harness, hint), "{hint}");
        }
        let id = harness.app.workspace(tab).unwrap().active_tab.unwrap();
        harness.app.apply(crate::model::Action::RunSql {
            tab,
            sql_tab: id,
            all: false,
        });
        harness.answer_sql(
            Ok(crate::testing::script_outcome(vec![
                crate::testing::rows_outcome(3),
            ])),
            None,
        );
        harness.settle();
        assert!(
            painted(&harness, "ln 2:3 · 3 rows · 14 ms · rolled back"),
            "{:?}",
            harness.painted
        );
        // A table tab keeps its own line.
        harness.click("users");
        harness.answer_rows(crate::testing::page(5, false));
        harness.settle();
        assert!(painted(&harness, "j/k row"));
        assert!(!painted(&harness, "ctrl+enter run"));
    }

    #[test]
    fn a_sql_tab_is_named_as_the_look_names_things() {
        let (mut harness, _tab) = sql_harness(crate::theme::Look::omarchy());
        harness.settle();
        assert!(painted(&harness, "query 1"), "{:?}", harness.painted);
        assert!(!painted(&harness, "Query 1"));
        let (mut harness, _tab) = sql_harness(crate::theme::Look::macos());
        harness.settle();
        assert!(painted(&harness, "Query 1"), "{:?}", harness.painted);
    }

    fn selection(harness: &Harness, tab: crate::model::ConnTabId) -> Option<crate::model::CellPos> {
        harness
            .app
            .workspace(tab)
            .unwrap()
            .active_object_tab()
            .unwrap()
            .selection
    }

    #[test]
    fn clicking_a_row_fills_the_row_panel_and_its_copy_buttons_copy_full_values() {
        let mut harness = Harness::new();
        let tab = with_page(&mut harness);
        assert!(harness.has("Select a row to see its fields"));
        harness.click("Row 1");
        assert_eq!(
            selection(&harness, tab),
            Some(crate::model::CellPos { row: 0, col: 0 })
        );
        assert!(harness.has("Copy email"));
        harness.click("Copy meta");
        assert_eq!(harness.copied.as_deref(), Some(r#"{"plan":"pro"}"#));
    }

    #[test]
    fn the_row_panel_labels_each_field_with_its_type() {
        let mut harness = Harness::new();
        with_page(&mut harness);
        harness.click("Row 1");
        for label in ["id · INTEGER", "email · TEXT", "meta · JSON"] {
            assert!(harness.has(label), "{label}");
        }
    }

    #[test]
    fn the_row_panel_is_titled_by_its_row() {
        let mut harness = Harness::new();
        with_page(&mut harness);
        harness.click("Row 1");
        assert!(harness.has("Copy email"), "the panel shows the row");
        assert!(harness.has("Row 1"), "no key known yet, so its number");
    }

    #[test]
    fn arrow_keys_move_the_selection_and_copy_takes_the_cell_or_row() {
        let mut harness = Harness::new();
        let tab = with_page(&mut harness);
        focus_grid(&mut harness, tab);
        harness.press(Key::ArrowDown, Modifiers::NONE);
        assert_eq!(
            selection(&harness, tab),
            Some(crate::model::CellPos { row: 0, col: 0 })
        );
        harness.press(Key::ArrowDown, Modifiers::NONE);
        harness.press(Key::ArrowRight, Modifiers::NONE);
        assert_eq!(
            selection(&harness, tab),
            Some(crate::model::CellPos { row: 1, col: 1 })
        );
        harness.press(Key::End, Modifiers::NONE);
        assert_eq!(
            selection(&harness, tab),
            Some(crate::model::CellPos { row: 4, col: 1 })
        );
        harness.copy(false);
        assert_eq!(harness.copied.as_deref(), Some("user5@example.com"));
        harness.copy(true);
        assert_eq!(
            harness.copied.as_deref(),
            Some("5\tuser5@example.com\tNULL")
        );
    }

    #[test]
    fn space_and_ctrl_shift_r_toggle_the_row_panel() {
        let mut harness = Harness::new();
        let tab = with_page(&mut harness);
        focus_grid(&mut harness, tab);
        harness.press(Key::Space, Modifiers::NONE);
        assert!(!harness.app.workspace(tab).unwrap().row_panel);
        harness.press(Key::R, Modifiers::COMMAND | Modifiers::SHIFT);
        assert!(harness.app.workspace(tab).unwrap().row_panel);
    }

    #[test]
    fn object_tab_and_page_shortcuts_work() {
        let mut harness = Harness::new();
        let tab = harness.connect_fake();
        harness.app.apply(crate::model::Action::OpenObject {
            tab,
            object: tabletist_db::ObjectRef::new("main", "users"),
            kind: tabletist_db::ObjectKind::Table,
            pin: true,
        });
        harness.answer_rows(crate::testing::page(300, true));
        harness.press(Key::ArrowRight, Modifiers::COMMAND | Modifiers::ALT);
        match crate::testing::last_sent(&harness.app) {
            Command::FetchRows { query, .. } => assert_eq!(query.offset, 300),
            other => panic!("{other:?}"),
        }
        harness.press(Key::Period, Modifiers::COMMAND);
        assert!(matches!(
            crate::testing::last_sent(&harness.app),
            Command::Cancel { .. }
        ));
        harness.press(Key::R, Modifiers::COMMAND);
        assert!(matches!(
            crate::testing::last_sent(&harness.app),
            Command::FetchRows { .. }
        ));
        harness.press(Key::W, Modifiers::COMMAND);
        assert!(harness.app.workspace(tab).unwrap().tabs.is_empty());
        assert_eq!(
            harness.app.tabs.len(),
            1,
            "Cmd+W closes the object tab, not the connection"
        );
    }

    #[test]
    fn a_colour_value_shows_a_swatch_in_the_grid_and_the_row_panel() {
        let mut harness = Harness::new();
        harness.connect_fake();
        harness.click("users");
        let mut page = crate::testing::page(3, false);
        page.rows[0][1] = tabletist_db::Value::Text("#3a7bd5".into());
        page.rows[1][1] = tabletist_db::Value::Text("#3a7bd".into());
        page.rows[2][1] = tabletist_db::Value::Text("#e94f3780".into());
        harness.answer_rows(page);
        let opaque = egui::Color32::from_rgb(0x3a, 0x7b, 0xd5);
        let translucent = egui::Color32::from_rgba_unmultiplied(0xe9, 0x4f, 0x37, 0x80);
        let swatches = |harness: &Harness, color: egui::Color32| {
            harness
                .fills
                .iter()
                .filter(|(_, fill)| *fill == color)
                .count()
        };
        harness.settle();
        assert_eq!(swatches(&harness, opaque), 1, "the grid's cell");
        assert!(harness.painted_color("#3a7bd5").is_some(), "and its text");
        assert_eq!(swatches(&harness, translucent), 1, "alpha is a colour too");
        harness.click("Row 1");
        harness.settle();
        assert_eq!(swatches(&harness, opaque), 2, "and the row panel's field");
    }

    #[test]
    fn a_huge_single_line_value_is_collapsed_in_the_row_panel() {
        let mut harness = Harness::new();
        harness.connect_fake();
        harness.click("users");
        let mut page = crate::testing::page(1, false);
        page.rows[0][1] = tabletist_db::Value::Text("x".repeat(1_000_000).into());
        harness.answer_rows(page);
        harness.click("Row 1");
        let tree = harness.settle();
        assert!(
            crate::testing::labels(&tree)
                .iter()
                .any(|label| label.starts_with("Show all"))
        );
    }

    /// Opens users with `meta` set to `json` on the first row and selects it.
    fn with_json(harness: &mut Harness, json: &str) {
        harness.connect_fake();
        harness.click("users");
        let mut page = crate::testing::page(1, false);
        page.rows[0][2] = tabletist_db::Value::Text(json.into());
        harness.answer_rows(page);
        harness.click("Row 1");
    }

    #[test]
    fn json_in_the_row_panel_is_a_highlighted_tree_that_folds() {
        let mut harness = Harness::new();
        with_json(&mut harness, r#"{"plan":"pro","seats":[3,4]}"#);
        // Keys keep their order and every line is JSON, commas included.
        assert!(harness.has("{"));
        assert!(harness.has(r#""plan": "pro","#));
        assert!(harness.has(r#""seats": ["#));
        assert!(harness.has("3,"));
        assert!(harness.has("}"));
        let palette = harness.app.palette;
        assert_eq!(
            harness.painted_color(r#""plan": "pro","#),
            Some(palette.accent_hover),
            "keys in the strong accent"
        );
        assert_eq!(harness.painted_color("3,"), Some(palette.orange));
        harness.click("Collapse meta.seats");
        assert!(harness.has(r#""seats": [ 2 items ]"#));
        assert!(!harness.has("3,"));
        harness.click("Collapse meta");
        assert!(harness.has("{ 2 keys }"));
        harness.click("Copy meta");
        assert_eq!(
            harness.copied.as_deref(),
            Some(r#"{"plan":"pro","seats":[3,4]}"#)
        );
    }

    #[test]
    fn a_fold_toggle_keeps_the_keyboard_after_it_is_pressed() {
        let mut harness = Harness::new();
        with_json(&mut harness, r#"{"plan":"pro","seats":[3,4]}"#);
        focus(&mut harness, "Collapse meta", egui::accesskit::Role::Button);
        harness.press(Key::Enter, Modifiers::NONE);
        assert!(harness.has("{ 2 keys }"));
        assert_eq!(focused_name(&harness.settle()), "Expand meta");
        harness.press(Key::Enter, Modifiers::NONE);
        assert!(harness.has(r#""plan": "pro","#));
        assert_eq!(focused_name(&harness.settle()), "Collapse meta");
    }

    #[test]
    fn a_long_json_document_opens_its_top_level_and_expand_all_opens_the_rest() {
        let mut harness = Harness::new();
        let items = (0..50).map(|n| n.to_string()).collect::<Vec<_>>().join(",");
        with_json(
            &mut harness,
            &format!(r#"{{"ids":[{items}],"deep":{{"a":1}}}}"#),
        );
        assert!(harness.has(r#""ids": [ 50 items ],"#));
        assert!(harness.has(r#""deep": { 1 key }"#));
        harness.click("Expand all");
        assert!(harness.has("49"));
        assert!(harness.has(r#""a": 1"#));
        harness.click("Collapse all");
        assert!(harness.has("{ 2 keys }"));
    }

    #[test]
    fn json_in_a_text_column_is_a_tree_and_a_chip_and_other_text_stays_text() {
        let mut harness = Harness::new();
        harness.set_look(crate::theme::Look::macos());
        harness.connect_fake();
        harness.click("users");
        let mut page = crate::testing::page(2, false);
        page.rows[0][1] = tabletist_db::Value::Text(r#"{"plan":"pro","seats":[3,4]}"#.into());
        page.rows[1][1] = tabletist_db::Value::Text("[draft] not json]".into());
        harness.answer_rows(page);
        harness.settle();
        // The grid counts the document's keys, as it does for a JSON column.
        assert!(harness.painted_color("{ 2 }").is_some());
        harness.click("Row 1");
        assert!(harness.has("email · TEXT"));
        assert!(harness.has(r#""plan": "pro","#));
        harness.click("Collapse email.seats");
        assert!(harness.has(r#""seats": [ 2 items ]"#));
        harness.click("Copy email");
        assert_eq!(
            harness.copied.as_deref(),
            Some(r#"{"plan":"pro","seats":[3,4]}"#)
        );
        harness.click("Row 2");
        let tree = harness.settle();
        assert!(
            crate::testing::labels(&tree)
                .iter()
                .any(|label| label == "[draft] not json]")
        );
        assert!(!harness.has("Collapse email"));
    }

    #[test]
    fn invalid_json_in_a_json_column_is_shown_as_text() {
        let mut harness = Harness::new();
        with_json(&mut harness, "{not json");
        let tree = harness.settle();
        assert!(
            crate::testing::labels(&tree)
                .iter()
                .any(|label| label == "{not json")
        );
        assert!(!harness.has("Collapse meta"));
    }

    #[test]
    fn the_row_panel_formats_a_row_once_not_every_frame() {
        use crate::ui::format::FULL_TEXTS;
        let mut harness = Harness::new();
        harness.connect_fake();
        harness.click("users");
        let mut page = crate::testing::page(2, false);
        page.rows[0][1] = tabletist_db::Value::Text("x".repeat(1_000_000).into());
        harness.answer_rows(page);
        harness.click("Row 1");
        harness.settle();
        let before = FULL_TEXTS.with(std::cell::Cell::get);
        for step in 0..5 {
            let at = egui::pos2(900.0 + step as f32 * 10.0, 300.0);
            harness.frame(vec![egui::Event::PointerMoved(at)]);
        }
        assert_eq!(FULL_TEXTS.with(std::cell::Cell::get), before);
        // Another row is formatted, once.
        harness.click("Row 2");
        let columns = crate::testing::page(1, false).columns.len();
        let after = FULL_TEXTS.with(std::cell::Cell::get);
        assert_eq!(after - before, columns);
        harness.settle();
        assert_eq!(FULL_TEXTS.with(std::cell::Cell::get), after);
    }

    #[test]
    fn the_structure_view_lists_columns_indexes_and_keys() {
        let mut harness = Harness::new();
        let tab = with_page(&mut harness);
        harness.click("Structure");
        let structure = tabletist_db::Structure {
            columns: vec![tabletist_db::ColumnInfo {
                name: "email".into(),
                type_name: "TEXT".into(),
                nullable: false,
                default: None,
                comment: None,
                allowed_values: None,
                generated: false,
            }],
            primary_key: vec!["id".into()],
            indexes: vec![tabletist_db::IndexInfo {
                name: "users_email_idx".into(),
                columns: vec!["email".into()],
                key_columns: Some(vec!["email".into()]),
                unique: true,
                primary: false,
                method: None,
                partial: false,
            }],
            foreign_keys: Vec::new(),
        };
        harness.answer_structure(structure);
        assert!(harness.has("Columns"));
        assert!(harness.has("users_email_idx"));
        assert!(harness.has("No foreign keys"));
        assert!(harness.app.workspace(tab).is_some());
    }

    #[test]
    fn the_structure_view_is_inset_from_the_panel_edge() {
        let mut harness = Harness::new();
        with_page(&mut harness);
        harness.click("Structure");
        harness.answer_structure(tabletist_db::Structure::default());
        let tree = harness.settle();
        let left = |text: &str| {
            tree.nodes
                .iter()
                .find(|(_, node)| node.label().or_else(|| node.value()) == Some(text))
                .and_then(|(_, node)| node.bounds())
                .unwrap_or_else(|| panic!("{text} is shown"))
                .x0
        };
        // The object's tab starts at the content's left edge.
        assert!(
            left("Columns") >= left("users tab") + 8.0,
            "Columns at {}, the content at {}",
            left("Columns"),
            left("users tab")
        );
    }

    #[test]
    fn shortcuts_are_ignored_while_the_dialog_is_open() {
        let mut harness = Harness::new();
        harness.connect_fake();
        harness.app.apply(crate::model::Action::ShowConnections);
        harness.press(Key::N, Modifiers::COMMAND);
        if let Some(crate::model::Dialog::Connection(form)) = &mut harness.app.dialog {
            form.name = "typed".into();
        }
        harness.press(Key::W, Modifiers::COMMAND | Modifiers::SHIFT);
        harness.press(Key::O, Modifiers::COMMAND);
        harness.press(Key::N, Modifiers::COMMAND);
        assert_eq!(
            harness.app.tabs.len(),
            2,
            "no tab opened or closed behind the dialog"
        );
        match &harness.app.dialog {
            Some(crate::model::Dialog::Connection(form)) => assert_eq!(form.name, "typed"),
            _ => panic!("the connection dialog closed"),
        }
    }

    #[test]
    fn escape_with_a_list_open_keeps_the_dialog() {
        let mut harness = Harness::new();
        harness.press(Key::N, Modifiers::COMMAND);
        harness.click("PostgreSQL");
        let tree = harness.settle();
        let combo = tree
            .nodes
            .iter()
            .find(|(_, node)| node.role() == egui::accesskit::Role::ComboBox)
            .map(|(id, _)| *id)
            .expect("the SSL mode list");
        harness.frame(vec![egui::Event::AccessKitActionRequest(
            egui::accesskit::ActionRequest {
                target_tree: egui::accesskit::TreeId::ROOT,
                target_node: combo,
                action: egui::accesskit::Action::Click,
                data: None,
            },
        )]);
        harness.press(Key::Escape, Modifiers::NONE);
        assert!(
            harness.app.dialog.is_some(),
            "Escape closes only the open list"
        );
        harness.press(Key::Escape, Modifiers::NONE);
        assert!(harness.app.dialog.is_none());
    }

    fn form_mut(harness: &mut Harness) -> &mut crate::model::ConnectionForm {
        match &mut harness.app.dialog {
            Some(crate::model::Dialog::Connection(form)) => form,
            _ => panic!("the connection dialog is open"),
        }
    }

    /// The open connection dialog's form.
    fn form(harness: &Harness) -> &crate::model::ConnectionForm {
        match &harness.app.dialog {
            Some(crate::model::Dialog::Connection(form)) => form,
            _ => panic!("the connection dialog is open"),
        }
    }

    #[test]
    fn a_new_connection_starts_in_name_and_escape_closes_it_while_typing() {
        let mut harness = Harness::new();
        harness.press(Key::N, Modifiers::COMMAND);
        harness.frame(vec![egui::Event::Text("Staging".into())]);
        harness.settle();
        assert_eq!(form(&harness).name, "Staging", "typing goes to Name");
        harness.press(Key::Escape, Modifiers::NONE);
        assert!(
            harness.app.dialog.is_none(),
            "one Escape closes it, as Cancel"
        );
    }

    #[test]
    fn the_connection_dialog_keeps_its_place_as_it_grows() {
        let mut harness = Harness::with_size(egui::vec2(1280.0, 900.0));
        harness.press(Key::N, Modifiers::COMMAND);
        let name = |harness: &mut Harness| {
            let tree = harness.settle();
            tree.nodes
                .iter()
                .find(|(_, node)| node.label().or_else(|| node.value()) == Some("Name"))
                .and_then(|(_, node)| node.bounds())
                .expect("the Name label")
                .y0
        };
        let before = name(&mut harness);
        // SQLite's short form grows by PostgreSQL's fields and the TLS
        // warning a remote host brings.
        harness.click("PostgreSQL");
        if let Some(crate::model::Dialog::Connection(form)) = &mut harness.app.dialog {
            form.host = "db.example.com".into();
        }
        assert_eq!(
            name(&mut harness),
            before,
            "the fields stay under the pointer"
        );
    }

    #[test]
    fn choosing_an_environment_sets_the_connections() {
        let mut harness = Harness::new();
        harness.press(Key::N, Modifiers::COMMAND);
        harness.click("Production");
        let form = form(&harness);
        assert_eq!(form.environment(), crate::env::Environment::Production);
        assert!(form.read_only(), "production is read-only by default");
        // The environments are a radio group, named as the dialog names
        // them: the one chosen is the one checked.
        let tree = harness.settle();
        let names = ["Local", "Dev", "Staging", "Production", "None"];
        let selected: Vec<_> = crate::env::Environment::ALL
            .iter()
            .zip(names)
            .filter(|(_, name)| {
                let id = crate::testing::node(&tree, name, egui::accesskit::Role::RadioButton);
                assert!(id.is_some(), "{name} is offered");
                tree.nodes.iter().any(|(node, data)| {
                    Some(*node) == id && data.toggled() == Some(egui::accesskit::Toggled::True)
                })
            })
            .map(|(env, _)| env)
            .collect();
        assert_eq!(selected, [&crate::env::Environment::Production]);
    }

    #[test]
    fn a_new_connections_environment_follows_its_host_until_chosen() {
        let mut harness = Harness::new();
        harness.press(Key::N, Modifiers::COMMAND);
        harness.click("PostgreSQL");
        assert_eq!(form(&harness).environment(), crate::env::Environment::Local);
        form_mut(&mut harness).host = "db.example.com".into();
        assert_eq!(form(&harness).environment(), crate::env::Environment::None);
        harness.click("Staging");
        form_mut(&mut harness).host = "localhost".into();
        assert_eq!(
            form(&harness).environment(),
            crate::env::Environment::Staging,
            "a chosen environment stays"
        );
    }

    #[test]
    fn the_read_only_box_follows_the_environment_until_it_is_set() {
        // The sheet says it under the box; the terminal look after it.
        for (look, said) in [
            (
                crate::theme::Look::standard(),
                "Blocks every write from this app. On by default for production; turn off to edit.",
            ),
            (
                crate::theme::Look::macos(),
                "Blocks every write from this app. On by default for production; turn off to edit.",
            ),
            (crate::theme::Look::omarchy(), "· default for production"),
        ] {
            let mut harness = Harness::new();
            harness.set_look(look);
            harness.press(Key::N, Modifiers::COMMAND);
            assert!(harness.has(said), "{}", look.name);
            let read_only = |harness: &mut Harness| {
                let tree = harness.settle();
                let (_, node) = tree
                    .nodes
                    .iter()
                    .find(|(_, node)| {
                        node.role() == egui::accesskit::Role::CheckBox
                            && node.label() == Some("Open read-only")
                    })
                    .expect("the read-only box");
                assert!(!node.is_disabled(), "{}", look.name);
                node.toggled() == Some(egui::accesskit::Toggled::True)
            };
            let click_box = |harness: &mut Harness| {
                let tree = harness.settle();
                let place = crate::testing::bounds(
                    &tree,
                    "Open read-only",
                    egui::accesskit::Role::CheckBox,
                )
                .expect("the read-only box");
                click_at(harness, place.center());
            };
            // A new connection is not production: writable, nothing set.
            assert!(!read_only(&mut harness), "{}", look.name);
            // The terminal look names its choices in lower case.
            harness.click(&look.label("Production"));
            assert!(read_only(&mut harness), "{}", look.name);
            assert_eq!(form(&harness).read_only, None, "the default, not a choice");
            // The box is the user's from the first click. Clicked by its
            // role: the sheet's title beside it has the same name.
            click_box(&mut harness);
            assert!(!read_only(&mut harness), "{}", look.name);
            assert_eq!(form(&harness).read_only, Some(false), "{}", look.name);
            click_box(&mut harness);
            assert_eq!(form(&harness).read_only, Some(true), "{}", look.name);
            // The keyboard has it too: Tab to the box, and Space toggles.
            tab_to(&mut harness, "Open read-only");
            harness.press(Key::Space, Modifiers::NONE);
            assert_eq!(form(&harness).read_only, Some(false), "{}", look.name);
            harness.press(Key::Space, Modifiers::NONE);
            assert_eq!(form(&harness).read_only, Some(true), "{}", look.name);
            if !look.terminal {
                // The sheet's title beside the box toggles it as the box
                // does (the terminal's box is its whole line).
                let tree = harness.settle();
                let title =
                    crate::testing::bounds(&tree, "Open read-only", egui::accesskit::Role::Label)
                        .expect("the title");
                click_at(&mut harness, title.center());
                assert_eq!(form(&harness).read_only, Some(false), "{}", look.name);
                assert!(!read_only(&mut harness), "{}", look.name);
            }
            // For all that there is one box of the name, and one Tab stop.
            let mut boxes = std::collections::HashSet::new();
            let mut stops = std::collections::HashSet::new();
            for _ in 0..80 {
                harness.press(Key::Tab, Modifiers::NONE);
                let tree = harness.settle();
                if focused_name(&tree) == "Open read-only" {
                    stops.insert(tree.focus);
                }
                boxes.extend(tree.nodes.iter().filter_map(|(id, node)| {
                    (node.role() == egui::accesskit::Role::CheckBox
                        && node.label() == Some("Open read-only"))
                    .then_some(*id)
                }));
            }
            assert_eq!((boxes.len(), stops.len()), (1, 1), "{}", look.name);
            // And what a connection was saved with comes back as it was.
            let mut harness = Harness::new();
            harness.set_look(look);
            let id = add_saved(&mut harness, "Shop");
            let mut saved = harness.app.connections.get(&id).unwrap().clone();
            saved.read_only = Some(false);
            harness.app.connections.upsert(saved);
            harness
                .app
                .apply(crate::model::Action::EditConnection(id.clone()));
            // The dialog is measured, unseen, before it is shown.
            harness.settle();
            assert!(!read_only(&mut harness), "{}", look.name);
            harness.click("Save");
            assert!(harness.app.dialog.is_none(), "{}", look.name);
            assert_eq!(
                harness.app.connections.get(&id).unwrap().read_only,
                Some(false),
                "{}",
                look.name
            );
        }
    }

    /// Tabs until the widget named `name` has the keyboard.
    fn tab_to(harness: &mut Harness, name: &str) {
        for _ in 0..80 {
            harness.press(Key::Tab, Modifiers::NONE);
            if focused_name(&harness.settle()) == name {
                return;
            }
        }
        panic!("{name} is no Tab stop");
    }

    #[test]
    fn a_terminal_check_has_one_ring_and_only_from_the_keyboard() {
        let mut harness = Harness::new();
        harness.set_look(crate::theme::Look::omarchy());
        harness.press(Key::N, Modifiers::COMMAND);
        harness.click("postgresql");
        let accent = harness.app.palette.accent;
        // The accent outlines round the check named `name`.
        let rings = |harness: &mut Harness, name: &str| {
            let tree = harness.settle();
            let place = crate::testing::bounds(&tree, name, egui::accesskit::Role::CheckBox)
                .expect("the check");
            let outlines = harness.outlines.iter();
            outlines
                .filter(|(rect, stroke)| {
                    stroke.color == accent
                        && rect.expand(1.0).contains_rect(place)
                        && place.expand(8.0).contains_rect(*rect)
                })
                .count()
        };
        for name in ["Keyring", "Connect through SSH tunnel", "Open read-only"] {
            tab_to(&mut harness, name);
            assert_eq!(rings(&mut harness, name), 1, "{name}");
        }
        // The pointer gives a check the keyboard too, and no ring.
        let tree = harness.settle();
        let name = "Open read-only";
        let place = crate::testing::bounds(&tree, name, egui::accesskit::Role::CheckBox)
            .expect("the read-only box");
        click_at(&mut harness, place.center());
        assert_eq!(focused_name(&harness.settle()), name);
        assert_eq!(rings(&mut harness, name), 0);
    }

    #[test]
    fn the_url_field_suggests_the_chosen_drivers_url() {
        let mut harness = Harness::new();
        harness.press(Key::N, Modifiers::COMMAND);
        let placeholders = |harness: &mut Harness| -> Vec<String> {
            harness
                .settle()
                .nodes
                .iter()
                .filter_map(|(_, node)| node.placeholder().map(str::to_owned))
                .collect()
        };
        harness.click("URL");
        assert!(placeholders(&mut harness).contains(&"sqlite:///path/to/file.db".to_owned()));
        harness.click("Parameters");
        harness.click("PostgreSQL");
        harness.click("URL");
        assert!(placeholders(&mut harness).contains(&"postgres://user@host/db".to_owned()));
    }

    #[test]
    fn show_all_on_a_huge_single_line_value_stays_responsive() {
        let mut harness = Harness::new();
        harness.connect_fake();
        harness.click("users");
        let mut page = crate::testing::page(1, false);
        page.rows[0][1] = tabletist_db::Value::Text("x".repeat(1_000_000).into());
        harness.answer_rows(page);
        harness.click("Row 1");
        let tree = harness.settle();
        let show_all = crate::testing::labels(&tree)
            .into_iter()
            .find(|label| label.starts_with("Show all"))
            .expect("a Show all link");
        let started = std::time::Instant::now();
        harness.click(&show_all);
        assert!(
            started.elapsed() < std::time::Duration::from_secs(5),
            "Show all took {:?}",
            started.elapsed()
        );
    }

    #[test]
    fn grid_keys_do_nothing_in_the_structure_view() {
        let mut harness = Harness::new();
        let tab = with_page(&mut harness);
        harness.click("Structure");
        harness.press(Key::ArrowDown, Modifiers::NONE);
        assert_eq!(selection(&harness, tab), None);
    }

    #[test]
    fn space_is_left_to_a_focused_button() {
        let mut harness = Harness::new();
        let tab = with_page(&mut harness);
        // Tab gives a real widget keyboard focus, as a keyboard user would.
        harness.press(Key::Tab, Modifiers::NONE);
        assert!(harness.ctx.memory(|memory| memory.focused().is_some()));
        harness.press(Key::Space, Modifiers::NONE);
        assert!(
            harness.app.workspace(tab).unwrap().row_panel,
            "Space belongs to the focused widget"
        );
    }

    #[test]
    fn choosing_postgres_shows_its_fields() {
        let mut harness = Harness::new();
        harness.press(Key::N, Modifiers::COMMAND);
        assert!(!harness.has("Host"));
        harness.click("PostgreSQL");
        for label in ["Host", "Port", "User", "Password", "Database", "SSL mode"] {
            assert!(harness.has(label), "{label}");
        }
        assert!(!harness.has("File"));
    }

    #[test]
    fn a_notice_shows_until_dismissed() {
        let mut harness = Harness::new();
        harness.app.notice = Some("Could not save the password in the keyring.".into());
        assert!(harness.has("Could not save the password in the keyring."));
        harness.click("Dismiss");
        assert!(harness.app.notice.is_none());
        assert!(!harness.has("Could not save the password in the keyring."));
    }

    /// The field named by the "CA certificate" label beside it.
    fn ca_field(tree: &egui::accesskit::TreeUpdate) -> &egui::accesskit::Node {
        let (label, _) = tree
            .nodes
            .iter()
            .find(|(_, node)| node.value() == Some("CA certificate"))
            .expect("CA certificate label");
        &tree
            .nodes
            .iter()
            .find(|(_, node)| {
                node.role() == egui::accesskit::Role::TextInput
                    && node.labelled_by().contains(label)
            })
            .expect("CA certificate field")
            .1
    }

    #[test]
    fn the_ca_certificate_is_offered_where_it_is_checked() {
        let mut harness = Harness::new();
        harness.press(Key::N, Modifiers::COMMAND);
        harness.click("PostgreSQL");
        let offered = |harness: &mut Harness, tls| {
            if let Some(crate::model::Dialog::Connection(form)) = &mut harness.app.dialog {
                form.tls = tls;
            }
            let tree = harness.settle();
            !ca_field(&tree).is_disabled()
        };
        assert!(!offered(&mut harness, tabletist_db::TlsMode::Prefer));
        // `require` checks against a CA file too, as libpq does.
        assert!(offered(&mut harness, tabletist_db::TlsMode::Require));
        assert!(offered(&mut harness, tabletist_db::TlsMode::VerifyFull));
    }

    #[test]
    fn the_password_prompt_connects_with_the_typed_password() {
        let mut harness = Harness::new();
        let (spec, _) = tabletist_db::ConnectSpec::from_url("postgres://me@db/app").unwrap();
        let saved = crate::connections::SavedConnection {
            id: crate::connections::ConnectionId::new(),
            name: "Prod".into(),
            environment: crate::env::Environment::Production,
            read_only: None,
            password: crate::connections::PasswordMode::Ask,
            ssh_secret: crate::connections::PasswordMode::None,
            spec,
        };
        let conn = saved.id.clone();
        harness.app.connections.upsert(saved);
        let tab = harness.app.active_tab_id();
        harness
            .app
            .apply(crate::model::Action::Connect { tab, conn });
        assert!(harness.has("Password for Prod"));
        if let Some(crate::model::Dialog::Password(prompt)) = &mut harness.app.dialog {
            prompt.password = "typed".into();
        }
        harness.click("Connect");
        match crate::testing::last_sent(&harness.app) {
            Command::Connect { secrets, .. } => {
                assert_eq!(secrets.password.as_deref(), Some("typed"))
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn escape_cancels_the_password_prompt() {
        let mut harness = Harness::new();
        let (spec, _) = tabletist_db::ConnectSpec::from_url("postgres://me@db/app").unwrap();
        let saved = crate::connections::SavedConnection {
            id: crate::connections::ConnectionId::new(),
            name: "Prod".into(),
            environment: crate::env::Environment::None,
            read_only: None,
            password: crate::connections::PasswordMode::Ask,
            ssh_secret: crate::connections::PasswordMode::None,
            spec,
        };
        let conn = saved.id.clone();
        harness.app.connections.upsert(saved);
        let tab = harness.app.active_tab_id();
        harness
            .app
            .apply(crate::model::Action::Connect { tab, conn });
        harness.press(Key::Escape, Modifiers::NONE);
        assert!(harness.app.dialog.is_none());
        assert!(harness.has("Retry"));
    }

    #[test]
    fn choosing_mysql_shows_the_server_fields() {
        let mut harness = Harness::new();
        harness.press(Key::N, Modifiers::COMMAND);
        harness.click("MySQL");
        for label in ["Host", "Port", "User", "Password", "Database", "SSL mode"] {
            assert!(harness.has(label), "{label}");
        }
        assert_eq!(
            match &harness.app.dialog {
                Some(crate::model::Dialog::Connection(form)) => form.port.clone(),
                _ => panic!("no dialog"),
            },
            "3306"
        );
    }

    #[test]
    fn the_host_key_prompt_shows_the_fingerprint_and_trusts_it() {
        let mut harness = Harness::new();
        let tab = harness.app.active_tab_id();
        harness.app.dialog = Some(crate::model::Dialog::HostKey(Box::new(
            crate::model::HostKeyPrompt {
                tab,
                host: "bastion".into(),
                port: 22,
                fingerprint: "SHA256:abc".into(),
            },
        )));
        harness.settle();
        assert!(harness.has("SHA256:abc"));
        harness.click("Trust and connect");
        assert!(harness.app.dialog.is_none());
        assert_eq!(
            harness.app.host_keys.fingerprint("bastion", 22),
            Some("SHA256:abc")
        );
    }

    #[test]
    fn the_prompt_names_the_ssh_secret_it_asks_for() {
        let mut harness = Harness::new();
        let tab = harness.app.active_tab_id();
        for (kind, title) in [
            (
                crate::model::SecretKind::SshPassword,
                "SSH password for Prod",
            ),
            (
                crate::model::SecretKind::SshPassphrase,
                "Key passphrase for Prod",
            ),
            (crate::model::SecretKind::Database, "Password for Prod"),
        ] {
            harness.app.dialog = Some(crate::model::Dialog::Password(Box::new(
                crate::model::PasswordPrompt {
                    tab,
                    kind,
                    name: "Prod".into(),
                    password: String::new(),
                    save: false,
                    message: None,
                },
            )));
            harness.settle();
            assert!(harness.has(title), "{title}");
        }
    }

    #[test]
    fn the_ssh_tunnel_shows_the_fields_for_each_method() {
        let mut harness = Harness::new();
        harness.press(Key::N, Modifiers::COMMAND);
        harness.click("PostgreSQL");
        assert!(!harness.has("SSH host"));
        harness.click("Connect through SSH tunnel");
        for label in [
            "SSH host",
            "SSH port",
            "SSH user",
            "Authentication",
            "SSH password",
        ] {
            assert!(harness.has(label), "{label}");
        }
        assert!(!harness.has("Passphrase"));
        match &mut harness.app.dialog {
            Some(crate::model::Dialog::Connection(form)) => {
                form.ssh_auth = crate::model::SshAuthKind::KeyFile
            }
            other => panic!("{other:?}"),
        }
        harness.settle();
        assert!(harness.has("Key file"));
        assert!(harness.has("Passphrase"));
        assert!(!harness.has("SSH password"));
    }

    #[test]
    fn the_dialog_draws_each_looks_own_form() {
        // The terminal look: lower-case labels, a row each, keys for buttons.
        let mut harness = Harness::new();
        harness.set_look(crate::theme::Look::omarchy());
        harness.press(Key::N, Modifiers::COMMAND);
        harness.click("postgresql");
        for label in [
            "new connection",
            "host : port",
            "ssl mode",
            "ca cert",
            "read-only",
        ] {
            assert!(harness.has(label), "{label}");
        }
        // Screen readers still get each field and button by its name.
        for label in ["Host", "Port", "Test", "Save", "Save & Connect", "Cancel"] {
            assert!(harness.has(label), "{label}");
        }
        // Elsewhere: labelled fields in groups.
        let mut harness = Harness::new();
        harness.press(Key::N, Modifiers::COMMAND);
        harness.click("PostgreSQL");
        for label in ["Server", "Security", "SSL mode", "CA certificate"] {
            assert!(harness.has(label), "{label}");
        }
    }

    #[test]
    fn the_dialog_says_the_connection_opens_read_only() {
        for look in crate::theme::Look::ALL {
            let mut harness = Harness::new();
            harness.set_look(look);
            harness.press(Key::N, Modifiers::COMMAND);
            let tree = harness.settle();
            assert!(
                crate::testing::node(&tree, "Open read-only", egui::accesskit::Role::CheckBox)
                    .is_some(),
                "{}",
                look.name
            );
        }
    }

    #[test]
    fn an_edit_can_delete_its_connection_and_a_new_one_cannot() {
        let mut harness = Harness::new();
        let id = add_saved(&mut harness, "Shop");
        harness
            .app
            .apply(crate::model::Action::EditConnection(id.clone()));
        assert!(harness.has("Edit connection"));
        harness.click("Delete Shop");
        assert!(harness.app.dialog.is_none());
        assert!(harness.app.connections.get(&id).is_none());
        harness.press(Key::N, Modifiers::COMMAND);
        let tree = harness.settle();
        let deletes: Vec<String> = crate::testing::labels(&tree)
            .into_iter()
            .filter(|label| label.starts_with("Delete"))
            .collect();
        assert!(deletes.is_empty(), "{deletes:?}");
    }

    #[test]
    fn a_test_that_passes_says_what_it_reached() {
        let mut harness = Harness::new();
        harness.press(Key::N, Modifiers::COMMAND);
        harness.click("PostgreSQL");
        if let Some(crate::model::Dialog::Connection(form)) = &mut harness.app.dialog {
            form.user = "me".into();
        }
        harness.click("Test");
        assert!(harness.has("Testing…"));
        let request = match &form(&harness).test {
            crate::model::TestState::Running(request) => *request,
            other => panic!("{other:?}"),
        };
        harness.app.apply(crate::model::Action::Backend(
            crate::backend::Event::Tested {
                request,
                result: Ok(()),
            },
        ));
        let tree = harness.settle();
        let said = crate::testing::labels(&tree);
        assert!(
            said.iter()
                .any(|label| label.starts_with("Connected · PostgreSQL · ")),
            "{said:?}"
        );
    }

    #[test]
    fn a_key_pressed_twice_is_two_presses() {
        let mut harness = Harness::new();
        harness.press(Key::N, Modifiers::COMMAND);
        harness.click("PostgreSQL");
        if let Some(crate::model::Dialog::Connection(form)) = &mut harness.app.dialog {
            form.user = "me".into();
        }
        let tests = |harness: &Harness| {
            harness
                .app
                .backend
                .sent
                .iter()
                .filter(|command| matches!(command, Command::Test { .. }))
                .count()
        };
        harness.press(Key::T, Modifiers::COMMAND);
        assert_eq!(tests(&harness), 1);
        let request = match &form(&harness).test {
            crate::model::TestState::Running(request) => *request,
            other => panic!("{other:?}"),
        };
        harness.app.apply(crate::model::Action::Backend(
            crate::backend::Event::Tested {
                request,
                result: Err(tabletist_db::Error::Connect("nope".into())),
            },
        ));
        harness.press(Key::T, Modifiers::COMMAND);
        assert_eq!(tests(&harness), 2, "the second press is not a repeat");
    }

    #[test]
    fn the_keyring_box_saves_the_password_or_asks_every_time() {
        use crate::connections::PasswordMode;
        let mut harness = Harness::new();
        harness.press(Key::N, Modifiers::COMMAND);
        harness.click("PostgreSQL");
        assert_eq!(form(&harness).password_mode, PasswordMode::Keyring);
        harness.click("Keyring");
        assert_eq!(form(&harness).password_mode, PasswordMode::Ask);
        harness.click("Keyring");
        assert_eq!(form(&harness).password_mode, PasswordMode::Keyring);
    }

    #[test]
    fn the_url_tab_fills_the_parameters_and_returns_to_them() {
        let mut harness = Harness::new();
        harness.press(Key::N, Modifiers::COMMAND);
        harness.click("URL");
        assert!(
            !harness.has("Environment"),
            "the URL tab shows the URL alone"
        );
        if let Some(crate::model::Dialog::Connection(form)) = &mut harness.app.dialog {
            form.url = "postgres://me@db.example.com/app".into();
        }
        harness.click("Fill");
        assert!(!form(&harness).url_mode);
        assert_eq!(form(&harness).host, "db.example.com");
        assert!(harness.has("Environment"));
    }

    #[test]
    fn choose_asks_for_a_ca_certificate() {
        let mut harness = Harness::new();
        harness.press(Key::N, Modifiers::COMMAND);
        harness.click("PostgreSQL");
        if let Some(crate::model::Dialog::Connection(form)) = &mut harness.app.dialog {
            form.tls = tabletist_db::TlsMode::VerifyFull;
        }
        harness.click("Choose a CA certificate");
        assert_eq!(form(&harness).pick_target, crate::model::PickTarget::CaFile);
        assert!(form(&harness).pick_request.is_some());
    }

    #[test]
    fn choose_asks_for_a_database_file_and_a_key_file() {
        use crate::model::PickTarget;
        for look in crate::theme::Look::ALL {
            let mut harness = Harness::new();
            harness.set_look(look);
            harness.press(Key::N, Modifiers::COMMAND);
            harness.click("Choose a database file");
            assert_eq!(form(&harness).pick_target, PickTarget::Sqlite);
            let asked = form(&harness).pick_request;
            assert!(asked.is_some(), "{}", look.name);
            harness.click(&look.label("PostgreSQL"));
            harness.click("Connect through SSH tunnel");
            if let Some(crate::model::Dialog::Connection(form)) = &mut harness.app.dialog {
                form.ssh_auth = crate::model::SshAuthKind::KeyFile;
            }
            harness.click("Choose a key file");
            assert_eq!(
                form(&harness).pick_target,
                PickTarget::KeyFile,
                "{}",
                look.name
            );
            assert_ne!(form(&harness).pick_request, asked, "{}", look.name);
        }
    }

    #[test]
    fn the_close_button_closes_the_dialog() {
        let mut harness = Harness::new();
        harness.press(Key::N, Modifiers::COMMAND);
        harness.click("Close");
        assert!(harness.app.dialog.is_none());
    }

    #[test]
    fn the_dialogs_keys_test_and_save() {
        let mut harness = Harness::new();
        harness.press(Key::N, Modifiers::COMMAND);
        harness.click("PostgreSQL");
        if let Some(crate::model::Dialog::Connection(form)) = &mut harness.app.dialog {
            form.name = "Shop".into();
            form.user = "me".into();
        }
        let before = harness.app.backend.sent.len();
        harness.press(Key::T, Modifiers::COMMAND);
        assert!(
            harness.app.backend.sent[before..]
                .iter()
                .any(|command| matches!(command, Command::Test { .. })),
            "Mod+T tests"
        );
        assert_eq!(
            harness.app.tabs.len(),
            1,
            "and opens no tab behind the dialog"
        );
        harness.press(Key::S, Modifiers::COMMAND);
        assert!(harness.app.dialog.is_none(), "Mod+S saves");
        assert_eq!(harness.app.connections.connections.len(), 1);
    }

    #[test]
    fn u_shows_the_url_field_in_the_terminal_look() {
        let mut harness = Harness::new();
        harness.set_look(crate::theme::Look::omarchy());
        let id = add_saved(&mut harness, "Shop");
        harness.app.apply(crate::model::Action::EditConnection(id));
        harness.press(Key::U, Modifiers::NONE);
        assert!(form(&harness).url_mode);
        assert!(harness.has("url"));
        // A new connection opens with the keyboard in Name: there, u is a
        // letter.
        let mut harness = Harness::new();
        harness.set_look(crate::theme::Look::omarchy());
        harness.press(Key::N, Modifiers::COMMAND);
        harness.press(Key::U, Modifiers::NONE);
        assert!(!form(&harness).url_mode);
    }

    #[test]
    fn the_terminal_footers_status_stays_clear_of_its_keys() {
        // A footer too narrow for the status and every key drops keys, not
        // the buttons they stand for.
        // The smallest window, with a quick Test and with one whose time is
        // long enough to reach the keys.
        for took in [
            std::time::Duration::from_millis(42),
            std::time::Duration::from_secs(100_000),
        ] {
            let mut harness = Harness::with_size(egui::vec2(720.0, 480.0));
            harness.set_look(crate::theme::Look::omarchy());
            harness.press(Key::N, Modifiers::COMMAND);
            harness.click("postgresql");
            if let Some(crate::model::Dialog::Connection(form)) = &mut harness.app.dialog {
                form.test = crate::model::TestState::Passed;
                form.test_took = Some(took);
            }
            let tree = harness.settle();
            let status = tree
                .nodes
                .iter()
                .find(|(_, node)| {
                    node.label()
                        .or_else(|| node.value())
                        .is_some_and(|text| text.starts_with("connected · "))
                })
                .and_then(|(_, node)| node.bounds())
                .expect("the Test's status");
            let status = egui::Rect::from_min_max(
                egui::pos2(status.x0 as f32, status.y0 as f32),
                egui::pos2(status.x1 as f32, status.y1 as f32),
            );
            for label in ["Test", "Save", "Save & Connect", "Cancel"] {
                let button = crate::testing::bounds(&tree, label, egui::accesskit::Role::Button)
                    .unwrap_or_else(|| panic!("{label} is gone after {took:?}"));
                assert!(
                    !status.intersects(button),
                    "the status at {status:?} runs into {label} at {button:?} after {took:?}"
                );
            }
        }
    }

    #[test]
    fn each_keyring_box_says_which_secret_it_keeps() {
        use crate::connections::PasswordMode;
        for look in crate::theme::Look::ALL {
            let keyring = if look.faces == crate::theme::Faces::Plex {
                "Keychain"
            } else {
                "Keyring"
            };
            let ssh = format!("{keyring} for the SSH secret");
            let mut harness = Harness::new();
            harness.set_look(look);
            harness.press(Key::N, Modifiers::COMMAND);
            harness.click(&look.label("PostgreSQL"));
            harness.click("Connect through SSH tunnel");
            let tree = harness.settle();
            for name in [keyring, ssh.as_str()] {
                let boxes = tree
                    .nodes
                    .iter()
                    .filter(|(_, node)| {
                        node.role() == egui::accesskit::Role::CheckBox && node.label() == Some(name)
                    })
                    .count();
                assert_eq!(boxes, 1, "{name} in {}", look.name);
            }
            harness.click(&ssh);
            assert_eq!(
                form(&harness).ssh_secret_mode,
                PasswordMode::Ask,
                "{}",
                look.name
            );
            assert_eq!(
                form(&harness).password_mode,
                PasswordMode::Keyring,
                "{}",
                look.name
            );
        }
    }

    #[test]
    fn the_dialogs_keys_are_not_a_press_of_the_focused_button() {
        let mut harness = Harness::new();
        harness.press(Key::N, Modifiers::COMMAND);
        if let Some(crate::model::Dialog::Connection(form)) = &mut harness.app.dialog {
            form.name = "Shop".into();
            form.sqlite_path = "/tmp/shop.db".into();
        }
        // Cancel takes the keyboard, as a screen reader would give it.
        let tree = harness.settle();
        let cancel =
            crate::testing::node(&tree, "Cancel", egui::accesskit::Role::Button).expect("Cancel");
        harness.frame(vec![egui::Event::AccessKitActionRequest(
            egui::accesskit::ActionRequest {
                target_tree: egui::accesskit::TreeId::ROOT,
                target_node: cancel,
                action: egui::accesskit::Action::Focus,
                data: None,
            },
        )]);
        assert_eq!(focused_name(&harness.settle()), "Cancel");
        harness.press(Key::Enter, Modifiers::COMMAND);
        assert_eq!(
            harness.app.connections.connections.len(),
            1,
            "Mod+Enter saves, whatever has the keyboard"
        );
    }

    #[test]
    fn the_choices_are_named_by_their_label() {
        for look in crate::theme::Look::ALL {
            let mut harness = Harness::new();
            harness.set_look(look);
            harness.press(Key::N, Modifiers::COMMAND);
            harness.click(&look.label("PostgreSQL"));
            harness.click("Connect through SSH tunnel");
            let tree = harness.settle();
            // A heading, and one of the choices under it.
            let mut groups = vec![("Type", "MySQL"), ("Environment", "Production")];
            if look.terminal {
                groups.push(("SSL mode", "verify-full"));
                groups.push(("Authentication", "Agent"));
            }
            for (heading, choice) in groups {
                let heading = look.label(heading);
                let choice = look.label(choice);
                let (_, group) = tree
                    .nodes
                    .iter()
                    .find(|(_, node)| {
                        node.role() == egui::accesskit::Role::RadioGroup
                            && node.label() == Some(heading.as_str())
                    })
                    .unwrap_or_else(|| panic!("no group named {heading} in {}", look.name));
                let inside = tree.nodes.iter().any(|(id, node)| {
                    group.children().contains(id)
                        && node.role() == egui::accesskit::Role::RadioButton
                        && node.label() == Some(choice.as_str())
                });
                assert!(inside, "{choice} is not in {heading} in {}", look.name);
            }
        }
    }

    #[test]
    fn tab_reaches_the_dialogs_tabs_before_close() {
        let mut harness = Harness::new();
        let id = add_saved(&mut harness, "Shop");
        // An edit opens with the keyboard nowhere.
        harness.app.apply(crate::model::Action::EditConnection(id));
        let mut reached = Vec::new();
        for _ in 0..3 {
            harness.press(Key::Tab, Modifiers::NONE);
            reached.push(focused_name(&harness.settle()));
        }
        assert_eq!(reached, ["Parameters", "URL", "Close"]);
    }

    /// Where the first node `found` accepts sits, in points.
    fn bounds_where(
        tree: &egui::accesskit::TreeUpdate,
        found: impl Fn(&egui::accesskit::Node) -> bool,
    ) -> Option<egui::Rect> {
        let (_, node) = tree.nodes.iter().find(|(_, node)| found(node))?;
        let rect = node.bounds()?;
        Some(egui::Rect::from_min_max(
            egui::pos2(rect.x0 as f32, rect.y0 as f32),
            egui::pos2(rect.x1 as f32, rect.y1 as f32),
        ))
    }

    /// Where `text` is said, as a label or a widget's name.
    fn said_at(tree: &egui::accesskit::TreeUpdate, text: &str) -> Option<egui::Rect> {
        bounds_where(tree, |node| {
            node.label().or_else(|| node.value()) == Some(text)
        })
    }

    /// A saved PostgreSQL connection through an SSH tunnel with a key file:
    /// the tallest form the dialog opens with.
    fn add_saved_with_tunnel(harness: &mut Harness) -> crate::connections::ConnectionId {
        let (mut spec, _) =
            tabletist_db::ConnectSpec::from_url("postgres://me@db.example.com/app").unwrap();
        spec.ssh = Some(tabletist_db::SshSpec {
            host: "bastion".into(),
            port: Some(22),
            user: "ops".into(),
            auth: tabletist_db::SshAuth::KeyFile {
                path: "/home/me/.ssh/id_ed25519".into(),
            },
        });
        let saved = crate::connections::SavedConnection {
            id: crate::connections::ConnectionId::new(),
            name: "Prod".into(),
            environment: crate::env::Environment::Production,
            read_only: None,
            password: crate::connections::PasswordMode::None,
            ssh_secret: crate::connections::PasswordMode::None,
            spec,
        };
        let id = saved.id.clone();
        harness.app.connections.upsert(saved);
        id
    }

    #[test]
    fn what_went_wrong_is_always_on_screen() {
        use crate::model::TestState;
        for look in crate::theme::Look::ALL {
            // With the tunnel's fields the form is taller than the window
            // and scrolls.
            for (case, tunnel) in [
                ("a save without a name", false),
                ("a failed test", false),
                ("an unknown host key", false),
                ("a failed test", true),
                ("an unknown host key", true),
            ] {
                let mut harness = Harness::new();
                harness.set_look(look);
                harness.press(Key::N, Modifiers::COMMAND);
                harness.click(&look.label("PostgreSQL"));
                if tunnel {
                    harness.click("Connect through SSH tunnel");
                }
                let mut shown: Vec<String> = Vec::new();
                match case {
                    "a save without a name" => {
                        harness.press(Key::S, Modifiers::COMMAND);
                        shown.push(form(&harness).message.clone().expect("a message"));
                    }
                    "a failed test" => {
                        if let Some(crate::model::Dialog::Connection(form)) =
                            &mut harness.app.dialog
                        {
                            form.test = TestState::Failed("connection refused".into());
                        }
                        shown.push("connection refused".into());
                    }
                    _ => {
                        if let Some(crate::model::Dialog::Connection(form)) =
                            &mut harness.app.dialog
                        {
                            form.test = TestState::Untrusted {
                                host: "bastion".into(),
                                port: 22,
                                fingerprint: "SHA256:abc".into(),
                            };
                        }
                        shown.push("Unknown SSH host key for bastion: SHA256:abc".into());
                        shown.push("Trust and test".into());
                    }
                }
                let tree = harness.settle();
                let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, harness.size);
                let header = if look.terminal {
                    crate::testing::bounds(&tree, "Paste URL", egui::accesskit::Role::Button)
                } else {
                    crate::testing::bounds(&tree, "Close", egui::accesskit::Role::Button)
                }
                .expect("the header");
                let footer = crate::testing::bounds(&tree, "Test", egui::accesskit::Role::Button)
                    .expect("Test");
                assert!(
                    screen.contains_rect(footer),
                    "{case} in {}: the footer at {footer:?} is pushed off the window",
                    look.name
                );
                for text in shown {
                    let at = said_at(&tree, &text)
                        .unwrap_or_else(|| panic!("{text:?} is not said in {}", look.name));
                    assert!(
                        screen.contains_rect(at)
                            && at.top() >= header.bottom()
                            && at.bottom() <= footer.top(),
                        "{case} in {}: {text:?} at {at:?} is not between the header ({header:?}) \
                         and the footer ({footer:?})",
                        look.name
                    );
                }
            }
        }
    }

    #[test]
    fn a_long_error_keeps_the_footer_on_screen_and_the_dialog_as_wide() {
        use crate::model::TestState;
        let long = "The server closed the connection before it answered. ".repeat(60);
        assert!(long.len() > 3000, "{}", long.len());
        for look in crate::theme::Look::ALL {
            let mut save_right = Vec::new();
            for message in ["connection refused", long.as_str()] {
                let mut harness = Harness::new();
                harness.set_look(look);
                harness.press(Key::N, Modifiers::COMMAND);
                harness.click(&look.label("PostgreSQL"));
                if let Some(crate::model::Dialog::Connection(form)) = &mut harness.app.dialog {
                    form.test = TestState::Failed(message.into());
                }
                let tree = harness.settle();
                let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, harness.size);
                for button in ["Test", "Save", "Cancel", "Save & Connect"] {
                    let at = crate::testing::bounds(&tree, button, egui::accesskit::Role::Button)
                        .unwrap_or_else(|| panic!("{button} in {}", look.name));
                    assert!(
                        screen.contains_rect(at),
                        "{} characters of error in {}: {button} at {at:?} is pushed off the \
                         window",
                        message.len(),
                        look.name
                    );
                }
                let save = crate::testing::bounds(&tree, "Save", egui::accesskit::Role::Button)
                    .expect("Save");
                save_right.push(save.right());
            }
            assert!(
                (save_right[0] - save_right[1]).abs() <= 0.5,
                "{}: a long error moved the dialog's right edge from {} to {}",
                look.name,
                save_right[0],
                save_right[1]
            );
        }
    }

    #[test]
    fn the_connection_dialog_is_solid_from_the_first_frame_it_shows() {
        for look in crate::theme::Look::ALL {
            let mut harness = Harness::new();
            harness.set_look(look);
            let id = add_saved(&mut harness, "Shop");
            harness.settle();
            let palette = harness.app.palette;
            let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, harness.size);
            // The veil over the window, as the look's dialogs draw it.
            let veil =
                crate::ui::widgets::modal(egui::Id::new("veil"), &look, &palette).backdrop_color;
            let title = look.label("Edit connection");
            harness.app.apply(crate::model::Action::EditConnection(id));
            // egui fades an area in, and a sheet this large is see-through
            // while it does: the window shows through its fields. So the
            // veil is whole on every frame it is drawn, and never drawn
            // before the dialog.
            let mut shown = false;
            for frame in 0..8 {
                harness.frame(Vec::new());
                let veils: Vec<egui::Color32> = harness
                    .fills
                    .iter()
                    .filter(|(rect, color)| rect.contains_rect(screen) && !color.is_opaque())
                    .map(|(_, color)| *color)
                    .collect();
                let titled = harness.painted_color(&title).is_some();
                if veils.is_empty() && !titled {
                    assert!(!shown, "{}: the dialog went away", look.name);
                    continue;
                }
                shown = true;
                assert!(
                    titled,
                    "{}, frame {frame}: the window is veiled before the dialog shows",
                    look.name
                );
                assert_eq!(
                    veils,
                    [veil],
                    "{}, frame {frame}: the dialog is fading in",
                    look.name
                );
            }
            assert!(shown, "{}: the dialog never showed", look.name);
        }
    }

    #[test]
    fn the_dialog_reaches_its_height_at_once() {
        for look in crate::theme::Look::ALL {
            let mut harness = Harness::new();
            harness.set_look(look);
            harness.press(Key::N, Modifiers::COMMAND);
            let footer = |harness: &mut Harness| {
                let tree = harness.frame(Vec::new());
                crate::testing::bounds(&tree, "Cancel", egui::accesskit::Role::Button)
                    .expect("Cancel")
            };
            for step in [
                look.label("PostgreSQL"),
                "Connect through SSH tunnel".to_owned(),
            ] {
                harness.click(&step);
                let at_once = footer(&mut harness);
                for _ in 0..8 {
                    harness.frame(Vec::new());
                }
                assert_eq!(
                    footer(&mut harness),
                    at_once,
                    "the footer kept moving after {step} in {}",
                    look.name
                );
            }
        }
    }

    #[test]
    fn a_tall_form_that_fits_the_window_opens_whole() {
        for look in crate::theme::Look::ALL {
            let mut harness = Harness::with_size(egui::vec2(1280.0, 1100.0));
            harness.set_look(look);
            let id = add_saved_with_tunnel(&mut harness);
            harness.settle();
            let title = look.label("Edit connection");
            let last = if look.terminal {
                "· default for production"
            } else {
                "Blocks every write from this app. On by default for production; turn off to edit."
            };
            // The first time, and again: egui remembers the dialog's size.
            for opening in ["first", "second"] {
                harness
                    .app
                    .apply(crate::model::Action::EditConnection(id.clone()));
                // The first frame that paints the dialog.
                let mut tree = harness.frame(Vec::new());
                for _ in 0..5 {
                    if harness.painted.iter().any(|(text, _)| *text == title) {
                        break;
                    }
                    tree = harness.frame(Vec::new());
                }
                let shown_at = said_at(&tree, &title).expect("the title");
                let end = said_at(&tree, last).expect("the form's last line");
                let footer = crate::testing::bounds(&tree, "Test", egui::accesskit::Role::Button)
                    .expect("Test");
                assert!(
                    end.bottom() <= footer.top(),
                    "{}, {opening} opening: the form's end at {end:?} is under the footer at \
                     {footer:?}",
                    look.name
                );
                // It is first seen where it stays.
                let tree = harness.settle();
                assert_eq!(
                    said_at(&tree, &title),
                    Some(shown_at),
                    "{}, {opening} opening",
                    look.name
                );
                harness.press(Key::Escape, Modifiers::NONE);
                assert!(harness.app.dialog.is_none());
            }
        }
    }

    #[test]
    fn the_terminal_dialog_keeps_one_width_in_the_smallest_window() {
        for tunnel in [false, true] {
            let mut harness = Harness::with_size(egui::vec2(720.0, 480.0));
            harness.set_look(crate::theme::Look::omarchy());
            harness.press(Key::N, Modifiers::COMMAND);
            harness.click("postgresql");
            if tunnel {
                harness.click("Connect through SSH tunnel");
            }
            let tree = harness.settle();
            // Both end the same distance before their part's right edge.
            let header = crate::testing::bounds(&tree, "Paste URL", egui::accesskit::Role::Button)
                .expect("the header's hint");
            let footer = crate::testing::bounds(&tree, "Cancel", egui::accesskit::Role::Button)
                .expect("Cancel");
            assert!(
                (header.right() - footer.right()).abs() < 0.5,
                "the header ends at {} and the footer at {}",
                header.right(),
                footer.right()
            );
            assert!(footer.right() <= harness.size.x && header.left() >= 0.0);
        }
    }

    #[test]
    fn a_long_name_stays_clear_of_the_headers_controls() {
        let name = "analytics@replica-eu-west-1.internal.example.com:5432/warehouse_reporting";
        for look in crate::theme::Look::ALL {
            for size in [egui::vec2(720.0, 480.0), egui::vec2(1280.0, 800.0)] {
                let mut harness = Harness::with_size(size);
                harness.set_look(look);
                harness.press(Key::N, Modifiers::COMMAND);
                if let Some(crate::model::Dialog::Connection(form)) = &mut harness.app.dialog {
                    form.name = name.into();
                }
                let tree = harness.settle();
                // What the header says of the name: it may end in "…".
                let subtitle = bounds_where(&tree, |node| {
                    node.role() == egui::accesskit::Role::Label
                        && node
                            .label()
                            .or_else(|| node.value())
                            .is_some_and(|text| text.starts_with(&name[..8]))
                })
                .expect("the header names the connection");
                let control = if look.terminal {
                    crate::testing::bounds(&tree, "Paste URL", egui::accesskit::Role::Button)
                } else {
                    bounds_where(&tree, |node| {
                        node.role() == egui::accesskit::Role::RadioButton
                            && node.label() == Some("Parameters")
                    })
                }
                .expect("the header's control");
                assert!(
                    subtitle.right() <= control.left(),
                    "{} at {size:?}: the name at {subtitle:?} runs into {control:?}",
                    look.name
                );
            }
        }
    }

    #[test]
    fn a_held_test_key_tests_once() {
        let mut harness = Harness::new();
        harness.press(Key::N, Modifiers::COMMAND);
        harness.click("PostgreSQL");
        if let Some(crate::model::Dialog::Connection(form)) = &mut harness.app.dialog {
            form.name = "Shop".into();
            form.user = "me".into();
        }
        harness.settle();
        let tests = |harness: &Harness, from: usize| {
            harness.app.backend.sent[from..]
                .iter()
                .filter(|command| matches!(command, Command::Test { .. }))
                .count()
        };
        // The key goes down and the keyboard repeats it while it is held.
        let key = |pressed: bool| egui::Event::Key {
            key: Key::T,
            physical_key: None,
            pressed,
            repeat: false,
            modifiers: Modifiers::COMMAND,
        };
        let before = harness.app.backend.sent.len();
        harness.frame(vec![key(true)]);
        harness.frame(vec![key(true)]);
        assert_eq!(tests(&harness, before), 1);
        // The server refuses while the key is still held: the repeats that
        // follow are not new presses, so they do not log in again.
        let request = match &form(&harness).test {
            crate::model::TestState::Running(request) => *request,
            other => panic!("{other:?}"),
        };
        harness.app.apply(crate::model::Action::Backend(
            crate::backend::Event::Tested {
                request,
                result: Err(tabletist_db::Error::Connect("refused".into())),
            },
        ));
        for _ in 0..4 {
            harness.frame(vec![key(true)]);
        }
        assert_eq!(tests(&harness, before), 1, "a repeat is not a press");
        // Let go and pressed again: that is a press, and it tests.
        harness.frame(vec![key(false)]);
        harness.frame(vec![key(true)]);
        assert_eq!(tests(&harness, before), 2);
        // Pressed once more before that Test has answered: one at a time.
        harness.frame(vec![key(false)]);
        harness.frame(vec![key(true)]);
        harness.settle();
        assert!(matches!(
            form(&harness).test,
            crate::model::TestState::Running(_)
        ));
        assert_eq!(tests(&harness, before), 2, "one Test at a time");
    }

    #[test]
    fn both_secrets_are_password_fields_to_screen_readers() {
        for look in crate::theme::Look::ALL {
            let mut harness = Harness::new();
            harness.set_look(look);
            harness.press(Key::N, Modifiers::COMMAND);
            harness.click(&look.label("PostgreSQL"));
            // The tunnel logs in with a password until told otherwise.
            harness.click("Connect through SSH tunnel");
            let tree = harness.settle();
            let passwords = tree
                .nodes
                .iter()
                .filter(|(_, node)| node.role() == egui::accesskit::Role::PasswordInput)
                .count();
            assert_eq!(passwords, 2, "{}", look.name);
        }
    }

    #[test]
    fn a_url_typed_in_the_terminal_look_fills_the_form_when_the_keyboard_leaves_it() {
        let mut harness = Harness::new();
        harness.set_look(crate::theme::Look::omarchy());
        let id = add_saved(&mut harness, "Shop");
        harness
            .app
            .apply(crate::model::Action::EditConnection(id.clone()));
        harness.press(Key::U, Modifiers::NONE);
        harness.frame(vec![egui::Event::Text(
            "postgres://me@db.example.com/app".into(),
        )]);
        harness.settle();
        // No Enter: the keyboard moves on to the fields.
        harness.press(Key::Tab, Modifiers::NONE);
        assert_eq!(form(&harness).host, "db.example.com");
        assert!(!form(&harness).url_mode);
        // So an edit made after it is what is saved.
        if let Some(crate::model::Dialog::Connection(form)) = &mut harness.app.dialog {
            form.host = "replica.example.com".into();
        }
        harness.press(Key::S, Modifiers::COMMAND);
        assert!(harness.app.dialog.is_none());
        assert_eq!(
            harness.app.connections.get(&id).unwrap().spec.host,
            "replica.example.com"
        );
    }

    #[test]
    fn the_dialogs_keys_wait_for_an_open_list() {
        let mut harness = Harness::new();
        harness.press(Key::N, Modifiers::COMMAND);
        harness.click("PostgreSQL");
        if let Some(crate::model::Dialog::Connection(form)) = &mut harness.app.dialog {
            form.name = "Shop".into();
            form.user = "me".into();
        }
        let tree = harness.settle();
        let combo = tree
            .nodes
            .iter()
            .find(|(_, node)| node.role() == egui::accesskit::Role::ComboBox)
            .map(|(id, _)| *id)
            .expect("the SSL mode list");
        harness.frame(vec![egui::Event::AccessKitActionRequest(
            egui::accesskit::ActionRequest {
                target_tree: egui::accesskit::TreeId::ROOT,
                target_node: combo,
                action: egui::accesskit::Action::Click,
                data: None,
            },
        )]);
        let before = harness.app.backend.sent.len();
        harness.press(Key::T, Modifiers::COMMAND);
        harness.press(Key::S, Modifiers::COMMAND);
        assert!(
            harness.app.dialog.is_some(),
            "Mod+S saved under an open list"
        );
        assert!(harness.app.connections.connections.is_empty());
        assert!(
            !harness.app.backend.sent[before..]
                .iter()
                .any(|command| matches!(command, Command::Test { .. })),
            "Mod+T tested under an open list"
        );
    }

    #[test]
    fn the_terminal_hint_shows_and_hides_the_url_field() {
        let mut harness = Harness::new();
        harness.set_look(crate::theme::Look::omarchy());
        let id = add_saved(&mut harness, "Shop");
        harness.app.apply(crate::model::Action::EditConnection(id));
        assert!(!harness.has("url"));
        harness.click("Paste URL");
        assert!(form(&harness).url_mode);
        assert!(harness.has("url"));
        harness.click("Paste URL");
        assert!(!form(&harness).url_mode);
        assert!(!harness.has("url"));
    }

    #[test]
    fn trusting_the_host_key_tests_again() {
        for look in crate::theme::Look::ALL {
            let mut harness = Harness::new();
            harness.set_look(look);
            harness.press(Key::N, Modifiers::COMMAND);
            harness.click(&look.label("PostgreSQL"));
            if let Some(crate::model::Dialog::Connection(form)) = &mut harness.app.dialog {
                form.user = "me".into();
                form.test = crate::model::TestState::Untrusted {
                    host: "bastion".into(),
                    port: 22,
                    fingerprint: "SHA256:abc".into(),
                };
            }
            let before = harness.app.backend.sent.len();
            harness.click("Trust and test");
            assert_eq!(
                harness.app.host_keys.fingerprint("bastion", 22),
                Some("SHA256:abc"),
                "{}",
                look.name
            );
            assert!(
                harness.app.backend.sent[before..]
                    .iter()
                    .any(|command| matches!(command, Command::Test { .. })),
                "{}",
                look.name
            );
        }
    }

    #[test]
    fn a_key_the_footer_has_no_room_to_show_still_has_its_button() {
        let mut harness = Harness::with_size(egui::vec2(720.0, 480.0));
        harness.set_look(crate::theme::Look::omarchy());
        harness.press(Key::N, Modifiers::COMMAND);
        harness.click("postgresql");
        if let Some(crate::model::Dialog::Connection(form)) = &mut harness.app.dialog {
            form.user = "me".into();
            // A status long enough to leave no room for the Test key.
            form.test = crate::model::TestState::Passed;
            form.test_took = Some(std::time::Duration::from_secs(100_000));
        }
        let before = harness.app.backend.sent.len();
        harness.click("Test");
        assert!(
            harness.app.backend.sent[before..]
                .iter()
                .any(|command| matches!(command, Command::Test { .. }))
        );
    }

    #[test]
    fn enter_in_the_url_field_fills_the_form() {
        // The sheet's URL tab.
        let mut harness = Harness::new();
        harness.press(Key::N, Modifiers::COMMAND);
        harness.click("URL");
        harness.frame(vec![egui::Event::Text(
            "postgres://me@db.example.com/app".into(),
        )]);
        harness.settle();
        harness.press(Key::Enter, Modifiers::NONE);
        assert_eq!(form(&harness).host, "db.example.com");
        assert!(!form(&harness).url_mode);
        // The terminal look's URL row.
        let mut harness = Harness::new();
        harness.set_look(crate::theme::Look::omarchy());
        let id = add_saved(&mut harness, "Shop");
        harness.app.apply(crate::model::Action::EditConnection(id));
        harness.press(Key::U, Modifiers::NONE);
        harness.frame(vec![egui::Event::Text(
            "postgres://me@db.example.com/app".into(),
        )]);
        harness.settle();
        harness.press(Key::Enter, Modifiers::NONE);
        assert_eq!(form(&harness).host, "db.example.com");
        assert!(!form(&harness).url_mode);
    }

    #[test]
    fn mod_enter_saves_and_connects() {
        let mut harness = Harness::new();
        harness.press(Key::N, Modifiers::COMMAND);
        if let Some(crate::model::Dialog::Connection(form)) = &mut harness.app.dialog {
            form.name = "Shop".into();
            form.sqlite_path = "/tmp/shop.db".into();
        }
        let before = harness.app.backend.sent.len();
        harness.press(Key::Enter, Modifiers::COMMAND);
        assert_eq!(harness.app.connections.connections.len(), 1);
        assert!(
            harness.app.backend.sent[before..]
                .iter()
                .any(|command| matches!(command, Command::Connect { .. })),
            "saved, but not connected"
        );
    }

    use tabletist_db::ssh_config::{AgentSocket, ConfigHost, HostConfig, Proxy};

    fn config_host(alias: &str, config: HostConfig) -> ConfigHost {
        ConfigHost {
            alias: alias.into(),
            config,
        }
    }

    fn bastion() -> ConfigHost {
        config_host(
            "bastion",
            HostConfig {
                host_name: Some("10.0.0.5".into()),
                port: Some(2222),
                user: Some("ops".into()),
                identity_agent: Some(AgentSocket::Environment),
                ..HostConfig::default()
            },
        )
    }

    /// A new PostgreSQL connection through an SSH tunnel, with these hosts
    /// from ~/.ssh/config, drawn with `look`.
    fn ssh_dialog_in(look: crate::theme::Look, hosts: Vec<ConfigHost>) -> Harness {
        let mut harness = Harness::new();
        harness.set_look(look);
        harness.press(Key::N, Modifiers::COMMAND);
        harness.click(&look.label("PostgreSQL"));
        harness.click("Connect through SSH tunnel");
        ssh_form(&mut harness).ssh_hosts = hosts;
        harness.settle();
        harness
    }

    /// [`ssh_dialog_in`] the standard look.
    fn ssh_dialog(hosts: Vec<ConfigHost>) -> Harness {
        ssh_dialog_in(crate::theme::Look::standard(), hosts)
    }

    fn ssh_form(harness: &mut Harness) -> &mut crate::model::ConnectionForm {
        match &mut harness.app.dialog {
            Some(crate::model::Dialog::Connection(form)) => form,
            other => panic!("{other:?}"),
        }
    }

    /// The placeholder of the field named `name`: by its own name, where
    /// the dialog has no label beside it, or by the label that says so.
    fn placeholder(harness: &mut Harness, name: &str) -> Option<String> {
        let tree = harness.settle();
        let label = tree
            .nodes
            .iter()
            .find(|(_, node)| node.value() == Some(name))
            .map(|(id, _)| *id);
        tree.nodes
            .iter()
            .find(|(_, node)| {
                node.role() == egui::accesskit::Role::TextInput
                    && (node.label() == Some(name)
                        || label.is_some_and(|label| node.labelled_by().contains(&label)))
            })
            .unwrap_or_else(|| panic!("no field named {name:?}"))
            .1
            .placeholder()
            .map(str::to_owned)
    }

    #[test]
    fn the_config_hosts_button_shows_only_when_the_config_names_hosts() {
        let mut harness = ssh_dialog(Vec::new());
        assert!(!harness.has("Hosts from ~/.ssh/config"));
        let mut harness = ssh_dialog(vec![bastion()]);
        assert!(harness.has("Hosts from ~/.ssh/config"));
    }

    #[test]
    fn choosing_a_config_host_fills_the_ssh_host() {
        let replica = config_host("replica", HostConfig::default());
        let mut harness = ssh_dialog(vec![bastion(), replica]);
        harness.click("Hosts from ~/.ssh/config");
        assert!(harness.has("replica"));
        harness.click("bastion");
        let form = ssh_form(&mut harness);
        assert_eq!(form.ssh_host, "bastion");
        assert_eq!(form.ssh_auth, crate::model::SshAuthKind::Agent);
        // The list closes behind a pick made without the pointer too.
        harness.settle();
        assert!(!egui::Popup::is_any_open(&harness.ctx), "the list is open");
        assert!(!harness.has("replica"));
    }

    #[test]
    fn a_config_host_shows_its_values_as_hints() {
        let mut harness = ssh_dialog(vec![bastion()]);
        assert_eq!(placeholder(&mut harness, "SSH port").as_deref(), Some("22"));
        ssh_form(&mut harness).ssh_host = "bastion".into();
        assert_eq!(
            placeholder(&mut harness, "SSH port").as_deref(),
            Some("2222")
        );
        assert_eq!(
            placeholder(&mut harness, "SSH user").as_deref(),
            Some("ops (from ~/.ssh/config)")
        );
        assert!(harness.has("10.0.0.5 from ~/.ssh/config"));
    }

    #[test]
    fn a_config_host_behind_a_proxy_warns() {
        let jump = config_host(
            "gateway-only",
            HostConfig {
                proxy: Some(Proxy::Jump("bastion".into())),
                ..HostConfig::default()
            },
        );
        let mut harness = ssh_dialog(vec![jump]);
        ssh_form(&mut harness).ssh_host = "gateway-only".into();
        assert!(harness.has("Uses ProxyJump, which Tabletist does not support yet."));
    }

    #[test]
    fn the_terminal_look_offers_config_hosts_and_their_values() {
        let look = crate::theme::Look::omarchy();
        let mut harness = ssh_dialog_in(look, Vec::new());
        assert!(!harness.has("Hosts from ~/.ssh/config"));
        let replica = config_host("replica", HostConfig::default());
        let mut harness = ssh_dialog_in(look, vec![bastion(), replica]);
        assert_eq!(placeholder(&mut harness, "SSH port").as_deref(), Some("22"));
        harness.click("Hosts from ~/.ssh/config");
        assert!(harness.has("replica"));
        harness.click("bastion");
        let form = ssh_form(&mut harness);
        assert_eq!(form.ssh_host, "bastion");
        assert_eq!(form.ssh_auth, crate::model::SshAuthKind::Agent);
        assert_eq!(
            placeholder(&mut harness, "SSH port").as_deref(),
            Some("2222")
        );
        // The row's label is in the look's case; the config's values are
        // as the config has them.
        assert_eq!(
            placeholder(&mut harness, "ssh user").as_deref(),
            Some("ops (from ~/.ssh/config)")
        );
        assert!(harness.has("10.0.0.5 from ~/.ssh/config"));
    }

    #[test]
    fn a_config_hosts_key_file_and_proxy_command_show_in_every_look() {
        for look in crate::theme::Look::ALL {
            let vault = config_host(
                "vault",
                HostConfig {
                    host_name: Some("10.0.0.9".into()),
                    identity_file: Some("/home/me/.ssh/vault".into()),
                    proxy: Some(Proxy::Command("ssh -W %h:%p gateway".into())),
                    ..HostConfig::default()
                },
            );
            let mut harness = ssh_dialog_in(look, vec![vault]);
            harness.click("Hosts from ~/.ssh/config");
            harness.click("vault");
            // A host with a key file is logged in to with it.
            assert_eq!(
                ssh_form(&mut harness).ssh_auth,
                crate::model::SshAuthKind::KeyFile,
                "{}",
                look.name
            );
            // The sheet names the field itself; the terminal look's row
            // has a label, in its case.
            let key_file = if look.terminal {
                "key file"
            } else {
                "Key file"
            };
            assert_eq!(
                placeholder(&mut harness, key_file).as_deref(),
                Some("/home/me/.ssh/vault (from ~/.ssh/config)"),
                "{}",
                look.name
            );
            // The warning takes the place of the host name's line.
            assert!(
                harness.has("Uses ProxyCommand, which Tabletist does not support yet."),
                "{}",
                look.name
            );
            assert!(!harness.has("10.0.0.9 from ~/.ssh/config"), "{}", look.name);
        }
    }

    #[test]
    fn a_long_placeholder_from_the_config_keeps_the_dialog_as_wide() {
        // A user and a key file too long for their fields: a placeholder
        // must be cut to its field, not widen it and the dialog with it.
        let user = "deploy-automation-runner";
        let key = "/home/deploy/.ssh/keys/bookshop-production-bastion_ed25519";
        assert!(user.len() >= 24 && key.len() >= 50);
        for look in crate::theme::Look::ALL {
            for size in [egui::vec2(720.0, 480.0), egui::vec2(1280.0, 800.0)] {
                let mut save_right = Vec::new();
                for from_config in [false, true] {
                    let mut harness = Harness::with_size(size);
                    harness.set_look(look);
                    harness.press(Key::N, Modifiers::COMMAND);
                    harness.click(&look.label("PostgreSQL"));
                    harness.click("Connect through SSH tunnel");
                    let form = ssh_form(&mut harness);
                    form.ssh_auth = crate::model::SshAuthKind::KeyFile;
                    if from_config {
                        form.ssh_hosts = vec![config_host(
                            "runner",
                            HostConfig {
                                user: Some(user.into()),
                                identity_file: Some(key.into()),
                                ..HostConfig::default()
                            },
                        )];
                        form.ssh_host = "runner".into();
                    }
                    let tree = harness.settle();
                    let case = format!("{} at {size:?}", look.name);
                    if from_config {
                        // The placeholders are there, whole for whoever
                        // hears them.
                        let ssh_user = if look.terminal {
                            "ssh user"
                        } else {
                            "SSH user"
                        };
                        assert_eq!(
                            placeholder(&mut harness, ssh_user),
                            Some(format!("{user} (from ~/.ssh/config)")),
                            "{case}"
                        );
                        let key_file = if look.terminal {
                            "key file"
                        } else {
                            "Key file"
                        };
                        assert_eq!(
                            placeholder(&mut harness, key_file),
                            Some(format!("{key} (from ~/.ssh/config)")),
                            "{case}"
                        );
                    }
                    let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, harness.size);
                    for button in ["Test", "Save", "Cancel", "Save & Connect"] {
                        let at =
                            crate::testing::bounds(&tree, button, egui::accesskit::Role::Button)
                                .unwrap_or_else(|| panic!("{button}, {case}"));
                        assert!(
                            screen.contains_rect(at),
                            "{case}: {button} at {at:?} is pushed off the window"
                        );
                    }
                    let save = crate::testing::bounds(&tree, "Save", egui::accesskit::Role::Button)
                        .expect("Save");
                    save_right.push(save.right());
                }
                assert!(
                    (save_right[0] - save_right[1]).abs() <= 0.5,
                    "{} at {size:?}: the config's placeholders moved the dialog's right edge \
                     from {} to {}",
                    look.name,
                    save_right[0],
                    save_right[1]
                );
            }
        }
    }

    #[test]
    fn only_verify_full_offers_the_system_certificates() {
        let ca_hint = |harness: &mut Harness, tls| {
            match &mut harness.app.dialog {
                Some(crate::model::Dialog::Connection(form)) => form.tls = tls,
                other => panic!("{other:?}"),
            }
            let tree = harness.settle();
            ca_field(&tree).placeholder().map(str::to_owned)
        };
        let mut harness = Harness::new();
        harness.press(Key::N, Modifiers::COMMAND);
        harness.click("PostgreSQL");
        assert_eq!(
            ca_hint(&mut harness, tabletist_db::TlsMode::VerifyFull).as_deref(),
            Some("System certificates")
        );
        assert_eq!(
            ca_hint(&mut harness, tabletist_db::TlsMode::VerifyCa).as_deref(),
            Some("Required")
        );
    }

    #[test]
    fn sqlite_has_no_ssh_tunnel() {
        let mut harness = Harness::new();
        harness.press(Key::N, Modifiers::COMMAND);
        assert!(!harness.has("Connect through SSH tunnel"));
    }

    #[test]
    fn the_top_bar_says_the_connection_goes_through_ssh() {
        let mut harness = Harness::new();
        let (mut spec, _) = tabletist_db::ConnectSpec::from_url("postgres://me@db/app").unwrap();
        spec.ssh = Some(tabletist_db::SshSpec {
            host: "bastion".into(),
            port: Some(22),
            user: "ops".into(),
            auth: tabletist_db::SshAuth::Agent,
        });
        let saved = crate::connections::SavedConnection {
            id: crate::connections::ConnectionId::new(),
            name: "Prod".into(),
            environment: crate::env::Environment::Production,
            read_only: None,
            password: crate::connections::PasswordMode::None,
            ssh_secret: crate::connections::PasswordMode::None,
            spec,
        };
        let conn = saved.id.clone();
        harness.app.connections.upsert(saved);
        let tab = harness.app.active_tab_id();
        harness
            .app
            .apply(crate::model::Action::Connect { tab, conn });
        harness.settle();
        assert!(harness.has("via SSH bastion"));
    }

    /// A PostgreSQL tab with `tls`, connected with `encrypted` when given
    /// (still connecting otherwise).
    fn tls_top_bar(tls: tabletist_db::TlsMode, encrypted: Option<bool>) -> Harness {
        let mut harness = Harness::new();
        let (mut spec, _) = tabletist_db::ConnectSpec::from_url("postgres://me@db/app").unwrap();
        spec.tls = tls;
        let saved = crate::connections::SavedConnection {
            id: crate::connections::ConnectionId::new(),
            name: "Prod".into(),
            environment: crate::env::Environment::Production,
            read_only: None,
            password: crate::connections::PasswordMode::None,
            ssh_secret: crate::connections::PasswordMode::None,
            spec,
        };
        let conn = saved.id.clone();
        harness.app.connections.upsert(saved);
        let tab = harness.app.active_tab_id();
        harness
            .app
            .apply(crate::model::Action::Connect { tab, conn });
        if let Some(encrypted) = encrypted {
            let crate::backend::Command::Connect {
                session, request, ..
            } = *crate::testing::last_sent(&harness.app)
            else {
                panic!("expected Connect");
            };
            harness.app.apply(crate::model::Action::Backend(
                crate::backend::Event::Connected {
                    session,
                    request,
                    driver: tabletist_db::Driver::Postgres,
                    encrypted,
                    access: crate::testing::asked_access(&harness.app),
                },
            ));
        }
        harness.settle();
        harness
    }

    #[test]
    fn the_top_bar_says_how_far_tls_can_be_trusted() {
        use tabletist_db::TlsMode;
        for (tls, encrypted, text, warn) in [
            (TlsMode::Disable, None, "Not encrypted", true),
            (TlsMode::Disable, Some(false), "Not encrypted", true),
            (TlsMode::Prefer, None, "TLS, not verified", true),
            (TlsMode::Prefer, Some(true), "TLS, not verified", true),
            (TlsMode::Require, Some(true), "TLS, not verified", true),
            (
                TlsMode::VerifyCa,
                Some(true),
                "TLS, host name not checked",
                false,
            ),
            (TlsMode::VerifyFull, None, "TLS verified", false),
            (TlsMode::VerifyFull, Some(true), "TLS verified", false),
        ] {
            let mut harness = tls_top_bar(tls, encrypted);
            assert!(harness.has(text), "{tls:?} {encrypted:?}");
            let palette = harness.app.palette;
            let color = if warn {
                palette.warning
            } else {
                palette.success
            };
            assert_eq!(
                harness.painted_color(text),
                Some(color),
                "{tls:?} {encrypted:?}"
            );
        }
    }

    #[test]
    fn the_top_bar_says_when_prefer_fell_back_to_plain_text() {
        let mut harness = tls_top_bar(tabletist_db::TlsMode::Prefer, Some(false));
        assert!(harness.has("Not encrypted"));
        assert!(!harness.has("TLS, not verified"));
        assert_eq!(
            harness.painted_color("Not encrypted"),
            Some(harness.app.palette.warning)
        );
    }

    const INTERCEPT_WARNING: &str = "The password can be intercepted on the network. \
                                     To prevent it, verify the certificate and host, \
                                     or use an SSH tunnel.";

    #[test]
    fn the_dialog_warns_when_a_remote_password_is_not_protected() {
        use tabletist_db::TlsMode;
        let mut harness = Harness::new();
        harness.press(Key::N, Modifiers::COMMAND);
        harness.click("PostgreSQL");
        let set = |harness: &mut Harness, host: &str, tls: TlsMode, ssh: bool| {
            match &mut harness.app.dialog {
                Some(crate::model::Dialog::Connection(form)) => {
                    form.host = host.into();
                    form.tls = tls;
                    form.ssh = ssh;
                }
                other => panic!("{other:?}"),
            }
            harness.has(INTERCEPT_WARNING)
        };
        for tls in [TlsMode::Disable, TlsMode::Prefer, TlsMode::Require] {
            assert!(set(&mut harness, "db.example.com", tls, false), "{tls:?}");
            assert!(!set(&mut harness, "localhost", tls, false), "{tls:?}");
            assert!(!set(&mut harness, "db.example.com", tls, true), "{tls:?}");
        }
        for tls in [TlsMode::VerifyCa, TlsMode::VerifyFull] {
            assert!(!set(&mut harness, "db.example.com", tls, false), "{tls:?}");
        }
        assert!(set(&mut harness, "10.0.0.5", TlsMode::Prefer, false));
        assert_eq!(
            harness.painted_color(INTERCEPT_WARNING),
            Some(harness.app.palette.warning)
        );
    }

    #[test]
    fn the_footer_offers_count_and_then_shows_the_total() {
        let mut harness = Harness::new();
        let tab = harness.connect_fake();
        harness.app.apply(crate::model::Action::OpenObject {
            tab,
            object: tabletist_db::ObjectRef::new("main", "users"),
            kind: tabletist_db::ObjectKind::Table,
            pin: true,
        });
        harness.answer_rows(crate::testing::page(3, true));
        harness.click("Count");
        let (session, request) = match harness.app.backend.sent.last() {
            Some(crate::backend::Command::CountRows {
                session, request, ..
            }) => (*session, *request),
            other => panic!("{other:?}"),
        };
        harness.app.apply(crate::model::Action::Backend(
            crate::backend::Event::Count {
                session,
                request,
                result: Ok(1234),
            },
        ));
        assert!(harness.has("Rows 1–3 of 1,234"));
        assert!(!harness.has("Count"));
    }

    #[test]
    fn a_page_holding_every_row_offers_no_count() {
        let mut harness = Harness::new();
        harness.set_look(crate::theme::Look::macos());
        let tab = harness.connect_fake();
        harness.app.apply(crate::model::Action::OpenObject {
            tab,
            object: tabletist_db::ObjectRef::new("main", "users"),
            kind: tabletist_db::ObjectKind::Table,
            pin: true,
        });
        harness.answer_rows(crate::testing::page(3, false));
        assert!(harness.has("Rows 1–3 of 3"));
        assert!(!harness.has("Count"), "the total is already known");
    }

    #[test]
    fn command_f_opens_the_filter_bar_and_enter_applies_it() {
        let mut harness = Harness::new();
        let tab = harness.connect_fake();
        harness.app.apply(crate::model::Action::OpenObject {
            tab,
            object: tabletist_db::ObjectRef::new("main", "users"),
            kind: tabletist_db::ObjectKind::Table,
            pin: true,
        });
        harness.answer_rows(crate::testing::page(3, false));
        harness.press(Key::F, Modifiers::COMMAND);
        assert!(harness.has("Apply"));
        assert!(harness.has("Raw WHERE"));
        // The value field has focus: type and press Enter.
        harness.frame(vec![egui::Event::Text("30".into())]);
        harness.press(Key::Enter, Modifiers::NONE);
        assert!(matches!(
            harness.app.backend.sent.last(),
            Some(crate::backend::Command::FetchRows { query, .. }) if query.filters.len() == 1
        ));
    }

    /// A connected tab whose `main` schema shows `orders` and `users`.
    fn tree_harness() -> (Harness, crate::model::ConnTabId) {
        let mut harness = Harness::new();
        let tab = harness.connect_fake();
        let workspace = harness.app.workspace_mut(tab).unwrap();
        workspace.tree.schemas.value = Some(vec!["main".into()]);
        let node = workspace.tree.nodes.entry("main".into()).or_default();
        node.expanded = true;
        node.objects.value = Some(
            ["orders", "users"]
                .into_iter()
                .map(|name| tabletist_db::ObjectInfo {
                    name: name.into(),
                    kind: tabletist_db::ObjectKind::Table,
                    estimated_rows: None,
                })
                .collect(),
        );
        (harness, tab)
    }

    /// Gives the tab a database picker, as a PostgreSQL server with two
    /// databases would.
    fn with_database_picker(harness: &mut Harness, tab: crate::model::ConnTabId) {
        let workspace = harness.app.workspace_mut(tab).unwrap();
        workspace.spec.database = "tabletist".into();
        workspace.databases.value = Some(vec!["tabletist".into(), "postgres".into()]);
    }

    #[test]
    fn command_p_opens_quick_open_and_enter_opens_the_match() {
        let (mut harness, tab) = tree_harness();
        harness.press(Key::P, Modifiers::COMMAND);
        assert!(harness.has("Open table or view"));
        harness.frame(vec![egui::Event::Text("users".into())]);
        harness.press(Key::Enter, Modifiers::NONE);
        assert!(harness.app.dialog.is_none());
        let workspace = harness.app.workspace(tab).unwrap();
        let object = workspace.object_tab(workspace.active_tab.unwrap()).unwrap();
        assert_eq!(object.object.name, "users");
    }

    #[test]
    fn quick_open_shortens_long_names_and_shows_them_on_hover() {
        let (mut harness, tab) = tree_harness();
        let long = "an_uncommonly_long_table_name_that_runs_well_past_the_right_edge_of_the_quick_open_dialog_in_every_look";
        let objects = &mut harness
            .app
            .workspace_mut(tab)
            .unwrap()
            .tree
            .nodes
            .get_mut("main")
            .unwrap()
            .objects;
        objects
            .value
            .as_mut()
            .unwrap()
            .push(tabletist_db::ObjectInfo {
                name: long.into(),
                kind: tabletist_db::ObjectKind::Table,
                estimated_rows: None,
            });
        harness.press(Key::P, Modifiers::COMMAND);
        let mut count_on_hover = |name: &str| {
            let tree = harness.settle();
            let row = tree
                .nodes
                .iter()
                .find(|(_, node)| node.label() == Some(name))
                .and_then(|(_, node)| node.bounds())
                .unwrap_or_else(|| panic!("a row for {name}"));
            let over = egui::pos2(row.x0 as f32 + 20.0, ((row.y0 + row.y1) / 2.0) as f32);
            harness.frame(vec![egui::Event::PointerMoved(over)]);
            // Past the tooltip delay: each frame is 1/60 s.
            for _ in 0..60 {
                harness.frame(Vec::new());
            }
            let tree = harness.frame(Vec::new());
            crate::testing::labels(&tree)
                .iter()
                .filter(|label| *label == name)
                .count()
        };
        let full = format!("main.{long}");
        assert_eq!(count_on_hover(&full), 2, "the row and its tooltip");
        assert_eq!(
            count_on_hover("main.users"),
            1,
            "a short name has no tooltip"
        );
    }

    #[test]
    fn quick_open_explains_what_it_searches() {
        let mut harness = Harness::new();
        harness.connect_fake();
        harness.press(Key::P, Modifiers::COMMAND);
        assert!(harness.has("Searches schemas loaded in the sidebar"));
    }

    #[test]
    fn clicking_the_tree_sends_arrows_there_and_the_grid_takes_them_back() {
        let (mut harness, tab) = tree_harness();
        harness.click("orders");
        harness.answer_rows(crate::testing::page(3, false));
        assert_eq!(
            harness.app.workspace(tab).unwrap().pane,
            crate::model::Pane::Tree
        );
        harness.press(Key::ArrowDown, Modifiers::NONE);
        let workspace = harness.app.workspace(tab).unwrap();
        assert_eq!(
            workspace.tree.cursor,
            Some(crate::model::TreeNode::Object(
                tabletist_db::ObjectRef::new("main", "users"),
                tabletist_db::ObjectKind::Table
            ))
        );
        let object_tab = workspace.active_tab.unwrap();
        assert_eq!(
            workspace.object_tab(object_tab).unwrap().selection,
            None,
            "the grid did not move"
        );
        harness.app.apply(crate::model::Action::SelectCell {
            tab,
            id: object_tab,
            cell: crate::model::CellPos { row: 0, col: 0 },
        });
        harness.press(Key::ArrowDown, Modifiers::NONE);
        let workspace = harness.app.workspace(tab).unwrap();
        assert_eq!(workspace.pane, crate::model::Pane::Grid);
        assert_eq!(
            workspace.object_tab(object_tab).unwrap().selection,
            Some(crate::model::CellPos { row: 1, col: 0 })
        );
    }

    #[test]
    fn arrows_in_the_filter_bar_do_not_move_the_tree() {
        let (mut harness, tab) = tree_harness();
        harness.click("orders");
        harness.answer_rows(crate::testing::page(3, false));
        let before = harness.app.workspace(tab).unwrap().tree.cursor.clone();
        // Cmd/Ctrl+F focuses the filter bar's value field.
        harness.press(Key::F, Modifiers::COMMAND);
        harness.press(Key::ArrowDown, Modifiers::NONE);
        assert_eq!(harness.app.workspace(tab).unwrap().tree.cursor, before);
    }

    #[test]
    fn question_mark_opens_the_shortcuts_and_escape_closes_them() {
        let mut harness = Harness::new();
        harness.frame(vec![egui::Event::Text("?".into())]);
        assert!(harness.has("Keyboard shortcuts"));
        assert!(harness.has("Quick open"));
        assert!(harness.has("Format SQL"));
        harness.press(Key::Escape, Modifiers::NONE);
        assert!(harness.app.dialog.is_none());
    }

    #[test]
    fn the_shortcuts_keep_close_on_screen_in_the_smallest_window() {
        let size = egui::vec2(720.0, 480.0);
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, size);
        for look in crate::theme::Look::ALL {
            let mut harness = Harness::with_size(size);
            harness.set_look(look);
            harness.frame(vec![egui::Event::Text("?".into())]);
            let tree = harness.settle();
            let close = crate::testing::bounds(&tree, "Close", egui::accesskit::Role::Button)
                .expect("the Close button");
            assert!(
                screen.contains_rect(close),
                "{}: Close at {close:?}",
                look.name
            );
        }
    }

    #[test]
    fn typing_a_question_mark_in_a_field_does_not_open_help() {
        let mut harness = Harness::new();
        let tab = harness.connect_fake();
        harness.app.apply(crate::model::Action::OpenObject {
            tab,
            object: tabletist_db::ObjectRef::new("main", "users"),
            kind: tabletist_db::ObjectKind::Table,
            pin: true,
        });
        harness.answer_rows(crate::testing::page(3, false));
        // Cmd/Ctrl+F focuses the filter bar's value field.
        harness.press(Key::F, Modifiers::COMMAND);
        harness.frame(vec![egui::Event::Text("?".into())]);
        assert!(harness.app.dialog.is_none());
        let workspace = harness.app.workspace(tab).unwrap();
        let object = workspace.object_tab(workspace.active_tab.unwrap()).unwrap();
        assert_eq!(object.filter.rows[0].value, "?");
    }

    #[test]
    fn a_filtered_page_does_not_show_the_tables_estimate() {
        let mut harness = Harness::new();
        let tab = harness.connect_fake();
        harness.app.apply(crate::model::Action::OpenObject {
            tab,
            object: tabletist_db::ObjectRef::new("main", "users"),
            kind: tabletist_db::ObjectKind::Table,
            pin: true,
        });
        harness.answer_rows(crate::testing::page(3, true));
        let id = harness.app.workspace(tab).unwrap().active_tab.unwrap();
        let object = harness
            .app
            .workspace_mut(tab)
            .unwrap()
            .object_tab_mut(id)
            .unwrap();
        object.estimated_rows = Some(1_200_000);
        object.filter.rows = vec![crate::model::FilterRow {
            column: "id".into(),
            op: tabletist_db::FilterOp::Eq,
            value: "5".into(),
        }];
        harness.app.apply(crate::model::Action::ApplyFilters {
            tab,
            object_tab: id,
        });
        harness.answer_rows(crate::testing::page(3, true));
        assert!(harness.has("Rows 1–3"));
        assert!(!harness.has("Rows 1–3 of ~1.2M"));
    }

    #[test]
    fn the_tree_scrolls_to_the_keyboard_cursor() {
        let (mut harness, tab) = tree_harness();
        let names: Vec<String> = (0..200).map(|i| format!("t_{i:03}")).collect();
        let workspace = harness.app.workspace_mut(tab).unwrap();
        workspace.tree.nodes.get_mut("main").unwrap().objects.value = Some(
            names
                .iter()
                .map(|name| tabletist_db::ObjectInfo {
                    name: name.clone(),
                    kind: tabletist_db::ObjectKind::Table,
                    estimated_rows: None,
                })
                .collect(),
        );
        // One flat list: in groups, all would fold into `t_`.
        harness.app.workspace_mut(tab).unwrap().tree.flat = true;
        harness.settle();
        assert!(harness.has("t_000"));
        harness.app.apply(crate::model::Action::SetTreeCursor {
            tab,
            node: crate::model::TreeNode::Object(
                tabletist_db::ObjectRef::new("main", "t_000"),
                tabletist_db::ObjectKind::Table,
            ),
        });
        harness.press(Key::End, Modifiers::NONE);
        harness.settle();
        assert!(harness.has("t_199"), "the cursor row is on screen");
    }

    #[test]
    fn the_sidebar_keeps_its_width_frame_after_frame() {
        let (mut harness, _tab) = tree_harness();
        let right_edge = |harness: &mut Harness| {
            let tree = harness.frame(Vec::new());
            let id = crate::testing::node(&tree, "Refresh objects", egui::accesskit::Role::Button)
                .expect("the refresh button");
            let node = tree.nodes.iter().find(|(n, _)| *n == id).unwrap();
            node.1.bounds().expect("bounds").x1
        };
        let first = right_edge(&mut harness);
        for _ in 0..300 {
            harness.frame(Vec::new());
        }
        let later = right_edge(&mut harness);
        assert!(
            (later - first).abs() < 0.5,
            "the sidebar grew from {first} to {later}"
        );
    }

    fn bounds_of(tree: &egui::accesskit::TreeUpdate, label: &str) -> egui::accesskit::Rect {
        let id = crate::testing::node(tree, label, egui::accesskit::Role::Button)
            .unwrap_or_else(|| panic!("no {label}"));
        tree.nodes
            .iter()
            .find(|(n, _)| *n == id)
            .and_then(|(_, node)| node.bounds())
            .expect("bounds")
    }

    /// A macOS unified title bar: 40 pt tall, window buttons up to 80 pt.
    fn mac_title_bar(harness: &mut Harness) {
        harness.app.titlebar = crate::app::TitleBar {
            height: 40.0,
            inset: 80.0,
        };
    }

    #[test]
    fn several_connections_share_the_title_bar_with_the_mac_window_buttons() {
        let mut harness = Harness::new();
        mac_title_bar(&mut harness);
        harness.connect_fake();
        connect_another(&mut harness, "Second");
        let tree = harness.settle();
        let button = bounds_of(&tree, "Connections");
        assert!(
            button.x0 >= 80.0,
            "the bar starts after the buttons: {button:?}"
        );
        let chip = bounds_of(&tree, "Switch to Fixture · dev");
        assert!(chip.x0 > button.x1, "the chips follow: {chip:?}");
    }

    /// The middle of the node labelled (or valued) `label`.
    fn middle_of(tree: &egui::accesskit::TreeUpdate, label: &str) -> f64 {
        let bounds = tree
            .nodes
            .iter()
            .find(|(_, node)| node.label().or_else(|| node.value()) == Some(label))
            .and_then(|(_, node)| node.bounds())
            .unwrap_or_else(|| panic!("no {label}"));
        (bounds.y0 + bounds.y1) / 2.0
    }

    #[test]
    fn the_mac_window_buttons_centre_on_the_line_of_the_bar_that_leads() {
        let mut harness = Harness::new();
        harness.set_look(crate::theme::Look::macos());
        mac_title_bar(&mut harness);
        let line = |harness: &Harness| f64::from(super::window_buttons_line(&harness.app, 1.0));

        // The picker alone: its header, taller than the title bar.
        let tree = harness.settle();
        let header = middle_of(&tree, "New connection");
        assert!(
            (line(&harness) - header).abs() < 1.0,
            "the picker's header at {header}, the buttons at {}",
            line(&harness)
        );

        // One connection: its bar, under the environment stripe.
        harness.connect_fake();
        let tree = harness.settle();
        let bar = middle_of(&tree, "Connections");
        assert!(
            (line(&harness) - bar).abs() < 1.0,
            "the connection bar at {bar}, the buttons at {}",
            line(&harness)
        );
        assert!(line(&harness) > 20.0, "below the title bar's own middle");

        // Several: the same bar, a chip for each.
        connect_another(&mut harness, "Second");
        let tree = harness.settle();
        let bar = middle_of(&tree, "Connections");
        assert!(
            (line(&harness) - bar).abs() < 1.0,
            "the connection bar at {bar}, the buttons at {}",
            line(&harness)
        );

        // The picker again, with the connections open behind it: its header.
        harness.press(Key::O, Modifiers::COMMAND);
        let tree = harness.settle();
        let header = middle_of(&tree, "New connection");
        assert!(
            (line(&harness) - header).abs() < 1.0,
            "the picker's header at {header}, the buttons at {}",
            line(&harness)
        );
    }

    #[test]
    fn the_mac_title_bar_is_measured_in_window_points_not_zoomed_ones() {
        let mut harness = Harness::new();
        mac_title_bar(&mut harness);
        harness.connect_fake();
        harness.ctx.set_zoom_factor(2.0);
        let tree = harness.settle();
        let button = bounds_of(&tree, "Connections");
        // 80 window points are 40 egui points at 2x zoom: the bar starts
        // past those, well short of 80.
        assert!(
            button.x0 >= 40.0 && button.x0 < 80.0,
            "the bar starts after the buttons: {button:?}"
        );
    }

    #[test]
    fn alone_the_connection_bar_starts_after_the_mac_window_buttons() {
        let mut harness = Harness::new();
        mac_title_bar(&mut harness);
        harness.connect_fake();
        let tree = harness.settle();
        let name = tree
            .nodes
            .iter()
            .find(|(_, node)| node.label().or_else(|| node.value()) == Some("Fixture"))
            .and_then(|(_, node)| node.bounds())
            .expect("the connection's name");
        assert!(name.x0 >= 80.0, "after the buttons: {name:?}");
        assert!(name.y1 < 40.0, "in the title bar: {name:?}");
    }

    #[test]
    fn the_space_beside_the_mac_window_buttons_moves_the_window() {
        let mut harness = Harness::new();
        mac_title_bar(&mut harness);
        harness.settle();
        let at = egui::pos2(76.0, 20.0);
        harness.frame(vec![egui::Event::PointerMoved(at)]);
        harness.frame(vec![egui::Event::PointerButton {
            pos: at,
            button: egui::PointerButton::Primary,
            pressed: true,
            modifiers: Modifiers::NONE,
        }]);
        harness.frame(vec![egui::Event::PointerMoved(at + egui::vec2(30.0, 0.0))]);
        assert!(
            harness
                .viewport_commands
                .contains(&egui::ViewportCommand::StartDrag)
        );
    }

    #[test]
    fn dragging_the_empty_tab_bar_moves_the_window() {
        let mut harness = Harness::new();
        harness.settle();
        let at = egui::pos2(900.0, 18.0);
        harness.frame(vec![egui::Event::PointerMoved(at)]);
        harness.frame(vec![egui::Event::PointerButton {
            pos: at,
            button: egui::PointerButton::Primary,
            pressed: true,
            modifiers: Modifiers::NONE,
        }]);
        harness.frame(vec![egui::Event::PointerMoved(at + egui::vec2(30.0, 0.0))]);
        assert!(
            harness
                .viewport_commands
                .contains(&egui::ViewportCommand::StartDrag),
            "{:?}",
            harness.viewport_commands
        );
    }

    /// A tab with a page of `users` whose connection is then lost.
    fn lost(harness: &mut Harness) -> crate::model::ConnTabId {
        let tab = harness.connect_fake();
        harness.click("users");
        harness.answer_rows(crate::testing::page(3, false));
        let session = harness.app.workspace(tab).unwrap().session;
        harness.app.apply(crate::model::Action::Backend(
            crate::backend::Event::Disconnected {
                session,
                error: tabletist_db::Error::ConnectionLost("server closed the connection".into()),
            },
        ));
        tab
    }

    #[test]
    fn a_lost_connection_keeps_what_was_on_screen_and_offers_the_way_back() {
        for look in crate::theme::Look::ALL {
            let mut harness = Harness::new();
            harness.set_look(look);
            let tab = lost(&mut harness);
            let lead = format!(
                "{} Fixture · dev {}",
                look.label("Connection to"),
                look.label("lost.")
            );
            assert!(harness.has(&lead), "{lead} in {}", look.name);
            assert!(
                harness.has("the connection was lost: server closed the connection"),
                "{}",
                look.name
            );
            assert!(harness.has("Row 1"), "{}", look.name);
            harness.click("Reconnect");
            assert!(matches!(
                harness.app.workspace(tab).unwrap().status,
                crate::model::SessionStatus::Connecting { .. }
            ));
            let again = format!("{} Fixture…", look.label("Reconnecting to"));
            assert!(harness.has(&again), "{again} in {}", look.name);
            assert!(!harness.has("Reconnect"), "{}", look.name);
            assert!(harness.has("Row 1"), "{}", look.name);
        }
    }

    /// A terminal button's text is muted only beside its key, which then
    /// carries it: muted and alone, it would read as one that is off.
    #[test]
    fn a_terminal_button_without_a_key_reads_as_text_and_one_with_a_key_shows_it() {
        let look = crate::theme::Look::omarchy();
        let mut harness = Harness::new();
        harness.set_look(look);
        lost(&mut harness);
        harness.settle();
        let text = harness.app.palette.text;
        // The strip's lead is ordinary text; Reconnect has no key.
        let lead = format!(
            "{} Fixture · dev {}",
            look.label("Connection to"),
            look.label("lost.")
        );
        assert_eq!(harness.painted_color(&lead), Some(text));
        assert_eq!(harness.painted_color(&look.label("Reconnect")), Some(text));

        // A connect's Cancel shows the key that gives up.
        let mut harness = Harness::new();
        harness.set_look(look);
        add_saved(&mut harness, "Production");
        harness.click("Connect to Production");
        harness.settle();
        for piece in [look.label("Cancel"), "esc".to_owned()] {
            assert!(
                harness.painted.iter().any(|(text, _)| *text == piece),
                "{piece}: {:?}",
                harness.painted
            );
        }
        // The key carries the button, in the colour a button without one
        // has its text in, and the text stands back from it.
        let key = harness.painted_color("esc");
        assert_eq!(key, Some(harness.app.palette.text));
        assert_ne!(harness.painted_color(&look.label("Cancel")), key);
    }

    /// Fails what a refresh of `tab` asked for (its rows and, when it had
    /// one, its structure) as a connection that is gone does, then reports
    /// the loss: the order the backend sends them in.
    fn lose_on_refresh(
        harness: &mut Harness,
        tab: crate::model::ConnTabId,
        error: &tabletist_db::Error,
    ) {
        use crate::backend::Event;
        let workspace = harness.app.workspace(tab).unwrap();
        let session = workspace.session;
        let object = workspace.active_object_tab().unwrap();
        let (rows, structure) = (object.rows.pending, object.structure.pending);
        let request = rows.expect("the refresh fetched the rows");
        let result = Err(error.clone());
        harness
            .app
            .apply(crate::model::Action::Backend(Event::Rows {
                session,
                request,
                result,
            }));
        if let Some(request) = structure {
            let result = Err(error.clone());
            harness
                .app
                .apply(crate::model::Action::Backend(Event::Structure {
                    session,
                    request,
                    result,
                }));
        }
        let error = error.clone();
        harness
            .app
            .apply(crate::model::Action::Backend(Event::Disconnected {
                session,
                error,
            }));
    }

    /// How many times `text` is on screen.
    fn times_said(harness: &mut Harness, text: &str) -> usize {
        let tree = harness.settle();
        let said = crate::testing::labels(&tree);
        said.iter().filter(|said| *said == text).count()
    }

    #[test]
    fn a_refresh_that_finds_the_connection_lost_keeps_the_page() {
        let error = tabletist_db::Error::ConnectionLost("server closed the connection".into());
        for look in crate::theme::Look::ALL {
            let mut harness = Harness::new();
            harness.set_look(look);
            let tab = harness.connect_fake();
            harness.click("users");
            harness.answer_rows(crate::testing::page(3, false));
            // The keys are the tree's after a click in it.
            focus_grid(&mut harness, tab);
            let before = fetches(&harness);
            harness.press(Key::R, Modifiers::COMMAND);
            assert_eq!(fetches(&harness), before + 1, "{}", look.name);
            lose_on_refresh(&mut harness, tab, &error);
            // The strip says it, once, and the rows stay under it.
            assert!(harness.has("Reconnect"), "{}", look.name);
            assert!(harness.has("Row 1"), "{}", look.name);
            assert_eq!(
                times_said(&mut harness, &error.to_string()),
                1,
                "{}",
                look.name
            );
        }
    }

    #[test]
    fn a_refresh_that_finds_the_connection_lost_keeps_the_structure() {
        let error = tabletist_db::Error::ConnectionLost("server closed the connection".into());
        let mut harness = Harness::new();
        let tab = with_page(&mut harness);
        harness.click("Structure");
        harness.answer_structure(tabletist_db::Structure {
            indexes: vec![tabletist_db::IndexInfo {
                name: "users_email_idx".into(),
                columns: vec!["email".into()],
                key_columns: Some(vec!["email".into()]),
                unique: true,
                primary: false,
                method: None,
                partial: false,
            }],
            ..Default::default()
        });
        assert!(harness.has("users_email_idx"));
        harness.app.apply(crate::model::Action::Refresh(tab));
        lose_on_refresh(&mut harness, tab, &error);
        assert!(harness.has("Reconnect"));
        assert!(harness.has("users_email_idx"));
        assert_eq!(times_said(&mut harness, &error.to_string()), 1);
    }

    #[test]
    fn a_lost_connection_with_no_page_to_show_says_what_failed() {
        let error = tabletist_db::Error::ConnectionLost("server closed the connection".into());
        let mut harness = Harness::new();
        let tab = harness.connect_fake();
        // The first fetch of the table is what finds the loss.
        harness.click("users");
        lose_on_refresh(&mut harness, tab, &error);
        assert!(harness.has("Retry"));
    }

    /// Cancels what `tab`'s object is fetching and answers each request as
    /// the backend answers a cancelled one.
    fn cancel_fetches(harness: &mut Harness, tab: crate::model::ConnTabId) {
        use crate::backend::Event;
        let workspace = harness.app.workspace(tab).unwrap();
        let session = workspace.session;
        let object = workspace.active_object_tab().unwrap();
        let (rows, structure) = (object.rows.pending, object.structure.pending);
        assert!(rows.or(structure).is_some(), "nothing is being fetched");
        harness.app.apply(crate::model::Action::CancelQuery(tab));
        if let Some(request) = rows {
            harness
                .app
                .apply(crate::model::Action::Backend(Event::Rows {
                    session,
                    request,
                    result: Err(tabletist_db::Error::Cancelled),
                }));
        }
        if let Some(request) = structure {
            harness
                .app
                .apply(crate::model::Action::Backend(Event::Structure {
                    session,
                    request,
                    result: Err(tabletist_db::Error::Cancelled),
                }));
        }
    }

    #[test]
    fn a_cancelled_refresh_leaves_the_page_as_it_was() {
        for look in crate::theme::Look::ALL {
            let mut harness = Harness::new();
            harness.set_look(look);
            let tab = with_page(&mut harness);
            harness.app.apply(crate::model::Action::Refresh(tab));
            cancel_fetches(&mut harness, tab);
            assert!(harness.has("Row 1"), "{}", look.name);
            assert!(!harness.has("Retry"), "{}", look.name);
        }
    }

    #[test]
    fn a_cancelled_refresh_leaves_the_structure_as_it_was() {
        let mut harness = Harness::new();
        let tab = with_page(&mut harness);
        harness.click("Structure");
        harness.answer_structure(tabletist_db::Structure {
            indexes: vec![tabletist_db::IndexInfo {
                name: "users_email_idx".into(),
                columns: vec!["email".into()],
                key_columns: Some(vec!["email".into()]),
                unique: true,
                primary: false,
                method: None,
                partial: false,
            }],
            ..Default::default()
        });
        harness.app.apply(crate::model::Action::Refresh(tab));
        cancel_fetches(&mut harness, tab);
        assert!(harness.has("users_email_idx"));
        assert!(!harness.has("Retry"));
    }

    #[test]
    fn a_cancelled_first_fetch_says_so_and_offers_retry() {
        let mut harness = Harness::new();
        let tab = harness.connect_fake();
        // Nothing is held to go back to: the card is all there is to show.
        harness.click("users");
        cancel_fetches(&mut harness, tab);
        assert!(harness.has(&tabletist_db::Error::Cancelled.to_string()));
        assert!(harness.has("Retry"));
    }

    /// Answers the rows (or, with `structure`, the structure) `tab`'s
    /// object is fetching with `error`.
    fn fail_fetch(
        harness: &mut Harness,
        tab: crate::model::ConnTabId,
        structure: bool,
        error: tabletist_db::Error,
    ) {
        use crate::backend::Event;
        let workspace = harness.app.workspace(tab).unwrap();
        let session = workspace.session;
        let object = workspace.active_object_tab().unwrap();
        let event = if structure {
            Event::Structure {
                session,
                request: object.structure.pending.expect("a structure on its way"),
                result: Err(error),
            }
        } else {
            Event::Rows {
                session,
                request: object.rows.pending.expect("rows on their way"),
                result: Err(error),
            }
        };
        harness.app.apply(crate::model::Action::Backend(event));
    }

    /// Refresh over an error box is a retry by another name: given up, it
    /// leaves the error's "cancelled", not the rows from before the error.
    #[test]
    fn a_cancelled_refresh_over_an_error_does_not_bring_back_the_page() {
        let cancelled = tabletist_db::Error::Cancelled.to_string();
        let mut harness = Harness::new();
        let tab = with_page(&mut harness);
        harness.app.apply(crate::model::Action::Refresh(tab));
        let error = tabletist_db::Error::query("no such column: nope");
        fail_fetch(&mut harness, tab, false, error);
        assert!(harness.has("no such column: nope"));
        harness.app.apply(crate::model::Action::Refresh(tab));
        cancel_fetches(&mut harness, tab);
        assert!(!harness.has("Row 1"));
        assert!(harness.has(&cancelled));
        assert!(harness.has("Retry"));
    }

    #[test]
    fn a_cancelled_retry_does_not_bring_back_the_page_under_the_error() {
        let cancelled = tabletist_db::Error::Cancelled.to_string();
        for look in crate::theme::Look::ALL {
            let mut harness = Harness::new();
            harness.set_look(look);
            let tab = with_page(&mut harness);
            harness.app.apply(crate::model::Action::Refresh(tab));
            let error = tabletist_db::Error::query("no such column: nope");
            fail_fetch(&mut harness, tab, false, error);
            assert!(harness.has("no such column: nope"), "{}", look.name);
            assert!(!harness.has("Row 1"), "{}", look.name);
            // The error was the last thing on screen: a retry given up
            // has no page to go back to.
            harness.click("Retry");
            cancel_fetches(&mut harness, tab);
            assert!(!harness.has("Row 1"), "{}", look.name);
            assert!(harness.has(&cancelled), "{}", look.name);
            assert!(harness.has("Retry"), "{}", look.name);
        }
    }

    #[test]
    fn a_cancelled_retry_does_not_bring_back_the_structure_under_the_error() {
        let cancelled = tabletist_db::Error::Cancelled.to_string();
        for look in crate::theme::Look::ALL {
            let mut harness = Harness::new();
            harness.set_look(look);
            let tab = with_page(&mut harness);
            harness.click("Structure");
            harness.answer_structure(tabletist_db::Structure {
                indexes: vec![tabletist_db::IndexInfo {
                    name: "users_email_idx".into(),
                    columns: vec!["email".into()],
                    key_columns: Some(vec!["email".into()]),
                    unique: true,
                    primary: false,
                    method: None,
                    partial: false,
                }],
                ..Default::default()
            });
            assert!(harness.has("users_email_idx"), "{}", look.name);
            harness.app.apply(crate::model::Action::Refresh(tab));
            harness.answer_rows(crate::testing::page(5, false));
            let error = tabletist_db::Error::query("permission denied for table users");
            fail_fetch(&mut harness, tab, true, error);
            assert!(
                harness.has("permission denied for table users"),
                "{}",
                look.name
            );
            assert!(!harness.has("users_email_idx"), "{}", look.name);
            harness.click("Retry");
            cancel_fetches(&mut harness, tab);
            assert!(!harness.has("users_email_idx"), "{}", look.name);
            assert!(harness.has(&cancelled), "{}", look.name);
            assert!(harness.has("Retry"), "{}", look.name);
        }
    }

    #[test]
    fn a_lost_connection_can_be_left() {
        use egui::accesskit;
        for look in crate::theme::Look::ALL {
            let mut harness = Harness::new();
            harness.set_look(look);
            let tab = lost(&mut harness);
            // The connection bar has a Disconnect of its own: the strip's
            // is the lower one.
            let tree = harness.settle();
            let buttons = buttons_named(&tree, "Disconnect");
            assert_eq!(buttons.len(), 2, "{}", look.name);
            harness.frame(vec![egui::Event::AccessKitActionRequest(
                accesskit::ActionRequest {
                    target_tree: accesskit::TreeId::ROOT,
                    target_node: buttons[1].0,
                    action: accesskit::Action::Click,
                    data: None,
                },
            )]);
            harness.settle();
            assert!(harness.app.workspace(tab).is_none(), "{}", look.name);
        }
    }

    #[test]
    fn an_empty_schema_says_so() {
        let (mut harness, tab) = tree_harness();
        harness
            .app
            .workspace_mut(tab)
            .unwrap()
            .tree
            .nodes
            .get_mut("main")
            .unwrap()
            .objects
            .value = Some(Vec::new());
        assert!(harness.has("No tables or views"));
    }

    /// Notes a person reads (not decoration like NULL) use the secondary
    /// color, which meets the 4.5:1 text minimum; dim does not.
    #[test]
    fn informational_notes_are_readable_not_dim() {
        let (mut harness, tab) = tree_harness();
        let secondary = harness.app.palette.secondary;
        harness
            .app
            .workspace_mut(tab)
            .unwrap()
            .tree
            .nodes
            .get_mut("main")
            .unwrap()
            .objects
            .value = Some(Vec::new());
        harness.settle();
        assert_eq!(harness.painted_color("No tables or views"), Some(secondary));

        let (mut harness, _tab) = tree_harness();
        harness.click("orders");
        let mut page = crate::testing::page(3, true);
        page.ordered_by_key = false;
        harness.answer_rows(page);
        harness.click("Count");
        harness.settle();
        assert_eq!(harness.painted_color("Counting…"), Some(secondary));
        assert_eq!(harness.painted_color("Unordered"), Some(secondary));

        harness.press(Key::P, Modifiers::COMMAND);
        harness.settle();
        assert_eq!(
            harness.painted_color("Searches schemas loaded in the sidebar"),
            Some(secondary)
        );
    }

    /// `tree_harness` with the `main` schema loaded and empty.
    fn empty_schema_harness() -> (Harness, crate::model::ConnTabId) {
        let (mut harness, tab) = tree_harness();
        harness
            .app
            .workspace_mut(tab)
            .unwrap()
            .tree
            .nodes
            .get_mut("main")
            .unwrap()
            .objects
            .value = Some(Vec::new());
        (harness, tab)
    }

    #[test]
    fn an_empty_schema_offers_a_refresh() {
        let (mut harness, _tab) = empty_schema_harness();
        let before = harness.app.backend.sent.len();
        harness.click("Refresh");
        assert!(
            harness.app.backend.sent[before..].iter().any(
                |c| matches!(c, crate::backend::Command::ListObjects { schema, .. } if schema == "main")
            ),
            "the empty schema reloads its objects"
        );
    }

    /// `tree_harness` after the schema list came back empty.
    fn no_schemas_harness() -> (Harness, crate::model::ConnTabId) {
        let (mut harness, tab) = tree_harness();
        harness.app.workspace_mut(tab).unwrap().tree.schemas.value = Some(Vec::new());
        (harness, tab)
    }

    #[test]
    fn no_schemas_offers_a_refresh_and_explains_why() {
        let (mut harness, _tab) = no_schemas_harness();
        assert!(harness.has("This database has no schemas you can see."));
        assert!(harness.has("The connected user may lack the privileges to list them."));
        assert!(!harness.has("Or pick another database in the top bar."));
        let before = harness.app.backend.sent.len();
        harness.click("Refresh");
        assert!(
            harness.app.backend.sent[before..]
                .iter()
                .any(|c| matches!(c, crate::backend::Command::ListSchemas { .. })),
            "the tree reloads its schemas"
        );
    }

    #[test]
    fn no_schemas_suggests_another_database_when_there_is_a_picker() {
        let (mut harness, tab) = no_schemas_harness();
        with_database_picker(&mut harness, tab);
        assert!(harness.has("Or pick another database in the top bar."));
    }

    #[test]
    fn no_rows_under_a_filter_offers_to_clear_it() {
        let mut harness = Harness::new();
        let tab = harness.connect_fake();
        harness.app.apply(crate::model::Action::OpenObject {
            tab,
            object: tabletist_db::ObjectRef::new("main", "users"),
            kind: tabletist_db::ObjectKind::Table,
            pin: true,
        });
        harness.answer_rows(crate::testing::page(3, false));
        let id = harness.app.workspace(tab).unwrap().active_tab.unwrap();
        harness
            .app
            .workspace_mut(tab)
            .unwrap()
            .object_tab_mut(id)
            .unwrap()
            .filter
            .rows = vec![crate::model::FilterRow {
            column: "id".into(),
            op: tabletist_db::FilterOp::Eq,
            value: "999".into(),
        }];
        harness.app.apply(crate::model::Action::ApplyFilters {
            tab,
            object_tab: id,
        });
        harness.answer_rows(crate::testing::page(0, false));
        let title = harness.app.look.label("No rows match the filter");
        assert!(harness.has(&title));
        harness.click("Clear filters");
        assert!(matches!(
            harness.app.backend.sent.last(),
            Some(crate::backend::Command::FetchRows { query, .. }) if query.filters.is_empty()
        ));
    }

    /// The fixture's `users` table, open with a page of no rows.
    fn empty_users(harness: &mut Harness) -> crate::model::ConnTabId {
        let tab = harness.connect_fake();
        harness.app.apply(crate::model::Action::OpenObject {
            tab,
            object: tabletist_db::ObjectRef::new("main", "users"),
            kind: tabletist_db::ObjectKind::Table,
            pin: true,
        });
        harness.answer_rows(crate::testing::page(0, false));
        tab
    }

    #[test]
    fn an_empty_table_keeps_its_columns_and_says_so() {
        for look in crate::theme::Look::ALL {
            let mut harness = Harness::new();
            harness.set_look(look);
            empty_users(&mut harness);
            let title = format!("{} users", look.label("No rows in"));
            assert!(harness.has(&title), "{title} in {}", look.name);
            assert!(
                harness.has(&look.label("The table exists and is empty.")),
                "{}",
                look.name
            );
            // The structure stays readable: the grid's header (a button,
            // as it sorts) is there with no rows under it.
            let tree = harness.settle();
            assert!(
                crate::testing::node(&tree, "email", egui::accesskit::Role::Button).is_some(),
                "{}",
                look.name
            );
        }
    }

    #[test]
    fn an_estimate_of_no_rows_is_not_given_as_the_tables_size() {
        let mut harness = Harness::new();
        let tab = harness.connect_fake();
        harness.click("users");
        harness.answer_rows(crate::testing::page(3, false));
        let id = harness.app.workspace(tab).unwrap().active_tab.unwrap();
        let workspace = harness.app.workspace_mut(tab).unwrap();
        let object = workspace.object_tab_mut(id).unwrap();
        // What an older PostgreSQL says of a table it never analysed.
        object.estimated_rows = Some(0);
        object.filter.rows = vec![crate::model::FilterRow {
            column: "id".into(),
            op: tabletist_db::FilterOp::Eq,
            value: "999".into(),
        }];
        harness.app.apply(crate::model::Action::ApplyFilters {
            tab,
            object_tab: id,
        });
        harness.answer_rows(crate::testing::page(0, false));
        assert!(harness.has("No rows match the filter"));
        assert!(harness.has("None matches id = 999."));
    }

    #[test]
    fn a_table_opened_empty_fits_its_columns_to_the_rows_that_come() {
        use egui::accesskit::Role;
        let email = |harness: &mut Harness| {
            let tree = harness.settle();
            let header = crate::testing::bounds(&tree, "email", Role::Button);
            header.expect("the email header").width()
        };
        for look in crate::theme::Look::ALL {
            // Opened with rows: the columns fit them.
            let mut harness = Harness::new();
            harness.set_look(look);
            let tab = harness.connect_fake();
            harness.app.apply(crate::model::Action::OpenObject {
                tab,
                object: tabletist_db::ObjectRef::new("main", "users"),
                kind: tabletist_db::ObjectKind::Table,
                pin: true,
            });
            harness.answer_rows(crate::testing::page(3, false));
            let fitted = email(&mut harness);
            // Opened empty, the headers are all there is to fit.
            let mut harness = Harness::new();
            harness.set_look(look);
            let tab = empty_users(&mut harness);
            assert!(email(&mut harness) < fitted, "{}", look.name);
            // A refresh that finds rows fits the columns to them.
            harness.app.apply(crate::model::Action::Refresh(tab));
            harness.answer_rows(crate::testing::page(3, false));
            assert_eq!(email(&mut harness), fitted, "{}", look.name);
            // And a page with no rows after that leaves them as they are.
            harness.app.apply(crate::model::Action::Refresh(tab));
            harness.answer_rows(crate::testing::page(0, false));
            assert_eq!(email(&mut harness), fitted, "{}", look.name);
        }
    }

    #[test]
    fn a_page_past_the_last_row_says_so_and_offers_the_one_before() {
        use egui::accesskit::{self, Role};
        for look in crate::theme::Look::ALL {
            // Rows matched on the pages before, under a filter too.
            for filtered in [false, true] {
                let mut harness = Harness::new();
                harness.set_look(look);
                let tab = harness.connect_fake();
                harness.app.apply(crate::model::Action::OpenObject {
                    tab,
                    object: tabletist_db::ObjectRef::new("main", "users"),
                    kind: tabletist_db::ObjectKind::Table,
                    pin: true,
                });
                harness.answer_rows(crate::testing::page(300, true));
                let object_tab = harness.app.workspace(tab).unwrap().active_tab.unwrap();
                if filtered {
                    let workspace = harness.app.workspace_mut(tab).unwrap();
                    let object = workspace.object_tab_mut(object_tab).unwrap();
                    object.filter.rows = vec![crate::model::FilterRow {
                        column: "id".into(),
                        op: tabletist_db::FilterOp::Gt,
                        value: "0".into(),
                    }];
                    harness
                        .app
                        .apply(crate::model::Action::ApplyFilters { tab, object_tab });
                    harness.answer_rows(crate::testing::page(300, true));
                }
                harness
                    .app
                    .apply(crate::model::Action::NextPage { tab, object_tab });
                // The rows that were past the first page are gone by now.
                harness.answer_rows(crate::testing::page(0, false));
                let said = format!("{} (filtered: {filtered})", look.name);
                let title = format!("{} users", look.label("No more rows in"));
                assert!(harness.has(&title), "{title} in {said}");
                assert!(
                    harness.has(&look.label("This page is past the last row.")),
                    "{said}"
                );
                for other in ["The table exists and is empty.", "No rows match the filter"] {
                    assert!(!harness.has(&look.label(other)), "{other} in {said}");
                }
                // The footer pages too, where there is one: the state's own
                // button is the upper one, under its title.
                let tree = harness.settle();
                let title = crate::testing::bounds(&tree, &title, Role::Label).unwrap();
                let (button, at) = buttons_named(&tree, "Previous page")[0];
                assert!(at.top() >= title.bottom(), "{said}");
                harness.frame(vec![egui::Event::AccessKitActionRequest(
                    accesskit::ActionRequest {
                        target_tree: accesskit::TreeId::ROOT,
                        target_node: button,
                        action: accesskit::Action::Click,
                        data: None,
                    },
                )]);
                harness.settle();
                // The page before, with the filter it had.
                assert!(
                    matches!(
                        harness.app.backend.sent.last(),
                        Some(Command::FetchRows { query, .. })
                            if query.offset == 0 && query.filters.len() == usize::from(filtered)
                    ),
                    "{said}"
                );
            }
        }
    }

    #[test]
    fn reload_under_an_empty_table_fetches_again_and_takes_the_pointer() {
        let mut harness = Harness::new();
        empty_users(&mut harness);
        let before = fetches(&harness);
        // With the pointer, not through AccessKit: the grid under the
        // button must not take the click.
        let tree = harness.settle();
        let button =
            crate::testing::bounds(&tree, "Reload", egui::accesskit::Role::Button).unwrap();
        click_at(&mut harness, button.center());
        assert_eq!(fetches(&harness), before + 1);
    }

    #[test]
    fn a_reload_that_lasts_shows_its_wait_in_place_of_the_empty_state() {
        for look in crate::theme::Look::ALL {
            // The reload finds rows, or none again.
            for rows in [3, 0] {
                let mut harness = Harness::new();
                harness.set_look(look);
                let tab = empty_users(&mut harness);
                let title = format!("{} users", look.label("No rows in"));
                assert!(harness.has(&title), "{}", look.name);
                harness.click("Reload");
                age_fetch(&mut harness, tab);
                // The box says what is happening: nothing lies under it.
                assert!(harness.has(&look.label("Running query…")), "{}", look.name);
                assert!(!harness.has(&title), "{}", look.name);
                harness.answer_rows(crate::testing::page(rows, false));
                assert_eq!(harness.has(&title), rows == 0, "{} {rows}", look.name);
            }
        }
    }

    #[test]
    fn filters_that_match_nothing_are_named_and_the_last_one_can_go() {
        let mut harness = Harness::new();
        harness.set_look(crate::theme::Look::macos());
        let tab = harness.connect_fake();
        harness.click("users");
        harness.answer_rows(crate::testing::page(3, false));
        let id = harness.app.workspace(tab).unwrap().active_tab.unwrap();
        let row = |column: &str, value: &str| crate::model::FilterRow {
            column: column.into(),
            op: tabletist_db::FilterOp::Eq,
            value: value.into(),
        };
        harness
            .app
            .workspace_mut(tab)
            .unwrap()
            .object_tab_mut(id)
            .unwrap()
            .filter
            .rows = vec![row("id", "999"), row("email", "nobody")];
        harness.app.apply(crate::model::Action::ApplyFilters {
            tab,
            object_tab: id,
        });
        harness.answer_rows(crate::testing::page(0, false));
        assert!(harness.has("No rows match 2 filters"));
        // The fixture's estimate for `users`.
        assert!(
            harness
                .has("users has about 1,200,000 rows. None matches id = 999 and email = nobody.")
        );
        harness.click("Remove last filter");
        assert!(matches!(
            harness.app.backend.sent.last(),
            Some(Command::FetchRows { query, .. })
                if query.filters.len() == 1 && query.filters[0].column == "id"
        ));
    }

    #[test]
    fn errors_are_described_in_plain_words() {
        use tabletist_db::{Error, SshStage};
        for (error, words) in [
            (
                Error::Connect("Connection refused (os error 111)".into()),
                "Could not reach the server",
            ),
            (
                Error::Auth("password authentication failed".into()),
                "The server refused the login",
            ),
            (Error::Timeout, "The server did not answer in time"),
            (
                Error::ConnectionLost("eof".into()),
                "The connection was lost",
            ),
            (
                Error::Tls("invalid peer certificate".into()),
                "The secure connection failed",
            ),
            (
                Error::Ssh {
                    stage: SshStage::Auth,
                    message: "x".into(),
                },
                "The SSH login failed",
            ),
            (
                Error::Ssh {
                    stage: SshStage::Forward,
                    message: "x".into(),
                },
                "The SSH server could not reach the database",
            ),
        ] {
            let described = crate::ui::format::describe_error(crate::i18n::Locale::English, &error);
            assert!(described.starts_with(words), "{error:?}: {described}");
        }
    }

    #[test]
    fn the_schema_menu_closes_on_a_pick_without_a_pointer() {
        let (mut harness, tab) = tree_harness();
        let workspace = harness.app.workspace_mut(tab).unwrap();
        workspace.tree.schemas.value = Some(vec!["main".into(), "reports".into()]);
        harness.click("Schema");
        assert!(harness.has("reports"), "the menu lists the other schema");
        harness.click("reports");
        let workspace = harness.app.workspace(tab).unwrap();
        assert_eq!(
            workspace
                .tree
                .shown_schema(workspace.driver, false)
                .as_deref(),
            Some("reports")
        );
        // Closed: `main` was only in the menu once `reports` shows.
        assert!(!harness.has("main"), "the schema menu stays open");
        assert_eq!(focused_name(&harness.settle()), "Schema");
    }

    #[test]
    fn a_database_without_visible_schemas_says_so() {
        let (mut harness, tab) = tree_harness();
        harness.app.workspace_mut(tab).unwrap().tree.schemas.value = Some(Vec::new());
        assert!(harness.has("This database has no schemas you can see."));
    }

    /// Interactive roles a screen reader announces; each needs a name.
    const NAMED: [egui::accesskit::Role; 8] = [
        egui::accesskit::Role::Button,
        egui::accesskit::Role::TextInput,
        egui::accesskit::Role::PasswordInput,
        egui::accesskit::Role::RadioButton,
        egui::accesskit::Role::MultilineTextInput,
        egui::accesskit::Role::CheckBox,
        egui::accesskit::Role::ComboBox,
        egui::accesskit::Role::Tab,
    ];

    fn unnamed(tree: &egui::accesskit::TreeUpdate) -> Vec<String> {
        tree.nodes
            .iter()
            .filter(|(_, node)| NAMED.contains(&node.role()))
            .filter(|(_, node)| {
                // A label, a value, a placeholder, or a labelling widget
                // (how screen readers name a field beside its label).
                // A button's text is its name; anything else needs a name
                // besides its current value ("Prefer (not verified)" does
                // not say it is the TLS mode).
                let value_names_it = node.role() == egui::accesskit::Role::Button
                    && node.value().is_some_and(|value| !value.is_empty());
                node.label().is_none_or(str::is_empty)
                    && !value_names_it
                    && node.placeholder().is_none_or(str::is_empty)
                    && node.labelled_by().is_empty()
            })
            .map(|(id, node)| format!("{:?} {id:?}", node.role()))
            .collect()
    }

    /// Every main screen, settled.
    fn scenes() -> Vec<(&'static str, egui::accesskit::TreeUpdate)> {
        let mut scenes = Vec::new();
        let mut harness = Harness::new();
        add_saved(&mut harness, "Production");
        scenes.push(("picker", harness.settle()));

        let (mut harness, _tab) = tree_harness();
        harness.click("orders");
        harness.answer_rows(crate::testing::page(3, true));
        harness.app.apply(crate::model::Action::SelectCell {
            tab: harness.app.active_tab_id(),
            id: harness
                .app
                .workspace(harness.app.active_tab_id())
                .unwrap()
                .active_tab
                .unwrap(),
            cell: crate::model::CellPos { row: 0, col: 0 },
        });
        scenes.push(("workspace", harness.settle()));
        harness.press(Key::F, Modifiers::COMMAND);
        scenes.push(("filter bar", harness.settle()));
        harness.press(Key::F, Modifiers::COMMAND);
        harness.press(Key::P, Modifiers::COMMAND);
        scenes.push(("quick open", harness.settle()));
        harness.press(Key::Escape, Modifiers::NONE);
        harness.frame(vec![egui::Event::Text("?".into())]);
        scenes.push(("help", harness.settle()));

        for look in crate::theme::Look::ALL {
            let (mut harness, tab) = sql_harness(look);
            scenes.push(("sql editor", harness.settle()));
            with_sql_result(&mut harness, tab, 3);
            scenes.push(("sql result", harness.settle()));
        }

        let (mut harness, tab) = tree_harness();
        with_database_picker(&mut harness, tab);
        scenes.push(("database picker", harness.settle()));

        let (mut harness, _tab) = empty_schema_harness();
        scenes.push(("empty schema", harness.settle()));
        let (mut harness, tab) = no_schemas_harness();
        with_database_picker(&mut harness, tab);
        scenes.push(("no schemas", harness.settle()));

        for (name, look) in [
            ("connection dialog", crate::theme::Look::standard()),
            ("connection dialog (macos)", crate::theme::Look::macos()),
            (
                "connection dialog (terminal)",
                crate::theme::Look::omarchy(),
            ),
        ] {
            let mut harness = Harness::new();
            harness.set_look(look);
            harness.press(Key::N, Modifiers::COMMAND);
            harness.click(&look.label("PostgreSQL"));
            harness.click("Connect through SSH tunnel");
            // With hosts in ~/.ssh/config, the button that lists them.
            ssh_form(&mut harness).ssh_hosts = vec![bastion()];
            scenes.push((name, harness.settle()));
        }

        let mut harness = Harness::new();
        let tab = harness.app.active_tab_id();
        harness.app.dialog = Some(crate::model::Dialog::HostKey(Box::new(
            crate::model::HostKeyPrompt {
                tab,
                host: "bastion".into(),
                port: 22,
                fingerprint: "SHA256:abc".into(),
            },
        )));
        scenes.push(("host key prompt", harness.settle()));
        harness.app.dialog = Some(crate::model::Dialog::Password(Box::new(
            crate::model::PasswordPrompt {
                tab,
                kind: crate::model::SecretKind::Database,
                name: "Prod".into(),
                password: String::new(),
                save: true,
                message: None,
            },
        )));
        scenes.push(("password prompt", harness.settle()));
        scenes
    }

    #[test]
    fn every_interactive_node_is_named() {
        let mut failures = Vec::new();
        for (name, tree) in scenes() {
            let missing = unnamed(&tree);
            if !missing.is_empty() {
                failures.push(format!("{name}: {missing:?}"));
            }
        }
        assert!(failures.is_empty(), "{failures:#?}");
    }

    /// The name a screen reader would read for the focused node.
    fn focused_name(tree: &egui::accesskit::TreeUpdate) -> String {
        let Some((_, node)) = tree.nodes.iter().find(|(id, _)| *id == tree.focus) else {
            return String::new();
        };
        let by = node
            .labelled_by()
            .iter()
            .filter_map(|id| tree.nodes.iter().find(|(n, _)| n == id))
            .filter_map(|(_, label)| label.label().or_else(|| label.value()))
            .collect::<Vec<_>>()
            .join(" ");
        node.label()
            .or_else(|| node.placeholder())
            .map(str::to_owned)
            .unwrap_or(by)
    }

    #[test]
    fn tab_reaches_the_main_controls() {
        let (mut harness, _tab) = tree_harness();
        harness.click("orders");
        harness.answer_rows(crate::testing::page(3, true));
        let mut reached = std::collections::HashSet::new();
        for _ in 0..80 {
            harness.press(Key::Tab, Modifiers::NONE);
            reached.insert(focused_name(&harness.settle()));
        }
        // The tree and the grid are one stop each: the arrows move in them.
        for expected in [
            "Filter",
            "Objects",
            "Rows",
            "Count",
            "Next page",
            "Refresh objects",
        ] {
            assert!(
                reached.contains(expected),
                "{expected} not reached: {reached:?}"
            );
        }
    }

    #[test]
    fn every_tab_stop_shows_where_the_keyboard_is() {
        for look in crate::theme::Look::ALL {
            let (mut harness, _tab) = tree_harness();
            harness.set_look(look);
            harness.click("orders");
            harness.answer_rows(crate::testing::page(3, true));
            let accent = harness.app.palette.accent;
            let mut seen = 0;
            for _ in 0..60 {
                harness.press(Key::Tab, Modifiers::NONE);
                let name = focused_name(&harness.settle());
                // A text field shows its caret, in a box or not.
                if harness.ctx.memory(|memory| memory.focused().is_none())
                    || harness.ctx.text_edit_focused()
                {
                    continue;
                }
                seen += 1;
                // A ring, a field's border, or the terminal's reversed
                // button: something is drawn in the accent for it.
                let ringed = harness
                    .outlines
                    .iter()
                    .any(|(_, stroke)| stroke.color == accent && stroke.width >= 1.0);
                let reversed =
                    look.terminal && harness.fills.iter().any(|(_, fill)| *fill == accent);
                assert!(ringed || reversed, "{name} in {}", look.name);
            }
            assert!(seen > 10, "{}: {seen} stops", look.name);
        }
    }

    #[test]
    fn the_tab_key_gives_the_tree_and_the_grid_the_arrows() {
        use crate::model::Pane;
        for look in crate::theme::Look::ALL {
            let (mut harness, tab) = tree_harness();
            harness.set_look(look);
            harness.click("orders");
            harness.answer_rows(crate::testing::page(3, true));
            let tab_to = |harness: &mut Harness, name: &str| {
                for _ in 0..60 {
                    harness.press(Key::Tab, Modifiers::NONE);
                    if focused_name(&harness.settle()) == name {
                        return;
                    }
                }
                panic!("{name} is no Tab stop in {}", look.name);
            };
            // Onto the tree: its cursor is on the open table, and moves.
            tab_to(&mut harness, "Objects");
            let workspace = harness.app.workspace(tab).unwrap();
            assert_eq!(workspace.pane, Pane::Tree, "{}", look.name);
            let before = workspace.tree.cursor.clone();
            assert!(before.is_some(), "{}", look.name);
            harness.press(Key::ArrowDown, Modifiers::NONE);
            let workspace = harness.app.workspace(tab).unwrap();
            assert_ne!(workspace.tree.cursor, before, "{}", look.name);
            assert_eq!(focused_name(&harness.settle()), "Objects", "{}", look.name);
            // Onto the grid: its first cell, and the arrows are its own.
            tab_to(&mut harness, "Rows");
            let selection = |harness: &Harness| {
                let workspace = harness.app.workspace(tab).unwrap();
                workspace.active_object_tab().unwrap().selection
            };
            assert_eq!(harness.app.workspace(tab).unwrap().pane, Pane::Grid);
            assert_eq!(selection(&harness), None, "coming to it selects nothing");
            harness.press(Key::ArrowDown, Modifiers::NONE);
            let first = selection(&harness);
            assert!(first.is_some(), "{}", look.name);
            harness.press(Key::ArrowDown, Modifiers::NONE);
            assert_ne!(selection(&harness), first, "{}", look.name);
            assert_eq!(focused_name(&harness.settle()), "Rows", "{}", look.name);
        }
    }

    #[test]
    fn arrows_leave_the_grid_alone_while_a_button_has_the_keyboard() {
        let (mut harness, tab) = tree_harness();
        harness.click("orders");
        harness.answer_rows(crate::testing::page(3, true));
        harness.click("Row 1");
        let selection = |harness: &Harness| {
            let workspace = harness.app.workspace(tab).unwrap();
            workspace.active_object_tab().unwrap().selection
        };
        let first = selection(&harness);
        focus(&mut harness, "Add filter", egui::accesskit::Role::Button);
        harness.press(Key::ArrowDown, Modifiers::NONE);
        assert_eq!(selection(&harness), first, "the button had the keys");
    }

    #[test]
    fn a_switch_is_one_stop_and_the_arrows_choose_in_it() {
        let mut harness = Harness::new();
        harness.set_look(crate::theme::Look::macos());
        let tab = with_page(&mut harness);
        let view = |harness: &Harness| {
            let workspace = harness.app.workspace(tab).unwrap();
            workspace.active_object_tab().unwrap().view
        };
        focus(&mut harness, "Data", egui::accesskit::Role::Button);
        harness.press(Key::ArrowRight, Modifiers::NONE);
        assert_eq!(view(&harness), crate::model::ObjectView::Structure);
        assert_eq!(focused_name(&harness.settle()), "Structure");
        harness.press(Key::ArrowLeft, Modifiers::NONE);
        assert_eq!(view(&harness), crate::model::ObjectView::Data);
        assert_eq!(focused_name(&harness.settle()), "Data");
        // Tab passes the choice not made.
        let mut stops = std::collections::HashSet::new();
        for _ in 0..60 {
            harness.press(Key::Tab, Modifiers::NONE);
            stops.insert(focused_name(&harness.settle()));
        }
        assert!(
            stops.contains("Data") && !stops.contains("Structure"),
            "{stops:?}"
        );
    }

    #[test]
    fn f6_steps_through_the_parts_of_the_window() {
        for look in crate::theme::Look::ALL {
            let (mut harness, _tab) = tree_harness();
            harness.set_look(look);
            harness.click("orders");
            harness.answer_rows(crate::testing::page(3, true));
            let step = |harness: &mut Harness, modifiers| {
                harness.press(Key::F6, modifiers);
                focused_name(&harness.settle())
            };
            // From the tree, where opening the table left the arrows.
            let forward: Vec<String> = (0..6)
                .map(|_| step(&mut harness, Modifiers::NONE))
                .collect();
            assert_eq!(
                forward,
                [
                    "orders tab",
                    "Data",
                    "Rows",
                    "Connections",
                    "Filter",
                    "Objects"
                ],
                "{}",
                look.name
            );
            // And back the way it came.
            let back: Vec<String> = (0..3)
                .map(|_| step(&mut harness, Modifiers::SHIFT))
                .collect();
            assert_eq!(back, ["Filter", "Connections", "Rows"], "{}", look.name);
        }
    }

    #[test]
    fn the_terminal_steps_between_its_panes_with_ctrl_h_and_l() {
        let (mut harness, tab) = tree_harness();
        harness.set_look(crate::theme::Look::omarchy());
        harness.click("orders");
        harness.answer_rows(crate::testing::page(3, true));
        let pane = |harness: &Harness| harness.app.workspace(tab).unwrap().pane;
        harness.press(Key::L, Modifiers::CTRL);
        assert_eq!(focused_name(&harness.settle()), "Rows");
        assert_eq!(pane(&harness), crate::model::Pane::Grid);
        harness.press(Key::H, Modifiers::CTRL);
        assert_eq!(focused_name(&harness.settle()), "Objects");
        assert_eq!(pane(&harness), crate::model::Pane::Tree);
        // The other looks leave those keys alone.
        let (mut harness, tab) = tree_harness();
        harness.set_look(crate::theme::Look::macos());
        harness.click("orders");
        harness.answer_rows(crate::testing::page(3, true));
        harness.press(Key::L, Modifiers::CTRL);
        assert_eq!(focused_name(&harness.settle()), "");
        assert_eq!(
            harness.app.workspace(tab).unwrap().pane,
            crate::model::Pane::Tree
        );
    }

    #[test]
    fn a_button_that_cannot_be_pressed_still_says_why_to_the_keyboard() {
        let mut harness = Harness::new();
        harness.set_look(crate::theme::Look::macos());
        let tab = with_page(&mut harness);
        let mut reached = false;
        for _ in 0..60 {
            harness.press(Key::Tab, Modifiers::NONE);
            let tree = harness.settle();
            if focused_name(&tree) == "Add row" {
                reached = true;
                // Why not, where a screen reader finds it and on screen.
                let (_, node) = tree.nodes.iter().find(|(id, _)| *id == tree.focus).unwrap();
                assert_eq!(
                    node.description(),
                    Some("Editing arrives in a later version")
                );
                assert!(node.is_disabled());
                assert!(
                    harness
                        .painted
                        .iter()
                        .any(|(text, _)| text == "Editing arrives in a later version"),
                    "{:?}",
                    harness.painted
                );
                break;
            }
        }
        assert!(reached, "Add row is a Tab stop");
        // Enter does not press it.
        let sent = harness.app.backend.sent.len();
        harness.press(Key::Enter, Modifiers::NONE);
        assert_eq!(harness.app.backend.sent.len(), sent);
        assert!(harness.app.workspace(tab).is_some());
    }

    /// A table of twelve text columns behind a key, open in `harness`.
    fn wide_table(harness: &mut Harness) {
        use tabletist_db::{ColumnInfo, ColumnMeta, RowPage, Structure, Value, ValueKind};
        let tab = harness.connect_fake();
        harness.app.apply(crate::model::Action::OpenObject {
            tab,
            object: tabletist_db::ObjectRef::new("main", "wide"),
            kind: tabletist_db::ObjectKind::Table,
            pin: true,
        });
        let names: Vec<String> = std::iter::once("id".to_owned())
            .chain((1..12).map(|index| format!("a_rather_long_column_{index}")))
            .collect();
        harness.answer_structure(Structure {
            columns: names
                .iter()
                .map(|name| ColumnInfo {
                    name: name.clone(),
                    type_name: "text".into(),
                    nullable: true,
                    default: None,
                    comment: None,
                    allowed_values: None,
                    generated: false,
                })
                .collect(),
            primary_key: vec!["id".into()],
            indexes: Vec::new(),
            foreign_keys: Vec::new(),
        });
        harness.answer_rows(RowPage {
            columns: names
                .iter()
                .map(|name| ColumnMeta {
                    name: name.clone(),
                    type_name: "text".into(),
                    kind: ValueKind::Text,
                })
                .collect(),
            rows: vec![vec![Value::Text("value".into()); 12]; 3],
            has_more: false,
            ordered_by_key: true,
            elapsed: std::time::Duration::ZERO,
        });
        harness.settle();
        harness.settle();
    }

    #[test]
    fn the_status_line_says_which_columns_are_in_view_while_some_are_not() {
        for look in crate::theme::Look::ALL {
            let said = |harness: &Harness| {
                harness
                    .painted
                    .iter()
                    .any(|(text, _)| text.contains("of 12 · id pinned"))
            };
            let mut harness = Harness::with_size(egui::vec2(1600.0, 600.0));
            harness.set_look(look);
            wide_table(&mut harness);
            assert!(said(&harness), "{}: {:?}", look.name, harness.painted);
            // With room for every column there is nothing to say.
            let mut harness = Harness::with_size(egui::vec2(6000.0, 600.0));
            harness.set_look(look);
            wide_table(&mut harness);
            assert!(!said(&harness), "{}", look.name);
        }
    }

    #[test]
    fn the_terminal_marks_the_pane_the_keys_go_to() {
        // An accent line round something as tall as a pane.
        let marked = |harness: &Harness| {
            let accent = harness.app.palette.accent;
            harness
                .outlines
                .iter()
                .any(|(rect, stroke)| stroke.color == accent && rect.height() > 200.0)
        };
        for look in crate::theme::Look::ALL {
            let (mut harness, _tab) = tree_harness();
            harness.set_look(look);
            harness.click("orders");
            harness.answer_rows(crate::testing::page(3, true));
            // The pointer alone marks nothing.
            harness.settle();
            assert!(!marked(&harness), "{}", look.name);
            // A key: the tree has the arrows, where opening left them.
            harness.press(Key::ArrowDown, Modifiers::NONE);
            harness.settle();
            assert_eq!(marked(&harness), look.terminal, "{}", look.name);
        }
    }

    #[test]
    fn the_terminals_open_table_is_marked_quietly_while_the_keys_are_elsewhere() {
        let (mut harness, _tab) = tree_harness();
        harness.set_look(crate::theme::Look::omarchy());
        harness.click("orders");
        harness.answer_rows(crate::testing::page(3, true));
        // The bar at the left of the open table's row, in `color`.
        let bar = |harness: &Harness, color: egui::Color32| {
            harness
                .fills
                .iter()
                .any(|(rect, fill)| *fill == color && rect.width() < 4.0 && rect.height() > 10.0)
        };
        let (accent, muted) = (harness.app.palette.accent, harness.app.palette.dim);
        // Opening left the arrows with the tree.
        harness.settle();
        assert!(bar(&harness, accent) && !bar(&harness, muted));
        // A click in the grid takes them there.
        harness.click("Row 1");
        harness.settle();
        assert!(bar(&harness, muted));
    }

    #[test]
    fn the_pointer_takes_the_ring_away() {
        let (mut harness, _tab) = tree_harness();
        harness.set_look(crate::theme::Look::macos());
        harness.press(Key::Tab, Modifiers::NONE);
        assert!(crate::ui::focus::visible(&harness.ctx));
        let at = egui::pos2(900.0, 500.0);
        harness.frame(vec![
            egui::Event::PointerMoved(at),
            egui::Event::PointerButton {
                pos: at,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: Modifiers::NONE,
            },
        ]);
        assert!(!crate::ui::focus::visible(&harness.ctx));
    }

    #[test]
    fn command_r_refreshes_the_tree_from_the_tree() {
        let (mut harness, _tab) = tree_harness();
        harness.click("orders");
        harness.answer_rows(crate::testing::page(3, false));
        let before = harness.app.backend.sent.len();
        harness.press(Key::R, Modifiers::COMMAND);
        assert!(
            harness.app.backend.sent[before..]
                .iter()
                .any(|c| matches!(c, crate::backend::Command::ListSchemas { .. })),
            "the tree reloads its schemas"
        );
    }

    fn users_with_bar(harness: &mut Harness) -> (crate::model::ConnTabId, crate::model::TabId) {
        let tab = harness.connect_fake();
        harness.app.apply(crate::model::Action::OpenObject {
            tab,
            object: tabletist_db::ObjectRef::new("main", "users"),
            kind: tabletist_db::ObjectKind::Table,
            pin: true,
        });
        harness.answer_rows(crate::testing::page(3, false));
        let id = harness.app.workspace(tab).unwrap().active_tab.unwrap();
        harness.press(Key::F, Modifiers::COMMAND);
        (tab, id)
    }

    #[test]
    fn reopening_the_bar_focuses_a_control() {
        let mut harness = Harness::new();
        let (tab, id) = users_with_bar(&mut harness);
        harness
            .app
            .workspace_mut(tab)
            .unwrap()
            .object_tab_mut(id)
            .unwrap()
            .filter
            .rows[0]
            .op = tabletist_db::FilterOp::IsNull;
        harness.press(Key::F, Modifiers::COMMAND); // close
        harness.press(Key::F, Modifiers::COMMAND); // open again
        harness.settle();
        assert!(harness.ctx.memory(|memory| memory.focused().is_some()));
    }

    #[test]
    fn command_f_focuses_an_open_bar() {
        let mut harness = Harness::new();
        let (tab, id) = users_with_bar(&mut harness);
        harness.press(Key::Escape, Modifiers::NONE); // leave the field
        assert!(!harness.ctx.text_edit_focused());
        harness.press(Key::F, Modifiers::COMMAND);
        harness.settle();
        let object = harness.app.workspace(tab).unwrap().object_tab(id).unwrap();
        assert!(object.filter.open, "Cmd/Ctrl+F focuses, not closes");
        assert!(harness.ctx.text_edit_focused());
    }

    #[test]
    fn quick_open_keeps_a_wheel_scroll() {
        let (mut harness, tab) = tree_harness();
        let names: Vec<String> = (0..60).map(|i| format!("t_{i:03}")).collect();
        harness
            .app
            .workspace_mut(tab)
            .unwrap()
            .tree
            .nodes
            .get_mut("main")
            .unwrap()
            .objects
            .value = Some(
            names
                .iter()
                .map(|name| tabletist_db::ObjectInfo {
                    name: name.clone(),
                    kind: tabletist_db::ObjectKind::Table,
                    estimated_rows: None,
                })
                .collect(),
        );
        harness.press(Key::P, Modifiers::COMMAND);
        harness.settle();
        let tree = harness.settle();
        let list = tree
            .nodes
            .iter()
            .find(|(_, node)| node.label() == Some("main.t_005"))
            .and_then(|(_, node)| node.bounds())
            .expect("a result row");
        let over = egui::pos2(list.x0 as f32 + 20.0, list.y0 as f32 + 5.0);
        harness.frame(vec![egui::Event::PointerMoved(over)]);
        harness.frame(vec![
            egui::Event::PointerMoved(over),
            egui::Event::MouseWheel {
                unit: egui::MouseWheelUnit::Point,
                delta: egui::vec2(0.0, -200.0),
                modifiers: Modifiers::NONE,
                phase: egui::TouchPhase::Move,
            },
        ]);
        for _ in 0..120 {
            harness.frame(vec![]);
        }
        let Some(crate::model::Dialog::QuickOpen(open)) = &harness.app.dialog else {
            panic!("quick open");
        };
        assert!(
            open.scroll_offset > 50.0,
            "snapped back to {}",
            open.scroll_offset
        );
    }

    #[test]
    fn the_disconnected_banner_shows_the_exact_error_too() {
        let mut harness = Harness::new();
        let tab = harness.connect_fake();
        let session = harness.app.workspace(tab).unwrap().session;
        harness.app.apply(crate::model::Action::Backend(
            crate::backend::Event::Disconnected {
                session,
                error: tabletist_db::Error::Connect("Connection refused (os error 111)".into()),
            },
        ));
        assert!(harness.has(
            "Could not reach the server. Check the host and port, and that the server is running."
        ));
        assert!(harness.has("could not connect: Connection refused (os error 111)"));
    }

    #[test]
    fn a_server_refusal_is_shown_in_the_servers_words() {
        let error = tabletist_db::Error::Query {
            code: Some("3D000".into()),
            message: "database \"nosuchdb\" does not exist".into(),
            detail: None,
            hint: None,
        };
        assert_eq!(
            crate::ui::format::describe_error(crate::i18n::Locale::English, &error),
            "database \"nosuchdb\" does not exist"
        );
    }

    /// Server names and values with a right-to-left override or a zero
    /// width space: every view writes them out, and copying keeps them.
    #[test]
    fn hidden_characters_are_shown_and_copied_as_they_are() {
        let spoof = "users_\u{202E}atad";
        let (mut harness, tab) = tree_harness();
        let workspace = harness.app.workspace_mut(tab).unwrap();
        // Flat, so `users_…` is not folded into a `users` group.
        workspace.tree.flat = true;
        let table = |name: &str| tabletist_db::ObjectInfo {
            name: name.into(),
            kind: tabletist_db::ObjectKind::Table,
            estimated_rows: None,
        };
        let main = workspace.tree.nodes.get_mut("main").unwrap();
        main.objects.value = Some(vec![table(spoof), table("users"), table("users\u{200B}")]);

        let labels = crate::testing::labels(&harness.settle());
        assert!(labels.iter().any(|label| label == "users_<U+202E>atad"));
        assert!(labels.iter().any(|label| label == "users<U+200B>"));
        harness.click("users_<U+202E>atad");
        let mut page = crate::testing::page(1, false);
        page.columns[1].name = "e\u{202E}liam".into();
        page.rows[0][1] = tabletist_db::Value::Text("Total: \u{202E}00.0001".into());
        harness.answer_rows(page);
        let workspace = harness.app.workspace_mut(tab).unwrap();
        workspace.row_panel = true;
        let id = workspace.active_tab.unwrap();
        harness.app.apply(crate::model::Action::SelectCell {
            tab,
            id,
            cell: crate::model::CellPos { row: 0, col: 1 },
        });
        let labels = crate::testing::labels(&harness.settle());
        // The object tab, the grid header, the row panel's copy button.
        assert!(labels.iter().any(|label| label == "users_<U+202E>atad tab"));
        assert!(labels.iter().any(|label| label == "e<U+202E>liam"));
        assert!(labels.iter().any(|label| label == "Copy e<U+202E>liam"));
        let painted: Vec<&str> = harness
            .painted
            .iter()
            .map(|(text, _)| text.as_str())
            .collect();
        assert!(painted.contains(&"Total: <U+202E>00.0001"), "{painted:?}");
        assert!(painted.contains(&"users_<U+202E>atad"), "{painted:?}");
        assert!(
            !painted
                .iter()
                .any(|text| text.contains(['\u{202E}', '\u{200B}'])),
            "{painted:?}"
        );
        harness.copy(false);
        assert_eq!(harness.copied.as_deref(), Some("Total: \u{202E}00.0001"));
    }

    /// SQLite and MySQL keep a UUID key as sixteen bytes (`blob(16)`,
    /// `binary(16)`): the grid, the row panel and the clipboard give the
    /// UUID, not the value's size.
    #[test]
    fn a_sixteen_byte_key_reads_as_a_uuid() {
        let uuid = "0199a3f2-7c1e-7abc-8def-0123456789ab";
        let (mut harness, tab) = tree_harness();
        harness.click("users");
        let mut page = crate::testing::page(1, false);
        page.columns[0].type_name = "blob(16)".into();
        page.columns[0].kind = tabletist_db::ValueKind::Binary;
        page.rows[0][0] = tabletist_db::Value::Bytes(
            vec![
                0x01, 0x99, 0xa3, 0xf2, 0x7c, 0x1e, 0x7a, 0xbc, 0x8d, 0xef, 0x01, 0x23, 0x45, 0x67,
                0x89, 0xab,
            ]
            .into(),
        );
        harness.answer_rows(page);
        let workspace = harness.app.workspace_mut(tab).unwrap();
        workspace.row_panel = true;
        let id = workspace.active_tab.unwrap();
        harness.app.apply(crate::model::Action::SelectCell {
            tab,
            id,
            cell: crate::model::CellPos { row: 0, col: 0 },
        });
        harness.settle();
        let painted: Vec<&str> = harness
            .painted
            .iter()
            .map(|(text, _)| text.as_str())
            .collect();
        // Once in the grid, once as the row panel's field.
        let shown = painted.iter().filter(|text| **text == uuid).count();
        assert!(shown >= 2, "{painted:?}");
        assert!(
            !painted.iter().any(|text| text.contains("BLOB")),
            "{painted:?}"
        );
        harness.copy(false);
        assert_eq!(harness.copied.as_deref(), Some(uuid));
    }

    /// Following a sixteen-byte foreign key filters the target by the UUID
    /// the grid shows. The database layer reads that text back as the bytes
    /// (`tabletist_db::Dialect`), so the filter value has to stay this text.
    #[test]
    fn following_a_sixteen_byte_key_filters_by_the_uuid_it_shows() {
        let (mut harness, tab) = tree_harness();
        harness.click("users");
        let mut page = crate::testing::page(1, false);
        page.columns[0].type_name = "blob(16)".into();
        page.columns[0].kind = tabletist_db::ValueKind::Binary;
        page.rows[0][0] = tabletist_db::Value::Bytes(
            vec![
                0x01, 0x99, 0xa3, 0xf2, 0x7c, 0x1e, 0x7a, 0xbc, 0x8d, 0xef, 0x01, 0x23, 0x45, 0x67,
                0x89, 0xab,
            ]
            .into(),
        );
        harness.answer_rows(page);
        harness.answer_structure(tabletist_db::Structure {
            foreign_keys: vec![tabletist_db::ForeignKeyInfo {
                name: None,
                columns: vec!["id".into()],
                ref_schema: "main".into(),
                ref_table: "orders".into(),
                ref_columns: vec!["account_id".into()],
                on_update: String::new(),
                on_delete: String::new(),
            }],
            ..Default::default()
        });
        let id = harness.app.workspace(tab).unwrap().active_tab.unwrap();
        harness.app.apply(crate::model::Action::SelectCell {
            tab,
            id,
            cell: crate::model::CellPos { row: 0, col: 0 },
        });
        harness.app.apply(crate::model::Action::FollowSelectedKey {
            tab,
            object_tab: id,
        });
        match crate::testing::last_sent(&harness.app) {
            Command::FetchRows { query, .. } => {
                assert_eq!(query.object.name, "orders");
                assert_eq!(
                    query.filters,
                    [tabletist_db::Filter {
                        column: "account_id".into(),
                        op: tabletist_db::FilterOp::Eq,
                        value: "0199a3f2-7c1e-7abc-8def-0123456789ab".into(),
                    }]
                );
            }
            other => panic!("{other:?}"),
        }
    }

    /// Two schemas with a `users` table: the tab and the row panel name the
    /// schema, and a name only one schema has stays short.
    #[test]
    fn a_name_two_schemas_share_is_shown_with_its_schema() {
        let (mut harness, tab) = tree_harness();
        harness.click("orders");
        harness.answer_rows(crate::testing::page(1, false));
        assert!(harness.has("orders tab"));
        let orders = harness.app.workspace(tab).unwrap().active_tab.unwrap();
        harness.app.apply(crate::model::Action::PinObjectTab {
            tab,
            object_tab: orders,
        });
        let workspace = harness.app.workspace_mut(tab).unwrap();
        workspace.tree.schemas.value = Some(vec!["main".into(), "audit".into()]);
        let audit = workspace.tree.nodes.entry("audit".into()).or_default();
        audit.objects.value = Some(vec![tabletist_db::ObjectInfo {
            name: "users".into(),
            kind: tabletist_db::ObjectKind::Table,
            estimated_rows: None,
        }]);
        harness.click("users");
        harness.answer_rows(crate::testing::page(1, false));
        let workspace = harness.app.workspace_mut(tab).unwrap();
        workspace.row_panel = true;
        let id = workspace.active_tab.unwrap();
        harness.app.apply(crate::model::Action::SelectCell {
            tab,
            id,
            cell: crate::model::CellPos { row: 0, col: 0 },
        });
        harness.settle();
        assert!(harness.has("main.users tab"));
        assert!(harness.has("orders tab"));
        // The tab and the row panel's subtitle.
        let named = harness
            .painted
            .iter()
            .filter(|(text, _)| text == "main.users")
            .count();
        assert_eq!(named, 2, "{:?}", harness.painted);
    }

    #[test]
    fn closing_a_tab_with_a_pending_change_asks_first_and_discard_closes_it() {
        use crate::model::{Action, Advance, CellPos, Dialog, EditStart};
        let mut harness = Harness::new();
        let (tab, id) = harness.editable();
        harness.app.apply(Action::EditCell {
            tab,
            id,
            cell: CellPos { row: 1, col: 1 },
            start: EditStart::Replace("bob@example.com".into()),
        });
        harness.app.apply(Action::CommitEdit {
            tab,
            id,
            then: Advance::Stay,
        });
        let pending = |harness: &Harness| {
            let workspace = harness.app.workspace(tab).unwrap();
            workspace.object_tab(id).map(|open| open.edits.cells.len())
        };
        harness.click("Close users");
        assert!(matches!(harness.app.dialog, Some(Dialog::Leave(_))));
        assert_eq!(pending(&harness), Some(1), "nothing is dropped yet");
        // Esc stays: the tab and its change are as they were.
        harness.press(Key::Escape, Modifiers::NONE);
        assert!(harness.app.dialog.is_none());
        assert_eq!(pending(&harness), Some(1));
        harness.click("Close users");
        harness.click("Discard");
        assert!(harness.app.dialog.is_none());
        assert_eq!(pending(&harness), None, "the tab closed");
    }

    use crate::model::{Action, Advance, CellPos, ConnTabId, EditStart, TabId};
    use crate::theme::Look;
    use crate::ui::states::Tone;

    /// A writable table with its structure and five rows, the grid holding
    /// the keyboard, in `look`.
    fn editable_in(look: Look) -> (Harness, ConnTabId, TabId) {
        let mut harness = Harness::new();
        harness.set_look(look);
        let (tab, id) = harness.editable();
        focus_grid(&mut harness, tab);
        harness.settle();
        (harness, tab, id)
    }

    /// The middle of the cell that shows `text`.
    fn cell_of(harness: &Harness, text: &str) -> egui::Pos2 {
        harness
            .painted_rect(text)
            .unwrap_or_else(|| panic!("no cell shows {text}"))
            .center()
    }

    fn edits(harness: &Harness, tab: ConnTabId, id: TabId) -> &crate::edit::Edits {
        &harness
            .app
            .workspace(tab)
            .unwrap()
            .object_tab(id)
            .unwrap()
            .edits
    }

    /// Whether the last frame filled a cell's worth of `color` behind the
    /// cell that shows `text`.
    fn filled_behind(harness: &Harness, text: &str, color: egui::Color32) -> bool {
        let at = cell_of(harness, text);
        harness
            .fills
            .iter()
            .any(|(rect, fill)| *fill == color && rect.contains(at) && rect.width() < 500.0)
    }

    /// Makes the cell at `row`, `col` of the table pending as `text`.
    fn make_pending(
        harness: &mut Harness,
        tab: ConnTabId,
        id: TabId,
        at: (usize, usize),
        text: &str,
    ) {
        let cell = CellPos {
            row: at.0,
            col: at.1,
        };
        let start = EditStart::Replace(text.into());
        harness.app.apply(Action::EditCell {
            tab,
            id,
            cell,
            start,
        });
        let then = Advance::Stay;
        harness.app.apply(Action::CommitEdit { tab, id, then });
    }

    /// Moves the pointer to `at` and waits past the tooltip's delay. Returns
    /// every name on screen then.
    fn hover(harness: &mut Harness, at: egui::Pos2) -> Vec<String> {
        harness.frame(vec![egui::Event::PointerMoved(at)]);
        // Each frame is 1/60 s.
        for _ in 0..60 {
            harness.frame(Vec::new());
        }
        crate::testing::labels(&harness.frame(Vec::new()))
    }

    #[test]
    fn a_computed_column_is_drawn_locked_in_a_table_that_can_be_edited() {
        // A table whose `email` the database computes, on a connection
        // that is read-only or not, with its structure known or not.
        let open = |look: Look, read_only: bool, described: bool| {
            let mut harness = Harness::new();
            harness.set_look(look);
            let tab = harness.connect_fake_as(read_only);
            harness.app.apply(Action::OpenObject {
                tab,
                object: tabletist_db::ObjectRef::new("main", "users"),
                kind: tabletist_db::ObjectKind::Table,
                pin: true,
            });
            if described {
                let mut structure = crate::testing::fixture_structure();
                structure.columns[1].generated = true;
                harness.answer_structure(structure);
            }
            harness.answer_rows(crate::testing::page(5, false));
            harness.settle();
            harness
        };
        for look in Look::ALL {
            let harness = open(look, false, true);
            let palette = harness.app.palette;
            // The computed column's cells stand on the surface, their text
            // a step quieter. The terminal draws them as it did.
            for email in ["user1@example.com", "user4@example.com"] {
                assert_eq!(
                    filled_behind(&harness, email, palette.surface),
                    !look.terminal,
                    "{}",
                    look.name
                );
                let color = if look.terminal {
                    palette.text
                } else {
                    palette.secondary
                };
                assert!(painted_in(&harness, email, color), "{}", look.name);
            }
            // No other column is.
            assert!(!filled_behind(&harness, "4", palette.surface));
            // Nothing of it where nothing can be edited: a cell there says
            // why when it is asked.
            for (read_only, described) in [(true, true), (false, false)] {
                let harness = open(look, read_only, described);
                assert!(
                    !filled_behind(&harness, "user1@example.com", palette.surface),
                    "{}: read-only {read_only}, described {described}",
                    look.name
                );
                assert!(painted_in(&harness, "user1@example.com", palette.text));
            }
        }
    }

    #[test]
    fn a_pending_cell_shows_its_new_value_and_what_it_was() {
        for look in Look::ALL {
            let (mut harness, tab, id) = editable_in(look);
            let palette = harness.app.palette;
            make_pending(&mut harness, tab, id, (1, 1), "bob@example.com");
            // The row panel has the row's values too: the grid alone is
            // looked at here.
            harness.app.workspace_mut(tab).unwrap().row_panel = false;
            harness.settle();
            let painted = |harness: &Harness, text: &str| {
                harness.painted.iter().any(|(piece, _)| piece == text)
            };
            assert!(painted(&harness, "bob@example.com"), "{}", look.name);
            assert!(!painted(&harness, "user2@example.com"), "{}", look.name);
            let amber = Tone::Warning.fill(&look, &palette);
            assert!(
                filled_behind(&harness, "bob@example.com", amber),
                "{}",
                look.name
            );
            assert!(
                !filled_behind(&harness, "user3@example.com", amber),
                "{}",
                look.name
            );
            // Its row is marked: the terminal's gutter, or a bar at its left.
            let color = Tone::Warning.color(&palette);
            if look.terminal {
                assert!(painted_in(&harness, "~", color), "{}", look.name);
                assert!(painted_in(&harness, "bob@example.com", color));
            } else {
                let bar = harness.fills.iter().any(|(rect, fill)| {
                    *fill == color && rect.width() == 3.0 && rect.height() == look.grid_row
                });
                assert!(bar, "{}", look.name);
                // The row's key takes the colour too.
                assert!(painted_in(&harness, "2", color), "{}", look.name);
            }
            // Under the pointer the cell says what it was.
            let at = cell_of(&harness, "bob@example.com");
            let names = hover(&mut harness, at);
            assert!(
                names.iter().any(|name| name == "was user2@example.com"),
                "{}: {names:?}",
                look.name
            );
            // A cell with nothing pending says nothing.
            let at = cell_of(&harness, "user3@example.com");
            let names = hover(&mut harness, at);
            assert!(
                !names.iter().any(|name| name.starts_with("was ")),
                "{}",
                look.name
            );
            // NULL is a value too, and what was NULL says so.
            harness.app.apply(Action::SelectCell {
                tab,
                id,
                cell: CellPos { row: 0, col: 2 },
            });
            harness.app.apply(Action::SetNull { tab, id });
            make_pending(&mut harness, tab, id, (1, 2), "[1]");
            harness.settle();
            assert_eq!(edits(&harness, tab, id).cells.len(), 3);
            let nulls = harness
                .text_rects
                .iter()
                .filter(|(text, _)| text == "NULL")
                .map(|(_, rect)| rect.center());
            let tinted = nulls
                .filter(|at| {
                    harness
                        .fills
                        .iter()
                        .any(|(rect, fill)| *fill == amber && rect.contains(*at))
                })
                .count();
            assert_eq!(tinted, 1, "{}: the cell set to NULL", look.name);
        }
    }

    #[test]
    fn a_saved_cell_is_green_for_a_moment() {
        for look in Look::ALL {
            let (mut harness, tab, id) = editable_in(look);
            let palette = harness.app.palette;
            // Short enough to show whole beside the spinner of a save.
            make_pending(&mut harness, tab, id, (1, 1), "b@x.io");
            // The row panel has the row's values too: the grid alone is
            // looked at here.
            harness.app.workspace_mut(tab).unwrap().row_panel = false;
            harness.app.apply(Action::WriteEdits { tab, id });
            harness.settle();
            // Being saved, it is still amber.
            assert!(edits(&harness, tab, id).saving.is_some());
            let amber = Tone::Warning.fill(&look, &palette);
            assert!(filled_behind(&harness, "b@x.io", amber), "{}", look.name);
            harness.answer_written(Ok(tabletist_db::WriteOutcome::Written {
                rows: vec![vec![
                    tabletist_db::Value::Int(2),
                    tabletist_db::Value::Text("b@x.io".into()),
                    tabletist_db::Value::Null,
                ]],
                elapsed: std::time::Duration::from_millis(14),
            }));
            harness.settle();
            let green = Tone::Success.fill(&look, &palette);
            assert!(filled_behind(&harness, "b@x.io", green), "{}", look.name);
            assert!(!filled_behind(&harness, "b@x.io", amber));
            // The frame asks to be drawn again when the moment is over.
            assert!(
                harness.repaint_after <= crate::edit::SAVED_FOR,
                "{}: {:?}",
                look.name,
                harness.repaint_after
            );
            // Two seconds on, the cell is as any other.
            let workspace = harness.app.workspace_mut(tab).unwrap();
            let saved = workspace.object_tab_mut(id).unwrap().edits.saved.as_mut();
            let saved = saved.expect("the save that wrote");
            saved.at = std::time::Instant::now()
                .checked_sub(std::time::Duration::from_secs(2))
                .expect("a clock two seconds old");
            harness.settle();
            assert!(!filled_behind(&harness, "b@x.io", green), "{}", look.name);
        }
    }

    #[test]
    fn a_sql_result_is_drawn_as_before() {
        for look in Look::ALL {
            let mut harness = Harness::new();
            harness.set_look(look);
            // A writable connection: a result is never edited all the same.
            let tab = harness.connect_fake_as(false);
            with_sql_result(&mut harness, tab, 3);
            focus_grid(&mut harness, tab);
            harness.settle();
            let palette = harness.app.palette;
            assert!(painted_in(&harness, "user2@example.com", palette.text));
            for tone in [Tone::Warning, Tone::Danger, Tone::Success] {
                let fill = tone.fill(&look, &palette);
                assert!(
                    !filled_behind(&harness, "user2@example.com", fill),
                    "{}: {tone:?}",
                    look.name
                );
            }
            let at = cell_of(&harness, "user2@example.com");
            let names = hover(&mut harness, at);
            assert!(!names.iter().any(|name| name.starts_with("was ")));
        }
    }
}

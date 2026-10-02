//! The interface. Views read `App` and push `Action`s; they never change
//! state directly.

pub mod about;
pub mod conn_tabs;
pub mod connect_dialog;
pub mod data_view;
#[cfg(test)]
mod env_tests;
pub mod filter_bar;
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
pub mod sidebar;
pub mod sql_editor;
pub mod sql_results;
pub mod sql_text;
pub mod structure;
pub mod value_tags;
pub mod widgets;
pub mod workspace;

use egui::Frame;

use crate::app::App;
use crate::model::ConnTabContent;

pub fn show(app: &mut App, ui: &mut egui::Ui) {
    // One connection needs no tab bar: its own bar leads the window.
    if app.tabs.len() > 1 {
        conn_tabs::show(app, ui);
    }
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
}

/// A problem worth the user's attention that belongs to no one tab (a
/// keyring write that failed), until dismissed.
fn notice(app: &mut App, ui: &mut egui::Ui) {
    let Some(message) = app.notice.clone() else {
        return;
    };
    let palette = app.palette;
    let look = app.look;
    let mut dismiss = false;
    egui::Panel::top(egui::Id::new("notice"))
        .resizable(false)
        .show_separator_line(false)
        .frame(
            Frame::new()
                .fill(palette.warning.gamma_multiply(0.15))
                .inner_margin(egui::Margin::symmetric(12, 8)),
        )
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                crate::typography::Text::one(&look, widgets::body(&look), &message, palette.text)
                    .layout(ui.ctx())
                    .label(ui);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    dismiss =
                        widgets::button(ui, &crate::i18n::gettext(app.locale, "Dismiss"), &look)
                            .clicked();
                });
            });
        });
    if dismiss {
        app.actions.push(crate::model::Action::DismissNotice);
    }
}

/// Where the window's own buttons (the macOS traffic lights) centre, in egui
/// points below the window's top: on the line of the bar that leads the
/// window, so the buttons and that bar's contents share one line.
pub fn window_buttons_line(app: &App, zoom: f32) -> f32 {
    if app.tabs.len() > 1 {
        conn_tabs::line(app, zoom)
    } else if matches!(app.active_tab().content, ConnTabContent::Picker(_)) {
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

    #[test]
    fn the_disconnected_banner_keeps_its_corners_inside_the_window() {
        for look in crate::theme::Look::ALL {
            let mut harness = Harness::new();
            harness.set_look(look);
            let tab = harness.connect_fake();
            let session = harness.app.workspace(tab).unwrap().session;
            harness.app.apply(crate::model::Action::Backend(
                crate::backend::Event::Disconnected {
                    session,
                    error: tabletist_db::Error::ConnectionLost("server went away".into()),
                },
            ));
            let tree = harness.settle();
            let button =
                crate::testing::bounds(&tree, "Reconnect", egui::accesskit::Role::Button).unwrap();
            // The banner pads its contents by 12; rounded looks inset it by 8 more.
            let expected = if look.tab_radius == 0 { 12.0 } else { 20.0 };
            assert_eq!(harness.size.x - button.right(), expected, "{}", look.name);
        }
    }

    #[test]
    fn one_connection_needs_no_tab_bar() {
        let mut harness = Harness::new();
        assert!(!harness.has("New tab"));
        assert!(!harness.has("New connection tab"));
        harness.press(Key::O, Modifiers::COMMAND);
        assert!(harness.has("New tab"));
        assert!(harness.has("New connection tab"));
    }

    #[test]
    fn the_plus_button_opens_a_tab() {
        let mut harness = Harness::new();
        harness.app.apply(crate::model::Action::NewConnTab);
        harness.click("New connection tab");
        assert_eq!(harness.app.tabs.len(), 3);
        assert_eq!(harness.app.active, 2);
    }

    #[test]
    fn the_close_button_closes_its_tab() {
        let mut harness = Harness::new();
        let closing = harness.app.active_tab_id();
        let first = harness.connect_fake();
        // Named apart from the saved connection the new tab's picker lists.
        harness.app.workspace_mut(first).unwrap().name = "Tab one".into();
        harness.app.apply(crate::model::Action::NewConnTab);
        harness.click("Close Tab one");
        assert_eq!(harness.app.tabs.len(), 1);
        assert_ne!(harness.app.tabs[0].id, closing);
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
    fn ctrl_o_opens_a_tab() {
        let mut harness = Harness::new();
        harness.press(Key::O, Modifiers::COMMAND);
        assert_eq!(harness.app.tabs.len(), 2);
    }

    #[test]
    fn ctrl_shift_w_closes_the_connection_tab() {
        let mut harness = Harness::new();
        harness.app.apply(crate::model::Action::NewConnTab);
        let active = harness.app.active_tab_id();
        harness.press(Key::W, Modifiers::COMMAND | Modifiers::SHIFT);
        assert_eq!(harness.app.tabs.len(), 1);
        assert!(harness.app.tabs.iter().all(|tab| tab.id != active));
    }

    #[test]
    fn plain_ctrl_w_does_not_close_the_connection_tab() {
        let mut harness = Harness::new();
        harness.app.apply(crate::model::Action::NewConnTab);
        harness.press(Key::W, Modifiers::COMMAND);
        assert_eq!(harness.app.tabs.len(), 2);
    }

    #[test]
    fn number_shortcuts_and_ctrl_tab_switch_tabs() {
        let mut harness = Harness::new();
        harness.app.apply(crate::model::Action::NewConnTab);
        harness.app.apply(crate::model::Action::NewConnTab);
        harness.press(Key::Num1, Modifiers::COMMAND);
        assert_eq!(harness.app.active, 0);
        harness.press(Key::Tab, Modifiers::CTRL);
        assert_eq!(harness.app.active, 1);
        harness.press(Key::Tab, Modifiers::CTRL | Modifiers::SHIFT);
        assert_eq!(harness.app.active, 0);
    }

    #[test]
    fn the_picker_shows_its_empty_state() {
        let mut harness = Harness::new();
        assert!(harness.has("No saved connections yet"));
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
        assert!(!harness.has("No saved connections yet"));
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
    fn a_disconnected_workspace_offers_reconnect() {
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
        assert!(harness.has("Reconnect"));
        harness.click("Reconnect");
        assert!(matches!(
            harness.app.workspace(tab).unwrap().status,
            crate::model::SessionStatus::Connecting { .. }
        ));
    }

    #[test]
    fn cancelling_the_password_prompt_says_so_and_offers_reconnect() {
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
        assert!(harness.has("Connection cancelled."));
        assert!(!harness.has("The server refused the login. Check the user and password."));
        assert!(harness.has("Reconnect"));
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
    fn a_running_query_shows_a_cancel_button() {
        let mut harness = Harness::new();
        harness.connect_fake();
        harness.click("users");
        harness.click("Cancel query");
        assert!(matches!(
            crate::testing::last_sent(&harness.app),
            Command::Cancel { .. }
        ));
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
        harness.app.apply(crate::model::Action::NewSqlTab(tab));
        let id = harness.app.workspace(tab).unwrap().active_tab.unwrap();
        set_sql(harness, tab, "SELECT 1", 0);
        harness.app.apply(crate::model::Action::RunSql {
            tab,
            sql_tab: id,
            all: false,
        });
        harness.answer_sql(
            Ok(crate::testing::script_outcome(vec![
                crate::testing::rows_outcome(rows),
            ])),
            None,
        );
        id
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

    const FORMAT: Modifiers = Modifiers::COMMAND.plus(Modifiers::SHIFT);

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
        harness.press(Key::F, FORMAT);
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
    fn the_row_panel_and_paging_keys_do_nothing_on_a_sql_tab() {
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
        // With the keys out of the editor, where Space would be typed.
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
        harness.app.apply(crate::model::Action::NewConnTab);
        harness.press(Key::Num1, Modifiers::COMMAND);
        assert_eq!(harness.app.active_tab_id(), tab);
        harness.press(Key::T, Modifiers::COMMAND);
        let workspace = harness.app.workspace(tab).unwrap();
        assert!(workspace.active_sql_tab().is_some());
        harness.press(Key::Num2, Modifiers::COMMAND);
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
            let order = if look.terminal {
                ["Limit", "Timeout", "Run", "Run all"]
            } else {
                ["Run", "Run all", "Limit", "Timeout"]
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
    fn the_terminal_strip_has_no_row_panel_toggle_on_a_sql_tab() {
        let (mut harness, tab) = sql_harness(crate::theme::Look::omarchy());
        let toggle = "Show or hide the row panel";
        assert!(!harness.has(toggle), "an editor has no row panel");
        let panel = harness.app.workspace(tab).unwrap().row_panel;
        // A table tab keeps it, and coming back to the editor loses it.
        harness.click("users");
        harness.answer_rows(crate::testing::page(5, false));
        assert!(harness.has(toggle));
        harness.click(toggle);
        assert_ne!(harness.app.workspace(tab).unwrap().row_panel, panel);
        harness.click("Query 1 tab");
        assert!(!harness.has(toggle));
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
                // A menu reads in full or short, never neither.
                assert_eq!(
                    state[4] && !state[2],
                    painted(&harness, short),
                    "{short} at {width} in {}",
                    look.name
                );
                if states.last() != Some(&state) {
                    states.push(state);
                }
            }
            // keys, note, full labels, title, menus
            let mut expected = vec![
                [true, true, true, true, true],
                [false, true, true, true, true],
                [false, false, true, true, true],
            ];
            if look.terminal {
                // The title goes before the labels shorten: the tab says
                // it too.
                expected.push([false, false, true, false, true]);
            }
            let title = !look.terminal;
            expected.push([false, false, false, title, true]);
            expected.push([false, false, false, title, false]);
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
            }],
            primary_key: vec!["id".into()],
            indexes: vec![tabletist_db::IndexInfo {
                name: "users_email_idx".into(),
                columns: vec!["email".into()],
                unique: true,
                primary: false,
                method: None,
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
        harness.app.apply(crate::model::Action::NewConnTab);
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
    fn the_dialog_shows_every_connection_read_only() {
        // The sheet says it under the box; the terminal look after it.
        for (look, said) in [
            (
                crate::theme::Look::standard(),
                "Blocks every write from this app. Every connection is read-only in 0.1.0.",
            ),
            (
                crate::theme::Look::macos(),
                "Blocks every write from this app. Every connection is read-only in 0.1.0.",
            ),
            (crate::theme::Look::omarchy(), "· always on in 0.1.0"),
        ] {
            let mut harness = Harness::new();
            harness.set_look(look);
            harness.press(Key::N, Modifiers::COMMAND);
            let tree = harness.settle();
            let (_, node) = tree
                .nodes
                .iter()
                .find(|(_, node)| {
                    node.role() == egui::accesskit::Role::CheckBox
                        && node.label() == Some("Open read-only")
                })
                .expect("the read-only box");
            assert_eq!(
                node.toggled(),
                Some(egui::accesskit::Toggled::True),
                "{}",
                look.name
            );
            assert!(node.is_disabled(), "{}", look.name);
            assert!(harness.has(said), "{}", look.name);
            // The box is locked: a new connection has nothing set.
            assert_eq!(form(&harness).read_only, None, "{}", look.name);
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
        assert!(harness.has("Reconnect"));
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
                "· always on in 0.1.0"
            } else {
                "Blocks every write from this app. Every connection is read-only in 0.1.0."
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
    fn tabs_share_the_title_bar_with_the_mac_window_buttons() {
        let mut harness = Harness::new();
        mac_title_bar(&mut harness);
        let first = harness.connect_fake();
        // Named apart from the saved connection the new tab's picker lists.
        harness.app.workspace_mut(first).unwrap().name = "Tab one".into();
        harness.app.apply(crate::model::Action::NewConnTab);
        let tree = harness.settle();
        let tab = bounds_of(&tree, "Tab one");
        assert!(tab.x0 >= 80.0, "tabs start after the buttons: {tab:?}");
        let middle = (tab.y0 + tab.y1) / 2.0;
        assert!(
            (middle - 20.0).abs() < 1.0,
            "centred on the buttons' line: {tab:?}"
        );
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
        let first = harness.connect_fake();
        let tree = harness.settle();
        let bar = middle_of(&tree, "Fixture");
        assert!(
            (line(&harness) - bar).abs() < 1.0,
            "the connection bar at {bar}, the buttons at {}",
            line(&harness)
        );
        assert!(line(&harness) > 20.0, "below the title bar's own middle");

        // Several: the tab bar.
        harness.app.workspace_mut(first).unwrap().name = "Tab one".into();
        harness.app.apply(crate::model::Action::NewConnTab);
        let tree = harness.settle();
        let tab = bounds_of(&tree, "Tab one");
        let tabs = (tab.y0 + tab.y1) / 2.0;
        assert!(
            (line(&harness) - tabs).abs() < 1.0,
            "the tabs at {tabs}, the buttons at {}",
            line(&harness)
        );
    }

    #[test]
    fn the_mac_title_bar_is_measured_in_window_points_not_zoomed_ones() {
        let mut harness = Harness::new();
        mac_title_bar(&mut harness);
        let first = harness.connect_fake();
        // Named apart from the saved connection the new tab's picker lists.
        harness.app.workspace_mut(first).unwrap().name = "Tab one".into();
        harness.app.apply(crate::model::Action::NewConnTab);
        harness.ctx.set_zoom_factor(2.0);
        let tree = harness.settle();
        let tab = bounds_of(&tree, "Tab one");
        // 80 window points are 40 egui points at 2x zoom.
        assert!(
            (tab.x0 - 40.0).abs() < 1.0,
            "tabs start after the buttons: {tab:?}"
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
    fn the_space_beside_the_mac_window_buttons_stays_when_the_tabs_scroll() {
        let mut harness = Harness::new();
        mac_title_bar(&mut harness);
        for _ in 0..12 {
            harness.app.apply(crate::model::Action::NewConnTab);
        }
        let before = bounds_of(&harness.settle(), "New tab");
        let over = egui::pos2(400.0, 20.0);
        harness.frame(vec![egui::Event::PointerMoved(over)]);
        harness.frame(vec![
            egui::Event::PointerMoved(over),
            egui::Event::MouseWheel {
                unit: egui::MouseWheelUnit::Point,
                delta: egui::vec2(-400.0, 0.0),
                modifiers: Modifiers::NONE,
                phase: egui::TouchPhase::Move,
            },
        ]);
        for _ in 0..60 {
            harness.frame(vec![]);
        }
        let after = bounds_of(&harness.settle(), "New tab");
        assert!(after.x0 < before.x0 - 100.0, "the tabs scrolled: {after:?}");

        // The corner by the buttons still moves the window.
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

    #[test]
    fn a_disconnected_tab_offers_reconnect_and_edit() {
        let mut harness = Harness::new();
        let tab = harness.connect_fake();
        let session = harness.app.workspace(tab).unwrap().session;
        harness.app.apply(crate::model::Action::Backend(
            crate::backend::Event::Disconnected {
                session,
                error: tabletist_db::Error::ConnectionLost("server closed the connection".into()),
            },
        ));
        assert!(harness.has("The connection was lost."));
        assert!(harness.has("Reconnect"));
        harness.click("Edit connection");
        assert!(matches!(
            harness.app.dialog,
            Some(crate::model::Dialog::Connection(_))
        ));
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
        assert!(harness.has("No rows match the filter"));
        harness.click("Clear filter");
        assert!(matches!(
            harness.app.backend.sent.last(),
            Some(crate::backend::Command::FetchRows { query, .. }) if query.filters.is_empty()
        ));
    }

    #[test]
    fn an_empty_table_says_so() {
        let mut harness = Harness::new();
        let tab = harness.connect_fake();
        harness.app.apply(crate::model::Action::OpenObject {
            tab,
            object: tabletist_db::ObjectRef::new("main", "users"),
            kind: tabletist_db::ObjectKind::Table,
            pin: true,
        });
        harness.answer_rows(crate::testing::page(0, false));
        assert!(harness.has("This table is empty"));
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
        for expected in ["Filter", "orders", "Count", "Next page", "Refresh objects"] {
            assert!(
                reached.contains(expected),
                "{expected} not reached: {reached:?}"
            );
        }
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
}

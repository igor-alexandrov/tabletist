//! The interface. Views read `App` and push `Action`s; they never change
//! state directly.

pub mod conn_tabs;
pub mod connect_dialog;
pub mod data_view;
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
pub mod structure;
pub mod widgets;
pub mod workspace;

use egui::Frame;

use crate::app::App;
use crate::model::ConnTabContent;

pub fn show(app: &mut App, ui: &mut egui::Ui) {
    conn_tabs::show(app, ui);
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
                    ("New connection tab", egui::accesskit::Role::Button),
                    ("Refresh objects", egui::accesskit::Role::Button),
                    ("Row 1", egui::accesskit::Role::Button),
                ] {
                    assert!(
                        crate::testing::node(&tree, label, role).is_some(),
                        "{label} missing in {} at {size:?}",
                        look.name
                    );
                }
                assert!(
                    tree.nodes.iter().any(|(_, node)| {
                        node.role() == egui::accesskit::Role::TextInput
                            && node.placeholder() == Some("Filter")
                    }),
                    "the sidebar filter is missing in {} at {size:?}",
                    look.name
                );
                // The connection dialog keeps its buttons on screen; the
                // PostgreSQL form with the SSH tunnel open is the tallest.
                harness.press(Key::N, Modifiers::COMMAND);
                harness.click("PostgreSQL");
                harness.click("SSH tunnel");
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
    fn the_tab_bar_shows_a_new_tab_and_a_plus_button() {
        let mut harness = Harness::new();
        assert!(harness.has("New tab"));
        assert!(harness.has("New connection tab"));
        assert!(harness.has("Close New tab"));
    }

    #[test]
    fn the_plus_button_opens_a_tab() {
        let mut harness = Harness::new();
        harness.click("New connection tab");
        assert_eq!(harness.app.tabs.len(), 2);
        assert_eq!(harness.app.active, 1);
    }

    #[test]
    fn the_close_button_closes_its_tab() {
        // One tab, so the label "Close New tab" is unambiguous.
        let mut harness = Harness::new();
        let closing = harness.app.tabs[0].id;
        harness.click("Close New tab");
        assert_eq!(harness.app.tabs.len(), 1);
        assert_ne!(harness.app.tabs[0].id, closing);
    }

    #[test]
    fn ctrl_t_opens_a_tab() {
        let mut harness = Harness::new();
        harness.press(Key::T, Modifiers::COMMAND);
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
                crate::testing::node(&tree, "New connection tab", egui::accesskit::Role::Button)
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
            color: crate::connections::ColorTag::Red,
            password: crate::connections::PasswordMode::None,
            ssh_secret: crate::connections::PasswordMode::None,
            spec: tabletist_db::ConnectSpec::sqlite(format!("/tmp/{name}.db")),
        };
        let id = saved.id.clone();
        harness.app.connections.upsert(saved);
        id
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
            assert!(
                (field.x0 as f32 - card.left()).abs() < 1.0,
                "{}: field starts at {}, cards at {}",
                look.name,
                field.x0,
                card.left()
            );
            assert!(
                (button.right() - card.right()).abs() < 1.0,
                "{}: button ends at {}, cards at {}",
                look.name,
                button.right(),
                card.right()
            );
        }
    }

    #[test]
    fn picker_cards_name_the_driver_before_the_summary() {
        let mut harness = Harness::new();
        add_saved(&mut harness, "Production");
        assert!(harness.has("SQLite · Production.db"));
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
                color: crate::connections::ColorTag::Red,
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
        assert!(harness.has("main"));
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
        assert!(harness.has("1–5 of 5"));
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
        assert!(harness.app.workspace(tab).unwrap().objects.is_empty());
        assert!(harness.has("Select a table or view in the sidebar"));
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
        assert!(harness.app.workspace(tab).unwrap().objects.is_empty());
        assert_eq!(
            harness.app.tabs.len(),
            1,
            "Cmd+W closes the object tab, not the connection"
        );
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
            Some(palette.accent)
        );
        assert_eq!(harness.painted_color("3,"), Some(palette.warning));
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
        // The footer's Data toggle sits 8 pt in from the same panel edge.
        assert!(
            left("Columns") > left("Data"),
            "Columns at {}, Data at {}",
            left("Columns"),
            left("Data")
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
        harness.press(Key::T, Modifiers::COMMAND);
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
    fn escape_with_the_color_list_open_keeps_the_dialog() {
        let mut harness = Harness::new();
        harness.press(Key::N, Modifiers::COMMAND);
        let tree = harness.settle();
        let combo = tree
            .nodes
            .iter()
            .find(|(_, node)| node.role() == egui::accesskit::Role::ComboBox)
            .map(|(id, _)| *id)
            .expect("the colour list");
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
        for label in ["Host", "Port", "User", "Password", "Database", "TLS"] {
            assert!(harness.has(label), "{label}");
        }
        assert!(!harness.has("File"));
    }

    #[test]
    fn the_password_prompt_connects_with_the_typed_password() {
        let mut harness = Harness::new();
        let (spec, _) = tabletist_db::ConnectSpec::from_url("postgres://me@db/app").unwrap();
        let saved = crate::connections::SavedConnection {
            id: crate::connections::ConnectionId::new(),
            name: "Prod".into(),
            color: crate::connections::ColorTag::Red,
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
            color: crate::connections::ColorTag::None,
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
        for label in ["Host", "Port", "User", "Password", "Database", "TLS"] {
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
    fn the_ssh_section_shows_the_fields_for_each_method() {
        let mut harness = Harness::new();
        harness.press(Key::N, Modifiers::COMMAND);
        harness.click("PostgreSQL");
        harness.click("SSH tunnel");
        harness.click("Connect through SSH");
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
    fn sqlite_has_no_ssh_section() {
        let mut harness = Harness::new();
        harness.press(Key::N, Modifiers::COMMAND);
        assert!(!harness.has("SSH tunnel"));
    }

    #[test]
    fn the_top_bar_says_the_connection_goes_through_ssh() {
        let mut harness = Harness::new();
        let (mut spec, _) = tabletist_db::ConnectSpec::from_url("postgres://me@db/app").unwrap();
        spec.ssh = Some(tabletist_db::SshSpec {
            host: "bastion".into(),
            port: 22,
            user: "ops".into(),
            auth: tabletist_db::SshAuth::Agent,
        });
        let saved = crate::connections::SavedConnection {
            id: crate::connections::ConnectionId::new(),
            name: "Prod".into(),
            color: crate::connections::ColorTag::Red,
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
            color: crate::connections::ColorTag::Red,
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
                palette.secondary
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
        assert!(harness.has("1–3 of 1,234"));
        assert!(!harness.has("Count"));
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
        let object = workspace
            .object_tab(workspace.active_object.unwrap())
            .unwrap();
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
        let object_tab = workspace.active_object.unwrap();
        assert_eq!(
            workspace.object_tab(object_tab).unwrap().selection,
            None,
            "the grid did not move"
        );
        harness.app.apply(crate::model::Action::SelectCell {
            tab,
            object_tab,
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
        harness.press(Key::Escape, Modifiers::NONE);
        assert!(harness.app.dialog.is_none());
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
        let object = workspace
            .object_tab(workspace.active_object.unwrap())
            .unwrap();
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
        let id = harness.app.workspace(tab).unwrap().active_object.unwrap();
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
        assert!(harness.has("1–3"));
        assert!(!harness.has("1–3 of ~1.2M"));
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
        harness.settle();
        assert!(harness.has("t_000"));
        harness.app.apply(crate::model::Action::SetTreeCursor {
            tab,
            node: crate::model::TreeNode::Schema("main".into()),
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
        let tree = harness.settle();
        let tab = bounds_of(&tree, "New tab");
        assert!(tab.x0 >= 80.0, "tabs start after the buttons: {tab:?}");
        let middle = (tab.y0 + tab.y1) / 2.0;
        assert!(
            (middle - 20.0).abs() < 1.0,
            "centred on the buttons' line: {tab:?}"
        );
    }

    #[test]
    fn the_mac_title_bar_is_measured_in_window_points_not_zoomed_ones() {
        let mut harness = Harness::new();
        mac_title_bar(&mut harness);
        harness.ctx.set_zoom_factor(2.0);
        let tree = harness.settle();
        let tab = bounds_of(&tree, "New tab");
        // 80 window points are 40 egui points at 2x zoom.
        assert!(
            (tab.x0 - 40.0).abs() < 1.0,
            "tabs start after the buttons: {tab:?}"
        );
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
        let id = harness.app.workspace(tab).unwrap().active_object.unwrap();
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
    fn a_database_without_visible_schemas_says_so() {
        let (mut harness, tab) = tree_harness();
        harness.app.workspace_mut(tab).unwrap().tree.schemas.value = Some(Vec::new());
        assert!(harness.has("This database has no schemas you can see."));
    }

    /// Interactive roles a screen reader announces; each needs a name.
    const NAMED: [egui::accesskit::Role; 6] = [
        egui::accesskit::Role::Button,
        egui::accesskit::Role::TextInput,
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
            object_tab: harness
                .app
                .workspace(harness.app.active_tab_id())
                .unwrap()
                .active_object
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

        let (mut harness, tab) = tree_harness();
        with_database_picker(&mut harness, tab);
        scenes.push(("database picker", harness.settle()));

        let (mut harness, _tab) = empty_schema_harness();
        scenes.push(("empty schema", harness.settle()));
        let (mut harness, tab) = no_schemas_harness();
        with_database_picker(&mut harness, tab);
        scenes.push(("no schemas", harness.settle()));

        let mut harness = Harness::new();
        harness.press(Key::N, Modifiers::COMMAND);
        harness.click("PostgreSQL");
        harness.click("SSH tunnel");
        harness.click("Connect through SSH");
        scenes.push(("connection dialog", harness.settle()));

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

    fn users_with_bar(
        harness: &mut Harness,
    ) -> (crate::model::ConnTabId, crate::model::ObjectTabId) {
        let tab = harness.connect_fake();
        harness.app.apply(crate::model::Action::OpenObject {
            tab,
            object: tabletist_db::ObjectRef::new("main", "users"),
            kind: tabletist_db::ObjectKind::Table,
            pin: true,
        });
        harness.answer_rows(crate::testing::page(3, false));
        let id = harness.app.workspace(tab).unwrap().active_object.unwrap();
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
}

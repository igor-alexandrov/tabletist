//! Screenshots for visual review: `cargo test --features shots --lib
//! shots -- --ignored` writes PNGs to target/shots/ (Retina scale, light and
//! dark). Needs a GPU (wgpu); never part of the normal suite.

#![allow(clippy::unwrap_used)]

use std::path::PathBuf;
use std::time::Duration;

use tabletist_db::{
    ColumnInfo, ColumnMeta, ConnectSpec, Driver, ForeignKeyInfo, IndexInfo, ObjectInfo, ObjectKind,
    ObjectRef, RowPage, Structure, Value, ValueKind,
};

use crate::connections::{ColorTag, ConnectionId, PasswordMode, SavedConnection};
use crate::model::{Action, CellPos, ConnTabId};
use crate::testing::Harness;

const SIZE: egui::Vec2 = egui::vec2(1000.0, 650.0);

fn out(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target/shots")
        .join(name)
}

fn saved() -> SavedConnection {
    let (spec, _) =
        ConnectSpec::from_url("postgres://igor@127.0.0.1:5433/safari_development").unwrap();
    SavedConnection {
        id: ConnectionId::new(),
        name: "Safari Portal".into(),
        color: ColorTag::Green,
        password: PasswordMode::None,
        ssh_secret: PasswordMode::None,
        spec,
    }
}

const TABLES: [&str; 30] = [
    "account_bonding_logos",
    "account_brand_settings",
    "account_business_profiles",
    "account_companies",
    "account_exchange_rates",
    "account_favicons",
    "account_flowbrite_integrations",
    "account_flywire_integrations",
    "account_google_integrations",
    "account_gtm_containers",
    "account_landing_configs",
    "account_logos",
    "account_nmi_integrations",
    "account_pci_vault_integrations",
    "account_pps_integrations",
    "account_privacy_policy_settings",
    "account_quickbooks_customers",
    "account_quickbooks_integrations",
    "account_redpoint_integrations",
    "account_saved_inputs",
    "account_stripe_connect_integrations",
    "account_subscriptions",
    "accounts",
    "accounts_custom_mobile_apps",
    "accounts_font_families",
    "action_buttons",
    "activity_events",
    "addresses",
    "admin_users",
    "api_tokens",
];

fn page() -> RowPage {
    let column = |name: &str, type_name: &str, kind| ColumnMeta {
        name: name.into(),
        type_name: type_name.into(),
        kind,
    };
    let rows = (0..13i64)
        .map(|i| {
            vec![
                Value::Int(i + 2),
                Value::Int(1_628_910_071_209_526_786 + i * 7_919),
                Value::Text(if i % 2 == 0 { "atol" } else { "abta" }.into()),
                Value::Text(format!(
                    r#"{{"id": "account/bondinglogo/{}/logo/aea111f831796e6828e311cfdb791394.png", "metadata": {{"filename": "badge (1).png", "height": 504, "mime_type": "image/png", "size": 29795, "width": 1116}}, "storage": "store"}}"#,
                    i + 2
                ).into()),
                Value::Text("2026-06-03 15:47:52.977704".into()),
                Value::Null,
            ]
        })
        .collect();
    RowPage {
        columns: vec![
            column("id", "int8", ValueKind::Numeric),
            column("account_id", "int8", ValueKind::Numeric),
            column("kind", "varchar", ValueKind::Text),
            column("logo_data", "jsonb", ValueKind::Json),
            column("created_at", "timestamp", ValueKind::Temporal),
            column("deleted_at", "timestamp", ValueKind::Temporal),
        ],
        rows,
        has_more: false,
        ordered_by_key: true,
        elapsed: Duration::from_millis(3),
    }
}

/// What `account_bonding_logos` describes as: the columns of [`page`], its
/// primary key, two indexes and one foreign key.
fn structure() -> Structure {
    let column = |name: &str, type_name: &str, nullable, default: Option<&str>| ColumnInfo {
        name: name.into(),
        type_name: type_name.into(),
        nullable,
        default: default.map(Into::into),
        comment: None,
    };
    Structure {
        columns: vec![
            column(
                "id",
                "bigint",
                false,
                Some("nextval('account_bonding_logos_id_seq'::regclass)"),
            ),
            column("account_id", "bigint", false, None),
            column(
                "kind",
                "character varying",
                false,
                Some("'atol'::character varying"),
            ),
            column("logo_data", "jsonb", true, None),
            column("created_at", "timestamp(6) without time zone", false, None),
            column("deleted_at", "timestamp(6) without time zone", true, None),
        ],
        primary_key: vec!["id".into()],
        indexes: vec![
            IndexInfo {
                name: "account_bonding_logos_pkey".into(),
                columns: vec!["id".into()],
                unique: true,
                primary: true,
                method: Some("btree".into()),
            },
            IndexInfo {
                name: "index_account_bonding_logos_on_account_id_and_kind".into(),
                columns: vec!["account_id".into(), "kind".into()],
                unique: true,
                primary: false,
                method: Some("btree".into()),
            },
        ],
        foreign_keys: vec![ForeignKeyInfo {
            name: Some("fk_rails_3b1d8f0c2e".into()),
            columns: vec!["account_id".into()],
            ref_schema: "public".into(),
            ref_table: "accounts".into(),
            ref_columns: vec!["id".into()],
            on_update: "NO ACTION".into(),
            on_delete: "CASCADE".into(),
        }],
    }
}

/// A connected PostgreSQL-looking tab with a table open and row 5 selected.
fn workspace(harness: &mut Harness) -> ConnTabId {
    let tab = harness.connect_fake();
    let saved = saved();
    let workspace = harness.app.workspace_mut(tab).unwrap();
    workspace.name = saved.name.clone();
    workspace.color = saved.color;
    workspace.spec = saved.spec.clone();
    workspace.driver = Driver::Postgres;
    workspace.tree.schemas.value = Some(vec!["public".into()]);
    let node = workspace.tree.nodes.entry("public".into()).or_default();
    node.expanded = true;
    node.objects.value = Some(
        TABLES
            .iter()
            .map(|name| ObjectInfo {
                name: (*name).into(),
                kind: ObjectKind::Table,
                estimated_rows: Some(13),
            })
            .collect(),
    );
    harness.app.apply(Action::OpenObject {
        tab,
        object: ObjectRef::new("public", "account_bonding_logos"),
        kind: ObjectKind::Table,
        pin: true,
    });
    harness.answer_rows(page());
    let object_tab = harness.app.workspace(tab).unwrap().active_object.unwrap();
    harness.app.apply(Action::SelectCell {
        tab,
        object_tab,
        cell: CellPos { row: 4, col: 0 },
    });
    tab
}

fn both(name: &str, scene: impl Fn(&mut Harness)) {
    for look in crate::theme::Look::ALL {
        for (light, suffix) in [(true, "light"), (false, "dark")] {
            let mut harness = Harness::for_shots(SIZE, 2.0, light, look);
            if look == crate::theme::Look::macos() {
                // Measured on a macOS 26 window with a unified compact toolbar.
                harness.app.titlebar = crate::app::TitleBar {
                    height: 38.0,
                    inset: 80.0,
                };
            }
            scene(&mut harness);
            harness.shot(&out(&format!("{name}-{}-{suffix}.png", look.name)));
        }
    }
}

#[test]
#[ignore = "renders with wgpu; run with --features shots -- --ignored"]
fn shots() {
    both("picker", |harness| {
        harness.app.connections.upsert(saved());
    });
    both("workspace", |harness| {
        workspace(harness);
    });
    both("filter", |harness| {
        workspace(harness);
        harness.press(egui::Key::F, egui::Modifiers::COMMAND);
    });
    both("dialog", |harness| {
        harness.press(egui::Key::N, egui::Modifiers::COMMAND);
        harness.click("PostgreSQL");
    });
    both("state-disconnected", |harness| {
        let tab = workspace(harness);
        let session = harness.app.workspace(tab).unwrap().session;
        harness
            .app
            .apply(Action::Backend(crate::backend::Event::Disconnected {
                session,
                error: tabletist_db::Error::ConnectionLost(
                    "server closed the connection unexpectedly".into(),
                ),
            }));
    });
    both("state-empty", |harness| {
        let tab = workspace(harness);
        let workspace = harness.app.workspace_mut(tab).unwrap();
        workspace.tree.schemas.value = Some(vec!["public".into(), "reports".into()]);
        // Collapsed, so the empty schema below it is on screen.
        workspace.tree.nodes.get_mut("public").unwrap().expanded = false;
        let node = workspace.tree.nodes.entry("reports".into()).or_default();
        node.expanded = true;
        node.objects.value = Some(Vec::new());
        let object_tab = workspace.active_object.unwrap();
        let object = workspace.object_tab_mut(object_tab).unwrap();
        object.filter.rows = vec![crate::model::FilterRow {
            column: "kind".into(),
            op: tabletist_db::FilterOp::Eq,
            value: "iata".into(),
        }];
        harness.app.apply(Action::ApplyFilters { tab, object_tab });
        let mut empty = page();
        empty.rows.clear();
        harness.answer_rows(empty);
    });
    both("state-no-schemas", |harness| {
        let tab = workspace(harness);
        let workspace = harness.app.workspace_mut(tab).unwrap();
        workspace.tree.schemas.value = Some(Vec::new());
        workspace.databases.value = Some(vec![
            "safari_development".into(),
            "safari_test".into(),
            "postgres".into(),
        ]);
    });
    both("state-error", |harness| {
        let tab = workspace(harness);
        let object_tab = harness.app.workspace(tab).unwrap().active_object.unwrap();
        harness.app.apply(Action::RetryRows { tab, object_tab });
        let (session, request) = match harness.app.backend.sent.last() {
            Some(crate::backend::Command::FetchRows {
                session, request, ..
            }) => (*session, *request),
            other => panic!("{other:?}"),
        };
        harness
            .app
            .apply(Action::Backend(crate::backend::Event::Rows {
                session,
                request,
                result: Err(tabletist_db::Error::Query {
                    code: Some("42703".into()),
                    message: "column \"kindd\" does not exist".into(),
                    detail: None,
                    hint: Some("Perhaps you meant to reference the column \"kind\".".into()),
                }),
            }));
    });
    both("structure", |harness| {
        let tab = workspace(harness);
        let object_tab = harness.app.workspace(tab).unwrap().active_object.unwrap();
        harness.app.apply(Action::SetView {
            tab,
            object_tab,
            view: crate::model::ObjectView::Structure,
        });
        harness.answer_structure(structure());
    });
}

#[test]
#[ignore = "renders with wgpu; run with --features shots -- --ignored"]
fn sidebar_width_over_time() {
    let mut harness = Harness::for_shots(SIZE, 2.0, true, crate::theme::Look::standard());
    workspace(&mut harness);
    for frame in 0..300 {
        let tree = harness.frame(Vec::new());
        if frame % 50 == 0 {
            let refresh = crate::testing::node(&tree, "Refresh", egui::accesskit::Role::Button)
                .and_then(|id| tree.nodes.iter().find(|(n, _)| *n == id))
                .and_then(|(_, node)| node.bounds());
            eprintln!("frame {frame}: refresh button {refresh:?}");
        }
    }
    harness.shot(&out("workspace-after-300-frames.png"));
}

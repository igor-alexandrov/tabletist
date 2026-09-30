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

/// A made-up bookshop: screenshots never show a real project's names or data.
fn saved() -> SavedConnection {
    let (spec, _) =
        ConnectSpec::from_url("postgres://demo@localhost:5432/bookshop_development").unwrap();
    SavedConnection {
        id: ConnectionId::new(),
        name: "Bookshop".into(),
        color: ColorTag::Green,
        password: PasswordMode::None,
        ssh_secret: PasswordMode::None,
        spec,
    }
}

const TABLES: [&str; 30] = [
    "addresses",
    "authors",
    "book_authors",
    "book_categories",
    "book_images",
    "book_reviews",
    "books",
    "carts",
    "cart_items",
    "categories",
    "coupons",
    "customer_preferences",
    "customers",
    "gift_cards",
    "inventory_movements",
    "invoices",
    "newsletter_subscriptions",
    "order_items",
    "orders",
    "payment_methods",
    "payments",
    "publishers",
    "refunds",
    "returns",
    "sessions",
    "shipments",
    "shipping_zones",
    "stores",
    "suppliers",
    "wishlists",
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
                Value::Int(1_048_576 + i * 7_919),
                Value::Text(if i % 2 == 0 { "cover" } else { "preview" }.into()),
                Value::Text(format!(
                    r#"{{"id": "books/{}/images/3f9c2a7d.png", "metadata": {{"filename": "cover.png", "height": 1600, "mime_type": "image/png", "size": 184320, "width": 1024}}, "storage": "store"}}"#,
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
            column("book_id", "int8", ValueKind::Numeric),
            column("kind", "varchar", ValueKind::Text),
            column("image_data", "jsonb", ValueKind::Json),
            column("created_at", "timestamp", ValueKind::Temporal),
            column("deleted_at", "timestamp", ValueKind::Temporal),
        ],
        rows,
        has_more: false,
        ordered_by_key: true,
        elapsed: Duration::from_millis(3),
    }
}

/// What `book_images` describes as: the columns of [`page`], its
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
                Some("nextval('book_images_id_seq'::regclass)"),
            ),
            column("book_id", "bigint", false, None),
            column(
                "kind",
                "character varying",
                false,
                Some("'cover'::character varying"),
            ),
            column("image_data", "jsonb", true, None),
            column("created_at", "timestamp(6) without time zone", false, None),
            column("deleted_at", "timestamp(6) without time zone", true, None),
        ],
        primary_key: vec!["id".into()],
        indexes: vec![
            IndexInfo {
                name: "book_images_pkey".into(),
                columns: vec!["id".into()],
                unique: true,
                primary: true,
                method: Some("btree".into()),
            },
            IndexInfo {
                name: "index_book_images_on_book_id_and_kind".into(),
                columns: vec!["book_id".into(), "kind".into()],
                unique: true,
                primary: false,
                method: Some("btree".into()),
            },
        ],
        foreign_keys: vec![ForeignKeyInfo {
            name: Some("fk_rails_3b1d8f0c2e".into()),
            columns: vec!["book_id".into()],
            ref_schema: "public".into(),
            ref_table: "books".into(),
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
    workspace.databases.value = Some(vec![
        "bookshop_development".into(),
        "bookshop_test".into(),
        "postgres".into(),
    ]);
    // A second, inactive tab next to the active one.
    for name in ["books", "book_images"] {
        harness.app.apply(Action::OpenObject {
            tab,
            object: ObjectRef::new("public", name),
            kind: ObjectKind::Table,
            pin: true,
        });
        harness.answer_rows(page());
    }
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
            value: "cover".into(),
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
            "bookshop_development".into(),
            "bookshop_test".into(),
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

/// Scenes at the design mockups' size: 1.5 pixels per point, so each PNG
/// lines up pixel for pixel with its mockup.
mod mock {
    use super::*;

    /// The macOS mockups: 2000 x 1250 pixels.
    const MAC: egui::Vec2 = egui::vec2(2000.0 / 1.5, 1250.0 / 1.5);
    /// The Omarchy workspace mockup's window, inside the tiling border.
    const OMARCHY: egui::Vec2 = egui::vec2(1974.0 / 1.5, 1098.0 / 1.5);
    /// The Omarchy picker mockup's window, drawn at 2x.
    const OMARCHY_PICKER: egui::Vec2 = egui::vec2(1798.0 / 2.0, 1952.0 / 2.0);

    /// Tokyo Night through the Omarchy template, as the mockups draw it.
    pub fn tokyo_night() -> crate::theme::Palette {
        let colors = "mode\tdark\nbackground\t#1a1b26\ndark_background\t#16161e\n\
                      lighter_background\t#292e42\nforeground\t#c0caf5\nmuted\t#565f89\n\
                      accent\t#7aa2f7\nselection\t#283457\nred\t#f7768e\ngreen\t#9ece6a\n\
                      yellow\t#e0af68\ncyan\t#7dcfff\norange\t#ff9e64\nmagenta\t#bb9af7\n";
        let rendered = fastframe_theme::omarchy::render_seed::<crate::theme::Palette>(
            include_str!("../contrib/omarchy/tabletist.json.tpl"),
            colors,
        )
        .unwrap();
        fastframe_theme::parse_palette::<crate::theme::Palette>(&rendered)
            .unwrap()
            .with_readable_labels()
    }

    fn harness(size: egui::Vec2, look: crate::theme::Look) -> Harness {
        let scale = if size == OMARCHY_PICKER { 2.0 } else { 1.5 };
        let light = look == crate::theme::Look::macos();
        let mut harness = Harness::for_shots(size, scale, light, look);
        if !light {
            harness.app.palette = tokyo_night();
            crate::theme::apply(&harness.ctx, &harness.app.palette, &look);
        }
        harness
    }

    /// Twenty-one `book_` objects (three views), then the other groups.
    fn objects() -> Vec<ObjectInfo> {
        let table = |name: String| ObjectInfo {
            name,
            kind: ObjectKind::Table,
            estimated_rows: Some(13),
        };
        let view = |name: &str| ObjectInfo {
            name: name.into(),
            kind: ObjectKind::View,
            estimated_rows: None,
        };
        let mut objects = vec![
            view("book_active_covers"),
            view("book_cover_counts"),
            view("book_unused_images"),
        ];
        for name in [
            "authors",
            "awards",
            "blurbs",
            "bundles",
            "covers",
            "editions",
            "excerpts",
            "formats",
            "genres",
            "imprints",
            "isbns",
            "prices",
            "ratings",
            "reviews",
            "series",
            "tags",
            "translations",
            "watermarks",
        ] {
            objects.push(table(format!("book_{name}")));
        }
        for name in [
            "accounts",
            "credits",
            "discounts",
            "invoices",
            "ledgers",
            "payments",
            "payouts",
            "plans",
            "refunds",
            "taxes",
        ] {
            objects.push(table(format!("billing_{name}")));
        }
        for name in [
            "orders",
            "orders_archive",
            "orders_events",
            "orders_items",
            "orders_notes",
            "orders_returns",
        ] {
            objects.push(table(name.into()));
        }
        for name in [
            "users",
            "users_addresses",
            "users_devices",
            "users_emails",
            "users_prefs",
            "users_roles",
            "users_sessions",
        ] {
            objects.push(table(name.into()));
        }
        objects
    }

    const STAMPS: [(&str, &str, &str); 13] = [
        (
            "2026-01-12 09:14:03.482915",
            "2026-01-12 09:14:03.571204",
            "7b1e04c2a5f8d3160e9b4a27c81d05f3",
        ),
        (
            "2026-01-12 17:02:41.118302",
            "2026-01-12 17:02:41.204511",
            "3fa9d6e10b27c4d58a1e6f3b92c07d44",
        ),
        (
            "2026-02-03 10:26:55.730019",
            "2026-02-03 10:26:56.001877",
            "c05b8e7f4d21a9360be5c14f7a82d93e",
        ),
        (
            "2026-02-05 07:48:19.265530",
            "2026-02-05 07:48:19.340012",
            "58d2a1b93e6f0c47d12b8a5e9f3c6071",
        ),
        (
            "2026-02-05 14:31:08.902144",
            "2026-02-05 14:31:08.977320",
            "e4c7093fd2b18a65c3f0e9d47b21a58c",
        ),
        (
            "2026-02-06 08:05:47.011938",
            "2026-02-06 08:05:47.090215",
            "9a61f2c05e3d8b47a0c19f6e28d73b54",
        ),
        (
            "2026-02-06 08:06:12.548702",
            "2026-02-06 08:06:12.611409",
            "2d8b4e6a13f07c95b2e4d81a6c3f90e7",
        ),
        (
            "2026-02-11 12:40:36.377115",
            "2026-02-11 12:40:37.004263",
            "f173c9d284a6b0e53d1f7c2e98a4b605",
        ),
        (
            "2026-02-13 09:22:50.640881",
            "2026-02-13 09:22:50.712093",
            "6ce05a4b7d913f28e6a0c5b1f47d29e8",
        ),
        (
            "2026-02-13 09:23:14.199236",
            "2026-02-13 09:23:14.265870",
            "b4892f1e6a0d7c53f9b2e18d4a6c07f3",
        ),
        (
            "2026-03-04 11:57:02.834470",
            "2026-03-04 11:57:02.901158",
            "05ad7c3e9b1f46d28a0e7c5b3d91f24a",
        ),
        (
            "2026-03-04 11:59:27.456093",
            "2026-03-04 11:59:27.528714",
            "d9e6b1084f3a2c7e5b90d16a8f4c32e1",
        ),
        (
            "2026-03-10 15:18:44.120567",
            "2026-03-10 15:18:44.193342",
            "41f8a2c6d0e7b395f1a4c8e62d07b9a3",
        ),
    ];

    /// `book_covers`, sorted by `created_at`: the mockups' badge table.
    fn covers() -> RowPage {
        let column = |name: &str, type_name: &str, kind| ColumnMeta {
            name: name.into(),
            type_name: type_name.into(),
            kind,
        };
        let books = [1, 2, 1, 3, 4, 5, 5, 6, 7, 7, 8, 8, 9];
        let rows = STAMPS
            .iter()
            .enumerate()
            .map(|(i, (created, updated, hash))| {
                let id = i as i64 + 2;
                let kind = if [0, 4, 6, 7, 9, 10, 12].contains(&i) { "front" } else { "back" };
                vec![
                    Value::Int(id),
                    Value::Int(4_700_000_000_000_000_000 + books[i]),
                    Value::Text(kind.into()),
                    Value::Text(format!(
                        r#"{{"id": "book/cover/{id}/image/{hash}.jpg", "storage": "store", "metadata": {{"size": {}, "width": 103, "height": 102, "filename": "{kind}-cover.jpg", "mime_type": "image/jpeg"}}}}"#,
                        6024 + i * 37
                    ).into()),
                    Value::Text((*created).into()),
                    Value::Text((*updated).into()),
                ]
            })
            .collect();
        RowPage {
            columns: vec![
                column("id", "int8", ValueKind::Numeric),
                column("book_id", "int8", ValueKind::Numeric),
                column("kind", "varchar", ValueKind::Text),
                column("image_data", "jsonb", ValueKind::Json),
                column("created_at", "timestamp", ValueKind::Temporal),
                column("updated_at", "timestamp", ValueKind::Temporal),
            ],
            rows,
            has_more: false,
            ordered_by_key: true,
            elapsed: Duration::from_millis(2),
        }
    }

    fn covers_structure() -> Structure {
        let column = |name: &str, type_name: &str, nullable| ColumnInfo {
            name: name.into(),
            type_name: type_name.into(),
            nullable,
            default: None,
            comment: None,
        };
        Structure {
            columns: vec![
                column("id", "bigint", false),
                column("book_id", "bigint", false),
                column("kind", "character varying", false),
                column("image_data", "jsonb", true),
                column("created_at", "timestamp(6) without time zone", false),
                column("updated_at", "timestamp(6) without time zone", false),
            ],
            primary_key: vec!["id".into()],
            indexes: Vec::new(),
            foreign_keys: vec![ForeignKeyInfo {
                name: Some("fk_rails_book_covers_books".into()),
                columns: vec!["book_id".into()],
                ref_schema: "public".into(),
                ref_table: "books".into(),
                ref_columns: vec!["id".into()],
                on_update: "NO ACTION".into(),
                on_delete: "CASCADE".into(),
            }],
        }
    }

    /// Bookshop, connected, with `book_covers` open and its first row
    /// selected, `orders` in a second tab.
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
        node.objects.value = Some(objects());
        workspace.databases.value =
            Some(vec!["bookshop_development".into(), "bookshop_test".into()]);
        for name in ["orders", "book_covers"] {
            harness.app.apply(Action::OpenObject {
                tab,
                object: ObjectRef::new("public", name),
                kind: ObjectKind::Table,
                pin: true,
            });
            harness.answer_structure(covers_structure());
            harness.answer_rows(covers());
        }
        let object_tab = harness.app.workspace(tab).unwrap().active_object.unwrap();
        harness.app.apply(Action::SortBy {
            tab,
            object_tab,
            column: "created_at".into(),
        });
        harness.answer_rows(covers());
        // The tabs in the order the mockups show them.
        let workspace = harness.app.workspace_mut(tab).unwrap();
        workspace.objects.reverse();
        harness.app.apply(Action::SelectCell {
            tab,
            object_tab,
            cell: CellPos { row: 0, col: 0 },
        });
        tab
    }

    #[test]
    #[ignore = "renders with wgpu; run with --features shots -- --ignored"]
    fn mock() {
        use crate::theme::Look;
        let mut mac = harness(MAC, Look::macos());
        workspace(&mut mac);
        mac.shot(&out("mock-workspace-macos.png"));

        let mut omarchy = harness(OMARCHY, Look::omarchy());
        let tab = workspace(&mut omarchy);
        let object_tab = omarchy.app.workspace(tab).unwrap().active_object.unwrap();
        let object = omarchy
            .app
            .workspace_mut(tab)
            .unwrap()
            .object_tab_mut(object_tab)
            .unwrap();
        object.filter.raw = true;
        object.filter.raw_text = "kind = 'front'".into();
        omarchy.app.apply(Action::ApplyFilters { tab, object_tab });
        omarchy.answer_rows(covers());
        omarchy.app.apply(Action::SelectCell {
            tab,
            object_tab,
            cell: CellPos { row: 0, col: 0 },
        });
        omarchy.shot(&out("mock-workspace-omarchy.png"));

        let mut mac = harness(MAC, Look::macos());
        pickers(&mut mac);
        mac.shot(&out("mock-picker-macos.png"));
        let mut omarchy = harness(OMARCHY_PICKER, Look::omarchy());
        pickers(&mut omarchy);
        omarchy.shot(&out("mock-picker-omarchy.png"));
    }

    /// Five saved connections, as the picker mockups list them.
    fn pickers(harness: &mut Harness) {
        let entries = [
            (
                "Bookshop",
                ColorTag::Green,
                "postgres://dev@localhost:5433/bookshop_development",
            ),
            (
                "Bookshop",
                ColorTag::Orange,
                "postgres://app_readonly@db.staging.example.com:5432/bookshop_staging?sslmode=verify-full",
            ),
            (
                "Bookshop",
                ColorTag::Red,
                "postgres://app_readonly@db.example.com:5432/bookshop_production?sslmode=verify-full",
            ),
            (
                "Playground",
                ColorTag::Purple,
                "postgres://postgres@localhost:5432/playground",
            ),
            (
                "Rails test",
                ColorTag::None,
                "postgres://dev@localhost:5433/bookshop_test",
            ),
        ];
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let today = now - now % 86_400 + 43_200;
        let used = [
            Some(now - 120),
            Some(today - 86_400),
            Some(today - 17 * 86_400),
            Some(today - 30 * 86_400),
            None,
        ];
        for ((name, color, url), used) in entries.into_iter().zip(used) {
            let (mut spec, _) = ConnectSpec::from_url(url).unwrap();
            if name == "Bookshop" && color != ColorTag::Green {
                spec.ssh = Some(tabletist_db::SshSpec {
                    host: "bastion".into(),
                    port: 22,
                    user: "deploy".into(),
                    auth: tabletist_db::SshAuth::Agent,
                });
            }
            let id = ConnectionId::new();
            harness.app.connections.upsert(SavedConnection {
                id: id.clone(),
                name: name.into(),
                color,
                password: PasswordMode::None,
                ssh_secret: PasswordMode::None,
                spec,
            });
            if let Some(used) = used {
                harness.app.connections.mark_used(&id, used);
            }
        }
        let first = crate::ui::picker::visible(&harness.app, "")
            .first()
            .cloned();
        let tab = harness.app.active_tab_id();
        harness
            .app
            .apply(Action::SelectConnection { tab, conn: first });
    }
}

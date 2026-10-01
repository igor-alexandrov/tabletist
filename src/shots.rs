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

use crate::connections::{ConnectionId, PasswordMode, SavedConnection};
use crate::env::Environment;
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
        environment: Environment::Dev,
        read_only: None,
        password: PasswordMode::None,
        ssh_secret: PasswordMode::None,
        spec,
    }
}

/// The Bookshop's production database behind a bastion, open in the
/// connection dialog after a Test that passed: the scene the dialog's
/// mockups show.
fn edit_production(harness: &mut Harness) {
    let (mut spec, _) = ConnectSpec::from_url(
        "postgres://app_readonly@db.example.com:5432/bookshop_production?sslmode=verify-full",
    )
    .unwrap();
    spec.ca_file = Some("ca-bundle.pem".into());
    spec.ssh = Some(tabletist_db::SshSpec {
        host: "bastion.example.com".into(),
        port: Some(22),
        user: "deploy".into(),
        auth: tabletist_db::SshAuth::KeyFile {
            path: "~/.ssh/id_ed25519".into(),
        },
    });
    // The picker's scene has this connection already: edit that one.
    let id = harness
        .app
        .connections
        .connections
        .iter()
        .find(|connection| connection.spec.database == "bookshop_production")
        .map_or_else(ConnectionId::new, |connection| connection.id.clone());
    harness.app.connections.upsert(SavedConnection {
        id: id.clone(),
        name: "Bookshop".into(),
        environment: Environment::Production,
        read_only: None,
        password: PasswordMode::Keyring,
        ssh_secret: PasswordMode::None,
        spec,
    });
    harness.app.apply(Action::EditConnection(id));
    if let Some(crate::model::Dialog::Connection(form)) = &mut harness.app.dialog {
        form.test = crate::model::TestState::Passed;
        form.test_took = Some(Duration::from_millis(42));
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
    page_of(13)
}

/// The first `rows` rows of `book_images`.
fn page_of(rows: i64) -> RowPage {
    let column = |name: &str, type_name: &str, kind| ColumnMeta {
        name: name.into(),
        type_name: type_name.into(),
        kind,
    };
    let rows = (0..rows)
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
        // `kind` has a CHECK (kind IN ('cover', 'preview')).
        allowed_values: (name == "kind").then(|| vec!["cover".into(), "preview".into()]),
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
    workspace.environment = saved.environment;
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
    let object_tab = harness.app.workspace(tab).unwrap().active_tab.unwrap();
    harness.app.apply(Action::SelectCell {
        tab,
        id: object_tab,
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
        edit_production(harness);
    });
    both("dialog-new", |harness| {
        harness.press(egui::Key::N, egui::Modifiers::COMMAND);
        let postgres = harness.app.look.label("PostgreSQL");
        harness.click(&postgres);
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
        let object_tab = workspace.active_tab.unwrap();
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
        let object_tab = harness.app.workspace(tab).unwrap().active_tab.unwrap();
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
        let object_tab = harness.app.workspace(tab).unwrap().active_tab.unwrap();
        harness.app.apply(Action::SetView {
            tab,
            object_tab,
            view: crate::model::ObjectView::Structure,
        });
        harness.answer_structure(structure());
    });
    both("sql", |harness| {
        sql_editor(harness);
    });
    both("sql-menu", |harness| {
        sql_editor(harness);
        harness.click("Timeout");
    });
    // The cursor in the first statement, which fails on its second line
    // (the position counts from the comment the statement starts with).
    both("sql-error", |harness| {
        let tab = sql_editor(harness);
        for _ in 0..3 {
            harness.press(egui::Key::ArrowUp, egui::Modifiers::NONE);
        }
        let sql_tab = harness.app.workspace(tab).unwrap().active_tab.unwrap();
        harness.app.apply(Action::RunSql {
            tab,
            sql_tab,
            all: false,
        });
        let message = "relation \"book_images\" does not exist";
        let failed = crate::testing::error_outcome(message, Some(63));
        harness.answer_sql(Ok(crate::testing::script_outcome(vec![failed])), None);
    });
    // The results' other states: nothing run yet, a run on its way, every
    // statement in Messages, a refusal, a cancel and a cut result. Each
    // answer is one its script could get.
    both("sql-idle", |harness| {
        let tab = workspace(harness);
        harness.app.apply(Action::NewSqlTab(tab));
    });
    both("sql-running", |harness| {
        let tab = sql_editor(harness);
        run_sql(harness, tab, true);
    });
    // The first statement names a column that is not there, on its second
    // line (the position counts from the comment it starts with).
    both("sql-messages", |harness| {
        let script = SCRIPT
            .replace("SELECT kind", "SELECT kindd")
            .replace("BY kind", "BY kindd");
        let tab = sql_script(harness, &script);
        run_sql(harness, tab, true);
        let failed = tabletist_db::StatementOutcome::Error {
            error: tabletist_db::Error::Query {
                code: Some("42703".into()),
                message: "column \"kindd\" does not exist".into(),
                detail: None,
                hint: Some("Perhaps you meant to reference the column \"kind\".".into()),
            },
            position: Some(31),
        };
        harness.answer_sql(Ok(crate::testing::script_outcome(vec![failed])), None);
    });
    // The script's sixth line would end the transaction.
    both("sql-refused", |harness| {
        let script = SCRIPT.replace("SELECT * FROM book_images", "COMMIT");
        let tab = sql_script(harness, &script);
        run_sql(harness, tab, true);
        let refused = tabletist_db::Error::Refused {
            line: 6,
            what: "COMMIT".into(),
        };
        harness.answer_sql(Err(refused), None);
        let sql_tab = harness.app.workspace(tab).unwrap().active_tab.unwrap();
        harness.app.apply(Action::SetResultPane {
            tab,
            sql_tab,
            pane: crate::model::ResultPane::Results,
        });
    });
    // The first statement counts the two kinds; the second runs out of time.
    both("sql-cancelled", |harness| {
        let tab = sql_script(harness, SCRIPT);
        run_sql(harness, tab, true);
        let column = |name: &str, type_name: &str, kind| ColumnMeta {
            name: name.into(),
            type_name: type_name.into(),
            kind,
        };
        let kinds = tabletist_db::StatementOutcome::Rows {
            columns: vec![
                column("kind", "varchar", ValueKind::Text),
                column("images", "int8", ValueKind::Numeric),
            ],
            rows: vec![
                vec![Value::Text("cover".into()), Value::Int(7)],
                vec![Value::Text("preview".into()), Value::Int(6)],
            ],
            truncated: false,
        };
        let outcome =
            crate::testing::script_outcome(vec![kinds, tabletist_db::StatementOutcome::Cancelled]);
        let timeout = crate::backend::CancelReason::Timeout(Duration::from_secs(30));
        harness.answer_sql(Ok(outcome), Some(timeout));
    });
    // A limit of 100 rows, and a table with more.
    both("sql-truncated", |harness| {
        let tab = sql_script(harness, SCRIPT);
        let sql_tab = harness.app.workspace(tab).unwrap().active_tab.unwrap();
        harness.app.apply(Action::SetSqlLimit {
            tab,
            sql_tab,
            limit: 100,
        });
        run_sql(harness, tab, false);
        let page = page_of(100);
        let cut = tabletist_db::StatementOutcome::Rows {
            columns: page.columns,
            rows: page.rows,
            truncated: true,
        };
        harness.answer_sql(Ok(crate::testing::script_outcome(vec![cut])), None);
    });
}

/// Runs the active SQL editor's statement, or `all` of its script.
fn run_sql(harness: &mut Harness, tab: ConnTabId, all: bool) {
    let sql_tab = harness.app.workspace(tab).unwrap().active_tab.unwrap();
    harness.app.apply(Action::RunSql { tab, sql_tab, all });
}

/// The script the SQL scenes show: two statements over `book_images`.
const SCRIPT: &str = "-- Images of each kind\n\
                      SELECT kind, count(*) AS images\n  \
                      FROM book_images\n \
                      GROUP BY kind;\n\n\
                      SELECT * FROM book_images";

/// A SQL editor beside the open tables, holding `script`.
fn sql_script(harness: &mut Harness, script: &str) -> ConnTabId {
    let tab = workspace(harness);
    harness.app.apply(Action::NewSqlTab(tab));
    let workspace = harness.app.workspace_mut(tab).unwrap();
    workspace.server_version.value = Some("PostgreSQL 17.2".into());
    let id = workspace.active_tab.unwrap();
    let sql = workspace.sql_tab_mut(id).unwrap();
    sql.text = script.into();
    sql.cursor = sql.text.len();
    tab
}

/// A SQL editor beside the open tables, its last statement run and
/// answered with the table's rows.
fn sql_editor(harness: &mut Harness) -> ConnTabId {
    let tab = sql_script(harness, SCRIPT);
    let id = harness.app.workspace(tab).unwrap().active_tab.unwrap();
    harness.app.apply(Action::RunSql {
        tab,
        sql_tab: id,
        all: false,
    });
    let page = page();
    harness.answer_sql(
        Ok(tabletist_db::ScriptOutcome {
            results: vec![tabletist_db::StatementResult {
                elapsed: Duration::from_millis(14),
                outcome: tabletist_db::StatementOutcome::Rows {
                    columns: page.columns,
                    rows: page.rows,
                    truncated: false,
                },
            }],
            stopped: false,
        }),
        None,
    );
    tab
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

/// The design screens at the mockups' scales, so each PNG lines up pixel
/// for pixel with its mockup: `target/shots/mock-<screen>.png`.
mod mock {
    use super::*;
    use crate::theme::{Look, Palette};

    /// The moment the scenes happen: 2026-09-29 12:00 UTC, so "last used"
    /// reads as the mockups do (2 min ago, yesterday, Sep 12, Aug 30).
    pub const NOW: u64 = 1_790_683_200;

    /// A mockup's screen, in the Bookshop scene.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum Screen {
        MacWorkspace,
        OmarchyWorkspace,
        MacPicker,
        OmarchyPicker,
        MacDialog,
        OmarchyDialog,
    }

    impl Screen {
        pub const ALL: [Screen; 6] = [
            Self::MacWorkspace,
            Self::OmarchyWorkspace,
            Self::MacPicker,
            Self::OmarchyPicker,
            Self::MacDialog,
            Self::OmarchyDialog,
        ];

        /// The screen's name (screenshot files).
        pub fn name(self) -> &'static str {
            match self {
                Self::MacWorkspace => "macos-workspace",
                Self::OmarchyWorkspace => "omarchy-workspace",
                Self::MacPicker => "macos-connections",
                Self::OmarchyPicker => "omarchy-connections",
                Self::MacDialog => "macos-connection-edit",
                Self::OmarchyDialog => "omarchy-connection-edit",
            }
        }

        pub fn look(self) -> Look {
            match self {
                Self::MacWorkspace | Self::MacPicker | Self::MacDialog => Look::macos(),
                Self::OmarchyWorkspace | Self::OmarchyPicker | Self::OmarchyDialog => {
                    Look::omarchy()
                }
            }
        }

        /// The window in points. The Omarchy artboards hold the window inside
        /// a 10 pt wallpaper margin and Hyprland's 2 pt border.
        pub fn size(self) -> egui::Vec2 {
            match self {
                Self::MacWorkspace | Self::MacPicker | Self::MacDialog => egui::vec2(1440.0, 900.0),
                Self::OmarchyWorkspace => egui::vec2(1896.0, 1056.0),
                Self::OmarchyPicker | Self::OmarchyDialog => egui::vec2(936.0, 1016.0),
            }
        }

        /// The mockups' export scale (pixels per point), for screenshots that
        /// line up with them.
        pub fn design_scale(self) -> f32 {
            match self {
                Self::MacWorkspace | Self::MacPicker | Self::MacDialog => 2000.0 / 1440.0,
                Self::OmarchyWorkspace => 2000.0 / 1920.0,
                Self::OmarchyPicker | Self::OmarchyDialog => 1846.0 / 960.0,
            }
        }

        /// The palette the screen is drawn in: macOS light, Omarchy in Tokyo
        /// Night, as the mockups.
        pub fn palette(self) -> Palette {
            match self {
                Self::MacWorkspace | Self::MacPicker | Self::MacDialog => Palette::light(),
                Self::OmarchyWorkspace | Self::OmarchyPicker | Self::OmarchyDialog => tokyo_night(),
            }
        }

        /// Puts the scene on `harness`.
        pub fn stage(self, harness: &mut Harness) {
            crate::util::pin_now(Some(NOW));
            match self {
                Self::MacWorkspace => {
                    workspace(harness);
                }
                Self::OmarchyWorkspace => {
                    let tab = workspace(harness);
                    let object_tab = harness.app.workspace(tab).unwrap().active_tab.unwrap();
                    let object = harness
                        .app
                        .workspace_mut(tab)
                        .unwrap()
                        .object_tab_mut(object_tab)
                        .unwrap();
                    object.filter.raw = true;
                    object.filter.raw_text = "kind = 'front'".into();
                    harness.app.apply(Action::ApplyFilters { tab, object_tab });
                    harness.answer_rows(covers());
                    harness.app.apply(Action::SelectCell {
                        tab,
                        id: object_tab,
                        cell: CellPos { row: 0, col: 0 },
                    });
                }
                Self::MacPicker | Self::OmarchyPicker => pickers(harness),
                Self::MacDialog | Self::OmarchyDialog => {
                    pickers(harness);
                    edit_production(harness);
                }
            }
        }
    }

    /// Tokyo Night through the Omarchy template, as the mockups draw it.
    pub fn tokyo_night() -> Palette {
        let colors = "mode\tdark\nbackground\t#1a1b26\ndark_background\t#16161e\n\
                      lighter_background\t#292e42\nforeground\t#c0caf5\nmuted\t#565f89\n\
                      accent\t#7aa2f7\nselection\t#283457\nred\t#f7768e\ngreen\t#9ece6a\n\
                      yellow\t#e0af68\ncyan\t#7dcfff\norange\t#ff9e64\nmagenta\t#bb9af7\n\
                      blue\t#7aa2f7\nbright_red\t#f7768e\nbright_green\t#9ece6a\n";
        let rendered = fastframe_theme::omarchy::render_seed::<crate::theme::Palette>(
            include_str!("../contrib/omarchy/tabletist.json.tpl"),
            colors,
        )
        .unwrap();
        fastframe_theme::parse_palette::<crate::theme::Palette>(&rendered)
            .unwrap()
            .with_readable_labels()
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
            allowed_values: None,
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
        workspace.environment = saved.environment;
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
        let object_tab = harness.app.workspace(tab).unwrap().active_tab.unwrap();
        harness.app.apply(Action::SortBy {
            tab,
            object_tab,
            column: "created_at".into(),
        });
        harness.answer_rows(covers());
        // The tabs in the order the mockups show them.
        let workspace = harness.app.workspace_mut(tab).unwrap();
        workspace.tabs.reverse();
        harness.app.apply(Action::SelectCell {
            tab,
            id: object_tab,
            cell: CellPos { row: 0, col: 0 },
        });
        tab
    }

    /// Five saved connections, as the picker mockups list them.
    fn pickers(harness: &mut Harness) {
        let entries = [
            (
                "Bookshop",
                Environment::Dev,
                "postgres://dev@localhost:5433/bookshop_development",
            ),
            (
                "Bookshop",
                Environment::Staging,
                "postgres://app_readonly@db.staging.example.com:5432/bookshop_staging?sslmode=verify-full",
            ),
            (
                "Bookshop",
                Environment::Production,
                "postgres://app_readonly@db.example.com:5432/bookshop_production?sslmode=verify-full",
            ),
            (
                "Playground",
                Environment::Local,
                "postgres://postgres@localhost:5432/playground",
            ),
            (
                "Rails test",
                Environment::None,
                "postgres://dev@localhost:5433/bookshop_test",
            ),
        ];
        // 2 min ago, yesterday, Sep 12 and Aug 30 (10:00 UTC), never.
        let used = [
            Some(NOW - 120),
            Some(NOW - 86_400),
            Some(1_789_207_200),
            Some(1_788_084_000),
            None,
        ];
        for ((name, environment, url), used) in entries.into_iter().zip(used) {
            let (mut spec, _) = ConnectSpec::from_url(url).unwrap();
            if name == "Bookshop" && environment != Environment::Dev {
                spec.ssh = Some(tabletist_db::SshSpec {
                    host: "bastion".into(),
                    port: Some(22),
                    user: "deploy".into(),
                    auth: tabletist_db::SshAuth::Agent,
                });
            }
            let id = ConnectionId::new();
            harness.app.connections.upsert(SavedConnection {
                id: id.clone(),
                name: name.into(),
                environment,
                read_only: None,
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

    #[test]
    #[ignore = "renders with wgpu; run with --features shots -- --ignored"]
    fn mock() {
        for screen in Screen::ALL {
            let look = screen.look();
            let palette = screen.palette();
            let mut harness = Harness::for_shots(screen.size(), screen.design_scale(), true, look);
            harness.app.palette = palette;
            crate::theme::apply(&harness.ctx, &palette, &look);
            screen.stage(&mut harness);
            harness.shot(&out(&format!("mock-{}.png", screen.name())));
        }
    }
}

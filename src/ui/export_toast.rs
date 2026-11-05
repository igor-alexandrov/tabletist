//! An export's states in the macOS look (and Windows'): the progress toast
//! at the bottom centre while it runs, hidden to a pill at the bottom
//! right; the done and cancelled toasts for 4 s; the error toast until it
//! is dismissed. The terminal look says each in its status line
//! (`workspace::export_line`). Several stack, a toast or a pill each
//! (spec 4, S13). The states are the app's (`App::exports`): this draws
//! them and asks.

use std::time::Instant;

use egui::{Align2, Area, Frame, Id, Margin, Order, Rect, Sense, vec2};

use crate::export_sheet::{ExportStatus, Exporting, of_total, size_words, took_words};
use crate::i18n::gettext;
use crate::model::Action;
use crate::typography::{Text, TextRole};
use crate::ui::widgets::{self, ButtonSpec};
use crate::ui::{Paint, toast};

/// How far above the window's bottom the toast floats: over the one-line
/// toast while that says something.
fn lift(ctx: &egui::Context) -> f32 {
    match toast::said(ctx) {
        Some(_) => 64.0,
        None => 16.0,
    }
}

/// Show in Finder on macOS, Show in folder elsewhere (spec, D9).
pub fn reveal_words(locale: crate::i18n::Locale) -> String {
    let words = if cfg!(target_os = "macos") {
        "Show in Finder"
    } else {
        "Show in folder"
    };
    gettext(locale, words).into_owned()
}

/// "Exporting reviews.csv", or "Copying to the clipboard".
pub fn running_title(exporting: &Exporting, locale: crate::i18n::Locale) -> String {
    match exporting.file.is_empty() {
        true => gettext(locale, "Copying {table}").replace("{table}", &exporting.table),
        false => gettext(locale, "Exporting {file}").replace("{file}", &exporting.file),
    }
}

/// "Exported 31,400 rows to reviews.csv · 7.8 MB · 18 s".
pub fn done_words(exporting: &Exporting, locale: crate::i18n::Locale) -> Option<String> {
    let ExportStatus::Done {
        rows, bytes, took, ..
    } = exporting.status
    else {
        return None;
    };
    Some(
        crate::i18n::ngettext(
            locale,
            "Exported {n} row to {file} · {size} · {time}",
            "Exported {n} rows to {file} · {size} · {time}",
            u32::try_from(rows).unwrap_or(u32::MAX),
        )
        .replace("{n}", &crate::format::group_digits(rows))
        .replace("{file}", &exporting.file)
        .replace("{size}", &size_words(bytes, locale))
        .replace("{time}", &took_words(took)),
    )
}

pub const CANCELLED: &str = "Export cancelled. Nothing was saved: the partial file was removed.";

/// What a screen reader is told of a running export, at most every 2 s:
/// "Exporting reviews, 58 percent".
fn spoken(ctx: &egui::Context, exporting: &Exporting, locale: crate::i18n::Locale) -> String {
    let id = Id::new(("export-spoken", exporting.job.id.0));
    let now = ctx.input(|input| input.time);
    let kept: Option<(f64, String)> = ctx.data(|data| data.get_temp(id));
    if let Some((when, text)) = &kept
        && now - when < 2.0
    {
        return text.clone();
    }
    let text = match exporting.share() {
        Some(share) => gettext(locale, "Exporting {table}, {n} percent")
            .replace("{table}", &exporting.table)
            .replace("{n}", &((share * 100.0).round() as u32).to_string()),
        None => gettext(locale, "Exporting {table}").replace("{table}", &exporting.table),
    };
    ctx.data_mut(|data| data.insert_temp(id, (now, text.clone())));
    text
}

/// Draws the exports' states, and returns what their buttons ask: a
/// toast each, stacked at the bottom centre with the newest at the bottom,
/// and a pill each for those hidden, stacked up from the bottom right
/// (spec 4, S13 and F8). ⌘⇧' goes to the newest's first button.
pub fn show(
    ctx: &egui::Context,
    paint: Paint,
    exports: &[Exporting],
    home: Option<&std::path::Path>,
) -> Vec<Action> {
    let mut actions = Vec::new();
    if exports.is_empty() || paint.look.terminal {
        return actions;
    }
    let now = Instant::now();
    // While one runs, the time left and the bar move: a frame now and then.
    for exporting in exports {
        match exporting.status {
            ExportStatus::Running { .. } | ExportStatus::Asking => {
                ctx.request_repaint_after(std::time::Duration::from_millis(500));
            }
            ExportStatus::Done { at, .. } | ExportStatus::Cancelled { at } => {
                let left = crate::export_sheet::TOLD_FOR.saturating_sub(now.duration_since(at));
                ctx.request_repaint_after(left);
            }
            ExportStatus::Failed { .. } => {}
        }
    }
    let newest = exports.last().map(|exporting| exporting.job.id);
    let pilled = |exporting: &&Exporting| exporting.hidden && exporting.running();
    let pills: Vec<&Exporting> = exports.iter().filter(pilled).collect();
    let toasts: Vec<&Exporting> = exports
        .iter()
        .filter(|exporting| !pilled(exporting))
        .collect();
    if !pills.is_empty() {
        Area::new(Id::new("export-pill"))
            .order(Order::Foreground)
            .anchor(Align2::RIGHT_BOTTOM, vec2(-16.0, -16.0))
            .show(ctx, |ui| {
                ui.spacing_mut().item_spacing = vec2(0.0, GAP);
                ui.with_layout(egui::Layout::top_down(egui::Align::Max), |ui| {
                    for exporting in pills {
                        let first = Some(exporting.job.id) == newest;
                        pill(ui, paint, exporting, first, &mut actions);
                    }
                });
            });
    }
    if !toasts.is_empty() {
        Area::new(Id::new("export-toast"))
            .order(Order::Foreground)
            .anchor(Align2::CENTER_BOTTOM, vec2(0.0, -lift(ctx)))
            .show(ctx, |ui| {
                ui.spacing_mut().item_spacing = vec2(0.0, GAP);
                ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                    for exporting in toasts {
                        let first = Some(exporting.job.id) == newest;
                        // Its own ids, whatever goes above it.
                        ui.push_id(("export-toast", exporting.job.id.0), |ui| {
                            one_toast(ui, paint, exporting, (first, now), home, &mut actions);
                        });
                    }
                });
            });
    }
    actions
}

/// How far apart the toasts are, and the pills.
const GAP: f32 = 8.0;

/// The pill of a hidden export, the bottom right's: a click shows its
/// toast again.
fn pill(
    ui: &mut egui::Ui,
    paint: Paint,
    exporting: &Exporting,
    first: bool,
    actions: &mut Vec<Action>,
) {
    let Paint {
        locale,
        palette,
        look,
    } = paint;
    let small = widgets::secondary(&look);
    let share = exporting
        .share()
        .map(|share| format!(" {}%", (share * 100.0).round() as u32))
        .unwrap_or_default();
    let text = format!("{}{share}", exporting.name());
    let drawn = Frame::new()
        .fill(palette.text)
        .corner_radius(14)
        .inner_margin(Margin::symmetric(12, 6))
        .show(ui, |ui| {
            widgets::label(ui, small, &text, palette.window, &look);
        })
        .response;
    // A widget of its own over the frame, which takes the keyboard as a
    // button does.
    let response = ui.interact(
        drawn.rect,
        Id::new(("export-pill-button", exporting.job.id.0)),
        Sense::click(),
    );
    let spoken = spoken(ui.ctx(), exporting, locale);
    response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, &spoken));
    toast::button(ui.ctx(), &response, first);
    if response.clicked() {
        actions.push(Action::HideExport {
            job: exporting.job.id,
            hidden: false,
        });
    }
}

/// One export's toast; `first` where its first button is the one ⌘⇧'
/// goes to.
fn one_toast(
    ui: &mut egui::Ui,
    paint: Paint,
    exporting: &Exporting,
    (first, now): (bool, Instant),
    home: Option<&std::path::Path>,
    actions: &mut Vec<Action>,
) {
    let Paint {
        locale,
        palette,
        look,
    } = paint;
    let job = exporting.job.id;
    let body = widgets::body(&look);
    let small = widgets::secondary(&look);
    let failed = matches!(exporting.status, ExportStatus::Failed { .. });
    let (fill, ink, dim) = match failed {
        // The error banner's danger-soft, with the text's colour.
        true => (
            palette.danger.gamma_multiply(0.14).blend(palette.window),
            palette.text,
            palette.secondary,
        ),
        false => (
            palette.text,
            palette.window,
            palette.window.gamma_multiply(0.7),
        ),
    };
    {
        {
            let frame = Frame::new()
                .fill(fill)
                .corner_radius(9)
                .inner_margin(Margin::symmetric(14, 10))
                .shadow(egui::Shadow {
                    offset: [0, 8],
                    blur: 24,
                    spread: 0,
                    color: palette.shadow,
                });
            let frame = match failed {
                true => frame.stroke(egui::Stroke::new(1.0, palette.danger.gamma_multiply(0.5))),
                false => frame,
            };
            frame.show(ui, |ui| match &exporting.status {
                ExportStatus::Asking | ExportStatus::Running { .. } => {
                    let rows = match exporting.status {
                        ExportStatus::Running { rows, .. } => rows,
                        _ => 0,
                    };
                    ui.set_width(420.0);
                    ui.horizontal(|ui| {
                        let title = running_title(exporting, locale);
                        widgets::label(ui, TextRole::UiBodyStrong, &title, ink, &look);
                        widgets::label(
                            ui,
                            body,
                            &of_total(rows, exporting.total, locale),
                            dim,
                            &look,
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            // Right to left: Cancel is drawn after ×, and
                            // is the first the keyboard comes to.
                            let hide = gettext(locale, "Hide");
                            let hidden = widgets::icon_button_tipped(
                                ui,
                                crate::style::Icon::X,
                                (&hide, &hide),
                                &look,
                                &palette,
                            );
                            toast::button(ui.ctx(), &hidden, false);
                            if hidden.clicked() {
                                actions.push(Action::HideExport { job, hidden: true });
                            }
                            let cancel = gettext(locale, "Cancel");
                            let cancelled = ButtonSpec::new(&cancel)
                                .link()
                                .show(ui, 22.0, &look, &palette);
                            toast::button(ui.ctx(), &cancelled, first);
                            if cancelled.clicked() {
                                actions.push(Action::CancelExport(Some(job)));
                            }
                        });
                    });
                    ui.add_space(6.0);
                    let (bar, _) =
                        ui.allocate_exact_size(vec2(ui.available_width(), 4.0), Sense::hover());
                    ui.painter().rect_filled(bar, 2, ink.gamma_multiply(0.25));
                    if let Some(share) = exporting.share() {
                        let done =
                            Rect::from_min_size(bar.min, vec2(bar.width() * share, bar.height()));
                        ui.painter().rect_filled(done, 2, palette.accent);
                    }
                    // A status region, told at most every 2 s.
                    let spoken = spoken(ui.ctx(), exporting, locale);
                    let response = ui.interact(
                        bar,
                        Id::new(("export-progress", exporting.job.id.0)),
                        Sense::hover(),
                    );
                    response.widget_info(|| {
                        egui::WidgetInfo::labeled(
                            egui::WidgetType::ProgressIndicator,
                            true,
                            &spoken,
                        )
                    });
                    ui.ctx().accesskit_node_builder(response.id, |node| {
                        node.set_live(egui::accesskit::Live::Polite);
                    });
                    // The time left where the total is known, else the
                    // connection, as the board's second toast says it.
                    let under = match exporting.total {
                        Some(_) => exporting.time_left(now, locale),
                        None => Some(exporting.connection.clone()),
                    };
                    if let Some(under) = under {
                        ui.add_space(4.0);
                        widgets::label(ui, small, &under, dim, &look);
                    }
                }
                ExportStatus::Done { .. } => {
                    ui.horizontal(|ui| {
                        let words = done_words(exporting, locale).unwrap_or_default();
                        let said = widgets::label(ui, body, &words, ink, &look);
                        ui.ctx().accesskit_node_builder(said.id, |node| {
                            node.set_live(egui::accesskit::Live::Polite);
                        });
                        let reveal = reveal_words(locale);
                        let shown = ButtonSpec::new(&reveal)
                            .link()
                            .show(ui, 22.0, &look, &palette);
                        toast::button(ui.ctx(), &shown, first);
                        if shown.clicked() {
                            actions.push(Action::RevealExport(Some(job)));
                        }
                    });
                }
                ExportStatus::Cancelled { .. } => {
                    let said = widgets::label(ui, body, &gettext(locale, CANCELLED), ink, &look);
                    ui.ctx().accesskit_node_builder(said.id, |node| {
                        node.set_live(egui::accesskit::Live::Polite);
                    });
                }
                ExportStatus::Failed {
                    failure,
                    rows,
                    bytes,
                    ..
                } => {
                    let (words, action) = crate::export_sheet::failure_words(
                        exporting,
                        failure,
                        (*rows, *bytes),
                        home,
                        locale,
                    );
                    ui.horizontal(|ui| {
                        ui.set_max_width(560.0);
                        let said = Text::one(&look, body, &words, ink)
                            .wrap(440.0)
                            .layout(ui.ctx())
                            .label(ui);
                        // An alert: read out as it appears.
                        ui.ctx().accesskit_node_builder(said.id, |node| {
                            node.set_live(egui::accesskit::Live::Assertive);
                        });
                        if let Some(action) = &action {
                            let acted = ButtonSpec::new(action).show(ui, 24.0, &look, &palette);
                            toast::button(ui.ctx(), &acted, first);
                            if acted.clicked() {
                                actions.push(Action::ExportFailureAction(Some(job)));
                            }
                        }
                        let dismiss = gettext(locale, "Dismiss");
                        let dismissed = widgets::icon_button_tipped(
                            ui,
                            crate::style::Icon::X,
                            (&dismiss, &dismiss),
                            &look,
                            &palette,
                        );
                        toast::button(ui.ctx(), &dismissed, first && action.is_none());
                        if dismissed.clicked() {
                            actions.push(Action::DismissExport(Some(job)));
                        }
                    });
                }
            });
        }
    }
}

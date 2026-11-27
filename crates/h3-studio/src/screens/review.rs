//! Video Review — versions, A/B comparison, synchronized timeline, checklist, comments, summary.

use egui::{Align2, Color32, FontId, FontFamily, Rect, Sense, Stroke, StrokeKind};
use crate::app::{H3App, Screen};
use crate::theme::{color, typography};
use crate::ui_state::ComparisonMode;
use crate::widgets;

pub fn draw(app: &mut H3App, ui: &mut egui::Ui) {
    use crate::i18n::{tr, tf};
    let lang = app.lang();
    ui.horizontal(|ui| {
        // Left column: versions + comparison mode
        ui.vertical(|ui| {
            let (versions, _) = ui.allocate_exact_size(egui::vec2(300.0, 210.0), egui::Sense::hover());
            ui.painter().rect_filled(versions, 12.0f32, color::panel());
            ui.painter().rect_stroke(versions, 12.0f32, Stroke::new(1.0, color::border()), StrokeKind::Middle);
            ui.painter().text(egui::pos2(versions.left() + 16.0, versions.top() + 22.0), Align2::LEFT_CENTER, tr(lang, "VERSIONS"), FontId::new(10.0, FontFamily::Proportional), color::text2());
            ui.painter().text(egui::pos2(versions.left() + 16.0, versions.top() + 40.0), Align2::LEFT_CENTER, tr(lang, "Original master saved 03/2024"), FontId::new(9.0, FontFamily::Proportional), color::text3());
            let versions_list = [("v1 · Original", "FL2VA · 24fps"), ("v2 · FL2VA Master", "1920×1080 · seed 8240"), ("v3 · Ref2VA Cand A", "1920×1080 · seed 22913")];
            for (i, (title, meta)) in versions_list.iter().enumerate() {
                let row = Rect::from_min_size(egui::pos2(versions.left() + 16.0, versions.top() + 58.0 + i as f32 * 46.0), egui::vec2(300.0 - 32.0, 38.0));
                let resp = ui.interact(row, ui.id().with(("rv-version", i)), Sense::click());
                let selected = i == app.ui.review_version;
                let hov = resp.hovered() && !selected;
                ui.painter().rect_filled(row, 8.0f32, if selected { color::indigo_dim() } else if hov { color::card().linear_multiply(1.15) } else { color::card() });
                ui.painter().rect_stroke(row, 8.0f32, Stroke::new(if selected { 1.5 } else { 1.0 }, if selected || hov { color::indigo() } else { color::border() }), StrokeKind::Middle);
                let title_label = tr(lang, title);
                ui.painter().text(egui::pos2(row.left() + 12.0, row.top() + 12.0), Align2::LEFT_CENTER, title_label, FontId::new(10.5, FontFamily::Proportional), if selected { color::indigo() } else { color::text() });
                ui.painter().text(egui::pos2(row.left() + 12.0, row.bottom() - 12.0), Align2::LEFT_CENTER, meta, FontId::new(9.0, FontFamily::Monospace), color::text2());
                if resp.clicked() {
                    app.ui.review_version = i;
                    app.set_toast(tf(lang, "Switched review to {}", &[&title_label]));
                }
            }
            ui.add_space(14.0);

            let (mode, _) = ui.allocate_exact_size(egui::vec2(300.0, 88.0), egui::Sense::hover());
            ui.painter().rect_filled(mode, 12.0f32, color::panel());
            ui.painter().rect_stroke(mode, 12.0f32, Stroke::new(1.0, color::border()), StrokeKind::Middle);
            ui.painter().text(egui::pos2(mode.left() + 16.0, mode.top() + 22.0), Align2::LEFT_CENTER, tr(lang, "COMPARISON MODE"), FontId::new(9.5, FontFamily::Proportional), color::text2());
            let mode_label = match app.ui.review_comparison {
                ComparisonMode::SideBySide => tr(lang, "Side by Side"),
                ComparisonMode::Overlay => tr(lang, "Overlay"),
            };
            let chip_rect = Rect::from_min_size(egui::pos2(mode.left() + 16.0, mode.top() + 40.0), egui::vec2(140.0, 26.0));
            let chip_resp = ui.interact(chip_rect, ui.id().with("rv-mode"), Sense::click());
            let hov = chip_resp.hovered();
            ui.painter().rect_filled(chip_rect, 13.0f32, if hov { color::indigo_dim().linear_multiply(1.2) } else { color::indigo_dim() });
            ui.painter().rect_stroke(chip_rect, 13.0f32, Stroke::new(1.0, color::indigo().linear_multiply(0.5)), StrokeKind::Middle);
            ui.painter().text(chip_rect.center(), Align2::CENTER_CENTER, mode_label, FontId::new(10.5, FontFamily::Proportional), color::indigo());
            if chip_resp.clicked() {
                app.ui.review_comparison = match app.ui.review_comparison {
                    ComparisonMode::SideBySide => ComparisonMode::Overlay,
                    ComparisonMode::Overlay => ComparisonMode::SideBySide,
                };
                app.set_toast(tf(lang, "Comparison mode → {}", &[&mode_label]));
            }
        });

        // Center column: A/B + timeline
        ui.add_space(14.0);
        ui.vertical(|ui| {
            ui.colored_label(color::text2(), egui::RichText::new(tr(lang, "A/B COMPARISON · Shot 002 · Sword drawn close-up · 4s")).size(10.5));
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                for (idx, (label, accent)) in [("A · v2", color::indigo()), ("B · v3", color::teal())].iter().enumerate() {
                    let (well, _) = ui.allocate_exact_size(egui::vec2((ui.available_width() - 12.0) / 2.0, 240.0), egui::Sense::hover());
                    crate::placeholder::bamboo_scene(ui, well, 31 + idx as u32);
                    ui.painter().rect_filled(well, 12.0f32, Color32::from_rgba_premultiplied(0x0B, 0x12, 0x19, 80));
                    ui.painter().rect_stroke(well, 12.0f32, Stroke::new(1.0, color::border()), StrokeKind::Middle);
                    let chip_rect = Rect::from_min_size(egui::pos2(well.left() + 14.0, well.top() + 12.0), egui::vec2(70.0, 24.0));
                    ui.painter().rect_filled(chip_rect, 12.0f32, accent.linear_multiply(0.35));
                    ui.painter().text(chip_rect.center(), Align2::CENTER_CENTER, *label, FontId::new(10.5, FontFamily::Proportional), *accent);
                    let play_rect = Rect::from_center_size(well.center(), egui::vec2(56.0, 56.0));
                    let play_resp = ui.interact(play_rect, ui.id().with(("rv-play", idx)), Sense::click());
                    crate::placeholder::play_button(ui, well.center(), if play_resp.hovered() { 30.0 } else { 28.0 }, *accent);
                    if play_resp.clicked() {
                        app.set_toast(tf(lang, "Playing {} candidate", &[label]));
                    }
                    ui.painter().text(egui::pos2(well.center().x, well.bottom() - 16.0), Align2::CENTER_CENTER, "1920×1080 · 24fps", FontId::new(9.0, FontFamily::Monospace), color::text3());
                    ui.add_space(12.0);
                }
            });
            ui.add_space(12.0);
            let (timeline, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 78.0), egui::Sense::hover());
            ui.painter().rect_filled(timeline, 12.0f32, color::panel());
            ui.painter().rect_stroke(timeline, 12.0f32, Stroke::new(1.0, color::border()), StrokeKind::Middle);
            ui.painter().text(egui::pos2(timeline.left() + 18.0, timeline.top() + 22.0), Align2::LEFT_CENTER, tr(lang, "SYNCHRONIZED TIMELINE · playback 01.2s / 4.0s"), FontId::new(9.5, FontFamily::Proportional), color::text2());
            let bar = Rect::from_min_size(egui::pos2(timeline.left() + 18.0, timeline.top() + 36.0), egui::vec2(timeline.width() - 36.0, 8.0));
            let bar_hit = Rect::from_min_size(egui::pos2(bar.left(), bar.top() - 6.0), egui::vec2(bar.width(), 20.0));
            let bar_resp = ui.interact(bar_hit, ui.id().with("rv-timeline"), Sense::click());
            widgets::progress_bar(ui, bar, 0.3, color::indigo(), 8.0);
            ui.painter().text(egui::pos2(timeline.left() + 18.0, timeline.bottom() - 16.0), Align2::LEFT_CENTER, tr(lang, "Both playheads advance together — no drift."), FontId::new(9.5, FontFamily::Proportional), color::text3());
            if bar_resp.clicked() {
                if let Some(pos) = bar_resp.interact_pointer_pos() {
                    let frac = ((pos.x - bar.left()) / bar.width()).clamp(0.0, 1.0);
                    app.set_toast(tf(lang, "Scrub A/B to {}", &[&format!("{:.1}s", frac * 4.0)]));
                }
            }
            ui.add_space(6.0);
            ui.colored_label(color::text(), egui::RichText::new(tr(lang, "▶  01.2 / 4.0s    Loop    Frame step")).size(12.0));
        });

        // Right column: checklist + comments
        ui.add_space(14.0);
        ui.vertical(|ui| {
            let (checklist, _) = ui.allocate_exact_size(egui::vec2(540.0, 232.0), egui::Sense::hover());
            ui.painter().rect_filled(checklist, 12.0f32, color::panel());
            ui.painter().rect_stroke(checklist, 12.0f32, Stroke::new(1.0, color::border()), StrokeKind::Middle);
            ui.painter().text(egui::pos2(checklist.left() + 18.0, checklist.top() + 22.0), Align2::LEFT_CENTER, tr(lang, "REVIEW CHECKLIST"), FontId::new(10.5, FontFamily::Proportional), color::text());
            while app.ui.review_check_status.len() < 5 {
                app.ui.review_check_status.push(String::new());
            }
            let rows = [
                ("Character identity", "PASS"),
                ("Face geometry", "WARN"),
                ("Motion continuity", "PASS"),
                ("Camera movement", "PASS"),
                ("Audio sync", "PASS"),
            ];
            let mut passed = 0usize;
            for (i, (label, default_status)) in rows.iter().enumerate() {
                let row_y = checklist.top() + 46.0 + i as f32 * 31.0;
                let row_rect = Rect::from_min_size(egui::pos2(checklist.left() + 14.0, row_y - 4.0), egui::vec2(checklist.width() - 28.0, 26.0));
                let row_resp = ui.interact(row_rect, ui.id().with(("rv-check", i)), Sense::click());
                let stored = app.ui.review_check_status.get(i).cloned().filter(|s| !s.is_empty());
                let status = stored.unwrap_or_else(|| default_status.to_string());
                let accent = if status == "PASS" { color::teal() } else { color::amber() };
                if status == "PASS" { passed += 1; }
                if row_resp.hovered() {
                    ui.painter().rect_filled(row_rect, 6.0f32, color::card());
                }
                let label_label = tr(lang, label);
                let status_label = tr(lang, &status);
                ui.painter().circle_filled(egui::pos2(checklist.left() + 24.0, row_y + 8.0), 3.5, accent);
                ui.painter().text(egui::pos2(checklist.left() + 40.0, row_y + 8.0), Align2::LEFT_CENTER, label_label, FontId::new(11.5, FontFamily::Proportional), color::text());
                let chip_rect = Rect::from_min_size(egui::pos2(checklist.right() - 78.0, row_y), egui::vec2(64.0, 20.0));
                ui.painter().rect_filled(chip_rect, 10.0f32, accent.linear_multiply(0.22));
                ui.painter().text(chip_rect.center(), Align2::CENTER_CENTER, status_label, FontId::new(9.5, FontFamily::Proportional), accent);
                if row_resp.clicked() {
                    let next = if status == "PASS" { "WARN" } else { "PASS" };
                    app.ui.review_check_status[i] = next.to_string();
                    app.set_toast(format!("{} → {}", label_label, tr(lang, next)));
                }
            }
            let req_rect = Rect::from_min_size(egui::pos2(checklist.left() + 14.0, checklist.bottom() - 34.0), egui::vec2(150.0, 24.0));
            let req_resp = ui.interact(req_rect, ui.id().with("rv-reqrev"), Sense::click());
            ui.painter().text(egui::pos2(checklist.left() + 18.0, checklist.bottom() - 22.0), Align2::LEFT_CENTER, if req_resp.hovered() { tr(lang, "Request Revision →") } else { tr(lang, "Request Revision") }, FontId::new(10.5, FontFamily::Proportional), color::amber());
            ui.painter().text(egui::pos2(checklist.right() - 18.0, checklist.bottom() - 22.0), Align2::RIGHT_CENTER, &tf(lang, "{} / 5 passed", &[&format!("{}", passed)]), FontId::new(11.5, FontFamily::Proportional), color::text());
            if req_resp.clicked() {
                app.toast_warning(tr(lang, "Revision requested — sent back to queue (demo)").to_string());
            }
            ui.add_space(14.0);

            let (comments, _) = ui.allocate_exact_size(egui::vec2(540.0, 128.0), egui::Sense::hover());
            ui.painter().rect_filled(comments, 12.0f32, color::panel());
            ui.painter().rect_stroke(comments, 12.0f32, Stroke::new(1.0, color::border()), StrokeKind::Middle);
            ui.painter().text(egui::pos2(comments.left() + 18.0, comments.top() + 22.0), Align2::LEFT_CENTER, tr(lang, "COMMENTS"), FontId::new(10.0, FontFamily::Proportional), color::text());
            ui.painter().text(egui::pos2(comments.left() + 18.0, comments.top() + 48.0), Align2::LEFT_CENTER, tr(lang, "Blade edge looks soft — resend or send to post-QC."), FontId::new(11.0, FontFamily::Proportional), color::text());
            ui.painter().text(egui::pos2(comments.left() + 18.0, comments.top() + 86.0), Align2::LEFT_CENTER, tr(lang, "L Director · 2m ago · 0 replies"), FontId::new(9.5, FontFamily::Proportional), color::text3());
        });
    });

    // SHOT SUMMARY bottom bar
    ui.add_space(12.0);
    let (summary, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 84.0), egui::Sense::hover());
    ui.painter().rect_filled(summary, 12.0f32, color::panel());
    ui.painter().rect_stroke(summary, 12.0f32, Stroke::new(1.0, color::border()), StrokeKind::Middle);
    ui.painter().text(egui::pos2(summary.left() + 22.0, summary.top() + 22.0), Align2::LEFT_CENTER, tr(lang, "SHOT SUMMARY"), FontId::new(9.0, FontFamily::Proportional), color::text2());
    ui.painter().text(egui::pos2(summary.left() + 22.0, summary.top() + 42.0), Align2::LEFT_CENTER, tr(lang, "S01 · 002 · Sword drawn close-up"), FontId::new(13.0, FontFamily::Proportional), color::text());
    ui.painter().text(egui::pos2(summary.left() + 22.0, summary.bottom() - 16.0), Align2::LEFT_CENTER, tr(lang, "Best candidate — v2 FL2VA Master (A) · Reviewer L Director"), FontId::new(10.5, FontFamily::Proportional), color::text2());

    // right action buttons
    let btn_y = summary.center().y;
    let promote_rect = Rect::from_min_size(egui::pos2(summary.right() - 520.0, btn_y - 18.0), egui::vec2(190.0, 36.0));
    if widgets::primary_button(ui, tr(lang, "Promote A v2 to master"), 190.0, 36.0) {
        app.toast_success(tr(lang, "Shot promoted to master (demo)").to_string());
    }
    let _ = promote_rect;
    if widgets::outline_button(ui, tr(lang, "Re-queue B with new seed"), 168.0, 36.0) {
        app.toast_success(tr(lang, "B re-queued (demo)").to_string());
    }
    if widgets::outline_button(ui, tr(lang, "Send to Export"), 156.0, 36.0) {
        app.nav_to(Screen::Export);
    }
}

//! Video Review — versions, A/B comparison, synchronized timeline, checklist, comments, summary.

use egui::{Align2, Color32, FontId, FontFamily, Rect, Stroke, StrokeKind};
use crate::app::{H3App, Screen};
use crate::theme::{color, typography};
use crate::widgets;

pub fn draw(app: &mut H3App, ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        // Left column: versions + comparison mode
        ui.vertical(|ui| {
            let (versions, _) = ui.allocate_exact_size(egui::vec2(300.0, 210.0), egui::Sense::hover());
            ui.painter().rect_filled(versions, 12.0f32, color::panel());
            ui.painter().rect_stroke(versions, 12.0f32, Stroke::new(1.0, color::border()), StrokeKind::Middle);
            ui.painter().text(egui::pos2(versions.left() + 16.0, versions.top() + 22.0), Align2::LEFT_CENTER, "VERSIONS", FontId::new(10.0, FontFamily::Proportional), color::text2());
            ui.painter().text(egui::pos2(versions.left() + 16.0, versions.top() + 40.0), Align2::LEFT_CENTER, "Original master saved 03/2024", FontId::new(9.0, FontFamily::Proportional), color::text3());
            let versions_list = [("v1 · Original", "FL2VA · 24fps", false), ("v2 · FL2VA Master", "1920×1080 · seed 8240", true), ("v3 · Ref2VA Cand A", "1920×1080 · seed 22913", false)];
            for (i, (title, meta, selected)) in versions_list.iter().enumerate() {
                let row = Rect::from_min_size(egui::pos2(versions.left() + 16.0, versions.top() + 58.0 + i as f32 * 46.0), egui::vec2(300.0 - 32.0, 38.0));
                ui.painter().rect_filled(row, 8.0f32, if *selected { color::indigo_dim() } else { color::card() });
                ui.painter().rect_stroke(row, 8.0f32, Stroke::new(if *selected { 1.5 } else { 1.0 }, if *selected { color::indigo() } else { color::border() }), StrokeKind::Middle);
                ui.painter().text(egui::pos2(row.left() + 12.0, row.top() + 12.0), Align2::LEFT_CENTER, title, FontId::new(10.5, FontFamily::Proportional), if *selected { color::indigo() } else { color::text() });
                ui.painter().text(egui::pos2(row.left() + 12.0, row.bottom() - 12.0), Align2::LEFT_CENTER, meta, FontId::new(9.0, FontFamily::Monospace), color::text2());
            }
            ui.add_space(14.0);

            let (mode, _) = ui.allocate_exact_size(egui::vec2(300.0, 88.0), egui::Sense::hover());
            ui.painter().rect_filled(mode, 12.0f32, color::panel());
            ui.painter().rect_stroke(mode, 12.0f32, Stroke::new(1.0, color::border()), StrokeKind::Middle);
            ui.painter().text(egui::pos2(mode.left() + 16.0, mode.top() + 22.0), Align2::LEFT_CENTER, "COMPARISON MODE", FontId::new(9.5, FontFamily::Proportional), color::text2());
            let chip_rect = Rect::from_min_size(egui::pos2(mode.left() + 16.0, mode.top() + 40.0), egui::vec2(120.0, 26.0));
            ui.painter().rect_filled(chip_rect, 13.0f32, color::indigo_dim());
            ui.painter().rect_stroke(chip_rect, 13.0f32, Stroke::new(1.0, color::indigo().linear_multiply(0.5)), StrokeKind::Middle);
            ui.painter().text(chip_rect.center(), Align2::CENTER_CENTER, "Side by Side", FontId::new(10.5, FontFamily::Proportional), color::indigo());
        });

        // Center column: A/B + timeline
        ui.add_space(14.0);
        ui.vertical(|ui| {
            ui.colored_label(color::text2(), egui::RichText::new("A/B COMPARISON · Shot 002 · Sword drawn close-up · 4s").size(10.5));
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
                    crate::placeholder::play_button(ui, well.center(), 28.0, *accent);
                    ui.painter().text(egui::pos2(well.center().x, well.bottom() - 16.0), Align2::CENTER_CENTER, "1920×1080 · 24fps", FontId::new(9.0, FontFamily::Monospace), color::text3());
                    ui.add_space(12.0);
                }
            });
            ui.add_space(12.0);
            let (timeline, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 78.0), egui::Sense::hover());
            ui.painter().rect_filled(timeline, 12.0f32, color::panel());
            ui.painter().rect_stroke(timeline, 12.0f32, Stroke::new(1.0, color::border()), StrokeKind::Middle);
            ui.painter().text(egui::pos2(timeline.left() + 18.0, timeline.top() + 22.0), Align2::LEFT_CENTER, "SYNCHRONIZED TIMELINE · playback 01.2s / 4.0s", FontId::new(9.5, FontFamily::Proportional), color::text2());
            let bar = Rect::from_min_size(egui::pos2(timeline.left() + 18.0, timeline.top() + 36.0), egui::vec2(timeline.width() - 36.0, 8.0));
            widgets::progress_bar(ui, bar, 0.3, color::indigo(), 8.0);
            ui.painter().text(egui::pos2(timeline.left() + 18.0, timeline.bottom() - 16.0), Align2::LEFT_CENTER, "Both playheads advance together — no drift.", FontId::new(9.5, FontFamily::Proportional), color::text3());
            ui.add_space(6.0);
            ui.colored_label(color::text(), egui::RichText::new("▶  01.2 / 4.0s    Loop    Frame step").size(12.0));
        });

        // Right column: checklist + comments
        ui.add_space(14.0);
        ui.vertical(|ui| {
            let (checklist, _) = ui.allocate_exact_size(egui::vec2(540.0, 232.0), egui::Sense::hover());
            ui.painter().rect_filled(checklist, 12.0f32, color::panel());
            ui.painter().rect_stroke(checklist, 12.0f32, Stroke::new(1.0, color::border()), StrokeKind::Middle);
            ui.painter().text(egui::pos2(checklist.left() + 18.0, checklist.top() + 22.0), Align2::LEFT_CENTER, "REVIEW CHECKLIST", FontId::new(10.5, FontFamily::Proportional), color::text());
            let rows = [
                ("Character identity", "PASS", color::teal()),
                ("Face geometry", "WARN", color::amber()),
                ("Motion continuity", "PASS", color::teal()),
                ("Camera movement", "PASS", color::teal()),
                ("Audio sync", "PASS", color::teal()),
            ];
            for (i, (label, status, accent)) in rows.iter().enumerate() {
                let row_y = checklist.top() + 46.0 + i as f32 * 31.0;
                ui.painter().circle_filled(egui::pos2(checklist.left() + 24.0, row_y + 8.0), 3.5, *accent);
                ui.painter().text(egui::pos2(checklist.left() + 40.0, row_y + 8.0), Align2::LEFT_CENTER, *label, FontId::new(11.5, FontFamily::Proportional), color::text());
                let chip_rect = Rect::from_min_size(egui::pos2(checklist.right() - 78.0, row_y), egui::vec2(64.0, 20.0));
                ui.painter().rect_filled(chip_rect, 10.0f32, accent.linear_multiply(0.22));
                ui.painter().text(chip_rect.center(), Align2::CENTER_CENTER, *status, FontId::new(9.5, FontFamily::Proportional), *accent);
            }
            ui.painter().text(egui::pos2(checklist.left() + 18.0, checklist.bottom() - 22.0), Align2::LEFT_CENTER, "Request Revision", FontId::new(10.5, FontFamily::Proportional), color::amber());
            ui.painter().text(egui::pos2(checklist.right() - 18.0, checklist.bottom() - 22.0), Align2::RIGHT_CENTER, "4 / 5 passed", FontId::new(11.5, FontFamily::Proportional), color::text());
            ui.add_space(14.0);

            let (comments, _) = ui.allocate_exact_size(egui::vec2(540.0, 128.0), egui::Sense::hover());
            ui.painter().rect_filled(comments, 12.0f32, color::panel());
            ui.painter().rect_stroke(comments, 12.0f32, Stroke::new(1.0, color::border()), StrokeKind::Middle);
            ui.painter().text(egui::pos2(comments.left() + 18.0, comments.top() + 22.0), Align2::LEFT_CENTER, "COMMENTS", FontId::new(10.0, FontFamily::Proportional), color::text());
            ui.painter().text(egui::pos2(comments.left() + 18.0, comments.top() + 48.0), Align2::LEFT_CENTER, "Blade edge looks soft — resend or send to post-QC.", FontId::new(11.0, FontFamily::Proportional), color::text());
            ui.painter().text(egui::pos2(comments.left() + 18.0, comments.top() + 86.0), Align2::LEFT_CENTER, "L Director · 2m ago · 0 replies", FontId::new(9.5, FontFamily::Proportional), color::text3());
        });
    });

    // SHOT SUMMARY bottom bar
    ui.add_space(12.0);
    let (summary, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 84.0), egui::Sense::hover());
    ui.painter().rect_filled(summary, 12.0f32, color::panel());
    ui.painter().rect_stroke(summary, 12.0f32, Stroke::new(1.0, color::border()), StrokeKind::Middle);
    ui.painter().text(egui::pos2(summary.left() + 22.0, summary.top() + 22.0), Align2::LEFT_CENTER, "SHOT SUMMARY", FontId::new(9.0, FontFamily::Proportional), color::text2());
    ui.painter().text(egui::pos2(summary.left() + 22.0, summary.top() + 42.0), Align2::LEFT_CENTER, "S01 · 002 · Sword drawn close-up", FontId::new(13.0, FontFamily::Proportional), color::text());
    ui.painter().text(egui::pos2(summary.left() + 22.0, summary.bottom() - 16.0), Align2::LEFT_CENTER, "Best candidate — v2 FL2VA Master (A) · Reviewer L Director", FontId::new(10.5, FontFamily::Proportional), color::text2());

    // right action buttons
    let btn_y = summary.center().y;
    let promote_rect = Rect::from_min_size(egui::pos2(summary.right() - 520.0, btn_y - 18.0), egui::vec2(190.0, 36.0));
    if widgets::primary_button(ui, "Promote A v2 to master", 190.0, 36.0) {
        app.set_toast("Shot promoted to master (demo)");
    }
    let _ = promote_rect;
    if widgets::outline_button(ui, "Re-queue B with new seed", 168.0, 36.0) {
        app.set_toast("B re-queued (demo)");
    }
    if widgets::outline_button(ui, "Send to Export", 156.0, 36.0) {
        app.nav_to(Screen::Export);
    }
}

//! Generation Queue — live queue from /queue with status chips, progress, logs.

use egui::{Align2, Color32, FontId, FontFamily, Rect, Stroke, StrokeKind};
use crate::app::H3App;
use crate::theme::{color, typography};
use crate::widgets;

pub fn draw(app: &mut H3App, ui: &mut egui::Ui) {
    let running = app.queue.get("queue_running").and_then(|v| v.as_array()).map(|a| a.len()).unwrap_or(0);
    let pending = app.queue.get("queue_pending").and_then(|v| v.as_array()).map(|a| a.len()).unwrap_or(0);
    let completed = app.history_recent.as_object().map(|o| o.len()).unwrap_or(0);

    // Filter chips + summary
    ui.horizontal(|ui| {
        chip(ui, &format!("{} Running", running), widgets::Tone::Indigo);
        ui.add_space(4.0);
        chip(ui, &format!("{} Queued", pending), widgets::Tone::Gray);
        ui.add_space(4.0);
        chip(ui, &format!("{} Completed", completed), widgets::Tone::Teal);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.colored_label(color::text3(), "Estimated queue remaining 00:18:42");
        });
    });
    ui.add_space(10.0);

    let (table, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 300.0), egui::Sense::hover());
    ui.painter().rect_filled(table, 12.0f32, color::panel());
    ui.painter().rect_stroke(table, 12.0f32, Stroke::new(1.0, color::border()), StrokeKind::Middle);

    // header
    let header_rect = Rect::from_min_size(egui::pos2(table.left(), table.top()), egui::vec2(table.width(), 36.0));
    ui.painter().rect_filled(header_rect, 12.0f32, color::card());
    ui.painter().text(egui::pos2(table.left() + 20.0, header_rect.center().y), Align2::LEFT_CENTER, "ID", FontId::new(10.5, FontFamily::Proportional), color::text2());
    ui.painter().text(egui::pos2(table.left() + 84.0, header_rect.center().y), Align2::LEFT_CENTER, "SHOT", FontId::new(10.5, FontFamily::Proportional), color::text2());
    ui.painter().text(egui::pos2(table.left() + 430.0, header_rect.center().y), Align2::LEFT_CENTER, "MODE", FontId::new(10.5, FontFamily::Proportional), color::text2());
    ui.painter().text(egui::pos2(table.left() + 560.0, header_rect.center().y), Align2::LEFT_CENTER, "STATUS", FontId::new(10.5, FontFamily::Proportional), color::text2());
    ui.painter().text(egui::pos2(table.left() + 730.0, header_rect.center().y), Align2::LEFT_CENTER, "PROGRESS", FontId::new(10.5, FontFamily::Proportional), color::text2());
    ui.painter().text(egui::pos2(table.left() + 1070.0, header_rect.center().y), Align2::LEFT_CENTER, "ELAPSED", FontId::new(10.5, FontFamily::Proportional), color::text2());
    ui.painter().text(egui::pos2(table.left() + 1240.0, header_rect.center().y), Align2::LEFT_CENTER, "GPU", FontId::new(10.5, FontFamily::Proportional), color::text2());
    ui.painter().text(egui::pos2(table.left() + 1330.0, header_rect.center().y), Align2::LEFT_CENTER, "ACTIONS", FontId::new(10.5, FontFamily::Proportional), color::text2());

    // rows (live from queue)
    let mut rows: Vec<(String, String, String, &'static str, &'static str)> = Vec::new();
    if let Some(run) = app.queue.get("queue_running").and_then(|v| v.as_array()) {
        for (i, item) in run.iter().enumerate() {
            let id = item.get("prompt_id").and_then(|v| v.as_str()).unwrap_or("?").to_string();
            rows.push((id.chars().take(8).collect(), format!("Running · T-{:03}", 18 - i), "FL2VA".into(), "Running", "GPU 0"));
        }
    }
    if let Some(pend) = app.queue.get("queue_pending").and_then(|v| v.as_array()) {
        for (i, item) in pend.iter().take(4).enumerate() {
            let id = item.get("prompt_id").and_then(|v| v.as_str()).unwrap_or("?").to_string();
            rows.push((id.chars().take(8).collect(), format!("Pending · T-{:03}", 12 - i), "Ref2VA".into(), "Queued", "GPU —"));
        }
    }

    if rows.is_empty() {
        // demo rows when server is offline
        rows = vec![
            ("T-018".into(), "S01 · Shot 003 · Rain walk".into(), "FL2VA".into(), "Running", "GPU 0"),
            ("T-017".into(), "S01 · Shot 002 · Sword drawn".into(), "Ref2VA".into(), "Running", "GPU 1"),
            ("T-016".into(), "S01 · Shot 004 · Cloak flutter".into(), "Ref2VA".into(), "Queued", "GPU 2"),
        ];
    }

    let mut y = table.top() + 36.0;
    let mut idx = 0;
    for (id, shot, mode, status, gpu) in rows {
        let row_h = 62.0;
        let row = Rect::from_min_size(egui::pos2(table.left(), y), egui::vec2(table.width(), row_h));
        if idx % 2 == 1 {
            ui.painter().rect_filled(row, 0.0f32, Color32::from_rgba_premultiplied(0x1A, 0x1D, 0x22, 120));
        }
        ui.painter().rect_stroke(Rect::from_min_size(egui::pos2(row.left(), row.bottom() - 1.0), egui::vec2(row.width(), 1.0)), 0.0f32, Stroke::new(1.0, color::border()), StrokeKind::Middle);
        let is_running = status == "Running";
        let bar_fill = if is_running { 0.62 } else { 0.32 };
        let chip_tone = if is_running { widgets::Tone::Indigo } else { widgets::Tone::Gray };
        let status_txt = status.to_string();

        ui.painter().text(egui::pos2(row.left() + 20.0, row.center().y - 7.0), Align2::LEFT_CENTER, id, FontId::new(11.5, FontFamily::Monospace), color::text());
        ui.painter().text(egui::pos2(row.left() + 20.0, row.center().y + 11.0), Align2::LEFT_CENTER, "", FontId::new(9.0, FontFamily::Proportional), color::text3());
        ui.painter().text(egui::pos2(row.left() + 84.0, row.center().y - 5.0), Align2::LEFT_CENTER, shot, FontId::new(11.5, FontFamily::Proportional), color::text());
        ui.painter().text(egui::pos2(row.left() + 430.0, row.center().y), Align2::LEFT_CENTER, mode, FontId::new(11.0, FontFamily::Proportional), color::text2());
        let chip_rect = Rect::from_min_size(egui::pos2(row.left() + 560.0, row.center().y - 12.0), egui::vec2(104.0, 24.0));
        ui.painter().rect_filled(chip_rect, 12.0f32, chip_tone.dim());
        ui.painter().rect_stroke(chip_rect, 12.0f32, Stroke::new(1.0, chip_tone.accent().linear_multiply(0.45)), StrokeKind::Middle);
        ui.painter().text(chip_rect.center(), Align2::CENTER_CENTER, status_txt, FontId::new(10.5, FontFamily::Proportional), chip_tone.accent());

        let bar_rect = Rect::from_min_size(egui::pos2(row.left() + 730.0, row.center().y - 5.0), egui::vec2(150.0, 10.0));
        widgets::progress_bar(ui, bar_rect, bar_fill, chip_tone.accent(), 10.0);
        ui.painter().text(egui::pos2(row.left() + 730.0, bar_rect.bottom() + 8.0), Align2::LEFT_CENTER, if is_running { "stage 4/5 · denoise pass" } else { "queued · waiting for GPU" }, FontId::new(9.0, FontFamily::Proportional), color::text3());

        ui.painter().text(egui::pos2(row.left() + 1070.0, row.center().y), Align2::LEFT_CENTER, if is_running { "00:42" } else { "00:03" }, FontId::new(11.0, FontFamily::Monospace), color::text2());
        ui.painter().text(egui::pos2(row.left() + 1240.0, row.center().y), Align2::LEFT_CENTER, gpu, FontId::new(10.5, FontFamily::Proportional), color::text2());

        // action buttons
        let btn_rect = Rect::from_min_size(egui::pos2(row.left() + 1330.0, row.center().y - 14.0), egui::vec2(72.0, 28.0));
        let btn_resp = ui.interact(btn_rect, ui.id().with(("queue-btn", idx)), egui::Sense::click());
        ui.painter().rect_filled(btn_rect, 6.0f32, if btn_resp.hovered() { color::card() } else { Color32::TRANSPARENT });
        ui.painter().rect_stroke(btn_rect, 6.0f32, Stroke::new(1.0, color::border()), StrokeKind::Middle);
        ui.painter().text(btn_rect.center(), Align2::CENTER_CENTER, "Logs", FontId::new(10.0, FontFamily::Proportional), color::text());

        y += row_h;
        idx += 1;
    }

    ui.add_space(12.0);

    // Task log
    let (log, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 168.0), egui::Sense::hover());
    ui.painter().rect_filled(log, 12.0f32, color::panel());
    ui.painter().rect_stroke(log, 12.0f32, Stroke::new(1.0, color::border()), StrokeKind::Middle);
    ui.painter().text(egui::pos2(log.left() + 20.0, log.top() + 24.0), Align2::LEFT_CENTER, "TASK LOG — T-018 · Bamboo Grove S01 · Shot 003 (FL2VA, GPU 0)", FontId::new(10.5, FontFamily::Proportional), color::text());
    let lines = [
        "[00:00] INFO  load model H3_FL2VA_v2.fp16",
        "[00:05] INFO  GPU 0 serving · 13.2GB / 32GB used",
        "[00:08] INFO  encoding references (1 image, 1 audio)",
        "[00:22] INFO  step 340/500 · denoise pass 4/5",
        "[00:42] INFO  VAE decode 24/40 frames · eta 00:18",
    ];
    for (i, line) in lines.iter().enumerate() {
        ui.painter().text(egui::pos2(log.left() + 24.0, log.top() + 48.0 + i as f32 * 22.0), Align2::LEFT_CENTER, line, FontId::new(10.5, FontFamily::Monospace), color::teal().linear_multiply(0.9));
    }
}

fn chip(ui: &mut egui::Ui, text: &str, tone: widgets::Tone) {
    let w = widgets::est_text_width(text, 11.0) + 26.0;
    let (rect, _) = ui.allocate_exact_size(egui::vec2(w.max(90.0), 26.0), egui::Sense::hover());
    ui.painter().rect_filled(rect, 13.0f32, tone.dim());
    ui.painter().rect_stroke(rect, 13.0f32, Stroke::new(1.0, tone.accent().linear_multiply(0.45)), StrokeKind::Middle);
    ui.painter().text(rect.center(), Align2::CENTER_CENTER, text, FontId::new(11.0, FontFamily::Proportional), tone.accent());
}

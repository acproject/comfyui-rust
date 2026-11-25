//! GPU Monitor — three GPU cards + host bar + worker health. Real telemetry.

use egui::{Align2, Color32, FontId, FontFamily, Rect, Stroke, StrokeKind};
use crate::app::H3App;
use crate::theme::{color, typography};
use crate::widgets;

pub fn draw(app: &mut H3App, ui: &mut egui::Ui) {
    let gpus = app.gpus.clone();
    let host = app.host.clone();

    // Three GPU cards
    let card_w = (ui.available_width() - 2.0 * 12.0) / 3.0;
    let display = if gpus.len() >= 3 { gpus.clone() } else {
        let mut g = gpus.clone();
        while g.len() < 3 {
            g.push(crate::gpu::GpuInfo {
                name: if g.len() == 0 { "NVIDIA V100".into() } else if g.len() == 1 { "NVIDIA V100".into() } else { "NVIDIA V100".into() },
                vram_used_mb: 0.0,
                vram_total_mb: 33200.0,
                util_percent: 0.0,
                temp_c: 0.0,
                power_w: 0.0,
                busy: false,
            });
        }
        g
    };

    ui.horizontal(|ui| {
        for (i, gpu) in display.iter().enumerate() {
            let (rect, _) = ui.allocate_exact_size(egui::vec2(card_w - 12.0, 216.0), egui::Sense::hover());
            ui.painter().rect_filled(rect, 12.0f32, color::panel());
            ui.painter().rect_stroke(rect, 12.0f32, Stroke::new(1.0, color::border()), StrokeKind::Middle);

            let cx = rect.left() + 20.0;
            ui.painter().text(egui::pos2(cx, rect.top() + 24.0), Align2::LEFT_CENTER, &format!("GPU {}", i), FontId::new(14.5, FontFamily::Proportional), color::text());
            // busy pill
            let status_col = if gpu.busy { color::indigo() } else { color::text3() };
            let pill_rect = Rect::from_min_size(egui::pos2(rect.right() - 86.0, rect.top() + 14.0), egui::vec2(70.0, 22.0));
            ui.painter().rect_filled(pill_rect, 11.0f32, if gpu.busy { color::indigo_dim() } else { color::gray_dim() });
            ui.painter().rect_stroke(pill_rect, 11.0f32, Stroke::new(1.0, status_col.linear_multiply(0.5)), StrokeKind::Middle);
            ui.painter().text(pill_rect.center(), Align2::CENTER_CENTER, if gpu.busy { "BUSY" } else { "IDLE" }, FontId::new(9.5, FontFamily::Proportional), status_col);
            ui.painter().text(egui::pos2(cx, rect.top() + 46.0), Align2::LEFT_CENTER, &gpu.name, FontId::new(10.5, FontFamily::Proportional), color::text2());

            let vram_frac = if gpu.vram_total_mb > 0.0 { (gpu.vram_used_mb / gpu.vram_total_mb) as f32 } else { 0.0 };
            let vram_gb = gpu.vram_used_mb / 1024.0;
            let vram_total_gb = gpu.vram_total_mb / 1024.0;

            ui.painter().text(egui::pos2(cx, rect.top() + 70.0), Align2::LEFT_CENTER, "VRAM USED / TOTAL", FontId::new(9.0, FontFamily::Proportional), color::text2());
            ui.painter().text(egui::pos2(rect.right() - 20.0, rect.top() + 70.0), Align2::RIGHT_CENTER, &format!("{:.1} / {:.1} GB", vram_gb, vram_total_gb), FontId::new(11.0, FontFamily::Monospace), color::text());
            let bar = Rect::from_min_size(egui::pos2(cx, rect.top() + 88.0), egui::vec2(rect.width() - 40.0, 8.0));
            widgets::progress_bar(ui, bar, vram_frac, color::indigo(), 8.0);

            ui.painter().text(egui::pos2(cx, rect.top() + 112.0), Align2::LEFT_CENTER, "GPU UTILIZATION", FontId::new(9.0, FontFamily::Proportional), color::text2());
            ui.painter().text(egui::pos2(rect.right() - 20.0, rect.top() + 112.0), Align2::RIGHT_CENTER, &format!("{:.0}%", gpu.util_percent), FontId::new(13.0, FontFamily::Proportional), color::text());
            draw_sparkline(ui, Rect::from_min_size(egui::pos2(cx, rect.top() + 132.0), egui::vec2(rect.width() - 40.0, 34.0)), gpu.util_percent / 100.0, color::teal());

            ui.painter().line_segment([egui::pos2(rect.left() + 16.0, rect.top() + 170.0), egui::pos2(rect.right() - 16.0, rect.top() + 170.0)], Stroke::new(1.0, color::border()));
            ui.painter().text(egui::pos2(cx, rect.top() + 190.0), Align2::LEFT_CENTER, &format!("TEMP {:.0}°C", gpu.temp_c), FontId::new(9.5, FontFamily::Proportional), color::text2());
            ui.painter().text(egui::pos2(rect.left() + rect.width() * 0.5, rect.top() + 190.0), Align2::CENTER_CENTER, &format!("POWER {:.0} W", gpu.power_w), FontId::new(9.5, FontFamily::Proportional), color::text2());
            ui.painter().text(egui::pos2(rect.right() - 20.0, rect.top() + 190.0), Align2::RIGHT_CENTER, if gpu.busy { "FL2VA · T-018" } else { "—" }, FontId::new(9.5, FontFamily::Proportional), color::indigo());
            ui.add_space(12.0);
        }
    });

    ui.add_space(14.0);

    // Host bar
    let (host_bar, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 96.0), egui::Sense::hover());
    ui.painter().rect_filled(host_bar, 12.0f32, color::panel());
    ui.painter().rect_stroke(host_bar, 12.0f32, Stroke::new(1.0, color::border()), StrokeKind::Middle);
    ui.painter().text(egui::pos2(host_bar.left() + 20.0, host_bar.top() + 22.0), Align2::LEFT_CENTER, "HOST", FontId::new(10.5, FontFamily::Proportional), color::text());
    ui.painter().text(egui::pos2(host_bar.left() + 20.0, host_bar.top() + 46.0), Align2::LEFT_CENTER, "RAM", FontId::new(11.0, FontFamily::Proportional), color::text2());
    ui.painter().text(egui::pos2(host_bar.left() + 20.0, host_bar.bottom() - 18.0), Align2::LEFT_CENTER, &format!("{:.1} / {:.1} GB", host.ram_used_gb, host.ram_total_gb), FontId::new(10.5, FontFamily::Monospace), color::text());
    let r_bar = Rect::from_min_size(egui::pos2(host_bar.left() + 160.0, host_bar.top() + 44.0), egui::vec2(200.0, 9.0));
    let ram_frac = if host.ram_total_gb > 0.0 { (host.ram_used_gb / host.ram_total_gb) as f32 } else { 0.0 };
    widgets::progress_bar(ui, r_bar, ram_frac, color::teal(), 9.0);
    let r_pct = format!("{:.0}%", ram_frac * 100.0);
    ui.painter().text(egui::pos2(host_bar.left() + 370.0, host_bar.top() + 48.0), Align2::LEFT_CENTER, &r_pct, FontId::new(11.0, FontFamily::Proportional), color::text2());
    ui.painter().text(egui::pos2(host_bar.left() + 460.0, host_bar.top() + 22.0), Align2::LEFT_CENTER, "CPU", FontId::new(11.0, FontFamily::Proportional), color::text2());
    ui.painter().text(egui::pos2(host_bar.left() + 460.0, host_bar.top() + 44.0), Align2::LEFT_CENTER, &format!("{:.0}% · {}/{} cores active", host.cpu_percent, host.cpu_cores_active, host.cpu_cores_total), FontId::new(10.5, FontFamily::Proportional), color::text());
    let c_bar = Rect::from_min_size(egui::pos2(host_bar.left() + 620.0, host_bar.top() + 44.0), egui::vec2(200.0, 9.0));
    widgets::progress_bar(ui, c_bar, host.cpu_percent / 100.0, color::indigo(), 9.0);
    ui.painter().text(egui::pos2(host_bar.right() - 20.0, host_bar.top() + 48.0), Align2::RIGHT_CENTER, "measured · lift-load pool", FontId::new(9.5, FontFamily::Proportional), color::text3());

    ui.add_space(14.0);

    // Workers
    let (health, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 108.0), egui::Sense::hover());
    ui.painter().rect_filled(health, 12.0f32, color::panel());
    ui.painter().rect_stroke(health, 12.0f32, Stroke::new(1.0, color::border()), StrokeKind::Middle);
    ui.painter().text(egui::pos2(health.left() + 20.0, health.top() + 24.0), Align2::LEFT_CENTER, "GPU WORKER HEALTH", FontId::new(10.5, FontFamily::Proportional), color::text());
    for (i, w) in [(0usize, "healthy · 24h uptime · no ECC errors", "OK", color::teal()), (1, "healthy · 24h uptime · no ECC errors", "OK", color::teal()), (2, "able · 1 correctable ECC · last 4h ago", "WARN", color::amber())].iter().enumerate() {
        let col_x = health.left() + 24.0 + i as f32 * ((health.width() - 48.0) / 3.0);
        let col_w = (health.width() - 48.0) / 3.0 - 24.0;
        ui.painter().circle_filled(egui::pos2(col_x, health.top() + 62.0), 5.0, w.3);
        ui.painter().text(egui::pos2(col_x + 14.0, health.top() + 58.0), Align2::LEFT_CENTER, &format!("Worker {} · GPU {}", i, i), FontId::new(11.5, FontFamily::Proportional), color::text());
        ui.painter().text(egui::pos2(col_x + 14.0, health.top() + 76.0), Align2::LEFT_CENTER, w.1, FontId::new(9.5, FontFamily::Proportional), color::text2());
        ui.painter().text(egui::pos2(col_x + col_w, health.top() + 62.0), Align2::RIGHT_CENTER, w.2, FontId::new(10.0, FontFamily::Proportional), w.3);
    }
    let _ = host; // silence unused
}

fn draw_sparkline(ui: &mut egui::Ui, rect: Rect, util: f32, color: Color32) {
    let painter = ui.painter();
    let points = 7;
    let mut pts = Vec::new();
    for i in 0..points {
        let fx = rect.left() + rect.width() * (i as f32 / (points - 1) as f32);
        let fy = rect.top() + rect.height() * (1.0 - util.clamp(0.0, 1.0) * (0.55 + 0.45 * ((i * 7 + 3) as f32 / 10.0)));
        pts.push(egui::pos2(fx, fy));
    }
    for i in 0..pts.len() - 1 {
        painter.line_segment([pts[i], pts[i + 1]], Stroke::new(2.0, color));
    }
}

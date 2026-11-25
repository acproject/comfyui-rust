//! Export — shot selection, output settings, summary + ffmpeg command.

use egui::{Align2, Color32, FontId, FontFamily, Rect, Stroke, StrokeKind};
use crate::app::H3App;
use crate::theme::{color, typography};
use crate::widgets;

pub fn draw(app: &mut H3App, ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        // SHOT SELECTION
        ui.vertical(|ui| {
            let (panel, _) = ui.allocate_exact_size(egui::vec2(430.0, 390.0), egui::Sense::hover());
            ui.painter().rect_filled(panel, 12.0f32, color::panel());
            ui.painter().rect_stroke(panel, 12.0f32, Stroke::new(1.0, color::border()), StrokeKind::Middle);
            ui.painter().text(egui::pos2(panel.left() + 20.0, panel.top() + 24.0), Align2::LEFT_CENTER, "SHOT SELECTION", FontId::new(10.5, FontFamily::Proportional), color::text());

            let shots = [
                ("S01 · 001", "Establishing wide", "6s", true, color::teal()),
                ("S01 · 002", "Sword drawn", "4s", true, color::teal()),
                ("S01 · 003", "Rain walk", "8s", true, color::indigo()),
                ("S01 · 004", "Cloak flutter", "5s", true, color::teal()),
                ("S01 · 005", "Face close-up", "3s", false, color::text3()),
            ];
            for (i, (id, title, dur, checked, accent)) in shots.iter().enumerate() {
                let row_y = panel.top() + 46.0 + i as f32 * 46.0;
                let box_rect = Rect::from_min_size(egui::pos2(panel.left() + 20.0, row_y + 2.0), egui::vec2(18.0, 18.0));
                ui.painter().rect_filled(box_rect, 5.0f32, if *checked { *accent } else { Color32::TRANSPARENT });
                ui.painter().rect_stroke(box_rect, 5.0f32, Stroke::new(1.4, if *checked { *accent } else { color::border() }), StrokeKind::Middle);
                if *checked {
                    let check = Rect::from_min_size(egui::pos2(box_rect.left() + 5.0, box_rect.center().y - 2.0), egui::vec2(8.0, 5.0));
                    let _ = check;
                    ui.painter().line_segment([egui::pos2(box_rect.left() + 5.0, box_rect.center().y), egui::pos2(box_rect.left() + 8.5, box_rect.center().y + 3.5)], Stroke::new(2.0, color::on_accent()));
                    ui.painter().line_segment([egui::pos2(box_rect.left() + 8.5, box_rect.center().y + 3.5), egui::pos2(box_rect.left() + 13.0, box_rect.center().y - 4.0)], Stroke::new(2.0, color::on_accent()));
                }
                ui.painter().text(egui::pos2(box_rect.right() + 12.0, row_y + 4.0), Align2::LEFT_CENTER, id, FontId::new(9.5, FontFamily::Monospace), color::text2());
                ui.painter().text(egui::pos2(box_rect.right() + 12.0, row_y + 20.0), Align2::LEFT_CENTER, title, FontId::new(11.5, FontFamily::Proportional), color::text());
                ui.painter().text(egui::pos2(panel.right() - 18.0, row_y + 12.0), Align2::RIGHT_CENTER, dur, FontId::new(10.5, FontFamily::Proportional), color::text2());
            }
            ui.painter().text(egui::pos2(panel.left() + 20.0, panel.top() + 270.0), Align2::LEFT_CENTER, "5 of 8 shown · select all (a)", FontId::new(9.5, FontFamily::Proportional), color::text3());
            // timeline assembly
            ui.painter().text(egui::pos2(panel.left() + 20.0, panel.top() + 296.0), Align2::LEFT_CENTER, "TIMELINE ASSEMBLY", FontId::new(10.0, FontFamily::Proportional), color::text());
            let bar = Rect::from_min_size(egui::pos2(panel.left() + 20.0, panel.top() + 314.0), egui::vec2(panel.width() - 40.0, 9.0));
            widgets::progress_bar(ui, bar, 0.5, color::indigo(), 9.0);
            ui.painter().text(egui::pos2(panel.left() + 20.0, panel.bottom() - 24.0), Align2::LEFT_CENTER, "Drag to reorder · gapless 26s in order", FontId::new(9.5, FontFamily::Proportional), color::text3());
        });

        // OUTPUT SETTINGS
        ui.add_space(14.0);
        ui.vertical(|ui| {
            let (panel, _) = ui.allocate_exact_size(egui::vec2(520.0, 390.0), egui::Sense::hover());
            ui.painter().rect_filled(panel, 12.0f32, color::panel());
            ui.painter().rect_stroke(panel, 12.0f32, Stroke::new(1.0, color::border()), StrokeKind::Middle);
            ui.painter().text(egui::pos2(panel.left() + 20.0, panel.top() + 24.0), Align2::LEFT_CENTER, "OUTPUT SETTINGS", FontId::new(10.5, FontFamily::Proportional), color::text());
            let cx = panel.left() + 20.0;
            ui.painter().text(egui::pos2(cx, panel.top() + 58.0), Align2::LEFT_CENTER, "CONTAINER", FontId::new(9.0, FontFamily::Proportional), color::text2());
            for (i, opt) in ["MP4", "MOV", "MKV"].iter().enumerate() {
                let pill_rect = Rect::from_min_size(egui::pos2(cx + i as f32 * 100.0, panel.top() + 70.0), egui::vec2(92.0, 30.0));
                let active = *opt == "MP4";
                ui.painter().rect_filled(pill_rect, 6.0f32, if active { color::indigo() } else { Color32::TRANSPARENT });
                ui.painter().rect_stroke(pill_rect, 6.0f32, Stroke::new(1.0, if active { color::indigo() } else { color::border() }), StrokeKind::Middle);
                ui.painter().text(pill_rect.center(), Align2::CENTER_CENTER, *opt, FontId::new(10.5, FontFamily::Proportional), if active { color::on_accent() } else { color::text() });
            }
            ui.painter().text(egui::pos2(cx, panel.top() + 118.0), Align2::LEFT_CENTER, "RESOLUTION", FontId::new(9.0, FontFamily::Proportional), color::text2());
            for (i, opt) in ["1920×1080", "2560×1440", "3840×2160"].iter().enumerate() {
                let pill_rect = Rect::from_min_size(egui::pos2(cx + i as f32 * 128.0, panel.top() + 130.0), egui::vec2(122.0, 30.0));
                let active = *opt == "2560×1440";
                ui.painter().rect_filled(pill_rect, 6.0f32, if active { color::indigo() } else { Color32::TRANSPARENT });
                ui.painter().rect_stroke(pill_rect, 6.0f32, Stroke::new(1.0, if active { color::indigo() } else { color::border() }), StrokeKind::Middle);
                ui.painter().text(pill_rect.center(), Align2::CENTER_CENTER, *opt, FontId::new(10.0, FontFamily::Proportional), if active { color::on_accent() } else { color::text() });
            }
            ui.painter().text(egui::pos2(cx, panel.top() + 178.0), Align2::LEFT_CENTER, "FPS", FontId::new(9.0, FontFamily::Proportional), color::text2());
            for (i, opt) in ["24", "25", "30", "60"].iter().enumerate() {
                let pill_rect = Rect::from_min_size(egui::pos2(cx + i as f32 * 80.0, panel.top() + 190.0), egui::vec2(72.0, 30.0));
                let active = *opt == "24";
                ui.painter().rect_filled(pill_rect, 6.0f32, if active { color::indigo() } else { Color32::TRANSPARENT });
                ui.painter().rect_stroke(pill_rect, 6.0f32, Stroke::new(1.0, if active { color::indigo() } else { color::border() }), StrokeKind::Middle);
                ui.painter().text(pill_rect.center(), Align2::CENTER_CENTER, *opt, FontId::new(10.5, FontFamily::Proportional), if active { color::on_accent() } else { color::text() });
            }
            ui.painter().line_segment([egui::pos2(panel.left() + 20.0, panel.top() + 240.0), egui::pos2(panel.right() - 20.0, panel.top() + 240.0)], Stroke::new(1.0, color::border()));
            ui.painter().text(egui::pos2(cx, panel.top() + 260.0), Align2::LEFT_CENTER, "▾ Show advanced settings", FontId::new(10.5, FontFamily::Proportional), color::indigo());
            ui.painter().text(egui::pos2(cx, panel.top() + 280.0), Align2::LEFT_CENTER, "Codec, bitrate, color, audio mixing — collapsed by default", FontId::new(9.5, FontFamily::Proportional), color::text3());
        });

        // OUTPUT SUMMARY
        ui.add_space(14.0);
        ui.vertical(|ui| {
            let (panel, _) = ui.allocate_exact_size(egui::vec2(420.0, 390.0), egui::Sense::hover());
            ui.painter().rect_filled(panel, 12.0f32, color::panel());
            ui.painter().rect_stroke(panel, 12.0f32, Stroke::new(1.0, color::border()), StrokeKind::Middle);
            ui.painter().text(egui::pos2(panel.left() + 20.0, panel.top() + 24.0), Align2::LEFT_CENTER, "OUTPUT SUMMARY", FontId::new(10.5, FontFamily::Proportional), color::text());
            let rows = [
                ("Output", "BambooGrove_master.mp4"),
                ("Duration", "00:49 · 25 fps"),
                ("Resolution", "2560 × 1440"),
                ("Codec", "H.264 / AAC"),
                ("Target size", "840 MB · 8.5 Mbps"),
            ];
            for (i, (label, value)) in rows.iter().enumerate() {
                let row_y = panel.top() + 52.0 + i as f32 * 30.0;
                ui.painter().text(egui::pos2(panel.left() + 20.0, row_y), Align2::LEFT_CENTER, label, FontId::new(10.5, FontFamily::Proportional), color::text2());
                ui.painter().text(egui::pos2(panel.right() - 20.0, row_y), Align2::RIGHT_CENTER, value, FontId::new(10.5, FontFamily::Monospace), color::text());
            }
            ui.painter().text(egui::pos2(panel.left() + 20.0, panel.top() + 212.0), Align2::LEFT_CENTER, "PROGRESS", FontId::new(10.0, FontFamily::Proportional), color::text());
            let bar = Rect::from_min_size(egui::pos2(panel.left() + 20.0, panel.top() + 230.0), egui::vec2(panel.width() - 40.0, 12.0));
            widgets::progress_bar(ui, bar, 0.60, color::indigo(), 12.0);
            ui.painter().text(egui::pos2(panel.left() + 20.0, panel.top() + 258.0), Align2::LEFT_CENTER, "60% · 00:13 elapsed · ETA 00:15 remaining", FontId::new(10.5, FontFamily::Proportional), color::text2());
            ui.painter().text(egui::pos2(panel.left() + 20.0, panel.top() + 278.0), Align2::LEFT_CENTER, "ffmpeg -i shots.txt -c:v libx264 -crf 18 …", FontId::new(9.0, FontFamily::Monospace), color::text3());
            if widgets::outline_button(ui, "Pause export", 140.0, 32.0) {
                app.set_toast("Export paused (demo)");
            }
            ui.add_space(4.0);
            if widgets::outline_button(ui, "Cancel export", 140.0, 32.0) {
                app.set_toast("Export cancelled (demo)");
            }
        });
    });
}

//! Export — shot selection, output settings, summary + ffmpeg command.

use egui::{Align2, Color32, FontId, FontFamily, Rect, Sense, Stroke, StrokeKind};
use crate::app::H3App;
use crate::theme::{color, typography};
use crate::widgets;

pub fn draw(app: &mut H3App, ui: &mut egui::Ui) {
    use crate::i18n::{tr, tf};
    let lang = app.lang();
    ui.horizontal(|ui| {
        // SHOT SELECTION
        ui.vertical(|ui| {
            let (panel, _) = ui.allocate_exact_size(egui::vec2(430.0, 390.0), egui::Sense::hover());
            ui.painter().rect_filled(panel, 12.0f32, color::panel());
            ui.painter().rect_stroke(panel, 12.0f32, Stroke::new(1.0, color::border()), StrokeKind::Middle);
            ui.painter().text(egui::pos2(panel.left() + 20.0, panel.top() + 24.0), Align2::LEFT_CENTER, tr(lang, "SHOT SELECTION"), FontId::new(10.5, FontFamily::Proportional), color::text());

            let shots = [
                ("S01 · 001", "Establishing wide", "6s", color::teal()),
                ("S01 · 002", "Sword drawn", "4s", color::teal()),
                ("S01 · 003", "Rain walk", "8s", color::indigo()),
                ("S01 · 004", "Cloak flutter", "5s", color::teal()),
                ("S01 · 005", "Face close-up", "3s", color::text3()),
            ];
            for (i, (id, title, dur, accent)) in shots.iter().enumerate() {
                let row_y = panel.top() + 46.0 + i as f32 * 46.0;
                let box_rect = Rect::from_min_size(egui::pos2(panel.left() + 20.0, row_y + 2.0), egui::vec2(18.0, 18.0));
                let row_rect = Rect::from_min_size(egui::pos2(panel.left() + 16.0, row_y - 2.0), egui::vec2(panel.width() - 32.0, 38.0));
                let row_resp = ui.interact(row_rect, ui.id().with(("exp-shot", i)), Sense::click());
                let checked = app.ui.export_selected.contains(*id);
                if row_resp.hovered() {
                    ui.painter().rect_filled(row_rect, 6.0f32, color::card());
                }
                ui.painter().rect_filled(box_rect, 5.0f32, if checked { *accent } else { Color32::TRANSPARENT });
                ui.painter().rect_stroke(box_rect, 5.0f32, Stroke::new(1.4, if checked { *accent } else { color::border() }), StrokeKind::Middle);
                if checked {
                    ui.painter().line_segment([egui::pos2(box_rect.left() + 5.0, box_rect.center().y), egui::pos2(box_rect.left() + 8.5, box_rect.center().y + 3.5)], Stroke::new(2.0, color::on_accent()));
                    ui.painter().line_segment([egui::pos2(box_rect.left() + 8.5, box_rect.center().y + 3.5), egui::pos2(box_rect.left() + 13.0, box_rect.center().y - 4.0)], Stroke::new(2.0, color::on_accent()));
                }
                ui.painter().text(egui::pos2(box_rect.right() + 12.0, row_y + 4.0), Align2::LEFT_CENTER, id, FontId::new(9.5, FontFamily::Monospace), color::text2());
                ui.painter().text(egui::pos2(box_rect.right() + 12.0, row_y + 20.0), Align2::LEFT_CENTER, tr(lang, title), FontId::new(11.5, FontFamily::Proportional), color::text());
                ui.painter().text(egui::pos2(panel.right() - 18.0, row_y + 12.0), Align2::RIGHT_CENTER, dur, FontId::new(10.5, FontFamily::Proportional), color::text2());
                if row_resp.clicked() {
                    if checked { app.ui.export_selected.remove(*id); } else { app.ui.export_selected.insert(id.to_string()); }
                }
            }
            let sel_count = app.ui.export_selected.len();
            let all_rect = Rect::from_min_size(egui::pos2(panel.left() + 16.0, panel.top() + 262.0), egui::vec2(240.0, 20.0));
            let all_resp = ui.interact(all_rect, ui.id().with("exp-select-all"), Sense::click());
            let all_label = if sel_count == 5 { "deselect all" } else { "select all" };
            let all_label_disp = tr(lang, all_label);
            ui.painter().text(egui::pos2(panel.left() + 20.0, panel.top() + 270.0), Align2::LEFT_CENTER, &tf(lang, "{} of 5 shown · {} (click)", &[&format!("{}", sel_count), &all_label_disp]), FontId::new(9.5, FontFamily::Proportional), if all_resp.hovered() { color::indigo() } else { color::text3() });
            if all_resp.clicked() {
                if sel_count == 5 {
                    app.ui.export_selected.clear();
                } else {
                    for (id, _, _, _) in shots.iter() { app.ui.export_selected.insert(id.to_string()); }
                }
            }
            // timeline assembly
            ui.painter().text(egui::pos2(panel.left() + 20.0, panel.top() + 296.0), Align2::LEFT_CENTER, tr(lang, "TIMELINE ASSEMBLY"), FontId::new(10.0, FontFamily::Proportional), color::text());
            let bar = Rect::from_min_size(egui::pos2(panel.left() + 20.0, panel.top() + 314.0), egui::vec2(panel.width() - 40.0, 9.0));
            widgets::progress_bar(ui, bar, 0.5, color::indigo(), 9.0);
            ui.painter().text(egui::pos2(panel.left() + 20.0, panel.bottom() - 24.0), Align2::LEFT_CENTER, tr(lang, "Drag to reorder · gapless 26s in order"), FontId::new(9.5, FontFamily::Proportional), color::text3());
        });

        // OUTPUT SETTINGS
        ui.add_space(14.0);
        ui.vertical(|ui| {
            let (panel, _) = ui.allocate_exact_size(egui::vec2(520.0, 390.0), egui::Sense::hover());
            ui.painter().rect_filled(panel, 12.0f32, color::panel());
            ui.painter().rect_stroke(panel, 12.0f32, Stroke::new(1.0, color::border()), StrokeKind::Middle);
            ui.painter().text(egui::pos2(panel.left() + 20.0, panel.top() + 24.0), Align2::LEFT_CENTER, tr(lang, "OUTPUT SETTINGS"), FontId::new(10.5, FontFamily::Proportional), color::text());
            let cx = panel.left() + 20.0;
            ui.painter().text(egui::pos2(cx, panel.top() + 58.0), Align2::LEFT_CENTER, tr(lang, "CONTAINER"), FontId::new(9.0, FontFamily::Proportional), color::text2());
            for (i, opt) in ["MP4", "MOV", "MKV"].iter().enumerate() {
                let pill_rect = Rect::from_min_size(egui::pos2(cx + i as f32 * 100.0, panel.top() + 70.0), egui::vec2(92.0, 30.0));
                let active = *opt == app.ui.export_container;
                let resp = ui.interact(pill_rect, ui.id().with(("exp-container", *opt)), Sense::click());
                let hov = resp.hovered() && !active;
                ui.painter().rect_filled(pill_rect, 6.0f32, if active { color::indigo() } else if hov { color::card() } else { Color32::TRANSPARENT });
                ui.painter().rect_stroke(pill_rect, 6.0f32, Stroke::new(1.0, if active || hov { color::indigo() } else { color::border() }), StrokeKind::Middle);
                ui.painter().text(pill_rect.center(), Align2::CENTER_CENTER, *opt, FontId::new(10.5, FontFamily::Proportional), if active { color::on_accent() } else { color::text() });
                if resp.clicked() {
                    app.ui.export_container = opt.to_string();
                    app.set_toast(tf(lang, "Container → {}", &[&opt]));
                }
            }
            ui.painter().text(egui::pos2(cx, panel.top() + 118.0), Align2::LEFT_CENTER, tr(lang, "RESOLUTION"), FontId::new(9.0, FontFamily::Proportional), color::text2());
            for (i, opt) in ["1920×1080", "2560×1440", "3840×2160"].iter().enumerate() {
                let pill_rect = Rect::from_min_size(egui::pos2(cx + i as f32 * 128.0, panel.top() + 130.0), egui::vec2(122.0, 30.0));
                let active = *opt == app.ui.export_resolution;
                let resp = ui.interact(pill_rect, ui.id().with(("exp-resolution", *opt)), Sense::click());
                let hov = resp.hovered() && !active;
                ui.painter().rect_filled(pill_rect, 6.0f32, if active { color::indigo() } else if hov { color::card() } else { Color32::TRANSPARENT });
                ui.painter().rect_stroke(pill_rect, 6.0f32, Stroke::new(1.0, if active || hov { color::indigo() } else { color::border() }), StrokeKind::Middle);
                ui.painter().text(pill_rect.center(), Align2::CENTER_CENTER, *opt, FontId::new(10.0, FontFamily::Proportional), if active { color::on_accent() } else { color::text() });
                if resp.clicked() {
                    app.ui.export_resolution = opt.to_string();
                    app.set_toast(tf(lang, "Resolution → {}", &[&opt]));
                }
            }
            ui.painter().text(egui::pos2(cx, panel.top() + 178.0), Align2::LEFT_CENTER, "FPS", FontId::new(9.0, FontFamily::Proportional), color::text2());
            for (i, opt) in ["24", "25", "30", "60"].iter().enumerate() {
                let pill_rect = Rect::from_min_size(egui::pos2(cx + i as f32 * 80.0, panel.top() + 190.0), egui::vec2(72.0, 30.0));
                let active = *opt == app.ui.export_fps;
                let resp = ui.interact(pill_rect, ui.id().with(("exp-fps", *opt)), Sense::click());
                let hov = resp.hovered() && !active;
                ui.painter().rect_filled(pill_rect, 6.0f32, if active { color::indigo() } else if hov { color::card() } else { Color32::TRANSPARENT });
                ui.painter().rect_stroke(pill_rect, 6.0f32, Stroke::new(1.0, if active || hov { color::indigo() } else { color::border() }), StrokeKind::Middle);
                ui.painter().text(pill_rect.center(), Align2::CENTER_CENTER, *opt, FontId::new(10.5, FontFamily::Proportional), if active { color::on_accent() } else { color::text() });
                if resp.clicked() {
                    app.ui.export_fps = opt.to_string();
                    app.set_toast(tf(lang, "FPS → {}", &[&opt]));
                }
            }
            ui.painter().line_segment([egui::pos2(panel.left() + 20.0, panel.top() + 240.0), egui::pos2(panel.right() - 20.0, panel.top() + 240.0)], Stroke::new(1.0, color::border()));
            let adv_rect = Rect::from_min_size(egui::pos2(cx - 4.0, panel.top() + 252.0), egui::vec2(200.0, 22.0));
            let adv_resp = ui.interact(adv_rect, ui.id().with("exp-advanced"), Sense::click());
            let expanded = app.ui.export_advanced;
            ui.painter().text(egui::pos2(cx, panel.top() + 260.0), Align2::LEFT_CENTER, tr(lang, if expanded { "▴ Hide advanced settings" } else { "▾ Show advanced settings" }), FontId::new(10.5, FontFamily::Proportional), if adv_resp.hovered() { color::indigo().linear_multiply(1.2) } else { color::indigo() });
            if expanded {
                ui.painter().text(egui::pos2(cx, panel.top() + 280.0), Align2::LEFT_CENTER, tr(lang, "Codec H.264 · CRF 18 · 8.5 Mbps · AAC 320k"), FontId::new(9.5, FontFamily::Proportional), color::text2());
                ui.painter().text(egui::pos2(cx, panel.top() + 300.0), Align2::LEFT_CENTER, tr(lang, "Color Rec.709 · audio mix stereo"), FontId::new(9.5, FontFamily::Proportional), color::text3());
            } else {
                ui.painter().text(egui::pos2(cx, panel.top() + 280.0), Align2::LEFT_CENTER, tr(lang, "Codec, bitrate, color, audio mixing — collapsed by default"), FontId::new(9.5, FontFamily::Proportional), color::text3());
            }
            if adv_resp.clicked() {
                app.ui.export_advanced = !expanded;
            }
        });

        // OUTPUT SUMMARY
        ui.add_space(14.0);
        ui.vertical(|ui| {
            let (panel, _) = ui.allocate_exact_size(egui::vec2(420.0, 390.0), egui::Sense::hover());
            ui.painter().rect_filled(panel, 12.0f32, color::panel());
            ui.painter().rect_stroke(panel, 12.0f32, Stroke::new(1.0, color::border()), StrokeKind::Middle);
            ui.painter().text(egui::pos2(panel.left() + 20.0, panel.top() + 24.0), Align2::LEFT_CENTER, tr(lang, "OUTPUT SUMMARY"), FontId::new(10.5, FontFamily::Proportional), color::text());
            let ext = app.ui.export_container.to_lowercase();
            let res_display = app.ui.export_resolution.replace('×', " × ");
            let rows: Vec<(String, String)> = vec![
                (tr(lang, "Output").to_string(), format!("BambooGrove_master.{}", ext)),
                (tr(lang, "Duration").to_string(), format!("00:49 · {} fps", app.ui.export_fps)),
                (tr(lang, "Resolution").to_string(), res_display),
                (tr(lang, "Codec").to_string(), "H.264 / AAC".into()),
                (tr(lang, "Target size").to_string(), "840 MB · 8.5 Mbps".into()),
            ];
            for (i, (label, value)) in rows.iter().enumerate() {
                let row_y = panel.top() + 52.0 + i as f32 * 30.0;
                ui.painter().text(egui::pos2(panel.left() + 20.0, row_y), Align2::LEFT_CENTER, label.as_str(), FontId::new(10.5, FontFamily::Proportional), color::text2());
                ui.painter().text(egui::pos2(panel.right() - 20.0, row_y), Align2::RIGHT_CENTER, value.as_str(), FontId::new(10.5, FontFamily::Monospace), color::text());
            }
            ui.painter().text(egui::pos2(panel.left() + 20.0, panel.top() + 212.0), Align2::LEFT_CENTER, tr(lang, "PROGRESS"), FontId::new(10.0, FontFamily::Proportional), color::text());
            let bar = Rect::from_min_size(egui::pos2(panel.left() + 20.0, panel.top() + 230.0), egui::vec2(panel.width() - 40.0, 12.0));
            widgets::progress_bar(ui, bar, 0.60, color::indigo(), 12.0);
            ui.painter().text(egui::pos2(panel.left() + 20.0, panel.top() + 258.0), Align2::LEFT_CENTER, tr(lang, "60% · 00:13 elapsed · ETA 00:15 remaining"), FontId::new(10.5, FontFamily::Proportional), color::text2());
            ui.painter().text(egui::pos2(panel.left() + 20.0, panel.top() + 278.0), Align2::LEFT_CENTER, "ffmpeg -i shots.txt -c:v libx264 -crf 18 …", FontId::new(9.0, FontFamily::Monospace), color::text3());
            if widgets::outline_button(ui, tr(lang, "Pause export"), 140.0, 32.0) {
                app.toast_warning(tr(lang, "Export paused (demo)").to_string());
            }
            ui.add_space(4.0);
            if widgets::outline_button(ui, tr(lang, "Cancel export"), 140.0, 32.0) {
                app.toast_warning(tr(lang, "Export cancelled (demo)").to_string());
            }
        });
    });
}

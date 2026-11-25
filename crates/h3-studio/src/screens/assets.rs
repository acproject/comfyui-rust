//! Reference assets — import zone, filters, server asset grid with previews.

use egui::{Align2, Color32, FontId, FontFamily, Rect, Stroke, StrokeKind};
use crate::app::{H3App};
use crate::theme::{color, typography};
use crate::widgets;

pub fn draw(app: &mut H3App, ui: &mut egui::Ui) {
    // Intro line
    ui.colored_label(color::text3(), "Drag & drop to import · assign reference roles explicitly");
    ui.add_space(14.0);

    let n_assets = app.assets.get("assets").and_then(|v| v.as_array()).map(|a| a.len()).unwrap_or(0);

    ui.horizontal(|ui| {
        // IMPORT ZONE
        ui.vertical(|ui| {
            let (zone, resp) = ui.allocate_exact_size(egui::vec2(340.0, 240.0), egui::Sense::click());
            ui.painter().rect_filled(zone, 12.0f32, color::panel());
            ui.painter().rect_stroke(zone, 12.0f32, Stroke::new(1.0, color::border()), StrokeKind::Middle);
            ui.painter().text(egui::pos2(zone.left() + 20.0, zone.top() + 24.0), Align2::LEFT_CENTER, "IMPORT ZONE", FontId::new(10.5, FontFamily::Proportional), color::text2());
            let drop = Rect::from_min_size(egui::pos2(zone.left() + 20.0, zone.top() + 44.0), egui::vec2(300.0, 130.0));
            ui.painter().rect_filled(drop, 10.0f32, color::bg());
            // dashed border (simulated with segments)
            let dash = 8.0;
            let gap = 8.0;
            let stroke = Stroke::new(1.6, color::indigo().linear_multiply(0.8));
            for x in (drop.left() as i32..drop.right() as i32).step_by((dash + gap) as usize) {
                let x0 = x as f32;
                let x1 = (x0 + dash).min(drop.right());
                ui.painter().line_segment([egui::pos2(x0, drop.top()), egui::pos2(x1, drop.top())], stroke);
                ui.painter().line_segment([egui::pos2(x0, drop.bottom()), egui::pos2(x1, drop.bottom())], stroke);
            }
            for y in (drop.top() as i32..drop.bottom() as i32).step_by((dash + gap) as usize) {
                let y0 = y as f32;
                let y1 = (y0 + dash).min(drop.bottom());
                ui.painter().line_segment([egui::pos2(drop.left(), y0), egui::pos2(drop.left(), y1)], stroke);
                ui.painter().line_segment([egui::pos2(drop.right(), y0), egui::pos2(drop.right(), y1)], stroke);
            }
            ui.painter().text(drop.center(), Align2::CENTER_CENTER, "Drag & Drop Files", FontId::new(13.0, FontFamily::Proportional), color::text());
            ui.painter().text(egui::pos2(drop.center().x, drop.bottom() - 24.0), Align2::CENTER_CENTER, "Supported: images, video, audio, pose, depth", FontId::new(9.5, FontFamily::Proportional), color::text3());
            ui.painter().text(egui::pos2(zone.left() + 22.0, zone.bottom() - 18.0), Align2::LEFT_CENTER, "Files import as assets only · role assigned by user", FontId::new(9.5, FontFamily::Proportional), color::text3());

            if resp.clicked() || resp.drag_started() {
                if let Some(path) = rfd::FileDialog::new().add_filter("images", &["png", "jpg", "jpeg", "webp"]).add_filter("all", &["*"]).pick_file() {
                    if let Ok(bytes) = std::fs::read(&path) {
                        let filename = path.file_name().and_then(|n| n.to_str()).unwrap_or("upload.png").to_string();
                        let toast_name = filename.clone();
                        let client = app.client.clone();
                        std::thread::spawn(move || {
                            let _ = client.upload_asset(bytes, &filename);
                        });
                        app.set_toast(format!("Imported {} (uploading → Assets server)", toast_name));
                    }
                }
            }

            // filters
            let (filters, _) = ui.allocate_exact_size(egui::vec2(340.0, 140.0), egui::Sense::hover());
            ui.painter().rect_filled(filters, 12.0f32, color::panel());
            ui.painter().rect_stroke(filters, 12.0f32, Stroke::new(1.0, color::border()), StrokeKind::Middle);
            ui.painter().text(egui::pos2(filters.left() + 20.0, filters.top() + 24.0), Align2::LEFT_CENTER, "FILTERS", FontId::new(10.0, FontFamily::Proportional), color::text2());
            ui.painter().text(egui::pos2(filters.left() + 20.0, filters.top() + 52.0), Align2::LEFT_CENTER, "Type: All · Images · Video · Audio", FontId::new(10.5, FontFamily::Proportional), color::text2());
            ui.painter().text(egui::pos2(filters.left() + 20.0, filters.top() + 76.0), Align2::LEFT_CENTER, "Role: All · Character · Identity · Costume", FontId::new(10.5, FontFamily::Proportional), color::text2());
            ui.painter().text(egui::pos2(filters.left() + 20.0, filters.top() + 100.0), Align2::LEFT_CENTER, "Project: Bamboo Grove", FontId::new(10.5, FontFamily::Proportional), color::text2());
        });

        // Asset library
        ui.add_space(14.0);
        ui.vertical(|ui| {
            // header
            let (header, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 42.0), egui::Sense::hover());
            ui.painter().rect_filled(header, 8.0f32, color::card());
            ui.painter().text(egui::pos2(header.left() + 18.0, header.center().y), Align2::LEFT_CENTER, "ASSET LIBRARY", FontId::new(10.5, FontFamily::Proportional), color::text());
            ui.painter().text(egui::pos2(header.right() - 18.0, header.center().y), Align2::RIGHT_CENTER, &format!("{} assets · sorted by project", n_assets), FontId::new(10.5, FontFamily::Proportional), color::text3());
            ui.add_space(12.0);

            // asset grid — 3 cols × 2 rows. Real server assets first; if the
            // server has none, fill the grid with mock cards so the layout is
            // always populated (matches the design mockup).
            let grid_w = (ui.available_width() - 2.0 * 12.0) / 3.0;
            let assets = app.assets.get("assets").and_then(|v| v.as_array()).cloned().unwrap_or_default();

            struct Card { key: String, name: String, role: String, accent: Color32, rel_path: String, audio: bool }
            let mut cards: Vec<Card> = Vec::new();
            for (i, asset) in assets.iter().take(6).enumerate() {
                let role = asset.get("asset_type").and_then(|t| t.as_str()).unwrap_or("image").to_string();
                cards.push(Card {
                    key: format!("asset-{}", i),
                    name: asset.get("name").and_then(|v| v.as_str()).unwrap_or("asset").to_string(),
                    role: role.clone(),
                    accent: accent_of_type(&role),
                    rel_path: asset.get("relative_path").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                    audio: false,
                });
            }
            if cards.is_empty() {
                let mock: [(&str, &str); 6] = [
                    ("bamboo_forest.png", "Environment"),
                    ("xiao_mei_front.png", "Character"),
                    ("rain_loop.mp4", "Motion ref"),
                    ("sword_pose.jpg", "Pose"),
                    ("ambience.wav", "Audio"),
                    ("costume_ref.png", "Costume"),
                ];
                for (idx, (name, role)) in mock.iter().enumerate() {
                    cards.push(Card {
                        key: format!("mock-{}", idx),
                        name: (*name).to_string(),
                        role: (*role).to_string(),
                        accent: if idx % 2 == 0 { color::teal() } else { color::indigo() },
                        rel_path: String::new(),
                        audio: role.contains("Audio"),
                    });
                }
            }

            // Kick off texture loads for real assets (outside the row borrow).
            for card in &cards {
                if !card.rel_path.is_empty() && app.textures.peek(&card.key).is_none() {
                    app.request_asset_bytes(&card.key, &card.rel_path);
                }
            }

            for row in cards.chunks(3) {
                ui.horizontal(|ui| {
                    for card in row {
                        let accent = card.accent;
                        let (rect, _) = ui.allocate_exact_size(egui::vec2(grid_w - 12.0, 138.0), egui::Sense::hover());
                        ui.painter().rect_filled(rect, 10.0f32, color::panel());
                        ui.painter().rect_stroke(rect, 10.0f32, Stroke::new(1.0, color::border()), StrokeKind::Middle);
                        ui.painter().rect_filled(Rect::from_min_size(rect.min, egui::vec2(rect.width(), 4.0)), 3.0f32, accent);
                        let thumb = Rect::from_min_size(egui::pos2(rect.left() + 10.0, rect.top() + 14.0), egui::vec2(rect.width() - 20.0, 74.0));
                        if let Some(tex) = app.textures.peek(&card.key) {
                            ui.painter().rect_filled(thumb, 6.0f32, color::thumb());
                            ui.painter().image(tex.id(), thumb, Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1.0, 1.0)), color::text());
                        } else if card.audio {
                            ui.painter().rect_filled(thumb, 6.0f32, color::well());
                            for k in 0..24 {
                                let bx = thumb.left() + 14.0 + (k as f32) * (thumb.width() - 28.0) / 24.0;
                                let bh = 8.0 + ((k * 37) % 21) as f32;
                                ui.painter().rect_filled(Rect::from_min_size(egui::pos2(bx, thumb.center().y - bh / 2.0), egui::vec2(3.5, bh)), 2.0f32, accent.linear_multiply(0.85));
                            }
                        } else {
                            crate::placeholder::silhouette(ui, thumb, accent);
                        }
                        ui.painter().text(egui::pos2(rect.left() + 14.0, rect.bottom() - 40.0), Align2::LEFT_CENTER, &truncate(&card.name, 26), FontId::new(11.0, FontFamily::Proportional), color::text());
                        ui.painter().text(egui::pos2(rect.left() + 14.0, rect.bottom() - 22.0), Align2::LEFT_CENTER, &card.role, FontId::new(9.5, FontFamily::Proportional), color::text2());
                        ui.add_space(12.0);
                    }
                });
                ui.add_space(12.0);
            }
        });
    });
}

fn accent_of_type(asset_type: &str) -> Color32 {
    match asset_type.to_lowercase().as_str() {
        "video" | "mp4" => color::teal(),
        "audio" => color::amber(),
        "pose" => color::indigo(),
        "costume" => color::purple(),
        _ => color::indigo(),
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() > max {
        let cut: String = s.chars().take(max - 1).collect();
        format!("{}…", cut)
    } else {
        s.to_string()
    }
}

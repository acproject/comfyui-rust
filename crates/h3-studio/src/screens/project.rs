//! Project workspace — project header, global settings, linked characters, scenes, production progress.

use egui::{Align2, Color32, FontId, FontFamily, Rect, Stroke, StrokeKind};
use crate::app::{H3App, Screen};
use crate::theme::{color, typography};
use crate::widgets;

pub fn draw(app: &mut H3App, ui: &mut egui::Ui) {
    let project = app.local.project().cloned();
    let Some(project) = project else {
        ui.centered_and_justified(|ui| {
            ui.colored_label(color::text2(), "No project selected");
        });
        return;
    };

    // Project hero
    let (hero, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 172.0), egui::Sense::hover());
    crate::placeholder::bamboo_scene(ui, hero, 7);
    ui.painter().rect_filled(hero, 12.0f32, Color32::from_rgba_premultiplied(0x0B, 0x12, 0x19, 128));
    ui.painter().rect_stroke(hero, 12.0f32, Stroke::new(1.0, color::border()), StrokeKind::Middle);
    ui.painter().text(egui::pos2(hero.left() + 26.0, hero.top() + 46.0), Align2::LEFT_CENTER, "PROJECT", FontId::new(10.5, FontFamily::Proportional), color::indigo());
    ui.painter().text(egui::pos2(hero.left() + 26.0, hero.top() + 78.0), Align2::LEFT_CENTER, &project.name, FontId::new(28.0, FontFamily::Proportional), color::text());
    ui.painter().text(egui::pos2(hero.left() + 26.0, hero.top() + 106.0), Align2::LEFT_CENTER, &project.description, FontId::new(12.5, FontFamily::Proportional), color::text2());
    let saved_rect = Rect::from_min_size(egui::pos2(hero.right() - 100.0, hero.top() + 24.0), egui::vec2(86.0, 24.0));
    ui.painter().rect_filled(saved_rect, 12.0f32, color::teal_dim());
    ui.painter().rect_stroke(saved_rect, 12.0f32, Stroke::new(1.0, color::teal().linear_multiply(0.5)), StrokeKind::Middle);
    ui.painter().text(saved_rect.center(), Align2::CENTER_CENTER, "Saved", FontId::new(10.5, FontFamily::Proportional), color::teal());

    ui.add_space(14.0);

    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            // Global visual settings
            let (gs, _) = ui.allocate_exact_size(egui::vec2(980.0, 182.0), egui::Sense::hover());
            ui.painter().rect_filled(gs, 12.0f32, color::panel());
            ui.painter().rect_stroke(gs, 12.0f32, Stroke::new(1.0, color::border()), StrokeKind::Middle);
            ui.painter().text(egui::pos2(gs.left() + 22.0, gs.top() + 24.0), Align2::LEFT_CENTER, "Global Visual Settings", FontId::new(13.0, FontFamily::Proportional), color::text());
            ui.painter().text(egui::pos2(gs.left() + 22.0, gs.top() + 44.0), Align2::LEFT_CENTER, "Project-wide defaults · individual shots can override", FontId::new(10.0, FontFamily::Proportional), color::text3());
            // aspect pills
            ui.painter().text(egui::pos2(gs.left() + 22.0, gs.top() + 70.0), Align2::LEFT_CENTER, "Aspect Ratio", FontId::new(11.0, FontFamily::Proportional), color::text2());
            for (i, aspect) in ["16:9", "2.39:1", "1:1"].iter().enumerate() {
                let pill_x = gs.left() + 132.0 + i as f32 * 118.0;
                let pill_rect = Rect::from_min_size(egui::pos2(pill_x, gs.top() + 52.0), egui::vec2(104.0, 28.0));
                let active = aspect == &project.aspect;
                ui.painter().rect_filled(pill_rect, 6.0f32, if active { color::indigo() } else { Color32::TRANSPARENT });
                ui.painter().rect_stroke(pill_rect, 6.0f32, Stroke::new(1.0, if active { color::indigo() } else { color::border() }), StrokeKind::Middle);
                ui.painter().text(pill_rect.center(), Align2::CENTER_CENTER, aspect, FontId::new(10.5, FontFamily::Proportional), if active { color::on_accent() } else { color::text() });
            }
            // FPS pills
            ui.painter().text(egui::pos2(gs.left() + 22.0, gs.top() + 104.0), Align2::LEFT_CENTER, "Default FPS", FontId::new(11.0, FontFamily::Proportional), color::text2());
            for (i, fps) in ["24", "25", "30", "60"].iter().enumerate() {
                let pill_x = gs.left() + 132.0 + i as f32 * 88.0;
                let pill_rect = Rect::from_min_size(egui::pos2(pill_x, gs.top() + 92.0), egui::vec2(80.0, 28.0));
                let active = fps.parse::<u32>().ok().unwrap() == project.fps;
                ui.painter().rect_filled(pill_rect, 6.0f32, if active { color::indigo() } else { Color32::TRANSPARENT });
                ui.painter().rect_stroke(pill_rect, 6.0f32, Stroke::new(1.0, if active { color::indigo() } else { color::border() }), StrokeKind::Middle);
                ui.painter().text(pill_rect.center(), Align2::CENTER_CENTER, fps, FontId::new(10.5, FontFamily::Proportional), if active { color::on_accent() } else { color::text() });
            }
            // resolution pills
            ui.painter().text(egui::pos2(gs.left() + 22.0, gs.top() + 140.0), Align2::LEFT_CENTER, "Resolution", FontId::new(11.0, FontFamily::Proportional), color::text2());
            for (i, res) in ["1920×1080", "2560×1440", "3840×2160"].iter().enumerate() {
                let pill_x = gs.left() + 132.0 + i as f32 * 138.0;
                let pill_rect = Rect::from_min_size(egui::pos2(pill_x, gs.top() + 124.0), egui::vec2(130.0, 28.0));
                let active = res == &project.resolution;
                ui.painter().rect_filled(pill_rect, 6.0f32, if active { color::indigo() } else { Color32::TRANSPARENT });
                ui.painter().rect_stroke(pill_rect, 6.0f32, Stroke::new(1.0, if active { color::indigo() } else { color::border() }), StrokeKind::Middle);
                ui.painter().text(pill_rect.center(), Align2::CENTER_CENTER, res, FontId::new(10.0, FontFamily::Proportional), if active { color::on_accent() } else { color::text() });
            }
            ui.add_space(196.0);
        });

        // Scenes + production progress
        ui.add_space(16.0);
        ui.vertical(|ui| {
            let (scenes, _) = ui.allocate_exact_size(egui::vec2(540.0, 158.0), egui::Sense::hover());
            ui.painter().rect_filled(scenes, 12.0f32, color::panel());
            ui.painter().rect_stroke(scenes, 12.0f32, Stroke::new(1.0, color::border()), StrokeKind::Middle);
            ui.painter().text(egui::pos2(scenes.left() + 18.0, scenes.top() + 22.0), Align2::LEFT_CENTER, "Scenes", FontId::new(13.0, FontFamily::Proportional), color::text());
            ui.painter().text(egui::pos2(scenes.left() + 18.0, scenes.top() + 40.0), Align2::LEFT_CENTER, "Storyboard order · drag to reorder", FontId::new(10.0, FontFamily::Proportional), color::text3());
            for (i, scene) in project.scenes.iter().take(4).enumerate() {
                let row_y = scenes.top() + 50.0 + i as f32 * 40.0;
                let active = i == 0;
                ui.painter().rect_filled(Rect::from_min_size(egui::pos2(scenes.left() + 12.0, row_y), egui::vec2(6.0, 30.0)), 3.0f32, if active { color::indigo() } else { color::text3() });
                ui.painter().text(egui::pos2(scenes.left() + 30.0, row_y + 10.0), Align2::LEFT_CENTER, &format!("Scene {:02}", i + 1), FontId::new(11.5, FontFamily::Proportional), color::text());
                ui.painter().text(egui::pos2(scenes.left() + 30.0, row_y + 25.0), Align2::LEFT_CENTER, &scene.name, FontId::new(10.0, FontFamily::Proportional), color::text2());
                ui.painter().text(egui::pos2(scenes.right() - 16.0, row_y + 15.0), Align2::RIGHT_CENTER, &format!("{} shots", scene.shots.len()), FontId::new(10.5, FontFamily::Proportional), color::text2());
            }
            ui.add_space(168.0);
            // Production progress
            let (prog, _) = ui.allocate_exact_size(egui::vec2(540.0, 148.0), egui::Sense::hover());
            ui.painter().rect_filled(prog, 12.0f32, color::panel());
            ui.painter().rect_stroke(prog, 12.0f32, Stroke::new(1.0, color::border()), StrokeKind::Middle);
            ui.painter().text(egui::pos2(prog.left() + 18.0, prog.top() + 24.0), Align2::LEFT_CENTER, "Production Progress", FontId::new(13.0, FontFamily::Proportional), color::text());
            let all_shots: Vec<_> = project.scenes.iter().flat_map(|s| &s.shots).collect();
            let done = all_shots.iter().filter(|s| s.status == crate::state::ShotStatus::Done).count();
            let frac = if all_shots.is_empty() { 0.0 } else { done as f32 / all_shots.len() as f32 };
            ui.painter().text(egui::pos2(prog.left() + 18.0, prog.top() + 62.0), Align2::LEFT_CENTER, &format!("{}", done), FontId::new(38.0, FontFamily::Proportional), color::text());
            ui.painter().text(egui::pos2(prog.left() + 18.0 + 86.0, prog.top() + 66.0), Align2::LEFT_CENTER, &format!(" / {} shots", all_shots.len()), FontId::new(17.0, FontFamily::Proportional), color::text3());
            let bar_rect = Rect::from_min_size(egui::pos2(prog.left() + 18.0, prog.top() + 86.0), egui::vec2(508.0, 10.0));
            widgets::progress_bar(ui, bar_rect, frac, color::indigo(), 10.0);
            ui.painter().text(egui::pos2(prog.left() + 18.0, prog.bottom() - 20.0), Align2::LEFT_CENTER, &format!("{:.0}% generated · estimated remaining 00:42:18", frac * 100.0), FontId::new(10.5, FontFamily::Proportional), color::text2());
        });
    });

    // Character library strip
    ui.add_space(14.0);
    let (strip, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 152.0), egui::Sense::hover());
    ui.painter().rect_filled(strip, 12.0f32, color::panel());
    ui.painter().rect_stroke(strip, 12.0f32, Stroke::new(1.0, color::border()), StrokeKind::Middle);
    ui.painter().text(egui::pos2(strip.left() + 20.0, strip.top() + 22.0), Align2::LEFT_CENTER, "Character Library", FontId::new(13.0, FontFamily::Proportional), color::text());
    ui.painter().text(egui::pos2(strip.right() - 20.0, strip.top() + 22.0), Align2::RIGHT_CENTER, "Reusable across projects · 4 characters linked", FontId::new(10.0, FontFamily::Proportional), color::text3());
    for (i, c) in app.local.characters.iter().enumerate() {
        let card_x = strip.left() + 20.0 + i as f32 * 248.0;
        let card_rect = Rect::from_min_size(egui::pos2(card_x, strip.top() + 36.0), egui::vec2(236.0, 98.0));
        ui.painter().rect_filled(card_rect, 10.0f32, color::card());
        ui.painter().rect_stroke(card_rect, 10.0f32, Stroke::new(1.0, color::border()), StrokeKind::Middle);
        let accent = char_accent(&c.accent);
        ui.painter().rect_filled(Rect::from_min_size(card_rect.min, egui::vec2(4.0, card_rect.height())), 0.0f32, accent);
        ui.painter().circle_filled(egui::pos2(card_rect.left() + 34.0, card_rect.top() + 32.0), 15.0, color::thumb());
        ui.painter().circle_stroke(egui::pos2(card_rect.left() + 34.0, card_rect.top() + 32.0), 15.0, Stroke::new(1.6, accent));
        ui.painter().text(egui::pos2(card_rect.left() + 58.0, card_rect.top() + 26.0), Align2::LEFT_CENTER, &c.name, FontId::new(12.5, FontFamily::Proportional), color::text());
        ui.painter().text(egui::pos2(card_rect.left() + 58.0, card_rect.top() + 46.0), Align2::LEFT_CENTER, &format!("{} · {} refs", c.role, c.refs), FontId::new(10.0, FontFamily::Proportional), color::text2());
        ui.painter().text(egui::pos2(card_rect.left() + 58.0, card_rect.top() + 66.0), Align2::LEFT_CENTER, &format!("consistency {:.0}% · {}", c.consistency, c.updated_at), FontId::new(9.5, FontFamily::Proportional), color::text3());
    }
}

fn char_accent(accent: &str) -> Color32 {
    match accent {
        "teal" => color::teal(),
        "indigo" => color::indigo(),
        "amber" => color::amber(),
        "purple" => color::purple(),
        _ => color::text2(),
    }
}

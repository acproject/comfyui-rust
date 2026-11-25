//! Home dashboard — local project library + new project form + generation activity.

use egui::{Align2, Color32, FontId, FontFamily, Rect, Stroke, StrokeKind};
use crate::app::{H3App, Screen};
use crate::theme::{color, metrics, typography};
use crate::widgets;

pub fn draw(app: &mut H3App, ui: &mut egui::Ui) {

    // Section header
    ui.horizontal(|ui| {
        ui.colored_label(color::text3(), egui::RichText::new("LOCAL PROJECT LIBRARY · SORTED BY LAST MODIFIED").size(10.5));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.colored_label(color::text3(), "k%").on_hover_text("⌘K command palette — coming soon");
        });
    });
    ui.add_space(6.0);

    ui.horizontal(|ui| {
        let avail = ui.available_width();
        let gap = 18.0;
        let right_w = 440.0f32.min(avail * 0.38).max(340.0);
        let left_w = (avail - right_w - gap).max(420.0);
        ui.vertical(|ui| {
            ui.set_max_width(left_w);
            // Project cards
            ui.horizontal(|ui| {
                ui.colored_label(color::text(), egui::RichText::new("Recent Projects").size(15.0).strong());
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.colored_label(color::text3(), "2 projects");
                });
            });
            ui.add_space(10.0);

            let mut clicked_project = None;
            for proj in &app.local.projects {
                let shots_total = proj.scenes.iter().flat_map(|s| &s.shots).count();
                let done = proj.scenes.iter().flat_map(|s| &s.shots).filter(|s| s.status == crate::state::ShotStatus::Done).count();
                let frac = if shots_total == 0 { 0.0 } else { done as f32 / shots_total as f32 };
                let project_name = proj.name.clone();
                let (rect, response) = ui.allocate_exact_size(egui::vec2(left_w, 132.0), egui::Sense::click());
                ui.painter().rect_filled(rect, 12.0f32, color::panel());
                ui.painter().rect_stroke(rect, 12.0f32, Stroke::new(1.0, color::border()), StrokeKind::Middle);
                ui.painter().rect_filled(
                    Rect::from_min_size(rect.min, egui::vec2(4.0, rect.height())),
                    2.0f32,
                    if proj.id == "bamboo-grove" { color::indigo() } else { color::teal() },
                );
                ui.painter().text(egui::pos2(rect.left() + 24.0, rect.top() + 26.0), Align2::LEFT_CENTER, &proj.name, FontId::new(17.5, FontFamily::Proportional), color::text());
                ui.painter().text(egui::pos2(rect.left() + 24.0, rect.top() + 48.0), Align2::LEFT_CENTER, &proj.description, FontId::new(11.5, FontFamily::Proportional), color::text2());
                ui.painter().text(egui::pos2(rect.left() + 24.0, rect.top() + 70.0), Align2::LEFT_CENTER, &format!("Last modified {} · {} scenes · {} shots · {} generated", proj.updated_at, proj.scenes.len(), shots_total, done), FontId::new(10.0, FontFamily::Proportional), color::text3());
                let bar_rect = Rect::from_min_size(egui::pos2(rect.left() + 24.0, rect.top() + 88.0), egui::vec2(left_w - 200.0, 9.0));
                widgets::progress_bar(ui, bar_rect, frac, if proj.id == "bamboo-grove" { color::indigo() } else { color::teal() }, 9.0);
                ui.painter().text(egui::pos2(rect.left() + 24.0, rect.top() + 112.0), Align2::LEFT_CENTER, &format!("Generation: {:.0}%", frac * 100.0), FontId::new(10.5, FontFamily::Proportional), color::text2());
                let chip_txt = if proj.id == "bamboo-grove" { "FL2VA" } else { "Ref2VA" };
                let chip_tone = if proj.id == "bamboo-grove" { widgets::Tone::Indigo } else { widgets::Tone::Teal };
                let accent = chip_tone.accent();
                let chip_rect = Rect::from_min_size(egui::pos2(rect.right() - 104.0, rect.top() + 50.0), egui::vec2(86.0, 24.0));
                ui.painter().rect_filled(chip_rect, 12.0f32, chip_tone.dim());
                ui.painter().rect_stroke(chip_rect, 12.0f32, Stroke::new(1.0, accent.linear_multiply(0.5)), StrokeKind::Middle);
                ui.painter().text(chip_rect.center(), Align2::CENTER_CENTER, chip_txt, FontId::new(10.5, FontFamily::Proportional), accent);
                if response.hovered() {
                    ui.painter().rect_stroke(rect, 12.0f32, Stroke::new(1.4, color::border().linear_multiply(1.4)), StrokeKind::Middle);
                }
                if response.clicked() {
                    clicked_project = Some(project_name);
                }
                ui.add_space(10.0);
            }
            if let Some(name) = clicked_project {
                app.local.selected_project_id = Some(name.to_lowercase().replace(' ', "-"));
                app.local.save();
                app.nav_to(Screen::Projects);
            }
        });

        // New project form (right column)
        ui.add_space(gap);
        ui.vertical(|ui| {
            ui.set_max_width(right_w);
            let (rect, _) = ui.allocate_exact_size(egui::vec2(right_w, 340.0), egui::Sense::hover());
            ui.painter().rect_filled(rect, 12.0f32, color::panel());
            ui.painter().rect_stroke(rect, 12.0f32, Stroke::new(1.0, color::border()), StrokeKind::Middle);
            ui.painter().text(egui::pos2(rect.left() + 22.0, rect.top() + 32.0), Align2::LEFT_CENTER, "New Project", FontId::new(13.5, FontFamily::Proportional), color::text());
            // name input
            ui.painter().text(egui::pos2(rect.left() + 22.0, rect.top() + 62.0), Align2::LEFT_CENTER, "NAME", FontId::new(9.5, FontFamily::Proportional), color::text2());
            let input_rect = Rect::from_min_size(egui::pos2(rect.left() + 22.0, rect.top() + 70.0), egui::vec2(right_w - 44.0, 34.0));
            ui.painter().rect_filled(input_rect, 7.0f32, color::bg());
            ui.painter().rect_stroke(input_rect, 7.0f32, Stroke::new(1.0, color::border()), StrokeKind::Middle);
            ui.painter().text(egui::pos2(input_rect.left() + 10.0, input_rect.center().y), Align2::LEFT_CENTER, "Bamboo Grove · 02", FontId::new(12.0, FontFamily::Proportional), color::text2());
            // aspect
            ui.painter().text(egui::pos2(rect.left() + 22.0, rect.top() + 122.0), Align2::LEFT_CENTER, "ASPECT RATIO", FontId::new(9.5, FontFamily::Proportional), color::text2());
            let input_rect2 = Rect::from_min_size(egui::pos2(rect.left() + 22.0, rect.top() + 130.0), egui::vec2(right_w - 44.0, 34.0));
            ui.painter().rect_filled(input_rect2, 7.0f32, color::bg());
            ui.painter().rect_stroke(input_rect2, 7.0f32, Stroke::new(1.0, color::border()), StrokeKind::Middle);
            ui.painter().text(egui::pos2(input_rect2.left() + 10.0, input_rect2.center().y), Align2::LEFT_CENTER, "16:9", FontId::new(12.0, FontFamily::Proportional), color::text2());
            // style
            ui.painter().text(egui::pos2(rect.left() + 22.0, rect.top() + 182.0), Align2::LEFT_CENTER, "STYLE PRESET", FontId::new(9.5, FontFamily::Proportional), color::text2());
            let input_rect3 = Rect::from_min_size(egui::pos2(rect.left() + 22.0, rect.top() + 190.0), egui::vec2(right_w - 44.0, 34.0));
            ui.painter().rect_filled(input_rect3, 7.0f32, color::bg());
            ui.painter().rect_stroke(input_rect3, 7.0f32, Stroke::new(1.0, color::border()), StrokeKind::Middle);
            ui.painter().text(egui::pos2(input_rect3.left() + 10.0, input_rect3.center().y), Align2::LEFT_CENTER, "Cinematic Wuxia", FontId::new(12.0, FontFamily::Proportional), color::text2());
            // create button
            let btn_rect = Rect::from_min_size(egui::pos2(rect.left() + 22.0, rect.top() + 240.0), egui::vec2(right_w - 44.0, 40.0));
            ui.painter().rect_filled(btn_rect, 8.0f32, color::indigo());
            ui.painter().text(btn_rect.center(), Align2::CENTER_CENTER, "Create Project", FontId::new(12.5, FontFamily::Proportional), color::on_accent());
        });
    });

    ui.add_space(16.0);

    // Generation activity
    let (act, _resp) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 118.0), egui::Sense::hover());
    ui.painter().rect_filled(act, 12.0f32, color::panel());
    ui.painter().rect_stroke(act, 12.0f32, Stroke::new(1.0, color::border()), StrokeKind::Middle);
    ui.painter().text(egui::pos2(act.left() + 22.0, act.top() + 24.0), Align2::LEFT_CENTER, "Generation Activity", FontId::new(12.5, FontFamily::Proportional), color::text());
    let chip_rect = Rect::from_min_size(egui::pos2(act.left() + 22.0, act.top() + 40.0), egui::vec2(250.0, 26.0));
    ui.painter().rect_filled(chip_rect, 13.0f32, color::indigo_dim());
    ui.painter().rect_stroke(chip_rect, 13.0f32, Stroke::new(1.0, color::indigo().linear_multiply(0.45)), StrokeKind::Middle);
    ui.painter().text(chip_rect.center(), Align2::CENTER_CENTER, "Running · Bamboo Grove · Shot 003", FontId::new(10.5, FontFamily::Proportional), color::indigo());

    let n_gpus = app.gpus.len();
    let gpus_str = if n_gpus == 0 { "GPU —".to_string() } else {
        (0..n_gpus).map(|i| format!("GPU {}  {:.0}%", i, app.gpus[i].util_percent)).collect::<Vec<_>>().join("   ")
    };
    let gpus_line = if n_gpus == 0 {
        "GPU — · VRAM — · RAM —".to_string()
    } else {
        let total_vram: f64 = app.gpus.iter().map(|g| g.vram_used_mb).sum();
        let total_denom: f64 = app.gpus.iter().map(|g| g.vram_total_mb).sum();
        let ram_u = format!("RAM {:.1} / {:.1} GB", app.host.ram_used_gb, app.host.ram_total_gb);
        format!("{}   VRAM {:.1} / {:.1} GB   {}", gpus_str, total_vram / 1024.0, total_denom / 1024.0, ram_u)
    };
    ui.painter().text(egui::pos2(act.left() + 22.0, act.top() + 76.0), Align2::LEFT_CENTER, &gpus_line, FontId::new(10.5, FontFamily::Proportional), color::text2());
    ui.painter().text(egui::pos2(act.right() - 22.0, act.top() + 76.0), Align2::RIGHT_CENTER, "Rendering Shot 003 · FL2VA · GPU 0 →", FontId::new(10.5, FontFamily::Proportional), color::indigo());
}

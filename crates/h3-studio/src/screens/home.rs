//! Home dashboard — local project library + new project form + generation activity.

use egui::{Align2, Color32, FontId, FontFamily, Rect, Sense, Stroke, StrokeKind};
use crate::app::{H3App, Screen};
use crate::state::{ProjectRecord, Scene, Shot, ShotStatus};
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
            // name input — real editable text overlaid on the painted box
            ui.painter().text(egui::pos2(rect.left() + 22.0, rect.top() + 62.0), Align2::LEFT_CENTER, "NAME", FontId::new(9.5, FontFamily::Proportional), color::text2());
            let input_rect = Rect::from_min_size(egui::pos2(rect.left() + 22.0, rect.top() + 70.0), egui::vec2(right_w - 44.0, 34.0));
            ui.painter().rect_filled(input_rect, 7.0f32, color::bg());
            ui.painter().rect_stroke(input_rect, 7.0f32, Stroke::new(1.0, color::border()), StrokeKind::Middle);
            let _ = ui.allocate_ui_at_rect(input_rect, |ui| {
                ui.add(
                    egui::TextEdit::singleline(&mut app.ui.new_project_name)
                        .hint_text("Bamboo Grove · 02")
                        .font(egui::TextStyle::Body)
                        .text_color(color::text())
                        .background_color(Color32::TRANSPARENT)
                        .frame(false)
                        .margin(egui::Margin::symmetric(10, 9)),
                );
            });
            // aspect ratio — click to cycle
            ui.painter().text(egui::pos2(rect.left() + 22.0, rect.top() + 122.0), Align2::LEFT_CENTER, "ASPECT RATIO", FontId::new(9.5, FontFamily::Proportional), color::text2());
            let input_rect2 = Rect::from_min_size(egui::pos2(rect.left() + 22.0, rect.top() + 130.0), egui::vec2(right_w - 44.0, 34.0));
            let aspect_resp = ui.interact(input_rect2, ui.id().with("np-aspect"), Sense::click());
            ui.painter().rect_filled(input_rect2, 7.0f32, if aspect_resp.hovered() { color::card() } else { color::bg() });
            ui.painter().rect_stroke(input_rect2, 7.0f32, Stroke::new(1.0, if aspect_resp.hovered() { color::indigo() } else { color::border() }), StrokeKind::Middle);
            ui.painter().text(egui::pos2(input_rect2.left() + 10.0, input_rect2.center().y), Align2::LEFT_CENTER, &app.ui.new_project_aspect, FontId::new(12.0, FontFamily::Proportional), color::text());
            ui.painter().text(egui::pos2(input_rect2.right() - 12.0, input_rect2.center().y), Align2::RIGHT_CENTER, "▾", FontId::new(11.0, FontFamily::Proportional), color::text3());
            if aspect_resp.clicked() {
                let aspects = ["16:9", "2.39:1", "1:1", "9:16"];
                let cur = aspects.iter().position(|a| a == &app.ui.new_project_aspect).unwrap_or(0);
                app.ui.new_project_aspect = aspects[(cur + 1) % aspects.len()].to_string();
            }
            // style preset — click to cycle
            ui.painter().text(egui::pos2(rect.left() + 22.0, rect.top() + 182.0), Align2::LEFT_CENTER, "STYLE PRESET", FontId::new(9.5, FontFamily::Proportional), color::text2());
            let input_rect3 = Rect::from_min_size(egui::pos2(rect.left() + 22.0, rect.top() + 190.0), egui::vec2(right_w - 44.0, 34.0));
            let style_resp = ui.interact(input_rect3, ui.id().with("np-style"), Sense::click());
            ui.painter().rect_filled(input_rect3, 7.0f32, if style_resp.hovered() { color::card() } else { color::bg() });
            ui.painter().rect_stroke(input_rect3, 7.0f32, Stroke::new(1.0, if style_resp.hovered() { color::indigo() } else { color::border() }), StrokeKind::Middle);
            ui.painter().text(egui::pos2(input_rect3.left() + 10.0, input_rect3.center().y), Align2::LEFT_CENTER, &app.ui.new_project_style, FontId::new(12.0, FontFamily::Proportional), color::text());
            ui.painter().text(egui::pos2(input_rect3.right() - 12.0, input_rect3.center().y), Align2::RIGHT_CENTER, "▾", FontId::new(11.0, FontFamily::Proportional), color::text3());
            if style_resp.clicked() {
                let presets = ["Cinematic Wuxia", "Documentary", "Studio Portrait", "Anime Cel", "Noir"];
                let cur = presets.iter().position(|a| a == &app.ui.new_project_style).unwrap_or(0);
                app.ui.new_project_style = presets[(cur + 1) % presets.len()].to_string();
            }
            // create button
            let btn_rect = Rect::from_min_size(egui::pos2(rect.left() + 22.0, rect.top() + 240.0), egui::vec2(right_w - 44.0, 40.0));
            let btn_resp = ui.interact(btn_rect, ui.id().with("np-create"), Sense::click());
            let btn_col = if btn_resp.hovered() { color::indigo().linear_multiply(1.1) } else { color::indigo() };
            ui.painter().rect_filled(btn_rect, 8.0f32, btn_col);
            ui.painter().rect_stroke(btn_rect, 8.0f32, Stroke::new(1.0, btn_col.linear_multiply(0.9)), StrokeKind::Middle);
            ui.painter().text(btn_rect.center(), Align2::CENTER_CENTER, "Create Project", FontId::new(12.5, FontFamily::Proportional), color::on_accent());
            if btn_resp.clicked() {
                let name = if app.ui.new_project_name.trim().is_empty() { "Untitled Project".to_string() } else { app.ui.new_project_name.trim().to_string() };
                let id = name.to_lowercase().replace(|c: char| !c.is_alphanumeric(), "-");
                let proj = ProjectRecord {
                    id: id.clone(),
                    name: name.clone(),
                    genre: app.ui.new_project_style.clone(),
                    description: format!("{} · {} · new project", app.ui.new_project_style, app.ui.new_project_aspect),
                    aspect: app.ui.new_project_aspect.clone(),
                    fps: 24,
                    resolution: "1920×1080".into(),
                    updated_at: "just now".into(),
                    scenes: vec![Scene {
                        id: "scene-01".into(),
                        name: "Scene 01".into(),
                        shots: (1..=4)
                            .map(|i| Shot {
                                id: format!("{}-01-{:02}", id, i),
                                number: i,
                                title: format!("Shot {:02}", i),
                                mode: "FL2VA".into(),
                                duration: 4,
                                status: ShotStatus::Pending,
                            })
                            .collect(),
                    }],
                };
                if !app.local.projects.iter().any(|p| p.id == id) {
                    app.local.projects.push(proj);
                }
                app.local.selected_project_id = Some(id);
                app.local.save();
                app.ui.new_project_name.clear();
                app.set_toast(format!("Created project · {}", name));
                app.nav_to(Screen::Projects);
            }
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

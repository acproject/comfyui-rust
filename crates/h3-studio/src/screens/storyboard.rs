//! Storyboard editor — shot timeline strip, 4×2 shot card grid, shot tools.

use egui::{Align2, Color32, FontId, FontFamily, Rect, Sense, Stroke, StrokeKind};
use crate::app::{H3App, Screen};
use crate::theme::{color, typography};
use crate::widgets;

/// Draw one shot card into the current cell. Returns (clicked, generate_clicked).
fn draw_shot_card(
    app: &mut H3App,
    ui: &mut egui::Ui,
    shot: Option<&crate::state::Shot>,
    seed: u32,
    card_w: f32,
) -> (bool, bool) {
    let (rect, outer) = ui.allocate_exact_size(egui::vec2(card_w, 156.0), egui::Sense::click());

    let selected = shot.map(|s| app.local.selected_shot_id.as_deref() == Some(s.id.as_str())).unwrap_or(false);

    ui.painter().rect_filled(rect, 12.0f32, color::panel());
    ui.painter().rect_stroke(
        rect, 12.0f32,
        Stroke::new(if selected { 1.6 } else { 1.0 }, if selected { color::indigo() } else { color::border() }),
        StrokeKind::Middle,
    );
    if selected {
        ui.painter().rect_filled(Rect::from_min_size(rect.min, egui::vec2(rect.width(), 3.5)), 3.0f32, color::indigo());
    }

    let mut clicked = false;
    let mut generate = false;

    if let Some(shot) = shot {
        let tone = status_tone(shot.status);
        let well = Rect::from_min_size(egui::pos2(rect.left() + 12.0, rect.top() + 12.0), egui::vec2(rect.width() - 24.0, 56.0));
        crate::placeholder::bamboo_scene(ui, well, 3 + seed);
        ui.painter().rect_stroke(well, 8.0f32, Stroke::new(1.0, color::border()), StrokeKind::Middle);

        ui.painter().text(
            egui::pos2(well.left() + 12.0, well.top() + 15.0),
            Align2::LEFT_CENTER,
            &format!("SHOT {:02}", shot.number),
            FontId::new(9.5, FontFamily::Proportional),
            color::text(),
        );
        let chip_rect = Rect::from_min_size(egui::pos2(well.right() - 62.0, well.top() + 10.0), egui::vec2(52.0, 20.0));
        ui.painter().rect_filled(chip_rect, 10.0f32, tone.dim());
        ui.painter().text(chip_rect.center(), Align2::CENTER_CENTER, status_label(shot.status), FontId::new(9.5, FontFamily::Proportional), tone.accent());

        ui.painter().text(egui::pos2(rect.left() + 16.0, rect.top() + 78.0), Align2::LEFT_CENTER, &shot.title, FontId::new(12.5, FontFamily::Proportional), color::text());
        ui.painter().text(egui::pos2(rect.left() + 16.0, rect.top() + 97.0), Align2::LEFT_CENTER, &format!("{} · {}s", shot.mode, shot.duration), FontId::new(10.5, FontFamily::Proportional), color::text2());

        let btn_rect = Rect::from_min_size(egui::pos2(rect.right() - 86.0, rect.bottom() - 28.0), egui::vec2(70.0, 22.0));
        let btn_resp = ui.interact(btn_rect, ui.id().with(("shot-gen", shot.id.clone())), egui::Sense::click());
        ui.painter().rect_filled(btn_rect, 6.0f32, color::card());
        ui.painter().rect_stroke(btn_rect, 6.0f32, Stroke::new(1.0, color::border()), StrokeKind::Middle);
        ui.painter().text(btn_rect.center(), Align2::CENTER_CENTER, "Generate", FontId::new(9.5, FontFamily::Proportional), color::text());
        generate = btn_resp.clicked();

        clicked = outer.clicked() && !generate;
    }

    let _ = outer;
    (clicked, generate)
}

pub fn draw(app: &mut H3App, ui: &mut egui::Ui) {
    let project = app.local.project().cloned();
    let scene = project.as_ref().and_then(|p| p.scenes.first().cloned());
    let (Some(project), Some(scene)) = (project, scene) else {
        return;
    };
    let selected_shot_id = app.local.selected_shot_id.clone();

    // Timeline strip
    let (tl, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 92.0), egui::Sense::hover());
    ui.painter().rect_filled(tl, 10.0f32, color::panel());
    ui.painter().rect_stroke(tl, 10.0f32, Stroke::new(1.0, color::border()), StrokeKind::Middle);
    ui.painter().text(egui::pos2(tl.left() + 20.0, tl.top() + 24.0), Align2::LEFT_CENTER, "SHOT TIMELINE · SCENE 01", FontId::new(10.0, FontFamily::Proportional), color::text2());
    let total_width = tl.right() - tl.left() - 40.0;
    let chip_w = (total_width / 12.0) - 10.0;
    let mut timeline_pick = None;
    for (i, shot) in scene.shots.iter().take(12).enumerate() {
        let x = tl.left() + 20.0 + i as f32 * (chip_w + 10.0);
        let chip_rect = Rect::from_min_size(egui::pos2(x, tl.top() + 38.0), egui::vec2(chip_w, 40.0));
        let is_selected = Some(shot.id.as_str()) == selected_shot_id.as_deref();
        let chip_resp = ui.interact(chip_rect, ui.id().with(("tl-chip", i)), Sense::click());
        let hover = chip_resp.hovered() && !is_selected;
        ui.painter().rect_filled(chip_rect, 8.0f32, if is_selected || hover { color::card() } else { color::panel() });
        let chip_stroke = if is_selected { Stroke::new(1.6, color::indigo()) } else { Stroke::new(if hover { 1.3 } else { 1.0 }, if hover { color::indigo() } else { color::border() }) };
        ui.painter().rect_stroke(chip_rect, 8.0f32, chip_stroke, StrokeKind::Middle);
        let num = format!("{:02}", shot.number);
        let num_col = if is_selected { color::indigo() } else { color::text() };
        ui.painter().text(egui::pos2(chip_rect.center().x, chip_rect.top() + 12.0), Align2::CENTER_CENTER, &num, FontId::new(12.0, FontFamily::Monospace), num_col);
        ui.painter().text(egui::pos2(chip_rect.center().x, chip_rect.bottom() - 11.0), Align2::CENTER_CENTER, &format!("{}s", shot.duration), FontId::new(9.5, FontFamily::Monospace), color::text3());
        if chip_resp.clicked() { timeline_pick = Some(shot.id.clone()); }
    }
    if let Some(id) = timeline_pick {
        app.local.selected_shot_id = Some(id.clone());
        app.local.save();
        app.set_toast(format!("Selected shot {:?}", id));
    }

    ui.add_space(12.0);

    // 4×2 grid — each row drawn as a horizontal flow
    let mut clicked_id = None;
    let mut generate_id = None;

    let col_w = (ui.available_width() - 3.0 * 12.0) / 4.0;
    for row in 0..2 {
        ui.horizontal(|ui| {
            for col in 0..4 {
                let idx = row * 4 + col;
                let shot = scene.shots.get(idx);
                let (clicked, generated) = draw_shot_card(app, ui, shot, 3 + idx as u32, col_w);
                if let Some(s) = shot {
                    if clicked { clicked_id = Some(s.id.clone()); }
                    if generated { generate_id = Some(s.id.clone()); }
                }
                ui.add_space(12.0);
            }
        });
        ui.add_space(12.0);
    }

    if let Some(id) = clicked_id {
        app.local.selected_shot_id = Some(id);
        app.local.save();
    }
    if generate_id.is_some() {
        app.set_toast("Shot queued for generation (demo)");
        app.nav_to(Screen::Queue);
    }
}

fn status_tone(status: crate::state::ShotStatus) -> widgets::Tone {
    match status {
        crate::state::ShotStatus::Done => widgets::Tone::Teal,
        crate::state::ShotStatus::Running => widgets::Tone::Indigo,
        crate::state::ShotStatus::Failed => widgets::Tone::Red,
        _ => widgets::Tone::Gray,
    }
}

fn status_label(status: crate::state::ShotStatus) -> &'static str {
    match status {
        crate::state::ShotStatus::Done => "Done",
        crate::state::ShotStatus::Running => "Running",
        crate::state::ShotStatus::Queued => "Queued",
        crate::state::ShotStatus::Pending => "Pending",
        crate::state::ShotStatus::Failed => "Failed",
    }
}

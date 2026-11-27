//! Storyboard editor — shot timeline strip, 4×2 shot card grid, shot inspector.
//!
//! Each shot card drives a *real* generation job: clicking Generate builds the
//! FL2VA / Ref2VA workflow from the shot's prompt + reference image, submits it
//! to the server, and the shot's status / progress / output are then driven by
//! the live `/queue` + `/history` snapshots and the WebSocket progress feed.

use egui::{Align2, Color32, FontId, FontFamily, Rect, Sense, Stroke, StrokeKind};
use crate::app::{H3App, Screen};
use crate::theme::color;
use crate::widgets;
use crate::workflows::{fl2va_workflow, ref2va_workflow, Fl2VaParams, Ref2VaParams};

/// Mirrors the Generation Studio's default Ref2VA VAE path.
const VAE_DIR: &str = "/home/acproject/usb/comfyui/models/MiniMax-H3/Ref2VA/video_vae";

/// Build the video workflow graph for a shot from its editable prompt + ref image.
fn build_shot_graph(app: &H3App, shot: &crate::state::Shot) -> serde_json::Value {
    let input_image = shot
        .reference_image
        .clone()
        .or_else(|| app.input_images.first().cloned())
        .unwrap_or_else(|| "OK_033_00000.png".to_string());
    let prompt = if shot.prompt.trim().is_empty() {
        default_prompt(&shot.mode)
    } else {
        shot.prompt.clone()
    };
    let fps = 24;
    let num_frames = (shot.duration * fps).max(24);
    let prefix = format!("h3_{}_{:02}", shot.mode.to_lowercase(), shot.number);
    match shot.mode.as_str() {
        "Ref2VA" => ref2va_workflow(&Ref2VaParams {
            image: input_image,
            view_descriptions: ["Front".into(), "Back".into(), "Left".into(), "Right".into()],
            vae_dir: VAE_DIR.into(),
            prompt,
            embed_frames: true,
            fps,
            height: 480,
            width: 832,
            num_frames,
            seed: 42,
            split_gpu: true,
            steps: 30,
            strength: 1.0,
            prefix,
        }),
        _ => fl2va_workflow(&Fl2VaParams {
            first_frame: input_image.clone(),
            last_frame: input_image,
            prompt,
            fps,
            height: 480,
            width: 832,
            num_frames,
            seed: 42,
            steps: 30,
            embed_frames: true,
            prefix,
        }),
    }
}

fn default_prompt(mode: &str) -> String {
    match mode {
        "Ref2VA" => "character turns 360 degrees, consistent features, cinematic lighting".into(),
        _ => "cinematic, smooth motion, shallow depth of field, natural lighting".into(),
    }
}

/// Real progress ratio for a shot (0..1), or an indeterminate pulse while Running.
fn shot_progress(app: &H3App, ui: &egui::Ui, shot: &crate::state::Shot) -> f32 {
    if let Some(pid) = shot.prompt_id.as_deref() {
        if let Some(p) = app.ws_progress.get(pid) {
            if p.max > 0.0 {
                return (p.value / p.max).clamp(0.0, 1.0) as f32;
            }
        }
    }
    if shot.status == crate::state::ShotStatus::Running {
        let t = ui.input(|i| i.time);
        return (0.25 + 0.25 * (t * 2.0).sin()) as f32;
    }
    0.0
}

/// Draw one shot card into the current cell. Returns (clicked, generate_clicked).
fn draw_shot_card(
    app: &mut H3App,
    ui: &mut egui::Ui,
    shot: Option<&crate::state::Shot>,
    seed: u32,
    card_w: f32,
) -> (bool, bool) {
    let (rect, outer) = ui.allocate_exact_size(egui::vec2(card_w, 160.0), egui::Sense::click());

    let selected = shot
        .map(|s| app.local.selected_shot_id.as_deref() == Some(s.id.as_str()))
        .unwrap_or(false);

    ui.painter().rect_filled(rect, 12.0f32, color::panel());
    ui.painter().rect_stroke(
        rect,
        12.0f32,
        Stroke::new(
            if selected { 1.6 } else { 1.0 },
            if selected { color::indigo() } else { color::border() },
        ),
        StrokeKind::Middle,
    );
    if selected {
        ui.painter().rect_filled(
            Rect::from_min_size(rect.min, egui::vec2(rect.width(), 3.5)),
            3.0f32,
            color::indigo(),
        );
    }

    let mut clicked = false;
    let mut generate = false;
    let lang = app.lang();

    if let Some(shot) = shot {
        let tone = status_tone(shot.status);
        let well = Rect::from_min_size(
            egui::pos2(rect.left() + 12.0, rect.top() + 12.0),
            egui::vec2(rect.width() - 24.0, 56.0),
        );
        // Show the real reference image when available; otherwise the placeholder.
        if let Some(img) = shot.reference_image.as_deref() {
            let key = format!("story-ref-{}", img);
            app.request_image_bytes(&key, img);
            if let Some(tex) = app.textures.peek(&key) {
                ui.painter().image(
                    tex.id(),
                    well,
                    Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1.0, 1.0)),
                    Color32::WHITE,
                );
            } else {
                crate::placeholder::bamboo_scene(ui, well, 3 + seed);
            }
        } else {
            crate::placeholder::bamboo_scene(ui, well, 3 + seed);
        }
        ui.painter()
            .rect_stroke(well, 8.0f32, Stroke::new(1.0, color::border()), StrokeKind::Middle);

        ui.painter().text(
            egui::pos2(well.left() + 12.0, well.top() + 15.0),
            Align2::LEFT_CENTER,
            &crate::i18n::tf(lang, "SHOT {}", &[&format!("{:02}", shot.number)]),
            FontId::new(9.5, FontFamily::Proportional),
            color::text(),
        );
        let chip_rect =
            Rect::from_min_size(egui::pos2(well.right() - 62.0, well.top() + 10.0), egui::vec2(52.0, 20.0));
        ui.painter().rect_filled(chip_rect, 10.0f32, tone.dim());
        ui.painter().text(
            chip_rect.center(),
            Align2::CENTER_CENTER,
            status_label(lang, shot.status),
            FontId::new(9.5, FontFamily::Proportional),
            tone.accent(),
        );

        ui.painter().text(
            egui::pos2(rect.left() + 16.0, rect.top() + 78.0),
            Align2::LEFT_CENTER,
            &shot.title,
            FontId::new(12.5, FontFamily::Proportional),
            color::text(),
        );
        ui.painter().text(
            egui::pos2(rect.left() + 16.0, rect.top() + 97.0),
            Align2::LEFT_CENTER,
            &crate::i18n::tf(lang, "{} · {}s", &[&shot.mode, &shot.duration]),
            FontId::new(10.5, FontFamily::Proportional),
            color::text2(),
        );

        // Live progress bar (real value/max from WebSocket, or pulse while Running).
        if shot.status == crate::state::ShotStatus::Running
            || shot.status == crate::state::ShotStatus::Queued
        {
            let prog_rect = Rect::from_min_size(
                egui::pos2(rect.left() + 16.0, rect.top() + 114.0),
                egui::vec2(rect.width() - 108.0, 4.0),
            );
            let ratio = shot_progress(app, ui, shot);
            widgets::progress_bar(ui, prog_rect, ratio, tone.accent(), 2.0);
        }

        // Output filename once generated.
        if let Some(out) = shot.output.as_deref() {
            let short: String = out.chars().rev().take(26).collect::<String>().chars().rev().collect();
            ui.painter().text(
                egui::pos2(rect.left() + 16.0, rect.top() + 126.0),
                Align2::LEFT_CENTER,
                &format!("↳ {}", short),
                FontId::new(9.0, FontFamily::Monospace),
                color::text3(),
            );
        }

        let btn_rect =
            Rect::from_min_size(egui::pos2(rect.right() - 86.0, rect.bottom() - 28.0), egui::vec2(70.0, 22.0));
        let btn_resp = ui.interact(
            btn_rect,
            ui.id().with(("shot-gen", shot.id.clone())),
            egui::Sense::click(),
        );
        ui.painter().rect_filled(btn_rect, 6.0f32, color::card());
        ui.painter()
            .rect_stroke(btn_rect, 6.0f32, Stroke::new(1.0, color::border()), StrokeKind::Middle);
        ui.painter().text(
            btn_rect.center(),
            Align2::CENTER_CENTER,
            crate::i18n::tr(lang, "Generate"),
            FontId::new(9.5, FontFamily::Proportional),
            color::text(),
        );
        generate = btn_resp.clicked();

        clicked = outer.clicked() && !generate;
    }

    let _ = outer;
    (clicked, generate)
}

/// Inspector panel for the selected shot: editable prompt + reference image.
fn draw_shot_inspector(app: &mut H3App, ui: &mut egui::Ui, shot_id: &str) {
    let lang = app.lang();
    let shot = match app.local.shot(shot_id) {
        Some((_, _, s)) => s.clone(),
        None => return,
    };

    let panel_h = 168.0;
    let (rect, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), panel_h), Sense::hover());
    ui.painter().rect_filled(rect, 12.0, color::panel());
    ui.painter()
        .rect_stroke(rect, 12.0, Stroke::new(1.0, color::border()), StrokeKind::Middle);

    let label = format!("S01 · Shot {:02} · {} · {} ({}s)", shot.number, shot.title, shot.mode, shot.duration);
    ui.painter().text(
        egui::pos2(rect.left() + 16.0, rect.top() + 16.0),
        Align2::LEFT_CENTER,
        &label,
        FontId::new(13.0, FontFamily::Proportional),
        color::text(),
    );
    let tone = status_tone(shot.status);
    let chip = Rect::from_min_size(egui::pos2(rect.right() - 72.0, rect.top() + 10.0), egui::vec2(56.0, 22.0));
    ui.painter().rect_filled(chip, 11.0, tone.dim());
    ui.painter().text(
        chip.center(),
        Align2::CENTER_CENTER,
        status_label(lang, shot.status),
        FontId::new(9.5, FontFamily::Proportional),
        tone.accent(),
    );

    ui.painter().text(
        egui::pos2(rect.left() + 16.0, rect.top() + 42.0),
        Align2::LEFT_CENTER,
        crate::i18n::tr(lang, "Prompt"),
        FontId::new(10.0, FontFamily::Proportional),
        color::text2(),
    );
    ui.painter().text(
        egui::pos2(rect.left() + 16.0, rect.top() + 104.0),
        Align2::LEFT_CENTER,
        crate::i18n::tr(lang, "Reference image (first frame)"),
        FontId::new(10.0, FontFamily::Proportional),
        color::text2(),
    );
    if let Some(out) = shot.output.as_deref() {
        ui.painter().text(
            egui::pos2(rect.left() + 16.0, rect.top() + 148.0),
            Align2::LEFT_CENTER,
            &format!("Output · {}", out),
            FontId::new(9.5, FontFamily::Monospace),
            color::text3(),
        );
    }

    // Snapshot inputs before the mutable borrow of app.local.
    let input_images = app.input_images.clone();
    let mut changed = false;
    {
        let Some(shot) = app.local.shot_mut(shot_id) else {
            return;
        };
        let prompt_rect = Rect::from_min_size(
            egui::pos2(rect.left() + 16.0, rect.top() + 52.0),
            egui::vec2((rect.width() - 48.0) * 0.66, 46.0),
        );
        let resp = ui.put(
            prompt_rect,
            egui::TextEdit::multiline(&mut shot.prompt)
                .desired_rows(2)
                .font(FontId::new(11.0, FontFamily::Proportional)),
        );
        if resp.changed() {
            changed = true;
        }

        let combo_rect = Rect::from_min_size(
            egui::pos2(rect.left() + 16.0, rect.top() + 114.0),
            egui::vec2((rect.width() - 48.0) * 0.45, 22.0),
        );
        let before = shot.reference_image.clone();
        let mut sel = before.clone();
        let sel_text = sel
            .clone()
            .unwrap_or_else(|| "(default — first input)".to_string());
        let mut combo_ui = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(combo_rect)
                .layout(egui::Layout::top_down(egui::Align::Min)),
        );
        egui::ComboBox::from_id_salt(ui.id().with(("ref-combo", shot_id)))
            .selected_text(sel_text)
            .show_ui(&mut combo_ui, |ui| {
                ui.selectable_value(&mut sel, None, "(default — first input)");
                for img in &input_images {
                    ui.selectable_value(&mut sel, Some(img.clone()), img.as_str());
                }
            });
        if sel != before {
            shot.reference_image = sel;
            changed = true;
        }
    }
    if changed {
        app.local.save();
    }

    // Generate button (drawn outside the mutable borrow so we can call submit_shot).
    let btn_rect =
        Rect::from_min_size(egui::pos2(rect.right() - 110.0, rect.top() + 114.0), egui::vec2(94.0, 26.0));
    let btn_resp = ui.interact(btn_rect, ui.id().with(("inspector-gen", shot_id)), Sense::click());
    let disabled = shot.status == crate::state::ShotStatus::Running
        || shot.status == crate::state::ShotStatus::Queued;
    ui.painter().rect_filled(
        btn_rect,
        6.0,
        if disabled { color::text3() } else { color::indigo() },
    );
    ui.painter().text(
        btn_rect.center(),
        Align2::CENTER_CENTER,
        crate::i18n::tr(lang, "Generate"),
        FontId::new(10.0, FontFamily::Proportional),
        Color32::WHITE,
    );
    if btn_resp.clicked() && !disabled {
        let graph = build_shot_graph(app, &shot);
        let label = format!("Shot {:02} · {}", shot.number, shot.title);
        app.submit_shot(shot_id, graph, &label);
        app.nav_to(Screen::Queue);
    }
}

pub fn draw(app: &mut H3App, ui: &mut egui::Ui) {
    let lang = app.lang();
    let project = app.local.project().cloned();
    let scene = project.as_ref().and_then(|p| p.scenes.first().cloned());
    let (Some(project), Some(scene)) = (project, scene) else {
        return;
    };
    let selected_shot_id = app.local.selected_shot_id.clone();

    // Timeline strip
    let (tl, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 92.0), egui::Sense::hover());
    ui.painter().rect_filled(tl, 10.0f32, color::panel());
    ui.painter()
        .rect_stroke(tl, 10.0f32, Stroke::new(1.0, color::border()), StrokeKind::Middle);
    ui.painter().text(
        egui::pos2(tl.left() + 20.0, tl.top() + 24.0),
        Align2::LEFT_CENTER,
        crate::i18n::tr(lang, "SHOT TIMELINE · SCENE 01"),
        FontId::new(10.0, FontFamily::Proportional),
        color::text2(),
    );
    let total_width = tl.right() - tl.left() - 40.0;
    let chip_w = (total_width / 12.0) - 10.0;
    let mut timeline_pick = None;
    for (i, shot) in scene.shots.iter().take(12).enumerate() {
        let x = tl.left() + 20.0 + i as f32 * (chip_w + 10.0);
        let chip_rect = Rect::from_min_size(egui::pos2(x, tl.top() + 38.0), egui::vec2(chip_w, 40.0));
        let is_selected = Some(shot.id.as_str()) == selected_shot_id.as_deref();
        let chip_resp = ui.interact(chip_rect, ui.id().with(("tl-chip", i)), Sense::click());
        let hover = chip_resp.hovered() && !is_selected;
        ui.painter().rect_filled(
            chip_rect,
            8.0f32,
            if is_selected || hover { color::card() } else { color::panel() },
        );
        let chip_stroke = if is_selected {
            Stroke::new(1.6, color::indigo())
        } else {
            Stroke::new(
                if hover { 1.3 } else { 1.0 },
                if hover { color::indigo() } else { color::border() },
            )
        };
        ui.painter()
            .rect_stroke(chip_rect, 8.0f32, chip_stroke, StrokeKind::Middle);
        let num = format!("{:02}", shot.number);
        let num_col = if is_selected { color::indigo() } else { color::text() };
        ui.painter().text(
            egui::pos2(chip_rect.center().x, chip_rect.top() + 12.0),
            Align2::CENTER_CENTER,
            &num,
            FontId::new(12.0, FontFamily::Monospace),
            num_col,
        );
        let tone = status_tone(shot.status);
        ui.painter().text(
            egui::pos2(chip_rect.center().x, chip_rect.bottom() - 11.0),
            Align2::CENTER_CENTER,
            &format!("{}s", shot.duration),
            FontId::new(9.5, FontFamily::Monospace),
            tone.accent(),
        );
        if chip_resp.clicked() {
            timeline_pick = Some(shot.id.clone());
        }
    }
    if let Some(id) = timeline_pick {
        app.local.selected_shot_id = Some(id.clone());
        app.local.save();
        app.set_toast(crate::i18n::tf(lang, "Selected shot {}", &[&id]));
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
                    if clicked {
                        clicked_id = Some(s.id.clone());
                    }
                    if generated {
                        generate_id = Some(s.id.clone());
                    }
                }
                ui.add_space(12.0);
            }
        });
        ui.add_space(12.0);
    }

    // Inspector for the selected shot (editable prompt + reference image).
    if let Some(id) = selected_shot_id.as_ref() {
        draw_shot_inspector(app, ui, id);
    }
    ui.add_space(12.0);

    if let Some(id) = clicked_id {
        app.local.selected_shot_id = Some(id);
        app.local.save();
    }
    if let Some(id) = generate_id {
        // Real submission — build the graph from the shot and submit to the server.
        if let Some(shot) = app.local.shot(&id).map(|(_, _, s)| s.clone()) {
            let graph = build_shot_graph(app, &shot);
            let label = format!("Shot {:02} · {}", shot.number, shot.title);
            app.submit_shot(&id, graph, &label);
        }
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

fn status_label(lang: crate::i18n::Lang, status: crate::state::ShotStatus) -> &'static str {
    match status {
        crate::state::ShotStatus::Done => crate::i18n::tr(lang, "Done"),
        crate::state::ShotStatus::Running => crate::i18n::tr(lang, "Running"),
        crate::state::ShotStatus::Queued => crate::i18n::tr(lang, "Queued"),
        crate::state::ShotStatus::Pending => crate::i18n::tr(lang, "Pending"),
        crate::state::ShotStatus::Failed => crate::i18n::tr(lang, "Failed"),
    }
}

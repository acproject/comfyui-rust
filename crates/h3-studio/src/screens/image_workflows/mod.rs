//! Image workflow screens (去背景 / 电影画质 / 4视图 / 组合流程) — shared scaffolding.

use egui::{Align2, Color32, FontId, FontFamily, Margin, Rect, Stroke, StrokeKind};
use crate::app::H3App;
use crate::theme::{color, metrics, typography};
use crate::widgets;

pub mod remove_bg;
pub mod cinematic;
pub mod multi_view;
pub mod combined;

/// Frame-wrapped panel container.
pub fn panel_frame() -> egui::Frame {
    egui::Frame::none()
        .fill(color::panel())
        .stroke(Stroke::new(1.0, color::border()))
        .inner_margin(16.0)
        .corner_radius(12.0)
}

/// Field label with a thin accent bar on the left.
pub fn field_label(ui: &mut egui::Ui, text: &str, accent: Color32) {
    ui.horizontal(|ui| {
        let (rect, _) = ui.allocate_exact_size(egui::vec2(4.0, 16.0), egui::Sense::hover());
        ui.painter().rect_filled(rect, 2.0f32, accent);
        ui.add_space(6.0);
        ui.colored_label(color::text2(), egui::RichText::new(text.to_uppercase()).size(10.0).strong());
    });
}

/// Mono node box used in the pipeline graph + an arrow between nodes.
pub fn pipeline_node(ui: &mut egui::Ui, node_w: f32, name: &str, subtitle: &str, tone: widgets::Tone, active: bool) -> Rect {
    let rect = ui.allocate_exact_size(egui::vec2(node_w, 54.0), egui::Sense::hover()).0;
    ui.painter().rect_filled(rect, 8.0f32, if active { Color32::from_rgb(0x1A, 0x20, 0x30) } else { color::well() });
    ui.painter().rect_stroke(rect, 8.0f32, Stroke::new(if active { 1.4 } else { 1.0 }, if active { tone.accent() } else { color::border() }), StrokeKind::Middle);
    ui.painter().text(egui::pos2(rect.center().x, rect.top() + 19.0), Align2::CENTER_CENTER, name, FontId::new(10.5, FontFamily::Monospace), if active { color::text() } else { color::text2() });
    ui.painter().text(egui::pos2(rect.center().x, rect.bottom() - 13.0), Align2::CENTER_CENTER, subtitle, FontId::new(8.5, FontFamily::Proportional), color::text3());
    rect
}

/// Arrow drawn inside the given gap (current cell width) centered on y.
pub fn arrow_between(ui: &mut egui::Ui, y: f32) {
    let tip = egui::pos2(ui.cursor().min.x + ui.available_width() - 4.0, y);
    let from = egui::pos2(ui.cursor().min.x + 4.0, y);
    ui.painter().line_segment([from, tip], Stroke::new(1.6, color::text3()));
    ui.painter().text(tip, Align2::CENTER_CENTER, "▸", FontId::new(12.0, FontFamily::Proportional), color::text3());
}

/// Allocate an arrow gap (width w) with a horizontal line + chevron in the middle.
pub fn arrow_gap(ui: &mut egui::Ui, gap_w: f32) {
    let rect = ui.allocate_exact_size(egui::vec2(gap_w, 54.0), egui::Sense::hover()).0;
    let tip = egui::pos2(rect.right() - 4.0, rect.center().y);
    let from = egui::pos2(rect.left() + 4.0, rect.center().y);
    ui.painter().line_segment([from, tip], Stroke::new(1.6, color::text3()));
    ui.painter().text(tip, Align2::CENTER_CENTER, "▸", FontId::new(11.0, FontFamily::Proportional), color::text3());
}

/// Draw a row of nodes with arrows — node layout adapts to available width.
pub fn node_row(ui: &mut egui::Ui, specs: &[(&str, &str, widgets::Tone, bool)]) {
    let gap = 22.0;
    let avail = ui.available_width();
    let node_w = (avail - (specs.len() - 1) as f32 * gap) / specs.len() as f32;
    ui.horizontal(|ui| {
        for (i, (name, subtitle, tone, active)) in specs.iter().enumerate() {
            if i > 0 {
                arrow_gap(ui, gap);
            }
            pipeline_node(ui, node_w, name, subtitle, *tone, *active);
        }
    });
}

pub fn spec_table(ui: &mut egui::Ui, rows: &[(&str, String)]) {
    for (label, value) in rows.iter() {
        let rect = ui.allocate_exact_size(egui::vec2(ui.available_width(), 32.0), egui::Sense::hover()).0;
        ui.painter().rect_filled(rect, 6.0f32, color::well());
        ui.painter().rect_stroke(rect, 6.0f32, Stroke::new(1.0, color::border().linear_multiply(0.7)), StrokeKind::Middle);
        ui.painter().text(egui::pos2(rect.left() + 14.0, rect.center().y), Align2::LEFT_CENTER, label, FontId::new(10.5, FontFamily::Proportional), color::text2());
        ui.painter().text(egui::pos2(rect.right() - 14.0, rect.center().y), Align2::RIGHT_CENTER, value, FontId::new(10.5, FontFamily::Monospace), color::text());
        ui.add_space(4.0);
    }
}

/// Checkerboard base with optional transparent-image overlay.
pub fn output_preview(ui: &mut egui::Ui, app: &mut H3App, filename_key: &str, filename_for_preview: Option<&str>, overlay_scene: bool) {
    let rect = ui.allocate_exact_size(egui::vec2(ui.available_width(), 380.0), egui::Sense::hover()).0;
    crate::placeholder::checkerboard(ui, rect, 20.0, Color32::from_rgb(0x1A, 0x1E, 0x24), Color32::from_rgb(0x27, 0x2C, 0x35));
    ui.painter().rect_stroke(rect, 10.0f32, Stroke::new(1.0, color::border()), StrokeKind::Middle);

    if let Some(filename) = filename_for_preview {
        if !filename.is_empty() && app.textures.peek(filename_key).is_none() && !app.pending_inputs.contains(filename_key) {
            app.request_output_bytes(filename_key, filename);
        }
        if let Some(tex) = app.textures.peek(filename_key) {
            ui.painter().image(tex.id(), rect, Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1.0, 1.0)), Color32::WHITE);
            return;
        }
    }

    if overlay_scene {
        crate::placeholder::bamboo_scene(ui, rect, 4);
    } else {
        crate::placeholder::silhouette(ui, rect, color::teal());
    }
}

/// Bottom info row + save button (shared).
pub fn bottom_action(ui: &mut egui::Ui, label: &str, accent: Color32) -> bool {
    let action_rect = ui.allocate_exact_size(egui::vec2(ui.available_width(), 40.0), egui::Sense::click()).0;
    let resp = ui.interact(action_rect, ui.id().with(("bottom_action", label)), egui::Sense::click());
    let hover = resp.hovered();
    ui.painter().rect_filled(action_rect, 8.0f32, if hover { color::card() } else { Color32::TRANSPARENT });
    ui.painter().rect_stroke(action_rect, 8.0f32, Stroke::new(1.0, color::border()), StrokeKind::Middle);
    ui.painter().text(action_rect.center(), Align2::CENTER_CENTER, label, FontId::new(11.5, FontFamily::Proportional), accent);
    resp.clicked()
}

/// INPUT panel — shared frame with optional preview.
pub fn input_panel(
    ui: &mut egui::Ui,
    app: &mut H3App,
    image_key: &str,
    input_image: Option<&str>,
    name: &str,
    id_line: &str,
    note: &str,
    label: &str,
    accent: Color32,
    transparent: bool,
) {
    field_label(ui, label, accent);
    ui.add_space(12.0);

    let well = ui.allocate_exact_size(egui::vec2(ui.available_width(), 300.0), egui::Sense::hover()).0;
    if transparent {
        crate::placeholder::checkerboard(ui, well, 16.0, Color32::from_rgb(0x1A, 0x1E, 0x24), Color32::from_rgb(0x27, 0x2C, 0x35));
    } else {
        ui.painter().rect_filled(well, 10.0f32, color::well());
    }
    ui.painter().rect_stroke(well, 10.0f32, Stroke::new(1.0, color::border()), StrokeKind::Middle);

    if let Some(filename) = input_image {
        if !filename.is_empty() && app.textures.peek(image_key).is_none() && !app.pending_inputs.contains(image_key) {
            app.request_image_bytes(image_key, filename);
        }
        if let Some(tex) = app.textures.peek(image_key) {
            ui.painter().image(tex.id(), well, Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1.0, 1.0)), Color32::WHITE);
        } else {
            crate::placeholder::silhouette(ui, well, accent);
        }
    } else {
        crate::placeholder::silhouette(ui, well, accent);
    }

    let caption = input_image.unwrap_or("front.png");
    ui.add_space(10.0);
    ui.colored_label(color::text3(), egui::RichText::new(caption).monospace().size(9.5));
    ui.add_space(4.0);
    ui.colored_label(color::text(), egui::RichText::new(name).size(15.0).strong());
    ui.add_space(2.0);
    ui.colored_label(color::text2(), egui::RichText::new(id_line).monospace().size(10.5));
    ui.add_space(12.0);
    let (line_rect, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 1.0), egui::Sense::hover());
    ui.painter().rect_filled(line_rect, 0.0f32, color::border());
    ui.add_space(10.0);
    ui.colored_label(color::text2(), egui::RichText::new(note).size(10.5));
}

/// Key/value row — label left, mono value right.
pub fn kv_row(ui: &mut egui::Ui, label: &str, value: &str, value_color: Color32) {
    ui.horizontal(|ui| {
        ui.colored_label(color::text2(), label);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.colored_label(value_color, egui::RichText::new(value).monospace().size(10.0));
        });
    });
}

/// Input source picker — browse button + filename display.
pub fn input_picker(ui: &mut egui::Ui, app: &mut H3App, current: &str, label: &str) {
    let lang = app.lang();
    ui.colored_label(color::text2(), label);
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        if widgets::outline_button(ui, "…", 34.0, 30.0) {
            if let Some(path) = rfd::FileDialog::new().add_filter("images", &["png", "jpg", "jpeg", "webp"]).pick_file() {
                let filename = path.file_name().and_then(|n| n.to_str()).unwrap_or("upload.png").to_string();
                if let Ok(bytes) = std::fs::read(&path) {
                    let client = app.client.clone();
                    let fname = filename.clone();
                    std::thread::spawn(move || {
                        let _ = client.upload_input_image(bytes, &fname);
                    });
                    app.selected_input_image = Some(filename.clone());
                    app.set_toast(crate::i18n::tf(lang, "Uploading {} · input folder", &[&filename]));
                    app.last_poll = std::time::Instant::now() - std::time::Duration::from_secs(999);
                }
            }
        }
        ui.colored_label(color::text3(), egui::RichText::new(current).monospace().size(9.5));
    });
}

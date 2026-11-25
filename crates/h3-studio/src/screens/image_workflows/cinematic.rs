//! Cinematic from Reference — 参卡图生成电影画质图.json

use egui::{Align2, Color32, FontId, FontFamily, Rect, Stroke, StrokeKind};
use crate::app::H3App;
use crate::theme::{color, typography};
use crate::widgets;
use super::{field_label, spec_table, node_row, input_panel, output_preview, bottom_action, kv_row, panel_frame, input_picker};

pub fn draw(app: &mut H3App, ui: &mut egui::Ui) {
    ui.colored_label(color::text3(), egui::RichText::new("Pipeline · image workflow — runs in parallel with the combined flow").size(11.0));
    ui.add_space(8.0);

    let image = app.selected_input_image.clone()
        .or_else(|| app.input_images.first().cloned())
        .unwrap_or_else(|| "front.png".to_string());

    let total_w = ui.available_width();
    let w_in = total_w * 0.27;
    let w_pipe = total_w * 0.40;
    let w_out = total_w - w_in - w_pipe - 28.0;
    ui.horizontal(|ui| {
        // INPUT
        ui.vertical(|ui| {
            ui.set_width(w_in);
            panel_frame().show(ui, |ui| {
                input_panel(ui, app, "input-front-c", Some(&image), "Xiao Mei — Swordswoman", "char_001 · v3", "Reference card · clothing & styling locked", "INPUT", color::indigo(), false);
            });
        });

        // PIPELINE
        ui.add_space(14.0);
        ui.vertical(|ui| {
            ui.set_width(w_pipe);
            panel_frame().show(ui, |ui| {
                field_label(ui, "PIPELINE", color::indigo());
                ui.add_space(10.0);

                node_row(ui, &[
                    ("LoadImage", "ref card", widgets::Tone::Gray, false),
                    ("QwenImagePipeline", "Qwen-Image-2.1", widgets::Tone::Indigo, true),
                    ("SaveImage", "1344×768", widgets::Tone::Gray, false),
                ]);

                ui.add_space(14.0);
                ui.colored_label(color::text2(), egui::RichText::new("PROMPT").size(9.5).strong());
                ui.add_space(4.0);
                let _ = ui.add(
                    egui::TextEdit::multiline(&mut app.cine_prompt)
                        .desired_rows(4)
                        .frame(false)
                        .margin(egui::Margin::same(6))
                        .font(egui::TextStyle::Body)
                        .text_color(color::text())
                        .background_color(Color32::TRANSPARENT)
                        .hint_text("Enter cinematic prompt…"),
                );

                ui.add_space(10.0);
                ui.colored_label(color::text2(), egui::RichText::new("PARAMETERS").size(9.5).strong());
                ui.add_space(6.0);
                let rows = vec![
                    ("model", "Qwen-Image-2.1".to_string()),
                    ("cfg", "6.5".to_string()),
                    ("steps", "40".to_string()),
                    ("resolution", "1344 × 768".to_string()),
                    ("seed", "42".to_string()),
                ];
                spec_table(ui, &rows);

                ui.add_space(4.0);
                ui.colored_label(color::text3(), egui::RichText::new("Advanced ▸ negative prompt / subject-lock strength (hidden)").size(9.5));
                ui.add_space(10.0);

                if widgets::primary_button(ui, "Generate", 220.0, 46.0) {
                    let prompt = if app.cine_prompt.trim().is_empty() { crate::workflows::DEFAULT_CINE_PROMPT.to_string() } else { app.cine_prompt.clone() };
                    let p = crate::workflows::CinematicParams {
                        image: image.clone(),
                        prompt,
                        negative_prompt: crate::workflows::DEFAULT_CINE_NEGATIVE.to_string(),
                        model: "Qwen-Image-2.1".into(),
                        cfg: 6.5,
                        steps: 40,
                        width: 1344,
                        height: 768,
                        seed: 42,
                        prefix: "h3_cinematic".into(),
                    };
                    let graph = crate::workflows::cinematic_workflow(&p);
                    app.submit_workflow(graph, "Cinematic");
                }
                ui.add_space(8.0);
                ui.colored_label(color::text3(), "queued · est. 14s");

                ui.add_space(12.0);
                input_picker(ui, app, &image, "Source");
            });
        });

        // OUTPUT
        ui.add_space(14.0);
        ui.vertical(|ui| {
            ui.set_width(w_out);
            panel_frame().show(ui, |ui| {
                field_label(ui, "OUTPUT", color::indigo());
                ui.add_space(10.0);
                ui.colored_label(color::indigo(), egui::RichText::new("cinematic_bamboo.png").monospace().size(13.0).strong());
                ui.add_space(8.0);

                // preview well (bamboo scene + amber glow)
                let cine_preview = app.last_cinematic.clone().unwrap_or_default();
                output_preview(ui, app, "output-cinematic", if cine_preview.is_empty() { None } else { Some(&cine_preview) }, true);

                ui.add_space(10.0);
                kv_row(ui, "size", "1344 × 768", color::text());
                ui.add_space(2.0);
                kv_row(ui, "file", "PNG · 3.4 MB", color::text());
                ui.add_space(2.0);
                kv_row(ui, "model", "Qwen-Image-2.1", color::text());
                ui.add_space(2.0);
                kv_row(ui, "parallel", "independent of chain", color::text());

                ui.add_space(12.0);
                if bottom_action(ui, "↓  Save to project assets", color::indigo()) {
                    app.set_toast("cinematic saved to project assets (demo)");
                }
            });
        });
    });
}

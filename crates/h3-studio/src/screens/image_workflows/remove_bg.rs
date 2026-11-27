//! Remove Background — 去背景.json (LoadImage → RemoveBackground → SaveImage).

use egui::{Align2, Color32, FontId, FontFamily, Rect, Stroke, StrokeKind};
use crate::app::H3App;
use crate::theme::{color, typography};
use crate::widgets;
use super::{field_label, spec_table, node_row, input_panel, output_preview, bottom_action, kv_row, panel_frame, input_picker};

pub fn draw(app: &mut H3App, ui: &mut egui::Ui) {
    use crate::i18n::{tr, tf};
    let lang = app.lang();
    ui.colored_label(color::text3(), egui::RichText::new(tr(lang, "Pipeline · image workflow — run standalone or as step 1 of the combined flow")).size(11.0));
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
                input_panel(ui, app, "input-front", Some(&image), tr(lang, "Xiao Mei — Swordswoman"), "char_001 · v3", tr(lang, "Source character · front reference shot"), tr(lang, "INPUT"), color::teal(), false);
            });
        });

        // PIPELINE
        ui.add_space(14.0);
        ui.vertical(|ui| {
            ui.set_width(w_pipe);
            panel_frame().show(ui, |ui| {
                field_label(ui, tr(lang, "PIPELINE"), color::teal());
                ui.add_space(10.0);

                // node row (LoadImage → RemoveBackground → SaveImage)
                node_row(ui, &[
                    ("LoadImage", tr(lang, "load source"), widgets::Tone::Gray, false),
                    ("RemoveBackground", "bria-rmbg", widgets::Tone::Teal, true),
                    ("SaveImage", "PNG · RGBA", widgets::Tone::Gray, false),
                ]);

                ui.add_space(14.0);
                ui.colored_label(color::text2(), egui::RichText::new(tr(lang, "PARAMETERS")).size(9.5).strong());
                ui.add_space(6.0);
                let rows = vec![
                    (tr(lang, "model"), "bria-rmbg".to_string()),
                    (tr(lang, "output format"), "PNG · RGBA".to_string()),
                    (tr(lang, "resolution"), "1024 × 1024".to_string()),
                    (tr(lang, "alpha matting"), "auto".to_string()),
                ];
                spec_table(ui, &rows);

                ui.add_space(4.0);
                ui.colored_label(color::text3(), egui::RichText::new(tr(lang, "Advanced ▸ threshold / feather / foreground erosion (hidden)")).size(9.5));
                ui.add_space(12.0);

                if widgets::teal_button(ui, tr(lang, "Generate"), 220.0, 46.0) {
                    let graph = crate::workflows::remove_background_workflow(&image, "h3_bg", "bria-rmbg");
                    app.submit_workflow(graph, tr(lang, "Remove BG"));
                }
                ui.add_space(8.0);
                ui.colored_label(color::text3(), tr(lang, "last run 2m ago · 0.8s"));

                ui.add_space(12.0);
                input_picker(ui, app, &image, tr(lang, "Source"));
            });
        });

        // OUTPUT
        ui.add_space(14.0);
        ui.vertical(|ui| {
            ui.set_width(w_out);
            panel_frame().show(ui, |ui| {
                field_label(ui, tr(lang, "OUTPUT"), color::teal());
                ui.add_space(10.0);
                ui.colored_label(color::teal(), egui::RichText::new("front_cutout.png").monospace().size(13.0).strong());
                ui.add_space(8.0);

                let cutout_preview = app.last_cutout.clone().unwrap_or_default();
                output_preview(ui, app, "output-cutout", if cutout_preview.is_empty() { None } else { Some(&cutout_preview) }, false);

                ui.add_space(10.0);
                let cutout_file = app.last_cutout.clone().unwrap_or_else(|| "front_cutout.png".to_string());
                let file_size = if app.last_cutout.is_some() { tr(lang, "actual").to_string() } else { "PNG · 1.2 MB".to_string() };
                kv_row(ui, tr(lang, "size"), "1024 × 1024 · RGBA", color::text());
                ui.add_space(2.0);
                kv_row(ui, tr(lang, "file"), &format!("{} · {}", cutout_file, file_size), color::text());
                ui.add_space(2.0);
                kv_row(ui, tr(lang, "pipeline"), "RemoveBackground", color::text());
                ui.add_space(2.0);
                kv_row(ui, tr(lang, "feeds"), tr(lang, "→ Multi-View (step 2)"), color::teal());

                ui.add_space(12.0);
                if bottom_action(ui, tr(lang, "↓  Save to project assets"), color::teal()) {
                    app.toast_success(tr(lang, "cutout saved to project assets (demo)").to_string());
                }
            });
        });
    });
}

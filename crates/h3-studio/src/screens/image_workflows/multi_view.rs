//! Multi-View Turnaround — 将人物切换为4视图.json

use egui::{Align2, Color32, FontId, FontFamily, Rect, Stroke, StrokeKind};
use crate::app::H3App;
use crate::theme::{color, typography};
use crate::widgets;
use super::{field_label, spec_table, node_row, input_panel, bottom_action, kv_row, panel_frame, input_picker};

pub fn draw(app: &mut H3App, ui: &mut egui::Ui) {
    ui.colored_label(color::text3(), egui::RichText::new("Pipeline · image workflow — step 2 of the combined flow (consumes transparent PNG)").size(11.0));
    ui.add_space(8.0);

    let image = app.selected_input_image.clone()
        .or_else(|| app.input_images.first().cloned())
        .unwrap_or_else(|| "front_cutout.png".to_string());

    let total_w = ui.available_width();
    let w_in = total_w * 0.27;
    let w_pipe = total_w * 0.40;
    let w_out = total_w - w_in - w_pipe - 28.0;
    ui.horizontal(|ui| {
        // INPUT
        ui.vertical(|ui| {
            ui.set_width(w_in);
            panel_frame().show(ui, |ui| {
                input_panel(ui, app, "input-front-m", Some(&image), "Xiao Mei — Swordswoman", "char_001 · v3", "From Remove BG · transparent source (best result)", "INPUT", color::amber(), true);
            });
        });

        // PIPELINE
        ui.add_space(14.0);
        ui.vertical(|ui| {
            ui.set_width(w_pipe);
            panel_frame().show(ui, |ui| {
                field_label(ui, "PIPELINE", color::amber());
                ui.add_space(10.0);

                node_row(ui, &[
                    ("LoadImage", "cutout", widgets::Tone::Gray, false),
                    ("MultiViewPose", "Qwen-Image-Edit-2511", widgets::Tone::Amber, true),
                    ("ImageGrid", "2×2", widgets::Tone::Gray, false),
                    ("SaveImage", "per-view", widgets::Tone::Gray, false),
                ]);

                ui.add_space(12.0);
                ui.colored_label(color::text2(), egui::RichText::new("VIEWS").size(9.5).strong());
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    for view in ["front", "side", "back", "top-down"] {
                        let w = widgets::est_text_width(view, 10.5) + 24.0;
                        let (rect, _) = ui.allocate_exact_size(egui::vec2(w.max(64.0), 26.0), egui::Sense::hover());
                        ui.painter().rect_filled(rect, 13.0f32, color::amber_dim());
                        ui.painter().rect_stroke(rect, 13.0f32, Stroke::new(1.0, color::amber().linear_multiply(0.5)), StrokeKind::Middle);
                        ui.painter().text(rect.center(), Align2::CENTER_CENTER, view, FontId::new(10.5, FontFamily::Proportional), color::amber());
                        ui.add_space(4.0);
                    }
                });

                ui.add_space(10.0);
                ui.colored_label(color::text2(), egui::RichText::new("PARAMETERS").size(9.5).strong());
                ui.add_space(4.0);
                let rows = vec![
                    ("subject lock", "same face, hair, clothing".to_string()),
                    ("cfg", "2.5".to_string()),
                    ("steps", "40".to_string()),
                    ("grid", "2 × 2".to_string()),
                    ("resolution", "1226 × 832".to_string()),
                ];
                spec_table(ui, &rows);

                ui.add_space(4.0);
                ui.colored_label(color::text3(), egui::RichText::new("Advanced ▸ lora / lora_strength (hidden)").size(9.5));
                ui.add_space(12.0);

                // save-as segmented
                ui.colored_label(color::text2(), egui::RichText::new("SAVE AS").size(9.5).strong());
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    let merged = app.multiview_save_as == crate::workflows::SaveAs::Merged;
                    let seg_w = 118.0;
                    let (merged_rect, merged_resp) = ui.allocate_exact_size(egui::vec2(seg_w, 28.0), egui::Sense::click());
                    ui.painter().rect_filled(merged_rect, 14.0f32, if merged { color::amber() } else { color::card() });
                    ui.painter().rect_stroke(merged_rect, 14.0f32, Stroke::new(1.0, color::border()), StrokeKind::Middle);
                    ui.painter().text(merged_rect.center(), Align2::CENTER_CENTER, "merged 2×2", FontId::new(9.5, FontFamily::Proportional), if merged { color::on_accent() } else { color::text2() });
                    if merged_resp.clicked() { app.multiview_save_as = crate::workflows::SaveAs::Merged; }
                    ui.add_space(4.0);
                    let (perview_rect, perview_resp) = ui.allocate_exact_size(egui::vec2(seg_w, 28.0), egui::Sense::click());
                    ui.painter().rect_filled(perview_rect, 14.0f32, if !merged { color::amber() } else { color::card() });
                    ui.painter().rect_stroke(perview_rect, 14.0f32, Stroke::new(1.0, color::border()), StrokeKind::Middle);
                    ui.painter().text(perview_rect.center(), Align2::CENTER_CENTER, "per-view", FontId::new(9.5, FontFamily::Proportional), if !merged { color::on_accent() } else { color::text2() });
                    if perview_resp.clicked() { app.multiview_save_as = crate::workflows::SaveAs::PerView; }
                });
                ui.add_space(12.0);

                if widgets::amber_button(ui, "Generate", 220.0, 46.0) {
                    let p = crate::workflows::MultiViewParams {
                        image: image.clone(),
                        instructions: crate::workflows::DEFAULT_MULTI_INSTRUCTIONS.into(),
                        subject_guidance: crate::workflows::DEFAULT_MULTI_GUIDANCE.into(),
                        lora: crate::workflows::DEFAULT_LORA.into(),
                        lora_strength: 0.0,
                        negative_prompt: "anime, manga, cartoon, illustration, painting, sketch, watercolor, 3d render, cgi".into(),
                        model: "Qwen-Image-Edit-2511".into(),
                        cfg: 2.5,
                        steps: 40,
                        width: 1226,
                        height: 832,
                        seed: 42,
                        prefix: "h3_4view".into(),
                        save_as: app.multiview_save_as,
                    };
                    let graph = crate::workflows::multiview_workflow(&p);
                    app.submit_workflow(graph, "Multi-View");
                }
                ui.add_space(8.0);
                ui.colored_label(color::text3(), "idle · est. 22s");

                ui.add_space(12.0);
                input_picker(ui, app, &image, "Source");
            });
        });

        // OUTPUT
        ui.add_space(14.0);
        ui.vertical(|ui| {
            ui.set_width(w_out);
            panel_frame().show(ui, |ui| {
                field_label(ui, "OUTPUT", color::amber());
                ui.add_space(10.0);
                let mv_label = app.last_multiview.clone().unwrap_or_else(|| "4view_grid.png".to_string());
                ui.colored_label(color::amber(), egui::RichText::new(&mv_label).monospace().size(13.0).strong());
                ui.add_space(8.0);

                // 2×2 view grid
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.colored_label(color::text3(), egui::RichText::new("merged 2×2 (active)").size(9.5));
                        ui.add_space(4.0);
                        let (grid, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 260.0), egui::Sense::hover());
                        // Real generated 4-view grid, once the server finishes rendering it.
                        let mv_file = app.last_multiview.clone();
                        if let Some(f) = &mv_file {
                            if !f.is_empty()
                                && app.textures.peek("output-multiview").is_none()
                                && !app.pending_inputs.contains("output-multiview")
                            {
                                app.request_output_bytes("output-multiview", f);
                            }
                        }
                        if let Some(tex) = app.textures.peek("output-multiview") {
                            ui.painter().image(tex.id(), grid, Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1.0, 1.0)), Color32::WHITE);
                        } else {
                            let views = ["front", "side", "back", "top"];
                            let cell_w = (grid.width() - 6.0) / 2.0;
                            let cell_h = (grid.height() - 6.0) / 2.0;
                            for (i, view) in views.iter().enumerate() {
                                let cx = grid.left() + (i % 2) as f32 * (cell_w + 6.0);
                                let cy = grid.top() + (i / 2) as f32 * (cell_h + 6.0);
                                let cell = Rect::from_min_size(egui::pos2(cx, cy), egui::vec2(cell_w, cell_h));
                                ui.painter().rect_filled(cell, 8.0f32, color::well());
                                ui.painter().rect_stroke(cell, 8.0f32, Stroke::new(1.0, color::border()), StrokeKind::Middle);
                                ui.painter().circle_filled(egui::pos2(cell.center().x, cell.center().y - 8.0), 16.0, color::thumb());
                                ui.painter().circle_stroke(egui::pos2(cell.center().x, cell.center().y - 8.0), 16.0, Stroke::new(1.4, color::amber()));
                                let tag_rect = Rect::from_min_size(egui::pos2(cell.left() + 8.0, cell.top() + 8.0), egui::vec2(50.0, 17.0));
                                ui.painter().rect_filled(tag_rect, 8.0f32, color::amber_dim());
                                ui.painter().text(tag_rect.center(), Align2::CENTER_CENTER, view, FontId::new(9.5, FontFamily::Proportional), color::amber());
                            }
                        }
                    });

                    ui.add_space(10.0);

                    ui.vertical(|ui| {
                        ui.colored_label(color::text3(), egui::RichText::new("per-view individual").size(9.5));
                        ui.add_space(4.0);
                        for file in ["v_front.png", "v_side.png", "v_back.png", "v_top.png"] {
                            let (row, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 46.0), egui::Sense::hover());
                            ui.painter().rect_filled(row, 6.0f32, color::well());
                            ui.painter().rect_stroke(row, 6.0f32, Stroke::new(1.0, color::border()), StrokeKind::Middle);
                            ui.painter().circle_filled(egui::pos2(row.left() + 22.0, row.center().y), 8.0, color::thumb());
                            ui.painter().circle_stroke(egui::pos2(row.left() + 22.0, row.center().y), 8.0, Stroke::new(1.2, color::amber()));
                            ui.painter().text(egui::pos2(row.left() + 40.0, row.center().y), Align2::LEFT_CENTER, file, FontId::new(9.0, FontFamily::Monospace), color::text());
                            ui.painter().text(egui::pos2(row.right() - 10.0, row.center().y), Align2::RIGHT_CENTER, "↓ save", FontId::new(8.5, FontFamily::Proportional), color::amber());
                            ui.add_space(6.0);
                        }
                    });
                });

                ui.add_space(10.0);
                kv_row(ui, "merged", "4view_grid.png · 1226×832", color::text());
                ui.add_space(2.0);
                kv_row(ui, "per-view", "v_front/side/back/top.png", color::text());
                ui.add_space(2.0);
                kv_row(ui, "model", "Qwen-Image-Edit-2511", color::text());
                ui.add_space(2.0);
                kv_row(ui, "source", "front_cutout.png (transparent)", color::text());

                ui.add_space(12.0);
                if bottom_action(ui, "↓  Save merged + per-view", color::amber()) {
                    app.set_toast("4-view saved to project assets (demo)");
                }
            });
        });
    });
}

//! Generation Studio — FL2VA / Ref2VA video generation (real workflow submission).

use egui::{Align2, Color32, FontId, FontFamily, Rect, Stroke, StrokeKind, Frame};
use crate::app::H3App;
use crate::theme::{color, typography};
use crate::widgets;

#[derive(Clone, Copy, PartialEq, Default)]
pub enum StudioMode {
    #[default]
    Fl2VA,
    Ref2VA,
}

impl StudioMode {
    pub fn label(&self) -> &'static str {
        match self {
            StudioMode::Fl2VA => "FL2VA",
            StudioMode::Ref2VA => "Ref2VA",
        }
    }
    pub fn prompt_label(&self) -> &'static str {
        match self {
            StudioMode::Fl2VA => "Prompt + first/last frame",
            StudioMode::Ref2VA => "Multimodal references",
        }
    }
}

pub fn draw(app: &mut H3App, ui: &mut egui::Ui) {
    let mode = app.studio_mode;

    // Mode segmented control
    ui.horizontal(|ui| {
        if mode == StudioMode::Fl2VA {
            let _ = widgets::primary_button(ui, "FL2VA · first/last frame", 250.0, 30.0);
        } else if widgets::outline_button(ui, "FL2VA · first/last frame", 250.0, 30.0) {
            app.studio_mode = StudioMode::Fl2VA;
        }
        ui.add_space(4.0);
        if mode == StudioMode::Ref2VA {
            let _ = widgets::primary_button(ui, "Ref2VA · multimodal refs", 250.0, 30.0);
        } else if widgets::outline_button(ui, "Ref2VA · multimodal refs", 250.0, 30.0) {
            app.studio_mode = StudioMode::Ref2VA;
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.colored_label(color::text3(), "switch: ⌘↵");
        });
    });
    ui.add_space(10.0);

    ui.horizontal(|ui| {
        // ── SHOT CONTEXT (left) ──
        ui.vertical(|ui| {
            ui.set_width(340.0);
            Frame::none().fill(color::panel()).stroke(Stroke::new(1.0, color::border())).inner_margin(16.0).corner_radius(12.0).show(ui, |ui| {
                ui.colored_label(color::text2(), egui::RichText::new("SHOT CONTEXT").size(9.5).strong());
                ui.add_space(4.0);
                ui.colored_label(color::text(), egui::RichText::new("S01 · Shot 003").size(16.5).strong());
                ui.add_space(2.0);
                ui.colored_label(color::text2(), egui::RichText::new("Rain walk through bamboo forest").size(10.5));
                ui.add_space(2.0);
                ui.colored_label(color::text3(), egui::RichText::new("8s · 25fps · 1920×1080").monospace().size(10.0));

                ui.add_space(12.0);
                ui.colored_label(color::text2(), egui::RichText::new("CHARACTER REFERENCE").size(9.0).strong());
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    let (rect, _) = ui.allocate_exact_size(egui::vec2(28.0, 28.0), egui::Sense::hover());
                    ui.painter().circle_filled(egui::pos2(rect.center().x, rect.center().y), 13.0, color::thumb());
                    ui.painter().circle_stroke(egui::pos2(rect.center().x, rect.center().y), 13.0, Stroke::new(1.4, color::indigo()));
                    ui.add_space(2.0);
                    ui.vertical(|ui| {
                        ui.colored_label(color::text(), egui::RichText::new("Xiao Mei — front.png · v3").size(10.5));
                        ui.colored_label(color::text3(), egui::RichText::new("last used 2m ago").size(9.5));
                    });
                });

                ui.add_space(12.0);
                ui.colored_label(color::text2(), egui::RichText::new("REFERENCE ASSETS").size(9.0).strong());
                ui.add_space(6.0);
                let assets = [
                    ("bamboo_forest.png", "Environment", color::teal()),
                    ("rain_loop.mp4", "Motion ref", color::teal()),
                    ("bamboo_ambience.wav", "Audio ref", color::amber()),
                ];
                for (name, role, accent) in assets {
                    let row = ui.allocate_exact_size(egui::vec2(ui.available_width(), 38.0), egui::Sense::hover()).0;
                    ui.painter().rect_filled(row, 8.0f32, color::card());
                    ui.painter().rect_stroke(row, 8.0f32, Stroke::new(1.0, color::border()), StrokeKind::Middle);
                    ui.painter().rect_filled(Rect::from_min_size(row.min, egui::vec2(3.0, row.height())), 0.0f32, accent);
                    ui.painter().text(egui::pos2(row.left() + 12.0, row.center().y), Align2::LEFT_CENTER, name, FontId::new(10.0, FontFamily::Proportional), color::text());
                    ui.painter().text(egui::pos2(row.right() - 10.0, row.center().y), Align2::RIGHT_CENTER, role, FontId::new(8.5, FontFamily::Proportional), color::text3());
                    ui.add_space(4.0);
                }

                ui.add_space(4.0);
                ui.colored_label(color::text3(), egui::RichText::new("3 references attached · within model limits").size(9.5));

                ui.add_space(8.0);
                ui.colored_label(color::text2(), egui::RichText::new("CAPABILITY STATUS").size(9.0).strong());
                ui.add_space(6.0);
                let caps: Vec<(&str, &str, &str)> = match mode {
                    StudioMode::Fl2VA => vec![
                        ("FL2VA · first/last frame", "SUPPORTED", "teal"),
                        ("Ref2VA · multimodal", "SUPPORTED", "teal"),
                        ("Audio-driven motion", "EXPERIMENTAL", "amber"),
                        ("Attention backbones", "UNSUPPORTED", "red"),
                    ],
                    StudioMode::Ref2VA => vec![
                        ("Ref2VA · multimodal", "SUPPORTED", "teal"),
                        ("FL2VA · first/last frame", "SUPPORTED", "teal"),
                        ("Attention backbones", "UNSUPPORTED", "red"),
                    ],
                };
                for (label, status, tone) in caps {
                    let dot_col = match tone { "amber" => color::amber(), "red" => color::red(), _ => color::teal() };
                    ui.horizontal(|ui| {
                        let (dot_rect, _) = ui.allocate_exact_size(egui::vec2(8.0, 8.0), egui::Sense::hover());
                        ui.painter().circle_filled(egui::pos2(dot_rect.center().x, dot_rect.center().y), 4.0, dot_col);
                        ui.add_space(2.0);
                        ui.colored_label(color::text2(), egui::RichText::new(label).size(9.5));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.colored_label(dot_col, egui::RichText::new(status).size(8.5).strong());
                        });
                    });
                    ui.add_space(3.0);
                }
                ui.add_space(2.0);
            });
        });

        // ── CENTER: preview well ──
        ui.add_space(14.0);
        ui.vertical(|ui| {
            ui.colored_label(color::text2(), egui::RichText::new("VIDEO PREVIEW · first frame + last frame").size(10.5));
            ui.add_space(8.0);
            let (well, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 380.0), egui::Sense::hover());
            crate::placeholder::bamboo_scene(ui, well, 21);
            ui.painter().rect_filled(well, 12.0f32, Color32::from_rgba_premultiplied(0x0B, 0x12, 0x19, 64));
            ui.painter().rect_stroke(well, 12.0f32, Stroke::new(1.0, color::border()), StrokeKind::Middle);
            crate::placeholder::play_button(ui, well.center(), 34.0, if mode == StudioMode::Fl2VA { color::indigo() } else { color::teal() });
            ui.painter().text(egui::pos2(well.center().x, well.bottom() - 18.0), Align2::CENTER_CENTER, "1920×1080 · 25fps · 8s", FontId::new(9.5, FontFamily::Monospace), color::text3());
            ui.add_space(8.0);
            ui.colored_label(color::text3(), "First + last frame · H3 Video");
        });

        // ── RIGHT: PROMPT panel ──
        ui.add_space(14.0);
        ui.vertical(|ui| {
            ui.set_width(360.0);
            Frame::none().fill(color::panel()).stroke(Stroke::new(1.0, color::border())).inner_margin(16.0).corner_radius(12.0).show(ui, |ui| {
                ui.colored_label(color::text(), egui::RichText::new("PROMPT").size(10.5).strong());
                ui.add_space(2.0);
                ui.add(
                    egui::TextEdit::multiline(&mut app.studio_prompt)
                        .desired_rows(3)
                        .frame(false)
                        .margin(egui::Margin::same(0))
                        .font(egui::TextStyle::Body)
                        .text_color(color::text())
                        .background_color(Color32::TRANSPARENT)
                        .hint_text("Enter prompt…"),
                );
                ui.add_space(4.0);
                ui.colored_label(color::text2(), egui::RichText::new("Duration  8s   ·   FPS  25").size(11.0));
                ui.colored_label(color::text3(), egui::RichText::new(format!("Mode  {}", mode.prompt_label())).size(10.0));
                ui.add_space(8.0);

                if widgets::primary_button(ui, "Generate Video", ui.available_width() - 2.0, 40.0) {
                    let graph = build_video_graph(app, mode);
                    app.submit_workflow(graph, &format!("Video · {}", mode.label()));
                }

                ui.add_space(8.0);
                ui.colored_label(color::text3(), egui::RichText::new("Estimated 00:12 on GPU 0 · 13.2GB VRAM").size(10.0));
                ui.colored_label(color::amber(), egui::RichText::new("⚠ first/last frame not yet verified").size(10.0));
            });
        });
    });
}

fn build_video_graph(app: &H3App, mode: StudioMode) -> serde_json::Value {
    let image = app.input_images.first().cloned().unwrap_or_else(|| "OK_033_00000.png".into());
    let prompt = if app.studio_prompt.trim().is_empty() {
        "Swordswoman walks through bamboo, rain falling, cloak moving, volumetric light, cinematic, wuxia, 35mm.".to_string()
    } else {
        app.studio_prompt.clone()
    };

    match mode {
        StudioMode::Fl2VA => {
            let p = crate::workflows::Fl2VaParams {
                first_frame: image.clone(),
                last_frame: image.clone(),
                prompt,
                fps: 24,
                height: 480,
                width: 832,
                num_frames: 123,
                seed: 42,
                steps: 30,
                embed_frames: true,
                prefix: "h3_fl2va".into(),
            };
            crate::workflows::fl2va_workflow(&p)
        }
        StudioMode::Ref2VA => {
            let p = crate::workflows::Ref2VaParams {
                image: image.clone(),
                view_descriptions: [
                    "bamboo woman front view: crimson red and black hanfu, high ponytail, jian sword".into(),
                    "bamboo woman side view: identical crimson red and black hanfu".into(),
                    "bamboo woman back view: red robe back, black belt, ponytail, jian in right hand".into(),
                    "bamboo woman top-down view: identical crimson red and black hanfu".into(),
                ],
                vae_dir: "/home/acproject/usb/comfyui/models/MiniMax-H3/Ref2VA/video_vae".into(),
                prompt,
                embed_frames: true,
                fps: 24,
                height: 480,
                width: 832,
                num_frames: 123,
                seed: 42,
                split_gpu: true,
                steps: 30,
                strength: 1.0,
                prefix: "h3_refmod_bamboo_s".into(),
            };
            crate::workflows::ref2va_workflow(&p)
        }
    }
}

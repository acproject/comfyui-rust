//! Combined Pipeline — 去背景 → 4 视图 chained · cinematic in parallel.

use egui::{Align2, Color32, FontId, FontFamily, Rect, Stroke, StrokeKind};
use crate::app::{H3App, Screen};
use crate::theme::{color, metrics, typography};
use crate::widgets;
use super::{field_label, bottom_action, kv_row};

/// Painted node box in the combined-pipeline graph.
fn node_box(ui: &mut egui::Ui, rect: Rect, name: &str, subtitle: &str, tone: widgets::Tone, active: bool) {
    ui.painter().rect_filled(rect, 8.0f32, if active { Color32::from_rgb(0x1A, 0x20, 0x30) } else { color::well() });
    ui.painter().rect_stroke(rect, 8.0f32, Stroke::new(if active { 1.4 } else { 1.0 }, if active { tone.accent() } else { color::border() }), StrokeKind::Middle);
    ui.painter().text(egui::pos2(rect.center().x, rect.top() + 24.0), Align2::CENTER_CENTER, name, FontId::new(12.5, FontFamily::Monospace), if active { color::text() } else { color::text2() });
    ui.painter().text(egui::pos2(rect.center().x, rect.bottom() - 14.0), Align2::CENTER_CENTER, subtitle, FontId::new(9.5, FontFamily::Proportional), color::text3());
}

/// Arrow between two node rects.
fn arrow_between(ui: &mut egui::Ui, from_x: f32, to_x: f32, y: f32) {
    ui.painter().line_segment([egui::pos2(from_x + 6.0, y), egui::pos2(to_x - 6.0, y)], Stroke::new(1.6, color::text3()));
    ui.painter().text(egui::pos2(to_x - 5.0, y), Align2::CENTER_CENTER, "▸", FontId::new(12.0, FontFamily::Proportional), color::text3());
}

pub fn draw(app: &mut H3App, ui: &mut egui::Ui) {
    ui.colored_label(color::text3(), egui::RichText::new("Overview · chained image pipeline — run all at once, or open any step independently").size(11.0));
    ui.add_space(8.0);

    let image = app.selected_input_image.clone()
        .or_else(|| app.input_images.first().cloned())
        .unwrap_or_else(|| "front.png".to_string());

    // ── COMBINED PIPELINE graph panel ──
    let (graph_rect, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 330.0), egui::Sense::hover());
    ui.painter().rect_filled(graph_rect, 12.0f32, color::panel());
    ui.painter().rect_stroke(graph_rect, 12.0f32, Stroke::new(1.0, color::border()), StrokeKind::Middle);

    // label + pills (painter — above the main content row)
    let label_y = graph_rect.top() + 14.0;
    ui.painter().rect_filled(Rect::from_min_size(egui::pos2(graph_rect.left() + 18.0, label_y + 2.0), egui::vec2(4.0, 16.0)), 2.0f32, color::indigo());
    ui.painter().text(egui::pos2(graph_rect.left() + 30.0, label_y + 10.0), Align2::LEFT_CENTER, "COMBINED PIPELINE", FontId::new(10.0, FontFamily::Proportional), color::text2());
    let pill_y = label_y + 30.0;
    let steps_pill = Rect::from_min_size(egui::pos2(graph_rect.left() + 18.0, pill_y), egui::vec2(76.0, 24.0));
    ui.painter().rect_filled(steps_pill, 12.0f32, color::indigo_dim());
    ui.painter().text(steps_pill.center(), Align2::CENTER_CENTER, "3 steps", FontId::new(10.0, FontFamily::Proportional), color::indigo());
    let mock_pill = Rect::from_min_size(egui::pos2(steps_pill.right() + 8.0, pill_y), egui::vec2(104.0, 24.0));
    ui.painter().rect_filled(mock_pill, 12.0f32, color::gray_dim());
    ui.painter().text(mock_pill.center(), Align2::CENTER_CENTER, "mock data", FontId::new(10.0, FontFamily::Proportional), color::text2());

    // main chain
    let chain_y = graph_rect.top() + 76.0;
    let node_w = 218.0;
    let node_h = 76.0;
    let right_col_x = graph_rect.right() - 260.0;

    let n1_x = graph_rect.left() + 24.0;
    let n2_x = n1_x + node_w + 40.0;
    let n3_x = n2_x + node_w + 40.0;
    let n4_x = n3_x + node_w + 40.0;

    let n1 = Rect::from_min_size(egui::pos2(n1_x, chain_y), egui::vec2(node_w, node_h));
    let n2 = Rect::from_min_size(egui::pos2(n2_x, chain_y), egui::vec2(node_w, node_h));
    let n3 = Rect::from_min_size(egui::pos2(n3_x, chain_y), egui::vec2(node_w, node_h));
    let n4 = Rect::from_min_size(egui::pos2(n4_x, chain_y), egui::vec2(node_w, node_h));

    // source
    node_box(ui, n1, "front.png", "SOURCE · char_001", widgets::Tone::Teal, false);
    status_dot_label(ui, n1, "done", color::teal());
    arrow_between(ui, n1.right(), n2.left(), chain_y + node_h / 2.0);

    // remove background
    node_box(ui, n2, "RemoveBackground", "去背景 · bria-rmbg", widgets::Tone::Teal, false);
    status_dot_label(ui, n2, "done", color::teal());
    arrow_between(ui, n2.right(), n3.left(), chain_y + node_h / 2.0);

    // multiview
    node_box(ui, n3, "MultiViewPose", "4 视图", widgets::Tone::Amber, true);
    status_dot_label(ui, n3, "run", color::amber());
    arrow_between(ui, n3.right(), n4.left(), chain_y + node_h / 2.0);

    // save
    node_box(ui, n4, "Save", "2×2 / per-view", widgets::Tone::Gray, false);
    status_dot_label(ui, n4, "idle", color::text3());

    // parallel branch (dashed connector + node)
    let branch_y = chain_y + node_h + 44.0;
    let branch_x = n2_x + 20.0;
    let branch_w = 200.0;
    ui.painter().line_segment(
        [egui::pos2(n1_x + node_w / 2.0, n1.bottom()), egui::pos2(n1_x + node_w / 2.0, branch_y + node_h / 2.0)],
        Stroke::new(1.6, color::text3()),
    );
    ui.painter().line_segment(
        [egui::pos2(n1_x + node_w / 2.0, branch_y + node_h / 2.0), egui::pos2(branch_x, branch_y + node_h / 2.0)],
        Stroke::new(1.6, color::text3()),
    );
    ui.painter().text(egui::pos2(branch_x - 8.0, branch_y - 8.0), Align2::RIGHT_CENTER, "parallel branch", FontId::new(9.5, FontFamily::Proportional), color::text3());
    let b1 = Rect::from_min_size(egui::pos2(branch_x, branch_y), egui::vec2(branch_w, node_h));
    node_box(ui, b1, "QwenImagePipeline", "电影画质 · parallel", widgets::Tone::Indigo, false);
    status_dot_label(ui, b1, "idle", color::text3());
    let b2 = Rect::from_min_size(egui::pos2(branch_x + branch_w + 40.0, branch_y), egui::vec2(160.0, node_h));
    node_box(ui, b2, "Save", "cinematic still", widgets::Tone::Gray, false);
    status_dot_label(ui, b2, "idle", color::text3());

    // right action column
    ui.painter().line_segment(
        [egui::pos2(right_col_x - 18.0, graph_rect.top() + 40.0), egui::pos2(right_col_x - 18.0, graph_rect.bottom() - 28.0)],
        Stroke::new(1.4, color::border()),
    );
    ui.painter().text(egui::pos2(right_col_x, graph_rect.top() + 70.0), Align2::LEFT_CENTER, "Run Combined Flow", FontId::new(13.5, FontFamily::Proportional), color::text());
    ui.painter().text(egui::pos2(right_col_x, graph_rect.top() + 92.0), Align2::LEFT_CENTER, "去背景 → 4 视图 · cinematic in parallel", FontId::new(9.5, FontFamily::Proportional), color::text2());
    ui.painter().text(egui::pos2(right_col_x, graph_rect.top() + 108.0), Align2::LEFT_CENTER, "single graph · 4 steps · shared source", FontId::new(9.0, FontFamily::Proportional), color::text3());

    // Run Combined button
    let btn_rect = Rect::from_min_size(egui::pos2(right_col_x, graph_rect.top() + 128.0), egui::vec2(graph_rect.right() - right_col_x - 22.0, 44.0));
    let btn_resp = ui.interact(btn_rect, ui.id().with("run-combined"), egui::Sense::click());
    let hover = btn_resp.hovered();
    ui.painter().rect_filled(btn_rect, 8.0f32, if hover { color::indigo().linear_multiply(1.1) } else { color::indigo() });
    ui.painter().text(btn_rect.center(), Align2::CENTER_CENTER, "▶  Run pipeline", FontId::new(12.5, FontFamily::Proportional), color::on_accent());
    if btn_resp.clicked() {
        let cinematic = crate::workflows::CinematicParams {
            image: image.clone(),
            prompt: crate::workflows::DEFAULT_CINE_PROMPT.to_string(),
            negative_prompt: crate::workflows::DEFAULT_CINE_NEGATIVE.to_string(),
            model: "Qwen-Image-2.1".into(),
            cfg: 6.5,
            steps: 40,
            width: 1344,
            height: 768,
            seed: 42,
            prefix: "h3_cinematic".into(),
        };
        let multi = crate::workflows::MultiViewParams {
            image: image.clone(),
            instructions: crate::workflows::DEFAULT_MULTI_INSTRUCTIONS.into(),
            subject_guidance: crate::workflows::DEFAULT_MULTI_GUIDANCE.into(),
            lora: crate::workflows::DEFAULT_LORA.into(),
            lora_strength: 0.0,
            negative_prompt: String::new(),
            model: "Qwen-Image-Edit-2511".into(),
            cfg: 2.5,
            steps: 40,
            width: 1226,
            height: 832,
            seed: 42,
            prefix: "h3_4view".into(),
            save_as: crate::workflows::SaveAs::Merged,
        };
        let graph = crate::workflows::combined_workflow(&cinematic, &multi);
        app.submit_workflow(graph, "Combined Pipeline");
    }
    ui.painter().text(egui::pos2(right_col_x, graph_rect.top() + 184.0), Align2::LEFT_CENTER, "or open a step independently ↓", FontId::new(9.5, FontFamily::Proportional), color::text3());

    // ── 3 step cards ──
    ui.add_space(12.0);
    let card_w = (ui.available_width() - 2.0 * 14.0) / 3.0;
    ui.horizontal(|ui| {
        draw_step_card(app, ui, card_w, 280.0, "Remove Background", "12-remove-background", "front_cutout.png", "transparent PNG · 1024²", color::teal(), true, Screen::RemoveBg);
        draw_step_card(app, ui, card_w, 280.0, "Cinematic from Reference", "13-cinematic-from-reference", "cinematic_bamboo.png", "1344×768 · still", color::indigo(), false, Screen::Cinematic);
        draw_step_card(app, ui, card_w, 280.0, "Multi-View Turnaround", "14-multi-view-turnaround", "4view_grid.png", "1226×832 · 2×2", color::amber(), true, Screen::MultiView);
    });
}

fn status_dot_label(ui: &mut egui::Ui, node: Rect, text: &str, tone: Color32) {
    ui.painter().circle_filled(egui::pos2(node.left() + 14.0, node.top() + 12.0), 4.0, tone);
    ui.painter().text(egui::pos2(node.left() + 24.0, node.top() + 12.0), Align2::LEFT_CENTER, text, FontId::new(9.5, FontFamily::Proportional), tone);
}

fn draw_step_card(
    app: &mut H3App,
    ui: &mut egui::Ui,
    w: f32,
    h: f32,
    title: &str,
    filename: &str,
    output_name: &str,
    caption: &str,
    accent: Color32,
    running: bool,
    screen: Screen,
) {
    let (rect, response) = ui.allocate_exact_size(egui::vec2(w, h), egui::Sense::click());
    ui.painter().rect_filled(rect, 12.0f32, color::panel());
    ui.painter().rect_stroke(rect, 12.0f32, Stroke::new(1.0, color::border()), StrokeKind::Middle);
    ui.painter().rect_filled(Rect::from_min_size(rect.min, egui::vec2(5.0, rect.height())), 0.0f32, accent);

    // preview area
    let preview = Rect::from_min_size(egui::pos2(rect.left() + 22.0, rect.top() + 20.0), egui::vec2(rect.width() - 42.0, 108.0));
    crate::placeholder::bamboo_scene(ui, preview, title.len() as u32 * 7 + 2);
    ui.painter().rect_stroke(preview, 10.0f32, Stroke::new(1.0, color::border()), StrokeKind::Middle);

    ui.painter().text(egui::pos2(rect.left() + 22.0, rect.top() + 144.0), Align2::LEFT_CENTER, title, FontId::new(14.5, FontFamily::Proportional), color::text());
    ui.painter().text(egui::pos2(rect.left() + 22.0, rect.top() + 164.0), Align2::LEFT_CENTER, filename, FontId::new(9.5, FontFamily::Monospace), color::text3());

    // status pill
    let pill_rect = Rect::from_min_size(egui::pos2(rect.right() - 96.0, rect.top() + 140.0), egui::vec2(74.0, 24.0));
    let (pill_tone, pill_label) = if running { (color::amber(), "run") } else { (color::text3(), "idle") };
    ui.painter().rect_filled(pill_rect, 12.0f32, pill_tone.linear_multiply(0.22));
    ui.painter().rect_stroke(pill_rect, 12.0f32, Stroke::new(1.0, pill_tone.linear_multiply(0.5)), StrokeKind::Middle);
    ui.painter().text(pill_rect.center(), Align2::CENTER_CENTER, pill_label, FontId::new(9.5, FontFamily::Proportional), pill_tone);

    ui.painter().text(egui::pos2(rect.left() + 22.0, rect.top() + 186.0), Align2::LEFT_CENTER, "OUTPUT", FontId::new(9.5, FontFamily::Proportional), color::text3());
    ui.painter().text(egui::pos2(rect.right() - 22.0, rect.top() + 186.0), Align2::RIGHT_CENTER, output_name, FontId::new(10.5, FontFamily::Monospace), color::text());
    ui.painter().text(egui::pos2(rect.left() + 22.0, rect.top() + 208.0), Align2::LEFT_CENTER, caption, FontId::new(10.5, FontFamily::Proportional), color::text2());

    // open button
    let btn_rect = Rect::from_min_size(egui::pos2(rect.left() + 22.0, rect.bottom() - 38.0), egui::vec2(rect.width() - 44.0, 32.0));
    let btn_resp = ui.interact(btn_rect, ui.id().with(("open-step", title)), egui::Sense::click());
    let hover = btn_resp.hovered();
    ui.painter().rect_filled(btn_rect, 8.0f32, if hover { color::card() } else { Color32::TRANSPARENT });
    ui.painter().rect_stroke(btn_rect, 8.0f32, Stroke::new(1.0, accent.linear_multiply(0.5)), StrokeKind::Middle);
    ui.painter().text(btn_rect.center(), Align2::CENTER_CENTER, "Open step →", FontId::new(11.5, FontFamily::Proportional), accent);

    if btn_resp.clicked() || response.clicked() {
        app.nav_to(screen);
    }
}

//! Minimal 20×20 line-art icons drawn with egui shapes.

use egui::{pos2, Rect, Stroke, StrokeKind, Color32, Painter};
use crate::app::NavTab;

pub fn draw_icon(painter: &Painter, tab: NavTab, cx: f32, cy: f32, color: Color32) {
    let s = Stroke::new(1.6, color);
    let half = 8.0;
    let x0 = cx - half; let y0 = cy - half;
    let x1 = cx + half; let y1 = cy + half;
    let r = 4.0;

    match tab {
        NavTab::Home => {
            // roof + body
            painter.line_segment([pos2(x0 + 1.0, y0 + 8.5), pos2(cx, y0 + 1.0)], s);
            painter.line_segment([pos2(cx, y0 + 1.0), pos2(x1 - 1.0, y0 + 8.5)], s);
            painter.rect_stroke(Rect::from_min_max(pos2(x0 + 3.5, y0 + 9.5), pos2(x1 - 3.5, y1 - 1.0)), r, s, StrokeKind::Middle);
        }
        NavTab::Projects => {
            // folder
            painter.rect_stroke(Rect::from_min_max(pos2(x0 + 0.5, y0 + 6.5), pos2(x1 - 0.5, y1 - 1.5)), r, s, StrokeKind::Middle);
            painter.line_segment([pos2(x0 + 1.0, y0 + 6.5), pos2(x0 + 1.0, y0 + 3.5)], s);
            painter.line_segment([pos2(x0 + 1.0, y0 + 3.5), pos2(x0 + 8.0, y0 + 3.5)], s);
            painter.line_segment([pos2(x0 + 8.0, y0 + 3.5), pos2(x1 - 2.5, y0 + 6.5)], s);
        }
        NavTab::Storyboard => {
            // film strip
            painter.rect_stroke(Rect::from_min_max(pos2(x0, y0 + 2.0), pos2(x1, y1 - 2.0)), 2.0, s, StrokeKind::Middle);
            for i in 0..3 {
                let y = y0 + 4.0 + i as f32 * 4.4;
                painter.line_segment([pos2(x0 + 6.5, y), pos2(x1 - 6.5, y)], s);
            }
            painter.rect_filled(Rect::from_min_max(pos2(x0 + 1.5, y0 + 3.5), pos2(x0 + 5.0, y0 + 5.0)), 0.0, color);
            painter.rect_filled(Rect::from_min_max(pos2(x0 + 1.5, y0 + 7.9), pos2(x0 + 5.0, y0 + 9.4)), 0.0, color);
            painter.rect_filled(Rect::from_min_max(pos2(x0 + 1.5, y0 + 12.3), pos2(x0 + 5.0, y0 + 13.8)), 0.0, color);
        }
        NavTab::Characters => {
            // person
            painter.circle_stroke(pos2(cx, cy - 4.0), 4.0, s);
            painter.line_segment([pos2(cx - 6.0, y1 - 1.5), pos2(cx + 6.0, y1 - 1.5)], s);
            painter.rect_stroke(Rect::from_min_max(pos2(cx - 6.0, cy + 1.5), pos2(cx + 6.0, y1 - 1.0)), 3.0, s, StrokeKind::Middle);
        }
        NavTab::Assets => {
            // stacked layers
            painter.rect_stroke(Rect::from_min_max(pos2(x0 + 2.0, y0 + 5.5), pos2(x1 - 2.0, y1 - 3.5)), 2.0, s, StrokeKind::Middle);
            painter.line_segment([pos2(x0 + 2.0, y0 + 4.0), pos2(x1 - 2.0, y0 + 4.0)], s);
            painter.line_segment([pos2(x0 + 3.5, y0 + 1.5), pos2(x1 - 3.5, y0 + 1.5)], s);
            painter.line_segment([pos2(x0 + 3.5, y0 + 1.5), pos2(x0 + 2.0, y0 + 5.5)], s);
            painter.line_segment([pos2(x1 - 3.5, y0 + 1.5), pos2(x1 - 2.0, y0 + 5.5)], s);
        }
        NavTab::Studio => {
            // play on monitor
            painter.rect_stroke(Rect::from_min_max(pos2(x0, y0 + 2.5), pos2(x1, y1 - 2.5)), 2.5, s, StrokeKind::Middle);
            painter.line_segment([pos2(cx - 2.5, cy - 2.5), pos2(cx - 2.5, cy + 2.5)], s);
            painter.line_segment([pos2(cx - 2.5, cy), pos2(cx + 3.0, cy - 2.5)], s);
            painter.line_segment([pos2(cx - 2.5, cy), pos2(cx + 3.0, cy + 2.5)], s);
            painter.line_segment([pos2(cx - 3.0, y1 - 0.5), pos2(cx + 3.5, y1 - 0.5)], s);
        }
        NavTab::Queue => {
            // list
            painter.line_segment([pos2(x0 + 4.5, y0 + 3.5), pos2(x1 - 1.5, y0 + 3.5)], s);
            painter.line_segment([pos2(x0 + 4.5, y0 + 9.0), pos2(x1 - 1.5, y0 + 9.0)], s);
            painter.line_segment([pos2(x0 + 4.5, y0 + 14.5), pos2(x1 - 1.5, y0 + 14.5)], s);
            painter.circle_filled(pos2(x0 + 1.5, y0 + 3.5), 1.5, color);
            painter.circle_filled(pos2(x0 + 1.5, y0 + 9.0), 1.5, color);
            painter.circle_filled(pos2(x0 + 1.5, y0 + 14.5), 1.5, color);
        }
        NavTab::Review => {
            // clapper/film
            painter.rect_stroke(Rect::from_min_max(pos2(x0 + 1.0, y0 + 5.0), pos2(x1 - 1.0, y1 - 1.5)), 2.0, s, StrokeKind::Middle);
            for i in 0..3 {
                let x = x0 + 2.5 + i as f32 * 4.6;
                painter.line_segment([pos2(x, y0 + 1.5), pos2(x + 3.0, y0 + 5.5)], s);
            }
            painter.line_segment([pos2(x0 + 1.0, y0 + 5.0), pos2(x1 - 1.0, y0 + 5.0)], s);
        }
        NavTab::Export => {
            // download tray
            painter.line_segment([pos2(cx, y0 + 1.5), pos2(cx, y1 - 5.5)], s);
            painter.line_segment([pos2(cx - 4.5, y1 - 8.5), pos2(cx, y1 - 4.5)], s);
            painter.line_segment([pos2(cx + 4.5, y1 - 8.5), pos2(cx, y1 - 4.5)], s);
            painter.line_segment([pos2(x0 + 1.0, y1 - 1.0), pos2(x1 - 1.0, y1 - 1.0)], s);
        }
        NavTab::Gpu => {
            // chip
            painter.rect_stroke(Rect::from_min_max(pos2(x0 + 2.5, y0 + 2.5), pos2(x1 - 2.5, y1 - 2.5)), 2.0, s, StrokeKind::Middle);
            painter.rect_stroke(Rect::from_min_max(pos2(cx - 3.5, cy - 3.5), pos2(cx + 3.5, cy + 3.5)), 1.0, s, StrokeKind::Middle);
            for i in 0..3 {
                let dx = (i as f32 - 1.0) * 4.5;
                painter.line_segment([pos2(cx + dx, y0 + 0.5), pos2(cx + dx, y0 + 2.5)], s);
                painter.line_segment([pos2(cx + dx, y1 - 2.5), pos2(cx + dx, y1 - 0.5)], s);
                painter.line_segment([pos2(x0 + 0.5, cy + dx), pos2(x0 + 2.5, cy + dx)], s);
                painter.line_segment([pos2(x1 - 2.5, cy + dx), pos2(x1 - 0.5, cy + dx)], s);
            }
        }
    }
}

/// Trash-can line icon (~14px), centered at (cx, cy).
pub fn draw_trash(painter: &Painter, cx: f32, cy: f32, color: Color32) {
    let s = Stroke::new(1.5, color);
    let top = cy - 6.0;
    // lid handle
    painter.line_segment([pos2(cx - 3.0, top), pos2(cx + 3.0, top)], s);
    // lid
    painter.line_segment([pos2(cx - 5.5, top + 2.0), pos2(cx + 5.5, top + 2.0)], s);
    // body (slightly tapered)
    let body_l = cx - 4.5;
    let body_r = cx + 4.5;
    let body_t = top + 2.0;
    let body_b = cy + 6.0;
    painter.line_segment([pos2(body_l, body_t), pos2(body_l + 1.0, body_b)], s);
    painter.line_segment([pos2(body_r, body_t), pos2(body_r - 1.0, body_b)], s);
    painter.line_segment([pos2(body_l + 1.0, body_b), pos2(body_r - 1.0, body_b)], s);
    // inner stripes
    painter.line_segment([pos2(cx - 1.5, top + 4.5), pos2(cx - 1.0, body_b - 1.5)], s);
    painter.line_segment([pos2(cx + 1.5, top + 4.5), pos2(cx + 1.0, body_b - 1.5)], s);
}

/// 4-tile toggle box (compact/expanded, cosmetic).
pub fn draw_toggle_at(ui: &mut egui::Ui, rect: egui::Rect) {
    let painter = ui.painter();
    painter.rect_filled(rect, 6.0f32, Color32::from_rgb(0x28, 0x2D, 0x34));
    painter.rect_stroke(rect, 6.0f32, Stroke::new(1.0, Color32::from_rgb(0x34, 0x3A, 0x43)), StrokeKind::Middle);
    // 4 small tiles
    let tile = 4.0;
    let off = 6.0;
    for dx in [0, 1] {
        for dy in [0, 1] {
            let r = Rect::from_min_size(
                egui::pos2(rect.left() + off + dx as f32 * 10.0, rect.top() + 5.0 + dy as f32 * 8.0),
                egui::vec2(tile, tile),
            );
            painter.rect_filled(r, 1.5f32, Color32::from_rgb(0x8B, 0x9D, 0xFF));
        }
    }
}

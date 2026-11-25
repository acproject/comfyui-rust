//! Canvas painters mirroring the SVG design motifs (bamboo scene, silhouette,
//! checkerboard transparency). Pure egui painting — no external assets.

use egui::{Color32, Pos2, Rect, Stroke, StrokeKind, Align2, FontId, FontFamily};

/// Deterministic pseudo-random from a seed (simple LCG, returns 0..1).
fn prand(seed: u32) -> f32 {
    let mut x = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
    x ^= x >> 13;
    x = x.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
    (x & 0xFFFF) as f32 / 65_535.0
}

/// Draw a bamboo-forest scene (dark vignette + silhouette + teal stalks).
pub fn bamboo_scene(ui: &mut egui::Ui, rect: Rect, seed: u32) {
    let painter = ui.painter();
    painter.rect_filled(rect, 8.0, Color32::from_rgb(0x0B, 0x12, 0x19));
    // faint vertical gradient bands
    let steps = 8;
    for i in 0..steps {
        let t = i as f32 / steps as f32;
        let y = rect.top() + rect.height() * t;
        let band_h = rect.height() / steps as f32 + 1.0;
        let col = Color32::from_rgba_premultiplied(0x11, 0x13, 0x15, (22.0 * (1.0 - t)) as u8);
        painter.rect_filled(Rect::from_min_size(Pos2::new(rect.left(), y), egui::vec2(rect.width(), band_h)), 0.0, col);
    }
    // teal bamboo stalks + leaf rings
    let stalks = [0.06, 0.14, 0.46, 0.78, 0.92];
    for (i, fx) in stalks.iter().enumerate() {
        let x = rect.left() + fx * rect.width();
        let top = rect.top() + rect.height() * (0.12 + prand(seed + i as u32) * 0.22);
        painter.line_segment(
            [Pos2::new(x, top), Pos2::new(x, rect.bottom())],
            Stroke::new(1.6, Color32::from_rgba_premultiplied(0x45, 0xC6, 0xB0, 88)),
        );
        let ring_y = top + prand(seed + i as u32 + 100) * rect.height() * 0.16;
        ui.painter().add(egui::epaint::Shape::Ellipse(egui::epaint::EllipseShape::stroke(
            Pos2::new(x, ring_y),
            egui::vec2(11.0, 2.8),
            Stroke::new(1.4, Color32::from_rgba_premultiplied(0x45, 0xC6, 0xB0, 74)),
        )));
    }
    // dark silhouette figure (head + shoulders)
    let hx = rect.center().x + rect.width() * 0.05;
    let hy = rect.center().y - rect.height() * 0.12;
    painter.circle_filled(Pos2::new(hx, hy), 10.0, Color32::from_rgb(0x0B, 0x12, 0x19));
    let shoulder = Rect::from_min_size(Pos2::new(hx - 19.0, hy + 14.0), egui::vec2(38.0, 24.0));
    painter.rect_filled(shoulder, 12.0, Color32::from_rgba_premultiplied(0x0B, 0x12, 0x19, 235));
    painter.line_segment(
        [Pos2::new(hx - 12.0, hy + 20.0), Pos2::new(hx + 12.0, hy + 20.0)],
        Stroke::new(2.0, Color32::from_rgba_premultiplied(0x45, 0xC6, 0xB0, 190)),
    );
}

/// Draw a simple avatar silhouette (head + shoulders) with a colored ring.
pub fn silhouette(ui: &mut egui::Ui, rect: Rect, accent: Color32) {
    let painter = ui.painter();
    painter.rect_filled(rect, 10.0, Color32::from_rgb(0x15, 0x19, 0x1E));
    painter.rect_stroke(rect, 10.0, Stroke::new(1.0, Color32::from_rgb(0x34, 0x3A, 0x43)), StrokeKind::Middle);
    let cx = rect.center().x;
    let cy = rect.center().y - rect.height() * 0.08;
    painter.circle_filled(Pos2::new(cx, cy), rect.height() * 0.26, Color32::from_rgb(0x0B, 0x12, 0x19));
    painter.circle_stroke(Pos2::new(cx, cy), rect.height() * 0.26, Stroke::new(1.6, accent));
    let sh = Rect::from_min_size(
        Pos2::new(cx - rect.width() * 0.27, cy + rect.height() * 0.2),
        egui::vec2(rect.width() * 0.54, rect.height() * 0.34),
    );
    painter.rect_filled(sh, 14.0, Color32::from_rgb(0x0B, 0x12, 0x19));
}

/// Draw a transparency checkerboard pattern.
pub fn checkerboard(ui: &mut egui::Ui, rect: Rect, cell: f32, dark: Color32, light: Color32) {
    let painter = ui.painter();
    painter.rect_filled(rect, 8.0, dark);
    let cols = (rect.width() / cell).ceil() as usize;
    let rows = (rect.height() / cell).ceil() as usize;
    for i in 0..cols {
        for j in 0..rows {
            if (i + j) % 2 == 0 {
                let x0 = rect.left() + i as f32 * cell;
                let y0 = rect.top() + j as f32 * cell;
                let w = cell.min(rect.right() - x0);
                let h = cell.min(rect.bottom() - y0);
                painter.rect_filled(Rect::from_min_size(Pos2::new(x0, y0), egui::vec2(w, h)), 0.0, light);
            }
        }
    }
}

/// Small label centered inside a well.
pub fn well_caption(ui: &mut egui::Ui, rect: Rect, text: &str, color: Color32) {
    ui.painter().text(
        rect.center(),
        Align2::CENTER_CENTER,
        text,
        FontId::new(11.0, FontFamily::Proportional),
        color,
    );
}

/// Draw a play button (circle + triangle).
pub fn play_button(ui: &mut egui::Ui, center: Pos2, radius: f32, accent: Color32) {
    let painter = ui.painter();
    painter.circle_filled(center, radius, Color32::from_rgba_premultiplied(0x0B, 0x12, 0x19, 220));
    painter.circle_stroke(center, radius, Stroke::new(2.0, accent));
    let r = radius * 0.4;
    let p = [
        Pos2::new(center.x - r * 0.55, center.y - r),
        Pos2::new(center.x - r * 0.55, center.y + r),
        Pos2::new(center.x + r * 0.95, center.y),
    ];
    ui.painter().add(egui::epaint::Shape::Path(egui::epaint::PathShape::convex_polygon(
        p.to_vec(),
        accent,
        Stroke::new(1.0, accent),
    )));
}

//! Character library — 4 accent-coded cards with consistency bars.

use egui::{Align2, Color32, FontId, FontFamily, Rect, Stroke, StrokeKind};
use crate::app::{H3App, Screen};
use crate::theme::{color, typography};
use crate::widgets;

pub fn draw(app: &mut H3App, ui: &mut egui::Ui) {
    use crate::i18n::{tr, tf};
    let lang = app.lang();
    // Intro line
    ui.colored_label(color::text3(), tr(lang, "Reusable identity across projects · click a card to open the full asset panel"));
    ui.add_space(14.0);

    let n = app.local.characters.len();
    let card_w = (ui.available_width() - (n.saturating_sub(1) as f32 * 12.0)) / n as f32;

    let mut clicked = None;
    ui.horizontal_top(|ui| {
    for (i, c) in app.local.characters.iter().enumerate() {
        let accent = accent_of(&c.accent);
        let (rect, response) = ui.allocate_exact_size(egui::vec2(card_w, 330.0), egui::Sense::click());
        ui.painter().rect_filled(rect, 14.0f32, color::panel());
        ui.painter().rect_stroke(rect, 14.0f32, Stroke::new(1.0, color::border()), StrokeKind::Middle);
        ui.painter().rect_filled(Rect::from_min_size(rect.min, egui::vec2(5.0, rect.height())), 0.0f32, accent);

        // avatar well
        let well = Rect::from_min_size(egui::pos2(rect.left() + 22.0, rect.top() + 20.0), egui::vec2(card_w - 44.0, 118.0));
        ui.painter().rect_filled(well, 10.0f32, color::well());
        ui.painter().rect_stroke(well, 10.0f32, Stroke::new(1.0, color::border()), StrokeKind::Middle);
        crate::placeholder::silhouette(ui, well, accent);
        ui.painter().text(egui::pos2(well.center().x, well.bottom() - 14.0), Align2::CENTER_CENTER, tr(lang, "identity · front"), FontId::new(9.5, FontFamily::Monospace), color::text3());

        // text block
        let tx = rect.left() + 22.0;
        let n_projects = (c.refs as f32 / 4.0).round() as u32 + 1;
        ui.painter().text(egui::pos2(tx, well.bottom() + 26.0), Align2::LEFT_CENTER, &c.name, FontId::new(14.5, FontFamily::Proportional), color::text());
        ui.painter().text(egui::pos2(tx, well.bottom() + 46.0), Align2::LEFT_CENTER, &format!("ID · {} · v{}", c.id, 1 + i % 3), FontId::new(9.5, FontFamily::Monospace), color::text3());
        ui.painter().text(egui::pos2(tx, well.bottom() + 63.0), Align2::LEFT_CENTER, &tf(lang, "{} projects · {} shots", &[&n_projects, &c.refs]), FontId::new(10.5, FontFamily::Proportional), color::text2());
        ui.painter().line_segment([egui::pos2(rect.left() + 22.0, well.bottom() + 82.0), egui::pos2(rect.right() - 22.0, well.bottom() + 82.0)], Stroke::new(1.0, color::border()));

        ui.painter().text(egui::pos2(tx, well.bottom() + 100.0), Align2::LEFT_CENTER, tr(lang, "Face · Hair · Form"), FontId::new(10.0, FontFamily::Proportional), color::text2());
        ui.painter().text(egui::pos2(tx, well.bottom() + 128.0), Align2::LEFT_CENTER, &tf(lang, "consistency {} · {}", &[&format!("{:.0}%", c.consistency), &c.updated_at]), FontId::new(10.0, FontFamily::Proportional), color::text2());

        // consistency bar
        let bar = Rect::from_min_size(egui::pos2(rect.left() + 22.0, rect.bottom() - 20.0), egui::vec2(rect.width() - 44.0, 6.0));
        widgets::progress_bar(ui, bar, c.consistency / 100.0, accent, 6.0);

        if response.clicked() {
            clicked = Some(c.id.clone());
        }
        ui.add_space(12.0);
    }
    });

    if clicked.is_some() {
        app.nav_to(Screen::Assets);
    }
}

pub fn accent_of(accent: &str) -> Color32 {
    match accent {
        "teal" => color::teal(),
        "indigo" => color::indigo(),
        "amber" => color::amber(),
        "purple" => color::purple(),
        _ => color::text2(),
    }
}

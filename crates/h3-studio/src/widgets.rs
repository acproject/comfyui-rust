//! Reusable design-system widgets — panels, chips, buttons, bars, labels.

use egui::{Align2, Color32, FontId, FontFamily, Layout, Rect, Stroke, StrokeKind};
use crate::theme::{color, typography};

#[derive(Clone, Copy, PartialEq)]
pub enum Tone {
    Indigo, Teal, Amber, Purple, Red, Gray,
}

impl Tone {
    pub fn accent(self) -> Color32 {
        match self {
            Tone::Indigo => color::indigo(),
            Tone::Teal => color::teal(),
            Tone::Amber => color::amber(),
            Tone::Purple => color::purple(),
            Tone::Red => color::red(),
            Tone::Gray => color::text2(),
        }
    }
    pub fn dim(self) -> Color32 {
        match self {
            Tone::Indigo => color::indigo_dim(),
            Tone::Teal => color::teal_dim(),
            Tone::Amber => color::amber_dim(),
            Tone::Purple => color::purple_dim(),
            Tone::Red => color::red_dim(),
            Tone::Gray => color::gray_dim(),
        }
    }
}

/// Section label — uppercase, letterspaced (emulated).
pub fn section_label(ui: &mut egui::Ui, text: &str) {
    ui.colored_label(color::text2(), egui::RichText::new(text.to_uppercase()).size(typography::TINY).strong());
    ui.add_space(4.0);
}

/// Section label + right-aligned meta.
pub fn section_label_with_meta(ui: &mut egui::Ui, text: &str, meta: &str) {
    ui.horizontal(|ui| {
        ui.colored_label(color::text2(), egui::RichText::new(text.to_uppercase()).size(typography::TINY).strong());
        ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
            ui.colored_label(color::text3(), meta);
        });
    });
    ui.add_space(6.0);
}

/// Estimate text width in pixels for pill sizing (Lato ~0.52em, CJK 1.0em).
pub fn est_text_width(text: &str, size: f32) -> f32 {
    text.chars().map(|c| if c.is_ascii() { 0.52 } else { 1.0 } * size).sum::<f32>()
}

/// A pill chip with accent-tinted fill + colored text.
pub fn chip(ui: &mut egui::Ui, text: &str, tone: Tone, height: f32) -> egui::Response {
    let text_size = 11.0;
    let accent = tone.accent();
    let w = est_text_width(text, text_size) + 24.0;
    let (rect, response) = ui.allocate_exact_size(egui::vec2(w.max(40.0), height), egui::Sense::hover());
    let painter = ui.painter();
    painter.rect_filled(rect, height / 2.0, tone.dim());
    painter.rect_stroke(
        rect,
        height / 2.0,
        Stroke::new(1.0, accent.linear_multiply(0.45)),
        StrokeKind::Middle,
    );
    painter.text(
        rect.center(),
        Align2::CENTER_CENTER,
        text,
        FontId::new(text_size, FontFamily::Proportional),
        accent,
    );
    response
}

fn button(ui: &mut egui::Ui, text: &str, w: f32, h: f32, fill: Color32, text_color: Color32, outline: bool) -> bool {
    let (rect, response) = ui.allocate_exact_size(egui::vec2(w, h), egui::Sense::click());
    let painter = ui.painter();
    let hover = response.hovered();
    let pressed = response.dragged();
    if outline {
        let hover_col = if hover { color::indigo() } else { color::border() };
        painter.rect_filled(rect, 8.0, if hover { color::card() } else { Color32::TRANSPARENT });
        painter.rect_stroke(rect, 8.0, Stroke::new(if hover { 1.4 } else { 1.0 }, hover_col), StrokeKind::Middle);
    } else {
        let fill = if pressed { fill.linear_multiply(0.85) } else if hover { fill.linear_multiply(1.12) } else { fill };
        painter.rect_filled(rect, 8.0, fill);
        painter.rect_stroke(rect, 8.0, Stroke::new(1.0, fill.linear_multiply(0.9)), StrokeKind::Middle);
    }
    painter.text(
        rect.center(),
        Align2::CENTER_CENTER,
        text,
        FontId::new(12.0, FontFamily::Proportional),
        text_color,
    );
    response.clicked()
}

pub fn primary_button(ui: &mut egui::Ui, text: &str, w: f32, h: f32) -> bool {
    button(ui, text, w, h, color::indigo(), color::on_accent(), false)
}
pub fn teal_button(ui: &mut egui::Ui, text: &str, w: f32, h: f32) -> bool {
    button(ui, text, w, h, color::teal(), color::on_teal(), false)
}
pub fn amber_button(ui: &mut egui::Ui, text: &str, w: f32, h: f32) -> bool {
    button(ui, text, w, h, color::amber(), color::on_amber(), false)
}
pub fn outline_button(ui: &mut egui::Ui, text: &str, w: f32, h: f32) -> bool {
    button(ui, text, w, h, Color32::TRANSPARENT, color::text(), true)
}
pub fn outline_button_indigo(ui: &mut egui::Ui, text: &str, w: f32, h: f32) -> bool {
    // Secondary/topbar action: outline with white label (design system secondary
    // button + screen mockups both use #F0F1F3 text); hover tints border indigo.
    button(ui, text, w, h, Color32::TRANSPARENT, color::text(), true)
}
pub fn ghost_button(ui: &mut egui::Ui, text: &str, w: f32, h: f32) -> bool {
    button(ui, text, w, h, color::card(), color::text(), false)
}

/// Progress bar with accent fill.
pub fn progress_bar(ui: &mut egui::Ui, rect: Rect, frac: f32, fill: Color32, height: f32) {
    let painter = ui.painter();
    painter.rect_filled(rect, height / 2.0, color::card());
    let clamped = frac.clamp(0.0, 1.0);
    let fill_w = (rect.width() * clamped).max(height);
    painter.rect_filled(
        Rect::from_min_size(rect.min, egui::vec2(fill_w, height)),
        height / 2.0,
        fill,
    );
}

/// Status dot + label.
pub fn status_dot(ui: &mut egui::Ui, rect: Rect, dot: Color32, label: &str) {
    ui.painter().circle_filled(
        egui::Pos2::new(rect.left() + 4.0, rect.center().y),
        4.0,
        dot,
    );
    ui.painter().text(
        egui::Pos2::new(rect.left() + 14.0, rect.center().y),
        Align2::LEFT_CENTER,
        label,
        FontId::new(11.0, FontFamily::Proportional),
        color::text2(),
    );
}

pub fn mono_line(ui: &mut egui::Ui, text: &str, size: f32, color: Color32) {
    ui.colored_label(color, egui::RichText::new(text).monospace().size(size));
}

pub fn caption(ui: &mut egui::Ui, text: &str) {
    ui.colored_label(color::text2(), egui::RichText::new(text).size(typography::CAPTION));
}

/// Key/value row — label left, mono value right.
pub fn key_val(ui: &mut egui::Ui, label: &str, value: &str) {
    ui.horizontal(|ui| {
        ui.colored_label(color::text2(), label);
        ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
            ui.colored_label(color::text(), egui::RichText::new(value).monospace().size(typography::SMALL));
        });
    });
}

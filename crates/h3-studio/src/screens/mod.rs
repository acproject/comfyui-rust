//! Screen dispatch + shared screen helpers.

mod home;
mod project;
mod storyboard;
mod characters;
mod assets;
pub mod studio;
mod queue;
mod gpu_screen;
mod review;
mod export;
mod image_workflows;

use crate::app::{H3App, Screen};

/// Top-level dispatch on the current screen — every screen renders inside the
/// central panel's UI.
pub fn draw(app: &mut H3App, ui: &mut egui::Ui) {
    let scroll = egui::ScrollArea::both()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing = egui::vec2(8.0, 8.0);
            ui.add_space(metrics_gap());

            match app.screen {
                Screen::Home => home::draw(app, ui),
                Screen::Projects => project::draw(app, ui),
                Screen::Storyboard => storyboard::draw(app, ui),
                Screen::Characters => characters::draw(app, ui),
                Screen::Assets => assets::draw(app, ui),
                Screen::Studio => studio::draw(app, ui),
                Screen::Queue => queue::draw(app, ui),
                Screen::Gpu => gpu_screen::draw(app, ui),
                Screen::Review => review::draw(app, ui),
                Screen::Export => export::draw(app, ui),
                Screen::RemoveBg => image_workflows::remove_bg::draw(app, ui),
                Screen::Cinematic => image_workflows::cinematic::draw(app, ui),
                Screen::MultiView => image_workflows::multi_view::draw(app, ui),
                Screen::Combined => image_workflows::combined::draw(app, ui),
            }
        });
    let _ = scroll;
}

/// Top padding inside content area.
fn metrics_gap() -> f32 { 8.0 }

//! H3 Studio — desktop AI filmmaking workstation.
//! egui app shell; talks to the comfyui-rust server at 127.0.0.1:8188.

mod app;
mod client;
mod fonts;
mod gpu;
mod placeholder;
mod screens;
mod state;
mod theme;
mod ui_state;
mod workflows;
mod widgets;

fn main() -> eframe::Result<()> {
    let mut args = std::env::args().skip(1);
    let mut initial_screen = app::Screen::Home;
    let mut server_url = "http://127.0.0.1:8188".to_string();

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--screen" => {
                if let Some(name) = args.next() {
                    initial_screen = app::Screen::from_name(&name).unwrap_or(app::Screen::Home);
                }
            }
            "--url" => {
                if let Some(url) = args.next() { server_url = url; }
            }
            _ => {}
        }
    }

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1600.0, 940.0])
            .with_min_inner_size([1180.0, 700.0])
            .with_title("H3 Studio"),
        // Force the OpenGL (glow) renderer. The default Wgpu backend fails to
        // create a drawable surface in this sandboxed/Wayland environment,
        // yielding a blank (transparent) framebuffer.
        renderer: eframe::Renderer::Glow,
        ..Default::default()
    };

    eframe::run_native(
        "H3 Studio",
        options,
        Box::new(move |cc| {
            fonts::load_fonts(&cc.egui_ctx);
            let app = app::H3App::new(initial_screen, server_url);
            Ok(Box::new(app) as Box<dyn eframe::App>)
        }),
    )
}

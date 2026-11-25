//! Font loading for H3 Studio.
//!
//! Loads system-installed fonts at runtime so the binary stays small.
//! Lato (sans) + Droid Sans Fallback (CJK) for Latin/Chinese; Ubuntu Mono for mono.

use egui::{FontData, FontDefinitions, FontFamily};
use std::sync::Arc;

fn push_font(defs: &mut FontDefinitions, family: &str, path: &str) -> bool {
    match std::fs::read(path) {
        Ok(bytes) => {
            defs.font_data.insert(family.to_string(), Arc::new(FontData::from_owned(bytes)));
            true
        }
        Err(e) => {
            eprintln!("[h3-studio] font not loaded {}: {}", path, e.to_string());
            false
        }
    }
}

/// Install fonts on the egui context. Never panics — missing fonts fall back.
pub fn load_fonts(ctx: &egui::Context) {
    let mut defs = FontDefinitions::default();

    // Sans: Lato (Inter stand-in) — Latin + ASCII
    push_font(&mut defs, "lato", "/usr/share/fonts/truetype/lato/Lato-Regular.ttf");
    push_font(&mut defs, "lato-medium", "/usr/share/fonts/truetype/lato/Lato-Medium.ttf");
    push_font(&mut defs, "lato-semibold", "/usr/share/fonts/truetype/lato/Lato-Semibold.ttf");
    push_font(&mut defs, "lato-bold", "/usr/share/fonts/truetype/lato/Lato-Bold.ttf");

    // Mono: Ubuntu Mono (JetBrains Mono stand-in)
    push_font(&mut defs, "ubuntu-mono", "/usr/share/fonts/truetype/ubuntu/UbuntuMono-R.ttf");
    push_font(&mut defs, "ubuntu-mono-bold", "/usr/share/fonts/truetype/ubuntu/UbuntuMono-B.ttf");

    // CJK fallback: Droid Sans Fallback (plain TTF, full Mandarin coverage)
    push_font(&mut defs, "droid-cjk", "/usr/share/fonts/truetype/droid/DroidSansFallbackFull.ttf");

    // Families: primary first, CJK fallback second.
    defs.families.insert(
        FontFamily::Proportional,
        vec!["lato".to_string(), "droid-cjk".to_string()],
    );
    defs.families.insert(
        FontFamily::Monospace,
        vec!["ubuntu-mono".to_string(), "droid-cjk".to_string()],
    );

    ctx.set_fonts(defs);
}

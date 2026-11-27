//! H3 Studio design tokens — cinematic dark workstation palette.
//! Values mirror the SVG artboards in H3_Studio_Design.
//!
//! Canvas 1920×1080 · sidebar 240px · top bar 72px · bottom bar 56px.

pub mod color {
    use egui::Color32;

    /// Canvas background
    pub fn bg() -> Color32 { Color32::from_rgb(0x11, 0x13, 0x15) }
    /// Sidebar
    pub fn sidebar() -> Color32 { Color32::from_rgb(0x19, 0x1C, 0x20) }
    /// Top bar
    pub fn topbar() -> Color32 { Color32::from_rgb(0x1A, 0x1D, 0x22) }
    /// Panel / card surface
    pub fn panel() -> Color32 { Color32::from_rgb(0x20, 0x24, 0x2A) }
    /// Inner card (rows, lists)
    pub fn card() -> Color32 { Color32::from_rgb(0x28, 0x2D, 0x34) }
    /// Dark image well
    pub fn well() -> Color32 { Color32::from_rgb(0x15, 0x19, 0x1E) }
    /// Dark thumbnail fill (near-black image well)
    pub fn thumb() -> Color32 { Color32::from_rgb(0x0B, 0x12, 0x19) }
    /// Border
    pub fn border() -> Color32 { Color32::from_rgb(0x34, 0x3A, 0x43) }
    /// Border strong / divider
    pub fn divider() -> Color32 { Color32::from_rgb(0x34, 0x3A, 0x43) }

    /// Primary text
    pub fn text() -> Color32 { Color32::from_rgb(0xF0, 0xF1, 0xF3) }
    /// Secondary text
    pub fn text2() -> Color32 { Color32::from_rgb(0x96, 0x9D, 0xA8) }
    /// Muted hint
    pub fn text3() -> Color32 { Color32::from_rgb(0x6B, 0x72, 0x80) }

    /// Indigo accent (primary)
    pub fn indigo() -> Color32 { Color32::from_rgb(0x8B, 0x9D, 0xFF) }
    /// Teal accent
    pub fn teal() -> Color32 { Color32::from_rgb(0x45, 0xC6, 0xB0) }
    /// Amber accent
    pub fn amber() -> Color32 { Color32::from_rgb(0xE5, 0xAD, 0x59) }
    /// Purple accent
    pub fn purple() -> Color32 { Color32::from_rgb(0xB0, 0x7C, 0xE0) }
    /// Red / danger
    pub fn red() -> Color32 { Color32::from_rgb(0xE3, 0x6D, 0x6D) }

    /// Indigo translucent chip fill
    pub fn indigo_dim() -> Color32 { Color32::from_rgba_unmultiplied(0x8B, 0x9D, 0xFF, 26) }
    /// Indigo selected-row highlight (sidebar nav) — tuned so the displayed color
    /// matches the design-system source `fill="#8B9DFF" fill-opacity="0.10"`
    /// (which renders as ~#242936 over the sidebar bg). egui blends in linear
    /// space on an sRGB framebuffer, so a smaller alpha reproduces the same look.
    pub fn indigo_selected() -> Color32 { Color32::from_rgba_unmultiplied(0x8B, 0x9D, 0xFF, 3) }
    /// Teal translucent chip fill
    pub fn teal_dim() -> Color32 { Color32::from_rgba_unmultiplied(0x45, 0xC6, 0xB0, 26) }
    /// Amber translucent chip fill
    pub fn amber_dim() -> Color32 { Color32::from_rgba_unmultiplied(0xE5, 0xAD, 0x59, 26) }
    /// Purple translucent chip fill
    pub fn purple_dim() -> Color32 { Color32::from_rgba_unmultiplied(0xB0, 0x7C, 0xE0, 26) }
    /// Red translucent chip fill
    pub fn red_dim() -> Color32 { Color32::from_rgba_unmultiplied(0xE3, 0x6D, 0x6D, 26) }
    /// Gray chip fill
    pub fn gray_dim() -> Color32 { Color32::from_rgba_unmultiplied(0x96, 0x9D, 0xA8, 22) }

    /// Ink text on colored chips (near-black) — indigo / red accent fills
    pub fn on_accent() -> Color32 { Color32::from_rgb(0x11, 0x13, 0x15) }
    /// Ink text on teal accent fills (#0B1219, per design mockups)
    pub fn on_teal() -> Color32 { Color32::from_rgb(0x0B, 0x12, 0x19) }
    /// Ink text on amber accent fills (#1A1206, per design mockups)
    pub fn on_amber() -> Color32 { Color32::from_rgb(0x1A, 0x12, 0x06) }
}

pub mod metrics {
    /// Sidebar width
    pub const SIDEBAR: f32 = 240.0;
    /// Top bar height
    pub const TOPBAR: f32 = 72.0;
    /// Bottom status bar height
    pub const BOTTOMBAR: f32 = 56.0;

    /// Base spacing unit
    pub const SP_1: f32 = 4.0;
    pub const SP_2: f32 = 8.0;
    pub const SP_3: f32 = 12.0;
    pub const SP_4: f32 = 16.0;
    pub const SP_5: f32 = 24.0;
    pub const SP_6: f32 = 32.0;

    /// Border radius
    pub const R_SMALL: f32 = 6.0;
    pub const R_MED: f32 = 10.0;
    pub const R_LARGE: f32 = 12.0;
}

pub mod typography {
    pub const DISPLAY: f32 = 28.0;
    pub const H1: f32 = 21.0;
    pub const H2: f32 = 18.0;
    pub const BODY: f32 = 14.0;
    pub const CAPTION: f32 = 12.0;
    pub const SMALL: f32 = 11.0;
    pub const TINY: f32 = 10.0;
    pub const MONO: f32 = 13.0;
    pub const MONO_SMALL: f32 = 11.0;
}

//! H3 Studio — application shell: sidebar, top bar, status bar, screen router.

pub mod icons;

use egui::{Align2, Color32, FontId, FontFamily, Margin, Stroke, StrokeKind};
use crate::theme::{color, metrics, typography};
use crate::widgets;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Screen {
    Home,
    Projects,
    Storyboard,
    Characters,
    Assets,
    Studio,
    Queue,
    Gpu,
    Review,
    Export,
    RemoveBg,
    Cinematic,
    MultiView,
    Combined,
}

impl Screen {
    pub fn from_name(name: &str) -> Option<Screen> {
        let n = name.to_lowercase();
        Some(match n.as_str() {
            "home" => Screen::Home,
            "projects" | "project" => Screen::Projects,
            "storyboard" | "sb" => Screen::Storyboard,
            "characters" | "character" => Screen::Characters,
            "assets" | "reference" | "reference-assets" => Screen::Assets,
            "studio" | "generation" | "fl2va" | "ref2va" => Screen::Studio,
            "queue" => Screen::Queue,
            "gpu" | "settings" => Screen::Gpu,
            "review" => Screen::Review,
            "export" => Screen::Export,
            "remove-background" | "removebackground" | "removebg" | "bg" => Screen::RemoveBg,
            "cinematic" | "cine" => Screen::Cinematic,
            "multiview" | "multi-view" | "4view" | "turnaround" => Screen::MultiView,
            "combined" | "pipeline" | "chain" => Screen::Combined,
            _ => return None,
        })
    }

    pub fn title(&self) -> &'static str {
        match self {
            Screen::Home => "Home",
            Screen::Projects => "Bamboo Grove",
            Screen::Storyboard => "Storyboard",
            Screen::Characters => "Character Library",
            Screen::Assets => "Reference Assets",
            Screen::Studio => "Generation Studio",
            Screen::Queue => "Generation Queue",
            Screen::Gpu => "GPU Monitor",
            Screen::Review => "Video Review",
            Screen::Export => "Export",
            Screen::RemoveBg => "Remove Background",
            Screen::Cinematic => "Cinematic from Reference",
            Screen::MultiView => "Multi-View Turnaround",
            Screen::Combined => "Combined Pipeline",
        }
    }

    pub fn subtitle(&self) -> &'static str {
        match self {
            Screen::Home => "Local project library · 2 workers live · GPU 0/1/2",
            Screen::Projects => "Project · 12 scenes · 24 shots · 17 generated",
            Screen::Storyboard => "Bamboo Grove · Scene 01 · Rainy Bamboo Forest",
            Screen::Characters => "Reusable identity across projects · click a card to open the full asset panel",
            Screen::Assets => "Drag & drop to import · assign reference roles explicitly",
            Screen::Studio => "Bamboo Grove · S01 · Shot 003 — Rain walk through bamboo",
            Screen::Queue => "live · 3 running · 2 queued · 10 completed · 2 failed",
            Screen::Gpu => "3× NVIDIA V100 · separate memory pools",
            Screen::Review => "Bamboo Grove · S01 · Shot 002 · Sword drawn close-up · 4s",
            Screen::Export => "Bamboo Grove · 8 shots selected · project-level assembly",
            Screen::RemoveBg => "去背景 · isolate subject to transparent PNG · mock preview",
            Screen::Cinematic => "参卡图 → 电影画质 · reference-locked cinematic still · mock preview",
            Screen::MultiView => "人物 → 4 视图 · save merged 2×2 or each view individually · mock preview",
            Screen::Combined => "去背景 → 4 视图 chained · cinematic runs in parallel · mock overview",
        }
    }

    pub fn nav_tab(&self) -> Option<NavTab> {
        Some(match self {
            Screen::Home => NavTab::Home,
            Screen::Projects => NavTab::Projects,
            Screen::Storyboard => NavTab::Storyboard,
            Screen::Characters => NavTab::Characters,
            Screen::Assets => NavTab::Assets,
            Screen::Studio | Screen::RemoveBg | Screen::Cinematic | Screen::MultiView | Screen::Combined => NavTab::Studio,
            Screen::Queue => NavTab::Queue,
            Screen::Gpu => NavTab::Gpu,
            Screen::Review => NavTab::Review,
            Screen::Export => NavTab::Export,
        })
    }
}

/// Categorizes a toast so its title, accent color, and border match the
/// design system's state palette (indigo info / teal success / amber warning /
/// red error — see H3_Studio_Design/00-design-system.svg).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ToastKind {
    Info,
    Success,
    Warning,
    Error,
}

impl ToastKind {
    /// Accent color for the left bar, title, and (dimmed) border.
    pub fn accent(&self) -> Color32 {
        match self {
            ToastKind::Info => color::indigo(),
            ToastKind::Success => color::teal(),
            ToastKind::Warning => color::amber(),
            ToastKind::Error => color::red(),
        }
    }

    /// i18n key for the category title shown in the toast (提示/成功/警告/错误).
    pub fn title_key(&self) -> &'static str {
        match self {
            ToastKind::Info => "Info",
            ToastKind::Success => "Success",
            ToastKind::Warning => "Warning",
            ToastKind::Error => "Error",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum NavTab {
    Home, Projects, Storyboard, Characters, Assets, Studio, Queue, Review, Export, Gpu,
}

impl NavTab {
    pub fn label(&self) -> &'static str {
        match self {
            NavTab::Home => "Home",
            NavTab::Projects => "Projects",
            NavTab::Storyboard => "Storyboard",
            NavTab::Characters => "Characters",
            NavTab::Assets => "Reference Assets",
            NavTab::Studio => "Generation Studio",
            NavTab::Queue => "Generation Queue",
            NavTab::Review => "Video Review",
            NavTab::Export => "Export",
            NavTab::Gpu => "Model & GPU Settings",
        }
    }
}

/// What the background telemetry worker sends back.
#[derive(Default)]
pub struct PollResult {
    pub ok: bool,
    pub gpus: Vec<crate::gpu::GpuInfo>,
    pub host: crate::gpu::HostInfo,
    pub queue: serde_json::Value,
    pub history_recent: serde_json::Value,
    pub assets: serde_json::Value,
    pub input_images: Vec<String>,
}

/// Which UI preview slot a generated output filename should populate.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum OutputSlot { Cutout, Cinematic, Multiview, None }

/// Message from a background workflow submission.
#[derive(Default)]
pub struct JobResult {
    pub ok: bool,
    pub msg: String,
    /// Output filenames discovered by polling /history/{prompt_id}, mapped to UI slots.
    pub outputs: Vec<(OutputSlot, String)>,
}

#[derive(Default)]
pub struct AssetResult {
    pub key: String,
    pub bytes: Vec<u8>,
}

/// Pull every saved output filename out of a /history/{prompt_id} response
/// and classify it to a UI preview slot by its filename prefix.
fn extract_output_files(history: &serde_json::Value, prompt_id: &str) -> Option<Vec<(OutputSlot, String)>> {
    let outputs = history.get(prompt_id)?.get("outputs")?.as_object()?;
    let mut files = Vec::new();
    for (_node, node_out) in outputs {
        for key in ["images", "gifs", "videos"] {
            if let Some(arr) = node_out.get(key).and_then(|v| v.as_array()) {
                for item in arr {
                    if let Some(fname) = item.get("filename").and_then(|v| v.as_str()) {
                        let slot = classify_slot(fname);
                        if slot != OutputSlot::None {
                            files.push((slot, fname.to_string()));
                        }
                    }
                }
            }
        }
    }
    if files.is_empty() { None } else { Some(files) }
}

fn classify_slot(filename: &str) -> OutputSlot {
    if filename.starts_with("h3_bg") {
        OutputSlot::Cutout
    } else if filename.starts_with("h3_4view") {
        OutputSlot::Multiview
    } else if filename.starts_with("h3_cinematic") {
        OutputSlot::Cinematic
    } else {
        OutputSlot::None
    }
}

pub struct H3App {
    pub screen: Screen,
    pub server_url: String,
    pub client: crate::client::ComfyClient,
    pub local: crate::state::AppLocalState,

    pub gpus: Vec<crate::gpu::GpuInfo>,
    pub host: crate::gpu::HostInfo,
    pub connect_ok: bool,
    pub last_poll: std::time::Instant,
    pub queue: serde_json::Value,
    pub history_recent: serde_json::Value,
    pub input_images: Vec<String>,
    pub assets: serde_json::Value,

    /// Channel + sender used to deliver background telemetry/texture bytes.
    pub telemetry_rx: Option<std::sync::mpsc::Receiver<PollResult>>,
    pub telemetry_tx: Option<std::sync::mpsc::Sender<PollResult>>,
    pub texture_rx: Option<std::sync::mpsc::Receiver<AssetResult>>,
    pub texture_tx: Option<std::sync::mpsc::Sender<AssetResult>>,
    pub job_rx: Option<std::sync::mpsc::Receiver<JobResult>>,
    pub job_tx: Option<std::sync::mpsc::Sender<JobResult>>,

    /// Input image key → source description for the last generated/uploaded image.
    pub selected_input_image: Option<String>,
    pub last_cutout: Option<String>,
    pub last_cinematic: Option<String>,
    pub last_multiview: Option<String>,

    pub textures: TextureCache,
    pub pending_inputs: std::collections::HashSet<String>,
    pub toast: Option<(ToastKind, String, std::time::Instant)>,
    pub initialized: bool,
    pub multiview_save_as: crate::workflows::SaveAs,
    pub studio_mode: crate::screens::studio::StudioMode,
    pub studio_prompt: String,
    pub cine_prompt: String,

    /// Transient, in-memory UI selections/toggles (not persisted).
    pub ui: crate::ui_state::UiState,

    /// When set, the app captures its own window to this PNG after a few frames
    /// and exits. Used for headless screenshot verification (H3_SCREENSHOT_TO).
    pub screenshot_path: Option<String>,
    pub screenshot_taken: bool,
    pub frame: u64,
}

pub struct TextureCache {
    map: std::collections::HashMap<String, egui::TextureHandle>,
    failed: std::collections::HashSet<String>,
}

impl TextureCache {
    pub fn new() -> Self {
        TextureCache { map: Default::default(), failed: Default::default() }
    }

    /// Decode bytes → texture handle. Returns None if unusable.
    pub fn get(&mut self, ctx: &egui::Context, key: &str, bytes: &[u8]) -> Option<egui::TextureHandle> {
        if let Some(t) = self.map.get(key) { return Some(t.clone()); }
        if self.failed.contains(key) { return None; }
        let img = image::load_from_memory(bytes).ok()?;
        let size = [img.width() as usize, img.height() as usize];
        let rgba = img.to_rgba8();
        let color_image = egui::ColorImage::from_rgba_unmultiplied(size, rgba.as_raw());
        let texture = ctx.load_texture(format!("img-{}", key), color_image, egui::TextureOptions::LINEAR);
        self.map.insert(key.to_string(), texture.clone());
        Some(texture)
    }

    pub fn peek(&self, key: &str) -> Option<egui::TextureHandle> {
        self.map.get(key).cloned()
    }

    pub fn note_failure(&mut self, key: &str) {
        self.failed.insert(key.to_string());
    }
}

impl H3App {
    /// Current UI language, read from the persisted local state.
    pub fn lang(&self) -> crate::i18n::Lang {
        crate::i18n::Lang::from_str(&self.local.language)
    }

    pub fn set_language(&mut self, lang: crate::i18n::Lang) {
        self.local.language = lang.as_str().to_string();
        self.local.save();
    }

    pub fn new(initial_screen: Screen, server_url: String) -> Self {
        let local = crate::state::AppLocalState::load();
        H3App {
            screen: initial_screen,
            client: crate::client::ComfyClient::new(&server_url),
            server_url,
            local,
            gpus: Vec::new(),
            host: crate::gpu::HostInfo::default(),
            connect_ok: false,
            last_poll: std::time::Instant::now() - std::time::Duration::from_secs(999),
            queue: serde_json::json!({"queue_running": [], "queue_pending": []}),
            history_recent: serde_json::json!({}),
            input_images: Vec::new(),
            assets: serde_json::json!({"assets": []}),
            telemetry_rx: None,
            telemetry_tx: None,
            texture_rx: None,
            texture_tx: None,
            job_rx: None,
            job_tx: None,
            selected_input_image: None,
            last_cutout: None,
            last_cinematic: None,
            last_multiview: None,
            textures: TextureCache::new(),
            pending_inputs: std::collections::HashSet::new(),
            toast: None,
            initialized: false,
            multiview_save_as: crate::workflows::SaveAs::Merged,
            studio_mode: crate::screens::studio::StudioMode::default(),
            studio_prompt: String::new(),
            cine_prompt: String::new(),
            ui: crate::ui_state::UiState {
                new_project_aspect: "16:9".into(),
                new_project_style: "Cinematic Wuxia".into(),
                export_container: "MP4".into(),
                export_resolution: "2560×1440".into(),
                export_fps: "24".into(),
                export_select_all: true,
                export_selected: ["S01 · 001", "S01 · 002", "S01 · 003", "S01 · 004"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
                review_version: 1,
                ..Default::default()
            },
            screenshot_path: std::env::var("H3_SCREENSHOT_TO").ok(),
            screenshot_taken: false,
            frame: 0,
        }
    }

    pub fn nav_to(&mut self, screen: Screen) {
        self.screen = screen;
    }

    pub fn set_toast(&mut self, msg: impl Into<String>) {
        self.toast = Some((ToastKind::Info, msg.into(), std::time::Instant::now()));
    }

    pub fn toast_success(&mut self, msg: impl Into<String>) {
        self.toast = Some((ToastKind::Success, msg.into(), std::time::Instant::now()));
    }

    pub fn toast_warning(&mut self, msg: impl Into<String>) {
        self.toast = Some((ToastKind::Warning, msg.into(), std::time::Instant::now()));
    }

    pub fn toast_error(&mut self, msg: impl Into<String>) {
        self.toast = Some((ToastKind::Error, msg.into(), std::time::Instant::now()));
    }

    /// Lazily create the job result channel.
    fn ensure_job_channel(&mut self) -> std::sync::mpsc::Sender<JobResult> {
        if self.job_tx.is_none() {
            let (tx, rx) = std::sync::mpsc::channel::<JobResult>();
            self.job_tx = Some(tx);
            self.job_rx = Some(rx);
        }
        self.job_tx.clone().unwrap()
    }

    /// Submit a workflow graph to the server in the background.
    /// Surfaces a "Queued" toast immediately, then polls /history/{prompt_id}
    /// and surfaces a "Ready" toast once the saved output filenames appear, so
    /// the preview panel can fetch and display the real generated image.
    pub fn submit_workflow(&mut self, graph: serde_json::Value, label: &str) {
        let tx = self.ensure_job_channel();
        let client = self.client.clone();
        let label_owned = label.to_string();
        let lang = self.lang();
        std::thread::spawn(move || {
            let (ok, prompt_id) = match client.submit_prompt(graph, "h3-studio") {
                Ok(resp) => {
                    let id = resp.get("prompt_id").and_then(|v| v.as_str()).unwrap_or("?").to_string();
                    (true, Some(id))
                }
                Err(e) => {
                    let _ = tx.send(JobResult {
                        ok: false,
                        msg: format!("✗ {} — {}", label_owned, e),
                        outputs: Vec::new(),
                    });
                    return;
                }
            };
            let id_str = prompt_id.clone().unwrap_or_default();
            let _ = tx.send(JobResult {
                ok,
                msg: crate::i18n::tf(lang, "✓ Queued ({}) · {}", &[&label_owned, &id_str]),
                outputs: Vec::new(),
            });
            // Poll history until the saved output(s) appear (≤ ~3 min).
            if let Some(id) = prompt_id {
                for attempt in 0..180 {
                    let delay = if attempt == 0 { 800 } else { 1000 };
                    std::thread::sleep(std::time::Duration::from_millis(delay));
                    if let Some(outs) = client
                        .history(&id)
                        .ok()
                        .and_then(|h| extract_output_files(&h, &id))
                    {
                        if !outs.is_empty() {
                            let names: Vec<&str> = outs.iter().map(|(_, f)| f.as_str()).collect();
                            let joined = names.join(", ");
                            let _ = tx.send(JobResult {
                                ok: true,
                                msg: crate::i18n::tf(lang, "✓ Ready ({}) · {}", &[&label_owned, &joined]),
                                outputs: outs,
                            });
                            return;
                        }
                    }
                }
            }
        });
    }

    /// Lazily create the texture channel (shared by all in-flight image fetches).
    fn ensure_texture_channel(&mut self) -> std::sync::mpsc::Sender<AssetResult> {
        if self.texture_tx.is_none() {
            let (tx, rx) = std::sync::mpsc::channel::<AssetResult>();
            self.texture_tx = Some(tx);
            self.texture_rx = Some(rx);
        }
        self.texture_tx.clone().unwrap()
    }

    /// Schedule a texture download in the background.
    /// `filename` is the server-relative input-image name.
    pub fn request_image_bytes(&mut self, key: &str, filename: &str) {
        if self.pending_inputs.contains(key) || self.textures.peek(key).is_some() { return; }
        self.pending_inputs.insert(key.to_string());
        let tx = self.ensure_texture_channel();
        let client = self.client.clone();
        let key_owned = key.to_string();
        let file_owned = filename.to_string();
        std::thread::spawn(move || {
            let result = client.view_input_image(&file_owned).unwrap_or_default();
            let _ = tx.send(AssetResult { key: key_owned, bytes: result });
        });
    }

    /// Schedule a texture download for an asset's relative_path (input or output).
    pub fn request_asset_bytes(&mut self, key: &str, relative_path: &str) {
        if self.pending_inputs.contains(key) || self.textures.peek(key).is_some() { return; }
        self.pending_inputs.insert(key.to_string());
        let tx = self.ensure_texture_channel();
        let client = self.client.clone();
        let key_owned = key.to_string();
        let path_owned = relative_path.to_string();
        std::thread::spawn(move || {
            let result = client.view_asset(&path_owned).unwrap_or_default();
            let _ = tx.send(AssetResult { key: key_owned, bytes: result });
        });
    }

    /// Schedule a texture download for a generated output file.
    /// `filename` is the output-relative name (e.g. "h3_bg_00000.png").
    pub fn request_output_bytes(&mut self, key: &str, filename: &str) {
        if self.pending_inputs.contains(key) || self.textures.peek(key).is_some() { return; }
        self.pending_inputs.insert(key.to_string());
        let tx = self.ensure_texture_channel();
        let client = self.client.clone();
        let key_owned = key.to_string();
        let file_owned = filename.to_string();
        std::thread::spawn(move || {
            let result = client.view_output_image(&file_owned).unwrap_or_default();
            let _ = tx.send(AssetResult { key: key_owned, bytes: result });
        });
    }

    /// Ingest background telemetry + texture payloads.
    fn ingest_background(&mut self, ctx: &egui::Context) {
        if let Some(rx) = self.telemetry_rx.as_ref() {
            loop {
                match rx.try_recv() {
                    Ok(result) => {
                        self.connect_ok = result.ok;
                        if !result.gpus.is_empty() { self.gpus = result.gpus; }
                        self.host = result.host;
                        self.queue = result.queue;
                        self.history_recent = result.history_recent;
                        self.assets = result.assets;
                        self.input_images = result.input_images;
                        self.telemetry_tx = None;
                        self.telemetry_rx = None;
                        break;
                    }
                    Err(_) => break,
                }
            }
        }
        let mut pending_texture_load = Vec::new();
        if let Some(rx) = self.texture_rx.as_ref() {
            loop {
                match rx.try_recv() {
                    Ok(result) => pending_texture_load.push(result),
                    Err(_) => break,
                }
            }
        }
        for result in pending_texture_load {
            self.pending_inputs.remove(&result.key);
            if result.bytes.is_empty() {
                self.textures.note_failure(&result.key);
            } else {
                let _ = self.textures.get(ctx, &result.key, &result.bytes);
            }
        }
        // Job results
        let mut job_results = Vec::new();
        if let Some(rx) = self.job_rx.as_ref() {
            while let Ok(result) = rx.try_recv() {
                job_results.push(result);
            }
        }
        for result in job_results {
            if result.ok {
                self.toast_success(result.msg);
            } else {
                self.toast_error(result.msg);
            }
            for (slot, filename) in result.outputs {
                match slot {
                    OutputSlot::Cutout => self.last_cutout = Some(filename),
                    OutputSlot::Cinematic => self.last_cinematic = Some(filename),
                    OutputSlot::Multiview => self.last_multiview = Some(filename),
                    OutputSlot::None => {}
                }
            }
        }
    }

    /// Poll server + local telemetry ~every 1.5s.
    fn maybe_poll(&mut self, ctx: &egui::Context) {
        self.ingest_background(ctx);
        let every = std::time::Duration::from_millis(1500);
        if !self.initialized || self.last_poll.elapsed() >= every {
            self.initialized = true;
            self.last_poll = std::time::Instant::now();
            let (tx, rx) = std::sync::mpsc::channel::<PollResult>();
            self.telemetry_rx = Some(rx);
            self.telemetry_tx = Some(tx.clone());
            let client = self.client.clone();
            std::thread::spawn(move || {
                let gpus = crate::gpu::collect_gpus();
                let host = crate::gpu::collect_host();
                let queue = client.queue().unwrap_or_else(|_| serde_json::json!({"queue_running": [], "queue_pending": []}));
                let history = client.history_recent().unwrap_or_else(|_| serde_json::json!({}));
                let assets = client.list_assets().unwrap_or_else(|_| serde_json::json!({"assets": []}));
                let inputs = client.list_input_images().unwrap_or_else(|_| serde_json::json!({"images": []}));
                let images = inputs.get("images").and_then(|v| v.as_array())
                    .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect::<Vec<_>>())
                    .unwrap_or_default();
                let ok = client.ping();
                let _ = tx.send(PollResult { ok, gpus, host, queue, history_recent: history, assets, input_images: images });
            });
            ctx.request_repaint_after(std::time::Duration::from_millis(300));
        } else {
            // While waiting for the poll to return, keep the UI ticking.
            ctx.request_repaint_after(std::time::Duration::from_millis(150));
        }
        if self.screen == Screen::Gpu {
            ctx.request_repaint_after(std::time::Duration::from_secs(1));
        }
    }

    fn apply_theme(&mut self, ctx: &egui::Context) {
        use egui::{CornerRadius, Margin};
        let mut style = (*ctx.style()).clone();
        style.visuals.dark_mode = true;
        style.visuals.override_text_color = Some(color::text());
        style.visuals.faint_bg_color = color::bg();
        style.visuals.panel_fill = color::bg();
        style.visuals.extreme_bg_color = color::thumb();
        style.visuals.code_bg_color = color::thumb();
        style.visuals.selection.bg_fill = color::indigo_dim();
        style.visuals.selection.stroke.color = color::indigo();
        style.spacing.item_spacing = egui::vec2(10.0, 8.0);
        style.spacing.window_margin = Margin::same(10);
        style.spacing.menu_margin = Margin::same(4);
        style.spacing.button_padding = egui::vec2(12.0, 7.0);
        style.spacing.interact_size = egui::vec2(20.0, 22.0);
        style.spacing.scroll.bar_width = 8.0;
        style.visuals.widgets.inactive.bg_fill = color::card();
        style.visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, color::border());
        style.visuals.widgets.inactive.corner_radius = CornerRadius::same(8);
        style.visuals.widgets.hovered.bg_fill = color::card();
        style.visuals.widgets.hovered.fg_stroke = Stroke::new(1.2, color::indigo());
        style.visuals.widgets.hovered.corner_radius = CornerRadius::same(8);
        style.visuals.widgets.active.bg_fill = color::indigo_dim();
        style.visuals.widgets.active.fg_stroke = Stroke::new(1.6, color::indigo());
        style.visuals.widgets.active.corner_radius = CornerRadius::same(8);
        style.visuals.widgets.noninteractive.bg_fill = color::panel();
        style.visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, color::border());
        style.visuals.widgets.noninteractive.corner_radius = CornerRadius::same(8);
        ctx.set_style(style);
    }

    fn draw_sidebar(&mut self, ctx: &egui::Context) {
        let compact = self.ui.compact_mode;
        let sidebar_w = if compact { 76.0 } else { metrics::SIDEBAR };
        egui::SidePanel::left("side")
            .exact_width(sidebar_w)
            .frame(egui::Frame::NONE.fill(color::sidebar()))
            .show(ctx, |ui| {
                if compact {
                    self.draw_sidebar_compact(ui);
                } else {
                    self.draw_sidebar_expanded(ui);
                }
            });
    }

    /// Collapse button (expanded sidebar only) — a "panel-left" glyph pinned to the
    /// top-right corner of the sidebar. No text; colors follow the sidebar control
    /// pattern (muted `text3` by default, `indigo` on hover) shared with the compact
    /// expand chevron. Clicking collapses the sidebar to compact mode.
    fn draw_sidebar_collapse_button(&mut self, ui: &mut egui::Ui) -> bool {
        let lang = self.lang();
        let panel = ui.max_rect();
        let (btn, margin) = (28.0_f32, 10.0_f32);
        let rect = egui::Rect::from_min_size(
            egui::pos2(panel.right() - btn - margin, panel.top() + margin),
            egui::vec2(btn, btn),
        );
        let resp = ui.interact(rect, ui.make_persistent_id("sidebar_collapse"), egui::Sense::click());
        let col = if resp.hovered() { color::indigo() } else { color::text3() };
        let painter = ui.painter();

        // 16×16 "panel-left" glyph centered in the button: a rounded-square frame
        // with a filled vertical panel docked to the left (matches the design icon).
        let (cx, cy) = (rect.center().x, rect.center().y);
        let (s, inset, bar_w, radius) = (16.0_f32, 2.5_f32, 5.0_f32, 3.5_f32);
        let frame = egui::Rect::from_center_size(egui::pos2(cx, cy), egui::vec2(s, s));
        painter.rect_stroke(frame, radius, Stroke::new(1.5, col), StrokeKind::Middle);
        let bar = egui::Rect::from_min_max(
            egui::pos2(frame.left() + inset, frame.top() + inset),
            egui::pos2(frame.left() + inset + bar_w, frame.bottom() - inset),
        );
        painter.rect_filled(bar, 1.5, col);

        resp.on_hover_text(crate::i18n::tr(lang, "Collapse sidebar")).clicked()
    }

    fn draw_sidebar_expanded(&mut self, ui: &mut egui::Ui) {
        let lang = self.lang();
        // Collapse affordance — panel glyph pinned to the top-right corner.
        if self.draw_sidebar_collapse_button(ui) {
            self.ui.compact_mode = true;
        }
        // 18px gap between the top edge and the brand — aligns it with the
        // topbar title / action button.
        ui.add_space(18.0);
        // Brand
        ui.horizontal(|ui| {
            ui.add_space(16.0);
            ui.with_layout(egui::Layout::top_down(egui::Align::Min), |ui| {
                ui.horizontal(|ui| {
                    let (rect, _) = ui.allocate_exact_size(egui::vec2(40.0, 44.0), egui::Sense::hover());
                    ui.painter().rect_filled(rect, 10.0f32, color::panel());
                    ui.painter().rect_stroke(rect, 10.0f32, Stroke::new(1.0, color::border()), StrokeKind::Middle);
                    ui.painter().text(
                        rect.center(),
                        Align2::CENTER_CENTER,
                        "H3",
                        FontId::new(17.0, FontFamily::Monospace),
                        color::indigo(),
                    );
                    ui.add_space(9.0);
                    ui.vertical(|ui| {
                        ui.colored_label(color::text(), egui::RichText::new(crate::i18n::tr(lang, "STUDIO")).size(13.0).strong());
                        ui.colored_label(color::text2(), egui::RichText::new(crate::i18n::tr(lang, "AI filmmaking workstation")).size(9.5));
                    });
                });
            });
        });
        ui.add_space(10.0);

        // Language toggle — global Chinese/English switch
        ui.horizontal(|ui| {
            ui.add_space(16.0);
            for (lang_opt, label) in [(crate::i18n::Lang::Zh, "中文"), (crate::i18n::Lang::En, "EN")] {
                let active = lang == lang_opt;
                let (rect, resp) = ui.allocate_exact_size(egui::vec2(70.0, 26.0), egui::Sense::click());
                ui.painter().rect_filled(rect, 7.0f32, if active { color::indigo_dim() } else { Color32::TRANSPARENT });
                ui.painter().rect_stroke(rect, 7.0f32, Stroke::new(1.0, if active { color::indigo() } else { color::border() }), StrokeKind::Middle);
                ui.painter().text(
                    rect.center(),
                    Align2::CENTER_CENTER,
                    label,
                    FontId::new(10.5, FontFamily::Proportional),
                    if active { color::indigo() } else { color::text2() },
                );
                if resp.clicked() && !active {
                    self.set_language(lang_opt);
                }
                ui.add_space(4.0);
            }
        });
        ui.add_space(8.0);

        // Nav
        let active = self.screen.nav_tab();
        let nav_items = [
            (NavTab::Home, Screen::Home),
            (NavTab::Projects, Screen::Projects),
            (NavTab::Storyboard, Screen::Storyboard),
            (NavTab::Characters, Screen::Characters),
            (NavTab::Assets, Screen::Assets),
            (NavTab::Studio, Screen::Studio),
            (NavTab::Queue, Screen::Queue),
            (NavTab::Review, Screen::Review),
            (NavTab::Export, Screen::Export),
            (NavTab::Gpu, Screen::Gpu),
        ];
        let mut nav_click = None;
        for (tab, screen) in nav_items {
            let is_active = active == Some(tab);
            let height = 40.0;
            let (rect, response) = ui.allocate_exact_size(egui::vec2(196.0, height), egui::Sense::click());
            if is_active {
                ui.painter().rect_filled(rect, 8.0f32, color::indigo_selected());
            }
            let icon_color = if is_active { color::indigo() } else { color::indigo().linear_multiply(0.45) };
            icons::draw_icon(ui.painter(), tab, rect.left() + 26.0, rect.center().y, icon_color);
            let label_col = if is_active { color::indigo() } else { color::text2() };
            let tab_label = crate::i18n::tr(lang, tab.label()).to_string();
            let galley = ui.painter().layout_no_wrap(
                tab_label.clone(),
                FontId::new(13.0, FontFamily::Proportional),
                Color32::PLACEHOLDER,
            );
            ui.painter().galley(
                egui::pos2(rect.left() + 48.0, rect.center().y - galley.size().y / 2.0),
                galley,
                label_col,
            );
            let response = response.on_hover_text(tab_label);
            if response.clicked() {
                nav_click = Some(screen);
            }
            ui.add_space(6.0);
        }
        if let Some(screen) = nav_click {
            self.nav_to(screen);
        }

        // Filler
        let remaining = ui.available_height() - 64.0;
        if remaining > 0.0 {
            ui.add_space(remaining);
        }

        // Bottom pill
        let (rect, _) = ui.allocate_exact_size(egui::vec2(184.0, 46.0), egui::Sense::hover());
        ui.painter().rect_filled(rect, 8.0f32, color::panel());
        ui.painter().rect_stroke(rect, 8.0f32, Stroke::new(1.0, color::border()), StrokeKind::Middle);
        ui.painter().circle_filled(egui::pos2(rect.left() + 15.0, rect.center().y), 5.0, color::teal());
        ui.painter().text(
            egui::pos2(rect.left() + 29.0, rect.center().y),
            Align2::LEFT_CENTER,
            crate::i18n::tr(lang, "Local inference · 3× GPU"),
            FontId::new(11.0, FontFamily::Proportional),
            color::text2(),
        );
    }

    /// Compact icon rail — sidebar collapsed to 76px: brand tile (click to expand),
    /// a chevron expand button, icon-only nav, and a status dot.
    fn draw_sidebar_compact(&mut self, ui: &mut egui::Ui) {
        let lang = self.lang();

        // Brand tile — clickable to expand
        let (brect, bresp) = ui.allocate_exact_size(egui::vec2(76.0, 60.0), egui::Sense::click());
        let tile = egui::Rect::from_min_size(egui::pos2(brect.center().x - 20.0, brect.center().y - 22.0), egui::vec2(40.0, 44.0));
        ui.painter().rect_filled(tile, 10.0, color::panel());
        ui.painter().rect_stroke(tile, 10.0, Stroke::new(1.0, color::border()), StrokeKind::Middle);
        ui.painter().text(tile.center(), Align2::CENTER_CENTER, "H3", FontId::new(17.0, FontFamily::Monospace), color::indigo());

        // Expand chevron
        let (erect, eresp) = ui.allocate_exact_size(egui::vec2(76.0, 26.0), egui::Sense::click());
        let ecol = if eresp.hovered() { color::indigo() } else { color::text3() };
        ui.painter().text(erect.center(), Align2::CENTER_CENTER, "»", FontId::new(16.0, FontFamily::Proportional), ecol);

        let tip = crate::i18n::tr(lang, "Expand sidebar");
        let bresp = bresp.on_hover_text(tip);
        let eresp = eresp.on_hover_text(tip);
        if bresp.clicked() || eresp.clicked() {
            self.ui.compact_mode = false;
        }
        ui.add_space(8.0);

        // Nav — icon only
        let active = self.screen.nav_tab();
        let nav_items = [
            (NavTab::Home, Screen::Home),
            (NavTab::Projects, Screen::Projects),
            (NavTab::Storyboard, Screen::Storyboard),
            (NavTab::Characters, Screen::Characters),
            (NavTab::Assets, Screen::Assets),
            (NavTab::Studio, Screen::Studio),
            (NavTab::Queue, Screen::Queue),
            (NavTab::Review, Screen::Review),
            (NavTab::Export, Screen::Export),
            (NavTab::Gpu, Screen::Gpu),
        ];
        let mut nav_click = None;
        for (tab, screen) in nav_items {
            let is_active = active == Some(tab);
            let (rect, resp) = ui.allocate_exact_size(egui::vec2(76.0, 44.0), egui::Sense::click());
            if is_active {
                ui.painter().rect_filled(rect, 8.0, color::indigo_selected());
            }
            let icon_color = if is_active { color::indigo() } else { color::indigo().linear_multiply(0.45) };
            icons::draw_icon(ui.painter(), tab, rect.center().x, rect.center().y, icon_color);
            let resp = resp.on_hover_text(crate::i18n::tr(lang, tab.label()).to_string());
            if resp.clicked() {
                nav_click = Some(screen);
            }
            ui.add_space(4.0);
        }
        if let Some(screen) = nav_click {
            self.nav_to(screen);
        }

        // Filler
        let remaining = ui.available_height() - 56.0;
        if remaining > 0.0 {
            ui.add_space(remaining);
        }

        // Bottom status dot
        let (rect, _) = ui.allocate_exact_size(egui::vec2(76.0, 40.0), egui::Sense::hover());
        ui.painter().circle_filled(rect.center(), 5.0, color::teal());
    }

    fn draw_topbar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("topbar")
            .exact_height(metrics::TOPBAR)
            .frame(egui::Frame::NONE.fill(color::topbar()))
            .show(ctx, |ui| {
                ui.painter().rect_filled(egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(4.0, metrics::TOPBAR)), 0.0f32, color::border());
                ui.painter().rect_filled(egui::Rect::from_min_size(egui::pos2(24.0, 15.0), egui::vec2(26.0, 4.0)), 2.0f32, color::indigo());
                // 18px gap between the top edge and the title / action button.
                ui.add_space(18.0);
                ui.horizontal(|ui| {
                    ui.add_space(56.0);
                    ui.vertical(|ui| {
                        ui.colored_label(color::text(), egui::RichText::new(crate::i18n::tr(self.lang(), self.screen.title())).size(typography::H1).strong());
                        ui.colored_label(color::text2(), crate::i18n::tr(self.lang(), self.screen.subtitle()));
                    });
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        self.topbar_action(ui);
                    });
                });
            });
    }

    fn topbar_action(&mut self, ui: &mut egui::Ui) {
        use crate::i18n::tr;
        let lang = self.lang();
        match self.screen {
            Screen::Home => {
                if widgets::primary_button(ui, tr(lang, "New Project"), 150.0, 36.0) {
                    self.set_toast(tr(lang, "New project — pick a template to get started"));
                }
            }
            Screen::Storyboard => {
                if widgets::outline_button(ui, tr(lang, "⌘S · Add Shot"), 170.0, 36.0) {
                    self.set_toast(tr(lang, "Shot editor — coming soon"));
                }
            }
            Screen::Characters => {
                if widgets::outline_button(ui, tr(lang, "⌘N · New Character"), 200.0, 36.0) {
                    self.set_toast(tr(lang, "New character — coming soon"));
                }
            }
            Screen::Assets => {
                if widgets::outline_button_indigo(ui, tr(lang, "Import Assets"), 170.0, 36.0) {
                    self.set_toast(tr(lang, "Use the import zone — or press the Import button"));
                }
            }
            Screen::Queue => {
                if widgets::outline_button(ui, tr(lang, "Pause All"), 170.0, 36.0) {
                    self.set_toast(tr(lang, "Pause — coming soon"));
                }
            }
            Screen::Review => {
                if widgets::primary_button(ui, tr(lang, "Approve Shot"), 170.0, 36.0) {
                    self.toast_success(tr(lang, "Shot approved (demo)"));
                }
            }
            Screen::Export => {
                if widgets::primary_button(ui, tr(lang, "Start Export"), 170.0, 36.0) {
                    self.set_toast(tr(lang, "Export — coming soon"));
                }
            }
            Screen::Gpu => {
                if widgets::outline_button_indigo(ui, tr(lang, "Auto-refresh 1s"), 180.0, 36.0) {
                    self.set_toast(tr(lang, "Auto-refresh is on"));
                }
            }
            Screen::RemoveBg | Screen::Cinematic | Screen::MultiView | Screen::Combined => {
                if widgets::outline_button(ui, tr(lang, "⌫ Characters"), 170.0, 36.0) {
                    self.nav_to(Screen::Characters);
                }
            }
            Screen::Projects => {
                if widgets::outline_button(ui, "⌘↵", 110.0, 36.0) {
                    self.set_toast(tr(lang, "Shortcut — coming soon"));
                }
            }
            Screen::Studio => {
                let label = if self.studio_mode == crate::screens::studio::StudioMode::Fl2VA { "⌘↵ switch to Ref2VA" } else { "⌘↵ switch to FL2VA" };
                let label = tr(lang, label);
                if widgets::outline_button(ui, label, 200.0, 36.0) {
                    use crate::screens::studio::StudioMode;
                    self.studio_mode = match self.studio_mode {
                        StudioMode::Fl2VA => StudioMode::Ref2VA,
                        StudioMode::Ref2VA => StudioMode::Fl2VA,
                    };
                }
            }
        }
    }

    fn draw_statusbar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::bottom("statusbar")
            .exact_height(metrics::BOTTOMBAR)
            .frame(egui::Frame::NONE.fill(color::bg()))
            .show(ctx, |ui| {
                ui.painter().rect_filled(egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(metrics::TOPBAR * 0.0 + ui.available_width() + metrics::SIDEBAR - metrics::SIDEBAR, 1.0)), 0.0f32, color::border());
                ui.horizontal(|ui| {
                    ui.add_space(20.0);
                    let dot_col = if self.connect_ok { color::teal() } else { color::red() };
                    ui.painter().circle_filled(ui.cursor().min + egui::vec2(5.0, ui.cursor().height() / 2.0), 4.5, dot_col);
                    ui.add_space(18.0);
                    let label_col = if self.connect_ok { color::text() } else { color::red() };
                    let label = if self.connect_ok {
                        crate::i18n::tf(self.lang(), "Server · {}", &[&self.server_url])
                    } else {
                        crate::i18n::tr(self.lang(), "Server offline — start comfy-server on :8188").to_string()
                    };
                    ui.colored_label(label_col, egui::RichText::new(label).monospace().size(typography::SMALL));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.colored_label(color::text3(), egui::RichText::new(crate::i18n::tf(self.lang(), "{} GPUs detected", &[&self.gpus.len()])).size(typography::SMALL));
                    });
                });
            });
    }
}

impl eframe::App for H3App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.frame += 1;
        if self.frame <= 4 || self.frame % 60 == 0 {
            eprintln!("[h3] frame {} screen={:?}", self.frame, self.screen);
        }

        // App-level screenshot capture. Unlike eframe's auto __screenshot
        // (which reads the framebuffer *after* swap_buffers and returns blank
        // on this platform), ViewportCommand::Screenshot is serviced *before*
        // swap_buffers, so the returned ColorImage contains the painted UI.
        if let Some(path) = self.screenshot_path.clone() {
            let target_frame: u64 = std::env::var("H3_SCREENSHOT_FRAMES")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(12);
            if !self.screenshot_taken && self.frame >= target_frame {
                self.screenshot_taken = true;
                eprintln!("[h3] requesting screenshot -> {}", path);
                ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::default()));
            }
            let mut shot: Option<std::sync::Arc<egui::ColorImage>> = None;
            ctx.input(|i| {
                for ev in &i.events {
                    if let egui::Event::Screenshot { image, .. } = ev {
                        shot = Some(image.clone());
                    }
                }
            });
            if let Some(img) = shot {
                let rgba = img.as_raw();
                match image::save_buffer(
                    &path,
                    rgba,
                    img.width() as u32,
                    img.height() as u32,
                    image::ColorType::Rgba8,
                ) {
                    Ok(_) => eprintln!(
                        "[h3] screenshot saved {} ({}x{})",
                        path, img.width(), img.height()
                    ),
                    Err(e) => eprintln!("[h3] screenshot save FAILED: {}", e),
                }
                std::process::exit(0);
            }
            // Keep ticking so we reach the target frame and receive the reply.
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
        }

        self.apply_theme(ctx);
        self.maybe_poll(ctx);

        if let Some((_, _, t)) = &self.toast {
            if t.elapsed() > std::time::Duration::from_secs(4) {
                self.toast = None;
            }
        }

        self.draw_sidebar(ctx);
        self.draw_topbar(ctx);
        self.draw_statusbar(ctx);

        egui::CentralPanel::default()
            .frame(egui::Frame::NONE.fill(color::bg()))
            .show(ctx, |ui| {
                crate::screens::draw(self, ui);
            });

        // Toast banner — categorized by kind: a colored category pill (accent
        // fill + white bold label, mirroring the design system status chips)
        // followed by the message, with a dimmed border of the same accent.
        // Colors follow the design system state palette.
        if let Some((kind, ref msg, _)) = self.toast {
            let lang = self.lang();
            let accent = kind.accent();
            let title = crate::i18n::tr(lang, kind.title_key());
            let border = accent.linear_multiply(0.5);
            egui::Window::new("__toast__")
                .fixed_pos(egui::pos2(ctx.screen_rect().center().x - 230.0, ctx.screen_rect().bottom() - 86.0))
                .fixed_size(egui::vec2(460.0, 52.0))
                .collapsible(false)
                .title_bar(false)
                .frame(egui::Frame::NONE
                    .fill(color::card())
                    .stroke(Stroke::new(1.0, border))
                    .corner_radius(egui::CornerRadius::same(10)))
                .show(ctx, |ui| {
                    ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                        // Drive spacing from explicit add_space values (no auto item_spacing)
                        // so the pill/dot/message offsets stay exactly as specified.
                        ui.spacing_mut().item_spacing.x = 0.0;
                        ui.add_space(16.0);
                        // Colored category pill — matches design system status chips
                        // (accent fill, fully rounded, white bold label).
                        egui::Frame::NONE
                            // .fill(accent)
                            // .corner_radius(egui::CornerRadius::same(10))
                            // .inner_margin(egui::Margin::symmetric(10, 3))
                            .show(ui, |ui| {
                                ui.colored_label(accent,
                                    egui::RichText::new(title).size(typography::CAPTION).strong());
                            });
                        // ui.add_space(8.0);
                        ui.colored_label(color::text3(), egui::RichText::new("·").size(typography::CAPTION));
                        // ui.add_space(8.0);
                        ui.colored_label(color::text(), egui::RichText::new(msg).size(typography::CAPTION));
                        ui.add_space(16.0);
                    });
                });
        }
    }
}

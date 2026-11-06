//! Windows-native inference backend.
//!
//! On Windows the sd.cpp FFI/CLI backends are often unavailable or awkward to
//! build, and the one-shot `PythonBackend` scripts (which live under
//! `py/flash_attn_v100/comfy_fallback`) do not ship in this repository.
//!
//! This backend delegates every supported task to a real ComfyUI Python server
//! running in the `M:\Git-ComfyUI` venv. It can either connect to an already
//! running server or auto-start one with the venv interpreter, keeping models
//! loaded across requests (unlike the one-shot fallback scripts).
//!
//! It is intended to be the **highest-priority** backend on Windows: when the
//! `COMFY_BACKEND` is unset (or set to `windows`), the server selects this
//! backend instead of the local/CLI backends.

use crate::backend::InferenceBackend;
use crate::error::{InferenceError, InferenceResult};
use crate::image::{SdImage, SdVideo};
use crate::params::{
    ContextIrParams, Gaussian3DOutput, Gaussian3DParams, H3Context, H3Params, ImageGenParams,
    Music3Output, Music3Params, UpscaleParams, VideoGenParams,
};
use crate::python::{resolve_python, venv_python_path};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::Mutex;
use std::time::Duration;

/// Default ComfyUI server address.
pub const DEFAULT_COMFY_URL: &str = "http://127.0.0.1:8188";

/// Default Windows ComfyUI installation root.
pub const DEFAULT_COMFY_ROOT: &str = r"M:\Git-ComfyUI";

#[derive(Debug, Clone)]
pub struct WindowsBackendConfig {
    /// ComfyUI server base URL (e.g. http://127.0.0.1:8188).
    pub server_url: String,
    /// Auto-start the ComfyUI server if it is not already running.
    pub auto_start: bool,
    /// Path to the ComfyUI installation root (contains `main.py`).
    pub comfy_root: Option<String>,
    /// Explicit Python interpreter (None = auto-detect the venv).
    pub python_path: Option<String>,
    /// Request timeout in seconds.
    pub timeout_sec: u64,
    /// Polling interval for /history in milliseconds.
    pub poll_interval_ms: u64,
}

impl Default for WindowsBackendConfig {
    fn default() -> Self {
        Self {
            server_url: DEFAULT_COMFY_URL.to_string(),
            auto_start: true,
            comfy_root: Some(DEFAULT_COMFY_ROOT.to_string()),
            python_path: None,
            timeout_sec: 600,
            poll_interval_ms: 500,
        }
    }
}

impl WindowsBackendConfig {
    pub fn new(server_url: impl Into<String>) -> Self {
        Self {
            server_url: server_url.into(),
            ..Default::default()
        }
    }

    pub fn with_auto_start(mut self, auto: bool) -> Self {
        self.auto_start = auto;
        self
    }

    pub fn with_comfy_root(mut self, root: impl Into<String>) -> Self {
        self.comfy_root = Some(root.into());
        self
    }

    pub fn with_python_path(mut self, path: impl Into<String>) -> Self {
        self.python_path = Some(path.into());
        self
    }
}

pub struct WindowsBackend {
    config: WindowsBackendConfig,
    client: reqwest::blocking::Client,
    /// Child process we started (if any), killed on drop.
    child: Mutex<Option<Child>>,
    start_lock: Mutex<()>,
}

impl std::fmt::Debug for WindowsBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WindowsBackend")
            .field("config", &self.config)
            .field("started", &self.child.lock().map(|c| c.is_some()).unwrap_or(false))
            .finish()
    }
}

impl Drop for WindowsBackend {
    fn drop(&mut self) {
        if let Ok(mut guard) = self.child.lock() {
            if let Some(child) = guard.take() {
                let _ = kill_child(child);
            }
        }
    }
}

impl WindowsBackend {
    pub fn new(config: WindowsBackendConfig) -> Self {
        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(config.timeout_sec))
            .connect_timeout(Duration::from_secs(10))
            .build()
            .expect("Failed to create HTTP client for WindowsBackend");
        Self {
            config,
            client,
            child: Mutex::new(None),
            start_lock: Mutex::new(()),
        }
    }

    pub fn config(&self) -> &WindowsBackendConfig {
        &self.config
    }

    fn url(&self, path: &str) -> String {
        format!("{}{}", self.config.server_url.trim_end_matches('/'), path)
    }

    // ---------- server lifecycle ----------

    /// Whether the ComfyUI server responds to a health probe.
    fn is_server_up(&self) -> bool {
        self.client
            .get(self.url("/"))
            .send()
            .map(|r| r.status().is_success())
            .unwrap_or(false)
    }

    /// Resolve the Python interpreter to use for launching ComfyUI.
    fn resolve_python(&self) -> String {
        if let Some(ref py) = self.config.python_path {
            if !py.is_empty() {
                return py.clone();
            }
        }
        // Prefer the ComfyUI venv next to the configured root.
        if let Some(ref root) = self.config.comfy_root {
            let py = venv_python_path(&PathBuf::from(root).join("venv"));
            if py.exists() {
                return py.to_string_lossy().to_string();
            }
        }
        resolve_python(None, None)
    }

    /// Start the ComfyUI server if it is not already running. Returns once the
    /// server responds to health checks (or after the timeout).
    pub fn ensure_server(&self) -> InferenceResult<()> {
        if self.is_server_up() {
            return Ok(());
        }
        if !self.config.auto_start {
            return Err(InferenceError::BackendNotAvailable(format!(
                "ComfyUI server at {} is not running and auto_start is disabled",
                self.config.server_url
            )));
        }

        let _guard = self.start_lock.lock().unwrap();
        // Re-check after acquiring the lock (another thread may have started it).
        if self.is_server_up() {
            return Ok(());
        }

        let root = self.config.comfy_root.as_deref().unwrap_or(DEFAULT_COMFY_ROOT);
        let root_path = PathBuf::from(root);
        let main_py = root_path.join("main.py");
        if !main_py.exists() {
            return Err(InferenceError::BackendNotAvailable(format!(
                "ComfyUI main.py not found at {}; cannot auto-start",
                main_py.display()
            )));
        }

        let python = self.resolve_python();
        tracing::info!("WindowsBackend: starting ComfyUI server (python={python}, root={root})");

        // Parse host:port from server_url.
        let (host, port) = parse_host_port(&self.config.server_url);

        // Point ComfyUI's output/temp dirs at subdirectories of the
        // comfyui-rust workspace so writes succeed under the sandbox (the
        // ComfyUI checkout itself may be read-only).
        // Note: ComfyUI appends a "temp" subfolder to --temp-directory, so we
        // pass the workspace root (→ <workspace>/temp) while --output-directory
        // is used as-is (→ <workspace>/output).
        let output_dir = std::env::current_dir()
            .map(|d| d.join("output"))
            .unwrap_or_else(|_| PathBuf::from("output"));
        let temp_base = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let _ = std::fs::create_dir_all(&output_dir);
        let _ = std::fs::create_dir_all(temp_base.join("temp"));

        let mut cmd = Command::new(&python);
        cmd.arg(&main_py)
            .arg("--listen")
            .arg(&host)
            .arg("--port")
            .arg(port.to_string())
            .arg("--output-directory")
            .arg(&output_dir)
            .arg("--temp-directory")
            .arg(&temp_base)
            .current_dir(&root_path)
            .stdout(Stdio::null())
            .stderr(Stdio::null());

        let child = cmd.spawn().map_err(|e| {
            InferenceError::BackendNotAvailable(format!(
                "Failed to start ComfyUI server with '{python}': {e}"
            ))
        })?;

        let pid = child.id();
        {
            let mut guard = self.child.lock().unwrap();
            *guard = Some(child);
        }
        tracing::info!("WindowsBackend: ComfyUI server started (pid={pid})");

        // Wait for the server to become ready.
        let deadline = std::time::Instant::now() + Duration::from_secs(180);
        while std::time::Instant::now() < deadline {
            if self.is_server_up() {
                tracing::info!("WindowsBackend: ComfyUI server is ready");
                return Ok(());
            }
            // If the child died, bail out early.
            if let Ok(mut guard) = self.child.lock() {
                if let Some(child) = guard.as_mut() {
                    if let Ok(Some(_status)) = child.try_wait() {
                        return Err(InferenceError::BackendNotAvailable(
                            "ComfyUI server process exited during startup".to_string(),
                        ));
                    }
                }
            }
            std::thread::sleep(Duration::from_millis(self.config.poll_interval_ms));
        }
        Err(InferenceError::BackendNotAvailable(format!(
            "ComfyUI server did not become ready within 180s at {}",
            self.config.server_url
        )))
    }

    // ---------- workflow helpers ----------

    /// Build a ComfyUI workflow JSON for text-to-image (or image-to-image when
    /// an init image is supplied).
    fn build_image_workflow(&self, params: &ImageGenParams) -> InferenceResult<Value> {
        self.ensure_server()?;
        let loader = self.probe_model_loader(params)?;

        // Qwen Image models use a non-standard conditioning pipeline
        // (QwenImage21Cache + TextEncodeQwenImage21 + layered latents).
        let is_qwen = match &loader {
            ModelLoader::Checkpoint(n) => n.to_lowercase().contains("qwen"),
            ModelLoader::Components { unet, .. } => unet.to_lowercase().contains("qwen"),
        };
        if is_qwen {
            return self.build_qwen21_workflow(params, &loader);
        }

        let sampler_name = comfy_sampler_name(params.sample_params.sample_method);
        let scheduler = comfy_scheduler_name(params.sample_params.scheduler);
        let steps = params.sample_params.sample_steps.max(1);
        let cfg = params.sample_params.guidance.txt_cfg;
        let seed = params.seed;

        let mut wf = serde_json::Map::new();
        let (model_out, clip_out, vae_out): (Value, Value, Value);

        match loader {
            ModelLoader::Checkpoint(ref name) => {
                wf.insert(
                    "4".into(),
                    json!({
                        "class_type": "CheckpointLoaderSimple",
                        "inputs": { "ckpt_name": name }
                    }),
                );
                model_out = json!(["4", 0]);
                clip_out = json!(["4", 1]);
                vae_out = json!(["4", 2]);
            }
            ModelLoader::Components { unet, clip, vae } => {
                wf.insert(
                    "41".into(),
                    json!({
                        "class_type": "UNETLoader",
                        "inputs": { "unet_name": unet, "weight_dtype": "default" }
                    }),
                );
                wf.insert(
                    "42".into(),
                    json!({
                        "class_type": "CLIPLoader",
                        "inputs": { "clip_name": clip, "type": clip_arch_type(&unet) }
                    }),
                );
                wf.insert(
                    "43".into(),
                    json!({
                        "class_type": "VAELoader",
                        "inputs": { "vae_name": vae }
                    }),
                );
                model_out = json!(["41", 0]);
                clip_out = json!(["42", 0]);
                vae_out = json!(["43", 0]);
            }
        }

        // 6: positive prompt
        wf.insert(
            "6".into(),
            json!({
                "class_type": "CLIPTextEncode",
                "inputs": { "text": params.prompt, "clip": clip_out }
            }),
        );
        // 7: negative prompt
        wf.insert(
            "7".into(),
            json!({
                "class_type": "CLIPTextEncode",
                "inputs": { "text": params.negative_prompt, "clip": clip_out }
            }),
        );

        // Latent source: EmptyLatentImage (t2i) or VAEEncode (i2i).
        let denoise;
        if let Some(ref init) = params.init_image {
            let uploaded = self.upload_image(init)?;
            wf.insert(
                "10".into(),
                json!({
                    "class_type": "LoadImage",
                    "inputs": { "image": uploaded }
                }),
            );
            wf.insert(
                "5".into(),
                json!({
                    "class_type": "VAEEncode",
                    "inputs": { "pixels": ["10", 0], "vae": vae_out }
                }),
            );
            denoise = params.strength.clamp(0.0, 1.0);
        } else {
            wf.insert(
                "5".into(),
                json!({
                    "class_type": "EmptyLatentImage",
                    "inputs": {
                        "width": params.width.max(64),
                        "height": params.height.max(64),
                        "batch_size": params.batch_count.max(1)
                    }
                }),
            );
            denoise = 1.0;
        }

        // 3: KSampler
        wf.insert(
            "3".into(),
            json!({
                "class_type": "KSampler",
                "inputs": {
                    "seed": seed,
                    "steps": steps,
                    "cfg": cfg,
                    "sampler_name": sampler_name,
                    "scheduler": scheduler,
                    "denoise": denoise,
                    "model": model_out,
                    "positive": ["6", 0],
                    "negative": ["7", 0],
                    "latent_image": ["5", 0]
                }
            }),
        );
        // 8: VAEDecode
        wf.insert(
            "8".into(),
            json!({
                "class_type": "VAEDecode",
                "inputs": { "samples": ["3", 0], "vae": vae_out }
            }),
        );
        // 9: SaveImage
        wf.insert(
            "9".into(),
            json!({
                "class_type": "SaveImage",
                "inputs": { "images": ["8", 0], "filename_prefix": "comfyui-rust-win" }
            }),
        );

        Ok(Value::Object(wf))
    }

    /// Build a Qwen Image 2.1 workflow (uses QwenImage21Cache +
    /// TextEncodeQwenImage21 + EmptyQwenImageLayeredLatentImage).
    fn build_qwen21_workflow(&self, params: &ImageGenParams, loader: &ModelLoader) -> InferenceResult<Value> {
        let (unet, clip, vae) = match loader {
            ModelLoader::Components { unet, clip, vae } => (unet.clone(), clip.clone(), vae.clone()),
            ModelLoader::Checkpoint(_) => {
                return Err(InferenceError::UnsupportedOperation(
                    "Qwen checkpoint loading not supported; use component models".to_string(),
                ));
            }
        };
        let sampler_name = comfy_sampler_name(params.sample_params.sample_method);
        let scheduler = comfy_scheduler_name(params.sample_params.scheduler);
        let steps = params.sample_params.sample_steps.max(1);
        let cfg = params.sample_params.guidance.txt_cfg;
        let seed = params.seed;

        let mut wf = serde_json::Map::new();
        wf.insert(
            "41".into(),
            json!({
                "class_type": "UNETLoader",
                "inputs": { "unet_name": unet, "weight_dtype": "default" }
            }),
        );
        wf.insert(
            "42".into(),
            json!({
                "class_type": "CLIPLoader",
                "inputs": { "clip_name": clip, "type": "qwen_image" }
            }),
        );
        wf.insert(
            "43".into(),
            json!({
                "class_type": "VAELoader",
                "inputs": { "vae_name": vae }
            }),
        );
        // QwenImage21Cache wraps the model to enable KV cache.
        wf.insert(
            "44".into(),
            json!({
                "class_type": "QwenImage21Cache",
                "inputs": { "model": ["41", 0], "device": "auto", "dtype": "default" }
            }),
        );
        // TextEncodeQwenImage21 returns (positive, negative, latent) — the
        // latent at output index 2 is the correctly-shaped 4D latent.
        wf.insert(
            "6".into(),
            json!({
                "class_type": "TextEncodeQwenImage21",
                "inputs": {
                    "clip": ["42", 0],
                    "prompt": params.prompt,
                    "negative_prompt": params.negative_prompt,
                    "resolution": params.width.max(params.height).max(512)
                }
            }),
        );
        // KSampler uses the cached model and the latent from TextEncodeQwenImage21.
        wf.insert(
            "3".into(),
            json!({
                "class_type": "KSampler",
                "inputs": {
                    "seed": seed,
                    "steps": steps,
                    "cfg": cfg,
                    "sampler_name": sampler_name,
                    "scheduler": scheduler,
                    "denoise": 1.0,
                    "model": ["44", 0],
                    "positive": ["6", 0],
                    "negative": ["6", 1],
                    "latent_image": ["6", 2]
                }
            }),
        );
        wf.insert(
            "8".into(),
            json!({
                "class_type": "VAEDecode",
                "inputs": { "samples": ["3", 0], "vae": ["43", 0] }
            }),
        );
        wf.insert(
            "9".into(),
            json!({
                "class_type": "SaveImage",
                "inputs": { "images": ["8", 0], "filename_prefix": "comfyui-rust-win" }
            }),
        );

        Ok(Value::Object(wf))
    }

    /// Determine whether the requested model is a full checkpoint (loaded via
    /// `CheckpointLoaderSimple`) or a set of component files (UNET + CLIP +
    /// VAE). The decision is made by querying ComfyUI's `/object_info` for the
    /// available model filenames.
    fn probe_model_loader(&self, params: &ImageGenParams) -> InferenceResult<ModelLoader> {
        let primary = params
            .model_config
            .model_path
            .clone()
            .or_else(|| params.model_config.diffusion_model_path.clone())
            .ok_or_else(|| {
                InferenceError::InvalidParameter("No model path for Windows backend".to_string())
            })?;
        let primary_name = file_name(&primary);

        // Check if it's a full checkpoint.
        let checkpoints = self.fetch_model_list("CheckpointLoaderSimple", "ckpt_name")?;
        if checkpoints.iter().any(|c| c == &primary_name) {
            return Ok(ModelLoader::Checkpoint(primary_name));
        }

        // Otherwise treat it as a component model: UNET + CLIP + VAE.
        let unets = self.fetch_model_list("UNETLoader", "unet_name")?;
        let clips = self.fetch_model_list("CLIPLoader", "clip_name")?;
        let vaes = self.fetch_model_list("VAELoader", "vae_name")?;

        let unet = pick_match(&unets, &primary_name)
            .ok_or_else(|| {
                InferenceError::BackendNotAvailable(format!(
                    "UNET model '{primary_name}' not found in ComfyUI's diffusion_models. Available: {}",
                    unets.join(", ")
                ))
            })?
            .clone();

        let clip_name = params
            .model_config
            .text_encoder_path
            .as_deref()
            .or(params.model_config.llm_path.as_deref())
            .map(file_name)
            .unwrap_or_default();
        let clip = pick_match(&clips, &clip_name)
            .or_else(|| clips.first())
            .cloned()
            .ok_or_else(|| {
                InferenceError::BackendNotAvailable(
                    "No CLIP model found in ComfyUI".to_string(),
                )
            })?;

        let vae_name = params
            .model_config
            .vae_path
            .as_deref()
            .map(file_name)
            .unwrap_or_default();
        let vae = pick_match(&vaes, &vae_name)
            .or_else(|| vaes.first())
            .cloned()
            .ok_or_else(|| {
                InferenceError::BackendNotAvailable(
                    "No VAE model found in ComfyUI".to_string(),
                )
            })?;

        Ok(ModelLoader::Components { unet, clip, vae })
    }

    /// Fetch the list of accepted filenames for a given ComfyUI loader node.
    fn fetch_model_list(&self, class_type: &str, input_name: &str) -> InferenceResult<Vec<String>> {
        let url = self.url(&format!("/object_info/{}", class_type));
        let resp = self
            .client
            .get(&url)
            .send()
            .map_err(|e| InferenceError::NetworkError(e.to_string()))?;
        if !resp.status().is_success() {
            return Ok(Vec::new());
        }
        let v: Value = resp
            .json()
            .map_err(|e| InferenceError::GenerationFailed(format!("object_info bad JSON: {e}")))?;
        let list = v
            .get(class_type)
            .and_then(|x| x.get("input"))
            .and_then(|x| x.get("required"))
            .and_then(|x| x.get(input_name))
            .and_then(|x| x.get(0))
            .and_then(|x| x.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|x| x.as_str().map(|s| s.to_string()))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        Ok(list)
    }

    /// Upload an image to ComfyUI's input folder via POST /upload/image.
    fn upload_image(&self, image: &SdImage) -> InferenceResult<String> {
        let png = image.to_png_bytes()?;
        let filename = format!("comfyui-rust-win-init-{}.png", std::process::id());
        let form = reqwest::blocking::multipart::Form::new()
            .text("overwrite", "true")
            .part(
                "image",
                reqwest::blocking::multipart::Part::bytes(png)
                    .file_name(filename.clone())
                    .mime_str("image/png")
                    .map_err(|e| InferenceError::ImageEncodeError(e.to_string()))?,
            );
        let resp = self
            .client
            .post(self.url("/upload/image"))
            .multipart(form)
            .send()
            .map_err(|e| InferenceError::NetworkError(e.to_string()))?;
        if !resp.status().is_success() {
            return Err(InferenceError::GenerationFailed(format!(
                "upload/image failed: {}",
                resp.status()
            )));
        }
        let v: Value = resp
            .json()
            .map_err(|e| InferenceError::GenerationFailed(format!("upload/image bad JSON: {e}")))?;
        let name = v
            .get("name")
            .and_then(|x| x.as_str())
            .unwrap_or(&filename)
            .to_string();
        Ok(name)
    }

    // ---------- prompt submission ----------

    /// Submit a workflow and wait for it to finish, returning the history entry.
    fn run_workflow(&self, workflow: &Value) -> InferenceResult<Value> {
        self.ensure_server()?;

        let client_id = format!("comfyui-rust-win-{}", std::process::id());
        let body = json!({ "prompt": workflow, "client_id": client_id });

        let resp = self
            .client
            .post(self.url("/prompt"))
            .json(&body)
            .send()
            .map_err(|e| InferenceError::NetworkError(e.to_string()))?;
        if !resp.status().is_success() {
            let status = resp.status();
            let txt = resp.text().unwrap_or_default();
            return Err(InferenceError::GenerationFailed(format!(
                "/prompt rejected ({status}): {txt}"
            )));
        }
        let v: Value = resp
            .json()
            .map_err(|e| InferenceError::GenerationFailed(format!("/prompt bad JSON: {e}")))?;
        let prompt_id = v
            .get("prompt_id")
            .and_then(|x| x.as_str())
            .ok_or_else(|| InferenceError::GenerationFailed("/prompt returned no prompt_id".to_string()))?
            .to_string();

        // Poll /history until the prompt appears with outputs or an error.
        let deadline = std::time::Instant::now() + Duration::from_secs(self.config.timeout_sec);
        loop {
            let hist = self
                .client
                .get(self.url(&format!("/history/{}", prompt_id)))
                .send()
                .map_err(|e| InferenceError::NetworkError(e.to_string()))?;
            if hist.status().is_success() {
                let hv: Value = hist
                    .json()
                    .map_err(|e| InferenceError::GenerationFailed(format!("/history bad JSON: {e}")))?;
                if let Some(entry) = hv.get(&prompt_id) {
                    if let Some(err) = entry.get("status").and_then(|s| s.get("status_str")) {
                        if err.as_str() == Some("error") {
                            let msg = entry
                                .get("status")
                                .and_then(|s| s.get("messages"))
                                .and_then(|m| m.as_array())
                                .and_then(|a| {
                                    a.iter().find_map(|m| {
                                        m.get(1).and_then(|x| {
                                            x.get("exception_message")
                                                .or_else(|| x.get("exception_type"))
                                                .and_then(|v| v.as_str())
                                        })
                                    })
                                })
                                .unwrap_or("ComfyUI execution error")
                                .to_string();
                            return Err(InferenceError::GenerationFailed(msg));
                        }
                    }
                    if entry.get("outputs").is_some() {
                        return Ok(entry.clone());
                    }
                }
            }
            if std::time::Instant::now() >= deadline {
                return Err(InferenceError::GenerationFailed(format!(
                    "ComfyUI prompt {prompt_id} timed out after {}s",
                    self.config.timeout_sec
                )));
            }
            std::thread::sleep(Duration::from_millis(self.config.poll_interval_ms));
        }
    }

    /// Download a saved image from ComfyUI's /view endpoint.
    fn download_image(&self, filename: &str, subfolder: &str) -> InferenceResult<SdImage> {
        let url = self.url(&format!(
            "/view?filename={}&subfolder={}&type=output",
            filename, subfolder
        ));
        let bytes = self
            .client
            .get(&url)
            .send()
            .map_err(|e| InferenceError::NetworkError(e.to_string()))?
            .bytes()
            .map_err(|e| InferenceError::NetworkError(e.to_string()))?;
        SdImage::from_png_bytes(&bytes).map_err(InferenceError::from)
    }

    /// Extract image outputs from a history entry and download each one.
    fn collect_images(&self, entry: &Value) -> InferenceResult<Vec<SdImage>> {
        let outputs = entry
            .get("outputs")
            .ok_or_else(|| InferenceError::GenerationFailed("history entry has no outputs".to_string()))?;
        let mut images = Vec::new();
        for (_node_id, out) in outputs.as_object().unwrap().iter() {
            if let Some(arr) = out.get("images").and_then(|x| x.as_array()) {
                for img in arr {
                    let filename = img.get("filename").and_then(|x| x.as_str()).unwrap_or("");
                    let subfolder = img.get("subfolder").and_then(|x| x.as_str()).unwrap_or("");
                    images.push(self.download_image(filename, subfolder)?);
                }
            }
        }
        if images.is_empty() {
            return Err(InferenceError::GenerationFailed(
                "ComfyUI produced no image outputs".to_string(),
            ));
        }
        Ok(images)
    }
}

impl InferenceBackend for WindowsBackend {
    fn supports_image_generation(&self) -> bool {
        true
    }

    fn supports_video_generation(&self) -> bool {
        // ComfyUI supports video but the workflow translation is not wired yet.
        false
    }

    fn generate_image(&self, params: ImageGenParams) -> InferenceResult<Vec<SdImage>> {
        let workflow = self.build_image_workflow(&params)?;
        let entry = self.run_workflow(&workflow)?;
        self.collect_images(&entry)
    }

    fn generate_video(&self, _params: VideoGenParams) -> InferenceResult<SdVideo> {
        Err(InferenceError::UnsupportedOperation(
            "WindowsBackend video generation is not implemented yet".to_string(),
        ))
    }

    fn upscale(&self, _image: SdImage, _params: UpscaleParams) -> InferenceResult<SdImage> {
        Err(InferenceError::UnsupportedOperation(
            "WindowsBackend upscale is not implemented yet".to_string(),
        ))
    }

    fn generate_3d_gaussian(&self, _params: Gaussian3DParams) -> InferenceResult<Gaussian3DOutput> {
        Err(InferenceError::UnsupportedOperation(
            "WindowsBackend 3D generation is not implemented yet".to_string(),
        ))
    }

    fn generate_av(&self, _params: H3Params) -> InferenceResult<SdVideo> {
        Err(InferenceError::UnsupportedOperation(
            "WindowsBackend AV generation is not implemented yet".to_string(),
        ))
    }

    fn context_ir(&self, _params: ContextIrParams) -> InferenceResult<H3Context> {
        Err(InferenceError::UnsupportedOperation(
            "WindowsBackend Context-IR is not implemented yet".to_string(),
        ))
    }

    fn generate_music3(&self, _params: Music3Params) -> InferenceResult<Music3Output> {
        Err(InferenceError::UnsupportedOperation(
            "WindowsBackend Music3 generation is not implemented yet".to_string(),
        ))
    }
}

// ---------- helpers ----------

/// How the model is loaded inside ComfyUI.
#[derive(Debug, Clone)]
enum ModelLoader {
    /// A full checkpoint file (model + clip + vae bundled).
    Checkpoint(String),
    /// Separate UNET / CLIP / VAE component files.
    Components { unet: String, clip: String, vae: String },
}

/// Extract the file name from a path string (file or dir).
fn file_name(path: &str) -> String {
    Path::new(path)
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| path.to_string())
}

/// Find the first list entry whose file name matches `target`, or the first
/// entry that contains `target` as a substring.
fn pick_match<'a>(list: &'a [String], target: &str) -> Option<&'a String> {
    if target.is_empty() {
        return None;
    }
    list.iter().find(|x| x == &target).or_else(|| {
        list.iter().find(|x| x.contains(target))
    })
}

/// Pick the CLIPLoader `type` based on the UNET filename.
fn clip_arch_type(unet_name: &str) -> &'static str {
    let n = unet_name.to_lowercase();
    if n.contains("qwen") {
        "qwen_image"
    } else if n.contains("flux") {
        "flux"
    } else if n.contains("sd3") {
        "sd3"
    } else if n.contains("mochi") {
        "mochi"
    } else if n.contains("wan") {
        "wan"
    } else if n.contains("hunyuan") {
        "hunyuan_image"
    } else if n.contains("pixart") {
        "pixart"
    } else if n.contains("ltxv") {
        "ltxv"
    } else if n.contains("cosmos") {
        "cosmos"
    } else {
        "stable_diffusion"
    }
}

/// Map our SampleMethod to ComfyUI's sampler naming convention.
fn comfy_sampler_name(method: crate::types::SampleMethod) -> String {
    let name = method.to_string();
    // ComfyUI uses "euler_ancestral" where we use "euler_a".
    if name == "euler_a" {
        "euler_ancestral".to_string()
    } else {
        name
    }
}

/// Map our Scheduler enum to ComfyUI scheduler names.
fn comfy_scheduler_name(scheduler: crate::types::Scheduler) -> &'static str {
    use crate::types::Scheduler::*;
    match scheduler {
        Discrete => "normal",
        Karras => "karras",
        Exponential => "exponential",
        SgmUniform => "sgm_uniform",
        Simple => "simple",
        // ComfyUI has no exact equivalents for the rest; fall back to normal.
        Ays | Gits | Smoothstep | KlOptimal | Lcm | BongTangent | Ltx2 => "normal",
    }
}

/// Parse host and port from a ComfyUI server URL.
fn parse_host_port(url: &str) -> (String, u16) {
    let trimmed = url.trim().trim_end_matches('/');
    let without_scheme = trimmed
        .strip_prefix("http://")
        .or_else(|| trimmed.strip_prefix("https://"))
        .unwrap_or(trimmed);
    let (host_port, _path) = without_scheme.split_once('/').unwrap_or((without_scheme, ""));
    match host_port.rsplit_once(':') {
        Some((host, port)) => match port.parse::<u16>() {
            Ok(p) => (host.to_string(), p),
            Err(_) => (host_port.to_string(), 8188),
        },
        None => (host_port.to_string(), 8188),
    }
}

/// Kill a child process portably (Windows uses taskkill, Unix uses kill).
fn kill_child(mut child: Child) -> std::io::Result<()> {
    let pid = child.id();
    #[cfg(windows)]
    {
        let _ = Command::new("taskkill")
            .args(["/F", "/T", "/PID", &pid.to_string()])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    }
    #[cfg(not(windows))]
    {
        let _ = Command::new("kill")
            .args(["-TERM", &pid.to_string()])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    }
    let _ = child.wait();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_host_port() {
        assert_eq!(parse_host_port("http://127.0.0.1:8188"), ("127.0.0.1".to_string(), 8188));
        assert_eq!(parse_host_port("http://127.0.0.1:8188/"), ("127.0.0.1".to_string(), 8188));
        assert_eq!(parse_host_port("http://localhost:9000/path"), ("localhost".to_string(), 9000));
    }

    #[test]
    fn scheduler_mapping() {
        use crate::types::Scheduler;
        assert_eq!(comfy_scheduler_name(Scheduler::Discrete), "normal");
        assert_eq!(comfy_scheduler_name(Scheduler::Karras), "karras");
    }

    /// End-to-end smoke test against a live ComfyUI server.
    /// Run with: cargo test -p comfy-inference windows_backend::tests::e2e_generate_image -- --ignored --nocapture
    #[test]
    #[ignore = "requires a running ComfyUI server on 127.0.0.1:8188"]
    fn e2e_generate_image() {
        let backend = WindowsBackend::new(WindowsBackendConfig::default());
        let mut params = crate::params::ImageGenParams::new("a cute orange cat sitting on a windowsill, soft lighting")
            .with_dimensions(512, 512)
            .with_sample_steps(4)
            .with_seed(42);
        params.model_config = crate::params::ModelConfig {
            diffusion_model_path: Some("qwen-image-2.1-UC-int8_convrot.safetensors".to_string()),
            text_encoder_path: Some("qwen_3_8b_fp8mixed.safetensors".to_string()),
            vae_path: Some("qwen_image_2.1_vae.safetensors".to_string()),
            ..Default::default()
        };
        let images = backend.generate_image(params).expect("generation failed");
        assert!(!images.is_empty(), "expected at least one image");
        for (i, img) in images.iter().enumerate() {
            println!("image {i}: {}x{}", img.width, img.height);
        }
    }
}

//! Python HF-model fallback backend.
//!
//! Many new models ship only in HuggingFace format (diffusers pipeline dirs
//! for diffusion, HF CausalLM dirs for LLMs); the C++ engines (sd-cli /
//! llama-cli) usually lag behind in supporting them. This backend shells out
//! to one-shot Python scripts (transformers / diffusers) living under
//! `py/flash_attn_v100/comfy_fallback/`, trading raw performance for fast
//! model coverage.
//!
//! [`FallbackBackend`] wraps a primary CLI/FFI backend and routes HF-format
//! models (or failed primary attempts) to Python automatically.

use crate::backend::InferenceBackend;
use crate::error::{InferenceError, InferenceResult};
use crate::image::{SdImage, SdVideo};
use crate::params::{
    Gaussian3DOutput, Gaussian3DParams, ImageGenParams, UpscaleParams, VideoGenParams,
};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;

/// Name of the fallback package directory inside py/flash_attn_v100.
pub const FALLBACK_PKG_DIR: &str = "py/flash_attn_v100/comfy_fallback";

#[derive(Debug, Clone)]
pub struct PythonInferConfig {
    /// Explicit interpreter path (None = auto-detect).
    pub python_path: Option<String>,
    /// Directory containing llm_generate.py / img_generate.py.
    /// None = `<workspace_root>/py/flash_attn_v100/comfy_fallback`.
    pub script_dir: Option<String>,
    /// "auto" | "cuda" | "cpu"
    pub device: String,
    /// "auto" | "float16" | "bfloat16" | "float32"
    pub dtype: String,
    /// Master switch for the Python fallback.
    pub enabled: bool,
}

impl Default for PythonInferConfig {
    fn default() -> Self {
        Self {
            python_path: None,
            script_dir: None,
            device: "auto".to_string(),
            dtype: "auto".to_string(),
            enabled: true,
        }
    }
}

/// Default fallback script directory relative to the workspace root.
pub fn default_script_dir(workspace_root: &Path) -> PathBuf {
    workspace_root.join(FALLBACK_PKG_DIR)
}

/// Resolve the fallback script directory:
/// explicit config -> env `COMFY_PYTHON_SCRIPT_DIR` -> `<workspace_root>/...` -> cwd-relative.
pub fn resolve_script_dir(explicit: Option<&str>, workspace_root: Option<&Path>) -> Option<PathBuf> {
    if let Some(dir) = explicit {
        let p = PathBuf::from(dir);
        if p.is_dir() {
            return Some(p);
        }
    }
    if let Ok(dir) = std::env::var("COMFY_PYTHON_SCRIPT_DIR") {
        let p = PathBuf::from(dir);
        if p.is_dir() {
            return Some(p);
        }
    }
    if let Some(root) = workspace_root {
        let p = default_script_dir(root);
        if p.is_dir() {
            return Some(p);
        }
    }
    if let Ok(cwd) = std::env::current_dir() {
        let p = cwd.join(FALLBACK_PKG_DIR);
        if p.is_dir() {
            return Some(p);
        }
    }
    None
}

/// Resolve a Python interpreter:
/// explicit -> env `COMFY_PYTHON_PATH` -> venvs next to the script dir ->
/// known venv-cu128 locations -> system python3.
pub fn resolve_python(explicit: Option<&str>, script_dir: Option<&Path>) -> String {
    if let Some(py) = explicit {
        if !py.is_empty() && py != "python3" {
            return py.to_string();
        }
    }
    if let Ok(py) = std::env::var("COMFY_PYTHON_PATH") {
        if !py.is_empty() {
            return py;
        }
    }

    // script_dir = <workspace>/py/flash_attn_v100/comfy_fallback;
    // its parent is the py project root that may contain a venv.
    if let Some(dir) = script_dir {
        if let Some(project_root) = dir.parent() {
            for venv in &["venv-cu128", ".venv", "venv", "env"] {
                let py = project_root.join(venv).join("bin").join("python");
                if py.exists() {
                    return py.to_string_lossy().to_string();
                }
            }
        }
    }

    // Prefer the venv shipped inside this repository's py project.
    let repo_venv = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../py/flash_attn_v100/venv-cu128/bin/python"
    );
    if Path::new(repo_venv).exists() {
        return repo_venv.to_string();
    }

    // Secondary machine-local venv (older layout, see flash_attn_backend).
    let known = "/home/acproject/workspace/python_projects/flash_attn_v100/venv-cu128/bin/python";
    if Path::new(known).exists() {
        return known.to_string();
    }

    "python3".to_string()
}

/// A directory containing a HuggingFace checkpoint (config.json present).
pub fn is_hf_model_dir(path: &str) -> bool {
    if path.is_empty() {
        return false;
    }
    let p = Path::new(path);
    p.is_dir() && p.join("config.json").exists()
}

/// A directory containing a diffusers pipeline (model_index.json present),
/// e.g. `stable-diffusion-xl-base-1.0/` as produced by `snapshot_download`.
pub fn is_diffusers_pipeline_dir(path: &str) -> bool {
    if path.is_empty() {
        return false;
    }
    let p = Path::new(path);
    p.is_dir() && p.join("model_index.json").exists()
}

pub struct PythonBackend {
    config: PythonInferConfig,
    interpreter: String,
    script_dir: Option<PathBuf>,
}

impl std::fmt::Debug for PythonBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PythonBackend")
            .field("config", &self.config)
            .field("interpreter", &self.interpreter)
            .field("script_dir", &self.script_dir)
            .finish()
    }
}

impl PythonBackend {
    pub fn new(config: PythonInferConfig, workspace_root: Option<&Path>) -> Self {
        let script_dir = resolve_script_dir(config.script_dir.as_deref(), workspace_root);
        let interpreter = resolve_python(config.python_path.as_deref(), script_dir.as_deref());
        if script_dir.is_none() {
            tracing::warn!(
                "Python fallback scripts directory not found (looked for {}); \
                 Python inference will fail until it is available",
                FALLBACK_PKG_DIR
            );
        }
        Self {
            config,
            interpreter,
            script_dir,
        }
    }

    pub fn interpreter(&self) -> &str {
        &self.interpreter
    }

    fn script_path(&self, name: &str) -> InferenceResult<PathBuf> {
        let dir = self
            .script_dir
            .clone()
            .ok_or_else(|| InferenceError::BackendNotAvailable(
                "Python fallback script directory not found (py/flash_attn_v100/comfy_fallback)"
                    .to_string(),
            ))?;
        Ok(dir.join(name))
    }

    /// Best model path for HF routing: standalone checkpoint first,
    /// then a diffusion-model component dir.
    fn pick_model_path(params: &ImageGenParams) -> Option<String> {
        params
            .model_config
            .model_path
            .clone()
            .or_else(|| params.model_config.diffusion_model_path.clone())
    }

    fn temp_dir() -> PathBuf {
        let dir = std::env::temp_dir().join("comfyui-rust");
        std::fs::create_dir_all(&dir).ok();
        dir
    }

    fn write_temp_png(image: &SdImage, prefix: &str) -> InferenceResult<PathBuf> {
        let path = Self::temp_dir().join(format!("{}_{}.png", prefix, std::process::id()));
        std::fs::write(&path, image.to_png_bytes()?)
            .map_err(|e| InferenceError::ImageDecodeError(e.to_string()))?;
        Ok(path)
    }

    pub fn generate_image_blocking(&self, params: &ImageGenParams) -> InferenceResult<Vec<SdImage>> {
        let model_path = Self::pick_model_path(params)
            .ok_or_else(|| InferenceError::InvalidParameter("No model path for Python backend".to_string()))?;
        let script = self.script_path("img_generate.py")?;

        let output_path = Self::temp_dir().join(format!("py_img_{}.png", params.seed));
        let init_path = params
            .init_image
            .as_ref()
            .map(|img| Self::write_temp_png(img, "py_init"))
            .transpose()?;

        let mut args: Vec<String> = vec![
            script.to_string_lossy().to_string(),
            "--model-path".to_string(),
            model_path,
            "--prompt".to_string(),
            params.prompt.clone(),
            "--negative-prompt".to_string(),
            params.negative_prompt.clone(),
            "--width".to_string(),
            params.width.to_string(),
            "--height".to_string(),
            params.height.to_string(),
            "--steps".to_string(),
            params.sample_params.sample_steps.to_string(),
            "--guidance-scale".to_string(),
            params.sample_params.guidance.txt_cfg.to_string(),
            "--seed".to_string(),
            params.seed.to_string(),
            "--output".to_string(),
            output_path.to_string_lossy().to_string(),
            "--device".to_string(),
            self.config.device.clone(),
            "--dtype".to_string(),
            self.config.dtype.clone(),
        ];
        if let Some(ref init) = init_path {
            args.push("--init-image".to_string());
            args.push(init.to_string_lossy().to_string());
            args.push("--strength".to_string());
            args.push(format!("{}", params.strength));
        }

        tracing::info!(
            "Python fallback image generation: {} {}",
            self.interpreter,
            args.join(" ")
        );

        let output = Command::new(&self.interpreter)
            .args(&args)
            .output()
            .map_err(|e| InferenceError::BackendNotAvailable(format!(
                "Failed to execute Python fallback '{}': {}",
                self.interpreter, e
            )))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(InferenceError::GenerationFailed(format!(
                "Python img_generate.py exited with status {}: {}",
                output.status,
                stderr.lines().rev().take(10).collect::<Vec<_>>().into_iter().rev().collect::<Vec<_>>().join("\n")
            )));
        }

        if !output_path.exists() {
            return Err(InferenceError::GenerationFailed(
                "Python fallback produced no output image".to_string(),
            ));
        }

        let bytes = std::fs::read(&output_path)
            .map_err(|e| InferenceError::ImageDecodeError(e.to_string()))?;
        let img = SdImage::from_png_bytes(&bytes)?;
        Ok(vec![img])
    }

    /// Run the Bernini-R one-shot script (t2i / i2i single frame).
    pub fn generate_bernini_blocking(
        &self,
        params: &ImageGenParams,
    ) -> InferenceResult<Vec<SdImage>> {
        let model_path = Self::pick_model_path(params)
            .ok_or_else(|| InferenceError::InvalidParameter("No model path for Bernini fallback".to_string()))?;
        let script = self.script_path("bernini_generate.py")?;

        let output_path = Self::temp_dir().join(format!("py_bernini_{}.png", params.seed));
        let init_path = params
            .init_image
            .as_ref()
            .map(|img| Self::write_temp_png(img, "py_bernini_init"))
            .transpose()?;

        let mut args: Vec<String> = vec![
            script.to_string_lossy().to_string(),
            "--config".to_string(),
            model_path,
            "--prompt".to_string(),
            params.prompt.clone(),
            "--width".to_string(),
            params.width.to_string(),
            "--height".to_string(),
            params.height.to_string(),
            "--steps".to_string(),
            params.sample_params.sample_steps.to_string(),
            "--seed".to_string(),
            params.seed.to_string(),
            "--output".to_string(),
            output_path.to_string_lossy().to_string(),
            "--device".to_string(),
            self.config.device.clone(),
            "--dtype".to_string(),
            self.config.dtype.clone(),
        ];
        if !params.negative_prompt.is_empty() {
            args.push("--neg-prompt".to_string());
            args.push(params.negative_prompt.clone());
        }
        if let Some(ref init) = init_path {
            args.push("--input".to_string());
            args.push(init.to_string_lossy().to_string());
        }

        tracing::info!(
            "Python fallback Bernini-R image generation: {} {}",
            self.interpreter,
            args.join(" ")
        );

        let output = Command::new(&self.interpreter)
            .args(&args)
            .output()
            .map_err(|e| InferenceError::BackendNotAvailable(format!(
                "Failed to execute Python fallback '{}': {}",
                self.interpreter, e
            )))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(InferenceError::GenerationFailed(format!(
                "Python bernini_generate.py exited with status {}: {}",
                output.status,
                stderr.lines().rev().take(10).collect::<Vec<_>>().into_iter().rev().collect::<Vec<_>>().join("\n")
            )));
        }

        if !output_path.exists() {
            return Err(InferenceError::GenerationFailed(
                "Python Bernini fallback produced no output image".to_string(),
            ));
        }

        let bytes = std::fs::read(&output_path)
            .map_err(|e| InferenceError::ImageDecodeError(e.to_string()))?;
        let img = SdImage::from_png_bytes(&bytes)?;
        Ok(vec![img])
    }

    /// Whether `path` is a Bernini-R self-contained model directory.
    ///
    /// Such dirs ship a custom `config.json` (`model_type=bernini_renderer`)
    /// rather than a diffusers `model_index.json`, so the regular pipeline-dir
    /// detection does not see them.
    pub fn is_bernini_model_dir(path: &str) -> bool {
        if path.is_empty() {
            return false;
        }
        let p = Path::new(path);
        let name_ok = p
            .file_name()
            .map(|n| n.to_string_lossy().to_lowercase().contains("bernini"))
            .unwrap_or(false);
        name_ok && p.is_dir() && p.join("config.json").exists()
    }

    /// Run the TripoSplat one-shot script (image -> 3D Gaussian PLY/SPLAT).
    pub fn generate_3d_gaussian_blocking(
        &self,
        params: &Gaussian3DParams,
    ) -> InferenceResult<Gaussian3DOutput> {
        let script = self.script_path("triposplat_generate.py")?;
        let mc = &params.model_config;

        let ckpt = mc
            .model_path
            .clone()
            .or_else(|| mc.diffusion_model_path.clone())
            .ok_or_else(|| {
                InferenceError::InvalidParameter(
                    "No TripoSplat checkpoint path for Python backend".to_string(),
                )
            })?;
        let decoder = mc.decoder_path.clone().ok_or_else(|| {
            InferenceError::InvalidParameter(
                "No TripoSplat gaussian decoder path (triposplat_vae_decoder)".to_string(),
            )
        })?;
        let dinov3 = mc.clip_vision_path.clone().ok_or_else(|| {
            InferenceError::InvalidParameter("No DINOv3 clip-vision path".to_string())
        })?;
        let vae_encoder = mc.vae_path.clone().ok_or_else(|| {
            InferenceError::InvalidParameter("No FLUX2 VAE encoder path".to_string())
        })?;
        let rmbg = mc.rmbg_path.clone().ok_or_else(|| {
            InferenceError::InvalidParameter("No background-removal (BiRefNet) path".to_string())
        })?;

        let input_image = params.input_image.as_ref().ok_or_else(|| {
            InferenceError::InvalidParameter("No input image for 3D generation".to_string())
        })?;
        let input_path = Self::write_temp_png(input_image, "py_3d_input")?;

        let format = if params.output_format == 1 { "splat" } else { "ply" };
        let output_path = match params.output_path.clone() {
            Some(p) => PathBuf::from(p),
            None => Self::temp_dir().join(format!("py_3d_{}.{}", params.seed, format)),
        };
        let prepared_path = output_path
            .with_extension("")
            .with_file_name(format!(
                "{}_preprocessed.webp",
                output_path
                    .with_extension("")
                    .file_name()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_default()
            ));

        let args: Vec<String> = vec![
            script.to_string_lossy().to_string(),
            "--input".into(),
            input_path.to_string_lossy().to_string(),
            "--output".into(),
            output_path.to_string_lossy().to_string(),
            "--format".into(),
            format.into(),
            "--prepared-output".into(),
            prepared_path.to_string_lossy().to_string(),
            "--ckpt".into(),
            ckpt,
            "--decoder".into(),
            decoder,
            "--dinov3".into(),
            dinov3,
            "--vae-encoder".into(),
            vae_encoder,
            "--rmbg".into(),
            rmbg,
            "--seed".into(),
            params.seed.to_string(),
            "--steps".into(),
            params.steps.to_string(),
            "--guidance-scale".into(),
            params.guidance_scale.to_string(),
            "--num-gaussians".into(),
            params.num_gaussians.to_string(),
            "--erode-radius".into(),
            params.erode_radius.to_string(),
            "--device".into(),
            self.config.device.clone(),
            "--dtype".into(),
            self.config.dtype.clone(),
        ];

        tracing::info!(
            "Python fallback TripoSplat 3D generation: {} {}",
            self.interpreter,
            args.join(" ")
        );

        let output = Command::new(&self.interpreter)
            .args(&args)
            .output()
            .map_err(|e| InferenceError::BackendNotAvailable(format!(
                "Failed to execute Python fallback '{}': {}",
                self.interpreter, e
            )))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(InferenceError::GenerationFailed(format!(
                "Python triposplat_generate.py exited with status {}: {}",
                output.status,
                stderr
                    .lines()
                    .rev()
                    .take(10)
                    .collect::<Vec<_>>()
                    .into_iter()
                    .rev()
                    .collect::<Vec<_>>()
                    .join("\n")
            )));
        }

        if !output_path.exists() {
            return Err(InferenceError::GenerationFailed(
                "Python TripoSplat fallback produced no output file".to_string(),
            ));
        }

        // The script's last stdout line is a JSON result object.
        let stdout = String::from_utf8_lossy(&output.stdout);
        let last_line = stdout.lines().rev().find(|l| l.trim_start().starts_with('{'));
        let num_gaussians = last_line
            .and_then(|l| serde_json::from_str::<serde_json::Value>(l).ok())
            .and_then(|v| v.get("num_gaussians").and_then(|n| n.as_u64()))
            .unwrap_or(0) as usize;

        Ok(Gaussian3DOutput {
            xyz: Vec::new(),
            features_dc: Vec::new(),
            opacity: Vec::new(),
            scaling: Vec::new(),
            rotation: Vec::new(),
            num_gaussians,
            output_file: Some(output_path.to_string_lossy().to_string()),
        })
    }
}

impl InferenceBackend for PythonBackend {
    fn supports_image_generation(&self) -> bool {
        self.script_path("img_generate.py").map(|p| p.exists()).unwrap_or(false)
    }

    fn supports_video_generation(&self) -> bool {
        false
    }

    fn supports_3d_generation(&self) -> bool {
        self.script_path("triposplat_generate.py")
            .map(|p| p.exists())
            .unwrap_or(false)
    }

    fn generate_image(&self, params: ImageGenParams) -> InferenceResult<Vec<SdImage>> {
        self.generate_image_blocking(&params)
    }

    fn generate_3d_gaussian(
        &self,
        params: Gaussian3DParams,
    ) -> InferenceResult<Gaussian3DOutput> {
        self.generate_3d_gaussian_blocking(&params)
    }

    fn generate_video(&self, _params: VideoGenParams) -> InferenceResult<SdVideo> {
        Err(InferenceError::UnsupportedOperation(
            "Video generation is not supported by the Python fallback backend".to_string(),
        ))
    }

    fn upscale(&self, _image: SdImage, _params: UpscaleParams) -> InferenceResult<SdImage> {
        Err(InferenceError::UnsupportedOperation(
            "Upscale is not supported by the Python fallback backend".to_string(),
        ))
    }
}

/// Composite backend: CLI/FFI first, Python HF fallback when the model is in
/// HF/diffusers format or the primary backend fails.
pub struct FallbackBackend {
    primary: Arc<dyn InferenceBackend>,
    python: PythonBackend,
}

impl std::fmt::Debug for FallbackBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FallbackBackend")
            .field("python", &self.python)
            .finish()
    }
}

impl FallbackBackend {
    pub fn new(primary: Arc<dyn InferenceBackend>, python: PythonBackend) -> Self {
        Self { primary, python }
    }

    /// Families whose HF/diffusers pipeline directory also ships a
    /// standalone component layout that sd.cpp can load directly:
    /// `<dir>/transformer/*.safetensors(.index.json)` + `<dir>/vae/...`.
    const SDCPP_PIPELINE_FAMILIES: &'static [&'static str] = &[
        "qwen_image_2.1",
        "qwen_image_edit",
        "boogu_image",
        "boogu_image_edit",
    ];

    fn models_base_dir() -> PathBuf {
        let base =
            std::env::var("COMFY_MODELS_DIR").unwrap_or_else(|_| "models".to_string());
        let p = Path::new(&base);
        if p.is_relative() {
            std::env::current_dir().unwrap_or_default().join(p)
        } else {
            p.to_path_buf()
        }
    }

    /// Locate a pipeline directory even if the loader produced a
    /// `<base>/checkpoints/<name>` style path while the directory actually
    /// lives at the models root.
    fn resolve_pipeline_dir(raw: &str) -> Option<PathBuf> {
        let p = Path::new(raw);
        let candidates = if p.is_dir() {
            vec![p.to_path_buf()]
        } else {
            let name = p.file_name()?;
            let base = Self::models_base_dir();
            vec![base.join(name), base.join("checkpoints").join(name)]
        };
        candidates
            .into_iter()
            .find(|c| c.is_dir() && c.join("model_index.json").exists())
    }

    fn first_existing(paths: &[PathBuf]) -> Option<PathBuf> {
        paths.iter().find(|p| p.is_file()).cloned()
    }

    /// Best-effort search for the Qwen3-VL LLM encoder used by
    /// Qwen Image 2.1 / Boogu: prefer a flattened (Comfy-Org style)
    /// safetensors/GGUF file over the raw HF sharded directory.
    fn find_qwen3vl_encoder(with_vision: bool) -> Option<String> {
        let te_dir = Self::models_base_dir().join("text_encoders");
        let entries = std::fs::read_dir(&te_dir).ok()?;
        let mut files: Vec<String> = entries
            .filter_map(|e| e.ok())
            .map(|e| e.file_name().to_string_lossy().to_string())
            .filter(|n| {
                let n = n.to_lowercase();
                n.contains("qwen3vl") && (n.ends_with(".safetensors") || n.ends_with(".gguf"))
            })
            .collect();
        files.sort();
        let pick = if with_vision {
            // Extracted visual-only weights (keys unprefixed to `visual.*`,
            // matching sd.cpp's llm_vision loader).
            files
                .iter()
                .find(|n| n.contains("visual"))
                .cloned()
        } else {
            // Full encoder, excluding the vision-only extract.
            files
                .iter()
                .find(|n| !n.contains("visual"))
                .cloned()
        };
        pick.map(|n| te_dir.join(n).to_string_lossy().to_string())
    }

    /// Rewrite an sd.cpp-supported HF pipeline directory into standalone
    /// component paths (transformer shard index + pipeline-local VAE +
    /// Qwen3-VL LLM) so it runs on the primary backend instead of Python.
    fn rewrite_pipeline_for_sdcpp(params: ImageGenParams) -> ImageGenParams {
        let Some(raw) = params.model_config.model_path.clone() else {
            return params;
        };
        if Path::new(&raw).is_file() {
            return params;
        }
        let Some(dir) = Self::resolve_pipeline_dir(&raw) else {
            return params;
        };
        let Some(family) = crate::sdcpp_support::match_identifier(&dir.to_string_lossy()) else {
            return params;
        };
        if !Self::SDCPP_PIPELINE_FAMILIES.contains(&family.id.as_str()) {
            return params;
        }

        let diffusion = Self::first_existing(&[
            dir.join("transformer").join("diffusion_pytorch_model.safetensors.index.json"),
            dir.join("transformer").join("model.safetensors.index.json"),
            dir.join("transformer").join("diffusion_pytorch_model.safetensors"),
            dir.join("transformer").join("model.safetensors"),
        ]);
        let Some(diffusion) = diffusion else {
            tracing::warn!(
                "Pipeline dir '{}' matched family {} but contains no standalone \
                 transformer shard index; keeping original routing",
                dir.display(),
                family.id
            );
            return params;
        };

        let vae = Self::first_existing(&[
            dir.join("vae").join("diffusion_pytorch_model.safetensors"),
            dir.join("vae").join("model.safetensors"),
        ])
        .or_else(|| {
            std::fs::read_dir(dir.join("vae")).ok().and_then(|entries| {
                entries
                    .filter_map(|e| e.ok())
                    .map(|e| e.path())
                    .find(|p| {
                        p.extension()
                            .map(|x| x == "safetensors")
                            .unwrap_or(false)
                    })
            })
        });

        let mut params = params;
        params.model_config.model_path = None;
        params.model_config.diffusion_model_path = Some(diffusion.to_string_lossy().to_string());
        if let Some(vae) = vae {
            // The component VAE shipped inside the pipeline directory is the
            // authoritative match (e.g. Qwen Image 2.1 VAE is NOT
            // interchangeable with the earlier Qwen Image VAE).
            params.model_config.vae_path = Some(vae.to_string_lossy().to_string());
        }
        if params.model_config.llm_path.is_none() {
            if let Some(llm) = Self::find_qwen3vl_encoder(false) {
                tracing::info!("Pipeline '{}': auto-selected LLM encoder '{}'", dir.display(), llm);
                params.model_config.llm_path = Some(llm);
            }
        }
        if params.model_config.llm_vision_path.is_none() && !params.ref_images.is_empty() {
            if let Some(vision) = Self::find_qwen3vl_encoder(true) {
                tracing::info!(
                    "Pipeline '{}': reference images present, auto-selected vision weights '{}'",
                    dir.display(),
                    vision
                );
                params.model_config.llm_vision_path = Some(vision);
            }
        }

        tracing::info!(
            "Pipeline dir '{}' matched sd.cpp family {} ({}); rewritten to native component paths",
            dir.display(),
            family.id,
            family.name
        );
        params
    }

    /// Bernini-R ships a custom (non-diffusers) pipeline layout that neither
    /// sd.cpp nor img_generate.py understand; it needs bernini_generate.py.
    fn supports_bernini(&self) -> bool {
        self.python
            .script_path("bernini_generate.py")
            .map(|p| p.exists())
            .unwrap_or(false)
    }

    fn route_to_python(params: &ImageGenParams) -> bool {
        match PythonBackend::pick_model_path(params) {
            Some(p) if is_diffusers_pipeline_dir(&p) => true,
            Some(p) => {
                // Weight files go to the primary (sd.cpp) backend. Log the
                // matched family so unsupported models are easy to spot.
                match crate::sdcpp_support::match_identifier(&p) {
                    Some(family) => tracing::info!(
                        "Model '{}' matched stable-diffusion.cpp supported family: {} ({:?})",
                        p, family.name, family.kind
                    ),
                    None => tracing::info!(
                        "Model '{}' is not in the known stable-diffusion.cpp support list; \
                         primary backend will be tried first anyway",
                        p
                    ),
                }
                false
            }
            None => false,
        }
    }
}

impl InferenceBackend for FallbackBackend {
    fn supports_image_generation(&self) -> bool {
        self.primary.supports_image_generation()
            || self.python.supports_image_generation()
            || self.supports_bernini()
    }

    fn supports_video_generation(&self) -> bool {
        self.primary.supports_video_generation()
    }

    fn supports_3d_generation(&self) -> bool {
        self.primary.supports_3d_generation() || self.python.supports_3d_generation()
    }

    fn supports_audio_video_generation(&self) -> bool {
        self.primary.supports_audio_video_generation()
    }

    fn supports_context_ir(&self) -> bool {
        self.primary.supports_context_ir()
    }

    fn generate_image(&self, params: ImageGenParams) -> InferenceResult<Vec<SdImage>> {
        let params = Self::rewrite_pipeline_for_sdcpp(params);
        if let Some(path) = PythonBackend::pick_model_path(&params) {
            if PythonBackend::is_bernini_model_dir(&path) {
                if !self.supports_bernini() {
                    return Err(InferenceError::BackendNotAvailable(
                        "Bernini-R model selected but bernini_generate.py is unavailable".to_string(),
                    ));
                }
                tracing::info!(
                    "Bernini-R model directory detected; routing directly to Python bernini_generate.py"
                );
                return self.python.generate_bernini_blocking(&params);
            }
        }
        if Self::route_to_python(&params) {
            tracing::info!("Model is a HF/diffusers pipeline directory, routing directly to Python fallback");
            return self.python.generate_image(params);
        }

        match self.primary.generate_image(params.clone()) {
            Ok(images) => Ok(images),
            Err(primary_err) => {
                if !self.python.supports_image_generation() {
                    return Err(primary_err);
                }
                tracing::warn!(
                    "Primary image backend failed ({}); retrying with Python HF fallback",
                    primary_err
                );
                match self.python.generate_image(params) {
                    Ok(images) => Ok(images),
                    Err(py_err) => Err(InferenceError::GenerationFailed(format!(
                        "Primary backend error: {} | Python fallback error: {}",
                        primary_err, py_err
                    ))),
                }
            }
        }
    }

    fn generate_video(&self, params: VideoGenParams) -> InferenceResult<SdVideo> {
        self.primary.generate_video(params)
    }

    fn upscale(&self, image: SdImage, params: UpscaleParams) -> InferenceResult<SdImage> {
        self.primary.upscale(image, params)
    }

    fn generate_3d_gaussian(
        &self,
        params: crate::params::Gaussian3DParams,
    ) -> InferenceResult<crate::params::Gaussian3DOutput> {
        // sd.cpp has no TripoSplat support; if the primary backend cannot do
        // 3D, route straight to the Python TripoSplat fallback.
        if !self.primary.supports_3d_generation() {
            if !self.python.supports_3d_generation() {
                return Err(InferenceError::BackendNotAvailable(
                    "3D Gaussian generation is unavailable (no primary backend, \
                     no Python triposplat_generate.py)"
                        .to_string(),
                ));
            }
            tracing::info!(
                "Primary backend does not support 3D Gaussian generation; \
                 routing to Python TripoSplat fallback"
            );
            return self.python.generate_3d_gaussian(params);
        }

        match self.primary.generate_3d_gaussian(params.clone()) {
            Ok(out) => Ok(out),
            Err(primary_err) => {
                if !self.python.supports_3d_generation() {
                    return Err(primary_err);
                }
                tracing::warn!(
                    "Primary 3D backend failed ({}); retrying with Python TripoSplat fallback",
                    primary_err
                );
                match self.python.generate_3d_gaussian(params) {
                    Ok(out) => Ok(out),
                    Err(py_err) => Err(InferenceError::GenerationFailed(format!(
                        "Primary backend error: {} | Python TripoSplat fallback error: {}",
                        primary_err, py_err
                    ))),
                }
            }
        }
    }

    fn generate_av(&self, params: crate::params::H3Params) -> InferenceResult<SdVideo> {
        self.primary.generate_av(params)
    }

    fn context_ir(&self, params: crate::params::ContextIrParams) -> InferenceResult<crate::params::H3Context> {
        self.primary.context_ir(params)
    }

    fn decode_video_latent(
        &self,
        latent: &serde_json::Value,
        params: &VideoGenParams,
    ) -> InferenceResult<SdVideo> {
        self.primary.decode_video_latent(latent, params)
    }
}
